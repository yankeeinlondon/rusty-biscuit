# Layout and Style

This document describes the **layout and style primitives** introduced into
the `renderable`, `biscuit-terminal`, and `darkmatter` crates: two
target-agnostic, sibling types — `Layout` (where a block sits) and `Style`
(what a box looks like) — that components declare and the tree renderers apply
across Terminal, Browser, and Markdown.

It is a status-and-direction document. Like `tree-rendering.md`, it is
deliberately honest about what is *proven* versus what is only *wired up*.

The feature is specified across two documents in
`renderable/features/2026-04-17-layout-and-style/`: `layout-spec.md` (Spec A —
`Layout`) and `style-spec.md` (Spec B — `Style`).

## 1. The problem this replaced

Before this work, three crates each carried their own forked notion of layout:

- `renderable::layout::Layout` — a legacy struct with flat margin fields, a
  `Margin` enum (`Chars` / `Percent` / `Offset` / `None`), `MaxWidth`, and
  `RowFill`.
- `renderable::tree::LayoutHints` — a parallel, render-tree-only struct.
- `darkmatter`'s `DarkmatterPage` page types (`PageMargin`, `PagePadding`,
  `PageFill`, `PageAlignment`) — a third bespoke representation.

A component declaring "two cells of left margin, centered" had to say it three
different ways, and the three renderers each interpreted layout differently.
The legacy `Layout`, `LayoutHints`, `MaxWidth`, `RowFill`, and the old `Margin`
enum are now **removed**. There is one `Layout`.

## 2. The layout primitive — `renderable::layout`

`renderable` owns the layout model so every other crate depends on it without a
dependency cycle.

### Units — `Length`

```rust
Length::Zero            // unit-independent zero
Length::Ch(u32)         // whole cells: columns horizontally, rows vertically
Length::Percent(f32)    // 0.0..=100.0 of available width
Length::Css(CssSizing)  // a target-native CSS length
```

`Zero`, `Ch`, and `Percent` are **universal units** — meaningful on every
render target. `Css` is target-native and is only valid inside a per-target
branch of a `TargetValue`. `Length::percent()` is a checked constructor
returning `Result`; `is_universal()` reports whether a value is target-portable.

### Per-target values — `TargetValue<T>`

```rust
TargetValue::Universal(T)                       // one value for every target
TargetValue::PerTarget(BTreeMap<RenderTarget, T>) // non-empty; native units OK
```

`resolve(target)` looks the value up. `Universal` always resolves. `PerTarget`
returns the named entry, and a `MarkdownPlus` lookup **falls back to the
`Markdown` entry** — the one place the four `RenderTarget` variants
(`Markdown`, `MarkdownPlus`, `Browser`, `Terminal`) are not independent.

### `Margin`, `Alignment`, `Layout`

`Margin` is a four-sided box (`top`/`right`/`bottom`/`left`), each side a
`TargetValue<Length>`; constructors `all` / `x` / `y`. `Alignment` is
`Left` (default) / `Center` / `Right`. `Layout` ties them together:

```rust
pub struct Layout {
    pub margin: Margin,
    pub alignment: Alignment,
    pub max_width: Option<TargetValue<Length>>,
    pub word_wrap: WordWrap,
}
```

`Layout` describes a **block-level** component's relationship to its parent
only. Appearance (background, fill, color) is deliberately *not* here — that is
a `Style` concern (see §6).

`Layout::default()` is zero margins, `Alignment::Left`, no `max_width`, and
`WordWrap::None`. The `Default` impl is hand-written: `word_wrap` is explicitly
`WordWrap::None`, **not** `WordWrap::default()` (which is a wrapping policy).
Deriving `Default` here was the original implementation and caused a
crate-wide regression — every `Prose` silently began wrapping — so the
hand-written impl is load-bearing, not incidental.

### Validation — `LayoutError`

`Layout::validate()`, `Margin::validate()`, and `TargetValue::validate()`
return the first `LayoutError`:

- `InvalidPercent` — a percentage outside `0.0..=100.0`, or non-finite.
- `NonUniversalUnit` — a `Length::Css` in a `Universal` branch.
- `EmptyPerTarget` — an empty `PerTarget` map.

Validation is **opt-in**. A caller constructing a `Layout` should call
`validate()`, but the render pipeline does not — once a `Layout` is on the
tree the renderers lower it as-is. See §5 for why this matters.

## 3. Layout on the render tree

`Layout` rides on a block `RenderNode` via `NodeAttrs`, serialized as JSON
under the layout hint namespace:

