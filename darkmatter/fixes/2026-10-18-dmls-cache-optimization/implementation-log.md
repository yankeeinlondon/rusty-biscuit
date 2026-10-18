---
spec: /Volumes/coding/wt/rusty-biscuit/feat-pipeline/darkmatter/fixes/2026-10-18-dmls-cache-optimization/spec.md
plan: darkmatter/fixes/2026-10-18-dmls-cache-optimization/plan.md
implemented_by: claude/opus
started_phase: 1
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages: []
human_review: false
message_to_agent: |-
    Phase 1 found that plan ruling 1's memo key is incomplete. Nested delegations,
    imports, and examples resolve with original.for_cwd(D), where D is each
    nested file's own folder, not only source.parent(). Two originals can agree on
    for_cwd(source.parent()) yet diverge for another D when one has a boundary
    tree and the other a Fallback tree. Phase 2 must add the ORIGINAL context's
    tree to the key: original.base_dir() and original.base_dir_origin() (both
    existing public accessors; no new biscuit-file API). The full key and the
    case analysis are in the log under "Memo inputs". Resolution never reads the
    requesting context except through for_cwd / for_trusted_external_cwd, so
    nothing else is needed. The log also lists the dmls tests that Phases 3-4
    must update: (a) schedule/pass counters, (b) the registry-before-file-list
    order that initialize() relies on, (c) tests that send an unchanged
    {settings:{}} and expect invalidation. Baseline just test: 10482 passed.
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
