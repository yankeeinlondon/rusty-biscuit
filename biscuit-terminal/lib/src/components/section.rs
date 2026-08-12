use renderable::browser::PageOptions;
use renderable::browser::fragment::{BrowserFragment, Ready};
use renderable::html::HtmlPage;
use renderable::markdown::MarkdownRenderable;
use renderable::style::{Style, TextEmphasis};
use renderable::tree::render::{
    BrowserRenderOptions, MarkdownDialect, MarkdownRenderOptions, render_browser_node,
    render_markdown_node,
};
use renderable::tree::{HeadingDepth, RenderNode, RenderStrictness, TreeRenderable};
use std::any::Any;

use crate::{
    components::renderable::{BrowserRenderable, RenderableTerminalContent, TerminalRenderable},
    render_tree::{
        TerminalRenderOptions,
        projection::{ProjectionMode, project_renderable_content},
        render_terminal_node,
    },
    terminal::Terminal,
    utils::layout::Layout,
};

/// Heading level for sections, from h1 (largest) to h6 (smallest).
///
/// Levels h1-h3 use bold styling, h4-h5 use italic, and h6 renders as plain text.
/// The level also determines the Markdown-style prefix: `# ` for h1, `## ` for h2, etc.
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::components::section::{Section, HeadingLevel};
///
/// // Create sections at different levels
/// let h1 = Section::new(HeadingLevel::h1, "Title");
/// let h2 = Section::new(HeadingLevel::h2, "Section");
/// let h3 = Section::new(HeadingLevel::h3, "Subsection");
/// let h6 = Section::new(HeadingLevel::h6, "Minor heading");
///
/// // Get numeric level
/// assert_eq!(HeadingLevel::h2.level(), 2);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum HeadingLevel {
    h1,
    h2,
    h3,
    h4,
    h5,
    h6,
}

impl HeadingLevel {
    /// Get the numeric level (1-6).
    ///
    /// ## Returns
    ///
    /// A value in `1..=6`. The mapping is exhaustive over the enum and total:
    /// every [`HeadingLevel`] variant produces a distinct level in this range,
    /// so callers can safely feed the result into APIs that demand a 1-6
    /// heading depth (e.g. [`renderable::tree::HeadingDepth::new`]) without
    /// additional bounds checking.
    pub fn level(&self) -> u8 {
        match self {
            HeadingLevel::h1 => 1,
            HeadingLevel::h2 => 2,
            HeadingLevel::h3 => 3,
            HeadingLevel::h4 => 4,
            HeadingLevel::h5 => 5,
            HeadingLevel::h6 => 6,
        }
    }

    /// The declared heading [`Style`] for this level.
    ///
    /// Replaces the hard-coded heading SGR: levels h1-h3 declare bold
    /// emphasis, h4-h5 declare italic, and h6 declares no emphasis. The
    /// renderers lower the [`TextEmphasis`] to the target instead of
    /// hand-writing escape codes.
    pub fn heading_style(&self) -> Style {
        Style {
            emphasis: heading_emphasis(self.level()),
            ..Style::default()
        }
    }
}

/// The [`TextEmphasis`] declared for a heading depth (1-6).
///
/// Depths 1-3 are bold, 4-5 italic, and 6 carries no emphasis.
pub(crate) fn heading_emphasis(depth: u8) -> TextEmphasis {
    match depth {
        1..=3 => TextEmphasis {
            bold: true,
            ..TextEmphasis::default()
        },
        4 | 5 => TextEmphasis {
            italic: true,
            ..TextEmphasis::default()
        },
        _ => TextEmphasis::default(),
    }
}

