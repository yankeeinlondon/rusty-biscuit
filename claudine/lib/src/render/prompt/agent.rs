//! The agent (user) prompt render component.
//!
//! Provides header rendering and body rendering (partial/full) inside a
//! green block quote for the user (agent) prompt.

use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::layout::Layout;

use super::formatting::{
    create_user_prompt_blockquote, prompt_body_width, render_markdown_for_terminal,
    user_prompt_blockquote_styled,
};
use super::truncation::{strip_leading_whitespace, truncate_front_back, truncate_head};
use super::{ReportMode, TruncationMode};

/// Render the user-prompt header line.
fn render_user_prompt_header(term: &Terminal) -> String {
    Prose::new("\n<green-500>■ <b>Agent Prompt</b></green-500>").render(term)
}

/// Render the user-prompt body inside a green block quote.
///
/// Leading whitespace is stripped from `text` before rendering. `Summary`
/// returns the empty string; `Partial` truncates per the embedded
/// `TruncationMode`; `Full` renders the complete body.
fn render_user_prompt_body(
    text: &str,
    mode: ReportMode,
    term: &Terminal,
) -> String {
    let stripped = strip_leading_whitespace(text);

    match mode {
        ReportMode::Summary | ReportMode::Silent => String::new(),
        ReportMode::Partial { truncation } => {
            // Render the COMPLETE document before truncating. Slicing Markdown
            // *source* by line can orphan an indented list continuation (or
            // split a fenced block), and the parser then re-reads the orphan as
            // an indented code block — the spurious "text" fences. Rendering
            // first guarantees the parser always sees a syntactically complete
            // document; truncation then only drops already-rendered rows.
            let rendered =
                render_markdown_for_terminal(&stripped, term, prompt_body_width(term));
            let truncated = match truncation {
                TruncationMode::FrontBack => truncate_front_back(&rendered, 20, 10),
                TruncationMode::Truncate => truncate_head(&rendered, 20),
            };
            user_prompt_blockquote_styled(&truncated).render(term)
        }
        ReportMode::Full => create_user_prompt_blockquote(&stripped, term).render(term),
    }
}

/// The user/agent-prompt render component.
///
/// Suppression is decided at construction: [`AgentPrompt::from_mode`] returns
/// `None` for [`ReportMode::Silent`], so a constructed value always produces
/// output. Sink concerns (TTY detection, writer choice) stay with the caller.
#[derive(Debug)]
pub struct AgentPrompt {
    text: String,
    mode: ReportMode,
    layout: Layout,
}

impl AgentPrompt {
    /// Build the component, or `None` when `mode` is [`ReportMode::Silent`].
    pub fn from_mode(text: impl Into<String>, mode: ReportMode) -> Option<Self> {
        if matches!(mode, ReportMode::Silent) {
            return None;
        }

        Some(Self {
            text: text.into(),
            mode,
            layout: Layout::default(),
        })
    }
}

