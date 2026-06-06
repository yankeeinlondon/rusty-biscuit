---
ready: false
agent: codex
model: ""
---

# Review: Renderer Folds, Iteration 3

## Findings

### High: Level 2 assertions still do not prove the full terminal box-model contract

The real-terminal suite now exercises every width mode and all three alignment
settings, but several assertions are deliberately weaker than the specified
user-visible behavior.

- The painted-padding check asserts exactly two cells only on the left and says
  the right padding is left to Level 1 because trailing cells collapse in the
  capture (`biscuit-terminal/cli/tests/level2_render_tree_style.rs:914-954`).
  Acceptance criterion 1 requires two painted columns on **each** side at L2.
  Add a right border after the padding so those cells are not trailing, then
  parse the captured SGR run and assert two painted cells on both sides while
  the margin remains outside it.
- The `Auto` width check only requires a box wider than 30 cells
  (`level2_render_tree_style.rs:793-821`). A renderer that leaves substantial
  unused pane width would pass even though `Auto` must fill the largest content
  box allowed by margin, padding, border, and the pane width. Assert the exact
  box width from the harness pane geometry.
- The alignment checks only establish `left < center < right` for `Fixed`,
  `FitContent`, and capped `Auto` (`level2_render_tree_style.rs:762-791`,
  `862-912`). Incorrect offsets such as one-third/two-thirds placement would
  pass. Assert the exact lead for center and right from pane width, margins, and
  measured box width.

These are Level 2 gaps under the review rubric, not optional test improvements:
padding paint, terminal cell width, and box placement are terminal-emulator
observable requirements. The implementation's Level 1 tests pin the exact
values, but manufactured output cannot substitute for the required real-terminal
verification.

## Requirement Verification

| Requirement | Strongest verification present | Required level | Result |
|---|---:|---:|---|
| Painted padding and transparent margin | Partial L2 | L2 | Gap: right padding is not verified |
| `Fixed` and `FitContent` box widths | L2 | L2 | Pass |
| `Auto`, caps, and box-order clamp | Partial L2 | L2 | Gap: `Auto` fill is not measured exactly |
| Alignment for all width modes | Partial L2 | L2 | Gap: only relative ordering is asserted |
| Border adjacency and explicit padding gap | L2 | L2 | Pass |
| Browser padding/width/content-box lowering | L1 | L1 | Pass |
| Browser full border matrix | L1 | L1 | Pass |
| Borrowed-accessor performance invariant | L1/source inspection | L1 | Pass |
| Darkmatter remains unchanged | Source inspection | Source inspection | Pass |
| Skill and documentation updates | Source inspection | Source inspection | Pass |

Level 3 is not applicable because this feature has no keyboard, mouse, paste,
IME, or terminal input-encoder behavior.

## Verification Run

- `cargo test -p renderable --lib tree::render::browser::tests:: --color=never` — 111 passed.
- `cargo test -p biscuit-terminal --lib render_tree_ --color=never` — 150 passed.
- `cargo test -p biscuit-terminal-cli --test level2_render_tree_style --no-run --color=never` — compiled successfully.
- `cargo run -q -p biscuit-terminal-cli -- block hi --width 20 --border all` — rendered a 20-cell bordered interior correctly.
- `just test-l2` from `biscuit-terminal` — 68 passed on the available WezTerm, Kitty, tmux, and Apple Terminal harnesses.

## Production Readiness

Not ready for production under the mandated test-rigor standard. The iteration-2
painted-width defect is fixed and no new implementation defect was found, but
the L2 suite does not yet prove the exact padding, `Auto` fill, and placement
contracts it is required to verify.
