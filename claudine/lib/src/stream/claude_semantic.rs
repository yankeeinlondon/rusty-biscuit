//! Native [`SemanticStreamParser`] implementation for Claude Code's
//! `stream-json` format.
//!
//! This parser emits typed [`SemanticEvent`]s directly:
//!
//! - Assistant text deltas → [`SemanticEvent::OutputText`].
//! - Thinking deltas → [`SemanticEvent::Reasoning`].
//! - `tool_use` → [`SemanticEvent::ToolCall`]; `tool_result` → [`SemanticEvent::ToolResult`].
//! - `rate_limit_event` / billing / auth errors → typed [`SemanticEvent::Warning`] or [`SemanticEvent::Error`].
//! - `task_*` sub-agent events → [`SemanticEvent::SubagentStart`] / [`SemanticEvent::SubagentStop`] / [`SemanticEvent::Info`].
//! - Anything else that is still valid JSON is preserved as
//!   [`SemanticEvent::ProviderExtension`] rather than silently dropped.

use serde_json::{Map, Value};

use super::parser::{SemanticStreamParser, StreamParseError};
use super::protocol::claude::{
    ClaudeAssistant, ClaudeContentBlockDelta, ClaudeContentBlockStart, ClaudeErrorEvent,
    ClaudeEvent, ClaudeInit, ClaudeRateLimit, ClaudeResult, ClaudeToolResult, ClaudeToolUse,
    ClaudeUser,
};
use super::semantic::{SemanticEvent, SemanticEventSink};
use super::summary::{RateLimitInfo, StreamExecutionSummary};
use super::token_usage::NormalizedTokenUsage;
use crate::events::Provider;

/// Max number of hook events to buffer before `SessionStart` is emitted.
/// If the buffer grows past this, we flush early so live streaming wins
/// over cosmetic ordering (spec: "preserving streaming wins over
/// cosmetic ordering").
const MAX_PRE_INIT_HOOK_EVENTS: usize = 32;

/// Known raw event `type` strings that are NOT modeled by [`ClaudeEvent`] but
/// should still map to specific semantic events rather than
/// [`SemanticEvent::ProviderExtension`]. Kept alongside the parser so the
/// allowlist is obvious during review.
mod allowlist {
    pub const TASK_STARTED: &str = "task_started";
    pub const TASK_PROGRESS: &str = "task_progress";
    pub const TASK_NOTIFICATION: &str = "task_notification";
    pub const TASK_COMPLETED: &str = "task_completed";
    pub const SYSTEM_API_RETRY: &str = "system/api_retry";
}

/// Native stream parser for Claude Code emitting [`SemanticEvent`]s.
pub struct ClaudeSemanticStreamParser<S: SemanticEventSink> {
    sink: S,
    line_num: usize,
    // Session state
    session_id: Option<String>,
    model: Option<String>,
    // Accumulated assistant output
    assistant_text: String,
    // Summary state
    token_usage: Option<NormalizedTokenUsage>,
    cost_usd: Option<f64>,
    duration_ms: Option<u64>,
    duration_api_ms: Option<u64>,
    num_turns: Option<u32>,
    tool_calls: u32,
    provider_status: Option<String>,
    is_error: bool,
    error_kind: Option<String>,
    error_message: Option<String>,
    rate_limit: Option<RateLimitInfo>,
    raw_summary: Option<Value>,
    /// Tracks whether a terminal `SemanticEvent::Error` has already been
    /// emitted so that an `assistant.error` followed by a
    /// `result.is_error=true` pair does not double-emit.
    terminal_error_emitted: bool,
    /// Set once `SessionStart` has been emitted. Hook events that arrive
    /// before this flips are buffered so they trail the session-ID marker.
    session_started: bool,
    /// Buffered hook events (raw kind + raw value) that arrived prior to
    /// `SessionStart`. Drained in FIFO order once session_start is emitted.
    pre_init_hook_buffer: Vec<(String, Value)>,
}

impl<S: SemanticEventSink> ClaudeSemanticStreamParser<S> {
    pub fn new(sink: S) -> Self {
        Self {
            sink,
            line_num: 0,
            session_id: None,
            model: None,
            assistant_text: String::new(),
            token_usage: None,
            cost_usd: None,
            duration_ms: None,
            duration_api_ms: None,
            num_turns: None,
            tool_calls: 0,
            provider_status: None,
            is_error: false,
            error_kind: None,
            error_message: None,
            rate_limit: None,
            raw_summary: None,
            terminal_error_emitted: false,
            session_started: false,
            pre_init_hook_buffer: Vec::new(),
        }
    }

    fn base_extra(&self) -> Map<String, Value> {
        let mut m = Map::new();
        m.insert("provider".into(), Value::from("claude"));
        m.insert("line_num".into(), Value::from(self.line_num));
        m
    }

    fn extra_with(&self, raw_kind: &str) -> Value {
        let mut m = self.base_extra();
        m.insert("raw_kind".into(), Value::from(raw_kind));
        Value::Object(m)
    }

