---
ready: false
agent: codex
model: ""
---

# Review 8

## Findings

### Medium: The traversal inventory still describes the superseded OSC8 assertion

The authoritative traversal artifact says the Level 2 companion compares the
link's visible representation
(`renderable/features/2026-06-06-tree-closeout/traversal-inventory.md:180-184`).
The current test intentionally proves the opposite: the URL must be absent from
visible text, and OSC8 URI metadata is extracted from the escaped WezTerm
capture and compared directly
(`darkmatter/lib/tests/level2_render_tree_terminal.rs:1754-1778`).

This is stale closeout evidence in an artifact that acceptance criterion 9
requires to describe the final architecture accurately. Update the inventory to
state that both variants render inside the WezTerm pane, the matched policy is
shown to apply by the table-position difference, foreground-color sets match,
and captured OSC8 opener metadata is present and byte-identical.

## Verification Levels

| Requirement | Strongest present verification | Assessment |
|---|---|---|
| Matched layout policy does not alter unrelated color/OSC8 capabilities | Level 1 capability-signature parity | Appropriate regression coverage |
| Same capability parity through a real terminal | Level 2 WezTerm render-in-pane with foreground-color and OSC8 metadata comparison | Appropriate and passing |
| The matched policy actually applies | Level 2 captured table-position difference | Appropriate and non-vacuous |
| Page-frame width independence from unmatched policy | Level 1 discriminating parity | Appropriate |
| Terminal HR appearance and layout | Level 2 real-terminal capture | Appropriate |
| Browser HR and component layout/style | Browser computed style/geometry | Appropriate |
| Markdown/MarkdownPlus degradation | Level 1 | Appropriate |
| Keyboard, modifier, paste, IME, or mouse behavior | Not applicable | No Level 3 requirement |

## Focused Verification

- `cargo metadata --no-deps --format-version 1`: `darkmatter` exposes no
  `dm_l2_render_probe` production binary; the review-7 blocker is resolved.
- `cargo test -p darkmatter --test level2_render_tree_terminal level2_render_probe_entrypoint --color=never`:
  passed.
- `cargo test -p darkmatter --lib terminal_matched_layout_policy_does_not_change_unrelated_capabilities --color=never`:
  passed.
- `BISCUIT_TEST_LEVEL_REQUIRED=2 just -f darkmatter/justfile test-l2 level2_matched_layout_policy_matches_no_policy_capabilities_in_real_terminal`:
  the named Darkmatter Level 2 test passed in WezTerm; the CLI package had no
  matching test.
- `git diff --check` and `git diff --cached --check`: clean.

The production-binary and real-terminal verification defects from reviews 7
and 6 are fixed. The feature remains not ready until the required closeout
documentation matches the implemented verification.
