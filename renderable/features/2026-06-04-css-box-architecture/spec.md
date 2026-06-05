---
status: ready for planning and implementation
reviewed: true
date: 2026-06-04
owner: ken
parent: renderable/features/2026-06-02-tree-cutover/spec.md
child_specs:
    - "2026-06-05-darkmatter-cutover"
    - "2026-06-05-renderer-folds"
    - "2026-06-05-tree-attrs"
    - "2026-06-05-style-vocabulary"
---

# CSS Box Architecture

A **CSS-faithful, multi-target rendering foundation** whose tree pipeline
resolves layout/style **once** and stays cheap as features accrue.

This directory holds **only** the high-level architecture and a high-level
description of each sub-spec. Each sub-spec lives in its own dated directory
with a single `spec.md`; the links below point to them.

## Origin

This started as "retire `DarkmatterPage`'s deprecated layout types." During
brainstorming it was reframed: retiring those types is the *last* step, not the
goal.

> **Reader note:** an earlier draft described the tree as not being consumed in
> production. That is no longer true. The tree-cutover work put Darkmatter's
> public terminal/browser paths and the relevant component paths on the render
> tree. The accurate basis for this spec is narrower: the existing `Layout` /
> `Style` vocabulary is still young, incomplete with respect to the CSS box
> model, and already surrounded by compatibility shims. We should fix the
> vocabulary before more public style behavior is built on top of those shims.

