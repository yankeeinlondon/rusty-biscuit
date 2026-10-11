---
spec: /Volumes/coding/wt/rusty-biscuit/feat-symbols/tree-hugger/features/2026-10-11-reexports-all-languages/spec.md
plan: tree-hugger/features/2026-10-11-reexports-all-languages/plan.md
implemented_by: claude/sonnet
started_phase: 1
source_files_during_phase_1: []
docs_updated_during_phase_1:
  - tree-hugger/features/2026-10-11-reexports-all-languages/spec.md
docs_created_during_phase_1:
  - tree-hugger/features/2026-10-11-reexports-all-languages/audit/notes-module-syntax.md
  - tree-hugger/features/2026-10-11-reexports-all-languages/audit/notes-static-conventions.md
  - tree-hugger/features/2026-10-11-reexports-all-languages/audit/notes-interface-boundaries.md
  - tree-hugger/features/2026-10-11-reexports-all-languages/audit/samples/ (279 sample files across 16 languages)
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - tree-hugger/lib/src/shared/schema_v2/reexports.rs
  - tree-hugger/lib/src/shared/schema_v2/mod.rs
  - tree-hugger/lib/src/shared/schema_v2/types.rs
  - tree-hugger/lib/src/analysis/reexports.rs
  - tree-hugger/lib/src/analysis/mod.rs
  - tree-hugger/lib/src/analysis/coverage.rs
  - tree-hugger/lib/src/analysis/counters.rs
  - tree-hugger/lib/src/file/query.rs
  - tree-hugger/lib/src/file/mod.rs
  - tree-hugger/lib/src/file/tree_file.rs
  - tree-hugger/lib/src/cache/mod.rs
  - tree-hugger/lib/src/cache/stored_evidence.rs
  - tree-hugger/lib/src/cache/symbol_cache.rs
  - tree-hugger/cli/src/refs.rs
  - tree-hugger/lib/tests/l1/main.rs
  - tree-hugger/lib/tests/l1/symbols_reexports.rs
  - tree-hugger/lib/tests/l1/symbols_cache.rs
  - tree-hugger/lib/tests/l1/cache_tests.rs
  - tree-hugger/cli/tests/symbols/cache_robustness.rs
  - tree-hugger/cli/tests/symbols/cache_upgrade.rs
  - tree-hugger/cli/tests/symbols/refs.rs
  - tree-hugger/cli/tests/symbols/report.rs
  - tree-hugger/cli/tests/symbols/reports.rs
  - tree-hugger/cli/tests/symbols/wiring.rs
  - tree-hugger/cli/tests/fixtures/golden/symbols.json
  - tree-hugger/cli/tests/fixtures/golden/symbols.jsonl
docs_updated_during_phase_2:
  - tree-hugger/features/2026-10-11-reexports-all-languages/spec.md
  - tree-hugger/features/2026-10-11-reexports-all-languages/implementation-log.md
  - tree-hugger/docs/symbol-records.md
  - tree-hugger/docs/symbol-records/file-index.md
  - tree-hugger/docs/symbol-records/versions.md
  - tree-hugger/docs/cache-design/entry-format.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
  - .claude/skills/tree-hugger/analysis-pipeline.md
  - .claude/skills/tree-hugger/api-reference.md
  - .claude/skills/tree-hugger/testing.md
source_files_during_phase_3: []
docs_updated_during_phase_3:
  - tree-hugger/features/2026-10-11-reexports-all-languages/implementation-log.md
  - tree-hugger/features/2026-10-11-reexports-all-languages/spec.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4: []
docs_updated_during_phase_4:
  - tree-hugger/features/2026-10-11-reexports-all-languages/implementation-log.md
  - tree-hugger/features/2026-10-11-reexports-all-languages/spec.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_files_during_phase_5: []
docs_updated_during_phase_5:
  - tree-hugger/features/2026-10-11-reexports-all-languages/implementation-log.md
  - tree-hugger/features/2026-10-11-reexports-all-languages/spec.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5: []
source_files_during_phase_6: []
docs_updated_during_phase_6:
  - tree-hugger/features/2026-10-11-reexports-all-languages/implementation-log.md
  - tree-hugger/features/2026-10-11-reexports-all-languages/spec.md
