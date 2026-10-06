---
kind: plan
feature: symbol-type-relations
spec: spec.md
total_phases: 9
created: 2026-10-05
phase: 9
agent: claude/sonnet
yolo: true
packages:
  - tree-hugger
  - tree-hugger-cli
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - tree-hugger/lib/src/shared/schema_v2/types.rs
  - tree-hugger/lib/src/shared/schema_v2/facets.rs
  - tree-hugger/lib/src/shared/schema_v2/mod.rs
  - tree-hugger/lib/src/file/mod.rs
  - tree-hugger/lib/src/file/tree_file.rs
  - tree-hugger/lib/src/file/extract/attributes.rs
  - tree-hugger/lib/src/file/extract/flags.rs
  - tree-hugger/lib/src/file/extract/metrics.rs
  - tree-hugger/lib/src/cache/symbol_cache.rs
  - tree-hugger/lib/src/cache/mod.rs
  - tree-hugger/lib/src/analysis/mod.rs
  - tree-hugger/lib/src/analysis/tests.rs
  - tree-hugger/lib/src/god_files/analysis.rs
  - tree-hugger/lib/tests/l1/cache_tests.rs
  - tree-hugger/lib/tests/l1/symbols_adapter.rs
  - tree-hugger/lib/tests/l1/symbols_cache.rs
  - tree-hugger/lib/tests/l1/symbols_metrics.rs
  - tree-hugger/lib/tests/l1/symbols_query.rs
  - tree-hugger/cli/src/declarations.rs
  - tree-hugger/cli/src/main.rs
  - tree-hugger/cli/src/report.rs
  - tree-hugger/cli/src/stats.rs
  - tree-hugger/cli/tests/symbols/cache_robustness.rs
  - tree-hugger/cli/tests/symbols/cache_upgrade.rs
  - tree-hugger/cli/tests/symbols/report.rs
  - tree-hugger/cli/tests/symbols/reports.rs
  - tree-hugger/cli/tests/symbols/wiring.rs
  - tree-hugger/cli/tests/fixtures/golden/symbols.json
  - tree-hugger/cli/tests/fixtures/golden/symbols.jsonl
docs_updated_during_phase_2:
  - tree-hugger/README.md
  - tree-hugger/lib/README.md
  - tree-hugger/cli/README.md
  - tree-hugger/docs/cache-design.md
  - tree-hugger/docs/symbol-kinds-and-visibility.md
  - tree-hugger/docs/symbol-reports.md
  - tree-hugger/docs/symbol-schema-design.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
  - .claude/skills/tree-hugger/SKILL.md
  - .claude/skills/tree-hugger/api-reference.md
source_files_during_phase_3:
  - tree-hugger/lib/src/file/extract/generics.rs
  - tree-hugger/lib/src/file/extract/mod.rs
  - tree-hugger/lib/src/file/tree_file.rs
  - tree-hugger/lib/src/file/mod.rs
  - tree-hugger/lib/src/analysis/mod.rs
  - tree-hugger/lib/src/error.rs
  - tree-hugger/lib/tests/l1/symbols_generics.rs
  - tree-hugger/lib/tests/l1/main.rs
  - tree-hugger/lib/tests/l1/symbols_coverage.rs
  - tree-hugger/cli/src/report.rs
  - tree-hugger/cli/tests/symbols/report.rs
  - tree-hugger/cli/tests/symbols/cache_upgrade.rs
  - tree-hugger/cli/tests/fixtures/golden/symbols.json
  - tree-hugger/cli/tests/fixtures/golden/symbols.jsonl
docs_updated_during_phase_3:
  - tree-hugger/docs/symbol-schema-design.md
  - tree-hugger/docs/symbol-reports.md
  - tree-hugger/docs/cache-design.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
  - .claude/skills/tree-hugger/SKILL.md
  - .claude/skills/tree-hugger/api-reference.md
source_files_during_phase_4:
  - tree-hugger/lib/src/file/extract/modifiers.rs
  - tree-hugger/lib/src/file/extract/attributes.rs
  - tree-hugger/lib/src/file/tree_file.rs
  - tree-hugger/lib/src/file/mod.rs
  - tree-hugger/lib/src/file/query.rs
  - tree-hugger/lib/tests/l1/symbols_modifiers.rs
  - tree-hugger/lib/tests/l1/main.rs
  - tree-hugger/lib/tests/l1/symbols_coverage.rs
  - tree-hugger/lib/tests/l1/symbols_flags.rs
  - tree-hugger/cli/tests/symbols/report.rs
  - tree-hugger/cli/tests/symbols/cache_upgrade.rs
  - tree-hugger/cli/tests/symbols/cache_robustness.rs
  - tree-hugger/cli/tests/fixtures/golden/symbols.json
  - tree-hugger/cli/tests/fixtures/golden/symbols.jsonl
docs_updated_during_phase_4:
  - tree-hugger/docs/symbol-schema-design.md
  - tree-hugger/docs/symbol-kinds-and-visibility.md
  - tree-hugger/docs/cache-design.md
  - tree-hugger/lib/README.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
  - .claude/skills/tree-hugger/SKILL.md
  - .claude/skills/tree-hugger/api-reference.md
