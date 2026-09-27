---
$schema: feature-review.yaml
ready: false
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6-sol
created: 2026-09-26T14:58:41-07:00
spec: 2026-09-25-list-remove-performance/spec.md
implemented: true
next: 2026-09-25-list-remove-performance/review-2.md
implemented_by: claude/default
log: worktree/fixes/2026-09-25-list-remove-performance/implementation-log.md
description: "A **fix** review of `2026-09-25-list-remove-performance/spec.md`"
fix: 2026-09-25-list-remove-performance/review-1.md
findings:
    - title: A foreground PR request can show badges from a previous origin
      priority: high
    - title: The stale PR age line has no real-terminal style verification
      priority: high
---

# Review 1: List and remove performance

## Verdict

**Not ready for production.** The stale-answer path avoids waiting for the background request, and the removal handoff preserves its content and remote safety checks. Two requirements remain unresolved: a foreground request can display an answer from an origin that changed during the request, and the newly required dim age line has no Level 2 rendering check.

## Findings

### High: A foreground PR request can show badges from a previous origin

In the `worktree` library, [fetch_and_publish](../../lib/src/pull_requests.rs), which obtains PR badges when `wt list` has no usable stored answer, checks `origin` again before saving. If the URL changed, it declines to save but still returns the old answer. The `worktree-cli` package's [gather_prs](../../cli/src/commands/list.rs) then renders those badges for the current table. The existing [unit test](../../lib/src/pull_requests.rs) explicitly asserts that the old answer may be shown.

This violates the spec's requirement that a changed origin never show badges from the previous repository. A user who changes the remote while a slow first request is in flight can see a badge that points to an unrelated repository. The safeguard protects future runs but not the run in progress.

**Required change:** if the URL no longer matches after the request, return no PR answer for this run as well as leaving the store untouched. Add a Level 1 regression that changes `origin` while a foreground request is blocked, releases that request, and checks the actual `wt list` output for absence of the old badge. The current unit test should assert this contract too.

### High: The stale PR age line has no real-terminal style verification

The `worktree-cli` package's [pr_age_markup](../../cli/src/commands/list_table.rs) adds a dim line beneath the legend when a displayed PR answer is stale. Level 1 [rendering tests](../../cli/tests/list_table.rs) check its text, and Level 1 [binary tests](../../cli/tests/list_prs.rs) check that it appears with stale badges. The existing Level 2 [styled terminal capture](../../cli/tests/level2_list_verbose.rs) checks a PR badge and other dim cells, but it seeds a fresh answer and never captures this new line.

The spec asks for a visibly dim age line in the real terminal. Text and markup assertions cannot prove that the terminal actually renders it beneath the legend with dim styling. Under this review's test-level rule, this user-visible styling requirement needs Level 2 verification.

**Required change:** add a stale-answer scene to the live `worktree` Level 2 terminal recipe. Capture the pane and assert that the age line follows the legend and its cells are dim. Keep the existing Level 1 tests for freshness, text, and request behavior. This needs no Level 3 keyboard test.

## Requirement verification

| Requirement | Strongest present verification | Assessment |
| --- | --- | --- |
| Stale answer shown immediately; one detached worker; lock recovery after a crash; failed refresh preserves the answer | Level 1 binary tests with blocked local requests and worker process checks | Appropriate. The parent returns while the worker is blocked. |
| Missing, changed, old-format, or future-dated answer follows the foreground path; origin binds stored badges | Level 1 library and binary tests | Incomplete for a URL change **during** the foreground request; see first finding. |
| Stale badge age text and dim appearance beneath the legend | Level 1 text checks; Level 2 fresh-badge capture only | Wrong level for the new dim appearance; see second finding. |
| Full non-image command stays within the stale-answer bound | Level 1 serial performance gate | Appropriate for timing and request behavior. |
| Handoff with a kept or explicitly deleted branch avoids a second network request; local proof also avoids one | Level 1 binary tests with a counting local proxy | Appropriate. |
| Automatic deletion rechecks PR or remote proof; remote deletion rechecks the live head and lease | Level 1 library and binary tests with a local provider and bare remote | Appropriate for state and subprocess behavior. |
| Same-size, same-time dirty-file edits, nested repositories, and read failures refuse the handoff | Level 1 library and binary tests | Appropriate for content safety. |
| `git status` behavior stays as before | Source inspection and the documented measurements | This is an investigation outcome, with no new user-facing behavior to test. |

No requirement here depends on the terminal's physical keyboard encoder, so Level 3 is not needed. The Level 1 integration files are compiled by discovered targets and selected by `just test`; the serial timing tests are selected by `just test-perf`. The existing Level 2 test target has the `terminal-tests` feature and a live `test-l2` recipe. `just check-tier-coverage worktree` reported zero stranded tests.

## Validation

- `just test` in `worktree`: 456 passed, 18 excluded by the tier filter.
- `just test-perf` in `worktree`: 18 passed, run serially.
- `just check-tier-coverage worktree`: zero stranded tests.

This review made no source changes. Cross-OS results and a separate human sign-off are outside this readiness decision. The spec's `review_iterations` is now 1; it was not marked completed.
