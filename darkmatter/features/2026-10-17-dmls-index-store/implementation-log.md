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
source_files_during_phase_2:
    - darkmatter/dmls/src/wiki/universe.rs
    - darkmatter/dmls/src/wiki/mod.rs
    - darkmatter/dmls/src/graph/arena.rs
    - darkmatter/dmls/src/graph/node.rs
    - darkmatter/dmls/src/graph/invalidate.rs
    - darkmatter/dmls/src/providers/rename.rs
    - darkmatter/dmls/src/providers/wiki.rs
    - darkmatter/dmls/src/providers/code_actions.rs
    - darkmatter/dmls/src/bench.rs
    - darkmatter/dmls/tests/common/oracle.rs
    - darkmatter/dmls/tests/l1/main.rs
    - darkmatter/dmls/tests/l1/level1_wiki.rs
    - darkmatter/dmls/tests/l1/wiki_universe_equivalence.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
    - .claude/skills/darkmatter/dmls.md
source_files_during_phase_3:
    - darkmatter/dmls/src/lib.rs
    - darkmatter/dmls/src/index_store/mod.rs
    - darkmatter/dmls/src/index_store/facts.rs
    - darkmatter/dmls/src/index_store/failure.rs
    - darkmatter/dmls/src/index_store/keys.rs
    - darkmatter/dmls/src/index_store/location.rs
    - darkmatter/dmls/src/index_store/lock.rs
    - darkmatter/dmls/src/index_store/source.rs
    - darkmatter/dmls/src/index_store/source_tests.rs
    - darkmatter/dmls/src/index_store/store.rs
    - darkmatter/dmls/tests/l1/main.rs
    - darkmatter/dmls/tests/l1/index_store_concurrency.rs
    - darkmatter/dmls/tests/l1/index_store_facts.rs
    - darkmatter/dmls/tests/l1/index_store_keys.rs
    - darkmatter/dmls/tests/l1/index_store_open.rs
    - darkmatter/dmls/tests/l1/index_store_sweep.rs
    - darkmatter/dmls/tests/fixtures/index_store_golden.txt
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
    - .claude/skills/darkmatter/dmls.md
    - .claude/skills/os/windows.md
source_files_during_phase_4:
    - biscuit-file/lib/src/file_reference/context.rs
    - biscuit-file/lib/src/file_reference/mod.rs
    - biscuit-file/lib/src/lib.rs
    - darkmatter/dmls/src/full_index.rs
    - darkmatter/dmls/src/graph/invalidate.rs
    - darkmatter/dmls/src/index_store/mod.rs
    - darkmatter/dmls/src/index_store/source.rs
    - darkmatter/dmls/src/index_store/source_tests.rs
    - darkmatter/dmls/src/index_store/store.rs
    - darkmatter/dmls/src/index_store/workspace.rs
    - darkmatter/dmls/src/index_store/workspace_tests.rs
    - darkmatter/dmls/src/router.rs
    - darkmatter/dmls/src/router/facts.rs
    - darkmatter/dmls/tests/common/mod.rs
    - darkmatter/dmls/tests/l1/main.rs
    - darkmatter/dmls/tests/l1/index_store_live.rs
docs_updated_during_phase_4:
    - darkmatter/docs/lsp/indexing.md
    - biscuit-file/docs/topics/file-references.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
    - .claude/skills/darkmatter/dmls.md
    - .claude/skills/biscuit-file/SKILL.md
    - .claude/skills/biscuit-file/references/api.md
source_files_during_phase_5:
    - darkmatter/dmls/src/index_store/workspace.rs
    - darkmatter/dmls/src/router.rs
    - darkmatter/dmls/src/router/facts.rs
    - darkmatter/dmls/src/providers/symbols.rs
    - darkmatter/dmls/src/providers/mod.rs
    - darkmatter/dmls/src/providers/rename.rs
    - darkmatter/dmls/src/graph/arena.rs
    - darkmatter/dmls/src/full_index.rs
    - darkmatter/dmls/tests/common/oracle.rs
    - darkmatter/dmls/tests/l1/main.rs
    - darkmatter/dmls/tests/l1/index_store_requests.rs
    - darkmatter/dmls/tests/l1/index_store_sessions.rs
    - darkmatter/dmls/tests/l1/index_store_stdio.rs
docs_updated_during_phase_5:
    - darkmatter/docs/lsp/indexing.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
    - .claude/skills/darkmatter/dmls.md
message_to_agent: |-
  Phase 5 is done: references (including wiki references and DSL transcluded-by), heading rename, file rename, and workspace symbols now *answer* from the index store, but the full index still gates them. For Phase 6:
  - Answer path: `ServerState::request_graph(targets)` (router/facts.rs) -> `WorkspaceFacts::request_graph` (candidates over every root, listed only, + listed targets + overlay of open buffers and unlisted targets' `disk_parse`, universe = live snapshot's `shared_wiki_universe()`, shared `RepositoryContexts`). Workspace symbols: `ServerState::symbol_documents()` -> `WorkspaceFacts::symbol_documents` -> `registry.workspace_symbols(query, &[SymbolDocument])` (no graph). References targets come from `providers::reference_target_at` over the live snapshot; that keeps working without the full index because every listed file is a (root-only) snapshot document and the open document is parsed.
  - What is still the full index, and goes in Phase 6: `Router::needs_full_index` requires `index_is_full()` as well as `facts_ready()`; each build round parses for `WorkspaceIndex::begin_full_build`/`finish_full_build` and then runs the store pass with those parses as `known` (so with the full build removed, `revalidate` parses changed files itself); `bridge_to_index`; the `full index built` line. The store pass, `FactsEvents`, the fallback pre-rename build (`Router::revalidated`), and `refresh_rename_referrers` (which now refreshes closed *candidates*, not snapshot referrers) must survive the removal.
  - Measured on this repository (loaded host): the first references request is about 9 s and the process peaks at about 1.1 GB, all of it the full build; each later references request is 10 ms and workspace symbols 54 ms. Once the build is gone, add the server-level "warm restart parses zero" test Phase 4 deferred, and re-measure for acceptance 11 in Phase 7.
  - Tests that prove the request path: `tests/l1/index_store_requests.rs` (oracle equivalence at every corpus probe, no-miss, memory modes; the oracle gained `*_over(graph, ...)` methods), `index_store_sessions.rs` (restart, two roots, nested roots), `index_store_stdio.rs` (spawned binary, `off` and corrupt). Mutation check: a `request_graph` without candidates fails 18 of them, so they will catch a removal that falls back to the snapshot.
  - Pre-existing: a `../` link out of a workspace folder that is in no repository does not resolve (store or not); a two-root fixture needs a `.git` above both folders.
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

## Phase 2

Implemented directly by the orchestrator (one design, one agent, as Wave 3 asks); no subagents.

### What changed

- **`WikiUniverse`** (`darkmatter/dmls/src/wiki/universe.rs`, re-exported as `dmls::wiki::WikiUniverse`): one slot per logical path, sorted and de-duplicated, each a `WikiDoc` canonicalized against the universe's wiki roots. `slot(path)`, `path(slot)`, `docs()`, `wiki_roots()`. `WikiDoc::for_path(path, wiki_roots)` is now the one canonicalization helper; it replaces `arena::document_wiki_meta` and `providers/rename.rs::wiki_doc_for` (two copies of the same logic)
- **Graph API** (`graph/arena.rs`): `build_with_roots(indices, generation, universe: impl Into<Arc<WikiUniverse>>, contexts)` and `build_listed(.., universe, ..)` take the universe instead of `wiki_roots`; `assemble` takes `Arc<WikiUniverse>`. The wiki roots now travel inside the universe. `WorkspaceGraph::build` (no production caller) passes a universe of its own documents with no roots, as before
- **Slots**: the graph keeps a private `WikiSlots` (`Arc<WikiUniverse>`, `DocumentId -> slot`, `slot -> DocumentId`). Public accessors `wiki_universe()`, `wiki_slot(doc)`, `slot_document(slot)`. `WorkspaceGraph::wiki_docs()` is **removed**
- **Invariant (decision)**: every graph document has a slot. A document missing from the supplied universe is appended at build time (`Arc::make_mut`, so the shared universe is copied only when this happens); a dependency is appended as an unmatchable slot (empty canonical, `NO_ROOT`), exactly what `DocumentRecord::wiki_doc` used to give it. Reason: the matcher needs a source slot for every document with wiki links, and an open buffer outside the file list was a wiki candidate before this change, so it stays one
- **Resolver** (`resolve_wiki_link`): calls `wiki::resolve_file` with the source's slot over `universe.docs()`
    - `Match::Many(slots)` -> `WikiResolution::Ambiguous` listing only the candidates that are graph documents (ambiguity is judged against the universe; a scoped graph may list fewer than two, or none)
    - `Match::One(slot)` whose file is not a graph document -> **new** `WikiResolution::OutsideGraph`: no edge, no diagnostic (it is resolved, so reporting it `Unresolved` would be wrong); hover text is a plain `Wiki target: [[...]]`. Only a scoped graph can produce it: the `WorkspaceIndex` universe never holds a file that is not one of its documents
