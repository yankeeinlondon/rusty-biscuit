---
status: draft
date: 2026-05-22
owner: ken
parent: docs/rendering/style.md
sub-spec: 1-of-7
revision: 2
---

# `style:` Frontmatter — Sub-Spec #1: Schema & Parser

## Revision History

- **r1** (2026-05-22): Initial draft.
- **r2** (2026-05-22): Incorporated `review-spec.md`. Major changes: anchored
  schema to `renderable::{layout,color,style}` primitives instead of inventing
  local types; switched API from `serde_yaml::Value` to `serde_json::Value` to
  match `Frontmatter` storage; canonicalized kebab-case key spelling with
  snake-case aliases; split warning kinds so strict mode does not fail on valid
  unwired keys; removed `--strict-style` CLI flag from acceptance; added
  source-location hook and `serde_ignored` viability spike.

## Problem

The `md` CLI silently ignores the `style:` frontmatter property. A user document
like `darkmatter/example-docs/rendering/style-prop.md` carries page margins,
table alignment, list indents, and list max-widths under `style.*`, and
`md style-prop.md` renders it as if the block were absent.

Root cause (verified in `darkmatter/cli/src/output.rs:render_terminal_output`
and `html_artifact`): no code path reads `style:` from frontmatter. The only
`style` frontmatter reader in the codebase is HR's per-block `style: waves`
attribute in `hr_builder.rs:141`, which is unrelated.

The `style:` contract is documented in `darkmatter/docs/rendering/style.md` and
enumerates 9 component buckets (`page`, `table`, `hyperlinks`, `images`, `hr`,
`ul`, `ol`, `li`, `block_quote`), 5 common mutations (`width`, `max-width`,
`alignment`, `color`, `bg-color`), and bespoke per-component knobs
(`stylesheet`, `meta`, `code`, `local-style`, `kind`).

The full contract is too large for a single spec. This document covers
**sub-spec #1 only**: the frontmatter schema (Rust types built on existing
`renderable` primitives + serde deserialization) and the parser. No
render-pipeline integration. No `DarkmatterPage` changes. No HR migration.

## Decomposition Map (Context Only — NOT in Scope for This Spec)

| # | Sub-spec | Scope |
|---|---|---|
| **1 (this spec)** | **Schema & parser** | Define types (anchored to `renderable`), parse frontmatter, emit warnings for unknown keys. |
| 2 | Page-level wiring | Map parsed `style.page.*` onto existing `DarkmatterPage` builders. Introduce `--strict-style` CLI flag. |
| 3 | Existing-component wiring | Map `style.{table,images,block-quote}.*` onto existing `PageComponent::{Tables,Images,BlockQuotes}` builders. |
| 4 | `ul` / `ol` / `li` split + wiring | Split `PageComponent::Lists` into `Ul`/`Ol`/`Li` and wire `style.{ul,ol,li}.*`. |
| 5 | Color / bg-color mutations | New `DarkmatterPage` capability for per-component color and background-color (terminal + browser). Lowers through `renderable::color::Color`. |
| 6 | HR migration | Move `hr_builder.rs:117` HR style to `style.hr.*`; rename `hr.style` → `hr.kind`. |
| 7 | Bespoke knobs | `style.page.{stylesheet,meta,code}`, `style.hyperlinks.local-style`, `style.images.local-style`. |

Sub-specs #2 and #3, taken with this one, are the minimum needed for the test
doc `darkmatter/example-docs/rendering/style-prop.md` to render with its
configured margins/alignments/widths.

## Goals

- A sparse, typed schema for the `style:` frontmatter object, with every length
  / alignment / color value carrying a `renderable` runtime type (no
  darkmatter-local duplicates).
- A parser that consumes a `Markdown`'s `Frontmatter` and returns
  `(StyleFrontmatter, Vec<StyleWarning>)` without panicking on any well-formed
  YAML input.
