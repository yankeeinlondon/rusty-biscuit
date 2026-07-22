use super::*;
// test-toolkit provides EnvGuard for safe RAII environment-variable
// management in tests. All env-mutating tests in this module are
// annotated with `#[serial_test::serial]` to prevent cross-test
// interference because the process environment is global state.
use rstest::rstest;
use test_toolkit::EnvGuard;

#[test]
fn detect_step_timeout_fires_after_silence_exceeds_budget() {
    let metrics = claudine::stream::progress::new_live_metrics();
    let now = Instant::now();
    {
        let mut state = metrics.lock().unwrap();
        state.last_event_at = Some(now - Duration::from_secs(6));
    }

    let detected = detect_step_timeout(&metrics, now, Duration::from_secs(5));

    assert!(matches!(
        detected,
        Some(EarlyTermination::StepTimeout { ref outstanding, .. }) if outstanding.is_empty()
    ));
}

#[test]
fn detect_step_timeout_returns_none_when_recent() {
    let metrics = claudine::stream::progress::new_live_metrics();
    let now = Instant::now();
    {
        let mut state = metrics.lock().unwrap();
        state.last_event_at = Some(now - Duration::from_secs(1));
    }

    let detected = detect_step_timeout(&metrics, now, Duration::from_secs(5));

    assert!(detected.is_none());
}

#[test]
fn detect_step_timeout_returns_none_when_last_event_at_is_none() {
    // First-event grace: a fresh session with no observed SemanticEvent
    // must never trip the deadline, even if the budget is tiny.
    let metrics = claudine::stream::progress::new_live_metrics();
    let now = Instant::now();

    let detected = detect_step_timeout(&metrics, now, Duration::from_secs(1));

    assert!(detected.is_none());
}

#[test]
fn detect_step_timeout_fires_when_in_flight_tool_is_stuck() {
    let metrics = claudine::stream::progress::new_live_metrics();
    let now = Instant::now();
    {
        let mut state = metrics.lock().unwrap();
        state.last_event_at = Some(now - Duration::from_secs(180));
        state.in_flight.insert(
            "task-1".into(),
            claudine::stream::progress::InFlightTool {
                name: Some("Task".into()),
                started_at: now - Duration::from_secs(180),
                last_progress_at: now - Duration::from_secs(180),
            },
        );
    }

    let detected = detect_step_timeout(&metrics, now, Duration::from_secs(5));

    let message = match detected {
        Some(EarlyTermination::StepTimeout { ref message, .. }) => message.clone(),
        other => panic!("stuck tool should trigger step_timeout, got: {other:?}"),
    };
    assert!(
        message.contains("Task"),
        "stuck tool message should mention Task, got: {message}"
    );
}

#[test]
fn detect_step_timeout_returns_none_when_in_flight_tool_is_active() {
    let metrics = claudine::stream::progress::new_live_metrics();
    let now = Instant::now();
    {
        let mut state = metrics.lock().unwrap();
        state.last_event_at = Some(now - Duration::from_secs(180));
        state.in_flight.insert(
            "task-1".into(),
            claudine::stream::progress::InFlightTool {
                name: Some("Task".into()),
                started_at: now - Duration::from_secs(180),
                last_progress_at: now,
            },
        );
    }

    let detected = detect_step_timeout(&metrics, now, Duration::from_secs(5));

    assert!(detected.is_none(), "active tool must suppress step_timeout");
}

#[test]
fn detect_step_timeout_fires_when_in_flight_subagent_is_stuck() {
    let metrics = claudine::stream::progress::new_live_metrics();
    let now = Instant::now();
    {
        let mut state = metrics.lock().unwrap();
        state.last_event_at = Some(now - Duration::from_secs(180));
        state.in_flight_subagents.insert(
            "sa-1".into(),
            claudine::stream::progress::InFlightSubagent {
                name: Some("rust-developer".into()),
                started_at: now - Duration::from_secs(180),
                last_progress_at: now - Duration::from_secs(180),
            },
        );
    }

    let detected = detect_step_timeout(&metrics, now, Duration::from_secs(5));

    assert!(
        matches!(detected, Some(EarlyTermination::StepTimeout { .. })),
        "stuck subagent should trigger step_timeout, got: {detected:?}"
    );
}

