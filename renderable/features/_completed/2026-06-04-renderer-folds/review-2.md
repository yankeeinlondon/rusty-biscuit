---
ready: false
agent: codex
model: ""
---

# Review: Renderer Folds, Iteration 2

## Findings

### High: Terminal `Fixed` and `Auto` widths do not determine the painted box width

`render_with_layout` resolves a content-box width and uses it for wrapping and outer placement, but `paint_text` independently derives the painted band from the widest emitted content line (`biscuit-terminal/lib/src/render_tree/render.rs:386-400`, `biscuit-terminal/lib/src/render_tree/style.rs:170-192`). The resolved width is never passed to the paint/border layer. A short line in a `Fixed(20)` node therefore still produces a two-column content box when a background or border makes the box visible.

This is directly reproducible with `bt block hi --width 20 --border all`, which renders:

```text
┌──┐
│hi│
└──┘
```

The expected border encloses 20 content cells. The same defect affects an `Auto` box with short content: the renderer computes the full available content width but background and border painting shrink to the text. The current `render_tree_width_fixed_sets_content_box` and `render_tree_border_is_added_around_fixed_content_box` tests assert only the box's leading offset, so they pass without checking its rendered width (`render.rs:3456-3478`, `render.rs:3620-3650`).

Pass the resolved content-box width into the paint layer and use it as the rectangular band's inner width when box paint makes that width observable. Add assertions for the actual border/background width for `Auto`, `Fixed`, capped `Auto`, and `FitContent`, including short and wrapped multi-line content.

Strongest verification: Level 1 tests encode placement but miss rendered box width; the direct CLI reproduction fails the specified behavior.

### High: Level 2 coverage still does not verify the terminal width-mode and clamp requirements

The new real-terminal tests cover background presence, one visible left-padding cell, border adjacency, and relative left/center/right placement of a `Fixed(20)` node (`biscuit-terminal/cli/tests/level2_render_tree_style.rs:618-683`). They do not assert the rendered width of that fixed box, exercise `Auto` or `FitContent`, or cover `max_width`, margin/padding/border clamps, percentage padding, and narrow available widths. The relative alignment test therefore passes even while the fixed box itself renders at the wrong width.

Acceptance criterion 1 specifically asks for two painted columns on each side and a transparent outer margin in L2. The current test only searches the whole raw frame for any background SGR and checks that some row contains ` padglyph`; it does not verify the right padding, the requested two-column width, or margin transparency. Criteria 2 and 3 require all width modes, caps, clamps, and placement to be terminal-observable, but their strongest complete checks remain Level 1.

Add real-terminal captures that measure the visible painted/bordered box for `Auto`, `Fixed`, and `FitContent`; cover center/right placement for each mode; cover `max_width` and a representative box-order clamp; and inspect the raw styled row to prove exactly two background-painted padding cells per side while margin cells remain outside the SGR run.

Strongest verification: partial Level 2. Required: Level 2 for the user-visible terminal geometry and paint contracts.

## Requirement Verification

| Requirement | Strongest verification present | Required level | Result |
|---|---:|---:|---|
| Painted terminal padding and transparent margin | Partial L2 | L2 | Gap: exact side widths and transparent margin are not captured |
| `Auto` / `Fixed` / `FitContent`, caps, and box-order clamp | L1 | L2 | Gap; `Fixed`/`Auto` painted width is functionally wrong |
| Alignment for all three width modes | Fixed-only L2; all modes L1 | L2 | Gap |
| Border adjacency and explicit padding gap | L2 | L2 | Pass |
| Browser padding/width/content-box lowering | L1 | L1 | Pass |
| Browser full border matrix | L1 | L1 | Pass |
| Borrowed-accessor performance invariant | L1 perf gate/source inspection | L1 | Pass |
| Darkmatter remains unchanged | Source inspection | Source inspection | Pass |
| Skill and documentation updates | Source inspection | Source inspection | Pass |

Level 3 is not applicable: this feature has no OS keyboard, mouse, paste, or terminal input-encoder behavior.

## Verification Run

- `cargo test -p renderable --lib tree::render::browser::tests:: --color=never` — 111 passed.
- `cargo test -p biscuit-terminal --lib render_tree_ --color=never` — 143 passed.
- `cargo test -p biscuit-terminal-cli --test level2_render_tree_style --no-run --color=never` — compiled successfully.
- `cargo run -q -p biscuit-terminal-cli -- block hi --width 20 --border all` — reproduced the undersized fixed-width border shown above.

## Production Readiness

Not ready for production. The browser gaps and the prior parent-basis/alignment defects are resolved, but terminal width currently controls wrapping and placement without controlling the visible painted box, and the L2 suite does not verify the remaining width-mode contract strongly enough to catch that defect.
