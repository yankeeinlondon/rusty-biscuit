---
ready: false
agent: codex
model: ""
---

# Review: 2026-06-04 dry-run touchup, iteration 5

## Verdict

Not ready for production.

The iteration fixes the prior `sequence` live-path drift for no-TTY prompting states: `stderr` is now the TTY gate, invalid/zero-installed TTY runs emit the pre-prompt before the review UI, and the no-TTY live tests prove prompting states no longer silently launch a fallback provider.

One high-severity live-path mismatch remains for `sequence`: auto-selectable agent states still prompt in TTY sessions, even though the spec says those states are selected silently and never enter the prompt gate.

## Findings

### High: live `sequence` prompts in TTY for auto-selectable agent states

Spec requirements:

- A list with exactly one valid+installed agent is silently auto-selected; no prompt is shown.
- A scalar valid+installed frontmatter agent is already selected and should not be routed into interactive selection.
- The dry-run table must be a faithful prediction of the live path.

Direct compose implements that contract: `resolve_live_target_with_tty` returns immediately for `Selected` and `ListOneInstalled` before checking `is_tty` or calling the picker ([composition/mod.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/src/commands/wrap/composition/mod.rs:549)).

`sequence` computes the same resolved provider for those states ([sequence.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/src/commands/wrap/sequence.rs:276)), but then sends every TTY run without an explicit provider into `review_sequence` ([sequence.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/src/commands/wrap/sequence.rs:337)). The deterministic auto-select path is only used in the `else` branch ([sequence.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/src/commands/wrap/sequence.rs:368)), so it is skipped precisely when stderr is a terminal.

That means `claudine sequence doc.md` in a normal terminal still opens the review/picker UI for:

- `agent: claude` when `claude` is installed.
- `agent: [claude, gemini]` when only `claude` is installed.

The dry-run Agent cell for the second case says the selected agent "will be used without the need for interactive prompting", but the live TTY path still prompts. That is a user-observable behavior mismatch.

Test level: current L1 coverage proves the no-TTY auto-selectable case launches (`sequence_live_auto_selectable_launches_provider`), but there is no L1 PTY/TTY test proving auto-selectable sequence states skip the review UI. The new PTY tests cover invalid, zero-installed, and stdout-redirected prompting states ([level2_schema_prompt_pty.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/tests/level2_schema_prompt_pty.rs:1013), [level2_schema_prompt_pty.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/tests/level2_schema_prompt_pty.rs:1074), [level2_schema_prompt_pty.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/tests/level2_schema_prompt_pty.rs:1155)), but not the silent auto-select contract.

Recommended fix: make the sequence TTY branch mirror direct compose's split. If `shared_state` is `Selected` or `ListOneInstalled`, bypass `review_sequence` and build `ResolvedExecutionTarget`s from `draft.resolved_provider`; only prompting states should enter the review/picker path. Add a PTY-style L1 test that stages an installed provider, uses a one-installed-list frontmatter hint, and asserts the provider launches without the review UI marker appearing or requiring keyboard input.

## Verification

I ran:

```text
cargo test --color=never -p claudine-cli sequence_live_ --test wrap_commands
cargo test --color=never -p claudine-cli sequence_dry_run_ --test wrap_commands
cargo test --color=never -p claudine-cli level2_pty_sequence_ --test level2_schema_prompt_pty
```

Results: all three focused runs passed. They do not cover the remaining TTY auto-select/no-prompt requirement above.
