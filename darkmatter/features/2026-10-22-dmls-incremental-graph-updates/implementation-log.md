---
spec: "/Volumes/coding/wt/rusty-biscuit/feat-merman/darkmatter/features/2026-10-22-dmls-incremental-graph-updates/spec.md"
plan: "darkmatter/features/2026-10-22-dmls-incremental-graph-updates/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
message_to_agent: |-
  Phase 2 is in place. `graph/arena.rs` now has a private `DocumentTable`
  (path records, `by_path`, `slot_documents`, basename `KeyIndex`,
  portability collisions, the universe `Arc`, and `Listing`) shared by
  `Arc` between snapshots, plus a per-snapshot `Vec<DocumentState>`
  (hash, parsed). `DocumentRecord<'a>` is a borrowed `Copy` view
  (`path: &PathBuf`, `canonical: &Vec<String>`); `documents()` returns an
  `ExactSizeIterator + DoubleEndedIterator`. Roots are allocated first, so
  `DocumentId(d)`'s root is `NodeId(d)`. Only `WorkspaceIndex::rebuild`
  calls `build_listed_reusing`; the public builders always build a fresh
  table, so they are the uncached oracle for Phase 3. Reuse needs equal
  sorted paths, equal dependency flags, equal `Listing`, and the *same*
  universe `Arc` (`DocumentTable::matches`). For tests inside the crate,
  `WorkspaceGraph::shares_document_table` (`#[cfg(test)]`) checks table
  identity. Found while testing: opening a dependency's buffer with
  byte-identical text returns `Unchanged` and does not rebuild, so the
  snapshot keeps that document marked as a dependency until the next
  rebuild. That behavior predates this change, and rule 12 says to keep
  publication behavior, so it is unchanged and listed in
  `decisions_for_review`. Phase 3's transition harness should change the
  text when it opens a dependency, or assert that quirk explicitly. Other
  sessions are still editing `darkmatter/lib`, `claudine`, and
  `biscuit-hash` in this worktree; their uncommitted work causes the
  non-`dmls` failures recorded under Phase 2 Gates.
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
