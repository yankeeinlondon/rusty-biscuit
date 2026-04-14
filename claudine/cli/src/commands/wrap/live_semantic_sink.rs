// This module is the forward-looking CLI sink. Its call-site wiring (in
// `exec.rs`, `composition.rs`, `sequence.rs`) is deferred to the Phase 3.3
// migration step; until then the binary doesn't construct it, so the
// per-item dead-code lints would be noisy. Tests still exercise every
// public surface.
#![allow(dead_code)]

//! [`LiveSemanticSink`] — a CLI-side sink that consumes [`SemanticEvent`]s
//! from a [`SemanticStreamParser`](claudine::stream::parser::SemanticStreamParser)
//! and fans them out to:
//!
//! 1. Status rendering on STDERR (tool / subagent / info / warning / error /
//!    provider-extension).
//! 2. [`LiveMetrics::observe_event`] for heartbeat tracking.
//! 3. Claudine's higher-level [`AgenticEvent`] dispatcher for user-configured
//!    hooks.
//! 4. The `StructuredSummaryDetails` tool-name rollup.
//!
//! This sink is the forward-looking replacement for
//! [`super::LiveStreamSink`]. Its behavior on the wire mirrors the legacy
//! sink for tool-call / tool-result / session-start, but it adds:
//!
//! - Arrow prefixes: `→ <tool>` for start, `← <tool>` for completion; same
//!   for subagents using [`StatusState::Subagent`].
//! - A [`SemanticEvent::ProviderExtension`] fallback formatter that prints
//!   `{provider}/{kind} · {summary}`.
//! - Reasoning / output text passthrough for the forthcoming live renderer
//!   migration (the emissions are exposed as rendered lines via a simple
//!   callback hook so tests can verify ordering without pulling in the full
//!   [`StreamOutput`](super::stream_io::StreamOutput) surface).
//!
//! Wiring this sink into `exec.rs`, `composition.rs`, and `sequence.rs` is
//! intentionally deferred to the Phase 3.3–3.6 migration step. The module is
//! landable stand-alone and covered by its own unit tests.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use biscuit_terminal::components::renderable::Renderable;
use biscuit_terminal::components::status::{Status, StatusState};
use biscuit_terminal::terminal::Terminal;
use claudine::events::{
    AgenticEvent, EnvironmentContext, EventMeta as DispatchEventMeta, Provider,
};
use claudine::stream::progress::{self, LiveMetrics};
use claudine::stream::semantic::{SemanticEvent, SemanticEventSink};
use claudine::stream::stderr::Verbosity;
use serde_json::Value;

use super::StructuredSummaryDetails;

/// Borrow-friendly terminal used for status rendering. Mirrors the helper in
/// `wrap/mod.rs` so both sinks render against the same capabilities.
fn wrap_terminal() -> Terminal {
    Terminal::builder().build()
}

/// Function type for dispatching an agentic-event to claudine's hook pipeline.
pub(crate) type SemanticDispatchFn =
    Box<dyn Fn(AgenticEvent, DispatchEventMeta) + Send + Sync + 'static>;

/// Function type for emitting a rendered STDERR line. Pluggable so tests can
/// capture output without going through the real `StreamOutput`.
pub(crate) type StderrEmitFn = Box<dyn Fn(&str) + Send + Sync + 'static>;

pub(crate) struct LiveSemanticSink {
    provider: Provider,
    env: EnvironmentContext,
    cwd: PathBuf,
    verbosity: Verbosity,
    session_id: Option<String>,
    model: Option<String>,
    summary_details: Arc<Mutex<StructuredSummaryDetails>>,
    context_extra: HashMap<String, Value>,
    dispatch: SemanticDispatchFn,
    emit_stderr: StderrEmitFn,
    live_metrics: LiveMetrics,
}

impl LiveSemanticSink {
    pub(crate) fn new(
        provider: Provider,
        env: EnvironmentContext,
        cwd: &Path,
        verbosity: Verbosity,
        summary_details: Arc<Mutex<StructuredSummaryDetails>>,
        dispatch: SemanticDispatchFn,
        emit_stderr: StderrEmitFn,
    ) -> Self {
        Self {
            provider,
            env,
            cwd: cwd.to_path_buf(),
            verbosity,
            session_id: None,
            model: None,
            summary_details,
            context_extra: HashMap::new(),
            dispatch,
            emit_stderr,
            live_metrics: progress::new_live_metrics(),
        }
    }

