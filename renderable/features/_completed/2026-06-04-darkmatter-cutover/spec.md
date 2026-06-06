---
status: ready for planning
date: 2026-06-04
owner: ken
parent: renderable/features/2026-06-04-css-box-architecture/spec.md
depends-on: renderable/features/2026-06-04-renderer-folds/spec.md
---

# darkmatter Cutover — Retire the Deprecated Page Vocabulary

The final chapter of the [CSS Box Architecture](../2026-06-04-css-box-architecture/spec.md),
and where the program's original goal lands: **delete `DarkmatterPage`'s
deprecated layout types.** With the renderable foundation in place — the CSS box
[vocabulary](../2026-06-04-style-vocabulary/spec.md), typed
[attrs](../2026-06-04-tree-attrs/spec.md), and the box-model
[folds](../2026-06-04-renderer-folds/spec.md) — darkmatter's `style:` frontmatter
lowers **directly** into `Layout`/`Style` node attrs, the per-component
side-channel and bespoke browser CSS are deleted, and every
`#![allow(deprecated)]` they forced is removed.

## Background

### Today's pipeline (the thing this collapses)

```
style: frontmatter
  → apply.rs            down-converts renderable → deprecated:
                        map_alignment (Alignment→PageAlignment),
                        lower_length_to_fill (Length→WidthUnit/PageFill)
  → DarkmatterPage      builder stores Page* + HashMap<PageComponent, PageAlignment/PageFill>
  → LayoutContext::from_page
     ├─ terminal:  decorate.rs queries ctx per node → sets Layout/Style attrs;
     │             apply_row_decoration (page-frame post-pass)
     └─ browser:   build_component_css + the .darkmatter-page wrapper <div>
```

The deprecated types (`PageMargin`, `PagePadding`, `PageAlignment`, `PageFill`,
`WidthUnit`, `PageComponent::Lists`) and their `From`/`TryFrom` bridges in
`darkmatter/lib/src/layout/types.rs` are an *intermediate* vocabulary — the
renderers already terminate in `renderable::layout::Layout` / `renderable::style`
on node attrs. The bridges and the per-component `LayoutContext` math exist only
to translate `style:` policy into those attrs.

### What the foundation already provides

- The CSS box vocabulary (`Edges`, `Width`, `padding`, `background`, `Border`)
  and the deletion of `Fill` (style-vocabulary).
- Typed sparse node attrs and the shared `InheritedStyle` resolver (tree-attrs).
- Terminal + browser folds that render `padding` / `width` / `border` and lower
  them per node — including the browser lowering that replaces
  `build_component_css` (renderer-folds).

So the per-component cell/CSS math darkmatter does today (`resolve_component_width`,
`alignment_padding`, `component_side_padding`, `build_component_css`) is now the
**fold's** job; darkmatter only has to *set the attrs*.

## Goals

- Lower `style.{table,images,block-quote,ul,ol,li,code-blocks,hyperlinks,hr}.*`
  **directly** into per-component `Layout`/`Style`, with no down-conversion to
  deprecated types.
- Write that policy onto nodes in `decorate.rs`; let the renderer folds do all
  width/padding/alignment/CSS resolution.
- Delete `PageMargin`, `PagePadding`, `PageAlignment`, `PageFill`, `WidthUnit`,
  `PageComponent::Lists`, the `types.rs` bridges, `build_component_css`, the
  per-component `LayoutContext` math, and **every** `#![allow(deprecated)]` they
  forced.
- Keep a **slim, renderable-typed page frame** (`DarkmatterPage` stays the
  assembler facade) for full-bleed background/margin/padding rows, max-width
  centering, and the `PageBackground::Pronounced` render-mode flip.
- Switch darkmatter's `decorate` onto the shared `InheritedStyle` resolver.
- Preserve `style:` v1 frontmatter **input** byte-for-byte (architecture D1).

## Non-Goals

- Removing `DarkmatterPage` or `PageComponent` (the page-assembler facade and
  the component taxonomy are retained; neither is deprecated).
