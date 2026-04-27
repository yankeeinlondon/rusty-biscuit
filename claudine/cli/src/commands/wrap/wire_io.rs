//! Kimi `--wire` JSON-RPC 2.0 line transport.
//!
//! Wire mode is Kimi's structured client protocol. Stdout carries one
//! JSON-RPC envelope per line (notification, server-initiated request, or
//! response to a previously-sent client request); stdin accepts the same
//! shape on a single serialized writer. The semantic parser
//! ([`claudine::stream::kimi_semantic::KimiSemanticStreamParser`]) consumes
//! every stdout line and emits the user-visible event surface, while this
//! module owns the IO loop that:
//!
//! - sends `initialize` and validates the negotiated protocol version,
//! - sends the resolved prompt as a `prompt` request after initialize,
//! - auto-responds to server-initiated `ApprovalRequest`, `QuestionRequest`,
//!   `ToolCallRequest`, and routes `HookRequest` through Claudine's
//!   dispatch pipeline,
//! - sends `cancel` on Ctrl+C / wall-clock timeout,
//! - flushes after every line so Kimi sees the response before the next
//!   stdin read.
//!
//! The pure builder helpers ([`build_initialize_request`],
//! [`build_prompt_request`], [`build_cancel_request`],
//! [`build_approval_response`], [`build_question_response`],
//! [`build_tool_call_unsupported_error`], [`build_hook_response`]) return
//! `serde_json::Value` envelopes so they can be unit-tested without any
//! child process. [`WireWriter`] serializes them to the child's stdin
//! behind a `Mutex` and flushes after each newline, satisfying the
//! "one serialized writer path" requirement from the Phase 3 plan.

#![allow(dead_code)]

use std::collections::HashMap;
use std::ffi::OsString;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use claudine::events::{AgenticEvent, EnvironmentContext, EventMeta, Provider};
use claudine::stream::parser::{SemanticStreamParser, StreamParseError};
use claudine::stream::progress::LiveMetrics;
use claudine::stream::protocol::kimi::{
    KimiEnvelope, KimiHookRequest, KimiInitializeResult, KimiJsonRpcError, KimiWireRequest,
};
use claudine::stream::summary::StreamExecutionSummary;
use color_eyre::eyre::Result;
use serde_json::{Value, json};
use tracing::{Span, debug, info, info_span, warn};

use super::exec::{
    OutputTextCallback, ProcessResult, ProcessTelemetry, ReasoningCallback, SemanticParserBuilder,
};
use super::stream_io::StreamOutput;

// ---------------------------------------------------------------------------
// Stable identifiers and protocol contract constants
// ---------------------------------------------------------------------------

/// Wire protocol version Claudine negotiates with Kimi. Phase 0 evidence
/// confirmed `1.9` is the minimum revision the live `kimi 1.38.0` build
/// accepts. The parser keys initialize-response handling on this id.
pub(crate) const WIRE_PROTOCOL_VERSION: &str = "1.9";

/// JSON-RPC id used for the `initialize` request. Matches the literal id
/// the Kimi semantic parser routes through `handle_initialize_response`.
pub(crate) const INITIALIZE_REQUEST_ID: &str = "init-1";

/// JSON-RPC id used for the `prompt` request that delivers the resolved
/// prompt body. Matches the literal id the Kimi semantic parser routes
/// through `handle_prompt_response`.
pub(crate) const PROMPT_REQUEST_ID: &str = "prompt-2";

/// JSON-RPC id used for the `cancel` request emitted by Ctrl+C or
/// timeout-driven cancellation. Distinct from the prompt id so the
/// originating prompt can still surface `result.status == "cancelled"`.
pub(crate) const CANCEL_REQUEST_ID: &str = "cancel-3";

/// Capabilities Claudine advertises to Kimi during initialize. Per Phase 0
/// findings the server only inspects `supports_question` and
/// `supports_plan_mode`; the other fields named in the Phase 3 plan
/// (`approvals`, `hooks`, `subagents`) are silently ignored but are still
/// declared so future protocol revisions see the explicit capability set.
#[derive(Debug, Clone, Copy)]
pub(crate) struct WireClientCapabilities {
    pub approvals: bool,
    pub questions: bool,
    pub hooks: bool,
    pub subagents: bool,
    pub plan_mode: bool,
}

impl WireClientCapabilities {
    /// Capability set the Phase 3 plan specifies for non-interactive
    /// Claudine-managed Kimi runs.
    pub(crate) const fn default_for_claudine() -> Self {
        Self {
            approvals: true,
            questions: false,
            hooks: true,
            subagents: true,
            plan_mode: false,
        }
    }
}

impl Default for WireClientCapabilities {
    fn default() -> Self {
        Self::default_for_claudine()
    }
}

// ---------------------------------------------------------------------------
// JSON-RPC line builders (pure functions, fully unit-testable)
// ---------------------------------------------------------------------------

/// Build a JSON-RPC `initialize` request body.
///
/// `client_name` and `client_version` populate `params.client`; the server
/// echoes them on its `KimiInitializeResult` for telemetry.
pub(crate) fn build_initialize_request(
    client_name: &str,
    client_version: &str,
    capabilities: WireClientCapabilities,
) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": INITIALIZE_REQUEST_ID,
        "method": "initialize",
        "params": {
            "protocol_version": WIRE_PROTOCOL_VERSION,
            "client": {
                "name": client_name,
                "version": client_version,
            },
            "capabilities": {
                "approvals": capabilities.approvals,
                "supports_question": capabilities.questions,
                "hooks": capabilities.hooks,
                "subagents": capabilities.subagents,
                "supports_plan_mode": capabilities.plan_mode,
            },
        },
    })
}

