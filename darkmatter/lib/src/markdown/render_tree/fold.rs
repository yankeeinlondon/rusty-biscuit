//! Stack-based fold of a `pulldown-cmark` event stream into a render tree.
//!
//! [`fold_markdown_to_document`] walks a [`pulldown_cmark`] 0.13 event stream
//! and builds a canonical [`renderable::tree::Document`]. It covers the common
//! CommonMark + GFM subset (paragraphs, headings, lists, tables, code,
//! emphasis, links, images, breaks, raw HTML, task lists), plus footnotes,
//! grouped raw-HTML blocks, and native superscript/subscript spans.
//!
//! Two darkmatter inline conveniences are intentionally **deferred** to a
//! follow-up feature: `==mark==` / dim inline styles and horizontal rules with
//! attribute blocks. Both are produced by darkmatter's `InlineStyleProcessor` /
//! `RuleProcessor`, which are iterator adapters bounded
//! `where I: Iterator<Item = Event<'a>>`. They cannot consume an `OffsetIter`
//! (item `(Event, Range)`) and they discard source byte ranges (splitting text
//! events, replacing whole paragraphs). Routing the fold through them would
//! make every node `Synthetic` with no location — a regression. Reconciling
//! offset preservation with those processors needs a separate design decision,
//! so the fold stays on `Parser::new_ext(...).into_offset_iter()` and the two
//! features wait for that follow-up.
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
    ColumnAlign, Diagnostic, Document, DocumentMetadata, Frontmatter as TreeFrontmatter,
    FrontmatterFormat, HeadingDepth, HintNamespace, NodeKind, Provenance, RenderNode,
    SourceDescriptor, SourceId, SourceLocation, SourceSpan,
};
use std::ops::Range;

use super::source::single_source_registry;
use crate::markdown::inline::HorizontalRuleAttrs;

/// Canonical parser-option set the render-tree fold uses.
///
/// Tables + strikethrough match darkmatter's terminal render path; task lists
/// let `ListItem.checked` be populated; footnotes and super/subscript fold to
/// real nodes (`FootnoteDefinition` / `FootnoteReference`, `Span`). Math,
/// definition lists, and metadata blocks stay off.
///
/// See `renderable/features/2026-05-20-darkmatter-tree/parser-options.md` for
/// the public-now / tree-experimental / deferred classification this matches.
#[must_use]
pub(crate) fn render_tree_parser_options() -> Options {
    Options::ENABLE_TABLES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_SUPERSCRIPT
        | Options::ENABLE_SUBSCRIPT
}

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
    /// A generic inline span carrying the given CSS classes. Used for native
    /// superscript (`["sup"]`) and subscript (`["sub"]`) folds.
    Span {
        classes: Vec<String>,
    },
    /// A footnote definition; `identifier` is the footnote label.
    FootnoteDefinition {
        identifier: String,
    },
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
    /// `Tag::HtmlBlock` — the `Event::Html` lines inside it are concatenated
    /// into a single `NodeKind::Html { block: true }` node on close.
    HtmlBlock,
    /// A darkmatter `==mark==` inline span. Pushed onto the main fold stack
    /// by the span-aware fold when an [`InlineEvent::Start(InlineTag::Mark)`]
    /// arrives; nested standard events (`Emphasis`, `Strong`, links, …)
    /// accumulate inside this frame naturally before the matching
    /// [`InlineEvent::End`] closes it.
    Mark,
    /// A darkmatter `⌄dim⌄` inline span. Pushed onto the main fold stack by
    /// the span-aware fold and closed the same way as [`ContainerKind::Mark`],
    /// gaining a [`Style`](renderable::style::Style) whose
    /// `emphasis.dim` is set on the produced [`NodeKind::Span`].
    Dim,
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

    /// Feeds a single `(Event, Range)` pair through the fold's main dispatch.
    ///
    /// Extracted from the body loop so the span-aware fold (see
    /// [`fold_markdown_spanned_with_frontmatter`]) can replay the same logic
    /// for `InlineEvent::Standard(_)` events while routing mark/dim and HR
    /// attribute events separately.
    pub(super) fn feed_event(&mut self, event: Event<'_>, range: Range<usize>) {
        match event {
            Event::Start(tag) => self.start(tag, range),
            Event::End(tag_end) => self.end(&tag_end, range),
            Event::Text(text) => {
                self.push_leaf(RenderNode::text(text.into_string()), range);
            }
            Event::Code(code) => {
                self.push_leaf(RenderNode::inline_code(code.into_string()), range);
            }
            Event::Html(html) => {
                self.push_leaf(RenderNode::html(html.into_string(), true), range);
            }
            Event::InlineHtml(html) => {
                self.push_leaf(RenderNode::html(html.into_string(), false), range);
            }
            Event::SoftBreak => self.push_leaf(RenderNode::soft_break(), range),
            Event::HardBreak => self.push_leaf(RenderNode::hard_break(), range),
            Event::Rule => self.push_leaf(RenderNode::thematic_break(), range),
            Event::TaskListMarker(checked) => self.task_marker(checked, range),
            Event::FootnoteReference(name) => {
                self.push_leaf(RenderNode::footnote_reference(name.into_string()), range);
            }
            // Math stays disabled by the chosen options; the fold must stay
            // total regardless.
            Event::InlineMath(_) | Event::DisplayMath(_) => {
                let label = "math expression".to_string();
                self.push_leaf(RenderNode::unsupported(label.clone()), range.clone());
                self.diagnostics.push(Diagnostic::unsupported(
                    label,
                    Some(self.parsed_span(range)),
                ));
            }
        }
    }
}

