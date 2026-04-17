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

use biscuit_terminal::components::block_quote::BlockQuote;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::{Renderable, RenderableContent};
use biscuit_terminal::components::status::{Status, StatusState};
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::color::{Color, Tailwind};
use biscuit_terminal::utils::layout::Margin;
use claudine::events::{
    AgenticEvent, EnvironmentContext, EventMeta as DispatchEventMeta, Provider,
};
use claudine::stream::progress::{self, LiveMetrics};
use claudine::stream::semantic::{SemanticErrorKind, SemanticEvent, SemanticEventSink};
use claudine::stream::stderr::Verbosity;
use claudine::stream::thinking::render_thinking_block;
use claudine::stream::tool_display::{ToolCallDisplay, ToolDirection, ToolStatus};
use serde_json::Value;

use super::StructuredSummaryDetails;
use super::section::{Section, SectionStream, SectionTracker};
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
    claude_api_key_source: Option<String>,
    start_emitted: bool,
    summary_details: Arc<Mutex<StructuredSummaryDetails>>,
    context_extra: HashMap<String, Value>,
    dispatch: SemanticDispatchFn,
    emit_stderr: StderrEmitFn,
    emit_output_text: Option<OutputTextFn>,
    emit_event_log: Option<SemanticEventLoggerFn>,
    live_metrics: LiveMetrics,
    stream_output: Arc<StreamOutput>,
    /// Cached `Terminal` used for every status / thinking render. Building
    /// a `Terminal` runs capability detection, so hoisting the instance out
    /// of the per-event render path avoids unnecessary work for long
    /// structured sessions.
    terminal: Terminal,
    /// Section-spacing state machine shared with [`super::section::SectionStream`]
    /// and any post-stream trailer emitter obtained via [`Self::section_stream`].
    /// Encapsulates the dedup and section-transition logic so every writer
    /// against this sink sees the same running state.
    section_tracker: Arc<Mutex<SectionTracker>>,
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
            claude_api_key_source: None,
            start_emitted: false,
            summary_details,
            context_extra: HashMap::new(),
            dispatch,
            emit_stderr,
            emit_output_text: None,
            emit_event_log: None,
            live_metrics: progress::new_live_metrics(),
            stream_output: StreamOutput::new(),
            terminal: wrap_terminal(),
            section_tracker: Arc::new(Mutex::new(SectionTracker::new())),
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
                    && let Err(error) =
                        handle.block_on(claudine::dispatch::dispatch_event_meta_with_runtime(
                            provider,
                            event,
                            meta,
                            &runtime_context,
                        ))
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

        let event_logger: SemanticEventLoggerFn =
            Box::new(move |_event: &SemanticEvent, meta: &DispatchEventMeta| {
                if let Err(error) = claudine::stream::reporting::write_summary_event(meta) {
                    tracing::debug!(%provider, "semantic event log write failed: {error}");
                }
            });

        Self {
            provider,
            env,
            cwd: cwd.to_path_buf(),
            verbosity,
            session_id: None,
            model: None,
            claude_api_key_source: None,
            start_emitted: false,
            summary_details,
            context_extra: HashMap::new(),
            dispatch,
            emit_stderr,
            emit_output_text: None,
            emit_event_log: Some(event_logger),
            live_metrics: progress::new_live_metrics(),
            stream_output,
            terminal: wrap_terminal(),
            section_tracker: Arc::new(Mutex::new(SectionTracker::new())),
        }
    }

    /// Clone the shared stream-output coordinator so callers can share it
    /// with the heartbeat thread and stdout renderer in `exec.rs`.
    pub(crate) fn stream_output(&self) -> Arc<StreamOutput> {
        self.stream_output.clone()
    }

    /// Return a [`SectionStream`] that shares this sink's
    /// [`SectionTracker`] and [`StreamOutput`]. Callers use it to emit
    /// post-stream sections (final stdout trailer separator, trailer
    /// metadata) with the same spacing invariants as the live sink path.
    pub(crate) fn section_stream(&self) -> SectionStream {
        SectionStream::with_tracker(self.stream_output.clone(), self.section_tracker.clone())
    }

    pub(crate) fn with_context_extra(mut self, context_extra: HashMap<String, Value>) -> Self {
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

    /// Set the output-text callback on an already-constructed sink.
    ///
    /// Used by the structured wrapper path when the sink has been wrapped in
    /// a [`claudine::stream::semantic::SharedSemanticSink`] ahead of thread
    /// spawning — the parser builder closure locks the inner sink and calls
    /// this setter so the exec-layer `StreamTextRenderer` stays in the stdout
    /// thread while the sink itself is shared with the stderr log bridge.
    pub(crate) fn set_output_text_sink(&mut self, emit: OutputTextFn) {
        self.emit_output_text = Some(emit);
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

    /// Section-aware stderr emit used by every status render path.
    ///
    /// Delegates to [`SectionTracker::classify`] for the dedup and
    /// section-transition logic so this sink stays in sync with the
    /// [`super::section::SectionStream`] reference implementation.
    ///
    /// ## Rules
    /// - Section transitions are separated by exactly one blank line.
    /// - Consecutive blank lines inside a section collapse to one.
    /// - No leading blank line is emitted before the first rendered line.
    fn emit_section_line(&mut self, section: Section, line: &str) {
        let result = {
            let mut tracker = self
                .section_tracker
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            tracker.classify(section, line)
        };
        if let Some((needs_separator, _)) = result {
            if needs_separator {
                (self.emit_stderr)("");
            }
            (self.emit_stderr)(line);
        }
    }

    /// Emit a line tagged as [`Section::TrailerMetadata`], using the
    /// section tracker for spacing. Intended for post-execution summary
    /// lines that belong in the trailer section (cost, duration, tool
    /// rollup).
    pub(crate) fn emit_trailer_line(&mut self, line: &str) {
        self.emit_section_line(Section::TrailerMetadata, line);
    }

    fn render_status(&mut self, section: Section, state: StatusState, description: String) {
        let rendered = Status::new(description).state(state).render(&self.terminal);
        self.emit_section_line(section, &rendered);
    }

    fn render_status_prose(&mut self, section: Section, state: StatusState, description: String) {
        let rendered = Status::from_prose(description)
            .state(state)
            .render(&self.terminal);
        self.emit_section_line(section, &rendered);
    }

    /// Render a typed [`SemanticEvent::Error`] as a colored `BlockQuote`
    /// with a label and border derived from [`SemanticErrorKind`]. Each
    /// rendered line is fed through [`Self::emit_section_line`] so the
    /// surrounding section spacing stays consistent with status renders.
    fn render_error_block(&mut self, section: Section, kind: SemanticErrorKind, message: &str) {
        let (label, border_color) = error_kind_presentation(kind);
        let escaped = escape_prose(message);
        let body = format!("<red><b>{label}</b></red>\n{escaped}");
        let mut block = BlockQuote::new(RenderableContent::from(Prose::new(body)), None::<&str>)
            .with_left_block_color(border_color)
            .with_border("\u{258c} ");
        block.layout_mut().left_margin = Margin::Chars(0);
        block.layout_mut().right_margin = Margin::Chars(0);
        let rendered = block.render(&self.terminal);
        for line in rendered.lines() {
            self.emit_section_line(section, line);
        }
    }

    /// Render a `ToolCallDisplay` into prose-markup for
    /// [`Status::from_prose`]. Per spec:
    ///
    /// - Outgoing summary / incoming success / incoming pending render the
    ///   slot as dim-italic (`<dim><i>…</i></dim>`).
    /// - Incoming error renders the word `error` as red + bold, followed
    ///   by a dim-italic error snippet (exit code + last non-empty line of
    ///   output) when the upstream event provides one.
    /// - User-controlled content (commands, URLs, paths, raw JSON
    ///   fallbacks) is passed through [`escape_prose`] so stray `<`, `>`,
    ///   `{`, or `\` cannot be interpreted as prose markup.
    /// - The slot is wrapped in parentheses attached to the tool name
    ///   (e.g. `→ Bash(<dim><i>bash ls</i></dim>)`) so the rendering
    ///   reads like a function call rather than a `Name · summary` pair.
    fn render_tool_display(display: ToolCallDisplay) -> String {
        let arrow = match display.direction {
            ToolDirection::Outgoing => '\u{2192}',
            ToolDirection::Incoming => '\u{2190}',
        };
        let name = escape_prose(&display.display_name);
        let slot = match (display.status, display.summary) {
            (Some(ToolStatus::Success), _) => Some("<dim><i>successful</i></dim>".to_string()),
            (Some(ToolStatus::Error), _) => {
                let mut s = String::from("<red><b>error</b></red>");
                if let Some(detail) = display.error_detail.as_deref().filter(|s| !s.is_empty()) {
                    s.push_str(&format!(" <dim><i>{}</i></dim>", escape_prose(detail)));
                }
                Some(s)
            }
            (Some(ToolStatus::Pending), _) => Some("<dim><i>pending</i></dim>".to_string()),
            (None, Some(summary)) => Some(format!("<dim><i>{}</i></dim>", escape_prose(&summary))),
            (None, None) => None,
        };
        match slot {
            Some(text) => format!("{arrow} {name}({text})"),
            None => format!("{arrow} {name}"),
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
        let line =
            crate::output::format_session_start(self.provider, session_id, self.model.as_deref());
        // Route through the section-aware emitter so tests that capture
        // the `emit_stderr` closure see this line, and so subsequent
        // section transitions get exactly one blank separator (the dedup
        // absorbs the trailing blank below when the next event is in a
        // different section).
        self.emit_section_line(Section::SessionAndModel, &line);
        self.emit_section_line(Section::SessionAndModel, "");
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
            SemanticEvent::Error { terminal: true, .. } => AgenticEvent::TurnError,
            SemanticEvent::Info { .. }
            | SemanticEvent::Warning { .. }
            | SemanticEvent::Error {
                terminal: false, ..
            }
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
        // Every event variant handled in this match renders into the
        // ToolUseAndEvents section. SessionAndModel is handled separately
        // via `emit_agent_session_id`; Thinking is handled via the
        // `Reasoning` branch of [`SemanticEventSink::on_semantic_event`];
        // FinalStdout is entered via `enter_final_stdout` in the
        // `OutputText` branch of that same method; TrailerMetadata is
        // emitted post-stream through [`Self::section_stream`] by callers
        // in `wrap/mod.rs` and `wrap/composition.rs`.
        let section = Section::ToolUseAndEvents;
        match event {
            SemanticEvent::ToolCall { .. } => {
                if let Some(display) = ToolCallDisplay::from_call(event) {
                    let desc = Self::render_tool_display(display);
                    self.render_status_prose(section, StatusState::ToolUse, desc);
                }
            }
            SemanticEvent::ToolResult { .. } => {
                if let Some(display) = ToolCallDisplay::from_result(event) {
                    let desc = Self::render_tool_display(display);
                    self.render_status_prose(section, StatusState::ToolUse, desc);
                }
            }
            SemanticEvent::SubagentStart { name, .. } => {
                self.render_status(
                    section,
                    StatusState::Subagent,
                    Self::subagent_description('\u{2192}', name),
                );
            }
            SemanticEvent::SubagentStop { name, .. } => {
                self.render_status(
                    section,
                    StatusState::Subagent,
                    Self::subagent_description('\u{2190}', name),
                );
            }
            SemanticEvent::FileChange {
                path, change_kind, ..
            } => {
                // Suppress rendering when the event carries neither a path
                // nor a classification. Codex can emit provisional
                // `file_change` items with empty bodies that would otherwise
                // appear as a bare "change" line with no context.
                let path = path.as_deref().unwrap_or("");
                let kind = change_kind.as_deref();
                if path.is_empty() && kind.is_none() {
                    return;
                }
                let kind_label = kind.unwrap_or("change");
                let line = if path.is_empty() {
                    kind_label.to_string()
                } else {
                    format!("{kind_label} {path}")
                };
                self.render_status(section, StatusState::Info, line);
            }
            SemanticEvent::PlanUpdate { message, .. } => {
                if let Some(msg) = message {
                    self.render_status(section, StatusState::Info, msg.clone());
                }
            }
            SemanticEvent::Info { message, .. } => {
                self.render_status(section, StatusState::Info, message.clone());
            }
            SemanticEvent::Warning { message, extra } => {
                // Suppress noisy malformed-line warnings on stderr — these
                // are common when providers mix non-JSON output into the
                // stream (Gemini hook logs, stack traces, etc.) and the
                // semantic parser surfaces them as Warning events per the
                // Phase 2 policy. Still dispatched and logged.
                //
                // Also suppress the legacy generic Claude rate-limit Warning
                // when the session metadata shows a subscription auth source.
                // Explicit metadata text such as "approaching limit" must
                // still render so users can see the next reset window.
                if !message.starts_with("Malformed JSON on line ")
                    && !is_suppressed_claude_rate_limit(
                        self.provider,
                        message,
                        extra,
                        self.claude_api_key_source.as_deref(),
                    )
                {
                    self.render_status(section, StatusState::Warning, message.clone());
                }
            }
            SemanticEvent::Error { message, kind, .. } => {
                self.render_error_block(section, *kind, message);
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
                    section,
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
            session_id,
            model,
            extra,
        } = &event
        {
            self.update_session_state(session_id, model);
            self.claude_api_key_source = extra
                .get("api_key_source")
                .and_then(Value::as_str)
                .map(String::from);
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
                    // Route the transition into the FinalStdout section
                    // through the shared section tracker so the separator
                    // blank (between stderr events and final stdout) is
                    // emitted exactly once. The raw text bytes continue to
                    // flow directly to the caller's renderer.
                    {
                        let mut tracker = self
                            .section_tracker
                            .lock()
                            .unwrap_or_else(|e| e.into_inner());
                        if let Some((needs_separator, _)) =
                            tracker.classify(Section::FinalStdout, "x")
                        {
                            drop(tracker);
                            if needs_separator {
                                (self.emit_stderr)("");
                            }
                        }
                    }
                    emit(text);
                }
            }
            SemanticEvent::Reasoning { text, .. } => {
                let block = render_thinking_block(text, &self.terminal);
                if !block.is_empty() {
                    // Split the multi-line block render into lines so the
                    // dedup works per-line; section transitions only
                    // insert blanks between sections, not between lines of
                    // a single block.
                    for line in block.lines() {
                        self.emit_section_line(Section::Thinking, line);
                    }
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

/// Escape user-controlled text so it can be safely interpolated into
/// biscuit-terminal prose markup without being parsed as tags / tokens.
///
/// Biscuit-terminal's `Prose` parser recognises backslash escapes for `<`,
/// `>`, `{`, and `\`; escaping those four characters is sufficient to
/// prevent arbitrary user strings (commands, paths, URLs, raw JSON) from
/// being interpreted as markup.
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

/// Pick the human label and border color for a typed
/// [`SemanticErrorKind`] when rendered as a live-sink BlockQuote.
fn error_kind_presentation(kind: SemanticErrorKind) -> (&'static str, Color) {
    match kind {
        SemanticErrorKind::Configuration => {
            ("Configuration Error", Color::Tailwind(Tailwind::Orange700))
        }
        SemanticErrorKind::AgentNative => ("Agent Error", Color::Tailwind(Tailwind::Red700)),
        SemanticErrorKind::ApiRemote => ("API Error", Color::Tailwind(Tailwind::Red700)),
        SemanticErrorKind::Interrupted => ("Interrupted", Color::Tailwind(Tailwind::Yellow700)),
        SemanticErrorKind::Unknown => ("Error", Color::Tailwind(Tailwind::Red700)),
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
        if ok && let Some(s) = cursor.as_str().filter(|s| !s.is_empty()) {
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

/// Suppress only the legacy generic Claude `rate limit` Warning on stderr
/// when the session metadata shows subscription auth. Explicit Claude
/// metadata text must still render because it can include cap-window timing.
fn is_suppressed_claude_rate_limit(
    provider: Provider,
    message: &str,
    extra: &Value,
    api_key_source: Option<&str>,
) -> bool {
    if provider != Provider::Claude {
        return false;
    }
    let raw_kind = extra.get("raw_kind").and_then(Value::as_str).unwrap_or("");
    if raw_kind != "rate_limit_event" || message.trim() != "rate limit" {
        return false;
    }
    if let Some(api_key_source) = api_key_source {
        return api_key_source != "ANTHROPIC_API_KEY";
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
    fn tool_call_renders_with_parentheses_format() {
        let lines = Arc::new(StdMutex::new(Vec::new()));
        let dispatched = Arc::new(StdMutex::new(Vec::new()));
        let mut sink = make_sink(lines.clone(), dispatched.clone());
        sink.on_semantic_event(SemanticEvent::ToolCall {
            name: Some("Bash".into()),
            id: Some("t1".into()),
            input: Some(json!({"command": "ls -la"})),
            extra: json!({}),
        });
        let rendered = lines.lock().unwrap().join("\n");
        assert!(
            rendered.contains("Bash(") && rendered.contains(")"),
            "tool call must render as Name(summary) with parentheses: {rendered:?}"
        );
        assert!(
            !rendered.contains(" \u{00b7} "),
            "tool call must no longer use the `·` separator: {rendered:?}"
        );
        // Shell name gets prepended to the command inside the parens.
        assert!(
            rendered.contains("bash ls -la"),
            "summary inside parens must include prepended shell name: {rendered:?}"
        );
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
        // ToolStatus::Success, which renders as "successful".
        assert!(
            rendered.contains("successful"),
            "expected mapped status word 'successful': {rendered:?}"
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
            kind: SemanticErrorKind::ApiRemote,
            extra: json!({}),
        });
        let dispatches = dispatched.lock().unwrap().clone();
        assert_eq!(dispatches[0].0, AgenticEvent::TurnError);
    }

    #[test]
    fn error_event_renders_blockquote_with_red_border() {
        let lines = Arc::new(StdMutex::new(Vec::new()));
        let dispatched = Arc::new(StdMutex::new(Vec::new()));
        let mut sink = make_sink(lines.clone(), dispatched.clone());
        sink.on_semantic_event(SemanticEvent::Error {
            message: "Quota exceeded".into(),
            terminal: true,
            kind: SemanticErrorKind::ApiRemote,
            extra: json!({}),
        });
        let rendered = lines.lock().unwrap().join("\n");
        assert!(
            rendered.contains("API Error"),
            "expected API Error label, got: {rendered:?}"
        );
        assert!(
            rendered.contains("Quota exceeded"),
            "expected message text, got: {rendered:?}"
        );
        assert!(
            rendered.contains('\u{258c}'),
            "expected wider block-quote border (▌), got: {rendered:?}"
        );
    }

    #[test]
    fn interrupted_error_renders_blockquote_with_interrupted_label() {
        let lines = Arc::new(StdMutex::new(Vec::new()));
        let dispatched = Arc::new(StdMutex::new(Vec::new()));
        let mut sink = make_sink(lines.clone(), dispatched.clone());
        sink.on_semantic_event(SemanticEvent::Error {
            message: "User cancelled".into(),
            terminal: true,
            kind: SemanticErrorKind::Interrupted,
            extra: json!({}),
        });
        let rendered = lines.lock().unwrap().join("\n");
        assert!(
            rendered.contains("Interrupted"),
            "expected Interrupted label, got: {rendered:?}"
        );
        assert!(rendered.contains('\u{258c}'));
    }

    #[test]
    fn configuration_error_renders_blockquote_with_configuration_label() {
        let lines = Arc::new(StdMutex::new(Vec::new()));
        let dispatched = Arc::new(StdMutex::new(Vec::new()));
        let mut sink = make_sink(lines.clone(), dispatched.clone());
        sink.on_semantic_event(SemanticEvent::Error {
            message: "Bad API key".into(),
            terminal: true,
            kind: SemanticErrorKind::Configuration,
            extra: json!({}),
        });
        let rendered = lines.lock().unwrap().join("\n");
        assert!(
            rendered.contains("Configuration Error"),
            "expected Configuration Error label, got: {rendered:?}"
        );
        assert!(rendered.contains('\u{258c}'));
    }

    #[test]
    fn error_kind_presentation_returns_expected_labels() {
        assert_eq!(
            error_kind_presentation(SemanticErrorKind::Configuration).0,
            "Configuration Error"
        );
        assert_eq!(
            error_kind_presentation(SemanticErrorKind::AgentNative).0,
            "Agent Error"
        );
        assert_eq!(
            error_kind_presentation(SemanticErrorKind::ApiRemote).0,
            "API Error"
        );
        assert_eq!(
            error_kind_presentation(SemanticErrorKind::Interrupted).0,
            "Interrupted"
        );
        assert_eq!(
            error_kind_presentation(SemanticErrorKind::Unknown).0,
            "Error"
        );
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
    fn session_start_updates_cached_state_and_emits_session_header() {
        let lines = Arc::new(StdMutex::new(Vec::new()));
        let dispatched = Arc::new(StdMutex::new(Vec::new()));
        let mut sink = make_sink(lines.clone(), dispatched.clone());
        sink.on_semantic_event(SemanticEvent::SessionStart {
            session_id: Some("s1".into()),
            model: Some("claude".into()),
            extra: json!({"api_key_source": "none"}),
        });
        assert_eq!(sink.session_id.as_deref(), Some("s1"));
        assert_eq!(sink.model.as_deref(), Some("claude"));
        assert_eq!(sink.claude_api_key_source.as_deref(), Some("none"));
        // Task 3.2 routes the session header through the section-aware
        // emit path so the `emit_stderr` closure captures it. The header
        // line must appear; a trailing blank is allowed but not required.
        let collected = lines.lock().unwrap().clone();
        assert!(
            collected.iter().any(|l| l.contains("s1")),
            "session id must appear in stderr capture: {collected:?}"
        );
        let dispatches = dispatched.lock().unwrap().clone();
        assert_eq!(dispatches[0].0, AgenticEvent::SessionStart);
    }

    #[test]
    fn full_run_has_no_two_consecutive_blank_lines() {
        let lines = Arc::new(StdMutex::new(Vec::new()));
        let dispatched = Arc::new(StdMutex::new(Vec::new()));
        let mut sink = make_sink(lines.clone(), dispatched.clone());
        // Synthetic sequence representing every section transition.
        sink.on_semantic_event(SemanticEvent::SessionStart {
            session_id: Some("s1".into()),
            model: Some("claude-opus-4-6".into()),
            extra: json!({}),
        });
        sink.on_semantic_event(SemanticEvent::Reasoning {
            text: "thinking…".into(),
            extra: json!({}),
        });
        sink.on_semantic_event(SemanticEvent::ToolCall {
            name: Some("Bash".into()),
            id: None,
            input: Some(json!({"command": "ls"})),
            extra: json!({}),
        });
        sink.on_semantic_event(SemanticEvent::ToolResult {
            name: Some("Bash".into()),
            id: None,
            status: Some("success".into()),
            exit_code: Some(0),
            output: None,
            extra: json!({}),
        });
        sink.on_semantic_event(SemanticEvent::OutputText {
            text: "answer".into(),
            extra: json!({}),
        });
        sink.on_semantic_event(SemanticEvent::TurnComplete {
            provider_status: Some("ok".into()),
            token_usage: None,
            cost_usd: None,
            duration_ms: Some(100),
            extra: json!({}),
        });
        let collected = lines.lock().unwrap().clone();
        let mut prev_blank = false;
        for line in &collected {
            let is_blank = line.trim().is_empty();
            if is_blank && prev_blank {
                panic!("two consecutive blank lines in {collected:?}");
            }
            prev_blank = is_blank;
        }
    }

    /// Combined section golden test: feed every section transition through
    /// the sink and verify no two consecutive blank lines exist in the
    /// combined stderr output.
    #[test]
    fn combined_sections_have_no_consecutive_blanks() {
        let lines = Arc::new(StdMutex::new(Vec::new()));
        let dispatched = Arc::new(StdMutex::new(Vec::new()));
        let mut sink = make_sink(lines.clone(), dispatched.clone());

        // SessionStart -> SessionAndModel section
        sink.on_semantic_event(SemanticEvent::SessionStart {
            session_id: Some("s-combined".into()),
            model: Some("test-model".into()),
            extra: json!({}),
        });
        // Reasoning -> Thinking section
        sink.on_semantic_event(SemanticEvent::Reasoning {
            text: "pondering deeply".into(),
            extra: json!({}),
        });
        // ToolCall -> ToolUseAndEvents section
        sink.on_semantic_event(SemanticEvent::ToolCall {
            name: Some("Bash".into()),
            id: Some("t1".into()),
            input: Some(json!({"command": "echo hello"})),
            extra: json!({}),
        });
        // ToolResult -> stays in ToolUseAndEvents
        sink.on_semantic_event(SemanticEvent::ToolResult {
            name: Some("Bash".into()),
            id: Some("t1".into()),
            status: Some("success".into()),
            exit_code: Some(0),
            output: None,
            extra: json!({}),
        });
        // OutputText -> does not emit to stderr (goes to stdout callback)
        sink.on_semantic_event(SemanticEvent::OutputText {
            text: "the answer".into(),
            extra: json!({}),
        });
        // TurnComplete -> envelope-only, no section emission
        sink.on_semantic_event(SemanticEvent::TurnComplete {
            provider_status: Some("ok".into()),
            token_usage: None,
            cost_usd: None,
            duration_ms: Some(500),
            extra: json!({}),
        });

        let collected = lines.lock().unwrap().clone();
        // Basic sanity: the sink must have emitted something.
        assert!(!collected.is_empty(), "sink must emit lines: {collected:?}");

        // Core invariant: no two consecutive blank lines.
        let mut prev_blank = false;
        for line in &collected {
            let is_blank = line.trim().is_empty();
            if is_blank && prev_blank {
                panic!("two consecutive blank lines in combined section output:\n{collected:#?}");
            }
            prev_blank = is_blank;
        }
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

        let output_cb = {
            let buf = rendered_text.clone();
            Box::new(move |text: &str| {
                buf.lock().unwrap().push_str(text);
            })
        };

        let mut sink = make_sink(lines.clone(), dispatched).with_output_text_sink(output_cb);

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
        // Reasoning is now rendered directly by LiveSemanticSink via
        // render_thinking_block into the stderr lines (not through an
        // external callback).
        let captured = lines.lock().unwrap().join("\n");
        assert!(
            captured.contains("pondering"),
            "reasoning text must appear in stderr: {captured:?}"
        );
    }

    #[test]
    fn first_output_text_inserts_section_separator_between_tool_and_final_stdout() {
        let lines = Arc::new(StdMutex::new(Vec::new()));
        let dispatched = Arc::new(StdMutex::new(Vec::new()));
        let rendered_text = Arc::new(StdMutex::new(String::new()));

        let output_cb = {
            let buf = rendered_text.clone();
            Box::new(move |text: &str| {
                buf.lock().unwrap().push_str(text);
            })
        };

        let mut sink = make_sink(lines.clone(), dispatched).with_output_text_sink(output_cb);

        sink.on_semantic_event(SemanticEvent::ToolResult {
            name: Some("bash".into()),
            id: None,
            status: Some("success".into()),
            exit_code: None,
            output: None,
            extra: json!({}),
        });
        sink.on_semantic_event(SemanticEvent::OutputText {
            text: "chunk-a ".into(),
            extra: json!({}),
        });
        sink.on_semantic_event(SemanticEvent::OutputText {
            text: "chunk-b".into(),
            extra: json!({}),
        });

        let captured = lines.lock().unwrap();
        // Exactly one blank separator between the tool render (stderr)
        // and the stdout-bound assistant text.
        let blanks = captured.iter().filter(|l| l.is_empty()).count();
        assert_eq!(
            blanks, 1,
            "expected exactly one section-separator blank: {captured:?}"
        );
        // The last stderr line is that separator (the OutputText payload
        // does not go through `emit_stderr`).
        assert!(captured.last().is_some_and(|l| l.is_empty()));
        // And both OutputText chunks still reach the external renderer.
        assert_eq!(*rendered_text.lock().unwrap(), "chunk-a chunk-b");
    }

    #[test]
    fn emit_trailer_line_inserts_single_separator_after_final_stdout() {
        let lines = Arc::new(StdMutex::new(Vec::new()));
        let dispatched = Arc::new(StdMutex::new(Vec::new()));

        let output_cb = Box::new(move |_text: &str| {
            // The renderer would normally write to stdout; we do not need
            // to capture it for this assertion. The section tracker is
            // what matters.
        });

        let mut sink = make_sink(lines.clone(), dispatched).with_output_text_sink(output_cb);

        sink.on_semantic_event(SemanticEvent::ToolResult {
            name: Some("bash".into()),
            id: None,
            status: Some("success".into()),
            exit_code: None,
            output: None,
            extra: json!({}),
        });
        sink.on_semantic_event(SemanticEvent::OutputText {
            text: "result".into(),
            extra: json!({}),
        });

        sink.emit_trailer_line("✓ 5s");
        sink.emit_trailer_line("  secondary");

        let captured = lines.lock().unwrap().clone();
        // Captured stderr lines should be, in order:
        //   <tool line>, "" (tool→final separator), "" (final→trailer
        //   separator), "✓ 5s", "  secondary"
        // The two separators are tagged to different sections so the
        // dedup does NOT collapse them; every section transition emits a
        // separator.
        assert!(!captured.is_empty());
        let blanks_then: Vec<_> = captured
            .iter()
            .enumerate()
            .filter(|(_, l)| l.is_empty())
            .map(|(idx, _)| idx)
            .collect();
        assert_eq!(
            blanks_then.len(),
            2,
            "one separator into FinalStdout and one into TrailerMetadata: {captured:?}"
        );
        assert!(
            captured.iter().any(|l| l.contains("✓ 5s")),
            "trailer line must be present: {captured:?}"
        );
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
        assert_eq!(
            kinds,
            vec!["output_text", "tool_call", "provider_extension"]
        );

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
        for kind in [
            "system/hook_started",
            "system/hook_response",
            "system/hook_progress",
        ] {
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
        assert!(rendered.contains("successful"));
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
            let stderr_lines = golden_stderr::replay_to_stderr(provider, &lines_ref, None);

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
    fn claude_generic_rate_limit_warning_suppressed_for_subscription_metadata() {
        let _guard = TestEnvGuard::set("ANTHROPIC_API_KEY", "sk-test");
        let lines = Arc::new(StdMutex::new(Vec::new()));
        let dispatched = Arc::new(StdMutex::new(Vec::new()));
        let mut sink = make_sink(lines.clone(), dispatched.clone());
        sink.on_semantic_event(SemanticEvent::SessionStart {
            session_id: Some("s1".into()),
            model: Some("claude".into()),
            extra: json!({"api_key_source": "none"}),
        });
        lines.lock().unwrap().clear();
        sink.on_semantic_event(SemanticEvent::Warning {
            message: "rate limit".into(),
            extra: json!({"raw_kind": "rate_limit_event"}),
        });
        assert!(
            lines.lock().unwrap().is_empty(),
            "generic rate-limit Warning must not render for subscription auth"
        );
        assert!(
            !dispatched.lock().unwrap().is_empty(),
            "underlying dispatch must still fire so JSONL log retains the event"
        );
    }

    #[test]
    #[serial_test::serial]
    fn claude_generic_rate_limit_warning_suppression_preserves_jsonl_log() {
        // Definition of Done: "underlying event still in JSONL for
        // subscription users". Explicit assertion that the event-log
        // closure fires even when the stderr render is suppressed.
        let _guard = TestEnvGuard::set("ANTHROPIC_API_KEY", "sk-test");
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
        sink.on_semantic_event(SemanticEvent::SessionStart {
            session_id: Some("s1".into()),
            model: Some("claude".into()),
            extra: json!({"api_key_source": "none"}),
        });
        lines.lock().unwrap().clear();
        logged.lock().unwrap().clear();
        sink.on_semantic_event(SemanticEvent::Warning {
            message: "rate limit".into(),
            extra: json!({"raw_kind": "rate_limit_event"}),
        });
        assert!(
            lines.lock().unwrap().is_empty(),
            "stderr must be suppressed"
        );
        let kinds = logged.lock().unwrap().clone();
        assert_eq!(
            kinds,
            vec!["warning".to_string()],
            "JSONL log must still receive the Warning event"
        );
    }

    #[test]
    #[serial_test::serial]
    fn claude_generic_rate_limit_warning_renders_for_api_key_metadata() {
        let _guard = TestEnvGuard::remove("ANTHROPIC_API_KEY");
        let lines = Arc::new(StdMutex::new(Vec::new()));
        let dispatched = Arc::new(StdMutex::new(Vec::new()));
        let mut sink = make_sink(lines.clone(), dispatched.clone());
        sink.on_semantic_event(SemanticEvent::SessionStart {
            session_id: Some("s1".into()),
            model: Some("claude".into()),
            extra: json!({"api_key_source": "ANTHROPIC_API_KEY"}),
        });
        lines.lock().unwrap().clear();
        sink.on_semantic_event(SemanticEvent::Warning {
            message: "rate limit".into(),
            extra: json!({"raw_kind": "rate_limit_event"}),
        });
        let rendered = lines.lock().unwrap().join("\n");
        assert!(
            rendered.contains("rate limit"),
            "generic rate-limit Warning must render for API-key auth: {rendered:?}"
        );
    }

    #[test]
    #[serial_test::serial]
    fn claude_explicit_rate_limit_message_renders_for_subscription_metadata() {
        let _guard = TestEnvGuard::remove("ANTHROPIC_API_KEY");
        let lines = Arc::new(StdMutex::new(Vec::new()));
        let dispatched = Arc::new(StdMutex::new(Vec::new()));
        let mut sink = make_sink(lines.clone(), dispatched.clone());
        sink.on_semantic_event(SemanticEvent::SessionStart {
            session_id: Some("s1".into()),
            model: Some("claude".into()),
            extra: json!({"api_key_source": "none"}),
        });
        lines.lock().unwrap().clear();
        sink.on_semantic_event(SemanticEvent::Warning {
            message: "Claude session usage limit approaching; next session window opens at 2024-04-01 19:33:20 UTC".into(),
            extra: json!({
                "raw_kind": "rate_limit_event",
                "rate_limit_status": "approaching_limit",
                "reset_at": "2024-04-01T19:33:20+00:00"
            }),
        });
        let rendered = lines.lock().unwrap().join("\n");
        let unwrapped = rendered.replace("\n  ", "");
        assert!(
            unwrapped.contains("next session window opens at 2024-04-01 19:33:20 UTC"),
            "explicit Claude rate-limit metadata must render for subscriptions: {rendered:?}"
        );
    }

    #[test]
    fn reasoning_emits_block_quote_to_stderr_in_thinking_section() {
        let lines = Arc::new(StdMutex::new(Vec::new()));
        let dispatched = Arc::new(StdMutex::new(Vec::new()));
        let mut sink = make_sink(lines.clone(), dispatched.clone());
        sink.on_semantic_event(SemanticEvent::Reasoning {
            text: "considering the options".into(),
            extra: json!({}),
        });
        let rendered = lines.lock().unwrap().join("\n");
        assert!(
            rendered.contains("considering the options"),
            "reasoning text must appear in stderr: {rendered:?}"
        );
    }

    #[test]
    fn reasoning_then_tool_call_transitions_with_single_blank() {
        let lines = Arc::new(StdMutex::new(Vec::new()));
        let dispatched = Arc::new(StdMutex::new(Vec::new()));
        let mut sink = make_sink(lines.clone(), dispatched.clone());
        sink.on_semantic_event(SemanticEvent::Reasoning {
            text: "planning".into(),
            extra: json!({}),
        });
        sink.on_semantic_event(SemanticEvent::ToolCall {
            name: Some("Bash".into()),
            id: None,
            input: Some(json!({"command": "ls"})),
            extra: json!({}),
        });
        let collected = lines.lock().unwrap().clone();
        let mut prev_blank = false;
        for line in &collected {
            let is_blank = line.trim().is_empty();
            assert!(
                !(is_blank && prev_blank),
                "two consecutive blank lines in reasoning→tool transition: {collected:?}"
            );
            prev_blank = is_blank;
        }
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
        use claudine::stream::{ParserConfig, create_semantic_parser};

        pub(super) fn replay_to_stderr(
            provider: Provider,
            fixture: &[&str],
            model: Option<String>,
        ) -> Vec<String> {
            replay_to_combined(provider, fixture, model)
                .into_iter()
                .filter_map(|(is_stdout, line)| (!is_stdout).then_some(line))
                .collect()
        }

        /// Replay fixture lines through the parser + [`LiveSemanticSink`]
        /// and capture every emission — both stderr status lines and
        /// stdout `OutputText` bytes — in the order they were written. The
        /// boolean is `true` when the emission was destined for stdout.
        ///
        /// This is the authoritative view of the sink's rendered output
        /// because the spec's spacing invariant is defined against the
        /// combined stream: "there are no two consecutive blank lines"
        /// across the whole rendered surface, not just stderr.
        pub(super) fn replay_to_combined(
            provider: Provider,
            fixture: &[&str],
            model: Option<String>,
        ) -> Vec<(bool, String)> {
            let captured: Arc<StdMutex<Vec<(bool, String)>>> = Arc::new(StdMutex::new(Vec::new()));
            let dispatched: Arc<StdMutex<Vec<(AgenticEvent, String)>>> =
                Arc::new(StdMutex::new(Vec::new()));

            let dispatch = {
                let cap = dispatched.clone();
                Box::new(move |ev: AgenticEvent, _meta: DispatchEventMeta| {
                    cap.lock().unwrap().push((ev, String::new()));
                })
                    as Box<dyn Fn(AgenticEvent, DispatchEventMeta) + Send + Sync + 'static>
            };
            let emit_stderr = {
                let cap = captured.clone();
                Box::new(move |line: &str| {
                    cap.lock().unwrap().push((false, line.to_string()));
                }) as Box<dyn Fn(&str) + Send + Sync + 'static>
            };
            let output_cb: OutputTextFn = {
                let cap = captured.clone();
                Box::new(move |text: &str| {
                    // Emit every embedded line separately so the combined
                    // view sees each stdout line as its own entry. Using
                    // `str::lines` drops the spurious trailing empty that
                    // `split('\n')` produces for newline-terminated text —
                    // that trailing empty is a chunk artifact, not a real
                    // blank line in the rendered output. Internal blank
                    // lines (`"a\n\nb"`) are preserved so the combined
                    // blank-line assertion can still catch real content
                    // problems that straddle the stdout surface.
                    for line in text.lines() {
                        cap.lock().unwrap().push((true, line.to_string()));
                    }
                })
            };

            let mut sink = LiveSemanticSink::new(
                provider,
                EnvironmentContext::default(),
                Path::new("/tmp"),
                Verbosity::Normal,
                Arc::new(Mutex::new(StructuredSummaryDetails::default())),
                dispatch,
                emit_stderr,
            )
            .with_output_text_sink(output_cb);

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
            let lines = replay_to_stderr(
                Provider::Claude,
                &[
                    r#"{"type":"init","session_id":"s1","model":"claude-sonnet-4"}"#,
                    r#"{"type":"tool_use","id":"t1","name":"bash","input":{"cmd":"ls -la"}}"#,
                    r#"{"type":"tool_result","tool_use_id":"t1","content":"file.txt"}"#,
                    r#"{"type":"task_started","task_id":"sa1","name":"researcher"}"#,
                    r#"{"type":"task_progress","message":"working"}"#,
                    r#"{"type":"task_completed","task_id":"sa1","name":"researcher","status":"success"}"#,
                    r#"{"type":"rate_limit_event","is_throttled":true,"retry_after_ms":5000,"message":"Rate limited"}"#,
                    r#"{"type":"some_future_event","x":1}"#,
                ],
                None,
            );
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
            let lines = replay_to_stderr(
                Provider::Codex,
                &[
                    r#"{"type":"thread.started","thread_id":"th-1"}"#,
                    r#"{"type":"item.started","item":{"id":"cmd1","type":"command_exec","tool_name":"bash","input":{"command":"ls"}}}"#,
                    r#"{"type":"item.completed","item":{"id":"cmd1","type":"command_exec","status":"success","exit_code":0,"output":"file.txt"}}"#,
                    r#"{"type":"item.completed","item":{"id":"f1","type":"file_change","path":"src/lib.rs","change_kind":"modified"}}"#,
                    r#"{"type":"item.completed","item":{"id":"p1","type":"plan_update","message":"Step 2"}}"#,
                    r#"{"type":"error","error_type":"rate_limit","error_message":"Too many requests"}"#,
                    r#"{"type":"future.unknown","payload":{"k":1}}"#,
                ],
                Some("codex-mini".into()),
            );
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
            let lines = replay_to_stderr(
                Provider::Gemini,
                &[
                    r#"{"type":"init","session_id":"g1","model":"gemini-2.5-pro"}"#,
                    r#"{"type":"tool_use","tool_id":"t1","tool_name":"search","parameters":{"q":"rust"}}"#,
                    r#"{"type":"tool_result","tool_id":"t1","status":"success","output":{"hits":3}}"#,
                    r#"{"type":"error","severity":"warning","message":"Loop detected"}"#,
                    r#"{"type":"some_unknown","data":"x"}"#,
                ],
                None,
            );
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
            let lines = replay_to_stderr(
                Provider::KimiCode,
                &[
                    r#"{"type":"init","session_id":"k1","model":"kimi-coder"}"#,
                    r#"{"type":"tool_use","id":"k1","name":"bash","input":{"cmd":"ls"}}"#,
                    r#"{"type":"tool_result","tool_use_id":"k1","status":"success","content":"ok"}"#,
                    r#"{"type":"error","error":{"type":"rate_limit","message":"slow down"}}"#,
                    r#"{"type":"future.unknown"}"#,
                ],
                None,
            );
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
            let lines = replay_to_stderr(
                Provider::OpenCode,
                &[
                    r#"{"type":"step_start","sessionID":"ses_1"}"#,
                    r#"{"type":"tool_start","part":{"id":"t1","tool_name":"bash","input":{"cmd":"ls"}}}"#,
                    r#"{"type":"tool_end","part":{"tool_use_id":"t1","status":"success","content":"ok"}}"#,
                    r#"{"type":"error","error_message":"API timeout"}"#,
                    r#"{"type":"some_future_event","x":1}"#,
                ],
                Some("gpt-4o".into()),
            );
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
            let lines = replay_to_stderr(
                Provider::OpenCode,
                &[
                    r#"{"type":"step_start","sessionID":"ses_1"}"#,
                    r#"{"type":"tool_use","part":{"id":"t1","tool":"bash",
                     "state":{"status":"completed","input":{"command":"ls -la"},"output":"file.txt"}}}"#,
                ],
                Some("gpt-4o".into()),
            );
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
            let lines = replay_to_stderr(
                Provider::QwenCode,
                &[
                    r#"{"type":"init","session_id":"q1","model":"qwen-coder"}"#,
                    r#"{"type":"tool_call","id":"q1","name":"bash","input":{"command":"git status"}}"#,
                    r#"{"type":"tool_response","tool_use_id":"q1","status":"success","content":"clean"}"#,
                    r#"{"type":"error","error":{"type":"rate_limit","message":"slow down"}}"#,
                    r#"{"type":"something.new"}"#,
                ],
                None,
            );
            assert!(!lines.is_empty());
            let joined = lines.join("\n");
            assert!(joined.contains('\u{2192}'), "expected →: {joined:?}");
            assert!(joined.contains('\u{2190}'), "expected ←: {joined:?}");
            assert!(joined.contains("Bash"));
            assert!(joined.contains("slow down"));
            assert!(joined.contains("qwen/something.new"));
        }

        #[test]
        #[serial_test::serial]
        fn captured_fixtures_have_no_two_consecutive_blank_lines_per_provider() {
            // Some replays (notably Claude rate-limit events) consult
            // ANTHROPIC_API_KEY; set it so any warnings render normally and
            // serialize with other env-touching tests.
            let _guard = super::TestEnvGuard::set("ANTHROPIC_API_KEY", "sk-test");

            let fixtures: &[(Provider, &str, Option<&str>)] = &[
                (Provider::Claude, "claude.ndjson", None),
                (Provider::Codex, "codex.ndjson", Some("codex-mini")),
                (Provider::Gemini, "gemini.ndjson", None),
                (Provider::OpenCode, "opencode.ndjson", Some("gpt-4o")),
            ];
            for (provider, fname, model) in fixtures {
                let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("..")
                    .join("lib")
                    .join("tests/fixtures/providers")
                    .join(fname);
                if !path.exists() {
                    eprintln!("skip: fixture not found at {path:?}");
                    continue;
                }
                let raw = std::fs::read_to_string(&path).expect("read fixture");
                let fixture_lines: Vec<&str> = raw.lines().collect();
                // Assert against the COMBINED stdout+stderr emission
                // stream — per spec, the spacing invariant is defined
                // over all rendered output in emission order, not just
                // stderr. A stderr-only assertion would miss consecutive
                // blanks that straddle the FinalStdout section boundary.
                let combined =
                    replay_to_combined(*provider, &fixture_lines, model.map(String::from));
                let mut prev_blank = false;
                for (is_stdout, line) in &combined {
                    let is_blank = line.trim().is_empty();
                    assert!(
                        !(is_blank && prev_blank),
                        "provider={provider:?} ({fname}): two consecutive blank lines in combined rendered output (is_stdout={is_stdout}):\n{combined:#?}"
                    );
                    prev_blank = is_blank;
                }
            }
        }

        /// Regression test for the duplicate-reasoning fix.
        ///
        /// Before the fix, reasoning was rendered twice on the structured-
        /// stream path:
        /// 1. `LiveSemanticSink` rendered it as a BlockQuote via
        ///    `render_thinking_block`.
        /// 2. `StreamThinkingRenderer` in `exec.rs` ALSO rendered it via
        ///    the `reasoning_cb` callback (emitting a dim "Thinking..."
        ///    header plus dimmed lines).
        ///
        /// After the fix, `LiveSemanticSink` owns reasoning rendering
        /// end-to-end. The `reasoning_cb` is a no-op. This test verifies:
        /// - Reasoning text appears in stderr exactly once.
        /// - The old "Thinking..." header from `StreamThinkingRenderer`
        ///   does NOT appear.
        #[test]
        fn reasoning_appears_exactly_once_in_stderr() {
            let lines: Arc<StdMutex<Vec<String>>> = Arc::new(StdMutex::new(Vec::new()));
            let dispatched: Arc<StdMutex<Vec<(AgenticEvent, String)>>> =
                Arc::new(StdMutex::new(Vec::new()));

            let dispatch = {
                let cap = dispatched.clone();
                Box::new(move |ev: AgenticEvent, _meta: DispatchEventMeta| {
                    cap.lock().unwrap().push((ev, String::new()));
                })
                    as Box<dyn Fn(AgenticEvent, DispatchEventMeta) + Send + Sync + 'static>
            };
            let emit = {
                let cap = lines.clone();
                Box::new(move |line: &str| {
                    cap.lock().unwrap().push(line.to_string());
                }) as Box<dyn Fn(&str) + Send + Sync + 'static>
            };

            let mut sink = LiveSemanticSink::new(
                Provider::Claude,
                EnvironmentContext::default(),
                Path::new("/tmp"),
                Verbosity::Normal,
                Arc::new(Mutex::new(StructuredSummaryDetails::default())),
                dispatch,
                emit,
            );

            // Feed a reasoning event directly (simulating what the parser
            // would emit after parsing a thinking content block).
            sink.on_semantic_event(SemanticEvent::Reasoning {
                text: "Let me analyze this step by step.".into(),
                extra: json!({}),
            });

            let captured = lines.lock().unwrap().join("\n");

            // Reasoning text must appear in stderr.
            assert!(
                captured.contains("analyze this step by step"),
                "reasoning text must appear in stderr: {captured:?}"
            );

            // The old StreamThinkingRenderer "Thinking..." header must NOT
            // appear — that was the duplicate-rendering artifact.
            assert!(
                !captured.contains("Thinking..."),
                "old StreamThinkingRenderer header must not appear: {captured:?}"
            );

            // Count occurrences of the reasoning text to verify it appears
            // exactly once (not duplicated).
            let count = captured.matches("analyze this step by step").count();
            assert_eq!(
                count, 1,
                "reasoning text must appear exactly once, found {count} times: {captured:?}"
            );
        }
    }
}
