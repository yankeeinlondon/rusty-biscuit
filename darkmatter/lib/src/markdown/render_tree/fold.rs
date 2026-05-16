//! Stack-based fold of a `pulldown-cmark` event stream into a render tree.
//!
//! [`fold_markdown_to_document`] walks a [`pulldown_cmark`] 0.13 event stream
//! and builds a canonical [`renderable::tree::Document`]. It is the minimal
//! Milestone 1 fold: it covers the common CommonMark + GFM subset (paragraphs,
//! headings, lists, tables, code, emphasis, links, images, breaks, raw HTML and
//! task lists). Footnotes, custom darkmatter inline styles, attributed
//! horizontal rules and full raw-HTML grouping are intentionally out of scope.
//!
//! The fold is **total**: every event the parser can emit either becomes a node
//! or produces a [`Diagnostic`] — no event is silently dropped. The
//! [`inventory`](super::inventory) module drives the unsupported/lossy
//! classification.
//!
//! ## How container spans are computed
//!
//! Leaf nodes (`Text`, `Code`, breaks, `Rule`, raw HTML) take their
//! [`renderable::tree::SourceLocation`] byte range straight from
//! `into_offset_iter`. Container nodes record the byte range of their
//! `Event::Start` … `Event::End` pair: the start offset is captured when the
//! start event is seen and the end offset when the matching end event arrives.
//! A container with a concrete range carries [`Provenance::Parsed`]. The
//! synthetic [`NodeKind::Root`] keeps a synthetic span with no location.

use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use renderable::tree::{
    ColumnAlign, Diagnostic, Document, DocumentMetadata, HeadingDepth, NodeKind, Provenance,
    RenderNode, SourceDescriptor, SourceId, SourceLocation, SourceSpan,
};
use std::ops::Range;

use super::source::single_source_registry;

/// A single in-progress container on the fold stack.
struct Frame {
    /// The container tag being built.
    tag: ContainerKind,
    /// Children accumulated so far.
    children: Vec<RenderNode>,
    /// Byte offset of the container's `Event::Start`.
    start: usize,
}

/// The container kinds the fold pushes onto its stack.
///
/// This is a small fold-local mirror of the `Tag` variants that open a
/// container; leaf tags never reach the stack.
enum ContainerKind {
    /// The synthetic document root; never closed by an end event.
    Root,
    Paragraph,
    Heading(HeadingDepth),
    BlockQuote,
    CodeBlock {
        lang: Option<String>,
        meta: Option<String>,
    },
    List {
        ordered: bool,
        start: Option<u64>,
    },
    /// A list item; `checked` is set if a `TaskListMarker` is seen inside it.
    Item {
        checked: Option<bool>,
    },
    Table(Vec<ColumnAlign>),
    TableHead,
    TableRow,
    TableCell,
    Emphasis,
    Strong,
    Delete,
    /// `Span` is used for lossy super/subscript folds.
    Span,
    Link {
        url: String,
        title: Option<String>,
    },
    Image {
        url: String,
        title: Option<String>,
    },
    /// A container that should be discarded on close, emitting an `Unsupported`
    /// node with the given label plus a diagnostic.
    Unsupported {
        label: String,
    },
    /// `Tag::HtmlBlock` — a pure structural wrapper whose children (the
    /// `Event::Html` lines) are spliced directly into the parent on close.
    HtmlBlock,
}

/// The mutable state threaded through the fold.
struct Fold {
    /// Container stack; the last entry is the innermost open container.
    stack: Vec<Frame>,
    /// The single source every parsed node refers to.
    source: SourceId,
    /// Diagnostics accumulated during the fold.
    diagnostics: Vec<Diagnostic>,
}

impl Fold {
    /// Builds a [`SourceSpan`] for a parsed node spanning `range`.
    fn parsed_span(&self, range: Range<usize>) -> SourceSpan {
        SourceSpan {
            provenance: Provenance::Parsed,
            location: Some(SourceLocation {
                source: self.source,
                bytes: range,
            }),
        }
    }

