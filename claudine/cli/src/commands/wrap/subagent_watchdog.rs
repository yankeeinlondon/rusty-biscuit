// Helpers and types will be used in Phase 2-5; silence dead-code lints
// until then.
#![allow(dead_code)]

//! Subagent watchdog state model.
//!
//! Tracks active subagents across the lifetime of a wrapped provider session.
//! The state is intentionally decoupled from child-process termination so it
//! can be unit-tested in isolation and reused by both the live semantic sink
//! and the exec-layer watchdog ticker.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use claudine::stream::progress::LiveMetrics;

/// Identifier for a subagent.
pub(crate) type SubagentId = String;

/// Mutable information stored for an active subagent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ActiveSubagentInfo {
    /// Optional human-readable name or title.
    pub(crate) name: Option<String>,
    /// When the subagent was first observed starting.
    pub(crate) started_at: Instant,
    /// When the subagent last made progress.
    pub(crate) last_progress_at: Instant,
    /// When a diagnostic line was last emitted for this subagent.
    pub(crate) last_diagnostic_at: Option<Instant>,
}

/// Read-only snapshot of an active subagent suitable for diagnostics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ActiveSubagentSnapshot {
    pub(crate) id: SubagentId,
    pub(crate) name: Option<String>,
    pub(crate) started_at: Instant,
    pub(crate) last_progress_at: Instant,
    pub(crate) elapsed_since_start: Duration,
    pub(crate) elapsed_since_progress: Duration,
}

/// A single diagnostic line for a stuck subagent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SubagentDiagnosticLine {
    pub(crate) id: SubagentId,
    pub(crate) display_name: String,
    pub(crate) elapsed_since_start: Duration,
}

/// Reason for a watchdog-initiated termination.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum WatchdogTerminationReason {
    /// One or more subagents have been idle longer than the threshold.
    SubagentsUnresponsive,
    /// The provider stream has been completely idle with no outstanding
    /// subagents for longer than the threshold.
    StreamIdleTimeout,
}

/// Request sent by the watchdog ticker to the exec wait loop asking for
/// child-process termination.
///
/// Carries the reason, a human-readable message, and optional snapshots
/// of stuck subagents so the summary can be enriched with details.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WatchdogTermination {
    pub(crate) reason: WatchdogTerminationReason,
    pub(crate) message: String,
    pub(crate) stuck_subagents: Vec<ActiveSubagentSnapshot>,
}

/// Shared watchdog state tracking active subagents.
///
/// All mutation methods accept an explicit `now` so tests can drive time
/// deterministically. Thin convenience wrappers that call `Instant::now()`
/// are provided for production call sites.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct WatchdogState {
    active: HashMap<SubagentId, ActiveSubagentInfo>,
}

impl WatchdogState {
    /// Record that a subagent started.
    ///
    /// If the id is already known, metadata is refreshed without resetting
    /// timestamps that the caller did not provide. This matches the contract
    /// that duplicate starts update metadata without losing timestamps
    /// unexpectedly.
    pub(crate) fn subagent_started(
        &mut self,
        id: SubagentId,
        name: Option<String>,
        now: Instant,
    ) {
        match self.active.get_mut(&id) {
            Some(info) => {
                if name.is_some() {
                    info.name = name;
                }
            }
            None => {
                self.active.insert(
                    id,
                    ActiveSubagentInfo {
                        name,
                        started_at: now,
                        last_progress_at: now,
                        last_diagnostic_at: None,
                    },
                );
            }
        }
    }

    /// Convenience: record a subagent start using the current instant.
    #[allow(dead_code)]
    pub(crate) fn subagent_started_now(&mut self, id: SubagentId, name: Option<String>) {
        self.subagent_started(id, name, Instant::now());
    }

    /// Record that a subagent stopped.
    pub(crate) fn subagent_stopped(&mut self, id: &SubagentId, _now: Instant) {
        self.active.remove(id);
    }

    /// Convenience: record a subagent stop using the current instant.
    #[allow(dead_code)]
    pub(crate) fn subagent_stopped_now(&mut self, id: &SubagentId) {
        self.subagent_stopped(id, Instant::now());
    }

