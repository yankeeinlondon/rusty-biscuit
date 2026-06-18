---
phases: 6
created: 2026-05-11
start_phase: 1
source_files_during_phase_1:
  - darkmatter/lib/Cargo.toml
  - darkmatter/lib/src/markdown/mod.rs
  - darkmatter/lib/src/markdown/schemas/mod.rs
  - darkmatter/lib/src/markdown/schemas/errors.rs
  - darkmatter/lib/src/markdown/schemas/simplified/mod.rs
  - darkmatter/lib/src/markdown/schemas/simplified/types.rs
  - darkmatter/lib/src/markdown/schemas/simplified/grammar.rs
  - darkmatter/lib/src/markdown/schemas/simplified/serialize.rs
  - darkmatter/lib/tests/schemas_grammar_proptest.rs
  - darkmatter/lib/tests/schemas_grammar_proptest.proptest-regressions
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - darkmatter/lib/src/markdown/schemas/mod.rs
  - darkmatter/lib/src/markdown/schemas/errors.rs
  - darkmatter/lib/src/markdown/schemas/simplified/mod.rs
  - darkmatter/lib/src/markdown/schemas/simplified/grammar.rs
  - darkmatter/lib/src/markdown/schemas/simplified/convert.rs
  - darkmatter/lib/tests/schemas_convert_snapshots.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - darkmatter/lib/Cargo.toml
  - darkmatter/lib/src/markdown/schemas/mod.rs
  - darkmatter/lib/src/markdown/schemas/errors.rs
  - darkmatter/lib/src/markdown/schemas/format.rs
  - darkmatter/lib/src/markdown/schemas/validate.rs
  - darkmatter/lib/src/markdown/schemas/resolve.rs
  - darkmatter/lib/tests/schemas_validate_table.rs
  - darkmatter/lib/tests/fixtures/validate/simple_required_present/schema.yaml
  - darkmatter/lib/tests/fixtures/validate/simple_required_present/doc.md
  - darkmatter/lib/tests/fixtures/validate/simple_required_present/expected.json
  - darkmatter/lib/tests/fixtures/validate/missing_required/schema.yaml
  - darkmatter/lib/tests/fixtures/validate/missing_required/doc.md
  - darkmatter/lib/tests/fixtures/validate/missing_required/expected.json
  - darkmatter/lib/tests/fixtures/validate/inline_schema/doc.md
  - darkmatter/lib/tests/fixtures/validate/inline_schema/expected.json
  - darkmatter/lib/tests/fixtures/validate/range_violation/doc.md
  - darkmatter/lib/tests/fixtures/validate/range_violation/expected.json
  - darkmatter/lib/tests/fixtures/validate/json_schema_ref/schema.json
  - darkmatter/lib/tests/fixtures/validate/json_schema_ref/doc.md
  - darkmatter/lib/tests/fixtures/validate/json_schema_ref/expected.json
  - darkmatter/lib/tests/fixtures/validate/root_union/doc.md
  - darkmatter/lib/tests/fixtures/validate/root_union/expected.json
  - darkmatter/lib/tests/fixtures/validate/root_union_none_match/doc.md
  - darkmatter/lib/tests/fixtures/validate/root_union_none_match/expected.json
  - darkmatter/lib/tests/fixtures/validate/enum_member_invalid/doc.md
  - darkmatter/lib/tests/fixtures/validate/enum_member_invalid/expected.json
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
  - darkmatter/cli/src/args.rs
  - darkmatter/cli/src/commands.rs
  - darkmatter/cli/src/commands/schema/mod.rs
  - darkmatter/cli/src/commands/schema/validate.rs
  - darkmatter/cli/tests/schema_validate.rs
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_files_during_phase_5:
  - darkmatter/lib/src/markdown/schemas/mod.rs
  - darkmatter/lib/src/markdown/schemas/detect.rs
  - darkmatter/lib/tests/schemas_detect_table.rs
  - darkmatter/lib/tests/fixtures/detect/basic_scalars/inputs/doc.md
  - darkmatter/lib/tests/fixtures/detect/basic_scalars/expected.yaml
  - darkmatter/lib/tests/fixtures/detect/date_url_email/inputs/doc.md
  - darkmatter/lib/tests/fixtures/detect/date_url_email/expected.yaml
  - darkmatter/lib/tests/fixtures/detect/arrays/inputs/doc.md
  - darkmatter/lib/tests/fixtures/detect/arrays/expected.yaml
  - darkmatter/lib/tests/fixtures/detect/merge_widen_number/inputs/a.md
  - darkmatter/lib/tests/fixtures/detect/merge_widen_number/inputs/b.md
  - darkmatter/lib/tests/fixtures/detect/merge_widen_number/options.json
  - darkmatter/lib/tests/fixtures/detect/merge_widen_number/expected.yaml
  - darkmatter/lib/tests/fixtures/detect/merge_disjoint_union/inputs/a.md
  - darkmatter/lib/tests/fixtures/detect/merge_disjoint_union/inputs/b.md
  - darkmatter/lib/tests/fixtures/detect/merge_disjoint_union/options.json
  - darkmatter/lib/tests/fixtures/detect/merge_disjoint_union/expected.yaml
  - darkmatter/lib/tests/fixtures/detect/merge_required_promotion/inputs/a.md
  - darkmatter/lib/tests/fixtures/detect/merge_required_promotion/inputs/b.md
  - darkmatter/lib/tests/fixtures/detect/merge_required_promotion/options.json
  - darkmatter/lib/tests/fixtures/detect/merge_required_promotion/expected.yaml
  - darkmatter/lib/tests/fixtures/detect/no_merge_no_required/inputs/a.md
  - darkmatter/lib/tests/fixtures/detect/no_merge_no_required/inputs/b.md
  - darkmatter/lib/tests/fixtures/detect/no_merge_no_required/options.json
  - darkmatter/lib/tests/fixtures/detect/no_merge_no_required/expected.yaml
  - darkmatter/cli/src/args.rs
  - darkmatter/cli/src/commands.rs
  - darkmatter/cli/src/commands/schema/mod.rs
  - darkmatter/cli/src/commands/schema/detect.rs
  - darkmatter/cli/tests/schema_detect.rs
