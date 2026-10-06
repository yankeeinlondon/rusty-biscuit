---
kind: plan
name: cyclomatic-complexity
feature: 2026-10-07-cyclomatic-complexity
created: 2026-10-05
phase: 4
total_phases: 5
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
  - tree-hugger/lib/src/file/extract/complexity/mod.rs
  - tree-hugger/lib/src/file/extract/complexity/rust.rs
  - tree-hugger/lib/src/file/extract/complexity/tests.rs
  - tree-hugger/lib/src/file/extract/classify.rs
  - tree-hugger/lib/src/file/extract/metrics.rs
  - tree-hugger/lib/src/file/extract/mod.rs
  - tree-hugger/lib/src/file/tree_file.rs
  - tree-hugger/lib/src/file/mod.rs
  - tree-hugger/lib/src/file/query.rs
  - tree-hugger/lib/src/shared/schema_v2/facets.rs
  - tree-hugger/lib/src/shared/schema_v2/types.rs
  - tree-hugger/lib/src/shared/schema_v2/mod.rs
  - tree-hugger/lib/src/analysis/mod.rs
  - tree-hugger/lib/src/analysis/tests.rs
  - tree-hugger/lib/src/cache/mod.rs
  - tree-hugger/lib/src/cache/symbol_cache.rs
  - tree-hugger/lib/src/god_files/analysis.rs
  - tree-hugger/cli/src/declarations.rs
  - tree-hugger/cli/src/main.rs
  - tree-hugger/cli/src/report.rs
  - tree-hugger/cli/src/stats.rs
  - tree-hugger/lib/tests/l1/symbols_metrics.rs
  - tree-hugger/lib/tests/l1/symbols_query.rs
  - tree-hugger/lib/tests/l1/symbols_adapter.rs
  - tree-hugger/lib/tests/l1/symbols_cache.rs
  - tree-hugger/lib/tests/l1/cache_tests.rs
  - tree-hugger/cli/tests/symbols/cache_upgrade.rs
  - tree-hugger/cli/tests/symbols/cache_robustness.rs
  - tree-hugger/cli/tests/symbols/wiring.rs
  - tree-hugger/cli/tests/symbols/reports.rs
  - tree-hugger/cli/tests/symbols/report.rs
  - tree-hugger/cli/tests/fixtures/golden/symbols.json
  - tree-hugger/cli/tests/fixtures/golden/symbols.jsonl
  - tree-hugger/cli/tests/fixtures/golden/symbols.tree
docs_updated_during_phase_2:
  - tree-hugger/docs/symbol-schema-design.md
  - tree-hugger/docs/cache-design.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
  - .claude/skills/tree-hugger/SKILL.md
  - .claude/skills/tree-hugger/api-reference.md
source_files_during_phase_3:
  - tree-hugger/lib/src/file/extract/complexity/mod.rs
  - tree-hugger/lib/src/file/extract/complexity/javascript.rs
  - tree-hugger/lib/src/file/extract/complexity/typescript.rs
  - tree-hugger/lib/src/file/mod.rs
  - tree-hugger/lib/tests/l1/symbols_metrics.rs
  - tree-hugger/cli/tests/symbols/cache_upgrade.rs
  - tree-hugger/cli/tests/fixtures/golden/symbols.json
  - tree-hugger/cli/tests/fixtures/golden/symbols.jsonl
docs_updated_during_phase_3:
  - tree-hugger/docs/symbol-schema-design.md
  - tree-hugger/docs/cache-design.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
  - .claude/skills/tree-hugger/SKILL.md
source_files_during_phase_4:
  - tree-hugger/lib/src/analysis/counters.rs
  - tree-hugger/lib/src/file/extract/complexity/mod.rs
  - tree-hugger/lib/tests/l1/symbols_query.rs
  - tree-hugger/lib/tests/l1/symbols_cache.rs
  - tree-hugger/lib/tests/l1/symbols_adapter.rs
  - tree-hugger/cli/tests/symbols/cache_robustness.rs
  - tree-hugger/cli/tests/symbols/reports.rs
docs_updated_during_phase_4:
  - tree-hugger/docs/cache-design.md
  - tree-hugger/docs/symbol-schema-design.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
  - .claude/skills/tree-hugger/SKILL.md
  - .claude/skills/tree-hugger/api-reference.md
source_files_during_phase_5:
  - tree-hugger/cli/tests/symbols/reports.rs
