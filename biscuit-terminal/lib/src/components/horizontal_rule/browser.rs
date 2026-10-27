use renderable::browser::fragment::{BrowserFragment, Ready};

use crate::components::renderable::BrowserRenderable;
use crate::render_tree::BrowserTreeComponent;

use super::HorizontalRule;

impl BrowserRenderable for HorizontalRule {
    /// Renders the rule through the shared render tree: the output is the
    /// tree's `ThematicBreak` rendering, so geometry, alignment-driven
    /// margins, outer layout, and `width`/`color` validation are identical to
    /// any other `ThematicBreak` reaching the browser.
    ///
    /// Under the adapter's default rich graphics mode this is an inline SVG.
    /// Callers wanting a plain `<hr>` with `data-hr-*` attributes can render
    /// [`TreeRenderable::render_tree`] with `GraphicsMode::Off` instead.
    ///
    /// [`TreeRenderable::render_tree`]: renderable::tree::TreeRenderable::render_tree
    fn render_html_fragment(&self) -> BrowserFragment<Ready> {
        BrowserTreeComponent::new(self.clone()).render_html_fragment()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
