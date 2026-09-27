---
$schema: feature-review.yaml
ready: true
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/default
created: 2026-09-25T15:22:37-07:00
spec: 2026-09-24-ux-improvements/spec.md
implemented: false
description: "A **fix** review of `2026-09-24-ux-improvements/spec.md`"
fix: 2026-09-24-ux-improvements/review-10.md
previous: 2026-09-24-ux-improvements/review-09.md
---

# Review 10: Worktree UX improvements

## Verdict

**Ready for production under this review's criteria.** The previous review's concurrency-test defect is resolved. No additional implementation defect or requirement with an insufficient verification level was found. The Level 1 suite, selected real-terminal tests, serial performance checks, lint, and tier audit passed.

Cross-OS execution evidence remains the responsibility of CI/CD, as requested. This verdict does not claim that every platform or dependency suite was executed again in this iteration.

## Previous findings

| Previous unblocked finding | Assessment |
| --- | --- |
| Graph concurrency test fails under valid thread scheduling | Implemented. The test now coordinates the gathering boundaries instead of comparing incidental Git-call ordering. Both compiled copies passed in the normal Level 1 run. |

In the `worktree-cli` package, [run_pipeline](../../cli/src/commands/list.rs:57), which gathers the table and graph data, has test-only callbacks immediately before each gather. The [replacement concurrency test and coordination helper](../../cli/src/commands/list/tests.rs:257) wait for both workers to enter, with a ten-second bound. Each worker records whether the other entered before its wait ended. The assertion therefore rejects either sequential ordering without assuming which thread the operating system schedules first. The installed helper is removed on drop, and non-test builds contain neither callback.

The implementation log records deliberate sequential variants failing both target copies, followed by restoration of the concurrent implementation. Those mutation experiments were not repeated in this review; source inspection confirms the bounded failure mechanism, and the fresh full-suite run confirms both copies execute successfully. No sleep, retry, or production synchronization was introduced to satisfy the assertion. The helper's comments accurately describe its contract.

Review #9 had **no blocked findings** and no pending human decisions to unblock. The separately scheduled ignored-file policy and list/removal performance changes are outside this review's implementation scope.

## Unblocked Findings

None.

## Blocked Findings

None. No new human design decision is required.

## Requirement verification

Level 1 verifies logic, generated output, and behavior in temporary repositories or subprocesses. Level 2 verifies output and workflows through an actual terminal or console. “Present” below distinguishes existing coverage from tests executed afresh in this iteration.

