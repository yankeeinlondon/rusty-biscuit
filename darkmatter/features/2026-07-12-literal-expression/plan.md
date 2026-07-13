---
agent: codex/
total_phases: 7
created: 2026-07-12
phase: 5
yolo: "true"
source_files_during_phase_1:
  - darkmatter/lib/tests/schemas_literal_expression.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1:
  - darkmatter/features/2026-07-12-literal-expression/phase1-impact.md
  - darkmatter/features/2026-07-12-literal-expression/phase1-test-matrix.md
  - darkmatter/features/2026-07-12-literal-expression/phase1-baseline.txt
  - darkmatter/features/2026-07-12-literal-expression/phase1-baseline-about.txt
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - darkmatter/lib/src/markdown/schemas/simplified/types.rs
  - darkmatter/lib/src/markdown/schemas/simplified/grammar.rs
  - darkmatter/lib/src/markdown/schemas/simplified/serialize.rs
  - darkmatter/lib/src/markdown/schemas/simplified/convert.rs
  - darkmatter/lib/src/markdown/schemas/about.rs
  - darkmatter/lib/src/markdown/schemas/triggers/matcher.rs
  - darkmatter/lib/src/markdown/compose/expression/catalog/mod.rs
  - darkmatter/lib/tests/schemas_literal_expression.rs
  - darkmatter/lib/tests/schemas_grammar_proptest.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - darkmatter/lib/src/markdown/schemas/simplified/convert.rs
  - darkmatter/lib/src/markdown/schemas/format.rs
  - darkmatter/lib/src/markdown/schemas/triggers/matcher.rs
  - darkmatter/lib/src/markdown/schemas/triggers/grammar.rs
  - darkmatter/lib/tests/schemas_literal_expression.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
  - darkmatter/lib/src/markdown/schemas/coerce.rs
  - darkmatter/lib/src/markdown/schemas/discriminant.rs
  - darkmatter/lib/src/markdown/schemas/format.rs
  - darkmatter/lib/src/markdown/schemas/validate.rs
  - darkmatter/lib/src/markdown/schemas/mod.rs
  - darkmatter/lib/tests/schemas_literal_expression.rs
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_files_during_phase_5:
  - darkmatter/lib/src/markdown/schemas/simplified/yaml_scalar.rs
  - darkmatter/lib/src/markdown/schemas/simplified/source.rs
  - darkmatter/lib/src/markdown/schemas/simplified/mod.rs
  - darkmatter/lib/src/markdown/schemas/mod.rs
  - darkmatter/dmls/src/overlay/expressions.rs
  - darkmatter/dmls/src/providers/dsl.rs
  - darkmatter/dmls/src/providers/frontmatter.rs
  - darkmatter/dmls/src/diagnostics/frontmatter.rs
docs_updated_during_phase_5: []
docs_created_during_phase_5: []
skills_files_updated_during_phase_5: []
packages:
  - darkmatter
---

# Execution Plan: SimplifiedSchema `literal` and `expression` Types

This plan implements the two schema types through the Darkmatter library and
DMLS, then closes with public documentation and L1/L2 verification. Tasks are
ordered so editor behavior consumes stable library APIs rather than duplicating
schema semantics.

## Success Criteria

- [ ] `literal(value)` supports typed scalar identity, canonical round trips,
  `const` emission, arrays, valid defaults, coercion, trigger matching, and
  mixed/property and inline-object unions.
- [ ] `expression` validates strings with the condition-mode parse superset,
  remains parse-only and side-effect-free, emits the registered
  `darkmatter-expression` format, and coerces native booleans/numbers to
  canonical strings.
- [ ] Literal discriminants select exactly one union arm only when typed values
  match unambiguously; absent, unknown, duplicate, or conflicting values retain
  existing union behavior.
- [ ] DMLS provides schema-gated expression and literal completion, hover,
  diagnostics, code actions, and discriminated-union narrowing with accurate
  YAML decoded-to-authored ranges.
- [ ] Existing schemas retain byte-identical validation behavior, public
  descriptors/docs stay in parity, and `just test`, `just test-l2`, and
  `just lint` pass from the Darkmatter package area.

## Phase 1: Baseline, Impact Analysis, and Test Matrix

