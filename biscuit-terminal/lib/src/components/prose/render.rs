//! [`TerminalRenderable`] implementation for [`Prose`].

use crate::{
    components::renderable::TerminalRenderable,
    render_tree::TerminalRenderOptions,
    render_tree::render::render_terminal_node_keeping_unbreakable,
    terminal::Terminal,
    utils::block_constraint::{sanitize_wrapped_lines, strip_unbreakable_markers, wrap_lines},
    utils::layout::{Layout, WordWrap},
};
use renderable::tree::{RenderStrictness, TreeRenderable};

use super::prose::Prose;

impl TerminalRenderable for Prose {
    fn render_optimistic(&self, term_width: Option<u32>) -> String {
        let term = Terminal::new_optimistic(term_width.unwrap_or(80));
        self.render_via_tree(&term)
    }

    fn render(&self, term: &Terminal) -> String {
        self.render_via_tree(term)
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

    fn is_block_level(&self) -> bool {
        true
    }
}

impl Prose {
    /// Renders this `Prose` through the canonical render tree.
    fn render_via_tree(&self, term: &Terminal) -> String {
        let node = <Self as TreeRenderable>::render_tree(self);
        let mut opts = TerminalRenderOptions::new(term, RenderStrictness::Warn);
        opts.context.verbosity = self.verbosity;
        // Printed URLs keep their unbreakable markers through the wrap below;
        // they are removed once it has run.
        let output = match render_terminal_node_keeping_unbreakable(&node, &opts) {
            Ok(rendered) => rendered.output,
            Err(error) => {
                tracing::error!(
                    component = "Prose",
                    error = %error,
                    "render_terminal_node failed; emitting empty output"
                );
                return String::new();
            }
        };

        // Apply word wrap from the layout when configured. The tree renderer
        // handles margins and alignment but does not fold lines; post-process
        // wrapping restores the behavior Prose provided on the bespoke path.
        let output = match (&self.layout.word_wrap, term.columns()) {
            (WordWrap::None, _) | (_, None) => output,
            (strategy, Some(width)) => {
                // `split`, not `lines`: trailing empty rows are the bottom
                // margin and must survive the wrap pass.
                let lines = output.split('\n').map(String::from).collect();
                let wrapped = wrap_lines(lines, strategy, width);
                let sanitized = sanitize_wrapped_lines(wrapped);
                sanitized.join("\n")
            }
        };
        strip_unbreakable_markers(&output)
    }
}
