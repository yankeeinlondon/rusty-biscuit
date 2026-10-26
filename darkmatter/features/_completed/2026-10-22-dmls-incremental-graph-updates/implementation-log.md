---
spec: "/Volumes/coding/wt/rusty-biscuit/feat-merman/darkmatter/features/2026-10-22-dmls-incremental-graph-updates/spec.md"
plan: "darkmatter/features/2026-10-22-dmls-incremental-graph-updates/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
source_files_during_phase_1:
  - darkmatter/dmls/src/work_counters.rs
  - darkmatter/dmls/src/wiki/universe.rs
  - darkmatter/dmls/src/graph/arena.rs
  - darkmatter/dmls/src/graph/invalidate.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages:
  - dmls
decisions_for_review:
  - |-
    The document-table counter is recorded from Phase 1 on. The plan adds the
    counter in Phase 1 and the table in Phase 2, so the helper had no caller
    yet; an unused helper fails `-D warnings`. I record one table per graph
    assembly, which is what every build constructs today, so the counter is
    accurate now and a content edit reads 1 until Phase 2 makes it 0. The
    alternative was a temporary `#[allow(dead_code)]`. If the reviewer
    prefers that, the call goes in Phase 2's table constructor instead (a
    one-line move).
  - |-
    The plan says a non-dependency document takes its canonical facts from
    its universe slot. A request graph built over the live universe can hold
    a live dependency as an ordinary document, and its slot there is
    unmatchable (empty canonical). Reading facts from that slot would have
    emptied the record's canonical path. Grouping by the slot's empty key
    would also have made unrelated dependencies "collide". So a document
    whose slot is unmatchable takes `WikiDoc::for_path` facts, and its
    portability key is computed from those real facts. That is exactly the
    behavior before this change. Live graphs never take this path (their
    candidates always have matchable slots), so content edits still compute
    zero keys. The fallback key computation is counted in
    `wiki_portability_keys` too, so the counter reports every key computed.
    The alternative, reading the slot unconditionally, changes request-graph
    collisions and rename spellings for dependency targets. Undoing this
    decision would mean deleting the fallback arms in `assemble_listed` and
    `portability_collisions`.
  - |-
    The plan gives `build_listed_reusing` "the same arguments" as
    `build_listed` plus `previous`. Its universe parameter is a concrete
    `Arc<WikiUniverse>` rather than `impl Into<Arc<WikiUniverse>>`. Table
    reuse compares the universe by pointer identity, and only an `Arc` the
    caller keeps can ever match; a converted value would always be new. The
    only caller (the live index) already holds an `Arc`. Changing it back to
    `impl Into` is a one-line signature change.
  - |-
    The plan calls `GraphAssembly`, `assemble`, and `finalize` crate-visible.
    Nothing outside `graph/arena.rs` calls them, and they now take the
    private table, so they are private. The two passes, assembly and then
    reverse-index finalization, are kept. Making them `pub(crate)` again
    would also mean exposing `DocumentTable`.
  - |-
    Opening a referenced dependency's buffer with byte-identical text
    returns `Unchanged` without rebuilding. The snapshot then still marks
    the document as a dependency, so it is not a wiki candidate, until the
    next rebuild. This behavior predates this change. Plan rule 12 says to
    keep the unchanged-content short-circuit, so I left it alone; the tests
    open dependencies with different text. The fix would be to rebuild in
    `open_buffer` when the path was a dependency before it was opened, even
    for unchanged text. That is a small, separate change to publication
    behavior, which a reviewer may want to schedule.
  - |-
    Phase 3 had not been implemented when this final phase started: the log
    had no Phase 3 section, and both of its tasks (fresh-graph equivalence
    tests and server-session tests) were unchecked, with no test file
    present. The plan treats Phase 4 as the close of the whole plan, so this
    run implemented Phase 3's two tasks first, then Phase 4's. Their files
    are listed under the Phase 4 file lists, because no Phase 3 run produced
    them. The alternative was to fail this phase and wait for a Phase 3
    rerun; nothing in Phase 4 depends on how Phase 3 is attributed. If the
    reviewer wants separate attribution, only the frontmatter lists move.
  - |-
    The server-session test for build output (an unreferenced creation
    under `target/`) proves zero graph path work, zero parses and reads,
    and unchanged diagnostics. It cannot prove "no new snapshot
    publication", because a session cannot see the snapshot generation and
    the router re-sends diagnostics after every watched change. The
    in-crate unit test `test_only_an_awaited_unlisted_creation_rebuilds`
    already proves the generation does not move. Proving it in a session
    would need a production hook exposing the generation, which this
    graph-only change does not add.
  - |-
    Measurement: comparing the cached wiki universe's inputs is now the
    largest single rebuild stage (about 1 ms, roughly 36% of rebuild CPU in
    the sample), because it clones and compares about 5,900 candidate paths
    on every rebuild. The plan says a result over or under the bounds
    authorizes no further optimization, and the sample met both bounds
    (0.78% of one core; 2.6 ms median), so it was left alone. A cheaper
    comparison (for example a generation number on the listing and open
    set) would be a follow-up of its own.