docs_created_during_phase_6: []
skills_files_updated_during_phase_6: []
packages:
  - tree-hugger
  - tree-hugger-cli
---

# Implementation Log for 2026-10-11-reexports-all-languages (6 phases)

## Phase 1

Phase 1 is an evidence and design phase. No Rust source changed, so there are
no behavior changes to map to tests; the requirement-to-test mapping is
"none, audit only". The 279 samples are intended as Phase 3 regression fixtures.

### What was done

- Built a disposable AST probe outside the repo (`/tmp/th-probe`, lockfile-pinned
  grammars, tree-sitter 0.26.9) that prints named nodes, field names and
  ERROR/MISSING flags. It hides anonymous tokens; one worker made a private
  variant to read them.
- Wave 1: three foreground subagents audited, in parallel, module syntax (Rust,
  JS, TS, Swift, Scala), static conventions (Python, Go, Perl, Lua) and
  interface boundaries (Java, PHP, Bash, Zsh, C, C++, C#). Each delivered a notes
  file under `audit/` and positive/negative samples under `audit/samples/<lang>/`.
- Wave 2 (consolidate): added an "Audit results" section and an "Author decision
  package" to `spec.md`. Checked consumers: the placeholder
  `reexported_symbols()` is already removed (only the release note remains);
  `cli/src/main.rs` still builds `ImportSymbol` with `is_reexport: false`;
  `EXTRACTOR_REVISION` is "51"; `SchemaVersion::CURRENT` is V5_1.

### Findings that matter

- C++ blocker: tree-sitter-cpp 0.23.4 (newest published) cannot parse C++20
  modules; `import x;`/`module m;` parse as ordinary declarations with no error.
- Silent misparse in Perl (`@Pkg::EXPORT_OK`) with no error flag, so coverage
  cannot rest on `has_error`.
- Scala restricted/annotated `export` yields ERROR but the export is recovered,
  so a restricted export would read as public unless limited.
- TypeScript `export type *` yields ERROR; JS import attributes yield ERROR.
- Java, PHP, Bash, Zsh, C and C# `using`/`import` forms: no applicable
  construct (borderline cases listed for the author).
- Several semantic claims are marked "source not verified" in the notes (e.g.
  `pub extern crate`, Scala 3 release, `@_exported` on selective imports).

### Not done (blocked on the author)

- Task "Freeze support contract" remains unchecked: it requires the author's
  rulings, and this run is non-interactive. The decision package is in the spec;
  `human_review: true` is set on the spec. The spec status stays `draft-spec`.

### Gates

- Scope command (`affected_scope.py` over the working tree) returned
  `"packages": ["tree-hugger","tree-hugger-cli","test-toolkit"]`,
  `change_class: "package"` (the tree also holds earlier uncommitted work), so
  the gates were run.
- `just test` in `tree-hugger/`: 2089 passed, 0 failed, 26 slow, 0 skipped.
- `just lint`: clean for tree-hugger and tree-hugger-cli.
- No OS-specific code changed; no cross-check run was needed.

## Phase 2

Implemented the shared report model and the analysis seam. (The earlier
"not implemented" stop was void once the author's rulings landed; see
"Author rulings".) Per-language extraction is Phase 3.

### What was built

- `shared/schema_v2/reexports.rs`: `ReexportReport`, `ReexportCoverage`,
  `ReexportCoverageState`, `ReexportLimit`/`Reason`, `ReexportEntry`,
  `ReexportForm`, `ReexportEvidence`, `ReexportCaveat`, exactly as pinned in
  the spec. Every key is always written; no `serde(default)`/`skip_serializing_if`;
  nullable roles use a `nullable` deserializer (absent key is an error, not
  `None`); `deny_unknown_fields`. `ReexportEntry::forwards_import_at` is the
  single rule for the import marker (`exposed` never marks).
- `ImportRecord::reexported` (required bool) and `FileSymbolIndex::reexports`
  (`Option`, key omitted when `None`, null refused on read).
- `SchemaVersion::V5_2` is `CURRENT`; `EXTRACTOR_REVISION` is `"52"`, with the
  `SchemaBump` upgrade row for revision 51.
- `Capability::Reexports` (last in `ALL`; prerequisite `Declarations`);
  `Diagnostics` now requires it.
- `analysis/reexports.rs`: `extract` (one call per analysis, counted by the new
  `WorkCounters::reexport_extractions`) and `mark_forwarded_imports` (exact
  import span match). `extract` matches every language: Java, C, Bash, Zsh,
  PHP are `not_applicable`; the other 11 are a whole-file
  `limited`/`grammar_gap` placeholder until Phase 3 replaces each arm, so an
  unimplemented language can never read as a `complete` negative.
- `PassPlan::reexports`; `coverage::project` drops the report and clears the
  markers together; `record_passes_suffice` never widens `Reexports` without a
  parse.
- `TreeFile::reexports()`.
- Cache: `reexports` (required, nullable) in the entry, `reexported` in each
  stored import (and in the import-inventory digest, whose pinned value moved
  with the schema), and two consistency checks (report iff capability; marker
  needs capability). The full invariant list and matrix are Phase 4.
- CLI `refs.rs`: boxed `FileSymbolIndex` in `Phased::Analyzed` (clippy
  `large_enum_variant` after the index grew). No behavior change.

### Requirement-to-test mapping

| Behavior | Test |
|---|---|
| types round-trip with every key; absent key refused, null name read; unknown key/variant refused; constructors' JSON | `schema_v2::reexports::tests::*` (unit) |
| marker only for `syntax`/`declared` at the exact import span | `analysis::reexports::tests::*` (unit) |
| capability order, closure, `Diagnostics` needs `Reexports`, no reference prerequisites | `symbols_reexports::capability_closure_and_order_are_pinned` |
| uncomputed is absent (no key, no capability, markers false), not empty | `symbols_reexports::an_unrequested_report_is_absent_not_empty` |
| all 16 languages: 5 `not_applicable`, 11 limited whole-file with root span | `symbols_reexports::every_language_reports_not_applicable_or_a_limited_whole_file` |
| whole-file report under kind filter; direct API equals query | `symbols_reexports::the_report_describes_the_whole_file_whatever_kinds_are_selected`, `the_complete_index_carries_the_report_and_the_capability` |
| one extraction, none unrequested, no reference captures/resolutions/diagnostics for a cheap request | `symbols_reexports::extraction_runs_once_and_only_when_required` |
| cache hit, narrowing, widening equal uncached | `symbols_reexports::a_cached_report_equals_the_uncached_report_across_a_narrowing_and_a_widening` |
| damaged entries (report/capability mismatch, absent key, stray marker) recomputed and rewritten | `symbols_reexports::an_entry_whose_report_and_capability_disagree_is_recomputed` |
| revision 51 entries not served | `cache_upgrade::entry_from_extractor_revision_fifty_one_is_not_served_after_the_reexports_bump` |
| real `hug --json-full` carries the report, `--json` does not | `wiring::json_full_carries_the_reexport_report_and_json_does_not` |

Existing tests updated for the version/revision/capability changes: schema
5.1 literals, golden files, the pinned import-inventory digest, the
incompatible-version lists, `cache_robustness` expectations.

### Not done here (by plan)

Per-language extraction (Phase 3); persistence strictness beyond the two
checks, the JSON matrix, import-site linkage for `unused-import`,
`imported_symbols()`/CLI `is_reexport` (Phase 4); `docs/reexports.md` (Phase 5).
Docs updated now: `symbol-records*`, `cache-design/entry-format.md`, with the
extractors marked planned.

### Gates

- Scope command: `"packages": ["tree-hugger","tree-hugger-cli","test-toolkit"]`,
  `change_class: "package"`, so both gates ran.
- `just test` in `tree-hugger/`: 2106 passed, 0 failed.
- `just lint`: clean for tree-hugger and tree-hugger-cli.
- OS: no `cfg` or path code touched; no cross-check run (macOS only).

## Phase 3

Phase 3 was **not implemented**. It is blocked twice over.

- It depends on Phase 2 (report model and planner seam), which was itself not
  implemented; no `shared/schema_v2` types, capability or extraction seam exist
  for language workers to target.
- Phase 2 depends on the author's rulings (support contract, canonical
  representation, consumer exposure, C++ modules grammar blocker). Task "Freeze
  support contract" is still unchecked and the spec is still `draft-spec` with
  `human_review: true`. Per-language contracts cannot be approved by an agent.
