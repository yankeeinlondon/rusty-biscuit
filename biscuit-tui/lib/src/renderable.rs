//! Bridge `TerminalRenderable` components into ratatui.
//!
//! [`TuiRenderable`] turns any biscuit-terminal / darkmatter component that
//! implements [`TerminalRenderable`] — `CodeBlock`, `Table`, `Prose`,
//! `UnorderedList`, ... — into owned, styled [`ratatui::text::Text`] laid out
//! to a given column width, so it can be dropped into a ratatui frame:
//!
//! ```ignore
//! use biscuit_tui::TuiRenderable;
//! use darkmatter::markdown::code_block::CodeBlock;
//! use ratatui::widgets::Paragraph;
//!
//! let text = CodeBlock::rust(src).to_tui_text(area.width);
//! frame.render_widget(Paragraph::new(text).scroll((scroll, 0)), area);
//! ```
//!
//! Gated behind the off-by-default `renderables` feature.
//!
//! ## Fidelity tiers
//!
//! This module ships **Tier 0**: the component renders itself to ANSI at the
//! requested width, and that ANSI is parsed into ratatui `Text`. It reuses
//! biscuit-terminal's mature layout and syntax highlighting and covers the
//! whole component catalog with one blanket impl, at the cost of a static
//! (read-only) projection. **Tier 1** (a native `RenderNode` → `Text` fold,
//! grafted behind the unchanged [`TuiRenderable::to_tui_text`] seam) and
//! **Tier 2** (interactive `StatefulWidget`s) are documented as future work in
//! [`docs/tui-renderable.md`](../../docs/tui-renderable.md).

use ansi_to_tui::IntoText;
use biscuit_terminal::components::renderable::TerminalRenderable;
use ratatui::text::Text;

/// Renders a [`TerminalRenderable`] component into owned, styled ratatui
/// [`Text`].
///
/// Implemented for every [`TerminalRenderable`] via a blanket impl, so the
/// whole biscuit-terminal / darkmatter component catalog gains ratatui
/// rendering at once.
pub trait TuiRenderable {
    /// Renders into ratatui [`Text`] laid out to `width` columns.
    ///
    /// The result is a static, fully-styled projection: wrap it in a
    /// [`ratatui::widgets::Paragraph`] for scrolling or word wrap, or render it
    /// directly.
    fn to_tui_text(&self, width: u16) -> Text<'static>;
}

impl<T: TerminalRenderable> TuiRenderable for T {
    fn to_tui_text(&self, width: u16) -> Text<'static> {
        // Tier-1 graft point: once a native `RenderNode` → `Text` fold exists,
        // branch on `self.render_tree_node()` here. `to_tui_text`'s signature is
        // the stable seam, so call sites are unaffected by the upgrade. See
        // `docs/tui-renderable.md`.
        let ansi = self.render_optimistic(Some(u32::from(width)));
        ansi.into_text().unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use biscuit_terminal::prelude::Prose;

    #[test]
    fn projects_terminal_renderable_into_ratatui_text() {
        let text = Prose::new("hello world").to_tui_text(40);
        let rendered: String = text
            .lines
            .iter()
            .flat_map(|line| line.spans.iter())
            .map(|span| span.content.as_ref())
            .collect();
        assert!(rendered.contains("hello world"), "got: {rendered:?}");
    }

    #[test]
    fn zero_width_degrades_without_panicking() {
        // A nonsensical width must never panic the host TUI's draw loop.
        let _ = Prose::new("x").to_tui_text(0);
    }
}
