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

### `Edges`, `Width`, `Alignment`, `Layout`

`Edges` is a four-sided box (`top`/`right`/`bottom`/`left`), each side a
`TargetValue<Length>`; constructors `all` / `x` / `y`. It is used for **both**
`margin` and `padding` (renamed from the former `Margin` struct, which had no
`type Margin = Edges` alias left behind). `Width` is the content-box sizing
mode — `Auto` (default) / `FitContent` / `Fixed(TargetValue<Length>)`.
`Alignment` is `Left` (default) / `Center` / `Right`. `Layout` ties them into
the full CSS box model:

```rust
pub struct Layout {
    pub margin: Edges,                       // transparent outer space
    pub padding: Edges,                      // reserved inner space; PAINTED by Style.background
    pub width: Width,                        // content-box width mode
    pub max_width: Option<TargetValue<Length>>,  // orthogonal upper cap
    pub alignment: Alignment,
    pub word_wrap: WordWrap,
}
```

`Layout` describes a **block-level** component's CSS box only. Paint is
deliberately *not* here — color, `background`, and `border` are `Style`
concerns (see §6). The painted inner gutter that the deleted `Fill` once
expressed is now `padding` (geometry, here) + `Style.background` (paint). Total
horizontal occupancy is `margin + border + padding + used_width`, matching CSS
`box-sizing: content-box`; renderers must reserve the cells a drawn border
consumes.

`width` and `max_width` are orthogonal and compose (CSS `width` + `max-width`):
`Auto` + an 80ch cap, or `FitContent` + a 100ch cap, are both expressible — the
expressiveness gain over the deprecated flat `PageFill`, which could state only
one sizing fact at a time.

`Layout::default()` is zero margins, zero padding, `Width::Auto`,
`Alignment::Left`, no `max_width`, and `WordWrap::None`. The `Default` impl is
hand-written: `word_wrap` is explicitly `WordWrap::None`, **not**
`WordWrap::default()` (which is a wrapping policy). Deriving `Default` here was
the original implementation and caused a crate-wide regression — every `Prose`
silently began wrapping — so the hand-written impl is load-bearing, not
incidental. `Layout::default()` + `Style::default()` is bit-identical to a node
with no layout/style config; a node carrying *neither* attr renders identically
and skips the styling pass (absence is the cheap default).

### Serialization contract

`Layout`, `Edges`, and `Width` derive serde. `Width` is
`#[serde(rename_all = "snake_case")]` → `"auto"` / `"fit_content"` / `"fixed"`.
The new `padding` and `width` fields are `#[serde(default)]`, so an older
serialized tree carrying only `margin` / `alignment` / `max_width` /
`word_wrap` deserializes to `Layout { padding: Edges::default(), width:
Width::Auto, .. }`. The `Margin → Edges` change is an API rename, not a field
rename: the `margin` field stays named `margin`; the new field is `padding`.

### Validation — `LayoutError`

`Layout::validate()` (covering `margin`, `padding`, `width`, and `max_width`),
`Edges::validate()`, `Width::validate()`, and `TargetValue::validate()` return
the first `LayoutError`:

- `InvalidPercent` — a percentage outside `0.0..=100.0`, or non-finite.
- `NonUniversalUnit` — a `Length::Css` in a `Universal` branch.
- `EmptyPerTarget` — an empty `PerTarget` map.

Validation is **opt-in**. A caller constructing a `Layout` should call
`validate()`, but the render pipeline does not — once a `Layout` is on the
tree the renderers lower it as-is. See §5 for why this matters.

## 3. Layout on the render tree

`Layout` rides on a block `RenderNode` as the typed `NodeAttrs::layout` sparse
field (`Option<Box<Layout>>`), not as a serialized `data`-bag entry — so a
renderer reads it with no serde round-trip:

```rust
node.attrs.set_layout(&layout);
let recovered: Option<Layout> = node.attrs.layout();   // clone
let borrowed: Option<&Layout> = node.attrs.layout_ref(); // hot path, no clone
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
sides (`top`/`bottom`) lowered to `lh` units, `padding` to `padding-*` (same
unit rules as margin), `width` to a `width` declaration (`Auto` omits it,
`FitContent` → `width:fit-content`, `Fixed(tv)` → an explicit `width`),
`max_width` to `max-width`, and alignment to `auto` margins **only when a
`max_width` is present**. `word_wrap` becomes `white-space:nowrap` (`None`) or
`overflow-wrap:break-word` (any wrapping variant). It also emits
`box-sizing:content-box` on any node that lowers a non-default `width`,
`padding`, or `border`, so a page's global `* { box-sizing:border-box }` reset
cannot silently reinterpret the renderable content-box width contract.

**Terminal** (`biscuit-terminal`, `render_tree::render::render_with_layout` and
`LayoutTerminalExt`) resolves margins, `padding`, and the `width` modes to
whole cells against the available width via the shared `resolve_cells` helper
(`Ch(n)`→`n`, `Percent(p)`→`round(width*p/100)`, `Zero`/`Css`/absent→`0`,
resolving for `RenderTarget::Terminal`). It resolves the content-box width from
`layout.width` — `Auto` fills `available − margin − padding − border`,
`Fixed(tv)` resolves `tv` clamped to that cap, `FitContent` renders once at the
cap then re-renders at the measured widest line — narrows the child render
width accordingly, renders the content at exactly the content-box width, and
paints `padding` + `border` **around** it (a `Fixed(n)` box keeps all `n`
content columns; the border is never carved out of them). It then **block-aligns
the box within `available − margin`** for every width mode (`margin:auto`
semantics): the painted box is `content_width + padding + border`, and when it
is narrower than the area — a sub-available `Fixed` / `FitContent` box, or an
`Auto` box capped by `max_width` — the alignment offset positions the whole box
(center/right). A box that fills the area centers its visible content instead.
Top/bottom margins emit as blank rows. The `padding` box is painted by
`paint_text` with `Style.background`; the margin stays transparent. The legacy
`LayoutTerminalExt` retains `apply_layout` / `apply_block_layout` for the
bespoke (non-tree) component path.

> `max_width` caps the terminal content box just as it does in the browser, and
> the capped box is block-placed within `available − margin`. There is no
> separate terminal `max_width` rule beyond that cap-then-place — the property is
> not a terminal no-op.

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
- **Terminal `max_width` caps the content box and the capped box is
  block-placed** (see §4) — symmetric with the browser, not a no-op.
- **darkmatter's `LayoutContext` page-frame pass is retained.** `DarkmatterPage`
  now builds a `renderable::layout::Layout` (see §7) and the document body
  renders through the tree terminal renderer, but `apply_row_decoration` still
  runs as the page-frame **post-pass** that wraps the rendered body in page-level
  margins, padding, and background. This is a complementary step (page frame vs.
  body content), not a "not yet on the tree" fallback.
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
    pub color: Option<TargetValue<PerMode<PaintColor>>>,
    pub background: Option<TargetValue<PerMode<PaintColor>>>,  // paints content + padding box
    pub emphasis: TextEmphasis,
    pub border: Option<Border>,                               // Border.color is also PaintColor
}
```

- **`color` / `background`** — foreground and box background paint. They carry
  `PaintColor` (a `Color` plus an `Opacity` alpha byte), not bare `Color`, so
  alpha survives the tree without a side channel; `Border.color` is `PaintColor`
  too. `PerMode` accepts `impl Into<PaintColor>`, so opaque construction from a
  `Color` stays concise. The terminal target reads `PaintColor::color` and
  ignores the alpha at every color depth; the browser lowers the pair to
  `rgb(...)` / `rgba(...)` (or a `transparent` / `currentColor` / `inherit`
  keyword) through the shared `paint_to_css_color`. `Opacity` defaults to opaque
  and is elided from the serialized form when opaque, so an alpha-less tree
  serializes exactly as it did before alpha existed. Per CSS, `background` paints
  the content box *and* the `Layout.padding` box (out to the border edge), but
  not the margin.
- **`emphasis`** — the shared `TextEmphasis` leaf (bold, dim, italic,
  underline, strikethrough, blink, inverse), also reused by `Prose`.
- **`border`** — `Border { color, weight, line_style, sides, radius }`.

