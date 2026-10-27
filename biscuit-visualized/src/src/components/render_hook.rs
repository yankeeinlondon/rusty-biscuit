//! The diagram hook `biscuit-terminal`'s render-tree renderer calls for a
//! Mermaid code fence.

use biscuit_terminal::components::terminal_image::TerminalImage;
use biscuit_terminal::render_tree::{
    DiagramOutcome, DiagramRenderError, DiagramRenderRequest, DiagramRenderer,
};
use renderable::tree::{GraphicsMode, TerminalMermaidMode};

use super::fallback_policy::MermaidFallbackPolicy;
use super::mermaid::{MermaidDiagram, text_charset};

/// Draws Mermaid code fences for `biscuit-terminal`'s render-tree renderer.
///
/// Install it on the render options to turn `mermaid` fences into diagrams;
/// without it they stay code blocks. What it draws depends on the mode that
/// offered the fence:
///
/// - `Image` mode: the image, or an error.
/// - `Automatic` mode: the image at `Rich` when the terminal's graphics route
///   has pixels; otherwise the diagram drawn as text (the same nodes as the
///   component's text presentation, partial note included). A fence that
///   Merman does not draw as a diagram, a drawing that does not fit, or a
///   [fallback policy](Self::with_fallback_policy) that forbids text is
///   [declined](DiagramOutcome::Declined), so it renders as its code block
///   with no diagnostic; ZenUML is such a fence. The hook's fit allows for
///   the code block's padding only; biscuit-terminal declines the drawing
///   the same way when the installed code renderer's decoration (a
///   line-number gutter) makes a finished row wider than the fence. A source that does not
///   preprocess (malformed frontmatter or directive) or has no detectable
///   diagram type (a header typo), a parse error, Merman failure, time budget or
///   resource limit, or an image failure is an error, which the render's
///   strictness handles.
///
/// ## Examples
///
/// ```
/// use std::rc::Rc;
///
/// use biscuit_terminal::render_tree::TerminalRenderOptions;
/// use biscuit_terminal::terminal::Terminal;
/// use biscuit_visualized::components::{MermaidFallbackPolicy, TerminalDiagramRenderer, TextFallback};
/// use renderable::tree::RenderStrictness;
///
/// let hook = TerminalDiagramRenderer::default()
///     .with_fallback_policy(MermaidFallbackPolicy::default().with_text_fallback(TextFallback::NoPartial));
/// let options = TerminalRenderOptions::new(&Terminal::new_optimistic(80), RenderStrictness::Warn)
///     .with_diagram_renderer(Rc::new(hook));
/// assert!(options.diagram_renderer.is_some());
/// ```
#[derive(Debug, Clone, Copy, Default)]
pub struct TerminalDiagramRenderer {
    fallback: MermaidFallbackPolicy,
}

impl TerminalDiagramRenderer {
    /// Sets the text fallback policy of `Automatic` mode; settings left unset
    /// resolve on each fence ([`MermaidFallbackPolicy::resolve`]).
    pub fn with_fallback_policy(mut self, policy: MermaidFallbackPolicy) -> Self {
        self.fallback = policy;
        self
    }

    /// The policy set by [`with_fallback_policy`](Self::with_fallback_policy).
    pub fn fallback_policy(&self) -> MermaidFallbackPolicy {
        self.fallback
    }
}

impl DiagramRenderer for TerminalDiagramRenderer {
    fn render_terminal_diagram(
        &self,
        request: &DiagramRenderRequest<'_>,
    ) -> Result<DiagramOutcome, DiagramRenderError> {
        // Size the diagram to the fence's columns (a list item's or quote's
        // inner width), not the whole terminal.
        let mut terminal = request.terminal.clone();
        terminal.fixed_width = Some(request.available_width.max(1));
        let diagram = MermaidDiagram::new(request.source);

        let image = match request.mermaid_mode {
            TerminalMermaidMode::Image => true,
            TerminalMermaidMode::Automatic => TerminalImage::route_has_pixels(&terminal),
            _ => return Ok(DiagramOutcome::Declined),
        };
        if image && request.graphics_mode == GraphicsMode::Rich {
            // `try_render` draws the image itself; it never builds a render
            // tree, so the fence cannot be promoted again.
            return diagram
                .try_render(&terminal)
                .map(|rendered| DiagramOutcome::Rendered(rendered.output))
                .map_err(DiagramRenderError::new);
        }
        // Image mode never answers with text, and draws no image below `Rich`.
        if request.mermaid_mode != TerminalMermaidMode::Automatic {
            return Ok(DiagramOutcome::Declined);
        }
        match diagram.text_nodes(request.available_width, text_charset(&terminal), self.fallback.resolve()) {
            Ok(Some(nodes)) => Ok(DiagramOutcome::Nodes(nodes)),
            Ok(None) => Ok(DiagramOutcome::Declined),
            Err(error) => Err(DiagramRenderError::new(error)),
        }
    }
}
