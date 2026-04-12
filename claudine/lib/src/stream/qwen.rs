use std::collections::HashMap;

use serde_json::Value;

use super::parser::{EventMeta, StreamChunk, StreamEventSink, StreamParseError, StreamParser};
use super::protocol::qwen::{
    QwenErrorEvent, QwenEvent, QwenInit, QwenMessage, QwenResult, QwenTool,
};
use super::summary::StreamExecutionSummary;
use super::token_usage::NormalizedTokenUsage;
use crate::events::Provider;

/// Stream parser for Qwen CLI's `stream-json` format.
///
/// Shares Gemini-style parsing logic where event shapes match,
/// but tolerates Qwen-specific event names and result envelopes.
pub struct QwenStreamParser<S: StreamEventSink> {
    sink: S,
    line_num: usize,
    session_id: Option<String>,
    model: Option<String>,
    assistant_text: String,
    token_usage: Option<NormalizedTokenUsage>,
    cost_usd: Option<f64>,
    duration_ms: Option<u64>,
    num_turns: Option<u32>,
    tool_calls: u32,
    provider_status: Option<String>,
    is_error: bool,
    error_kind: Option<String>,
    error_message: Option<String>,
    raw_summary: Option<Value>,
    tool_uses: HashMap<String, (Option<String>, Option<Value>)>,
}

impl<S: StreamEventSink> QwenStreamParser<S> {
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
            num_turns: None,
            tool_calls: 0,
            provider_status: None,
            is_error: false,
            error_kind: None,
            error_message: None,
            raw_summary: None,
            tool_uses: HashMap::new(),
        }
    }

    fn handle_init(&mut self, init: QwenInit) {
        self.session_id = init.session_id;
        self.model = init.model;
        super::trace_session_metadata(
            Provider::QwenCode,
            self.session_id.as_deref(),
            self.model.as_deref(),
        );

        let mut meta = EventMeta::default();
        if let Some(session_id) = &self.session_id {
            meta.extra
                .insert("session_id".into(), Value::String(session_id.clone()));
        }
        if let Some(model) = &self.model {
            meta.extra
                .insert("model".into(), Value::String(model.clone()));
        }
        self.sink.on_session_start(&meta);
    }

    /// `from_assistant_type` is true when the event type was `assistant` —
    /// in that case we skip the `role == "assistant"` filter so messages
    /// dispatched via the type alias are still accepted.
    fn handle_message(
        &mut self,
        msg: QwenMessage,
        from_assistant_type: bool,
    ) -> Option<StreamChunk> {
        if !from_assistant_type && msg.role.as_deref() != Some("assistant") {
            return None;
        }

        let content = msg.content?;

        // Try content array (Gemini-style)
        if let Some(parts) = content.as_array() {
            let mut text_parts = String::new();
            for part in parts {
                if let Some(text) = part.get("text").and_then(|t| t.as_str()) {
                    text_parts.push_str(text);
                }
            }
            if !text_parts.is_empty() {
                self.assistant_text.push_str(&text_parts);
                return Some(StreamChunk::Text(super::ensure_message_newline(text_parts)));
            }
        }

        // Try content as string (Qwen-specific)
        if let Some(text) = content.as_str()
            && !text.is_empty()
        {
            self.assistant_text.push_str(text);
            return Some(StreamChunk::Text(super::ensure_message_newline(
                text.to_string(),
            )));
        }

        None
    }

    fn handle_result(&mut self, result: QwenResult, raw: Value) {
        let (usage, meta) = result.resolved_usage();
        self.duration_ms = meta.duration_ms;
        self.num_turns = meta.num_turns.map(|v| v as u32);
        self.provider_status = meta.stop_reason;
        self.cost_usd = meta.cost_usd;

        if let Some(usage) = usage {
            let input = usage.input_tokens;
            let output = usage.output_tokens;
            let cache_read = usage.cache_read_input_tokens;
            let total = match (input, output) {
                (Some(i), Some(o)) => Some(i + o),
                _ => usage.total_tokens,
            };
            self.token_usage = Some(NormalizedTokenUsage {
                input,
                output,
                total,
                cache_read,
            });
        }

        self.raw_summary = Some(raw);
        super::trace_summary_update(
            Provider::QwenCode,
            self.provider_status.as_deref(),
            self.duration_ms,
            self.cost_usd,
        );
    }

    fn handle_error(&mut self, event: QwenErrorEvent) {
        self.is_error = true;
        let detail = event.error;
        self.error_kind = detail.as_ref().and_then(|d| d.kind.clone());
        self.error_message = detail.and_then(|d| d.message);
        let mut meta = EventMeta::default();
        if let Some(message) = &self.error_message {
            meta.extra
                .insert("error_message".into(), Value::String(message.clone()));
        }
        if let Some(kind) = &self.error_kind {
            meta.extra
                .insert("error_kind".into(), Value::String(kind.clone()));
        }
        self.sink.on_turn_error(&meta);
    }

    fn handle_tool_use(&mut self, mut tool: QwenTool) {
        self.tool_calls += 1;
        let tool_id = tool.resolved_tool_id().map(ToOwned::to_owned);
        let tool_name = tool.resolved_tool_name().map(ToOwned::to_owned);
        let tool_input = tool.take_input();
        super::trace_tool_event(Provider::QwenCode, self.tool_calls, tool_name.as_deref());
        let mut meta = EventMeta::default();
        if let Some(tool_id) = &tool_id {
            meta.extra
                .insert("tool_id".into(), Value::String(tool_id.clone()));
        }
        if let Some(tool_name) = &tool_name {
            meta.extra
                .insert("tool_name".into(), Value::String(tool_name.clone()));
        }
        if let Some(tool_input) = &tool_input {
            meta.extra.insert("tool_input".into(), tool_input.clone());
        }
        if let Some(tool_id) = tool_id {
            self.tool_uses.insert(tool_id, (tool_name, tool_input));
        }
        self.sink.on_before_tool(&meta);
    }

    fn handle_tool_result(&mut self, mut tool: QwenTool) {
        let tool_id = tool.resolved_tool_id().map(ToOwned::to_owned);
        let (tool_name, tool_input) = tool_id
            .as_ref()
            .and_then(|id| self.tool_uses.remove(id))
            .unwrap_or((None, None));
        let tool_output = tool.take_output();
        let status = tool.status.take();
        let error = tool.error.take();

        let mut meta = EventMeta::default();
        if let Some(tool_id) = tool_id {
            meta.extra.insert("tool_id".into(), Value::String(tool_id));
        }
        if let Some(tool_name) = tool_name {
            meta.extra
                .insert("tool_name".into(), Value::String(tool_name));
        }
        if let Some(tool_input) = tool_input {
            meta.extra.insert("tool_input".into(), tool_input);
        }
        if let Some(tool_output) = tool_output {
            meta.extra.insert("tool_response".into(), tool_output);
        }
        if let Some(status) = status {
            meta.extra.insert("status".into(), Value::String(status));
        }
        if let Some(error) = error {
            meta.extra.insert("error".into(), error);
        }
        self.sink.on_after_tool(&meta);
    }
}

