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
source_files_during_phase_4:
    - darkmatter/dmls/src/lib.rs
    - darkmatter/dmls/src/overlay/mod.rs
    - darkmatter/dmls/src/overlay/registry.rs
    - darkmatter/dmls/src/registry_jobs.rs
    - darkmatter/dmls/src/router.rs
    - darkmatter/dmls/src/work_counters.rs
    - darkmatter/dmls/tests/common/mod.rs
    - darkmatter/dmls/tests/l1/main.rs
    - darkmatter/dmls/tests/l1/startup_preparation.rs
    - darkmatter/dmls/tests/l1/workspace_file_list.rs
docs_updated_during_phase_4:
    - darkmatter/docs/lsp/indexing.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
    - .claude/skills/darkmatter/dmls.md
source_files_during_phase_5:
    - darkmatter/dmls/src/overlay/mod.rs
    - darkmatter/dmls/src/overlay/registry.rs
    - darkmatter/dmls/src/request_timing.rs
    - darkmatter/dmls/src/router.rs
    - darkmatter/dmls/tests/common/mod.rs
    - darkmatter/dmls/tests/l1/main.rs
    - darkmatter/dmls/tests/l1/registry_observability.rs
docs_updated_during_phase_5:
    - darkmatter/docs/lsp/indexing.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
    - .claude/skills/darkmatter/dmls.md
source_files_during_phase_6: []
docs_updated_during_phase_6:
    - darkmatter/docs/lsp/indexing.md
docs_created_during_phase_6: []
skills_files_updated_during_phase_6:
    - .claude/skills/darkmatter/dmls.md
source_code:
    - darkmatter/lib/src/markdown/schemas/triggers/assemble.rs
    - darkmatter/lib/src/markdown/schemas/triggers/mod.rs
    - darkmatter/lib/src/markdown/schemas/triggers/payload_memo_tests.rs
    - darkmatter/dmls/src/lib.rs
    - darkmatter/dmls/src/overlay/mod.rs
    - darkmatter/dmls/src/overlay/registry.rs
    - darkmatter/dmls/src/registry_jobs.rs
    - darkmatter/dmls/src/request_timing.rs
    - darkmatter/dmls/src/router.rs
    - darkmatter/dmls/src/work_counters.rs
    - darkmatter/dmls/tests/common/mod.rs
    - darkmatter/dmls/tests/l1/main.rs
    - darkmatter/dmls/tests/l1/registry_observability.rs
    - darkmatter/dmls/tests/l1/registry_preparation.rs
    - darkmatter/dmls/tests/l1/repository_contexts.rs
    - darkmatter/dmls/tests/l1/startup_preparation.rs
    - darkmatter/dmls/tests/l1/workspace_file_list.rs
    - darkmatter/dmls/tests/l1/workspace_schema_scopes.rs
documentation:
    - darkmatter/docs/lsp/indexing.md
    - darkmatter/docs/lsp/configuration.md
    - .claude/skills/darkmatter/dmls.md
completed_phase: 6
implemented: true
human_review: false
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

## Phase 4

### Recovery note

This run found Phase 4's code, tests, docs, and skill edits already in the working tree from an
earlier attempt, with every Phase 4 task checked off but no log section written. This run reviewed
that diff against rulings 5–7 and the plan. It found nothing to change and then ran the gates
itself (below). This section records that work.

### What changed

**Startup on the worker (R4, `dmls/src/registry_jobs.rs`).** `RegistryQueue` now has three lanes:
documents, a `startup: VecDeque<StartupPhase>` lane, and the workspace slot. `next()` returns a
document first, then a startup phase, then a queued workspace pass. `StartupPhase` (public, re-exported
from `lib.rs`) is `Initial` (folders and the packages and areas inside them) or `Discovered` (the
repositories the walk found). `PendingRegistries` gained a `StartupProgress` record, used as follows:

