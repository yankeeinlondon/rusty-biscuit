---
$schema: feature-review.yaml
ready: false
findings:
    - "R2-F1 (high): Target identities are captured after confirmation"
    - "R2-F2 (high): Special targets bypass durable approval and operation locking"
    - "R2-F3 (high): New same-device mounts are invisible during deletion"
    - "R2-F4 (high): Handoff and root-link removals omit late-change reporting"
observations:
    - WSL Windows-held-file behavior remains unverified
human_review: true
human_review_items:
    - |-
        Choose the time allowance for discovering programs using a large checkout. The current two-second allowance expired before process discovery on the recorded 500,000-entry checkout. This does not block the code repairs in this review.

        - Keep two seconds and accept incomplete process information on these checkouts.
        - Raise the allowance, for example to ten seconds, accepting a longer possible wait before the question.
        - Schedule a separate change to Sniff so discovering programs does not first require walking every checkout entry.

        Recommendation: raise the allowance now and consider redesign only if measurements justify it.
    - |-
        Decide whether saved approvals on Windows drives accessed through WSL may rely on Windows permissions that this implementation does not inspect. The current exception checks ownership but accepts the permissive Unix mode bits reported by drvfs. This does not block the code repairs in this review.

        - Retain the exception and accept Windows access controls as part of the trust boundary.
        - Store approvals outside drvfs in protected storage, with repository association and recovery verified there.
        - Explicitly change the supported-environment requirement and refuse these drives.

        Recommendation: retain the exception if that trust boundary is acceptable; otherwise choose protected storage.
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: 2026-10-23T17:51:06-07:00
spec: 2026-10-22-confirmed-remove-always-deletes/spec.md
implemented: true
description: "A **feature** review of `2026-10-22-confirmed-remove-always-deletes/spec.md`"
feature: 2026-10-22-confirmed-remove-always-deletes/review-2.md
previous: 2026-10-22-confirmed-remove-always-deletes/review-1.md
log: /Volumes/coding/wt/rusty-biscuit/fix-remove-large-checkout/worktree/features/2026-10-22-confirmed-remove-always-deletes/log.md
implemented_by: claude/opus
next: 2026-10-22-confirmed-remove-always-deletes/review-3.md
---

# Review 2: implementation needs repair

The feature is **not production ready**. Four high findings violate explicit requirements for binding approval to the selected objects, preserving approval across interruptions, protecting newly mounted filesystems, and reporting files added after confirmation. The findings are actionable without settling the two human decisions carried from the first review.

## Previous Finding Verification

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |

There are no previous findings to populate this table. `review-1.md` has `findings: []` and explicitly says it performed no implementation review. The repair's `### Finding Ledger` in `log.md` confirms that no source or test changes followed. There is no partial repair, regression of a repair, or newly discovered sibling of an earlier finding.

The ledger's other entries are decisions and deferred review notes, not findings:

| Ledger entry | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R1-H1: process-discovery allowance (human decision) | Allowance and matching documentation | still blocked | No human ruling is recorded; `SniffUsage` still uses Sniff’s default discovery allowance of two seconds, and the warning behavior remains documented. | Author chooses among the options retained in this review's human-review items. |
| R1-H2: drvfs approval-storage trust boundary (human decision) | Ownership exception and matching documentation | still blocked | No human ruling is recorded; `recovery/storage.rs` still accepts drvfs mode bits after checking ownership. | Author accepts that trust boundary or chooses a different storage/support policy. |
| R1-N1: deferred implementation checks, not a finding | Special-target recovery, pre-question binding, move refusal, WSL contention, terminal wrapping, discovery-budget testing | unresolved | The first two are now reproduced as R2-F1 and R2-F2. An unsuccessful move may leave an approved unlock applied; the spec requires saving before unlock, which the ordinary directory path does. WSL contention is described below. | Repair the new findings; retain the independent human decisions. The historical phase notes are not accepted exceptions. |

## Unblocked Findings

### R2-F1 (high): Target identities are captured after confirmation