    fn handle_init(&mut self, init: ClaudeInit, raw_kind: &str, raw: &Value) {
        // `system` events carry a `subtype` discriminator. Only `init` (and
        // `null`/missing — legacy `init` envelopes) establish session state
        // and emit a `SessionStart`. Everything else (notably `hook_started`
        // and `hook_response`) is surfaced as a `ProviderExtension`. Hook
        // events that arrive BEFORE `SessionStart` are buffered so the
        // session-ID marker renders first (per the response-refinement
        // spec). If the buffer saturates we flush inline to preserve live
        // streaming.
        let subtype = init.subtype.as_deref();
        let is_session_init =
            raw_kind == "init" || matches!(subtype, None | Some("init") | Some(""));
        if !is_session_init {
            let ext_kind = match subtype {
                Some(s) if !s.is_empty() => format!("system/{s}"),
                _ => "system".to_string(),
            };
            if !self.session_started
                && self.pre_init_hook_buffer.len() < MAX_PRE_INIT_HOOK_EVENTS
            {
                self.pre_init_hook_buffer.push((ext_kind, raw.clone()));
            } else {
                self.emit_provider_extension(&ext_kind, raw.clone());
            }
            return;
        }
        self.session_id = init.session_id;
        self.model = init.model;
        super::trace_session_metadata(
            Provider::Claude,
            self.session_id.as_deref(),
            self.model.as_deref(),
        );
        self.sink.on_semantic_event(SemanticEvent::SessionStart {
            session_id: self.session_id.clone(),
            model: self.model.clone(),
            extra: self.extra_with(raw_kind),
        });
        self.session_started = true;
        // Drain any hook events that arrived before the session-ID marker so
        // they render after it.
        let drained: Vec<(String, Value)> = std::mem::take(&mut self.pre_init_hook_buffer);
        for (ext_kind, payload) in drained {
            self.emit_provider_extension(&ext_kind, payload);
        }
    }

    fn handle_assistant_message(&mut self, event: ClaudeAssistant, raw_kind: &str) {
        // Newer Claude Code releases surface synthetic failure turns as an
        // `assistant` envelope with a top-level `error` discriminator. Treat
        // these as terminal errors and do not mingle their synthetic text
        // with real assistant output.
        let error_kind = event.error.clone();
        // Claude Code wraps content as {"message":{"content":[...]}} while the
        // simplified test format uses {"content":[...]} at the top level.
        let Some(content) = event.message.and_then(|m| m.content).or(event.content) else {
            return;
        };
        let mut text_parts = String::new();
        for part in content {
            if part.kind.as_deref() == Some("text")
                && let Some(text) = part.text
            {
                text_parts.push_str(&text);
            }
        }

        if let Some(kind) = error_kind {
            self.is_error = true;
            self.error_kind = Some(kind.clone());
            let message = if !text_parts.is_empty() {
                text_parts
            } else {
                format!("Claude reported {kind}")
            };
            self.error_message = Some(message.clone());

            if !self.terminal_error_emitted {
                let mut extra = self.base_extra();
                extra.insert("raw_kind".into(), Value::from(raw_kind));
                extra.insert("error_kind".into(), Value::from(kind.as_str()));
                self.sink.on_semantic_event(SemanticEvent::Error {
                    message,
                    terminal: true,
                    extra: Value::Object(extra),
                });
                self.terminal_error_emitted = true;
            }
            return;
        }

        if text_parts.is_empty() {
            return;
        }
        self.assistant_text.push_str(&text_parts);
        self.sink.on_semantic_event(SemanticEvent::OutputText {
            text: super::ensure_message_newline(text_parts),
            extra: self.extra_with(raw_kind),
        });
    }

    fn handle_content_block_delta(&mut self, event: ClaudeContentBlockDelta, raw_kind: &str) {
        let Some(delta) = event.delta else {
            return;
        };
        match delta.kind.as_deref() {
            Some("text_delta") => {
                if let Some(text) = delta.text {
                    self.assistant_text.push_str(&text);
                    let mut extra = self.base_extra();
                    extra.insert("raw_kind".into(), Value::from(raw_kind));
                    extra.insert("delta_kind".into(), Value::from("text_delta"));
                    self.sink.on_semantic_event(SemanticEvent::OutputText {
                        text,
                        extra: Value::Object(extra),
                    });
                }
            }
            Some("thinking_delta") => {
                if let Some(text) = delta.thinking {
                    let mut extra = self.base_extra();
                    extra.insert("raw_kind".into(), Value::from(raw_kind));
                    extra.insert("delta_kind".into(), Value::from("thinking_delta"));
                    self.sink.on_semantic_event(SemanticEvent::Reasoning {
                        text,
                        extra: Value::Object(extra),
                    });
                }
            }
            _ => {
                // input_json_delta and other deltas fall through — preserve as
                // a ProviderExtension so partial JSON stream data isn't lost.
                let mut payload = Map::new();
                if let Some(kind) = delta.kind {
                    payload.insert("type".into(), Value::from(kind));
                }
                if let Some(text) = delta.partial_json {
                    payload.insert("partial_json".into(), Value::from(text));
                }
                self.sink.on_semantic_event(SemanticEvent::ProviderExtension {
                    provider: Provider::Claude,
                    kind: "content_block_delta.other".into(),
                    payload: Value::Object(payload),
                });
            }
        }
    }

    fn handle_content_block_start(&mut self, cbs: ClaudeContentBlockStart, raw_kind: &str) {
        if let Some(block) = cbs.content_block {
            if block.kind.as_deref() == Some("tool_use") {
                self.handle_tool_use(block.into_tool_use(), raw_kind);
            } else {
                // Non-tool content blocks (text, image) don't carry data here;
                // they show up via content_block_delta. Skip without logging.
            }
        }
    }

    fn handle_error(&mut self, event: ClaudeErrorEvent, raw_kind: &str) {
        self.is_error = true;
        let detail = event.error;
        self.error_kind = detail.as_ref().and_then(|d| d.kind.clone());
        self.error_message = detail.and_then(|d| d.message).clone();

        let mut extra = self.base_extra();
        extra.insert("raw_kind".into(), Value::from(raw_kind));
        if let Some(kind) = &self.error_kind {
            extra.insert("error_kind".into(), Value::from(kind.as_str()));
        }

        self.sink.on_semantic_event(SemanticEvent::Error {
            message: self.error_message.clone().unwrap_or_default(),
            terminal: true,
            extra: Value::Object(extra),
        });
        self.terminal_error_emitted = true;
    }

