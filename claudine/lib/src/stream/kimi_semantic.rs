//! Native [`SemanticStreamParser`] implementation for Kimi Code's `--wire`
//! JSON-RPC 2.0 protocol.
//!
//! Each line of `kimi --wire` stdout is a JSON-RPC envelope: a notification
//! (`method: "event"`), a server-initiated request (`method: "request"`), or
//! a response to a previously-sent client request (`id` plus `result` or
//! `error`). Envelopes are decoded via
//! [`crate::stream::protocol::kimi::KimiEnvelope::classify`] and dispatched
//! to typed [`KimiWireEvent`] / [`KimiWireRequest`] handlers.
//!
//! Assistant text and reasoning stream as `ContentPart` deltas which are
//! mapped to [`SemanticEvent::OutputText`] / [`SemanticEvent::Reasoning`].
//! Tool invocations arrive as a `ToolCall` envelope plus a series of
//! `ToolCallPart` `arguments_part` string deltas; the parser concatenates
//! the deltas and JSON-decodes the final blob before emitting
//! [`SemanticEvent::ToolCall`]. The canonical end-of-turn signal is the
//! `prompt` request response, which carries one of `finished`, `cancelled`,
//! `max_steps_reached`, or `steered`. Context-window pressure crossing
//! [`CONTEXT_PRESSURE_WARN_PERCENT`] emits a [`SemanticEvent::Warning`].

use std::collections::HashMap;

use serde_json::{Map, Value};
use tracing::debug;

use super::parser::{SemanticStreamParser, StreamParseError};
use super::protocol::kimi::{
    KimiContentPart, KimiEnvelope, KimiJsonRpcError, KimiPromptResult, KimiToolCall,
    KimiToolCallPart, KimiToolResult, KimiWireEvent, KimiWireRequest, KimiWireStatusUpdate,
    KimiWireTokenUsage,
};
use super::semantic::{SemanticErrorKind, SemanticEvent, SemanticEventSink};
use super::summary::{ContextUsage, StreamExecutionSummary};
use super::token_usage::NormalizedTokenUsage;
use crate::provider::Provider;
const CONTEXT_PRESSURE_WARN_PERCENT: f64 = 80.0;

/// In-flight tool-call accumulator. Holds the tool name, the optional id, and
/// the streamed `arguments_part` deltas captured between the originating
/// `ToolCall` envelope and the next non-`ToolCallPart` event.
#[derive(Debug, Default)]
struct PendingToolCall {
    id: Option<String>,
    name: Option<String>,
    arguments: String,
    /// Unique slot for this pending call so tests can correlate ids when the
    /// provider doesn't emit one.
    slot: u64,
}

pub struct KimiSemanticStreamParser<S: SemanticEventSink> {
    sink: S,
    line_num: usize,
    session_id: Option<String>,
    model: Option<String>,
    assistant_text: String,
    /// Buffer for the in-flight assistant message; flushed to `assistant_text`
    /// on `TurnEnd` or when reasoning interrupts text streaming.
    pending_text: String,
    token_usage: Option<NormalizedTokenUsage>,
    cost_usd: Option<f64>,
    duration_ms: Option<u64>,
    num_turns: u32,
    tool_calls: u32,
    provider_status: Option<String>,
    is_error: bool,
    error_kind: Option<String>,
    error_message: Option<String>,
    context_usage: Option<ContextUsage>,
    /// Tool name keyed by `tool_call_id` so `ToolResult` can recover the
    /// originating tool name (Kimi's `ToolResult` payload omits it).
    tool_uses: HashMap<String, String>,
    pending_tool_call: Option<PendingToolCall>,
    next_pending_slot: u64,
    /// Toggles to `true` once the parser observes a `prompt` response so the
    /// summary's `provider_status` reflects the canonical status string
    /// (`finished` / `cancelled` / `max_steps_reached` / `steered`).
    prompt_status_seen: bool,
}

impl<S: SemanticEventSink> KimiSemanticStreamParser<S> {
    pub fn new(sink: S) -> Self {
        Self {
            sink,
            line_num: 0,
            session_id: None,
            model: None,
            assistant_text: String::new(),
            pending_text: String::new(),
            token_usage: None,
            cost_usd: None,
            duration_ms: None,
            num_turns: 0,
            tool_calls: 0,
            provider_status: None,
            is_error: false,
            error_kind: None,
            error_message: None,
            context_usage: None,
            tool_uses: HashMap::new(),
            pending_tool_call: None,
            next_pending_slot: 0,
            prompt_status_seen: false,
        }
    }

    fn base_extra(&self, raw_kind: &str) -> Map<String, Value> {
        let mut m = Map::new();
        m.insert("provider".into(), Value::from("kimi"));
        m.insert("line_num".into(), Value::from(self.line_num));
        m.insert("raw_kind".into(), Value::from(raw_kind));
        m
    }

    fn info_extra_with_kind(&self, raw_kind: &str, info_kind: &str) -> Map<String, Value> {
        let mut extra = self.base_extra(raw_kind);
        extra.insert("kind".into(), Value::from(info_kind));
        extra
    }

    fn flush_pending_tool_call(&mut self, raw_kind: &str) {
        let Some(pending) = self.pending_tool_call.take() else {
            return;
        };
        self.tool_calls += 1;
        super::trace_tool_event(Provider::KimiCode, self.tool_calls, pending.name.as_deref());
        let parsed_input = if pending.arguments.trim().is_empty() {
            None
        } else {
            match KimiToolCall::parse_arguments_string(&pending.arguments) {
                Ok(value) => value,
                Err(raw) => Some(Value::String(raw)),
            }
        };
        let mut extra = self.base_extra(raw_kind);
        if let Some(id) = &pending.id {
            extra.insert("tool_id".into(), Value::from(id.as_str()));
        }
        if let Some(name) = &pending.name {
            extra.insert("tool_name".into(), Value::from(name.as_str()));
        }
        extra.insert("slot".into(), Value::from(pending.slot));
        if let (Some(id), Some(name)) = (pending.id.as_ref(), pending.name.as_ref()) {
            self.tool_uses.insert(id.clone(), name.clone());
        }
        self.sink.on_semantic_event(SemanticEvent::ToolCall {
            name: pending.name,
            id: pending.id,
            input: parsed_input,
            extra: Value::Object(extra),
        });
    }

    fn handle_session_start(&mut self) {
        if self.session_id.is_some() || self.model.is_some() {
            return;
        }
        super::trace_session_metadata(
            Provider::KimiCode,
            self.session_id.as_deref(),
            self.model.as_deref(),
        );
        self.sink.on_semantic_event(SemanticEvent::SessionStart {
            session_id: self.session_id.clone(),
            model: self.model.clone(),
            extra: Value::Object(self.base_extra("initialize")),
        });
    }

    fn handle_initialize_response(&mut self, result: Value) {
        let parsed: super::protocol::kimi::KimiInitializeResult =
            match serde_json::from_value(result.clone()) {
                Ok(v) => v,
                Err(err) => {
                    debug!(error = %err, "kimi initialize result decode failed");
                    self.handle_session_start();
                    return;
                }
            };
        if let Some(server) = parsed.server.as_ref()
            && self.model.is_none()
            && let Some(name) = server.name.clone()
        {
            self.model = Some(name);
        }
        let mut extra = self.base_extra("initialize");
        if let Some(version) = parsed.protocol_version {
            extra.insert("protocol_version".into(), Value::from(version));
        }
        if let Some(server) = parsed.server {
            if let Some(name) = server.name {
                extra.insert("server_name".into(), Value::from(name));
            }
            if let Some(version) = server.version {
                extra.insert("server_version".into(), Value::from(version));
            }
        }
        super::trace_session_metadata(
            Provider::KimiCode,
            self.session_id.as_deref(),
            self.model.as_deref(),
        );
        self.sink.on_semantic_event(SemanticEvent::SessionStart {
            session_id: self.session_id.clone(),
            model: self.model.clone(),
            extra: Value::Object(extra),
        });
    }

