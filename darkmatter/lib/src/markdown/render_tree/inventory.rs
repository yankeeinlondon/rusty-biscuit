//! Verified `pulldown-cmark` 0.13 event inventory.
//!
//! This module is the authoritative, compile-checked catalog of every
//! [`pulldown_cmark`] 0.13 [`Event`], [`Tag`], and [`TagEnd`] variant, paired
//! with the [`Disposition`] the render-tree fold (a later phase) must apply to
//! it. It contains **no fold logic** — only verification and documentation.
//!
//! The inventory was verified against `pulldown-cmark` **0.13.3** (the version
//! pinned by `darkmatter/lib/Cargo.toml` as `pulldown-cmark = "0.13"`).
//!
//! ## Disposition catalog
//!
//! Each variant the parser can emit is assigned exactly one [`Disposition`]:
//!
//! - [`Node`](Disposition::Node) — maps to a [`renderable::tree::NodeKind`].
//! - [`Attr`](Disposition::Attr) — folded into an enclosing node's `attrs`
//!   (or another node field) rather than producing its own node.
//! - [`Meta`](Disposition::Meta) — folded into
//!   [`renderable::tree::DocumentMetadata`].
//! - [`Lossy`](Disposition::Lossy) — produces a node, but some payload detail
//!   is intentionally dropped by the v1 mapping.
//! - [`Unsupported`](Disposition::Unsupported) — has no canonical tree
//!   representation in v1; the fold emits [`NodeKind::Unsupported`].
//! - [`Noise`](Disposition::Noise) — purely structural; carries no content of
//!   its own (the matching [`Event::End`] of a balanced pair).
//!
//! [`NodeKind::Unsupported`]: renderable::tree::NodeKind::Unsupported
//!
//! ## Options the fold enables
//!
//! The render-tree fold ([`fold_markdown_to_document`]) constructs its parser
//! with:
//!
//! ```text
//! Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH
//!   | Options::ENABLE_TASKLISTS | Options::ENABLE_FOOTNOTES
//!   | Options::ENABLE_SUPERSCRIPT | Options::ENABLE_SUBSCRIPT
//! ```
//!
//! [`fold_markdown_to_document`]: super::fold::fold_markdown_to_document
//!
//! Consequently the fold **does not** enable math, definition lists, heading
//! attributes, or metadata blocks. Variants gated behind those flags
//! (`InlineMath`, `DisplayMath`, `DefinitionList*`, `MetadataBlock`, and
//! heading `id`/`classes`/`attrs`) are **never produced by the fold** today.
//! They are still inventoried below — fully and exhaustively — because the fold
//! must remain total: a wildcard-free `match` over the real enums is the only
//! safe contract.
//!
//! ## Intentionally deferred: `==mark==`/dim and HR-with-attributes
//!
//! Two darkmatter inline conveniences are **deliberately not folded** and are
//! deferred to a follow-up feature: `==mark==` / dim inline styles and
//! horizontal rules carrying an attribute block. These are produced by
//! darkmatter's `InlineStyleProcessor` and `RuleProcessor`, which are iterator
//! adapters bounded `where I: Iterator<Item = Event<'a>>`. They cannot consume
//! the fold's `OffsetIter` (item `(Event, Range)`) and they destroy source byte
//! ranges — splitting `Event::Text` and replacing whole paragraphs — so routing
//! the fold through them would strip every node of its `SourceLocation`, a
//! regression. Reconciling offset preservation with those processors needs a
//! separate design decision; until then the fold reads the raw
//! `pulldown-cmark` stream and `==mark==` text and bare `---` rules fold to
//! ordinary `Text` / `ThematicBreak` nodes.
//!
//! Note: `darkmatter` extracts YAML frontmatter itself (see
//! `markdown::frontmatter`) *before* handing content to `pulldown-cmark`, so
//! frontmatter never reaches the parser as a `MetadataBlock` on the render
//! path regardless of `Options`.
//!
//! ## Verified disposition table
//!
//! Every variant of the real 0.13.3 enums, with corrections noted against the
//! Phase 4 working spec.
//!
//! ### `Event` variants
//!
//! | Variant | Disposition | Note |
//! |---|---|---|
//! | `Start(Tag)` | (see `Tag` table) | Delegates to the tag's disposition. |
//! | `End(TagEnd)` | `Noise` | Closes a balanced pair; the fold pops its builder stack. |
//! | `Text(CowStr)` | `Node` | `NodeKind::Text`. |
//! | `Code(CowStr)` | `Node` | `NodeKind::InlineCode`. |
//! | `InlineMath(CowStr)` | `Unsupported` | Requires `ENABLE_MATH`; not enabled. Revisit. |
//! | `DisplayMath(CowStr)` | `Unsupported` | Requires `ENABLE_MATH`; not enabled. Revisit. |
//! | `Html(CowStr)` | `Noise` | One line inside an HTML block; the fold concatenates these into the block's single `NodeKind::Html { block: true }`. |
//! | `InlineHtml(CowStr)` | `Node` | `NodeKind::Html { block: false }`. |
//! | `FootnoteReference(CowStr)` | `Node` | `NodeKind::FootnoteReference`. Requires `ENABLE_FOOTNOTES`. |
//! | `SoftBreak` | `Node` | `NodeKind::SoftBreak`. |
//! | `HardBreak` | `Node` | `NodeKind::HardBreak`. |
//! | `Rule` | `Node` | `NodeKind::ThematicBreak`. |
//! | `TaskListMarker(bool)` | `Attr` | Sets the enclosing `ListItem.checked`. Requires `ENABLE_TASKLISTS`. |
//!
//! ### `Tag` variants (carried by `Event::Start`)
//!
//! | Variant | Disposition | Note |
//! |---|---|---|
//! | `Paragraph` | `Node` | `NodeKind::Paragraph`. |
//! | `Heading { level, id, classes, attrs }` | `Node` | `NodeKind::Heading`. `level` → `depth`; `id` → `attrs.id`. `classes`/`attrs` only populated with `ENABLE_HEADING_ATTRIBUTES`. |
//! | `BlockQuote(Option<BlockQuoteKind>)` | `Lossy` | `NodeKind::BlockQuote`. The optional GFM alert kind (`Note`/`Tip`/`Important`/`Warning`/`Caution`) has no tree field in v1 and is dropped. |
//! | `CodeBlock(CodeBlockKind)` | `Node` | `NodeKind::Code`. `Fenced(info)` → `lang`/`meta`; `Indented` → `lang: None`. |
//! | `HtmlBlock` | `Node` | `NodeKind::Html { block: true }`. The fold concatenates the contained `Event::Html` lines into one node spanning the block. |
//! | `List(Option<u64>)` | `Node` | `NodeKind::List`. `Some(n)` → `ordered: true, start: Some(n)`; `None` → `ordered: false`. |
//! | `Item` | `Node` | `NodeKind::ListItem`. |
//! | `FootnoteDefinition(CowStr)` | `Node` | `NodeKind::FootnoteDefinition`. Requires `ENABLE_FOOTNOTES`. |
//! | `DefinitionList` | `Unsupported` | Requires `ENABLE_DEFINITION_LIST`; not enabled v1. |
//! | `DefinitionListTitle` | `Unsupported` | Requires `ENABLE_DEFINITION_LIST`; not enabled v1. |
//! | `DefinitionListDefinition` | `Unsupported` | Requires `ENABLE_DEFINITION_LIST`; not enabled v1. |
//! | `Table(Vec<Alignment>)` | `Node` | `NodeKind::Table`. Column `Alignment` → `ColumnAlign`. Requires `ENABLE_TABLES`. |
//! | `TableHead` | `Node` | Folds to a leading `NodeKind::TableRow` (no `TableHead` kind; header rows are ordinary rows). |
//! | `TableRow` | `Node` | `NodeKind::TableRow`. |
//! | `TableCell` | `Node` | `NodeKind::TableCell`. |
//! | `Emphasis` | `Node` | `NodeKind::Emphasis`. |
//! | `Strong` | `Node` | `NodeKind::Strong`. |
//! | `Strikethrough` | `Node` | `NodeKind::Delete`. Requires `ENABLE_STRIKETHROUGH`. |
//! | `Superscript` | `Node` | `NodeKind::Span` with `attrs.classes = ["sup"]`. Requires `ENABLE_SUPERSCRIPT`. |
//! | `Subscript` | `Node` | `NodeKind::Span` with `attrs.classes = ["sub"]`. Requires `ENABLE_SUBSCRIPT`. |
//! | `Link { link_type, dest_url, title, id }` | `Node` | `NodeKind::Link`. `dest_url` → `url`; `title` → `title`. `link_type`/`id` are not represented. |
//! | `Image { link_type, dest_url, title, id }` | `Node` | `NodeKind::Image`. `dest_url` → `url`; `title` → `title`; alt text gathered from child text. |
//! | `MetadataBlock(MetadataBlockKind)` | `Meta` | Folds into `DocumentMetadata`. Requires `ENABLE_YAML_STYLE_METADATA_BLOCKS` or `ENABLE_PLUSES_DELIMITED_METADATA_BLOCKS`. |
//!
//! ## Corrections versus the Phase 4 working spec
//!
//! The working spec table was reconciled against the real 0.13.3 enums. Key
//! corrections:
//!
//! 1. **`HtmlBlock` is a `Tag`, not an `Event`.** 0.13 has a `Tag::HtmlBlock`
//!    start/end pair; the actual HTML text arrives as `Event::Html` lines
//!    *inside* it. The spec line "`HtmlBlock / Html | Node`" conflated the two.
//!    The fold groups the block: `Tag::HtmlBlock` → `Node` (one
//!    `NodeKind::Html { block: true }` spanning the block); the inner
//!    `Event::Html` lines are concatenated into it and so are `Noise`.
//! 2. **`BlockQuote` carries `Option<BlockQuoteKind>`.** 0.13's
//!    `Tag::BlockQuote(Option<BlockQuoteKind>)` and
//!    `TagEnd::BlockQuote(Option<BlockQuoteKind>)` both carry the optional GFM
//!    alert kind. The tree has no field for it, so the disposition is `Lossy`,
//!    not a plain `Node`.
//! 3. **No `TableBody` tag exists.** Per the 0.13 docs, the table body starts
//!    immediately after `TagEnd::TableHead`; rows are plain `TableRow`s.
//! 4. **`Superscript`/`Subscript` exist** as real `Tag`/`TagEnd` variants in
//!    0.13 (gated by `ENABLE_SUPERSCRIPT`/`ENABLE_SUBSCRIPT`). The fold maps
//!    them to `NodeKind::Span` carrying the class `"sup"` / `"sub"`; the
//!    semantic is preserved on `attrs.classes`, so the disposition is `Node`.
//! 5. **`InlineMath`/`DisplayMath` are `Event` variants** (gated by
//!    `ENABLE_MATH`), not tags — confirmed `Unsupported`.
//! 6. **`MetadataBlock` is a `Tag`** carrying `MetadataBlockKind`
//!    (`YamlStyle` / `PlusesStyle`) — confirmed; disposition `Meta`.
//! 7. **`Heading` is a struct variant** `{ level, id, classes, attrs }`, and
//!    `TagEnd::Heading(HeadingLevel)` carries the level. The slug/id comes
//!    from the `id` field (only populated under `ENABLE_HEADING_ATTRIBUTES`),
//!    not from a separate event.
//! 8. **`TagEnd` is a distinct enum** from `Tag` (kept ≤ 2 bytes). `End`
//!    variants `Heading`, `BlockQuote`, `List`, and `MetadataBlock` carry
//!    payload; all other `TagEnd` variants are fieldless.
//! 9. **No variant in the spec table is missing from the real enum**, and the
//!    real enum has no `Event`/`Tag` variant the inventory omits — the
//!    exhaustive-match coverage tests below prove this at compile time.

