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

`Layout` is non-semantic plain data. Whether `Style` gets a **semantic tier**
(named intents like "callout" / "hero" / "de-emphasized" that resolve to
concrete appearance) is **(OPEN)** — see Open Questions.

### D2 — Appearance vocabulary

The properties `Style` should express, drawn from the bespoke material above:

| Property | Type sketch | Re-homes |
|---|---|---|
| foreground color | `Option<TargetValue<Color>>` | `BlockQuote.text_color`, section heading color |
| background color | `Option<TargetValue<Color>>` | `BlockQuote.bg_color`, `Table` row tint, `PageBackground` |
| text emphasis | shared `TextEmphasis` leaf (bold, italic, dim, underline, strikethrough, blink) | `Section` heading bold/italic |
| border | `Option<Border>` (color, and possibly weight/style) | `BlockQuote.left_block_color` |
| fill / background band | `Option<Fill>` — paint a width band behind the block | `PageBackground` intensity, `RowFill` coloring |

**(OPEN)** Exact field set and whether `border`/`fill` are one concept or two.
`PageBackground`'s `Subtle` / `Pronounced` are *mode-relative* (defined against
the terminal's light/dark mode) — `Style` color values may therefore need a
mode-relative form, not just absolute `Color`s. See Open Questions.

### D3 — Per-target values and capability degradation

- Per-target overrides reuse `TargetValue<T>` (e.g. `TargetValue<Color>` for a
  color that differs between Browser and Terminal). Universal is the common
  case; `PerTarget` is the escape hatch.
- **Capability degradation is part of the contract.** A `Color` is authored
  once; the terminal renderer degrades it to the terminal's `ColorDepth`
  (truecolor → 256 → 16) using `renderable::color`'s existing machinery. The
  component never branches on color depth.
- **(OPEN)** Light/dark adaptation. `Table` already adapts its tints to
  `ColorMode`; `PageBackground::Subtle` is mode-relative by definition. Options:
  (a) a mode-relative color variant (`Color::adaptive(light, dark)`), (b) a
  `PerMode` wrapper analogous to `TargetValue`, (c) leave adaptation to the
  component. Needs a decision.

### D4 — The `Style` struct (DRAFT sketch)

```rust
/// How a block-level component paints itself. Sibling of `Layout`.
pub struct Style {
    /// Foreground (text) color.
    pub color: Option<TargetValue<Color>>,
    /// Background color of the component's box.
    pub background: Option<TargetValue<Color>>,
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

/// Shared appearance leaf, defined in `renderable::style` and reused by both
/// `Style` and the `Prose` IR's `ProseStyle`. All-default is no emphasis.
pub struct TextEmphasis {
    pub bold: bool,
    pub italic: bool,
    pub dim: bool,
    /// `None` / `Single` / `Double`. The terminal emitter degrades `Double`
    /// per terminal capability.
    pub underline: Underline,
    pub strikethrough: bool,
    pub blink: bool,
}
```

`Border` and `Fill` are **(OPEN)** — sketches only. `Border` likely carries a
`color` and possibly a `weight` (thin/medium/thick, mirroring darkmatter's
`HorizontalRule` weights). `Fill` likely carries a background `Color` and an
intensity, re-homing `PageBackground`.

This is a plain serde data struct, like `Layout`.

### D5 — The slot system — styling named sub-parts **(OPEN — the hard part)**

A single `Style` per component is not enough. A `Table` has rows, columns,
cells, a header, borders; a `Section` has a heading and a body; a `BlockQuote`
has a bar and content; a `Progress` has a filled track, an empty track, and
brackets. Spec A D1 explicitly says `Style` covers *"named sub-parts."*

Candidate approaches (to be decided):

- **(a) Slot map on the node** — the component attaches a
  `BTreeMap<SlotName, Style>` where `SlotName` is a component-defined string
  (`"heading"`, `"row.odd"`, `"cell"`, `"bar.filled"`). One `renderable.style`
  hint namespace, keyed by slot.
- **(b) Per-slot hint sub-namespaces** — `renderable.style.table.row`,
  `renderable.style.table.cell`, mirroring how `renderable.widget.progress`
  already namespaces widget hints.
- **(c) Typed per-component style structs** — `TableStyle { header, row_odd,
  row_even, cell, border }`, each field a `Style`. Type-safe; not generic.

Trade-off: (a)/(b) are uniform and serialize cleanly but stringly-typed;
(c) is type-safe but every component needs its own struct. A hybrid — generic
`Style` for the common case, typed component structs where slots are rich
(`Table`) — is plausible. **Needs a decision before planning.**

### D6 — `Style` on the render tree

- `Style` (and any slot styles) ride on `NodeAttrs` under a new
  `HintNamespace::STYLE` (`renderable.style`), with typed accessors
  `set_style` / `style`, exactly mirroring `set_layout` / `layout`.
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
- **Composition / inheritance — (OPEN).** `Layout` is non-inherited (Spec A
  D6). CSS `color` *does* inherit. A non-inherited `Style` is simpler and
  matches `Layout`; an inherited `color`/`emphasis` is more ergonomic (set the
  accent on a `Section`, children pick it up) but complicates every renderer.
  Recommendation leans **non-inherited for v1** (uniform with `Layout`, no
  cascade engine), revisited if authoring proves painful — but this is open.

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

