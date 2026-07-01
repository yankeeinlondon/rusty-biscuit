use std::any::Any;
use std::rc::Rc;

use renderable::browser::PageOptions;
use renderable::browser::fragment::{BrowserFragment, Ready};
use renderable::html::HtmlPage;
use renderable::markdown::MarkdownRenderable;
use renderable::tree::render::{
    BrowserRenderOptions, MarkdownDialect, MarkdownRenderOptions, render_browser_node,
    render_markdown_node,
};
use renderable::style::Style;
use renderable::tree::{ListRenderHints, RenderNode, RenderStrictness, TreeRenderable};

use crate::{
    components::renderable::{BrowserRenderable, RenderableTerminalContent, TerminalRenderable},
    prelude::Prose,
    render_tree::projection::{
        ProjectionMode, fold_prose_nodes_into_blocks, project_renderable_content,
    },
    render_tree::{TerminalRenderOptions, render_terminal_node},
    terminal::Terminal,
    utils::{block_constraint::visible_width, layout::Layout},
};

/// Configure word wrap on an inline component so that wrapped continuation
/// lines align with content after the list prefix (bullet or number).
///
/// Sets the hanging indent only when one is not already configured,
/// preserving any explicit value the caller set on the component.
fn configure_component_wrap(content: &mut RenderableTerminalContent, hanging_indent: u32) {
    if let RenderableTerminalContent::Component(arc) = content
        && let Some(component) = Rc::get_mut(arc)
        && !component.is_block_level()
    {
        let layout = component.layout_mut();
        layout.word_wrap = layout
            .word_wrap
            .clone()
            .with_hanging_indent_if_none(hanging_indent);
    }
}

/// Force-update the hanging indent on a component, replacing any previously
/// configured value. Used when the bullet changes after initial construction.
fn force_component_hanging_indent(content: &mut RenderableTerminalContent, hanging_indent: u32) {
    if let RenderableTerminalContent::Component(arc) = content
        && let Some(component) = Rc::get_mut(arc)
        && !component.is_block_level()
    {
        let layout = component.layout_mut();
        layout.word_wrap = layout.word_wrap.clone().with_hanging_indent(hanging_indent);
    }
}

// =============================================================================
// OrderedList
// =============================================================================

/// An **OrderedList** contains a list of renderable items
/// which will be rendered into an **ordered** (numbered) list.
///
/// OrderedList renders items with numeric prefixes (1., 2., 3., etc.)
/// and handles word-wrapping with proper indentation so continuation
/// lines align with the start of the item text.
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::components::list::OrderedList;
///
/// // Create from a vec of strings
/// let list = OrderedList::new(vec!["First item", "Second item", "Third item"]);
/// // Renders as:
/// // 1. First item
/// // 2. Second item
/// // 3. Third item
///
/// // Build incrementally with add()
/// let mut list = OrderedList::empty();
/// list.add("Install dependencies").add("Run build").add("Deploy");
///
/// // Add with custom indentation for nested content
/// let list = OrderedList::new(vec!["Parent item"])
///     .with_indent_children(8);  // 8 spaces for nested content
/// ```
///
/// ## Layout & Style Contract
///
/// `OrderedList` is an internal-layout component (spec C2). It projects to a
/// [`List`](renderable::tree::NodeKind::List) node carrying
/// [`ListRenderHints`] and the configured [`Layout`]; the shared render-tree
/// fold resolves the outer box, and the list renderer fills the resolved
/// content width:
///
/// - [`Width::Auto`] (default) and [`Width::Fixed`] **fill** the available
///   width by wrapping the item body; [`Width::FitContent`] hugs the widest
///   item (short items stay short).
/// - **Slack sink** (spec D2): the item body text column. The numeric marker
///   (`"1. "`) and the hanging indent stay fixed across width modes, so a
///   narrower or wider box only reflows the body text.
/// - A fractional `Fixed(50%)` is resolved exactly once by the fold; the
///   list renderer wraps the body to the resolved content width and never
///   re-resolves the raw percentage (the `Fixed(50%) → 25%`
///   double-application bug).
/// - The projected [`ListRenderHints`] round-trip carries the [`Layout`] onto
///   the list node (C4), so the wrapping policy and hanging-indent contract
///   survive a second render pass.
///
/// [`Width::Auto`]: renderable::layout::Width::Auto
/// [`Width::Fixed`]: renderable::layout::Width::Fixed
/// [`Width::FitContent`]: renderable::layout::Width::FitContent
#[derive(Debug)]
pub struct OrderedList {
    items: Vec<RenderableTerminalContent>,
    layout: Layout,
    style: Style,
    indent_children: u32,
}

impl Default for OrderedList {
    fn default() -> Self {
        OrderedList {
            items: vec![],
            layout: Layout::default(),
            style: Style::default(),
            indent_children: 4,
        }
    }
}

impl<T: Into<String>> From<Vec<T>> for OrderedList {
    fn from(value: Vec<T>) -> Self {
        OrderedList {
            items: value
                .into_iter()
                .map(|f| RenderableTerminalContent::String(f.into()))
                .collect(),
            ..OrderedList::default()
        }
    }
}

impl From<Vec<RenderableTerminalContent>> for OrderedList {
    fn from(value: Vec<RenderableTerminalContent>) -> Self {
        OrderedList {
            items: value,
            ..OrderedList::default()
        }
    }
}

impl From<Vec<&RenderableTerminalContent>> for OrderedList {
    fn from(value: Vec<&RenderableTerminalContent>) -> Self {
        OrderedList {
            items: value.into_iter().cloned().collect(),
            ..OrderedList::default()
        }
    }
}

impl OrderedList {
    /// Create a new ordered list from items.
    pub fn new<T: Into<String>>(items: Vec<T>) -> Self {
        Self::from(items)
    }

