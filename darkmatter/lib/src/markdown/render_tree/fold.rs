//! Stack-based fold of a `pulldown-cmark` event stream into a render tree.
//!
//! [`fold_markdown_to_document`] walks a [`pulldown_cmark`] 0.13 event stream
//! and builds a canonical [`renderable::tree::Document`]. It covers the common
//! CommonMark + GFM subset (paragraphs, headings, lists, tables, code,
//! emphasis, links, images, breaks, raw HTML, task lists), plus footnotes,
//! grouped raw-HTML blocks, and native superscript/subscript spans.
//!
//! Two darkmatter inline conveniences — `==mark==` / `⌄dim⌄` inline styles and
//! horizontal rules with attribute blocks — are folded by
//! [`fold_markdown_spanned_with_frontmatter`], the span-aware entry point the
//! tree renderers route through. Rather than the old `InlineStyleProcessor` /
//! `RuleProcessor` iterator adapters (which discarded source byte ranges by
//! splitting text events and replacing whole paragraphs, forcing every node to
//! be `Synthetic`), the fold preserves offsets with two pre-parse passes:
//! `rewrite_inline_extensions` lowers `==mark==` / `⌄dim⌄` into canonical
//! GFM-strikethrough envelopes and records a provenance table that maps every
//! rewritten range back to the original source, and `BlockExtensionProcessor`
//! lifts HR-attribute paragraphs out of the event stream. The fold itself stays
//! on `Parser::new_ext(...).into_offset_iter()`, so every node keeps a concrete
//! span; `Fold::dispatch_strikethrough` then routes each `~~…~~` container to
//! [`NodeKind::Extended`] (registered envelope) or [`NodeKind::Delete`]
//! (ordinary strikethrough).
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
    FrontmatterFormat, HeadingDepth, HrAlignment, HrKind, HrWeight, NodeKind, Provenance,
    RenderNode, SourceDescriptor, SourceId, SourceLocation, SourceSpan, ThematicBreakAttrs,
};
use std::ops::Range;

use super::build_context::{TreeBuildContext, apply_node_policy, apply_page_colors};
use super::source::single_source_registry;
use crate::markdown::MarkdownError;
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
#[allow(clippy::large_enum_variant)]
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
    /// A deferred embedded render-tree region. Pushed when an opening marker
    /// decodes; buffers the portable fallback as children until the closing
    /// marker finalizes it (discarding the buffer in favor of `node`). If the
    /// region is never closed, the drain restores the buffered fallback.
    EmbedRegion {
        node: RenderNode,
        marker_span: SourceSpan,
    },
}

/// The mutable state threaded through the fold.
struct Fold<'ctx> {
    /// Container stack; the last entry is the innermost open container.
    stack: Vec<Frame>,
    /// The single source every parsed node refers to.
    source: SourceId,
    /// Diagnostics accumulated during the fold.
    diagnostics: Vec<Diagnostic>,
    /// Construction-time policy; `None` on the plain fold, `Some` on the
    /// context-aware fold that bakes component policy into nodes.
    ctx: Option<&'ctx TreeBuildContext<'ctx>>,
}