docs_updated_during_phase_5:
  - tree-hugger/docs/symbol-schema-design.md
  - tree-hugger/docs/symbol-kinds-and-visibility.md
  - tree-hugger/docs/symbol-reports.md
  - tree-hugger/docs/cache-design.md
  - tree-hugger/README.md
  - tree-hugger/lib/README.md
  - tree-hugger/cli/README.md
docs_created_during_phase_5:
  - tree-hugger/docs/cyclomatic-complexity.md
skills_files_updated_during_phase_5:
  - .claude/skills/tree-hugger/SKILL.md
source_code:
  - tree-hugger/lib/src/file/extract/complexity/mod.rs
  - tree-hugger/lib/src/file/extract/complexity/rust.rs
  - tree-hugger/lib/src/file/extract/complexity/javascript.rs
  - tree-hugger/lib/src/file/extract/complexity/typescript.rs
  - tree-hugger/lib/src/file/extract/complexity/tests.rs
  - tree-hugger/lib/src/file/extract/classify.rs
  - tree-hugger/lib/src/file/extract/metrics.rs
  - tree-hugger/lib/src/file/extract/mod.rs
  - tree-hugger/lib/src/file/tree_file.rs
  - tree-hugger/lib/src/file/mod.rs
  - tree-hugger/lib/src/file/query.rs
  - tree-hugger/lib/src/shared/schema_v2/facets.rs
  - tree-hugger/lib/src/shared/schema_v2/types.rs
  - tree-hugger/lib/src/shared/schema_v2/mod.rs
  - tree-hugger/lib/src/analysis/mod.rs
  - tree-hugger/lib/src/analysis/tests.rs
  - tree-hugger/lib/src/analysis/counters.rs
  - tree-hugger/lib/src/cache/mod.rs
  - tree-hugger/lib/src/cache/symbol_cache.rs
  - tree-hugger/lib/src/god_files/analysis.rs
  - tree-hugger/cli/src/declarations.rs
  - tree-hugger/cli/src/main.rs
  - tree-hugger/cli/src/report.rs
  - tree-hugger/cli/src/stats.rs
  - tree-hugger/lib/tests/l1/symbols_metrics.rs
  - tree-hugger/lib/tests/l1/symbols_query.rs
  - tree-hugger/lib/tests/l1/symbols_adapter.rs
  - tree-hugger/lib/tests/l1/symbols_cache.rs
  - tree-hugger/lib/tests/l1/cache_tests.rs
  - tree-hugger/cli/tests/symbols/cache_upgrade.rs
  - tree-hugger/cli/tests/symbols/cache_robustness.rs
  - tree-hugger/cli/tests/symbols/wiring.rs
  - tree-hugger/cli/tests/symbols/reports.rs
  - tree-hugger/cli/tests/symbols/report.rs
  - tree-hugger/cli/tests/fixtures/golden/symbols.json
  - tree-hugger/cli/tests/fixtures/golden/symbols.jsonl
  - tree-hugger/cli/tests/fixtures/golden/symbols.tree
documentation:
  - tree-hugger/docs/cyclomatic-complexity.md
  - tree-hugger/docs/symbol-schema-design.md
  - tree-hugger/docs/cache-design.md
  - tree-hugger/docs/symbol-kinds-and-visibility.md
  - tree-hugger/docs/symbol-reports.md
  - tree-hugger/README.md
  - tree-hugger/lib/README.md
  - tree-hugger/cli/README.md
  - .claude/skills/tree-hugger/SKILL.md
  - .claude/skills/tree-hugger/api-reference.md
completed_phase: 5
implemented: true
---

# Plan: Per-Symbol Cyclomatic Complexity

## Summary and definition of done

Add `cyclomatic: Option<u32>` to the tree-hugger library's `MetricsFacet`
(`lib/src/shared/schema_v2/facets.rs`). It is computed in `symbol_metrics`
(`lib/src/file/extract/metrics.rs`) by an iterative, pruned traversal of the
callable body node already chosen by `body_node_for_declaration`
(`lib/src/file/extract/classify.rs`) for `source.body_span` and `lines`.

The work has four moving parts:

1. **Counting engine**: a language-agnostic traversal that stops at nested
   callable/type boundaries, returns `None` on error or missing nodes inside the
   counted body, uses checked `u32` arithmetic, and is driven by a per-language
   rule table keyed on grammar node kinds, fields and operator tokens.
2. **Per-language rule tables** for the accepted initial language set, each with
   a reviewed inventory and executable expectations. Unreviewed languages stay
   `None`.
3. **Schema, cache and extractor invalidation**: a required nullable key, a
   schema minor bump, an `EXTRACTOR_REVISION` bump, and strict deserialization.
