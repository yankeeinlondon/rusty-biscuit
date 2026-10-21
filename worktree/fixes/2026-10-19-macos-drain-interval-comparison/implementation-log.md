---
spec: "/Volumes/coding/wt/rusty-biscuit/feat-pipeline/worktree/fixes/2026-10-19-macos-drain-interval-comparison/spec.md"
plan: "worktree/fixes/2026-10-19-macos-drain-interval-comparison/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages: []
human_review: false
budget: "option 1 (12 runs, no replacement slots), chosen by the author on 2026-10-20"
implemented: true
completed_phase: 4
conclusion: inconclusive (keep 512)
source_files_during_phase_2: []
source_files_during_phase_3: []
source_files_during_phase_4: []
---

# Implementation Log for 2026-10-19-macos-drain-interval-comparison (4 phases)

## Phase 1

Phase 1 fixes the rulings and the run plan. It changes no source, test, or
documentation file. Nothing has been built or measured.

### Budget (R1): option 1

The author chose **option 1** on 2026-10-20: one quick comparison of 12 runs
(3 intervals × 2 settle conditions × 2 runs), fixed up front, with **no
replacement slots**. A slot lost to a startup loss, a refusal, or an
interruption stays in the table as a gap; it is not re-run. The run table
below stands as written.

### Rulings R2–R8

These are the plan's proposals, adopted as written unless noted. Phase 3
follows them; any later change is logged as a deviation.

- **R2, interval order.** Fixed mixed order (table below). The 12 slots are
  four blocks of three; each block runs every interval once, the first three
  blocks rotate (2,048 → 512 → 256, then 512 → 256 → 2,048, then
  256 → 2,048 → 512), and the fourth is the reverse of the first. Each interval
  gets two fresh and two settled runs, and fresh and settled runs alternate
  within each block.
- **R3, durations.**
    - *Elapsed* is the wall time of the whole `wt remove` command, measured by
      the recorder from launch to exit.
    - *Deletion-only* is `content_removed.at_ms − moved.at_ms` from the
      report's `timeline` (milliseconds since the Unix epoch). For a run that
      stops during deletion, the end is the last `deletion_batch` event. If
      either event is missing, deletion-only is recorded as unknown.
    - Every deletion rate uses **deletion-only** duration, and its count is the
      report's `journal.deleted` files plus directories plus links. The spec's
      earlier rates do not state their duration, so they are not compared
      with these.
- **R4, fseventsd CPU.** `ps -o %cpu= -p "$(pgrep -x fseventsd)"`, sampled once,
  immediately before `wt remove` is launched. `ps` reports a decaying average,
  not an instant value; that is accepted as long as every run uses the same
  method. Missing process or command output is recorded as unknown, never 0.
  The 1-minute load average comes from `sysctl -n vm.loadavg` at the same
  moment.
- **R5, settle.** The fixture generator prints a timestamp when tree creation
  ends; the recorder prints one when it launches `wt remove`. The difference
  is the actual settle delay. "0 s" slots launch with no intentional wait;
  "90 s" slots sleep 90 s after creation ends.
- **R6, outcome classes.**
    - `completed`: the removal finished with no coverage loss.
    - `dropped-coverage`: `observation.loss.stage` is `deletion` and the
      reason is an FSEvents drop (`user_dropped`, `kernel_dropped`,
      `must_scan_sub_dirs`, or another named drop flag).
    - `other-observation-failure`: any other observation stop at the deletion
      stage (for example a stall or the 5-minute limit).
    - `startup-loss`: loss at `startup`, `pre_move`, or `post_move`. Not an
      interval trial.
    - `unrelated-refusal-or-interruption`: a refusal, an interruption, or any
      failure not caused by observation. Not an interval trial.

  Only `completed` and `dropped-coverage` count toward interval results.
  A loss at `unregistration` does not stop the removal; the run is
  `completed`, and the loss is noted.
- **R7, wider measurement.** Not scheduled. A second tree size, a second host,
  or more repeats needs a separate author decision.
