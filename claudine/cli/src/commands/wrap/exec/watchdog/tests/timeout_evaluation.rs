//! Watchdog timeout evaluation tests.
//!
//! Covers the unified `evaluate_timeout_tick` predicate under every
//! silence / wall-clock / in-flight / one-shot / per-step combination.
//! Per-provider OpenCode grace and `format_step_timeout_breach_message`
//! coverage live in sibling modules.

use super::super::*;
use std::sync::atomic::{AtomicBool, Ordering};

#[allow(dead_code)]
fn now() -> Instant {
    Instant::now()
}

#[test]
fn evaluate_timeout_tick_ok_when_no_rule_enabled() {
    let config = TimeoutConfig::default();
    let state = Arc::new(std::sync::Mutex::new(WatchdogState::default()));
    let metrics = claudine::stream::progress::new_live_metrics();
    let fired = AtomicBool::new(false);
    let now_t = Instant::now();
    let result = evaluate_timeout_tick(&config, now_t, now_t, &state, &metrics, &fired);
    assert_eq!(result, WatchdogTickResult::Ok);
}

#[test]
fn evaluate_timeout_tick_wall_clock_breach() {
    let config = TimeoutConfig {
        timeout: Some(Duration::from_secs(5)),
        step_timeout: None,
        ..Default::default()
    };
    let state = Arc::new(std::sync::Mutex::new(WatchdogState::default()));
    let metrics = claudine::stream::progress::new_live_metrics();
    let fired = AtomicBool::new(false);
    let started_at = Instant::now() - Duration::from_secs(10);

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
        "expected wall-clock Timeout breach, got: {result:?}"
    );
    assert!(fired.load(Ordering::SeqCst));
}

#[test]
fn evaluate_timeout_tick_silence_breach_with_outstanding_subagents() {
    let config = TimeoutConfig {
        timeout: None,
        step_timeout: Some(Duration::from_secs(5)),
        ..Default::default()
    };
    let state = Arc::new(std::sync::Mutex::new(WatchdogState::default()));
    let metrics = claudine::stream::progress::new_live_metrics();
    let fired = AtomicBool::new(false);
    let t0 = Instant::now() - Duration::from_secs(10);

    {
        let mut s = state.lock().unwrap();
        s.subagent_started("sa1".into(), Some("Researcher".into()), t0);
    }
    {
        let mut m = metrics.lock().unwrap();
        m.last_event_at = Some(t0);
    }

    let result = evaluate_timeout_tick(&config, Instant::now(), t0, &state, &metrics, &fired);
    match result {
        WatchdogTickResult::Breach(ref w) => {
            assert_eq!(w.reason, WatchdogTerminationReason::StepTimeout);
            assert_eq!(w.stuck_subagents.len(), 1);
            assert_eq!(w.stuck_subagents[0].id, "sa1");
            assert!(w.message.contains("Researcher"), "got: {}", w.message);
        }
        other => panic!("expected StepTimeout breach, got: {other:?}"),
    }
    assert!(fired.load(Ordering::SeqCst));
}

