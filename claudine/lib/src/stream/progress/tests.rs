use super::*;

#[test]
fn records_and_completes_tool_lifecycle() {
    let mut state = LiveMetricsState::default();
    let now = Instant::now();
    state.record_tool_start("id-1".into(), Some("Bash".into()), now);
    assert_eq!(state.in_flight.len(), 1);
    let removed = state.record_tool_end(Some("id-1"), now);
    assert!(removed.is_some());
    assert_eq!(removed.unwrap().name.as_deref(), Some("Bash"));
    assert_eq!(state.in_flight.len(), 0);
    assert_eq!(state.done_count, 1);
}

#[test]
fn should_warn_stall_returns_false_when_threshold_not_reached() {
    let mut state = LiveMetricsState::default();
    let now = Instant::now();
    state.last_event_at = Some(now);
    assert!(
        !should_warn_stall(&state, now, Duration::from_secs(60)),
        "fresh activity must not trigger a stall warning"
    );
}

#[test]
fn should_warn_stall_returns_true_after_threshold() {
    let mut state = LiveMetricsState::default();
    let now = Instant::now();
    state.last_event_at = Some(now - Duration::from_secs(120));
    assert!(
        should_warn_stall(&state, now, Duration::from_secs(60)),
        "elapsed-since-activity past threshold must trigger a warning"
    );
}

#[test]
fn should_warn_stall_dedupes_within_one_episode() {
    let mut state = LiveMetricsState::default();
    let last_event = Instant::now() - Duration::from_secs(120);
    state.last_event_at = Some(last_event);
    // Mark the warning as already fired during this stall episode.
    state.last_stall_warning_at = Some(last_event + Duration::from_secs(60));
    assert!(
        !should_warn_stall(&state, Instant::now(), Duration::from_secs(60)),
        "stall warning must not re-fire within the same stall episode"
    );
}

#[test]
fn should_warn_stall_re_fires_after_activity_resumes() {
    let mut state = LiveMetricsState::default();
    // Activity resumed AFTER a previous stall warning was emitted.
    let prior_warning = Instant::now() - Duration::from_secs(180);
    let resumed_at = Instant::now() - Duration::from_secs(120);
    state.last_stall_warning_at = Some(prior_warning);
    state.last_event_at = Some(resumed_at);
    assert!(
        should_warn_stall(&state, Instant::now(), Duration::from_secs(60)),
        "a fresh stall episode after resumed activity must warn again"
    );
}

#[test]
fn should_warn_stall_returns_false_when_no_activity_seen_yet() {
    let state = LiveMetricsState::default();
    assert!(
        !should_warn_stall(&state, Instant::now(), Duration::from_secs(60)),
        "must not warn when no activity has been observed at all"
    );
}

#[test]
fn record_byte_activity_updates_last_byte_at_for_non_whitespace() {
    let mut state = LiveMetricsState::default();
    let now = Instant::now();
    state.record_byte_activity("hello", now);
    assert_eq!(state.last_byte_at, Some(now));
}

#[test]
fn record_byte_activity_ignores_whitespace_only() {
    let mut state = LiveMetricsState::default();
    let now = Instant::now();
    state.record_byte_activity("", now);
    state.record_byte_activity("   \t  ", now);
    state.record_byte_activity("\n", now);
    assert_eq!(
        state.last_byte_at, None,
        "whitespace-only writes must not refresh the byte clock"
    );
}

#[test]
fn last_activity_at_picks_more_recent_clock() {
    let mut state = LiveMetricsState::default();
    assert_eq!(state.last_activity_at(), None);

    let earlier = Instant::now() - Duration::from_secs(10);
    let later = Instant::now();

    state.last_event_at = Some(earlier);
    assert_eq!(state.last_activity_at(), Some(earlier));

    state.last_byte_at = Some(later);
    assert_eq!(
        state.last_activity_at(),
        Some(later),
        "byte clock newer than event clock must win"
    );

    state.last_event_at = Some(later + Duration::from_millis(1));
    assert_eq!(
        state.last_activity_at(),
        Some(later + Duration::from_millis(1)),
        "event clock newer than byte clock must win"
    );
}

#[test]
fn record_activity_updates_last_event_at() {
    let mut state = LiveMetricsState::default();
    let now = Instant::now();
    state.record_activity(now);
    assert_eq!(state.last_event_at, Some(now));
}

#[test]
fn stuck_tools_returns_empty_when_all_fresh() {
    let mut state = LiveMetricsState::default();
    let now = Instant::now();
    state.record_tool_start("tool-1".into(), Some("Bash".into()), now);
    state.record_tool_start("tool-2".into(), Some("Read".into()), now);
    let stuck = state.stuck_tools(now, Duration::from_secs(5));
    assert!(stuck.is_empty(), "all fresh tools must return empty vec");
}

