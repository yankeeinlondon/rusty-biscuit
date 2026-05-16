//! `SemanticErrorKind` colored BlockQuote rendering.

use super::LiveSemanticSink;
use super::{Section, SemanticErrorKind};
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::components::status::StatusState;
use biscuit_terminal::prelude::StatusBlock;
use biscuit_terminal::utils::color::{Color, Tailwind};
use biscuit_terminal::utils::layout::{Margin, WordWrap};

impl LiveSemanticSink {
    /// Render a typed [`SemanticEvent::Error`] as a colored `BlockQuote`
    /// with a label and border derived from [`SemanticErrorKind`]. Each
    /// rendered line is fed through [`Self::emit_section_line`] so the
    /// surrounding section spacing stays consistent with status renders.
    pub(crate) fn render_error_block(
        &mut self,
        section: Section,
        kind: SemanticErrorKind,
        message: &str,
    ) {
        // When the user has Ctrl+C'd this run, the agent's dying-breath
        // event is not an "Agent Error" — relabel it so operators see the
        // real cause. The classification machinery itself stays untouched
        // because the parsers cannot know about our process-scoped flag.
        let interrupted = crate::output::user_interrupt_observed();
        let (label, border_color, body_text) = if interrupted {
            (
                "User Action",
                Color::Tailwind(Tailwind::Yellow700),
                "User pressed CTRL+C to stop the session".to_string(),
            )
        } else {
            let (label, border_color) = error_kind_presentation(kind);
            (label, border_color, super::escape_prose(message))
        };
        let body = format!("<red><b>{label}</b></red>\n{body_text}");
        let prose = Prose::new(body).with_word_wrap(WordWrap::WrapProse(None, None));
        let block = StatusBlock::new(StatusState::Error)
            .body(prose)
            .border_color(border_color)
            .left_margin(Margin::Chars(0))
            .right_margin(Margin::Chars(0));
        let rendered = block.render(&self.terminal);
        for line in rendered.lines() {
            self.emit_section_line(section, line);
        }
    }

    /// Render a Codex tracing-style diagnostic as a two-part stderr block:
    ///
    /// - A `Status`-shaped header line with an orange `WARN(<target>:)`
    ///   prose label so operators can see which module fired.
    /// - An orange-bordered `BlockQuote` body carrying the diagnostic text.
    ///
    /// Execution is not interrupted for these records (Codex classifies
    /// them at `ERROR` severity but continues) so the label is forced to
    /// `WARN` regardless of the incoming level.
    ///
    /// The BlockQuote border uses the centered `┃` glyph (U+2503) — the
    /// `StatusBlock` default — so the bar lines up visually with the
    /// centered `⚠` warning icon emitted one row above by
    /// `Status::from_prose(Warning)`.
    pub(crate) fn render_tracing_diagnostic(
        &mut self,
        section: Section,
        target: &str,
        message: &str,
    ) {
        let header_prose = format!(
            "<orange>WARN(<dim><i>{}:</i></dim>)</orange>",
            super::escape_prose(target)
        );
        self.render_warning_header_and_body(section, header_prose, &super::escape_prose(message));
    }

    /// Render the two-part `Warning` block shape: a `Status::from_prose`
    /// header with the [`StatusState::Warning`] glyph, followed by an
    /// orange-bordered `BlockQuote` carrying `body_prose` verbatim. Used
    /// by the Codex tracing bridge and the file-tool error path so both
    /// share the same spacing/indent conventions.
    pub(crate) fn render_warning_header_and_body(
        &mut self,
        section: Section,
        header_prose: String,
        body_prose: &str,
    ) {
        let border_color = Color::Tailwind(Tailwind::Orange700);
        let body =
            Prose::new(body_prose.to_string()).with_word_wrap(WordWrap::WrapProse(None, None));
        let block = StatusBlock::new(StatusState::Warning)
            .header(header_prose)
            .body(body)
            .border_color(border_color)
            .left_margin(Margin::Chars(0))
            .right_margin(Margin::Chars(0));
        let rendered = block.render(&self.terminal);
        for line in rendered.lines() {
            self.emit_section_line(section, line);
        }
    }
}

/// Pick the human label and border color for a typed
/// [`SemanticErrorKind`] when rendered as a live-sink BlockQuote.
pub(crate) fn error_kind_presentation(kind: SemanticErrorKind) -> (&'static str, Color) {
    match kind {
        SemanticErrorKind::Configuration => {
            ("Configuration Error", Color::Tailwind(Tailwind::Orange700))
        }
        SemanticErrorKind::AgentNative => ("Agent Error", Color::Tailwind(Tailwind::Red700)),
        SemanticErrorKind::ApiRemote => ("API Error", Color::Tailwind(Tailwind::Red700)),
        SemanticErrorKind::Interrupted => ("Interrupted", Color::Tailwind(Tailwind::Yellow700)),
        SemanticErrorKind::Unknown => ("Error", Color::Tailwind(Tailwind::Red700)),
    }
}
