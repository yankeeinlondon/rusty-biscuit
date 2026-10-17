---
$schema: feature-review.yaml
ready: true
findings: []
observations:
  - Windows Search can refuse an otherwise safe removal
  - Windows test timing remains a separate measurement
human_review: true
human_review_items:
  - |-
    The existing Windows indexing policy question remains open. Windows Search can briefly hold a newly created file, so the specified rule pauses removal even when the user did not start that process. The current implementation correctly keeps this strict rule; no policy change is needed to resolve this review.

    Before changing that behavior, choose whether to:
    - Keep the strict rule and investigate isolating test folders from background indexing.
    - Amend the specification to permit one bounded second check for an explicit list of Windows indexing processes.
    - Amend the specification to exempt those processes, accepting the weaker protection for files they hold.

    Keeping the strict rule is the preferred starting point. This is the same decision requested in review #1, carried forward without changing it.
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: "2026-10-12T20:48:11-07:00"
spec: 2026-10-05-graceful-file-contention/spec.md
implemented: false
description: "A **fix** review of `2026-10-05-graceful-file-contention/spec.md`"
fix: 2026-10-05-graceful-file-contention/review-2.md
previous: 2026-10-05-graceful-file-contention/review-1.md
---

# Review 2

The fix is **production ready under the specified contract**. All four blocking findings from review #1 are implemented and verified. This review found no new high, medium, or low findings. The existing Windows policy question is separate from implementation readiness.

## Previous review findings

Review #1 has no separate `Unblocked Findings` heading. Its four high findings appear under `Findings`, and its metadata explicitly marks none as blocked. All four were checked first.

| Previous finding | Implementation and verification | Result |
| --- | --- | --- |
| Replacing a retry journal can make a pending removal unrecoverable | In the `worktree` library, [RecoveryStore::reopen](../../lib/src/remove/recovery/store.rs:217), which initializes another removal attempt, copies the current journal to an archive, builds and flushes its replacement under a temporary name, and publishes it over the still-readable current journal. Store tests cover absent, empty, header-only, partially initialized, fully initialized but unpublished, and failed replacement states through public recovery results. Lifecycle tests exercise quarantine deletion, registration-only retry, branch-only retry, and release of an unstarted operation. | Resolved |
| Completion discards removal history and confirmed deletion details | In the `worktree` library, [OperationReport::continue_from](../../lib/src/remove/outcome.rs:357) carries saved attempts into later reports. Journal replay includes prior confirmed deletions and uncertainty, using the next inventory to avoid counting surviving entries twice. Quarantine, registration-only, and branch-only retry tests assert final counts and entries; the truncation test verifies all 215 entries remain in the saved report despite the 200-path display cap. Completion keeps journals when their full entries could not be reported. | Resolved |
| The CLI does not handle interruption at deletion boundaries | In `worktree-cli`, [interrupt::Window](../../cli/src/commands/remove/interrupt.rs:31) connects received interruption signals to the lifecycle predicate for both initial removal and retry. Real-binary Unix tests deliver SIGINT before the move, after the move, and during quarantine retry. They assert ordinary exit status, retained files and branches, recovery guidance where needed, and successful later retry. Library tests retain entry-boundary cancellation and abrupt-termination recovery coverage. | Resolved |
| Human recovery reports omit process-discovery coverage | In `worktree-cli`, [stopped_markup](../../cli/src/commands/remove/recovery.rs:88) and [inspection_markup](../../cli/src/commands/remove/recovery.rs:339) render limited discovery through the existing coverage formatter. Tests cover partial discovery with exhausted budget, unavailable discovery, unsupported discovery, failed diagnostics, earlier-attempt evidence, and complete-discovery controls. Ordinary successful removal remains quiet. | Resolved |

The sibling sites listed in review #1 were swept again. Journal initialization is shared by all four retry/release paths; history preservation covers all three resumable states and first completion; the shared CLI execution environment supplies cancellation for both removal and retry; coverage projection covers stopped reports, pending inspection, existing usage refusal, and quiet success. No unresolved sibling of the earlier findings was found.

## Findings

None.

## Verification and requirement coverage

Reviewed the specification, earlier review, repair log, changed source and tests, recovery record and journal readers, removal lifecycle and retry paths, checked deletion, native rename and observation boundaries, CLI reporting, manifests, README, removal topic documentation, and updated worktree skill. Applied the worktree, rust-testing, OS, just, and terminal-harness skills.

Local macOS checks:

