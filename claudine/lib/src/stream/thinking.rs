//! Render `SemanticEvent::Reasoning` as a biscuit-terminal `BlockQuote` so
//! it visually anchors as a distinct section. Per spec §"Thinking Prose
//! Rendering": grey vertical line + dim-italic prose, word-wrapped via
//! `Layout`. Section 6 (routed to stderr).

use biscuit_terminal::components::block_quote::BlockQuote;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::Renderable;
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::layout::WordWrap;

/// Render a single thinking chunk as a block-quote string ready to write
/// to stderr. The caller chooses the section and writer.
///
/// Empty input returns an empty string so callers can skip the emit.
pub fn render_thinking_block(text: &str, terminal: &Terminal) -> String {
    if text.trim().is_empty() {
        return String::new();
    }
    // `WordWrap::WrapProse` is required so the Prose component actually
    // breaks long inputs at the BlockQuote's child_width. Without it Prose
    // renders each logical line as a single unwrapped string, leaving the
    // host terminal to soft-wrap the result after the border has already
    // been applied — which makes continuation lines render without the
    // left bar.
    let prose = Prose::new(format!("<dim><i>{text}</i></dim>"))
        .with_word_wrap(WordWrap::WrapProse(None, None));
    BlockQuote::from(prose).with_border("▌ ").render(terminal)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rendered_block_contains_the_input_text() {
        let term = Terminal::builder().build();
        let rendered = render_thinking_block("considering options", &term);
        assert!(
            rendered.contains("considering options"),
            "rendered: {rendered:?}"
        );
    }

    #[test]
    fn empty_input_returns_empty_string() {
        let term = Terminal::builder().build();
        let rendered = render_thinking_block("", &term);
        assert_eq!(rendered, "");
    }

    #[test]
    fn whitespace_only_returns_empty() {
        let term = Terminal::builder().build();
        let rendered = render_thinking_block("   \n  ", &term);
        assert_eq!(rendered, "");
    }

    #[test]
    fn block_quote_uses_wider_border_character() {
        let term = Terminal::builder().build();
        let rendered = render_thinking_block("reflecting", &term);
        assert!(
            rendered.contains("▌"),
            "rendered thinking block must use the wider `▌ ` border to match \
             the system-prompt and agent-prompt visual style; rendered: {rendered:?}"
        );
        assert!(
            !rendered.contains("│"),
            "rendered thinking block must NOT use the default narrow `│` border; \
             rendered: {rendered:?}"
        );
    }

    #[test]
    fn long_prose_wraps_with_border_on_every_line() {
        // Force a narrow terminal so the single input line *must* wrap into
        // multiple output lines. Without the `WordWrap` hint Prose returns
        // one unwrapped line that the host terminal soft-wraps after the
        // border is already applied, and continuation lines visually lose
        // the left bar. This test pins the fix in place: every non-empty
        // rendered line must begin with the `▌` border glyph.
        let term = Terminal::builder().width(40).build();
        let long = "The quick brown fox jumps over the lazy dog and keeps running for several more words to force wrapping at forty columns.";
        let rendered = render_thinking_block(long, &term);
        let lines: Vec<&str> = rendered.lines().filter(|l| !l.trim().is_empty()).collect();
        assert!(
            lines.len() > 1,
            "long input should wrap into multiple lines; rendered: {rendered:?}"
        );
        for line in &lines {
            assert!(
                line.contains('\u{258c}'),
                "every wrapped continuation line must carry the ▌ border; \
                 rendered line: {line:?}; full: {rendered:?}"
            );
        }
    }
}
