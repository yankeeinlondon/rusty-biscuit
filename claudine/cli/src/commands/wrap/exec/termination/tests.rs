use super::*;
use claudine::stream::summary::StreamExecutionSummary;

#[test]
fn apply_early_termination_rate_limit_sets_usage_limit_summary_fields() {
    use chrono::TimeZone;
    let reset_at = chrono::Utc
        .with_ymd_and_hms(2026, 4, 16, 4, 18, 56)
        .unwrap();
    let mut summary = StreamExecutionSummary {
        exit_code: 143,
        is_error: false,
        ..Default::default()
    };
    let termination = EarlyTermination::RateLimit {
        message: "Usage limit reached; resets at 2026-04-16 04:18:56 UTC".into(),
        reset_at: Some(reset_at),
    };

    apply_early_termination_to_summary(&mut summary, &termination);

    assert_eq!(summary.exit_code, 1);
    assert!(summary.is_error);
    assert_eq!(summary.error_kind.as_deref(), Some("usage_limit_reached"));
    assert!(
        summary
            .error_message
            .as_deref()
            .unwrap_or("")
            .to_lowercase()
            .contains("usage limit"),
    );
    let rl = summary.rate_limit.as_ref().expect("rate_limit populated");
    assert_eq!(rl.is_throttled, Some(true));
    assert_eq!(rl.reset_at, Some(reset_at));
    assert!(
        rl.message
            .as_deref()
            .unwrap_or("")
            .to_lowercase()
            .contains("usage limit")
    );
}