    /// Create an empty ordered list.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Add an item that can be converted to RenderableTerminalContent.
    ///
    /// Inline components automatically get word wrap configured with
    /// the correct hanging indent so continuation lines align after
    /// the number prefix.
    ///
    /// ## Examples
    ///
    /// ```
    /// use biscuit_terminal::components::list::OrderedList;
    /// use biscuit_terminal::components::prose::Prose;
    ///
    /// let mut list = OrderedList::empty();
    /// list.add("First item").add("Second item");
    /// ```
    pub fn add<T: Into<RenderableTerminalContent>>(&mut self, item: T) -> &mut Self {
        let mut content = item.into();
        // Prefix width depends on the item number: "1. " = 3, "10. " = 4, etc.
        let number = self.items.len() + 1;
        let prefix = format!("{number}. ");
        let prefix_width = visible_width(&prefix);
        configure_component_wrap(&mut content, prefix_width);
        self.items.push(content);
        self
    }

    /// Set the indentation width for child block-level components.
    pub fn with_indent_children(mut self, indent: u32) -> Self {
        self.indent_children = indent;
        self
    }

    /// Builds the canonical [`NodeKind::List`] tree node for this ordered list.
    ///
    /// This is the **single private projection helper**. Both
    /// [`TreeRenderable::render_tree`] and the legacy
    /// [`TerminalRenderable::render_tree_node`] hook delegate to it, so the
    /// terminal compatibility surface cannot drift away from the canonical
    /// tree-renderable producer.
    ///
    /// Each item is projected into a [`NodeKind::ListItem`] via
    /// [`project_list_items`]. The list seeds typed [`ListRenderHints`]
    /// (`bullet: None`, `hanging_indent: true`,
    /// `indent_children: Some(self.indent_children)`) and a non-default
    /// [`Layout`] onto the root node.
    ///
    /// [`NodeKind::List`]: renderable::tree::NodeKind::List
    /// [`NodeKind::ListItem`]: renderable::tree::NodeKind::ListItem
    fn to_render_tree_node(&self) -> RenderNode {
        self.to_render_tree_node_with_terminal(None)
    }

    /// Builds the canonical [`NodeKind::List`] tree node, threading an
    /// optional `terminal_hint` through to [`project_list_items`] so that
    /// bespoke-only child components get capability-honest fallback
    /// rendering. Used by [`Self::render_via_tree`] which has the caller's
    /// real terminal.
    fn to_render_tree_node_with_terminal(&self, terminal_hint: Option<&Terminal>) -> RenderNode {
        let children = project_list_items(&self.items, terminal_hint);
        let mut node = RenderNode::list(true, None, children);
        // Only emit `indent_children` when the caller customized it; emitting
        // the default would erase the "did the user customize?" signal in the
        // projected node.
        let indent_children = (self.indent_children != OrderedList::default().indent_children)
            .then_some(self.indent_children);
        node.attrs.set_list_hints(&ListRenderHints {
            bullet: None,
            hanging_indent: true,
            indent_children,
        });
        if self.layout != Layout::default() {
            node.attrs.set_layout(&self.layout);
        }
        crate::components::renderable::overlay_style_onto_node(&mut node, &self.style);
        node
    }

    /// Renders the ordered list through the canonical render tree.
    ///
    /// Used by the [`TerminalRenderable`] impl to route Terminal output
    /// through the same tree the Browser and Markdown paths consume.
    ///
    /// Failures are logged via `tracing::error!` and fall back to an empty
    /// string: the [`TerminalRenderable::render`] trait is infallible by
    /// contract, and surfacing a `[render-tree error: …]` sentinel as in-band
    /// terminal text would pollute user output.
    fn render_via_tree(&self, term: &Terminal) -> String {
        // Thread the caller's real terminal through projection so any
        // bespoke-only child (no `render_tree_node`) gets a capability-honest
        // fallback — matching the pattern Compose established.
        let node = self.to_render_tree_node_with_terminal(Some(term));
        let opts = TerminalRenderOptions::new(term, RenderStrictness::Warn);
        match render_terminal_node(&node, &opts) {
            Ok(rendered) => rendered.output,
            Err(error) => {
                tracing::error!(
                    component = "OrderedList",
                    error = %error,
                    "render_terminal_node failed; emitting empty output"
                );
                String::new()
            }
        }
    }
}

impl TerminalRenderable for OrderedList {
    /// Renders to a terminal string at an explicit width.
    ///
    /// Routes through the canonical render tree via [`Self::render_via_tree`]
    /// so terminal output matches the Browser and Markdown paths for the same
    /// component.
    fn render_optimistic(&self, term_width: Option<u32>) -> String {
        let term = match term_width {
            Some(width) => Terminal::new_optimistic(width),
            None => Terminal::new_optimistic(80),
        };
        self.render_via_tree(&term)
    }

    /// Renders to the supplied terminal.
    ///
    /// Routes through the canonical render tree via [`Self::render_via_tree`].
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

    fn style(&self) -> Style {
        self.style.clone()
    }

    fn style_mut(&mut self) -> Option<&mut Style> {
        Some(&mut self.style)
    }

    fn is_block_level(&self) -> bool {
        true
    }

    /// Projects this ordered list into a [`NodeKind::List`] render-tree node.
    ///
    /// Delegates to the single private projection helper
    /// [`Self::to_render_tree_node`], shared with
    /// [`TreeRenderable::render_tree`] so the terminal compatibility hook and
    /// the canonical tree producer cannot drift.
    ///
    /// [`NodeKind::List`]: renderable::tree::NodeKind::List
    fn render_tree_node(&self) -> Option<RenderNode> {
        Some(self.to_render_tree_node())
    }
}

