---
total_phases: 4
implemented: true
created: 2026-10-19
phase: 1
agent: claude/sonnet
yolo: true
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages: []
---

# Plan: macOS drain interval comparison

## Summary and success criteria

The spec asks one question: does `DeletionOptions::drain_every` (macOS default
512, `DEFAULT_DRAIN_EVERY` in `lib/src/remove/observation/deletion.rs`) change
how often `wt remove` loses FSEvents coverage? Existing samples cannot answer
it because no run compared intervals under the same conditions.

This is a **manual measurement**, not a feature. It ships no code. The work is:

1. fix the run budget and run plan (an author decision),
2. build three local release `wt` binaries that differ only in the macOS default
   interval (2,048, 512, 256), then restore the baseline source,
3. run the budgeted comparison sequentially on the macOS development host,
   each run on a newly generated 101,002-entry tree,
4. write an implementation log with one of three conclusions: **Keep 512**,
   **Recommend another interval**, or **Inconclusive**.

The work is done when:

- [x] the implementation log exists beside `spec.md` and holds the run plan, build
      identities, per-run observations, deviations, and one conclusion;
- [x] `git diff` on `lib/` is empty (baseline source restored, no flag, env var,
      or setting added);
- [x] no shipped default, test, or doc changed (that needs a separate author
      decision, see Phase 4);
- [x] runs that were not valid interval trials (startup loss, refusal,
      interruption) are listed, not dropped or counted as successes.

Nothing here needs `just test` or `just lint` because no source is retained.
The Input Robustness Matrix does not apply: nothing reads a file format.

Constraints carried from the spec: stop-on-loss, batch size (256), depth drains,
FSEvents latency (0.1 s), journal durability, wait policy and queue capacity stay
unchanged. Do not manufacture host load or restart fseventsd. Linux, WSL2,
and Windows keep 2,048 and are out of scope.

## Phase 1: Rulings and run plan

Gate: the author has confirmed the run budget. Without it, stop after this phase
(the spec says the comparison stays unexecuted until then).

### Necessary Rules

Rulings needed before execution. Each carries a proposed answer so the author can
accept it quickly.

