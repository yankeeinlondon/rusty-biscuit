# Spec A — The `Layout` Primitive

> **Status:** Draft for review — revised to incorporate architect review
> (`spec-review.md`).
> **Date:** 2026-05-17
> **Package area:** `renderable` (with `darkmatter` / `biscuit-terminal` migration)
> **Relationship:** First of two specs. Spec B (the `Style` / `CssStyle` slot
> system, the `style` frontmatter namespace, `HtmlPage` class dedup) is a
> sibling and builds on a settled `Layout`.

## Context

The render-tree architecture has been implemented for seven components
(`Section`, `UnorderedList`, `OrderedList`, `TwoColumn`, `Progress`, `Table`,
and darkmatter's `YamlBlock`). A drift report comparing the tree renderers
against the bespoke renderers shows **291 drift entries**. A large share of
that drift is *pattern-based* — the tree renderers do nothing with margins,
alignment, max-width, or wrapping. Layout is, today, simply ignored.

The deeper problem is not that layout is unimplemented — it is that the
*concept* of layout is **forked across three incompatible representations**:

| Representation | Where | Problem |
|---|---|---|
| `renderable::layout::Layout` | `renderable/src/layout.rs` | Rich, but terminal-only in spirit (`resolve_margin` against a width) and it leaks *appearance* (`page_bg_color`, `row_fill_strategy`). |
| `tree::LayoutHints` | `renderable/src/tree/attrs.rs` | Anemic — four optional margins, nothing else. What the tree pipeline actually carries. |
| `DarkmatterPage` page types | `darkmatter/src/layout/` | `PageMargin` / `PagePadding` / `PageFill` / `PageAlignment` — a divergent re-invention of the `Layout` contract, bound to terminal cells. |

None of the three is consumed by the tree renderers. We are not retrofitting a
working system; we are deciding what the *real* layout model is.

## Goals

- One `Layout` primitive, used by **every block-level component**, replacing
  all three forks above.
- A component **declares** its layout; it never **implements** it. The tree
  renderers apply layout.
- One layout expression renders correctly on **Terminal**, **Browser**, and
  **Markdown** with no lossy unit conversion in the common path.
- Burn down the layout-related slice of the 291-entry drift ledger.
- Prefer the cleanest, highest-performing architecture even when it requires
  more refactoring.

## Non-Goals (deferred to Spec B)

- The `Style` / `CssStyle` slot system (per-component styleable targets such as
  table row/col/cell).
- The `style` frontmatter namespace schema (`style.hr.{layout,style}`, …) and
  the retirement of the ad-hoc `hr_css_variables` "superpowers."
- `HtmlPage` component-default class dedup and inline-`style` override
  bubbling.
- Appearance properties pulled out of today's `Layout` (`page_bg_color`,
  `row_fill_strategy`).
- Inline content / `Prose`. Inline components carry **no** `Layout`.
- **Markdown frontmatter emission.** Spec A emits *no* `style` frontmatter; it
  only leaves a clean seam (see D7).

## Design Decisions

### D1 — Two primitives, split on an "outer vs. inner" axis

Layout and Style are distinct concerns and must be distinct types:

- **`Layout`** — a block-level component's relationship to its **parent/page**:
  outer margins, alignment *within* the parent, max-width, content wrapping.
  Generic; every block component has one.
- **`Style`** — how a component **paints itself and its named sub-parts**
  (Spec B).

`DarkmatterPage` is the cautionary tale: it fuses both (page margins *and*
per-component alignment/fill) into one terminal-cell-bound struct. This spec
pulls them apart.

### D2 — `Layout` is a single presentational tier

`Layout` has **no semantic tier**. Its vocabulary — margins, alignment,
max-width, wrapping — is already small and target-neutral. Named semantic
intents ("callout", "hero", "de-emphasized") are a **Style** concern and are
out of scope here. `Layout` stays a plain typed data struct.

### D3 — Unit model: `Length`, `TargetValue<T>`, and universal units

#### `Length`

`Length` is the concrete length type. The **universal units** (`Ch`,
`Percent`) and `Zero` are valid on every render target. `Css` carries a
target-native value and is valid **only** inside a per-target branch (see
`TargetValue` below).

```rust
/// A layout length.
pub enum Length {
    /// Zero. Unit-independent.
    Zero,
    /// Cells. Whole cells only — `u32`, non-negative by construction. A cell
    /// is the universal spacing unit on both axes; the browser renderer
    /// lowers it per-axis automatically (see below).
    Ch(u32),
    /// Percentage of the available width. Stored as `0.0..=100.0`.
    Percent(f32),
    /// A target-native CSS length (`rem`, `px`, `em`, …). Reuses the
    /// stylesheet module's `CssSizing`. Only valid inside the per-target
    /// branch of a `TargetValue`.
    Css(CssSizing),
}
```

