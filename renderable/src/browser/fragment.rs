use std::collections::HashMap;
use std::marker::PhantomData;

use crate::{
    browser::{ComponentStylesheet, feature::PageFeature},
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

/// A composable node extends `HtmlNode` with two extra shapes: caller-owned
/// raw HTML, and nested components.
///
/// Per decisions.md item 1, the [`Component`](ComposableNode::Component)
/// variant holds an eager [`BrowserFragment<Ready>`] — composition is
/// structural, not a boxed trait object. This makes the node tree
/// homogeneous: every nested component is already "done", so page-level
/// aggregation is a pure recursive walk with no trait calls.
#[allow(private_interfaces)]
pub enum ComposableNode {
    /// A non-void element with attributes and children.
    BlockTag(HtmlBlockTag),
    /// A void element with attributes and no children.
    VoidTag(HtmlVoidTag),
    /// A run of literal text. **Escaped on emit** by the renderer.
    TextFragment(String),
    /// Caller-owned prebuilt HTML (SVG, third-party markup). **Never
    /// escaped** by the renderer — the caller owns correctness.
    RawHtml(String),
    /// A nested, fully-rendered component fragment.
    ///
    /// Boxed because `BrowserFragment` itself stores a `ComposableNode`;
    /// the indirection breaks the otherwise-infinite type recursion.
    Component(Box<BrowserFragment<Ready>>),
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
/// Shape ──define_as_block_tag──▶ RefineBlock ───┐
///       ──define_as_void_tag──▶ RefineVoid ─────┼── finalize() ──▶ Ready
///       ──define_as_text_fragment──▶ RefineText ┤
///       ──define_as_raw_html──▶ RefineText ─────┘
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

    /// Commit the fragment to a raw-HTML shape. The string is caller-owned
    /// and **never escaped** by the renderer. Transitions to [`RefineText`]
    /// so the cross-cutting builders still apply.
    ///
    /// Use this only for final, already-escaped markup — SVG, third-party
    /// HTML. For literal text content use
    /// [`define_as_text_fragment`](BrowserFragment::define_as_text_fragment),
    /// which is escaped on emit.
    pub fn define_as_raw_html(self, html: impl Into<String>) -> BrowserFragment<RefineText> {
        self.into_state(Some(ComposableNode::RawHtml(html.into())))
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

    /// Append a fully-rendered child component fragment.
    ///
    /// Convenience over `add_child(ComposableNode::Component(child))` —
    /// the recursion point that lets components compose other components.
    pub fn add_component(self, child: BrowserFragment<Ready>) -> Self {
        self.add_child(ComposableNode::Component(Box::new(child)))
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
    ///
    /// `TextFragment` content is HTML-escaped; `RawHtml` content is
    /// emitted verbatim (caller-owned). Nested `Component` fragments
    /// recurse.
    pub fn render(&self) -> String {
        match &self.node {
            Some(node) => render_node(node),
            None => String::new(),
        }
    }

    /// Returns `true` when `render()` would produce well-formed HTML:
    /// the top-level node is present, and every descendant fragment is
    /// itself valid.
    pub fn validate_render_content(&self) -> bool {
        match &self.node {
            None => false,
            Some(node) => validate_node(node),
        }
    }
}

/// Recursively renders a single composable node to HTML.
fn render_node(node: &ComposableNode) -> String {
    match node {
        ComposableNode::TextFragment(text) => crate::browser::utils::escape_text(text).into_owned(),
        ComposableNode::RawHtml(html) => html.clone(),
        ComposableNode::Component(fragment) => fragment.render(),
        ComposableNode::VoidTag(void) => {
            format!(
                "<{}{}>",
                void.tag.name(),
                render_attributes(&void.attributes, None)
            )
        }
        ComposableNode::BlockTag(block) => {
            let name = block.tag.name();
            let children: String = block.content.children.iter().map(render_node).collect();
            format!(
                "<{name}{}>{children}</{name}>",
                render_attributes(&block.attributes, Some(&block.base_class))
            )
        }
    }
}

/// Recursively validates a composable node.
fn validate_node(node: &ComposableNode) -> bool {
    match node {
        ComposableNode::TextFragment(_) | ComposableNode::RawHtml(_) => true,
        ComposableNode::VoidTag(_) => true,
        ComposableNode::Component(fragment) => fragment.validate_render_content(),
        ComposableNode::BlockTag(block) => block.content.children.iter().all(validate_node),
    }
}

/// Serializes a slice of attributes into an opening-tag attribute string
/// (leading space included when non-empty). Attribute values are escaped.
///
/// `extra_class` supplies a component wrapper class (the block's
/// `base_class`) which is merged ahead of any explicit
/// [`HtmlAttribute::Class`] values into a single `class="..."` attribute.
/// Pass `None` for void tags, which have no `base_class`.
fn render_attributes(attributes: &[HtmlAttribute], extra_class: Option<&str>) -> String {
    use crate::browser::utils::escape_attribute;

    /// Appends a leading-space `key="value"` pair with the value escaped.
    fn push_pair(out: &mut String, key: &str, value: &str) {
        out.push_str(&format!(r#" {key}="{}""#, escape_attribute(value)));
    }
    /// Appends a leading-space bare boolean attribute when `present`.
    fn push_bool(out: &mut String, name: &str, present: bool) {
        if present {
            out.push(' ');
            out.push_str(name);
        }
    }

    let mut out = String::new();

    // Merge the component wrapper class with every explicit `Class`
    // attribute into a single `class="..."` (base class first).
    let mut class_parts: Vec<Option<&str>> = vec![extra_class];
    for attr in attributes {
        if let HtmlAttribute::Class(class) = attr {
            class_parts.push(Some(class.as_str()));
        }
    }
    let merged_class = crate::browser::utils::classes(class_parts);
    if !merged_class.is_empty() {
        push_pair(&mut out, "class", &merged_class);
    }

    for attr in attributes {
        match attr {
            // Handled above as part of the merged class attribute.
            HtmlAttribute::Class(_) => {}
            HtmlAttribute::Id(id) => push_pair(&mut out, "id", id.as_str()),
            HtmlAttribute::Style(style) => {
                let value = style.to_css().replace('\n', " ");
                push_pair(&mut out, "style", &value);
            }
            HtmlAttribute::Title(value) => push_pair(&mut out, "title", value),
            HtmlAttribute::Data(name, value) => {
                push_pair(&mut out, &format!("data-{}", name.as_str()), value);
            }
            HtmlAttribute::Hidden(present) => push_bool(&mut out, "hidden", *present),
            HtmlAttribute::Role(role) => push_pair(&mut out, "role", role.as_str()),
            HtmlAttribute::Aria(aria) => out.push_str(&aria.render()),
            HtmlAttribute::Href(url) => push_pair(&mut out, "href", url.as_str()),
            HtmlAttribute::Src(url) => push_pair(&mut out, "src", url.as_str()),
            HtmlAttribute::Alt(value) => push_pair(&mut out, "alt", value),
            HtmlAttribute::Type(html_type) => {
                push_pair(&mut out, "type", &html_type.as_value());
            }
            HtmlAttribute::Name(value) => push_pair(&mut out, "name", value),
            HtmlAttribute::Value(value) => push_pair(&mut out, "value", value),
            HtmlAttribute::Placeholder(value) => {
                push_pair(&mut out, "placeholder", value);
            }
            HtmlAttribute::Disabled(present) => {
                push_bool(&mut out, "disabled", *present);
            }
            HtmlAttribute::Checked(present) => {
                push_bool(&mut out, "checked", *present);
            }
            HtmlAttribute::Selected(present) => {
                push_bool(&mut out, "selected", *present);
            }
            HtmlAttribute::Target(value) => push_pair(&mut out, "target", value),
            HtmlAttribute::Rel(rel) => push_pair(&mut out, "rel", rel.as_str()),
            HtmlAttribute::Other(key, value) => {
                push_pair(&mut out, &escape_attribute(key), value);
            }
        }
    }
    out
}

impl BrowserFragment<Ready> {
    /// The fragment's top-level composable node, if set.
    pub fn node(&self) -> Option<&ComposableNode> {
        self.node.as_ref()
    }

    /// The fragment's component stylesheet, if attached.
    pub fn stylesheet(&self) -> Option<&ComponentStylesheet> {
        self.stylesheet.as_ref()
    }

    /// The page-level features this fragment depends on.
    pub fn features(&self) -> &[PageFeature] {
        &self.features
    }

    /// The microdata key/value pairs this fragment contributes.
    pub fn metadata(&self) -> &HashMap<MicrodataKey, String> {
        &self.metadata
    }

    /// The `<link>` dependencies this fragment declares.
    pub fn dependency_links(&self) -> &[LinkTag] {
        &self.dependency_links
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::html::tag::BlockTag;

    #[test]
    fn define_as_raw_html_finalizes_to_ready() {
        let fragment = BrowserFragment::new()
            .define_as_raw_html("<svg></svg>")
            .finalize();
        match fragment.node {
            Some(ComposableNode::RawHtml(ref html)) => assert_eq!(html, "<svg></svg>"),
            _ => panic!("expected RawHtml node"),
        }
    }

    #[test]
    fn add_component_nests_a_ready_fragment() {
        let child = BrowserFragment::new()
            .define_as_text_fragment("child")
            .finalize();
        let parent = BrowserFragment::new()
            .define_as_block_tag(BlockTag::Div, "parent")
            .add_component(child)
            .finalize();
        match parent.node {
            Some(ComposableNode::BlockTag(ref block)) => {
                assert_eq!(block.content.children.len(), 1);
                assert!(matches!(
                    block.content.children[0],
                    ComposableNode::Component(_)
                ));
            }
            _ => panic!("expected BlockTag node"),
        }
    }

    #[test]
    fn raw_html_carries_cross_cutting_builders() {
        let fragment = BrowserFragment::new()
            .define_as_raw_html("<svg></svg>")
            .add_metadata_keypair(MicrodataKey::Title, "Diagram")
            .finalize();
        assert_eq!(
            fragment
                .metadata
                .get(&MicrodataKey::Title)
                .map(String::as_str),
            Some("Diagram")
        );
    }

    #[test]
    fn raw_html_is_unescaped_text_fragment_is_escaped() {
        let raw = BrowserFragment::new()
            .define_as_raw_html("<b>hi</b>")
            .finalize()
            .render();
        assert!(
            raw.contains("<b>hi</b>"),
            "RawHtml must pass through verbatim"
        );

        let text = BrowserFragment::new()
            .define_as_text_fragment("<b>hi</b>")
            .finalize()
            .render();
        assert!(!text.contains("<b>hi</b>"), "TextFragment must be escaped");
        assert_eq!(
            text, "&lt;b&gt;hi&lt;/b&gt;",
            "TextFragment must escape <, > to entities"
        );
    }
}
