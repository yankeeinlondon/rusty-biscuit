//! OpenCode-specific watchdog tests.
//!
//! Covers the provider grace window, the per-step `step_in_flight` gate,
//! the byte-heartbeat backstop, cold-start suppression, and the
//! subagent-count diagnostic embedded in breach messages.

use super::super::*;
use std::sync::atomic::{AtomicBool, Ordering};

#[test]
fn evaluate_timeout_tick_opencode_cold_start_is_bounded_by_the_budget() {
    // OpenCode + step_timeout + no `step_finish` boundary observed
    // (provider_status is None) + silence beyond budget → breach. Missing
    // `provider_status` used to suppress this unconditionally, which is what
    // let the 2026-08-30 startup stalls run until the wall-clock budget (or,
    // with no `timeout` configured, forever). The cold-start window is now
    // bounded by the same `step_timeout` budget as every other silence.
    //
    // Pair with `opencode_cold_start_suppresses_within_the_budget`, which
    // holds the other side: a slow first turn inside the budget is still
    // protected.
    let config = TimeoutConfig {
        timeout: None,
        step_timeout: Some(Duration::from_secs(5)),
        provider: Some(claudine::provider::Provider::OpenCode),
        ..Default::default()
    };
    let state = Arc::new(std::sync::Mutex::new(WatchdogState::default()));
    let metrics = claudine::stream::progress::new_live_metrics();
    let fired = AtomicBool::new(false);
    let t0 = Instant::now() - Duration::from_secs(10);

    {
        let mut m = metrics.lock().unwrap();
        m.last_event_at = Some(t0);
        // provider_status intentionally left at None.
    }

    let result = evaluate_timeout_tick(&config, Instant::now(), t0, &state, &metrics, &fired);
    match result {
        WatchdogTickResult::Breach(ref w) => {
            assert_eq!(w.reason, WatchdogTerminationReason::StepTimeout);
            assert!(
                w.message.contains("never completed a step"),
                "a pre-first-step OpenCode stall must be named as such; got: {}",
                w.message
            );
        }
        other => panic!(
            "OpenCode cold start must not exempt a stale silence clock; got: {other:?}"
        ),
    }
    assert!(fired.load(Ordering::SeqCst));
}

#[test]
fn opencode_cold_start_suppresses_within_the_budget() {
    // Counter-case for the bound above: a slow first turn that is still
    // inside `step_timeout` must not be killed. Without this the fix could
    // collapse into "always time out".
    let config = TimeoutConfig {
        timeout: None,
        step_timeout: Some(Duration::from_secs(60)),
        provider: Some(claudine::provider::Provider::OpenCode),
        ..Default::default()
    };
    let state = Arc::new(std::sync::Mutex::new(WatchdogState::default()));
    let metrics = claudine::stream::progress::new_live_metrics();
    let fired = AtomicBool::new(false);
    let t0 = Instant::now() - Duration::from_secs(10);

    {
        let mut m = metrics.lock().unwrap();
        m.last_event_at = Some(t0);
        assert!(m.provider_status.is_none());
        assert!(!m.step_in_flight);
    }

    let result = evaluate_timeout_tick(&config, Instant::now(), t0, &state, &metrics, &fired);
    assert_eq!(
        result,
        WatchdogTickResult::Ok,
        "a cold start inside the budget must still be protected; got: {result:?}"
    );
    assert!(!fired.load(Ordering::SeqCst));
}

#[test]
fn evaluate_timeout_tick_opencode_grace_releases_after_step_finish() {
    // OpenCode + step_timeout + `step_finish` boundary observed
    // (provider_status is Some) + silence beyond budget → fires.
    let config = TimeoutConfig {
        timeout: None,
        step_timeout: Some(Duration::from_secs(5)),
        provider: Some(claudine::provider::Provider::OpenCode),
        ..Default::default()
    };
    let state = Arc::new(std::sync::Mutex::new(WatchdogState::default()));
    let metrics = claudine::stream::progress::new_live_metrics();
    let fired = AtomicBool::new(false);
    let t0 = Instant::now() - Duration::from_secs(10);

    {
        let mut m = metrics.lock().unwrap();
        m.last_event_at = Some(t0);
        m.provider_status = Some("stop".into());
    }

    let result = evaluate_timeout_tick(&config, Instant::now(), t0, &state, &metrics, &fired);
    match result {
        WatchdogTickResult::Breach(ref w) => {
            assert_eq!(w.reason, WatchdogTerminationReason::StepTimeout);
        }
        other => panic!(
            "expected StepTimeout breach once a step_finish boundary has been observed, \
             got: {other:?}"
        ),
    }
    assert!(fired.load(Ordering::SeqCst));
}