Decisions this pins down:

- **Numeric types:** `Ch` is a `u32` whole unit — fractional terminal spacing
  is meaningless and integers make resolution exact. Fractional browser values
  are only reachable via `Length::Css` inside a per-target branch.
- **Negative lengths:** not representable (`u32`). Invalid by construction.
- **Percent range:** `0.0..=100.0` (matches the existing
  `Layout::resolve_margin` convention). `NaN`, infinity, and out-of-range
  values are rejected at construction / validation time with a
  `LayoutError::InvalidPercent`.
- **Zero:** `Length::Zero` is unit-independent, so `margin: 0` need not pick a
  unit.
- **Axis handling is automatic.** `Ch` is the single universal cell unit and
  is valid on every side. The browser renderer lowers it **per axis**: a `Ch`
  on a horizontal property emits `ch`, a `Ch` on a vertical property
  (`margin-top` / `margin-bottom`) emits `lh` — the line-height multiplier,
  which is what most authors expect ("`2ch` of top margin" = two blank rows in
  the terminal = `2lh` in the browser). This resolves the architect's
  "vertical `ch` is awkward" concern without a second unit or author-facing
  axis rules. An author who wants different vertical behavior uses the
  per-target escape hatch.

#### `TargetValue<T>`

```rust
pub enum TargetValue<T> {
    /// One value for every target. Universal-units only.
    Universal(T),
    /// Per-target values. Non-empty. Each entry may use that target's
    /// native units.
    PerTarget(BTreeMap<RenderTarget, T>),
}
```

- **Universal** holds a `Length` restricted to `Zero` / `Ch` / `Percent`. A
  `Length::Css` in the `Universal` branch is a validation error pointing the
  author to the per-target escape hatch.
- **PerTarget** maps `RenderTarget → Length`. A `PerTarget` map applies the
  property **only** to the targets it names; unnamed targets do not receive it.
- An **empty** `PerTarget` map is invalid.
- A single-key `PerTarget` map cleanly expresses a target-only property.

#### `RenderTarget` and fallback

`RenderTarget` is `{ Terminal, Browser, Markdown, MarkdownPlus }`. The legacy
`Ast` variant on `renderable::target::RenderTarget` is **removed** as part of
this migration (the `AstRenderable` trait is already gone).

Per-target lookup is deterministic:

- A renderer for target `T` looks up `T` in the map.
- **`MarkdownPlus` falls back to `Markdown`** when it has no own entry.
- A map may carry both `Markdown` and `MarkdownPlus` entries.
- If, after fallback, no entry matches, the property is **absent** for that
  target (it does **not** fall back to a universal default — `PerTarget` and
  `Universal` are mutually exclusive forms of a single field).

#### Why this model

Terminal cells are discrete integers; browser units are continuous.
Discrete→continuous conversion is lossless; continuous→discrete always rounds
unpredictably. Restricting the universal form to `Ch`/`Percent` means we
**never** down-sample browser units to the terminal. No magic constants.

`TargetValue<T>` is the **one piece of unit machinery shared with Spec B**.

### D4 — The `Layout` struct

`Layout` lives in `renderable::layout` and replaces the three forks.

```rust
pub struct Layout {
    pub margin: Margin,
    pub alignment: Alignment,
    pub max_width: Option<TargetValue<Length>>,
    pub word_wrap: WordWrap,
}

pub struct Margin {
    pub top: TargetValue<Length>,
    pub right: TargetValue<Length>,
    pub bottom: TargetValue<Length>,
    pub left: TargetValue<Length>,
}
```

| Field | Notes |
|---|---|
| `margin` | 4-sided box. All sides take the same `Ch`/`Percent`/`Zero` units; the browser renderer lowers vertical sides to `lh` automatically (D3). |
| `alignment` | `Left` / `Center` / `Right` within the parent's available width. |
| `max_width` | Horizontal cap on content width. |
| `word_wrap` | Content flow (re-exported from `wrap_policy`). Kept — it is flow, not appearance. |

- **Removed** from today's `Layout`: `page_bg_color`, `row_fill_strategy`
  (appearance → Spec B).
- **Excluded** deliberately: `padding` — inner space is a
  component-internal/appearance concern → Spec B.
- `Margin` exposes ergonomic constructors (`all`, `x`, `y`, per-side) mirroring
  the current `PageMargin` builders.

### D5 — `Layout` lives on the render tree, on block nodes only

`Layout` attaches to a block `RenderNode` via a typed accessor on `NodeAttrs`,
**promoting** today's `LayoutHints` into the full `Layout` type. The renderers
read it during the fold.