    fn handle_result(&mut self, result: ClaudeResult, raw: Value, raw_kind: &str) {
        self.duration_ms = result.duration_ms;
        self.duration_api_ms = result.duration_api_ms;
        self.num_turns = result.num_turns.map(|v| v as u32);
        self.provider_status = result.stop_reason.clone();
        self.cost_usd = result.effective_cost_usd();

        // Surface a terminal `result.is_error=true` as a typed semantic
        // Error when no upstream failure (assistant.error / error event) has
        // already emitted one. Without this, a session that fails only at
        // the terminal envelope appears successful until callers inspect the
        // summary's `is_error` flag.
        if result.is_error.unwrap_or(false) {
            self.is_error = true;
            let result_text = result.result.clone();
            if self.error_message.is_none() {
                self.error_message = result_text.clone();
            }
            if !self.terminal_error_emitted {
                let message = result_text
                    .or_else(|| self.error_message.clone())
                    .unwrap_or_else(|| "Claude session reported failure".to_string());
                let mut extra = self.base_extra();
                extra.insert("raw_kind".into(), Value::from(raw_kind));
                if let Some(kind) = self.error_kind.as_deref() {
                    extra.insert("error_kind".into(), Value::from(kind));
                }
                if let Some(reason) = result.terminal_reason.as_deref() {
                    extra.insert("terminal_reason".into(), Value::from(reason));
                }
                self.sink.on_semantic_event(SemanticEvent::Error {
                    message,
                    terminal: true,
                    extra: Value::Object(extra),
                });
                self.terminal_error_emitted = true;
            }
        }

        if let Some(usage) = &result.usage {
            let input = usage.input_tokens;
            let output = usage.output_tokens;
            let cache_read = usage.cache_read_input_tokens;
            let total = match (input, output) {
                (Some(i), Some(o)) => Some(i + o),
                _ => None,
            };
            self.token_usage = Some(NormalizedTokenUsage {
                input,
                output,
                total,
                cache_read,
            });
        }

        // Strip large arrays from the raw summary so SQLite ingest stays lean.
        let mut raw = raw;
        if let Some(map) = raw.as_object_mut() {
            map.remove("tools");
            map.remove("skills");
            map.remove("agents");
            map.remove("mcp_servers");
        }
        self.raw_summary = Some(raw);

        super::trace_summary_update(
            Provider::Claude,
            self.provider_status.as_deref(),
            self.duration_ms,
            self.cost_usd,
        );

        self.sink.on_semantic_event(SemanticEvent::TurnComplete {
            provider_status: self.provider_status.clone(),
            token_usage: self.token_usage.clone(),
            cost_usd: self.cost_usd,
            duration_ms: self.duration_ms,
            extra: self.extra_with(raw_kind),
        });
    }

    fn handle_rate_limit(&mut self, event: ClaudeRateLimit, raw_kind: &str) {
        let info = RateLimitInfo {
            is_throttled: event.is_throttled,
            retry_after_ms: event.retry_after_ms,
            message: event.message.clone(),
        };

        let mut extra = self.base_extra();
        extra.insert("raw_kind".into(), Value::from(raw_kind));
        if let Some(is_throttled) = info.is_throttled {
            extra.insert("is_throttled".into(), Value::from(is_throttled));
        }
        if let Some(retry_after_ms) = info.retry_after_ms {
            extra.insert("retry_after_ms".into(), Value::from(retry_after_ms));
        }

        let message = info
            .message
            .clone()
            .unwrap_or_else(|| "rate limit".to_string());
        self.rate_limit = Some(info);

        self.sink.on_semantic_event(SemanticEvent::Warning {
            message,
            extra: Value::Object(extra),
        });
    }

    fn handle_tool_use(&mut self, mut tu: ClaudeToolUse, raw_kind: &str) {
        self.tool_calls += 1;
        let tool_id = tu.resolved_tool_id().map(String::from);
        let tool_name = tu.resolved_tool_name().map(String::from);
        let tool_input = tu.take_input();
        super::trace_tool_event(Provider::Claude, self.tool_calls, tool_name.as_deref());

        let mut extra = self.base_extra();
        extra.insert("raw_kind".into(), Value::from(raw_kind));
        if let Some(id) = &tool_id {
            extra.insert("tool_id".into(), Value::from(id.as_str()));
        }
        if let Some(name) = &tool_name {
            extra.insert("tool_name".into(), Value::from(name.as_str()));
        }

        self.sink.on_semantic_event(SemanticEvent::ToolCall {
            name: tool_name,
            id: tool_id,
            input: tool_input,
            extra: Value::Object(extra),
        });
    }

    fn handle_tool_result(&mut self, tr: ClaudeToolResult, raw_kind: &str) {
        let tool_id = tr.resolved_tool_id().map(String::from);

        let mut extra = self.base_extra();
        extra.insert("raw_kind".into(), Value::from(raw_kind));
        if let Some(id) = &tool_id {
            extra.insert("tool_id".into(), Value::from(id.as_str()));
        }

        let output = tr.response();

        self.sink.on_semantic_event(SemanticEvent::ToolResult {
            name: None,
            id: tool_id,
            status: None,
            exit_code: None,
            output,
            extra: Value::Object(extra),
        });
    }

