//! Ledger arithmetic, state transitions, and persistence.
//!
//! End-to-end enforcement through a real `claudine sequence` lives in
//! `cli/tests/l1/sequence_budget.rs`.

use chrono::{DateTime, TimeZone, Utc};

use super::error::BudgetError;
use super::model::{
    Allowance, CloseOutcome, EventKind, Ledger, LedgerState, OrphanOutcome,
};
use super::store::{self, LedgerFile};

fn at(seconds: i64) -> DateTime<Utc> {
    Utc.timestamp_opt(1_800_000_000 + seconds, 0).unwrap()
}

fn limits(invocations: u64, active_ms: u64) -> Allowance {
    Allowance {
        invocations,
        active_ms,
    }
}

fn ledger(invocations: u64, active_ms: u64) -> Ledger {
    Ledger::new("2026-09-17-0000abcd", "discord", limits(invocations, active_ms), 1_000, None, at(0))
        .unwrap()
}

fn kinds(ledger: &Ledger) -> Vec<EventKind> {
    ledger.events.iter().map(|event| event.kind).collect()
}

#[test]
fn a_ledger_requires_both_positive_limits_and_an_identity() {
    for (invocations, active_ms) in [(0, 1_000), (3, 0), (0, 0)] {
        let err = Ledger::new("run", "discord", limits(invocations, active_ms), 1_000, None, at(0))
            .unwrap_err();
        assert!(matches!(err, BudgetError::Invalid(_)), "{invocations}/{active_ms}: {err}");
    }
    for (run_id, platform) in [("", "discord"), ("run", " ")] {
        assert!(Ledger::new(run_id, platform, limits(1, 1), 1_000, None, at(0)).is_err());
    }
    let created = ledger(3, 60_000);
    assert_eq!(created.state, LedgerState::Stopped);
    assert_eq!(created.used, Allowance::default());
    assert_eq!(kinds(&created), vec![EventKind::Initialized]);
}

#[test]
fn every_admission_debits_one_invocation_before_spawn_and_settling_never_refunds() {
    let mut ledger = ledger(2, 60_000);
    ledger.open(at(0)).unwrap();
    assert!(!ledger.enter_stage("discovery", 100, at(1)));

    let first = ledger.admit(1, 200, at(2)).unwrap();
    assert_eq!(first.invocation, 1);
    assert_eq!(ledger.used, limits(1, 300));
    assert_eq!(first.remaining_ms, 59_700);
    assert_eq!(ledger.in_flight.len(), 1);
    assert_eq!(ledger.in_flight[0].stage.as_deref(), Some("discovery"));

    ledger.record_child(1, 4242, Some(99), at(3));
    assert_eq!(ledger.in_flight[0].pid, Some(4242));
    ledger.settle(1, 1_000, at(4));
    assert!(ledger.in_flight.is_empty());
    assert_eq!(ledger.used, limits(1, 1_300), "settling keeps the invocation charged");

    // A retry of the same stage is a new launch and a new debit.
    let second = ledger.admit(2, 0, at(5)).unwrap();
    assert_eq!(second.invocation, 2);
    ledger.settle(2, 0, at(6));

    // The limit is reached: the next launch is refused and nothing is charged.
    let refused = ledger.admit(3, 0, at(7)).unwrap_err();
    assert!(matches!(refused, BudgetError::Exhausted(_)), "{refused}");
    assert_eq!(ledger.used.invocations, 2);
    assert!(ledger.in_flight.is_empty());
    assert!(ledger.exhaustion_noted());
    assert!(refused.to_string().contains("stage `discovery`"), "{refused}");
    assert!(refused.to_string().contains("2/2 invocations"), "{refused}");
}

#[test]
fn elapsed_time_exhausts_at_a_stage_boundary_and_is_recorded_once() {
    let mut ledger = ledger(10, 1_000);
    ledger.open(at(0)).unwrap();
    assert!(!ledger.enter_stage("discovery", 400, at(1)));
    // Automatic waits and orchestration are charged the same as agent time.
    ledger.heartbeat(600, at(2));
    assert!(ledger.enter_stage("reconcile", 0, at(3)));
    assert!(!ledger.note_exhaustion(at(4)), "a second note is a no-op");
    let exhausted = ledger
        .events
        .iter()
        .filter(|event| event.kind == EventKind::Exhausted)
        .count();
    assert_eq!(exhausted, 1);

    ledger.close(&CloseOutcome::Failed(130), 50, at(5));
    assert_eq!(ledger.state, LedgerState::Exhausted);
    assert_eq!(ledger.stage.as_deref(), Some("reconcile"), "the incomplete stage is kept");
    assert_eq!(ledger.used.active_ms, 1_050, "time past the limit is still charged");
    assert!(ledger.segment.is_none());
}

