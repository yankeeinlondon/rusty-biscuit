---
$schema: feature-review.yaml
reviewed_by: codex/gpt-6.1-sol
created: "2026-10-22T16:01:33-07:00"
spec: 2026-10-22-remove-large-checkout/spec.md
fix: 2026-10-22-remove-large-checkout/review-1.md
implemented: true
ready: false
description: "A **fix** review of `2026-10-22-remove-large-checkout/spec.md`"
recurrence: false
findings: []
observations: []
human_review: true
human_resolved: true
has_blocked_findings: false
blocked: false
human_in_the_loop:
    - |-
        **Decide whether this fix must also make large removals finish in one run on macOS.**

        The implementation log reports that the journal grew to 265 MB without reaching its former 64 MiB limit. However, all three attempts to remove a disposable 500,000-file checkout stopped partway because macOS dropped file-change notifications. The safety checks stopped deletion because they could no longer establish that another process had not changed the checkout. Retrying continued the removal; one-run completion was not demonstrated.

        This matters because the specification promises one removal command with no retry. The implementer deferred changing how quickly deletion proceeds relative to file-change observation and requested a scope decision. Choose one:

        - **Extend this fix to address deletion pacing and repeat the acceptance run.** Keeps the promised outcome intact, but requires further design and testing of the observation engine.
        - **Keep the implemented journal and process-check improvements, and schedule observation behavior separately.** Keeps this change focused, but explicitly revises the completion boundary: large macOS checkouts may still require retries until the follow-up lands. That limitation must be stated rather than treating the original one-run outcome as achieved.
        - **Accept retries as an ongoing limitation and document them.** Requires no observation-engine changes, but gives up the specification's one-run outcome.

        Recommendation: schedule the observation work separately if the immediate journal and process-check improvements are useful on their own, while explicitly acknowledging that the original completion outcome remains unresolved. Otherwise extend this fix. Do not weaken the safety response to lost notifications merely to make deletion finish.
human_review_items:
    - |-
        Choose whether to extend this fix to make a 500,000-entry macOS removal finish in one run, schedule that work separately while documenting that retries remain necessary, or accept retries as an ongoing limitation. The options and their consequences are explained in `human_in_the_loop` and in the review body.
log: /Volumes/coding/wt/rusty-biscuit/fix-remove-large-checkout/worktree/fixes/2026-10-22-remove-large-checkout/log.md
implemented_by: claude/opus
next: 2026-10-22-remove-large-checkout/review-2.md
---

# Review 1: scope decision required

The review stopped at the requested initial decision check. Two independent
reviewers examined the specification and plan, and the implementation log.
The specification has no unresolved design questions, but the log records a
newly observed obstacle to its headline outcome and explicitly defers a repair
pending a scope decision.

Production readiness has **not been assessed**. No implementation comparison,
code findings, or fresh test execution was performed. The `ready` property is
intentionally unset at this early-stop stage: neither approval nor rejection
based on code findings has been established. The specification is not marked
completed.

## Decision needed

The specification says a 500,000-entry checkout should be removed in one run,
including its registration and approved branch actions, with no retry. The
[implementation log](implementation-log.md#500000-entry-acceptance-run-macos)
reports these outcomes on a disposable macOS checkout:

| Build and attempt | Reported result |
| --- | --- |
| Debug, first attempt | Stopped after 204,750 entries were deleted |
| Debug, retry | Stopped with 310,500 entries deleted cumulatively |
| Release, fresh checkout | Stopped after 130,500 entries were deleted |

The log attributes every stop to dropped macOS file-change notifications.
The journal exceeded its old ceiling successfully, but deletion still stopped
because losing those notifications removes the evidence needed to continue
safely. The faster release build also stopped, so this was not limited to
debug-build performance. These are implementation-reported results; this review
did not independently repeat them.

The implementer chose to leave deletion pacing unchanged and ask whether this
newly exposed obstacle belongs in this fix or in a follow-up. That is a
meaningful change to the completion boundary, not a routine implementation
choice. It requires the author's decision before a normal review continues.

The options are:

1. **Extend this fix.** Design how deletion should keep pace with observation,
   then repeat the large-checkout acceptance run. This preserves the original
   outcome but increases the scope of this change.
2. **Schedule observation work separately.** Review the journal and process-check
   improvements as a focused change, explicitly recording that large macOS
   removals may still require retries. The original one-run outcome remains
   unresolved until the follow-up succeeds.
3. **Accept and document retries.** This avoids further code changes but changes
   the promised outcome permanently.

The implementation log recommends the second option. That is reasonable if
the current improvements are worth releasing independently; it must not be
represented as meeting the original one-run acceptance outcome. If that
outcome is required for this fix's closure, choose the first option.

## Routine decisions and deferred verification

The reviewers also identified smaller choices that do not warrant a separate
human decision: report-version increments; rejecting explicit null provenance;
preserving unknown provenance when old reports are saved again; a focused
free-space API with injectable test readings; sharing relative-path identity
indexes; and borrowing inventory data during serialization. These choices
implement the specified behavior without introducing a new user-facing policy.

Additional performance measurements and a 500,000-entry acceptance run on every
operating system were explicitly left unscheduled. Local Linux and WSL2
verification was left to CI. Those verification choices do not independently
require human review under this review's rules.

## Human rulings

Ruled by the author on 2026-10-22: **close this fix on a revised completion
boundary, and move the macOS stop into a new feature spec.**

The author also set the rule the new spec builds on: **a confirmed
`wt remove` always deletes.** Interactive runs warn rather than block. The
confirmation prompt shows everything `wt` can see (processes using the
checkout, uncommitted files, anything unusual), and a "yes" deletes. The only
reasons a removal stops are the user's own Ctrl-C and, on Windows, waiting on
a named program that holds a file open so it physically cannot be deleted.
Non-interactive runs refuse before anything changes when there is something
to warn about, unless `--force` is given. That rule is specified in
`2026-10-22-confirmed-remove-always-deletes`.

Completion boundary for this fix, which replaces the spec's "one run, no
retry" outcome:

- the journal's size can never stop a removal, on any OS (Decision 1);
- the usage check reuses the captured inventory (Decision 3);
- the serialized-report compatibility and the merged discovery line
  (Decisions 3 and 4) work as specified;
- a stop on macOS after dropped notifications is **out of scope** here. It
  remains a resumable stop until the new spec removes it, and this fix's
  docs must say so rather than claim one run.

Decision 2's free-space refusal is reviewed as built. The new spec replaces
it, because a refusal unrelated to processes conflicts with the author's
rule. A review finding must not ask this fix to remove it.
