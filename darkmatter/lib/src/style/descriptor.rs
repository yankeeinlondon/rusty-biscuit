//! Static catalog of every leaf path the schema understands.
//!
//! Used by the canonicalization walker (pass 1 of the parser) and the
//! typed-pre-validation pass to detect unknown keys, snake-case aliases, and
//! to dispatch to the right typed validator. Add a row here whenever a field
//! is added to any per-bucket schema struct.
//!
//! Sub-specs covered: #2 (page), #3 (table/images/block-quote), #4 (lists),
//! #5 (color), #6 (hr), #7 (bespoke knobs), #8 (disclosure), #9 (Style
//! Everywhere expanded surface). The themes, code-block mode, line numbers,
//! horizontal padding sides, and rule padding are lowered, so they carry an
//! active sub-spec (#8) although they came later. The page shorthands and the
//! top-level page keys carry the sub-spec of the page field they normalize
//! onto ([`crate::style::normalize`]).
//!
//! ## Canonical-path discipline
//!
//! The walker and validators canonicalize *per segment* by replacing `_` with
//! `-` before looking up against `SCHEMA.canonical`. A leaf segment written
//! with `_` resolves only when the leaf's documented `alias` spells it that
//! way ([`leaf_for_key`]): keys added without a snake-case alias, whose typed
//! fields have no serde alias either, are unknown in snake case rather than
//! silently dropped.

/// Typed leaf categories. Drives the typed-error pre-validation pass in
/// `parse::pre_validate` so that bad inputs surface as
/// `StyleParseError::{InvalidLength, InvalidPercent, InvalidColor, Structure}`
/// rather than the catch-all `Serde` variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeafType {
    /// `renderable::layout::Length` parsed from `"2ch"`, `"50%"`, `"40"`.
    HorizontalLength,
    /// `CommonStyle.width` parsed from explicit width modes or a horizontal
    /// length (`"auto" | "fit-content" | "fit_content" | "40ch"`).
    WidthOrMode,
    /// `u16` row count parsed from a JSON integer. Also the type of a page
    /// shorthand that sets a vertical side (`margin`, `padding`, `my`, `py`,
    /// `mt`, ...): an all-sides value must be valid for the vertical sides
    /// too, and every row count is also a valid horizontal cell count.
    RowCount,
    /// A horizontal length that must be positive: the page content cap
    /// (`max-width`). Zero in any spelling (`0`, `"0"`, `"0ch"`, `"0%"`) is
    /// rejected.
    PositivePageWidth,
    /// `renderable::layout::Alignment` parsed from `"left" | "center" |
    /// "centered" | "right"`. Validated by serde — no pre-validator emitted.
    Alignment,
    /// `StyleColor` parsed from a Tailwind name, hex, or web-color name.
    Color,
    /// `PageBackground` enum (`"transparent" | "subtle" | "pronounced"`).
    /// Validated by serde — no pre-validator emitted.
    BackgroundEnum,
    /// Free-form string (e.g. `stylesheet`).
    StringValue,
    /// A theme pair name accepted by `ThemePair::try_from` (`page.theme`,
    /// `page.code.theme`), including its case and alias spellings.
    ThemeName,
    /// A code-block mode: `"inverse" | "dark" | "light" | "same"`.
    CodeBlockMode,
    /// A JSON boolean (e.g. `code-block.line-numbers`).
    Boolean,
    /// Open object of opaque values (e.g. `page.meta`): any JSON object.
    OpaqueValue,
    /// `HrKind` enum (`"dashes" | "dots" | "waves" | "line-star" |
    /// "line-circle" | "inset-line" | "curtain-rod"`).
    HrKind,
    /// `HrWeight` enum (`"thin" | "medium" | "thick"`).
    HrWeight,
    /// `HrAlignment` enum (`"full" | "left" | "center" | "right"`).
    HrAlignment,
    /// A compound style value (`border`, `emphasis`). Validated by serde
    /// during typed deserialization.
    CompoundStyle,
    /// A `word-wrap` keyword (`"none" | "wrap" | "truncate" | "wrap-prose"`).
    /// Validated by serde during typed deserialization.
    WordWrap,
    /// A `style.markdown` cleanup rule. Validated by
    /// [`crate::markdown::cleanup::rules`], the rules' single validator;
    /// rendering never reads it.
    MarkdownRule,
}

/// A single schema leaf: its canonical kebab-case path, any documented
/// snake-case alias, the sub-spec that wires it, and its typed category.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SchemaLeaf {
    /// Dotted canonical path under `style.` (e.g. `"page.margin-left"`).
    pub canonical: &'static str,
    /// Snake-case alias path, if any (e.g. `"page.max_width"`). Its last
    /// segment is the only snake-case spelling [`leaf_for_key`] accepts for
    /// the leaf.
    pub alias: Option<&'static str>,
    /// Sub-spec number that wires this key to rendering. `0` marks a
    /// `markdown.*` cleanup rule, which rendering accepts but never reads.
    /// A number above [`crate::style::parse::ACTIVE_STYLE_WIRING_SUB_SPEC`]
    /// marks a key that is parsed and validated but not lowered.
    pub sub_spec: u8,
    /// Typed category of this leaf — drives typed-error pre-validation.
    pub leaf_type: LeafType,
}

