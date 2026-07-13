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
fn direct_peer_with_zero_last_synced_is_never_synced() {
    let now = 1_000_000;
    // REMOTE is a direct peer (present in ListPeers) but has never
    // completed a sync — last_synced_unix_ms is 0.
    let snap = MeshSnapshot::fold(
        LOCAL,
        now,
        &synced(&[(REMOTE, 0)]),
        &[],
        &[],
        &[],
        false,
    );
    let remote = snap.hosts.iter().find(|h| !h.is_local).expect("peer row");
    assert_eq!(remote.staleness, Staleness::NeverSynced);
    assert!(!remote.sessions_trusted());
}

#[test]
fn replica_without_direct_peer_is_freshness_unknown() {
    let now = 1_000_000;
    // REMOTE has a register replica (capability) that reached us
    // transitively but no direct peer entry — freshness is unknown, NOT
    // never-synced (Finding 7).
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
    assert_eq!(remote.staleness, Staleness::FreshnessUnknown);
    assert!(!remote.sessions_trusted());
}

#[test]
fn peer_only_host_appears_with_empty_sessions() {
    let now = 1_000_000;
    // A fresh direct peer with NO register documents of any kind must
    // still render a row (Finding 7: every remote host is accounted for).
    let snap = MeshSnapshot::fold(
        LOCAL,
        now,
        &synced(&[(REMOTE, now - 5_000)]),
        &[],
        &[],
        &[],
        false,
    );
    let remote = snap.hosts.iter().find(|h| !h.is_local).expect("peer-only row");
    assert!(matches!(remote.staleness, Staleness::Fresh { .. }));
    assert!(remote.sessions.is_empty());
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
    assert_eq!(snap.trusted_live_sessions(), 0);
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
///
/// Unix-only: spawns the real `rendezvous-daemon`, a `cfg(unix)`-gated
/// dev-dependency (its server binds a `UnixListener`). The Windows
/// named-pipe daemon server is a tracked follow-up.
#[cfg(unix)]
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

#[test]
fn stale_sessions_excluded_from_live_count() {
    let now = 1_000_000;
    let active = vec![
        (LOCAL, r#"{"s-local":{"agent":"claude","status":"active"}}"#),
        (REMOTE, r#"{"s-remote":{"agent":"codex","status":"active"}}"#),
    ];
    let snap = MeshSnapshot::fold(
        LOCAL,
        now,
        // REMOTE is stale — its session is last-known, not live.
        &synced(&[(REMOTE, now - STALENESS_THRESHOLD_MS - 60_000)]),
        &active,
        &[],
        &[],
        false,
    );
    assert_eq!(snap.trusted_live_sessions(), 1, "only the local session is live");
    assert_eq!(snap.last_known_sessions(), 1, "the stale remote session is last-known");

    // The headline must count only the live session, and flag the
    // last-known one separately.
    let report = DashboardReport::new(snap, "mesh")
        .with_inline_terminal(Terminal::new_optimistic(160));
    let out = report.render(&Terminal::new_optimistic(160));
    assert!(out.contains("1 session"), "headline must count the live session: {out}");
    assert!(!out.contains("2 session"), "headline must not count stale as live: {out}");
    assert!(out.contains("1 last-known"), "last-known missing from headline: {out}");
}

#[test]
fn local_session_renders_reported_age() {
    let now = 1_000_000;
    let active = vec![(
        LOCAL,
        r#"{"s-local":{"agent":"claude","status":"active","updated_at_unix_ms":997000}}"#,
    )];
    let snap = MeshSnapshot::fold(LOCAL, now, &synced(&[]), &active, &[], &[], false);
    let local = snap.hosts.iter().find(|h| h.is_local).expect("local host");
    let session = &local.sessions[0];
    assert_eq!(session.updated_at_unix_ms, Some(997_000));
    assert_eq!(session.reported_age_ms, Some(3_000));

    let report = DashboardReport::new(snap, "local")
        .with_inline_terminal(Terminal::new_optimistic(180));
    let out = report.render(&Terminal::new_optimistic(180));
    assert!(out.contains("3s ago"), "reported age missing: {out}");
}

#[test]
fn remote_session_has_no_reported_age() {
    let now = 1_000_000;
    // Remote wall-clock skew is out of scope: never derive age remotely.
    let active = vec![(
        REMOTE,
        r#"{"s-remote":{"agent":"codex","status":"active","updated_at_unix_ms":500000}}"#,
    )];
    let snap = MeshSnapshot::fold(
        LOCAL,
        now,
        &synced(&[(REMOTE, now - 5_000)]),
        &active,
        &[],
        &[],
        false,
    );
    let remote = snap.hosts.iter().find(|h| !h.is_local).expect("remote host");
    let session = &remote.sessions[0];
    assert_eq!(session.updated_at_unix_ms, Some(500_000));
    assert_eq!(session.reported_age_ms, None);
}

#[test]
fn aged_needs_input_badge_shows_its_age() {
    let now = 1_000_000;
    // A LOCAL waiting_on_user entry last stamped 30 minutes ago — a
    // phantom left by a crashed producer. It stays a needs-input signal
    // but must render visibly aged, not freshly urgent.
    let payload = format!(
        r#"{{"s":{{"agent":"claude","status":"waiting_on_user","updated_at_unix_ms":{}}}}}"#,
        now - 1_800_000
    );
    let active = vec![(LOCAL, payload.as_str())];
    let snap = MeshSnapshot::fold(LOCAL, now, &synced(&[]), &active, &[], &[], false);
    let local = snap.hosts.iter().find(|h| h.is_local).expect("local host");
    let session = &local.sessions[0];
    assert_eq!(session.intervention, Intervention::NeedsInput);
    assert_eq!(session.reported_age_ms, Some(1_800_000));

    let report = DashboardReport::new(snap, "local")
        .with_inline_terminal(Terminal::new_optimistic(200));
    let out = report.render(&Terminal::new_optimistic(200));
    assert!(out.contains("30m ago"), "aged badge missing its age: {out}");
}

#[test]
fn malformed_and_duplicate_registers_are_tolerated() {
    let now = 1_000_000;
    let active = vec![
        (REMOTE, "not json at all"),
        // Duplicate owner — last blob wins, no panic.
        (REMOTE, r#"{"s":{"agent":"codex"}}"#),
        // Wrong shape: array, not object.
        (LOCAL, "[]"),
    ];
    let snap = MeshSnapshot::fold(
        LOCAL,
        now,
        &synced(&[(REMOTE, now - 5_000)]),
        &active,
        &[(LOCAL, "garbage"), (REMOTE, "{")],
        &[(LOCAL, "not json")],
        false,
    );
    assert!(snap.hosts.iter().any(|h| h.is_local), "local host must survive bad blobs");
    // The report renders without panicking on any of the above.
    let report = DashboardReport::new(snap, "mesh")
        .with_inline_terminal(Terminal::new_optimistic(140));
    let _ = report.render(&Terminal::new_optimistic(140));
}

#[tokio::test]
async fn fetch_within_deadline_degrades_when_the_daemon_stalls() {
    // A connected-but-wedged daemon: the fetch future never resolves.
    // The wrapper must give up at the deadline and return None so the
    // command can degrade instead of hanging (Finding 10).
    let stalled = std::future::pending::<color_eyre::eyre::Result<MeshSnapshot>>();
    let out = super::fetch_within_deadline(std::time::Duration::from_millis(50), stalled).await;
    assert!(out.is_none(), "a stalled fetch must time out to None");
}