docs_updated_during_phase_5: []
docs_created_during_phase_5: []
skills_files_updated_during_phase_5: []
source_files_during_phase_6:
  - darkmatter/lib/Cargo.toml
  - darkmatter/lib/src/markdown/schemas/errors.rs
  - darkmatter/lib/src/markdown/schemas/mod.rs
  - darkmatter/lib/src/markdown/schemas/completion.rs
  - darkmatter/lib/src/markdown/errors/mod.rs
  - darkmatter/lib/benches/schema_validation.rs
  - darkmatter/cli/src/commands/schema/validate.rs
docs_updated_during_phase_6: []
docs_created_during_phase_6: []
skills_files_updated_during_phase_6: []
packages:
  - darkmatter
---

# Execution Plan: Schemas in Darkmatter

## Phase 1: Foundation (AST and Parser)
*Dependency: None*

- [x] Implement `SimplifiedSchema`, `SimplifiedType`, and `Constraint` AST models in `darkmatter/lib/src/markdown/schemas/simplified/types.rs`.
- [x] Implement EBNF lexer and parser for the type-and-constraint strings in `darkmatter/lib/src/markdown/schemas/simplified/grammar.rs`.
- [x] Implement YAML-shape layer parsing over `serde_yaml_ng::Value` in `darkmatter/lib/src/markdown/schemas/simplified/mod.rs` to handle property-level and root-level unions.
- [x] Create `SchemaError` variants for parsing (`Grammar` variant) in `darkmatter/lib/src/markdown/schemas/errors.rs`.
- [x] Create unit tests and `proptest` round-trip tests to verify random valid type expressions parse and re-serialise correctly.
- [x] **Validation Checkpoint**: AST structures correctly represent all types, arrays, union variants, and constraints from the spec. Parser passes all proptests.