/// The complete schema catalog. Every leaf reachable through any per-bucket
/// struct must appear here.
#[rustfmt::skip]
pub const SCHEMA: &[SchemaLeaf] = &[
    // ── page ────────────────────────────────────────────────────────────
    SchemaLeaf { canonical: "page.margin-left", alias: None, sub_spec: 2, leaf_type: LeafType::HorizontalLength },
    SchemaLeaf { canonical: "page.margin-right", alias: None, sub_spec: 2, leaf_type: LeafType::HorizontalLength },
    SchemaLeaf { canonical: "page.margin-top", alias: None, sub_spec: 2, leaf_type: LeafType::RowCount },
    SchemaLeaf { canonical: "page.margin-bottom", alias: None, sub_spec: 2, leaf_type: LeafType::RowCount },
    SchemaLeaf { canonical: "page.padding-left", alias: None, sub_spec: 2, leaf_type: LeafType::HorizontalLength },
    SchemaLeaf { canonical: "page.padding-right", alias: None, sub_spec: 2, leaf_type: LeafType::HorizontalLength },
    SchemaLeaf { canonical: "page.padding-top", alias: None, sub_spec: 2, leaf_type: LeafType::RowCount },
    SchemaLeaf { canonical: "page.padding-bottom", alias: None, sub_spec: 2, leaf_type: LeafType::RowCount },
    SchemaLeaf { canonical: "page.max-width", alias: Some("page.max_width"), sub_spec: 2, leaf_type: LeafType::PositivePageWidth },
    SchemaLeaf { canonical: "page.alignment", alias: None, sub_spec: 2, leaf_type: LeafType::Alignment },
    SchemaLeaf { canonical: "page.color", alias: None, sub_spec: 5, leaf_type: LeafType::Color },
    SchemaLeaf { canonical: "page.bg-color", alias: Some("page.bg_color"), sub_spec: 5, leaf_type: LeafType::Color },
    SchemaLeaf { canonical: "page.background", alias: None, sub_spec: 2, leaf_type: LeafType::BackgroundEnum },
    SchemaLeaf { canonical: "page.stylesheet", alias: None, sub_spec: 7, leaf_type: LeafType::StringValue },
    SchemaLeaf { canonical: "page.meta", alias: None, sub_spec: 7, leaf_type: LeafType::OpaqueValue },
    SchemaLeaf { canonical: "page.code.theme", alias: None, sub_spec: 7, leaf_type: LeafType::ThemeName },
    SchemaLeaf { canonical: "page.code.mode", alias: None, sub_spec: 8, leaf_type: LeafType::CodeBlockMode },
    SchemaLeaf { canonical: "page.theme", alias: None, sub_spec: 8, leaf_type: LeafType::ThemeName },
    SchemaLeaf { canonical: "page.margin", alias: None, sub_spec: 2, leaf_type: LeafType::RowCount },
    SchemaLeaf { canonical: "page.padding", alias: None, sub_spec: 2, leaf_type: LeafType::RowCount },
    SchemaLeaf { canonical: "page.mx", alias: None, sub_spec: 2, leaf_type: LeafType::HorizontalLength },
    SchemaLeaf { canonical: "page.my", alias: None, sub_spec: 2, leaf_type: LeafType::RowCount },
    SchemaLeaf { canonical: "page.px", alias: None, sub_spec: 2, leaf_type: LeafType::HorizontalLength },
    SchemaLeaf { canonical: "page.py", alias: None, sub_spec: 2, leaf_type: LeafType::RowCount },
    SchemaLeaf { canonical: "page.ml", alias: None, sub_spec: 2, leaf_type: LeafType::HorizontalLength },
    SchemaLeaf { canonical: "page.mr", alias: None, sub_spec: 2, leaf_type: LeafType::HorizontalLength },
    SchemaLeaf { canonical: "page.mt", alias: None, sub_spec: 2, leaf_type: LeafType::RowCount },
    SchemaLeaf { canonical: "page.mb", alias: None, sub_spec: 2, leaf_type: LeafType::RowCount },
    SchemaLeaf { canonical: "page.pl", alias: None, sub_spec: 2, leaf_type: LeafType::HorizontalLength },
    SchemaLeaf { canonical: "page.pr", alias: None, sub_spec: 2, leaf_type: LeafType::HorizontalLength },
    SchemaLeaf { canonical: "page.pt", alias: None, sub_spec: 2, leaf_type: LeafType::RowCount },
    SchemaLeaf { canonical: "page.pb", alias: None, sub_spec: 2, leaf_type: LeafType::RowCount },

    // ── top level: every page key and shorthand, applying to `page` ────
    SchemaLeaf { canonical: "margin-left", alias: None, sub_spec: 2, leaf_type: LeafType::HorizontalLength },
    SchemaLeaf { canonical: "margin-right", alias: None, sub_spec: 2, leaf_type: LeafType::HorizontalLength },
    SchemaLeaf { canonical: "margin-top", alias: None, sub_spec: 2, leaf_type: LeafType::RowCount },
    SchemaLeaf { canonical: "margin-bottom", alias: None, sub_spec: 2, leaf_type: LeafType::RowCount },
    SchemaLeaf { canonical: "padding-left", alias: None, sub_spec: 2, leaf_type: LeafType::HorizontalLength },
    SchemaLeaf { canonical: "padding-right", alias: None, sub_spec: 2, leaf_type: LeafType::HorizontalLength },
    SchemaLeaf { canonical: "padding-top", alias: None, sub_spec: 2, leaf_type: LeafType::RowCount },
    SchemaLeaf { canonical: "padding-bottom", alias: None, sub_spec: 2, leaf_type: LeafType::RowCount },
    SchemaLeaf { canonical: "max-width", alias: None, sub_spec: 2, leaf_type: LeafType::PositivePageWidth },
    SchemaLeaf { canonical: "alignment", alias: None, sub_spec: 2, leaf_type: LeafType::Alignment },
    SchemaLeaf { canonical: "color", alias: None, sub_spec: 5, leaf_type: LeafType::Color },
    SchemaLeaf { canonical: "bg-color", alias: None, sub_spec: 5, leaf_type: LeafType::Color },
    SchemaLeaf { canonical: "background", alias: None, sub_spec: 2, leaf_type: LeafType::BackgroundEnum },
    SchemaLeaf { canonical: "stylesheet", alias: None, sub_spec: 7, leaf_type: LeafType::StringValue },
    SchemaLeaf { canonical: "meta", alias: None, sub_spec: 7, leaf_type: LeafType::OpaqueValue },
    SchemaLeaf { canonical: "code.theme", alias: None, sub_spec: 7, leaf_type: LeafType::ThemeName },
    SchemaLeaf { canonical: "code.mode", alias: None, sub_spec: 8, leaf_type: LeafType::CodeBlockMode },
    SchemaLeaf { canonical: "theme", alias: None, sub_spec: 8, leaf_type: LeafType::ThemeName },
    SchemaLeaf { canonical: "margin", alias: None, sub_spec: 2, leaf_type: LeafType::RowCount },
    SchemaLeaf { canonical: "padding", alias: None, sub_spec: 2, leaf_type: LeafType::RowCount },
    SchemaLeaf { canonical: "mx", alias: None, sub_spec: 2, leaf_type: LeafType::HorizontalLength },
    SchemaLeaf { canonical: "my", alias: None, sub_spec: 2, leaf_type: LeafType::RowCount },
    SchemaLeaf { canonical: "px", alias: None, sub_spec: 2, leaf_type: LeafType::HorizontalLength },
    SchemaLeaf { canonical: "py", alias: None, sub_spec: 2, leaf_type: LeafType::RowCount },
    SchemaLeaf { canonical: "ml", alias: None, sub_spec: 2, leaf_type: LeafType::HorizontalLength },
    SchemaLeaf { canonical: "mr", alias: None, sub_spec: 2, leaf_type: LeafType::HorizontalLength },
    SchemaLeaf { canonical: "mt", alias: None, sub_spec: 2, leaf_type: LeafType::RowCount },
    SchemaLeaf { canonical: "mb", alias: None, sub_spec: 2, leaf_type: LeafType::RowCount },
    SchemaLeaf { canonical: "pl", alias: None, sub_spec: 2, leaf_type: LeafType::HorizontalLength },
    SchemaLeaf { canonical: "pr", alias: None, sub_spec: 2, leaf_type: LeafType::HorizontalLength },
    SchemaLeaf { canonical: "pt", alias: None, sub_spec: 2, leaf_type: LeafType::RowCount },
    SchemaLeaf { canonical: "pb", alias: None, sub_spec: 2, leaf_type: LeafType::RowCount },

    // ── table ───────────────────────────────────────────────────────────
    SchemaLeaf { canonical: "table.width",       alias: None,                    sub_spec: 3, leaf_type: LeafType::WidthOrMode },
    SchemaLeaf { canonical: "table.max-width",   alias: Some("table.max_width"), sub_spec: 3, leaf_type: LeafType::HorizontalLength },
    SchemaLeaf { canonical: "table.alignment",   alias: None,                    sub_spec: 3, leaf_type: LeafType::Alignment },
    SchemaLeaf { canonical: "table.color",       alias: None,                    sub_spec: 5, leaf_type: LeafType::Color },
    SchemaLeaf { canonical: "table.bg-color",    alias: Some("table.bg_color"),  sub_spec: 5, leaf_type: LeafType::Color },
    SchemaLeaf { canonical: "table.margin",      alias: None,                    sub_spec: 9, leaf_type: LeafType::HorizontalLength },
    SchemaLeaf { canonical: "table.padding",     alias: None,                    sub_spec: 9, leaf_type: LeafType::HorizontalLength },
    SchemaLeaf { canonical: "table.padding-left", alias: None, sub_spec: 8, leaf_type: LeafType::HorizontalLength },
    SchemaLeaf { canonical: "table.padding-right", alias: None, sub_spec: 8, leaf_type: LeafType::HorizontalLength },
    SchemaLeaf { canonical: "table.border",      alias: None,                    sub_spec: 9, leaf_type: LeafType::CompoundStyle },
    SchemaLeaf { canonical: "table.emphasis",    alias: None,                    sub_spec: 9, leaf_type: LeafType::CompoundStyle },
    SchemaLeaf { canonical: "table.word-wrap",   alias: Some("table.word_wrap"), sub_spec: 9, leaf_type: LeafType::WordWrap },

    // ── block-quote ─────────────────────────────────────────────────────
    SchemaLeaf { canonical: "block-quote.width",     alias: Some("block_quote.width"),     sub_spec: 3, leaf_type: LeafType::WidthOrMode },
    SchemaLeaf { canonical: "block-quote.max-width", alias: Some("block_quote.max_width"), sub_spec: 3, leaf_type: LeafType::HorizontalLength },
    SchemaLeaf { canonical: "block-quote.alignment", alias: Some("block_quote.alignment"), sub_spec: 3, leaf_type: LeafType::Alignment },
    SchemaLeaf { canonical: "block-quote.color",     alias: Some("block_quote.color"),     sub_spec: 5, leaf_type: LeafType::Color },
    SchemaLeaf { canonical: "block-quote.bg-color",  alias: Some("block_quote.bg_color"),  sub_spec: 5, leaf_type: LeafType::Color },
    SchemaLeaf { canonical: "block-quote.margin",    alias: Some("block_quote.margin"),    sub_spec: 9, leaf_type: LeafType::HorizontalLength },
    SchemaLeaf { canonical: "block-quote.padding",   alias: Some("block_quote.padding"),   sub_spec: 9, leaf_type: LeafType::HorizontalLength },
    SchemaLeaf { canonical: "block-quote.padding-left", alias: Some("block_quote.padding-left"), sub_spec: 8, leaf_type: LeafType::HorizontalLength },
    SchemaLeaf { canonical: "block-quote.padding-right", alias: Some("block_quote.padding-right"), sub_spec: 8, leaf_type: LeafType::HorizontalLength },
    SchemaLeaf { canonical: "block-quote.border",    alias: Some("block_quote.border"),    sub_spec: 9, leaf_type: LeafType::CompoundStyle },
    SchemaLeaf { canonical: "block-quote.emphasis",  alias: Some("block_quote.emphasis"),  sub_spec: 9, leaf_type: LeafType::CompoundStyle },
    SchemaLeaf { canonical: "block-quote.word-wrap", alias: Some("block_quote.word_wrap"), sub_spec: 9, leaf_type: LeafType::WordWrap },

    // ── ul ──────────────────────────────────────────────────────────────
    SchemaLeaf { canonical: "ul.width",       alias: None,                   sub_spec: 4, leaf_type: LeafType::WidthOrMode },
    SchemaLeaf { canonical: "ul.max-width",   alias: Some("ul.max_width"),   sub_spec: 4, leaf_type: LeafType::HorizontalLength },
    SchemaLeaf { canonical: "ul.alignment",   alias: None,                   sub_spec: 4, leaf_type: LeafType::Alignment },
    SchemaLeaf { canonical: "ul.color",       alias: None,                   sub_spec: 5, leaf_type: LeafType::Color },
    SchemaLeaf { canonical: "ul.bg-color",    alias: Some("ul.bg_color"),    sub_spec: 5, leaf_type: LeafType::Color },
    SchemaLeaf { canonical: "ul.left-margin", alias: Some("ul.left_margin"), sub_spec: 4, leaf_type: LeafType::HorizontalLength },
    SchemaLeaf { canonical: "ul.margin",      alias: None,                   sub_spec: 9, leaf_type: LeafType::HorizontalLength },
    SchemaLeaf { canonical: "ul.padding",     alias: None,                   sub_spec: 9, leaf_type: LeafType::HorizontalLength },
    SchemaLeaf { canonical: "ul.padding-left", alias: None, sub_spec: 8, leaf_type: LeafType::HorizontalLength },
    SchemaLeaf { canonical: "ul.padding-right", alias: None, sub_spec: 8, leaf_type: LeafType::HorizontalLength },
    SchemaLeaf { canonical: "ul.border",      alias: None,                   sub_spec: 9, leaf_type: LeafType::CompoundStyle },
    SchemaLeaf { canonical: "ul.emphasis",    alias: None,                   sub_spec: 9, leaf_type: LeafType::CompoundStyle },
    SchemaLeaf { canonical: "ul.word-wrap",   alias: Some("ul.word_wrap"),   sub_spec: 9, leaf_type: LeafType::WordWrap },

    // ── ol ──────────────────────────────────────────────────────────────
    SchemaLeaf { canonical: "ol.width",     alias: None,                  sub_spec: 4, leaf_type: LeafType::WidthOrMode },
    SchemaLeaf { canonical: "ol.max-width", alias: Some("ol.max_width"),  sub_spec: 4, leaf_type: LeafType::HorizontalLength },
    SchemaLeaf { canonical: "ol.alignment", alias: None,                  sub_spec: 4, leaf_type: LeafType::Alignment },
    SchemaLeaf { canonical: "ol.color",     alias: None,                  sub_spec: 5, leaf_type: LeafType::Color },
    SchemaLeaf { canonical: "ol.bg-color",  alias: Some("ol.bg_color"),   sub_spec: 5, leaf_type: LeafType::Color },
    SchemaLeaf { canonical: "ol.margin",    alias: None,                  sub_spec: 9, leaf_type: LeafType::HorizontalLength },
    SchemaLeaf { canonical: "ol.padding",   alias: None,                  sub_spec: 9, leaf_type: LeafType::HorizontalLength },
    SchemaLeaf { canonical: "ol.padding-left", alias: None, sub_spec: 8, leaf_type: LeafType::HorizontalLength },
    SchemaLeaf { canonical: "ol.padding-right", alias: None, sub_spec: 8, leaf_type: LeafType::HorizontalLength },
    SchemaLeaf { canonical: "ol.border",    alias: None,                  sub_spec: 9, leaf_type: LeafType::CompoundStyle },
    SchemaLeaf { canonical: "ol.emphasis",  alias: None,                  sub_spec: 9, leaf_type: LeafType::CompoundStyle },
    SchemaLeaf { canonical: "ol.word-wrap", alias: Some("ol.word_wrap"),  sub_spec: 9, leaf_type: LeafType::WordWrap },

    // ── li ──────────────────────────────────────────────────────────────
    SchemaLeaf { canonical: "li.width",     alias: None,                  sub_spec: 4, leaf_type: LeafType::WidthOrMode },
    SchemaLeaf { canonical: "li.max-width", alias: Some("li.max_width"),  sub_spec: 4, leaf_type: LeafType::HorizontalLength },
    SchemaLeaf { canonical: "li.alignment", alias: None,                  sub_spec: 4, leaf_type: LeafType::Alignment },
    SchemaLeaf { canonical: "li.color",     alias: None,                  sub_spec: 5, leaf_type: LeafType::Color },
    SchemaLeaf { canonical: "li.bg-color",  alias: Some("li.bg_color"),   sub_spec: 5, leaf_type: LeafType::Color },
    SchemaLeaf { canonical: "li.margin",    alias: None,                  sub_spec: 9, leaf_type: LeafType::HorizontalLength },
    SchemaLeaf { canonical: "li.padding",   alias: None,                  sub_spec: 9, leaf_type: LeafType::HorizontalLength },
    SchemaLeaf { canonical: "li.padding-left", alias: None, sub_spec: 8, leaf_type: LeafType::HorizontalLength },
    SchemaLeaf { canonical: "li.padding-right", alias: None, sub_spec: 8, leaf_type: LeafType::HorizontalLength },
    SchemaLeaf { canonical: "li.border",    alias: None,                  sub_spec: 9, leaf_type: LeafType::CompoundStyle },
    SchemaLeaf { canonical: "li.emphasis",  alias: None,                  sub_spec: 9, leaf_type: LeafType::CompoundStyle },
    SchemaLeaf { canonical: "li.word-wrap", alias: Some("li.word_wrap"),  sub_spec: 9, leaf_type: LeafType::WordWrap },

    // ── hyperlinks ──────────────────────────────────────────────────────
    SchemaLeaf { canonical: "hyperlinks.width",                 alias: None,                                            sub_spec: 7, leaf_type: LeafType::WidthOrMode },
    SchemaLeaf { canonical: "hyperlinks.max-width",             alias: Some("hyperlinks.max_width"),                    sub_spec: 7, leaf_type: LeafType::HorizontalLength },
    SchemaLeaf { canonical: "hyperlinks.alignment",             alias: None,                                            sub_spec: 7, leaf_type: LeafType::Alignment },
    SchemaLeaf { canonical: "hyperlinks.color",                 alias: None,                                            sub_spec: 5, leaf_type: LeafType::Color },
    SchemaLeaf { canonical: "hyperlinks.bg-color",              alias: Some("hyperlinks.bg_color"),                     sub_spec: 5, leaf_type: LeafType::Color },
    SchemaLeaf { canonical: "hyperlinks.local-style.width",     alias: Some("hyperlinks.local_style.width"),            sub_spec: 7, leaf_type: LeafType::WidthOrMode },
    SchemaLeaf { canonical: "hyperlinks.local-style.max-width", alias: Some("hyperlinks.local_style.max_width"),        sub_spec: 7, leaf_type: LeafType::HorizontalLength },
    SchemaLeaf { canonical: "hyperlinks.local-style.alignment", alias: Some("hyperlinks.local_style.alignment"),        sub_spec: 7, leaf_type: LeafType::Alignment },
    SchemaLeaf { canonical: "hyperlinks.local-style.color",     alias: Some("hyperlinks.local_style.color"),            sub_spec: 7, leaf_type: LeafType::Color },
    SchemaLeaf { canonical: "hyperlinks.local-style.bg-color",  alias: Some("hyperlinks.local_style.bg_color"),         sub_spec: 7, leaf_type: LeafType::Color },
    SchemaLeaf { canonical: "hyperlinks.local-style.margin",    alias: Some("hyperlinks.local_style.margin"),           sub_spec: 9, leaf_type: LeafType::HorizontalLength },
    SchemaLeaf { canonical: "hyperlinks.local-style.padding",   alias: Some("hyperlinks.local_style.padding"),          sub_spec: 9, leaf_type: LeafType::HorizontalLength },
    SchemaLeaf { canonical: "hyperlinks.local-style.padding-left", alias: Some("hyperlinks.local_style.padding-left"), sub_spec: 8, leaf_type: LeafType::HorizontalLength },
    SchemaLeaf { canonical: "hyperlinks.local-style.padding-right", alias: Some("hyperlinks.local_style.padding-right"), sub_spec: 8, leaf_type: LeafType::HorizontalLength },
    SchemaLeaf { canonical: "hyperlinks.local-style.border",    alias: Some("hyperlinks.local_style.border"),           sub_spec: 9, leaf_type: LeafType::CompoundStyle },
    SchemaLeaf { canonical: "hyperlinks.local-style.emphasis",  alias: Some("hyperlinks.local_style.emphasis"),         sub_spec: 9, leaf_type: LeafType::CompoundStyle },
    SchemaLeaf { canonical: "hyperlinks.local-style.word-wrap", alias: Some("hyperlinks.local_style.word_wrap"),        sub_spec: 9, leaf_type: LeafType::WordWrap },

    // ── images ──────────────────────────────────────────────────────────
    SchemaLeaf { canonical: "images.width",                 alias: None,                                       sub_spec: 3, leaf_type: LeafType::WidthOrMode },
    SchemaLeaf { canonical: "images.max-width",             alias: Some("images.max_width"),                   sub_spec: 3, leaf_type: LeafType::HorizontalLength },
    SchemaLeaf { canonical: "images.alignment",             alias: None,                                       sub_spec: 3, leaf_type: LeafType::Alignment },
    SchemaLeaf { canonical: "images.color",                 alias: None,                                       sub_spec: 5, leaf_type: LeafType::Color },
    SchemaLeaf { canonical: "images.bg-color",              alias: Some("images.bg_color"),                    sub_spec: 5, leaf_type: LeafType::Color },
    SchemaLeaf { canonical: "images.padding",   alias: None, sub_spec: 8, leaf_type: LeafType::HorizontalLength },
    SchemaLeaf { canonical: "images.padding-left", alias: None, sub_spec: 8, leaf_type: LeafType::HorizontalLength },
    SchemaLeaf { canonical: "images.padding-right", alias: None, sub_spec: 8, leaf_type: LeafType::HorizontalLength },
    SchemaLeaf { canonical: "images.local-style.width",     alias: Some("images.local_style.width"),           sub_spec: 7, leaf_type: LeafType::WidthOrMode },
    SchemaLeaf { canonical: "images.local-style.max-width", alias: Some("images.local_style.max_width"),       sub_spec: 7, leaf_type: LeafType::HorizontalLength },
    SchemaLeaf { canonical: "images.local-style.alignment", alias: Some("images.local_style.alignment"),       sub_spec: 7, leaf_type: LeafType::Alignment },
    SchemaLeaf { canonical: "images.local-style.color",     alias: Some("images.local_style.color"),           sub_spec: 7, leaf_type: LeafType::Color },
    SchemaLeaf { canonical: "images.local-style.bg-color",  alias: Some("images.local_style.bg_color"),        sub_spec: 7, leaf_type: LeafType::Color },
    SchemaLeaf { canonical: "images.local-style.margin",    alias: Some("images.local_style.margin"),          sub_spec: 9, leaf_type: LeafType::HorizontalLength },
    SchemaLeaf { canonical: "images.local-style.padding",   alias: Some("images.local_style.padding"),         sub_spec: 9, leaf_type: LeafType::HorizontalLength },
    SchemaLeaf { canonical: "images.local-style.padding-left", alias: Some("images.local_style.padding-left"), sub_spec: 8, leaf_type: LeafType::HorizontalLength },
    SchemaLeaf { canonical: "images.local-style.padding-right", alias: Some("images.local_style.padding-right"), sub_spec: 8, leaf_type: LeafType::HorizontalLength },
    SchemaLeaf { canonical: "images.local-style.border",    alias: Some("images.local_style.border"),          sub_spec: 9, leaf_type: LeafType::CompoundStyle },
    SchemaLeaf { canonical: "images.local-style.emphasis",  alias: Some("images.local_style.emphasis"),        sub_spec: 9, leaf_type: LeafType::CompoundStyle },
    SchemaLeaf { canonical: "images.local-style.word-wrap", alias: Some("images.local_style.word_wrap"),       sub_spec: 9, leaf_type: LeafType::WordWrap },

    // ── hr ──────────────────────────────────────────────────────────────
    SchemaLeaf { canonical: "hr.width",     alias: None,                 sub_spec: 6, leaf_type: LeafType::HorizontalLength },
    SchemaLeaf { canonical: "hr.max-width", alias: Some("hr.max_width"), sub_spec: 6, leaf_type: LeafType::HorizontalLength },
    SchemaLeaf { canonical: "hr.alignment", alias: None,                 sub_spec: 6, leaf_type: LeafType::HrAlignment },
    SchemaLeaf { canonical: "hr.color",     alias: None,                 sub_spec: 6, leaf_type: LeafType::Color },
    SchemaLeaf { canonical: "hr.bg-color",  alias: Some("hr.bg_color"),  sub_spec: 6, leaf_type: LeafType::Color },
    SchemaLeaf { canonical: "hr.kind",      alias: None,                 sub_spec: 6, leaf_type: LeafType::HrKind },
    SchemaLeaf { canonical: "hr.weight",    alias: None,                 sub_spec: 6, leaf_type: LeafType::HrWeight },
    SchemaLeaf { canonical: "hr.padding",   alias: None,                 sub_spec: 8, leaf_type: LeafType::HorizontalLength },
    SchemaLeaf { canonical: "hr.padding-left", alias: None, sub_spec: 8, leaf_type: LeafType::HorizontalLength },
    SchemaLeaf { canonical: "hr.padding-right", alias: None, sub_spec: 8, leaf_type: LeafType::HorizontalLength },

    // ── disclosure ──────────────────────────────────────────────────────
    SchemaLeaf { canonical: "disclosure.width",     alias: None,                          sub_spec: 8, leaf_type: LeafType::WidthOrMode },
    SchemaLeaf { canonical: "disclosure.max-width", alias: Some("disclosure.max_width"),  sub_spec: 8, leaf_type: LeafType::HorizontalLength },
    SchemaLeaf { canonical: "disclosure.alignment", alias: None,                          sub_spec: 8, leaf_type: LeafType::Alignment },
    SchemaLeaf { canonical: "disclosure.color",     alias: None,                          sub_spec: 8, leaf_type: LeafType::Color },
    SchemaLeaf { canonical: "disclosure.bg-color",  alias: Some("disclosure.bg_color"),   sub_spec: 8, leaf_type: LeafType::Color },
    SchemaLeaf { canonical: "disclosure.margin",    alias: None,                          sub_spec: 9, leaf_type: LeafType::HorizontalLength },
    SchemaLeaf { canonical: "disclosure.padding",   alias: None,                          sub_spec: 9, leaf_type: LeafType::HorizontalLength },
    SchemaLeaf { canonical: "disclosure.padding-left", alias: None, sub_spec: 8, leaf_type: LeafType::HorizontalLength },
    SchemaLeaf { canonical: "disclosure.padding-right", alias: None, sub_spec: 8, leaf_type: LeafType::HorizontalLength },
    SchemaLeaf { canonical: "disclosure.border",    alias: None,                          sub_spec: 9, leaf_type: LeafType::CompoundStyle },
    SchemaLeaf { canonical: "disclosure.emphasis",  alias: None,                          sub_spec: 9, leaf_type: LeafType::CompoundStyle },
    SchemaLeaf { canonical: "disclosure.word-wrap", alias: Some("disclosure.word_wrap"),  sub_spec: 9, leaf_type: LeafType::WordWrap },

    // ── code-block ──────────────────────────────────────────────────────
    SchemaLeaf { canonical: "code-block.width",     alias: Some("code_block.width"),     sub_spec: 9, leaf_type: LeafType::WidthOrMode },
    SchemaLeaf { canonical: "code-block.max-width", alias: Some("code_block.max_width"), sub_spec: 9, leaf_type: LeafType::HorizontalLength },
    SchemaLeaf { canonical: "code-block.alignment", alias: Some("code_block.alignment"), sub_spec: 9, leaf_type: LeafType::Alignment },
    SchemaLeaf { canonical: "code-block.color",     alias: Some("code_block.color"),     sub_spec: 9, leaf_type: LeafType::Color },
    SchemaLeaf { canonical: "code-block.bg-color",  alias: Some("code_block.bg_color"),  sub_spec: 9, leaf_type: LeafType::Color },
    SchemaLeaf { canonical: "code-block.margin",    alias: Some("code_block.margin"),    sub_spec: 9, leaf_type: LeafType::HorizontalLength },
    SchemaLeaf { canonical: "code-block.padding",   alias: Some("code_block.padding"),   sub_spec: 9, leaf_type: LeafType::HorizontalLength },
    SchemaLeaf { canonical: "code-block.padding-left", alias: Some("code_block.padding-left"), sub_spec: 8, leaf_type: LeafType::HorizontalLength },
    SchemaLeaf { canonical: "code-block.padding-right", alias: Some("code_block.padding-right"), sub_spec: 8, leaf_type: LeafType::HorizontalLength },
    SchemaLeaf { canonical: "code-block.border",    alias: Some("code_block.border"),    sub_spec: 9, leaf_type: LeafType::CompoundStyle },
    SchemaLeaf { canonical: "code-block.emphasis",  alias: Some("code_block.emphasis"),  sub_spec: 9, leaf_type: LeafType::CompoundStyle },
    SchemaLeaf { canonical: "code-block.word-wrap", alias: Some("code_block.word_wrap"), sub_spec: 9, leaf_type: LeafType::WordWrap },
    SchemaLeaf { canonical: "code-block.line-numbers", alias: Some("code_block.line-numbers"), sub_spec: 8, leaf_type: LeafType::Boolean },

    // ── markdown (cleanup rules, not render styles; sub_spec 0 = never inactive) ──
    SchemaLeaf { canonical: "markdown.frontmatter",           alias: None, sub_spec: 0, leaf_type: LeafType::MarkdownRule },
    SchemaLeaf { canonical: "markdown.table-layout",          alias: None, sub_spec: 0, leaf_type: LeafType::MarkdownRule },
    SchemaLeaf { canonical: "markdown.fixed-width",           alias: None, sub_spec: 0, leaf_type: LeafType::MarkdownRule },
    SchemaLeaf { canonical: "markdown.unwrap",                alias: None, sub_spec: 0, leaf_type: LeafType::MarkdownRule },
    SchemaLeaf { canonical: "markdown.indentation",           alias: None, sub_spec: 0, leaf_type: LeafType::MarkdownRule },
    SchemaLeaf { canonical: "markdown.ordered-lists",         alias: None, sub_spec: 0, leaf_type: LeafType::MarkdownRule },
    SchemaLeaf { canonical: "markdown.list-padding",          alias: None, sub_spec: 0, leaf_type: LeafType::MarkdownRule },
    SchemaLeaf { canonical: "markdown.list-item-padding",     alias: None, sub_spec: 0, leaf_type: LeafType::MarkdownRule },
    SchemaLeaf { canonical: "markdown.list-interior-padding", alias: None, sub_spec: 0, leaf_type: LeafType::MarkdownRule },
    SchemaLeaf { canonical: "markdown.default-language",      alias: None, sub_spec: 0, leaf_type: LeafType::MarkdownRule },
];

