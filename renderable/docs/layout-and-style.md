# Layout and Style

This document describes the **layout primitive** introduced into the
`renderable`, `biscuit-terminal`, and `darkmatter` crates: one target-agnostic
`Layout` type that every block-level component declares and the tree renderers
apply across Terminal, Browser, and Markdown.

It is a status-and-direction document. Like `tree-rendering.md`, it is
deliberately honest about what is *proven* versus what is only *wired up*, and
about what the feature's name promises but does not yet deliver — the **style**
half (Spec B) is not implemented.

The feature spec is `renderable/features/2026-04-17-layout-and-style/spec.md`.

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
- Seven components emit `Layout` (`Section`, `OrderedList`, `UnorderedList`,
  `Progress`, `TwoColumn`, `Table`, `BlockQuote`, plus darkmatter's
  `YamlBlock`). Their tree output is snapshot-tested in `layout_matrix` and
  parity-checked against the bespoke renderers in `render_comparison`.

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

## 6. Style — the unbuilt half (Spec B)

This feature is named "layout **and style**." Only layout (Spec A) is built.

Spec B — a `style` frontmatter schema, a `Style` slot system, and appearance
properties (background, fill, color tiers) — is **not implemented**. `Layout`
was deliberately scoped to *positioning only* so that `Style` can later be a
separate, orthogonal primitive riding alongside it on `NodeAttrs`. Appearance
concerns intentionally left out of `Layout`: page/row background, row fill
(the removed `RowFill`), and color.

When Spec B is taken up it should follow the same discipline: one primitive,
declared once by components, lowered per target, parity-gated against the
renderer it replaces.

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
- `.claude/skills/biscuit-terminal/render-tree.md` — terminal layout
  application.
- `renderable/features/2026-04-17-layout-and-style/` — the feature spec and
  implementation plan.
