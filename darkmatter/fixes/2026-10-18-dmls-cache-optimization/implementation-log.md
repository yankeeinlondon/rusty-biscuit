---
spec: /Volumes/coding/wt/rusty-biscuit/feat-pipeline/darkmatter/fixes/2026-10-18-dmls-cache-optimization/spec.md
plan: darkmatter/fixes/2026-10-18-dmls-cache-optimization/plan.md
implemented_by: claude/opus
started_phase: 1
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages:
    - darkmatter
    - dmls
human_review: false
message_to_agent: |-
    Phase 3 landed R2 (unchanged effective config invalidates nothing and
    schedules nothing) and R3 (two-lane RegistryQueue in registry_jobs.rs with
    a workspace slot Idle/Queued/Running/RequestedAgain; the worker in
    router.rs RegistryWorker::run_workspace builds the probe list itself and
    runs waiting document jobs before each probe). Startup is NOT on the
    worker yet: spawn_startup still calls OverlayState::prepare_workspace_registries
    on its own thread, so Phase 4 must move it into the queue (consider a
    third lane or a startup job kind; the slot machine assumes one workspace
    pass at a time). New seams: StartupHooks::registry_probe_gate,
    StartupEvent::WorkspaceProbePreparing, LspFixture::start_with_registry_probe_gate,
    release_registry_probe, open_registry_probe_gate, and
    registry_jobs_until_idle (job kinds in handling order). Note the dispatch
    thread reports RegistryJobsOutstanding(n-1) BEFORE RegistryPrepared for
    the same job; registry_jobs_until_idle accounts for this. Ruling 8 was
    satisfied through the existing finish_registry_job -> proceed recheck,
    not on the worker (untitled buffers use for_untitled); see the log.
    Phase 1's test-update list (b) still applies to Phase 4; (c) is done.
    docs/lsp/indexing.md and configuration.md were corrected for R2/R3 drift
    only; Phase 6 still owes the Mermaid sequence and full write-up.
    darkmatter `just test` is now 10512 passed.
source_files_during_phase_2:
    - darkmatter/lib/src/markdown/schemas/triggers/assemble.rs
    - darkmatter/lib/src/markdown/schemas/triggers/mod.rs
    - darkmatter/lib/src/markdown/schemas/triggers/payload_memo_tests.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
    - darkmatter/dmls/src/registry_jobs.rs
    - darkmatter/dmls/src/router.rs
    - darkmatter/dmls/src/overlay/mod.rs
    - darkmatter/dmls/tests/common/mod.rs
    - darkmatter/dmls/tests/l1/registry_preparation.rs
    - darkmatter/dmls/tests/l1/repository_contexts.rs
    - darkmatter/dmls/tests/l1/workspace_schema_scopes.rs
docs_updated_during_phase_3:
    - darkmatter/docs/lsp/indexing.md
    - darkmatter/docs/lsp/configuration.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
    - .claude/skills/darkmatter/dmls.md
---

# Implementation Log for 2026-10-18-dmls-cache-optimization (6 phases)

## Phase 1

### Before sample (R5 / acceptance "before/after repro")

Host: this macOS dev host, 2026-10-18. Binary: `cargo build --locked --release -p dmls`
at `df86e5163` (the `just install-dmls` profile). The throwaway stdio harness is
`/tmp/dmls-repro/repro.py` and is not committed. It sends `initialize` (root = this
worktree), `initialized`, an optional `workspace/didChangeConfiguration
{"settings": {}}`, `didOpen` of
`darkmatter/features/_completed/2026-10-17-dmls-index-store/review-3.md`, then
times the first `textDocument/semanticTokens/full`. Two runs of each variant
(load average about 17 while research subagents ran, so the host was not
fully quiet):