/// Return the canonical schema leaf for `raw_path` if it matches either a
/// canonical entry or an alias. Returns `None` for unknown paths.
///
/// Prefer [`leaf_for_canonical`] in walker/validator code where each segment
/// is already canonicalized; this accepts-either helper is for tests and
/// direct alias-lookup callers.
pub fn canonicalize(raw_path: &str) -> Option<&'static SchemaLeaf> {
    SCHEMA
        .iter()
        .find(|leaf| leaf.canonical == raw_path || leaf.alias == Some(raw_path))
}

/// Find a leaf whose canonical path exactly matches `path`. Aliases are
/// **not** considered — call after segment-wise [`kebabify`].
pub fn leaf_for_canonical(path: &str) -> Option<&'static SchemaLeaf> {
    SCHEMA.iter().find(|leaf| leaf.canonical == path)
}

/// The leaf at `canonical_path` (already kebabified segment-wise) when its
/// last segment was written as `raw_key`.
///
/// `raw_key` must be the leaf's canonical segment or the last segment of its
/// documented alias; any other snake-case spelling returns `None`, because
/// serde has no alias for it and would drop the value.
pub fn leaf_for_key(canonical_path: &str, raw_key: &str) -> Option<&'static SchemaLeaf> {
    let leaf = leaf_for_canonical(canonical_path)?;
    let accepted = last_segment(leaf.canonical) == raw_key
        || leaf.alias.is_some_and(|alias| last_segment(alias) == raw_key);
    accepted.then_some(leaf)
}