    /// Appends a node to the innermost open container's children.
    ///
    /// The root [`Frame`] is always present, so this never silently loses a
    /// node in a well-formed stream.
    fn push_child(&mut self, node: RenderNode) {
        if let Some(frame) = self.stack.last_mut() {
            frame.children.push(node);
        }
    }

    /// Appends a leaf node carrying a parsed span over `range`.
    fn push_leaf(&mut self, kind_node: RenderNode, range: Range<usize>) {
        let span = self.parsed_span(range);
        let mut node = kind_node;
        node.span = span;
        self.push_child(node);
    }
}

/// Folds a Markdown string into a canonical [`renderable::tree::Document`].
///
/// The input is parsed with `ENABLE_TABLES | ENABLE_STRIKETHROUGH |
/// ENABLE_TASKLISTS`. Task lists are enabled (unlike darkmatter's existing
/// render path) because the render tree models `ListItem.checked`; the other
/// extensions (footnotes, math, definition lists, super/subscript, metadata
/// blocks) stay off for Milestone 1.
///
/// Frontmatter is **not** populated: darkmatter extracts frontmatter before the
/// parser ever sees the content, and the chosen options do not enable metadata
/// blocks, so [`DocumentMetadata::frontmatter`] is always `None` here. A later
/// phase wires real frontmatter through.
///
/// ## Returns
///
/// The folded [`Document`] and any non-fatal [`Diagnostic`]s raised while
/// folding (unsupported variants, lossy conversions, malformed structure).
/// Malformed input never panics.
#[must_use]
pub fn fold_markdown_to_document(
    source: SourceDescriptor,
    input: &str,
) -> (Document, Vec<Diagnostic>) {
    let (registry, source_id) = single_source_registry(source);

    // Milestone 1 option set: tables + strikethrough match darkmatter's render
    // path, plus task lists so `ListItem.checked` can be populated. Footnotes,
    // math, definition lists, super/subscript and metadata blocks stay off.
    let options =
        Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS;

    let mut fold = Fold {
        stack: vec![Frame {
            tag: ContainerKind::Root,
            children: Vec::new(),
            start: 0,
        }],
        source: source_id,
        diagnostics: Vec::new(),
    };

    for (event, range) in Parser::new_ext(input, options).into_offset_iter() {
        match event {
            Event::Start(tag) => fold.start(tag, range),
            Event::End(tag_end) => fold.end(&tag_end, range),
            Event::Text(text) => {
                fold.push_leaf(RenderNode::text(text.into_string()), range);
            }
            Event::Code(code) => {
                fold.push_leaf(RenderNode::inline_code(code.into_string()), range);
            }
            Event::Html(html) => {
                fold.push_leaf(RenderNode::html(html.into_string(), true), range);
            }
            Event::InlineHtml(html) => {
                fold.push_leaf(RenderNode::html(html.into_string(), false), range);
            }
            Event::SoftBreak => fold.push_leaf(RenderNode::soft_break(), range),
            Event::HardBreak => fold.push_leaf(RenderNode::hard_break(), range),
            Event::Rule => fold.push_leaf(RenderNode::thematic_break(), range),
            Event::TaskListMarker(checked) => fold.task_marker(checked, range),
            // Disabled by the chosen options, but the fold must stay total.
            Event::FootnoteReference(name) => {
                let label = format!("footnote reference [{name}]");
                fold.push_leaf(RenderNode::unsupported(label.clone()), range.clone());
                fold.diagnostics
                    .push(Diagnostic::unsupported(label, Some(fold.parsed_span(range))));
            }
            Event::InlineMath(_) | Event::DisplayMath(_) => {
                let label = "math expression".to_string();
                fold.push_leaf(RenderNode::unsupported(label.clone()), range.clone());
                fold.diagnostics
                    .push(Diagnostic::unsupported(label, Some(fold.parsed_span(range))));
            }
        }
    }

    // Drain remaining frames. Only the root should remain; if the parser left
    // containers open (malformed), splice their children upward and diagnose.
    while fold.stack.len() > 1 {
        let frame = fold.stack.pop().expect("len checked");
        fold.diagnostics.push(Diagnostic::structural(
            "unclosed container in event stream",
            None,
        ));
        if let Some(parent) = fold.stack.last_mut() {
            parent.children.extend(frame.children);
        }
    }

    let root_children = fold.stack.pop().map(|f| f.children).unwrap_or_default();
    let document = Document {
        sources: registry,
        metadata: DocumentMetadata::default(),
        root: RenderNode::root(root_children),
    };
    (document, fold.diagnostics)
}