| After `initialized` | Run 1 | Run 2 | `slow request` / `waited_for_trigger_registry` | `schema registry ready elapsed_ms` |
| --- | --- | --- | --- | --- |
| nothing | 207 ms | 161 ms | none | 1044 / 998 |
| `didChangeConfiguration {"settings": {}}` | 1260 ms | 1334 ms | yes / `true` (server elapsed 1072 / 1146 ms) | 1151 / 1222 |

The incident is reproduced: the unchanged config notification adds about 1.1 s,
all of it spent waiting on a trigger registry.

### Memo inputs: what the payload resolution reads (fixes `K` for Phase 2)

**Today's key.** `file_payload_memo::resolve` (`lib/src/markdown/schemas/triggers/assemble.rs:342-384`)
matches on `source`, `schema_roots`, `payload`, and `folder_context(original)`
(`:391-398`). `folder_context` replaces `source_path` with `cwd`, and only when the
source's parent is `cwd`. Every other field is compared through the derived
`PartialEq`, so the probe folder (`cwd`) and its recomputed repository/package scopes
are part of the key. That is why each Claudine probe misses. The fingerprint
(`inputs_fingerprint`, `:403-434`) hashes the listing of each schema root and of
each dependency's parent folder, plus each dependency's bytes. Only successful
results are stored. `resolve_file_payload` (`:271-303`) calls
`resolve_yaml_schema_with_roots(payload, source.parent(), roots, original)`.

**`FileResolutionContext::derive`** (`biscuit-file/lib/src/file_reference/context.rs:852-893`;
struct `:637-674`, `#[derive(PartialEq, Eq)]`, so every field is compared):

| Field | What derivation does |
| --- | --- |
| `source_path` | set to `None` (`for_cwd` / `for_trusted_external_cwd`) |
| `cwd`, `trusted_external_authoring_cwd`, `derived` | set to the new values |
| `inherited_failure` | set to `original.check().err()`, which reads the original `cwd`, tree, request tree, explicit base dir, and repository root |
| `repository_root` / `package_root` / `package_area` | recomputed from the new `cwd` when a scope catalog exists; otherwise kept, except a trusted derivation that changes trees clears them |
| catalog, home, env, magic paths, vault roots, launch `@` scope, `request_cwd`, `request_tree`, `allow_external_relative` | kept |
| `explicit_base_dir`, `opening_anchor` | kept; cleared when a trusted derivation changes trees |
| `tree` | if the original tree is a boundary (not `Fallback`), kept for `for_cwd` and kept for a trusted root only when the tree contains it; otherwise `select_tree()` runs for the new `cwd` |

**Context use in resolution** (`lib/src/markdown/schemas/resolve.rs`). The requesting
context is never read directly. It is forwarded unchanged, including through the
import engine's `NamespaceSource::Files { context }` (`:1396`, `:1713`, `:1753`).
Derivation happens in exactly two places:

- `request_context.for_cwd(D)` in `resolve_file_reference_in_context` (`:647-652`). Its
  callers are path `$schema` strings and delegation (`:589`, `:692-698`), root-union arms
  (`:357`, `:1043`), `Name@file` imports (`:1754` → `named_type_file_path` `:1891`), and
  examples (`:3076`, via `:190`/`:2930`/`:1912`).
- `request_context.for_trusted_external_cwd(root)` in `try_bare_name_in_roots` (`:458`),
  for bare names when roots are set (`:553`, `:1870`, examples).

**Departure from plan ruling 1 (recorded here; spec left as written).** Each nested
delegation, import, or example resolves from **its own file's folder `D`**, so
resolution derives `original.for_cwd(D)` for folders that cannot be listed before
resolving. The planned `K` covers `D = source.parent()` and the roots. Case
analysis of `derive` shows when `A.for_cwd(P) == B.for_cwd(P)` implies
`A.for_cwd(D) == B.for_cwd(D)` and `A.for_trusted_external_cwd(r) == B.for_trusted_external_cwd(r)`
for every `D` and `r`. Equal `for_cwd(P)` results already force equal retained fields,
equal `inherited_failure` (equal `original.check()`), and equal repository scopes.
The one remaining divergence is a mix of tree kinds: one original has a boundary
tree `T` that it keeps, and the other has a `Fallback` tree whose `select_tree()`
for `P` also yields `T`. For a `D` outside `T`, the first keeps `T` and fails the
escape check, while the second selects a different tree. Phase 2's key is therefore:

