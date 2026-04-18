//! Live progress reporting shared between stream parsers and the heartbeat
//! thread.
//!
//! Parsers push tool lifecycle and token/cost deltas into [`LiveMetrics`];
//! the CLI heartbeat reads the snapshot to render a `Status::Info` line when
//! the stream goes quiet. Per-tool announcements (`Status::ToolUse`) are
//! rendered through the same formatters so every provider surfaces the same
//! visual vocabulary regardless of which fields its stream exposes.
//!
//! The announcer only *formats* descriptions. Callers are responsible for
//! wrapping the descriptions in `Status`, rendering against a `Terminal`, and
//! writing to stderr so unit tests can assert on plain text.
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use super::semantic::SemanticEvent;
use super::token_usage::NormalizedTokenUsage;

/// Named constants for the heartbeat-emission policy.
///
/// - `interval`: granularity at which the heartbeat thread wakes up.
/// - `silence_window`: time of stream inactivity required before a heartbeat
///   tick is allowed to emit.
/// - `force_window`: hard cadence that overrides `silence_window` — once this
///   much time has passed since the last heartbeat, the next tick fires even
///   if the stream is busy, so long-running subagents still surface progress.
///
/// Moving the timing into a named struct keeps the exec-loop policy explicit
/// and testable, and establishes a single place to tune behavior across every
/// provider.
#[derive(Debug, Clone, Copy)]
pub struct HeartbeatPolicy {
    pub interval: Duration,
    pub silence_window: Duration,
    pub force_window: Duration,
}

impl HeartbeatPolicy {
    pub const DEFAULT_INTERVAL: Duration = Duration::from_secs(30);
    pub const DEFAULT_SILENCE_WINDOW: Duration = Duration::from_secs(30);
    pub const DEFAULT_FORCE_WINDOW: Duration = Duration::from_secs(120);
}

impl Default for HeartbeatPolicy {
    fn default() -> Self {
        Self {
            interval: Self::DEFAULT_INTERVAL,
            silence_window: Self::DEFAULT_SILENCE_WINDOW,
            force_window: Self::DEFAULT_FORCE_WINDOW,
        }
    }
}

/// Shared, thread-safe handle to the live metrics snapshot.
pub type LiveMetrics = Arc<Mutex<LiveMetricsState>>;

/// Create a fresh metrics handle.
pub fn new_live_metrics() -> LiveMetrics {
    Arc::new(Mutex::new(LiveMetricsState::default()))
}

/// A tool invocation the parser has started but not yet seen a result for.
#[derive(Debug, Clone)]
pub struct InFlightTool {
    pub name: Option<String>,
    pub started_at: Instant,
}

/// A subagent invocation the parser has started but not yet seen a stop for.
#[derive(Debug, Clone)]
pub struct InFlightSubagent {
    pub name: Option<String>,
    pub started_at: Instant,
}

/// Mutable state the announcer and the heartbeat share.
#[derive(Debug, Default, Clone)]
pub struct LiveMetricsState {
    /// Tool calls that have started but have not reported a result. Keyed by
    /// the provider-assigned tool id (or a synthetic name-based fallback).
    pub in_flight: HashMap<String, InFlightTool>,
    /// Subagents that have started but have not reported a stop. Keyed by
    /// the provider-assigned subagent id (or name-based fallback).
    pub in_flight_subagents: HashMap<String, InFlightSubagent>,
    /// Cumulative count of tools that have completed.
    pub done_count: u32,
    /// Cumulative count of subagents that have stopped.
    pub subagent_done_count: u32,
    /// Latest known token usage (overwritten on each delta).
    pub token_usage: Option<NormalizedTokenUsage>,
    /// Latest known cost-in-USD for the session.
    pub cost_usd: Option<f64>,
    /// Wall-clock time of the most recent observed event of any kind (tool
    /// start, tool end, or assistant text delta). The heartbeat suppresses
    /// ticks when this is recent so busy streams stay quiet; a stale value
    /// means the provider is silent and the user deserves a status update.
    pub last_event_at: Option<Instant>,
    /// Wall-clock time of the most recent heartbeat emission. Ensures the
    /// heartbeat surfaces at a hard cadence even during sustained activity —
    /// otherwise a flood of tool events can hide a long-running subagent
    /// indefinitely.
    pub last_heartbeat_at: Option<Instant>,
    /// Wall-clock time of the most recent stalled-stream warning emission.
    /// Used by [`should_warn_stall`] to dedupe warnings within a single
    /// stall episode — once activity resumes (`last_event_at` advances past
    /// this value), the next stall is allowed to warn again.
    pub last_stall_warning_at: Option<Instant>,
}

