//! Render `SemanticEvent::Reasoning` as a biscuit-terminal `BlockQuote` so
//! it visually anchors as a distinct section. Per spec §"Thinking Prose
//! Rendering": grey vertical line + dim-italic prose, word-wrapped via
//! `Layout`. Section 6 (routed to stderr).

use biscuit_terminal::components::block_quote::BlockQuote;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::Renderable;
use biscuit_terminal::terminal::Terminal;

/// Render a single thinking chunk as a block-quote string ready to write
/// to stderr. The caller chooses the section and writer.
///
/// Empty input returns an empty string so callers can skip the emit.
pub fn render_thinking_block(text: &str, terminal: &Terminal) -> String {
    if text.trim().is_empty() {
        return String::new();
    }
    let prose = Prose::new(format!("<dim><i>{text}</i></dim>"));
    BlockQuote::from(prose).render(terminal)
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
}
