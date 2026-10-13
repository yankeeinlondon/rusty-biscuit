---
kind: plan
name: reexports-all-languages
total_phases: 6
created: 2026-10-11
phase: 6
agent: codex/gpt-6.1-sol
yolo: true
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

- [x] **Run package checks**
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
- [x] **Prepare review handoff**
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