impl Fold {
    /// Handles an `Event::Start`, pushing a container frame.
    fn start(&mut self, tag: Tag<'_>, range: Range<usize>) {
        let kind = match tag {
            Tag::Paragraph => ContainerKind::Paragraph,
            Tag::Heading { level, .. } => ContainerKind::Heading(heading_depth(level)),
            Tag::BlockQuote(alert) => {
                if let Some(alert) = alert {
                    self.diagnostics.push(Diagnostic::lossy(
                        format!("GFM alert kind {alert:?} dropped from block quote"),
                        Some(self.parsed_span(range.clone())),
                    ));
                }
                ContainerKind::BlockQuote
            }
            Tag::CodeBlock(kind) => {
                let (lang, meta) = match kind {
                    CodeBlockKind::Fenced(info) => split_info(info.as_ref()),
                    CodeBlockKind::Indented => (None, None),
                };
                ContainerKind::CodeBlock { lang, meta }
            }
            Tag::HtmlBlock => ContainerKind::HtmlBlock,
            Tag::List(first) => ContainerKind::List {
                ordered: first.is_some(),
                start: first,
            },
            Tag::Item => ContainerKind::Item { checked: None },
            Tag::Table(aligns) => {
                ContainerKind::Table(aligns.into_iter().map(column_align).collect())
            }
            Tag::TableHead => ContainerKind::TableHead,
            Tag::TableRow => ContainerKind::TableRow,
            Tag::TableCell => ContainerKind::TableCell,
            Tag::Emphasis => ContainerKind::Emphasis,
            Tag::Strong => ContainerKind::Strong,
            Tag::Strikethrough => ContainerKind::Delete,
            Tag::Superscript | Tag::Subscript => {
                self.diagnostics.push(Diagnostic::lossy(
                    "super/subscript folded to a generic span",
                    Some(self.parsed_span(range.clone())),
                ));
                ContainerKind::Span
            }
            Tag::Link {
                dest_url, title, ..
            } => ContainerKind::Link {
                url: dest_url.into_string(),
                title: non_empty(title.into_string()),
            },
            Tag::Image {
                dest_url, title, ..
            } => ContainerKind::Image {
                url: dest_url.into_string(),
                title: non_empty(title.into_string()),
            },
            // Disabled by the chosen options; kept total per the inventory.
            Tag::FootnoteDefinition(name) => ContainerKind::Unsupported {
                label: format!("footnote definition [{name}]"),
            },
            Tag::DefinitionList | Tag::DefinitionListTitle | Tag::DefinitionListDefinition => {
                ContainerKind::Unsupported {
                    label: "definition list".to_string(),
                }
            }
            Tag::MetadataBlock(_) => ContainerKind::Unsupported {
                label: "metadata block".to_string(),
            },
        };
        self.stack.push(Frame {
            tag: kind,
            children: Vec::new(),
            start: range.start,
        });
    }

