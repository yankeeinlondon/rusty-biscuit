---
$schema: feature-review.yaml
ready: false
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/default
created: 2026-09-25T15:13:42-07:00
spec: 2026-09-24-ux-improvements/spec.md
implemented: true
implemented_by: claude/opus
log: worktree/fixes/2026-09-24-ux-improvements/implementation-log.md
description: "A **fix** review of `2026-09-24-ux-improvements/spec.md`"
fix: 2026-09-24-ux-improvements/review-9.md
previous: 2026-09-24-ux-improvements/review-8.md
next: 2026-09-24-ux-improvements/review-10.md
findings:
    - title: Graph concurrency test fails under valid thread scheduling
      priority: medium
---

# Review 9: Worktree UX improvements

## Verdict

**Not ready for production.** The executable-bit removal defect from review #8 is fixed, and all five added regressions passed. This review found one unstable test in the redesigned list pipeline: the full Level 1 run failed because the test assumes an execution order between independent threads. This is a test defect, not evidence that the list command has stopped running its work concurrently.

## Previous findings

| Review #8 unblocked finding | Assessment |
| --- | --- |
| Executable-bit changes after approval escape the removal handoff check | Implemented. Regular-file fingerprints now include the owner-execute bit on Unix, both for individual dirty files and for files inside dirty directories. |

In the `worktree` package, [entry_digest](../../lib/src/remove/inventory.rs:254), which records the working content approved for deletion, now includes the mode supplied by [file_mode](../../lib/src/remove/inventory.rs:276). It reuses metadata already read, adds no Git subprocess, and leaves timestamps and unrelated permission bits out of the comparison. Exact-byte path identity and the rule against following symlinks remain intact. The fingerprint and helper comments were updated with the behavior.

The library's [executable-bit regressions](../../lib/src/remove/inventory.rs:629) cover an already-modified tracked file, a file inside an untracked nested repository, stability without a change, and permission changes Git does not record. The `worktree-cli` package's [handoff regressions](../../cli/tests/remove.rs:1023) exercise both invocations: a mode-only change refuses with exit 3 and preserves file contents, executable permissions, worktree registration, and branch; unchanged mode allows removal. All five new tests executed successfully on this host.

Review #8 had no blocked findings or pending human decisions. Nothing needed to be unblocked. The separately scheduled ignored-file policy and performance work are not additional requirements of this review.

## Unblocked Findings

### Medium: Graph concurrency test fails under valid thread scheduling

In the `worktree-cli` package, [run_pipeline_graph_git_calls_begin_before_list_gather_completes](../../cli/src/commands/list/tests.rs:259) is intended to protect the list command's concurrent graph gathering. Its [ordering assertion](../../cli/src/commands/list/tests.rs:302) requires the first graph Git call to appear before the last list comparison Git call in a shared recording.

The production [run_pipeline](../../cli/src/commands/list.rs:57) spawns the graph worker before gathering list status, but spawning a thread does not guarantee when that worker runs. In addition, the `worktree` package's [git_command](../../lib/src/git.rs:90) records a call **before** executing the subprocess. The final list call's position therefore marks its start, not completion of list gathering. A graph call can legitimately start after that recorded position while list work is still in progress. The test treats this valid schedule as a regression.

**Observed in this review:** `just test` from the worktree area ran 331 tests: 330 passed and this test failed in the `wt` binary target. Its recorded order was:

```text
... list rev-list
... dirty status calls
... list merge-tree
... graph log
... graph merge-base
... graph log
```

The command itself returned successfully. A single diagnostic run of `just test run_pipeline_graph_git_calls_begin_before_list_gather_completes` then passed both copies, in the library and binary targets, without any source changes. That passing run does not supersede the full-suite failure; it is consistent with the scheduling dependency visible in the code.

**Why this matters:** the test was introduced with the list redesign and runs in the normal Level 1 gate. Valid scheduling can make a correct implementation fail unpredictably, obscuring actual regressions and making the review/build gate unreliable. An in-process serial-test annotation does not order the workers spawned by this test.

**Required change:** verify concurrency with explicit coordination at the gathering boundary. For example, use a small test seam that holds list gathering open until the graph worker signals entry, with a bounded wait and cleanup on failure. Assert that graph gathering can start while list gathering remains unfinished. Keep the test capable of rejecting an implementation that gathers the graph only after list gathering returns. Do not add sleeps, retries, or production synchronization merely to satisfy the current incidental Git-call order.

**Verification needed: Level 1.** Demonstrate that the replacement fails for a deliberately sequential pipeline, passes for the concurrent pipeline, and passes in the normal area run. Keep both target copies compiled and selected by the live Level 1 recipe. No real-terminal or physical-key test is needed for this thread-coordination contract.

## Blocked Findings

None. The test can be corrected without a human design decision.

## Requirement verification

