---
spec: "/Volumes/coding/wt/rusty-biscuit/feat-pipeline/darkmatter/fixes/2026-10-18-dmls-cache-optimization-follow-ups/spec.md"
plan: "darkmatter/fixes/2026-10-18-dmls-cache-optimization-follow-ups/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
source_files_during_phase_1:
    - biscuit-file/lib/src/file_reference/mod.rs
    - biscuit-file/lib/src/file_reference/resolve.rs
    - biscuit-file/lib/tests/l1/detailed_resolution.rs
    - darkmatter/lib/src/markdown/schemas/resolve.rs
    - darkmatter/lib/src/markdown/schemas/resolve/searched_tests.rs
    - darkmatter/lib/src/markdown/schemas/triggers/assemble.rs
    - darkmatter/lib/src/markdown/schemas/triggers/discovery.rs
    - darkmatter/lib/src/markdown/schemas/triggers/payload_memo_tests.rs
docs_updated_during_phase_1:
    - biscuit-file/docs/topics/file-references.md
docs_created_during_phase_1: []
skills_files_updated_during_phase_1:
    - .claude/skills/biscuit-file/references/file-references.md
source_files_during_phase_2:
    - darkmatter/lib/src/markdown/schemas/triggers/assemble.rs
    - darkmatter/lib/src/markdown/schemas/triggers/payload_memo_tests.rs
docs_updated_during_phase_2:
    - darkmatter/docs/lsp/indexing.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
    - .claude/skills/darkmatter/dmls.md
source_files_during_phase_3:
    - darkmatter/dmls/src/overlay/registry.rs
    - darkmatter/dmls/src/overlay/mod.rs
    - darkmatter/dmls/src/router.rs
    - darkmatter/dmls/src/workspace/watch.rs
    - darkmatter/dmls/tests/l1/main.rs
    - darkmatter/dmls/tests/l1/searched_candidates.rs
docs_updated_during_phase_3:
    - darkmatter/docs/lsp/indexing.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
    - .claude/skills/darkmatter/dmls.md
packages:
    - darkmatter
    - biscuit-file
    - dmls
human_review: false
source_files_during_phase_4:
    - darkmatter/lib/src/markdown/schemas/resolve.rs
docs_updated_during_phase_4:
    - darkmatter/docs/lsp/indexing.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
    - .claude/skills/darkmatter/schema.md
source_code:
    - biscuit-file/lib/src/file_reference/mod.rs
    - biscuit-file/lib/src/file_reference/resolve.rs
    - biscuit-file/lib/tests/l1/detailed_resolution.rs
    - darkmatter/lib/src/markdown/schemas/resolve.rs
    - darkmatter/lib/src/markdown/schemas/resolve/searched_tests.rs
    - darkmatter/lib/src/markdown/schemas/triggers/assemble.rs
    - darkmatter/lib/src/markdown/schemas/triggers/discovery.rs
    - darkmatter/lib/src/markdown/schemas/triggers/payload_memo_tests.rs
    - darkmatter/dmls/src/overlay/registry.rs
    - darkmatter/dmls/src/overlay/mod.rs
    - darkmatter/dmls/src/router.rs
    - darkmatter/dmls/src/workspace/watch.rs
    - darkmatter/dmls/tests/l1/main.rs
    - darkmatter/dmls/tests/l1/searched_candidates.rs
documentation:
    - biscuit-file/docs/topics/file-references.md
    - darkmatter/docs/lsp/indexing.md
    - .claude/skills/biscuit-file/references/file-references.md
    - .claude/skills/darkmatter/dmls.md
    - .claude/skills/darkmatter/schema.md
completed_phase: 4
implemented: true
---

# Implementation Log for 2026-10-18-dmls-cache-optimization-follow-ups (4 phases)

## Phase 1

### What changed

- **`ResolvedSchema::searched`** (`lib/src/markdown/schemas/resolve.rs`): a new
  `Vec<PathBuf>` beside `imports` / `examples` / `referenced_files`. It holds
  every location probed while selecting a schema file, named-type import, or
  example file, up to and including the selected one. It is sorted (via
  `BTreeSet`), deduplicated, lexical (absent paths have no canonical form),
  and filled after evaluation, so the candidate order is never changed. The
  existing dependency lists are untouched (R2).