Byte-for-byte parity with the deprecated `DarkmatterPage` layout internals is
therefore *not* the goal. Characterization tests are reference points ("did the
cells change, and is the change an improvement?"), not immutable byte contracts
("the bytes must not move"). Public `style:` v1 input compatibility remains a
contract; internal lowering and exact whitespace/paint details may change when
the new behavior is documented and tested as an improvement.

## The unifying thesis

> **`style:` frontmatter (and every component) lowers *once* into `Layout` +
> `Style` attributes on the render tree. Every target — terminal, browser,
> markdown — is a single fold over those attrs. No `LayoutContext`
> side-channel, no bespoke per-target CSS, no second decorate pass.**

This is simultaneously the *flexibility* win and the *performance* win. Today
one policy exists in three representations — `LayoutContext` (resolved state),
a per-node `decorate.rs` pass that re-derives policy via
`component_for(NodeKind) → PageComponent → HashMap` lookups, and a third
bespoke `build_component_css` for the browser. Every new feature adds another
lookup/branch to that hot loop — exactly the structure that erodes performance
over time. Collapsing all three into "policy is baked into node attrs at
build time; renderers just fold" makes per-node cost constant and each
renderer a flat traversal.

This thesis is about ownership of policy, not a promise that every target can
express every visual property. The fold still applies target-specific
degradation rules:

- **Terminal** maps the CSS box model to cells: margin is transparent outer
  spacing, padding is reserved inner cells, background paints content and
  padding cells, and borders consume cells when present.
- **Browser** lowers the same attrs to HTML/CSS, preferably by shared renderer
  lowering rather than component-specific inline CSS builders.
- **Markdown** remains portable CommonMark/GFM by default. It ignores geometry
  and paint that cannot be represented portably.
- **MarkdownPlus** may lower safe inline HTML for style attrs where this is
  already part of the renderable contract, and may keep existing structural
  block HTML features. It must not become a second browser renderer; CSS-box
  fidelity belongs to Terminal and Browser.

## The CSS mental model

Layout and style map onto how a web author already thinks, which in turn maps
cleanly onto every render target:

- **`Layout` = the CSS box geometry** — `margin` (transparent outer space),
  `padding` (reserved inner space), `width` / `max-width`, `alignment`.
- **`Style` = paint** — `color`, `background` (which paints the padding box,
  per CSS), `border`, text emphasis.
- The bespoke `Page*` / `Fill` vocabularies collapse into these standard
  primitives with no loss of capability.

The hard part is never the model — it is faithfully mapping the model onto
**terminal** cell rendering. That mapping is where the engineering attention
goes.

## Sub-specs

Four design areas, each its own directory + `spec.md`. They are **chapters of
one coordinated change**, not independently shippable units: the vocabulary
change in *style-vocabulary* alters core types the terminal renderer and
darkmatter consume, so implementation is phased to keep the workspace
compiling, but the pieces interlock.

| Sub-spec | Area | Status |
|---|---|---|
| [`2026-06-04-style-vocabulary`](../2026-06-04-style-vocabulary/spec.md) | **Layout/Style vocabulary** — the CSS box model; geometry vs. paint; delete `Fill`; `padding` / `width` / `fit-content`; the defaulting contract. | **designed; ready for planning** |
| [`2026-06-04-tree-attrs`](../2026-06-04-tree-attrs/spec.md) | **Tree attrs & inheritance** — typed sparse `NodeAttrs` (no per-node JSON round-trip), one canonical inheritance resolver, and a deterministic structural-invariant performance *gate*. | **designed; ready for planning** |
| [`2026-06-04-renderer-folds`](../2026-06-04-renderer-folds/spec.md) | **One fold per target** — terminal + browser learn to paint the padding box, honor `fit-content`, and lower `padding`/`width`/`border`/`background`; provides the lowering that lets *darkmatter-cutover* retire `build_component_css`. | **designed; ready for planning** |
| [`2026-06-04-darkmatter-cutover`](../2026-06-04-darkmatter-cutover/spec.md) | **darkmatter cutover** — `style:` lowers directly to `Layout`/`Style` attrs; delete `Page*`, the per-component `LayoutContext` math, the bespoke CSS, and every `#![allow(deprecated)]`. *Absorbs the original "style-based-alignment" work.* | **designed; ready for planning** |

## Sequencing contract

These sub-specs are coordinated, but implementation should still land in
reviewable slices. The dependency order is:

1. **style-vocabulary** defines the Rust types and their defaults. It may make
   minimal compile fixes in consumers, but it does not finish renderer behavior.
2. **tree-attrs** defines sparse attr storage, inheritance, default elision, and
   the benchmark gate. It depends on the vocabulary names and defaults.
3. **renderer-folds** implements target behavior against those attrs and
   deletes per-target duplicate style paths, including bespoke browser CSS
   assembly.
4. **darkmatter-cutover** changes `style:` lowering to write attrs directly and
   deletes deprecated Darkmatter page/layout vocabulary.

Each child spec should carry a `parent` link to this document. Specs after
`style-vocabulary` should also carry `depends-on` for the immediately preceding
spec so planning tools preserve the order.

## Cross-cutting principles

1. **Policy is resolved exactly once, at tree-build time, into attrs.** No
   render-time re-derivation; no side-channel keyed by component.
2. **Performance is a tested gate, not a hope.** The gate is a *deterministic
   structural-invariant test* — folding a styled corpus must do zero per-node
   JSON round-trips and zero per-node key allocations on the hot path — chosen
   over a flaky timing budget. The `render_tree_parity` / `migration_parity` /
   `render_tree` Criterion benches are kept for *trend* visibility, not as the
   gate. (Designed in [`tree-attrs`](../2026-06-04-tree-attrs/spec.md).)
3. **Fewer, orthogonal primitives.** Properties compose (CSS-style) rather than
   bundling into flat one-of enums; this keeps lowering branch-free.
4. **Parity is a reference, not a contract.** Snapshot diffs are expected;
   each is judged as improvement vs. regression, not rejected on sight.

## Design decisions

### D1 — Preserve public `style:` input compatibility

The frozen `style:` v1 frontmatter keys remain accepted. This program changes
their internal lowering target from Darkmatter-specific page/layout state to
render-tree attrs. It does **not** rename frontmatter keys or remove deprecated
aliases unless a later Darkmatter spec explicitly schedules that as a separate
compatibility change.

### D2 — Absence means default, and default attrs may be elided

A node with no `Layout`/`Style` attrs must render the same as a node carrying
`Layout::default()` / `Style::default()`. Renderers may use attr absence as
their cheap path. This preserves the current performance intent of
`needs_decoration() == false` without keeping a second decoration model.

### D3 — Inheritance is explicit and paint-oriented

The planned `tree-attrs` spec should make inheritance opt-in per property.
Text color, text emphasis, and mode-dependent colors may inherit where that
matches CSS and current component expectations. Box geometry (`margin`,
`padding`, `width`, `max_width`, borders) must not inherit. Background should
not inherit by default; descendants paint their own box only when they carry a
background or when a renderer is painting the ancestor's padding/content box.

### D4 — Markdown degradation is intentional

CommonMark/GFM output remains structural. It should not grow synthetic spacing,
HTML wrappers, or CSS just to approximate terminal/browser layout. MarkdownPlus
can use narrowly scoped HTML for inline style fidelity, but block layout
fidelity is a Browser/Terminal concern.

### D5 — Deprecated internals are deleted only after direct lowering exists

`Page*`, `LayoutContext`, `decorate.rs` policy lookup, and
`build_component_css` are removed in the darkmatter cutover only after the
same public style cases lower to render-tree attrs and are covered by tests.
Deletion is the proof that there is one policy path.

## Acceptance criteria for the architecture program

1. All child specs are present, linked from this overview, and ordered with
   `depends-on` where applicable.
2. `renderable::layout` / `renderable::style` express the CSS box vocabulary
   without `Fill` or Darkmatter-specific page layout concepts.
3. `NodeAttrs` stores layout/style sparsely, treats absence as default, and has
   documented inheritance semantics.
4. Terminal and browser renderers perform one traversal over attrs for box
   layout and paint; duplicate per-target style lookup paths are deleted.
5. Markdown and MarkdownPlus behavior is documented as target-specific
   degradation rather than missing parity.
6. Darkmatter `style:` v1 inputs lower directly to attrs, with deprecated
   internal `Page*` / `LayoutContext` paths removed.
7. Performance gates compare against the post-tree-cutover baseline and fail
   on structural regressions outside explicitly documented fidelity exceptions.

## Open Questions

No architecture-blocking questions remain after this review. The remaining
choices belong in child specs as implementation details, especially the exact
`NodeAttrs` storage shape and the terminal algorithm for measuring
`fit-content` after inline style lowering.

## Relationship to prior work

- Parent: [`../2026-06-02-tree-cutover/spec.md`](../2026-06-02-tree-cutover/spec.md)
  flipped every render path onto the tree renderers; this program makes the
  *vocabulary* those renderers consume CSS-faithful and single-pass.
- Supersedes the original "retire deprecated `DarkmatterPage` layout types
  under byte-for-byte parity" framing. That work now lives in
  *darkmatter-cutover* and is *smaller*, because the foundation does the heavy
  lifting.
- The frozen `style:` v1 contract
  ([`../_completed/2026-05-23-style-property/`](../_completed/2026-05-23-style-property/))
  is preserved at the *frontmatter* surface; only its internal lowering target
  changes.
