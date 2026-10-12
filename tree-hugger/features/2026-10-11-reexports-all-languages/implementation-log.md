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
source_files_during_phase_3:
  - tree-hugger/lib/src/analysis/reexports/mod.rs
  - tree-hugger/lib/src/analysis/reexports/common.rs
  - tree-hugger/lib/src/analysis/reexports/rust.rs
  - tree-hugger/lib/src/analysis/reexports/ecma/mod.rs
  - tree-hugger/lib/src/analysis/reexports/ecma/bindings.rs
  - tree-hugger/lib/src/analysis/reexports/ecma/cjs.rs
  - tree-hugger/lib/src/analysis/reexports/python.rs
  - tree-hugger/lib/src/analysis/reexports/go.rs
  - tree-hugger/lib/src/analysis/reexports/perl.rs
  - tree-hugger/lib/src/analysis/reexports/lua.rs
  - tree-hugger/lib/src/analysis/reexports/swift.rs
  - tree-hugger/lib/src/analysis/reexports/scala.rs
  - tree-hugger/lib/src/analysis/reexports/cpp.rs
  - tree-hugger/lib/src/analysis/reexports/csharp.rs
  - tree-hugger/lib/src/file/mod.rs
  - tree-hugger/lib/tests/l1/main.rs
  - tree-hugger/lib/tests/l1/reexports_support.rs
  - tree-hugger/lib/tests/l1/symbols_reexports.rs
  - tree-hugger/lib/tests/l1/symbols_reexports_rust.rs
  - tree-hugger/lib/tests/l1/symbols_reexports_ecma.rs
  - tree-hugger/lib/tests/l1/symbols_reexports_python_perl.rs
  - tree-hugger/lib/tests/l1/symbols_reexports_go_lua.rs
  - tree-hugger/lib/tests/l1/symbols_reexports_swift_scala_csharp.rs
  - tree-hugger/lib/tests/l1/symbols_reexports_cpp.rs
  - tree-hugger/cli/tests/symbols/cache_upgrade.rs
  - tree-hugger/cli/tests/symbols/wiring.rs
  - tree-hugger/cli/tests/fixtures/golden/symbols.json
  - tree-hugger/cli/tests/fixtures/golden/symbols.jsonl
docs_updated_during_phase_3:
  - tree-hugger/features/2026-10-11-reexports-all-languages/implementation-log.md
  - tree-hugger/features/2026-10-11-reexports-all-languages/spec.md
  - tree-hugger/docs/symbol-records/file-index.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
  - .claude/skills/tree-hugger/analysis-pipeline.md
  - .claude/skills/tree-hugger/api-reference.md
  - .claude/skills/tree-hugger/testing.md
source_files_during_phase_4:
  - tree-hugger/cli/src/main.rs
  - tree-hugger/cli/tests/symbols/cache_robustness.rs
  - tree-hugger/lib/src/analysis/mod.rs
  - tree-hugger/lib/src/analysis/reexports/mod.rs
  - tree-hugger/lib/src/cache/mod.rs
  - tree-hugger/lib/src/cache/stored_evidence.rs
  - tree-hugger/lib/src/cache/stored_reexports.rs
  - tree-hugger/lib/src/cache/symbol_cache.rs
  - tree-hugger/lib/src/file/tree_file.rs
  - tree-hugger/lib/src/shared/symbol.rs
  - tree-hugger/lib/tests/l1/main.rs
  - tree-hugger/lib/tests/l1/reexports_consumers.rs
docs_updated_during_phase_4:
  - tree-hugger/features/2026-10-11-reexports-all-languages/implementation-log.md
  - tree-hugger/features/2026-10-11-reexports-all-languages/spec.md
  - tree-hugger/docs/cache-design/entry-format.md
  - tree-hugger/docs/cache-design/robustness.md
  - tree-hugger/docs/cli.md
  - tree-hugger/docs/library.md
  - tree-hugger/docs/library/diagnostics.md
  - tree-hugger/docs/release-notes.md
  - tree-hugger/docs/symbol-reports.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
  - .claude/skills/tree-hugger/analysis-pipeline.md
  - .claude/skills/tree-hugger/api-reference.md
  - .claude/skills/tree-hugger/diagnostics.md
  - .claude/skills/tree-hugger/testing.md
source_files_during_phase_5:
  - tree-hugger/lib/tests/l1/main.rs
  - tree-hugger/lib/tests/l1/reexports_equivalence.rs
  - tree-hugger/cli/tests/symbols/cache_lookup.rs
docs_updated_during_phase_5:
  - tree-hugger/features/2026-10-11-reexports-all-languages/implementation-log.md
  - tree-hugger/features/2026-10-11-reexports-all-languages/spec.md
  - tree-hugger/features/2026-10-11-reexports-all-languages/plan.md
  - tree-hugger/docs/overview.md
  - tree-hugger/docs/testing.md
  - tree-hugger/README.md
docs_created_during_phase_5:
  - tree-hugger/docs/reexports.md
skills_files_updated_during_phase_5:
  - .claude/skills/tree-hugger/SKILL.md
  - .claude/skills/tree-hugger/testing.md
source_files_during_phase_6: []
docs_updated_during_phase_6:
  - tree-hugger/features/2026-10-11-reexports-all-languages/implementation-log.md
  - tree-hugger/features/2026-10-11-reexports-all-languages/spec.md
  - tree-hugger/features/2026-10-11-reexports-all-languages/plan.md
docs_created_during_phase_6: []
skills_files_updated_during_phase_6: []
packages:
  - tree-hugger
  - tree-hugger-cli
