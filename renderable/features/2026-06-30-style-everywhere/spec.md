---
status: ready for planning and implementation
reviewed: true
date: 2026-06-30
owner: ken
review_iterations: 1
origin: biscuit-terminal Table width:auto/fill gap (tables ignored layout.width; fractional Fixed double-applied)
depends-on:
  - renderable/features/_completed/2026-04-17-layout-and-style/layout-spec.md
  - renderable/features/_completed/2026-04-17-layout-and-style/style-spec.md
  - renderable/features/_completed/2026-06-04-css-box-architecture/spec.md
---

# Style Everywhere — Universal Layout & Style Support

> **Reader note from inline review:** this spec intentionally raises the bar
> from "component-specific layout happens to work" to a repo-wide support
> contract. The review resolved the original draft's broad claims into target-
> specific rules: Markdown is a structural target and deliberately degrades
> layout/appearance to no-op output; inline spans may carry inline
> `background`, but they do not own a block box; and terminal `max_width` is
> treated as supported because the current renderer resolves and tests it even
> though one older skill note still says otherwise. The implementation should
> update that stale skill note when this feature lands.

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
| `background` | `Option<TargetValue<PerMode<PaintColor>>>` | no | block + inline `Span` | block: paints content + padding box; inline: paints inline content only |
| `emphasis` | `TextEmphasis` | **yes** | block slot + inline | bold/dim/italic/strikethrough/blink/inverse/underline |
| `border` | `Option<Border>` | no | block | `color`, `weight`, `line_style`, `sides`, `radius` |

### Targets

`Terminal`, `Browser`, and `Markdown` (incl. the `MarkdownPlus` dialect). Each
target has a different expressible surface; the matrix below fixes the expected
result per target.

Target defaults:

- **Terminal** Honors universal `Layout` and `Style` values through the shared
  tree fold. It Degrades color precision and unsupported emphasis/border
  variants according to terminal capability.
- **Browser** Honors universal `Layout` and `Style` values through CSS/HTML.
  `Length::Css` is Browser-only and valid only inside a per-target value.
- **Markdown / MarkdownPlus** are structural outputs. They intentionally ignore
  `Layout` and `Style` and must not emit ANSI, CSS, or raw styling HTML merely
  to preserve appearance. Any existing semantic Markdown emitted by a node
  (lists, block quotes, task checkboxes, emphasis nodes, links, images, tables)
  is still rendered as structure; appearance attrs are
  `Degraded(markdown_ignores_appearance_or_layout)`.

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

For Markdown, "derive from the shared fold" means the Markdown fold sees the
same attrs and applies the Markdown degradation rule in one place. Components
must not add bespoke Markdown-only styling preservation.

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
`margin`, `alignment`, and `max_width` where the component owns a block box
(outer-box placement is target-agnostic), and MUST document in rustdoc the exact
set of properties it cannot honor and why (e.g. an image protocol cannot paint a
CSS `border`). Every such limitation is a **Degraded** or **N/A** matrix cell
with a test, not an undocumented gap.
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
`PadLeft`/`PadRight`) do not own a `Layout` box: `margin`/`padding`/`width`/
`max_width`/`alignment` are **N/A** (the containing block owns them). They MUST
honor inherited `color` and `emphasis`. Inline `Span` nodes MAY honor
`Style.background` by painting only the inline content; that is not the block
padding-box background contract. `border` is **N/A** for inline components until
a separate inline-border design exists. `Pad*` width is its own explicit
contract, not the `Layout` box.

`Compose` and `Prose` can be block containers when used through their component
APIs. In that mode they are governed by C1; only their nested inline content is
governed by this contract.

### C8 — User-settable means either support or validation

Any public builder, frontmatter key, directive parameter, or CLI claim that lets
a caller set a `Layout`/`Style` property must land in one of three places:
Honored, Degraded(rule), or rejected during validation with a specific error.
"Accepted but ignored" is not allowed. This covers darkmatter `style:`, inline
Disclosure parameters, and any compatibility builder retained on
`biscuit-terminal` components.

