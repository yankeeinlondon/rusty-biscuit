---
$schema: feature-review.yaml
ready: true
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6-sol
created: 2026-09-26T15:42:08-07:00
spec: 2026-09-25-list-remove-performance/spec.md
implemented: false
description: "A **fix** review of `2026-09-25-list-remove-performance/spec.md`"
fix: 2026-09-25-list-remove-performance/review-2.md
previous: 2026-09-25-list-remove-performance/review-1.md
---

# Review 2: List and remove performance

## Verdict

**Ready for production.** Both findings from the first review are resolved. I found no further gap against the spec. Cross-OS execution evidence is left to the CI process and does not affect this readiness decision.

## Previous findings

The first review had two unblocked findings and no blocked findings:

1. **Foreground PR results after an origin change.** The `worktree` library's [fetch_and_publish](../../lib/src/pull_requests.rs) now returns no answer if `origin` changes while the request is in flight, so the `worktree-cli` list command cannot display badges from the old repository. A library test checks the returned value and store, and a Level 1 [binary test](../../cli/tests/list_prs.rs) changes `origin` during the request and checks the displayed list. Both passed.
2. **Stale PR age line in a real terminal.** The `worktree-cli` [Level 2 tmux test](../../cli/tests/level2_list_verbose.rs) now displays a stale answer, checks the badge, and verifies that the age line sits immediately below the legend with dim styling in captured terminal cells. It also confirms that the background request reached a local stub and ended without changing the stored answer. This test passed.

No blocked finding became actionable between reviews because the first review listed none.

## Requirement verification

| User-observable requirement | Strongest verification | Assessment |
| --- | --- | --- |
| Stale PR badges appear without a network wait; one worker refreshes them, and failed refreshes retain the answer | Level 1 binary and process tests, plus a serial full-command performance gate | Appropriate for process, cache, and timing behavior. |
| A missing, old-format, future-dated, or differently bound answer follows the foreground path; an origin change during the request never shows old badges | Level 1 library and binary tests | Appropriate for request and output behavior. |
| The stale age line appears below the legend with dim styling | Level 2 tmux cell capture, alongside Level 1 text tests | Appropriate for real-terminal rendering. |
| A removal handoff avoids unnecessary second-run network requests while rechecking remote-dependent safety and the deletion lease | Level 1 library and binary tests with controlled providers and a local bare remote | Appropriate for decisions and subprocess behavior. |
| Changed dirty content, including same-size and same-time edits and nested repository files, prevents removal | Level 1 filesystem and binary tests | Appropriate for the content-safety contract. |
| `git status` keeps its existing behavior | Source inspection and documented performance measurements | The spec deliberately makes no behavior change here. |

These requirements do not depend on a physical keyboard event, so Level 3 testing is not needed. The Level 2 test is compiled by the declared `level2_list_verbose` target with `terminal-tests`, selected by the live `test-l2` recipe, and `just check-tier-coverage worktree` reports no stranded tests.

## Validation

- `just test` in `worktree`: 457 passed, 18 excluded by the tier filter.
- `just test-perf` in `worktree`: 18 passed serially.
- `BISCUIT_TEST_REQUIRED_BACKENDS=tmux just test-l2 level2_list_stale_pr_answer_shows_a_dim_age_line_in_tmux`: passed with tmux backend proof.
- `just lint` in `worktree`: passed for the library and CLI.
- `just check-tier-coverage worktree`: zero stranded tests.

This review made no source changes. The implementation is complete and ready for review under `2026-09-25-list-remove-performance`.
