//! Kimi `--wire` JSON-RPC 2.0 line transport.
//!
//! Wire mode is Kimi's structured client protocol. Stdout carries one
//! JSON-RPC envelope per line (notification, server-initiated request, or
//! response to a previously-sent client request); stdin accepts the same
//! shape on a single serialized writer. The semantic parser
//! ([`claudine::stream::providers::kimi::KimiSemanticStreamParser`]) consumes
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

use std::collections::HashMap;
use std::ffi::OsString;
use std::io::{self, BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use claudine::events::{AgenticEvent, EnvironmentContext, EventMeta};

use claudine::provider::Provider;
use claudine::stream::parser::{SemanticStreamParser, StreamParseError};
use claudine::stream::progress::LiveMetrics;
use claudine::stream::protocol::kimi::{
    KimiEnvelope, KimiHookRequest, KimiInitializeResult, KimiJsonRpcError, KimiWireRequest,
};
use claudine::stream::summary::StreamExecutionSummary;
use color_eyre::eyre::Result;
use serde_json::{Value, json};
use tracing::{Span, debug, info, info_span, warn};

use super::super::stream_io::StreamOutput;
use super::{
    OutputTextCallback, ProcessResult, ProcessTelemetry, ReasoningCallback, SemanticParserBuilder,
};

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

/// Result returned by [`dispatch_hook_request`]. Carries the Kimi-bound
/// hook outcome plus an optional user-facing warning message that the
/// caller can surface via the live semantic sink.
///
/// `warning` is set when the dispatch infrastructure itself fails — e.g.
/// no async runtime handle is available, the canonical event name is
/// unrecognized, or `dispatch_event_meta_with_runtime` returns `Err`. A
/// dispatch that returns a normal `Allow`/`Deny`/`Ask` decision has no
/// warning attached even when the decision is `Deny`, because that's a
/// successful policy evaluation rather than an infrastructure error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HookDispatchResult {
    pub outcome: HookOutcome,
    pub warning: Option<String>,
}

impl HookDispatchResult {
    pub(crate) fn allow_default() -> Self {
        Self {
            outcome: HookOutcome::allow_default(),
            warning: None,
        }
    }

    pub(crate) fn allow_with_warning(warning: impl Into<String>) -> Self {
        Self {
            outcome: HookOutcome::allow_default(),
            warning: Some(warning.into()),
        }
    }
}

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

    /// Close the underlying child stdin pipe by replacing the inner writer
    /// with `io::sink()`. This drops the original `ChildStdin`, signalling
    /// EOF to the Kimi child so it exits its read loop cleanly after the
    /// prompt response has been received. Subsequent `send_value` calls
    /// succeed silently (the bytes go to the sink), so any late cancel
    /// path remains a no-op rather than a panic.
    pub(crate) fn close_stdin(&self) {
        let mut guard = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        *guard = Box::new(io::sink());
    }
}

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

/// Build the [`EventMeta`] for a Kimi `HookRequest` dispatch.
///
/// Copies the request context into the typed slots and `extra` map, then
/// stamps the wrapper [`EnvironmentContext`] and the spawned child PID so the
/// dispatched hook/action context exposes `claudine_pid` (via
/// `EnvironmentContext`) and `agent_pid`. Both PIDs are also mirrored into
/// `extra` — matching the summary/semantic helpers in
/// [`claudine::stream::reporting`] — so templates and expressions can resolve
/// them even though the typed fields remain authoritative for JSONL and SQL
/// ingest.
fn build_hook_event_meta(
    request: &KimiHookRequest,
    canonical_event: AgenticEvent,
    env_context: &EnvironmentContext,
    agent_pid: Option<u32>,
) -> EventMeta {
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

    meta.env = env_context.clone();
    meta.agent_pid = agent_pid;
    if let Some(pid) = env_context.claudine_pid {
        meta.extra
            .entry("claudine_pid".to_string())
            .or_insert(Value::Number(serde_json::Number::from(pid)));
    }
    if let Some(pid) = agent_pid {
        meta.extra
            .entry("agent_pid".to_string())
            .or_insert(Value::Number(serde_json::Number::from(pid)));
    }
    meta
}