/// Folds a Markdown string into a canonical [`renderable::tree::Document`].
///
/// The input is parsed with `ENABLE_TABLES | ENABLE_STRIKETHROUGH |
/// ENABLE_TASKLISTS | ENABLE_FOOTNOTES | ENABLE_SUPERSCRIPT |
/// ENABLE_SUBSCRIPT`. Task lists let the render tree model
/// `ListItem.checked`; footnotes fold to `FootnoteDefinition` /
/// `FootnoteReference` nodes; superscript/subscript fold to class-carrying
/// `Span` nodes. Math, definition lists and metadata blocks stay off.
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
    fold_markdown_to_document_with_metadata(source, input, DocumentMetadata::default())
}

/// Folds a Markdown string into a [`Document`] with the supplied
/// [`DocumentMetadata`].
///
/// Identical to [`fold_markdown_to_document`] except that the caller provides
/// pre-built metadata — typically darkmatter's already extracted frontmatter
/// wired in via [`fold_markdown_with_frontmatter`]. The body fold itself never
/// sees a frontmatter block; the input must already have it stripped.
#[must_use]
pub fn fold_markdown_to_document_with_metadata(
    source: SourceDescriptor,
    input: &str,
    metadata: DocumentMetadata,
) -> (Document, Vec<Diagnostic>) {
    let (registry, source_id) = single_source_registry(source);

    let options = render_tree_parser_options();

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
        fold.feed_event(event, range);
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
        metadata,
        root: RenderNode::root(root_children),
    };
    (document, fold.diagnostics)
}

/// Folds a darkmatter [`Markdown`](crate::markdown::Markdown) value into a
/// [`Document`], wiring its already extracted frontmatter into
/// [`DocumentMetadata::frontmatter`].
///
/// The body fold sees only Markdown content — darkmatter extracts frontmatter
/// before the parser runs, and pulldown-cmark's metadata-block options stay
/// off (see `parser-options.md`). The metadata's raw source comes from
/// [`Frontmatter::raw_source`](crate::markdown::Frontmatter::raw_source);
/// programmatically constructed darkmatter frontmatter (no raw text) is
/// reported as `None`. Darkmatter's frontmatter is always YAML.
///
/// ## Returns
///
/// The folded [`Document`] (with [`DocumentMetadata::frontmatter`] populated
/// when raw frontmatter is available) and any non-fatal [`Diagnostic`]s.
#[must_use]
pub fn fold_markdown_with_frontmatter(
    source: SourceDescriptor,
    md: &crate::markdown::Markdown,
) -> (Document, Vec<Diagnostic>) {
    let metadata = DocumentMetadata {
        frontmatter: md.frontmatter().raw_source().map(|raw| TreeFrontmatter {
            format: FrontmatterFormat::Yaml,
            raw: raw.to_string(),
        }),
    };
    fold_markdown_to_document_with_metadata(source, md.content(), metadata)
}

/// Builds the synthetic [`NodeKind::ThematicBreak`] node for an HR-attribute
/// paragraph that the block-extension processor lifted from the event stream.
///
/// The returned node has [`Provenance::Generated`] (it was synthesized from a
/// paragraph) and carries the parsed `darkmatter.hr.*` hints. `body_range`
/// must point at the original paragraph body bytes — not the wider
/// `End(Paragraph)` range — so the produced
/// [`renderable::tree::SourceLocation`] is byte-identical to the legacy
/// `SpannedRuleProcessor` output.
fn lower_hr_attrs_to_node(
    attrs: HorizontalRuleAttrs,
    body_range: Range<usize>,
    source: SourceId,
) -> RenderNode {
    let mut node = RenderNode::thematic_break();
    node.span = SourceSpan {
        provenance: Provenance::Generated,
        location: Some(SourceLocation {
            source,
            bytes: body_range,
        }),
    };
    let hr_ns = HintNamespace("darkmatter.hr");
    if let Some(kind) = attrs.kind.as_ref().or(attrs.legacy_style.as_ref()) {
        node.attrs.set_hint(hr_ns, "kind", serde_json::json!(kind));
    }
    if let Some(alignment) = attrs.alignment {
        node.attrs
            .set_hint(hr_ns, "alignment", serde_json::json!(alignment));
    }
    if let Some(weight) = attrs.weight {
        node.attrs
            .set_hint(hr_ns, "weight", serde_json::json!(weight));
    }
    if let Some(width) = attrs.width {
        node.attrs
            .set_hint(hr_ns, "width", serde_json::json!(width));
    }
    if let Some(color) = attrs.color {
        node.attrs
            .set_hint(hr_ns, "color", serde_json::json!(color));
    }
    node
}