- No source, test or skill changed; no plan tasks were checked off.
- Gates: only docs under `tree-hugger/features/` changed (log, plan and spec
  frontmatter), the same set as Phase 2, whose scope result was
  `"packages": []`, `change_class: "documentation"`. The gates were skipped as
  permitted; no behavior changed so no tests were mapped.

## Phase 4

Phase 4 was **not implemented**. It is blocked transitively.

- It persists and consumes the forwarding report, schema, capability and
  import marker that Phase 2 defines and Phase 3 populates. Neither was built:
  no report type, capability, schema bump or extraction exists to cache or to
  convert in `analysis::import_record` and `cli/src/main.rs`.
- Both depend on the author's rulings (support contract, canonical
  representation, consumer exposure, C++ grammar blocker). The spec is still
  `draft-spec` with `human_review: true`; an agent cannot approve them.
- No source, test or skill changed; no plan tasks were checked off, and no
  behavior changed, so there is no requirement-to-test mapping.
- Gates: only docs under `tree-hugger/features/` changed. The scope result is
  recorded below.
- Scope command result: `"packages": []`, `change_class: "documentation"`, so the test and lint gates were skipped as permitted.

## Phase 5

Phase 5 was **not implemented**. It is blocked transitively.

- It verifies equivalence of, and documents, a forwarding report, capability,
  cache evidence and consumer output that Phases 2-4 were meant to build. None
  exist, so there is nothing to compare, count work for, or document as current
  behavior (the `docs/` tree must not describe behavior that does not exist).
