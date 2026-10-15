---
$schema: feature-review.yaml
ready: false
findings:
  - title: Replacing a retry journal can make a pending removal unrecoverable
    priority: high
  - title: Completion discards removal history and confirmed deletion details
    priority: high
  - title: The CLI does not handle interruption at deletion boundaries
    priority: high
  - title: Human recovery reports omit process-discovery coverage
    priority: high
observations:
  - Windows Search can refuse an otherwise safe removal
  - Windows test timing needs a separate measurement
human_review: true
human_in_the_loop: true
human_review_items:
  - |-
    Decide whether Windows indexing processes should remain subject to the existing strict usage rule. Windows Search briefly opened a newly created test file on three recorded occasions, causing the command to refuse removal. A later invocation succeeded. This is the required current behavior, not a reason to silently ignore those processes.

    - Keep the rule that every observed holder refuses removal, and isolate test folders from background indexing while keeping separate tests for actual contention. This preserves the specification and is the preferred starting point.
    - Explicitly change the specification to allow a bounded second query for a fixed list of Windows indexing processes. This changes the no-automatic-retry policy and needs author approval before implementation.
    - Explicitly exempt those processes. This weakens the protection against deleting files another process holds and is not recommended.

    Choose whether the production rule should change. The four blocking findings below can be repaired without waiting for this decision.
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: "2026-10-12T19:12:14-07:00"
spec: 2026-10-05-graceful-file-contention/spec.md
implemented: true
implemented_by: claude/opus
log: worktree/fixes/2026-10-05-graceful-file-contention/log.md
description: "A **fix** review of `2026-10-05-graceful-file-contention/spec.md`"
fix: 2026-10-05-graceful-file-contention/review-1.md
next: 2026-10-05-graceful-file-contention/review-2.md
---

# Review 1

The fix is **not production ready**. Its normal removal, usage refusal, writer detection, and consent checks pass the existing tests, but recovery has gaps in journal replacement, retained evidence, interruption handling, and human reporting. These are four blocking findings grounded in the specification. None needs a new design decision to repair.

## Findings

### High — Replacing a retry journal can make a pending removal unrecoverable

**Authority: spec, “Durable preparation”:** “The required recovery tests cover process termination” and “If a later write fails, stop further destructive work and report the last durable phase plus any subsequent known or uncertain actions.” Acceptance criterion 6 requires crash injection at each phase to leave recoverable records. A process interrupted during an ordinary retry currently leaves a valid archived journal that recovery cannot use.

**Defect class:** replacing the only readable recovery artifact before its successor is complete creates an unrecoverable intermediate state.

In the `worktree` library, [RecoveryStore::reopen](../../lib/src/remove/recovery/store.rs:213), which starts another removal attempt, renames `journal.jsonl` to an archive before creating and writing its replacement. [RecoveryStore::recover](../../lib/src/remove/recovery/store.rs:251) reads only `journal.jsonl`. Termination or a write failure between those steps leaves the pending record discoverable but prevents inspection and retry, even though the old valid journal remains beside it.

I copied the store's existing fixture, recorded one confirmed deletion, and reproduced each intermediate state using the same owner-only permissions as production. Each case then called the public recovery API. The sibling callers all use this same replacement operation.

| Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- |
| Initial `begin` | Complete initial journal; abandoned staging directory | Valid operation recovers; staging directory is ignored | Same; clean control |
| `reopen`: after archiving | Replacement absent | Recovery fails with `StorageProblem::Missing` | Recover the previous valid attempt without authorizing new deletion |
| `reopen`: after file creation | Empty owner-only replacement | Recovery rejects missing header | Previous valid attempt remains recoverable |
| `reopen`: during initialization | Header only; header plus incomplete inventory | Recovery rejects missing inventory in both cases | Previous valid attempt remains recoverable |
| `reopen`: complete replacement | Valid header, inventory, and recorded result | Recovery returns the confirmed deletion | Same; clean control |
| Retry deleting a quarantine, retry removing registration, retry performing branch actions, and `release_unstarted` | Each calls `reopen`; same absent/empty/incomplete replacement states | Each inherits the reader's failure before it can proceed | Each retains a readable previous generation |

Build and flush the replacement under a temporary name while the old journal remains readable, then publish it through a recoverable transition. Preserve the old generation until publication is established. Add fault-injection tests at these internal boundaries, including initialization failure, rather than testing only the boundary after `reopen` returns. This needs no weaker malformed-record policy or broad fallback to arbitrary files.

### High — Completion discards removal history and confirmed deletion details

**Authority: spec, “Recovery report and retry”:** “The persistent report includes … timeline and completed steps; initial and later Sniff evidence with coverage; observed activity; an operation-owned deletion journal” and “Summary truncation must not discard confirmed journal entries.” Acceptance criteria 8 and 11 require accurate partial-deletion reports and preservation of confirmed versus uncertain results. A normal interrupted removal followed by a successful retry loses that distinction permanently.

**Defect class:** replacing an operation's history with its latest attempt, then deleting the supporting journals, loses durable evidence of earlier mutations.