/// Build a JSON-RPC `prompt` request body.
///
/// Kimi's wire protocol expects `params.user_input` to carry either a
/// plain `String` or a `[ContentPart]` array. Claudine sends a plain
/// string for v1 — multi-modal prompt input is out of scope.
pub(crate) fn build_prompt_request(prompt: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": PROMPT_REQUEST_ID,
        "method": "prompt",
        "params": {
            "user_input": prompt,
        },
    })
}

/// Build a JSON-RPC `cancel` request body. Kimi replies with `{result: {}}`
/// and emits a `TurnEnd` plus a `prompt` response with `status: "cancelled"`.
pub(crate) fn build_cancel_request() -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": CANCEL_REQUEST_ID,
        "method": "cancel",
        "params": {},
    })
}

/// Build a successful response to an `ApprovalRequest`. Claudine
/// auto-approves all approvals in v1 — the visible `auto_approved` Info
/// event is emitted by the semantic parser, not this builder.
pub(crate) fn build_approval_response(request_id: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": request_id,
        "result": {
            "response": "approve",
        },
    })
}

/// Build a synthetic empty answer to an unexpected `QuestionRequest`.
/// Kimi should not send this when Claudine declares
/// `supports_question: false`, but if it does we return an empty answer
/// so the agent can continue rather than block.
pub(crate) fn build_question_response(request_id: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": request_id,
        "result": {
            "answer": "",
        },
    })
}

/// Build a JSON-RPC `METHOD_NOT_FOUND` error response for unsupported
/// `ToolCallRequest` envelopes. Claudine does not expose external tool
/// execution to Kimi in v1; the visible warning is emitted by the
/// semantic parser.
pub(crate) fn build_tool_call_unsupported_error(request_id: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": request_id,
        "error": {
            "code": KimiJsonRpcError::METHOD_NOT_FOUND,
            "message": "External tool-call execution is not supported by this Claudine client",
        },
    })
}

/// Build a hook response from a Claudine [`HookOutcome`]. Outcomes that
/// allow the action map to `decision: "approve"`, denials to
/// `decision: "reject"`, and `Ask` to `decision: "ask"`. The provider
/// adapter's `format_response` shape is reused for compatibility with the
/// existing Kimi adapter contract.
pub(crate) fn build_hook_response(request_id: Value, outcome: HookOutcome) -> Value {
    let (decision, reason) = match outcome {
        HookOutcome::Allow { reason } => ("approve", reason),
        HookOutcome::Deny { reason } => ("reject", reason),
        HookOutcome::Ask { reason } => ("ask", reason),
    };
    let mut body = json!({ "decision": decision });
    if let Some(reason) = reason {
        body["reason"] = Value::String(reason);
    }
    json!({
        "jsonrpc": "2.0",
        "id": request_id,
        "result": body,
    })
}

/// Hook dispatch outcome surfaced from Claudine's canonical dispatch
/// pipeline. Mirrors [`claudine::actions::HookDecision`] but stays
/// CLI-local so the wire-mode response builder doesn't depend on the
/// dispatch internals.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum HookOutcome {
    Allow { reason: Option<String> },
    Deny { reason: Option<String> },
    Ask { reason: Option<String> },
}

impl HookOutcome {
    /// Default outcome used when no Claudine config is loaded or when the
    /// dispatch pipeline returns no explicit decision.
    pub(crate) fn allow_default() -> Self {
        Self::Allow { reason: None }
    }
}

// ---------------------------------------------------------------------------
// Serialized writer
// ---------------------------------------------------------------------------

/// Single serialized writer over the child's stdin.
///
/// Wraps `ChildStdin` behind a `Mutex` so multiple sender call sites
/// (main-thread `send_initialize`, reader-thread auto-response handlers,
/// signal-handler cancel path) cannot interleave bytes in the middle of a
/// JSON-RPC line. Every `send_value` call serializes the value, appends
/// `\n`, and flushes immediately.
#[derive(Clone)]
pub(crate) struct WireWriter {
    inner: Arc<Mutex<Box<dyn Write + Send>>>,
}

impl WireWriter {
    /// Wrap the child's stdin pipe.
    pub(crate) fn from_child_stdin(stdin: ChildStdin) -> Self {
        Self::from_writer(Box::new(stdin))
    }

    /// Wrap an arbitrary `Write` impl. Used by tests to capture emitted
    /// JSON-RPC lines without spawning a process.
    pub(crate) fn from_writer(writer: Box<dyn Write + Send>) -> Self {
        Self {
            inner: Arc::new(Mutex::new(writer)),
        }
    }

    /// Serialize `value`, append `\n`, write to the inner writer, and
    /// flush. Returns the serialized bytes (sans newline) on success so
    /// tracing spans can record the body.
    pub(crate) fn send_value(&self, value: &Value) -> std::io::Result<String> {
        let serialized = serde_json::to_string(value)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        let mut guard = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        guard.write_all(serialized.as_bytes())?;
        guard.write_all(b"\n")?;
        guard.flush()?;
        Ok(serialized)
    }
}

// ---------------------------------------------------------------------------
// Server request auto-response routing
// ---------------------------------------------------------------------------

/// Outcome of dispatching a server-initiated request through the wire
/// auto-response pipeline. The reader thread uses this to decide which
/// response builder to invoke and which trace span to emit.
#[derive(Debug, Clone)]
pub(crate) enum WireRequestDispatch {
    /// Approve a tool action and return the auto-approval response.
    AutoApprove,
    /// Return a synthetic empty answer to an unexpected question.
    EmptyQuestionAnswer,
    /// Return a `METHOD_NOT_FOUND` error for unsupported external tool
    /// calls.
    UnsupportedToolCall,
    /// Hook request — caller should route through Claudine dispatch and
    /// return the negotiated decision.
    HookRequest(KimiHookRequest),
}