#[test]
fn evaluate_timeout_tick_opencode_grace_does_not_block_wall_clock() {
    // OpenCode + wall-clock timeout breach must still fire even when
    // provider_status is None. The OpenCode grace only suppresses the
    // step_timeout silence rule, never the wall-clock backstop.
    let config = TimeoutConfig {
        timeout: Some(Duration::from_secs(5)),
        step_timeout: Some(Duration::from_secs(5)),
        provider: Some(claudine::provider::Provider::OpenCode),
        ..Default::default()
    };
    let state = Arc::new(std::sync::Mutex::new(WatchdogState::default()));
    let metrics = claudine::stream::progress::new_live_metrics();
    let fired = AtomicBool::new(false);
    let started_at = Instant::now() - Duration::from_secs(10);

    {
        let mut m = metrics.lock().unwrap();
        m.last_event_at = Some(started_at);
        // provider_status intentionally left at None.
    }

    let result = evaluate_timeout_tick(
        &config,
        Instant::now(),
        started_at,
        &state,
        &metrics,
        &fired,
    );
    assert!(
        matches!(result, WatchdogTickResult::Breach(ref w) if w.reason == WatchdogTerminationReason::Timeout),
        "wall-clock timeout must still fire on OpenCode regardless of provider_status; got: {result:?}"
    );
    assert!(fired.load(Ordering::SeqCst));
}

#[test]
fn evaluate_timeout_tick_grace_does_not_apply_to_other_providers() {
    // Claude (or any non-OpenCode provider) does not get the
    // provider_status grace: silence beyond budget with no
    // step_finish observed still fires step_timeout.
    let config = TimeoutConfig {
        timeout: None,
        step_timeout: Some(Duration::from_secs(5)),
        provider: Some(claudine::provider::Provider::Claude),
        ..Default::default()
    };
    let state = Arc::new(std::sync::Mutex::new(WatchdogState::default()));
    let metrics = claudine::stream::progress::new_live_metrics();
    let fired = AtomicBool::new(false);
    let t0 = Instant::now() - Duration::from_secs(10);

    {
        let mut m = metrics.lock().unwrap();
        m.last_event_at = Some(t0);
        // provider_status intentionally left at None.
    }

    let result = evaluate_timeout_tick(&config, Instant::now(), t0, &state, &metrics, &fired);
    match result {
        WatchdogTickResult::Breach(ref w) => {
            assert_eq!(w.reason, WatchdogTerminationReason::StepTimeout);
        }
        other => panic!(
            "Claude provider must not get OpenCode-specific grace; expected StepTimeout, got: {other:?}"
        ),
    }
    assert!(fired.load(Ordering::SeqCst));
}

#[test]
fn evaluate_timeout_tick_grace_does_not_apply_when_provider_unset() {
    // No provider plumbed (None) → no grace; step_timeout fires
    // normally so wrapper passthrough paths don't accidentally inherit
    // OpenCode-specific behaviour.
    let config = TimeoutConfig {
        timeout: None,
        step_timeout: Some(Duration::from_secs(5)),
        provider: None,
        ..Default::default()
    };
    let state = Arc::new(std::sync::Mutex::new(WatchdogState::default()));
    let metrics = claudine::stream::progress::new_live_metrics();
    let fired = AtomicBool::new(false);
    let t0 = Instant::now() - Duration::from_secs(10);

    {
        let mut m = metrics.lock().unwrap();
        m.last_event_at = Some(t0);
    }

    let result = evaluate_timeout_tick(&config, Instant::now(), t0, &state, &metrics, &fired);
    assert!(
        matches!(result, WatchdogTickResult::Breach(ref w) if w.reason == WatchdogTerminationReason::StepTimeout),
        "provider=None must not enable OpenCode grace; got: {result:?}"
    );
}

