//! Provider-neutral process-outcome mapping and detector/watchdog conversions.

use super::*;

#[test]
fn early_termination_process_outcome_maps_step_timeout_to_timed_out() {
    let termination = EarlyTermination::StepTimeout {
        message: "no stream activity for 6s; terminating due to step_timeout".into(),
        outstanding: Vec::new(),
    };

    let outcome = early_termination_process_outcome(Some(&termination));

    assert_eq!(outcome, claudine::harness::ProcessTermination::TimedOut);
}

#[test]
fn early_termination_process_outcome_maps_timeout_to_timed_out() {
    let termination = EarlyTermination::Timeout {
        message: "wall-clock budget exceeded after 2h".into(),
    };

    let outcome = early_termination_process_outcome(Some(&termination));

    assert_eq!(outcome, claudine::harness::ProcessTermination::TimedOut);
}

#[test]
fn early_termination_process_outcome_maps_exit_expression_to_aborted() {
    let termination = EarlyTermination::ExitExpression {
        pattern: "STOP.".into(),
        scope: None,
    };

    let outcome = early_termination_process_outcome(Some(&termination));

    assert_eq!(outcome, claudine::harness::ProcessTermination::Aborted);
}

#[test]
fn early_termination_process_outcome_maps_runaway_repetition_to_aborted() {
    let termination = EarlyTermination::RunawayRepetition {
        cycle_len: 6,
        repeats: 30,
    };

    let outcome = early_termination_process_outcome(Some(&termination));

    assert_eq!(outcome, claudine::harness::ProcessTermination::Aborted);
}

#[test]
fn early_termination_process_outcome_maps_runaway_volume_to_aborted() {
    let termination = EarlyTermination::RunawayVolume {
        lines: 50_001,
        bytes: 32 * 1024 * 1024 + 1,
    };

    let outcome = early_termination_process_outcome(Some(&termination));

    assert_eq!(outcome, claudine::harness::ProcessTermination::Aborted);
}

#[test]
fn early_termination_process_outcome_maps_stalled_generation_to_aborted() {
    // Fail-fast: a stalled-generation loop must never route through
    // `TimedOut` / `handle_timeout:` (which would re-run the provider and
    // reproduce the silent generation-drop stall).
    let termination = EarlyTermination::StalledGeneration {
        generation_count: 4,
        stall_duration: Duration::from_secs(600),
        context: StalledGenerationContext::default(),
    };

    let outcome = early_termination_process_outcome(Some(&termination));

    assert_eq!(outcome, claudine::harness::ProcessTermination::Aborted);
}

#[test]
fn trip_to_early_termination_preserves_exit_expression_fields() {
    let trip = claudine::runaway::Trip::ExitExpression {
        pattern: "STOP.".into(),
        scope: Some("opencode/kimi-for-coding/k2p7".into()),
    };

    let early = trip_to_early_termination(trip);

    match early {
        EarlyTermination::ExitExpression { pattern, scope } => {
            assert_eq!(pattern, "STOP.");
            assert_eq!(scope.as_deref(), Some("opencode/kimi-for-coding/k2p7"));
        }
        other => panic!("expected ExitExpression, got {other:?}"),
    }
}

#[test]
fn trip_to_early_termination_preserves_runaway_repetition_fields() {
    let trip = claudine::runaway::Trip::RunawayRepetition {
        cycle_len: 6,
        repeats: 30,
    };

    let early = trip_to_early_termination(trip);

    match early {
        EarlyTermination::RunawayRepetition { cycle_len, repeats } => {
            assert_eq!(cycle_len, 6);
            assert_eq!(repeats, 30);
        }
        other => panic!("expected RunawayRepetition, got {other:?}"),
    }
}

#[test]
fn trip_to_early_termination_preserves_runaway_volume_fields() {
    let trip = claudine::runaway::Trip::RunawayVolume {
        lines: 50_001,
        bytes: 33_554_432,
    };

    let early = trip_to_early_termination(trip);

    match early {
        EarlyTermination::RunawayVolume { lines, bytes } => {
            assert_eq!(lines, 50_001);
            assert_eq!(bytes, 33_554_432);
        }
        other => panic!("expected RunawayVolume, got {other:?}"),
    }
}

#[test]
fn watchdog_request_to_early_termination_maps_timeout_reason() {
    let req = WatchdogTermination {
        reason: WatchdogTerminationReason::Timeout,
        message: "wall-clock budget exceeded".into(),
        stuck_subagents: Vec::new(),
    };

    let early = watchdog_request_to_early_termination(req);
    assert!(matches!(
        early,
        EarlyTermination::Timeout { ref message } if message == "wall-clock budget exceeded"
    ));
}

#[test]
fn watchdog_request_to_early_termination_carries_stuck_subagents() {
    use crate::commands::wrap::exec::subagent_watchdog::ActiveSubagentSnapshot;

    let now = Instant::now();
    let snapshot = ActiveSubagentSnapshot {
        id: "ses_a".into(),
        name: Some("Commit feature work".into()),
        started_at: now,
        last_progress_at: now,
        elapsed_since_start: Duration::from_secs(900),
        elapsed_since_progress: Duration::from_secs(900),
    };
    let req = WatchdogTermination {
        reason: WatchdogTerminationReason::StepTimeout,
        message: "no stream activity for 30m".into(),
        stuck_subagents: vec![snapshot],
    };

    let early = watchdog_request_to_early_termination(req);
    let outstanding = match early {
        EarlyTermination::StepTimeout { outstanding, .. } => outstanding,
        other => panic!("expected StepTimeout, got {other:?}"),
    };
    assert_eq!(outstanding.len(), 1);
    assert_eq!(outstanding[0].id, "ses_a");
    assert_eq!(outstanding[0].name.as_deref(), Some("Commit feature work"));
    assert_eq!(
        outstanding[0].elapsed_since_progress,
        Duration::from_secs(900)
    );
}
