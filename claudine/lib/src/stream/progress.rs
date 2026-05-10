//! Live progress reporting collected while stream parsers emit semantic
//! events.
//!
//! Parsers push tool lifecycle and token/cost deltas into [`LiveMetrics`];
//! the CLI's prompt-scoped timing monitor reads the silence clock (via
//! [`should_warn_stall`]) to decide whether to emit a `step_timeout_warn`
//! line. Per-tool announcements (`Status::ToolUse`) are rendered through
//! the same formatters so every provider surfaces the same visual
//! vocabulary regardless of which fields its stream exposes.
//!
//! Callers wrap the descriptions in `Status`, render against a `Terminal`,
//! and write to stderr — this module only tracks state.
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use super::semantic::SemanticEvent;
use super::token_usage::NormalizedTokenUsage;

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
    /// Wall-clock time of the most recent progress event for this tool.
    pub last_progress_at: Instant,
}

/// A subagent invocation the parser has started but not yet seen a stop for.
#[derive(Debug, Clone)]
pub struct InFlightSubagent {
    pub name: Option<String>,
    pub started_at: Instant,
    /// Wall-clock time of the most recent progress event for this subagent.
    pub last_progress_at: Instant,
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
    /// start, tool end, or assistant text delta). The step-timeout warning
    /// compares against this to measure silence; a stale value means the
    /// provider has gone quiet.
    pub last_event_at: Option<Instant>,
    /// Wall-clock time of the most recent stalled-stream warning emission.
    /// Used by [`should_warn_stall`] to dedupe warnings within a single
    /// stall episode — once activity resumes (`last_event_at` advances past
    /// this value), the next stall is allowed to warn again.
    pub last_stall_warning_at: Option<Instant>,
}

impl LiveMetricsState {
    fn remove_in_flight_tool(&mut self, id: Option<&str>, name: Option<&str>) {
        if let Some(id) = id
            && self.in_flight.remove(id).is_some()
        {
            return;
        }
        if let Some(name) = name {
            self.in_flight.remove(name);
        }
    }

    fn remove_in_flight_subagent(&mut self, id: Option<&str>, name: Option<&str>) {
        if let Some(id) = id
            && self.in_flight_subagents.remove(id).is_some()
        {
            return;
        }
        if let Some(name) = name {
            self.in_flight_subagents.remove(name);
        }
    }

    pub fn record_tool_start(&mut self, id: String, name: Option<String>, now: Instant) {
        self.in_flight.insert(
            id,
            InFlightTool {
                name,
                started_at: now,
                last_progress_at: now,
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
        for tool in self.in_flight.values_mut() {
            tool.last_progress_at = now;
        }
        for subagent in self.in_flight_subagents.values_mut() {
            subagent.last_progress_at = now;
        }
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
                last_progress_at: now,
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
                        last_progress_at: now,
                    },
                );
            }
            SemanticEvent::ToolResult { id, .. } => {
                let name = match event {
                    SemanticEvent::ToolResult { name, .. } => name.as_deref(),
                    _ => None,
                };
                self.remove_in_flight_tool(id.as_deref(), name);
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
                        last_progress_at: now,
                    },
                );
            }
            SemanticEvent::SubagentStop { id, .. } => {
                let name = match event {
                    SemanticEvent::SubagentStop { name, .. } => name.as_deref(),
                    _ => None,
                };
                self.remove_in_flight_subagent(id.as_deref(), name);
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

    /// Returns all in-flight tools whose `last_progress_at` is at least
    /// `threshold` older than `now`.
    pub fn stuck_tools(&self, now: Instant, threshold: Duration) -> Vec<&InFlightTool> {
        self.in_flight
            .values()
            .filter(|tool| now.saturating_duration_since(tool.last_progress_at) >= threshold)
            .collect()
    }

    /// Returns all in-flight subagents whose `last_progress_at` is at least
    /// `threshold` older than `now`.
    pub fn stuck_subagents(&self, now: Instant, threshold: Duration) -> Vec<&InFlightSubagent> {
        self.in_flight_subagents
            .values()
            .filter(|subagent| {
                now.saturating_duration_since(subagent.last_progress_at) >= threshold
            })
            .collect()
    }
}

/// Decide whether the step-silence warning should emit.
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
/// subsequent timing tick. Once activity resumes, `last_event_at`
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
    }
}