**Authority — spec:** “Resolve and bind the target before asking,” and “Do not resume into a directory or Git record that has been replaced.” The spec separately requires binding both the original root link and its resolved checkout. This is an object-binding defect: the implementation treats the object found after consent as the one the user saw before consent.

In the worktree library, [approval creation](../../lib/src/remove/lifecycle.rs:433) calls `plan_move` and captures the checkout and admin-directory identities only after the CLI has asked its question. [Confirmation facts](../../lib/src/remove/facts.rs:109) retain paths and target kinds, but no approved root identity. The unreadable-Git fallback likewise binds the current directory after consent. The [root-link path](../../lib/src/remove/lifecycle.rs:816) freshly reads the resolved directory's identity and never binds the original link. Non-directory entries are removed according to their current kind without comparing identities.

Reproduction: use the existing `lifecycle::tests::fixture`, gather facts as the CLI does, move the approved object aside, put a different object at the same path, then call the public `lifecycle::remove` API with the gathered facts. For the readable-directory case, copy the original `.git` file into the replacement to preserve its forward association. For the own-link case, no `.git` in the replacement target is needed. Both operations report success and destroy the replacement. This is expressly prohibited even when the replacement's contents happen to resemble the old checkout.

| Child ID | Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- | --- |
| R2-F1.1 | Ordinary checkout, `approve` / `plan_move` | Directory replaced after gathering facts; original `.git` copied | Replacement deleted; approved old directory survives elsewhere | Refuse mutation of the replacement |
| R2-F1.2 | Unreadable-Git checkout, `bind_in_place` | Broken `.git`; directory replaced after facts | Replacement without `.git` deleted in place | Refuse mutation of the replacement |
| R2-F1.3 | Own root-link resolved checkout | Resolved directory replaced after facts; link unchanged | Replacement target deleted, despite missing association | Keep replacement target and explain the changed binding |
| R2-F1.4 | Non-directory target, `remove_entry` | Approved file moved aside and replaced by another file | Replacement file unlinked | Keep replacement file |
| R2-F1.5 | Unrelated root link, `remove_entry` | Approved symlink replaced by a regular file | Replacement file unlinked; unrelated target survives | Keep replacement entry as well as unrelated target |
| R2-F1.6 | Original own root link | Approved symlink replaced by a regular file after facts | Replacement file and previously resolved real checkout both deleted | Reject changed original entry before deleting either object |
| R2-F1.7 | Git admin record at first approval | Admin directory moved aside and reconstructed at the same path after facts | Replacement admin record accepted and removed | Reject the changed admin identity before mutation |
| R2-F1.8 | Saved-operation resume and handoff | Replaced quarantine, in-place root, or admin record after saving | Clean: existing replacement tests reject/retain these objects | Preserve these checks when moving binding before consent |
| R2-F1.9 | Missing/vanished target | New directory appears where no content was approved | Clean by contract: `remove_registration` refuses an existing registered path, and this path skips content deletion | Keep the new directory |

The normal-directory binding also supplies shell-handoff approval, so repair both consumers together. Bind the repository/admin record before the question along with the root object; a fresh identity read after consent cannot establish that it is the approved record. The uninspectable case needs an explicit conservative outcome when identity cannot be established. Add tests for replacement during the question, rather than relying only on the existing tests for replacement after saving approval.

### R2-F2 (high): Special targets bypass durable approval and operation locking

**Authority — spec:** “Save it atomically and durably after consent and before the first unlock, move, or deletion,” and interrupted removals must finish “asking nothing.” This is a lifecycle-bypass defect: several target kinds use a separate destructive path with neither a saved approval nor the operation lock.

The worktree library's [target dispatch](../../lib/src/remove/lifecycle.rs:338) sends root links, files, missing/vanished paths, and uninspectable paths to [the path without a record](../../lib/src/remove/lifecycle.rs:816). That function can unlock, recursively delete a real checkout behind a link, unregister, and delete branches without opening the recovery store or taking the lock. Its comment that the steps are idempotent does not satisfy the requirement to preserve consent and branch approvals.