fn last_segment(path: &str) -> &str {
    path.rsplit('.').next().unwrap_or(path)
}

/// Page spacing keys removed in favor of CSS order, as `(old, new)` segment
/// pairs. The old spellings are unknown keys; the walker suggests the new
/// one.
const RENAMED_PAGE_KEYS: &[(&str, &str)] = &[
    ("left-margin", "margin-left"),
    ("right-margin", "margin-right"),
    ("top-margin", "margin-top"),
    ("bottom-margin", "margin-bottom"),
    ("left-padding", "padding-left"),
    ("right-padding", "padding-right"),
    ("top-padding", "padding-top"),
    ("bottom-padding", "padding-bottom"),
];

/// The canonical key to use instead of the unknown `canonical_path`
/// (already kebabified), when the path is a known misspelling: a removed
/// page spacing name under `page` or at the top level (`page.left-margin`
/// suggests `page.margin-left`), or a snake-case spelling of a leaf that has
/// no snake-case alias (`page.margin_left` suggests `page.margin-left`).
pub fn suggested_key(canonical_path: &str) -> Option<String> {
    if let Some(leaf) = leaf_for_canonical(canonical_path) {
        return Some(leaf.canonical.to_string());
    }
    let (parent, segment) = match canonical_path.rsplit_once('.') {
        Some((parent, segment)) => (parent, segment),
        None => ("", canonical_path),
    };
    if !matches!(parent, "" | "page") {
        return None;
    }
    RENAMED_PAGE_KEYS
        .iter()
        .find(|(old, _)| *old == segment)
        .map(|(_, new)| join_path(parent, new))
}

