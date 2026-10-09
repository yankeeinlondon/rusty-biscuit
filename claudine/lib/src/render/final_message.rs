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
/// The Markdown render is pinned to the *supplied* terminal's width: when the
/// [`TerminalOptions`] carry no explicit `max_width`, `render` fills it from
/// `term.width()` (which honors `fixed_width`). Without that, darkmatter
/// re-detects the host terminal's width per render and a deliberately
/// narrowed terminal — e.g. the sequence gutter inset — would reach only the
/// plain-text fallback, not the rendered Markdown.
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
    /// build the options once and pass them here. An unset `max_width` is
    /// still filled from the render-time terminal; only an explicit pin wins.
    pub fn with_options(mut self, options: TerminalOptions) -> Self {
        self.options = Some(options);
        self
    }
}

impl TerminalRenderable for FinalMessage {
    fn render(&self, term: &Terminal) -> String {
        let mut opts = match &self.options {
            Some(options) => options.clone(),
            None => TerminalOptions::default(),
        };
        if opts.max_width.is_none() {
            opts.max_width = Some(term.width().clamp(1, u32::from(u16::MAX)) as u16);
        }

        let md = Markdown::new(self.text.trim());
        match md.as_terminal(opts) {
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

    /// The rendered line lengths of `text` rendered on `term`, escape codes
    /// stripped.
    fn visual_line_widths(text: &str, term: &Terminal) -> Vec<usize> {
        let rendered = FinalMessage::new(text).render(term);
        biscuit_terminal::prelude::strip_escape_codes(&rendered)
            .lines()
            .map(|line| line.chars().count())
            .collect()
    }

    /// R7: rendered Markdown prose must wrap to the *supplied* terminal, not
    /// to independently detected defaults — a sequence task's gutter inset
    /// narrows the terminal, and detection would silently discard that.
    /// Width 40 sits far below any detected default (80+), so a render that
    /// ignored the supplied terminal overflows it.
    #[test]
    fn markdown_prose_honors_the_supplied_terminal_width() {
        let term = Terminal::new_optimistic(40);
        let widths = visual_line_widths(
            "The quick brown fox jumps over the lazy dog while the eager cat \
             watches carefully from the windowsill above.",
            &term,
        );
        assert!(
            widths.len() > 1,
            "long prose must wrap at the supplied width, got {widths:?}"
        );
        assert!(
            widths.iter().all(|width| *width <= 40),
            "every rendered line must fit the supplied 40-column terminal, got {widths:?}"
        );
    }

    /// The code-block twin: the fenced panel's rows are laid out to the
    /// supplied width (a short line is padded to it), where the pre-fix
    /// render padded to the detected default (80 in a test process).
    #[test]
    fn markdown_code_block_honors_the_supplied_terminal_width() {
        let term = Terminal::new_optimistic(40);
        let widths = visual_line_widths("```rust\nlet x = 1;\n```", &term);
        assert!(
            widths.iter().all(|width| *width <= 40),
            "the code panel must be laid out to the supplied 40-column terminal, got {widths:?}"
        );
    }

    /// The sequence inset saturates at one column for a terminal narrower
    /// than the gutter (`new_assistant_stream_inset`), so rendering must
    /// survive a one-column terminal: no panic, no lost content.
    #[test]
    fn a_terminal_narrower_than_the_inset_still_renders() {
        let term = Terminal::new_optimistic(1);
        let rendered = FinalMessage::new("narrow but present").render(&term);
        let stripped = biscuit_terminal::prelude::strip_escape_codes(&rendered);
        let visible: String = stripped.chars().filter(|ch| !ch.is_whitespace()).collect();
        assert_eq!(visible, "narrowbutpresent");
    }

    /// Cached options built without an explicit `max_width` (the streaming
    /// hot path's shape) must honor the supplied terminal exactly like the
    /// default-options path, so the streaming and final renders agree.
    #[test]
    fn cached_options_without_max_width_honor_the_supplied_terminal() {
        let term = Terminal::new_optimistic(40);
        let rendered = FinalMessage::new(
            "The quick brown fox jumps over the lazy dog while the eager cat \
             watches carefully from the windowsill above.",
        )
        .with_options(TerminalOptions::default())
        .render(&term);
        let widths: Vec<usize> = biscuit_terminal::prelude::strip_escape_codes(&rendered)
            .lines()
            .map(|line| line.chars().count())
            .collect();
        assert!(
            widths.iter().all(|width| *width <= 40),
            "cached options must not re-open detection, got {widths:?}"
        );
    }

    /// A caller that pinned `max_width` explicitly keeps that pin; the
    /// supplied terminal only fills the unset case.
    #[test]
    fn an_explicit_max_width_wins_over_the_supplied_terminal() {
        let term = Terminal::new_optimistic(40);
        let mut options = TerminalOptions::default();
        options.max_width = Some(30);
        let rendered = FinalMessage::new(
            "The quick brown fox jumps over the lazy dog while the eager cat \
             watches carefully from the windowsill above.",
        )
        .with_options(options)
        .render(&term);
        let widths: Vec<usize> = biscuit_terminal::prelude::strip_escape_codes(&rendered)
            .lines()
            .map(|line| line.chars().count())
            .collect();
        assert!(
            widths.iter().all(|width| *width <= 30),
            "an explicit max_width must stay authoritative, got {widths:?}"
        );
    }
}