In the `worktree` library, [Resumed::report](../../lib/src/remove/lifecycle/resume.rs:337), which prepares the retry report, starts an empty report. [Attempt::summarize_journal](../../lib/src/remove/lifecycle.rs:891) counts only the current journal. [Operation::complete](../../lib/src/remove/recovery/store.rs:349) then removes the current and archived journals. Earlier diagnostics, activity, timeline, deletion counts, and uncertainty are not carried forward into the final report. The initial discovery query still exists in `record.json`; that does not retain the other missing history.

I swept all three resumable states using the existing lifecycle repository fixture. For the registration-only case, I also removed the last result line exactly as the existing crash test does, so its final unlink was uncertain. I inspected the returned report, saved report, and remaining operation files after each retry.

| Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- |
| First stopped attempt | Stop after four ignored files were deleted | Report counts four deletions; complete journal remains | Same; clean control |
| Quarantine deletion retry | Resume that stopped attempt under fresh consent | Final report counts only three more files and three directories; earlier activity disappears; all journals are removed | Retain the four earlier deletions and earlier stop evidence alongside this attempt |
| Registration-only retry | Nine confirmed entries and one uncertain file before retry | Final counts become zero confirmed and zero uncertain; no deletion paths; only `record.json` and `report.json` remain | Preserve nine confirmed entries and one uncertain file without upgrading uncertainty |
| Branch-only retry | Seven files and three directories deleted before interruption | Final deletion count becomes zero; no deletion paths or discovery runs | Retain the completed checkout-removal evidence |
| First successful completion | Existing fixture plus 205 ignored object files | 215 confirmed entries, 200 listed paths, 15 omitted; journal removed | Compact summaries may be capped, but full confirmed entry details remain available |

Preserve attempt history and complete confirmed/uncertain entry details in durable report data before clearing temporary journals. Counts should describe the operation accurately across retries, without double-counting entries or claiming that an uncertain deletion became confirmed. Keep console lists compact. Extend the existing retry tests to assert final evidence, not just final filesystem state and successful completion.

### High — The CLI does not handle interruption at deletion boundaries

**Authority: spec, “Durable preparation”:** “Handle interruption at the next entry boundary; abrupt termination uses the same restart recovery.” The activity section also requires deletion to be interruptible at entry boundaries. Ctrl-C is an ordinary user action, and the shipped CLI currently takes only the abrupt-termination path.

**Defect class:** an implemented cancellation check is disconnected from the production caller's interruption source.

In `worktree-cli`, [execute](../../cli/src/commands/remove/mod.rs:746), which runs both first removals and pending retries, supplies `Environment::interrupted` as `|| false`. The implementation log explicitly defers signal handling because adding a dependency was considered outside the phase. That deferral does not implement the spec's required behavior.

I used the existing CLI recovery fixture and its Git shim to send SIGINT to the actual `wt` process after the checkout moved. The process terminated with signal 2, left one pending operation, and printed no `Full report:` line or post-move recovery guidance. No keyboard injection or terminal focus was involved.

| Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- |
| Library deletion engine | Injected interruption at an entry boundary; existing deletion/lifecycle tests | Stops, preserves remaining content, records cleanup failure | Same; clean control |
| Shipped CLI first removal | SIGINT delivered after move through the existing fixture shim | Process dies immediately; pending record survives; no stopped report | Set cancellation state, stop at the next entry boundary, save and print the partial report |
| Shipped CLI quarantine retry | Same `execute` environment passed to `resume` | Cancellation predicate is also permanently false | Same graceful cancellation source as the first removal |
| Abrupt termination recovery | Existing killed-child lifecycle test | Original-name retry finds the record | Same; retain this separate crash-recovery coverage |

Wire a cross-platform interruption source into the shared environment and keep abrupt termination recovery. Add an L1 real-binary signal test with a deterministic post-move barrier. This tests reaction to a delivered signal, so it does not require Level 3 keyboard injection. Choosing an ordinary implementation dependency does not block the repair on human approval.

### High — Human recovery reports omit process-discovery coverage

**Authority: spec, “Preflight and refusal”:** “Preserve coverage in the operation report and describe it if recovery is needed.” Acceptance criterion 8 requires human reports and structured outcomes to agree about discovery coverage. Partial or unavailable discovery is deliberately allowed to proceed, so this is a supported input rather than an adversarial case.

**Defect class:** recovery message builders omit a required field that their structured inputs already retain.

In `worktree-cli`, [stopped_markup](../../cli/src/commands/remove/recovery.rs:83), which describes a removal that stopped, and [inspection_markup](../../cli/src/commands/remove/recovery.rs:297), which describes a pending removal that cannot continue, never render discovery coverage. They describe local watcher failure when present, but that is different from whether Sniff could inspect other processes.

I used the existing report-rendering fixture, changed only discovery to each supported limited-coverage shape, and called the public builders. The same coverage summaries were passed to the existing [coverage_note](../../cli/src/commands/remove/usage.rs:140) builder as a clean comparison.

| Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- |
| Stopped-removal message | Usable query with exhausted budget; unavailable query; unsupported query | Output is identical to the fixture with no discovery record in all three cases | Concise coverage limitation beside the recovery guidance |
| Pending-inspection message | Same three shapes in the saved record | No coverage limitation in any case | Describe the saved discovery's limits, clearly as earlier evidence |
| Usage-refusal coverage builder | Same three summaries | Reports partial discovery, inability to run, and unsupported discovery respectively | Same; reuse this existing formatting |
| Ordinary successful removal | Existing CLI success fixture | No discovery chatter | Same; keep coverage notes specific to refusal/recovery |

Reuse the existing coverage formatter for recovery messages and distinguish earlier evidence from a diagnostic query made after the stop. Add assertions for the three shapes to the existing L1 report tests. No new discovery query, retry rule, or terminal rendering layer is needed.

## Verification and requirement coverage

Reviewed the specification, implementation plan/log, library lifecycle and recovery store, checked deletion, native observer adapters, CLI wiring, report builders, relevant tests, manifests, and removal documentation. Used the worktree, rust-testing, OS, just, and terminal-harness skills.

Local macOS verification:

- `just test`: **1,358 passed, 15 skipped**, 24.3 seconds.
- `just check-tier-coverage worktree`: **no stranded tests**. The CLI's declared terminal targets require `terminal-tests`, which its CI metadata enables.
- `just test-l2 --test level2_remove`: **11 passed, none skipped**, through detached tmux panes. These include bash/zsh/fish handoff, consent, and failure preservation. One L2 test took 6.4 seconds; the five-second rule concerns L1.
- Seven temporary L1 review probes exercised journal replacement states, all three retry resolutions, report truncation, actual SIGINT, and coverage rendering. Their final runs reproduced the results above. All temporary source edits were restored; no implementation change is included in this review.

The requirements need these verification levels:

| Spec acceptance requirement | Strongest relevant maintained verification | Assessment |
| --- | --- | --- |
| 1. Known usage refuses; force cannot override | L1 policy cases and actual CLI/process fixtures | Appropriate level |
| 2. Limited discovery proceeds; root errors refuse; success remains quiet | L1 library and CLI fixtures | Appropriate level; recovery coverage display is missing |
| 3. Writes into the moved checkout and recreation at the original | L1 real child writers plus CLI recovery fixtures | Appropriate level |
| 4. Modification, replacement, delayed events, overflow, invalidation | L1 injected cases and native observer/deletion tests | Appropriate level |
| 5. Rename, handles, topology, locks, links, association checks | L1 library/native fixtures and injected OS error mapping | Appropriate level; cross-OS evidence remains CI's responsibility |
| 6. Crashes leave recoverable records; original-name retry | L1 phase hooks and killed-child recovery | Missing internal journal-replacement crash boundaries |
| 7. Fresh file consent, branch tip, remote endpoint, no original cleanup | L1 library, CLI, and local remote stand-ins | Appropriate level |
| 8. Human and structured reports agree on actual state and coverage | L1 serialization, snapshots, and CLI report parsing | Missing retained history and coverage projection assertions |
| 9. Inside-target shell handoff and stdout protocol | L1 wrapper tests and L2 actual shell wrappers | Appropriate level; L2 checked locally |
| 10. Missing-directory removal, pending aliases, ambiguity, partial checkout | L1 library and CLI fixtures | Appropriate level |
| 11. Write failures, uncertain unlinks, interruption, branch failures | L1 injected/store/lifecycle tests | Missing CLI signal wiring and final retry evidence assertions |
| 12. Native paths, safe link handling, no replacement, exit classes | L1 native/library tests and CLI exit/report cases | Appropriate level |

The format-reading boundary is JSON recovery records plus JSON Lines journals. The maintained `record_robustness_matrix` and `journal_robustness_matrix` tests exercise the public store result from store-written fixtures, including absent/null/wrong-type/empty/duplicate/trailing-content cases, array element failures, and torn journal tails. Both passed. The journal-replacement finding concerns a valid production transition between files, not permissive parsing. No text grammar or writer input was treated as configuration.

No unrelated implementation cleanup, formatter run, commit, lifecycle-directory move, or change to the preexisting edit in `prompts/_reviews/feature-review.md` was made. No next specification or plan was supplied, so there is nothing to refresh. The spec remains incomplete with `review_iterations: 1`.

## Observations

### Windows Search can refuse an otherwise safe removal

The implementation log records three native Windows runs in which `SearchProtocolHost.exe` briefly held a newly created fixture file and the strict usage check refused removal. This follows the spec's explicit any-holder refusal policy. The production rule is not a defect; the observed test flake and the implementer's proposed retry exception need an author decision, described in frontmatter. Do not silently add process-name exemptions or automatic retries during this repair cycle.

### Windows test timing needs a separate measurement

The implementation log reports numerous CLI tests above five seconds on the native Windows host, including inherited tests outside removal, and estimates another 1.5–2 seconds per removal. The evidence does not isolate a single avoidable bottleneck or establish that all those results are caused by this fix. Measure discovery and watcher-drain cost before proposing a remedy; do not hide slow L1 cases behind tier names, retries, or larger timeouts. No statistical performance gate or cross-OS proof requirement is added by this review.