#[test]
fn evaluate_timeout_tick_silence_breach_without_subagents() {
    let config = TimeoutConfig {
        timeout: None,
        step_timeout: Some(Duration::from_secs(5)),
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
    match result {
        WatchdogTickResult::Breach(ref w) => {
            assert_eq!(w.reason, WatchdogTerminationReason::StepTimeout);
            assert!(w.stuck_subagents.is_empty());
            assert!(w.message.contains("step_timeout"), "got: {}", w.message);
        }
        other => panic!("expected StepTimeout breach, got: {other:?}"),
    }
    assert!(fired.load(Ordering::SeqCst));
}

#[test]
fn evaluate_timeout_tick_does_not_fire_silence_without_first_event() {
    // Spec: silence rule requires at least one observed activity event
    // (matches `last_event_at: Option<Instant>` first-event grace).
    let config = TimeoutConfig {
        timeout: None,
        step_timeout: Some(Duration::from_secs(1)),
        ..Default::default()
    };
    let state = Arc::new(std::sync::Mutex::new(WatchdogState::default()));
    let metrics = claudine::stream::progress::new_live_metrics();
    let fired = AtomicBool::new(false);
    let started_at = Instant::now() - Duration::from_secs(60);

    let result = evaluate_timeout_tick(
        &config,
        Instant::now(),
        started_at,
        &state,
        &metrics,
        &fired,
    );
    assert_eq!(result, WatchdogTickResult::Ok);
    assert!(!fired.load(Ordering::SeqCst));
}

#[test]
fn evaluate_timeout_tick_wall_clock_wins_over_silence_on_same_tick() {
    let config = TimeoutConfig {
        timeout: Some(Duration::from_secs(5)),
        step_timeout: Some(Duration::from_secs(5)),
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
        matches!(result, WatchdogTickResult::Breach(ref w) if w.reason == WatchdogTerminationReason::Timeout),
        "wall-clock must win; got: {result:?}"
    );
}

#[test]
fn evaluate_timeout_tick_one_shot_guard() {
    let config = TimeoutConfig {
        timeout: Some(Duration::from_secs(1)),
        step_timeout: Some(Duration::from_secs(1)),
        ..Default::default()
    };
    let state = Arc::new(std::sync::Mutex::new(WatchdogState::default()));
    let metrics = claudine::stream::progress::new_live_metrics();
    let fired = AtomicBool::new(true); // already fired
    let started_at = Instant::now() - Duration::from_secs(60);

    {
        let mut m = metrics.lock().unwrap();
        m.last_event_at = Some(started_at);
    }

    let result = evaluate_timeout_tick(
        &config,
        Instant::now(),
        started_at,
        &state,
        &metrics,
        &fired,
    );
    assert_eq!(result, WatchdogTickResult::Ok);
}

#[test]
fn evaluate_timeout_tick_silence_suppressed_by_in_flight_tool() {
    let config = TimeoutConfig {
        timeout: None,
        step_timeout: Some(Duration::from_secs(5)),
        ..Default::default()
    };
    let state = Arc::new(std::sync::Mutex::new(WatchdogState::default()));
    let metrics = claudine::stream::progress::new_live_metrics();
    let fired = AtomicBool::new(false);
    let t0 = Instant::now() - Duration::from_secs(10);

    {
        let mut m = metrics.lock().unwrap();
        m.last_event_at = Some(t0);
        m.in_flight.insert(
            "tool-1".into(),
            claudine::stream::progress::InFlightTool {
                name: Some("Task".into()),
                started_at: t0,
                last_progress_at: t0,
            },
        );
    }

    let result = evaluate_timeout_tick(&config, Instant::now(), t0, &state, &metrics, &fired);
    // Stuck tool (no progress for 10s >= 5s budget) does NOT suppress step_timeout.
    match result {
        WatchdogTickResult::Breach(ref w) => {
            assert_eq!(w.reason, WatchdogTerminationReason::StepTimeout);
            assert!(
                w.message.contains("Task"),
                "stuck tool should be named: {}",
                w.message
            );
        }
        other => panic!("expected StepTimeout breach for stuck tool, got: {other:?}"),
    }
    assert!(fired.load(Ordering::SeqCst));
}

#[test]
fn evaluate_timeout_tick_silence_suppressed_by_in_flight_subagent() {
    let config = TimeoutConfig {
        timeout: None,
        step_timeout: Some(Duration::from_secs(5)),
        ..Default::default()
    };
    let state = Arc::new(std::sync::Mutex::new(WatchdogState::default()));
    let metrics = claudine::stream::progress::new_live_metrics();
    let fired = AtomicBool::new(false);
    let t0 = Instant::now() - Duration::from_secs(10);

    {
        let mut m = metrics.lock().unwrap();
        m.last_event_at = Some(t0);
        m.in_flight_subagents.insert(
            "sa-1".into(),
            claudine::stream::progress::InFlightSubagent {
                name: Some("rust-developer".into()),
                started_at: t0,
                last_progress_at: t0,
            },
        );
    }

    let result = evaluate_timeout_tick(&config, Instant::now(), t0, &state, &metrics, &fired);
    // Stuck subagent (no progress for 10s >= 5s budget) does NOT suppress step_timeout.
    match result {
        WatchdogTickResult::Breach(ref w) => {
            assert_eq!(w.reason, WatchdogTerminationReason::StepTimeout);
        }
        other => panic!("expected StepTimeout breach for stuck subagent, got: {other:?}"),
    }
    assert!(fired.load(Ordering::SeqCst));
}

#[test]
fn evaluate_timeout_tick_silence_fires_after_in_flight_cleared() {
    let config = TimeoutConfig {
        timeout: None,
        step_timeout: Some(Duration::from_secs(5)),
        ..Default::default()
    };
    let state = Arc::new(std::sync::Mutex::new(WatchdogState::default()));
    let metrics = claudine::stream::progress::new_live_metrics();
    let fired = AtomicBool::new(false);
    let t0 = Instant::now() - Duration::from_secs(10);
    let fresh = Instant::now();

    {
        let mut m = metrics.lock().unwrap();
        m.last_event_at = Some(t0);
        m.in_flight.insert(
            "tool-1".into(),
            claudine::stream::progress::InFlightTool {
                name: Some("Task".into()),
                started_at: t0,
                last_progress_at: fresh,
            },
        );
    }

    let result = evaluate_timeout_tick(&config, Instant::now(), t0, &state, &metrics, &fired);
    assert_eq!(
        result,
        WatchdogTickResult::Ok,
        "must not fire while active tool is in-flight"
    );

    {
        let mut m = metrics.lock().unwrap();
        m.in_flight.clear();
    }

    let result = evaluate_timeout_tick(&config, Instant::now(), t0, &state, &metrics, &fired);
    match result {
        WatchdogTickResult::Breach(ref w) => {
            assert_eq!(w.reason, WatchdogTerminationReason::StepTimeout);
        }
        other => panic!("expected StepTimeout breach after in-flight cleared, got: {other:?}"),
    }
    assert!(fired.load(Ordering::SeqCst));
}

#[test]
fn evaluate_timeout_tick_silence_suppressed_when_tool_is_active() {
    let config = TimeoutConfig {
        timeout: None,
        step_timeout: Some(Duration::from_secs(5)),
        ..Default::default()
    };
    let state = Arc::new(std::sync::Mutex::new(WatchdogState::default()));
    let metrics = claudine::stream::progress::new_live_metrics();
    let fired = AtomicBool::new(false);
    let t0 = Instant::now() - Duration::from_secs(10);
    let fresh = Instant::now();

    {
        let mut m = metrics.lock().unwrap();
        m.last_event_at = Some(t0);
        m.in_flight.insert(
            "tool-1".into(),
            claudine::stream::progress::InFlightTool {
                name: Some("Task".into()),
                started_at: t0,
                last_progress_at: fresh,
            },
        );
    }

    let result = evaluate_timeout_tick(&config, Instant::now(), t0, &state, &metrics, &fired);
    assert_eq!(
        result,
        WatchdogTickResult::Ok,
        "active tool must suppress step_timeout"
    );
    assert!(!fired.load(Ordering::SeqCst));
}

#[test]
fn evaluate_timeout_tick_silence_suppressed_when_subagent_is_active() {
    let config = TimeoutConfig {
        timeout: None,
        step_timeout: Some(Duration::from_secs(5)),
        ..Default::default()
    };
    let state = Arc::new(std::sync::Mutex::new(WatchdogState::default()));
    let metrics = claudine::stream::progress::new_live_metrics();
    let fired = AtomicBool::new(false);
    let t0 = Instant::now() - Duration::from_secs(10);
    let fresh = Instant::now();

    {
        let mut m = metrics.lock().unwrap();
        m.last_event_at = Some(t0);
        m.in_flight_subagents.insert(
            "sa-1".into(),
            claudine::stream::progress::InFlightSubagent {
                name: Some("rust-developer".into()),
                started_at: t0,
                last_progress_at: fresh,
            },
        );
    }

    let result = evaluate_timeout_tick(&config, Instant::now(), t0, &state, &metrics, &fired);
    assert_eq!(
        result,
        WatchdogTickResult::Ok,
        "active subagent must suppress step_timeout"
    );
    assert!(!fired.load(Ordering::SeqCst));
}

#[test]
fn evaluate_timeout_tick_silence_fires_when_tool_is_stuck() {
    let config = TimeoutConfig {
        timeout: None,
        step_timeout: Some(Duration::from_secs(5)),
        ..Default::default()
    };
    let state = Arc::new(std::sync::Mutex::new(WatchdogState::default()));
    let metrics = claudine::stream::progress::new_live_metrics();
    let fired = AtomicBool::new(false);
    let t0 = Instant::now() - Duration::from_secs(10);

    {
        let mut m = metrics.lock().unwrap();
        m.last_event_at = Some(t0);
        m.in_flight.insert(
            "tool-1".into(),
            claudine::stream::progress::InFlightTool {
                name: Some("Task".into()),
                started_at: t0,
                last_progress_at: t0,
            },
        );
    }

    let result = evaluate_timeout_tick(&config, Instant::now(), t0, &state, &metrics, &fired);
    match result {
        WatchdogTickResult::Breach(ref w) => {
            assert_eq!(w.reason, WatchdogTerminationReason::StepTimeout);
            assert!(w.message.contains("Task"));
        }
        other => panic!("expected StepTimeout breach for stuck tool, got: {other:?}"),
    }
    assert!(fired.load(Ordering::SeqCst));
}

#[test]
fn evaluate_timeout_tick_silence_fires_when_subagent_is_stuck() {
    let config = TimeoutConfig {
        timeout: None,
        step_timeout: Some(Duration::from_secs(5)),
        ..Default::default()
    };
    let state = Arc::new(std::sync::Mutex::new(WatchdogState::default()));
    let metrics = claudine::stream::progress::new_live_metrics();
    let fired = AtomicBool::new(false);
    let t0 = Instant::now() - Duration::from_secs(10);

    {
        let mut m = metrics.lock().unwrap();
        m.last_event_at = Some(t0);
        m.in_flight_subagents.insert(
            "sa-1".into(),
            claudine::stream::progress::InFlightSubagent {
                name: Some("rust-developer".into()),
                started_at: t0,
                last_progress_at: t0,
            },
        );
    }

    let result = evaluate_timeout_tick(&config, Instant::now(), t0, &state, &metrics, &fired);
    match result {
        WatchdogTickResult::Breach(ref w) => {
            assert_eq!(w.reason, WatchdogTerminationReason::StepTimeout);
            assert!(w.message.contains("rust-developer"));
        }
        other => panic!("expected StepTimeout breach for stuck subagent, got: {other:?}"),
    }
    assert!(fired.load(Ordering::SeqCst));
}

#[test]
fn evaluate_timeout_tick_mixed_active_and_stuck_fires() {
    let config = TimeoutConfig {
        timeout: None,
        step_timeout: Some(Duration::from_secs(5)),
        ..Default::default()
    };
    let state = Arc::new(std::sync::Mutex::new(WatchdogState::default()));
    let metrics = claudine::stream::progress::new_live_metrics();
    let fired = AtomicBool::new(false);
    let t0 = Instant::now() - Duration::from_secs(10);
    let fresh = Instant::now();

    {
        let mut m = metrics.lock().unwrap();
        m.last_event_at = Some(t0);
        m.in_flight.insert(
            "tool-active".into(),
            claudine::stream::progress::InFlightTool {
                name: Some("ActiveTask".into()),
                started_at: t0,
                last_progress_at: fresh,
            },
        );
        m.in_flight.insert(
            "tool-stuck".into(),
            claudine::stream::progress::InFlightTool {
                name: Some("StuckTask".into()),
                started_at: t0,
                last_progress_at: t0,
            },
        );
    }

    let result = evaluate_timeout_tick(&config, Instant::now(), t0, &state, &metrics, &fired);
    match result {
        WatchdogTickResult::Breach(ref w) => {
            assert_eq!(w.reason, WatchdogTerminationReason::StepTimeout);
            assert!(w.message.contains("StuckTask"));
        }
        other => {
            panic!("expected StepTimeout breach when mix of active and stuck, got: {other:?}")
        }
    }
    assert!(fired.load(Ordering::SeqCst));
}

#[test]
fn evaluate_timeout_tick_silence_suppressed_by_recent_byte_activity() {
    // Stale `last_event_at`, but `last_byte_at` is fresh — bytes are
    // still flowing from the child even though the structured parser
    // has not produced a new SemanticEvent. The silence rule must
    // pick the byte clock as the activity reference and suppress.
    let config = TimeoutConfig {
        timeout: None,
        step_timeout: Some(Duration::from_secs(5)),
        ..Default::default()
    };
    let state = Arc::new(std::sync::Mutex::new(WatchdogState::default()));
    let metrics = claudine::stream::progress::new_live_metrics();
    let fired = AtomicBool::new(false);
    let now_t = Instant::now();
    let stale = now_t - Duration::from_secs(10);

    {
        let mut m = metrics.lock().unwrap();
        m.last_event_at = Some(stale);
        m.last_byte_at = Some(now_t); // fresh byte activity
    }

    let result = evaluate_timeout_tick(&config, now_t, stale, &state, &metrics, &fired);
    assert_eq!(
        result,
        WatchdogTickResult::Ok,
        "recent byte activity must suppress step_timeout, got: {result:?}"
    );
    assert!(!fired.load(Ordering::SeqCst));
}

#[test]
fn evaluate_timeout_tick_silence_fires_when_neither_clock_recent() {
    // Both `last_event_at` and `last_byte_at` are stale beyond budget,
    // and no in-flight items — the byte heartbeat must NOT mask a
    // genuine zero-byte hang.
    let config = TimeoutConfig {
        timeout: None,
        step_timeout: Some(Duration::from_secs(5)),
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
        matches!(
            result,
            WatchdogTickResult::Breach(ref w) if w.reason == WatchdogTerminationReason::StepTimeout
        ),
        "stale byte clock must allow step_timeout to fire; got: {result:?}"
    );
    assert!(fired.load(Ordering::SeqCst));
}

#[test]
fn evaluate_timeout_tick_silence_suppressed_when_only_byte_clock_set() {
    // Defensive case: no structured events ever fired, but bytes are
    // flowing (a provider that emits only raw text on stdout). The
    // first-event grace on `last_event_at` is satisfied transitively
    // by `last_byte_at` via `last_activity_at()`.
    let config = TimeoutConfig {
        timeout: None,
        step_timeout: Some(Duration::from_secs(5)),
        ..Default::default()
    };
    let state = Arc::new(std::sync::Mutex::new(WatchdogState::default()));
    let metrics = claudine::stream::progress::new_live_metrics();
    let fired = AtomicBool::new(false);
    let now_t = Instant::now();

    {
        let mut m = metrics.lock().unwrap();
        m.last_event_at = None;
        m.last_byte_at = Some(now_t);
    }

    let result = evaluate_timeout_tick(&config, now_t, now_t, &state, &metrics, &fired);
    assert_eq!(
        result,
        WatchdogTickResult::Ok,
        "byte clock alone must satisfy first-event grace and suppress; got: {result:?}"
    );
    assert!(!fired.load(Ordering::SeqCst));
}

#[test]
fn evaluate_timeout_tick_claude_in_flight_gate_still_suppresses() {
    let config = TimeoutConfig {
        timeout: None,
        step_timeout: Some(Duration::from_secs(5)),
        provider: Some(claudine::provider::Provider::Claude),
        ..Default::default()
    };
    let state = Arc::new(std::sync::Mutex::new(WatchdogState::default()));
    let metrics = claudine::stream::progress::new_live_metrics();
    let fired = AtomicBool::new(false);
    let t0 = Instant::now() - Duration::from_secs(30);
    let fresh = Instant::now();

    {
        let mut m = metrics.lock().unwrap();
        m.last_event_at = Some(t0);
        m.in_flight_subagents.insert(
            "sa-1".into(),
            claudine::stream::progress::InFlightSubagent {
                name: Some("rust-developer".into()),
                started_at: t0,
                last_progress_at: fresh,
            },
        );
    }

    let result = evaluate_timeout_tick(&config, Instant::now(), t0, &state, &metrics, &fired);
    assert_eq!(
        result,
        WatchdogTickResult::Ok,
        "Claude active subagent must suppress step_timeout via in-flight gate; got: {result:?}"
    );
    assert!(!fired.load(Ordering::SeqCst));
}

#[test]
fn evaluate_timeout_tick_claude_in_flight_gate_fires_when_stuck() {
    // Counter-test for the previous: when a Claude subagent is stuck
    // (no progress for longer than budget) the in-flight gate releases
    // and step_timeout fires normally, just as on any other provider.
    let config = TimeoutConfig {
        timeout: None,
        step_timeout: Some(Duration::from_secs(5)),
        provider: Some(claudine::provider::Provider::Claude),
        ..Default::default()
    };
    let state = Arc::new(std::sync::Mutex::new(WatchdogState::default()));
    let metrics = claudine::stream::progress::new_live_metrics();
    let fired = AtomicBool::new(false);
    let t0 = Instant::now() - Duration::from_secs(30);

    {
        let mut m = metrics.lock().unwrap();
        m.last_event_at = Some(t0);
        m.in_flight_subagents.insert(
            "sa-1".into(),
            claudine::stream::progress::InFlightSubagent {
                name: Some("rust-developer".into()),
                started_at: t0,
                last_progress_at: t0,
            },
        );
    }

    let result = evaluate_timeout_tick(&config, Instant::now(), t0, &state, &metrics, &fired);
    match result {
        WatchdogTickResult::Breach(ref w) => {
            assert_eq!(w.reason, WatchdogTerminationReason::StepTimeout);
            assert!(w.message.contains("rust-developer"));
        }
        other => panic!(
            "Claude stuck subagent must release the in-flight gate; expected StepTimeout, got: {other:?}"
        ),
    }
    assert!(fired.load(Ordering::SeqCst));
}