    pub(crate) fn with_context_extra(
        mut self,
        context_extra: HashMap<String, Value>,
    ) -> Self {
        self.context_extra = context_extra;
        self
    }

    pub(crate) fn live_metrics(&self) -> LiveMetrics {
        self.live_metrics.clone()
    }

    fn should_render(&self) -> bool {
        self.verbosity != Verbosity::Silent
    }

    fn emit_line(&self, line: &str) {
        (self.emit_stderr)(line);
    }

    fn render_status(&self, state: StatusState, description: String) {
        let rendered = Status::new(description)
            .state(state)
            .render(&wrap_terminal());
        self.emit_line(&rendered);
    }

    fn tool_call_description(name: &Option<String>, input: &Option<Value>) -> String {
        let name_part = name.as_deref().unwrap_or("(tool)");
        let summary = input.as_ref().and_then(summarize_input);
        match summary {
            Some(s) => format!("\u{2192} {name_part} \u{00b7} {s}"),
            None => format!("\u{2192} {name_part}"),
        }
    }

    fn tool_result_description(
        name: &Option<String>,
        status: &Option<String>,
        exit_code: Option<i32>,
    ) -> String {
        let name_part = name.as_deref().unwrap_or("(tool)");
        let mut parts = vec![format!("\u{2190} {name_part}")];
        if let Some(code) = exit_code {
            parts.push(format!("exit {code}"));
        } else if let Some(s) = status {
            parts.push(s.clone());
        }
        parts.join(" \u{00b7} ")
    }

    fn subagent_description(arrow: char, name: &Option<String>) -> String {
        let name_part = name.as_deref().unwrap_or("(subagent)");
        format!("{arrow} {name_part}")
    }

    fn provider_extension_description(provider: Provider, kind: &str, payload: &Value) -> String {
        let summary = summarize_provider_payload(payload);
        match summary {
            Some(s) => format!("{}/{kind} \u{00b7} {s}", provider_short(provider)),
            None => format!("{}/{kind}", provider_short(provider)),
        }
    }

    fn update_session_state(&mut self, session_id: &Option<String>, model: &Option<String>) {
        if let Some(sid) = session_id {
            self.session_id = Some(sid.clone());
        }
        if let Some(m) = model {
            self.model = Some(m.clone());
        }
    }

    /// Map a [`SemanticEvent`] to the higher-level [`AgenticEvent`] used by
    /// claudine's hook dispatcher. Mirrors the table in `tech-design.md`
    /// §Hook Dispatch Compatibility. Returns `None` when the event is not
    /// bridged to an agentic event (e.g. OutputText / Reasoning / some
    /// ProviderExtension events).
    fn to_agentic(event: &SemanticEvent) -> Option<AgenticEvent> {
        Some(match event {
            SemanticEvent::SessionStart { .. } => AgenticEvent::SessionStart,
            SemanticEvent::TurnStart { .. } => AgenticEvent::BeforePrompt,
            SemanticEvent::TurnComplete { .. } => AgenticEvent::TurnComplete,
            SemanticEvent::ToolCall { .. } => AgenticEvent::BeforeTool,
            SemanticEvent::ToolResult { .. } => AgenticEvent::AfterTool,
            SemanticEvent::PermissionRequest { .. } => AgenticEvent::PermissionRequest,
            SemanticEvent::SubagentStart { .. } => AgenticEvent::SubagentStart,
            SemanticEvent::SubagentStop { .. } => AgenticEvent::SubagentStop,
            SemanticEvent::Error {
                terminal: true, ..
            } => AgenticEvent::TurnError,
            SemanticEvent::Info { .. }
            | SemanticEvent::Warning { .. }
            | SemanticEvent::Error { terminal: false, .. }
            | SemanticEvent::FileChange { .. }
            | SemanticEvent::PlanUpdate { .. }
            | SemanticEvent::ProviderExtension { .. } => AgenticEvent::Notification,
            SemanticEvent::OutputText { .. } | SemanticEvent::Reasoning { .. } => return None,
        })
    }

    fn dispatch_meta(&self, event: &SemanticEvent, agentic: AgenticEvent) -> DispatchEventMeta {
        let meta = claudine::stream::reporting::semantic_event_to_event_meta(
            event,
            self.provider,
            &self.env,
            if self.context_extra.is_empty() {
                None
            } else {
                Some(&self.context_extra)
            },
        );
        // Override fields that belong to the live sink but wouldn't be
        // populated by `semantic_event_to_event_meta` alone.
        DispatchEventMeta {
            event: agentic,
            cwd: Some(self.cwd.display().to_string()),
            ..meta
        }
    }