source_code:
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
  - tree-hugger/lib/src/analysis/reexports/mod.rs
  - tree-hugger/lib/src/analysis/reexports/common.rs
  - tree-hugger/lib/src/analysis/reexports/rust.rs
  - tree-hugger/lib/src/analysis/reexports/ecma/mod.rs
  - tree-hugger/lib/src/analysis/reexports/ecma/bindings.rs
  - tree-hugger/lib/src/analysis/reexports/ecma/cjs.rs
  - tree-hugger/lib/src/analysis/reexports/python.rs
  - tree-hugger/lib/src/analysis/reexports/go.rs
  - tree-hugger/lib/src/analysis/reexports/perl.rs
  - tree-hugger/lib/src/analysis/reexports/lua.rs
  - tree-hugger/lib/src/analysis/reexports/swift.rs
  - tree-hugger/lib/src/analysis/reexports/scala.rs
  - tree-hugger/lib/src/analysis/reexports/cpp.rs
  - tree-hugger/lib/src/analysis/reexports/csharp.rs
  - tree-hugger/lib/tests/l1/reexports_support.rs
  - tree-hugger/lib/tests/l1/symbols_reexports_rust.rs
  - tree-hugger/lib/tests/l1/symbols_reexports_ecma.rs
  - tree-hugger/lib/tests/l1/symbols_reexports_python_perl.rs
  - tree-hugger/lib/tests/l1/symbols_reexports_go_lua.rs
  - tree-hugger/lib/tests/l1/symbols_reexports_swift_scala_csharp.rs
  - tree-hugger/lib/tests/l1/symbols_reexports_cpp.rs
  - tree-hugger/cli/src/main.rs
  - tree-hugger/lib/src/cache/stored_reexports.rs
  - tree-hugger/lib/src/shared/symbol.rs
  - tree-hugger/lib/tests/l1/reexports_consumers.rs
  - tree-hugger/lib/tests/l1/reexports_equivalence.rs
  - tree-hugger/cli/tests/symbols/cache_lookup.rs
documentation:
  - tree-hugger/features/2026-10-11-reexports-all-languages/spec.md
  - tree-hugger/features/2026-10-11-reexports-all-languages/audit/notes-module-syntax.md
  - tree-hugger/features/2026-10-11-reexports-all-languages/audit/notes-static-conventions.md
  - tree-hugger/features/2026-10-11-reexports-all-languages/audit/notes-interface-boundaries.md
  - tree-hugger/features/2026-10-11-reexports-all-languages/implementation-log.md
  - tree-hugger/docs/symbol-records.md
  - tree-hugger/docs/symbol-records/file-index.md
  - tree-hugger/docs/symbol-records/versions.md
  - tree-hugger/docs/cache-design/entry-format.md
  - .claude/skills/tree-hugger/analysis-pipeline.md
  - .claude/skills/tree-hugger/api-reference.md
  - .claude/skills/tree-hugger/testing.md
  - tree-hugger/docs/cache-design/robustness.md
  - tree-hugger/docs/cli.md
  - tree-hugger/docs/library.md
  - tree-hugger/docs/library/diagnostics.md
  - tree-hugger/docs/release-notes.md
  - tree-hugger/docs/symbol-reports.md
  - .claude/skills/tree-hugger/diagnostics.md
  - tree-hugger/features/2026-10-11-reexports-all-languages/plan.md
  - tree-hugger/docs/overview.md
  - tree-hugger/docs/testing.md
  - tree-hugger/README.md
  - tree-hugger/docs/reexports.md
  - .claude/skills/tree-hugger/SKILL.md
completed_phase: 6
implemented: true
---

# Re-exports across all supported languages: implementation plan

## Phase 1: Establish language evidence and settle the contract

### Necessary Rules

**Update 2026-10-11:** the author accepted every recommendation below; the
spec is `finalized-spec` and the exit gate is closed. The text that follows
is kept as written at planning time.

The specification is still `draft-spec`. Its three open questions require
the author's decisions; recommendations below are planning assumptions, not
approvals. Phase 1 can run now. Phases 2–6 are conditional on its exit gate.
If a decision changes the assumed design, revise those phases before coding.
`yolo: true` does not waive the specification's author decision requirement.

The work comprises a 16-language semantic and pinned-grammar audit, a shared
forwarding report, syntax and bounded same-file binding extraction, selective
analysis and cache integration, import/diagnostic/CLI consumers, and current
documentation. Success means every language has an explicit support contract;
all agreed forwarding forms preserve their source evidence; computed absence,
limited coverage, no applicable construct, and unrequested analysis remain
distinct. Equivalent direct, indexed, selective, cached, and CLI requests must
agree, with no ordinary cross-file reads or unrequested expensive analysis.

Author rulings needed after the audit:

1. **Supported conventions.** Recommended baseline: dedicated syntax plus
   explicitly bounded static conventions. Approve each language's supported
   patterns, exclusions, language versions/dialects, and reason codes. Decide
   whether Java dependency exposure, C/C++ includes, shell sourcing, PHP runtime
   aliasing, and C# assembly forwarding qualify under the definition. Do not
   preclassify them from their names. Decide which ambiguities limit a region
   and how those limits aggregate to file coverage; complete means complete
   under the documented contract, not a claim about arbitrary runtime behavior.
2. **Canonical representation.** Recommended baseline: a dedicated typed
   report in `FileSymbolIndex`, exposed through a `TreeFile` method and
   `SymbolQuery`, with a stored imported-binding marker. Approve exact Rust
   types, JSON shapes, method name, evidence/form/coverage enums, qualifier and
   conditional representation, source/scope spans, unknown-value encoding, and
   capability prerequisites. Decide schema version from the changed shape and
   meaning; do not assume that filling an existing flag avoids a bump.
3. **Consumer exposure.** Recommended minimum: expose forwarding separately,
   preserve declaration-only `exported_symbols()` and `--exported`, preserve
   flags in `hug imports`, and include requested forwarding evidence in
   full-index JSON. Approve the exact CLI capability requests, JSON envelope,
   ordinary import output, and coverage discoverability. No new command,
   combined export view, or synthesized alias declarations is assumed. If
   approved, add their concrete identity/rendering/test work before Phase 2.
