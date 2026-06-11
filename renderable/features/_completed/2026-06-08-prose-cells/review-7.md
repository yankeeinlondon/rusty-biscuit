---
ready: false
agent: codex
model: ""
---

# Review 7

## Findings

### High: Level 2 containment assertions still ignore the leading edge of styled cells

The stateful SGR helper verifies that the style is active over the content and
inactive from the end of the content through the final border
(`biscuit-terminal/cli/tests/level2_prose_cells.rs:420`). It never inspects the
visible cells before the content. The OSC8 helper has the same shape
(`biscuit-terminal/cli/tests/level2_prose_cells.rs:555`).

Consequently, these assertions accept captures where styling starts before the
cell content and incorrectly covers leading padding, the leading border, a
separator, or an earlier cell. For example, the bold helper accepts a row
equivalent to `bold-on, leading-border, padding, active, bold-off, trailing
border`; the red helper accepts a row where red begins before the first border
and remains active through the preceding `active` cell and separator before
ending after `Alice`. An OSC8 opener before the leading border has the same
false-positive window.

This also leaves adjacent-row containment incomplete: if a style carries into
the next captured row and is cleared only after that row's leading geometry,
the current assertions do not inspect the affected cells.

The specification requires styled multiline and wrapped content not to bleed
into padding, separators, borders, or adjacent rows. Under the review rubric,
that user-visible requirement needs complete Level 2 state verification.
Extend the shared containment logic to locate the nearest preceding `│` and
require the state to be inactive from that border through the cell's leading
padding, then active only over the intended content and inactive afterward.
Apply the same rule to OSC8 state. Add negative helper tests for a styled
leading border, styled leading padding/separator, and a link opened before the
leading border.

## Verification Levels

| Requirement | Strongest verification | Assessment |
|---|---|---|
| Conversion, projection, hints, code degradation, layout isolation | Level 1 | Appropriate |
| No-color output | Level 1 | Appropriate |
| Bold, dim, red, and OSC8 state over content and trailing geometry | Level 2 | Appropriate |
| Style/link containment over leading padding, separators, borders, and adjacent-row leading geometry | Level 1 effective coverage | Gap; current Level 2 assertions do not inspect cells before the content |
| Wrapped and multiline visible geometry | Level 2 | Appropriate |
| Standard and cursor-alignment path styling | Level 2 | Partial because both use the incomplete containment helper |
| Mixed typed formatting and alignment in both terminal paths | Level 2 | Appropriate |
| Resolve each Prose cell once | Level 1 instrumentation | Appropriate |
| Browser semantic markup and supported visual styles | Browser tier | Appropriate |
| Markdown and MarkdownPlus parity | Level 1 | Appropriate |
| Keyboard, modifier, paste, IME, or mouse behavior | Not applicable | No Level 3 requirement |

## Verification Run

- `cargo test -p biscuit-terminal --test prose_cells_parity --color=never`:
  58 passed.
- `cargo test -p biscuit-terminal-cli --test integration_test test_table_
  --color=never`: 16 passed.
- `cargo test -p biscuit-terminal-cli --test level2_prose_cells containment
  --color=never`: 30 passed.
- `cargo check -p biscuit-terminal-cli -p biscuit-terminal -p renderable
  --color=never`: passed.
- `just -f biscuit-terminal/justfile test-l2 prose_cells`: Kitty passed; tmux
  failed to create a session after four attempts, and nextest canceled WezTerm.

Iteration 7 fixes review 6's public CLI compatibility regression by moving
typed test columns to `--column-types`, and it closes trailing color and
cursor-path bold containment. Production readiness remains blocked by the
leading-edge false-positive window in the Level 2 state assertions.
