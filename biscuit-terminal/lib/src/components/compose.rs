use std::any::Any;

use renderable::browser::PageOptions;
use renderable::browser::fragment::{BrowserFragment, Ready};
use renderable::html::HtmlPage;
use renderable::markdown::MarkdownRenderable;
use renderable::tree::render::{
    BrowserRenderOptions, MarkdownDialect, MarkdownRenderOptions, render_browser_node,
    render_markdown_node,
};
use renderable::style::Style;
use renderable::tree::{RenderNode, RenderStrictness, SequenceJoin, TreeRenderable};

use crate::components::{
    filesystem::FileSystem,
    list::{OrderedList, UnorderedList},
    prose::Prose,
    renderable::{BrowserRenderable, RenderableTerminalContent, TerminalRenderable},
    section::{HeadingLevel, Section},
    table::table::Table,
};
use crate::render_tree::{TerminalRenderOptions, render_terminal_node};
use crate::terminal::Terminal;
use crate::utils::layout::Layout;

/// Composes multiple renderable components into a single renderable output.
///
/// This struct allows combining text, styled prose, tables, lists, and other
/// renderable components into one cohesive output for terminal display.
/// Parts are rendered sequentially with no automatic spacing between them.
///
/// ## Layout & Style Contract
///
/// `Compose` is a sequence container with a dual-mode contract (spec C1/C7):
///
/// - **Block-container mode** (the public component API, i.e. calling
///   `render()` / `render_tree()` directly): the `Compose` root routes through
///   the shared render-tree fold, so `Layout` box properties (`margin`,
///   `padding`, `width`, `max_width`, `alignment`) and `Style` (`color`,
///   `background`, `emphasis`, `border`) are honored via the fold (C1).
/// - **Inline mode** (when `Compose` content is nested inside another
///   component): the sequence itself carries no block box; the containing
///   block owns the box. The concatenated parts are inline content, so
///   inherited `color` / `emphasis` and inline `background` flow through
///   (C7). `Compose::is_block_level` remains `false` because its public
///   contract is inline concatenation, even though its own `Layout` is
///   applied when it is the top-level rendered node.
///
/// `word_wrap` is seeded onto the root node and honored by nested
/// prose-bearing parts.
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::prelude::*;
///
/// // From a vec of pre-converted items
/// let compose = Compose::new(vec![
///     RenderableTerminalContent::from("Hello, "),
///     RenderableTerminalContent::from(Prose::new("<bold>world</bold>!")),
/// ]);
/// ```
///
/// ```
/// use biscuit_terminal::prelude::*;
///
/// // Builder-style with fluent API
/// let mut compose = Compose::default();
/// compose
///     .add_text("Hello, ")
///     .add_prose(Prose::new("<bold>world</bold>!"));
/// ```
///
/// ```
/// use biscuit_terminal::prelude::*;
///
/// // Using From implementations for ergonomic creation
/// let text: Compose = "Hello, ".into();
/// let prose = Prose::new("<bold>bold text</bold>");
/// let combined = Compose::new(vec![text.into(), RenderableTerminalContent::from(prose)]);
/// ```
///
/// ```
/// use biscuit_terminal::prelude::*;
///
/// // Building a mixed content document
/// let mut doc = Compose::default();
/// doc
///     .add_heading("Project Overview", 1)
///     .add_text("This project contains ")
///     .add_prose(Prose::new("<bold>important</bold> files"))
///     .add_text(" for processing.");
/// ```
#[derive(Debug)]
pub struct Compose {
    parts: Vec<RenderableTerminalContent>,
    layout: Layout,
    style: Style,
}

impl Default for Compose {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}

impl From<String> for Compose {
    fn from(value: String) -> Self {
        Compose {
            parts: vec![RenderableTerminalContent::String(value)],
            layout: Layout::default(),
            style: Style::default(),
        }
    }
}

impl From<&str> for Compose {
    fn from(value: &str) -> Self {
        Compose {
            parts: vec![RenderableTerminalContent::String(value.into())],
            layout: Layout::default(),
            style: Style::default(),
        }
    }
}

impl From<RenderableTerminalContent> for Compose {
    fn from(value: RenderableTerminalContent) -> Self {
        Compose {
            parts: vec![value],
            layout: Layout::default(),
            style: Style::default(),
        }
    }
}

impl From<Vec<RenderableTerminalContent>> for Compose {
    fn from(items: Vec<RenderableTerminalContent>) -> Self {
        Compose {
            parts: items,
            layout: Layout::default(),
            style: Style::default(),
        }
    }
}

impl TerminalRenderable for Compose {
    fn render_optimistic(&self, term_width: Option<u32>) -> String {
        let term = match term_width {
            Some(width) => Terminal::new_optimistic(width),
            None => Terminal::default(),
        };
        self.render_via_tree(&term)
    }

    fn render(&self, term: &Terminal) -> String {
        tracing::trace!(parts = self.parts.len(), "Compose rendering");
        self.render_via_tree(term)
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

    fn as_any(&self) -> &dyn Any {
        self
    }

    /// Compose is a sequence container; the public contract is inline
    /// concatenation. Routing through the tree must not silently flip this
    /// flag, so it stays `false` regardless of contained children.
    fn is_block_level(&self) -> bool {
        false
    }

    /// Exposes the tree projection through the canonical
    /// [`TerminalRenderable::render_tree_node`] hook so cross-target adapters
    /// can consume Compose like any other tree-backed component.
    fn render_tree_node(&self) -> Option<RenderNode> {
        Some(<Self as TreeRenderable>::render_tree(self))
    }
}

impl Compose {
    /// Creates a new `Compose` from a vector of renderable items.
    pub fn new(items: Vec<RenderableTerminalContent>) -> Self {
        Compose {
            parts: items,
            layout: Layout::default(),
            style: Style::default(),
        }
    }

    /// Returns `true` when no parts have been added.
    ///
    /// This is the structural check — it inspects the in-memory part list
    /// directly, without performing any rendering. Prefer this over probing
    /// rendered output (e.g. `render_markdown().is_empty()`), which would
    /// couple the caller to renderer behavior and run an unnecessary pass.
    ///
    /// ## Examples
    ///
    /// ```
    /// use biscuit_terminal::prelude::*;
    ///
    /// assert!(Compose::default().is_empty());
    /// let mut c = Compose::default();
    /// c.add_text("x");
    /// assert!(!c.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.parts.is_empty()
    }

    /// Returns the number of parts currently held.
    pub fn len(&self) -> usize {
        self.parts.len()
    }

    /// Adds a block of _prose_ which is text that is allowed
    /// to embed styling tokens in it that can be rendered lazily
    /// when we're ready to send to the terminal.
    pub fn add_prose(&mut self, content: Prose) -> &mut Self {
        self.parts.push(RenderableTerminalContent::from(content));
        self
    }

    /// Adds plain text content.
    pub fn add_text<T: Into<String>>(&mut self, content: T) -> &mut Self {
        let text = content.into();
        self.parts.push(RenderableTerminalContent::from(text));
        self
    }

    /// Adds an unordered list.
    pub fn add_unordered_list(&mut self, content: UnorderedList) -> &mut Self {
        self.parts.push(RenderableTerminalContent::from(content));
        self
    }

    /// Adds an ordered list.
    pub fn add_ordered_list(&mut self, content: OrderedList) -> &mut Self {
        self.parts.push(RenderableTerminalContent::from(content));
        self
    }

