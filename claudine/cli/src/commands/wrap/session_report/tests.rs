use super::*;
use std::time::Instant;

/// A private directory to hold the daemon's endpoint.
///
/// `tempfile` creates 0755 directories, which the daemon refuses: the endpoint
/// directory is the Unix security boundary. The real default resolves into a
/// private directory, so a fixture has to build one too.
fn private_endpoint_dir(parent: &std::path::Path) -> std::path::PathBuf {
    use std::os::unix::fs::DirBuilderExt;

    let dir = parent.join("rendezvous-runtime");
    if !dir.exists() {
        std::fs::DirBuilder::new()
            .mode(0o700)
            .create(&dir)
            .expect("create runtime dir");
    }
    dir
}


fn env_context() -> EnvironmentContext {
    EnvironmentContext::default()
}

/// nextest runs each test in its own process, so mutating the
/// process environment here cannot race other tests.
fn point_socket_at(path: &std::path::Path) {
    unsafe { std::env::set_var("RENDEZVOUS_SOCKET", path) };
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn no_daemon_is_fast_and_silent() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    point_socket_at(&tmp.path().join("absent.sock"));

    let started = Instant::now();
    let presence = SessionPresence::started(
        Provider::Claude,
        Some("opus"),
        false,
        &env_context(),
        &std::collections::HashMap::new(),
    );
    assert!(presence.session_id.is_none(), "no daemon: nothing acknowledged");
    drop(presence);
    assert!(
        started.elapsed() < Duration::from_secs(2),
        "absent daemon must not stall the wrapper: {:?}",
        started.elapsed(),
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn kill_switch_disables_reporting() {
    unsafe { std::env::set_var(ENABLE_ENV, "false") };
    let presence = SessionPresence::started(
        Provider::Claude,
        None,
        true,
        &env_context(),
        &std::collections::HashMap::new(),
    );
    assert!(presence.session_id.is_none());
}

// Spawns the real `rendezvous-daemon`, which is a `cfg(unix)`-only
// dev-dependency (its server binds a `UnixListener`; the Windows
// named-pipe server is a tracked follow-up). Gate so the crate still
// compiles on Windows.
#[cfg(unix)]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn round_trip_against_live_daemon() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let socket = private_endpoint_dir(tmp.path()).join("daemon.sock");
    point_socket_at(&socket);

    let mut config = rendezvous_daemon::server::DaemonConfig::with_data_dir(
        tmp.path().join("data"),
    )
    .with_in_memory_projection();
    config.networking = None;
    let daemon = rendezvous_daemon::server::spawn_uds_server(socket.clone(), config)
        .expect("spawn daemon");
    // Wait for the socket to appear.
    let deadline = Instant::now() + Duration::from_secs(5);
    while !socket.exists() {
        assert!(Instant::now() < deadline, "daemon socket never appeared");
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    let child_env: std::collections::HashMap<std::ffi::OsString, std::ffi::OsString> =
        [(
            std::ffi::OsString::from("CLAUDINE_SESSION_ID"),
            std::ffi::OsString::from("sess-presence"),
        )]
        .into_iter()
        .collect();
    let presence = SessionPresence::started(
        Provider::Claude,
        Some("opus"),
        false,
        &env_context(),
        &child_env,
    );
    assert_eq!(presence.session_id.as_deref(), Some("sess-presence"));

    let mut client = rendezvous_client::connect(&rendezvous_core::socket::legacy_local_endpoint(socket.clone())).await.expect("client");
    let hosts = client
        .list_active_sessions(rendezvous_core::ListActiveSessionsRequest {})
        .await
        .expect("list")
        .into_inner()
        .hosts;
    assert_eq!(hosts.len(), 1);
    let sessions: serde_json::Value =
        serde_json::from_str(&hosts[0].sessions_json).expect("json");
    assert_eq!(sessions["sess-presence"]["agent"], json!("claude"));
    assert_eq!(sessions["sess-presence"]["model"], json!("opus"));
    assert_eq!(sessions["sess-presence"]["interactive"], json!(false));

    // Dropping the guard clears the entry.
    drop(presence);
    let hosts = client
        .list_active_sessions(rendezvous_core::ListActiveSessionsRequest {})
        .await
        .expect("list after drop")
        .into_inner()
        .hosts;
    let sessions: serde_json::Value =
        serde_json::from_str(&hosts[0].sessions_json).expect("json");
    assert!(
        sessions.as_object().is_some_and(|m| m.is_empty()),
        "drop must clear the entry: {sessions}",
    );

    daemon.shutdown().await.expect("daemon shutdown");
}

// Unix-only: spawns the `cfg(unix)`-gated `rendezvous-daemon`.
#[cfg(unix)]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn status_reporter_flips_and_clears_waiting() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let socket = private_endpoint_dir(tmp.path()).join("daemon.sock");
    point_socket_at(&socket);

    let mut config = rendezvous_daemon::server::DaemonConfig::with_data_dir(
        tmp.path().join("data"),
    )
    .with_in_memory_projection();
    config.networking = None;
    let daemon = rendezvous_daemon::server::spawn_uds_server(socket.clone(), config)
        .expect("spawn daemon");
    let deadline = Instant::now() + Duration::from_secs(5);
    while !socket.exists() {
        assert!(Instant::now() < deadline, "daemon socket never appeared");
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    let child_env: std::collections::HashMap<std::ffi::OsString, std::ffi::OsString> = [(
        std::ffi::OsString::from("CLAUDINE_SESSION_ID"),
        std::ffi::OsString::from("sess-status"),
    )]
    .into_iter()
    .collect();
    let presence = SessionPresence::started(
        Provider::Claude,
        Some("opus"),
        true,
        &env_context(),
        &child_env,
    );
    let reporter = presence.status_reporter();

    let mut client = rendezvous_client::connect(&rendezvous_core::socket::legacy_local_endpoint(socket.clone())).await.expect("client");

    // Trigger 1: a permission ask flips the session to waiting.
    reporter.report("waiting_on_user");
    await_status(&mut client, "sess-status", "waiting_on_user").await;

    // Progress clears it back to active.
    reporter.report("active");
    await_status(&mut client, "sess-status", "active").await;

    drop(presence);
    daemon.shutdown().await.expect("daemon shutdown");
}

#[test]
fn idle_hook_contribution_maps_idle_and_clears_to_active() {
    let idle = idle_hook_contribution("idle");
    assert_eq!(idle.state, rendezvous_core::SessionStatusState::Idle as i32);
    assert_eq!(idle.producer, rendezvous_core::StatusProducer::IdleHook as i32);
    assert_eq!(
        idle.basis,
        rendezvous_core::SessionStatusBasis::InteractiveTurnComplete as i32,
    );

    let active = idle_hook_contribution("active");
    assert_eq!(active.state, rendezvous_core::SessionStatusState::Active as i32);
    assert_eq!(active.producer, rendezvous_core::StatusProducer::IdleHook as i32);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn report_status_kill_switch_and_missing_session_are_fast_noops() {
    // Kill switch off: returns without touching the socket.
    unsafe { std::env::set_var(ENABLE_ENV, "false") };
    report_status("sess", "idle").await;
    unsafe { std::env::remove_var(ENABLE_ENV) };

    // Empty session id: nothing to address, no connect attempted.
    let tmp = tempfile::TempDir::new().expect("tempdir");
    point_socket_at(&tmp.path().join("absent.sock"));
    let started = Instant::now();
    report_status("", "idle").await;
    assert!(
        started.elapsed() < Duration::from_secs(1),
        "empty session id must short-circuit: {:?}",
        started.elapsed(),
    );
}

// Unix-only: spawns the `cfg(unix)`-gated `rendezvous-daemon`. The
// idle hook (Trigger 2) reports through the explicit-session
// `report_status` helper; this proves the round-trip flips a STARTED
// session to `idle` and the next prompt clears it back to `active`.
#[cfg(unix)]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn report_status_flips_idle_and_clears_to_active() {
    unsafe { std::env::remove_var(ENABLE_ENV) };
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let socket = private_endpoint_dir(tmp.path()).join("daemon.sock");
    point_socket_at(&socket);

    let mut config = rendezvous_daemon::server::DaemonConfig::with_data_dir(
        tmp.path().join("data"),
    )
    .with_in_memory_projection();
    config.networking = None;
    let daemon = rendezvous_daemon::server::spawn_uds_server(socket.clone(), config)
        .expect("spawn daemon");
    let deadline = Instant::now() + Duration::from_secs(5);
    while !socket.exists() {
        assert!(Instant::now() < deadline, "daemon socket never appeared");
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    let child_env: std::collections::HashMap<std::ffi::OsString, std::ffi::OsString> = [(
        std::ffi::OsString::from("CLAUDINE_SESSION_ID"),
        std::ffi::OsString::from("sess-idle"),
    )]
    .into_iter()
    .collect();
    let presence = SessionPresence::started(
        Provider::Claude,
        Some("opus"),
        true,
        &env_context(),
        &child_env,
    );

    let mut client = rendezvous_client::connect(&rendezvous_core::socket::legacy_local_endpoint(socket.clone())).await.expect("client");

    // Turn complete on an interactive session → idle.
    report_status("sess-idle", "idle").await;
    await_status(&mut client, "sess-idle", "idle").await;

    // Next user prompt clears the idle slot → active.
    report_status("sess-idle", "active").await;
    await_status(&mut client, "sess-idle", "active").await;

    drop(presence);
    daemon.shutdown().await.expect("daemon shutdown");
}

// Unix-only: spawns the `cfg(unix)`-gated `rendezvous-daemon`.
// `permission_signal` is computed at STARTED from the launched
// provider's PermissionRequest support, so the dashboard can tell
// "no intervention needed" apart from "signal unavailable."
#[cfg(unix)]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn started_records_permission_signal_per_provider() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let socket = private_endpoint_dir(tmp.path()).join("daemon.sock");
    point_socket_at(&socket);

    let mut config = rendezvous_daemon::server::DaemonConfig::with_data_dir(
        tmp.path().join("data"),
    )
    .with_in_memory_projection();
    config.networking = None;
    let daemon = rendezvous_daemon::server::spawn_uds_server(socket.clone(), config)
        .expect("spawn daemon");
    let deadline = Instant::now() + Duration::from_secs(5);
    while !socket.exists() {
        assert!(Instant::now() < deadline, "daemon socket never appeared");
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    let child_env = |session: &str| {
        [(
            std::ffi::OsString::from("CLAUDINE_SESSION_ID"),
            std::ffi::OsString::from(session),
        )]
        .into_iter()
        .collect::<std::collections::HashMap<_, _>>()
    };

    // Claude exposes a PermissionRequest hook → supported.
    let supported =
        SessionPresence::started(Provider::Claude, None, true, &env_context(), &child_env("sup"));
    // Codex has no permission-ask surface → unsupported.
    let unsupported =
        SessionPresence::started(Provider::Codex, None, true, &env_context(), &child_env("uns"));

    let mut client = rendezvous_client::connect(&rendezvous_core::socket::legacy_local_endpoint(socket.clone())).await.expect("client");
    let hosts = client
        .list_active_sessions(rendezvous_core::ListActiveSessionsRequest {})
        .await
        .expect("list")
        .into_inner()
        .hosts;
    let sessions: serde_json::Value =
        serde_json::from_str(&hosts[0].sessions_json).expect("json");
    assert_eq!(sessions["sup"]["permission_signal"], json!("supported"));
    assert_eq!(sessions["uns"]["permission_signal"], json!("unsupported"));

    drop(supported);
    drop(unsupported);
    daemon.shutdown().await.expect("daemon shutdown");
}

/// Poll the active-sessions register until `session_id` carries
/// `expected` status, since `StatusReporter::report` is
/// fire-and-forget. Only used by the Unix-only live-daemon test.
#[cfg(unix)]
async fn await_status(
    client: &mut rendezvous_core::RendezvousClient<tonic::transport::Channel>,
    session_id: &str,
    expected: &str,
) {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let hosts = client
            .list_active_sessions(rendezvous_core::ListActiveSessionsRequest {})
            .await
            .expect("list")
            .into_inner()
            .hosts;
        let observed = hosts.first().and_then(|h| {
            serde_json::from_str::<serde_json::Value>(&h.sessions_json)
                .ok()
                .and_then(|v| {
                    v.get(session_id)
                        .and_then(|e| e.get("status"))
                        .and_then(|s| s.as_str())
                        .map(str::to_string)
                })
        });
        if observed.as_deref() == Some(expected) {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "status never became {expected} (saw {observed:?})",
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}