    fn handle_prompt_response(&mut self, result: Value) {
        let parsed: KimiPromptResult = serde_json::from_value(result).unwrap_or_default();
        if let Some(status) = parsed.status {
            self.prompt_status_seen = true;
            self.provider_status = Some(status.clone());
            if status == KimiPromptResult::STATUS_CANCELLED {
                self.is_error = true;
                if self.error_kind.is_none() {
                    self.error_kind = Some("interrupted".into());
                }
                if self.error_message.is_none() {
                    self.error_message = Some("Prompt cancelled".into());
                }
                let mut extra = self.base_extra("prompt_response");
                extra.insert("status".into(), Value::from("cancelled"));
                self.sink.on_semantic_event(SemanticEvent::Error {
                    message: "Prompt cancelled".into(),
                    terminal: true,
                    kind: SemanticErrorKind::Interrupted,
                    extra: Value::Object(extra),
                });
            } else {
                let mut extra = self.info_extra_with_kind("prompt_response", "prompt_status");
                extra.insert("status".into(), Value::from(status.as_str()));
                self.sink.on_semantic_event(SemanticEvent::Info {
                    message: format!("Prompt status: {status}"),
                    extra: Value::Object(extra),
                });
            }
            super::trace_summary_update(
                Provider::KimiCode,
                self.provider_status.as_deref(),
                self.duration_ms,
                self.cost_usd,
            );
        }
    }

    fn handle_error_response(&mut self, error: KimiJsonRpcError) {
        self.is_error = true;
        let kind_label = match error.code {
            KimiJsonRpcError::AUTH_EXPIRED => "auth_expired",
            KimiJsonRpcError::CHAT_PROVIDER_ERROR => "chat_provider_error",
            KimiJsonRpcError::PARSE_ERROR => "parse_error",
            KimiJsonRpcError::INVALID_REQUEST => "invalid_request",
            KimiJsonRpcError::METHOD_NOT_FOUND => "method_not_found",
            KimiJsonRpcError::INVALID_PARAMS => "invalid_params",
            KimiJsonRpcError::INTERNAL_ERROR => "internal_error",
            _ => "jsonrpc_error",
        };
        self.error_kind = Some(kind_label.into());
        self.error_message = Some(error.message.clone());
        let mut extra = self.base_extra("error_response");
        extra.insert("code".into(), Value::from(error.code));
        extra.insert("error_kind".into(), Value::from(kind_label));
        if let Some(data) = error.data {
            extra.insert("data".into(), data);
        }
        let kind = classify_jsonrpc_error(error.code, &error.message);
        self.sink.on_semantic_event(SemanticEvent::Error {
            message: error.message,
            terminal: true,
            kind,
            extra: Value::Object(extra),
        });
    }

