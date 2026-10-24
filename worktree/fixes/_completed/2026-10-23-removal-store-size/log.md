---
implementation_1: "2026-10-23T15:00:36-07:00"
implementation_2: "2026-10-23T16:02:34-07:00"
implementation_3: "2026-10-23T16:25:19-07:00"
---

## Implementation of Review Findings #1

> **started at:** 2026-10-23T14:44:11-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/feat-merman/worktree/fixes/2026-10-23-removal-store-size/review-1.md'
- this is iteration 1 of the review-to-implement cycle 

## Follow-up Pass for Review #1

- the review found in '/Volumes/coding/wt/rusty-biscuit/feat-merman/worktree/fixes/2026-10-23-removal-store-size/review-1.md' indicated that the specification is **production ready**
- the specification can be found at '/Volumes/coding/wt/rusty-biscuit/feat-merman/worktree/fixes/2026-10-23-removal-store-size/spec.md'
- the review's `findings` list is empty: there are no medium or low findings to repair
- its `observations` list has two entries; this pass does not repair observations, so both are carried to the backlog

### Finding Ledger

| ID | Finding | Disposition | Evidence or reason |
| --- | --- | --- | --- |
| O1 | Existing unreadable-journal diagnostics omit the journal filename | backlog | Observation; the behavior predates this fix (`lib/src/remove/recovery/journal.rs`, `JournalInvalid::Malformed`), and the exit-class mismatch (1 vs 3) needs a decision on which class is right |
| O2 | Existing graph tests exceed the L1 time budget | backlog | Observation; the 21 slow tests are in graph code outside this fix's changes |

### Outcome

- no code changed, so there were no focused tests to run and no package area to re-run `just test` / `just lint` in; the review's own run (1,636 passed, lint clean) still describes the tree
- no cross-OS check is needed: nothing changed
- backlog entry written at `worktree/fixes/_unscheduled/removal-store-size-follow-ups/spec.md` with one section per observation
- set `implemented: true` and `implemented_by: claude/opus` on `review-1.md`

## Review #1 Reopened

> **reopened at:** 2026-10-23

- the author reopened review #1: its two observations are now findings `R1-F1` and `R1-F2`, and the review is `ready: false` and `implemented: false`
- the backlog entry at `fixes/_unscheduled/removal-store-size-follow-ups/spec.md` written by the follow-up pass above was deleted; both items are repaired in the next cycle instead

## Implementation of Review Findings #1

> **started at:** 2026-10-23T15:00:36-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/feat-merman/worktree/fixes/2026-10-23-removal-store-size/review-1.md'
- this is iteration 1 of the review-to-implement cycle 
- the review's `implemented` flag was confirmed `false` before repairs started

### Finding Ledger