impl TreeRenderable for OrderedList {
    /// Projects the ordered list into the canonical render tree.
    ///
    /// Delegates to the single private projection helper
    /// [`OrderedList::to_render_tree_node`] so this canonical entry point and
    /// the terminal-compatibility [`TerminalRenderable::render_tree_node`]
    /// hook share one source of truth.
    ///
    /// The projected tree is a [`NodeKind::List`](renderable::tree::NodeKind::List)
    /// with `ordered = true` and one [`NodeKind::ListItem`](renderable::tree::NodeKind::ListItem)
    /// per source item. A non-default [`Layout`] is recorded on the root
    /// node's attributes; typed [`ListRenderHints`] carry `hanging_indent` and
    /// `indent_children` so the renderers can lower the list correctly.
    ///
    /// ## Notes
    ///
    /// When `Layout.width` resolves the outer list box, the terminal fold
    /// narrows `available_width`; the **item body text column** is the
    /// documented slack sink (spec decision D2) — the number marker and its
    /// hanging indent stay fixed while only the body text reflows to the
    /// narrowed width. `Layout.word_wrap` is a default fed to item text only
    /// where a per-item wrap policy is absent (spec decision D4).
    fn render_tree(&self) -> RenderNode {
        self.to_render_tree_node()
    }
}

impl MarkdownRenderable for OrderedList {
    /// Renders the ordered list as portable Markdown via the canonical render
    /// tree.
    ///
    /// The tree's [`NodeKind::List`](renderable::tree::NodeKind::List) lowers
    /// to standard CommonMark numbered-list syntax (`1. First\n2. Second\n…`).
    /// Layout is intentionally ignored by the Markdown renderer; styling on
    /// child components (for example a [`Prose`] item with `<b>` tokens) is
    /// degraded to plain text by the cross-target tree projection.
    fn render_markdown(&self) -> String {
        let node = self.render_tree();
        render_markdown_node(&node, &MarkdownRenderOptions::default())
            .map(|r| r.output)
            .unwrap_or_default()
    }

    /// Renders the ordered list as MarkdownPlus.
    ///
    /// Output is identical to [`Self::render_markdown`] for ordered lists:
    /// `OrderedList` is a structural container with no color, border, or fill
    /// of its own, so neither Markdown dialect has anything extra to emit.
    /// The method is provided so callers do not need to special-case lists
    /// when iterating over heterogeneous renderables.
    fn render_markdown_plus(&self) -> String {
        let node = self.render_tree();
        let opts = MarkdownRenderOptions {
            dialect: MarkdownDialect::MarkdownPlus,
            ..MarkdownRenderOptions::default()
        };
        render_markdown_node(&node, &opts)
            .map(|r| r.output)
            .unwrap_or_default()
    }
}

