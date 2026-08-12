//! Tool-call rendering — the `ToolCallDisplay` contract, tool-result bodies,
//! file-tool errors, and the `task_progress` dedup helpers.

use biscuit_terminal::components::block_quote::BlockQuote;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::{RenderableTerminalContent, TerminalRenderable};
use biscuit_terminal::components::status::StatusState;
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::color::{Color, Tailwind};
use biscuit_terminal::utils::layout::{Edges, Length, WordWrap};
use serde_json::Value;

use super::{EventRenderer, RenderUnit, escape_prose};
use crate::provider::EventClass;
use crate::stream::semantic::SemanticEvent;
use crate::stream::tool_display::{ToolCallDisplay, ToolDirection, ToolStatus};

pub(super) const TOOL_RESULT_BODY_MAX_LINES: usize = 10;

/// Bounded, normalized text extracted from a successful tool result, ready to
/// render as a block quote.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ToolResultBody {
    pub(super) text: String,
    pub(super) truncated: bool,
}

impl EventRenderer {
    /// Render the bounded body of a successful Codex tool result. The
    /// header line stays terse (`← Zsh(successful)`), while the actual
    /// stdout/stderr text is surfaced immediately below as a purple
    /// BlockQuote with grey text.
    pub(super) fn render_tool_result_body(
        &self,
        terminal: &Terminal,
        body: &ToolResultBody,
    ) -> Vec<String> {
        let mut lines = Vec::new();
        let prose = Prose::new(escape_prose(&body.text))
            .with_word_wrap(WordWrap::Truncate(Some("…".into())));
        let mut block = BlockQuote::new(RenderableTerminalContent::from(prose), None::<&str>)
            .with_text_color(Color::Tailwind(Tailwind::Gray500))
            .with_left_block_color(Color::Tailwind(Tailwind::Purple700))
            .with_border("\u{2503} ");
        block.layout_mut().margin = Edges::x(Length::ch(0));
        block.layout_mut().margin = Edges::x(Length::ch(0));
        let rendered = block.render(terminal);
        for line in rendered.lines() {
            lines.push(line.to_string());
        }

        if body.truncated {
            let note = Prose::new("<b>tool call</b>'s response truncated for brevity".to_string());
            let mut note_block =
                BlockQuote::new(RenderableTerminalContent::from(note), None::<&str>)
                    .with_left_block_color(Color::Tailwind(Tailwind::Orange700))
                    .with_border("\u{2503} ");
            note_block.layout_mut().margin = Edges::x(Length::ch(0));
            note_block.layout_mut().margin = Edges::x(Length::ch(0));
            let rendered = note_block.render(terminal);
            for line in rendered.lines() {
                lines.push(line.to_string());
            }
        }
        lines
    }

