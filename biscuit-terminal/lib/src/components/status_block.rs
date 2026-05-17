use std::any::Any;

use crate::{
    components::{
        block_quote::BlockQuote,
        prose::Prose,
        renderable::{RenderableTerminalContent, TerminalRenderable},
        status::{Status, StatusState},
    },
    terminal::Terminal,
    utils::{
        color::Color,
        layout::{Layout, Length, Margin, TargetValue},
        wrap_policy::WordWrap,
    },
};

/// A severity-colored block with optional header, body, and hint content.
///
/// `StatusBlock` combines a `Status` header line, a `BlockQuote` body, and an
/// optional prose hint into one renderable surface for error and warning-style
/// output.
#[derive(Debug, Clone)]
pub struct StatusBlock {
    severity: StatusState,
    header: Option<String>,
    body: Vec<Prose>,
    hint: Option<String>,
    border_color: Option<Color>,
    border: String,
    layout: Layout,
}

impl StatusBlock {
    /// Create a new status block for the given severity.
    pub fn new(severity: StatusState) -> Self {
        Self {
            severity,
            header: None,
            body: Vec::new(),
            hint: None,
            border_color: None,
            border: "┃ ".to_string(),
            layout: Layout {
                margin: Margin {
                    right: TargetValue::universal(Length::ch(5)),
                    ..Margin::default()
                },
                word_wrap: WordWrap::WrapProse(Some(8), None),
                ..Layout::default()
            },
        }
    }

    /// Set a prose-formatted header rendered as a [`Status`] line.
    pub fn header(mut self, header: impl Into<String>) -> Self {
        self.header = Some(header.into());
        self
    }

    /// Set the body content as a vector of [`Prose`] items.
    ///
    /// Each item is rendered individually and stacked vertically with a
    /// single blank line between them inside a continuous block quote.
    pub fn body(mut self, body: impl crate::components::prose::IntoProseVec) -> Self {
        self.body = body.into_prose_vec();
        self
    }

    /// Set the body to a single [`Prose`] item.
    ///
    /// Convenience shortcut for the common case where the body is a single
    /// styled line or paragraph.
    pub fn body_line(self, line: impl Into<Prose>) -> Self {
        self.body(vec![line.into()])
    }

    /// Set a prose-formatted hint rendered below the block quote.
    pub fn hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    /// Override the severity-derived border color.
    pub fn border_color(mut self, color: Color) -> Self {
        self.border_color = Some(color);
        self
    }

    /// Override the border glyph used by the body block quote.
    pub fn border(mut self, border: impl Into<String>) -> Self {
        self.border = border.into();
        self
    }

    fn resolved_border_color(&self) -> Color {
        self.border_color
            .unwrap_or_else(|| self.severity.default_color())
    }
}

impl TerminalRenderable for StatusBlock {
    fn render_optimistic(&self, term_width: Option<u32>) -> String {
        let term = Terminal::new_optimistic(term_width.unwrap_or(80));
        self.render(&term)
    }

    fn render(&self, term: &Terminal) -> String {
        let mut parts = Vec::new();

        if let Some(ref header_text) = self.header {
            let status = Status::from_prose(header_text).state(self.severity.clone());
            parts.push(status.render(term));
        }

        if !self.body.is_empty() {
            let composed = self
                .body
                .iter()
                .map(|p| p.render(term))
                .collect::<Vec<_>>()
                .join("\n\n");
            let mut block =
                BlockQuote::new(RenderableTerminalContent::String(composed), None::<&str>)
                    .with_left_block_color(self.resolved_border_color())
                    .with_border(&self.border);
            block.layout_mut().margin.left = self.layout.margin.left.clone();
            block.layout_mut().margin.right = self.layout.margin.right.clone();
            block.layout_mut().word_wrap = self.layout.word_wrap.clone();
            parts.push(block.render(term));
        }

        if let Some(ref hint_text) = self.hint {
            parts.push(Prose::new(hint_text).render(term));
        }

        parts.join("\n")
    }