/// Classify a typed [`KimiWireRequest`] into a wire-level dispatch
/// outcome the reader thread can act on without re-parsing the envelope.
pub(crate) fn dispatch_for_request(request: &KimiWireRequest) -> WireRequestDispatch {
    match request {
        KimiWireRequest::Approval(_) => WireRequestDispatch::AutoApprove,
        KimiWireRequest::Question(_) => WireRequestDispatch::EmptyQuestionAnswer,
        KimiWireRequest::ToolCall(_) => WireRequestDispatch::UnsupportedToolCall,
        KimiWireRequest::Hook(hook) => WireRequestDispatch::HookRequest(hook.clone()),
    }
}

/// Map a Kimi-canonical hook event name to Claudine's
/// [`AgenticEvent`]. Returns `None` for unknown event names; the caller
/// responds with `Allow` so the agent doesn't deadlock waiting on an
/// unrecognized hook.
pub(crate) fn map_kimi_hook_event(event: &str) -> Option<AgenticEvent> {
    let canonical = match event {
        "PreToolUse" | "BeforeTool" => AgenticEvent::BeforeTool,
        "PostToolUse" | "AfterTool" => AgenticEvent::AfterTool,
        "Stop" | "TurnComplete" => AgenticEvent::TurnComplete,
        "TurnError" | "Error" => AgenticEvent::TurnError,
        "BeforePrompt" | "TurnBegin" | "UserPromptSubmit" => AgenticEvent::BeforePrompt,
        "Notification" => AgenticEvent::Notification,
        "SessionStart" => AgenticEvent::SessionStart,
        "SessionEnd" => AgenticEvent::SessionEnd,
        "PermissionRequest" | "ApprovalRequest" => AgenticEvent::PermissionRequest,
        "BeforeCompact" | "CompactionBegin" => AgenticEvent::BeforeCompact,
        "AfterModel" | "ContentPart" => AgenticEvent::AfterModel,
        "SubagentStart" => AgenticEvent::SubagentStart,
        "SubagentStop" => AgenticEvent::SubagentStop,
        _ => return None,
    };
    Some(canonical)
}

// ---------------------------------------------------------------------------
// Hook dispatch wiring
// ---------------------------------------------------------------------------

/// Dispatch a Kimi `HookRequest` through Claudine's canonical pipeline
/// and return the negotiated [`HookOutcome`].
///
/// Falls back to [`HookOutcome::allow_default`] when no Claudine config is
/// loaded, when the runtime resolution fails, when the event name is
/// unknown, or when no async runtime handle is available. Errors are
/// logged via tracing rather than propagated so a misconfigured hook
/// never deadlocks the agent.
pub(crate) fn dispatch_hook_request(
    request: &KimiHookRequest,
    runtime_handle: Option<&tokio::runtime::Handle>,
    runtime_context: &claudine::dispatch::DispatchRuntimeContext,
) -> HookOutcome {
    let Some(event_name) = request.event.as_deref() else {
        return HookOutcome::allow_default();
    };
    let Some(canonical_event) = map_kimi_hook_event(event_name) else {
        debug!(
            provider = "kimi",
            event = %event_name,
            "unknown Kimi hook event name; defaulting to allow"
        );
        return HookOutcome::allow_default();
    };
    let Some(handle) = runtime_handle else {
        debug!(
            provider = "kimi",
            event = %event_name,
            "no async runtime handle for hook dispatch; defaulting to allow"
        );
        return HookOutcome::allow_default();
    };

    let mut meta = EventMeta::new(Provider::KimiCode, canonical_event);
    if let Some(context) = request.context.as_ref()
        && let Some(map) = context.as_object()
    {
        for (key, value) in map {
            meta.extra.insert(key.clone(), value.clone());
        }
        if let Some(tool_name) = map.get("tool_name").and_then(Value::as_str) {
            meta.tool_name = Some(tool_name.to_string());
        }
        if let Some(tool_input) = map.get("tool_input") {
            meta.tool_input = Some(tool_input.clone());
        }
        if let Some(tool_response) = map.get("tool_response") {
            meta.tool_response = Some(tool_response.clone());
        }
        if let Some(session) = map.get("session_id").and_then(Value::as_str) {
            meta.session_id = Some(session.to_string());
        }
    }

    let dispatch_result = handle.block_on(claudine::dispatch::dispatch_event_meta_with_runtime(
        Provider::KimiCode,
        canonical_event,
        meta,
        runtime_context,
    ));

    match dispatch_result {
        Ok(outcome) => outcome_to_hook_outcome(&outcome),
        Err(error) => {
            warn!(
                provider = "kimi",
                event = %event_name,
                error = %error,
                "hook dispatch failed; defaulting to allow"
            );
            HookOutcome::allow_default()
        }
    }
}

fn outcome_to_hook_outcome(outcome: &claudine::dispatch::DispatchOutcome) -> HookOutcome {
    if outcome.protect_pre.is_some() || outcome.protect_post.is_some() {
        return HookOutcome::Deny {
            reason: Some("Blocked by Claudine Protect".to_string()),
        };
    }

    let Some(response) = outcome.response.as_ref() else {
        return HookOutcome::allow_default();
    };

    let decision = response.get("decision").and_then(Value::as_str);
    let reason = response
        .get("reason")
        .and_then(Value::as_str)
        .map(str::to_string);

    match decision {
        Some("approve") | Some("allow") | None => HookOutcome::Allow { reason },
        Some("reject") | Some("deny") | Some("block") => HookOutcome::Deny { reason },
        Some("ask") => HookOutcome::Ask { reason },
        Some(other) => {
            debug!(decision = %other, "unrecognized hook decision; defaulting to allow");
            HookOutcome::Allow { reason }
        }
    }
}

// ---------------------------------------------------------------------------
// Initialize response validation
// ---------------------------------------------------------------------------