- **One recording point per probe kind (R5):**
  `resolve_file_reference_in_context` now calls `resolve_detailed` on the same
  `for_cwd(base_dir)` context, records `probed_paths()`, then projects with
  `into_convenience()`. That is the exact body of `resolve_in_context`, so the
  error and `NoMatch` mapping is identical. `try_bare_name_in_roots` does the
  same per root on `for_trusted_external_cwd(root)`.
- **Threading:** a `searched: &mut BTreeSet<PathBuf>` sink goes through
  `expand_document_imports` / `expand_imports` (the `ImportEngine` gets a
  `searched` field, merged out at the end), `named_type_file_path`,
  `rehome_examples` / `rehome_atom_examples` (now `&mut self`),
  `resolve_document_examples` / `resolve_examples_in_json` /
  `resolve_one_example` / `resolve_example_path`. Delegation hops merge
  through `accumulate_searched` (the `accumulate_referenced_file` analogue);
  both root-union resolvers merge string arms and pass their own sink to
  inline arms.
- **Discarded sinks:** the public `named_type_reference_path` (editor
  go-to-definition) and `expand_named_types` (editor expansion) pass a
  throwaway set, because their signatures return no `ResolvedSchema`.
  Embedded expansion passes a throwaway set too, since it probes no disk.
- **Propagation (Wave 1.2):** `ResolvedPayload.searched` is copied from
  `resolved.searched` in `resolve_file_payload` (empty for embedded payloads),
  and `TriggerRegistry::searched_locations()` sits next to `dependencies()`
  in `discovery.rs`. The memo `Entry` clones the whole `ResolvedPayload`, so
  a hit reports the same set (tested).

### Departures from the plan

- **biscuit-file gained `DetailedResolution::probed_paths()`** (plan listed
  only `darkmatter`). The probed candidate paths are not normalized: they can
  be relative (resolved against `cwd`) or contain `.`/`..` (`./name`,
  `../x`). biscuit-file's lexical normalizer (`normalize_absolute`) was
  crate-private. Writing a third ad-hoc normalizer in darkmatter (there is
  already `effects::fs_write::lexically_clean`) would split the spelling rule.
  Instead `probed_paths()` yields each non-`SearchRoot` candidate through the
  same `normalize_absolute` used for `matched_path()`, so the selected
  candidate's entry equals the matched path exactly. `normalize_absolute`
  became `pub(crate)`. This is additive: no existing behavior changes.
  Recursive `%` search roots are omitted, which implements R1 at the source.
- **R6 needed no cache-carry code.** `ImportEngine::resolve_namespace`
  resolves the reference (and so probes) *before* it consults
  `namespace_cache`, and a cached namespace's nested imports and examples are
  re-expanded at every use site. Every site therefore records its own probes.
  The `searched` field doc on `ImportEngine` states this invariant, and
  `second_import_of_a_cached_namespace_still_records_its_own_search` locks it.

### Requirement → test mapping

Boundary: library resolver → `ResolvedSchema` (public field) →
`TriggerRegistry::searched_locations()` (public). Fidelity F2 (real temp
filesystem and real `FileReference` resolution), tier L1. No CLI or terminal
boundary is crossed in this phase.

