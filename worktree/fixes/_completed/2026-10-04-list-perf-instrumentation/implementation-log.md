---
spec: "/Volumes/coding/wt/rusty-biscuit/fix-wt-skill/worktree/fixes/2026-10-04-list-perf-instrumentation/spec.md"
plan: "worktree/fixes/2026-10-04-list-perf-instrumentation/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
source_files_during_phase_1:
  - worktree/lib/src/lib.rs
  - worktree/lib/src/timing.rs
  - worktree/lib/src/timing/tests.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1:
  - .claude/skills/worktree/list.md
source_files_during_phase_2:
  - worktree/lib/src/git.rs
  - worktree/lib/src/git/calls/tests.rs
  - worktree/lib/src/fast_forward.rs
  - worktree/lib/src/live_remote.rs
  - worktree/lib/src/listing.rs
  - worktree/lib/src/worktree.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
  - .claude/skills/worktree/SKILL.md
  - .claude/skills/worktree/testing.md
source_files_during_phase_4:
  - worktree/lib/src/timing.rs
  - worktree/lib/src/graph.rs
  - worktree/lib/src/list.rs
  - worktree/lib/src/list/wait.rs
  - worktree/lib/src/list/tests/pipeline.rs
  - worktree/lib/src/list/tests/pipeline/timings.rs
  - worktree/cli/src/args.rs
  - worktree/cli/src/lib.rs
  - worktree/cli/src/perf.rs
  - worktree/cli/src/perf/tests.rs
  - worktree/cli/src/commands/list.rs
  - worktree/cli/src/commands/list/tests.rs
  - worktree/cli/src/commands/list/tests/pipeline.rs
  - worktree/cli/tests/perf_support/mod.rs
  - worktree/cli/tests/perf_flag.rs
  - worktree/cli/tests/perf_pr_request.rs
  - worktree/cli/tests/perf_graph_stages.rs
  - worktree/cli/tests/list_prs.rs
  - worktree/cli/tests/cache_warm_path.rs
  - worktree/cli/tests/cache_cold_path.rs
  - worktree/cli/tests/snapshots/list_flags__global_flag_completions.snap
docs_updated_during_phase_4:
  - worktree/docs/cli/list.md
  - worktree/README.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
  - .claude/skills/worktree/list.md
  - .claude/skills/worktree/testing.md
  - .claude/skills/worktree/git-graph.md
source_files_during_phase_5:
  - worktree/lib/src/timing.rs
  - worktree/lib/src/strict_json.rs
  - worktree/lib/src/remote_head.rs
  - worktree/lib/src/list.rs
  - worktree/lib/src/list/wait.rs
  - worktree/lib/src/list/wait/tests.rs
  - worktree/lib/src/list/wait/tests/reports.rs
  - worktree/lib/src/list/tests/pipeline.rs
  - worktree/lib/src/list/tests/pipeline/timings.rs
  - worktree/cli/src/args.rs
  - worktree/cli/src/main.rs
  - worktree/cli/src/commands/refresh_worker.rs
  - worktree/cli/src/commands/list.rs
  - worktree/cli/src/commands/list/tests.rs
  - worktree/cli/tests/perf_flag.rs
docs_updated_during_phase_5:
  - worktree/docs/cli/list.md
  - worktree/README.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
  - .claude/skills/worktree/list-remote.md
  - .claude/skills/os/build-hosts.md
source_files_during_phase_6:
  - worktree/cli/tests/perf_support/mod.rs
  - worktree/cli/tests/perf_flag.rs
  - worktree/cli/tests/list_prs.rs
  - worktree/cli/src/perf/tests.rs
  - worktree/lib/src/list/tests/pipeline/timings.rs
docs_updated_during_phase_6:
  - worktree/docs/performance-testing.md
docs_created_during_phase_6: []
skills_files_updated_during_phase_6:
  - .claude/skills/worktree/testing.md
source_files_during_phase_7:
  - worktree/cli/tests/perf_support/graph.rs
docs_updated_during_phase_7:
  - worktree/docs/performance-testing.md
  - worktree/docs/cli/list.md
  - worktree/docs/git-graph.md
  - worktree/README.md
  - docs/dependencies.md
docs_created_during_phase_7: []
skills_files_updated_during_phase_7:
  - .claude/skills/worktree/git-graph.md
  - .claude/skills/worktree/list.md
source_code:
  - worktree/lib/src/lib.rs
  - worktree/lib/src/timing.rs
  - worktree/lib/src/timing/tests.rs
  - worktree/lib/src/git.rs
  - worktree/lib/src/git/calls/tests.rs
  - worktree/lib/src/fast_forward.rs
  - worktree/lib/src/live_remote.rs
  - worktree/lib/src/listing.rs
  - worktree/lib/src/worktree.rs
  - worktree/lib/src/graph.rs
  - worktree/lib/src/graph/topology.rs
  - worktree/lib/src/graph/tests.rs
  - worktree/lib/src/list.rs
  - worktree/lib/src/list/tests.rs
  - worktree/lib/src/list/tests/pipeline.rs
  - worktree/lib/src/list/tests/pipeline/timings.rs
  - worktree/lib/src/list/wait.rs
  - worktree/lib/src/list/wait/tests.rs
  - worktree/lib/src/list/wait/tests/reports.rs
  - worktree/lib/src/remote_head.rs
  - worktree/lib/src/strict_json.rs
  - worktree/lib/benches/list_status.rs
  - worktree/cli/src/args.rs
  - worktree/cli/src/lib.rs
  - worktree/cli/src/main.rs
  - worktree/cli/src/perf.rs
  - worktree/cli/src/perf/tests.rs
  - worktree/cli/src/commands/git_graph.rs
  - worktree/cli/src/commands/git_graph/tests.rs
  - worktree/cli/src/commands/list.rs
  - worktree/cli/src/commands/list/progress.rs
  - worktree/cli/src/commands/list/progress/tests.rs
  - worktree/cli/src/commands/list/tests.rs
  - worktree/cli/src/commands/list_table.rs
  - worktree/cli/src/commands/refresh_worker.rs
  - worktree/cli/tests/perf_support/mod.rs
  - worktree/cli/tests/perf_support/graph.rs
  - worktree/cli/tests/perf_flag.rs
  - worktree/cli/tests/perf_pr_request.rs
  - worktree/cli/tests/perf_graph_stages.rs
  - worktree/cli/tests/list_prs.rs
  - worktree/cli/tests/cache_warm_path.rs
  - worktree/cli/tests/cache_cold_path.rs
  - worktree/cli/tests/snapshots/list_flags__global_flag_completions.snap
documentation:
  - worktree/docs/performance-testing.md
  - worktree/docs/cli/list.md
  - worktree/docs/git-graph.md
  - worktree/README.md
  - docs/dependencies.md
completed_phase: 7
implemented: true
packages:
  - worktree
  - worktree-cli
---

# Implementation Log for 2026-10-04-list-perf-instrumentation (7 phases)

## Phase 1

### Rulings (R1–R9)

All rulings were taken at the plan's defaults; nothing needed the author.
Where a default left a gap, the ruling below fills it:

- **R1** The `Stage` enum is closed at the spec §3 ids plus `local_reads`,
  `verbose_history`, and the seven worker ids (41 variants). Ids and labels
  live in one table in `lib/src/timing.rs`, so adding a stage edits one row.
- **R2** Child kinds are a property of each recorded span, not of `Stage`;
  the pipeline (Phase 4) chooses them as the plan states. The root is always
  sequential.
- **R3** A span with concurrent children serializes zero remainders, and the
  decoder rejects a nonzero one. **Gap filled:** a span with no children is
  not a parent, so it also carries zero remainders (otherwise every leaf
  would claim its whole elapsed time as `unattributed`). Leaves serialize
  `children_kind: "sequential"` with `children: []`.
- **R4** `launch_index` is 0-based and the decoder requires owned launches to
  be listed as `0..n` in order.
- **R5** Unknown stage ids are rejected, unknown fields are ignored, and a
  repeated JSON key anywhere is rejected through `strict_json::members`.
- **R6** The matrix test is `timing::tests::the_decoder_matrix_rejects_every_malformed_cell`.
- **R7** No cross-OS measurement of disabled-mode overhead.
- **R8** The module has no OS-specific code; nothing to cross-check here.
- **R9** Applies in Phase 4; `local_reads` and `remote_and_local` are both
  in the enum.

### Spike S1 — scoped git-call counter

Throwaway `rustc` program (not kept): a thread-local stack of `u64` counters,
incremented only after `Command::output()` returns `Ok` (so a nonzero exit
counts and a failed spawn does not). Closing a scope adds its count to the
enclosing scope, so nested scopes each include their descendants and a parent
counts each call once. Spawned tasks open their own scope only when the
spawner had one active (the "handle" is the spawner's `is_active()` bool
captured before spawning), return their count, and the spawner adds the sum
after joining. An unrelated thread running three `git` calls during the outer
scope added nothing. Result: `outer = 1 + 2 + 4` as expected.