| User-observable requirement | Verification present | Review assessment |
| --- | --- | --- |
| Branch and directory completion names, `base`, detached checkouts, ambiguous names, and refusal to remove the base | Level 1 resolver and CLI tests | Passed in the area run. |
| Safety tiers, force flags, dirty/ignored consent, branch retention, merged-branch deletion, and exit codes | Level 1 policy matrix, temporary repositories, and CLI tests | Passed in the area run. |
| PR identity and head matching, live remote evidence, approved push destination, reinterpretation refusal, and remote deletion leases | Level 1 injected answers and local bare repositories; provider fixtures in `sniff` | Worktree tests passed. Provider suites were not rerun. |
| Handoff expiry/replay, changed branch/index/content/risk, nested repositories, exact-byte paths, and executable bits | Level 1 library/CLI regressions; Level 2 wrapper failure scenes | Local tests passed. Tests that return early when a filesystem rejects unusual names do not prove that case on this host. |
| Whole report before questions, one blank line, safe prompt defaults | Level 2 tmux captures and Level 1 policy tests | Selected terminal tests passed. These tests inject response bytes and do not claim keyboard-encoder coverage. |
| Dirty-file tree glyphs and colors; count replacing a list above ten paths | Level 2 styled tree capture and Level 1 count/rendering tests | Passed locally. |
| Bash, zsh, and fish move-first removal, parent/base landing, preserved subdirectories, failed movement, expired tokens, and changed tips | Level 2 tmux shell scenes plus Level 1 wrapper tests | Selected scenes passed; each scene iterates the shells available to its fixture. |
| PowerShell updates shell/process directory state and preserves Unicode paths; held Windows directories refuse before deletion | Level 1 Windows wrapper tests and Level 2 console-screen tests | Appropriate coverage exists; Windows tests were not executed in this iteration. |
| Explicit local `--from`, invalid/existing destinations, detached sources, fork records, and explicit wrapper detection | Level 1 repository and CLI tests; Level 2 wrappers | Passed in the area and selected terminal runs. |
| Caption/default target, branch lineage, absent/deleted parents, merge vocabulary, PR placement, and cache-age text | Level 1 data and snapshot tests; Level 2 table captures | Passed locally. |
| Dot, badge, connector, and legend styling; current-row emphasis on dark/light backgrounds | Level 1 style assertions and Level 2 assertions on rendered cell styles | Passed locally. |
| Linked PR badges and number-only fallback | Level 1 OSC 8/fallback assertions and Level 2 badge rendering | Passed locally. |
| Graph facts, lanes/tags, origin/fork relationships, elision, scale, and width/height planning | Level 1 worktree and `biscuit-terminal` component tests | Worktree graph tests passed; component suite inspected, not rerun. |
| Actual graph image placement, narrow-terminal sizing, half-height lane cap, omitted-lane notice, and intact table | Level 2 private-Kitty image transmission and screenshot assertions | Tests inspected, not rerun. The passing tmux graph-path test alone is not image-rendering proof. |
| Warm/cold gathering, whole-command latency, unavailable/slow PR service, fresh-cache request suppression, and concurrent graph gathering | Level 1 serial performance tests, request-count tests, and coordinated concurrency test | Passed locally. |
| Four-provider PR normalization and renderer pie-color compatibility | Level 1 provider fixtures and renderer tests | Existing coverage retained; dependency suites were not rerun. |

No requirement in this specification introduces bare-modifier detection, hotkeys, paste, IME, mouse handling, or another terminal keyboard-encoding contract requiring Level 3. The prompt scenes verify display and command outcomes through the terminal; they do not establish what bytes a physical key produces.

The replacement test is included by the test-only child module in both the `worktree-cli` library and `wt` binary targets. Its name selects Level 1, and both copies ran. The CLI retains automatic integration-test discovery. Terminal test targets require `terminal-tests`, enabled by the live Level 2 recipe and package CI metadata. The tier audit reports zero stranded tests.

## Validation

- `worktree/just test`: **331 passed**, 17 excluded by the recipe's filter.
- `BISCUIT_TEST_REQUIRED_BACKENDS=tmux just test-l2 level2_dirty level2_list level2_move level2_remove`, from `worktree`: **9 passed**. Backend proof records nine tmux executions, zero skips, and zero panics.
- `worktree/just test-perf`: **17 passed**, run serially after the terminal tests. These are the repository's existing latency checks, not a guarantee for arbitrary repository sizes or machine load.
- `worktree/just lint`: **passed** for both packages.
- `just check-tier-coverage worktree`: **zero stranded tests**.

Kitty screenshot tests, cross-OS tests, and dependency suites were not rerun. This review changed only review/specification documents, ran no formatting command, and created no repository commit. No speculative refactor or optimization is requested.

## Metadata and closure

Neither the requested `review-09.md` nor its stated location under `prompts/_reviews` exists. The actual previous review is [review-9.md](review-9.md); its `implemented: true` was already set and has been retained, and its `next` now points to this review. This review retains the explicitly requested zero-padded `previous` value in frontmatter, with this note documenting the existing filename.

The specification now records `review_iterations: 10` and `completed: true`. Its directory remains in place for the author's lifecycle closure. The upstream renderer issue and patch remain drafts as previously arranged; this review does not publish them or reopen previously accepted design choices.