- [x] Refresh the GitNexus index with `node .gitnexus/run.cjs analyze` because
  the planning-time index reported no indexed repositories; confirm
  `gitnexus://repo/rusty-biscuit/context` is current before code edits.
- [x] Run upstream GitNexus impact analysis before editing each existing symbol
  selected during implementation, including the grammar entry point, schema
  conversion/type-fragment logic, constraint linting, coercion target
  selection/write-back, format registration, trigger primitive matching,
  union validation/reporting, and DMLS frontmatter completion/hover/diagnostic
  entry points; record direct callers, affected processes, and risk. Stop and
  warn before proceeding on any HIGH or CRITICAL result.
- [x] Capture the pre-change output for representative existing schema fixtures
  and `md schema validate` union failures so byte-identical behavior can be
  checked for schemas without literal discriminants.
- [x] Map each acceptance criterion to a focused test location: grammar and
  serialization unit/proptest coverage, conversion snapshots, validation table
  fixtures, coercion tests, trigger matcher tests, schema-about parity tests,
  DMLS provider tests, and `dmls/tests/no_side_effects.rs`.
- [x] Add failing tests for the complete scalar matrix and boundary cases:
  quoted versus bare strings/booleans/numbers, numberlike boundaries, rejected
  bare `null`, protected punctuation, missing/multiple literal values, arrays,
  defaults, optional nullability, pending values, expression dialect superset,
  and mappings/sequences that must not coerce.
- [x] Validation checkpoint: verify the baseline suites are green before new
  failing tests, and verify each new test fails for the intended missing
  behavior rather than fixture or harness errors.

## Phase 2: Schema Vocabulary, Grammar, and Canonical Serialization

- [x] Add `SimplifiedType::Literal` and `SimplifiedType::Expression` with stable
  keyword mappings, plus `Constraint::LiteralValue(serde_json::Value)` whose
  keyword placeholder mirrors `Members`; update exhaustive matches and parity
  tests without making either type inferable in `detect.rs`.
- [x] Extract or reuse the enum member lexer so literal parsing accepts exactly
  one bare or quoted positional scalar, preserves quoted values as strings,
  types bare bool/numberlike values, and rejects bare `null` with actionable
  errors; keep `SimplifiedType: Copy` by storing the value in the constraint.
- [x] Extend grammar parsing so `literal(value; constraints)` follows the enum
  delimiter rules, reports `literal requires a value`, directs multiple values
  to `enum(...)`, rejects a bare Literal atom without `LiteralValue`, and allows
  the uniform `[]` suffix.
- [x] Parse bare `expression` through the ordinary keyword path and reserve
  parameterized `expression(...)` by rejecting it in v1.
- [x] Extend canonical serialization and proptest generators/shrinkers so all
  literal scalar forms, quoting-sensitive values, arrays, unions, and
  `expression` parse-serialize-reparse to equivalent ASTs.
- [x] Add/update schema type descriptors and parity coverage so
  `md schema about` lists both types in deterministic order with their allowed
  constraints and coercion semantics.
- [x] Validation checkpoint: run focused grammar, serialization, proptest, and
  descriptor parity tests; confirm malformed forms produce the specified
  `SchemaError` class and messages.

## Phase 3: JSON Schema Conversion, Formats, Linting, and Triggers

- [x] Compile Literal atoms to typed JSON Schema `const` fragments, preserving
  the existing optional-nullable wrapper and placing `const` under `items` for
  `literal(x)[]`; add snapshots for strings, booleans, numbers, arrays, mixed
  unions, and inline-object discriminants.
- [x] Register `DARKMATTER_EXPRESSION_FORMAT` beside the YAML/JSON formats and
  implement it with the pure `parse_condition(value).is_ok()` predicate; add a
  corpus regression proving every value-dialect-valid expression also parses
  in condition mode.
- [x] Compile Expression atoms to `{ "type": "string", "format":
  "darkmatter-expression" }`, preserving nullable, required, array, default,
  generated, and pending-value behavior shared by YAML/JSON.
- [x] Enforce constraint applicability: Literal permits only `required` and an
  equal `default`, rejects `suggest` and unrelated constraints; Expression
  mirrors YAML/JSON and rejects string constraints and `suggest`; schema loading
  rejects defaults that violate the literal const or expression format.