    fn handle_event(&mut self, event: KimiWireEvent, raw_kind: &str) {
        match event {
            KimiWireEvent::TurnBegin(turn) => {
                self.flush_pending_tool_call(raw_kind);
                let mut extra = self.base_extra(raw_kind);
                if let Some(text) = turn.user_input_text() {
                    extra.insert("user_input".into(), Value::from(text));
                }
                self.sink.on_semantic_event(SemanticEvent::TurnStart {
                    extra: Value::Object(extra),
                });
            }
            KimiWireEvent::TurnEnd(_) => {
                self.flush_pending_tool_call(raw_kind);
                self.flush_pending_text();
                self.num_turns = self.num_turns.saturating_add(1);
                let mut extra = self.base_extra(raw_kind);
                extra.insert("num_turns".into(), Value::from(self.num_turns));
                self.sink.on_semantic_event(SemanticEvent::TurnComplete {
                    provider_status: self.provider_status.clone(),
                    token_usage: self.token_usage.clone(),
                    cost_usd: self.cost_usd,
                    duration_ms: self.duration_ms,
                    extra: Value::Object(extra),
                });
            }
            KimiWireEvent::StepBegin(step) => {
                let mut extra = self.info_extra_with_kind(raw_kind, "step_begin");
                if let Some(n) = step.n {
                    extra.insert("step".into(), Value::from(n));
                }
                self.sink.on_semantic_event(SemanticEvent::Info {
                    message: match step.n {
                        Some(n) => format!("Step {n}"),
                        None => "Step".into(),
                    },
                    extra: Value::Object(extra),
                });
            }
            KimiWireEvent::StepInterrupted(step) => {
                let mut extra = self.info_extra_with_kind(raw_kind, "step_interrupted");
                if let Some(reason) = &step.reason {
                    extra.insert("reason".into(), Value::from(reason.as_str()));
                }
                self.sink.on_semantic_event(SemanticEvent::Warning {
                    message: match step.reason {
                        Some(reason) => format!("Step interrupted: {reason}"),
                        None => "Step interrupted".into(),
                    },
                    extra: Value::Object(extra),
                });
            }
            KimiWireEvent::SteerInput(_) => {
                let extra = self.info_extra_with_kind(raw_kind, "steer_input");
                self.sink.on_semantic_event(SemanticEvent::Info {
                    message: "Steering input received".into(),
                    extra: Value::Object(extra),
                });
            }
            KimiWireEvent::CompactionBegin(begin) => {
                let mut extra = self.info_extra_with_kind(raw_kind, "compaction_begin");
                if let Some(focus) = &begin.focus {
                    extra.insert("focus".into(), Value::from(focus.as_str()));
                }
                self.sink.on_semantic_event(SemanticEvent::Info {
                    message: "Compaction started".into(),
                    extra: Value::Object(extra),
                });
            }
            KimiWireEvent::CompactionEnd(end) => {
                let mut extra = self.info_extra_with_kind(raw_kind, "compaction_end");
                if let Some(saved) = end.tokens_saved {
                    extra.insert("tokens_saved".into(), Value::from(saved));
                }
                self.sink.on_semantic_event(SemanticEvent::Info {
                    message: match end.tokens_saved {
                        Some(saved) => format!("Compaction complete (saved {saved} tokens)"),
                        None => "Compaction complete".into(),
                    },
                    extra: Value::Object(extra),
                });
            }
            KimiWireEvent::McpLoadingBegin(begin) => {
                let mut extra = self.info_extra_with_kind(raw_kind, "mcp_loading_begin");
                if let Some(server) = &begin.server {
                    extra.insert("server".into(), Value::from(server.as_str()));
                }
                self.sink.on_semantic_event(SemanticEvent::Info {
                    message: match begin.server {
                        Some(server) => format!("Loading MCP server {server}"),
                        None => "Loading MCP server".into(),
                    },
                    extra: Value::Object(extra),
                });
            }
            KimiWireEvent::McpLoadingEnd(end) => {
                let mut extra = self.info_extra_with_kind(raw_kind, "mcp_loading_end");
                if let Some(server) = &end.server {
                    extra.insert("server".into(), Value::from(server.as_str()));
                }
                if let Some(status) = &end.status {
                    extra.insert("status".into(), Value::from(status.as_str()));
                }
                self.sink.on_semantic_event(SemanticEvent::Info {
                    message: match (end.server, end.status) {
                        (Some(server), Some(status)) => {
                            format!("MCP server {server} loaded ({status})")
                        }
                        (Some(server), None) => format!("MCP server {server} loaded"),
                        _ => "MCP loading complete".into(),
                    },
                    extra: Value::Object(extra),
                });
            }
            KimiWireEvent::StatusUpdate(status) => {
                self.handle_status_update(status, raw_kind);
            }
            KimiWireEvent::Notification(notification) => {
                let mut extra = self.info_extra_with_kind(raw_kind, "notification");
                if let Some(level) = &notification.level {
                    extra.insert("level".into(), Value::from(level.as_str()));
                }
                if let Some(source) = &notification.source {
                    extra.insert("source".into(), Value::from(source.as_str()));
                }
                if let Some(title) = &notification.title {
                    extra.insert("title".into(), Value::from(title.as_str()));
                }
                let level = notification.level.as_deref().map(str::to_ascii_lowercase);
                let message = notification
                    .message
                    .or(notification.title)
                    .unwrap_or_else(|| "Notification".into());
                // Error/warning-level notifications surface as Warning so the
                // user sees them in the live stderr surface. This also lets
                // wire_io feed synthetic Notification envelopes for hook
                // dispatch failures and have them rendered as Warnings.
                let is_warning = matches!(
                    level.as_deref(),
                    Some("error") | Some("warning") | Some("warn")
                );
                if is_warning {
                    self.sink.on_semantic_event(SemanticEvent::Warning {
                        message,
                        extra: Value::Object(extra),
                    });
                } else {
                    self.sink.on_semantic_event(SemanticEvent::Info {
                        message,
                        extra: Value::Object(extra),
                    });
                }
            }
            KimiWireEvent::PlanDisplay(plan) => {
                let mut extra = self.info_extra_with_kind(raw_kind, "plan_display");
                if let Some(plan_value) = plan.plan {
                    extra.insert("plan".into(), plan_value);
                }
                if let Some(display) = plan.display {
                    extra.insert("display".into(), display);
                }
                self.sink.on_semantic_event(SemanticEvent::PlanUpdate {
                    message: Some("Plan updated".into()),
                    extra: Value::Object(extra),
                });
            }
            KimiWireEvent::ContentPart(part) => {
                self.handle_content_part(part, raw_kind);
            }
            KimiWireEvent::ToolCall(call) => {
                self.handle_tool_call(call, raw_kind);
            }
            KimiWireEvent::ToolCallPart(part) => {
                self.handle_tool_call_part(part);
            }
            KimiWireEvent::ToolResult(result) => {
                self.flush_pending_tool_call(raw_kind);
                self.handle_tool_result(result, raw_kind);
            }
            KimiWireEvent::ApprovalResponse(echo) => {
                let mut extra = self.info_extra_with_kind(raw_kind, "approval_response");
                if let Some(id) = &echo.request_id {
                    extra.insert("request_id".into(), Value::from(id.as_str()));
                }
                if let Some(response) = &echo.response {
                    extra.insert("response".into(), Value::from(response.as_str()));
                }
                if let Some(feedback) = &echo.feedback
                    && !feedback.is_empty()
                {
                    extra.insert("feedback".into(), Value::from(feedback.as_str()));
                }
                self.sink.on_semantic_event(SemanticEvent::Info {
                    message: match echo.response {
                        Some(response) => format!("Approval echoed: {response}"),
                        None => "Approval echoed".into(),
                    },
                    extra: Value::Object(extra),
                });
            }
            KimiWireEvent::SubagentEvent(sub) => {
                self.flush_pending_tool_call(raw_kind);
                let mut extra = self.info_extra_with_kind(raw_kind, "subagent_event");
                if let Some(parent) = &sub.parent_tool_call_id {
                    extra.insert("parent_tool_call_id".into(), Value::from(parent.as_str()));
                }
                if let Some(agent_id) = &sub.agent_id {
                    extra.insert("agent_id".into(), Value::from(agent_id.as_str()));
                }
                if let Some(subagent_type) = &sub.subagent_type {
                    extra.insert("subagent_type".into(), Value::from(subagent_type.as_str()));
                }
                if let Some(nested) = sub.event.clone() {
                    extra.insert("event".into(), nested);
                }
                let message = match sub.subagent_type.as_deref() {
                    Some(kind) => format!("Subagent event ({kind})"),
                    None => "Subagent event".into(),
                };
                self.sink.on_semantic_event(SemanticEvent::Info {
                    message,
                    extra: Value::Object(extra),
                });
            }
            KimiWireEvent::BtwBegin(begin) => {
                let mut extra = self.info_extra_with_kind(raw_kind, "btw_begin");
                if let Some(topic) = &begin.topic {
                    extra.insert("topic".into(), Value::from(topic.as_str()));
                }
                self.sink.on_semantic_event(SemanticEvent::Info {
                    message: match begin.topic {
                        Some(topic) => format!("By the way: {topic}"),
                        None => "By the way".into(),
                    },
                    extra: Value::Object(extra),
                });
            }
            KimiWireEvent::BtwEnd(_) => {
                let extra = self.info_extra_with_kind(raw_kind, "btw_end");
                self.sink.on_semantic_event(SemanticEvent::Info {
                    message: "By the way (end)".into(),
                    extra: Value::Object(extra),
                });
            }
            KimiWireEvent::HookTriggered(hook) => {
                let mut extra = self.info_extra_with_kind(raw_kind, "hook_triggered");
                if let Some(name) = &hook.hook_name {
                    extra.insert("hook_name".into(), Value::from(name.as_str()));
                }
                if let Some(event) = &hook.event {
                    extra.insert("event".into(), Value::from(event.as_str()));
                }
                self.sink.on_semantic_event(SemanticEvent::Info {
                    message: match (hook.hook_name, hook.event) {
                        (Some(name), Some(event)) => format!("Hook {name} triggered for {event}"),
                        (Some(name), None) => format!("Hook {name} triggered"),
                        (None, Some(event)) => format!("Hook triggered for {event}"),
                        _ => "Hook triggered".into(),
                    },
                    extra: Value::Object(extra),
                });
            }
            KimiWireEvent::HookResolved(hook) => {
                let mut extra = self.info_extra_with_kind(raw_kind, "hook_resolved");
                if let Some(name) = &hook.hook_name {
                    extra.insert("hook_name".into(), Value::from(name.as_str()));
                }
                if let Some(outcome) = &hook.outcome {
                    extra.insert("outcome".into(), Value::from(outcome.as_str()));
                }
                self.sink.on_semantic_event(SemanticEvent::Info {
                    message: match (hook.hook_name, hook.outcome) {
                        (Some(name), Some(outcome)) => format!("Hook {name} resolved: {outcome}"),
                        (Some(name), None) => format!("Hook {name} resolved"),
                        _ => "Hook resolved".into(),
                    },
                    extra: Value::Object(extra),
                });
            }
            KimiWireEvent::DiffDisplayBlock(diff) => {
                let mut extra = self.info_extra_with_kind(raw_kind, "diff_display");
                if let Some(is_summary) = diff.is_summary {
                    extra.insert("is_summary".into(), Value::from(is_summary));
                }
                if let Some(body) = diff.diff {
                    extra.insert("diff".into(), Value::from(body));
                }
                self.sink.on_semantic_event(SemanticEvent::Info {
                    message: "Diff display".into(),
                    extra: Value::Object(extra),
                });
            }
        }
    }