> `K = (source, payload, schema_roots, original.base_dir(), original.base_dir_origin(), original.for_cwd(source.parent()), [original.for_trusted_external_cwd(r) for r in roots])`

The original tree is read through the existing public `base_dir()` / `base_dir_origin()`,
so no new biscuit-file API is needed. With the original tree in the key, both
remaining cases (both trees boundary, or both trees fallback) derive identically for
every `D` and trusted root. The rule "when in doubt keep the field" is satisfied.
DMLS probes in one repository share the repository's boundary tree and a passing
`check()`, so the three Claudine triggers still give 3 keys (target unchanged).
Execution still uses the original context.

**Pre-existing fingerprint gap (not in scope; spec says the fingerprint is unchanged).**
Path references search the document folder and then the repository root. The
fingerprint lists only dependency parent folders and schema roots, so a newly added
file that would *shadow* a dependency found later in the search order goes
unnoticed until a fingerprinted listing or byte changes. This is unchanged by the
new key, because the trigger's own folder (`source.parent()`) is the same for every
requester, but Phase 2 should not claim the new key fixes it.

### Worker plumbing: tests to update in Phases 3–4

Architecture summary:

- `registry_jobs.rs`: `schedule_workspace` (89-91) sends a `Workspace` job on every call
  (no merge); `outstanding` is incremented per send and decremented in `finished`.
  `wait_for` (115-125) queues one `Document` job per path.
- `router.rs`: `spawn_registry_worker` (477-507) is a FIFO over an unbounded crossbeam
  channel; probe lists are built inside `OverlayState::prepare_workspace_registries`
  (`overlay/mod.rs:560-573`, `workspace_probes` 904-939) on the worker for workspace
  jobs. `spawn_startup` (426-456) runs `prepare_workspace_registries` on its own thread,
  then the file walk, then a second pass if the repository set changed, then sends
  `RegistryReady` **before** `FileList`. Startup never touches `outstanding`.
- `update_config` (1018-1063) clears contexts, calls `invalidate_all_trigger_registries`,
  and calls `schedule_workspace` before comparing configs. Workspace schedulers:
  `install_listing` (635), `rescan_workspace` (966), `update_config` (1021), YAML
  `didSave` (2332), the save fallback (2341), and `apply_watched_changes` (2464). A
  config-file watched event can queue up to three passes.
- `overlay/registry.rs`: `begin` (466-485) starts a `Preparation` even when one is in
  flight; `install` (499-556) handles epoch replay and sequence-number ordering.

(a) Tests that count schedules, passes, preparations, or completions:
- `tests/l1/registry_preparation.rs`: `a_first_document_registry_is_prepared_off_the_dispatch_thread`,
  `requests_on_a_ready_document_do_no_registry_work`, the `assert_refresh_prepares_off_dispatch`
  helper and its users (`a_watched_schema_reload_…`, `a_saved_yaml_reload_…`,
  `a_configuration_reload_…`, `a_save_fallback_rescan_…`, `the_post_refresh_workspace_preparation_…`),
  `startup_preparation_runs_off_the_dispatch_thread` (waits on `RegistryPrepared{Startup}`),
  `a_held_preparation_leaves_edits_cancellation_and_other_requests_running`, and the 13
  `waits_for_its_registry!` tests (each `release_registry_job()` releases one queued job).
- `tests/l1/workspace_schema_scopes.rs::first_level_directories_without_a_schema_root_add_no_scopes`
  (startup scan-count baseline).