    fn handle_user(&mut self, user: ClaudeUser, raw_kind: &str) {
        let Some(content) = user.message.and_then(|m| m.content) else {
            return;
        };
        for block in content {
            let block_kind = block
                .get("type")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            match block_kind.as_str() {
                "tool_result" => {
                    let tool_id = block
                        .get("tool_use_id")
                        .and_then(Value::as_str)
                        .or_else(|| block.get("id").and_then(Value::as_str))
                        .map(String::from);
                    let output = block
                        .get("content")
                        .cloned()
                        .or_else(|| block.get("output").cloned())
                        .or_else(|| block.get("result").cloned());
                    let is_error = block
                        .get("is_error")
                        .and_then(Value::as_bool)
                        .unwrap_or(false);
                    let status = if is_error {
                        Some("error".to_string())
                    } else {
                        Some("success".to_string())
                    };

                    let mut extra = self.base_extra();
                    extra.insert("raw_kind".into(), Value::from(raw_kind));
                    extra.insert("source".into(), Value::from("user.tool_result"));
                    if let Some(id) = &tool_id {
                        extra.insert("tool_id".into(), Value::from(id.as_str()));
                    }

                    self.sink.on_semantic_event(SemanticEvent::ToolResult {
                        name: None,
                        id: tool_id,
                        status,
                        exit_code: None,
                        output,
                        extra: Value::Object(extra),
                    });
                }
                "text" => {
                    // User turn replay text — intentionally dropped; it only
                    // echoes the prompt back and would pollute assistant
                    // output.
                }
                other => {
                    let kind_label = if other.is_empty() {
                        "user.unknown".to_string()
                    } else {
                        format!("user.{other}")
                    };
                    self.sink.on_semantic_event(SemanticEvent::ProviderExtension {
                        provider: Provider::Claude,
                        kind: kind_label,
                        payload: block,
                    });
                }
            }
        }
    }

    fn handle_unknown_known_kind(&mut self, kind: &str, raw: Value) {
        match kind {
            allowlist::TASK_STARTED => {
                let name = raw
                    .get("name")
                    .or_else(|| raw.get("task_name"))
                    .and_then(Value::as_str)
                    .map(String::from);
                let id = raw
                    .get("task_id")
                    .or_else(|| raw.get("id"))
                    .and_then(Value::as_str)
                    .map(String::from);
                self.sink.on_semantic_event(SemanticEvent::SubagentStart {
                    name,
                    id,
                    extra: raw,
                });
            }
            allowlist::TASK_COMPLETED => {
                let name = raw
                    .get("name")
                    .or_else(|| raw.get("task_name"))
                    .and_then(Value::as_str)
                    .map(String::from);
                let id = raw
                    .get("task_id")
                    .or_else(|| raw.get("id"))
                    .and_then(Value::as_str)
                    .map(String::from);
                let status = raw
                    .get("status")
                    .and_then(Value::as_str)
                    .map(String::from);
                self.sink.on_semantic_event(SemanticEvent::SubagentStop {
                    name,
                    id,
                    status,
                    extra: raw,
                });
            }
            allowlist::TASK_PROGRESS | allowlist::TASK_NOTIFICATION => {
                let message = raw
                    .get("message")
                    .and_then(Value::as_str)
                    .map(String::from)
                    .unwrap_or_else(|| kind.to_string());
                self.sink.on_semantic_event(SemanticEvent::Info {
                    message,
                    extra: raw,
                });
            }
            allowlist::SYSTEM_API_RETRY => {
                let message = raw
                    .get("message")
                    .and_then(Value::as_str)
                    .map(String::from)
                    .unwrap_or_else(|| "api_retry".to_string());
                self.sink.on_semantic_event(SemanticEvent::Warning {
                    message,
                    extra: raw,
                });
            }
            _ => unreachable!("caller must match allowlist"),
        }
    }

    fn is_known_kind(kind: &str) -> bool {
        matches!(
            kind,
            allowlist::TASK_STARTED
                | allowlist::TASK_PROGRESS
                | allowlist::TASK_NOTIFICATION
                | allowlist::TASK_COMPLETED
                | allowlist::SYSTEM_API_RETRY
        )
    }

    fn emit_provider_extension(&mut self, kind: &str, payload: Value) {
        self.sink.on_semantic_event(SemanticEvent::ProviderExtension {
            provider: Provider::Claude,
            kind: kind.to_string(),
            payload,
        });
    }

    fn emit_malformed_warning(&mut self, err: &str) {
        let mut extra = self.base_extra();
        extra.insert("raw_kind".into(), Value::from("malformed_json"));
        self.sink.on_semantic_event(SemanticEvent::Warning {
            message: format!("Malformed JSON on line {}: {err}", self.line_num),
            extra: Value::Object(extra),
        });
    }
}

impl<S: SemanticEventSink> SemanticStreamParser for ClaudeSemanticStreamParser<S> {
    fn feed_line(&mut self, line: &str) -> Result<(), StreamParseError> {
        self.line_num += 1;
        let line = line.trim();
        if line.is_empty() {
            return Ok(());
        }

        // Parse as Value first so we preserve the raw payload for
        // `ProviderExtension` and the `result` raw summary.
        let raw: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(e) => {
                super::trace_malformed_line(Provider::Claude, self.line_num, &e.to_string());
                self.emit_malformed_warning(&e.to_string());
                return Ok(());
            }
        };

        let raw_kind = raw
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        super::trace_parser_event(Provider::Claude, &raw_kind, self.line_num);

