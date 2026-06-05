# Spec B — The `Style` Primitive

> **Status:** DRAFT — preliminary. This document gathers the raw material for
> the styling work and sketches a design. Decisions marked **(OPEN)** are not
> settled; the document is meant to be iterated on before it becomes an
> implementation plan.
> **Date:** 2026-05-17
> **Package area:** `renderable` (with `biscuit-terminal` / `darkmatter` migration)
> **Relationship:** Sibling of [Spec A — The `Layout` Primitive](./spec.md),
> which is implemented and shipped. Spec A built `Layout`; this spec builds the
> appearance sibling, `Style`. Spec A explicitly deferred everything here.
> `Prose`'s own cross-target rendering is a **separate, decoupled** feature —
> see [`2026-05-17-prose-cross-target/spec.md`](../2026-05-17-prose-cross-target/spec.md);
> `Style` does not absorb `Prose` (see D6).
> **Initial scope (this pass):** styling for the **`biscuit-terminal`
> components**, proven on the **Terminal** target first. Browser and Markdown
> lowering are designed here but implemented later.

## Context

Spec A consolidated *layout* — a block component's relationship to its parent
(margins, alignment, max-width, wrapping) — into one `Layout` primitive that
components declare and the tree renderers apply. It deliberately left
*appearance* untouched, and even **evicted** two appearance fields
(`page_bg_color`, `row_fill_strategy`) from the legacy `Layout` without
re-homing them.

The result is that appearance has **no home in the tree pipeline**. Today it is
expressed three incompatible ways, all bespoke:

| Where | How appearance is expressed | Problem |
|---|---|---|
| `BlockQuote` | inline `text_color` / `bg_color` / `left_block_color` fields, applied as raw ANSI during `render` | not on the tree; the tree renderer cannot see or apply it |
| `Section` | hard-coded `\x1b[1m` / `\x1b[3m` per heading level | no color, no configurability, not declarative |
| `Table` | boolean `alternate_background_color()` / `alternate_text_color()` flags, resolved against light/dark mode at render time | table-wide only; no per-row/col/cell control |
| `Progress` | `char` fields for fill/empty/bracket glyphs | no color at all |
| `darkmatter` | `PageBackground` (`Subtle`/`Pronounced`), `PageFill`, and the ad-hoc `hr_css_variables` raw `:root` CSS map | a fourth representation, page-bound, bypasses `CssStyle` |

The tree renderers now apply `Layout` faithfully but **discard appearance** —
this is a large share of the `Styling`-facet entries in the drift ledger. A
component cannot declare "this heading is bold and accent-colored" in a way the
tree renderer will honor.

This spec defines `Style`: the appearance primitive a component **declares**
and the tree renderers **apply**, exactly as `Layout` works today.

## Goals

- One `Style` primitive, declared by components, applied by the tree
  renderers — the appearance sibling to `Layout`.
- A component **declares** appearance; it never hand-writes ANSI or CSS.
- Re-home the appearance concepts evicted from `Layout` and scattered across
  bespoke component fields: foreground/background color, text emphasis,
  borders, and the fill/background-band concepts from `PageBackground` /
  `PageFill`.
- Reuse Spec A's machinery — `TargetValue<T>` for per-target values, and the
  existing `renderable::color` and `renderable::stylesheet` modules — rather
  than inventing parallel infrastructure.
- Burn down the `Styling`-facet slice of the drift ledger (see the
  `render-tree behind` metric in `just drift-report`).
- Degrade gracefully: a `Style` must render on a 16-color terminal, a
  truecolor terminal, and a browser, without the component knowing which.

## Non-Goals / Scope Boundaries

In scope for this pass (the `biscuit-terminal` component slice):

- The `Style` type and its terminal application.
- Re-homing `page_bg_color` / `RowFill` / `PageBackground` appearance.
- Migrating the bespoke appearance fields on `BlockQuote`, `Section`, `Table`,
  `Progress` (and the other tree components) onto `Style`.

Designed here but **implemented later** (or in a follow-on spec):

- **Browser lowering** of `Style` to CSS. The design is sketched (D7) so the
  terminal work does not paint us into a corner, but the browser renderer wiring
  is a later step.
- **The `style` frontmatter namespace** (`style.{component}.{layout,style}`)
  and **retiring `hr_css_variables`**. This is a darkmatter-frontmatter concern;
  it depends on a settled `Style` type and should be its own sub-spec.
- **`HtmlPage` component-default class dedup** and inline-`style` override
  bubbling — a browser-page-assembly concern, deferred with browser lowering.

Explicitly **out of scope** (not `Style` at all):

