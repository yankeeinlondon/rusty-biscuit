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
    /// Wall-clock time of the most recent non-empty byte chunk read from
    /// the wrapped child's stdout or stderr — refreshed **before** the bytes
    /// are handed to the semantic parser. Provider-agnostic activity signal
    /// that protects against false silence kills on providers whose stream
    /// is sparse enough that `last_event_at` can lag behind real progress
    /// (e.g. OpenCode, which emits no `tool_start` and no `task_started`).
    /// Empty / whitespace-only writes do not refresh this field.
    pub last_byte_at: Option<Instant>,
    /// Wall-clock time of the most recent stalled-stream warning emission.
    /// Used by [`should_warn_stall`] to dedupe warnings within a single
    /// stall episode — once activity resumes (`last_event_at` advances past
    /// this value), the next stall is allowed to warn again.
    pub last_stall_warning_at: Option<Instant>,
    /// Last observed provider step-completion status (e.g. OpenCode's
    /// `step_finish.reason`: `"stop"`, `"tool-calls"`, `"length"`, …).
    /// Populated from `SemanticEvent::Info` payloads carrying
    /// `extra.step_phase = "finish"`. Used by the wrapper's silence-rule
    /// guard for sparse-stream providers (notably OpenCode) so a session
    /// that has not yet crossed any step boundary cannot trip
    /// `step_timeout` during slow startup or a slow first turn.
    pub provider_status: Option<String>,
    /// Whether a provider step is currently in flight (between
    /// `step_start` and the next `step_finish`). OpenCode-specific:
    /// the silence rule is suppressed while this flag is true.
    pub step_in_flight: bool,
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

    /// Record raw byte activity from the wrapped child's stdout/stderr.
    ///
    /// Called from the wrapper's reader threads **before** the bytes reach
    /// the semantic parser so that even partially-buffered output (provider
    /// mid-flush) refreshes the silence clock. `chunk` is inspected to
    /// suppress empty / whitespace-only writes — a child that flushes blank
    /// lines must not look infinitely active.
    pub fn record_byte_activity(&mut self, chunk: &str, now: Instant) {
        if chunk.chars().any(|c| !c.is_whitespace()) {
            self.last_byte_at = Some(now);
        }
    }

    /// Most recent activity instant from either the structured-event clock
    /// (`last_event_at`) or the raw-byte clock (`last_byte_at`), whichever
    /// is newer. `None` when neither has fired yet.
    pub fn last_activity_at(&self) -> Option<Instant> {
        match (self.last_event_at, self.last_byte_at) {
            (Some(a), Some(b)) => Some(a.max(b)),
            (Some(a), None) => Some(a),
            (None, Some(b)) => Some(b),
            (None, None) => None,
        }
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
                provider_status,
                token_usage,
                cost_usd,
                ..
            } => {
                if let Some(status) = provider_status {
                    self.provider_status = Some(status.clone());
                }
                if let Some(usage) = token_usage {
                    self.token_usage = Some(usage.clone());
                }
                if let Some(cost) = cost_usd {
                    self.cost_usd = Some(*cost);
                }
            }
            SemanticEvent::Info { extra, .. }
                if extra.get("step_phase").and_then(|v| v.as_str()) == Some("start") =>
            {
                self.step_in_flight = true;
            }
            SemanticEvent::Info { extra, .. }
                if extra.get("step_phase").and_then(|v| v.as_str()) == Some("finish") =>
            {
                self.step_in_flight = false;
                let reason = extra
                    .get("reason")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "finish".to_string());
                self.provider_status = Some(reason);
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
mod tests;