/// A section with a heading and content.
///
/// Sections render a Markdown-style heading followed by arbitrary content.
/// The heading level (h1-h6) controls both the visual styling and the
/// Markdown prefix (`#`, `##`, etc.).
///
/// ## Heading Levels
///
/// - Levels h1-h3: Bold styling
/// - Levels h4-h5: Italic styling
/// - Level h6: Plain text (no styling)
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::components::section::{Section, HeadingLevel};
/// use biscuit_terminal::components::renderable::TerminalRenderable;
///
/// // Create a section with heading and content
/// let mut section = Section::new(HeadingLevel::h2, "Getting Started");
/// section
///     .push("Welcome to the tutorial.")
///     .push("Let's begin with installation.");
///
/// // Render to terminal string
/// let output = section.render_optimistic(Some(80));
/// assert!(output.contains("## Getting Started"));
/// ```
///
/// ## Layout & Style Contract
///
/// `Section` is a block component that routes through the shared render-tree
/// fold (spec C1). All applicable `Layout` properties (`margin`, `padding`,
/// `width`, `max_width`, `alignment`, `word_wrap`) and `Style` properties
/// (`color`, `background`, `emphasis`, `border`) are honored on Terminal and
/// Browser; Markdown degrades layout/appearance attrs by Decision D1 and
/// preserves the Markdown heading structure and nested content.
///
/// ## Notes
///
/// Content can be strings, [`Prose`][crate::components::prose::Prose],
/// or any type implementing [`Into<RenderableTerminalContent>`][crate::components::renderable::RenderableTerminalContent].
#[derive(Debug)]
pub struct Section {
    level: HeadingLevel,
    title: String,
    content: Vec<RenderableTerminalContent>,
    layout: Layout,
    style: Style,
}

impl Section {
    /// Create a new section with a heading level and title.
    pub fn new<T: Into<String>>(level: HeadingLevel, title: T) -> Self {
        Section {
            level,
            title: title.into(),
            content: Vec::new(),
            layout: Layout::default(),
            style: Style::default(),
        }
    }

    /// Add content to the section.
    pub fn with_content(mut self, content: Vec<RenderableTerminalContent>) -> Self {
        self.content = content;
        self
    }

    /// Add a string item to the content.
    pub fn add_string<T: Into<String>>(&mut self, s: T) {
        self.content
            .push(RenderableTerminalContent::String(s.into()));
    }

    /// Add any content that can be converted to RenderableTerminalContent.
    ///
    /// This is a convenience method that accepts strings, Prose, and other
    /// renderable components without requiring manual wrapping.
    ///
    /// ## Examples
    ///
    /// ```
    /// use biscuit_terminal::components::section::{Section, HeadingLevel};
    /// use biscuit_terminal::components::prose::Prose;
    ///
    /// let mut section = Section::new(HeadingLevel::h2, "My Section");
    /// section.push("Plain text");
    /// section.push(Prose::new("<bold>Styled</bold> text"));
    /// ```
    pub fn push<T: Into<RenderableTerminalContent>>(&mut self, item: T) -> &mut Self {
        self.content.push(item.into());
        self
    }

    /// Builds the canonical [`NodeKind::Section`] tree node for this section.
    ///
    /// This is the **single private projection helper**. Both
    /// [`TreeRenderable::render_tree`] and the legacy
    /// [`TerminalRenderable::render_tree_node`] hook delegate to it, so the
    /// terminal compatibility surface cannot drift from the canonical
    /// tree-renderable producer.
    ///
    /// The heading level maps to a [`HeadingDepth`]; the title becomes a single
    /// [`RenderNode::text`] in the heading's phrasing content; each content
    /// item is projected through [`project_renderable_content`] using
    /// [`ProjectionMode::Structural`] so block-capable children (lists,
    /// tables, nested sections) keep their canonical tree shape while a
    /// `Prose` child's inline emphasis projects into structured inline nodes.
    ///
    /// A non-default [`Layout`] is seeded onto the returned node's `attrs`
    /// exactly once. Adapters that consume the node (`TreeComponent`,
    /// `BrowserTreeComponent`) render this node directly and do not apply the
    /// optional [`TreeRenderable::tree_layout`] hook — so the layout must live
    /// on the projected node itself, not on a separate adapter-level layout.
    ///
    /// [`NodeKind::Section`]: renderable::tree::NodeKind::Section
    fn to_render_node(&self, terminal_hint: Option<&Terminal>) -> RenderNode {
        // HeadingLevel::level() is guaranteed to be 1..=6, which
        // HeadingDepth::new accepts.
        let depth = HeadingDepth::new(self.level.level())
            .expect("HeadingLevel::level() always returns 1..=6");
        let heading = vec![RenderNode::text(&self.title)];

        let mut children = Vec::new();
        for item in &self.content {
            children.extend(project_renderable_content(
                item,
                ProjectionMode::Structural { terminal_hint },
            ));
        }

        let mut node = RenderNode::section(depth, heading, children);
        if self.layout != Layout::default() {
            node.attrs.set_layout(&self.layout);
        }
        crate::components::renderable::overlay_style_onto_node(&mut node, &self.style);
        node
    }

