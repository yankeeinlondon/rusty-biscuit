---
$schema: feature-review.yaml
ready: false
findings:
    - title: Forced refresh can finish without a pull request result after lock contention
      priority: high
    - title: Spinner phase changes lack real-terminal verification
      priority: high
    - title: Ignored API preferences merge distinct SSH ports
      priority: medium
    - title: A failed remote check hides an available fast-forward suggestion
      priority: medium
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6-sol
created: 2026-09-27T12:22:32-07:00
spec: 2026-09-27-list-freshness-ux/spec.md
implemented: true
next: 2026-09-27-list-freshness-ux/review-2.md
implemented_by: claude/opus
log: worktree/fixes/2026-09-27-list-freshness-ux/implementation-log.md
description: "A **fix** review of `2026-09-27-list-freshness-ux/spec.md`"
fix: 2026-09-27-list-freshness-ux/review-1.md
---

# Review 1: list freshness UX

## Verdict

**Not production ready.** Three reachable cases break the specified behavior, and the spinner's changing messages lack the required real-terminal test. The findings can be resolved in code and tests without a human design decision.

## Findings

### High: Forced refresh can finish without a pull request result after lock contention

In `worktree-cli`, the [forced wait](../../cli/src/commands/list/wait.rs:169) accepts a completion receipt whose pull-request half says `Contended` as soon as the other process releases the pull-request lock. It never checks whether that process published an answer. The [pull-request refresh](../../lib/src/pull_requests.rs:278) leaves the store untouched if its request fails. Thus, if another refresh holds the lock and fails, `wt --refresh` returns after the lock opens with the old or missing pull-request answer and no failure from the holder. This violates the spec's requirement that forced refresh check the contended half's store before reporting completion. A lock becoming free proves only that the request ended.

Make the forced wait inspect a result tied to the contending request or validate that a fresh answer was published after the lock clears. If there is no qualifying result, report the failure or make a bounded retry within the same refresh deadline. Extend [the current lock-contention test](../../cli/src/commands/list/wait/tests.rs:338), which only asserts elapsed time, to cover both a successful publication and a failed holder. Add a Level 1 binary test that confirms `--refresh` shows the fresh pull-request answer or a truthful failure.

### High: Spinner phase changes lack real-terminal verification

The spec requires the live spinner to change from `updating` to `pulling remote updates`, `no API key, using fallback method`, or `rate limited, using fallback method` as the worker progresses. In `worktree-cli`, [the Level 2 spinner test](../../cli/tests/level2_list_verbose.rs:918) captures only the initial `updating` phase and its cleanup. [Level 1 tests](../../cli/src/commands/list/wait/tests.rs:430) check the strings and a manufactured output writer, but cannot verify that a real terminal displays each replacement on one line, without remnants of the longer prior message, before the final caption. The strongest tests for these visible phases are therefore at the wrong level.

Add focus-preserving Level 2 terminal scenes that hold the worker in its fallback and fetch phases long enough to capture each message, then release it and verify the spinner line disappears before the caption. Keep the Level 1 mapping and output tests.

### Medium: Ignored API preferences merge distinct SSH ports

In the `worktree` library, [RepoIdentity::from_origin](../../lib/src/api_preference.rs:45) assigns port 443 to every SSH URL on a recognized provider, even when the URL explicitly names another port. For example, `ssh://git@github.com:2222/owner/repo.git` gets the same stored identity as `https://github.com/owner/repo.git`. Recording `--ignore-api` for one therefore disables provider requests and pull-request badges for the other. The spec requires distinct ports to remain distinct while equivalent SSH and HTTPS spellings share an identity.

Normalize the known provider's standard SSH port to its HTTPS API identity, but preserve an explicit nonstandard port. Add a Level 1 preference test for two otherwise identical provider URLs with different explicit SSH ports, including a persisted preference round trip. The existing test covers only the standard SSH port.

### Medium: A failed remote check hides an available fast-forward suggestion

In `worktree-cli`, [the suggestion gate](../../cli/src/commands/list.rs:442) requires the current check to finish as `CheckedNow` or `Fetched`. If `origin/main` already contains commits ahead of the local `main` and this run's remote check fails, the caption still shows that the local branch is behind the tracking ref, but the listing omits the `wt --ff` suggestion. The spec excludes a suggestion while a fetch is still running, but does not exclude a completed failed check. It explicitly allows `--ff` to use the local tracking ref after an update failure, so the suggestion remains actionable. The same gate suppresses it after a completed fetch failure.

Base the suggestion on the post-wait local comparison once no fetch remains in progress. Keep the caption's failure reason so users understand that the tracking ref may be old. Add a Level 1 binary test for a behind local branch with a failed check and with a failed fetch, and retain the existing no-suggestion test while fetching is unfinished.

## Requirement verification

| User-observable requirement | Strongest verification found | Assessment |
| --- | --- | --- |
| Check, fetch, preserve prior answers, and render matching local counts | Level 1 real-Git CLI tests and injected worker tests; serial performance tests | Appropriate for Git state and timing. The lock-contention result gap is above. |
| Caption states, credentials lines, fallback notice, suggestion, and flag behavior | Level 1 snapshots and binary tests | Appropriate for text and command behavior. The two wording or flag gaps are above. |
| Caption styling, initial spinner and cleanup, dim hint and credentials line in an actual terminal | Level 2 tmux pane capture in `level2_list_verbose` | Appropriate for the covered scenes. Dynamic spinner phases have only Level 1 verification, as noted above. |
| Physical keys, paste, mouse, and terminal input encoding | No changed requirement | Level 3 is not needed for this fix's command-line flags. |

The Level 2 test is a declared `worktree-cli` target with `terminal-tests` enabled by the live `test-l2` recipe. The existing Level 1 tests are compiled by declared targets and selected by `just test`; the performance tests have the live `test-perf` recipe. `just check-tier-coverage worktree` reported zero stranded tests.

## Validation

- `just check-tier-coverage worktree`: zero stranded tests.
- `worktree/just test a_forced_wait_waits_for_a_contended_pr_half_to_be_released`: two copies of the focused test passed, one in each CLI target. The test confirms the wait duration but does not check whether the contending process produced a result.

Cross-OS run results and human review are external to this readiness decision.