- `src/overlay/registry.rs` unit tests (`repeated_ready_lookups_do_no_io`,
  `a_preparation_runs_apart_from_the_cache_and_installs_in_one_step`,
  `an_event_for_an_unchanged_file_does_not_rescan`,
  `a_change_during_a_preparation_leaves_its_result_stale`,
  `an_older_preparation_never_replaces_a_newer_one`) and `src/overlay/mod.rs`
  (`repeated_overlay_builds_skip_the_registry_but_keep_schema_freshness`,
  `the_server_overlay_build_only_looks_registries_up`). These should stay unchanged
  (criterion 5).

(b) Tests that depend on the registry line arriving before the file list. No test compares the
two directly; these call `initialize()` (which waits for `FileListInstalled`) and then
expect the `schema registry ready` line or its diagnostics refresh. R4 reverses the
order, so `initialize()` (or these tests) must also wait for `RegistryReady`:
- `tests/l1/startup_log_lines.rs`: `startup_and_revalidation_lines_carry_the_fixture_counts`,
  `a_failed_trigger_switches_the_registry_line_to_its_failure_variant`, the 5
  `payload_registry_cases!` tests, `a_malformed_envelope_counts_a_failed_trigger`.
- `tests/l1/workspace_schema_scopes.rs` (`assert_registry_line`):
  `startup_counts_the_named_schema_of_an_unopened_package_area`,
  `startup_reports_an_unopened_package_failure_with_no_open_documents`,
  `a_workspace_subfolder_keeps_the_repository_roots_above_it`,
  `each_workspace_folder_prepares_only_the_packages_inside_it`,
  `nested_package_areas_each_prepare_their_roots`,
  `startup_prepares_a_fallback_package_area_with_no_catalog_entry`,
  `startup_prepares_repositories_nested_in_a_non_repository_folder`, and
  `startup_classifies_a_broken_trigger_in_an_unopened_package` (diagnostic from the
  `RegistryReady` refresh).
- `tests/common/mod.rs::initialize` (577); its doc says the registry line comes first.

(c) Tests that rely on an *unchanged* `didChangeConfiguration {settings:{}}` invalidating or
scheduling (R2 makes them wrong as written):
- `registry_preparation.rs::a_configuration_reload_prepares_off_the_dispatch_thread`
  (expects `Workspace` + `Document` preparations); it needs an effective config change.
- `workspace_schema_scopes.rs::a_configuration_change_prepares_a_new_package`: still valid only
  if the dropped context rebuilds differently, so `is_fresh` re-prepares. This is a
  direct check of the R2 context-rebuild path.
- `tests/l1/repository_contexts.rs::a_configuration_change_drops_every_context` (~430):
  expects 3 context rebuilds, one of them for the workspace preparation that will no
  longer be queued.
- `tests/l1/lsp_session.rs` (~109) only checks survival; unaffected.

### Checkpoint 1

- Baseline `just test` in `darkmatter/` before any change: **10482 passed, 10 skipped**
  (37.6 s).
- No source files changed in this phase. The plan and log are the only edits.

### Phase 1 gates

- `git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`
  → `packages: ["darkmatter"]`, `change_class: package`, so the gates were run (not skipped).
- `just lint` (darkmatter): pass.
- `just test` (darkmatter): 10482 passed, 10 skipped.
- Requirement-to-test mapping: Phase 1 changes no behavior, so it adds no tests. The
  tests that later phases must add or update are listed above.
- No dependent-package tests and no cross-OS check: no source changed.
- The `darkmatter` skill is not updated in this phase; Phase 6 adds the worker priority
  rule and memo key once implemented.

## Phase 2

### What changed (R1)

`lib/src/markdown/schemas/triggers/assemble.rs`, module `file_payload_memo`:

- **Key.** `Key { source, payload, schema_roots, tree, tree_origin, folder_context, root_contexts }`,
  exactly the Phase 1 key: `original.base_dir()` and `original.base_dir_origin()`,
  `original.for_cwd(source.parent())` (mirroring `resolve_file_payload`'s `"."`
  fallback), and `original.for_trusted_external_cwd(root)` for each schema root, all
  derived **directly from the original** and compared by full `PartialEq`. The key is
  built per lookup. Execution still passes the original context to
  `resolve_file_payload`. `folder_context()` (the old requester-folder key) is removed.
- **Capacity and eviction.** `CAPACITY = 32` (was `RECENT_PAYLOADS = 16`). The entry
  list is least-recently-used first: a fingerprint-validated hit moves its entry to the
  end, and an insert at the bound removes the front. The fingerprint and the
  "only successes are kept" rule are unchanged.
- **Counter seam (ruling 2).** New public `PayloadMemoTally` (guard) and
  `PayloadMemoCounts { hits, misses }`, re-exported from
  `darkmatter::markdown::schemas::triggers`. A thread-local stack of counts. Only the
  most recently started tally on the thread records, so a document job that interrupts
  a workspace pass on the worker thread starts its own tally and is excluded from the
  pass's counts. Dropping a tally truncates the stack to its depth. The guard is `!Send`.
  A hit is recorded after a passing fingerprint; a miss before each uncached resolution
  attempt, whether it succeeds or fails. A registry lookup that never reaches the memo
  records neither. No new biscuit-file API.
- Doc comment on the memo rewritten. It now describes the new key and why the requester's
  folder is excluded and the requester's tree is kept. The old text ("the request
  context are equal") described the removed behavior. That drift is resolved by the
  rewrite.

### Tests (F1, L1, in-process; `lib/src/markdown/schemas/triggers/payload_memo_tests.rs`)

These are unit tests rather than `tests/l1` integration tests. `TriggerRegistry::payloads`
(the `Arc` under test) is `pub(crate)`, and exposing it only for tests would widen the
public API. They drive the public `scan` and the public tally. The fixture is a repository
with a `RepositoryScopeCatalog` (as DMLS builds one context per repository), packages
`pkg-a`/`pkg-b`, and two empty homes.

| Requirement | Test |
| --- | --- |
| Two package folders share one resolution, same `Arc`, own registry context | `documents_in_two_package_folders_share_one_resolution` |
| Matrix control row (unedited context hits) | `control_same_repository_context_hits` |
| Different request boundary (no catalog) | `a_different_request_boundary_misses` |
| Different launch `@` scope | `a_different_launch_scope_misses` |
| Different home | `a_different_home_misses` |
| Different environment | `a_different_environment_misses` |
| Explicit tree controls | `an_explicit_tree_control_misses` (`allow_external_relative`), `an_explicit_base_dir_misses` |
| Inherited failure never cleared by a hit | `an_inherited_validation_failure_is_never_cleared_by_a_hit` |
| Trusted external root (`SCHEMAS_DIR`): shared within one repository context, kept apart by launch scope | `a_trusted_external_root_is_shared_within_one_repository_context` |
| Inline payload with nested-folder `Name@file` import and an inherited `example(...)` | `an_inline_payload_with_nested_imports_and_examples_is_shared` |
| Boundary vs. fallback tree requesters never share | `requesters_with_different_tree_kinds_never_share` |
| 18 distinct full keys then a forced repeat: 18 misses, then 18 hits / 0 misses | `eighteen_distinct_keys_fit_and_a_repeat_scan_resolves_nothing` |
| LRU: a hit protects an entry; the least recently used is evicted at 32 | `a_hit_protects_an_entry_and_the_least_recently_used_is_evicted` |
| Changed dependency still misses | `an_edited_dependency_is_resolved_again` |
| File added to a schema root still misses | `a_file_added_to_a_schema_root_is_resolved_again` |
| Failure counts a miss and is not kept | `a_failed_resolution_is_a_miss_and_is_not_kept` |
| Nested tally excludes an interruption; other threads are not counted | `a_nested_tally_excludes_the_interruption_from_the_outer_one` |
| Criterion 6 (registries, diagnostics, summaries unchanged on `schema-triggers` fixtures) | existing `cli/tests/l1/schema_triggers.rs`, `lib/tests/l1/shipped_schema_corpus.rs`, `schema_phase_validation.rs`, `inline_document_text.rs`, and the 3 existing memo tests in `assemble.rs`, all passing unchanged |

