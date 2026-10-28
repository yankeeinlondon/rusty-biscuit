---
spec: "/Volumes/coding/wt/rusty-biscuit/feat-schema-narrowing-and-extraction/darkmatter/features/2026-10-27-schema-narrowing-metadata/spec.md"
plan: "darkmatter/features/2026-10-27-schema-narrowing-metadata/plan.md"
implemented_by: "claude/sonnet"
started_phase: 1
source_files_during_phase_1:
  - darkmatter/lib/src/markdown/schemas/simplified/types.rs
  - darkmatter/lib/src/markdown/schemas/simplified/grammar.rs
  - darkmatter/lib/src/markdown/schemas/simplified/serialize.rs
  - darkmatter/lib/src/markdown/schemas/simplified/signature.rs
  - darkmatter/lib/src/markdown/schemas/simplified/subtype.rs
  - darkmatter/lib/src/markdown/schemas/simplified/mod.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - darkmatter/lib/src/markdown/compose/expression/call_binding/registration.rs
  - darkmatter/lib/src/markdown/compose/expression/call_binding/mod.rs
  - darkmatter/lib/src/markdown/compose/expression/catalog/mod.rs
  - darkmatter/lib/src/markdown/compose/expression/catalog/tests.rs
  - darkmatter/lib/src/markdown/compose/expression/functions/mod.rs
  - darkmatter/lib/src/markdown/compose/expression/functions/paths.rs
  - darkmatter/lib/src/markdown/compose/expression/functions/predicates.rs
  - darkmatter/lib/src/markdown/compose/expression/host/artifact.rs
  - darkmatter/lib/src/markdown/compose/expression/host/mod.rs
  - darkmatter/schemas/partials/functions.yaml
  - darkmatter/lib/tests/fixtures/host_functions/project.yaml
  - darkmatter/lib/tests/fixtures/host_functions/project.facts.json
  - darkmatter/lib/tests/l1/flow_sensitive_narrowing.rs
  - darkmatter/lib/tests/l1/host_functions.rs
  - darkmatter/lib/tests/l1/scope_site_conformance.rs
  - darkmatter/lib/tests/l1/expression_type_scope.rs
docs_updated_during_phase_2:
  - darkmatter/docs/topics/host-functions.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
packages:
  - darkmatter
decisions_for_review:
  - |-
    Narrowing target of a grouped-union array: the plan says a target must be
    one non-reference arm. I reject only named references (`Name`,
    `Name@file`) and multi-arm unions; a grouped union followed by `[]`
    (`(a | b)[]`) parses as one array atom and is accepted. Rejecting it too
    would be a one-line change in `narrows_target_problem` (types.rs).
  - |-
    Phase 3 work pulled into Phase 2: the plan sequenced the artifact reader's
    compare-instead-of-supply change and the fixture re-bless in Phase 3 and
    expected the freshness test to stay red meanwhile. The reader called the
    removed setter, so it had to change to compile, and a red suite blocks the
    Phase 2 gate. I implemented the whole-fact comparison (including absence in
    both directions) and blessed `project.facts.json` now. Alternative: keep
    the reader minimal and leave the suite red until Phase 3. Phase 3 then only
    needs the DMLS rows; nothing else needs redoing if a reviewer disagrees
    beyond moving those edits.
  - |-
    `file_exists` now narrows an `unknown` subject to `file` (spec Ownership
    step 3), which changed two existing assertions that expected `string`
    (`scope_site_conformance` shell-candidate shape, `expression_type_scope`).
    I updated them to `file`. A `true-iff` clause without a target (presence
    only) is accepted by the compiler as before; the old setter accepted it
    too. Rejecting it would be a one-line check in `compile_declared_fact`.
source_files_during_phase_3:
  - darkmatter/dmls/tests/l1/host_function_diagnostics.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
message_to_agent: |-
  Phase 3 is complete. The artifact reader compares (never supplies) the fact,
  the reader matrix has the fact-comparison cells, the fixture is blessed, and
  DMLS has a row for a mismatched fact artifact. The only Phase 3 source edit
  was the DMLS test. `just lint` still fails only on the pre-existing
  darkmatter-cli `render.rs:22` too_many_arguments. Phase 4 can start directly.
---

# Implementation Log for 2026-10-27-schema-narrowing-metadata (5 phases)

## Phase 1

- Added `NarrowsClause`/`NarrowsKind` and `FunctionType.narrows` (types.rs);
  `parse_function` parses `narrows(name[: type]; kind)` in any clause order,
  with post-loop checks (duplicate, parameter declared/not rest, boolean-only
  returns, single non-reference target, kind keyword). EBNF and the
  unknown-constraint message updated.