use pulldown_cmark::{Event, Tag, TagEnd};

/// The render-tree disposition assigned to a parser variant.
///
/// Every [`pulldown_cmark`] [`Event`] / [`Tag`] / [`TagEnd`] variant maps to
/// exactly one disposition. The render-tree fold (a later phase) uses these to
/// decide how each variant becomes — or does not become — part of the
/// canonical [`renderable::tree::Document`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Disposition {
    /// Maps to a [`renderable::tree::NodeKind`] with no loss of represented
    /// payload.
    Node,
    /// Folded into an enclosing node's `attrs` (or another node field) rather
    /// than producing a node of its own.
    Attr,
    /// Folded into [`renderable::tree::DocumentMetadata`].
    Meta,
    /// Produces a node, but some payload detail is intentionally dropped by the
    /// v1 mapping.
    Lossy,
    /// Has no canonical tree representation in v1; the fold emits
    /// [`NodeKind::Unsupported`](renderable::tree::NodeKind::Unsupported).
    Unsupported,
    /// Purely structural; carries no content of its own.
    Noise,
}

impl Disposition {
    /// Returns a short, stable label for the disposition.
    ///
    /// ## Examples
    ///
    /// ```
    /// use darkmatter::markdown::render_tree::inventory::Disposition;
    ///
    /// assert_eq!(Disposition::Node.label(), "Node");
    /// assert_eq!(Disposition::Noise.label(), "Noise");
    /// ```
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Disposition::Node => "Node",
            Disposition::Attr => "Attr",
            Disposition::Meta => "Meta",
            Disposition::Lossy => "Lossy",
            Disposition::Unsupported => "Unsupported",
            Disposition::Noise => "Noise",
        }
    }
}