#[test]
fn close_chooses_the_resting_state_from_the_outcome() {
    let cases = [
        (CloseOutcome::Completed, LedgerState::Suspended, "awaiting human review"),
        (CloseOutcome::Failed(1), LedgerState::Stopped, "sequence exited with status 1"),
        (CloseOutcome::Interrupted, LedgerState::Stopped, "interrupted by the operator"),
        (CloseOutcome::Error("boom".into()), LedgerState::Stopped, "boom"),
    ];
    for (outcome, state, reason) in cases {
        let mut ledger = ledger(5, 60_000);
        ledger.open(at(0)).unwrap();
        ledger.admit(1, 10, at(1)).unwrap();
        ledger.close(&outcome, 10, at(2));
        assert_eq!(ledger.state, state, "{outcome:?}");
        assert_eq!(ledger.stop_reason.as_deref(), Some(reason));
        assert!(ledger.in_flight.is_empty());
        assert_eq!(ledger.used, limits(1, 20));
    }
}

#[test]
fn only_a_stopped_ledger_opens_and_suspension_requires_an_operator_resume() {
    let mut ledger = ledger(5, 60_000);
    ledger.suspend("awaiting approval", at(0)).unwrap();
    let refused = ledger.open(at(1)).unwrap_err();
    assert!(matches!(refused, BudgetError::NotRunnable { state: LedgerState::Suspended, .. }));
    assert_eq!(ledger.runs, 0);

    ledger.resume("ken", at(2)).unwrap();
    assert_eq!(ledger.state, LedgerState::Stopped);
    assert!(ledger.resume("ken", at(3)).is_err(), "a stopped ledger has nothing to resume");
    ledger.open(at(4)).unwrap();
    assert_eq!(ledger.runs, 1);
    assert!(ledger.suspend("x", at(5)).is_err(), "an active run cannot be suspended from outside");
    assert!(ledger.grant("ken", "more", limits(1, 0), at(5)).is_err());
}

#[test]
fn crash_recovery_charges_the_unseen_tail_and_requires_resumption() {
    let mut ledger = ledger(5, 60_000);
    ledger.open(at(0)).unwrap();
    ledger.admit(1, 0, at(1)).unwrap();
    ledger.record_child(1, 4242, Some(7), at(1));
    ledger.heartbeat(2_000, at(3));
    let before = ledger.used;

    ledger.recover_crash(&OrphanOutcome::NotRunning, at(100));
    assert_eq!(ledger.state, LedgerState::Interrupted);
    assert_eq!(ledger.used.invocations, before.invocations, "no refund");
    assert_eq!(
        ledger.used.active_ms,
        before.active_ms + 1_000,
        "one heartbeat past the last heartbeat; downtime is operator time"
    );
    assert!(ledger.in_flight.is_empty());
    assert!(ledger.open(at(101)).is_err(), "restart is refused until resumed");

    ledger.resume("ken", at(102)).unwrap();
    ledger.open(at(103)).unwrap();
    assert_eq!(ledger.runs, 2, "a restart is a new run, not a reset");
    assert_eq!(ledger.used.invocations, 1);
}

#[test]
fn a_terminated_orphan_charges_the_time_it_kept_working() {
    let mut ledger = ledger(5, 600_000);
    ledger.open(at(0)).unwrap();
    ledger.heartbeat(0, at(10));
    ledger.recover_crash(&OrphanOutcome::Terminated, at(70));
    assert_eq!(ledger.used.active_ms, 60_000);
    assert!(ledger.stop_reason.as_deref().unwrap().contains("terminated"));
}

