//! [`BrowserTreeComponent`] — a [`BrowserRenderable`] adapter for any type
//! that implements [`TreeRenderable`].

use std::any::Any;

use renderable::browser::fragment::{BrowserFragment, Ready};
use renderable::browser::{BrowserRenderable, PageOptions};
use renderable::html::HtmlPage;
use renderable::tree::{
    BrowserRenderOptions, RenderError, RenderStrictness, Rendered, TreeRenderable,
    render_browser_node,
};

/// Adapts any [`TreeRenderable`] into a [`BrowserRenderable`].
///
/// The wrapped value projects itself into the canonical render tree via
/// [`TreeRenderable::render_tree`], and the tree is then rendered for the
/// browser by [`render_browser_node`].
///
/// ## Error Policy
///
/// [`BrowserRenderable::render_html_fragment`] is infallible — it returns a
/// plain `BrowserFragment<Ready>`. The render-tree renderer, by contrast, can
/// fail. To bridge the two, the adapter:
///
/// 1. always renders in a **non-strict** mode (the wrapped `strictness`
///    defaults to [`RenderStrictness::Warn`] and is clamped away from
///    [`RenderStrictness::Strict`] at render time), so lossy or unsupported
///    content degrades to diagnostics rather than an error; and
/// 2. if rendering *still* fails — for example, the projected tree fails
///    structural validation — produces a visible diagnostic fallback fragment
///    instead of panicking.
///
/// This keeps the trait contract intact: rendering a `BrowserTreeComponent`
/// always yields a `BrowserFragment<Ready>`.
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::components::block_quote::BlockQuote;
/// use biscuit_terminal::render_tree::BrowserTreeComponent;
/// use renderable::browser::BrowserRenderable;
///
/// let component = BrowserTreeComponent::new(BlockQuote::from("quoted text"));
/// let fragment = component.render_html_fragment();
/// let html = fragment.render();
/// assert!(html.contains("quoted text"));
/// ```
#[derive(Debug)]
pub struct BrowserTreeComponent<T: TreeRenderable + std::fmt::Debug> {
    /// The wrapped tree-renderable value.
    pub inner: T,
    /// The strictness used when rendering the projected tree.
    ///
    /// [`RenderStrictness::Strict`] is clamped to [`RenderStrictness::Warn`]
    /// at render time so the infallible trait contract holds.
    pub strictness: RenderStrictness,
}

impl<T: TreeRenderable + std::fmt::Debug> BrowserTreeComponent<T> {
    /// Wraps `inner` with [`RenderStrictness::Warn`].
    #[must_use]
    pub fn new(inner: T) -> Self {
        Self {
            inner,
            strictness: RenderStrictness::Warn,
        }
    }

    /// Creates a browser tree component with specific strictness.
    ///
    /// Note that [`RenderStrictness::Strict`] is clamped to
    /// [`RenderStrictness::Warn`] at render time to satisfy the infallible
    /// trait contract.
    #[must_use]
    pub fn with_strictness(inner: T, strictness: RenderStrictness) -> Self {
        Self { inner, strictness }
    }

    /// Renders to HTML with full error information.
    ///
    /// Unlike [`render_html_fragment`](BrowserRenderable::render_html_fragment),
    /// this method returns a [`Result`] that preserves both the rendered output
    /// and any diagnostics collected during rendering.
    ///
    /// ## Default Behavior
    ///
    /// With [`RenderStrictness::Warn`]:
    /// - Structural errors produce a visible fallback fragment with diagnostics
    /// - Non-fatal losses produce diagnostics only
    ///
    /// ## Returns
    ///
    /// A [`Rendered<BrowserFragment<Ready>>`] with the HTML output and
    /// diagnostics.
    ///
    /// ## Errors
    ///
    /// Returns [`RenderError`] if the tree fails structural validation.
    pub fn render_html(&self) -> Result<Rendered<BrowserFragment<Ready>>, RenderError> {
        let tree = self.inner.render_tree();
        let opts = BrowserRenderOptions {
            strictness: self.effective_strictness(),
            ..Default::default()
        };
        render_browser_node(&tree, &opts)
    }

    /// Returns the effective strictness, clamping `Strict` to `Warn`.
    fn effective_strictness(&self) -> RenderStrictness {
        match self.strictness {
            RenderStrictness::Strict => RenderStrictness::Warn,
            other => other,
        }
    }

    /// Renders the projected tree, applying the adapter's error policy.
    fn render_tree_fragment(&self) -> BrowserFragment<Ready> {
        match self.render_html() {
            Ok(rendered) => rendered.output,
            Err(error) => fallback_fragment(&error),
        }
    }
}