    /// Handles an `Event::End`, popping the innermost frame into a finished
    /// node and appending it to the new innermost frame.
    fn end(&mut self, _tag_end: &TagEnd, range: Range<usize>) {
        // Never pop the root sentinel.
        if self.stack.len() <= 1 {
            self.diagnostics.push(Diagnostic::structural(
                "end event with no open container",
                None,
            ));
            return;
        }
        let frame = self.stack.pop().expect("len checked above");
        let span = self.parsed_span(frame.start..range.end);
        let Frame { tag, children, .. } = frame;

        match tag {
            ContainerKind::Root => {
                // Unreachable: the root is guarded above. Restore defensively.
                self.stack.push(Frame {
                    tag: ContainerKind::Root,
                    children,
                    start: 0,
                });
            }
            ContainerKind::HtmlBlock => {
                // Structural wrapper: splice the contained HTML lines upward.
                if let Some(parent) = self.stack.last_mut() {
                    parent.children.extend(children);
                }
            }
            ContainerKind::TableHead => {
                // A table head folds into an ordinary leading TableRow.
                let mut node = RenderNode::table_row(children);
                node.span = span;
                self.push_child(node);
            }
            ContainerKind::Unsupported { label } => {
                let mut node = RenderNode::unsupported(label.clone());
                node.span = span.clone();
                self.push_child(node);
                self.diagnostics
                    .push(Diagnostic::unsupported(label, Some(span)));
            }
            other => {
                let mut node = build_container(other, children);
                node.span = span;
                self.push_child(node);
            }
        }
    }

    /// Handles a `TaskListMarker`, setting `checked` on the enclosing list item.
    ///
    /// A marker with no enclosing list-item frame is malformed: it is dropped
    /// and a structural [`Diagnostic`] is recorded.
    fn task_marker(&mut self, checked: bool, range: Range<usize>) {
        match self.stack.last_mut().map(|f| &mut f.tag) {
            Some(ContainerKind::Item { checked: slot }) => {
                *slot = Some(checked);
            }
            _ => {
                self.diagnostics.push(Diagnostic::structural(
                    "task-list marker outside a list item",
                    Some(self.parsed_span(range)),
                ));
            }
        }
    }
}

/// Builds the finished node for a closed container frame.
///
/// `Root`, `HtmlBlock`, `TableHead` and `Unsupported` are handled by the caller
/// and never reach this function.
fn build_container(kind: ContainerKind, children: Vec<RenderNode>) -> RenderNode {
    match kind {
        ContainerKind::Paragraph => RenderNode::paragraph(children),
        ContainerKind::Heading(depth) => {
            let mut node = RenderNode::heading(depth, children);
            if let Some(slug) = slug_from_children(node.children()) {
                node.attrs.id = Some(slug);
            }
            node
        }
        ContainerKind::BlockQuote => RenderNode::block_quote(children),
        ContainerKind::CodeBlock { lang, meta } => {
            RenderNode::code(lang, meta, code_text(&children))
        }
        ContainerKind::List { ordered, start } => RenderNode::list(ordered, start, children),
        ContainerKind::Item { checked } => RenderNode::list_item(checked, children),
        ContainerKind::Table(align) => RenderNode::table(align, children),
        ContainerKind::TableRow => RenderNode::table_row(children),
        ContainerKind::TableCell => RenderNode::table_cell(children),
        ContainerKind::Emphasis => RenderNode::emphasis(children),
        ContainerKind::Strong => RenderNode::strong(children),
        ContainerKind::Delete => RenderNode::delete(children),
        ContainerKind::Span => RenderNode::span(Vec::new(), children),
        ContainerKind::Link { url, title } => RenderNode::link(url, title, children),
        ContainerKind::Image { url, title } => {
            RenderNode::image(url, title, image_alt(&children))
        }
        // Handled by `Fold::end`; unreachable here.
        ContainerKind::Root
        | ContainerKind::HtmlBlock
        | ContainerKind::TableHead
        | ContainerKind::Unsupported { .. } => RenderNode::unsupported("internal: unhandled"),
    }
}

