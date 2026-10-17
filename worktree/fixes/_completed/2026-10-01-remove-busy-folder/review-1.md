---
$schema: feature-review.yaml
ready: false
findings:
    - title: Automatic recovery success lacks the specified public-path tests
      priority: high
observations:
    - Current removal documentation contradicts the implemented recovery
human_review: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: 2026-10-11T08:37:20-07:00
spec: 2026-10-01-remove-busy-folder/spec.md
implemented: true
next: 2026-10-01-remove-busy-folder/review-2.md
implemented_by: claude/opus
log: worktree/fixes/2026-10-01-remove-busy-folder/log.md
description: "A **fix** review of `2026-10-01-remove-busy-folder/spec.md`"
fix: 2026-10-01-remove-busy-folder/review-1.md
---

# Review of Remove Busy Folder

The implementation follows the specified behavior in the cases checked. It is
not production ready because the automatic recovery success paths lack the
specified automated coverage. No implementation failure was reproduced.

## Findings

### High: Automatic recovery success lacks the specified public-path tests

**Authority — spec:** the Tests section requires worktree library Level-1
coverage proving that “a worktree whose admin directory git removed but whose
folder survives is finished by the recovery.” Decision 2 also requires
“Unregistered, folder gone: success” and says successful cleanup “continues
normally (copy record, branch, remote).” These are ordinary outcomes of the
concurrent writer failure this fix addresses.

**Defect class:** automated tests do not verify successful recovery through the
removal entry point after Git returns an error.

In the worktree package,
[`remove_worktree`](../../lib/src/remove/mod.rs:100) decides whether a failed Git
command counts as success or needs cleanup. The existing
[`a_folder_git_unregistered_but_left_behind_is_reported_then_finished`](../../lib/src/remove/mod.rs:407)
test makes cleanup fail, expects `FolderNotFullyRemoved`, restores permissions,
and calls [`finish_unregistered`](../../lib/src/remove/mod.rs:157) directly.
That proves the failure path and the cleanup helper, but never proves that
`remove_worktree` returns success when its automatic cleanup succeeds. No test
injects a failed Git command whose folder is already gone, either. A regression
that reports failure after successful cleanup would leave the removal workflow
unfinished and prevent the branch steps from running.

The sweep covered the before/after registration checks, the cleanup helper,
and both CLI routes into the shared removal operation. For each route, a
temporary repository contained a clean `feat/busy` worktree and a tracked
`README.md`. A Git stand-in changed only the removal outcome: it either refused
without mutation or ran the real removal, optionally recreated build-output
leftovers, and returned failure. An undeletable leftover used a non-writable
subdirectory. The handoff cases used the actual first-run token and completed
from the base checkout. All observed results matched the spec:

| Site | Shape tested | Observed result | Expected result | Existing automated coverage |
| --- | --- | --- | --- | --- |
| Ordinary CLI → registration check | Git fails; target remains registered | Exit 1; folder, registration, branch retained | Return Git's error; retain state | Library dirty-worktree refusal covers the decision |
| Handoff CLI → registration check | Same registered refusal | Exit 1; folder, registration, branch retained | Same | Shared library decision covered |
| Ordinary CLI → automatic recovery | Git fails; target unregistered and folder gone | Exit 0; branch step completes | Success; continue removal | Missing failed-Git success case |
| Handoff CLI → automatic recovery | Same absent-folder state | Exit 0; branch step completes | Same | Same shared gap |
| Ordinary CLI → cleanup helper | Git fails; target unregistered with deletable leftovers | Exit 0; leftovers removed; branch step completes | Finish deletion; continue removal | Helper success only; automatic success missing |
| Handoff CLI → cleanup helper | Same deletable leftovers | Exit 0; leftovers removed; branch step completes | Same | Same shared gap |
| Ordinary CLI → cleanup failure | Git fails; unregistered target has undeletable leftovers | Exit 1; leftovers and branch retained; manual cleanup command shown | `FolderNotFullyRemoved`; stop subsequent steps | Library test covers failure |
| Handoff CLI → cleanup failure | Same undeletable leftovers | Exit 1; leftovers and branch retained | Same | Shared library decision covered |
| Removal entry point → initial registration guard | Ordinary folder never registered as a worktree | Error; folder contents retained | Never recover-delete this folder | `an_unregistered_folder_is_never_deleted` passes |

Add two deterministic Level-1 tests through the worktree library's removal
entry point: failed Git removal with an already-absent folder, and failed Git
removal with deletable remnants. Assert successful return, absent folder and
registration, and preservation of the library caller's branch. A representative
CLI test can additionally assert that the later branch step runs. Reuse a
controlled Git stand-in rather than depending on a writer winning a timing
race; separate handoff copies of every case are unnecessary because both routes
reach the same implementation. This is a small coverage repair, not a request
for a new deletion architecture.

## Verification

| User-facing requirement | Strongest checked verification | Assessment |
| --- | --- | --- |
| Find a child process in a descendant directory; exclude a sibling prefix and the caller | Sniff Level-1 process tests | Appropriate for process-table discovery |
| Refuse before the report, name the process, exit 4 despite force flags, retain files | CLI Level-1 subprocess test | Appropriate; no terminal input encoding or styled rendering requirement |
| Recheck usage in the library before deletion; retain registration and files | Library Level-1 test with a held subdirectory | Appropriate |
| Keep Windows' existing open-file rename probe | Existing probe and refusal tests; call remains after process check | Preserved; cross-OS execution evidence is left to CI |
| Recover after Git unregisters the target; stop branch operations if cleanup fails | Level-1 library failure/helper tests plus review-time CLI probes above | Automatic-success test gap described in the finding |

Executed on macOS:

- `just test remove::tests` from `worktree/`: 11 passed.
- `just test a_process_standing_in_the_worktree` from `worktree/`: 1 passed.
- The root invocation `just test sniff working_processes` ran the full Sniff
  Level-1 area suite rather than narrowing it: 3,353 passed, including the
  process-discovery tests. Its unrelated slow tests are outside this review.
- `just check-tier-coverage worktree`: no stranded tests.
- Eight review-time CLI probes: four Git-failure states through each removal
  route, all matching the expected results in the table.

The changed tests are compiled and selected in Level 1. This fix adds no file
format or configuration parser, so the input robustness matrix does not apply.
There is no earlier review in this fix directory; the finding is not recurrent.

## Observations

### Current removal documentation contradicts the implemented recovery

The current-behavior section of
[`worktree/docs/cli/remove.md`](../../docs/cli/remove.md:126) says that after Git
leaves an unregistered folder, `wt` stops and keeps the branch. The implemented
behavior first retries deletion and, on success, proceeds to branch removal.
That page also describes exit 4's busy-folder case as Windows-only, although
the process check now runs on every OS. The README correctly describes the
fix. The topic page has subsequently acquired other planned behavior and is
outside this fix's original changed files; update its current-behavior text
without treating the planned design as a requirement for this review.
