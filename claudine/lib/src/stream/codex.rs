use std::collections::HashMap;

use serde_json::Value;

use super::parser::{EventMeta, StreamChunk, StreamEventSink, StreamParseError, StreamParser};
use super::protocol::codex::{
    CodexAgentMessage, CodexErrorEnvelope, CodexEvent, CodexItem, CodexItemEnvelope,
    CodexPermissionItem, CodexThreadMeta, CodexToolItemFields, CodexTurnCompleted,
};
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
    tool_items: HashMap<String, CodexToolItemFields>,
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

    fn handle_thread_started(&mut self, meta: CodexThreadMeta) {
        self.session_id = meta.resolved_id();
        super::trace_session_metadata(
            Provider::Codex,
            self.session_id.as_deref(),
            self.model.as_deref(),
        );

        let session_meta = self.session_meta();
        self.sink.on_session_start(&session_meta);
    }

    fn handle_turn_started(&mut self) {
        self.num_turns += 1;
        let meta = self.session_meta();
        self.sink.on_turn_start(&meta);
    }

    fn handle_turn_completed(&mut self, tc: CodexTurnCompleted, raw: Value) {
        let provider_status = tc.provider_status().map(String::from);

        if let Some(usage) = &tc.usage {
            let input = usage.input_tokens;
            let output = usage.output_tokens;
            let cache_read = usage.cache_read();
            let total = match (input, output) {
                (Some(i), Some(o)) => Some(i + o),
                _ => usage.total_tokens,
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

        self.duration_ms = tc.duration_ms;
        self.cost_usd = tc.cost_usd;
        self.provider_status = provider_status;

        self.raw_summary = Some(raw);
        super::trace_summary_update(
            Provider::Codex,
            self.provider_status.as_deref(),
            self.duration_ms,
            self.cost_usd,
        );

        let mut meta = self.session_meta();
        if let Some(status) = &self.provider_status {
            meta.extra
                .insert("provider_status".into(), Value::String(status.clone()));
        }
        self.sink.on_turn_complete(&meta);
    }

    fn handle_error(&mut self, env: CodexErrorEnvelope) {
        self.is_error = true;
        self.error_kind = env.resolved_kind();
        self.error_message = env.resolved_message();

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
    fn handle_agent_message_item(&mut self, msg: &CodexAgentMessage) {
        if let Some(text) = msg.collected_text() {
            self.assistant_text.push_str(&text);
        }
    }

    fn tool_meta_from_fields(&self, fields: &CodexToolItemFields) -> EventMeta {
        let mut meta = self.session_meta();
        if let Some(tool_name) = fields.resolved_tool_name() {
            meta.extra
                .insert("tool_name".into(), Value::String(tool_name.to_string()));
        }
        if let Some(tool_id) = fields.resolved_tool_id() {
            meta.extra
                .insert("tool_id".into(), Value::String(tool_id.to_string()));
        }
        if let Some(input) = fields.resolved_input() {
            meta.extra.insert("tool_input".into(), input.clone());
        }
        if let Some(output) = fields.resolved_output() {
            meta.extra.insert("tool_response".into(), output.clone());
        }
        meta
    }

    fn permission_meta(&self, perm: &CodexPermissionItem) -> EventMeta {
        let mut meta = self.session_meta();
        if let Some(name) = perm.name.as_deref() {
            meta.extra
                .insert("tool_name".into(), Value::String(name.to_string()));
        }
        if let Some(id) = perm.id.as_deref() {
            meta.extra
                .insert("tool_id".into(), Value::String(id.to_string()));
        }
        meta
    }

    fn handle_item_started(&mut self, env: CodexItemEnvelope) {
        let Some(item) = env.item else {
            return;
        };

        if let Some(perm) = item.as_permission() {
            let meta = self.permission_meta(perm);
            self.sink.on_permission_request(&meta);
            return;
        }

        if item.is_tool_item() {
            let fields = item
                .as_tool_fields()
                .expect("is_tool_item implies tool fields");
            self.tool_calls += 1;
            super::trace_tool_event(
                Provider::Codex,
                self.tool_calls,
                fields.resolved_tool_name(),
            );
            let meta = self.tool_meta_from_fields(fields);
            if let Some(id) = fields.id.clone()
                && let Some(owned_fields) = item.into_tool_fields()
            {
                self.tool_items.insert(id, owned_fields);
            }
            self.sink.on_before_tool(&meta);
        }
    }

    fn handle_item_completed(&mut self, env: CodexItemEnvelope) -> Option<StreamChunk> {
        let item = env.item?;

        if let Some(msg) = item.as_agent_message() {
            self.handle_agent_message_item(msg);
            return None;
        }

        if item.is_tool_item() {
            let id = item.as_tool_fields().and_then(|f| f.id.clone());
            let merged_item = if let Some(id) = &id
                && let Some(started) = self.tool_items.remove(id)
            {
                // Wrap started back into the same variant so merge_started runs
                // through CodexItem and we can keep using the enum API.
                let started_item = CodexItem::ToolUse(started);
                item.merge_started(started_item)
            } else {
                item
            };
            let fields = merged_item
                .as_tool_fields()
                .expect("is_tool_item implies tool fields");
            let meta = self.tool_meta_from_fields(fields);
            self.sink.on_after_tool(&meta);
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

        // Parse as Value first so we preserve the existing Fatal error path
        // for malformed JSON and keep the raw value for `turn.completed` raw
        // summaries.
        let raw: Value = serde_json::from_str(line).map_err(|e| {
            self.sink
                .on_warning(&format!("Malformed JSON on line {}: {e}", self.line_num));
            super::trace_malformed_line(Provider::Codex, self.line_num, &e.to_string());
            StreamParseError::Fatal(format!("Malformed JSON on line {}: {e}", self.line_num))
        })?;

        let event_type = raw.get("type").and_then(|t| t.as_str()).unwrap_or("");
        super::trace_parser_event(Provider::Codex, event_type, self.line_num);

        match serde_json::from_value::<CodexEvent>(raw.clone()) {
            Ok(CodexEvent::ThreadCreated(meta) | CodexEvent::ThreadStarted(meta)) => {
                self.handle_thread_started(meta);
                Ok(None)
            }
            Ok(CodexEvent::TurnStarted(_)) => {
                self.handle_turn_started();
                Ok(None)
            }
            Ok(CodexEvent::TurnCompleted(tc)) => {
                self.handle_turn_completed(tc, raw);
                Ok(None)
            }
            Ok(
                CodexEvent::Error(err)
                | CodexEvent::TurnError(err)
                | CodexEvent::TurnFailed(err)
                | CodexEvent::StreamError(err),
            ) => {
                self.handle_error(err);
                Ok(None)
            }
            Ok(CodexEvent::ItemStarted(env)) => {
                self.handle_item_started(env);
                Ok(None)
            }
            Ok(CodexEvent::ItemCompleted(env)) => Ok(self.handle_item_completed(env)),
            Ok(CodexEvent::ItemToolUse(fields) | CodexEvent::ToolUse(fields)) => {
                self.tool_calls += 1;
                super::trace_tool_event(
                    Provider::Codex,
                    self.tool_calls,
                    fields.resolved_tool_name(),
                );
                let meta = self.tool_meta_from_fields(&fields);
                self.sink.on_before_tool(&meta);
                Ok(None)
            }
            Ok(CodexEvent::ItemToolResult(fields) | CodexEvent::ToolResult(fields)) => {
                let meta = self.tool_meta_from_fields(&fields);
                self.sink.on_after_tool(&meta);
                Ok(None)
            }
            Err(_) => {
                // Codex stream is control-plane oriented; unknown events are
                // silently skipped, matching the prior behavior.
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