source_files_during_phase_5:
  - tree-hugger/lib/src/file/extract/classify.rs
  - tree-hugger/lib/src/file/extract/metrics.rs
  - tree-hugger/lib/src/file/tree_file.rs
  - tree-hugger/lib/src/file/mod.rs
  - tree-hugger/lib/src/file/query.rs
  - tree-hugger/lib/tests/l1/symbols_metrics.rs
  - tree-hugger/lib/tests/l1/symbols_coverage.rs
  - tree-hugger/lib/tests/l1/symbols_query.rs
  - tree-hugger/cli/tests/symbols/report.rs
  - tree-hugger/cli/tests/symbols/cache_upgrade.rs
  - tree-hugger/cli/tests/symbols/cache_robustness.rs
  - tree-hugger/cli/tests/fixtures/golden/symbols.json
  - tree-hugger/cli/tests/fixtures/golden/symbols.jsonl
docs_updated_during_phase_5:
  - tree-hugger/docs/symbol-kinds-and-visibility.md
  - tree-hugger/docs/symbol-schema-design.md
  - tree-hugger/docs/cache-design.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
  - .claude/skills/tree-hugger/SKILL.md
  - .claude/skills/tree-hugger/api-reference.md
source_files_during_phase_6:
  - tree-hugger/lib/src/file/extract/relations.rs
  - tree-hugger/lib/src/file/extract/mod.rs
  - tree-hugger/lib/src/file/extract/rust_impl.rs
  - tree-hugger/lib/src/file/extract/classify.rs
  - tree-hugger/lib/src/file/tree_file.rs
  - tree-hugger/lib/src/file/tree_file/relations.rs
  - tree-hugger/lib/src/file/tree_file/definitions.rs
  - tree-hugger/lib/src/file/mod.rs
  - tree-hugger/lib/src/analysis/mod.rs
  - tree-hugger/lib/src/analysis/coverage.rs
  - tree-hugger/lib/src/cache/symbol_cache.rs
  - tree-hugger/lib/tests/l1/symbols_relations.rs
  - tree-hugger/lib/tests/l1/main.rs
  - tree-hugger/lib/tests/l1/symbols_coverage.rs
  - tree-hugger/lib/tests/l1/symbols_kinds.rs
  - tree-hugger/lib/tests/l1/symbols_owner_identity.rs
  - tree-hugger/lib/tests/l1/symbols_shared_context.rs
  - tree-hugger/lib/tests/l1/symbols_cache.rs
  - tree-hugger/cli/tests/symbols/cache_upgrade.rs
  - tree-hugger/cli/tests/fixtures/golden/symbols.json
  - tree-hugger/cli/tests/fixtures/golden/symbols.jsonl
docs_updated_during_phase_6:
  - tree-hugger/README.md
  - tree-hugger/lib/README.md
  - tree-hugger/docs/cache-design.md
  - tree-hugger/docs/symbol-kinds-and-visibility.md
  - tree-hugger/docs/symbol-schema-design.md
docs_created_during_phase_6:
  - tree-hugger/docs/type-relations.md
skills_files_updated_during_phase_6:
  - .claude/skills/tree-hugger/SKILL.md
  - .claude/skills/tree-hugger/api-reference.md
source_files_during_phase_7:
  - tree-hugger/lib/src/analysis/counters.rs
  - tree-hugger/lib/src/analysis/coverage.rs
  - tree-hugger/lib/src/file/query.rs
  - tree-hugger/lib/src/file/tree_file/relations.rs
  - tree-hugger/lib/tests/l1/symbols_adapter.rs
  - tree-hugger/lib/tests/l1/symbols_cache.rs
  - tree-hugger/lib/tests/l1/symbols_query.rs
  - tree-hugger/lib/tests/l1/symbols_relations.rs
  - tree-hugger/lib/tests/l1/symbols_shared_context.rs
docs_updated_during_phase_7:
  - tree-hugger/docs/symbol-schema-design.md
  - tree-hugger/docs/symbol-kinds-and-visibility.md
docs_created_during_phase_7: []
skills_files_updated_during_phase_7:
  - .claude/skills/tree-hugger/SKILL.md
  - .claude/skills/tree-hugger/api-reference.md
source_files_during_phase_8:
  - tree-hugger/cli/src/render.rs
  - tree-hugger/cli/tests/symbols/outline.rs
  - tree-hugger/cli/tests/symbols/main.rs
  - tree-hugger/cli/tests/level2_declarations.rs
  - tree-hugger/cli/tests/fixtures/golden/symbols.tree
docs_updated_during_phase_8:
  - tree-hugger/docs/symbol-reports.md
  - tree-hugger/docs/type-relations.md
  - tree-hugger/cli/README.md
docs_created_during_phase_8: []
skills_files_updated_during_phase_8:
  - .claude/skills/tree-hugger/SKILL.md
source_files_during_phase_9:
  - tree-hugger/lib/src/file/tree_file.rs
docs_updated_during_phase_9:
  - tree-hugger/docs/symbol-kinds-and-visibility.md
  - tree-hugger/docs/type-relations.md
  - tree-hugger/docs/symbol-schema-design.md
  - tree-hugger/README.md
  - tree-hugger/lib/README.md
  - tree-hugger/cli/README.md
docs_created_during_phase_9: []
skills_files_updated_during_phase_9:
  - .claude/skills/tree-hugger/SKILL.md
  - .claude/skills/tree-hugger/api-reference.md
