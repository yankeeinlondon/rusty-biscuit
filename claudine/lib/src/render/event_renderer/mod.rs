//! The [`EventRenderer`] — the functional home for the live sink's STDERR
//! status dispatch (design/render-components.md, ruling 2: "parser emits;
//! EventRenderer dispatches").
//!
//! The renderer consumes normalized [`SemanticEvent`]s plus a provider
//! [`DisplayPolicy`] and produces [`RenderUnit`]s — each unit is exactly one
//! of the live sink's `emit_section_line` calls, at today's emission
//! granularity. It contains **zero** `match provider` / `provider ==`:
//! provider variance enters only as `DisplayPolicy` data, so two providers'
//! equivalent events render identically by construction. Reads of an event's
//! *own* `provider` field (e.g. a `ProviderExtension`'s short name) are data
//! lookups, not dispatch.
//!
//! [`DISPATCH`] is the load-bearing artifact: an exhaustive variant-name →
//! [`Disposition`] table whose completeness is compiler- and test-enforced
//! against [`SemanticEvent`]'s `strum::VariantNames`.

mod error_block;
mod helpers;
mod provider_extension;
mod tool_use;

use std::path::PathBuf;

use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::components::status::{Status, StatusState};
use biscuit_terminal::terminal::Terminal;
use serde_json::Value;

use crate::provider::{DisplayPolicy, EventClass, ToolResultSummary, Provider, provider_info};
use crate::stream::semantic::SemanticEvent;
use crate::stream::stderr::Verbosity;
use crate::stream::tool_display::{ToolCallDisplay, ToolStatus};

pub use error_block::error_kind_presentation;
pub(crate) use helpers::escape_prose;
use helpers::subagent_description;
pub use tool_use::{pending_matches_tool_call, strip_progress_verb};

/// One rendered STDERR emission — the string plus the event class that decides
/// which output section the sink routes it to. Each unit corresponds to
/// exactly one `emit_section_line` call in the live sink.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderUnit {
    /// Classifies the unit for section routing. This round every produced
    /// unit routes to the tool-use / events section; `Thinking` and
    /// `FinalMessage` are reserved for the streaming paths that stay
    /// sink-side (part 2).
    pub class: EventClass,
    /// The fully rendered line, ready to hand to `emit_section_line`.
    pub text: String,
}

/// What the renderer does with a given [`SemanticEvent`] variant.
///
/// The [`DISPATCH`] table pairs every variant name with one of these so the
/// render path has the equivalent of enum exhaustiveness: a new variant with
/// no entry fails [`dispatch_table_covers_every_variant`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Disposition {
    /// Rendered by the named component into one or more [`RenderUnit`]s.
    Component(&'static str),
    /// Envelope-only or policy-suppressed — produces no [`RenderUnit`].
    Silent,
    /// Rendered through the sink's dedicated streaming paths (OutputText,
    /// Reasoning), which stay in the CLI for this migration (part 2).
    Streaming,
}

/// Exhaustive variant-name → [`Disposition`] mapping. Keyed by the PascalCase
/// [`SemanticEvent`] variant name (matching `strum::VariantNames`).
pub const DISPATCH: &[(&str, Disposition)] = &[
    ("SessionStart", Disposition::Silent),
    ("TurnStart", Disposition::Silent),
    ("TurnComplete", Disposition::Silent),
    ("OutputText", Disposition::Streaming),
    ("Reasoning", Disposition::Streaming),
    ("ToolCall", Disposition::Component("ToolUse")),
    ("ToolResult", Disposition::Component("ToolUse")),
    ("PermissionRequest", Disposition::Silent),
    ("SubagentStart", Disposition::Component("Status")),
    ("SubagentStop", Disposition::Component("Status")),
    ("FileChange", Disposition::Component("Status")),
    ("PlanUpdate", Disposition::Component("Status")),
    ("Info", Disposition::Component("Status")),
    ("Warning", Disposition::Component("Status")),
    ("Error", Disposition::Component("ErrorBlock")),
    ("ProviderExtension", Disposition::Component("Status")),
];