/// Dispatch a Kimi `HookRequest` through Claudine's canonical pipeline
/// and return the negotiated [`HookDispatchResult`].
///
/// `env_context` and `agent_pid` are the wrapper's session environment and
/// the spawned child PID; they are stamped onto the dispatched
/// [`EventMeta`] so the hook/action context, dispatch JSONL, and reporting
/// ingest all carry `claudine_pid` and `agent_pid`.
///
/// Falls back to [`HookDispatchResult::allow_default`] when no Claudine
/// config is loaded, when the runtime resolution fails, when the event
/// name is unknown, or when no async runtime handle is available. The
/// `warning` field is populated when the dispatch infrastructure itself
/// fails so the caller can surface the failure via a synthetic
/// `Notification` envelope routed through the semantic parser.
pub(crate) fn dispatch_hook_request(
    request: &KimiHookRequest,
    runtime_handle: Option<&tokio::runtime::Handle>,
    runtime_context: &claudine::dispatch::DispatchRuntimeContext,
    env_context: &EnvironmentContext,
    agent_pid: Option<u32>,
) -> HookDispatchResult {
    let Some(event_name) = request.event.as_deref() else {
        return HookDispatchResult::allow_default();
    };
    let Some(canonical_event) = map_kimi_hook_event(event_name) else {
        debug!(
            provider = "kimi",
            event = %event_name,
            "unknown Kimi hook event name; defaulting to allow"
        );
        return HookDispatchResult::allow_default();
    };
    let Some(handle) = runtime_handle else {
        debug!(
            provider = "kimi",
            event = %event_name,
            "no async runtime handle for hook dispatch; defaulting to allow"
        );
        return HookDispatchResult::allow_default();
    };

    let meta = build_hook_event_meta(request, canonical_event, env_context, agent_pid);

    let dispatch_result = handle.block_on(claudine::dispatch::dispatch_event_meta_with_runtime(
        Provider::KimiCode,
        canonical_event,
        meta,
        runtime_context,
    ));

    match dispatch_result {
        Ok(outcome) => HookDispatchResult {
            outcome: outcome_to_hook_outcome(&outcome),
            warning: None,
        },
        Err(error) => {
            warn!(
                provider = "kimi",
                event = %event_name,
                error = %error,
                "hook dispatch failed; defaulting to allow"
            );
            HookDispatchResult::allow_with_warning(format!(
                "Hook dispatch failed for {event_name}: {error}"
            ))
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

/// Validate that the server's initialize response uses a protocol version
/// Claudine knows how to drive. Returns `Ok(())` on a recognized version
/// and an error otherwise so the caller can convert into a terminal
/// `SemanticEvent::Error { kind: Configuration }`.
#[allow(dead_code)]
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
#[allow(dead_code)]
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
                write!(
                    f,
                    "kimi initialize response did not include protocol_version"
                )
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

/// Configuration for a Kimi wire-mode session.
pub(crate) struct WireSessionConfig<'a> {
    pub binary: &'a Path,
    pub args: &'a [String],
    pub env: &'a HashMap<OsString, OsString>,
    pub cwd: &'a Path,
    pub prompt: String,
    pub timeout: Option<Duration>,
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
    let captured_pid = child.id();
    *child_spawned = true;
    Span::current().record("child_pid", tracing::field::display(captured_pid));

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
    let env_context_for_reader = config.env_context.clone();
    let stream_span = Span::current();
    let stream_output = wiring.stream_output.clone();
    let prompt_finished = Arc::new(AtomicBool::new(false));
    let prompt_finished_for_reader = Arc::clone(&prompt_finished);
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
            let mut parser: Box<dyn SemanticStreamParser> =
                build_parser(output_cb, reasoning_cb, Some(captured_pid));

            for line in reader.lines() {
                let Ok(line) = line else { break };
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }

                let synthetic = handle_request_dispatch(
                    trimmed,
                    &writer_for_reader,
                    runtime_handle.as_ref(),
                    &runtime_context_for_reader,
                    &env_context_for_reader,
                    Some(captured_pid),
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

                // Feed any synthetic diagnostic envelope produced by the
                // request-dispatch path (e.g. hook dispatch failures) so
                // the semantic parser surfaces it as a `SemanticEvent::Warning`.
                if let Some(envelope) = synthetic
                    && let Ok(serialized) = serde_json::to_string(&envelope)
                    && let Err(error) = parser.feed_line(&serialized)
                {
                    debug!(?error, "failed to feed synthetic warning envelope");
                }

                // Signal the wait loop the moment the prompt response
                // arrives. `close_stdin` is the graceful path (Kimi exits
                // on EOF), and `prompt_finished` is the hard fallback so
                // the wait loop forces exit if Kimi keeps the channel
                // open after responding.
                if is_prompt_response_line(trimmed) {
                    info!("kimi prompt response received; closing wire stdin");
                    writer_for_reader.close_stdin();
                    prompt_finished_for_reader.store(true, Ordering::SeqCst);
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
        config.timeout,
        &cancel_requested,
        &prompt_finished,
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
        agent_pid: Some(captured_pid),
    })
}

/// Classify `line` as a Kimi request and, if so, write the auto-response.
///
/// Pulled out as a free function so the reader-thread closure stays
/// readable and so unit tests can drive the dispatch path without
/// constructing a full session.
///
/// Returns `Some(envelope)` when the request produced a user-visible
/// diagnostic (currently: hook dispatch infrastructure failures) that
/// the caller should feed back into the semantic parser so it surfaces
/// as a `SemanticEvent::Warning`. Returns `None` for normal flows.
#[must_use = "synthetic envelope must be fed into the semantic parser"]
fn handle_request_dispatch(
    trimmed: &str,
    writer: &WireWriter,
    runtime_handle: Option<&tokio::runtime::Handle>,
    runtime_context: &claudine::dispatch::DispatchRuntimeContext,
    env_context: &EnvironmentContext,
    agent_pid: Option<u32>,
) -> Option<Value> {
    let Ok(value) = serde_json::from_str::<Value>(trimmed) else {
        return None;
    };
    let Some(KimiEnvelope::Request { id, params }) = KimiEnvelope::classify(value) else {
        return None;
    };
    let request = params.into_request()?;
    let dispatch = dispatch_for_request(&request);
    let mut synthetic: Option<Value> = None;
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
            let result = dispatch_hook_request(
                &hook,
                runtime_handle,
                runtime_context,
                env_context,
                agent_pid,
            );
            info!(
                request_id = %id,
                event = ?hook.event,
                outcome = ?result.outcome,
                "Kimi HookRequest dispatched"
            );
            if let Some(message) = &result.warning {
                synthetic = Some(build_synthetic_warning_envelope(message));
            }
            build_hook_response(id, result.outcome)
        }
    };
    if let Err(error) = writer.send_value(&response) {
        warn!(error = %error, "failed to write Kimi wire auto-response");
    }
    synthetic
}