- [x] Extend trigger matching so Literal performs typed equality as a pure
  constraint and Expression mirrors YAML/JSON string/format behavior, with
  tests covering allowed matches and prohibited/nonmatching values.
- [x] Parallelizable after Phase 2: format registration/expression conversion
  and literal conversion/trigger matching may proceed independently, with
  coordination limited to exhaustive matches in shared schema modules.
- [x] Validation checkpoint: run conversion snapshots, schema validation table
  tests, format/default lint tests, trigger tests, and a side-effect test proving
  expression validation never evaluates functions, shell, I/O, or context.

## Phase 4: Coercion and Shared Discriminated-Union Selection

- [x] Extend coercion target discovery so non-string Literal values reuse the
  existing boolish/numberlike conversions, string literals never coerce, and
  write-back occurs only when the resulting value validates; cover pending
  values and equality failures.
- [x] Extend Expression coercion so native booleans and numbers serialize to
  canonical expression strings while quoted strings retain spelling and
  mappings/sequences remain type mismatches.
- [x] Introduce one presentation-neutral library helper for selecting a union
  arm from shared Literal discriminants: require the same discriminant key in
  at least two arms, an authored instance value, exactly one type-sensitive
  match, and agreement across all qualifying keys.
- [x] Reuse the selector in library validation/reporting so matched literal
  discriminants report only the chosen arm's missing/unknown/type problems;
  preserve the current `anyOf` diagnostics byte-for-byte for schemas without
  literal discriminants and for absent, unknown, duplicate, ambiguous, or
  conflicting discriminants.
- [x] Add validation fixtures for root and property-level inline-object unions,
  typed `2` versus string `'2'`, multiple agreeing/conflicting discriminants,
  duplicate tags, partial objects, and `[literal(auto), number]`.
- [x] Escape-hatch checkpoint: measure the validation-reporting change against
  the focused fixtures. If diagnostics narrowing requires an unexpectedly broad
  reporting-layer rewrite, document the evidence and split only that half into
  a follow-up fix as pre-approved by Q4; retain the shared selector and DMLS key
  completion narrowing in this feature. **Not triggered:** both root and
  property-level narrowing landed in the existing reporting layer with no broad
  rewrite (a synthetic single-arm validator for the property path, the existing
  arm validators for the root path), so no follow-up split was needed.
- [x] Validation checkpoint: run focused coercion and validation suites and
  compare the Phase 1 baseline outputs for all pre-existing schema cases.

## Phase 5: Shared YAML Scalar Projection and DMLS Expression Intelligence

- [x] Extract decoded-YAML-scalar-to-authored-byte projection from
  `simplified/source.rs` into a public or crate-shared Darkmatter helper without
  changing existing schema-source ranges; cover plain, single-quoted,
  double-quoted escaped, and multibyte scalars plus safe whole-node fallback.
  Landed as the public `schemas::simplified::yaml_scalar` module
  (`DecodedScalar` + `decode_scalar`/`decode_scalar_at`, re-exported from
  `schemas`); `source.rs` now builds on it and its ranges are unchanged.
- [x] Expose effective-schema queries that identify an Expression-typed value
  and its authored/decoded span without duplicating type or union logic in DMLS.
  `providers::frontmatter::expression_values`/`ExpressionValue` reuse the
  existing `def_at_path`/`atoms_of` union walk plus the library `DecodedScalar`
  projection; no type/union logic is re-implemented.
- [x] Refactor `overlay::expressions` only as needed to share catalog completion,
  `format_ctx_hover_block`, `format_function_block`, deepest-call lookup, and
  unknown-root analysis between body interpolation sites and schema-typed
  frontmatter values. New shared authorities: `completion_candidates`,
  `hover_markdown`, `is_unknown_root`, `value_completion_partial`; `dsl.rs`
  delegates to them.
- [x] Activate completion inside Expression-typed frontmatter values for
  `ctx.*`, expression functions, and same-document frontmatter keys, scoped to
  the YAML value and existing `.`/`(` triggers.