/// Validate that the server's initialize response uses a protocol version
/// Claudine knows how to drive. Returns `Ok(())` on a recognized version
/// and an error otherwise so the caller can convert into a terminal
/// `SemanticEvent::Error { kind: Configuration }`.
pub(crate) fn validate_initialize_response(
    result: &KimiInitializeResult,
) -> std::result::Result<(), WireInitError> {
    let Some(version) = result.protocol_version.as_deref() else {
        return Err(WireInitError::MissingProtocolVersion);
    };
    if version != WIRE_PROTOCOL_VERSION {
        return Err(WireInitError::UnsupportedProtocolVersion {
            negotiated: version.to_string(),
            expected: WIRE_PROTOCOL_VERSION.to_string(),
        });
    }
    Ok(())
}

/// Failure modes produced by [`validate_initialize_response`]. Maps onto a
/// terminal `SemanticEvent::Error { kind: Configuration }` at the call site.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum WireInitError {
    MissingProtocolVersion,
    UnsupportedProtocolVersion {
        negotiated: String,
        expected: String,
    },
}

impl std::fmt::Display for WireInitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingProtocolVersion => {
                write!(f, "kimi initialize response did not include protocol_version")
            }
            Self::UnsupportedProtocolVersion {
                negotiated,
                expected,
            } => write!(
                f,
                "kimi negotiated protocol version {negotiated} is not supported by this Claudine release (expected {expected})"
            ),
        }
    }
}

// ---------------------------------------------------------------------------
// Session orchestrator
// ---------------------------------------------------------------------------

/// Configuration for a Kimi wire-mode session.
pub(crate) struct WireSessionConfig<'a> {
    pub binary: &'a Path,
    pub args: &'a [String],
    pub env: &'a HashMap<OsString, OsString>,
    pub cwd: &'a Path,
    pub prompt: String,
    pub timeout: Option<u64>,
    pub client_name: &'a str,
    pub client_version: &'a str,
    pub capabilities: WireClientCapabilities,
    pub env_context: EnvironmentContext,
}

/// Live wiring threaded into the session. The parser builder, stream
/// output coordinator, and metrics handle are owned by the same surface
/// that already powers `run_child_stream_semantic`, so wire mode reuses
/// stderr coordination, JSONL logging, and tracing spans.
pub(crate) struct WireSessionWiring {
    pub build_parser: SemanticParserBuilder,
    pub stream_output: Arc<StreamOutput>,
    pub live_metrics: LiveMetrics,
    pub runtime_context: claudine::dispatch::DispatchRuntimeContext,
}

/// Run the full Kimi wire-mode lifecycle: spawn child, send initialize,
/// send prompt, route auto-responses, dispatch hooks, handle
/// cancellation, drain stdout/stderr, and finalize the parser into a
/// [`StreamExecutionSummary`].
pub(crate) fn run_kimi_wire_session(
    config: WireSessionConfig<'_>,
    wiring: WireSessionWiring,
    child_spawned: &mut bool,
) -> Result<ProcessResult<StreamExecutionSummary>> {
    debug_assert!(config.env.contains_key(&OsString::from("PATH")));
    debug_assert!(config.env.contains_key(&OsString::from("HOME")));

    let started_at = Instant::now();
    let span = info_span!("kimi_wire_session");
    let _guard = span.enter();
    let _ = wiring.live_metrics; // reserved for Phase 4 wiring of stall detection
    let _ = config.env_context; // reserved for Phase 5 hook dispatch context

    // Spawn child with stdin/stdout/stderr piped.
    let mut command = Command::new(config.binary);
    command
        .args(config.args)
        .env_clear()
        .envs(config.env)
        .current_dir(config.cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }

    let mut child = command.spawn()?;
    *child_spawned = true;
    Span::current().record("child_pid", tracing::field::display(child.id()));

    let stdin = child
        .stdin
        .take()
        .expect("child stdin must be piped: Stdio::piped() set above");
    let stdout_pipe = child
        .stdout
        .take()
        .expect("child stdout must be piped: Stdio::piped() set above");
    let stderr_pipe = child
        .stderr
        .take()
        .expect("child stderr must be piped: Stdio::piped() set above");

    let writer = WireWriter::from_child_stdin(stdin);

    // Forward stderr verbatim so kimi panics still surface.
    let stderr_handle = thread::spawn(move || {
        let reader = BufReader::new(stderr_pipe);
        let mut captured = String::new();
        for line in reader.lines() {
            let Ok(line) = line else { break };
            if !captured.is_empty() {
                captured.push('\n');
            }
            captured.push_str(&line);
            let mut err = std::io::stderr().lock();
            let _ = writeln!(err, "{line}");
        }
        captured
    });

    // Stdout reader thread: feeds the semantic parser, also classifies
    // each envelope inline so the writer can auto-respond to requests
    // without round-tripping through the parser sink.
    let writer_for_reader = writer.clone();
    let runtime_handle = tokio::runtime::Handle::try_current().ok();
    let runtime_context_for_reader = wiring.runtime_context.clone();
    let stream_span = Span::current();
    let stream_output = wiring.stream_output.clone();
    let stdout_handle: thread::JoinHandle<Box<dyn SemanticStreamParser>> = {
        let build_parser = wiring.build_parser;
        thread::spawn(move || {
            let _stream_guard = stream_span.enter();
            let _parse_span = info_span!("kimi_wire_stdout").entered();
            let reader = BufReader::new(stdout_pipe);
            let mut out = stream_output.stdout_writer();

            let output_cb: OutputTextCallback = Box::new(move |chunk: &str| {
                if !chunk.is_empty() {
                    let _ = out.write_all(chunk.as_bytes());
                }
            });
            let reasoning_cb: ReasoningCallback = Box::new(|_chunk: &str| {});
            let mut parser: Box<dyn SemanticStreamParser> = build_parser(output_cb, reasoning_cb);

            for line in reader.lines() {
                let Ok(line) = line else { break };
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }

                handle_request_dispatch(
                    trimmed,
                    &writer_for_reader,
                    runtime_handle.as_ref(),
                    &runtime_context_for_reader,
                );

                match parser.feed_line(&line) {
                    Ok(()) => {}
                    Err(StreamParseError::MalformedLine { .. }) => {
                        debug!("skipping malformed kimi wire line: {line}");
                    }
                    Err(StreamParseError::Fatal(error)) => {
                        warn!(error = %error, "kimi wire parser fatal error; continuing");
                    }
                }
            }

            parser
        })
    };

    // Send initialize and prompt on the main thread so we hit a single
    // tracing span for handshake timing.
    {
        let _init_span = info_span!("kimi_wire_initialize").entered();
        let initialize = build_initialize_request(
            config.client_name,
            config.client_version,
            config.capabilities,
        );
        if let Err(error) = writer.send_value(&initialize) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(error.into());
        }
    }

    {
        let _prompt_span = info_span!("kimi_wire_prompt_send").entered();
        let prompt_request = build_prompt_request(&config.prompt);
        if let Err(error) = writer.send_value(&prompt_request) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(error.into());
        }
    }

    // SIGINT forwarder: flip the cancel flag so the wait loop sends the
    // JSON-RPC cancel before falling back to SIGTERM/SIGKILL.
    let cancel_requested = Arc::new(AtomicBool::new(false));
    let signal_guard = install_sigint_forwarder(Arc::clone(&cancel_requested));

    let exit_code = match wait_for_child_exit(
        &mut child,
        config.timeout.map(Duration::from_secs),
        &cancel_requested,
        &writer,
    ) {
        Ok(code) => code,
        Err(error) => {
            warn!(error = %error, "kimi wire wait loop failed");
            -1
        }
    };

    let _ = signal_guard;

    let parser = match stdout_handle.join() {
        Ok(parser) => parser,
        Err(_) => {
            return Err(color_eyre::eyre::eyre!(
                "kimi wire stdout reader thread panicked"
            ));
        }
    };

    let stderr_text = stderr_handle.join().unwrap_or_default();

    let mut summary = parser.finish(exit_code);
    if summary.duration_ms.is_none() {
        summary.duration_ms = Some(started_at.elapsed().as_millis() as u64);
    }
    if !stderr_text.is_empty() && summary.stderr_text.is_none() {
        summary.stderr_text = Some(stderr_text);
    }

    let total_elapsed = started_at.elapsed();
    let termination = if cancel_requested.load(Ordering::SeqCst) {
        claudine::harness::ProcessTermination::Interrupted
    } else {
        claudine::harness::ProcessTermination::Completed
    };
    Ok(ProcessResult {
        data: summary,
        termination,
        telemetry: ProcessTelemetry {
            total_elapsed,
            first_response_latency: None,
        },
    })
}