4. **Tests and docs**: metrics matrix column for all 16 languages, selective
   query/cache/work-counter tests, CLI JSON/JSONL/full-index checks, golden
   updates, and current-behavior docs plus the tree-hugger skill.

Done means (from the spec): the author has accepted the convention and language
set; every selected rule has an executable expectation; all 16 matrix rows are
explicit; narrow, complete, cached and uncached queries agree; docs explain how
to read a number or `null`; `just test` and `just lint` pass in `tree-hugger/`.
The feature directory stays active (an agent never moves it to `_completed`).

Working-tree facts checked while planning (2026-10-05):

- `MetricsFacet` has only `parameters` and `has_docs`; its `nullable`
  deserializer already rejects an absent key.
- The emitted schema is `SchemaVersion::V4_0` and `EXTRACTOR_REVISION` is `"10"`.
  The prerequisite `2026-10-06-symbol-type-relations` (adds `lines`, normally
  V4_1) has **not** landed. This plan therefore cannot start Phase 3 until it does.
- `body_node_for_declaration` exists in `classify.rs`; `lines` does not.

## Phase 1: Rulings, prerequisites and rule inventories

Goal: nothing in later phases depends on an unresolved choice.

### Necessary Rules

Each needs an author ruling recorded in the implementation log before the
phase that depends on it. Defaults below follow the spec's recommendations and
are what the plan assumes if the author accepts without change.

1. **Counting convention** (blocks Phase 2). Assumed: the recommended
   convention. Each `if`/else-if, conditional loop, ternary and short-circuit
   Boolean operator is +1; plain `else`, `finally`, unconditional loops and
   unconditional return/break/throw are 0; each non-default arm is +1 (grouped
   labels/patterns are one arm); each separate guard is +1; each exception
   handler is +1.
2. **Initial language set** (blocks Phase 3). Assumed: Rust, JavaScript,
   TypeScript, with others added only if their complete inventory passes
   review. At least one language must report reviewed non-null values.
3. **Residual forms the recommendation leaves open**, decided per language in
   the inventory: comprehensions and generator filters, null-coalescing `??`,
   optional chaining `?.`, logical assignment (`&&=`, `||=`, `??=`), Rust `?`,
   pattern alternatives. Plan default: `??` and logical assignment count as
   short-circuit operators (+1); `?.` and Rust `?` contribute 0 by design,
   documented as such. The author may flip any of these; the rule table is the
   single place that changes.
4. **Recognized-but-unsupported forms** make the metric `None` rather than a
   partial count. The inventory must name each such form per language.
5. **Prerequisite gating.** Phase 3 starts only after `2026-10-06-symbol-type-relations`
   has landed `lines` and its schema bump. The schema version for this feature is
   the next minor above whatever is emitted at that point (normally V4_2 after V4_1),
   read from the tree, not hard-coded from this plan.
6. **Wider measurement.** The spec rules out a performance spike and CI changes.
   If the author wants any large-file timing measurement, it is their call and
   is not planned here.
7. **Extractor revision** is bumped once for this feature's first rules and again
   (per the spec) by any later change to supported rules, not per language in
   this plan's own commits.

### Spike

One spike, run once, before Phase 2 rule work. No re-check later.

- [x] **Grammar shape dump**: for each candidate language (Rust, JavaScript,
  TypeScript first), print the pinned-grammar parse of one small fixture per
  inventory category (if/else-if, loops, switch/match with guards, ternary,
  `&&`/`||`/`??`, try/catch/finally, nested fn/closure/class, expression-body
  arrow, comprehension where present). Record the node kinds, field names and
  operator token spellings in the language's rule table draft.
  - Question answered: do the pinned grammars expose operators as tokens or
    fields, do guards and else-if have distinct nodes, and what are the
    nested-boundary kinds? This avoids guessing at rules in Phase 3.
  - Output is the draft rule table in the implementation log; one host
    (macOS) is enough since grammar shape is OS-independent.

### Tasks

- [x] **Confirm prerequisite**: check that `lines`, the shared body reader and
  the next schema minor have landed; if not, do Phases 1 and 2 and hold Phase 3.
- [x] **Record rulings**: write the accepted answers to rules 1–4 into
  `implementation-log.md` before engine code is merged.
