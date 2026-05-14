//! Formatting utilities for prompt reporting.
//!
//! Provides markdown-to-terminal rendering with blank-line collapsing and
//! styled [`BlockQuote`] construction for system and user prompts.

use biscuit_terminal::components::block_quote::BlockQuote;
use biscuit_terminal::components::renderable::Renderable;
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::color::{Color, Tailwind};
use biscuit_terminal::utils::layout::Margin;
use biscuit_terminal::utils::wrap_policy::WordWrap;

/// The full-block vertical border (U+2588) used by both the system- and
/// user-prompt [`BlockQuote`]s.
///
/// `█` fills the entire cell, giving a thick, continuous vertical stripe
/// that tiles seamlessly across rows (unlike dingbat bars such as `❚`,
/// which leave hairline gaps). It sits in column 0 — directly under the
/// left edge of the 2-cell-wide 📔 / 🗣️ emojis on the header line above.
/// A trailing space separates the border from the body content.
pub(crate) const PROMPT_BORDER: &str = "█ ";

/// The visible cell-width consumed by [`PROMPT_BORDER`] (border glyph plus
/// the trailing space).
pub(crate) const PROMPT_BORDER_WIDTH: u32 = 2;

/// The [`Margin::Chars`] value used as the BlockQuote's `left_margin` for both
/// system- and user-prompt reporting.
///
/// Set to **0** so the border glyph lands at column 0 of the BlockQuote's
/// rendering area — directly under the left edge of the 2-cell `📔` / `🗣️`
/// emoji on the header line above. Adding a leading space would offset the
/// bar one column right of the icon and break visual alignment.
pub(crate) const PROMPT_LEFT_MARGIN: u32 = 0;

/// Total horizontal cells consumed by margin + border before content begins.
pub(crate) const PROMPT_CHROME_WIDTH: u32 = PROMPT_LEFT_MARGIN + PROMPT_BORDER_WIDTH;

/// Compute the content width available inside a prompt-reporting
/// [`BlockQuote`] for the given terminal.
pub(crate) fn prompt_body_width(term: &Terminal) -> u32 {
    term.width().saturating_sub(PROMPT_CHROME_WIDTH).max(1)
}

