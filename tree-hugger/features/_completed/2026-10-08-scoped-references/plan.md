---
kind: plan
feature: 2026-10-08-scoped-references
total_phases: 6
created: 2026-10-05
phase: 6
agent: claude/sonnet
yolo: true
packages:
  - tree-hugger
  - tree-hugger-cli
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1:
  - tree-hugger/docs/references.md
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - tree-hugger/lib/src/shared/schema_v2/occurrences.rs
  - tree-hugger/lib/src/shared/schema_v2/mod.rs
  - tree-hugger/lib/src/shared/schema_v2/facets.rs
  - tree-hugger/lib/src/analysis/mod.rs
  - tree-hugger/lib/src/analysis/occurrences.rs
  - tree-hugger/lib/src/analysis/scope/mod.rs
  - tree-hugger/lib/src/analysis/scope/rust.rs
  - tree-hugger/lib/src/analysis/scope/ecma.rs
  - tree-hugger/lib/src/cache/mod.rs
  - tree-hugger/lib/src/cache/symbol_cache.rs
  - tree-hugger/lib/src/file/tree_file.rs
  - tree-hugger/lib/src/file/tree_file/definitions.rs
  - tree-hugger/lib/queries/rust/references.scm
  - tree-hugger/lib/queries/typescript/references.scm
  - tree-hugger/lib/queries/javascript/references.scm
  - tree-hugger/lib/tests/l1/symbols_occurrences.rs
  - tree-hugger/lib/tests/l1/main.rs
  - tree-hugger/lib/tests/l1/cache_tests.rs
docs_updated_during_phase_2:
  - tree-hugger/docs/references.md
  - tree-hugger/docs/symbol-schema-design.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
  - .claude/skills/tree-hugger/SKILL.md
  - .claude/skills/tree-hugger/query-system.md
source_files_during_phase_3:
  - tree-hugger/lib/src/file/query.rs
  - tree-hugger/lib/src/file/tree_file.rs
  - tree-hugger/lib/src/analysis/mod.rs
  - tree-hugger/lib/src/analysis/occurrences.rs
  - tree-hugger/lib/src/analysis/coverage.rs
  - tree-hugger/lib/src/analysis/counters.rs
  - tree-hugger/lib/src/analysis/tests.rs
  - tree-hugger/lib/src/cache/symbol_cache.rs
  - tree-hugger/lib/src/shared/schema_v2/mod.rs
  - tree-hugger/lib/src/shared/schema_v2/facets.rs
  - tree-hugger/lib/tests/l1/symbols_reference_summaries.rs
  - tree-hugger/lib/tests/l1/main.rs
  - tree-hugger/lib/tests/l1/symbols_query.rs
  - tree-hugger/lib/tests/l1/symbols_cache.rs
  - tree-hugger/cli/tests/symbols/cache_robustness.rs
docs_updated_during_phase_3:
  - tree-hugger/docs/references.md
  - tree-hugger/docs/symbol-schema-design.md
  - tree-hugger/docs/cache-design.md
  - tree-hugger/docs/symbol-reports.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
  - .claude/skills/tree-hugger/SKILL.md
  - .claude/skills/tree-hugger/api-reference.md
source_files_during_phase_4:
  - tree-hugger/lib/src/cache/stored_evidence.rs
  - tree-hugger/lib/src/cache/mod.rs
  - tree-hugger/lib/src/cache/symbol_cache.rs
  - tree-hugger/lib/src/analysis/occurrences.rs
  - tree-hugger/lib/src/shared/schema_v2/types.rs
  - tree-hugger/lib/src/file/mod.rs
  - tree-hugger/lib/tests/l1/symbols_cache.rs
  - tree-hugger/lib/tests/l1/tree_file.rs
  - tree-hugger/cli/tests/symbols/cache_robustness.rs
  - tree-hugger/cli/tests/symbols/cache_upgrade.rs
  - tree-hugger/cli/tests/symbols/report.rs
  - tree-hugger/cli/tests/symbols/reports.rs
  - tree-hugger/cli/tests/symbols/wiring.rs
  - tree-hugger/cli/tests/fixtures/golden/symbols.json
  - tree-hugger/cli/tests/fixtures/golden/symbols.jsonl