    /// Record progress for an active subagent.
    ///
    /// Silently ignores unknown ids so callers do not need to gate on
    /// `SubagentStart` having already been seen.
    pub(crate) fn observe_subagent_progress(&mut self, id: &SubagentId, now: Instant) {
        if let Some(info) = self.active.get_mut(id) {
            info.last_progress_at = now;
        }
    }

    /// Convenience: record progress using the current instant.
    #[allow(dead_code)]
    pub(crate) fn observe_subagent_progress_now(&mut self, id: &SubagentId) {
        self.observe_subagent_progress(id, Instant::now());
    }

    /// Return a snapshot of every currently active subagent.
    pub(crate) fn active_subagents(&self, now: Instant) -> Vec<ActiveSubagentSnapshot> {
        self.active
            .iter()
            .map(|(id, info)| ActiveSubagentSnapshot {
                id: id.clone(),
                name: info.name.clone(),
                started_at: info.started_at,
                last_progress_at: info.last_progress_at,
                elapsed_since_start: now.saturating_duration_since(info.started_at),
                elapsed_since_progress: now.saturating_duration_since(info.last_progress_at),
            })
            .collect()
    }

    /// Return active subagents whose time since last progress exceeds the
    /// given threshold.
    pub(crate) fn stuck_subagents(
        &self,
        now: Instant,
        threshold: Duration,
    ) -> Vec<ActiveSubagentSnapshot> {
        self.active
            .iter()
            .filter(|(_, info)| now.saturating_duration_since(info.last_progress_at) >= threshold)
            .map(|(id, info)| ActiveSubagentSnapshot {
                id: id.clone(),
                name: info.name.clone(),
                started_at: info.started_at,
                last_progress_at: info.last_progress_at,
                elapsed_since_start: now.saturating_duration_since(info.started_at),
                elapsed_since_progress: now.saturating_duration_since(info.last_progress_at),
            })
            .collect()
    }

    /// Produce diagnostic lines for active subagents.
    ///
    /// Each subagent emits at most one diagnostic per `silence_window`.
    /// On emission, `last_diagnostic_at` is updated so subsequent calls
    /// within the same window are suppressed.
    pub(crate) fn diagnostic_lines(
        &mut self,
        now: Instant,
        silence_window: Duration,
    ) -> Vec<SubagentDiagnosticLine> {
        let mut lines = Vec::new();
        for (id, info) in &mut self.active {
            let elapsed = now.saturating_duration_since(info.started_at);
            if let Some(last) = info.last_diagnostic_at
                && now.saturating_duration_since(last) < silence_window
            {
                continue;
            }
            let display_name = info
                .name
                .as_deref()
                .filter(|n| !n.is_empty())
                .unwrap_or(id)
                .to_string();
            lines.push(SubagentDiagnosticLine {
                id: id.clone(),
                display_name,
                elapsed_since_start: elapsed,
            });
            info.last_diagnostic_at = Some(now);
        }
        lines
    }
}

/// Configuration for the subagent and stream-idle watchdogs, parsed from
/// environment variables.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct WatchdogConfig {
    /// Per-subagent silence ceiling before SIGTERM. `0` disables subagent
    /// watchdog and diagnostic emissions.
    pub(crate) subagent_idle_kill_seconds: u64,
    /// Grace period between SIGTERM and SIGKILL for watchdog-initiated
    /// termination.
    pub(crate) subagent_kill_grace_seconds: u64,
    /// Ticker cadence for evaluating watchdog rules.
    pub(crate) subagent_watchdog_interval_seconds: u64,
    /// Stream-level silence ceiling when no subagents are outstanding.
    /// `0` disables stream-idle watchdog.
    pub(crate) stream_idle_kill_seconds: u64,
}

impl Default for WatchdogConfig {
    fn default() -> Self {
        Self {
            subagent_idle_kill_seconds: 180,
            subagent_kill_grace_seconds: 10,
            subagent_watchdog_interval_seconds: 5,
            stream_idle_kill_seconds: 300,
        }
    }
}