impl Fold<'_> {
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
    fn push_leaf(&mut self, mut kind_node: RenderNode, range: Range<usize>) {
        let span = self.parsed_span(range);
        kind_node.span = span;
        if let Some(ctx) = self.ctx {
            apply_node_policy(&mut kind_node, ctx);
        }
        self.push_child(kind_node);
    }

    /// Feeds a single `(Event, Range)` pair through the fold's main dispatch.
    ///
    /// Extracted from the body loop so the span-aware fold (see
    /// [`fold_markdown_spanned_with_frontmatter`]) can replay the same logic
    /// for [`BlockExtensionEvent::Standard`](super::block_extension::BlockExtensionEvent::Standard)
    /// events while routing lifted HR-attribute events separately.
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

    /// Lowers a disclosure block's summary and body events into a dedicated
    /// [`NodeKind::Disclosure`] node.
    ///
    /// The summary is phrasing-only content and is folded inside a temporary
    /// paragraph so the result is a flat inline sequence. The body is folded
    /// through a nested block-extension pass so disclosures can nest; if the
    /// result contains only inline nodes they are wrapped in a paragraph so the
    /// disclosed body is always block-level. Construction-time policy is applied
    /// when the fold runs in context-aware mode.
    fn lower_disclosure_to_node(
        &mut self,
        summary_events: Vec<(Event<'_>, Range<usize>)>,
        body_events: Vec<(Event<'_>, Range<usize>)>,
        inline_style: Option<&crate::style::schema::CommonStyle>,
        range: Range<usize>,
    ) -> Result<RenderNode, MarkdownError> {
        use super::disclosure_style::common_style_to_disclosure_hints;

        let summary = self.lower_inline_events(summary_events, range.start);
        let children = self.lower_body_events(body_events)?;

        let hints = inline_style.map(common_style_to_disclosure_hints);
        let mut node = RenderNode::disclosure(summary, children, hints);
        node.span = self.parsed_span(range);
        if let Some(ctx) = self.ctx {
            apply_node_policy(&mut node, ctx);
        }
        Ok(node)
    }

    /// Folds a sequence of inline events inside a temporary paragraph and
    /// returns the paragraph's children.
    ///
    /// Leading and trailing soft line breaks are trimmed so the summary does
    /// not retain the newline padding between the directive lines.
    fn lower_inline_events(
        &mut self,
        mut events: Vec<(Event<'_>, Range<usize>)>,
        anchor: usize,
    ) -> Vec<RenderNode> {
        while events
            .first()
            .map(|(e, _)| matches!(e, Event::SoftBreak))
            .unwrap_or(false)
        {
            events.remove(0);
        }
        while events
            .last()
            .map(|(e, _)| matches!(e, Event::SoftBreak))
            .unwrap_or(false)
        {
            events.pop();
        }

        let mut children = Vec::new();
        if !events.is_empty() {
            let anchor_range = anchor..anchor;
            let paragraph_events: Vec<_> = std::iter::once((Event::Start(Tag::Paragraph), anchor_range.clone()))
                .chain(events)
                .chain(std::iter::once((Event::End(TagEnd::Paragraph), anchor_range)))
                .collect();
            // Summary content is phrasing-only; a nested block-extension pass is
            // unnecessary and would reject any block-level construct anyway.
            children = self.run_plain_sub_fold(paragraph_events);

            // The sub-fold should have produced a single paragraph; unwrap it
            // so the summary is a flat inline sequence.
            if children.len() == 1 && matches!(children[0].kind, NodeKind::Paragraph { .. }) {
                children = children.into_iter().next().unwrap().children().to_vec();
            }
        }
        children
    }

    /// Folds a sequence of body events and returns block-level children.
    ///
    /// A nested block-extension pass recognizes nested disclosures in the body.
    /// When the folded result contains only inline nodes they are wrapped in a
    /// paragraph so the disclosed body is always structurally block-level.
    fn lower_body_events(
        &mut self,
        events: Vec<(Event<'_>, Range<usize>)>,
    ) -> Result<Vec<RenderNode>, MarkdownError> {
        let children = self.run_sub_fold(events)?;
        if children.is_empty() {
            return Ok(children);
        }
        if children.iter().all(|n| is_inline_kind(&n.kind)) {
            Ok(vec![RenderNode::paragraph(children)])
        } else {
            Ok(children)
        }
    }

    /// Runs a temporary sub-fold over `events` through a nested
    /// block-extension pass, using the same source and context as this fold.
    ///
    /// This is the body path: it re-runs [`BlockExtensionProcessor`] so nested
    /// disclosures inside the disclosed body are recognized recursively. Text
    /// events are pre-split at disclosure directive line boundaries so a body
    /// and its closer that share a single `Event::Text` are still handled
    /// correctly.
    /// Diagnostics raised by the sub-fold are appended to this fold's
    /// diagnostics so nothing is lost.
    fn run_sub_fold<'a>(
        &mut self,
        events: Vec<(Event<'a>, Range<usize>)>,
    ) -> Result<Vec<RenderNode>, MarkdownError> {
        use super::block_extension::{BlockExtensionEvent, BlockExtensionProcessor};

        let events = split_disclosure_directives(events);

        let mut sub = Fold {
            stack: vec![Frame {
                tag: ContainerKind::Root,
                children: Vec::new(),
                start: 0,
            }],
            source: self.source,
            diagnostics: Vec::new(),
            ctx: self.ctx,
        };

        for be in BlockExtensionProcessor::new(events.into_iter()) {
            match be? {
                BlockExtensionEvent::Standard(event, range) => sub.feed_event(event, range),
                BlockExtensionEvent::HorizontalRule { attrs, body_range } => {
                    let mut node = lower_hr_attrs_to_node(attrs, body_range, sub.source);
                    if let Some(ctx) = sub.ctx {
                        apply_node_policy(&mut node, ctx);
                    }
                    sub.push_child(node);
                }
                BlockExtensionEvent::Disclosure {
                    summary_events,
                    body_events,
                    inline_style,
                    range,
                } => {
                    let node = sub.lower_disclosure_to_node(
                        summary_events,
                        body_events,
                        inline_style.as_ref(),
                        range,
                    )?;
                    sub.push_child(node);
                }
            }
        }

        while sub.stack.len() > 1 {
            let frame = sub.stack.pop().expect("len checked");
            sub.diagnostics.push(Diagnostic::structural(
                "unclosed container in disclosure region",
                None,
            ));
            if let Some(parent) = sub.stack.last_mut() {
                parent.children.extend(frame.children);
            }
        }

        self.diagnostics.append(&mut sub.diagnostics);
        Ok(sub.stack.pop().map(|f| f.children).unwrap_or_default())
    }

    /// Runs a temporary sub-fold over plain `events` with no block-extension
    /// pass.
    ///
    /// Used for summary content, which is phrasing-only and therefore cannot
    /// contain nested disclosures.
    fn run_plain_sub_fold<'a>(
        &mut self,
        events: impl IntoIterator<Item = (Event<'a>, Range<usize>)>,
    ) -> Vec<RenderNode> {
        let mut sub = Fold {
            stack: vec![Frame {
                tag: ContainerKind::Root,
                children: Vec::new(),
                start: 0,
            }],
            source: self.source,
            diagnostics: Vec::new(),
            ctx: self.ctx,
        };

        for (event, event_range) in events {
            sub.feed_event(event, event_range);
        }

        while sub.stack.len() > 1 {
            let frame = sub.stack.pop().expect("len checked");
            sub.diagnostics.push(Diagnostic::structural(
                "unclosed container in disclosure region",
                None,
            ));
            if let Some(parent) = sub.stack.last_mut() {
                parent.children.extend(frame.children);
            }
        }

        self.diagnostics.append(&mut sub.diagnostics);
        sub.stack.pop().map(|f| f.children).unwrap_or_default()
    }
}

/// Returns `true` when `kind` is an inline (phrasing-level) render-tree node.
fn is_inline_kind(kind: &NodeKind) -> bool {
    !matches!(
        kind,
        NodeKind::Root { .. }
            | NodeKind::Heading { .. }
            | NodeKind::Section { .. }
            | NodeKind::Paragraph { .. }
            | NodeKind::BlockQuote { .. }
            | NodeKind::List { .. }
            | NodeKind::ListItem { .. }
            | NodeKind::Code { .. }
            | NodeKind::ThematicBreak
            | NodeKind::Table { .. }
            | NodeKind::TableRow { .. }
            | NodeKind::TableCell { .. }
            | NodeKind::FootnoteDefinition { .. }
            | NodeKind::Disclosure { .. }
            | NodeKind::Html { block: true, .. }
    )
}

/// Splits `Event::Text` events at disclosure directive line boundaries.
///
/// A body and its `::end-disclosure` closer can end up in the same
/// `Event::Text` when the body is plain text. Pre-splitting ensures the
/// block-extension pass sees each directive as its own text event, so
/// disclosures with plain-text summaries/bodies are recognized correctly.
///
/// The `::disclosure` opener keeps the remainder of its line attached to the
/// directive event so the block-extension opener handler can parse inline
/// `key=value` style tokens (and any same-line summary) via
/// `parse_disclosure_opener_style`. Splitting at the bare keyword would strand
/// those tokens in a following text event, where they would be mistaken for
/// summary content. The other directives carry no inline tail and split at the
/// keyword boundary.
fn split_disclosure_directives<'a>(
    events: Vec<(Event<'a>, Range<usize>)>,
) -> Vec<(Event<'a>, Range<usize>)> {
    use pulldown_cmark::CowStr;

    const DIRECTIVES: &[&str] = &["::disclosure", "::details", "::end-disclosure"];

    let mut out = Vec::with_capacity(events.len());
    for (event, range) in events {
        let Event::Text(text) = event else {
            out.push((event, range));
            continue;
        };

        let text_str = text.as_ref();
        let mut split_from = 0;
        let mut offset = range.start;
        loop {
            let rest = &text_str[split_from..];
            let match_pos = DIRECTIVES
                .iter()
                .filter_map(|dir| {
                    rest.find(dir).and_then(|pos| {
                        let abs_pos = split_from + pos;
                        let before = &text_str[..abs_pos];
                        let after_dir = &text_str[abs_pos + dir.len()..];
                        let prefix_ok = before.is_empty() || before.ends_with('\n');
                        let suffix_ok = after_dir.is_empty()
                            || after_dir.starts_with(|c: char| c.is_ascii_whitespace());
                        if prefix_ok && suffix_ok {
                            Some((abs_pos, dir.len()))
                        } else {
                            None
                        }
                    })
                })
                .min_by_key(|(pos, _)| *pos);

            let Some((dir_pos, dir_len)) = match_pos else {
                if split_from == 0 {
                    // No directive occurs anywhere in this text event — the
                    // common case for documents with no disclosure directives.
                    // Forward the borrowed event unchanged rather than copying
                    // its bytes into a fresh `String` (finding 20).
                    out.push((Event::Text(text), range));
                } else if split_from < text_str.len() {
                    let tail = text_str[split_from..].to_string();
                    let tail_len = tail.len();
                    out.push((Event::Text(CowStr::from(tail)), offset..offset + tail_len));
                }
                break;
            };

            if dir_pos > split_from {
                let before = text_str[split_from..dir_pos].to_string();
                let before_len = before.len();
                out.push((Event::Text(CowStr::from(before)), offset..offset + before_len));
                offset += before_len;
            }

            // For the `::disclosure` opener, keep the remainder of the line
            // (inline `key=value` style tokens and any same-line summary text)
            // attached to the directive event. The block-extension opener
            // handler parses that tail via `parse_disclosure_opener_style`;
            // splitting at the keyword boundary would strand the style tokens in
            // a following text event, where they would be mistaken for summary
            // content. The other directives carry no inline tail, so they split
            // at the keyword boundary as before.
            let is_opener = &text_str[dir_pos..dir_pos + dir_len] == DIRECTIVES[0];
            let dir_end = if is_opener {
                text_str[dir_pos..]
                    .find('\n')
                    .map_or(text_str.len(), |nl| dir_pos + nl)
            } else {
                dir_pos + dir_len
            };

            let dir_text = text_str[dir_pos..dir_end].to_string();
            let dir_text_len = dir_text.len();
            out.push((Event::Text(CowStr::from(dir_text)), offset..offset + dir_text_len));
            offset += dir_text_len;
            split_from = dir_end;
        }
    }
    out
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
        ctx: None,
    };

    for (event, range) in Parser::new_ext(input, options).into_offset_iter() {
        fold.feed_event(event, range);
    }

    // Drain remaining frames. Only the root should remain; if the parser left
    // containers open (malformed), splice their children upward and diagnose.
    // An unterminated embedded region restores its buffered fallback this way.
    while fold.stack.len() > 1 {
        let frame = fold.stack.pop().expect("len checked");
        let message = if matches!(frame.tag, ContainerKind::EmbedRegion { .. }) {
            "unterminated embedded render-tree region: missing closing marker"
        } else {
            "unclosed container in event stream"
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
/// paragraph) and carries the parsed HR styling in the typed
/// [`ThematicBreakAttrs`] field. `body_range`
/// must point at the original paragraph body bytes — not the wider
/// `End(Paragraph)` range — so the produced
/// [`renderable::tree::SourceLocation`] is byte-identical to the retired
/// span-aware HR output.
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
    node.attrs.set_thematic_break(&ThematicBreakAttrs {
        // The canonical `kind` key wins over the deprecated `style:` alias.
        // Author text is parsed to the shared enums here, at the single fold
        // boundary; an unrecognized spelling becomes `None` and the renderers
        // fall back to their defaults.
        kind: attrs
            .kind
            .or(attrs.legacy_style)
            .as_deref()
            .and_then(HrKind::from_authored),
        alignment: attrs.alignment.as_deref().and_then(HrAlignment::from_authored),
        weight: attrs.weight.as_deref().and_then(HrWeight::from_authored),
        width: attrs.width,
        color: attrs.color,
    });
    node
}

/// Folds Markdown through the source-rewrite inline-extension pipeline,
/// preserving source byte ranges for `==mark==`, `⌄dim⌄`, and HR-attribute
/// paragraphs.
///
/// The pipeline is:
///
/// 1. [`rewrite_inline_extensions`](super::inline_extension::rewrite_inline_extensions)
///    rewrites darkmatter inline syntax (`==mark==`, `⌄dim⌄`) into canonical
///    GFM-strikethrough envelopes before `pulldown-cmark` ever sees the source.
/// 2. `pulldown-cmark` parses the rewritten source; the
///    [`BlockExtensionProcessor`](super::block_extension::BlockExtensionProcessor)
///    lifts HR-attribute paragraphs out of the event stream.
/// 3. The plain [`Fold`] consumes the events. Each `~~…~~` container is
///    dispatched by [`Fold::dispatch_strikethrough`]: a registered envelope
///    folds to [`NodeKind::Extended`] (`mark` / `dim`), an ordinary
///    strikethrough folds to [`NodeKind::Delete`].
/// 4. Because the fold parsed a *rewritten* source, every node and diagnostic
///    range is mapped back to the original source through the rewriter's
///    provenance table.
///
/// HR-attribute paragraphs fold to a [`NodeKind::ThematicBreak`] with typed
/// [`ThematicBreakAttrs`] (and [`Provenance::Generated`] because the event was
/// synthesized from a paragraph). The body fold sees only Markdown content;
/// darkmatter's already extracted frontmatter flows into
/// [`DocumentMetadata::frontmatter`].
///
/// ## Returns
///
/// The folded [`Document`] and any non-fatal [`Diagnostic`]s, or a fatal
/// [`MarkdownError`] raised by the block-extension processor (for example, a
/// malformed disclosure block).
pub fn fold_markdown_spanned_with_frontmatter(
    source: SourceDescriptor,
    md: &crate::markdown::Markdown,
) -> crate::markdown::MarkdownResult<(Document, Vec<Diagnostic>)> {
    use super::block_extension::{BlockExtensionEvent, BlockExtensionProcessor};
    use super::inline_extension::rewrite_inline_extensions;

    let metadata = DocumentMetadata {
        frontmatter: md.frontmatter().raw_source().map(|raw| TreeFrontmatter {
            format: FrontmatterFormat::Yaml,
            raw: raw.to_string(),
        }),
    };
    let (registry, source_id) = single_source_registry(source);

    // 1. Source-layer rewrite. `==mark==` / `⌄dim⌄` become canonical
    //    strikethrough envelopes; everything else (including ordinary `~~…~~`)
    //    is untouched. A document with no darkmatter inline borrows its source
    //    unchanged with an empty provenance table.
    let rewrite = rewrite_inline_extensions(md.content());

    // 2. Parse the rewritten source. HR-attribute paragraphs are lifted out
    //    before the fold sees them, exactly as on the plain path. Text events
    //    are pre-split at disclosure directive line boundaries so disclosures
    //    with plain-text summaries/bodies are recognized correctly.
    let options = render_tree_parser_options();
    let parser = Parser::new_ext(rewrite.source.as_ref(), options).into_offset_iter();
    let chain = BlockExtensionProcessor::new(split_disclosure_directives(parser.collect()).into_iter());

    let mut fold = Fold {
        stack: vec![Frame {
            tag: ContainerKind::Root,
            children: Vec::new(),
            start: 0,
        }],
        source: source_id,
        diagnostics: Vec::new(),
        ctx: None,
    };

    // 3. Plain fold. `Fold::end` dispatches `~~…~~` containers to `Extended`
    //    (registered envelope) or `Delete` (ordinary strikethrough); HR events
    //    lower to a generated `ThematicBreak`; disclosure blocks replay as
    //    summary-then-body fallback content until Phase 2 lowers them to a
    //    dedicated render-tree node.
    for be in chain {
        match be? {
            BlockExtensionEvent::Standard(event, range) => fold.feed_event(event, range),
            BlockExtensionEvent::HorizontalRule { attrs, body_range } => {
                let mut node = lower_hr_attrs_to_node(attrs, body_range, fold.source);
                if let Some(ctx) = fold.ctx {
                    apply_node_policy(&mut node, ctx);
                }
                fold.push_child(node);
            }
            BlockExtensionEvent::Disclosure {
                summary_events,
                body_events,
                inline_style,
                range,
            } => {
                let node = fold.lower_disclosure_to_node(
                    summary_events,
                    body_events,
                    inline_style.as_ref(),
                    range,
                )?;
                fold.push_child(node);
            }
        }
    }

    // Drain remaining frames. Only the root should remain; an unclosed
    // container splices its children upward and emits a structural diagnostic.
    // An unterminated embedded region restores its buffered fallback this way.
    while fold.stack.len() > 1 {
        let frame = fold.stack.pop().expect("len checked");
        let message = if matches!(frame.tag, ContainerKind::EmbedRegion { .. }) {
            "unterminated embedded render-tree region: missing closing marker"
        } else {
            "unclosed container in event stream"
        };
        fold.diagnostics.push(Diagnostic::structural(message, None));
        if let Some(parent) = fold.stack.last_mut() {
            parent.children.extend(frame.children);
        }
    }

    let root_children = fold.stack.pop().map(|f| f.children).unwrap_or_default();
    let mut diagnostics = fold.diagnostics;
    let mut root = RenderNode::root(root_children);

    // 4. The fold built every span over rewritten-source offsets. When a
    //    rewrite occurred, map each node and diagnostic range back to the
    //    original source so provenance points at the bytes the user typed. An
    //    empty provenance table is the identity map, so the borrowed-source
    //    case skips the walk entirely.
    if !rewrite.provenance.is_empty() {
        resolve_node_spans(&mut root, &rewrite.provenance);
        for diagnostic in &mut diagnostics {
            if let Some(location) = diagnostic
                .span
                .as_mut()
                .and_then(|span| span.location.as_mut())
            {
                location.bytes = rewrite.provenance.resolve_range(location.bytes.clone());
            }
        }
    }

    let document = Document {
        sources: registry,
        metadata,
        root,
    };
    Ok((document, diagnostics))
}

/// Context-aware span fold: identical to [`fold_markdown_spanned_with_frontmatter`]
/// but attaches construction-time policy (component layout, colors, text-layout
/// hints, structured directives, HR defaults) from `ctx` as nodes are built.
///
/// The resulting [`Document`] is the complete typed render input — no
/// post-fold decoration or attribute injection is needed.
///
/// ## Returns
///
/// The folded [`Document`] and any non-fatal [`Diagnostic`]s, or a fatal
/// [`MarkdownError`] raised by the block-extension processor.
pub(crate) fn fold_markdown_spanned_with_context(
    source: SourceDescriptor,
    md: &crate::markdown::Markdown,
    ctx: &TreeBuildContext,
) -> crate::markdown::MarkdownResult<(Document, Vec<Diagnostic>)> {
    use super::block_extension::{BlockExtensionEvent, BlockExtensionProcessor};
    use super::inline_extension::rewrite_inline_extensions;

    let metadata = DocumentMetadata {
        frontmatter: md.frontmatter().raw_source().map(|raw| TreeFrontmatter {
            format: FrontmatterFormat::Yaml,
            raw: raw.to_string(),
        }),
    };
    let (registry, source_id) = single_source_registry(source);

    let rewrite = rewrite_inline_extensions(md.content());

    let options = render_tree_parser_options();
    let parser = Parser::new_ext(rewrite.source.as_ref(), options).into_offset_iter();
    let chain = BlockExtensionProcessor::new(split_disclosure_directives(parser.collect()).into_iter());

    let mut fold = Fold {
        stack: vec![Frame {
            tag: ContainerKind::Root,
            children: Vec::new(),
            start: 0,
        }],
        source: source_id,
        diagnostics: Vec::new(),
        ctx: Some(ctx),
    };

    for be in chain {
        match be? {
            BlockExtensionEvent::Standard(event, range) => fold.feed_event(event, range),
            BlockExtensionEvent::HorizontalRule { attrs, body_range } => {
                let mut node = lower_hr_attrs_to_node(attrs, body_range, fold.source);
                if let Some(ctx) = fold.ctx {
                    apply_node_policy(&mut node, ctx);
                }
                fold.push_child(node);
            }
            BlockExtensionEvent::Disclosure {
                summary_events,
                body_events,
                inline_style,
                range,
            } => {
                let node = fold.lower_disclosure_to_node(
                    summary_events,
                    body_events,
                    inline_style.as_ref(),
                    range,
                )?;
                fold.push_child(node);
            }
        }
    }

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
    let mut diagnostics = fold.diagnostics;
    let mut root = RenderNode::root(root_children);

    // Apply page-level colors to the root so they inherit to all descendants.
    apply_page_colors(&mut root, ctx);

    if !rewrite.provenance.is_empty() {
        resolve_node_spans(&mut root, &rewrite.provenance);
        for diagnostic in &mut diagnostics {
            if let Some(location) = diagnostic
                .span
                .as_mut()
                .and_then(|span| span.location.as_mut())
            {
                location.bytes = rewrite.provenance.resolve_range(location.bytes.clone());
            }
        }
    }

    let document = Document {
        sources: registry,
        metadata,
        root,
    };
    Ok((document, diagnostics))
}

impl Fold<'_> {
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
                // An embedded render-tree opening marker opens a deferred
                // region: the portable fallback that follows buffers as its
                // children until the closing marker discards it in favor of the
                // decoded subtree. Deferring (rather than splicing now) lets an
                // unterminated region restore its fallback instead of dropping
                // it. A malformed marker falls through to the raw-HTML path.
                if let Some(node) = renderable::tree::decode_embedded_open(&value) {
                    self.stack.push(Frame {
                        tag: ContainerKind::EmbedRegion {
                            node,
                            marker_span: span,
                        },
                        children: Vec::new(),
                        start: range.start,
                    });
                    return;
                }
                // A closing marker whose innermost frame is an open region
                // finalizes it: discard the buffered fallback, splice the
                // decoded subtree. A stray close with no open region falls
                // through and renders as ordinary HTML.
                if renderable::tree::is_embedded_close(&value)
                    && let Some(Frame {
                        tag: ContainerKind::EmbedRegion { .. },
                        ..
                    }) = self.stack.last()
                {
                    let Frame { tag, .. } = self.stack.pop().expect("checked above");
                    let ContainerKind::EmbedRegion { node, marker_span } = tag else {
                        unreachable!("matched EmbedRegion above")
                    };
                    self.splice_embedded_node(node, marker_span);
                    return;
                }
                let mut node = RenderNode::html(value, true);
                node.span = span;
                self.push_child(node);
            }
            ContainerKind::EmbedRegion { .. } => {
                // An ordinary End event popped the region before its closing
                // marker: the region is malformed. Restore the buffered
                // fallback and diagnose rather than dropping content.
                self.diagnostics.push(Diagnostic::structural(
                    "unterminated embedded render-tree region",
                    Some(span),
                ));
                for child in children {
                    self.push_child(child);
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
            ContainerKind::Delete => {
                // A `~~…~~` container is either an ordinary GFM strikethrough or
                // a darkmatter inline-extension envelope produced by the source
                // rewriter (`==mark==` / `⌄dim⌄`). Peek the leading marker and
                // dispatch accordingly.
                let mut node = self.dispatch_strikethrough(children, span);
                if let Some(ctx) = self.ctx {
                    apply_node_policy(&mut node, ctx);
                }
                self.push_child(node);
            }
            other => {
                let mut node = build_container(other, children);
                node.span = span;
                if let Some(ctx) = self.ctx {
                    apply_node_policy(&mut node, ctx);
                }
                self.push_child(node);
            }
        }
    }

    /// Splices a decoded embedded subtree into the current container. A `Root`
    /// contributes its children (it may appear only at the document top level);
    /// any other node splices directly, carrying `span`.
    fn splice_embedded_node(&mut self, node: RenderNode, span: SourceSpan) {
        if let NodeKind::Root { children } = node.kind {
            for child in children {
                self.push_child(child);
            }
        } else {
            let mut node = node;
            node.span = span;
            self.push_child(node);
        }
    }

    /// Decides whether a closed `~~…~~` container is a darkmatter inline
    /// extension envelope or an ordinary GFM strikethrough.
    ///
    /// The inline source rewriter (see
    /// [`inline_extension`](super::inline_extension)) emits the canonical
    /// envelope `~~{{!TOKEN!}}\u{FDD0}payload{{!TOKEN!}}\u{FDD0}~~`, so the
    /// first child of an extension container is a text node whose value begins
    /// with the `{{!TOKEN!}}\u{FDD0}` marker. A registered token strips its
    /// markers and folds to [`NodeKind::Extended`]; an unknown token (only
    /// reachable on a rewriter/registry drift bug) records a diagnostic and
    /// falls back to [`NodeKind::Delete`]; any other container is a plain
    /// strikethrough.
    fn dispatch_strikethrough(
        &mut self,
        children: Vec<RenderNode>,
        span: SourceSpan,
    ) -> RenderNode {
        let Some(token) = envelope_token(&children) else {
            let mut node = RenderNode::delete(children);
            node.span = span;
            return node;
        };
        match super::inline_extension::token_by_name(&token) {
            Some(spec) => {
                let stripped = strip_envelope_markers(children, spec.name);
                let mut node = RenderNode::extended(spec.name, stripped, None);
                node.span = span;
                node
            }
            None => {
                self.diagnostics.push(Diagnostic::structural(
                    format!("unknown inline extension token `{token}` in strikethrough envelope"),
                    Some(span.clone()),
                ));
                let mut node = RenderNode::delete(children);
                node.span = span;
                node
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
            let mut node = RenderNode::code(lang.clone(), meta, code_text(&children));
            // Request a code-block header row so the terminal tree path emits
            // the same right-aligned language pill the legacy terminal renderer
            // produces for every fenced block (parity with
            // `output::terminal::emit_highlighted_code_block`). The browser code
            // hook ignores this hint, so HTML output is unaffected; the plain
            // (no-`CodeRenderer`) tree fallback ignores it too.
            node.attrs.set_code_hints(&renderable::tree::CodeRenderHints {
                header_row: true,
                language_label: lang,
                highlight: true,
            });
            node
        }
        ContainerKind::List { ordered, start } => RenderNode::list(ordered, start, children),
        ContainerKind::Item { checked } => RenderNode::list_item(checked, children),
        ContainerKind::Table(align) => RenderNode::table(align, children),
        ContainerKind::TableRow => RenderNode::table_row(children),
        ContainerKind::TableCell => RenderNode::table_cell(children),
        ContainerKind::Emphasis => RenderNode::emphasis(children),
        ContainerKind::Strong => RenderNode::strong(children),
        ContainerKind::Span { classes } => RenderNode::span(classes, children),
        ContainerKind::FootnoteDefinition { identifier } => {
            RenderNode::footnote_definition(identifier, children)
        }
        ContainerKind::Link { url, title } => RenderNode::link(url, title, children),
        ContainerKind::Image { url, title } => RenderNode::image(url, title, image_alt(&children)),
        // Handled by `Fold::end`; unreachable here. `Delete` is intercepted by
        // `Fold::end`'s strikethrough dispatch before it can reach this match.
        ContainerKind::Root
        | ContainerKind::HtmlBlock
        | ContainerKind::EmbedRegion { .. }
        | ContainerKind::TableHead
        | ContainerKind::Delete
        | ContainerKind::Unsupported { .. } => RenderNode::unsupported("internal: unhandled"),
    }
}

/// Peeks the first child of a closed `~~…~~` container for a darkmatter
/// inline-extension opener marker, returning the token name when present.
///
/// The rewriter emits `{{!TOKEN!}}\u{FDD0}` as the opener, so an extension
/// container always begins with a text node whose value starts with that
/// marker. The U+FDD0 sentinel must immediately follow the `{{!TOKEN!}}` form;
/// a bare `{{!TOKEN!}}` a user typed in prose (with no adjacent sentinel) is
/// not an envelope and returns `None`.
fn envelope_token(children: &[RenderNode]) -> Option<String> {
    let NodeKind::Text { value } = &children.first()?.kind else {
        return None;
    };
    let rest = value.strip_prefix("{{!")?;
    let (name, after) = rest.split_once("!}}")?;
    after.strip_prefix(super::inline_extension::ENVELOPE_SENTINEL)?;
    if name.is_empty() {
        None
    } else {
        Some(name.to_string())
    }
}

/// Strips the `{{!name!}}\u{FDD0}` opener from the first text child and the
/// matching closer from the last text child of an envelope container.
///
/// The marker bytes are removed in the rewritten-offset space the fold builds
/// its spans in; the start/end of each stripped node's `SourceLocation` is
/// advanced/retracted by the marker length so the later provenance pass
/// resolves the remaining payload bytes back to the original source. Boundary
/// text nodes left empty by stripping are dropped.
fn strip_envelope_markers(mut children: Vec<RenderNode>, name: &str) -> Vec<RenderNode> {
    let marker = format!("{{{{!{name}!}}}}{}", super::inline_extension::ENVELOPE_SENTINEL);
    if let Some(first) = children.first_mut() {
        strip_text_prefix(first, &marker);
    }
    if let Some(last) = children.last_mut() {
        strip_text_suffix(last, &marker);
    }
    children.retain(|child| !matches!(&child.kind, NodeKind::Text { value } if value.is_empty()));
    children
}

/// Removes `marker` from the start of a text `node`, advancing its
/// rewritten-space source range by the removed byte length. A no-op unless
/// `node` is a text node beginning with `marker`.
fn strip_text_prefix(node: &mut RenderNode, marker: &str) {
    if let NodeKind::Text { value } = &mut node.kind
        && let Some(rest) = value.strip_prefix(marker)
    {
        let removed = value.len() - rest.len();
        *value = rest.to_string();
        if let Some(location) = node.span.location.as_mut() {
            location.bytes.start += removed;
        }
    }
}

/// Removes `marker` from the end of a text `node`, retracting its
/// rewritten-space source range by the removed byte length. A no-op unless
/// `node` is a text node ending with `marker`.
fn strip_text_suffix(node: &mut RenderNode, marker: &str) {
    if let NodeKind::Text { value } = &mut node.kind
        && let Some(rest) = value.strip_suffix(marker)
    {
        let removed = value.len() - rest.len();
        *value = rest.to_string();
        if let Some(location) = node.span.location.as_mut() {
            location.bytes.end -= removed;
        }
    }
}

/// Maps every `SourceLocation` byte range in the tree rooted at `node` from
/// rewritten-source offsets back to original-source offsets.
///
/// The span-aware fold parses a rewritten source string, so each node's range
/// initially indexes the rewritten bytes. This pass walks the whole tree and
/// applies the rewriter's [`ProvenanceTable`](super::inline_extension::ProvenanceTable)
/// so downstream consumers see the bytes the user actually typed.
fn resolve_node_spans(node: &mut RenderNode, provenance: &super::inline_extension::ProvenanceTable) {
    if let Some(location) = node.span.location.as_mut() {
        location.bytes = provenance.resolve_range(location.bytes.clone());
    }
    if let Some(children) = node.children_mut() {
        for child in children {
            resolve_node_spans(child, provenance);
        }
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
    fn embedded_render_tree_marker_splices_decoded_subtree_and_drops_fallback() {
        use renderable::style::{Style, TextEmphasis};
        use renderable::tree::encode_embedded_subtree;

        // A styled span whose dim weight + class have no plain-Markdown form.
        let mut span =
            RenderNode::span(vec!["fs-dir".to_string()], vec![RenderNode::text("topics")]);
        span.attrs.set_style(&Style {
            emphasis: TextEmphasis {
                dim: true,
                ..TextEmphasis::default()
            },
            ..Style::default()
        });
        let block = encode_embedded_subtree(&span).unwrap();
        let input = format!("Intro\n\n{block}\n\nOutro\n");

        let (doc, _diags) = fold(&input);

        fn collect<'a>(node: &'a RenderNode, out: &mut Vec<&'a RenderNode>) {
            if matches!(&node.kind, NodeKind::Span { .. })
                && node.attrs.classes.iter().any(|c| c == "fs-dir")
            {
                out.push(node);
            }
            for child in node.children() {
                collect(child, out);
            }
        }
        let mut spans = Vec::new();
        collect(&doc.root, &mut spans);
        assert_eq!(spans.len(), 1, "decoded span spliced exactly once");
        assert!(
            spans[0].attrs.style().unwrap().emphasis.dim,
            "dim styling survived the round-trip"
        );

        // The raw marker comment is consumed, not left as an Html node, and the
        // portable fallback paragraph is dropped in favor of the decoded span.
        fn has_marker_html(node: &RenderNode) -> bool {
            if let NodeKind::Html { value, .. } = &node.kind
                && value.contains("bt:render-tree")
            {
                return true;
            }
            node.children().iter().any(has_marker_html)
        }
        assert!(!has_marker_html(&doc.root), "marker comments are consumed");
    }

    /// Builds a styled `fs-dir` span wrapping `text`, whose dim weight has no
    /// plain-Markdown form, then encodes it as an embedded-subtree block.
    fn embed_block(text: &str) -> String {
        use renderable::style::{Style, TextEmphasis};
        use renderable::tree::encode_embedded_subtree;

        let mut span =
            RenderNode::span(vec!["fs-dir".to_string()], vec![RenderNode::text(text)]);
        span.attrs.set_style(&Style {
            emphasis: TextEmphasis {
                dim: true,
                ..TextEmphasis::default()
            },
            ..Style::default()
        });
        encode_embedded_subtree(&span).unwrap()
    }

    /// Collects every `fs-dir` span in the tree rooted at `node`.
    fn collect_fs_dir_spans<'a>(node: &'a RenderNode, out: &mut Vec<&'a RenderNode>) {
        if matches!(&node.kind, NodeKind::Span { .. })
            && node.attrs.classes.iter().any(|c| c == "fs-dir")
        {
            out.push(node);
        }
        for child in node.children() {
            collect_fs_dir_spans(child, out);
        }
    }

    /// Whether any node in the tree is an `Html` node leaking the marker text.
    fn leaks_marker_html(node: &RenderNode) -> bool {
        if let NodeKind::Html { value, .. } = &node.kind
            && value.contains("bt:render-tree")
        {
            return true;
        }
        node.children().iter().any(leaks_marker_html)
    }

    /// Whether any text node in the tree rooted at `node` equals `needle`.
    fn has_text(node: &RenderNode, needle: &str) -> bool {
        if let NodeKind::Text { value } = &node.kind
            && value == needle
        {
            return true;
        }
        node.children().iter().any(|c| has_text(c, needle))
    }

    #[test]
    fn embedded_render_tree_missing_close_marker_restores_fallback_and_diagnoses() {
        // Drop the trailing closing-marker line so the region is unterminated.
        let block = embed_block("topics");
        let without_close = block
            .strip_suffix(renderable::tree::EMBED_CLOSE)
            .expect("block ends with a close marker")
            .trim_end()
            .to_string();
        let input = format!("Intro\n\n{without_close}\n\nOutro\n");

        let (doc, diags) = fold(&input);

        assert!(
            diags
                .iter()
                .any(|d| d.message.contains("unterminated embedded render-tree region")),
            "an unterminated region must be diagnosed: {diags:?}",
        );
        // The decoded subtree is never spliced (the region never closed), but
        // the buffered portable fallback — text "topics" — is restored rather
        // than dropped, and content after the marker ("Outro") survives.
        assert!(
            has_text(&doc.root, "topics"),
            "the portable fallback must be restored, not dropped",
        );
        assert!(
            has_text(&doc.root, "Outro"),
            "content after the opening marker must survive",
        );
    }

    #[test]
    fn embedded_render_tree_duplicate_close_marker_is_tolerated() {
        let block = embed_block("topics");
        let input = format!("{block}\n\n{}\n\nOutro\n", renderable::tree::EMBED_CLOSE);

        let (doc, _diags) = fold(&input);

        let mut spans = Vec::new();
        collect_fs_dir_spans(&doc.root, &mut spans);
        assert_eq!(
            spans.len(),
            1,
            "the well-formed region splices exactly once; the stray close adds none",
        );
        assert!(
            has_text(&doc.root, "Outro"),
            "the stray close must not corrupt the trailing content",
        );
    }

    #[test]
    fn embedded_render_tree_adjacent_regions_both_splice() {
        let block_a = embed_block("alpha");
        let block_b = embed_block("beta");
        let input = format!("{block_a}\n\n{block_b}\n");

        let (doc, _diags) = fold(&input);

        let mut spans = Vec::new();
        collect_fs_dir_spans(&doc.root, &mut spans);
        assert_eq!(spans.len(), 2, "both adjacent regions splice their subtrees");
        assert!(
            has_text(&doc.root, "alpha") && has_text(&doc.root, "beta"),
            "each decoded subtree carries its distinguishing text",
        );
        assert!(
            !leaks_marker_html(&doc.root),
            "no marker comment leaks as visible Html",
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
            ctx: None,
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
        .expect("span-aware fold must succeed")
    }

    /// Visits `node` and every descendant, collecting `Extended` nodes whose
    /// `token` equals `token`.
    fn collect_extended<'a>(node: &'a RenderNode, token: &str) -> Vec<&'a RenderNode> {
        let mut out = Vec::new();
        fn walk<'a>(node: &'a RenderNode, token: &str, out: &mut Vec<&'a RenderNode>) {
            if matches!(&node.kind, NodeKind::Extended { token: t, .. } if t == token) {
                out.push(node);
            }
            for child in node.children() {
                walk(child, token, out);
            }
        }
        walk(node, token, &mut out);
        out
    }

    #[test]
    fn span_aware_fold_emits_mark_extended_node() {
        let (doc, diags) = fold_spanned("plain ==highlighted== after");
        assert!(diags.is_empty(), "clean fixture must fold cleanly");
        let marks = collect_extended(&doc.root, "mark");
        assert_eq!(marks.len(), 1, "expected one mark Extended node");
        // The mark contains the highlighted text.
        let mut text = String::new();
        collect_text(marks[0].children(), &mut text);
        assert_eq!(text, "highlighted");
    }

    /// Locates the first descendant `Extended` node carrying the given `token`.
    fn find_extended<'a>(node: &'a RenderNode, token: &str) -> Option<&'a RenderNode> {
        if matches!(&node.kind, NodeKind::Extended { token: t, .. } if t == token) {
            return Some(node);
        }
        for child in node.children() {
            if let Some(found) = find_extended(child, token) {
                return Some(found);
            }
        }
        None
    }

    #[test]
    fn span_aware_fold_emits_dim_extended_node() {
        let (doc, diags) = fold_spanned("normal \u{2304}dimmed\u{2304} after");
        assert!(diags.is_empty());
        let dim = find_extended(&doc.root, "dim").expect("dim Extended node must exist");
        let mut text = String::new();
        collect_text(dim.children(), &mut text);
        assert_eq!(text, "dimmed");
    }

    #[test]
    fn span_aware_fold_ordinary_strikethrough_stays_delete() {
        // A real GFM strikethrough carries no envelope marker, so it must fold
        // to `NodeKind::Delete`, never an `Extended` node.
        let (doc, diags) = fold_spanned("plain ~~struck~~ after");
        assert!(diags.is_empty(), "clean fixture must fold cleanly: {diags:?}");
        assert!(
            collect_extended(&doc.root, "mark").is_empty()
                && collect_extended(&doc.root, "dim").is_empty(),
            "ordinary strikethrough must not produce an Extended node",
        );
        fn find_delete(node: &RenderNode) -> Option<&RenderNode> {
            if matches!(node.kind, NodeKind::Delete { .. }) {
                return Some(node);
            }
            node.children().iter().find_map(find_delete)
        }
        let delete = find_delete(&doc.root).expect("ordinary `~~…~~` must fold to Delete");
        let mut text = String::new();
        collect_text(delete.children(), &mut text);
        assert_eq!(text, "struck");
    }

    #[test]
    fn span_aware_fold_unknown_token_diagnoses_and_falls_back_to_delete() {
        // A synthetic envelope whose token is not registered is only reachable
        // on a rewriter/registry drift bug. The fold must record a diagnostic
        // and degrade to a standard `Delete` rather than emit an `Extended`
        // node for an unknown token. The U+FDD0 sentinel cannot be typed by a
        // user, so this envelope is constructed directly.
        let md: crate::markdown::Markdown =
            "~~{{!bogus!}}\u{FDD0}content{{!bogus!}}\u{FDD0}~~".into();
        let (doc, diags) = fold_markdown_spanned_with_frontmatter(
            SourceDescriptor::Virtual {
                name: "spanned".into(),
            },
            &md,
        )
        .expect("span-aware fold must succeed");
        assert!(
            collect_extended(&doc.root, "bogus").is_empty(),
            "unknown token must not produce an Extended node",
        );
        assert!(
            diags
                .iter()
                .any(|d| d.message.contains("unknown inline extension token")),
            "unknown token must record a diagnostic: {diags:?}",
        );
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
        let tb = hr.attrs.thematic_break_ref().expect("HR styling attached");
        assert_eq!(tb.kind, Some(HrKind::Waves));
        assert_eq!(tb.width.as_deref(), Some("50%"));
        // Generated provenance: this HR was synthesized from a paragraph.
        assert_eq!(hr.span.provenance, Provenance::Generated);
        assert!(hr.span.location.is_some());
    }

    /// An HR-attribute paragraph whose quoted scalar contains delimiter-like
    /// text (`==…==` / `⌄…⌄`) must still lift to a generated `ThematicBreak`.
    /// The block-extension construct owns the whole paragraph, so the inline
    /// rewriter must leave the body untouched — otherwise the embedded `==`
    /// would be rewritten into a strikethrough envelope, split the paragraph,
    /// and defeat HR recognition, leaking raw `---`/`==` source into the tree.
    /// Regression guard for the spec's block-before-inline sequencing.
    #[test]
    fn span_aware_fold_hr_attribute_with_delimiter_scalar_still_lifts_to_rule() {
        for input in [
            "--- { kind: \"==waves==\" }\n",
            "--- { kind: \"\u{2304}dots\u{2304}\" }\n",
        ] {
            let (doc, diags) = fold_spanned(input);
            assert!(diags.is_empty(), "HR fixture must fold cleanly: {diags:?}");

            fn find_hr(node: &RenderNode) -> Option<&RenderNode> {
                if matches!(node.kind, NodeKind::ThematicBreak) {
                    return Some(node);
                }
                node.children().iter().find_map(find_hr)
            }
            let hr = find_hr(&doc.root)
                .unwrap_or_else(|| panic!("ThematicBreak must exist for {input:?}"));
            assert_eq!(hr.span.provenance, Provenance::Generated);

            // The delimiter-like scalar must survive verbatim as the HR kind,
            // never be re-interpreted as a mark/dim inline extension.
            assert!(
                collect_extended(&doc.root, "mark").is_empty()
                    && collect_extended(&doc.root, "dim").is_empty(),
                "HR-attribute body must not fold to an inline Extended node: {input:?}",
            );

            // No raw HR-paragraph source may leak as visible text.
            let mut text = String::new();
            collect_text(doc.root.children(), &mut text);
            assert!(
                !text.contains("---") && !text.contains("=="),
                "raw HR source leaked into the tree for {input:?}: {text:?}",
            );
        }
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
        assert!(
            hr.attrs.thematic_break_ref().is_none(),
            "plain rule must not carry HR styling"
        );
    }

    /// A mark envelope followed by an emphasis must keep both as separate,
    /// populated children of the paragraph — the mark folds to its own
    /// `Extended` node and the emphasis stays an independent sibling.
    #[test]
    fn span_aware_fold_preserves_emphasis_sibling_after_mark() {
        let (doc, diags) = fold_spanned("==marked== then *italic*");
        assert!(
            diags.is_empty(),
            "clean fixture must fold cleanly: {diags:?}"
        );
        let para = &doc.root.children()[0];

        let marks = collect_extended(&doc.root, "mark");
        assert_eq!(marks.len(), 1, "expected one mark Extended node");
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

    /// `==*highlighted*==` must fold to a mark `Extended` node whose child is
    /// the Emphasis container. The rewriter preserves the `*emphasized*`
    /// payload verbatim inside the envelope, so `pulldown-cmark` parses the
    /// emphasis structurally and the dispatcher keeps it as a nested child.
    #[test]
    fn span_aware_fold_wraps_emphasis_inside_mark() {
        let (doc, diags) = fold_spanned("==*highlighted*== rest");
        assert!(
            diags.is_empty(),
            "clean fixture must fold cleanly: {diags:?}"
        );
        let marks = collect_extended(&doc.root, "mark");
        assert_eq!(marks.len(), 1, "expected one mark Extended node");
        let mark = marks[0];
        let first = mark
            .children()
            .first()
            .expect("mark node must have at least one child");
        assert!(
            matches!(first.kind, NodeKind::Emphasis { .. }),
            "expected Emphasis inside mark; got {:?}",
            first.kind,
        );
        let mut text = String::new();
        collect_text(first.children(), &mut text);
        assert_eq!(text, "highlighted");
    }

    /// The `⌄*dim and italic*⌄` fixture must fold to a dim `Extended` node
    /// whose single child is the Emphasis container.
    #[test]
    fn span_aware_fold_wraps_emphasis_inside_dim() {
        let (doc, diags) = fold_spanned("\u{2304}*dim and italic*\u{2304}");
        assert!(
            diags.is_empty(),
            "clean fixture must fold cleanly: {diags:?}"
        );
        let dim = find_extended(&doc.root, "dim").expect("dim Extended node must exist");
        let first = dim
            .children()
            .first()
            .expect("dim node must have at least one child");
        assert!(
            matches!(first.kind, NodeKind::Emphasis { .. }),
            "expected Emphasis inside dim; got {:?}",
            first.kind,
        );
        let mut text = String::new();
        collect_text(first.children(), &mut text);
        assert_eq!(text, "dim and italic");
    }

    /// An unclosed `==` must revert to literal text rather than emit a mark
    /// node. The rewriter finds no closing delimiter and leaves the `==`
    /// untouched, so `pulldown-cmark` never sees a strikethrough envelope.
    #[test]
    fn span_aware_fold_unclosed_cross_event_mark_reverts() {
        let (doc, _diags) = fold_spanned("==*never closed* and on");
        let marks = collect_extended(&doc.root, "mark");
        assert!(
            marks.is_empty(),
            "unclosed mark must not emit an Extended node: {marks:?}",
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

    /// Mixed mark/dim nesting: `==highlighted and ⌄dim within mark⌄==` must
    /// fold to a mark `Extended` node containing a nested dim `Extended` node.
    /// The rewriter emits nested strikethrough envelopes, which
    /// `pulldown-cmark` parses as nested `Strikethrough` containers, and the
    /// dispatcher lowers each to its own `Extended` node.
    #[test]
    fn span_aware_fold_nests_dim_inside_mark() {
        let (doc, diags) = fold_spanned("==highlighted and \u{2304}dim within mark\u{2304}==");
        assert!(
            diags.is_empty(),
            "clean fixture must fold cleanly: {diags:?}"
        );

        let marks = collect_extended(&doc.root, "mark");
        assert_eq!(marks.len(), 1, "expected one mark Extended node: {marks:?}");
        let mark = marks[0];

        // The mark contains literal text plus a nested dim node.
        let dim = mark
            .children()
            .iter()
            .find(|n| matches!(&n.kind, NodeKind::Extended { token, .. } if token == "dim"))
            .expect("mark must contain a nested dim Extended node");
        let mut dim_text = String::new();
        collect_text(dim.children(), &mut dim_text);
        assert_eq!(dim_text, "dim within mark");

        // The leading text "highlighted and " must precede the dim node
        // inside the mark.
        let mut leading = String::new();
        for child in mark.children() {
            if matches!(&child.kind, NodeKind::Extended { token, .. } if token == "dim") {
                break;
            }
            collect_text(std::slice::from_ref(child), &mut leading);
        }
        assert_eq!(leading.trim_end(), "highlighted and");
    }

    /// Mixed nesting in the reverse direction: a mark envelope inside a dim
    /// envelope yields a dim `Extended` node containing a nested mark
    /// `Extended` node. Guards the symmetric nesting of the dispatcher.
    #[test]
    fn span_aware_fold_nests_mark_inside_dim() {
        let (doc, diags) = fold_spanned("\u{2304}dim with ==marked inside==\u{2304}");
        assert!(
            diags.is_empty(),
            "clean fixture must fold cleanly: {diags:?}"
        );

        let dim = find_extended(&doc.root, "dim").expect("dim Extended node must exist");
        let nested_mark = dim
            .children()
            .iter()
            .find(|n| matches!(&n.kind, NodeKind::Extended { token, .. } if token == "mark"))
            .expect("dim must contain a nested mark Extended node");
        let mut nested_text = String::new();
        collect_text(nested_mark.children(), &mut nested_text);
        assert_eq!(nested_text, "marked inside");
    }

    #[test]
    fn span_aware_fold_emits_no_mark_for_escaped_delimiter() {
        // `\==` is escaped, so the rewriter leaves it literal and no envelope
        // is emitted — the dispatcher therefore never produces a mark node.
        let (doc, diags) = fold_spanned("foo \\== not highlighted\n");
        assert!(diags.is_empty());
        let marks = collect_extended(&doc.root, "mark");
        assert!(
            marks.is_empty(),
            "escaped mark delimiter must not produce an Extended node"
        );
    }

    // -----------------------------------------------------------------------
    // Provenance (fold tier): the fold parses a rewritten source, so every
    // node and diagnostic range is mapped back to the original source through
    // the rewriter's provenance table. These tests pin the resolved
    // `SourceLocation.bytes` that downstream tools and diagnostics consume.
    // -----------------------------------------------------------------------

    /// `plain ==highlighted== after` must fold to a mark `Extended` node whose
    /// `SourceLocation.bytes` resolves back to the full **original** delimited
    /// region `6..21` (`==` opener at `6..8`, `highlighted` at `8..19`, `==`
    /// closer at `19..21`) — not to the synthetic envelope bytes the fold
    /// actually parsed. Pins the provenance round-trip for the container span.
    #[test]
    fn span_aware_fold_mark_container_span_covers_full_delimited_region() {
        let input = "plain ==highlighted== after";
        let (doc, diags) = fold_spanned(input);
        assert!(
            diags.is_empty(),
            "clean fixture must fold cleanly: {diags:?}"
        );
        let marks = collect_extended(&doc.root, "mark");
        assert_eq!(marks.len(), 1, "expected exactly one mark Extended node");
        let mark = marks[0];
        let location = mark
            .span
            .location
            .as_ref()
            .expect("mark node must carry a SourceLocation");
        assert_eq!(
            location.bytes,
            6..21,
            "mark container span must resolve to the original opener, inner text, and closer",
        );
        assert_eq!(&input[location.bytes.clone()], "==highlighted==");
    }

    /// The inner payload of a mark envelope must resolve to the original
    /// payload bytes. For `plain ==highlighted== after` the `highlighted` text
    /// node sits at original bytes `8..19` once the synthetic markers are
    /// stripped and provenance is applied.
    #[test]
    fn span_aware_fold_mark_inner_text_resolves_to_original_payload_bytes() {
        let input = "plain ==highlighted== after";
        let (doc, diags) = fold_spanned(input);
        assert!(diags.is_empty(), "clean fixture must fold cleanly: {diags:?}");
        let mark = find_extended(&doc.root, "mark").expect("mark Extended node must exist");
        let text = mark
            .children()
            .iter()
            .find(|n| matches!(&n.kind, NodeKind::Text { value } if value == "highlighted"))
            .expect("mark must contain the highlighted text node");
        let location = text
            .span
            .location
            .as_ref()
            .expect("inner text must carry a SourceLocation");
        assert_eq!(location.bytes, 8..19);
        assert_eq!(&input[location.bytes.clone()], "highlighted");
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

    /// When an earlier inline rewrite shifts byte offsets, the generated HR
    /// `SourceLocation.bytes` must still resolve to the **original** paragraph
    /// body range — not the rewritten-source range the fold actually parsed.
    /// The leading `==marked==` rewrites to a canonical envelope longer than
    /// `==marked==`, so every byte after it is shifted in the parsed source;
    /// provenance must undo that shift for the HR span. This is the regression
    /// guard for HR source-span parity behind an offset-shifting rewrite.
    #[test]
    fn span_aware_fold_hr_source_location_survives_earlier_inline_rewrite() {
        let body = "--- { style: waves }";
        let input = format!("Lead ==marked== text.\n\n{body}\n");
        let body_start = input.find(body).expect("HR body must be present in source");
        let expected = body_start..body_start + body.len();

        let (doc, diags) = fold_spanned(&input);
        assert!(diags.is_empty(), "HR fixture must fold cleanly: {diags:?}");

        // Sanity: the leading inline extension actually folded, so the rewrite
        // ran and offsets were shifted before the HR paragraph.
        assert_eq!(
            collect_extended(&doc.root, "mark").len(),
            1,
            "leading ==marked== must fold to one mark Extended node",
        );

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
            location.bytes, expected,
            "generated HR SourceLocation must resolve to the original body range despite the rewrite",
        );
        assert_eq!(&input[location.bytes.clone()], body);
    }

    /// An escaped `\==` must survive as literal `==` visible text and produce
    /// no mark node. The rewriter leaves the escaped delimiter untouched, so
    /// `pulldown-cmark` applies its own CommonMark escape (consuming the
    /// backslash) and the `==` reaches the fold as ordinary text.
    #[test]
    fn span_aware_fold_escaped_mark_stays_literal_text() {
        let input = "foo \\== bar";
        let (doc, diags) = fold_spanned(input);
        assert!(
            diags.is_empty(),
            "escape fixture must fold cleanly: {diags:?}"
        );

        assert!(
            collect_extended(&doc.root, "mark").is_empty(),
            "escaped `\\==` must not produce a mark Extended node",
        );

        // The literal `==` must survive as visible text.
        let mut text = String::new();
        collect_text(doc.root.children(), &mut text);
        assert!(
            text.contains("=="),
            "escaped delimiter must keep a literal `==`: {text:?}",
        );
    }

    // -----------------------------------------------------------------------
    // Verbatim-region protection at the fold tier (review-1 finding 1). The
    // source rewriter protects code/HTML/link/image regions, so the fold must
    // never see an envelope for delimiters that lived inside them.
    // -----------------------------------------------------------------------

    /// `==code==` inside an inline code span must stay literal: the fold sees an
    /// `InlineCode` node carrying the delimiters verbatim, not a mark `Extended`.
    #[test]
    fn span_aware_fold_inline_code_keeps_delimiters_literal() {
        let (doc, diags) = fold_spanned("text `==code==` more");
        assert!(diags.is_empty(), "clean fixture must fold cleanly: {diags:?}");
        assert!(
            collect_extended(&doc.root, "mark").is_empty(),
            "an inline code span must not produce a mark Extended node",
        );
        fn find_code(node: &RenderNode) -> Option<&RenderNode> {
            if matches!(node.kind, NodeKind::InlineCode { .. }) {
                return Some(node);
            }
            node.children().iter().find_map(find_code)
        }
        let code = find_code(&doc.root).expect("inline code node must exist");
        match &code.kind {
            NodeKind::InlineCode { value } => assert_eq!(value, "==code=="),
            other => panic!("expected inline code, got {other:?}"),
        }
    }

    /// A fenced code block containing `==`/`⌄` must preserve them verbatim in
    /// the `Code` node body and emit no `Extended` nodes.
    #[test]
    fn span_aware_fold_fenced_code_keeps_delimiters_literal() {
        let (doc, diags) = fold_spanned("```\n==code== and \u{2304}dim\u{2304}\n```\n");
        assert!(diags.is_empty(), "clean fixture must fold cleanly: {diags:?}");
        assert!(
            collect_extended(&doc.root, "mark").is_empty()
                && collect_extended(&doc.root, "dim").is_empty(),
            "a fenced code block must not produce Extended nodes",
        );
        let code = &doc.root.children()[0];
        match &code.kind {
            NodeKind::Code { value, .. } => {
                assert!(
                    value.contains("==code==") && value.contains('\u{2304}'),
                    "code body must keep delimiters verbatim: {value:?}",
                );
            }
            other => panic!("expected code block, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // Table-cell inline extensions (review-1 finding 2). A pipe-free envelope
    // keeps GFM cell boundaries intact, so mark/dim render inside table cells.
    // -----------------------------------------------------------------------

    /// `==hi==` in a table header cell must fold to a mark `Extended` node while
    /// the table keeps its two columns — proving the envelope injected no stray
    /// `|` bytes that would split the row.
    #[test]
    fn span_aware_fold_mark_inside_table_cell_preserves_columns() {
        let (doc, diags) = fold_spanned("| ==hi== | ok |\n|----|----|\n| a | b |\n");
        assert!(diags.is_empty(), "clean fixture must fold cleanly: {diags:?}");

        fn find_table(node: &RenderNode) -> Option<&RenderNode> {
            if matches!(node.kind, NodeKind::Table { .. }) {
                return Some(node);
            }
            node.children().iter().find_map(find_table)
        }
        let table = find_table(&doc.root).expect("table node must exist");
        let NodeKind::Table { align, children: rows } = &table.kind else {
            unreachable!("matched above");
        };
        assert_eq!(align.len(), 2, "table must keep exactly two columns");
        // Header row first; it must carry two cells, not extra ones split off a
        // stray marker pipe.
        let header = &rows[0];
        assert_eq!(
            header.children().len(),
            2,
            "header row must keep two cells: {:?}",
            header.children(),
        );

        // The first header cell holds the mark Extended node with text "hi".
        let mark = find_extended(&doc.root, "mark").expect("mark Extended node must exist");
        let mut mark_text = String::new();
        collect_text(mark.children(), &mut mark_text);
        assert_eq!(mark_text, "hi");
    }
}
