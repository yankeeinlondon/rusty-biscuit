//! Static catalog of every leaf path the schema understands.
//!
//! Used by the canonicalization walker (pass 1 of the parser) and the
//! typed-pre-validation pass to detect unknown keys, snake-case aliases, and
//! to dispatch to the right typed validator. Add a row here whenever a field
//! is added to any per-bucket schema struct.
//!
//! Sub-specs covered: #2 (page), #3 (table/images/block-quote), #4 (lists),
//! #5 (color), #6 (hr), #7 (bespoke knobs), #8 (disclosure), #9 (Style
//! Everywhere expanded surface).
//!
//! ## Canonical-path discipline
//!
//! The walker and validators canonicalize *per segment* by replacing `_` with
//! `-` before looking up against `SCHEMA.canonical`. The `alias` field on
//! `SchemaLeaf` is therefore redundant for lookups — it exists so the
//! alias-coverage tests can enumerate every documented snake-case spelling
//! without re-deriving them.

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
    /// `u16` row count parsed from a JSON integer.
    RowCount,
    /// `renderable::layout::Alignment` parsed from `"left" | "center" |
    /// "centered" | "right"`. Validated by serde — no pre-validator emitted.
    Alignment,
    /// `StyleColor` parsed from a Tailwind name, hex, or web-color name.
    Color,
    /// `PageBackground` enum (`"transparent" | "subtle" | "pronounced"`).
    /// Validated by serde — no pre-validator emitted.
    BackgroundEnum,
    /// Free-form string (e.g. `stylesheet`, `code.theme`).
    StringValue,
    /// Opaque `serde_json::Value` (e.g. `page.meta`). Accepts anything.
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
}

/// A single schema leaf: its canonical kebab-case path, any documented
/// snake-case alias, the sub-spec that wires it, and its typed category.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SchemaLeaf {
    /// Dotted canonical path under `style.` (e.g. `"page.left-margin"`).
    pub canonical: &'static str,
    /// Snake-case alias path, if any (e.g. `"page.left_margin"`). Kept for
    /// the alias-coverage tests; not used by the walker.
    pub alias: Option<&'static str>,
    /// Sub-spec number that wires this key to rendering. All current entries
    /// point to one of sub-specs #2..#7.
    pub sub_spec: u8,
    /// Typed category of this leaf — drives typed-error pre-validation.
    pub leaf_type: LeafType,
}