/// Classify `line` as a Kimi request and, if so, write the auto-response.
///
/// Pulled out as a free function so the reader-thread closure stays
/// readable and so unit tests can drive the dispatch path without
/// constructing a full session.
fn handle_request_dispatch(
    trimmed: &str,
    writer: &WireWriter,
    runtime_handle: Option<&tokio::runtime::Handle>,
    runtime_context: &claudine::dispatch::DispatchRuntimeContext,
) {
    let Ok(value) = serde_json::from_str::<Value>(trimmed) else {
        return;
    };
    let Some(KimiEnvelope::Request { id, params }) = KimiEnvelope::classify(value) else {
        return;
    };
    let Some(request) = params.into_request() else {
        return;
    };
    let dispatch = dispatch_for_request(&request);
    let response = match dispatch {
        WireRequestDispatch::AutoApprove => {
            info!(request_id = %id, "auto-approving Kimi ApprovalRequest");
            build_approval_response(id)
        }
        WireRequestDispatch::EmptyQuestionAnswer => {
            warn!(
                request_id = %id,
                "responding to unexpected Kimi QuestionRequest with empty answer"
            );
            build_question_response(id)
        }
        WireRequestDispatch::UnsupportedToolCall => {
            warn!(
                request_id = %id,
                "rejecting unsupported Kimi ToolCallRequest with METHOD_NOT_FOUND"
            );
            build_tool_call_unsupported_error(id)
        }
        WireRequestDispatch::HookRequest(hook) => {
            let outcome = dispatch_hook_request(&hook, runtime_handle, runtime_context);
            info!(
                request_id = %id,
                event = ?hook.event,
                outcome = ?outcome,
                "Kimi HookRequest dispatched"
            );
            build_hook_response(id, outcome)
        }
    };
    if let Err(error) = writer.send_value(&response) {
        warn!(error = %error, "failed to write Kimi wire auto-response");
    }
}

#[cfg(unix)]
fn install_sigint_forwarder(flag: Arc<AtomicBool>) -> Option<signal_hook::SigId> {
    // SAFETY: `signal_hook::low_level::register` requires the closure to be
    // async-signal-safe; only an atomic store is performed.
    let register = unsafe {
        signal_hook::low_level::register(signal_hook::consts::SIGINT, move || {
            flag.store(true, Ordering::SeqCst);
        })
    };
    match register {
        Ok(id) => Some(id),
        Err(error) => {
            warn!(error = %error, "failed to install SIGINT handler for kimi wire session");
            None
        }
    }
}

#[cfg(not(unix))]
fn install_sigint_forwarder(_flag: Arc<AtomicBool>) -> Option<()> {
    None
}