    fn handle_request(&mut self, id: Value, request: KimiWireRequest, raw_kind: &str) {
        match request {
            KimiWireRequest::Approval(approval) => {
                let mut extra = self.info_extra_with_kind(raw_kind, "auto_approved");
                extra.insert("request_id".into(), id);
                if let Some(req_id) = &approval.id {
                    extra.insert("approval_id".into(), Value::from(req_id.as_str()));
                }
                if let Some(tool_call_id) = &approval.tool_call_id {
                    extra.insert("tool_call_id".into(), Value::from(tool_call_id.as_str()));
                }
                if let Some(sender) = &approval.sender {
                    extra.insert("sender".into(), Value::from(sender.as_str()));
                }
                if let Some(action) = &approval.action {
                    extra.insert("action".into(), Value::from(action.as_str()));
                }
                if let Some(description) = &approval.description {
                    extra.insert("description".into(), Value::from(description.as_str()));
                }
                if let Some(shell) = approval.shell_command() {
                    extra.insert("shell_command".into(), Value::from(shell));
                }
                let message = match approval.description {
                    Some(desc) => format!("Auto-approved: {desc}"),
                    None => "Auto-approved tool action".into(),
                };
                self.sink.on_semantic_event(SemanticEvent::Info {
                    message,
                    extra: Value::Object(extra),
                });
            }
            KimiWireRequest::Question(question) => {
                let mut extra = self.info_extra_with_kind(raw_kind, "unexpected_question");
                extra.insert("request_id".into(), id);
                if let Some(prompt) = &question.question {
                    extra.insert("question".into(), Value::from(prompt.as_str()));
                }
                self.sink.on_semantic_event(SemanticEvent::Warning {
                    message: match question.question {
                        Some(prompt) => format!("Unexpected question from agent: {prompt}"),
                        None => "Unexpected question from agent".into(),
                    },
                    extra: Value::Object(extra),
                });
            }
            KimiWireRequest::ToolCall(tool) => {
                let mut extra = self.info_extra_with_kind(raw_kind, "external_tool_request");
                extra.insert("request_id".into(), id);
                if let Some(call) = tool.tool_call {
                    extra.insert("tool_call".into(), call);
                }
                self.sink.on_semantic_event(SemanticEvent::Warning {
                    message: "Unsupported external tool-call request".into(),
                    extra: Value::Object(extra),
                });
            }
            KimiWireRequest::Hook(hook) => {
                let mut extra = self.info_extra_with_kind(raw_kind, "hook_request");
                extra.insert("request_id".into(), id);
                if let Some(event) = &hook.event {
                    extra.insert("event".into(), Value::from(event.as_str()));
                }
                if let Some(context) = hook.context {
                    extra.insert("context".into(), context);
                }
                self.sink.on_semantic_event(SemanticEvent::Info {
                    message: match hook.event {
                        Some(event) => format!("Hook request: {event}"),
                        None => "Hook request".into(),
                    },
                    extra: Value::Object(extra),
                });
            }
        }
    }

    fn handle_status_update(&mut self, status: KimiWireStatusUpdate, raw_kind: &str) {
        let percent = status.computed_context_percent();
        if let Some(usage) = status.token_usage.as_ref() {
            self.merge_token_usage(usage, status.context_tokens);
        }
        let context = ContextUsage {
            used: status.context_tokens,
            total: status.max_context_tokens,
            percent,
        };
        if let Some(pct) = percent
            && pct >= CONTEXT_PRESSURE_WARN_PERCENT
        {
            let mut extra = self.base_extra(raw_kind);
            extra.insert("percent".into(), Value::from(pct));
            extra.insert(
                "used".into(),
                Value::from(status.context_tokens.unwrap_or(0)),
            );
            extra.insert(
                "total".into(),
                Value::from(status.max_context_tokens.unwrap_or(0)),
            );
            self.sink.on_semantic_event(SemanticEvent::Warning {
                message: format!(
                    "Context window pressure: {pct:.0}% used ({}/{} tokens)",
                    status.context_tokens.unwrap_or(0),
                    status.max_context_tokens.unwrap_or(0)
                ),
                extra: Value::Object(extra),
            });
        }
        self.context_usage = Some(context);

        super::trace_summary_update(
            Provider::KimiCode,
            self.provider_status.as_deref(),
            self.duration_ms,
            self.cost_usd,
        );
    }

    fn merge_token_usage(&mut self, usage: &KimiWireTokenUsage, context_tokens: Option<u64>) {
        let input = usage.total_input().or(context_tokens);
        let output = usage.output;
        let cache_read = usage.cache_read_input();
        let total = match (input, output) {
            (Some(i), Some(o)) => Some(i + o),
            _ => None,
        };
        let next = NormalizedTokenUsage {
            input,
            output,
            total,
            cache_read,
        };
        match self.token_usage.as_mut() {
            Some(existing) => existing.merge(&next),
            None => self.token_usage = Some(next),
        }
    }

    fn handle_content_part(&mut self, part: KimiContentPart, raw_kind: &str) {
        if part.is_thinking() {
            self.flush_pending_text();
            let Some(text) = part.resolved_text() else {
                return;
            };
            if text.is_empty() {
                return;
            }
            let extra = self.base_extra(raw_kind);
            self.sink.on_semantic_event(SemanticEvent::Reasoning {
                text: text.to_string(),
                extra: Value::Object(extra),
            });
        } else if part.is_text() {
            let Some(text) = part.resolved_text() else {
                return;
            };
            if text.is_empty() {
                return;
            }
            self.pending_text.push_str(text);
            self.assistant_text.push_str(text);
            let extra = self.base_extra(raw_kind);
            self.sink.on_semantic_event(SemanticEvent::OutputText {
                text: text.to_string(),
                extra: Value::Object(extra),
            });
        } else {
            let mut extra = self.info_extra_with_kind(raw_kind, "content_media");
            extra.insert("part_type".into(), Value::from(part.part_type.as_str()));
            self.sink.on_semantic_event(SemanticEvent::Info {
                message: format!("Content part: {}", part.part_type),
                extra: Value::Object(extra),
            });
        }
    }

    fn flush_pending_text(&mut self) {
        if self.pending_text.is_empty() {
            return;
        }
        if !self.assistant_text.ends_with('\n') {
            self.assistant_text.push('\n');
        }
        self.pending_text.clear();
    }

    fn handle_tool_call(&mut self, mut call: KimiToolCall, raw_kind: &str) {
        // A new ToolCall envelope flushes any in-flight call so accumulated
        // arguments don't leak across tools.
        self.flush_pending_tool_call(raw_kind);
        let id = call.resolved_tool_id().map(str::to_owned);
        let name = call.resolved_tool_name().map(str::to_owned);
        let initial = call.take_arguments_string().unwrap_or_default();
        let slot = self.next_pending_slot;
        self.next_pending_slot = self.next_pending_slot.saturating_add(1);
        self.pending_tool_call = Some(PendingToolCall {
            id,
            name,
            arguments: initial,
            slot,
        });
    }

    fn handle_tool_call_part(&mut self, part: KimiToolCallPart) {
        if let Some(pending) = self.pending_tool_call.as_mut()
            && let Some(delta) = part.arguments_part
        {
            pending.arguments.push_str(&delta);
        }
    }

    fn handle_tool_result(&mut self, mut result: KimiToolResult, raw_kind: &str) {
        let tool_id = result.resolved_tool_id().map(str::to_owned);
        let tool_name = tool_id.as_ref().and_then(|id| self.tool_uses.remove(id));
        let status = result.derived_status().to_string();
        let is_error = result.is_error();
        let output = result.take_output();

        let mut extra = self.base_extra(raw_kind);
        if let Some(id) = &tool_id {
            extra.insert("tool_id".into(), Value::from(id.as_str()));
        }
        if let Some(name) = &tool_name {
            extra.insert("tool_name".into(), Value::from(name.as_str()));
        }
        extra.insert("status".into(), Value::from(status.as_str()));
        if is_error {
            extra.insert("is_error".into(), Value::from(true));
        }

        self.sink.on_semantic_event(SemanticEvent::ToolResult {
            name: tool_name,
            id: tool_id,
            status: Some(status),
            exit_code: None,
            output,
            extra: Value::Object(extra),
        });
    }