impl LiveMetricsState {
    pub fn record_tool_start(&mut self, id: String, name: Option<String>, now: Instant) {
        self.in_flight.insert(
            id,
            InFlightTool {
                name,
                started_at: now,
            },
        );
        self.last_event_at = Some(now);
    }

    pub fn record_tool_end(&mut self, id: Option<&str>, now: Instant) -> Option<InFlightTool> {
        let removed = id.and_then(|id| self.in_flight.remove(id));
        self.done_count += 1;
        self.last_event_at = Some(now);
        removed
    }

    /// Record assistant-text or other non-tool activity so the heartbeat
    /// suppresses while the provider is actively producing output.
    pub fn record_activity(&mut self, now: Instant) {
        self.last_event_at = Some(now);
    }

    pub fn update_token_usage(&mut self, usage: NormalizedTokenUsage) {
        self.token_usage = Some(usage);
    }

    pub fn update_cost(&mut self, cost_usd: f64) {
        self.cost_usd = Some(cost_usd);
    }

    pub fn record_subagent_start(&mut self, id: String, name: Option<String>, now: Instant) {
        self.in_flight_subagents.insert(
            id,
            InFlightSubagent {
                name,
                started_at: now,
            },
        );
        self.last_event_at = Some(now);
    }

    pub fn record_subagent_stop(
        &mut self,
        id: Option<&str>,
        now: Instant,
    ) -> Option<InFlightSubagent> {
        let removed = id.and_then(|id| self.in_flight_subagents.remove(id));
        self.subagent_done_count += 1;
        self.last_event_at = Some(now);
        removed
    }

    /// Fold a [`SemanticEvent`] into live-metrics state.
    ///
    /// This is the forward-looking entry point used by
    /// `LiveSemanticSink`: it replaces the scattered `record_tool_start`
    /// / `record_tool_end` / `record_activity` / `update_token_usage`
    /// calls. Every event that passes [`SemanticEvent::is_activity`]
    /// refreshes `last_event_at`, keeping the heartbeat silence-suppression
    /// honest.
    pub fn observe_event(&mut self, event: &SemanticEvent, now: Instant) {
        if event.is_activity() {
            self.last_event_at = Some(now);
        }
        match event {
            SemanticEvent::ToolCall { id, name, .. } => {
                let key = id
                    .clone()
                    .or_else(|| name.clone())
                    .unwrap_or_else(|| format!("tool-{}-{:?}", self.done_count, now));
                self.in_flight.insert(
                    key,
                    InFlightTool {
                        name: name.clone(),
                        started_at: now,
                    },
                );
            }
            SemanticEvent::ToolResult { id, .. } => {
                if let Some(id) = id {
                    self.in_flight.remove(id.as_str());
                }
                self.done_count += 1;
            }
            SemanticEvent::SubagentStart { id, name, .. } => {
                let key = id
                    .clone()
                    .or_else(|| name.clone())
                    .unwrap_or_else(|| format!("subagent-{}-{:?}", self.subagent_done_count, now));
                self.in_flight_subagents.insert(
                    key,
                    InFlightSubagent {
                        name: name.clone(),
                        started_at: now,
                    },
                );
            }
            SemanticEvent::SubagentStop { id, .. } => {
                if let Some(id) = id {
                    self.in_flight_subagents.remove(id.as_str());
                }
                self.subagent_done_count += 1;
            }
            SemanticEvent::TurnComplete {
                token_usage,
                cost_usd,
                ..
            } => {
                if let Some(usage) = token_usage {
                    self.token_usage = Some(usage.clone());
                }
                if let Some(cost) = cost_usd {
                    self.cost_usd = Some(*cost);
                }
            }
            _ => {}
        }
    }
}

