---
spec: "/Volumes/coding/wt/rusty-biscuit/feat-merman/darkmatter/features/2026-10-22-dmls-incremental-graph-updates/spec.md"
plan: "darkmatter/features/2026-10-22-dmls-incremental-graph-updates/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
message_to_agent: |-
  Phase 1 is in place: `WorkspaceIndex::wiki_universe` caches the universe
  by `WikiUniverseInputs` and prepopulates dependency slots as unmatchable.
  `WikiUniverse::portability_key(slot)` holds the per-slot keys.
  `WikiDoc::is_unmatchable` marks dependency slots. The four counters exist.
  `record_document_table` is currently called once per `assemble_listed`.
  When you add `DocumentTable`, move that call into actual table
  construction, so reuse reads 0. In `assemble_listed` and
  `portability_collisions`, a document whose *supplied* slot is unmatchable
  (a request graph built over the live universe that contains a live
  dependency as an ordinary document; see `router/facts.rs::request_graph`)
  keeps `WikiDoc::for_path` facts and a computed key. Keep that fallback
  when you move facts into the table. Phase 1 tests to update when Phase 2
  lands: `test_a_content_edit_reuses_the_universe_a_retained_snapshot_shares`
  asserts `wiki_path_facts == 1` (the dependency's real facts) on a content
  edit, which Phase 2 should make 0. Watch out: another session (the
  frontmatter-key-order fix) is editing `darkmatter/lib`, `biscuit-hash`,
  and `dmls/src/overlay/mod.rs` in this same worktree. Its uncommitted
  key-order work makes 44 darkmatter/darkmatter-cli L1 tests fail
  (`schemas_convert_snapshots`, JSON5 formatting, and others). Those
  failures are not caused by the `dmls` changes. Do not edit those files.
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