/// Return true when `line` is the JSON-RPC response (success or error) to
/// the `prompt-2` request Claudine sent at session start.
///
/// The Kimi wire protocol is a persistent JSON-RPC channel: after the
/// prompt response arrives, kimi sits idle waiting for further commands
/// rather than exiting. For non-interactive Claudine sessions there are no
/// further commands, so the reader thread closes stdin (signalling EOF)
/// the moment this line is observed. Both `result` and `error` shapes are
/// treated as terminal — an auth-expired error on `prompt-2`, for example,
/// still ends the session.
fn is_prompt_response_line(line: &str) -> bool {
    let Ok(value) = serde_json::from_str::<Value>(line) else {
        return false;
    };
    let id_matches = value.get("id").and_then(Value::as_str) == Some(PROMPT_REQUEST_ID);
    if !id_matches {
        return false;
    }
    value.get("result").is_some() || value.get("error").is_some()
}

/// Construct a synthetic `Notification` envelope with `level == "error"`
/// so the Kimi semantic parser surfaces it as a
/// [`SemanticEvent::Warning`] on the live stderr surface and JSONL log.
///
/// Used by [`handle_request_dispatch`] to make hook dispatch
/// infrastructure failures visible to the user. The envelope shape
/// matches Kimi's wire `Notification` event so it round-trips through
/// the existing parser path without special-casing.
pub(crate) fn build_synthetic_warning_envelope(message: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "method": "event",
        "params": {
            "type": "Notification",
            "payload": {
                "level": "error",
                "source": "claudine",
                "message": message,
            },
        },
    })
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

/// Grace period after the `prompt-2` response arrives before SIGKILL.
///
/// Kimi's wire session is persistent — it does not exit on its own when a
/// prompt completes. Stdin is already closed (EOF) at this point so a
/// well-behaved Kimi build will quit promptly; the grace period covers
/// any final stderr flush or async cleanup. After the grace period
/// elapses, the child is killed unconditionally.
const PROMPT_FINISHED_GRACE: Duration = Duration::from_millis(750);

/// Poll the child for exit, sending `cancel` when the cancel flag is set
/// or the wall-clock timeout elapses, and forcibly terminating the child
/// shortly after the prompt response arrives so the non-interactive
/// wrapper does not hang on Kimi's persistent JSON-RPC session.
fn wait_for_child_exit(
    child: &mut Child,
    timeout: Option<Duration>,
    cancel_flag: &Arc<AtomicBool>,
    prompt_finished: &Arc<AtomicBool>,
    writer: &WireWriter,
) -> std::io::Result<i32> {
    let deadline = timeout.map(|d| Instant::now() + d);
    let mut cancel_sent = false;
    let mut cancel_sent_at: Option<Instant> = None;
    let mut prompt_finished_at: Option<Instant> = None;

    loop {
        if let Some(status) = child.try_wait()? {
            return Ok(status_to_code(&status));
        }

        let timeout_elapsed = deadline.is_some_and(|d| Instant::now() >= d);
        let user_canceled = cancel_flag.load(Ordering::SeqCst);
        let prompt_done = prompt_finished.load(Ordering::SeqCst);

        if prompt_done && prompt_finished_at.is_none() {
            prompt_finished_at = Some(Instant::now());
        }

        // Hard-stop fallback: Kimi's wire mode does not terminate when a
        // prompt completes — stdin EOF is the expected signal but some
        // builds keep async tasks alive. Once the grace period elapses,
        // kill the child directly. Report exit code 0 because the prompt
        // already completed; the semantic parser surfaces real errors
        // (auth-expired, cancelled, etc.) from the response payload, not
        // from the synthetic SIGKILL exit code.
        if let Some(at) = prompt_finished_at
            && Instant::now() >= at + PROMPT_FINISHED_GRACE
        {
            info!("kimi prompt finished; terminating child after grace period");
            let _ = child.kill();
            let _ = child.wait()?;
            return Ok(0);
        }

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
mod tests;