    /// Renders the section through the canonical render tree.
    ///
    /// Used by the [`TerminalRenderable`] impl to route Terminal output
    /// through the same tree the Browser and Markdown paths consume.
    ///
    /// Failures are logged via `tracing::error!` and fall back to an empty
    /// string: the [`TerminalRenderable::render`] trait is infallible by
    /// contract, and surfacing a `[render-tree error: …]` sentinel as in-band
    /// terminal text would pollute user output.
    fn render_via_tree(&self, term: &Terminal) -> String {
        let node = self.to_render_node(Some(term));
        let opts = TerminalRenderOptions::new(term, RenderStrictness::Warn);
        match render_terminal_node(&node, &opts) {
            Ok(rendered) => rendered.output,
            Err(error) => {
                tracing::error!(
                    component = "Section",
                    error = %error,
                    "render_terminal_node failed; emitting empty output"
                );
                String::new()
            }
        }
    }
}

impl TerminalRenderable for Section {
    /// Renders to a terminal string at an explicit width.
    ///
    /// Routes through the canonical render tree via [`Self::render_via_tree`]
    /// so terminal output matches the Browser and Markdown paths for the same
    /// component.
    fn render_optimistic(&self, term_width: Option<u32>) -> String {
        let term = Terminal::new_optimistic(term_width.unwrap_or(80));
        self.render_via_tree(&term)
    }

    /// Renders to the supplied terminal.
    ///
    /// Routes through the canonical render tree via [`Self::render_via_tree`].
    fn render(&self, term: &Terminal) -> String {
        self.render_via_tree(term)
    }

    fn as_any(&self) -> &dyn Any {
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

    /// Projects this section into a [`NodeKind::Section`] render-tree node.
    ///
    /// Delegates to the single private projection helper
    /// [`Self::to_render_node`], shared with [`TreeRenderable::render_tree`] so
    /// the terminal compatibility hook and the canonical tree producer cannot
    /// drift.
    ///
    /// [`NodeKind::Section`]: renderable::tree::NodeKind::Section
    fn render_tree_node(&self) -> Option<RenderNode> {
        Some(self.to_render_node(None))
    }
}

impl TreeRenderable for Section {
    /// Projects the section into the canonical render tree.
    ///
    /// Delegates to the single private projection helper
    /// [`Section::to_render_node`] so this canonical entry point and the
    /// terminal-compatibility [`TerminalRenderable::render_tree_node`] hook
    /// share one source of truth.
    ///
    /// [`NodeKind::Section`]: renderable::tree::NodeKind::Section
    fn render_tree(&self) -> RenderNode {
        self.to_render_node(None)
    }
}

impl MarkdownRenderable for Section {
    /// Renders the section as portable Markdown via the canonical render tree.
    ///
    /// Output is a heading line (`# Title`/`## Title`/…) followed by the body
    /// content separated by a blank line. Layout is intentionally ignored —
    /// CommonMark has no portable layout primitive.
    fn render_markdown(&self) -> String {
        let node = <Self as TreeRenderable>::render_tree(self);
        match render_markdown_node(&node, &MarkdownRenderOptions::default()) {
            Ok(rendered) => rendered.output,
            Err(error) => {
                tracing::error!(
                    component = "Section",
                    dialect = "Markdown",
                    error = %error,
                    "render_markdown_node failed; emitting empty output"
                );
                String::new()
            }
        }
    }