#[test]
fn stuck_tools_returns_stuck_ones() {
    let mut state = LiveMetricsState::default();
    let now = Instant::now();
    let fresh = now;
    let stale = now - Duration::from_secs(10);
    state.record_tool_start("tool-1".into(), Some("Bash".into()), stale);
    state.record_tool_start("tool-2".into(), Some("Read".into()), fresh);
    // Manually override last_progress_at for the stale tool
    if let Some(tool) = state.in_flight.get_mut("tool-1") {
        tool.last_progress_at = stale;
    }
    let stuck = state.stuck_tools(now, Duration::from_secs(5));
    assert_eq!(stuck.len(), 1, "exactly one tool should be stuck");
    assert_eq!(stuck[0].name.as_deref(), Some("Bash"));
}

#[test]
fn stuck_subagents_returns_stuck_ones() {
    let mut state = LiveMetricsState::default();
    let now = Instant::now();
    let fresh = now;
    let stale = now - Duration::from_secs(10);
    state.record_subagent_start("sa-1".into(), Some("researcher".into()), stale);
    state.record_subagent_start("sa-2".into(), Some("coder".into()), fresh);
    // Manually override last_progress_at for the stale subagent
    if let Some(subagent) = state.in_flight_subagents.get_mut("sa-1") {
        subagent.last_progress_at = stale;
    }
    let stuck = state.stuck_subagents(now, Duration::from_secs(5));
    assert_eq!(stuck.len(), 1, "exactly one subagent should be stuck");
    assert_eq!(stuck[0].name.as_deref(), Some("researcher"));
}

mod observe_event_tests {
    use super::*;
    use crate::stream::semantic::SemanticEvent;

    #[test]
    fn activity_event_refreshes_last_event_at() {
        let mut state = LiveMetricsState::default();
        let now = Instant::now();
        state.observe_event(
            &SemanticEvent::OutputText {
                text: "x".into(),
                extra: serde_json::json!({}),
            },
            now,
        );
        assert_eq!(state.last_event_at, Some(now));
    }

    #[test]
    fn envelope_event_does_not_refresh_last_event_at() {
        let mut state = LiveMetricsState::default();
        let now = Instant::now();
        state.observe_event(
            &SemanticEvent::SessionStart {
                session_id: None,
                model: None,
                extra: serde_json::json!({}),
            },
            now,
        );
        assert_eq!(state.last_event_at, None);
    }

    #[test]
    fn tool_call_and_result_track_in_flight() {
        let mut state = LiveMetricsState::default();
        let now = Instant::now();
        state.observe_event(
            &SemanticEvent::ToolCall {
                name: Some("bash".into()),
                id: Some("t1".into()),
                input: None,
                extra: serde_json::json!({}),
            },
            now,
        );
        assert_eq!(state.in_flight.len(), 1);
        state.observe_event(
            &SemanticEvent::ToolResult {
                name: None,
                id: Some("t1".into()),
                status: None,
                exit_code: None,
                output: None,
                extra: serde_json::json!({}),
            },
            now,
        );
        assert!(state.in_flight.is_empty());
        assert_eq!(state.done_count, 1);
    }

    #[test]
    fn tool_result_without_id_uses_name_fallback_to_clear_in_flight() {
        let mut state = LiveMetricsState::default();
        let now = Instant::now();
        state.observe_event(
            &SemanticEvent::ToolCall {
                name: Some("Task".into()),
                id: None,
                input: None,
                extra: serde_json::json!({}),
            },
            now,
        );
        assert_eq!(state.in_flight.len(), 1);
        state.observe_event(
            &SemanticEvent::ToolResult {
                name: Some("Task".into()),
                id: None,
                status: Some("success".into()),
                exit_code: None,
                output: None,
                extra: serde_json::json!({}),
            },
            now,
        );
        assert!(
            state.in_flight.is_empty(),
            "id-less completion should clear the name-keyed in-flight tool"
        );
        assert_eq!(state.done_count, 1);
    }

    #[test]
    fn subagent_start_stop_tracked() {
        let mut state = LiveMetricsState::default();
        let now = Instant::now();
        state.observe_event(
            &SemanticEvent::SubagentStart {
                name: Some("researcher".into()),
                id: Some("sa1".into()),
                extra: serde_json::json!({}),
            },
            now,
        );
        assert_eq!(state.in_flight_subagents.len(), 1);
        state.observe_event(
            &SemanticEvent::SubagentStop {
                name: None,
                id: Some("sa1".into()),
                status: None,
                extra: serde_json::json!({}),
            },
            now,
        );
        assert!(state.in_flight_subagents.is_empty());
        assert_eq!(state.subagent_done_count, 1);
    }

