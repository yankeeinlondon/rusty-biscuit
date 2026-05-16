use std::collections::HashMap;
use std::marker::PhantomData;

use crate::{
    browser::{BrowserRenderable, ComponentStylesheet, feature::PageFeature},
    html::tag::{BlockTag, HtmlAttribute, HtmlBlockTag, HtmlVoidTag, VoidTag, link::LinkTag},
    microdata::MicrodataKey,
};

// ---------------------------------------------------------------------------
// State markers
// ---------------------------------------------------------------------------

mod sealed {
    /// Sealed bound preventing downstream crates from implementing
    /// `FragmentState` or `Refine` against their own types.
    pub trait Sealed {}
}

/// Marker trait identifying the workflow state of a [`BrowserFragment`].
///
/// Implemented only by the in-crate state types: [`Shape`], [`RefineVoid`],
/// [`RefineBlock`], [`RefineText`], and [`Ready`].
pub trait FragmentState: sealed::Sealed {}

/// Sub-marker for the three "refine" states ([`RefineVoid`], [`RefineBlock`],
/// [`RefineText`]). Cross-cutting builders (stylesheet, features, metadata,
/// dependency links) are gated by this trait so they're available exactly
/// where the fragment is mid-construction.
pub trait Refine: FragmentState + sealed::Sealed {}

/// Initial state. No `HtmlNode` has been chosen yet. Only `define_as_*`
/// builders are exposed.
pub struct Shape;
/// A `VoidTag` has been chosen. Attributes can be added; children cannot.
pub struct RefineVoid;
/// A `BlockTag` has been chosen. Attributes and children can be added.
pub struct RefineBlock;
/// A `TextFragment` has been chosen. No attributes, no children — only
/// cross-cutting builders apply before `finalize()`.
pub struct RefineText;
/// Fragment is fully composed. `render()` and `validate_render_content()`
/// live here.
pub struct Ready;

impl sealed::Sealed for Shape {}
impl sealed::Sealed for RefineVoid {}
impl sealed::Sealed for RefineBlock {}
impl sealed::Sealed for RefineText {}
impl sealed::Sealed for Ready {}

impl FragmentState for Shape {}
impl FragmentState for RefineVoid {}
impl FragmentState for RefineBlock {}
impl FragmentState for RefineText {}
impl FragmentState for Ready {}

impl Refine for RefineVoid {}
impl Refine for RefineBlock {}
impl Refine for RefineText {}

// ---------------------------------------------------------------------------
// ComposableNode
// ---------------------------------------------------------------------------

/// A composable node is an extension of `HtmlNode` that allows raw HTML
/// structures as well as other `BrowserRenderable` components to be composed
/// as children of a component.
#[allow(private_interfaces)]
pub enum ComposableNode {
    BlockTag(HtmlBlockTag),
    VoidTag(HtmlVoidTag),
    TextFragment(String),
    Component(Box<dyn BrowserRenderable>),
}

// ---------------------------------------------------------------------------
// BrowserFragment
// ---------------------------------------------------------------------------

/// The key output of a component which represents an HTML fragment along
/// with page-level attributes the fragment expects the page to honor when
/// it's rendered.
///
/// The type parameter `S` tracks the construction workflow at compile time:
///
/// ```text
/// Shape ──define_as_block_tag──▶ RefineBlock ─┐
///       ──define_as_void_tag──▶ RefineVoid ───┼── finalize() ──▶ Ready
///       ──define_as_text_fragment──▶ RefineText ─┘
/// ```
pub struct BrowserFragment<S: FragmentState = Shape> {
    node: Option<ComposableNode>,
    stylesheet: Option<ComponentStylesheet>,
    features: Vec<PageFeature>,
    metadata: HashMap<MicrodataKey, String>,
    pub dependency_links: Vec<LinkTag>,
    _state: PhantomData<S>,
}

impl Default for BrowserFragment<Shape> {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Shape — entry point
// ---------------------------------------------------------------------------

impl BrowserFragment<Shape> {
    /// Construct an empty fragment in the [`Shape`] state.
    pub fn new() -> Self {
        BrowserFragment {
            node: None,
            stylesheet: None,
            features: Vec::new(),
            metadata: HashMap::new(),
            dependency_links: Vec::new(),
            _state: PhantomData,
        }
    }

    /// Commit the fragment to a block-tag shape, scoped under `base_class`.
    /// Transitions to [`RefineBlock`].
    pub fn define_as_block_tag(
        self,
        tag: BlockTag,
        base_class: impl Into<String>,
    ) -> BrowserFragment<RefineBlock> {
        let block = HtmlBlockTag::new(tag, base_class.into());
        self.into_state(Some(ComposableNode::BlockTag(block)))
    }

    /// Commit the fragment to a void-tag shape. Transitions to [`RefineVoid`].
    pub fn define_as_void_tag(self, tag: VoidTag) -> BrowserFragment<RefineVoid> {
        let void = HtmlVoidTag::new(tag);
        self.into_state(Some(ComposableNode::VoidTag(void)))
    }