/// Returns the verified [`Disposition`] for a [`pulldown_cmark`] [`Event`].
///
/// This is the authoritative, exhaustive mapping for `pulldown-cmark` 0.13.
/// The `match` deliberately has **no wildcard arm**: if a future
/// `pulldown-cmark` release changes the [`Event`] enum, this function fails to
/// compile, forcing the inventory to be re-verified before the fold can use
/// it.
///
/// ## Examples
///
/// ```
/// use darkmatter::markdown::render_tree::inventory::{disposition_for_event, Disposition};
/// use pulldown_cmark::Event;
///
/// assert_eq!(disposition_for_event(&Event::SoftBreak), Disposition::Node);
/// assert_eq!(disposition_for_event(&Event::Rule), Disposition::Node);
/// ```
#[must_use]
pub fn disposition_for_event(event: &Event<'_>) -> Disposition {
    match event {
        Event::Start(tag) => disposition_for_tag(tag),
        Event::End(tag_end) => disposition_for_tag_end(tag_end),
        Event::Text(_) => Disposition::Node,
        Event::Code(_) => Disposition::Node,
        Event::InlineMath(_) => Disposition::Unsupported,
        Event::DisplayMath(_) => Disposition::Unsupported,
        Event::Html(_) => Disposition::Noise,
        Event::InlineHtml(_) => Disposition::Node,
        Event::FootnoteReference(_) => Disposition::Node,
        Event::SoftBreak => Disposition::Node,
        Event::HardBreak => Disposition::Node,
        Event::Rule => Disposition::Node,
        Event::TaskListMarker(_) => Disposition::Attr,
    }
}

