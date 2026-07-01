---
status: draft
reviewed: false
date: 2026-06-30
owner: ken
origin: biscuit-terminal Table width:auto/fill gap (tables ignored layout.width; fractional Fixed double-applied)
depends-on:
  - renderable/features/_completed/2026-04-17-layout-and-style/layout-spec.md
  - renderable/features/_completed/2026-04-17-layout-and-style/style-spec.md
  - renderable/features/_completed/2026-06-04-css-box-architecture/spec.md
---

# Style Everywhere — Universal Layout & Style Support

Every renderable component in `biscuit-terminal` and `darkmatter` MUST honor
every `Layout` and `Style` property that applies to it, on every render target,
**or** degrade in a single, documented, test-pinned way. There must be no
component that silently ignores a property a caller set.

The trigger was the `Table` component: it carried a `Layout` whose `width` it
never read, so `width: 100%` / `width: auto` had no effect on its columns, and
once a fill was added a fractional `Fixed(50%)` resolved its percentage twice
(box sized to 50%, then columns filled to 50% of *that* = 25%). That class of
bug — a component with its own internal layout that partially re-implements,
ignores, or double-applies the shared box model — is what this feature
eliminates across the board.

## Goal

A single, enforceable invariant:

> For each (component, property, target) where the property is *applicable*, the
> component's output reflects that property, and a regression test pins it.
> Where a property is *not applicable* or *cannot be honored* on a target, that
> is an explicit, documented, tested degradation — never a silent no-op.

```rust
// All of these MUST take effect on Terminal and Browser, and degrade
// deterministically on Markdown — for EVERY block component, not just some.
let mut c = SomeComponent::new(/* ... */);
c.layout_mut().width      = Width::Fixed(TargetValue::universal(Length::Percent(50.0)));
c.layout_mut().margin     = Edges::x(Length::ch(4));
c.layout_mut().padding    = Edges::all(Length::ch(1));
c.layout_mut().max_width  = Some(TargetValue::universal(Length::ch(60)));
c.layout_mut().alignment  = Alignment::Center;
// Style:
c.style_mut().background  = Some(Background::subtle());
c.style_mut().border      = Some(Border::thin());
c.style_mut().emphasis    = TextEmphasis { bold: true, ..Default::default() };
c.style_mut().color       = Some(/* adaptive accent */);
```

## Motivation

The render tree already provides a correct, shared box-model fold:
`render_with_layout` (`biscuit-terminal/lib/src/render_tree/render.rs`) resolves
`margin`, `padding`, `width` (Auto/Fixed/FitContent), `max_width`, and
`alignment` for *any* block node, and `render_styled` paints `background`,
`border`, and text appearance. A component that projects a `RenderNode` and
routes through this fold gets the whole box model for free.

The gaps are the components that **don't** fully rely on that fold:

1. **Internal-layout components** plan their own widths and may not fill or
   honor the box they are handed (`Table` — fixed; `TwoColumn`, `OrderedList`,
   `UnorderedList`, `StatusBlock`, `FileSystem` — unaudited).