**Decision:** thread-local counter stack plus tasks that return their scoped
counts, as the spec describes. No `Arc<AtomicU64>`: it needs no atomics, the
disabled path is one thread-local `is_empty()` read, and two concurrent
listings on different threads cannot mix counts. Cost: every thread-spawn
site inside a counted region must re-enter the scope, or `git_calls` must be
reported as unknown for that span. Spawn sites Phase 2–4 must handle:
`lib/src/listing.rs:207`, `lib/src/worktree.rs:319,360,408,452`,
`cli/src/commands/list.rs:511,577`, `cli/src/commands/git_graph.rs:1071`,
`cli/src/commands/git_graph/topology.rs:439` (the last four move to the
library in Phase 3). `git.rs:214` spawns only a stdin writer, no Git.

**Recorder seam today:** `recorder::record(args)` is called at the top of
each of four helpers (`git_command_status`, `git_command_in`,
`git_from_output`, `git_from_bytes_status`; injected failure only in
`git_command_status`) under `cfg(any(test, feature = "count-git"))`, into a
global `Mutex`, *before* the spawn — so the recorder counts injected
failures and failed spawns, which the new counter must not.
`git_rev_parse` (used by `repo_info`) spawns Git without the recorder; the
new counter must still count it. Keep the two separate.

### The `worktree::timing` module

New `worktree/lib/src/timing.rs` (exported as `pub mod timing`) with unit
tests in `worktree/lib/src/timing/tests.rs`. No existing code calls it yet;
the CLI's `perf.rs` is untouched until Phase 4.

- **`Stage`** — 41 variants generated from one `stages!` table (id, label,
  doc). `id()`, `label()`, `from_id()`, `FromStr` (error `UnknownStage`),
  `Display` (the id), `Serialize`/`Deserialize` through the id, `Stage::ALL`.
- **`Span`** — stage, `Duration` elapsed, `Option<u64>` git_calls (`None` =
  unknown), `ChildrenKind`, children. `find(path)`, `unattributed()`,
  `over_attributed()`.
- **`SpanList`** (the builder) — `sequential()`/`concurrent()`, `push` (a
  repeated stage accumulates: elapsed adds, a count stays known only when
  both are, children merge), `time(stage, f)`, `time_parent(stage, kind, f)`.
  Concurrent tasks push after joining.
