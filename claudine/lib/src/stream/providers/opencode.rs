//! Native [`SemanticStreamParser`] implementation for OpenCode CLI's NDJSON
//! output.
//!
//! OpenCode emits a step-oriented stream: `session_start` metadata, then
//! alternating `step_start` / `text` / `reasoning` / `tool_use` /
//! `tool_result` / `step_finish` events, plus a final `step_complete` /
//! `turn_complete` that carries usage / cost / duration. Parent-session
//! permission gaps aren't currently exposed in the NDJSON stream.
//!
//! Routing:
//!
//! - `init` / `session_start` → [`SemanticEvent::SessionStart`]. First
//!   `step_start` also triggers `SessionStart` when no session id was seen yet.
//! - `step_start` / `step_finish` → [`SemanticEvent::Info`] with a
//!   `step_phase` marker so renderers / heartbeats can key off activity.
//! - `text` / `text_delta` / `assistant_text` → [`SemanticEvent::OutputText`].
//! - `reasoning` → [`SemanticEvent::Reasoning`].
//! - `tool_start` → [`SemanticEvent::ToolCall`] (pre-completion).
//! - `tool_use` → paired [`SemanticEvent::ToolCall`] + [`SemanticEvent::ToolResult`]
//!   (OpenCode emits `tool_use` only after the tool has reached `completed` / `error`).
//! - `tool_result` / `tool_end` → [`SemanticEvent::ToolResult`].
//! - `step_complete` / `turn_complete` → [`SemanticEvent::TurnComplete`].
//! - `error` / `step_error` → [`SemanticEvent::Error`].
//! - Anything else → [`SemanticEvent::ProviderExtension`].

use std::collections::HashMap;

use serde_json::{Map, Value};
use tracing::debug;

use super::parser::{SemanticStreamParser, StreamParseError};
use super::protocol::opencode::{
    OpenCodeError, OpenCodeEvent, OpenCodeInit, OpenCodeReasoning, OpenCodeStepComplete,
    OpenCodeStepFinish, OpenCodeStepStart, OpenCodeTaskEvent, OpenCodeTaskProgress, OpenCodeText,
    OpenCodeTool,
};
use super::semantic::{SemanticErrorKind, SemanticEvent, SemanticEventSink};
use super::summary::StreamExecutionSummary;
use super::token_usage::NormalizedTokenUsage;
use crate::provider_id::Provider;
pub struct OpenCodeSemanticStreamParser<S: SemanticEventSink> {
    sink: S,
    line_num: usize,
    session_id: Option<String>,
    model: Option<String>,
    assistant_text: String,
    token_usage: NormalizedTokenUsage,
    cost_usd: f64,
    duration_ms: Option<u64>,
    num_turns: u32,
    tool_calls: u32,
    provider_status: Option<String>,
    is_error: bool,
    error_kind: Option<String>,
    error_message: Option<String>,
    tool_uses: HashMap<String, (Option<String>, Option<Value>)>,
}

impl<S: SemanticEventSink> OpenCodeSemanticStreamParser<S> {
    pub fn new(sink: S, model: Option<String>) -> Self {
        Self {
            sink,
            line_num: 0,
            session_id: None,
            model,
            assistant_text: String::new(),
            token_usage: NormalizedTokenUsage::default(),
            cost_usd: 0.0,
            duration_ms: None,
            num_turns: 0,
            tool_calls: 0,
            provider_status: None,
            is_error: false,
            error_kind: None,
            error_message: None,
            tool_uses: HashMap::new(),
        }
    }

    fn base_extra(&self, raw_kind: &str) -> Map<String, Value> {
        let mut m = Map::new();
        m.insert("provider".into(), Value::from("opencode"));
        m.insert("line_num".into(), Value::from(self.line_num));
        m.insert("raw_kind".into(), Value::from(raw_kind));
        m
    }

    fn emit_session_start(&mut self, raw_kind: &str) {
        self.sink.on_semantic_event(SemanticEvent::SessionStart {
            session_id: self.session_id.clone(),
            model: self.model.clone(),
            extra: Value::Object(self.base_extra(raw_kind)),
        });
    }

    fn handle_init(&mut self, init: OpenCodeInit, raw_kind: &str) {
        self.session_id = init.session_id;
        if let Some(model) = init.model {
            self.model = Some(model);
        }
        super::trace_session_metadata(
            Provider::OpenCode,
            self.session_id.as_deref(),
            self.model.as_deref(),
        );
        self.emit_session_start(raw_kind);
    }

    fn handle_step_start(&mut self, step: OpenCodeStepStart, raw_kind: &str) {
        if self.session_id.is_none() {
            self.session_id = step.resolved_session_id();
            super::trace_session_metadata(
                Provider::OpenCode,
                self.session_id.as_deref(),
                self.model.as_deref(),
            );
            self.emit_session_start(raw_kind);
        }
        self.num_turns += 1;
        let mut extra = self.base_extra(raw_kind);
        extra.insert("step_phase".into(), Value::from("start"));
        self.sink.on_semantic_event(SemanticEvent::Info {
            message: "step_start".into(),
            extra: Value::Object(extra),
        });
    }

