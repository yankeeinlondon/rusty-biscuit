---
spec: "/Volumes/coding/wt/rusty-biscuit/feat-pipeline/claudine/features/2026-10-29-recursive-schema-types/spec.md"
plan: "claudine/features/2026-10-29-recursive-schema-types/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
source_files_during_phase_1:
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/recursive_schema_validator_probe.rs
    - darkmatter/lib/tests/l1/recursive_schema_golden.rs
    - darkmatter/lib/tests/fixtures/recursive_schema_golden/manifest.yaml
    - darkmatter/lib/tests/fixtures/recursive_schema_golden/inputs/
    - darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/
docs_updated_during_phase_1: []
docs_created_during_phase_1:
    - claudine/features/2026-10-29-recursive-schema-types/inventory/contracts.md
    - claudine/features/2026-10-29-recursive-schema-types/inventory/consumers.md
    - darkmatter/lib/tests/fixtures/recursive_schema_golden/README.md
skills_files_updated_during_phase_1: []
packages:
    - darkmatter
    - darkmatter-cli
    - dmls
    - claudine
    - claudine-gen
human_review: false
source_files_during_phase_2:
    - claudine/gen/src/schema_compat.rs
    - claudine/lib/src/composition/schema/classify.rs
    - darkmatter/cli/src/commands/schema/assignment.rs
    - darkmatter/dmls/src/diagnostics/named_types.rs
    - darkmatter/dmls/src/overlay/mod.rs
    - darkmatter/dmls/src/overlay/named_types.rs
    - darkmatter/dmls/src/overlay/suggestions.rs
    - darkmatter/dmls/src/providers/frontmatter.rs
    - darkmatter/dmls/tests/l1/named_type_references.rs
    - darkmatter/lib/src/markdown/compose/context/catalog.rs
    - darkmatter/lib/src/markdown/compose/expression/call_binding/translate.rs
    - darkmatter/lib/src/markdown/compose/expression/scope/shape.rs
    - darkmatter/lib/src/markdown/schemas/completion.rs
    - darkmatter/lib/src/markdown/schemas/detect.rs
    - darkmatter/lib/src/markdown/schemas/errors.rs
    - darkmatter/lib/src/markdown/schemas/frontmatter_shape.rs
    - darkmatter/lib/src/markdown/schemas/mod.rs
    - darkmatter/lib/src/markdown/schemas/phase.rs
    - darkmatter/lib/src/markdown/schemas/resolve.rs
    - darkmatter/lib/src/markdown/schemas/resolve/graph.rs
    - darkmatter/lib/src/markdown/schemas/resolve/recursive.rs
    - darkmatter/lib/src/markdown/schemas/simplified/convert.rs
    - darkmatter/lib/src/markdown/schemas/simplified/definitions.rs
    - darkmatter/lib/src/markdown/schemas/simplified/lint.rs
    - darkmatter/lib/src/markdown/schemas/simplified/mod.rs
    - darkmatter/lib/src/markdown/schemas/simplified/query.rs
    - darkmatter/lib/src/markdown/schemas/simplified/serialize.rs
    - darkmatter/lib/src/markdown/schemas/simplified/source.rs
    - darkmatter/lib/src/markdown/schemas/simplified/standalone.rs
    - darkmatter/lib/src/markdown/schemas/simplified/subtype.rs
    - darkmatter/lib/src/markdown/schemas/simplified/types.rs
    - darkmatter/lib/src/markdown/schemas/tests/embedded_sources.rs
    - darkmatter/lib/src/markdown/schemas/triggers/matcher.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/recursive_schema_types.rs
    - darkmatter/lib/tests/l1/schemas_named_type_merge.rs
    - darkmatter/lib/tests/l1/schemas_union_postfix.rs
    - darkmatter/lib/tests/l1/suggest_constraint_phase4.rs
docs_updated_during_phase_2:
    - darkmatter/docs/topics/schemas/definition.md
    - darkmatter/docs/topics/schemas/dmls-schema-support.md
    - claudine/features/2026-10-29-recursive-schema-types/inventory/contracts.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
    - .claude/skills/darkmatter/schema.md
source_files_during_phase_3:
    - darkmatter/lib/src/markdown/compose/schema_validation.rs
    - darkmatter/lib/src/markdown/schemas/coerce.rs
    - darkmatter/lib/src/markdown/schemas/errors.rs
    - darkmatter/lib/src/markdown/schemas/mod.rs
    - darkmatter/lib/src/markdown/schemas/phase.rs
    - darkmatter/lib/src/markdown/schemas/resolve.rs
    - darkmatter/lib/src/markdown/schemas/rewrite.rs
    - darkmatter/lib/src/markdown/schemas/scope.rs
    - darkmatter/lib/src/markdown/schemas/simplified/definitions.rs
    - darkmatter/lib/src/markdown/schemas/triggers/assemble.rs
    - darkmatter/lib/src/markdown/schemas/validate.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/recursive_schema_merge.rs
docs_updated_during_phase_3:
    - darkmatter/docs/topics/schemas/definition.md
    - claudine/features/2026-10-29-recursive-schema-types/inventory/contracts.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
    - .claude/skills/darkmatter/schema.md
source_files_during_phase_4:
    - claudine/lib/src/composition/preflight.rs
    - claudine/lib/src/composition/prepare.rs
    - claudine/lib/src/composition/sequence/preflight/mod.rs
    - darkmatter/dmls/src/diagnostics/call_check.rs
    - darkmatter/dmls/src/providers/dsl.rs
    - darkmatter/dmls/tests/l1/main.rs
    - darkmatter/lib/src/lifecycle/md_host.rs
    - darkmatter/lib/src/lifecycle/preflight/shell_walk.rs
    - darkmatter/lib/src/lifecycle/shell_template.rs
    - darkmatter/lib/src/markdown/coercion/engine.rs
    - darkmatter/lib/src/markdown/coercion/error.rs
    - darkmatter/lib/src/markdown/coercion/mod.rs
    - darkmatter/lib/src/markdown/coercion/shape.rs
    - darkmatter/lib/src/markdown/compose/call_precheck.rs
    - darkmatter/lib/src/markdown/compose/context/catalog.rs
    - darkmatter/lib/src/markdown/compose/directive_targets.rs
    - darkmatter/lib/src/markdown/compose/expression/call_binding/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/call_binding/translate.rs
    - darkmatter/lib/src/markdown/compose/expression/call_check.rs
    - darkmatter/lib/src/markdown/compose/expression/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/scope/projection.rs
    - darkmatter/lib/src/markdown/compose/expression/scope/shape.rs
    - darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs
    - darkmatter/lib/src/markdown/compose/inline/shell_expansion.rs
    - darkmatter/lib/src/markdown/compose/pipeline/mod.rs
    - darkmatter/lib/src/markdown/compose/pipeline/phases.rs
    - darkmatter/lib/src/markdown/compose/preflight/collect.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/site.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/template/fill.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/template/typing.rs
    - darkmatter/lib/src/markdown/schemas/catalog.rs
    - darkmatter/lib/src/markdown/schemas/clean.rs
    - darkmatter/lib/src/markdown/schemas/coerce.rs
    - darkmatter/lib/src/markdown/schemas/completion.rs
    - darkmatter/lib/src/markdown/schemas/discriminant.rs
    - darkmatter/lib/src/markdown/schemas/frontmatter_shape.rs
    - darkmatter/lib/src/markdown/schemas/json_shape.rs
    - darkmatter/lib/src/markdown/schemas/mod.rs
    - darkmatter/lib/src/markdown/schemas/rewrite.rs
    - darkmatter/lib/src/markdown/schemas/simplified/mod.rs
    - darkmatter/lib/src/markdown/schemas/simplified/query.rs
    - darkmatter/lib/src/markdown/schemas/simplified/subtype.rs
    - darkmatter/lib/src/markdown/schemas/validate.rs
    - darkmatter/lib/tests/l1/coercion_translator_conformance.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/raw_schema_conversion.rs
    - darkmatter/lib/tests/l1/recursive_schema_types.rs
    - darkmatter/lib/tests/l1/shell_template_policy.rs
    - darkmatter/dmls/tests/l1/recursive_call_check.rs
    - darkmatter/lib/tests/l1/recursive_schema_clean.rs
    - darkmatter/lib/tests/l1/recursive_schema_completion.rs
    - darkmatter/lib/tests/l1/recursive_schema_conversion.rs
    - darkmatter/lib/tests/l1/recursive_schema_reporting.rs
    - darkmatter/lib/tests/l1/recursive_schema_rewrite.rs
    - darkmatter/lib/tests/l1/recursive_schema_typed_consumers.rs
docs_updated_during_phase_4:
    - darkmatter/docs/topics/schemas/definition.md
    - darkmatter/docs/inline/call-check.md
    - darkmatter/docs/inline/preflight-checks.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
    - .claude/skills/darkmatter/schema.md
source_files_during_phase_5:
    - darkmatter/dmls/src/overlay/schema.rs
    - darkmatter/dmls/src/overlay/mod.rs
    - darkmatter/dmls/src/providers/frontmatter.rs
    - darkmatter/dmls/src/providers/frontmatter/sequence_tests.rs
    - darkmatter/dmls/src/providers/dsl.rs
    - darkmatter/dmls/src/providers/code_actions.rs
    - darkmatter/dmls/src/diagnostics/named_types.rs
    - darkmatter/dmls/src/work_counters.rs
    - darkmatter/dmls/tests/l1/main.rs
    - darkmatter/dmls/tests/l1/recursive_schema_editor.rs
    - darkmatter/cli/src/commands/schema/assignment.rs
    - darkmatter/cli/tests/l1/main.rs
    - darkmatter/cli/tests/l1/schema_recursive.rs
    - darkmatter/lib/Cargo.toml
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/recursive_schema_frozen_fixture.rs
    - darkmatter/lib/tests/fixtures/recursive_schema_frozen/action.yaml
    - darkmatter/lib/tests/fixtures/recursive_schema_frozen/lifecycle.yaml
    - darkmatter/lib/tests/fixtures/recursive_schema_frozen/lifecycle-recursive.yaml
    - claudine/features/_completed/2026-10-20-lifecycle-grammar/spike/gen-action-schema.py
docs_updated_during_phase_5:
    - darkmatter/docs/topics/schemas/definition.md
    - darkmatter/docs/topics/schemas/authoring-schemas.md
    - darkmatter/docs/topics/schemas/local-type-references.md
    - darkmatter/docs/topics/schemas/dmls-schema-support.md
    - darkmatter/dmls/docs/autocomplete.md
    - darkmatter/dmls/docs/hover.md
    - claudine/features/_completed/2026-10-20-lifecycle-grammar/spike/README.md
docs_created_during_phase_5:
    - darkmatter/lib/tests/fixtures/recursive_schema_frozen/README.md
skills_files_updated_during_phase_5:
    - .claude/skills/darkmatter/schema.md
    - .claude/skills/darkmatter/SKILL.md
    - .claude/skills/darkmatter/dmls.md
source_files_during_phase_6:
    - darkmatter/cli/src/commands/mod.rs
    - darkmatter/cli/src/commands/schema/validate.rs
    - darkmatter/cli/tests/l1/main.rs
    - darkmatter/cli/tests/l1/schema_validate_timings.rs
    - darkmatter/dmls/src/config/mod.rs
    - darkmatter/dmls/src/diagnostics/frontmatter.rs
    - darkmatter/dmls/src/lib.rs
    - darkmatter/dmls/src/overlay/mod.rs
    - darkmatter/dmls/src/overlay/schema.rs
    - darkmatter/dmls/src/router.rs
    - darkmatter/dmls/src/validation_timing.rs
    - darkmatter/dmls/tests/l1/main.rs
    - darkmatter/dmls/tests/l1/searched_candidates.rs
    - darkmatter/dmls/tests/l1/validation_timing_log.rs
    - darkmatter/justfile
    - darkmatter/lib/benches/schema_validation.rs
    - darkmatter/lib/benches/fixtures/claudine-schemas/action.yaml
    - darkmatter/lib/benches/fixtures/claudine-schemas/claudine.yaml
    - darkmatter/lib/benches/fixtures/claudine-schemas/globals.yaml
    - darkmatter/lib/benches/fixtures/claudine-schemas/inline-compose.yaml
    - darkmatter/lib/benches/fixtures/claudine-schemas/lifecycle.yaml
    - darkmatter/lib/benches/fixtures/claudine-schemas/linking.yaml
    - darkmatter/lib/benches/fixtures/claudine-schemas/review.yaml
    - darkmatter/lib/benches/fixtures/claudine-schemas/sequence-types.yaml
    - darkmatter/lib/benches/fixtures/claudine-schemas/sequence.yaml
    - darkmatter/lib/src/markdown/schemas/coerce.rs
    - darkmatter/lib/src/markdown/schemas/completion.rs
    - darkmatter/lib/src/markdown/schemas/example.rs
    - darkmatter/lib/src/markdown/schemas/json_shape.rs
    - darkmatter/lib/src/markdown/schemas/mod.rs
    - darkmatter/lib/src/markdown/schemas/resolve.rs
    - darkmatter/lib/src/markdown/schemas/scope.rs
    - darkmatter/lib/src/markdown/schemas/simplified/convert.rs
    - darkmatter/lib/src/markdown/schemas/tests/mod.rs
    - darkmatter/lib/src/markdown/schemas/tests/reset_process_caches.rs
    - darkmatter/lib/src/markdown/schemas/timings.rs
    - darkmatter/lib/src/markdown/schemas/triggers/assemble.rs
    - darkmatter/lib/src/markdown/schemas/triggers/discovery.rs
    - darkmatter/lib/src/markdown/schemas/triggers/matcher.rs
    - darkmatter/lib/src/markdown/schemas/validate.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/schema_cache_validity.rs
    - darkmatter/lib/tests/l1/schema_validation_timings.rs
    - darkmatter/lib/tests/l1/trigger_payload_load_errors.rs
docs_updated_during_phase_6:
    - darkmatter/docs/inline/schema-validation.md
    - darkmatter/docs/lsp/configuration.md
    - darkmatter/docs/lsp/indexing.md
    - darkmatter/docs/topics/schemas/definition.md
    - claudine/features/2026-10-30-claudine-recursive-schemas/spec.md
    - claudine/features/2026-10-30-claudine-recursive-schemas/plan.md
    - claudine/features/2026-10-31-type-aware-evaluation/spec.md
    - claudine/features/2026-10-31-type-aware-evaluation/plan.md
docs_created_during_phase_6:
    - darkmatter/lib/benches/fixtures/claudine-schemas/README.md
skills_files_updated_during_phase_6:
    - .claude/skills/darkmatter/dmls.md
    - .claude/skills/darkmatter/schema.md
