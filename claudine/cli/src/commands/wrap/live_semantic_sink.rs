// Some helpers in this module exist for tests or future call sites; keeping
// the per-item dead-code lints quiet matches the rest of the wrap module.
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
//! This sink is the primary CLI-side consumer of semantic events. It adds:
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
//! This sink is wired into `exec.rs`, `composition.rs`, and `sequence.rs`
//! as the primary stream event consumer.

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
use super::stream_io::StreamOutput;

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

/// Function type for forwarding assistant text to an external
/// [`super::exec::StreamTextRenderer`]-style renderer on stdout. This keeps
/// the sink decoupled from the rendering machinery that lives inside
/// `exec.rs` while still letting `OutputText` events flow through the
/// standard semantic pipeline.
pub(crate) type OutputTextFn = Box<dyn FnMut(&str) + Send + 'static>;

/// Function type for forwarding reasoning / thinking text to an external
/// stderr renderer (dimmed + italic).
pub(crate) type ReasoningFn = Box<dyn FnMut(&str) + Send + 'static>;

/// Function type for recording a semantic event to the JSONL stream log.
///
/// The sink constructs a [`DispatchEventMeta`] via
/// [`claudine::stream::reporting::semantic_event_to_event_meta`] and passes it
/// to the logger alongside the original semantic event so writers can choose
/// between the flattened `EventMeta` shape (the default JSONL format) and the
/// raw semantic-event JSON embedded under `extra["semantic_event"]`.
///
/// Wiring (in Phase 3.4) typically points this at
/// [`claudine::stream::reporting::write_summary_event`] as a best-effort
/// append-only writer.
pub(crate) type SemanticEventLoggerFn =
    Box<dyn Fn(&SemanticEvent, &DispatchEventMeta) + Send + Sync + 'static>;