impl WatchdogConfig {
    /// Parse configuration from environment variables. Missing or invalid
    /// variables fall back to the documented defaults.
    pub(crate) fn from_env() -> Self {
        let mut config = Self::default();
        if let Ok(v) = std::env::var("CLAUDINE_SUBAGENT_IDLE_KILL_SECONDS")
            && let Ok(n) = v.parse::<u64>()
        {
            config.subagent_idle_kill_seconds = n;
        }
        if let Ok(v) = std::env::var("CLAUDINE_SUBAGENT_KILL_GRACE_SECONDS")
            && let Ok(n) = v.parse::<u64>()
        {
            config.subagent_kill_grace_seconds = n;
        }
        if let Ok(v) = std::env::var("CLAUDINE_SUBAGENT_WATCHDOG_INTERVAL_SECONDS")
            && let Ok(n) = v.parse::<u64>()
        {
            config.subagent_watchdog_interval_seconds = n;
        }
        if let Ok(v) = std::env::var("CLAUDINE_STREAM_IDLE_KILL_SECONDS")
            && let Ok(n) = v.parse::<u64>()
        {
            config.stream_idle_kill_seconds = n;
        }
        config
    }

    /// Returns `true` when the subagent watchdog is enabled (threshold > 0).
    pub(crate) fn subagent_watchdog_enabled(&self) -> bool {
        self.subagent_idle_kill_seconds > 0
    }

    /// Returns `true` when the stream-idle watchdog is enabled (threshold > 0).
    pub(crate) fn stream_idle_watchdog_enabled(&self) -> bool {
        self.stream_idle_kill_seconds > 0
    }

    /// Returns `true` when any watchdog rule is enabled.
    pub(crate) fn any_watchdog_enabled(&self) -> bool {
        self.subagent_watchdog_enabled() || self.stream_idle_watchdog_enabled()
    }
}

/// Result of evaluating watchdog rules on a single tick.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum WatchdogTickResult {
    /// No rules triggered; continue monitoring.
    Ok,
    /// Subagent silence threshold breached; terminate with this request.
    SubagentBreach(WatchdogTermination),
    /// Stream-idle threshold breached; terminate with this request.
    StreamIdleBreach(WatchdogTermination),
}

/// Evaluate watchdog rules for the current tick.
///
/// Rules are evaluated in priority order:
/// 1. If active subagents exist and any exceeded the idle threshold,
///    return a subagent breach.
/// 2. If active subagents exist but none is over threshold, suppress
///    stream-idle evaluation for this tick.
/// 3. If no subagents are active, evaluate stream-level silence using
///    `LiveMetrics.last_event_at`, only when at least one activity event
///    has been observed.
///
/// `fired` is an atomic flag that prevents double-fire; once set to
/// `true`, all subsequent evaluations return `Ok`.
pub(crate) fn evaluate_watchdog_tick(
    config: &WatchdogConfig,
    now: Instant,
    watchdog_state: &Arc<Mutex<WatchdogState>>,
    live_metrics: &LiveMetrics,
    fired: &AtomicBool,
) -> WatchdogTickResult {
    // One-shot guard: after the first breach, never fire again.
    if fired.load(Ordering::SeqCst) {
        return WatchdogTickResult::Ok;
    }

    // Rule 1: subagent silence.
    if config.subagent_watchdog_enabled() {
        let stuck = {
            let state = match watchdog_state.lock() {
                Ok(g) => g,
                Err(_) => return WatchdogTickResult::Ok,
            };
            state.stuck_subagents(
                now,
                Duration::from_secs(config.subagent_idle_kill_seconds),
            )
        };

        if !stuck.is_empty() {
            fired.store(true, Ordering::SeqCst);
            let message = format_subagent_breach_message(&stuck);
            return WatchdogTickResult::SubagentBreach(WatchdogTermination {
                reason: WatchdogTerminationReason::SubagentsUnresponsive,
                message,
                stuck_subagents: stuck,
            });
        }

        // Subagents are active but not yet stuck — suppress stream-idle.
        let has_active = {
            let state = match watchdog_state.lock() {
                Ok(g) => g,
                Err(_) => return WatchdogTickResult::Ok,
            };
            !state.active_subagents(now).is_empty()
        };
        if has_active {
            return WatchdogTickResult::Ok;
        }
    }

    // Rule 2: stream-level silence (only when no subagents are outstanding).
    if config.stream_idle_watchdog_enabled() {
        let breach = {
            let metrics = match live_metrics.lock() {
                Ok(g) => g,
                Err(_) => return WatchdogTickResult::Ok,
            };
            if let Some(last_event_at) = metrics.last_event_at {
                let silence = now.saturating_duration_since(last_event_at);
                silence >= Duration::from_secs(config.stream_idle_kill_seconds)
            } else {
                false
            }
        };

        if breach {
            fired.store(true, Ordering::SeqCst);
            let threshold = config.stream_idle_kill_seconds;
            let message = format!(
                "no stream activity for {threshold}s; terminating due to stream_idle_timeout"
            );
            return WatchdogTickResult::StreamIdleBreach(WatchdogTermination {
                reason: WatchdogTerminationReason::StreamIdleTimeout,
                message,
                stuck_subagents: Vec::new(),
            });
        }
    }

    WatchdogTickResult::Ok
}

