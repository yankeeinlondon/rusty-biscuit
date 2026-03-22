use std::collections::HashMap;

use serde_json::Value;

use super::parser::{EventMeta, StreamChunk, StreamEventSink, StreamParseError, StreamParser};
use super::summary::StreamExecutionSummary;
use super::token_usage::NormalizedTokenUsage;
use crate::events::Provider;

/// Stream parser for Codex CLI's JSONL `exec --json` format.
///
/// Codex streams metadata/control events only. The assistant text
/// is NOT sourced from the stream — it comes from a separate
/// `--output-last-message` temp file read after child exit.
///
/// The parser tracks thread lifecycle, turn events, and token usage
/// from `turn.completed` events.
pub struct CodexStreamParser<S: StreamEventSink> {
    sink: S,
    line_num: usize,
    session_id: Option<String>,
    model: Option<String>,
    token_usage: Option<NormalizedTokenUsage>,
    cost_usd: Option<f64>,
    duration_ms: Option<u64>,
    num_turns: u32,
    tool_calls: u32,
    provider_status: Option<String>,
    is_error: bool,
    error_kind: Option<String>,
    error_message: Option<String>,
    raw_summary: Option<Value>,
    assistant_text: String,
    tool_items: HashMap<String, Value>,
}

impl<S: StreamEventSink> CodexStreamParser<S> {
    pub fn new(sink: S, model: Option<String>) -> Self {
        Self {
            sink,
            line_num: 0,
            session_id: None,
            model,
            token_usage: None,
            cost_usd: None,
            duration_ms: None,
            num_turns: 0,
            tool_calls: 0,
            provider_status: None,
            is_error: false,
            error_kind: None,
            error_message: None,
            raw_summary: None,
            assistant_text: String::new(),
            tool_items: HashMap::new(),
        }
    }

    fn session_meta(&self) -> EventMeta {
        let mut meta = EventMeta::default();
        if let Some(session_id) = &self.session_id {
            meta.extra
                .insert("session_id".into(), Value::String(session_id.clone()));
        }
        if let Some(model) = &self.model {
            meta.extra
                .insert("model".into(), Value::String(model.clone()));
        }
        meta
    }

    fn handle_thread_started(&mut self, obj: &Value) {
        self.session_id = obj
            .get("thread_id")
            .or_else(|| obj.get("id"))
            .and_then(|v| v.as_str())
            .map(String::from);

        let meta = self.session_meta();
        self.sink.on_session_start(&meta);
    }

    fn handle_turn_started(&mut self) {
        self.num_turns += 1;
        let meta = self.session_meta();
        self.sink.on_turn_start(&meta);
    }

    fn handle_turn_completed(&mut self, obj: &Value) {
        // Extract usage from turn.completed
        if let Some(usage) = obj.get("usage") {
            let input = usage.get("input_tokens").and_then(|v| v.as_u64());
            let output = usage.get("output_tokens").and_then(|v| v.as_u64());
            let cache_read = usage
                .get("cached_input_tokens")
                .or_else(|| usage.get("cache_read_input_tokens"))
                .and_then(|v| v.as_u64());
            let total = match (input, output) {
                (Some(i), Some(o)) => Some(i + o),
                _ => usage.get("total_tokens").and_then(|v| v.as_u64()),
            };
            let step_usage = NormalizedTokenUsage {
                input,
                output,
                total,
                cache_read,
            };
            // Merge (last snapshot wins for Codex)
            match &mut self.token_usage {
                Some(existing) => existing.merge(&step_usage),
                None => self.token_usage = Some(step_usage),
            }
        }

        self.duration_ms = obj.get("duration_ms").and_then(|v| v.as_u64());
        self.cost_usd = obj.get("cost_usd").and_then(|v| v.as_f64());

        self.provider_status = obj
            .get("status")
            .or_else(|| obj.get("stop_reason"))
            .and_then(|v| v.as_str())
            .map(String::from);

        self.raw_summary = Some(obj.clone());

        let mut meta = self.session_meta();
        if let Some(status) = &self.provider_status {
            meta.extra
                .insert("provider_status".into(), Value::String(status.clone()));
        }
        self.sink.on_turn_complete(&meta);
    }

    fn handle_error(&mut self, obj: &Value) {
        self.is_error = true;
        self.error_kind = obj
            .get("error_type")
            .or_else(|| obj.get("error").and_then(|e| e.get("type")))
            .and_then(|t| t.as_str())
            .map(String::from);
        self.error_message = obj
            .get("error_message")
            .or_else(|| obj.get("error").and_then(|e| e.get("message")))
            .or_else(|| obj.get("message"))
            .and_then(|m| m.as_str())
            .map(String::from);

        let mut meta = self.session_meta();
        if let Some(kind) = &self.error_kind {
            meta.extra
                .insert("error_kind".into(), Value::String(kind.clone()));
        }
        if let Some(message) = &self.error_message {
            meta.extra
                .insert("error_message".into(), Value::String(message.clone()));
        }
        self.sink.on_turn_error(&meta);
    }