#[test]
fn apply_early_termination_preserves_existing_rate_limit_fields() {
    use chrono::TimeZone;
    let existing_reset = chrono::Utc.with_ymd_and_hms(2026, 4, 16, 2, 0, 0).unwrap();
    let mut summary = StreamExecutionSummary {
        rate_limit: Some(claudine::stream::summary::RateLimitInfo {
            is_throttled: Some(false),
            retry_after_ms: Some(5000),
            message: Some("pre-existing".into()),
            reset_at: Some(existing_reset),
        }),
        ..Default::default()
    };
    let incoming_reset = chrono::Utc
        .with_ymd_and_hms(2026, 4, 16, 4, 18, 56)
        .unwrap();
    let termination = EarlyTermination::RateLimit {
        message: "Usage limit reached".into(),
        reset_at: Some(incoming_reset),
    };

    apply_early_termination_to_summary(&mut summary, &termination);

    let rl = summary.rate_limit.as_ref().unwrap();
    // is_throttled is forced to true even when existing said false.
    assert_eq!(rl.is_throttled, Some(true));
    // Existing message is preserved.
    assert_eq!(rl.message.as_deref(), Some("pre-existing"));
    // Existing reset_at is preserved (do not clobber parser-provided state).
    assert_eq!(rl.reset_at, Some(existing_reset));
    // retry_after_ms is untouched.
    assert_eq!(rl.retry_after_ms, Some(5000));
}

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
fn apply_early_termination_step_timeout_sets_step_timeout_error() {
    let mut summary = StreamExecutionSummary::default();

    apply_early_termination_to_summary(
        &mut summary,
        &EarlyTermination::StepTimeout {
            message: "no stream activity for 6s; terminating due to step_timeout".into(),
            outstanding: Vec::new(),
        },
    );

    assert_eq!(summary.exit_code, 1);
    assert!(summary.is_error);
    assert_eq!(summary.error_kind.as_deref(), Some("step_timeout"));
    assert!(
        summary
            .error_message
            .as_deref()
            .unwrap_or("")
            .contains("no stream activity"),
    );
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
fn apply_early_termination_timeout_sets_timeout_error() {
    let mut summary = StreamExecutionSummary::default();

    apply_early_termination_to_summary(
        &mut summary,
        &EarlyTermination::Timeout {
            message: "wall-clock budget exceeded after 2h".into(),
        },
    );

    assert_eq!(summary.exit_code, 1);
    assert!(summary.is_error);
    assert_eq!(summary.error_kind.as_deref(), Some("timeout"));
    assert_eq!(
        summary.error_message.as_deref(),
        Some("wall-clock budget exceeded after 2h"),
    );
}

// ---- Phase 4: content-guard variant coverage (VC-4.1 / VC-4.2 / VC-4.3) ----

#[test]
fn apply_early_termination_exit_expression_sets_summary_fields_without_scope() {
    let mut summary = StreamExecutionSummary {
        exit_code: 143,
        is_error: false,
        ..Default::default()
    };

    apply_early_termination_to_summary(
        &mut summary,
        &EarlyTermination::ExitExpression {
            pattern: "STOP.".into(),
            scope: None,
        },
    );

    assert_eq!(summary.exit_code, 1);
    assert!(summary.is_error);
    assert_eq!(summary.error_kind.as_deref(), Some("exit_expression"));
    let msg = summary
        .error_message
        .as_deref()
        .expect("error_message must be populated");
    assert!(
        msg.contains("STOP."),
        "error_message must name the pattern: {msg}",
    );
    assert!(
        !msg.contains('('),
        "without a scope the message must omit the parenthetical: {msg}",
    );
}

#[test]
fn apply_early_termination_exit_expression_names_scope_when_present() {
    let mut summary = StreamExecutionSummary::default();

    apply_early_termination_to_summary(
        &mut summary,
        &EarlyTermination::ExitExpression {
            pattern: "STOP.".into(),
            scope: Some("opencode/kimi-for-coding/k2p7".into()),
        },
    );

    assert_eq!(summary.error_kind.as_deref(), Some("exit_expression"));
    let msg = summary.error_message.as_deref().unwrap_or("");
    assert!(
        msg.contains("STOP."),
        "message must name the pattern: {msg}"
    );
    assert!(
        msg.contains("opencode/kimi-for-coding/k2p7"),
        "message must name the scope verbatim (model may contain '/'): {msg}",
    );
}

#[test]
fn apply_early_termination_runaway_repetition_names_cycle_and_repeats() {
    let mut summary = StreamExecutionSummary::default();

    apply_early_termination_to_summary(
        &mut summary,
        &EarlyTermination::RunawayRepetition {
            cycle_len: 6,
            repeats: 30,
        },
    );

    assert_eq!(summary.exit_code, 1);
    assert!(summary.is_error);
    assert_eq!(summary.error_kind.as_deref(), Some("runaway_repetition"));
    let msg = summary.error_message.as_deref().unwrap_or("");
    assert!(
        msg.contains("6"),
        "message must name the cycle length: {msg}",
    );
    assert!(
        msg.contains("30"),
        "message must name the repeat count: {msg}",
    );
}

#[test]
fn apply_early_termination_runaway_volume_names_lines_and_bytes() {
    let mut summary = StreamExecutionSummary::default();

    apply_early_termination_to_summary(
        &mut summary,
        &EarlyTermination::RunawayVolume {
            lines: 51_234,
            bytes: 34_603_008,
        },
    );

    assert_eq!(summary.exit_code, 1);
    assert!(summary.is_error);
    assert_eq!(summary.error_kind.as_deref(), Some("runaway_volume"));
    let msg = summary.error_message.as_deref().unwrap_or("");
    assert!(
        msg.contains("51234"),
        "message must name the line count verbatim: {msg}",
    );
    assert!(
        msg.contains("34603008"),
        "message must name the byte count verbatim: {msg}",
    );
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
fn apply_early_termination_stalled_generation_sets_summary_fields_and_context() {
    let mut summary = StreamExecutionSummary {
        exit_code: 143,
        is_error: false,
        ..Default::default()
    };

    apply_early_termination_to_summary(
        &mut summary,
        &EarlyTermination::StalledGeneration {
            generation_count: 4,
            stall_duration: Duration::from_secs(600),
            context: StalledGenerationContext {
                session_id: Some("ses_x".into()),
                step: Some(7),
                agent: Some("build".into()),
                provider_id: Some("zai-coding-plan".into()),
                model_id: Some("glm-5.2".into()),
                mode: Some("build".into()),
            },
        },
    );

    assert_eq!(summary.exit_code, 1);
    assert!(summary.is_error);
    assert_eq!(summary.error_kind.as_deref(), Some("stalled_generation"));
    let msg = summary.error_message.as_deref().unwrap_or("");
    assert!(
        msg.contains('4'),
        "message must name the generation-attempt count: {msg}",
    );
    assert!(
        msg.contains("600"),
        "message must name the elapsed progress silence: {msg}",
    );
    // Safe context is surfaced; prompt text / tool payloads never are.
    assert!(msg.contains("ses_x"), "message must name the session id: {msg}");
    assert!(msg.contains("glm-5.2"), "message must name the model id: {msg}");
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
fn early_termination_guard_context_populates_relevant_cluster() {
    // Exit-expression → pattern + scope, nothing else.
    let gc = early_termination_guard_context(&EarlyTermination::ExitExpression {
        pattern: "STOP.".into(),
        scope: Some("opencode".into()),
    })
    .expect("exit-expression has guard context");
    assert_eq!(gc.pattern.as_deref(), Some("STOP."));
    assert_eq!(gc.scope.as_deref(), Some("opencode"));
    assert!(gc.cycle_len.is_none() && gc.lines.is_none());

    // Repetition → cycle_len + repeats.
    let gc = early_termination_guard_context(&EarlyTermination::RunawayRepetition {
        cycle_len: 6,
        repeats: 30,
    })
    .expect("repetition has guard context");
    assert_eq!(gc.cycle_len, Some(6));
    assert_eq!(gc.repeats, Some(30));
    assert!(gc.pattern.is_none() && gc.bytes.is_none());

    // Volume → lines + bytes.
    let gc = early_termination_guard_context(&EarlyTermination::RunawayVolume {
        lines: 50_001,
        bytes: 33_554_432,
    })
    .expect("volume has guard context");
    assert_eq!(gc.lines, Some(50_001));
    assert_eq!(gc.bytes, Some(33_554_432));

    // Stalled-generation → generation_count + stall_duration_ms, and every
    // runaway/exit-expression cluster field stays None. With an empty
    // context the optional OpenCode identity fields are absent too.
    let gc = early_termination_guard_context(&EarlyTermination::StalledGeneration {
        generation_count: 4,
        stall_duration: Duration::from_secs(600),
        context: StalledGenerationContext::default(),
    })
    .expect("stalled-generation has guard context");
    assert_eq!(gc.generation_count, Some(4));
    assert_eq!(gc.stall_duration_ms, Some(600_000));
    assert!(
        gc.pattern.is_none()
            && gc.scope.is_none()
            && gc.cycle_len.is_none()
            && gc.repeats.is_none()
            && gc.lines.is_none()
            && gc.bytes.is_none(),
        "stalled-generation must not populate runaway/exit-expression clusters",
    );
    assert!(
        gc.session_id.is_none()
            && gc.step.is_none()
            && gc.agent.is_none()
            && gc.provider_id.is_none()
            && gc.model_id.is_none()
            && gc.mode.is_none(),
        "absent OpenCode metadata must leave the identity fields None",
    );

    // Non-content terminations carry no guard context.
    assert!(
        early_termination_guard_context(&EarlyTermination::Timeout {
            message: "x".into()
        })
        .is_none()
    );
}

#[test]
fn early_termination_guard_context_carries_opencode_identity_metadata() {
    // When the detector captured OpenCode identity tags, the structured
    // guard context exposes them so lifecycle consumers can branch on the
    // run without parsing the prose message.
    let gc = early_termination_guard_context(&EarlyTermination::StalledGeneration {
        generation_count: 4,
        stall_duration: Duration::from_secs(600),
        context: StalledGenerationContext {
            session_id: Some("ses_10ea40010ffeUlahGfHA4R7Mmv".into()),
            step: Some(70),
            agent: Some("rust-developer".into()),
            provider_id: Some("zai-coding-plan".into()),
            model_id: Some("glm-5.2".into()),
            mode: Some("all".into()),
        },
    })
    .expect("stalled-generation has guard context");

    assert_eq!(gc.generation_count, Some(4));
    assert_eq!(gc.stall_duration_ms, Some(600_000));
    assert_eq!(
        gc.session_id.as_deref(),
        Some("ses_10ea40010ffeUlahGfHA4R7Mmv")
    );
    assert_eq!(gc.step, Some(70));
    assert_eq!(gc.agent.as_deref(), Some("rust-developer"));
    assert_eq!(gc.provider_id.as_deref(), Some("zai-coding-plan"));
    assert_eq!(gc.model_id.as_deref(), Some("glm-5.2"));
    assert_eq!(gc.mode.as_deref(), Some("all"));
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

/// `early_termination_message` must return the same string that
/// `apply_early_termination_to_summary` writes to `summary.error_message`
/// — the inline stderr rendering and the summary field must never drift
/// apart (the spawn.rs post-wait match relies on this contract).
#[test]
fn early_termination_message_matches_summary_error_message_for_all_variants() {
    let cases: Vec<EarlyTermination> = vec![
        EarlyTermination::RateLimit {
            message: "usage limit reached".into(),
            reset_at: None,
        },
        EarlyTermination::Timeout {
            message: "wall-clock budget exceeded".into(),
        },
        EarlyTermination::StepTimeout {
            message: "stream silence".into(),
            outstanding: Vec::new(),
        },
        EarlyTermination::ExitExpression {
            pattern: "STOP.".into(),
            scope: Some("opencode".into()),
        },
        EarlyTermination::RunawayRepetition {
            cycle_len: 4,
            repeats: 31,
        },
        EarlyTermination::RunawayVolume {
            lines: 99,
            bytes: 1234,
        },
        EarlyTermination::StalledGeneration {
            generation_count: 4,
            stall_duration: Duration::from_secs(600),
            context: StalledGenerationContext {
                session_id: Some("ses_x".into()),
                step: Some(7),
                agent: Some("build".into()),
                provider_id: Some("zai-coding-plan".into()),
                model_id: Some("glm-5.2".into()),
                mode: Some("build".into()),
            },
        },
    ];

    for termination in cases {
        let mut summary = StreamExecutionSummary::default();
        apply_early_termination_to_summary(&mut summary, &termination);
        let summary_msg = summary.error_message.expect("error_message populated");
        let helper_msg = early_termination_message(&termination).expect("helper returns Some");
        assert_eq!(
            summary_msg, helper_msg,
            "drift between apply_early_termination_to_summary and early_termination_message \
             for {termination:?}",
        );
    }
}

#[test]
fn watchdog_request_to_early_termination_maps_timeout_reason() {
    use super::{WatchdogTermination, WatchdogTerminationReason};
    use crate::commands::wrap::exec::subagent_watchdog::ActiveSubagentSnapshot;

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
    use super::{WatchdogTermination, WatchdogTerminationReason};
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

/// Regression: a disconnected watchdog channel must log a warning rather
/// than silently disabling timeout enforcement. The wait loop should still
/// return normally once the child exits.
#[cfg(unix)]
#[test]
#[tracing_test::traced_test]
fn disconnected_watchdog_channel_warns_and_returns_on_child_exit() {
    use std::os::unix::process::CommandExt;
    use std::process::Command;
    use std::sync::mpsc::channel;

    let mut child = Command::new("sleep")
        .arg("0.5")
        .process_group(0)
        .spawn()
        .expect("sleep must be available on PATH");
    let (_early_tx, early_rx) = channel::<EarlyTermination>();
    let (watchdog_tx, watchdog_rx) = channel::<WatchdogTermination>();
    // Drop the sender to disconnect the channel before the loop polls it.
    drop(watchdog_tx);

    let result = wait_with_signal_and_early_termination(
        &mut child,
        true,
        early_rx,
        Some(watchdog_rx),
        Duration::from_secs(1),
        true,
    );

    let (code, termination, _) = result.expect("wait loop must return when child exits");
    assert_eq!(code, 0, "sleep should exit 0; got {code}");
    assert_eq!(termination, claudine::harness::ProcessTermination::Completed);
    assert!(
        logs_contain("watchdog ticker channel disconnected"),
        "expected warning log for disconnected watchdog channel"
    );
}

/// The SIGTERM/SIGKILL escalation path should still reap a normally
/// exiting child after an early-termination signal. This exercises the
/// loop-driven kill path and its PID-recycle guard (the guard re-checks
/// try_wait immediately before each signal).
#[cfg(unix)]
#[test]
fn early_termination_signal_reaps_child_and_reports_timed_out() {
    use std::os::unix::process::CommandExt;
    use std::process::Command;
    use std::sync::mpsc::channel;

    let mut child = Command::new("sleep")
        .arg("10")
        .process_group(0)
        .spawn()
        .expect("sleep must be available on PATH");
    let (early_tx, early_rx) = channel::<EarlyTermination>();
    let (_watchdog_tx, watchdog_rx) = channel::<WatchdogTermination>();

    // Send the early-termination signal from another thread so the wait
    // loop has time to start polling.
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(50));
        let _ = early_tx.send(EarlyTermination::Timeout {
            message: "test timeout".into(),
        });
    });

    let result = wait_with_signal_and_early_termination(
        &mut child,
        true,
        early_rx,
        Some(watchdog_rx),
        Duration::from_millis(100),
        true,
    );

    let (code, termination, early) = result.expect("wait loop must return");
    assert!(
        matches!(early, Some(EarlyTermination::Timeout { ref message }) if message == "test timeout"),
        "early termination should be carried through; got {early:?}"
    );
    assert_eq!(termination, claudine::harness::ProcessTermination::TimedOut);
    // Killed by SIGTERM or SIGKILL; either way the exit code is non-zero.
    assert!(code != 0, "child should have been terminated; got {code}");
}

/// Kimi wire mode uses completion termination after receiving the final
/// prompt response: the child tree must still be terminated through the
/// shared signal-aware path, but the wrapper outcome remains Completed.
#[cfg(unix)]
#[test]
fn completion_termination_reaps_child_and_reports_completed() {
    use std::os::unix::process::CommandExt;
    use std::process::Command;
    use std::sync::mpsc::channel;

    let mut child = Command::new("sleep")
        .arg("10")
        .process_group(0)
        .spawn()
        .expect("sleep must be available on PATH");
    let (_early_tx, early_rx) = channel::<EarlyTermination>();
    let (completion_tx, completion_rx) = channel::<CompletionTermination>();

    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(50));
        let _ = completion_tx.send(CompletionTermination);
    });

    let result = wait_with_signal_early_termination_and_completion(
        &mut child,
        true,
        early_rx,
        None,
        Some(completion_rx),
        Duration::from_millis(100),
        false,
    );

    let (code, termination, early) = result.expect("wait loop must return");
    assert_eq!(code, 0);
    assert_eq!(termination, claudine::harness::ProcessTermination::Completed);
    assert!(early.is_none(), "completion is not an error path: {early:?}");
}