source_files_during_phase_2:
  - darkmatter/dmls/src/graph/arena.rs
  - darkmatter/dmls/src/graph/invalidate.rs
  - darkmatter/dmls/src/providers/completion.rs
  - darkmatter/dmls/src/providers/definition.rs
  - darkmatter/dmls/src/providers/dsl.rs
  - darkmatter/dmls/src/providers/frontmatter.rs
  - darkmatter/dmls/src/providers/location.rs
  - darkmatter/dmls/src/providers/mod.rs
  - darkmatter/dmls/src/providers/rename.rs
  - darkmatter/dmls/src/providers/symbols.rs
  - darkmatter/dmls/src/providers/wiki.rs
  - darkmatter/dmls/tests/l1/entry_point_parity.rs
  - darkmatter/dmls/tests/l1/index_store_keys.rs
  - darkmatter/dmls/tests/l1/index_store_requests.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_4:
  - darkmatter/dmls/tests/l1/incremental_graph_equivalence.rs
  - darkmatter/dmls/tests/l1/main.rs
  - darkmatter/dmls/tests/l1/workspace_file_list.rs
  - darkmatter/features/2026-10-22-dmls-incremental-graph-updates/measure-live-rebuild.py
docs_updated_during_phase_4:
  - darkmatter/docs/lsp/architecture.md
  - darkmatter/docs/lsp/indexing.md
docs_created_during_phase_4:
  - darkmatter/features/2026-10-22-dmls-incremental-graph-updates/measurement.md
skills_files_updated_during_phase_4:
  - .claude/skills/darkmatter/dmls.md
source_code:
  - darkmatter/dmls/src/work_counters.rs
  - darkmatter/dmls/src/wiki/universe.rs
  - darkmatter/dmls/src/graph/arena.rs
  - darkmatter/dmls/src/graph/invalidate.rs
  - darkmatter/dmls/src/providers/completion.rs
  - darkmatter/dmls/src/providers/definition.rs
  - darkmatter/dmls/src/providers/dsl.rs
  - darkmatter/dmls/src/providers/frontmatter.rs
  - darkmatter/dmls/src/providers/location.rs
  - darkmatter/dmls/src/providers/mod.rs
  - darkmatter/dmls/src/providers/rename.rs
  - darkmatter/dmls/src/providers/symbols.rs
  - darkmatter/dmls/src/providers/wiki.rs
  - darkmatter/dmls/tests/l1/entry_point_parity.rs
  - darkmatter/dmls/tests/l1/index_store_keys.rs
  - darkmatter/dmls/tests/l1/index_store_requests.rs
  - darkmatter/dmls/tests/l1/incremental_graph_equivalence.rs
  - darkmatter/dmls/tests/l1/main.rs
  - darkmatter/dmls/tests/l1/workspace_file_list.rs
  - darkmatter/features/2026-10-22-dmls-incremental-graph-updates/measure-live-rebuild.py
documentation:
  - darkmatter/docs/lsp/architecture.md
  - darkmatter/docs/lsp/indexing.md
  - darkmatter/features/2026-10-22-dmls-incremental-graph-updates/measurement.md
  - .claude/skills/darkmatter/dmls.md
completed_phase: 4
implemented: true
---

# Implementation Log for 2026-10-22-dmls-incremental-graph-updates (4 phases)

## Phase 1

### Work done

- **Count graph work.** `dmls/src/work_counters.rs` gained `wiki_universes`,
  `wiki_path_facts`, `wiki_portability_keys`, and `document_tables` on
  `WorkCounts`, in `since` and `work_counts`, with the crate-private helpers
  `record_wiki_universe`, `record_wiki_path_facts`,
  `record_wiki_portability_key`, and `record_document_table`. The unit test
  covers every helper and the subtraction between two later snapshots.
