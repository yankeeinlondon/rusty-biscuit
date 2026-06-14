---
ready: false
agent: codex
model: ""
---

# Review: 2026-06-04 dry-run touchup, iteration 3

## Verdict

Not ready for production.

The iteration 3 patch fixes the prior dry-run sequence gap: `sequence --dry-run` now avoids the legacy non-TTY resolver and the new L1 tests prove no-agent, scalar invalid, scalar not-installed, and zero-installed-list states render per step without launching a provider.

One high-severity live-path gap remains. The spec is explicitly full-scope: the dry-run table must faithfully predict real non-`--dry-run` behavior, and no-TTY prompting states must emit the same styled message and abort. The direct `compose` path uses the new `AgentResolutionState` live gate, but `sequence` still does not.

## Findings

### High: live `sequence` still auto-runs a provider for unresolved/invalid agent states in no-TTY mode

Spec requirements:

- No agent at all, invalid scalar agent, not-installed scalar agent, list with two or more installed agents, and zero-installed-list are prompting states.
- In no-TTY mode, each prompting state must emit the same styled message the TTY path would show to stderr and abort with a structured non-zero exit.
- Do not auto-pick a substitute agent, and do not fall back to a generic/raw resolver error.
- The real live path must match what dry-run predicts.

Implementation issue: the non-dry-run sequence path still resolves each step through the old non-TTY resolver at [sequence.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/src/commands/wrap/sequence.rs:216). That resolver ignores `agent_invalid` and falls back to the configured favorite/default when `hints.agent` is `None` at [select.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/lib/src/composition/select.rs:220). The later non-TTY success branch then picks the default provider from the picker plan at [sequence.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/src/commands/wrap/sequence.rs:320).

Observed behavior: with a temp HOME, only a fake `claude` on PATH, and a sequence document containing `agent: not-real`, `claudine sequence compose.md` exited `0`, launched the fake provider for both steps, and reported `via Claude`. A no-agent sequence similarly launched Claude and exited `0`. Both should have aborted before provider execution with the styled `Invalid Agent:` / no-agent message on stderr.

Impact: `sequence --dry-run` now tells the user an unresolved/invalid state would prompt or abort, but the real `sequence` command can silently run a provider instead. This is the exact drift the feature scope was intended to prevent.

Test level: the strongest sequence tests added in this iteration are L1 dry-run tests at [wrap_commands.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/tests/wrap_commands.rs:4810). There is no L1 live sequence coverage for no-agent, invalid, not-installed, multi-installed-list, zero-installed-list, no-TTY abort messaging, or `--silent` not changing agent-resolution reporting. Under the review rubric, that is a high-severity verification mismatch for user-observable behavior.

Recommended fix: route non-dry-run sequence provider selection through the same classified live gate as direct compose, or factor a shared helper that returns either a resolved target or an `AgentResolutionFailed` state per the TTY-only gate. Add L1 CLI tests proving live no-TTY sequence aborts without launching a provider for each prompting state, plus TTY/planner tests for sequence picker scope.

## Notes

The prior dry-run-specific sequence finding appears resolved. I ran:

```text
cargo test --color=never -p claudine-cli sequence_dry_run_ --test wrap_commands
```

Result: 7 passed.

I also manually reproduced the live-path gap with temp HOME/PATH fixtures as described above.