## Phase 2: JSON Schema Conversion
*Dependency: Phase 1*

- [x] Implement `to_json_schema(&SimplifiedSchema) -> serde_json::Value` mapping in `darkmatter/lib/src/markdown/schemas/simplified/convert.rs`.
- [x] Implement hoisting logic for `required` and `default` constraints on property-level unions within the conversion step.
- [x] Implement generation of `x-darkmatter-*` extensions annotations (`x-darkmatter-match`, `x-darkmatter-url-scheme`).
- [x] Add `insta` snapshot tests in `tests/snapshots/` to cover all rows of the mapping table from the spec.
- [x] **Validation Checkpoint**: Snapshot tests prove that every AST variant outputs exact expected Draft 2020-12 JSON Schema structures.

## Phase 3: Resolution & Validation Engine
*Dependency: Phase 2*

- [x] Add new dependencies (`jsonschema`, `globset`, `url`) to `darkmatter/lib/Cargo.toml`.
- [x] Implement custom format validators (`darkmatter-file` with glob match, `darkmatter-url-scheme`) in `darkmatter/lib/src/markdown/schemas/format.rs`.
- [x] Implement `ValidatorCache` with LRU eviction and `jsonschema::Validator` construction in `darkmatter/lib/src/markdown/schemas/validate.rs`.
- [x] Implement `$schema` resolution (inline, file references, root unions) and baseline merge logic in `darkmatter/lib/src/markdown/schemas/resolve.rs`.
- [x] Implement `DarkmatterSchemas`, `EffectiveSchema`, `ValidationReport`, and `ValidationProblem` models in `darkmatter/lib/src/markdown/schemas/mod.rs`.
- [x] Add table-driven validation tests (`tests/fixtures/validate/<case>/...`).
- [x] **Validation Checkpoint**: Table-driven tests pass for valid/invalid YAML inputs against constructed schemas. File references resolve cleanly.

## Phase 4: CLI Validation Command
*Dependency: Phase 3*

- [x] Create CLI command scaffolding in `darkmatter/cli/src/commands/schema/mod.rs`.
- [x] Implement `md schema validate <file>...` in `darkmatter/cli/src/commands/schema/validate.rs`.
- [x] Implement `pretty` output format via `biscuit-terminal::Prose` and `json` output format for validation reports.
- [x] Handle exit codes (0=success, 1=failed validation, 2=schema error, 3=parse error) and `BASELINE_SCHEMA` env var fallback.
- [x] Add CLI tests using `assert_cmd` and `predicates` to verify output shapes and exit codes.
- [x] **Validation Checkpoint**: The CLI command `md schema validate` operates properly on valid and invalid files, matching standard output designs.

## Phase 5: Schema Detection and CLI Command
*Dependency: Phase 1*
*Parallelizable*: Yes, this phase can be executed in parallel with Phase 2, 3, and 4.

- [x] Implement single-file schema inference in `darkmatter/lib/src/markdown/schemas/detect.rs`.
- [x] Implement multi-file merge logic (union detections, widening hierarchy) in `darkmatter/lib/src/markdown/schemas/detect.rs`.
- [x] Add table-driven tests for schema detection (`tests/fixtures/detect/<case>/...`).
- [x] Implement `md schema detect <file>...` in `darkmatter/cli/src/commands/schema/detect.rs`.
- [x] Wire up formatting (`yaml` / `json` output formats) and the `--merge` flag.
- [x] **Validation Checkpoint**: Running `md schema detect` over sample documents produces accurate SimplifiedSchema yaml matching the type widening rules.

## Phase 6: Final Integration and Release
*Dependency: Phase 4, Phase 5*

- [x] Add `biscuit_terminal::errors::BlockError` implementations to all `SchemaError` variants for rich CLI error rendering.
- [x] Write Criterion performance tests for evaluating validation speed across a 1000-file corpus.
- [x] Integrate shell completions for `file` and `enum` types as read-only consumer.
- [x] **Validation Checkpoint**: Complete build, lint, and test pass of the `darkmatter` project workspace. All features documented and performance matches expectations.