- **Cache wiki facts.**
  - `wiki/universe.rs`: `WikiUniverse` keeps a `portability_keys` slot array,
    filled in `push` (an unmatchable slot gets the empty key) and read with
    `portability_key(slot)`. `WikiUniverse::new` counts a universe,
    `WikiDoc::for_path` counts path facts, and the key computation counts
    through `counted_portability_key`. `WikiDoc::is_unmatchable` identifies
    an unmatchable slot (no canonical segments; every real path has at least
    one).
  - `graph/invalidate.rs`: private `WikiUniverseInputs` (sorted/unique
    candidates, sorted dependencies, roots in order) and `CachedWikiUniverse`;
    `WorkspaceIndex::wiki_cache`. `rebuild` asks `wiki_universe`, which
    returns the cached `Arc` when the inputs compare equal, and otherwise
    builds `WikiUniverse::new(candidates)` and appends each dependency as
    an unmatchable slot before wrapping it.
  - `graph/arena.rs`: `assemble_listed` takes a candidate's canonical facts
    from its slot. A dependency, or a document whose supplied slot is
    unmatchable, gets `WikiDoc::for_path`. A missing slot is computed and
    appended as before. `portability_collisions` groups only the graph's
    non-dependency documents by their slot's cached key, and falls back to
    keying the record's canonical path when the slot is unmatchable.
    `record_document_table` is counted once per assembly.
  - The stale docs on `GraphAssembly`, `assemble`, and `finalize` claimed
    the bench harness times those passes; nothing calls them outside
    `arena.rs` (`bench.rs` measures store operations). They now describe the
    real assembly/finalization boundary. The `WikiSlots` invariant doc now
    covers prepopulated dependency slots.

### Lookup terminology correction (plan rule 6)

The spec calls the shared lookup a "frontmatter-key lookup". The lookup that
is path-derived, and so shareable, is the **wiki basename** `KeyIndex`
(`graph/key_index.rs`, built from non-dependency paths in `finalize`).
`FrontmatterKey` nodes and `key_by_name` come from parsed content and are
rebuilt on every edit. The spec text is left as written. The key-renaming
test `test_a_renamed_frontmatter_key_re_resolves_its_variable_uses` guards
against a cached key map.

### Requirement-to-test mapping

| Requirement | Test |
|---|---|
| Counters, `since`, snapshot | `work_counters::tests::each_helper_counts_its_own_work` |
| Slot keys computed once, aligned across `new`/`push`/clone/default; unmatchable key | `wiki::universe::tests::test_every_slot_carries_its_portability_key_once` |
| Content edit reuses the universe while an old snapshot is held (`Arc::ptr_eq`, no `make_mut` copy); zero universe/key work; dependency prepopulated last, unmatchable; dependency record keeps real facts; old reader unchanged; dependency-to-listed builds one universe | `graph::invalidate::tests::test_a_content_edit_reuses_the_universe_a_retained_snapshot_shares` |
| Input equality: same list, listing-state-only change, listed insertion, open-unlisted join/edit/leave, equal-root setter, root reorder (first root owns) | `graph::invalidate::tests::test_each_universe_input_change_builds_one_universe_and_nothing_else_does` |
| Collisions only over graph documents, keyed from slots, no path facts or keys recomputed | `graph::arena::tests::test_collisions_group_only_graph_documents_by_their_cached_slot_keys` |
| Request graph over a live dependency's unmatchable slot keeps real canonical facts and collisions | `graph::arena::tests::test_a_document_on_an_unmatchable_supplied_slot_keeps_its_real_facts` |
| Frontmatter keys rebuilt on edit (rule 6) | `graph::invalidate::tests::test_a_renamed_frontmatter_key_re_resolves_its_variable_uses` |
| Request-graph universe behavior unchanged | existing `wiki_universe_equivalence` (6 tests) |

All of these are L1 tests at the crate API, fidelity F1: the boundary is
in-process graph construction, and no CLI, terminal, or persisted format is
touched. The Input Robustness Matrix does not apply (plan rule 14: no
loader changes).

Focused checkpoint: `cargo nextest run -p dmls -E 'test(wiki::universe) |
test(graph::invalidate) | test(graph::arena) | test(work_counters) |
test(wiki_universe_equivalence)'` ran 57 tests; all 57 passed.

### Environment note

Another session was editing this worktree at the same time (the
frontmatter-key-order fix: `darkmatter/lib/**`, `darkmatter/dmls/src/overlay/mod.rs`,
`biscuit-hash/**`). My first build caught that session's edit halfway
through and failed in `darkmatter/lib`; the next build succeeded. I
touched none of those files, and none of them are listed as this phase's
sources.

### Gates

- `just lint` (darkmatter, which covers the lib, the CLI, `dmls`, and
  `zed-dmls`) passed.