- [x] **Run grammar spike** (above) and save the draft per-language rule tables.
- [x] **Finalize rule tables**: for each selected language, a compact table of
  pinned node/field/operator matches, contribution, exclusions, and source
  examples with expected totals; every category listed in the spec's inventory
  section is classified as contributes / zero by design / unavailable.
  Checkpoint: the author accepts the tables (rule 2 gate). Tables drafted in
  the implementation log; author acceptance is pending (spec `human_review`).

### Input robustness matrix (deserialization of `metrics.cyclomatic`)

The new key is a load-bearing field read by the record/index/envelope
deserializer for JSON (JSONL and the full index use the same record type). It
is a required nullable `u32`.

| Shape | Edit | Required outcome |
|---|---|---|
| control | unedited fixture | parses, value preserved |
| absent | key omitted | deserialization error |
| explicit null | `"cyclomatic": null` | parses as `None` (the contract) |
| wrong type, whole field | `"cyclomatic": "3"`, `[3]`, `true` | error, never `None` |
| wrong type, negative / fractional | `-1`, `1.5` | error |
| out of range | `4294967296` | error, not wrapped or capped |
| empty | `{}` or `[]` | error |
| duplicate key | `"cyclomatic"` twice | error (serde default for structs) |
| trailing/invalid content | valid document plus garbage | error |
| older schema/entries | previous schema version or old revision | recomputed, not defaulted to `null` |

The test is one walk over this table from a real fixture, asserting through
the public parse/cache result. Grep before declaring done: no
`#[serde(default)]` or bare `Option<u32>` without `deserialize_with = "nullable"`
on `cyclomatic`.

## Phase 2: Counting engine, schema and plumbing

Goal: the key exists end to end and the engine counts for one language.
Depends on Phase 1 ruling 1 and the prerequisite (see Phase 1 rule 5).

### Wave 1 (parallel)

- [x] **Facet field**: add `cyclomatic` with `#[serde(deserialize_with = "nullable")]`
  to `MetricsFacet` with a doc comment stating that `None` means unavailable
  for this declaration; update the doc that says how many metrics exist.
  - Update every `MetricsFacet { .. }` construction site (compiler finds them).
- [x] **Traversal engine**: new module next to `metrics.rs` implementing an
  iterative cursor traversal over the selected body node.
  - Takes a per-language rule table (node kind to contribution, nested-boundary
    kinds, operator-token matches).
  - Prunes nested callable and type boundaries without counting the boundary.
  - Returns `None` on an error or missing node in a *visited* region only;
    errors in pruned subtrees or elsewhere in the file are ignored.
  - Checked arithmetic; result over `u32` returns `None`.
  - No per-callable whole-file scan; no recursion per syntax node (include a
    deep-nesting test that would overflow a recursive version).
  - Excludes parameter defaults, decorators, attributes, return/type syntax
    because only the body node is traversed.
- [x] **Schema bump**: advance `SchemaVersion` to the next minor above the
  prerequisite's, and update records, indexes, CLI envelopes and fingerprints
  consistently; update `docs/symbol-schema-design.md` version history.
- [x] **Extractor revision**: bump `EXTRACTOR_REVISION` in `lib/src/file/mod.rs`.

### Wave 2 (after Wave 1)

- [x] **Wire up**: call the engine from `symbol_metrics` only for
  Function/Method/Constructor with a body from the shared body reader;
  every other kind, bodyless declaration or unsupported language yields `None`
  while `parameters`, `has_docs` and `lines` stay intact.
  - `symbol_metrics` is only reached when Metrics is requested, so a
    declarations-only query never traverses; confirm and keep it that way.
  - Do not add a second body discovery path.
- [x] **Cache projection check**: confirm `analysis/coverage.rs` still strips the
  whole facet when Metrics is unrequested; no new capability.
- [x] **Rust rule table** (first language, exercises nested items, match arms,
  guards): implement per the reviewed Phase 1 table.

Validation checkpoint: `just test` and `just lint` in `tree-hugger/` pass;
Rust fixtures give the exact expected totals; fixtures assert no parse errors.

## Phase 3: Language rule tables

Goal: every accepted language has a reviewed table and executable expectations.
Gate: author-accepted language set (Phase 1 rule 2).

### Wave 1 (parallel, one subagent per language; files are disjoint)

- [x] **JavaScript rules** and fixtures.
- [x] **TypeScript rules** and fixtures (shares most JS shapes; includes type
  declarations as nested-type boundaries).
- [x] **Further reviewed languages**, one task each, only if their complete
  inventory was accepted in Phase 1. (None: the drafted set is Rust, JS, TS.)

Each language task delivers:

- [x] The rule table in code, matching the reviewed inventory exactly.
- [x] Fixtures with exact totals for every included rule and zero-contribution
  look-alikes (decisions inside comments, strings and parameter defaults).
- [x] Empty, block and expression bodies; bodyless declarations; nested named
  and anonymous callables; nested types; an inner callable omitted by the query.
- [x] A malformed counted body (`None`) and a malformed excluded nested body
  (outer still measured, valid sibling retains its value).

### Wave 2 (after Wave 1)

- [x] **Remaining languages explicit `None`**: every language not selected
  returns `None` through the engine's default "no reviewed rules" path, with no
  approximate fallback.
- [x] **Metrics matrix**: add the `cyclomatic` support column and expected values
  to `lib/tests/l1/symbols_metrics.rs`; all 16 languages keep a row, either
  reviewed values or `None` with a stated reason. Complexity support is not
  inferred from parameter or doc support.

Validation checkpoint: matrix has 16 explicit rows; `just test` passes.

## Phase 4: Query, cache, adapter and CLI validation

Goal: every consumer sees consistent values and invalidation works.
Tasks in this phase are independent tests plus golden updates, so they can run
as a parallel wave once Phase 3 is green.

### Wave 1 (parallel)

- [x] **Query equivalence**: narrow query equals complete query; a query returning
  only an outer function still gives the pruned total; LF and CRLF source give
  equal values; metrics absent when unrequested; other metrics intact when
  complexity is `None`.
- [x] **Work counters**: declarations-only query does no optional passes or
  complexity traversal (add a focused complexity counter only if the existing
  counters cannot observe it); widening a cache entry to Metrics parses once;
  a warm entry covering Metrics is an actual `CacheLookup::Hit` with no parse;
  a language with unavailable complexity still completes Metrics and caches
  (no perpetual recompute).
- [x] **Cache and schema**: previous schema version and previous extractor
  revision entries are recomputed, never migrated; run the robustness matrix
  from Phase 1 once per format; update `cache_upgrade.rs` and version-sensitive
  tests using the revision current at implementation time.
- [x] **Adapters**: older-model adapters that cannot represent metrics stay
  explicitly lossy and do not invent complexity.
- [x] **CLI**: JSON, JSONL and full-index outputs expose the key wherever Metrics
  is already requested; no command newly requests Metrics; no new filter, sort,
  column, threshold or statistic.
- [x] **Unchanged behaviors**: symbol IDs and qualified names, god-files scores
  and exit codes are unchanged; missing complexity is never treated as zero.

### Wave 2 (after Wave 1)

- [x] **Golden updates**: regenerate `tests/fixtures/golden/symbols.json` and
  `symbols.jsonl` only for the new key and schema version; review the diff is
  limited to those.

Validation checkpoint: `just test`, `just test-l2` and `just lint` pass in
`tree-hugger/`. No new CI environment, gate or performance workload. Windows
and WSL2 are covered by the existing CI schedule; load the `os` skill before
making any claim about not being able to test them.

## Phase 5: Documentation and close-out

Goal: docs describe current behavior; implementation is ready for review.

### Wave 1 (parallel, disjoint files)

- [x] **Counting rules doc**: publish the accepted convention in the `docs/`
  tree, with small source examples per language, the unavailable-form limits,
  and the syntax-only caveats (no macro expansion, no reachability, called
  functions not added, opaque macro token trees and embedded languages, nested
  type field initializers excluded). Written for a developer with no
  repository experience; include a compact example per rule. The doc does not
  name or link this feature.
- [x] **Schema, cache and symbol-report docs**: update
  `docs/symbol-schema-design.md`, `docs/cache-design.md`,
  `docs/symbol-kinds-and-visibility.md` for the new key, version and
  revision, and how to read a number versus `null`.
- [x] **README / API docs and skill**: update tree-hugger README, public API docs,
  and `.claude/skills/tree-hugger/SKILL.md`.
- [x] **Comment drift pass**: review `///` and `//` comments on every changed
  symbol, including any that state how many metrics exist; fix drift on the
  assumption the code is right and report what drifted.

### Wave 2 (after Wave 1)

- [x] **Implementation log**: record the accepted open-question choices and any
  departures from the spec; do not edit closed prerequisite specs.
- [x] **Final verification**: `just test` and `just lint` in `tree-hugger/`;
  confirm every matrix row, rule expectation, query/cache agreement and the docs
  checklist in the spec's completion paragraph.
- [x] **Hand off**: leave the feature active, with status "implementation
  complete, ready for review". Do not run `just complete` and do not commit
  unless told to.