    #[test]
    fn subagent_stop_without_id_uses_name_fallback() {
        let mut state = LiveMetricsState::default();
        let now = Instant::now();
        state.observe_event(
            &SemanticEvent::SubagentStart {
                name: Some("researcher".into()),
                id: None,
                extra: serde_json::json!({}),
            },
            now,
        );
        assert_eq!(state.in_flight_subagents.len(), 1);
        state.observe_event(
            &SemanticEvent::SubagentStop {
                name: Some("researcher".into()),
                id: None,
                status: Some("success".into()),
                extra: serde_json::json!({}),
            },
            now,
        );
        assert!(
            state.in_flight_subagents.is_empty(),
            "id-less stop should clear the name-keyed in-flight subagent"
        );
        assert_eq!(state.subagent_done_count, 1);
    }

    #[test]
    fn turn_complete_updates_token_usage_and_cost() {
        let mut state = LiveMetricsState::default();
        let now = Instant::now();
        state.observe_event(
            &SemanticEvent::TurnComplete {
                provider_status: Some("stop".into()),
                token_usage: Some(NormalizedTokenUsage {
                    input: Some(100),
                    output: Some(50),
                    total: Some(150),
                    cache_read: None,
                }),
                cost_usd: Some(0.01),
                duration_ms: None,
                extra: serde_json::json!({}),
            },
            now,
        );
        assert_eq!(state.cost_usd, Some(0.01));
        let tu = state.token_usage.unwrap();
        assert_eq!(tu.input, Some(100));
    }

    #[test]
    fn subagent_start_tracks_in_flight() {
        let mut state = LiveMetricsState::default();
        let now = Instant::now();
        state.observe_event(
            &SemanticEvent::SubagentStart {
                name: Some("researcher".into()),
                id: Some("sa1".into()),
                extra: serde_json::json!({}),
            },
            now,
        );
        assert_eq!(state.in_flight_subagents.len(), 1);
        let entry = state.in_flight_subagents.get("sa1").unwrap();
        assert_eq!(entry.name.as_deref(), Some("researcher"));
    }

    #[test]
    fn turn_complete_records_provider_status() {
        let mut state = LiveMetricsState::default();
        let now = Instant::now();
        state.observe_event(
            &SemanticEvent::TurnComplete {
                provider_status: Some("end_turn".into()),
                token_usage: None,
                cost_usd: None,
                duration_ms: None,
                extra: serde_json::json!({}),
            },
            now,
        );
        assert_eq!(state.provider_status.as_deref(), Some("end_turn"));
    }

    #[test]
    fn info_step_finish_records_provider_status_from_reason_and_clears_step_in_flight() {
        // OpenCode's `step_finish` is parsed into
        // `SemanticEvent::Info { extra: { step_phase: "finish", reason: ... } }`.
        // The watchdog OpenCode-grace check keys off `provider_status`,
        // so the reason must land in that field.
        let mut state = LiveMetricsState::default();
        let now = Instant::now();
        state.observe_event(
            &SemanticEvent::Info {
                message: "step_finish".into(),
                extra: serde_json::json!({
                    "step_phase": "finish",
                    "reason": "tool-calls",
                }),
            },
            now,
        );
        assert_eq!(state.provider_status.as_deref(), Some("tool-calls"));
        assert!(
            !state.step_in_flight,
            "step_finish must set step_in_flight = false"
        );
    }

    #[test]
    fn info_step_start_sets_step_in_flight_and_does_not_set_provider_status() {
        let mut state = LiveMetricsState::default();
        let now = Instant::now();
        state.observe_event(
            &SemanticEvent::Info {
                message: "step_start".into(),
                extra: serde_json::json!({"step_phase": "start"}),
            },
            now,
        );
        assert_eq!(state.provider_status, None);
        assert!(
            state.step_in_flight,
            "step_start must set step_in_flight = true"
        );
    }

    #[test]
    fn info_step_finish_without_reason_falls_back_to_finish() {
        let mut state = LiveMetricsState::default();
        let now = Instant::now();
        state.observe_event(
            &SemanticEvent::Info {
                message: "step_finish".into(),
                extra: serde_json::json!({"step_phase": "finish"}),
            },
            now,
        );
        assert_eq!(state.provider_status.as_deref(), Some("finish"));
        assert!(
            !state.step_in_flight,
            "step_finish must clear step_in_flight even without reason"
        );
    }
}
