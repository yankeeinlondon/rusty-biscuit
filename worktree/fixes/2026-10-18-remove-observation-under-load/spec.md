---
status: draft-spec
reviewed: false
review_iterations: 0
completed: false
human_review: false
implemented: false
related:
    - 2026-10-01-remove-busy-folder
---

# `wt remove` gives up when the change watcher is slow or overrun

## Outcome

`wt remove` finishes a large checkout on a Mac whose fseventsd is slow, and
never overruns fseventsd with its own deletions. When watching really does
fail, the message says which part failed and what the user can do about it.
Resuming a removal never shows `wt`'s own deletions as uncommitted work.

## What happened

On 2026-10-18 `wt remove feat/dmls-indexing` (about 240,000 entries, macOS)
took four runs and still did not finish.

| Run | Result | Cause |
| --- | --- | --- |
| 1 | Stopped before the move. Nothing changed. | `CoverageLost::DrainTimedOut`: a check was not confirmed within 5 s |
| 2 | Moved to the hidden name, deleted 2,665 entries, stopped | `DrainTimedOut` |
| 3 | Resumed, deleted 2,242 more, stopped | `DrainTimedOut` |
| 4 | Refused: "473 uncommitted files" | all 473 were tracked files that runs 2–3 had deleted |
| 5 | With `--force-worktree`: deleted 25,845 entries in 3.9 s, stopped | `CoverageLost::Overflow`: "the platform dropped events (its queue overflowed)" |

Runs 1–3 were not caused by `wt`. fseventsd had been running for 26 days and
its `notify` thread was pinned at one full core (`sample` showed it in the
path-matching loop, `strncmp` under a mutex). That was with only about 5
events per second system-wide. An event took more than 10 s to arrive. Quitting
nine watching programs changed nothing; `sudo killall fseventsd` brought it to
0.1 % CPU and same-second delivery. Overall CPU stayed low throughout (one
thread on an efficiency core), so nothing pointed the user at fseventsd. It
took a long manual investigation to find.

Run 5 was caused by `wt`. fseventsd was healthy (0.6 % CPU afterward), and
`wt` deleted about 6,600 entries a second, faster than FSEvents could take
the events, so it dropped some.

### Second occurrence: a modest checkout, healthy fseventsd

Later the same day, `wt remove agent-a9a8d534778b78274` (an agent worktree
with 61,778 entries, 34,545 of them under `target/`) stopped on its first
attempt with `CoverageLost::Overflow`. The timeline in `report.json` shows the
move, then 7,035 entries deleted in about 1.0 s (about 7,000 a second), then
the loss. That is a little over three `DEFAULT_DRAIN_EVERY` intervals. Two
hours later fseventsd was at 0.5 % CPU, eight hours after its last restart.
This was run 5's failure without run 1–3's degraded fseventsd. So it is not
limited to huge trees: any worktree that has been built is large enough to
hit it. Retrying hits the same burst. The removal had to be finished by hand
(`rm -rf` of the hidden directory, `git worktree prune`, `git branch -d`).

The report could not say which drop flag fired, which is the gap Decision 2's
first bullet closes.

## Decisions

### 1. A slow stream is not a lost stream (macOS and Windows)

`DRAIN_TIMEOUT` (5 s, in both `fsevents.rs` and `read_directory_changes.rs`)
is a total budget for one drain's marker rounds. When delivery runs 10 s
late, every drain fails, though nothing was lost.

Replace it with two limits:

- **Stall limit.** Coverage is lost when *nothing* arrives for the stall
  limit: no marker, and no event from a watched path. Any delivery
  restarts the clock. FSEvents rounds that keep bringing in late events from
  `wt`'s own deletions count as progress.
- **Ceiling.** A drain that has not settled within the ceiling is lost, even
  while events keep arriving, so a stream that never goes quiet cannot hold
  a removal open forever.

Proposed values: stall limit 30 s, ceiling 5 min. The stall limit has to be
longer than the worst delivery delay we want to ride out (10 s+ measured
here). See Open Questions.

`CoverageLost::DrainTimedOut` splits into `DeliveryStalled { waited }` and
`DrainCeiling { waited }`. Their messages name the cause in the user's
terms, for example "the system's file-change service (fseventsd) reported
nothing for 30 s".

### 2. `wt` must not overrun FSEvents with its own deletions (macOS)

A drain waits for delivery, so draining more often slows deletion to the rate
FSEvents can keep up with. `DEFAULT_DRAIN_EVERY` (2,048 entries, one or two
events each) lets up to about 4,096 events pile up between drains. That is
also the size of the kernel's FSEvents buffer (`MAX_KFS_EVENTS` in xnu),
which the whole system shares. That match is a hypothesis to confirm, not a
measured fact.

- Record **which** FSEvents flag ended coverage: `KernelDropped`,
  `UserDropped`, or `MustScanSubDirs`. Today all three become
  `CoverageLost::Overflow`, and the report cannot say which happened.
  Keep them separate in the error and in `report.json`.