source_code:
  - tree-hugger/lib/src/shared/schema_v2/types.rs
  - tree-hugger/lib/src/shared/schema_v2/facets.rs
  - tree-hugger/lib/src/shared/schema_v2/mod.rs
  - tree-hugger/lib/src/file/mod.rs
  - tree-hugger/lib/src/file/tree_file.rs
  - tree-hugger/lib/src/file/extract/attributes.rs
  - tree-hugger/lib/src/file/extract/flags.rs
  - tree-hugger/lib/src/file/extract/metrics.rs
  - tree-hugger/lib/src/cache/symbol_cache.rs
  - tree-hugger/lib/src/cache/mod.rs
  - tree-hugger/lib/src/analysis/mod.rs
  - tree-hugger/lib/src/analysis/tests.rs
  - tree-hugger/lib/src/god_files/analysis.rs
  - tree-hugger/lib/tests/l1/cache_tests.rs
  - tree-hugger/lib/tests/l1/symbols_adapter.rs
  - tree-hugger/lib/tests/l1/symbols_cache.rs
  - tree-hugger/lib/tests/l1/symbols_metrics.rs
  - tree-hugger/lib/tests/l1/symbols_query.rs
  - tree-hugger/cli/src/declarations.rs
  - tree-hugger/cli/src/main.rs
  - tree-hugger/cli/src/report.rs
  - tree-hugger/cli/src/stats.rs
  - tree-hugger/cli/tests/symbols/cache_robustness.rs
  - tree-hugger/cli/tests/symbols/cache_upgrade.rs
  - tree-hugger/cli/tests/symbols/report.rs
  - tree-hugger/cli/tests/symbols/reports.rs
  - tree-hugger/cli/tests/symbols/wiring.rs
  - tree-hugger/cli/tests/fixtures/golden/symbols.json
  - tree-hugger/cli/tests/fixtures/golden/symbols.jsonl
  - tree-hugger/lib/src/file/extract/generics.rs
  - tree-hugger/lib/src/file/extract/mod.rs
  - tree-hugger/lib/src/error.rs
  - tree-hugger/lib/tests/l1/symbols_generics.rs
  - tree-hugger/lib/tests/l1/main.rs
  - tree-hugger/lib/tests/l1/symbols_coverage.rs
  - tree-hugger/lib/src/file/extract/modifiers.rs
  - tree-hugger/lib/src/file/query.rs
  - tree-hugger/lib/tests/l1/symbols_modifiers.rs
  - tree-hugger/lib/tests/l1/symbols_flags.rs
  - tree-hugger/lib/src/file/extract/classify.rs
  - tree-hugger/lib/src/file/extract/relations.rs
  - tree-hugger/lib/src/file/extract/rust_impl.rs
  - tree-hugger/lib/src/file/tree_file/relations.rs
  - tree-hugger/lib/src/file/tree_file/definitions.rs
  - tree-hugger/lib/src/analysis/coverage.rs
  - tree-hugger/lib/tests/l1/symbols_relations.rs
  - tree-hugger/lib/tests/l1/symbols_kinds.rs
  - tree-hugger/lib/tests/l1/symbols_owner_identity.rs
  - tree-hugger/lib/tests/l1/symbols_shared_context.rs
  - tree-hugger/lib/src/analysis/counters.rs
  - tree-hugger/cli/src/render.rs
  - tree-hugger/cli/tests/symbols/outline.rs
  - tree-hugger/cli/tests/symbols/main.rs
  - tree-hugger/cli/tests/level2_declarations.rs
  - tree-hugger/cli/tests/fixtures/golden/symbols.tree
documentation:
  - tree-hugger/README.md
  - tree-hugger/lib/README.md
  - tree-hugger/cli/README.md
  - tree-hugger/docs/cache-design.md
  - tree-hugger/docs/symbol-kinds-and-visibility.md
  - tree-hugger/docs/symbol-reports.md
  - tree-hugger/docs/symbol-schema-design.md
  - tree-hugger/docs/type-relations.md
completed_phase: 9
implemented: true
---

# Plan: generics, inheritance, modifiers, attributes and body line counts

## Summary and definition of success

### The work

The `SymbolRecord` fields `type_info.generic_params`, `type_info.constraints`,
`relations.extends`, `relations.implements`, `modifiers` and `attributes` are
mostly empty today, and no metric reports a callable's size. This feature fills
them from syntax alone, in one parse and one definitions query per file, and
adds `metrics.lines`. Seven strands of work:

1. **Schema and cache plumbing** — the `lines` key (a record-shape change, so
   schema 4.1 and an `EXTRACTOR_REVISION` bump), strict metrics deserialization,
   and updating every test that treats 4.1 as incompatible.
2. **Structured generics** — a shared syntactic reader producing `GenericParam`
   (name, kind, bounds, default, variance, position) per declaration, with the
   older name-list readers kept as a projection.
3. **Modifiers and raw attributes** — Const/Readonly/Unsafe/Mut plus
   `Custom("abstract" | "final" | "override" | "virtual" | …)`; the existing
   `declaration_attributes` helper feeds `attributes` and is parsed once per
   analysis.
4. **Callable body lines** — extend the shared body-node reader to expression
   bodies and transparent wrappers; compute `lines` from the exact stored
   `body_span`.