/// Build the `Status::Info` description for a heartbeat tick.
///
/// Returns `None` when the caller should suppress the tick because the stream
/// has been actively producing output within `quiet_window`. The suppression
/// is overridden by `force_window`: once that much time has passed since the
/// last heartbeat, a tick is emitted regardless of ongoing activity so busy
/// streams still surface progress to the user.
///
/// `elapsed` is the session duration since the provider launched.
pub fn describe_heartbeat(
    state: &LiveMetricsState,
    elapsed: Duration,
    now: Instant,
    quiet_window: Duration,
    force_window: Duration,
) -> Option<String> {
    let should_force = state
        .last_heartbeat_at
        .map(|last| now.saturating_duration_since(last) >= force_window)
        .unwrap_or(false);

    if !should_force
        && let Some(last) = state.last_event_at
        && now.saturating_duration_since(last) < quiet_window
    {
        return None;
    }

    let mut parts = Vec::new();
    parts.push(format_duration(elapsed));

    if !state.in_flight.is_empty() {
        let mut names: Vec<&str> = state
            .in_flight
            .values()
            .filter_map(|t| t.name.as_deref())
            .collect();
        names.sort_unstable();
        let running = state.in_flight.len();
        let label = if names.is_empty() {
            format!("{running} running")
        } else {
            format!("{running} running ({})", names.join(", "))
        };
        parts.push(label);
    }

    if !state.in_flight_subagents.is_empty() {
        let mut names: Vec<&str> = state
            .in_flight_subagents
            .values()
            .filter_map(|s| s.name.as_deref())
            .collect();
        names.sort_unstable();
        let count = state.in_flight_subagents.len();
        let label = if names.is_empty() {
            format!("{count} subagent(s)")
        } else {
            format!("{count} subagent(s) ({})", names.join(", "))
        };
        parts.push(label);
    }

    if state.done_count > 0 {
        parts.push(format!("{} done", state.done_count));
    }

    if let Some(usage) = &state.token_usage {
        if let (Some(i), Some(o)) = (usage.input, usage.output) {
            parts.push(format!(
                "{}\u{2192}{} tok",
                format_number(i),
                format_number(o)
            ));
        } else if let Some(i) = usage.input {
            parts.push(format!("{} in", format_number(i)));
        } else if let Some(o) = usage.output {
            parts.push(format!("{} out", format_number(o)));
        }
    }

    if let Some(cost) = state.cost_usd {
        parts.push(format_cost(cost));
    }

    Some(parts.join(" \u{00b7} "))
}

/// Decide whether the heartbeat should emit a stalled-stream warning.
///
/// Returns `true` when:
/// - some activity has been observed (`last_event_at.is_some()`),
/// - the elapsed time since that last activity meets or exceeds
///   `stall_threshold`, AND
/// - no warning has been emitted yet during this stall episode (i.e.
///   `last_stall_warning_at` is `None` or strictly older than
///   `last_event_at`).
///
/// Callers are expected to set `last_stall_warning_at = Some(now)` after a
/// successful warning emission so the same stall does not re-fire on every
/// subsequent heartbeat tick. Once activity resumes, `last_event_at`
/// naturally advances past the stored warning timestamp and the next stall
/// is again eligible to warn.
pub fn should_warn_stall(
    state: &LiveMetricsState,
    now: Instant,
    stall_threshold: Duration,
) -> bool {
    let Some(last_event) = state.last_event_at else {
        return false;
    };
    if now.saturating_duration_since(last_event) < stall_threshold {
        return false;
    }
    match state.last_stall_warning_at {
        Some(warned_at) => warned_at < last_event,
        None => true,
    }
}

fn format_duration(d: Duration) -> String {
    let secs = d.as_secs_f64();
    if secs < 10.0 {
        format!("{secs:.1}s")
    } else {
        format!("{:.0}s", secs)
    }
}

fn format_number(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.0}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

fn format_cost(cost: f64) -> String {
    if cost < 0.01 {
        format!("${cost:.4}")
    } else {
        format!("${cost:.2}")
    }
}

#[cfg(test)]
mod tests {
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
    fn heartbeat_suppresses_when_activity_is_recent() {
        let mut state = LiveMetricsState::default();
        let now = Instant::now();
        state.last_event_at = Some(now);
        assert!(
            describe_heartbeat(
                &state,
                Duration::from_secs(90),
                now,
                Duration::from_secs(30),
                Duration::from_secs(120),
            )
            .is_none()
        );
    }

