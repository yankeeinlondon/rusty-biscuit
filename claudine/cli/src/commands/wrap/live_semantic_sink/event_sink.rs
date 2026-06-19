//! [`SemanticEventSink`] implementation — the per-event fan-out that drives
//! metrics, summary rollups, watchdog state, stdout/reasoning forwarding,
//! status rendering, dispatch, and JSONL logging for [`LiveSemanticSink`].

use super::LiveSemanticSink;
use super::Section;
use claudine::events::AgenticEvent;
use claudine::stream::semantic::{SemanticEvent, SemanticEventSink};
use claudine::stream::thinking::render_thinking_block;
use serde_json::Value;

impl SemanticEventSink for LiveSemanticSink {
    fn on_semantic_event(&mut self, event: SemanticEvent) {
        // 1. LiveMetrics observation for the heartbeat.
        if let Ok(mut state) = self.live_metrics.lock() {
            state.observe_event(&event, std::time::Instant::now());
        }

        // 2. Update cached session id / model from envelope events.
        if let SemanticEvent::SessionStart {
            session_id,
            model,
            extra,
        } = &event
        {
            self.update_session_state(session_id, model);
            self.claude_api_key_source = extra
                .get("api_key_source")
                .and_then(Value::as_str)
                .map(String::from);
        }

        // 3. Update structured summary's tool-name rollup and the
        //    final-response accumulator. The accumulator captures only the
        //    output text emitted after the last tool call: a `ToolCall`
        //    resets it (dropping any narration that preceded the tool), and
        //    `OutputText` appends to it. `inline-compose` writes this final
        //    turn — never the full accumulated narration — into the body.
        match &event {
            SemanticEvent::ToolCall { name, .. } => {
                if let Ok(mut details) = self.summary_details.lock() {
                    if let Some(n) = name {
                        details.record_tool_name(n);
                    }
                    details.reset_final_response();
                }
            }
            SemanticEvent::OutputText { text, .. } => {
                if let Ok(mut details) = self.summary_details.lock() {
                    details.push_final_response(text);
                }
            }
            _ => {}
        }

        // 4. Update shared watchdog state for subagent tracking.
        //    Start/stop manage the active set; any event with a recognized
        //    subagent id resets `last_progress_at` for that id.
        {
            let now = std::time::Instant::now();
            if let Ok(mut state) = self.watchdog_state.lock() {
                match &event {
                    SemanticEvent::SubagentStart { id, name, .. } => {
                        state.subagent_started(id.clone().unwrap_or_default(), name.clone(), now);
                    }
                    SemanticEvent::SubagentStop {
                        id,
                        name,
                        status,
                        extra,
                        ..
                    } => {
                        if let Some(id) = id {
                            let description = extra
                                .get("description")
                                .and_then(|v| v.as_str())
                                .map(String::from)
                                .or_else(|| {
                                    extra
                                        .get("subagent_type")
                                        .and_then(|v| v.as_str())
                                        .map(String::from)
                                });
                            state.subagent_stopped(
                                id,
                                name.clone(),
                                description,
                                status.clone(),
                                now,
                            );
                        }
                    }
                    _ => {
                        for subagent_id in extract_subagent_ids_from_event(&event) {
                            state.observe_subagent_progress(&subagent_id, now);
                        }
                    }
                }
            }
        }

        // 5. Forward text/reasoning to their dedicated renderers before the
        //    status-line rendering so stdout writes happen in stream order.
        match &event {
            SemanticEvent::OutputText { text, .. } if self.emit_output_text.is_some() => {
                // Route the transition into the FinalStdout section
                // through the shared section tracker so the separator
                // blank (between stderr events and final stdout) is
                // emitted exactly once. The raw text bytes continue to
                // flow directly to the caller's renderer.
                //
                // The separator is suppressed in two cases:
                // 1. The combined output is already at a visual blank row
                //    (previous stdout ended with \n\n, or the last stderr
                //    line was blank). Injecting would create consecutive
                //    blank lines.
                // 2. The text itself starts with \n. The text provides its
                //    own visual break, making the separator redundant.
                let needs_separator = {
                    let mut tracker = self
                        .section_tracker
                        .lock()
                        .unwrap_or_else(|e| e.into_inner());
                    tracker
                        .classify(Section::FinalStdout, "x")
                        .is_some_and(|(s, _)| s)
                };
                let text_starts_with_newline = text.starts_with('\n');
                if needs_separator && !self.at_visual_blank() && !text_starts_with_newline {
                    (self.emit_stderr)("");
                    self.at_blank_row = true;
                }
                if let Some(emit) = self.emit_output_text.as_mut() {
                    emit(text);
                }
                self.update_stdout_trailing_newlines(text);
            }
            SemanticEvent::Reasoning { text, .. } => {
                let block = render_thinking_block(text, &self.terminal);
                if !block.is_empty() {
                    // Split the multi-line block render into lines so the
                    // dedup works per-line; section transitions only
                    // insert blanks between sections, not between lines of
                    // a single block.
                    for line in block.lines() {
                        self.emit_section_line(Section::Thinking, line);
                    }
                }
            }
            _ => {}
        }

        // 6. Render status line to STDERR.
        self.render_event(&event);

        // 7. Dispatch to agentic hooks when applicable, and log the
        //    resulting `DispatchEventMeta` to JSONL when a logger is wired.
        //    The logger always sees every event (even Output/Reasoning which
        //    have no agentic mapping); we build a Notification-shaped meta
        //    for those so the JSONL row still carries the full serialized
        //    semantic event under `extra["semantic_event"]`.
        let agentic = Self::to_agentic(&event);
        let log_agentic = agentic.unwrap_or(AgenticEvent::Notification);
        let meta = self.dispatch_meta(&event, log_agentic);
        if let Some(emit_log) = self.emit_event_log.as_ref() {
            emit_log(&event, &meta);
        }
        if let Some(agentic) = agentic {
            (self.dispatch)(agentic, meta);
        }
    }
}

/// Extract any recognized subagent identifiers from a semantic event.
///
/// Returns all ids found so the caller can update `last_progress_at`
/// for every matching active entry.  The list is usually empty or a
/// single element, but OpenCode `task_progress` payloads may carry a
/// `task_id` alongside the primary event id.
fn extract_subagent_ids_from_event(
    event: &SemanticEvent,
) -> Vec<super::super::subagent_watchdog::SubagentId> {
    use super::super::subagent_watchdog::SubagentId;

    let mut ids = Vec::new();

    match event {
        SemanticEvent::SubagentStart { id, .. } | SemanticEvent::SubagentStop { id, .. } => {
            if let Some(id) = id {
                ids.push(id.clone());
            }
        }
        SemanticEvent::Info { extra, .. } => {
            // OpenCode task_progress style: extra["task_id"] or extra["id"]
            if let Some(task_id) = extra.get("task_id").and_then(Value::as_str) {
                ids.push(SubagentId::from(task_id));
            } else if let Some(id) = extra.get("id").and_then(Value::as_str) {
                ids.push(SubagentId::from(id));
            }
        }
        SemanticEvent::ProviderExtension { payload, .. } => {
            // Some provider extensions carry a task/subagent id in the payload.
            if let Some(task_id) = payload.get("task_id").and_then(Value::as_str) {
                ids.push(SubagentId::from(task_id));
            } else if let Some(id) = payload.get("id").and_then(Value::as_str) {
                ids.push(SubagentId::from(id));
            }
        }
        _ => {}
    }

    ids
}