- `write_function` serializes the clause right after `returns(...)`;
  `canonical_atoms` clears it; subtype deliberately ignores it (comment added).
- Tests (in-process via `parse_type_definition`/`parse_type_expr`):
  `grammar::narrows_clause_tests` (control, absence, all kinds, order,
  duplicate, shape errors, parameter, returns matrix),
  `serialize::narrows_serialization_tests` (repeated round trip, placement),
  `signature::narrows_contract_tests` (duplicates despite differing clauses),
  `subtype::narrows_subtype_tests`.
- Tier: all L1 unit tests in the lib crate; no spawn, no repository capture.
- Gates: `just test` 12921 passed. `just lint`: darkmatter-cli fails with a
  pre-existing `clippy::too_many_arguments` at `cli/src/commands/render.rs:22`
  (file untouched this phase); `cargo clippy -p darkmatter -p dmls
  --all-targets -D warnings` is clean. Serde-smell grep not applicable (no
  serde on the new fields).
- No skill change needed this phase (grammar surfaces documented in Phase 5).

## Phase 2

- `compile_declared_fact(function, &[FunctionType])` (registration.rs) is the
  one fact compiler: `None` when no overload declares a clause; every overload
  must declare it (no silent skip of an overload that omits it), name a
  declared non-rest parameter at the same index, return exactly `boolean`, and
  agree on parameter, kind, and canonical target (descriptions stripped,
  constraints sorted); `true-iff` needs `is_exact_fact_type`; presence-only
  compiles to `FactType::NonNull`. It reuses Phase 1's
  `returns_exact_boolean`/`narrows_target_problem`.
- `build_with` calls it for both sources; `RegistrationBuilder::narrows`, the
  builder `fact` field, `compile_fact`, `FunctionBinding::fact`/`narrowing`
  and all seven `.narrowing(...)` calls are gone. `load_catalog` calls it per
  function and fails with `` `types.{name}` ``.
- Catalog migration: eight clauses in `functions.yaml` (`file_exists` now
  `file`), `is_project_file` in the fixture.
- Tests added: catalog `tests.rs` (overload agreement, equivalent target
  spellings, omitted parameter, duplicate contract despite clauses);
  `flow_sensitive_narrowing` (clauses via signature text, grammar versus
  compiler rejection, overload disagreement incl. programmatic later overload
  omitting the parameter); `host_functions` (rejection through
  `HostFunctionCatalog::load`; four reader cells); two assertions updated to
  `file`.
- Gates: `just test` 12926 passed. `just lint`: darkmatter-cli fails with the
  pre-existing `too_many_arguments` (`render.rs:22`); `cargo clippy -p
  darkmatter -p dmls --all-targets -D warnings` clean. Dependents: claudine
  `host_catalog`/`call_check` 59 passed. Tier: all L1; nothing spawned.
- Doc drift fixed: module docs in registration.rs and artifact.rs and the
  host-functions topic example/rejection list. Fuller docs remain Phase 5.
  No skill change needed yet.

## Phase 3

- Most of Phase 3 landed in Phase 2 (see its decision item): `artifact.rs::rebuild`
  no longer calls a setter and compares the whole decoded fact with the
  catalog's compiled fact (absence both ways, parameter, kind, type spelling
  including `<non-null>`, position); `project.facts.json` blessed. Reviewed the
  `artifact.rs` messages ("records no fact, but the catalog declares one",
  "records a fact for ..., but the catalog declares none", "differs from the
  one the catalog declares") and the module doc; they match behavior.
- Reader matrix (`host_functions.rs`) already carries one-edit-per-cell rows
  for `facts: []`, an artifact-only fact on `project_label`, wrong kind, wrong
  parameter, wrong position, and wrong type spelling (`file`→`string`,
  `<non-null>`); the unedited artifact is the control.
- Added `a_host_facts_artifact_that_disagrees_with_the_catalog_fact_reports_that_function_incomplete`
  in `dmls/tests/l1/host_function_diagnostics.rs` (`facts: []` and wrong kind):
  `dm.expression.incomplete_analysis` is reported, the message names the
  catalog, and `project_label(42)` is still `incompatible_call` (that
  function stays complete). Whole-file failure rows are unchanged.
- Code-smell grep over `artifact.rs` (`serde(default)`, `unwrap_or_default`,
  `filter_map(.. as_str())`) found nothing new.
- Gates: `just test` 12927 passed, 10 skipped. `just lint`: only the
  pre-existing `darkmatter-cli` `render.rs:22` too_many_arguments fails
  (untouched by this feature). Tier: all L1; nothing spawned.
