use std::collections::HashMap;
use std::marker::PhantomData;

use crate::{
    browser::{ComponentStylesheet, feature::PageFeature},
    html::attribute::{ClassDefinition, DomId},
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
    /// A prompted-link popover whose document-unique `id` is deferred to render
    /// time (see [`PopoverNode`]).
    Popover(Box<PopoverNode>),
}

/// A prompted-link popover carried through the fragment IR with its
/// document-unique `id` **unallocated** until final render.
///
/// The render-tree browser writer emits this typed node instead of a fully
/// baked `<span>` wrapper, so two independently rendered prompted links can be
/// composed into one page (via [`HtmlPage::from_fragments`]) without colliding
/// on their ids — the collision the eager, per-writer allocator produced
/// (spec criterion 7). [`HtmlPage::render`] threads a single
/// [`PopoverIdAllocator`] across every fragment so each occurrence of a base
/// slug becomes `dm-popover-<base>`, `dm-popover-<base>-1`, … in document
/// order, and the anchor's `interestfor` / `aria-describedby` always name that
/// same occurrence's prompt.
///
/// [`HtmlPage::from_fragments`]: crate::html::HtmlPage::from_fragments
/// [`HtmlPage::render`]: crate::html::HtmlPage::render
pub(crate) struct PopoverNode {
    /// Readable slug derived from the link target (e.g. `https-example-com`);
    /// the allocator turns it into the document-unique id at render.
    pub(crate) id_base: String,
    /// Anchor attributes **excluding** the id-dependent `interestfor` /
    /// `aria-describedby`, which are appended once the id is allocated.
    pub(crate) anchor_attrs: Vec<HtmlAttribute>,
    /// The anchor's rendered child nodes (the link's display content).
    pub(crate) anchor_children: Vec<ComposableNode>,
    /// The prompt text. Escaped on emit. The prompt span's remaining
    /// attributes (`class`, `popover`, `role`) are constant and are rebuilt at
    /// render alongside the allocated `id`.
    pub(crate) prompt_text: String,
}

/// Allocates document-unique, readable, deterministic ids for prompted-link
/// popovers.
///
/// The first occurrence of a base slug uses it verbatim (`dm-popover-<base>`)
/// and each later occurrence appends an `-N` suffix, so repeated identical
/// prompted links never collide. A single allocator threaded across every
/// fragment of a page ([`HtmlPage::render`](crate::html::HtmlPage::render)) —
/// or across one streamed document walk — yields document-unique ids; the
/// streaming and fragment browser paths share this one type so their id
/// sequences stay byte-identical.
#[derive(Default)]
pub(crate) struct PopoverIdAllocator {
    counts: HashMap<String, usize>,
}

impl PopoverIdAllocator {
    /// Allocates the next id for a popover whose slug is `base`.
    pub(crate) fn allocate(&mut self, base: &str) -> String {
        let count = self.counts.entry(base.to_string()).or_insert(0);
        let id = if *count == 0 {
            format!("dm-popover-{base}")
        } else {
            format!("dm-popover-{base}-{count}")
        };
        *count += 1;
        id
    }
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