| ID | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| --- | --- | --- | --- |
| R1-F1 | Unreadable-journal diagnostics omit the journal filename and disagree on exit class | fixed | R1-F1.1 and R1-F1.2 fixed and verified; sibling sweep clean |
| R1-F1.1 | Every unreadable-journal error names the file it read (`journal.jsonl` or `journal-<n>.jsonl`) as well as the line | fixed | `StoreError::Journal { path, reason }` renders the path; lib test `an_unusable_journal_error_names_the_current_or_archived_file` (current and archived); CLI tests `a_retry_refusing_an_archived_journal_names_that_file` and the three journal-refusal tests assert `<op-id>/journal*.jsonl` in stderr |
| R1-F1.2 | Pending-operation lookup and the library's refusal classification report one exit class for an unreadable journal | fixed | `StoreError::exit_class` is the single classification used by `Refusal::exit_class` and the CLI's `store_error` (via shared `recovery::exit_error`); unit test `a_store_error_exits_alike_from_name_lookup_and_the_lifecycle` over all 12 variants; `assert_retry_refused_by_the_journal` now asserts exit 3. Decision recorded in `human_review_items` |
| R1-F1.3 | Sibling: other `StoreError` → exit-code conversions in `cli/src` | fixed | `refusal_error` now uses `exit_error`; grep of `StoreError`/`store_error`/`RemovalFailed(`/`exit_class` finds no other bypass; "released but still pending" kept as explicit exit 1 (internal invariant) |
| R1-F1.4 | Sibling: other recovery-store read errors that lose their file (`Report`, `Storage`, `Record`, `InventoryMismatch`) | clean | `Report` and `Storage` already carry `path`; `Record` is one `record.json` per named operation and is also built where no file was read; `InventoryMismatch` is not a file read |
| R1-F2 | Graph L1 tests exceed the five-second L1 time budget | fixed | R1-F2.1–R1-F2.3 verified |
| R1-F2.1 | Every graph L1 test reported slow by `just test` (`gathered_graph_lays_out_*`, `the_observed_graph_keeps_recent_commits_on_every_lane_at_200x60`, `run_pipeline_gathers_the_graph_on_a_narrow_image_terminal`, in both lib and bin targets) finishes within 5 s | fixed | Merman/fontdb/ttf-parser optimized in the test profile; `git_graph` and `list` unit tests run once (lib target, `lib_target_tests!`). Same lib test binary, ten graph tests in parallel at load ~24: 1.3–2.4 s each from `/tmp`, 9.2–10.3 s from the 212,344-entry `target/debug/deps` (CoreText lists the executable's directory per process; a host-artifact of this worktree's long-lived target, recorded in the `os` skill) |
| R1-F2.2 | Sibling: the three `journal_size` 100k-entry L1 tests finish within 5 s with margin | fixed | production: `entries::write` buffers (64 KiB) before gzip (was a compression step per serde token); test: block-wise `differences()`; alone 1.3–2.2 s (was 2.5–4.1 s at similar load); full run 2.8–4.1 s at load 40–87, not marked slow |
| R1-F2.3 | Sibling: the `worktree-cli::remove` tests nextest marked slow under load | clean | load-sensitive: `--test remove` alone 137 passed, none over 4 s; final run's other slow tests (`native_recreation_…` 1.5 s, `a_burst_is_delivered_…` 0.6 s, `--test remove_recovery` 16 tests in 1.5 s) all within budget alone |

### Progress