```rust
node.attrs.set_layout(&layout);
let recovered: Option<Layout> = node.attrs.layout();
```

`TreeRenderable::tree_layout(&self) -> Option<Layout>` is the optional hook a
component implements to supply layout; components seed it onto their projected
root node during tree projection.

Tree validation (`renderable::tree::validate`) enforces **block-only**
placement: a `Layout` on an inline node (`Text`, `Emphasis`, `Link`, …) is a
validation error — *"layout attributes are permitted only on block-level
nodes."* This rule **is** reachable: both the Browser and Markdown renderers
run `validate()` in their gates.

## 4. Per-target consumption

Each renderer consumes `Layout` on its own terms.

**Browser** (`renderable/src/tree/render/browser.rs`, `layout_to_css`) lowers a
node's `Layout` to an inline `style` attribute: margins to `margin-*`, vertical
sides (`top`/`bottom`) lowered to `lh` units, `max_width` to `max-width`, and
alignment to `auto` margins **only when a `max_width` is present**. `word_wrap`
becomes `white-space:nowrap` (`None`) or `overflow-wrap:break-word` (any
wrapping variant).

**Terminal** (`biscuit-terminal`, `render_tree::render::render_with_layout` and
`LayoutTerminalExt`) resolves margins to whole cells against the available
width via the shared `resolve_cells` helper (`Ch(n)`→`n`, `Percent(p)`→
`round(width*p/100)`, `Zero`/`Css`/absent→`0`, resolving for
`RenderTarget::Terminal`). It narrows the child render width by left+right
margins, prefixes each line, block-aligns the component as a unit, and emits
top/bottom margins as blank rows. The legacy `LayoutTerminalExt` retains
`apply_layout` / `apply_block_layout` for the bespoke (non-tree) component path.

> The terminal renderer **does not apply `max_width`** — it is a Browser-only
> property. `max_width` only influences the terminal path indirectly, by being
> the precondition for browser block-alignment.

**Markdown** deliberately **ignores** `Layout` entirely. Markdown body output
is byte-identical whether or not a node carries a layout, and no diagnostic is
emitted — this is locked by a regression test.

## 5. Proven vs. wired — honest gaps

What is proven:

- The `renderable::layout` types — units, per-target resolution, the
  `MarkdownPlus`→`Markdown` fallback, serde round-trips, validation — are unit-
  tested.
- All three renderers' consumption is tested: Browser CSS lowering (including
  vertical `lh`), terminal cell margins and percentage resolution, the
  Markdown-unchanged lock.
- Thirteen components emit `Layout` — the twelve `biscuit-terminal`
  components flipped to the render tree in Stage 2 (`BlockQuote`,
  `Compose`, `FileSystem`, `OrderedList`, `UnorderedList`, `Progress`,
  `Section`, `StatusBlock`, `Table`, `TextBlock`, `Todo`, `TwoColumn`)
  plus darkmatter's `YamlBlock`. Their tree output is snapshot-tested in
  `layout_matrix` and parity-checked against the bespoke renderers in
  `render_comparison`. See
  [`renderable/features/2026-05-19-pushing-toward-ir/lessons-learned.md`](../features/2026-05-19-pushing-toward-ir/lessons-learned.md)
  for the per-component migration notes.

Known gaps and loose ends:

- **Validators are not invoked by the render pipeline.** `Layout::validate()`
  et al. are tested in isolation but no renderer, and not `set_layout`, calls
  them. An out-of-range `Percent` or a `Css` unit in a `Universal` branch
  survives onto the tree and is lowered as-is. `Layout` is, in practice, a
  trusted structure once constructed. Wiring `validate()` into the tree
  `validate()` gate is a recommended follow-up.
- **`TerminalRenderContext::active_layout` is a dead field** — set by
  `with_layout`, never read. The terminal renderer reads `node.attrs.layout()`
  directly. The field is retained for API shape; remove it or wire a consumer.
- **Terminal `max_width` is a silent no-op** (see §4) — consistent with the
  spec, but a Browser/Terminal asymmetry a reader would not expect.
- **darkmatter's legacy `LayoutContext` pipeline is retained.** `DarkmatterPage`
  now builds a `renderable::layout::Layout` (see §7), but the bespoke
  `apply_row_decoration` `u16` margin pipeline still runs for paths not yet on
  the tree renderer. It is correct but duplicative — a transitional state.
- **Drift is recorded, not eliminated.** The `render_comparison` `KNOWN_DRIFT`
  ledgers carry the tree-vs-bespoke divergences. Some entries are the tree path
  being *more* correct than the legacy bespoke renderer (e.g. applying vertical
  margins the bespoke path never honored) — those are deliberately left, not
  "fixed" by regressing the tree path.

