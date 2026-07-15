//! Sessions-active register projection: STARTED/UPDATED merge only
//! descriptive attributes (daemon-owned clocks and status are stamped by
//! the reducer), ENDED removes the entry, the per-producer status reducer
//! honors causal revisions and cross-producer precedence, and the atomic
//! read-modify-write barrier prevents UPDATE-after-END resurrection and
//! START/END lost updates.

use super::*;

#[tokio::test]
async fn session_events_drive_the_active_register() {
    let h = harness();

    // START carries descriptive attrs + the typed LIFECYCLE Active
    // slot (the wrapper's initial contribution). The daemon projects
    // status:"active" from that slot.
    let response = h
        .service
        .report_session_event(Request::new(rendezvous_core::ReportSessionEventRequest {
            session_id: "sess-1".into(),
            kind: rendezvous_core::SessionEventKind::Started as i32,
            details_json: r#"{"agent":"claude"}"#.into(),
            status: Some(proto_status(
                rendezvous_core::SessionStatusState::Active,
                rendezvous_core::StatusProducer::Lifecycle,
                rendezvous_core::SessionStatusBasis::Lifecycle,
                1,
            )),
        }))
        .await
        .expect("start")
        .into_inner();
    assert_eq!(response.active_count, 1);

    // UPDATE routes a typed SINK waiting_on_user opinion into the
    // sink slot; the reducer projects it as the flat status fields.
    h.service
        .report_session_event(Request::new(rendezvous_core::ReportSessionEventRequest {
            session_id: "sess-1".into(),
            kind: rendezvous_core::SessionEventKind::Updated as i32,
            details_json: String::new(),
            status: Some(proto_status(
                rendezvous_core::SessionStatusState::WaitingOnUser,
                rendezvous_core::StatusProducer::Sink,
                rendezvous_core::SessionStatusBasis::PermissionAsk,
                2,
            )),
        }))
        .await
        .expect("update");

    let hosts = h
        .service
        .list_active_sessions(Request::new(rendezvous_core::ListActiveSessionsRequest {}))
        .await
        .expect("list")
        .into_inner()
        .hosts;
    assert_eq!(hosts.len(), 1);
    let sessions: serde_json::Value =
        serde_json::from_str(&hosts[0].sessions_json).expect("sessions json");
    let entry = &sessions["sess-1"];
    assert_eq!(entry["agent"], serde_json::json!("claude"));
    assert_eq!(entry["status"], serde_json::json!("waiting_on_user"));
    assert_eq!(entry["status_basis"], serde_json::json!("permission_ask"));
    assert_eq!(entry["status_producer"], serde_json::json!("sink"));
    assert!(entry["started_at_unix_ms"].is_i64(), "entry: {entry}");
    assert!(entry["updated_at_unix_ms"].is_i64(), "entry: {entry}");

    // END removes the entry — the register holds only live sessions.
    let response = h
        .service
        .report_session_event(Request::new(rendezvous_core::ReportSessionEventRequest {
            session_id: "sess-1".into(),
            kind: rendezvous_core::SessionEventKind::Ended as i32,
            details_json: String::new(),
            status: None,
        }))
        .await
        .expect("end")
        .into_inner();
    assert_eq!(response.active_count, 0);

    // Ending an unknown session is a harmless no-op.
    let response = h
        .service
        .report_session_event(Request::new(rendezvous_core::ReportSessionEventRequest {
            session_id: "never-started".into(),
            kind: rendezvous_core::SessionEventKind::Ended as i32,
            details_json: String::new(),
            status: None,
        }))
        .await
        .expect("end unknown")
        .into_inner();
    assert_eq!(response.active_count, 0);
}

#[tokio::test]
async fn updated_on_missing_session_does_not_resurrect_it() {
    let h = harness();

    // An UPDATE for a session that was never STARTED (or already
    // ENDED) must not create an entry — otherwise a late,
    // fire-and-forget status report could race a completed ENDED
    // and leave a phantom live session in the register.
    let response = h
        .service
        .report_session_event(Request::new(rendezvous_core::ReportSessionEventRequest {
            session_id: "ghost".into(),
            kind: rendezvous_core::SessionEventKind::Updated as i32,
            details_json: String::new(),
            status: Some(proto_status(
                rendezvous_core::SessionStatusState::WaitingOnUser,
                rendezvous_core::StatusProducer::Sink,
                rendezvous_core::SessionStatusBasis::PermissionAsk,
                1,
            )),
        }))
        .await
        .expect("update missing")
        .into_inner();
    assert_eq!(response.active_count, 0, "UPDATE must not create a session");

    let hosts = h
        .service
        .list_active_sessions(Request::new(rendezvous_core::ListActiveSessionsRequest {}))
        .await
        .expect("list")
        .into_inner()
        .hosts;
    let empty = hosts.is_empty()
        || serde_json::from_str::<serde_json::Value>(&hosts[0].sessions_json)
            .expect("sessions json")
            .as_object()
            .is_some_and(serde_json::Map::is_empty);
    assert!(empty, "register must hold no sessions: {hosts:?}");
}

