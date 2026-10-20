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
human_review: true
human_review_items:
    - |-
      **Choose the run budget before any measurement runs (the spec's open question).**

      The spec says the comparison stays unexecuted until you pick how many runs it may use. Phase 2
      builds three test copies of `wt` and Phase 3 runs them, so the budget has to be fixed first;
      otherwise there is a temptation to keep sampling until a preferred answer shows up.

      - **Option 1 – one quick comparison, 12 runs (recommended).** 3 intervals (2,048, 512, 256)
        × 2 tree ages (fresh, settled 90 s) × 2 runs each, in the fixed order in the log's run
        table. About 25 s per run plus 90 s waits on half of them, so roughly 15–20 minutes of
        runs, done one at a time. Pro: cheap, shows any large difference or obvious time cost.
        Con: losses are occasional, so 12 runs may see none and end "Inconclusive".
      - **Option 2 – a larger fixed count, e.g. 4 runs per cell (24 runs).** Pro: more chances to
        see the occasional loss. Con: about twice the time and disk churn, host load still mixes
        into the results, and it still may not decide anything.
      - **Option 3 – defer.** Keep 512 and close the plan. Pro: no cost. Con: whether the interval
        matters stays unknown.

      Recommendation: **Option 1**, because the only decision on the table is whether there is any
      evidence to revisit 512, and the spec recommends it too. Also say whether a slot lost to an
      unrelated cause (a loss while watching starts, a refusal, an interruption) may be re-run as a
      replacement; the proposal below is **no replacements**: the gap stays visible.
message_to_agent: |-
  Phase 2 must not start until the author's run-budget answer (R1) is recorded under
  "## Phase 1 → Budget (R1)" in this log; if it is option 3, close the plan instead.
  The run table and R2-R8 rulings are fixed in this log. For deletion-only duration use the
  report's `timeline` events (`moved` to `content_removed`, `at_ms`), defined in
  `worktree/lib/src/remove/outcome.rs` (`Step`, `TimelineEvent`). The report lives at
  `<git-common-dir>/wt-removal/<operation-id>/report.json`; copy it out before deleting a
  fixture. Confirm the 101,002 entry count against the first smoke run's inventory. Build
  the three binaries from one source tree and restore `deletion.rs` afterward.
---

# Implementation Log for 2026-10-19-macos-drain-interval-comparison (4 phases)

## Phase 1

Phase 1 fixes the rulings and the run plan. It changes no source, test, or
documentation file. Nothing has been built or measured.

### Budget (R1): awaiting the author

The author has not chosen a budget. This session runs without anyone to ask,
so the choice is raised in `human_review_items` and the plan's
"Confirm budget" task stays open. The run table below is written for the
proposed **option 1 (12 runs, no replacement slots)**. If the author picks
option 2, the table is extended with the same rotation before run 1. If the
author picks option 3, the log records "deferred, 512 retained" and the plan
ends. Phase 2 must not start until the answer is recorded here.

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

- R1 and the "Confirm budget" task are open until the author answers
  (`human_review_items`). Phases 2–4 are blocked on that answer.
