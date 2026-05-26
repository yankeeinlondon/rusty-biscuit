//! User prompt reporting for Phase 4.
//!
//! Provides header rendering and body rendering (partial/full) inside a
//! green block quote for the user (agent) prompt.

use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::terminal::Terminal;

use super::{
    PromptReportFormat, TruncationMode, UserPromptReportConfig, create_user_prompt_blockquote,
    strip_leading_whitespace, truncate_front_back,
};

/// Render the user-prompt header line.
pub fn render_user_prompt_header(term: &Terminal) -> String {
    Prose::new("🗣️ <b>Agent Prompt</b>").render(term)
}

/// Render the user-prompt body inside a green block quote.
///
/// Leading whitespace is stripped from `text` before rendering. `Summary`
/// returns the empty string; `PartialPrompt` truncates per `truncation`.
pub fn render_user_prompt_body(
    text: &str,
    format: PromptReportFormat,
    truncation: TruncationMode,
    term: &Terminal,
) -> String {
    let stripped = strip_leading_whitespace(text);

    match format {
        PromptReportFormat::Summary => String::new(),
        PromptReportFormat::PartialPrompt => {
            let truncated = match truncation {
                TruncationMode::FrontBack => truncate_front_back(&stripped, 20, 10),
                TruncationMode::Truncate => {
                    // Show first 20 lines
                    let lines: Vec<&str> = stripped.lines().collect();
                    if lines.len() <= 20 {
                        stripped
                    } else {
                        lines[..20].join("\n")
                    }
                }
            };
            let quote = create_user_prompt_blockquote(&truncated, term);
            quote.render(term)
        }
        PromptReportFormat::FullPrompt => {
            let quote = create_user_prompt_blockquote(&stripped, term);
            quote.render(term)
        }
    }
}