impl<T: TreeRenderable + std::fmt::Debug + 'static> BrowserRenderable for BrowserTreeComponent<T> {
    fn render_html_fragment(&self) -> BrowserFragment<Ready> {
        self.render_tree_fragment()
    }

    fn render_html_page(&self, page: Option<PageOptions>) -> HtmlPage {
        let mut html_page = HtmlPage::from(self.render_html_fragment());
        if let Some(options) = page {
            html_page.apply_page_options(options);
        }
        html_page
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Builds a visible fallback fragment for render errors.
fn fallback_fragment(error: &RenderError) -> BrowserFragment<Ready> {
    BrowserFragment::new()
        .define_as_text_fragment(format!("[render-tree error: {error}]"))
        .finalize()
}

#[cfg(test)]
mod tests {
    use super::*;
    use renderable::browser::BrowserRenderable;
    use renderable::tree::RenderNode;

    /// A minimal `TreeRenderable` test type.
    #[derive(Debug)]
    struct Para(String);

    impl TreeRenderable for Para {
        fn render_tree(&self) -> RenderNode {
            RenderNode::root(vec![RenderNode::paragraph(vec![RenderNode::text(
                self.0.clone(),
            )])])
        }
    }

    /// A `TreeRenderable` whose tree is structurally invalid.
    #[derive(Debug)]
    struct Broken;

    impl TreeRenderable for Broken {
        fn render_tree(&self) -> RenderNode {
            // An orphaned TableCell inside a Paragraph fails validation.
            RenderNode::root(vec![RenderNode::paragraph(vec![RenderNode::table_cell(
                vec![RenderNode::text("x")],
            )])])
        }
    }

    /// A `TreeRenderable` with unsupported content.
    #[derive(Debug)]
    struct Unsupported;

    impl TreeRenderable for Unsupported {
        fn render_tree(&self) -> RenderNode {
            RenderNode::root(vec![RenderNode::unsupported("widget")])
        }
    }

    #[test]
    fn renders_valid_component_to_html() {
        let component = BrowserTreeComponent::new(Para("body text".into()));
        let html = component.render_html_fragment().render();
        assert!(html.contains("body text"), "{html}");
        assert!(html.contains("<p>"), "{html}");
    }

    #[test]
    fn render_html_returns_result_with_diagnostics() {
        let component = BrowserTreeComponent::new(Para("test".into()));
        let result = component.render_html();
        assert!(result.is_ok());
        let rendered = result.unwrap();
        assert!(rendered.output.render().contains("test"));
        assert!(rendered.diagnostics.is_empty());
    }

    #[test]
    fn invalid_tree_yields_fallback_fragment() {
        let component = BrowserTreeComponent::new(Broken);
        let html = component.render_html_fragment().render();
        assert!(html.contains("render-tree error"), "{html}");
    }

    #[test]
    fn invalid_tree_render_html_returns_error() {
        let component = BrowserTreeComponent::new(Broken);
        let result = component.render_html();
        assert!(result.is_err());
    }

    #[test]
    fn unsupported_content_degrades_with_warn_diagnostics() {
        let component = BrowserTreeComponent::new(Unsupported);
        let result = component.render_html();
        assert!(result.is_ok());
        let rendered = result.unwrap();
        // Unsupported content produces an HTML comment under Warn strictness.
        let html = rendered.output.render();
        assert!(html.contains("unsupported: widget"), "{html}");
        // Diagnostics are collected.
        assert!(!rendered.diagnostics.is_empty());
    }

    #[test]
    fn strict_is_clamped_to_warn() {
        let mut component = BrowserTreeComponent::new(Unsupported);
        component.strictness = RenderStrictness::Strict;
        // Even with Strict requested, an Unsupported node degrades rather
        // than producing an InvalidTree fallback.
        let html = component.render_html_fragment().render();
        assert!(html.contains("unsupported: widget"), "{html}");
        assert!(!html.contains("render-tree error"), "{html}");
    }

    #[test]
    fn lossy_strictness_suppresses_diagnostics() {
        let component = BrowserTreeComponent::with_strictness(Unsupported, RenderStrictness::Lossy);
        let result = component.render_html();
        assert!(result.is_ok());
        let rendered = result.unwrap();
        // Under Lossy, unsupported content silently disappears.
        let html = rendered.output.render();
        assert!(!html.contains("unsupported"), "{html}");
        // No diagnostics under Lossy.
        assert!(rendered.diagnostics.is_empty());
    }

    #[test]
    fn render_html_page_produces_full_page() {
        let component = BrowserTreeComponent::new(Para("page content".into()));
        let page = component.render_html_page(None);
        let html = page.render().expect("render");
        assert!(html.contains("<html"), "{html}");
        assert!(html.contains("<body>"), "{html}");
        assert!(html.contains("page content"), "{html}");
    }

    #[test]
    fn with_strictness_constructor_works() {
        let component =
            BrowserTreeComponent::with_strictness(Para("test".into()), RenderStrictness::Lossy);
        assert_eq!(component.strictness, RenderStrictness::Lossy);
    }
}