Reproduction: gather facts for a link to its own checkout, then invoke `lifecycle::remove` with cancellation set. It returns `Cancelled`, leaves the real checkout, saves no operation, and `resolve` returns a live worktree rather than a pending removal. The next CLI invocation therefore follows the confirmation path again. A second public-API probe makes the store path a regular file: every special-target shape still mutates/removes the registration successfully, whereas saving approval should fail before mutation.

| Child ID | Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- | --- |
| R2-F2.1 | Own root link | Cancellation during deletion; separately, unavailable store | No durable record or pending resolution; store failure does not prevent deletion | Save both objects and approvals, lock the operation, and resume without consent |
| R2-F2.2 | Unrelated root link | Unavailable store | Link and registration removed without saving | Save approval before unlink/unregister; retain external target |
| R2-F2.3 | File at checkout path | Unavailable store | File and registration removed without saving | Fail before mutation if approval cannot be saved |
| R2-F2.4 | Missing checkout | Unavailable store | Registration removed without saving | Persist approved unlock, registration, and branch steps first |
| R2-F2.5 | Vanished checkout | Missing fixture with `Target::Vanished` injected; unavailable store | Registration removed without saving | Same durable lifecycle as a missing checkout |
| R2-F2.6 | Uninspectable target | File fixture with `Target::Uninspectable` injected; unavailable store | Entry and registration removed without saving | Establish a safe binding or fail plainly; do not bypass approval storage |
| R2-F2.7 | Ordinary moved/in-place checkout, including unreadable Git | Existing save-failure, cancellation, crash, lock-wait and resume cases | Clean: compact record and operation lock govern these paths | Retain those guarantees for every target kind |

The injected target kinds in the last two special-target probes exercise dispatch, rather than claiming a native filesystem inspection failure. The missing and vanished variants share the same implementation. All special-target branch actions and unlocks are consumers of this bypass and need inclusion in the repair's crash boundaries. Extend the compact lifecycle to represent these objects and absent content; do not simply ask again after interruption. Update the module/skill descriptions that currently describe the bypass as intentional behavior.

### R2-F3 (high): New same-device mounts are invisible during deletion

**Authority — spec:** “Recheck a directory boundary before descending so a newly mounted tree is not traversed,” including “same-device bind mounts on Linux.” This is a stale-boundary defect: deletion checks current device identity against a mount table captured before traversal, so a newly introduced same-device mount has no remaining detector.

The worktree library's [deletion environment](../../lib/src/remove/delete/mod.rs:76) accepts an immutable mount slice. [The lifecycle](../../lib/src/remove/lifecycle.rs:651) gathers that slice once; `delete_tree` copies its nested mounts once. Entry descent checks current device ID and that old list. Reopening a directory and comparing its identity with a value read after the mount appeared does not identify it as a mount. Backoff passes retain the same list.

Reproduction: supply a mutable `MountInfo` source initially returning no mounts. At the first root `ReadDir` fault hook, publish a same-device mount at `target/debug` before traversal reaches it. Invoke the public lifecycle API. The table source now identifies the protected directory, but the operation deletes its payload and reports success. This uses the spec's injected mount seam; no native filesystem was mounted and no privilege or focus change was needed.

| Child ID | Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- | --- |
| R2-F3.1 | Moved checkout deletion | Mount published during root traversal | Payload under new mount deleted; success | Retain mounted contents and necessary parents; report incomplete removal |
| R2-F3.2 | In-place saved-operation deletion | Same publication after an in-place crash/resume setup | Payload deleted; success | Retain mount, Git registration, branches, and approval |
| R2-F3.3 | Own root-link checkout deletion | Same publication during traversal of resolved root | Payload deleted; success | Do not cross the new mount; preserve resumable approval as in R2-F2 |
| R2-F3.4 | Mount present before deletion starts | Existing injected nested-mount tests | Clean: known mount and its payload survive; unmount/retry completes | Retain this behavior |
| R2-F3.5 | New different-device directory | Live device check in `Run::entry` | Clean by invariant: differing volume is retained before descent | Keep the live device check alongside a live same-device boundary check |

