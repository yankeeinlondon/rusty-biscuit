---
ready: false
agent: codex
model: ""
---

# Review: StatusBlock Hint Formatting

## Findings

### High: the required leading quoted blank row was removed

- Requirement: whenever a `StatusBlock` has body content, the body block quote must start with a leading blank paragraph inside the block quote. The target shape in the spec is:

  ```text
  Header
  ┃
  ┃ Body
  ┃
  ┃ Hint
  ```

- Implementation: `StatusBlock::to_render_node()` now starts the body block quote directly with the body paragraph at [status_block.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/biscuit-terminal/lib/src/components/status_block.rs:262), and the bespoke terminal fallback similarly starts `composed_parts` with body content at [status_block.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/biscuit-terminal/lib/src/components/status_block.rs:385). There is no structural leading blank paragraph and no visible quoted blank row before the body.
- Observed Terminal output from `bt status-block --severity error --header Header --hint "Verify the endpoint URL and retry" "Telemetry upload failed"`:

  ```text
  ⤫ Header

  ┃ Telemetry upload failed
  ┃
  ┃ Verify the endpoint URL and retry
  ```

  Markdown has the same issue:

  ```text
  ⤫ Header

  > Telemetry upload failed
  >
  > _Verify the endpoint URL and retry_
  ```

- Test gap: the current tests now assert the opposite of the spec. `every_status_state_renders_body_with_body_paragraph_as_first_child` expects the first block-quote child to be the body paragraph at [status_block_parity.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/biscuit-terminal/lib/tests/status_block_parity.rs:473), and `terminal_body_only_has_no_blank_rows_before_body` / `markdown_body_only_has_one_quoted_row` assert body-only output has no leading quoted blank row at [status_block_parity.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/biscuit-terminal/lib/tests/status_block_parity.rs:863) and [status_block_parity.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/biscuit-terminal/lib/tests/status_block_parity.rs:901). The Level 2 tmux test checks the body/hint separator but not the required leading quoted blank row before the body at [level2_status_block.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/biscuit-terminal/cli/tests/level2_status_block.rs:83).
- Verification level mismatch: this is user-observable terminal layout, so it needs Level 2 coverage that captures the real terminal pane and asserts exactly one quoted blank row before the body. Level 1 render-tree tests should also assert the structural empty first paragraph because the spec makes the render tree the canonical contract.
- Fix direction: restore the leading blank paragraph in the canonical projection and bespoke fallback, then adjust the terminal/Markdown lowering so that structural blank paragraphs render as exactly one quoted blank row rather than duplicating separator rows. Update the Level 1 and Level 2 tests to assert both row counts: one leading quoted blank row before the body and one separator row between body and hint.

## Coverage Notes

- The previous duplicated body/hint separator is fixed: current Level 1 and Level 2 tests verify one quoted separator row between body and hint.
- The hint is inside the block quote for Terminal, Markdown, Browser, and Claudine composition-error regressions.
- Italic coverage is present at Level 1 and Level 2: component tests assert SGR/emphasis/HTML behavior, and the WezTerm Level 2 test verifies raw italic SGR.
- No Level 3 coverage is required; this feature does not involve keyboard input, mouse input, paste, IME, or terminal encoder behavior.

## Verification Run

- `cargo test --color=never -p biscuit-terminal-cli --test level2_status_block -- --nocapture` passed.
- `cargo test --color=never -p biscuit-terminal --test status_block_parity terminal_body_plus_hint_has_exactly_one_blank_separator_row -- --nocapture` passed.
- `cargo test --color=never -p biscuit-terminal --test status_block_parity markdown_body_plus_hint_has_exactly_one_blank_separator_row -- --nocapture` passed.
- `cargo test --color=never -p claudine hint_appears_inside_block_quote_border -- --nocapture` passed.
- Manual Terminal and Markdown renders showed the missing leading quoted blank row.

## Production Readiness

Not ready for production. The hint placement, separator, and italic behavior are now covered well enough, but the implementation still misses the spec's required leading blank paragraph/row for every body block.