/// The complete schema catalog. Every leaf reachable through any per-bucket
/// struct must appear here.
#[rustfmt::skip]
pub const SCHEMA: &[SchemaLeaf] = &[
    // ── page ────────────────────────────────────────────────────────────
    SchemaLeaf { canonical: "page.left-margin",    alias: Some("page.left_margin"),    sub_spec: 2, leaf_type: LeafType::HorizontalLength },
    SchemaLeaf { canonical: "page.right-margin",   alias: Some("page.right_margin"),   sub_spec: 2, leaf_type: LeafType::HorizontalLength },
    SchemaLeaf { canonical: "page.top-margin",     alias: Some("page.top_margin"),     sub_spec: 2, leaf_type: LeafType::RowCount },
    SchemaLeaf { canonical: "page.bottom-margin",  alias: Some("page.bottom_margin"),  sub_spec: 2, leaf_type: LeafType::RowCount },
    SchemaLeaf { canonical: "page.left-padding",   alias: Some("page.left_padding"),   sub_spec: 2, leaf_type: LeafType::HorizontalLength },
    SchemaLeaf { canonical: "page.right-padding",  alias: Some("page.right_padding"),  sub_spec: 2, leaf_type: LeafType::HorizontalLength },
    SchemaLeaf { canonical: "page.top-padding",    alias: Some("page.top_padding"),    sub_spec: 2, leaf_type: LeafType::RowCount },
    SchemaLeaf { canonical: "page.bottom-padding", alias: Some("page.bottom_padding"), sub_spec: 2, leaf_type: LeafType::RowCount },
    SchemaLeaf { canonical: "page.max-width",      alias: Some("page.max_width"),      sub_spec: 2, leaf_type: LeafType::HorizontalLength },
    SchemaLeaf { canonical: "page.alignment",      alias: None,                        sub_spec: 2, leaf_type: LeafType::Alignment },
    SchemaLeaf { canonical: "page.color",          alias: None,                        sub_spec: 5, leaf_type: LeafType::Color },
    SchemaLeaf { canonical: "page.bg-color",       alias: Some("page.bg_color"),       sub_spec: 5, leaf_type: LeafType::Color },
    SchemaLeaf { canonical: "page.background",     alias: None,                        sub_spec: 2, leaf_type: LeafType::BackgroundEnum },
    SchemaLeaf { canonical: "page.stylesheet",     alias: None,                        sub_spec: 7, leaf_type: LeafType::StringValue },
    SchemaLeaf { canonical: "page.meta",           alias: None,                        sub_spec: 7, leaf_type: LeafType::OpaqueValue },
    SchemaLeaf { canonical: "page.code.theme",     alias: None,                        sub_spec: 7, leaf_type: LeafType::StringValue },

    // ── table ───────────────────────────────────────────────────────────
    SchemaLeaf { canonical: "table.width",       alias: None,                    sub_spec: 3, leaf_type: LeafType::WidthOrMode },
    SchemaLeaf { canonical: "table.max-width",   alias: Some("table.max_width"), sub_spec: 3, leaf_type: LeafType::HorizontalLength },
    SchemaLeaf { canonical: "table.alignment",   alias: None,                    sub_spec: 3, leaf_type: LeafType::Alignment },
    SchemaLeaf { canonical: "table.color",       alias: None,                    sub_spec: 5, leaf_type: LeafType::Color },
    SchemaLeaf { canonical: "table.bg-color",    alias: Some("table.bg_color"),  sub_spec: 5, leaf_type: LeafType::Color },
    SchemaLeaf { canonical: "table.margin",      alias: None,                    sub_spec: 9, leaf_type: LeafType::HorizontalLength },
    SchemaLeaf { canonical: "table.padding",     alias: None,                    sub_spec: 9, leaf_type: LeafType::HorizontalLength },
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
    SchemaLeaf { canonical: "hyperlinks.local-style.border",    alias: Some("hyperlinks.local_style.border"),           sub_spec: 9, leaf_type: LeafType::CompoundStyle },
    SchemaLeaf { canonical: "hyperlinks.local-style.emphasis",  alias: Some("hyperlinks.local_style.emphasis"),         sub_spec: 9, leaf_type: LeafType::CompoundStyle },
    SchemaLeaf { canonical: "hyperlinks.local-style.word-wrap", alias: Some("hyperlinks.local_style.word_wrap"),        sub_spec: 9, leaf_type: LeafType::WordWrap },

    // ── images ──────────────────────────────────────────────────────────
    SchemaLeaf { canonical: "images.width",                 alias: None,                                       sub_spec: 3, leaf_type: LeafType::WidthOrMode },
    SchemaLeaf { canonical: "images.max-width",             alias: Some("images.max_width"),                   sub_spec: 3, leaf_type: LeafType::HorizontalLength },
    SchemaLeaf { canonical: "images.alignment",             alias: None,                                       sub_spec: 3, leaf_type: LeafType::Alignment },
    SchemaLeaf { canonical: "images.color",                 alias: None,                                       sub_spec: 5, leaf_type: LeafType::Color },
    SchemaLeaf { canonical: "images.bg-color",              alias: Some("images.bg_color"),                    sub_spec: 5, leaf_type: LeafType::Color },
    SchemaLeaf { canonical: "images.local-style.width",     alias: Some("images.local_style.width"),           sub_spec: 7, leaf_type: LeafType::WidthOrMode },
    SchemaLeaf { canonical: "images.local-style.max-width", alias: Some("images.local_style.max_width"),       sub_spec: 7, leaf_type: LeafType::HorizontalLength },
    SchemaLeaf { canonical: "images.local-style.alignment", alias: Some("images.local_style.alignment"),       sub_spec: 7, leaf_type: LeafType::Alignment },
    SchemaLeaf { canonical: "images.local-style.color",     alias: Some("images.local_style.color"),           sub_spec: 7, leaf_type: LeafType::Color },
    SchemaLeaf { canonical: "images.local-style.bg-color",  alias: Some("images.local_style.bg_color"),        sub_spec: 7, leaf_type: LeafType::Color },
    SchemaLeaf { canonical: "images.local-style.margin",    alias: Some("images.local_style.margin"),          sub_spec: 9, leaf_type: LeafType::HorizontalLength },
    SchemaLeaf { canonical: "images.local-style.padding",   alias: Some("images.local_style.padding"),         sub_spec: 9, leaf_type: LeafType::HorizontalLength },
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
    SchemaLeaf { canonical: "hr.margin",    alias: None,                 sub_spec: 9, leaf_type: LeafType::HorizontalLength },
    SchemaLeaf { canonical: "hr.padding",   alias: None,                 sub_spec: 9, leaf_type: LeafType::HorizontalLength },
    SchemaLeaf { canonical: "hr.border",    alias: None,                 sub_spec: 9, leaf_type: LeafType::CompoundStyle },
    SchemaLeaf { canonical: "hr.emphasis",  alias: None,                 sub_spec: 9, leaf_type: LeafType::CompoundStyle },
    SchemaLeaf { canonical: "hr.word-wrap", alias: Some("hr.word_wrap"), sub_spec: 9, leaf_type: LeafType::WordWrap },

    // ── disclosure ──────────────────────────────────────────────────────
    SchemaLeaf { canonical: "disclosure.width",     alias: None,                          sub_spec: 8, leaf_type: LeafType::WidthOrMode },
    SchemaLeaf { canonical: "disclosure.max-width", alias: Some("disclosure.max_width"),  sub_spec: 8, leaf_type: LeafType::HorizontalLength },
    SchemaLeaf { canonical: "disclosure.alignment", alias: None,                          sub_spec: 8, leaf_type: LeafType::Alignment },
    SchemaLeaf { canonical: "disclosure.color",     alias: None,                          sub_spec: 8, leaf_type: LeafType::Color },
    SchemaLeaf { canonical: "disclosure.bg-color",  alias: Some("disclosure.bg_color"),   sub_spec: 8, leaf_type: LeafType::Color },
    SchemaLeaf { canonical: "disclosure.margin",    alias: None,                          sub_spec: 9, leaf_type: LeafType::HorizontalLength },
    SchemaLeaf { canonical: "disclosure.padding",   alias: None,                          sub_spec: 9, leaf_type: LeafType::HorizontalLength },
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
    SchemaLeaf { canonical: "code-block.border",    alias: Some("code_block.border"),    sub_spec: 9, leaf_type: LeafType::CompoundStyle },
    SchemaLeaf { canonical: "code-block.emphasis",  alias: Some("code_block.emphasis"),  sub_spec: 9, leaf_type: LeafType::CompoundStyle },
    SchemaLeaf { canonical: "code-block.word-wrap", alias: Some("code_block.word_wrap"), sub_spec: 9, leaf_type: LeafType::WordWrap },
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

/// `true` if `path` is a canonical container — a node that has children but
/// is not itself a leaf. Containers include the top-level buckets plus the
/// nested `local-style` and `page.code` paths.
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
            | "disclosure"
            | "code-block"
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
        let leaf = canonicalize("page.left-margin").expect("found");
        assert_eq!(leaf.canonical, "page.left-margin");
        assert_eq!(leaf.sub_spec, 2);
        assert_eq!(leaf.leaf_type, LeafType::HorizontalLength);
    }

    #[test]
    fn canonical_lookup_finds_snake_alias() {
        let leaf = canonicalize("page.left_margin").expect("found");
        assert_eq!(leaf.canonical, "page.left-margin");
        assert_eq!(leaf.alias, Some("page.left_margin"));
    }

    #[test]
    fn unknown_path_returns_none() {
        assert!(canonicalize("page.lft-margin").is_none());
        assert!(canonicalize("planet.left-margin").is_none());
    }

    #[test]
    fn leaf_for_canonical_rejects_aliases() {
        assert!(leaf_for_canonical("page.left-margin").is_some());
        assert!(leaf_for_canonical("page.left_margin").is_none());
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
        assert_eq!(join_path("page", "left-margin"), "page.left-margin");
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
