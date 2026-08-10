use super::*;
use std::sync::atomic::{AtomicBool, Ordering};

/// Registry with no QUIC endpoint or sync service — enough to
/// exercise the state machine (`apply_resync_outcome`,
/// reconnection bookkeeping) with hand-built records.
fn test_registry() -> PeerRegistry {
    PeerRegistry {
        inner: Arc::new(Inner {
            peers: RwLock::new(HashMap::new()),
            endpoint: Arc::new(RwLock::new(None)),
            sync_service: RwLock::new(None),
        }),
    }
}

/// A `Connected` record with no live connection handle (the handle is
/// irrelevant to the state-machine transitions under test).
fn connected_record(node_id: &str) -> PeerRecord {
    PeerRecord {
        node_id: node_id.to_string(),
        socket_addr: "127.0.0.1:9000".parse().expect("addr"),
        source: PeerSource::Manual,
        state: PeerConnectionState::Connected,
        last_seen_unix_ms: unix_now_ms(),
        last_synced_unix_ms: 0,
        last_error: None,
        connection: None,
        reconnect_after_unix_ms: 0,
        reconnect_attempts: 0,
    }
}

fn insert(reg: &PeerRegistry, rec: PeerRecord) {
    reg.inner.peers.write().insert(rec.node_id.clone(), rec);
}

fn state_of(reg: &PeerRegistry, node_id: &str) -> PeerConnectionState {
    reg.inner.peers.read()[node_id].state
}

// A successful round advances the freshness clock; a failed round
// (soft error OR timeout) leaves it untouched and keeps the peer
// Connected.
#[test]
fn ok_advances_last_synced_while_failures_do_not() {
    let reg = test_registry();
    insert(&reg, connected_record("ok"));
    insert(&reg, connected_record("soft"));
    insert(&reg, connected_record("slow"));

    reg.apply_resync_outcome("ok", ResyncOutcome::Ok);
    reg.apply_resync_outcome("soft", ResyncOutcome::SoftError("boom".into()));
    reg.apply_resync_outcome("slow", ResyncOutcome::TimedOut);

    let map = reg.inner.peers.read();
    assert!(
        map["ok"].last_synced_unix_ms > 0,
        "success stamps last_synced"
    );
    assert_eq!(
        map["soft"].last_synced_unix_ms, 0,
        "soft error must not stamp"
    );
    assert_eq!(map["slow"].last_synced_unix_ms, 0, "timeout must not stamp");
    assert_eq!(map["soft"].last_error.as_deref(), Some("boom"));
    // A wedged/soft-failing peer stays Connected (the connection is
    // presumed alive); only a dead connection leaves Connected.
    assert_eq!(map["ok"].state, PeerConnectionState::Connected);
    assert_eq!(map["soft"].state, PeerConnectionState::Connected);
    assert_eq!(map["slow"].state, PeerConnectionState::Connected);
}

// A dead connection leaves Connected, drops the handle, and arms the
// exponential-backoff re-dial path; a later success clears it.
#[test]
fn dead_connection_disconnects_and_backs_off_redial() {
    let reg = test_registry();
    let mut rec = connected_record("gone");
    rec.last_synced_unix_ms = unix_now_ms();
    insert(&reg, rec);

    reg.apply_resync_outcome("gone", ResyncOutcome::DeadConnection("reset".into()));
    {
        let map = reg.inner.peers.read();
        let r = &map["gone"];
        assert_eq!(r.state, PeerConnectionState::Disconnected);
        assert!(r.connection.is_none(), "dead handle must be dropped");
        assert_eq!(r.last_error.as_deref(), Some("reset"));
    }

    // Reconnect path is exercised: the peer is a re-dial candidate.
    let now = unix_now_ms();
    let targets = reg.collect_reconnect_targets(now + 1);
    assert_eq!(targets.len(), 1, "disconnected peer is a re-dial target");
    assert_eq!(targets[0].0, "gone");

    // A failed re-dial parks it back in Disconnected with a backoff
    // window that hides it from the next tick's candidate set.
    reg.note_reconnect_failure("gone");
    let after_one = reg.inner.peers.read()["gone"].reconnect_after_unix_ms;
    assert!(after_one > now, "backoff pushes the next attempt out");
    assert_eq!(state_of(&reg, "gone"), PeerConnectionState::Disconnected);
    assert!(
        reg.collect_reconnect_targets(now).is_empty(),
        "peer is not a candidate during its backoff window",
    );

    // Backoff grows with successive failures.
    reg.note_reconnect_failure("gone");
    let after_two = reg.inner.peers.read()["gone"].reconnect_after_unix_ms;
    assert!(after_two >= after_one, "backoff must not shrink");

    // A successful sync clears the backoff bookkeeping.
    reg.record_sync_success("gone");
    let map = reg.inner.peers.read();
    assert_eq!(map["gone"].reconnect_attempts, 0);
    assert_eq!(map["gone"].reconnect_after_unix_ms, 0);
}

