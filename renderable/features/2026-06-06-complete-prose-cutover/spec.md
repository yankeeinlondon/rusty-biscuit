---
title: Complete the Prose render-tree cutover
date: 2026-06-06
status: draft
reviewed: false
area: renderable
scope: biscuit-terminal Prose standalone terminal rendering; render-tree projection parity
depends-on: []
---

# Complete the Prose Render-Tree Cutover

> **Draft.** This spec describes a *discovered* incomplete-migration state and
> frames the work to finish it. It is intentionally light on implementation
> detail pending review. Numbers and file references reflect the tree as of
> 2026-06-06.

## Summary

`Prose` was supposed to be fully cut over to the `renderable` render-tree: one
parsed representation (`ProseDocument`) lowered into canonical `RenderNode`s,
with every target (Terminal, Browser, Markdown) reached through the shared
render-tree lowering and its capability-aware `Style`→output degradation.

That cutover is **incomplete**. The standalone terminal path —
`Prose::render` / `Prose::render_optimistic` — still emits through a **bespoke,
parallel ANSI emitter** (`components/prose/terminal.rs`) instead of the
render-tree. The render-tree projection (`Prose::to_render_nodes`) exists and is
used only when a `Prose` is *embedded* inside another render tree. So Prose has
two terminal emission paths that must be kept in agreement by hand — and they
have already drifted.

## Background: how Prose renders today

`Prose` parses its input once into the private `ProseDocument` IR, then emits to
targets through **four independent emitters**:

| Target | Entry point | Emitter | Path |
|--------|-------------|---------|------|
| Terminal (standalone) | `TerminalRenderable::render` / `render_optimistic` | `prose/terminal.rs` (`parse_tokens` → `terminal::render`) | **bespoke** |
| Terminal (embedded) | `Prose::to_render_nodes` via `render_tree/projection.rs:292` | render-tree terminal renderer | render-tree |
| Browser | `BrowserRenderable` | `prose/browser.rs` | bespoke |
| Markdown | `MarkdownRenderable` | `prose/to_markdown.rs` | bespoke |

The intended end state is that **terminal output has a single source of
truth** — the render-tree terminal renderer — reached by lowering
`ProseDocument` through `to_render_nodes`. Instead, standalone terminal
rendering (the overwhelmingly common case) bypasses the tree.

`prose/terminal.rs` describes itself as *"the behavioral oracle for `Prose`…
all terminal capability decisions happen here."* That comment encodes the
problem: capability decisions are supposed to live in the render-tree's shared
`Style` lowering, not in a Prose-private emitter.

## Problem

Two terminal emitters for one component is a standing drift hazard. The
render-tree path degrades colors/attributes through the shared, capability-aware
`render_tree::style` lowering; the bespoke path reimplements a subset of that by
hand. Nothing forces the two to agree, and there is no equivalence test pinning
them together.

### Evidence: this already caused a shipped bug (2026-06-05)

While adding a grey `µs` unit to the `MetricsTree` `--perf` report, a foreground
color reached the standalone path and exposed that
`prose/terminal.rs::fg_escape`/`bg_escape` **emitted truecolor `38;2;…`
regardless of the terminal's `ColorDepth`** — leaking color even under
`ColorDepth::None`. The render-tree path had degraded correctly the whole time;
only the bespoke emitter was wrong.

It went unnoticed because:

- the only color-degradation tests exercised the render-tree path;
- the standalone emitter's degradation was never tested;
- `MetricsTree`'s `no_color` test used a fixture with no foreground colors, so
  it passed vacuously until the first real color appeared.

That bug was patched by routing the bespoke emitter's fg/bg through the shared
`render_tree::style::color_sgr` and adding
`prose::standalone_render_degrades_colors_across_depths`. **That patch treats a
symptom.** The root cause — two emitters — remains.

## Goal

Make the render-tree the single source of truth for Prose terminal output:
standalone `Prose::render` / `render_optimistic` lower `ProseDocument` through
`to_render_nodes` and the render-tree terminal renderer, and the bespoke
`prose/terminal.rs` ANSI emitter is deleted.

