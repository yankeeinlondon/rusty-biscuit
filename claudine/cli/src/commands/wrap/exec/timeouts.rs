use std::time::{Duration, Instant};

use claudine::stream::logs::EarlyTermination;
use claudine::stream::progress::LiveMetrics;

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

    pub(crate) fn timeout_enabled(&self) -> bool {
        self.timeout.is_some()
    }

    pub(crate) fn step_timeout_enabled(&self) -> bool {
        self.step_timeout.is_some()
    }

    pub(crate) fn any_enabled(&self) -> bool {
        self.timeout_enabled() || self.step_timeout_enabled()
    }
}

/// Parse a duration env var using the harness `parse_timeout` grammar.
///
/// Returns `None` when the variable is unset, empty, or unparseable.
pub(crate) fn parse_env_duration(name: &str) -> Option<Duration> {
    let raw = std::env::var(name).ok()?;
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    claudine::harness::parse_timeout(trimmed, std::path::Path::new("<env>")).ok()
}

#[cfg(test)]
mod tests;