pub(crate) struct LiveSemanticSink {
    provider: Provider,
    env: EnvironmentContext,
    cwd: PathBuf,
    verbosity: Verbosity,
    session_id: Option<String>,
    model: Option<String>,
    start_emitted: bool,
    summary_details: Arc<Mutex<StructuredSummaryDetails>>,
    context_extra: HashMap<String, Value>,
    dispatch: SemanticDispatchFn,
    emit_stderr: StderrEmitFn,
    emit_output_text: Option<OutputTextFn>,
    emit_reasoning: Option<ReasoningFn>,
    emit_event_log: Option<SemanticEventLoggerFn>,
    live_metrics: LiveMetrics,
    stream_output: Arc<StreamOutput>,
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
            start_emitted: false,
            summary_details,
            context_extra: HashMap::new(),
            dispatch,
            emit_stderr,
            emit_output_text: None,
            emit_reasoning: None,
            emit_event_log: None,
            live_metrics: progress::new_live_metrics(),
            stream_output: StreamOutput::new(),
        }
    }

    /// Convenience constructor for the wrapped-provider call sites
    /// wrapped-provider call sites in `wrap/mod.rs`. Builds a Tokio-based
    /// dispatch closure, an emit_stderr closure backed by
    /// [`StreamOutput::emit_stderr_line`], and a best-effort JSONL logger
    /// pointing at [`claudine::stream::reporting::write_summary_event`].
    pub(crate) fn with_default_wiring(
        provider: Provider,
        env: EnvironmentContext,
        cwd: &Path,
        verbosity: Verbosity,
        summary_details: Arc<Mutex<StructuredSummaryDetails>>,
    ) -> Self {
        let handle = tokio::runtime::Handle::try_current().ok();
        let runtime_context = match claudine::dispatch::DispatchRuntimeContext::load_for_env(&env) {
            Ok(runtime) => runtime,
            Err(error) => {
                tracing::warn!(%provider, "failed to preload wrapper runtime config: {error}");
                claudine::dispatch::DispatchRuntimeContext::default()
            }
        };
        let stream_output = StreamOutput::new();

        let dispatch: SemanticDispatchFn = {
            let runtime_context = runtime_context;
            Box::new(move |event, meta| {
                if let Some(handle) = handle.as_ref()
                    && let Err(error) = handle.block_on(
                        claudine::dispatch::dispatch_event_meta_with_runtime(
                            provider,
                            event,
                            meta,
                            &runtime_context,
                        ),
                    )
                {
                    tracing::warn!(%provider, %event, "live semantic dispatch failed: {error}");
                }
            })
        };

        let emit_stderr: StderrEmitFn = {
            let output = stream_output.clone();
            Box::new(move |line: &str| {
                output.emit_stderr_line(line);
            })
        };

        let event_logger: SemanticEventLoggerFn = Box::new(
            move |_event: &SemanticEvent, meta: &DispatchEventMeta| {
                if let Err(error) = claudine::stream::reporting::write_summary_event(meta) {
                    tracing::debug!(%provider, "semantic event log write failed: {error}");
                }
            },
        );

        Self {
            provider,
            env,
            cwd: cwd.to_path_buf(),
            verbosity,
            session_id: None,
            model: None,
            start_emitted: false,
            summary_details,
            context_extra: HashMap::new(),
            dispatch,
            emit_stderr,
            emit_output_text: None,
            emit_reasoning: None,
            emit_event_log: Some(event_logger),
            live_metrics: progress::new_live_metrics(),
            stream_output,
        }
    }

    /// Clone the shared stream-output coordinator so callers can share it
    /// with the heartbeat thread and stdout renderer in `exec.rs`.
    pub(crate) fn stream_output(&self) -> Arc<StreamOutput> {
        self.stream_output.clone()
    }

    pub(crate) fn with_context_extra(
        mut self,
        context_extra: HashMap<String, Value>,
    ) -> Self {
        self.context_extra = context_extra;
        self
    }

    /// Wire a stdout-rendering callback that receives every
    /// [`SemanticEvent::OutputText`]. Typically this forwards into an
    /// `exec.rs`-owned `StreamTextRenderer` so the markdown-boundary logic
    /// stays in one place.
    pub(crate) fn with_output_text_sink(mut self, emit: OutputTextFn) -> Self {
        self.emit_output_text = Some(emit);
        self
    }

    /// Wire a stderr-dimmed rendering callback for
    /// [`SemanticEvent::Reasoning`].
    pub(crate) fn with_reasoning_sink(mut self, emit: ReasoningFn) -> Self {
        self.emit_reasoning = Some(emit);
        self
    }

    /// Wire a JSONL logger invoked for every [`SemanticEvent`] the sink
    /// receives. Typical wiring points this at
    /// [`claudine::stream::reporting::write_summary_event`] with best-effort
    /// error swallowing; the callback receives both the raw semantic event
    /// and the derived [`DispatchEventMeta`] so richer writers can pick the
    /// shape that best suits their storage.
    pub(crate) fn with_event_logger(mut self, emit: SemanticEventLoggerFn) -> Self {
        self.emit_event_log = Some(emit);
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
        input: Option<&Value>,
    ) -> String {
        let name_part = name.as_deref().unwrap_or("(tool)");
        let mut parts = vec![format!("\u{2190} {name_part}")];
        if let Some(summary) = input.and_then(summarize_input) {
            parts.push(summary);
        }
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
        self.emit_agent_session_id();
    }

    /// Emit the agent's session ID to stderr unconditionally (unless already
    /// emitted). This is operational tracking info that must always be
    /// visible regardless of --quiet or --silent, matching the legacy
        /// contract.
    fn emit_agent_session_id(&mut self) {
        if self.start_emitted {
            return;
        }
        let Some(session_id) = self.session_id.as_deref() else {
            return;
        };
        let line = crate::output::format_session_start(
            self.provider,
            session_id,
            self.model.as_deref(),
        );
        self.stream_output.emit_stderr_line(&line);
        self.stream_output.emit_stderr_line("");
        self.start_emitted = true;
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
                extra,
                ..
            } => {
                let input = extra.get("input");
                self.render_status(
                    StatusState::ToolUse,
                    Self::tool_result_description(name, status, *exit_code, input),
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
                // Suppress noisy malformed-line warnings on stderr — these
                // are common when providers mix non-JSON output into the
                // stream (Gemini hook logs, stack traces, etc.) and the
                // semantic parser surfaces them as Warning events per the
                // Phase 2 policy. Still dispatched and logged.
                if !message.starts_with("Malformed JSON on line ") {
                    self.render_status(StatusState::Warning, message.clone());
                }
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

        // 4. Forward text/reasoning to their dedicated renderers before the
        //    status-line rendering so stdout writes happen in stream order.
        match &event {
            SemanticEvent::OutputText { text, .. } => {
                if let Some(emit) = self.emit_output_text.as_mut() {
                    emit(text);
                }
            }
            SemanticEvent::Reasoning { text, .. } => {
                if let Some(emit) = self.emit_reasoning.as_mut() {
                    emit(text);
                }
            }
            _ => {}
        }

        // 5. Render status line to STDERR.
        self.render_event(&event);

        // 6. Dispatch to agentic hooks when applicable, and log the
        //    resulting `DispatchEventMeta` to JSONL when a logger is wired.
        //    The logger always sees every event (even Output/Reasoning which
        //    have no agentic mapping); we build a Notification-shaped meta
        //    for those so the JSONL row still carries the full serialized
        //    semantic event under `extra["semantic_event"]`.
        let agentic = Self::to_agentic(&event);
        let log_agentic = agentic.unwrap_or(AgenticEvent::Notification);
        let meta = self.dispatch_meta(&event, log_agentic);
        if let Some(emit_log) = self.emit_event_log.as_ref() {
            emit_log(&event, &meta);
        }
        if let Some(agentic) = agentic {
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
    fn tool_result_renders_input_summary_when_extra_input_present() {
        let lines = Arc::new(StdMutex::new(Vec::new()));
        let dispatched = Arc::new(StdMutex::new(Vec::new()));
        let mut sink = make_sink(lines.clone(), dispatched.clone());
        sink.on_semantic_event(SemanticEvent::ToolResult {
            name: Some("bash".into()),
            id: Some("t1".into()),
            status: Some("completed".into()),
            exit_code: None,
            output: None,
            extra: json!({ "input": { "command": "ls -la" } }),
        });
        let rendered = lines.lock().unwrap().join("\n");
        assert!(rendered.contains('\u{2190}'), "expected ← arrow");
        assert!(rendered.contains("bash"), "expected tool name");
        assert!(
            rendered.contains("ls -la"),
            "expected input preview on the completion line: {rendered:?}"
        );
        assert!(
            rendered.contains("completed"),
            "expected status label: {rendered:?}"
        );
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
    fn output_text_flows_through_external_renderer() {
        let lines = Arc::new(StdMutex::new(Vec::new()));
        let dispatched = Arc::new(StdMutex::new(Vec::new()));
        let rendered_text = Arc::new(StdMutex::new(String::new()));
        let rendered_thinking = Arc::new(StdMutex::new(String::new()));

        let output_cb = {
            let buf = rendered_text.clone();
            Box::new(move |text: &str| {
                buf.lock().unwrap().push_str(text);
            })
        };
        let reasoning_cb = {
            let buf = rendered_thinking.clone();
            Box::new(move |text: &str| {
                buf.lock().unwrap().push_str(text);
            })
        };

        let mut sink = make_sink(lines, dispatched)
            .with_output_text_sink(output_cb)
            .with_reasoning_sink(reasoning_cb);

        sink.on_semantic_event(SemanticEvent::OutputText {
            text: "hello ".into(),
            extra: json!({}),
        });
        sink.on_semantic_event(SemanticEvent::Reasoning {
            text: "pondering".into(),
            extra: json!({}),
        });
        sink.on_semantic_event(SemanticEvent::OutputText {
            text: "world".into(),
            extra: json!({}),
        });

        assert_eq!(*rendered_text.lock().unwrap(), "hello world");
        assert_eq!(*rendered_thinking.lock().unwrap(), "pondering");
    }

    #[test]
    fn output_text_without_callback_is_dropped_not_rendered_as_status() {
        let lines = Arc::new(StdMutex::new(Vec::new()));
        let dispatched = Arc::new(StdMutex::new(Vec::new()));
        let mut sink = make_sink(lines.clone(), dispatched.clone());
        sink.on_semantic_event(SemanticEvent::OutputText {
            text: "hello".into(),
            extra: json!({}),
        });
        // No status line should be emitted for OutputText, and no hook dispatch.
        assert!(lines.lock().unwrap().is_empty());
        assert!(dispatched.lock().unwrap().is_empty());
    }

    #[test]
    fn event_logger_records_every_event_with_full_payload() {
        let lines = Arc::new(StdMutex::new(Vec::new()));
        let dispatched = Arc::new(StdMutex::new(Vec::new()));
        let logged: Arc<StdMutex<Vec<(String, DispatchEventMeta)>>> =
            Arc::new(StdMutex::new(Vec::new()));

        let logger = {
            let captured = logged.clone();
            Box::new(move |event: &SemanticEvent, meta: &DispatchEventMeta| {
                captured
                    .lock()
                    .unwrap()
                    .push((event.kind_str().into(), meta.clone()));
            })
        };

        let mut sink = make_sink(lines, dispatched).with_event_logger(logger);

        // OutputText has no agentic mapping but should still be logged.
        sink.on_semantic_event(SemanticEvent::OutputText {
            text: "hello".into(),
            extra: json!({}),
        });
        sink.on_semantic_event(SemanticEvent::ToolCall {
            name: Some("bash".into()),
            id: Some("t1".into()),
            input: Some(json!({"command": "ls"})),
            extra: json!({}),
        });
        sink.on_semantic_event(SemanticEvent::ProviderExtension {
            provider: Provider::Codex,
            kind: "item.updated".into(),
            payload: json!({"message": "still working"}),
        });

        let collected = logged.lock().unwrap().clone();
        let kinds: Vec<&str> = collected.iter().map(|(k, _)| k.as_str()).collect();
        assert_eq!(kinds, vec!["output_text", "tool_call", "provider_extension"]);

        // Every row must carry the full serialized semantic event under
        // extra["semantic_event"] so JSONL readers can replay fidelity.
        for (kind, meta) in &collected {
            let sem = meta
                .extra
                .get("semantic_event")
                .expect("semantic_event payload missing");
            assert_eq!(sem["type"], *kind);
            assert_eq!(
                meta.extra.get("synthetic_kind"),
                Some(&Value::String("stream_semantic_event".into()))
            );
        }
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

    mod golden_stderr {
        use super::*;
        use claudine::stream::{create_semantic_parser, ParserConfig};

        fn replay_to_stderr(
            provider: Provider,
            fixture: &[&str],
            model: Option<String>,
        ) -> Vec<String> {
            let captured: Arc<StdMutex<Vec<String>>> = Arc::new(StdMutex::new(Vec::new()));
            let dispatched: Arc<StdMutex<Vec<(AgenticEvent, String)>>> =
                Arc::new(StdMutex::new(Vec::new()));

            let dispatch = {
                let cap = dispatched.clone();
                Box::new(move |ev: AgenticEvent, _meta: DispatchEventMeta| {
                    cap.lock().unwrap().push((ev, String::new()));
                }) as Box<dyn Fn(AgenticEvent, DispatchEventMeta) + Send + Sync + 'static>
            };
            let emit = {
                let cap = captured.clone();
                Box::new(move |line: &str| {
                    cap.lock().unwrap().push(line.to_string());
                }) as Box<dyn Fn(&str) + Send + Sync + 'static>
            };

            let mut sink = LiveSemanticSink::new(
                provider,
                EnvironmentContext::default(),
                Path::new("/tmp"),
                Verbosity::Normal,
                Arc::new(Mutex::new(StructuredSummaryDetails::default())),
                dispatch,
                emit,
            );

            struct Rec {
                events: Arc<StdMutex<Vec<SemanticEvent>>>,
            }
            impl SemanticEventSink for Rec {
                fn on_semantic_event(&mut self, event: SemanticEvent) {
                    self.events.lock().unwrap().push(event);
                }
            }

            let inner = Arc::new(StdMutex::new(Vec::new()));
            let parser_sink = Rec {
                events: inner.clone(),
            };
            let config = ParserConfig { model };
            let mut parser = create_semantic_parser(provider, parser_sink, config);
            for line in fixture {
                parser.feed_line(line).unwrap();
            }
            drop(parser);

            let events = inner.lock().unwrap().clone();
            for event in events {
                sink.on_semantic_event(event);
            }

            captured.lock().unwrap().clone()
        }

        #[test]
        fn claude_stderr_snapshot() {
            let lines = replay_to_stderr(Provider::Claude, &[
                r#"{"type":"init","session_id":"s1","model":"claude-sonnet-4"}"#,
                r#"{"type":"tool_use","id":"t1","name":"bash","input":{"cmd":"ls -la"}}"#,
                r#"{"type":"tool_result","tool_use_id":"t1","content":"file.txt"}"#,
                r#"{"type":"task_started","task_id":"sa1","name":"researcher"}"#,
                r#"{"type":"task_progress","message":"working"}"#,
                r#"{"type":"task_completed","task_id":"sa1","name":"researcher","status":"success"}"#,
                r#"{"type":"rate_limit_event","is_throttled":true,"retry_after_ms":5000,"message":"Rate limited"}"#,
                r#"{"type":"some_future_event","x":1}"#,
            ], None);
            assert!(!lines.is_empty());
            let joined = lines.join("\n");
            assert!(joined.contains('\u{2192}'), "expected → arrow: {joined:?}");
            assert!(joined.contains('\u{2190}'), "expected ← arrow: {joined:?}");
            assert!(joined.contains("bash"));
            assert!(joined.contains("researcher"));
            assert!(joined.contains("Rate limited"));
            assert!(joined.contains("claude/some_future_event"));
        }

        #[test]
        fn codex_stderr_snapshot() {
            let lines = replay_to_stderr(Provider::Codex, &[
                r#"{"type":"thread.started","thread_id":"th-1"}"#,
                r#"{"type":"item.started","item":{"id":"cmd1","type":"command_exec","tool_name":"bash","input":{"command":"ls"}}}"#,
                r#"{"type":"item.completed","item":{"id":"cmd1","type":"command_exec","status":"success","exit_code":0,"output":"file.txt"}}"#,
                r#"{"type":"item.completed","item":{"id":"f1","type":"file_change","path":"src/lib.rs","change_kind":"modified"}}"#,
                r#"{"type":"item.completed","item":{"id":"p1","type":"plan_update","message":"Step 2"}}"#,
                r#"{"type":"error","error_type":"rate_limit","error_message":"Too many requests"}"#,
                r#"{"type":"future.unknown","payload":{"k":1}}"#,
            ], Some("codex-mini".into()));
            assert!(!lines.is_empty());
            let joined = lines.join("\n");
            assert!(joined.contains('\u{2192}'), "expected →: {joined:?}");
            assert!(joined.contains('\u{2190}'), "expected ←: {joined:?}");
            assert!(joined.contains("bash"));
            assert!(joined.contains("modified src/lib.rs"));
            assert!(joined.contains("Step 2"));
            assert!(joined.contains("Too many requests"));
            assert!(joined.contains("codex/future.unknown"));
        }

        #[test]
        fn gemini_stderr_snapshot() {
            let lines = replay_to_stderr(Provider::Gemini, &[
                r#"{"type":"init","session_id":"g1","model":"gemini-2.5-pro"}"#,
                r#"{"type":"tool_use","tool_id":"t1","tool_name":"search","parameters":{"q":"rust"}}"#,
                r#"{"type":"tool_result","tool_id":"t1","status":"success","output":{"hits":3}}"#,
                r#"{"type":"error","severity":"warning","message":"Loop detected"}"#,
                r#"{"type":"some_unknown","data":"x"}"#,
            ], None);
            assert!(!lines.is_empty());
            let joined = lines.join("\n");
            assert!(joined.contains('\u{2192}'), "expected →: {joined:?}");
            assert!(joined.contains('\u{2190}'), "expected ←: {joined:?}");
            assert!(joined.contains("search"));
            assert!(joined.contains("Loop detected"));
            assert!(joined.contains("gemini/some_unknown"));
        }

        #[test]
        fn kimi_stderr_snapshot() {
            let lines = replay_to_stderr(Provider::KimiCode, &[
                r#"{"type":"init","session_id":"k1","model":"kimi-coder"}"#,
                r#"{"type":"tool_use","id":"k1","name":"bash","input":{"cmd":"ls"}}"#,
                r#"{"type":"tool_result","tool_use_id":"k1","status":"success","content":"ok"}"#,
                r#"{"type":"error","error":{"type":"rate_limit","message":"slow down"}}"#,
                r#"{"type":"future.unknown"}"#,
            ], None);
            assert!(!lines.is_empty());
            let joined = lines.join("\n");
            assert!(joined.contains('\u{2192}'), "expected →: {joined:?}");
            assert!(joined.contains('\u{2190}'), "expected ←: {joined:?}");
            assert!(joined.contains("bash"));
            assert!(joined.contains("slow down"));
            assert!(joined.contains("kimi/future.unknown"));
        }

        #[test]
        fn opencode_stderr_snapshot() {
            let lines = replay_to_stderr(Provider::OpenCode, &[
                r#"{"type":"step_start","sessionID":"ses_1"}"#,
                r#"{"type":"tool_start","part":{"id":"t1","tool_name":"bash","input":{"cmd":"ls"}}}"#,
                r#"{"type":"tool_end","part":{"tool_use_id":"t1","status":"success","content":"ok"}}"#,
                r#"{"type":"error","error_message":"API timeout"}"#,
                r#"{"type":"some_future_event","x":1}"#,
            ], Some("gpt-4o".into()));
            assert!(!lines.is_empty());
            let joined = lines.join("\n");
            assert!(joined.contains('\u{2192}'), "expected →: {joined:?}");
            assert!(joined.contains('\u{2190}'), "expected ←: {joined:?}");
            assert!(joined.contains("bash"));
            assert!(joined.contains("API timeout"));
            assert!(joined.contains("opencode/some_future_event"));
        }

        #[test]
        fn opencode_tool_use_completion_shows_both_arrows() {
            let lines = replay_to_stderr(Provider::OpenCode, &[
                r#"{"type":"step_start","sessionID":"ses_1"}"#,
                r#"{"type":"tool_use","part":{"id":"t1","tool":"bash",
                     "state":{"status":"completed","input":{"command":"ls -la"},"output":"file.txt"}}}"#,
            ], Some("gpt-4o".into()));
            let joined = lines.join("\n");
            assert!(
                joined.contains('\u{2192}') && joined.contains('\u{2190}'),
                "expected both → and ← arrows in {joined:?}"
            );
            assert!(joined.contains("bash"));
            assert!(
                joined.contains("ls -la"),
                "expected input preview on the tool line: {joined:?}"
            );
        }

        #[test]
        fn qwen_stderr_snapshot() {
            let lines = replay_to_stderr(Provider::QwenCode, &[
                r#"{"type":"init","session_id":"q1","model":"qwen-coder"}"#,
                r#"{"type":"tool_call","id":"q1","name":"bash","input":{"command":"git status"}}"#,
                r#"{"type":"tool_response","tool_use_id":"q1","status":"success","content":"clean"}"#,
                r#"{"type":"error","error":{"type":"rate_limit","message":"slow down"}}"#,
                r#"{"type":"something.new"}"#,
            ], None);
            assert!(!lines.is_empty());
            let joined = lines.join("\n");
            assert!(joined.contains('\u{2192}'), "expected →: {joined:?}");
            assert!(joined.contains('\u{2190}'), "expected ←: {joined:?}");
            assert!(joined.contains("bash"));
            assert!(joined.contains("slow down"));
            assert!(joined.contains("qwen/something.new"));
        }
    }
}