- The root cause is unchanged: the author's rulings (support contract,
  canonical representation, consumer exposure, C++ modules grammar blocker) are
  still pending. The spec is still `draft-spec` with `human_review: true`.
- No source, test, skill or plan task changed; no tasks were checked off. There
  is no requirement-to-test mapping because no behavior changed. Wave 9 is not
  applicable (no styled view exists).
- Gates: only docs under `tree-hugger/features/` changed.
- Scope command result: see below.
- Scope command result: `"packages": []`, `change_class: "documentation"`, so the test and lint gates were skipped as permitted.

## Phase 6

Phase 6 was **not completed**; the feature is not implemented. It is blocked
transitively by the same pending author rulings.

- Phase 6 validates and hands off the feature built by Phases 2-5. None of that
  exists: no report model, capability, language extraction, cache evidence,
  consumer change, equivalence tests or docs. `spec.md` is still `draft-spec`
  with `human_review: true`; the support contract, canonical representation,
  consumer exposure and C++ modules grammar blocker are unruled.
- Because nothing was implemented, "implementation complete, ready for review"
  cannot be reported and `implemented`/`completed_phase` were deliberately NOT
  set (doing so would be false). No plan tasks were checked off.
- No source, test or skill file changed. No requirement-to-test mapping exists.
- Gates: `just test` / `just lint` skipped; only docs under
  `tree-hugger/features/` changed. Scope command
  (`git status --porcelain ... | xargs python3 scripts/ci/affected_scope.py`)
  was run from the repository root; result `"packages": []`,
  `change_class: "documentation"`.
- OS evidence: none needed; no code changed.
- Placeholder-removal coordination: `reexported_symbols()` is already removed.

## Author rulings (2026-10-11)

The author accepted every recommendation in the decision package, which
closes the Phase 1 exit gate. The Phase 2–6 "blocked" entries above are void:
nothing was built, so execution restarts at Phase 2.

- `spec.md` is now `finalized-spec` (`human_review: false`). It records the
  rulings, and its **Finalized design** pins the following:
  - `TreeFile::reexports()` and `Capability::Reexports`, whose only
    prerequisite is `Declarations`. `Diagnostics` now depends on
    `Reexports`.
  - The `ReexportReport` types and their JSON, with every key always
    written.
  - `ImportRecord::reexported` and `SchemaVersion::V5_2`.
  - `EXTRACTOR_REVISION` `"52"`.
  - The per-language contract table, the CLI behavior (`--json-full` only)
    and the cache invariants.
- Intended behavior change: Rust `pub(self) use` is no longer a re-export,
  so it loses its `unused-import` exemption. This must be documented when
  Phase 4 lands.
- C++20 modules: there is no grammar change. A file containing module syntax
  reports `limited`/`grammar_gap`.
- In `plan.md`, "Freeze support contract" is checked off. The robustness
  matrix role columns are frozen to concrete JSON paths.