    #[test]
    fn heartbeat_emits_when_activity_is_stale() {
        let mut state = LiveMetricsState::default();
        let now = Instant::now();
        state.last_event_at = Some(now - Duration::from_secs(60));
        state.done_count = 3;
        let desc = describe_heartbeat(
            &state,
            Duration::from_secs(90),
            now,
            Duration::from_secs(30),
            Duration::from_secs(120),
        )
        .unwrap();
        assert!(desc.contains("90s"));
        assert!(desc.contains("3 done"));
    }

    #[test]
    fn heartbeat_lists_running_tool_names() {
        let mut state = LiveMetricsState::default();
        let now = Instant::now() - Duration::from_secs(60);
        state.record_tool_start("a".into(), Some("Bash".into()), now);
        state.record_tool_start("b".into(), Some("Read".into()), now);
        let later = Instant::now();
        let desc = describe_heartbeat(
            &state,
            Duration::from_secs(120),
            later,
            Duration::from_secs(30),
            Duration::from_secs(120),
        )
        .unwrap();
        assert!(desc.contains("2 running"));
        assert!(desc.contains("Bash"));
        assert!(desc.contains("Read"));
    }

    #[test]
    fn heartbeat_includes_tokens_and_cost() {
        let mut state = LiveMetricsState::default();
        let now = Instant::now() - Duration::from_secs(60);
        state.last_event_at = Some(now);
        state.update_token_usage(NormalizedTokenUsage {
            input: Some(12_000),
            output: Some(3_000),
            total: None,
            cache_read: None,
        });
        state.update_cost(0.0215);
        let later = Instant::now();
        let desc = describe_heartbeat(
            &state,
            Duration::from_secs(90),
            later,
            Duration::from_secs(30),
            Duration::from_secs(120),
        )
        .unwrap();
        assert!(desc.contains("12K\u{2192}3K tok"));
        assert!(desc.contains("$0.02"));
    }

    #[test]
    fn heartbeat_always_emits_when_no_events_observed() {
        let state = LiveMetricsState::default();
        let now = Instant::now();
        let desc = describe_heartbeat(
            &state,
            Duration::from_secs(30),
            now,
            Duration::from_secs(30),
            Duration::from_secs(120),
        )
        .unwrap();
        assert!(desc.starts_with("30s"));
    }

    #[test]
    fn heartbeat_forces_emission_when_last_heartbeat_is_stale() {
        let mut state = LiveMetricsState::default();
        let now = Instant::now();
        // Busy stream: an event happened just now, so the quiet window would
        // normally suppress. But the last heartbeat was 3 minutes ago and the
        // force window is 2 minutes — the tick must fire anyway.
        state.last_event_at = Some(now);
        state.last_heartbeat_at = Some(now - Duration::from_secs(180));
        let desc = describe_heartbeat(
            &state,
            Duration::from_secs(200),
            now,
            Duration::from_secs(30),
            Duration::from_secs(120),
        )
        .unwrap();
        assert!(desc.contains("200s"));
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
    fn record_activity_updates_last_event_at() {
        let mut state = LiveMetricsState::default();
        let now = Instant::now();
        state.record_activity(now);
        assert_eq!(state.last_event_at, Some(now));
    }

    mod observe_event_tests {
        use super::*;
        use crate::stream::semantic::SemanticEvent;

        #[test]
        fn heartbeat_policy_defaults_match_contract() {
            let p = HeartbeatPolicy::default();
            assert_eq!(p.interval, Duration::from_secs(30));
            assert_eq!(p.silence_window, Duration::from_secs(30));
            assert_eq!(p.force_window, Duration::from_secs(120));
        }

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
        fn turn_complete_updates_token_usage_and_cost() {
            let mut state = LiveMetricsState::default();
            let now = Instant::now();
            state.observe_event(
                &SemanticEvent::TurnComplete {
                    provider_status: None,
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
        fn heartbeat_lists_in_flight_subagents() {
            let mut state = LiveMetricsState::default();
            let now = Instant::now() - Duration::from_secs(60);
            state.observe_event(
                &SemanticEvent::SubagentStart {
                    name: Some("researcher".into()),
                    id: Some("sa1".into()),
                    extra: serde_json::json!({}),
                },
                now,
            );
            let later = Instant::now();
            let desc = describe_heartbeat(
                &state,
                Duration::from_secs(120),
                later,
                Duration::from_secs(30),
                Duration::from_secs(120),
            )
            .unwrap();
            assert!(desc.contains("1 subagent"));
            assert!(desc.contains("researcher"));
        }
    }
}