/// Folds Markdown through the span-aware processor chain (DMTR-3), preserving
/// source byte ranges for `==mark==`, dim, and HR-attribute paragraphs.
///
/// Walks the chain
/// `Parser::new_ext(...).into_offset_iter() -> BlockExtensionProcessor ->
/// SpannedInlineStyleProcessor` and folds:
///
/// - `InlineEvent::Standard(_)` events through the existing
///   [`Fold::feed_event`] dispatch.
/// - `InlineEvent::Start(InlineTag::Mark)` / `End(Mark)` into
///   [`NodeKind::Span`] containers with class `"mark"`.
/// - `InlineEvent::Start(InlineTag::Dim)` / `End(Dim)` into
///   [`NodeKind::Span`] containers whose `Style.emphasis.dim` is set so the
///   browser renderer lowers it to `opacity: 0.6` and the terminal renderer
///   emits the dim SGR (`\x1b[2m`) automatically.
/// - `BlockExtensionEvent::HorizontalRule` (from the crate-private
///   `block_extension` module) into a [`NodeKind::ThematicBreak`] with
///   `darkmatter.hr.*` hints (and [`Provenance::Generated`] because the event
///   was synthesized from a paragraph).
///
/// The body fold sees only Markdown content; darkmatter's already extracted
/// frontmatter flows into [`DocumentMetadata::frontmatter`].
///
/// ## Returns
///
/// The folded [`Document`] and any non-fatal [`Diagnostic`]s.
#[must_use]
pub fn fold_markdown_spanned_with_frontmatter(
    source: SourceDescriptor,
    md: &crate::markdown::Markdown,
) -> (Document, Vec<Diagnostic>) {
    use super::block_extension::{BlockExtensionEvent, BlockExtensionProcessor};
    use super::span::{SpannedInlineEvent, SpannedInlineStyleProcessor};
    use crate::markdown::inline::{InlineEvent, InlineTag};

    let metadata = DocumentMetadata {
        frontmatter: md.frontmatter().raw_source().map(|raw| TreeFrontmatter {
            format: FrontmatterFormat::Yaml,
            raw: raw.to_string(),
        }),
    };
    let (registry, source_id) = single_source_registry(source);

    let options = render_tree_parser_options();
    let parser = Parser::new_ext(md.content(), options).into_offset_iter();
    // Block-level extensions (HR-attribute paragraphs) run before the inline
    // span processor so the inline tier never sees a paragraph that has
    // already been lifted into a `ThematicBreak`. The map adapter rewraps
    // each `BlockExtensionEvent` as a `SpannedInlineEvent` so the existing
    // mark/dim chain can keep its shape.
    let chain = SpannedInlineStyleProcessor::new(
        md.content(),
        BlockExtensionProcessor::new(parser).map(|be| match be {
            BlockExtensionEvent::Standard(event, range) => {
                SpannedInlineEvent::parsed(InlineEvent::Standard(event), range)
            }
            BlockExtensionEvent::HorizontalRule { attrs, body_range } => {
                SpannedInlineEvent::generated(
                    InlineEvent::HorizontalRule(attrs),
                    body_range.clone(),
                    body_range,
                )
            }
        }),
    );

    let mut fold = Fold {
        stack: vec![Frame {
            tag: ContainerKind::Root,
            children: Vec::new(),
            start: 0,
        }],
        source: source_id,
        diagnostics: Vec::new(),
    };

    // Mark/dim frames live directly on `fold.stack` as `ContainerKind::Mark`
    // / `ContainerKind::Dim` so that nested standard events (Emphasis, Strong,
    // links, …) accumulate inside them naturally instead of being drained out
    // by a parallel sidecar. The fold's existing `feed_event` already pushes
    // children into whichever frame is innermost, so no extra wiring is
    // required for the standard branch.
    for spanned in chain {
        match spanned.event {
            InlineEvent::Standard(event) => {
                fold.feed_event(event, spanned.range);
            }
            InlineEvent::Start(tag) => {
                let kind = match tag {
                    InlineTag::Mark => ContainerKind::Mark,
                    InlineTag::Dim => ContainerKind::Dim,
                };
                fold.stack.push(Frame {
                    tag: kind,
                    children: Vec::new(),
                    start: spanned.range.start,
                });
            }
            InlineEvent::End(tag) => {
                // The innermost frame should be the matching inline tag.
                let matches_open = fold.stack.last().is_some_and(|f| {
                    matches!(
                        (&f.tag, tag),
                        (ContainerKind::Mark, InlineTag::Mark)
                            | (ContainerKind::Dim, InlineTag::Dim)
                    )
                });
                if !matches_open {
                    fold.diagnostics.push(Diagnostic::structural(
                        format!("mismatched inline tag end: closing {tag:?}"),
                        Some(fold.parsed_span(spanned.range.clone())),
                    ));
                    continue;
                }
                let frame = fold.stack.pop().expect("checked non-empty above");
                let span = fold.parsed_span(frame.start..spanned.range.end);
                let Frame {
                    tag: kind,
                    children,
                    ..
                } = frame;
                let mut node = build_container(kind, children);
                node.span = span;
                fold.push_child(node);
            }
            InlineEvent::HorizontalRule(attrs) => {
                // The block-extension processor synthesizes HR events from
                // paragraph bodies; provenance always points at the body
                // bytes. The shared lowering helper builds the
                // `Provenance::Generated` ThematicBreak with the
                // `darkmatter.hr.*` hints.
                let body_range = match spanned.provenance {
                    super::span::SpannedEventProvenance::GeneratedFrom { source } => source,
                    super::span::SpannedEventProvenance::Parsed => spanned.range.clone(),
                };
                let node = lower_hr_attrs_to_node(attrs, body_range, fold.source);
                fold.push_child(node);
            }
        }
    }

    // Drain remaining frames. Only the root should remain. Any unclosed
    // mark/dim or pulldown container splices its children upward and emits a
    // structural diagnostic — matching the legacy "unclosed reverts to
    // literal" posture without losing the inner content.
    while fold.stack.len() > 1 {
        let frame = fold.stack.pop().expect("len checked");
        let message = match &frame.tag {
            ContainerKind::Mark => "unclosed mark span".to_string(),
            ContainerKind::Dim => "unclosed dim span".to_string(),
            _ => "unclosed container in event stream".to_string(),
        };
        fold.diagnostics.push(Diagnostic::structural(message, None));
        if let Some(parent) = fold.stack.last_mut() {
            parent.children.extend(frame.children);
        }
    }

    let root_children = fold.stack.pop().map(|f| f.children).unwrap_or_default();
    let document = Document {
        sources: registry,
        metadata,
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
            Tag::Superscript => ContainerKind::Span {
                classes: vec!["sup".to_string()],
            },
            Tag::Subscript => ContainerKind::Span {
                classes: vec!["sub".to_string()],
            },
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
            Tag::FootnoteDefinition(name) => ContainerKind::FootnoteDefinition {
                identifier: name.into_string(),
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
                // Concatenate the contained `Event::Html` lines into one
                // block-level `Html` node spanning the whole `HtmlBlock`.
                let mut value = String::new();
                for child in &children {
                    if let NodeKind::Html { value: line, .. } = &child.kind {
                        value.push_str(line);
                    }
                }
                let mut node = RenderNode::html(value, true);
                node.span = span;
                self.push_child(node);
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
        ContainerKind::Span { classes } => RenderNode::span(classes, children),
        ContainerKind::FootnoteDefinition { identifier } => {
            RenderNode::footnote_definition(identifier, children)
        }
        ContainerKind::Link { url, title } => RenderNode::link(url, title, children),
        ContainerKind::Image { url, title } => RenderNode::image(url, title, image_alt(&children)),
        ContainerKind::Mark => RenderNode::span(vec!["mark".to_string()], children),
        ContainerKind::Dim => {
            // The span-aware design (`span-aware-processor-design.md`) requires
            // dim to ride in `Style.emphasis.dim`, not in semantic emphasis or
            // a custom hint, so both the terminal renderer's inline-style SGR
            // path (`\x1b[2m`) and the browser renderer's CSS lowering
            // (`opacity:0.6`) consume it automatically.
            let mut node = RenderNode::span(Vec::new(), children);
            let style = renderable::style::Style {
                emphasis: renderable::style::TextEmphasis {
                    dim: true,
                    ..Default::default()
                },
                ..Default::default()
            };
            node.attrs.set_style(&style);
            node
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
    if slug.is_empty() { None } else { Some(slug) }
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
    if value.is_empty() { None } else { Some(value) }
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
        assert!(kinds.iter().any(|k| matches!(k, NodeKind::Strong { .. })));
        assert!(kinds.iter().any(|k| matches!(k, NodeKind::Delete { .. })));
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
        assert!(
            para.children()
                .iter()
                .any(|n| matches!(n.kind, NodeKind::SoftBreak))
        );

        let (doc, _) = fold("line one\\\nline two");
        let para = &roots(&doc)[0];
        assert!(
            para.children()
                .iter()
                .any(|n| matches!(n.kind, NodeKind::HardBreak))
        );
    }

    #[test]
    fn raw_html_block_is_one_grouped_node() {
        let (doc, _) = fold("<div>\nblock\n</div>\n");
        // The whole HtmlBlock folds to exactly one block-level Html node whose
        // value concatenates the contained lines.
        let children = roots(&doc);
        let html: Vec<&RenderNode> = children
            .iter()
            .filter(|n| matches!(n.kind, NodeKind::Html { block: true, .. }))
            .collect();
        assert_eq!(html.len(), 1, "an HTML block must fold to a single node");
        match &html[0].kind {
            NodeKind::Html { value, block } => {
                assert!(*block);
                assert_eq!(value, "<div>\nblock\n</div>\n");
            }
            other => panic!("expected block Html, got {other:?}"),
        }
    }

    #[test]
    fn inline_html_is_a_non_block_html_node() {
        let (doc, _) = fold("text <span>inline</span> more");
        let para = &roots(&doc)[0];
        assert!(
            para.children()
                .iter()
                .any(|n| matches!(n.kind, NodeKind::Html { block: false, .. }))
        );
    }

    #[test]
    fn footnote_reference_and_definition_fold_to_real_nodes() {
        let (doc, diags) = fold("A claim.[^note]\n\n[^note]: The supporting detail.\n");
        assert!(diags.is_empty(), "footnotes fold without diagnostics");
        let children = roots(&doc);

        // The reference lands inside the paragraph.
        let para = &children[0];
        let reference = para
            .children()
            .iter()
            .find(|n| matches!(n.kind, NodeKind::FootnoteReference { .. }))
            .expect("paragraph carries a footnote reference");
        match &reference.kind {
            NodeKind::FootnoteReference { identifier } => assert_eq!(identifier, "note"),
            other => panic!("expected footnote reference, got {other:?}"),
        }

        // The definition is a top-level node.
        let definition = children
            .iter()
            .find(|n| matches!(n.kind, NodeKind::FootnoteDefinition { .. }))
            .expect("document carries a footnote definition");
        match &definition.kind {
            NodeKind::FootnoteDefinition {
                identifier,
                children,
            } => {
                assert_eq!(identifier, "note");
                assert!(!children.is_empty());
            }
            other => panic!("expected footnote definition, got {other:?}"),
        }
    }

    #[test]
    fn footnote_reference_carries_source_location() {
        let input = "A claim.[^note]\n\n[^note]: detail\n";
        let (doc, _) = fold(input);
        let para = &roots(&doc)[0];
        let reference = para
            .children()
            .iter()
            .find(|n| matches!(n.kind, NodeKind::FootnoteReference { .. }))
            .expect("paragraph carries a footnote reference");
        let location = reference
            .span
            .location
            .as_ref()
            .expect("footnote reference must carry a parsed source location");
        assert_eq!(reference.span.provenance, Provenance::Parsed);
        assert!(location.bytes.start < location.bytes.end);
        assert!(location.bytes.end <= input.len());
    }

    /// `pulldown-cmark`'s superscript extension only fires when the `^`
    /// delimiters flank a word boundary (`^text^`); `x^2^` mid-word parses as
    /// plain text. The test uses the form the parser accepts.
    #[test]
    fn native_superscript_folds_to_span_with_sup_class() {
        let (doc, diags) = fold("the ^2nd^ time");
        assert!(diags.is_empty(), "superscript folds without diagnostics");
        let para = &roots(&doc)[0];
        let span = para
            .children()
            .iter()
            .find(|n| matches!(n.kind, NodeKind::Span { .. }))
            .expect("paragraph carries a span");
        assert_eq!(span.attrs.classes, vec!["sup".to_string()]);
    }

    /// `pulldown-cmark`'s subscript extension uses single `~` delimiters at a
    /// word boundary (`~text~`); the double `~~` form stays strikethrough.
    #[test]
    fn native_subscript_folds_to_span_with_sub_class() {
        let (doc, diags) = fold("water is ~aqua~ here");
        assert!(diags.is_empty(), "subscript folds without diagnostics");
        let para = &roots(&doc)[0];
        let span = para
            .children()
            .iter()
            .find(|n| matches!(n.kind, NodeKind::Span { .. }))
            .expect("paragraph carries a span");
        assert_eq!(span.attrs.classes, vec!["sub".to_string()]);
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
        assert!(
            fold.diagnostics[0]
                .message
                .contains("task-list marker outside a list item")
        );
    }

    // -------------------------------------------------------------------
    // DMTR-3: span-aware fold for mark / dim / HR-attribute paragraphs.
    // -------------------------------------------------------------------

    /// Folds `input` through the span-aware path using a synthetic
    /// [`crate::markdown::Markdown`] value.
    fn fold_spanned(input: &str) -> (Document, Vec<Diagnostic>) {
        let md: crate::markdown::Markdown = input.into();
        fold_markdown_spanned_with_frontmatter(
            SourceDescriptor::Virtual {
                name: "spanned".into(),
            },
            &md,
        )
    }

    /// Visits `node` and every descendant, collecting `Span` nodes whose
    /// `attrs.classes` contains `class`.
    fn collect_spans_with_class<'a>(node: &'a RenderNode, class: &str) -> Vec<&'a RenderNode> {
        let mut out = Vec::new();
        fn walk<'a>(node: &'a RenderNode, class: &str, out: &mut Vec<&'a RenderNode>) {
            if matches!(node.kind, NodeKind::Span { .. })
                && node.attrs.classes.iter().any(|c| c == class)
            {
                out.push(node);
            }
            for child in node.children() {
                walk(child, class, out);
            }
        }
        walk(node, class, &mut out);
        out
    }

    #[test]
    fn span_aware_fold_emits_mark_span_with_class() {
        let (doc, diags) = fold_spanned("plain ==highlighted== after");
        assert!(diags.is_empty(), "clean fixture must fold cleanly");
        let marks = collect_spans_with_class(&doc.root, "mark");
        assert_eq!(marks.len(), 1, "expected one mark Span");
        // The mark contains the highlighted text.
        let mut text = String::new();
        collect_text(marks[0].children(), &mut text);
        assert_eq!(text, "highlighted");
    }

    /// Locates the first descendant `Span` whose `Style.emphasis.dim` is set.
    fn find_dim_span(node: &RenderNode) -> Option<&RenderNode> {
        if matches!(node.kind, NodeKind::Span { .. })
            && node.attrs.style().is_some_and(|s| s.emphasis.dim)
        {
            return Some(node);
        }
        for child in node.children() {
            if let Some(found) = find_dim_span(child) {
                return Some(found);
            }
        }
        None
    }

    #[test]
    fn span_aware_fold_emits_dim_span_with_style() {
        let (doc, diags) = fold_spanned("normal \u{2304}dimmed\u{2304} after");
        assert!(diags.is_empty());
        let dim = find_dim_span(&doc.root).expect("dim Span must exist");
        let mut text = String::new();
        collect_text(dim.children(), &mut text);
        assert_eq!(text, "dimmed");
    }

    #[test]
    fn span_aware_fold_emits_hr_with_attribute_hints() {
        let (doc, diags) = fold_spanned("--- { style: waves, width: \"50%\" }\n");
        assert!(diags.is_empty());
        // Find the ThematicBreak.
        fn find_hr(node: &RenderNode) -> Option<&RenderNode> {
            if matches!(node.kind, NodeKind::ThematicBreak) {
                return Some(node);
            }
            for child in node.children() {
                if let Some(found) = find_hr(child) {
                    return Some(found);
                }
            }
            None
        }
        let hr = find_hr(&doc.root).expect("ThematicBreak must exist");
        let ns = renderable::tree::HintNamespace("darkmatter.hr");
        assert_eq!(
            hr.attrs.get_hint(ns, "kind"),
            Some(&serde_json::json!("waves"))
        );
        assert_eq!(
            hr.attrs.get_hint(ns, "width"),
            Some(&serde_json::json!("50%"))
        );
        // Generated provenance: this HR was synthesized from a paragraph.
        assert_eq!(hr.span.provenance, Provenance::Generated);
        assert!(hr.span.location.is_some());
    }

    #[test]
    fn span_aware_fold_plain_rule_keeps_parsed_provenance() {
        // A bare `---` arrives as `Event::Rule` and folds straight through to
        // a parsed `ThematicBreak` — no HR attribute hints.
        let (doc, diags) = fold_spanned("para\n\n---\n\nmore");
        assert!(diags.is_empty());
        fn find_hr(node: &RenderNode) -> Option<&RenderNode> {
            if matches!(node.kind, NodeKind::ThematicBreak) {
                return Some(node);
            }
            for child in node.children() {
                if let Some(found) = find_hr(child) {
                    return Some(found);
                }
            }
            None
        }
        let hr = find_hr(&doc.root).expect("ThematicBreak must exist");
        assert_eq!(hr.span.provenance, Provenance::Parsed);
        let ns = renderable::tree::HintNamespace("darkmatter.hr");
        assert!(
            hr.attrs.get_hint(ns, "style").is_none(),
            "plain rule must not carry HR hints"
        );
    }

    /// Review-2 finding 2: the previous fold drained nested standard children
    /// out of the active fold frame into a sidecar, breaking sibling
    /// containers that appeared *after* a closed mark span. With the
    /// sidecar replaced by [`ContainerKind::Mark`] / [`ContainerKind::Dim`]
    /// frames on the main fold stack, an emphasis that follows a closed mark
    /// must remain its own container with its own text — not become an empty
    /// sibling of the drained text.
    ///
    /// Cross-text-event mark/dim spanning (review-3 finding 2) is covered by
    /// the `span_aware_fold_wraps_emphasis_inside_*` tests below.
    #[test]
    fn span_aware_fold_preserves_emphasis_sibling_after_mark() {
        let (doc, diags) = fold_spanned("==marked== then *italic*");
        assert!(
            diags.is_empty(),
            "clean fixture must fold cleanly: {diags:?}"
        );
        let para = &doc.root.children()[0];

        // The mark span and the emphasis must each survive as separate,
        // populated children of the paragraph — the sidecar bug would have
        // either swallowed the emphasis or left it empty.
        let marks = collect_spans_with_class(&doc.root, "mark");
        assert_eq!(marks.len(), 1, "expected one mark Span");
        let mut marked_text = String::new();
        collect_text(marks[0].children(), &mut marked_text);
        assert_eq!(marked_text, "marked");

        let emphasis = para
            .children()
            .iter()
            .find(|n| matches!(n.kind, NodeKind::Emphasis { .. }))
            .expect("paragraph must carry an Emphasis sibling after the mark");
        let mut italic_text = String::new();
        collect_text(emphasis.children(), &mut italic_text);
        assert_eq!(italic_text, "italic");
    }

    /// Review-3 finding 2: `==*highlighted*==` must fold to a mark Span whose
    /// child is the Emphasis container — proving the
    /// [`SpannedInlineStyleProcessor`](super::span::SpannedInlineStyleProcessor)
    /// tracks mark state across pulldown text events. The Emphasis arrives
    /// between the two `==` delimiters, which pulldown-cmark emits as
    /// separate text events.
    #[test]
    fn span_aware_fold_wraps_emphasis_inside_mark() {
        let (doc, diags) = fold_spanned("==*highlighted*== rest");
        assert!(
            diags.is_empty(),
            "clean fixture must fold cleanly: {diags:?}"
        );
        let marks = collect_spans_with_class(&doc.root, "mark");
        assert_eq!(marks.len(), 1, "expected one mark Span");
        let mark = marks[0];
        let first = mark
            .children()
            .first()
            .expect("mark span must have at least one child");
        assert!(
            matches!(first.kind, NodeKind::Emphasis { .. }),
            "expected Emphasis inside mark; got {:?}",
            first.kind,
        );
        let mut text = String::new();
        collect_text(first.children(), &mut text);
        assert_eq!(text, "highlighted");
    }

    /// Review-3 finding 2: the design's `⌄*dim and italic*⌄` fixture must
    /// fold to a dim Span whose single child is the Emphasis container.
    #[test]
    fn span_aware_fold_wraps_emphasis_inside_dim() {
        let (doc, diags) = fold_spanned("\u{2304}*dim and italic*\u{2304}");
        assert!(
            diags.is_empty(),
            "clean fixture must fold cleanly: {diags:?}"
        );
        let dim = find_dim_span(&doc.root).expect("dim Span must exist");
        let first = dim
            .children()
            .first()
            .expect("dim span must have at least one child");
        assert!(
            matches!(first.kind, NodeKind::Emphasis { .. }),
            "expected Emphasis inside dim; got {:?}",
            first.kind,
        );
        let mut text = String::new();
        collect_text(first.children(), &mut text);
        assert_eq!(text, "dim and italic");
    }

    /// Review-3 finding 2: an unclosed cross-text-event mark must revert to
    /// literal text rather than leaking an open container.
    #[test]
    fn span_aware_fold_unclosed_cross_event_mark_reverts() {
        let (doc, _diags) = fold_spanned("==*never closed* and on");
        let marks = collect_spans_with_class(&doc.root, "mark");
        assert!(
            marks.is_empty(),
            "unclosed cross-event mark must not emit a Span: {marks:?}",
        );
        // The literal `==` and the emphasis must both still appear.
        let para = &doc.root.children()[0];
        let mut text = String::new();
        collect_text(para.children(), &mut text);
        assert!(text.contains("=="), "literal `==` must survive: {text}");
        assert!(
            text.contains("never closed"),
            "italic text must survive: {text}"
        );
    }

    /// Review-4 finding 1 / span-aware-processor-design "Mixed Mark and
    /// Dim": `==highlighted and ⌄dim within mark⌄==` must fold to a mark
    /// Span containing a nested dim Span. The earlier single-slot
    /// implementation modeled mark and dim as mutually exclusive — a dim
    /// delimiter inside an open mark became literal text — which made the
    /// designed fixture impossible. The stack-based processor preserves
    /// nesting.
    #[test]
    fn span_aware_fold_nests_dim_inside_mark() {
        let (doc, diags) = fold_spanned("==highlighted and \u{2304}dim within mark\u{2304}==");
        assert!(
            diags.is_empty(),
            "clean fixture must fold cleanly: {diags:?}"
        );

        let marks = collect_spans_with_class(&doc.root, "mark");
        assert_eq!(marks.len(), 1, "expected one mark Span: {marks:?}");
        let mark = marks[0];

        // The mark contains literal text plus a nested dim Span.
        let dim = mark
            .children()
            .iter()
            .find(|n| {
                matches!(n.kind, NodeKind::Span { .. })
                    && n.attrs.style().is_some_and(|s| s.emphasis.dim)
            })
            .expect("mark must contain a nested dim Span");
        let mut dim_text = String::new();
        collect_text(dim.children(), &mut dim_text);
        assert_eq!(dim_text, "dim within mark");

        // The leading text "highlighted and " must precede the dim Span
        // inside the mark.
        let mut leading = String::new();
        for child in mark.children() {
            if matches!(child.kind, NodeKind::Span { .. })
                && child.attrs.style().is_some_and(|s| s.emphasis.dim)
            {
                break;
            }
            collect_text(std::slice::from_ref(child), &mut leading);
        }
        assert_eq!(leading.trim_end(), "highlighted and");
    }

    /// Review-4 finding 1 (reverse direction): nesting works in the other
    /// direction too — a mark delimiter inside an open dim opens a nested
    /// mark frame. This is not in the design's fixture list explicitly but
    /// falls out of the symmetric stack policy and guards against regressing
    /// to the old single-slot model in the other direction.
    #[test]
    fn span_aware_fold_nests_mark_inside_dim() {
        let (doc, diags) = fold_spanned("\u{2304}dim with ==marked inside==\u{2304}");
        assert!(
            diags.is_empty(),
            "clean fixture must fold cleanly: {diags:?}"
        );

        let dim = find_dim_span(&doc.root).expect("dim Span must exist");
        let nested_mark = dim
            .children()
            .iter()
            .find(|n| {
                matches!(n.kind, NodeKind::Span { .. })
                    && n.attrs.classes.iter().any(|c| c == "mark")
            })
            .expect("dim must contain a nested mark Span");
        let mut nested_text = String::new();
        collect_text(nested_mark.children(), &mut nested_text);
        assert_eq!(nested_text, "marked inside");
    }

    #[test]
    fn span_aware_fold_emits_no_mark_for_escaped_delimiter() {
        // `\==` is escaped and must not open a mark span. The legacy
        // pre-processor rewrites `\==` upstream; the span-aware processor
        // honors a literal backslash escape directly in its own pass.
        let (doc, diags) = fold_spanned("foo \\== not highlighted\n");
        assert!(diags.is_empty());
        let marks = collect_spans_with_class(&doc.root, "mark");
        assert!(
            marks.is_empty(),
            "escaped mark delimiter must not open a Span"
        );
    }

    // -----------------------------------------------------------------------
    // Review-6 finding 2 (fold tier): pin the exact `SourceLocation.bytes`
    // ranges from `span-aware-processor-design.md` for the container Span
    // and the generated thematic break. The span-tier event-range tests in
    // `super::span::tests` cover the inline event stream; these fold-tier
    // tests cover the assembled `RenderNode.span` that downstream tools and
    // diagnostics actually consume.
    // -----------------------------------------------------------------------

    /// `plain ==highlighted== after` must fold to a mark `Span` whose
    /// `SourceLocation.bytes` spans the full delimited region `6..21` (the
    /// opener at `6..8`, the inner text at `8..19`, and the closer at
    /// `19..21`). A bug that built container spans from only the opener or
    /// inner-text range would fail this test.
    #[test]
    fn span_aware_fold_mark_container_span_covers_full_delimited_region() {
        let input = "plain ==highlighted== after";
        let (doc, diags) = fold_spanned(input);
        assert!(
            diags.is_empty(),
            "clean fixture must fold cleanly: {diags:?}"
        );
        let marks = collect_spans_with_class(&doc.root, "mark");
        assert_eq!(marks.len(), 1, "expected exactly one mark Span");
        let mark = marks[0];
        let location = mark
            .span
            .location
            .as_ref()
            .expect("mark Span must carry a SourceLocation");
        assert_eq!(
            location.bytes,
            6..21,
            "mark container span must cover the opener, inner text, and closer",
        );
    }

    /// `--- { style: waves }` must fold to a `ThematicBreak` whose
    /// `SourceLocation.bytes` covers the **paragraph body** (`0..20`), not
    /// the wider `End(Paragraph)` range that includes the trailing newline.
    /// The span-aware HR processor synthesizes the event from the paragraph
    /// body bytes (provenance `Generated`); the fold then attaches the same
    /// `SourceLocation`. This pins the design's "HR Attributes: Basic"
    /// fixture (`location=0..20`) at the fold layer.
    #[test]
    fn span_aware_fold_hr_source_location_pins_paragraph_body_bytes() {
        let body = "--- { style: waves }";
        let input = format!("{body}\n");
        assert_eq!(body.len(), 20);
        let (doc, diags) = fold_spanned(&input);
        assert!(diags.is_empty(), "HR fixture must fold cleanly: {diags:?}");

        fn find_hr(node: &RenderNode) -> Option<&RenderNode> {
            if matches!(node.kind, NodeKind::ThematicBreak) {
                return Some(node);
            }
            for child in node.children() {
                if let Some(found) = find_hr(child) {
                    return Some(found);
                }
            }
            None
        }
        let hr = find_hr(&doc.root).expect("ThematicBreak must exist");
        assert_eq!(hr.span.provenance, Provenance::Generated);
        let location = hr
            .span
            .location
            .as_ref()
            .expect("generated HR must carry a SourceLocation");
        assert_eq!(
            location.bytes,
            0..body.len(),
            "generated HR SourceLocation must exactly cover the paragraph body bytes",
        );
    }

    /// Fold-tier counterpart of the span-tier escaped-mark test: the
    /// escaped `\==` literal must reach the fold as a Text node whose
    /// `SourceLocation.bytes` includes the backslash byte. This pins
    /// `span-aware-processor-design.md` *Mark: Escaped* at the fold layer.
    #[test]
    fn span_aware_fold_escaped_mark_literal_text_includes_backslash_byte() {
        // Byte layout: `foo \== bar` (11 bytes). The `\` is at byte 4 and the
        // `==` token is at bytes 5..7, so the design-conforming literal span
        // is `4..7`.
        let input = "foo \\== bar";
        let (doc, diags) = fold_spanned(input);
        assert!(
            diags.is_empty(),
            "escape fixture must fold cleanly: {diags:?}"
        );

        // Walk every Text descendant; the literal `==` must appear with the
        // exact `4..7` source range.
        fn find_text<'a>(node: &'a RenderNode, value: &str) -> Option<&'a RenderNode> {
            if let NodeKind::Text { value: text } = &node.kind
                && text == value
            {
                return Some(node);
            }
            for child in node.children() {
                if let Some(found) = find_text(child, value) {
                    return Some(found);
                }
            }
            None
        }
        let literal = find_text(&doc.root, "==")
            .expect("escaped mark literal must reach the fold as a Text node");
        let location = literal
            .span
            .location
            .as_ref()
            .expect("escaped mark literal must carry a SourceLocation");
        assert_eq!(
            location.bytes,
            4..7,
            "escaped mark literal SourceLocation must include the backslash byte",
        );
    }
}