5. **Type relations** — complete written base text, conservative same-file
   target IDs from one per-file light-declaration index, Rust impl ownership
   (member + resolved self type, empty/negative impls, containment
   aggregation), and the mixed-list languages (C#, Swift, Scala).
6. **Query/cache equivalence** — declarations-only, containment and metrics
   capabilities agree between a complete index, a selective query, a cache hit,
   narrowing and widening.
7. **Outline output and docs** — `extends X; implements Y; unclassified bases Z` on container rows of
   `hug types --tree` / `hug symbols --tree`, golden updates, docs and skill.

### What "done" looks like

Implementation complete, ready for review (the agent never closes the feature):

- [x] Every cell of the per-language table (16 languages) is either supported
      and tested positive/negative, marked an absent language construct, or
      carries a *demonstrated* grammar limit (snippet, pinned grammar identity
      and node observation) in the implementation log and current docs. An
      unfinished extractor is not a grammar limit.
    - **Author follow-up:** Option B is accepted and C#, Swift and Scala
      relations (6.5–6.7) are implemented. See `log.md` for the decision,
      verification and remaining Swift extension limitations.
- [x] `metrics.lines` exists, is required inside a present facet, serializes
      `None` as `null`, and the whole facet stays absent when unrequested.
- [x] Schema is 4.1 (or the next minor if 4.0 is no longer current) on records,
      file indexes, CLI envelopes and fingerprints; `EXTRACTOR_REVISION`
      bumped; symbol IDs unchanged.
- [x] For every capability set, a narrow query equals the matching slice of the
      complete index, including after a real `CacheLookup::Hit`, narrowing and
      widening. Declarations-only skips every optional pass; work counters show
      no repeated query or per-symbol whole-file scan.
- [x] `hug types --tree` and `hug symbols --tree` show inheritance on container
      rows; JSON/JSONL/full-index carry the same facts; goldens changed only
      for intended reasons.
- [x] `just test` and `just lint` green in `tree-hugger/`; `just test-l2` green
      for the rendering change, with no window gaining focus.
- [x] Docs, READMEs, skill, comments on changed readers and the implementation
      log are current; no doc names this feature.

## Phase 1: Gate, rulings and grammar probe

Nothing after this phase is written until the prerequisites are confirmed and
the rulings below are settled. Phase 1 is where the author can override any
default.

### Necessary Rules

Rulings the spec leaves open or implicit. Each carries the plan's default.

1. **Mixed inheritance/conformance lists (C#, Swift, Scala) — the spec's Open
   Question; blocks tasks 6.5–6.7 only.** The spec says its recommendation does
   not become design until the author accepts it. *Default (recommended):*
   written base lists go to `extends`, except entries proved to be conformance
   by same-file declaration syntax (target is a same-file interface/protocol/
   trait **and** the owner is a concrete implementing type), which go to
   `implements`; interface/protocol/trait inheritance stays `extends`; Swift
   extension entries are `implements` on a uniquely resolved local self type,
   and an unresolved extension owner is documented as unattachable. *Alternatives:*
   a new unclassified-base field (schema addition, extra label), or emit only
   provable entries. All other phases proceed without this ruling; if the
   author has not answered by Phase 6, build everything else and leave 6.5–6.7
   open rather than guess.
   Before 6.5–6.7 start, the accepted meanings with same-file and external
   examples (Swift struct/protocol/class/extension, Scala trait bases) are
   recorded in the implementation log.
2. **Schema version.** The spec rules 4.1 (or next minor of the current major).
   *Default:* read the current version in task 1.1; bump one minor; new version
   constant is introduced once, and tests that hard-code `V4_0` or treat 4.1 as
   incompatible are updated to use a genuinely different version (e.g. a 5.0 or
   a clearly invalid one).
3. **Prerequisite state.** `2026-10-03-symbol-filters-and-reports` is in
   review and `2026-10-05-deferred-kind-precision` is planned; the worktree is
   dirty. *Default:* proceed on the working state when `just test` is green
   (task 1.1), and make no assumption about which deferred kinds have landed:
   kind-sensitive logic (Go interface, Swift class, constructors) reads
   declaration syntax rather than `SymbolKind` where the kind is too broad.
4. **Relation target text vs. existing Rust trait text.** The spec says the
   full written base expression replaces the shortened last-segment text. This
   changes method-level `implements` for Rust. *Default:* accept; update
   affected tests/goldens as intended changes and record them.
5. **Modifier ordering is a serialization concern.** *Default:* normalize at
   one point (record construction) into the spec's fixed order
   (Static, StaticStorage, Async, Const, Readonly, Unsafe, Mut, then distinct
   Custom values in lexical order); extractors may push in any order.
6. **Impl-without-record container fallback.** For an unresolved Rust self type
   the member keeps its textual identity container and gets a textual
   `relations.container` named with the full self-type expression, with no
   parent ID and no local membership; qualified names and symbol IDs do not
   change. *Default:* as spec.
7. **`u16` overflow.** `GenericParam.position` is `u16`. A list of more than
   65 535 parameters yields an explicit analysis error, never truncation.
   *Default:* a unit test with a synthesized oversized list asserts the error.
8. **Wider performance measurement.** The spec requires no new performance
   spike; counter-based checks cover repeated-work risk. Any wider measurement
   is *not* planned; the author may request it. A proposed extra whole-file scan
   must be justified in the implementation log before it lands.
9. **Spike scope.** One spike, run once (task 1.2): which pinned-grammar nodes
   exist for generics, decorators/attributes and expression bodies. No
   performance spike (spec's figures answer it, see ruling 8).

### Spike

- [x] **1.2 Grammar probe** (runs once; informs Phases 3–5; not repeated)
    - For each of the 16 languages, parse one small snippet per applicable
      column (type parameters with bound/default/variance, base lists,
      decorators/attributes, expression-bodied callables, abstract/virtual/
      override/final tokens) and record the node kinds and field names the
      pinned grammar produces.
    - Output: a table in `implementation-log.md` (language × feature →
      node kind / "no node" / "grammar limit: snippet + grammar identity").
      Use the existing scope-dump or a throwaway test; delete any throwaway.
    - Success: every "None" cell of the spec table is confirmed as an absent
      construct, and every doubtful cell is classified supported or limited
      before Phase 3 starts.

### Tasks

- [x] **1.1 Prerequisite check** (sequential, first)
    - Run `just test` and `just lint` in `tree-hugger/`; record the baseline
      (failures here are pre-existing and must be listed, not fixed silently).
    - Record the current `SchemaVersion` constant, `EXTRACTOR_REVISION`
      (currently `"10"`; actually `"17"` at Phase 1, see the log), and which kinds from deferred-kind-precision exist.
    - Locate and list the exact files each later phase touches (see below) and
      confirm they exist: `lib/src/shared/schema_v2/{facets,mod}.rs`,
      `lib/src/file/extract/{attributes,classify,declaration,modifiers,
      rust_impl,type_metadata,metrics,flags}.rs`, `lib/src/file/tree_file.rs`,
      `lib/src/file/tree_file/definitions.rs`, `lib/src/file/query.rs`,
      `lib/src/analysis/coverage.rs`, `lib/src/cache/symbol_cache.rs`,
      `cli/src/{classes,declarations,render}.rs`.
- [x] **1.3 Author sign-off on rulings** (sequential, after 1.1 and 1.2)
    - Present the nine rulings (especially 1) and the probe table; record the
      answers in `implementation-log.md`. Proceed on defaults for any
      unanswered ruling except 1, which only gates 6.5–6.7.

**Validation checkpoint:** baseline recorded, probe table complete, rulings
logged.

## Phase 2: Schema, cache version and shared foundations

Single-owner phase: it touches shared types and files every later phase edits,
so it is not parallelized. Later phases then work on disjoint files.

### Wave 1 (sequential)

- [x] **2.1 `lines` key**
    - Add `lines: Option<u32>` to `MetricsFacet` with the same
      `deserialize_with = "nullable"` strictness as its siblings, so a missing
      or wrongly typed key is rejected and an old two-key metrics object is
      never defaulted.
    - Update every constructor of `MetricsFacet` (compile-driven) to set
      `lines: None` for now.
- [x] **2.2 Version bumps**
    - Introduce the next-minor `SchemaVersion` variant; update cache entries,
      per-record versions, file indexes, CLI envelopes, fingerprints and the
      exact-version check in `cache/symbol_cache.rs` (current check at the
      `!= V4_0` guard). Older versions are recomputed, never migrated.
    - Bump `EXTRACTOR_REVISION`. Confirm symbol IDs are unchanged (a test that
      pins an ID across the bump).
    - Update version-sensitive tests, including `tests/symbols/cache_upgrade.rs`
      in the CLI crate, replacing the "4.1 is incompatible" example with a
      genuinely different version.
- [x] **2.3 Shared parsed attributes**
    - Make `declaration_attributes` return a structure holding the raw
      attribute list (name, value) once per declaration, shared by the flag
      readers and the new `attributes` population, so one analysis parses each
      attribute once. Keep `is_test` / `is_deprecated` behavior and tests
      unchanged.
- [x] **2.4 Declaration modifier normalizer**
    - Add the single normalizer that dedups and orders modifiers per ruling 5.
      Existing Static/StaticStorage/Async sets must be unchanged.

### Input robustness matrix (cache entry reader, `metrics.lines`)

`lines` is the one new load-bearing field the cache/JSON reader consumes. Walk
the cells with one real-record fixture and one edit per cell; assert through the
public cache result (hit vs. recompute) or CLI JSON, not the deserializer's
return value.

| Shape | Edit | Required outcome |
|---|---|---|
| control | unedited fixture | warm `CacheLookup::Hit`, `lines` as stored |
| absent | `lines` key removed from a present metrics facet | entry rejected → recompute; never `None` |
| explicit null | `"lines": null` | accepted; `None` (unsupported/no body), distinct from absent |
| wrong type | `"lines": "3"` / `3.5` / `-1` | entry rejected → recompute |
| empty | `"lines": 0` | accepted; `Some(0)` (empty body span) |
| duplicate key | `lines` twice | rejected (or documented serde behavior, asserted) |
| facet absent | whole `metrics` removed | accepted as "not requested"; never read as computed `None` |
| old version | entry with prior schema version | rejected → recompute |
| trailing content | valid entry + garbage | rejected → recompute |

Grep before declaring done: `#[serde(default)]` on `lines`, `unwrap_or_default`
or `.ok()` on a metrics parse. A review finding about one row must list the
state of the others.

### Wave 2 (sequential)

- [x] **2.5 Foundation tests and goldens**
    - Run `just test` in `tree-hugger/`; fix compile-driven fallout only.
      Regenerate golden files **only** for the version/shape change and review
      the diff.

**Validation checkpoint:** `just test` and `just lint` green; matrix test
passing; IDs stable across the bump.

## Phase 3: Structured generic parameters

Phases 3, 4 and 5 touch disjoint extractor files once Phase 2 lands; they may
run in parallel by separate owners, with one merge/verify step each. Within this
phase the language groups are parallel.

### Wave 1 (single owner)

- [x] **3.1 Shared generic reader**
    - Add a syntactic reader (new module beside `type_metadata.rs`) returning
      `Vec<GenericParam>` plus declaration-level constraints, driven by a
      per-language node-kind table from the Phase 1 probe.
    - Rules to encode: source order and zero-based `position`; Type/Lifetime/
      Const kinds; Rust lifetimes keep the apostrophe; bound and default text
      read from grammar nodes, outer whitespace trimmed; bound lists split at
      grammar-defined entries (never on a comma inside `<…>`); default excludes
      the delimiter; variance recorded as written (`in`/`out`/`+`/`-`); const
      parameter type is not a constraint; separately written bounds on a named
      parameter attach to it in source order; `where T::Item: Copy` goes to
      `type_info.constraints`, never duplicated at both levels; no
      constraint inferred from use; `u16` overflow is an explicit analysis
      error (ruling 7).
    - Never copy enclosing type parameters to methods or an impl block's
      parameters to the implemented type.
    - Project the older name lists (`type_metadata.rs`) from the new reader only
      where behavior is preserved; otherwise leave the older readers alone.
- [x] **3.2 Record wiring**
    - Populate `type_info.generic_params` and `type_info.constraints` in record
      construction (`tree_file.rs`) as a declarations-capability fact.

### Wave 2 (parallel by language group; disjoint test and query files)

- [x] **3.3 Group A: Rust, C++** — lifetime/const/type, defaults, `where`;
  C++ template type and non-type parameters, defaults, representable
  constraints. Spec example: `struct Bag<T: Clone = String>`; `fn get<U>` owns
  only `U`.
- [x] **3.4 Group B: TypeScript, Java, C#, Scala** — bounds, defaults,
  variance (C# `in`/`out`, Scala `+`/`-`), C# `where` constraints, aliases,
  callables, constructors.
- [x] **3.5 Group C: Go, Python, Swift** — Go type/callable parameters with
  constraint text (a union or `~` term is retained as constraint text);
  Python explicit class/function/type-alias syntax if the pinned grammar has it;
  Swift parameters and explicit requirements.
- [x] **3.6 Group D: JavaScript, C, PHP, Perl, Bash, Zsh, Lua** — assert empty
  generics and add matrix rows classifying each as an absent construct.
- Each group adds fixtures to a new generics matrix under the existing symbols
  matrix framework: positive, negative (ownership, no copy to methods, no
  inference) and limitation rows per language.

### Wave 3 (sequential)

- [x] **3.7 Merge and verify** — run `just test`; extend
  `tests/l1/symbols_coverage.rs` so the generics matrix has a row for all 16
  languages (supported / absent / grammar-limit).

**Validation checkpoint:** generics matrix green; older name lists byte-identical
in their existing tests.

## Phase 4: Modifiers and raw attributes

### Wave 1 (single owner)

- [x] **4.1 Modifier reader**
    - Extend `modifiers.rs` (and the shared declaration/wrapper helpers it
      uses) to read declaration-owned tokens: Const (explicit const-callable
      syntax only: Rust `const fn`), Unsafe, Readonly (explicit declaration
      readonly), Mut (Rust binding/receiver `mut` on the declaration), and
      `Custom` lowercase `abstract`, `final`, `override`, `virtual`, plus
      `sealed` (C#) and `mutating` (Swift). Never map Java `final` or C++
      object `const` to Const.
    - Read through wrapper helpers (templates, decorated declarations); a
      wrapper's facts attach only to its declarations; a modifier inside a
      body or on a nested parameter must not mark the enclosing callable; do
      not scan raw text for keywords.
    - Feed through the normalizer from 2.4.
- [x] **4.2 Attribute population**
    - Populate `attributes` from the shared parse (2.3): name excludes the
      marker, whitespace compacted only in syntactic paths; value is raw
      argument text without outer delimiters, trimmed (no arguments → `None`,
      explicit `()` → `Some("")`); preserve order, repeats and list entries
      individually; Rust `cfg_attr` stays one raw attribute; inner/file/
      assembly attributes and doc comments never attach to every declaration;
      unusual decorator expressions keep their expression text as the name.
    - Add TS/JS decorators where the grammar supports them, without changing
      the existing test/deprecation flag recognition table.

### Wave 2 (parallel by language group; disjoint files)

- [x] **4.3 Group A: Rust, C++, Swift, Scala** — `unsafe`, `const fn`, `mut`
  receivers; `virtual`/`override`/`final`; Swift `final`/`override`/
  `mutating` (not Rust Mut); Scala `abstract`/`final`/`override`.
- [x] **4.4 Group B: TypeScript, JavaScript, Java, C#, PHP, Python** —
  TS `abstract`/`override`/`readonly`; JS decorators only, no invented TS
  modifiers; Java annotations + `abstract`/`final`; C# attribute lists,
  `abstract`/`override`/`virtual`/`readonly`, `sealed` as `Custom("sealed")`;
  PHP `abstract`/`final`, no PHP attributes; Python decorators with no inferred
  modifier.
- [x] **4.5 Group C: Go, C, Perl, Bash, Zsh, Lua** — assert none; classify each
  as an absent construct in the matrix.
- Each group adds rows to a new modifiers/attributes matrix, including: keyword
  only in a body or string, nested parameter, repeated attributes, empty
  arguments, whitespace inside literals, decorator expressions, and
  existing Static/Async sets unchanged.

### Wave 3 (sequential)

- [x] **4.6 Merge and verify** — `just test`; extend coverage guard for the
  modifiers/attributes matrix (16 languages); update the
  comments saying attributes are read "only for flags".

**Validation checkpoint:** flag recognition tests unchanged; matrix green.

## Phase 5: Callable body-line metric

Parallel with Phases 3 and 4 after Phase 2 (touches `classify.rs`,
`metrics.rs`, `symbols_metrics.rs`).

### Wave 1 (single owner)

- [x] **5.1 Body node reader**
    - Extend the shared body-node logic in `classify.rs` for grammar-supported
      expression bodies and transparent wrappers; the stored `body_span` and
      the metric must use the *same* node. Do not mistake a default value, type
      expression, returned callback body or nested function for the callable's
      own body. An expression body spans its expression, excluding `=>`/`=`.
    - Abstract/interface declarations and defaulted/deleted C++ constructors
      stay without a body.
- [x] **5.2 `lines` computation**
    - For Function, Method, Constructor only: no body → `None`; empty byte span
      → `Some(0)`; otherwise
      `end.line - start.line + (end.column > 1 ? 1 : 0)` with one-based
      locations. Other kinds are always `None`, even with a body span. CRLF and
      LF give equal counts. Populate it where the other metrics are computed so
      all three arrive together under the Metrics capability.
    - Confirm the God-files declaration-span calculation and its score
      regressions are untouched.

### Wave 2 (parallel by language group; test rows only)

- [x] **5.3 Matrix rows per language** — extend `symbols_metrics.rs`: LF, CRLF,
  `{}` on one line (1), span ending at column 1 (spec example 3→6 col 1 = 3),
  comments/blank/nested declarations counted, expression bodies (arrow,
  Rust none, Scala/Kotlin-style `=`, Python lambda excluded as a default),
  nested callable not mistaken for the body, abstract/interface `None`,
  defaulted/deleted C++ constructors `None`, non-callable kinds `None`.
  Groups A–D as in Phase 3 for owners.

### Wave 3 (sequential)

- [x] **5.4 Verify** — `just test`; confirm existing metrics cases for
  `parameters` and `has_docs` are unchanged.

**Validation checkpoint:** a strict test that `lines` is present, `null` where
unsupported, and absent with the facet when unrequested.

## Phase 6: Declared type relations

Depends on Phase 3 only in that both edit record construction; sequence Phase 6
after Phases 3–5 merge to avoid conflicts in `tree_file.rs`.

### Wave 1 (sequential; one owner for the shared index)

- [x] **6.1 Per-file light-declaration index**
    - Build once per file from all light declarations (independent of kind
      selection) keyed for bare-name and qualified lookup with scope. No
      per-relation or per-symbol rescans. Records unselected by the query still
      feed it.
- [x] **6.2 Target resolution rules**
    - `name` = complete written base expression, outer whitespace trimmed
      (`pkg.Base<T>` stays whole). `id` and in-file `qualified_name` only when
      syntax, qualification and lexical scope identify exactly one eligible
      same-file declaration; imported names, aliases needing expansion, type
      parameters, dynamic expressions and unproven external paths stay
      unresolved even if the last segment equals a local name; never take the
      first match or fall back from a failed qualified match to a bare name;
      no scan-root or module prefix in stored targets. Preserve source order,
      dedup equal target expressions per list, keep distinct instantiations.
      No inherited or transitive edges.
- [x] **6.3 Plain-language relations** — wire `extends`/`implements` from the
  per-language table: TS (class extends/implements, interface extends), JS
  (written extends expression, unresolved if dynamic), Java, PHP (trait `use`
  is composition), Python (positional bases; `metaclass=` excluded), Go
  (interface embedded *named* types only; not unions or `~` terms; embedded
  struct fields are composition), C++ (all base specifiers, access/virtual
  keywords not part of the target text), Rust trait supertraits (lifetime
  bounds are constraints, not bases). C, Perl, Bash, Zsh, Lua: none, with
  negative rows (`@ISA`, `use parent`, metatables not inferred).

### Wave 2 (sequential; Rust impl ownership)

- [x] **6.4 Rust impl traversal**
    - In `rust_impl.rs`, traverse impl declarations once, including empty impls
      (`impl Marker for Local {}`); stop shortening the trait to its last path
      segment; keep `implements` on each directly declared trait-impl member
      and aggregate it on the uniquely resolved self type; inherent impls add
      members but no relation; negative impls add no `implements`. Self types
      match by full written path under the base-target rules; bare or
      same-file-qualified generic self types match their base without alias
      expansion; reference, tuple and composite self types do not attach.
    - Containment: aggregate same-file impl members onto the resolved type's
      `relations.members` without overwriting ordinary members or members from
      other impls; unresolved self types follow ruling 6. Do not change
      qualified-name construction or symbol IDs.
    - The containment pass must reuse this resolution, not re-guess from the
      short container name.
    - Replace the "exactly one same-named trait in the file" link logic in
      `tree_file.rs` with the shared resolver so an explicitly external trait
      is no longer linked to a local declaration.

### Wave 3 (author accepted Option B; implemented)

- [x] **6.5 C# base lists** — per accepted ruling; record examples.
- [x] **6.6 Swift lists and extensions** — struct/protocol/class/extension
  cases; extension entries as `implements` on a uniquely resolved local self
  type with no new type record; document the unresolved-owner limitation.
- [x] **6.7 Scala `extends`/`with`** — trait bases per ruling.

### Wave 4 (sequential)

- [x] **6.8 Relations matrix**
    - New matrix under the symbols framework with a row for all 16 languages.
      Required cases: qualified external target sharing a local short name,
      duplicate names in different scopes, ambiguous and alias targets, type
      parameter base, multiple and empty Rust impls, negative impls, full
      trait/self-type text, method-only queries, repeated instantiations kept,
      dedup of equal expressions, and the accepted mixed-list cases.
    - Extend `symbols_coverage.rs`.

**Validation checkpoint:** relations matrix green, including the accepted
Option B cases for 6.5–6.7. Current validation is recorded in `log.md`.

## Phase 7: Capabilities, cache and query/index equivalence

### Wave 1 (single owner)

- [x] **7.1 Planner and projection**
    - Extend `analysis/coverage.rs` and `file/query.rs` so Declarations carries
      generics, modifiers, attributes and extends/implements with conservative
      IDs (selected records get full metadata; unselected records stay light
      but still feed the index); Containment carries impl aggregation and
      container links using the same evidence; Metrics carries all three
      measurements, widening reparses once, narrowing drops the whole facet.
      No new capability. One definitions query and at most one parse per
      analysis; no references, semantic or diagnostics passes for these facts.
    - Persist enough resolved-self-type evidence for correct widening from
      declarations to containment; if the record shape cannot retain it,
      document the limitation and reparse once on that path rather than
      guessing.

### Wave 2 (parallel; disjoint test files)

- [x] **7.2 Query tests** — `symbols_query.rs`: narrow query equals the matching
  slice of the complete index under each capability set; declarations-only
  skips optional passes (assert via the work counters).
- [x] **7.3 Cache tests** — `symbols_cache.rs`: a real warm `CacheLookup::Hit`
  (not silent recomputation), projection, widening (declarations →
  containment, → metrics), narrowing, invalidation by the new
  `EXTRACTOR_REVISION`/schema version, and the Phase 2 matrix.
- [x] **7.4 Adapter tests** — `symbols_adapter.rs`: the record-only older-model
  adapters remain explicitly lossy for structured generics, relations, raw
  attributes and new modifiers; assert and document the losses.
- [x] **7.5 Counter checks** — assert no repeated definitions query and no
  per-symbol whole-file walk (one impl traversal, one index build per file).

### Wave 3 (sequential)

- [x] **7.6 Verify** — `just test` and `just lint`.

**Validation checkpoint:** equivalence proven for every capability combination.

## Phase 8: Outline output

### Wave 1 (single owner; CLI renderer)

- [x] **8.1 Container-row inheritance**
    - In the CLI renderer (`cli/src/render.rs`, `classes.rs` as applicable)
      append `extends A; implements B` using recorded written targets in
      source order to a visible type container row, including containers with
      no displayed children, for `hug types --tree` and `hug symbols --tree`.
    - A method's retained Rust trait relation is not shown as the method's
      inheritance heading; `hug functions --tree` gains no synthetic type row;
      no filtered-out container or target is added; with full/short signatures
      do not repeat an inheritance clause already in the header; relations are
      shown with `--signatures none`. Use biscuit-terminal renderable
      components (`Prose`, wrapping), preserve sorting and plain-output rules.
    - Counts and statistics gain no new category.

### Wave 2 (parallel; disjoint test files)

- [x] **8.2 CLI assertions** — outline tests under `cli/tests/symbols/*.rs`
  for the cases above, plus JSON/JSONL/full-index carrying the same facts and
  `metrics.lines`.
- [x] **8.3 Golden files** — regenerate `symbols.json`/`symbols.jsonl` and any
  outline goldens; review every diff; only intended changes (schema version,
  new fields) are accepted.
- [x] **8.4 L2 rendering** — run `just test-l2` through the Test Toolkit;
  confirm no terminal/browser window gains focus.

**Validation checkpoint:** `just test` and `just test-l2` green in
`tree-hugger/`.

## Phase 9: Docs, skill, review of comments and close

### Wave 1 (parallel documentation tasks; disjoint files)

- [x] **9.1 Current docs** — update `docs/` schema, cache (including
  `cache-design.md` version and revision), symbol-kinds, report and API pages;
  document unsupported forms (with snippet, grammar identity, node limit), the
  mixed-list meaning per the accepted ruling, and the adapter losses. Use
  Mermaid where a flow is clearer, written for a reader with no repo
  experience. Docs never name or link this feature.
- [x] **9.2 READMEs** — area, `lib/` and `cli/` READMEs for the new facts and
  outline output.
- [x] **9.3 Skill** — update `.claude/skills/tree-hugger/SKILL.md` for the new
  fields, the 4.1 schema and the capability behavior.
- [x] **9.4 Comment drift pass** — review comments on every changed reader,
  especially "attributes are read only for flags" and "metrics have two
  members"; fix per the code-is-correct rule and report each drift found.

### Wave 2 (sequential)

- [x] **9.5 Implementation log** — record accepted rulings, the grammar probe
  table, every departure from the spec, and the unsupported-form list.
- [x] **9.6 Final verification** — `just test`, `just lint`, `just test-l2` in
  `tree-hugger/`; confirm the Definition-of-success checklist above. Leave the
  feature directory active (the author closes it); the agent's terminal state
  is "implementation complete, ready for review". No commit unless asked.

**Validation checkpoint:** every box in the success list is checked or has a
logged, accepted exception.