impl<S: StreamEventSink + Send> StreamParser for QwenStreamParser<S> {
    fn feed_line(&mut self, line: &str) -> Result<Option<StreamChunk>, StreamParseError> {
        self.line_num += 1;
        let line = line.trim();
        if line.is_empty() {
            return Ok(None);
        }

        let raw: Value = serde_json::from_str(line).map_err(|e| {
            self.sink
                .on_warning(&format!("Malformed JSON on line {}: {e}", self.line_num));
            super::trace_malformed_line(Provider::QwenCode, self.line_num, &e.to_string());
            StreamParseError::MalformedLine {
                line_num: self.line_num,
                message: e.to_string(),
            }
        })?;

        let event_type = raw.get("type").and_then(|t| t.as_str()).unwrap_or("");
        super::trace_parser_event(Provider::QwenCode, event_type, self.line_num);

        match serde_json::from_value::<QwenEvent>(raw.clone()) {
            Ok(QwenEvent::Init(init)) => {
                self.handle_init(init);
                Ok(None)
            }
            Ok(QwenEvent::System(sys)) => {
                if sys.is_session_start() {
                    self.handle_init(sys.into_init());
                }
                Ok(None)
            }
            Ok(QwenEvent::Message(msg) | QwenEvent::AssistantMessage(msg)) => {
                Ok(self.handle_message(msg, false))
            }
            Ok(QwenEvent::Assistant(msg)) => Ok(self.handle_message(msg, true)),
            Ok(QwenEvent::Error(err)) => {
                self.handle_error(err);
                Ok(None)
            }
            Ok(QwenEvent::Result(result) | QwenEvent::Summary(result)) => {
                self.handle_result(result, raw);
                Ok(None)
            }
            Ok(QwenEvent::ToolUse(tool) | QwenEvent::ToolCall(tool)) => {
                self.handle_tool_use(tool);
                Ok(None)
            }
            Ok(QwenEvent::ToolResult(tool) | QwenEvent::ToolResponse(tool)) => {
                self.handle_tool_result(tool);
                Ok(None)
            }
            Err(_) => Ok(None),
        }
    }