/// VC-5.4: the non-interactive ladder is SIGTERM-first (F5). A single
/// counted press on a non-interactive run must escalate straight to
/// SIGTERM (no human is mid-session to react to a graceful SIGINT), while
/// an interactive run keeps the full `SIGINT → SIGTERM → SIGKILL` ladder.
#[cfg(unix)]
#[test]
fn escalation_signal_compresses_ladder_when_non_interactive() {
    // Interactive: three presses to force-kill.
    assert_eq!(escalation_signal(true, 1), libc::SIGINT);
    assert_eq!(escalation_signal(true, 2), libc::SIGTERM);
    assert_eq!(escalation_signal(true, 3), libc::SIGKILL);

    // Non-interactive: SIGTERM on the first press, SIGKILL on the next.
    assert_eq!(escalation_signal(false, 1), libc::SIGTERM);
    assert_eq!(escalation_signal(false, 2), libc::SIGKILL);
    assert_eq!(escalation_signal(false, 3), libc::SIGKILL);
}

/// Smoke-test for the non-Unix parity path. Only runs on Windows, but the
/// branch is compile-checked on every target.
#[cfg(not(unix))]
#[test]
fn non_unix_wait_loop_returns_on_child_exit() {
    use std::process::Command;
    use std::sync::mpsc::channel;

    let mut child = Command::new("cmd")
        .args(["/C", "timeout /T 1 /nobreak >nul"])
        .spawn()
        .expect("cmd must be available");
    let (_early_tx, early_rx) = channel::<EarlyTermination>();
    let (_watchdog_tx, watchdog_rx) = channel::<WatchdogTermination>();

    let result = wait_with_signal_and_early_termination(
        &mut child,
        false,
        early_rx,
        Some(watchdog_rx),
        Duration::from_secs(1),
        true,
    );

    let (code, termination, _) = result.expect("wait loop must return when child exits");
    assert_eq!(code, 0);
    assert_eq!(termination, claudine::harness::ProcessTermination::Completed);
}