    /// Commit the fragment to a text-fragment shape. Transitions to
    /// [`RefineText`].
    pub fn define_as_text_fragment(self, text: impl Into<String>) -> BrowserFragment<RefineText> {
        self.into_state(Some(ComposableNode::TextFragment(text.into())))
    }
}

// ---------------------------------------------------------------------------
// Shared cross-cutting builders — Refine states only
// ---------------------------------------------------------------------------

impl<S: Refine> BrowserFragment<S> {
    /// Attach (or replace) the component's stylesheet.
    pub fn with_stylesheet(mut self, stylesheet: ComponentStylesheet) -> Self {
        self.stylesheet = Some(stylesheet);
        self
    }

    /// Declare a page-level feature this fragment depends on. The page
    /// rolls up JS/CSS/meta requirements from its fragments.
    pub fn add_feature(mut self, feature: PageFeature) -> Self {
        self.features.push(feature);
        self
    }

    /// Set a microdata key/value pair this fragment would like the page to
    /// honor. Page-level values override component-level values.
    pub fn add_metadata_keypair(mut self, key: MicrodataKey, value: impl Into<String>) -> Self {
        self.metadata.insert(key, value.into());
        self
    }

    /// Declare a `<link>` dependency the page should include in `<head>`.
    pub fn add_linked_dependency(mut self, link: LinkTag) -> Self {
        self.dependency_links.push(link);
        self
    }
}

// ---------------------------------------------------------------------------
// RefineVoid
// ---------------------------------------------------------------------------

impl BrowserFragment<RefineVoid> {
    /// Add an attribute to the void tag.
    pub fn add_attribute(mut self, attr: HtmlAttribute) -> Self {
        if let Some(ComposableNode::VoidTag(void)) = self.node.as_mut() {
            void.attributes.push(attr);
        }
        self
    }

    /// Close out the build phase. Transitions to [`Ready`].
    pub fn finalize(self) -> BrowserFragment<Ready> {
        self.into_state_preserving_node()
    }
}

// ---------------------------------------------------------------------------
// RefineBlock
// ---------------------------------------------------------------------------

impl BrowserFragment<RefineBlock> {
    /// Add an attribute to the block tag.
    pub fn add_attribute(mut self, attr: HtmlAttribute) -> Self {
        if let Some(ComposableNode::BlockTag(block)) = self.node.as_mut() {
            block.attributes.push(attr);
        }
        self
    }

    /// Append a child node. Children may themselves be other components via
    /// [`ComposableNode::Component`].
    pub fn add_child(mut self, child: ComposableNode) -> Self {
        if let Some(ComposableNode::BlockTag(block)) = self.node.as_mut() {
            block.content.children.push(child);
        }
        self
    }

    /// Close out the build phase. Transitions to [`Ready`].
    pub fn finalize(self) -> BrowserFragment<Ready> {
        self.into_state_preserving_node()
    }
}

// ---------------------------------------------------------------------------
// RefineText
// ---------------------------------------------------------------------------

impl BrowserFragment<RefineText> {
    /// Close out the build phase. Transitions to [`Ready`].
    pub fn finalize(self) -> BrowserFragment<Ready> {
        self.into_state_preserving_node()
    }
}

// ---------------------------------------------------------------------------
// Ready — terminal state
// ---------------------------------------------------------------------------

impl BrowserFragment<Ready> {
    /// Render the fragment as an HTML string.
    pub fn render(self) -> String {
        todo!()
    }

    /// Validate that `render()` would produce well-formed HTML:
    ///
    /// 1. the top-level node is present and is a valid tag,
    /// 2. attributes are well-formed for the chosen tag,
    /// 3. all descendant fragments are themselves valid.
    pub fn validate_render_content(&self) -> bool {
        todo!()
    }
}

// ---------------------------------------------------------------------------
// State-transition plumbing
// ---------------------------------------------------------------------------

impl<S: FragmentState> BrowserFragment<S> {
    /// Move all cross-cutting fields into a new state, overriding `node`.
    fn into_state<T: FragmentState>(self, node: Option<ComposableNode>) -> BrowserFragment<T> {
        BrowserFragment {
            node,
            stylesheet: self.stylesheet,
            features: self.features,
            metadata: self.metadata,
            dependency_links: self.dependency_links,
            _state: PhantomData,
        }
    }

    /// Move all fields (including `node`) into a new state. Used by
    /// `finalize()` after the node has already been chosen.
    fn into_state_preserving_node<T: FragmentState>(self) -> BrowserFragment<T> {
        BrowserFragment {
            node: self.node,
            stylesheet: self.stylesheet,
            features: self.features,
            metadata: self.metadata,
            dependency_links: self.dependency_links,
            _state: PhantomData,
        }
    }
}
