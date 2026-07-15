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

use std::borrow::Cow;
use std::collections::HashMap;

use serde_json::{Map, Value};

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

/// An unsupported runtime identity was supplied to the shared OpenCode parser.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[error("OpenCode stream parser supports only OpenCode and Kilo identities, got {provider:?}")]
pub struct InvalidOpenCodeParserProvider {
    /// The identity rejected at the parser construction boundary.
    pub provider: Provider,
}

pub struct OpenCodeSemanticStreamParser<S: SemanticEventSink> {
    sink: S,
    /// Runtime identity: `OpenCode` or its `Kilo` fork. Kilo reuses this exact
    /// wire parser but keeps its own error vocabulary and stamps its own
    /// provider on every emitted event, so parser reuse never collapses the
    /// two identities.
    provider: Provider,
    error_vocabulary: &'static super::common::ErrorKeywords,
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
    pub fn new(
        sink: S,
        model: Option<String>,
        provider: Provider,
    ) -> Result<Self, InvalidOpenCodeParserProvider> {
        Self::new_with_vocabulary_resolver(
            sink,
            model,
            provider,
            super::vocabulary::error_keywords,
        )
    }

    fn new_with_vocabulary_resolver(
        sink: S,
        model: Option<String>,
        provider: Provider,
        vocabulary_for: fn(Provider) -> &'static super::common::ErrorKeywords,
    ) -> Result<Self, InvalidOpenCodeParserProvider> {
        let provider = opencode_parser_identity(provider)?;
        Ok(Self {
            sink,
            provider,
            error_vocabulary: vocabulary_for(provider),
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
        })
    }

    fn base_extra(&self, raw_kind: &str) -> Map<String, Value> {
        super::common::base_extra(self.provider, self.line_num, raw_kind)
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
            self.provider,
            self.session_id.as_deref(),
            self.model.as_deref(),
        );
        self.emit_session_start(raw_kind);
    }

    fn handle_step_start(&mut self, step: OpenCodeStepStart, raw_kind: &str) {
        if self.session_id.is_none() {
            self.session_id = step.resolved_session_id();
            super::trace_session_metadata(
                self.provider,
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
            self.provider,
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
            self.provider,
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
        // MiniMax-M2/M3 (and similar reasoning models) wrap their chain of
        // thought in literal `<think>…</think>` sentinels. OpenCode routes the
        // enclosed prose to `reasoning` events, but the boundary delimiter can
        // leak out as a lone `text` delta. Drop those orphan sentinel lines so
        // they neither render in the main output nor pollute `assistant_text`.
        let text = strip_orphan_think_delimiters(&text);
        if text.is_empty() {
            return;
        }
        self.assistant_text.push_str(&text);
        self.sink.on_semantic_event(SemanticEvent::OutputText {
            text: text.into_owned(),
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
        let semantic_kind = classify_error(
            self.error_vocabulary,
            self.error_kind.as_deref(),
            Some(&message),
        );
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
            self.provider,
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
            self.provider,
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
        super::common::emit_provider_extension(&mut self.sink, self.provider, kind, payload);
    }

    fn emit_malformed_warning(&mut self, err: &str) {
        super::common::emit_malformed_warning(&mut self.sink, self.provider, self.line_num, err);
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
                super::trace_parser_event(self.provider, &raw_kind, self.line_num);
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
                            self.provider,
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
                super::trace_parser_event(self.provider, &raw_kind, self.line_num);
                self.emit_provider_extension(&raw_kind, Value::Object(raw));
            }
        }
        Ok(())
    }

    fn finish(self: Box<Self>, exit_code: i32) -> StreamExecutionSummary {
        super::trace_parser_finish(
            self.provider,
            exit_code,
            self.tool_calls,
            self.num_turns,
            self.provider_status.as_deref(),
        );
        let has_usage = self.token_usage.input.is_some() || self.token_usage.output.is_some();
        super::common::finish_summary(
            self.provider,
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
                token_usage: has_usage.then_some(self.token_usage),
                cost_usd: (self.cost_usd > 0.0).then_some(self.cost_usd),
                tool_calls: (self.tool_calls > 0).then_some(self.tool_calls),
                ..Default::default()
            },
        )
    }
}

/// Remove standalone reasoning-delimiter sentinels (`<think>` / `</think>`)
/// from an OpenCode `text` payload.
///
/// Reasoning models such as MiniMax-M2/M3 emit their chain of thought wrapped
/// in literal `<think>…</think>` tokens. OpenCode forwards the enclosed prose as
/// `reasoning` events, but the boundary delimiter itself can arrive as its own
/// `text` delta — which would otherwise render as a stray `</think>` line in
/// assistant output. Only a sentinel that occupies an entire line (after
/// trimming) is dropped, so prose that legitimately mentions the tag inline
/// (for example, documentation about `<think>`) is preserved verbatim.
fn strip_orphan_think_delimiters(text: &str) -> Cow<'_, str> {
    // Cheap reject: no `think>` substring means nothing to strip.
    if !text.contains("think>") {
        return Cow::Borrowed(text);
    }
    let mut out = String::with_capacity(text.len());
    let mut stripped_any = false;
    for segment in text.split_inclusive('\n') {
        let body = segment.strip_suffix('\n').unwrap_or(segment);
        if matches!(body.trim(), "<think>" | "</think>") {
            stripped_any = true;
            continue;
        }
        out.push_str(segment);
    }
    if !stripped_any {
        return Cow::Borrowed(text);
    }
    // When the delta was nothing but the delimiter (optionally wrapped in
    // whitespace), the leftover whitespace carries no content — collapse it so
    // no empty OutputText is emitted. Real content keeps its surrounding
    // blank lines intact.
    if out.trim().is_empty() {
        return Cow::Owned(String::new());
    }
    Cow::Owned(out)
}

/// Validate that the shared parser's runtime identity belongs to its wire
/// protocol family.
fn opencode_parser_identity(
    provider: Provider,
) -> Result<Provider, InvalidOpenCodeParserProvider> {
    match provider {
        Provider::OpenCode | Provider::Kilo => Ok(provider),
        provider => Err(InvalidOpenCodeParserProvider { provider }),
    }
}

/// Map an OpenCode error envelope onto a typed [`SemanticErrorKind`] using the
/// runtime provider's generated vocabulary (`OpenCode` or `Kilo`).
fn classify_error(
    vocabulary: &super::common::ErrorKeywords,
    error_kind: Option<&str>,
    message: Option<&str>,
) -> SemanticErrorKind {
    super::common::classify_error_by_keywords(
        vocabulary,
        None,
        error_kind,
        message,
    )
}

#[cfg(test)]
mod tests;