/// Poll the child for exit, sending `cancel` when the cancel flag is set
/// or the wall-clock timeout elapses. After cancel is sent the child has
/// up to 5 s to terminate before SIGKILL is used as a hard fallback.
fn wait_for_child_exit(
    child: &mut Child,
    timeout: Option<Duration>,
    cancel_flag: &Arc<AtomicBool>,
    writer: &WireWriter,
) -> std::io::Result<i32> {
    let deadline = timeout.map(|d| Instant::now() + d);
    let mut cancel_sent = false;
    let mut cancel_sent_at: Option<Instant> = None;

    loop {
        if let Some(status) = child.try_wait()? {
            return Ok(status_to_code(&status));
        }

        let timeout_elapsed = deadline.is_some_and(|d| Instant::now() >= d);
        let user_canceled = cancel_flag.load(Ordering::SeqCst);

        if !cancel_sent && (timeout_elapsed || user_canceled) {
            let _cancel_span = info_span!("kimi_wire_cancel").entered();
            let envelope = build_cancel_request();
            match writer.send_value(&envelope) {
                Ok(_) => info!("sent kimi wire cancel"),
                Err(error) => warn!(error = %error, "failed to send kimi wire cancel"),
            }
            cancel_flag.store(true, Ordering::SeqCst);
            cancel_sent = true;
            cancel_sent_at = Some(Instant::now());
        }

        if let Some(sent_at) = cancel_sent_at
            && Instant::now() >= sent_at + Duration::from_secs(5)
        {
            warn!("kimi child did not exit 5s after cancel; sending SIGKILL");
            let _ = child.kill();
            let status = child.wait()?;
            return Ok(status_to_code(&status));
        }

        std::thread::sleep(Duration::from_millis(50));
    }
}

