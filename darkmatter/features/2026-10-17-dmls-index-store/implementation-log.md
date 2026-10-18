---
spec: "/Volumes/coding/wt/rusty-biscuit/feat-dmls-indexing/darkmatter/features/2026-10-17-dmls-index-store/spec.md"
plan: "darkmatter/features/2026-10-17-dmls-index-store/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
packages:
    - biscuit-file
    - dmls
source_files_during_phase_1:
    - darkmatter/dmls/Cargo.toml
    - Cargo.lock
    - biscuit-file/lib/src/file_reference/context.rs
    - biscuit-file/lib/src/file_reference/mod.rs
    - biscuit-file/lib/src/lib.rs
    - darkmatter/dmls/tests/common/mod.rs
    - darkmatter/dmls/tests/common/oracle.rs
    - darkmatter/dmls/tests/l1/main.rs
    - darkmatter/dmls/tests/l1/index_store_oracle.rs
    - darkmatter/dmls/tests/fixtures/index_store_corpus/
docs_updated_during_phase_1:
    - darkmatter/docs/dependencies.md
    - docs/dependencies.md
    - biscuit-file/docs/topics/file-references.md
    - biscuit-file/README.md
docs_created_during_phase_1: []
skills_files_updated_during_phase_1:
    - .claude/skills/biscuit-file/SKILL.md
    - .claude/skills/biscuit-file/references/api.md
    - .claude/skills/darkmatter/testing.md
human_review: false
message_to_agent: |-
  Phase 1 is done. Notes for Phase 2:
  - Rulings 2-18 in the plan were adopted as written (the plan has `yolo: true`; the author was not consulted). Ruling 8 (`WikiUniverse` as a required parameter of `build_with_roots`) is the first one code depends on.
  - The working tree still holds the dependency's (`2026-10-17-logging-improvements`) uncommitted review-cycle edits; they are not this feature's work.
  - The test-only oracle is `darkmatter/dmls/tests/common/oracle.rs` (`Oracle::build`, `Corpus::materialize`), corpus `darkmatter/dmls/tests/fixtures/index_store_corpus/`, self-check `tests/l1/index_store_oracle.rs`. When `build_with_roots` gains the `WikiUniverse` parameter, update the oracle's single call site and keep the self-check green; that self-check is the "wiki universe equivalence" baseline.
  - `WorkspaceGraph::wiki_docs()` has three production callers (`providers/wiki.rs` `file_completions` and `heading_target_doc`, `providers/rename.rs` `apply_file_rename`); ruling 8 says resolution and rename index the universe, so Phase 2 must decide each. The full removal inventory is in the Phase 1 log.
  - `Waiter` in `full_index.rs` is also used by `registry_jobs.rs`; it moves rather than being deleted in Phase 6.
  - `biscuit_file::user_cache_dir()` returns `None` on Windows when `LOCALAPPDATA` is unusable (no home fallback); treat `None` as memory mode. When dmls starts calling it (Phase 3), check darkmatter's context-construction guard tests for an ambient-read allowlist.
  - Ruling 16: record the exact `storage-strategy` skill line in the log once the store path and file name are fixed (Phase 3).
---

# Implementation Log for 2026-10-17-dmls-index-store (7 phases)

## Phase 1

### Dependency gate

- `2026-10-17-logging-improvements/plan.md`: 63 of 63 tasks checked, 0 open
- its implementation log records Phase 6 and "Validation checkpoint 6 (final)" is checked in its plan; the review-to-implement log (`log.md`) closes review cycle 2 with "open items: none"
- **gate met**
- note: at the start of this phase the working tree already held the dependency's uncommitted review-cycle edits (review-1 and review-2 repairs across `darkmatter/dmls/src/**`, its tests, and its docs, plus `registry_jobs.rs`, `referenced_targets.rs`, `registry_preparation.rs`, `workspace_schema_scopes.rs`). They are not this phase's work; the frontmatter lists only files this phase created or changed

### Rulings 2-18

- the plan marks rulings 2-18 as recommendations for the author to confirm before Wave 2. The plan carries `yolo: true` and this session is headless, so **they are adopted as written**; none was overridden. Nothing built in Phase 1 depends on them except ruling 16 (no repo copy of `storage-strategy`)
- ruling 7 (spec text corrections) returns to the author; this phase did not edit the spec text
- the author can still override any ruling before Phase 2 (ruling 8, the `WikiUniverse` shape, is the first one code depends on)

### What changed

