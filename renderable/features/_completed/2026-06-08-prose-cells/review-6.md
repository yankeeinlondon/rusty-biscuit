---
ready: false
agent: codex
model: ""
---

# Review 6

## Findings

### High: Color and cursor-path styling containment remain unverified at Level 2

The specification requires styled cells not to bleed into padding, separators,
borders, or adjacent rows, and requires both standard and cursor-alignment
terminal paths to preserve styling. The new stateful assertions close this gap
for bold wrapping, multiline bold, dim, and OSC8 links, but not for color or the
cursor path's bold fixture.

`assert_prose_cell_styled` only checks that a red foreground sequence appears
somewhere in the captured row
(`biscuit-terminal/cli/tests/level2_prose_cells.rs:431`). A row whose red
foreground remains active over padding or borders still passes.
`assert_prose_cursor_align` likewise checks only that bold appears somewhere and
that a border exists
(`biscuit-terminal/cli/tests/level2_prose_cells.rs:530`). A cursor-aligned row
whose bold state reaches the border also passes.

These are user-visible SGR-state requirements, so Level 1 reset tests or Level 2
presence checks are insufficient under the review rubric. Generalize the
captured-cell state parser to track foreground state and assert that red is
active on `Alice` but inactive on following padding and borders. Apply
`assert_bold_contained` to `cursorword` in the cursor-alignment fixture. Run
both containment checks in Kitty and WezTerm.

### Medium: Recognized type suffixes still change valid legacy headers

`parse_column_spec` now preserves unknown colon suffixes, but it consumes any
recognized suffix from the existing `--columns` value
(`biscuit-terminal/cli/src/commands/table.rs:28`). Before this test scaffolding,
all non-comma header text was literal. A valid existing command such as:

```text
bt table --columns "Revenue: USD" --row "9.99"
```

now renders the header as `Revenue` and changes the column to right-aligned
currency semantics. The new compatibility test covers only an unrecognized
suffix (`Time: Value`), so it misses this collision
(`biscuit-terminal/cli/tests/integration_test.rs:3877`).

Do not overload literal header text for test-only typing. Use a separate typed
column option or another unambiguous test fixture mechanism, and add regression
tests for literal headers ending in `int`, `integer`, `float`, `usd`, `gbp`, and
`eur`.

## Verification Levels

| Requirement | Strongest verification | Assessment |
|---|---|---|
| Conversion, projection, hints, code degradation, layout isolation | Level 1 | Appropriate |
| No-color output | Level 1 | Appropriate |
| Bold wrapping/multiline containment | Level 2 | Appropriate |
| Dim containment | Level 2 | Appropriate |
| Red foreground presence | Level 2 | Partial; containment is not verified |
| OSC8 links, including cursor-alignment containment | Level 2 | Appropriate |
| Cursor-alignment bold styling | Level 2 presence only | Gap; containment is not verified |
| Mixed typed formatting and alignment in both terminal paths | Level 2 | Appropriate |
| Resolve each Prose cell once | Level 1 instrumentation | Appropriate |
| Browser semantic markup and supported visual styles | Browser tier | Appropriate |
| Markdown and MarkdownPlus parity | Level 1 | Appropriate |
| Keyboard, modifier, paste, IME, or mouse behavior | Not applicable | No Level 3 requirement |

## Verification Run

- `cargo test -p biscuit-terminal-cli --test integration_test test_table_
  --color=never`: 14 passed.
- `cargo test -p biscuit-terminal --test prose_cells_parity --color=never`: 58
  passed, including the browser tests.
- Direct `level2_prose_cells` execution passed Kitty and tmux; WezTerm could not
  spawn because `openpty` returned `Device not configured`.
- Canonical `just -f biscuit-terminal/justfile test-l2 prose_cells` passed Kitty,
  then failed because tmux could not create a session; WezTerm was canceled.
- Manual CLI reproduction confirmed that `Revenue: USD` is rendered as
  `Revenue` and assigned typed right alignment.

Iteration 6 closes review 5's OSC8 containment and cursor-path link coverage
gap. Production readiness remains blocked by the remaining SGR containment gap
and the public CLI compatibility regression.