**Load-bearing proof.** Two temporary mutations, both reverted. (1) Keying on the
requester's context (`folder_context: original.clone()`, the old behavior) fails the five
sharing tests: package folders, control row, trusted root, imports/examples, and nested
tally. (2) An over-broad key (no context, no tree) fails all nine "misses" rows.

The eviction tests rely on nextest's process per test (documented in the module). The
memo is process-wide, so under `cargo test` another test's inserts could reorder recency.

The "3 misses for the three-trigger DMLS input set" acceptance number is not measured here.
It needs the DMLS pass log (Phase 5) and the after-sample (Phase 6). The library-level
equivalent is the 18-key test above.

Input Robustness Matrix: not applicable (no parser or loader changed). Code-smell grep of the
touched code: no `#[serde(default)]` or `.ok()`. The one `unwrap_or_default()` is in
`PayloadMemoTally::counts` for an out-of-order drop, not on a parse.

### Gates

- `git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`
  → `packages: ["darkmatter", "test-toolkit"]`, `change_class: package`. The gates were run.
  `test-toolkit` has only a lint row (a suite input it owns): `cargo clippy -p test-toolkit --all-targets -- -D warnings` is clean.
- `just lint` (darkmatter): pass (one `cloned_ref_to_slice_refs` in the new test, fixed).
- `just test` (darkmatter): **10500 passed, 10 skipped**. The baseline was 10482, and this phase adds 18.
  No failures, no reruns.
- Dependents, serially and targeted to schema/trigger users. `claudine` (`claudine_schemas*`, `composition::schema`,
  `composition::completion`, `inline_prompt`, `sequence::formal`): 318 passed. `claudine-cli`
  (`schema_completion`, `signals`): 57 passed. `messenger --features research` (`research*`): 59 passed.
- Cross-OS: `scripts/cross-check.sh --os windows darkmatter schemas::triggers` passed on build-win-native, 126 tests.
  (`just cross-check … -E 'test(...)'` fails in the remote shell's quoting of `(`; a plain
  positional filter works.)

### Notes for later phases

- `PayloadMemoTally::start()` must be started on the **worker thread** that runs the pass, and an
  interrupting document job must start its own tally (and drop it) inside the pass's lifetime.
- No docs or skills changed in this phase. `docs/lsp/indexing.md` and `.claude/skills/darkmatter/dmls.md`
  get the memo key in Phase 6. No `docs/` page described the old key, so nothing drifted.

## Phase 3

### What changed

**R2: config reload (`dmls/src/router.rs`, `update_config`).** The function snapshots `before`,
applies `update`, and compares the effective config **first**. When the config is unchanged it
still calls `contexts.clear()`, and `relink` + `refresh_all_diagnostics` run when contexts were
dropped. It no longer calls `invalidate_all_trigger_registries` or `schedule_workspace`. When the
config changed, the previous path is kept (relist, wiki roots, relink, semantic-token refresh
decision), followed by blanket invalidation and **one** workspace schedule after the new config
applies, then `refresh_all_diagnostics`. Config-error diagnostics are published by
`after_config_change`, outside `update_config`, so they are unaffected. `is_fresh` (`scope.context
== context.for_source(path)`) was confirmed to re-prepare a document whose context rebuilds
differently; a test covers it below. The `reload_config` doc comment said "even when the effective
config is unchanged … the workspace scopes' preparation is queued at once"; that drift was rewritten.