source_code:
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/recursive_schema_validator_probe.rs
    - darkmatter/lib/tests/l1/recursive_schema_golden.rs
    - darkmatter/lib/tests/fixtures/recursive_schema_golden/manifest.yaml
    - darkmatter/lib/tests/fixtures/recursive_schema_golden/inputs/
    - darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/
    - claudine/gen/src/schema_compat.rs
    - claudine/lib/src/composition/schema/classify.rs
    - darkmatter/cli/src/commands/schema/assignment.rs
    - darkmatter/dmls/src/diagnostics/named_types.rs
    - darkmatter/dmls/src/overlay/mod.rs
    - darkmatter/dmls/src/overlay/named_types.rs
    - darkmatter/dmls/src/overlay/suggestions.rs
    - darkmatter/dmls/src/providers/frontmatter.rs
    - darkmatter/dmls/tests/l1/named_type_references.rs
    - darkmatter/lib/src/markdown/compose/context/catalog.rs
    - darkmatter/lib/src/markdown/compose/expression/call_binding/translate.rs
    - darkmatter/lib/src/markdown/compose/expression/scope/shape.rs
    - darkmatter/lib/src/markdown/schemas/completion.rs
    - darkmatter/lib/src/markdown/schemas/detect.rs
    - darkmatter/lib/src/markdown/schemas/errors.rs
    - darkmatter/lib/src/markdown/schemas/frontmatter_shape.rs
    - darkmatter/lib/src/markdown/schemas/mod.rs
    - darkmatter/lib/src/markdown/schemas/phase.rs
    - darkmatter/lib/src/markdown/schemas/resolve.rs
    - darkmatter/lib/src/markdown/schemas/resolve/graph.rs
    - darkmatter/lib/src/markdown/schemas/resolve/recursive.rs
    - darkmatter/lib/src/markdown/schemas/simplified/convert.rs
    - darkmatter/lib/src/markdown/schemas/simplified/definitions.rs
    - darkmatter/lib/src/markdown/schemas/simplified/lint.rs
    - darkmatter/lib/src/markdown/schemas/simplified/mod.rs
    - darkmatter/lib/src/markdown/schemas/simplified/query.rs
    - darkmatter/lib/src/markdown/schemas/simplified/serialize.rs
    - darkmatter/lib/src/markdown/schemas/simplified/source.rs
    - darkmatter/lib/src/markdown/schemas/simplified/standalone.rs
    - darkmatter/lib/src/markdown/schemas/simplified/subtype.rs
    - darkmatter/lib/src/markdown/schemas/simplified/types.rs
    - darkmatter/lib/src/markdown/schemas/tests/embedded_sources.rs
    - darkmatter/lib/src/markdown/schemas/triggers/matcher.rs
    - darkmatter/lib/tests/l1/recursive_schema_types.rs
    - darkmatter/lib/tests/l1/schemas_named_type_merge.rs
    - darkmatter/lib/tests/l1/schemas_union_postfix.rs
    - darkmatter/lib/tests/l1/suggest_constraint_phase4.rs
    - darkmatter/lib/src/markdown/compose/schema_validation.rs
    - darkmatter/lib/src/markdown/schemas/coerce.rs
    - darkmatter/lib/src/markdown/schemas/rewrite.rs
    - darkmatter/lib/src/markdown/schemas/scope.rs
    - darkmatter/lib/src/markdown/schemas/triggers/assemble.rs
    - darkmatter/lib/src/markdown/schemas/validate.rs
    - darkmatter/lib/tests/l1/recursive_schema_merge.rs
    - claudine/lib/src/composition/preflight.rs
    - claudine/lib/src/composition/prepare.rs
    - claudine/lib/src/composition/sequence/preflight/mod.rs
    - darkmatter/dmls/src/diagnostics/call_check.rs
    - darkmatter/dmls/src/providers/dsl.rs
    - darkmatter/dmls/tests/l1/main.rs
    - darkmatter/lib/src/lifecycle/md_host.rs
    - darkmatter/lib/src/lifecycle/preflight/shell_walk.rs
    - darkmatter/lib/src/lifecycle/shell_template.rs
    - darkmatter/lib/src/markdown/coercion/engine.rs
    - darkmatter/lib/src/markdown/coercion/error.rs
    - darkmatter/lib/src/markdown/coercion/mod.rs
    - darkmatter/lib/src/markdown/coercion/shape.rs
    - darkmatter/lib/src/markdown/compose/call_precheck.rs
    - darkmatter/lib/src/markdown/compose/directive_targets.rs
    - darkmatter/lib/src/markdown/compose/expression/call_binding/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/call_check.rs
    - darkmatter/lib/src/markdown/compose/expression/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/scope/projection.rs
    - darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs
    - darkmatter/lib/src/markdown/compose/inline/shell_expansion.rs
    - darkmatter/lib/src/markdown/compose/pipeline/mod.rs
    - darkmatter/lib/src/markdown/compose/pipeline/phases.rs
    - darkmatter/lib/src/markdown/compose/preflight/collect.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/site.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/template/fill.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/template/typing.rs
    - darkmatter/lib/src/markdown/schemas/catalog.rs
    - darkmatter/lib/src/markdown/schemas/clean.rs
    - darkmatter/lib/src/markdown/schemas/discriminant.rs
    - darkmatter/lib/src/markdown/schemas/json_shape.rs
    - darkmatter/lib/tests/l1/coercion_translator_conformance.rs
    - darkmatter/lib/tests/l1/raw_schema_conversion.rs
    - darkmatter/lib/tests/l1/shell_template_policy.rs
    - darkmatter/dmls/tests/l1/recursive_call_check.rs
    - darkmatter/lib/tests/l1/recursive_schema_clean.rs
    - darkmatter/lib/tests/l1/recursive_schema_completion.rs
    - darkmatter/lib/tests/l1/recursive_schema_conversion.rs
    - darkmatter/lib/tests/l1/recursive_schema_reporting.rs
    - darkmatter/lib/tests/l1/recursive_schema_rewrite.rs
    - darkmatter/lib/tests/l1/recursive_schema_typed_consumers.rs
    - darkmatter/dmls/src/overlay/schema.rs
    - darkmatter/dmls/src/providers/frontmatter/sequence_tests.rs
    - darkmatter/dmls/src/providers/code_actions.rs
    - darkmatter/dmls/src/work_counters.rs
    - darkmatter/dmls/tests/l1/recursive_schema_editor.rs
    - darkmatter/cli/tests/l1/main.rs
    - darkmatter/cli/tests/l1/schema_recursive.rs
    - darkmatter/lib/Cargo.toml
    - darkmatter/lib/tests/l1/recursive_schema_frozen_fixture.rs
    - darkmatter/lib/tests/fixtures/recursive_schema_frozen/action.yaml
    - darkmatter/lib/tests/fixtures/recursive_schema_frozen/lifecycle.yaml
    - darkmatter/lib/tests/fixtures/recursive_schema_frozen/lifecycle-recursive.yaml
    - claudine/features/_completed/2026-10-20-lifecycle-grammar/spike/gen-action-schema.py
    - darkmatter/cli/src/commands/mod.rs
    - darkmatter/cli/src/commands/schema/validate.rs
    - darkmatter/cli/tests/l1/schema_validate_timings.rs
    - darkmatter/dmls/src/config/mod.rs
    - darkmatter/dmls/src/diagnostics/frontmatter.rs
    - darkmatter/dmls/src/lib.rs
    - darkmatter/dmls/src/router.rs
    - darkmatter/dmls/src/validation_timing.rs
    - darkmatter/dmls/tests/l1/searched_candidates.rs
    - darkmatter/dmls/tests/l1/validation_timing_log.rs
    - darkmatter/justfile
    - darkmatter/lib/benches/schema_validation.rs
    - darkmatter/lib/benches/fixtures/claudine-schemas/action.yaml
    - darkmatter/lib/benches/fixtures/claudine-schemas/claudine.yaml
    - darkmatter/lib/benches/fixtures/claudine-schemas/globals.yaml
    - darkmatter/lib/benches/fixtures/claudine-schemas/inline-compose.yaml
    - darkmatter/lib/benches/fixtures/claudine-schemas/lifecycle.yaml
    - darkmatter/lib/benches/fixtures/claudine-schemas/linking.yaml
    - darkmatter/lib/benches/fixtures/claudine-schemas/review.yaml
    - darkmatter/lib/benches/fixtures/claudine-schemas/sequence-types.yaml
    - darkmatter/lib/benches/fixtures/claudine-schemas/sequence.yaml
    - darkmatter/lib/src/markdown/schemas/example.rs
    - darkmatter/lib/src/markdown/schemas/tests/mod.rs
    - darkmatter/lib/src/markdown/schemas/tests/reset_process_caches.rs
    - darkmatter/lib/src/markdown/schemas/timings.rs
    - darkmatter/lib/src/markdown/schemas/triggers/discovery.rs
    - darkmatter/lib/tests/l1/schema_cache_validity.rs
    - darkmatter/lib/tests/l1/schema_validation_timings.rs
    - darkmatter/lib/tests/l1/trigger_payload_load_errors.rs
documentation:
    - claudine/features/2026-10-29-recursive-schema-types/inventory/contracts.md
    - claudine/features/2026-10-29-recursive-schema-types/inventory/consumers.md
    - darkmatter/lib/tests/fixtures/recursive_schema_golden/README.md
    - darkmatter/docs/topics/schemas/definition.md
    - darkmatter/docs/topics/schemas/dmls-schema-support.md
    - darkmatter/docs/inline/call-check.md
    - darkmatter/docs/inline/preflight-checks.md
    - darkmatter/docs/topics/schemas/authoring-schemas.md
    - darkmatter/docs/topics/schemas/local-type-references.md
    - darkmatter/dmls/docs/autocomplete.md
    - darkmatter/dmls/docs/hover.md
    - claudine/features/_completed/2026-10-20-lifecycle-grammar/spike/README.md
    - darkmatter/lib/tests/fixtures/recursive_schema_frozen/README.md
    - darkmatter/docs/inline/schema-validation.md
    - darkmatter/docs/lsp/configuration.md
    - darkmatter/docs/lsp/indexing.md
    - claudine/features/2026-10-30-claudine-recursive-schemas/spec.md
    - claudine/features/2026-10-30-claudine-recursive-schemas/plan.md
    - claudine/features/2026-10-31-type-aware-evaluation/spec.md
    - claudine/features/2026-10-31-type-aware-evaluation/plan.md
    - darkmatter/lib/benches/fixtures/claudine-schemas/README.md
completed_phase: 6
implemented: true
pipeline_refreshed:
    - 2026-10-30-claudine-recursive-schemas
    - 2026-10-31-type-aware-evaluation
decisions_for_author: []
---

# Implementation Log for 2026-10-29-recursive-schema-types (6 phases)

## Phase 1

Phase 1 (Wave 1, "Starting Evidence") started 2026-10-23 on a clean tree at
`e5976e3f0` (schema-enhancements review iteration 1 committed as
`f6161e3f7`).

### Wave 1: Starting Evidence

**Prerequisites verified.**

- `2026-10-14-union-type-constraints` sits in `_completed/` with its
  implementation log, review rounds, and baseline; its behavior is pinned by
  the passing `schemas_union_*` L1 modules.
- `2026-10-28-schema-enhancements` records all six phases in its
  implementation log (Phase 6 closed in `d466b2d45`) and review iteration 1
  was committed as `f6161e3f7` / `e5976e3f0`; its spec frontmatter has
  `implemented: true` and its log `human_review: false`. Its reader matrix
  (`schema_enhancements_matrix::{envelope, raw_schema}`), conversion engine,
  call binder, catalog, and design-time call check are all in the tree.
- The earlier shared-table, YAML-loader, named-presence/local-name, and
  diagnostic-position contracts are present in code: `Arc`-shared namespace
  tables in `resolve.rs` (`Namespace.named_types`), bare local names
  (`TypeExpr::local`), `SchemaReferenceSite`/`SchemaConstraintOrigin` spans in
  `simplified/source.rs`, and `diagnostic_positions` L1 coverage.
- Old cutoffs confirmed in code: `json_shape.rs` `CYCLE_EXPANSIONS = 4`,
  `MAX_REFERENCE_DEPTH = 32`, `REFERENCE_BUDGET = 4096`;
  `call_binding/translate.rs` `MAX_REFERENCE_DEPTH = 32` → `TooDeep`;
  `resolve.rs` `MAX_IMPORT_DEPTH = 32`; signatures `16`. Recorded in
  `inventory/contracts.md` §1 with each one's fate.

**Validator probe.** `darkmatter/lib/tests/l1/recursive_schema_validator_probe.rs`
(6 tests, all pass, ~0.04 s each) pins pinned `jsonschema` 0.55 / Draft 2020-12:
self-reference and mutual recursion accept a depth-8 instance and reject a bad
value at depth 8 with that node's instance path; a required child missing at
depth 8 is reported at that item; a `$ref` sibling constraint rejects what the
target accepts while the target still applies; an escaped pointer key
(`a/b~c` → `a~1b~0c`) resolves; a missing local target fails to compile. Run
once as the prerequisite probe:
`cargo nextest run -p darkmatter --features effects-instrumentation --test l1 -E 'test(recursive_schema_validator_probe)'`.

**Golden compatibility corpus.** `darkmatter/lib/tests/l1/recursive_schema_golden.rs`
plus `darkmatter/lib/tests/fixtures/recursive_schema_golden/` (`manifest.yaml`,
`README.md`, `inputs/`, `golden/`), built by a subagent and reviewed here:

- 104 manifest entries: schema-file 23, pure-schema-document 24, inline-root
  39, root-union 5, trigger-payload 6, embedded-baseline 1, effective 6
  (baseline merge: `effective-baseline-{inline,schema-file,root-union}`;
  trigger merge: `effective-trigger-{inline-compose,sequence,with-document-schema}`
  through `DarkmatterSchemas::effective_for` with the frozen Claudine schemas
  as an embedded bundle, discovery in an empty tempdir).
- 112 input files: 92 byte-identical copies of the files at `e5976e3f0`
  (mirroring repository paths, so relative imports still resolve) and 20
  authored under `inputs/authored/` (bare local names / `@this`, union lists,
  effective cases), each with a `note:`; `union_fixtures` are Rust constants,
  and no shipped inline root used bare local names. 9 `imports_only` paths
  each name the entry that must read them.
- 104 goldens: 90 full pretty JSON, 14 compact-text digests.
- Skipped inputs are listed in the manifest with reasons: function catalogs,
  `kind: example` artifacts, empty placeholders, drafts whose definitions do
  not parse, `loop.yaml` (`typed:`), `_fleet_future.md` (unknown type
  `crate`), two messenger probes ("could not project expression spans"),
  `triggers/feature-review-incident.yaml` (invalid YAML at line 17), and
  string `$schema`s whose target is already an entry.
- No lowered output named an absolute path; the test fails if the fixture or
  scratch directory ever appears.
- Self-checks: every manifest input exists, every input file is an entry or a
  declared import, every golden belongs to an entry, ids are unique, each
  entry reads only frozen inputs, and five entries resolve twice to identical
  bytes. The fixture-directory literal sits inside each `#[test]`.
- 17 tests. Seven entries that lower to megabytes have a test each
  (`GROUP_TESTS`, guarded by `every_manifest_kind_and_group_has_a_test`). The
  slowest, `schema_file_claudine_sequence_types_matches_its_golden` (44 MB of
  output), runs in 1.9 s alone at host load ~12–15; about 1.1 s is the
  library's own lowering.
- Regeneration: `DARKMATTER_BLESS_RECURSIVE_GOLDEN=1` (README); legitimate
  only for a change meant to alter acyclic lowering.

**Consumer inventory.** `claudine/features/2026-10-29-recursive-schema-types/inventory/consumers.md`
(subagent, read-only): 53 `TypeExpr` matches (31 exhaustive, 22 with wildcard
or catch-all arms that would silently absorb `Ref`, notably `union_arms` and
`admits_null` in `types.rs`) and 49 partial destructures; definition storage
and namespaces; lowering and resolve callers; merge boundaries; 14 synthetic
validator sites; JSON walkers (only `json_shape.rs` and
`schema_declares_property` follow `$ref` today); typed-tree walkers; cycle and
depth errors; downstream Claudine consumers; test targets per area; and file
ownership by wave. It found that `merge_baseline` and DMLS extension baselines
drop every baseline root key but `type`/`$schema`/`additionalProperties`, and
that `ImportCycle`'s doc and status text still describe a DAG; both are now in
the contract.

**Reference API contracts.** Written to
`claudine/features/2026-10-29-recursive-schema-types/inventory/contracts.md`:
`DefKey`/`DefNamespace`, `Definition`/`DefinitionTable` ownership, key
allocation and re-keying, graph edge classification and admission order, new
error variants, use-site semantics, raw merge-ingress rules and the duplicate
ruling, `SchemaScope` and synthetic-validator scope, phase memoization,
coinductive type comparison, and the robustness-matrix confirmation.

**Duplicate-member ruling.** Every raw reader feeding the new merge ingress
already rejects duplicate members (raw `.json` through
`coercion::content_error`, schema-enhancements R3; both YAML loaders), and an
in-memory `Value` cannot carry one. Rejection therefore tightens no accepted
schema; no author decision is needed.

### Design Decisions

1. **Golden inputs are frozen copies; the embedded baseline is live.**
   - Decision: the compatibility corpus copies each file input byte-for-byte
     under `darkmatter/lib/tests/fixtures/recursive_schema_golden/inputs/`
     (mirroring repository paths) and resolves the copies; only the compiled-in
     baseline and `@darkmatter:` partials are read live.
   - Alternatives considered: resolve the live shipped files listed in a
     manifest; store only a digest of the live files.
   - Why: the golden must move only when resolver code changes. Live inputs
     would fail on every unrelated schema edit (Claudine's lifecycle schemas
     are scheduled to change in `2026-10-30-claudine-recursive-schemas`), and
     CI's test-input index would schedule this test on each such edit, turning
     the golden into a rubber stamp.
   - Downsides: the corpus does not follow later edits to shipped schemas; the
     separate `shipped_schema_corpus` test keeps covering the live files.
2. **Large goldens are stored as a `biscuit-hash` BLAKE3 digest plus length.**
   - Decision: a schema whose compact serialization exceeds 128 KiB is stored
     as the `bytes:` + `blake3:` of that compact text; smaller ones as full
     pretty JSON. (The first cut digested pretty text over 256 KiB; pretty-
     printing and hashing the 44 MB `sequence-types.yaml` output cost about
     1 s of a debug test, so digests now cover the compact form, which keeps
     key order in the comparison.)
   - Alternatives considered: commit every golden in full; digest everything.
   - Why: byte-identity is what the criterion needs, and a digest proves it
     without committing multi-megabyte JSON; full text keeps small failures
     diffable.
   - Downsides: a mismatch on a digest golden shows no diff in the test
     output; the test writes the actual output to a temp file for inspection.
3. **`DefNamespace` replaces the shared `Root` sentinel; `SchemaSourceId`
   stays narrow.**
   - Decision: inline roots, each root-union arm, embedded documents, shipped
     partials, files, and raw `$defs` tables get distinct namespace identities
     (`contracts.md` §2); span sources are unchanged.
   - Alternatives considered: widen `SchemaSourceId`; key inline namespaces by
     document text.
   - Why: the spec requires distinct identities; widening the public span
     type would ripple through every diagnostic consumer without need.
   - Downsides: two identity types to keep in step.
4. **Generated keys are the type name with a numeric suffix on collision.**
   - Decision: `item`, then `item-2`, … in traversal order; never paths or
     hashes; consumers use the explicit identity/key map.
   - Alternatives considered: purely opaque `d0`, `d1`, …; content hashes.
   - Why: deterministic and path-free as the spec requires, and validator
     schema paths (`/$defs/item/...`) stay readable in diagnostics.
   - Downsides: a key looks like a name; the contract forbids parsing it.
5. **Only cyclic definitions enter the table.**
   - Decision: SCC members (including self-loops) become `Ref`s; everything
     else inlines exactly as today.
   - Alternatives considered: emit `$defs` for every named type.
   - Why: the spec requires byte-identical acyclic output, and an empty table
     emits nothing.
   - Downsides: output size for acyclic but heavily reused types is unchanged
     (the spec accepts this).