    /// Commit the fragment to a prompted-link popover shape ([`PopoverNode`]).
    ///
    /// The popover's document-unique `id` is allocated at render, not here, so
    /// composed fragments never collide. Transitions to [`RefineText`] so the
    /// cross-cutting builders still apply — the caller declares
    /// [`PageFeature::Popover`](crate::browser::feature::PageFeature::Popover)
    /// with `add_feature` before `finalize`.
    pub(crate) fn define_as_popover(self, popover: PopoverNode) -> BrowserFragment<RefineText> {
        self.into_state(Some(ComposableNode::Popover(Box::new(popover))))
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
    ///
    /// A standalone `render` scopes popover-id allocation to this fragment
    /// alone; to keep ids document-unique across several composed fragments,
    /// [`HtmlPage::render`](crate::html::HtmlPage::render) threads one allocator
    /// across them all via [`render_with`](BrowserFragment::render_with).
    pub fn render(&self) -> String {
        self.render_with(&mut PopoverIdAllocator::default())
    }

    /// Renders the fragment, drawing popover ids from a caller-owned
    /// [`PopoverIdAllocator`] so a page can keep ids unique across every
    /// fragment it composes.
    pub(crate) fn render_with(&self, popover_ids: &mut PopoverIdAllocator) -> String {
        match &self.node {
            Some(node) => render_node(node, popover_ids),
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

/// Recursively renders a single composable node to HTML, drawing prompted-link
/// popover ids from `popover_ids` so they stay unique across the render.
fn render_node(node: &ComposableNode, popover_ids: &mut PopoverIdAllocator) -> String {
    match node {
        ComposableNode::TextFragment(text) => crate::browser::utils::escape_text(text).into_owned(),
        ComposableNode::RawHtml(html) => html.clone(),
        ComposableNode::Component(fragment) => fragment.render_with(popover_ids),
        ComposableNode::VoidTag(void) => {
            format!(
                "<{}{}>",
                void.tag.name(),
                render_attributes(&void.attributes, None)
            )
        }
        ComposableNode::BlockTag(block) => {
            let name = block.tag.name();
            let children: String = block
                .content
                .children
                .iter()
                .map(|child| render_node(child, popover_ids))
                .collect();
            format!(
                "<{name}{}>{children}</{name}>",
                render_attributes(&block.attributes, Some(&block.base_class))
            )
        }
        ComposableNode::Popover(popover) => render_popover(popover, popover_ids),
    }
}

/// Renders a prompted-link popover, allocating its document-unique id from
/// `popover_ids` and injecting it into the anchor's `interestfor` /
/// `aria-describedby` and the prompt span's `id`.
///
/// The emitted bytes match the streaming full-document writer's prompted-link
/// markup so the fragment and streaming browser paths stay byte-identical.
fn render_popover(popover: &PopoverNode, popover_ids: &mut PopoverIdAllocator) -> String {
    let id = popover_ids.allocate(&popover.id_base);

    // `HtmlAttribute` is not `Clone`, so the id-dependent association attributes
    // are rendered from their own short-lived list and appended to the base
    // anchor attributes. Both sub-lists emit their class first (there is none
    // here) then their entries in order, so the concatenation is byte-identical
    // to rendering one combined list (the streaming path's single list).
    let base_attrs = render_attributes(&popover.anchor_attrs, None);
    let id_attrs = render_attributes(
        &[
            HtmlAttribute::Other("interestfor".into(), id.clone()),
            HtmlAttribute::Other("aria-describedby".into(), id.clone()),
        ],
        None,
    );
    let anchor_children: String = popover
        .anchor_children
        .iter()
        .map(|child| render_node(child, popover_ids))
        .collect();
    let anchor = format!("<a{base_attrs}{id_attrs}>{anchor_children}</a>");

    // `write_attributes` always emits the merged class first, so the vector
    // order below yields `class, id, popover, role` — matching the streaming
    // writer's prompt element exactly.
    let prompt_attrs = [
        HtmlAttribute::Class(ClassDefinition::new("dm-popover-prompt")),
        HtmlAttribute::Id(DomId::new(id)),
        HtmlAttribute::Other("popover".into(), "hint".to_string()),
        HtmlAttribute::Other("role".into(), "note".to_string()),
    ];
    let prompt = format!(
        "<span{}>{}</span>",
        render_attributes(&prompt_attrs, None),
        crate::browser::utils::escape_text(&popover.prompt_text)
    );

    format!(r#"<span class="dm-popover-wrapper">{anchor}{prompt}</span>"#)
}

/// Recursively validates a composable node.
fn validate_node(node: &ComposableNode) -> bool {
    match node {
        ComposableNode::TextFragment(_) | ComposableNode::RawHtml(_) => true,
        ComposableNode::VoidTag(_) => true,
        ComposableNode::Component(fragment) => fragment.validate_render_content(),
        ComposableNode::BlockTag(block) => block.content.children.iter().all(validate_node),
        ComposableNode::Popover(popover) => popover.anchor_children.iter().all(validate_node),
    }
}

/// Serializes a slice of attributes into an opening-tag attribute string
/// (leading space included when non-empty). Attribute values are escaped.
///
/// `extra_class` supplies a component wrapper class (the block's
/// `base_class`) which is merged ahead of any explicit
/// [`HtmlAttribute::Class`] values into a single `class="..."` attribute.
/// Pass `None` for void tags, which have no `base_class`.
///
/// `pub(crate)` so the direct render-tree document writer
/// ([`render_browser_document_html`](crate::tree::render::render_browser_document_html))
/// can stream byte-identical opening tags without building a fragment.
pub(crate) fn render_attributes(attributes: &[HtmlAttribute], extra_class: Option<&str>) -> String {
    let mut out = String::new();
    write_attributes(&mut out, attributes, extra_class);
    out
}

/// Streams the opening-tag attribute text for `attributes` into `out`, with a
/// leading space before each emitted attribute. Byte-for-byte identical to
/// [`render_attributes`], but writes straight into the destination buffer so
/// the direct document writer avoids an intermediate `String` per element and
/// the per-element class-parts vector.
pub(crate) fn write_attributes(
    out: &mut String,
    attributes: &[HtmlAttribute],
    extra_class: Option<&str>,
) {
    use crate::browser::utils::escape_attribute;

    /// Appends a leading-space `key="value"` pair with the value escaped,
    /// writing each piece straight into `out` rather than through a `format!`
    /// temporary.
    fn push_pair(out: &mut String, key: &str, value: &str) {
        out.push(' ');
        out.push_str(key);
        out.push_str("=\"");
        out.push_str(&escape_attribute(value));
        out.push('"');
    }
    /// Appends a leading-space bare boolean attribute when `present`.
    fn push_bool(out: &mut String, name: &str, present: bool) {
        if present {
            out.push(' ');
            out.push_str(name);
        }
    }
    /// Appends one class token to the in-progress merged class string, applying
    /// the same trim / skip-empty / single-space-join rule as
    /// [`classes`](crate::browser::utils::classes).
    fn push_class(merged: &mut String, part: &str) {
        let trimmed = part.trim();
        if trimmed.is_empty() {
            return;
        }
        if !merged.is_empty() {
            merged.push(' ');
        }
        merged.push_str(trimmed);
    }

    // Merge the component wrapper class with every explicit `Class` attribute
    // into a single `class="..."` (base class first). Built in place to avoid
    // the per-element `Vec<Option<&str>>` the old parts list allocated.
    let mut merged_class = String::new();
    if let Some(extra) = extra_class {
        push_class(&mut merged_class, extra);
    }
    for attr in attributes {
        if let HtmlAttribute::Class(class) = attr {
            push_class(&mut merged_class, class.as_str());
        }
    }
    if !merged_class.is_empty() {
        push_pair(out, "class", &merged_class);
    }

    for attr in attributes {
        match attr {
            // Handled above as part of the merged class attribute.
            HtmlAttribute::Class(_) => {}
            HtmlAttribute::Id(id) => push_pair(out, "id", id.as_str()),
            HtmlAttribute::Style(style) => {
                let value = style.to_css().replace('\n', " ");
                push_pair(out, "style", &value);
            }
            HtmlAttribute::Title(value) => push_pair(out, "title", value),
            HtmlAttribute::Data(name, value) => {
                push_pair(out, &format!("data-{}", name.as_str()), value);
            }
            HtmlAttribute::Hidden(present) => push_bool(out, "hidden", *present),
            HtmlAttribute::Role(role) => push_pair(out, "role", role.as_str()),
            HtmlAttribute::Aria(aria) => out.push_str(&aria.render()),
            HtmlAttribute::Href(url) => push_pair(out, "href", url.as_str()),
            HtmlAttribute::Src(url) => push_pair(out, "src", url.as_str()),
            HtmlAttribute::Alt(value) => push_pair(out, "alt", value),
            HtmlAttribute::Type(html_type) => {
                push_pair(out, "type", &html_type.as_value());
            }
            HtmlAttribute::Name(value) => push_pair(out, "name", value),
            HtmlAttribute::Value(value) => push_pair(out, "value", value),
            HtmlAttribute::Placeholder(value) => {
                push_pair(out, "placeholder", value);
            }
            HtmlAttribute::Disabled(present) => {
                push_bool(out, "disabled", *present);
            }
            HtmlAttribute::Checked(present) => {
                push_bool(out, "checked", *present);
            }
            HtmlAttribute::Selected(present) => {
                push_bool(out, "selected", *present);
            }
            HtmlAttribute::Target(value) => push_pair(out, "target", value),
            HtmlAttribute::Rel(rel) => push_pair(out, "rel", rel.as_str()),
            HtmlAttribute::Other(key, value) => {
                push_pair(out, &escape_attribute(key), value);
            }
        }
    }
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

    /// Rolls up this fragment's declared features and those of every nested
    /// component fragment, in first-seen order with duplicates removed.
    ///
    /// [`features`](BrowserFragment::features) returns only this fragment's own
    /// top-level requests; a fragment composed of nested components (e.g. the
    /// browser writer's document body) carries its feature requests on those
    /// nested [`Component`](ComposableNode::Component) fragments. This is the
    /// single-fragment analogue of [`HtmlPage::features`], so
    /// [`render_browser_node`] can return the same first-seen rollup a whole
    /// page would.
    ///
    /// [`HtmlPage::features`]: crate::html::HtmlPage::features
    /// [`render_browser_node`]: crate::tree::render::render_browser_node
    pub fn collect_features(&self) -> Vec<PageFeature> {
        let mut out: Vec<PageFeature> = Vec::new();
        self.push_features(&mut out);
        out
    }

    /// Appends this fragment's own features, then recurses into nested
    /// component fragments — first-seen order, duplicates skipped.
    fn push_features(&self, out: &mut Vec<PageFeature>) {
        for &feature in &self.features {
            if !out.contains(&feature) {
                out.push(feature);
            }
        }
        if let Some(node) = &self.node {
            push_node_features(node, out);
        }
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

/// Walks a composable node tree, descending into `BlockTag` children and
/// recursing into every `Component` fragment to roll up its features — the
/// feature analogue of [`HtmlPage`](crate::html::HtmlPage)'s fragment walk.
fn push_node_features(node: &ComposableNode, out: &mut Vec<PageFeature>) {
    match node {
        ComposableNode::Component(child) => child.push_features(out),
        ComposableNode::BlockTag(block) => {
            for child in &block.content.children {
                push_node_features(child, out);
            }
        }
        ComposableNode::Popover(popover) => {
            for child in &popover.anchor_children {
                push_node_features(child, out);
            }
        }
        _ => {}
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
