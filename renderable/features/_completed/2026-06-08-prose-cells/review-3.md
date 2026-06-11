---
ready: false
agent: codex
model: ""
---

# Review 3

## Findings

### High: The canonical browser tier fails due to leaked Chrome handles

The package now correctly wires `test-browser` into the canonical lifecycle
(`biscuit-terminal/justfile:79`), but the recipe is not green. Running
`just -f biscuit-terminal/justfile test-browser` failed
`browser_prose_cell_background_computes` on all four nextest attempts with
`LKFAIL`: the test process exited successfully, but Chrome retained inherited
handles beyond the configured five-second browser leak timeout.

This means `just all` cannot pass and the required real-browser computed-style
coverage is not operational in the lifecycle that is intended to enforce it.
Fix the browser harness teardown rather than increasing or disabling the leak
timeout. The current `Drop` implementation aborts the handler and merely
spawns `browser.close()` without awaiting completion
(`biscuit-browser-harness/src/lib.rs:407`). Provide an explicit async shutdown
path that awaits browser closure and handler completion, and call it from each
browser test before the Tokio runtime exits.

### High: Level 2 tests still do not verify style containment at borders

The new wrapped and multiline fixtures verify that each plain-text line has
box borders and that its raw row contains a bold SGR
(`biscuit-terminal/cli/tests/level2_prose_cells.rs:146`,
`biscuit-terminal/cli/tests/level2_prose_cells.rs:171`). Those checks can pass
when the trailing padding or border is also bold: `assert_bordered_line`
strips attributes, while `sgr_carries_bold` only proves that bold appears
somewhere on the row.

The separate reset check has the same false-positive window. It finds a reset
after `alphaword`, but never proves that the reset occurs before the trailing
border (`biscuit-terminal/cli/tests/level2_prose_cells.rs:214`). A row with a
bold border followed by a reset would pass.

The specification requires styled multiline and wrapped content not to bleed
into padding, separators, borders, or adjacent rows. Under the review rubric,
that user-visible styling requirement needs a Level 2 assertion against the
terminal's captured cell state. Parse SGR transitions through the row and
assert that bold is active over the content cells but inactive at the trailing
padding and border. Apply that assertion to every wrapped and explicit-newline
line, accepting the documented reset encodings rather than requiring one
literal reset sequence.

## Verification Levels

| Requirement | Strongest verification | Assessment |
|---|---|---|
| Conversion, projection, hints, code degradation, layout isolation | Level 1 | Appropriate |
| No-color output | Level 1 | Appropriate |
| Single-line terminal content and styling | Level 2 | Appropriate |
| Wrapped and multiline geometry | Level 2 | Appropriate |
| Wrapped and multiline style containment at padding/borders | Level 1 effective coverage | Gap; current Level 2 assertions permit false positives |
| Cursor-alignment visible content and styling | Level 2 | Appropriate |
| Resolve each Prose cell once | Level 1 instrumentation | Appropriate |
| Browser semantic markup and links | Level 1 | Appropriate |
| Browser supported visual styles | Browser tier | Tests exist, but the canonical tier fails with leaked handles |
| Markdown and MarkdownPlus parity | Level 1 | Appropriate |
| Keyboard, modifier, paste, IME, or mouse behavior | Not applicable | No Level 3 requirement |

## Verification Run

- `cargo test -p biscuit-terminal --test prose_cells_parity --color=never`:
  58 passed.
- `cargo test -p biscuit-terminal-cli --test level2_prose_cells --no-run
  --color=never`: passed.
- `cargo check -p biscuit-terminal-cli -p biscuit-terminal -p renderable
  --color=never`: passed.
- `just -f biscuit-terminal/justfile test-browser`: failed after four
  `LKFAIL` attempts on `browser_prose_cell_background_computes`.
- `just -f biscuit-terminal/justfile test-l2`: did not reach the test run
  because `biscuit-harness-broker spawn apple-terminal` remained blocked for
  more than 60 seconds; the command was terminated rather than left running.

The implementation itself remains well covered at Level 1, and iteration 3
closes the browser-recipe wiring and reset-encoding issues from review 2.
Production readiness is still blocked by the failing browser lifecycle and
the incomplete Level 2 style-containment assertion.
