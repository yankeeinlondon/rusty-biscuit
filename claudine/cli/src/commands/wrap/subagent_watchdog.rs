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

impl ActiveSubagentSnapshot {
    /// Convert this snapshot into the lib-side
    /// [`claudine::stream::logs::StuckSubagentInfo`] used to enrich
    /// `EarlyTermination::StepTimeout`.
    pub(crate) fn to_stuck_info(&self) -> claudine::stream::logs::StuckSubagentInfo {
        claudine::stream::logs::StuckSubagentInfo {
            id: self.id.clone(),
            name: self.name.clone(),
            elapsed_since_progress: self.elapsed_since_progress,
        }
    }
}

/// A single diagnostic line for a stuck subagent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SubagentDiagnosticLine {
    pub(crate) id: SubagentId,
    pub(crate) display_name: String,
    pub(crate) elapsed_since_start: Duration,
}

/// Reason for a watchdog-initiated termination.
///
/// The unified two-rule design has exactly two reasons. Stuck-subagent
/// detail is surfaced through [`WatchdogTermination::stuck_subagents`] for
/// the rendered error block, not through a distinct reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum WatchdogTerminationReason {
    /// Wall-clock budget (`timeout`) elapsed since the child was spawned.
    Timeout,
    /// Stream-silence budget (`step_timeout`) elapsed since the last parent
    /// stream event. Stuck subagents (if any) are carried in the
    /// [`WatchdogTermination`] for diagnostic enrichment.
    StepTimeout,
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

    /// Return a snapshot of every active subagent for diagnostic enrichment
    /// at the moment a `step_timeout` watchdog rule fires.
    ///
    /// Equivalent to [`Self::active_subagents`], but named to make the call
    /// site at the breach point self-documenting.
    pub(crate) fn outstanding_at_breach(&self, now: Instant) -> Vec<ActiveSubagentSnapshot> {
        self.active_subagents(now)
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

/// Unified timeout configuration for the watchdog ticker.
///
/// There are exactly
/// two timeout rules:
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
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            timeout: None,
            step_timeout: None,
            kill_grace: Duration::from_secs(10),
            interval: Duration::from_secs(5),
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
    pub(crate) fn resolve(
        timeout: Option<Duration>,
        step_timeout: Option<Duration>,
    ) -> Self {
        let defaults = Self::default();
        let kill_grace = parse_env_duration("CLAUDINE_KILL_GRACE").unwrap_or(defaults.kill_grace);
        let interval =
            parse_env_duration("CLAUDINE_WATCHDOG_INTERVAL").unwrap_or(defaults.interval);
        Self {
            timeout,
            step_timeout,
            kill_grace,
            interval,
        }
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

/// Result of evaluating the unified two-rule timeout watchdog on a single tick.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum WatchdogTickResult {
    /// No rules triggered; continue monitoring.
    Ok,
    /// A timeout rule breached; terminate with this request.
    Breach(WatchdogTermination),
}

/// Evaluate the unified `timeout` and `step_timeout` rules for one tick.
///
/// Rules:
///
/// 1. **Wall-clock (`timeout`).** If `config.timeout` is set and
///    `now - started_at >= timeout`, fire `Timeout`.
/// 2. **Stream-silence (`step_timeout`).** If `config.step_timeout` is set
///    AND at least one activity event has been observed
///    (`LiveMetrics.last_event_at.is_some()`) AND no tools or subagents
///    are currently in flight AND
///    `now - last_event_at >= step_timeout`, fire `StepTimeout` with
///    `outstanding = watchdog_state.active_subagents(now)` for diagnostic
///    enrichment.
///
/// The wall-clock rule is evaluated first so a deadline that elapses on
/// the same tick as a silence breach is reported as `timeout` rather than
/// `step_timeout`.
///
/// `fired` is an atomic flag that prevents double-fire across both rules;
/// once set to `true`, all subsequent evaluations return `Ok`.
pub(crate) fn evaluate_timeout_tick(
    config: &TimeoutConfig,
    now: Instant,
    started_at: Instant,
    watchdog_state: &Arc<Mutex<WatchdogState>>,
    live_metrics: &LiveMetrics,
    fired: &AtomicBool,
) -> WatchdogTickResult {
    if fired.load(Ordering::SeqCst) {
        return WatchdogTickResult::Ok;
    }

    // Rule 1: wall-clock budget.
    if let Some(budget) = config.timeout {
        let elapsed = now.saturating_duration_since(started_at);
        if elapsed >= budget {
            fired.store(true, Ordering::SeqCst);
            let message = format!(
                "wall-clock budget exceeded after {}",
                format_duration(elapsed),
            );
            return WatchdogTickResult::Breach(WatchdogTermination {
                reason: WatchdogTerminationReason::Timeout,
                message,
                stuck_subagents: Vec::new(),
            });
        }
    }

    // Rule 2: stream silence. Requires that at least one activity event
    // has been observed past initial session start, matching the existing
    // `last_event_at: Option<Instant>` first-event grace semantics. When
    // in-flight tools or subagents exist the rule is suppressed: a
    // long-running Task/subagent call produces parent-stream silence by
    // design while the child works. The wall-clock `timeout` rule serves as
    // the backstop for truly stuck tool calls.
    if let Some(budget) = config.step_timeout {
        let guarded = match live_metrics.lock() {
            Ok(g) => {
                let has_in_flight = !g.in_flight.is_empty() || !g.in_flight_subagents.is_empty();
                (g.last_event_at, has_in_flight)
            }
            Err(_) => return WatchdogTickResult::Ok,
        };
        let (last_event_at, has_in_flight) = guarded;
        if has_in_flight {
            return WatchdogTickResult::Ok;
        }
        if let Some(last) = last_event_at {
            let silence = now.saturating_duration_since(last);
            if silence >= budget {
                let outstanding = match watchdog_state.lock() {
                    Ok(g) => g.outstanding_at_breach(now),
                    Err(_) => Vec::new(),
                };
                fired.store(true, Ordering::SeqCst);
                let message = format_step_timeout_breach_message(silence, &outstanding);
                return WatchdogTickResult::Breach(WatchdogTermination {
                    reason: WatchdogTerminationReason::StepTimeout,
                    message,
                    stuck_subagents: outstanding,
                });
            }
        }
    }

    WatchdogTickResult::Ok
}

/// Format the human-readable breach message for a `step_timeout` event.
///
/// When `outstanding` is non-empty the message enumerates the subagents
/// that were still in flight at the moment the silence rule fired
/// (id, optional name, elapsed since last progress) so the operator
/// knows which workers stalled. When empty, only the silence duration
/// is reported.
fn format_step_timeout_breach_message(
    silence: Duration,
    outstanding: &[ActiveSubagentSnapshot],
) -> String {
    let silence_text = format_duration(silence);
    if outstanding.is_empty() {
        return format!("no stream activity for {silence_text}; terminating due to step_timeout");
    }

    let count = outstanding.len();
    let plural = if count == 1 { "subagent" } else { "subagents" };
    let mut lines = format!(
        "no stream activity for {silence_text}. The wrapped process was terminated. {count} {plural} were still outstanding when the timeout fired:\n"
    );
    for snap in outstanding {
        let idle = format_duration(snap.elapsed_since_progress);
        let name = snap.name.as_deref().unwrap_or("(unnamed)");
        lines.push_str(&format!("  • {} \"{name}\" (idle {idle})\n", snap.id));
    }
    lines
}

/// Format a duration in a human-readable way (e.g. "3m 0s").
pub(crate) fn format_duration(d: Duration) -> String {
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

    // --- evaluate_timeout_tick tests ---

    #[test]
    fn evaluate_timeout_tick_ok_when_no_rule_enabled() {
        let config = TimeoutConfig::default();
        let state = Arc::new(Mutex::new(WatchdogState::default()));
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
        let state = Arc::new(Mutex::new(WatchdogState::default()));
        let metrics = claudine::stream::progress::new_live_metrics();
        let fired = AtomicBool::new(false);
        let started_at = Instant::now() - Duration::from_secs(10);

        let result =
            evaluate_timeout_tick(&config, Instant::now(), started_at, &state, &metrics, &fired);
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
        let state = Arc::new(Mutex::new(WatchdogState::default()));
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

        let result =
            evaluate_timeout_tick(&config, Instant::now(), t0, &state, &metrics, &fired);
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
        let state = Arc::new(Mutex::new(WatchdogState::default()));
        let metrics = claudine::stream::progress::new_live_metrics();
        let fired = AtomicBool::new(false);
        let t0 = Instant::now() - Duration::from_secs(10);

        {
            let mut m = metrics.lock().unwrap();
            m.last_event_at = Some(t0);
        }

        let result =
            evaluate_timeout_tick(&config, Instant::now(), t0, &state, &metrics, &fired);
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
        let state = Arc::new(Mutex::new(WatchdogState::default()));
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
        let state = Arc::new(Mutex::new(WatchdogState::default()));
        let metrics = claudine::stream::progress::new_live_metrics();
        let fired = AtomicBool::new(false);
        let t0 = Instant::now() - Duration::from_secs(10);

        {
            let mut m = metrics.lock().unwrap();
            m.last_event_at = Some(t0);
        }

        let result =
            evaluate_timeout_tick(&config, Instant::now(), t0, &state, &metrics, &fired);
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
        let state = Arc::new(Mutex::new(WatchdogState::default()));
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
        let state = Arc::new(Mutex::new(WatchdogState::default()));
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
                },
            );
        }

        let result =
            evaluate_timeout_tick(&config, Instant::now(), t0, &state, &metrics, &fired);
        assert_eq!(result, WatchdogTickResult::Ok);
        assert!(!fired.load(Ordering::SeqCst));
    }

    #[test]
    fn evaluate_timeout_tick_silence_suppressed_by_in_flight_subagent() {
        let config = TimeoutConfig {
            timeout: None,
            step_timeout: Some(Duration::from_secs(5)),
            ..Default::default()
        };
        let state = Arc::new(Mutex::new(WatchdogState::default()));
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
                },
            );
        }

        let result =
            evaluate_timeout_tick(&config, Instant::now(), t0, &state, &metrics, &fired);
        assert_eq!(result, WatchdogTickResult::Ok);
        assert!(!fired.load(Ordering::SeqCst));
    }

    #[test]
    fn evaluate_timeout_tick_silence_fires_after_in_flight_cleared() {
        let config = TimeoutConfig {
            timeout: None,
            step_timeout: Some(Duration::from_secs(5)),
            ..Default::default()
        };
        let state = Arc::new(Mutex::new(WatchdogState::default()));
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
                },
            );
        }

        let result =
            evaluate_timeout_tick(&config, Instant::now(), t0, &state, &metrics, &fired);
        assert_eq!(result, WatchdogTickResult::Ok, "must not fire while tool in-flight");

        {
            let mut m = metrics.lock().unwrap();
            m.in_flight.clear();
        }

        let result =
            evaluate_timeout_tick(&config, Instant::now(), t0, &state, &metrics, &fired);
        match result {
            WatchdogTickResult::Breach(ref w) => {
                assert_eq!(w.reason, WatchdogTerminationReason::StepTimeout);
            }
            other => panic!("expected StepTimeout breach after in-flight cleared, got: {other:?}"),
        }
        assert!(fired.load(Ordering::SeqCst));
    }

    #[test]
    fn format_step_timeout_breach_message_no_outstanding() {
        let msg = format_step_timeout_breach_message(Duration::from_secs(180), &[]);
        assert!(msg.contains("3m 0s"));
        assert!(msg.contains("step_timeout"));
        assert!(!msg.contains("subagent"));
    }

    #[test]
    fn format_step_timeout_breach_message_lists_outstanding() {
        let snap = ActiveSubagentSnapshot {
            id: "ses_a".into(),
            name: Some("Commit work".into()),
            started_at: Instant::now(),
            last_progress_at: Instant::now(),
            elapsed_since_start: Duration::from_secs(900),
            elapsed_since_progress: Duration::from_secs(900),
        };
        let msg =
            format_step_timeout_breach_message(Duration::from_secs(1800), std::slice::from_ref(&snap));
        assert!(msg.contains("30m 0s"));
        assert!(msg.contains("1 subagent"));
        assert!(msg.contains("ses_a"));
        assert!(msg.contains("Commit work"));
        assert!(msg.contains("idle 15m 0s"));
    }

    // --- outstanding_at_breach tests ---

    #[test]
    fn outstanding_at_breach_returns_all_active_subagents() {
        let mut state = WatchdogState::default();
        let t0 = now();
        state.subagent_started("a".into(), Some("Alpha".into()), t0);
        state.subagent_started("b".into(), None, t0);

        let t5 = t0 + Duration::from_secs(5);
        let outstanding = state.outstanding_at_breach(t5);
        assert_eq!(outstanding.len(), 2);
        let alpha = outstanding.iter().find(|s| s.id == "a").unwrap();
        assert_eq!(alpha.name.as_deref(), Some("Alpha"));
        assert_eq!(alpha.elapsed_since_start, Duration::from_secs(5));
    }

    #[test]
    fn outstanding_at_breach_skips_stopped_subagents() {
        let mut state = WatchdogState::default();
        let t0 = now();
        state.subagent_started("a".into(), None, t0);
        state.subagent_started("b".into(), None, t0);
        state.subagent_stopped(&"a".into(), t0);

        let outstanding = state.outstanding_at_breach(t0);
        assert_eq!(outstanding.len(), 1);
        assert_eq!(outstanding[0].id, "b");
    }

    #[test]
    fn outstanding_at_breach_empty_when_no_active() {
        let state = WatchdogState::default();
        let outstanding = state.outstanding_at_breach(now());
        assert!(outstanding.is_empty());
    }

    // --- TimeoutConfig tests ---

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

    #[test]
    #[serial_test::serial]
    fn timeout_config_resolve_honours_pre_resolved_inputs() {
        // Ensure env knobs are absent so we observe the inputs cleanly.
        let _g1 = TestEnvGuard::clear("CLAUDINE_KILL_GRACE");
        let _g2 = TestEnvGuard::clear("CLAUDINE_WATCHDOG_INTERVAL");

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

    #[test]
    #[serial_test::serial]
    fn timeout_config_resolve_does_not_consult_timeout_env_vars() {
        // Composition layer owns timeout/step_timeout precedence; resolve
        // must NOT read these env vars itself.
        let _g1 = TestEnvGuard::set("CLAUDINE_TIMEOUT", "1h");
        let _g2 = TestEnvGuard::set("CLAUDINE_STEP_TIMEOUT", "5m");
        let _g3 = TestEnvGuard::clear("CLAUDINE_KILL_GRACE");
        let _g4 = TestEnvGuard::clear("CLAUDINE_WATCHDOG_INTERVAL");

        let config = TimeoutConfig::resolve(None, None);
        assert_eq!(config.timeout, None, "resolve must not read CLAUDINE_TIMEOUT");
        assert_eq!(
            config.step_timeout, None,
            "resolve must not read CLAUDINE_STEP_TIMEOUT"
        );
    }

    #[test]
    #[serial_test::serial]
    fn timeout_config_resolve_parses_kill_grace_and_interval_env_vars() {
        let _g1 = TestEnvGuard::set("CLAUDINE_KILL_GRACE", "30s");
        let _g2 = TestEnvGuard::set("CLAUDINE_WATCHDOG_INTERVAL", "2s");

        let config = TimeoutConfig::resolve(None, None);
        assert_eq!(config.kill_grace, Duration::from_secs(30));
        assert_eq!(config.interval, Duration::from_secs(2));
    }

    #[test]
    #[serial_test::serial]
    fn timeout_config_resolve_falls_back_when_env_invalid() {
        let _g1 = TestEnvGuard::set("CLAUDINE_KILL_GRACE", "garbage");
        let _g2 = TestEnvGuard::set("CLAUDINE_WATCHDOG_INTERVAL", "");

        let config = TimeoutConfig::resolve(None, None);
        assert_eq!(config.kill_grace, Duration::from_secs(10));
        assert_eq!(config.interval, Duration::from_secs(5));
    }

    #[test]
    #[serial_test::serial]
    fn timeout_config_resolve_accepts_minute_and_hour_units() {
        let _g1 = TestEnvGuard::set("CLAUDINE_KILL_GRACE", "1m");
        let _g2 = TestEnvGuard::set("CLAUDINE_WATCHDOG_INTERVAL", "1h");

        let config = TimeoutConfig::resolve(None, None);
        assert_eq!(config.kill_grace, Duration::from_secs(60));
        assert_eq!(config.interval, Duration::from_secs(3600));
    }

    #[test]
    #[serial_test::serial]
    fn timeout_config_resolve_cli_wins_over_frontmatter_env_and_default() {
        let _g1 = TestEnvGuard::clear("CLAUDINE_TIMEOUT");
        let _g2 = TestEnvGuard::clear("CLAUDINE_STEP_TIMEOUT");
        let _g3 = TestEnvGuard::clear("CLAUDINE_KILL_GRACE");
        let _g4 = TestEnvGuard::clear("CLAUDINE_WATCHDOG_INTERVAL");

        // Simulating the composition layer resolving CLI > frontmatter > env
        let resolved_timeout = Some(Duration::from_secs(7200)); // from CLI
        let resolved_step_timeout = Some(Duration::from_secs(1800)); // from CLI
        let config = TimeoutConfig::resolve(resolved_timeout, resolved_step_timeout);
        assert_eq!(config.timeout, Some(Duration::from_secs(7200)));
        assert_eq!(config.step_timeout, Some(Duration::from_secs(1800)));
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

        fn clear(key: &'static str) -> Self {
            let prior = std::env::var(key).ok();
            unsafe {
                std::env::remove_var(key);
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