4. **Removal coordination.** `2026-10-10-docs-audit-code-defects` requires
   removing the empty `reexported_symbols()` placeholder. (Resolved: the
   removal has landed; the replacement is `TreeFile::reexports()`.) Record who owns that removal and the final replacement API;
   neither preserve empty behavior nor restore an import-only API promising
   general forwarding. Reconcile docs when the prerequisite lands.
5. **Risk scope.** No performance spike: the specification already identifies
   structural work-counter tests as the proof for repeated-analysis risk.
   No grammar upgrade, compiler execution, runtime execution, dependency
   resolution, wildcard enumeration, or broader performance measurement is
   authorized. A grammar blocker goes to the author for a scope decision.
6. **Robustness scope.** Apply the input matrix below to persisted cache JSON,
   including nested evidence and capability metadata. Source-language syntax
   uses semantic/grammar fixture tests, not the configuration matrix. This
   feature adds no manifest, lockfile, YAML, or TOML reader. Reading pinned
   grammar versions for the audit does not change their readers.

### Wave 1: Concurrent language audit tasks

All waves use globally unique numbers. Tasks within a concurrent wave may use
subagents; waves otherwise run in order. A task finishes only when its evidence
and tests are reviewable. Delegate bounded file ownership, repeat that the
session tree is non-interactive, and never assign two workers the same shared
Rust file or document. Audit workers deliver separate notes; the integration
owner consolidates them into the specification after this wave.

- [x] **Audit module syntax**
  - Audit Rust, JavaScript, TypeScript, Swift, and Scala against authoritative
    language documentation and each grammar/version pinned by the manifest
    and lockfile. Cover every candidate listed in the specification, including
    local export clauses, restrictions, type-only selectors, conditional forms,
    Swift version stability, and Scala object-member forwarding.
  - Deliver per-construct rows with minimal positive and nearby negative sample
    files, semantic source URLs and dialect/version, exact nodes/fields,
    error/missing-node results, and expected names, paths, scope, qualifiers,
    form, and coverage limits. Treat dedicated syntax and conventions separately.
- [x] **Audit static conventions**
  - Produce the same evidence for Python, Go, Perl, and Lua: ordinary Python
    modules and export-list variants, aliases versus Go defined types,
    imported versus locally defined Perl exports, and Lua's returned interface.
  - Identify the smallest same-file binding and mutation facts each accepted
    pattern needs. Pair conditional, rebound, computed, shadowed, and ambiguous
    forms with explicit expected limits rather than guessed forwarding.
- [x] **Audit interface boundaries**
  - Produce the same evidence for Java, PHP, Bash, Zsh, C, C++, and C#.
    Parse Bash and Zsh independently; separate dependency/interface exposure,
    local aliases, namespace/class members, preprocessing, and runtime effects.
  - Establish whether each candidate qualifies, is outside the definition, or
    is a grammar/support gap. An empty fixture alone never proves absence of
    a language concept.

These are one-time semantic/grammar discovery spikes, run before the design
they inform. Keep their samples as later regression fixtures; do not repeat
the discovery work as a later spike. Use the repository's pinned Tree-sitter
runtime and an existing AST inspection route, or a small disposable in-process
probe if necessary. No runtime language interpreter or compiler is needed.

### Wave 2: Sequential design closure

- [x] **Consolidate audit evidence**
  - Merge Wave 1 notes into the specification's audit, as it requests. Account
    for all 16 languages and all listed candidates; retain grammar failures
    separately from semantic negatives. Record unanswered questions explicitly.
  - Check the current placeholder-removal state and affected consumer paths:
    `file/tree_file.rs`, `analysis/mod.rs`, `file/query.rs`,
    `analysis/coverage.rs`, `shared/schema_v2`, both cache layers,
    `cli/src/main.rs`, and `cli/src/prelude.rs`.
- [x] **Freeze support contract**
  - Obtain and record the author's rulings above. In a non-interactive run,
    prepare the concrete decision package and report the pending gate; do not
    treat silence as approval or mark the specification finalized/planned.
  - Pin API and JSON examples for renamed, wildcard, module/namespace,
    restricted, type-only, conditional, limited, no-applicable, and not-computed
    results. Preserve exported/original/local names separately and unknown
    target IDs, kinds, signatures, and definition locations without fabrication.
  - Define a syntax-only route for direct forms and the minimum binding route
    for indirect forms. Specify diagnostic prerequisites, capability completion,
    cache projection/widening, whole-file coverage under kind filters, and
    source-order/deduplication rules. Define stable import-site linkage for
    lint exemption rather than file-wide name matching.
  - Resolve exact stored fields and expand the input matrix below to one column
    per field, including every nested load-bearing leaf and new inventory or
    integrity metadata. Approve invariants that reject inconsistent states or
    lost evidence; define how these checks use already available source facts.

**Checkpoint:** authoritative and grammar evidence covers all 16 languages;
author decisions and concrete examples settle the API, schema, capabilities,
coverage, and CLI. Blocked grammar requirements remain explicit. Do not start
implementation with a language subset or unresolved public contract.

### Input robustness matrix

This is the proposed strict cache-reader policy for the recommended report.
Names denote contract roles until Wave 2 pins actual JSON paths. They are not
approved API spellings. Wave 2 must replace role columns with those paths and
split grouped roles into individual columns; no implementation is considered
matrix-complete until that expansion is done.

`R` means refuse the entire entry, recompute, rewrite, and return the same public
result as an uncached request. `S` means serve only when source/evidence and
capability invariants hold. `N/A` means the field is scalar and an element edit
is inapplicable; apply whole-field edits instead. All required keys are
written even for nullable or empty values in the stored representation.