/// `true` if `path` is a canonical container — a node that has children but
/// is not itself a leaf. Containers include the top-level buckets plus the
/// nested `local-style` and `page.code` paths, and the top-level `code`
/// (the page's code settings written at the top level).
pub fn is_canonical_container(path: &str) -> bool {
    matches!(
        path,
        "page"
            | "table"
            | "block-quote"
            | "hyperlinks"
            | "hyperlinks.local-style"
            | "images"
            | "images.local-style"
            | "hr"
            | "ul"
            | "ol"
            | "li"
            | "page.code"
            | "code"
            | "disclosure"
            | "code-block"
            | "markdown"
    )
}

/// Convert a snake_case path segment to its kebab-case equivalent.
/// Idempotent for kebab-case input. Every documented alias differs from its
/// canonical only by `_↔-`, so this transform alone canonicalizes any
/// alias-spelled segment.
pub fn kebabify(segment: &str) -> String {
    segment.replace('_', "-")
}

/// Join a parent path and a segment with `.`. An empty parent returns the
/// segment unchanged.
pub fn join_path(parent: &str, segment: &str) -> String {
    if parent.is_empty() {
        segment.to_string()
    } else {
        format!("{}.{}", parent, segment)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_lookup_finds_kebab_form() {
        let leaf = canonicalize("page.margin-left").expect("found");
        assert_eq!(leaf.canonical, "page.margin-left");
        assert_eq!(leaf.sub_spec, 2);
        assert_eq!(leaf.leaf_type, LeafType::HorizontalLength);
    }

    #[test]
    fn canonical_lookup_finds_snake_alias() {
        let leaf = canonicalize("page.max_width").expect("found");
        assert_eq!(leaf.canonical, "page.max-width");
        assert_eq!(leaf.alias, Some("page.max_width"));
    }

    #[test]
    fn unknown_path_returns_none() {
        assert!(canonicalize("page.lft-margin").is_none());
        assert!(canonicalize("planet.margin-left").is_none());
    }

    #[test]
    fn removed_page_spacing_names_are_not_leaves() {
        for old in ["left-margin", "right-margin", "top-margin", "bottom-margin", "left-padding", "right-padding", "top-padding", "bottom-padding"] {
            assert!(canonicalize(&format!("page.{old}")).is_none(), "{old}");
            assert!(canonicalize(&format!("page.{}", old.replace('-', "_"))).is_none(), "{old}");
        }
    }

    #[test]
    fn suggested_key_maps_removed_page_names_to_css_order() {
        assert_eq!(suggested_key("page.left-margin").as_deref(), Some("page.margin-left"));
        assert_eq!(suggested_key("page.bottom-padding").as_deref(), Some("page.padding-bottom"));
        assert_eq!(suggested_key("top-margin").as_deref(), Some("margin-top"));
        assert_eq!(suggested_key("table.left-margin"), None);
        assert_eq!(suggested_key("page.lft-margin"), None);
    }

    #[test]
    fn leaf_for_key_accepts_only_documented_spellings() {
        assert!(leaf_for_key("page.max-width", "max-width").is_some());
        assert!(leaf_for_key("page.max-width", "max_width").is_some());
        assert!(leaf_for_key("page.margin-left", "margin-left").is_some());
        assert!(leaf_for_key("page.margin-left", "margin_left").is_none());
        assert!(leaf_for_key("code-block.line-numbers", "line_numbers").is_none());
    }

    #[test]
    fn leaf_for_canonical_rejects_aliases() {
        assert!(leaf_for_canonical("page.max-width").is_some());
        assert!(leaf_for_canonical("page.max_width").is_none());
    }

    #[test]
    fn kebabify_idempotent_on_kebab() {
        assert_eq!(kebabify("max-width"), "max-width");
        assert_eq!(kebabify("alignment"), "alignment");
    }

    #[test]
    fn kebabify_converts_snake() {
        assert_eq!(kebabify("max_width"), "max-width");
        assert_eq!(kebabify("local_style"), "local-style");
        assert_eq!(kebabify("block_quote"), "block-quote");
    }

    #[test]
    fn join_path_handles_empty_parent() {
        assert_eq!(join_path("", "page"), "page");
        assert_eq!(join_path("page", "margin-left"), "page.margin-left");
    }

    #[test]
    fn schema_paths_are_unique() {
        // Detect duplicate canonical or alias entries — would cause double-
        // counted warnings.
        let mut seen = std::collections::BTreeSet::new();
        for leaf in SCHEMA {
            assert!(
                seen.insert(leaf.canonical),
                "duplicate canonical: {}",
                leaf.canonical
            );
            if let Some(alias) = leaf.alias {
                assert!(seen.insert(alias), "duplicate alias: {}", alias);
            }
        }
    }

    #[test]
    fn every_alias_differs_only_in_separator() {
        // The kebabify-based walker relies on every alias being a pure
        // `_↔-` transform of its canonical. If a future entry diverges from
        // that rule, the walker will silently miscategorize it.
        for leaf in SCHEMA {
            if let Some(alias) = leaf.alias {
                let canonical_from_alias = alias.replace('_', "-");
                assert_eq!(
                    canonical_from_alias, leaf.canonical,
                    "alias `{}` is not a pure `_↔-` transform of canonical `{}`",
                    alias, leaf.canonical
                );
            }
        }
    }
}