docs_updated_during_phase_4:
  - tree-hugger/docs/cache-design.md
  - tree-hugger/docs/symbol-schema-design.md
  - tree-hugger/docs/references.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
  - .claude/skills/tree-hugger/SKILL.md
  - .claude/skills/tree-hugger/api-reference.md
source_files_during_phase_5:
  - tree-hugger/cli/src/refs.rs
  - tree-hugger/cli/src/main.rs
  - tree-hugger/cli/src/render.rs
  - tree-hugger/cli/tests/symbols/refs.rs
  - tree-hugger/cli/tests/symbols/main.rs
  - tree-hugger/cli/tests/symbols/terminal_scope.rs
  - tree-hugger/cli/tests/support/terminal_report.rs
  - tree-hugger/cli/tests/level2_declarations.rs
docs_updated_during_phase_5:
  - tree-hugger/docs/references.md
  - tree-hugger/docs/symbol-reports.md
  - tree-hugger/cli/README.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
  - .claude/skills/tree-hugger/SKILL.md
source_files_during_phase_6:
  - tree-hugger/lib/src/cache/symbol_cache.rs
  - tree-hugger/lib/src/cache/mod.rs
  - tree-hugger/lib/src/analysis/scope/mod.rs
  - tree-hugger/lib/src/file/query.rs
  - tree-hugger/cli/src/render.rs
docs_updated_during_phase_6:
  - tree-hugger/docs/references.md
  - tree-hugger/README.md
  - tree-hugger/lib/README.md
docs_created_during_phase_6: []
skills_files_updated_during_phase_6:
  - .claude/skills/tree-hugger/SKILL.md
  - .claude/skills/tree-hugger/api-reference.md
  - .claude/skills/tree-hugger/query-system.md
source_code:
  - tree-hugger/lib/src/shared/schema_v2/occurrences.rs
  - tree-hugger/lib/src/shared/schema_v2/mod.rs
  - tree-hugger/lib/src/shared/schema_v2/facets.rs
  - tree-hugger/lib/src/analysis/mod.rs
  - tree-hugger/lib/src/analysis/occurrences.rs
  - tree-hugger/lib/src/analysis/scope/mod.rs
  - tree-hugger/lib/src/analysis/scope/rust.rs
  - tree-hugger/lib/src/analysis/scope/ecma.rs
  - tree-hugger/lib/src/cache/mod.rs
  - tree-hugger/lib/src/cache/symbol_cache.rs
  - tree-hugger/lib/src/file/tree_file.rs
  - tree-hugger/lib/src/file/tree_file/definitions.rs
  - tree-hugger/lib/queries/rust/references.scm
  - tree-hugger/lib/queries/typescript/references.scm
  - tree-hugger/lib/queries/javascript/references.scm
  - tree-hugger/lib/tests/l1/symbols_occurrences.rs
  - tree-hugger/lib/tests/l1/main.rs
  - tree-hugger/lib/tests/l1/cache_tests.rs
  - tree-hugger/lib/src/file/query.rs
  - tree-hugger/lib/src/analysis/coverage.rs
  - tree-hugger/lib/src/analysis/counters.rs
  - tree-hugger/lib/src/analysis/tests.rs
  - tree-hugger/lib/tests/l1/symbols_reference_summaries.rs
  - tree-hugger/lib/tests/l1/symbols_query.rs
  - tree-hugger/lib/tests/l1/symbols_cache.rs
  - tree-hugger/cli/tests/symbols/cache_robustness.rs
  - tree-hugger/lib/src/cache/stored_evidence.rs
  - tree-hugger/lib/src/shared/schema_v2/types.rs
  - tree-hugger/lib/src/file/mod.rs
  - tree-hugger/lib/tests/l1/tree_file.rs
  - tree-hugger/cli/tests/symbols/cache_upgrade.rs
  - tree-hugger/cli/tests/symbols/report.rs
  - tree-hugger/cli/tests/symbols/reports.rs
  - tree-hugger/cli/tests/symbols/wiring.rs
  - tree-hugger/cli/tests/fixtures/golden/symbols.json
  - tree-hugger/cli/tests/fixtures/golden/symbols.jsonl
  - tree-hugger/cli/src/refs.rs
  - tree-hugger/cli/src/main.rs
  - tree-hugger/cli/src/render.rs
  - tree-hugger/cli/tests/symbols/refs.rs
  - tree-hugger/cli/tests/symbols/main.rs
  - tree-hugger/cli/tests/symbols/terminal_scope.rs
  - tree-hugger/cli/tests/support/terminal_report.rs
  - tree-hugger/cli/tests/level2_declarations.rs