/// Concatenates the literal text of code-block child nodes.
///
/// A fenced or indented code block emits its body as `Event::Text` lines; the
/// fold gathers them back into the `Code` node's `value`.
fn code_text(children: &[RenderNode]) -> String {
    let mut out = String::new();
    for child in children {
        if let NodeKind::Text { value } = &child.kind {
            out.push_str(value);
        }
    }
    out
}

/// Gathers image alt text from an image's child text nodes.
fn image_alt(children: &[RenderNode]) -> String {
    let mut out = String::new();
    collect_text(children, &mut out);
    out
}

/// Recursively appends the plain text of `children` to `out`.
fn collect_text(children: &[RenderNode], out: &mut String) {
    for child in children {
        match &child.kind {
            NodeKind::Text { value } | NodeKind::InlineCode { value } => out.push_str(value),
            _ => collect_text(child.children(), out),
        }
    }
}

/// Derives a heading slug id from the heading's text content.
///
/// ## Returns
///
/// `None` when the heading has no slug-able text (the empty string).
fn slug_from_children(children: &[RenderNode]) -> Option<String> {
    let mut text = String::new();
    collect_text(children, &mut text);
    let slug = slugify(&text);
    if slug.is_empty() {
        None
    } else {
        Some(slug)
    }
}

/// Slugifies text: lowercase, spaces to `-`, non-alphanumeric (other than `-`)
/// stripped.
fn slugify(text: &str) -> String {
    let mut slug = String::with_capacity(text.len());
    for ch in text.chars() {
        if ch.is_whitespace() {
            slug.push('-');
        } else if ch.is_alphanumeric() || ch == '-' {
            slug.extend(ch.to_lowercase());
        }
    }
    slug
}

/// Splits a fenced code-block info string into `(lang, meta)`.
///
/// The first whitespace-delimited token is the language; the remainder is meta.
fn split_info(info: &str) -> (Option<String>, Option<String>) {
    let trimmed = info.trim();
    if trimmed.is_empty() {
        return (None, None);
    }
    match trimmed.split_once(char::is_whitespace) {
        Some((lang, rest)) => {
            let meta = rest.trim();
            (
                Some(lang.to_string()),
                if meta.is_empty() {
                    None
                } else {
                    Some(meta.to_string())
                },
            )
        }
        None => (Some(trimmed.to_string()), None),
    }
}

/// Maps a `pulldown-cmark` heading level to a [`HeadingDepth`].
fn heading_depth(level: HeadingLevel) -> HeadingDepth {
    let depth = match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    };
    HeadingDepth::new(depth).expect("heading levels are within 1..=6")
}

/// Maps a `pulldown-cmark` column alignment to a [`ColumnAlign`].
fn column_align(alignment: pulldown_cmark::Alignment) -> ColumnAlign {
    match alignment {
        pulldown_cmark::Alignment::None => ColumnAlign::None,
        pulldown_cmark::Alignment::Left => ColumnAlign::Left,
        pulldown_cmark::Alignment::Center => ColumnAlign::Center,
        pulldown_cmark::Alignment::Right => ColumnAlign::Right,
    }
}