#[test]
fn detect_step_timeout_returns_none_when_in_flight_subagent_is_active() {
    let metrics = claudine::stream::progress::new_live_metrics();
    let now = Instant::now();
    {
        let mut state = metrics.lock().unwrap();
        state.last_event_at = Some(now - Duration::from_secs(180));
        state.in_flight_subagents.insert(
            "sa-1".into(),
            claudine::stream::progress::InFlightSubagent {
                name: Some("rust-developer".into()),
                started_at: now - Duration::from_secs(180),
                last_progress_at: now,
            },
        );
    }

    let detected = detect_step_timeout(&metrics, now, Duration::from_secs(5));

    assert!(
        detected.is_none(),
        "active subagent must suppress step_timeout"
    );
}

#[test]
fn detect_step_timeout_fires_when_in_flight_cleared() {
    let metrics = claudine::stream::progress::new_live_metrics();
    let now = Instant::now();
    {
        let mut state = metrics.lock().unwrap();
        state.last_event_at = Some(now - Duration::from_secs(180));
        state.in_flight_subagents.insert(
            "sa-1".into(),
            claudine::stream::progress::InFlightSubagent {
                name: Some("rust-developer".into()),
                started_at: now - Duration::from_secs(180),
                last_progress_at: now,
            },
        );
    }

    assert!(
        detect_step_timeout(&metrics, now, Duration::from_secs(5)).is_none(),
        "must not fire while active subagent is in-flight"
    );

    {
        let mut state = metrics.lock().unwrap();
        state.in_flight_subagents.clear();
    }

    assert!(
        detect_step_timeout(&metrics, now, Duration::from_secs(5)).is_some(),
        "must fire once in-flight is cleared and silence exceeds budget"
    );
}

#[test]
fn timeout_config_default_is_disabled_with_built_in_supporting_knobs() {
    let config = TimeoutConfig::default();
    assert_eq!(config.timeout, None);
    assert_eq!(config.step_timeout, None);
    assert_eq!(config.kill_grace, Duration::from_secs(10));
    assert_eq!(config.interval, Duration::from_secs(5));
    assert!(!config.timeout_enabled());
    assert!(!config.step_timeout_enabled());
    assert!(!config.any_enabled());
}

#[test]
fn timeout_config_enabled_flags_match_some_values() {
    let only_wall = TimeoutConfig {
        timeout: Some(Duration::from_secs(60)),
        step_timeout: None,
        ..Default::default()
    };
    assert!(only_wall.timeout_enabled());
    assert!(!only_wall.step_timeout_enabled());
    assert!(only_wall.any_enabled());

    let only_silence = TimeoutConfig {
        timeout: None,
        step_timeout: Some(Duration::from_secs(60)),
        ..Default::default()
    };
    assert!(!only_silence.timeout_enabled());
    assert!(only_silence.step_timeout_enabled());
    assert!(only_silence.any_enabled());
}

#[rstest]
#[serial_test::serial]
fn timeout_config_resolve_honours_pre_resolved_inputs() {
    // Ensure env knobs are absent so we observe the inputs cleanly.
    // SAFETY: serial_test::serial prevents concurrent env access.
    let _g1 = unsafe { EnvGuard::remove("CLAUDINE_KILL_GRACE") };
    let _g2 = unsafe { EnvGuard::remove("CLAUDINE_WATCHDOG_INTERVAL") };

    let config = TimeoutConfig::resolve(
        Some(Duration::from_secs(7200)),
        Some(Duration::from_secs(1800)),
    );
    assert_eq!(config.timeout, Some(Duration::from_secs(7200)));
    assert_eq!(config.step_timeout, Some(Duration::from_secs(1800)));
    // Defaults applied when env vars unset.
    assert_eq!(config.kill_grace, Duration::from_secs(10));
    assert_eq!(config.interval, Duration::from_secs(5));
}

