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

use biscuit_terminal::terminal::Terminal;
use claudine::events::{AgenticEvent, EnvironmentContext, EventMeta as DispatchEventMeta};
use claudine::provider::Provider;
use claudine::stream::progress::{self, LiveMetrics};
use claudine::stream::semantic::{SemanticErrorKind, SemanticEvent};
use claudine::stream::stderr::Verbosity;
use serde_json::Value;

// Re-exports from the parent wrap module so submodules can reach them easily.
pub(crate) use super::StructuredSummaryDetails;
pub(crate) use super::section::{Section, SectionStream, SectionTracker};
pub(crate) use super::stream_io::StreamOutput;
pub(crate) use super::subagent_watchdog::WatchdogState;

// Submodules
mod errors;
mod event_sink;
mod heartbeat;
mod provider_extension;
mod render_event;
mod sections;
mod spacing;
mod thinking;
mod tool_calls;

// Re-exports from submodules to preserve the API surface for tests and callers.
#[allow(unused_imports)]
pub(crate) use errors::error_kind_presentation;
// Re-exported so `render_event` can reach the provider-extension classifiers
// and payload summarizer through the parent module via `super::`.
pub(crate) use provider_extension::{
    is_claude_task_progress, is_silent_extension_kind, is_suppressed_claude_rate_limit,
    provider_short, summarize_provider_payload,
};
#[allow(unused_imports)]
pub(crate) use tool_calls::{
    pending_matches_tool_call, strip_progress_verb, tool_result_body, tool_result_output_text,
};

/// Borrow-friendly terminal used for status rendering. Mirrors the helper in
/// `wrap/mod.rs` so both sinks render against the same capabilities.
fn wrap_terminal() -> Terminal {
    crate::log::terminal()
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

const TOOL_RESULT_BODY_MAX_LINES: usize = 10;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ToolResultBody {
    pub(crate) text: String,
    pub(crate) truncated: bool,
}

pub(crate) struct LiveSemanticSink {
    provider: Provider,
    env: EnvironmentContext,
    cwd: PathBuf,
    home: Option<PathBuf>,
    /// Immediate child PID captured after a successful provider spawn.
    ///
    /// `None` until the wrapper spawns the provider; the parser-builder
    /// closure sets it once spawn returns so every live dispatched/logged
    /// record carries `EventMeta.agent_pid`. `claudine_pid` rides along in
    /// `env` and needs no separate slot here.
    agent_pid: Option<u32>,
    verbosity: Verbosity,
    pending_task_progress: Option<String>,
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
    /// Unified "visual blank row" state that tracks whether the combined
    /// output (stdout + stderr) is currently at a visually blank row.
    ///
    /// Set to `true` when:
    /// - A blank stderr line is written (section separator or explicit blank)
    /// - Stdout text ending with two or more consecutive `\n` is forwarded
    ///
    /// Set to `false` when:
    /// - A non-blank stderr line is written via `emit_section_line`
    /// - Stdout text containing non-newline bytes is forwarded
    ///
    /// When `true`, the automatic section-transition separator is suppressed
    /// because injecting another blank line would produce visually consecutive
    /// blank lines in the combined output.
    at_blank_row: bool,
    /// Running count of consecutive trailing `\n` bytes in the assistant
    /// text forwarded to the stdout renderer. Used by `at_blank_row` to
    /// detect when stdout has ended on a visual blank line (>= 2 trailing
    /// newlines). Extended by text events that are entirely newlines, reset
    /// to the text's own trailing newline count when the text contains
    /// other bytes, and cleared to `0` whenever the sink emits non-blank
    /// stderr content.
    stdout_trailing_newlines: usize,
    /// Shared subagent watchdog state. Updated from the semantic event
    /// stream so the exec-layer ticker can detect stuck subagents and
    /// request termination. The same `Arc<Mutex<_>>` is shared with
    /// `exec.rs` via [`Self::watchdog_state`].
    watchdog_state: Arc<Mutex<WatchdogState>>,
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
            home: dirs::home_dir(),
            agent_pid: None,
            verbosity,
            pending_task_progress: None,
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
            at_blank_row: false,
            stdout_trailing_newlines: 0,
            watchdog_state: Arc::new(Mutex::new(WatchdogState::default())),
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
            home: dirs::home_dir(),
            agent_pid: None,
            verbosity,
            pending_task_progress: None,
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
            at_blank_row: false,
            stdout_trailing_newlines: 0,
            watchdog_state: Arc::new(Mutex::new(WatchdogState::default())),
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

    /// Record the spawned provider's immediate child PID.
    ///
    /// Called by the parser-builder closure after a successful spawn so that
    /// the live dispatched and logged records carry `EventMeta.agent_pid`.
    pub(crate) fn set_agent_pid(&mut self, agent_pid: Option<u32>) {
        self.agent_pid = agent_pid;
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

    /// Return a clone of the shared [`WatchdogState`] handle so the exec
    /// layer and heartbeat threads can inspect active subagents without
    /// holding the sink mutex during writes.
    pub(crate) fn watchdog_state(&self) -> Arc<Mutex<WatchdogState>> {
        self.watchdog_state.clone()
    }

    fn should_render(&self) -> bool {
        self.verbosity != Verbosity::Silent
    }

    fn emit_line(&self, line: &str) {
        (self.emit_stderr)(line);
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
        // populated by `semantic_event_to_event_meta` alone. `agent_pid` is
        // known only after spawn, so the sink stamps it here rather than the
        // shared helper (which always emits `None`).
        DispatchEventMeta {
            event: agentic,
            cwd: Some(self.cwd.display().to_string()),
            agent_pid: self.agent_pid,
            ..meta
        }
    }
}

impl Drop for LiveSemanticSink {
    fn drop(&mut self) {
        // Any `task_progress` Info still buffered at stream end was never
        // matched against a follow-up tool call. Flush it so the
        // narration is not lost when the sink is dropped mid-turn.
        if self.pending_task_progress.is_some() {
            self.flush_pending_task_progress();
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

#[cfg(test)]
mod tests {
    use super::*;
    // The `SemanticEventSink` trait is no longer imported by the production
    // module (the impl moved to `event_sink`), so bring it back into scope
    // here for the `on_semantic_event` driver calls in the child suites.
    use claudine::stream::semantic::SemanticEventSink;

    mod dispatch_and_recording;
    mod golden_stderr;
    mod provider_extension_and_opencode;
    mod render_basics;
    mod sections_and_output;
}