- On macOS, lower the drain interval until a stress test deleting at least
  100,000 entries finishes without a drop on an idle machine and on one
  running a parallel `cargo build`. Start at 512.
- Linux (inotify `IN_Q_OVERFLOW`) and Windows (zero-byte completion) keep
  their current interval unless the same stress test drops there too.

### 3. Say what failed and what to do about it

When coverage is lost, the stop message and `report.json` include:

- the **measured delivery delay**: how long the last confirmed marker took,
  and the longest seen in this run;
- on macOS, for `DeliveryStalled` or `DrainCeiling`, a hint that fseventsd may
  be degraded and how to check it: one core in `ps -o pcpu= -p $(pgrep -x
  fseventsd)` near 100 % while little is changing, and `sudo killall
  fseventsd` as the fix. On Windows, the equivalent hint names the volume;
- for a drop flag, which flag it was, and whether the drop happened during
  `wt`'s own deletion batch.

The start barrier already measures delivery. When it takes more than a
second, `wt` warns before moving anything that the run may stop for this
reason.

### 4. Resume does not count its own deletions as uncommitted work

Run 4 refused with "473 uncommitted files". Each one was a tracked file
deleted in an earlier run of the same removal, and the journal records all of
them. On resume, a tracked file that is missing from the worktree, and that
the journal says this removal deleted, is not uncommitted work. Anything else
still counts: a file the journal doesn't name, or a modified or untracked
file.

## Non-goals

- Fixing fseventsd itself, or restarting it from `wt`. That needs root and
  affects every program on the machine. `wt` explains; the user decides.
- Removing watching from `wt remove`, or running it without watching. Coverage
  loss still stops the removal.
- Speeding up deletion. Decision 2 deliberately trades speed for keeping
  every change observed.

## Tests

- **Stall vs slow (L1, all OSes, injected).** With the existing `Injector`
  (`delayed` / `deliver_delayed`): delivery delayed past the old 5 s but
  with progress within the stall limit completes, and no delivery for the stall
  limit loses coverage as `DeliveryStalled`. A test-only override makes
  both limits short, so the test runs in well under a second.
- **Ceiling (L1, injected).** A stream that never goes quiet ends in
  `DrainCeiling`.
- **Drop flags (L1, macOS).** Each of the three flags, sent through
  `Shared::deliver`, produces its own `CoverageLost` variant and report value.
- **No self-inflicted drop (L2, macOS).** Remove a generated worktree of at
  least 100,000 entries. The removal completes with no `CoverageLost`. It
  runs serially with the other FSEvents tests (fseventsd serializes stream
  registration) and must not take focus.
- **Resume after partial deletion (L1).** A removal stopped after a deletion
  batch, then resumed: tracked files the journal deleted are not reported as
  uncommitted; a tracked file deleted outside the journal still is.

## Documentation

- `docs/cli/remove.md`, "Watching for changes": replace both "not confirmed
  within 5 seconds" statements (macOS and Windows) with the stall limit and the
  ceiling. Describe the delay warning, the fseventsd hint, and that a resumed
  removal ignores its own deletions.
- `.claude/skills/worktree/observation.md`: `DRAIN_TIMEOUT` and the
  `DrainTimedOut` variant; the split drop flags; the macOS drain interval and
  the stress result that set it.
- `.claude/skills/os/`: add the fseventsd failure mode: a long-running
  fseventsd can pin its `notify` thread at low event rates, delivery lags 10 s+,
  overall CPU looks normal, stopping clients does not help, and `sudo killall
  fseventsd` fixes it.

## Open Questions

1. **Stall limit and ceiling values.** 30 s / 5 min are proposals. A longer
   stall limit leaves a removal open longer while watching is blind, but
   deletion is already paused for that whole time, and the per-entry checks
   still run. Is 30 s acceptable?
2. **Drain interval.** Is lowering it enough, or should `wt` also check
   the stream's own pending event count, where FSEvents exposes one?
3. **Delay warning threshold.** Should a slow start barrier (> 1 s) only warn,
   or refuse up front with exit 4, before anything has moved?
4. **Rescan instead of stopping on a drop.** `MustScanSubDirs` tells a client
   to rescan, not that it has failed. The inventory records every entry, so a
   rescan of the hidden directory could take the place of the missing events.
   It would check that every entry this removal deleted is still absent, that
   every entry it has not reached still matches its record, and that nothing
   else exists. Then deletion would continue on a fresh drain. A change that
   is made and then undone between drains would go unseen, but it leaves
   nothing to lose. A write into an entry before its unlink is already caught
   by the per-entry check. This would make a drop a slower path instead of a
   stop, and the drain interval would only tune speed. It contradicts the
   non-goal "coverage loss still stops the removal", so it needs a decision.
   Should the drain interval (Decision 2) be the only fix, or should a
   bounded number of rescans per removal (for example 3) also be allowed?