- **Providers**
    - `rename.rs::apply_file_rename`: post-rename universe is `universe.docs()` with the old document's slot replaced by `WikiDoc::for_path(new_path)`; each wiki link's source is its document's slot. **Behavior addition**: the destination-conflict check also refuses a destination that is a universe file, not only a graph document (identical for the `WorkspaceIndex` graph, whose universe equals its documents)
    - `wiki.rs::file_completions`: still offers graph documents (non-dependency, not the source), spelled through their slots in the universe
    - `wiki.rs::heading_target_doc`: `Match::One(slot)` -> `slot_document(slot)`
- **`WorkspaceIndex` universe (Wave 3 decision)**: its own documents, i.e. the file list (`listed`) plus every parsed document that is not a referenced dependency (open buffers outside the list). This is exactly the set the graph used to derive for itself, so open-document diagnostics are unchanged; the full L1 suite and `graph::invalidate` tests confirm it. `WorkspaceIndex::new` builds its empty, `Pending` graph with `WikiUniverse::default()` (no documents, nothing to resolve)
- **Callers updated**: `graph/invalidate.rs` (`new`, `rebuild`, one test assertion that read `wiki_docs()`), `providers/code_actions.rs` test (now `WorkspaceGraph::build`), `bench.rs` (universe = every benched file, the full-build shape; the bench is deleted in Phase 6), `tests/l1/level1_wiki.rs`, and the oracle (`tests/common/oracle.rs`)
- **Oracle** (test-only): `Oracle::scoped(roots, snapshot, documents, UniverseScope)` builds a graph over only `documents`, answered exactly like the oracle, matching against `UniverseScope::FileList` (the whole listing, what the store will supply) or `UniverseScope::Documents` (a negative control). `Oracle::wiki_diagnostics(path)` returns `(line, code, message)`; related information is left out because it lists in-graph candidates only, which a scoped graph has fewer of by design

### Not changed (recorded for later phases)

- `portability_collisions` (`wiki.portability-collision`) and the `KeyIndex` still derive from graph documents. Neither is one of the four request paths, and both serve open-document diagnostics on the `WorkspaceIndex` graph, whose universe equals its documents
- an ambiguous diagnostic's related information on a scoped graph lists only in-graph candidates. No production diagnostic is computed over a scoped graph (Phase 5 builds them for references and rename only)

### Validation checkpoint 2: universe sources

Every `WikiUniverse` construction (`rg 'WikiUniverse::(new|default)'`):

| Caller | Universe | Why allowed |
| --- | --- | --- |
| `graph/invalidate.rs` `WorkspaceIndex::rebuild` | file list + non-dependency documents | the recorded Wave 3 choice |
| `graph/invalidate.rs` `WorkspaceIndex::new` | empty | the empty initial graph has no documents |
| `graph/arena.rs` `WorkspaceGraph::build` | its documents, no roots | test-only convenience (no production caller), whole-set graphs |
| `bench.rs` | every benched file | a full build over the whole listing; removed in Phase 6 |
| `tests/common/oracle.rs` | full listing (`FileList`) or scoped documents (`Documents`, negative control) | the oracle helper |
| `tests/l1/level1_wiki.rs`, `wiki/universe.rs` and `graph/arena.rs` unit tests | literal path sets | tests |

No production caller passes an empty or document-derived universe to a scoped graph; none exists yet (Phase 5 adds them with a file-list universe).

### Requirement-to-test mapping

| Behavior | Test |
| --- | --- |
| R3: ambiguous `[[topic]]` whose rival is not a document stays ambiguous; diagnostics and references equal the oracle; control (own-documents universe) is unique | `dmls` `l1` `wiki_universe_equivalence::ambiguous_wiki_link_stays_ambiguous_when_its_rival_is_not_a_document` |
| a unique match outside the graph is not reported unresolved (diagnostics equal the oracle) | `wiki_universe_equivalence::wiki_link_to_a_listed_file_outside_the_graph_is_not_reported_unresolved` |
| R3: file-rename spelling avoids a rival that is not a document (`deep/topic`, oracle-equal); control picks the colliding `topic` | `wiki_universe_equivalence::file_rename_spelling_avoids_a_rival_that_is_not_a_document` |
| file rename onto a listed file outside the graph is refused like the oracle; control would rename onto it | `wiki_universe_equivalence::file_rename_onto_a_listed_file_outside_the_graph_is_refused` |
| cross-root file rename and wiki references over the referrers equal the oracle | `wiki_universe_equivalence::cross_root_file_rename_over_its_referrers_equals_the_oracle` |
| heading rename and heading references (anchor and wiki heading links) over the referrers equal the oracle | `wiki_universe_equivalence::heading_rename_over_its_referrers_equals_the_oracle` |
| graph: `Ambiguous` keeps in-graph candidates and their edges; `OutsideGraph` for a unique outside match | `graph::arena::tests::test_supplied_universe_judges_wiki_links_beyond_the_documents` |
| graph: a document missing from the universe is appended and stays a wiki candidate | `graph::arena::tests::test_document_missing_from_the_universe_is_a_wiki_candidate` |
| universe slots: path order, de-duplication, root membership, first containing root | `wiki::universe::tests::{test_slots_follow_path_order_and_drop_duplicates, test_path_outside_every_root_has_no_root, test_first_containing_root_wins}` |
| dependency is never a wiki candidate (unmatchable slot) | existing `graph::invalidate::tests` assertion, retargeted to `wiki_slot`/`wiki_universe` |
| no regression: wiki, rename, graph, oracle self-check | existing `level1_wiki::*`, `level1_graph_index::*`, `providers::rename::tests::*`, `index_store_oracle::*` (unchanged expectations), plus the full suite |

Each `wiki_universe_equivalence` test passed on its first run; the negative controls show the universe is what keeps each answer equal. The Input Robustness Matrix does not apply to this phase: no file format or configuration reader was added or changed.

### Gates and downstream tests

- gate-skip calculator not consulted for skipping: the phase changed source files, so both gates ran
- scoped during the work: `cargo nextest run -p dmls --features effects-instrumentation --test l1 -E 'test(wiki_universe_equivalence::) | test(index_store_oracle::) | test(test_layout) | test(level1_wiki::)'` 29 passed (each under 0.12 s); `cargo nextest run -p dmls --features effects-instrumentation --lib -E 'test(graph::arena::) | test(wiki::) | test(providers::rename::) | test(graph::invalidate::)'` 79 passed
- `just test` in `darkmatter/`: 10371 passed, 10 skipped, 0 failed
- `just lint` in `darkmatter/`: exit 0 (includes the zed-dmls wasm check)
- `just check-tier-coverage darkmatter`: 0 stranded
- `just test-l2` not run: no L2 test was touched
- downstream: no workspace crate depends on `dmls` (Phase 1 inventory), and the public API change (`build_with_roots`/`build_listed` signature, `wiki_docs()` removed, `WikiResolution::OutsideGraph`) has no consumer outside `dmls`
- cross-OS: not run. The change adds no path handling beyond the `Path::starts_with` root membership that already existed (moved, not changed) and no `cfg` code; the corpus tests it reuses passed on native Windows in Phase 1
- spec frontmatter: this phase was asked to set no spec properties
- no skipped or pre-existing failures