documentation:
  - tree-hugger/docs/references.md
  - tree-hugger/docs/symbol-schema-design.md
  - tree-hugger/docs/cache-design.md
  - tree-hugger/docs/symbol-reports.md
  - tree-hugger/cli/README.md
  - tree-hugger/README.md
  - tree-hugger/lib/README.md
completed_phase: 6
implemented: true
---

# Plan: Scope-Aware References and `hug refs`

## Summary and Definition of Done

Replace first-match binding in `bind_references` (`lib/src/analysis/mod.rs`)
with a per-file lexical scope tree, emit per-occurrence evidence
(`occurrences`, `reference_coverage`) on `FileSymbolIndex`, derive the existing
relationship summaries and an honest `is_recursive` from that evidence, thread a
new `Capability::Occurrences` through query, coverage projection and cache
(schema 5.0, extractor revision 11), and add `hug refs`.

Done means all of the following are observable:

- [x] A shadowed local, a same-named method on another type, and `other.method()` no longer bind to the wrong declaration; each is `Scoped`, `NameOnly` or `Unresolved` as the spec defines.
- [x] `semantics.is_recursive` is `Yes` only for a `Scoped` direct `Call` to own ID; `No` only with complete call coverage; otherwise `Unknown`; `None` when no analysis requested.
- [x] Cold, warm, capability-widened and kind-narrowed results equal uncached analysis; a real cache hit, malformed-evidence rejection and old-version invalidation are asserted; work counters prove skipped/non-duplicated work.
- [x] All 16 languages have an explicit supported or limited state in the language table (no language is silently empty).
- [x] `hug refs` works in text and `--json`, states the file-local boundary (also when empty), and never turns a cross-file same-name call into a resolved incoming reference.
- [x] `just test`, `just lint` and `just test-l2` pass in `tree-hugger/`; docs, READMEs and `.claude/skills/tree-hugger/` are updated; departures are in the implementation log. Agent's terminal state is "implementation complete, ready for review" (no move to `_completed`, no `just complete`).

Dependencies (`2026-10-03-symbol-improvements`, `2026-10-03-symbol-filters-and-reports`, `2026-10-05-deferred-kind-precision`) are in the working tree; Phase 1 confirms they are landed before Phase 3 begins.

## Phase 1: Rulings, Scope Study and Language Table

Goal: remove ambiguity and size the work before any production code changes.

### Necessary Rules

The spec leaves these for the author. Each carries a recommended default so work can proceed (`yolo: true`); record the accepted outcome in the implementation log before Phase 3.