- Field-aware length parsing: vertical fields take `u16` row counts; horizontal
  fields take `renderable::layout::Length` parsed from `"2ch"` / `"50%"` /
  bare `"40"`.
- Unknown-key diagnostics keyed by YAML path
  (`style.page.lft-margin`), distinct from "known but unwired" diagnostics so
  strict-mode validation doesn't fail on schema-valid documents.
- An `into_strict` library helper that converts schema-validation warnings
  (`UnknownKey`, `Deprecated`) into errors. CLI integration is sub-spec #2.

## Non-Goals

- No application of parsed values to rendering. (Sub-specs #2..#5.)
- No splitting of `PageComponent::Lists`. (Sub-spec #4.)
- No HR migration; the schema reserves `style.hr.*` keys but does not yet route
  them. (Sub-spec #6.)
- No CLI surface. `--strict-style` lands in sub-spec #2 alongside render
  wiring.
- No graph-level style propagation from composed parent documents to child
  documents. This spec also does not change render-tree style inheritance;
  runtime inheritance remains limited to the fields documented by
  `renderable::style::Style::inherited_from` (color and emphasis).
- No new runtime layout/style/color types. The schema MUST lower into
  `renderable::{layout,color,style}` types.

## Decisions (Settled by Brainstorm + Review)

1. **Scope target.** Full schema per `docs/rendering/style.md`, decomposed into
   seven sub-specs (this is #1).
2. **Unknown-key behavior.** Warn-and-continue by default; library helper
   `into_strict` collapses schema warnings to errors. CLI flag in sub-spec #2.
3. **Length typing.** Per-field:
   - Vertical fields (`top-margin`, `bottom-margin`, `top-padding`,
     `bottom-padding`) accept `u16` row counts only. Bare number required;
     `ch` and `%` rejected at parse.
   - Horizontal fields (`left-margin`, `right-margin`, `left-padding`,
     `right-padding`, `width`, `max-width`) accept `renderable::layout::Length`.
     Custom string deserializer accepts `"Nch"`, `"N%"`, or bare `"N"` (treated
     as `Length::Ch(N)`).
4. **Color in scope as a parser.** The schema parses color syntax now and
   lowers it through `renderable::color::Color` so sub-spec #5 does not need a
   second color migration. Opacity (which `renderable::color::Color` does not
   carry) is preserved via a thin `StyleColor` wrapper; sub-spec #5 decides
   whether to honor it on HTML-only targets.
5. **Canonical key spelling: kebab-case.** All multi-word frontmatter keys use
   hyphens: `max-width`, `bg-color`, `local-style`, `block-quote`,
   `left-margin`, etc. Snake-case spellings from the original `style.md`
   (`max_width`, `bg_color`, `local_style`, `block_quote`) are accepted via
   serde `alias` for one release cycle and emit `StyleWarning { kind:
   Deprecated { replacement: "<kebab>" } }`. Rationale: the user fixture,
   `biscuit-terminal`'s emitter, and the YAML community already use kebab-case;
   the parent doc gets a follow-up edit to match.
6. **No reuse of deprecated `darkmatter::layout::WidthUnit`.** Lengths lower to
   `renderable::layout::Length`. If sub-spec #2/#3 needs the legacy `WidthUnit`
   at the `DarkmatterPage` boundary, it converts at that boundary.

## Public API

### Module Location

A new top-level module: `darkmatter/lib/src/style/`.

```text
darkmatter/lib/src/style/
├── mod.rs              # Re-exports
├── schema.rs           # StyleFrontmatter + per-bucket structs
├── length.rs           # Custom deserializers producing renderable::layout::Length
├── color.rs            # StyleColor wrapper + deserializers producing renderable::color::Color
├── parse.rs            # from_frontmatter / from_json_value / into_strict
├── warning.rs          # StyleWarning + StyleWarningKind
└── tests/              # Parser tests + roundtrip tests
```

### Entry Points

```rust
// darkmatter::style

use crate::markdown::Frontmatter;

/// Parse a `Markdown`'s frontmatter `style:` value.
///
/// Reads `frontmatter.as_map().get("style")` and delegates to
/// [`from_json_value`]. Returns `(StyleFrontmatter::default(), vec![])` when no
/// `style:` key is present.
pub fn from_frontmatter(
    fm: &Frontmatter,
) -> Result<(StyleFrontmatter, Vec<StyleWarning>), StyleParseError>;

/// Parse the value at `style:` directly from a `serde_json::Value`.
///
/// `Frontmatter` stores its map as `IndexMap<String, serde_json::Value>`
/// (`darkmatter/lib/src/markdown/types.rs:18`), so this is the canonical entry
/// point for callers that already hold the value. Unknown keys are collected
/// into `warnings`; structural / type errors short-circuit via
/// `StyleParseError`.
pub fn from_json_value(
    value: &serde_json::Value,
) -> Result<(StyleFrontmatter, Vec<StyleWarning>), StyleParseError>;

/// Promote schema-validation warnings to errors. Schema-validation warnings
/// are `UnknownKey` and `Deprecated`; `KnownButInactive` warnings (valid keys
/// not yet wired to rendering) are deliberately ignored here so a
/// schema-strict caller does not fail on a forward-compatible document.
///
/// Returns `Ok(style)` when no `UnknownKey`/`Deprecated` warnings were
/// collected; otherwise `StyleParseError::Strict { warnings }` carries the
/// failing subset.
pub fn into_strict(
    parsed: (StyleFrontmatter, Vec<StyleWarning>),
) -> Result<StyleFrontmatter, StyleParseError>;
```

### Warning Channel

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct StyleWarning {
    /// Fully-qualified YAML path, e.g., `style.page.lft-margin`.
    pub path: String,
    /// Discriminated diagnostic category.
    pub kind: StyleWarningKind,
    /// Source position. Always `None` in v1; reserved so later spans don't
    /// break the public type.
    pub source_span: Option<StyleSpan>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StyleWarningKind {
    /// Schema-level: the path does not appear anywhere in the schema. Strict
    /// mode upgrades this to an error.
    UnknownKey,
    /// Schema-level: the path matched an alias for a renamed/canonicalized
    /// key. Strict mode upgrades this to an error.
    Deprecated { replacement: String },
    /// Informational: the key parsed and is structurally valid, but no
    /// rendering wiring exists for it yet in this binary. Lists which
    /// sub-spec will wire it. NEVER causes strict mode to fail.
    KnownButInactive { sub_spec: u8 },
}

/// Opaque source-span placeholder. v1 produces no spans; the type exists so
/// later sub-specs can populate it without changing the public surface of
/// `StyleWarning`.
#[derive(Debug, Clone, PartialEq)]
pub struct StyleSpan {
    pub line: u32,
    pub column: u32,
    pub length: u32,
}
```

### Error Type

```rust
#[derive(Debug, thiserror::Error)]
pub enum StyleParseError {
    #[error("Invalid YAML structure at `{path}`: expected {expected}, got {actual}")]
    Structure { path: String, expected: &'static str, actual: String },

    #[error("Invalid length `{raw}` at `{path}`: {reason}")]
    InvalidLength { path: String, raw: String, reason: &'static str },

    #[error("Invalid percent `{value}` at `{path}`: must be in 0.0..=100.0")]
    InvalidPercent { path: String, value: f32 },

    #[error("Invalid color `{raw}` at `{path}`: {reason}")]
    InvalidColor { path: String, raw: String, reason: &'static str },

    #[error("Strict mode: {} schema warning(s)", warnings.len())]
    Strict { warnings: Vec<StyleWarning> },

    #[error("serde error: {0}")]
    Serde(#[from] serde_json::Error),
}
```

## Schema

The schema is a sparse tree of `Option` fields keyed by component. Every
length, alignment, and color value lowers into a `renderable` primitive at
parse time — no darkmatter-local duplicates.

### Root

```rust
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
pub struct StyleFrontmatter {
    pub page: Option<PageStyle>,
    pub table: Option<TableStyle>,
    pub hyperlinks: Option<HyperlinkStyle>,
    pub images: Option<ImageStyle>,
    pub hr: Option<HrStyle>,
    pub ul: Option<UlStyle>,
    pub ol: Option<OlStyle>,
    pub li: Option<LiStyle>,
    #[serde(alias = "block_quote")]
    pub block_quote: Option<BlockQuoteStyle>,
}
```

Note `#[serde(rename_all = "kebab-case")]` rewrites the field `block_quote` to
the wire key `block-quote`; the `alias` accepts the legacy `block_quote`
spelling and triggers a `Deprecated` warning via the post-deserialization
walker (since serde does not surface alias hits, the walker compares the raw
map keys against the canonical set — see "Unknown Key Detection").

All buckets are `Option` so a sparse user input (only `page.left-margin`) does
not materialize default values across the tree.

### Common Mutations

```rust
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
pub struct CommonStyle {
    #[serde(deserialize_with = "length::deserialize_optional_length")]
    pub width: Option<renderable::layout::Length>,
    #[serde(deserialize_with = "length::deserialize_optional_length",
            alias = "max_width")]
    pub max_width: Option<renderable::layout::Length>,
    #[serde(deserialize_with = "alignment::deserialize_optional")]
    pub alignment: Option<renderable::layout::Alignment>,
    #[serde(deserialize_with = "color::deserialize_optional")]
    pub color: Option<StyleColor>,
    #[serde(deserialize_with = "color::deserialize_optional", alias = "bg_color")]
    pub bg_color: Option<StyleColor>,
}
```

### Length Deserializer

```rust
// darkmatter::style::length

use renderable::layout::Length;

/// Parse a horizontal-length string into a `renderable::layout::Length`.
///
/// Accepted forms:
/// - `"2ch"` / `"2 ch"` → `Length::Ch(2)`
/// - `"40"` (bare) → `Length::Ch(40)`
/// - `"50%"` / `"50.5%"` → `Length::Percent(50.0)` / `Length::Percent(50.5)`
///
/// Rejected forms (return `StyleParseError::InvalidLength`):
/// - `"-2"`, `"-2ch"` (negative)
/// - `"2px"`, `"2em"`, `"2rem"` (unsupported units)
/// - `""` (empty)
/// - `"50%%"`, `"50%/"` (malformed)
/// - `"101%"` (percent out of `0.0..=100.0` range — uses
///   `Length::percent` constructor's existing range check)
pub fn deserialize<'de, D>(deserializer: D) -> Result<Length, D::Error>
where D: serde::Deserializer<'de> { /* ... */ }

pub fn deserialize_optional<'de, D>(deserializer: D)
    -> Result<Option<Length>, D::Error>
where D: serde::Deserializer<'de> { /* ... */ }
```

For vertical fields, `u16` deserialization is built-in. A small
`row_count::deserialize` helper explicitly rejects strings to catch
`top-margin: "2ch"` with a `Structure` error rather than serde's default
"invalid type" message.

### Alignment Deserializer

```rust
// darkmatter::style::alignment

use renderable::layout::Alignment;

/// Deserialize an alignment string into `renderable::layout::Alignment`.
///
/// `renderable::layout::Alignment` already derives `Deserialize` with
/// `#[serde(rename_all = "snake_case")]` — it accepts `"left"`, `"center"`,
/// `"right"`. This wrapper adds the documented alias `"centered"` → `Center`.
pub fn deserialize<'de, D>(deserializer: D) -> Result<Alignment, D::Error>
where D: serde::Deserializer<'de> { /* ... */ }
```

### Color Schema

```rust
// darkmatter::style::color

/// A frontmatter color value: an underlying `renderable::color::Color`, plus
/// optional opacity for HTML targets.
///
/// `renderable::color::Color` already models RGB, named, Tailwind, web-named,
/// and reset colors; opacity is the only delta from the existing type. This
/// wrapper exists so opacity survives the parse without forking the color
/// model.
#[derive(Debug, Clone, PartialEq)]
pub struct StyleColor {
    pub color: renderable::color::Color,
    /// Tailwind-style opacity (`/50` → `Some(50)`), in `0..=100`. Documented
    /// as HTML-only by `docs/rendering/style.md`; terminal targets drop it.
    pub opacity: Option<u8>,
}

/// Parse a color string into `StyleColor`.
///
/// Accepted forms:
/// - Tailwind name: `"red-500"`, `"red-500/50"`. Lowered to
///   `Color::Tailwind(Tailwind::Red500)` plus opacity 50.
/// - Hex: `"#fff"`, `"#ffffff"`, `"#ffffffff"`. Lowered to
///   `Color::Rgb(RgbColor::new(r, g, b, fallback))`. Alpha hex (last 2 chars
///   of `#rrggbbaa`) is converted to `opacity = Some(alpha * 100 / 255)`.
/// - Web named: `"orange"`, `"rebeccapurple"`. Lowered via the existing
///   `WEB_COLOR_LOOKUP`.
///
/// Rejected forms: anything that doesn't match the three patterns above
/// (`StyleParseError::InvalidColor`).
pub fn deserialize<'de, D>(deserializer: D) -> Result<StyleColor, D::Error>
where D: serde::Deserializer<'de> { /* ... */ }
```

The Tailwind family + level enumeration is taken from
`renderable::color::Tailwind` (which already enumerates all 21 families × 11
levels). The Tailwind string parser maps `"red-500"` → the matching enum
variant; it does not redefine the taxonomy.

### Per-Component Buckets

#### `PageStyle`

```rust
use renderable::layout::Length;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
pub struct PageStyle {
    // Margins (4 sides, mixed types).
    #[serde(deserialize_with = "length::deserialize_optional")]
    pub left_margin: Option<Length>,
    #[serde(deserialize_with = "length::deserialize_optional")]
    pub right_margin: Option<Length>,
    pub top_margin: Option<u16>,
    pub bottom_margin: Option<u16>,
    // Padding (4 sides, mixed types).
    #[serde(deserialize_with = "length::deserialize_optional")]
    pub left_padding: Option<Length>,
    #[serde(deserialize_with = "length::deserialize_optional")]
    pub right_padding: Option<Length>,
    pub top_padding: Option<u16>,
    pub bottom_padding: Option<u16>,
    // Page knobs.
    #[serde(deserialize_with = "length::deserialize_optional",
            alias = "max_width")]
    pub max_width: Option<Length>,
    #[serde(deserialize_with = "alignment::deserialize_optional")]
    pub alignment: Option<renderable::layout::Alignment>,
    #[serde(deserialize_with = "color::deserialize_optional")]
    pub color: Option<StyleColor>,
    #[serde(deserialize_with = "color::deserialize_optional", alias = "bg_color")]
    pub bg_color: Option<StyleColor>,
    pub background: Option<PageBackgroundLevel>,
    // Bespoke (parsed but inactive in v1).
    pub stylesheet: Option<String>,                    // file path or URL
    pub meta: Option<serde_json::Value>,               // opaque map
    pub code: Option<CodeStyle>,
}

/// `transparent` | `subtle` | `pronounced`. Re-exported from
/// `darkmatter::layout::PageBackground` to avoid duplicating the variant set.
pub use crate::layout::PageBackground as PageBackgroundLevel;
```

#### `TableStyle`, `BlockQuoteStyle`

```rust
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
pub struct TableStyle {
    #[serde(flatten)]
    pub common: CommonStyle,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
pub struct BlockQuoteStyle {
    #[serde(flatten)]
    pub common: CommonStyle,
}
```

#### `UlStyle`, `OlStyle`, `LiStyle`

```rust
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
pub struct UlStyle {
    #[serde(flatten)]
    pub common: CommonStyle,
    /// Indent applied to ul content. The test doc exercises this with
    /// `style.ul.left-margin: 4ch`. Wiring (sub-spec #4) will translate to
    /// `PageFill::Indent` on `PageComponent::Ul`.
    #[serde(deserialize_with = "length::deserialize_optional")]
    pub left_margin: Option<renderable::layout::Length>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
pub struct OlStyle {
    #[serde(flatten)]
    pub common: CommonStyle,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
pub struct LiStyle {
    #[serde(flatten)]
    pub common: CommonStyle,
}
```

#### `HyperlinkStyle`, `ImageStyle`

```rust
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
pub struct HyperlinkStyle {
    #[serde(flatten)]
    pub common: CommonStyle,
    /// Override applied to file-local links only.
    #[serde(alias = "local_style")]
    pub local_style: Option<Box<CommonStyle>>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
pub struct ImageStyle {
    #[serde(flatten)]
    pub common: CommonStyle,
    #[serde(alias = "local_style")]
    pub local_style: Option<Box<CommonStyle>>,
}
```

#### `HrStyle`

```rust
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
pub struct HrStyle {
    #[serde(flatten)]
    pub common: CommonStyle,
    /// Replaces the legacy per-block `style: waves` attribute (sub-spec #6
    /// migration). v1 schema accepts an opaque `String`; the HR builder is
    /// not rerouted until #6. The exact `kind` enum is intentionally deferred
    /// to that sub-spec to avoid duplicating the HR-kind taxonomy in two
    /// places before the migration.
    pub kind: Option<String>,
}
```

## Unknown Key & Alias Detection

The parser needs three pieces of information for every key it sees:

1. Is this key in the schema? (UnknownKey if not.)
2. Did the user write the canonical kebab spelling, or a snake-case alias?
   (Deprecated if alias.)
3. Is the key in the schema but unwired in this binary? (KnownButInactive.)

`#[serde(deny_unknown_fields)]` aborts on the first typo and gives one error
per call — not what we want. The `serde_ignored` crate captures ignored fields
during deserialization, but the reviewer flagged a real concern:
**`serde_ignored` requires the deserializer to surface ignored fields, and
behavior across `#[serde(flatten)]` structs and `Option<T>` newtypes is not
guaranteed.**

This spec therefore commits to **a two-pass parser**:

```text
Pass 1 — Canonicalization walk
  Walk the raw `serde_json::Value` map of the `style:` value.
  For each leaf path:
    a. Compute its canonical kebab-case path against the schema descriptor.
    b. If the raw key is a documented snake_case alias, emit
       Deprecated { replacement }.
    c. If the path is not in the schema at all, emit UnknownKey.

Pass 2 — Typed deserialization
  Deserialize the (possibly already-canonicalized) value into
  `StyleFrontmatter` via standard serde. Structural errors short-circuit via
  StyleParseError.

Pass 3 — KnownButInactive annotation
  Walk the parsed `StyleFrontmatter` and emit a KnownButInactive warning for
  every `Some` field whose wiring sub-spec is not #1.
```

The pass-1 walk is driven by a static schema descriptor (one entry per leaf:
canonical name, set of accepted aliases, parent path). This avoids depending
on `serde_ignored`'s behavior under `flatten`/`Option`, and gives us
total control over alias detection. A small acceptance test against the schema
descriptor proves coverage of every leaf.

If a future spike shows `serde_ignored` works cleanly through `flatten`, the
two-pass design can be collapsed into a single deserialize-with-tracking call;
the public API doesn't change.

## Tests

### Unit Tests (`darkmatter/lib/src/style/tests/`)

1. **Length deserializer** — table-driven, covering every row from the
   "Length Deserializer" section: accepted forms (`"2ch"`, `"40"`, `"50%"`,
   `"50.5%"`) and rejected forms (`"-2"`, `"2px"`, `""`, `"50%%"`, `"101%"`).
2. **Color deserializer** — every `Tailwind` family × level combination
   parses; opacity `/0`, `/50`, `/100` accepted, `/101` rejected. Hex `#fff`,
   `#ffffff`, `#ffffffff` accepted; alpha hex converted to opacity. `#fg0`
   rejected with reason `"non-hex digit"`. Web named (`"orange"`) routed
   through `WEB_COLOR_LOOKUP`.
3. **Alignment deserializer** — `left`, `center`, `centered`, `right` all
   produce the canonical `renderable::layout::Alignment` variant; `middle`
   rejected.
4. **Vertical-field type guard** — `top-margin: 2ch` fails with
   `StyleParseError::Structure`; `top-margin: 1` succeeds.
5. **Sparse parse** — a frontmatter with only `style.page.left-margin` yields
   a `StyleFrontmatter` where every other bucket is `None`.
6. **UnknownKey warning** — `style.page.lft-margin` emits exactly one
   `StyleWarning { path: "style.page.lft-margin", kind: UnknownKey, .. }`.
7. **Multiple UnknownKey warnings** — three typos under different buckets
   emit three distinct warnings (regression for the pass-1 walk completeness).
8. **Deprecated alias warning** — `style.block_quote.max_width: 50%` emits
   two `Deprecated` warnings (`block_quote → block-quote`,
   `max_width → max-width`) and parses successfully.
9. **KnownButInactive annotation** — every wired field in `style-prop.md`
   produces a `KnownButInactive { sub_spec }` warning with the expected
   sub-spec number from the decomposition map.
10. **Strict mode** — `into_strict` returns `Ok` when only
    `KnownButInactive` warnings are present; returns
    `StyleParseError::Strict` when any `UnknownKey` / `Deprecated` warning is
    present.
11. **Unknown-key coverage through `flatten`** — typos *inside* a
    `CommonStyle`-flattened struct (e.g., `style.table.maxx-width`) are
    detected. This is the explicit serde_ignored-fragility test.
12. **Unknown-key coverage in nested `local_style`** —
    `style.hyperlinks.local-style.maxx-width` is detected. Same rationale.
13. **No new color model** — `StyleColor.color` is a
    `renderable::color::Color`, not a darkmatter-local enum. Compile-time
    test: `let _: renderable::color::Color = parse(...).color;`.

### Integration Test (`darkmatter/lib/tests/style_frontmatter.rs`)

14. **Test-doc roundtrip** — parse
    `darkmatter/example-docs/rendering/style-prop.md` and assert the resulting
    `StyleFrontmatter` matches a hand-written expected value field-by-field
    (see Acceptance Criteria). All warnings are `KnownButInactive`.

## Open Questions

These need a decision before implementation begins. None block implementation.

1. **`renderable::color::Tailwind` opacity coverage.** Does the existing
   `Tailwind` enum carry per-level opacity variants, or is opacity purely
   our wrapper concern? If the existing type already supports opacity, drop
   `StyleColor.opacity` and use `renderable::color::Color` directly.
   *Disposition:* needs a 5-minute read of
   `renderable/src/color/tailwind.rs`.

2. **`PageBackgroundLevel` re-export source.** Re-export from
   `darkmatter::layout::PageBackground` (current draft) or pull the variant
   set into `renderable::style`? *Disposition:* keep as draft (re-export
   darkmatter's existing type); revisit if sub-spec #5 needs cross-target
   semantics.

3. **`page.code` schema shape.** `style.md` lists `page.code` as a bespoke
   knob but doesn't define its fields. v1 parses it as
   `Option<CodeStyle>` with a single `theme: Option<String>` field, deferring
   the real shape to sub-spec #7.

4. **`page.meta` semantics.** v1 parses as opaque
   `Option<serde_json::Value>`; meaning deferred to sub-spec #7.

5. **`hr.kind` taxonomy.** v1 carries as `Option<String>`; the typed enum is
   defined as part of sub-spec #6's HR migration.

6. **`Markdown::style()` convenience accessor.** Defer to sub-spec #2 when
   render code starts using it. v1 ships the free function only.

7. **Crate dependency: any new dep?** v1 design avoids `serde_ignored`
   (two-pass walk handles unknowns). If the two-pass walk proves too slow on
   large frontmatters, sub-spec #1.5 can introduce `serde_ignored` as an
   optimization. v1 needs no new workspace deps.

## Acceptance Criteria

- `darkmatter::style::from_frontmatter(md.frontmatter())` is callable from any
  workspace member that depends on `darkmatter`.
- Parsing `darkmatter/example-docs/rendering/style-prop.md` returns a
  `StyleFrontmatter` whose:
  - `page.left_margin == Some(Length::Ch(2))`
  - `page.right_margin == Some(Length::Ch(4))`
  - `page.top_margin == Some(1)`
  - `page.bottom_margin == Some(0)`
  - `table.common.alignment == Some(Alignment::Right)`
  - `table.common.max_width == Some(Length::Percent(50.0))`
  - `ol.common.alignment == Some(Alignment::Right)`
  - `ul.common.alignment == Some(Alignment::Left)`
  - `ul.left_margin == Some(Length::Ch(4))`
  - `ul.common.max_width == Some(Length::Ch(40))`
- All warnings from the fixture parse are `KnownButInactive`.
- All 14 tests in the **Tests** section pass.
- `cargo doc -p darkmatter` produces no new warnings for the new module.
- No existing test in `darkmatter/` regresses.
- `md` CLI behavior is unchanged (no rendering wired yet); `md --strict-style`
  does not exist in this sub-spec.
- **No second runtime model:** the schema introduces no new layout/style/color
  *runtime* types. Every length is `renderable::layout::Length`, every
  alignment is `renderable::layout::Alignment`, every color is
  `renderable::color::Color` (wrapped only by `StyleColor` for opacity).
- **Kebab-case canonical, snake-case alias:** every documented snake-case key
  from `style.md` is accepted via serde `alias` and emits a `Deprecated`
  warning; the spec's tests prove this for every multi-word key.
- **Unknown-key detection through `flatten` and nested `local_style`** is
  proven by tests #11 and #12.
- **Frontmatter representation match:** the parser operates on
  `serde_json::Value` (the actual storage of `Frontmatter::as_map`) and uses
  `biscuit_file::serde_yaml_ng` only where raw YAML text parsing is needed.
- **Strict mode is well-behaved on valid documents:** `into_strict` succeeds
  on documents whose only warnings are `KnownButInactive`, even when every
  key is unwired in v1.

## Risks

- **Schema descriptor drift.** The pass-1 walk depends on a static schema
  descriptor (canonical names + aliases). If a developer adds a new field to
  a bucket struct but forgets to add it to the descriptor, the new field
  becomes an `UnknownKey` warning instead of parsing. Mitigation: a build-time
  consistency test (`#[test] fn descriptor_matches_schema`) that iterates the
  schema via a derived `Reflect`-like trait or via a hand-maintained list and
  compares against the descriptor.
- **`serde_yaml_ng` ↔ `serde_json::Value` lossiness.** Frontmatter is parsed
  from YAML into `serde_json::Value`, which loses YAML-specific types (e.g.,
  YAML timestamps become strings). For `style:` this is fine — every value is
  a string, number, or nested map — but document the constraint in the
  module rustdoc.
- **Schema churn between sub-specs.** Sub-spec #4 (lists split) and #6 (HR
  migration) may force schema renames. Mitigation: every rename ships behind
  a serde `alias` for one release cycle, with a `Deprecated` warning.

## Out-of-Spec — Where the Behavior Lands

The user's failing scenario (`md style-prop.md` ignoring `style:`) is **not
fixed by this spec alone**. It is fixed by sub-spec #1 + #2 + #3 together
(parse + page wiring + existing-component wiring). Shipping #1 alone gives
one immediate, library-callable improvement: a downstream caller (a linter,
an LSP, a future `md --strict-style`) can read the schema and report every
parsed style key with its current wiring status, making the silent-ignore
problem observable.