Use a current boundary check that recognizes mount changes during traversal and waits. A list fixed at engine creation cannot enforce this requirement. Exercise all three callers with a mount introduced after the initial snapshot, and retain the existing static-mount controls.

### R2-F4 (high): Handoff and root-link removals omit late-change reporting

**Authority — spec:** the final line must name uncommitted files the confirmation did not show, using one `git status` immediately before deletion. Its verification explicitly includes files created, changed, or staged while the question is open. This is an omitted-reporting defect: only the fresh ordinary-directory path passes the question's information to the late-change reporter.

The worktree library calls [the late-change reporter](../../lib/src/remove/lifecycle.rs:594) only when `facts` is present. [Resume, including shell handoff](../../lib/src/remove/lifecycle/resume.rs:124), always passes `None`; the root-link path never calls it. Therefore a normal wrapper removal from inside the checkout silently omits the required final-line information.

Reproduction: gather facts, save approval, create `after-question.txt`, and finish through the public `finish_handoff` API from outside the checkout. Removal succeeds but `Finished::late_changes` is empty. Gathering facts for an own root link, creating that file in the real checkout, and calling `remove` produces the same omission. The CLI renders this field, so no final-line renderer can recover the missing names.

| Child ID | Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- | --- |
| R2-F4.1 | Shell-handoff second run | New untracked file after confirmation/approval saving | File deleted; `late_changes` empty | Name the newly observed file in the final line |
| R2-F4.2 | Own root-link removal | New untracked file after gathering facts | File deleted; `late_changes` empty | Same reporting as ordinary checkout removal |
| R2-F4.3 | Fresh moved/in-place ordinary checkout | Existing created/changed/staged-file tests; both placement modes enter the same reporter with facts | Clean: newly dirty paths reported; already shown paths excluded | Preserve existing scope and nonblocking behavior |
| R2-F4.4 | Recovery after interruption | Same `resume_with` path as handoff, with no saved question inventory | Reporting context is absent by construction | Preserve the required question-to-deletion reporting when repairing handoff; do not invent exhaustive reporting for post-interruption writes |
| R2-F4.5 | Missing, vanished, file, unrelated link, uninspectable target | No inspectable approved checkout inventory | Clean by applicability: no meaningful Git dirty-path comparison is available | Do not invent file counts or block deletion for failed status |

This does not request an activity journal or reporting every write. Carry only the context needed for the spec's bounded, nonblocking status comparison through handoff and the own-link route. Add public-result tests for the two missing paths and CLI wiring that checks the final line.

## Blocked Findings

None. The two pending author decisions concern product/security policy and are separate from the four implementable findings above.

## Recurrence

`recurrence: false`. Review 1 contained no findings. Its deferred notes anticipated some areas now found defective, but were expressly unverified notes rather than earlier findings or accepted exceptions.

## Input Robustness Matrix

The changed stored inputs are compact recovery records, migrated legacy records with optional reports, and operation handoffs. The default for malformed authority-bearing records is refusal without deletion; an optional legacy report is the exception because it grants no authority and may be ignored. Empty alias and branch-step arrays are valid explicit collections. No new user configuration format is introduced by process-context output.

The compact-record public `parse` matrix exercises required fields and nested branch bindings for absence, null, wrong whole-field type, allowed/invalid emptiness, malformed content, and duplicate keys. It also exercises mixed/all-invalid elements in aliases and branch steps, duplicate identity members, unknown fields, non-object documents, malformed JSON, and trailing documents. These fields use strict JSON loading plus typed validation; none defaults a missing authority-bearing collection to empty. Store-level tests exercise corrupt/foreign/unknown-version records and ensure the files stay unchanged; lifecycle and CLI unreadable-record tests ensure nothing is deleted and the error names the operation.

The legacy reader validates its record separately from the report; tests compare a valid record with no report, report version 6, corrupt/mismatched reports, and remote actions whose authority cannot be reconstructed. Reports cannot grant a remote action. The shared strict loader rejects duplicate/trailing record content. Handoffs use typed `serde_json::from_slice` with required fields and unknown-field rejection; existing public consume tests cover the control, single-use consumption, unknown/old formats, malformed operation identifiers, and expiry. A full handoff field-shape matrix is not present; no malformed-input deletion was demonstrated here.

