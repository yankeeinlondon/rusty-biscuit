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
packages:
  - worktree
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