#[test]
fn reconnect_backoff_grows_and_caps() {
    assert_eq!(reconnect_backoff(0), RECONNECT_BACKOFF_BASE);
    assert_eq!(reconnect_backoff(1), RECONNECT_BACKOFF_BASE);
    assert!(reconnect_backoff(2) > reconnect_backoff(1));
    assert!(reconnect_backoff(3) > reconnect_backoff(2));
    // Deep backoff saturates at the ceiling, never beyond it.
    assert_eq!(reconnect_backoff(1_000), RECONNECT_BACKOFF_MAX);
}

// Core regression: one peer wedged for the full per-peer timeout must
// not delay a concurrently-healthy peer. The wedged job's timeout
// (200ms here) runs alongside the healthy job's 10ms sync — the
// healthy peer settles first, an order of magnitude inside the
// timeout, even though it is listed AFTER the wedged one. (Short real
// timers rather than paused time, which needs tokio's `test-util`
// feature; the margin between 10ms and 200ms keeps it deterministic.)
#[tokio::test]
async fn wedged_peer_does_not_starve_healthy_peer() {
    let per_peer_timeout = Duration::from_millis(200);
    let jobs: Vec<(String, BoxResyncFuture)> = vec![
        (
            "wedged".to_string(),
            Box::pin(async {
                tokio::time::sleep(Duration::from_secs(3600)).await;
                ResyncOutcome::Ok
            }),
        ),
        (
            "healthy".to_string(),
            Box::pin(async {
                tokio::time::sleep(Duration::from_millis(10)).await;
                ResyncOutcome::Ok
            }),
        ),
    ];

    let results = run_bounded_round(jobs, per_peer_timeout, RESYNC_MAX_CONCURRENCY).await;

    let healthy = results.iter().find(|r| r.0 == "healthy").expect("healthy");
    let wedged = results.iter().find(|r| r.0 == "wedged").expect("wedged");

    assert!(matches!(healthy.1, ResyncOutcome::Ok));
    assert!(matches!(wedged.1, ResyncOutcome::TimedOut));
    assert!(
        healthy.2 < per_peer_timeout,
        "healthy settled at {:?}, not behind the wedged peer's {per_peer_timeout:?} timeout",
        healthy.2,
    );
    assert!(
        wedged.2 >= per_peer_timeout,
        "wedged peer is bounded by the per-peer timeout",
    );
}

// Dropping a round (as the worker's `resync_task.abort()` does at
// shutdown) must cancel every in-flight per-peer job.
#[tokio::test]
async fn aborting_a_round_cancels_in_flight_jobs() {
    let cancelled = Arc::new(AtomicBool::new(false));
    let completed = Arc::new(AtomicBool::new(false));

    struct CancelGuard(Arc<AtomicBool>);
    impl Drop for CancelGuard {
        fn drop(&mut self) {
            self.0.store(true, Ordering::SeqCst);
        }
    }

    let jobs: Vec<(String, BoxResyncFuture)> = {
        let cancelled = Arc::clone(&cancelled);
        let completed = Arc::clone(&completed);
        vec![(
            "wedged".to_string(),
            Box::pin(async move {
                let _guard = CancelGuard(cancelled);
                tokio::time::sleep(Duration::from_secs(3600)).await;
                completed.store(true, Ordering::SeqCst);
                ResyncOutcome::Ok
            }),
        )]
    };

    // A generous per-peer timeout so the abort — not the timeout — is
    // what ends the job.
    let handle = tokio::spawn(run_bounded_round(
        jobs,
        Duration::from_secs(3600),
        RESYNC_MAX_CONCURRENCY,
    ));
    // Let the job start and park on its sleep.
    tokio::time::sleep(Duration::from_millis(20)).await;
    handle.abort();
    let _ = handle.await;

    // Drive the runtime so the dropped JoinSet reaps its children.
    for _ in 0..100 {
        if cancelled.load(Ordering::SeqCst) {
            break;
        }
        tokio::time::sleep(Duration::from_millis(2)).await;
    }

    assert!(
        cancelled.load(Ordering::SeqCst),
        "in-flight job must be dropped when the round is aborted",
    );
    assert!(
        !completed.load(Ordering::SeqCst),
        "aborted job must not run to completion",
    );
}