impl TerminalRenderable for AgentPrompt {
    fn render(&self, term: &Terminal) -> String {
        let mut parts = vec![render_user_prompt_header(term)];

        if !matches!(self.mode, ReportMode::Summary) {
            let body = render_user_prompt_body(&self.text, self.mode, term);
            if !body.is_empty() {
                parts.push(body);
            }
        }

        parts.join("\n")
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn layout(&self) -> &Layout {
        &self.layout
    }

    fn layout_mut(&mut self) -> &mut Layout {
        &mut self.layout
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use biscuit_terminal::terminal::Terminal;

    fn test_terminal() -> Terminal {
        Terminal::new()
    }

    /// Strip ANSI escape sequences for test assertions.
    fn strip_ansi_codes(s: &str) -> String {
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

    // --- Header tests ---

    #[test]
    fn header_contains_marker_glyph() {
        let term = test_terminal();
        let header = render_user_prompt_header(&term);
        assert!(header.contains("■"));
    }

    #[test]
    fn header_contains_agent_prompt_label() {
        let term = test_terminal();
        let header = render_user_prompt_header(&term);
        assert!(header.contains("Agent Prompt"));
    }

    // --- Body tests ---

    #[test]
    fn summary_format_returns_empty_body() {
        let term = test_terminal();
        let body = render_user_prompt_body(
            "some content",
            ReportMode::Summary,
            &term,
        );
        assert!(body.is_empty());
    }

    #[test]
    fn full_format_renders_content() {
        let term = test_terminal();
        let body = render_user_prompt_body(
            "Hello world",
            ReportMode::Full,
            &term,
        );
        let plain = strip_ansi_codes(&body);
        assert!(plain.contains("Hello world"));
    }

    #[test]
    fn full_format_strips_leading_whitespace() {
        let term = test_terminal();
        let body = render_user_prompt_body(
            "  Hello\n    World",
            ReportMode::Full,
            &term,
        );
        let plain = strip_ansi_codes(&body);
        assert!(plain.contains("Hello"));
        assert!(plain.contains("World"));
        assert!(
            !plain.contains("  Hello"),
            "leading whitespace should be stripped"
        );
    }

    #[test]
    fn partial_format_truncates_long_text() {
        let text: String = (1..=50)
            .map(|i| format!("- Line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let term = test_terminal();
        let body = render_user_prompt_body(
            &text,
            ReportMode::Partial {
                truncation: TruncationMode::FrontBack,
            },
            &term,
        );
        let plain = strip_ansi_codes(&body);
        assert!(plain.contains("Line 1"));
        // Because of line wrapping, "Line 50" may be split across lines;
        // check for " 50" (with leading space or newline) instead.
        assert!(plain.contains(" 50"), "should contain the last line number");
        // Verify truncation happened by checking that not all lines are present
        assert!(
            !plain.contains("Line 25"),
            "middle lines should be truncated"
        );
    }

    #[test]
    fn partial_format_short_text_unchanged() {
        let text = "Line 1\nLine 2\nLine 3";
        let term = test_terminal();
        let body = render_user_prompt_body(
            text,
            ReportMode::Partial {
                truncation: TruncationMode::FrontBack,
            },
            &term,
        );
        let plain = strip_ansi_codes(&body);
        assert!(plain.contains("Line 1"));
        assert!(plain.contains("Line 2"));
        assert!(plain.contains("Line 3"));
        // Should NOT contain truncation marker for short text
        assert!(!plain.contains("---"));
    }

    #[test]
    fn partial_format_uses_20_10_for_long_text() {
        // Create text with more than 30 lines to trigger truncation with 20/10
        let text: String = (1..=50)
            .map(|i| format!("- Line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let term = test_terminal();
        let body = render_user_prompt_body(
            &text,
            ReportMode::Partial {
                truncation: TruncationMode::FrontBack,
            },
            &term,
        );
        let plain = strip_ansi_codes(&body);
        // The body lines are prefixed with the BlockQuote border (`█ `),
        // and word-wrap may split "Line N" so that "Line " ends one row and
        // the digits start the next. Strip the chrome before scanning for
        // the digit tokens.
        let body_content: String = plain
            .lines()
            .map(|l| l.trim_start_matches(|c: char| c.is_whitespace() || c == '┃'))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(body_content.contains("Line 1"));
        let contains_line_20 =
            body_content.contains("Line 20") || body_content.lines().any(|l| l.starts_with("20"));
        assert!(contains_line_20, "should contain Line 20: {plain:?}");
        let contains_line_50 =
            body_content.contains("Line 50") || body_content.lines().any(|l| l.starts_with("50"));
        assert!(contains_line_50, "should contain Line 50: {plain:?}");
        let contains_line_30 =
            body_content.contains("Line 30") || body_content.lines().any(|l| l.starts_with("30"));
        assert!(
            !contains_line_30,
            "middle lines should be truncated: {plain:?}"
        );
    }

    // --- AgentPrompt direct tests ---

    #[test]
    fn agent_report_silent_returns_none() {
        assert!(AgentPrompt::from_mode("Test prompt", ReportMode::Silent).is_none());
    }

    #[test]
    fn agent_report_summary_renders_header_only() {
        let term = test_terminal();
        let report = AgentPrompt::from_mode("Test prompt", ReportMode::Summary)
            .expect("should produce output");
        let plain = strip_ansi_codes(&report.render(&term));
        assert!(plain.contains("Agent Prompt"));
        assert!(!plain.contains("Test prompt"));
    }

    #[test]
    fn agent_report_full_renders_header_and_body() {
        let term = test_terminal();
        let report = AgentPrompt::from_mode("Full prompt body.", ReportMode::Full)
            .expect("should produce output");
        let plain = strip_ansi_codes(&report.render(&term));
        assert!(plain.contains("Agent Prompt"));
        assert!(plain.contains("Full prompt body"));
    }

    #[test]
    fn agent_report_partial_renders_truncated_body() {
        let text: String = (1..=50)
            .map(|i| format!("- Line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let term = test_terminal();
        let report = AgentPrompt::from_mode(
            text,
            ReportMode::Partial {
                truncation: TruncationMode::FrontBack,
            },
        )
        .expect("should produce output");
        let plain = strip_ansi_codes(&report.render(&term));
        assert!(plain.contains("Agent Prompt"));
        assert!(plain.contains("Line 1"));
        assert!(plain.contains(" 50"), "should contain the last line number");
        assert!(!plain.contains("Line 25"), "middle lines should be truncated");
    }

    #[test]
    fn quiet_flag_does_not_suppress_user_prompt() {
        // Per spec 6.3: `--quiet` is a no-op for the user prompt. The
        // resolved mode is `Full` for short prompts.
        let term = test_terminal();
        let mode = crate::render::resolve_agent_prompt_report_mode(
            false, // silent
            false, // verbose
            10,
        );
        let report = AgentPrompt::from_mode("Test prompt", mode)
            .expect("--quiet should not suppress user prompt");
        let plain = strip_ansi_codes(&report.render(&term));
        assert!(plain.contains("Agent Prompt"));
        assert!(plain.contains("Test prompt"));
    }

    #[test]
    fn short_prompt_renders_full_body() {
        let text = "Line 1\nLine 2\nLine 3";
        let term = test_terminal();
        let report =
            AgentPrompt::from_mode(text, ReportMode::Full).expect("should produce output");
        let plain = strip_ansi_codes(&report.render(&term));
        assert!(plain.contains("Line 1"));
        assert!(plain.contains("Line 2"));
        assert!(plain.contains("Line 3"));
        assert!(!plain.contains("---"), "short text should not be truncated");
    }

    #[test]
    fn user_body_lives_inside_blockquote_at_column_one() {
        // Spec 6.1: the green BlockQuote should sit at column 1 (one-space
        // left margin) so the bar visually centers under the 2-column
        // 🗣️ emoji on the header.
        let term = test_terminal();
        let report = AgentPrompt::from_mode("Body line one", ReportMode::Full)
            .expect("should produce output");
        let plain = strip_ansi_codes(&report.render(&term));
        let mut lines = plain.lines().filter(|l| !l.trim().is_empty());
        let header = lines.next().expect("header line");
        assert!(header.contains("■"), "header line should contain ■");
        let mut saw_quote = false;
        for line in lines {
            if line.trim().is_empty() {
                continue;
            }
            assert!(
                line.starts_with("┃ "),
                "expected BlockQuote prefix on body line, got {line:?}"
            );
            saw_quote = true;
        }
        assert!(saw_quote, "expected at least one BlockQuote-wrapped line");
    }

    #[test]
    fn strips_leading_whitespace_in_report() {
        let text = "    Line 1\n      Line 2\n        Line 3";
        let term = test_terminal();
        let report =
            AgentPrompt::from_mode(text, ReportMode::Full).expect("should produce output");
        let plain = strip_ansi_codes(&report.render(&term));
        assert!(plain.contains("Line 1"));
        assert!(plain.contains("Line 2"));
        assert!(plain.contains("Line 3"));
        assert!(
            !plain.contains("    Line"),
            "leading whitespace should be stripped"
        );
    }
}
