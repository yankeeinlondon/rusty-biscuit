---
$schema: feature-review.yaml
ready: true
findings:
    - "R3-F1 (medium): A locked worktree whose path holds a file still refuses removal"
observations: []
human_review: false
human_review_items: []
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: 2026-10-23T19:00:20-07:00
spec: 2026-10-22-confirmed-remove-always-deletes/spec.md
implemented: true
description: "A **feature** review of `2026-10-22-confirmed-remove-always-deletes/spec.md`"
feature: 2026-10-22-confirmed-remove-always-deletes/review-3.md
previous: 2026-10-22-confirmed-remove-always-deletes/review-2.md
implemented_by: claude/opus
---

# Review 3: ready, with one nonblocking finding

The feature is production ready under this review's priority rules. All four previous high findings are verified fixed. One unusual combination remains: a locked worktree whose directory has been replaced by a file refuses even with `--force`. That is a medium finding; it leaves the file intact and does not block readiness. The two pending author decisions remain separate from code readiness.

## Previous Finding Verification

The repair inventory is the second `### Finding Ledger` in [the repair log](log.md). Its completion claims were checked against the implementation and the tests run in this review. The following table accounts for every child row, including the earlier clean controls.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R2-F1 — Target identities are captured after confirmation | R2-F1.1 ordinary checkout; R2-F1.2 unreadable checkout | verified fixed | The worktree library's [confirmation facts](../../lib/src/remove/facts.rs:273) capture entry identity before the question; [binding](../../lib/src/remove/lifecycle.rs:379) compares it before approval storage. `a_checkout_replaced_after_the_question_is_refused_and_kept` and `an_unreadable_checkout_replaced_after_the_question_is_refused_and_kept` passed; the former also exercises `save_approval`, which supplies shell handoff. | None. |
| R2-F1 — Target identities are captured after confirmation | R2-F1.3 own-link resolved checkout; R2-F1.4 file; R2-F1.5 unrelated link; R2-F1.6 original own link | verified fixed | `an_own_links_checkout_replaced_after_the_question_is_kept`, `a_file_replaced_after_the_question_is_kept`, `an_unrelated_link_replaced_by_a_file_after_the_question_is_kept`, and `an_own_link_replaced_by_a_file_after_the_question_deletes_neither` passed. Binding compares both identities for an own link and the original entry for other special targets. | None. |
| R2-F1 — Target identities are captured after confirmation | R2-F1.7 admin record; R2-F1.8 saved resume/handoff controls; R2-F1.9 missing/vanished replacement | verified fixed | Admin replacement before and during removal tests passed; [unregistration](../../lib/src/remove/lifecycle.rs:783) also checks the saved admin identity. Existing quarantine, in-place root, admin and handoff replacement tests passed. `a_directory_appearing_where_the_checkout_was_missing_is_kept` checks both missing and vanished variants; the unidentifiable-target test confirms conservative refusal. | None. |
| R2-F2 — Special targets bypass durable approval and operation locking | R2-F2.1 own link | verified fixed | `a_cancelled_own_link_removal_is_pending_and_finished_without_a_question`, own-link crash-boundary tests, and the CLI retry test passed. The saved record binds the link and resolved checkout; the shared lifecycle holds the operation lock through cleanup. | None. |
| R2-F2 — Special targets bypass durable approval and operation locking | R2-F2.2 unrelated link; R2-F2.3 file; R2-F2.4 missing; R2-F2.5 vanished; R2-F2.6 uninspectable | verified fixed | `an_unsavable_approval_of_any_target_changes_nothing` exercises every listed target and checks retained entries, lock, registration and branch. Missing-target crash tests also check approved unlock and branch steps. Uninspectable-without-identity is refused before mutation; the injected identifiable form uses the shared lifecycle. | None for this finding. The locked-file classification interaction is separately reported below; the unlocked file does use durable approval. |
| R2-F2 — Special targets bypass durable approval and operation locking | R2-F2.7 ordinary moved/in-place controls | verified fixed | [Approval](../../lib/src/remove/lifecycle.rs:521) saves every represented target under the operation lock. Ordinary save-failure, crash, cancellation, lock-wait, stale-operation and resume tests passed. The former destructive path without a record is gone. | None. |
| R2-F3 — New same-device mounts are invisible during deletion | R2-F3.1 moved checkout; R2-F3.2 in-place resume; R2-F3.3 own link | verified fixed | All three `a_mount_made_while_…` lifecycle tests passed. They publish a mount during traversal, verify retained contents and approval, remove the injected mount, and finish the same operation. The deletion engine now receives the live mount source. | None. |
| R2-F3 — New same-device mounts are invisible during deletion | R2-F3.4 existing mount; R2-F3.5 different-device boundary | verified fixed | Static mount tests and the mount-created-during-refill test passed. [Directory descent](../../lib/src/remove/delete/mod.rs:366) checks the current mount boundary on every open. The earlier live volume-identity comparison still retains different-device directories before descent. Native macOS mount lookup control passed. | None. Native evidence on other operating systems remains CI's responsibility. |
| R2-F4 — Handoff and root-link removals omit late-change reporting | R2-F4.1 shell handoff; R2-F4.2 own link | verified fixed | The handoff now carries the question's dirty paths; `the_handoff_second_run_finishes_once_the_caller_left` and CLI `a_file_created_between_the_runs_is_named_by_the_second_run` passed. `a_file_created_in_an_own_links_checkout_after_the_question_is_named` passed through the unified deletion step. | None. |
| R2-F4 — Handoff and root-link removals omit late-change reporting | R2-F4.3 fresh moved/in-place; R2-F4.4 interrupted recovery; R2-F4.5 targets without an inspectable checkout | verified fixed | Existing created/changed/staged tests and the new in-place test passed. `without_the_questions_list_late_files_are_deleted_unnamed` and `a_target_without_a_checkout_names_no_late_change` preserve the bounded comparison and avoid invented counts. [The reporter](../../lib/src/remove/lifecycle/late.rs) stays nonblocking when status fails. | None. |

