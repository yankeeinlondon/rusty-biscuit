---
$schema:
    status: |-
        enum(
            draft-spec,
            finalized-spec,
            planned,
            implemented,
            review-findings,
            human-in-the-loop,
            completed,
            on-hold,
            abandoned
        ) -> an indicator of progress for this specification
    reviewed: boolean -> indicates whether the specification file has been reviewed by another agent from the one which created the spec
    reviewed_by: string -> the agent and model used in the spec review
    reviewed_on: date -> the date the spec was reviewed
    review_iterations: number -> the number of implementation reviews have taken place in the review/fix cycle
    clarified: boolean -> indicates whether the specification was built -- _in part_ -- with the 'clarify.md' prompt
    implemented: boolean -> indicates whether this spec's plan has been implemented
    implemented_by: string -> the agent who implemented the plan
status: draft-spec
reviewed: true
reviewed_by: codex/gpt-6.1-sol
reviewed_on: 2026-10-19
review_iterations: 0
implemented: false
related:
    - 2026-10-18-remove-observation-under-load
---

# Does the macOS drain interval change how often `wt remove` loses FSEvents coverage?

`2026-10-18-remove-observation-under-load` lowered the macOS deletion
interval from 2,048 to 512 entries. The `worktree` library's
[DeletionOptions::drain_every](../../lib/src/remove/observation/deletion.rs)
controls when deletion pauses to receive and check file-change notifications
(a drain). The earlier fix's
regression sample showed that 512 does not prevent loss. The author accepted
512 as a mitigation that lowers pressure without preventing loss, and asked
for this comparison as a follow-up. No sample so far compares intervals
under the same conditions, so whether the interval matters at all is
unknown.

## Evidence so far

Each sample used a 101,002-entry linked worktree (1,000 directories × 100
files plus a seed file) on the macOS development host, with a release `wt`
and the 512 interval.

| Date | Tree | Result | Notes |
| --- | --- | --- | --- |
| 2026-10-18 | fresh (about 1 min after creation), load average about 17 | lost after 18,600 deletions at about 2,900/s | `UserDropped` + `MustScanSubDirs`, `entries_since_drain` 600 |
| 2026-10-18 | retry of the same tree | completed at about 8,200/s | no loss |
| 2026-10-18 | settled 90 s | completed in 24.7 s at about 4,400/s | no loss |
| 2026-10-19 | fresh (0 s settle), load average about 19 | completed in 25.3 s | no loss |
| 2026-10-19 | settled 90 s, load average about 4 | completed in 21.2 s | no loss |
| 2026-10-19 | fresh, stream latency temporarily 0.5 s | lost after 72,600 deletions | `UserDropped` + `MustScanSubDirs`, `entries_since_drain` 600; latency differs, so only a loose data point |

Both recorded losses happened on freshly created trees, and one loss had a
lower reported deletion rate than successful runs. These observations do not
isolate deletion rate, tree age, or host load as the cause. The working
explanation, that fseventsd (the macOS file-change service) is still processing
the creation burst, is unconfirmed.

Marker round trips cannot serve as a pressure signal here. On macOS a
round took about 11 ms or about 1 s, with nothing between in the recorded
probe (see the [worktree observation guidance](../../../.claude/skills/worktree/observation.md)).

## Scope and comparison method

This is a manual investigation of a known macOS-specific cost on the same
macOS development host. It adds no CI gate, other host, workload, or automated
large-tree regression test. The existing figures show that 512 can still lose
coverage; they do not answer whether changing the interval affects losses
under comparable conditions.

Keep the proposed comparison bounded. The run count is an author decision in
Open Questions; do not interpret “enough runs” as permission to keep sampling
until a preferred outcome appears.

- Fresh (0 s settle) and settled (90 s) trees, with `drain_every` at 2,048,
  512, and 256. Resolve the run-count question before starting; no repetition
  count or statistical threshold is established by this draft.
- Use a newly generated, equivalent full tree for each run. A retry has fewer
  surviving entries and a different event history, so it is useful recovery
  evidence but cannot stand in for an independent interval comparison.
- Define settle time from the end of fixture creation to launching `wt remove`.
  Record the actual delay: “0 s” means no intentional wait, not that deletion
  begins immediately. Startup checks and inventory capture still take time.
- Use the same release source revision, build settings, filesystem location,
  fixture contents, FSEvents latency (0.1 s), journal durability, wait policy,
  and queue capacity. Vary only the interval and the already proposed settle
  time. Predetermine a mixed interval order so one interval is not always
  tested first. Do not manufacture host load or restart fseventsd between
  samples; record background activity as context.