        // Typed dispatch first; known-but-untyped and shape-drifted events
        // fall through to allowlist / ProviderExtension below.
        match serde_json::from_value::<ClaudeEvent>(raw.clone()) {
            Ok(ClaudeEvent::Init(init) | ClaudeEvent::System(init)) => {
                self.handle_init(init, &raw_kind, &raw);
            }
            Ok(ClaudeEvent::Assistant(assistant)) => {
                self.handle_assistant_message(assistant, &raw_kind);
            }
            Ok(ClaudeEvent::User(user)) => {
                self.handle_user(user, &raw_kind);
            }
            Ok(ClaudeEvent::ContentBlockStart(cbs)) => {
                self.handle_content_block_start(cbs, &raw_kind);
            }
            Ok(ClaudeEvent::ContentBlockDelta(d)) => {
                self.handle_content_block_delta(d, &raw_kind);
            }
            Ok(ClaudeEvent::Error(err) | ClaudeEvent::AssistantError(err)) => {
                self.handle_error(err, &raw_kind);
            }
            Ok(ClaudeEvent::Result(result)) => {
                self.handle_result(result, raw, &raw_kind);
            }
            Ok(ClaudeEvent::RateLimit(rl)) => {
                self.handle_rate_limit(rl, &raw_kind);
            }
            Ok(ClaudeEvent::ToolUse(tu)) => {
                self.handle_tool_use(tu, &raw_kind);
            }
            Ok(ClaudeEvent::ToolResult(tr)) => {
                self.handle_tool_result(tr, &raw_kind);
            }
            Err(_) => {
                if Self::is_known_kind(&raw_kind) {
                    self.handle_unknown_known_kind(&raw_kind, raw);
                } else {
                    self.emit_provider_extension(&raw_kind, raw);
                }
            }
        }