    fn handle_step_finish(&mut self, event: OpenCodeStepFinish, raw_kind: &str) {
        if let Some(part) = event.part {
            if let Some(tokens) = part.tokens {
                let step = NormalizedTokenUsage {
                    input: tokens.input,
                    output: tokens.output,
                    total: tokens.total,
                    cache_read: tokens.cache.and_then(|c| c.read),
                };
                self.token_usage.accumulate(&step);
            }
            if let Some(cost) = part.cost {
                self.cost_usd += cost;
            }
            if let Some(reason) = part.reason {
                self.provider_status = Some(reason);
            }
        }
        super::trace_summary_update(
            Provider::OpenCode,
            self.provider_status.as_deref(),
            self.duration_ms,
            Some(self.cost_usd),
        );
        let mut extra = self.base_extra(raw_kind);
        extra.insert("step_phase".into(), Value::from("finish"));
        if let Some(status) = &self.provider_status {
            extra.insert("reason".into(), Value::from(status.as_str()));
        }
        self.sink.on_semantic_event(SemanticEvent::Info {
            message: "step_finish".into(),
            extra: Value::Object(extra),
        });
    }

    fn handle_step_complete(&mut self, event: OpenCodeStepComplete, raw_kind: &str) {
        self.num_turns += 1;
        let mut step_usage = None;
        if let Some(usage) = event.usage {
            let step = NormalizedTokenUsage {
                input: usage.input_tokens,
                output: usage.output_tokens,
                total: usage.total_tokens,
                cache_read: None,
            };
            self.token_usage.accumulate(&step);
            step_usage = Some(step);
        }
        if let Some(cost) = event.cost_usd {
            self.cost_usd += cost;
        }
        if let Some(duration) = event.duration_ms {
            self.duration_ms = Some(duration);
        }
        super::trace_summary_update(
            Provider::OpenCode,
            self.provider_status.as_deref(),
            self.duration_ms,
            Some(self.cost_usd),
        );
        self.sink.on_semantic_event(SemanticEvent::TurnComplete {
            provider_status: self.provider_status.clone(),
            token_usage: step_usage,
            cost_usd: event.cost_usd,
            duration_ms: event.duration_ms,
            extra: Value::Object(self.base_extra(raw_kind)),
        });
    }

    fn handle_text(&mut self, event: OpenCodeText, raw_kind: &str) {
        let Some(text) = event.resolved_text() else {
            return;
        };
        if text.is_empty() {
            return;
        }
        self.assistant_text.push_str(&text);
        self.sink.on_semantic_event(SemanticEvent::OutputText {
            text,
            extra: Value::Object(self.base_extra(raw_kind)),
        });
    }

    fn handle_reasoning(&mut self, event: OpenCodeReasoning, raw_kind: &str) {
        let Some(text) = event.resolved_text() else {
            return;
        };
        if text.is_empty() {
            return;
        }
        self.sink.on_semantic_event(SemanticEvent::Reasoning {
            text,
            extra: Value::Object(self.base_extra(raw_kind)),
        });
    }

    fn handle_task_started(&mut self, event: OpenCodeTaskEvent, raw_kind: &str) {
        let mut extra = self.base_extra(raw_kind);
        if let Some(status) = &event.status {
            extra.insert("status".into(), Value::from(status.as_str()));
        }
        self.sink.on_semantic_event(SemanticEvent::SubagentStart {
            name: event.resolved_name(),
            id: event.resolved_task_id(),
            extra: Value::Object(extra),
        });
    }

    fn handle_task_completed(&mut self, event: OpenCodeTaskEvent, raw_kind: &str) {
        let mut extra = self.base_extra(raw_kind);
        if let Some(status) = &event.status {
            extra.insert("status".into(), Value::from(status.as_str()));
        }
        self.sink.on_semantic_event(SemanticEvent::SubagentStop {
            name: event.resolved_name(),
            id: event.resolved_task_id(),
            status: event.status,
            extra: Value::Object(extra),
        });
    }

    fn handle_task_progress(&mut self, event: OpenCodeTaskProgress, raw_kind: &str) {
        let message = event.message.unwrap_or_else(|| raw_kind.to_string());
        self.sink.on_semantic_event(SemanticEvent::Info {
            message,
            extra: Value::Object(self.base_extra(raw_kind)),
        });
    }

    fn handle_error(&mut self, event: OpenCodeError, raw_kind: &str) {
        self.is_error = true;
        self.error_kind = event.resolved_kind();
        self.error_message = event.resolved_message();

        let mut extra = self.base_extra(raw_kind);
        if let Some(kind) = &self.error_kind {
            extra.insert("error_kind".into(), Value::from(kind.as_str()));
        }
        let message = self
            .error_message
            .clone()
            .unwrap_or_else(|| "Step failure".to_string());
        let semantic_kind = classify_error(self.error_kind.as_deref(), Some(&message));
        self.sink.on_semantic_event(SemanticEvent::Error {
            message,
            terminal: true,
            kind: semantic_kind,
            extra: Value::Object(extra),
        });
    }

    fn handle_tool_use(&mut self, tool: OpenCodeTool, raw_kind: &str) {
        self.tool_calls += 1;
        let resolved = tool.resolve();
        super::trace_tool_event(
            Provider::OpenCode,
            self.tool_calls,
            resolved.name.as_deref(),
        );

        let mut extra = self.base_extra(raw_kind);
        if let Some(id) = &resolved.id {
            extra.insert("tool_id".into(), Value::from(id.as_str()));
        }
        if let Some(name) = &resolved.name {
            extra.insert("tool_name".into(), Value::from(name.as_str()));
        }

        if let Some(id) = &resolved.id {
            self.tool_uses
                .insert(id.clone(), (resolved.name.clone(), resolved.input.clone()));
        }

        self.sink.on_semantic_event(SemanticEvent::ToolCall {
            name: resolved.name,
            id: resolved.id,
            input: resolved.input,
            extra: Value::Object(extra),
        });
    }