    fn finish(self: Box<Self>, exit_code: i32) -> StreamExecutionSummary {
        StreamExecutionSummary {
            provider: Provider::QwenCode,
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
            num_turns: self.num_turns,
            token_usage: self.token_usage,
            cost_usd: self.cost_usd,
            tool_calls: if self.tool_calls > 0 {
                Some(self.tool_calls)
            } else {
                None
            },
            rate_limit: None,
            context_usage: None,
            raw_summary: self.raw_summary,
            stderr_text: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stream::parser::{EventMeta, NullSink, StreamEventSink};
    use crate::stream::test_support::{ToolContractExpectation, assert_tool_event_contract};

    #[derive(Default)]
    struct ToolRecordingSink {
        before_tool: Vec<EventMeta>,
        after_tool: Vec<EventMeta>,
    }

    impl StreamEventSink for ToolRecordingSink {
        fn on_before_tool(&mut self, meta: &EventMeta) {
            self.before_tool.push(meta.clone());
        }

        fn on_after_tool(&mut self, meta: &EventMeta) {
            self.after_tool.push(meta.clone());
        }
    }

    fn make_parser() -> Box<QwenStreamParser<NullSink>> {
        Box::new(QwenStreamParser::new(NullSink))
    }

    #[test]
    fn happy_path() {
        let mut parser = make_parser();

        parser
            .feed_line(r#"{"type":"init","session_id":"qw-1","model":"qwen-coder-plus"}"#)
            .unwrap();

        let text = parser
            .feed_line(
                r#"{"type":"message","role":"assistant","content":[{"text":"Hello from Qwen"}]}"#,
            )
            .unwrap();
        assert_eq!(text, Some(StreamChunk::Text("Hello from Qwen\n".into())));

        parser
            .feed_line(r#"{"type":"result","duration_ms":5000,"usage":{"input_tokens":300,"output_tokens":150}}"#)
            .unwrap();

        let summary = parser.finish(0);
        assert_eq!(summary.provider, Provider::QwenCode);
        assert_eq!(summary.assistant_text, "Hello from Qwen");

        let usage = summary.token_usage.unwrap();
        assert_eq!(usage.input, Some(300));
        assert_eq!(usage.output, Some(150));
        assert_eq!(usage.total, Some(450));
    }

    #[test]
    fn qwen_specific_event_names() {
        let mut parser = make_parser();

        // Qwen-specific message type
        let text = parser
            .feed_line(
                r#"{"type":"assistant_message","role":"assistant","content":"String content"}"#,
            )
            .unwrap();
        assert_eq!(text, Some(StreamChunk::Text("String content\n".into())));

        // Qwen-specific tool event names
        parser
            .feed_line(r#"{"type":"tool_call","name":"search"}"#)
            .unwrap();
        parser
            .feed_line(r#"{"type":"tool_response","content":"result"}"#)
            .unwrap();

        // Qwen-specific result name
        parser
            .feed_line(
                r#"{"type":"summary","duration_ms":3000,"token_usage":{"input_tokens":100,"output_tokens":50}}"#,
            )
            .unwrap();

        let summary = parser.finish(0);
        assert_eq!(summary.tool_calls, Some(1));
        assert_eq!(summary.duration_ms, Some(3000));

        let usage = summary.token_usage.unwrap();
        assert_eq!(usage.input, Some(100));
    }

    #[test]
    fn qwen_hook_design_session_and_assistant_events_are_supported() {
        let mut parser = make_parser();

        parser
            .feed_line(r#"{"type":"system","subtype":"session_start","session_id":"qw-2","model":"qwen3-coder"}"#)
            .unwrap();
        let text = parser
            .feed_line(r#"{"type":"assistant","content":[{"text":"Hook design assistant event"}]}"#)
            .unwrap();

        assert_eq!(
            text,
            Some(StreamChunk::Text("Hook design assistant event\n".into()))
        );

        let summary = parser.finish(0);
        assert_eq!(summary.session_id.as_deref(), Some("qw-2"));
        assert_eq!(summary.model.as_deref(), Some("qwen3-coder"));
    }

    #[test]
    fn content_as_string() {
        let mut parser = make_parser();
        let text = parser
            .feed_line(r#"{"type":"message","role":"assistant","content":"Plain string content"}"#)
            .unwrap();
        assert_eq!(
            text,
            Some(StreamChunk::Text("Plain string content\n".into()))
        );
    }

    #[test]
    fn tool_events_preserve_parameters_and_results() {
        let mut parser = Box::new(QwenStreamParser::new(ToolRecordingSink::default()));

        parser
            .feed_line(
                r#"{"type":"tool_call","id":"q1","name":"bash","input":{"command":"git status"}}"#,
            )
            .unwrap();
        parser
            .feed_line(
                r#"{"type":"tool_response","tool_use_id":"q1","status":"success","content":"clean"}"#,
            )
            .unwrap();

        let sink = parser.sink;
        assert_eq!(sink.before_tool.len(), 1);
        assert_eq!(sink.after_tool.len(), 1);
        assert_tool_event_contract(
            &sink.before_tool[0],
            Some(&sink.after_tool[0]),
            ToolContractExpectation {
                name: "bash",
                id: Some("q1"),
                input_field: Some(("command", "git status")),
                status: Some("success"),
                response: Some(Value::String("clean".into())),
            },
        );
    }
}