        Ok(())
    }

    fn finish(self: Box<Self>, exit_code: i32) -> StreamExecutionSummary {
        let mut summary = StreamExecutionSummary {
            provider: Provider::Claude,
            session_id: self.session_id,
            model: self.model,
            assistant_text: self.assistant_text,
            provider_status: self.provider_status,
            exit_code,
            is_error: self.is_error,
            error_kind: self.error_kind,
            error_message: self.error_message,
            duration_ms: self.duration_ms,
            duration_api_ms: self.duration_api_ms,
            num_turns: self.num_turns,
            token_usage: self.token_usage,
            cost_usd: self.cost_usd,
            tool_calls: if self.tool_calls > 0 {
                Some(self.tool_calls)
            } else {
                None
            },
            permission_prompts: None,
            user_input_prompts: None,
            rate_limit: self.rate_limit,
            context_usage: None,
            badges: Vec::new(),
            raw_summary: self.raw_summary,
            stderr_text: None,
        };
        summary.badges = crate::stream::badges::derive_badges(&summary, Provider::Claude);
        summary
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;
    use serde_json::json;

    /// Recording sink that collects every emitted semantic event.
    struct RecordingSink {
        events: Arc<Mutex<Vec<SemanticEvent>>>,
    }

    impl RecordingSink {
        fn new() -> Self {
            Self {
                events: Arc::new(Mutex::new(Vec::new())),
            }
        }
        fn snapshot(&self) -> Vec<SemanticEvent> {
            self.events.lock().unwrap().clone()
        }
        fn kinds(&self) -> Vec<&'static str> {
            self.events
                .lock()
                .unwrap()
                .iter()
                .map(|e| e.kind_str())
                .collect()
        }
    }

    impl SemanticEventSink for RecordingSink {
        fn on_semantic_event(&mut self, event: SemanticEvent) {
            self.events.lock().unwrap().push(event);
        }
    }

    fn new_parser() -> (RecordingSink, Box<ClaudeSemanticStreamParser<RecordingSink>>) {
        let sink = RecordingSink::new();
        let sink_shared = RecordingSink {
            events: sink.events.clone(),
        };
        let parser = Box::new(ClaudeSemanticStreamParser::new(sink_shared));
        (sink, parser)
    }

    #[test]
    fn init_emits_session_start() {
        let (sink, mut parser) = new_parser();
        parser
            .feed_line(r#"{"type":"init","session_id":"s1","model":"claude"}"#)
            .unwrap();
        let events = sink.snapshot();
        assert_eq!(events.len(), 1);
        match &events[0] {
            SemanticEvent::SessionStart { session_id, model, .. } => {
                assert_eq!(session_id.as_deref(), Some("s1"));
                assert_eq!(model.as_deref(), Some("claude"));
            }
            other => panic!("expected SessionStart, got {other:?}"),
        }
    }

    #[test]
    fn assistant_text_emits_output_text_and_accumulates() {
        let (sink, mut parser) = new_parser();
        parser
            .feed_line(r#"{"type":"assistant","content":[{"type":"text","text":"Hello"}]}"#)
            .unwrap();
        parser
            .feed_line(r#"{"type":"assistant","content":[{"type":"text","text":", world"}]}"#)
            .unwrap();
        let kinds = sink.kinds();
        assert_eq!(kinds, vec!["output_text", "output_text"]);
        let summary = parser.finish(0);
        assert_eq!(summary.assistant_text, "Hello, world");
    }

    #[test]
    fn thinking_delta_emits_reasoning() {
        let (sink, mut parser) = new_parser();
        parser
            .feed_line(
                r#"{"type":"content_block_delta","delta":{"type":"thinking_delta","thinking":"pondering"}}"#,
            )
            .unwrap();
        let events = sink.snapshot();
        assert!(matches!(
            events[0],
            SemanticEvent::Reasoning { ref text, .. } if text == "pondering"
        ));
        // Thinking must NOT contribute to assistant_text.
        let summary = parser.finish(0);
        assert_eq!(summary.assistant_text, "");
    }

    #[test]
    fn text_delta_emits_output_text() {
        let (sink, mut parser) = new_parser();
        parser
            .feed_line(
                r#"{"type":"content_block_delta","delta":{"type":"text_delta","text":"Hi"}}"#,
            )
            .unwrap();
        let events = sink.snapshot();
        assert!(matches!(
            events[0],
            SemanticEvent::OutputText { ref text, .. } if text == "Hi"
        ));
    }

    #[test]
    fn tool_use_and_result_emit_typed_events() {
        let (sink, mut parser) = new_parser();
        parser
            .feed_line(
                r#"{"type":"tool_use","id":"t1","name":"bash","input":{"cmd":"ls"}}"#,
            )
            .unwrap();
        parser
            .feed_line(r#"{"type":"tool_result","tool_use_id":"t1","content":"ok"}"#)
            .unwrap();

        let events = sink.snapshot();
        assert_eq!(events.len(), 2);
        match &events[0] {
            SemanticEvent::ToolCall { name, id, input, .. } => {
                assert_eq!(name.as_deref(), Some("bash"));
                assert_eq!(id.as_deref(), Some("t1"));
                assert_eq!(input, &Some(json!({"cmd": "ls"})));
            }
            other => panic!("expected ToolCall, got {other:?}"),
        }
        match &events[1] {
            SemanticEvent::ToolResult { id, output, .. } => {
                assert_eq!(id.as_deref(), Some("t1"));
                assert_eq!(output, &Some(json!("ok")));
            }
            other => panic!("expected ToolResult, got {other:?}"),
        }

        let summary = parser.finish(0);
        assert_eq!(summary.tool_calls, Some(1));
    }

    #[test]
    fn content_block_start_tool_use_dispatches_as_tool_call() {
        let (sink, mut parser) = new_parser();
        parser
            .feed_line(
                r#"{"type":"content_block_start","content_block":{"type":"tool_use","id":"t2","name":"bash","input":{"cmd":"ls -la"}}}"#,
            )
            .unwrap();
        let events = sink.snapshot();
        assert!(matches!(events[0], SemanticEvent::ToolCall { .. }));
    }

    #[test]
    fn rate_limit_emits_warning() {
        let (sink, mut parser) = new_parser();
        parser
            .feed_line(
                r#"{"type":"rate_limit_event","is_throttled":true,"retry_after_ms":5000,"message":"Rate limited"}"#,
            )
            .unwrap();
        let events = sink.snapshot();
        match &events[0] {
            SemanticEvent::Warning { message, extra } => {
                assert_eq!(message, "Rate limited");
                assert_eq!(extra.get("retry_after_ms"), Some(&json!(5000)));
            }
            other => panic!("expected Warning, got {other:?}"),
        }
        let summary = parser.finish(0);
        assert!(summary.rate_limit.is_some());
    }

    #[test]
    fn error_event_emits_terminal_error() {
        let (sink, mut parser) = new_parser();
        parser
            .feed_line(
                r#"{"type":"error","error":{"type":"billing_error","message":"Insufficient credits"}}"#,
            )
            .unwrap();
        let events = sink.snapshot();
        match &events[0] {
            SemanticEvent::Error { message, terminal, extra } => {
                assert_eq!(message, "Insufficient credits");
                assert!(*terminal);
                assert_eq!(extra.get("error_kind"), Some(&json!("billing_error")));
            }
            other => panic!("expected Error, got {other:?}"),
        }
        let summary = parser.finish(1);
        assert!(summary.is_error);
        assert_eq!(summary.error_kind.as_deref(), Some("billing_error"));
    }

    #[test]
    fn result_emits_turn_complete_and_populates_summary() {
        let (sink, mut parser) = new_parser();
        parser
            .feed_line(
                r#"{"type":"result","duration_ms":12345,"num_turns":1,"stop_reason":"end_turn","cost_usd":0.0042,"usage":{"input_tokens":1000,"output_tokens":500}}"#,
            )
            .unwrap();
        let events = sink.snapshot();
        match &events[0] {
            SemanticEvent::TurnComplete { provider_status, cost_usd, duration_ms, token_usage, .. } => {
                assert_eq!(provider_status.as_deref(), Some("end_turn"));
                assert_eq!(*cost_usd, Some(0.0042));
                assert_eq!(*duration_ms, Some(12345));
                let tu = token_usage.as_ref().unwrap();
                assert_eq!(tu.input, Some(1000));
                assert_eq!(tu.output, Some(500));
                assert_eq!(tu.total, Some(1500));
            }
            other => panic!("expected TurnComplete, got {other:?}"),
        }
        let summary = parser.finish(0);
        assert_eq!(summary.duration_ms, Some(12345));
        assert_eq!(summary.cost_usd, Some(0.0042));
    }

    #[test]
    fn malformed_json_emits_warning_and_returns_ok() {
        let (sink, mut parser) = new_parser();
        let result = parser.feed_line("not json {{{");
        assert!(result.is_ok(), "malformed line must not return Err");
        let events = sink.snapshot();
        assert!(matches!(events[0], SemanticEvent::Warning { .. }));
    }

    #[test]
    fn unknown_event_becomes_provider_extension() {
        let (sink, mut parser) = new_parser();
        parser
            .feed_line(r#"{"type":"some_future_event","foo":"bar"}"#)
            .unwrap();
        let events = sink.snapshot();
        match &events[0] {
            SemanticEvent::ProviderExtension { provider, kind, payload } => {
                assert_eq!(*provider, Provider::Claude);
                assert_eq!(kind, "some_future_event");
                assert_eq!(payload.get("foo"), Some(&json!("bar")));
            }
            other => panic!("expected ProviderExtension, got {other:?}"),
        }
    }

    #[test]
    fn task_started_becomes_subagent_start() {
        let (sink, mut parser) = new_parser();
        parser
            .feed_line(r#"{"type":"task_started","task_id":"sa_1","name":"researcher"}"#)
            .unwrap();
        let events = sink.snapshot();
        match &events[0] {
            SemanticEvent::SubagentStart { name, id, .. } => {
                assert_eq!(name.as_deref(), Some("researcher"));
                assert_eq!(id.as_deref(), Some("sa_1"));
            }
            other => panic!("expected SubagentStart, got {other:?}"),
        }
    }

    #[test]
    fn task_completed_becomes_subagent_stop() {
        let (sink, mut parser) = new_parser();
        parser
            .feed_line(
                r#"{"type":"task_completed","task_id":"sa_1","name":"researcher","status":"success"}"#,
            )
            .unwrap();
        let events = sink.snapshot();
        match &events[0] {
            SemanticEvent::SubagentStop { name, id, status, .. } => {
                assert_eq!(name.as_deref(), Some("researcher"));
                assert_eq!(id.as_deref(), Some("sa_1"));
                assert_eq!(status.as_deref(), Some("success"));
            }
            other => panic!("expected SubagentStop, got {other:?}"),
        }
    }

    #[test]
    fn task_progress_becomes_info() {
        let (sink, mut parser) = new_parser();
        parser
            .feed_line(r#"{"type":"task_progress","message":"working on it"}"#)
            .unwrap();
        let events = sink.snapshot();
        match &events[0] {
            SemanticEvent::Info { message, .. } => {
                assert_eq!(message, "working on it");
            }
            other => panic!("expected Info, got {other:?}"),
        }
    }

    #[test]
    fn empty_and_whitespace_lines_emit_nothing() {
        let (sink, mut parser) = new_parser();
        parser.feed_line("").unwrap();
        parser.feed_line("  ").unwrap();
        parser.feed_line("\t").unwrap();
        assert!(sink.snapshot().is_empty());
    }

    #[test]
    fn multi_turn_concatenation() {
        let (sink, mut parser) = new_parser();
        parser
            .feed_line(r#"{"type":"assistant","content":[{"type":"text","text":"First. "}]}"#)
            .unwrap();
        parser
            .feed_line(r#"{"type":"assistant","content":[{"type":"text","text":"Second."}]}"#)
            .unwrap();
        assert_eq!(sink.kinds(), vec!["output_text", "output_text"]);
        let summary = parser.finish(0);
        assert_eq!(summary.assistant_text, "First. Second.");
    }

    #[test]
    fn large_init_arrays_not_stored_in_raw_summary() {
        let (_, mut parser) = new_parser();
        parser
            .feed_line(r#"{"type":"init","session_id":"s","model":"m","tools":[{"name":"a"}]}"#)
            .unwrap();
        parser
            .feed_line(r#"{"type":"result","duration_ms":1,"tools":["a"],"skills":["s"],"agents":["x"],"mcp_servers":["m"]}"#)
            .unwrap();
        let summary = parser.finish(0);
        let raw = summary.raw_summary.unwrap();
        assert!(raw.get("tools").is_none());
        assert!(raw.get("skills").is_none());
        assert!(raw.get("agents").is_none());
        assert!(raw.get("mcp_servers").is_none());
        assert!(raw.get("duration_ms").is_some());
    }

    #[test]
    fn badges_derived_on_billing_error() {
        let (_, mut parser) = new_parser();
        parser
            .feed_line(r#"{"type":"init","session_id":"s","model":"m"}"#)
            .unwrap();
        parser
            .feed_line(
                r#"{"type":"error","error":{"type":"billing_error","message":"Insufficient credits"}}"#,
            )
            .unwrap();
        let summary = parser.finish(1);
        assert_eq!(summary.badges.len(), 1);
        assert_eq!(
            summary.badges[0].category,
            crate::stream::badges::BadgeCategory::Billing
        );
    }

    #[test]
    fn user_event_routes_tool_result_to_semantic_tool_result() {
        let (sink, mut parser) = new_parser();
        parser
            .feed_line(r#"{"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"tu_1","content":"hello","is_error":false}]},"session_id":"s1"}"#)
            .unwrap();
        let kinds = sink.kinds();
        assert!(
            kinds.contains(&"tool_result"),
            "expected tool_result; got {kinds:?}"
        );
        assert!(
            !kinds.contains(&"provider_extension"),
            "user event must not leak as ProviderExtension"
        );
    }

    #[test]
    fn billing_error_on_assistant_surfaces_terminal_error_not_rate_limit() {
        let (sink, mut parser) = new_parser();
        parser
            .feed_line(r#"{"type":"assistant","message":{"model":"<synthetic>","content":[{"type":"text","text":"Credit balance is too low"}]},"session_id":"s1","error":"billing_error"}"#)
            .unwrap();
        let events = sink.snapshot();
        let terminal_errors: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, SemanticEvent::Error { terminal: true, .. }))
            .collect();
        assert_eq!(terminal_errors.len(), 1, "expected exactly one terminal Error; got {terminal_errors:?}");
        if let SemanticEvent::Error { message, extra, .. } = terminal_errors[0] {
            let lower = message.to_lowercase();
            assert!(
                lower.contains("billing") || lower.contains("credit"),
                "billing error message must mention billing/credit: {message:?}"
            );
            assert_eq!(
                extra.get("error_kind").and_then(|v| v.as_str()),
                Some("billing_error")
            );
        }
        let summary = parser.finish(1);
        assert_eq!(summary.error_kind.as_deref(), Some("billing_error"));
        let billing = summary.badges.iter().find(|b| b.category == crate::stream::badges::BadgeCategory::Billing);
        assert!(billing.is_some(), "summary must carry a Billing badge, not a RateLimit one; got {:?}", summary.badges);
        assert!(
            !summary.badges.iter().any(|b| b.category == crate::stream::badges::BadgeCategory::RateLimit),
            "billing_error must NOT produce a RateLimit badge; got {:?}", summary.badges
        );
    }

    #[test]
    fn hook_events_without_init_do_not_fabricate_session_start() {
        // Without an `init` event, hook_* system subtypes must NOT be
        // promoted to a `SessionStart`. They stay buffered until a real
        // init arrives (or flush inline once the buffer saturates). In
        // either case, nothing should synthesize a session_start.
        let (sink, mut parser) = new_parser();
        parser
            .feed_line(r#"{"type":"system","subtype":"hook_started","hook_name":"SessionStart:startup","session_id":"s1"}"#)
            .unwrap();
        parser
            .feed_line(r#"{"type":"system","subtype":"hook_response","hook_id":"x","output":"ok","exit_code":0,"session_id":"s1"}"#)
            .unwrap();
        let kinds = sink.kinds();
        assert!(
            !kinds.contains(&"session_start"),
            "hook_* subtypes must not emit SessionStart; got {kinds:?}"
        );
    }

    #[test]
    fn hook_events_emitted_after_session_start() {
        let (sink, mut parser) = new_parser();
        parser
            .feed_line(r#"{"type":"system","subtype":"hook_started","hook_name":"SessionStart:startup","session_id":"s1"}"#)
            .unwrap();
        parser
            .feed_line(r#"{"type":"system","subtype":"hook_response","hook_id":"x","output":"ok","exit_code":0,"session_id":"s1"}"#)
            .unwrap();
        parser
            .feed_line(r#"{"type":"init","session_id":"s1","model":"claude-opus-4-6"}"#)
            .unwrap();
        let kinds: Vec<&'static str> = sink.kinds();
        let session_idx = kinds
            .iter()
            .position(|k| *k == "session_start")
            .expect("session_start emitted");
        let provider_ext_indices: Vec<usize> = kinds
            .iter()
            .enumerate()
            .filter(|(_, k)| **k == "provider_extension")
            .map(|(i, _)| i)
            .collect();
        for idx in provider_ext_indices {
            assert!(
                idx > session_idx,
                "provider_extension hook event at {idx} must follow session_start at {session_idx}; got {kinds:?}"
            );
        }
    }

    #[test]
    fn hook_events_after_session_start_emit_inline() {
        // Hooks that arrive AFTER SessionStart must NOT be buffered — they
        // pass through inline to preserve live streaming semantics.
        let (sink, mut parser) = new_parser();
        parser
            .feed_line(r#"{"type":"init","session_id":"s1","model":"claude-opus-4-6"}"#)
            .unwrap();
        parser
            .feed_line(r#"{"type":"system","subtype":"hook_started","hook_name":"PreToolUse","session_id":"s1"}"#)
            .unwrap();
        let kinds: Vec<&'static str> = sink.kinds();
        // Order: session_start, then immediately provider_extension.
        assert_eq!(kinds.first(), Some(&"session_start"));
        assert_eq!(kinds.get(1), Some(&"provider_extension"));
    }

    #[test]
    fn pre_init_hook_buffer_flushes_when_oversized() {
        // If enough hooks arrive before init that the buffer saturates,
        // flush early so streaming wins over cosmetic ordering.
        let (sink, mut parser) = new_parser();
        for _ in 0..40 {
            parser
                .feed_line(r#"{"type":"system","subtype":"hook_started","hook_name":"X","session_id":"s1"}"#)
                .unwrap();
        }
        let kinds: Vec<&'static str> = sink.kinds();
        let provider_ext_count = kinds.iter().filter(|k| **k == "provider_extension").count();
        assert!(
            provider_ext_count > 0,
            "hooks past the buffer cap must flush inline; got {kinds:?}"
        );
    }

    #[test]
    fn claude_fixture_full_replay_produces_no_provider_extensions() {
        let fixture = std::fs::read_to_string(
            concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/providers/claude.ndjson"),
        )
        .expect("claude.ndjson must exist");

        let (sink, mut parser) = new_parser();
        for (i, line) in fixture.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() { continue; }
            parser
                .feed_line(line)
                .unwrap_or_else(|e| panic!("line {}: {:?}", i + 1, e));
        }
        let events = sink.snapshot();
        // Hook events (`system/hook_started`, `system/hook_response`, etc.)
        // are intentionally surfaced as `ProviderExtension` so the sink can
        // render them after the session-ID marker. They are excluded from
        // the "no unrouted provider extensions" guarantee.
        let ext: Vec<&SemanticEvent> = events
            .iter()
            .filter(|e| match e {
                SemanticEvent::ProviderExtension { kind, .. } => !kind.starts_with("system/"),
                _ => false,
            })
            .collect();
        assert!(
            ext.is_empty(),
            "captured Claude fixture must produce zero non-hook ProviderExtension events; found {}: {:#?}",
            ext.len(),
            ext.iter().take(3).collect::<Vec<_>>()
        );
    }

    #[test]
    fn round_trip_fidelity_across_mixed_events() {
        // Replay a mixed fixture and confirm every emitted event survives a
        // serde round-trip with identical JSON.
        let (sink, mut parser) = new_parser();
        for line in [
            r#"{"type":"init","session_id":"s","model":"m"}"#,
            r#"{"type":"content_block_delta","delta":{"type":"text_delta","text":"Hi"}}"#,
            r#"{"type":"tool_use","id":"t1","name":"bash","input":{"cmd":"ls"}}"#,
            r#"{"type":"rate_limit_event","is_throttled":true,"retry_after_ms":5000,"message":"Slow"}"#,
            r#"{"type":"task_progress","message":"working"}"#,
            r#"{"type":"some_future_event","x":1}"#,
            r#"{"type":"result","duration_ms":1}"#,
        ] {
            parser.feed_line(line).unwrap();
        }
        let events = sink.snapshot();
        assert!(!events.is_empty());
        for event in events {
            let v = serde_json::to_value(&event).unwrap();
            let decoded: SemanticEvent = serde_json::from_value(v.clone()).unwrap();
            let v2 = serde_json::to_value(&decoded).unwrap();
            assert_eq!(v, v2, "round-trip lost fidelity for {}", event.kind_str());
        }
    }
}
