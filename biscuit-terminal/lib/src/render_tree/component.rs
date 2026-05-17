//! [`TreeComponent`] — a [`TerminalRenderable`] adapter for any type that
//! implements [`TreeRenderable`].

use std::any::Any;
use std::rc::Rc;

use renderable::tree::{CodeRenderer, RenderStrictness, TreeRenderable};

use crate::components::renderable::TerminalRenderable;
use crate::terminal::Terminal;
use crate::utils::escape_codes::strip_escape_codes;
use crate::utils::layout::Layout;

use super::options::TerminalRenderOptions;
use super::render::render_terminal_node;

/// Adapts any [`TreeRenderable`] into a [`TerminalRenderable`].
///
/// The wrapped value projects itself into the canonical render tree via
/// [`TreeRenderable::render_tree`], and the tree is then rendered for the
/// terminal by [`render_terminal_node`].
///
/// ## Error policy
///
/// [`TerminalRenderable`] is infallible — its `render` returns a plain
/// `String`. The render-tree renderer, by contrast, can fail. To bridge the
/// two, the adapter:
///
/// 1. always renders in a **non-strict** mode (the wrapped `strictness`
///    defaults to [`RenderStrictness::Warn`] and is clamped away from
///    [`RenderStrictness::Strict`] at render time), so lossy or unsupported
///    content degrades to diagnostics rather than an error; and
/// 2. if rendering *still* fails — for example, the projected tree fails
///    structural validation — produces a visible diagnostic fallback string
///    instead of panicking.
///
/// This keeps the trait contract intact: rendering a `TreeComponent` always
/// yields a `String`.
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::components::block_quote::BlockQuote;
/// use biscuit_terminal::components::renderable::TerminalRenderable;
/// use biscuit_terminal::render_tree::TreeComponent;
///
/// let component = TreeComponent::new(BlockQuote::from("quoted text"));
/// let rendered = component.render_optimistic(Some(80));
/// assert!(rendered.contains("quoted text"));
/// ```
pub struct TreeComponent<T: TreeRenderable + std::fmt::Debug> {
    /// The wrapped tree-renderable value.
    pub inner: T,
    /// The layout applied to this component.
    pub layout: Layout,
    /// The strictness used when rendering the projected tree.
    ///
    /// [`RenderStrictness::Strict`] is clamped to [`RenderStrictness::Warn`]
    /// at render time so the infallible trait contract holds.
    pub strictness: RenderStrictness,
    /// An optional hook for custom code-block rendering.
    ///
    /// When set, it is threaded into the [`TerminalRenderOptions`] used to
    /// render the projected tree.
    pub code_renderer: Option<Rc<dyn CodeRenderer>>,
}

impl<T: TreeRenderable + std::fmt::Debug> std::fmt::Debug for TreeComponent<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // `dyn CodeRenderer` is not `Debug`; report only whether one is set.
        f.debug_struct("TreeComponent")
            .field("inner", &self.inner)
            .field("layout", &self.layout)
            .field("strictness", &self.strictness)
            .field("code_renderer", &self.code_renderer.is_some())
            .finish()
    }
}

impl<T: TreeRenderable + std::fmt::Debug> TreeComponent<T> {
    /// Wraps `inner` with a default layout and [`RenderStrictness::Warn`].
    #[must_use]
    pub fn new(inner: T) -> Self {
        Self {
            inner,
            layout: Layout::default(),
            strictness: RenderStrictness::Warn,
            code_renderer: None,
        }
    }

    /// Sets the optional code-block render hook.
    #[must_use]
    pub fn with_code_renderer(mut self, code_renderer: Rc<dyn CodeRenderer>) -> Self {
        self.code_renderer = Some(code_renderer);
        self
    }

