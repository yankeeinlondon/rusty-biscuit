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
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};

use biscuit_terminal::terminal::Terminal;
use claudine::events::{AgenticEvent, EnvironmentContext, EventMeta as DispatchEventMeta};
use claudine::provider::{EventClass, Provider};
use claudine::render::{EventRenderer, ThinkingStream};
use claudine::runaway::ContentDetector;
use claudine::stream::logs::EarlyTermination;
use claudine::stream::progress::{self, LiveMetrics};
use claudine::stream::semantic::SemanticEvent;
use claudine::stream::stderr::Verbosity;
use serde_json::Value;

// Re-exports from the parent wrap module so submodules can reach them easily.
pub(crate) use super::StructuredSummaryDetails;
pub(crate) use super::section::{Section, SectionStream, SectionTracker};
pub(crate) use super::stream_io::StreamOutput;
pub(crate) use super::subagent_watchdog::WatchdogState;

// Submodules
mod event_sink;
mod heartbeat;
mod sections;
mod spacing;
mod thinking;

// Re-exports of the moved render helpers, kept at this path so the existing
// per-helper unit tests (which reach them via `super::`) compile unchanged.
// The implementations now live in the shared `claudine::render::EventRenderer`.
#[allow(unused_imports)]
pub(crate) use claudine::render::{
    error_kind_presentation, pending_matches_tool_call, strip_progress_verb,
};
// Re-exported for the test suites' `use super::*`; the production render path
// now handles error kinds inside `EventRenderer`.
#[allow(unused_imports)]
pub(crate) use claudine::stream::semantic::SemanticErrorKind;