1. **First-release language breadth** (spec Open Question 1). Default: option 2, proven Rust and TypeScript forms resolve; the other 14 languages (JavaScript shares the ECMA/TypeScript query; decide whether JS ships resolved with TS — default yes, since the query and grammar rules are shared) emit conservative occurrences and `Limited` coverage with reason codes. Limited rows never count as resolved forms.
2. **Receiver inference** (Open Question 2). Default: option 1, resolve only explicit local paths and receiver forms whose language rules prove one declaration; `self`/`this` method calls stay `NameOnly` candidates (dispatch can select another implementation) except Rust inherent `Self::f` / `self.f()` where no trait method of that name exists in-file and one inherent declaration matches (language table must state this explicitly or drop it).
3. **Schema number.** Spec says 5.0 from the 4.0 baseline; confirm no other pending feature has already advanced to 5.0 (read `docs/symbol-schema-design.md` and the other `features/2026-10-*` specs). If it has, take the next major.
4. **Report envelope.** `hug refs` JSON gets its own envelope; ruling needed on whether `REPORT_SCHEMA_VERSION` (`cli/src/report.rs`, currently 1) bumps for existing reports (spec: "update report versions where envelope shapes change"). Default: existing declaration/stats envelopes carry the new `occurrences`/`reference_coverage` only under `--json-full`, so their report version bumps to 2 with the schema change; `refs` starts at its own version 1 within the same constant if the envelope is shared.
5. **Hoisting/order rules by language** (Rust: items hoist within a block, `let` does not and initializer sees outer binding; TS: `let`/`const` TDZ hides outer name for the whole block, `function` and `var` hoist; Python: assignment makes name local for the whole function unless `global`/`nonlocal`). Default: encode exactly these three plus the table's rows; anything else is `Limited`.
6. **Ownerless regions.** Default-argument, decorator and initializer evaluation: Rust has no default args; TS/Python parameter defaults and decorators are owned by the enclosing scope (not the function), field initializers by the type/constructor per language rule; if unsure the owner is absent and the region is `Limited`.
7. **Anonymous callable owner identity.** `owner` absent, `owner_span` set; coverage regions keyed by span. No synthetic IDs.
8. **Reason-code vocabulary** for `Limited` (`unsupported_binding_form`, `missing_call_capture`, `parse_error`, `indirect_dispatch`, `unsupported_language`, `ownership_unknown`, `empty_reference_query`). Fixed as a serialized enum; new codes are additive.
9. **Wider performance measurement.** Spec requires none. The indexed-lookup timing sample, if taken, is one host and a quick sample; any wider measurement is listed in the implementation log for the author, not run.

### Spikes

