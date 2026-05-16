//! [`TerminalRenderable`] implementation for [`Prose`].

use crate::{components::renderable::TerminalRenderable, terminal::Terminal, utils::layout::{Layout, LayoutTerminalExt}};

use super::prose::Prose;

impl TerminalRenderable for Prose {
    fn render_optimistic(&self, term_width: Option<u32>) -> String {
        let width = term_width.unwrap_or(80);
        let parsed = self.parse_tokens(None);
        self.layout.apply_layout(&parsed, width)
    }

    fn render(&self, term: &Terminal) -> String {
        let width = term.width();
        let parsed = self.parse_tokens(Some(term));
        self.layout.apply_layout(&parsed, width)
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