    /// Render a `ToolCallDisplay` into prose-markup for
    /// [`Status::from_prose`](biscuit_terminal::components::status::Status::from_prose).
    /// Per spec:
    ///
    /// - Outgoing summary / incoming success / incoming pending render the
    ///   slot as dim-italic (`<dim><i>…</i></dim>`).
    /// - Incoming error renders the word `error` as red + bold, followed
    ///   by a dim-italic error snippet (exit code + last non-empty line of
    ///   output) when the upstream event provides one.
    /// - User-controlled content (commands, URLs, paths, raw JSON
    ///   fallbacks) is passed through [`escape_prose`] so stray `<`, `>`,
    ///   `{`, or `\` cannot be interpreted as prose markup.
    /// - File-tool paths (`Read`/`Write`/`Edit`/`read_file`/…) are
    ///   rendered as OSC8 hyperlinks whose *visible* text is
    ///   cwd-relative or `~/`-relative; the full absolute path stays in
    ///   the `href` so click-to-open still works. Success lines append
    ///   the same link after a comma so each direction identifies its
    ///   target.
    /// - The slot is wrapped in parentheses attached to the tool name
    ///   (e.g. `→ Bash(<dim><i>bash ls</i></dim>)`) so the rendering
    ///   reads like a function call rather than a `Name · summary` pair.
    pub(super) fn render_tool_display(&self, display: ToolCallDisplay) -> String {
        let arrow = match display.direction {
            ToolDirection::Outgoing => '\u{2192}',
            ToolDirection::Incoming => '\u{2190}',
        };
        let name = escape_prose(&display.display_name);
        let is_file_tool = display.is_file_tool();
        let file_path = if is_file_tool {
            display.summary.as_deref()
        } else {
            None
        };
        let slot = match (display.status, &display.summary) {
            (Some(ToolStatus::Success), summary) => {
                let mut s = String::from("<dim><i>successful</i></dim>");
                if let Some(path) = file_path {
                    s.push_str(", ");
                    s.push_str(&format!("<dim>{}</dim>", self.render_file_link(path)));
                } else if let Some(text) = summary.as_deref().filter(|t| !t.is_empty()) {
                    s.push_str(", ");
                    s.push_str(&format!("<dim><i>{}</i></dim>", escape_prose(text)));
                }
                Some(s)
            }
            (Some(ToolStatus::Error), _) => {
                let mut s = String::from("<red><b>error</b></red>");
                if let Some(path) = file_path {
                    s.push_str(", ");
                    s.push_str(&format!("<dim>{}</dim>", self.render_file_link(path)));
                }
                if let Some(detail) = display.error_detail.as_deref().filter(|s| !s.is_empty()) {
                    s.push_str(&format!(" <dim><i>{}</i></dim>", escape_prose(detail)));
                }
                Some(s)
            }
            (Some(ToolStatus::Pending), summary) => {
                let mut s = String::from("<dim><i>pending</i></dim>");
                if let Some(path) = file_path {
                    s.push_str(", ");
                    s.push_str(&format!("<dim>{}</dim>", self.render_file_link(path)));
                } else if let Some(text) = summary.as_deref().filter(|t| !t.is_empty()) {
                    s.push_str(", ");
                    s.push_str(&format!("<dim><i>{}</i></dim>", escape_prose(text)));
                }
                Some(s)
            }
            (None, Some(summary)) => {
                if is_file_tool {
                    Some(self.render_file_link(summary))
                } else {
                    Some(format!("<dim><i>{}</i></dim>", escape_prose(summary)))
                }
            }
            (None, None) => None,
        };
        match slot {
            Some(text) => format!("{arrow} {name}({text})"),
            None => format!("{arrow} {name}"),
        }
    }

    /// Resolve `raw` into styled Prose markup using the renderer's cwd and
    /// home-directory context. Shared by outgoing and incoming file-tool
    /// rendering so the link shape stays consistent in both directions.
    pub(super) fn render_file_link(&self, raw: &str) -> String {
        crate::stream::path_link::format_file_link(raw, &self.cwd, self.home.as_deref())
    }

    /// Render a file-tool error as a two-part block: a `Warning` header
    /// line matching the legacy tool-call contract (`← Read(error, path)`)
    /// plus an orange `BlockQuote` carrying the provider's error message
    /// verbatim. The upstream provider may itself have truncated the
    /// message with a trailing ellipsis; this path never adds a local
    /// truncation of its own.
    pub(super) fn render_file_tool_error(
        &self,
        terminal: &Terminal,
        display: &ToolCallDisplay,
    ) -> Vec<String> {
        let name = escape_prose(&display.display_name);
        let path_markup = display
            .summary
            .as_deref()
            .filter(|s| !s.is_empty())
            .map(|p| self.render_file_link(p));
        let header = match path_markup {
            Some(link) => format!("\u{2190} {name}(<red><b>error</b></red>, <dim>{link}</dim>)"),
            None => format!("\u{2190} {name}(<red><b>error</b></red>)"),
        };
        let body = display
            .error_detail
            .as_deref()
            .filter(|s| !s.is_empty())
            .map(escape_prose)
            .unwrap_or_default();
        self.render_warning_header_and_body(terminal, header, &body)
    }

    /// Decide whether the pending `task_progress` Info line is redundant
    /// with the event about to render.
    ///
    /// - If `event` is a `ToolCall` whose summary visibly overlaps the
    ///   progress message's content tail (Reading `<x>` vs `→ Read(<x>)`),
    ///   drop the pending buffer silently.
    /// - Otherwise flush the pending buffer as a standard `Info` status
    ///   line and clear it. This keeps genuinely novel progress prose on
    ///   stderr while collapsing the common duplicate pair.
    pub(super) fn resolve_pending_task_progress(
        &mut self,
        event: &SemanticEvent,
        terminal: &Terminal,
        units: &mut Vec<RenderUnit>,
    ) {
        let Some(pending) = self.pending_task_progress.take() else {
            return;
        };
        if pending_matches_tool_call(&pending, event) {
            return;
        }
        let text = self.status_line(terminal, StatusState::Info, pending);
        units.push(RenderUnit {
            class: EventClass::StepProgress,
            text,
        });
    }
}