    fn emit_provider_extension(&mut self, kind: &str, payload: Value) {
        debug!(
            provider = "kimi",
            event_type = %kind,
            "kimi parser falling back to provider extension for unknown envelope shape"
        );
        self.sink
            .on_semantic_event(SemanticEvent::ProviderExtension {
                provider: Provider::KimiCode,
                kind: kind.to_string(),
                payload,
            });
    }

    fn emit_malformed_warning(&mut self, err: &str) {
        let mut extra = self.base_extra("malformed_json");
        extra.insert("line_num".into(), Value::from(self.line_num));
        self.sink.on_semantic_event(SemanticEvent::Warning {
            message: format!("Malformed JSON on line {}: {err}", self.line_num),
            extra: Value::Object(extra),
        });
    }
}

impl<S: SemanticEventSink> SemanticStreamParser for KimiSemanticStreamParser<S> {
    fn feed_line(&mut self, line: &str) -> Result<(), StreamParseError> {
        self.line_num += 1;
        let line = line.trim();
        if line.is_empty() {
            return Ok(());
        }
        let raw: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(e) => {
                super::trace_malformed_line(Provider::KimiCode, self.line_num, &e.to_string());
                self.emit_malformed_warning(&e.to_string());
                return Ok(());
            }
        };

        let raw_kind = envelope_raw_kind(&raw);
        super::trace_parser_event(Provider::KimiCode, &raw_kind, self.line_num);

        let envelope = match KimiEnvelope::classify(raw.clone()) {
            Some(env) => env,
            None => {
                self.emit_provider_extension(&raw_kind, raw);
                return Ok(());
            }
        };

        match envelope {
            KimiEnvelope::Notification(params) => match params.into_event() {
                Some(event) => self.handle_event(event, &raw_kind),
                None => self.emit_provider_extension(&raw_kind, raw),
            },
            KimiEnvelope::Request { id, params } => match params.into_request() {
                Some(request) => self.handle_request(id, request, &raw_kind),
                None => self.emit_provider_extension(&raw_kind, raw),
            },
            KimiEnvelope::SuccessResponse { id, result } => {
                if id.as_str() == Some("init-1") {
                    self.handle_initialize_response(result);
                } else if id.as_str() == Some("prompt-2") {
                    self.handle_prompt_response(result);
                } else {
                    let mut extra = self.info_extra_with_kind(&raw_kind, "response");
                    extra.insert("response_id".into(), id);
                    if !result.is_null() {
                        extra.insert("result".into(), result);
                    }
                    self.sink.on_semantic_event(SemanticEvent::Info {
                        message: "JSON-RPC response".into(),
                        extra: Value::Object(extra),
                    });
                }
            }
            KimiEnvelope::ErrorResponse { id, error } => {
                let mut extra = self.base_extra(&raw_kind);
                extra.insert("response_id".into(), id);
                self.handle_error_response(error);
                drop(extra);
            }
        }
        Ok(())
    }

    fn finish(mut self: Box<Self>, exit_code: i32) -> StreamExecutionSummary {
        // Flush any in-flight tool call so it is reported even if the stream
        // ended without a `TurnEnd` envelope. Pending text is intentionally
        // *not* flushed here — `flush_pending_text` adds a separator newline
        // between messages, and the trailing assistant message at end-of-run
        // should not gain a trailing newline.
        self.flush_pending_tool_call("finish");
        self.pending_text.clear();
        super::trace_parser_finish(
            Provider::KimiCode,
            exit_code,
            self.tool_calls,
            self.num_turns,
            self.provider_status.as_deref(),
        );
        let mut summary = StreamExecutionSummary {
            provider: Provider::KimiCode,
            session_id: self.session_id,
            model: self.model,
            assistant_text: self.assistant_text,
            provider_status: self.provider_status,
            exit_code,
            is_error: self.is_error,
            error_kind: self.error_kind,
            error_message: self.error_message,
            duration_ms: self.duration_ms,
            duration_api_ms: None,
            num_turns: if self.num_turns > 0 {
                Some(self.num_turns)
            } else {
                None
            },
            token_usage: self.token_usage,
            cost_usd: self.cost_usd,
            tool_calls: if self.tool_calls > 0 {
                Some(self.tool_calls)
            } else {
                None
            },
            permission_prompts: None,
            user_input_prompts: None,
            rate_limit: None,
            context_usage: self.context_usage,
            badges: Vec::new(),
            raw_summary: None,
            stderr_text: None,
            stderr_diagnostics: None,
        };
        let _ = self.prompt_status_seen;
        summary.badges = crate::stream::badges::derive_badges(&summary, Provider::KimiCode);
        summary
    }
}

fn envelope_raw_kind(value: &Value) -> String {
    if let Some(method) = value.get("method").and_then(Value::as_str) {
        let inner = value
            .pointer("/params/type")
            .and_then(Value::as_str)
            .unwrap_or("");
        if inner.is_empty() {
            method.to_string()
        } else {
            format!("{method}:{inner}")
        }
    } else if value.get("result").is_some() {
        "response".into()
    } else if value.get("error").is_some() {
        "error_response".into()
    } else {
        String::new()
    }
}