- Folding the page frame onto a root `RenderNode` ("page-as-box") — this spec
  keeps the slim bespoke page frame (the rejected alternative (b) from
  brainstorming); a future spec may unify it.
- New `style:` keys, a schema version bump, or any frontmatter-key rename
  (those are a separate darkmatter compatibility spec).
- Changing the renderable folds (owned by renderer-folds) or the attr/vocabulary
  types (owned by the earlier sub-specs).

## The design

### 1. Per-component policy lowers directly to `Layout`/`Style`

Introduce the per-component policy value darkmatter carries between the apply
layer and `decorate`:

```rust
/// The renderable policy a `style:`-configured PageComponent contributes.
struct ComponentPolicy {
    layout: renderable::layout::Layout,        // margin/padding/width/max_width/alignment
    style: Option<renderable::style::Style>,   // color/background (+ border where used)
}
// carried as HashMap<PageComponent, ComponentPolicy>
```

- `apply.rs` builds `ComponentPolicy` directly from the `style:` values
  (`Alignment`, `Length`, `Color`) — **delete `map_alignment`,
  `lower_length_to_fill`**, and the `Length→u16`/`→WidthUnit` helpers that fed
  the deprecated builder. The `style:` mapping is the one fixed in
  style-vocabulary (`fill: pad` → `padding`, `fill: max` → `max_width`,
  `width` → `Width::Fixed`, `align` → `Layout.alignment`, `bg` →
  `Style.background`, etc.).
- `apply_color_style` folds component `color` / `bg-color` into the
  `ComponentPolicy.style`. The sub-spec-#7 bespoke knobs (`page.stylesheet`,
  `page.meta`, `page.code.theme`, hyperlink/image local-style) are **untouched**
  — they are not layout types.

### 2. `decorate.rs` writes attrs; the fold does the math

`decorate_document` maps each block's `NodeKind` → `PageComponent` (unchanged
`component_for`), looks up its `ComponentPolicy`, and writes `Layout` (+ optional
`Style`) onto the node via the typed accessors. It performs **no** width/offset
math. The deleted `LayoutContext` methods (`resolve_component_width`,
`alignment_padding`, `component_side_padding`, `component_fill`,
`component_alignment`) are gone — `renderer-folds` resolves width/padding/
alignment from the `Layout` attr at fold time. Inheritance push-down uses the
shared `InheritedStyle` resolver instead of darkmatter's bespoke pass.

### 3. Browser: delete `build_component_css`

Per-component browser CSS now comes from `renderer-folds` lowering each node's
`Layout`/`Style`. `build_component_css` and its `component_selectors` /
`emit_component_*` helpers are deleted. The `.darkmatter-page` **wrapper** stays
(page frame, below).

### 4. Slim, renderable-typed page frame

`DarkmatterPage` stays the facade but stores renderable types:

- page margin/padding → `Edges`; max-width → `Length`/`TargetValue<Length>`;
  page background → `Background` (adaptive tint) driven by the retained
  `PageBackground` knob.
- `apply_row_decoration` (terminal full-bleed margin/padding/background rows +
  centering) and the `.darkmatter-page` wrapper `<div>` (browser) read those
  renderable values instead of `Page*`.
- `LayoutContext` is reduced to a **page-frame residue** (effective width,
  resolved page background color, and the `PageBackground::Pronounced`
  render-mode flip — which inverts the code-theme color mode and stays bespoke).
  Its per-component HashMaps and math are deleted.
- The page-frame terminal background constants unify onto the `Background`
  tints (reference, not contract).

### 5. Delete the deprecated vocabulary + allows

Remove from `darkmatter/lib/src/layout/types.rs`: `PageMargin`, `PagePadding`,
`PageAlignment`, `PageFill`, `WidthUnit`, `PageComponent::Lists`, and the
`From`/`TryFrom` bridges. Remove every `#![allow(deprecated)]` /
`#[allow(deprecated)]` in `layout/{mod,page,context,types}.rs`,
`darkmatter/lib/src/cli.rs`, and `darkmatter/lib/tests/{layout_snapshots,style_frontmatter}.rs`
that existed for these types. `DarkmatterPage::rebuild_layout` keeps exporting
the page-frame `Layout` for `TerminalRenderable`, now from the page's
renderable-typed fields.

