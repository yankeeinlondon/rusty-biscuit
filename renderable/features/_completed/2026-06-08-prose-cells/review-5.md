---
ready: false
agent: codex
model: ""
---

# Review 5

## Findings

### High: OSC8 coverage does not prove hyperlink containment or the cursor-alignment path

The new Level 2 link assertion only checks that the captured row contains an
OSC8 opener for the expected destination
(`biscuit-terminal/cli/tests/level2_prose_cells.rs:455`). It does not track the
OSC8 state across visible cells or require the link to close before trailing
padding and the border. A regression that leaves the rest of the cell or its
border clickable would pass this test.

The same fixture only runs without `--cursor-align`
(`biscuit-terminal/cli/tests/level2_prose_cells.rs:445`). The bespoke path
resolves `StyledProse` through `Prose::render(term)` before planning, which is a
different implementation boundary from canonical tree reconstruction. Its
Level 2 fixture verifies bold SGR, but no test verifies OSC8 output or
containment through that path.

This remains a wrong-level gap for the user-visible requirements that links
resolve through the active terminal and that both terminal paths preserve
styled content. Extend the raw-row state parser to track OSC8 open/close state,
assert that only the link label is active, and run the fixture through both the
standard and cursor-alignment paths in Kitty and WezTerm.

### Medium: Typed test columns break existing colon-bearing CLI headers

`parse_column_spec` treats every colon as a type delimiter and rejects any
unrecognized suffix (`biscuit-terminal/cli/src/commands/table.rs:30`). Before
this iteration, `--columns` accepted arbitrary header text other than the
comma separator. A command such as:

```text
bt table --columns "Time: Value" --row "12:00"
```

now fails with `unknown column type "value"`. This is a public CLI
compatibility regression introduced solely to manufacture mixed typed cells
for Level 2 coverage.

Preserve bare header behavior by moving test typing to a separate option, or
only consume an explicitly recognized suffix while retaining the full input as
the header otherwise. Add CLI tests for both typed columns and literal
colon-bearing headers.

## Verification Levels

| Requirement | Strongest verification | Assessment |
|---|---|---|
| Conversion, projection, hints, code degradation, layout isolation | Level 1 | Appropriate |
| No-color output | Level 1 | Appropriate |
| Bold, color, and dim in terminal cells | Level 2 | Appropriate |
| OSC8 link presence in a terminal cell | Level 2 | Partial; opener only |
| OSC8 containment and cursor-alignment link behavior | Level 1 / absent | Gap; needs Level 2 |
| Wrapped and multiline geometry/style containment | Level 2 | Appropriate |
| Cursor-alignment visible content and bold styling | Level 2 | Appropriate |
| Mixed typed formatting and alignment, standard and cursor paths | Level 2 | Appropriate |
| Resolve each Prose cell once | Level 1 instrumentation | Appropriate |
| Browser semantic markup and links | Level 1 | Appropriate |
| Browser supported visual styles | Browser tier | Appropriate |
| Markdown and MarkdownPlus parity | Level 1 | Appropriate |
| Keyboard, modifier, paste, IME, or mouse behavior | Not applicable | No Level 3 requirement |

## Verification Run

- `cargo test -p biscuit-terminal --test prose_cells_parity --color=never`:
  58 passed.
- `cargo check -p biscuit-terminal-cli -p biscuit-terminal -p renderable
  --color=never`: passed.
- A direct `level2_prose_cells` run passed Kitty and tmux; WezTerm failed while
  spawning its shell because `openpty` returned `Device not configured`.
- Canonical `just -f biscuit-terminal/justfile test-l2 prose_cells` passed
  Kitty, then failed because tmux could not create a session; nextest canceled
  WezTerm after that failure.
- Manual CLI reproduction confirmed that `--columns "Time: Value"` now errors.

Iteration 5 closes review 4's dim and mixed typed-row geometry findings, but
production readiness remains blocked by incomplete OSC8 containment/path
verification and the newly introduced CLI compatibility regression.