/// Build a bare `RegisterStore` for the sync barrier tests: they
/// exercise `apply_session_event` directly (the daemon runs it in
/// `spawn_blocking`) so they need no gRPC service.
fn register_store() -> (crate::register::RegisterStore, TempDir) {
    let tmp = TempDir::new().expect("tempdir");
    let storage = Storage::open(tmp.path().join("registers.redb")).expect("storage");
    let identity = Arc::new(NodeIdentity::from_seed([7u8; 32]));
    let store = crate::register::RegisterStore::new(storage, identity).expect("store");
    (store, tmp)
}

fn proto_status(
    state: rendezvous_core::SessionStatusState,
    producer: rendezvous_core::StatusProducer,
    basis: rendezvous_core::SessionStatusBasis,
    revision: i64,
) -> rendezvous_core::proto::StatusContribution {
    rendezvous_core::proto::StatusContribution {
        state: state as i32,
        producer: producer as i32,
        basis: basis as i32,
        revision,
    }
}

fn details(pairs: &[(&str, &str)]) -> serde_json::Map<String, serde_json::Value> {
    pairs
        .iter()
        .map(|(k, v)| ((*k).to_string(), serde_json::json!(v)))
        .collect()
}

fn session_present(store: &crate::register::RegisterStore, id: &str) -> bool {
    store
        .deep_value(&store.local_sessions_active_id())
        .expect("read")
        .and_then(|v| v.as_object().map(|m| m.contains_key(id)))
        .unwrap_or(false)
}

// Barrier: START-A committed, then UPDATE-A and END-A released
// simultaneously. Under the atomic primitive the absence check and
// the removal share one held lock, so neither interleaving can
// resurrect A. Repeated to prove causality, not luck; each round
// uses a fresh id so leftover state never masks a resurrection.
#[test]
fn concurrent_update_and_end_never_resurrect() {
    use rendezvous_core::SessionEventKind as Kind;
    use std::sync::Barrier;
    let (store, _tmp) = register_store();
    for round in 0..100 {
        let id = format!("A-{round}");
        apply_session_event(&store, &id, Kind::Started, details(&[("agent", "claude")]), None)
            .expect("start A");

        let barrier = Arc::new(Barrier::new(2));
        let s1 = store.clone();
        let b1 = Arc::clone(&barrier);
        let id1 = id.clone();
        let upd = std::thread::spawn(move || {
            b1.wait();
            let _ = apply_session_event(
                &s1,
                &id1,
                Kind::Updated,
                details(&[("model", "opus")]),
                None,
            );
        });
        let s2 = store.clone();
        let b2 = Arc::clone(&barrier);
        let id2 = id.clone();
        let end = std::thread::spawn(move || {
            b2.wait();
            let _ = apply_session_event(&s2, &id2, Kind::Ended, serde_json::Map::new(), None);
        });
        upd.join().expect("upd join");
        end.join().expect("end join");

        assert!(
            !session_present(&store, &id),
            "UPDATE racing END resurrected session {id}",
        );
    }
}

// Barrier: START-A committed, then END-A and START-B released
// simultaneously. END recomputes the map from the live register, so
// START-B is never lost and A is always removed.
#[test]
fn concurrent_end_and_start_new_session_no_lost_update() {
    use rendezvous_core::SessionEventKind as Kind;
    use std::sync::Barrier;
    let (store, _tmp) = register_store();
    for round in 0..100 {
        let a = format!("A-{round}");
        let b = format!("B-{round}");
        apply_session_event(&store, &a, Kind::Started, details(&[("agent", "a")]), None)
            .expect("start A");

        let barrier = Arc::new(Barrier::new(2));
        let s1 = store.clone();
        let b1 = Arc::clone(&barrier);
        let a1 = a.clone();
        let end = std::thread::spawn(move || {
            b1.wait();
            let _ = apply_session_event(&s1, &a1, Kind::Ended, serde_json::Map::new(), None);
        });
        let s2 = store.clone();
        let b2 = Arc::clone(&barrier);
        let b_start = b.clone();
        let start_b = std::thread::spawn(move || {
            b2.wait();
            let _ = apply_session_event(
                &s2,
                &b_start,
                Kind::Started,
                details(&[("agent", "b")]),
                None,
            );
        });
        end.join().expect("end join");
        start_b.join().expect("start B join");

        assert!(session_present(&store, &b), "START-B was lost to END-A");
        assert!(!session_present(&store, &a), "END-A did not remove A");
    }
}