/// Renders markdown text to terminal output with blank-line collapsing.
///
/// Uses `darkmatter` to convert markdown to ANSI-styled terminal output at
/// `max_width`, then enforces the constraint that no more than one
/// consecutive blank line appears in the result.
///
/// darkmatter delegates terminal capability detection to biscuit-terminal
/// during its internal `Terminal::builder().build()` call, so the
/// [`TerminalOptions::image_mode`](darkmatter::markdown::output::terminal::TerminalOptions::image_mode)
/// is left at its default [`Auto`](darkmatter::markdown::output::terminal::TerminalImageMode::Auto) —
/// horizontal rules emit as Tier 1 Kitty graphics on capable terminals
/// and fall back through Tier 2 (Unicode) and Tier 3 (ASCII) otherwise.
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::terminal::Terminal;
/// use claudine::prompt_reporting::render_markdown_for_terminal;
///
/// let term = Terminal::new();
/// let markdown = "# Hello\n\n\n\nWorld";
/// let output = render_markdown_for_terminal(markdown, &term, 60);
/// // Should never contain more than two consecutive newlines
/// assert!(!output.contains("\n\n\n"));
/// ```
pub fn render_markdown_for_terminal(text: &str, _term: &Terminal, max_width: u32) -> String {
    use darkmatter::markdown::output::terminal::{for_terminal, TerminalOptions};
    use darkmatter::markdown::Markdown;

    if text.trim().is_empty() {
        return String::new();
    }

    let md: Markdown = text.into();
    let mut options = TerminalOptions::default();
    options.max_width = Some(max_width.clamp(1, u16::MAX as u32) as u16);
    let rendered = for_terminal(&md, options).unwrap_or_else(|_| text.to_string());

    // Cap consecutive blank lines at 1. darkmatter's `HorizontalRule`
    // writer emits `<rule>\n\n` and the following markdown paragraph adds
    // its own leading `\n`, so without this cap a rule is followed by two
    // visible blank rows instead of the expected single separator.
    collapse_blank_lines(&rendered, 1)
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
/// The content is rendered from markdown to terminal output at the
/// terminal's available body width (so the wrapping BlockQuote does not
/// have to re-wrap), then placed inside a block quote with the heavy
/// orange left border.
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::components::renderable::Renderable;
/// use biscuit_terminal::terminal::Terminal;
/// use claudine::prompt_reporting::create_system_prompt_blockquote;
///
/// let term = Terminal::new();
/// let quote = create_system_prompt_blockquote("**Bold** text", &term);
/// let rendered = quote.render_optimistic(None);
/// assert!(rendered.contains("Bold"));
/// ```
pub fn create_system_prompt_blockquote(content: &str, term: &Terminal) -> BlockQuote {
    let rendered = render_markdown_for_terminal(content, term, prompt_body_width(term));
    style_system_prompt_blockquote(BlockQuote::from(rendered))
}

/// Wraps an already-rendered (ANSI) string in the orange system-prompt
/// [`BlockQuote`] without re-running it through the Markdown renderer.
///
/// Use this when the content was produced by [`Prose::render`] (for the
/// summary line) and/or by [`render_markdown_for_terminal`] (for the body)
/// and the caller just needs the styled border + margin applied.
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::components::renderable::Renderable;
/// use claudine::prompt_reporting::system_prompt_blockquote_styled;
///
/// let quote = system_prompt_blockquote_styled("pre-rendered content");
/// let rendered = quote.render_optimistic(None);
/// assert!(rendered.contains("pre-rendered content"));
/// ```
pub fn system_prompt_blockquote_styled(rendered_content: &str) -> BlockQuote {
    style_system_prompt_blockquote(BlockQuote::from(rendered_content.to_string()))
}

fn style_system_prompt_blockquote(mut quote: BlockQuote) -> BlockQuote {
    quote = quote
        .with_left_block_color(Color::Tailwind(Tailwind::Orange500))
        .with_border(PROMPT_BORDER);
    quote.layout_mut().left_margin = Margin::Chars(PROMPT_LEFT_MARGIN);
    // Content is already wrapped at `prompt_body_width(term)` by darkmatter,
    // so re-wrapping inside the BlockQuote would just chop trailing ANSI off
    // already-fit lines. `WordWrap::None` preserves the rendered geometry.
    quote.layout_mut().word_wrap = WordWrap::None;
    quote
}

/// Creates a [`BlockQuote`] styled for the user prompt (green border).
///
/// The content is rendered from markdown to terminal output at the
/// terminal's available body width (so the wrapping BlockQuote does not
/// have to re-wrap), then placed inside a block quote with the heavy
/// green left border.
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::components::renderable::Renderable;
/// use biscuit_terminal::terminal::Terminal;
/// use claudine::prompt_reporting::create_user_prompt_blockquote;
///
/// let term = Terminal::new();
/// let quote = create_user_prompt_blockquote("**Bold** text", &term);
/// let rendered = quote.render_optimistic(None);
/// assert!(rendered.contains("Bold"));
/// ```
pub fn create_user_prompt_blockquote(content: &str, term: &Terminal) -> BlockQuote {
    let rendered = render_markdown_for_terminal(content, term, prompt_body_width(term));
    let mut quote = BlockQuote::from(rendered)
        .with_left_block_color(Color::Tailwind(Tailwind::Green500))
        .with_border(PROMPT_BORDER);
    quote.layout_mut().left_margin = Margin::Chars(PROMPT_LEFT_MARGIN);
    // See note in `style_system_prompt_blockquote`.
    quote.layout_mut().word_wrap = WordWrap::None;
    quote
}

#[cfg(test)]
mod tests {
    use super::*;
    use biscuit_terminal::components::renderable::Renderable;

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
        let term = Terminal::new();
        let md = "# Hello\n\nWorld";
        let output = render_markdown_for_terminal(md, &term, 80);
        assert!(output.contains("Hello"));
        assert!(output.contains("World"));
    }

    #[test]
    fn collapses_excessive_blank_lines_in_markdown() {
        let term = Terminal::new();
        let md = "# Hello\n\n\n\n\nWorld";
        let output = render_markdown_for_terminal(md, &term, 80);
        assert!(!output.contains("\n\n\n"));
    }

    #[test]
    fn handles_empty_markdown() {
        let term = Terminal::new();
        let output = render_markdown_for_terminal("", &term, 80);
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
    fn system_blockquote_border_starts_at_column_zero() {
        // left_margin = 0 places the `█` border at column 0 so it lines up
        // directly under the left edge of the 2-cell 📔 emoji on the
        // header line above. A non-zero margin would shift the bar right
        // of the icon and break visual alignment.
        let term = Terminal::new();
        let quote = create_system_prompt_blockquote("Test content", &term);
        let rendered = quote.render_optimistic(None);
        let stripped = strip_ansi(&rendered);
        assert!(stripped.starts_with("█ "), "stripped = {stripped:?}");
        assert!(stripped.contains("Test content"));
    }

    #[test]
    fn user_blockquote_border_starts_at_column_zero() {
        let term = Terminal::new();
        let quote = create_user_prompt_blockquote("Test content", &term);
        let rendered = quote.render_optimistic(None);
        let stripped = strip_ansi(&rendered);
        assert!(stripped.starts_with("█ "), "stripped = {stripped:?}");
        assert!(stripped.contains("Test content"));
    }

    #[test]
    fn system_blockquote_styled_does_not_reprocess_markdown() {
        // The "styled" helper takes pre-rendered ANSI/text and must not
        // pass it through the markdown renderer — bold markers should
        // come through verbatim.
        let quote = system_prompt_blockquote_styled("**unchanged** literal");
        let rendered = quote.render_optimistic(None);
        let stripped = strip_ansi(&rendered);
        assert!(stripped.contains("**unchanged**"));
    }

    #[test]
    fn system_blockquote_renders_markdown() {
        let term = Terminal::new();
        let quote = create_system_prompt_blockquote("**bold** and _italic_", &term);
        let rendered = quote.render_optimistic(None);
        let stripped = strip_ansi(&rendered);
        assert!(stripped.contains("bold"));
        assert!(stripped.contains("italic"));
    }

    #[test]
    fn user_blockquote_renders_markdown() {
        let term = Terminal::new();
        let quote = create_user_prompt_blockquote("**bold** and _italic_", &term);
        let rendered = quote.render_optimistic(None);
        let stripped = strip_ansi(&rendered);
        assert!(stripped.contains("bold"));
        assert!(stripped.contains("italic"));
    }

}
