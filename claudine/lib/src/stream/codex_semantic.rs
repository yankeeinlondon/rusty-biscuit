//! Native [`SemanticStreamParser`] implementation for Codex CLI's
//! `exec --json` JSONL format.
//!
//! Addresses the gaps called out in the motivating spec:
//!
//! - Routes `reasoning` items through [`SemanticEvent::Reasoning`] so the
//!   thinking renderer can surface Codex's long reasoning phases instead of
//!   bare elapsed-time heartbeats.
//! - Adds typed handling for `item.updated`, `file_change`, and
//!   `plan_update` / `todo_list` items that the legacy parser silently drops.
//! - Surfaces `command_execution` completion metadata (`status`, `exit_code`)
//!   inside `ToolResult` events.
//! - Preserves every other successfully-parsed line as
//!   [`SemanticEvent::ProviderExtension`] rather than dropping it.
//!
//! Authoritative assistant text still arrives via the
//! `--output-last-message` temp file read in `cli/commands/wrap/exec.rs`; this
//! parser therefore accumulates `agent_message` text into the summary but does
//! NOT emit [`SemanticEvent::OutputText`] for it (doing so would double-emit
//! to stdout). The full event still appears in the semantic log as a
//! `ProviderExtension` so fidelity is preserved.

use std::collections::HashMap;

use serde_json::{Map, Value};

use super::parser::{SemanticStreamParser, StreamParseError};
use super::protocol::codex::{
    CodexAgentMessage, CodexErrorEnvelope, CodexEvent, CodexFileChange, CodexItem,
    CodexItemEnvelope, CodexPermissionItem, CodexPlanUpdate, CodexReasoning, CodexThreadMeta,
    CodexToolItemFields, CodexTurnCompleted,
};
use super::semantic::{SemanticEvent, SemanticEventSink};
use super::summary::StreamExecutionSummary;
use super::token_usage::NormalizedTokenUsage;
use crate::events::Provider;

/// Native stream parser for Codex CLI emitting [`SemanticEvent`]s.
pub struct CodexSemanticStreamParser<S: SemanticEventSink> {
    sink: S,
    line_num: usize,
    session_id: Option<String>,
    model: Option<String>,
    token_usage: Option<NormalizedTokenUsage>,
    cost_usd: Option<f64>,
    duration_ms: Option<u64>,
    num_turns: u32,
    tool_calls: u32,
    permission_prompts: u32,
    user_input_prompts: u32,
    provider_status: Option<String>,
    is_error: bool,
    error_kind: Option<String>,
    error_message: Option<String>,
    raw_summary: Option<Value>,
    assistant_text: String,
    /// Tool-item started snapshots keyed by id, merged into the matching
    /// `item.completed` payload so completion events carry the original input.
    tool_items: HashMap<String, CodexToolItemFields>,
}