/// Format the human-readable breach message for stuck subagents.
fn format_subagent_breach_message(stuck: &[ActiveSubagentSnapshot]) -> String {
    let mut lines = String::new();
    let count = stuck.len();
    let plural = if count == 1 { "subagent" } else { "subagents" };
    lines.push_str(&format!(
        "{count} {plural} went silent and were terminated by the watchdog:\n"
    ));
    for snap in stuck {
        let idle = format_duration(snap.elapsed_since_progress);
        let name = snap.name.as_deref().unwrap_or("(unnamed)");
        lines.push_str(&format!("  • {} \"{name}\" (idle {idle})\n", snap.id));
    }
    lines.push_str("The wrapped process was terminated. Re-run the prompt to retry.");
    lines
}

/// Format a duration in a human-readable way (e.g. "3m 0s").
fn format_duration(d: Duration) -> String {
    let secs = d.as_secs();
    if secs < 60 {
        format!("{secs}s")
    } else if secs < 3_600 {
        format!("{}m {}s", secs / 60, secs % 60)
    } else {
        format!("{}h {}m {}s", secs / 3_600, (secs % 3_600) / 60, secs % 60)
    }
}

/// Render a watchdog breach as an `AgentNative` error block on stderr.
///
/// This is the standalone rendering path used by the watchdog ticker
/// thread so it can surface the error before the child is killed,
/// without needing access to the `LiveSemanticSink` section tracker.
pub(crate) fn render_watchdog_error_to_stream(
    termination: &WatchdogTermination,
    stream_output: &super::stream_io::StreamOutput,
) {
    use biscuit_terminal::components::prose::Prose;
    use biscuit_terminal::components::renderable::Renderable;
    use biscuit_terminal::components::status::StatusState;
    use biscuit_terminal::prelude::StatusBlock;
    use biscuit_terminal::utils::color::{Color, Tailwind};
    use biscuit_terminal::utils::layout::{Margin, WordWrap};

    let term = crate::log::terminal();
    let border_color = Color::Tailwind(Tailwind::Red700);
    let body_text = escape_prose(&termination.message);
    let body = format!("<red><b>Agent Error</b></red>\n{body_text}");
    let prose = Prose::new(body).with_word_wrap(WordWrap::WrapProse(None, None));
    let block = StatusBlock::new(StatusState::Error)
        .body(prose)
        .border_color(border_color)
        .left_margin(Margin::Chars(0))
        .right_margin(Margin::Chars(0));
    let rendered = block.render(&term);
    for line in rendered.lines() {
        stream_output.emit_stderr_line(line);
    }
}