/// Returns the verified [`Disposition`] for a [`pulldown_cmark`] [`Tag`].
///
/// Exhaustive, wildcard-free mapping for `pulldown-cmark` 0.13. A change to the
/// [`Tag`] enum breaks compilation here.
///
/// ## Examples
///
/// ```
/// use darkmatter::markdown::render_tree::inventory::{disposition_for_tag, Disposition};
/// use pulldown_cmark::Tag;
///
/// assert_eq!(disposition_for_tag(&Tag::Paragraph), Disposition::Node);
/// assert_eq!(disposition_for_tag(&Tag::BlockQuote(None)), Disposition::Lossy);
/// assert_eq!(disposition_for_tag(&Tag::HtmlBlock), Disposition::Node);
/// ```
#[must_use]
pub fn disposition_for_tag(tag: &Tag<'_>) -> Disposition {
    match tag {
        Tag::Paragraph => Disposition::Node,
        Tag::Heading { .. } => Disposition::Node,
        Tag::BlockQuote(_) => Disposition::Lossy,
        Tag::CodeBlock(_) => Disposition::Node,
        Tag::HtmlBlock => Disposition::Node,
        Tag::List(_) => Disposition::Node,
        Tag::Item => Disposition::Node,
        Tag::FootnoteDefinition(_) => Disposition::Node,
        Tag::DefinitionList => Disposition::Unsupported,
        Tag::DefinitionListTitle => Disposition::Unsupported,
        Tag::DefinitionListDefinition => Disposition::Unsupported,
        Tag::Table(_) => Disposition::Node,
        Tag::TableHead => Disposition::Node,
        Tag::TableRow => Disposition::Node,
        Tag::TableCell => Disposition::Node,
        Tag::Emphasis => Disposition::Node,
        Tag::Strong => Disposition::Node,
        Tag::Strikethrough => Disposition::Node,
        Tag::Superscript => Disposition::Node,
        Tag::Subscript => Disposition::Node,
        Tag::Link { .. } => Disposition::Node,
        Tag::Image { .. } => Disposition::Node,
        Tag::MetadataBlock(_) => Disposition::Meta,
    }
}