The requested smell search found no `#[serde(default)]`, permissive string-element filtering, or missing-field fallback in compact authority parsing. The report read's `.ok()` is intentional: report readability is optional and does not change deletion authority. Its `Option` for a report operation ID is informational, not an approval binding.

## Verification and test levels

Review probes temporarily extended the existing library test module, exercised public lifecycle APIs with real disposable Git repositories, then were removed by restoring the original file byte for byte. The main sweep selected **11 tests, all passed**, in 1.03 seconds; a final sweep selected **2 additional tests, both passed**, in 0.280 seconds; their assertions deliberately confirm the defects described above, rather than claiming correctness. The longest successful probe was 1.025 seconds. The admin-replacement probe initially kept the retired record inside Git’s worktree-record directory and hit the existing duplicate-association refusal; moving that setup artifact outside the directory isolated the intended replacement case, which then reproduced the defect. The probes covered each failing target route and each of the three mount callers. Reproduction setup and observed public results are recorded with each finding above.

| User-facing requirement | Strongest relevant coverage inspected | Assessment |
| --- | --- | --- |
| Warnings, force policy, lock/submodule handling, special entries, main-checkout refusal | L1 public lifecycle and shipped CLI cases | Appropriate level; binding and special-target lifecycle defects are R2-F1/R2-F2 |
| Process warning shown in one question, yes deletes | L2 detached tmux pane, real process, captured screen, injected answer | Re-run during this review: **1 passed**, 2.129 seconds; no window took focus |
| New/changed/staged files during confirmation | L1 real-repository and CLI tests | Ordinary path covered; missing paths are R2-F4 |
| Nested mounts, symlink replacement, hard links, permanent errors, refill waits and cancellation | L1 filesystem tests with injected sources/clock | Appropriate level; dynamic same-device mount coverage is missing and its defect reproduced in R2-F3 |
| Crash recovery, legacy migration, replacement roots/admin records, concurrency | L1 fault/crash and store/lifecycle/CLI tests | Ordinary-directory routes covered; special-target routes need R2-F2 |
| Wrapper navigation and branch approvals | L1 policy/protocol tests plus existing L2 wrapper tests | Live recipes and declared targets; no new keyboard encoder behavior requiring L3 |
| Windows held-file waiting | L1 injected error cases; implementation log records native ConPTY held-directory run | Not re-run here; cross-OS evidence alone does not affect readiness |
| WSL drvfs warning and fallback | L1 injected rename fallback and log's native drvfs removal | Held-file caveat below; no cross-OS run required solely for this review |
| One-run 500,000-entry acceptance | Implementation log's single macOS run: 18.9 seconds after yes, 1,202-byte compact record, cleanup complete | Measurement read, not independently repeated; human allowance decision remains separate |

The area's L1 recipe selects the library tests and CLI integration targets; the L2 question test is a declared `terminal-tests` target selected by the live `test-l2` recipe and that feature is enabled in CI metadata. No review probe or test-only source change remains in the implementation.

L1 suite result: `just test` passed **1,528 of 1,528 selected tests**, with 14 excluded by the recipe, in 34.914 seconds. This run used the restored implementation, without the review probes.

`just lint` was not run: this review leaves no Rust changes, and formatting was not requested. Existing implementation-log gate results were treated as historical claims, not new execution evidence. No commit was made.

## Observations

### WSL Windows-held-file behavior remains unverified

The implementation log explicitly leaves untested the native case where a Windows program holds a file in a checkout removed through WSL drvfs. The Unix deletion classifier treats access-denied errors as permanent after its permission repair, and no test establishes which error this native condition produces or whether the required wait occurs. The native Windows held-directory test exercises different code. This is an unresolved verification lead for CI or a targeted rig run, not proof of a failing WSL input and not an additional readiness finding based solely on absent cross-OS evidence.