- starting the work on 'R1-F1 unreadable-journal diagnostics' at 15:01:31
        - defect class: a recovery-store read failure is reported without naming the file that failed, or is assigned an exit class by a path other than the library's single classification
        - orchestrator trace: `journal::parse` errors are wrapped in `StoreError::Journal { id, reason }` (`lib/src/remove/recovery/store.rs`, `replay_generations`) for both `journal.jsonl` and every `journal-<n>.jsonl`, so the file is lost; the CLI's `store_error` (`cli/src/commands/remove/mod.rs`) maps every `StoreError` from name resolution and `inspect` to `RemovalFailed` (exit 1), while `Refusal::exit_class` classifies `Journal`/`Record`/`Report` as refused (3) and `Storage`/`Io` as blocked (4)
        - subagent (rust-developer) report, verified by orchestrator diff review:
                - discovery: `replay_generations` used one parse closure for the current and archived journals, so `StoreError::Journal` could not say which file failed
                - discovery: name lookup sent every `StoreError` through `store_error` to `RemovalFailed` (exit 1); an existing CLI test asserted exit 1 on purpose, and `docs/cli/remove.md` documented exit 1
                - fixed R1-F1.1: `StoreError::Journal` now carries `path` (replacing `id`, which the path contains) and renders it; new lib test `an_unusable_journal_error_names_the_current_or_archived_file` and CLI test `a_retry_refusing_an_archived_journal_names_that_file`; existing journal-refusal CLI tests assert `<op-id>/journal.jsonl` in the message
                - fixed R1-F1.2: new `StoreError::exit_class` is the single classification; `Refusal::exit_class` delegates to it, and the CLI's `store_error` maps through a shared `recovery::exit_error` (also used by `refusal_error`); unit test `a_store_error_exits_alike_from_name_lookup_and_the_lifecycle` covers all 12 variants; `assert_retry_refused_by_the_journal` now asserts exit 3
                - decision: name-lookup storage/IO errors now exit 4 instead of 1 (the library's class); `WrongLock`/`PhaseOrder`/`Completed` stay 1; the "released but still pending" invariant failure stays an explicit exit 1
                - sibling sweep: `StoreError::Report` and `Storage` already name their path (clean); `Record` is per-operation and is also built where no file was read (clean); `InventoryMismatch` is not a file read (clean)
                - docs: `docs/cli/remove.md` exit-3 row and journal paragraph; `.claude/skills/worktree/lifecycle.md` bullet
                - scoped checks: lib filter 93 passed; CLI `binary(remove_recovery)` 16 passed; new tests passed; `just lint` passed
                - noticed: three `journal_size::*` 100k-entry tests ran 5.1–6.4 s; carried into R1-F2's sweep (same class: L1 test over budget)
- work completed for 'R1-F1 unreadable-journal diagnostics' at 15:08:04
- starting the work on 'R1-F2 graph L1 tests over budget' at 15:08:04
        - defect class: an L1 test in the worktree area takes longer than the 5 s L1 budget
        - full `just test` at 15:11 (host load average 40–60 from other sessions): 1,640 passed, 15 skipped, 49 slow
        - isolation runs separated load-sensitivity from real budget violations:
                - `cargo nextest run -p worktree-cli --test remove` alone: 137 passed in 8.4–13.1 s total, no test over 4 s, so the 26 slow `worktree-cli::remove` tests are **load-sensitive**, not over budget (clean)
                - `commands::git_graph::tests::gathered_graph_lays_out_*` (9), `the_observed_graph_keeps_recent_commits_on_every_lane_at_200x60`, and `commands::list::tests::run_pipeline_gathers_the_graph_on_a_narrow_image_terminal`: 10–12 s as a group of 11, 6.4 s for one alone; they run twice (lib and `wt` bin targets), giving the review's ~21–22 slow graph tests
                - `remove::recovery::store::tests::journal_size::*` (3): 3.8–5.3 s alone; same defect class, added as R1-F2.2
        - two foreground subagents ran in parallel (graph tests; `journal_size` tests); their items, grouped:
                - graph subagent (R1-F2.1):
                        - root cause, from temporary timers and macOS `sample` (all removed): the first `GitGraph::plan` per process builds the font chain, and fontique's CoreText scan lists the test binary's directory (`target/debug/deps`); unoptimized Merman layout repeated per measured candidate; every test ran twice (lib and `wt` bin targets)
                        - no redundant per-viewport work found in the code under test
                        - changed: root `Cargo.toml` test-profile `opt-level = 3` for `merman`, `merman-core`, `merman-render`, `fontdb`, `ttf-parser`; `lib_target_tests!` macro in `cli/src/lib.rs` (pass-through) and `cli/src/main.rs` (empty), wrapping `mod tests` in `commands/git_graph.rs` and `commands/list.rs`; test files unchanged byte-for-byte, so no assertion dropped
                        - skills: `os/macos.md` CoreText fact; `worktree/SKILL.md` macro; `worktree/git-graph.md` speed note and corrected stale test name
                        - incident: while removing its timers it restored `biscuit-visualized/src/src/mermaid/render.rs` from a backup, reverting two edits by the concurrent merman session, then replayed them verbatim; orchestrator confirmed the file's diff holds only that session's `author_text_colors`/`readable_text` change and no instrumentation (the one `Instant::now` predates HEAD)
                - `journal_size` subagent (R1-F2.2):
                        - root cause: `entries::write` fed each `serde_json` token straight into `GzEncoder` (a compression step per write); 5.4 s of a 6 s `op.complete` under load; also a byte-by-byte scan of 32 MB journal openings in the size test
                        - changed: `lib/src/remove/recovery/store/entries.rs` (64 KiB `BufWriter`, with a WHY comment); `lib/src/remove/recovery/store/tests/journal_size.rs` (`differences()` helper); fixture size and every assertion unchanged
                        - remaining floor ~2 s CPU per read-back test is unoptimized serde over 32 MB; within budget
        - orchestrator verification:
                - full `just test` after both: 1,560 passed, 14 skipped, 18 slow at load 34–87 (the count drop from 1,640 is the 80 graph/list unit tests no longer duplicated in the bin target)
                - isolated reruns: every non-graph slow test within budget alone (R1-F2.3); graph tests 1.3–2.4 s from a small directory vs 9.2–10.3 s from `deps`, same binary and load (R1-F2.1)
                - added the measurement method and remedy to `.claude/skills/os/macos.md`
                - drift fixed: the root `Cargo.toml` comment over `[profile.test.package.miniz_oxide]` quoted timings measured before buffering; reworded without the stale figures
- work completed for 'R1-F2 graph L1 tests over budget' at 15:58:10

### Completion Report

- **outcome: complete**; elapsed 2026-10-23T15:00:36-07:00 → 15:58 (about 58 minutes)
- parent findings: total 2 = fixed 2 + already fixed 0 + blocked 0 + unresolved 0 + disputed 0
        - R1-F1 fixed (children R1-F1.1, R1-F1.2, R1-F1.3 fixed; R1-F1.4 clean)
        - R1-F2 fixed (children R1-F2.1, R1-F2.2 fixed; R1-F2.3 clean)
- open findings: none
- decisions for the author (also in the review's `human_review_items`):
        - name-lookup store errors now take the library's exit class, so storage/IO problems exit 4 (were 1) and unusable records/journals/reports exit 3 (were 1)
        - `StoreError::Journal` carries `path` instead of `id` (the path contains the operation id)
        - `commands::git_graph` and `commands::list` unit tests run only in the lib target (`lib_target_tests!`)
        - test-profile `opt-level = 3` for Merman and font crates in the root `Cargo.toml` (may widen CI scope; check `just ci-local --plan`)
        - local graph-test overrun is a host artifact of a 212k-entry `target/debug/deps` (CoreText); sweeping that target removes it; not changed in code
- no cross-OS check run: the changes are error classification, a `BufWriter`, a cfg-free macro, and profile settings, none OS-specific
- changed files (this cycle):
        - R1-F1: `worktree/lib/src/remove/recovery/store.rs`, `worktree/lib/src/remove/recovery/store/tests.rs`, `worktree/lib/src/remove/lifecycle.rs`, `worktree/cli/src/commands/remove/mod.rs`, `worktree/cli/src/commands/remove/recovery.rs`, `worktree/cli/tests/remove_recovery.rs`, `worktree/docs/cli/remove.md`, `.claude/skills/worktree/lifecycle.md`
        - R1-F2: `Cargo.toml`, `worktree/cli/src/lib.rs`, `worktree/cli/src/main.rs`, `worktree/cli/src/commands/git_graph.rs`, `worktree/cli/src/commands/list.rs`, `worktree/lib/src/remove/recovery/store/entries.rs`, `worktree/lib/src/remove/recovery/store/tests/journal_size.rs`, `.claude/skills/os/macos.md`, `.claude/skills/worktree/SKILL.md`, `.claude/skills/worktree/git-graph.md`
- final verification:
        - `just test` (worktree): exit 0, 1,560 passed, 14 skipped — R1-F1, R1-F2
        - `just lint` (worktree): exit 0 — all
        - `cargo clippy -p worktree -p worktree-cli --all-targets --no-deps -- -D warnings`: clean — all
        - `cargo nextest run -p worktree-cli --test remove_recovery`: 16 passed — R1-F1
        - `cargo nextest run -p worktree --lib -E 'test(journal_size)'` (subagent, `-j1`): 1.3–2.2 s each — R1-F2.2
        - parallel run of the copied lib test binary from `/tmp`: ten graph tests 1.3–2.4 s each — R1-F2.1

## Implementation of Review Findings #2

> **started at:** 2026-10-23T16:02:34-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/feat-merman/worktree/fixes/2026-10-23-removal-store-size/review-2.md'
- this is iteration 2 of the review-to-implement cycle 
- the review's `implemented` flag was confirmed `false` before repairs started

### Finding Ledger

| ID | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| --- | --- | --- | --- |
| R1-F2 | Graph L1 tests exceed the five-second L1 time budget in the ordinary `just test` run | fixed | two consecutive ordinary `just test` runs: 1,560 passed, 0 slow, slowest test 3.27 s / 3.05 s; children R1-F2.1.1–R1-F2.1.11 fixed, R1-F2.2 and R1-F2.3 clean |
| R1-F2.1.1 | `gathered_graph_lays_out_continued_after_merge_at_merge` ≤5 s in ordinary run | fixed | ordinary `just test` after the repair: 2.493 s (run 3), every graph test 1.04–3.05 s (run 4); none marked slow; 7.8–15 s before the repair at the same `deps` size |
| R1-F2.1.2 | `gathered_graph_lays_out_continued_after_merge_behind` ≤5 s | fixed | ordinary `just test` after the repair: 1.741 s (run 3), every graph test 1.04–3.05 s (run 4); none marked slow; 7.8–15 s before the repair at the same `deps` size |
| R1-F2.1.3 | `gathered_graph_lays_out_continued_after_merge_diverged` ≤5 s | fixed | ordinary `just test` after the repair: 1.794 s (run 3), every graph test 1.04–3.05 s (run 4); none marked slow; 7.8–15 s before the repair at the same `deps` size |
| R1-F2.1.4 | `gathered_graph_lays_out_long_labels` ≤5 s | fixed | ordinary `just test` after the repair: 1.825 s (run 3), every graph test 1.04–3.05 s (run 4); none marked slow; 7.8–15 s before the repair at the same `deps` size |
| R1-F2.1.5 | `gathered_graph_lays_out_merged_twice` ≤5 s | fixed | ordinary `just test` after the repair: 1.404 s (run 3), every graph test 1.04–3.05 s (run 4); none marked slow; 7.8–15 s before the repair at the same `deps` size |
| R1-F2.1.6 | `gathered_graph_lays_out_new_branch_at_merged_tip` ≤5 s | fixed | ordinary `just test` after the repair: 1.456 s (run 3), every graph test 1.04–3.05 s (run 4); none marked slow; 7.8–15 s before the repair at the same `deps` size |
| R1-F2.1.7 | `gathered_graph_lays_out_observation_1` ≤5 s | fixed | ordinary `just test` after the repair: 1.672 s (run 3), every graph test 1.04–3.05 s (run 4); none marked slow; 7.8–15 s before the repair at the same `deps` size |
| R1-F2.1.8 | `gathered_graph_lays_out_observation_2` ≤5 s | fixed | ordinary `just test` after the repair: 2.036 s (run 3), every graph test 1.04–3.05 s (run 4); none marked slow; 7.8–15 s before the repair at the same `deps` size |
| R1-F2.1.9 | `gathered_graph_lays_out_sparse_lanes` ≤5 s | fixed | ordinary `just test` after the repair: 3.265 s (run 3), every graph test 1.04–3.05 s (run 4); none marked slow; 7.8–15 s before the repair at the same `deps` size |
| R1-F2.1.10 | `the_observed_graph_keeps_recent_commits_on_every_lane_at_200x60` ≤5 s | fixed | ordinary `just test` after the repair: 2.274 s (run 3), every graph test 1.04–3.05 s (run 4); none marked slow; 7.8–15 s before the repair at the same `deps` size |
| R1-F2.1.11 | `run_pipeline_gathers_the_graph_on_a_narrow_image_terminal` ≤5 s | fixed | ordinary `just test` after the repair: 1.061 s (run 3), every graph test 1.04–3.05 s (run 4); none marked slow; 7.8–15 s before the repair at the same `deps` size |
| R1-F2.2 | Three `journal_size` 100k-entry tests ≤5 s | clean | load-sensitive: 6.6–10.8 s in runs 1–2 at host load 30–46 (other sessions' rustc, Finder, DaisyDisk on 16 cores); alone 1.9–2.0 s at load 49; in the final ordinary runs 1.95 / 2.72 / 2.95 s and 2.40 / 2.53 s (top two), none slow; no code change |
| R1-F2.4 | Sibling sweep: other worktree L1 tests that measure text with the host's fonts from a test binary in `deps` | clean | only `GitGraph::plan` (and `run_pipeline` with image support) measures fonts; grep of `.plan(`/`gitgraph_geometry` in `lib/src/graph/tests.rs`, `cli/tests/` finds none in L1; CLI integration tests spawn `target/debug/wt` (21-entry directory); Kitty and perf graph tests are L2/perf tiers; `cargo nextest show-config test-groups` (while the group existed) matched exactly the 11 tests |
| R1-F2.3 | Removal tests previously slow under load ≤5 s | clean | none marked slow in any of the four ordinary runs this cycle; slowest non-graph, non-journal test 2.70 s (`list_prs`) |

### Progress

- starting the work on 'R1-F2 graph L1 tests over budget' at 16:02:34
        - defect class: an L1 test that measures text with the host's fonts pays a per-process OS font scan whose cost depends on the size of the directory its test executable sits in, not on the code under test, so concurrent test processes exceed the 5 s budget on a long-lived macOS target
        - orchestrator did this finding directly (one finding, no subagents)
        - discovered: `target/debug/deps` held 221,677 entries, 206,034 of them `*.rcgu.o` (macOS `split-debuginfo = "unpacked"` leaves each incremental rebuild's object files beside the old ones); no file in `deps` was older than one day, so `just sweep`'s 14-day pass (the remedy the `os` skill named) could not remove any of them
        - review's smallest repair tried first: listed every object each of the 468 executables references (`nm -ap … | grep ' OSO '`, 53,499 referenced); deleted the 113,359 unreferenced objects older than 60 minutes (a still-linking build's objects kept); `deps` fell to ~108,800 entries and refilled to 111,080 within minutes from concurrent sessions
        - ordinary `just test` after cleanup (run 1, load ~40): 1,560 passed, 13 slow; the 11 graph tests 6.6–15.0 s, so cleanup alone does not repair the finding
        - one graph test alone: 1.4–2.0 s, and 0.67–0.71 s with `CFProcessPath=/usr/bin/true` (scan lists `/usr/bin` instead of `deps`); the 9–15 s figures are the serialized listings of ~11 concurrent processes
        - tried a macOS-only `max-threads = 1` test group (as `fsevents-l1`): run 2 left 2 graph tests over budget (9.4 s, 5.2 s, the first to run while the suite was busiest); group removed
        - fixed: `.config/nextest.toml` setup script `coretext-small-bundle` (macOS hosts only) writes `CFProcessPath=/usr/bin/true` to `$NEXTEST_ENV` for exactly the 11 font-measuring `worktree-cli` tests; the 11 then ran together in 1.0–2.5 s at load 46; no production code or test assertion changed; string `command` form matches the existing setup script so older nextest builds on the rigs parse it
        - decision (recorded in the review's `human_review_items`): per-test `CFProcessPath` instead of (a) repeated target cleanup, which halves `deps` but does not bring the tests under budget and regrows in hours, (b) serializing the tests, which still overran, or (c) changing `biscuit-visualized`'s font discovery, which is production code in another package and would not change production cost (an installed `wt` lists its own small directory)
        - docs: `.claude/skills/os/macos.md` CoreText fact corrected (drift: it said sweeping the target removes the overrun; the objects are hours old and cleanup is insufficient) and now names the setup script; `.claude/skills/worktree/git-graph.md` speed note says a new `GitGraph::plan` test must join the script's filter
        - R1-F2.2 `journal_size`: 6.6–10.8 s in runs 1–2 at host load 30–46 from other processes; alone 1.9–2.0 s; final ordinary runs ≤2.95 s; load-sensitive, no change
- work completed for 'R1-F2 graph L1 tests over budget' at 16:21:28

### Completion Report

- **outcome: complete**; elapsed 2026-10-23T16:02:34-07:00 → 16:22 (about 20 minutes)
- parent findings: total 1 = fixed 1 + already fixed 0 + blocked 0 + unresolved 0 + disputed 0
        - R1-F2 fixed: R1-F2.1.1–R1-F2.1.11 fixed; R1-F2.2, R1-F2.3, R1-F2.4 clean
- open findings: none
- changed files (this cycle), all for R1-F2:
        - `.config/nextest.toml` (setup script `coretext-small-bundle` and its macOS-only profile entry)
        - `.claude/skills/os/macos.md`
        - `.claude/skills/worktree/git-graph.md`
        - not a repository change: 113,359 unreferenced `*.rcgu.o` files deleted from this worktree's `target/debug/deps`
- final verification:
        - `just test` (worktree), run 3: exit 0, 1,560 passed, 14 skipped, **0 slow**, slowest 3.265 s, 25.7 s total — R1-F2
        - `just test` (worktree), run 4: exit 0, 1,560 passed, 14 skipped, **0 slow**, slowest 3.053 s, 25.3 s total — R1-F2
        - `cargo nextest run -p worktree-cli --lib -E 'test(/gathered_graph_lays_out_|…/)'` after the final `command` form: 11 passed, setup script ran — R1-F2.1
        - `cargo nextest run -p worktree --lib -E 'test(journal_size)'`: 3 passed, 1.9–2.0 s — R1-F2.2
        - `just lint` (worktree): exit 0
        - `just check-tier-coverage worktree`: exit 0, 0 stranded
        - test-toolkit config contracts (`nextest_config_verification`, `ci_workflow_contracts`, `junit_staging_contracts`): 187 passed, 1 failed, **unrelated**: `declared_test_tiers_match_owned_tests` fails because `biscuit-visualized` now owns 2 `browser_` tests without declaring that tier (the concurrent merman work in that package; not touched here)
- no cross-OS check run: the only behavior change is a nextest setup script scheduled on macOS hosts only; Linux, Windows, and WSL2 test processes are unchanged
- no performance measurement deferred

## Implementation of Review Findings #3

> **started at:** 2026-10-23T16:25:19-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/feat-merman/worktree/fixes/2026-10-23-removal-store-size/review-3.md'
- this is iteration 3 of the review-to-implement cycle 
- the review's `implemented` flag was already `false`; left `false` until the cycle closes

### Finding Ledger

| ID | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| --- | --- | --- | --- |
| R1-F2 | graph L1 tests exceed the five-second L1 time budget | fixed | every child row below fixed or clean; two ordinary `just test` runs: 0 slow, slowest graph test 2.406 s |
| R1-F2.1.9 | `gathered_graph_lays_out_sparse_lanes` ≤5 s with margin in the ordinary `just test` run, every assertion kept | fixed | split into `gathered_graph_lays_out_sparse_lanes_at_120` / `_at_56` plus guard `sparse_lanes_has_a_test_per_width`; ordinary runs 2.015/2.406 s and 1.826/2.068 s (was 5.403/3.672 s combined); every assertion kept, cross-viewport checks now compare with the whole graph (equal or stronger, see progress) |
| R1-F2.1.1–R1-F2.1.8, R1-F2.1.10, R1-F2.1.11 | other graph/list cases stay ≤5 s | clean | same `assert_laid_out` call at both widths (`&Width::ALL`); focused run 1.2–1.8 s; both ordinary runs 0 slow, none above 2.41 s; split not needed (alone 1.2–1.8 s, ≥2.5× margin) |
| R1-F2.2 | three `journal_size` tests ≤5 s | clean | ordinary runs 1.47/1.98/2.15 s and ≤2.28 s |
| R1-F2.3 | removal tests ≤5 s in the ordinary run | clean | both ordinary runs report 0 slow tests out of 1,562 |
| R1-F2.4 | no other font-measuring L1 site | clean | `git grep` of `assert_laid_out`/`gathered_graph_lays_out`: 8 callers, all in `cli/src/commands/git_graph/tests.rs`; the setup-script filter regex `^commands::git_graph::tests::gathered_graph_lays_out_` covers both new names (setup ran in the focused run); `just check-tier-coverage worktree`: 0 stranded |

### Progress

- starting the work on 'R1-F2 graph L1 tests over budget' at 16:25:19
        - defect class: an L1 layout test bundles several independently checkable viewports into one process, so its runtime is the sum of every viewport's measured plan and crosses the budget under suite contention
        - orchestrator did this finding directly (one finding, one test file; no subagents)
        - measured the sparse-lanes test's stages alone (temporary timing prints, removed): fixture 0.08–0.13 s, gather 0.03 s, whole-graph plan 0.35–0.43 s, 120-column plan 0.67 s, 56-column plan 0.77–0.83 s, floor plan 0.13 s, geometry 0.01 s each, explicit width 0.01 s
        - discovered: each `GitGraph::plan` makes many `MermaidDiagram::natural_size` calls (41 distinct per test run, ~45 ms each, each a full SVG render); the `$TMPDIR` render cache never helps across runs because the fixture's SHAs (and so the Mermaid text) differ per run; production font discovery is not the cost, so no production change
        - fixed: `assert_laid_out` takes `widths: &[Width]` (`Width::Ordinary` = 120, `Width::Narrow` = 56; `Evidence::hidden_lanes` is sized by `Width::ALL`); the sparse-lanes fixture runs as `gathered_graph_lays_out_sparse_lanes_at_120` and `_at_56`; every other fixture still checks both widths in one test
        - decision: the two cross-viewport checks now compare each viewport with the whole graph instead of the 120-column plan, so a width can run alone
                - tag texts: each viewport's sorted tag texts must equal the whole graph's (`graph.mermaid()` geometry, 10 ms); stronger than 56-vs-120 equality, and holds for the sparse fixture's 120-column plan with a hidden lane
                - trimmed-or-shrunk: applies when the whole graph is wider than the viewport and that viewport hides no lane; it now also fires at 120 columns (`continued-*`, `observation-2`, `sparse-lanes` trim there); the one case it no longer covers, a narrow viewport that hides lanes, exists in no fixture
                - alternative rejected: recomputing the 120-column plan inside the 56-column test keeps the old comparison but costs the 0.67 s the split removes
        - guard: `sparse_lanes_has_a_test_per_width` maps every `Width` through an exhaustive `match` to its sparse-lanes test, so a new width does not compile until it names a test
        - docs: `.claude/skills/worktree/git-graph.md` "L1 layout proof" describes the per-width split, the guard, and the whole-graph comparisons
- work completed for 'R1-F2 graph L1 tests over budget' at 16:34:10

### Completion Report

- **outcome: complete**; elapsed 2026-10-23T16:25:19-07:00 → 16:35 (about 10 minutes)
- parent findings: total 1 = fixed 1 + already fixed 0 + blocked 0 + unresolved 0 + disputed 0
        - R1-F2 fixed: R1-F2.1.9 fixed; R1-F2.1.1–R1-F2.1.8, R1-F2.1.10, R1-F2.1.11, R1-F2.2, R1-F2.3, R1-F2.4 clean
- open findings: none
- changed files (this cycle), all for R1-F2:
        - `worktree/cli/src/commands/git_graph/tests.rs`
        - `.claude/skills/worktree/git-graph.md`
- final verification:
        - `cargo nextest run -p worktree-cli --lib -E 'test(/^commands::git_graph::tests::(gathered_graph_lays_out_|sparse_lanes_has)/)'`: 11 passed, setup script ran, slowest 2.046 s — R1-F2.1
        - `just test` (worktree), run 1: exit 0, 1,562 passed, 14 skipped, **0 slow**; sparse-lanes 2.015 s / 2.406 s; `journal_size` 1.468/1.984/2.150 s — R1-F2.1.9, R1-F2.2, R1-F2.3
        - `just test` (worktree), run 2: exit 0, 1,562 passed, 14 skipped, **0 slow**; slowest test 2.278 s; sparse-lanes `_at_56` 2.068 s — R1-F2
        - `just check-tier-coverage worktree`: 0 stranded — R1-F2.4
        - `just lint` (worktree): **exit 1, unrelated**: clippy stops in `biscuit-visualized/src/src/raster/png.rs` (`clippy::chunks_exact_to_as_chunks`) before reaching worktree; that file is uncommitted merman work being edited concurrently in this checkout (modified 16:29:49 during this run, the error line moved 100 → 101 between two runs); not touched here, to avoid clobbering that session
        - `cargo clippy -p worktree --all-targets -- -D warnings -A clippy::chunks_exact_to_as_chunks` and the same for `worktree-cli`: exit 0, no warnings — the worktree code this cycle changed is lint-clean
- no cross-OS check run: the change is a portable test split with no `cfg`, path, or process code
- no performance measurement deferred