## Acceptance Criteria

1. **Deprecated types gone.** `rg 'PageMargin|PagePadding|PageAlignment|PageFill|WidthUnit'`
   over the repo returns nothing outside historical `features/` docs.
   `PageComponent::Lists` no longer exists.
2. **No `#![allow(deprecated)]` for these types** anywhere in darkmatter
   (`layout/*`, `cli.rs`, tests). Any remaining allow is re-justified by an
   unrelated deprecation.
3. **`build_component_css` and the per-component `LayoutContext` math are
   deleted**; `rg 'build_component_css|resolve_component_width|component_side_padding'`
   returns nothing.
4. **`style:` apply layer carries renderable types end to end** — no
   `Alignment→PageAlignment` or `Length→WidthUnit` conversion remains in
   `darkmatter/lib/src/style/`.
5. **`style:` v1 input unchanged.** Every previously-passing `style:` test
   parses and applies the same frontmatter; `ACTIVE_STYLE_WIRING_SUB_SPEC` and
   the `--strict-style` warning surface are unchanged.
6. **Output is a documented reference.** `md` terminal + browser output for the
   fixture corpus (`layout_snapshots`, the `style:` suite, `render_tree_*`) is
   reviewed; intended diffs vs. the old `Page*`/`build_component_css` path are
   snapshot-updated with rationale (parity is a reference, not a byte contract).
7. **Page frame preserved.** Full-bleed background, max-width centering, and the
   `Pronounced` mode-flip still render; `DarkmatterPage` and `PageComponent`
   remain.
8. **Docs updated.** `layout/mod.rs` "Migration deferral" section (now *done*),
   the darkmatter skill's deferral note, and
   `darkmatter/docs/rendering/style.md` reflect the direct-to-attrs lowering and
   the removed types.

## Risks

- **Terminal cell-math parity.** The deleted `LayoutContext` math now happens in
  the renderer fold; per-component output can shift. Mitigation: characterize
  representative `style:` cases (centered table, padded code block, indented
  block-quote, list left-margin) before the cut; diff after; document intended
  diffs. Parity is a reference.
- **Page-frame vs node double-application.** Page margin/padding (frame) and
  per-component padding (node) must not double-apply. Mitigation: the frame owns
  only page-level spacing; per-component spacing is on nodes; a test renders a
  page with both and asserts no doubled gutter.
- **`Pronounced` mode-flip.** Easy to lose when `PageBackground` is reworked.
  Mitigation: keep the flip in the page-frame residue with the existing
  `pronounced_background_snapshot` / `end_to_end_example_snapshot` as guards.
- **Wide `#![allow(deprecated)]` removal.** Removing the module allows may expose
  unrelated deprecations. Mitigation: remove allows last; re-justify or fix each
  warning the compiler then surfaces.

## Related

- [`../2026-06-04-css-box-architecture/spec.md`](../2026-06-04-css-box-architecture/spec.md)
  — architecture overview; this is the chapter that absorbs the original
  "retire deprecated types" goal.
- [`../2026-06-04-renderer-folds/spec.md`](../2026-06-04-renderer-folds/spec.md)
  — provides the per-node lowering that replaces `build_component_css` and the
  `LayoutContext` math.
- [`darkmatter/lib/src/layout/`](../../../darkmatter/lib/src/layout/) — `types.rs`
  (deprecated types + bridges), `page.rs` (`DarkmatterPage`,
  `build_component_css`, `apply_row_decoration`), `context.rs` (`LayoutContext`).
- [`darkmatter/lib/src/style/apply.rs`](../../../darkmatter/lib/src/style/apply.rs)
  — `map_alignment` / `lower_length_to_fill` and the per-component application
  this spec rewrites.
- [`darkmatter/lib/src/markdown/render_tree/decorate.rs`](../../../darkmatter/lib/src/markdown/render_tree/decorate.rs)
  — the decorate pass that switches from `LayoutContext` queries to direct attr
  writes.