/// Returns the [`Disposition`] for a [`pulldown_cmark`] [`TagEnd`].
///
/// Every [`TagEnd`] closes a balanced [`Event::Start`]/[`Event::End`] pair and
/// carries no content of its own, so every variant is
/// [`Disposition::Noise`]. The mapping is still written as an exhaustive,
/// wildcard-free `match` so a change to the [`TagEnd`] enum breaks compilation.
///
/// ## Examples
///
/// ```
/// use darkmatter::markdown::render_tree::inventory::{disposition_for_tag_end, Disposition};
/// use pulldown_cmark::TagEnd;
///
/// assert_eq!(disposition_for_tag_end(&TagEnd::Paragraph), Disposition::Noise);
/// ```
#[must_use]
pub fn disposition_for_tag_end(tag_end: &TagEnd) -> Disposition {
    match tag_end {
        TagEnd::Paragraph => Disposition::Noise,
        TagEnd::Heading(_) => Disposition::Noise,
        TagEnd::BlockQuote(_) => Disposition::Noise,
        TagEnd::CodeBlock => Disposition::Noise,
        TagEnd::HtmlBlock => Disposition::Noise,
        TagEnd::List(_) => Disposition::Noise,
        TagEnd::Item => Disposition::Noise,
        TagEnd::FootnoteDefinition => Disposition::Noise,
        TagEnd::DefinitionList => Disposition::Noise,
        TagEnd::DefinitionListTitle => Disposition::Noise,
        TagEnd::DefinitionListDefinition => Disposition::Noise,
        TagEnd::Table => Disposition::Noise,
        TagEnd::TableHead => Disposition::Noise,
        TagEnd::TableRow => Disposition::Noise,
        TagEnd::TableCell => Disposition::Noise,
        TagEnd::Emphasis => Disposition::Noise,
        TagEnd::Strong => Disposition::Noise,
        TagEnd::Strikethrough => Disposition::Noise,
        TagEnd::Superscript => Disposition::Noise,
        TagEnd::Subscript => Disposition::Noise,
        TagEnd::Link => Disposition::Noise,
        TagEnd::Image => Disposition::Noise,
        TagEnd::MetadataBlock(_) => Disposition::Noise,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pulldown_cmark::{Options, Parser};
    use std::collections::HashSet;

    /// Compile-time exhaustive-coverage guard for [`Event`].
    ///
    /// The `match` in [`disposition_for_event`] has no wildcard arm, so this
    /// test exists purely so that a `pulldown-cmark` enum change surfaces as a
    /// failed build of the test target as well as the library.
    #[test]
    fn event_coverage_is_exhaustive() {
        let samples = [
            Event::Start(Tag::Paragraph),
            Event::End(TagEnd::Paragraph),
            Event::Text("t".into()),
            Event::Code("c".into()),
            Event::InlineMath("m".into()),
            Event::DisplayMath("m".into()),
            Event::Html("<p>".into()),
            Event::InlineHtml("<i>".into()),
            Event::FootnoteReference("1".into()),
            Event::SoftBreak,
            Event::HardBreak,
            Event::Rule,
            Event::TaskListMarker(true),
        ];
        for event in &samples {
            let _ = disposition_for_event(event);
        }
    }

    /// Compile-time exhaustive-coverage guard for [`Tag`].
    #[test]
    fn tag_coverage_is_exhaustive() {
        let link_like = |t: Tag<'static>| disposition_for_tag(&t);
        let samples: Vec<Tag<'static>> = vec![
            Tag::Paragraph,
            Tag::Heading {
                level: pulldown_cmark::HeadingLevel::H1,
                id: None,
                classes: Vec::new(),
                attrs: Vec::new(),
            },
            Tag::BlockQuote(None),
            Tag::BlockQuote(Some(pulldown_cmark::BlockQuoteKind::Note)),
            Tag::CodeBlock(pulldown_cmark::CodeBlockKind::Indented),
            Tag::HtmlBlock,
            Tag::List(None),
            Tag::List(Some(3)),
            Tag::Item,
            Tag::FootnoteDefinition("1".into()),
            Tag::DefinitionList,
            Tag::DefinitionListTitle,
            Tag::DefinitionListDefinition,
            Tag::Table(Vec::new()),
            Tag::TableHead,
            Tag::TableRow,
            Tag::TableCell,
            Tag::Emphasis,
            Tag::Strong,
            Tag::Strikethrough,
            Tag::Superscript,
            Tag::Subscript,
            Tag::Link {
                link_type: pulldown_cmark::LinkType::Inline,
                dest_url: "u".into(),
                title: "".into(),
                id: "".into(),
            },
            Tag::Image {
                link_type: pulldown_cmark::LinkType::Inline,
                dest_url: "u".into(),
                title: "".into(),
                id: "".into(),
            },
            Tag::MetadataBlock(pulldown_cmark::MetadataBlockKind::YamlStyle),
        ];
        for tag in samples {
            let _ = link_like(tag);
        }
    }

    /// Compile-time exhaustive-coverage guard for [`TagEnd`].
    #[test]
    fn tag_end_coverage_is_exhaustive() {
        let samples = [
            TagEnd::Paragraph,
            TagEnd::Heading(pulldown_cmark::HeadingLevel::H1),
            TagEnd::BlockQuote(None),
            TagEnd::CodeBlock,
            TagEnd::HtmlBlock,
            TagEnd::List(true),
            TagEnd::Item,
            TagEnd::FootnoteDefinition,
            TagEnd::DefinitionList,
            TagEnd::DefinitionListTitle,
            TagEnd::DefinitionListDefinition,
            TagEnd::Table,
            TagEnd::TableHead,
            TagEnd::TableRow,
            TagEnd::TableCell,
            TagEnd::Emphasis,
            TagEnd::Strong,
            TagEnd::Strikethrough,
            TagEnd::Superscript,
            TagEnd::Subscript,
            TagEnd::Link,
            TagEnd::Image,
            TagEnd::MetadataBlock(pulldown_cmark::MetadataBlockKind::YamlStyle),
        ];
        for tag_end in &samples {
            assert_eq!(disposition_for_tag_end(tag_end), Disposition::Noise);
        }
    }

    /// `TagEnd` always closes a pair and therefore is always `Noise`.
    #[test]
    fn tag_end_is_always_noise() {
        assert_eq!(
            disposition_for_tag_end(&TagEnd::Heading(pulldown_cmark::HeadingLevel::H2)),
            Disposition::Noise
        );
    }

    /// `BlockQuote` is `Lossy` because the optional GFM alert kind has no tree
    /// field — this pins correction #2 from the module docs.
    #[test]
    fn block_quote_kind_is_lossy() {
        assert_eq!(
            disposition_for_tag(&Tag::BlockQuote(None)),
            Disposition::Lossy
        );
        assert_eq!(
            disposition_for_tag(&Tag::BlockQuote(Some(
                pulldown_cmark::BlockQuoteKind::Warning
            ))),
            Disposition::Lossy
        );
    }

    /// `Superscript`/`Subscript` exist in 0.13 and fold to a class-carrying
    /// `Span` node — pins correction #4.
    #[test]
    fn super_and_subscript_are_nodes() {
        assert_eq!(disposition_for_tag(&Tag::Superscript), Disposition::Node);
        assert_eq!(disposition_for_tag(&Tag::Subscript), Disposition::Node);
    }

    /// Runtime fixture: a Markdown string exercising several event categories,
    /// parsed with the same `Options` darkmatter uses on its render path
    /// (`ENABLE_TABLES | ENABLE_STRIKETHROUGH`). This proves the inventory's
    /// events are actually produced by the parser.
    #[test]
    fn render_path_options_produce_expected_categories() {
        let markdown = "\
# Heading

A paragraph with *emphasis*, **strong**, ~~struck~~ and `code`.

> a block quote

- item one
- item two

```rust
let x = 1;
```

| A | B |
|---|---|
| 1 | 2 |

[link](https://example.com)

---
";
        let options = Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH;
        let events: Vec<Event<'_>> = Parser::new_ext(markdown, options).collect();

        // Every produced event must have a disposition (exhaustive match runs).
        let dispositions: HashSet<Disposition> = events.iter().map(disposition_for_event).collect();
        assert!(dispositions.contains(&Disposition::Node));
        assert!(dispositions.contains(&Disposition::Noise));

        // Spot-check that the expected categories of variant were emitted.
        let has = |pred: &dyn Fn(&Event<'_>) -> bool| events.iter().any(pred);

        assert!(
            has(&|e| matches!(e, Event::Start(Tag::Heading { .. }))),
            "heading"
        );
        assert!(
            has(&|e| matches!(e, Event::Start(Tag::Paragraph))),
            "paragraph"
        );
        assert!(
            has(&|e| matches!(e, Event::Start(Tag::BlockQuote(_)))),
            "blockquote"
        );
        assert!(has(&|e| matches!(e, Event::Start(Tag::List(_)))), "list");
        assert!(has(&|e| matches!(e, Event::Start(Tag::Item))), "item");
        assert!(
            has(&|e| matches!(e, Event::Start(Tag::CodeBlock(_)))),
            "code block"
        );
        assert!(has(&|e| matches!(e, Event::Start(Tag::Table(_)))), "table");
        assert!(
            has(&|e| matches!(e, Event::Start(Tag::TableHead))),
            "table head"
        );
        assert!(
            has(&|e| matches!(e, Event::Start(Tag::TableRow))),
            "table row"
        );
        assert!(
            has(&|e| matches!(e, Event::Start(Tag::TableCell))),
            "table cell"
        );
        assert!(
            has(&|e| matches!(e, Event::Start(Tag::Emphasis))),
            "emphasis"
        );
        assert!(has(&|e| matches!(e, Event::Start(Tag::Strong))), "strong");
        assert!(
            has(&|e| matches!(e, Event::Start(Tag::Strikethrough))),
            "strikethrough (ENABLE_STRIKETHROUGH)"
        );
        assert!(
            has(&|e| matches!(e, Event::Start(Tag::Link { .. }))),
            "link"
        );
        assert!(has(&|e| matches!(e, Event::Code(_))), "inline code");
        assert!(has(&|e| matches!(e, Event::Text(_))), "text");
        assert!(has(&|e| matches!(e, Event::Rule)), "rule");
        assert!(has(&|e| matches!(e, Event::End(_))), "end");
    }

    /// Without `ENABLE_STRIKETHROUGH`, `~~text~~` does **not** produce a
    /// `Strikethrough` tag — confirms the extension is opt-in and that
    /// darkmatter's render-path options matter.
    #[test]
    fn strikethrough_requires_its_option() {
        let markdown = "~~struck~~";
        let plain: Vec<Event<'_>> = Parser::new(markdown).collect();
        assert!(
            !plain
                .iter()
                .any(|e| matches!(e, Event::Start(Tag::Strikethrough))),
            "strikethrough must be gated behind ENABLE_STRIKETHROUGH"
        );

        let with_ext: Vec<Event<'_>> =
            Parser::new_ext(markdown, Options::ENABLE_STRIKETHROUGH).collect();
        assert!(
            with_ext
                .iter()
                .any(|e| matches!(e, Event::Start(Tag::Strikethrough))),
            "strikethrough must appear with ENABLE_STRIKETHROUGH"
        );
    }
}