## 6. The style primitive — `renderable::style`

The feature is named "layout **and style**." Both halves are now built.
`Style` is the appearance sibling of `Layout`: `Layout` decides *where the box
sits*, `Style` decides *what the box looks like*. A component declares a
`Style`; the tree renderers apply it. A component never hand-writes ANSI.

### The `Style` struct

```rust
pub struct Style {
    pub color: Option<TargetValue<PerMode<Color>>>,
    pub background: Option<TargetValue<PerMode<Color>>>,
    pub emphasis: TextEmphasis,
    pub border: Option<Border>,
    pub fill: Option<Fill>,
}
```

- **`color` / `background`** — foreground and box background color.
- **`emphasis`** — the shared `TextEmphasis` leaf (bold, dim, italic,
  underline, strikethrough, blink), also reused by `Prose`.
- **`border`** — `Border { color, weight, line_style, sides, radius }`.
- **`fill`** — `Fill { color, intensity, band, inset }`: painted-band
  behavior, deliberately distinct from `background`.

`PerMode<T>` (`Universal` / `Adaptive { light, dark }`) is the light/dark
adaptation wrapper, composed with `TargetValue` for color-bearing fields as
`TargetValue<PerMode<Color>>` — `TargetValue` selects per render target,
`PerMode` then adapts to light/dark within a target.

### Style on the render tree

`Style` rides on `NodeAttrs` under the `renderable.style` hint namespace, with
`set_style` / `style` accessors mirroring `set_layout` / `layout`. Unlike
`Layout`, `Style` may attach to block nodes **and** inline `Span` nodes.

Inheritance is **limited**: only the text-appearance fields — `color` and
`emphasis` — cascade through tree traversal (`Style::inherited_from`). The
box-painting fields — `background`, `border`, `fill` — never inherit and stay
explicit on the node that paints them.

### Per-target consumption

**Terminal** (`biscuit-terminal`, `render_tree::style::apply_style`) lowers
`Style` during the fold, the same step `Layout` is applied:

- `color` / `background` degrade to the terminal's `ColorDepth`
  (truecolor → 256 → 16) and emit foreground / background SGR; `PerMode`
  selects the light- or dark-mode value.
- `emphasis` lowers to SGR; an unsupported underline variant degrades against
  the terminal's reported underline support.
- `border` draws box-drawing glyphs — thin / heavy / double weight, solid /
  dashed / dotted line style, per-side selection, and **rounded corners** via
  `Border::radius` (any non-zero radius selects the light-arc corner set; a
  heavy or double border has no arc variant and keeps square corners).
- `fill` paints a background band: `FillBand::Full` (the available width),
  `Padded` (the content band), or `Indented` (inset from both edges).
  `Fill::inset` adds leading unpainted columns and narrows the band.

> **Code blocks are outside this `Style` primitive.** Syntax-highlighted code
> panels are driven by darkmatter's `ThemePair` + `ColorMode`, not `Style`. As a
> terminal-specific concern, darkmatter resolves the *code* theme against the
> **inverted** color mode for page contrast (light code on a dark terminal); the
> Browser path does not invert. See darkmatter's
> [Code Highlighting](../../darkmatter/docs/rendering/code-highlighting.md).

**Browser** (`renderable/src/tree/render/browser.rs`, `style_css_declarations`
plus `wrap_style_emphasis`) lowers `Style`'s text-appearance and box-color
layers to CSS during fragment emission:

- `color` and `background` resolve through `TargetValue` and `PerMode` and
  emit `color:` / `background-color:` declarations on the node's inline
  `style` attribute.
- `emphasis` lowers in two paths. The strikethrough / italic / bold trio
  on inline nodes lowers to semantic HTML wrappers (`<s>`, `<em>`,
  `<strong>`); on block nodes it lowers to CSS (`text-decoration:line-through`,
  `font-style:italic`, `font-weight:bold`) because nesting block content
  inside `<strong>` is invalid HTML.
- `underline` variants, `dim`, and `blink` always lower to CSS:
  `text-decoration:underline` (with `wavy` / `double` / `dotted` style
  selectors and the per-mode color), `opacity:0.6`, and a small CSS
  keyframe animation respectively.

`border` and `fill` lowering to CSS is **not yet wired** — the helper
explicitly leaves those two layers for a follow-up. The terminal target
remains the only consumer of border glyphs and fill bands today.