#[test]
fn opencode_step_in_flight_suppresses_silence() {
    // OpenCode + step_timeout + step_finish observed (provider_status is
    // Some) BUT step_in_flight is true → silence beyond budget must be
    // suppressed. Once step_in_flight is cleared, the breach should fire.
    let config = TimeoutConfig {
        timeout: None,
        step_timeout: Some(Duration::from_secs(5)),
        provider: Some(claudine::provider::Provider::OpenCode),
        ..Default::default()
    };
    let state = Arc::new(std::sync::Mutex::new(WatchdogState::default()));
    let metrics = claudine::stream::progress::new_live_metrics();
    let fired = AtomicBool::new(false);
    let t0 = Instant::now() - Duration::from_secs(10);

    {
        let mut m = metrics.lock().unwrap();
        m.last_event_at = Some(t0);
        m.provider_status = Some("stop".into());
        m.step_in_flight = true;
    }

    let result = evaluate_timeout_tick(&config, Instant::now(), t0, &state, &metrics, &fired);
    assert_eq!(
        result,
        WatchdogTickResult::Ok,
        "OpenCode with step_in_flight=true must suppress step_timeout even after step_finish"
    );
    assert!(!fired.load(Ordering::SeqCst));

    // Now clear step_in_flight (emulate step_finish) and re-evaluate
    {
        let mut m = metrics.lock().unwrap();
        m.step_in_flight = false;
    }

    let result = evaluate_timeout_tick(&config, Instant::now(), t0, &state, &metrics, &fired);
    match result {
        WatchdogTickResult::Breach(ref w) => {
            assert_eq!(w.reason, WatchdogTerminationReason::StepTimeout);
        }
        other => {
            panic!("expected StepTimeout breach once step_in_flight is cleared, got: {other:?}")
        }
    }
    assert!(fired.load(Ordering::SeqCst));
}

/// Mid-step suppression is unchanged: one fresh clock is enough while a
/// step is open, even with the other stale past the budget.
#[test]
fn opencode_mid_step_suppresses_while_one_clock_is_fresh() {
    let config = TimeoutConfig {
        timeout: None,
        step_timeout: Some(Duration::from_secs(5)),
        provider: Some(claudine::provider::Provider::OpenCode),
        ..Default::default()
    };
    let state = Arc::new(std::sync::Mutex::new(WatchdogState::default()));
    let metrics = claudine::stream::progress::new_live_metrics();
    let fired = AtomicBool::new(false);
    let now_t = Instant::now();
    let stale = now_t - Duration::from_secs(30);

    {
        let mut m = metrics.lock().unwrap();
        m.last_event_at = Some(stale);
        m.last_byte_at = Some(now_t);
        m.step_in_flight = true;
        m.provider_status = Some("tool-calls".into());
    }

    let result = evaluate_timeout_tick(&config, now_t, stale, &state, &metrics, &fired);
    assert_eq!(
        result,
        WatchdogTickResult::Ok,
        "a fresh byte clock must protect an open step; got: {result:?}"
    );
    assert!(!fired.load(Ordering::SeqCst));
}