fn core_status(
    state: rendezvous_core::session_status::SessionState,
    producer: rendezvous_core::session_status::ProducerId,
    basis: rendezvous_core::session_status::Basis,
    revision: i64,
) -> rendezvous_core::session_status::StatusContribution {
    rendezvous_core::session_status::StatusContribution {
        state,
        producer,
        basis,
        revision,
    }
}

/// Read a projected field out of a session's stored entry.
fn entry_field(
    store: &crate::register::RegisterStore,
    id: &str,
    field: &str,
) -> Option<serde_json::Value> {
    store
        .deep_value(&store.local_sessions_active_id())
        .ok()
        .flatten()
        .and_then(|v| v.get(id).and_then(|e| e.as_str()).map(str::to_string))
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
        .and_then(|entry| entry.get(field).cloned())
}

// Finding 2: a late, reordered same-producer update carries the
// LOWER revision and is rejected, so the causally-latest opinion
// wins. Repeated ×50 to prove it is causal, not lucky.
#[test]
fn reordered_same_producer_updates_are_causal() {
    use rendezvous_core::SessionEventKind as Kind;
    use rendezvous_core::session_status::{Basis, ProducerId, SessionState};
    let (store, _tmp) = register_store();
    for round in 0..50 {
        let id = format!("s-{round}");
        apply_session_event(
            &store,
            &id,
            Kind::Started,
            details(&[("agent", "claude")]),
            Some(core_status(
                SessionState::Active,
                ProducerId::Lifecycle,
                Basis::Lifecycle,
                1,
            )),
        )
        .expect("start");
        // The sink emits waiting_on_user then active in program order.
        apply_session_event(
            &store,
            &id,
            Kind::Updated,
            serde_json::Map::new(),
            Some(core_status(
                SessionState::WaitingOnUser,
                ProducerId::Sink,
                Basis::PermissionAsk,
                10,
            )),
        )
        .expect("waiting");
        apply_session_event(
            &store,
            &id,
            Kind::Updated,
            serde_json::Map::new(),
            Some(core_status(SessionState::Active, ProducerId::Sink, Basis::Lifecycle, 20)),
        )
        .expect("active");
        // The late detached task re-delivers the stale waiting@10.
        apply_session_event(
            &store,
            &id,
            Kind::Updated,
            serde_json::Map::new(),
            Some(core_status(
                SessionState::WaitingOnUser,
                ProducerId::Sink,
                Basis::PermissionAsk,
                10,
            )),
        )
        .expect("late waiting");

        assert_eq!(
            entry_field(&store, &id, "status"),
            Some(serde_json::json!("active")),
            "reordered stale waiting_on_user must not win (round {round})",
        );
    }
}

// Finding 4: a weaker idle from a DIFFERENT producer, arriving with a
// higher revision, must not lower a live waiting_on_user. Revisions
// are incomparable across producers; precedence decides.
#[test]
fn cross_producer_precedence_keeps_strongest() {
    use rendezvous_core::SessionEventKind as Kind;
    use rendezvous_core::session_status::{Basis, ProducerId, SessionState};
    let (store, _tmp) = register_store();
    apply_session_event(
        &store,
        "s",
        Kind::Started,
        details(&[("agent", "claude")]),
        Some(core_status(SessionState::Active, ProducerId::Lifecycle, Basis::Lifecycle, 1)),
    )
    .expect("start");
    apply_session_event(
        &store,
        "s",
        Kind::Updated,
        serde_json::Map::new(),
        Some(core_status(
            SessionState::WaitingOnUser,
            ProducerId::Sink,
            Basis::PermissionAsk,
            10,
        )),
    )
    .expect("sink waiting");
    // Idle hook fires later (higher revision) but is a weaker state.
    apply_session_event(
        &store,
        "s",
        Kind::Updated,
        serde_json::Map::new(),
        Some(core_status(
            SessionState::Idle,
            ProducerId::IdleHook,
            Basis::InteractiveTurnComplete,
            999,
        )),
    )
    .expect("idle hook");

    assert_eq!(
        entry_field(&store, "s", "status"),
        Some(serde_json::json!("waiting_on_user")),
    );
    assert_eq!(
        entry_field(&store, "s", "status_producer"),
        Some(serde_json::json!("sink")),
    );
}

