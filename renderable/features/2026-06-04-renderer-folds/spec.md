---
status: ready for planning
date: 2026-06-04
owner: ken
parent: renderable/features/2026-06-04-css-box-architecture/spec.md
depends-on: renderable/features/2026-06-04-tree-attrs/spec.md
---

# Renderer Folds — Render the CSS Box Model on Terminal and Browser

The third chapter of the [CSS Box Architecture](../2026-06-04-css-box-architecture/spec.md).
The [style-vocabulary](../2026-06-04-style-vocabulary/spec.md) defined the CSS
box (`margin`/`padding`/`width`/`max_width`/`alignment`/`background`/`border`)
and [tree-attrs](../2026-06-04-tree-attrs/spec.md) made nodes carry it cheaply.
This spec makes the **terminal** and **browser** folds actually render it — the
faithful terminal mapping is the hard part the program exists to get right —
and closes the renderable lowering gaps (`padding`, `width`, `border`) so
*darkmatter-cutover* can delete its bespoke per-component CSS.

## Background

### What the folds render today

**Terminal** (`biscuit-terminal/lib/src/render_tree/render.rs` + `style.rs`):

- `render_with_layout` owns *margin* (left/right shrink the inner width, the
  left margin + an alignment offset prefix each line, top/bottom emit blank
  rows) and the `max_width` cap.
- `apply_style` / `paint_text` owns *paint* — `color`/`background`/`emphasis`
  wrap each line, then `render_border` wraps the result. `paint_text` already
  measures the widest visible line and pads lines out to a band width.
- Neither renders `padding` or the `FitContent` / `Fixed` width modes.

**Browser** (`renderable/src/tree/render/browser.rs`):

- `layout_to_css` lowers `margin` (vertical → `lh`), `max_width`, and
  `alignment` (→ `margin-left/right: auto`). It does **not** emit `padding` or
  a `width` mode.
- `style_css_declarations` lowers `color`/`background`/`emphasis` but
  **explicitly ignores `border`** (line ~2289, "intentionally ignored until the
  broader Browser …").

**Markdown / MarkdownPlus:** ignore geometry and paint by design (portable
CommonMark). Unchanged here.

### The key reuse

The CSS box layers as `margin → border → padding → content`. The machinery for
"pad content out to a box width and paint it" is exactly the band-pad logic
`paint_text` still has from the now-deleted `Fill`. `padding` revives it, driven
by `Layout.padding` + `Style.background` instead of `Fill`.

## Goals

- **Terminal:** render `padding` (reserved inner cells painted by
  `background`), the `width` modes (`Auto` / `FitContent` / `Fixed`) under the
  `max_width` cap, and keep `margin` transparent — composing the full CSS box.
- **Browser:** lower `padding` and `width` in `layout_to_css`, and the **full
  `Border` matrix** in `style_css_declarations`; rely on CSS to paint the
  padding box from `background`.
- Read layout/style through tree-attrs' borrowed `*_ref` accessors so the folds
  stay within the performance gate (no per-node clone/JSON).
- Provide the renderable per-node lowering that lets *darkmatter-cutover* delete
  `build_component_css`.

## Non-Goals (deferred / out of scope)

- Deleting `build_component_css` or rewiring darkmatter `style:` → attrs
  (→ *darkmatter-cutover*). This spec only provides the renderable lowering.
- Turning Markdown / MarkdownPlus into a box-model renderer. They stay portable;
  MarkdownPlus is not made a second browser renderer (architecture spec).
- Any change to the `Layout` / `Style` field set (owned by style-vocabulary) or
  to attr storage (owned by tree-attrs).
- Vertical-percentage padding fidelity on the terminal (the vocabulary spec
  already permits target-specific degradation of fractional rows).

## The design

### 1. Terminal — layered split (geometry outside, painted box inside)

**`render.rs` (geometry, transparent outer):**

1. Resolve the content-box width from `Layout.width` under the cap:
   - `Auto` → `available − margin_lr`
   - `Fixed(n)` → resolve `n` (cells / `%` of available)
   - `FitContent` → render the content unconstrained, take its **max-content**
     widest visible line, use that
   - then `used = min(used, max_width?, available − margin_lr − padding_lr − border_lr)`.
   Content-box sizing: `used` is the *content* width; the painted box adds
   `padding` then `border` around it. Clamp so
   `margin + border + padding + used ≤ available`, shrinking `used` first
   (min 1).
2. Apply `margin` (transparent: left/right prefix/suffix as spaces with no SGR,
   top/bottom blank rows) and the **alignment offset** that places the painted
   box within `available − margin` when `used + padding + border < that`.
   `FitContent` is the render-then-measure-then-place case: content is rendered,
   measured, sized, then placed.

**`paint_text` / `style.rs` (painted inner box):**

3. Pad the content by `Layout.padding` cells (left/right columns, top/bottom
   rows) and paint content + padding with `Style.background` — the padding cells
   carry the background SGR, the margin cells do not. This reuses the existing
   widest-line/band-pad code, now parameterized by `padding` + `used` width
   instead of `Fill`.
4. `render_border` wraps the padded, painted content (unchanged nesting; border
   sits between padding and margin and already reserves its cells via
   `border_horizontal_overhead`).

Inline `Span` nodes carry no `Layout`, so they get no padding/width; an inline
`background` paints the inline run only (unchanged).

### 2. Browser — close the lowering gaps

**`layout_to_css`** additionally emits:

- `padding`: `padding-top/right/bottom/left` from `Layout.padding`
  (vertical → `lh` like margin; horizontal → `ch`/`%`/native).
- `width`: `Auto` → omit (block default); `FitContent` → `width: fit-content`;
  `Fixed(n)` → `width: {n}ch` / `{n}%` / native.

Background painting the padding box needs no code: CSS `background` paints the
padding box automatically once `padding` is present, matching the terminal.

**`style_css_declarations`** lowers the full `Border` matrix (no longer
ignored):

- `weight` → `border-width` (`Thin`/`Medium`/`Thick` → px steps),
- `line_style` → `border-style` (`solid` / `dashed` / `dotted` / `double`),
- `color` → `border-color` (via the existing `PerMode` → CSS color path),
- `sides` → `border` for `All`, per-side `border-{top,right,bottom,left}` for
  `Sides { … }`, omitted for `None`,
- `radius` → `border-radius`.

### 3. Performance

Both folds read `Layout` / `Style` through the tree-attrs borrowed accessors
(`layout_ref` / `style_ref`), so they perform no per-node clone or JSON
round-trip and the tree-attrs structural perf gate stays green.

## Acceptance Criteria

1. **Terminal padding** is reserved and painted: a block with
   `padding: Edges::x(2)` + `Style.background` renders two background-painted
   columns on each side of the content, inside any border, outside no margin.
   Unit + a real-terminal (L2) check.
2. **Terminal `width` modes:** `Fixed(n)` renders the content box at `n`
   columns; `FitContent` sizes the box to the content's widest line (matching
   the former `Fill::Padded` / `Indented` band sizing as a *reference*); both
   honor `max_width` and the box-order clamp; `Auto` is unchanged from today.