/// Normalizes an empty title string to `None`.
fn non_empty(value: String) -> Option<String> {
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Folds `input` from a virtual source named `"test"`.
    fn fold(input: &str) -> (Document, Vec<Diagnostic>) {
        fold_markdown_to_document(
            SourceDescriptor::Virtual {
                name: "test".into(),
            },
            input,
        )
    }

    /// Returns the document root's children.
    fn roots(doc: &Document) -> &[RenderNode] {
        doc.root.children()
    }

    #[test]
    fn simple_paragraph() {
        let (doc, diags) = fold("Hello world");
        assert!(diags.is_empty());
        let children = roots(&doc);
        assert_eq!(children.len(), 1);
        assert!(matches!(children[0].kind, NodeKind::Paragraph { .. }));
        assert!(matches!(
            children[0].children()[0].kind,
            NodeKind::Text { .. }
        ));
    }

    #[test]
    fn heading_gets_slug_id() {
        let (doc, _) = fold("## Hello, World!");
        let heading = &roots(&doc)[0];
        match &heading.kind {
            NodeKind::Heading { depth, .. } => assert_eq!(depth.get(), 2),
            other => panic!("expected heading, got {other:?}"),
        }
        assert_eq!(heading.attrs.id.as_deref(), Some("hello-world"));
    }

    #[test]
    fn nested_emphasis_strong_delete() {
        let (doc, diags) = fold("*a **b** ~~c~~*");
        assert!(diags.is_empty());
        let para = &roots(&doc)[0];
        let emphasis = &para.children()[0];
        assert!(matches!(emphasis.kind, NodeKind::Emphasis { .. }));
        let kinds: Vec<&NodeKind> = emphasis.children().iter().map(|n| &n.kind).collect();
        assert!(kinds
            .iter()
            .any(|k| matches!(k, NodeKind::Strong { .. })));
        assert!(kinds
            .iter()
            .any(|k| matches!(k, NodeKind::Delete { .. })));
    }

    #[test]
    fn fenced_code_block_with_lang_and_meta() {
        let (doc, _) = fold("```rust ignore,no_run\nlet x = 1;\n```");
        let code = &roots(&doc)[0];
        match &code.kind {
            NodeKind::Code { lang, meta, value } => {
                assert_eq!(lang.as_deref(), Some("rust"));
                assert_eq!(meta.as_deref(), Some("ignore,no_run"));
                assert_eq!(value, "let x = 1;\n");
            }
            other => panic!("expected code, got {other:?}"),
        }
    }

    #[test]
    fn ordered_and_unordered_lists() {
        let (doc, _) = fold("3. one\n4. two\n");
        match &roots(&doc)[0].kind {
            NodeKind::List { ordered, start, .. } => {
                assert!(ordered);
                assert_eq!(*start, Some(3));
            }
            other => panic!("expected ordered list, got {other:?}"),
        }

        let (doc, _) = fold("- one\n- two\n");
        match &roots(&doc)[0].kind {
            NodeKind::List { ordered, start, .. } => {
                assert!(!ordered);
                assert_eq!(*start, None);
            }
            other => panic!("expected unordered list, got {other:?}"),
        }
    }

    #[test]
    fn task_list_item_is_checked() {
        let (doc, diags) = fold("- [x] done\n- [ ] todo\n");
        assert!(diags.is_empty());
        let list = &roots(&doc)[0];
        let items = list.children();
        match &items[0].kind {
            NodeKind::ListItem { checked, .. } => assert_eq!(*checked, Some(true)),
            other => panic!("expected list item, got {other:?}"),
        }
        match &items[1].kind {
            NodeKind::ListItem { checked, .. } => assert_eq!(*checked, Some(false)),
            other => panic!("expected list item, got {other:?}"),
        }
    }

    #[test]
    fn table_has_header_row_first() {
        let (doc, _) = fold("| A | B |\n|:--|--:|\n| 1 | 2 |\n");
        let table = &roots(&doc)[0];
        match &table.kind {
            NodeKind::Table { align, children } => {
                assert_eq!(align, &[ColumnAlign::Left, ColumnAlign::Right]);
                // Header row first, then the body row.
                assert_eq!(children.len(), 2);
                assert!(matches!(children[0].kind, NodeKind::TableRow { .. }));
                assert!(matches!(children[1].kind, NodeKind::TableRow { .. }));
                // First row holds the header cells.
                let header_cell = &children[0].children()[0];
                assert!(matches!(header_cell.kind, NodeKind::TableCell { .. }));
            }
            other => panic!("expected table, got {other:?}"),
        }
    }

    #[test]
    fn link_and_image() {
        let (doc, _) = fold("[text](https://example.com \"t\")");
        let link = &roots(&doc)[0].children()[0];
        match &link.kind {
            NodeKind::Link { url, title, .. } => {
                assert_eq!(url, "https://example.com");
                assert_eq!(title.as_deref(), Some("t"));
            }
            other => panic!("expected link, got {other:?}"),
        }

        let (doc, _) = fold("![alt text](img.png)");
        let image = &roots(&doc)[0].children()[0];
        match &image.kind {
            NodeKind::Image { url, alt, .. } => {
                assert_eq!(url, "img.png");
                assert_eq!(alt, "alt text");
            }
            other => panic!("expected image, got {other:?}"),
        }
    }

    #[test]
    fn soft_and_hard_breaks() {
        let (doc, _) = fold("line one\nline two");
        let para = &roots(&doc)[0];
        assert!(para
            .children()
            .iter()
            .any(|n| matches!(n.kind, NodeKind::SoftBreak)));

        let (doc, _) = fold("line one\\\nline two");
        let para = &roots(&doc)[0];
        assert!(para
            .children()
            .iter()
            .any(|n| matches!(n.kind, NodeKind::HardBreak)));
    }

    #[test]
    fn raw_html_block_and_inline() {
        let (doc, _) = fold("<div>\nblock\n</div>\n");
        // HtmlBlock is spliced; its Html lines land at the document root.
        assert!(roots(&doc)
            .iter()
            .any(|n| matches!(n.kind, NodeKind::Html { block: true, .. })));

        let (doc, _) = fold("text <span>inline</span> more");
        let para = &roots(&doc)[0];
        assert!(para
            .children()
            .iter()
            .any(|n| matches!(n.kind, NodeKind::Html { block: false, .. })));
    }

    #[test]
    fn leaf_node_carries_source_location() {
        let input = "Hello world";
        let (doc, _) = fold_markdown_to_document(
            SourceDescriptor::Virtual {
                name: "test".into(),
            },
            input,
        );
        let text = &roots(&doc)[0].children()[0];
        let location = text
            .span
            .location
            .as_ref()
            .expect("parsed leaf must carry a source location");
        assert_eq!(text.span.provenance, Provenance::Parsed);
        // The single registered source has id 0.
        assert_eq!(location.source.0, 0);
        // The byte range must lie within the input and be non-empty.
        assert!(location.bytes.start < location.bytes.end);
        assert!(location.bytes.end <= input.len());
        assert_eq!(&input[location.bytes.clone()], "Hello world");
    }

    #[test]
    fn container_span_covers_start_to_end() {
        let input = "# Title";
        let (doc, _) = fold_markdown_to_document(
            SourceDescriptor::Virtual {
                name: "test".into(),
            },
            input,
        );
        let heading = &roots(&doc)[0];
        let location = heading
            .span
            .location
            .as_ref()
            .expect("container must carry a parsed location");
        assert_eq!(location.bytes.start, 0);
        assert!(location.bytes.end <= input.len());
    }

    /// A bare task marker outside a list is not reachable through CommonMark
    /// syntax: `pulldown-cmark` only emits `TaskListMarker` immediately inside
    /// an `Item`. The malformed-marker path is therefore covered by a direct
    /// unit test of `Fold::task_marker` rather than by a Markdown fixture.
    #[test]
    fn task_marker_outside_item_diagnoses() {
        let mut fold = Fold {
            stack: vec![Frame {
                tag: ContainerKind::Paragraph,
                children: Vec::new(),
                start: 0,
            }],
            source: SourceId(0),
            diagnostics: Vec::new(),
        };
        fold.task_marker(true, 0..3);
        assert_eq!(fold.diagnostics.len(), 1);
        assert!(fold.diagnostics[0]
            .message
            .contains("task-list marker outside a list item"));
    }
}
