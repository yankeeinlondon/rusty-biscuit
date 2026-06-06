---
ready: false
agent: codex
model: ""
---

# Review: 2026-06-04 dry-run touchup, iteration 4

## Verdict

Not ready for production.

The previous live `sequence` no-TTY drift has been fixed: the new `sequence_live_*` tests prove no-agent, scalar invalid, scalar not-installed, zero-installed-list, auto-selectable, and `--silent` cases no longer silently launch a fallback provider in a piped/no-TTY session.

One high-severity live TTY gap remains. The direct `compose` path keys the agent prompt gate off `stderr`, as required by the spec, but `sequence` still gates on `stdin && stdout` and then jumps straight into the sequence review screen without emitting the state-specific pre-prompt message.

## Findings

### High: live `sequence` still uses the wrong TTY gate and skips the required TTY pre-prompt message

Spec requirements:

- The agent re-prompt gate is TTY presence only and must key off the prompting/status channel, `stderr`, not `stdout`.
- For invalid scalar agents, TTY runs must emit the styled `Invalid Agent:` message, then prompt.
- For zero-installed-list states, TTY runs must emit the styled zero-installed-list message, then prompt over all installed agents.
- The live path must match the dry-run table's prediction.

Direct `compose` does this correctly: [composition/mod.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/src/commands/wrap/composition/mod.rs:524) passes `std::io::stderr().is_terminal()` into the live resolver.

`sequence` does not. It computes `is_tty` as `stdin && stdout` at [sequence.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/src/commands/wrap/sequence.rs:181), and the prompting-state abort gate uses that value at [sequence.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/src/commands/wrap/sequence.rs:225). In a normal CLI pattern like `claudine sequence doc.md > out.md` with `stderr` still attached to the terminal, the spec says the prompt can be shown; this implementation treats it as no-TTY and aborts instead.

The TTY branch also calls `review_sequence` directly at [sequence.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/src/commands/wrap/sequence.rs:333). `review_sequence` immediately runs the input table at [selection_ui.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/src/commands/wrap/selection_ui.rs:76) and has no hook for the state-specific message. That means invalid-agent and zero-installed-list sequence runs do not show the same `Invalid Agent:` / zero-installed-list pre-prompt message that direct compose shows and that dry-run predicts.

Test level: the new L1 sequence tests at [wrap_commands.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/tests/wrap_commands.rs:4975) cover no-TTY behavior by piping stdin, but there is no L1 PTY-style coverage for the `stderr`-TTY/stdout-redirected case, and no TTY/planner coverage proving invalid or zero-installed-list sequence states emit the required pre-prompt before the picker/review UI. Under the review rubric, this is a high-severity verification mismatch for user-observable live agent-resolution behavior.

Recommended fix: make `sequence` use the same `stderr().is_terminal()` gate as direct compose for agent resolution, and share the pre-prompt emission helper or equivalent message rendering before entering `review_sequence` for states that require a TTY message. Add L1 coverage for `stderr` TTY with redirected stdout, plus a focused test that invalid and zero-installed-list sequence states route through the same pre-prompt message path as direct compose.

## Verification

I ran:

```text
cargo test --color=never -p claudine-cli sequence_live_ --test wrap_commands
cargo test --color=never -p claudine-cli sequence_dry_run_ --test wrap_commands
```

Results: 6 live sequence tests passed; 7 sequence dry-run tests passed.