    /// Accumulate agent message text for fallback use only.
    ///
    /// The authoritative assistant text comes from the `--output-last-message`
    /// temp file, not from the stream. We accumulate stream text as a fallback
    /// but do NOT return it from `feed_line` to avoid emitting it to stdout
    /// (which would prevent the file-based text from being used).
    fn handle_agent_message_item(&mut self, item: &Value) {
        if let Some(text) = item.get("text").and_then(|v| v.as_str()) {
            self.assistant_text.push_str(text);
            return;
        }

        if let Some(parts) = item.get("content").and_then(|v| v.as_array()) {
            let text = parts
                .iter()
                .filter_map(|part| part.get("text").and_then(|v| v.as_str()))
                .collect::<String>();
            if !text.is_empty() {
                self.assistant_text.push_str(&text);
            }
        }
    }

    fn tool_meta_from_item(&self, item: &Value) -> EventMeta {
        let mut meta = self.session_meta();
        if let Some(tool_name) = item
            .get("tool_name")
            .or_else(|| item.get("name"))
            .and_then(|v| v.as_str())
        {
            meta.extra
                .insert("tool_name".into(), Value::String(tool_name.to_string()));
        }
        if let Some(tool_id) = item.get("id").and_then(|v| v.as_str()) {
            meta.extra
                .insert("tool_id".into(), Value::String(tool_id.to_string()));
        }
        if let Some(input) = item
            .get("input")
            .or_else(|| item.get("arguments"))
            .or_else(|| item.get("parameters"))
        {
            meta.extra.insert("tool_input".into(), input.clone());
        }
        if let Some(output) = item
            .get("output")
            .or_else(|| item.get("result"))
            .or_else(|| item.get("content"))
        {
            meta.extra.insert("tool_response".into(), output.clone());
        }
        meta
    }

    fn is_tool_item_type(item_type: &str) -> bool {
        matches!(
            item_type,
            "tool_use"
                | "tool_call"
                | "mcp_tool_call"
                | "web_search"
                | "command_exec"
                | "patch_apply"
                | "image_generation"
                | "view_image"
        )
    }

    fn handle_item_started(&mut self, obj: &Value) {
        let Some(item) = obj.get("item") else {
            return;
        };
        let item_type = item.get("type").and_then(|v| v.as_str()).unwrap_or("");

        if matches!(
            item_type,
            "permission_request" | "approval_request" | "user_input_request"
        ) {
            self.sink
                .on_permission_request(&self.tool_meta_from_item(item));
            return;
        }

        if Self::is_tool_item_type(item_type) {
            self.tool_calls += 1;
            if let Some(id) = item.get("id").and_then(|v| v.as_str()) {
                self.tool_items.insert(id.to_string(), item.clone());
            }
            self.sink.on_before_tool(&self.tool_meta_from_item(item));
        }
    }

    fn handle_item_completed(&mut self, obj: &Value) -> Option<StreamChunk> {
        let Some(item) = obj.get("item") else {
            return None;
        };
        let item_type = item.get("type").and_then(|v| v.as_str()).unwrap_or("");

        if item_type == "agent_message" {
            self.handle_agent_message_item(item);
            return None;
        }

        if Self::is_tool_item_type(item_type) {
            let merged_item = item
                .get("id")
                .and_then(|v| v.as_str())
                .and_then(|id| self.tool_items.remove(id))
                .map(|mut started| {
                    if let (Some(started_map), Some(completed_map)) =
                        (started.as_object_mut(), item.as_object())
                    {
                        for (key, value) in completed_map {
                            started_map.insert(key.clone(), value.clone());
                        }
                    }
                    started
                })
                .unwrap_or_else(|| item.clone());
            self.sink
                .on_after_tool(&self.tool_meta_from_item(&merged_item));
        }

        None
    }
}