- `begin_startup` queues `Initial` and counts startup as **one** outstanding job until it is ready.
- `startup_phase_finished` and `startup_discovered` record the two halves. `advance_startup` queues
  `Discovered` only after **both** the initial phase finished and discovery reported a changed
  repository set. Otherwise it ends startup and returns `StartupReady`: the summary after the last
  phase, the summed preparation time, and the worker thread.
- `drain_waiters` became `shut_down`. It closes the queue, drops startup, zeroes `outstanding` (and
  reports it), and returns the waiters, which are answered once.

**Worker and dispatch (`dmls/src/router.rs`).** `spawn_startup` now runs only discovery and sends
`Internal::FileList`. It never prepares a registry. `RegistryWorker::run_pass` is shared by
`run_workspace` and the new `run_startup`. It runs waiting document jobs, builds the probe list on the
worker, and runs waiting document jobs again before each probe. Its returned preparation time counts
probe-list building and probe preparation only, so queue waits, interrupting document jobs, and the
file walk are excluded. `run_startup` sends `Internal::StartupPhase { phase, preparing, summary, thread }`
(this replaces `Internal::RegistryReady`). Dispatch handles the messages as follows:

- `FileList` installs the list at once (new `ServerState::install_file_list`, which does not queue
  anything), refreshes diagnostics, sends `FileListInstalled`, starts a waiting index round, and then
  reports discovery to `PendingRegistries`.
- `StartupPhase` publishes trigger transitions, sends `StartupPhaseFinished(phase)`, and reports the
  phase.
- When startup is ready, `Router::finish_startup` logs the one `schema registry ready` line, refreshes
  diagnostics, retires the outstanding job, and sends `RegistryReady` and then
  `RegistryPrepared{Startup}`.

A changed repository set during startup is covered by the `Discovered` phase. Outside startup it
still schedules a workspace pass.

**Overlay (`dmls/src/overlay/mod.rs`, `overlay/registry.rs`).** `prepare_workspace_registries` was
removed, since nothing calls it any longer. `prepare_workspace_probe` takes a `hold` callback, and the
new `prepare_registry_held` calls it after `Preparation::run` (inputs read) and before the install, so
a test can record a change while a preparation is in flight. Preparation still holds no overlay lock
across I/O. The existing change/epoch replay in `install` is unchanged. `Preparation::run` keeps a
`RegistryScan` guard alive for the scan.

**Overlap detector (`dmls/src/work_counters.rs`).** `record_registry_scan` now returns a guard that
counts in-flight scans. The new public `peak_concurrent_registry_scans()` reports the most scans that
were ever in progress at once.

**Test seams.** The changes span the server and the fixture:

- In the server: `StartupHooks::startup_probe_gate`, plus `StartupEvent::StartupProbePreparing(phase)`
  and `StartupEvent::StartupPhaseFinished(phase)`.
- In the fixture: `LspFixture::start_with_startup_probe_gate`, `release_startup_probe`,
  `open_startup_probe_gate`, `startup_events_until`, and `wait_for_startup_events`.
- `spawn_with_all_hooks` now takes a `StartupHooks` instead of positional gates.
- `initialize` now waits for both `FileListInstalled` and the startup `RegistryPrepared`, in either
  order.

### Departures and notable decisions

- Startup is a **third lane** rather than a variant of the workspace slot, so a refresh scheduled
  during startup still merges in the slot and runs after the phase. One worker thread still runs one
  job at a time, which keeps ruling 6's "no two preparations overlap".
- The startup gate holds **after reads, before install** (unlike `registry_probe_gate`, which holds
  before the probe). That position is what makes the "change during the in-flight preparation leaves
  the scope stale" test meaningful. A probe that is still fresh prepares nothing and does not wait.
- The `RegistryReady` and `FileListInstalled` order is reversed: the list now installs first. One test
  had to change: `workspace_file_list::path_diagnostics_wait_for_the_file_list` used `latest` (a
  flush), which now waits for startup, and startup is outstanding until discovery returns. The test
  now settles with a `documentSymbol` request instead. This was the remaining item of Phase 1's
  test-update list (b).
- Ruling 8 (stale document job) is exercised as the Phase 3 log asked:
  `a_context_change_while_a_document_job_is_queued_requeues_its_waiter`.

