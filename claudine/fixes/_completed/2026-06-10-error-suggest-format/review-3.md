---
ready: false
agent: codex
model: ""
---

# Review: StatusBlock Hint Formatting

## Findings

### High: rendered block quotes contain extra blank quoted rows

- Requirement: the body block quote must have one leading blank quoted row, body content, one blank separator row, and the italic hint inside the same block quote. The spec's target terminal shape is:

  ```text
  Header
  ┃
  ┃ Body
  ┃
  ┃ Hint
  ```

- Implementation: `StatusBlock::to_render_node()` inserts explicit empty `Paragraph` nodes before the body and before the hint at [status_block.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/biscuit-terminal/lib/src/components/status_block.rs:260) and [status_block.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/biscuit-terminal/lib/src/components/status_block.rs:267). The renderers also add paragraph separation between block children, so the externally visible output is over-spaced.
- Observed output from `bt status-block --severity error --header Header --hint "Verify the endpoint URL and retry" "Telemetry upload failed"`:

  ```text
  ⤫ Header

  ┃
  ┃
  ┃ Telemetry upload failed
  ┃
  ┃
  ┃
  ┃ Verify the endpoint URL and retry
  ```

- Markdown has the same defect: `bt status-block --md ...` renders two leading quoted blank lines before the body and three quoted blank lines before the hint. Browser output is structurally closer because empty paragraphs are real `<p></p>` nodes, but Terminal and Markdown miss the user-visible visual contract.
- Test gap: the new Level 2 test at [level2_status_block.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/biscuit-terminal/cli/tests/level2_status_block.rs:53) only asserts that the body and hint lines contain the border glyph. It does not assert the leading quoted blank row count or separator row count, so it passes while the rendered shape still differs from the spec. Existing Level 1 tests also inspect render-tree structure rather than rendered row counts for these separators.
- Verification level mismatch: this is user-observable terminal and Markdown layout. Level 1 render-tree checks are not enough here; the new Level 2 terminal capture should assert the exact relevant output region, including one leading quoted blank row and one quoted separator row.
- Fix direction: either adjust the render-tree projection/renderers so structural blank paragraphs lower to exactly one visible blank quoted row in Terminal and Markdown, or avoid combining explicit blank paragraph nodes with renderer-inserted paragraph separators. Add Level 1 rendered-output assertions for Terminal and Markdown row counts, and extend the Level 2 capture to fail on duplicated quoted blank rows.

## Coverage Notes

- The previous Markdown emphasis issue is fixed: the hint is now a semantic `Emphasis` child and Markdown asserts `_Fix this hint_`.
- The previous Level 2 terminal coverage gap is partially fixed: iteration 3 adds a real-terminal `bt status-block` test and verifies italic SGR through WezTerm.
- No Level 3 coverage is required; this feature does not involve keyboard input.

## Verification Run

- `cargo test --color=never -p biscuit-terminal-cli --test level2_status_block -- --nocapture` passed.
- `cargo test --color=never -p biscuit-terminal --test status_block_parity body_plus_hint -- --nocapture` passed.
- `cargo test --color=never -p claudine hint_appears_inside_block_quote_border -- --nocapture` passed.
- Manual CLI renders showed the extra quoted blank rows in both Terminal and Markdown output.

## Production Readiness

Not ready for production. The hint is now inside the quote and italicized, including Level 2 terminal SGR coverage, but the rendered spacing contract still differs from the specification and the new Level 2 test does not catch it.
