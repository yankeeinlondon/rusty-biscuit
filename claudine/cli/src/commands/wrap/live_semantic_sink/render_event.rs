//! STDERR status rendering for individual [`SemanticEvent`]s.
//!
//! [`LiveSemanticSink::render_event`] is the central per-event dispatcher that
//! maps each [`SemanticEvent`] variant to a status / block render through the
//! section-aware emitters in [`super::sections`], [`super::tool_calls`], and
//! [`super::errors`].

use super::LiveSemanticSink;
use super::Section;
use super::tool_calls;
use biscuit_terminal::components::status::StatusState;
use claudine::provider::Provider;
use claudine::stream::semantic::SemanticEvent;
use claudine::stream::stderr::Verbosity;
use claudine::stream::tool_display::{ToolCallDisplay, ToolStatus};
use serde_json::Value;

impl LiveSemanticSink {
    pub(crate) fn render_event(&mut self, event: &SemanticEvent) {
        if !self.should_render() {
            return;
        }
        // Every event variant handled in this match renders into the
        // ToolUseAndEvents section. SessionAndModel is handled separately
        // via `emit_agent_session_id`; Thinking is handled via the
        // `Reasoning` branch of [`SemanticEventSink::on_semantic_event`];
        // FinalStdout is entered via `enter_final_stdout` in the
        // `OutputText` branch of that same method; TrailerMetadata is
        // emitted post-stream through [`Self::section_stream`] by callers
        // in `wrap/mod.rs` and `wrap/composition.rs`.
        let section = Section::ToolUseAndEvents;

        // Redundancy suppression: Claude's `task_progress` events arrive
        // immediately before the matching tool call ("Reading <path>" →
        // `→ Read(<path>)`). Hold the most recent progress message in a
        // one-slot buffer and consult it on the next event so the pair
        // collapses to the canonical tool line.
        if super::is_claude_task_progress(self.provider, event) {
            if let SemanticEvent::Info { message, .. } = event {
                self.pending_task_progress = Some(message.clone());
            }
            return;
        }
        self.resolve_pending_task_progress(section, event);
        match event {
            SemanticEvent::ToolCall { .. } => {
                if let Some(display) = ToolCallDisplay::from_call(event) {
                    let desc = self.render_tool_display(display);
                    self.render_status_prose(section, StatusState::ToolUse, desc);
                }
            }
            SemanticEvent::ToolResult { .. } => {
                if let Some(display) = ToolCallDisplay::from_result(event) {
                    if display.is_file_tool() && display.status == Some(ToolStatus::Error) {
                        self.render_file_tool_error(section, &display);
                    } else {
                        let body = tool_calls::tool_result_body(event);
                        let mut header_display = display.clone();
                        if self.provider == Provider::Codex
                            && body.is_some()
                            && header_display.status == Some(ToolStatus::Success)
                            && !header_display.is_file_tool()
                        {
                            header_display.summary = None;
                        }
                        let desc = self.render_tool_display(header_display);
                        self.render_status_prose(section, StatusState::ToolUse, desc);
                        if self.provider == Provider::Codex
                            && self.verbosity == Verbosity::Normal
                            && let Some(body) = body
                        {
                            self.render_tool_result_body(section, &body);
                        }
                    }
                }
            }
            SemanticEvent::SubagentStart { name, .. } => {
                self.render_status(
                    section,
                    StatusState::Subagent,
                    Self::subagent_description('\u{2192}', name),
                );
            }
            SemanticEvent::SubagentStop { name, .. } => {
                self.render_status(
                    section,
                    StatusState::Subagent,
                    Self::subagent_description('\u{2190}', name),
                );
            }
            SemanticEvent::FileChange {
                path, change_kind, ..
            } => {
                // Suppress rendering when the event carries neither a path
                // nor a classification. Codex can emit provisional
                // `file_change` items with empty bodies that would otherwise
                // appear as a bare "change" line with no context.
                let path = path.as_deref().unwrap_or("");
                let kind = change_kind.as_deref();
                if path.is_empty() && kind.is_none() {
                    return;
                }
                let kind_label = kind.unwrap_or("change");
                let line = if path.is_empty() {
                    kind_label.to_string()
                } else {
                    format!("{kind_label} {path}")
                };
                self.render_status(section, StatusState::Info, line);
            }
            SemanticEvent::PlanUpdate { message, .. } => {
                if let Some(msg) = message {
                    self.render_status(section, StatusState::Info, msg.clone());
                }
            }
            SemanticEvent::Info { message, extra } => {
                // Suppress OpenCode's `step_start` / `step_finish` phase
                // markers from the rendered stderr surface. They carry no
                // user-visible meaning (internal phase boundaries between
                // tool batches) and produce visual noise around tool lines.
                // The events still flow through dispatch, JSONL logging, and
                // the LiveMetrics heartbeat — only the human-visible Status
                // line is suppressed. Suppression is gated on
                // `extra["step_phase"]` so unrelated Info events from
                // OpenCode (or any other provider) are unaffected.
                //
                // Invisible-event invariant: suppressed events MUST return
                // before any call to `emit_section_line` so the
                // `SectionTracker` state is not updated. This prevents the
                // tracker from detecting a phantom section change that would
                // inject a redundant separator before the next visible
                // event.
                if self.provider == Provider::OpenCode && extra.get("step_phase").is_some() {
                    return;
                }
                self.render_status(section, StatusState::Info, message.clone());
            }
            SemanticEvent::Warning { message, extra } => {
                // Suppress noisy malformed-line warnings on stderr — these
                // are common when providers mix non-JSON output into the
                // stream (Gemini hook logs, stack traces, etc.) and the
                // semantic parser surfaces them as Warning events per the
                // Phase 2 policy. Still dispatched and logged.
                //
                // Also suppress the legacy generic Claude rate-limit Warning
                // when the session metadata shows a subscription auth source.
                // Explicit metadata text such as "approaching limit" must
                // still render so users can see the next reset window.
                if message.starts_with("Malformed JSON on line ")
                    || super::is_suppressed_claude_rate_limit(
                        self.provider,
                        message,
                        extra,
                        self.claude_api_key_source.as_deref(),
                    )
                {
                    return;
                }
                // Codex stderr bridge emits Warnings enriched with a
                // `tracing_target` extra. Those want a two-line rendering
                // (Status header + orange BlockQuote) so operators can read
                // the diagnostic without the raw `TIMESTAMP LEVEL ...`
                // formatting leaking through.
                if let Some(target) = extra.get("tracing_target").and_then(Value::as_str) {
                    self.render_tracing_diagnostic(section, target, message);
                } else {
                    self.render_status(section, StatusState::Warning, message.clone());
                }
            }
            SemanticEvent::Error { message, kind, .. } => {
                self.render_error_block(section, *kind, message);
            }
            SemanticEvent::ProviderExtension {
                provider,
                kind,
                payload,
            } => {
                if super::is_silent_extension_kind(*provider, kind) {
                    // Suppress stderr rendering; the event still flows
                    // through dispatch and the JSONL log.
                    return;
                }
                self.render_status(
                    section,
                    StatusState::Info,
                    Self::provider_extension_description(*provider, kind, payload),
                );
            }
            // OutputText / Reasoning / SessionStart / TurnStart / TurnComplete /
            // PermissionRequest do not render through Status — Output/Reasoning
            // flow through their own renderers in the Phase 3.3 wiring step;
            // the others are envelope-only.
            SemanticEvent::SessionStart { .. }
            | SemanticEvent::TurnStart { .. }
            | SemanticEvent::TurnComplete { .. }
            | SemanticEvent::OutputText { .. }
            | SemanticEvent::Reasoning { .. }
            | SemanticEvent::PermissionRequest { .. } => {}
        }
    }

    fn subagent_description(arrow: char, name: &Option<String>) -> String {
        let name_part = name.as_deref().unwrap_or("(subagent)");
        format!("{arrow} {name_part}")
    }

    fn provider_extension_description(provider: Provider, kind: &str, payload: &Value) -> String {
        let summary = super::summarize_provider_payload(payload);
        match summary {
            Some(s) => format!("{}/{kind} \u{00b7} {s}", super::provider_short(provider)),
            None => format!("{}/{kind}", super::provider_short(provider)),
        }
    }
}