The former `fill` field and the whole fill abstraction (its intensity and band
knobs) are **deleted**. The only thing fill offered beyond the box model was the
implicit adaptive tint of its subtle / pronounced intensities; that survives as
the `Background` constructor namespace — `Background::subtle()` / `pronounced()`
return the `TargetValue<PerMode<Color>>` value `background` already holds
(`Background` is zero-sized and never stored). The painted bands fill drew are
now expressed structurally: a painted gutter is `Layout.padding` + `background`,
a band hugging the text is `Layout.width: FitContent` + `alignment` +
`background`. `Style::is_empty()` treats a `Style` carrying only
`Background::subtle()` / `pronounced()` as non-empty.

`PerMode<T>` (`Universal` / `Adaptive { light, dark }`) is the light/dark
adaptation wrapper, composed with `TargetValue` for color-bearing fields as
`TargetValue<PerMode<Color>>` — `TargetValue` selects per render target,
`PerMode` then adapts to light/dark within a target.

### Style on the render tree

`Style` rides on `NodeAttrs` as the typed `style` sparse field
(`Option<Box<Style>>`), with `set_style` / `style` / `style_ref` accessors
mirroring `set_layout` / `layout` / `layout_ref`. Unlike `Layout`, `Style` may
attach to block nodes **and** inline `Span` nodes.

Inheritance is **limited**: only the text-appearance fields — `color` and
`emphasis` — cascade through tree traversal (`Style::inherited_from`). The
box-painting fields — `background` and `border` — never inherit and stay
explicit on the node that paints them. Every render fold threads this push-down
through one shared resolver, `renderable::tree::InheritedStyle`: `enter` returns
both the effective `Style` for the current node and the child context to thread
into its descendants, so the inheritance rule lives in exactly one place rather
than being re-implemented per renderer.

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
> Terminal painting of the `padding` box, `Width` resolution (`Auto` /
> `Fixed` / `FitContent`), and `Background` tints landed in the *renderer-folds*
> sub-spec. `render_with_layout` resolves the content-box width and clamps it
> against `available − margin − padding − border`; `FitContent` measures the
> content's widest line then re-renders at that width. `paint_text` paints the
> `padding` box with `Style.background`, and the implicit one-cell interior
> border gap was removed so `Layout.padding` is the single source of inner
> spacing.

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

`border` is lowered to CSS by `style_css_declarations`: `weight` maps to a
`border-width` px step (`Thin`→1px, `Medium`→2px, `Thick`→3px), `line_style` to
the matching `border-style` keyword (`Solid`/`Dashed`/`Dotted`/`Double`),
`color` through the existing `PerMode`→CSS color path, and `radius` to
`border-radius`. `BorderSides::All` emits the `border-*` shorthands;
`BorderSides::Sides { .. }` emits per-side `border-{side}-{width,style,color}`
for each enabled edge; `BorderSides::None` emits nothing. The terminal target
also renders border glyphs.

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
bespoke `FileSystem` renderer — Stage 3 deferred that terminal flip
(Nerd Font icon parity; see [`components.md`](./components.md)). The
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

- **Browser `Style` lowering is complete.** Color, background, emphasis
  (the `<strong>` / `<em>` / `<s>` wrappers for inline, the equivalent
  CSS for block), underline variants, dim, and blink are wired, and the
  box-painting `border` layer (weight / line-style / radius / per-side matrix)
  now lowers to CSS via `style_css_declarations`. `padding` and `Width` (the
  box-model replacement for the deleted `Fill` band) lower to `padding-*` and
  `width` via `layout_to_css`.
- **Darkmatter `style:` frontmatter is a separate policy layer.** Its v1
  schema is now wired through sub-spec #7 (see §7), but it applies page and
  component policy to `DarkmatterPage`; it does not mean Markdown frontmatter
  is automatically converted into render-tree `Style` attributes.

## 7. darkmatter migration

`DarkmatterPage` keeps its public builder API (`with_margin`, `with_padding`,
`with_max_width`, alignment/fill setters) unchanged. Internally it builds a
`renderable::layout::Layout` from its page settings rather than doing bespoke
`PageMargin` arithmetic.

