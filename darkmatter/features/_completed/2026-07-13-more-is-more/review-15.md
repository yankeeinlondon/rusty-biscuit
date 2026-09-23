---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-18T00:50:01-07:00
spec: 2026-07-13-more-is-more/spec.md
implemented: true
implemented_by: claude/default
log: darkmatter/features/2026-07-13-more-is-more/log.md
description: "A **feature** review of `2026-07-13-more-is-more/spec.md`"
feature: 2026-07-13-more-is-more/review-15.md
previous: 2026-07-13-more-is-more/review-14.md
next: 2026-07-13-more-is-more/review-16.md
---

# Review 15

## Summary

The feature is **not production ready**. The implementation and execution plan cover only the Git context and conflict-prediction subset in acceptance criteria 1–16, while the current specification has 30 acceptance criteria. The indexed-file functions, object/array expression literals, preferred-remote selection, remote branch/vendor functions, PR and CI/CD query surfaces, network policy, and enum return descriptors have no implementation or tests.

The implemented subset passes the focused macOS Level-1 and lint gates, but it still contains two correctness gaps: unsafe merge configuration is checked only after the built-in merge reports conflicts, and bare repositories are treated as discovery failures before valid branch state can be captured.

## Findings

### Critical: Acceptance criteria 17–30 are not implemented

