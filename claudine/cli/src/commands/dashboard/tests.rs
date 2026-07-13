use std::collections::BTreeMap;

use biscuit_terminal::components::renderable::{BrowserRenderable, TerminalRenderable};
use biscuit_terminal::terminal::Terminal;

use super::model::{Intervention, MeshSnapshot, Staleness, STALENESS_THRESHOLD_MS};
use super::report::DashboardReport;

const LOCAL: &str = "aaaa1111";
const REMOTE: &str = "bbbb2222";

fn synced(map: &[(&str, i64)]) -> BTreeMap<String, i64> {
    map.iter().map(|(k, v)| (k.to_string(), *v)).collect()
}

/// A fresh two-host mesh: local + one remote synced 5s ago, each with a
/// session. The remote's session is waiting on the user.
fn fresh_mesh(now: i64) -> MeshSnapshot {
    let active = vec![
        (
            LOCAL,
            r#"{"s-local":{"agent":"claude","model":"opus","interactive":true,"status":"active","repo_root":"/work/rusty-biscuit"}}"#,
        ),
        (
            REMOTE,
            r#"{"s-remote":{"agent":"codex","status":"waiting_on_user","interactive":false}}"#,
        ),
    ];
    let capabilities = vec![
        (LOCAL, r#"{"name":"studio","os":"macOS","os_version":"15.1","arch":"arm64","cpu_cores":12,"memory":34359738368}"#),
        (REMOTE, r#"{"name":"server","os":"Linux","arch":"x86_64","cpu_cores":32}"#),
    ];
    let repos = vec![(LOCAL, r#"{"rusty-biscuit":"abc123"}"#)];
    MeshSnapshot::fold(
        LOCAL,
        now,
        &synced(&[(REMOTE, now - 5_000)]),
        &active,
        &capabilities,
        &repos,
        false,
    )
}

#[test]
fn local_host_is_always_fresh_and_trusted() {
    let snap = fresh_mesh(1_000_000);
    let local = snap.hosts.iter().find(|h| h.is_local).expect("local host");
    assert_eq!(local.staleness, Staleness::Local);
    assert!(local.sessions_trusted());
    assert_eq!(local.hostname.as_deref(), Some("studio"));
    assert_eq!(local.repo_count, 1);
}

#[test]
fn local_host_sorts_first() {
    let snap = fresh_mesh(1_000_000);
    assert!(snap.hosts[0].is_local, "local host must render first");
}

#[test]
fn fresh_remote_within_threshold_is_trusted_and_flags_intervention() {
    let now = 1_000_000;
    let snap = fresh_mesh(now);
    let remote = snap.hosts.iter().find(|h| !h.is_local).expect("remote host");
    assert!(matches!(remote.staleness, Staleness::Fresh { .. }));
    assert!(remote.sessions_trusted());
    let session = &remote.sessions[0];
    assert_eq!(session.status.as_deref(), Some("waiting_on_user"));
    assert_eq!(session.intervention, Intervention::NeedsInput);
    assert_eq!(snap.needs_input_count(), 1);
}

#[test]
fn remote_past_threshold_is_stale_and_suppresses_intervention() {
    let now = 1_000_000;
    // Last synced just over the 60s threshold.
    let active = vec![(
        REMOTE,
        r#"{"s-remote":{"agent":"codex","status":"waiting_on_user"}}"#,
    )];
    let snap = MeshSnapshot::fold(
        LOCAL,
        now,
        &synced(&[(REMOTE, now - STALENESS_THRESHOLD_MS - 1)]),
        &active,
        &[],
        &[],
        false,
    );
    let remote = snap.hosts.iter().find(|h| !h.is_local).expect("remote host");
    assert!(matches!(remote.staleness, Staleness::Stale { .. }));
    assert!(!remote.sessions_trusted());
    // Stale data: the waiting signal is NOT trusted.
    assert_eq!(remote.sessions[0].intervention, Intervention::None);
    assert_eq!(snap.needs_input_count(), 0);
}

#[test]
fn boundary_at_exactly_threshold_is_still_fresh() {
    let now = 1_000_000;
    let snap = MeshSnapshot::fold(
        LOCAL,
        now,
        &synced(&[(REMOTE, now - STALENESS_THRESHOLD_MS)]),
        &[(REMOTE, r#"{"s":{"agent":"codex"}}"#)],
        &[],
        &[],
        false,
    );
    let remote = snap.hosts.iter().find(|h| !h.is_local).expect("remote host");
    assert_eq!(remote.staleness, Staleness::Fresh { age_ms: STALENESS_THRESHOLD_MS });
}

#[test]
fn remote_never_synced_when_absent_or_zero() {
    let now = 1_000_000;
    // REMOTE has a register replica (capability) but no peer entry.
    let snap = MeshSnapshot::fold(
        LOCAL,
        now,
        &synced(&[]),
        &[],
        &[(REMOTE, r#"{"name":"ghost"}"#)],
        &[],
        false,
    );
    let remote = snap.hosts.iter().find(|h| !h.is_local).expect("remote host");
    assert_eq!(remote.staleness, Staleness::NeverSynced);
    assert!(!remote.sessions_trusted());
}

#[test]
fn local_flag_drops_remote_hosts() {
    let snap = MeshSnapshot::fold(
        LOCAL,
        1_000_000,
        &synced(&[(REMOTE, 999_000)]),
        &[(REMOTE, r#"{"s":{"agent":"codex"}}"#)],
        &[],
        &[],
        true,
    );
    assert_eq!(snap.hosts.len(), 1);
    assert!(snap.hosts[0].is_local);
}

#[test]
fn empty_mesh_still_lists_the_local_host() {
    let snap = MeshSnapshot::fold(LOCAL, 1_000_000, &synced(&[]), &[], &[], &[], false);
    assert_eq!(snap.hosts.len(), 1);
    assert!(snap.hosts[0].is_local);
    assert_eq!(snap.total_sessions(), 0);
}

#[test]
fn report_renders_heading_and_session() {
    let snap = fresh_mesh(1_000_000);
    let report = DashboardReport::new(snap, "mesh")
        .with_inline_terminal(Terminal::new_optimistic(120));
    let out = report.render(&Terminal::new_optimistic(120));
    assert!(out.contains("Rendezvous Dashboard"), "heading missing: {out}");
    assert!(out.contains("studio"), "local hostname missing: {out}");
    assert!(out.contains("claude"), "session agent missing: {out}");
}

#[test]
fn report_marks_stale_sessions_unknown() {
    let now = 1_000_000;
    let snap = MeshSnapshot::fold(
        LOCAL,
        now,
        &synced(&[(REMOTE, now - STALENESS_THRESHOLD_MS - 60_000)]),
        &[(REMOTE, r#"{"s-remote":{"agent":"codex","status":"active"}}"#)],
        &[(REMOTE, r#"{"name":"server"}"#)],
        &[],
        false,
    );
    let report = DashboardReport::new(snap, "mesh")
        .with_inline_terminal(Terminal::new_optimistic(140));
    let out = report.render(&Terminal::new_optimistic(140));
    assert!(out.contains("unknown"), "stale host must render unknown: {out}");
}

#[test]
fn report_html_fragment_carries_the_figures() {
    let snap = fresh_mesh(1_000_000);
    let report = DashboardReport::new(snap, "mesh");
    let html = report.render_html_fragment().render();
    assert!(html.contains("Rendezvous Dashboard"), "html heading: {html}");
    assert!(html.contains("studio"), "html hostname: {html}");
}

/// End-to-end: spawn a real daemon, report a live session over gRPC,
/// then drive the command's own `fetch_snapshot` against it. Guards the
/// RPC-response → fold field mapping the unit tests above stub out.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn fetch_snapshot_reflects_a_live_session() {
    use std::time::{Duration, Instant};

    let tmp = tempfile::TempDir::new().expect("tempdir");
    let socket = tmp.path().join("daemon.sock");
    let mut config = rendezvous_daemon::server::DaemonConfig::with_data_dir(
        tmp.path().join("data"),
    )
    .with_in_memory_projection();
    config.networking = None;
    let daemon =
        rendezvous_daemon::server::spawn_uds_server(socket.clone(), config).expect("spawn daemon");

    let deadline = Instant::now() + Duration::from_secs(5);
    while !socket.exists() {
        assert!(Instant::now() < deadline, "daemon socket never appeared");
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    let mut client = rendezvous_client::connect(&socket).await.expect("client");
    client
        .report_session_event(rendezvous_core::ReportSessionEventRequest {
            session_id: "sess-live".into(),
            kind: rendezvous_core::SessionEventKind::Started as i32,
            details_json:
                r#"{"agent":"claude","model":"opus","interactive":true,"status":"active"}"#.into(),
        })
        .await
        .expect("report session");

    let snapshot = super::fetch_snapshot(&mut client, 1_000_000, false)
        .await
        .expect("fetch snapshot");

    // Exactly the local host, carrying the session we reported.
    assert_eq!(snapshot.hosts.len(), 1, "hosts: {:?}", snapshot.hosts);
    let host = &snapshot.hosts[0];
    assert!(host.is_local);
    assert_eq!(host.staleness, Staleness::Local);
    assert_eq!(host.sessions.len(), 1);
    let session = &host.sessions[0];
    assert_eq!(session.session_id, "sess-live");
    assert_eq!(session.agent.as_deref(), Some("claude"));
    assert_eq!(session.model.as_deref(), Some("opus"));
    assert_eq!(session.interactive, Some(true));
    assert_eq!(snapshot.local_node_id, host.node_id);

    daemon.shutdown().await.expect("daemon shutdown");
}