**R3: queue core (`dmls/src/registry_jobs.rs`).** The crossbeam FIFO is replaced by
`RegistryQueue`: a `Mutex` + `Condvar` over a document lane (`VecDeque<RegistryNeed>`) and one
workspace slot `Idle → Queued → Running → RequestedAgain`. Both threads change the slot under the
one lock. `schedule_workspace` returns whether it added a pass: Idle→Queued and
Running→RequestedAgain add one; Queued and RequestedAgain merge. The worker's `next()` returns a
document first, otherwise moves Queued→Running. `finish_workspace()` (worker, before it reports the
completion) moves RequestedAgain→Queued and anything else →Idle, so a schedule made after it always
adds exactly one pass. `PendingRegistries` holds `Arc<RegistryQueue>`. `outstanding` is incremented
only when a job was really added (each document job, the queued/running pass, and its follow-up),
and decremented once per completion. `Drop` closes the queue so the worker exits. The module doc now
carries the slot diagram.

**R3: worker (`router.rs`, `spawn_registry_worker` / `RegistryWorker`).** `run_workspace` holds on
the job gate, runs waiting document jobs, builds the probe list **on the worker**
(`OverlayState::workspace_probes`, new), then for each probe runs waiting document jobs first
(each reported as it finishes), and prepares that one probe (`OverlayState::prepare_workspace_probe`,
new). `prepare_workspace_registries` is now those two calls; startup still uses it unchanged, since
Phase 4 moves startup. New test seams: `StartupHooks::registry_probe_gate` and
`StartupEvent::WorkspaceProbePreparing`.

**Departure (ruling 8, stale document job).** The worker prepares the context captured in
`RegistryNeed`; it does not re-read the context. Re-reading on the worker is not equivalent,
because untitled buffers resolve through `for_untitled`, not `for_document`. Ruling 8's intent
holds through the existing dispatch path: `finish_registry_job` → `proceed` re-checks
`registry_need` with the current context and re-parks waiters on a new job, and
`refresh_diagnostics` holds the publish again. So no waiter is answered from an outdated-context
registry. The cost is one wasted preparation in that race. Phase 4's lifecycle test ("a context
change during queued work cannot strand waiters") should exercise it.

**Fixture (`dmls/tests/common/mod.rs`).** New `start_with_registry_probe_gate`,
`release_registry_probe`, `open_registry_probe_gate`, and `registry_jobs_until_idle` (the finished
job kinds in handling order). The dispatch thread reports a job's decremented count *before* its
`RegistryPrepared`. The collector therefore owes one completion event per decrement; otherwise it
stops one event early (found while writing the tests).

### Requirement-to-test mapping (all L1, F2 in-process LSP sessions, unless noted)