- `just check-tier-coverage darkmatter`: 0 stranded tests.
- `just test` (darkmatter) ran 10620 tests: 10576 passed, **44 failed**,
  10 skipped (logged in `/tmp/dm-phase1-test.log`). The first run stopped
  at the first failure; I reran with `--no-fail-fast` to list every
  failure. None of the 44 are in `dmls`. All are in `darkmatter` (lib) and
  `darkmatter-cli`: `schemas_convert_snapshots` (40),
  `fn_list_formatting::as_json5_uses_the_formatter_object_idiom`,
  `frontmatter::tests::parsed_frontmatter_retains_nested_mapping_order_as_metadata`,
  `suggest_constraint_phase3`, and
  `schema_validate_baseline::schema_validate_legacy_json_output_is_byte_identical`.
  Each is a JSON/YAML **key-order** difference. Run alone,
  `as_json5_uses_the_formatter_object_idiom` fails the same way: it expects
  `{'dev deps': …, package: …}` and gets `{package: …, 'dev deps': …}`.
  That is exactly the behavior the concurrent frontmatter-key-order session
  is changing in its uncommitted edits to `darkmatter/lib` and
  `biscuit-hash`. `cargo tree` shows neither `darkmatter` nor
  `darkmatter-cli` depends on `dmls`, so this phase's edits cannot reach
  them. I could not run them on a clean tree, because that would mean
  stashing the other session's files. I did not touch them.
- Every `dmls` test passed in the area run.
- Dependent packages: `cargo tree -i dmls` shows nothing depends on `dmls`,
  so there was nothing downstream to test.
- Affected-scope check (`git status … | xargs python3
  scripts/ci/affected_scope.py`): `packages` = biscuit-hash, darkmatter,
  dmls, test-toolkit, claudine, darkmatter-cli, sniff; `change_class` =
  package. The gates were required, and I ran them. The list is wide
  because the other session's uncommitted files share this working tree.
- Cross-OS: no OS-specific code was added. Production compares
  `Path`/`PathBuf` values only; tests use `abs()` logical paths or
  `canonicalize_simplified` temp roots. I did not run a `just cross-check`;
  CI covers the other operating systems.


## Phase 2

### Work done

- **Split snapshot state** (`dmls/src/graph/arena.rs`).
  - New private `DocumentTable` holds the path-derived part of a snapshot:
    `DocumentPathRecord { path, canonical, workspace_id, dependency,
    wiki_slot }` in path order, `by_path`, `slot_documents`, the wiki
    **basename** `KeyIndex`, portability collisions, the universe `Arc`,
    and `Listing`. `WorkspaceGraph` holds `Arc<DocumentTable>` plus a dense
    `Vec<DocumentState { content_hash, parsed }>`, nodes, edges, the
    reverse index, the generation, and awaited paths. The old `WikiSlots`
    struct and the owned `documents`/`by_path`/`keys`/`portability`/
    `listing` fields are gone.
  - `DocumentTable::build` is the only place that counts
    `document_tables`, computes dependency path facts, appends to a supplied
    universe that lacks a document (through `Arc::make_mut`), and groups
    collisions. The Phase 1 fallback is kept: a document whose supplied
    slot is unmatchable gets `WikiDoc::for_path` facts and a computed key.
    A request graph's table keeps the universe as finally extended.
  - `DocumentTable::matches` merges the sorted parsed and listed paths into
    a temporary vector of references, then compares paths and dependency
    flags record by record. It also compares `Listing` by equality and the
    universe by `Arc::ptr_eq`. It never builds records, maps, canonical
    strings, or collision buckets just to compare them.
  - `build`, `build_with_roots`, and `build_listed` keep their signatures
    and always build a new table. The crate-private `build_listed_reusing`
    adds `previous: &WorkspaceGraph`, and only `WorkspaceIndex::rebuild`
    calls it. Both routes go through one `build_from(…, Option<&WorkspaceGraph>)`.
  - Assembly allocates every root first (`DocumentId(d)` → root `NodeId(d)`).
    It then appends content for each parsed index in `BTreeMap` order, in
    the same node sequence per document as before, and fills each document's
    state by ID. Heading facts exist only for parsed documents; a missing
    entry reads as `HeadingMissing`, and the explicit parsed flag still
    leaves unparsed targets unchecked. Frontmatter keys, `key_by_name`,
    headings, references, wiki outcomes, reverse edges, and awaited paths
    are rebuilt for every snapshot.
  - `ReferenceResolver`, `resolve_link`, `resolve_file_edge`, and
    `resolve_wiki_link` read the table and the state slice, and derive a
    document's root from its ID.
  - `DocumentRecord<'a>` is now a `Copy` view (`path: &PathBuf`,
    `canonical: &Vec<String>`, the same scalar fields).
    `document(id) -> Option<DocumentRecord<'_>>`;
    `documents() -> impl ExactSizeIterator + DoubleEndedIterator`.
  - The module docs on `arena.rs` and `invalidate.rs` now describe table
    sharing.
