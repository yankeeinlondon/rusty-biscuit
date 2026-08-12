//! (split out of `wiring/mod.rs`; see that file for the protocol overview)
#![allow(unused_imports)]
use super::*;

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
pub(crate) fn build_hook_event_meta(
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

pub(crate) fn outcome_to_hook_outcome(outcome: &claudine::dispatch::DispatchOutcome) -> HookOutcome {
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
pub(crate) fn handle_request_dispatch(
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
pub(crate) fn is_prompt_response_line(line: &str) -> bool {
    let Ok(value) = serde_json::from_str::<Value>(line) else {
        return false;
    };
    let id_matches = value.get("id").and_then(Value::as_str) == Some(PROMPT_REQUEST_ID);
    if !id_matches {
        return false;
    }
    value.get("result").is_some() || value.get("error").is_some()
}

/// Return true when `line` is a JSON-RPC *error* response to the `init-1`
/// initialize request.
///
/// A server that rejects the handshake (e.g. a strict protocol-version
/// check on our `initialize` request) answers `init-1` with an error and
/// will never process the prompt, so no `prompt-2` response ever arrives.
/// The reader thread treats this like the prompt response — close stdin and
/// signal completion — so handshake failures fail fast with the
/// parser-surfaced error instead of hanging until the wall-clock timeout.
/// A *successful* `init-1` response is not terminal: the session proceeds
/// to the prompt.
pub(crate) fn is_initialize_error_line(line: &str) -> bool {
    let Ok(value) = serde_json::from_str::<Value>(line) else {
        return false;
    };
    value.get("id").and_then(Value::as_str) == Some(INITIALIZE_REQUEST_ID)
        && value.get("error").is_some()
}
