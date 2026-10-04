---
$schema: feature-review.yaml
ready: true
findings: []
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: 2026-10-03T20:36:54-07:00
spec: 2026-10-03-list-overlap-and-keyless-notice/spec.md
implemented: false
description: "A **fix** review of `2026-10-03-list-overlap-and-keyless-notice/spec.md`"
fix: 2026-10-03-list-overlap-and-keyless-notice/review-3.md
previous: 2026-10-03-list-overlap-and-keyless-notice/review-2.md
---

# Review 3

The fix is **production ready**. Review #2's finding is implemented, including both replaced and removed origins, and the sibling sweep found no remaining incorrect request notice. The earlier overlap, snapshot, credential-evidence, and timing requirements remain implemented and verified at the appropriate test levels. No human decision is required.

## Previous findings

Review #2 has one unblocked finding: **Changed origins still show the old head request's warning and fallback notice**. It is repaired. Its blocked-findings section says “None,” so there was nothing to unblock before this implementation.

In `worktree-cli`, [RemoteAnswers::observed](../../cli/src/commands/list.rs:240) validates that the origin still identifies the repository the requests concerned. [request_notices](../../cli/src/commands/list.rs:271), which selects the PR status item and terminal notices, uses that shared guard. [caption_status](../../cli/src/commands/list.rs:252), which selects the caption's remote status, uses it too. The caption's last-known date no longer reads the previous origin's stored answer after an origin change; it falls back to the tracking ref's reflog. That additional repair follows the specification's requirement to suppress information about the old requests and needs no new design decision.

Review #1's three findings remain resolved: local gathers overlap the remote wait; successful requests retain their actual credential selection and produce the keyless notice when appropriate; measured timing groups exclude concurrent children from additive accounting.

## Unblocked Findings

None.

## Blocked Findings

None.

## Recurrence

No new finding repeats a defect class from review #1 or review #2. The request-identity defect class from review #2 was swept across every output projection below; all checked sites are clean.

## Request-identity sweep

**Class checked:** request-result projections must validate repository identity consistently so evidence about an old remote cannot be presented as a result for the current repository.

The existing network-free tests use the same fixtures for unchanged, replaced, and removed origins. The shipped-binary regressions hold the PR reply until a completed head attempt with a new attempt ID exists, then change only the origin before releasing the reply. Their unchanged-origin controls prove the warning and successful Git fallback actually happened. I ran both regressions in the full L1 suite. The in-process fixture additionally walks 11 projection shapes through all three origin states and checks the resulting notices, badges, and caption status.

| Site in `worktree-cli` | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- |
| Head credentials warning | Rejected key; unchanged/replaced/removed origin, through shipped binary and in-process fixture | Control shows warning; both changes suppress it | Same — clean |
| Other head-warning conditions | Insufficient permission, keyed/anonymous rate limit, invisible repository; three origin states | Each control shows its condition; both changes suppress it | Same — clean |
| PR credentials warning | Rejected key; three origin states | Control shows warning; both changes suppress it | Same — clean |
| Head keyless notice | Anonymous success; three origin states | Control shows notice; both changes suppress it | Same — clean |
| PR keyless notice | Anonymous empty answer; three origin states | Control shows notice; both changes suppress it | Same — clean |
| Closing Git fallback notice | Anonymous API refusal and successful `ls-remote`; three origin states, including shipped binary | Control shows notice; both changes suppress it | Same — clean |
| PR status item and table/graph badge input | Successful publication; three origin states | Control retains publication/badges; both changes supply no item or badges | Same — clean |
| Caption head status | Completed fetch; three origin states, including shipped binary | Control reports update; changed origins report a check that could not be made, where a caption exists | Same — clean |
| Caption last-known date | Changed origin with an old stored answer | Stored-answer selection is bypassed; only reflog/never fallback remains — source inspection | Never date the caption from the old origin's answer — clean |
| Fast-forward suggestion | Guarded caption status and accepted local comparison | Uses local behind count; old “still pulling” status cannot control it — source inspection and passing flag tests | Describe the local tracking comparison — clean |
| Refresh hint, wait spinner, performance rows | Timeout, spinner cleanup, grouped durations | Hint promises only another listing; spinner is cleared; timing rows contain durations rather than request claims | No old-origin result presented — clean |

The warning conditions share one guarded projection, so the rejected-key reproduction is not repaired in isolation. No extra refresh is launched for a changed origin. Warning precedence and fallback/keyless coexistence remain covered for unchanged origins.

## Input robustness matrix

This repair adds no parser or file format. I rechecked the JSON readers introduced by the original fix and ran their real-writer matrix tests through the public cache, publication, and attempt-selection results in L1. Existing envelope, identity, timestamp, answer, and receipt fields retain their broader matrices. No permissive credential default or silent element filtering was introduced.

The following summarizes the credential fields on both stored-result sides. “Drop attempt” keeps an independently valid head answer; “miss” rejects the PR publication. Variable-list cells use the `keyed` state; anonymous and unknown states have no required variable list.