### Requirement-to-test mapping

All tests are L1. They are F2 (in-process LSP sessions) unless marked F1 (unit tests in
`registry_jobs::tests`). The new L1 tests are in `dmls/tests/l1/startup_preparation.rs`.

| Requirement | Test |
| --- | --- |
| Queue order: documents, then startup phases, then the workspace pass | F1 `documents_run_before_a_startup_phase_and_a_startup_phase_before_a_workspace_pass` |
| The second phase waits for both the initial phase and discovery (both arrival orders); startup stays one outstanding job between phases; elapsed time is the sum of the phases | F1 `the_discovered_phase_waits_for_both_the_initial_phase_and_discovery` |
| Unchanged repository set: no discovered phase | F1 `discovery_that_changes_no_repository_ends_startup_after_the_initial_phase` |
| Shutdown releases startup and the worker takes nothing | F1 `shutdown_during_startup_releases_it_and_takes_no_further_phase` |
| With the initial phase held, the file list is installed and published first; the discovered phase follows both; one ready line counts every startup scope (`roots=2`) | `the_file_list_installs_while_the_initial_phase_is_held_and_the_discovered_phase_follows_both` |
| Startup and a refresh together: peak concurrent scans = 1, every scan on one non-dispatch thread, fresh probes skipped (scans equal a plain startup's), discovered repositories prepared before ready | `startup_and_a_refresh_share_one_worker_and_skip_fresh_probes` |
| A change recorded during the in-flight preparation leaves the scope stale (the broken trigger's diagnostic is published by the follow-up) | `a_change_during_an_in_flight_startup_preparation_leaves_the_scope_stale` |
| The incident repro: unchanged config + open + `semanticTokens/full` during startup; the response arrives while the next startup probe is held; no workspace pass queued | `a_request_overtakes_the_remaining_startup_probes_after_an_unchanged_configuration` |
| Criterion 3: the unchanged notification costs zero extra preparations (scans = plain startup + the document's own) | same test, compared with a control session |
| Shutdown during the initial phase: the waiter is answered once (`ServerCancelled`), outstanding drops to 0, never ready | `shutdown_during_the_initial_phase_releases_waiters_and_startup` |
| Shutdown during the discovered phase | `shutdown_during_the_discovered_phase_releases_startup` |
| A context change during queued work does not strand the waiter (ruling 8) | `a_context_change_while_a_document_job_is_queued_requeues_its_waiter` |
| Waiters share a document job; cancelling one still answers the other once | existing `registry_preparation::a_held_preparation_leaves_edits_cancellation_and_other_requests_running` (passes unchanged) |
| Bounded gates | every wait is bounded by `MESSAGE_BOUND`; dropping the fixture drops the gate senders |

Input Robustness Matrix: not applicable (no parser or loader).

### Docs and skill

- `darkmatter/docs/lsp/indexing.md`: the startup section described the old three-step sequence on one
  thread, with the file list installed after the registry line. That was drift; it now describes
  discovery running beside the initial phase, the list installing first, the conditional
  discovered phase, and shutdown. The flowchart was replaced by a sequence diagram.
- `.claude/skills/darkmatter/dmls.md`: the startup order, the three-lane queue, the overlap detector,
  the new seams, and the reversed event order.

### Gates

- `git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`
  → `packages: ["dmls", "test-toolkit", "darkmatter"]`, `change_class: package`. The gates were run.
- `just lint` (darkmatter): pass, no warnings.
- `just test` (darkmatter): **10523 passed, 10 skipped** (36 s). That is 10512 plus 11 new: 4 unit tests
  and 7 L1 tests. No failures.
- Flakiness check: the gate-heavy binaries (`startup_preparation`, `registry_preparation`,
  `workspace_file_list`, 49 tests) were run 5 more times, and all passed each time.
- `just check-tier-coverage darkmatter`: stranded 0.
- Dependents: no workspace package depends on `dmls`. No library (`darkmatter`) code changed.
- Cross-OS: not run. This is thread, queue, and channel logic with no path, process, or console
  handling; the nightly Windows/WSL2 cells will cover it.
- Pre-existing failures: none seen.

## Phase 5

### What changed (R5)

**Pass line (`dmls/src/router.rs`, `dmls/src/overlay/`).** `OverlayState::prepare_workspace_probe` now
returns a `ProbeOutcome` (`Fresh`, `Prepared`, `Failed`). It comes from a private `run_preparation`
that `prepare_registry_held` also uses, so the existing `bool` ("fresh afterwards") contract is
unchanged. `Prepared::failed()` (in `overlay/registry.rs`) tells whether the registry loaded. A
folder whose context cannot be built counts as `Failed`. A probe under no trigger boundary counts as
`Fresh`, since nothing is read for it. `RegistryWorker::run_pass` now returns a `RegistryPass`: its
preparation time, the probe count, the per-outcome counts, and the `PayloadMemoCounts` of a
`PayloadMemoTally` it starts on the worker thread after the first `run_waiting_documents`.
`run_document` starts its own tally, which takes over counting until the job ends, so a document job
that interrupts a pass is never counted in it (ruling 2). `run_workspace` logs one `debug`
`trigger registries prepared probes prepared fresh failed payload_hits payload_misses elapsed_ms
workspace` line through `log_workspace_pass`. The worker now gets the workspace identifier;
`workspace_identifier` is computed before the worker is spawned. Startup phases do not log the line.
The info-level `schema registry ready` and failure/recovery lines are unchanged.

**Wait duration (`dmls/src/request_timing.rs`, `router.rs`).** `RequestTimings` keeps a
`RegistryWait` (closed-interval total plus the start of an open interval) on the injected clock.
Three methods use it:

- `begin_registry_wait` records `Wait::TriggerRegistry` and opens an interval. `proceed` calls it
  instead of `record_wait` when it parks a request.
- `end_registry_wait` closes the open interval. It runs at the top of `proceed` (resume, including
  the requeue path, which parks again) and in `answer_waiter` (cancellation, shutdown, index
  failure).
- `registry_wait` returns the total, including an interval that is still open.

The completion event gains `trigger_registry_wait_ms`, present only when the request parked. No
second event was added. Handler work and index waits fall outside every interval.

### Departures and decisions

- The pass line is for **workspace passes only**, as the plan's wording and Phase 4's handoff say.
  Startup keeps its single info line.
- `trigger_registry_wait_ms` is **omitted** for a request that never parked, matching how stage
  fields are omitted. `waited_for_trigger_registry` stays as before.
- `fresh` includes a probe outside every trigger boundary. `failed` includes a probe whose folder
  context could not be built. The docs state both, so `prepared + fresh + failed = probes`.
- No library (`darkmatter/lib`) code changed: Phase 2's `PayloadMemoTally` was enough.

### Requirement-to-test mapping

All tests are L1. The F2 tests are in-process LSP sessions in the new
`dmls/tests/l1/registry_observability.rs`; the F1 tests are unit tests in `request_timing::tests`.

| Requirement | Test |
| --- | --- |
| Pass-local counts exclude an interrupting document job. A payload edit not reported to the server makes the job miss while the pass is held before its second probe, and the request is answered during the hold. The line reads `probes=2 prepared=2 fresh=0 failed=0 payload_hits=2 payload_misses=0` | F2 `the_pass_line_excludes_a_document_job_that_interrupts_it`. Mutation check: with `run_document`'s tally removed, it fails with `payload_misses=1` |
| A fresh scope records neither a hit nor a miss (`prepared=1 fresh=1 payload_hits=0 payload_misses=1`) | F2 `a_fresh_probe_records_neither_a_hit_nor_a_miss` |
| Failed probes are counted separately, and a failed resolution is a miss (`prepared=1 failed=1 payload_hits=1 payload_misses=1`) | F2 `a_failed_probe_is_counted_apart_from_prepared_and_fresh` |
| Log hygiene: the line has exactly the pass fields, every value but `workspace` is numeric, and no payload property, trigger text, or document text appears | the three tests above (`only_pass_counts`, `assert_no_schema_text`) |
| Resume plus requeue: parked 100 ms, the context changes, the request is parked again for 50 ms, and its one event shows `elapsed_ms=150 trigger_registry_wait_ms=150` | F2 `a_requeued_request_reports_the_sum_of_its_parked_intervals` |
| Cancellation: `outcome="cancelled" cancelled=true trigger_registry_wait_ms=70`; time after the answer is not counted | F2 `a_cancelled_request_reports_its_parked_time` |
| Shutdown: answered once with `ServerCancelled`, `trigger_registry_wait_ms=30` | F2 `a_request_parked_at_shutdown_reports_its_parked_time` |
| A request that never parks has no wait field | F2 `a_request_that_never_parks_has_no_wait_figure`, F1 `a_message_that_never_parked_omits_the_wait_figure` |
| Summing excludes time between intervals (handler work, index waits); an open interval counts up to now; a repeated begin or end is a no-op | F1 `registry_wait_sums_parked_intervals_and_excludes_the_time_between_them`, `a_second_begin_keeps_the_interval_in_progress_and_an_end_without_one_does_nothing` |

The F2 wait tests run on a `ManualClock` through the new fixture seam
`LspFixture::start_with_registry_gate_and_clock`. The clock moves only when a test advances it, so
every figure is exact.

Input Robustness Matrix: not applicable (no parser or loader).

### Docs and skill

- `darkmatter/docs/lsp/indexing.md`: a log-catalog row for `trigger registries prepared`, a new "The
  workspace pass line" subsection (sample, field table, exclusion rule, hygiene), the
  `trigger_registry_wait_ms` field row, and an updated hover sample with an explanation of what the
  wait covers. Without these, the docs would not have mentioned either new behavior.
- `.claude/skills/darkmatter/dmls.md`: the wait methods, the tally placement, and the new test file
  and fixture seam.
- Doc comments: `request_timing` module docs (the new field), `prepare_workspace_probe` (the
  `Failed` outcome), and `run_pass` (it now returns a `RegistryPass`). No drifted comment was found.

### Gates

- Flakiness: the first run of the interruption test timed out once, because the probe was released
  before dispatch queued the document job. A `round_trip()` before the release fixed it: the job is
  now always queued first. After the fix, the new binary passed 6 runs in a row.
- `git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`
  → `packages: ["dmls", "test-toolkit", "darkmatter"]`, `change_class: package`. The gates were run.
- `just lint` (darkmatter): pass, no warnings.
- `just test` (darkmatter): **10533 passed, 10 skipped** (61 s). That is 10523 plus 10 new tests:
  7 L1 and 3 unit. No failures.
- `just check-tier-coverage darkmatter`: stranded 0.
- Dependents: no workspace package depends on `dmls`, and no `darkmatter` library code changed.
- Cross-OS: not run. The change is counters, clock arithmetic, and logging, with no path, process,
  or console handling.
- Pre-existing failures: none seen.

## Phase 6

### Starting state

The working tree held Phase 5's changes uncommitted (the last commit is the Phase 4 close),
so `git status` shows Phase 5's files as well as this phase's. This phase changed no source
file. It edited `darkmatter/docs/lsp/indexing.md`, `.claude/skills/darkmatter/dmls.md`, and
this fix's plan, log, and spec.

### Docs and skill (Wave 11)

Phases 3–5 had already documented most of the behavior, so this wave reviewed those pages
and filled the gaps.

- `darkmatter/docs/lsp/indexing.md`
    - Already covered: job ordering, workspace-job merging, the file list installed before
      registry-ready (with its startup Mermaid sequence), when a config reload invalidates
      (the "When the registry changes" table), and the R5 log and timing fields.
    - **Added** the Mermaid sequence "a request on a document opened while a workspace pass
      runs waits only for the scope in progress" under *Preparing registries off the dispatch
      thread*.
    - **Added** the subsection *Resolved payloads are shared*: the memo's inputs, an example
      of two package folders sharing one resolution, what forces a new resolution, the
      32-entry LRU, failures not kept, and the link to the pass line's
      `payload_hits`/`payload_misses`. No page described the memo before, so nothing had
      drifted.
- `darkmatter/docs/lsp/configuration.md`: reviewed, no change. *Changes apply without a
  restart* already says that an unchanged effective reload drops only the contexts and
  re-publishes diagnostics, and that any effective change marks every registry stale.
- `.claude/skills/darkmatter/dmls.md`: the worker priority rule was already present (Phase 3/4).
  **Added** a payload-memo bullet: the key fields, the fingerprint, why the requester's context
  must stay out of the key, and the tests' reliance on nextest's process-per-test model.
- Doc comments reviewed against the code: `file_payload_memo` (assemble.rs), `update_config` /
  `reload_config` (whose docs carry the contract), `spawn_startup`, `spawn_registry_worker`,
  `RegistryPass`, and the `registry_jobs` module docs. All match the code. **No drift found**,
  so no comment changed.
- README: not changed. Public behavior (CLI and LSP surface) is the same; the work only makes
  existing behavior faster.

### Consumers (Wave 12)

`git diff df86e5163 -- darkmatter/lib/src` (the fix's base) adds two public items:
`PayloadMemoTally` and `PayloadMemoCounts` (start/counts). It removes or changes no public
item, so no consumer can break. Only `dmls` uses the new items (a repository-wide grep found
none in `claudine` or `messenger`). No workspace package depends on `dmls`. Phase 2 already ran the
targeted dependent tests (claudine 318, claudine-cli 57, messenger 59), and this phase changed
no library code, so no dependent tests were rerun.

### After sample

Same host, same harness (`/tmp/dmls-repro/repro.py`, not committed), same document, binary
from `cargo build --locked --release -p dmls` at this phase's tree. Load average about 14–20
(not quiet; the before sample was taken at about 17).

| After `initialized` | Before (run 1 / 2) | After (run 1 / 2) | `slow request` after | `schema registry ready elapsed_ms` before → after |
| --- | --- | --- | --- | --- |
| nothing | 207 / 161 ms | 183 / 133 ms | none | 1044 / 998 → 310 / 273 |
| `didChangeConfiguration {"settings": {}}` | 1260 / 1334 ms | 178 / 192 ms | none | 1151 / 1222 → 322 / 314 |

The incident is fixed. The unchanged notification no longer adds anything (178–192 ms against
133–183 ms without it), and both variants are under the non-gating ≤ 250 ms target.

**Remaining wait and its cause.** A debug-level run of the configuration variant shows the server's
own completion event: `elapsed_ms=22 … waited_for_trigger_registry=true
trigger_registry_wait_ms=20`. The request parks only on its document's own job, 20 ms. The rest
of the client-measured ~180 ms is spent before the server reads the request. It is
present in both variants, so the configuration notification does not cause it. It is
dispatch work queued ahead of the request (the `initialized`/`didOpen` handling and the
diagnostics publish for the opened document). It is not trigger-registry work, and it is
out of scope for this fix.

**Criterion 2 (payload misses).** Startup phases do not log the pass line (Phase 5 ruling). To
count the cold scan, a **temporary, uncommitted** patch made `run_startup` call
`log_workspace_pass`, the same technique as the spec's original timer. A second script
(`/tmp/dmls-repro/forced.py`) then sent an effective config change
(`diagnostics.debounce_ms: 301`), which marks every registry stale and queues one workspace
pass. Results on this repository:

| Pass | Line |
| --- | --- |
| cold startup, initial phase | `probes=111 prepared=111 fresh=0 failed=0 payload_hits=15 payload_misses=3 elapsed_ms=336` |
| startup, discovered phase | `probes=111 prepared=0 fresh=111 failed=0 payload_hits=0 payload_misses=0 elapsed_ms=5` |
| forced workspace pass (unchanged inputs) | `probes=111 prepared=111 fresh=0 failed=0 payload_hits=18 payload_misses=0 elapsed_ms=66` |

The cold scan records 3 misses (one per Claudine trigger payload, down from 18–32), and the forced
second scan records 0. The patch was reverted (`router.rs` restored from a copy, no
`TEMP-startup` string in the source or the rebuilt binary).

### Done-when: requirement-to-test mapping (all phases)

| Done-when / acceptance | Evidence |
| --- | --- |
| 1, 3: unchanged notification adds no pass, no invalidation, zero extra preparations | `registry_preparation::an_unchanged_configuration_notification_prepares_nothing`, `repository_contexts::an_unchanged_configuration_drops_every_context_and_prepares_no_workspace_scope`, `workspace_schema_scopes::an_unchanged_configuration_prepares_an_open_document_whose_context_changed`, `startup_preparation::a_request_overtakes_the_remaining_startup_probes_after_an_unchanged_configuration` (with its control session); after sample above |
| 2: 3 cold misses, 0 on a forced rescan | this phase's measurement above; library equivalent `payload_memo_tests::eighteen_distinct_keys_fit_and_a_repeat_scan_resolves_nothing` and the sharing/missing rows in Phase 2 |
| 4: merging, one follow-up (incl. the completion/dispatch boundary), documents before remaining probes | `registry_preparation::a_queued_document_runs_before_a_merged_workspace_pass`, `…::schedules_during_a_running_pass_add_one_follow_up`, `…::a_document_job_runs_between_two_probes_of_a_running_pass`; F1 `registry_jobs::tests::a_schedule_after_a_pass_ends_but_before_its_completion_is_handled_adds_one_pass` |
| 5: no concurrent preparations; a change during preparation leaves the scope stale | `startup_preparation::startup_and_a_refresh_share_one_worker_and_skip_fresh_probes` (peak = 1), `…::a_change_during_an_in_flight_startup_preparation_leaves_the_scope_stale`; existing overlay change-replay and failure-transition tests unchanged |
| 6: registries, diagnostics, summaries identical on `darkmatter/tests/fixtures/schema-triggers/` | existing `cli/tests/l1/schema_triggers.rs`, `lib/tests/l1/shipped_schema_corpus.rs`, `schema_phase_validation.rs`, `inline_document_text.rs`, all passing unchanged in this phase's `just test` |
| R5 observability | `registry_observability.rs` (7 tests) and `request_timing::tests` (3), Phase 5 |

### Key field set (R1, final)

`file_payload_memo::Key` = trigger `source` path, `payload` YAML value, `schema_roots`, the
requester's tree (`base_dir` + `base_dir_origin`), `for_cwd(trigger folder)`, and
`for_trusted_external_cwd(root)` for each root. Reuse also requires an unchanged fingerprint
(dependency bytes plus the entry listing of each root and dependency folder). Capacity 32,
true LRU, successes only.

### Departures from the spec (all phases)

- The pass line is logged for workspace passes only. Startup keeps its single `schema registry
  ready` info line (Phase 5). Criterion 2's cold count was therefore measured with a temporary
  patch, as described above.
- `trigger_registry_wait_ms` is omitted for a request that never parked (Phase 5).
- The worker-completion/dispatch-boundary case of criterion 4 is covered at F1 only (Phase 3). The
  window cannot be held from outside without a seam inside the queue transition.

### Gates

- `git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`
  → `packages: ["dmls", "test-toolkit", "darkmatter"]`, `change_class: package`. That includes
  Phase 5's uncommitted files. The gates were run.
- `just lint` (darkmatter): pass, no warnings.
- `just test` (darkmatter): **10533 passed, 10 skipped** (33 s). That is unchanged from Phase 5,
  since this phase added no tests. No failures, no reruns.
- `just check-tier-coverage darkmatter`: stranded 0.
- `cargo clippy -p test-toolkit --all-targets -- -D warnings`: clean.
- Cross-OS: not run. This phase changed only docs. Earlier phases' cross-OS evidence is in their
  sections (Phase 2 ran Windows for the memo tests).
- Pre-existing failures: none seen.