    fn render_event(&mut self, event: &SemanticEvent) {
        if !self.should_render() {
            return;
        }
        match event {
            SemanticEvent::ToolCall { name, input, .. } => {
                self.render_status(StatusState::ToolUse, Self::tool_call_description(name, input));
            }
            SemanticEvent::ToolResult {
                name,
                status,
                exit_code,
                ..
            } => {
                self.render_status(
                    StatusState::ToolUse,
                    Self::tool_result_description(name, status, *exit_code),
                );
            }
            SemanticEvent::SubagentStart { name, .. } => {
                self.render_status(
                    StatusState::Subagent,
                    Self::subagent_description('\u{2192}', name),
                );
            }
            SemanticEvent::SubagentStop { name, .. } => {
                self.render_status(
                    StatusState::Subagent,
                    Self::subagent_description('\u{2190}', name),
                );
            }
            SemanticEvent::FileChange {
                path, change_kind, ..
            } => {
                let kind = change_kind.as_deref().unwrap_or("change");
                let path = path.as_deref().unwrap_or("");
                self.render_status(StatusState::Info, format!("{kind} {path}"));
            }
            SemanticEvent::PlanUpdate { message, .. } => {
                if let Some(msg) = message {
                    self.render_status(StatusState::Info, msg.clone());
                }
            }
            SemanticEvent::Info { message, .. } => {
                self.render_status(StatusState::Info, message.clone());
            }
            SemanticEvent::Warning { message, .. } => {
                self.render_status(StatusState::Warning, message.clone());
            }
            SemanticEvent::Error { message, .. } => {
                self.render_status(StatusState::Failure, message.clone());
            }
            SemanticEvent::ProviderExtension {
                provider,
                kind,
                payload,
            } => {
                self.render_status(
                    StatusState::Info,
                    Self::provider_extension_description(*provider, kind, payload),
                );
            }
            // OutputText / Reasoning / SessionStart / TurnStart / TurnComplete /
            // PermissionRequest do not render through Status — Output/Reasoning
            // flow through their own renderers in the Phase 3.3 wiring step;
            // the others are envelope-only.
            SemanticEvent::SessionStart { .. }
            | SemanticEvent::TurnStart { .. }
            | SemanticEvent::TurnComplete { .. }
            | SemanticEvent::OutputText { .. }
            | SemanticEvent::Reasoning { .. }
            | SemanticEvent::PermissionRequest { .. } => {}
        }
    }
}

impl SemanticEventSink for LiveSemanticSink {
    fn on_semantic_event(&mut self, event: SemanticEvent) {
        // 1. LiveMetrics observation for the heartbeat.
        if let Ok(mut state) = self.live_metrics.lock() {
            state.observe_event(&event, std::time::Instant::now());
        }

        // 2. Update cached session id / model from envelope events.
        if let SemanticEvent::SessionStart {
            session_id, model, ..
        } = &event
        {
            self.update_session_state(session_id, model);
        }

        // 3. Update structured summary's tool-name rollup.
        if let SemanticEvent::ToolCall { name: Some(n), .. } = &event
            && let Ok(mut details) = self.summary_details.lock()
        {
            details.record_tool_name(n);
        }

        // 4. Render to STDERR.
        self.render_event(&event);

        // 5. Dispatch to agentic hooks when applicable.
        if let Some(agentic) = Self::to_agentic(&event) {
            let meta = self.dispatch_meta(&event, agentic);
            (self.dispatch)(agentic, meta);
        }
    }
}

fn provider_short(p: Provider) -> &'static str {
    match p {
        Provider::Claude => "claude",
        Provider::Codex => "codex",
        Provider::Gemini => "gemini",
        Provider::KimiCode => "kimi",
        Provider::OpenCode => "opencode",
        Provider::QwenCode => "qwen",
        _ => "unknown",
    }
}