/// Functional renderer for the live sink's STDERR status dispatch.
///
/// Holds the per-provider render policy plus the small amount of render-scoped
/// state that used to live on the sink: the cwd/home used for file-link
/// rendering, the auth source consulted for rate-limit suppression, and the
/// one-slot `task_progress` dedup buffer.
pub struct EventRenderer {
    /// Provider render policy. The single source of truth for every
    /// provider-conditional decision in this module — there is no stored
    /// `Provider` and no `match provider`.
    policy: &'static DisplayPolicy,
    cwd: PathBuf,
    home: Option<PathBuf>,
    /// Auth source reported at `SessionStart`, consulted by the Claude
    /// rate-limit suppression policy. Self-updated in [`Self::render`].
    api_key_source: Option<String>,
    /// One-slot buffer holding the most recent `task_progress` narration so
    /// it can collapse into the matching tool-call line that follows (Claude's
    /// "Reading `<path>`" → `→ Read(<path>)`). Flushed at stream end.
    pending_task_progress: Option<String>,
}

impl EventRenderer {
    /// Build a renderer for `provider`. Takes a [`Provider`] as constructor
    /// sugar (design ruling 1) but stores only the resolved policy, so the
    /// render internals never see a `Provider` to branch on.
    pub fn new(provider: Provider, cwd: PathBuf, home: Option<PathBuf>) -> Self {
        Self {
            policy: &provider_info(provider).display_policy,
            cwd,
            home,
            api_key_source: None,
            pending_task_progress: None,
        }
    }

    /// The auth source last reported at `SessionStart`, if any.
    pub fn api_key_source(&self) -> Option<&str> {
        self.api_key_source.as_deref()
    }