/// Escape user-controlled text so it can be safely interpolated into
/// biscuit-terminal prose markup.
fn escape_prose(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '\\' | '<' | '>' | '{' => {
                out.push('\\');
                out.push(ch);
            }
            other => out.push(other),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn now() -> Instant {
        Instant::now()
    }

    #[test]
    fn starts_and_stops() {
        let mut state = WatchdogState::default();
        let t0 = now();
        state.subagent_started("a".into(), Some("Alpha".into()), t0);
        state.subagent_started("b".into(), Some("Beta".into()), t0);

        let active = state.active_subagents(t0);
        assert_eq!(active.len(), 2);
        assert!(active.iter().any(|s| s.id == "a" && s.name.as_deref() == Some("Alpha")));
        assert!(active.iter().any(|s| s.id == "b" && s.name.as_deref() == Some("Beta")));

        state.subagent_stopped(&"a".into(), t0);
        let active = state.active_subagents(t0);
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].id, "b");
    }

    #[test]
    fn duplicate_start_updates_metadata_without_losing_timestamps() {
        let mut state = WatchdogState::default();
        let t0 = now();
        state.subagent_started("a".into(), Some("Alpha".into()), t0);

        // Progress after 1s
        let t1 = t0 + Duration::from_secs(1);
        state.observe_subagent_progress(&"a".into(), t1);

        // Duplicate start with a new name
        let t2 = t0 + Duration::from_secs(2);
        state.subagent_started("a".into(), Some("Alpha-2".into()), t2);

        let active = state.active_subagents(t2);
        assert_eq!(active.len(), 1);
        let snap = &active[0];
        // Name updated
        assert_eq!(snap.name.as_deref(), Some("Alpha-2"));
        // Started_at preserved from first start
        assert_eq!(snap.started_at, t0);
        // Last progress preserved
        assert_eq!(snap.last_progress_at, t1);
    }

    #[test]
    fn progress_resets_only_matching_subagent() {
        let mut state = WatchdogState::default();
        let t0 = now();
        state.subagent_started("a".into(), None, t0);
        state.subagent_started("b".into(), None, t0);

        let t1 = t0 + Duration::from_secs(5);
        state.observe_subagent_progress(&"a".into(), t1);

        let active = state.active_subagents(t1);
        let a = active.iter().find(|s| s.id == "a").unwrap();
        let b = active.iter().find(|s| s.id == "b").unwrap();
        assert_eq!(a.last_progress_at, t1);
        assert_eq!(b.last_progress_at, t0);
    }

    #[test]
    fn stuck_subagents_respects_threshold() {
        let mut state = WatchdogState::default();
        let t0 = now();
        state.subagent_started("a".into(), None, t0);
        state.subagent_started("b".into(), None, t0);

        // a makes progress at t=2, b stays silent
        let t2 = t0 + Duration::from_secs(2);
        state.observe_subagent_progress(&"a".into(), t2);

        let t5 = t0 + Duration::from_secs(5);
        // a has been silent for 3s (5-2), b for 5s. threshold = 4s → only b.
        let stuck = state.stuck_subagents(t5, Duration::from_secs(4));
        assert_eq!(stuck.len(), 1);
        assert_eq!(stuck[0].id, "b");
    }

    #[test]
    fn diagnostic_lines_emitted_at_most_once_per_silence_window() {
        let mut state = WatchdogState::default();
        let t0 = now();
        state.subagent_started("a".into(), Some("Alpha".into()), t0);

        let window = Duration::from_secs(10);

        // First call emits
        let lines = state.diagnostic_lines(t0, window);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].id, "a");
        assert_eq!(lines[0].display_name, "Alpha");

        // Second call within window suppresses
        let t5 = t0 + Duration::from_secs(5);
        let lines = state.diagnostic_lines(t5, window);
        assert!(lines.is_empty());

        // After window passes, emits again
        let t15 = t0 + Duration::from_secs(15);
        let lines = state.diagnostic_lines(t15, window);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].id, "a");
    }

    #[test]
    fn diagnostic_lines_falls_back_to_id_when_name_missing() {
        let mut state = WatchdogState::default();
        let t0 = now();
        state.subagent_started("x".into(), None, t0);

        let lines = state.diagnostic_lines(t0, Duration::from_secs(10));
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].display_name, "x");
    }

    #[test]
    fn diagnostic_lines_includes_all_active_subagents() {
        let mut state = WatchdogState::default();
        let t0 = now();
        state.subagent_started("a".into(), Some("A".into()), t0);
        state.subagent_started("b".into(), Some("B".into()), t0);

        let lines = state.diagnostic_lines(t0, Duration::from_secs(10));
        assert_eq!(lines.len(), 2);
        assert!(lines.iter().any(|l| l.id == "a"));
        assert!(lines.iter().any(|l| l.id == "b"));
    }

    #[test]
    fn diagnostic_lines_skips_stopped_subagents() {
        let mut state = WatchdogState::default();
        let t0 = now();
        state.subagent_started("a".into(), None, t0);
        state.subagent_stopped(&"a".into(), t0);

        let lines = state.diagnostic_lines(t0, Duration::from_secs(10));
        assert!(lines.is_empty());
    }

    #[test]
    fn stuck_subagents_empty_when_all_active_below_threshold() {
        let mut state = WatchdogState::default();
        let t0 = now();
        state.subagent_started("a".into(), None, t0);

        let stuck = state.stuck_subagents(t0, Duration::from_secs(10));
        assert!(stuck.is_empty());
    }

    #[test]
    fn active_subagents_reflects_elapsed_times() {
        let mut state = WatchdogState::default();
        let t0 = now();
        state.subagent_started("a".into(), None, t0);

        let t7 = t0 + Duration::from_secs(7);
        let active = state.active_subagents(t7);
        assert_eq!(active[0].elapsed_since_start, Duration::from_secs(7));
        assert_eq!(active[0].elapsed_since_progress, Duration::from_secs(7));
    }

    // --- WatchdogConfig tests ---

    #[test]
    fn watchdog_config_default_values() {
        let config = WatchdogConfig::default();
        assert_eq!(config.subagent_idle_kill_seconds, 180);
        assert_eq!(config.subagent_kill_grace_seconds, 10);
        assert_eq!(config.subagent_watchdog_interval_seconds, 5);
        assert_eq!(config.stream_idle_kill_seconds, 300);
        assert!(config.subagent_watchdog_enabled());
        assert!(config.stream_idle_watchdog_enabled());
        assert!(config.any_watchdog_enabled());
    }

    #[test]
    fn watchdog_config_zero_disables() {
        let config = WatchdogConfig {
            subagent_idle_kill_seconds: 0,
            stream_idle_kill_seconds: 0,
            ..Default::default()
        };
        assert!(!config.subagent_watchdog_enabled());
        assert!(!config.stream_idle_watchdog_enabled());
        assert!(!config.any_watchdog_enabled());
    }

    #[test]
    #[serial_test::serial]
    fn watchdog_config_parses_from_env() {
        let _g1 = TestEnvGuard::set("CLAUDINE_SUBAGENT_IDLE_KILL_SECONDS", "90");
        let _g2 = TestEnvGuard::set("CLAUDINE_SUBAGENT_KILL_GRACE_SECONDS", "15");
        let _g3 = TestEnvGuard::set("CLAUDINE_SUBAGENT_WATCHDOG_INTERVAL_SECONDS", "2");
        let _g4 = TestEnvGuard::set("CLAUDINE_STREAM_IDLE_KILL_SECONDS", "600");

        let config = WatchdogConfig::from_env();
        assert_eq!(config.subagent_idle_kill_seconds, 90);
        assert_eq!(config.subagent_kill_grace_seconds, 15);
        assert_eq!(config.subagent_watchdog_interval_seconds, 2);
        assert_eq!(config.stream_idle_kill_seconds, 600);
    }

    #[test]
    #[serial_test::serial]
    fn watchdog_config_ignores_invalid_env() {
        let _g1 = TestEnvGuard::set("CLAUDINE_SUBAGENT_IDLE_KILL_SECONDS", "not_a_number");
        let _g2 = TestEnvGuard::set("CLAUDINE_STREAM_IDLE_KILL_SECONDS", "");

        let config = WatchdogConfig::from_env();
        assert_eq!(config.subagent_idle_kill_seconds, 180); // default
        assert_eq!(config.stream_idle_kill_seconds, 300);   // default
    }

    // --- evaluate_watchdog_tick tests ---

    #[test]
    fn evaluate_tick_ok_when_nothing_breached() {
        let config = WatchdogConfig::default();
        let state = Arc::new(Mutex::new(WatchdogState::default()));
        let metrics = claudine::stream::progress::new_live_metrics();
        let fired = AtomicBool::new(false);

        let result = evaluate_watchdog_tick(&config, Instant::now(), &state, &metrics, &fired);
        assert_eq!(result, WatchdogTickResult::Ok);
    }

    #[test]
    fn evaluate_tick_subagent_breach() {
        let config = WatchdogConfig {
            subagent_idle_kill_seconds: 5,
            ..Default::default()
        };
        let state = Arc::new(Mutex::new(WatchdogState::default()));
        let metrics = claudine::stream::progress::new_live_metrics();
        let fired = AtomicBool::new(false);
        let t0 = Instant::now() - Duration::from_secs(10);

        {
            let mut s = state.lock().unwrap();
            s.subagent_started("sa1".into(), Some("Researcher".into()), t0);
        }

        let result = evaluate_watchdog_tick(&config, Instant::now(), &state, &metrics, &fired);
        assert!(
            matches!(result, WatchdogTickResult::SubagentBreach(ref w) if w.reason == WatchdogTerminationReason::SubagentsUnresponsive),
            "expected SubagentBreach, got: {result:?}"
        );
        assert!(fired.load(Ordering::SeqCst), "fired flag must be set");
    }

    #[test]
    fn evaluate_tick_suppresses_stream_idle_when_subagents_active() {
        let config = WatchdogConfig {
            subagent_idle_kill_seconds: 60, // not breached
            stream_idle_kill_seconds: 5,    // would breach
            ..Default::default()
        };
        let state = Arc::new(Mutex::new(WatchdogState::default()));
        let metrics = claudine::stream::progress::new_live_metrics();
        let fired = AtomicBool::new(false);
        let t0 = Instant::now() - Duration::from_secs(10);

        {
            let mut s = state.lock().unwrap();
            s.subagent_started("sa1".into(), None, t0);
        }
        {
            let mut m = metrics.lock().unwrap();
            m.last_event_at = Some(t0);
        }

        let result = evaluate_watchdog_tick(&config, Instant::now(), &state, &metrics, &fired);
        assert_eq!(
            result,
            WatchdogTickResult::Ok,
            "stream-idle must be suppressed while subagents are active"
        );
    }

    #[test]
    fn evaluate_tick_stream_idle_breach() {
        let config = WatchdogConfig {
            subagent_idle_kill_seconds: 0, // disabled
            stream_idle_kill_seconds: 5,
            ..Default::default()
        };
        let state = Arc::new(Mutex::new(WatchdogState::default()));
        let metrics = claudine::stream::progress::new_live_metrics();
        let fired = AtomicBool::new(false);
        let t0 = Instant::now() - Duration::from_secs(10);

        {
            let mut m = metrics.lock().unwrap();
            m.last_event_at = Some(t0);
        }

        let result = evaluate_watchdog_tick(&config, Instant::now(), &state, &metrics, &fired);
        assert!(
            matches!(result, WatchdogTickResult::StreamIdleBreach(ref w) if w.reason == WatchdogTerminationReason::StreamIdleTimeout),
            "expected StreamIdleBreach, got: {result:?}"
        );
        assert!(fired.load(Ordering::SeqCst), "fired flag must be set");
    }

    #[test]
    fn evaluate_tick_no_stream_idle_when_no_activity_yet() {
        let config = WatchdogConfig {
            subagent_idle_kill_seconds: 0,
            stream_idle_kill_seconds: 5,
            ..Default::default()
        };
        let state = Arc::new(Mutex::new(WatchdogState::default()));
        let metrics = claudine::stream::progress::new_live_metrics();
        let fired = AtomicBool::new(false);

        let result = evaluate_watchdog_tick(&config, Instant::now(), &state, &metrics, &fired);
        assert_eq!(
            result,
            WatchdogTickResult::Ok,
            "must not fire when no activity has been observed"
        );
    }

    #[test]
    fn evaluate_tick_one_shot_only() {
        let config = WatchdogConfig {
            subagent_idle_kill_seconds: 5,
            ..Default::default()
        };
        let state = Arc::new(Mutex::new(WatchdogState::default()));
        let metrics = claudine::stream::progress::new_live_metrics();
        let fired = AtomicBool::new(true); // already fired
        let t0 = Instant::now() - Duration::from_secs(10);

        {
            let mut s = state.lock().unwrap();
            s.subagent_started("sa1".into(), None, t0);
        }

        let result = evaluate_watchdog_tick(&config, Instant::now(), &state, &metrics, &fired);
        assert_eq!(
            result,
            WatchdogTickResult::Ok,
            "must not re-fire after fired flag is set"
        );
    }

    #[test]
    fn evaluate_tick_disabled_subagent_watchdog_skips() {
        let config = WatchdogConfig {
            subagent_idle_kill_seconds: 0,
            stream_idle_kill_seconds: 0,
            ..Default::default()
        };
        let state = Arc::new(Mutex::new(WatchdogState::default()));
        let metrics = claudine::stream::progress::new_live_metrics();
        let fired = AtomicBool::new(false);
        let t0 = Instant::now() - Duration::from_secs(10);

        {
            let mut s = state.lock().unwrap();
            s.subagent_started("sa1".into(), None, t0);
        }
        {
            let mut m = metrics.lock().unwrap();
            m.last_event_at = Some(t0);
        }

        let result = evaluate_watchdog_tick(&config, Instant::now(), &state, &metrics, &fired);
        assert_eq!(result, WatchdogTickResult::Ok);
    }

    #[test]
    fn format_subagent_breach_message_single() {
        let stuck = vec![ActiveSubagentSnapshot {
            id: "sa1".into(),
            name: Some("Research".into()),
            started_at: Instant::now(),
            last_progress_at: Instant::now(),
            elapsed_since_start: Duration::from_secs(180),
            elapsed_since_progress: Duration::from_secs(180),
        }];
        let msg = format_subagent_breach_message(&stuck);
        assert!(msg.contains("1 subagent went silent"));
        assert!(msg.contains("sa1"));
        assert!(msg.contains("Research"));
        assert!(msg.contains("idle 3m 0s"));
    }

    #[test]
    fn format_subagent_breach_message_plural() {
        let stuck = vec![
            ActiveSubagentSnapshot {
                id: "sa1".into(),
                name: Some("A".into()),
                started_at: Instant::now(),
                last_progress_at: Instant::now(),
                elapsed_since_start: Duration::from_secs(180),
                elapsed_since_progress: Duration::from_secs(180),
            },
            ActiveSubagentSnapshot {
                id: "sa2".into(),
                name: Some("B".into()),
                started_at: Instant::now(),
                last_progress_at: Instant::now(),
                elapsed_since_start: Duration::from_secs(200),
                elapsed_since_progress: Duration::from_secs(200),
            },
        ];
        let msg = format_subagent_breach_message(&stuck);
        assert!(msg.contains("2 subagents went silent"));
        assert!(msg.contains("sa1"));
        assert!(msg.contains("sa2"));
    }

    /// RAII wrapper that restores the prior env var value on drop.
    struct TestEnvGuard {
        key: &'static str,
        prior: Option<String>,
    }
    impl TestEnvGuard {
        fn set(key: &'static str, value: &str) -> Self {
            let prior = std::env::var(key).ok();
            unsafe {
                std::env::set_var(key, value);
            }
            Self { key, prior }
        }
    }
    impl Drop for TestEnvGuard {
        fn drop(&mut self) {
            unsafe {
                match &self.prior {
                    Some(v) => std::env::set_var(self.key, v),
                    None => std::env::remove_var(self.key),
                }
            }
        }
    }
}