| JSON shape | report object | entries array | coverage object | coverage state | reasons array | imported-binding marker | completed capabilities |
|---|---|---|---|---|---|---|---|
| Unedited producer control | S | S | S | S | S | S | S |
| Absent key | R | R | R | R | R | R | R |
| Explicit null | R | R | R | R | R | R | R |
| Wrong whole-field type | R | R | R | R | R | R | R |
| One wrong element/member | R | R | R | N/A | R | N/A | R |
| Every element/member wrong | R | R | R | N/A | R | N/A | R |
| Empty value | R for `{}` | S only for genuine absence | R for `{}` | R for `""` | S if state allows no reasons | `false` valid only for an unforwarded import | R for `[]` |
| Duplicate key | R | R | R | R | R | R | R |
| Trailing/invalid document | R | R | R | R | R | R | R |

| JSON shape | form/evidence enum | exported/original/local name, source/member path (each nullable role) | scope identity (nullable) | entry/scope/reason spans (each object) | qualifier/condition collections (each array) | reason code | schema/fingerprint/inventory (each existing or added field) |
|---|---|---|---|---|---|---|---|
| Unedited producer control | S | S | S | S | S | S | S |
| Absent key | R | R | R | R | R | R | R |
| Explicit null | R | S only where the form defines unknown/not applicable | S only for no enclosing scope | R unless a particular span is explicitly nullable | R | R | R |
| Wrong whole-field type | R | R | R | R | R | R | R |
| One wrong element/member | N/A | N/A | N/A | R | R | N/A | R for objects; N/A for scalars |
| Every element/member wrong | N/A | N/A | N/A | R | R | N/A | R for objects; N/A for scalars |
| Empty value | R for `""` | R for `""` | R for `""` | R for `{}` | S only for genuine absence | R for `""` | R; preserve any documented existing exception |
| Duplicate key | R | R | R | R | R | R | R |
| Trailing/invalid document | R | R | R | R | R | R | R |

For every numeric span leaf, absence/null/wrong type/duplicate/trailing content
means R; zero is valid only where byte/column conventions and the source permit
it, while lines remain 1-based. For any boolean qualifier leaf, absence/null/
wrong type/duplicate/trailing content means R; `false` is a value, not absence.
Freeze the corresponding individual columns in Wave 2. Do not add configurable
inputs merely to satisfy this matrix.

#### Frozen field columns (2026-10-11)

The author accepted the recommended design (see the spec's **Finalized
design**). The role columns above now map to the JSON paths below. Each row
is one matrix column. The shape rows (absent, null, wrong type, element edits,
empty, duplicate key, trailing document) apply to every path, with the
null/empty exceptions given here. Every key is always written, so an absent
key is `R` for every path.