- **`Timings`** — `new(scope, total, SpanList)`, `with_worker_reports`,
  `span(path)`, root remainders, `to_json()`, `from_json()`; also
  `Serialize`. **`WorkerTimings`** is the same tree without scope or reports
  (the receipt's `durations` object), with its own `to_json`/`from_json`.
- **Worker observations** — `WorkerReport { launch_index, attempt_id,
  report: LaunchReport }` where `LaunchReport` is `Complete(t) | Partial(t) |
  Missing | Invalid`, so a status/report mismatch cannot be built (the wire
  format still carries `status` plus optional `report`, and the decoder
  rejects mismatches). `summarize_worker_reports(reports, adopted,
  origin_changed)` implements the spec §3 summary rules.
- **Decoder** — strict parse through `strict_json::members(..).into_value()`
  (repeated key at any depth or trailing content → `Malformed`); version
  checked from the raw value before the shape, at the top level and for each
  nested worker report (`UnsupportedVersion`); stage ids parsed by hand
  (`UnknownStage`); `DuplicateSibling(path)`; `InconsistentReconciliation(path)`
  at the root, sequential parents, concurrent parents, and leaves;
  `InvalidWorkerReports` for out-of-order launches, a status/report mismatch,
  a per-launch `adopted`/`origin_changed`, a non-attempt-id `attempt_id`, or a
  summary `summarize_worker_reports` could not produce. Unknown fields are
  ignored.

**Departure from the plan text (none from the spec):** the plan describes a
`WorkerReport { .., status, report: Option<..> }`; the code uses the
`LaunchReport` enum instead so invalid combinations are unrepresentable. The
serialized shape is exactly the spec's (`status` + optional `report`).

**Additional ruling:** the document carries `worker_report_status` only when
a worker was followed (absent otherwise; `null` rejected). Without it,
`worker_reports` must be empty.

### Decoder matrix (R6)

`timing::tests::the_decoder_matrix_rejects_every_malformed_cell` walks every
cell from a real `Timings::to_json()` fixture (remote listing with a forced
retry), one edit per cell, plus a control row. `R` = rejected, `A` = accepted
with the stated observable result.

| Shape | `format_version` | `scope` | `total_us` | root remainders | `spans` | `worker_reports` | `worker_report_status` | `stage` | `elapsed_us` | `git_calls` | `children_kind` | `children` | span remainders | launch `report` |
| ----- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| absent | R | R | R | R | R | R | R (with launches); A with no launches and `[]` | R | R | **A → `None`** | R | R | R | R under `complete`; A under `missing` |
| null | R | R | R | R | R | R | R | R | R | R | R | R | R | R |
| wrong type, whole | R | R | R | R | R | R | R | R | R | R | R | R | R | R |
| wrong type, one element | n/a | n/a | n/a | n/a | R | R | n/a | n/a | n/a | n/a | n/a | R | n/a | n/a |
| wrong type, every element | n/a | n/a | n/a | n/a | R | R | n/a | n/a | n/a | n/a | n/a | R | n/a | n/a |
| empty | n/a | R (`""`) | n/a | n/a | A (`[]` with `unattributed_us = total_us`) | R under a summary; A with no summary | R (`""`) | R (`""`) | n/a | n/a | R (`""`) | A (`[]` on a concurrent parent) | n/a | R (`{}`) |
| negative / fractional | R | n/a | R | R | n/a | n/a | n/a | n/a | R | R | n/a | n/a | R | n/a |
| unknown value | R (`2` → `UnsupportedVersion`) | R | n/a | n/a | n/a | n/a | R | R (label, `"parallel"`) | n/a | `0` → A `Some(0)` | R | n/a | n/a | n/a |
| repeated key | R | R | R | R | R | R | R | R | R | R | R | R | R | R |
| trailing content | R (document) | | | | | | | | | | | | | |

Control row: the unedited fixture decodes equal to the in-memory fixture with
`git_calls == Some(12)`. An unknown field at the root and inside a span is
ignored (decodes equal to the fixture).

**Mutation check:** replacing the strict parse with plain `serde_json` made
15 repeated-key cells fail, so those cells are load-bearing. Disabling the
explicit `null` check in `present` left the matrix green because every
`present` field's inner type already rejects `null`; the check is kept as
defense for any future `present` field whose type accepts `null`.

**Smell grep:** `#[serde(default)]` appears only on `git_calls`,
`worker_report_status`, and a launch's `report`, each paired with
`deserialize_with = "present"` (absent → `None`, `null` → error) and each
covered by an absent and a null cell. No `unwrap_or_default`, `.ok()`, or
`filter_map` in the decode path.

### Requirement → test mapping

| Requirement (spec "Tests" §1 / plan) | Test (`worktree` lib, `timing::tests::…`) |
| --- | --- |
| ids round-trip, unique; labels unique; closed list | `every_stage_id_round_trips_and_ids_and_labels_are_unique` |
| exact reconciliation | `sequential_spans_and_unattributed_reconcile_exactly_to_the_total` |
| over-attribution surfaced at each sequential level | `over_attribution_is_surfaced_at_every_sequential_level` |
| concurrent children excluded from sums | `concurrent_children_never_enter_a_sum` |
| sequential children with their own remainder | `a_sequential_parent_reconciles_its_own_remainder_and_a_leaf_has_none` |
| path lookup with a stage under two parents | `a_stage_under_two_parents_is_found_by_its_path` |
| repeated calls accumulate into one step | `a_repeated_stage_accumulates_into_one_sibling`, `timing_a_parent_records_its_children_inside_it` |
| JSON round trip | `a_document_round_trips_through_json` |
| integer rounding | `durations_become_whole_microseconds_before_remainders_are_computed` |
| version mismatch | `an_unsupported_version_is_rejected_at_either_level` |
| unknown stage id | `an_unknown_stage_id_is_rejected` |
| duplicate siblings | `a_repeated_sibling_stage_is_rejected` |
| invalid reconciliation data | `inconsistent_reconciliation_is_rejected_where_it_occurs` |
| worker summary rules | `worker_report_summary_follows_the_launches`, `worker_reports_must_be_ordered_launches_matching_their_summary` |
| Input Robustness Matrix | `the_decoder_matrix_rejects_every_malformed_cell` |

All 17 are L1 unit tests inside the `worktree` lib target (no tier marker in
any path segment), so `just test` runs them.

### Gates

- `just lint` (worktree): clean.
- `just test` (worktree): first run 964/965 passed with one **TIMEOUT**
  (30 s) in
  `worktree-cli::list_remote_head a_fetch_still_running_at_the_deadline_is_still_pulling_and_publishes_after_the_listing`,
  a CLI integration test this phase does not touch (the CLI code is
  unchanged). Alone it passed in 3.5 s; a full rerun passed 1106/1106 (32
  skipped). Treated as a load flake from the first run's cold build.
- Not run: `just test-l2`, `just test-perf` (no CLI or pipeline change in
  this phase), cross-OS checks (no OS-specific code; R8).
- `just check-tier-coverage` not needed: no new tier-marked tests.

## Phase 2

### What was built

`worktree::git::calls` (new public module inside `lib/src/git.rs`, tests in
`lib/src/git/calls/tests.rs`), a per-thread scoped count of **started** Git
processes, separate from `recorder` and failure injection. This follows the
Phase 1 spike's decision (thread-local stack, no `Arc`/atomics).

- `CallScope::enter()` pushes a counter on this thread's stack; `finish()`
  (or drop) pops it, adds its total to the enclosing scope, and returns it,
  so nested scopes include their descendants and the parent counts each call
  once. `calls()` reads the running count. A scope leaked with `mem::forget`
  is folded into the next scope that closes, so it cannot keep absorbing
  calls. `CallScope` is `!Send`.
- `TaskHandle::current()` (taken on the spawner) → `handle.run(task)` on the
  spawned thread → `(value, count)` → `calls::joined(..)` on the spawner adds
  the count after joining. With no scope open on the spawner, `run` opens
  nothing and returns 0. If `run` is used on a thread that already has a
  scope open (no spawn happened), it returns 0 so the call is not counted
  twice.
- `is_counting()`, `add_joined(n)` are public; `started()` and `output(cmd)`
  are `pub(crate)` hooks. With no scope open, `started()` is one thread-local
  access (no allocation, no atomics).

### Where Git is counted

Counted once, after the process started (a failed spawn counts nothing; an
injected failure returns before spawning; a nonzero exit counts):

| Spawn site | Hook |
| --- | --- |
| `git_rev_parse` (via `repo_info`; not recorded) | `calls::output` |
| `git_command_status` (`git_command`, `git_command_allow_no_match`) | `calls::output` |
| `git_command_in` | `calls::output` |
| `git_from_output` | `calls::output` |
| `git_from_bytes_status` (`git_from`, `git_from_raw`, `git_from_bytes`, `git_from_bytes_with_env`, `git_from_bytes_allow_no_match`) | `calls::started()` after `spawn()` |
| `fast_forward::merge_ff_only` | `calls::output` |
| `live_remote::run_transport` | `calls::started()` after `spawn()` |

`calls::output` is `spawn()` + `wait_with_output()` with stdin null and
stdout/stderr piped, which is what `Command::output()` did at every one of
these sites, so behavior is unchanged.

**Departure from the plan text:** the plan scoped Phase 2 to one file
(`lib/src/git.rs`). Two other library helpers start Git outside `git.rs`
(`fast_forward::merge_ff_only`, `live_remote::run_transport`), so the spec's
"count at the lowest spawning helper" needed a one-line hook in each. Without
them, `--ff` would undercount. I also passed `TaskHandle` through the four
library thread-spawn sites the spike listed (`listing::compare_live`;
`worktree::gather_dirtiness`, `compare_tree`, `WorktreeList::gather_local`).
Without that, a scope around the local gather would report a count that looks
complete but is wrong. In `compare_live` the two joins were reordered (the
`merge-tree` result is joined before the `rev-list` result's `?`), so an early
return cannot drop the `merge-tree` count. The result is unchanged. The CLI
spawn sites (`cli/src/commands/list.rs:511,577`, `git_graph.rs:1071`,
`git_graph/topology.rs:439`) are still unpropagated. They move into the
library in Phase 3 and must use `TaskHandle` there.

### Requirement → test mapping

| Requirement (spec "Tests" / plan) | Test |
| --- | --- |
| nested scopes include descendants; parent counts each call once | `git::calls::tests::nested_scopes_each_include_their_descendants`, `a_dropped_scope_adds_its_count_to_the_enclosing_one` |
| unrelated thread excluded; concurrent scopes do not mix | `git::calls::tests::a_call_on_an_unrelated_thread_is_not_counted` |
| scoped threaded aggregation | `git::calls::tests::spawned_tasks_return_counts_that_add_to_the_spawner_after_joining`, `a_task_handle_run_without_spawning_counts_once`, `the_library_spawn_sites_return_their_tasks_counts`, `listing::repo_tests::a_counting_scope_sees_every_call_of_a_threaded_local_gather` |
| failed spawn / injected failure excluded | `git::calls::tests::a_failed_spawn_or_an_injected_failure_counts_nothing` (missing working dir for 5 helpers incl. the transport; `fail_matching` for `git_command`) |
| nonzero exit counted | `git::calls::tests::a_process_that_exits_nonzero_is_counted`, `the_transport_counts_its_own_process_and_its_config_read` (failing row) |
| byte helpers / wrappers counted once | `git::calls::tests::every_helper_counts_one_start_however_it_is_wrapped` (10 helpers, each exactly 1) |
| `--ff` and transport spawn sites counted | `a_fast_forward_counts_every_git_process_it_starts` (count == recorder, includes `merge`), `the_transport_counts_its_own_process_and_its_config_read` (count == recorder + 1) |
| disabled collection | `git::calls::tests::with_no_scope_open_nothing_is_counted` |
| recorder unchanged | `git::calls::tests::the_recorder_still_logs_every_requested_call_while_counting` (recorder still logs the injected failure and the failed spawn; counter = 1), plus all existing recorder-backed tests unchanged and green |

All 14 are L1 tests in the `worktree` lib target (no tier marker in any path
segment). Tests touching the global recorder or `fail_matching` are
`#[serial_test::serial]`.

**Mutation checks:** I replaced `counted.run(..)` with an uncounted task in
`gather_dirtiness`, and `the_library_spawn_sites_return_their_tasks_counts`
failed (0 ≠ 3). I did the same to the `merge-tree` task in `compare_live`,
and `a_counting_scope_sees_every_call_of_a_threaded_local_gather` failed
(7 ≠ 10). A first version of `finish()` read the count before folding in a
leaked inner scope; `a_dropped_scope_adds_its_count_to_the_enclosing_one`
caught it, and I fixed it.

**Finding:** `live_remote::run_transport` starts **two** Git processes: a
recorded `config --get core.sshCommand` read (unless `GIT_SSH_COMMAND` is
set), then the unrecorded transport. A future `head_check`/`pr_request`
`git_calls` figure will include both.

Input Robustness Matrix: not applicable (no parser or format reader changed).

### Gates

- `just lint` (worktree): clean. `cargo clippy -p worktree --all-targets`: no warnings.
- `just test` (worktree): 1120/1120 passed, 32 skipped.
- Counter tests with `--features count-git` and with `--no-default-features`: 14/14 each.
- Doc example `git::calls` (`no_run`) compiles (`cargo test -p worktree --doc calls`).
- `just test-l2`, `just test-perf`: not run; no CLI or pipeline output changed in this phase.
- Cross-OS: `just cross-check worktree --os windows` **did not run**. The
  storage preflight refused because `build-win-native` has 15.4 GiB free and
  needs 50 GiB; the automatic sweep freed 0. `--os linux` **did not run**
  either: `build-linux` failed compiling a dependency with `output file
  .../target/release/deps/libprofiling-….rmeta is not writeable`. Both are
  host problems, not code failures. The code is pure `std` (thread-locals,
  `Command`) with no `cfg` branches. The one OS-sensitive assumption is that a
  missing `current_dir` makes `spawn()` fail on Windows (it does:
  `CreateProcessW` rejects the directory). Pull-request CI runs Linux, and a
  push to `main` runs Windows.
- `just check-tier-coverage`: not needed; no tier-marked tests added.

## Phase 4

### Starting state: Phase 3 was only partly done

At the start of this phase, Phase 3's commits had moved the graph gather and
the wait core into the library and added `worktree::list::gather`, but the
CLI still ran its own copy of the pipeline (`gather_listing`,
`prepare_remote`, `follow_remote`, `RemoteAnswers`, …) and never called the
library. Its old pipeline tests (`cli/src/commands/list/tests/pipeline.rs`)
were still present beside the library copies. Phase 3's "CLI thin adapter",
"Move pipeline tests", "pipeline entry", "graph gather", and "Benchmark"
boxes and both Checkpoint 3 boxes were unchecked. Phase 4 adds spans to the
library pipeline, which needs the CLI to use it, so I finished the adapter
first:

- `cli/src/commands/list.rs` now builds `ListOptions` from the flags, calls
  `worktree::list::gather(&current_dir, ..)`, drives the spinner from
  `on_phase` (`WaitProgress::Started` creates it, `Phase` updates it,
  `Finished` clears it), and renders. Its duplicate pipeline, `Stores`,
  `RemoteAnswers`, `RemotePlan`, `Observed`, `Listing`, `prepare_remote`,
  `follow_remote`, and `record_ignore_api` are deleted. The request-notice
  and caption projections stay in the CLI and read the library's
  `RemoteAnswers`/`Observed`.
- I deleted `cli/src/commands/list/tests/pipeline.rs`, because every test in
  it has a library copy in `lib/src/list/tests/pipeline.rs` (diffed by name:
  the library has the same 9 tests plus 5 more). I also deleted the CLI
  overlap seam and the `run_pipeline_*` stage-name tests. The CLI's
  remote-stage tests (`list::tests::gather`) now call the library's
  `prepare_remote` and `follow_remote`.
- I verified the graph gather, pipeline entry, and benchmark items and
  checked them off: `cargo tree -p worktree -e normal` has no
  biscuit-terminal, and `lib/benches/list_status.rs` uses `gather` with
  `ListOptions::default()` (timings off).
- **Phase 3 test fixed:**
  `list::tests::pipeline::concurrent_listings_of_two_repositories_stay_apart_and_leave_the_cwd_alone`
  failed on macOS. Git reports the `/private/var/...` realpath of a temp
  directory, and the test compared it with the `/var/...` spelling. It now
  compares canonical paths (the macOS symlinked-temp trap in the `os` skill).

### Library spans (`lib/src/list.rs`, `lib/src/graph.rs`, `lib/src/timing.rs`)

- New crate-private helpers in `timing.rs`:
  - `measure(stage, work)` returns the span: elapsed time plus `git_calls`
    from a `CallScope`.
  - `record(steps: Option<&mut SpanList>, stage, work)` records into a list,
    or with `None` only runs `work`, reading no clock and opening no scope.
  - `record_parent(steps, stage, kind, work)` does the same for a parent;
    `work` gets the child list.

  Every library step goes through these, so the disabled path is one
  `Option` check.
- `ListOptions::timings`, `Listing::timings: Option<Timings>` (a
  `Scope::Library` document). The Phase 3 `Durations`/`RegatherDurations`
  struct and the `pr_reread`/`remote_wait`/`pr_gather` fields are removed.
- Stages recorded, as spec §3 lays them out: `read_worktrees` (parse plus
  store paths), `origin_lookup`, `prepare_local` (snapshot clone, cache
  load, graph input), the concurrent region, `fast_forward`, `ref_reread`,
  `regather`, `commit`, and `checkout_refresh`. The region is
  `remote_and_local` when `remote.waited.is_some()` and `local_reads`
  otherwise (R9). Its children are `refresh_worker`, `pr_cache_read`,
  `local_gather`, and `graph_history`/`verbose_history`.
- **`refresh_worker`:** `wait::wait` now takes
  `impl FnMut(&Path, &LaunchArgs) -> io::Result<WorkerHandle>` instead of
  the `WorkerLaunch` fn pointer. Existing callers that pass a fn item are
  unchanged. `follow_remote` wraps the launch in a `LaunchClock` that times
  each launch on the foreground monotonic clock only when timing is on.
  `worker_launch` is the sum of every launch attempt (a retry or a failed
  spawn included; the launches do not overlap). `worker_wait` is the `wait`
  call's elapsed time minus that sum. The span has **no `git_calls`**: the
  worker's Git runs in another process, so the count is unknown, not zero.
- **`pr_cache_read`:** the origin recheck plus the store read after the wait.
  With no worker (`ListOptions::worker: None`) it is the stored-answer read
  inside `local_reads`.
- **`local_gather`:** when timed, the list module runs `gather_dirtiness` on
  a counted task beside `gather_ref_facts`, the same split as
  `WorktreeList::gather_local`, which is still used when untimed. Children
  are `worktree_status` ∥ `branch_comparisons`, both concurrent.
- **Graph:** `graph::gather_timed` returns `GraphHistory` (graph needed) or
  `VerboseHistory` (verbose only), with sequential steps that each carry
  counts: `shallow_check` (graph only), `default_tips`, `focused_merge_base`
  (focused view), `verbose_details`, and `lane_assembly`, which covers the
  whole `focused_view`/`base_view` loop. `graph::gather` is unchanged and
  untimed.
- **Regather ruling (gap in R1/R2):** spec §3 asks for "input preparation
  and the concurrent `branch_comparisons` and history steps" under
  `regather`, but the closed stage list has no id for that concurrent group.
  I reused `local_reads`: `regather` (sequential) → `prepare_local`, then
  `local_reads` (concurrent) → `branch_comparisons` ∥
  `graph_history`/`verbose_history`. Dirtiness is never repeated.
- **Counting:** the region's `CallScope` is entered before
  `TaskHandle::current()` is taken, so every task counts into it. Each task
  opens its own nested scopes inside `handle.run`. Every library span except
  `refresh_worker` and its children has a complete count.

### CLI (`cli/src/perf.rs`, `cli/src/commands/list.rs`, `cli/src/args.rs`)

- `perf.rs` only renders now: `report_tree`/`human_report` (a metrics tree
  with `Stage::label()`, `[n git]` suffixes, shares only under sequential
  parents, a generated `unattributed` row hidden under 1 ms, an
  `over-attributed` row always shown), `worker_tree` (a separate section
  titled "Refresh worker (diagnostic, measured in the worker): <status>",
  one row per launch, no shares; it renders whatever `worker_reports`
  carries, which stays empty until Phase 5), and `json_record`
  (`\nWT_PERF_JSON <doc>\n`). `PerfCollector` is gone.
- `run_pipeline` builds a `Scope::Command` document: `startup` (process
  start to pipeline entry), then the library's top-level spans cloned in
  order (no span for the library total), then `caption_status`,
  `display_facts`, `table_render`, `verbose_render`, `notes_render` (status
  plus preliminary notes), `graph_budget`, `graph_render`,
  `final_notes_render`, and `write_output` (assemble plus `eprint!`). The
  total is read right after the write; rendering and writing the report
  happen afterward, so they are excluded. A listing error returns before
  any report.
- `--perf[=human|json]`: `Option<PerfFormat>`, with `num_args = 0..=1`,
  `require_equals`, `default_missing_value = "human"`, and `value_enum`. The
  variants carry `//` comments rather than `///` docs, because a documented
  variant gives `--perf` a long help, which changed `-h` output and the
  completion snapshot. Accepted snapshot change: the `--perf` completion
  description.

### Phase 6 work pulled forward to keep `just test` and `just test-perf` green

Renaming the rows broke seven L1 integration tests and every `perf_` test
that scraped labels. Instead of patching label strings that Phase 6 deletes
anyway, I added the Phase 6 reader and migrated every consumer:

- `perf_support`: `perf_timings(output)` (final nonempty line, LF or CRLF,
  prefix stripped, strict decode, panics on a missing or malformed record),
  `stage_at(&timings, path)`, and `local_gather(&timings)` (under either
  region). `MixedFixture::list_gather_duration` reads `local_gather` from
  `--perf=json`.
- Migrated with the spec's mapping: `perf_pr_request.rs` (`list gather` →
  `local_gather`, `pr gather` → `origin_lookup`, `pr reread` →
  `pr_cache_read`, `remote wait` → `[remote_and_local, refresh_worker]`),
  `perf_graph_stages.rs` (`graph_history` under either region, and
  `graph_render`, both read from `--perf=json` in the pty),
  `cache_warm_path.rs`/`cache_cold_path.rs` (comments; the helper does the
  reading), `list_prs.rs` (`--perf=json`; the two held-request tests read
  `refresh_worker`), and `perf_flag.rs` (the real-report tests read stage
  paths).
- **Left for Phase 6:** deleting `perf_rows`, `stage_from_perf`,
  `list_gather_from_perf`, and their self-tests (`perf_flag.rs` `NESTED`
  tests and `perf_support::tests`), which have no other consumers now. Also
  left: removing `--perf` and the `refresh_worker` bound from the
  `list_prs.rs` functional tests (the plan's scripted-clock replacement),
  and the rest of the `perf_flag.rs` feature list.
- I checked off "Migrate perf consumers". The "Perf helper" item stays
  unchecked until the deletion is done.

### Requirement → test mapping

| Requirement (plan Phase 4 / spec) | Test |
| --- | --- |
| Stage paths per path: no origin | `list::tests::pipeline::timings::without_an_origin_the_local_reads_hold_the_gathers_and_the_graph_steps` |
| remote (wait ran → `remote_and_local`; launch + follow; unknown worker count) | `…::a_followed_wait_makes_the_remote_region_with_the_launch_and_the_follow` |
| no worker (origin, no launch → `local_reads` with `pr_cache_read`) | `…::without_a_worker_the_stored_answers_are_read_inside_the_local_reads` |
| regather (input + `local_reads` group, no dirtiness; graph and `-v` gathered once) | `…::a_regather_records_its_input_and_its_reads_but_never_dirtiness_again` |
| `--ff` moving a checkout | `…::a_fast_forward_that_moves_a_checkout_records_its_refresh` |
| `-v` without image (`verbose_history`, no `shallow_check`) | `…::verbose_details_alone_are_a_verbose_history_with_no_shallow_check` |
| exact reconciliation of every library document | each test above decodes through `Timings::from_json` (strict) and compares the re-encoded JSON |
| `git_calls` complete: top-level counts sum to the recorder; history steps sum to `graph_history`; status count = walks | `…::the_git_counts_cover_every_call_once` |
| disabled mode: same Git calls and facts; no document | `…::timings_on_and_off_do_the_same_work_and_show_the_same_facts` |
| command composition (startup, library spans in place, render stages; reconciles) | `commands::list::tests::a_command_report_holds_the_library_spans_between_startup_and_the_render_stages` |
| renderer: shares only under sequential parents; `[n git]`; remainder rows | `perf::tests::sequential_rows_carry_a_share_and_concurrent_rows_none`, `…::a_sequential_parent_shows_its_steps_with_git_counts_and_its_own_remainder` |
| `unattributed` hidden under 1 ms; over-attribution always shown, at each level | `perf::tests::an_unattributed_remainder_under_a_millisecond_is_hidden`, `…::over_attribution_is_shown_at_every_sequential_level_however_small` |
| worker reports: separate section, no shares, never in the foreground tree | `perf::tests::worker_reports_are_a_separate_section_without_shares` |
| JSON record: one framed line, no ESC/CR, decodes to the same document; human and JSON describe one tree | `perf::tests::the_json_record_is_one_framed_line_that_decodes_to_the_same_document`, `…::the_human_report_names_the_stages_and_both_renderers_describe_one_tree` |
| `--perf` needs `=`; `--perf json` leaves `json` as a subcommand (exit 2); unknown value exit 2 | `perf_flag::the_perf_value_needs_an_equals_sign_and_a_known_format` |
| record follows the listing, final line, stdout empty | `perf_flag::the_json_record_is_the_final_line_after_the_listing` |
| no report on listing errors (both forms) | `perf_flag::list_perf_error_path_emits_no_report` |
| graph is still gathered on a narrow image terminal; none without image or `-v` (recorder, not stage names) | `commands::list::tests::run_pipeline_gathers_the_graph_on_a_narrow_image_terminal`, `…::run_pipeline_without_image_support_or_verbose_gathers_no_graph` |

All new tests are L1 (no tier marker in any path segment; `perf::tests` is
not `perf_`). `cargo nextest list` with the `just test` filter selects all 26
instances (CLI unit tests compile twice). Input Robustness Matrix: no new
reader. The record is read by Phase 1's decoder, whose matrix test already
covers it.

### Gates

- `just lint` (worktree): clean.
- `just test` (worktree): 1043/1043 passed, 32 skipped.
- `just test-perf`: 32/32 passed, including the migrated
  `perf_graph_stages`, `perf_pr_request`, and cache SLA tests.
- `just test-l2`: 39/39 passed. Five Kitty pixel sub-checks reported
  "unavailable" because the Kitty window was covered by another window, a
  host condition the test reports rather than fails.
- Cross-OS: `just cross-check worktree --os linux` **did not run**, for the
  same host problem as Phase 2 (`build-linux` cannot write dependency
  `.rmeta` files in its target directory). The new code is `std` only, with
  no `cfg` branches. `perf_timings` splits CRLF, and the macOS path fix uses
  `canonicalize` on both sides, which is also consistent on Windows (both
  sides get `\\?\`). Windows was not attempted (Phase 2 recorded its storage
  preflight refusal).

### Checkpoint 4: three warm `wt list --perf` runs on this checkout

Debug build (`target/debug/wt`) from `fix-wt-skill`, non-image terminal
(captured stderr), one warm-up run first, read from `--perf=json`:

| Run | Total | Top-level `unattributed` | Human row |
| --- | --- | --- | --- |
| 1 | 565.0 ms | 153 µs | hidden (< 1 ms) |
| 2 | 559.1 ms | 156 µs | hidden |
| 3 | 564.3 ms | 146 µs | hidden |

The rest is pure bookkeeping between steps (for example the `tracking_tip`
computation the facts borrow, which is outside `display_facts` because of
that borrow). On every run the region was ≈470 ms, of which
`refresh_worker` was ≈460 ms (`worker_launch` ≈1 ms). `local_gather` was
≈273–307 ms, almost all `worktree_status` (10 `git status` walks); `commit`
took ≈17–20 ms.

An extra run in a pty with `TERM_PROGRAM=ghostty` showed `graph_history`
1160.8 ms [40 git] = `shallow_check` [1] + `default_tips` [0] +
`focused_merge_base` [1] + `lane_assembly` [38], with an 18.3 ms remainder
under `graph_history`. That run was a loaded debug build with a 5.6 s
`graph_render`. The remainder is time between the recorded steps on the
graph thread (no Git runs there), most likely scheduling while 10 status
walks ran beside it. It is an observation, not a gate.

### Comment and doc drift

- Rewrote the stale docs on the CLI pipeline (the deleted `gather_listing`
  doc described the old `--perf` groups). The library `list` module docs now
  describe `timings`, and `wait::wait` documents the closure launch.
- Updated `worktree/docs/cli/list.md` (the `--perf[=human|json]` row) and the
  `worktree/README.md` `--perf` paragraph. **Not updated:**
  `worktree/docs/performance-testing.md` still describes the old rows and
  `stage_from_perf` (54 mentions). It needs the full rewrite Phase 7
  schedules.
- Skill: `list.md` (the pipeline is now the library's; timing shape and
  helpers; the count-scope ordering trap; round-trip precision),
  `testing.md` (the overlap seam is in the library; assert through
  `regathered`; read perf with `perf_timings`/`stage_at`), and
  `git-graph.md` (gather paths moved in Phase 3; graph timing stages).

## Phase 5

### Worker side (`cli/src/commands/refresh_worker.rs`, `cli/src/args.rs`, `cli/src/main.rs`)

- `LaunchArgs` gained `timings: bool`, and the hidden subcommand gained
  `--timings`. `worker_command` (split out of `spawn_worker` so its arguments
  can be tested) passes the flag only when asked. `run(repo, attempt,
  timings)` reads `Instant::now()` at entry only with timing on.
- The report is `worker_setup` (entry until the halves start: checkout,
  origin, preference, and receipt-target reads), then `worker_halves`
  (concurrent) with `pr_refresh` and `head_refresh`. The total is read as
  soon as both halves join, so the stale-receipt sweep, serialization, and
  write are outside it.
- Each half closure now takes `(main, Option<&Steps>)`. `Steps` is a per-thread
  `Rc<RefCell<SpanList>>`, and `timed_half` wraps each half in its span.
- **Ruling: time the library operations by wrapping the injected seams, not
  by changing `pull_requests::refresh` or `remote_update::run_attempt`.**
  `TimedPrSource` times `OpenPrSource::fetch` (`pr_request`).
  `TimedBranchHeads` and `TimedGit::live_head` add to `head_check`.
  `TimedGit::fetch` is `head_fetch`. Repeated runs (an API request and its
  Git fallback, or `confirm_tip`'s recheck) are summed through
  `SpanList::push`; the runs never overlap. The fallback's phase-record write
  between the two calls falls in `head_refresh`'s remainder rather than in
  `head_check`. The spec asks for the check's duration to "cover both"; it
  covers both calls, without the store write between them. That keeps the
  library API unchanged, so I chose it over threading a timer through `check`.
- A half that panics returns no span. Its outcome is recorded as failed, as
  before, and the report keeps only the other half. Durations hold only stage
  ids and microseconds; a test asserts that the receipt's `durations` member
  has no origin URL, host, key name, or repository.

### Reader side (`lib/src/remote_head.rs`, `lib/src/list/wait.rs`, `lib/src/list.rs`, `lib/src/timing.rs`, `lib/src/strict_json.rs`)

- `Receipt::durations: LaunchReport` (`#[serde(skip)]`, default `Missing`).
  `write_receipt` adds a `durations` member only for a usable report, and
  `RECEIPT_FORMAT_VERSION` stays 1.
- `load_receipt` keeps the typed outcome reader unchanged. Only after the
  outcome is accepted does `receipt_durations` read the member through
  `strict_json::members` (new `Members::contains`):
  - absent is `Missing`;
  - a repeated top-level key, a member that repeats a key, or any decode
    failure is `Invalid`.

  The typed reader ignores unknown members, so a repeated `durations` key
  gets past it, and the strict pass is what refuses last-wins.
- **Ruling (gap in Phase 1): who decides `complete` vs `partial`.** The
  reader infers it from the document. `LaunchReport::from_worker` gives
  `Complete` when `worker_halves` holds both halves, `Partial` when one or
  none is there, and `Invalid` without a `worker_halves` group. The worker
  writes no status of its own. `decode_report` (the `WT_PERF_JSON` decoder)
  now rejects a stated `complete`/`partial` that disagrees with the halves.
  `WorkerTimings::to_value`/`from_value` let the receipt embed and read the
  same version-1 worker document.
- `WaitRequest::timings` sets `LaunchArgs::timings`. With timing on,
  `launch_and_follow` adds a `Missing` entry per successful launch
  (`launch_index` 0-based, `attempt_id` = the launched token). On every poll,
  `Follow::keep_report` copies the first report the polled receipt carries.
  That is the read the wait already makes, before `discard_receipt`, so
  instrumentation adds no read, poll, or wait. `WaitEnd::worker_reports` is
  empty unless timing was requested. A failed spawn has no entry.
- `list::worker_reports` (private) builds the summary for `gather`'s
  library document. It returns `None` without a wait. A changed origin gives
  `OriginChanged` with no entries. Otherwise `adopted` is true when the
  followed head's attempt id is not among the entries (every launch has one
  when timed), and `summarize_worker_reports` decides the summary status.
  `cli/src/commands/list.rs::run_pipeline` copies the library's reports and
  status onto the command document. `perf.rs` already rendered them, so the
  renderer is unchanged.

### Requirement → test mapping

| Requirement (plan Phase 5 / spec) | Test |
| --- | --- |
| Receipt v1 with and without durations; read/write/read round trip; `Missing`/`Invalid` write no member | `remote_head::tests::a_receipt_round_trips_with_and_without_the_workers_durations` |
| attempt/origin/branch/age mismatch still rejected with durations present | `remote_head::tests::durations_never_rescue_a_receipt_for_another_attempt_origin_or_branch` |
| Durations Input Robustness Matrix (166 cells plus a control row; outcome intact in every cell; absent → missing, everything else → invalid, one half → partial, unknown fields ignored, trailing content = the receipt's own matrix) | `remote_head::tests::the_receipt_durations_walk_the_input_robustness_matrix` |
| Capture before deletion (no receipt read after a discard), same end time, same outcomes | `list::wait::tests::reports::a_report_is_taken_from_the_receipt_before_the_receipt_is_deleted` |
| Early publication: ends early, report `missing` | `…::an_early_publication_ends_the_wait_with_the_report_missing` |
| Timeout: report `missing`, budget unchanged | `…::a_timeout_leaves_the_report_missing` |
| Adoption: an owned PR report still shows; without one the summary is `adopted` | `…::an_adopted_head_keeps_our_own_pr_report_and_is_adopted_without_one` |
| Retry: two entries (0, 1), never summed; summary `partial` | `…::a_forced_retry_is_a_second_entry_never_summed_into_the_first` |
| Half panic: `partial` beside the unchanged outcome | `…::a_half_that_panicked_is_a_partial_report_beside_the_unchanged_outcome` |
| Malformed report: `invalid`, outcome stands | `…::an_unreadable_report_is_invalid_and_the_outcome_stands` |
| Failed spawn: no entry, summary `missing` | `…::a_worker_that_cannot_start_has_no_entry` |
| Changed origin suppresses reports; no wait → no section | `…::a_changed_origin_suppresses_every_report` |
| Untimed wait asks for no timings and keeps none (every case above runs both ways) | `untimed_then_timed` helper in `list::wait::tests::reports` |
| Library document carries the reports; no worker → no section; strict decode of the whole document | `list::tests::pipeline::timings::a_followed_wait_makes_the_remote_region_with_the_launch_and_the_follow`, `…::without_a_worker_the_stored_answers_are_read_inside_the_local_reads` |
| `--timings` passed only when asked | `commands::refresh_worker::tests::the_launch_asks_for_timings_only_when_the_listing_does` |
| Worker report shape, interval reconciles, outcome equals the untimed attempt's, no secrets in the document | `commands::refresh_worker::tests::a_timed_attempt_measures_itself_into_its_receipt` |
| `pr_request` only when made | `commands::refresh_worker::tests::a_pr_request_is_recorded_only_when_it_is_made` |
| A check's API request plus its Git fallback form one `head_check`, with no fetch after a failed check | `commands::refresh_worker::tests::a_check_that_falls_back_to_git_is_one_check_covering_both` |
| A panicking half leaves a partial report and the other half measured | `commands::refresh_worker::tests::a_panicking_half_leaves_a_partial_report_and_the_other_half_measured` |
| End to end through the shipped binary: `--perf=json` → real worker with `--timings` → receipt → wait → `WT_PERF_JSON` with `worker_reports` and `complete` | `perf_flag::a_perf_listing_carries_its_workers_own_report` |

Mutation checks, run once and reverted: with `keep_report` disabled, 6 of the
9 `reports` tests fail (the other 3 expect `missing` or no entry). With
invalid durations read as `missing`, the matrix fails 156 of 166 cells.

All new tests are L1. No path segment starts with a tier marker; the
end-to-end test starts `a_perf_`, not `perf_`. The new module is declared
with `mod reports;` in `lib/src/list/wait/tests.rs`, and
`just check-tier-coverage worktree` reports nothing stranded.

### Gates

- `just lint` (worktree): clean.
- `just test` (worktree): 1066/1066 passed, 32 skipped (Phase 4 ended at
  1043).
- `just test-l2`: 39/39 passed.
- `just test-perf`: 32/32 passed. Wait budgets and timing behavior are
  unchanged, including the held-request, held-check, and `--ff`
  held-fetch bounds.
- Cross-OS (Linux, `build-linux`): `just cross-check worktree --os linux`
  failed again in archive mode on read-only `.rmeta` links in the
  `fix-wt-skill` standing clone. That is the host trap in the `os` skill;
  its fix deletes files over SSH, which this session is not authorized to
  run. The skill's workaround, a build flag that forces the native path, ran
  green: `just cross-check worktree --os linux --features count-git` passed
  531/531, and `just cross-check worktree-cli --os linux --features
  terminal-tests` passed (609 tests, including the new worker and
  end-to-end tests). Windows and WSL2 were not run; per the repo's CI
  schedule they come after merge. The new code has no `cfg` branches and no
  path handling.

### Docs, skill, and comment drift

- `worktree/docs/cli/list.md`: a paragraph on the optional receipt timings
  (read after the outcome, never cost the outcome, never extend the wait),
  and the `--perf` row mentions the diagnostic worker section.
- `worktree/README.md`: the `--perf` paragraph describes the worker section
  and its statuses.
- `.claude/skills/worktree/list-remote.md`: worker timing seams, the receipt's
  optional `durations`, and the wait's capture rule. The page is now 300
  lines, the router's threshold for a split, so the next addition should
  give remote timing a page of its own.
- **Drift fixed:** the `remote_head::tests::receipt_cells` doc named
  `worktree-cli`'s `commands::list::wait::tests`. The wait moved to the
  library in Phase 3, and the table is now walked in
  `list::wait::tests::receipt_matrix`, so I corrected the path.
- **Not updated (Phase 7):** `worktree/docs/performance-testing.md` still
  needs the full rewrite Phase 7 schedules. It should now also describe the
  worker section and the receipt `durations`.

## Phase 6

### Starting state

Phase 4 had already moved every perf consumer onto `--perf=json`
(`perf_timings`, `stage_at`, `local_gather`), so this phase was deletion,
two functional-test cleanups, and the missing feature and isolation tests.

### Perf helper (`cli/tests/perf_support/mod.rs`)

- Deleted `list_gather_from_perf`, `PerfRow`, `perf_rows`, `perf_row`,
  `stage_from_perf`, `parse_perf_duration`, `strip_ansi`, and the module's
  `#[cfg(test)] mod tests`. `perf_timings` (final nonempty line, LF or CRLF,
  strict decode, panics on a missing or malformed record) and `stage_at` (the
  path-lookup convenience) stay.
- **Finding:** `perf_support::tests` ran only in `just test-perf`, because
  the `perf_support` path segment matches the `perf_` tier filter, and it ran
  once per test binary that declares the module (9 binaries). That explains
  the drop in `just test-perf` from 32 to 14 tests and in L1 skips from 32 to
  14. The skill now says not to put tests in `perf_support`.
- `rg 'stage_from_perf|perf_rows|list_gather_from_perf'` outside `fixes/`
  finds nothing. To get there I replaced one line in
  `worktree/docs/performance-testing.md` ("Reading a stage in a test") and
  one paragraph in the skill's `testing.md`. The page's full rewrite is still
  Phase 7's job.

### Functional test cleanups (`cli/tests/list_prs.rs`)

- **`list_with` passed `--perf=json` to every `list_prs.rs` test,** not only
  the two named in the spec. It now runs plain `wt list`, so every
  `list_prs.rs` functional test passes without `--perf`.
- `a_held_live_head_check_holds_the_listing_only_until_its_deadline`: the
  `refresh_worker < 5 s` bound is gone. The `.output()`-returns-while-held,
  lock, single-worker, caption, request, and store assertions are unchanged.
- `a_held_pr_request_with_nothing_stored_ends_at_the_budget_with_only_the_hint`:
  the `3 s ≤ refresh_worker < 5 s` bound is gone. The held-request and hint
  assertions are unchanged.
- The scripted-clock budget proofs already existed, so I added none:
  `list::wait::tests::the_budget_ends_the_wait_with_the_last_phase_seen`
  covers the held head check (head running at the budget), and
  `…::the_budget_ends_the_wait_with_the_head_finished_and_the_pr_half_running`
  covers the held PR request with the head finished. Both assert the end
  within 50 ms of `ORDINARY_BUDGET` on the fake clock. The real bounds stay
  in `perf_pr_request::perf_a_held_live_head_check_costs_the_listing_only_its_wait`
  and `…::perf_a_held_pr_request_costs_the_listing_only_its_wait`. A comment
  in each functional test names both.
- **Audit:** outside `perf_`-named tests and `perf_flag.rs`, no test in
  `cli/tests`, `cli/src`, or `lib/src` passes `--perf`, sets
  `timings: true`, or calls `perf_timings`/`stage_at`/`local_gather`/
  `MixedFixture::list_gather_duration`. `cache_warm_path.rs` and
  `cache_cold_path.rs` each hold one test, and both are `perf_`-named.
  `cli/src/commands/list/tests.rs`'s stage checks test the report's
  shape, which the spec allows; they do not prove behavior.

### `perf_flag.rs` feature tests

Already covered before this phase: stderr only and empty stdout, no report on
listing errors (both formats), the framed record after the listing, invalid
values (exit 2), `--perf json` not consuming a subcommand, disabled mode,
and the local-only top level reconciling (strict decode). Added:

- `the_reader_takes_only_the_final_record_with_lf_or_crlf`: the output has a
  decoy record earlier, and trailing blank lines after the real one. Run
  with LF and with CRLF, the reader returns the real record.
- `the_reader_refuses_a_missing_or_malformed_final_record`: the control is
  that the decoy alone decodes. Refused: no output, only blank lines, a
  record followed by listing text, no prefix, truncated JSON, trailing
  content, and a lowercase prefix.
- `a_commit_message_resembling_the_record_is_never_taken_for_it`: a linked
  worktree whose commit subject is a well-formed record. `wt list -v
  --perf=json` shows the subject in the listing (control) and the reader
  returns the real document, which has `verbose_render`. Plain `wt list -v`
  shows the same subject and the reader refuses it.
- `a_pseudo_terminal_capture_ends_in_the_record` (`#[cfg(unix)]`, through
  `script`): the capture has `\r\nWT_PERF_JSON {` and decodes.
- `the_human_and_json_reports_of_one_listing_show_the_same_stages`: two
  runs, one per renderer. Every span of the JSON record, depth first, is a
  human row in the same order. Durations are not compared.
- Same synthetic tree (`cli/src/perf/tests.rs`,
  `the_human_report_names_the_stages_and_both_renderers_describe_one_tree`):
  this unit test now compares the whole labeled tree from `report_tree`,
  without its remainder rows, against the decoded JSON's span tree at every
  depth, and asserts the decoded document equals the original. **Ruling:** I
  added no `insta` snapshot. A snapshot test has to live in an integration
  test, which would need the private `perf` module exposed from the CLI
  library target, plus a width parameter on `human_report`. A structural
  comparison checks the same agreement without that new API, so I chose it.
- Mutation check, run once and reverted: when the reader takes the first
  line containing the prefix, the three reader tests above fail.

### Library isolation tests (`lib/src/list/tests/pipeline/timings.rs`)

- `assert_reconciles`, which every path test calls (no origin, remote, no
  worker, regather, `--ff`, `-v`), now also asserts the document holds no
  command or worker stage. Seventeen stages are listed in `NOT_LIBRARY`.
- `concurrent_listings_of_two_repositories_mix_no_spans_or_counts`: two
  repositories that differ (graph versus `-v`, an extra worktree), each
  listed alone and then both at once on two threads behind a barrier. Each
  document's span tree, with its `git` count at every node, equals the one
  from its solo run. A control asserts the two shapes differ, so mixing
  would show, and the current directory is unchanged afterwards. The test
  passed in 6 of 6 repeated runs.
- On and off giving identical facts and `git` calls was already covered by
  `timings_on_and_off_do_the_same_work_and_show_the_same_facts` (Phase 4).

### Requirement → test mapping

| Requirement (plan Phase 6 / spec §4–5, Tests) | Test |
| --- | --- |
| Final nonempty line only; LF and CRLF | `perf_flag::the_reader_takes_only_the_final_record_with_lf_or_crlf` |
| Reject a missing or malformed final record; never search earlier text | `perf_flag::the_reader_refuses_a_missing_or_malformed_final_record` |
| A commit message resembling the prefix is not mistaken | `perf_flag::a_commit_message_resembling_the_record_is_never_taken_for_it` |
| CRLF (pty) capture | `perf_flag::a_pseudo_terminal_capture_ends_in_the_record` |
| Human and JSON describe the same stages (real runs) | `perf_flag::the_human_and_json_reports_of_one_listing_show_the_same_stages` |
| Both renderers on the same synthetic tree | `perf::tests::the_human_report_names_the_stages_and_both_renderers_describe_one_tree` |
| Report on stderr only, stdout empty, disabled emits nothing, errors emit none, framed after the listing, flag values | existing `perf_flag` tests (listed above) |
| Library reports exclude CLI stages | `assert_reconciles` in all six `list::tests::pipeline::timings` path tests |
| Concurrent calls mix no spans or counts and leave the cwd alone | `list::tests::pipeline::timings::concurrent_listings_of_two_repositories_mix_no_spans_or_counts` |
| Timings on and off: identical facts and Git calls | `…::timings_on_and_off_do_the_same_work_and_show_the_same_facts` (existing) |
| Held-check and held-PR functional tests carry no timing evidence; scripted clock proves the budget | `list_prs` (both tests) + `list::wait::tests::the_budget_ends_the_wait_*` (existing) |

Placement: every new test is L1. No path segment starts with a tier
marker, and `perf_flag` is a test binary name, which the `perf_` filter
does not match. `worktree-cli` builds its integration tests automatically
(no `autotests = false`). `just check-tier-coverage worktree` reports 0
stranded tests.

### Gates

- `just lint` (worktree): clean.
- `just test`: 1070/1070 passed, 14 skipped. Phase 5 ended at 1066; this
  phase added 1 library and 5 `perf_flag` tests and deleted 2.
- `just test-perf`: 14/14 passed. The drop from 32 is the 18 deleted
  `perf_support::tests` copies.
- `just test-l2`: 39/39 passed.
- Cross-OS: `just cross-check worktree-cli --os linux --features
  terminal-tests` passed 592/592, including the pty test with the Linux
  `script -qec` form. `just cross-check worktree --os linux --features
  count-git` passed 532/532, including the concurrency test. Windows and
  WSL2 were not run. The pty test is `#[cfg(unix)]`, the CRLF reader test
  is pure and runs everywhere, and nothing new touches paths or `cfg`.
- Pre-existing failures: none.

## Phase 7

### Docs

- **`worktree/docs/performance-testing.md`**: rewrote the `--perf` section
  (the "Runtime `--perf` flag" heading is kept, so the anchors in
  `docs/cli/list.md` and `docs/git-graph.md` still resolve). It now covers the
  two forms; who measures what, with a Mermaid flow from
  `ListOptions::timings` through the library and CLI documents to both
  renderers and both kinds of test; the full stage table (id, label, what it
  measures, with the CLI-only stages marked) and the worker stage table; a
  real report from this checkout; the two child kinds, reconciliation, and
  `git` counts; worker reports and their six statuses; `--perf=json` framing
  and the version-1 schema with the decoder's rejections; reading a stage by
  path (library and process examples); the perf/functional test rule; and a
  table mapping the old row names to stage paths, because the dated
  measurement tables are historical records and keep their old names. The
  same page also had stale stage names and paths outside that section, which
  I corrected: the graph gather and wait links now point into `lib/src`, the
  bench paragraph describes `worktree::list::gather` instead of
  `list_worktrees()`, the cache gates read `local_gather`, the graph-stage
  section names `graph_history` and `graph_render`, and the PR-request and
  live-head bullets use stage ids. The page does not link to or name this
  fix.
- **Drift fixed on the same page:** `perf_subprocess_counts_meet_sla` was
  said to run "in the ambient `rusty-biscuit` checkout" from `list.rs`. The
  test is in `cli/src/commands/list/tests.rs` and runs on a fixture with one
  linked worktree, so I corrected the doc to match the code.
  `graph_and_verbose_share_one_merge_base` moved to `lib/src/graph/tests.rs`.
  The page's frontmatter `hash` was already stale on `main`; it is now
  recomputed with `md hash`, and `last_updated` is 2026-10-04.
- **`worktree/docs/cli/list.md`**: the `--perf[=human|json]` row now states
  that stdout stays empty, a failing listing prints no report, and the JSON
  line follows an empty line and is the last nonempty stderr line. The
  receipt-timings paragraph was already current (Phase 5).
- **`worktree/docs/git-graph.md`**: the "Who does what" table now says the
  library gathers (`lib/src/graph.rs`, `graph/topology.rs`) and the CLI
  converts (`to_git_graph`), and that the library uses its own types. The
  `--perf` paragraph describes the `graph history` row (`graph_history`) and
  its sequential sub-steps with `git` counts. The Tests section names both
  test files.
- **`worktree/README.md`**: the `--perf` paragraph says to read only the final
  line and that the library returns the same document (scope `library`). The
  bench sentence now describes `worktree::list::gather`.
- **`docs/dependencies.md`**: no crate was added or removed on this branch
  (`git diff main...HEAD -- worktree/lib/Cargo.toml worktree/cli/Cargo.toml`
  is empty). The `worktree/lib` line now says it holds the listing pipeline,
  graph gathering, and timings, and has no terminal dependency. The `worktree`
  area has no per-area `dependencies.md`.

### Skill

- `git-graph.md` "Division of labor" said `commands/git_graph.rs` gathers.
  It now says the library gathers in its own types, the CLI's `to_git_graph`
  converts, and names where each kind of test lives.
- `list.md`: added the `MetricsTree` root-share trap (below).
- `testing.md` (perf/functional split, Phase 6) and `list-remote.md`
  (receipt timings, unchanged waits, Phase 5) were already current. I added
  nothing to `list-remote.md`, which is at the 300-line split threshold.
  The router's "shared modules compiled twice" note still holds: `perf` is
  still declared in both `cli/src/lib.rs` and `cli/src/main.rs`.

### Comment drift pass

I grepped `lib/src`, `cli/src`, `cli/tests`, and `lib/benches` for the old row
names (`pre-dispatch`, `pr gather`, `remote wait`, `list gather`,
`graph gather`, `pr reread`, `list regather`), the deleted helpers, and the
old module paths (`commands::list::wait`, `git_graph::gather`,
`git_graph/topology`). I also read the module docs of `lib/src/list.rs`,
`lib/src/list/wait.rs`, `lib/src/graph.rs`, `cli/src/commands/git_graph.rs`,
`cli/src/commands/refresh_worker.rs`, and `cli/src/perf.rs`.

- **Drift found and fixed:** `cli/tests/perf_support/graph.rs`'s module doc
  named the `graph gather` and `graph image render (biscuit-terminal)` stages
  of `wt list --perf`. It now names `graph_history` and `graph_render` of
  `wt list --perf=json`. This change is comment only.
- The other hits use "remote wait" or "list gather" as ordinary English (in
  panic messages on thread joins and in test docs), not as stage names. They
  are accurate and I left them unchanged.

### Finding: the worker section's heading shows `100%`

`MetricsTree` (biscuit-terminal) always renders its root row's share as
`100%` (`collect_rows`, `is_root`), whatever `MetricShare` it is given.
`perf::worker_tree` builds the worker section as its own tree, so its heading,
`Refresh worker (diagnostic, measured in the worker): complete`, shows `—` and
then `100%`. Every row beneath it shows `—`. Spec §3 says to render worker
reports "without percentages", and Acceptance 5 says "no foreground share".
The worker rows meet that, but the heading does not quite. Fixing it needs
either a biscuit-terminal option to leave the root's share blank or a
different layout in `perf.rs`. Both change shared rendering code, which is
outside a docs phase, so I did not attempt either. I recorded the finding for
review and as a trap in the skill's `list.md`, and the doc's example report
omits that column on the heading.

### Departures from the spec (docs follow the code; spec left as written)

Collected from Phases 1–6:

- `WorkerReport` carries a `LaunchReport` enum rather than `status` plus
  `Option<report>`; the serialized shape is the spec's (Phase 1).
- `worker_report_status` is absent when no worker was followed, and `null` is
  rejected (Phase 1).
- Counting needed hooks in `fast_forward::merge_ff_only` and
  `live_remote::run_transport` as well as `git.rs` (Phase 2).
- `regather` holds `prepare_local` and then a `local_reads` concurrent group,
  because the closed stage list has no other id for that group (Phase 4).
- `head_check` covers the API request and its Git fallback, but not the
  phase-record write between them, which falls into `head_refresh`'s
  remainder. Worker spans are timed by wrapping the injected seams (Phase 5).
- The reader infers `complete` or `partial` from the halves present; the
  worker writes no status (Phase 5).
- No `insta` snapshot compares the two renderers. A structural comparison in
  `cli/src/perf/tests.rs` keeps the `perf` module private (Phase 6).
- The worker heading's `100%` (above, Phase 7).

### Rulings taken at their defaults

R1–R9 (Phase 1) were all taken at the plan's defaults; the gaps they left are
recorded there. The spike S1 outcome (a thread-local counter stack, with
tasks that return their scoped counts) is in Phase 1, "Spike S1".

### Warm-run `unattributed` observations

Phase 4 recorded the three required runs (153, 156, and 146 µs at the top
level, each hidden). I repeated them on the final tree, using the debug
build from `fix-wt-skill` with stderr captured (no graph), after one warm-up
run:

| Run | Total | Top-level `unattributed` | `remote_and_local` | `refresh_worker` | `local_gather` | Worker report |
| --- | ---: | ---: | ---: | ---: | ---: | --- |
| 1 | 372.3 ms | 95 µs | 327.1 ms | 320.9 ms | 196.9 ms | complete |
| 2 | 373.7 ms | 89 µs | 328.8 ms | 323.1 ms | 195.8 ms | complete |
| 3 | 346.8 ms | 89 µs | 302.3 ms | 295.8 ms | 186.7 ms | complete |

On every run the top-level row is hidden (under 1 ms), and no
over-attribution appeared.

### Acceptance

| # | Item | Evidence |
| --- | --- | --- |
| 1 | `worktree::list` exposes the pipeline with `ListOptions.timings`; `wt list --perf` sets it; `perf.rs` only renders | `lib/src/list.rs` (`gather`, `ListOptions::timings`, `Listing::timings`); Phase 4, "CLI"; `cli/src/perf.rs` has only `time`, `json_record`, `human_report`, and the tree builders |
| 2 | Typed `Stage` with id and label in `worktree::timing`; labels are no lookup key | `lib/src/timing.rs` `stages!` table; Phase 6 deleted every label scraper, and `rg 'stage_from_perf\|perf_rows\|list_gather_from_perf'` outside `fixes/` finds nothing |
| 3 | Exact reconciliation in integer µs; over-attribution surfaced; three warm runs logged | `timing::tests` (Phase 1), `assert_reconciles` in every library path test (Phase 6), the strict decode in every `perf_flag` test; runs in Phase 4 and above |
| 4 | `graph_history` shows sub-steps with `git` counts | Phase 4 pty run (`graph_history` [40 git] = 1 + 0 + 1 + 38); `lib/src/graph/tests.rs::a_counting_scope_sees_every_call_of_a_threaded_graph_gather`; the graph path test in `list::tests::pipeline::timings` |
| 5 | Worker reports are diagnostics with no foreground share; missing/adopted/invalid are explicit; return is never delayed | `list::wait::tests::reports` (9 tests, untimed and timed), `perf_flag::a_perf_listing_carries_its_workers_own_report`; the worker rows show `—`. **Caveat:** the section heading shows `100%` (finding above) |
| 6 | No test reads the human report; functional tests use outcomes; real bounds stay in `perf_` tests | Phase 6 audit and deletions; `list_prs` runs without `--perf` |
| 7 | No terminal dependency; no output; behavior preserved; no extra Git or network calls | `cargo tree -p worktree -e normal,build` has no `biscuit-terminal` (and `--all-features` has none of `biscuit-terminal`, `ratatui`, or `crossterm`); the only `print!`-family calls under `lib/src` are the two `eprintln!` calls inside `#[cfg(test)]` modules (`include/copy.rs`, `remove/inventory.rs`); `timings_on_and_off_do_the_same_work_and_show_the_same_facts` |
| 8 | `just test`, `just test-perf`, `just test-l2`, `just lint` pass | Gates below |

### Requirement → test mapping

This phase changed docs, the skill, and one comment. It changed no
behavior, so it added no tests. The behavior the docs describe is covered by
the tests in the Phase 1–6 mappings. Every claim on the rewritten page was
checked against the code or a run: the stage table against `stages!`, the
schema against the `Wire*` types, the framing against `json_record` and
`perf_timings`, `--perf json` against
`perf_flag::the_perf_value_needs_an_equals_sign_and_a_known_format` (exit
2), and the example report against a real run.

### Gates

- `just lint` (worktree): clean.
- `just test`: 1070/1070 passed, 14 skipped.
- `just test-perf`: 14/14 passed.
- `just test-l2`: 39/39 passed.
- Cross-OS: not run. This phase changed one doc comment and no code; the
  Phase 6 Linux cross-checks still cover the code.
- Pre-existing failures: none.

Implementation complete, ready for review. The fix was not moved to
`_completed`, and the spec's `status` was left unchanged: the author's
process gives that change to the author.