Phase 2 implementation complete, ready for review.

## Phase 3

Implemented directly by the orchestrator, without subagents. The plan asked for three parallel agents in Wave 5, but the facts row, candidate keys, store, lock, and fallback ladder form one design that shares types at every seam, so splitting it would have risked agents diverging for little wall-clock gain.

### What changed

New module `darkmatter/dmls/src/index_store/` (`pub mod index_store` in `lib.rs`). As the plan says, nothing is wired into the server yet.

- **`facts.rs`**: `FileFacts { path, hash, headings, links, wiki_links, transclusions, candidate_keys }`. `FileFacts::from_index(&DocumentIndex)` and `into_index(self)` (ruling 9: unstored fields empty, `content_hash` = row hash, `LinkTarget` recomputed with `classify_link_target` on decode). Rows store every field of `HeadingFact`, `WikiLinkFact`, and `TransclusionFact`, plus the raw target, span, and line of `LinkFact`, so `into_index` is exact on the stored fields
    - **departure from the plan's sketch**: the plan wrote `from_index(&DocumentIndex, hash, candidate_keys)`. The hash already is `DocumentIndex::content_hash` (raw-bytes xxHash via `work_counters::hash_bytes`), and the keys are a pure function of the index, so `from_index` takes only the index. Two parameters that must agree with the index were not worth having