The specification requires 30 acceptance criteria ([spec.md:1611](spec.md#L1611)), including the indexed-file functions and expression literals at AC17–18, the remote resolver/functions at AC19–21, PR and CI/CD APIs/functions at AC22–25, and their network, error, catalog, DMLS, and enum-return contracts at AC26–30 ([spec.md:1670](spec.md#L1670)). None of the defining names for these surfaces exists in the Darkmatter, DMLS, or Sniff implementation: `find_first_index`, `find_last_index`, `branch_exists_on_remote`, `remote_vendor`, `pr_list`, `cicd_list`, object/array literal AST nodes, and a catalog enum-return representation are all absent.

The execution plan explains the mismatch. It narrows the feature to three Git context variables and `predict_conflicts` ([plan.md:113](plan.md#L113)), declares success against “all 16” criteria ([plan.md:120](plan.md#L120)), and closes only AC1–AC16 ([plan.md:415](plan.md#L415)). The current spec's second half therefore never entered the implementation plan.

Recommendation: either implement and verify AC17–30 or split the specification so this delivery is explicitly the Git-context/conflict-prediction phase and the remaining contracts live in separately scheduled features. Do not mark the current 30-criterion feature complete while those public surfaces are absent.

Strongest verification present: **none** for AC17–30. These requirements need Level 1 in-process/runtime tests, including Wiremock-backed network tests where specified. L2 and L3 are not applicable because none of these contracts depends on real-terminal rendering or terminal keyboard encoding.

### High: Unsafe merge settings are silently accepted when the built-in merge is clean

`merge_conflicts_between` performs the hermetic built-in merge first and only then calls `reject_unsafe_configuration` with the conflicts produced by that approximation ([merge_conflicts.rs:74](../../../sniff/lib/src/filesystem/git/merge_conflicts.rs#L74)). The rejection helper immediately returns success when that conflict list is empty, and it inspects only `conflict.ours.location()` for non-empty results ([merge_conflicts.rs:170](../../../sniff/lib/src/filesystem/git/merge_conflicts.rs#L170)).

This violates the spec's hermetic contract: applicable external drivers/filters and `merge.renormalize` must be rejected because ignoring them can change whether a merge conflicts ([spec.md:1743](spec.md#L1743)). For example, a divergent merge that is clean under the built-in text driver with `merge.renormalize=true` currently returns a clean prediction instead of `UnsupportedMergeConfiguration`. A custom driver or filter on a participating path is similarly missed whenever the built-in approximation does not already conflict on that path.

The existing test only applies each unsafe setting to `shared.txt` in a fixture that already produces a built-in content conflict ([merge_conflict_prediction.rs:476](../../../sniff/lib/tests/merge_conflict_prediction.rs#L476)), so it cannot catch this false-clean path.

Recommendation: determine the committed paths participating in the merge and reject applicable executable behavior before accepting the prediction. Treat global `merge.renormalize` as unsupported before the merge result can be returned. Add regression fixtures in which the built-in merge is clean but a committed custom merge/filter attribute or renormalization setting is applicable; assert an error and unchanged repository/process/network state.

Strongest verification present: **Level 1**, but only for unsafe settings on an already-conflicting path. Level 1 is the appropriate tier; the missing false-clean cases leave the safety requirement unverified.

### High: Bare-repository Git capture discards valid branch state and emits a discovery diagnostic

`GitRepo::discover` converts a successfully discovered bare repository into `NotARepository` because it requires `gix.workdir()` ([types.rs:596](../../../sniff/lib/src/filesystem/git/types.rs#L596)). Darkmatter catches that as a Git discovery failure, records a partial-capture diagnostic, and skips all three Git field probes ([snapshot.rs:81](../../lib/src/markdown/compose/context/capture/snapshot.rs#L81)).

That behavior is broader than the spec's neutral bare-worktree rule. `ctx.worktree` must be null for a bare repository, but `ctx.branch` must return any attached local branch and is only neutral outside a repository, at unborn HEAD, or at detached HEAD ([spec.md:1621](spec.md#L1621)). A committed bare repository with symbolic HEAD therefore loses a valid branch value, and the whole Git group degrades even though only worktree/index-dependent values need neutral handling.

The current bare-repository integration assertion checks only that `ctx.worktree` is null; it does not assert the branch or diagnostics ([git_context_integration.rs:197](../../lib/tests/git_context_integration.rs#L197)). It consequently passes whether the repository was handled correctly or rejected during discovery.

Recommendation: represent a discovered bare repository as a valid `GitRepo` with an optional worktree/repository root appropriate to each API, or provide a capture-specific discovery path that preserves HEAD queries. Add a committed bare fixture with attached HEAD and assert `ctx.branch`, null `ctx.worktree`, empty conflicts, and no spurious discovery diagnostic.

Strongest verification present: **Level 1**, but only for the neutral `ctx.worktree` projection. Level 1 is the appropriate tier; the branch and diagnostic behavior is not verified.

## Verification Matrix

| Requirement group | Required level | Strongest present | Result |
|---|---:|---:|---|
| AC1–16: Git context and conflict prediction | Level 1 | Level 1 | Partial: focused suites pass, but unsafe false-clean and bare-branch cases fail review |
| AC17: indexed-file endpoint functions | Level 1 | None | Missing |
| AC18: object/array expression literals | Level 1 | None | Missing |
| AC19–25: remote selection, branch/vendor, PR, and CI/CD APIs/functions | Level 1, with Wiremock for network behavior | None | Missing |
| AC26–30: surface parity, network policy, errors, catalog/DMLS, enum returns | Level 1 | None | Missing |

No requirement in this feature depends on terminal glyph rendering, scrolling, mouse/paste/IME behavior, or a terminal emulator's keyboard encoder. Level 2 and Level 3 tests are therefore not required.

## Checks Run

The following focused macOS gates passed during this review:

```text
cd sniff && just test
cd sniff && just lint
cd darkmatter && just test
cd darkmatter && just lint
```

These are Level-1 verification. The repository's CI definitions provide macOS, Linux, and Windows compile/test matrices for the affected Sniff and Darkmatter areas, but this local review did not independently execute non-macOS hosts.

## Production Readiness

**Not ready for production.** Fourteen of 30 acceptance criteria have no implementation or verification, and the shipped Git subset has two high-severity correctness gaps. Passing Level-1 area suites and lint gates does not compensate for missing public functionality or tests that cannot exercise the unsafe false-clean and bare-branch cases.