- Per run: configured interval, actual settle delay, outcome, confirmed deleted
  count, elapsed time, load average at start, and fseventsd CPU. State the CPU
  sampling method consistently; unavailable measurements are unknown, not zero.
  Record the command's elapsed time separately from any deletion-only duration,
  and name which duration a reported deletion rate uses.
- Preserve existing reports and journals before fixture cleanup. For a loss,
  record its stage, reason, raw macOS flags and all named flags, and
  `entries_since_drain` when available. The `worktree` library's
  [ObservationDiagnostics](../../lib/src/remove/observation/diagnostics.rs)
  already records this observation context; its entry count is entries
  processed since the previous drain, not total deletions or queued events.
- Distinguish a completed removal, dropped FSEvents coverage, other observation
  failures, and unrelated refusals or interruptions. A startup loss is not a
  deletion-interval result. Keep such runs visible rather than silently
  replacing or counting them as successful interval trials.
- Keep stop-on-loss unchanged. This follow-up measures. It does not change
  recovery behavior.

### Exercise the actual deletion path

The `worktree` library's
[removal lifecycle](../../lib/src/remove/lifecycle.rs) currently constructs
`DeletionOptions::default()` internally. Setting an option in a separate
microbenchmark would not exercise the full removal. For this investigation,
use local release builds that differ only in the macOS default interval,
identify each build and its interval in the results, and restore the baseline
source afterward. Do not add a public CLI flag, environment variable, or
persisted setting just to select a sample's interval.

Keep the existing batch and depth rules. The interval is checked after a
journal batch, which contains entries from one directory and at most 256
entries. A drain may therefore exceed the configured interval by up to 255
entries; depth-level drains can also occur sooner. For example, consecutive
100-file batches can reach the 512 threshold at 600. Configured 256 can
similarly mean 300. Do not shrink batches to make the labels exact: that would
change journal work as well as drain frequency.

A macOS drain waits for a quiet marker round, but is not proof that every
previous filesystem event has arrived. Keep direct entry checks, cumulative
FSEvents flag handling, and all existing stop and recovery rules unchanged.

## Outcome and completion criteria

Record the run plan, build identities, per-run observations, deviations, and
conclusion in an implementation log alongside this spec. No comparison has
been run as part of this review.

Choose one of these evidence-limited conclusions:

- **Keep 512.** The sample gives no reason to change the accepted mitigation.
- **Recommend another interval.** Explain the observed loss behavior and
  elapsed-time trade-off, including contradictory runs. This investigation
  recommends a default change; it does not silently ship one.
- **Inconclusive.** The sample shows no clear difference, or host variation or
  unrelated failures prevent a useful comparison. Keep 512 and state what
  remains unknown. Do not conclude that the interval has no effect.

Reader's note: the original outcome required choosing between interval tuning
and only pacing or recovery. A small observational comparison cannot rule out
an interval effect or establish that those are the only remedies. Any further
pacing design needs a reliable signal; automatic continuation after a drop
needs a separate safety design. Neither is authorized here.

If the author subsequently selects a shipped default change, update the
`worktree` library's default and cadence tests, `worktree/docs/cli/remove.md`,
and `.claude/skills/worktree/observation.md` together. Retain the 2,048 default
on Linux, WSL2, and Windows. Use the existing package-area test and lint
recipes for that implementation; this document-only review needs no runtime
or performance test. Do not copy provisional findings into user documentation
as guarantees.

## Open Questions

### How much sampling should this comparison authorize?

The draft asks for enough runs to see a difference without setting a bound.
Losses are intermittent, so a quick sample can be inconclusive. The author
must choose the budget before execution; this review adds no runs or
statistical requirement.

1. **One quick comparison on the existing host (recommended).** Keep the
   intervals and settle conditions already listed and agree on a small fixed
   run budget before starting. Pros: proportional to a tuning investigation,
   exposes obvious time costs, and requires no permanent measurement system.
   Cons: may observe no losses or give conflicting results; it cannot estimate
   a reliable failure probability. This is the recommended starting point
   because the immediate decision is whether there is evidence to revisit 512.
2. **An author-budgeted repeated study on the same host.** Agree on the run
   count and interpretation before starting. Pros: more opportunities to
   observe intermittent losses. Cons: more time and filesystem activity, host
   load still confounds results, and no guarantee of a decisive answer.
3. **Defer measurement and retain 512.** Pros: no investigation cost and the
   limitation is already documented. Cons: the effect of the interval remains
   unknown and the requested comparison is not completed.

Until the author chooses a budget, the comparison remains unexecuted and the
current default and safety behavior remain in force.
