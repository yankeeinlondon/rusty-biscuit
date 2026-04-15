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
use claudine::stream::tool_display::{ToolCallDisplay, ToolDirection, ToolStatus};
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

    fn render_status_prose(&self, state: StatusState, description: String) {
        let rendered = Status::from_prose(description)
            .state(state)
            .render(&wrap_terminal());
        self.emit_line(&rendered);
    }

    fn render_tool_display(display: ToolCallDisplay) -> (String, bool) {
        let arrow = match display.direction {
            ToolDirection::Outgoing => '\u{2192}',
            ToolDirection::Incoming => '\u{2190}',
        };
        let slot = match (display.status, display.summary) {
            (Some(ToolStatus::Success), _) => Some(("success".to_string(), false)),
            (Some(ToolStatus::Error), _) => Some(("error".to_string(), true)),
            (Some(ToolStatus::Pending), _) => Some(("pending".to_string(), false)),
            (None, Some(summary)) => Some((summary, false)),
            (None, None) => None,
        };
        match slot {
            Some((text, is_error)) => {
                if is_error {
                    // Error styling: wrap in red + bold via biscuit-terminal
                    // prose markup. Only this branch goes through
                    // `Status::from_prose`; every other tool render stays on
                    // `Status::new` so user-controlled content (commands,
                    // URLs, paths) is never interpreted as markup.
                    (
                        format!(
                            "{arrow} {} \u{00b7} <red><b>{text}</b></red>",
                            display.display_name
                        ),
                        true,
                    )
                } else {
                    (
                        format!("{arrow} {} \u{00b7} {text}", display.display_name),
                        false,
                    )
                }
            }
            None => (format!("{arrow} {}", display.display_name), false),
        }
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
            SemanticEvent::ToolCall { .. } => {
                if let Some(display) = ToolCallDisplay::from_call(event) {
                    let (desc, wants_prose) = Self::render_tool_display(display);
                    if wants_prose {
                        self.render_status_prose(StatusState::ToolUse, desc);
                    } else {
                        self.render_status(StatusState::ToolUse, desc);
                    }
                }
            }
            SemanticEvent::ToolResult { .. } => {
                if let Some(display) = ToolCallDisplay::from_result(event) {
                    let (desc, wants_prose) = Self::render_tool_display(display);
                    if wants_prose {
                        self.render_status_prose(StatusState::ToolUse, desc);
                    } else {
                        self.render_status(StatusState::ToolUse, desc);
                    }
                }
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
            SemanticEvent::Warning { message, extra } => {
                // Suppress noisy malformed-line warnings on stderr — these
                // are common when providers mix non-JSON output into the
                // stream (Gemini hook logs, stack traces, etc.) and the
                // semantic parser surfaces them as Warning events per the
                // Phase 2 policy. Still dispatched and logged.
                //
                // Also suppress the Claude rate-limit Warning when the user
                // is on a Subscription (no `ANTHROPIC_API_KEY` set); the
                // dispatch and JSONL log still fire.
                if !message.starts_with("Malformed JSON on line ")
                    && !is_suppressed_claude_rate_limit(self.provider, extra)
                {
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
                if is_silent_extension_kind(*provider, kind) {
                    // Suppress stderr rendering; the event still flows
                    // through dispatch and the JSONL log.
                    return;
                }
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

/// Produce a terse one-line human summary of a [`SemanticEvent::ProviderExtension`]
/// payload.
///
/// Returns `None` when no summary can be derived from known nested shapes —
/// callers must render `provider/kind` WITHOUT a trailing ` · <payload>` in
/// that case rather than falling back to raw JSON. This is a deliberate UX
/// trade-off: a bare `provider/kind` is less informative but still readable,
/// whereas a truncated raw JSON blob is actively harmful noise on stderr.
fn summarize_provider_payload(payload: &Value) -> Option<String> {
    // Known single-string text locations, in descending specificity. Each
    // entry is a path of object keys from the root of the payload.
    let known_paths: &[&[&str]] = &[
        &["message"],
        &["status"],
        &["name"],
        &["path"],
        &["text"],
        &["content"],
        &["error", "message"],
        &["error_message"],
        &["title"],
        &["description"],
    ];

    payload.as_object()?;

    for path in known_paths {
        let mut cursor: &Value = payload;
        let mut ok = true;
        for segment in path.iter() {
            match cursor.get(*segment) {
                Some(next) => cursor = next,
                None => {
                    ok = false;
                    break;
                }
            }
        }
        if ok
            && let Some(s) = cursor.as_str().filter(|s| !s.is_empty())
        {
            return Some(s.to_string());
        }
    }

    // Nested content arrays: message.content[*].text, item.content.parts[*].text, etc.
    let nested_array_paths: &[&[&str]] = &[
        &["message", "content"],
        &["item", "content", "parts"],
        &["content", "parts"],
        &["parts"],
    ];
    for nested_path in nested_array_paths {
        let mut cursor: &Value = payload;
        let mut ok = true;
        for seg in nested_path.iter() {
            match cursor.get(*seg) {
                Some(next) => cursor = next,
                None => {
                    ok = false;
                    break;
                }
            }
        }
        if !ok {
            continue;
        }
        if let Some(array) = cursor.as_array() {
            for elem in array {
                if let Some(text) = elem
                    .get("text")
                    .and_then(Value::as_str)
                    .filter(|s| !s.is_empty())
                {
                    return Some(text.to_string());
                }
                if let Some(text) = elem
                    .get("content")
                    .and_then(Value::as_str)
                    .filter(|s| !s.is_empty())
                {
                    return Some(text.to_string());
                }
            }
        }
    }

    // Last resort: the first non-empty top-level string value. This recovers
    // summary text for shapes we haven't explicitly enumerated while still
    // avoiding a raw-JSON dump.
    if let Some(obj) = payload.as_object() {
        for (_, v) in obj.iter() {
            if let Some(s) = v.as_str().filter(|s| !s.is_empty()) {
                return Some(s.to_string());
            }
        }
    }

    None
}

/// Kinds that are known to be high-volume or entirely redundant on stderr.
/// Listed explicitly rather than relying on summary heuristics so the
/// suppression is visible, reviewable, and reversible. Events in this set
/// still flow through dispatch and the JSONL log; only the stderr status
/// line is suppressed.
const SILENT_PROVIDER_EXTENSION_KINDS: &[(Provider, &str)] = &[
    // Claude: partial assistant token deltas — redundant with OutputText.
    (Provider::Claude, "stream_event"),
    // Claude: hook lifecycle events. Claude parser (Task 2a.2) emits
    // these as ProviderExtension with kind `system/<subtype>` after
    // buffering them to trail SessionStart.
    (Provider::Claude, "system/hook_started"),
    (Provider::Claude, "system/hook_response"),
    (Provider::Claude, "system/hook_progress"),
];

fn is_silent_extension_kind(provider: Provider, kind: &str) -> bool {
    SILENT_PROVIDER_EXTENSION_KINDS
        .iter()
        .any(|(p, k)| *p == provider && *k == kind)
}

/// Suppress the Claude rate-limit Warning on stderr when the user is on a
/// Subscription (no `ANTHROPIC_API_KEY` set). The dispatch and JSONL log
/// continue to fire — only the stderr render is gated.
fn is_suppressed_claude_rate_limit(provider: Provider, extra: &Value) -> bool {
    if provider != Provider::Claude {
        return false;
    }
    let raw_kind = extra
        .get("raw_kind")
        .and_then(Value::as_str)
        .unwrap_or("");
    if raw_kind != "rate_limit_event" {
        return false;
    }
    std::env::var("ANTHROPIC_API_KEY")
        .map(|v| v.trim().is_empty())
        .unwrap_or(true)
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
        assert!(rendered.contains("Bash"));
        assert!(rendered.contains("ls"));
    }

    #[test]
    fn tool_result_renders_arrow_left_prefix_with_error_status() {
        let lines = Arc::new(StdMutex::new(Vec::new()));
        let dispatched = Arc::new(StdMutex::new(Vec::new()));
        let mut sink = make_sink(lines.clone(), dispatched.clone());
        sink.on_semantic_event(SemanticEvent::ToolResult {
            name: Some("bash".into()),
            id: Some("t1".into()),
            status: Some("failure".into()),
            exit_code: None,
            output: None,
            extra: json!({}),
        });
        let rendered = lines.lock().unwrap().join("\n");
        assert!(rendered.contains('\u{2190}'));
        assert!(rendered.contains("Bash"));
        // Status wins over summary/exit code; "failure" maps to ToolStatus::Error
        // which renders as "error" in the dim slot.
        assert!(
            rendered.contains("error"),
            "expected error status word: {rendered:?}"
        );
        assert!(
            !rendered.contains("<red>"),
            "prose markup must be interpreted, not leaked as literal text: {rendered:?}"
        );
        assert!(
            !rendered.contains("<b>"),
            "prose markup must be interpreted, not leaked as literal text: {rendered:?}"
        );
        assert!(
            rendered.contains("\u{1b}[31m"),
            "error-path rendering must emit red ANSI escape: {rendered:?}"
        );
        assert!(
            !rendered.contains("exit 1"),
            "exit_code must not render when status is present; status wins: {rendered:?}"
        );
    }

    #[test]
    fn tool_call_with_markup_looking_summary_is_not_interpreted() {
        let lines = Arc::new(StdMutex::new(Vec::new()));
        let dispatched = Arc::new(StdMutex::new(Vec::new()));
        let mut sink = make_sink(lines.clone(), dispatched.clone());
        sink.on_semantic_event(SemanticEvent::ToolCall {
            name: Some("Bash".into()),
            id: None,
            input: Some(json!({"command": "echo '<b>hi</b>'"})),
            extra: json!({}),
        });
        let rendered = lines.lock().unwrap().join("\n");
        // User input containing markup must appear verbatim — Status::new
        // does NOT interpret prose markup on the summary path.
        assert!(
            rendered.contains("<b>hi</b>"),
            "user input with prose tokens must render literally: {rendered:?}"
        );
    }

    #[test]
    fn tool_result_status_wins_over_input_summary() {
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
        assert!(rendered.contains("Bash"), "expected humanized tool name");
        // Per Task 1.4's "status wins over summary" rule, the input summary
        // "ls -la" must NOT appear when status is present — status text only.
        assert!(
            !rendered.contains("ls -la"),
            "input summary must not appear when status is present: {rendered:?}"
        );
        // "completed" is a success-ish status; from_result maps it to
        // ToolStatus::Success, which renders as "success".
        assert!(
            rendered.contains("success"),
            "expected mapped status word 'success': {rendered:?}"
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

    #[test]
    fn provider_extension_with_only_nested_text_renders_summary_not_raw_json() {
        let lines = Arc::new(StdMutex::new(Vec::new()));
        let dispatched = Arc::new(StdMutex::new(Vec::new()));
        let mut sink = make_sink(lines.clone(), dispatched.clone());

        // Payload has no top-level message/status/name/path, but nested text.
        sink.on_semantic_event(SemanticEvent::ProviderExtension {
            provider: Provider::Codex,
            kind: "future.unknown".into(),
            payload: json!({
                "item": {
                    "content": { "parts": [ { "text": "meaningful text here" } ] }
                }
            }),
        });

        let rendered = lines.lock().unwrap().join("\n");
        assert!(
            rendered.contains("meaningful text here"),
            "expected nested text preview in stderr: {rendered}"
        );
        assert!(
            !rendered.contains(r#"{"item":"#),
            "raw JSON must not appear in stderr: {rendered}"
        );
    }

    #[test]
    fn provider_extension_unresolvable_drops_payload_tail_entirely() {
        let lines = Arc::new(StdMutex::new(Vec::new()));
        let dispatched = Arc::new(StdMutex::new(Vec::new()));
        let mut sink = make_sink(lines.clone(), dispatched.clone());

        sink.on_semantic_event(SemanticEvent::ProviderExtension {
            provider: Provider::Codex,
            kind: "opaque.event".into(),
            payload: json!({
                "some_numeric_field": 42,
                "another": [1, 2, 3]
            }),
        });

        let rendered = lines.lock().unwrap().join("\n");
        assert!(
            rendered.contains("codex/opaque.event"),
            "provider/kind label must still appear: {rendered}"
        );
        assert!(
            !rendered.contains(r#"{"some_numeric_field":"#) && !rendered.contains("42"),
            "raw payload must not appear when no human-readable summary is available: {rendered}"
        );
        assert!(
            !rendered.contains(" \u{00b7} {"),
            "must not render the summary separator followed by raw JSON: {rendered}"
        );
    }

    #[test]
    fn provider_extension_respects_silent_kind_allowlist() {
        let lines = Arc::new(StdMutex::new(Vec::new()));
        let dispatched = Arc::new(StdMutex::new(Vec::new()));
        let mut sink = make_sink(lines.clone(), dispatched.clone());

        // Kinds in the silent allowlist must produce NO stderr line at all
        // (they still get dispatched and logged, just not rendered).
        sink.on_semantic_event(SemanticEvent::ProviderExtension {
            provider: Provider::Claude,
            kind: "stream_event".into(),
            payload: json!({ "delta": "chunk" }),
        });

        let rendered = lines.lock().unwrap().join("\n");
        assert!(
            !rendered.contains("claude/stream_event"),
            "silent-kind allowlist must suppress the status line entirely: {rendered}"
        );
    }

    #[test]
    fn provider_extension_claude_system_hook_kinds_are_silent() {
        // Task 2a.2 parser emits hook events with kinds `system/hook_started`,
        // `system/hook_response`, `system/hook_progress`. The sink allowlist
        // must suppress all three so subscription users don't see hook noise.
        for kind in ["system/hook_started", "system/hook_response", "system/hook_progress"] {
            let lines = Arc::new(StdMutex::new(Vec::new()));
            let dispatched = Arc::new(StdMutex::new(Vec::new()));
            let mut sink = make_sink(lines.clone(), dispatched.clone());
            sink.on_semantic_event(SemanticEvent::ProviderExtension {
                provider: Provider::Claude,
                kind: kind.into(),
                payload: json!({"hook_name": "SessionStart:startup"}),
            });
            let rendered = lines.lock().unwrap().join("\n");
            assert!(
                !rendered.contains(&format!("claude/{kind}")),
                "kind {kind:?} must be suppressed: {rendered}"
            );
        }
    }

    #[test]
    fn opencode_firecrawl_tool_use_does_not_render_via_info_glyph() {
        // Task 2c.4 regression: the ⚙ firecrawl line was OpenCode's own
        // TUI output (suppressed via noise-prefix list in profile.rs);
        // the sink's own rendering of a firecrawl ToolResult must use
        // the ← arrow via ToolCallDisplay, NOT the ⚙ Info glyph.
        let lines = Arc::new(StdMutex::new(Vec::new()));
        let dispatched = Arc::new(StdMutex::new(Vec::new()));
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
            Box::new(move |line: &str| {
                lines.lock().unwrap().push(line.to_string());
            })
        };
        let mut sink = LiveSemanticSink::new(
            Provider::OpenCode,
            EnvironmentContext::default(),
            Path::new("/tmp"),
            Verbosity::Normal,
            Arc::new(Mutex::new(StructuredSummaryDetails::default())),
            dispatch,
            emit,
        );
        sink.on_semantic_event(SemanticEvent::ToolResult {
            name: Some("firecrawl_firecrawl_search".into()),
            id: None,
            status: Some("success".into()),
            exit_code: None,
            output: None,
            extra: json!({"input": {"query": "NFL draft 2026 date"}}),
        });
        let rendered = lines.lock().unwrap().join("\n");
        assert!(
            !rendered.contains('\u{2699}'),
            "must not use the ⚙ Info glyph for tool events: {rendered:?}"
        );
        assert!(
            rendered.contains("Firecrawl Search"),
            "humanized tool name must appear: {rendered:?}"
        );
        assert!(
            rendered.contains('\u{2190}'),
            "incoming ← arrow must render: {rendered:?}"
        );
    }

    #[test]
    fn tool_call_renders_canonical_format_with_humanized_name_and_query_summary() {
        let lines = Arc::new(StdMutex::new(Vec::new()));
        let dispatched = Arc::new(StdMutex::new(Vec::new()));
        let mut sink = make_sink(lines.clone(), dispatched.clone());
        sink.on_semantic_event(SemanticEvent::ToolCall {
            name: Some("firecrawl_firecrawl_search".into()),
            id: None,
            input: Some(json!({"query": "NFL draft 2026 date"})),
            extra: json!({}),
        });
        let rendered = lines.lock().unwrap().join("\n");
        assert!(
            rendered.contains("Firecrawl Search"),
            "expected humanized name in {rendered:?}"
        );
        assert!(
            rendered.contains("NFL draft 2026 date"),
            "expected query summary in {rendered:?}"
        );
        assert!(rendered.contains('\u{2192}'), "expected → arrow");
    }

    #[test]
    fn tool_result_renders_status_word_when_status_present() {
        let lines = Arc::new(StdMutex::new(Vec::new()));
        let dispatched = Arc::new(StdMutex::new(Vec::new()));
        let mut sink = make_sink(lines.clone(), dispatched.clone());
        sink.on_semantic_event(SemanticEvent::ToolResult {
            name: Some("firecrawl_firecrawl_search".into()),
            id: None,
            status: Some("success".into()),
            exit_code: None,
            output: None,
            extra: json!({}),
        });
        let rendered = lines.lock().unwrap().join("\n");
        assert!(rendered.contains("Firecrawl Search"));
        assert!(rendered.contains("success"));
        assert!(rendered.contains('\u{2190}'));
    }

    #[test]
    fn no_captured_fixture_ever_renders_raw_json_on_stderr() {
        use std::path::Path as StdPath;

        let fixtures_dir = StdPath::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("lib")
            .join("tests")
            .join("fixtures")
            .join("providers");

        assert!(
            fixtures_dir.exists(),
            "fixtures dir must exist: {fixtures_dir:?}"
        );

        for provider_slug in &["claude", "codex", "gemini", "opencode"] {
            let fixture = fixtures_dir.join(format!("{provider_slug}.ndjson"));
            if !fixture.exists() {
                continue; // optional fixtures
            }
            let provider = match *provider_slug {
                "claude" => Provider::Claude,
                "codex" => Provider::Codex,
                "gemini" => Provider::Gemini,
                "opencode" => Provider::OpenCode,
                _ => unreachable!(),
            };
            let fixture_lines: Vec<String> = std::fs::read_to_string(&fixture)
                .expect("read fixture")
                .lines()
                .map(String::from)
                .collect();

            let lines_ref: Vec<&str> = fixture_lines.iter().map(String::as_str).collect();
            let stderr_lines =
                golden_stderr::replay_to_stderr(provider, &lines_ref, None);

            for line in &stderr_lines {
                // Heuristic: a line is "raw JSON" if it contains both `{`
                // and a JSON-shaped key-value opener like `":`.
                let has_json_obj_opener = line.contains('{') && line.contains("\":");
                assert!(
                    !has_json_obj_opener,
                    "provider={provider_slug}: stderr line contains raw JSON: {line:?}"
                );
            }
        }
    }

    /// Strip biscuit-terminal Layout soft-wrap continuations ("-\n  " or
    /// "\n  ") so assertions can check the pre-wrap content regardless of
    /// the terminal-aware column budget applied by `Status` + `Layout`.
    fn strip_layout_wraps(rendered: &str) -> String {
        rendered.replace("-\n  ", "").replace("\n  ", "")
    }

    #[test]
    fn long_summary_is_not_truncated_to_60_or_80_chars() {
        let lines = Arc::new(StdMutex::new(Vec::new()));
        let dispatched = Arc::new(StdMutex::new(Vec::new()));
        let mut sink = make_sink(lines.clone(), dispatched.clone());
        let long = "a".repeat(200);
        sink.on_semantic_event(SemanticEvent::ToolCall {
            name: Some("Bash".into()),
            id: None,
            input: Some(json!({"command": long.clone()})),
            extra: json!({}),
        });
        let rendered = lines.lock().unwrap().join("\n");
        let unwrapped = strip_layout_wraps(&rendered);
        assert!(
            unwrapped.contains(&long),
            "long command must not be truncated; got {rendered:?}"
        );
        assert!(!rendered.contains('\u{2026}'), "no ellipsis expected");
    }

    #[test]
    fn long_provider_extension_payload_is_not_capped_at_80() {
        let lines = Arc::new(StdMutex::new(Vec::new()));
        let dispatched = Arc::new(StdMutex::new(Vec::new()));
        let mut sink = make_sink(lines.clone(), dispatched.clone());
        let long = "x".repeat(300);
        sink.on_semantic_event(SemanticEvent::ProviderExtension {
            provider: Provider::Codex,
            kind: "custom.kind".into(),
            payload: json!({"message": long.clone()}),
        });
        let rendered = lines.lock().unwrap().join("\n");
        let unwrapped = strip_layout_wraps(&rendered);
        assert!(
            unwrapped.contains(&long),
            "long provider-extension message must not be truncated; got {rendered:?}"
        );
        assert!(!rendered.contains('\u{2026}'), "no ellipsis expected");
    }

    #[test]
    #[serial_test::serial]
    fn claude_rate_limit_warning_suppressed_when_anthropic_api_key_unset() {
        let _guard = TestEnvGuard::remove("ANTHROPIC_API_KEY");
        let lines = Arc::new(StdMutex::new(Vec::new()));
        let dispatched = Arc::new(StdMutex::new(Vec::new()));
        let mut sink = make_sink(lines.clone(), dispatched.clone());
        sink.on_semantic_event(SemanticEvent::Warning {
            message: "rate limit".into(),
            extra: json!({"raw_kind": "rate_limit_event"}),
        });
        assert!(
            lines.lock().unwrap().is_empty(),
            "rate-limit Warning must not render to stderr without ANTHROPIC_API_KEY"
        );
        assert!(
            !dispatched.lock().unwrap().is_empty(),
            "underlying dispatch must still fire so JSONL log retains the event"
        );
    }

    #[test]
    #[serial_test::serial]
    fn claude_rate_limit_warning_suppression_preserves_jsonl_log() {
        // Definition of Done: "underlying event still in JSONL for
        // subscription users". Explicit assertion that the event-log
        // closure fires even when the stderr render is suppressed.
        let _guard = TestEnvGuard::remove("ANTHROPIC_API_KEY");
        let lines = Arc::new(StdMutex::new(Vec::new()));
        let dispatched = Arc::new(StdMutex::new(Vec::new()));
        let logged: Arc<StdMutex<Vec<String>>> = Arc::new(StdMutex::new(Vec::new()));
        let logger = {
            let captured = logged.clone();
            Box::new(move |event: &SemanticEvent, _meta: &DispatchEventMeta| {
                captured.lock().unwrap().push(event.kind_str().into());
            })
        };
        let mut sink = make_sink(lines.clone(), dispatched.clone()).with_event_logger(logger);
        sink.on_semantic_event(SemanticEvent::Warning {
            message: "rate limit".into(),
            extra: json!({"raw_kind": "rate_limit_event"}),
        });
        assert!(lines.lock().unwrap().is_empty(), "stderr must be suppressed");
        let kinds = logged.lock().unwrap().clone();
        assert_eq!(kinds, vec!["warning".to_string()], "JSONL log must still receive the Warning event");
    }

    #[test]
    #[serial_test::serial]
    fn claude_rate_limit_warning_renders_when_anthropic_api_key_set() {
        let _guard = TestEnvGuard::set("ANTHROPIC_API_KEY", "sk-test");
        let lines = Arc::new(StdMutex::new(Vec::new()));
        let dispatched = Arc::new(StdMutex::new(Vec::new()));
        let mut sink = make_sink(lines.clone(), dispatched.clone());
        sink.on_semantic_event(SemanticEvent::Warning {
            message: "rate limit".into(),
            extra: json!({"raw_kind": "rate_limit_event"}),
        });
        let rendered = lines.lock().unwrap().join("\n");
        assert!(
            rendered.contains("rate limit"),
            "rate-limit Warning must render with API key set: {rendered:?}"
        );
    }

    /// RAII wrapper that restores the prior env var value on drop. Tests
    /// using this guard must be annotated `#[serial_test::serial]` because
    /// env vars are process-wide.
    struct TestEnvGuard {
        key: &'static str,
        prior: Option<String>,
    }
    impl TestEnvGuard {
        fn remove(key: &'static str) -> Self {
            let prior = std::env::var(key).ok();
            // SAFETY: tests are serialized via serial_test; no other thread
            // races on env vars while the guard exists.
            unsafe {
                std::env::remove_var(key);
            }
            Self { key, prior }
        }
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

    mod golden_stderr {
        use super::*;
        use claudine::stream::{create_semantic_parser, ParserConfig};

        pub(super) fn replay_to_stderr(
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
        #[serial_test::serial]
        fn claude_stderr_snapshot() {
            // Set ANTHROPIC_API_KEY so the rate-limit Warning renders on
            // stderr; the Subscription-mode suppression is covered by
            // `claude_rate_limit_warning_suppressed_when_anthropic_api_key_unset`.
            let _guard = super::TestEnvGuard::set("ANTHROPIC_API_KEY", "sk-test");
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
            assert!(joined.contains("Bash"));
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
            assert!(joined.contains("Bash"));
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
            assert!(joined.contains("Search"));
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
            assert!(joined.contains("Bash"));
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
            assert!(joined.contains("Bash"));
            assert!(joined.contains("API timeout"));
            assert!(joined.contains("opencode/some_future_event"));
        }

        #[test]
        fn opencode_tool_use_completion_shows_incoming_arrow_only() {
            let lines = replay_to_stderr(Provider::OpenCode, &[
                r#"{"type":"step_start","sessionID":"ses_1"}"#,
                r#"{"type":"tool_use","part":{"id":"t1","tool":"bash",
                     "state":{"status":"completed","input":{"command":"ls -la"},"output":"file.txt"}}}"#,
            ], Some("gpt-4o".into()));
            let joined = lines.join("\n");
            assert!(
                joined.contains('\u{2190}'),
                "incoming ← arrow must still render: {joined:?}"
            );
            assert!(
                !joined.contains('\u{2192}'),
                "outgoing → arrow must NOT render (no synthesized ToolCall): {joined:?}"
            );
            assert!(
                joined.contains("Bash"),
                "humanized tool name must appear: {joined:?}"
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
            assert!(joined.contains("Bash"));
            assert!(joined.contains("slow down"));
            assert!(joined.contains("qwen/something.new"));
        }
    }
}