| Shape | PR credentials | PR state | PR keyed variables | Head credentials | Head state | Head keyed variables |
| --- | --- | --- | --- | --- | --- | --- |
| Absent | Miss | Miss | Miss | Drop attempt | Drop attempt | Drop attempt |
| Explicit null | Miss | Miss | Miss | Drop attempt | Drop attempt | Drop attempt |
| Wrong whole type | Miss | Miss | Miss | Drop attempt | Drop attempt | Drop attempt |
| One wrong element | Not an array | Not an array | Miss | Not an array | Not an array | Drop attempt |
| Every element wrong | Not an array | Not an array | Miss | Not an array | Not an array | Drop attempt |
| Empty object/list/string | Miss | Miss | Miss | Drop attempt | Drop attempt | Drop attempt |
| Duplicate key | Miss | Miss | Miss | Drop attempt | Drop attempt | Drop attempt |
| Trailing/invalid document content | Miss | Miss | Miss | Whole-file miss | Whole-file miss | Whole-file miss |
| Valid anonymous control | Usable, anonymous | Usable | Not applicable | Usable answer and attempt | Anonymous | Not applicable |
| Valid unknown/keyed control | Usable, never anonymous | Usable | Valid names retained | Usable answer and attempt | Usable | Valid names retained |

Strict nested parsing preserves duplicate-key rejection before tagged-enum deserialization. Invalid variable names and token-like mixed-case strings are rejected. Head formats 1/2 and PR format 5 retain their documented migration behavior; unknown future versions are misses. Neither missing nor malformed evidence becomes anonymous success.

## Requirement and verification coverage

| Requirement | Appropriate level | Verification present and run | Result |
| --- | --- | --- | --- |
| Local list/history work starts during the remote wait | L1 bounded synchronization | Held-worker arrival test and no-origin/non-image controls | Pass |
| Equal successful snapshots reuse work; advancement, rewind, additions, deletions, and failed reads select final results | L1 real Git fixtures and public listing results | Pipeline snapshot cases compare caption, target, tree, comparisons, graph, and verbose labels with final-state gathers | Pass |
| Fetch reuses dirtiness; fast-forward refreshes only a moved checkout | L1 operation counts and public results | Moved-holder, no-holder, up-to-date, and refused fast-forward cases | Pass |
| Speculation has no persistent effects; accepted results save/prune once and retain successful cached comparisons | L1 stored state and counts | Held-gather persistence fixture, shared-cache and concurrent-fork controls | Pass |
| Timeout remains bounded, preserves pending output, and leaves detached work running | L1 wait seams and shipped binary | Timeout/ref-change cases, held-request and worker-cleanup tests | Pass |
| Credential evidence reflects actual requests, including pagination and host-bound selection | L1 public provider API and request assertions | Relevant `sniff` branch-head/open-PR suites; worktree store and worker tests | Pass |
| Head/PR success, adoption, publication before receipt, contention, warning precedence, and suppression select one correct credentials line | L1 shipped binary and wait/selector matrices | Keyless, cached-only, ignored, unknown, warning, and fallback cases | Pass |
| Replaced/removed origins suppress all old request results | L1 shipped binary and projection matrix | Completed-current-attempt regressions and 11-shape sweep above | Pass |
| Credential line is dim, below the caption, after spinner cleanup | L2 real-terminal rendered cells | Both credentials-warning and keyless tmux captures actually executed | Pass |
| Concurrent timings reconcile, conditional rows are correct, and nested readers select exact stages | L1 collector, pipeline, and shipped binary | Group/reconciliation/parser tests | Pass |

No keyboard-input behavior is introduced, so L3 is unnecessary. The new tests compile through the existing CLI integration target and shared unit modules, have ordinary L1 names, and ran in this review. L2 targets declare `terminal-tests`, the recipe enables it, and CI metadata includes it. Tier coverage reports no stranded tests.

## Validation

- Worktree `just test`: **958 passed**, 32 excluded performance tests.
- Worktree `just test-l2`: **32 passed**, including the real-terminal keyless line and spinner-cleanup assertions.
- Worktree `just test-perf`: **32 passed**, run serially after the other suites finished; the held-fetch case exercised its full 60-second deadline.
- Sniff `just test branch_head open_pull_requests`: **29 passed**, covering the affected provider interfaces, pagination, credential selection, and deadlines.
- Worktree and sniff `just lint`: passed.
- `just check-tier-coverage worktree`: passed, no stranded tests.
- `git diff --check`: passed.

Build/test commands used `LIBGIT2_NO_PKG_CONFIG=1` as documented for this host. Cross-platform execution evidence remains CI's responsibility and does not affect this readiness decision. Current list documentation and worktree skill pages describe the shared guard and caption fallback consistently. The required release-build overlap sample is already recorded in the performance topic page; this review does not add a new benchmark requirement.

Only review documents and requested lifecycle metadata are changed by this review. Product code, tests, and the specification's decision text are left as reviewed; no commit or lifecycle-directory move is performed.