The darkmatter cutover is complete. The deprecated page-layout value types —
`PageMargin`, `PagePadding`, `PageFill`, `PageAlignment`, `WidthUnit`, and
`PageComponent::Lists` — and their conversion bridges have been **deleted**.
`DarkmatterPage` now stores `renderable::layout::Edges`,
`TargetValue<Length>`, and `ComponentPolicy` directly; `style:` frontmatter
lowers straight into the per-component `ComponentPolicy` — a
`renderable::layout::Layout` plus `color` / `bg_color` carried as alpha-bearing
`renderable::style::PaintColor`. The parsed `StyleColor` (which carries optional
Tailwind/hex opacity) is lowered to `PaintColor` at the parser/apply boundary,
so opacity rides in the paint's alpha channel rather than a side channel — there
is no `StyleColor` left on post-construction component types.

The render tree is built **complete** during construction: darkmatter's
context-aware fold (`render_tree::build_context`, a `TreeBuildContext`) bakes
component policy, page-inheriting color, alpha paint, text layout, and HR
defaults onto the nodes as it folds, and each target then runs **one fold** over
that tree. The old post-fold `decorate` pass and the `darkmatter.style` /
`darkmatter.li` render hints have been **deleted**; the browser fold lowers
alpha straight to `rgba(...)` with no post-render HTML rewrite. The render-tree
folds perform all width, padding, alignment, and CSS resolution. `DarkmatterPage`
survives as a slim, renderable-typed page frame.

### `style:` frontmatter status

Darkmatter's document-level `style:` frontmatter pipeline is now active through
sub-spec #7 of `renderable/features/2026-05-23-style-property/`:

- **#1 schema/parser** — parses sparse `style:` YAML into
  `darkmatter::style::StyleFrontmatter`, using `renderable::layout::Length`,
  `renderable::layout::Alignment`, and `renderable::color::Color`-backed
  values rather than darkmatter-local duplicates. Canonical keys are
  kebab-case; snake-case aliases parse with deprecation warnings.
- **#2 page wiring** — applies `style.page.*` onto `DarkmatterPage`
  (`margin`, `padding`, `max-width`, `alignment`, `background`) after CLI
  layout flags. CLI flags win field-by-field. `md --strict-style` promotes
  unknown and deprecated schema warnings to errors.
- **#3 component wiring** — applies `style.table.*`, `style.images.*`, and
  `style.block-quote.*` alignment and width/fill settings.
- **#4 list wiring** — splits list targets into `ul`, `ol`, and `li`; applies
  list alignment/fill and `style.ul.left-margin`.
- **#5 color wiring** — applies `color` / `bg-color` at page and wired
  component scopes. Page colors are inherited defaults; component colors
  override them.
- **#6 HR migration** — makes `style.hr.*` the canonical horizontal-rule
  styling namespace, keeps top-level `hr:` and inline `{ style: ... }` as
  deprecated aliases, and wires HR kind/weight/alignment/width/color settings.
- **#7 bespoke knobs** — wires `style.page.stylesheet`,
  `style.page.meta`, `style.page.code.theme`, `style.hyperlinks.*`,
  `style.hyperlinks.local-style.*`, and `style.images.local-style.*`.
  Local stylesheets are inlined for HTML; remote stylesheets are emitted as
  links and are not fetched by the renderer.

The active wiring phase is recorded in
`darkmatter::style::parse::ACTIVE_STYLE_WIRING_SUB_SPEC` and is currently `7`.
No valid v1 schema keys should emit `KnownButInactive`; unsupported or
ambiguous v1 combinations are rejected with documented `StyleApplyError`
variants instead.

This frontmatter pipeline is adjacent to, but not the same thing as,
`renderable::style::Style`. The frontmatter applicator writes into
`DarkmatterPage`'s page/component layout, color, HR, stylesheet, metadata,
code-theme, hyperlink, and image-style state; the render-tree `Style`
primitive remains the target-agnostic appearance value carried by `RenderNode`
attributes.

## 8. See also

- `renderable/docs/tree-rendering.md` — the render-tree architecture `Layout`
  rides on.
- `darkmatter/docs/rendering/style.md` — user-facing `style:` frontmatter
  contract and examples.
- `.claude/skills/renderable/layout.md` — the `renderable::layout` API.
- `.claude/skills/renderable/style.md` — the `renderable::style` API.
- `.claude/skills/biscuit-terminal/render-tree.md` — terminal layout
  application.
- `renderable/features/2026-04-17-layout-and-style/` — the feature spec and
  implementation plan.
- `renderable/features/2026-05-23-style-property/` — darkmatter `style:`
  frontmatter sub-specs.