Review 1 contained no findings, so there are no earlier open findings to carry. Its two human decisions have no recorded resolution and remain in this review's frontmatter. Neither is a code finding. The conservative refusal when no identity can be established respects the spec's retained identity protection; it does not authorize speculative deletion to satisfy the general uninspectable-path rule.

## Unblocked Findings

### R3-F1 (medium): A locked worktree whose path holds a file still refuses removal

**Authority — spec:** the file-at-checkout-path rule says the question describes what is there and “yes deletes it and Git's record”; the lock rule says “Yes unlocks and deletes,” and `--force` is the scripted equivalent. This is a target-classification defect: absence of Git's `prunable` marker is treated as evidence of a directory even when filesystem inspection found a file. The combination of a surviving worktree lock and a replacement file is unusual, so this is medium, not a blocker for normal removals.

In the worktree library, [fact gathering](../../lib/src/remove/facts.rs:295) compensates for Git's omission of `prunable` on locked entries only when the path is absent. The unchanged classifier otherwise returns healthy for a non-link entry without that marker. The new removal workflow consequently selects `Target::Checkout` for the file, then [checkout binding](../../lib/src/remove/lifecycle.rs:447) rejects it as `NotARealDirectory`. This is within the feature boundary because supporting deletion of these formerly refused targets is new behavior; changing the shared classifier is not necessarily required to fix the removal workflow.

Reproduction used the existing real-repository fixtures. Lock the linked worktree, replace its directory with a regular file, then gather confirmation facts and call the public removal API. The shipped CLI was also exercised with `wt remove feat-x --force`: it printed “whatever is there will be deleted,” then “Nothing was removed: … is not a real directory,” and exited **3**, leaving the file and Git registration intact. The same unlocked fixture removes successfully.

The class sweep changed only the target shape or lock state in that fixture. Every API probe reached the public removal result; the successful cases asserted cleanup, and foreign-link cases also asserted the external file survived.

| Child ID | Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- | --- |
| R3-F1.1 | Facts → checkout binding → CLI result | Locked entry, regular file at checkout path | Classified as checkout; API refuses; CLI exits 3 | Describe a file, save approval, unlock, unlink file, remove registration |
| R3-F1.2 | Facts → entry lifecycle | Same file, unlocked control | Clean: file and registration removed | Same successful removal |
| R3-F1.3 | Facts → own-link lifecycle | Locked own link; unlocked control | Clean: both link and approved real checkout removed | Same successful removal |
| R3-F1.4 | Facts → unrelated-link lifecycle | Locked unrelated link; unlocked control | Clean: link/registration removed, external contents retained | Same boundary protection |
| R3-F1.5 | Facts → absent-target lifecycle | Locked missing checkout | Clean: registration removed | Same successful cleanup |
| R3-F1.6 | Facts → unreadable-checkout fallback | Locked directory with broken `.git` | Clean: directory and registration removed | Same successful removal |
| R3-F1.7 | Facts → ordinary checkout lifecycle | Locked ordinary directory | Clean: unlock and removal succeed | Same successful removal |
| R3-F1.8 | Vanished and uninspectable variants | Code contract and existing tests | Clean for this class: vanished is assigned after confirming absence; an unidentifiable target refuses before deletion; injected identifiable variants use durable approval | Preserve those outcomes; do not infer a directory from Git's marker alone |