- `just test`: **1,378 passed, 15 skipped**, 24.1 seconds.
- `just lint`: **passed**.
- `just check-tier-coverage worktree`: **no stranded tests**.
- `BISCUIT_TEST_REQUIRED_BACKENDS=tmux just test-l2 --test level2_remove`: **11 passed, zero skipped**, 6.7 seconds, using detached tmux panes. The longest test took 6.7 seconds and belongs to Level 2; the five-second Level 1 limit does not apply to it.

The added tests compile in the declared library, binary, or integration targets and run in Level 1. The existing terminal target is enabled by `terminal-tests` and has a running Level 2 recipe. Signal delivery tests need Level 1 because the production code reacts to the delivered signal; they do not test a terminal keyboard encoder or need OS keyboard injection.

| Specification acceptance requirement | Maintained verification | Assessment |
| --- | --- | --- |
| 1. Observed usage refuses, names evidence, preserves branches, and resists force flags | Level 1 policy tests and CLI/process fixtures | Appropriate |
| 2. Limited discovery permits removal, root errors refuse, and success stays quiet | Level 1 lifecycle and CLI fixtures; new coverage-message tests | Appropriate |
| 3. Writers into the moved directory and recreation at the original stop removal | Level 1 child-writer, native observer, and CLI recovery tests | Appropriate |
| 4. Modification, replacement, recreation, delayed events, overflow, and invalidation stop safely | Level 1 injected deletion cases and native observation tests | Appropriate |
| 5. Rename failure/collision, handles, topology, locks, links, and association safeguards remain | Level 1 native and library fixtures, platform-specific tests, and error classification cases | Appropriate; cross-OS execution evidence remains CI's responsibility |
| 6. Crashes remain recoverable and original-name retry works | Level 1 phase injection, killed-child recovery, and new internal journal replacement failures across every caller | Appropriate |
| 7. Consent, expected tips, endpoints, and recreated-original protection remain | Level 1 lifecycle, handoff, and local remote fixtures | Appropriate |
| 8. Human and saved reports describe actual paths, deletion, coverage, and branch state | Level 1 serialization, report rendering, CLI recovery, and retry-history assertions | Appropriate |
| 9. Inside-target shell handoff and stdout protocol remain correct | Level 1 wrapper tests and Level 2 bash/zsh/fish handoff and failure tests | Appropriate; Level 2 executed locally |
| 10. Missing-directory removal, pending aliases, ambiguity, and partial checkouts remain safe | Level 1 library and CLI fixtures | Appropriate |
| 11. Write failures, uncertain unlinks, interruption, and branch failures preserve recovery | Level 1 store/deletion/lifecycle tests plus new real-binary SIGINT tests | Appropriate |
| 12. Native paths, link boundaries, no-replacement rename, and exit classes remain correct | Level 1 native-path, library, and CLI result tests | Appropriate |

The input robustness boundary remains JSON recovery records and JSON Lines deletion journals. The maintained `record_robustness_matrix` and `journal_robustness_matrix` tests start with store-written fixtures, edit one field at a time, and assert the public recovery result for missing, null, wrong-type, empty, duplicate, invalid, and trailing-content shapes, including collection element errors and torn journal tails. Both passed. Archived journals use the same journal reader. The new saved-report reader uses typed deserialization and reports a read failure as a warning rather than silently treating it as an empty history; it supplies reporting history, never deletion approval. Fresh filesystem, identity, index, branch, and consent checks still govern retry.

Structural review found one initial inventory with targeted deletion checks, bounded batches and event queues, and hashing restricted to consent-sensitive content. This repair adds no repeated checkout walk per process or per deleted entry. Retry history is assembled at reporting/recovery boundaries. No speculative performance requirement or new terminal input requirement was added.

No implementation code was changed by this review. The existing user edits were preserved. No formatter, commit, terminal focus change, or lifecycle-directory move was performed.

## Recurrence

None. The four earlier defect classes are resolved; `recurrence` is false.

## Observations

### Windows Search can refuse an otherwise safe removal

The earlier implementation evidence records transient Windows Search handles causing refusal. This follows the specification's any-holder rule and prohibition on automatic retries. The repair preserves that rule. The existing author decision is carried forward in frontmatter; it does not block production readiness under the current contract.

### Windows test timing remains a separate measurement

The earlier implementation log reports Windows tests above five seconds, including inherited tests outside this fix. This review's macOS run does not resolve that host-specific observation. Measure discovery and observation costs on native Windows before attributing the delays or changing the implementation. Do not mask slow Level 1 tests with retries, renamed tiers, or larger timeouts.
