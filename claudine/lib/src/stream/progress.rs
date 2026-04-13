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

use serde_json::Value;

use super::parser::EventMeta;
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
}

/// Mutable state the announcer and the heartbeat share.
#[derive(Debug, Default, Clone)]
pub struct LiveMetricsState {
    /// Tool calls that have started but have not reported a result. Keyed by
    /// the provider-assigned tool id (or a synthetic name-based fallback).
    pub in_flight: HashMap<String, InFlightTool>,
    /// Cumulative count of tools that have completed.
    pub done_count: u32,
    /// Latest known token usage (overwritten on each delta).
    pub token_usage: Option<NormalizedTokenUsage>,
    /// Latest known cost-in-USD for the session.
    pub cost_usd: Option<f64>,
    /// Wall-clock time of the last tool start/end observed.
    pub last_tool_event_at: Option<Instant>,
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
        self.last_tool_event_at = Some(now);
    }

    pub fn record_tool_end(&mut self, id: Option<&str>, now: Instant) -> Option<InFlightTool> {
        let removed = id.and_then(|id| self.in_flight.remove(id));
        self.done_count += 1;
        self.last_tool_event_at = Some(now);
        removed
    }

    pub fn update_token_usage(&mut self, usage: NormalizedTokenUsage) {
        self.token_usage = Some(usage);
    }

    pub fn update_cost(&mut self, cost_usd: f64) {
        self.cost_usd = Some(cost_usd);
    }
}

/// Build the `Status::ToolUse` description for a tool-start event.
pub fn describe_tool_start(meta: &EventMeta) -> Option<String> {
    let name = str_from_extra(&meta.extra, &["tool_name", "name"]);
    let input_summary = value_from_extra(
        &meta.extra,
        &["tool_input", "parameters", "input", "arguments"],
    )
    .as_ref()
    .and_then(summarize_input);

    match (name, input_summary) {
        (Some(name), Some(summary)) => Some(format!("{name} \u{00b7} {summary}")),
        (Some(name), None) => Some(name),
        (None, Some(summary)) => Some(summary),
        (None, None) => None,
    }
}

/// Build the `Status::ToolUse` description for a tool-end event.
///
/// `duration` is an optional wall-clock duration for the completed tool call;
/// callers can fill it from `LiveMetrics::record_tool_end`.
pub fn describe_tool_end(meta: &EventMeta, duration: Option<Duration>) -> Option<String> {
    let name = str_from_extra(&meta.extra, &["tool_name", "name"]);
    let status = str_from_extra(&meta.extra, &["status"]);
    let error = str_from_extra(&meta.extra, &["error_message", "message"]);

    let mut parts = Vec::new();
    if let Some(n) = name {
        parts.push(n);
    }
    if let Some(d) = duration {
        parts.push(format_duration(d));
    }
    if let Some(err) = error {
        parts.push(format!("error: {}", truncate(&err, 80)));
    } else if let Some(s) = status {
        parts.push(s);
    }

    if parts.is_empty() {
        None
    } else {
        Some(parts.join(" \u{00b7} "))
    }
}

/// Build the `Status::Info` description for a heartbeat tick.
///
/// Returns `None` when the caller should suppress the tick (most recent tool
/// event happened within `quiet_window`). `elapsed` is the session duration
/// since the provider launched.
pub fn describe_heartbeat(
    state: &LiveMetricsState,
    elapsed: Duration,
    now: Instant,
    quiet_window: Duration,
) -> Option<String> {
    if let Some(last) = state.last_tool_event_at
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

fn summarize_input(value: &Value) -> Option<String> {
    // Prefer the single most useful scalar field; fall back to compact JSON.
    if let Some(s) = value.as_str() {
        return Some(truncate(s, 60));
    }
    if let Some(obj) = value.as_object() {
        for key in ["command", "path", "file_path", "pattern", "query", "url"] {
            if let Some(Value::String(s)) = obj.get(key) {
                return Some(truncate(s, 60));
            }
        }
    }
    let rendered = serde_json::to_string(value).ok()?;
    Some(truncate(&rendered, 60))
}

fn str_from_extra(extra: &HashMap<String, Value>, keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|key| extra.get(*key).and_then(|v| v.as_str().map(ToOwned::to_owned)))
}

