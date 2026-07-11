//! (split out of `wiring/mod.rs`; see that file for the protocol overview)
#![allow(unused_imports)]
use super::*;

/// Wire protocol version Claudine requests on `initialize`. Live Kimi
/// servers now speak Wire 1.10 and reject a client that requests 1.9, so
/// Claudine advertises 1.10; the lib parser accepts any response version in
/// `claudine::stream::protocol::kimi::SUPPORTED_WIRE_PROTOCOL_VERSIONS`
/// (currently 1.9 and 1.10), so an older server that answers 1.9 still
/// works.
pub(crate) const WIRE_PROTOCOL_VERSION: &str = "1.10";

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