    /// Renders the projected tree, applying the adapter's error policy.
    fn render_tree_for(&self, term: &Terminal) -> String {
        // Clamp Strict away so the renderer never returns an error solely
        // because of strictness; structural validation can still fail.
        let strictness = match self.strictness {
            RenderStrictness::Strict => RenderStrictness::Warn,
            other => other,
        };
        let mut opts = TerminalRenderOptions::new(term, strictness);
        opts.code_renderer = self.code_renderer.clone();
        let node = self.inner.render_tree();
        match render_terminal_node(&node, &opts) {
            Ok(rendered) => rendered.output,
            Err(error) => format!("[render-tree error: {error}]"),
        }
    }
}

impl<T: TreeRenderable + std::fmt::Debug + 'static> TerminalRenderable for TreeComponent<T> {
    fn render(&self, term: &Terminal) -> String {
        self.render_tree_for(term)
    }

    fn render_optimistic(&self, term_width: Option<u32>) -> String {
        let term = Terminal::new_optimistic(term_width.unwrap_or(80));
        self.render_tree_for(&term)
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

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl<T: TreeRenderable + std::fmt::Debug> TreeComponent<T> {
    /// Returns the rendered output with all ANSI escape sequences stripped.
    ///
    /// Useful for tests and for non-terminal sinks.
    #[must_use]
    pub fn render_plain(&self, term: &Terminal) -> String {
        strip_escape_codes(self.render_tree_for(term))
    }
}

#[cfg(test)]
mod render_tree_component_tests {
    use super::*;
    use renderable::tree::RenderNode;

    use crate::components::block_quote::BlockQuote;
    use crate::utils::escape_codes::strip_escape_codes;
    use crate::utils::layout::{Length, TargetValue};

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

    #[test]
    fn render_tree_component_renders_inner() {
        let component = TreeComponent::new(Para("body text".into()));
        let term = Terminal::new_optimistic(80);
        let plain = strip_escape_codes(component.render(&term));
        assert!(plain.contains("body text"));
    }

    #[test]
    fn render_tree_component_wraps_block_quote() {
        let component = TreeComponent::new(BlockQuote::from("a quote"));
        let term = Terminal::new_optimistic(80);
        let out = component.render(&term);
        assert!(out.contains('│'));
        assert!(strip_escape_codes(&out).contains("a quote"));
    }

    #[test]
    fn render_tree_component_layout_accessors() {
        let mut component = TreeComponent::new(Para("x".into()));
        assert_eq!(
            component.layout().margin.left,
            Layout::default().margin.left
        );
        component.layout_mut().margin.left = TargetValue::universal(Length::ch(4));
        assert_eq!(
            component.layout().margin.left,
            TargetValue::universal(Length::ch(4))
        );
    }

    #[test]
    fn render_tree_component_invalid_tree_yields_fallback_string() {
        let component = TreeComponent::new(Broken);
        let term = Terminal::new_optimistic(80);
        // The infallible contract holds: a string, not a panic.
        let out = component.render(&term);
        assert!(out.contains("render-tree error"));
    }

    #[test]
    fn render_tree_component_strict_is_clamped() {
        // Even with Strict requested, an Unsupported node degrades rather
        // than producing an InvalidTree fallback string.
        #[derive(Debug)]
        struct Unsupported;
        impl TreeRenderable for Unsupported {
            fn render_tree(&self) -> RenderNode {
                RenderNode::root(vec![RenderNode::unsupported("widget")])
            }
        }
        let mut component = TreeComponent::new(Unsupported);
        component.strictness = RenderStrictness::Strict;
        let term = Terminal::new_optimistic(80);
        let out = strip_escape_codes(component.render(&term));
        assert!(out.contains("[unsupported: widget]"));
        assert!(!out.contains("render-tree error"));
    }

    #[test]
    fn render_tree_component_render_optimistic_works() {
        let component = TreeComponent::new(Para("optimistic".into()));
        let plain = strip_escape_codes(component.render_optimistic(Some(80)));
        assert!(plain.contains("optimistic"));
    }
}
