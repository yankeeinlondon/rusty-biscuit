---
ready: false
agent: codex
model: ""
---

# Review: 2026-06-04 dry-run touchup, iteration 2

## Verdict

Not ready for production.

The direct `compose` / `inline-compose` path appears to have addressed the prior review's main gaps: dry-run reclassifies non-explicit resolved targets from the installed snapshot, single-entry invalid lists preserve list-ness, no-TTY message bodies share canonical text, and the live picker scope has L1 planner coverage.

One high-severity gap remains in the sequence path. The spec explicitly includes sequence dry-run formatting and the same agent-resolution dry-run table behavior, but `claudine sequence --dry-run` still performs older non-TTY provider resolution before any dry-run render can happen.

## Findings

### High: `sequence --dry-run` still aborts or auto-resolves before rendering unresolved agent states

Spec requirements:

- Under `--dry-run`, no picker may fire and unresolved, invalid, and not-installed agents must be captured as render states, not prompts or errors.
- Sequence dry-run is in scope: each prompt is rendered with dry-run metadata and separated by horizontal rules.
- The dry-run table must be a faithful prediction of live agent-resolution behavior for every state.

Implementation issue: `run_sequence` still resolves each step with `resolve_target_non_tty_with_hints` before it reaches the dry-run execution seam. See [sequence.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/src/commands/wrap/sequence.rs:195), where `provider_result` is computed through the old non-TTY resolver, and [sequence.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/src/commands/wrap/sequence.rs:273), where any failure immediately becomes `SequenceSelectionFailed`.

That older resolver does not return the new `AgentResolutionState`; it either auto-selects a provider or returns legacy selection errors. See [select.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/lib/src/composition/select.rs:169). This means sequence dry-run cannot render the required `NoAgent`, `SingleInvalid`, `SingleNotInstalled`, `ListMultipleInstalled`, or `ZeroInstalledList` metadata state when resolution fails before `execute_composition_request_inner` reaches its dry-run branch at [mod.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/src/commands/wrap/composition/mod.rs:819).

Impact: `claudine sequence --dry-run compose.md` without an explicit provider still requires the old resolver to succeed. For example, a no-agent sequence with no configured favorite, or a scalar invalid `agent`, fails as sequence selection before dry-run can show the metadata table state the spec requires. This is a user-facing dry-run prediction gap, not just missing test coverage.

Test level: current sequence dry-run integration tests use an explicit provider (`--goose`), so they verify horizontal-rule/body behavior but not the L1 agent-resolution states for sequence dry-run. Add L1 sequence dry-run tests for at least no-agent, single-invalid, single-not-installed, and zero-installed-list without an explicit provider.

## Notes

The prior review's direct-path findings look resolved by the current code and tests. I did not find a remaining L2 styling gap: the metadata capture tests now include real-terminal assertions for red invalid-agent styling, yellow/dim not-installed styling, visible horizontal-rule rendering, inverse-theme YAML, and heading spacing.

## Verification Performed

Reviewed the spec, prior review, implementation, and relevant L1/L2 tests. I did not run the full test suite in this review pass.