| Requirement | Test |
| --- | --- |
| Concrete example records absent `schemas/shared/types.yaml` and selected `shared/types.yaml`; dependency lists unchanged | `resolve::searched_tests::implicit_import_records_its_absent_nearer_candidate_and_the_selection` |
| Explicit `./` / `../` records one location (no fallback) | `…::explicit_relative_references_record_only_their_one_location`; biscuit-file `probed_paths_lists_every_candidate_of_a_no_match` |
| Bare name records each root tried, stops at selection | `…::bare_name_records_each_schema_root_tried_through_the_selection` |
| Delegation hop merge | `…::whole_file_delegation_merges_each_hop` |
| Root-union arm merge | `…::root_union_arms_merge_their_searches` |
| Example file nearer candidate | `…::example_file_records_its_absent_nearer_candidate` |
| Transitive import + rehomed example search from their own folder | `…::transitive_import_and_its_rehomed_example_are_recorded` |
| Namespace-cache hit still records (R6) | `…::second_import_of_a_cached_namespace_still_records_its_own_search` |
| Nearer candidate that is a directory is recorded (seed for the Phase 2 swap) | `…::nearer_candidate_that_is_a_directory_is_recorded_before_the_fallback` |
| Inline mapping imports | `…::inline_mapping_records_the_searches_of_its_imports` |
| Failed resolution still errors (R4) | `…::failed_resolution_still_errors_and_returns_no_partial_record` |
| Registry exposes searched apart from dependencies; memo hit returns the same set | `triggers::payload_memo_tests::a_registry_reports_searched_locations_apart_from_its_dependencies` |
| biscuit-file lexical normalization (`../` collapsed in an absent folder), selection is last | `biscuit-file l1 detailed_resolution::probed_paths_lists_missing_candidates_through_the_match_lexically` |
| Recursive search roots omitted (R1) | `biscuit-file l1 detailed_resolution::probed_paths_omits_recursive_search_roots` |

Targeted runs: `cargo nextest run -p darkmatter --lib -E
'test(payload_memo_tests) | test(searched_tests)'` → 30 passed;
`cargo nextest run -p biscuit-file -E 'test(probed_paths) | binary(l1) &
test(detailed_)'` → 17 passed. No existing `dependencies` assertion in
`payload_memo_tests.rs` was changed.

Input Robustness Matrix: not applicable (no file-format or configuration
reader added).

Docs: biscuit-file's `docs/topics/file-references.md` and the `biscuit-file`
skill reference list `probed_paths()`. Darkmatter's `docs/lsp/indexing.md`
and the darkmatter skill are scheduled for Phase 4 and are not touched here.
Phase 1 changes no observable darkmatter behavior: nothing reads `searched`
yet.

### Gates

- `just lint` (darkmatter): pass. `just test` (darkmatter): 10545 passed, 0
  failed, 10 skipped (pre-existing skips), first run, no reruns.
- `just test` (biscuit-file): 1147 passed plus the minimal-feature run; `just
  lint` (biscuit-file): pass.
- `just check-tier-coverage darkmatter` / `biscuit-file`: 0 stranded.
- Dependents: no package outside `darkmatter/` names `ResolvedSchema`,
  `searched_locations`, or `probed_paths`, and no struct literal of
  `ResolvedSchema` / `ResolvedPayload` exists outside their own files, so no
  dependent test exercises a changed symbol. `cargo check -p claudine --tests`
  (largest consumer) compiles clean. The biscuit-file change is purely
  additive.
- CI scope (`affected_scope.py` over the working tree): `packages:
  [biscuit-file, darkmatter, test-toolkit]`, `change_class: package`, so the
  gates were required and were run.
- Cross-OS (path normalization risk): `just cross-check darkmatter --os
  windows searched_tests payload_memo_tests` → 30 passed;
  `just cross-check biscuit-file --os windows probed_paths detailed_` → pass.
  The first attempt with `-E '...(...)'` failed before running anything
  (just re-splits args); positional filters work.
- No load-sensitive or pre-existing failures observed.

Phase 1 status: implementation complete, ready for review.

## Phase 2

### What changed