- [x] Activate byte-identical expression hovers and emit
  `dm.expression.malformed` / `dm.expression.unknown_identifier` diagnostics
  with source `darkmatter.frontmatter`; project parser byte offsets through the
  shared scalar mapper and exclude YAML quotes from precise ranges.
- [x] Suppress only the generic schema-format diagnostic replaced by
  `dm.expression.malformed`; retain all unrelated schema problems on the same
  property and verify native scalar coercion does not create false diagnostics.
  Suppression is scoped to `TypeMismatch`/`ConstraintViolation` on
  expression-typed **scalar** pointers, so a mapping/sequence value keeps its
  schema error.
- [x] Parallelizable after the projection API is fixed: completion/hover wiring
  and diagnostic deduplication/range tests can be implemented concurrently in
  separate DMLS provider modules.
- [x] Validation checkpoint: focused DMLS tests prove schema gating, completion
  contents/details, hover byte parity, unknown-root policy, one malformed
  diagnostic, accurate plain/quoted/escaped/multibyte ranges, fallback ranges,
  and no side effects. Expression analysis reuses only pure library parsers
  (`parse`/`parse_condition`), so it adds no new side-effect surface over the
  interpolation path already covered by `tests/no_side_effects.rs`.

## Phase 6: DMLS Literal UX and Union Narrowing

- [ ] Add schema accessors that expose Literal values alongside enum members,
  including every Literal arm in a property union without erasing scalar type.
- [ ] Offer exactly the authored Literal value as a preselected completion item;
  for unions, combine each Literal value with the existing non-literal
  scaffolds and serialize insertion text as valid YAML for its scalar type.
- [ ] Render Literal hover details as `Type: **literal**` plus the exact value,
  then the existing constraint and description lines.
- [ ] Make add-missing-required-key code actions insert the Literal value rather
  than an empty scaffold, including string quoting where YAML requires it.
- [ ] Reuse the Phase 4 arm selector for sibling-key completion and DMLS
  missing-required/unknown-key diagnostics; never implement a second DMLS-only
  discriminant algorithm.
- [ ] Add DMLS tests for matched-arm key sets and diagnostics, root/property
  unions, typed equality, absent/unknown/duplicate/conflicting tags, exact-value
  completion, hover, and correct-by-construction required-key edits.
- [ ] Parallelizable after shared accessors land: Literal completion/hover/code
  actions and union-narrowing provider work can proceed independently.
- [ ] Validation checkpoint: run the focused DMLS provider and integration tests
  and confirm non-Literal schemas retain their baseline completion and
  diagnostic behavior.

## Phase 7: Documentation, Hashes, and Release Gate

- [ ] Update `darkmatter/docs/topics/schema-definition.md` with Literal and
  Expression syntax, typed scalar/quoting rules, arrays, constraints, coercion,
  union discriminants, trigger use, parse-only expression semantics, and the
  explicit Literal-versus-enum relationship; do not document deferred
  `expression(condition)` as available.
- [ ] Review all changed `///`, `//!`, and inline comments for behavioral drift;
  remove or correct stale narration and update the Darkmatter skill only if the
  public schema architecture or workflow changed.
- [ ] Refresh every changed Markdown state hash with Darkmatter's Markdown-aware
  hasher (`md hash <file>`), including skill documentation if changed; do not use
  a generic file hash for Markdown.
- [ ] Run `just test` and `just test-l2` from the Darkmatter package area, then
  `just lint`; do not run `cargo fmt` in write mode.
- [ ] Exercise `md schema about` and representative `md schema validate`
  examples for Literal, Expression, trigger matching, coercion, and narrowed
  diagnostics on macOS; review implementation choices for Windows/Linux path,
  newline, and Unicode portability.
- [ ] Run `cargo fmt --check` only as a read-only diagnostic if needed, and
  inspect the full diff for unrelated formatting, snapshot churn, stale docs,
  accidental schema detection changes, or regenerated files outside scope.
- [ ] Run GitNexus `detect_changes(scope: "compare", base_ref: "main")` and
  verify changed symbols and execution flows match the planned schema/DMLS
  surface; investigate any unexpected affected process before handoff.
- [ ] Final validation checkpoint: trace every specification acceptance
  criterion to a passing test or parity check, record any Q4 diagnostics split
  explicitly, and summarize changed files, commands run, and skipped checks.