impl BrowserRenderable for OrderedList {
    /// Renders the ordered list as an HTML fragment via the canonical render
    /// tree.
    ///
    /// Calls [`render_browser_node`] directly on
    /// [`TreeRenderable::render_tree`]'s output, applying the same
    /// non-strict error policy as [`crate::render_tree::BrowserTreeComponent`]:
    /// rendering failures fall back to a visible diagnostic fragment so the
    /// infallible [`BrowserRenderable`] contract holds.
    fn render_html_fragment(&self) -> BrowserFragment<Ready> {
        let node = self.render_tree();
        let opts = BrowserRenderOptions {
            strictness: RenderStrictness::Warn,
            ..BrowserRenderOptions::default()
        };
        match render_browser_node(&node, &opts) {
            Ok(rendered) => rendered.output,
            Err(error) => BrowserFragment::new()
                .define_as_text_fragment(format!("[render-tree error: {error}]"))
                .finalize(),
        }
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

/// Projects a list's items into [`NodeKind::ListItem`] nodes.
///
/// Each [`RenderableTerminalContent`] item becomes one `ListItem`. Inline
/// projection delegates to the shared
/// [`project_renderable_content`] helper (the fourth migrated container
/// reusing the Prose-downcast pattern after `BlockQuote`,
/// [`Compose`](crate::components::compose::Compose), and `OrderedList`).
///
/// When an item is a [`Prose`] component, the helper returns the inline
/// nodes (and any fenced code block); this projection folds them into
/// block-level children via
/// [`fold_prose_nodes_into_blocks`]. A purely inline body collapses to a
/// single [`Paragraph`](renderable::tree::NodeKind::Paragraph) so the terminal
/// list renderer carries the prefix through to one wrapped line — without that
/// single wrapper, sibling inline children after the first would be
/// misclassified as block children and get `indent_children`-style
/// indentation. A `Prose` body carrying a fenced code block keeps the
/// [`Code`](renderable::tree::NodeKind::Code) node as a block-level sibling
/// instead of nesting it inside a `Paragraph` (which render-tree validation
/// rejects, leaving the list renderer with empty output).
///
/// For any other item the helper's structural fallback is used directly —
/// block-level children (a nested `List`, `Section`, etc.) become the
/// `ListItem`'s block children and are indented under the prefix.
///
/// `terminal_hint` threads the caller's real terminal context through the
/// shared structural projection so any bespoke-only child component (one
/// whose `render_tree_node()` returns `None`) gets rendered against that
/// terminal's actual capabilities rather than the projection layer's
/// optimistic default. This matches the pattern Compose established for
/// `HorizontalRule`-style children (text-only terminals must stay on the
/// Unicode/ASCII tier instead of jumping to the Kitty image tier). Pass
/// `None` when no terminal is available (e.g. the canonical
/// [`TreeRenderable::render_tree`] entry point, which is target-agnostic).
///
/// [`NodeKind::ListItem`]: renderable::tree::NodeKind::ListItem
fn project_list_items(
    items: &[RenderableTerminalContent],
    terminal_hint: Option<&Terminal>,
) -> Vec<RenderNode> {
    items
        .iter()
        .map(|item| {
            let is_prose = matches!(
                item,
                RenderableTerminalContent::Component(c)
                    if c.as_any().downcast_ref::<Prose>().is_some()
            );
            let projected =
                project_renderable_content(item, ProjectionMode::Structural { terminal_hint });
            let children = if is_prose {
                fold_prose_nodes_into_blocks(projected)
            } else {
                projected
            };
            RenderNode::list_item(None, children)
        })
        .collect()
}

// =============================================================================
// UnorderedList
// =============================================================================

/// An **UnorderedList** contains a list of renderable items
/// which will be rendered into an **unordered** (bullet-point) list.
///
/// Each item is prefixed with a bullet character (default: `- `) and
/// supports proper word-wrapping with hanging indentation so that
/// continuation lines align with the start of the item text.
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::components::list::UnorderedList;
///
/// // Create from a vec of strings
/// let list = UnorderedList::new(vec!["First item", "Second item", "Third item"]);
/// // Renders as:
/// // - First item
/// // - Second item
/// // - Third item
///
/// // Build incrementally with add()
/// let mut list = UnorderedList::empty();
/// list.add("Install dependencies").add("Run build").add("Deploy");
///
/// // Custom bullet character
/// let list = UnorderedList::new(vec!["Option A", "Option B", "Option C"])
///     .with_bullet("→ ");  // Renders with → instead of -
///
/// // Disable hanging indent for wrapped lines
/// let list = UnorderedList::new(vec!["Long item that wraps"])
///     .without_hanging_indent();
/// ```
///
/// ## Features
///
/// - **Hanging indent**: Continuation lines align after the bullet (enabled by default)
/// - **Custom bullets**: Use any character or string as the bullet marker
/// - **Nested content**: Block-level children are indented without bullets
/// - **Mixed content**: Supports both string items and renderable components
///
/// ## Layout & Style Contract
///
/// `UnorderedList` is an internal-layout component (spec C2). It projects to
/// a [`List`](renderable::tree::NodeKind::List) node carrying
/// [`ListRenderHints`] and the configured [`Layout`]; the shared render-tree
/// fold resolves the outer box, and the list renderer fills the resolved
/// content width:
///
/// - [`Width::Auto`] (default) and [`Width::Fixed`] **fill** the available
///   width by wrapping the item body; [`Width::FitContent`] hugs the widest
///   item (short items stay short).
/// - **Slack sink** (spec D2): the item body text column. The bullet (`"- "`)
///   and the hanging indent stay fixed across width modes, so a narrower or
///   wider box only reflows the body text.
/// - A fractional `Fixed(50%)` is resolved exactly once by the fold; the
///   list renderer wraps the body to the resolved content width and never
///   re-resolves the raw percentage (the `Fixed(50%) → 25%`
///   double-application bug).
/// - The projected [`ListRenderHints`] round-trip carries the [`Layout`] onto
///   the list node (C4), so the wrapping policy and hanging-indent contract
///   survive a second render pass.
///
/// [`Width::Auto`]: renderable::layout::Width::Auto
/// [`Width::Fixed`]: renderable::layout::Width::Fixed
/// [`Width::FitContent`]: renderable::layout::Width::FitContent
#[derive(Debug)]
pub struct UnorderedList {
    items: Vec<RenderableTerminalContent>,
    bullet: String,
    hanging_indent: bool,
    layout: Layout,
    style: Style,
    indent_children: Option<u32>,
}

impl Default for UnorderedList {
    fn default() -> Self {
        UnorderedList {
            items: vec![],
            bullet: "- ".to_string(),
            hanging_indent: true,
            layout: Layout::default(),
            style: Style::default(),
            indent_children: None,
        }
    }
}

impl<T: Into<String>> From<Vec<T>> for UnorderedList {
    fn from(value: Vec<T>) -> Self {
        UnorderedList {
            items: value
                .into_iter()
                .map(|f| RenderableTerminalContent::String(f.into()))
                .collect(),
            ..UnorderedList::default()
        }
    }
}

impl From<Vec<RenderableTerminalContent>> for UnorderedList {
    fn from(mut value: Vec<RenderableTerminalContent>) -> Self {
        let list = UnorderedList::default();
        if list.hanging_indent {
            let indent = list.indent_children.unwrap_or(visible_width(&list.bullet));
            for item in &mut value {
                configure_component_wrap(item, indent);
            }
        }
        UnorderedList {
            items: value,
            ..list
        }
    }
}

impl From<Vec<&RenderableTerminalContent>> for UnorderedList {
    fn from(value: Vec<&RenderableTerminalContent>) -> Self {
        let mut items: Vec<RenderableTerminalContent> = value.into_iter().cloned().collect();
        let list = UnorderedList::default();
        if list.hanging_indent {
            let indent = list.indent_children.unwrap_or(visible_width(&list.bullet));
            for item in &mut items {
                configure_component_wrap(item, indent);
            }
        }
        UnorderedList { items, ..list }
    }
}

impl From<Prose> for UnorderedList {
    fn from(value: Prose) -> Self {
        let mut list = UnorderedList::default();
        let mut content: RenderableTerminalContent = value.into();
        if list.hanging_indent {
            let indent = list.indent_children.unwrap_or(visible_width(&list.bullet));
            configure_component_wrap(&mut content, indent);
        }
        list.items = vec![content];
        list
    }
}

impl From<&Prose> for UnorderedList {
    fn from(value: &Prose) -> Self {
        UnorderedList::from(value.clone())
    }
}

impl UnorderedList {
    /// Create a new unordered list from items.
    pub fn new<T: Into<String>>(items: Vec<T>) -> Self {
        Self::from(items)
    }

    /// Create an empty unordered list.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Add an item that can be converted to RenderableTerminalContent.
    ///
    /// When `hanging_indent` is enabled (the default), inline components
    /// automatically get word wrap configured with the correct hanging
    /// indent so continuation lines align after the bullet.
    ///
    /// ## Examples
    ///
    /// ```
    /// use biscuit_terminal::components::list::UnorderedList;
    /// use biscuit_terminal::components::prose::Prose;
    ///
    /// let mut list = UnorderedList::empty();
    /// list.add("First item").add("Second item");
    /// ```
    pub fn add<T: Into<RenderableTerminalContent>>(&mut self, item: T) -> &mut Self {
        let mut content = item.into();
        if self.hanging_indent {
            let indent = self.indent_children.unwrap_or(visible_width(&self.bullet));
            configure_component_wrap(&mut content, indent);
        }
        self.items.push(content);
        self
    }

    /// Set a custom bullet character.
    ///
    /// When the new bullet has a different visible width than the current one,
    /// the hanging indent on all inline component items is updated to match
    /// so that continuation lines stay aligned with the start of the item text.
    pub fn with_bullet<T: Into<String>>(mut self, bullet: T) -> Self {
        let new_bullet: String = bullet.into();
        if self.hanging_indent {
            let old_width = visible_width(&self.bullet);
            let new_width = visible_width(&new_bullet);
            if old_width != new_width {
                let new_indent = self.indent_children.unwrap_or(new_width);
                for item in &mut self.items {
                    force_component_hanging_indent(item, new_indent);
                }
            }
        }
        self.bullet = new_bullet;
        self
    }

    /// Set the indentation width for child block-level components.
    ///
    /// When `None` (default), uses the visible width of the bullet.
    pub fn with_indent_children(mut self, indent: Option<u32>) -> Self {
        self.indent_children = indent;
        self
    }

    /// Disable hanging indent on wrapped lines.
    pub fn without_hanging_indent(mut self) -> Self {
        self.hanging_indent = false;
        self
    }

    /// Enable hanging indent on wrapped lines (default).
    pub fn with_hanging_indent(mut self) -> Self {
        self.hanging_indent = true;
        self
    }

    /// Builds the canonical [`NodeKind::List`] tree node for this unordered
    /// list.
    ///
    /// This is the **single private projection helper**. Both
    /// [`TreeRenderable::render_tree`] and the legacy
    /// [`TerminalRenderable::render_tree_node`] hook delegate to it, so the
    /// terminal compatibility surface cannot drift away from the canonical
    /// tree-renderable producer.
    ///
    /// Each item is projected into a [`NodeKind::ListItem`] via
    /// [`project_list_items`]. Typed [`ListRenderHints`] carry the bullet
    /// (omitted for the default `- `), the hanging-indent flag, and any
    /// explicit `indent_children` onto the root node. A non-default
    /// [`Layout`] is also recorded so renderers honor the list's margins,
    /// alignment, and word-wrap.
    ///
    /// [`NodeKind::List`]: renderable::tree::NodeKind::List
    /// [`NodeKind::ListItem`]: renderable::tree::NodeKind::ListItem
    fn to_render_tree_node(&self) -> RenderNode {
        self.to_render_tree_node_with_terminal(None)
    }

    /// Builds the canonical [`NodeKind::List`] tree node, threading an
    /// optional `terminal_hint` through to [`project_list_items`] so that
    /// bespoke-only child components get capability-honest fallback
    /// rendering. Used by [`Self::render_via_tree`] which has the caller's
    /// real terminal.
    fn to_render_tree_node_with_terminal(&self, terminal_hint: Option<&Terminal>) -> RenderNode {
        let children = project_list_items(&self.items, terminal_hint);
        let mut node = RenderNode::list(false, None, children);
        // Normalize the default `- ` bullet to `None` so the canonical tree
        // does not carry redundant component-level styling. Markdown and
        // Browser ignore the bullet hint entirely; the terminal renderer
        // falls back to `- ` when no hint is set.
        let bullet = if self.bullet == "- " {
            None
        } else {
            Some(self.bullet.clone())
        };
        node.attrs.set_list_hints(&ListRenderHints {
            bullet,
            hanging_indent: self.hanging_indent,
            indent_children: self.indent_children,
        });
        if self.layout != Layout::default() {
            node.attrs.set_layout(&self.layout);
        }
        crate::components::renderable::overlay_style_onto_node(&mut node, &self.style);
        node
    }

    /// Renders the unordered list through the canonical render tree.
    ///
    /// Used by the [`TerminalRenderable`] impl to route Terminal output
    /// through the same tree the Browser and Markdown paths consume.
    ///
    /// Failures are logged via `tracing::error!` and fall back to an empty
    /// string: the [`TerminalRenderable::render`] trait is infallible by
    /// contract, and surfacing a `[render-tree error: …]` sentinel as in-band
    /// terminal text would pollute user output.
    fn render_via_tree(&self, term: &Terminal) -> String {
        // Thread the caller's real terminal through projection so any
        // bespoke-only child (no `render_tree_node`) gets a capability-honest
        // fallback — matching the pattern Compose established.
        let node = self.to_render_tree_node_with_terminal(Some(term));
        let opts = TerminalRenderOptions::new(term, RenderStrictness::Warn);
        match render_terminal_node(&node, &opts) {
            Ok(rendered) => rendered.output,
            Err(error) => {
                tracing::error!(
                    component = "UnorderedList",
                    error = %error,
                    "render_terminal_node failed; emitting empty output"
                );
                String::new()
            }
        }
    }
}

impl TerminalRenderable for UnorderedList {
    /// Renders to a terminal string at an explicit width.
    ///
    /// Routes through the canonical render tree via [`Self::render_via_tree`]
    /// so terminal output matches the Browser and Markdown paths for the same
    /// component.
    fn render_optimistic(&self, term_width: Option<u32>) -> String {
        let term = match term_width {
            Some(width) => Terminal::new_optimistic(width),
            None => Terminal::new_optimistic(80),
        };
        self.render_via_tree(&term)
    }

    /// Renders to the supplied terminal.
    ///
    /// Routes through the canonical render tree via [`Self::render_via_tree`].
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

    fn style(&self) -> Style {
        self.style.clone()
    }

    fn style_mut(&mut self) -> Option<&mut Style> {
        Some(&mut self.style)
    }

    fn is_block_level(&self) -> bool {
        true
    }

    /// Projects this unordered list into a [`NodeKind::List`] render-tree
    /// node.
    ///
    /// Delegates to the single private projection helper
    /// [`Self::to_render_tree_node`], shared with
    /// [`TreeRenderable::render_tree`] so the terminal compatibility hook and
    /// the canonical tree producer cannot drift.
    ///
    /// [`NodeKind::List`]: renderable::tree::NodeKind::List
    fn render_tree_node(&self) -> Option<RenderNode> {
        Some(self.to_render_tree_node())
    }
}

impl TreeRenderable for UnorderedList {
    /// Projects the unordered list into the canonical render tree.
    ///
    /// Delegates to the single private projection helper
    /// [`UnorderedList::to_render_tree_node`] so this canonical entry point
    /// and the terminal-compatibility
    /// [`TerminalRenderable::render_tree_node`] hook share one source of
    /// truth.
    ///
    /// The projected tree is a [`NodeKind::List`](renderable::tree::NodeKind::List)
    /// with `ordered = false` and one
    /// [`NodeKind::ListItem`](renderable::tree::NodeKind::ListItem) per
    /// source item. Typed [`ListRenderHints`] carry the (optional) custom
    /// bullet, the hanging-indent flag, and any explicit `indent_children`
    /// width. A non-default [`Layout`] is recorded on the root node's
    /// attributes.
    ///
    /// Note that the bullet hint is a **terminal-rendering concern only**.
    /// The Markdown and Browser renderers ignore it — Markdown always emits
    /// standard `- ` markers and Browser always emits `<ul>`/`<li>`.
    ///
    /// ## Notes
    ///
    /// When `Layout.width` resolves the outer list box, the terminal fold
    /// narrows `available_width`; the **item body text column** is the
    /// documented slack sink (spec decision D2) — the bullet and its hanging
    /// indent stay fixed while only the body text reflows to the narrowed
    /// width. `Layout.word_wrap` is a default fed to item text only where a
    /// per-item wrap policy is absent (spec decision D4).
    fn render_tree(&self) -> RenderNode {
        self.to_render_tree_node()
    }
}

impl MarkdownRenderable for UnorderedList {
    /// Renders the unordered list as portable Markdown via the canonical
    /// render tree.
    ///
    /// The tree's [`NodeKind::List`](renderable::tree::NodeKind::List) lowers
    /// to standard CommonMark bullet-list syntax (`- First\n- Second\n…`).
    /// A custom terminal bullet (set via [`Self::with_bullet`]) does **not**
    /// affect Markdown output — Markdown's unordered list syntax has no
    /// facility for custom bullets, and round-tripping through a
    /// `-`-using renderer is the portable contract.
    ///
    /// Layout is intentionally ignored by the Markdown renderer; styling on
    /// child components (for example a [`Prose`] item with `<b>` tokens) is
    /// degraded to plain text by the cross-target tree projection.
    fn render_markdown(&self) -> String {
        let node = self.render_tree();
        render_markdown_node(&node, &MarkdownRenderOptions::default())
            .map(|r| r.output)
            .unwrap_or_default()
    }

    /// Renders the unordered list as MarkdownPlus.
    ///
    /// Output is identical to [`Self::render_markdown`] for unordered lists:
    /// `UnorderedList` is a structural container with no color, border, or
    /// fill of its own, so neither Markdown dialect has anything extra to
    /// emit. The method is provided so callers do not need to special-case
    /// lists when iterating over heterogeneous renderables.
    fn render_markdown_plus(&self) -> String {
        let node = self.render_tree();
        let opts = MarkdownRenderOptions {
            dialect: MarkdownDialect::MarkdownPlus,
            ..MarkdownRenderOptions::default()
        };
        render_markdown_node(&node, &opts)
            .map(|r| r.output)
            .unwrap_or_default()
    }
}

impl BrowserRenderable for UnorderedList {
    /// Renders the unordered list as an HTML fragment via the canonical
    /// render tree.
    ///
    /// Calls [`render_browser_node`] directly on
    /// [`TreeRenderable::render_tree`]'s output, applying the same
    /// non-strict error policy as
    /// [`crate::render_tree::BrowserTreeComponent`]: rendering failures fall
    /// back to a visible diagnostic fragment so the infallible
    /// [`BrowserRenderable`] contract holds.
    ///
    /// HTML output is a standard `<ul>` containing one `<li>` per item. A
    /// custom terminal bullet does not affect HTML output (browsers control
    /// list-marker presentation via CSS `list-style-type`).
    fn render_html_fragment(&self) -> BrowserFragment<Ready> {
        let node = self.render_tree();
        let opts = BrowserRenderOptions {
            strictness: RenderStrictness::Warn,
            ..BrowserRenderOptions::default()
        };
        match render_browser_node(&node, &opts) {
            Ok(rendered) => rendered.output,
            Err(error) => BrowserFragment::new()
                .define_as_text_fragment(format!("[render-tree error: {error}]"))
                .finalize(),
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::wrap_policy::WordWrap;

    #[test]
    fn test_ordered_list_simple() {
        // OrderedList now routes through the canonical render tree, which
        // does not append a trailing newline. The bespoke path's trailing
        // newline is therefore an accepted divergence (`KNOWN_DRIFT`).
        let list = OrderedList::new(vec!["First", "Second", "Third"]);
        let result = list.render_optimistic(None);
        assert_eq!(result, "1. First\n2. Second\n3. Third");
    }

    #[test]
    fn test_unordered_list_simple() {
        // UnorderedList now routes through the canonical render tree, which
        // does not append a trailing newline. The bespoke path's trailing
        // newline is therefore an accepted divergence (`KNOWN_DRIFT`).
        let list = UnorderedList::new(vec!["Apple", "Banana", "Cherry"]);
        let result = list.render_optimistic(None);
        assert_eq!(result, "- Apple\n- Banana\n- Cherry");
    }

    #[test]
    fn test_unordered_list_custom_bullet() {
        // Tree renderer omits the trailing newline (`KNOWN_DRIFT`).
        let list = UnorderedList::new(vec!["Item 1", "Item 2"]).with_bullet("- ");
        let result = list.render_optimistic(None);
        assert_eq!(result, "- Item 1\n- Item 2");
    }

    #[test]
    fn test_empty_ordered_list() {
        let list: OrderedList = OrderedList::new(Vec::<String>::new());
        let result = list.render_optimistic(None);
        assert_eq!(result, "");
    }

    #[test]
    fn test_empty_unordered_list() {
        let list: UnorderedList = UnorderedList::new(Vec::<String>::new());
        let result = list.render_optimistic(None);
        assert_eq!(result, "");
    }

    // =========================================================================
    // Recursive Rendering Tests
    // =========================================================================

    #[test]
    fn test_nested_ordered_list() {
        // Tree renderer omits the trailing newline (`KNOWN_DRIFT`).
        let inner = OrderedList::new(vec!["Nested A", "Nested B"]);
        let items = vec![
            RenderableTerminalContent::String("First".to_string()),
            RenderableTerminalContent::Component(Rc::new(inner)),
        ];
        let list = OrderedList::from(items);
        let result = list.render_optimistic(Some(80));
        assert_eq!(result, "1. First\n    1. Nested A\n    2. Nested B");
    }

    #[test]
    fn test_three_level_nesting_width_compounds() {
        // Tree renderer omits the trailing newline (`KNOWN_DRIFT`).
        let inner = OrderedList::new(vec!["Deep"]);
        let middle = OrderedList::from(vec![RenderableTerminalContent::Component(Rc::new(inner))]);
        let outer = OrderedList::from(vec![RenderableTerminalContent::Component(Rc::new(middle))]);
        let result = outer.render_optimistic(Some(80));
        assert_eq!(result, "        1. Deep");
    }

    #[test]
    fn test_mixed_inline_and_block_children_ordered() {
        use crate::components::prose::Prose;

        let inner_list = OrderedList::new(vec!["Sub item"]);
        let prose = Prose::new("Inline text");
        let items = vec![
            RenderableTerminalContent::String("Plain string".to_string()),
            RenderableTerminalContent::Component(Rc::new(prose)),
            RenderableTerminalContent::Component(Rc::new(inner_list)),
        ];
        let list = OrderedList::from(items);
        let result = list.render_optimistic(Some(80));
        let lines: Vec<&str> = result.lines().collect();
        assert_eq!(lines[0], "1. Plain string");
        assert_eq!(lines[1], "2. Inline text");
        assert_eq!(lines[2], "    1. Sub item");
    }

    #[test]
    fn test_nested_unordered_list() {
        // Tree renderer omits the trailing newline (`KNOWN_DRIFT`).
        let inner = UnorderedList::new(vec!["Sub A", "Sub B"]);
        let items = vec![
            RenderableTerminalContent::String("Top".to_string()),
            RenderableTerminalContent::Component(Rc::new(inner)),
        ];
        let list = UnorderedList::from(items);
        let result = list.render_optimistic(Some(80));
        assert_eq!(result, "- Top\n  - Sub A\n  - Sub B");
    }

    #[test]
    fn test_ordered_list_containing_unordered_child() {
        // Tree renderer omits the trailing newline (`KNOWN_DRIFT`).
        let inner = UnorderedList::new(vec!["Apple", "Banana"]);
        let items = vec![
            RenderableTerminalContent::String("Fruits:".to_string()),
            RenderableTerminalContent::Component(Rc::new(inner)),
        ];
        let list = OrderedList::from(items);
        let result = list.render_optimistic(Some(80));
        assert_eq!(result, "1. Fruits:\n    - Apple\n    - Banana");
    }

    #[test]
    fn test_empty_nested_list() {
        // The tree renderer pads the empty nested list's blank line with
        // spaces under the indent ("    \n") and omits the trailing newline.
        // The bespoke path emitted a true blank line and a trailing newline
        // ("\n3. After\n"). Both are accepted under `KNOWN_DRIFT`.
        let inner = OrderedList::new(Vec::<String>::new());
        let items = vec![
            RenderableTerminalContent::String("Before".to_string()),
            RenderableTerminalContent::Component(Rc::new(inner)),
            RenderableTerminalContent::String("After".to_string()),
        ];
        let list = OrderedList::from(items);
        let result = list.render_optimistic(Some(80));
        // Numbering increments past the empty inner list (bespoke parity).
        assert!(result.starts_with("1. Before\n"), "got: {result:?}");
        assert!(result.ends_with("3. After"), "got: {result:?}");
    }

    #[test]
    fn test_no_output_exceeds_term_width() {
        let inner = OrderedList::new(vec!["Short"]);
        let items = vec![RenderableTerminalContent::Component(Rc::new(inner))];
        let list = OrderedList::from(items);
        let width = 40u32;
        let result = list.render_optimistic(Some(width));
        for line in result.lines() {
            let vis = visible_width(line);
            assert!(
                vis <= width,
                "Line exceeds width {}: {:?} ({})",
                width,
                line,
                vis
            );
        }
    }

    // =========================================================================
    // Hanging Indent Tests
    // =========================================================================

    #[test]
    fn test_unordered_string_wraps_with_hanging_indent() {
        // "- " is 2 chars wide, so with width=20 content gets 18 chars.
        let list = UnorderedList::new(vec!["This is a long item that wraps"]);
        let result = list.render_optimistic(Some(20));
        let lines: Vec<&str> = result.lines().collect();
        assert!(lines.len() > 1, "Expected wrapping: {:?}", lines);
        assert!(lines[0].starts_with("- "));
        for line in &lines[1..] {
            assert!(
                line.starts_with("  "),
                "Should have 2-space indent: {:?}",
                line
            );
            assert!(
                !line.starts_with("   "),
                "Should not exceed 2-space indent: {:?}",
                line
            );
        }
    }

    #[test]
    fn test_unordered_prose_gets_automatic_wrap() {
        // A Prose with no explicit word wrap gets WrapProse at add time.
        use crate::components::prose::Prose;

        let mut list = UnorderedList::empty();
        list.add(Prose::new(
            "This is a long prose item that should wrap automatically",
        ));
        let result = list.render_optimistic(Some(25));
        let lines: Vec<&str> = result.lines().collect();
        assert!(lines.len() > 1, "Expected wrapping: {:?}", lines);
        assert!(lines[0].starts_with("- "));
        // Continuation lines aligned after bullet (2 spaces)
        for line in &lines[1..] {
            assert!(
                line.starts_with("  "),
                "Should align after bullet: {:?}",
                line
            );
        }
    }

    #[test]
    fn test_unordered_prose_bespoke_wrap_no_double_indent() {
        // A Prose with BespokeProse(_, _, None) gets hanging indent filled
        // by the list. No double-indentation.
        use crate::components::prose::Prose;

        let prose = Prose::new("aaa, bbb, ccc, ddd, eee, fff, ggg")
            .with_word_wrap(WordWrap::BespokeProse(Some(50), vec![' ', ','], None));
        let mut list = UnorderedList::empty();
        list.add(prose);

        let result = list.render_optimistic(Some(20));
        let lines: Vec<&str> = result.lines().collect();
        assert!(lines.len() > 1, "Expected wrapping: {:?}", lines);
        assert!(lines[0].starts_with("- "));
        for line in &lines[1..] {
            assert!(
                line.starts_with("  "),
                "Should have 2-space indent: {:?}",
                line
            );
            assert!(
                !line.starts_with("    "),
                "Should NOT have 4-space double indent: {:?}",
                line
            );
        }
    }

    #[test]
    fn test_unordered_no_hanging_indent() {
        // Tree renderer omits the trailing newline (`KNOWN_DRIFT`).
        let list = UnorderedList::new(vec!["Short"]).without_hanging_indent();
        let result = list.render_optimistic(Some(80));
        assert_eq!(result, "- Short");
    }

    #[test]
    fn test_prose_explicit_indent_dropped_under_tree_path() {
        // Bespoke parity behavior: an explicit `WordWrap::WrapProse(_, Some(4))`
        // on a Prose item produced 4-space continuation indent on the
        // bespoke path. After the render-tree migration, the projection
        // extracts Prose's inline structure (via `to_render_nodes`) but does
        // not carry the per-Prose `word_wrap` field — the list renderer
        // wraps using only the list's bullet width (2 spaces by default for
        // `- `). This is an accepted divergence documented in `KNOWN_DRIFT`,
        // following the same "Prose styling loss" pattern as OrderedList.
        use crate::components::prose::Prose;

        let prose = Prose::new("aaa bbb ccc ddd eee fff")
            .with_word_wrap(WordWrap::WrapProse(None, Some(4)));
        let mut list = UnorderedList::empty();
        list.add(prose);

        let result = list.render_optimistic(Some(20));
        let lines: Vec<&str> = result.lines().collect();
        assert!(lines.len() > 1, "Expected wrapping: {:?}", lines);
        // Tree path: continuation lines align under the bullet width (2).
        for line in &lines[1..] {
            assert!(
                line.starts_with("  "),
                "Should align after bullet (2 spaces): {:?}",
                line
            );
        }
    }

    #[test]
    fn ordered_list_render_tree_node_carries_layout_when_margins_set() {
        use crate::utils::layout::{Length, Edges};
        let mut list = OrderedList::new(vec!["First", "Second"]);
        list.layout_mut().margin = Edges::x(Length::ch(2));
        let node = list.render_tree_node().unwrap();
        assert!(node.attrs.layout().is_some());
    }

    #[test]
    fn unordered_list_render_tree_node_carries_layout_when_margins_set() {
        use crate::utils::layout::{Length, Edges};
        let mut list = UnorderedList::new(vec!["Apple", "Banana"]);
        list.layout_mut().margin = Edges::x(Length::ch(2));
        let node = list.render_tree_node().unwrap();
        assert!(node.attrs.layout().is_some());
    }

    // =========================================================================
    // Embedded Prose Fenced Code (regression for review-2 container gap)
    // =========================================================================

    /// `true` if any `Paragraph` node directly contains a block-level `Code`
    /// child — the invalid shape that tripped render-tree validation before the
    /// fold fix (the list renderer then swallowed it into empty output).
    fn paragraph_contains_code(node: &RenderNode) -> bool {
        use renderable::tree::NodeKind;
        let bad_here = matches!(node.kind, NodeKind::Paragraph { .. })
            && node
                .children()
                .iter()
                .any(|c| matches!(c.kind, NodeKind::Code { .. }));
        bad_here || node.children().iter().any(paragraph_contains_code)
    }

    /// `true` if a block-level `Code` node appears anywhere in the tree.
    fn has_code(node: &RenderNode) -> bool {
        use renderable::tree::NodeKind;
        matches!(node.kind, NodeKind::Code { .. }) || node.children().iter().any(has_code)
    }

    const STYLED_FENCE: &str = "<red>before\n```\ncode\n```\nafter</red>";

    #[test]
    fn ordered_list_with_styled_fenced_code_prose_renders_via_tree() {
        use crate::components::prose::Prose;
        let items = vec![RenderableTerminalContent::Component(Rc::new(Prose::new(
            STYLED_FENCE,
        )))];
        let list = OrderedList::from(items);
        let result = list.render_optimistic(Some(80));
        // Non-empty proves validation passed: `render_via_tree` swallows a
        // validation failure into an empty string.
        assert!(!result.is_empty(), "expected non-empty render");
        for needle in ["before", "code", "after"] {
            assert!(result.contains(needle), "missing `{needle}`: {result:?}");
        }
        let node = list.render_tree();
        assert!(
            !paragraph_contains_code(&node),
            "a block-level Code node must not nest inside a Paragraph"
        );
        assert!(has_code(&node), "expected a block-level Code node in the tree");
    }

    #[test]
    fn unordered_list_with_styled_fenced_code_prose_renders_via_tree() {
        use crate::components::prose::Prose;
        let items = vec![RenderableTerminalContent::Component(Rc::new(Prose::new(
            STYLED_FENCE,
        )))];
        let list = UnorderedList::from(items);
        let result = list.render_optimistic(Some(80));
        assert!(!result.is_empty(), "expected non-empty render");
        for needle in ["before", "code", "after"] {
            assert!(result.contains(needle), "missing `{needle}`: {result:?}");
        }
        let node = list.render_tree();
        assert!(
            !paragraph_contains_code(&node),
            "a block-level Code node must not nest inside a Paragraph"
        );
        assert!(has_code(&node), "expected a block-level Code node in the tree");
    }
}
