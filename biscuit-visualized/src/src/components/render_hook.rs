//! The diagram hook `biscuit-terminal`'s render-tree renderer calls for a
//! Mermaid code fence.

use biscuit_terminal::render_tree::{
    DiagramOutcome, DiagramRenderError, DiagramRenderRequest, DiagramRenderer,
};
use renderable::tree::GraphicsMode;

use super::mermaid::MermaidDiagram;

/// Draws Mermaid code fences for `biscuit-terminal`'s render-tree renderer.
///
/// Install it on the render options to turn `mermaid` fences into diagrams;
/// without it they stay code blocks.
///
/// ## Examples
///
/// ```
/// use std::rc::Rc;
///
/// use biscuit_terminal::render_tree::TerminalRenderOptions;
/// use biscuit_terminal::terminal::Terminal;
/// use biscuit_visualized::components::TerminalDiagramRenderer;
/// use renderable::tree::RenderStrictness;
///
/// let options = TerminalRenderOptions::new(&Terminal::new_optimistic(80), RenderStrictness::Warn)
///     .with_diagram_renderer(Rc::new(TerminalDiagramRenderer));
/// assert!(options.diagram_renderer.is_some());
/// ```
#[derive(Debug, Clone, Copy, Default)]
pub struct TerminalDiagramRenderer;

impl DiagramRenderer for TerminalDiagramRenderer {
    fn render_terminal_diagram(
        &self,
        request: &DiagramRenderRequest<'_>,
    ) -> Result<DiagramOutcome, DiagramRenderError> {
        // This hook draws images only, so below the `Rich` ceiling it leaves
        // the fence to the code block.
        if request.graphics_mode != GraphicsMode::Rich {
            return Ok(DiagramOutcome::Declined);
        }
        // Size the diagram to the fence's columns (a list item's or quote's
        // inner width), not the whole terminal.
        let mut terminal = request.terminal.clone();
        terminal.fixed_width = Some(request.available_width.max(1));
        // `try_render` draws the image itself; it never builds a render tree,
        // so the fence cannot be promoted again.
        MermaidDiagram::new(request.source)
            .try_render(&terminal)
            .map(|rendered| DiagramOutcome::Rendered(rendered.output))
            .map_err(DiagramRenderError::new)
    }
}