    /// Render one event into zero or more emission units.
    ///
    /// An empty vec means "silent" (envelope-only, policy-suppressed, or
    /// verbosity-gated). The `Silent` verbosity gate lives here so the sink
    /// stays simple; `SessionStart`'s `api_key_source` self-update runs
    /// *before* the gate to match the sink's previous unconditional update.
    pub fn render(
        &mut self,
        event: &SemanticEvent,
        terminal: &Terminal,
        verbosity: Verbosity,
    ) -> Vec<RenderUnit> {
        // Self-update the auth source on every SessionStart, regardless of
        // verbosity — later Warning suppression reads it and the update must
        // not depend on whether the SessionStart itself rendered anything.
        if let SemanticEvent::SessionStart { extra, .. } = event {
            self.api_key_source = extra
                .get("api_key_source")
                .and_then(Value::as_str)
                .map(String::from);
        }

        if verbosity == Verbosity::Silent {
            return Vec::new();
        }

        let mut units = Vec::new();

        // Redundancy suppression: Claude's `task_progress` events arrive
        // immediately before the matching tool call ("Reading <path>" →
        // `→ Read(<path>)`). Hold the most recent progress message in a
        // one-slot buffer and consult it on the next event so the pair
        // collapses to the canonical tool line.
        if provider_extension::is_claude_task_progress(self.policy, event) {
            if let SemanticEvent::Info { message, .. } = event {
                self.pending_task_progress = Some(message.clone());
            }
            return units;
        }
        self.resolve_pending_task_progress(event, terminal, &mut units);

        match event {
            SemanticEvent::ToolCall { .. } => {
                if let Some(display) = ToolCallDisplay::from_call(event) {
                    let desc = self.render_tool_display(display);
                    units.push(self.tool_status_prose(terminal, desc));
                }
            }
            SemanticEvent::ToolResult { .. } => {
                if let Some(display) = ToolCallDisplay::from_result(event) {
                    if display.is_file_tool() && display.status == Some(ToolStatus::Error) {
                        for line in self.render_file_tool_error(terminal, &display) {
                            units.push(RenderUnit {
                                class: EventClass::ToolUse,
                                text: line,
                            });
                        }
                    } else {
                        // PreferBody policy (Codex today): a successful
                        // non-file result with a renderable body drops the
                        // one-line summary and renders the body as a block
                        // quote at Normal verbosity.
                        let prefer_body =
                            self.policy.tool_result_summary == ToolResultSummary::PreferBody;
                        let body = tool_use::tool_result_body(event);
                        let mut header_display = display.clone();
                        if prefer_body
                            && body.is_some()
                            && header_display.status == Some(ToolStatus::Success)
                            && !header_display.is_file_tool()
                        {
                            header_display.summary = None;
                        }
                        let desc = self.render_tool_display(header_display);
                        units.push(self.tool_status_prose(terminal, desc));
                        if prefer_body
                            && verbosity == Verbosity::Normal
                            && let Some(body) = body
                        {
                            for line in self.render_tool_result_body(terminal, &body) {
                                units.push(RenderUnit {
                                    class: EventClass::ToolUse,
                                    text: line,
                                });
                            }
                        }
                    }
                }
            }
            SemanticEvent::SubagentStart { name, .. } => {
                let text = self.status_line(
                    terminal,
                    StatusState::Subagent,
                    subagent_description('\u{2192}', name),
                );
                units.push(RenderUnit {
                    class: EventClass::SubagentActivity,
                    text,
                });
            }
            SemanticEvent::SubagentStop { name, .. } => {
                let text = self.status_line(
                    terminal,
                    StatusState::Subagent,
                    subagent_description('\u{2190}', name),
                );
                units.push(RenderUnit {
                    class: EventClass::SubagentActivity,
                    text,
                });
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
                    return units;
                }
                let kind_label = kind.unwrap_or("change");
                let line = if path.is_empty() {
                    kind_label.to_string()
                } else {
                    format!("{kind_label} {path}")
                };
                let text = self.status_line(terminal, StatusState::Info, line);
                units.push(RenderUnit {
                    class: EventClass::FileChange,
                    text,
                });
            }
            SemanticEvent::PlanUpdate { message, .. } => {
                if let Some(msg) = message {
                    let text = self.status_line(terminal, StatusState::Info, msg.clone());
                    units.push(RenderUnit {
                        class: EventClass::PlanUpdate,
                        text,
                    });
                }
            }
            SemanticEvent::Info { message, extra } => {
                // Suppress `step_start` / `step_finish` phase markers from
                // the rendered stderr surface when the provider's policy
                // suppresses the StepProgress event class (OpenCode today:
                // internal phase boundaries between tool batches carry no
                // user-visible meaning and produce visual noise around tool
                // lines). The events still flow through dispatch, JSONL
                // logging, and the LiveMetrics heartbeat — only the
                // human-visible Status line is suppressed. Suppression is
                // gated on `extra["step_phase"]` (the StepProgress marker)
                // so unrelated Info events are unaffected.
                //
                // Invisible-event invariant: suppressed events MUST produce
                // no RenderUnit so the sink's `SectionTracker` state is not
                // updated. This prevents the tracker from detecting a phantom
                // section change that would inject a redundant separator
                // before the next visible event.
                if self
                    .policy
                    .info_event_suppression
                    .contains(&EventClass::StepProgress)
                    && extra.get("step_phase").is_some()
                {
                    return units;
                }
                let text = self.status_line(terminal, StatusState::Info, message.clone());
                units.push(RenderUnit {
                    class: EventClass::StepProgress,
                    text,
                });
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
                    || provider_extension::is_suppressed_claude_rate_limit(
                        self.policy,
                        message,
                        extra,
                        self.api_key_source.as_deref(),
                    )
                {
                    return units;
                }
                // Codex stderr bridge emits Warnings enriched with a
                // `tracing_target` extra. Those want a two-line rendering
                // (Status header + orange BlockQuote) so operators can read
                // the diagnostic without the raw `TIMESTAMP LEVEL ...`
                // formatting leaking through.
                if let Some(target) = extra.get("tracing_target").and_then(Value::as_str) {
                    for line in self.render_tracing_diagnostic(terminal, target, message) {
                        units.push(RenderUnit {
                            class: EventClass::HookEvent,
                            text: line,
                        });
                    }
                } else {
                    let text = self.status_line(terminal, StatusState::Warning, message.clone());
                    units.push(RenderUnit {
                        class: EventClass::HookEvent,
                        text,
                    });
                }
            }
            SemanticEvent::Error { message, kind, .. } => {
                for line in self.render_error_block(terminal, *kind, message) {
                    units.push(RenderUnit {
                        class: EventClass::Error,
                        text: line,
                    });
                }
            }
            SemanticEvent::ProviderExtension {
                provider,
                kind,
                payload,
            } => {
                if provider_extension::is_silent_extension_kind(*provider, kind) {
                    // Suppress stderr rendering; the event still flows
                    // through dispatch and the JSONL log.
                    return units;
                }
                let text = self.status_line(
                    terminal,
                    StatusState::Info,
                    provider_extension::provider_extension_description(*provider, kind, payload),
                );
                units.push(RenderUnit {
                    class: EventClass::HookEvent,
                    text,
                });
            }
            // OutputText / Reasoning / SessionStart / TurnStart / TurnComplete /
            // PermissionRequest do not render through Status — Output/Reasoning
            // flow through the sink's own streaming renderers (part 2); the
            // others are envelope-only.
            SemanticEvent::SessionStart { .. }
            | SemanticEvent::TurnStart { .. }
            | SemanticEvent::TurnComplete { .. }
            | SemanticEvent::OutputText { .. }
            | SemanticEvent::Reasoning { .. }
            | SemanticEvent::PermissionRequest { .. } => {}
        }