| Requirement | Test |
| --- | --- |
| R2: unchanged notification queues no job and leaves every registry fresh (0 scans then, 0 on the next request) | `registry_preparation::an_unchanged_configuration_notification_prepares_nothing` |
| R2: unchanged notification still drops contexts, and the folder's context is not rebuilt (no workspace pass) | `repository_contexts::an_unchanged_configuration_drops_every_context_and_prepares_no_workspace_scope` |
| R2: context rebuilt differently → only that open document re-prepares (`[Document]`, no Workspace) and publishes the new package's trigger error | `workspace_schema_scopes::an_unchanged_configuration_prepares_an_open_document_whose_context_changed` |
| R2 control: effective change → one Workspace + the Document | `registry_preparation::an_effective_configuration_change_prepares_one_workspace_pass_and_the_open_document` |
| R2: effective change still invalidates (updated from `{settings:{}}` to a formatting change) | `registry_preparation::a_configuration_reload_prepares_off_the_dispatch_thread`, `workspace_schema_scopes::a_configuration_change_prepares_a_new_package`, `repository_contexts::a_configuration_change_drops_every_context` (still 3 rebuilds) |
| R3: W, W, D queued behind a held job → `[Document, Document, Workspace]` | `registry_preparation::a_queued_document_runs_before_a_merged_workspace_pass`; unit (F1) `registry_jobs::tests::a_document_queued_after_two_workspace_schedules_runs_first_and_one_pass_follows` |
| R3: document job queued while probe 1 is held runs before probe 2; its request is answered while the pass is held at probe 2 | `registry_preparation::a_document_job_runs_between_two_probes_of_a_running_pass`; unit `a_running_pass_takes_the_documents_queued_meanwhile_in_order` |
| R3: schedules during a running pass → exactly one follow-up | `registry_preparation::schedules_during_a_running_pass_add_one_follow_up`; unit `schedules_during_a_running_pass_add_exactly_one_follow_up` |
| R3: schedule after the pass ends but before dispatch handles its completion → exactly one new pass | unit `a_schedule_after_a_pass_ends_but_before_its_completion_is_handled_adds_one_pass`. Not L1: the window between the worker's `finish_workspace` and the dispatch handling cannot be held from outside without a new seam inside it; the queue owns the transition, so F1 covers it fully |
| Shutdown wakes a waiting worker; a closed queue takes nothing | unit `a_closed_queue_takes_and_hands_out_nothing` |
| Bounded gates | every wait is bounded by `MESSAGE_BOUND` (10 s); dropping the fixture drops the gate senders, so a held worker's `recv` returns |
| Criterion 5 (change replay, failure transitions unchanged) | existing `overlay/registry.rs` and `overlay/mod.rs` unit tests and the rest of `registry_preparation.rs`, passing unchanged |

**Load-bearing proof (temporary mutations, all reverted).** (1) Reverting `update_config` to
invalidate and schedule before the compare fails all three unchanged-config tests. (2) Removing the
in-loop yield makes `a_document_job_runs_between_two_probes_of_a_running_pass` time out. (3)
Workspace-before-documents in `next()` fails the unit order test. The L1 order test still passes
then, because the pass's yield before its first probe also puts documents first: two independent
guarantees. (4) Workspace-first with no yields at all (the old FIFO behavior) fails the L1 order test
with `[Document, Workspace, Document]`.

Input Robustness Matrix: not applicable (no parser or loader).

### Docs and skill (drift fixed in this change)

- `docs/lsp/indexing.md`: the "When the registry changes" table said every configuration change marks
  every registry stale. It now splits into effective-change and unchanged rows. The worker section
  ("queues one job") now describes workspace-pass merging and the document-first rule. The fuller
  Phase 6 write-up and its Mermaid sequence are still owed.
- `docs/lsp/configuration.md`: the reload paragraph now states the unchanged-reload behavior.
- `.claude/skills/darkmatter/dmls.md`: queue model, priority rule, R2 rule, and the new test seams.

### Gates

- `git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`
  → `packages: ["darkmatter", "dmls", "test-toolkit"]`, `change_class: package`. The gates were run.
  (The tree still holds Phase 2's uncommitted library changes, so `darkmatter` is listed too.)
- `just lint` (darkmatter): pass, no warnings. `cargo clippy -p test-toolkit --all-targets -- -D warnings`: clean.
- `just test` (darkmatter): **10512 passed, 10 skipped** (39 s). That is 10500 plus 12 new: 5 queue unit
  tests, 5 in `registry_preparation.rs`, 1 in `repository_contexts.rs`, and 1 in
  `workspace_schema_scopes.rs`. No failures and no reruns. No new test was slow (the slowest new L1
  test is about 0.1 s).
- `just check-tier-coverage darkmatter`: stranded 0.
- Dependents: no workspace package depends on `dmls` (`cargo metadata`), so there are no
  dependent-package tests. No library (`darkmatter`) code changed in this phase.
- Cross-OS: not run. The change is thread and queue logic (`Mutex`/`Condvar`) with no path,
  process, or console handling, so there is no notable OS-specific risk. The nightly Windows/WSL2
  cells will cover it.
- Pre-existing failures: none seen.