#[test]
fn opencode_step_in_flight_resets_per_step() {
    // The per-step grace must reset for each new step_start/step_finish
    // cycle, not be a one-shot guard. This test drives `observe_event`
    // through a sequence of real `Info{step_phase=start|finish}` events
    // on a SINGLE `LiveMetricsState` instance so the toggling logic is
    // exercised end-to-end — not just the predicate behavior against
    // a hand-set field.
    let config = TimeoutConfig {
        timeout: None,
        step_timeout: Some(Duration::from_secs(5)),
        provider: Some(claudine::provider::Provider::OpenCode),
        ..Default::default()
    };
    let state = Arc::new(std::sync::Mutex::new(WatchdogState::default()));
    let metrics = claudine::stream::progress::new_live_metrics();
    let fired = AtomicBool::new(false);
    let session_start = Instant::now() - Duration::from_secs(60);

    // --- Cycle 1: step_start fed through observe_event ---
    {
        let mut m = metrics.lock().unwrap();
        m.observe_event(
            &claudine::stream::semantic::SemanticEvent::Info {
                message: "step_start".into(),
                extra: serde_json::json!({"step_phase": "start"}),
            },
            Instant::now() - Duration::from_secs(30),
        );
        assert!(
            m.step_in_flight,
            "observe_event must set step_in_flight=true on step_start"
        );
    }
    let result = evaluate_timeout_tick(
        &config,
        Instant::now(),
        session_start,
        &state,
        &metrics,
        &fired,
    );
    assert_eq!(
        result,
        WatchdogTickResult::Ok,
        "first step_start must suppress step_timeout (silence stale, step_in_flight=true)"
    );
    assert!(!fired.load(Ordering::SeqCst));

    // --- Cycle 1: step_finish fed through observe_event ---
    {
        let mut m = metrics.lock().unwrap();
        m.observe_event(
            &claudine::stream::semantic::SemanticEvent::Info {
                message: "step_finish".into(),
                extra: serde_json::json!({"step_phase": "finish", "reason": "tool-calls"}),
            },
            Instant::now() - Duration::from_secs(20),
        );
        assert!(
            !m.step_in_flight,
            "observe_event must clear step_in_flight on step_finish"
        );
        assert_eq!(m.provider_status.as_deref(), Some("tool-calls"));
    }
    let result = evaluate_timeout_tick(
        &config,
        Instant::now(),
        session_start,
        &state,
        &metrics,
        &fired,
    );
    assert!(
        matches!(result, WatchdogTickResult::Breach(ref w) if w.reason == WatchdogTerminationReason::StepTimeout),
        "step_finish must release suppression and allow step_timeout to fire; got: {result:?}"
    );
    assert!(fired.load(Ordering::SeqCst));

    // --- Cycle 2: step_start fed through observe_event again (per-step reset) ---
    fired.store(false, Ordering::SeqCst);
    {
        let mut m = metrics.lock().unwrap();
        m.observe_event(
            &claudine::stream::semantic::SemanticEvent::Info {
                message: "step_start".into(),
                extra: serde_json::json!({"step_phase": "start"}),
            },
            Instant::now() - Duration::from_secs(10),
        );
        assert!(
            m.step_in_flight,
            "second step_start must toggle step_in_flight back to true (per-step reset)"
        );
        // provider_status remains Some from the first step_finish — proves
        // the grace does NOT rely on the cold-start (`provider_status=None`)
        // protection; it must come from `step_in_flight` alone.
        assert_eq!(m.provider_status.as_deref(), Some("tool-calls"));
    }
    let result = evaluate_timeout_tick(
        &config,
        Instant::now(),
        session_start,
        &state,
        &metrics,
        &fired,
    );
    assert_eq!(
        result,
        WatchdogTickResult::Ok,
        "second step_start must suppress step_timeout again (per-step grace reset)"
    );
    assert!(!fired.load(Ordering::SeqCst));
}

#[test]
fn opencode_byte_heartbeat_still_catches_zero_byte_hang() {
    // Even when step_in_flight is true, if BOTH the structured-event
    // clock and the raw-byte clock are stale beyond step_timeout, the
    // breach must fire. The per-step grace must not override the byte
    // heartbeat backstop.
    let config = TimeoutConfig {
        timeout: None,
        step_timeout: Some(Duration::from_secs(5)),
        provider: Some(claudine::provider::Provider::OpenCode),
        ..Default::default()
    };
    let state = Arc::new(std::sync::Mutex::new(WatchdogState::default()));
    let metrics = claudine::stream::progress::new_live_metrics();
    let fired = AtomicBool::new(false);
    let stale = Instant::now() - Duration::from_secs(10);

    {
        let mut m = metrics.lock().unwrap();
        m.last_event_at = Some(stale);
        m.last_byte_at = Some(stale);
        m.step_in_flight = true;
        m.provider_status = Some("stop".into());
    }

    let result =
        evaluate_timeout_tick(&config, Instant::now(), stale, &state, &metrics, &fired);
    match result {
        WatchdogTickResult::Breach(ref w) => {
            assert_eq!(w.reason, WatchdogTerminationReason::StepTimeout);
        }
        other => panic!(
            "expected StepTimeout breach when both byte and event clocks are stale, even with step_in_flight; got: {other:?}"
        ),
    }
    assert!(fired.load(Ordering::SeqCst));
}

#[test]
fn opencode_step_started_but_never_finished_fires_on_stale_clocks() {
    // Regression for the 2026-05-10 ndjson capture where OpenCode
    // emitted `step_start` and a handful of `text` + `tool_use` events
    // (~7.5s), then went totally silent — no `step_finish` ever. With
    // the old `(step_in_flight && !both_stale) || !provider_status_seen`
    // condition the `!provider_status_seen` arm suppressed the breach
    // indefinitely because no step had ever finished. The fix scopes
    // the cold-start grace to `step_in_flight=false` so an open step
    // with both clocks stale falls through to the byte-heartbeat
    // backstop and fires.
    let config = TimeoutConfig {
        timeout: None,
        step_timeout: Some(Duration::from_secs(5)),
        provider: Some(claudine::provider::Provider::OpenCode),
        ..Default::default()
    };
    let state = Arc::new(std::sync::Mutex::new(WatchdogState::default()));
    let metrics = claudine::stream::progress::new_live_metrics();
    let fired = AtomicBool::new(false);
    let stale = Instant::now() - Duration::from_secs(10);

    {
        let mut m = metrics.lock().unwrap();
        m.last_event_at = Some(stale);
        m.last_byte_at = Some(stale);
        m.step_in_flight = true;
        // provider_status intentionally None — step_start arrived but
        // step_finish never did, mirroring the captured hang.
        assert!(m.provider_status.is_none());
    }

    let result =
        evaluate_timeout_tick(&config, Instant::now(), stale, &state, &metrics, &fired);
    match result {
        WatchdogTickResult::Breach(ref w) => {
            assert_eq!(w.reason, WatchdogTerminationReason::StepTimeout);
        }
        other => panic!(
            "expected StepTimeout breach when step_start arrived but step_finish never did and both clocks are stale; got: {other:?}"
        ),
    }
    assert!(fired.load(Ordering::SeqCst));
}