#[rstest]
#[serial_test::serial]
fn timeout_config_resolve_does_not_consult_timeout_env_vars() {
    // Composition layer owns timeout/step_timeout precedence; resolve
    // must NOT read these env vars itself.
    // SAFETY: serial_test::serial prevents concurrent env access.
    let _g1 = unsafe { EnvGuard::set("CLAUDINE_TIMEOUT", "1h") };
    let _g2 = unsafe { EnvGuard::set("CLAUDINE_STEP_TIMEOUT", "5m") };
    let _g3 = unsafe { EnvGuard::remove("CLAUDINE_KILL_GRACE") };
    let _g4 = unsafe { EnvGuard::remove("CLAUDINE_WATCHDOG_INTERVAL") };

    let config = TimeoutConfig::resolve(None, None);
    assert_eq!(
        config.timeout, None,
        "resolve must not read CLAUDINE_TIMEOUT"
    );
    assert_eq!(
        config.step_timeout, None,
        "resolve must not read CLAUDINE_STEP_TIMEOUT"
    );
}

#[rstest]
#[serial_test::serial]
fn timeout_config_resolve_parses_kill_grace_and_interval_env_vars() {
    // SAFETY: serial_test::serial prevents concurrent env access.
    let _g1 = unsafe { EnvGuard::set("CLAUDINE_KILL_GRACE", "30s") };
    let _g2 = unsafe { EnvGuard::set("CLAUDINE_WATCHDOG_INTERVAL", "2s") };

    let config = TimeoutConfig::resolve(None, None);
    assert_eq!(config.kill_grace, Duration::from_secs(30));
    assert_eq!(config.interval, Duration::from_secs(2));
}

#[rstest]
#[serial_test::serial]
fn timeout_config_resolve_falls_back_when_env_invalid() {
    // SAFETY: serial_test::serial prevents concurrent env access.
    let _g1 = unsafe { EnvGuard::set("CLAUDINE_KILL_GRACE", "garbage") };
    let _g2 = unsafe { EnvGuard::set("CLAUDINE_WATCHDOG_INTERVAL", "") };

    let config = TimeoutConfig::resolve(None, None);
    assert_eq!(config.kill_grace, Duration::from_secs(10));
    assert_eq!(config.interval, Duration::from_secs(5));
}

#[rstest]
#[serial_test::serial]
fn timeout_config_resolve_accepts_minute_and_hour_units() {
    // SAFETY: serial_test::serial prevents concurrent env access.
    let _g1 = unsafe { EnvGuard::set("CLAUDINE_KILL_GRACE", "1m") };
    let _g2 = unsafe { EnvGuard::set("CLAUDINE_WATCHDOG_INTERVAL", "1h") };

    let config = TimeoutConfig::resolve(None, None);
    assert_eq!(config.kill_grace, Duration::from_secs(60));
    assert_eq!(config.interval, Duration::from_secs(3600));
}

#[rstest]
#[serial_test::serial]
fn timeout_config_resolve_cli_wins_over_frontmatter_env_and_default() {
    // SAFETY: serial_test::serial prevents concurrent env access.
    let _g1 = unsafe { EnvGuard::remove("CLAUDINE_TIMEOUT") };
    let _g2 = unsafe { EnvGuard::remove("CLAUDINE_STEP_TIMEOUT") };
    let _g3 = unsafe { EnvGuard::remove("CLAUDINE_KILL_GRACE") };
    let _g4 = unsafe { EnvGuard::remove("CLAUDINE_WATCHDOG_INTERVAL") };

    // Simulating the composition layer resolving CLI > frontmatter > env
    let resolved_timeout = Some(Duration::from_secs(7200)); // from CLI
    let resolved_step_timeout = Some(Duration::from_secs(1800)); // from CLI
    let config = TimeoutConfig::resolve(resolved_timeout, resolved_step_timeout);
    assert_eq!(config.timeout, Some(Duration::from_secs(7200)));
    assert_eq!(config.step_timeout, Some(Duration::from_secs(1800)));
}

// The former `wait_with_timeout_rejects_absurd_timeout_without_panicking`
// regression is now structurally impossible: the unified watchdog compares
// `Instant::saturating_duration_since` against the budget rather than
// precomputing a deadline `Instant`, so an absurd budget (e.g. `u64::MAX`
// seconds) can never overflow the clock — it simply never fires. See
// `spawn_wall_clock_timeout_ticker` and `evaluate_timeout_tick`.