/// Windows-specific coverage for Kimi's prompt-finished fallback: the
/// child is spawned in a new process group and completion termination
/// travels through the Job Object wait loop rather than `Child::kill`.
#[cfg(windows)]
#[test]
fn windows_completion_termination_uses_job_object_path() {
    use std::os::windows::process::CommandExt;
    use std::process::Command;
    use std::sync::mpsc::channel;

    const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
    let mut child = Command::new("cmd")
        .args(["/C", "timeout /T 30 /nobreak >nul"])
        .creation_flags(CREATE_NEW_PROCESS_GROUP)
        .spawn()
        .expect("cmd must be available");
    let (_early_tx, early_rx) = channel::<EarlyTermination>();
    let (completion_tx, completion_rx) = channel::<CompletionTermination>();

    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(50));
        let _ = completion_tx.send(CompletionTermination);
    });

    let result = wait_with_signal_early_termination_and_completion(
        &mut child,
        true,
        early_rx,
        None,
        Some(completion_rx),
        Duration::from_millis(100),
        false,
    );

    let (code, termination, early) = result.expect("wait loop must return");
    assert_eq!(code, 0);
    assert_eq!(termination, claudine::harness::ProcessTermination::Completed);
    assert!(early.is_none(), "completion is not an error path: {early:?}");
}