/// Top-level user-prompt reporter. Returns `None` when `config.show_header`
/// is `false`.
pub fn report_user_prompt(
    text: &str,
    config: UserPromptReportConfig,
    term: &Terminal,
) -> Option<String> {
    if !config.show_header {
        return None;
    }

    let mut parts = vec![render_user_prompt_header(term)];

    if config.show_body {
        let body = render_user_prompt_body(text, config.format, config.truncation, term);
        if !body.is_empty() {
            parts.push(body);
        }
    }

    Some(parts.join("\n"))
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
        assert!(header.contains("🗣"));
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
            PromptReportFormat::Summary,
            TruncationMode::FrontBack,
            &term,
        );
        assert!(body.is_empty());
    }

    #[test]
    fn full_format_renders_content() {
        let term = test_terminal();
        let body = render_user_prompt_body(
            "Hello world",
            PromptReportFormat::FullPrompt,
            TruncationMode::FrontBack,
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
            PromptReportFormat::FullPrompt,
            TruncationMode::FrontBack,
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
            .map(|i| format!("Line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let term = test_terminal();
        let body = render_user_prompt_body(
            &text,
            PromptReportFormat::PartialPrompt,
            TruncationMode::FrontBack,
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
            PromptReportFormat::PartialPrompt,
            TruncationMode::FrontBack,
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
            .map(|i| format!("Line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let term = test_terminal();
        let body = render_user_prompt_body(
            &text,
            PromptReportFormat::PartialPrompt,
            TruncationMode::FrontBack,
            &term,
        );
        let plain = strip_ansi_codes(&body);
        // The body lines are prefixed with the BlockQuote border (`█ `),
        // and word-wrap may split "Line N" so that "Line " ends one row and
        // the digits start the next. Strip the chrome before scanning for
        // the digit tokens.
        let body_content: String = plain
            .lines()
            .map(|l| l.trim_start_matches(|c: char| c.is_whitespace() || c == '█'))
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

    // --- Top-level reporter tests ---

    #[test]
    fn silent_config_returns_none() {
        let term = test_terminal();
        let config = UserPromptReportConfig {
            show_header: false,
            show_body: false,
            format: PromptReportFormat::Summary,
            truncation: TruncationMode::FrontBack,
        };
        let result = report_user_prompt("Test prompt", config, &term);
        assert!(result.is_none());
    }

    #[test]
    fn quiet_flag_does_not_suppress_user_prompt() {
        // Per spec 6.3: `--quiet` is a no-op for the user prompt. The
        // resolved config has show_header=true and show_body=true.
        let term = test_terminal();
        let config = crate::prompt_reporting::resolve_user_prompt_report_config(
            false, // silent
            true,  // quiet
            false, // verbose
            10,
        );
        let result = report_user_prompt("Test prompt", config, &term);
        let output = result.expect("--quiet should not suppress user prompt");
        let plain = strip_ansi_codes(&output);
        assert!(plain.contains("Agent Prompt"));
        assert!(plain.contains("Test prompt"));
    }

    #[test]
    fn full_format_renders_header_and_body() {
        let term = test_terminal();
        let config = UserPromptReportConfig {
            show_header: true,
            show_body: true,
            format: PromptReportFormat::FullPrompt,
            truncation: TruncationMode::FrontBack,
        };
        let result = report_user_prompt("Full prompt body.", config, &term);
        let output = result.expect("should produce output");
        let plain = strip_ansi_codes(&output);
        assert!(plain.contains("🗣"));
        assert!(plain.contains("Agent Prompt"));
        assert!(plain.contains("Full prompt body"));
    }

    #[test]
    fn partial_format_renders_truncated_body() {
        let text: String = (1..=50)
            .map(|i| format!("Line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let term = test_terminal();
        let config = UserPromptReportConfig {
            show_header: true,
            show_body: true,
            format: PromptReportFormat::PartialPrompt,
            truncation: TruncationMode::FrontBack,
        };
        let result = report_user_prompt(&text, config, &term);
        let output = result.expect("should produce output");
        let plain = strip_ansi_codes(&output);
        assert!(plain.contains("🗣"));
        assert!(plain.contains("Agent Prompt"));
        assert!(plain.contains("Line 1"));
        // Because of line wrapping, "Line 50" may be split across lines
        assert!(plain.contains(" 50"), "should contain the last line number");
        // Verify truncation happened by checking middle lines are omitted
        assert!(
            !plain.contains("Line 25"),
            "middle lines should be truncated"
        );
    }

    #[test]
    fn short_prompt_renders_full_body() {
        let text = "Line 1\nLine 2\nLine 3";
        let term = test_terminal();
        let config = UserPromptReportConfig {
            show_header: true,
            show_body: true,
            format: PromptReportFormat::FullPrompt,
            truncation: TruncationMode::FrontBack,
        };
        let result = report_user_prompt(text, config, &term);
        let output = result.expect("should produce output");
        let plain = strip_ansi_codes(&output);
        assert!(plain.contains("Line 1"));
        assert!(plain.contains("Line 2"));
        assert!(plain.contains("Line 3"));
        assert!(!plain.contains("---"), "short text should not be truncated");
    }

    #[test]
    fn show_body_false_suppresses_body() {
        let term = test_terminal();
        let config = UserPromptReportConfig {
            show_header: true,
            show_body: false,
            format: PromptReportFormat::FullPrompt,
            truncation: TruncationMode::FrontBack,
        };
        let result = report_user_prompt("This should not appear", config, &term);
        let output = result.expect("should produce output");
        let plain = strip_ansi_codes(&output);
        assert!(plain.contains("Agent Prompt"));
        assert!(!plain.contains("This should not appear"));
    }

    #[test]
    fn user_body_lives_inside_blockquote_at_column_one() {
        // Spec 6.1: the green BlockQuote should sit at column 1 (one-space
        // left margin) so the bar visually centers under the 2-column
        // 🗣️ emoji on the header.
        let term = test_terminal();
        let config = UserPromptReportConfig {
            show_header: true,
            show_body: true,
            format: PromptReportFormat::FullPrompt,
            truncation: TruncationMode::FrontBack,
        };
        let result = report_user_prompt("Body line one", config, &term);
        let output = result.expect("should produce output");
        let plain = strip_ansi_codes(&output);
        let mut lines = plain.lines().filter(|l| !l.trim().is_empty());
        let header = lines.next().expect("header line");
        assert!(header.contains("🗣"), "header line should contain 🗣");
        let mut saw_quote = false;
        for line in lines {
            if line.trim().is_empty() {
                continue;
            }
            assert!(
                line.starts_with("█ "),
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
        let config = UserPromptReportConfig {
            show_header: true,
            show_body: true,
            format: PromptReportFormat::FullPrompt,
            truncation: TruncationMode::FrontBack,
        };
        let result = report_user_prompt(text, config, &term);
        let output = result.expect("should produce output");
        let plain = strip_ansi_codes(&output);
        assert!(plain.contains("Line 1"));
        assert!(plain.contains("Line 2"));
        assert!(plain.contains("Line 3"));
        assert!(
            !plain.contains("    Line"),
            "leading whitespace should be stripped"
        );
    }
}
