use std::process::Child;
use std::time::{Duration, Instant};

use claudine::stream::logs::EarlyTermination;
use claudine::stream::progress::LiveMetrics;
use color_eyre::eyre::Result;

/// Wait for the child with a timeout, sending SIGTERM then SIGKILL.
///
/// Returns `(exit_code, termination_kind)`.
#[cfg(unix)]
pub(crate) fn wait_with_timeout(
    child: &mut Child,
    seconds: u64,
) -> Result<(i32, claudine::harness::ProcessTermination)> {
    use std::time::{Duration, Instant};

    let deadline = Instant::now() + Duration::from_secs(seconds);
    let grace_period = Duration::from_secs(5);

    loop {
        match child.try_wait()? {
            Some(status) => {
                return Ok((
                    super::exit_code_from_status(status),
                    claudine::harness::ProcessTermination::Completed,
                ));
            }
            None => {
                if Instant::now() >= deadline {
                    tracing::warn!(
                        timeout_secs = seconds,
                        child_pid = child.id(),
                        "child process timed out; sending SIGTERM"
                    );
                    // Send SIGTERM
                    unsafe {
                        libc::kill(child.id() as i32, libc::SIGTERM);
                    }

                    // Wait for grace period
                    let kill_deadline = Instant::now() + grace_period;
                    loop {
                        match child.try_wait()? {
                            Some(status) => {
                                return Ok((
                                    super::exit_code_from_status(status),
                                    claudine::harness::ProcessTermination::TimedOut,
                                ));
                            }
                            None => {
                                if Instant::now() >= kill_deadline {
                                    tracing::warn!(
                                        timeout_secs = seconds,
                                        child_pid = child.id(),
                                        "child process did not exit after SIGTERM; sending SIGKILL"
                                    );
                                    // Send SIGKILL
                                    unsafe {
                                        libc::kill(child.id() as i32, libc::SIGKILL);
                                    }
                                    let status = child.wait()?;
                                    return Ok((
                                        super::exit_code_from_status(status),
                                        claudine::harness::ProcessTermination::TimedOut,
                                    ));
                                }
                                std::thread::sleep(Duration::from_millis(100));
                            }
                        }
                    }
                }
                std::thread::sleep(Duration::from_millis(100));
            }
        }
    }
}

#[cfg(not(unix))]
pub(crate) fn wait_with_timeout(
    child: &mut Child,
    seconds: u64,
) -> Result<(i32, claudine::harness::ProcessTermination)> {
    use std::time::{Duration, Instant};

    let deadline = Instant::now() + Duration::from_secs(seconds);

    loop {
        match child.try_wait()? {
            Some(status) => {
                return Ok((
                    super::exit_code_from_status(status),
                    claudine::harness::ProcessTermination::Completed,
                ));
            }
            None => {
                if Instant::now() >= deadline {
                    tracing::warn!(
                        timeout_secs = seconds,
                        child_pid = child.id(),
                        "child process timed out; killing process"
                    );
                    child.kill()?;
                    let status = child.wait()?;
                    return Ok((
                        super::exit_code_from_status(status),
                        claudine::harness::ProcessTermination::TimedOut,
                    ));
                }
                std::thread::sleep(Duration::from_millis(100));
            }
        }
    }
}

/// Detect a step-silence timeout for the harness `step_timeout` field.
///
/// Returns `Some(EarlyTermination::StepTimeout)` when the time since the last
/// stream event exceeds `step_timeout`. Returns `None` when `last_event_at`
/// is not yet populated (first-event grace so provider startup does not
/// trip a kill), when silence is still under budget, or when in-flight tools
/// or subagents are still active.
///
/// This helper gates on `in_flight` and `in_flight_subagents`: a long-running
/// Task/subagent call produces parent-stream silence by design while the
/// child works. The wall-clock `timeout` rule serves as the backstop for
/// truly stuck tool calls. The caller is responsible for SIGTERM escalation.
#[allow(dead_code)]
pub(crate) fn detect_step_timeout(
    metrics: &LiveMetrics,
    now: Instant,
    step_timeout: Duration,
) -> Option<EarlyTermination> {
    let state = metrics.lock().ok()?;
    let last_event_at = state.last_event_at?;

    // Stuck-aware evaluation: only suppress step_timeout when ALL in-flight
    // items are active (none stuck). If any item is stuck, the silence rule
    // is allowed to fire so hung work does not block termination indefinitely.
    let stuck_tools = state.stuck_tools(now, step_timeout);
    let stuck_subagents = state.stuck_subagents(now, step_timeout);
    let any_stuck = !stuck_tools.is_empty() || !stuck_subagents.is_empty();
    let any_active = !state.in_flight.is_empty() || !state.in_flight_subagents.is_empty();

    if any_active && !any_stuck {
        return None;
    }

    let silence = now.saturating_duration_since(last_event_at);
    if silence >= step_timeout {
        let silence_text = format_internal_duration(silence.as_secs());
        let message = if stuck_tools.is_empty() && stuck_subagents.is_empty() {
            format!("no stream activity for {silence_text}; terminating due to step_timeout")
        } else {
            let mut msg = format!(
                "no stream activity for {silence_text}. The wrapped process was terminated."
            );
            if !stuck_tools.is_empty() {
                let count = stuck_tools.len();
                let plural = if count == 1 { "tool" } else { "tools" };
                msg.push_str(&format!(
                    " {count} {plural} were stuck when the timeout fired:\n"
                ));
                for tool in stuck_tools {
                    let name = tool.name.as_deref().unwrap_or("(unnamed)");
                    msg.push_str(&format!("  • \"{name}\"\n"));
                }
            }
            if !stuck_subagents.is_empty() {
                let count = stuck_subagents.len();
                let plural = if count == 1 { "subagent" } else { "subagents" };
                msg.push_str(&format!(
                    " {count} {plural} were stuck when the timeout fired:\n"
                ));
                for subagent in stuck_subagents {
                    let name = subagent.name.as_deref().unwrap_or("(unnamed)");
                    msg.push_str(&format!("  • \"{name}\"\n"));
                }
            }
            msg
        };
        Some(EarlyTermination::StepTimeout {
            message,
            outstanding: Vec::new(),
        })
    } else {
        None
    }
}