/// Section that a rendered [`claudine::render::RenderUnit`] is emitted into,
/// keyed by its [`EventClass`]. Every status class routes to
/// [`Section::ToolUseAndEvents`]; `Thinking` and `FinalMessage` are reserved
/// for the streaming paths that stay sink-side (part 2) and are not produced
/// by [`EventRenderer::render`] this round.
pub(crate) fn section_for(class: EventClass) -> Section {
    match class {
        EventClass::Thinking => Section::Thinking,
        EventClass::FinalMessage => Section::FinalStdout,
        EventClass::ToolUse
        | EventClass::McpCall
        | EventClass::HookEvent
        | EventClass::StepProgress
        | EventClass::FileChange
        | EventClass::PlanUpdate
        | EventClass::SubagentActivity
        | EventClass::Error => Section::ToolUseAndEvents,
    }
}

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
/// `AssistantStream`-style renderer on stdout (the lib streaming component
/// the exec layer drives). This keeps the sink decoupled from the rendering
/// machinery while still letting `OutputText` events flow through the
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
    /// STDERR status renderer. Owns the render policy, the file-link cwd/home
    /// context, the `SessionStart` auth source, and the `task_progress` dedup
    /// buffer. The sink drives it per event and routes each returned
    /// [`claudine::render::RenderUnit`] through [`Self::emit_section_line`].
    renderer: EventRenderer,
    /// Immediate child PID captured after a successful provider spawn.
    ///
    /// `None` until the wrapper spawns the provider; the parser-builder
    /// closure sets it once spawn returns so every live dispatched/logged
    /// record carries `EventMeta.agent_pid`. `claudine_pid` rides along in
    /// `env` and needs no separate slot here.
    agent_pid: Option<u32>,
    verbosity: Verbosity,
    session_id: Option<String>,
    model: Option<String>,
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
    /// Buffers `Reasoning` deltas and flushes coalesced thinking blocks. A
    /// [`claudine::render::StreamRenderable`]: token-level Claude fragments and
    /// a single accumulated Kimi thought both render as one clean `▌ ` block by
    /// construction. Drained at the top of `on_semantic_event` before any
    /// non-`Reasoning` event and again in `Drop`. Owns its own `Terminal`
    /// clone; the (test-only) `terminal` mutation does not re-target it.
    ///
    /// No idle-flush ticker (deliberate — unlike the exec-layer
    /// [`AssistantStream`], which has [`super::super::exec::spawn_flush_if_idle_ticker`]).
    /// The next-event boundary drain covers every real thinking→output/tool
    /// transition and `Drop` covers stream end / timeout termination, so
    /// buffered thinking is never lost. What a ticker would add is *mid-stall*
    /// surfacing of the held buffer, and that buffer is bounded to ~one
    /// in-progress sentence (this stream flushes per line and early-flushes past
    /// 200 bytes on a sentence terminator) — versus the whole paragraph
    /// `AssistantStream` can hold, which is what justifies *its* ticker. Wiring
    /// one here would mean moving this field behind `Arc<Mutex<_>>`, threading
    /// the handle out of sink construction (`policy.rs`) down to the ticker, and
    /// adding a second concurrent stderr writer racing `emit_section_line`
    /// against the no-double-blank-line contract — disproportionate for a
    /// sub-sentence cosmetic. Revisit only if long silent mid-thought stalls
    /// holding large partials become a real complaint.
    thinking_stream: ThinkingStream,
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
    /// Runaway-output content detector (Phase 6). `None` when guards are
    /// fully opted out, in which case `on_semantic_event` does no
    /// detection work. Fed from `OutputText`/`Reasoning` text only (never
    /// tool payloads — A2) and reset per turn on `TurnComplete`.
    content_detector: Option<ContentDetector>,
    /// Validated guard config kept so the in-scope exit-expression set can
    /// be re-filtered when a provider reports its actual model via
    /// `SessionStart`. `None` when no re-scope source was wired (e.g. unit
    /// tests that arm a detector directly). See
    /// [`Self::set_guard_rescope_source`].
    guard_inputs: Option<Arc<super::runaway_guard::ResolvedGuardInputs>>,
    /// Model the currently-compiled exit-expression set was filtered for
    /// (the launch-time hint, possibly empty). A `SessionStart` model that
    /// differs from this triggers a re-scope; an identical one is a no-op.
    detector_scope_model: Option<String>,
    /// Trip sender wired to the unified early-termination channel the
    /// exec wait loop polls. Set by `build_structured_plumbing` so the
    /// detector and (for OpenCode) the stderr bridge share one channel.
    trip_sender: Option<Sender<EarlyTermination>>,
    /// Set once a content trip has fired. Guards against a double-send (a
    /// trip is terminal) and suppresses further output rendering so the
    /// tail of a runaway is not echoed to the terminal (spec Part 1).
    content_tripped: bool,
    /// Best-effort mid-session status reporter to the rendezvous
    /// dashboard's `sessions-active` register (trigger 1). Inert unless
    /// a session was bracketed against a live daemon.
    status_reporter: super::session_report::StatusReporter,
    /// Whether the last observed transition left the session waiting on
    /// the user (a `PermissionRequest` not yet followed by progress).
    /// Debounces the `waiting_on_user` → `active` status reports so only
    /// real edges hit the daemon.
    awaiting_user: bool,
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
        let terminal = wrap_terminal();
        Self {
            provider,
            env,
            cwd: cwd.to_path_buf(),
            renderer: EventRenderer::new(provider, cwd.to_path_buf(), dirs::home_dir()),
            agent_pid: None,
            verbosity,
            session_id: None,
            model: None,
            start_emitted: false,
            summary_details,
            context_extra: HashMap::new(),
            dispatch,
            emit_stderr,
            emit_output_text: None,
            emit_event_log: None,
            live_metrics: progress::new_live_metrics(),
            stream_output: StreamOutput::shared(),
            thinking_stream: ThinkingStream::new(terminal.clone()),
            terminal,
            section_tracker: Arc::new(Mutex::new(SectionTracker::new())),
            at_blank_row: false,
            stdout_trailing_newlines: 0,
            watchdog_state: Arc::new(Mutex::new(WatchdogState::default())),
            content_detector: None,
            guard_inputs: None,
            detector_scope_model: None,
            trip_sender: None,
            content_tripped: false,
            status_reporter: super::session_report::StatusReporter::inert(),
            awaiting_user: false,
        }
    }

    /// Wire the dashboard status reporter (trigger 1). Defaults to inert.
    pub(crate) fn with_status_reporter(
        mut self,
        reporter: super::session_report::StatusReporter,
    ) -> Self {
        self.status_reporter = reporter;
        self
    }

    /// Convenience constructor for the wrapped-provider call sites
    /// wrapped-provider call sites in `wrap/mod.rs`. Builds a Tokio-based
    /// dispatch closure, an emit_stderr closure backed by
    /// [`StreamOutput::emit_stderr_line`], and a best-effort JSONL logger
    /// pointing at [`claudine::stream::reporting::write_summary_event`].
    ///
    /// `task_gutter` is a sequence task's rendered bar. When present every
    /// status and reasoning line this sink emits carries it, because the sink's
    /// coordinator — and therefore the section stream, the watchdog tickers, and
    /// the timing monitor that are handed it — is the decorated handle.
    pub(crate) fn with_default_wiring(
        provider: Provider,
        env: EnvironmentContext,
        cwd: &Path,
        verbosity: Verbosity,
        summary_details: Arc<Mutex<StructuredSummaryDetails>>,
        task_gutter: Option<String>,
    ) -> Self {
        let handle = tokio::runtime::Handle::try_current().ok();
        let runtime_context = match claudine::dispatch::DispatchRuntimeContext::load_for_env(&env) {
            Ok(runtime) => runtime,
            Err(error) => {
                tracing::warn!(%provider, "failed to preload wrapper runtime config: {error}");
                claudine::dispatch::DispatchRuntimeContext::default()
            }
        };
        let stream_output = match task_gutter {
            Some(gutter) => StreamOutput::shared().decorated(gutter),
            None => StreamOutput::shared(),
        };

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

        let terminal = wrap_terminal();
        Self {
            provider,
            env,
            cwd: cwd.to_path_buf(),
            renderer: EventRenderer::new(provider, cwd.to_path_buf(), dirs::home_dir()),
            agent_pid: None,
            verbosity,
            session_id: None,
            model: None,
            start_emitted: false,
            summary_details,
            context_extra: HashMap::new(),
            dispatch,
            emit_stderr,
            emit_output_text: None,
            emit_event_log: Some(event_logger),
            live_metrics: progress::new_live_metrics(),
            stream_output,
            thinking_stream: ThinkingStream::new(terminal.clone()),
            terminal,
            section_tracker: Arc::new(Mutex::new(SectionTracker::new())),
            at_blank_row: false,
            stdout_trailing_newlines: 0,
            watchdog_state: Arc::new(Mutex::new(WatchdogState::default())),
            content_detector: None,
            guard_inputs: None,
            detector_scope_model: None,
            trip_sender: None,
            content_tripped: false,
            status_reporter: super::session_report::StatusReporter::inert(),
            awaiting_user: false,
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
    /// exec-driven `AssistantStream` so the markdown-boundary logic
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
    /// this setter so the exec-layer `AssistantStream` stays in the stdout
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

    /// Arm the runaway-output content detector (Phase 6). Called by the
    /// streaming wiring point before the sink is handed to
    /// `build_structured_plumbing`. A `None` argument leaves the sink with
    /// no detector (zero per-event overhead).
    pub(crate) fn set_content_detector(&mut self, detector: Option<ContentDetector>) {
        self.content_detector = detector;
    }

    /// Whether a content detector is armed. `build_structured_plumbing`
    /// uses this to decide whether to create + wire the trip channel.
    pub(crate) fn has_content_detector(&self) -> bool {
        self.content_detector.is_some()
    }

    /// Wire the source needed to re-scope the exit-expression set when a
    /// provider reports its actual model.
    ///
    /// `inputs` is the validated, model-independent guard config (the
    /// expensive half of resolution, already run once before streaming);
    /// `launch_model` is the launch-time model hint the currently-armed
    /// detector was filtered for (CLI `--model` / `MODEL` env / frontmatter
    /// `model`, possibly absent → `None`).
    ///
    /// When the active detector was filtered with no launch-time model hint
    /// (`""`), an agent/model-scoped exit expression cannot match yet. If
    /// the provider later reports a model via `SessionStart` that brings
    /// such an expression into scope, [`Self::rescope_for_model`] re-filters
    /// and recompiles the in-scope set and swaps it into the live detector
    /// without losing volume / repetition state.
    pub(crate) fn set_guard_rescope_source(
        &mut self,
        inputs: Arc<super::runaway_guard::ResolvedGuardInputs>,
        launch_model: Option<&str>,
    ) {
        self.guard_inputs = Some(inputs);
        self.detector_scope_model = Some(launch_model.unwrap_or("").to_string());
    }

    /// Whether the run wants a content-detector trip channel wired.
    ///
    /// True when a detector is already armed, OR when a re-scope source
    /// carries an exit expression that could come into scope for this
    /// provider under a model the provider has not yet reported. The latter
    /// case matters because the launch-time compile can legitimately produce
    /// **no** detector — every other guard off and the only exit expression
    /// out of scope for the launch-time model — yet a `SessionStart` model
    /// can still bring that expression into scope. `build_structured_plumbing`
    /// uses this (not [`Self::has_content_detector`]) to decide whether to
    /// own the trip channel so a re-scope-armed detector has somewhere to
    /// send.
    pub(crate) fn wants_content_channel(&self) -> bool {
        self.content_detector.is_some()
            || self
                .guard_inputs
                .as_ref()
                .is_some_and(|inputs| inputs.has_provider_scoped_entries())
    }

    /// Re-scope the content detector's exit-expression set for a
    /// provider-reported model, when it differs from the model the current
    /// set was filtered for.
    ///
    /// A no-op when no re-scope source is wired or the reported model matches
    /// the model already in force (so a `SessionStart` echoing the
    /// launch-time model never rebuilds the set or drops state). When a
    /// detector is already armed, only its compiled pattern set is swapped,
    /// preserving the volume / repetition state. When the launch-time compile
    /// produced no detector (every other guard off and the only exit
    /// expression out of scope), a fresh detector is built and the existing
    /// trip sender is re-attached so the newly in-scope expression can fire.
    ///
    /// The recompile stays fail-closed: re-filtering an already-validated
    /// entry set should never produce a compile error, but if it somehow
    /// does, the error is logged and the previous set is left in force rather
    /// than silently disabling all exit expressions.
    fn rescope_for_model(&mut self, reported_model: Option<&str>) {
        let Some(inputs) = self.guard_inputs.clone() else {
            return;
        };
        let reported = reported_model.unwrap_or("");
        if self.detector_scope_model.as_deref() == Some(reported) {
            return;
        }

        if self.content_detector.is_some() {
            // Hot path: a detector already exists, so swap only the compiled
            // pattern set and keep its accumulated state.
            match inputs.compile_patterns_for_model(reported_model) {
                Ok(compiled) => {
                    if let Some(detector) = self.content_detector.as_mut() {
                        detector.set_exit_expressions(compiled);
                    }
                    self.detector_scope_model = Some(reported.to_string());
                }
                Err(error) => Self::warn_rescope_failed(inputs.provider(), reported, &error),
            }
            return;
        }

        // No detector yet (launch-time set was empty and other guards off).
        // Rebuild a full detector for the reported model; it is `None` again
        // only if the reported model also brings nothing into scope.
        match inputs.compile_for_model(reported_model) {
            Ok(resolved) => {
                // The trip sender lives on the sink (already wired by
                // `build_structured_plumbing` because `wants_content_channel`
                // reported `true`), so installing the detector is enough for
                // it to fire.
                self.content_detector = resolved.detector;
                self.detector_scope_model = Some(reported.to_string());
            }
            Err(error) => Self::warn_rescope_failed(inputs.provider(), reported, &error),
        }
    }

    fn warn_rescope_failed(provider: Provider, reported_model: &str, error: &claudine::error::ClaudineError) {
        tracing::warn!(
            %provider,
            reported_model,
            "failed to re-scope exit expressions for reported model: {error}; \
             keeping the previous set"
        );
    }

    /// Wire the detector's trip sender to the unified early-termination
    /// channel the exec wait loop polls. Set by `build_structured_plumbing`
    /// (which owns the channel) after the detector is armed.
    pub(crate) fn set_trip_sender(&mut self, sender: Sender<EarlyTermination>) {
        self.trip_sender = Some(sender);
    }

    /// Feed assistant text to the content detector, firing a trip on the
    /// first guard breach. No-op when no detector is armed or a trip has
    /// already fired. Returns `true` when this call fired the trip so the
    /// caller can suppress rendering of the tripping chunk.
    fn feed_content_detector(&mut self, text: &str) -> bool {
        if self.content_tripped {
            return false;
        }
        let Some(detector) = self.content_detector.as_mut() else {
            return false;
        };
        if let Some(trip) = detector.feed(text) {
            self.fire_content_trip(trip);
            return true;
        }
        false
    }

    /// Reset the detector's per-turn volume counters (`TurnComplete`).
    fn reset_content_detector_turn(&mut self) {
        if let Some(detector) = self.content_detector.as_mut() {
            detector.reset_turn();
        }
    }

    /// Send a content trip on the unified termination channel exactly once.
    ///
    /// A trip is terminal: the `content_tripped` flag guards against a
    /// double-send and also suppresses further output rendering. The send
    /// is best-effort — if the receiver has already hung up (the wait loop
    /// killed the child), dropping the signal is fine.
    fn fire_content_trip(&mut self, trip: claudine::runaway::Trip) {
        if self.content_tripped {
            return;
        }
        self.content_tripped = true;
        if let Some(sender) = self.trip_sender.as_ref() {
            let early = super::exec::termination::trip_to_early_termination(trip);
            let _ = sender.send(early);
        }
    }

    /// Whether a content trip has fired (rendering is then suppressed).
    fn content_tripped(&self) -> bool {
        self.content_tripped
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
        // Drain any buffered reasoning first so a lone trailing thought (a
        // `Reasoning` event with no following non-`Reasoning` event to trigger
        // the boundary flush) still renders at stream end.
        self.flush_pending_thinking();
        // Any `task_progress` Info still buffered at stream end was never
        // matched against a follow-up tool call. Flush it so the
        // narration is not lost when the sink is dropped mid-turn.
        let units = self.renderer.flush(&self.terminal);
        for unit in units {
            self.emit_section_line(section_for(unit.class), &unit.text);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // The `SemanticEventSink` trait is no longer imported by the production
    // module (the impl moved to `event_sink`), so bring it back into scope
    // here for the `on_semantic_event` driver calls in the child suites.
    use claudine::stream::semantic::SemanticEventSink;

    mod content_guard;
    mod dispatch_and_recording;
    mod golden_stderr;
    mod provider_extension_and_opencode;
    mod render_basics;
    mod sections_and_output;
}