- **Syntax highlighting.** Code-block highlighting already has its own seam —
  the `CodeRenderer` trait (`render_terminal_code` / `render_browser_code`) and
  `CodeRenderHints`. Highlighting is a renderer-provided *capability*, not a
  declarative property a component sets. `Style` does **not** absorb it; the
  `CodeRenderer` hook stays independent. (A code block may still carry a
  `Style` for its *frame* — border, background — separate from the highlighted
  *content*.)
- Layout. Margins, alignment, max-width, wrapping are `Layout` (Spec A) and are
  not duplicated here.

## Raw Material — Existing Infrastructure

This is reference material for the design decisions below.

- **`renderable::color`** — `Color` enum (`BasicColor`, `Rgb(RgbColor)`,
  `Web(WebColor)`, `Tailwind`, plus `DefaultForeground` / `DefaultBackground` /
  `Reset`). Deterministic `.to_rgb()`. `ColorDepth` / `ColorMode` /
  `TerminalCodeContext` capability descriptors already exist for
  capability-aware lowering.
- **`renderable::stylesheet`** — `CssStyle` (ordered typed declaration map),
  `CssProp` and typed property subsets (`CssColorProp`, `CssSizingProp`, …),
  `CssValue` / `CssSizing` / `CssUnit` / `CssColor`. Type-safe builder; runtime
  validation on parse.