#[test]
fn opencode_cold_start_breaches_when_both_clocks_are_stale() {
    // Cold start with BOTH the structured-event clock and the raw-byte clock
    // stale past `step_timeout`. This is the exact state the 2026-08-30
    // OpenCode incidents sat in for 30 minutes: configuration files loaded
    // (so bytes arrived), then nothing, and no step ever started. The
    // byte-heartbeat backstop must reach it.
    let config = TimeoutConfig {
        timeout: None,
        step_timeout: Some(Duration::from_secs(5)),
        provider: Some(claudine::provider::Provider::OpenCode),
        ..Default::default()
    };
    let state = Arc::new(std::sync::Mutex::new(WatchdogState::default()));
    let metrics = claudine::stream::progress::new_live_metrics();
    let fired = AtomicBool::new(false);
    let stale = Instant::now() - Duration::from_secs(10);

    {
        let mut m = metrics.lock().unwrap();
        m.last_event_at = Some(stale);
        m.last_byte_at = Some(stale);
        // provider_status stays None — no step_finish has been observed.
        // step_in_flight stays false — no step_start either.
        assert!(m.provider_status.is_none());
        assert!(!m.step_in_flight);
    }

    let result =
        evaluate_timeout_tick(&config, Instant::now(), stale, &state, &metrics, &fired);
    match result {
        WatchdogTickResult::Breach(ref w) => {
            assert_eq!(w.reason, WatchdogTerminationReason::StepTimeout);
        }
        other => panic!(
            "cold start with both clocks stale must breach, not suppress; got: {other:?}"
        ),
    }
    assert!(fired.load(Ordering::SeqCst));
}

/// The OpenCode incident with no output at all: nothing ever reached either
/// activity clock, so the spawn instant is the only reference there is.
#[test]
fn opencode_startup_with_no_output_at_all_breaches_from_spawn() {
    let config = TimeoutConfig {
        timeout: None,
        step_timeout: Some(Duration::from_secs(5)),
        provider: Some(claudine::provider::Provider::OpenCode),
        ..Default::default()
    };
    let state = Arc::new(std::sync::Mutex::new(WatchdogState::default()));
    let metrics = claudine::stream::progress::new_live_metrics();
    let fired = AtomicBool::new(false);
    let started_at = Instant::now() - Duration::from_secs(10);

    let result =
        evaluate_timeout_tick(&config, Instant::now(), started_at, &state, &metrics, &fired);
    match result {
        WatchdogTickResult::Breach(ref w) => {
            assert_eq!(w.reason, WatchdogTerminationReason::StepTimeout);
            assert!(
                w.message
                    .contains("no output since the wrapped process launched"),
                "the diagnostic must say nothing was ever produced; got: {}",
                w.message
            );
            assert!(
                !w.message.contains("never completed a step"),
                "a child that produced nothing has no OpenCode-activity story to tell; got: {}",
                w.message
            );
        }
        other => panic!("expected a spawn-anchored StepTimeout breach; got: {other:?}"),
    }
    assert!(fired.load(Ordering::SeqCst));
}