- **Dependency add**: `rusqlite = { version = "0.31", features = ["bundled"] }` in `darkmatter/dmls/Cargo.toml` (same version and feature as `claudine/lib` and `biscuit-icon/lib`, so `Cargo.lock` gains no new `rusqlite`/`libsqlite3-sys` version). Listed in `darkmatter/docs/dependencies.md` (DMLS section) and `docs/dependencies.md` (dmls paragraph, manifest line, and the `rusqlite` catalog entry). `cargo check -p dmls` passes; the crate is not yet used in code
- **`biscuit_file::user_cache_dir()`** (`biscuit-file/lib/src/file_reference/context.rs`, re-exported from `file_reference/mod.rs` and `lib.rs`, under the `file-reference` feature like `home_dir`)
    - macOS `<home>/Library/Caches` (`XDG_CACHE_HOME` ignored); other Unix `$XDG_CACHE_HOME` when set, non-empty and absolute, else `<home>/.cache`; Windows `%LOCALAPPDATA%` when set, non-empty and absolute, else `None`
    - **decision:** no `<home>\AppData\Local` fallback on Windows; Windows sets `LOCALAPPDATA` for every loaded profile and a home-derived guess is wrong where the folder is redirected. `None` means "run without a persistent store" (R7's memory mode), which the store's failure ladder already handles
    - one pure helper `cache_dir_for(CacheOs, env, home)` with a private `CacheOs` enum chosen by `cfg!`, so every OS branch compiles and is tested on every host
    - no `dirs` dependency; biscuit-file's only source guard (`path_lookup_guard.rs`) covers `canonicalize`, not environment reads, so no guard entry was needed; darkmatter's context-construction guards are unaffected because nothing in darkmatter calls it yet (Phase 3 must check them when `dmls` starts calling it)
- **Oracle harness** (test-only): `darkmatter/dmls/tests/common/oracle.rs`, declared from `tests/common/mod.rs`
    - `Oracle::build(roots, snapshot)`: `ConfigState::load`, `discover_workspace`, a sequential `index_document` loop, `resolve_wiki_roots`, one `WorkspaceGraph::build_with_roots` with `RepositoryContexts` from the snapshot. It uses **none** of the functions Phase 6 deletes (`collect_indices*`, `index_workspace`, `parse_full_index_round`, `WorkspaceIndex::from_indices`, `bench`)
    - answers: `references`, `rename`, `will_rename_files`, `workspace_symbols`, through `ProviderRegistry::with_substrate()` and `providers::rename::*`, built the way the router builds `DocumentContext`/`RenameEnv` (overlay `None`: no references provider reads it)
    - normalized, `Ord` results (`OracleLocation`, `OracleEdit`, `OracleSymbol`) for later `assert_eq!(store, oracle)`
    - `Corpus::materialize(source)` copies the static corpus into a canonicalized tempdir and adds runtime-only parts: minimal `.git` in each root, a fixture home, `docs/absolute.md` (`file://` and absolute-path links), and the symlink `ws/linked -> ws/docs`
    - no production visibility change was needed
- **Shared corpus**: `darkmatter/dmls/tests/fixtures/index_store_corpus/` (21 static files; roots `ws`, nested `ws/nested`, `other`). Covers relative and bare links, root-relative bare paths, case and NFD differences, extension-elided and ambiguous wiki names, percent-encoded and `<...>` destinations, `file://` and absolute forms, directory links, `::file`/`::code`, a symlinked directory, two roots, a nested root, duplicate file names, and an anchor link. A table in the module docs maps files to cases
- **Oracle self-check**: `darkmatter/dmls/tests/l1/index_store_oracle.rs` (11 tests, hand-computed expectations)

### Current behavior the oracle records (not changed)

These are today's answers, recorded as expectations with a "current behavior" comment. The store path must reproduce them (acceptance 4, 5); whether any is a defect is outside this feature.

1. `my%20file.md` is not percent-decoded, so the link is unresolved (`<my file.md>` resolves)
2. `[[upper]]` does not resolve to `docs/Upper.md` (case), while NFD `[[café]]` resolves to the NFC-named file
3. an ambiguous wiki link (`[[topic]]`) counts as a referrer of every candidate; a cursor on it answers for the first candidate only
4. a link through the symlinked directory (`../linked/a.md`) is not a reference to `docs/a.md`; the symlinked directory is not listed (R-8 v1: symlinks not followed)
5. `/docs/a.md` and `sub/` (directory) are unresolved; `file://` links are external
6. case- and NFD-differing Markdown links are never references, on any OS
7. the root-relative bare `docs/a.md` **does** resolve, so the 18 S1 misses that ruling 2 fixes for R2 are present in the oracle's answers; Phase 3's key no-miss test can lean on this

### Removal inventory (for Phase 6)

No other workspace crate depends on `dmls` (`zed-dmls`/`zed-dmls-cli` only spawn binaries).

- **startup.rs**: `index_workspace` and `collect_indices` have **no production caller**; `collect_indices_from_files` is called only by `bench.rs:148`; `parse_full_index_round`/`FullIndexRound` by the router (`router.rs:71`, `:207-210`, `:1492`, `:1556`, `:1563`, `:1570`). `list_files` (called by `collect_indices`) is also used by the router (`router.rs:434`, `:632`) and **survives**. Test users of `index_workspace`: `startup.rs` tests, `tests/l1/entry_point_parity.rs:41,:408` (DMLS graph parity row), `tests/l1/level1_graph_index.rs:17,39,66,119`; these need the oracle (or `WorkspaceGraph::build_with_roots`) in Phase 6
- **invalidate.rs**: `from_indices` only via `index_workspace`; `begin_full_build` (`router.rs:1546`), `finish_full_build`/`FullBuildStep` (`router.rs:1577`, `:60`), `abandon_full_build` (`router.rs:1698`, `:1750`), `is_full` (`router.rs:706`)
- **full_index.rs**: `Trigger` (`router.rs:59`, `:1504`, `:1778`, logging near `:1640`, `:1692`; `for_method` is the only list of methods needing the full index); `Build`/`FullIndexState` (`router.rs:1288` and `:1488-1746`). **`Waiter` is shared with `registry_jobs.rs` (`:29`, `:65`, `:115`, `:135`, `:143`) and must move, not be deleted**
- **router**: `wait_for_full_index` `:1504`, `start_full_index_round` `:1541`, `finish_full_index_round` `:1570`, `log_full_index_built` `:1635`, `abandon_full_index` `:1745`, `index_is_full` `:705`, `cancel_full_index_waiter` `:1685` (from `$/cancelRequest` `:2262`); `Wait::FullIndex` in `request_timing.rs` (`:68`, `:106`, `:119`, `:144`, `:158`, `:177`, `:219`)
- **bench**: `bench_index`/`BenchReport` (`src/bench.rs`, re-exported `lib.rs:13,:34`), `--bench-index` in `src/main.rs` (`:28-175`), `darkmatter/justfile` `bench-dmls` (`:481-495`), docs `README.md:108`, `docs/editors/README.md:183`, `darkmatter/docs/lsp/indexing.md:601`. R14 replaces it
- **tests**: `tests/l1/full_index.rs` (32 tests), `tests/l1/startup_log_lines.rs:76,105,110`, `tests/common/mod.rs` `full_index_gate` hook (`:170-171`, `:236-249`, `:312-320`, `:394-410`, `:445`)
- **`WorkspaceGraph::wiki_docs()`**: production callers `providers/wiki.rs:290,301` (`file_completions`), `:327` (`heading_target_doc`), `providers/rename.rs:271` (`apply_file_rename`); all read whatever the snapshot holds. Under ruling 8 resolution and rename must index the `WikiUniverse`, so Phase 2 decides each of these three
- **`WorkspaceIndex` uses that survive** (open-document diagnostics, lazy per-document parse, referenced-target cache, file list): `new`, `set_wiki_roots`, `snapshot`, `generation`, `set_document`, `open_buffer`, `close_buffer` (non-`built` part), `install_listing`, `disk_changed`, `yaml_changed`, `disk_deleted`, `recheck_parsed`, `needs_parse`, `is_dependency`, `dependencies`, `unlisted_changed`, `recheck_dependencies`, `reresolve`, `ensure_parsed`, `prepare_targets`, `target_paths`, `listing`, `listed`, `listed_yaml`, `remove_document`, `relink`, `contains`, `len`, `is_empty`, `dependents_of`, `rebuild`
- **`WorkspaceIndex` parts removed in Phase 6**: `from_indices`, `begin_full_build`, `finish_full_build`, `abandon_full_build`, `is_full`, `unparsed_paths`, `touch`, fields `built` and `touched`, `FullBuildStep`, and the `self.built` branches in `close_buffer` (`:208`), `install_listing` (`:253`), `disk_changed` (`:280`), `recheck_parsed` (`:338`)

### Input robustness (Phase 1 slice)

- `user_cache_dir()` loader rows from the plan's matrix: control per OS, `XDG_CACHE_HOME` unset / empty / relative / absolute, home unavailable, plus `LOCALAPPDATA` unset / empty / relative / absolute and "XDG ignored on macOS and Windows". All tested in-process through the injected helper
- smell grep over the new `user_cache_dir` code (`unwrap_or_default\(\)|\.ok\(\)|filter_map|serde\(default`): **no matches**. `darkmatter/dmls/src/index_store` does not exist yet (Phase 3)

### Requirement-to-test mapping

| Behavior | Test |
| --- | --- |
| R7 `user_cache_dir`, macOS | `biscuit-file` `file_reference::context::tests::user_cache_dir::{macos_cache_dir_is_library_caches_under_home, macos_cache_dir_ignores_xdg_cache_home, macos_cache_dir_without_home_is_none}` |
| R7 `user_cache_dir`, other Unix | `…::{unix_cache_dir_prefers_absolute_xdg_cache_home, unix_cache_dir_without_xdg_is_dot_cache_under_home, unix_cache_dir_treats_empty_xdg_as_unset, unix_cache_dir_treats_relative_xdg_as_unset, unix_cache_dir_without_home_or_xdg_is_none, unix_cache_dir_without_home_uses_absolute_xdg}` |
| R7 `user_cache_dir`, Windows | `…::{windows_cache_dir_is_absolute_local_app_data, windows_cache_dir_without_usable_local_app_data_is_none, windows_cache_dir_ignores_xdg_cache_home}` |
| public function on the host | `…::user_cache_dir_is_absolute_when_home_is_known` |
| oracle file references (relative, bare, root-relative, absolute) | `dmls` `l1` `index_store_oracle::file_references_cover_relative_bare_root_relative_and_absolute_links` |
| oracle percent / angle destinations | `index_store_oracle::percent_encoded_destination_does_not_resolve_but_angle_form_does` |
| oracle unresolved forms (directory, `/`, case, NFD, `file://`) | `index_store_oracle::unresolved_and_non_document_link_forms_have_no_references` |
| oracle heading references | `index_store_oracle::heading_references_follow_anchor_links_and_wiki_headings` |
| oracle wiki references, two roots, ambiguity | `index_store_oracle::wiki_references_resolve_across_roots_and_count_ambiguous_links` |
| oracle DSL transcluded-by | `index_store_oracle::dsl_transcluded_by_lists_every_file_transclusion` |
| oracle heading rename | `index_store_oracle::heading_rename_rewrites_anchor_slugs_and_wiki_heading_text` |
| oracle file rename | `index_store_oracle::will_rename_files_repaths_markdown_links_and_respells_wiki_links` |
| oracle workspace symbols, nested-root de-duplication | `index_store_oracle::workspace_symbols_match_document_titles_and_headings` |
| oracle file list (each file once, symlinked dir unlisted) | `index_store_oracle::file_list_holds_each_corpus_document_once` |
| oracle symlinked directory | `index_store_oracle::link_through_symlinked_directory_is_not_a_reference` (skips only when a Windows host cannot create the symlink) |

### Gates and downstream tests

- the gate-skip calculator was not consulted for skipping: the phase changed source files, so both gates ran
- scoped (subagents): `cargo nextest run -p biscuit-file -E 'test(user_cache_dir) | test(cache_dir)'` 13 passed; `cargo nextest run -p dmls --features effects-instrumentation --test l1 -E 'test(index_store_oracle::) | test(test_layout)'` 12 passed (each under 0.3 s)
- `just test` in `biscuit-file/`: 1140 passed, 0 skipped; `just lint` in `biscuit-file/`: pass
- `just test` in `darkmatter/`: 10360 passed (6 slow), 10 skipped, 0 failed; `just lint` in `darkmatter/`: exit 0 (includes the zed-dmls wasm check)
- `just check-tier-coverage darkmatter`: 0 stranded
- `just test-l2` not run: no L2 test was touched
- downstream: the biscuit-file change only adds `user_cache_dir` and its re-exports. No crate glob-imports `biscuit_file::*` and no other crate defines `user_cache_dir` (`rg`), so no dependent can break; `dmls`, the consumer that matters, ran in the darkmatter suite above. The `rusqlite` add touches only `dmls`, which has no Rust dependents
- cross-OS (the corpus has a symlink, Unicode file names and absolute-path links; `user_cache_dir` is OS-specific): `just cross-check biscuit-file --os windows` pass (all 13 `user_cache_dir` tests ran), `just cross-check dmls --os windows` 1109 passed, 9 skipped, including all 11 `index_store_oracle::` tests. The cross-check published no observation because the tested tree is the uncommitted working tree, not HEAD
- no skipped or pre-existing failures

Phase 1 implementation complete, ready for review.