One spike, run once, before the work it informs (the spec's "scope study"). No re-checks in later phases.

- [x] **Scope study (Rust and TypeScript).** Build scope trees from the effective `@local.scope` captures (`lib/queries/rust`, `lib/queries/typescript/locals.scm`, inherited queries) and compare against first-match binding on the existing fixture corpus; list targets that change, classified as corrected vs. still uncertain. Record scope and reference-query gaps (e.g. scope captures attached to import/declaration nodes, missing member-call captures). Output: a short "Scope study" section in `implementation-log.md` sizing Phase 2. Throwaway code only; not a benchmark; changed-target counts are not correctness evidence.

### Tasks

- [x] **Confirm prerequisites.** Verify the three `depends-on` features are in the tree: candidate-name index and selective capabilities (`Capability` enum in `lib/src/file/query.rs`), qualified names, constructor kinds. Note any gap in the log.
- [x] **Inventory captures.** For all 16 languages, list effective scope, definition and reference captures (`lib/queries/*`, including `vendor/` and inherited). Wave 1 can fan out by language group.
- [x] **Scope study spike.** As above (depends on inventory for Rust/TS).
- [x] **Language/form support table.** Write the author-approved table (per language: block vs function scope, parameters/destructuring, loop/catch/pattern bindings, use-before-declaration, hoisting, nested-callable capture, imports/aliases, type vs value names; each row `Resolved` or `Limited` with reason). Draft in `docs/` as **planned**; rows cannot be `Resolved` for non-Rust/TS languages under default ruling 1.
- [x] **Record rulings.** Write accepted rulings 1–9 into `implementation-log.md`.

#### Wave 1 (concurrent)

- Inventory captures for language groups (A: Rust, TS, JS, Go; B: Python, Perl, Lua, Bash, Zsh; C: C, C++, C#, Java, Scala, Swift, PHP).
- Prerequisite verification.

#### Wave 2

- Scope study (needs Wave 1 Rust/TS inventory), then language table and rulings.

**Checkpoint 1:** rulings recorded, table drafted, scope study written; sizing note for Phase 2 exists.

## Phase 2: Occurrence Model and Scoped Resolution Core

Goal: the shared model, scope tree and lookup, proven on Rust and TypeScript, with no schema/cache/CLI changes yet visible to consumers.

### Tasks

- [x] **Occurrence types.** Add `Occurrence`, `OccurrenceKind` (`Call`/`Usage`), `Confidence` (`Scoped`/`NameOnly`/`Unresolved`), `ReferenceCoverage`, coverage state and reason-code enum to `lib/src/shared/schema_v2/`; add `occurrences` and `reference_coverage` to `FileSymbolIndex` (`schema_v2/mod.rs`). Spans reuse `TextSpan` (one-based line/column, half-open byte offsets). Serde defaults must be additive for public output.
- [x] **Capture collection.** During the existing definitions-query traversal (`lib/src/file/tree_file*`), collect scope captures and keep binding facts for **all** declarations including kinds omitted from results (so uncaptured shadowing bindings block outer matches). Syntax needed only for resolution stays optional when references are not requested.
- [x] **Scope tree.** New module (e.g. `lib/src/analysis/scope.rs`): file-root scope, parent links, per-scope declaration tables keyed by language name category (Rust type/value; TS type/value), indexed lookup (no per-occurrence scan of all declarations). Declarations assigned to the scope where the name is visible (function name in enclosing scope, parameters in body scope).
- [x] **Reference extraction queries.** Extend `references.scm` for Rust and TypeScript (and JS via ECMA) to capture invocation targets, member names in `object.run()` (`object` Usage, `run` Call), `consume(run)` (`consume` Call, `run` Usage), constructors, macro invocations (Usage of macro; unknown expansion limits coverage). Exclude definition names, import binding names, comments, strings. Update query provenance notes alongside every vendored query edit.
- [x] **Lookup rules (Rust, TS).** Implement ruled order/hoisting/shadowing (the three spec counter-examples), type/value categories, import aliases (local import binding, absent external ID, source and original name kept in import record), qualified targets resolved only when full path/receiver is established and unique, qualifier preserved on unresolved targets, overloads/wildcards/dynamic access never unique.
- [x] **Confidence assignment.** `Scoped` only for one established binding; `NameOnly` with distinct sorted candidates (no target ID); `Unresolved` otherwise; method targets stay candidates unless one declaration is established.
- [x] **Ownership and coverage.** Owner = nearest enclosing callable record; anonymous callable boundaries retained (owner absent, `owner_span` set); file-level code and initializers kept; defaults/decorators per ruling 6. Emit `reference_coverage` per region (named bodies even with zero occurrences; anonymous boundaries); empty reference queries and parse errors → `Limited`.
- [x] **Non-supported languages.** Conservative identifier/call occurrences where an existing `references.scm` allows, all regions `Limited(unsupported_language)`, targets never `Scoped`. Explicit state for all 16 languages.
- [x] **Determinism.** Dedupe overlapping captures of the same occurrence (not distinct uses); sort by start byte, end byte, owner span, kind, written target, confidence; candidates by declaration position then ID. No hash-map iteration order dependence.

#### Wave 1 (concurrent, disjoint files)

- Occurrence/coverage types and `FileSymbolIndex` fields.
- Capture collection in the definitions traversal.
- Rust and TypeScript `references.scm` edits plus provenance notes.

#### Wave 2

- Scope tree and lookup (needs Wave 1 types and captures).

#### Wave 3 (concurrent)

- Ownership/coverage; non-supported-language handling; determinism pass.
- L1 tests (see below) written by a test agent against the Wave 2 API.

### Tests (L1, `lib/tests/l1/`, new `symbols_occurrences.rs`)

- [x] Shadowing: Rust `let f = f();`, TS block `let f` before init, Python local-by-assignment (Python asserts `Limited`, not a wrong binding, under default ruling 1).
- [x] Two types with same-named methods stay `NameOnly`; `other.method()` never binds to caller; unrelated/dynamic receivers uncertain; overload ambiguity has no target ID.
- [x] Spans/qualifiers incl. non-ASCII source; repeated call sites on one line stay two records; overlapping captures dedupe; constructors; file-level initializers; anonymous callables with distinct `owner_span`; nested named functions; declaration names and comment/string text excluded.
- [x] Alias import: `Scoped`, absent target ID, import record keeps source and original name.
- [x] Per-language state test over all 16 languages (supported or limited with reason; never empty-and-passing).
- [x] Determinism: shuffled capture order yields identical output.

**Checkpoint 2:** `just test` in `tree-hugger/` green; occurrences and coverage computed for Rust/TS; old summary behavior untouched.

## Phase 3: Summaries, Recursion and Selective Query

Goal: single extraction/resolution path feeding summaries, recursion and capabilities.

### Tasks

- [x] **Replace `bind_references`.** Derive `references` (established local targets and bound imports), `dependencies` (bound imports), `referenced_by` (declaration owners of established local targets) in first-occurrence order; keep dedupe by resolved ID else name+qualifier; uncertain occurrences excluded; ownerless occurrences make no reverse edges; containment and export binding unchanged. Diagnostic severity/confidence/default enablement must not change (assert with existing lint tests).
- [x] **Honest recursion.** Rewrite `semantic_pass` (`analysis/mod.rs`): `Yes` only for `Call` + own ID + `Scoped`; `Yes` wins over other uncertainty; else `Unknown` on limited coverage or potentially recursive unresolved/indirect call; `No` requires complete call coverage and none of that uncertainty; bodyless declarations `Unknown`; self-value reference and mutual recursion are not direct recursion; nested callable owners isolated; `None` when no semantics requested. Legacy adapter (`file/tree_file.rs` `is_recursive: false`) keeps mapping `Unknown`/`None` → `false` and gains no occurrence fields.
- [x] **`Capability::Occurrences`.** Add to `lib/src/file/query.rs`; `Occurrences` requires `References`; `Semantics` requires `Occurrences`; `References` alone builds summaries then discards occurrence output; complete indexes and `--json-full` request it; declaration/containment reports still skip reference work. `for_query`/`passes` in `analysis/mod.rs` updated.
- [x] **Whole-file occurrence semantics.** Occurrences and coverage describe the whole file independent of selected kinds/predicates; may name unselected declarations; dropping the capability clears occurrences and coverage.
- [x] **Work counters.** Extend `analysis/counters.rs` so skipped and non-duplicated work (no second definitions query or parse) is observable.

#### Wave 1

- Summaries derivation and recursion (same file region; one agent), concurrently with `Capability` wiring (`file/query.rs`, `for_query`), which only shares the pass list.

#### Wave 2

- Tests: recursion matrix (direct self-call `Yes`; self-value `No` only if complete; unresolved/indirect call, missing capture, parse error `Unknown`; nested call cannot prove enclosing recursion; proven self-call wins over uncertain), summary derivation order/dedupe, no fabricated reverse edges, kind-narrowed equals projected wide, counters.

**Checkpoint 3:** `just test` and `just lint` green; recursion table above passes; lint diagnostics fixtures unchanged.

## Phase 4: Schema 5.0, Coverage Projection and Cache

Goal: persist and project the new evidence safely.

### Tasks

- [x] **Schema bump.** Move 4.0 → 5.0 (per ruling 3) on records, indexes, envelopes and fingerprints; bump `EXTRACTOR_REVISION` ("10" → "11", `lib/src/file/mod.rs`); update report versions per ruling 4. Declaration IDs unchanged where identity unchanged (assert against golden fixtures; regenerate `cli/tests/fixtures/golden/symbols.json{,l}` only for intended shape changes).
- [x] **Coverage projection** (`lib/src/analysis/coverage.rs`). Widening by occurrences requires syntax unless complete occurrence evidence is stored; widening by semantics uses stored occurrences + coverage (summaries alone insufficient); resolve before discarding light records; never rerun definitions query or parse to build a second representation.
- [x] **Cache** (`lib/src/cache/symbol_cache.rs`). Store occurrences in-file names only (scan-root-relative prefixes applied only at presentation); keep one-entry-per-file; strict read validation (spans in range and on char boundaries, confidence/target-ID/candidate consistency, capability claims backed by evidence). Missing or malformed required data is a cache miss, never a computed empty result. Public serde defaults for additive lists must not let the strict reader accept an entry that claims `Occurrences` but omits them (use a present-required deserializer for the cache path).
- [x] **Docs for schema** (`docs/symbol-schema-design.md`, `docs/symbol-schema.md`, `docs/cache-design.md`): current behavior, with decided-but-unbuilt items marked **planned**.

### Input Robustness Matrix (cache entry, occurrence evidence)

Load-bearing fields: `capabilities` claim, `occurrences`, `reference_coverage`, per-occurrence `confidence`/`target.id`/`candidates`/`span`. Outcomes are the public result (cache hit vs. recompute giving correct output), asserted through the cache lookup API, one test walking the matrix from a real-tool fixture entry with one edit per cell.

| Shape | `occurrences` | `reference_coverage` | `confidence`/`id`/`candidates` | `span` | Required outcome |
|---|---|---|---|---|---|
| control (unedited) | present | present | consistent | valid | cache hit, result equals uncached |
| absent key | omitted while claiming `Occurrences` | omitted | key omitted | key omitted | miss + recompute (not empty) |
| explicit null | `null` | `null` | `null` | `null` | miss (null ≠ absent ≠ empty) |
| wrong type, whole field | number | string | number | string | miss |
| wrong type, one element | `[valid, 123]` | `[valid, 123]` | one bad item | n/a | miss (no silent filtering) |
| wrong type, every element | `[123]` | `[123]` | all bad | n/a | miss, never empty |
| empty | `[]` with claim and real occurrences expected from a file that has calls | `[]` | `NameOnly` with `[]` candidates; `Scoped` with candidates; `Unresolved` with ID | zero-length or end < start, out of file range | `[]` accepted only when the file genuinely has none and coverage says so; inconsistent rows miss |
| duplicate key | twice | twice | twice | twice | miss (strict reader rejects) |
| trailing/invalid content | valid entry + garbage | same | same | same | miss |

Code smells to grep before declaring done: `#[serde(default)]` on these fields in the cache read path, `unwrap_or_default()`, `.ok()` and `filter_map` on occurrence parsing.

#### Wave 1 (concurrent)

- Schema/version bump + golden regeneration.
- Coverage projection changes.
- Cache read/write and validation.

(Cache and projection both touch shared types; the version-bump agent lands first in the same wave via a short ordered sub-step: bump → then projection and cache in parallel.)

#### Wave 2

- Tests: cold/warm/widened/narrowed equal uncached; actual cache hit asserted; malformed-evidence rejection (matrix); old version invalidated; counters prove no duplicate work. Docs updates in parallel.

**Checkpoint 4:** `just test`, `just lint` green; goldens reviewed; matrix test passes.

## Phase 5: `hug refs` CLI

Goal: the command, in text and JSON.

### Tasks

- [x] **Command wiring** (`cli/src/main.rs`, new `cli/src/refs.rs`). `hug refs [<symbol> | --id <id>] [file-globs...]`; exactly one selector; `--callers`/`--callees` mutually exclusive (clap conflict); `--id` case-sensitive; reuse discovery, ignore rules, language overrides, file-globs and cache options; reject unsupported output formats and selectors with a clear error (only text and `--json`).
- [x] **Selection.** Names use existing exact smart-case rules, language separators, file-prefixed qualified names and trailing `!`; ambiguity checked across all selected files, kinds, scopes including locals (do not inherit the hidden-local default); multiple declarations → nonzero error listing locations and IDs with no partial output; unknown name/ID → success with an empty report.
- [x] **Report model.** Incoming: established rows matching target ID; separate "name-only suggestions" group only when candidates include the ID; unresolved same-name references never attached. Callers = incoming `Call`; callees = calls owned by selected declaration, including unresolved and import targets with visibly explained absent IDs. Default lists `Call` and `Usage` grouped by owner; each call site kept; file-level and anonymous owners labeled with spans. `--id` cannot upgrade uncertainty. Request `Occurrences` capability; apply scan-root prefixes only for presentation.
- [x] **JSON.** Versioned envelope per existing conventions: selected declaration identity, per-file occurrences, coverage, completed capabilities, file-local boundary statement (present on empty reports).
- [x] **Text rendering.** Through `biscuit-terminal` components (`Prose`, `UnorderedList`, existing location links/wrapping); boundary note always printed, including when empty.
- [x] **Boundary behavior.** A cross-file same-name call is never an incoming resolved reference.

#### Wave 1 (concurrent)

- CLI args/selection/ambiguity (`refs.rs`, `main.rs`).
- JSON envelope and report model types (`report.rs`).
- Text renderer (`render.rs`-adjacent, terminal components).

#### Wave 2

- Integration: wire the three, then tests.

### Tests

- [x] L1 CLI (`cli/tests/symbols/` new `refs.rs`): exact/smart-case, duplicate names across files (ambiguity error, no partial stdout), `--id`, local targets, empty reports, conflicting flags, unsupported format rejection, versioned JSON, callers/callees exclude `Usage`, repeated calls preserved, uncertain groups separate, cross-file same-name not resolved.
- [x] L2 (`just test-l2`) terminal report via the shared harness (see `rust-testing` and `biscuit-test-harness` skills), no window focus; use the Test Toolkit.

**Checkpoint 5:** `just test`, `just test-l2`, `just lint` green in `tree-hugger/`.

## Phase 6: Documentation, Skill, Cross-OS and Close-out

### Tasks

- [x] **Docs.** Update area, `lib`, `cli` READMEs; `docs/symbol-schema*.md`, cache and report docs (`docs/symbol-reports.md`), `docs/symbol-kinds-and-visibility.md` if affected. Explain confidence, language limitations (table from Phase 1, remove **planned** markers for what landed), direct recursion, schema migration (4.0 → 5.0, recompute under exact-version policy) and the file-local boundary, each with a compact example; audience is a developer new to the repo; add a Mermaid diagram of the recursion decision flow. Current docs do not link to or name this dated feature.
- [x] **Skill.** Update `.claude/skills/tree-hugger/` (`SKILL.md`, `api-reference.md`, `query-system.md`) for occurrences, `Capability::Occurrences`, `hug refs`.
- [x] **Drift pass.** Per CLAUDE.md, sweep doc comments in every changed symbol; fix drift in favor of code and report what drifted.
- [x] **OS review.** Load the `os` skill; check new path/prefix code and tests for Windows path spelling and symlinked temp dirs; no new CI environments or gates. Local macOS run only unless the `os` skill indicates a cheap Linux/Windows host is reachable; record what was and was not run.
- [x] **Quick timing note (optional).** One host, one quick sample of indexed lookup on a large fixture; record in the log. Wider measurement listed for the author only (ruling 9).
- [x] **Implementation log.** Record departures from spec, accepted rulings, scope study outcome and limitations; set spec status to `implemented` per repo convention. Do not move the feature to `_completed` and do not run `just complete`.

#### Wave 1 (concurrent)

- READMEs and `docs/` pages.
- Skill update.
- Drift pass and OS review.

#### Wave 2

- Full validation: `just test`, `just test-l2`, `just lint`; log results.

**Checkpoint 6 (final):** all Definition-of-Done boxes checked, implementation ready for review.