fn value_from_extra(extra: &HashMap<String, Value>, keys: &[&str]) -> Option<Value> {
    keys.iter().find_map(|key| extra.get(*key).cloned())
}

fn truncate(s: &str, max_chars: usize) -> String {
    let count = s.chars().count();
    if count <= max_chars {
        return s.to_string();
    }
    let prefix: String = s.chars().take(max_chars.saturating_sub(1)).collect();
    format!("{prefix}\u{2026}")
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
    use serde_json::json;

    fn meta(extra: serde_json::Value) -> EventMeta {
        let mut m = EventMeta::default();
        if let Value::Object(map) = extra {
            for (k, v) in map {
                m.extra.insert(k, v);
            }
        }
        m
    }

    #[test]
    fn describes_tool_start_with_command_input() {
        let m = meta(json!({
            "tool_name": "Bash",
            "tool_input": { "command": "git status" }
        }));
        assert_eq!(describe_tool_start(&m), Some("Bash · git status".into()));
    }

    #[test]
    fn describes_tool_start_without_input() {
        let m = meta(json!({ "tool_name": "Read" }));
        assert_eq!(describe_tool_start(&m), Some("Read".into()));
    }

    #[test]
    fn describes_tool_start_truncates_long_input() {
        let long = "x".repeat(200);
        let m = meta(json!({
            "tool_name": "Grep",
            "tool_input": { "pattern": long }
        }));
        let rendered = describe_tool_start(&m).unwrap();
        assert!(rendered.contains("Grep"));
        assert!(rendered.ends_with('\u{2026}'));
    }

    #[test]
    fn describes_tool_end_with_duration_and_status() {
        let m = meta(json!({ "tool_name": "Bash", "status": "success" }));
        let desc = describe_tool_end(&m, Some(Duration::from_millis(1234))).unwrap();
        assert!(desc.contains("Bash"));
        assert!(desc.contains("1s") || desc.contains("1.2s"));
        assert!(desc.contains("success"));
    }

    #[test]
    fn describes_tool_end_with_error() {
        let m = meta(json!({
            "tool_name": "Bash",
            "error_message": "permission denied"
        }));
        let desc = describe_tool_end(&m, None).unwrap();
        assert!(desc.contains("error: permission denied"));
    }

    #[test]
    fn describes_tool_end_returns_none_when_empty() {
        let m = meta(json!({}));
        assert!(describe_tool_end(&m, None).is_none());
    }

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
        state.last_tool_event_at = Some(now);
        assert!(
            describe_heartbeat(&state, Duration::from_secs(90), now, Duration::from_secs(30))
                .is_none()
        );
    }

    #[test]
    fn heartbeat_emits_when_activity_is_stale() {
        let mut state = LiveMetricsState::default();
        let now = Instant::now();
        state.last_tool_event_at = Some(now - Duration::from_secs(60));
        state.done_count = 3;
        let desc =
            describe_heartbeat(&state, Duration::from_secs(90), now, Duration::from_secs(30))
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
        let desc =
            describe_heartbeat(&state, Duration::from_secs(120), later, Duration::from_secs(30))
                .unwrap();
        assert!(desc.contains("2 running"));
        assert!(desc.contains("Bash"));
        assert!(desc.contains("Read"));
    }

    #[test]
    fn heartbeat_includes_tokens_and_cost() {
        let mut state = LiveMetricsState::default();
        let now = Instant::now() - Duration::from_secs(60);
        state.last_tool_event_at = Some(now);
        state.update_token_usage(NormalizedTokenUsage {
            input: Some(12_000),
            output: Some(3_000),
            total: None,
            cache_read: None,
        });
        state.update_cost(0.0215);
        let later = Instant::now();
        let desc =
            describe_heartbeat(&state, Duration::from_secs(90), later, Duration::from_secs(30))
                .unwrap();
        assert!(desc.contains("12K\u{2192}3K tok"));
        assert!(desc.contains("$0.02"));
    }

    #[test]
    fn heartbeat_always_emits_when_no_events_observed() {
        let state = LiveMetricsState::default();
        let now = Instant::now();
        let desc =
            describe_heartbeat(&state, Duration::from_secs(30), now, Duration::from_secs(30))
                .unwrap();
        assert!(desc.starts_with("30s"));
    }
}
