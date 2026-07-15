//! Summary-field and guard-context projection, plus message/summary drift.

use super::*;

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

// ---- Content-guard variant coverage (VC-4.1 / VC-4.2 / VC-4.3) ----

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
