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
//! to stdout). It also does NOT leak the event as a `ProviderExtension` —
//! the text is preserved in the summary's `assistant_text` field and any
//! consumer that needs the raw event can inspect the captured JSONL log
//! directly.

use std::collections::HashMap;

use serde_json::{Map, Value};

use super::parser::{SemanticStreamParser, StreamParseError};
use super::protocol::codex::{
    CodexAgentMessage, CodexErrorEnvelope, CodexEvent, CodexFileChange, CodexItem,
    CodexItemEnvelope, CodexPermissionItem, CodexPlanUpdate, CodexReasoning, CodexThreadMeta,
    CodexToolItemFields, CodexTurnCompleted,
};
use super::semantic::{SemanticErrorKind, SemanticEvent, SemanticEventSink};
use super::summary::StreamExecutionSummary;
use super::token_usage::NormalizedTokenUsage;
use crate::provider_id::Provider;
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
        let mut m = super::common::base_extra(Provider::Codex, self.line_num, raw_kind);
        if let Some(sid) = &self.session_id {
            m.insert("session_id".into(), Value::from(sid.as_str()));
        }
        m
    }

    fn emit_provider_extension(&mut self, kind: &str, payload: Value) {
        super::common::emit_provider_extension(&mut self.sink, Provider::Codex, kind, payload);
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

    fn handle_turn_completed(&mut self, tc: CodexTurnCompleted, raw_kind: &str) {
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
        // Reconstruct the raw payload from the typed struct without a second parse.
        let raw = serde_json::to_value(&tc).expect("CodexTurnCompleted serializes");
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
        let new_kind = env.resolved_kind();
        let new_message = env.resolved_message();

        // Codex commonly emits more than one terminal error for a single
        // failure — typically `turn.failed` plus a top-level `error` carrying
        // the same kind/message (rate-limit hits and auth failures both
        // surface this way). Without dedup the live stderr surface renders
        // two identical "Agent Error" blocks. Suppress sink emission when
        // the new error is byte-for-byte identical to the previously emitted
        // one, while still keeping summary state fresh.
        let is_duplicate =
            self.is_error && self.error_kind == new_kind && self.error_message == new_message;

        self.is_error = true;
        self.error_kind = new_kind;
        self.error_message = new_message;

        if is_duplicate {
            return;
        }

        let mut extra = self.base_extra(raw_kind);
        if let Some(kind) = &self.error_kind {
            extra.insert("error_kind".into(), Value::from(kind.as_str()));
        }
        let semantic_kind =
            classify_error(self.error_kind.as_deref(), self.error_message.as_deref());
        self.sink.on_semantic_event(SemanticEvent::Error {
            message: self.error_message.clone().unwrap_or_default(),
            terminal: true,
            kind: semantic_kind,
            extra: Value::Object(extra),
        });
    }

    fn handle_agent_message_item(&mut self, msg: &CodexAgentMessage, raw_kind: &str) {
        // Accumulate text for the summary's assistant_text (used as a fallback
        // when --output-last-message is unavailable). Do not emit OutputText
        // to avoid double-rendering with the file-based text source, and do
        // not leak to ProviderExtension — the event is preserved in the raw
        // JSONL log, and ProviderExtension should be reserved for events the
        // semantic layer genuinely does not understand.
        //
        // Also emit a Reasoning event so the live stderr surface renders the
        // intermediate prose Codex produces between tool calls as a grey
        // BlockQuote. Without this, non-interactive Codex sessions appear to
        // skip straight from one tool to the next with no visible narrative.
        // The authoritative final assistant text still arrives via
        // `--output-last-message`, so any duplication between the last
        // BlockQuote and the final stdout is intentional and rare.
        if let Some(text) = msg.collected_text() {
            self.assistant_text.push_str(&text);
            let mut extra = self.base_extra(raw_kind);
            extra.insert("origin".into(), Value::from("agent_message"));
            self.sink.on_semantic_event(SemanticEvent::Reasoning {
                text,
                extra: Value::Object(extra),
            });
        }
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
        // Codex emits file_change paths inside a `changes[]` array on
        // `item.completed`; fan out one FileChange per entry so the stderr
        // surface shows one line per touched path. Empty payloads (e.g.
        // bare `item.started` with no path or kind) are dropped rather than
        // rendered as a context-free "change" line.
        let entries = fc.resolved_entries();
        if entries.is_empty() {
            return;
        }
        for (path, change_kind) in entries {
            let mut extra = self.base_extra(raw_kind);
            if let Some(id) = &fc.id {
                extra.insert("id".into(), Value::from(id.as_str()));
            }
            if let Some(status) = &fc.status {
                extra.insert("status".into(), Value::from(status.as_str()));
            }
            self.sink.on_semantic_event(SemanticEvent::FileChange {
                path,
                change_kind,
                extra: Value::Object(extra),
            });
        }
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

    fn handle_permission_item(&mut self, perm: &CodexPermissionItem, kind: &str, raw_kind: &str) {
        if kind == "user_input" {
            self.user_input_prompts += 1;
        } else {
            self.permission_prompts += 1;
        }
        let mut extra = self.base_extra(raw_kind);
        extra.insert("permission_kind".into(), Value::from(kind));
        self.sink
            .on_semantic_event(SemanticEvent::PermissionRequest {
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
        if let Some(status) = &fields.status {
            extra.insert("status".into(), Value::from(status.as_str()));
        }
        SemanticEvent::ToolCall {
            name: fields.resolved_tool_name().map(String::from),
            id: fields.resolved_tool_id().map(String::from),
            input: fields.resolved_input(),
            extra: Value::Object(extra),
        }
    }

    fn tool_result_from_fields(
        &self,
        fields: &CodexToolItemFields,
        raw_kind: &str,
    ) -> SemanticEvent {
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
        if let Some(input) = fields.resolved_input() {
            extra.insert("input".into(), input);
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

    fn handle_item_started(&mut self, env: CodexItemEnvelope, raw_kind: &str) {
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
                // Agent message arrives on completion; Unknown items are handled
                // by the caller when needed.
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
        }
    }

    fn handle_item_completed(&mut self, env: CodexItemEnvelope, raw_kind: &str) {
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
                // Permission items only fire on started; Unknown is handled by
                // the caller when needed.
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
        }
    }

    fn handle_item_updated(&mut self, env: CodexItemEnvelope, raw_kind: &str) {
        // `item.updated` carries partial-progress snapshots for long-running
        // items. Route by inner item type:
        //
        // - `todo_list` / `plan_update` → `PlanUpdate` (the update really
        //   is a new plan state).
        // - `agent_message` / `reasoning` → `Reasoning` prose, so callers
        //   render it via the thinking block-quote rather than a one-line
        //   `Info` glyph.
        // - Everything else (tool-progress pulses, unknown types) is
        //   dropped on stderr; the raw line is still captured in the JSONL
        //   log for post-hoc inspection.
        let Some(item) = env.item else {
            return;
        };
        match item {
            CodexItem::TodoList(p) | CodexItem::PlanUpdate(p) => {
                self.handle_plan_update_item(&p, raw_kind);
            }
            CodexItem::AgentMessage(m) => {
                if let Some(text) = m.collected_text() {
                    let mut extra = self.base_extra(raw_kind);
                    extra.insert("origin".into(), Value::from("item.updated"));
                    self.sink.on_semantic_event(SemanticEvent::Reasoning {
                        text,
                        extra: Value::Object(extra),
                    });
                }
            }
            CodexItem::Reasoning(r) => {
                self.handle_reasoning_item(&r, raw_kind);
            }
            _ => {}
        }
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
        super::common::emit_malformed_warning(&mut self.sink, Provider::Codex, self.line_num, err);
    }
}

impl<S: SemanticEventSink> SemanticStreamParser for CodexSemanticStreamParser<S> {
    fn feed_line(&mut self, line: &str) -> Result<(), StreamParseError> {
        self.line_num += 1;
        let line = line.trim();
        if line.is_empty() {
            return Ok(());
        }

        // Try typed deserialization first to avoid `serde_json::Value` DOM
        // allocation on the hot path. Fall back to `Value` only for unknown
        // event types that must be preserved as `ProviderExtension`, or for
        // `turn.completed` which needs the raw payload for `raw_summary`.
        match serde_json::from_str::<CodexEvent>(line) {
            Ok(event) => {
                let raw_kind = event.type_str().to_string();
                super::trace_parser_event(Provider::Codex, &raw_kind, self.line_num);
                match event {
                    CodexEvent::ThreadCreated(meta) | CodexEvent::ThreadStarted(meta) => {
                        self.handle_thread_started(meta, &raw_kind);
                    }
                    CodexEvent::TurnStarted(_) => {
                        self.handle_turn_started(&raw_kind);
                    }
                    CodexEvent::TurnCompleted(tc) => {
                        self.handle_turn_completed(tc, &raw_kind);
                    }
                    CodexEvent::Error(err)
                    | CodexEvent::TurnError(err)
                    | CodexEvent::TurnFailed(err)
                    | CodexEvent::StreamError(err) => {
                        self.handle_error(err, &raw_kind);
                    }
                    CodexEvent::ItemStarted(env) => {
                        self.handle_item_started(env, &raw_kind);
                    }
                    CodexEvent::ItemCompleted(env) => {
                        self.handle_item_completed(env, &raw_kind);
                    }
                    CodexEvent::ItemUpdated(env) => {
                        self.handle_item_updated(env, &raw_kind);
                    }
                    CodexEvent::ItemToolUse(fields) | CodexEvent::ToolUse(fields) => {
                        self.emit_top_level_tool_use(fields, &raw_kind);
                    }
                    CodexEvent::ItemToolResult(fields) | CodexEvent::ToolResult(fields) => {
                        self.emit_top_level_tool_result(fields, &raw_kind);
                    }
                }
            }
            Err(_) => {
                let raw: Map<String, Value> = match serde_json::from_str(line) {
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
                self.emit_provider_extension(&raw_kind, Value::Object(raw));
            }
        }

        Ok(())
    }

    fn finish(self: Box<Self>, exit_code: i32) -> StreamExecutionSummary {
        super::common::finish_summary(
            Provider::Codex,
            StreamExecutionSummary {
                session_id: self.session_id,
                model: self.model,
                assistant_text: self.assistant_text,
                provider_status: self.provider_status,
                exit_code,
                is_error: self.is_error,
                error_kind: self.error_kind,
                error_message: self.error_message,
                duration_ms: self.duration_ms,
                num_turns: (self.num_turns > 0).then_some(self.num_turns),
                token_usage: self.token_usage,
                cost_usd: self.cost_usd,
                tool_calls: (self.tool_calls > 0).then_some(self.tool_calls),
                permission_prompts: (self.permission_prompts > 0).then_some(self.permission_prompts),
                user_input_prompts: (self.user_input_prompts > 0).then_some(self.user_input_prompts),
                raw_summary: self.raw_summary,
                ..Default::default()
            },
        )
    }
}

/// Map a Codex error envelope onto a typed [`SemanticErrorKind`].
///
/// Codex surfaces errors as either a typed envelope kind (e.g.
/// `rate_limit`, `auth`) or a free-form message. This helper inspects both
/// so the live error renderer and the end-of-run report can pick a
/// consistent label and color.
fn classify_error(error_kind: Option<&str>, message: Option<&str>) -> SemanticErrorKind {
    super::common::classify_error_by_keywords(
        super::vocabulary::error_keywords(Provider::Codex),
        None,
        error_kind,
        message,
    )
}

#[cfg(test)]
mod tests;