/// Format a duration in seconds for internal early-termination messages.
///
/// Used by the step-silence and OpenCode-hang detectors to compose their
/// `EarlyTermination::*` messages (these feed the summary, not the
/// user-visible timing surface). Kept as a small local helper so the
/// internal format stays stable.
pub(crate) fn format_internal_duration(secs: u64) -> String {
    if secs < 60 {
        format!("{secs}s")
    } else if secs < 3_600 {
        format!("{}", secs / 60)
    } else {
        format!("{}h{}m", secs / 3_600, (secs % 3_600) / 60)
    }
}

/// Unified timeout configuration for the watchdog ticker.
///
/// There are exactly two timeout rules:
///
/// - `timeout` — wall-clock budget from child spawn. `None` disables the
///   wall-clock kill (no built-in default).
/// - `step_timeout` — silence-since-last-parent-stream-event budget. `None`
///   disables the silence kill.
///
/// Plus two supporting knobs that govern the termination path itself:
///
/// - `kill_grace` — interval between SIGTERM and SIGKILL escalation
///   (default `10s`).
/// - `interval` — ticker cadence for evaluating the two rules
///   (default `5s`).
///
/// `kill_grace` and `interval` may be overridden by the
/// `CLAUDINE_KILL_GRACE` and `CLAUDINE_WATCHDOG_INTERVAL` env vars; the
/// `timeout` and `step_timeout` values themselves are resolved by the
/// composition layer (CLI > frontmatter > env > built-in default) and
/// passed in pre-resolved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TimeoutConfig {
    /// Wall-clock kill threshold. `None` disables.
    pub(crate) timeout: Option<Duration>,
    /// Stream-silence kill threshold. `None` disables.
    pub(crate) step_timeout: Option<Duration>,
    /// SIGTERM → SIGKILL grace period.
    pub(crate) kill_grace: Duration,
    /// Watchdog ticker cadence.
    pub(crate) interval: Duration,
    /// Wrapped provider, when known. Threaded through so the silence-rule
    /// evaluator can apply provider-specific guards (notably the OpenCode
    /// `provider_status` grace that suppresses `step_timeout` until at
    /// least one `step_finish` boundary has been observed). `None`
    /// disables all provider-specific guards; the wall-clock `timeout`
    /// rule is unaffected.
    pub(crate) provider: Option<claudine::provider::Provider>,
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            timeout: None,
            step_timeout: None,
            kill_grace: Duration::from_secs(10),
            interval: Duration::from_secs(5),
            provider: None,
        }
    }
}

impl TimeoutConfig {
    /// Build a [`TimeoutConfig`] from already-resolved `timeout` and
    /// `step_timeout` values, reading `CLAUDINE_KILL_GRACE` and
    /// `CLAUDINE_WATCHDOG_INTERVAL` from the environment for the
    /// supporting knobs.
    ///
    /// The `timeout` and `step_timeout` arguments come from the composition
    /// layer's precedence chain (CLI > frontmatter > env > built-in
    /// default); this function intentionally does NOT consult env vars for
    /// them — that single source-of-truth lives in `composition.rs`.
    ///
    /// Env values for `kill_grace` and `interval` use the
    /// [`claudine::harness::parse_timeout`] grammar (e.g. `30s`, `5m`,
    /// `2h`). Invalid or missing env values fall back to the built-in
    /// defaults (`10s` and `5s`).
    pub(crate) fn resolve(timeout: Option<Duration>, step_timeout: Option<Duration>) -> Self {
        let defaults = Self::default();
        let kill_grace = parse_env_duration("CLAUDINE_KILL_GRACE").unwrap_or(defaults.kill_grace);
        let interval =
            parse_env_duration("CLAUDINE_WATCHDOG_INTERVAL").unwrap_or(defaults.interval);
        Self {
            timeout,
            step_timeout,
            kill_grace,
            interval,
            provider: None,
        }
    }

    /// Set the wrapped provider so the silence-rule evaluator can apply
    /// provider-specific guards (notably the OpenCode `provider_status`
    /// grace).
    pub(crate) fn with_provider(mut self, provider: claudine::provider::Provider) -> Self {
        self.provider = Some(provider);
        self
    }

    /// Returns `true` when the wall-clock rule is enabled.
    pub(crate) fn timeout_enabled(&self) -> bool {
        self.timeout.is_some()
    }

    /// Returns `true` when the stream-silence rule is enabled.
    pub(crate) fn step_timeout_enabled(&self) -> bool {
        self.step_timeout.is_some()
    }

    /// Returns `true` when any rule is enabled.
    pub(crate) fn any_enabled(&self) -> bool {
        self.timeout_enabled() || self.step_timeout_enabled()
    }
}

/// Parse a duration env var using the harness `parse_timeout` grammar.
///
/// Returns `None` when the variable is unset, empty, or unparseable.
fn parse_env_duration(name: &str) -> Option<Duration> {
    let raw = std::env::var(name).ok()?;
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    claudine::harness::parse_timeout(trimmed, std::path::Path::new("<env>")).ok()
}

#[cfg(test)]
mod tests {
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
}