fn summarize_input(value: &Value) -> Option<String> {
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

fn summarize_provider_payload(payload: &Value) -> Option<String> {
    if let Some(obj) = payload.as_object() {
        for key in ["message", "status", "name", "path"] {
            if let Some(Value::String(s)) = obj.get(key) {
                return Some(truncate(s, 80));
            }
        }
    }
    let rendered = serde_json::to_string(payload).ok()?;
    Some(truncate(&rendered, 80))
}

fn truncate(s: &str, max_chars: usize) -> String {
    let count = s.chars().count();
    if count <= max_chars {
        return s.to_string();
    }
    let prefix: String = s.chars().take(max_chars.saturating_sub(1)).collect();
    format!("{prefix}\u{2026}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::sync::Mutex as StdMutex;

    fn make_sink(
        captured_lines: Arc<StdMutex<Vec<String>>>,
        captured_dispatch: Arc<StdMutex<Vec<(AgenticEvent, String)>>>,
    ) -> LiveSemanticSink {
        let dispatch = {
            let captured = captured_dispatch.clone();
            Box::new(move |event: AgenticEvent, meta: DispatchEventMeta| {
                captured
                    .lock()
                    .unwrap()
                    .push((event, meta.tool_name.unwrap_or_default()));
            })
        };
        let emit = {
            let lines = captured_lines.clone();
            Box::new(move |line: &str| {
                lines.lock().unwrap().push(line.to_string());
            })
        };
        LiveSemanticSink::new(
            Provider::Claude,
            EnvironmentContext::default(),
            Path::new("/tmp"),
            Verbosity::Normal,
            Arc::new(Mutex::new(StructuredSummaryDetails::default())),
            dispatch,
            emit,
        )
    }

    #[test]
    fn tool_call_renders_arrow_right_prefix() {
        let lines = Arc::new(StdMutex::new(Vec::new()));
        let dispatched = Arc::new(StdMutex::new(Vec::new()));
        let mut sink = make_sink(lines.clone(), dispatched.clone());
        sink.on_semantic_event(SemanticEvent::ToolCall {
            name: Some("bash".into()),
            id: Some("t1".into()),
            input: Some(json!({"command": "ls"})),
            extra: json!({}),
        });
        let rendered = lines.lock().unwrap().join("\n");
        assert!(rendered.contains('\u{2192}'), "expected → in {rendered:?}");
        assert!(rendered.contains("bash"));
        assert!(rendered.contains("ls"));
    }

    #[test]
    fn tool_result_renders_arrow_left_prefix_with_exit_code() {
        let lines = Arc::new(StdMutex::new(Vec::new()));
        let dispatched = Arc::new(StdMutex::new(Vec::new()));
        let mut sink = make_sink(lines.clone(), dispatched.clone());
        sink.on_semantic_event(SemanticEvent::ToolResult {
            name: Some("bash".into()),
            id: Some("t1".into()),
            status: Some("failure".into()),
            exit_code: Some(1),
            output: None,
            extra: json!({}),
        });
        let rendered = lines.lock().unwrap().join("\n");
        assert!(rendered.contains('\u{2190}'));
        assert!(rendered.contains("bash"));
        assert!(rendered.contains("exit 1"));
    }

    #[test]
    fn subagent_start_and_stop_use_arrows() {
        let lines = Arc::new(StdMutex::new(Vec::new()));
        let dispatched = Arc::new(StdMutex::new(Vec::new()));
        let mut sink = make_sink(lines.clone(), dispatched.clone());
        sink.on_semantic_event(SemanticEvent::SubagentStart {
            name: Some("researcher".into()),
            id: Some("sa1".into()),
            extra: json!({}),
        });
        sink.on_semantic_event(SemanticEvent::SubagentStop {
            name: Some("researcher".into()),
            id: Some("sa1".into()),
            status: Some("success".into()),
            extra: json!({}),
        });
        let collected = lines.lock().unwrap().clone();
        assert!(collected[0].contains('\u{2192}') && collected[0].contains("researcher"));
        assert!(collected[1].contains('\u{2190}') && collected[1].contains("researcher"));
    }

    #[test]
    fn provider_extension_formatter_uses_summary_extraction_order() {
        let lines = Arc::new(StdMutex::new(Vec::new()));
        let dispatched = Arc::new(StdMutex::new(Vec::new()));
        let mut sink = make_sink(lines.clone(), dispatched.clone());
        sink.on_semantic_event(SemanticEvent::ProviderExtension {
            provider: Provider::Codex,
            kind: "item.updated".into(),
            payload: json!({"message": "still working"}),
        });
        let rendered = lines.lock().unwrap().join("\n");
        assert!(rendered.contains("codex/item.updated"));
        assert!(rendered.contains("still working"));
    }

    #[test]
    fn warning_renders_and_dispatches_as_notification() {
        let lines = Arc::new(StdMutex::new(Vec::new()));
        let dispatched = Arc::new(StdMutex::new(Vec::new()));
        let mut sink = make_sink(lines.clone(), dispatched.clone());
        sink.on_semantic_event(SemanticEvent::Warning {
            message: "rate limited".into(),
            extra: json!({}),
        });
        let rendered = lines.lock().unwrap().join("\n");
        assert!(rendered.contains("rate limited"));
        let dispatches = dispatched.lock().unwrap().clone();
        assert_eq!(dispatches[0].0, AgenticEvent::Notification);
    }

    #[test]
    fn terminal_error_dispatches_turn_error() {
        let lines = Arc::new(StdMutex::new(Vec::new()));
        let dispatched = Arc::new(StdMutex::new(Vec::new()));
        let mut sink = make_sink(lines.clone(), dispatched.clone());
        sink.on_semantic_event(SemanticEvent::Error {
            message: "billing".into(),
            terminal: true,
            extra: json!({}),
        });
        let dispatches = dispatched.lock().unwrap().clone();
        assert_eq!(dispatches[0].0, AgenticEvent::TurnError);
    }

    #[test]
    fn silent_verbosity_suppresses_stderr_but_not_dispatch() {
        let lines = Arc::new(StdMutex::new(Vec::new()));
        let dispatched = Arc::new(StdMutex::new(Vec::new()));
        let mut sink = make_sink(lines.clone(), dispatched.clone());
        sink.verbosity = Verbosity::Silent;
        sink.on_semantic_event(SemanticEvent::Warning {
            message: "rate limited".into(),
            extra: json!({}),
        });
        assert!(lines.lock().unwrap().is_empty());
        assert!(!dispatched.lock().unwrap().is_empty());
    }

    #[test]
    fn session_start_updates_cached_state_without_rendering() {
        let lines = Arc::new(StdMutex::new(Vec::new()));
        let dispatched = Arc::new(StdMutex::new(Vec::new()));
        let mut sink = make_sink(lines.clone(), dispatched.clone());
        sink.on_semantic_event(SemanticEvent::SessionStart {
            session_id: Some("s1".into()),
            model: Some("claude".into()),
            extra: json!({}),
        });
        assert_eq!(sink.session_id.as_deref(), Some("s1"));
        assert_eq!(sink.model.as_deref(), Some("claude"));
        // SessionStart does not render through Status.
        assert!(lines.lock().unwrap().is_empty());
        let dispatches = dispatched.lock().unwrap().clone();
        assert_eq!(dispatches[0].0, AgenticEvent::SessionStart);
    }

    #[test]
    fn tool_call_records_tool_name_in_summary_details() {
        let lines = Arc::new(StdMutex::new(Vec::new()));
        let dispatched = Arc::new(StdMutex::new(Vec::new()));
        let details = Arc::new(Mutex::new(StructuredSummaryDetails::default()));
        let sink_details = details.clone();
        let dispatch = {
            let captured = dispatched.clone();
            Box::new(move |event: AgenticEvent, meta: DispatchEventMeta| {
                captured
                    .lock()
                    .unwrap()
                    .push((event, meta.tool_name.unwrap_or_default()));
            })
        };
        let emit = {
            let lines = lines.clone();
            Box::new(move |line: &str| lines.lock().unwrap().push(line.to_string()))
        };
        let mut sink = LiveSemanticSink::new(
            Provider::Claude,
            EnvironmentContext::default(),
            Path::new("/tmp"),
            Verbosity::Normal,
            sink_details,
            dispatch,
            emit,
        );
        sink.on_semantic_event(SemanticEvent::ToolCall {
            name: Some("bash".into()),
            id: None,
            input: None,
            extra: json!({}),
        });
        let names = details.lock().unwrap().tool_names.clone();
        assert_eq!(names, vec!["bash".to_string()]);
    }

    #[test]
    fn live_metrics_updated_from_events() {
        let lines = Arc::new(StdMutex::new(Vec::new()));
        let dispatched = Arc::new(StdMutex::new(Vec::new()));
        let mut sink = make_sink(lines, dispatched);
        let metrics = sink.live_metrics();
        sink.on_semantic_event(SemanticEvent::ToolCall {
            name: Some("bash".into()),
            id: Some("t1".into()),
            input: None,
            extra: json!({}),
        });
        let state = metrics.lock().unwrap();
        assert_eq!(state.in_flight.len(), 1);
    }
}