- **Callers** (rule 8): `.iter()` was removed after `documents()` in
  `providers/{completion,frontmatter,symbols,wiki}.rs` and
  `tests/l1/entry_point_parity.rs`. Owned paths taken from a record now use
  `.to_path_buf()` (`providers/{mod,definition,symbols}.rs`,
  `graph/invalidate.rs`, three L1 tests). Clippy's `needless_borrow`
  flagged the `&record.path` arguments in `providers/{dsl,frontmatter,
  rename,completion,symbols,location}.rs`, which now pass `record.path`
  directly. `tests/l1/index_store_keys.rs` used to borrow a field of a
  temporary record, and now copies the view. `graph/node.rs` and the
  `invalidate.rs` rustdoc links to `DocumentRecord::dependency` still
  resolve, so they are unchanged.
- **Phase 1 test updated:**
  `test_a_content_edit_reuses_the_universe_a_retained_snapshot_shares` now
  expects path facts, keys, universes, and tables all to be 0 on a content
  edit (previously path facts were 1).

### Departures and findings

- `build_listed_reusing` takes `Arc<WikiUniverse>` rather than
  `impl Into`, and `assemble`, `finalize`, and `GraphAssembly` are now
  private. Both are recorded in `decisions_for_review`.
- A pre-existing quirk turned up during testing. Opening a dependency's
  buffer with byte-identical text short-circuits as `Unchanged`, so the
  snapshot keeps the document flagged as a dependency until the next
  rebuild. Rule 12 says to keep publication semantics, so I did not change
  it; it is recorded in `decisions_for_review`.

### Requirement-to-test mapping

All tests are L1, fidelity F1: the boundary is in-process graph
construction inside `dmls`. Table identity is private, so the tests live in
the crate and use the `#[cfg(test)]` probe
`WorkspaceGraph::shares_document_table`. No loader is involved, so the
Input Robustness Matrix does not apply (rule 14).

| Requirement | Test |
|---|---|
| Content edits (heading, link retarget, frontmatter-key rename) with a parsed source, a parsed dependency, and three unparsed listed docs share the table and universe; zero deltas on all four counters; hash and parsed state updated; link and `UsesVariable` targets rebuilt; roots-first per document; old snapshot unchanged | `graph::invalidate::tests::test_content_edits_share_the_document_table_and_rebuild_every_reference` |
| Listed insertion and removal, ordered-root change, open-unlisted join and leave, dependency addition, dependency → open, open → dependency after preparation, dependency removal, dependency → listed: exactly one universe and one table each; path facts and keys = candidate slots + dependency records | `graph::invalidate::tests::test_each_structural_change_builds_one_universe_and_one_table` |
| Listed unparsed → parsed, and a listed buffer closed back to unparsed, reuse the table while state changes; a listing-state change alone gives one table and no universe; equal roots, relink, and a repeated listing each publish a new generation over the same table; an awaited creation re-resolves the link (`EdgeTarget::File`) over the same table | `graph::invalidate::tests::test_parse_and_publication_changes_over_equal_paths_reuse_the_table` |
| Unchanged text and an unreferenced build-output creation: no generation, no graph work | `graph::invalidate::tests::test_unchanged_text_and_unreferenced_creations_do_no_graph_work` |
| A batch of three joins: one generation, one universe, one table | `graph::invalidate::tests::test_a_batch_of_joins_builds_one_universe_and_one_table` |
| A retained snapshot keeps wiki ambiguity, portability twins, basename buckets, document IDs, and headings through an edit and a membership change that removes one member of each pair | `graph::invalidate::tests::test_a_retained_snapshot_keeps_its_lookups_through_edits_and_membership_changes` |
| Roots-first numbering with parsed and unparsed documents interleaved; content in path order; `documents()` is double-ended; wiki resolution to a root | `graph::arena::tests::test_roots_are_numbered_first_and_content_follows_in_path_order` |
| Public builders never reuse; reuse on an equal-input edit and on unparsed → parsed; a new table for each `Listing` state, for an equal but distinct universe `Arc`, and for a dependency-flag change; a request graph's extended universe is stored in its table and never mutates the supplied one | `graph::arena::tests::test_a_table_is_reused_only_for_equal_structural_inputs` |
| Frontmatter keys are never cached (rule 6) | Phase 1's `test_a_renamed_frontmatter_key_re_resolves_its_variable_uses`, plus the key-rename step above |

Focused checkpoint runs:

- `cargo nextest run -p dmls --lib -E 'test(graph::) | test(wiki::) |
  test(work_counters) | test(providers::) | test(router::) |
  test(index_store::)'`: 359/359 passed before the new tests were added.
- `cargo nextest run -p dmls --lib -E 'test(graph::)'`: 87/87 passed with
  the new tests.