- `TreeRenderable::tree_layout_hints()` is retyped to return `Option<Layout>`
  and **seeds the layout on the component's root node**. (The method may be
  renamed for clarity during implementation; the new name is left to the plan.)
- Nested block nodes may each carry their own `Layout`.
- **Rejected:** a separate node-id-keyed side-channel map — two structures that
  must travel and stay in sync is a maintenance tax; `NodeAttrs` already exists
  for exactly this and serializes with the tree.

**Block-only validation.** Because `NodeAttrs` can attach to any node, a layout
could be set on an inline node (`Text`, `Span`, `Emphasis`, `InlineCode`, …).
The tree validator gains a rule: **layout attributes are permitted only on
block-level nodes.**

- `RenderStrictness::Strict` — validation **fails** (`RenderError`).
- `RenderStrictness::Warn` / `Lossy` — the layout is **ignored** and a
  `Diagnostic` is recorded.

There is **no axis validation** — `Ch` is valid on every margin side and on
`max_width`; the browser renderer handles the horizontal/vertical distinction
automatically (D3).

### D6 — Layout composition (non-inherited)

Layout is **not inherited**. A child block's `Layout` neither merges with nor
inherits from its parent's. The **only** thing that flows downward is the
**available width**: a renderer applies a parent block's `margin` /
`max_width`, computes the resulting inner width, and passes that reduced width
to the children. Each child then resolves its own `Layout` against that width.

- `word_wrap` does **not** inherit — each block carries its own; a block with
  no explicit `word_wrap` uses the `Layout` default, not its parent's value.
- `alignment` does not inherit.

This keeps every renderer's behavior identical and avoids divergence.

### D7 — Per-target consumption

#### Terminal — applied, integer resolution

The terminal tree renderer applies layout. Resolution is **explicitly integer**
and part of the contract:

- `Ch(n)` → `n` whole cells: columns on horizontal sides, rows on vertical
  sides.
- `Percent(p)` → resolved against the **current available width** as
  `(width as f32 * p / 100.0).round()` — **round half up**, matching today's
  `Layout::resolve_margin`. This preserves existing visual output.
- Vertical margins resolve to **line counts**, never physical lengths.
- Overflow **saturates** (`saturating_sub` on width math) — never panics.
- `alignment` is an offset within the available width and is only observable
  when content is narrower than that width (typically under `max_width`).

This absorbs the behavior currently in `DarkmatterPage::apply_row_decoration`,
lifted into the terminal tree renderer.

#### Browser — lowered to CSS, per field

- `margin` → CSS margin longhands (`margin-top/right/bottom/left`). `Ch` lowers
  **per axis**: `ch` on `margin-left`/`margin-right`, `lh` on
  `margin-top`/`margin-bottom`. `Percent` → `%`, `Css(_)` → its native form.
- `max_width` → `max-width`.
- `alignment` → **block** alignment via `margin-left/right: auto`, emitted
  **only when `max_width` is present** (an unconstrained block fills its
  container and has nothing to align). With no `max_width`, `alignment` emits
  nothing. `text-align` is **not** emitted — inline alignment is a Style
  concern (Spec B).
- `word_wrap` → an explicit mapping: `WordWrap::None` → `white-space: nowrap`;
  wrapping variants → `overflow-wrap: break-word` (the wrap-prose tunables have
  no faithful CSS analogue and are documented as terminal-only).

#### Markdown — body stays clean, no emission

- The Markdown tree renderer emits **no** layout into the Markdown body.
- Dropping layout from the body is **by design**, not lossy-by-accident — no
  diagnostic is raised.
- Markdown's eventual layout path is the `style` frontmatter (a CSS-derived
  theming block interpreted by the downstream renderer). That **schema is Spec
  B**; Spec A implements **no frontmatter emission**.
- `MarkdownPlus` may use inline HTML for layout **only** where the dialect
  already opts into HTML for a construct; Spec A adds no new HTML-emission
  paths for layout.

### D8 — Serialization

`Layout` rides on `NodeAttrs`, which serializes with the tree, so `Layout`,
`Margin`, `TargetValue`, and `Length` need **stable serde shapes**. They derive
`Serialize` / `Deserialize`. Conventions:

- Enum casing: `snake_case` for unit/keyword variants (`ch`, `percent`, `zero`,
  `left`, `center`, `right`).
- `TargetValue::Universal` serializes as the bare value; `PerTarget` as a map
  keyed by lower-case target name.

Sample — a block node carrying `margin-left: 2ch` and
`margin-right: { browser: 5em, terminal: 5ch }`:

```json
{
  "layout": {
    "margin": {
      "top": { "zero": null },
      "right": { "per_target": { "browser": { "css": "5em" },
                                 "terminal": { "ch": 5 } } },
      "bottom": { "zero": null },
      "left": { "ch": 2 }
    },
    "alignment": "left",
    "max_width": null,
    "word_wrap": "none"
  }
}
```

The exact tagging (internally vs. externally tagged) is an implementation
detail, but a serialized example **must** appear in the `Layout` rustdoc and be
covered by a round-trip test so the shape cannot drift silently.

### D9 — Migration and compatibility

#### Render-tree side

- Delete `tree::LayoutHints`; `Layout` is the single tree-carried type.
- Rework the seven tree-migrated components (`Section`, `UnorderedList`,
  `OrderedList`, `TwoColumn`, `Progress`, `Table`, `YamlBlock`) to emit
  `Layout`, and burn down their layout-related drift.

#### `biscuit-terminal` extension ergonomics

`biscuit-terminal` owns `LayoutTerminalExt` and bespoke renderers call
`apply_layout` / `apply_block_layout`. To keep churn off the non-tree-migrated
components:

- `renderable::layout::Layout` remains the shared **data** type.
- Terminal-only layout **application** stays in `biscuit-terminal`.
- `LayoutTerminalExt` is adapted to the new field names and the `TargetValue` /
  `Length` model.
- Bespoke terminal renderers keep working throughout the migration — this is
  not a big-bang cutover.

#### `DarkmatterPage` and public page-layout types

- `DarkmatterPage` survives as a **page assembler** but stops re-inventing the
  layout contract: page-level margins become a `Layout` on the document root.
- `PageMargin` / `PagePadding` / `PageFill` / `PageAlignment` are **public**
  Darkmatter types. The migration:
  - identifies every public use site;
  - maps the concepts onto `Layout` (`PageMargin` → `Margin`;
    `PageFill::{Max,Explicit}` → `max_width`; `PageFill::{Pad,Indent}` →
    `margin`; `PageAlignment` → `Alignment`);
  - provides `From` / `TryFrom` conversions for a deprecation window rather
    than a hard break, **or** records an explicit decision to break if the
    package area accepts it;
  - updates **frontmatter parsing** separately from render-tree layout
    application, so the two concerns are not entangled in one change.

## Success Criteria

- A single `Layout` type exists in `renderable::layout`; `tree::LayoutHints`
  and `DarkmatterPage`'s `PageMargin`/`PagePadding`/`PageFill`/`PageAlignment`
  are gone (or behind deprecation conversions per D9).
- A block component declares margins / alignment / max-width / wrapping once,
  via `Layout`, and the Terminal and Browser tree renderers apply it without
  the component implementing layout itself.
- The legacy `RenderTarget::Ast` variant is removed.
- `cargo test` / `cargo clippy` pass for `renderable`, `darkmatter`, and
  `biscuit-terminal`.

### Required tests

- **Margins** applied on `Section`, `UnorderedList`, `OrderedList`, `Table`,
  `Progress`, `TwoColumn`, and `YamlBlock`.
- **Alignment** with and without `max_width` (alignment is observable only
  under a width constraint).
- **Percentage margins** resolved at several terminal widths, including small
  widths and asymmetric left/right margins, asserting the round-half-up policy.
- **Per-target value** — a `browser`-only length that produces browser CSS and
  does **not** affect terminal output.
- **Invalid universal units** — `Length::Css` (`px`/`em`/`rem`) in a
  `Universal` branch is rejected with an actionable error.
- **Vertical lowering** — a `Ch` margin on `margin-top`/`margin-bottom` emits
  `lh` in browser CSS, while a `Ch` on `margin-left`/`margin-right` emits `ch`.
- **Block-only validation** — layout on an inline node fails under `Strict`,
  warns under `Warn`/`Lossy`.
- **Composition** — a child block resolves against the width left by its
  parent's margin/`max_width`; layout fields do not inherit.
- **Markdown** — body output is byte-for-byte unchanged when a block carries a
  `Layout`, and no diagnostic is raised.
- **Serde round-trip** — the documented serialized shape for a layout-carrying
  node.

## Open Questions / Seams for Spec B

- The `style` frontmatter namespace schema (`style.{component}.{layout,style}`)
  — Spec A leaves the Markdown seam; Spec B defines the schema and the
  frontmatter emission path.
- Whether `Layout` and a future `Style` ultimately share a parsing/validation
  surface beyond `TargetValue<T>`.
- Per-instance (vs per-element-type) layout divergence in Markdown — expressible
  only in MarkdownPlus via generated inline HTML; detailed in Spec B.