impl<S: SemanticEventSink> CodexSemanticStreamParser<S> {
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
            permission_prompts: 0,
            user_input_prompts: 0,
            provider_status: None,
            is_error: false,
            error_kind: None,
            error_message: None,
            raw_summary: None,
            assistant_text: String::new(),
            tool_items: HashMap::new(),
        }
    }

    fn base_extra(&self, raw_kind: &str) -> Map<String, Value> {
        let mut m = Map::new();
        m.insert("provider".into(), Value::from("codex"));
        m.insert("line_num".into(), Value::from(self.line_num));
        m.insert("raw_kind".into(), Value::from(raw_kind));
        if let Some(sid) = &self.session_id {
            m.insert("session_id".into(), Value::from(sid.as_str()));
        }
        m
    }

    fn emit_provider_extension(&mut self, kind: &str, payload: Value) {
        self.sink.on_semantic_event(SemanticEvent::ProviderExtension {
            provider: Provider::Codex,
            kind: kind.to_string(),
            payload,
        });
    }

    fn handle_thread_started(&mut self, meta: CodexThreadMeta, raw_kind: &str) {
        self.session_id = meta.resolved_id();
        super::trace_session_metadata(
            Provider::Codex,
            self.session_id.as_deref(),
            self.model.as_deref(),
        );
        self.sink.on_semantic_event(SemanticEvent::SessionStart {
            session_id: self.session_id.clone(),
            model: self.model.clone(),
            extra: Value::Object(self.base_extra(raw_kind)),
        });
    }

    fn handle_turn_started(&mut self, raw_kind: &str) {
        self.num_turns += 1;
        self.sink.on_semantic_event(SemanticEvent::TurnStart {
            extra: Value::Object(self.base_extra(raw_kind)),
        });
    }

    fn handle_turn_completed(&mut self, tc: CodexTurnCompleted, raw: Value, raw_kind: &str) {
        let provider_status = tc.provider_status().map(String::from);

        let step_usage = tc.usage.as_ref().map(|usage| {
            let input = usage.input_tokens;
            let output = usage.output_tokens;
            let cache_read = usage.cache_read();
            let total = match (input, output) {
                (Some(i), Some(o)) => Some(i + o),
                _ => usage.total_tokens,
            };
            NormalizedTokenUsage {
                input,
                output,
                total,
                cache_read,
            }
        });
        if let Some(step) = &step_usage {
            match &mut self.token_usage {
                Some(existing) => existing.merge(step),
                None => self.token_usage = Some(step.clone()),
            }
        }

        self.duration_ms = tc.duration_ms;
        self.cost_usd = tc.cost_usd;
        self.provider_status = provider_status.clone();
        self.raw_summary = Some(raw);
        super::trace_summary_update(
            Provider::Codex,
            self.provider_status.as_deref(),
            self.duration_ms,
            self.cost_usd,
        );

        self.sink.on_semantic_event(SemanticEvent::TurnComplete {
            provider_status,
            token_usage: step_usage,
            cost_usd: self.cost_usd,
            duration_ms: self.duration_ms,
            extra: Value::Object(self.base_extra(raw_kind)),
        });
    }

    fn handle_error(&mut self, env: CodexErrorEnvelope, raw_kind: &str) {
        self.is_error = true;
        self.error_kind = env.resolved_kind();
        self.error_message = env.resolved_message();

        let mut extra = self.base_extra(raw_kind);
        if let Some(kind) = &self.error_kind {
            extra.insert("error_kind".into(), Value::from(kind.as_str()));
        }
        self.sink.on_semantic_event(SemanticEvent::Error {
            message: self.error_message.clone().unwrap_or_default(),
            terminal: true,
            extra: Value::Object(extra),
        });
    }

    fn handle_agent_message_item(&mut self, msg: &CodexAgentMessage, raw_kind: &str) {
        // Accumulate text for the summary's assistant_text (used as a fallback
        // when --output-last-message is unavailable). Do not emit OutputText
        // to avoid double-rendering with the file-based text source.
        if let Some(text) = msg.collected_text() {
            self.assistant_text.push_str(&text);
        }
        // Preserve the event for the semantic log via ProviderExtension.
        let mut payload = self.base_extra(raw_kind);
        if let Some(id) = &msg.id {
            payload.insert("id".into(), Value::from(id.as_str()));
        }
        if let Some(text) = msg.collected_text() {
            payload.insert("text".into(), Value::from(text));
        }
        self.emit_provider_extension("item.agent_message", Value::Object(payload));
    }

    fn handle_reasoning_item(&mut self, r: &CodexReasoning, raw_kind: &str) {
        if let Some(text) = r.text.clone().filter(|s| !s.is_empty()) {
            let mut extra = self.base_extra(raw_kind);
            if let Some(summary) = &r.summary {
                extra.insert("summary".into(), summary.clone());
            }
            self.sink.on_semantic_event(SemanticEvent::Reasoning {
                text,
                extra: Value::Object(extra),
            });
        }
    }

    fn handle_file_change_item(&mut self, fc: &CodexFileChange, raw_kind: &str) {
        let mut extra = self.base_extra(raw_kind);
        if let Some(id) = &fc.id {
            extra.insert("id".into(), Value::from(id.as_str()));
        }
        self.sink.on_semantic_event(SemanticEvent::FileChange {
            path: fc.resolved_path().map(String::from),
            change_kind: fc.resolved_kind().map(String::from),
            extra: Value::Object(extra),
        });
    }

    fn handle_plan_update_item(&mut self, p: &CodexPlanUpdate, raw_kind: &str) {
        let mut extra = self.base_extra(raw_kind);
        if let Some(id) = &p.id {
            extra.insert("id".into(), Value::from(id.as_str()));
        }
        self.sink.on_semantic_event(SemanticEvent::PlanUpdate {
            message: p.resolved_message(),
            extra: Value::Object(extra),
        });
    }

    fn handle_permission_item(
        &mut self,
        perm: &CodexPermissionItem,
        kind: &str,
        raw_kind: &str,
    ) {
        if kind == "user_input" {
            self.user_input_prompts += 1;
        } else {
            self.permission_prompts += 1;
        }
        let mut extra = self.base_extra(raw_kind);
        extra.insert("permission_kind".into(), Value::from(kind));
        self.sink.on_semantic_event(SemanticEvent::PermissionRequest {
            kind: Some(kind.to_string()),
            tool_name: perm.name.clone(),
            extra: Value::Object(extra),
        });
    }

    fn tool_call_from_fields(&self, fields: &CodexToolItemFields, raw_kind: &str) -> SemanticEvent {
        let mut extra = self.base_extra(raw_kind);
        if let Some(id) = fields.resolved_tool_id() {
            extra.insert("tool_id".into(), Value::from(id));
        }
        if let Some(name) = fields.resolved_tool_name() {
            extra.insert("tool_name".into(), Value::from(name));
        }
        SemanticEvent::ToolCall {
            name: fields.resolved_tool_name().map(String::from),
            id: fields.resolved_tool_id().map(String::from),
            input: fields.resolved_input(),
            extra: Value::Object(extra),
        }
    }

    fn tool_result_from_fields(&self, fields: &CodexToolItemFields, raw_kind: &str) -> SemanticEvent {
        let mut extra = self.base_extra(raw_kind);
        if let Some(id) = fields.resolved_tool_id() {
            extra.insert("tool_id".into(), Value::from(id));
        }
        if let Some(name) = fields.resolved_tool_name() {
            extra.insert("tool_name".into(), Value::from(name));
        }
        if let Some(status) = &fields.status {
            extra.insert("status".into(), Value::from(status.as_str()));
        }
        if let Some(exit_code) = fields.exit_code {
            extra.insert("exit_code".into(), Value::from(exit_code));
        }
        SemanticEvent::ToolResult {
            name: fields.resolved_tool_name().map(String::from),
            id: fields.resolved_tool_id().map(String::from),
            status: fields.status.clone(),
            exit_code: fields.exit_code,
            output: fields.resolved_output(),
            extra: Value::Object(extra),
        }
    }

    fn handle_item_started(&mut self, env: CodexItemEnvelope, raw_kind: &str, raw: Value) {
        let Some(item) = env.item else {
            return;
        };
        match &item {
            CodexItem::PermissionRequest(perm) | CodexItem::ApprovalRequest(perm) => {
                self.handle_permission_item(perm, "permission", raw_kind);
                return;
            }
            CodexItem::UserInputRequest(perm) => {
                self.handle_permission_item(perm, "user_input", raw_kind);
                return;
            }
            CodexItem::Reasoning(r) => {
                self.handle_reasoning_item(r, raw_kind);
                return;
            }
            CodexItem::FileChange(fc) => {
                self.handle_file_change_item(fc, raw_kind);
                return;
            }
            CodexItem::PlanUpdate(p) | CodexItem::TodoList(p) => {
                self.handle_plan_update_item(p, raw_kind);
                return;
            }
            CodexItem::AgentMessage(_) | CodexItem::Unknown => {
                // Agent message arrives on completion; Unknown items fall through
                // to ProviderExtension below.
            }
            _ => {}
        }

        if item.is_tool_item() {
            self.tool_calls += 1;
            let fields = item
                .as_tool_fields()
                .expect("is_tool_item implies tool fields");
            super::trace_tool_event(
                Provider::Codex,
                self.tool_calls,
                fields.resolved_tool_name(),
            );
            let event = self.tool_call_from_fields(fields, raw_kind);
            if let Some(id) = fields.id.clone()
                && let Some(owned_fields) = item.into_tool_fields()
            {
                self.tool_items.insert(id, owned_fields);
            }
            self.sink.on_semantic_event(event);
            return;
        }

        if matches!(item, CodexItem::Unknown) {
            self.emit_provider_extension("item.started", raw);
        }
    }

    fn handle_item_completed(&mut self, env: CodexItemEnvelope, raw_kind: &str, raw: Value) {
        let Some(item) = env.item else {
            return;
        };

        match &item {
            CodexItem::AgentMessage(msg) => {
                self.handle_agent_message_item(msg, raw_kind);
                return;
            }
            CodexItem::Reasoning(r) => {
                self.handle_reasoning_item(r, raw_kind);
                return;
            }
            CodexItem::FileChange(fc) => {
                self.handle_file_change_item(fc, raw_kind);
                return;
            }
            CodexItem::PlanUpdate(p) | CodexItem::TodoList(p) => {
                self.handle_plan_update_item(p, raw_kind);
                return;
            }
            CodexItem::PermissionRequest(_)
            | CodexItem::ApprovalRequest(_)
            | CodexItem::UserInputRequest(_)
            | CodexItem::Unknown => {
                // Permission items only fire on started; Unknown falls through.
            }
            _ => {}
        }

        if item.is_tool_item() {
            let id = item.as_tool_fields().and_then(|f| f.id.clone());
            let merged_item = if let Some(id) = &id
                && let Some(started) = self.tool_items.remove(id)
            {
                let started_item = CodexItem::ToolUse(started);
                item.merge_started(started_item)
            } else {
                item
            };
            let fields = merged_item
                .as_tool_fields()
                .expect("is_tool_item implies tool fields");
            let event = self.tool_result_from_fields(fields, raw_kind);
            self.sink.on_semantic_event(event);
            return;
        }

        if matches!(item, CodexItem::Unknown) {
            self.emit_provider_extension("item.completed", raw);
        }
    }

    fn handle_item_updated(&mut self, env: CodexItemEnvelope, raw_kind: &str, raw: Value) {
        // `item.updated` streams partial progress while a long-running tool
        // executes. Surfacing them as `Info` events lets the heartbeat track
        // activity during Codex's otherwise silent reasoning/command stretches.
        let message = env
            .item
            .as_ref()
            .and_then(|item| match item {
                CodexItem::AgentMessage(m) => m.collected_text(),
                CodexItem::Reasoning(r) => r.text.clone(),
                _ => None,
            })
            .unwrap_or_else(|| "item.updated".to_string());
        let mut extra = self.base_extra(raw_kind);
        if let Some(item) = raw.get("item") {
            extra.insert("item".into(), item.clone());
        }
        self.sink.on_semantic_event(SemanticEvent::Info {
            message,
            extra: Value::Object(extra),
        });
    }

    fn emit_top_level_tool_use(&mut self, fields: CodexToolItemFields, raw_kind: &str) {
        self.tool_calls += 1;
        super::trace_tool_event(
            Provider::Codex,
            self.tool_calls,
            fields.resolved_tool_name(),
        );
        let event = self.tool_call_from_fields(&fields, raw_kind);
        self.sink.on_semantic_event(event);
    }

    fn emit_top_level_tool_result(&mut self, fields: CodexToolItemFields, raw_kind: &str) {
        let event = self.tool_result_from_fields(&fields, raw_kind);
        self.sink.on_semantic_event(event);
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

impl<S: SemanticEventSink> SemanticStreamParser for CodexSemanticStreamParser<S> {
    fn feed_line(&mut self, line: &str) -> Result<(), StreamParseError> {
        self.line_num += 1;
        let line = line.trim();
        if line.is_empty() {
            return Ok(());
        }

        let raw: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(e) => {
                super::trace_malformed_line(Provider::Codex, self.line_num, &e.to_string());
                self.emit_malformed_warning(&e.to_string());
                return Ok(());
            }
        };

        let raw_kind = raw
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        super::trace_parser_event(Provider::Codex, &raw_kind, self.line_num);

        match serde_json::from_value::<CodexEvent>(raw.clone()) {
            Ok(CodexEvent::ThreadCreated(meta) | CodexEvent::ThreadStarted(meta)) => {
                self.handle_thread_started(meta, &raw_kind);
            }
            Ok(CodexEvent::TurnStarted(_)) => {
                self.handle_turn_started(&raw_kind);
            }
            Ok(CodexEvent::TurnCompleted(tc)) => {
                self.handle_turn_completed(tc, raw, &raw_kind);
            }
            Ok(
                CodexEvent::Error(err)
                | CodexEvent::TurnError(err)
                | CodexEvent::TurnFailed(err)
                | CodexEvent::StreamError(err),
            ) => {
                self.handle_error(err, &raw_kind);
            }
            Ok(CodexEvent::ItemStarted(env)) => {
                self.handle_item_started(env, &raw_kind, raw);
            }
            Ok(CodexEvent::ItemCompleted(env)) => {
                self.handle_item_completed(env, &raw_kind, raw);
            }
            Ok(CodexEvent::ItemUpdated(env)) => {
                self.handle_item_updated(env, &raw_kind, raw);
            }
            Ok(CodexEvent::ItemToolUse(fields) | CodexEvent::ToolUse(fields)) => {
                self.emit_top_level_tool_use(fields, &raw_kind);
            }
            Ok(CodexEvent::ItemToolResult(fields) | CodexEvent::ToolResult(fields)) => {
                self.emit_top_level_tool_result(fields, &raw_kind);
            }
            Err(_) => {
                self.emit_provider_extension(&raw_kind, raw);
            }
        }

        Ok(())
    }

    fn finish(self: Box<Self>, exit_code: i32) -> StreamExecutionSummary {
        let mut summary = StreamExecutionSummary {
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
            permission_prompts: if self.permission_prompts > 0 {
                Some(self.permission_prompts)
            } else {
                None
            },
            user_input_prompts: if self.user_input_prompts > 0 {
                Some(self.user_input_prompts)
            } else {
                None
            },
            rate_limit: None,
            context_usage: None,
            badges: Vec::new(),
            raw_summary: self.raw_summary,
            stderr_text: None,
        };
        summary.badges = crate::stream::badges::derive_badges(&summary, Provider::Codex);
        summary
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;
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
        Box<CodexSemanticStreamParser<Recording>>,
    ) {
        let events = Arc::new(Mutex::new(Vec::new()));
        let sink = Recording {
            events: events.clone(),
        };
        (
            events,
            Box::new(CodexSemanticStreamParser::new(sink, Some("codex-mini".into()))),
        )
    }

    fn kinds(events: &[SemanticEvent]) -> Vec<&'static str> {
        events.iter().map(|e| e.kind_str()).collect()
    }

    #[test]
    fn thread_started_emits_session_start() {
        let (events, mut parser) = new_parser();
        parser
            .feed_line(r#"{"type":"thread.started","thread_id":"thrd-1"}"#)
            .unwrap();
        let collected = events.lock().unwrap().clone();
        assert!(matches!(
            collected[0],
            SemanticEvent::SessionStart { ref session_id, .. }
                if session_id.as_deref() == Some("thrd-1")
        ));
    }

    #[test]
    fn turn_lifecycle_emits_turn_start_and_complete() {
        let (events, mut parser) = new_parser();
        parser.feed_line(r#"{"type":"turn.started"}"#).unwrap();
        parser
            .feed_line(
                r#"{"type":"turn.completed","usage":{"input_tokens":100,"output_tokens":50},"duration_ms":1200,"status":"completed"}"#,
            )
            .unwrap();
        let ks = kinds(&events.lock().unwrap());
        assert_eq!(ks, vec!["turn_start", "turn_complete"]);

        let summary = parser.finish(0);
        assert_eq!(summary.num_turns, Some(1));
        assert_eq!(summary.duration_ms, Some(1200));
        assert_eq!(summary.provider_status.as_deref(), Some("completed"));
    }

    #[test]
    fn reasoning_item_emits_reasoning_event() {
        let (events, mut parser) = new_parser();
        parser
            .feed_line(
                r#"{"type":"item.completed","item":{"id":"r1","type":"reasoning","text":"long thought"}}"#,
            )
            .unwrap();
        let collected = events.lock().unwrap().clone();
        assert!(matches!(
            collected[0],
            SemanticEvent::Reasoning { ref text, .. } if text == "long thought"
        ));
    }

    #[test]
    fn item_updated_emits_info_with_progress() {
        let (events, mut parser) = new_parser();
        parser
            .feed_line(
                r#"{"type":"item.updated","item":{"id":"r1","type":"reasoning","text":"still thinking"}}"#,
            )
            .unwrap();
        let collected = events.lock().unwrap().clone();
        match &collected[0] {
            SemanticEvent::Info { message, .. } => assert_eq!(message, "still thinking"),
            other => panic!("expected Info, got {other:?}"),
        }
    }

    #[test]
    fn file_change_item_emits_file_change_event() {
        let (events, mut parser) = new_parser();
        parser
            .feed_line(
                r#"{"type":"item.completed","item":{"id":"f1","type":"file_change","path":"src/lib.rs","change_kind":"modified"}}"#,
            )
            .unwrap();
        let collected = events.lock().unwrap().clone();
        match &collected[0] {
            SemanticEvent::FileChange {
                path,
                change_kind,
                ..
            } => {
                assert_eq!(path.as_deref(), Some("src/lib.rs"));
                assert_eq!(change_kind.as_deref(), Some("modified"));
            }
            other => panic!("expected FileChange, got {other:?}"),
        }
    }

    #[test]
    fn plan_update_item_emits_plan_update_event() {
        let (events, mut parser) = new_parser();
        parser
            .feed_line(
                r#"{"type":"item.completed","item":{"id":"p1","type":"plan_update","message":"Step 2 of 5"}}"#,
            )
            .unwrap();
        let collected = events.lock().unwrap().clone();
        match &collected[0] {
            SemanticEvent::PlanUpdate { message, .. } => {
                assert_eq!(message.as_deref(), Some("Step 2 of 5"));
            }
            other => panic!("expected PlanUpdate, got {other:?}"),
        }
    }

    #[test]
    fn todo_list_routed_as_plan_update() {
        let (events, mut parser) = new_parser();
        parser
            .feed_line(
                r#"{"type":"item.completed","item":{"id":"t1","type":"todo_list","title":"next steps"}}"#,
            )
            .unwrap();
        let collected = events.lock().unwrap().clone();
        assert!(matches!(collected[0], SemanticEvent::PlanUpdate { .. }));
    }

    #[test]
    fn command_execution_status_and_exit_code_preserved() {
        let (events, mut parser) = new_parser();
        // Started: records input
        parser
            .feed_line(
                r#"{"type":"item.started","item":{"id":"cmd1","type":"command_exec","tool_name":"bash","input":{"command":"false"}}}"#,
            )
            .unwrap();
        // Completed: brings status + exit_code
        parser
            .feed_line(
                r#"{"type":"item.completed","item":{"id":"cmd1","type":"command_exec","status":"failure","exit_code":1,"output":"boom"}}"#,
            )
            .unwrap();

        let collected = events.lock().unwrap().clone();
        let ks = kinds(&collected);
        assert_eq!(ks, vec!["tool_call", "tool_result"]);
        match &collected[1] {
            SemanticEvent::ToolResult {
                status,
                exit_code,
                output,
                name,
                ..
            } => {
                assert_eq!(status.as_deref(), Some("failure"));
                assert_eq!(*exit_code, Some(1));
                assert_eq!(output.as_ref().and_then(Value::as_str), Some("boom"));
                assert_eq!(name.as_deref(), Some("bash"));
            }
            other => panic!("expected ToolResult, got {other:?}"),
        }
    }

    #[test]
    fn permission_request_emits_permission_request_event() {
        let (events, mut parser) = new_parser();
        parser
            .feed_line(
                r#"{"type":"item.started","item":{"id":"perm-1","type":"permission_request","name":"bash"}}"#,
            )
            .unwrap();
        let collected = events.lock().unwrap().clone();
        assert!(matches!(
            collected[0],
            SemanticEvent::PermissionRequest { ref tool_name, .. }
                if tool_name.as_deref() == Some("bash")
        ));
        let summary = parser.finish(0);
        assert_eq!(summary.permission_prompts, Some(1));
    }

    #[test]
    fn user_input_request_increments_separate_counter() {
        let (events, mut parser) = new_parser();
        parser
            .feed_line(
                r#"{"type":"item.started","item":{"id":"inp-1","type":"user_input_request","name":"clarify"}}"#,
            )
            .unwrap();
        assert_eq!(kinds(&events.lock().unwrap()), vec!["permission_request"]);
        let summary = parser.finish(0);
        assert_eq!(summary.user_input_prompts, Some(1));
        assert_eq!(summary.permission_prompts, None);
    }

    #[test]
    fn error_event_marks_summary_and_emits_error() {
        let (events, mut parser) = new_parser();
        parser
            .feed_line(
                r#"{"type":"error","error_type":"rate_limit","error_message":"Too many requests"}"#,
            )
            .unwrap();
        let collected = events.lock().unwrap().clone();
        assert!(matches!(collected[0], SemanticEvent::Error { terminal: true, .. }));
        let summary = parser.finish(1);
        assert!(summary.is_error);
        assert_eq!(summary.error_kind.as_deref(), Some("rate_limit"));
    }

    #[test]
    fn agent_message_accumulates_text_without_emitting_output() {
        let (events, mut parser) = new_parser();
        parser
            .feed_line(
                r#"{"type":"item.completed","item":{"id":"a1","type":"agent_message","text":"hello from stream"}}"#,
            )
            .unwrap();
        let ks = kinds(&events.lock().unwrap());
        // No OutputText; preserved as ProviderExtension for semantic log only.
        assert!(!ks.contains(&"output_text"));
        assert!(ks.contains(&"provider_extension"));
        let summary = parser.finish(0);
        assert_eq!(summary.assistant_text, "hello from stream");
    }

    #[test]
    fn unknown_top_level_event_becomes_provider_extension() {
        let (events, mut parser) = new_parser();
        parser
            .feed_line(r#"{"type":"future.event.kind","payload":{"k":1}}"#)
            .unwrap();
        let collected = events.lock().unwrap().clone();
        match &collected[0] {
            SemanticEvent::ProviderExtension {
                provider,
                kind,
                payload,
            } => {
                assert_eq!(*provider, Provider::Codex);
                assert_eq!(kind, "future.event.kind");
                assert_eq!(payload.get("payload"), Some(&json!({"k": 1})));
            }
            other => panic!("expected ProviderExtension, got {other:?}"),
        }
    }

    #[test]
    fn malformed_json_emits_warning_and_continues() {
        let (events, mut parser) = new_parser();
        let result = parser.feed_line("not json");
        assert!(result.is_ok());
        parser
            .feed_line(r#"{"type":"turn.started"}"#)
            .unwrap();
        let ks = kinds(&events.lock().unwrap());
        assert_eq!(ks, vec!["warning", "turn_start"]);
    }

    #[test]
    fn top_level_tool_use_and_result() {
        let (events, mut parser) = new_parser();
        parser
            .feed_line(r#"{"type":"item.tool_use","name":"bash","input":{"cmd":"ls"}}"#)
            .unwrap();
        parser
            .feed_line(
                r#"{"type":"item.tool_result","name":"bash","status":"success","exit_code":0}"#,
            )
            .unwrap();
        let collected = events.lock().unwrap().clone();
        assert_eq!(kinds(&collected), vec!["tool_call", "tool_result"]);
        match &collected[1] {
            SemanticEvent::ToolResult { status, exit_code, .. } => {
                assert_eq!(status.as_deref(), Some("success"));
                assert_eq!(*exit_code, Some(0));
            }
            other => panic!("expected ToolResult, got {other:?}"),
        }
    }

    #[test]
    fn empty_lines_emit_nothing() {
        let (events, mut parser) = new_parser();
        parser.feed_line("").unwrap();
        parser.feed_line("   ").unwrap();
        assert!(events.lock().unwrap().is_empty());
    }

    #[test]
    fn round_trip_fidelity_mixed_fixture() {
        let (events, mut parser) = new_parser();
        for line in [
            r#"{"type":"thread.started","thread_id":"t1"}"#,
            r#"{"type":"turn.started"}"#,
            r#"{"type":"item.started","item":{"id":"cmd1","type":"command_exec","tool_name":"bash","input":{"command":"ls"}}}"#,
            r#"{"type":"item.updated","item":{"id":"r1","type":"reasoning","text":"thinking"}}"#,
            r#"{"type":"item.completed","item":{"id":"cmd1","type":"command_exec","status":"success","exit_code":0,"output":"file.txt"}}"#,
            r#"{"type":"item.completed","item":{"id":"f1","type":"file_change","path":"a.rs","change_kind":"modified"}}"#,
            r#"{"type":"item.completed","item":{"id":"p1","type":"plan_update","message":"next step"}}"#,
            r#"{"type":"turn.completed","usage":{"input_tokens":10,"output_tokens":5},"duration_ms":42,"status":"completed"}"#,
            r#"{"type":"future.unknown","k":1}"#,
        ] {
            parser.feed_line(line).unwrap();
        }
        for event in events.lock().unwrap().iter() {
            let v = serde_json::to_value(event).unwrap();
            let decoded: SemanticEvent = serde_json::from_value(v.clone()).unwrap();
            let v2 = serde_json::to_value(&decoded).unwrap();
            assert_eq!(v, v2, "round-trip lost fidelity for {}", event.kind_str());
        }
    }

    #[test]
    fn badges_derived_on_rate_limit_error() {
        let (_, mut parser) = new_parser();
        parser
            .feed_line(
                r#"{"type":"error","error_type":"rate_limit","error_message":"Too many requests"}"#,
            )
            .unwrap();
        let summary = parser.finish(1);
        assert_eq!(summary.badges.len(), 1);
        assert_eq!(
            summary.badges[0].category,
            crate::stream::badges::BadgeCategory::RateLimit
        );
    }
}
