use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::layout::{Layout, WordWrap};
use darkmatter::markdown::Markdown;
use darkmatter::markdown::output::terminal::TerminalOptions;

/// The agent's final assistant message, rendered as Markdown for the
/// terminal.
///
/// Renders through darkmatter's Markdown terminal output (syntax-highlighted
/// code blocks, tables, inline styling); when Markdown rendering fails, falls
/// back to `Prose` word-wrap so the message always reaches the user.
///
/// Sink concerns (TTY detection, raw-bytes passthrough, trailing newline,
/// flush) stay with the caller — this component only produces the rendered
/// string.
#[derive(Debug)]
pub struct FinalMessage {
    text: String,
    options: Option<TerminalOptions>,
    layout: Layout,
}

impl FinalMessage {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            options: None,
            layout: Layout::default(),
        }
    }

    /// Render with pre-built [`TerminalOptions`].
    ///
    /// Without this, each render constructs a fresh default (incurring theme
    /// detection). Hot paths that render repeatedly — e.g. streaming — should
    /// build the options once and pass them here.
    pub fn with_options(mut self, options: TerminalOptions) -> Self {
        self.options = Some(options);
        self
    }
}

impl TerminalRenderable for FinalMessage {
    fn render(&self, term: &Terminal) -> String {
        let owned;
        let opts = match &self.options {
            Some(options) => options,
            None => {
                owned = TerminalOptions::default();
                &owned
            }
        };

        let md = Markdown::new(self.text.trim());
        match md.as_terminal(opts.clone()) {
            Ok(rendered) => rendered,
            Err(_) => prose_fallback(&self.text, term),
        }
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

/// Markdown-failure fallback: terminal-aware word wrap with no Markdown
/// semantics.
fn prose_fallback(text: &str, term: &Terminal) -> String {
    Prose::new(text)
        .with_word_wrap(WordWrap::WrapProse(None, None))
        .render(term)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_produces_markdown_terminal_output() {
        let term = Terminal::new_optimistic(80);
        let rendered = FinalMessage::new("plain final answer").render(&term);
        assert!(rendered.contains("plain final answer"));
    }

    #[test]
    fn render_trims_surrounding_whitespace() {
        let term = Terminal::new_optimistic(80);
        let padded = FinalMessage::new("\n\nanswer\n\n").render(&term);
        let bare = FinalMessage::new("answer").render(&term);
        assert_eq!(padded, bare);
    }

    #[test]
    fn with_options_renders_same_content_as_default() {
        let term = Terminal::new_optimistic(80);
        let with_opts = FinalMessage::new("cached-options path")
            .with_options(TerminalOptions::default())
            .render(&term);
        assert!(with_opts.contains("cached-options path"));
    }

    #[test]
    fn prose_fallback_wraps_long_terminal_output() {
        let term = Terminal::new_optimistic(24);
        let rendered = prose_fallback(
            "This is a long assistant sentence that should wrap cleanly in the terminal.",
            &term,
        );
        assert!(rendered.contains('\n'));
    }
}