/// Map a JSON-RPC error code or message to a typed [`SemanticErrorKind`].
fn classify_jsonrpc_error(code: i32, message: &str) -> SemanticErrorKind {
    match code {
        KimiJsonRpcError::AUTH_EXPIRED => return SemanticErrorKind::Configuration,
        KimiJsonRpcError::CHAT_PROVIDER_ERROR => return SemanticErrorKind::ApiRemote,
        KimiJsonRpcError::PARSE_ERROR
        | KimiJsonRpcError::INVALID_REQUEST
        | KimiJsonRpcError::METHOD_NOT_FOUND
        | KimiJsonRpcError::INVALID_PARAMS
        | KimiJsonRpcError::INTERNAL_ERROR => return SemanticErrorKind::AgentNative,
        _ => {}
    }
    let lower = message.to_ascii_lowercase();
    if lower.contains("rate limit")
        || lower.contains("quota")
        || lower.contains("billing")
        || lower.contains("api error")
        || lower.contains("upstream")
    {
        return SemanticErrorKind::ApiRemote;
    }
    if lower.contains("api key")
        || lower.contains("authentication")
        || lower.contains("not authorized")
        || lower.contains("permission denied")
        || lower.contains("auth")
        || lower.contains("config")
    {
        return SemanticErrorKind::Configuration;
    }
    if lower.contains("interrupt") || lower.contains("cancel") || lower.contains("aborted") {
        return SemanticErrorKind::Interrupted;
    }
    SemanticErrorKind::AgentNative
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;

    struct Recording {
        events: Arc<Mutex<Vec<SemanticEvent>>>,
    }
    impl SemanticEventSink for Recording {
        fn on_semantic_event(&mut self, event: SemanticEvent) {
            self.events.lock().unwrap().push(event);
        }
    }

    fn new_parser() -> (
        Arc<Mutex<Vec<SemanticEvent>>>,
        Box<KimiSemanticStreamParser<Recording>>,
    ) {
        let events = Arc::new(Mutex::new(Vec::new()));
        let sink = Recording {
            events: events.clone(),
        };
        (events, Box::new(KimiSemanticStreamParser::new(sink)))
    }

    fn kinds(events: &[SemanticEvent]) -> Vec<&'static str> {
        events.iter().map(|e| e.kind_str()).collect()
    }

    fn feed_initialize(parser: &mut KimiSemanticStreamParser<Recording>) {
        parser
            .feed_line(
                r#"{"jsonrpc":"2.0","id":"init-1","result":{"protocol_version":"1.9","server":{"name":"Kimi Code CLI","version":"1.38.0"},"slash_commands":[],"hooks":[],"capabilities":{"supports_question":true}}}"#,
            )
            .unwrap();
    }

    #[test]
    fn initialize_response_emits_session_start() {
        let (events, mut parser) = new_parser();
        feed_initialize(&mut parser);
        let collected = events.lock().unwrap().clone();
        assert!(matches!(collected[0], SemanticEvent::SessionStart { .. }));
        if let SemanticEvent::SessionStart { extra, model, .. } = &collected[0] {
            assert_eq!(model.as_deref(), Some("Kimi Code CLI"));
            assert_eq!(
                extra.get("protocol_version").and_then(Value::as_str),
                Some("1.9")
            );
        }
    }

    #[test]
    fn turn_begin_emits_turn_start() {
        let (events, mut parser) = new_parser();
        feed_initialize(&mut parser);
        parser
            .feed_line(
                r#"{"jsonrpc":"2.0","method":"event","params":{"type":"TurnBegin","payload":{"user_input":"Hi Bob"}}}"#,
            )
            .unwrap();
        let collected = events.lock().unwrap().clone();
        assert!(
            collected
                .iter()
                .any(|e| matches!(e, SemanticEvent::TurnStart { .. }))
        );
    }

    #[test]
    fn content_part_text_emits_output_text() {
        let (events, mut parser) = new_parser();
        feed_initialize(&mut parser);
        parser
            .feed_line(
                r#"{"jsonrpc":"2.0","method":"event","params":{"type":"ContentPart","payload":{"type":"text","text":"Hello "}}}"#,
            )
            .unwrap();
        parser
            .feed_line(
                r#"{"jsonrpc":"2.0","method":"event","params":{"type":"ContentPart","payload":{"type":"text","text":"world"}}}"#,
            )
            .unwrap();
        let collected = events.lock().unwrap().clone();
        let texts: Vec<&str> = collected
            .iter()
            .filter_map(|e| match e {
                SemanticEvent::OutputText { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(texts, vec!["Hello ", "world"]);
        let summary = parser.finish(0);
        assert_eq!(summary.assistant_text, "Hello world");
    }

    #[test]
    fn content_part_think_emits_reasoning() {
        let (events, mut parser) = new_parser();
        feed_initialize(&mut parser);
        parser
            .feed_line(
                r#"{"jsonrpc":"2.0","method":"event","params":{"type":"ContentPart","payload":{"type":"think","think":"pondering"}}}"#,
            )
            .unwrap();
        assert!(events.lock().unwrap().iter().any(|e| matches!(e,
                SemanticEvent::Reasoning { text, .. } if text == "pondering")));
    }

    #[test]
    fn status_update_above_threshold_emits_warning() {
        let (events, mut parser) = new_parser();
        feed_initialize(&mut parser);
        parser
            .feed_line(
                r#"{"jsonrpc":"2.0","method":"event","params":{"type":"StatusUpdate","payload":{"context_usage":0.85,"context_tokens":110000,"max_context_tokens":128000}}}"#,
            )
            .unwrap();
        let collected = events.lock().unwrap().clone();
        assert!(
            collected
                .iter()
                .any(|e| matches!(e, SemanticEvent::Warning { message, .. } if message.contains("Context window pressure"))),
            "kinds = {:?}",
            kinds(&collected)
        );
    }

    #[test]
    fn status_update_below_threshold_no_warning() {
        let (events, mut parser) = new_parser();
        feed_initialize(&mut parser);
        parser
            .feed_line(
                r#"{"jsonrpc":"2.0","method":"event","params":{"type":"StatusUpdate","payload":{"context_usage":0.06,"context_tokens":15969,"max_context_tokens":262144,"token_usage":{"input_other":10000,"input_cache_read":5000,"output":50}}}}"#,
            )
            .unwrap();
        let collected = events.lock().unwrap().clone();
        assert!(
            !collected
                .iter()
                .any(|e| matches!(e, SemanticEvent::Warning { .. }))
        );
    }

    #[test]
    fn tool_call_with_streamed_arguments_decodes() {
        let (events, mut parser) = new_parser();
        feed_initialize(&mut parser);
        parser
            .feed_line(
                r#"{"jsonrpc":"2.0","method":"event","params":{"type":"ToolCall","payload":{"type":"function","id":"tool_1","function":{"name":"Shell","arguments":""}}}}"#,
            )
            .unwrap();
        for delta in ["{\\\"", "command", "\\\":", " \\\"ls", "\\\"}"] {
            let line = format!(
                r#"{{"jsonrpc":"2.0","method":"event","params":{{"type":"ToolCallPart","payload":{{"arguments_part":"{delta}"}}}}}}"#
            );
            parser.feed_line(&line).unwrap();
        }
        parser
            .feed_line(
                r#"{"jsonrpc":"2.0","method":"event","params":{"type":"ToolResult","payload":{"tool_call_id":"tool_1","return_value":{"is_error":false,"output":"ok"}}}}"#,
            )
            .unwrap();
        let collected = events.lock().unwrap().clone();
        let tool_call = collected
            .iter()
            .find_map(|e| match e {
                SemanticEvent::ToolCall {
                    name, id, input, ..
                } => Some((name.clone(), id.clone(), input.clone())),
                _ => None,
            })
            .expect("tool_call");
        assert_eq!(tool_call.0.as_deref(), Some("Shell"));
        assert_eq!(tool_call.1.as_deref(), Some("tool_1"));
        let input = tool_call.2.expect("input");
        assert_eq!(input.get("command").and_then(Value::as_str), Some("ls"));
        assert!(
            collected
                .iter()
                .any(|e| matches!(e, SemanticEvent::ToolResult { .. }))
        );
    }

    #[test]
    fn malformed_tool_arguments_pass_through_as_string() {
        let (events, mut parser) = new_parser();
        feed_initialize(&mut parser);
        parser
            .feed_line(
                r#"{"jsonrpc":"2.0","method":"event","params":{"type":"ToolCall","payload":{"id":"t1","function":{"name":"Shell","arguments":"{not json"}}}}"#,
            )
            .unwrap();
        parser
            .feed_line(
                r#"{"jsonrpc":"2.0","method":"event","params":{"type":"TurnEnd","payload":{}}}"#,
            )
            .unwrap();
        let collected = events.lock().unwrap().clone();
        let input = collected
            .iter()
            .find_map(|e| match e {
                SemanticEvent::ToolCall { input, .. } => input.clone(),
                _ => None,
            })
            .expect("tool input");
        assert_eq!(input.as_str(), Some("{not json"));
    }

    #[test]
    fn approval_request_emits_auto_approved_info() {
        let (events, mut parser) = new_parser();
        feed_initialize(&mut parser);
        parser
            .feed_line(
                r#"{"jsonrpc":"2.0","method":"request","id":"req-1","params":{"type":"ApprovalRequest","payload":{"id":"req-1","tool_call_id":"t1","sender":"Shell","action":"run command","description":"Run command `ls`","display":[{"type":"shell","language":"bash","command":"ls"}]}}}"#,
            )
            .unwrap();
        let collected = events.lock().unwrap().clone();
        let info = collected
            .iter()
            .find_map(|e| match e {
                SemanticEvent::Info { message, extra } => Some((message.clone(), extra.clone())),
                _ => None,
            })
            .expect("info");
        assert!(info.0.contains("Auto-approved"));
        assert_eq!(
            info.1.get("kind").and_then(Value::as_str),
            Some("auto_approved")
        );
        assert_eq!(
            info.1.get("shell_command").and_then(Value::as_str),
            Some("ls")
        );
    }

    #[test]
    fn unexpected_question_emits_warning() {
        let (events, mut parser) = new_parser();
        feed_initialize(&mut parser);
        parser
            .feed_line(
                r#"{"jsonrpc":"2.0","method":"request","id":"q-1","params":{"type":"QuestionRequest","payload":{"id":"q-1","question":"What now?"}}}"#,
            )
            .unwrap();
        let collected = events.lock().unwrap().clone();
        assert!(
            collected.iter().any(|e| matches!(e,
                SemanticEvent::Warning { message, .. } if message.contains("Unexpected question")))
        );
    }

    #[test]
    fn external_tool_call_request_emits_warning() {
        let (events, mut parser) = new_parser();
        feed_initialize(&mut parser);
        parser
            .feed_line(
                r#"{"jsonrpc":"2.0","method":"request","id":"x-1","params":{"type":"ToolCallRequest","payload":{"id":"x-1","tool_call":{"name":"external"}}}}"#,
            )
            .unwrap();
        let collected = events.lock().unwrap().clone();
        assert!(collected.iter().any(|e| matches!(e,
                SemanticEvent::Warning { message, .. } if message.contains("external tool"))));
    }

    #[test]
    fn hook_request_emits_info() {
        let (events, mut parser) = new_parser();
        feed_initialize(&mut parser);
        parser
            .feed_line(
                r#"{"jsonrpc":"2.0","method":"request","id":"h-1","params":{"type":"HookRequest","payload":{"id":"h-1","event":"PreToolUse","context":{"foo":"bar"}}}}"#,
            )
            .unwrap();
        let collected = events.lock().unwrap().clone();
        let info = collected
            .iter()
            .find_map(|e| match e {
                SemanticEvent::Info { message, extra } => Some((message.clone(), extra.clone())),
                _ => None,
            })
            .expect("info");
        assert!(info.0.contains("Hook request"));
        assert_eq!(
            info.1.get("event").and_then(Value::as_str),
            Some("PreToolUse")
        );
    }

    #[test]
    fn prompt_finished_response_sets_status() {
        let (events, mut parser) = new_parser();
        feed_initialize(&mut parser);
        parser
            .feed_line(r#"{"jsonrpc":"2.0","id":"prompt-2","result":{"status":"finished"}}"#)
            .unwrap();
        let collected = events.lock().unwrap().clone();
        assert!(collected.iter().any(|e| matches!(e,
                SemanticEvent::Info { extra, .. } if extra.get("kind").and_then(Value::as_str) == Some("prompt_status"))));
        let summary = parser.finish(0);
        assert_eq!(summary.provider_status.as_deref(), Some("finished"));
        assert!(!summary.is_error);
    }

    #[test]
    fn prompt_cancelled_response_emits_terminal_error() {
        let (events, mut parser) = new_parser();
        feed_initialize(&mut parser);
        parser
            .feed_line(r#"{"jsonrpc":"2.0","id":"prompt-2","result":{"status":"cancelled"}}"#)
            .unwrap();
        let collected = events.lock().unwrap().clone();
        let err = collected
            .iter()
            .find_map(|e| match e {
                SemanticEvent::Error {
                    message,
                    kind,
                    terminal,
                    ..
                } => Some((message.clone(), *kind, *terminal)),
                _ => None,
            })
            .expect("error");
        assert!(err.2);
        assert_eq!(err.1, SemanticErrorKind::Interrupted);
        assert!(err.0.contains("cancelled"));
        let summary = parser.finish(0);
        assert!(summary.is_error);
        assert_eq!(summary.provider_status.as_deref(), Some("cancelled"));
    }

    #[test]
    fn auth_expired_error_response_classifies_configuration() {
        let (events, mut parser) = new_parser();
        feed_initialize(&mut parser);
        parser
            .feed_line(
                r#"{"jsonrpc":"2.0","id":"prompt-2","error":{"code":-32004,"message":"Authentication expired; please re-authenticate via `kimi login`","data":null}}"#,
            )
            .unwrap();
        let collected = events.lock().unwrap().clone();
        let err = collected
            .iter()
            .find_map(|e| match e {
                SemanticEvent::Error { kind, message, .. } => Some((*kind, message.clone())),
                _ => None,
            })
            .expect("error");
        assert_eq!(err.0, SemanticErrorKind::Configuration);
        assert!(err.1.contains("Authentication"));
    }

    #[test]
    fn chat_provider_error_classifies_api_remote() {
        let kind = classify_jsonrpc_error(KimiJsonRpcError::CHAT_PROVIDER_ERROR, "boom");
        assert_eq!(kind, SemanticErrorKind::ApiRemote);
    }

    #[test]
    fn classify_jsonrpc_error_message_keywords() {
        assert_eq!(
            classify_jsonrpc_error(0, "Rate limit exceeded"),
            SemanticErrorKind::ApiRemote
        );
        assert_eq!(
            classify_jsonrpc_error(0, "user cancelled"),
            SemanticErrorKind::Interrupted
        );
        assert_eq!(
            classify_jsonrpc_error(0, "invalid api key"),
            SemanticErrorKind::Configuration
        );
    }

    #[test]
    fn unknown_envelope_shape_falls_back_to_provider_extension() {
        let (events, mut parser) = new_parser();
        parser.feed_line(r#"{"jsonrpc":"2.0"}"#).unwrap();
        assert!(matches!(
            events.lock().unwrap()[0],
            SemanticEvent::ProviderExtension { .. }
        ));
    }

    #[test]
    fn notification_with_error_level_emits_warning() {
        // Used by wire_io for hook dispatch failures: a synthetic
        // Notification envelope with `level: "error"` must surface as a
        // Warning so the user sees the diagnostic on the live stderr
        // surface, not as a quiet Info line.
        let (events, mut parser) = new_parser();
        feed_initialize(&mut parser);
        parser
            .feed_line(
                r#"{"jsonrpc":"2.0","method":"event","params":{"type":"Notification","payload":{"level":"error","source":"claudine","message":"Hook dispatch failed: boom"}}}"#,
            )
            .unwrap();
        let collected = events.lock().unwrap().clone();
        assert!(
            collected.iter().any(|e| matches!(e,
                SemanticEvent::Warning { message, .. } if message.contains("Hook dispatch failed"))),
            "error-level notification must surface as Warning; got {:?}",
            kinds(&collected)
        );
        assert!(
            !collected.iter().any(|e| matches!(e,
                SemanticEvent::Info { message, .. } if message.contains("Hook dispatch failed"))),
            "error-level notification must NOT also emit Info"
        );
    }

    #[test]
    fn notification_with_warn_level_emits_warning() {
        let (events, mut parser) = new_parser();
        feed_initialize(&mut parser);
        parser
            .feed_line(
                r#"{"jsonrpc":"2.0","method":"event","params":{"type":"Notification","payload":{"level":"warn","message":"Approaching limit"}}}"#,
            )
            .unwrap();
        let collected = events.lock().unwrap().clone();
        assert!(collected.iter().any(|e| matches!(e,
                SemanticEvent::Warning { message, .. } if message.contains("Approaching limit"))));
    }

    #[test]
    fn notification_with_info_level_emits_info() {
        // Default behavior is preserved: info/no-level notifications
        // remain Info events.
        let (events, mut parser) = new_parser();
        feed_initialize(&mut parser);
        parser
            .feed_line(
                r#"{"jsonrpc":"2.0","method":"event","params":{"type":"Notification","payload":{"level":"info","message":"Hello"}}}"#,
            )
            .unwrap();
        let collected = events.lock().unwrap().clone();
        assert!(collected.iter().any(|e| matches!(e,
                SemanticEvent::Info { message, .. } if message.contains("Hello"))));
        assert!(
            !collected
                .iter()
                .any(|e| matches!(e, SemanticEvent::Warning { .. })),
            "info-level notification must not surface as Warning"
        );
    }

    #[test]
    fn known_wire_events_do_not_leak_as_provider_extension() {
        // Phase 5 contract: every Kimi wire event covered by the typed
        // protocol catalog must route through the typed semantic surface
        // (OutputText / Reasoning / ToolCall / ToolResult / Info /
        // Warning / PlanUpdate / TurnStart / TurnComplete / SessionStart),
        // never through the `ProviderExtension` fallback path. Unknown
        // events still fall back, which is verified separately above.
        let (events, mut parser) = new_parser();
        feed_initialize(&mut parser);
        for line in [
            r#"{"jsonrpc":"2.0","method":"event","params":{"type":"TurnBegin","payload":{"user_input":"hi"}}}"#,
            r#"{"jsonrpc":"2.0","method":"event","params":{"type":"StepBegin","payload":{"n":1}}}"#,
            r#"{"jsonrpc":"2.0","method":"event","params":{"type":"ContentPart","payload":{"type":"think","think":"pondering"}}}"#,
            r#"{"jsonrpc":"2.0","method":"event","params":{"type":"ContentPart","payload":{"type":"text","text":"hello"}}}"#,
            r#"{"jsonrpc":"2.0","method":"event","params":{"type":"ToolCall","payload":{"id":"t1","function":{"name":"Shell","arguments":"{\"command\":\"ls\"}"}}}}"#,
            r#"{"jsonrpc":"2.0","method":"event","params":{"type":"ToolCallPart","payload":{"arguments_part":""}}}"#,
            r#"{"jsonrpc":"2.0","method":"event","params":{"type":"ToolResult","payload":{"tool_call_id":"t1","return_value":{"is_error":false,"output":"ok"}}}}"#,
            r#"{"jsonrpc":"2.0","method":"event","params":{"type":"PlanDisplay","payload":{"plan":["step a"]}}}"#,
            r#"{"jsonrpc":"2.0","method":"event","params":{"type":"Notification","payload":{"message":"hi"}}}"#,
            r#"{"jsonrpc":"2.0","method":"event","params":{"type":"StatusUpdate","payload":{"context_usage":0.05,"context_tokens":100,"max_context_tokens":1000}}}"#,
            r#"{"jsonrpc":"2.0","method":"event","params":{"type":"CompactionBegin","payload":{}}}"#,
            r#"{"jsonrpc":"2.0","method":"event","params":{"type":"CompactionEnd","payload":{"tokens_saved":42}}}"#,
            r#"{"jsonrpc":"2.0","method":"event","params":{"type":"MCPLoadingBegin","payload":{"server":"weather"}}}"#,
            r#"{"jsonrpc":"2.0","method":"event","params":{"type":"MCPLoadingEnd","payload":{"server":"weather","status":"ok"}}}"#,
            r#"{"jsonrpc":"2.0","method":"event","params":{"type":"SubagentEvent","payload":{"subagent_type":"explore","event":{}}}}"#,
            r#"{"jsonrpc":"2.0","method":"event","params":{"type":"BtwBegin","payload":{"topic":"hi"}}}"#,
            r#"{"jsonrpc":"2.0","method":"event","params":{"type":"BtwEnd","payload":{}}}"#,
            r#"{"jsonrpc":"2.0","method":"event","params":{"type":"DiffDisplayBlock","payload":{"diff":"+a","is_summary":false}}}"#,
            r#"{"jsonrpc":"2.0","method":"event","params":{"type":"ApprovalResponse","payload":{"request_id":"r1","response":"approve"}}}"#,
            r#"{"jsonrpc":"2.0","method":"event","params":{"type":"SteerInput","payload":{}}}"#,
            r#"{"jsonrpc":"2.0","method":"event","params":{"type":"StepInterrupted","payload":{"reason":"cancel"}}}"#,
            r#"{"jsonrpc":"2.0","method":"event","params":{"type":"TurnEnd","payload":{}}}"#,
            r#"{"jsonrpc":"2.0","method":"request","id":"r1","params":{"type":"ApprovalRequest","payload":{"id":"r1"}}}"#,
            r#"{"jsonrpc":"2.0","method":"request","id":"q1","params":{"type":"QuestionRequest","payload":{"question":"?"}}}"#,
            r#"{"jsonrpc":"2.0","method":"request","id":"x1","params":{"type":"ToolCallRequest","payload":{"id":"x1"}}}"#,
            r#"{"jsonrpc":"2.0","method":"request","id":"h1","params":{"type":"HookRequest","payload":{"event":"PreToolUse"}}}"#,
        ] {
            parser.feed_line(line).unwrap();
        }
        let collected = events.lock().unwrap().clone();
        let leaks: Vec<_> = collected
            .iter()
            .filter_map(|e| match e {
                SemanticEvent::ProviderExtension { kind, .. } => Some(kind.clone()),
                _ => None,
            })
            .collect();
        assert!(
            leaks.is_empty(),
            "known wire events must not surface as ProviderExtension; got leaks: {leaks:?}"
        );
    }

    #[test]
    fn unknown_event_type_emits_provider_extension_with_event_kind() {
        // Unknown event types must still surface so operators can see
        // protocol drift, but the live sink's silent allowlist will
        // suppress the high-volume ones (see live_semantic_sink.rs).
        let (events, mut parser) = new_parser();
        parser
            .feed_line(
                r#"{"jsonrpc":"2.0","method":"event","params":{"type":"FutureKimiEvent","payload":{"x":1}}}"#,
            )
            .unwrap();
        let collected = events.lock().unwrap().clone();
        let ext_kind = collected
            .iter()
            .find_map(|e| match e {
                SemanticEvent::ProviderExtension { kind, .. } => Some(kind.clone()),
                _ => None,
            })
            .expect("ProviderExtension fallback must fire for unknown event types");
        assert_eq!(ext_kind, "event:FutureKimiEvent");
    }

    #[test]
    fn malformed_json_emits_warning() {
        let (events, mut parser) = new_parser();
        assert!(parser.feed_line("x").is_ok());
        assert!(matches!(
            events.lock().unwrap()[0],
            SemanticEvent::Warning { .. }
        ));
    }

    #[test]
    fn turn_end_increments_num_turns_and_emits_turn_complete() {
        let (events, mut parser) = new_parser();
        feed_initialize(&mut parser);
        parser
            .feed_line(
                r#"{"jsonrpc":"2.0","method":"event","params":{"type":"TurnEnd","payload":{}}}"#,
            )
            .unwrap();
        let collected = events.lock().unwrap().clone();
        assert!(
            collected
                .iter()
                .any(|e| matches!(e, SemanticEvent::TurnComplete { .. }))
        );
        let summary = parser.finish(0);
        assert_eq!(summary.num_turns, Some(1));
    }

    #[test]
    fn round_trip_fidelity_for_emitted_events() {
        let (events, mut parser) = new_parser();
        for line in [
            r#"{"jsonrpc":"2.0","id":"init-1","result":{"protocol_version":"1.9","server":{"name":"Kimi Code CLI","version":"1.38.0"},"slash_commands":[],"hooks":[],"capabilities":{"supports_question":true}}}"#,
            r#"{"jsonrpc":"2.0","method":"event","params":{"type":"TurnBegin","payload":{"user_input":"hi"}}}"#,
            r#"{"jsonrpc":"2.0","method":"event","params":{"type":"ContentPart","payload":{"type":"text","text":"hi"}}}"#,
            r#"{"jsonrpc":"2.0","method":"event","params":{"type":"TurnEnd","payload":{}}}"#,
            r#"{"jsonrpc":"2.0","id":"prompt-2","result":{"status":"finished"}}"#,
        ] {
            parser.feed_line(line).unwrap();
        }
        for event in events.lock().unwrap().iter() {
            let v = serde_json::to_value(event).unwrap();
            let decoded: SemanticEvent = serde_json::from_value(v.clone()).unwrap();
            assert_eq!(v, serde_json::to_value(&decoded).unwrap());
        }
    }
}