fn status_to_code(status: &std::process::ExitStatus) -> i32 {
    if let Some(code) = status.code() {
        return code;
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        if let Some(signal) = status.signal() {
            return 128 + signal;
        }
    }
    1
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn build_initialize_request_sends_canonical_protocol_version() {
        let value = build_initialize_request(
            "claudine",
            "0.1.0",
            WireClientCapabilities::default_for_claudine(),
        );
        assert_eq!(value["jsonrpc"], "2.0");
        assert_eq!(value["id"], INITIALIZE_REQUEST_ID);
        assert_eq!(value["method"], "initialize");
        assert_eq!(value["params"]["protocol_version"], WIRE_PROTOCOL_VERSION);
        assert_eq!(value["params"]["client"]["name"], "claudine");
        assert_eq!(value["params"]["client"]["version"], "0.1.0");
        assert_eq!(value["params"]["capabilities"]["approvals"], true);
        assert_eq!(value["params"]["capabilities"]["supports_question"], false);
        assert_eq!(value["params"]["capabilities"]["hooks"], true);
        assert_eq!(value["params"]["capabilities"]["subagents"], true);
        assert_eq!(value["params"]["capabilities"]["supports_plan_mode"], false);
    }

    #[test]
    fn build_prompt_request_uses_user_input_field() {
        let value = build_prompt_request("Hi how are you?");
        assert_eq!(value["jsonrpc"], "2.0");
        assert_eq!(value["id"], PROMPT_REQUEST_ID);
        assert_eq!(value["method"], "prompt");
        assert_eq!(value["params"]["user_input"], "Hi how are you?");
        assert!(value["params"].get("prompt").is_none());
    }

    #[test]
    fn build_cancel_request_uses_cancel_method_and_distinct_id() {
        let value = build_cancel_request();
        assert_eq!(value["method"], "cancel");
        assert_eq!(value["id"], CANCEL_REQUEST_ID);
        assert!(value["id"] != PROMPT_REQUEST_ID);
        assert_eq!(value["params"], json!({}));
    }

    #[test]
    fn build_approval_response_carries_approve_decision() {
        let value = build_approval_response(json!("req-1"));
        assert_eq!(value["id"], "req-1");
        assert_eq!(value["result"]["response"], "approve");
        assert!(value.get("error").is_none());
    }

    #[test]
    fn build_question_response_returns_empty_answer() {
        let value = build_question_response(json!("req-2"));
        assert_eq!(value["id"], "req-2");
        assert_eq!(value["result"]["answer"], "");
    }

    #[test]
    fn build_tool_call_unsupported_error_uses_method_not_found_code() {
        let value = build_tool_call_unsupported_error(json!("req-3"));
        assert_eq!(value["id"], "req-3");
        assert_eq!(value["error"]["code"], KimiJsonRpcError::METHOD_NOT_FOUND);
        assert!(
            value["error"]["message"]
                .as_str()
                .unwrap()
                .contains("not supported")
        );
    }

    #[test]
    fn build_hook_response_maps_allow_to_approve() {
        let value = build_hook_response(
            json!("req-4"),
            HookOutcome::Allow {
                reason: Some("ok".to_string()),
            },
        );
        assert_eq!(value["result"]["decision"], "approve");
        assert_eq!(value["result"]["reason"], "ok");
    }

    #[test]
    fn build_hook_response_maps_deny_to_reject() {
        let value = build_hook_response(json!("req-5"), HookOutcome::Deny { reason: None });
        assert_eq!(value["result"]["decision"], "reject");
        assert!(value["result"].get("reason").is_none());
    }

    #[test]
    fn build_hook_response_maps_ask_to_ask() {
        let value = build_hook_response(json!("req-6"), HookOutcome::Ask { reason: None });
        assert_eq!(value["result"]["decision"], "ask");
    }

    #[test]
    fn writer_serializes_with_trailing_newline_and_flushes() {
        struct CaptureWriter {
            buf: Arc<Mutex<Vec<u8>>>,
            flushed: Arc<AtomicBool>,
        }
        impl Write for CaptureWriter {
            fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
                self.buf.lock().unwrap().extend_from_slice(data);
                Ok(data.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                self.flushed.store(true, Ordering::Relaxed);
                Ok(())
            }
        }

        let buf = Arc::new(Mutex::new(Vec::new()));
        let flushed = Arc::new(AtomicBool::new(false));
        let writer = WireWriter::from_writer(Box::new(CaptureWriter {
            buf: Arc::clone(&buf),
            flushed: Arc::clone(&flushed),
        }));

        let value = json!({"hello": "world"});
        let serialized = writer.send_value(&value).expect("send_value succeeds");
        let captured = buf.lock().unwrap().clone();
        let captured_str = String::from_utf8(captured).unwrap();
        assert_eq!(captured_str, format!("{serialized}\n"));
        assert!(flushed.load(Ordering::Relaxed));
        assert!(captured_str.ends_with('\n'));
    }

    #[test]
    fn writer_send_then_send_writes_two_lines() {
        struct CaptureWriter(Arc<Mutex<Vec<u8>>>);
        impl Write for CaptureWriter {
            fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
                self.0.lock().unwrap().extend_from_slice(data);
                Ok(data.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let buf = Arc::new(Mutex::new(Vec::new()));
        let writer = WireWriter::from_writer(Box::new(CaptureWriter(Arc::clone(&buf))));
        writer.send_value(&json!({"a": 1})).unwrap();
        writer.send_value(&json!({"b": 2})).unwrap();
        let captured = buf.lock().unwrap().clone();
        let text = String::from_utf8(captured).unwrap();
        let lines: Vec<&str> = text.split_terminator('\n').collect();
        assert_eq!(lines.len(), 2);
        assert_eq!(serde_json::from_str::<Value>(lines[0]).unwrap(), json!({"a": 1}));
        assert_eq!(serde_json::from_str::<Value>(lines[1]).unwrap(), json!({"b": 2}));
    }

    #[test]
    fn writer_clones_share_one_pipe() {
        struct CaptureWriter(Arc<Mutex<Vec<u8>>>);
        impl Write for CaptureWriter {
            fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
                self.0.lock().unwrap().extend_from_slice(data);
                Ok(data.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let buf = Arc::new(Mutex::new(Vec::new()));
        let writer = WireWriter::from_writer(Box::new(CaptureWriter(Arc::clone(&buf))));
        let clone = writer.clone();
        writer.send_value(&json!({"a": 1})).unwrap();
        clone.send_value(&json!({"b": 2})).unwrap();
        let captured = buf.lock().unwrap().clone();
        let text = String::from_utf8(captured).unwrap();
        let lines: Vec<&str> = text.split_terminator('\n').collect();
        assert_eq!(lines.len(), 2);
    }

    #[test]
    fn dispatch_for_request_classifies_each_kimi_variant() {
        use claudine::stream::protocol::kimi::{
            KimiApprovalRequest, KimiHookRequest, KimiQuestionRequest, KimiToolCallRequest,
        };
        assert!(matches!(
            dispatch_for_request(&KimiWireRequest::Approval(KimiApprovalRequest::default())),
            WireRequestDispatch::AutoApprove
        ));
        assert!(matches!(
            dispatch_for_request(&KimiWireRequest::Question(KimiQuestionRequest::default())),
            WireRequestDispatch::EmptyQuestionAnswer
        ));
        assert!(matches!(
            dispatch_for_request(&KimiWireRequest::ToolCall(KimiToolCallRequest::default())),
            WireRequestDispatch::UnsupportedToolCall
        ));
        assert!(matches!(
            dispatch_for_request(&KimiWireRequest::Hook(KimiHookRequest::default())),
            WireRequestDispatch::HookRequest(_)
        ));
    }

    #[test]
    fn map_kimi_hook_event_covers_canonical_aliases() {
        assert_eq!(map_kimi_hook_event("PreToolUse"), Some(AgenticEvent::BeforeTool));
        assert_eq!(map_kimi_hook_event("PostToolUse"), Some(AgenticEvent::AfterTool));
        assert_eq!(map_kimi_hook_event("Stop"), Some(AgenticEvent::TurnComplete));
        assert_eq!(map_kimi_hook_event("UserPromptSubmit"), Some(AgenticEvent::BeforePrompt));
        assert!(map_kimi_hook_event("UnknownEvent").is_none());
    }

    #[test]
    fn validate_initialize_response_accepts_negotiated_version() {
        let result = KimiInitializeResult {
            protocol_version: Some(WIRE_PROTOCOL_VERSION.to_string()),
            ..Default::default()
        };
        assert!(validate_initialize_response(&result).is_ok());
    }

    #[test]
    fn validate_initialize_response_rejects_missing_version() {
        let result = KimiInitializeResult::default();
        let err = validate_initialize_response(&result).unwrap_err();
        assert!(matches!(err, WireInitError::MissingProtocolVersion));
    }

    #[test]
    fn validate_initialize_response_rejects_unsupported_version() {
        let result = KimiInitializeResult {
            protocol_version: Some("0.9".to_string()),
            ..Default::default()
        };
        let err = validate_initialize_response(&result).unwrap_err();
        match err {
            WireInitError::UnsupportedProtocolVersion {
                negotiated,
                expected,
            } => {
                assert_eq!(negotiated, "0.9");
                assert_eq!(expected, WIRE_PROTOCOL_VERSION);
            }
            _ => panic!("expected UnsupportedProtocolVersion"),
        }
    }

    #[test]
    fn outcome_to_hook_outcome_no_response_defaults_allow() {
        let outcome = claudine::dispatch::DispatchOutcome::default();
        assert!(matches!(
            outcome_to_hook_outcome(&outcome),
            HookOutcome::Allow { .. }
        ));
    }

    #[test]
    fn outcome_to_hook_outcome_explicit_decisions() {
        let allow = claudine::dispatch::DispatchOutcome {
            response: Some(json!({"decision": "approve", "reason": "ok"})),
            ..Default::default()
        };
        match outcome_to_hook_outcome(&allow) {
            HookOutcome::Allow { reason } => assert_eq!(reason.as_deref(), Some("ok")),
            other => panic!("expected Allow, got {other:?}"),
        }

        let deny = claudine::dispatch::DispatchOutcome {
            response: Some(json!({"decision": "reject"})),
            ..Default::default()
        };
        assert!(matches!(
            outcome_to_hook_outcome(&deny),
            HookOutcome::Deny { reason: None }
        ));

        let ask = claudine::dispatch::DispatchOutcome {
            response: Some(json!({"decision": "ask"})),
            ..Default::default()
        };
        assert!(matches!(
            outcome_to_hook_outcome(&ask),
            HookOutcome::Ask { reason: None }
        ));
    }

    #[test]
    fn outcome_to_hook_outcome_protect_block_maps_to_deny() {
        use claudine::services::protect::catalog::{RuleGroup, ScanSurface};
        use claudine::services::protect::decision::{ProtectDecision, ProtectMatch};
        let block = ProtectDecision::blocked(ProtectMatch {
            group: RuleGroup::FilesystemDestruction,
            rule_id: "test_block".to_string(),
            pattern: "test".to_string(),
            matched_text: "test".to_string(),
            surface: ScanSurface::BashCommand,
            target_path: None,
            config_key: "protect.rules.filesystem_destruction".to_string(),
        });
        let outcome = claudine::dispatch::DispatchOutcome {
            response: None,
            exit_code: None,
            protect_pre: Some(block),
            protect_post: None,
        };
        match outcome_to_hook_outcome(&outcome) {
            HookOutcome::Deny { reason } => {
                assert!(reason.unwrap().contains("Protect"));
            }
            other => panic!("expected Deny, got {other:?}"),
        }
    }

    #[test]
    fn dispatch_hook_request_unknown_event_returns_allow() {
        let request = KimiHookRequest {
            event: Some("Mystery".into()),
            ..Default::default()
        };
        let runtime_context = claudine::dispatch::DispatchRuntimeContext::default();
        let outcome = dispatch_hook_request(&request, None, &runtime_context);
        assert!(matches!(outcome, HookOutcome::Allow { .. }));
    }

    #[test]
    fn dispatch_hook_request_missing_event_returns_allow() {
        let request = KimiHookRequest::default();
        let runtime_context = claudine::dispatch::DispatchRuntimeContext::default();
        let outcome = dispatch_hook_request(&request, None, &runtime_context);
        assert!(matches!(outcome, HookOutcome::Allow { .. }));
    }

    #[test]
    fn handle_request_dispatch_writes_approval_for_approval_request() {
        struct CaptureWriter(Arc<Mutex<Vec<u8>>>);
        impl Write for CaptureWriter {
            fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
                self.0.lock().unwrap().extend_from_slice(data);
                Ok(data.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let buf = Arc::new(Mutex::new(Vec::new()));
        let writer = WireWriter::from_writer(Box::new(CaptureWriter(Arc::clone(&buf))));
        let runtime_context = claudine::dispatch::DispatchRuntimeContext::default();

        let line = json!({
            "jsonrpc": "2.0",
            "method": "request",
            "id": "req-99",
            "params": {
                "type": "ApprovalRequest",
                "payload": {
                    "id": "approval-1",
                    "tool_call_id": "tool-1"
                }
            }
        });
        let trimmed = serde_json::to_string(&line).unwrap();
        handle_request_dispatch(&trimmed, &writer, None, &runtime_context);

        let captured = buf.lock().unwrap().clone();
        let text = String::from_utf8(captured).unwrap();
        let response: Value = serde_json::from_str(text.trim()).unwrap();
        assert_eq!(response["id"], "req-99");
        assert_eq!(response["result"]["response"], "approve");
    }

    #[test]
    fn handle_request_dispatch_ignores_notifications() {
        struct CaptureWriter(Arc<Mutex<Vec<u8>>>);
        impl Write for CaptureWriter {
            fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
                self.0.lock().unwrap().extend_from_slice(data);
                Ok(data.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let buf = Arc::new(Mutex::new(Vec::new()));
        let writer = WireWriter::from_writer(Box::new(CaptureWriter(Arc::clone(&buf))));
        let runtime_context = claudine::dispatch::DispatchRuntimeContext::default();

        let line = json!({
            "jsonrpc": "2.0",
            "method": "event",
            "params": {"type": "TurnEnd", "payload": {}}
        });
        let trimmed = serde_json::to_string(&line).unwrap();
        handle_request_dispatch(&trimmed, &writer, None, &runtime_context);
        assert!(buf.lock().unwrap().is_empty());
    }

    #[test]
    fn handle_request_dispatch_writes_method_not_found_for_tool_call_request() {
        struct CaptureWriter(Arc<Mutex<Vec<u8>>>);
        impl Write for CaptureWriter {
            fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
                self.0.lock().unwrap().extend_from_slice(data);
                Ok(data.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let buf = Arc::new(Mutex::new(Vec::new()));
        let writer = WireWriter::from_writer(Box::new(CaptureWriter(Arc::clone(&buf))));
        let runtime_context = claudine::dispatch::DispatchRuntimeContext::default();
        let line = json!({
            "jsonrpc": "2.0",
            "method": "request",
            "id": "req-tool",
            "params": {"type": "ToolCallRequest", "payload": {"id": "x"}}
        });
        let trimmed = serde_json::to_string(&line).unwrap();
        handle_request_dispatch(&trimmed, &writer, None, &runtime_context);
        let captured = buf.lock().unwrap().clone();
        let text = String::from_utf8(captured).unwrap();
        let response: Value = serde_json::from_str(text.trim()).unwrap();
        assert_eq!(response["id"], "req-tool");
        assert_eq!(response["error"]["code"], KimiJsonRpcError::METHOD_NOT_FOUND);
    }
}