- **`renderable::layout`** — `TargetValue<T>` is generic over `T` and is the
  designated shared unit machinery (Spec A D3: *"`TargetValue<T>` is the one
  piece of unit machinery shared with Spec B"*). `MarkdownPlus`→`Markdown`
  resolution fallback.
- **`NodeAttrs` hint namespaces** — `renderable.layout`, `renderable.list`,
  `renderable.table`, `renderable.code`, `renderable.terminal`,
  `renderable.widget.progress`, `renderable.widget.columns`. `Layout` rides on
  `renderable.layout` via `set_layout` / `layout`. `Style` would add
  `renderable.style` and follow the identical typed-accessor pattern.
- **Evicted / bespoke appearance to re-home** — `darkmatter`'s
  `PageBackground { Transparent, Subtle, Pronounced }` (mode-relative
  background intensity); `PageFill { Full, Pad, Indent, Max, Explicit }` (its
  *width* semantics already went to `Layout` in Spec A D9 — only the
  *paint-the-band* sense remains for `Style`); `BlockQuote`'s color fields;
  `Table`'s alternating-row tints.

## Design Decisions (DRAFT)

### D1 — The `Style` / `Layout` boundary

Settled by Spec A D1/D2 and restated here:

- **`Layout`** — a block's relationship to its parent: outer margins,
  alignment, max-width, wrapping. *Where the box sits.*
- **`Style`** — how a component **paints itself and its named sub-parts**:
  color, background, text emphasis, borders, fills. *What the box looks like.*

`Layout` is non-semantic plain data. For v1, `Style` does not get a
**semantic tier** (named intents like "callout" / "hero" / "de-emphasized"):
`Style` remains plain concrete serde data, like `Layout`. Higher-level APIs,
component helpers, or darkmatter shims may expose semantic concepts, but they
resolve to concrete `Style` before entering the render tree. `StyleIntent` does
not enter the render tree in v1.

### D2 — Appearance vocabulary

The properties `Style` should express, drawn from the bespoke material above:

| Property | Type sketch | Re-homes |
|---|---|---|
| foreground color | `Option<TargetValue<PerMode<Color>>>` | `BlockQuote.text_color`, section heading color |
| background color | `Option<TargetValue<PerMode<Color>>>` | `BlockQuote.bg_color`, `Table` row tint, `PageBackground` |
| text emphasis | shared `TextEmphasis` leaf (bold, italic, dim, underline, strikethrough, blink) | `Section` heading bold/italic |
| border | `Option<Border>` — color, weight, line style, sides, radius | `BlockQuote.left_block_color` |
| fill / background band | `Option<Fill>` — color, intensity, band, inset | `PageBackground` intensity, `RowFill` coloring |

**Settled:** `fill` remains separate from adaptive color. `Fill` models
band-painting behavior — how/whether the available width is painted — while
adaptive light/dark values are represented by `PerMode<T>` (D3). v1 uses the
rich visual model for both `Border` and `Fill` (D4).

### D3 — Per-target values and capability degradation

- Per-target overrides reuse `TargetValue<T>` (e.g.
  `TargetValue<PerMode<Color>>` for a color that differs between Browser and
  Terminal). Universal is the common case; `PerTarget` is the escape hatch.
- Light/dark adaptation uses a new `PerMode<T>` wrapper, composed with
  `TargetValue` for color-bearing style values. The common shape is
  `TargetValue<PerMode<Color>>`, with convenience constructors for universal
  values and light/dark pairs so callers do not hand-build nested enums for the
  common cases.
- `Fill` is not the general adaptive color mechanism. A fill may contain a
  color-bearing value, but its reason to exist is band-painting behavior.
- **Capability degradation is part of the contract.** A `Color` is authored
  once; the terminal renderer degrades it to the terminal's `ColorDepth`
  (truecolor → 256 → 16) using `renderable::color`'s existing machinery. The
  component never branches on color depth.

### D4 — The `Style` struct (DRAFT sketch)

```rust
/// How a block-level component paints itself. Sibling of `Layout`.
pub struct Style {
    /// Foreground (text) color.
    pub color: Option<TargetValue<PerMode<Color>>>,
    /// Background color of the component's box.
    pub background: Option<TargetValue<PerMode<Color>>>,
    /// Text emphasis — the shared `renderable::style::TextEmphasis` leaf
    /// (also reused by `Prose`'s `ProseStyle`). For *inline* content prefer the
    /// `Strong` / `Emphasis` / `Delete` node kinds; the `bold` / `italic` /
    /// `strikethrough` flags here exist for block-component slot styling and
    /// because the leaf is shared.
    pub emphasis: TextEmphasis,
    /// Border appearance, if any.
    pub border: Option<Border>,
    /// Fill — how/whether the component paints its band of available width.
    pub fill: Option<Fill>,
}

/// Shared appearance leaf — **already implemented** in `renderable::style`
/// (shipped by the Prose cross-target feature) and reused by `Prose`'s
/// `ProseStyle`. `Style` embeds it as-is. All-default is no emphasis.
pub struct TextEmphasis {
    pub bold: bool,
    pub dim: bool,
    pub italic: bool,
    pub strikethrough: bool,
    pub blink: bool,
    /// Underline variant, if any. `UnderlineStyle` is
    /// `Straight | Double | Curly | Dotted | Dashed`; the terminal emitter
    /// degrades unsupported variants per terminal capability.
    pub underline: Option<UnderlineStyle>,
}
```

`Border` and `Fill` use the v1 rich visual model:

```rust
pub struct Border {
    pub color: Option<TargetValue<PerMode<Color>>>,
    pub weight: BorderWeight,
    pub line_style: BorderLineStyle,
    pub sides: BorderSides,
    pub radius: Option<TargetValue<Length>>,
}

pub struct Fill {
    pub color: Option<TargetValue<PerMode<Color>>>,
    pub intensity: FillIntensity,
    pub band: FillBand,
    pub inset: Option<TargetValue<Length>>,
}
```

`Border` is one aggregate entity with uniform defaults plus side-specific
addressing. Authors can apply one border style to all sides, enable only
selected sides, or override an individual side such as `left` without
introducing a separate conceptual type like `LeftBorder`. Side-specific
settings override aggregate border defaults. `Fill` remains separate from
`background`: it models painted-band behavior, not adaptive color selection
(D2/D3).

`PerMode<T>` is the light/dark adaptation wrapper used by color-bearing style
values:

```rust
pub enum PerMode<T> {
    Universal(T),
    Adaptive { light: T, dark: T },
}
```

It should provide convenience constructors for universal values and adaptive
light/dark pairs, plus resolution against `ColorMode`.

`renderable::style` already ships, alongside `TextEmphasis`, the shared
emitters Spec B consumes: `TextEmphasis::sgr_ops` + `EmphasisLayer` (terminal
SGR, with per-layer parent restoration) and `TextEmphasis::html_wrappers`
(browser semantic-HTML / CSS). `Style` calls these rather than re-deriving
emphasis emission.

This is a plain serde data struct, like `Layout`. **D8 note:** the shipped
`TextEmphasis` / `UnderlineStyle` derive only `Debug, Clone, Copy, Default,
PartialEq, Eq` — Spec B must add `Serialize` / `Deserialize` to them when
`Style` embeds `TextEmphasis`, since `Style` rides on `NodeAttrs` and must
round-trip (D8).

### D5 — The slot system — styling named sub-parts

A single `Style` per component is not enough. A `Table` has rows, columns,
cells, a header, borders; a `Section` has a heading and a body; a `BlockQuote`
has a bar and content; a `Progress` has a filled track, an empty track, and
brackets. Spec A D1 explicitly says `Style` covers *"named sub-parts."*

**Settled:** v1 uses a hybrid model.

- `Style` remains the universal primitive on `NodeAttrs`.
- Simple components may use the node's generic `Style` directly.
- Components with rich slot surfaces use typed component style structs, e.g.
  `TableStyle` or `ProgressStyle`, whose fields are `Style` values or richer
  typed leaves where needed.
- v1 should avoid making fully stringly typed slots the stable public API.
  Internal serialization may still use stable field names, but the public
  authoring surface for rich components should be typed.

### D6 — `Style` on the render tree

- `Style` rides on `NodeAttrs` under a new `HintNamespace::STYLE`
  (`renderable.style`), with typed accessors `set_style` / `style`, exactly
  mirroring `set_layout` / `layout`. Rich component slot surfaces ride through
  typed component style structs (D5).
- **Inline vs block — settled.** Spec A D5 made `Layout` **block-only**.
  `Style` is **not** block-only: it carries *appearance* (`color`,
  `background`, `border`, `fill`) and may attach to block nodes **and** inline
  `Span` nodes — the latter matters for inline-HTML appearance carried into the
  tree from Markdown. `Style` does **not** carry **semantic emphasis**: bold /
  italic / strikethrough / links / inline code are the existing inline **node
  kinds** (`Strong`, `Emphasis`, `Delete`, `Link`, `InlineCode`), which
  round-trip to Markdown natively. Because semantic emphasis lives in node
  kinds, `Style` never needs to emit Markdown — D7 holds with no exception.
- **`Prose` is decoupled — settled.** `Prose`'s cross-target rendering is
  handled by its own IR (`ProseNode` / `ProseStyle`), specified separately in
  [`2026-05-17-prose-cross-target/spec.md`](../2026-05-17-prose-cross-target/spec.md).
  `Style` does **not** absorb `Prose` and is **not** the substrate `Prose`
  lowers onto. The two subsystems share only **leaf primitives** — the
  `renderable::color::Color` enum and the `renderable::style::TextEmphasis`
  decoration leaf (D4) — never the container types.
- **Composition / inheritance — settled, limited.** Only text appearance
  fields inherit through render tree traversal: `color` and `emphasis`.
  Box-painting properties do not inherit: `background`, `border`, and `fill`
  remain explicit on the node or typed slot that paints them. This keeps the
  useful text cascade without making `Style` a full CSS cascade engine.

### D7 — Per-target consumption

Mirrors Spec A D7. Terminal is implemented first.

- **Terminal** — `Style` lowers to ANSI SGR: `color`/`background` to foreground/
  background SGR (degraded to `ColorDepth`), `emphasis` to the SGR attribute
  set, `border`/`fill` to box-drawing glyphs and painted background bands. The
  terminal tree renderer applies it during the fold, the same place it applies
  `Layout`. Absorbs the bespoke ANSI currently in `BlockQuote` / `Section` /
  `Table`.
- **Browser** *(designed, implemented later)* — `Style` lowers to a `CssStyle`
  (the typed builder already exists): `color`→`color`, `background`→
  `background-color`, `emphasis`→`font-weight`/`font-style`/`text-decoration`/
  `opacity`, `border`→`border`. Per Spec A's deferral, this is where the
  `HtmlPage` class-dedup question lives.
- **Markdown** — emits **no** style into the Markdown body, by design, no
  diagnostic — identical to `Layout` (Spec A D7). A future `style` frontmatter
  block is the Markdown story and is out of scope here.

### D8 — Serialization

`Style` rides on `NodeAttrs` and serializes with the tree. `Style`,
`PerMode<T>`, `Border`, `Fill`, and typed component style structs derive
`Serialize` / `Deserialize` with `snake_case` enum casing; the embedded shared
leaves `TextEmphasis` / `UnderlineStyle` need those derives **added** (they ship
without them — see the D4 note). A documented JSON sample must appear in the
`Style` rustdoc with a round-trip test, exactly as Spec A D8 requires for
`Layout`.

### D9 — Migration

- **Core component v1.** Add `Style`, terminal support, and migrate
  `BlockQuote`, `Section`, `Table` stripe colors, and `Progress` slot colors /
  glyph styling into declared styles or typed component style structs per D5.
  Existing builders stay available as deprecated compatibility shims and
  internally write or convert to `Style`.
- **Narrow darkmatter bridge only as needed.** Use a small bridge for drift
  parity if the migrated component slice needs one. Full page-style migration
  is deferred.
- **`hr_css_variables`** — retire the ad-hoc `:root` CSS map in favor of `Style`
  + the `style` frontmatter schema. This is the most entangled piece and is
  **deferred** to the frontmatter sub-spec (see Non-Goals).
- **Frontmatter** — the `style` frontmatter namespace is deferred with
  `hr_css_variables`; it is not required for the component-styling v1.

## Success Criteria (DRAFT)

- A single `Style` type exists in `renderable::style` (the module already
  ships with the shared leaves), declared by components, applied by the
  terminal tree renderer without the component hand-writing ANSI.
- `BlockQuote`, `Section`, `Table` stripe colors, and `Progress` slot colors /
  glyph styling are expressed as declared styles or typed component style
  structs. Old component builders still work and internally write or convert to
  `Style`.
- Render-tree terminal output matches current bespoke output for the migrated
  components.
- Markdown output is unchanged.
- A `Color` in a `Style` renders correctly on 16-color, 256-color, and
  truecolor terminals (capability degradation), and `PerMode<Color>` adapts to
  light/dark mode where required.
- The `Styling`-facet drift attributable to dropped appearance is burned down
  in `just drift-report`.
- `cargo test` / `cargo clippy` pass for `renderable`, `biscuit-terminal`,
  `darkmatter`.

### Required tests (DRAFT)

- Foreground / background color applied on each migrated component, terminal
  target.
- Emphasis (bold / italic / dim / underline / strikethrough) → correct SGR.
- Color degradation at each `ColorDepth`.
- Light/dark adaptation for a `PerMode<Color>` value.
- Typed component slot styling — e.g. a styled table header vs body.
- Serde round-trip of the documented `Style` JSON shape.
- Serde round-trip coverage for migrated component style structs and the old
  builder compatibility path.
- Terminal parity fixtures for migrated components: render-tree output matches
  current bespoke output.
- Markdown body unchanged when a node carries a `Style` (no diagnostic).

## Open Questions

Resolved decisions and remaining questions to track before this draft becomes
an implementation plan.

1. **Inline vs block (D6).** *Resolved.* `Style` applies to block nodes and
   inline `Span` nodes for *appearance*; semantic emphasis stays in the
   inline node kinds. `Prose` is decoupled — it has its own IR and shares only
   leaf primitives with `Style`. See D6.
2. **The slot system (D5).** *Resolved.* Hybrid: `Style` remains the universal
   primitive on `NodeAttrs`; rich slot surfaces use typed component style
   structs. v1 does not commit fully stringly typed slots as the stable public
   API.
3. **Inheritance (D6).** *Resolved.* Limited inheritance for text appearance
   only: `color` and `emphasis` cascade; `background`, `border`, and `fill` do
   not.
4. **Light/dark adaptation (D3).** *Resolved.* Add `PerMode<T>` with
   convenience constructors, composed with `TargetValue` for adaptive
   light/dark style values. `Fill` remains band-painting behavior.
5. **Semantic tier (D1/D2).** *Resolved.* `Style` stays plain concrete serde
   data like `Layout`. Higher-level APIs, component helpers, or darkmatter
   shims may expose semantic concepts, but they resolve to concrete `Style`
   before entering the render tree. No `StyleIntent` enters the render tree in
   v1.
6. **Module home.** *Resolved:* `renderable::style` — the module **already
   exists** (`renderable/src/style.rs`, shipped by the Prose cross-target
   feature) and holds the shared leaves (`TextEmphasis`, `UnderlineStyle`,
   `EmphasisLayer`). Spec B adds the `Style` primitive to it. Still open: does
   `Style` *contain* a `CssStyle` for the browser path, or lower to one?
7. **Border / fill modeling (D2/D4).** *Resolved.* v1 uses the rich visual
   model: `Border { color, weight, line_style, sides, radius }` and
   `Fill { color, intensity, band, inset }`. `Border` is one aggregate entity
   with uniform defaults plus side-specific addressing; side-specific settings
   override aggregate border defaults. `Fill` remains separate from
   `background` and models painted-band behavior.
8. **`Style` ↔ `CodeRenderer`.** Confirmed separate — but a code block's
   *frame* (border/background) is `Style` while its *content* highlighting is
   `CodeRenderer`. Confirm the seam holds.
9. **Frontmatter & `hr_css_variables`.** Sequenced as a follow-on sub-spec and
   not needed for the component-styling v1.

## Relationship to Spec A

| Aspect | Spec A — `Layout` | Spec B — `Style` |
|---|---|---|
| Concern | where the box sits | what the box looks like |
| Scope | block-level only | block **and** inline `Span` (settled, D6) |
| Inheritance | non-inherited | limited: `color` / `emphasis` only |
| Per-target machinery | `TargetValue<T>` | `TargetValue<T>` (reused) |
| Tree attachment | `NodeAttrs` `renderable.layout` | `NodeAttrs` `renderable.style` |
| Targets | Terminal + Browser; Markdown ignores | Terminal first; Browser later; Markdown ignores |
| Status | implemented | this draft |