### C9 — Keep semantic style distinct from appearance attrs

`Style.emphasis` is visual inheritance. Existing semantic tree nodes
(`Strong`, `Emphasis`, `Delete`, links, images, list structure, GFM task
checkboxes) remain structural and continue to render to Markdown when the tree
contains them. This feature MUST NOT replace semantic nodes with `Style`
appearance attrs, because doing so would make Markdown lose semantics under the
normal Markdown degradation rule.

## Reviewed Design Decisions

### D1 — Markdown degradation is structural-only

Markdown and MarkdownPlus do not attempt to preserve `Layout` or `Style` with
raw HTML, inline CSS, or ANSI escapes. The single fallback rule is
`Degraded(markdown_ignores_appearance_or_layout)`.

Pros:

- Keeps Markdown portable, readable, and safe for plain Markdown consumers.
- Matches the current renderable skill contract that the Markdown renderer
  deliberately ignores appearance.
- Avoids creating a second styling implementation in MarkdownPlus.

Cons:

- Appearance parity with Browser/Terminal is intentionally impossible for
  Markdown outputs.
- Callers that need styled HTML must choose the Browser target.

Recommended because Markdown is the repository's structural interchange target,
not a visual rendering target.

### D2 — Slack policy is component-local but must be deterministic

Internal-layout components choose one stable slack sink and document it:

| Component | Slack sink |
|-----------|------------|
| `Table` | last visible flexible column (already the reference implementation) |
| `TwoColumn` | right column after honoring explicit/fractional left width and gap |
| `OrderedList` / `UnorderedList` / `Todo` | item body text column; marker/hanging indent stays fixed |
| `StatusBlock` | message/body region; prefix, status glyph, and border chrome stay fixed |
| `FileSystem` | entry-label region; connector and icon columns stay fixed |
| `Progress` | bar track width; labels/brackets stay fixed |
| `GraphExpression` | rendered graph canvas, capped by the component's own graph/image constraints |

Pros:

- Preserves stable marker/prefix/icon geometry.
- Gives each component one testable rule instead of inventing a shared
  abstraction too early.

Cons:

- The matrix must document the sink per component.
- Some components may need bespoke measurement helpers.

Recommended because component internals differ enough that a universal
"last child absorbs slack" rule would be wrong for lists, progress bars, and
file trees.

### D3 — Darkmatter defaults preserve current rendering unless a key is set

Absence of a `style:` key must preserve current output. A provided `width`
length maps to `Layout.width = Width::Fixed(length)`. New explicit width-mode
syntax may be added as `width-mode: auto | fit-content | fixed` or
`width: auto | fit-content | <length>`, but omitting width does not change the
component's existing default.

Pros:

- Avoids a broad visual churn from merely adding the schema.
- Lets users opt into fill/hug behavior explicitly.

Cons:

- Some component defaults remain non-obvious until the matrix documents them.
- Adding keyword width parsing requires careful compatibility with today's
  length parser.

Recommended because this feature is a support/compliance pass, not a visual
redesign of darkmatter documents.

### D4 — `word_wrap` is inherited only where the component has text leaves

For prose-like components, `Layout.word_wrap` is Honored by wrapping the text
content. For composite internal-layout components, `word_wrap` is a default
fed into text-bearing cells/items only when they do not already carry a more
specific wrap policy. It is **N/A** for non-text or image/protocol cores.

Pros:

- Gives callers a useful default for table/list cells without clobbering
  per-column or per-item policies.
- Keeps image and protocol components from pretending to wrap pixels.

Cons:

- Requires explicit precedence tests for tables, lists, and file trees.

Recommended because it matches the existing split between block layout and
component-specific text wrapping.

### D5 — Bespoke components stay bespoke only for irreducible output

The preferred fix is a tree wrapper with a bespoke leaf: use shared layout/style
around the component and keep direct terminal bytes only for image protocols,
cursor moves, or external renderer output. Fully bespoke rendering remains
allowed only when a tree wrapper cannot preserve behavior; those cases must
carry documented Degraded cells.

Pros:

- Maximizes reuse of the shared fold.
- Keeps protocol-specific code small and auditable.

Cons:

- Some components need an adapter layer before their existing renderer.
- Exact byte output may change where shared layout fixes previous no-ops.

Recommended because it makes future style properties cheaper and prevents the
same bug class from returning.

### D6 — Terminal border support is capability-aware, not all-or-nothing

Terminal Honors border presence, sides, and color through box drawing/SGR.
Unsupported line-style, weight, or radius variants Degrade through one
documented fallback table, e.g. nearest available glyph family and square
corners where rounded corners are not represented.

Pros:

- Accurately models terminal constraints.
- Lets Browser keep full CSS border fidelity.

Cons:

- Requires a small explicit degradation table and tests for each border
  sub-property.

Recommended because pretending every border sub-property is fully representable
in terminals would make the matrix misleading.

## Scope

### 1. Baseline: shared fold guarantees (audit, don't rebuild)

Confirm and pin that `render_with_layout` / `render_styled` (terminal) and the
browser sibling already Honor all six `Layout` and four `Style` properties for
a plain `Paragraph`/`Section`, while the markdown sibling applies D1's single
degradation rule. This is the reference output every other component is
compared against. No new behavior; add the missing matrix scenarios (see
Verification) so the baseline is locked.

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
- Apply D2's slack-sink decision unless implementation evidence proves a listed
  sink would break existing semantics; if so, update the matrix and the
  component rustdoc in the same change.

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

Initial reviewed disposition:

| Component/path | Direction | Rationale |
|----------------|-----------|-----------|
| `TerminalImage` | tree wrapper + bespoke image leaf | image protocol is irreducible; outer placement/style fallback can be shared |
| `MermaidDiagram` | tree wrapper + rendered-image/text leaf | external rendering is irreducible; box placement is not |
| `HorizontalRule` image tier | tree wrapper for placement; bespoke glyph/image core | HR has structural `ThematicBreak` semantics and target-specific drawing |
| `Table::prefer_cursor_alignment` | keep bespoke cursor core; assert parity on honored subset | cursor moves are terminal-only and cannot be represented in the tree |
| `MetricsTree` | evaluate tree projection first | output is structured text/tree data, so a projection should be feasible |
| `Status` | classify as inline/badge unless used as a block | avoid forcing a block box onto inline status labels |

### 4. Inline components

For `Prose` (inline), `InlineContent`, `Status`, `PadLeft`/`PadRight`,
`Compose`: apply C7. Confirm inherited `color`/`emphasis` flow; confirm inline
background behavior for `Span`; mark `Layout` box properties N/A with a one-line
rationale each. For `Prose` and `Compose` block-container entry points, apply
C1 instead of C7.

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
  `emphasis`, `word_wrap`, and explicit `width` mode.
- The `width` value MUST map to the correct `Width` **mode**: a provided length
  ⇒ `Width::Fixed`; absent ⇒ preserve the component's current default per D3.
  If keyword width syntax is added, `auto` maps to `Width::Auto` and
  `fit-content` maps to `Width::FitContent`. The mode, not just the length, must
  be set so darkmatter blocks fill/hug per C2.
- `apply_node_policy` (`build_context.rs`) MUST attach the resulting `Layout`/
  `Style` such that darkmatter's terminal/HTML/markdown output matches a
  hand-built `renderable` tree with the same properties (parity with §1).
- Disclosure inline `key=value` parameters MUST cover the same expanded surface
  they already pioneer (`max-width`, `color`, `alignment`, …).
- `Hyperlinks` and `Images` are special: their `width`/`max-width`/`alignment`
  are `TextLayoutHints` or lone-image block layout, not generic block `Layout`
  on every image/link node. The matrix must spell out that distinction so the
  implementation does not attach invalid block layout attrs to inline nodes.
- `CodeBlocks` must be represented in the schema/reference even if its public
  key remains under the existing `page.code`/code-theme area for compatibility;
  callers need one documented place to see which `Layout`/`Style` properties
  affect code blocks.

### 6. The property × component × target matrix (deliverable)

