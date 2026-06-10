---
ready: false
agent: codex
model: ""
---

# Review 2

## Findings

### High: Wrapped and multiline cells still lack Level 2 verification

The specification requires wrapped and multiline styled cells to preserve
visible-width measurement and prevent style bleed into padding, separators,
borders, and adjacent rows
(`renderable/features/2026-06-08-prose-cells/spec.md:217`). The new Level 2
suite exercises styled single-line rows and the cursor-alignment path, but
every fixture remains one visual line
(`biscuit-terminal/cli/tests/level2_prose_cells.rs:90`,
`biscuit-terminal/cli/tests/level2_prose_cells.rs:120`,
`biscuit-terminal/cli/tests/level2_prose_cells.rs:153`).

The multiline/reset assertions remain Level 1
(`biscuit-terminal/lib/tests/prose_cells_parity.rs:327`,
`biscuit-terminal/lib/tests/prose_cells_parity.rs:354`). Under the review
rubric, terminal wrapping, glyph width, border geometry, and rendered style
containment require a real-terminal capture. Add Level 2 fixtures that force
word wrapping at a narrow pane/column width and that render an explicit
newline inside one styled run. Verify each visual line's text and border
geometry in `frame.plain`, and verify styled cell attributes without relying
on source reset bytes.

### High: Browser verification is excluded from the canonical lifecycle

The three computed-style tests are correctly named `browser_*`
(`biscuit-terminal/lib/tests/prose_cells_parity.rs:789`,
`biscuit-terminal/lib/tests/prose_cells_parity.rs:812`,
`biscuit-terminal/lib/tests/prose_cells_parity.rs:833`). The shared `_test`
recipe deliberately excludes `browser_*`, but the package's `test-browser`
recipe still reports that browser testing is not applicable
(`biscuit-terminal/justfile:63`, `biscuit-terminal/justfile:78`).
Consequently, `just test` and `just all` never execute the required
real-browser verification.

Wire `test-browser` to `just _test_browser biscuit-terminal` and update the
stale applicability comment. The tests pass when invoked directly, but an
ungated verification tier is not production-ready coverage.

### Medium: The Level 2 reset assertion depends on unstable capture bytes

`assert_prose_rows_independently_styled` requires the captured row to contain
a literal `ESC[0m`
(`biscuit-terminal/cli/tests/level2_prose_cells.rs:137`). The repository's
WezTerm harness guidance explicitly states that `get-text --escapes` may
rewrite a reset as separate foreground/background resets or elide it entirely
(`.claude/skills/rust-testing/wezterm-harness-pitfalls.md:23`).

This passed on the current host but can fail across WezTerm versions or cell
state transitions without an implementation regression. Assert the
user-visible state instead: inspect the following border/row cells for the
absence of bold, or use an ordering/attribute-transition assertion that
accepts the documented reset forms.

## Verification Levels

| Requirement | Strongest verification | Assessment |
|---|---|---|
| Conversion, structured projection, hints, code degradation, layout isolation | Level 1 | Appropriate |
| No-color output | Level 1 | Appropriate; both paths reject every escape byte |
| Single-line terminal styling and borders | Level 2 | Appropriate |
| Wrapped and multiline geometry/reset isolation | Level 1 | Gap; Level 2 required |
| Cursor-alignment visible content and styling | Level 2 | Appropriate |
| Resolve each Prose cell once | Level 1 instrumentation | Appropriate |
| Browser semantic markup and links | Level 1 | Appropriate structural coverage |
| Browser supported visual styles | Browser tier | Correct tests exist, but no canonical recipe runs them |
| Markdown and MarkdownPlus parity | Level 1 | Appropriate |
| Keyboard, modifier, paste, IME, or mouse behavior | Not applicable | No Level 3 requirement |

## Verification Run

- `cargo test -p biscuit-terminal --test prose_cells_parity`: 58 passed,
  including 3 real-Chrome browser tests.
- `cargo test -p renderable --lib`: 475 passed.
- `cargo check -p biscuit-terminal-cli -p biscuit-terminal -p renderable`:
  passed.
- `just -f biscuit-terminal/justfile test-l2`: 71 passed, including the new
  WezTerm, Kitty, and tmux Prose-cell tests.

The implementation closes the prior no-color, browser-style, resolution-count,
Markdown parity, API documentation, and clone-cost findings. Production
readiness remains blocked by the two verification lifecycle gaps above.