// Phase 4: the hook (PermissionHook) and the fallback sink both
// report waiting_on_user for one session. Two slots are recorded and
// the effective status is a single waiting_on_user — redundant, never
// conflicting.
#[test]
fn two_producers_waiting_records_both_slots() {
    use rendezvous_core::SessionEventKind as Kind;
    use rendezvous_core::session_status::{self, Basis, ProducerId, SessionState};
    let (store, _tmp) = register_store();
    apply_session_event(
        &store,
        "s",
        Kind::Started,
        details(&[("agent", "claude")]),
        Some(core_status(SessionState::Active, ProducerId::Lifecycle, Basis::Lifecycle, 1)),
    )
    .expect("start");
    apply_session_event(
        &store,
        "s",
        Kind::Updated,
        serde_json::Map::new(),
        Some(core_status(
            SessionState::WaitingOnUser,
            ProducerId::PermissionHook,
            Basis::PermissionAsk,
            10,
        )),
    )
    .expect("hook waiting");
    apply_session_event(
        &store,
        "s",
        Kind::Updated,
        serde_json::Map::new(),
        Some(core_status(
            SessionState::WaitingOnUser,
            ProducerId::Sink,
            Basis::PermissionAsk,
            11,
        )),
    )
    .expect("sink waiting");

    assert_eq!(
        entry_field(&store, "s", "status"),
        Some(serde_json::json!("waiting_on_user")),
    );
    let slots = entry_field(&store, "s", "status_slots").expect("slots present");
    let parsed = session_status::slots_from_json(&slots);
    assert!(parsed.contains_key(&ProducerId::PermissionHook));
    assert!(parsed.contains_key(&ProducerId::Sink));
    assert!(parsed.contains_key(&ProducerId::Lifecycle));
}

// The daemon owns clocks + status: a payload that tries to set
// reserved keys via details_json is stripped before merge.
#[test]
fn reserved_keys_in_details_are_ignored() {
    use rendezvous_core::SessionEventKind as Kind;
    use rendezvous_core::session_status::{Basis, ProducerId, SessionState};
    let (store, _tmp) = register_store();
    let mut forged = serde_json::Map::new();
    forged.insert("agent".into(), serde_json::json!("claude"));
    forged.insert("started_at_unix_ms".into(), serde_json::json!(999));
    forged.insert("status".into(), serde_json::json!("hacked"));
    apply_session_event(
        &store,
        "s",
        Kind::Started,
        forged,
        Some(core_status(SessionState::Active, ProducerId::Lifecycle, Basis::Lifecycle, 1)),
    )
    .expect("start");

    assert_eq!(entry_field(&store, "s", "agent"), Some(serde_json::json!("claude")));
    assert_ne!(
        entry_field(&store, "s", "started_at_unix_ms"),
        Some(serde_json::json!(999)),
        "producer must not forge the daemon-owned clock",
    );
    assert_eq!(
        entry_field(&store, "s", "status"),
        Some(serde_json::json!("active")),
        "producer must not forge status; reducer is authoritative",
    );
}

// STARTED with the LIFECYCLE Active slot projects the backward-
// compatible status:"active" the dashboard reads.
#[test]
fn started_projects_active_from_lifecycle_slot() {
    use rendezvous_core::SessionEventKind as Kind;
    use rendezvous_core::session_status::{Basis, ProducerId, SessionState};
    let (store, _tmp) = register_store();
    apply_session_event(
        &store,
        "s",
        Kind::Started,
        details(&[("agent", "claude")]),
        Some(core_status(SessionState::Active, ProducerId::Lifecycle, Basis::Lifecycle, 1)),
    )
    .expect("start");
    assert_eq!(entry_field(&store, "s", "status"), Some(serde_json::json!("active")));
    assert_eq!(
        entry_field(&store, "s", "status_producer"),
        Some(serde_json::json!("lifecycle")),
    );
}