3. **Terminal alignment** places a sub-`available` painted box (center/right
   offset) for all three width modes.
4. **Browser `layout_to_css`** emits `padding-*` and the correct `width`
   declaration for each `Width` variant; existing `margin`/`max_width`/alignment
   output is unchanged.
5. **Browser `border`** lowers the full matrix (weight/line-style/color/per-side
   sides/radius); `style_css_declarations` no longer skips `border`.
6. **Performance gate green:** the tree-attrs fold-invariant test still asserts
   zero renderable-owned hint round-trips; the folds use borrowed accessors.
7. **No darkmatter change here:** `build_component_css` still exists and still
   works (its deletion is *darkmatter-cutover*); the markdown corpus output is
   unchanged.
8. Local renderable / biscuit-terminal skill + docs describe terminal padding &
   width rendering and browser `padding`/`width`/`border` lowering; the
   "border/fill not lowered to Browser" note in `layout-and-style.md` is updated
   (border is now lowered; fill no longer exists).

## Risks

- **Terminal box-order correctness.** Margin (transparent) vs padding (painted)
  vs border (cells) plus the content-box clamp is the subtle part. Mitigation:
  characterization tests pinning cell output for representative
  margin/padding/border/width combinations; parity is a reference, and any
  intended diff vs the former `Fill` output is documented.
- **`fit-content` measurement.** Measuring max-content requires an unconstrained
  render pass; getting the cap/clamp order wrong yields wrong widths.
  Mitigation: a dedicated test matrix over `FitContent` × `max_width` × narrow
  `available`, asserted against hand-computed widths.
- **Browser/terminal divergence.** CSS and cells round/resolve differently
  (e.g. `lh` vertical, `fit-content`). Mitigation: do not force byte parity
  across targets — assert each target's own contract; the architecture spec
  treats parity as a reference.
- **Border CSS surface.** The full matrix is more lowering than browser did
  before. Mitigation: drive it off the typed `Border` fields with one test per
  axis (weight, line-style, per-side, radius, color).

## Related

- [`../2026-06-04-css-box-architecture/spec.md`](../2026-06-04-css-box-architecture/spec.md)
  — architecture overview and sequencing.
- [`../2026-06-04-style-vocabulary/spec.md`](../2026-06-04-style-vocabulary/spec.md)
  — the box-model types these folds render.
- [`../2026-06-04-tree-attrs/spec.md`](../2026-06-04-tree-attrs/spec.md) — the
  typed attrs + borrowed accessors the folds read, and the perf gate they honor.
- [`biscuit-terminal/lib/src/render_tree/render.rs`](../../../biscuit-terminal/lib/src/render_tree/render.rs)
  / [`style.rs`](../../../biscuit-terminal/lib/src/render_tree/style.rs) — the
  terminal geometry + paint split this spec extends.
- [`renderable/src/tree/render/browser.rs`](../../../renderable/src/tree/render/browser.rs)
  — `layout_to_css` / `style_css_declarations`, the browser lowering this spec
  completes.