Correct the removal facts to distinguish a file even for a locked entry, preserving its lock warning and shared saved-approval path. Add a file-plus-lock control to the existing target tests and CLI wiring. This should be a small correction, not a new protection or recovery layer. The repair log already noted the limitation and excluded locked files from its save-failure matrix; that note is not an accepted spec exception.

## Blocked Findings

None. The pending author choices do not block this code finding or readiness.

## Recurrence

`recurrence: false`. The new finding concerns classification from Git's lock/prunable interaction. Earlier findings concerned identities captured after consent, missing durable approval, stale mount information, and omitted late-change reporting. None reported this classification defect. This is a newly verified interaction, not a partial repair or regression of one of those findings.

## Input Robustness Matrix

The relevant stored inputs are the compact recovery record, legacy recovery record and optional report, and shell handoff. The default for malformed authority-bearing input is refusal with no speculative deletion. Exceptions are explicit: empty aliases and branch-step arrays are valid; an absent or invalid optional legacy report is informational and grants no authority; handoff `shown_dirty` must be present, with `null` meaning no comparison inventory and `[]` meaning a known empty inventory.

The passing compact-record matrix covers absent/null/wrong whole-field type, applicable empty values, invalid collection elements, duplicate keys, unknown fields, malformed JSON and trailing content through the public parser. The added target variants share identity and path validators; variant tests check legal paths and missing/null own-link identities. Store/lifecycle and CLI malformed-record tests verify refusal without deletion and an error identifying the operation. Legacy-reader and migration tests preserve the distinction between approval-bearing records and optional reports. The handoff field matrix distinguishes missing, null, empty, wrong-type and invalid-element forms, alongside malformed-token/version/single-use checks; valid and invalid CLI handoff tests establish wiring.

A source search found no new defaulting, silent element filtering, or ignored parse error in authority-bearing fields. The legacy action `filter_map` deliberately omits the checkout intent when constructing branch actions; it is not filtering malformed input. No new file-format finding was established.

## Verification and limits

| User-facing requirement | Appropriate level and evidence |
| --- | --- |
| Process warning, one confirmation, answer yes, and successful final line | **L2:** `just test-l2 level2_a_process_warning_in_the_question_answered_yes_deletes_the_worktree` passed in tmux, with rendered warning, default question placement, injected answer and final cleanup assertions. No window gained focus. |
| Force/refusal behavior; lock, file, link, submodule and unreadable-target handling | **L1:** public-library fixtures and shipped-CLI tests in the passing area suite; the new locked-file interaction is separately reported above. |
| File changes during consent/handoff and final-line names | **L1:** filesystem/status comparison tests, handoff public result, and CLI final-line wiring passed. This is data reporting, not a terminal-input encoding requirement. |
| Recovery, branch safety, identity boundaries, mounts and permanent errors | **L1:** injected crash/fault/mount cases and real temporary repositories in the passing suite. All previous finding children are accounted for above. |
| Cancellation, contention and directory-refill waits | **L1:** cancellation and injected-clock tests; platform-specific held-file tests remain selected on their matching operating systems. The feature adds no bare-modifier or special-key encoding contract requiring L3. |
| Large-checkout completion and bookkeeping size | The existing implementation log records the requested one-time macOS 500,000-entry acceptance run. Read as historical measurement, not independently repeated; no additional performance campaign was required. |

Executed before temporary probes: `just test` passed **1,559/1,559** tests, with 14 excluded, in **26.602 seconds**. The focused tmux L2 run passed **1/1** in **1.218 seconds**. `just check-tier-coverage worktree` found **zero stranded tests**; the L2 target is declared, its recipe is live, and `terminal-tests` is enabled in CI metadata.

Temporary review probes ran through nextest: nine API cases produced eight passes and the expected locked-file failure; the separate shipped-CLI probe reproduced exit 3. All temporary Rust edits were restored byte for byte. These failures establish the medium finding; they are not failures in the unchanged existing suite. No source fix, formatting command, commit, or lifecycle-directory move was made. Lint was not rerun because the retained changes are review documentation and metadata only. No new cross-OS results are claimed.

## Observations

None.