- **R8, fixture shape.** No helper in the repository builds this tree (the
  `worktree` skill's `fixtures.md` covers small test fixtures only), so Phase 2
  writes a throwaway script outside the repository. Each run gets a new
  repository with a linked worktree holding 1,000 directories × 100 files plus
  one seed file. That is 101,001 directories and files; the spec's 101,002
  presumably also counts the linked worktree's `.git` file. Phase 2 confirms
  the count against the first smoke run's inventory and records it.

### Run table (fixed before run 1)

| Slot | Interval (`drain_every`) | Settle | Binary |
| ---: | ---: | --- | --- |
| 1 | 2,048 | 0 s | `wt-2048` |
| 2 | 512 | 90 s | `wt-512` |
| 3 | 256 | 0 s | `wt-256` |
| 4 | 512 | 0 s | `wt-512` |
| 5 | 256 | 90 s | `wt-256` |
| 6 | 2,048 | 90 s | `wt-2048` |
| 7 | 256 | 90 s | `wt-256` |
| 8 | 2,048 | 0 s | `wt-2048` |
| 9 | 512 | 0 s | `wt-512` |
| 10 | 256 | 0 s | `wt-256` |
| 11 | 512 | 90 s | `wt-512` |
| 12 | 2,048 | 90 s | `wt-2048` |

Per interval: 2,048 runs in positions 1, 3, 2, 3 of its blocks; 512 in
2, 1, 3, 2; 256 in 3, 2, 1, 1. No interval always runs first.

Each run records: slot, interval, actual settle delay, outcome class, confirmed
deleted count, elapsed time, deletion-only time, load average, fseventsd CPU,
and background activity noticed (Spotlight, Time Machine, builds) as context.
A loss also records stage, reason, `raw` flags, every named flag, and
`entries_since_drain`. A drain can come up to 255 entries after the configured
interval because the check runs after a whole journal batch, so the
`entries_since_drain` value for 256 can be about 300 and for 512 about 600.

### Host

- macOS 27.2 (build 26B5091g), Apple M4 Max, 16 cores, 128 GiB memory.
- Fixtures: one fixed parent directory,
  `/Volumes/coding/tmp/drain-interval-comparison/`, on the internal SSD's
  case-sensitive APFS volume that also holds the repository checkout. Results
  go to `results/` under the same parent, outside the repository.
- Source revision for all three builds: `128bddd2b` (the branch head when this
  plan was written). If the branch moves before Phase 2, Phase 2 records the
  revision it actually builds.
- Baseline: `DEFAULT_DRAIN_EVERY` in
  `worktree/lib/src/remove/observation/deletion.rs` is
  `if cfg!(target_os = "macos") { 512 } else { 2048 }`.
- Load average when this plan was written: 3.45 / 4.17 / 4.17.

### Gates

No package file changed, so `just test` and `just lint` were not run. The
scope check from the repository root:

```sh
git status --porcelain --untracked-files=all | awk '{print $NF}' \
  | xargs python3 scripts/ci/affected_scope.py
```

Result: `"packages": []`, `"change_class": "documentation"`. The gates are
skipped as the phase rules allow. The changed files are this log, `plan.md`
(checkboxes and frontmatter), and `spec.md` (untracked with the rest of the
fix directory, unchanged by this phase).

### Requirement-to-test mapping

This phase changes no behavior, so it adds no tests. The Input Robustness
Matrix does not apply: nothing reads a file format. The `claudine` skill was
loaded as instructed; it does not cover this `worktree` work and needs no
update.

### Unfinished

- None. R1 was answered on 2026-10-20 (option 1); Phases 2–4 follow.

## Phase 2

### Builds

All three binaries were built in this checkout with
`cargo build --release -p worktree-cli` (rustc 1.98.1, cargo 1.98.1), one
after another from source revision `f4cf0ebe7`. The only change between
builds was the macOS arm of `DEFAULT_DRAIN_EVERY`. The script restored
`deletion.rs` before each build and once more at the end.

| Binary | macOS `DEFAULT_DRAIN_EVERY` | SHA-256 | Size (bytes) |
| --- | ---: | --- | ---: |
| `wt-2048` | 2,048 | `d6c6c011cbab9714130cb16daab913f47367780394e497aae996842b8f5039ea` | 30,882,768 |
| `wt-512` (baseline) | 512 | `93ec855d7ee0c41c9867549bf18082e05f670a53b11c6ea8ec44b51b5c6aa2e5` | 30,882,768 |
| `wt-256` | 256 | `5fece910194666332051651bfb37cb3daaf5b5eb353a9eaf310d9d0f1e61f1ea` | 30,882,768 |

`cmp -l` finds 82 differing bytes between `wt-512` and `wt-2048` and 80
between `wt-512` and `wt-256`. That fits a change to one constant plus the
code signature. `git diff -- worktree/lib` was empty after the builds.

### Harness

Both scripts live outside the repository, under
`/Volumes/coding/tmp/drain-interval-comparison/`. `run_slot.py` creates the
fixture and records the run, and `run_all.sh` runs the fixed table.

- **Fixture.** Each run gets a new repository whose `main` holds one empty
  commit, plus a linked worktree `big` holding 1,000 directories × 100 files
  and `seed.txt`, all committed. Tag `keep-big` makes the branch "Pretty
  safe", so a non-interactive run can delete it without a force flag. Every
  run's journal confirmed **101,002** deleted entries: 100,001 files plus
  the worktree's `.git` file, and 1,000 directories.
- **Recorder.** It sleeps the settle delay, then samples
  `sysctl -n vm.loadavg`, the fseventsd `%cpu` (R4), and the five busiest
  processes (context only). It then runs
  `wt-<n> remove big` from the base checkout with stdin closed and `CI=1`,
  copies `.git/wt-removal/` to `results/<slot>/`, and only then deletes the
  run directory.
- **Smoke runs.** One small tree (10 × 10) per binary. All three completed
  with exit 0 and every field recorded.

## Phase 3

### Aborted first attempt (harness defect, not counted)

The first pass of the table stopped after slots 1 and 2. Both were refused
with **exit 3, "Removal paused: other processes are using big"**, before
anything moved. Two causes, both in the harness:

- **Spotlight.** Several `mdworker_shared` processes held fixture files open.
  The first fixture parent, `/Volumes/coding/tmp/drain-interval-comparison/runs/`,
  is indexed. The refusal still happened in slot 2, after the full 90 s
  settle.
- **Git auto-maintenance.** Two `git` processes had the worktree as their
  working directory. They match the detached `git gc --auto` that a commit
  of 100,000 loose objects starts. This likely also explains run 1a of the
  earlier `2026-10-18-remove-observation-under-load` sample ("two
  short-lived `git` processes").

The defect would have refused every slot, whatever its interval or settle,
and no interval data had been collected, so the **whole table was restarted
from slot 1**. No single slot was replaced. The aborted attempt's output is
kept under `aborted-attempt-1/` beside `results/`.

Harness fixes for the restart:

- fixtures moved to `runs.noindex/`, which Spotlight skips;
- every fixture `git` call uses `-c gc.auto=0 -c maintenance.auto=false`;
- the base commit is empty, so the linked worktree holds exactly 101,002
  entries (with a `README` it was 101,003).

A fixture-only check under the new location found no process holding the
tree 2, 4, and 6 s after creation.

### Runs

Times are in seconds. *Deletion* is `content_removed − moved` (R3), and the
rate is entries per second over that time. *RT* is
`longest_round_trip_ms`. fseventsd CPU is `ps %cpu`, a decaying average.
Every run deleted 101,002 entries, exited 0, and ended `removed` with
`loss: null`.

| Slot | Interval | Settle (actual) | Class | Elapsed | Deletion | Rate | Load (1/5/15 min) | fseventsd CPU | `startup_ms` | RT |
| ---: | ---: | ---: | --- | ---: | ---: | ---: | --- | ---: | ---: | ---: |
| 1 | 2,048 | 0.08 | completed | 11.43 | 10.06 | 10,040 | 11.84 / 11.90 / 8.04 | 22.9 | 6 | 1,014 |
| 2 | 512 | 90.13 | completed | 26.00 | 24.41 | 4,138 | 5.45 / 9.08 / 7.49 | 0.0 | 11 | 1,020 |
| 3 | 256 | 0.08 | completed | 44.07 | 42.71 | 2,365 | 5.91 / 8.52 / 7.40 | 20.9 | 12 | 1,015 |
| 4 | 512 | 0.08 | completed | 16.38 | 15.04 | 6,716 | 4.57 / 7.37 / 7.06 | 23.0 | 12 | 1,011 |
| 5 | 256 | 90.17 | completed | 45.52 | 43.94 | 2,299 | 4.09 / 6.22 / 6.65 | 1.5 | 12 | 1,016 |
| 6 | 2,048 | 90.12 | completed | 6.18 | 4.82 | 20,955 | 5.02 / 5.98 / 6.49 | 0.0 | 11 | 16 |
| 7 | 256 | 90.13 | completed | 26.63 | 25.27 | 3,997 | 4.79 / 5.83 / 6.37 | 2.6 | 12 | 1,014 |
| 8 | 2,048 | 0.08 | completed | 10.23 | 8.89 | 11,361 | 5.12 / 5.69 / 6.27 | 20.9 | 12 | 1,013 |
| 9 | 512 | 0.08 | completed | 20.79 | 19.41 | 5,204 | 5.64 / 5.69 / 6.23 | 21.5 | 12 | 1,013 |
| 10 | 256 | 0.08 | completed | 25.45 | 24.09 | 4,193 | 5.65 / 5.57 / 6.14 | 18.5 | 12 | 1,013 |
| 11 | 512 | 90.13 | completed | 13.24 | 11.92 | 8,473 | 3.80 / 5.35 / 6.00 | 0.0 | 11 | 1,011 |
| 12 | 2,048 | 90.13 | completed | 11.31 | 9.98 | 10,120 | 5.38 / 5.65 / 6.03 | 0.1 | 12 | 1,012 |

No run was a startup loss, a refusal, or an interruption, so the table has no
gaps. No extra runs were made beyond the budget.

Background context: the host had been up 28 days, and fseventsd for 1 day
20 h. WindowServer, the aerial-wallpaper extension, and the terminal were
the steady top consumers. `mds` was in the top five at launch in slots 3, 4,
8, 9, and 10, all 0 s slots, though the fixtures themselves were not
indexed. Slots 5 and 7 overlapped bursts from Stocks, `cloudd`, and NewsTag.
No builds ran during the table, and nothing was induced.

Artifacts: `/Volumes/coding/tmp/drain-interval-comparison/results/slot-NN/`
(`run.json`, stdout, stderr, the copied `wt-removal/` record and report) and
`results/summary.json`.

## Phase 4

### Summary per interval

| Interval | Runs | Completed | Dropped | Deletion time, fresh (0 s) | Deletion time, settled (90 s) | Median deletion | Median elapsed |
| ---: | ---: | ---: | ---: | --- | --- | ---: | ---: |
| 2,048 | 4 | 4 | 0 | 10.06, 8.89 | 4.82, 9.98 | 9.4 | 10.8 |
| 512 | 4 | 4 | 0 | 15.04, 19.41 | 24.41, 11.92 | 17.2 | 18.6 |
| 256 | 4 | 4 | 0 | 42.71, 24.09 | 43.94, 25.27 | 34.0 | 35.4 |

- **Coverage loss: none at any interval.** No drop means there are no
  stages, flags, or `entries_since_drain` values to tabulate.
- **Time cost: clear, and in the expected direction.** Every 2,048 run
  deleted faster than every 512 run, and every 512 run except slot 2 deleted
  faster than every 256 run. Slot 2 (24.41 s) overlaps the fastest 256 runs
  (24.09, 25.27 s). Against 2,048, the medians put 512 at about +8 s and 256
  at about +25 s per 101,002 entries. The 256 runs fall into two clusters
  (about 24 s and about 43 s), and settle time does not explain it.
- **Fresh trees.** At launch, fseventsd showed 18–23 % CPU on every 0 s
  slot and 0–2.6 % on every 90 s slot. That fits the working explanation
  that fseventsd is still digesting the creation burst. Even so, 2,048
  deleted fresh trees at about 10,000–11,400 entries per second (slots 1
  and 8) without loss. That is faster than the 6,600–7,000/s of the
  original incidents.
- **Round trips.** `longest_round_trip_ms` was about 1,010–1,020 in 11 of
  12 runs, and 16 in slot 6. This matches the documented fast-path-or-1 s
  pattern and does not separate the intervals.
- The spec's earlier samples, including the two losses, are context only and
  are not pooled with these. Their fixture location and the `git gc`
  confound noted above differ from this harness.

### Conclusion: **Inconclusive. Keep 512.**

Twelve runs showed no coverage loss at any interval, so this sample cannot
say whether the interval changes how often coverage is lost. It does **not**
show that the interval has no effect: the earlier losses were intermittent,
and 12 clean runs are consistent with a small loss rate at any of the three
intervals. What the sample does show is the price of each interval. On this
host and this tree shape, 512 costs about 8 s more than 2,048 per 101,002
entries, and 256 about 25 s more.

Still unknown:

- the loss rate at each interval, which would need far more runs (R7, an
  author decision);
- whether the earlier fresh-tree losses depended on conditions this harness
  removed, namely Spotlight indexing the fixture, Git auto-maintenance, or
  heavier host load (load averages of 17–19 then, at most 11.8 here);
- whether the 256 bimodality comes from the interval or from the host.

The shipped default, tests, and docs are unchanged. A change of default would
need the separate author decision that the plan describes.

### Observations for the author (not acted on)

- **A newly created large worktree in a Spotlight-indexed location cannot be
  removed for a while.** `mdworker_shared` holds its files, and `wt remove`
  correctly pauses with exit 3. In the aborted attempt this was still true
  90 s after creation. This is the safety rule working as designed, but
  `docs/cli/remove.md` does not name Spotlight as a usual holder. A
  sentence there could save a user some confusion.
- **The `biscuit-hash` CLI cannot hash a binary file.** `bh --file` /
  `bh -c --file` fails with "stream did not contain valid UTF-8".

### Deviations

- **Source revision** `f4cf0ebe7`, not `128bddd2b`. The branch moved by one
  planning-only commit (this fix's spec, plan, and log); no source differs.
- **Binary hashes** use `shasum -a 256`, because `bh` rejects non-UTF-8
  files (above). The plan asked for `biscuit-hash`.
- **Restarted table.** Recorded under Phase 3: the first attempt was aborted
  after two harness-caused refusals, and the table restarted whole, with no
  selective replacement.
- **Fixture location** `runs.noindex/` under the planned parent, not
  `runs/`, and an empty base commit. Both are Phase 3 harness fixes.

### Final check

- `git diff -- worktree/lib` is empty. No flag, environment variable, or
  setting was added.
- Under the fix directory, only `implementation-log.md` and `plan.md`
  changed; `spec.md` changed only in its frontmatter (`implemented`,
  `status`).
- No package changed, so `just test` and `just lint` were not run (the plan
  says they are not needed).

## Author decision: macOS returns to 2,048 (2026-10-20)

After reading the conclusion, the author chose to drop the macOS-specific
interval. The sample shows a consistent time cost for 512 and no loss benefit
it could measure, and a loss is a resumable stop, not lost work. This is the
plan's "Conditional follow-up", done as a separate change after the
measurement:

- `DEFAULT_DRAIN_EVERY` is now 2,048 on every platform
  (`worktree/lib/src/remove/observation/deletion.rs`), with its doc comment
  rewritten to give the reason;
- the cadence test is now
  `the_default_drain_interval_is_2048_on_every_platform`;
- `worktree/docs/cli/remove.md` ("When fseventsd is slow", "Checked
  deletion", and the report example), `.claude/skills/worktree/observation.md`
  (which also records the harness traps) and `.claude/skills/os/macos.md`
  were updated to match.

`just test` in `worktree/` passed (1,594 run, 15 skipped), and `just lint` was
clean. The Phase 4 final check above ("`lib/` is clean") described the
measurement itself and no longer holds for the working tree.