- **`file_payload_memo::inputs_fingerprint`**
  (`lib/src/markdown/schemas/triggers/assemble.rs`) now takes the whole
  `ResolvedPayload` and returns `Option<u64>`. On top of the dependency bytes
  and schema-root / dependency-folder listings it folds in:
  - the parent folder of every `searched` candidate, in the same sorted,
    deduplicated folder list (so an absent `schemas/shared/` is observed as
    missing, with a marker distinct from an empty folder's listing);
  - a per-candidate kind observation (`std::fs::metadata`: absent / file /
    directory / other), so a directory → file swap under the same name, which
    leaves every listing unchanged, still misses.
- **Expected absence vs failed observation.** `NotFound` and `NotADirectory`
  (a path component is a file; `ENOTDIR` / Windows `ERROR_DIRECTORY`) are
  expected observations with markers. Any other error, including one failed
  entry inside an otherwise readable listing (previously filtered out
  silently), makes the fingerprint `None`: a held entry is not reused, and a
  fresh resolution is returned but not stored (the stale entry for the key is
  still dropped). Previously an unreadable root or dependency hashed as
  "absent" and could validate a hit.
- **I/O stays outside the mutex** (both fingerprint calls precede the lock).
  Key, capacity (32), LRU, and `Arc` identity on hits are unchanged.
- **Test seam.** No filesystem-failure seam existed. The fingerprint's three
  reads go through a private `observe` module; under `#[cfg(test)]` a
  thread-local `fail_observing(path)` guard makes that one path's reads fail.
  This avoids permission-bit tests (different under elevated users and native
  Windows). Not compiled into release builds.
- **Comments.** The `file_payload_memo` doc's claim that "the schema roots and
  each dependency's folder" are every searched directory is replaced with the
  real coverage, including the recursive `%` limitation (R1) and the
  failed-observation rule.
- **Docs.** `docs/lsp/indexing.md` "Resolved payloads are shared" reuse
  paragraph described only the old rule; rewritten to the current one with the
  concrete example. The DMLS registry-change table and the fuller rewrite
  (Mermaid, watch advance) remain for Phases 3/4.

### Finding: which cases were actually stale in the library

Proving the tests against the old fingerprint (searched ignored) showed the
spec's concrete example was **not** stale in the library alone: creating
`schemas/shared/` adds an entry to the `schemas/` schema-root listing that
was already hashed. The library missed the case when the nearer candidate's
folder appears under a folder that is neither a schema root nor a dependency
folder: an existing `schemas/shared/` holding no dependency, a nested absent
folder (`shared/deep/...`), or a directory → file swap. The precedence rows
therefore use `shared/deep/...` under a pre-existing `shared/` folder so each
is load-bearing; the concrete example stays as the spec's acceptance-1 test.
In DMLS the concrete example is still stale (Phase 3).

### Requirement → test mapping

Boundary: library trigger discovery (`scan`) → process-wide payload memo,
observed through the public `TriggerRegistry` and `PayloadMemoTally`.
Fidelity F2 (real temp filesystem, real resolver), tier L1, in
`triggers::payload_memo_tests` (compiled by the `darkmatter` lib test
target; no tier marker). No CLI/terminal boundary. "Fails before" means the
test fails with the fingerprint's `searched` folding disabled.

| Requirement | Test | Fails before |
| --- | --- | --- |
| Acc. 1 concrete example: hit + same `Arc`; create nearer (+ folder) → miss, `number` fields, new dependency; delete → fallback | `a_nearer_import_candidate_created_in_an_absent_folder_is_resolved_again` | no (root listing covers it; see finding) |
| Acc. 2 payload path | `a_nearer_payload_path_is_resolved_again` | yes |
| Acc. 2 direct import (nested absent folder) | `a_nearer_import_is_resolved_again` | yes |
| Acc. 2 transitive import | `a_nearer_transitive_import_is_resolved_again` | yes |
| Acc. 2 example file | `a_nearer_example_file_is_resolved_again` | yes |
| Acc. 2 bare-name control (kept) | `a_file_added_to_a_schema_root_is_resolved_again` | n/a |
| Acc. 2 explicit `./` has no fallback; failure not kept; creating the named file recovers | `an_explicit_relative_reference_gains_no_fallback` | n/a (behavior lock) |
| Acc. 3 existing candidate parent | `a_nearer_candidate_in_an_existing_folder_is_resolved_again` | yes |
| Acc. 3 directory → file, same entry name | `a_nearer_candidate_swapped_from_directory_to_file_is_resolved_again` | yes |
| Acc. 3 failed observation is not a hit and is not kept; recovers after | `a_failed_observation_is_never_a_hit` | yes |
| Acc. 6 unrelated files / content edits keep the hit and `Arc` | `unrelated_edits_keep_the_held_payload` | n/a |
| Acc. 6 isolation, capacity, LRU, failed-resolution, tally | existing `payload_memo_tests` (unchanged, green) | n/a |

Counts via `PayloadMemoTally`; no sleeps, no global cache clears. Input
Robustness Matrix: not applicable (no file-format or configuration reader).

Targeted run: `cargo nextest run -p darkmatter --lib -E
'test(payload_memo_tests)'` → 29 passed.

### Gates

- `just lint` (darkmatter): pass (exit 0, no warnings).
- `just test` (darkmatter): 10555 passed, 0 failed, 10 skipped (pre-existing
  skips), first run, no reruns, no load-sensitive failures.
- `just check-tier-coverage darkmatter`: 0 stranded.
- CI scope: `git status --porcelain --untracked-files=all | awk '{print $NF}'
  | xargs python3 scripts/ci/affected_scope.py` (excluding an unrelated
  `worktree/fixes/2026-10-18-remove-observation-under-load/spec.md` edit made
  by another session in this worktree, not part of this phase) → `packages:
  [darkmatter, test-toolkit]`, `change_class: package`, so the gates were
  required and were run.
- Cross-OS (`NotFound` / `NotADirectory` mapping is OS-specific):
  `just cross-check darkmatter --os windows payload_memo_tests` → 29 passed.
- Dependents: no public symbol changed (`fail_observing` is `#[cfg(test)]`);
  the behavior change only narrows reuse. Claudine consumes trigger scanning,
  so its schema tests ran: `cargo nextest run -p claudine -E
  'test(claudine_schemas)'` → 174 passed (7 flagged slow by nextest; not
  touched by this phase).
- `.claude/skills/darkmatter/dmls.md` payload-memo bullet updated with the
  fingerprint coverage, the failed-observation rule, the test seam, and the
  "nearer candidate must be outside observed listings" testing trap.

Phase 2 status: implementation complete, ready for review.

## Phase 3

### What changed

- **`Dependencies`** (`dmls/src/overlay/registry.rs`) gained `searched`
  (every `TriggerRegistry::searched_locations()` candidate, through
  `normalized`, sorted, deduplicated) and `absent_bases` (absent root folders
  and absent candidate folders, each with its nearest existing ancestor).
  Both are filled in `Preparation::run` (`Dependencies::record_searched` /
  `record_absence`), the only place allowed to do I/O; the absence check is a
  `Path::is_dir` per distinct folder, bounded by the recorded candidates.
- **`scope_affected`**: after the existing byte comparison for known files
  (unchanged, still first), a Ready scope is affected when the event path is
  a searched candidate or a folder above one (`candidate.starts_with(path)`).
  That covers creation, deletion, replacement, and a directory-only event
  (R8). Unrelated events (same file name in folders nothing searched) do not
  affect it. I did not add the plan's "parent equals a candidate's parent"
  rule: every create/delete/rename of a candidate reports the candidate path
  itself, and a parent rule would invalidate on any sibling file, which the
  spec asks to avoid.
- **R7**: `begin` copies the replaced scope's `searched` and `folders` into
  `Preparation::carried`; on a failed scan `run` re-records them (so absence
  is re-observed off the lock: a now-present folder is watched as present).
  Chained failures keep carrying. Last-good and root-failure rules in
  `install` are untouched (they key on `installed`, not on `deps`).
- **Watch registration** (`dmls/src/workspace/watch.rs`): new
  `WatchedFolder { folder, absent_base }`. `schema_watch_registration` takes
  them: a present folder keeps the old `<parent>` + `<name>/*` watch; an
  absent folder is watched from `absent_base` with `<rel>` and `<rel>/*`
  (each segment glob-escaped). `TriggerRegistries::watch_folders()` (and
  `OverlayState::trigger_watch_folders`) now return `BTreeSet<WatchedFolder>`
  and include candidate folders; the router wraps Markdown dependency folders
  as present. Because `absent_base` is part of the set, a folder appearing
  changes the set and `sync_schema_watches` re-registers (coverage advance),
  with no I/O in the sync itself. Absent schema roots now also get the
  absent form (base = nearest existing ancestor, folder + children); that is
  a small, consistent widening of their watch.
- **Change replay**: needed no new code; `install` replays `Change::Path`
  through `scope_affected`, so the new rule applies there too (tested).
- **Fallback modes**: no code change. A server rescan already marks every
  registry stale, and the library memo (Phase 2) then re-resolves; a YAML
  `didSave` goes through `invalidate_trigger_inputs` and so the same
  candidate rule (tested). No external-change promise without client events.
- **Docs** (`docs/lsp/indexing.md`): registry-change table row, the
  "Each preparation records" paragraph (searched locations, R7, the concrete
  example, the recursive `%` limitation), and "Watching schema folders"
  (absent-folder watch and its advance). Phase 4 still owns the Mermaid
  diagram and the full sweep.
- **Skill** (`.claude/skills/darkmatter/dmls.md`): watch-registration and
  trigger-registry bullets.

### Finding

At the DMLS layer the concrete example's *file* event
(`schemas/shared/types.yaml`) was stale before this phase (its parent is
neither a root nor a dependency folder), but a *directory-only* event for
`schemas/shared` was already covered, since `schemas/` holds the payload and
is a dependency folder. The directory-event test therefore uses a nested
absent folder (`schemas/shared/deep/` under an existing `schemas/shared/`).

### Requirement → test mapping

Boundaries: registry cache (in-process, F2, real temp filesystem and real
resolver) and the LSP protocol (F3: the in-process `LspFixture` server over
`lsp_server` messages, real temp repository). Tier L1. "Fails before" =
fails with the candidate rule (and, for R7, the carry) disabled.

| Requirement | Test | Fails before |
| --- | --- | --- |
| Acc. 4 concrete example: ready → create nearer → event → reselect; absent folder watched from nearest ancestor; coverage advances; removal falls back | `overlay::registry::tests::a_nearer_candidate_created_in_an_absent_folder_invalidates_a_ready_registry` | yes |
| R8 directory-only event above a candidate (nested absent folder) | `…::a_directory_event_above_a_candidate_invalidates_a_ready_registry` | yes |
| Acc. 5 replay (registry level) | `…::a_candidate_created_during_a_preparation_leaves_its_result_stale` | yes |
| Acc. 5 malformed nearer → last-good, watch kept (R7), repeat failure keeps it, repair recovers | `…::a_malformed_nearer_candidate_keeps_last_good_and_its_watch_until_repaired` | yes |
| Acc. 6 unrelated events keep fresh, zero work | `…::unrelated_events_keep_a_ready_registry_fresh_and_do_no_work` | n/a (control) |
| Watch unit: absent nested folder → nearest ancestor, folder + children patterns, escaping; present form | `workspace::watch::tests::test_an_absent_folder_is_watched_from_its_nearest_existing_ancestor` (+ updated `test_schema_watch_registration_roots_each_folder_at_its_parent`) | n/a (new API) |
| Acc. 4 protocol: watched create → `/model` type mismatch appears; registration has `shared` + `shared/*` at `schemas/`, then re-registered without `shared`; watched delete → fallback | `searched_candidates::a_watched_nearer_candidate_reselects_the_import_and_its_removal_falls_back` | yes |
| Acc. 4 save notification | `searched_candidates::a_saved_nearer_candidate_reselects_the_import` | yes |
| Acc. 4 fallback (server-rescan) mode | `searched_candidates::a_fallback_rescan_rebuilds_from_the_new_candidate` | no (rescan already invalidates all; locks the guarantee) |
| Acc. 5 protocol: startup probe held after reading, malformed candidate + event, replay → trigger `dm.schema.prepare` published; repair clears it | `searched_candidates::a_candidate_created_during_an_in_flight_preparation_is_prepared_again_and_recovers` | yes |
| Acc. 6 protocol: unrelated events + ready hovers do 0 registry scans / enumerations | `searched_candidates::unrelated_events_and_ready_requests_prepare_nothing` | n/a (control) |

`searched_candidates` is declared in `tests/l1/main.rs` (no tier marker in
its path). Existing registry, watch, `referenced_targets`, and
`workspace_file_list` registration tests stay green. Input Robustness
Matrix: not applicable (no file-format or configuration reader).

Targeted runs: `cargo nextest run -p dmls --lib -E 'test(registry::tests) |
test(watch::tests)'` → 36 passed; `cargo nextest run -p dmls --test l1 -E
'test(searched_candidates)'` → 5 passed (each under 0.1 s).

### Gates

- `just lint` (darkmatter): pass (exit 0, no warnings).
- `just test` (darkmatter): 10566 passed, 0 failed, 10 skipped (pre-existing
  skips), first run, no reruns, no load-sensitive failures.
- `just check-tier-coverage darkmatter`: 0 stranded.
- CI scope: `git status --porcelain --untracked-files=all | awk '{print $NF}'
  | xargs python3 scripts/ci/affected_scope.py` → `packages: [darkmatter,
  dmls, test-toolkit]`, `change_class: package`, so the gates were required
  and were run. (The working tree also holds Phase 2's uncommitted library
  edits and an unrelated `worktree/` spec edit from another session.)
- Cross-OS (path comparison via `starts_with` and canonical prefixes):
  `just cross-check dmls --os windows searched_candidates registry::tests
  watch::tests` → 41 passed.
- Dependents: the changed public items (`WatchedFolder`, `watch_folders`,
  `trigger_watch_folders`, `schema_watch_registration`) are used only inside
  `dmls` (`git grep`); `zed-dmls` compiles in `just lint` and its tests ran
  in `just test`. No library symbol changed in this phase.

Phase 3 status: implementation complete, ready for review.

## Phase 4

### What changed

- `darkmatter/docs/lsp/indexing.md`: re-checked the payload-sharing, registry-change, and
  "Watching schema folders" sections Phase 3 updated against the code
  (`file_payload_memo::inputs_fingerprint`, `Dependencies`, `schema_watch_registration`);
  they were current: searched locations, the absent-folder watch from its nearest existing ancestor
  and its re-registration, fallback-save and server-rescan limits, and the recursive `%` limitation
  (R1). Added the missing Mermaid flowchart (probe → record → fingerprint / watch →
  event → stale → prepare again → nearer file selected) after the concrete
  `Fields@shared/types.yaml` example. No link to this fix or its spec.
- `.claude/skills/darkmatter/schema.md`: the imports bullet now says dependencies are canonical,
  hashed files and `ResolvedSchema::searched` is a separate lexical list (possibly absent; `%` adds
  nothing) that propagates into `ResolvedPayload` and is never hashed as a file. `dmls.md` was
  already current from Phase 3.
- Comment sweep: `ResolvedSchema::searched`, `ResolvedPayload::searched`, the `file_payload_memo`
  module doc and `inputs_fingerprint`, `TriggerRegistry::searched_locations`, the `registry.rs` module
  header, `Dependencies`, and the watch docs all match the code.
  **Drift found and fixed** (code taken as correct): `ResolvedSchema::imports` and
  `ResolvedSchema::examples` in `lib/src/markdown/schemas/resolve.rs` said "the invalidation wiring
  itself is deferred", but both lists are consumed as dependency edges by
  `EffectiveSchema::dependencies` and by `ResolvedPayload::dependencies` (payload memo and DMLS
  registry). Reworded to state that a change invalidates a cached effective schema or resolved
  payload. Comment-only; no behavior change.
- Consumers of changed `pub` items: no struct literal of `ResolvedSchema` exists outside
  `resolve.rs` (`grep -rln 'ResolvedSchema {'`; the one test hit is a return type), and Claudine
  names neither `ResolvedSchema` nor `searched`. Nothing to fix.

### Requirement → test mapping

Phase 4 changes no behavior (documentation, skill, and doc comments only), so it adds no tests. The
acceptance behaviors the diagram and docs describe are covered by the Phase 1–3 tests listed
above (`payload_memo_tests`, `searched_tests`, `dmls` `searched_candidates`, registry and watch
unit tests), all of which ran green in this phase's `just test`.

### Gates

- `just lint` (darkmatter): pass.
- `just test` (darkmatter): 10566 passed, 0 failed, 10 skipped (pre-existing), first run, no reruns.
- `just check-tier-coverage darkmatter`: 0 stranded.
- CI scope: `git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3
  scripts/ci/affected_scope.py` → `packages: [darkmatter, test-toolkit]`, `change_class: package`,
  so the gates were required and were run.
- Dependents: the only source edit is a doc-comment rewording in `resolve.rs`, which cannot change a
  dependent's compile or behavior; dependent-package tests were not run for that reason.
- Cross-OS: not run; no cross-OS risk in a comment and docs change.

Phase 4 status: implementation complete, ready for review.