    /// Handle OpenCode's `tool_use` event, which per the run.ts contract is
    /// only emitted *after* a tool reaches `completed` or `error`. OpenCode
    /// does not emit a paired request-side event, so we emit only a
    /// `ToolResult` (no synthesized `ToolCall`). The `tool_calls` counter
    /// still increments so trailer metadata matches the rendered line count.
    ///
    /// ## Notes
    ///
    /// Subagent lifecycle (`SubagentStart` / `SubagentStop`) is no longer
    /// synthesized here. The structured stderr log stream is now the
    /// authoritative source for OpenCode subagent events: child sessions
    /// emit `service=session ... parentID=...` at start and
    /// `service=session.prompt ... exiting loop` at stop, which the
    /// [`crate::stream::logs::opencode::OpenCodeLogBridge`] promotes into
    /// the same semantic events. Keeping synthesis here as well would
    /// double-count subagents in `LiveMetricsState.subagent_done_count`
    /// and produce duplicate render lines.
    fn handle_tool_use_completed(&mut self, tool: OpenCodeTool, raw_kind: &str) {
        self.tool_calls += 1;
        let resolved = tool.resolve();
        super::trace_tool_event(
            Provider::OpenCode,
            self.tool_calls,
            resolved.name.as_deref(),
        );

        let mut result_extra = self.base_extra(raw_kind);
        if let Some(id) = &resolved.id {
            result_extra.insert("tool_id".into(), Value::from(id.as_str()));
        }
        if let Some(name) = &resolved.name {
            result_extra.insert("tool_name".into(), Value::from(name.as_str()));
        }
        if let Some(status) = &resolved.status {
            result_extra.insert("status".into(), Value::from(status.as_str()));
        }
        if let Some(err) = &resolved.error {
            result_extra.insert("error".into(), err.clone());
        }
        if let Some(input) = &resolved.input {
            result_extra.insert("input".into(), input.clone());
        }

        self.sink.on_semantic_event(SemanticEvent::ToolResult {
            name: resolved.name.clone(),
            id: resolved.id.clone(),
            status: resolved.status.clone(),
            exit_code: None,
            output: resolved.output.clone(),
            extra: Value::Object(result_extra),
        });
    }

    fn handle_tool_result(&mut self, tool: OpenCodeTool, raw_kind: &str) {
        let resolved = tool.resolve();
        let (cached_name, cached_input) = resolved
            .id
            .as_ref()
            .and_then(|id| self.tool_uses.remove(id))
            .unwrap_or((resolved.name.clone(), None));

        let mut extra = self.base_extra(raw_kind);
        if let Some(id) = &resolved.id {
            extra.insert("tool_id".into(), Value::from(id.as_str()));
        }
        if let Some(name) = &cached_name {
            extra.insert("tool_name".into(), Value::from(name.as_str()));
        }
        if let Some(status) = &resolved.status {
            extra.insert("status".into(), Value::from(status.as_str()));
        }
        if let Some(err) = &resolved.error {
            extra.insert("error".into(), err.clone());
        }
        // Preserve the original tool input alongside the result so renderers
        // can annotate successful incoming tool events with the same slot
        // content the outgoing `→ Name(...)` arrow used. Prefer a
        // wire-provided `input` on the `tool_result` / `tool_end` payload
        // (rare but permitted) and fall back to the cached input captured
        // on the paired `tool_start`.
        if let Some(input) = resolved.input.or(cached_input) {
            extra.insert("input".into(), input);
        }

        self.sink.on_semantic_event(SemanticEvent::ToolResult {
            name: cached_name,
            id: resolved.id,
            status: resolved.status,
            exit_code: None,
            output: resolved.output,
            extra: Value::Object(extra),
        });
    }