6. **Definition bodies are presence-free; markers are inherited per use.**
   - Decision: required/generated/eager/description/default/suggest move from
     a definition's top-level atom to each use site; value constraints stay in
     the body; differing use-site value constraints lower as `$ref` siblings.
   - Alternatives considered: one body per distinct constraint set.
   - Why: the spec's use-site semantics, and the probe shows siblings apply
     together with the target in the pinned validator.
   - Downsides: lowering must merge inherited and use-site constraints at
     every use, not once per definition.
7. **Fixture names in helpers are `concat!` constants.**
   - Decision: `load_manifest`/`check_group` join `MANIFEST_FILE`,
     `INPUTS_DIR`, and `GOLDEN_DIR` (built with `concat!`); the only path
     literal is the fixture directory inside each `#[test]`.
   - Alternatives considered: keep `fixtures.join("manifest.yaml")` in the
     helper; pass every path in from each test.
   - Why: CI's test-input scanner attributed the helper literals to the whole
     `darkmatter::l1` binary (checked with
     `affected_scope.test_input_references`); after the change every golden
     input maps to the single test that reads it.
   - Downsides: a reader must know why the names are spelled with `concat!`;
     the constant's doc comment says so.

### Requirement-to-test mapping (Phase 1)

| Phase 1 requirement | Evidence |
| --- | --- |
| Prerequisites verified, old cutoffs recorded | this log (Wave 1) and `inventory/contracts.md` §1 |
| Pinned validator: self recursion at depth 8, valid and invalid with path | `recursive_schema_validator_probe::{a_self_referencing_definition_validates_depth_eight, a_required_child_missing_at_depth_eight_is_reported_there}` |
| Mutual recursion | `…::mutually_recursive_definitions_validate_depth_eight` |
| `$ref` sibling constraint visibly rejects | `…::a_constraint_beside_a_reference_applies_with_its_target` |
| Escaped JSON Pointer key; missing target is not unconstrained | `…::{an_escaped_pointer_key_resolves, a_reference_to_a_missing_definition_does_not_compile}` |
| Byte-identical golden: shipped schema/trigger artifacts, pure `$schema` documents, embedded baseline, resolver fixtures, inline roots, root-union references, effective merges | `recursive_schema_golden::*_matches_their_goldens` / `*_matches_its_golden` (13 tests by kind/group) |
| Manifest is explicit, complete, and drift-proof | `recursive_schema_golden::{manifest_accounts_for_every_fixture_file, every_manifest_kind_and_group_has_a_test}` |
| Lowering is deterministic | `recursive_schema_golden::repeated_resolution_is_byte_identical` |
| Consumer inventory, file ownership, test targets | `inventory/consumers.md` |
| Shared reference API contracts | `inventory/contracts.md` |
| Duplicate-member ruling | this log; existing `schema_enhancements_matrix::raw_schema::raw_json_schema_keyword_matrix` duplicate rows |