    fn layout(&self) -> &Layout {
        &self.layout
    }

    fn layout_mut(&mut self) -> &mut Layout {
        &mut self.layout
    }

    fn is_block_level(&self) -> bool {
        true
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{discovery::detection::ColorDepth, utils::color::Tailwind};

    fn strip_ansi(s: &str) -> String {
        let mut result = String::with_capacity(s.len());
        let mut in_escape = false;
        for ch in s.chars() {
            if in_escape {
                if ch.is_ascii_alphabetic() {
                    in_escape = false;
                }
            } else if ch == '\x1b' {
                in_escape = true;
            } else {
                result.push(ch);
            }
        }
        result
    }

    fn no_color_terminal(width: u32) -> Terminal {
        let mut term = Terminal::builder()
            .width(width)
            .color_depth(ColorDepth::None)
            .build();
        term.is_nerd_font = Some(false);
        term
    }

    fn color_terminal(width: u32) -> Terminal {
        let mut term = Terminal::builder()
            .width(width)
            .color_depth(ColorDepth::TrueColor)
            .build();
        term.is_nerd_font = Some(false);
        term
    }

    #[test]
    fn body_only() {
        let term = no_color_terminal(80);
        let block = StatusBlock::new(StatusState::Error).body("Body text");
        let rendered = strip_ansi(&block.render(&term));
        assert_eq!(rendered, "┃ Body text");
    }

    #[test]
    fn with_header() {
        let term = no_color_terminal(80);
        let block = StatusBlock::new(StatusState::Warning)
            .header("<b>Warning</b>")
            .body("Check the config");
        let rendered = strip_ansi(&block.render(&term));
        assert!(rendered.contains("⚠ Warning"));
        assert!(rendered.contains("\n┃ Check the config"));
        assert_eq!(rendered.lines().count(), 2);
    }

    #[test]
    fn with_hint() {
        let term = no_color_terminal(80);
        let block = StatusBlock::new(StatusState::Info)
            .body("Primary message")
            .hint("Try again with `--json`.");
        let rendered = strip_ansi(&block.render(&term));
        assert!(rendered.contains("┃ Primary message"));
        assert!(rendered.ends_with("Try again with `--json`."));
        assert_eq!(rendered.lines().count(), 2);
    }

    #[test]
    fn all_parts() {
        let term = no_color_terminal(80);
        let block = StatusBlock::new(StatusState::Error)
            .header("<b>Shell expansion failed</b>")
            .body("Missing closing brace")
            .hint("Check the template syntax and retry.");
        let rendered = strip_ansi(&block.render(&term));
        let lines: Vec<_> = rendered.lines().collect();
        assert_eq!(lines.len(), 3);
        assert!(lines[0].contains("Shell expansion failed"));
        assert_eq!(lines[1], "┃ Missing closing brace");
        assert_eq!(lines[2], "Check the template syntax and retry.");
    }

    #[test]
    fn error_severity_uses_red500() {
        let block = StatusBlock::new(StatusState::Error);
        assert_eq!(
            block.resolved_border_color(),
            Color::Tailwind(Tailwind::Red500)
        );
    }

    #[test]
    fn warning_severity_colors() {
        let block = StatusBlock::new(StatusState::Warning);
        assert_eq!(block.severity, StatusState::Warning);
        assert_eq!(
            block.resolved_border_color(),
            Color::Tailwind(Tailwind::Orange500)
        );
    }

    #[test]
    fn info_severity_colors() {
        let block = StatusBlock::new(StatusState::Info);
        assert_eq!(block.severity, StatusState::Info);
        assert_eq!(
            block.resolved_border_color(),
            Color::Tailwind(Tailwind::Blue500)
        );
    }

    #[test]
    fn default_color_matches_status_icon() {
        for state in [
            StatusState::Error,
            StatusState::Warning,
            StatusState::Info,
            StatusState::Success,
            StatusState::Active,
            StatusState::ToolUse,
            StatusState::Subagent,
        ] {
            assert_eq!(
                StatusBlock::new(state.clone()).resolved_border_color(),
                state.default_color()
            );
        }
    }

    #[test]
    fn custom_border_color_overrides_severity() {
        let block = StatusBlock::new(StatusState::Warning)
            .border_color(Color::Tailwind(Tailwind::Orange700));
        assert_eq!(
            block.resolved_border_color(),
            Color::Tailwind(Tailwind::Orange700)
        );
    }

    #[test]
    fn custom_border_glyph() {
        let term = no_color_terminal(80);
        let block = StatusBlock::new(StatusState::Error)
            .body("Border override")
            .border("┃ ");
        let rendered = strip_ansi(&block.render(&term));
        assert_eq!(rendered, "┃ Border override");
    }

    #[test]
    fn body_from_plain_string() {
        let term = no_color_terminal(80);
        let block = StatusBlock::new(StatusState::Error).body("plain text");
        let rendered = strip_ansi(&block.render(&term));
        assert!(rendered.contains("┃ plain text"));
    }

    #[test]
    fn body_from_prose() {
        let term = color_terminal(80);
        let block = StatusBlock::new(StatusState::Info).body(Prose::new("<b>bold</b> text"));
        let rendered = block.render(&term);
        assert!(rendered.contains('\x1b'));
        assert!(rendered.contains("bold"));
        assert!(rendered.contains("text"));
    }

    #[test]
    fn body_from_vec_prose() {
        let term = no_color_terminal(80);
        let block = StatusBlock::new(StatusState::Success)
            .body(vec![Prose::new("first"), Prose::new("<b>second</b>")]);
        let rendered = strip_ansi(&block.render(&term));
        assert!(rendered.contains("┃ first"));
        assert!(rendered.contains("┃ second"));
    }

    #[test]
    fn margins_respected() {
        let term = no_color_terminal(32);
        let block = StatusBlock::new(StatusState::Error)
            .body("alpha beta gamma delta epsilon")
            .left_margin(TargetValue::universal(Length::ch(4)))
            .right_margin(TargetValue::universal(Length::ch(10)));
        let rendered = strip_ansi(&block.render(&term));
        let lines: Vec<_> = rendered.lines().collect();
        assert!(lines.len() > 1, "expected wrapped output: {rendered:?}");
        assert!(lines.iter().all(|line| line.starts_with("    ┃ ")));
    }

    #[test]
    fn render_optimistic_matches_render() {
        let width = 80;
        let term = Terminal::new_optimistic(width);
        let block = StatusBlock::new(StatusState::Error)
            .header("<b>Header</b>")
            .body(Prose::new("<blue>Body</blue>"))
            .hint("<dim>Hint</dim>");
        assert_eq!(block.render_optimistic(Some(width)), block.render(&term));
    }

    #[test]
    fn is_block_level() {
        let block = StatusBlock::new(StatusState::Error);
        assert!(block.is_block_level());
    }

    #[test]
    fn clone_preserves_all_fields() {
        let block = StatusBlock::new(StatusState::Warning)
            .header("Header")
            .body("Body")
            .hint("Hint")
            .border_color(Color::Tailwind(Tailwind::Orange700))
            .border("┃ ")
            .left_margin(TargetValue::universal(Length::ch(2)))
            .right_margin(TargetValue::universal(Length::ch(3)));
        let cloned = block.clone();
        assert_eq!(
            cloned.render_optimistic(Some(80)),
            block.render_optimistic(Some(80))
        );
    }

    #[test]
    fn debug_output() {
        let block = StatusBlock::new(StatusState::Error).body("debug me");
        assert!(format!("{block:?}").contains("StatusBlock"));
    }

    #[test]
    fn empty_body_no_block_quote() {
        let term = no_color_terminal(80);
        let block = StatusBlock::new(StatusState::Info)
            .header("Header")
            .hint("Hint only");
        let rendered = strip_ansi(&block.render(&term));
        assert!(rendered.contains("ℹ Header"));
        assert!(rendered.contains("Hint only"));
        assert!(!rendered.contains("┃"));
    }
}