| JSON path (under the file index) | Null | Empty / zero value | Element edits |
|---|---|---|---|
| `reexports` (object; present iff `"reexports"` in `completed_capabilities`) | R | R for `{}` | — |
| `reexports.coverage` | R | R for `{}` | — |
| `reexports.coverage.state` | R | R for `""`; unknown variant R | N/A |
| `reexports.coverage.limits` | R | S only when state is not `limited` | R |
| `reexports.coverage.limits[].reason` | R | R for `""`; unknown variant R | N/A |
| `reexports.coverage.limits[].span` (+ 6 numeric leaves) | R | numeric rule below | R |
| `reexports.entries` | R | S only when coverage permits (never by deleting a positive report's entries) | R |
| `reexports.entries[].form` | R | R | N/A |
| `reexports.entries[].evidence` | R | R | N/A |
| `reexports.entries[].exported_name` | S only for `wildcard`/`module` | R for `""` | N/A |
| `reexports.entries[].original_name` | S unless `named`/`value_copy` | R for `""` | N/A |
| `reexports.entries[].local_name` | S | R for `""` | N/A |
| `reexports.entries[].source` | S | R for `""` | N/A |
| `reexports.entries[].excluded_names` | R | S | R (member `""` R) |
| `reexports.entries[].scope` | S (file level) | R for `""` | N/A |
| `reexports.entries[].restriction` | S (unrestricted) | R for `""` | N/A |
| `reexports.entries[].type_only` | R | `false` is a value | N/A |
| `reexports.entries[].conditional` | R | `false` is a value | N/A |
| `reexports.entries[].caveats` | R | S | R (unknown variant R) |
| `reexports.entries[].import_span` | S (forwards no import) | R for `{}`; must equal an `imports[].span` | R |
| `reexports.entries[].span`, `.statement_span` (+ leaves) | R | numeric rule below | R |
| `imports[].reexported` | R | `false` valid; `true` R unless a `syntax`/`declared` entry links it and the capability completed | N/A |
| `completed_capabilities` (`"reexports"` member) | R | R for `[]` | R (unknown member R) |
| `schema_version` (`5.2` only) and extractor revision `52` | R | R | R for objects |

Spans must also lie within the file's byte length. The well-typed
inconsistency cases are the spec's **Cache invariants**.

The JSON matrix test extends the existing CLI `cache_robustness` harness. Start
from a real `hug`-produced entry and a control proving each field affects the
public report, imported flag, or reuse decision. Edit one cell at a time,
including duplicate keys in raw JSON so a map serializer cannot erase them.
Include producer controls for genuinely empty, limited, and uncomputed states;
deleting a positive report's entries must not be accepted as a complete
negative. Assert public output plus cache hit/miss/recomputation evidence,
not just deserialization success. Preserve existing matrix rows and exceptions.

## Phase 2: Build the shared report and analysis seam

### Wave 3: Sequential foundation tasks

- [x] **Define report model**
  - Implement the approved types/re-exports in `shared/schema_v2` and the
    imported-binding stored marker. Update required constructors, public
    serialization/schema tests, and analyzer fingerprints together.
  - Add reusable test expectations for named/wildcard/module forms and coverage
    states. Keep local declaration exports and synthetic prelude aliases under
    their existing identities and flag semantics.
- [x] **Wire analysis capability**
  - Integrate the approved capability and internal extraction seam in
    `file/query.rs`, `analysis/mod.rs`, and `analysis/coverage.rs`. Route the
    direct API through the same planner; never call public extraction methods
    recursively from the planner.
  - Reuse definition/import captures, file facts, name-comparison rules, and
    indexed parent navigation. Separate minimum same-file binding facts from
    full reference captures, semantic work, and diagnostics. Make dependencies
    explicit, including diagnostics that need proven forwarding.
  - Stage binding before declaration filtering. Preserve whole-file coverage
    in projection and distinguish missing capability evidence from empty data.
  - Bump `EXTRACTOR_REVISION` and add the matching upgrade row for this extraction
    change. Every later extraction-changing revision must also get its row.

**Checkpoint:** model and planner compile; focused tests prove uncomputed versus
computed-empty states and capability selection. The extraction interface is
stable enough for language workers. Any temporary unimplemented dispatch is an
explicit limited/internal state and cannot be released as complete coverage.

## Phase 3: Implement the approved language contracts

### Wave 4: Concurrent language implementation tasks

Depends on Phase 2. Workers own distinct language modules and fixtures; one
integration owner owns shared dispatch, planner, constructors, and revision
bumps. Grouping does not presume which candidates the audit accepts.

- [x] **Implement explicit forwarding**
  - Implement approved Rust, Swift, and Scala contracts, with language-local
    positive/negative fixtures. Cover Rust grouping, `self`, restrictions,
    enclosing scopes, and external-crate forms where approved; Swift import
    qualifiers; Scala selectors, exclusions/givens, and object-member paths.
- [x] **Implement ECMA forwarding**
  - Implement JavaScript/TypeScript direct and indirect export forms, default
    exclusions, namespace/module distinction, aliases and type-only forms.
    Cover approved CommonJS forms with proof about shadowed globals and
    reassignment. Preserve existing local-export reference behavior.
- [x] **Implement remaining contracts**
  - Implement approved patterns and coverage reasons for Python, Go, Perl,
    Lua, Java, PHP, Bash, Zsh, C, C++, and C#. Use language-local modules and
    table-driven expectations to keep this bounded group reviewable.
  - A no-applicable result requires the settled audit; unsupported static or
    dynamic forms remain limited. No filename heuristic, include-only public
    inference, or guessed import binding may produce forwarding evidence.

Each task includes its focused L1 tests, not a later test-writing handoff.
Check exported/original/local names, paths, scopes, byte/1-based spans,
qualifiers, conditional evidence, and categorical evidence kind. Preserve
separate forwarding sites; deduplicate duplicate captures of one site.

### Wave 5: Sequential language integration

- [x] **Integrate language coverage**
  - Join implementations through the shared seam and add a 16-language coverage
    guard with dialect-specific rows where needed. Register new tests in the
    consolidated L1 targets and spell fixture reads in CI-resolvable forms.
  - Test shadowing, rebinding, mutation, conditional syntax, and computed lists
    against public reports; verify supported imports only receive the marker.
  - Add parse-recovery cases: unrelated errors preserve valid forwards with
    accurate coverage, while damaged forwarding syntax cannot yield a definite
    complete negative. Positive fixtures must parse without errors/missing nodes.

**Checkpoint:** every agreed construct has a passing public-result expectation
and nearby negative; every language has a support/coverage row. Extraction
uses only the analyzed file and does not fabricate target definitions.

## Phase 4: Preserve evidence through caches and consumers

### Wave 6: Concurrent integration tasks

- [x] **Persist forwarding evidence**
  - Update `cache/symbol_cache.rs`, strict stored evidence adapters, in-memory
    snapshots/conversions in `cache/mod.rs`, and coverage sufficiency/projection
    under one owner. Store the approved schema, computed state, entries,
    coverage, import linkage, and required integrity metadata.
  - Enforce capability/report consistency, valid source geometry, enum/form
    constraints, nullable distinctions, and inventory consistency. Reject old,
    missing, malformed, or inconsistent evidence and recompute; no defaults
    may turn damaged evidence into negative coverage.
  - Add final schema-bump and stale-revision rows without rewriting historical
    key formulas. Keep storage and identities independent of the scan root.
- [x] **Preserve consumer semantics**
  - Update `analysis::import_record`, direct imported-symbol output, and
    `unused-import` exemptions to use proven per-import forwarding evidence.
    Arrange diagnostics after necessary facts, without invoking public methods
    again. Retain Rust's restricted-use exemption and local ECMA export reads.
  - Update `cli/src/main.rs` import conversion and approved full-index envelopes
    and capability requests. Preserve parse errors and limited/not-computed
    coverage in output; remove the stale comment claiming the marker is absent.
  - Preserve Rust `--prelude` public-only selection, target resolution, and
    synthetic identities. Change its detection source only if needed; no
    resolver expansion to other languages. Add L1 scratch-repo regressions.

These workers must agree on the frozen model before starting. The cache owner
also owns `analysis/coverage.rs`; the consumer owner owns diagnostics/planner
ordering edits. Shared schema changes return to the integration owner.

### Wave 7: Sequential cache validation

- [x] **Exercise robustness matrix**
  - Implement the completed JSON matrix through real producer fixtures, controls,
    and one edit per cell. Assert output equality with uncached analysis and
    observable refusal/recomputation; include nested names, reason and qualifier
    members, spans, capability metadata, and import marker/inventory coupling.
  - Add well-typed inconsistency cases: complete state with lost entries,
    limited state with deleted reasons, evidence without capability, wrong
    import site, and source-inconsistent spans. Use the approved invariants.
  - Inspect changed readers for `#[serde(default)]`, nullable `Option<T>`
    conflation, filtering invalid members, `unwrap_or_default()`, and `.ok()`.
    Resolve each occurrence against its field's matrix rather than banning
    these constructs in unrelated code. A finding records all cells in that
    field column and all fields in that shape row.

**Checkpoint:** fresh and cache-hit CLI/API results agree; malformed/stale entries
are recomputed, and import conversion retains its evidence. Existing diagnostics
and Rust prelude regressions pass.

## Phase 5: Prove equivalence and document the current behavior

### Wave 8: Concurrent verification and documentation tasks

- [x] **Verify analysis equivalence**
  - Compare direct API, complete index, equivalent selective queries and CLI
    full-index/import output. Cover fresh, hit, wider-to-narrower projection,
    narrower-to-wider computation, kind/predicate filters, and different scan
    roots. Ensure narrowing preserves file coverage and missing evidence causes
    computation. Declaration IDs/counts and original flags remain as approved.
  - Extend existing work-counter tests: one shared definitions/import extraction,
    bounded same-file binding, no repeated public extraction, and no unrequested
    reference captures/resolutions, semantic, or diagnostic passes. Include
    cheap direct, indirect, diagnostics, complete-index, and cache-widening requests.
  - Prove ordinary extraction succeeds with dependency files absent and cannot
    read forwarding targets; retain the deliberate Rust prelude exception only
    in its existing resolver tests.
- [x] **Document supported behavior**
  - Write `docs/reexports.md` for newcomers: how to request the report, concise
    examples per supported rule, all 16 language contracts, evidence/form
    distinctions, qualifiers, scope/reachability limits, and coverage reasons.
    Explain the request → extraction/binding → coverage → cache/output flow
    with a Mermaid diagram. No dated feature/fix names or links appear in docs.
  - Update library/API, symbol-record/index, CLI, diagnostics, testing, and cache
    format/robustness pages; reflect removal/replacement of the placeholder.
    Update README overview links only where needed.
  - Update the tree-hugger skill's API, extraction, analysis, cache, testing,
    diagnostics, and CLI pages where changed. Review comments on every changed
    symbol, correcting drift and distinguishing normal versus synthetic records.
    Update dependency docs only if approved work actually changes dependencies.

### Wave 9: Conditional terminal validation

- [x] **Validate styled output** (not applicable: no styled view added; `--json-full` only, L1 suffices)
  - Run only if the approved CLI contract adds or changes a styled view. Consult
    `biscuit-terminal` renderable components and the real-terminal test-harness
    skills. Add the appropriate shared scenarios to existing L2 entry points,
    covering all supported backends, report scoping, styling, and wrapping.
  - Use `just test-l2` with the repository's focus-preserving harness; never
    bring terminal or browser windows into focus. For JSON/flag preservation
    alone, mark this task not applicable with the concrete reason; L1 suffices.

**Checkpoint:** public equivalence, cache transitions, selective work, and
regressions pass; docs and skills describe the final contract and honest limits.
No benchmark or repeated audit spike is introduced here.

## Phase 6: Complete validation and hand off for review

### Wave 10: Sequential completion tasks

- [ ] **Run package checks**
  - From `tree-hugger/`, run `just test` and `just lint`; run applicable
    `just test-l2` when Wave 9 requires it. Use nextest and test-toolkit
    conventions, never the consolidated `cargo test` runner. Do not run
    `cargo fmt`; fix changed-line style directly if lint requires it.
  - Preserve macOS, Linux, native Windows, and WSL2 compatibility through
    existing workflows and qualifying evidence. Record which environments
    actually supplied evidence; load the `os` skill before path/cfg changes,
    an OS-specific failure, or a claim that an OS cannot be tested. Add no
    new CI environments, gates, or workspace-wide run for this feature.
  - Resolve failures and rerun affected checks. Inspect the final diff against
    the approved support contract, exact matrix, examples, and all specification
    acceptance items; record any departure in the implementation log and
    correct current docs without rewriting the settled specification snapshot.
- [ ] **Prepare review handoff**
  - Check off tasks only with observable completion evidence. Record author
    rulings, grammar limitations, tests/commands/results, any pending external
    evidence, and the placeholder-removal coordination outcome.
  - Report “implementation complete, ready for review” only when every approved
    requirement is met; otherwise name the specific unmet gate. Leave the
    feature active. Do not move it to `_completed`, run `just complete`, or
    commit without a separate explicit instruction.

**Final checkpoint:** the implementation answers the same forwarding question
through every approved surface for all 16 language contracts, preserves truthful
coverage and diagnostics, refuses damaged cache evidence, and has scoped passing
validation plus current developer documentation ready for author review.
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

The earlier "not implemented" stop below is void (the author's rulings landed and Phase 2 was built); Phase 3 was implemented, see "Phase 3 (implemented)" after it.

Phase 3 was **not implemented** at that time. It was blocked twice over.

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

### Phase 3 (implemented)

Implemented the eleven language extractors behind the Phase 2 seam. The
orchestrator split `analysis/reexports.rs` into `reexports/mod.rs` (dispatch),
`common.rs` (`entry`, `limit`, `finish`, `damaged_nodes`) and one module per
language, then ran six foreground-equivalent workers over disjoint files
(Rust; JS+TS; Swift+Scala+C#; Python+Perl; Go+Lua; C++), each owning its
module and its `tests/l1/symbols_reexports_<group>.rs`. The orchestrator then
integrated, replaced the placeholder guard and ran the gates.

- `EXTRACTOR_REVISION` moved `"52"` to `"53"` (the spec pinned 52). Revision 52
  entries hold the placeholder `grammar_gap` report, so they must not be
  served. Row: `cache_upgrade::entry_from_extractor_revision_fifty_two_is_not_served_after_the_language_extractors_land`.
  Goldens `symbols.json`/`symbols.jsonl` carry `rev.53`.
- `symbols_reexports::every_language_reports_not_applicable_or_its_forwarding_entries`
  is the 16-language guard (5 `not_applicable`, 11 one forwarding snippet each,
  all `complete` with entries; the two tables must partition all languages).
  `a_language_without_forwarding_syntax_is_a_complete_negative_not_a_placeholder`
  covers the negative.
- `wiring::json_full_carries_the_reexport_report_and_json_does_not` now asserts a real
  Rust `pub use` entry and the import marker on only the forwarded import.
- Each language file has per-contract-row tests asserting whole entries, spans,
  import linkage, markers, a nearby negative, parse recovery, and a corpus test
  over its audit samples (read with `include_str!` inside each test).

Rulings made by workers where the contract was silent (all in the code and its
tests; review in Phase 6): Rust `cfg_attr` counts as `conditional`; Rust `pub use x as _`
reports `exported_name` `_`; C++ `using enum` is a `wildcard`; C++ `#if` does not
set `conditional`; Python with any `__all__` reports only listed or redundant-alias
imports; Perl `use Mod;` with unlisted names gives `cross_file_export_list`; Lua
merge/copy/`pcall` forms give `dynamic_forwarding`; Go `amb-dot-import-alias`
stays `complete` with no entry; shadowed ES `import` uses `shadowed_global`.

Known gaps reported by workers: no Perl `ImportRecord`s exist, so Perl
`import_span` is always `None`; `.cppm` is not a recognized extension; Swift
`#if` is tracked among top-level siblings only; TS `export import A = B.C`
marks only `B`'s import; Go matches an unaliased import by its raw last path
segment (`yaml.v3`).

Phase 4 still owns: the full cache invariants and JSON matrix, import-site
linkage for `unused-import`, and `imported_symbols()`/CLI `is_reexport`.
`docs/reexports.md` is Phase 5.

### Gates

- Scope command: the tree changes `tree-hugger` source and tests, so the gates ran.
- `just test` in `tree-hugger/`: 2360 passed, 0 failed.
- `just lint`: clean for tree-hugger and tree-hugger-cli.
- OS: no `cfg(windows)` or path-comparison code; tests write temp files with
  literal names only. No cross-check run (macOS only).

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

### Phase 4 (implemented)

The earlier "not implemented" notes above are void; Phase 4 was built on the Phase 2 seam and the Phase 3 extractors.

**Consumers.** `analysis::reexports::flag_forwarded_imports` is now the single
place `ImportSymbol::is_reexport` is decided: it runs `extract`, marks the import
records by exact span (`mark_forwarded_imports`) and copies the result back to the
imports. The planner calls it right after the imports are read and before
diagnostics (`Diagnostics` requires `Reexports`), so the `unused-import`
exemption reads proven forwarding. `TreeFile::imported_symbols()`,
`lint_diagnostics()`, `try_lint_diagnostics()` and `try_diagnostics()` call it
through `forwarded_imports_from_nodes`. The Rust-only `import_is_reexport`
visibility-modifier check is deleted, so Rust `pub(self) use` loses its
exemption (documented in `release-notes.md` and `library/diagnostics.md`).
`analysis::import_record` now takes `&ImportSymbol` and copies `is_reexport`
into `ImportRecord::reexported`. The CLI's `import_symbols_from_index` copies
the stored marker (the stale comment is gone), and `hug imports` requests
`Capability::Reexports` so the marker is computed. Rust `--prelude` is
untouched (its detection already used its own resolver path).

**Cache.** `cache/stored_reexports.rs::validate_reexports` runs in
`StoredEntry::validate` after the capability/marker coupling Phase 2 added:
state against language (`is_not_applicable`, new beside `extract`), limits
against state, limit and entry spans against the source, limit order, entry
source order, statement-contains-specifier, name nullability per form,
empty-string names, excluded names only on wildcards, strictly ascending
caveats, `import_span` equal to an import's span, and marker agreement with the
entries. Deserialization (Phase 2) already refuses absent, null, mistyped,
repeated and unknown members. No `#[serde(default)]`, `unwrap_or_default` or
`.ok()` exists in the changed readers. The import inventory digest already
covers each `reexported` marker, so a flipped marker is refused by the
inventory check first.

**Requirement to test.**

| Behavior | Test |
|---|---|
| Every report field x every shape (absent, null, wrong type, one/every wrong element, empty, duplicate key, trailing), from a real `hug` entry | `cache_robustness::reexport_report_fields_are_served_only_when_consistent` (complete TS report), `a_limited_reexport_report_is_served_only_when_consistent` (limited JS report, limit reason and span leaves) |
| Well-typed inconsistencies: report without capability, deleted positive entries, wrong import site, spans outside the file, order, form vs names, excluded names, caveats, limits vs state, state vs language | `well_typed_reexport_inconsistencies_in_a_complete_report_are_refused`, `..._in_a_limited_report_are_refused`, `a_not_applicable_language_is_complete_only_as_not_applicable` |
| Narrow entry widened, not read as empty; then hit | `an_entry_without_reexports_is_widened_not_read_as_empty` |
| Nullable names served as null and visible in output | `a_nullable_reexport_name_is_served_as_null` |
| Raw duplicate and trailing members | `a_duplicated_or_trailing_reexport_member_is_refused_and_recomputed` |
| `is_reexport`, stored marker, `unused-import` agree; `pub(self)` loses the exemption; Python exposed import is not exempt; TS local export exempts only its import; uncomputed marks nothing | `tests/l1/reexports_consumers.rs` (six tests) |
| CLI conversion copies the marker | `hug` unit test `imports_from_an_index_carry_the_stored_reexport_marker` |

Every refused cell asserts a miss and output equal to an uncached run; inconsistency
cases also assert the refusal reason printed by `--verbose`.

**Decisions.** The matrix was built on the existing `cache_robustness` harness
(`Column`, `check_column`) and not a new one. Null `source` and null `local_name`
on a populated entry are served (the matrix says `S`); the served value is asserted
in `a_nullable_reexport_name_is_served_as_null`. Marker coupling cannot be reached
for flipped markers (inventory digest first) but is reached by deleting entries.
A deleted non-linked entry (for example a bare `export * from`) is undetectable
without a parse; the limit is the one the evidence checks already document.

**Departures from the plan.** None in behavior. `mark_forwarded_imports` stays (now
called by `flag_forwarded_imports`). The CLI `--json-full` capability request and
the golden files were Phase 3 work and needed no change.

**Gates.**

- Scope command not needed: source and tests changed, so both gates ran.
- `just test` in `tree-hugger/`: 2375 passed, 0 failed.
- `just lint`: clean for both packages.
- Several `cache_robustness` tests exceed 5 s under full-suite load (the existing
  matrix tests do too; the new complete-report matrix takes about 3 s alone). They
  are the repo's established thread-per-column pattern; not changed.
- OS: no `cfg(windows)` or path code added; fixtures are literal names in scratch
  repos. macOS only; no `cross-check` run.

## Phase 5 (superseded: void, written before the author rulings)

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

## Phase 6 (superseded: void, written before the author rulings)

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

## Phase 5

Phase 5 is implemented. (The earlier "Phase 5 (superseded)" section is void.)

**Requirement to test.**

| Behavior | Test |
|---|---|
| Direct API = complete index = selective query = kind-filtered query for all 16 fixtures; asking for the report leaves declarations and ids unchanged; unrequested report is `None`; no entries means no markers | `reexports_equivalence::every_surface_agrees_for_every_language` |
| Cache cold, hit, wide (complete index) then narrowed hit all equal the uncached report and markers, all 16 fixtures | `reexports_equivalence::the_cache_serves_the_uncached_report_for_every_language` |
| Narrow entry widened by exactly one extraction, no reference/occurrence/diagnostic work; a hit does no extraction, parse or definition capture | `reexports_equivalence::a_narrow_entry_is_widened_by_one_extraction_and_a_hit_extracts_nothing` |
| Extraction succeeds with forwarding targets absent, report identical when targets exist with different contents, one parser construction; report adds no second definitions query; diagnostics and complete index extract once | `reexports_equivalence::extraction_reads_no_forwarding_target_and_shares_one_definitions_pass` |
| Dependency-free snippet reports; planner report equals direct | `reexports_equivalence::a_dependency_free_snippet_reports_without_any_other_file` |
| CLI `--json-full` fresh (`--no-cache`), cold, hit (parses 0 files) and another scan root give equal `reexports` and import markers | `cache_lookup::json_full_reexports_agree_fresh_hit_and_from_another_scan_root` |

Already covered earlier and left as is: cache transitions and the robustness
matrix (Phase 4), `extraction_runs_once_and_only_when_required`, narrowing and
widening in `symbols_reexports`, and the Rust prelude resolver tests (the only
place a forwarding target is read).

**Decisions.** A library test of "another scan root" was dropped: the library
cache key includes the file's path, so a copy at another path is a miss by
design. The scan-root case (same file, different working directory) is the CLI
test. The input robustness matrix does not apply: no parser or reader changed.
Wave 9 is not applicable: no styled view was added or changed, and L1 suffices.

**Docs.** New `docs/reexports.md` (how to request the report, Mermaid flow,
field meanings, evidence, coverage and limit reasons, all 16 language
contracts, scope limits, caching). Linked from `README.md` and `docs/overview.md`;
`docs/testing.md` gained a re-export testing bullet. The library, symbol-record,
CLI, diagnostics, cache and release-notes pages were already current from
Phase 4 and carry no dated feature names; the placeholder
`reexported_symbols()` is gone from them. Skill: `SKILL.md` links the new page;
`testing.md` records the equivalence tests.

**Gates.** Source/test files changed, so both gates ran from `tree-hugger/`:
`just test` 2381 passed, 0 failed (before docs edits; docs-only after);
`just lint` clean for both packages. macOS only; no OS-specific code added, so
no `cross-check`. No pre-existing failures; slow cache_robustness tests noted in
Phase 4 still apply.

## Phase 6

Phase 6 is implemented: validation and review handoff. No source, test or skill
file changed in this phase; only the plan, log and spec frontmatter.

**Requirement to test.** No behavior changed, so no new tests. The acceptance
items map to tests already recorded: per-construct fixtures and parse recovery
(Phase 3, `symbols_reexports_*`), cache refusal and the robustness matrix
(Phase 4, `cache_robustness`), consumers and the Rust exemption
(`reexports_consumers`), equivalence and bounded work (Phase 5,
`reexports_equivalence`, `cache_lookup`).

**Scope command** (from the repository root, after all edits were saved):
`"packages": ["tree-hugger", "tree-hugger-cli", "test-toolkit"]`,
`change_class: "package"`. The working tree still holds the uncommitted Phase 5
tests, so both gates were required and run.

**Gates** (from `tree-hugger/`, on macOS):

- `just test`: 2381 run, 2381 passed, 0 failed (17 slow, none over budget in
  isolation; `refs::type_only_imports_preserve_id_selected_callers_callees_and_text`
  and the `cache_robustness` tests are the known slow ones under full load).
- `just lint`: clean for `tree-hugger` and `tree-hugger-cli`.
- `just test-l2`: not run; Wave 9 is not applicable (no styled view).
- OS evidence: macOS only. No `cfg(windows)` or path-comparison code was added
  and fixtures use temp dirs, so no `cross-check` was run; Linux, native Windows
  and WSL2 evidence comes from the existing CI workflows.

**Final diff against the spec.** All 16 language contracts, the `Reexports`
capability, schema 5.2 with extractor revision 52, strict cache validation,
import markers, `--json-full` exposure and the docs match the finalized design.
No departure from the spec was found.

**Placeholder removal.** `reexported_symbols()` was already removed; the
replacement is `TreeFile::reexports()`.

**Known grammar limits** (documented in `docs/reexports.md`): Perl has no
`ImportRecord`s; Go matches the raw last path segment; TS `export import A =
B.C`; Swift `#if` among top-level siblings; `.cppm` is unrecognized and C++20
module files report `limited`/`grammar_gap`.

**Status:** implementation complete, ready for review. The feature stays in its
active directory.