impl<S: StreamEventSink + Send> StreamParser for CodexStreamParser<S> {
    fn feed_line(&mut self, line: &str) -> Result<Option<StreamChunk>, StreamParseError> {
        self.line_num += 1;
        let line = line.trim();
        if line.is_empty() {
            return Ok(None);
        }

        let obj: Value = serde_json::from_str(line).map_err(|e| {
            self.sink
                .on_warning(&format!("Malformed JSON on line {}: {e}", self.line_num));
            StreamParseError::Fatal(format!("Malformed JSON on line {}: {e}", self.line_num))
        })?;

        let event_type = obj.get("type").and_then(|t| t.as_str()).unwrap_or("");

        match event_type {
            "thread.created" | "thread.started" => {
                self.handle_thread_started(&obj);
                Ok(None)
            }
            "turn.started" => {
                self.handle_turn_started();
                Ok(None)
            }
            "turn.completed" => {
                self.handle_turn_completed(&obj);
                Ok(None)
            }
            "error" | "turn.error" | "turn.failed" | "stream.error" => {
                self.handle_error(&obj);
                Ok(None)
            }
            "item.started" => {
                self.handle_item_started(&obj);
                Ok(None)
            }
            "item.completed" => Ok(self.handle_item_completed(&obj)),
            "item.tool_use" | "tool_use" => {
                self.tool_calls += 1;
                let meta = self.tool_meta_from_item(&obj);
                self.sink.on_before_tool(&meta);
                Ok(None)
            }
            "item.tool_result" | "tool_result" => {
                let meta = self.tool_meta_from_item(&obj);
                self.sink.on_after_tool(&meta);
                Ok(None)
            }
            _ => {
                // Codex stream is control-plane oriented; assistant text is
                // accumulated for fallback use but never emitted live.
                Ok(None)
            }
        }
    }

    fn finish(self: Box<Self>, exit_code: i32) -> StreamExecutionSummary {
        StreamExecutionSummary {
            provider: Provider::Codex,
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
    use crate::stream::parser::NullSink;

    fn make_parser() -> Box<CodexStreamParser<NullSink>> {
        Box::new(CodexStreamParser::new(NullSink, Some("codex-mini".into())))
    }

    #[test]
    fn happy_path_metadata_only() {
        let mut parser = make_parser();

        // Thread created
        let tc = r#"{"type":"thread.started","thread_id":"thrd-abc"}"#;
        assert_eq!(parser.feed_line(tc).unwrap(), None);

        // Turn started
        parser.feed_line(r#"{"type":"turn.started"}"#).unwrap();

        // Turn completed with usage
        let tc = r#"{"type":"turn.completed","usage":{"input_tokens":200,"output_tokens":100},"duration_ms":5000,"status":"completed"}"#;
        assert_eq!(parser.feed_line(tc).unwrap(), None);

        // Agent message text is accumulated for fallback but NOT emitted live
        // (authoritative text comes from --output-last-message file).
        let streamed = parser
            .feed_line(
                r#"{"type":"item.completed","item":{"id":"item_0","type":"agent_message","text":"Text from stream"}}"#,
            )
            .unwrap();
        assert_eq!(streamed, None);

        let summary = parser.finish(0);
        assert_eq!(summary.provider, Provider::Codex);
        assert_eq!(summary.session_id.as_deref(), Some("thrd-abc"));
        assert_eq!(summary.assistant_text, "Text from stream");
        assert_eq!(summary.model.as_deref(), Some("codex-mini"));
        assert_eq!(summary.num_turns, Some(1));
        assert_eq!(summary.duration_ms, Some(5000));

        let usage = summary.token_usage.unwrap();
        assert_eq!(usage.input, Some(200));
        assert_eq!(usage.output, Some(100));
        assert_eq!(usage.total, Some(300));
    }

    #[test]
    fn stream_accumulates_text_without_emitting() {
        let mut parser = make_parser();
        let result = parser
            .feed_line(
                r#"{"type":"item.completed","item":{"id":"item_0","type":"agent_message","text":"accumulated only"}}"#,
            )
            .unwrap();
        // Text is accumulated for fallback but not returned for live emission
        assert_eq!(result, None);
        let summary = parser.finish(0);
        assert_eq!(summary.assistant_text, "accumulated only");
    }

    #[test]
    fn error_handling() {
        let mut parser = make_parser();
        parser
            .feed_line(
                r#"{"type":"error","error_type":"rate_limit","error_message":"Too many requests"}"#,
            )
            .unwrap();

        let summary = parser.finish(1);
        assert!(summary.is_error);
        assert_eq!(summary.error_kind.as_deref(), Some("rate_limit"));
        assert_eq!(summary.error_message.as_deref(), Some("Too many requests"));
    }

    #[test]
    fn tool_counting() {
        let mut parser = make_parser();
        parser
            .feed_line(r#"{"type":"item.tool_use","name":"bash"}"#)
            .unwrap();
        parser
            .feed_line(r#"{"type":"item.tool_result","status":"ok"}"#)
            .unwrap();
        parser
            .feed_line(r#"{"type":"item.tool_use","name":"write"}"#)
            .unwrap();
        parser
            .feed_line(r#"{"type":"item.tool_result","status":"ok"}"#)
            .unwrap();

        let summary = parser.finish(0);
        assert_eq!(summary.tool_calls, Some(2));
    }
}