2. **Hint round-trip components** reconstruct themselves from typed hints for a
   second render pass and can drop `Layout`/`Style` in the round-trip (`Table`
   needed its node `layout.width` copied onto the reconstructed planning table —
   see [the hint-roundtrip gotcha](#c4-hint-round-trip-must-carry-layout-and-style)).
3. **Bespoke / escape-hatch components** render terminal bytes directly with no
   tree projection and so bypass the fold entirely (`MetricsTree`, `Status`,
   `MermaidDiagram`, `TerminalImage`, `HorizontalRule` image tier, the table
   `prefer_cursor_alignment` path).
4. **Darkmatter's `style:` frontmatter** exposes only a subset of the property
   surface (`width`, `max-width`, `alignment`, `color`, `bg-color`, plus
   page-level margins/padding/background). `ComponentPolicy.layout` is a full
   `Layout`, but per-component `padding`, `border`, `emphasis`, `word_wrap`, and
   the `Width` *mode* (Auto vs Fixed vs FitContent) are not addressable.

This feature makes the box model uniform, defines the degradation rules, and
builds the test matrix that keeps it uniform.

## Property Surface

The authoritative property set is the two structs in the `renderable` crate.
This spec does not add properties; it ensures the existing ones are honored
everywhere.

### `Layout` (`renderable/src/layout/mod.rs`) — the box

| Property | Type | Inherits | Applies to | Notes |
|----------|------|----------|------------|-------|
| `margin` | `Edges` | no | block | outer space; transparent |
| `padding` | `Edges` | no | block | inner space; painted by `background` |
| `width` | `Width` | no | block | `Auto` \| `FitContent` \| `Fixed(len)` |
| `max_width` | `Option<TargetValue<Length>>` | no | block | caps content box |
| `alignment` | `Alignment` | no | block | position of a sub-width box within the area |
| `word_wrap` | `WordWrap` | no | block text | per-component cell/column policy may override |

`Width` semantics (proven for `Table`, now normative for all):

- `Width::Auto` (default) and `Width::Fixed(len)` → **fill** the available
  width; a designated child/column absorbs the slack.
- `Width::FitContent` → **hug** the content's widest line.
- **Unbounded-width guard:** when the available width is the `u32::MAX`
  natural-measurement sentinel, the component hugs regardless of `width` (there
  is nothing finite to fill).

`Length` units: `Ch(n)`, `Percent(p)`, `Zero`, per-target `Css`.

### `Style` (`renderable/src/style.rs`) — the appearance

| Property | Type | Inherits | Applies to | Notes |
|----------|------|----------|------------|-------|
| `color` | `Option<TargetValue<PerMode<PaintColor>>>` | **yes** | block + inline `Span` | adaptive light/dark |
| `background` | `Option<TargetValue<PerMode<PaintColor>>>` | no | block | paints the padding box |
| `emphasis` | `TextEmphasis` | **yes** | block slot + inline | bold/dim/italic/strikethrough/blink/inverse/underline |
| `border` | `Option<Border>` | no | block | `color`, `weight`, `line_style`, `sides`, `radius` |

### Targets

`Terminal`, `Browser`, and `Markdown` (incl. the `MarkdownPlus` dialect). Each
target has a different expressible surface; the matrix below fixes the expected
result per target.

## Definitions — what "support" means

Every (component, property, target) cell has exactly one status:

- **Honored** — the property visibly takes effect, matching the shared fold's
  result for that property.
- **Degraded(rule)** — the target cannot express the property; the component
  applies the one documented fallback (e.g. Markdown drops `background` to
  nothing; truecolor `color` degrades to the nearest ANSI on a 16-color
  terminal). The fallback is deterministic and tested.
- **N/A** — the property does not apply to this component kind (e.g. box `margin`
  on an inline component; `word_wrap` on a `Table` whose wrapping is per-column).

Silent no-op (a property set by the caller that neither takes effect nor has a
documented degradation) is a **defect** this feature forbids.

## Contracts

### C1 — Route block components through the shared box fold

Every block component's terminal/browser/markdown output MUST derive its
`margin`, `padding`, `width`, `max_width`, `alignment`, `background`, `border`,
and inherited `color`/`emphasis` from the shared render-tree fold
(`render_with_layout` + `render_styled` and their browser/markdown siblings).
A component MUST NOT re-implement the box model. New components are tree-first.

### C2 — Internal-layout components fill the box they are handed

A component that plans its own content widths (columns, side-by-side panes,
hanging indents) operates **inside** the content width the fold hands it. It:

- MUST fill that width when `width` is `Auto`/`Fixed` and hug it when
  `FitContent`, using the [Width semantics](#layout-renderablesrclayoutmodrs--the-box) above;
- MUST define and document which sub-element absorbs slack (e.g. `Table`'s last
  visible column);
- MUST NOT re-resolve a `Length` the fold already resolved. The fold sizes the
  outer box from `width`/`max_width`; the component fills the *resolved cells*
  it receives, never the raw percentage again (the `Fixed(50%)` → 25%
  double-application bug).

### C3 — Honor the unbounded-width guard

When measuring natural width (no finite bound — the `u32::MAX` sentinel),
fill-style widths degrade to hug. "Fill the available width" requires a finite
available width.

### C4 — Hint round-trip must carry Layout and Style

A component that reconstructs itself from typed hints for a planning/second pass
(`Table` from `TableColumnHints`, `TwoColumn` from `ColumnsHints`, lists from
`ListRenderHints`, `Todo` from `TaskHints`) MUST round-trip the node's `Layout`
and `Style` onto the reconstructed instance so the second pass honors the same
properties as the first. Margins already consumed by the fold are not
re-applied; the *mode-bearing* fields (`width`, `alignment`, `background`,
`border`) must survive.

### C5 — Bespoke / escape-hatch components have a minimum bar

A component that emits terminal bytes directly with no tree projection
(`MetricsTree`, `Status`, `MermaidDiagram`, `TerminalImage`, the
`HorizontalRule` image tier, the table cursor-alignment path) MUST still honor
`margin`, `alignment`, and `max_width` (outer-box placement is target-agnostic),
and MUST document in rustdoc the exact set of properties it cannot honor and why
(e.g. an image protocol cannot paint a CSS `border`). Every such limitation is a
**Degraded** or **N/A** matrix cell with a test, not an undocumented gap.
Preferred direction: project a `RenderNode` so the fold applies the box, and
reserve bespoke emission for the irreducible core (image escapes, cursor moves).

### C6 — Cross-target parity and degradation

For a given component and property:

- Terminal and Browser MUST both Honor the property unless physically
  impossible, and the two render paths within a target (e.g. `render()` vs
  `render_tree`, or tree vs `prefer_cursor_alignment` bespoke) MUST agree.
- Markdown Honors only what GFM/MarkdownPlus can express and Degrades the rest
  by the documented rule; it never emits ANSI or raw CSS.

### C7 — Inline components carry no box

Inline components (`Prose` inline content, `InlineContent`, `Status`,
`PadLeft`/`PadRight`, `Compose`) do not own a box: `margin`/`padding`/`width`/
`max_width`/`alignment`/`background`/`border` are **N/A** (the containing block
owns them). They MUST honor inherited `color` and `emphasis`. `Pad*` width is
its own explicit contract, not the `Layout` box.

## Scope

### 1. Baseline: shared fold guarantees (audit, don't rebuild)

Confirm and pin that `render_with_layout` / `render_styled` (terminal) and the
browser/markdown siblings already Honor all six `Layout` and four `Style`
properties for a plain `Paragraph`/`Section`. This is the reference output every
other component is compared against. No new behavior; add the missing matrix
scenarios (see Verification) so the baseline is locked.

### 2. Internal-layout components (the highest-risk group)

For each of `Table` (✅ done — reference implementation), `TwoColumn`,
`OrderedList`, `UnorderedList`, `StatusBlock`, `FileSystem`, `GraphExpression`,
`Progress`:

- Apply C2/C3/C4.
- Audit `width` handling: does the component fill on Auto/Fixed and hug on
  FitContent? Does it absorb slack at a defined element? Does it double-resolve a
  percentage?
- Audit the hint round-trip for dropped `Layout`/`Style`.
- Add a per-component entry to the matrix and the snapshot harness.

`Table` is the worked example and its tests (`width_auto_fills_available`,
`width_fit_content_hugs_below_available`, `width_auto_hugs_when_width_is_unbounded`,
`width_fixed_full_*`, `render_tree_table_fixed_percent_does_not_double_apply`,
the `layout_matrix__Table__*` snapshots) are the template.

### 3. Bespoke / escape-hatch components

For `MetricsTree`, `Status`, `MermaidDiagram`, `TerminalImage`,
`HorizontalRule`, the table cursor-alignment path: apply C5. Decide per
component whether to (a) project a tree node and inherit the fold, or (b) remain
bespoke with a documented, tested honored-subset (`margin`/`alignment`/
`max_width`) and explicit N/A cells for the rest. Record the decision in this
spec's matrix.

### 4. Inline components

For `Prose` (inline), `InlineContent`, `Status`, `PadLeft`/`PadRight`,
`Compose`: apply C7. Confirm inherited `color`/`emphasis` flow; mark the box
properties N/A with a one-line rationale each.

### 5. Darkmatter `style:` frontmatter — expose the full applicable surface

`ComponentPolicy.layout` is already a full `renderable::layout::Layout`, but the
`style:` schema (`darkmatter/lib/src/style/schema/`, `descriptor.rs`) exposes
only `width`, `max-width`, `alignment`, `color`, `bg-color` per component plus
page-level margins/padding/background. Requirements:

- For each `PageComponent` (`Tables`, `BlockQuotes`, `CodeBlocks`, `Ul`, `Ol`,
  `Li`, `Disclosure`, `Images`, `Hyperlinks`, `Hr`), the schema MUST be able to
  express every `Layout`/`Style` property that is *applicable* to that
  component, or this spec MUST record why it is intentionally omitted.
  Candidate gaps to resolve: per-component `margin`/`padding`, `border`,
  `emphasis`, `word_wrap`.
- The `width` value MUST map to the correct `Width` **mode**: a provided length
  ⇒ `Width::Fixed`; absent ⇒ the component's sensible default (`Width::Auto`
  fill for full-bleed blocks like tables/quotes, or `FitContent` where hugging
  is the documented default). The mode, not just the length, must be set so
  darkmatter blocks fill/hug per C2.
- `apply_node_policy` (`build_context.rs`) MUST attach the resulting `Layout`/
  `Style` such that darkmatter's terminal/HTML/markdown output matches a
  hand-built `renderable` tree with the same properties (parity with §1).
- Disclosure inline `key=value` parameters MUST cover the same expanded surface
  they already pioneer (`max-width`, `color`, `alignment`, …).

### 6. The property × component × target matrix (deliverable)

Produce and maintain a matrix table (in this spec or an adjacent
`matrix.md`) with one row per component and columns for each
`Layout`/`Style` property × {Terminal, Browser, Markdown}, each cell tagged
`Honored` / `Degraded(rule)` / `N/A`. The matrix is the checklist; every cell
must be backed by a test before the feature is `implemented`.

## Verification

The existing `biscuit-terminal/lib/tests/layout_matrix.rs` harness is the
backbone: it renders each component case across layout scenarios through both
`render()` (`VIA_RENDER`) and `render_tree` (`VIA_TREE_DIRECT`) and snapshots
the ANSI-stripped, side-by-side block — which simultaneously pins the output and
asserts **render-path parity**. Extend it:

1. **Scenarios** — current set covers margins, `align_center/right`,
   `max_width_40`, `word_wrap_prose`, widths 40/120. ADD: `width_auto_fill`,
   `width_fit_content`, `width_fixed_pct_50`, `padding_all_1`,
   `background_subtle`, `border_thin_left`, `emphasis_bold_italic`. Each new
   scenario exercises one property in isolation.
2. **Cases** — every block component from the inventory MUST have a case (today
   the matrix covers a subset). Inline components get an inline-content matrix
   with the box scenarios asserted as no-ops (N/A) and the style scenarios
   asserted as Honored.
3. **Parity assertions** — `VIA_RENDER == VIA_TREE_DIRECT` for every cell; for
   components with a `prefer_cursor_alignment`/bespoke path, add a third column
   and assert it agrees on the honored subset.
4. **Width-mode unit tests** — per internal-layout component, mirror the `Table`
   tests: Auto fills, FitContent hugs, Fixed(%) does not double-apply, unbounded
   hugs, slack lands on the defined element.
5. **Browser + Markdown** — snapshot the HTML fragment and the
   Markdown/MarkdownPlus output per case; assert Honored properties appear (CSS
   for box/style on Browser; structure on Markdown) and Degraded ones follow
   their rule (no ANSI/CSS leakage into Markdown).
6. **Darkmatter** — a `style:` frontmatter round-trip suite: a document setting
   each property per component renders to terminal/HTML matching the equivalent
   hand-built `renderable` tree (parity with §1); unknown/omitted properties are
   reported by the existing schema validation, not silently dropped.
7. **No-silent-noop guard** — a meta-test (or review checklist item) that every
   matrix cell is either covered by an assertion or explicitly marked N/A with a
   rationale.

Level 2 (real-terminal) coverage is only required where a property introduces
new terminal protocol/emulator behavior; the box model and SGR are Level
1/unit + snapshot.

## Documentation

- Update each component's rustdoc to state which `Layout`/`Style` properties it
  Honors, Degrades, or treats as N/A, per the matrix.
- Update `biscuit-terminal/docs/components/*.md` and the
  `.claude/skills/biscuit-terminal` / `.claude/skills/renderable` /
  `.claude/skills/darkmatter` skill docs with the universal-support contract and
  the matrix.
- Update `darkmatter`'s `style:` frontmatter reference (`style/descriptor.rs`
  doc + any `style:` topic doc) to list the newly exposed properties per
  component.
- Refresh `md hash` frontmatter on any edited skill docs.

## Non-Goals

- Adding new `Layout`/`Style` properties. The surface is fixed; this is about
  honoring what exists.
- Changing the inheritance model (`color`/`emphasis` inherit; `background`/
  `border`/box do not).
- Forcing bespoke components into the tree where the tree cannot represent their
  irreducible core (image escapes, cursor positioning) — those keep a documented
  honored-subset.
- Per-target pixel/cell-exact parity between Terminal and Browser. Parity is
  *semantic* (the property took effect) and *within-target* (render paths
  agree), not cross-target byte equality.
- Reworking darkmatter's policy/merge architecture; only the exposed schema
  surface and the `Width`-mode mapping change.

## Open Questions (draft)

1. **Slack policy per component.** `Table` chose "last visible column absorbs
   slack." What is the right slack element for `TwoColumn` (proportional? last
   pane?), lists (the text column?), `StatusBlock`? Each needs a decision.