        units
    }

    /// Flush any buffered `task_progress` narration. Called when the stream
    /// terminates so a progress line that never matched a follow-up tool call
    /// is not lost. Empty when the buffer is empty.
    pub fn flush(&mut self, terminal: &Terminal) -> Vec<RenderUnit> {
        let mut units = Vec::new();
        if let Some(pending) = self.pending_task_progress.take() {
            let text = self.status_line(terminal, StatusState::Info, pending);
            units.push(RenderUnit {
                class: EventClass::StepProgress,
                text,
            });
        }
        units
    }

    /// Render a plain [`Status`] line (no prose-markup interpretation).
    fn status_line(&self, terminal: &Terminal, state: StatusState, description: String) -> String {
        Status::new(description).state(state).render(terminal)
    }

    /// Render a tool-call / tool-result [`Status`] line from prose markup with
    /// the tool-call hanging indent.
    fn tool_status_prose(&self, terminal: &Terminal, description: String) -> RenderUnit {
        RenderUnit {
            class: EventClass::ToolUse,
            text: self.status_prose_line(terminal, StatusState::ToolUse, description),
        }
    }

    fn status_prose_line(
        &self,
        terminal: &Terminal,
        state: StatusState,
        description: String,
    ) -> String {
        let mut status = Status::from_prose(description).state(state);
        // Tool-call and tool-result lines are the only consumers of this
        // helper today and they render as `→ Name(details)` / `← Name(…)`.
        // The Status default hanging indent of 2 lines continuation lines
        // up under the tool name glyph — but not under the text past the
        // `→ ` arrow. Bumping the hanging indent by 2 (one for the arrow,
        // one for the trailing space) lines the wrap under the first
        // letter of the tool name, which is what users expect.
        status.layout_mut().word_wrap = status.layout().word_wrap.clone().with_hanging_indent(4);
        status.render(terminal)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use strum::VariantNames;

    #[test]
    fn dispatch_table_covers_every_variant() {
        let mapped: std::collections::BTreeSet<&str> =
            DISPATCH.iter().map(|(name, _)| *name).collect();
        let variants: std::collections::BTreeSet<&str> =
            SemanticEvent::VARIANTS.iter().copied().collect();
        assert_eq!(
            mapped, variants,
            "DISPATCH must map exactly the SemanticEvent variants — no more, no less"
        );
    }

    #[test]
    fn dispatch_table_has_no_duplicate_entries() {
        let mut seen = std::collections::BTreeSet::new();
        for (name, _) in DISPATCH {
            assert!(seen.insert(*name), "duplicate DISPATCH entry for {name}");
        }
    }
}