**Markdown** ignores `Style` entirely — Markdown body output is byte-identical
whether or not a node carries a style, with no diagnostic.

### Migrated components

The bespoke appearance fields scattered across components were re-homed onto
`Style` or typed component style structs (Spec B D5):

- **`BlockQuote`** — `text_color` / `bg_color` / `left_block_color` → a
  declared `Style` (text color, background, left `Border`).
- **`Section`** — hard-coded heading SGR → declared emphasis.
- **`Table`** — alternating-row flags → the typed `TableStyle` with
  `stripe_bg` / `stripe_text` `Color` slots.
- **`Progress`** — glyph `char` fields → the typed `ProgressStyle`, which also
  carries `filled_color` / `empty_color` / `bracket_color` slots.

Old component builders remain as deprecated compatibility shims that write to
the declared style.

### CLI surface

A dozen `bt` subcommands now exercise the terminal tree renderer
(`render_terminal_node`) end-to-end. Two — `bt block` and `bt quote` —
call the renderer directly to demonstrate generic `Style` on a text block
and `BlockQuote` projection. The rest reach it transitively: `bt list`,
`bt progress`, `bt table`, `bt compose`, `bt section`, `bt status-block`,
`bt text-block`, `bt todo`, `bt columns`, and (for the Markdown and
HTML output modes only) `bt dir` all invoke their component's
`render` / `render_markdown` / `render_html_fragment`, which the Stage 2
flip routed through the tree. `bt dir` terminal default remains the
bespoke `FileSystem` renderer pending the Stage 3 decision. The
Level-2 real-terminal tests in
`biscuit-terminal/cli/tests/level2_render_tree_style.rs` continue to
back `bt block`, `bt progress`, and `bt table` specifically for
`Style` coverage.

### Proven vs. wired — honest gaps

Proven: the `Style` types and serde round-trips; terminal lowering of every
layer — color-depth degradation, light/dark adaptation, emphasis, border
glyphs (including rounded corners), and fill bands with inset — is unit-tested
at Level 1 and captured through real terminals (WezTerm / Kitty / tmux) at
Level 2. Browser lowering of the text-appearance and box-color layers
(color, background, emphasis wrappers and CSS, underline variants, dim,
blink) is unit-tested in `tree/render/browser.rs`'s test module.

Known gaps:

- **Browser `Style` lowering is partial.** Color, background, emphasis
  (the `<strong>` / `<em>` / `<s>` wrappers for inline, the equivalent
  CSS for block), underline variants, dim, and blink are wired. The
  box-painting layers — `border` and `fill` — are intentionally **not**
  lowered yet (`style_css_declarations` documents the gap explicitly).
  Adding them requires defining the CSS semantics for the typed
  `Border` weight / line-style / radius matrix and the `Fill` band /
  inset model.
- **`render_border` width mismatch.** The terminal border's top/bottom rule is
  two columns narrower than the content row — the interior padding spaces are
  not counted in the rule width. It affects square and rounded borders alike
  and predates the `Style` migration; tracked for a separate fix.
- **The `style` frontmatter namespace and `hr_css_variables` retirement** are
  deferred to a follow-on sub-spec (Spec B D9), as is full darkmatter
  page-style migration.

## 7. darkmatter migration

`DarkmatterPage` keeps its public builder API (`with_margin`, `with_padding`,
`with_max_width`, alignment/fill setters) unchanged. Internally it now builds a
`renderable::layout::Layout` from its page settings rather than doing bespoke
`PageMargin` arithmetic.

The page-layout value types — `PageMargin`, `PagePadding`, `PageFill`,
`PageAlignment` — are now `#[deprecated]` in favor of `renderable::layout`.
They remain `pub` (the darkmatter CLI builds them from flags) and carry
conversion bridges:

- `From<PageMargin>` / `From<PagePadding>` → `renderable::layout::Margin`
- `From<PageAlignment>` → `renderable::layout::Alignment`
- `TryFrom<WidthUnit>` → `Length`, and `TryFrom<PageFill>` →
  `Option<TargetValue<Length>>` for the width-cap meaning, with a separate
  `PageFill::margin_contribution()` for the inset meaning.

## 8. See also

- `renderable/docs/tree-rendering.md` — the render-tree architecture `Layout`
  rides on.
- `.claude/skills/renderable/layout.md` — the `renderable::layout` API.
- `.claude/skills/renderable/style.md` — the `renderable::style` API.
- `.claude/skills/biscuit-terminal/render-tree.md` — terminal layout
  application.
- `renderable/features/2026-04-17-layout-and-style/` — the feature spec and
  implementation plan.