Level 1 means in-process or subprocess/repository checks. Level 2 means assertions made through a real terminal or console. The table separates existing coverage from execution in this review; tests not rerun are not claimed as fresh passing evidence.

| User-observable requirement | Strongest relevant verification present | Assessment in this review |
| --- | --- | --- |
| Completion names, `base`, detached checkouts, ambiguous names, and refusal to remove the base | Level 1 resolver and CLI tests | Passed in the area run. |
| Safety tiers, independent force flags, dirty/ignored consent, branch retention, exit codes, and removal of merged branches | Level 1 policy, repository, and CLI tests | Passed in the area run. |
| PR source/head matching, live remote checks, approved destination, URL/remote-name reinterpretation, and deletion leases | Level 1 stubs and local bare repositories | Worktree coverage passed; provider suites not rerun. |
| Handoff expiry/replay, state/index changes, changed risks, nested contents, exact-byte paths, and executable bits | Level 1 library and CLI tests; Level 2 failure scenes | New mode tests and existing local handoff cases passed. Filename cases that return early on unsupported filesystems are not treated as executed coverage. |
| Report before questions, one blank line, and defaults that preserve work | Level 2 tmux captures and Level 1 policy tests | Selected terminal tests passed. Prompt responses are injected bytes. |
| Dirty-file tree glyphs/colors and count replacing more than ten paths | Level 2 styled terminal capture and Level 1 rendering tests | Passed locally. |
| Bash, zsh, and fish movement, parent/base landing, preserved subdirectories, and refusal after failed movement | Level 2 tmux shell scenes and Level 1 wrapper tests | Selected scenes passed; shell scenes iterate available shells. |
| PowerShell updates both directory states; Windows directory locks refuse before deletion | Windows Level 2 console-screen tests and Level 1 wrapper tests | Appropriate tests exist; not executed on Windows in this review. |
| Local `--from`, invalid/existing destinations, detached source, fork records, and explicit wrapper detection | Level 1 repository and CLI tests | Passed in the area run. |
| Caption/default target, parent/deleted-parent rows, merge vocabulary, PR placement, and cache age | Level 1 data/snapshot tests and Level 2 table capture | Passed locally. |
| Dot/badge/connector colors, legend, and current-row emphasis on light/dark backgrounds | Level 2 assertions on rendered cell styles and Level 1 style tests | Passed locally. |
| Linked PR badges and number-only fallback | Level 1 OSC 8/fallback assertions and Level 2 badge rendering | Passed locally. |
| Graph facts, lanes/tags, origin/fork relationships, trimming, and scale arithmetic | Level 1 worktree and component tests | Worktree graph facts passed; component suites not rerun. |
| Actual graph pixels, narrow width, half-height cap, omitted-lane notice, and intact table | Level 2 private-Kitty screenshot/transmission tests | Tests inspected, not rerun. The tmux graph-path test is not pixel-rendering proof. |
| Warm/cold gathering, full-command latency, unavailable/slow PR service, and fresh-cache request suppression | Level 1 serial performance/request-count tests | See validation below. Concurrency test reliability is the finding above. |
| Four-provider PR normalization and renderer pie-color compatibility | Level 1 provider fixtures and renderer tests | Prior review coverage retained; dependency suites not rerun. |

No additional requirement was identified whose strongest existing verification is at the wrong level. This spec adds no bare-modifier, hotkey, paste, IME, mouse, or physical-key encoding behavior requiring Level 3. Prompt-flow tests do not claim to verify a terminal's keyboard encoder. Cross-OS execution evidence and human sign-off are not readiness blockers in this review.

The new library regressions compile in the library unit-test target. CLI regressions compile in the automatically discovered `remove` integration target; this package does not disable automatic discovery. Their names select Level 1. Terminal targets require `terminal-tests`, enabled by both the Level 2 recipe and CI metadata. The tier audit found no stranded tests.

## Validation and metadata

- `worktree/just test`: **330 passed, 1 failed**, 17 excluded by the area filter. All five new executable-bit tests passed. The failure is documented above.
- Targeted diagnostic ordering-test run: **2 passed**; this does not clear the full-suite failure.
- `worktree/just test-perf`: **17 passed**, using the serial performance recipe.
- `BISCUIT_TEST_REQUIRED_BACKENDS=tmux just test-l2 level2_dirty level2_list level2_move level2_remove`: **9 passed**; backend proof recorded nine tmux executions, zero skips, and zero panics.
- `worktree/just lint`: **passed** for both packages.
- `just check-tier-coverage worktree`: **zero stranded tests**.

Kitty screenshot tests, cross-OS tests, and dependency suites were not rerun. This review changed no implementation or test files, ran no formatting command, and created no repository commit.

The requested previous-review location under `prompts/_reviews` does not exist. The existing review beside the spec was updated instead: `implemented: true` (already set) and `next: 2026-09-24-ux-improvements/review-9.md`. The specification now has `review_iterations: 9`; it is not marked completed and remains in its current lifecycle directory.
