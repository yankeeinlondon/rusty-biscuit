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