    fn emit_provider_extension(&mut self, kind: &str, payload: Value) {
        debug!(
            provider = "opencode",
            event_type = %kind,
            "opencode parser falling back to provider extension for unknown event type"
        );
        self.sink
            .on_semantic_event(SemanticEvent::ProviderExtension {
                provider: Provider::OpenCode,
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

impl<S: SemanticEventSink> SemanticStreamParser for OpenCodeSemanticStreamParser<S> {
    fn feed_line(&mut self, line: &str) -> Result<(), StreamParseError> {
        self.line_num += 1;
        let line = line.trim();
        if line.is_empty() {
            return Ok(());
        }

        // Try typed deserialization first to avoid `serde_json::Value` DOM
        // allocation on the hot path. Fall back to `Value` only for unknown
        // event types that must be preserved as `ProviderExtension`.
        match serde_json::from_str::<OpenCodeEvent>(line) {
            Ok(event) => {
                let raw_kind = event.type_str().to_string();
                super::trace_parser_event(Provider::OpenCode, &raw_kind, self.line_num);
                match event {
                    OpenCodeEvent::Init(init) | OpenCodeEvent::SessionStart(init) => {
                        self.handle_init(init, &raw_kind);
                    }
                    OpenCodeEvent::StepStart(step) => {
                        self.handle_step_start(step, &raw_kind);
                    }
                    OpenCodeEvent::Text(text)
                    | OpenCodeEvent::TextDelta(text)
                    | OpenCodeEvent::AssistantText(text) => {
                        self.handle_text(text, &raw_kind);
                    }
                    OpenCodeEvent::Reasoning(reasoning) => {
                        self.handle_reasoning(reasoning, &raw_kind);
                    }
                    OpenCodeEvent::TaskStarted(task) => {
                        self.handle_task_started(task, &raw_kind);
                    }
                    OpenCodeEvent::TaskCompleted(task) => {
                        self.handle_task_completed(task, &raw_kind);
                    }
                    OpenCodeEvent::TaskProgress(progress) => {
                        self.handle_task_progress(progress, &raw_kind);
                    }
                    OpenCodeEvent::StepFinish(sf) => {
                        self.handle_step_finish(sf, &raw_kind);
                    }
                    OpenCodeEvent::StepComplete(sc) | OpenCodeEvent::TurnComplete(sc) => {
                        self.handle_step_complete(sc, &raw_kind);
                    }
                    OpenCodeEvent::Error(err) | OpenCodeEvent::StepError(err) => {
                        self.handle_error(err, &raw_kind);
                    }
                    OpenCodeEvent::ToolUse(tool) => {
                        self.handle_tool_use_completed(tool, &raw_kind);
                    }
                    OpenCodeEvent::ToolStart(tool) => {
                        self.handle_tool_use(tool, &raw_kind);
                    }
                    OpenCodeEvent::ToolResult(tool) | OpenCodeEvent::ToolEnd(tool) => {
                        self.handle_tool_result(tool, &raw_kind);
                    }
                }
            }
            Err(_) => {
                let raw: Map<String, Value> = match serde_json::from_str(line) {
                    Ok(v) => v,
                    Err(e) => {
                        super::trace_malformed_line(
                            Provider::OpenCode,
                            self.line_num,
                            &e.to_string(),
                        );
                        self.emit_malformed_warning(&e.to_string());
                        return Ok(());
                    }
                };
                let raw_kind = raw
                    .get("type")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
                super::trace_parser_event(Provider::OpenCode, &raw_kind, self.line_num);
                self.emit_provider_extension(&raw_kind, Value::Object(raw));
            }
        }
        Ok(())
    }

    fn finish(self: Box<Self>, exit_code: i32) -> StreamExecutionSummary {
        super::trace_parser_finish(
            Provider::OpenCode,
            exit_code,
            self.tool_calls,
            self.num_turns,
            self.provider_status.as_deref(),
        );
        let has_usage = self.token_usage.input.is_some() || self.token_usage.output.is_some();
        let mut summary = StreamExecutionSummary {
            provider: Provider::OpenCode,
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
            token_usage: if has_usage {
                Some(self.token_usage)
            } else {
                None
            },
            cost_usd: if self.cost_usd > 0.0 {
                Some(self.cost_usd)
            } else {
                None
            },
            tool_calls: if self.tool_calls > 0 {
                Some(self.tool_calls)
            } else {
                None
            },
            permission_prompts: None,
            user_input_prompts: None,
            rate_limit: None,
            context_usage: None,
            badges: Vec::new(),
            raw_summary: None,
            stderr_text: None,
            stderr_diagnostics: None,
        };
        summary.badges = crate::stream::badges::derive_badges(&summary, Provider::OpenCode);
        summary
    }
}

/// Map an OpenCode error envelope onto a typed [`SemanticErrorKind`].
fn classify_error(error_kind: Option<&str>, message: Option<&str>) -> SemanticErrorKind {
    if let Some(kind) = error_kind {
        let lower = kind.to_ascii_lowercase();
        if lower.contains("rate") || lower.contains("quota") || lower.contains("billing") {
            return SemanticErrorKind::ApiRemote;
        }
        if lower.contains("auth")
            || lower.contains("config")
            || lower.contains("permission")
            || lower.contains("provider")
            || lower.contains("model")
        {
            return SemanticErrorKind::Configuration;
        }
        if lower.contains("interrupt") || lower.contains("cancel") || lower.contains("abort") {
            return SemanticErrorKind::Interrupted;
        }
        if lower.contains("api") || lower.contains("upstream") || lower.contains("server") {
            return SemanticErrorKind::ApiRemote;
        }
    }
    if let Some(msg) = message {
        let lower = msg.to_ascii_lowercase();
        if lower.contains("rate limit")
            || lower.contains("quota")
            || lower.contains("billing")
            || lower.contains("api error")
            || lower.contains("api timeout")
        {
            return SemanticErrorKind::ApiRemote;
        }
        if lower.contains("api key")
            || lower.contains("authentication")
            || lower.contains("not authorized")
            || lower.contains("permission denied")
            || lower.contains("model not found")
            || lower.contains("invalid model")
            || lower.contains("providermodelnotfound")
        {
            return SemanticErrorKind::Configuration;
        }
        if lower.contains("interrupt") || lower.contains("cancel") || lower.contains("aborted") {
            return SemanticErrorKind::Interrupted;
        }
    }
    SemanticErrorKind::AgentNative
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};
    use std::time::Instant;

    use super::*;
    use crate::stream::progress::LiveMetricsState;
    use serde_json::json;

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
        Box<OpenCodeSemanticStreamParser<Recording>>,
    ) {
        let events = Arc::new(Mutex::new(Vec::new()));
        let sink = Recording {
            events: events.clone(),
        };
        (
            events,
            Box::new(OpenCodeSemanticStreamParser::new(
                sink,
                Some("gpt-4o".into()),
            )),
        )
    }

    fn kinds(events: &[SemanticEvent]) -> Vec<&'static str> {
        events.iter().map(|e| e.kind_str()).collect()
    }

    #[test]
    fn step_start_emits_session_start_once_and_info() {
        let (events, mut parser) = new_parser();
        parser
            .feed_line(r#"{"type":"step_start","sessionID":"ses_1"}"#)
            .unwrap();
        parser
            .feed_line(r#"{"type":"step_start","sessionID":"ses_1"}"#)
            .unwrap();
        let ks = kinds(&events.lock().unwrap());
        // first step_start: session_start + info; second: just info
        assert_eq!(ks, vec!["session_start", "info", "info"]);
    }

    #[test]
    fn text_emits_output_text() {
        let (events, mut parser) = new_parser();
        parser
            .feed_line(r#"{"type":"text","text":"hello"}"#)
            .unwrap();
        let collected = events.lock().unwrap().clone();
        assert!(matches!(
            collected[0],
            SemanticEvent::OutputText { ref text, .. } if text == "hello"
        ));
    }

    #[test]
    fn tool_use_and_result_emit_typed_events() {
        let (events, mut parser) = new_parser();
        parser
            .feed_line(
                r#"{"type":"tool_start","part":{"id":"t1","tool_name":"bash","input":{"cmd":"ls"}}}"#,
            )
            .unwrap();
        parser
            .feed_line(
                r#"{"type":"tool_end","part":{"tool_use_id":"t1","status":"success","content":"ok"}}"#,
            )
            .unwrap();
        let ks = kinds(&events.lock().unwrap());
        assert_eq!(ks, vec!["tool_call", "tool_result"]);
    }

    #[test]
    fn orphan_tool_result_emits_unmatched_result() {
        // OpenCode's stream frequently has completion-only visibility.
        let (events, mut parser) = new_parser();
        parser
            .feed_line(
                r#"{"type":"tool_end","part":{"tool_use_id":"t999","status":"success","content":"ok","tool_name":"bash"}}"#,
            )
            .unwrap();
        let collected = events.lock().unwrap().clone();
        match &collected[0] {
            SemanticEvent::ToolResult {
                id, name, status, ..
            } => {
                assert_eq!(id.as_deref(), Some("t999"));
                assert_eq!(name.as_deref(), Some("bash"));
                assert_eq!(status.as_deref(), Some("success"));
            }
            other => panic!("expected orphan ToolResult, got {other:?}"),
        }
    }

    #[test]
    fn step_complete_emits_turn_complete_and_sums() {
        let (events, mut parser) = new_parser();
        parser
            .feed_line(
                r#"{"type":"step_complete","usage":{"input_tokens":10,"output_tokens":5,"total_tokens":15},"cost_usd":0.001,"duration_ms":200}"#,
            )
            .unwrap();
        parser
            .feed_line(
                r#"{"type":"step_complete","usage":{"input_tokens":20,"output_tokens":10,"total_tokens":30},"cost_usd":0.002}"#,
            )
            .unwrap();
        let collected = events.lock().unwrap().clone();
        assert_eq!(kinds(&collected), vec!["turn_complete", "turn_complete"]);
        let summary = parser.finish(0);
        let tu = summary.token_usage.unwrap();
        assert_eq!(tu.input, Some(30));
        assert_eq!(tu.output, Some(15));
        assert_eq!(summary.cost_usd, Some(0.003));
    }

    #[test]
    fn error_event_emits_error() {
        let (events, mut parser) = new_parser();
        parser
            .feed_line(r#"{"type":"error","error_message":"API timeout"}"#)
            .unwrap();
        let collected = events.lock().unwrap().clone();
        assert!(matches!(
            collected[0],
            SemanticEvent::Error { terminal: true, .. }
        ));
        let summary = parser.finish(1);
        assert!(summary.is_error);
    }

    #[test]
    fn unknown_event_becomes_provider_extension() {
        let (events, mut parser) = new_parser();
        parser
            .feed_line(r#"{"type":"some_future_event","x":1}"#)
            .unwrap();
        let collected = events.lock().unwrap().clone();
        assert!(matches!(
            collected[0],
            SemanticEvent::ProviderExtension { ref kind, .. } if kind == "some_future_event"
        ));
    }

    #[test]
    fn malformed_json_emits_warning() {
        let (events, mut parser) = new_parser();
        assert!(parser.feed_line("garbage").is_ok());
        assert!(matches!(
            events.lock().unwrap()[0],
            SemanticEvent::Warning { .. }
        ));
    }

    #[test]
    fn tool_input_string_fallback_parses_without_panic() {
        let (events, mut parser) = new_parser();
        parser
            .feed_line(r#"{"type":"tool_start","part":{"tool_name":"bash","input":"ls -la"}}"#)
            .unwrap();
        let collected = events.lock().unwrap().clone();
        assert_eq!(kinds(&collected), vec!["tool_call"]);
        match &collected[0] {
            SemanticEvent::ToolCall { input, .. } => {
                assert_eq!(input.as_ref().and_then(Value::as_str), Some("ls -la"));
            }
            other => panic!("expected ToolCall, got {other:?}"),
        }
    }

    #[test]
    fn missing_discriminator_falls_through_to_provider_extension() {
        let (events, mut parser) = new_parser();
        parser.feed_line(r#"{"payload":{"k":1}}"#).unwrap();
        let collected = events.lock().unwrap().clone();
        assert_eq!(collected.len(), 1);
        match &collected[0] {
            SemanticEvent::ProviderExtension {
                provider,
                kind,
                payload,
            } => {
                assert_eq!(*provider, Provider::OpenCode);
                assert_eq!(kind, "");
                assert_eq!(payload.get("payload"), Some(&json!({"k": 1})));
            }
            other => panic!("expected ProviderExtension, got {other:?}"),
        }
    }

    #[test]
    fn step_finish_info_has_phase_marker() {
        let (events, mut parser) = new_parser();
        parser
            .feed_line(
                r#"{"type":"step_finish","part":{"reason":"stop","cost":0.01,"tokens":{"input":1,"output":2,"total":3,"cache":{"read":0}}}}"#,
            )
            .unwrap();
        let collected = events.lock().unwrap().clone();
        match &collected[0] {
            SemanticEvent::Info { message, extra } => {
                assert_eq!(message, "step_finish");
                assert_eq!(extra.get("step_phase"), Some(&Value::from("finish")));
                assert_eq!(extra.get("reason"), Some(&Value::from("stop")));
            }
            other => panic!("expected Info, got {other:?}"),
        }
    }

    #[test]
    fn round_trip_fidelity_mixed_fixture() {
        let (events, mut parser) = new_parser();
        for line in [
            r#"{"type":"step_start","sessionID":"s"}"#,
            r#"{"type":"text","text":"hi"}"#,
            r#"{"type":"tool_start","part":{"id":"t","tool_name":"b","input":{"x":1}}}"#,
            r#"{"type":"tool_end","part":{"tool_use_id":"t","status":"success","content":"ok"}}"#,
            r#"{"type":"step_finish","part":{"reason":"stop","cost":0.01,"tokens":{"input":1,"output":2,"total":3,"cache":{"read":0}}}}"#,
            r#"{"type":"step_complete","usage":{"input_tokens":1,"output_tokens":2,"total_tokens":3},"cost_usd":0.01}"#,
            r#"{"type":"future.kind","x":1}"#,
        ] {
            parser.feed_line(line).unwrap();
        }
        for event in events.lock().unwrap().iter() {
            let v = serde_json::to_value(event).unwrap();
            let decoded: SemanticEvent = serde_json::from_value(v.clone()).unwrap();
            assert_eq!(v, serde_json::to_value(&decoded).unwrap());
        }
    }

    #[test]
    fn opencode_tool_use_emits_tool_result_only() {
        use super::super::parser::SemanticStreamParser;
        use super::super::semantic::{SemanticEvent, SemanticEventSink};
        use std::sync::{Arc, Mutex};

        #[derive(Default)]
        struct Capture(Arc<Mutex<Vec<SemanticEvent>>>);
        impl SemanticEventSink for Capture {
            fn on_semantic_event(&mut self, event: SemanticEvent) {
                self.0.lock().unwrap().push(event);
            }
        }

        let events = Arc::new(Mutex::new(Vec::new()));
        let sink = Capture(events.clone());
        let mut parser = OpenCodeSemanticStreamParser::new(sink, Some("gpt-4o".into()));

        parser
            .feed_line(
                r#"{"type":"tool_use","part":{"id":"t1","tool":"bash",
                 "state":{"status":"completed","input":{"command":"ls -la"},"output":"file.txt"}}}"#,
            )
            .unwrap();

        let captured = events.lock().unwrap().clone();
        let kinds: Vec<&str> = captured.iter().map(|e| e.kind_str()).collect();
        assert_eq!(
            kinds,
            vec!["tool_result"],
            "tool_use (completion) must emit only a ToolResult; OpenCode never emits a paired request"
        );

        let SemanticEvent::ToolResult { name, status, .. } = &captured[0] else {
            panic!("expected ToolResult");
        };
        assert_eq!(name.as_deref(), Some("bash"));
        assert_eq!(status.as_deref(), Some("completed"));
    }

    #[test]
    fn assistant_text_in_part_text_shape_emits_output_text() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/providers/opencode-assistant-text.ndjson");
        let raw = std::fs::read_to_string(&path).expect("fixture exists");
        let (events, mut parser) = new_parser();
        for line in raw.lines() {
            parser.feed_line(line).unwrap();
        }
        let has_non_empty_output_text = events
            .lock()
            .unwrap()
            .iter()
            .any(|e| matches!(e, SemanticEvent::OutputText { text, .. } if !text.is_empty()));
        assert!(
            has_non_empty_output_text,
            "fixture must cause at least one non-empty OutputText emission"
        );

        // Also assert that the assistant_text field was accumulated by
        // the parser (so the summary reflects it).
        let summary_text: String = events
            .lock()
            .unwrap()
            .iter()
            .filter_map(|e| match e {
                SemanticEvent::OutputText { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect();
        assert!(
            !summary_text.is_empty(),
            "concatenated OutputText must be non-empty"
        );
    }

    #[test]
    fn tool_use_event_emits_only_tool_result_not_synthesized_call() {
        let (events, mut parser) = new_parser();
        parser
            .feed_line(r#"{"type":"step_start","sessionID":"ses_1"}"#)
            .unwrap();
        parser
            .feed_line(r#"{"type":"tool_use","part":{"id":"t1","tool":"bash","state":{"status":"completed","input":{"command":"ls"},"output":"file.txt"}}}"#)
            .unwrap();
        let kinds: Vec<&'static str> = events
            .lock()
            .unwrap()
            .iter()
            .map(|e| e.kind_str())
            .collect();
        let n_calls = kinds.iter().filter(|k| **k == "tool_call").count();
        let n_results = kinds.iter().filter(|k| **k == "tool_result").count();
        assert_eq!(
            n_calls, 0,
            "must not synthesize a ToolCall when only a completion was observed; got {kinds:?}"
        );
        assert_eq!(n_results, 1, "must emit exactly one ToolResult");
    }

    #[test]
    fn reasoning_event_emits_semantic_reasoning() {
        let (events, mut parser) = new_parser();
        parser
            .feed_line(r#"{"type":"reasoning","text":"weighing options"}"#)
            .unwrap();
        let collected = events.lock().unwrap().clone();
        let kinds: Vec<&'static str> = collected.iter().map(|e| e.kind_str()).collect();
        assert_eq!(
            kinds,
            vec!["reasoning"],
            "OpenCode `reasoning` event must route to SemanticEvent::Reasoning, \
             not fall through to ProviderExtension; got {kinds:?}"
        );
        let SemanticEvent::Reasoning { text, .. } = &collected[0] else {
            panic!("expected Reasoning");
        };
        assert_eq!(text, "weighing options");
    }

    #[test]
    fn reasoning_with_empty_text_emits_nothing() {
        let (events, mut parser) = new_parser();
        parser
            .feed_line(r#"{"type":"reasoning","text":""}"#)
            .unwrap();
        parser
            .feed_line(r#"{"type":"reasoning","part":{"text":""}}"#)
            .unwrap();
        parser.feed_line(r#"{"type":"reasoning"}"#).unwrap();
        let collected = events.lock().unwrap().clone();
        assert!(
            collected.is_empty(),
            "empty / missing reasoning text must not emit any semantic event; got {collected:?}"
        );
    }

    #[test]
    fn tool_start_tool_end_pair_preserves_cached_input_on_result() {
        // Per the 2026-04-18 OpenCode reporting contract, a `tool_start`'s
        // input must survive into the paired `tool_end` ToolResult so
        // renderers can annotate successful incoming events with the same
        // slot content the outgoing arrow used.
        let (events, mut parser) = new_parser();
        parser
            .feed_line(
                r#"{"type":"tool_start","part":{"id":"t1","tool_name":"bash","input":{"command":"ls -la"}}}"#,
            )
            .unwrap();
        parser
            .feed_line(
                r#"{"type":"tool_end","part":{"tool_use_id":"t1","status":"success","content":"ok"}}"#,
            )
            .unwrap();
        let collected = events.lock().unwrap().clone();
        let result = collected
            .iter()
            .find(|e| matches!(e, SemanticEvent::ToolResult { .. }))
            .expect("expected a ToolResult");
        let SemanticEvent::ToolResult { extra, .. } = result else {
            unreachable!()
        };
        let input = extra
            .get("input")
            .expect("extra['input'] must survive into ToolResult");
        assert_eq!(input.get("command").and_then(Value::as_str), Some("ls -la"));
    }

    #[test]
    fn tool_end_wire_input_wins_over_cached_input() {
        // When the wire `tool_end` payload carries its own `input` (rare
        // but permitted), the parser must prefer it over the cached
        // request-side input so we never overwrite fresher data.
        let (events, mut parser) = new_parser();
        parser
            .feed_line(
                r#"{"type":"tool_start","part":{"id":"t1","tool_name":"bash","input":{"command":"ls"}}}"#,
            )
            .unwrap();
        parser
            .feed_line(
                r#"{"type":"tool_end","part":{"tool_use_id":"t1","status":"success","content":"ok","input":{"command":"pwd"}}}"#,
            )
            .unwrap();
        let collected = events.lock().unwrap().clone();
        let result = collected
            .iter()
            .find(|e| matches!(e, SemanticEvent::ToolResult { .. }))
            .expect("expected a ToolResult");
        let SemanticEvent::ToolResult { extra, .. } = result else {
            unreachable!()
        };
        assert_eq!(
            extra
                .get("input")
                .and_then(|v| v.get("command"))
                .and_then(Value::as_str),
            Some("pwd"),
            "wire-provided input must win over cached input"
        );
    }

    #[test]
    fn tool_use_event_still_increments_tool_calls_counter() {
        // Ensure the trailer count matches the rendered-line count by keeping
        // `tool_calls += 1` even though no ToolCall event is emitted.
        let (_events, mut parser) = new_parser();
        parser
            .feed_line(r#"{"type":"step_start","sessionID":"ses_1"}"#)
            .unwrap();
        parser
            .feed_line(r#"{"type":"tool_use","part":{"id":"t1","tool":"bash","state":{"status":"completed","input":{"command":"ls"},"output":"file.txt"}}}"#)
            .unwrap();
        parser
            .feed_line(r#"{"type":"tool_use","part":{"id":"t2","tool":"bash","state":{"status":"completed","input":{"command":"pwd"},"output":"/"}}}"#)
            .unwrap();
        let summary = Box::new(parser).finish(0);
        assert_eq!(summary.tool_calls, Some(2), "both completions must count");
    }

    #[test]
    fn task_started_becomes_subagent_start() {
        let (events, mut parser) = new_parser();
        parser
            .feed_line(r#"{"type":"task_started","task_id":"sa1","name":"researcher"}"#)
            .unwrap();
        let collected = events.lock().unwrap().clone();
        match &collected[0] {
            SemanticEvent::SubagentStart { id, name, .. } => {
                assert_eq!(id.as_deref(), Some("sa1"));
                assert_eq!(name.as_deref(), Some("researcher"));
            }
            other => panic!("expected SubagentStart, got {other:?}"),
        }
    }

    #[test]
    fn task_completed_becomes_subagent_stop() {
        let (events, mut parser) = new_parser();
        parser
            .feed_line(
                r#"{"type":"task_completed","task_id":"sa1","name":"researcher","status":"success"}"#,
            )
            .unwrap();
        let collected = events.lock().unwrap().clone();
        match &collected[0] {
            SemanticEvent::SubagentStop {
                id, name, status, ..
            } => {
                assert_eq!(id.as_deref(), Some("sa1"));
                assert_eq!(name.as_deref(), Some("researcher"));
                assert_eq!(status.as_deref(), Some("success"));
            }
            other => panic!("expected SubagentStop, got {other:?}"),
        }
    }

    #[test]
    fn task_progress_becomes_info() {
        let (events, mut parser) = new_parser();
        parser
            .feed_line(r#"{"type":"task_progress","message":"working"}"#)
            .unwrap();
        let collected = events.lock().unwrap().clone();
        match &collected[0] {
            SemanticEvent::Info { message, .. } => assert_eq!(message, "working"),
            other => panic!("expected Info, got {other:?}"),
        }
    }

    #[test]
    fn opencode_task_completion_no_longer_synthesizes_subagent_lifecycle() {
        // Phase 4 (2026-05-12): the structured stderr stream is now the
        // authoritative source for OpenCode subagent lifecycle. The stdout
        // NDJSON parser no longer synthesizes SubagentStart/SubagentStop
        // from `task` tool completions — those events come from
        // `service=session ... parentID=...` and the matching `exiting loop`
        // record on stderr.
        let (events, mut parser) = new_parser();
        parser
            .feed_line(r#"{"type":"step_start","sessionID":"ses_1"}"#)
            .unwrap();
        parser
            .feed_line(
                r#"{"type":"tool_use","part":{"id":"t1","tool":"task",
                 "state":{"status":"completed",
                          "input":{"description":"Commit wrap CLI refactor","subagent_type":"coder"},
                          "metadata":{"sessionId":"child-ses-1"},
                          "time":{"start":1715340000000},
                          "output":"ok"}}}"#,
            )
            .unwrap();

        let collected = events.lock().unwrap().clone();
        let ks: Vec<&str> = collected.iter().map(|e| e.kind_str()).collect();
        assert_eq!(
            ks,
            vec!["session_start", "info", "tool_result"],
            "task tool completion must emit only ToolResult; subagent lifecycle now comes from stderr"
        );

        // Verify NO subagent_start / subagent_stop in the stdout stream
        let n_subagent_starts = ks.iter().filter(|k| **k == "subagent_start").count();
        let n_subagent_stops = ks.iter().filter(|k| **k == "subagent_stop").count();
        assert_eq!(n_subagent_starts, 0);
        assert_eq!(n_subagent_stops, 0);

        // subagent_done_count must NOT increment from stdout-side events
        let mut metrics = LiveMetricsState::default();
        let now = Instant::now();
        for event in &collected {
            metrics.observe_event(event, now);
        }
        assert_eq!(
            metrics.subagent_done_count, 0,
            "stdout-side task completion must not increment subagent_done_count"
        );
    }

    #[test]
    fn opencode_task_error_completion_no_longer_synthesizes_subagent_lifecycle() {
        // Companion to opencode_task_completion_no_longer_synthesizes_subagent_lifecycle:
        // even when a `task` tool errors out, no SubagentStart/SubagentStop
        // is synthesized from stdout. The matching subagent stop comes from
        // the stderr `exiting loop` record for the child session.
        let (events, mut parser) = new_parser();
        parser
            .feed_line(r#"{"type":"step_start","sessionID":"ses_1"}"#)
            .unwrap();
        parser
            .feed_line(
                r#"{"type":"tool_use","part":{"id":"t-err","tool":"task",
                 "state":{"status":"error",
                          "input":{"description":"Failed subagent","subagent_type":"coder"},
                          "metadata":{"sessionId":"child-ses-err"},
                          "error":"agent crashed",
                          "output":""}}}"#,
            )
            .unwrap();

        let collected = events.lock().unwrap().clone();
        let ks: Vec<&str> = collected.iter().map(|e| e.kind_str()).collect();
        assert_eq!(
            ks,
            vec!["session_start", "info", "tool_result"],
            "task tool error completion must emit only ToolResult"
        );

        // ToolResult must still carry the error status so it is rendered.
        match &collected[2] {
            SemanticEvent::ToolResult { status, .. } => {
                assert_eq!(status.as_deref(), Some("error"));
            }
            other => panic!("expected ToolResult, got {other:?}"),
        }

        let mut metrics = LiveMetricsState::default();
        let now = Instant::now();
        for event in &collected {
            metrics.observe_event(event, now);
        }
        assert_eq!(
            metrics.subagent_done_count, 0,
            "stdout-side task error completion must not increment subagent_done_count"
        );
    }

    #[test]
    fn opencode_non_task_tool_does_not_synthesize_subagent_lifecycle() {
        let (events, mut parser) = new_parser();
        parser
            .feed_line(r#"{"type":"step_start","sessionID":"ses_1"}"#)
            .unwrap();
        parser
            .feed_line(
                r#"{"type":"tool_use","part":{"id":"t1","tool":"bash",
                 "state":{"status":"completed","input":{"command":"ls -la"},"output":"file.txt"}}}"#,
            )
            .unwrap();

        let collected = events.lock().unwrap().clone();
        let ks: Vec<&str> = collected.iter().map(|e| e.kind_str()).collect();
        let n_subagent_starts = ks.iter().filter(|k| **k == "subagent_start").count();
        let n_subagent_stops = ks.iter().filter(|k| **k == "subagent_stop").count();
        assert_eq!(
            n_subagent_starts, 0,
            "non-task tool must not synthesize SubagentStart; got {ks:?}"
        );
        assert_eq!(
            n_subagent_stops, 0,
            "non-task tool must not synthesize SubagentStop; got {ks:?}"
        );
        assert_eq!(
            ks,
            vec!["session_start", "info", "tool_result"],
            "bash tool must emit only Info + ToolResult"
        );
    }
}