The robustness matrix's new cells (recursive bodies, merge-ingress raw
references) are built and tested in Waves 2 and 3, as the plan assigns; Phase 1
changes no reader, so no matrix cell is implemented here
(`contracts.md` §12 records each table's state).

### Gates

- Scope: `git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`
  → `packages: ["darkmatter", "test-toolkit", "claudine"]`,
  `change_class: "package"`. Claudine is a narrowed `claudine::l1` cell
  because `claudine/lib/tests/l1/boundary_lint.rs:150` (`read_darkmatter_source`)
  joins `../../darkmatter/lib` in a helper, so every non-source file under
  `darkmatter/lib` maps to the whole binary; the helper reads only named
  source files, so the fixtures cannot affect it (pre-existing, left alone).
  `test-toolkit` is its archive-path guard lint cell.
- `darkmatter`: `just test` ran twice. Both runs: every darkmatter lib and CLI
  L1 test passed (10,374, confirmed by diffing `nextest list` against the run
  log); one failure, `dmls::l1 registry_preparation::a_held_preparation_leaves_edits_cancellation_and_other_requests_running`,
  stopped the run (fail-fast). **Load-sensitive, pre-existing, unrelated**:
  it passes alone 3/3; under load (host load average 30–39 from other
  sessions) request 502 took 765 ms, so DMLS logged it as `slow request`
  rather than `request complete`, and the test's log filter only matches the
  latter. The remaining DMLS and Zed tests were then run with only that test
  excluded: `cargo nextest run -p dmls --features effects-instrumentation -p zed-dmls-cli -E "(L1 filter) & !test(=…)"`
  → 1,351 passed.
- `darkmatter`: `just lint` passed; `just check-tier-coverage darkmatter`
  passed (0 stranded); clippy `-D warnings` on the test target passed.
- `claudine`: the whole `claudine::l1` binary (the CI cell) → 399 passed.
- `test-toolkit`: `just archive-path-guard` → 2 passed.
- Dependent packages: Phase 1 changes no library code or public symbol (only
  darkmatter test files and fixtures), so no downstream package test can be
  affected beyond the cells above.
- Cross-OS: `just cross-check darkmatter --os windows recursive_schema_` →
  23 passed on native Windows (goldens byte-identical there; the canonical
  path comparisons hold).

## Phase 2

Phase 2 (Wave 2, "Reference Graph") started 2026-10-23 on the tree at
`689147156` (Phase 1 committed). Implemented directly, without subagents:
the wave is sequential and every task edits the same resolver.

### Wave 2: Reference Graph

**Definition storage.** New `darkmatter/lib/src/markdown/schemas/simplified/definitions.rs`:
`DefKey { namespace, name }`, `DefNamespace` (`InlineRoot`, `RootUnionArm(i)`,
`File(path)`, `FileRootUnionArm(path, i)`, `Embedded`, `Shipped`, replacing
the private `NamespaceKey` and its shared `Root` sentinel), `Definition {
body, origin, json_key }`, `DefinitionTable` (allocation-ordered entries plus
an explicit `json_key → DefKey` map; `outer()` reports a body's top-level
kind through alias bodies), `DefinitionOuter`, and `definition_pointer`
(RFC 6901 escaping). `TypeExpr::Ref(DefKey)` was added to
`simplified/types.rs`. `ResolvedSchema.definitions: Arc<DefinitionTable>`,
`EffectiveSchema::definitions()`, and `expand_named_types` →
`NamedTypeExpansion { schema, imports, definitions }` carry the table. Bodies
refer to each other by `DefKey`, never `Arc`.

**Graph discovery and admission** (`schemas/resolve/graph.rs`). Before
expansion the engine walks every reachable named type once (worklist over
`(namespace, name)`), records an edge per reference with `descends` set for an
object property, dictionary value, array item (`is_array`), or tuple slot,
and never inside a `function(...)` signature. Iterative Tarjan finds the
groups; members of a group with a cycle (more than one member or a self-edge)
become the cyclic set. The same SCC pass over non-descending edges alone finds
the rejected cycles, reported as `SchemaError::NonDescendingCycle { cycles }`
with every reference of each rejected cycle (in discovery order) and, for a
schema file, the reference name's span from that file's source map. A
reference that does not resolve adds no edge (expansion reports it in its old
order); a reference inside a function signature is resolved with the
dependency and probe sets restored, so expansion's dependency list is
unchanged. Because a cyclic definition is never pushed on the import stack,
a back-edge can no longer reach the `MAX_IMPORT_DEPTH` check; acyclic chains
keep `ImportCycle` "exceeded 32 levels".

**Two-pass expansion** (`schemas/resolve/recursive.rs`). Without a cycle the
engine runs exactly as before (the golden corpus stayed byte-identical without
a bless). With one, every use of a cyclic definition becomes a `Ref` and the
engine expands twice: pass 1 builds use sites from the use-site postfix alone
and expands each reached body from a FIFO worklist; a `UseSummary` (outer
kind, first list arm of a union, inherited markers with origins, description,
the definition's own count bounds, null admission) is read off those bodies,
recursing only through top-level references; pass 2 rebuilds every use site
from the summary under the inlined-reference rules
(`apply_reference_postfix`) and the published body has its use-site markers
stripped. `apply_postfix` routes an acyclic alias of a cyclic definition
(`tree: node`) through the same rules, so `[]` over a list definition and
over a union with a list arm keep their existing errors, resolved from the
outer shape without recursive expansion.

**Lowering** (`simplified/convert.rs`). `to_json_schema_with_definitions`
threads the table through the converter; a `Ref` lowers to `{"$ref":
"#/$defs/<key>", …use-site siblings}` (key bounds beside an object, item
bounds and `unique` beside an array, none beside a union or tuple), and every
reachable body is lowered once under a root `$defs` after the other root
keys, in allocation order. A body is lowered under the property path of its
first use, so a function type reached only through a definition fails as
`Convert { property: "root.f" }`. An empty table emits no `$defs`.
`to_json_schema` is the empty-table case. Root unions allocate every inline
arm into one table and hoist the arms' `$defs` to the union root.

**Consumers kept compiling.** Every exhaustive `TypeExpr` match in
`darkmatter`, `darkmatter-cli`, `dmls`, `claudine`, and `claudine-gen` gained
a deliberate `Ref` arm: back-reference by name where a type is displayed
(`serialize_property_atom`, DMLS hover lines, Claudine `type_label_for_atom`,
`claudine-gen` `describe_atom`), "not completable / no primitive / no
families" where a typed walker cannot follow it without a table (later waves
add following), identity by `DefKey` in `subtype::identity`, pass-through in
`phase.rs`, and a typed error in the grammar translator (Wave 4 replaces it).
DMLS `diagnostics/named_types.rs` projects `NonDescendingCycle` as a
schema-definition diagnostic ranged on the first cycle reference this
document wrote, the others as related information.

**Existing tests whose expectations changed (intended behavior change).**
Alias cycles were `ImportCycle`; they are now `NonDescendingCycle`:
`resolve::schema_plus_phase1::import_cycle_is_detected`,
`schemas::tests::embedded_sources::malformed_imports_cycles_and_examples_fail_preparation`,
`schemas_named_type_merge::a_cycle_through_bare_and_explicit_references_reports_the_chain`,
`suggest_constraint_phase4::import_cycles_remain_bounded_across_envelope_forms`
(wrapped in `ReferencedSchema`, like the other named-type errors), and DMLS
`named_type_references::a_standalone_contradiction_and_a_mixed_cycle_are_schema_definition_errors`
(now also asserts the range and related location).
`schemas_union_postfix::cycles_and_depth_limits_still_apply_through_union_postfixes`:
`a: [b(required), number]` + `b: a[]` descends through list items, so it is
admitted, and `[]` over a union with a list arm is refused as before
(`UnionReference` `ArrayArm { arm: 0 }`); its depth-limit half is unchanged.

**Docs.** Two passages of `darkmatter/docs/topics/schemas/definition.md`
described the removed DAG rule and were corrected to current behavior (with
the not-yet-built cases marked **planned**); the DMLS diagnostic page
describes the cycle diagnostic. The full documentation rewrite remains Wave 7.

### Design Decisions

1. **Use-site summaries come from a first expansion pass.**
   - Decision: a schema with a cycle expands twice; the first pass yields the
     bodies a `UseSummary` is read from, the second builds every use site
     from its definition's summary.
   - Alternatives considered: compute summaries from unexpanded definitions
     (duplicates the union-flattening and postfix rules); patch use sites
     after one pass (loses each use site's provenance once an acyclic
     expansion is cloned into many places); keep markers in the body and
     merge them at lowering (every typed consumer would need the table to
     answer `is_required`).
   - Why: the summary of a definition depends only on its top level, but that
     top level may be a reference, a list of one, or a union with referenced
     arms; reusing the real expansion keeps one implementation of those
     rules, and use sites stay self-contained (`PropertyDef::is_required`
     needs no table), as `contracts.md` §7 requires.
   - Downsides: a recursive schema's expansion costs about twice as much;
     acyclic schemas take one pass and are unaffected.
2. **`DefNamespace::FileRootUnionArm(path, index)` was added.**
   - Decision: inline arms of a root-union schema file are separate
     namespaces (`contracts.md` §2 updated).
   - Alternatives considered: keep `File(path)` for every arm (two arms that
     each declare `item` would share one `DefKey` in the union's table).
   - Why: the spec requires distinct identities for root-union arms.
   - Downsides: none found.
3. **A file-reference root-union arm with definitions is refused for now.**
   - Decision: `resolve_root_union`/`resolve_standalone_root_union` return
     `SchemaError::Convert` when an arm that names a schema file resolved
     with a non-empty table.
   - Alternatives considered: hoist its `$defs` without re-keying (a key
     collision would bind references to the wrong body).
   - Why: combining independently allocated tables is Wave 3's re-keying
     task ("Isolate Merge Definitions", root-union arms); refusing is the
     only safe interim result, and no such schema existed before.
   - Downsides: those schemas fail until Wave 3.
4. **References inside a `function(...)` signature never descend and never
   fail discovery.**
   - Decision: everything inside a signature is a non-descending position;
     an unresolvable reference there is skipped, and resolving one leaves the
     dependency and probe sets as they were.
   - Alternatives considered: treat a signature's object parameters as
     descending (they describe a parameter, not a child of the instance);
     resolve signatures with full side effects (adds dependencies the
     lowering never reads and could raise errors expansion never raised).
   - Why: the spec lists callable signatures as non-descending; expansion
     never reads signatures, so discovery must not change their outcome.
   - Downsides: editing a file reached only through a signature does not
     invalidate a cache; such a file cannot affect the lowered schema except
     by creating a rejected cycle.
5. **A nullable definition is never required at a use site.**
   - Decision: when the definition admits null (a union with `null` or
     `unknown`), `required` is dropped from every use's atom.
   - Alternatives considered: give `Ref` atoms table-aware `admits_null`.
   - Why: keeps the core-type rule (a nullable union is never required)
     without making `PropertyAtom` methods need the table.
   - Downsides: `admits_null()` on a `Ref` atom still answers `false`; the
     optional wrapper `anyOf[null, $ref]` is redundant but correct.
6. **A union definition's arms keep their arm-local annotations.**
   - Decision: only `required`, `generated`, and `default` are hoisted off a
     union body's alternatives; a file arm's `eager`, its description, and
     examples stay in the body. A `Ref` property gets no optional
     empty-string `file` arm inside the shared body.
   - Alternatives considered: hoist everything (an `eager` would turn a file
     arm's format at every use).
   - Why: matches what a property-level union hoists today.
   - Downsides: an optional property typed by a recursive union with a
     `file` arm does not accept the legacy `""` sentinel.
7. **Use-site bounds are checked against the definition's own bounds, not
   merged into the body.**
   - Decision: a use's key or item bounds become `$ref` siblings; they are
     merged with the definition's top-level bounds only to report a
     contradiction with both origins (definition first).
   - Alternatives considered: emit one body per constraint set.
   - Why: the spec forbids mutating a shared definition, and Draft 2020-12
     applies siblings with the target (pinned by the Phase 1 probe).
   - Downsides: a looser use-site bound is emitted although the body's
     stricter one decides.

### Requirement-to-test mapping (Phase 2)

All in `darkmatter/lib/tests/l1/recursive_schema_types.rs` unless noted.

| Requirement | Test |
| --- | --- |
| Direct recursion validates depth 8; bad leaf reports JSON path + line/column through `DarkmatterSchemas::validate` | `a_self_recursive_type_validates_depth_eight_and_positions_a_bad_leaf` |
| Two-file cycle validates depth 6, rejects with the deep path; both files are dependency edges | `a_two_file_cycle_validates_depth_six_and_records_both_files` |
| Same-file mutual recursion, one body each | `mutual_recursion_in_one_file_stores_each_definition_once` |
| All four descending categories accepted | `every_descending_position_closes_a_cycle` |
| Alias cycle / self-union / callable signature rejected naming references | `an_alias_cycle_is_rejected_naming_each_reference`, `a_self_union_is_rejected_even_though_another_arm_accepts_values`, `a_cycle_through_a_function_signature_is_rejected` |
| Mixed group reports only the non-descending cycle; removing it resolves | `a_mixed_group_reports_only_its_non_descending_cycle` |
| Two-file non-descending cycle has spans in both files | `a_two_file_non_descending_cycle_reports_spans_in_both_files` |
| Unknown names and the acyclic chain limit keep their errors | `unknown_names_and_long_acyclic_chains_keep_their_errors`; `schemas_union_postfix::cycles_and_depth_limits_still_apply_through_union_postfixes` |
| Acyclic alias into a cycle inlines to a `Ref` | `an_acyclic_alias_into_a_cycle_inlines_to_a_reference` |
| Repeated imports share one definition; deterministic, path-free keys | `repeated_imports_share_one_definition_and_resolution_is_deterministic` |
| Namespace isolation (inline vs file, root-union arms), `$defs` at the union root | `same_named_types_from_different_namespaces_get_distinct_keys`, `root_union_arms_are_separate_namespaces_with_definitions_at_the_union_root` |
| Embedded self/mutual recursion with no filesystem | `resolve::embedded_recursion::self_and_mutual_recursion_resolve_from_embedded_documents` (unit) |
| Stable keys and JSON Pointer escaping | `simplified::definitions::tests::*` (unit); Phase 1 probe `an_escaped_pointer_key_resolves` |
| Acyclic lowering byte-identical | `recursive_schema_golden::*` (unchanged, not blessed); `an_acyclic_schema_lowers_with_no_definitions` |
| Inherited required/generated/eager, descriptions, defaults moved to uses; body presence-free | `definition_presence_and_annotations_move_to_each_use` |
| Nullable unions override required | `a_nullable_union_definition_is_never_required` |
| Two differently constrained uses, one unchanged body; array bounds | `two_differently_constrained_uses_share_one_unchanged_body`, `a_list_definition_counts_use_site_bounds_as_items` |
| Conflicting constraints name definition and use site | `contradictory_use_site_bounds_name_the_definition_and_the_use` |
| `suggest` on a recursive reference rejected | `a_suggestion_on_a_recursive_reference_is_rejected` |
| Function type reached only through a definition is a load error at the use-site path | `a_function_reached_only_through_a_definition_fails_at_the_use_site` |
| Required children with `[]` leaf; missing/null at depth 4; optional missing/null; null list items | `required_children_accept_an_empty_leaf_and_reject_missing_or_null_at_depth_four` |
| Input matrix, referenced `types.item` column (control recursive; absent, null, wrong type, one/every wrong element, empty map/list/string, duplicate key, trailing document) | `a_referenced_definition_takes_every_declaration_shape_from_a_recursive_control` |
| DMLS projects the cycle at its references | `dmls::l1 named_type_references::a_standalone_contradiction_and_a_mixed_cycle_are_schema_definition_errors` |

Matrix smell check over the touched readers: no `#[serde(default)]`,
absence-collapsing `Option`, `filter_map(.. as_str())`, `unwrap_or_default()`
on a load-bearing parse, or `.ok()` parse was added; the two `.ok()`s in
`graph.rs::locate_reference_spans` sit on the error path's span lookup, where
an unreadable file leaves the span `None` as `locate_origin_spans` does.

### Gates

- Scope: `git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`
  → `packages: ["claudine", "claudine-gen", "darkmatter", "darkmatter-cli", "dmls", "test-toolkit"]`,
  `change_class: "package"`, so no gate was skipped.
- `darkmatter`: `just lint` passed. `just test`: the first run stopped
  (fail-fast) on `embedded_sources::malformed_imports_cycles_and_examples_fail_preparation`,
  an alias-cycle expectation updated above; the second run passed every
  darkmatter library and CLI L1 test and stopped on
  `dmls::l1 searched_candidates::a_watched_nearer_candidate_reselects_the_import_and_its_removal_falls_back`
  (watch-registration count 3 vs 2). It passes alone 3/3 and touches no
  recursive schema: **load-sensitive, pre-existing**. The remaining DMLS and
  Zed L1 tests were run with only that test excluded → 1,347 passed.
  `just check-tier-coverage darkmatter` → 0 stranded.
- `claudine`: `just test` → 10,692 passed (a first run, overlapping the
  Windows cross-check's sync, stopped on
  `composition::lifecycle::prewarm::worker::tests::a_successful_or_no_op_preparation_counts_as_done`,
  a TTS pre-warm timing test that passes alone 3/3: load-sensitive and
  unrelated; the rerun with nothing else running passed in full).
  `just lint` passed. `just check-tier-coverage claudine` → 0 stranded.
- `claudine-gen`: `cargo nextest run -p claudine-gen` → 250 passed.
- `test-toolkit`: `just archive-path-guard` → 2 passed.
- Dependent packages: `biscuit-speaks-cli`, `messenger`, `messenger-cli`,
  `sniff-cli`, `research`, `biscuit-icon-cli`, and `playa-cli` depend on
  darkmatter but match no `TypeExpr`, build no `ResolvedSchema`, and call no
  `expand_named_types`; `cargo check --tests` on all of them passed. Their
  schema behavior changes only for schemas that recurse, which previously
  failed to resolve.
- Cross-OS: `just cross-check --os windows darkmatter recursive_schema_ embedded_recursion import_cycle cycles_and_depth_limits a_cycle_through_bare definitions::tests`
  → 54 passed on native Windows (canonical namespace paths and file spans
  hold there).

## Phase 3

Phase 3 (Wave 3, "Shared Runtime Context") started 2026-10-23 on the tree at
`689147156` with Phase 2's changes still uncommitted in the working tree
(Phase 2 was not committed before this run; the files Phase 3 touched are
listed in its own frontmatter key). Implemented directly, without subagents:
the wave is sequential and its three tasks share `resolve.rs`, `mod.rs`, and
the new scope module.

### Wave 3: Shared Runtime Context

**Reference lookup** (new `darkmatter/lib/src/markdown/schemas/scope.rs`).
`SchemaScope` resolves a local `$ref` (`#`, `#/…`, the empty reference, with
`%XX` and `~0`/`~1` decoding) against its own root; `resolve` is one hop
(`Ok(None)` for no `$ref` or a non-local one, which only the validator can
follow) and `chain` follows hops, failing with
`SchemaError::UnresolvedReference` on a missing target or a reference-only
loop (`a → b → a`) instead of treating it as unconstrained.
`with_root_definitions(sub, root)` gives a sub-schema compiled alone its
root's `$defs`. `check_joined_references` is the admission rule at every
merge boundary, and `fold_definitions` joins one schema's `$defs` to another
root, renaming only a colliding different body and only the joining side's
references, in schema keywords only.

**Synthetic validators** now compile with their root's definitions:
`wrap_arm_as_root_schema(arm, root)` (used by `CompiledValidators`,
`coerce::accepts`, `rewrite_root_union`, `root_schema_arm_applies`),
`narrow_discriminated_union_problems`, `rewrite_property_union` (the rewrite
walkers carry `root`), `select_file_mode` (the caller file-mode classifier in
`compose/schema_validation.rs` carries `root`), `schema_declares_property`
(follows a `chain`), and `example(...)` targets (`resolve_document_examples`
passes the root `$defs`; inline union arms now resolve examples before their
definitions are hoisted). Sites left alone, each with its reason, are listed
in `inventory/contracts.md` §9 "As built (Wave 3)".

**Merge isolation.** `validate_simple_object_schema(schema, label)` admits a
root `$defs` (root `$ref`, unions, and applicators stay rejected), checks
references, and meta-validates a schema that carries `$defs`.
`merge_baseline` moves a baseline's `$defs` into the document's root (a root
union's root, never an arm) and merges properties through the new
`merge_layer`; it refuses to move them into a root union whose arm declares
its own `$id`. Trigger payloads go through the same contract
(`enforce_merge_compatible`, reported as `TriggerPayloadNotMergeable`) and
the same merge. Root-union file arms: `reject_arm_definitions` is gone;
`join_file_arm_definitions` absorbs a SimplifiedSchema arm's typed table into
the union's (`DefinitionTable::absorb`) and renames its JSON references to
match, and folds raw arms (and file arms that are themselves unions) after
every arm. `ResolvedPayload`/`TriggerLayer` carry their typed tables and
`EffectiveSchema::definition(&DefKey)` looks a reference up across the
document's and the trigger payloads' tables.

**Phases.** `phase::project_definitions` projects every definition body under
the same keys (`DefinitionTable::map_bodies`); `validate_for_phase` lowers the
projected document against it and the projected `EffectiveSchema` carries it.
`merge_with_effective` replaces the document's own bodies in the effective
`$defs` with the projection's and keeps the layer-only ones.

**Docs and skill.** `darkmatter/docs/topics/schemas/definition.md`: the
recursion paragraph no longer calls file-arm, trigger, and baseline recursion
planned; "JSON Schema Baseline Restrictions" and "Merge Semantics" describe
`$defs` admission, the reference rules, and re-keying; the phase section
describes timing below a recursion. `.claude/skills/darkmatter/schema.md`
gained a "`$defs` across merges" entry. `inventory/contracts.md` records the
as-built API (sections 6, 8, 9, 10).

### Design Decisions

1. **The merge error fields name the schema with a label, not a path.**
   - Decision: `UnresolvedReference { schema, reference, location }` and
     `UnsupportedMergeReference { schema, location, reason }`, `schema` being
     a label (`the baseline`, `the payload \`x.yaml\``, `the root-union arm
     \`./a.json\``).
   - Alternatives considered: `path: PathBuf` as the contract drafted (an
     in-memory baseline has no path); `source: String` (`thiserror` treats a
     field named `source` as the error source).
   - Why: every ingress needs a name, only some have a path.
   - Downsides: a consumer cannot open the file from the error alone; inside
     a trigger the payload path is on the wrapping `TriggerPayloadNotMergeable`.
2. **A JSON-level key is shared only for an equal body.**
   - Decision: `fold_definitions` shares a colliding key when both bodies are
     byte-equal and every key they reference is shared too (a fixpoint);
     otherwise the joining side's key is renamed.
   - Alternatives considered: share by `DefKey` (a raw schema has none, and
     `merge_baseline` sees only JSON); always rename (the same baseline merged
     into each arm of a root union would add one copy per arm).
   - Why: two byte-equal bodies with equal references validate identically,
     so sharing can never bind a reference to a different meaning.
   - Downsides: two different `DefKey`s with equal bodies share one key in
     the merged JSON; nothing reads identity from a JSON key (contracts §4).
3. **`#/$defs/…` references are checked at every merge, with or without a
   table; other forms only when the schema carries `$defs`.**
   - Decision: as stated; a baseline without `$defs` keeps `#/properties/…`
     and similar references as before.
   - Alternatives considered: check nothing without `$defs` (contracts §8 as
     drafted: a missing `#/$defs/x` would then bind to the document's `x`);
     check every form always (tightens previously accepted baselines).
   - Why: the matrix requires "`$defs` absent → Unresolved", and the spec
     forbids a reference binding to the assembled root; only a `#/$defs/…`
     reference can newly bind to another layer, since no layer had `$defs`
     before.
   - Downsides: a raw document schema with its own `$defs` merged with a
     baseline that relied on them is now refused; no such pairing could be
     written with an intent other than the bug the rule prevents.
4. **Meta-validation runs only for a merge schema that carries `$defs`.**
   - Decision: `jsonschema::draft202012::meta::validate` on the incoming
     schema when it has a root `$defs`.
   - Alternatives considered: always (an invalid baseline property the
     document overrides was accepted before); never (the structural matrix
     cells would surface only when the merged validator compiles, without
     the baseline's location).
   - Why: matches the plan's "Existing accepted acyclic property schemas must
     not become more restrictive" while giving the new ingress a located
     structural error.
   - Downsides: one meta-validation per merge of a schema with `$defs`
     (merges of trigger layers are memoized).
5. **A typed file arm joins a root union through its typed table.**
   - Decision: `DefinitionTable::absorb` allocates the arm's definitions in
     the union's table, and the arm's JSON is renamed with the returned map.
   - Alternatives considered: fold JSON only (the union's typed table, which
     phase projection lowers the arms against, would then disagree with the
     JSON keys and bind a projected reference to another arm's body).
   - Why: `file_arms_with_same_named_definitions_are_rekeyed_into_the_union_root`
     validates both phases through those keys.
   - Downsides: raw arms fold afterwards, so a raw arm's key can depend on
     the typed keys around it; only key names change.
6. **`EffectiveSchema::definitions()` stays the document's table; a lookup
   across layers is `definition(&DefKey)`.**
   - Decision: trigger tables ride `TriggerLayer` and are searched by
     identity after the document's.
   - Alternatives considered: one merged table (a trigger entry's `json_key`
     would not match the key the JSON merge gave it, inviting a wrong
     `key_for_json` lookup).
   - Why: `DefKey` is unique across layers, so one identity lookup serves the
     merged shape; the document table's keys equal the effective JSON's.
   - Downsides: a consumer wanting every layer's table must use
     `definition`, not iterate `definitions()`.
7. **`DefNamespace::Raw` was not added.**
   - Decision: raw `$defs` exist only as JSON keys.
   - Alternatives considered: the contract's `Raw(RawScope)` variant.
   - Why: nothing produces a typed body for a raw definition; an unused
     variant would add a match arm everywhere for no reader.
   - Downsides: a later typed consumer of raw definitions must add it.
8. **Phase projection of definitions is not memoized.**
   - Decision: `project_definitions` runs per `validate_for_phase` call.
   - Alternatives considered: a per-phase `OnceLock` on `EffectiveSchema`
     (contracts §10 as drafted).
   - Why: it is linear in the table and runs beside a full lowering of the
     document that is not memoized either; the derived JSON is memoized by
     `phase_json_schema`.
   - Downsides: a repeated phase validation repeats the projection.
9. **The caller file-mode classifier and the rewrite walkers carry the root
   now, although Wave 4 owns their reference following.**
   - Decision: thread `root` through `select_file_mode`,
     `resolve_caller_file_value`, `collect_applicable_root_schema_fragments`,
     and the `rewrite_*` walkers, and compile each union arm with its root's
     `$defs`.
   - Alternatives considered: leave arms that hold a `$ref` failing to
     compile (an arm silently dropped from "exactly one applicable arm"
     could turn an ambiguous union into a guessed file mode).
   - Why: the plan's Wave 3 task covers every independently compiled
     synthetic schema, including rewriting.
   - Downsides: a wider signature change in two files Wave 4 also edits.

### Requirement-to-test mapping (Phase 3)

All in `darkmatter/lib/tests/l1/recursive_schema_merge.rs` (L1, F2: public
`DarkmatterSchemas`/`EffectiveSchema`/`resolve_yaml_schema` with real files in
a temp dir) unless noted.

| Requirement | Test |
| --- | --- |
| Lookup: local pointer, escaping, missing target, external, ref-only loop | `schemas::scope::tests::a_reference_resolves_in_its_own_root_and_a_missing_target_is_an_error`, `…a_reference_chain_that_never_reaches_a_schema_is_an_error` (unit) |
| Synthetic root-union arm keeps the union `$defs` (validate + both phases, depth 6) | `a_recursive_root_union_arm_validates_with_the_union_definitions` |
| Same-named definitions in two file arms re-keyed; typed and JSON keys agree through phase projection | `file_arms_with_same_named_definitions_are_rekeyed_into_the_union_root` |
| Mixed raw/simplified arms; raw key moves, generated key kept | `a_raw_arm_keeps_its_definitions_and_an_inline_arm_keeps_its_keys` |
| An arm reference that would bind to another arm is refused (Unresolved / Unsupported) | `an_arm_reference_that_would_bind_to_another_arm_is_refused` |
| Baseline `$defs` join the document root; document keys never move; literal data untouched | `a_baseline_with_definitions_joins_the_document_root_without_rebinding_either` |
| Per-arm baseline merge with definitions at the union root once | `a_baseline_with_definitions_joins_every_arm_of_a_root_union_once` |
| `$id` arm cannot receive moved definitions; without `$defs` it merges as before | `a_baseline_cannot_move_its_definitions_into_an_arm_with_its_own_id` |
| Raw ingress matrix, JSON (every `$defs`, `$defs.item`, `$ref`, `$id`, `type`, `properties`, `required`, `items`, `anyOf`/`oneOf` cell; duplicate keys; trailing content) | `raw_json_baseline_ingress_takes_every_cell_from_a_recursive_control` |
| Raw ingress matrix, YAML (same cells; duplicate keys; second document; trailing content) | `raw_yaml_baseline_ingress_takes_every_cell_from_a_recursive_control` |
| In-memory baseline uses the same validator | `an_in_memory_baseline_shares_the_ingress_validator` |
| Existing acyclic acceptance unchanged without `$defs` | `a_baseline_without_definitions_keeps_its_other_reference_forms` |
| Trigger payload with recursion merges beside a recursive document; phase merge keeps payload defs; identity lookup | `a_recursive_trigger_payload_merges_beside_a_recursive_document` |
| Raw trigger payload ingress wiring (control + unresolved) | `a_raw_trigger_payload_with_an_unresolved_reference_is_not_mergeable` |
| Embedded namespaces (embedded payload + embedded library + document) | `embedded_payload_namespaces_join_a_recursive_document_without_collision` |
| Launch/Completion below recursion | `a_phase_projects_presence_below_the_recursion` |
| One definition shared by eager and deferred properties | `a_definition_shared_by_eager_and_deferred_properties_projects_once` |
| Example target judged with root definitions | `an_example_on_a_recursive_property_is_judged_with_the_root_definitions` |
| Folding: equal bodies shared, different re-keyed, dependent equal bodies move, literal data kept | `schemas::scope::tests::folding_shares_equal_bodies_and_rekeys_different_ones`, `…an_equal_body_that_references_a_moved_key_moves_too` (unit) |

Matrix cells use non-convertible bad values (`{"not": "text"}`) so they stay
meaningful once Wave 4 extends conversion past the old cutoff; a numeric bad
leaf at depth ≤ 4 is converted to text today and would hide the assertion.

Matrix smell check over the touched readers: no `#[serde(default)]`,
absence-collapsing `Option`, or `filter_map(.. as_str())` on a load-bearing
field was added. `schema_declares_property` reads a failed `chain` as "not
declared" (`unwrap_or_default`): it is a passive predicate over an already
compiled schema, documented at the line. `merge_layer`'s existing
`unwrap_or_default` reads of `properties`/`required` are unchanged.

### Gates

- Scope: `git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`
  → `packages: ["claudine", "claudine-gen", "darkmatter", "darkmatter-cli", "dmls", "test-toolkit"]`,
  `change_class: "package"` (the working tree still holds Phase 2's
  uncommitted changes as well), so no gate was skipped.
- `darkmatter`: `just lint` passed (including the Zed WASM check).
  `just test`: every darkmatter library and CLI L1 test passed; it stopped
  (fail-fast) on `dmls::l1 registry_preparation::a_held_preparation_leaves_edits_cancellation_and_other_requests_running`
  (a trace-log equality), which passes alone 3/3. The remaining DMLS tests
  were then run without it: `searched_candidates::a_watched_nearer_candidate_reselects_the_import_and_its_removal_falls_back`
  (already load-sensitive in Phase 2) and
  `startup_preparation::a_request_overtakes_the_remaining_startup_probes_after_an_unchanged_configuration`
  failed together and pass alone (2/3 then 4/4); every other DMLS test
  passed, and `zed-dmls-cli` passed 27/27. The host's load average was 30–65
  throughout from processes outside this session (another `rustc`,
  DaisyDisk, Finder). None of the three tests reads a recursive schema:
  **load-sensitive, pre-existing**. `just check-tier-coverage darkmatter` →
  0 stranded.
- `claudine`: `just test` first stopped on
  `claudine-cli::l1 wrap_watchdog_startup_stall::watchdog_startup_stall_terminates_without_a_wall_clock_timeout`
  and `…watchdog_opencode_stall_before_first_step_is_bounded` (whole-run
  wall-clock 5.0–5.4 s against a 4.7 s budget; in-wrapper time 2.1–2.4 s),
  at load average 44–65. Alone they passed, failed, and passed again with
  the load unchanged. Wave 3's new reference walk was measured on Claudine's
  shipped `claudine.yaml` payload at 53 µs per walk, so it cannot account for
  a second; **load-sensitive**. `just test --no-fail-fast` then passed
  10,692/10,692. `just lint` passed. `just check-tier-coverage claudine` → 0
  stranded.
- Dependent packages: `cargo check --tests` on `biscuit-speaks-cli`,
  `messenger`, `messenger-cli`, `sniff-cli`, `research`, `biscuit-icon-cli`,
  and `playa-cli` passed; none matches `SchemaError` exhaustively or calls
  `wrap_arm_as_root_schema`.
- Cross-OS: `just cross-check --os windows darkmatter recursive_schema_merge scope::tests`
  → 31 passed on native Windows (canonical `DefNamespace::File` paths and
  embedded virtual paths hold there).

## Phase 4

Phase 4 (Waves 4 and 5, "Adapt Every Consumer") started 2026-10-23 on the
tree at `bf89eeef0` (Phase 3 committed, clean working tree). The
orchestrator registered one L1 test module per task in
`darkmatter/lib/tests/l1/main.rs` up front (`recursive_schema_reporting`,
`_clean`, `_conversion`, `_rewrite`, `_completion`, `_typed_consumers`), so
the parallel tasks never shared a file. Wave 4 ran as four parallel
subagents with disjoint file ownership (4a validate/discriminant, 4b clean,
4c conversion translators, 4d rewrite); Wave 5 ran as two after Wave 4's
checkpoint (5a completion, 5b typed consumers). Each subagent verified only
its own slice; the area gates ran once at the end.

### Wave 4: JSON Walkers

**4a, reporting and discriminant** (`schemas/validate.rs`,
`schemas/discriminant.rs`). `schema_nodes_at`/`push_union_arms` treat a node
as itself plus every hop of its `SchemaScope::chain`, so a reference's
siblings and its target both apply; `resolve_problem_description`,
`description_at_node`, and `articulate_union` read the use site first (a
sibling description wins) and resolve against the root's `$defs` even when
the walk starts at a root-union arm. `select_literal_discriminant_arm_in(arms,
instance, &SchemaScope)` (new, crate-private) follows reference arms and
reference-typed discriminant properties; the public
`select_literal_discriminant_arm` keeps its signature and does not follow
references (its callers never see `$ref` arms: completion opens reference arms
to their bodies first, root-union arms lower inline).
`collect_discriminated_arms` became `DiscriminantWalk::collect`, guarded on
`(schema pointer, instance-value pointer)` pairs, and records the schemas
beside a union at the same location; `narrow_discriminated_union_problems`
compiles the selected arm with the root `$defs`, wrapped in `allOf` with an
asserting sibling (for example `minProperties` beside a `$ref`).

**4b, cleanup** (`schemas/clean.rs`). One shared path walker
(`schemas_at`/`schemas_along`, `expand_applicable`) builds every schema that
applies at an instance location: nullable arm unwrapped, every chain hop kept,
and it gives up (`None`) at a genuine `anyOf`, a missing target, or a
reference-only loop. Walk 1 (`solely_string`, quoting repair) needs at least
one `type: string` and no other `type` among the applicable schemas; walk 2
(`nearest_declared_key`, did-you-mean) offers the union of their
`properties`. The repeated-visit guard is per instance location.

**4c, conversion** (`coercion/{shape,engine,mod,error}.rs`,
`schemas/json_shape.rs`, `schemas/coerce.rs`,
`call_binding/{translate,mod}.rs`; two `Shape::Ref` arms in `call_check.rs`).
The one conversion engine gained `Shape::Ref(ShapeRef)`: the first use of a
recursive target owns it with a strong link, back-references inside its body
are weak, so dropping a root shape frees the cycle. A run of reference hops at
one value is capped at 64 (`ResourceLimit`), a backstop against a defective
shape. `json_shape.rs` makes a reference whose target is already being
expanded on the path, with a value descent since, a back-reference handle;
`CYCLE_EXPANSIONS` (4) is removed; `MAX_REFERENCE_DEPTH` (32) and
`REFERENCE_BUDGET` (4,096) still bound acyclic reference chains; a raw cycle
with no descent stays validation-only. `translate.rs` resolves
`TypeExpr::Ref` through a `DefinitionTable` passed to new
`*_with_definitions` entry points (the old ones use an empty table; a missing
key is `TranslateError::Invalid`), and checks use-site constraints beside the
handle as the lowering writes them as `$ref` siblings; `TooDeep` (32) remains
only for over-deep acyclic chains and loops that never descend. Old cutoffs
(recorded in Phase 1): `CYCLE_EXPANSIONS` 4, `MAX_REFERENCE_DEPTH` 32,
`REFERENCE_BUDGET` 4,096, grammar `TooDeep` 32.

**4d, rewrite** (`schemas/rewrite.rs`). `rewrite_property` resolves the chain
and applies each hop's siblings then the target (`rewrite_shape` applies one
schema's own keywords); an unresolvable chain leaves the value untouched even
with an eager sibling. A stack of active `(schema, value)` addresses guards
re-entry; `contains_eager_file_fragment` searches referenced definitions once
each. Union eligibility is unchanged (property: exactly one validating eager
arm; root: every accepting arm agrees). The caller file-mode classifier
(`compose/schema_validation.rs`) was left unchanged: it never descends into
object properties, so a recursive object caller value keeps its raw form
exactly as an inline object does.

**Orchestrator fix: nested missing properties were attributed to a
same-named top-level key.** 4c found that `validate::identify_key` (behind
`error_top_level_key`) and `schemas/mod.rs::attributable_top_level_key` took
a `required` failure's key from the error kind at any depth. A pending
top-level `children: $(…)` therefore hid a real "missing `children`" six
levels down a recursive `tree` (and the same in any nested object). Both now
use the kind only for a failure on the root, else the first pointer segment;
`coerce.rs`'s local `problem_key` workaround was folded back into
`error_top_level_key`. Regression:
`recursive_schema_reporting::a_nested_missing_property_is_not_hidden_by_a_pending_key_of_the_same_name`
(fails under the old `attributable_top_level_key`: zero problems reported).

### Wave 5: Typed Library Consumers

**5a, completion** (`schemas/completion.rs`, `schemas/simplified/query.rs`).
Editor-facing API published for Wave 6:

```rust
pub trait DefinitionLookup { fn lookup_definition(&self, key: &DefKey) -> Option<&Definition>; }
// implemented for DefinitionTable and EffectiveSchema (EffectiveSchema::definition)
pub enum PathStep<'a> { Key(&'a str), Index(usize) }
pub fn for_path(effective: &EffectiveSchema, frontmatter: &Value, path: &[PathStep<'_>]) -> Option<AlternativeCompletion>;
pub fn for_path_in<L: DefinitionLookup + ?Sized>(schema: &SimplifiedSchema, definitions: &L, frontmatter: &Value, path: &[PathStep<'_>]) -> Option<AlternativeCompletion>;
pub fn def_at_path<'a, L: DefinitionLookup + ?Sized>(schema: &'a SimplifiedSchema, definitions: &'a L, frontmatter: &Value, path: &[PathStep<'_>]) -> Option<(Cow<'a, PropertyDef>, AlternativeSite)>;
pub fn alternatives_for_def_in<L: DefinitionLookup + ?Sized>(def: &PropertyDef, site: AlternativeSite, value: &Value, definitions: &L) -> Option<AlternativeCompletion>;
pub fn alternative_accepts_in<L: DefinitionLookup + ?Sized>(atom: &PropertyAtom, value: &Value, definitions: &L) -> bool;
// simplified::query
pub fn resolved_alternatives<'a, L: DefinitionLookup + ?Sized>(def: &'a PropertyDef, definitions: &'a L) -> Vec<Alternative<'a>>;
```

`resolved_alternatives` opens only references at the top of the current
definition (direct, union arm, array element, through alias/union bodies),
never an inline object's properties; each `PathStep::Key` opens one
definition, `Index` none. Only `alternative_accepts_in` builds the transitive
closure (a fresh table of the reached definitions), because validating a
value needs it. `for_list_item`/`for_property_value` now resolve through the
effective schema. The old table-less `alternative_accepts`/
`alternatives_for_def` keep their signatures as the empty-table case.

**5b, describe and compare** (`simplified/subtype.rs`,
`scope/{projection,shape}.rs`, `call_check.rs`, `frontmatter_shape.rs`,
`schemas/catalog.rs`, `compose/context/catalog.rs`; wiring in
`compose/call_precheck.rs`, `compose/pipeline/mod.rs`,
`dmls/src/diagnostics/call_check.rs`, re-exports). `is_subtype_with_definitions`
matches two references to one `DefKey` by identity, otherwise unfolds a
non-array `Ref` to its body's alternatives, and treats a `(source,
destination)` pair already on the path as holding (coinductive), so each pair
enters a body once per path. `DeclaredTypes::with_definitions` carries the
table; a recursive property projects to its named `Ref` (`top_level_types`).
`check_call_with_definitions`/`classify_argument_with_definitions`: `arm_fit`
translates through the table; `fit` unfolds a `Ref` on the argument side only,
so each re-entry is one level deeper into the parameter's finite shape.
`GlobalDeclaration` keeps its resolved table (it was dropped);
`compose/context/catalog.rs::project_one` follows a `Ref` to its value type
instead of reporting `Object`; whole-type text renders a `Ref` by its
authored name (`serialize_property_def`). Path lookups
(`property_def_at_path_with_definitions`, `nested_property_shape_with_definitions`)
open one body per segment.

Audit conclusions (no edits): `about.rs` has no production `TypeExpr` walk;
`detect.rs` builds from instances (`primitive()` maps `Ref` to `None`);
`triggers/matcher.rs`, `call_binding/registration.rs`,
`expression/catalog/mod.rs`, and `shell_expansion/template/canonical.rs`
parse passively, so a `Ref` cannot reach them and no file resolution was
added; `file_match.rs` reads only a bare `file` atom, which a recursive body
never is.

**Follow-up in this phase: shell-template holes.** The audit found that
`shell_expansion/template/typing.rs` does receive a `Ref` (from
`DeclaredTypes`) and judged it with the table-less `alternative_accepts`, so
a hole typed by a recursive union refused every value, failing closed;
Claudine's sequence preflight typed holes the same way. A third subagent
threaded the definitions: `HoleValidator::Declared { definition, definitions:
Arc<DefinitionTable> }` (pruned to what the declaration reaches; refinements
carry a table too), `SiteScope`/`FrontmatterTemplates` and preflight discovery
keep the `EffectiveSchema`, and `lifecycle_declared_types` takes
`Option<&EffectiveSchema>` (Claudine's `preflight.rs`, `prepare.rs`, and
`sequence/preflight/mod.rs` updated). `scope/shape.rs`: guard narrowing keeps
a `Ref` arm on both sides (it counts as every family), so it can never drop
one and prove a false category 5; `pinned_number` judges only a literal it
built; `ScopedType::admits` refused every value of a recursive type and now
admits for an arm that is or contains a reference (the same
over-approximation an unknown shape makes; a shape has no table).

**Docs and skill.** `darkmatter/docs/topics/schemas/definition.md` gained a
"Recursive Types" conversion subsection and the recursive completion API
(`for_path`, `def_at_path`, the `*_in` variants, with an example);
`darkmatter/docs/inline/call-check.md` a rule for recursive arguments;
`darkmatter/docs/inline/preflight-checks.md` a sentence on recursive-typed
holes; `.claude/skills/darkmatter/schema.md` names the reference-aware entry
points and that a `Ref` judged without a table is a defect.

### Design Decisions

1. **The conversion engine gains a reference shape; no second engine.**
   - Decision: `Shape::Ref(ShapeRef)` in `markdown/coercion/`, strong for the
     first use, weak for back-references; both translators build it.
   - Alternatives considered: instance-directed re-translation per value
     (shapes are cached per schema and per function signature, which only
     works if they do not depend on the value); raising the cutoffs (still a
     fixed unroll).
   - Why: spec Decided "Both conversion translators follow `Ref`"; a
     strong-only cycle would leak every recursive shape in long-lived DMLS.
   - Downsides: a fragment cloned out of a recursive body and kept after its
     root is dropped matches nothing (`NoConversion`), documented on
     `ShapeRef`.
2. **A missing reference target authorizes nothing in every passive or
   mutating walker.**
   - Decision: description lookup → `None`; discriminant selection → no
     narrowing; clean walks → no repair/suggestion; rewrite → value
     untouched; grammar translator → `TranslateError::Invalid`; completion →
     no suggestions.
   - Alternatives considered: surface a `SchemaError` from each walker (these
     walkers have no error channel and run after the validator, which already
     refuses to compile a missing target); treat the missing body as empty
     (would read as unconstrained, which spec design 7 forbids).
   - Why: resolution and validator compilation already report the missing
     target as a schema error; a walker must never turn it into permission.
   - Downsides: a broken raw schema silently loses repairs and rewrites while
     its error is reported elsewhere.
3. **Walker guards are `(schema, instance)` pairs, never a global visited
   set.** Applied in `DiscriminantWalk`, `expand_applicable`, the rewrite
   stack, and the `json_shape` path; per contracts §9.
4. **Reference-aware APIs are additive (`*_in`, `*_with_definitions`), with
   the old signatures as the empty-table case.**
   - Alternatives considered: change every public signature (Claudine and
     DMLS callers would all change in one wave and a later caller could not
     opt out of a table it does not have).
   - Why: matches Wave 4's naming; acyclic behavior stays byte-identical.
   - Downsides: a caller that forgets the table fails closed (category 4,
     "accepts nothing"), not loudly. The skill now calls that a defect, and
     every production caller that can see a `Ref` was given its table.
5. **`lifecycle_declared_types` takes `Option<&EffectiveSchema>` instead of
   gaining a variant.**
   - Alternatives considered: an additive `_with_definitions` variant.
   - Why: every caller already holds an effective schema or passes `None`;
     two functions would invite the table-less one.
   - Downsides: a public signature change, absorbed by Claudine in the same
     change (no external users).
6. **Comparison by identity ignores use-site bounds.**
   - Decision: `is_subtype` equates two references to one `DefKey`.
   - Alternatives considered: also compare their use-site constraints
     (contracts §11's wording).
   - Why: the only constraints a `$ref` site carries are bounds, which the
     subtype relation ignores for every type; the two readings agree.
   - Downsides: none found.
7. **Recursive variables project to their named `Ref`, not one unfolded
   level.**
   - Why: keeps comparison by `DefKey` and messages naming the type ("is
     node, expected …").
   - Downsides: guard narrowing has no table, so a recursive arm counts as
     every family (safe, less precise).
8. **Nested `required` failures are attributed to the key that holds them.**
   - Decision: fixed in `identify_key` and `attributable_top_level_key`
     (shared by validation's pending deferral, coercion, and rewrite).
   - Alternatives considered: the local workaround 4c first wrote in
     `coerce.rs` (left the same defect in validation and rewrite).
   - Why: Rule 5; the defect hid a real error under any pending key that
     shared a nested property's name, and recursion makes such names common.
   - Downsides: none; root-level absence is still keyed by the missing name.
9. **The caller file-mode classifier does not follow references.** It never
   descends into object properties, so a recursive object is treated as an
   inline object (left raw, never rewritten). A recursive union of files
   passed as a caller override is likewise left raw; following references
   there would mean changing `select_file_mode`,
   `resolve_caller_file_value`, and the projection together, for a shape no
   shipped schema uses.

### Requirement-to-test mapping (Phase 4)

L1 (lib `darkmatter/lib/tests/l1/`, DMLS `darkmatter/dmls/tests/l1/`), F2:
public `DarkmatterSchemas`/`EffectiveSchema`/composition/LSP session over
temp-dir fixtures. Each walker test was shown to fail with its dereference
disabled (temporary mutation, restored and diffed), except where noted.

| Requirement | Test |
| --- | --- |
| Depth ≥ 6 failure: JSON path + line/column + selected arm description | `recursive_schema_reporting::a_depth_seven_failure_is_reported_once_at_its_path_and_position_with_the_selected_arm_description` |
| Discriminant through reference-member arms | `recursive_schema_reporting::a_union_of_reference_arms_is_narrowed_by_the_literals_in_their_definitions`, `…a_reference_arm_selected_at_the_root_of_the_tree_explains_a_missing_field_below_it`; unit `discriminant::tests` (reference arms, nullable reference const, missing target) |
| Synthetic validator keeps root context | `recursive_schema_reporting::a_root_union_arm_explains_a_deep_failure_through_the_root_definitions`; unit `validate::tests::narrowing_follows_a_reference_and_keeps_its_sibling_assertion` (fails with root `$defs` or siblings dropped) |
| Nested missing key not hidden by a same-named pending key | `recursive_schema_reporting::a_nested_missing_property_is_not_hidden_by_a_pending_key_of_the_same_name` |
| Clean walk 1 permitted / forbidden | `recursive_schema_clean::recursive_schema_clean_quotes_a_number_deep_below_each_reference_form`, `…never_quotes_inside_a_union_reached_through_a_reference` |
| Clean walk 2 permitted / forbidden | `recursive_schema_clean::recursive_schema_clean_suggests_a_declared_key_at_every_depth`, `…never_suggests_an_arm_key_through_a_reference`; unit `clean::walk_tests` (missing target, ref-only loop, sibling veto, union stop, nullable cycle) |
| Conversion past the old cutoff; final validation; atomic failure | `recursive_schema_conversion::a_declared_conversion_applies_below_the_old_expansion_cutoff`, `…composition_writes_back_a_deep_recursive_conversion`, `…final_validation_still_runs_below_the_old_expansion_cutoff`, `…a_non_convertible_deep_value_leaves_the_whole_input_unchanged`, `…a_deep_missing_value_does_not_block_a_sibling_conversion` |
| Union candidate safety through a reference | `recursive_schema_conversion::an_ambiguous_union_reached_through_a_reference_is_not_coerced` |
| Engine 64-level bound → `ResourceLimit`, input unchanged | `recursive_schema_conversion::a_value_deeper_than_the_engine_bound_fails_with_resource_limit` (both translators) |
| Grammar translator past `TooDeep`; non-descending still `TooDeep` | `recursive_schema_conversion::a_recursive_named_type_binds_deep_function_arguments`, `…a_named_type_that_never_descends_still_fails_too_deep` |
| Raw cycles: descending converts, non-descending validation-only | `recursive_schema_conversion::a_raw_descending_cycle_converts_at_any_depth`, `…a_raw_cycle_that_never_descends_stays_validation_only` (pin) |
| Translator conformance with recursive cases | `coercion_translator_conformance::{recursive_object_through_a_list, recursive_list_through_a_union_item, mutually_recursive_objects_through_optional_properties, ambiguous_union_inside_a_recursive_type, use_site_constraints_beside_a_recursive_reference}` |
| Deep rewrite; lazy/ambiguous unchanged; passive validation; idempotence; missing target | `recursive_schema_rewrite::{composition_rewrites_an_eager_file_seven_levels_into_a_recursive_type, lazy_and_ambiguous_alternatives_through_a_reference_are_unchanged, passive_validation_rewrites_and_runs_nothing, repeated_normalization_through_a_recursive_type_is_idempotent, a_missing_reference_target_never_authorizes_a_rewrite}`; unit `rewrite::tests::a_definition_reentered_for_the_same_value_rewrites_nothing` |
| Completion at recursion levels 0–3, annotations, incomplete items | `recursive_schema_completion::{item_keys_are_offered_at_recursion_levels_zero_through_three, a_leaf_below_recursion_keeps_its_suggestions_and_description, incomplete_items_and_objects_offer_the_next_keys}` |
| Lazy lookup (only cursor-path definitions opened) | `recursive_schema_completion::descent_opens_only_the_definitions_on_the_cursor_path` (fails with eager closure expansion) |
| Missing definition offers nothing; recursive discriminated narrowing | `recursive_schema_completion::{a_missing_definition_offers_nothing, a_recursive_discriminated_union_narrows_at_every_level}` |
| `is_subtype` coinductive, self and mutual recursion | `recursive_schema_typed_consumers::subtyping::{a_recursive_union_is_compared_through_its_definition, self_and_mutually_recursive_references_terminate_with_identity_answers}` |
| Call check category 1 / 5 / 3 on recursive arguments | `recursive_schema_typed_consumers::call_check::{a_recursive_object_is_category_one_for_an_object_and_five_for_a_number, a_recursive_union_is_judged_by_its_alternatives}` |
| Composition rejects category 5 before evaluation; category 1 composes | `recursive_schema_typed_consumers::composition::{a_category_five_recursive_argument_is_rejected_before_evaluation, a_category_one_recursive_argument_composes}` (the latter passes either way; the direct category 1 test pins the difference) |
| DMLS `dm.expression.incompatible_call` agrees with composition | dmls `recursive_call_check::the_editor_reports_exactly_the_recursive_call_composition_rejects` |
| Variable projection keeps the named `Ref` | `recursive_schema_typed_consumers::call_check::the_projected_type_of_a_recursive_property_is_its_named_reference` |
| Nested shape lookup one body per segment (depth 4) | `recursive_schema_typed_consumers::lookups::a_nested_lookup_follows_one_definition_per_segment_at_depth_four` |
| Catalog renders the named back-reference; non-object refused | `recursive_schema_typed_consumers::catalogs::{a_recursive_global_loads_and_describes_itself_by_name, a_recursive_global_that_is_not_an_object_is_refused}`; unit `compose::context::catalog` `project_one_follows_a_recursive_reference_to_its_definition` |
| Shell-template holes typed by a recursive union | `recursive_schema_typed_consumers::shell_template_holes::{a_recursive_union_hole_fills_with_a_scalar_alternative_and_refuses_others, a_guard_narrowed_recursive_hole_judges_the_reference_by_its_body, a_document_hole_typed_by_a_recursive_union_fills_with_a_scalar}`, `…shell_template_preflight::a_late_recursive_union_value_passes_preflight_validation`, `…scoped_types::a_recursive_reference_arm_admits_a_value_of_its_type` |

Existing tests updated for the decided behavior change (conversion now
continues past four expansions): `raw_schema_conversion::cyclic_local_reference`
now asserts the deep `"2"` converts and a deep `pear` is still rejected;
`recursive_schema_types::a_self_recursive_type_validates_depth_eight…` uses
`[42]` (unconvertible) as its bad leaf instead of `42`.
`shell_template_policy.rs` matches `HoleValidator::Declared { .. }`.

Input robustness matrix: this phase adds no reader of a file format or
configuration (walkers consume already-loaded schemas), so no new cells.
Smell check over touched code: no new `#[serde(default)]`,
absence-collapsing `Option`, or `filter_map(.. as_str())` on a load-bearing
read.

### Gates

- Scope: `git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`
  → `packages: ["claudine", "darkmatter", "dmls", "test-toolkit", "claudine-cli"]`,
  `change_class: "package"`, so no gate was skipped.
- `darkmatter`: `just test` passed 11,850/11,850 (lib, CLI, DMLS, zed-dmls-cli;
  10 skipped by tier) in one run, with no failures, including the previously
  load-sensitive DMLS tests. `just lint` passed (including the Zed WASM
  check). `just check-tier-coverage darkmatter` → 0 stranded.
- `claudine` (changed directly: `composition/{preflight,prepare}.rs`,
  `sequence/preflight/mod.rs`): `just test` passed 10,692/10,692; `just lint`
  passed; `just check-tier-coverage claudine` → 0 stranded.
- Dependent packages: only darkmatter and Claudine reference the changed
  public items (`lifecycle_declared_types`, `HoleValidator`, `SiteScope`,
  `DeclaredTypes`, `completion`, `coercion::Shape`); `cargo check --tests` on
  `biscuit-speaks-cli`, `messenger`, `messenger-cli`, `sniff-cli`,
  `research`, `biscuit-icon-cli`, and `playa-cli` passed.
- Cross-OS: not run. This phase changes schema-walking logic over in-memory
  JSON and typed trees, with no path, process, or OS-specific code; its file
  fixtures use `TempDir` paths the way the Phase 3 tests that passed on
  native Windows do.
- Subagent mutation runs used `--no-fail-fast` on narrowly filtered tests
  only, to see each mutated test's verdict; no full-suite run used it.

## Phase 5

Phase 5 (Waves 6 and 7, "Prove User Paths and Maintain Documentation")
started 2026-10-23 on the tree at `bf89eeef0` plus the uncommitted Phase 4
working tree (Phase 4 was not committed before this phase began, so the
working tree carries both phases' changes). The orchestrator registered one
L1 module per test-writing task up front (`dmls/tests/l1/recursive_schema_editor.rs`,
`cli/tests/l1/schema_recursive.rs`, `lib/tests/l1/recursive_schema_frozen_fixture.rs`)
so the four parallel tasks never shared a file: 6a DMLS editing (also owning
`dmls-schema-support.md`), 6b CLI normal invocation, 7a frozen fixtures and
the generator path fix, 7b current docs. Each verified only its own slice;
the area gates ran once at the end.

The first launch of all four subagents died on an API overload (HTTP 529)
before any of them edited a file (confirmed with `git status`); all four were
relaunched from the same briefs.

### Wave 6: Editor and CLI

**6a, DMLS recursive editing** (`dmls/src/overlay/{schema,mod}.rs`,
`providers/{frontmatter,dsl,code_actions}.rs`,
`providers/frontmatter/sequence_tests.rs`, `diagnostics/named_types.rs`,
`work_counters.rs`). No library change was needed; DMLS uses the Phase 4
APIs. `SchemaBundle` now carries `extension_definitions` (extension
baselines merge into the baseline JSON before the effective schema exists,
so `EffectiveSchema::definition` cannot see their tables) and implements
`DefinitionLookup` (effective schema tables, including trigger tables, then
extension tables); `NoDefinitions` covers documents with no assembled
schema. Every frontmatter walk (`descend`, `def_at_path_ctx`,
`array_item_def`, `inline_object_shape`, `nested_shape`, `def_at_path`,
value completion, `has_union_items`, union-item matching through
`alternative_accepts_in`, item completion through `alternatives_for_def_in`)
and the overlay's authoring walk (`def_at_path`, `merged_inline_shape`,
`meta_schema_kinds`, through `resolved_alternatives`) follows references. No
table-less `alternative_accepts(`/`alternatives_for_def(`/
`property_def_at_path(`/`nested_property_shape(`/`is_subtype(`/`check_call(`
call remains in DMLS. Hover on a recursive property names the type and shows
its definition with recursive uses written by name; `alternative_line`
lists a reference arm's keys. Two defects found and fixed on the way:
`error_dependencies` did not record the files of a `NonDescendingCycle`, so
fixing a file reached only through the cycle did not clear the error without
a watched-file event; a cycle inside a document's own inline `$schema`
listed no related locations. An empty `- ` line in a list of objects now
offers the object's keys (it offered nothing for any object list before).
The existing document-text cache is unchanged; no schema cache was added.
Laziness is proven at the DMLS level through a `work_counters` recorder
(`record_definition_lookups()`/`definition_lookups()`, inert unless a test
turns it on): every request opens only `item`, never the unreached `meta`/
`tag` types, and lookups grow linearly with the cursor's level above a
per-request constant (the authoring-state rebuild re-walks the document's
own keys twice per completion; it is document-, not cursor-dependent).
`dmls-schema-support.md` gained a "Recursive types" section with a Mermaid
diagram, and its named-type table now spells the alias cycle
`NonDescendingCycle` (it said `ImportCycle`; the code is right, the doc was
stale). `dmls/docs/autocomplete.md`, `dmls/docs/hover.md`, and
`.claude/skills/darkmatter/dmls.md` got matching notes.

**6b, CLI normal invocation** (`cli/src/commands/schema/assignment.rs`,
`cli/tests/l1/schema_recursive.rs`). `assignment.rs` is the only
darkmatter-cli source walking `TypeExpr`; a `Ref` returned "not
string-preferring", so `md schema validate doc prop=value` YAML-parsed an
assignment to a recursive type with a string arm. `def_prefers_string` now
opens the reference through `EffectiveSchema::definition`, each definition
at most once (a missing one keeps the YAML parse). Unit test
`a_recursive_type_is_judged_by_its_definition_body` and CLI test
`a_recursive_assignment_keeps_its_text_when_the_definition_has_a_string_arm`
(`node: [string(min(4)), {…}]`, `label=1.20` passes, `label=1.2` fails;
a binary with the arm reverted fails `label=1.20`). The fourteen CLI tests
use `CliProcessFixture` with temp-dir fixtures, no repository reads, no
spawn loop (about 2 s total). Positions are compared with in-process
`DarkmatterSchemas::validate` on the same text, the validator the editor
publishes from. Observed and left as is (pre-existing): `md compose` runs
`$(…)` only in top-level frontmatter values; `md compose --frontmatter`
writes `item: null` for an optional type-declaring property (stable on
re-compose).

### Wave 7: Fixtures and Current Docs

**7a, frozen fixtures and the generator.** `darkmatter/lib/tests/fixtures/recursive_schema_frozen/`
holds byte-for-byte copies of the spike's `action.yaml` and depth-5
`lifecycle.yaml` (checked with `cmp` and by test), a README, and the
hand-written `lifecycle-recursive.yaml`: the unrolled `item-5` body with
`then`/`else` set to `[item[]@this, bundle@./action.yaml]`, `stack` the
same union, `loop.gate` `stack@this`; the spike's merged-mapping item and
inline verb table are kept. Wired into the same seven consumers the spike
measured (39.9 MB unrolled), it serializes to **430,195 bytes**. The
generator now sets `SPIKE = Path(__file__).resolve().parent`,
`REPO = SPIKE.parents[4]`, `OUT = SPIKE / "schemas"`; its usage text and the
spike README's generator line changed, nothing else. `darkmatter/lib/Cargo.toml`
`[package.metadata.ci.tests] source-inputs` lists the generator so a change
to it schedules the tests that read it. The generator cannot actually
regenerate in today's tree: its unchanged input
`darkmatter/docs/schemas/expression-functions.yaml` no longer exists (the
frozen fixtures never depend on regeneration). While the generator's
absolute path was temporarily restored for a mutation check, it created
empty directories under `/Volumes/coding/wt/rusty-biscuit/feat-schema-enhancement/`;
they held no files and were removed with `rmdir`.

**7b, current docs.** `definition.md` gained "Recursive Named Types" (finite
leaf, direct/mutual/two-file cycles, alias into a cycle, the child-value
rule with a Mermaid flowchart of cycle admission and the mixed-group case,
the typed error, definition versus use site, namespace identity, lowering,
validation/phases/conversion/merge/function-type behavior, brief editor
behavior linking `dmls-schema-support.md`); the Error Model table now
matches the code (`ImportCycle` only for an over-deep non-repeating chain;
rows for `NonDescendingCycle`, `UnresolvedReference`,
`UnsupportedMergeReference`); trigger payloads may carry recursive types.
`local-type-references.md` no longer calls self-reference `ImportCycle`;
`authoring-schemas.md` gained a "Recursive Types" walkthrough;
`.claude/skills/darkmatter/{schema,SKILL}.md` updated (the skill said typed
walkers treat a reference as opaque). `schema-triggers.md` is an empty stub
since an earlier docs consolidation (trigger docs live in `definition.md`
and `authoring-schemas.md`), so it was left alone. Examples were run with
`md schema validate --no-baseline-schema` in a temp dir. The orchestrator
added one sentence to `definition.md`'s assignment coercion paragraph for
the 6b behavior change.

### Design Decisions

1. **DMLS implements `DefinitionLookup` on `SchemaBundle` and passes it into
   its own walks.**
   - Alternatives considered: replace DMLS's descent with the library's
     `for_path`/`def_at_path` (would discard DMLS's memoized,
     discriminant-aware descent wholesale).
   - Why: smallest change; each step is still one library
     `resolved_alternatives` call, so laziness is the library's.
   - Downsides: several DMLS signatures gained a lookup parameter.
2. **Laziness is asserted against a per-request constant, not an absolute
   count.** The authoring-state rebuild re-walks the document's authored
   keys on every request; caching it is outside this feature (it would be a
   new cache). The test asserts only cursor-path definitions open and that
   growth per level is bounded.
   - Downsides: the constant (8 lookups in the fixture) is document-sized
     work on every request.
3. **An empty `- ` line offers keys for every object list, not only
   recursive ones.** The gap was identical for plain lists (Rule 5).
   - Downsides: a small visible completion change for plain lists; all
     existing tests pass.
4. **`md schema about` is covered as "terminates and is unaffected".**
   Acceptance criterion 9 asks to exercise `md schema about` with a
   recursive fixture, but the command is documentation-only and takes no
   schema input, so no recursive schema reaches it. The test proves it
   finishes with identical output with recursive schemas in cwd and the
   home schema directory; the named back-reference a user sees is proven
   through `md compose`'s call check (`argument 1 (a) is item, expected
   number`) and DMLS hover.
   - Alternatives considered: add a schema-input mode to `about` (new
     surface the spec does not ask for).
   - Downsides: criterion 9's `about` clause is met in that reading only.
5. **CLI/DMLS position agreement is asserted through the shared library
   validator.** CLI tests compare `md schema validate --format json` with
   in-process `DarkmatterSchemas::validate`; DMLS tests assert the
   published diagnostic's line/column for the same depth-7 failure.
6. **The unrolled/recursive verdict comparison compiles the resolved schema
   directly.** `effective_for` on the unrolled fixture took about 3.6 s to
   set up plus 3.2 s for the first rejection's diagnostics, over the 5 s L1
   budget. The comparison resolves `success: stack@./<file>` with
   `resolve_yaml_schema` and compiles with `jsonschema` (about 2 s alone);
   invalid leaves are unconvertible (`[42]`, `{not: text}`, an unknown
   enum), so conversion cannot change the verdict. Depth invariance and the
   depth-7 test go through full Darkmatter validation.
   - Downsides: conversion is not exercised by the comparison itself.
7. **The generator's repository root is `parents[4]` of the script.**
   - Alternatives considered: search upward for a marker file.
   - Why: the spike lives in a closed feature directory that does not move.
   - Downsides: a move would need that number changed; the behavioral test
     would catch it.
8. **The generator test runs Python with a stub `yaml` module.** PyYAML is
   not a repository dependency and is absent on this host; the test only
   needs the script's path behavior. It probes `python3`/`python` with
   `--version` (the Windows alias trap) and skips with a message if none is
   found, as `scripts/ci-rollup-tests.rs` does.
9. **The passive-validation CLI test uses `git init -q <dir>` as its
   sentinel command, with the fixture's `host_path()` escape.** `mkdir` is a
   `cmd` builtin with no executable on native Windows ("Command not found"),
   and `cmd /c mkdir` is refused by Darkmatter's interpreter policy
   ("Blocked command"). `git` is on every runner and rig.
   - Downsides: the test sees the host `PATH`.

### Requirement-to-test mapping (Phase 5)

All L1, F2/F3: DMLS through the L1 LSP session API over temp-dir
workspaces; CLI through `CliProcessFixture` spawning the real `md`; library
through public `resolve_yaml_schema`/`DarkmatterSchemas`.

| Requirement | Test |
| --- | --- |
| Completion at recursion levels 1–3 (keys and suggestions) | dmls `recursive_schema_editor::completion_offers_the_definition_at_recursion_levels_one_two_and_three` (mutation: bundle lookup returns nothing → fails) |
| Incomplete items/objects offer next keys | `…::an_empty_item_and_a_key_less_object_offer_the_next_keys` |
| Hover at levels 1–3 | `…::hover_describes_the_definition_at_recursion_levels_one_two_and_three` |
| Lazy lookup along the cursor path only | `…::completion_and_hover_open_only_the_definitions_on_the_cursor_path`; library `recursive_schema_completion::descent_opens_only_the_definitions_on_the_cursor_path` |
| Whole-type hover with named back-references | `…::whole_type_hover_names_the_recursive_back_reference` |
| Union arm that is a reference | `…::a_recursive_union_arm_is_listed_with_its_keys` |
| Navigation to the defining file/span across a two-file cycle | `…::definition_and_descent_follow_a_two_file_cycle` |
| Extension-baseline definitions reach the editor | `…::an_extension_baseline_recursive_type_is_descended` |
| Positioned deep (depth 7) failure published by DMLS | `…::a_deep_validation_failure_is_published_at_its_node` |
| Non-descending cycle diagnostic, related spans, same document | `…::an_inline_alias_cycle_is_reported_on_its_references` |
| Two-file non-descending cycle, spans in both files, clears when fixed | `…::a_two_file_alias_cycle_relates_both_files_and_clears_when_fixed` |
| `md schema validate` direct recursion depth 7: pass / fail with path + line/column (= library) | cli `schema_recursive::{a_self_recursive_document_seven_levels_deep_validates, a_self_recursive_failure_seven_levels_deep_reports_its_path_and_position}` |
| Two-file recursion depth 7 | `schema_recursive::{a_two_file_recursive_document_seven_levels_deep_validates, a_two_file_recursive_failure_seven_levels_deep_reports_its_path_and_position}` |
| Required `children`: `[]` leaf valid; null / missing at depth 4 fail at that node | `schema_recursive::required_children_accept_an_empty_leaf_and_reject_null_or_missing_at_depth_four` |
| Optional reference: missing/null accepted, null list item rejected | `schema_recursive::an_optional_recursive_reference_accepts_missing_and_null_but_not_a_null_list_item` |
| Passive validation runs no expression, source bytes unchanged | `schema_recursive::validation_runs_no_expression_inside_a_recursive_item_and_leaves_the_source_unchanged` |
| `assignment.rs` follows `Ref` | `schema_recursive::a_recursive_assignment_keeps_its_text_when_the_definition_has_a_string_arm`; unit `assignment::tests::a_recursive_type_is_judged_by_its_definition_body` (both fail with the arm reverted) |
| Function type only through a recursive definition is a schema-load error | `schema_recursive::a_function_type_reachable_only_through_a_recursive_definition_is_a_schema_load_error` |
| Conversion past the old cutoff in normal composition | `schema_recursive::composition_converts_a_value_below_the_former_expansion_cutoff` (depth 9) |
| Failed conversion: exit 1, source unchanged | `schema_recursive::a_failed_deep_conversion_fails_composition_and_leaves_the_source_unchanged` |
| Repeated normalization idempotent | `schema_recursive::repeated_composition_of_a_recursive_document_is_idempotent` |
| Named back-reference through `md` | `schema_recursive::the_call_check_names_a_recursive_argument_by_its_type_name` |
| `md schema about` with recursive schemas present | `schema_recursive::schema_about_terminates_beside_recursive_schemas` (Decision 4) |
| Frozen copies equal the committed spike inputs | lib `recursive_schema_frozen_fixture::{frozen_action_table_matches_the_committed_spike_schema, frozen_unrolled_lifecycle_matches_the_committed_spike_schema}` |
| Recursive fixture ≤ 2 MB (430,195 bytes) | `…::recursive_fixture_resolves_to_at_most_two_megabytes` |
| Instance depth does not change definitions/schema (3, 6, 10) | `…::instance_depth_does_not_change_the_recursive_schema` |
| Unrolled/recursive verdict equivalence ≤ 5 levels | `…::unrolled_and_recursive_fixtures_agree_within_five_levels` (2.1 s alone, 2.9 s on Windows) |
| Deeper levels only on the recursive fixture | `…::recursive_fixture_checks_levels_beyond_the_unroll` (depth 7) |
| Generator: no absolute path; writes beside itself from an unrelated CWD | `…::{generator_names_no_absolute_path, generator_writes_beside_itself_from_an_unrelated_cwd}` (both fail with the absolute path restored; the latter ran Python on native Windows, not skipped) |

Input robustness matrix: this phase adds no reader of a file format or
configuration, so no new cells. Smell check over touched code: no new
`#[serde(default)]`, absence-collapsing `Option`, or
`filter_map(.. as_str())` on a load-bearing read.

### Gates

- Scope: `git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`
  → `packages: ["claudine", "darkmatter", "darkmatter-cli", "dmls", "test-toolkit", "claudine-cli"]`,
  `change_class: "package"` (the working tree still carries Phase 4), so no
  gate was skipped.
- `darkmatter` `just test`: the first run stopped (fail-fast) at
  10,997/11,884 on a 30 s timeout of
  `darkmatter-cli::l1 schema_validate_core_types::null_admitting_presence_matches_the_phase_a_table_end_to_end`
  (untouched by this phase) while the host's load average was about 100
  from processes outside this run (installed `dmls` instances, Finder,
  other rustc builds; no subagent was running). Alone it passed in 6.5 s:
  **load-sensitive**. A full rerun passed **11,884/11,884** (10 skipped by
  tier). `just lint` passed (including the Zed WASM check), and passed
  again after the Windows fix below. `just check-tier-coverage darkmatter` → 0
  stranded.
- `claudine` `just test`: **10,692/10,692** passed; `just lint` passed;
  `just check-tier-coverage claudine` → 0 stranded. This phase changed no
  Claudine source; Claudine is darkmatter's main dependent.
- Dependent packages: this phase changed no public library API (DMLS and
  CLI internals, tests, fixtures); Phase 4's `cargo check --tests` sweep of
  the other dependents still applies.
- Cross-OS (native Windows, `just cross-check --os windows`):
  `darkmatter recursive_schema_frozen_fixture` 8/8, `dmls
  recursive_schema_editor` 11/11, `darkmatter-cli schema_recursive::`
  14/14 after two fixes to the passive-validation test's sentinel command
  (Decision 9; attempt 1 `mkdir` → command not found, attempt 2
  `cmd /c mkdir` → blocked interpreter, attempt 3 `git init` → pass). The
  generator test's output on Windows shows it ran, not skipped. Note: the
  `darkmatter` cross-check package is the library crate alone, so the CLI
  and DMLS tests need the `darkmatter-cli` and `dmls` package names; a
  quoted `-E` expression breaks the remote recipe (already in the `os`
  skill), so pass positional substrings, which nextest ORs.
- Known pre-existing failure outside scope: 7a observed
  `scripts/ci` `RealWorkspaceTestInputTests::test_readme_typos_schedule_nothing`
  failing because `darkmatter/README.md` is read by
  `trigger_spelling_guard::no_active_file_spells_a_renamed_trigger_form`;
  unrelated to this phase's files.

## Phase 6

Phase 6 (Waves 8–10, "Validate Compatibility and Prepare Review") started
2026-10-23 on the tree at `bf89eeef0` plus the uncommitted Phase 4 and
Phase 5 working tree.

On reading the spec against the plan, the spec's **Part A** ("Measurement and
validation waste", acceptance criteria 12–15) was never scheduled by any plan
phase, and none of it exists in the tree (no validation stage timings, no
DMLS slow-validation event or `schema.slow_validation_ms` setting, no
`md schema validate -vv` report, no cold-cache benchmark cases). This phase
adds it as Wave 8A (Decision 1).

### Wave 8: Performance Evidence

Measured 2026-10-23 on the dev Mac (Apple M4 Max, 16 cores, 128 GB,
macOS 27.2 build 26B5091g, rustc 1.98.1), one session, `bench` profile
(release, optimized), Criterion through `just bench-schema` in `darkmatter/`,
host load average 13 → 19 during the run. Groups `resolve_frozen`,
`resolve_frozen_io`, `resolve_frozen_compile` in
`darkmatter/lib/benches/schema_validation.rs`, 10 warm samples each
(Criterion's minimum; the plan asked for at least five), median reported.
Resolve boundary for both fixtures: one in-memory `resolve_yaml_schema` call
on the seven-consumer wiring the L1 size test uses (six events `stack@` plus
`loop@`), which includes SimplifiedSchema lowering.

| Fixture | Resolve (median) | Raw file read (I/O) | Validator compile | Serialized JSON |
| --- | --- | --- | --- | --- |
| unrolled depth 5 (`lifecycle.yaml`) | 1,561.4 ms | 0.028 ms | 1,196.6 ms | 37,291,010 bytes |
| recursive (`lifecycle-recursive.yaml`) | **15.4 ms** | 0.025 ms | 8.1 ms | 430,195 bytes |

Threshold: 150 ms × (1,561.4 / 2,683) = **87.3 ms**. The recursive median
(15.4 ms) meets it, about 100× faster than the unrolled fixture on the same
host. I/O is the raw read of `action.yaml` plus the lifecycle file (timed
alone, warm OS cache); compile is Darkmatter's
`ValidatorCache::structural_validator_for` on a fresh cache per iteration
(includes its content hash). This is recorded evidence only; no test
asserts a time.

### Wave 8A: Part A, Measurement and Validation Waste

**Benchmark (criterion 13).** `darkmatter/lib/benches/schema_validation.rs`
gained `schema_validation/claudine_cold/{no_trigger_match, document_events,
nested_sequence_group}` and matching `claudine_warm/*` cases; the existing
`corpus_1000` cases are unchanged. They validate against a frozen copy of
Claudine's nine schema files in `darkmatter/lib/benches/fixtures/claudine-schemas/`
(byte-for-byte from `bf89eeef0`, README says never refresh), copied into a
temporary repository's `schemas/` root so trigger discovery finds them as in
production; contexts are anchored there, never the process directory. Each
cold iteration calls the new `#[doc(hidden)]`
`darkmatter::markdown::schemas::reset_process_caches_for_bench()` in
`iter_batched(.., PerIteration)` setup, outside the timed work. It empties
every process-wide schema cache (shared, conversion, completion-alternative
and example validator caches; conversion-plan, format-validator,
example-artifact and pattern caches; trigger payload and embedded-trigger
memos; merge, phase and root-arm memos; the thread's matcher regexes; and,
after the waste work, the import-parse and partial memos). Before any timing
the bench asserts, by cache probe and never by time, that a cold validation
compiles and resolves payloads and a warm one does neither. Unit test
`markdown::schemas::tests::reset_process_caches::reset_process_caches_makes_the_next_validation_cold_with_the_same_verdict`.
The `recursive-schema-before` baseline was saved on today's tree before any
waste change.

**Stage timings (criterion 12, library and CLI).** New
`darkmatter/lib/src/markdown/schemas/timings.rs`: `ValidationStage`
(`discover`, `match`, `resolve`, `compile`, `validate`), `CompileCache`
(`hit`, `miss`, `not requested`), and `ValidationTimings` (`record`,
`stage`, `stages`, `total`, `unassigned`, `compile_cache`,
`matched_triggers`, `absorb`), plus `DarkmatterSchemas::validate_timed`.
A thread-local recorder attributes time to the innermost open stage, so a
validator built while resolving counts once, as `compile`; any build makes
the request a miss, lookups without a build a hit, no lookup "not
requested". Discovery runs in `with_trigger_discovery()`, so callers record
it and `absorb` it. Partial timings come back beside an unchanged error.
`md schema validate -vv` (verbosity count now passed through dispatch) prints
a `Prose` + `Table` breakdown to stderr after each document's normal output;
one `-v` and no `-v` are byte-identical to before; stdout and exit codes are
unchanged. The CLI now validates before its separate no-schema lookup so
that lookup no longer pre-warms the cache. Docs:
`darkmatter/docs/inline/schema-validation.md` ("Stage timings (`-vv`)"),
`definition.md` Key Types row.

**DMLS slow-validation event (criterion 12).** New
`darkmatter/dmls/src/validation_timing.rs`: one event per diagnostics
refresh that reached the schema lookup, `slow schema validation` at `warn`
when total ≥ `schema.slow_validation_ms`, else `schema validation complete`
at `debug` (target `dmls::validation_timing`), with `path`,
`matched_triggers`, `discover_ms` … `validate_ms`, `total_ms`,
`unassigned_ms`, `compile_cache`; never values or schema bodies.
`schema.slow_validation_ms` (default 200, zero logs every validation at
`warn`) is a plain integer in `config/mod.rs`, so invalid values follow the
existing config-error path; precedence is editor settings > `.dmls.toml` >
default. The threshold is not part of the schema cache key. Docs:
`darkmatter/docs/lsp/configuration.md`, `darkmatter/docs/lsp/indexing.md`.

**Waste items (criterion 14).**

| Hypothesis | Disposition | Evidence |
| --- | --- | --- |
| Unmatched triggers fully built | **Deferral not done** (contract), resolution made cheaper | Every payload error is reported at discovery today (`unmatched_trigger_with_missing_payload_is_hard_error`, `…non_mergeable_payload…`); the merge check reads lowered JSON, so neither resolution nor lowering can wait without hiding errors. Compilation was already deferred (no validator built for an unmatched trigger). `trigger_payload_load_errors::*` pins matched = unmatched errors for invalid YAML, unresolvable reference, payload cycle, embedded bundle. Lowering no longer re-serializes nested `anyOf` fragments through `json!` (`convert.rs` `any_of_value`), cached expansions are not cloned, import-file selection is memoized per expansion, imported files are parsed once per (path, xxHash of the bytes read; still read every time, failures never kept), shipped partials parse once per process |
| Cache key serializes the whole schema | **Already fine**, one gap fixed | `validator_for_shared` hashes a prepared schema once per `Arc` (held by `Weak`, so no address reuse). A document schema got a new `Arc` per request; it now goes through the existing merge memo |
| Same validator built twice | **Fixed** | Conversion of an invalid document compiled its own copy of the structural validator; `ConversionPlan::validator` now reads the shared cache first, read-only |
| Whole-schema `Value` clones | **Fixed where local** | `merged_trigger_schema` borrows the lowest layer; union error narrowing no longer rebuilds through `json!`; the remaining clone is inside `jsonschema` |
| DMLS registry key from a `Debug` dump | **Already fine** | The key is the installed registry's version counter; inputs are tracked by xxHash fingerprints in `overlay/registry.rs` |
| DMLS second document reaches the structural cache | **Fixed (library defect)** | `schema_digest` set `judges_files` on the substring `darkmatter-file`, which also matched the lazy, syntax-only `darkmatter-file-reference` used by the built-in baseline's `style.stylesheet`; every baseline-merged schema was therefore keyed by each document's own context and recompiled per document (the spec's ~270 ms per newly opened document). Now the quoted `"darkmatter-file"` token. The unit test `a_file_free_schema_shares_one_validator_across_contexts` asserted the old conservative keying for the reference format; it now asserts sharing (eager file and `match()` rows still context-keyed) |

**Before/after sample (criterion 14).** Same host and session as Wave 8,
release, `cargo bench … -- --baseline recursive-schema-before
'claudine|corpus'`, load average 11–12 (busy dev Mac, no subagent running):

| Case | Before | After | Change |
| --- | --- | --- | --- |
| `claudine_cold/no_trigger_match` | 130.4 ms | 45.1 ms | −65.7% |
| `claudine_cold/document_events` | 173.8 ms | 80.5 ms | −58.2% |
| `claudine_cold/nested_sequence_group` | 198.9 ms | 109.3 ms | −45.0% |
| `claudine_warm/*` | 1.28–1.32 ms | 1.27–1.30 ms | within noise (one +7.8% reading on `document_events` reran at +0.7%, p = 0.10) |
| `corpus_1000` cold / warm | 24.7 / 24.8 ms | 24.0 / 24.1 ms | −2.8% / −3.5% |

Causes: almost all of the cold gain is lowering (the `anyOf` re-serialization
alone took `no_trigger_match` from 130 to 85 ms) and import parsing/selection
memos. The bench constructs `DarkmatterSchemas` without the built-in
baseline, so the `judges_files` fix does not show here; its effect is a
second document in DMLS or composition reusing the compiled validator
(`compile_cache` `hit` instead of a full rebuild).

### Wave 9: Compatibility and Gates

**Golden corpus.** All 23 `darkmatter::l1 recursive_schema_golden::*` and
`recursive_schema_validator_probe::*` tests pass, and neither the 104
goldens under `darkmatter/lib/tests/fixtures/recursive_schema_golden/` nor
the golden harness has changed since `db8c6cc1c` (captured after the
prerequisites and before the recursion change; `git diff --stat db8c6cc1c`
is empty for both). Acyclic lowering is therefore still byte-identical,
key order included (full goldens compare pretty JSON text; large ones a
BLAKE3 digest of the compact text), including after this phase's lowering
change (`any_of_value` moves fragments instead of re-serializing them). No
golden was blessed. Repeated resolution (`repeated_resolution_is_byte_identical`),
absence of absolute paths in generated keys (Phase 1 self-check, Phase 2
`repeated_imports_share_one_definition_and_resolution_is_deterministic`),
collision and scope safety (Phase 2 namespace tests, Phase 3 merge tests),
and finite size (`recursive_fixture_resolves_to_at_most_two_megabytes`) are
proven by earlier phases' tests, all green in this phase's gate run.

**Consumer audit.** This phase's public API changes are additive
(`ValidationTimings`, `ValidationStage`, `CompileCache`,
`DarkmatterSchemas::validate_timed`, `#[doc(hidden)]
reset_process_caches_for_bench`). Darkmatter's reverse dependencies are
`biscuit-icon-cli`, `biscuit-speaks-cli`, `claudine`, `claudine-cli`,
`claudine-gen`, `darkmatter-cli`, `dmls`, `messenger`, `messenger-cli`,
`playa-cli`, `research`, `sniff-cli`; only Claudine, the darkmatter
crates, and `messenger` (research corpus loader) call schema validation.
Claudine's full `just test` and `messenger`'s 15 `research_corpus` tests
pass. Phase 4's `cargo check --tests` sweep of the remaining dependents
still applies (no signature changed).

**Two test defects found by the gates and fixed:**

- `dmls::l1 searched_candidates::a_watched_nearer_candidate_reselects_the_import_and_its_removal_falls_back`
  failed 1 in 3 full DMLS runs (watch registrations 3 vs 2), as two earlier
  features had recorded ("load-sensitive, pre-existing"). Printing the
  registrations showed the cause: after the nearer file is created, the
  diagnostics refresh and the registry worker each sync the watch set, in
  either order, so an intermediate registration (the new folder listed both
  present and absent) can precede the final one. The test asserted exactly
  one re-registration; it now asserts at least one and still checks the
  final set's contents. Six consecutive full DMLS runs passed afterwards
  (Decision 9).
- The DMLS timing tests were first written in a module named
  `slow_validation_log`. A `slow_` segment takes a test out of the local L1
  filter (darkmatter's CI L1 keeps them), so `just test` never ran the 11
  tests, and native Windows showed two of them failing. Renamed to
  `validation_timing_log`; the two Windows failures were the test's own
  `root.join("one/doc.md")` (a literal `/` survives on Windows, see the `os`
  skill's `windows-paths.md` item 8) and now use `join("one").join("doc.md")`
  (Decision 8).

### Wave 10: Review Handoff

Final-diff review over this phase's files for the plan's listed defects: no
new silent missing-reference fallback, global visited-definition set (the
walkers guard `(schema, instance)` pairs, Phase 4 Decision 3), dropped
sibling, owning `Arc` cycle, use-site contamination, depth-limited
conversion, reporting-driven mutation, scope-changing rewrite, or
invalid-element filtering was added; a grep of added lines for
`unwrap_or_default()`, `.ok()`, `filter_map(`, `#[serde(default)]`, and
`visited` finds only Phase 4's reviewed reporting-path lookups, the
existing conversion-cache `.ok()` fallback, and a timing default. Comment
drift fixed in this phase: `SchemaDigest::judges_files` and the
`a_file_free_schema_shares_one_validator_across_contexts` doc now describe
the eager-only rule; the bench's `FROZEN_EVENTS` doc said "seven" for six
keys. No unrelated edits, dependency upgrades, CI scheduling change, or
production Claudine schema migration entered this phase (`claudine/schemas/`
is untouched; the bench reads a frozen copy). Deviation from the plan: Part
A (criteria 12–15) was added as Wave 8A (Decision 1). The spec is left as
written.

### Design Decisions

1. **Part A is delivered in this phase as Wave 8A.**
   - Alternatives considered: leave it unimplemented and report the gap
     (drops requested scope); file it as a separate spec (defers requested
     scope).
   - Why: the spec asks for it (criteria 12–15) and this is the plan's last
     phase; the plan's omission is a drafting defect, not a decision. The
     spec meant Part A to land first; its "before" baseline is therefore
     taken on today's tree (Part B already landed), which is still before
     every Part A waste change.
   - Downsides: the before/after sample measures Part A's waste changes
     only; Part B's effect on these benchmark cases is not separated out.
2. **Wave 8's medians come from Criterion's ten-sample minimum, not exactly
   five resolves.**
   - Alternatives considered: a hand-rolled five-iteration timer.
   - Why: the plan asks for at least a five-warm-resolve median on one host;
     Criterion's ten samples give that with its warm-up and keep the cases in
     the existing `just bench-schema` recipe that Part A extends.
   - Downsides: the unrolled case takes about 16 s per run.
3. **Stage timing uses a thread-local recorder with stage markers.**
   - Alternatives considered: pass a timings value through the validation
     call graph (many signature changes); store discovery time on
     `DarkmatterSchemas` (a reused wrapper would report the same scan for
     every document).
   - Why: validation is synchronous on the calling thread; the innermost
     open stage takes the time, so compile is never counted twice.
   - Downsides: work handed to another thread is not seen; callers record
     discovery themselves and `absorb` it.
4. **Unmatched trigger payloads are still resolved at discovery.**
   - Alternatives considered: defer payload resolution or lowering until a
     match (the spec's first hypothesis).
   - Why: the spec permits deferral only where error behavior is unchanged;
     every payload error is reported at discovery today and the merge check
     needs the lowered JSON. Resolution was made cheaper instead.
   - Downsides: a document no trigger matches still pays payload resolution
     once per process (45 ms cold for Claudine's bundle on the dev Mac).
5. **Imported schema files are parsed once per (path, xxHash of the bytes
   read), and still read on every use.**
   - Alternatives considered: key on modification time; skip the read.
   - Why: any edit, deletion, or nearer new file is seen; failures are
     never stored (spec: a failed dependency read must not serve a cached
     success). Hashing uses `biscuit-hash`.
   - Downsides: one read and one hash per import; the memo holds at most
     256 files before starting over.
6. **`judges_files` matches the quoted `"darkmatter-file"` token.**
   - Alternatives considered: walk the schema for `format` keys (more code
     for the same answer); leave it (every DMLS/compose document recompiles).
   - Why: the lazy `darkmatter-file-reference` validator checks syntax only
     and reads no context, and an eager format value always serializes as
     that quoted string, so there are no false negatives.
   - Downsides: a description whose text is exactly `darkmatter-file` in
     quotes still selects the context path (costs a validator, never a
     wrong verdict).
7. **DMLS times validation in measured parts on the dispatch thread, and a
   schema DMLS serves from its own cache reports `hit`.**
   - Alternatives considered: one recording around the whole diagnostics
     refresh (a slow wiki or DSL provider would raise "slow schema
     validation"); report `not requested` for a reused schema (true of the
     library call, misleading to the reader).
   - Why: the event should describe schema validation only; the compiled
     validators really were reused.
   - Downsides: `total_ms` excludes overlay hashing and other providers;
     trigger-registry preparation (worker thread, shared across documents)
     has its own existing log line and is not attributed to one document.
8. **The DMLS timing test module is `validation_timing_log`, not
   `slow_validation_log`.**
   - Alternatives considered: keep the name and rely on CI's slow-inclusive
     L1.
   - Why: the tests are not slow (about 0.3 s); a `slow_` segment hid them
     from every local `just test`, which is how a Windows-only failure went
     unseen.
   - Downsides: none.
9. **`a_watched_nearer_candidate_reselects_the_import_and_its_removal_falls_back`
   asserts "re-registered at least once".**
   - Alternatives considered: record it as load-sensitive a third time;
     make DMLS coalesce the two syncs (a production change outside this
     feature, for a registration the client simply replaces).
   - Why: the extra registration is a correct intermediate state; the test,
     not the server, assumed an order between two threads. The final watch
     set is still asserted.
   - Downsides: the test no longer detects a redundant re-registration.
10. **A schema using only the lazy `darkmatter-file-reference` format now
    shares one validator across contexts** (the existing unit test's row
    for that format moved from "context-keyed" to "shared"). See Decision 6.

### Requirement-to-test mapping (Phase 6 and acceptance audit)

Criteria 1–11 were mapped in Phases 1–5's tables (each phase's
"Requirement-to-test mapping"); every test named there passed in this
phase's gate runs (darkmatter 11,930/11,930). Summary by criterion, with this
phase's additions:

| Criterion | Evidence |
| --- | --- |
| 1 Byte-identical golden corpus | `recursive_schema_golden::*` (23 with the probe), goldens unchanged since `db8c6cc1c` (Wave 9) |
| 2 Recursive validation depth ≥ 6, path + line/column | Phase 2 `a_self_recursive_type_validates_depth_eight_and_positions_a_bad_leaf`, `a_two_file_cycle_validates_depth_six_and_records_both_files`; Phase 5 CLI `schema_recursive::*seven_levels_deep*` |
| 3 DMLS levels 1–3, lazy | Phase 5 `recursive_schema_editor::*` (completion, hover, laziness) |
| 4 Frozen fixture ≤ 2 MB, depth invariance, measurement | `recursive_schema_frozen_fixture::*`; Wave 8 table (15.4 ms vs 1,561 ms; I/O and compile separate) |
| 5 Per-walker `$ref` coverage | Phase 4 Wave 4 tables (`recursive_schema_clean`, `_rewrite`, `_conversion`, `_reporting`, coercion conformance, discriminant, synthetic validators) |
| 6 Presence and finite leaf | Phase 2 `required_children_accept_an_empty_leaf_and_reject_missing_or_null_at_depth_four` and use-site tests; Phase 5 CLI required/optional tests |
| 7 Generator portability | `recursive_schema_frozen_fixture::{generator_names_no_absolute_path, generator_writes_beside_itself_from_an_unrelated_cwd}` |
| 8 Graph and merge safety | Phase 2 graph tests; Phase 3 `recursive_schema_merge::*` (namespaces, re-keying, literal data, raw arms, embedded, cache invalidation via cycle-closing import) |
| 9 Phase, conversion, normal invocation | Phase 3 phase tests; Phase 4 conversion past cutoff; Phase 5 `schema_recursive::*` (`md schema validate`, `about` per Phase 5 Decision 4, compose, idempotence) |
| 10 Docs and consumer audit | Phase 2–5 doc edits; Phase 4 and Wave 9 consumer audits |
| 11 Child-value boundary rule | Phase 2 `every_descending_position_closes_a_cycle`, alias/self-union/callable rejections, mixed group, two-file spans; DMLS `named_type_references` and Phase 5 alias-cycle editor tests |
| 12 Stage timings (library, CLI, DMLS) | darkmatter `schema_validation_timings::{second_validation_of_a_document_is_a_full_compile_cache_hit, one_cached_arm_and_one_new_arm_is_a_compile_miss, a_document_without_any_schema_requests_no_compilation, an_unresolvable_schema_keeps_its_error_and_reports_partial_timings, a_trigger_matched_document_names_its_triggers_and_discovery_is_recorded, timed_and_untimed_validation_report_the_same_verdict}`, unit `timings::tests::*`; darkmatter-cli `schema_validate_timings::{double_verbose_adds_a_stage_report_to_stderr_only, single_verbose_output_is_unchanged, json_stdout_is_unchanged_under_double_verbose, unresolvable_schema_keeps_its_error_and_reports_partial_timings}`; dmls `validation_timing_log::{a_zero_threshold_logs_every_validation_at_warn, a_validation_below_the_threshold_logs_at_debug, the_editor_setting_overrides_the_file_threshold, a_document_without_a_schema_requests_no_compilation, a_request_mixing_cached_and_built_validators_is_a_miss, an_unresolvable_schema_still_logs_its_partial_timings, the_threshold_defaults_to_200_ms, the_editor_threshold_overrides_the_file_and_null_settings_restore_it, an_invalid_threshold_is_a_config_error_and_keeps_the_lower_layer}`, unit `validation_timing::tests::*` |
| 13 Benchmark | `schema_validation/claudine_cold/*` and `claudine_warm/*` over `lib/benches/fixtures/claudine-schemas/`; the bench's cache-probe precheck (cold compiles and resolves, warm does neither); `tests::reset_process_caches::reset_process_caches_makes_the_next_validation_cold_with_the_same_verdict` |
| 14 Recorded results | Wave 8A waste table and before/after table; all existing Darkmatter and Claudine schema tests pass unchanged (except the one unit-test row of Decision 10) |
| 15 Cache and trigger behavior | darkmatter `schema_cache_validity::*` (12, including `documents_in_their_own_contexts_reuse_a_validator_over_the_baseline`, which fails with the old `judges_files` substring), `trigger_payload_load_errors::*` (4, matched = unmatched); dmls `validation_timing_log::{a_second_document_with_the_same_structural_schema_hits_and_keeps_its_positions, a_file_arm_below_a_union_keeps_each_documents_context}` |

Known gap against criterion 15: `match()` below a recursive `$ref` does not
take part in union-arm selection at all (only top-level arm properties carry
it; existing behavior, not a cache issue), so its cache test uses a named
type; file formats below a `$ref` are covered.

Input robustness matrix: the one new configuration field,
`schema.slow_validation_ms`, is a plain integer through DMLS's existing
config validator; covered shapes: absent (default 200), editor `null`
(restores the lower layer, the existing settings rule), wrong type /
negative / decimal (existing config error, lower layer kept), and the
control. No new file loader. Smell check: no `#[serde(default)]` on it beyond
the existing settings struct's default mechanism, no absence-collapsing
`Option`.

### Gates

- Scope: `git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`
  after every Phase 6 edit → `packages: ["claudine", "darkmatter", "darkmatter-cli", "dmls", "test-toolkit", "claudine-cli"]`,
  `change_class: "package"` (the working tree still carries Phases 4–6), so
  no gate was skipped.
- `darkmatter` `just test`: **11,930/11,930** passed (10 skipped by tier)
  after the two test fixes above; earlier runs stopped on
  `validate::tests::a_file_free_schema_shares_one_validator_across_contexts`
  (Decision 10) and on the `searched_candidates` race (Decision 9).
  `just lint` passed (Zed WASM check included).
  `just check-tier-coverage darkmatter` → 0 stranded.
- `claudine` `just test`: **10,692/10,692** passed; `just lint` passed;
  `just check-tier-coverage claudine` → 0 stranded.
- Dependents: `messenger` `research_corpus` 15/15.
- Cross-OS, native Windows (`just cross-check`): `darkmatter`
  `schema_cache_validity schema_validation_timings trigger_payload_load_errors
  reset_process_caches recursive_schema_golden` 40/40; `darkmatter-cli`
  `schema_validate_timings` 4/4; `dmls` `validation_timing validation_timing_log
  searched_candidates` pass (validation_timing_log 11/11 on the second
  attempt, after the path-join fix).
- Timing evidence (Wave 8, Wave 8A) is kept apart from correctness evidence
  and asserted by no test.

## Pipeline Refresh

This spec is pipeline step 19. Downstream active specs not yet implemented
(neither plan has started):

- `2026-10-30-claudine-recursive-schemas` (step 20): **changed** (spec and
  plan). The prerequisites bullet now lists what was delivered; a new
  decisions entry records the descending-edge rule (`NonDescendingCycle`),
  reuse of the existing 2 MB size test, proving the unroll is gone by type
  and `DefKey` names (collision keys like `item-2` are legal), leaving the
  spike files and generator untouched (L1 tests pin them), sampling with
  `md schema validate -vv`, that trigger payloads are still resolved at
  discovery, and no new depth limit. The plan had tracked only criteria 1–9
  (written before the spec's 2026-10-16 amendment); its waves and checkpoint
  now cover all eleven.
- `2026-10-31-type-aware-evaluation` (step 21): **changed** (spec and plan).
  Recursive variables project to their named `TypeExpr::Ref`; nested paths
  open one definition per segment; new decisions: the declared layer looks
  references up across all layers through `EffectiveSchema::definition`
  (today `DeclaredTypes::with_definitions` receives the document's table
  only), guard narrowing classifies a `Ref` arm by its definition body, and a
  raw `$ref` projects as `unknown`. Wave 1's dependency audit lists the
  recursive seams; Wave 2, 3 and 5 tasks were adjusted to match.

No critical decision for the author.
