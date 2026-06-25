//! Breach-message formatting and standalone error rendering for the watchdog.
//!
//! [`format_step_timeout_breach_message`] turns the outstanding / stuck-item
//! snapshot into the operator-facing diagnostic; [`render_watchdog_error_to_stream`]
//! draws a breach as an `AgentNative` error block on stderr from the ticker
//! thread, without needing the live sink's section tracker.

use std::time::{Duration, Instant};

use super::super::subagent_watchdog::{ActiveSubagentSnapshot, RecentSubagentInfo};
use super::super::termination::WatchdogTermination;
use super::super::super::stream_io::StreamOutput;

/// Extra diagnostic context used when enriching an OpenCode `step_timeout`
/// breach. Only populated when the provider is OpenCode and the standard
/// `outstanding_at_breach()` snapshot is empty.
#[derive(Debug, Clone)]
pub(crate) struct OpenCodeBreachContext {
    pub(crate) subagent_done_count: u32,
    pub(crate) step_in_flight: bool,
    pub(crate) recent_subagents: std::collections::VecDeque<RecentSubagentInfo>,
    pub(crate) now: Instant,
}

/// Format the human-readable breach message for a `step_timeout` event.
///
/// When `outstanding` is non-empty the message enumerates the subagents
/// that were still in flight at the moment the silence rule fired
/// (id, optional name, elapsed since last progress) so the operator
/// knows which workers stalled. When empty, only the silence duration
/// is reported.
///
/// When `stuck_tools` or `stuck_subagents` is non-empty the message also
/// enumerates those stuck items.
///
/// For OpenCode, when `outstanding` is empty, `opencode_context` provides
/// enriched diagnostics: subagent completion count, whether a step was in
/// flight, and the most recent completed subagents from the ring-buffer.
pub(crate) fn format_step_timeout_breach_message(
    silence: Duration,
    outstanding: &[ActiveSubagentSnapshot],
    stuck_tools: &[claudine::stream::progress::InFlightTool],
    stuck_subagents: &[claudine::stream::progress::InFlightSubagent],
    opencode_context: Option<OpenCodeBreachContext>,
) -> String {
    let silence_text = format_duration(silence);
    if outstanding.is_empty() && stuck_tools.is_empty() && stuck_subagents.is_empty() {
        let mut msg =
            format!("no stream activity for {silence_text}; terminating due to step_timeout");
        if let Some(ctx) = opencode_context {
            if ctx.subagent_done_count > 0 {
                let plural = if ctx.subagent_done_count == 1 {
                    "subagent"
                } else {
                    "subagents"
                };
                let count = ctx.subagent_done_count;
                msg = format!("{count} {plural} observed in this step. {msg}");
            }
            if let Some(last) = ctx.recent_subagents.front() {
                let since = format_duration(ctx.now.saturating_duration_since(last.completed_at));
                msg.push_str(&format!(" Last subagent completion {since} ago."));
            }
            if ctx.step_in_flight {
                // A breach with `step_in_flight=true` can only happen on the
                // byte-heartbeat backstop path (both event and byte clocks
                // stale) — the per-step grace otherwise suppresses the
                // silence rule while a step is open. Anchor the wording so
                // operators understand the child genuinely produced no
                // bytes during the silence window, not that a step closed
                // and then went silent.
                msg.push_str(&format!(
                    " A step boundary was still open and the child produced no bytes for {silence_text} — \
                     the parent agent may have been waiting on a parallel subagent that has not yet returned.",
                ));
            }
            if !ctx.recent_subagents.is_empty() {
                msg.push_str("\n\nRecent subagents:");
                for info in &ctx.recent_subagents {
                    let since =
                        format_duration(ctx.now.saturating_duration_since(info.completed_at));
                    let desc = info.description.as_ref().or(info.name.as_ref());
                    let label = desc.map(|s| s.as_str()).unwrap_or("(unnamed)");
                    let status = info.status.as_deref().unwrap_or("unknown");
                    msg.push_str(&format!("\n  ← {label} ({since} ago, {status})"));
                }
            }
        }
        return msg;
    }

    let mut lines =
        format!("no stream activity for {silence_text}. The wrapped process was terminated.");

    if !stuck_tools.is_empty() {
        let count = stuck_tools.len();
        let plural = if count == 1 { "tool" } else { "tools" };
        lines.push_str(&format!(
            " {count} {plural} were stuck when the timeout fired:\n"
        ));
        for tool in stuck_tools {
            let name = tool.name.as_deref().unwrap_or("(unnamed)");
            lines.push_str(&format!("  • \"{name}\"\n"));
        }
    }

    if !stuck_subagents.is_empty() {
        let count = stuck_subagents.len();
        let plural = if count == 1 { "subagent" } else { "subagents" };
        lines.push_str(&format!(
            " {count} {plural} were stuck when the timeout fired:\n"
        ));
        for subagent in stuck_subagents {
            let name = subagent.name.as_deref().unwrap_or("(unnamed)");
            lines.push_str(&format!("  • \"{name}\"\n"));
        }
    }

    if !outstanding.is_empty() {
        let count = outstanding.len();
        let plural = if count == 1 { "subagent" } else { "subagents" };
        lines.push_str(&format!(
            " {count} {plural} were still outstanding when the timeout fired:\n"
        ));
        for snap in outstanding {
            let idle = format_duration(snap.elapsed_since_progress);
            let name = snap.name.as_deref().unwrap_or("(unnamed)");
            lines.push_str(&format!("  • {} \"{name}\" (idle {idle})\n", snap.id));
        }
    }

    lines
}

