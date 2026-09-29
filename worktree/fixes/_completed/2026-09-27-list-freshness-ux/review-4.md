---
$schema: feature-review.yaml
ready: true
human_review: false
has_blocked_findings: false
blocked: false
recurrence: false
reviewed_by: codex/gpt-6-sol
created: 2026-09-27T20:18:02-07:00
spec: 2026-09-27-list-freshness-ux/spec.md
implemented: false
description: "A **fix** review of `2026-09-27-list-freshness-ux/spec.md`"
fix: 2026-09-27-list-freshness-ux/review-4.md
previous: 2026-09-27-list-freshness-ux/review-3.md
---

# Review 4: list freshness UX

## Verdict

**Production ready.** Review 3's unblocked finding is resolved. Its blocked findings section was empty, so there was nothing to reconsider for human input. This review found no new unblocked or blocked findings. Cross-platform CI evidence and the author's lifecycle move remain separate from this readiness decision.

## Previous findings

| Finding in review 3 | Result |
| --- | --- |
| An ordinary listing can replace a newer pull request answer | Resolved. The foreground request now takes the same persistent lock as the refresh worker before publishing and compares the stored publication identity it read before requesting. If another writer published while the request ran, it returns its answer to the caller without replacing the newer stored answer. The CLI reads the store again after its wait, so it renders that newer answer. Deterministic library and CLI tests exercise this order. |

## Unblocked Findings

None.

## Blocked Findings

None.

## Defect-class sweep

The earlier defect class was **a concurrent writer accepting an older snapshot or answer after another operation published newer state**. I checked each related write or prune path introduced or changed in this implementation. The same ordering reproduction is: take the first writer's snapshot, perform a second writer's change, then let the first writer finish.

| Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- |
| `worktree` pull request cache, ordinary listing versus forced refresh | Foreground request begins first; refresh publishes before it returns | Newer refresh answer stays stored and is rendered; the older answer is returned only to its immediate caller | Newer publication survives |
| `worktree` pull request cache, two ordinary listings | First request begins before the second publishes | First request leaves the second answer stored | Newer publication survives |
| `worktree` pull request cache, foreground request versus held lock | Foreground answer arrives while refresh holds the lock | Foreground answer is usable for that call but is not stored | Lock holder remains the only publisher |
| `worktree-cli` forced pull request wait | Refresh holder fails while an ordinary listing publishes | Listing publication does not count as the holder's success; the wait makes its bounded retry | A different writer cannot prove the holder succeeded |
| `worktree-cli` forced pull request wait | Refresh holder publishes while an ordinary listing overlaps it | The wait accepts the refresh publication and makes no second request | One successful refresh completes the wait |
| `worktree-cli` forced completion receipts | Two forced runs finish out of order | Each reads its own attempt-specific receipt; each removes only its own file | One run cannot replace or consume another's receipt |
| `worktree` fork-origin pruning | A create writes a fork record after a listing's ref snapshot | The record is reloaded and retained when the branch exists | Listing's older snapshot cannot erase the create |
| `worktree` included-file record pruning | A create writes a record after a listing's worktree snapshot | The record survives while its Git registration marker still matches | Listing's older snapshot cannot erase the create |

The fork-origin store still documents a narrow, pre-existing last-writer-wins window between simultaneous read-modify-write operations. The tested listing/prune order no longer loses a record. That separate window can flatten a fork's displayed parent but does not affect remote freshness or the reviewed pull request result.

## Requirement verification

| User-observable requirement | Strongest verification | Assessment |
| --- | --- | --- |
| Remote check, fetch, caption, local metrics, and fast-forward suggestion | Level 1 real-Git binary tests and worker tests | Appropriate for Git state and command results; review 3 also checked failed-check and failed-fetch suggestions. |
| Ordinary and forced pull request refresh under contention | Level 1 library, CLI orchestration, and forced-wait tests | Appropriate for the stored answer and badges. Tests cover same-second publications, failed holders, overlapping listing requests, and separate forced receipts. |
| Spinner phase changes and clearing, caption styling, hint, and warning placement | Level 2 tmux pane captures | Appropriate for real-terminal rendering; the focused fallback and fetch transition captures passed here. |
| Keyboard encoding | No key-press behavior was added by this fix | Level 3 is not applicable to command-line flags. |

The changed tests are compiled by declared Cargo targets. Level 1 tests are selected by `just test`; the Level 2 spinner tests are selected by the live `test-l2` recipe with `terminal-tests`. `just check-tier-coverage worktree` found no stranded tests.

The changed pull request cache format adds a required `writer` value. Its reader rejects an absent, null, wrong-type, or unknown writer through Serde and treats the document as a cache miss; a valid empty pull request list remains a valid answer. The receipt reader requires its attempt identity and rejects malformed content. These are internal, disposable cache documents: an invalid document cannot establish a successful refresh, and a failed read falls back to a bounded request or an unavailable result. The existing cache and receipt tests exercise valid controls, prior formats, corrupt content, foreign identities, and the new writer distinction through public selection and wait results.

## Validation

- `just test` in `worktree`: 712 passed, 26 skipped by tier.
- `just lint` in `worktree`: passed.
- `just test-l2 level2_list_spinner` in `worktree`: two tmux tests passed.
- `just check-tier-coverage worktree`: zero stranded tests.
- `git diff --check`: passed.
