---
status: architecture overview
date: 2026-06-04
owner: ken
parent: renderable/features/2026-06-02-tree-cutover/spec.md
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
goal. The reframe is licensed by one fact — **nothing consumes the render tree
or its `Style`/`Layout` types in production yet.** Byte-for-byte parity with
today's output is therefore *not* a constraint. Characterization tests become
*reference points* ("did the cells change, and is the change an improvement?"),
not *contracts* ("the bytes must not move"). We optimize for the right
foundation, not for compatibility with output no one depends on.

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
| `2026-06-04-tree-attrs` *(planned)* | **Tree attrs & inheritance** — `NodeAttrs` sparse storage, inheritance semantics, and the performance benchmark *gate* (a CI budget set now while the corpus is small). | to brainstorm |
| `2026-06-04-renderer-folds` *(planned)* | **One fold per target** — terminal + browser learn to paint the padding box, honor `fit-content`, and lower `padding`/`width`/`border`/`background`; every bespoke side path (incl. `build_component_css`) is retired. | to brainstorm |
| `2026-06-04-darkmatter-cutover` *(planned)* | **darkmatter cutover** — `style:` lowers directly to `Layout`/`Style` attrs; delete `Page*`, `LayoutContext`, the bespoke CSS, and every `#![allow(deprecated)]`. *Absorbs the original "style-based-alignment" work.* | to brainstorm |

## Cross-cutting principles

1. **Policy is resolved exactly once, at tree-build time, into attrs.** No
   render-time re-derivation; no side-channel keyed by component.
2. **Performance is a tested gate, not a hope.** The existing
   `render_tree_parity` / `migration_parity` benches become a *budget* with a
   CI threshold, set now while the tree is small, so feature growth cannot
   silently erode it.
3. **Fewer, orthogonal primitives.** Properties compose (CSS-style) rather than
   bundling into flat one-of enums; this keeps lowering branch-free.
4. **Parity is a reference, not a contract.** Snapshot diffs are expected;
   each is judged as improvement vs. regression, not rejected on sight.

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