/// Format a duration in a human-readable way (e.g. "3m 0s").
pub(crate) fn format_duration(d: Duration) -> String {
    let secs = d.as_secs();
    if secs < 60 {
        format!("{secs}s")
    } else if secs < 3_600 {
        format!("{}m {}s", secs / 60, secs % 60)
    } else {
        format!("{}h {}m {}s", secs / 3_600, (secs % 3_600) / 60, secs % 60)
    }
}

/// Render a watchdog breach as an `AgentNative` error block on stderr.
///
/// This is the standalone rendering path used by the watchdog ticker
/// thread so it can surface the error before the child is killed,
/// without needing access to the `LiveSemanticSink` section tracker.
pub(crate) fn render_watchdog_error_to_stream(
    termination: &WatchdogTermination,
    stream_output: &StreamOutput,
) {
    use biscuit_terminal::components::prose::Prose;
    use biscuit_terminal::components::renderable::TerminalRenderable;
    use biscuit_terminal::components::status::StatusState;
    use biscuit_terminal::prelude::StatusBlock;
    use biscuit_terminal::utils::color::{Color, Tailwind};
    use biscuit_terminal::utils::layout::{Length, Edges, TargetValue, WordWrap};

    let term = crate::log::terminal();
    let border_color = Color::Tailwind(Tailwind::Red700);
    let body_text = escape_prose(&termination.message);
    let body = format!("<red><b>Agent Error</b></red>\n{body_text}");
    let prose = Prose::new(body).with_word_wrap(WordWrap::WrapProse(None, None));
    let block = StatusBlock::new(StatusState::Error)
        .body(prose)
        .border_color(border_color)
        .left_margin(TargetValue::universal(Length::ch(0)))
        .right_margin(TargetValue::universal(Length::ch(0)));
    let rendered = block.render(&term);
    for line in rendered.lines() {
        stream_output.emit_stderr_line(line);
    }
}

/// Escape user-controlled text so it can be safely interpolated into
/// biscuit-terminal prose markup.
fn escape_prose(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '\\' | '<' | '>' | '{' => {
                out.push('\\');
                out.push(ch);
            }
            other => out.push(other),
        }
    }
    out
}