    /// Renders the section as MarkdownPlus via the canonical render tree.
    ///
    /// Section's structure is pure CommonMark (heading + paragraphs), so the
    /// MarkdownPlus output is structurally identical to portable Markdown for
    /// the section itself. Inline HTML may still appear inside child content
    /// when those children's MarkdownPlus dialects emit it.
    fn render_markdown_plus(&self) -> String {
        let node = <Self as TreeRenderable>::render_tree(self);
        let opts = MarkdownRenderOptions {
            dialect: MarkdownDialect::MarkdownPlus,
            ..MarkdownRenderOptions::default()
        };
        match render_markdown_node(&node, &opts) {
            Ok(rendered) => rendered.output,
            Err(error) => {
                tracing::error!(
                    component = "Section",
                    dialect = "MarkdownPlus",
                    error = %error,
                    "render_markdown_node failed; emitting empty output"
                );
                String::new()
            }
        }
    }
}

impl BrowserRenderable for Section {
    /// Renders the section as an HTML fragment via the canonical render tree.
    ///
    /// The browser tree renderer emits a semantic `<section>` element with the
    /// heading as `<h1>`-`<h6>` matching the section's level, body children
    /// as block elements, and `Layout` lowered to inline CSS where the tree
    /// path supports it.
    ///
    /// Failures are logged via `tracing::error!` and fall back to an empty
    /// [`BrowserFragment`]: the [`BrowserRenderable`] contract is infallible,
    /// and surfacing a `[render-tree error: …]` sentinel as in-band HTML would
    /// pollute the rendered page. This mirrors the Terminal path's behavior.
    fn render_html_fragment(&self) -> BrowserFragment<Ready> {
        let node = <Self as TreeRenderable>::render_tree(self);
        let opts = BrowserRenderOptions::default();
        match render_browser_node(&node, &opts) {
            Ok(rendered) => rendered.output,
            Err(error) => {
                tracing::error!(
                    component = "Section",
                    error = %error,
                    "render_browser_node failed; emitting empty fragment"
                );
                BrowserFragment::new()
                    .define_as_text_fragment(String::new())
                    .finalize()
            }
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

    #[test]
    fn test_h1_section_via_tree() {
        let section = Section::new(HeadingLevel::h1, "Title");
        let result = section.render_optimistic(None);
        // After the IR flip, `render_optimistic` routes through the tree
        // renderer. An empty section emits just the heading line via
        // `render_heading_line`, which uses the same `apply_style` lowering
        // as the bespoke path — bold open + single reset.
        assert_eq!(result, "\x1b[1m# Title\x1b[0m");
    }

    #[test]
    fn test_section_with_content_via_tree() {
        let mut section = Section::new(HeadingLevel::h2, "Header");
        section.add_string("Some content here.");
        let result = section.render_optimistic(None);
        // KNOWN_DRIFT: the tree renderer joins the heading and body with a
        // blank line (`{heading}\n\n{body}`), where the bespoke renderer
        // used a single newline. This is the user-facing canonical shape
        // after the flip.
        assert_eq!(result, "\x1b[1m## Header\x1b[0m\n\nSome content here.");
    }

    #[test]
    fn test_heading_levels() {
        assert_eq!(HeadingLevel::h1.level(), 1);
        assert_eq!(HeadingLevel::h6.level(), 6);
    }

    #[test]
    fn section_render_tree_node_carries_layout_when_margins_set() {
        use crate::utils::layout::{Length, Edges};
        let mut section = Section::new(HeadingLevel::h1, "Title");
        section.layout.margin = Edges::x(Length::ch(2));
        let node = section.render_tree_node().unwrap();
        assert!(node.attrs.layout().is_some());
    }

    #[test]
    fn tree_renderable_and_compatibility_hook_share_projection() {
        // The migration pattern: `TreeRenderable::render_tree` and the
        // terminal compatibility `TerminalRenderable::render_tree_node` hook
        // MUST share one private projection helper so they cannot drift.
        let mut section = Section::new(HeadingLevel::h2, "Header");
        section.add_string("Body");

        let canonical = <Section as TreeRenderable>::render_tree(&section);
        let compat = section.render_tree_node().expect("tree node");
        assert_eq!(
            serde_json::to_value(&canonical).unwrap(),
            serde_json::to_value(&compat).unwrap(),
            "canonical and compatibility projections must serialize identically"
        );
    }
}