- `cargo nextest run -p dmls --test l1 -E 'test(level1_graph_index) |
  test(entry_point_parity) | test(index_store_keys) |
  test(referenced_targets) | test(wiki_universe_equivalence) |
  test(level1_wiki) | test(index_store_)'`: 151/151 passed.

### Gates

- `just lint` (darkmatter: lib, CLI, `dmls`, `zed-dmls`, and the wasm32
  check) passed with exit code 0 once the clippy findings were fixed
  (`needless_borrow` at callers, `manual_next_back`, `type_complexity` in a
  test).
- `just check-tier-coverage darkmatter`: 0 stranded tests.
- `just test` (darkmatter) stopped at its first failure,
  `fn_list_formatting::as_json5_uses_the_formatter_object_idiom`. I reran
  it as `just test --no-fail-fast` to list every failure (log at
  `/tmp/dm-phase2-test-all.log`): **10643 run, 10600 passed, 43 failed,
  10 skipped**. **None of the failures are in `dmls`**; all 1283 `dmls`
  results passed. The 43 failures are the JSON/YAML key-order family that
  Phase 1 recorded, caused by the concurrent frontmatter-key-order
  session's uncommitted `darkmatter/lib` edits: 40
  `schemas_convert_snapshots`,
  `suggest_constraint_phase3::conversion_snapshot_covers_valid_and_invalid_metadata`,
  `fn_list_formatting::as_json5_uses_the_formatter_object_idiom`, and
  `schema_validate_baseline::schema_validate_legacy_json_output_is_byte_identical`.
  Neither `darkmatter` nor `darkmatter-cli` depends on `dmls`, so this
  phase's edits cannot reach them. I did not touch those files.
- Dependent packages: `cargo tree -i dmls --workspace` lists nothing but
  `dmls` itself, so there was nothing downstream to test.
- Affected scope (`git status --porcelain --untracked-files=all | awk
  '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`): `packages` =
  claudine, darkmatter, dmls, test-toolkit, darkmatter-cli; `change_class` =
  package. The gates were required, so I ran them. The list is wide because
  other sessions' uncommitted files share this working tree.
- Cross-OS: production compares only `Path`/`PathBuf` values and adds no
  OS-specific code or dependency. The tests use `abs()` logical paths or
  `canonicalize_simplified` temp roots. I did not run a `just cross-check`;
  CI covers the other operating systems.
- Docs and skills: no `docs/` page or skill line describes record storage
  or node numbering (the skill's `DocumentRecord::parsed`/`dependency`
  references still hold). The topic docs are Phase 4's task.

## Phase 4

### Phase 3 work was missing, and is done here

When this run started, Phase 3 had not been implemented. The log had no
Phase 3 section, both Wave 4 tasks were unchecked, and
`tests/l1/incremental_graph_equivalence.rs` did not exist. This was the final
phase and has to mark the plan implemented, so this run did Phase 3's two
tasks first. That choice is recorded in `decisions_for_review`. The files are
listed under `source_files_during_phase_4`, because no Phase 3 run produced
them.

### Work done

- **Compare fresh graphs** (`dmls/tests/l1/incremental_graph_equivalence.rs`,
  new, declared in `tests/l1/main.rs`).
  - A transition harness drives a live `WorkspaceIndex` and keeps a separate
    model of its inputs: listed paths, parse products, open buffers, stale
    and unlisted reads, dependencies, listing state, and roots.
  - After every transition it compares the live snapshot with a fresh
    `WorkspaceGraph::build_listed` over the model's inputs. The fresh
    universe comes from `WikiUniverse::new` over the model's candidates,
    never from the live snapshot.
  - Nodes are identified by (path, per-document ordinal, kind, span), and
    every `WikiResolution` variant is matched exhaustively. The comparison
    covers node payloads, per-document node order, edges of every kind,
    incoming and outgoing queries, document facts, basename buckets,
    portability twins, transclusion facts, the universe, awaited paths, and
    unresolved diagnoses.
  - Twelve tests; the mapping is below. **No production defect was found:**
    every live snapshot matched its fresh build.
- **Exercise server updates** (`dmls/tests/l1/workspace_file_list.rs`). Three
  new server-session tests, an edit to the module doc that mentions the
  path-work counters, and small request helpers. No existing test changed.
- **Measure release editing.**
  - New driver: `measure-live-rebuild.py`.
  - New record: [measurement.md](./measurement.md).
  - A temporary seam (`#[track_caller]` cause attribution, thread CPU time
    via a temporary direct `libc` dependency, four stage timers) was added,
    built in release, used for one 100.0 s sample, and removed.
  - After removal, `git diff HEAD -- darkmatter/dmls/src
    darkmatter/dmls/Cargo.toml Cargo.lock` is empty. The lockfile's `libc`
    line was restored from a pre-measurement copy; the lockfile had no other
    changes.
