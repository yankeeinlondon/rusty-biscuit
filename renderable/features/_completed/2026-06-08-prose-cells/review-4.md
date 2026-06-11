---
ready: false
agent: codex
model: ""
---

# Review 4

## Findings

### High: Table-cell dim and hyperlink behavior lack Level 2 verification

The specification requires dim, bold, color, and link content to resolve using
the supplied terminal capabilities. The Prose-cell Level 2 suite verifies bold
and red foreground styling, but every fixture is limited to `<b>` and `<red>`
(`biscuit-terminal/cli/tests/level2_prose_cells.rs:264`,
`biscuit-terminal/cli/tests/level2_prose_cells.rs:337`). It never renders
`<dim>` or an `<a href=...>` inside a table cell.

Standalone Prose has Level 2 OSC8 coverage in
`biscuit-terminal/cli/tests/level2_prose_styling.rs`, but that does not exercise
the new table-cell projection, terminal reconstruction, wrapping, or bespoke
resolution boundaries. The feature-specific Level 1 suite only proves that a
link projects as `NodeKind::Link` (`biscuit-terminal/lib/tests/prose_cells_parity.rs:135`);
it does not assert terminal link output at all. Dim is only present in no-color
and visible-text fixtures, so no test proves that dim styling appears when the
terminal supports it.

Add Level 2 table-cell fixtures for dim and OSC8-capable links in WezTerm and
Kitty. Assert dim is active over the intended content and inactive at padding
and borders. For links, assert the captured raw row retains the expected OSC8
destination and label while the plain capture keeps valid table geometry.

### High: Mixed typed-row alignment is not verified as rendered geometry

The specification requires mixed `StyledProse`, integer, float, and currency
rows to retain typed formatting and alignment. The current test confirms the
formatted values and checks that the integer cell's hint says `"right"`
(`biscuit-terminal/lib/tests/prose_cells_parity.rs:399`), but it never asserts
where those values are rendered within their cells.

This is a user-visible terminal layout requirement. A metadata assertion cannot
detect a renderer that ignores the hint or an interaction where a preceding
styled cell changes width planning. Add a Level 2 mixed-row fixture and assert
the captured cell boundaries place the Prose value at the left edge and the
numeric values at the right edge for both the standard and cursor-alignment
paths.

## Verification Levels

| Requirement | Strongest verification | Assessment |
|---|---|---|
| Conversion, projection, hints, code degradation, layout isolation | Level 1 | Appropriate |
| No-color output | Level 1 | Appropriate |
| Bold and color in terminal cells | Level 2 | Appropriate |
| Dim and OSC8 links in terminal cells | Level 1 / standalone-Prose Level 2 | Gap; the table-cell path is not verified |
| Wrapped and multiline geometry/style containment | Level 2 | Appropriate; iteration 4 closes the false-positive window |
| Cursor-alignment visible content and bold styling | Level 2 | Appropriate |
| Mixed typed formatting | Level 1 | Appropriate |
| Mixed typed alignment | Level 1 metadata only | Gap; rendered geometry needs Level 2 |
| Resolve each Prose cell once | Level 1 instrumentation | Appropriate |
| Browser semantic markup and links | Level 1 | Appropriate |
| Browser supported visual styles | Browser tier | Appropriate; canonical tier is green |
| Markdown and MarkdownPlus parity | Level 1 | Appropriate |
| Keyboard, modifier, paste, IME, or mouse behavior | Not applicable | No Level 3 requirement |

## Verification Run

- `cargo test -p biscuit-terminal --test prose_cells_parity --color=never`:
  58 passed.
- `cargo test -p biscuit-terminal-cli --test level2_prose_cells
  bold_containment --color=never`: 6 passed.
- `cargo check -p biscuit-browser-harness -p biscuit-terminal-cli
  -p biscuit-terminal -p renderable --color=never`: passed.
- `cargo nextest run -p biscuit-terminal -E
  'test(/browser_prose_cell_background_computes/)' --no-fail-fast
  --color=never`: passed without a leak failure.
- `just -f biscuit-terminal/justfile test-browser`: 81 passed, including all
  three real-browser Prose-cell computed-style tests.
- The Level 2 test binary compiled, but the real-terminal tier was not executed
  during this review.

Iteration 4 correctly fixes both review-3 blockers: browser teardown is awaited
and the Level 2 bold-containment assertions now reconstruct per-cell SGR state
with negative controls. Production readiness remains blocked by the two
user-visible terminal requirements above that are still verified at the wrong
level.