    /// Adds a [`FileSystem`] tree component.
    pub fn add_file_system(&mut self, content: FileSystem) -> &mut Self {
        self.parts.push(RenderableTerminalContent::from(content));
        self
    }

    /// Adds a [`Table`] component.
    pub fn add_table(&mut self, content: Table) -> &mut Self {
        self.parts.push(RenderableTerminalContent::from(content));
        self
    }

    /// Adds a heading as a [`Section`] component.
    ///
    /// The `level` parameter maps to heading levels 1-6 (h1-h6).
    pub fn add_heading<T: Into<String>>(&mut self, title: T, level: u8) -> &mut Self {
        let heading_level = match level {
            1 => HeadingLevel::h1,
            2 => HeadingLevel::h2,
            3 => HeadingLevel::h3,
            4 => HeadingLevel::h4,
            5 => HeadingLevel::h5,
            _ => HeadingLevel::h6,
        };
        let section = Section::new(heading_level, title);
        self.parts.push(RenderableTerminalContent::from(section));
        self
    }

    /// Renders the Compose component through the canonical render tree.
    ///
    /// Failures are logged via `tracing::error!` and fall back to an empty
    /// string: the [`TerminalRenderable::render`] trait is infallible by
    /// contract, and surfacing a `[render-tree error: …]` sentinel as in-band
    /// terminal text would pollute user output.
    fn render_via_tree(&self, term: &Terminal) -> String {
        // Thread the actual terminal through projection so bespoke-only
        // children (no `render_tree_node`) fall back via the real target —
        // for example, a text-only terminal keeps HorizontalRule on its
        // Unicode/ASCII text tier instead of jumping to the Kitty image
        // tier the projection layer's default optimistic terminal advertises.
        let node = self.render_tree_with_terminal(Some(term));
        let opts = TerminalRenderOptions::new(term, RenderStrictness::Warn);
        match render_terminal_node(&node, &opts) {
            Ok(rendered) => rendered.output,
            Err(error) => {
                tracing::error!(
                    component = "Compose",
                    error = %error,
                    "render_terminal_node failed; emitting empty output"
                );
                String::new()
            }
        }
    }

    /// Projects a single Compose part into one or more render-tree nodes.
    ///
    /// `String` parts become a single [`RenderNode::text`]. Components with a
    /// canonical [`TerminalRenderable::render_tree_node`] projection (the
    /// post-IR-migration components such as `Section`, `Table`,
    /// `OrderedList`, `UnorderedList`, `Progress`, `TwoColumn`, and `Compose`
    /// itself) contribute their structural node directly.
    ///
    /// [`Prose`] gets a dedicated downcast path so inline styling
    /// (`<b>` / `<i>` / `<red>` runs) survives as structured inline
    /// `RenderNode`s (`Strong` / `Emphasis` / styled `Span`) — matching the
    /// pattern established by the `BlockQuote` migration. Without this, the
    /// generic [`RenderableTerminalContent::to_tree_nodes`] fallback would
    /// flatten Prose to ANSI-stripped plain text and Compose would silently
    /// drop the user's authored styling.
    ///
    /// `terminal_hint` is the terminal context to use when a bespoke-only
    /// component must be rendered for the ANSI-stripping fallback. Threading
    /// the actual terminal through preserves the caller's capability flags
    /// (e.g. text-only terminals stay on the text rendering tier of
    /// `HorizontalRule` instead of jumping to the Kitty image tier). Pass
    /// `None` to use the projection layer's default.
    fn project_part(
        part: &RenderableTerminalContent,
        terminal_hint: Option<&Terminal>,
    ) -> Vec<RenderNode> {
        use crate::render_tree::projection::{ProjectionMode, project_renderable_content};
        // The shared helper handles the Prose downcast + bespoke-only
        // terminal-threaded fallback; Compose only adds its own `Root`
        // flattening on top.
        let nodes = project_renderable_content(part, ProjectionMode::Structural { terminal_hint });
        // A nested `Compose` (or any tree-renderable that returns a `Root`
        // node) is invalid as a child of our outer `Root`. Inline its
        // children so the nested sequence's contents become siblings —
        // preserving Compose's no-separator semantics recursively.
        nodes
            .into_iter()
            .flat_map(|node| match node.kind {
                renderable::tree::NodeKind::Root { children } => children,
                _ => vec![node],
            })
            .collect()
    }

    /// Builds the canonical render tree using the supplied terminal as the
    /// fallback rendering context for any bespoke-only child component.
    ///
    /// See [`Self::project_part`] for the per-part projection rules.
    fn render_tree_with_terminal(&self, terminal: Option<&Terminal>) -> RenderNode {
        let mut children = Vec::with_capacity(self.parts.len());
        for part in &self.parts {
            children.extend(Self::project_part(part, terminal));
        }

        let mut root = RenderNode::root(children);
        root.attrs.set_sequence_join(SequenceJoin::None);
        if self.layout != Layout::default() {
            root.attrs.set_layout(&self.layout);
        }
        crate::components::renderable::overlay_style_onto_node(&mut root, &self.style);
        root
    }
}

impl TreeRenderable for Compose {
    /// Projects Compose into a canonical render tree sequence container.
    ///
    /// The root node is a [`NodeKind::Root`](renderable::tree::NodeKind::Root)
    /// carrying the [`SequenceJoin::None`] hint so Terminal, Browser,
    /// Markdown, and MarkdownPlus renderers concatenate the children with no
    /// renderer-inserted separator — that is Compose's defining contract.
    ///
    /// Each part projects through
    /// [`RenderableTerminalContent::to_tree_nodes`]: `String` parts become
    /// `Text` nodes, components with tree support contribute their structural
    /// `RenderNode` directly, and bespoke-only components fall back to
    /// ANSI-stripped text under the configured [`RenderStrictness`].
    ///
    /// A non-default [`Layout`] (margin, alignment, max-width, word wrap) is
    /// seeded onto the sequence container so the tree renderers honor the
    /// component's layout.
    fn render_tree(&self) -> RenderNode {
        // No terminal context available at the `TreeRenderable` boundary; the
        // bespoke-only fallback uses the projection layer's default
        // optimistic terminal. Terminal `render(term)` calls
        // [`render_tree_with_terminal`] directly to thread the real
        // terminal through.
        self.render_tree_with_terminal(None)
    }
}

impl MarkdownRenderable for Compose {
    /// Renders Compose as portable Markdown via the canonical render tree.
    ///
    /// The tree carries [`SequenceJoin::None`], so adjacent parts are
    /// concatenated with no Markdown-inserted blank-line separator —
    /// preserving Compose's defining contract. Children's own block syntax
    /// (headings, lists, tables, etc.) renders normally between them.
    fn render_markdown(&self) -> String {
        let node = <Self as TreeRenderable>::render_tree(self);
        render_markdown_node(&node, &MarkdownRenderOptions::default())
            .map(|r| r.output)
            .unwrap_or_default()
    }

    /// Renders Compose as MarkdownPlus via the canonical render tree.
    ///
    /// Compose itself has no MarkdownPlus-specific behavior, so the output is
    /// identical to [`render_markdown`](Self::render_markdown) unless a child
    /// node carries dialect-sensitive content (for example
    /// `ColumnsHints`-bearing block quotes or `ProgressHints`-bearing
    /// paragraphs).
    fn render_markdown_plus(&self) -> String {
        let node = <Self as TreeRenderable>::render_tree(self);
        let opts = MarkdownRenderOptions {
            dialect: MarkdownDialect::MarkdownPlus,
            ..MarkdownRenderOptions::default()
        };
        render_markdown_node(&node, &opts)
            .map(|r| r.output)
            .unwrap_or_default()
    }
}