Produce and maintain a matrix table (in this spec or an adjacent
`matrix.md`) with one row per component and columns for each
`Layout`/`Style` property × {Terminal, Browser, Markdown}, each cell tagged
`Honored` / `Degraded(rule)` / `N/A`. The matrix is the checklist; every cell
must be backed by a test before the feature is `implemented`.

Minimum seed rows for that matrix:

| Component group | Terminal | Browser | Markdown |
|-----------------|----------|---------|----------|
| Plain block nodes (`Paragraph`, `Section`) | Honor all applicable `Layout`/`Style`, with terminal capability degradation | Honor all applicable `Layout`/`Style` through CSS | Degrade all `Layout`/`Style` attrs by D1; preserve structure |
| Internal-layout components | Honor C2/C3/D2 plus inherited style | Honor C2/C3/D2 plus CSS style | Degrade attrs by D1; preserve component structure where Markdown has syntax |
| Bespoke/protocol components | Honor C5 subset; Degrade irreducible protocol cells | Honor if a browser representation exists, otherwise N/A/Degraded documented per component | Degrade attrs by D1; preserve structural fallback |
| Inline content | Honor inherited color/emphasis and inline background where represented; no `Layout` box | Honor inherited color/emphasis and inline background where represented; no `Layout` box | Preserve semantic inline Markdown nodes; Degrade appearance attrs by D1 |
| Darkmatter `style:` policies | Must match equivalent hand-built render tree or validation error | Must match equivalent hand-built render tree or validation error | Must match D1 degradation and preserve structural Markdown |

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
   for box/style on Browser) and Markdown follows D1 (structure preserved, no
   ANSI/CSS/raw styling leakage).
6. **Darkmatter** — a `style:` frontmatter round-trip suite: a document setting
   each property per component renders to terminal/HTML matching the equivalent
   hand-built `renderable` tree (parity with §1); unknown/omitted properties are
   reported by the existing schema validation, not silently dropped.
7. **No-silent-noop guard** — a meta-test (or review checklist item) that every
   matrix cell is either covered by an assertion or explicitly marked N/A with a
   rationale.

8. **Validation guard** — for every newly exposed darkmatter `style:` key and
   directive parameter, add one positive lowering test and one unsupported/
   invalid-value test proving the key is either represented in the tree or
   rejected before rendering.

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

## Open Questions

All draft open questions that could be resolved from existing contracts are
closed in [Reviewed Design Decisions](#reviewed-design-decisions). The
remaining questions are implementation-scoping questions that should be answered
while building the matrix.

1. **Should darkmatter accept `width: auto` / `width: fit-content`, or add a
   separate `width-mode` key?**

   - Option A: parse keywords in `width`.
     Pros: compact and CSS-like. Cons: changes the existing `width` parser from
     length-only to length-or-keyword and needs careful errors.
   - Option B: add `width-mode`.
     Pros: preserves existing length parser and gives explicit mode control.
     Cons: less CSS-like; `width` + `width-mode: fixed` can be redundant.
   - Option C: defer explicit mode syntax and support only fixed lengths.
     Pros: smallest implementation. Cons: leaves `Auto`/`FitContent` unreachable
     from frontmatter.

   **Recommendation:** Option A if the parser can produce precise errors without
   compatibility fallout; otherwise Option B. Do not choose Option C unless
   implementation scope must be cut, because it leaves a known schema gap.

2. **How much Browser representation should protocol components expose?**

   - Option A: Browser renders a semantic placeholder/fallback node with full
     layout/style.
     Pros: property support is testable and deterministic. Cons: not a true
     visual equivalent for terminal image protocols.
   - Option B: Browser omits unsupported protocol output and marks most cells
     N/A.
     Pros: honest about missing browser functionality. Cons: callers cannot
     rely on styled fallbacks.
   - Option C: Browser invokes equivalent external rendering where available.
     Pros: highest visual parity. Cons: dependency and performance surface grows.

   **Recommendation:** Option A for this feature. It keeps the style contract
   enforceable without expanding external rendering dependencies.

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