#[test]
fn only_a_recorded_grant_reopens_an_exhausted_ledger() {
    let mut ledger = ledger(1, 60_000);
    ledger.open(at(0)).unwrap();
    ledger.admit(1, 0, at(1)).unwrap();
    ledger.settle(1, 0, at(2));
    ledger.close(&CloseOutcome::Failed(1), 0, at(3));
    assert_eq!(ledger.state, LedgerState::Exhausted);
    assert!(matches!(ledger.open(at(4)), Err(BudgetError::NotRunnable { .. })));
    assert!(ledger.resume("ken", at(4)).is_err(), "resume cannot bypass exhaustion");

    assert!(ledger.grant("", "reason", limits(1, 0), at(5)).is_err());
    assert!(ledger.grant("ken", "", limits(1, 0), at(5)).is_err());
    assert!(ledger.grant("ken", "nothing", limits(0, 0), at(5)).is_err());
    // Time alone does not reopen an invocation-exhausted ledger.
    ledger.grant("ken", "more time", limits(0, 5_000), at(6)).unwrap();
    assert_eq!(ledger.state, LedgerState::Exhausted);
    ledger.grant("ken", "one recovery", limits(1, 0), at(7)).unwrap();
    assert_eq!(ledger.state, LedgerState::Stopped);
    assert_eq!(ledger.allowed(), limits(2, 65_000));
    assert_eq!(ledger.used.invocations, 1, "a grant never resets consumption");
    assert_eq!(ledger.grants.len(), 2);
}

#[test]
fn a_ledger_round_trips_through_disk_byte_for_byte() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("runs/discord/run/budget.json");
    let mut record = ledger(3, 60_000);
    record.open(at(0)).unwrap();
    record.admit(1, 5, at(1)).unwrap();
    let file = LedgerFile::create(&path, &record).unwrap();
    let first = std::fs::read(&path).unwrap();
    let loaded = store::read(&path).unwrap();
    assert_eq!(loaded, record);
    file.write(&loaded).unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), first);
    assert!(first.ends_with(b"}\n"));
    assert!(!first.contains(&b'\r'));
}

#[test]
fn loading_rejects_missing_or_zero_limits_and_foreign_formats() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("budget.json");
    let valid = serde_json::to_value(ledger(3, 60_000)).unwrap();

    let mut missing = valid.clone();
    missing["limits"].as_object_mut().unwrap().remove("active_ms");
    let mut zero = valid.clone();
    zero["limits"]["invocations"] = 0.into();
    let mut foreign = valid.clone();
    foreign["format"] = "claudine-budget-ledger/2".into();
    let mut unknown = valid;
    unknown["limit_overrides"] = 1.into();

    for (label, document) in [("missing", missing), ("zero", zero), ("foreign", foreign), ("unknown", unknown)] {
        std::fs::write(&path, serde_json::to_vec(&document).unwrap()).unwrap();
        assert!(store::read(&path).is_err(), "{label} must be refused");
    }
}

#[test]
fn a_held_ledger_or_exclusive_lock_refuses_a_second_holder() {
    let dir = tempfile::tempdir().unwrap();
    let discord = dir.path().join("runs/discord/a/budget.json");
    let slack = dir.path().join("runs/slack/b/budget.json");
    let shared = Some("../../../fleet.lock".to_string());
    let make = |platform: &str| {
        Ledger::new("run", platform, limits(1, 1_000), 1_000, shared.clone(), at(0)).unwrap()
    };
    drop(LedgerFile::create(&discord, &make("discord")).unwrap());
    drop(LedgerFile::create(&slack, &make("slack")).unwrap());
    assert!(matches!(
        LedgerFile::create(&discord, &make("discord")),
        Err(BudgetError::AlreadyExists { .. })
    ));

    let (mut held, record) = LedgerFile::acquire(&discord).unwrap();
    assert!(matches!(LedgerFile::acquire(&discord), Err(BudgetError::Locked { .. })));
    held.lock_exclusive(&record).unwrap();
    assert!(dir.path().join("fleet.lock").is_file());

    let (mut other, other_record) = LedgerFile::acquire(&slack).unwrap();
    assert!(matches!(
        other.lock_exclusive(&other_record),
        Err(BudgetError::Locked { .. })
    ));
    drop(held);
    other.lock_exclusive(&other_record).unwrap();
}
