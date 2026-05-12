//! Formatting utilities for prompt reporting.
//!
//! Provides markdown-to-terminal rendering with blank-line collapsing and
//! styled [`BlockQuote`] construction for system and user prompts.

use biscuit_terminal::components::block_quote::BlockQuote;
use biscuit_terminal::utils::color::{Color, Tailwind};

/// Renders markdown text to terminal output with blank-line collapsing.
///
/// Uses `darkmatter` to convert markdown to ANSI-styled terminal output,
/// then enforces the constraint that no more than two consecutive blank
/// lines appear in the result.
///
/// ## Examples
///
/// ```
/// use claudine::prompt_reporting::render_markdown_for_terminal;
///
/// let markdown = "# Hello\n\n\n\nWorld";
/// let output = render_markdown_for_terminal(markdown);
/// // Should never contain more than two consecutive newlines
/// assert!(!output.contains("\n\n\n"));
/// ```
pub fn render_markdown_for_terminal(text: &str) -> String {
    use darkmatter::markdown::output::terminal::{TerminalOptions, TerminalImageMode, for_terminal};
    use darkmatter::markdown::Markdown;

    if text.trim().is_empty() {
        return String::new();
    }

    let md: Markdown = text.into();
    let mut options = TerminalOptions::default();
    options.image_mode = TerminalImageMode::Never;
    let rendered = for_terminal(&md, options).unwrap_or_else(|_| text.to_string());

    collapse_blank_lines(&rendered, 2)
}

/// Collapses consecutive blank lines to at most `max_consecutive`.
///
/// A "blank line" is a line that contains only whitespace characters.
///
/// ## Examples
///
/// ```
/// use claudine::prompt_reporting::collapse_blank_lines;
///
/// let text = "A\n\n\n\nB";
/// let result = collapse_blank_lines(text, 2);
/// assert_eq!(result, "A\n\n\nB");
/// ```
pub fn collapse_blank_lines(text: &str, max_consecutive: usize) -> String {
    if max_consecutive == 0 {
        return text
            .lines()
            .filter(|line| !line.trim().is_empty())
            .collect::<Vec<&str>>()
            .join("\n");
    }

    let mut result = String::new();
    let mut blank_streak: usize = 0;

    for line in text.lines() {
        if line.trim().is_empty() {
            blank_streak += 1;
            if blank_streak <= max_consecutive {
                result.push('\n');
            }
        } else {
            blank_streak = 0;
            if !result.is_empty() {
                result.push('\n');
            }
            result.push_str(line);
        }
    }

    result
}

/// Creates a [`BlockQuote`] styled for the system prompt (orange border).
///
/// The content is rendered from markdown to terminal output, then wrapped
/// in a block quote with an orange left border.
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::components::renderable::Renderable;
/// use claudine::prompt_reporting::create_system_prompt_blockquote;
///
/// let quote = create_system_prompt_blockquote("**Bold** text");
/// let rendered = quote.render_optimistic(None);
/// assert!(rendered.contains("Bold"));
/// ```
pub fn create_system_prompt_blockquote(content: &str) -> BlockQuote {
    let rendered = render_markdown_for_terminal(content);
    BlockQuote::from(rendered)
        .with_left_block_color(Color::Tailwind(Tailwind::Orange500))
}

/// Creates a [`BlockQuote`] styled for the user prompt (green border).
///
/// The content is rendered from markdown to terminal output, then wrapped
/// in a block quote with a green left border.
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::components::renderable::Renderable;
/// use claudine::prompt_reporting::create_user_prompt_blockquote;
///
/// let quote = create_user_prompt_blockquote("**Bold** text");
/// let rendered = quote.render_optimistic(None);
/// assert!(rendered.contains("Bold"));
/// ```
pub fn create_user_prompt_blockquote(content: &str) -> BlockQuote {
    let rendered = render_markdown_for_terminal(content);
    BlockQuote::from(rendered)
        .with_left_block_color(Color::Tailwind(Tailwind::Green500))
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- collapse_blank_lines tests ---

    #[test]
    fn no_change_when_under_limit() {
        let text = "A\n\nB";
        assert_eq!(collapse_blank_lines(text, 2), "A\n\nB");
    }

    #[test]
    fn collapses_to_two() {
        let text = "A\n\n\n\nB";
        assert_eq!(collapse_blank_lines(text, 2), "A\n\n\nB");
    }

    #[test]
    fn collapses_many_to_two() {
        let text = "A\n\n\n\n\n\nB";
        assert_eq!(collapse_blank_lines(text, 2), "A\n\n\nB");
    }

    #[test]
    fn collapses_to_one() {
        let text = "A\n\n\nB";
        assert_eq!(collapse_blank_lines(text, 1), "A\n\nB");
    }

    #[test]
    fn collapses_to_zero() {
        let text = "A\n\n\nB";
        assert_eq!(collapse_blank_lines(text, 0), "A\nB");
    }

    #[test]
    fn empty_string_unchanged() {
        assert_eq!(collapse_blank_lines("", 2), "");
    }

    #[test]
    fn only_blank_lines() {
        let text = "\n\n\n";
        assert_eq!(collapse_blank_lines(text, 2), "\n\n");
    }

    #[test]
    fn interleaved_blank_lines() {
        let text = "A\n\n\nB\n\n\n\nC";
        assert_eq!(collapse_blank_lines(text, 2), "A\n\n\nB\n\n\nC");
    }

    #[test]
    fn whitespace_only_counts_as_blank() {
        let text = "A\n   \n\nB";
        assert_eq!(collapse_blank_lines(text, 1), "A\n\nB");
    }

    // --- render_markdown_for_terminal tests ---

    #[test]
    fn renders_simple_markdown() {
        let md = "# Hello\n\nWorld";
        let output = render_markdown_for_terminal(md);
        assert!(output.contains("Hello"));
        assert!(output.contains("World"));
    }

    #[test]
    fn collapses_excessive_blank_lines_in_markdown() {
        let md = "# Hello\n\n\n\n\nWorld";
        let output = render_markdown_for_terminal(md);
        assert!(!output.contains("\n\n\n"));
    }

    #[test]
    fn handles_empty_markdown() {
        let output = render_markdown_for_terminal("");
        assert_eq!(output, "");
    }

    // --- BlockQuote styling tests ---

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

    #[test]
    fn system_blockquote_has_border() {
        let quote = create_system_prompt_blockquote("Test content");
        let rendered = quote.render_optimistic(None);
        let stripped = strip_ansi(&rendered);
        assert!(stripped.starts_with("│ "));
        assert!(stripped.contains("Test content"));
    }

    #[test]
    fn user_blockquote_has_border() {
        let quote = create_user_prompt_blockquote("Test content");
        let rendered = quote.render_optimistic(None);
        let stripped = strip_ansi(&rendered);
        assert!(stripped.starts_with("│ "));
        assert!(stripped.contains("Test content"));
    }

    #[test]
    fn system_blockquote_renders_markdown() {
        let quote = create_system_prompt_blockquote("**bold** and _italic_");
        let rendered = quote.render_optimistic(None);
        let stripped = strip_ansi(&rendered);
        assert!(stripped.contains("bold"));
        assert!(stripped.contains("italic"));
    }

    #[test]
    fn user_blockquote_renders_markdown() {
        let quote = create_user_prompt_blockquote("**bold** and _italic_");
        let rendered = quote.render_optimistic(None);
        let stripped = strip_ansi(&rendered);
        assert!(stripped.contains("bold"));
        assert!(stripped.contains("italic"));
    }
}