- **Document snapshot reuse.**
  - `darkmatter/docs/lsp/architecture.md` has a new section, "Rebuilding the
    live snapshot". It covers:
    - what is path-derived and what comes from content;
    - a Mermaid flow from input comparison to publication;
    - the exact invalidation inputs, with an example table;
    - cached basename keys versus rebuilt frontmatter keys;
    - roots-first numbering and snapshot-local IDs;
    - dependency slots;
    - that the public builders always build fresh.
  - `darkmatter/docs/lsp/indexing.md` has a paragraph that links to that
    section.
  - The DMLS skill now has a bullet on reuse, the four new counters, and
    one fixed stale claim: the universe extension happens at table
    construction, not in `assemble`.
  - `dmls/README.md` needed no change; it says nothing about record storage
    or numbering.
  - The stale benchmark-stage claims in `graph/` were already gone after
    Phase 1.

### Measurement

The full record is in [measurement.md](./measurement.md). The sample was
100.0 s on an Apple M4 Max with macOS 27.2, revision `183a542da`, release
build.

| | Baseline | Step-1 prototype | This change |
| --- | --- | --- | --- |
| Rebuild CPU share of one core | 13.6% | 4.4% | **0.78%** |
| Median rebuild CPU | 42 ms | 12.35 ms | **2.61 ms** |
| Median rebuild wall time | 52 ms | 14.3 ms | **2.61 ms** |
| Server CPU over the sample | 20.6 s | 12.6 s | **8.65 s** |

- **Rebuilds:** 271 in total, 259 of them open-buffer edits. The universe
  and table were each built only 3 times: the unlisted `AGENTS.md` opening,
  the listed join, and the listed leave.
- **Build output:** the 50 creations under `target/` caused no rebuild.
- **Slowest rebuild:** 52.6 ms of wall time, a `relink` with under 5 ms of
  CPU.
- **Largest remaining stage:** comparing the cached universe's inputs,
  1.0 ms median and about 36% of rebuild CPU. The plan does not authorize
  more optimization, so it is recorded only.

Limitations:

- The original driver was not preserved. This driver is a reconstruction
  from the spec's description, not a replay.
- The seam did not print parsed or dependency counts.
- This is one run on one host. Both bounds are aids, not gates.

### Departures

- **Lookup terminology (rule 6).** The spec calls the shared lookup a
  "frontmatter-key" lookup. The shared table actually holds the wiki
  **basename** `KeyIndex`. Frontmatter keys (`FrontmatterKey` nodes,
  `key_by_name`, `UsesVariable` targets) are content and are rebuilt for
  every snapshot. The spec text is left as written. The docs describe the
  distinction.
- **Measurement record placement.** The plan asks for the new figures in a
  separate `measurement.md`, not in the spec's dated measurement section.
  That was done, so the spec stays a snapshot.
- **Phase 3 done in this run.** See above and `decisions_for_review`.

### Requirement-to-test mapping

All tests are L1 at fidelity F1 (in-process `dmls`) or F2 (an in-process
LSP server session over channels, `workspace_file_list.rs`). No loader or
format reader changed, so the Input Robustness Matrix does not apply
(rule 14).

| Requirement | Test |
|---|---|
| Content edits (heading, link target, frontmatter key) and listed buffer open/close match a fresh build; table and universe reused | `incremental_graph_equivalence::content_edits_and_listed_buffers_match_a_fresh_build` |
| Unsaved buffers outside the list; listed join, leave, and relist | `…::unsaved_buffers_and_listed_membership_match_a_fresh_build` |
| A dependency becomes open, returns to referenced-only, joins and leaves the list; the identical-text open quirk asserted directly | `…::dependencies_open_join_and_return_to_referenced_only_like_a_fresh_build` |
| Stale reads: unchanged, changed, failed | `…::stale_unchanged_changed_and_failed_reads_match_a_fresh_build` |
| `Pending` → `Ready{complete:false}` → `Ready{complete:true}` | `…::every_listing_state_matches_a_fresh_build` |
| Roots reordered, narrowed, emptied | `…::reordered_and_narrowed_wiki_roots_match_a_fresh_build` |
| Context relink with an injected failure and its recovery | `…::context_relink_matches_a_fresh_build` |
| Awaited creation vs. unreferenced build output | `…::awaited_creation_matches_a_fresh_build` |
| Missing candidate vs. probe error (symlink loop) | `…::missing_and_unprobeable_targets_match_a_fresh_build` |
| Wiki match unique → ambiguous → unique; portability twins | `…::wiki_matches_turn_ambiguous_and_back_like_a_fresh_build` |
| Batch of joins: one table, one universe | `…::a_batch_of_joins_matches_a_fresh_build` |
| Request graphs: rival outside scope, unique target outside graph, collision pair half in graph; documents-only-universe negative control | `…::request_graphs_over_the_live_universe_match_a_fresh_universe` |
| Provider answers over a reused live snapshot equal the oracle: references, rename, will-rename, wiki diagnostics, symbols, plus direct definition, hover, completion, and diagnostics with ranges | `…::corpus_provider_answers_over_a_reused_live_snapshot_equal_the_oracle` |
| Editing a listed open target: zero path work; definition, hover, and anchor completion use the new headings; the transcluding source gets current versioned diagnostics | `workspace_file_list::a_listed_target_edit_reuses_path_work_and_refreshes_its_dependents` |
| One watched batch of creations, then one of deletions: ambiguity appears and clears; definition follows; one universe and one table per batch | `workspace_file_list::one_watched_batch_of_creations_and_one_of_deletions_each_rebuild_once` |
| An unlisted awaited creation resolves; build output costs zero path work, parses, and reads | `workspace_file_list::an_awaited_unlisted_creation_resolves_and_build_output_costs_nothing` |

