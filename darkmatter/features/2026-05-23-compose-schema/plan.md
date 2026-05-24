---
created: 2026-05-23
phases: 5
start_phase: 1
---

# Execution Plan: Schema Validation in the Compose Pipeline

This plan implements the "Schema Validation" stage in the `md compose` pipeline as specified in `darkmatter/features/2026-05-23-compose-schema/spec.md`.

## Phase 1: Foundation and Types

Establish the necessary data structures and error types to support schema validation within the compose pipeline.

- [ ] **Task 1.1: Update `ComposeOptions`**
  - Add `baseline_schema: Option<SimplifiedSchema>` field to `ComposeOptions` in `darkmatter/lib/src/markdown/compose/types.rs`.
  - Implement `pub fn with_baseline_schema(mut self, schema: SimplifiedSchema) -> Self` builder method.
  - Update `ComposeOptions::Debug` implementation to include `baseline_schema` (showing `Some(..)`/`None` without dumping full content).

- [ ] **Task 1.2: Define `SchemaValidationFailed` error variant**
  - Add `SchemaValidationFailed` variant to `MarkdownError` (or the appropriate compose error enum in `darkmatter/lib/src/markdown/compose/types.rs`).
  - The variant should include `path: PathBuf`, `problems: Vec<ValidationProblem>`, and `summary: String`.
  - Implement `biscuit_terminal::errors::BlockError` for this variant to produce the styled output required by the spec.

- [ ] **Task 1.3: Update Performance Metrics**
  - Add `PerfMetricKind::SchemaValidation` to `darkmatter/lib/src/markdown/compose/perf.rs`.
  - Add `ComposeStage::SchemaValidation` to `darkmatter/lib/src/markdown/compose/types.rs`.
  - Ensure the fixed-size perf arrays and stage display text are updated to include the new stage.

## Phase 2: Schema Validation Implementation

Implement the core logic for the new validation stage.

- [ ] **Task 2.1: Create `schema_validation.rs`**
  - Create `darkmatter/lib/src/markdown/compose/schema_validation.rs`.
  - Implement `pub(crate) fn run(...)` that performs the validation logic.
  - Logic: Check for `$schema` or baseline -> Build `DarkmatterSchemas` -> Call `.validate(&md)`.
  - Convert `SchemaError` and `ValidationReport` into the `MarkdownError::SchemaValidationFailed` variant.

- [ ] **Task 2.2: Unit Testing for Validation Logic**
  - Add unit tests in `schema_validation.rs` covering:
    - No-op case (no `$schema`, no baseline).
    - Successful validation with document `$schema`.
    - Failed validation with document `$schema`.
    - Successful/Failed validation with `baseline_schema` from `ComposeOptions`.
    - Interaction between document overrides (`--set`) and validation (validation runs post-override).

## Phase 3: Pipeline Integration

Integrate the validation stage into the existing compose pipeline and ensure cache safety.

- [ ] **Task 3.1: Inject Stage into Pipeline**
  - Modify `run_compose_pipeline_internal` in `darkmatter/lib/src/markdown/compose/mod.rs`.
  - Call `schema_validation::run` immediately after `prepare_frontmatter_for_compose` and before frontmatter interpolation.
  - Update the module's doc comments enumerating pipeline stages.

- [ ] **Task 3.2: Update Cache Hashing**
  - Modify `options_hash` in `darkmatter/lib/src/markdown/compose/cache/hashing.rs` to include `baseline_schema`.
  - Use canonical JSON from `schemas::to_json_schema(...)` for hashing.
  - Add a unit test in `hashing.rs` proving that different baseline schemas result in different option hashes.

## Phase 4: Integration Testing and Regression

Verify the feature end-to-end and ensure no regressions.

- [ ] **Task 4.1: Implementation of Planner-Prompt Regression Test**
  - Create an integration test (e.g., in `darkmatter/lib/src/markdown/compose/mod.rs` or CLI tests).
  - Use a document with an invalid property that would normally cause a shell expansion failure.
  - Assert that `md compose` fails fast at the schema stage with the correct `BlockError`.

- [ ] **Task 4.2: Snapshot Testing**
  - Create `insta` snapshots of the `BlockError` output for various failure cases (missing property, wrong type, etc.) to ensure styled output matches the specification.

- [ ] **Task 4.3: Recursive Compose Verification**
  - Add a test case with transcluded documents where the child has a schema.
  - Verify that parent `set=` overlays are correctly applied to the child before child validation.

## Phase 5: Documentation and Polish

Update documentation and skill files to reflect the new pipeline stage.

- [ ] **Task 5.1: Update `SKILL.md`**
  - Update `.claude/skills/darkmatter/SKILL.md` to include "Schema Validation" in the "Compose Pipeline" section.
  - Regenerate the `hash:` frontmatter for the skill file using `md hash`.

- [ ] **Task 5.2: Final Review**
  - Verify that no-schema documents still work as expected (no regressions for existing users).
  - Ensure all US-English naming conventions are followed.
  - Check that no interactive prompts or `cargo fmt` were used.