pub(super) fn tool_result_body(event: &SemanticEvent) -> Option<ToolResultBody> {
    let SemanticEvent::ToolResult { status, output, .. } = event else {
        return None;
    };
    let status = status.as_deref()?;
    if !matches!(status, "success" | "completed" | "ok") {
        return None;
    }
    let output = output.as_ref()?;
    let text = tool_result_output_text(output)?;
    let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
    let trimmed = normalized.trim_matches('\n');
    if trimmed.trim().is_empty() {
        return None;
    }
    let lines: Vec<&str> = trimmed.lines().collect();
    let truncated = lines.len() > TOOL_RESULT_BODY_MAX_LINES;
    let body = lines
        .into_iter()
        .take(TOOL_RESULT_BODY_MAX_LINES)
        .collect::<Vec<_>>()
        .join("\n");
    if body.trim().is_empty() {
        return None;
    }
    Some(ToolResultBody {
        text: body,
        truncated,
    })
}

pub(super) fn tool_result_output_text(value: &Value) -> Option<String> {
    if let Some(s) = value.as_str().filter(|s| !s.is_empty()) {
        return Some(s.to_string());
    }
    if let Some(obj) = value.as_object() {
        for key in ["aggregated_output", "stderr", "stdout", "message", "text"] {
            if let Some(Value::String(s)) = obj.get(key)
                && !s.is_empty()
            {
                return Some(s.clone());
            }
        }
        if let Some(Value::Array(parts)) = obj.get("content") {
            let collected = parts
                .iter()
                .filter_map(|part| part.get("text").and_then(Value::as_str))
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
                .join("\n");
            if !collected.is_empty() {
                return Some(collected);
            }
        }
    }
    if let Some(arr) = value.as_array() {
        let collected = arr
            .iter()
            .filter_map(|part| {
                part.get("text")
                    .and_then(Value::as_str)
                    .or_else(|| part.as_str())
            })
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("\n");
        if !collected.is_empty() {
            return Some(collected);
        }
    }
    None
}

/// Return `true` when the `pending` progress message refers to the same
/// work as the tool call about to render.
///
/// Strategy: strip the leading verb (`Reading `, `Running `, `Writing `,
/// `Editing `, `Searching `, `Fetching `, `Listing `) from the progress
/// message and compare the remainder with the tool call's extracted
/// summary. A minimum-length shared overlap guards against unrelated
/// short progress lines accidentally cancelling each other.
pub fn pending_matches_tool_call(pending: &str, event: &SemanticEvent) -> bool {
    let SemanticEvent::ToolCall { .. } = event else {
        return false;
    };
    let Some(display) = ToolCallDisplay::from_call(event) else {
        return false;
    };
    let Some(summary) = display.summary.as_deref() else {
        return false;
    };
    let progress_tail = strip_progress_verb(pending.trim()).trim();
    let summary_tail = summary.trim();
    if progress_tail.is_empty() || summary_tail.is_empty() {
        return false;
    }
    const MIN_SHARED: usize = 6;
    if progress_tail.len() < MIN_SHARED && summary_tail.len() < MIN_SHARED {
        return false;
    }
    summary_tail.contains(progress_tail) || progress_tail.contains(summary_tail)
}

/// Strip a progress-style leading verb (`Reading`, `Running`, …) from
/// `message` and return the remainder. Returns the original string when
/// no known verb prefix is present so unrelated Info lines are not
/// truncated.
pub fn strip_progress_verb(message: &str) -> &str {
    const VERBS: &[&str] = &[
        "Reading ",
        "Running ",
        "Writing ",
        "Editing ",
        "Searching ",
        "Fetching ",
        "Listing ",
        "Grepping ",
        "Globbing ",
    ];
    for verb in VERBS {
        if let Some(rest) = message.strip_prefix(verb) {
            return rest;
        }
    }
    message
}
