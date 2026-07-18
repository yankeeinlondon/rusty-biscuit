---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-17T17:14:51-07:00
spec: 2026-07-15-performance-followup/spec.md
implemented: true
implemented_by: claude/default
log: 2026-07-15-performance-followup/log.md
description: "A **feature** review of `2026-07-15-performance-followup/spec.md`"
feature: 2026-07-15-performance-followup/review-6.md
previous: 2026-07-15-performance-followup/review-5.md
---

# Review 6 — Performance Follow-up

## Verdict

This feature is **not ready for production**. Both implementation findings from review 5 are closed, but the integrated compose-regression threshold remains unestablished. The specification and performance-compliance ledger explicitly leave acceptance criteria 5 and 6 open until reproducible, admissible benchmark runs can determine whether the residual regression is within the 5% limit.

## Tracked Production-Readiness Blocker

The integrated compose-regression threshold is still unestablished. This is not a new review finding: the specification requires later implementation reviews to leave the deferred performance gap in `performance-compliance.md` rather than carry it forward as another finding.

The implementation evidence does not yet support a pass or fail against the feature's 5% integrated compose threshold. The accepted comparison reported a 0.76% regression, but it ran under host load between 5.42 and 7.16; the reconstructed-pin control showed 3.18% A/A drift; and the same base binary varied by 8.2% across runs. Those measurements are too noisy to adjudicate a 5% gate.

`performance-compliance.md` consequently still requires two independent quiet-host runs using committed base and after pins, all six benchmark cases, A/A drift below 1%, base means within 1.5%, and a recorded `compose_trivial` verdict. Until those predeclared conditions are satisfied, acceptance criteria 5 and 6 remain open and `ready` must remain `false`.

## Findings

### Low — The terminal-detection test documentation names the wrong evidence file and tier

The module comment in `darkmatter/cli/tests/compose_terminal_detection.rs` says that interactive OSC evidence lives in `biscuit-terminal`'s `level2_terminal_osc_cache.rs`. That cache test is now Level 1, while the genuine WezTerm Level-2 evidence lives in `level2_terminal_osc_wezterm.rs`. The implementation and test classification in the feature results are correct; the comment has drifted and should be updated to point readers to the actual Level-2 test, optionally naming the Level-1 cache test separately.

## Review 5 Closure

### Hash orchestration seam — closed

The `internal-hash-orchestration` feature and public `internal` module are absent from both package manifests and the library API. `Markdown::diff_hash` and `diff_with_computed` remain private, and the CLI now composes the public `compare_hash`, `plan_hash_save`, and `explain_hash_diff` operations directly. Cargo metadata exposes no replacement feature or callable internal seam.

Focused library hash tests passed, and all 16 spawned-CLI cases in `hash_kind_save_diff` passed, including persisted output, malformed-file behavior, and the documented exit codes.

### Parked-waiter notification proof — closed

The regression test now observes a test-only parked-waiter counter that is incremented under the same mutex immediately before `wait_timeout_while`. The handler is not released until the peer is known to be parked. Production synchronization remains unchanged.

As a mutation check, removing `notify_all()` made the focused test fail because the waiter remained parked through the notification budget. Restoring the notification made the same test pass in 1.077 seconds with retries disabled. This proves the test depends on the notification path rather than scheduler timing.

## Requirement Verification Levels

| User-observable requirement | Strongest verification | Assessment |
| --- | --- | --- |
| F2: repeated constructions share one OSC query and preserve cache behavior | Level 2 in real WezTerm on macOS, retained real Kitty evidence on Linux, plus Level-1 query-count tests | Appropriate. The terminal emulator's OSC behavior is exercised; Level 3 is not needed because no OS keyboard event is involved. |
| F3: verbose/performance/warning paths perform one terminal-detection event | Level 1 spawned-CLI test with event counting | Appropriate for process behavior that does not depend on terminal rendering or input encoding. |
| F21: fully redirected macOS output does not launch the appearance probe | Level 1 spawned-CLI test with a `defaults` sentinel; the interactive detection path is covered by F2's Level-2 evidence | Appropriate. |
| Hash save/diff output, persistence, and exit status | Level 1 spawned-CLI integration tests | Appropriate; these semantics do not depend on a terminal emulator. |
| Parked peers are notified when the allow-once handler fails | Level 1 deterministic concurrency test plus a failing notification mutation | Appropriate; this is an in-process synchronization contract. |

The feature introduces no keypress, hotkey, paste, IME, or mouse requirement, so Level-3 OS keyboard injection is not applicable. No user-observable requirement reviewed here has only a lower test level than its behavior requires.

## Verification Performed

- GitNexus impact analysis classified `complete_allow_once`, `run_hash_diff`, `run_hash_save`, `diff_hash`, and `plan_hash_save` as low risk; the CLI entry points affect only their expected command-dispatch flows.
- `sniff` identified `darkmatter`, `darkmatter-cli`, and `dmls` as the affected package area scope.
- Four focused library hash tests passed.
- All 16 `darkmatter-cli` `hash_kind_save_diff` integration tests passed.
- The focused parked-waiter test passed after restoration, and failed under the notification-removal mutation.
- `just build`, `just test`, and `just lint` passed for the complete `darkmatter` package area. This included 5,762 library tests, 559 CLI tests, and 566 DMLS tests, with the repository's configured skips.
- `md get` read back every required review, previous-review, and specification frontmatter value exactly. `md schema validate` accepted the specification but could not adjudicate either review because the repository's existing `schemas/feature-review.yaml` is itself rejected as a standalone schema: its `$schema` and `description` keys are unsupported by the current validator. This is schema-infrastructure drift rather than a failure in the feature implementation.

Level-2 and browser suites were not rerun because the review-5 changes affect private hash composition and test-only synchronization, not terminal or browser behavior. Their existing evidence remains applicable.

## Production Readiness

**Not ready.** Complete the tracked performance-compliance work with admissible benchmark evidence before marking this feature production-ready. The low-severity documentation drift should also be corrected, but it does not independently block release.