impl BrowserRenderable for Compose {
    /// Renders Compose as an HTML fragment via the canonical render tree.
    ///
    /// Unlike the BlockQuote migration, this impl renders the tree directly
    /// rather than delegating through [`BrowserTreeComponent`] — that adapter
    /// requires `Clone`, which Compose does not implement because
    /// `RenderableTerminalContent::Component` already holds an `Rc<dyn ...>`
    /// internally and a deep semantic clone would be misleading.
    fn render_html_fragment(&self) -> BrowserFragment<Ready> {
        let node = <Self as TreeRenderable>::render_tree(self);
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
    use crate::components::inline_content::InlineContent;
    use crate::components::table::TableColumn;
    use crate::components::text_block::TextBlock;
    use crate::utils::layout::{Alignment, Length, TargetValue};
    use crate::utils::wrap_policy::WordWrap;

    // =====================================================================
    // Construction
    // =====================================================================

    #[test]
    fn test_new_empty_vec() {
        let compose = Compose::new(Vec::new());
        assert_eq!(compose.render_optimistic(Some(80)), "");
    }

    #[test]
    fn test_new_with_items() {
        let compose = Compose::new(vec![
            RenderableTerminalContent::from("foo"),
            RenderableTerminalContent::from("bar"),
        ]);
        assert_eq!(compose.render_optimistic(Some(80)), "foobar");
    }

    #[test]
    fn test_new_with_mixed_items() {
        let compose = Compose::new(vec![
            RenderableTerminalContent::from("text "),
            RenderableTerminalContent::from(Prose::new("styled")),
        ]);
        let output = compose.render_optimistic(Some(80));
        assert!(output.starts_with("text "));
        assert!(output.contains("styled"));
    }

    #[test]
    fn test_default_is_empty() {
        let compose = Compose::default();
        assert_eq!(compose.render_optimistic(Some(80)), "");
    }

    #[test]
    fn test_new_empty_and_default_equivalent() {
        assert_eq!(
            Compose::new(Vec::new()).render_optimistic(Some(80)),
            Compose::default().render_optimistic(Some(80)),
        );
    }

    // =====================================================================
    // From implementations
    // =====================================================================

    #[test]
    fn test_from_str() {
        let compose = Compose::from("Hello");
        assert_eq!(compose.render_optimistic(Some(80)), "Hello");
    }

    #[test]
    fn test_from_string() {
        let compose = Compose::from(String::from("Hello"));
        assert_eq!(compose.render_optimistic(Some(80)), "Hello");
    }

    #[test]
    fn test_from_renderable_content_string_variant() {
        let content = RenderableTerminalContent::String("direct".into());
        let compose = Compose::from(content);
        assert_eq!(compose.render_optimistic(Some(80)), "direct");
    }

    #[test]
    fn test_from_renderable_content_component_variant() {
        let content = RenderableTerminalContent::from(Prose::new("component"));
        let compose = Compose::from(content);
        assert!(compose.render_optimistic(Some(80)).contains("component"));
    }

    #[test]
    fn test_from_vec_renderable_content() {
        let items = vec![
            RenderableTerminalContent::from("x"),
            RenderableTerminalContent::from("y"),
        ];
        let compose = Compose::from(items);
        assert_eq!(compose.render_optimistic(Some(80)), "xy");
    }

    #[test]
    fn test_from_empty_str() {
        let compose = Compose::from("");
        assert_eq!(compose.render_optimistic(Some(80)), "");
    }

    #[test]
    fn test_from_empty_string() {
        let compose = Compose::from(String::new());
        assert_eq!(compose.render_optimistic(Some(80)), "");
    }

    // =====================================================================
    // add_text
    // =====================================================================

    #[test]
    fn test_add_text_single() {
        let mut compose = Compose::default();
        compose.add_text("hello");
        assert_eq!(compose.render_optimistic(Some(80)), "hello");
    }

    #[test]
    fn test_add_text_chaining() {
        let mut compose = Compose::default();
        compose.add_text("Hello, ").add_text("world!");
        assert_eq!(compose.render_optimistic(Some(80)), "Hello, world!");
    }

    #[test]
    fn test_add_text_owned_string() {
        let mut compose = Compose::default();
        compose.add_text(String::from("owned"));
        assert_eq!(compose.render_optimistic(Some(80)), "owned");
    }

    #[test]
    fn test_add_text_multiple() {
        let mut compose = Compose::default();
        compose
            .add_text("a")
            .add_text("b")
            .add_text("c")
            .add_text("d");
        assert_eq!(compose.render_optimistic(Some(80)), "abcd");
    }

    // =====================================================================
    // add_prose
    // =====================================================================

    #[test]
    fn test_add_prose_plain() {
        let mut compose = Compose::default();
        compose.add_prose(Prose::new("plain"));
        assert!(compose.render_optimistic(Some(80)).contains("plain"));
    }

    #[test]
    fn test_add_prose_with_bold_tokens() {
        let mut compose = Compose::default();
        compose.add_prose(Prose::new("<bold>bold</bold>"));
        let output = compose.render_optimistic(Some(80));
        assert!(output.contains("\x1b[1m"));
        assert!(output.contains("bold"));
    }

    #[test]
    fn test_add_prose_with_html_tags() {
        let mut compose = Compose::default();
        compose.add_prose(Prose::new("<red>error</red>"));
        let output = compose.render_optimistic(Some(80));
        assert!(output.contains("error"));
    }

    #[test]
    fn test_add_prose_chaining() {
        let mut compose = Compose::default();
        compose
            .add_prose(Prose::new("first"))
            .add_prose(Prose::new("second"));
        let output = compose.render_optimistic(Some(80));
        assert!(output.contains("first"));
        assert!(output.contains("second"));
    }

    // =====================================================================
    // add_unordered_list / add_ordered_list
    // =====================================================================

    #[test]
    fn test_add_unordered_list() {
        let mut compose = Compose::default();
        compose.add_unordered_list(UnorderedList::new(vec!["item1", "item2"]));
        let output = compose.render_optimistic(Some(80));
        assert!(output.contains("item1"));
        assert!(output.contains("item2"));
    }

    #[test]
    fn test_add_ordered_list() {
        let mut compose = Compose::default();
        compose.add_ordered_list(OrderedList::new(vec!["first", "second"]));
        let output = compose.render_optimistic(Some(80));
        assert!(output.contains("first"));
        assert!(output.contains("second"));
    }

    #[test]
    fn test_add_unordered_list_chaining() {
        let mut compose = Compose::default();
        compose
            .add_text("List:\n")
            .add_unordered_list(UnorderedList::new(vec!["a", "b"]));
        let output = compose.render_optimistic(Some(80));
        assert!(output.contains("List:"));
        assert!(output.contains("a"));
        assert!(output.contains("b"));
    }

    #[test]
    fn test_add_ordered_list_chaining() {
        let mut compose = Compose::default();
        compose
            .add_text("Steps:\n")
            .add_ordered_list(OrderedList::new(vec!["do this", "then that"]));
        let output = compose.render_optimistic(Some(80));
        assert!(output.contains("Steps:"));
        assert!(output.contains("do this"));
    }

    // =====================================================================
    // add_file_system
    // =====================================================================

    fn make_fs_fixture() -> (tempfile::TempDir, FileSystem) {
        let temp = tempfile::tempdir().expect("create temp dir");
        std::fs::write(temp.path().join("hello.txt"), "world").expect("create file");
        std::fs::create_dir(temp.path().join("src")).expect("create dir");
        std::fs::write(temp.path().join("src/main.rs"), "fn main() {}").expect("create file");
        let mut fs = FileSystem::new(temp.path()).unwrap();
        fs.ensure_tree_built();
        (temp, fs)
    }

    #[test]
    fn test_add_file_system() {
        let (_tmp, fs) = make_fs_fixture();
        let mut compose = Compose::default();
        compose.add_file_system(fs);
        let output = compose.render_optimistic(Some(80));
        assert!(output.contains("hello.txt"));
        assert!(output.contains("src"));
    }

    #[test]
    fn test_add_file_system_chaining() {
        let (_tmp, fs) = make_fs_fixture();
        let mut compose = Compose::default();
        compose.add_text("Files:\n").add_file_system(fs);
        let output = compose.render_optimistic(Some(80));
        assert!(output.starts_with("Files:\n"));
        assert!(output.contains("hello.txt"));
    }

    #[test]
    fn test_add_file_system_with_prose() {
        let (_tmp, fs) = make_fs_fixture();
        let mut compose = Compose::default();
        compose
            .add_prose(Prose::new("<bold>Directory listing</bold>\n"))
            .add_file_system(fs);
        let output = compose.render_optimistic(Some(80));
        assert!(output.contains("Directory listing"));
        assert!(output.contains("hello.txt"));
    }

    #[test]
    fn test_add_file_system_with_depth() {
        let temp = tempfile::tempdir().expect("create temp dir");
        std::fs::create_dir_all(temp.path().join("a/b")).expect("create dirs");
        std::fs::write(temp.path().join("a/b/deep.txt"), "").expect("create file");
        let mut fs = FileSystem::new(temp.path()).unwrap().depth(1);
        fs.ensure_tree_built();
        let mut compose = Compose::default();
        compose.add_file_system(fs);
        let output = compose.render_optimistic(Some(80));
        // depth(1) shows the first level but not nested deep.txt
        assert!(!output.is_empty());
    }

    // =====================================================================
    // add_table
    // =====================================================================

    #[test]
    fn test_add_table() {
        let table = Table::new()
            .with_columns(vec![TableColumn::new("Name"), TableColumn::new("Value")])
            .with_data(vec![
                vec!["Alice".into(), "100".into()],
                vec!["Bob".into(), "200".into()],
            ]);
        let mut compose = Compose::default();
        compose.add_table(table);
        let output = compose.render_optimistic(Some(80));
        assert!(output.contains("Name"));
        assert!(output.contains("Value"));
        assert!(output.contains("Alice"));
        assert!(output.contains("Bob"));
    }

    #[test]
    fn test_add_table_chaining() {
        let table = Table::new()
            .with_columns(vec![TableColumn::new("Col")])
            .with_data(vec![vec!["data".into()]]);
        let mut compose = Compose::default();
        compose.add_text("Results:\n").add_table(table);
        let output = compose.render_optimistic(Some(80));
        assert!(output.starts_with("Results:\n"));
        assert!(output.contains("Col"));
        assert!(output.contains("data"));
        // Sequence-join must not insert an extra `\n` between the string's
        // trailing newline and the table's top border. `┌` is the actual
        // top-left glyph emitted by `emit_table` in the tree path; assert
        // it appears immediately after the string's trailing newline with
        // no blank line in between.
        assert!(
            output.contains("Results:\n┌"),
            "expected `Results:\\n┌` substring (no blank line before top border); got: {output:?}"
        );
    }

    #[test]
    fn test_string_plus_list_no_extra_newline() {
        // Same byte-level parity check for `String + List`: the trailing
        // newline of the string is preserved and the list's first marker
        // glyph follows immediately, with no inserted blank line.
        let mut compose = Compose::default();
        compose
            .add_text("Items:\n")
            .add_unordered_list(UnorderedList::new(vec!["one", "two"]));
        let output = compose.render_optimistic(Some(80));
        // The default unordered-list marker for tree-path rendering is `- `
        // (CommonMark marker; the bespoke terminal bullet only applies when
        // explicitly set).
        assert!(
            output.contains("Items:\n- ") || output.contains("Items:\n• "),
            "expected `Items:\\n` directly followed by a list marker; got: {output:?}"
        );
    }

    #[test]
    fn test_add_table_with_title() {
        let table = Table::new()
            .with_title("Summary")
            .with_columns(vec![TableColumn::new("Item")])
            .with_data(vec![vec!["test".into()]]);
        let mut compose = Compose::default();
        compose.add_table(table);
        let output = compose.render_optimistic(Some(80));
        assert!(output.contains("Summary"));
        assert!(output.contains("Item"));
    }

    #[test]
    fn test_add_table_empty() {
        let table = Table::new().with_columns(vec![TableColumn::new("Empty")]);
        let mut compose = Compose::default();
        compose.add_table(table);
        let output = compose.render_optimistic(Some(80));
        assert!(output.contains("Empty"));
    }

    #[test]
    fn test_add_table_with_prose() {
        let table = Table::new()
            .with_columns(vec![TableColumn::new("Key"), TableColumn::new("Val")])
            .with_data(vec![vec!["k".into(), "v".into()]]);
        let mut compose = Compose::default();
        compose
            .add_prose(Prose::new("<bold>Table:</bold>\n"))
            .add_table(table);
        let output = compose.render_optimistic(Some(80));
        assert!(output.contains("Table:"));
        assert!(output.contains("Key"));
    }

    // =====================================================================
    // Mixed add methods
    // =====================================================================

    #[test]
    fn test_mixed_text_and_prose() {
        let mut compose = Compose::default();
        compose
            .add_text("normal ")
            .add_prose(Prose::new("styled"))
            .add_text(" normal");
        let output = compose.render_optimistic(Some(80));
        assert!(output.starts_with("normal "));
        assert!(output.contains("styled"));
        assert!(output.ends_with(" normal"));
    }

    #[test]
    fn test_mixed_all_types() {
        let mut compose = Compose::default();
        compose
            .add_text("Header\n")
            .add_prose(Prose::new("description\n"))
            .add_unordered_list(UnorderedList::new(vec!["item"]));
        let output = compose.render_optimistic(Some(80));
        assert!(output.contains("Header"));
        assert!(output.contains("description"));
        assert!(output.contains("item"));
    }

    #[test]
    fn test_mixed_table_and_file_system() {
        let (_tmp, fs) = make_fs_fixture();
        let table = Table::new()
            .with_columns(vec![TableColumn::new("Metric")])
            .with_data(vec![vec!["count".into()]]);
        let mut compose = Compose::default();
        compose.add_table(table).add_text("\n").add_file_system(fs);
        let output = compose.render_optimistic(Some(80));
        assert!(output.contains("Metric"));
        assert!(output.contains("count"));
        assert!(output.contains("hello.txt"));
    }

    #[test]
    fn test_from_then_add() {
        let mut compose = Compose::from("start");
        compose.add_text(" end");
        assert_eq!(compose.render_optimistic(Some(80)), "start end");
    }

    // =====================================================================
    // No newlines between items (core behavior)
    // =====================================================================

    #[test]
    fn test_no_newlines_between_string_items() {
        let mut compose = Compose::default();
        compose.add_text("a").add_text("b").add_text("c");
        let output = compose.render_optimistic(Some(80));
        assert!(!output.contains('\n'));
        assert_eq!(output, "abc");
    }

    #[test]
    fn test_no_newlines_between_mixed_items() {
        let mut compose = Compose::default();
        compose.add_text("text").add_prose(Prose::new("prose"));
        let output = compose.render_optimistic(Some(80));
        assert!(!output.contains('\n'));
    }

    #[test]
    fn test_concatenation_preserves_spaces() {
        let mut compose = Compose::default();
        compose.add_text("hello ").add_text("world");
        assert_eq!(compose.render_optimistic(Some(80)), "hello world");
    }

    // =====================================================================
    // TerminalRenderable trait — render / render_optimistic
    // =====================================================================

    #[test]
    fn test_render_with_explicit_width() {
        let compose = Compose::from("test");
        assert_eq!(compose.render_optimistic(Some(120)), "test");
    }

    #[test]
    fn test_render_with_none_width() {
        let compose = Compose::from("test");
        assert_eq!(compose.render_optimistic(None), "test");
    }

    #[test]
    fn test_render() {
        let mut compose = Compose::default();
        compose.add_text("hello ").add_text("world");
        let term = Terminal::new_optimistic(80);
        assert_eq!(compose.render(&term), "hello world");
    }

    #[test]
    fn test_render_and_render_optimistic_consistent_for_plain_text() {
        let mut compose = Compose::default();
        compose.add_text("hello ").add_text("world");
        let term = Terminal::new_optimistic(80);
        assert_eq!(compose.render_optimistic(Some(80)), compose.render(&term),);
    }

    #[test]
    fn test_render_empty() {
        assert_eq!(Compose::default().render_optimistic(Some(80)), "");
    }

    #[test]
    fn test_render_empty_with_terminal() {
        let term = Terminal::new_optimistic(80);
        assert_eq!(Compose::default().render(&term), "");
    }

    // =====================================================================
    // TerminalRenderable trait — display
    // =====================================================================

    #[test]
    fn test_display_adds_newline() {
        let compose = Compose::from("no newline");
        let term = Terminal::new_optimistic(80);
        let output = compose.display(&term);
        assert!(output.ends_with('\n'));
        assert_eq!(output, "no newline\n");
    }

    #[test]
    fn test_display_does_not_double_newline() {
        let compose = Compose::from("has newline\n");
        let term = Terminal::new_optimistic(80);
        let output = compose.display(&term);
        assert!(output.ends_with('\n'));
        assert!(!output.ends_with("\n\n"));
        assert_eq!(output, "has newline\n");
    }

    #[test]
    fn test_display_empty_produces_newline() {
        let compose = Compose::default();
        let term = Terminal::new_optimistic(80);
        assert_eq!(compose.display(&term), "\n");
    }

    // =====================================================================
    // TerminalRenderable trait — is_block_level
    // =====================================================================

    #[test]
    fn test_is_not_block_level() {
        assert!(!Compose::default().is_block_level());
    }

    #[test]
    fn test_is_not_block_level_with_content() {
        let compose = Compose::from("content");
        assert!(!compose.is_block_level());
    }

    // =====================================================================
    // TerminalRenderable trait — layout builder methods
    // =====================================================================

    #[test]
    fn test_layout_access() {
        let mut compose = Compose::default();
        compose.layout_mut().alignment = Alignment::Center;
        assert_eq!(compose.layout().alignment, Alignment::Center);
    }

    #[test]
    fn test_left_margin_builder() {
        let compose = Compose::from("test").left_margin(TargetValue::universal(Length::ch(4)));
        assert_eq!(
            compose.layout().margin.left,
            TargetValue::universal(Length::ch(4))
        );
    }

    #[test]
    fn test_right_margin_builder() {
        let compose = Compose::from("test").right_margin(TargetValue::universal(Length::ch(4)));
        assert_eq!(
            compose.layout().margin.right,
            TargetValue::universal(Length::ch(4))
        );
    }

    #[test]
    fn test_top_margin_builder() {
        let compose = Compose::from("test").top_margin(TargetValue::universal(Length::ch(2)));
        assert_eq!(
            compose.layout().margin.top,
            TargetValue::universal(Length::ch(2))
        );
    }

    #[test]
    fn test_bottom_margin_builder() {
        let compose = Compose::from("test").bottom_margin(TargetValue::universal(Length::ch(2)));
        assert_eq!(
            compose.layout().margin.bottom,
            TargetValue::universal(Length::ch(2))
        );
    }

    #[test]
    fn test_alignment_builder() {
        let compose = Compose::from("test").alignment(Alignment::Right);
        assert_eq!(compose.layout().alignment, Alignment::Right);
    }

    #[test]
    fn test_word_wrap_builder() {
        let compose = Compose::from("test").word_wrap(WordWrap::None);
        assert_eq!(compose.layout().word_wrap, WordWrap::None);
    }

    #[test]
    fn test_chained_layout_builders() {
        let compose = Compose::from("test")
            .left_margin(TargetValue::universal(Length::ch(2)))
            .right_margin(TargetValue::universal(Length::ch(2)))
            .alignment(Alignment::Center);
        assert_eq!(
            compose.layout().margin.left,
            TargetValue::universal(Length::ch(2))
        );
        assert_eq!(
            compose.layout().margin.right,
            TargetValue::universal(Length::ch(2))
        );
        assert_eq!(compose.layout().alignment, Alignment::Center);
    }

    // =====================================================================
    // Layout parity — rendered terminal output across margin/alignment/
    // max_width/word_wrap. These pin the tree path's layout behavior at a
    // known width so a regression in `render_with_layout` is caught here
    // and not at a downstream component.
    // =====================================================================

    /// Renders at a fixed terminal width with `is_block_level` lifted via a
    /// trailing `\n` part so the layout pipeline applies horizontal margins
    /// and alignment to the rendered output. Compose itself reports
    /// `is_block_level() == false`, so we go through `render(&Terminal)` with
    /// a known-width optimistic terminal to get deterministic output.
    fn render_at(compose: &Compose, width: u32) -> String {
        let term = Terminal::new_optimistic(width);
        compose.render(&term)
    }

    #[test]
    fn test_layout_left_margin_indents_content() {
        let compose = Compose::from("hi").left_margin(TargetValue::universal(Length::ch(4)));
        let out = render_at(&compose, 20);
        // Each rendered line begins with four leading spaces.
        let first_line = out.lines().next().unwrap_or("");
        assert!(
            first_line.starts_with("    hi"),
            "expected 4-space indent, got {first_line:?}"
        );
    }

    #[test]
    fn test_layout_right_margin_does_not_overflow_content() {
        // Compose's sequence join emits verbatim text; word wrap only kicks
        // in for prose-bearing parts (inline kinds). For a single short
        // string part the rendered output should still respect the left
        // margin and not extend past the terminal's available width
        // (terminal width - right margin reduces inner width).
        let compose = Compose::from("hi")
            .left_margin(TargetValue::universal(Length::ch(2)))
            .right_margin(TargetValue::universal(Length::ch(4)));
        let out = render_at(&compose, 20);
        let line = out.lines().next().unwrap_or("");
        // The left margin contributes 2 leading spaces; the visible run is
        // `hi` (2 chars) → total width = 4, well within the 20-4 = 16 cell
        // budget. We don't check the right edge directly (Compose does not
        // pad to the right edge), but we do check that the left margin is
        // honored.
        assert!(
            line.starts_with("  hi"),
            "expected 2-space indent + `hi`, got: {line:?}"
        );
    }

    /// Builds a Compose with `max_width` applied directly to its layout —
    /// the trait's builder chain does not expose `max_width`, so we mutate
    /// the layout in-place. Mirrors `layout_mut()` usage elsewhere.
    fn compose_with_max_width(text: &str, ch: u32) -> Compose {
        let mut compose = Compose::from(text);
        compose.layout_mut().max_width = Some(TargetValue::universal(Length::ch(ch)));
        compose
    }

    /// Leading spaces of the first non-empty line (the alignment offset).
    fn leading_spaces(out: &str) -> usize {
        let line = out.lines().find(|l| !l.is_empty()).unwrap_or("");
        line.len() - line.trim_start().len()
    }

    #[test]
    fn test_layout_center_alignment_adds_leading_space() {
        // `max_width: 10` makes the box sub-available; center alignment then
        // places the 10-cell box within the 40-cell terminal (`margin:auto`
        // semantics): slack = 40 − 10 = 30, center → 15 leading spaces.
        let compose = compose_with_max_width("hi", 10).alignment(Alignment::Center);
        let out = render_at(&compose, 40);
        assert_eq!(
            leading_spaces(&out),
            15,
            "center under max_width=10 centers the box within 40: {out:?}"
        );
    }

    #[test]
    fn test_layout_right_alignment_pushes_content() {
        // Right alignment pushes the 10-cell box to the right edge of the
        // 40-cell terminal: slack = 30 → 30 leading spaces.
        let compose = compose_with_max_width("hi", 10).alignment(Alignment::Right);
        let out = render_at(&compose, 40);
        assert_eq!(
            leading_spaces(&out),
            30,
            "right under max_width=10 pushes the box to the right edge: {out:?}"
        );
    }

    #[test]
    fn test_layout_max_width_with_alignment_is_observable_for_short_content() {
        // The sub-available box is placed within the available width even for
        // short content: center → (40 − 10) / 2 = 15 leading spaces.
        let compose = compose_with_max_width("hi", 10).alignment(Alignment::Center);
        let out = render_at(&compose, 40);
        assert_eq!(
            leading_spaces(&out),
            15,
            "center under max_width=10 centers the box within 40: {out:?}"
        );
    }

    #[test]
    fn test_layout_word_wrap_records_on_node() {
        // Verbatim text parts in a sequence-join Root render literally and
        // do not get rewrapped — that is Compose's defining contract.
        // What we DO assert is that the Layout's `word_wrap` is correctly
        // seeded on the projected sequence container so downstream
        // consumers (paragraph/prose nodes nested inside) can opt in.
        let compose =
            Compose::from("alphabet soup is tasty").word_wrap(WordWrap::WrapProse(None, None));
        let node = TreeRenderable::render_tree(&compose);
        let layout = node.attrs.layout().expect("layout recorded");
        assert_eq!(layout.word_wrap, WordWrap::WrapProse(None, None));
    }

    #[test]
    fn test_layout_word_wrap_none_renders_verbatim() {
        // Verify the explicit `None` policy still renders the string
        // verbatim (no inserted wrap points).
        let compose = Compose::from("alphabet soup is tasty").word_wrap(WordWrap::None);
        let out = render_at(&compose, 12);
        // Sequence-join Text is emitted verbatim — no wrap-induced line
        // breaks, regardless of terminal width.
        assert!(
            out.contains("alphabet soup is tasty"),
            "verbatim string must survive intact, got {out:?}"
        );
    }

    // =====================================================================
    // TerminalRenderable trait — as_any / Debug
    // =====================================================================

    #[test]
    fn test_as_any_downcast() {
        let compose = Compose::from("test");
        let any_ref = TerminalRenderable::as_any(&compose);
        assert!(any_ref.downcast_ref::<Compose>().is_some());
    }

    #[test]
    fn test_as_any_wrong_type() {
        let compose = Compose::from("test");
        let any_ref = TerminalRenderable::as_any(&compose);
        assert!(any_ref.downcast_ref::<Prose>().is_none());
    }

    #[test]
    fn test_debug_output() {
        let compose = Compose::from("debug me");
        let debug = format!("{:?}", compose);
        assert!(debug.contains("Compose"));
    }

    #[test]
    fn test_debug_shows_parts() {
        let mut compose = Compose::default();
        compose.add_text("a").add_text("b");
        let debug = format!("{:?}", compose);
        assert!(debug.contains("Compose"));
        assert!(debug.contains("parts"));
    }

    // =====================================================================
    // Edge cases — unicode, emoji, special characters
    // =====================================================================

    #[test]
    fn test_unicode_content() {
        let mut compose = Compose::default();
        compose.add_text("Hello ").add_text("世界");
        assert_eq!(compose.render_optimistic(Some(80)), "Hello 世界");
    }

    #[test]
    fn test_emoji_content() {
        let mut compose = Compose::default();
        compose.add_text("Status: ").add_text("✅");
        assert_eq!(compose.render_optimistic(Some(80)), "Status: ✅");
    }

    #[test]
    fn test_mixed_unicode_scripts() {
        let mut compose = Compose::default();
        compose
            .add_text("English")
            .add_text(" • ")
            .add_text("日本語")
            .add_text(" • ")
            .add_text("العربية");
        let output = compose.render_optimistic(Some(80));
        assert!(output.contains("English"));
        assert!(output.contains("日本語"));
        assert!(output.contains("العربية"));
    }

    // =====================================================================
    // Edge cases — empty and whitespace items
    // =====================================================================

    #[test]
    fn test_empty_string_items() {
        let mut compose = Compose::default();
        compose.add_text("").add_text("content").add_text("");
        assert_eq!(compose.render_optimistic(Some(80)), "content");
    }

    #[test]
    fn test_whitespace_only_items() {
        let mut compose = Compose::default();
        compose.add_text("   ").add_text("text").add_text("   ");
        assert_eq!(compose.render_optimistic(Some(80)), "   text   ");
    }

    #[test]
    fn test_single_character() {
        let compose = Compose::from("X");
        assert_eq!(compose.render_optimistic(Some(80)), "X");
    }

    #[test]
    fn test_tab_characters() {
        let mut compose = Compose::default();
        compose.add_text("col1").add_text("\t").add_text("col2");
        assert_eq!(compose.render_optimistic(Some(80)), "col1\tcol2");
    }

    // =====================================================================
    // Edge cases — many items
    // =====================================================================

    #[test]
    fn test_many_items() {
        let mut compose = Compose::default();
        for i in 0..100 {
            compose.add_text(i.to_string());
        }
        let output = compose.render_optimistic(Some(1000));
        assert!(output.starts_with("0123"));
        assert!(output.ends_with("99"));
    }

    // =====================================================================
    // Prose styling integration
    // =====================================================================

    #[test]
    fn test_prose_bold_renders_inline() {
        let mut compose = Compose::default();
        compose
            .add_text("normal ")
            .add_prose(Prose::new("<bold>bold</bold>"))
            .add_text(" normal");
        let output = compose.render_optimistic(Some(80));
        assert!(output.contains("\x1b[1m"));
        assert!(output.contains("bold"));
        assert!(output.starts_with("normal "));
        assert!(output.ends_with(" normal"));
    }

    #[test]
    fn test_multiple_styled_prose() {
        let mut compose = Compose::default();
        compose
            .add_prose(Prose::new("<bold>key</bold>"))
            .add_text(": ")
            .add_prose(Prose::new("<dim>value</dim>"));
        let output = compose.render_optimistic(Some(80));
        assert!(output.contains("key"));
        assert!(output.contains(": "));
        assert!(output.contains("value"));
    }

    // =====================================================================
    // Nesting — component interop
    // =====================================================================

    #[test]
    fn test_compose_inside_compose() {
        let mut inner = Compose::default();
        inner.add_text("inner");
        // Use From<RenderableTerminalContent> to wrap inner Compose
        let content = RenderableTerminalContent::from(inner);
        let mut outer = Compose::from(content);
        outer.add_text(" after");
        let output = outer.render_optimistic(Some(80));
        assert!(output.contains("inner"));
        assert!(output.contains(" after"));
    }

    #[test]
    fn test_inline_content_inside_compose() {
        let inline = InlineContent::default().with("a").with("b");
        let content = RenderableTerminalContent::from(inline);
        let mut compose = Compose::from(content);
        compose.add_text(" end");
        assert_eq!(compose.render_optimistic(Some(80)), "ab end");
    }

    #[test]
    fn test_text_block_via_renderable_content() {
        let block = TextBlock::new("styled");
        let content = RenderableTerminalContent::from(block);
        let compose = Compose::from(content);
        assert!(compose.render_optimistic(Some(80)).contains("styled"));
    }

    // =====================================================================
    // TreeRenderable — projection shape
    // =====================================================================

    use renderable::tree::{NodeKind, SequenceJoin};

    fn strip_ansi(s: &str) -> String {
        crate::discovery::eval::strip_ansi_codes(s)
    }

    #[test]
    fn test_render_tree_root_has_sequence_join_none() {
        let compose: Compose = "x".into();
        let node = TreeRenderable::render_tree(&compose);
        assert!(matches!(node.kind, NodeKind::Root { .. }));
        assert_eq!(node.attrs.sequence_join(), Some(SequenceJoin::None));
    }

    #[test]
    fn test_render_tree_empty_compose_has_no_children() {
        let node = TreeRenderable::render_tree(&Compose::default());
        assert!(node.children().is_empty());
    }

    #[test]
    fn test_render_tree_string_part_becomes_text_node() {
        let compose: Compose = "hello".into();
        let node = TreeRenderable::render_tree(&compose);
        assert_eq!(node.children().len(), 1);
        assert!(matches!(
            &node.children()[0].kind,
            NodeKind::Text { value } if value == "hello"
        ));
    }

    #[test]
    fn test_render_tree_two_strings_become_two_text_nodes() {
        let compose = Compose::new(vec![
            RenderableTerminalContent::from("foo"),
            RenderableTerminalContent::from("bar"),
        ]);
        let node = TreeRenderable::render_tree(&compose);
        assert_eq!(node.children().len(), 2);
    }

    #[test]
    fn test_render_tree_nested_compose_inlines_inner_children() {
        // Nested `Root` is invalid; the inner sequence's children must be
        // hoisted into the outer sequence so the document validates.
        let mut inner = Compose::default();
        inner.add_text("inner");
        let outer = Compose::new(vec![
            RenderableTerminalContent::from(inner),
            RenderableTerminalContent::from(" after"),
        ]);
        let node = TreeRenderable::render_tree(&outer);
        for child in node.children() {
            assert!(
                !matches!(child.kind, NodeKind::Root { .. }),
                "nested Root must be inlined; got {:?}",
                child.kind
            );
        }
    }

    #[test]
    fn test_render_tree_records_non_default_layout() {
        let compose = Compose::from("x").alignment(Alignment::Center);
        let node = TreeRenderable::render_tree(&compose);
        assert!(node.attrs.layout().is_some());
    }

    #[test]
    fn test_render_tree_omits_default_layout() {
        let compose = Compose::from("x");
        let node = TreeRenderable::render_tree(&compose);
        assert!(node.attrs.layout().is_none());
    }

    #[test]
    fn test_render_tree_node_returns_some() {
        let compose: Compose = "x".into();
        let node = TerminalRenderable::render_tree_node(&compose);
        assert!(node.is_some());
    }

    /// Strictness regression: render_terminal_node should surface an error
    /// when a Compose-shaped sequence container holds a child the terminal
    /// renderer cannot lower (raw HTML). Compose itself routes through
    /// `RenderStrictness::Warn` (so user-facing output is robust), but
    /// callers that hand-build a tree from `render_tree_with_terminal` and
    /// invoke `render_terminal_node` directly under `Strict` MUST get back
    /// a visible failure rather than silently dropping the node.
    #[test]
    fn test_render_terminal_node_strict_rejects_unsupported_child() {
        let compose = Compose::from("ok ");
        // Real tree projection for Compose, then graft a synthetic raw-HTML
        // node onto the sequence container. Raw HTML has no terminal
        // lowering and is rejected by the strictness gate.
        let mut node = compose.render_tree_with_terminal(None);
        match &mut node.kind {
            renderable::tree::NodeKind::Root { children } => {
                children.push(RenderNode::html("<x>", false));
            }
            other => panic!("expected Root for Compose projection, got {other:?}"),
        }
        let term = Terminal::new_optimistic(80);
        let opts = TerminalRenderOptions::new(&term, RenderStrictness::Strict);
        let result = crate::render_tree::render_terminal_node(&node, &opts);
        assert!(
            result.is_err(),
            "Strict mode must reject the unsupported child; got Ok: {:?}",
            result.map(|r| r.output)
        );
    }

    // =====================================================================
    // Terminal — sequence-join concatenation parity
    // =====================================================================

    #[test]
    fn test_terminal_two_strings_concatenate_without_separator() {
        // Exact byte parity: no blank line, no inserted space.
        let compose = Compose::new(vec![
            RenderableTerminalContent::from("foo"),
            RenderableTerminalContent::from("bar"),
        ]);
        assert_eq!(compose.render_optimistic(Some(80)), "foobar");
    }

    #[test]
    fn test_terminal_three_strings_with_explicit_newline() {
        let compose = Compose::new(vec![
            RenderableTerminalContent::from("a"),
            RenderableTerminalContent::from("\n"),
            RenderableTerminalContent::from("b"),
        ]);
        assert_eq!(compose.render_optimistic(Some(80)), "a\nb");
    }

    #[test]
    fn test_terminal_string_plus_section_no_blank_line_before_heading() {
        let mut compose = Compose::default();
        compose.add_text("Before ").add_heading("Title", 1);
        let output = compose.render_optimistic(Some(80));
        let stripped = strip_ansi(&output);
        // The string and the heading are joined with no inserted blank line.
        // Concrete check: the heading line follows the prefix without any
        // intervening `\n\n`.
        assert!(stripped.starts_with("Before # Title"));
    }

    #[test]
    fn test_terminal_section_plus_string_no_blank_line_after() {
        let mut compose = Compose::default();
        compose.add_heading("Title", 2).add_text(" trailing");
        let stripped = strip_ansi(&compose.render_optimistic(Some(80)));
        // Heading is concatenated with the trailing text and no blank line
        // separator is inserted.
        assert!(stripped.contains("## Title trailing"));
    }

    // =====================================================================
    // Markdown — sequence-join concatenation
    // =====================================================================

    #[test]
    fn test_markdown_empty_compose_is_empty_string() {
        assert_eq!(Compose::default().render_markdown(), "");
    }

    #[test]
    fn test_markdown_single_string() {
        let compose: Compose = "hello".into();
        assert_eq!(compose.render_markdown(), "hello");
    }

    #[test]
    fn test_markdown_two_strings_concatenate_without_separator() {
        let compose = Compose::new(vec![
            RenderableTerminalContent::from("foo"),
            RenderableTerminalContent::from("bar"),
        ]);
        assert_eq!(compose.render_markdown(), "foobar");
    }

    #[test]
    fn test_markdown_three_strings_with_explicit_newline() {
        let compose = Compose::new(vec![
            RenderableTerminalContent::from("a"),
            RenderableTerminalContent::from("\n"),
            RenderableTerminalContent::from("b"),
        ]);
        assert_eq!(compose.render_markdown(), "a\nb");
    }

    #[test]
    fn test_markdown_string_plus_heading_no_blank_line() {
        let mut compose = Compose::default();
        compose.add_text("intro ").add_heading("Title", 1);
        // Concatenated: `intro ` directly followed by `# Title`, with no
        // blank line in between.
        assert_eq!(compose.render_markdown(), "intro # Title");
    }

    #[test]
    fn test_markdown_string_plus_list() {
        let mut compose = Compose::default();
        compose
            .add_text("Items:\n")
            .add_unordered_list(UnorderedList::new(vec!["one", "two"]));
        let md = compose.render_markdown();
        assert!(md.starts_with("Items:\n"));
        assert!(md.contains("- one"));
        assert!(md.contains("- two"));
    }

    #[test]
    fn test_markdown_layout_has_no_effect() {
        let plain = Compose::from("text");
        let laid_out = Compose::from("text")
            .alignment(Alignment::Center)
            .left_margin(TargetValue::universal(Length::ch(4)));
        assert_eq!(plain.render_markdown(), laid_out.render_markdown());
    }

    #[test]
    fn test_markdown_plus_matches_markdown_for_plain_content() {
        let mut compose = Compose::default();
        compose
            .add_text("intro\n")
            .add_heading("Title", 2)
            .add_text(" tail");
        assert_eq!(compose.render_markdown(), compose.render_markdown_plus());
    }

    #[test]
    fn test_markdown_nested_compose_preserves_inner_sequence() {
        let mut inner = Compose::default();
        inner.add_text("xy");
        let outer = Compose::new(vec![
            RenderableTerminalContent::from(inner),
            RenderableTerminalContent::from("z"),
        ]);
        assert_eq!(outer.render_markdown(), "xyz");
    }

    // =====================================================================
    // Browser — HTML fragment
    // =====================================================================

    #[test]
    fn test_browser_empty_compose() {
        let html =
            renderable::browser::BrowserRenderable::render_html_fragment(&Compose::default())
                .render();
        // An empty sequence produces an empty wrapper div.
        assert!(html.contains("<div") || html.is_empty());
    }

    #[test]
    fn test_browser_single_string_wrapped_in_div() {
        let compose: Compose = "hello".into();
        let html = renderable::browser::BrowserRenderable::render_html_fragment(&compose).render();
        assert!(html.contains("hello"));
        assert!(html.contains("<div"));
    }

    #[test]
    fn test_browser_two_strings_appear_in_order_no_textual_separator() {
        let compose = Compose::new(vec![
            RenderableTerminalContent::from("foo"),
            RenderableTerminalContent::from("bar"),
        ]);
        let html = renderable::browser::BrowserRenderable::render_html_fragment(&compose).render();
        // The two text nodes appear in DOM order with no inserted text
        // separator between them.
        let foo_idx = html.find("foo").expect("foo present");
        let bar_idx = html.find("bar").expect("bar present");
        assert!(foo_idx < bar_idx);
        // Any characters between the foo and bar literals must be markup
        // (start with `<`), not text content.
        let between = &html[foo_idx + "foo".len()..bar_idx];
        assert!(
            between.is_empty() || between.starts_with('<'),
            "unexpected text between text nodes: {between:?}"
        );
    }

    #[test]
    fn test_browser_string_plus_section_emits_heading_after_string() {
        let mut compose = Compose::default();
        compose.add_text("Intro").add_heading("Title", 2);
        let html = renderable::browser::BrowserRenderable::render_html_fragment(&compose).render();
        let intro_idx = html.find("Intro").expect("Intro present");
        let heading_idx = html.find("<h2").expect("<h2 present");
        assert!(intro_idx < heading_idx);
    }

    #[test]
    fn test_browser_string_plus_list_emits_ul() {
        let mut compose = Compose::default();
        compose
            .add_text("Items")
            .add_unordered_list(UnorderedList::new(vec!["a", "b"]));
        let html = renderable::browser::BrowserRenderable::render_html_fragment(&compose).render();
        assert!(html.contains("Items"));
        assert!(html.contains("<ul"));
        assert!(html.contains("a"));
        assert!(html.contains("b"));
    }

    #[test]
    fn test_browser_render_html_page_includes_fragment() {
        let compose: Compose = "hello".into();
        let page = renderable::browser::BrowserRenderable::render_html_page(&compose, None);
        let html = page.render().expect("render");
        assert!(html.contains("<html"));
        assert!(html.contains("<body>"));
        assert!(html.contains("hello"));
    }

    #[test]
    fn test_browser_as_any_downcast() {
        let compose: Compose = "x".into();
        let any = renderable::browser::BrowserRenderable::as_any(&compose);
        assert!(any.downcast_ref::<Compose>().is_some());
    }

    // =====================================================================
    // Many-part / Unicode / Prose styling preservation (regression pins)
    // =====================================================================

    #[test]
    fn test_terminal_many_parts_concatenate_correctly() {
        let mut compose = Compose::default();
        for ch in 'a'..='z' {
            compose.add_text(ch.to_string());
        }
        let expected: String = ('a'..='z').collect();
        assert_eq!(compose.render_optimistic(Some(200)), expected);
    }

    #[test]
    fn test_terminal_unicode_concatenates_verbatim() {
        let compose = Compose::new(vec![
            RenderableTerminalContent::from("Hello "),
            RenderableTerminalContent::from("世界 🌍"),
        ]);
        assert_eq!(compose.render_optimistic(Some(80)), "Hello 世界 🌍");
    }

    /// Regression: a Prose part inside Compose must keep its inline `<b>`
    /// styling through the tree path. Without the dedicated Prose downcast
    /// the generic fallback would strip ANSI to plain text.
    #[test]
    fn test_terminal_prose_bold_styling_survives_through_tree() {
        let mut compose = Compose::default();
        compose
            .add_text("plain ")
            .add_prose(Prose::new("<b>bold</b>"))
            .add_text(" tail");
        let rendered = compose.render_optimistic(Some(80));
        assert!(
            rendered.contains("\x1b[1m"),
            "expected SGR bold open in: {rendered:?}"
        );
        let stripped = strip_ansi(&rendered);
        assert!(stripped.contains("plain "));
        assert!(stripped.contains("bold"));
        assert!(stripped.contains(" tail"));
    }
}