`Style` rides on `NodeAttrs` and serializes with the tree. `Style`, `Emphasis`,
`Border`, `Fill`, and any slot keying derive `Serialize` / `Deserialize` with
`snake_case` enum casing. A documented JSON sample must appear in the `Style`
rustdoc with a round-trip test, exactly as Spec A D8 requires for `Layout`.

### D9 — Migration

- **Re-home evicted appearance.** `PageBackground` → `Style` background/fill;
  the *coloring* half of `RowFill` → `Style` (its *width* half already went to
  `Layout` in Spec A D9). `darkmatter`'s `PageBackground` becomes a
  `#[deprecated]` shim converting to `Style`, mirroring the `PageMargin` →
  `Margin` bridge Spec A established.
- **Migrate bespoke component fields.** `BlockQuote.text_color` /
  `bg_color` / `left_block_color`, `Section`'s heading SGR, `Table`'s
  alternating-color flags, `Progress`'s glyphs become a declared `Style` (or
  per-slot styles per D5). The bespoke `TerminalRenderable` renderers keep
  working during migration — not a big-bang cutover (Spec A D9 pattern).
- **`hr_css_variables`** — retire the ad-hoc `:root` CSS map in favor of `Style`
  + the `style` frontmatter schema. This is the most entangled piece and is
  **deferred** to the frontmatter sub-spec (see Non-Goals).

## Success Criteria (DRAFT)

- A single `Style` type exists in `renderable::style` (or
  `renderable::stylesheet`), declared by components, applied by the terminal
  tree renderer without the component hand-writing ANSI.
- The bespoke appearance fields on `BlockQuote`, `Section`, `Table`, `Progress`
  are expressed as `Style`; `darkmatter`'s `PageBackground` is a deprecated
  shim onto `Style`.
- A `Color` in a `Style` renders correctly on 16-color, 256-color, and
  truecolor terminals (capability degradation), and adapts to light/dark mode
  where required.
- The `Styling`-facet drift attributable to dropped appearance is burned down
  in `just drift-report`.
- `cargo test` / `cargo clippy` pass for `renderable`, `biscuit-terminal`,
  `darkmatter`.

### Required tests (DRAFT)

- Foreground / background color applied on each migrated component, terminal
  target.
- Emphasis (bold / italic / dim / underline / strikethrough) → correct SGR.
- Color degradation at each `ColorDepth`.
- Light/dark adaptation for a mode-relative value.
- Per-slot styling (once D5 is decided) — e.g. a styled table header vs body.
- Serde round-trip of the documented `Style` JSON shape.
- Markdown body unchanged when a node carries a `Style` (no diagnostic).

## Open Questions

These must be resolved before this draft becomes an implementation plan.

1. **Inline vs block (D6).** *Resolved.* `Style` applies to block nodes and
   inline `Span` nodes for *appearance*; semantic emphasis stays in the
   inline node kinds. `Prose` is decoupled — it has its own IR and shares only
   leaf primitives with `Style`. See D6.
2. **The slot system (D5).** Slot map, per-slot namespaces, or typed
   per-component style structs — or a hybrid?
3. **Inheritance (D6).** Non-inherited (uniform with `Layout`) or a `color`/
   `emphasis` cascade?
4. **Light/dark adaptation (D3).** Mode-relative color variant, a `PerMode`
   wrapper, or component-handled?
5. **Semantic tier (D1/D2).** Does `Style` get named intents ("callout",
   "hero"), or stay plain data like `Layout`?
6. **Module home.** *Resolved:* a new `renderable::style` module, the home for
   both the shared leaves (`TextEmphasis`, …) and the `Style` primitive — a
   neighbor of `renderable::color`. Still open: does `Style` *contain* a
   `CssStyle` for the browser path, or lower to one?
7. **Border / fill modeling (D2/D4).** One concept or two; what does `Border`
   carry beyond color (weight, line style)?
8. **`Style` ↔ `CodeRenderer`.** Confirmed separate — but a code block's
   *frame* (border/background) is `Style` while its *content* highlighting is
   `CodeRenderer`. Confirm the seam holds.
9. **Frontmatter & `hr_css_variables`.** Sequenced as a follow-on sub-spec —
   confirm it is not needed for the component-styling pass.

## Relationship to Spec A

| Aspect | Spec A — `Layout` | Spec B — `Style` |
|---|---|---|
| Concern | where the box sits | what the box looks like |
| Scope | block-level only | block **and** inline (OPEN) |
| Inheritance | non-inherited | OPEN |
| Per-target machinery | `TargetValue<T>` | `TargetValue<T>` (reused) |
| Tree attachment | `NodeAttrs` `renderable.layout` | `NodeAttrs` `renderable.style` |
| Targets | Terminal + Browser; Markdown ignores | Terminal first; Browser later; Markdown ignores |
| Status | implemented | this draft |