/// Wall-clock precedence survives the bounded cold-start guard.
#[test]
fn opencode_cold_start_breach_yields_to_the_wall_clock() {
    let config = TimeoutConfig {
        timeout: Some(Duration::from_secs(5)),
        step_timeout: Some(Duration::from_secs(5)),
        provider: Some(claudine::provider::Provider::OpenCode),
        ..Default::default()
    };
    let state = Arc::new(std::sync::Mutex::new(WatchdogState::default()));
    let metrics = claudine::stream::progress::new_live_metrics();
    let fired = AtomicBool::new(false);
    let stale = Instant::now() - Duration::from_secs(10);

    {
        let mut m = metrics.lock().unwrap();
        m.last_event_at = Some(stale);
        m.last_byte_at = Some(stale);
    }

    let result =
        evaluate_timeout_tick(&config, Instant::now(), stale, &state, &metrics, &fired);
    assert!(
        matches!(result, WatchdogTickResult::Breach(ref w) if w.reason == WatchdogTerminationReason::Timeout),
        "wall-clock must still win when both budgets breach on one tick; got: {result:?}"
    );
}

#[test]
fn opencode_breach_diagnostic_names_subagent_count() {
    // When 3 synthesized Task completions have been observed and then a
    // silence breach fires, the rendered message must contain the count.
    let config = TimeoutConfig {
        timeout: None,
        step_timeout: Some(Duration::from_secs(5)),
        provider: Some(claudine::provider::Provider::OpenCode),
        ..Default::default()
    };
    let state = Arc::new(std::sync::Mutex::new(WatchdogState::default()));
    let metrics = claudine::stream::progress::new_live_metrics();
    let fired = AtomicBool::new(false);
    let t0 = Instant::now() - Duration::from_secs(10);

    {
        let mut s = state.lock().unwrap();
        for i in 0..3 {
            s.subagent_stopped(
                &format!("sa-{i}"),
                Some(format!("Task {i}")),
                Some(format!("Description {i}")),
                Some("success".into()),
                t0,
            );
        }
    }

    {
        let mut m = metrics.lock().unwrap();
        m.last_event_at = Some(t0);
        m.provider_status = Some("stop".into());
        m.step_in_flight = false;
        m.subagent_done_count = 3;
    }

    let result = evaluate_timeout_tick(&config, Instant::now(), t0, &state, &metrics, &fired);
    match result {
        WatchdogTickResult::Breach(ref w) => {
            assert_eq!(w.reason, WatchdogTerminationReason::StepTimeout);
            assert!(
                w.message.contains("3 subagents observed"),
                "breach message should name subagent count: {}",
                w.message
            );
        }
        other => {
            panic!("expected StepTimeout breach with subagent count diagnostic; got: {other:?}")
        }
    }
    assert!(fired.load(Ordering::SeqCst));
}

#[test]
fn opencode_breach_diagnostic_lists_recent_subagent_descriptions() {
    // When 5 synthesized Task completions with distinct descriptions have
    // been observed, the breach message must list all 5 in newest-first
    // order.
    let config = TimeoutConfig {
        timeout: None,
        step_timeout: Some(Duration::from_secs(5)),
        provider: Some(claudine::provider::Provider::OpenCode),
        ..Default::default()
    };
    let state = Arc::new(std::sync::Mutex::new(WatchdogState::default()));
    let metrics = claudine::stream::progress::new_live_metrics();
    let fired = AtomicBool::new(false);
    let t0 = Instant::now() - Duration::from_secs(10);

    {
        let mut s = state.lock().unwrap();
        for i in 0..5 {
            s.subagent_stopped(
                &format!("sa-{i}"),
                Some(format!("Name {i}")),
                Some(format!("Desc-{i}")),
                Some("success".into()),
                t0 + Duration::from_secs(i as u64),
            );
        }
    }

    {
        let mut m = metrics.lock().unwrap();
        m.last_event_at = Some(t0);
        m.provider_status = Some("stop".into());
        m.step_in_flight = false;
        m.subagent_done_count = 5;
    }

    let result = evaluate_timeout_tick(&config, Instant::now(), t0, &state, &metrics, &fired);
    match result {
        WatchdogTickResult::Breach(ref w) => {
            assert_eq!(w.reason, WatchdogTerminationReason::StepTimeout);
            assert!(
                w.message.contains("5 subagents observed"),
                "breach message should name subagent count: {}",
                w.message
            );
            assert!(
                w.message.contains("Recent subagents:"),
                "breach message should list recent subagents: {}",
                w.message
            );
            // Newest first: Desc-4 should appear before Desc-3
            let idx_4 = w.message.find("Desc-4").expect("Desc-4 should be present");
            let idx_3 = w.message.find("Desc-3").expect("Desc-3 should be present");
            assert!(idx_4 < idx_3, "newest-first order required: {}", w.message);
        }
        other => panic!(
            "expected StepTimeout breach with recent subagent descriptions; got: {other:?}"
        ),
    }
    assert!(fired.load(Ordering::SeqCst));
}