- [x] **R1: run budget (the spec's Open Question).** Proposal: option 1, one quick
      comparison. 3 intervals × 2 settle conditions (0 s, 90 s) × 2 runs = 12
      runs, fixed up front, no extension on a preferred or disappointing outcome.
      The author may instead pick option 2 (larger fixed count) or option 3
      (defer; retain 512, plan ends).
- [x] **R2: interval order.** Proposal: predetermine a mixed order, a Latin-square
      style rotation across the 12 slots, so no interval is always first and
      fresh/settled runs interleave. Fix it in the log before run 1.
- [x] **R3: duration definitions.** Proposal: "elapsed" is the whole `wt remove`
      command wall time; "deletion-only" is the deletion stage duration from the
      report if available. State which one any deletion rate uses. If the
      deletion-only duration is unavailable, record it as unknown.
- [x] **R4: fseventsd CPU sampling.** Proposal: one method, `ps -o %cpu -p <pid>`
      for `fseventsd`, sampled at launch of `wt remove`; unavailable is
      recorded as unknown, never zero.
- [x] **R5: how "settle" is measured.** Proposal: timestamp the end of fixture
      creation and the launch of `wt remove`; record the actual delay. "0 s"
      means no intentional wait.
- [x] **R6: outcome classes.** Proposal: `completed`, `dropped-coverage`
      (`UserDropped`/`MustScanSubDirs`/other FSEvents drop at a deletion stage),
      `other-observation-failure`, `startup-loss` (not an interval trial), and
      `unrelated-refusal-or-interruption`. Only `completed` and
      `dropped-coverage` at the deletion stage count toward interval results.
- [x] **R7: wider measurement (author's call, not planned).** The spec limits this
      to one host and one workload. A second tree size, a second host, or a larger
      repeat count is listed here only as an optional author decision. It is not
      scheduled.
- [x] **R8: fixture shape.** Spec says 1,000 directories × 100 files plus a seed
      file in a linked worktree. Proposal: reuse the repo's fixture helpers if
      one produces this; otherwise a throwaway script kept outside the repo
      (see Phase 2).

No performance spike is scheduled. The spec's own figures (above) already give the
cost to expect (about 20 to 25 s per run when it completes), so a probe would add
nothing.

### Tasks

- [x] **Confirm budget** (blocks all later phases)
    - put R1 to the author; record the answer in the implementation log
    - if option 3, record "deferred, 512 retained" and close the plan
- [x] **Write run plan**
    - create `implementation-log.md` beside the spec
    - record R2 to R8 answers, the exact run table (slot, interval, settle) and the
      host (macOS version, hardware, filesystem location of fixtures)
    - the table is fixed before run 1; later changes are logged as deviations

Checkpoint: log has the run table and every ruling answered.

## Phase 2: Harness and builds

Depends on Phase 1. Wave 1 and Wave 2 are independent, so run them concurrently.

### Wave 1: fixture and recording tooling

- [x] **Fixture generator**
    - script that creates a repo plus linked worktree holding 1,000 directories × 100
      files plus a seed file (101,002 entries), fresh per run
    - keep the script outside the repo (or in a temp directory) because nothing
      ships; load the `worktree` skill's `fixtures.md` first for fixture facts
    - emit the end-of-creation timestamp
- [x] **Run recorder**
    - wrapper that sleeps the target settle delay, samples load average and
      fseventsd CPU (R4), launches `wt remove`, records elapsed time, then copies
      the removal report and journal to a results directory before any fixture
      cleanup
    - per run it writes: interval, actual settle delay, outcome class, confirmed
      deleted count, elapsed time, load average, fseventsd CPU
    - for a loss it also extracts the stage, reason, raw macOS flags, all named
      flags, and `entries_since_drain` from `ObservationDiagnostics` output
      (`docs/cli/remove.md` shows the JSON shape)

### Wave 2: interval builds (sequential within the wave; one source tree)

- [x] **Build three binaries**
    - change only `DEFAULT_DRAIN_EVERY`'s macOS arm to 2048, build release, copy to
      `wt-2048`; repeat with 512 (unmodified baseline) and 256
    - same source revision, build settings, and target directory each time
    - record per build: git SHA, the interval, and a binary hash (use `biscuit-hash`,
      e.g. `md hash`)
    - never add a flag, env var, or setting to choose the interval
- [x] **Restore baseline**
    - `git diff -- lib/` must be empty afterward; confirm in the log

Checkpoint: three identified binaries; a smoke run of each against a small throwaway
tree completes and records every field; source clean.

## Phase 3: Execute the comparison

Depends on Phase 2. Strictly sequential, one run at a time, so runs do not load
each other.

- [x] **Run the budget in planned order**
    - for each slot: generate a new tree, wait the settle delay, run the matching
      binary through the recorder
    - never use a retry as an independent trial; a retry may be run as recovery
      evidence only and is labeled so
    - note background activity (Spotlight, Time Machine, builds) as context only;
      do not induce load or restart fseventsd
- [x] **Classify every run** using R6; keep invalid trials visible in the table
    - if a startup loss or refusal happens, log it and run the slot again only if R1
      explicitly allows replacement slots; otherwise leave the gap and say so
- [x] **Preserve artifacts** (reports and journals) under the results directory
      before deleting each fixture

Checkpoint: every budgeted slot has a row (valid or classified invalid); no
extra runs beyond the budget.

## Phase 4: Analysis and conclusion

Depends on Phase 3.

- [x] **Tabulate results**
    - per interval and settle condition: runs, completions, drops, and
      elapsed-time spread
    - for drops: stage, reason, flags, `entries_since_drain`, and the deletion count
      at loss; remember `entries_since_drain` can exceed the configured interval by
      up to 255 (so 256 can mean about 300)
- [x] **Choose a conclusion** (evidence-limited)
    - **Keep 512**: no reason to change the accepted mitigation
    - **Recommend another interval**: describe loss behavior and elapsed-time
      trade-off, including contradictory runs; a recommendation, not a change
    - **Inconclusive**: no clear difference or confounded runs; keep 512, state what
      remains unknown; never claim "no effect"
    - prior samples (the spec's table) are cited as context only, not pooled
- [x] **Finish the implementation log** with run plan, build identities, per-run
      table, deviations, and conclusion
- [x] **Final check**: `git status` shows only the log (and this plan) changed
      under the fix directory; `lib/` is clean

Checkpoint: the log is complete and the conclusion matches the data.

### Conditional follow-up (only if the author picks a new default)

Not part of this plan. If the author selects a shipped default change after reading
the log, a new change must update together: the `DEFAULT_DRAIN_EVERY` macOS value
and the cadence tests in `lib/src/remove/observation/deletion/tests.rs`,
`worktree/docs/cli/remove.md` (the 512 mentions near lines 378 and 436), and
`.claude/skills/worktree/observation.md`. Linux, WSL2, and Windows stay at 2,048.
Verify with `just test` and `just lint` in `worktree/`.