- **`keys.rs`** (R2, ruling 2): `candidate_keys(&DocumentIndex)` and `target_keys(&Path) -> [path_key, name_key]`
    - `./`/`../` → the joined key only; bare → joined key + file-name key; `/`, `~`, `^`, `@`, `$`, any `scheme:` (also a `C:` drive), and directory (`sub/`) forms → file-name key; wiki links → file-name key of the last segment; `file://` (classified external) → file-name key; other external links and same-document anchors → none
    - every key is NFC + `to_lowercase`; path keys are `/`-separated with `.`/`..` folded lexically (never above the root or a drive), backslashes unified, `\\?\` stripped (ruling 10)
    - **refinement**: a link emits both the raw and the Markdown-extension-elided final segment (the target key uses the elided form), so `[[note.md]]` meets `note.md.md` (wiki `last_matches` accepts both forms). A `<...>` or percent-encoded target emits keys for each spelling: authored, angle-stripped, and percent-decoded
    - path keys always contain `/` and name keys never do, so one `cand.key` column serves both
- **`location.rs`** (R7, R8, R10): `StoreLocation::prepare(override, user_cache_dir)` (ruling 11: unset/empty = default `<cache>/darkmatter/dmls`, `off` any case = `Disabled`, relative or naming a file = `Unavailable`, a missing directory is created `0700` through `DirBuilder::mode`), `root_key` (xxHash of `canonicalize_simplified(root)` as 16 hex; a root that cannot be canonicalized keys as given), `store_path`, `lock_path`, `touch_mtime` (write handle, ruling 12), `sweep_stale_stores(dir, max_age, now)`, and `STALE_STORE_AGE` (30 days)
    - `prepare` takes the `DMLS_CACHE_DIR` value and `user_cache_dir()` as **parameters**: no production code in this phase reads the environment or calls `user_cache_dir()`, so the context-construction and ambient-read guards are untouched. Phase 4 supplies both (through the request snapshot) and must check those guards then
    - **departure**: the plan said the sweep uses an "injected clock from the dependency's `clock.rs` seam". `clock.rs` gives monotonic `Instant`s, which cannot be compared with file mtimes, so the sweep takes `now: SystemTime` as a parameter. The once-per-hour touch throttle uses `Clock`
    - the sweep groups `<16 hex>-f<digits>.sqlite` with its `-wal`, `-shm`, and `.lock` siblings, judges age by the newest sibling (a sibling whose mtime cannot be read keeps the store), deletes under the exclusive lock, deletes the lock file last while holding it, counts os error 32 (`cfg(windows)`) as in use, and never looks at the root
- **`lock.rs`** (R10, **added**: the spec requires the lock file, acceptance 10, but the plan's Phase 3 tasks list only the os-error-32 skip): `StoreLock::shared(path, wait)` and `StoreLock::try_exclusive(path)` on std `File::try_lock_shared`/`try_lock` (toolchain 1.98.1; no new crate). The lock file is created `0600`. After locking, Unix checks that the held file is still the file at the path (inode and device), so a lock taken on a file a sweep just unlinked is retried. `shared` waits up to 2 s for a brief exclusive holder
- **`store.rs`** (R1, R11, ruling 17): `IndexStore::{open, upsert_batch, remove_missing, hashes, load, candidates_for, scan_headings, touch}`
    - layout: `meta`, `files(id, path UNIQUE, hash)`, `headings`, `links`, `wiki`, `transclusions` (each `PRIMARY KEY (file_id, ord)`), and `cand(file_id, key)` with index `cand_key`. All tables are `STRICT`
    - open: shared lock, the database file pre-created `0600` (SQLite copies its mode to `-wal`/`-shm`), `busy_timeout` 5000 ms, `journal_mode=WAL` retried every 5 ms for up to 2 s (ruling 4; still busy = `Busy`), `synchronous=NORMAL`, then create-or-validate in one `BEGIN IMMEDIATE` (no tables = create and stamp; a missing table, a `format`/`facts_version` stamp other than this build's, or a missing stamp = `Corrupt`; another root = `RootMismatch`). Open touches the store
    - a zero-byte file is a valid empty database and is treated as a new store, consistent with the database file open itself creates
    - batches of 200 per `BEGIN IMMEDIATE`; upsert by path, skipped when the stored hash matches; a replaced row drops all its child rows first. `remove_missing` deletes only paths whose `symlink_metadata` is `NotFound` (a permission error keeps the row)
    - one decode path per table: an inverted span, a heading level outside 1-6, an `unsupported` flag other than 0/1, half a heading span, an out-of-range integer, or invalid UTF-8 is `StoreFailure::Corrupt`
    - **touch**: on open, then at most once per `TOUCH_INTERVAL` (1 h, on the store's `Clock`) on **any** read or write (ruling 12 says "any use or batch"; reads touch too)
    - the hash is stored bit for bit as a signed integer. A path that is not UTF-8 cannot be stored: upsert reports `Unavailable`, and the ladder moves to memory
- **`failure.rs`**: `StoreFailure::{Corrupt, RootMismatch, Busy, InUse, Unavailable}` with `reason()` labels. rusqlite `DatabaseCorrupt`/`NotADatabase` and every value-conversion error map to `Corrupt`, `DatabaseBusy`/`DatabaseLocked` map to `Busy`, everything else (read-only, full, cannot open) maps to `Unavailable`
- **`source.rs`** (R12): the `FactsSource` trait, `StoreFacts` (open-or-repair: corrupt or root mismatch → exclusive lock, delete the database files, reopen once; lock held or delete refused = `InUse`), `MemoryFacts(HashMap<PathBuf, FileFacts>)`, and `RootFacts`, the infallible ladder
    - **decision**: recreate-once applies at open. Any failure after open switches the root to memory for the process lifetime, first **salvaging** every row the failed store can still read (`hashes` + `load`) and then adding the facts being written. "Keeping facts already computed" is therefore exact for a disk-full write and best effort for corruption. After a switch, `take_switched()` returns true once, so the caller revalidates before trusting a candidate set; Phase 4 must honor it
    - one `warn` per process (`index store unavailable, using memory reason=… detail=… path=…`, a process-wide `AtomicBool`) across every root, `Disabled` included (acceptance 8 counts one warn for `off`), plus `warn` `index store recreated path=… reason=…`. The R13 `info` lines and the `workspace` field are Phase 6
    - fault-injection seam: a test-only `FaultyFacts` wrapper (fails every call from the Nth on) in `source_tests.rs`, injected through `RootFacts::with_source` (`pub(crate)`)

### Deferred (recorded for later phases)

- the plan's matrix asks for one stdio wiring test (`DMLS_CACHE_DIR=off` and a corrupt store answering references over stdio). It needs the server wiring, so it moves to Phase 4/5
- R13 `info` lines (`opened`, `revalidated`, `sweep`) and the `workspace` field: Phase 6

### Ruling 16: line for the user-global `storage-strategy` skill

Not applied in this repository (ruling 16). The author adds this line to `~/.claude/skills/storage-strategy` (its "safe to delete" list):

> - `<user cache dir>/darkmatter/dmls/` (macOS `~/Library/Caches/darkmatter/dmls/`, Linux `${XDG_CACHE_HOME:-~/.cache}/darkmatter/dmls/`, Windows `%LOCALAPPDATA%\darkmatter\dmls\`, or `$DMLS_CACHE_DIR`): DMLS index stores (`<hex16>-f<N>.sqlite` plus `-wal`, `-shm`, `.lock`), a pure cache of Markdown facts. Always safe to delete; a running `dmls` keeps working and the next start refills. Stores untouched for 30 days are swept automatically.

### Smell grep (Validation checkpoint 3)

`rg 'unwrap_or_default\(\)|\.ok\(\)|filter_map|serde\(default' darkmatter/dmls/src/index_store`: one hit, `source.rs` `MemoryFacts::get_all` (`paths.iter().filter_map(|path| self.0.get(path).cloned())`). That is a map lookup where "no facts for this path" is the same answer the store gives for a path without a row, not a decode that turns an error into "no rows". The two `unwrap_or_default()` calls I first wrote (`keys.rs` source directory; the sweep's mtime read through `.ok()`) were rewritten: the sweep now keeps a store whose sibling mtime cannot be read. Every row decode returns `rusqlite::Result` and propagates as `Corrupt`; there is no `query_map(..).filter_map(Result::ok)`.

### Input Robustness Matrix (this phase's loaders)

| Loader | Shape | Outcome | Test |
| --- | --- | --- | --- |
| `IndexStore::open` | control | created, then reopened with its rows | `index_store_open::control_store_opens_created_then_reopens_with_its_rows` |
| | zero-byte file | new store (exception) | `…::zero_byte_file_is_a_new_store` |
| | garbage, truncated | `Corrupt`; `StoreFacts::open` recreates once | `…::garbage_and_truncated_files_are_corrupt_and_recreated_once` |
| | missing table | `Corrupt` naming the table | `…::a_missing_table_is_corrupt` |
| | `format`/`facts_version` stamp wrong or missing | `Corrupt` | `…::a_format_or_facts_stamp_other_than_this_build_is_corrupt` |
| | `meta.root` mismatch | `RootMismatch`; refilled (exception) | `…::a_store_for_another_root_is_a_root_mismatch_and_refills`, `source::tests::a_store_recorded_for_another_root_is_refilled` |
| row decode (one shared path per table) | control + one corrupted cell each in `files`, `headings`, `links`, `wiki` (two shapes), `transclusions`, `cand` | `Corrupt`, never empty | `…::a_corrupted_cell_in_any_table_is_corruption_never_an_empty_result`; through the ladder: `…::a_corrupted_cell_read_through_root_facts_falls_back_to_memory` |
| | wrong type on write | refused by `STRICT` | `…::strict_tables_refuse_a_wrongly_typed_value` |
| | empty title, zero-byte Markdown file | valid stored facts (exception) | `index_store_facts::an_empty_file_and_an_empty_heading_title_are_stored_facts` |
| `DMLS_CACHE_DIR` (`StoreLocation::prepare`) | unset, empty, `off`/`OFF`/`Off`, absolute missing (created) and existing, relative, names a file, cannot be created; no user cache dir | per ruling 11 | `index_store_open::cache_dir_override_shapes`; `source::tests::disabled_location_uses_memory_and_creates_no_file` |
| `user_cache_dir()` | (Phase 1) | | Phase 1 tests |

### Requirement-to-test mapping

| Behavior | Test |
| --- | --- |
| R1 round trip, write/read/write stability, reopen | `dmls` `l1` `index_store_facts::corpus_facts_round_trip_through_the_store_and_a_reopen` |
| R1 replaced row drops old child rows and keys | `index_store_facts::a_changed_file_replaces_its_row_and_drops_its_old_facts` |
| R9 golden; fails on change without a `FACTS_VERSION` bump and refuses to regenerate without one (both guards checked by mutating the snapshot) | `index_store_facts::corpus_facts_match_the_golden_snapshot_for_this_facts_version` (snapshot `tests/fixtures/index_store_golden.txt`) |
| R2 no-miss over the whole corpus (every oracle `references`/`transcludes` pair), including a nested file holding only repository-root-relative bare links and `::file` (S1's shape); memory and store candidates equal. A mutation (bare → joined key only) makes it fail on exactly that referrer | `index_store_keys::candidates_never_miss_a_true_referrer_on_the_corpus` |
| R2 `../` yields no file-name key | `index_store_keys::an_explicitly_relative_link_is_not_a_candidate_for_a_same_named_file_elsewhere`, `index_store::keys::tests::explicitly_relative_link_yields_only_the_joined_key` |
| R2 forms: bare root-relative link and `::file`, wiki case/NFD/extension variants, `%20`, `<...>`, `file://`, `~`, directory, `/`, external links and anchors (none), Windows separators and verbatim prefix | `index_store::keys::tests::*` (6 tests) |
| R7 override shapes, directory `0700`, store/`-wal`/`-shm`/`.lock` `0600` | `index_store_open::cache_dir_override_shapes`, `…::store_files_are_private_and_a_created_directory_is_0700` (`cfg(unix)`), `index_store::lock::tests::the_lock_file_is_private_to_its_owner` |
| R7/R12 `off`: memory, no file, one warn | `index_store::source::tests::disabled_location_uses_memory_and_creates_no_file` |
| R8 key stable, 16 hex, canonical, file name carries `FACTS_VERSION`; macOS case pinned (canonicalization returns the on-disk case, so case spellings share a store) | `index_store_open::store_key_is_stable_sixteen_hex_digits_and_names_the_facts_version`, `…::macos_canonicalization_folds_a_differently_cased_root_to_one_key` (`cfg(target_os = "macos")`) |
| R10 sweep: stale deleted (all four siblings), fresh kept, unrelated files kept, newest sibling decides, missing root kept | `index_store_sweep::{a_store_untouched_for_thirty_days_is_deleted_and_a_fresh_one_kept, the_newest_sibling_decides_a_stores_age, a_store_whose_root_is_missing_is_kept_while_fresh}` |
| R10 touch by a batch and by a read after the interval (database file mtime, since WAL writes refresh only `-wal`); not within it | `index_store_sweep::a_store_touched_by_a_recent_batch_or_read_is_kept` |
| R10 holder open + aged: skipped, nothing unlinked, holder's later writes visible to a fresh connection (the Unix unlink-while-open case; runs on every OS) | `index_store_sweep::a_store_another_holder_has_open_is_skipped_and_stays_shared` |
| R10 Windows os error 32 skip + write-handle touch on an open store | `index_store_sweep::windows_skips_a_store_whose_database_is_open_without_its_lock` (`cfg(windows)`, native Windows leg) |
| R10 lock semantics | `index_store::lock::tests::*` (4) and `source::tests::removing_a_store_takes_every_sibling_file` |
| R11 interleaved connections converge; foreign mismatching row replaced | `index_store_concurrency::{two_connections_writing_interleaved_batches_converge, a_row_another_writer_left_for_different_bytes_is_replaced}` |
| R11 acceptance 9: two **processes** (the test binary re-run as two workers, started together on a go file) create one new store at once and fill 250 files in interleaved 50-file batches; each worker's view and the final rows equal a fresh extraction | `index_store_concurrency::two_processes_cold_filling_one_new_store_reach_identical_rows` (+ worker `cold_fill_worker`, a no-op unless spawned) |
| R11/R12 WAL switch busy past its bound: memory, one warn, the holder's database untouched (never recreated) | `index_store::source::tests::a_wal_switch_busy_past_its_bound_uses_memory_and_never_recreates` (a real `BEGIN EXCLUSIVE` holder; short bound via `OpenSettings`) |
| R12 corrupt recreated once; corrupt and held = memory, file untouched; mid-fill write error keeps salvaged rows + written facts; nothing salvageable keeps the written facts; memory persists, the failed backend is never asked again, one warn across roots | `index_store::source::tests::{a_corrupt_store_is_recreated_once, a_corrupt_store_another_process_holds_falls_back_to_memory, a_readable_store_that_fails_a_write_is_salvaged_into_memory, a_write_error_mid_fill_keeps_salvaged_rows_and_the_facts_being_written, memory_persists_for_later_requests_and_warns_once}` |
| store path spelling: two spellings `PathBuf` treats as equal (a doubled `/`, a `.` segment; on Windows `\` mixed with `/`) name one row | `index_store_facts::two_spellings_path_equality_treats_as_one_name_one_row` |

### Cross-OS finding (fixed)

The first `just cross-check dmls --os windows` failed one test: `two_processes_cold_filling_one_new_store_reach_identical_rows`. The test built paths as `root.join("d0/f0.md")`, which is `…\ws\d0/f0.md` on Windows, while the file list spells them `…\ws\d0\f0.md`. `PathBuf` equality (and the graph's `by_path` map) compares components, but the store looked rows up by raw text, so `load` found nothing. Fix in production code: `store.rs` stores and looks up `path.components().collect::<PathBuf>()` (`path_text`), so a row is named exactly as `PathBuf` equality names a path. The regression test above runs on every OS. The second cross-check passed: 1166 tests, 0 failed, 9 skipped, including `windows_skips_a_store_whose_database_is_open_without_its_lock` (os error 32, write-handle touch) and the two-process cold fill (2.8 s on Windows; I reduced its corpus from 450 to 250 files to keep margin under the 5 s L1 budget).

### Gates and downstream tests

- the gate-skip calculator was not consulted for skipping: the phase changed source files, so both gates ran
- scoped during the work: `cargo nextest run -p dmls --lib -E 'test(index_store::)'` (20 unit tests) and `cargo nextest run -p dmls --features effects-instrumentation -E 'test(index_store)'` (61 tests across lib and `l1`, every one under 0.5 s on macOS; the concurrency file passed 3 runs in a row)
- `just test` in `darkmatter/`: **10420 passed, 10 skipped, 0 failed** (final run, after the Windows fix)
- `just lint` in `darkmatter/`: exit 0 (includes the zed-dmls wasm check)
- `just check-tier-coverage darkmatter`: 0 stranded. No new test name carries a tier marker; every new file is declared in `tests/l1/main.rs`
- `just test-l2` not run: no L2 test was touched
- downstream: no workspace crate depends on `dmls` (Phase 1 inventory); `biscuit-file` was not changed in this phase
- cross-OS: `just cross-check dmls --os windows` pass on the second attempt (above). Linux/WSL not run separately: the only `cfg` branches are `unix` (exercised on macOS) and `windows` (exercised on native Windows)
- no skipped or pre-existing failures

Phase 3 implementation complete, ready for review.

## Phase 4

Implemented directly by the orchestrator, without subagents. As in Phase 3, the revalidation pass, ready state, live updates, and router wiring share one design (the build round's thread, the dispatch thread's event handling, and the rename path all meet at `WorkspaceFacts`), so splitting them across agents would have risked divergence for little wall-clock gain.

### What changed

- **`biscuit_file::user_cache_dir_from(env, home)`** (biscuit-file, additive): `user_cache_dir()`'s rules over an injected environment lookup and home (a relative home counts as none). `user_cache_dir()` now wraps it. **Why:** the plan said to "call `biscuit_file::user_cache_dir()`", but that reads the process environment from library code, against the darkmatter rule that only binaries read process state (`RequestSnapshot::from_process`). DMLS now derives the store location from its request snapshot (`DMLS_CACHE_DIR` from `snapshot.env()`, the cache dir from `snapshot.env()` + `snapshot.home()`), so no new ambient read and no guard entry was needed (`context_construction_guard` passes unchanged). Side effect, intended: test sessions built from `LspFixture::fixture_snapshot` (fixture home inside the workspace, empty env) keep their stores inside the workspace temp dir (macOS `<ws>/home/Library/Caches/darkmatter/dmls`, Linux `<ws>/home/.cache/...`; Windows has no `LOCALAPPDATA` in the empty env, so memory)
- **`IndexStore::remove_paths` / `FactsSource::remove_paths` / `RootFacts::remove_paths`**: per-path form of `remove_missing` (deletes only paths missing on disk), so a deletion event does not stat every row
- **`index_store/workspace.rs`: `WorkspaceFacts`** (new). One `RootFacts` per effective root plus `verified` (hash of each file read in this process)
    - `open(roots, location, clock)`: opens every root, then sweeps the store directory once per process (static `SWEPT`), after this workspace's stores are open (so they are locked and fresh)
    - `revalidate(files, known, progress, cancel)` (R4 steps 2-4): every root's `hashes()`, then a worker pool (same shape and size rule as `parallel_index`) reads and hashes each file through `work_counters`, re-parses only where some owning root's row differs, writes per root in batches of `BATCH_SIZE` checking `cancel` between batches, deletes rows of missing paths (`remove_paths`) and then `remove_missing` on every root. No size/mtime path
    - `refresh(paths, known)` (R6): the same for a few paths, comparing against those rows only (`load`), not every row's hash
    - `known`: a supplied parse is used only when its `content_hash` equals the bytes read (a parse is a function of path and bytes). The router passes the build round's parses plus open-buffer parses, so **the store pass adds no parses to what the full index already does**; all 32 dependency `full_index` tests' parse counts hold unchanged
    - ready (`is_ready(listed, roots)`): `mark_ready` by the caller (only it knows whether changes arrived during the pass), same roots, every listed path verified. A switch to memory (`take_switched` after every operation) withdraws it
    - `request_documents(paths, buffers)` (R5): stored facts into `DocumentIndex`, each open buffer in place of its row (or added); writes nothing
    - a file under nested roots gets a row in each store (R8)
- **Router wiring** (`router.rs`, new `router/facts.rs`):
    - `ServerState::facts: Arc<Mutex<Option<WorkspaceFacts>>>`, opened on the first build round's thread (never at startup), reopened when the effective roots change. `StoreInputs` captures `DMLS_CACHE_DIR` and the user cache dir from the snapshot at startup; `StoreLocation::prepare` runs on the round thread
    - each build round runs a store pass after its full-index parse (`FactsPlan::Full` on a build's first round and after a switch; `Paths(touched)` afterwards). `FactsEvents::touched` records Markdown paths changed while a round reads; the round finish starts another round when the full index needs one, when anything was touched, or when a root switched. Ready = both the full index and the store. A listed file the store pass cannot read fails the build like the full index's own reads (filtered by touched)
    - `proceed` now waits when a build runs, the index is not full, **or the store is not ready**, and (fallback watch mode) for every rename that has not had its own build (`Router::revalidated`), which is the R6 pre-rename full re-hash pass
    - events: before ready only the file list changes; during a build they are recorded; after ready, client watching → `refresh` at once (after the index applied the event, so its parse of the new bytes serves the row: one parse per event, asserted); fallback mode → held in `pending` until the next save, which refreshes the saved file plus held paths (`install_listing`'s joined/left paths are fed in, so a save's relist reaches it). Watched events of open documents reach the store (rows hold disk state)
    - `bridge_to_index`: any read whose hash differs from `WorkspaceIndex`'s disk parse calls `disk_changed` (and a missing one `disk_deleted`), so the transitional full index never answers from bytes the store already saw replaced. This is what makes acceptance 7 hold through the server today. **Phase 6 deletes it with the full index**
    - rename freshness: before computing a heading rename or file rename, the router refreshes the closed targets and every closed document with a `References`/`Transcludes` edge into a target (`referrers_of`), then reloads the snapshot. **Finding:** the plan's natural order (compute, then re-read edited files) does not work: `rewrite_heading_reference` drops an edit when the old slug is not found at the stale span, so the changed referrer is missing from the edit and never re-read. It must be refreshed *before* computing
- **`WorkspaceIndex::open_buffers`, `disk_parse`** (accessors used above). `FactsPlan` and a `facts` field on `Build` (`full_index.rs`)
- **`LspFixture::start_with_full_index_gate_and_snapshot`** (test harness)
- docs: `darkmatter/docs/lsp/indexing.md` gains "The index store" (location table, fill, live-update table per watch mode, rename re-read rule, example) and two state-diagram edges; it states plainly that the store does not answer requests yet. `biscuit-file/docs/topics/file-references.md` documents `user_cache_dir_from`. Phase 7 still owns the full rewrite

### Departures and decisions

- **Acceptance 1 and 2 are tested at the `WorkspaceFacts` level** (`index_store::workspace::tests::{a_restart_with_unchanged_files_parses_nothing, one_file_changed_between_restarts_is_the_only_parse}`), not through the server. While the full index exists, a server restart parses every closed file for it, and the store pass reuses those parses, so a server-level parse count cannot tell a warm store from a cold one. Once Phase 6 deletes the full build, a server-level "restart parses zero" test becomes meaningful and should be added there
- **"A fallback save re-hashes only the saved file plus files with pending events"**: the pending set is the paths the save's relist reported as joined or left. Paths the full index's own `recheck_parsed` found changed on that save are *not* fed to the store (that would re-hash everything parsed, which is what the amendment removed). The full index is ahead of the store for those until the next rename's full pass; the store answers nothing yet, and in Phase 5 that is the spec's accepted fallback-mode behavior
- the plan's "Unreadable listed file = state failed": a store-pass failure (exists but unreadable, or changed bytes not UTF-8) or a listed file missing on disk fails the build with the same `RequestFailed` message, unless the path changed during the round. A missing path's row is still deleted
- **pre-existing, not changed**: `workspace/willRenameFiles` does not edit a Markdown link that carries a heading fragment (`[x](target.md#target)`); only path links and wiki links are rewritten. Observed while writing acceptance 7's file-rename half; the test uses a path link. Whether this is a defect is outside this feature (the oracle records current behavior)

### Smell grep

`rg 'unwrap_or_default\(\)|\.ok\(\)|filter_map|serde\(default'` over `index_store/workspace.rs`, `router/facts.rs`, `index_store/store.rs`, `index_store/source.rs`:

- `source.rs:156` `MemoryFacts::get_all` (Phase 3, recorded there: a map lookup)
- `router/facts.rs` `finish_round`: `touched.take().unwrap_or_default()` on an `Option<BTreeSet>`: no build recording means nothing was touched; not a parse
- `router/facts.rs` `known_parses`: `filter_map` over `disk_parse(path)`: "no parse held" simply means the pass parses itself; not a decode
- the new `remove_paths` skips a non-UTF-8 path with `let Ok(..) else continue` (such a path can never have been written), not `filter_map(.. .ok())`

No row decode path was added; every store read still goes through Phase 3's decoders.

### Input Robustness Matrix (this phase)

No new loader. `DMLS_CACHE_DIR` reaches `StoreLocation::prepare` (whose shapes Phase 3 tested in-process) through the snapshot; two wiring cells are now covered through the server in-process: `off` (`index_store_live::the_kill_switch_answers_from_memory_and_writes_no_store`: answers, no file in the directory) and a corrupt store file (`…::a_corrupt_store_is_recreated_and_the_request_answers`: answers, three rows after recreate). The one spawned-binary stdio wiring test stays in Phase 5 Wave 13, as the plan schedules it. `user_cache_dir_from`: the injected-environment control per host OS and a relative home (`biscuit-file` `file_reference::context::tests::user_cache_dir::{user_cache_dir_from_reads_only_the_given_environment_and_home, user_cache_dir_from_treats_a_relative_home_as_no_home}`); every other shape is `cache_dir_for`'s, tested in Phase 1

### Requirement-to-test mapping

| Behavior | Test |
| --- | --- |
| R4 / acceptance 1: restart with unchanged files reads and hashes each file once, parses none | `dmls` lib `index_store::workspace::tests::a_restart_with_unchanged_files_parses_nothing` |
| R4 / acceptance 2: one file changed between restarts is the only parse; its row holds the new facts | `…::one_file_changed_between_restarts_is_the_only_parse` |
| R4: a deleted file's row is removed even when the new list omits it; a listed missing file is reported | `…::a_deleted_file_loses_its_row` |
| R4: unreadable (non-UTF-8) file fails with no row and withholds ready; zero-byte file and empty heading title are valid rows, reused on restart | `…::an_unreadable_file_fails_and_empty_files_are_valid_rows` |
| a supplied parse is reused only when its hash matches | `…::a_known_parse_is_reused_only_when_its_hash_matches` |
| cancellation before reading reads and writes nothing | `…::a_cancelled_revalidation_reads_nothing_and_writes_nothing` |
| ready: needs the mark, the same roots, every listed file read; a joined file withholds it until refreshed | `…::ready_needs_a_mark_the_same_roots_and_every_listed_file_read` |
| R6 refresh: re-reads only given paths, re-parses the changed one, deletes the missing one, keeps ready | `…::a_refresh_updates_one_row_and_deletes_a_missing_one` |
| R5: unsaved and saved-but-differing buffers replace rows; no buffer writes a row (row hashes unchanged) | `…::open_buffers_replace_rows_and_never_write_one` |
| R8: a file under nested roots has a row in each store; restart reuses both | `…::a_file_under_nested_roots_has_a_row_in_each_store` |
| failure to memory after ready: the refresh in flight completes, written facts kept, ready withdrawn, next revalidation refills | `…::a_store_failure_after_ready_switches_to_memory_without_losing_written_facts` |
| `off`: memory, nothing written | `…::disabled_stores_keep_facts_in_memory` |
| wiring: no store before the first need; first references fills one row per listed file with its disk hash | `dmls` `l1` `index_store_live::the_first_references_request_fills_the_store_from_disk` |
| wiring: `DMLS_CACHE_DIR=off` answers, creates no store file | `index_store_live::the_kill_switch_answers_from_memory_and_writes_no_store` |
| wiring: corrupt store file recreated, request answers | `index_store_live::a_corrupt_store_is_recreated_and_the_request_answers` |
| R6 before ready: a watched change opens no store; the first need reads the new bytes | `index_store_live::before_ready_a_watched_change_touches_only_the_file_list` |
| R6 after ready, client watching: one event updates one row with one parse (shared with the index); a deletion removes the row | `index_store_live::after_ready_a_watched_event_updates_one_row` |
| R7 reconciliation: a change during a held build round is re-read before ready | `index_store_live::a_change_during_a_build_round_is_reread_before_ready` |
| R6 fallback save re-reads only the saved file (an unannounced change keeps its old row) | `index_store_live::a_fallback_save_rereads_only_the_saved_file` |
| R6 / acceptance 7: fallback mode, a referrer added on disk without an event is edited by heading rename and by file rename | `index_store_live::a_fallback_rename_edits_a_referrer_added_on_disk_without_an_event` |
| R6: a rename edits the new bytes of a referrer changed after ready (link moved from line 2 to 4, no event) | `index_store_live::a_rename_edits_the_new_bytes_of_a_referrer_changed_after_ready` |
| acceptance 9 | Phase 3: `index_store_concurrency::two_processes_cold_filling_one_new_store_reach_identical_rows` |
| `user_cache_dir_from` | biscuit-file `user_cache_dir::user_cache_dir_from_*` (2 tests) |

Mutation check: disabling the fallback pre-rename build and the referrer refresh makes exactly `a_fallback_rename_edits_…` and `a_rename_edits_the_new_bytes_…` fail; both pass with the code restored.

### Gates and downstream tests

- the gate-skip calculator was not consulted for skipping: the phase changed source files, so every gate ran
- scoped during the work: `cargo nextest run -p dmls -E 'test(index_store::) | test(full_index) | test(workspace_file_list) | test(startup_log) | test(router::)'` (112 passed) and `-E 'test(index_store_live)'` (9 passed, each under 0.05 s)
- `just test` in `darkmatter/`: **10441 passed, 10 skipped, 0 failed** (Phase 3: 10420; +21 new)
- `just lint` in `darkmatter/`: exit 0 (one `clippy::cloned_ref_to_slice_refs` in a new test fixed first)
- `just test` in `biscuit-file/`: 1142 passed, 0 failed; `just lint` in `biscuit-file/`: exit 0
- `just check-tier-coverage darkmatter` and `biscuit-file`: 0 stranded. No new test name carries a tier marker; `index_store_live` is declared in `tests/l1/main.rs`, `workspace_tests.rs` is a `#[path]` unit module
- `just test-l2` not run: no L2 test was touched
- downstream: no crate depends on `dmls`. The biscuit-file change is one additive function and re-export; the darkmatter area (its consumer here) passed in full
- scope calculator (`git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`): `packages` = `biscuit-file`, `dmls`, `test-toolkit`, `darkmatter`, `darkmatter-cli`; `change_class` = `package`. The darkmatter packages ran in the area gate; `cargo nextest run -p test-toolkit`: 475 passed, **2 failed, both pre-existing and unrelated**: `ci_workflow_contracts::just_commit_is_deterministic_under_ci` (the root `justfile`'s `commit` recipe has no `CI` branch) and `ci_workflow_contracts::the_ci_documentation_states_the_implemented_behavior` (`.claude/skills/os/ci-evidence.md` does not state `BISCUIT_CI_CONSTRAINTS_DIR`). This phase touched neither file
- no other skipped or pre-existing failures
- cross-OS: `just cross-check dmls --os windows` **pass** (1187 tests, 9 skipped, including all 21 new ones) and `just cross-check biscuit-file --os windows` **pass**, both on the first attempt. Linux/WSL not run separately: the new code has no `cfg` branch; its OS risk was path spelling between URI-derived roots and test paths, which Windows exercises

Phase 4 implementation complete, ready for review.

## Phase 5

Implemented directly by the orchestrator, without subagents. Wave 11's builder decides what Wave 12's providers see, and the providers turned out to need almost no change (each already takes the graph it answers from), so one agent kept the design in one place.

### What changed

- **`WorkspaceFacts` request side** (`index_store/workspace.rs`):
    - `candidates_for(targets, listed)`: the union over every root's `RootFacts::candidates_for`, filtered to the listed files (a row another process wrote for a file this one does not list is never a candidate, R11), de-duplicated by path (ruling 15; file-list paths are already canonical-simplified, ruling 10)
    - `request_graph(targets, listed, overlay, universe, contexts)`: candidates ∪ listed targets → `request_documents` (stored facts, overlay in place) → `WorkspaceGraph::build_with_roots`. Dropped after the request; nothing is cached across requests except what was already there (the dependency's lazy target cache in `WorkspaceIndex`)
    - `symbol_documents(listed, overlay)`: `scan_headings` of every root, listed paths only, once per path, overlay buffers in place; no graph
    - every read runs `take_switched`, so a store failure mid-request withdraws ready and the request still answers from what memory salvaged
- **Router** (`router.rs`, `router/facts.rs`):
    - `ServerState::request_graph(targets)`: overlay = every open buffer plus the `disk_parse` of each target outside the file list (a referenced dependency); universe = the live snapshot's `shared_wiki_universe()` (new accessor on `WorkspaceGraph`, an `Arc` clone), which is the file list plus open buffers under the wiki roots, exactly what the full graph resolves against; contexts = the shared `RepositoryContexts` (reused across requests, as S1 recommended)
    - references: targets = the document and `providers::reference_target_at` under the cursor (now `pub`), answered through the new `with_document_over(uri, graph, f)`. This covers heading backlinks, link targets, wiki references, and DSL transcluded-by with no provider change
    - heading rename: targets = the document; file rename: targets = the renamed paths. Both build the request graph after `refresh_rename_referrers`
    - `refresh_rename_referrers` now re-reads the closed targets plus the closed **candidates** (`candidates_for`), replacing Phase 4's `referrers_of` over the live snapshot (deleted)
    - workspace symbols: `ServerState::symbol_documents()` → `registry.workspace_symbols(query, &documents)`
- **Providers**: `Provider::workspace_symbols` and `ProviderRegistry::workspace_symbols` take `&[SymbolDocument]` (`(PathBuf, Vec<HeadingFact>)`) instead of a graph. `symbols::workspace_symbols` keeps the title rule (first level-1 heading, else first heading, else file name), the subsequence match, title at line 1, headings at their line with the file name as container. `symbols::graph_symbol_documents(graph)` adapts a graph (non-dependency documents, as before) for the oracle. `document_title` shares the new `title_of`. `rename.rs` changed only in comments (`RenameEnv::graph` is now the request graph)
- **Oracle** (`tests/common/oracle.rs`): `references_over`, `rename_over`, `will_rename_files_over`, `wiki_diagnostics_over`, `workspace_symbols_over`, `contexts`, `file_list_universe`, `position_at`, `offset_at`; the existing methods delegate with the full graph
- **Docs**: `darkmatter/docs/lsp/indexing.md` no longer says the store answers nothing; new "Answering a request from the store" section (rule list, example, sequence diagram); the rename re-read paragraph names candidates; the prerequisites table names the store. Skill: `.claude/skills/darkmatter/dmls.md`

### Departures and decisions

- **Off the request thread.** The plan says to run store and parse work off the request thread "per the existing router pattern". Parsing and revalidation stay on the build round's thread (Phase 4). The per-request work (one SQL candidate query per root and target, loading about a hundred rows, building a small graph) runs on the dispatch thread inside the handler, as the rename providers' own file reads already do. Measured on this repository (below): 10 ms per references request, 54 ms per workspace-symbols request. Moving it to a worker would need asynchronous answering for these four methods; not worth it at these figures. `2026-10-11-dmls-request-timeouts` is still a draft spec with no timeout defined, so there is no interaction to honor; nothing was changed for it
- **The full index still gates these requests** (Phase 6 removes it). The requests now *answer* from the store; `proceed` still waits for the full build plus a ready store. Mutation check: making `request_graph` ignore candidates fails 18 tests, including every `index_store_sessions` and `index_store_stdio` test and the Phase 4 `index_store_live` tests, so the server really answers from the request graph, not the snapshot
- **Acceptance 3 and 6 are tested through the server across a restart**, but while the full index exists a restart still parses everything for it. The store answers are what is asserted; a "warm restart parses zero" server test belongs to Phase 6 (as Phase 4 recorded)
- **Pre-existing, not changed: a `../` link out of a workspace folder that is in no repository does not resolve**, with or without the store (verified by answering the same session from the live snapshot: the same empty result). The two-root test therefore makes the folders two directories of one repository, which is the realistic multi-root shape. The corpus's `other` root is a separate repository and reaches `ws` only by wiki link
- **Corpus breadth**: only one corpus heading is referenced from another file (`docs/b.md`'s "Some Heading", by path link, bare root-relative link, and wiki heading link). The corpus is pinned by the golden-facts snapshot and the oracle self-check, so it was not extended; the server sessions add heading-rename referrers (Markdown fragment and wiki heading spelling classes)
- the request graph's `generation` is `0`: nothing reads a request graph's generation (it stamps the live snapshot for supersession checks)

### Smell grep

`rg 'unwrap_or_default\(\)|\.ok\(\)|filter_map|serde\(default'` over `index_store/workspace.rs`, `router/facts.rs`, `providers/symbols.rs`:

- `symbols.rs` `filter_map(|(_, node)| node.as_heading())` (twice, plus the pre-existing one in `document_symbols`): heading nodes are exactly the nodes with a heading payload; not a decode
- `router/facts.rs` `request_graph`: `filter_map(|target| index.disk_parse(target))`: an unlisted target the index has not read is simply not in the overlay; not a decode
- `router/facts.rs` `finish_round` and `known_parses`: Phase 4, recorded there

No loader or decode path was added.

### Input Robustness Matrix (this phase)

No new loader or configuration field. `DMLS_CACHE_DIR` is now proven through the spawned binary (the stdio wiring test the plan owed): the process environment's `off` reaches the server (memory, one `warn` in the log file, no `.sqlite` under the process root) and an absolute directory holding a corrupt store is honored (one `index store recreated`, no fallback `warn`, two rows after). Every other value shape is `StoreLocation::prepare`'s, tested in Phase 3.

### Requirement-to-test mapping

| Behavior | Test |
| --- | --- |
| acceptance 4 / R3: references (heading backlinks, link targets, wiki backlinks, DSL transcluded-by), with and without the declaration, equal the oracle at every heading, link, wiki link, and transclusion of every corpus file; every probe kind found a cross-file referrer | `dmls` `l1` `index_store_requests::references_equal_the_oracle_at_every_probe` |
| acceptance 4: heading rename equals the oracle (edits or refusal) at every heading | `…::heading_renames_equal_the_oracle_at_every_heading` |
| acceptance 4: file rename equals the oracle for every file renamed in place, into a new directory, and onto another listed file (refusal); wiki spellings included; a directory rename makes no edit | `…::file_renames_equal_the_oracle_including_wiki_spellings` |
| acceptance 4: workspace symbols equal the oracle for seven queries; R8: a nested-root file listed once | `…::workspace_symbols_equal_the_oracle_and_list_a_nested_file_once` |
| R3: wiki diagnostics in a request graph equal the oracle's for every file; `[[topic]]` stays ambiguous though neither rival is in the graph | `…::wiki_ambiguity_is_judged_against_the_whole_file_list` |
| acceptance 5 / R2: for every target, candidates ⊇ every oracle referrer (≥ 10 pairs), including the bare root-relative `docs/a.md` | `…::candidates_never_miss_an_oracle_referrer` |
| R11: a row outside the file list is not a candidate | `…::a_row_outside_the_file_list_is_not_a_candidate` |
| acceptance 8: `off` → all four requests equal the oracle from memory, one `warn`, no store directory | `…::with_the_stores_off_every_request_answers_from_memory` |
| acceptance 8: read-only store directory (`cfg(unix)`) → same, one `warn`, nothing written | `…::with_a_read_only_store_directory_every_request_answers_from_memory` |
| acceptance 8: corrupt and locked (another holder's shared lock) → same, one `warn`, files untouched | `…::with_corrupt_stores_held_by_another_process_every_request_answers_from_memory` |
| corrupt, nobody holds it → recreated (three `index store recreated`, no fallback), answers equal | `…::with_corrupt_stores_nobody_holds_the_stores_are_recreated_and_answer` |
| acceptance 3: after a restart, references, heading rename (Markdown slug and wiki heading text), and file rename (path link, wiki re-spelling) edit referrers never open and not referenced by an open document | `index_store_sessions::after_a_restart_renames_edit_a_referrer_never_open` |
| acceptance 6: two roots, after a restart, a file rename in one edits a path link and a wiki link in the other | `index_store_sessions::after_a_restart_a_file_rename_edits_a_referrer_in_the_other_root` |
| acceptance 6: nested roots, references and workspace symbols list each result once | `index_store_sessions::with_nested_roots_no_result_appears_twice` |
| acceptance 8 wiring: spawned `dmls`, `DMLS_CACHE_DIR=off` from the process environment: four requests answer, one `warn`, no store file | `index_store_stdio::the_binary_answers_from_memory_when_the_stores_are_off` |
| acceptance 8 wiring: spawned `dmls`, corrupt store recreated once, four requests answer, rows filled | `index_store_stdio::the_binary_recreates_a_corrupt_store_and_answers` |
| acceptance 7 (fallback rename freshness), now through candidates | Phase 4 `index_store_live::a_fallback_rename_edits_a_referrer_added_on_disk_without_an_event`, `…::a_rename_edits_the_new_bytes_of_a_referrer_changed_after_ready` (still green after `referrers_of` was replaced) |

Mutation checks: (1) `request_graph` without candidates fails 18 tests (above); (2) a request graph whose wiki universe is only its own documents fails 7 `index_store_requests` tests, including the wiki-ambiguity, file-rename-spelling, references, and every memory-mode test. Both restored and re-run green.

None of the new tests spawns more than two processes (`index_store_stdio`, one `dmls` each) or opens a window; none carries a tier marker; all are declared in `tests/l1/main.rs`. Each `index_store_requests` test keeps its corpus path literal inside the `#[test]`.

### Measurement sample (Validation checkpoint 5; full numbers are Phase 7's)

Release `dmls` over stdio on this worktree (5,709 Markdown files, 75,650 headings), `DMLS_CACHE_DIR` in `/tmp`, references on `## The index store` in `darkmatter/docs/lsp/indexing.md`. **Host loaded: load average 25-31** (other sessions' builds).

| | Cold store | Warm restart |
| --- | --- | --- |
| first references (full-index build plus store pass) | 9,084 ms | 8,773 ms |
| `full index built … elapsed_ms` | 8,803 | 8,504 |
| second and third references (request graph only) | 10 ms, 10 ms | 10 ms, 10 ms |
| workspace symbols, query `index store` (57 results) | 54 ms | 54 ms |
| process peak RSS | 1,122 MB | 1,099 MB |

The first request and the peak RSS are still the full build's; they say nothing about acceptance 11 until Phase 6 removes it. The store-backed answers themselves are well inside ruling 6's 500 ms.

### Gates and downstream tests

- the gate-skip calculator was not consulted for skipping: the phase changed source files, so every gate ran
- scoped during the work: `cargo nextest run -p dmls` (1189 passed after Waves 11-12), `-E 'test(index_store_)'` (65 passed)
- `just test` in `darkmatter/`: **10457 passed, 10 skipped, 0 failed** (Phase 4: 10441; +16 new)
- `just lint` in `darkmatter/`: exit 0
- `just check-tier-coverage darkmatter`: 0 stranded
- `just test-l2` not run: no L2 test was touched
- downstream: no crate depends on `dmls` (the changed public items, `Provider::workspace_symbols`, `ProviderRegistry::workspace_symbols`, `providers::reference_target_at`, `WorkspaceGraph::shared_wiki_universe`, the `WorkspaceFacts` methods, are used only inside the `dmls` package and its tests). biscuit-file was not touched this phase
- scope calculator (`git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`): `packages` = `biscuit-file`, `dmls`, `test-toolkit`, `darkmatter`, `darkmatter-cli`; `change_class` = `package`. The working tree still holds Phase 4's uncommitted changes, which is why `biscuit-file` appears; this phase did not touch it (Phase 4 ran its gates). The darkmatter packages ran in the area gate. `cargo nextest run -p test-toolkit`: 475 passed, **2 failed, the same two pre-existing failures Phase 4 recorded**: `ci_workflow_contracts::just_commit_is_deterministic_under_ci` (the root `justfile`'s `commit` recipe has no `CI` branch) and `ci_workflow_contracts::the_ci_documentation_states_the_implemented_behavior` (`.claude/skills/os/ci-evidence.md` does not state `BISCUIT_CI_CONSTRAINTS_DIR`). This phase touched neither file
- cross-OS: `just cross-check dmls --os windows` **pass** on the first attempt (1202 passed, 9 skipped; every new test except the `cfg(unix)` read-only one ran there, including the spawned-binary and lock-contention tests). Linux and WSL were not run separately: no new `cfg` branch beyond the Unix-only test, and the OS risk (URI-derived roots against test paths, spawned binary, Windows lock semantics) is what Windows exercises
- no other skipped or pre-existing failures

Phase 5 implementation complete, ready for review.