2. **Default `Width` per darkmatter component.** Which blocks default to
   `Auto`-fill (tables, block quotes, code blocks?) vs `FitContent`-hug
   (inline-ish, short callouts)? This changes default rendering.
3. **Markdown degradation table.** Enumerate the exact fallback for each
   non-expressible property on Markdown vs MarkdownPlus (e.g. does MarkdownPlus
   carry `color` as an HTML span? does `border` map to a blockquote?).
4. **`word_wrap` ownership.** For components with per-column/per-cell wrap
   (`Table`), is layout-level `word_wrap` N/A, or a default the columns inherit?
5. **Bespoke vs tree-first decisions.** For each §3 component, choose (a)
   project-to-tree or (b) documented honored-subset, and record it.
6. **Border on terminal.** Confirm the terminal border paint
   (`border_horizontal_overhead` + glyphs) covers all `Border` sub-properties
   (`weight`, `line_style`, `sides`, `radius`) or document which degrade on
   terminal.

## Acceptance Criteria

1. A published property × component × target matrix exists, every cell tagged
   `Honored` / `Degraded(rule)` / `N/A`.
2. Every block component routes its box model through the shared fold (C1), or —
   if bespoke — honors the `margin`/`alignment`/`max_width` minimum and
   documents its honored subset (C5).
3. Every internal-layout component fills/hugs per `Width`, absorbs slack at a
   documented element, round-trips `Layout`/`Style` through its hints, and does
   not double-resolve a length (C2/C3/C4). `Table`'s test set is matched per
   component.
4. Inline components honor inherited `color`/`emphasis` and mark box properties
   N/A (C7).
5. Darkmatter's `style:` frontmatter can express every applicable property per
   component (or records the omission), maps `width` to the correct `Width`
   mode, and renders at parity with an equivalent hand-built `renderable` tree.
6. The `layout_matrix` harness covers every component across the expanded
   scenario set, with `VIA_RENDER == VIA_TREE_DIRECT` (and bespoke-path
   agreement) on every cell, plus Browser/Markdown snapshots.
7. No silent no-op: every property a caller can set either takes effect or hits
   a documented, tested degradation.
8. Component rustdoc, component docs, skill docs, and the darkmatter `style:`
   reference describe the implemented support per the matrix.
