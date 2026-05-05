use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::Renderable;
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::layout::WordWrap;

/// Render assistant terminal text through `Prose` so wrapping and styling are terminal-aware.
pub(crate) fn render_assistant_text(text: &str, term: &Terminal) -> String {
    Prose::new(text)
        .with_word_wrap(WordWrap::WrapProse(None, None))
        .render(term)
}

/// Render assistant text as full Markdown via darkmatter for rich terminal output.
///
/// Produces syntax-highlighted code blocks, formatted tables, bold/italic styling,
/// etc. Falls back to [`render_assistant_text`] (Prose word-wrap only) on error.
pub(crate) fn render_assistant_markdown(text: &str, term: &Terminal) -> String {
    render_assistant_markdown_with_options(text, term, None)
}

/// Render assistant text as Markdown with pre-built [`TerminalOptions`].
///
/// When `options` is `None`, a fresh default is created (incurs theme detection).
/// Pass a cached instance for hot paths like streaming.
pub(crate) fn render_assistant_markdown_with_options(
    text: &str,
    term: &Terminal,
    options: Option<&darkmatter::markdown::output::terminal::TerminalOptions>,
) -> String {
    use darkmatter::markdown::Markdown;
    use darkmatter::markdown::output::terminal::{TerminalOptions, for_terminal};

    let owned;
    let opts = match options {
        Some(o) => o,
        None => {
            owned = TerminalOptions::default();
            &owned
        }
    };

    let md = Markdown::new(text.trim());
    match for_terminal(&md, opts.clone()) {
        Ok(rendered) => rendered,
        Err(_) => render_assistant_text(text, term),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_assistant_text_wraps_long_terminal_output() {
        let term = Terminal::new_optimistic(24);
        let rendered = render_assistant_text(
            "This is a long assistant sentence that should wrap cleanly in the terminal.",
            &term,
        );
        assert!(rendered.contains('\n'));
    }
}