Success criteria (to be firmed up in review):

- `Prose::render` / `render_optimistic` produce output via the render-tree, not
  `prose/terminal.rs`.
- `prose/terminal.rs` is removed (or reduced to a thin shim with no independent
  capability logic).
- A behavioral-equivalence test pins terminal output across `ColorDepth`,
  unicode/ASCII folding, and underline support so the two-emitter drift class
  cannot reappear.
- No observable change to terminal output for existing call sites except where
  the bespoke emitter was provably wrong (e.g. the color-depth leak).

## Gaps blocking the cutover

The render-tree projection is **not yet a behavioral superset** of the bespoke
emitter. These must be closed (or consciously accepted as breaking) before the
swap:

1. **Dropped style knobs.** `to_render_nodes` (`prose/tree.rs`) intentionally
   **drops `inverse` and `hidden`** — they have no peer in the render-tree
   `Style`. The bespoke emitter emits `\x1b[7m` / `\x1b[8m` for them. Either
   the render-tree `Style` gains these, or their removal is an accepted break.
2. **OSC8 hyperlinks.** Link emission and the "unsupported terminal → Markdown
   fallback" behavior currently live in `prose/terminal.rs::render_link`.
   Confirm the render-tree `Link` lowering reproduces OSC8 emission *and* the
   fallback rule.
3. **Code blocks.** Fenced code-block terminal presentation
   (`render_code_block`) must match through the tree's `Code` node.
4. **Underline degradation.** Double-underline → straight/none degradation
   (`degraded_double_underline_open`, driven by `UnderlineSupport`) must be
   reproduced by the tree path.
5. **SGR layering / reset sequencing.** The bespoke emitter uses `StyleState`
   to open/close style layers and emits a single trailing `\x1b[0m`. The tree
   renderer's open/close ordering and reset emission must be equivalent (exact
   byte parity is likely too strict — define the equivalence as *semantic* SGR
   state per the WezTerm-capture lesson, not byte-for-byte).
6. **Layout/wrapping interaction.** Standalone render runs the emitted string
   through `Layout::apply_layout` for width wrapping. Confirm wrapping over
   tree-rendered output (with its SGR) stays correct (visible-width aware).

## Non-goals

- Changing the Prose input grammar (bracketed tags + Markdown subset) or its
  `ProseDocument` IR shape.
- Reworking the Browser or Markdown emitters (separate, out of scope here).
- Changing the render-tree `NodeKind` vocabulary beyond what gaps 1–4 require.
- Any visual redesign of components that consume Prose.

## Risk / blast radius

- ~847 `Prose::new` call sites across the workspace; standalone terminal render
  is the dominant consumer. Any SGR/wrapping regression is broadly visible.
- `prose/terminal.rs` is the current behavioral oracle; deleting it without a
  proven-equivalent replacement risks silent output regressions.
- Mitigation: land the equivalence test harness *first* (diff bespoke vs.
  tree output across a fixture matrix and capability axes), close the gaps
  until it is green, then flip `render`/`render_optimistic` and delete the
  bespoke emitter in a single reviewable change.

## Open questions

- Do `inverse` / `hidden` get first-class render-tree `Style` support, or are
  they dropped (breaking change for any caller relying on them)?
- Is exact-byte SGR parity required, or semantic-state equivalence (preferred,
  per the L2 WezTerm-capture lesson)?
- Should the bespoke emitter be deleted outright, or retained briefly behind a
  feature flag to de-risk rollout?

## References

- Bug + symptom patch: `biscuit-terminal/lib/src/components/prose/terminal.rs`
  (now delegates color to `render_tree::style::color_sgr`),
  `prose::mod::standalone_render_degrades_colors_across_depths`.
- Projection entry point: `render_tree/projection.rs:292` (`to_render_nodes`).
- Shared degradation: `render_tree/style.rs::color_sgr` / `rgb_sgr` / `basic_sgr`.
- Tree projection + documented drops: `components/prose/tree.rs`.