**Not covered in a session:** "no new publication" for build output. A
server session cannot observe the snapshot generation, and the router
re-sends diagnostics after every watched change. The in-crate unit test
`test_only_an_awaited_unlisted_creation_rebuilds` covers it.

### Focused runs

- `cargo nextest run -p dmls --test l1 -E 'test(incremental_graph_equivalence) | test(test_layout) | test(wiki_universe_equivalence)'`:
  20/20 passed. The slowest test took 0.15 s.
- `cargo nextest run -p dmls --test l1 -E 'test(workspace_file_list) | test(referenced_targets)'`:
  48/48 passed. The new tests passed three runs in a row, at 21–35 ms each.
- `cargo clippy -p dmls --tests -- -D warnings`: clean.

### Gates

These ran after the timing seam was removed and no subagent was running.

- **`just lint`** (darkmatter: lib, CLI, `dmls`, `zed-dmls`, wasm32 check):
  passed, exit 0.
- **`just check-tier-coverage darkmatter`:** 0 stranded tests.
- **`just test`** (darkmatter) stopped at its first failure,
  `fn_list_formatting::as_json5_uses_the_formatter_object_idiom`. A single
  rerun with `--no-fail-fast` (log `/tmp/dm-phase4-test-all.log`) gave
  **10659 run, 10616 passed, 43 failed, 10 skipped**.
  - **All 1,299 `dmls` tests passed:** Phase 2's 1,283 plus the 16 new ones.
  - The 43 failures are the same set Phase 2 recorded: 40
    `schemas_convert_snapshots::*`,
    `suggest_constraint_phase3::conversion_snapshot_covers_valid_and_invalid_metadata`,
    `fn_list_formatting::as_json5_uses_the_formatter_object_idiom`, and
    `schema_validate_baseline::schema_validate_legacy_json_output_is_byte_identical`.
  - They belong to the concurrent frontmatter-key-order fix: its plan's open
    Phase 5 "Review each candidate failure individually" task lists every
    one of them.
  - They are in `darkmatter` and `darkmatter-cli`, neither of which depends
    on `dmls`. This change does not touch them, and I left that work alone.
- **Dependents:** `cargo tree -i dmls --workspace` lists only `dmls`, so
  there is nothing downstream to test.
- **Affected scope** (`git status --porcelain --untracked-files=all | awk
  '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`):
  `packages` = darkmatter, dmls, test-toolkit, darkmatter-cli;
  `change_class` = package. The gates were required, and they ran. The
  list is wide because other sessions' uncommitted files share this
  working tree.
- **Final diff review.**
  - `git diff HEAD -- darkmatter/dmls/src darkmatter/dmls/Cargo.toml
    Cargo.lock` is empty, so no timing or cause logging, no `libc`
    dependency, and no production change remains.
  - The shared table holds no content data, and live rebuilds copy no
    universe; Phase 2's unit tests and the new equivalence tests assert
    both.
  - Both new test modules are compiled by declared targets, and the tier
    gate finds no stranded test.
  - The persistent facts format is unchanged.
- **Cross-OS.** No production code changed in this phase, and no
  OS-specific API or dependency was added. The tests use temp dirs,
  `canonicalize_simplified`, or logical `abs()` paths.
  - The probe-error case creates a symlink loop through `symlink_file` on
    Windows, the same as the existing in-crate test
    `test_an_unprobeable_earlier_candidate_is_not_answered_by_a_later_buffer`.
  - I did not run `just cross-check`. Evidence for Windows, Linux, and WSL2
    comes from CI, and none is claimed here.

### Ready for review

Every plan task is checked. The feature stays active, nothing was
committed, and `just complete` was not run.
