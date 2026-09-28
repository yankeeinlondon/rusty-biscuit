---
$schema: feature-review.yaml
ready: true
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6-sol
created: 2026-09-26T20:00:25-07:00
spec: 2026-09-26-stale-remote-caption/spec.md
implemented: false
description: "A **fix** review of `2026-09-26-stale-remote-caption/spec.md`"
fix: 2026-09-26-stale-remote-caption/review-1.md
---

# Review 1: stale remote caption

## Verdict

**Production ready.** I found no blocking implementation defect or missing requirement-level test. The caption distinguishes the local tracking ref from a dated remote observation, and listing does not wait for the live remote check.

## Requirement verification

| User-visible requirement | Strongest verification | Assessment |
| --- | --- | --- |
| Show the local tracking comparison separately from the live remote observation, with correct wording for matching, differing, absent, unverified, and missing-ref states | Level 1 caption snapshots and real-Git CLI tests; Level 2 tmux capture of the rendered comparison and observation | Appropriate. The Level 2 test also checks badges, colors, and paragraph wrapping in a real terminal. |
| Keep an aged answer visible, reject unusable answers, and avoid treating a failed request as remote-branch deletion | Level 1 store boundary and failure tests, plus real-Git deletion, pruning, recreation, and fetch-order tests | Appropriate for persisted state and Git behavior. |
| Keep the live request out of the foreground and launch at most one worker, with independent PR and remote-head refreshes | Level 1 orchestration and held-request CLI tests; serial performance tests for remote selection and full-command latency | Appropriate. The held-request test proves `wt list` returns while Git's remote request remains blocked. |
| Preserve non-interactive failure behavior and removal's shared transport | Level 1 unauthorized-response, malformed-output, deadline, and removal regression tests | Appropriate; these behaviors do not require a terminal emulator or physical key input. |

The new Level 1 tests are compiled by the library or automatically discovered CLI test targets. The Level 2 test is a declared target with its required feature and is selected by the live `test-l2` recipe. No key press, paste, mouse, or input encoding behavior changed, so Level 3 verification is not required.

## Validation

- `worktree/just test`: 527 passed, 21 excluded by the Level 1 filter.
- `worktree/just test-l2 level2_list_styles_follow_the_design_in_tmux`: 1 passed.
- `worktree/just test-perf`: 21 passed.
- `just check-tier-coverage worktree`: no stranded tests.
- `git diff --check`: passed before writing this review.

Cross-platform execution results belong to CI and do not affect this review's readiness decision.
