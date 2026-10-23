---
spec: "/Volumes/coding/wt/rusty-biscuit/feat-pipeline/claudine/features/2026-10-28-schema-enhancements/spec.md"
plan: "claudine/features/2026-10-28-schema-enhancements/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
source_files_during_phase_1:
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/schema_yaml_spellings.rs
    - darkmatter/lib/tests/l1/schema_enhancements_matrix/mod.rs
    - darkmatter/lib/tests/l1/schema_enhancements_matrix/envelope.rs
    - darkmatter/lib/tests/l1/schema_enhancements_matrix/raw_schema.rs
    - darkmatter/lib/tests/l1/schema_enhancements_matrix/frontmatter.rs
    - darkmatter/lib/tests/fixtures/schema_enhancements_matrix/detected.schema.json
    - darkmatter/lib/tests/fixtures/schema_enhancements_matrix/detected.schema.yaml
    - darkmatter/lib/tests/fixtures/schema_enhancements_matrix/frontmatter.md-get.yaml
    - darkmatter/lib/tests/fixtures/schema_enhancements_matrix/frontmatter.schema.yaml
    - darkmatter/lib/tests/fixtures/schema_enhancements_matrix/keywords.schema.json
    - darkmatter/lib/tests/fixtures/schema_enhancements_matrix/keywords.schema.yaml
    - darkmatter/lib/tests/fixtures/schema_enhancements_matrix/slash-commands.shipped.yaml
    - darkmatter/lib/tests/fixtures/schema_enhancements_matrix/timing.shipped.yaml
    - claudine/features/2026-10-28-schema-enhancements/spikes/numeric-ingress/src/main.rs
    - claudine/features/2026-10-28-schema-enhancements/spikes/numeric-ingress/src/Cargo.toml
    - claudine/features/2026-10-28-schema-enhancements/spikes/numeric-ingress/src/Cargo.lock
docs_updated_during_phase_1: []
docs_created_during_phase_1:
    - claudine/features/2026-10-28-schema-enhancements/inventory/criteria-map.md
    - claudine/features/2026-10-28-schema-enhancements/inventory/prerequisites.md
    - claudine/features/2026-10-28-schema-enhancements/inventory/function-registry.md
    - claudine/features/2026-10-28-schema-enhancements/inventory/ownership.md
    - claudine/features/2026-10-28-schema-enhancements/inventory/reader-matrix.md
    - claudine/features/2026-10-28-schema-enhancements/inventory/rulings.md
    - claudine/features/2026-10-28-schema-enhancements/behavior-ledger.md
    - claudine/features/2026-10-28-schema-enhancements/spikes/numeric-ingress/findings.md
    - claudine/features/2026-10-28-schema-enhancements/spikes/numeric-ingress/runs/default.md
    - claudine/features/2026-10-28-schema-enhancements/spikes/numeric-ingress/runs/arbitrary-precision.md
    - claudine/features/2026-10-28-schema-enhancements/spikes/numeric-ingress/runs/preserve-order.md
    - darkmatter/lib/tests/fixtures/schema_enhancements_matrix/PROVENANCE.md
skills_files_updated_during_phase_1: []
packages:
    - darkmatter
    - darkmatter-cli
    - dmls
    - claudine
    - claudine-gen
    - sniff-cli
    - claudine-cli
    - test-toolkit
human_review: false
source_files_during_phase_2:
    - claudine/gen/src/schema_compat.rs
    - claudine/lib/src/composition/lifecycle/tests/action_shape_control.rs
    - claudine/lib/src/composition/schema/classify.rs
    - claudine/lib/src/opencode_config.rs
    - claudine/lib/src/provider/tests.rs
    - darkmatter/cli/src/commands/compose.rs
    - darkmatter/cli/src/commands/schema/assignment.rs
    - darkmatter/cli/src/commands/schema/validate.rs
    - darkmatter/cli/tests/l1/compose_state_set.rs
    - darkmatter/cli/tests/l1/main.rs
    - darkmatter/cli/tests/l1/schema_grammar_types.rs
    - darkmatter/dmls/src/diagnostics/named_types.rs
    - darkmatter/dmls/src/overlay/named_types.rs
    - darkmatter/dmls/src/overlay/suggestions.rs
    - darkmatter/dmls/src/providers/frontmatter.rs
    - darkmatter/dmls/src/providers/frontmatter/named_references.rs
    - darkmatter/dmls/tests/l1/named_type_references.rs
    - darkmatter/dmls/tests/l1/schema_yaml_spellings.rs
    - darkmatter/lib/Cargo.toml
    - darkmatter/lib/src/lifecycle/actions.rs
    - darkmatter/lib/src/markdown/compose/context/catalog.rs
    - darkmatter/lib/src/markdown/compose/context/effective_state.rs
    - darkmatter/lib/src/markdown/compose/expression/ast.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/lexer.rs
    - darkmatter/lib/src/markdown/compose/expression/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/scope/projection.rs
    - darkmatter/lib/src/markdown/compose/expression/scope/shape.rs
    - darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs
    - darkmatter/lib/src/markdown/compose/replacement.rs
    - darkmatter/lib/src/markdown/compose/transclusion/parser.rs
    - darkmatter/lib/src/markdown/frontmatter.rs
    - darkmatter/lib/src/markdown/hash/mod.rs
    - darkmatter/lib/src/markdown/mod.rs
    - darkmatter/lib/src/markdown/numeric.rs
    - darkmatter/lib/src/markdown/numeric/ingress.rs
    - darkmatter/lib/src/markdown/schemas/coerce.rs
    - darkmatter/lib/src/markdown/schemas/completion.rs
    - darkmatter/lib/src/markdown/schemas/detect.rs
    - darkmatter/lib/src/markdown/schemas/frontmatter_shape.rs
    - darkmatter/lib/src/markdown/schemas/mod.rs
    - darkmatter/lib/src/markdown/schemas/phase.rs
    - darkmatter/lib/src/markdown/schemas/resolve.rs
    - darkmatter/lib/src/markdown/schemas/simplified/convert.rs
    - darkmatter/lib/src/markdown/schemas/simplified/grammar.rs
    - darkmatter/lib/src/markdown/schemas/simplified/lint.rs
    - darkmatter/lib/src/markdown/schemas/simplified/mod.rs
    - darkmatter/lib/src/markdown/schemas/simplified/query.rs
    - darkmatter/lib/src/markdown/schemas/simplified/serialize.rs
    - darkmatter/lib/src/markdown/schemas/simplified/signature.rs
    - darkmatter/lib/src/markdown/schemas/simplified/source.rs
    - darkmatter/lib/src/markdown/schemas/simplified/standalone.rs
    - darkmatter/lib/src/markdown/schemas/simplified/subtype.rs
    - darkmatter/lib/src/markdown/schemas/simplified/types.rs
    - darkmatter/lib/src/markdown/schemas/triggers/matcher.rs
    - darkmatter/lib/src/markdown/schemas/validate.rs
    - darkmatter/lib/tests/fixtures/schema_enhancements_matrix/categories.documented.yaml
    - darkmatter/lib/tests/l1/expression_type_scope.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/meta_schema_phase6.rs
    - darkmatter/lib/tests/l1/numeric_admission.rs
    - darkmatter/lib/tests/l1/schema_enhancements_matrix/envelope.rs
    - darkmatter/lib/tests/l1/schema_enhancements_matrix/frontmatter.rs
    - darkmatter/lib/tests/l1/schema_enhancements_matrix/raw_schema.rs
    - darkmatter/lib/tests/l1/schema_grammar_descriptors.rs
    - darkmatter/lib/tests/l1/schema_yaml_spellings.rs
    - darkmatter/lib/tests/l1/schemas_literal_expression.rs
    - darkmatter/lib/tests/l1/schemas_union_postfix.rs
    - darkmatter/lib/tests/l1/shipped_schema_corpus.rs
    - darkmatter/lib/tests/l1/snapshots/l1__schema_grammar_descriptors__snapshot_empty_tuple_and_category_annotation.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schema_grammar_descriptors__snapshot_fixed_tuple.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schema_grammar_descriptors__snapshot_union_operator.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schema_grammar_descriptors__snapshot_variadic_tuple_with_optional_position.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_array_string_with_pattern_and_array_constraints.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_array_with_description.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_atom_any.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_atom_boolean.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_atom_boolish.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_atom_date.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_atom_datetime.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_atom_email.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_atom_enum_members.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_atom_file_array_min_items.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_atom_file_bare.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_atom_file_match.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_atom_number_integer_with_min.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_atom_number.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_atom_numberlike.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_atom_object.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_atom_string_min_max.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_atom_string_not_empty.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_atom_string_pattern.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_atom_string.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_atom_time.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_atom_url_bare.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_atom_url_scheme.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_atom_with_description.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_default_and_required_atoms.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_enum_with_default_and_required.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_full_document_schema.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_inline_object_array.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_inline_object_as_union_arm.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_inline_object_bare.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_inline_object_constrained_array.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_inline_object_empty.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_inline_object_mixed_with_string_fallback.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_inline_object_multi_line.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_inline_object_nested.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_inline_object_required_value.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_property_union_basic.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_property_union_hoists_matching_default.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_property_union_hoists_required.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_root_union_inline_only.snap
    - darkmatter/lib/tests/l1/snapshots/l1__suggest_constraint_phase3__conversion_snapshot_covers_valid_and_invalid_metadata.snap
    - sniff/cli/src/commands/filesystem_query.rs
    - sniff/cli/src/output/mod.rs
    - sniff/cli/tests/l1/snapshots.rs
docs_updated_during_phase_2:
    - darkmatter/docs/topics/darkmatter-expressions.md
    - darkmatter/docs/topics/schemas/definition.md
    - darkmatter/docs/topics/schemas/dmls-schema-support.md
    - darkmatter/docs/dependencies.md
    - docs/dependencies.md
    - darkmatter/lib/tests/fixtures/schema_enhancements_matrix/PROVENANCE.md
docs_created_during_phase_2:
    - darkmatter/docs/topics/numbers.md
skills_files_updated_during_phase_2:
    - .claude/skills/darkmatter/expressions.md
    - .claude/skills/darkmatter/frontmatter.md
    - .claude/skills/darkmatter/schema.md
source_files_during_phase_3:
    - darkmatter/lib/src/markdown/cleanup/rules.rs
    - darkmatter/lib/src/markdown/coercion/engine.rs
    - darkmatter/lib/src/markdown/coercion/error.rs
    - darkmatter/lib/src/markdown/coercion/mod.rs
    - darkmatter/lib/src/markdown/coercion/shape.rs
    - darkmatter/lib/src/markdown/coercion/union.rs
    - darkmatter/lib/src/markdown/compose/expression/call_binding/bind.rs
    - darkmatter/lib/src/markdown/compose/expression/call_binding/error.rs
    - darkmatter/lib/src/markdown/compose/expression/call_binding/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/call_binding/registration.rs
    - darkmatter/lib/src/markdown/compose/expression/call_binding/signature.rs
    - darkmatter/lib/src/markdown/compose/expression/call_binding/translate.rs
    - darkmatter/lib/src/markdown/compose/expression/mod.rs
    - darkmatter/lib/src/markdown/compose/schema_validation.rs
    - darkmatter/lib/src/markdown/mod.rs
    - darkmatter/lib/src/markdown/schemas/about.rs
    - darkmatter/lib/src/markdown/schemas/coerce.rs
    - darkmatter/lib/src/markdown/schemas/format.rs
    - darkmatter/lib/src/markdown/schemas/json_shape.rs
    - darkmatter/lib/src/markdown/schemas/mod.rs
    - darkmatter/lib/src/markdown/schemas/resolve.rs
    - darkmatter/lib/src/markdown/schemas/rewrite.rs
    - darkmatter/lib/src/markdown/schemas/simplified/convert.rs
    - darkmatter/lib/src/markdown/schemas/tests/mod.rs
    - darkmatter/lib/src/markdown/schemas/validate.rs
    - darkmatter/lib/src/style/parse.rs
    - darkmatter/schemas/partials/style.yaml
    - darkmatter/lib/tests/fixtures/schema_enhancements_matrix/frontmatter.conversion.schema.yaml
    - darkmatter/lib/tests/l1/base_schema_end_to_end.rs
    - darkmatter/lib/tests/l1/clean_rules.rs
    - darkmatter/lib/tests/l1/coercion_core.rs
    - darkmatter/lib/tests/l1/coercion_engine_parity.rs
    - darkmatter/lib/tests/l1/coercion_translator_conformance.rs
    - darkmatter/lib/tests/l1/frontmatter_coercion_engine.rs
    - darkmatter/lib/tests/l1/function_call_binding.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/raw_schema_conversion.rs
    - darkmatter/lib/tests/l1/schema_enhancements_matrix/frontmatter.rs
    - darkmatter/lib/tests/l1/schema_enhancements_matrix/raw_schema.rs
    - darkmatter/lib/tests/l1/schemas_conjunctive_lowering.rs
    - darkmatter/lib/tests/l1/schemas_core_type_values.rs
    - darkmatter/lib/tests/l1/schemas_union_clean.rs
    - darkmatter/lib/tests/l1/schemas_union_coerce.rs
    - darkmatter/lib/tests/l1/schemas_union_report.rs
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_atom_boolish.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_atom_number_integer_with_min.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_atom_numberlike.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_property_union_hoists_matching_default.snap
    - darkmatter/lib/tests/l1/snapshots/l1__suggest_constraint_phase3__conversion_snapshot_covers_valid_and_invalid_metadata.snap
    - darkmatter/cli/tests/l1/compose_schema.rs
    - darkmatter/cli/tests/l1/schema_about.rs
    - darkmatter/cli/tests/l1/schema_union_pipeline.rs
    - darkmatter/dmls/tests/l1/style_markdown_parity.rs
    - claudine/lib/tests/l1/claudine_schemas.rs
docs_updated_during_phase_3:
    - darkmatter/docs/topics/schemas/definition.md
    - darkmatter/docs/inline/schema-validation.md
    - darkmatter/lib/tests/fixtures/schema_enhancements_matrix/PROVENANCE.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
    - .claude/skills/darkmatter/cleanup.md
    - .claude/skills/darkmatter/schema.md
source_files_during_phase_4:
    - claudine/cli/src/commands/context/expressions.rs
    - claudine/cli/src/commands/context/format.rs
    - claudine/cli/src/commands/context/mod.rs
    - claudine/cli/src/commands/context/tests.rs
    - claudine/cli/tests/l1/context_command.rs
    - claudine/cli/tests/l1/snapshots/l1__context_command__context_expressions_function_order_snapshot.snap
    - claudine/docs/schemas/partials/functions.yaml
    - claudine/docs/schemas/partials/mutations.yaml
    - claudine/lib/src/composition/lifecycle/tests/action_shape_control.rs
    - claudine/schemas/action.yaml
    - claudine/schemas/lifecycle.yaml
    - darkmatter/cli/src/commands/schema/about.rs
    - darkmatter/cli/tests/l1/compose_expression_failures.rs
    - darkmatter/cli/tests/l1/compose_schema_enhancements.rs
    - darkmatter/cli/tests/l1/main.rs
    - darkmatter/cli/tests/l1/schema_about.rs
    - darkmatter/dmls/src/overlay/expressions.rs
    - darkmatter/dmls/src/overlay/mod.rs
    - darkmatter/dmls/src/providers/dsl.rs
    - darkmatter/dmls/tests/l1/lsp_session.rs
    - darkmatter/docs/schemas/expression-functions.yaml
    - darkmatter/lib/Cargo.toml
    - darkmatter/lib/src/lib.rs
    - darkmatter/lib/src/lifecycle/actions/tests.rs
    - darkmatter/lib/src/lifecycle/signatures.rs
    - darkmatter/lib/src/markdown/compose/context/capture/capabilities.rs
    - darkmatter/lib/src/markdown/compose/expression/call_binding/bind.rs
    - darkmatter/lib/src/markdown/compose/expression/catalog/ast.rs
    - darkmatter/lib/src/markdown/compose/expression/catalog/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/catalog/parser.rs
    - darkmatter/lib/src/markdown/compose/expression/catalog/tests.rs
    - darkmatter/lib/src/markdown/compose/expression/error.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/agentic_cli.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/args.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/cicd.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/collections.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/composition.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/dates.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/fs.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/fs_tests.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/git.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/markdown_docs.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/network.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/paths.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/predicates.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/pull_requests.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/repository.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/shell.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/skills.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/strings.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/terminal.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/test_hooks.rs
    - darkmatter/lib/src/markdown/compose/expression/mod.rs
    - darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs
    - darkmatter/lib/src/markdown/compose/interpolation/fatality_characterization.rs
    - darkmatter/lib/src/markdown/compose/interpolation/mod.rs
    - darkmatter/lib/src/markdown/compose/preflight/collect.rs
    - darkmatter/lib/src/markdown/compose/tests/lookup_parity.rs
    - darkmatter/lib/src/markdown/compose/tests/provider_network.rs
    - darkmatter/lib/src/markdown/schemas/rewrite.rs
    - darkmatter/lib/tests/l1/coercion_engine_parity.rs
    - darkmatter/lib/tests/l1/compose_expression_failure_contract.rs
    - darkmatter/lib/tests/l1/context_function_contracts.rs
    - darkmatter/lib/tests/l1/context_functions.rs
    - darkmatter/lib/tests/l1/current_root_documentation_contract.rs
    - darkmatter/lib/tests/l1/find_files_and_try_frontmatter.rs
    - darkmatter/lib/tests/l1/function_binding_conversions.rs
    - darkmatter/lib/tests/l1/function_binding_registry.rs
    - darkmatter/lib/tests/l1/function_catalog_parity.rs
    - darkmatter/lib/tests/l1/function_dispatch.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/meta_schema_phase1.rs
    - darkmatter/lib/tests/l1/nested_composition.rs
    - darkmatter/lib/tests/l1/predict_conflicts.rs
    - darkmatter/lib/tests/l1/pure_function_contracts.rs
    - darkmatter/lib/tests/l1/shipped_schema_corpus.rs
    - darkmatter/lib/tests/l1/unknown_identifier_warning.rs
    - darkmatter/schemas/partials/function-examples/as_line_separated.yaml
    - darkmatter/schemas/partials/function-examples/as_ordered_list.yaml
    - darkmatter/schemas/partials/function-examples/as_tsv.yaml
    - darkmatter/schemas/partials/function-examples/as_unordered_list.yaml
    - darkmatter/schemas/partials/functions.yaml
    - darkmatter/schemas/partials/lifecycle.yaml
    - darkmatter/schemas/partials/md-lifecycle.yaml
    - darkmatter/schemas/partials/mutations.yaml
docs_updated_during_phase_4:
    - claudine/docs/topics/flow-control/lifecycle.md
    - claudine/docs/topics/state-management/context-variables.md
    - claudine/docs/topics/state-management/drift.md
    - claudine/docs/topics/state-management/expression-engine.md
    - darkmatter/docs/lsp/features.md
    - darkmatter/docs/topics/darkmatter-expressions.md
    - prompts/_add/add-expressions.md
    - claudine/features/2026-10-28-schema-enhancements/plan.md
    - claudine/features/2026-10-28-schema-enhancements/implementation-log.md
docs_created_during_phase_4:
    - claudine/features/2026-10-28-schema-enhancements/phase4-design.md
skills_files_updated_during_phase_4:
    - .claude/skills/claudine/cli-commands.md
    - .claude/skills/claudine/cli-reference.md
    - .claude/skills/darkmatter/compose.md
    - .claude/skills/darkmatter/library-surfaces.md
source_files_during_phase_5:
    - darkmatter/cli/tests/l1/compose_schema_enhancements.rs
    - darkmatter/dmls/src/config/mod.rs
    - darkmatter/dmls/src/diagnostics/call_check.rs
    - darkmatter/dmls/src/diagnostics/codes.rs
    - darkmatter/dmls/src/diagnostics/frontmatter.rs
    - darkmatter/dmls/src/diagnostics/mod.rs
    - darkmatter/dmls/src/providers/dsl.rs
    - darkmatter/dmls/tests/l1/call_check_agreement.rs
    - darkmatter/dmls/tests/l1/expression_call_diagnostics.rs
    - darkmatter/dmls/tests/l1/main.rs
    - darkmatter/lib/src/markdown/compose/call_precheck.rs
    - darkmatter/lib/src/markdown/compose/expression/call_binding/bind.rs
    - darkmatter/lib/src/markdown/compose/expression/call_binding/signature.rs
    - darkmatter/lib/src/markdown/compose/expression/call_check.rs
    - darkmatter/lib/src/markdown/compose/expression/error.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/test_hooks.rs
    - darkmatter/lib/src/markdown/compose/expression/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/prepared.rs
    - darkmatter/lib/src/markdown/compose/expression/scope/projection.rs
    - darkmatter/lib/src/markdown/compose/inline/interpolation.rs
    - darkmatter/lib/src/markdown/compose/mod.rs
    - darkmatter/lib/src/markdown/compose/pipeline/mod.rs
    - darkmatter/lib/src/markdown/compose/pipeline/phases.rs
    - darkmatter/lib/src/markdown/types.rs
    - darkmatter/lib/tests/l1/compose_call_precheck.rs
    - darkmatter/lib/tests/l1/compose_expression_failure_contract.rs
    - darkmatter/lib/tests/l1/design_time_call_check.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/type_scope_ownership_guard.rs
docs_updated_during_phase_5:
    - darkmatter/dmls/docs/diagnostics.md
    - darkmatter/docs/darkmatter-compose-pipeline.md
docs_created_during_phase_5:
    - darkmatter/docs/inline/call-check.md
skills_files_updated_during_phase_5:
    - .claude/skills/claudine/SKILL.md
    - .claude/skills/darkmatter/SKILL.md
    - .claude/skills/darkmatter/expressions.md
source_files_during_phase_6:
    - darkmatter/lib/src/markdown/coercion/mod.rs
    - darkmatter/lib/src/markdown/coercion/shape.rs
    - darkmatter/lib/src/markdown/compose/expression/call_binding/error.rs
    - darkmatter/lib/src/markdown/compose/expression/catalog/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/catalog/tests.rs
    - darkmatter/lib/tests/l1/function_call_binding.rs
    - darkmatter/schemas/partials/functions.yaml
    - darkmatter/dmls/src/config/mod.rs
    - darkmatter/dmls/tests/l1/type_scope_ownership_guard.rs
docs_updated_during_phase_6:
    - darkmatter/docs/topics/schemas/definition.md
    - darkmatter/docs/topics/schemas/authoring-schemas.md
    - darkmatter/docs/topics/numbers.md
    - darkmatter/docs/topics/darkmatter-expressions.md
    - darkmatter/docs/inline/call-check.md
    - darkmatter/docs/inline/schema-validation.md
    - darkmatter/README.md
    - darkmatter/docs/dependencies.md
    - docs/dependencies.md
    - darkmatter/dmls/docs/diagnostics.md
    - darkmatter/dmls/docs/features.md
    - darkmatter/dmls/docs/autocomplete.md
    - darkmatter/dmls/docs/hover.md
    - darkmatter/dmls/docs/editors/README.md
    - darkmatter/dmls/docs/editors/smoke-checklist.md
    - darkmatter/dmls/README.md
    - darkmatter/docs/lsp/configuration.md
    - darkmatter/docs/lsp/architecture.md
    - darkmatter/docs/lsp/features.md
    - claudine/docs/topics/state-management/expression-engine.md
    - claudine/docs/topics/state-management/drift.md
    - claudine/README.md
    - claudine/features/2026-10-28-schema-enhancements/inventory/criteria-map.md
    - claudine/features/2026-10-28-schema-enhancements/behavior-ledger.md
docs_created_during_phase_6: []
skills_files_updated_during_phase_6:
    - .claude/skills/darkmatter/SKILL.md
    - .claude/skills/darkmatter/schema.md
    - .claude/skills/darkmatter/expressions.md
    - .claude/skills/darkmatter/compose.md
    - .claude/skills/darkmatter/dmls.md
    - .claude/skills/claudine/SKILL.md
    - .claude/skills/claudine/cli-reference.md
source_code:
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/schema_yaml_spellings.rs
    - darkmatter/lib/tests/l1/schema_enhancements_matrix/mod.rs
    - darkmatter/lib/tests/l1/schema_enhancements_matrix/envelope.rs
    - darkmatter/lib/tests/l1/schema_enhancements_matrix/raw_schema.rs
    - darkmatter/lib/tests/l1/schema_enhancements_matrix/frontmatter.rs
    - darkmatter/lib/tests/fixtures/schema_enhancements_matrix/detected.schema.json
    - darkmatter/lib/tests/fixtures/schema_enhancements_matrix/detected.schema.yaml
    - darkmatter/lib/tests/fixtures/schema_enhancements_matrix/frontmatter.md-get.yaml
    - darkmatter/lib/tests/fixtures/schema_enhancements_matrix/frontmatter.schema.yaml
    - darkmatter/lib/tests/fixtures/schema_enhancements_matrix/keywords.schema.json
    - darkmatter/lib/tests/fixtures/schema_enhancements_matrix/keywords.schema.yaml
    - darkmatter/lib/tests/fixtures/schema_enhancements_matrix/slash-commands.shipped.yaml
    - darkmatter/lib/tests/fixtures/schema_enhancements_matrix/timing.shipped.yaml
    - claudine/features/2026-10-28-schema-enhancements/spikes/numeric-ingress/src/main.rs
    - claudine/features/2026-10-28-schema-enhancements/spikes/numeric-ingress/src/Cargo.toml
    - claudine/features/2026-10-28-schema-enhancements/spikes/numeric-ingress/src/Cargo.lock
    - claudine/gen/src/schema_compat.rs
    - claudine/lib/src/composition/lifecycle/tests/action_shape_control.rs
    - claudine/lib/src/composition/schema/classify.rs
    - claudine/lib/src/opencode_config.rs
    - claudine/lib/src/provider/tests.rs
    - darkmatter/cli/src/commands/compose.rs
    - darkmatter/cli/src/commands/schema/assignment.rs
    - darkmatter/cli/src/commands/schema/validate.rs
    - darkmatter/cli/tests/l1/compose_state_set.rs
    - darkmatter/cli/tests/l1/main.rs
    - darkmatter/cli/tests/l1/schema_grammar_types.rs
    - darkmatter/dmls/src/diagnostics/named_types.rs
    - darkmatter/dmls/src/overlay/named_types.rs
    - darkmatter/dmls/src/overlay/suggestions.rs
    - darkmatter/dmls/src/providers/frontmatter.rs
    - darkmatter/dmls/src/providers/frontmatter/named_references.rs
    - darkmatter/dmls/tests/l1/named_type_references.rs
    - darkmatter/dmls/tests/l1/schema_yaml_spellings.rs
    - darkmatter/lib/Cargo.toml
    - darkmatter/lib/src/lifecycle/actions.rs
    - darkmatter/lib/src/markdown/compose/context/catalog.rs
    - darkmatter/lib/src/markdown/compose/context/effective_state.rs
    - darkmatter/lib/src/markdown/compose/expression/ast.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/lexer.rs
    - darkmatter/lib/src/markdown/compose/expression/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/scope/projection.rs
    - darkmatter/lib/src/markdown/compose/expression/scope/shape.rs
    - darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs
    - darkmatter/lib/src/markdown/compose/replacement.rs
    - darkmatter/lib/src/markdown/compose/transclusion/parser.rs
    - darkmatter/lib/src/markdown/frontmatter.rs
    - darkmatter/lib/src/markdown/hash/mod.rs
    - darkmatter/lib/src/markdown/mod.rs
    - darkmatter/lib/src/markdown/numeric.rs
    - darkmatter/lib/src/markdown/numeric/ingress.rs
    - darkmatter/lib/src/markdown/schemas/coerce.rs
    - darkmatter/lib/src/markdown/schemas/completion.rs
    - darkmatter/lib/src/markdown/schemas/detect.rs
    - darkmatter/lib/src/markdown/schemas/frontmatter_shape.rs
    - darkmatter/lib/src/markdown/schemas/mod.rs
    - darkmatter/lib/src/markdown/schemas/phase.rs
    - darkmatter/lib/src/markdown/schemas/resolve.rs
    - darkmatter/lib/src/markdown/schemas/simplified/convert.rs
    - darkmatter/lib/src/markdown/schemas/simplified/grammar.rs
    - darkmatter/lib/src/markdown/schemas/simplified/lint.rs
    - darkmatter/lib/src/markdown/schemas/simplified/mod.rs
    - darkmatter/lib/src/markdown/schemas/simplified/query.rs
    - darkmatter/lib/src/markdown/schemas/simplified/serialize.rs
    - darkmatter/lib/src/markdown/schemas/simplified/signature.rs
    - darkmatter/lib/src/markdown/schemas/simplified/source.rs
    - darkmatter/lib/src/markdown/schemas/simplified/standalone.rs
    - darkmatter/lib/src/markdown/schemas/simplified/subtype.rs
    - darkmatter/lib/src/markdown/schemas/simplified/types.rs
    - darkmatter/lib/src/markdown/schemas/triggers/matcher.rs
    - darkmatter/lib/src/markdown/schemas/validate.rs
    - darkmatter/lib/tests/fixtures/schema_enhancements_matrix/categories.documented.yaml
    - darkmatter/lib/tests/l1/expression_type_scope.rs
    - darkmatter/lib/tests/l1/meta_schema_phase6.rs
    - darkmatter/lib/tests/l1/numeric_admission.rs
    - darkmatter/lib/tests/l1/schema_grammar_descriptors.rs
    - darkmatter/lib/tests/l1/schemas_literal_expression.rs
    - darkmatter/lib/tests/l1/schemas_union_postfix.rs
    - darkmatter/lib/tests/l1/shipped_schema_corpus.rs
    - darkmatter/lib/tests/l1/snapshots/l1__schema_grammar_descriptors__snapshot_empty_tuple_and_category_annotation.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schema_grammar_descriptors__snapshot_fixed_tuple.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schema_grammar_descriptors__snapshot_union_operator.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schema_grammar_descriptors__snapshot_variadic_tuple_with_optional_position.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_array_string_with_pattern_and_array_constraints.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_array_with_description.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_atom_any.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_atom_boolean.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_atom_boolish.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_atom_date.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_atom_datetime.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_atom_email.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_atom_enum_members.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_atom_file_array_min_items.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_atom_file_bare.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_atom_file_match.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_atom_number_integer_with_min.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_atom_number.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_atom_numberlike.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_atom_object.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_atom_string_min_max.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_atom_string_not_empty.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_atom_string_pattern.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_atom_string.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_atom_time.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_atom_url_bare.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_atom_url_scheme.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_atom_with_description.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_default_and_required_atoms.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_enum_with_default_and_required.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_full_document_schema.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_inline_object_array.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_inline_object_as_union_arm.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_inline_object_bare.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_inline_object_constrained_array.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_inline_object_empty.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_inline_object_mixed_with_string_fallback.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_inline_object_multi_line.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_inline_object_nested.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_inline_object_required_value.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_property_union_basic.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_property_union_hoists_matching_default.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_property_union_hoists_required.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_root_union_inline_only.snap
    - darkmatter/lib/tests/l1/snapshots/l1__suggest_constraint_phase3__conversion_snapshot_covers_valid_and_invalid_metadata.snap
    - sniff/cli/src/commands/filesystem_query.rs
    - sniff/cli/src/output/mod.rs
    - sniff/cli/tests/l1/snapshots.rs
    - darkmatter/lib/src/markdown/cleanup/rules.rs
    - darkmatter/lib/src/markdown/coercion/engine.rs
    - darkmatter/lib/src/markdown/coercion/error.rs
    - darkmatter/lib/src/markdown/coercion/mod.rs
    - darkmatter/lib/src/markdown/coercion/shape.rs
    - darkmatter/lib/src/markdown/coercion/union.rs
    - darkmatter/lib/src/markdown/compose/expression/call_binding/bind.rs
    - darkmatter/lib/src/markdown/compose/expression/call_binding/error.rs
    - darkmatter/lib/src/markdown/compose/expression/call_binding/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/call_binding/registration.rs
    - darkmatter/lib/src/markdown/compose/expression/call_binding/signature.rs
    - darkmatter/lib/src/markdown/compose/expression/call_binding/translate.rs
    - darkmatter/lib/src/markdown/compose/schema_validation.rs
    - darkmatter/lib/src/markdown/schemas/about.rs
    - darkmatter/lib/src/markdown/schemas/format.rs
    - darkmatter/lib/src/markdown/schemas/json_shape.rs
    - darkmatter/lib/src/markdown/schemas/rewrite.rs
    - darkmatter/lib/src/markdown/schemas/tests/mod.rs
    - darkmatter/lib/src/style/parse.rs
    - darkmatter/schemas/partials/style.yaml
    - darkmatter/lib/tests/fixtures/schema_enhancements_matrix/frontmatter.conversion.schema.yaml
    - darkmatter/lib/tests/l1/base_schema_end_to_end.rs
    - darkmatter/lib/tests/l1/clean_rules.rs
    - darkmatter/lib/tests/l1/coercion_core.rs
    - darkmatter/lib/tests/l1/coercion_engine_parity.rs
    - darkmatter/lib/tests/l1/coercion_translator_conformance.rs
    - darkmatter/lib/tests/l1/frontmatter_coercion_engine.rs
    - darkmatter/lib/tests/l1/function_call_binding.rs
    - darkmatter/lib/tests/l1/raw_schema_conversion.rs
    - darkmatter/lib/tests/l1/schemas_conjunctive_lowering.rs
    - darkmatter/lib/tests/l1/schemas_core_type_values.rs
    - darkmatter/lib/tests/l1/schemas_union_clean.rs
    - darkmatter/lib/tests/l1/schemas_union_coerce.rs
    - darkmatter/lib/tests/l1/schemas_union_report.rs
    - darkmatter/cli/tests/l1/compose_schema.rs
    - darkmatter/cli/tests/l1/schema_about.rs
    - darkmatter/cli/tests/l1/schema_union_pipeline.rs
    - darkmatter/dmls/tests/l1/style_markdown_parity.rs
    - claudine/lib/tests/l1/claudine_schemas.rs
    - claudine/cli/src/commands/context/expressions.rs
    - claudine/cli/src/commands/context/format.rs
    - claudine/cli/src/commands/context/mod.rs
    - claudine/cli/src/commands/context/tests.rs
    - claudine/cli/tests/l1/context_command.rs
    - claudine/cli/tests/l1/snapshots/l1__context_command__context_expressions_function_order_snapshot.snap
    - claudine/docs/schemas/partials/functions.yaml
    - claudine/docs/schemas/partials/mutations.yaml
    - claudine/schemas/action.yaml
    - claudine/schemas/lifecycle.yaml
    - darkmatter/cli/src/commands/schema/about.rs
    - darkmatter/cli/tests/l1/compose_expression_failures.rs
    - darkmatter/cli/tests/l1/compose_schema_enhancements.rs
    - darkmatter/dmls/src/overlay/expressions.rs
    - darkmatter/dmls/src/overlay/mod.rs
    - darkmatter/dmls/src/providers/dsl.rs
    - darkmatter/dmls/tests/l1/lsp_session.rs
    - darkmatter/docs/schemas/expression-functions.yaml
    - darkmatter/lib/src/lib.rs
    - darkmatter/lib/src/lifecycle/actions/tests.rs
    - darkmatter/lib/src/lifecycle/signatures.rs
    - darkmatter/lib/src/markdown/compose/context/capture/capabilities.rs
    - darkmatter/lib/src/markdown/compose/expression/catalog/ast.rs
    - darkmatter/lib/src/markdown/compose/expression/catalog/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/catalog/parser.rs
    - darkmatter/lib/src/markdown/compose/expression/catalog/tests.rs
    - darkmatter/lib/src/markdown/compose/expression/error.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/agentic_cli.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/args.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/cicd.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/collections.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/composition.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/dates.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/fs.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/fs_tests.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/git.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/markdown_docs.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/network.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/paths.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/predicates.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/pull_requests.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/repository.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/shell.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/skills.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/strings.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/terminal.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/test_hooks.rs
    - darkmatter/lib/src/markdown/compose/interpolation/fatality_characterization.rs
    - darkmatter/lib/src/markdown/compose/interpolation/mod.rs
    - darkmatter/lib/src/markdown/compose/preflight/collect.rs
    - darkmatter/lib/src/markdown/compose/tests/lookup_parity.rs
    - darkmatter/lib/src/markdown/compose/tests/provider_network.rs
    - darkmatter/lib/tests/l1/compose_expression_failure_contract.rs
    - darkmatter/lib/tests/l1/context_function_contracts.rs
    - darkmatter/lib/tests/l1/context_functions.rs
    - darkmatter/lib/tests/l1/current_root_documentation_contract.rs
    - darkmatter/lib/tests/l1/find_files_and_try_frontmatter.rs
    - darkmatter/lib/tests/l1/function_binding_conversions.rs
    - darkmatter/lib/tests/l1/function_binding_registry.rs
    - darkmatter/lib/tests/l1/function_catalog_parity.rs
    - darkmatter/lib/tests/l1/function_dispatch.rs
    - darkmatter/lib/tests/l1/meta_schema_phase1.rs
    - darkmatter/lib/tests/l1/nested_composition.rs
    - darkmatter/lib/tests/l1/predict_conflicts.rs
    - darkmatter/lib/tests/l1/pure_function_contracts.rs
    - darkmatter/lib/tests/l1/unknown_identifier_warning.rs
    - darkmatter/schemas/partials/function-examples/as_line_separated.yaml
    - darkmatter/schemas/partials/function-examples/as_ordered_list.yaml
    - darkmatter/schemas/partials/function-examples/as_tsv.yaml
    - darkmatter/schemas/partials/function-examples/as_unordered_list.yaml
    - darkmatter/schemas/partials/functions.yaml
    - darkmatter/schemas/partials/lifecycle.yaml
    - darkmatter/schemas/partials/md-lifecycle.yaml
    - darkmatter/schemas/partials/mutations.yaml
    - darkmatter/dmls/src/config/mod.rs
    - darkmatter/dmls/src/diagnostics/call_check.rs
    - darkmatter/dmls/src/diagnostics/codes.rs
    - darkmatter/dmls/src/diagnostics/frontmatter.rs
    - darkmatter/dmls/src/diagnostics/mod.rs
    - darkmatter/dmls/tests/l1/call_check_agreement.rs
    - darkmatter/dmls/tests/l1/expression_call_diagnostics.rs
    - darkmatter/dmls/tests/l1/main.rs
    - darkmatter/lib/src/markdown/compose/call_precheck.rs
    - darkmatter/lib/src/markdown/compose/expression/call_check.rs
    - darkmatter/lib/src/markdown/compose/expression/prepared.rs
    - darkmatter/lib/src/markdown/compose/inline/interpolation.rs
    - darkmatter/lib/src/markdown/compose/mod.rs
    - darkmatter/lib/src/markdown/compose/pipeline/mod.rs
    - darkmatter/lib/src/markdown/compose/pipeline/phases.rs
    - darkmatter/lib/src/markdown/types.rs
    - darkmatter/lib/tests/l1/compose_call_precheck.rs
    - darkmatter/lib/tests/l1/design_time_call_check.rs
    - darkmatter/lib/tests/l1/type_scope_ownership_guard.rs
    - darkmatter/dmls/tests/l1/type_scope_ownership_guard.rs
documentation:
    - claudine/features/2026-10-28-schema-enhancements/inventory/criteria-map.md
    - claudine/features/2026-10-28-schema-enhancements/inventory/prerequisites.md
    - claudine/features/2026-10-28-schema-enhancements/inventory/function-registry.md
    - claudine/features/2026-10-28-schema-enhancements/inventory/ownership.md
    - claudine/features/2026-10-28-schema-enhancements/inventory/reader-matrix.md
    - claudine/features/2026-10-28-schema-enhancements/inventory/rulings.md
    - claudine/features/2026-10-28-schema-enhancements/behavior-ledger.md
    - claudine/features/2026-10-28-schema-enhancements/spikes/numeric-ingress/findings.md
    - claudine/features/2026-10-28-schema-enhancements/spikes/numeric-ingress/runs/default.md
    - claudine/features/2026-10-28-schema-enhancements/spikes/numeric-ingress/runs/arbitrary-precision.md
    - claudine/features/2026-10-28-schema-enhancements/spikes/numeric-ingress/runs/preserve-order.md
    - darkmatter/lib/tests/fixtures/schema_enhancements_matrix/PROVENANCE.md
    - darkmatter/docs/topics/darkmatter-expressions.md
    - darkmatter/docs/topics/schemas/definition.md
    - darkmatter/docs/topics/schemas/dmls-schema-support.md
    - darkmatter/docs/dependencies.md
    - docs/dependencies.md
    - darkmatter/docs/topics/numbers.md
    - darkmatter/docs/inline/schema-validation.md
    - claudine/docs/topics/flow-control/lifecycle.md
    - claudine/docs/topics/state-management/context-variables.md
    - claudine/docs/topics/state-management/drift.md
    - claudine/docs/topics/state-management/expression-engine.md
    - darkmatter/docs/lsp/features.md
    - prompts/_add/add-expressions.md
    - claudine/features/2026-10-28-schema-enhancements/plan.md
    - claudine/features/2026-10-28-schema-enhancements/implementation-log.md
    - claudine/features/2026-10-28-schema-enhancements/phase4-design.md
    - darkmatter/dmls/docs/diagnostics.md
    - darkmatter/docs/darkmatter-compose-pipeline.md
    - darkmatter/docs/inline/call-check.md
    - darkmatter/docs/topics/schemas/authoring-schemas.md
    - darkmatter/README.md
    - darkmatter/dmls/docs/features.md
    - darkmatter/dmls/docs/autocomplete.md
    - darkmatter/dmls/docs/hover.md
    - darkmatter/dmls/docs/editors/README.md
    - darkmatter/dmls/docs/editors/smoke-checklist.md
    - darkmatter/dmls/README.md
    - darkmatter/docs/lsp/configuration.md
    - darkmatter/docs/lsp/architecture.md
    - claudine/README.md
completed_phase: 6
implemented: true
pipeline_refreshed:
    - 2026-10-29-recursive-schema-types
    - 2026-10-30-claudine-recursive-schemas
    - 2026-10-31-type-aware-evaluation
    - 2026-11-01-map-and-filter
decisions_for_author: []
---

# Implementation Log for 2026-10-28-schema-enhancements (6 phases)

## Phase 1

Phase 1 is inventory and a one-time numeric-ingress spike. It changes no
production code.

### What was produced

| Plan task | Output |
|---|---|
| Contract inventory: criterion → task/test map | `inventory/criteria-map.md`: all 61 criteria (1–59, 33a, 33b) mapped to a plan task, a proposed L1 test identity, and the public result the test observes |
| Contract inventory: prerequisites | `inventory/prerequisites.md`: checked in code, not by spec status. `core-schema-types`, `union-type-constraints`, `fm-namespace` and `type-narrowing` are all **supplied**; the inline `(A \| B)[]` spelling and the declared-type projection are this feature's own work (Phase 2 Grammar, Phase 5 Variable projection) |
| Contract inventory: registry and catalog | `inventory/function-registry.md`, enumerated from the registry source: 113 canonical functions (62 pure, 49 context, 2 lazy: `and`/`or`), 75 aliases; the current catalog matches the registry exactly |
| Contract inventory: behavior ledger | `behavior-ledger.md`: one draft row per canonical function with null family, fixture applicability and preserved/changed status. Reviewed and approved by the author on 2026-10-22 |
| Contract inventory: API/file ownership | `inventory/ownership.md` |
| Contract inventory: reader fields and matrix expansion | `inventory/reader-matrix.md` (with a "Cell coverage" appendix) |
| Contract inventory: rulings | `inventory/rulings.md` (R1–R8, all pending the author) |
| Numeric ingress spike (ran once) | `spikes/numeric-ingress/findings.md`, throwaway crate `spikes/numeric-ingress/src/` (outside the workspace), captured runs in `spikes/numeric-ingress/runs/` |
| Matrix fixtures | `darkmatter/lib/tests/l1/schema_enhancements_matrix/` (5 L1 tests) and `darkmatter/lib/tests/fixtures/schema_enhancements_matrix/` with `PROVENANCE.md` |

### Key findings

- **Examples: 121, not 117.** 74 executable, 47 display-only. The spec's 117
  is the catalog at commit `78540226f` (2026-09-17); `find_files` and
  `try_frontmatter` have since added one executable example each and
  `code_link` two display-only ones. The 72-executable floor is still met.
  Recommendation: carry all 121.
- **`trim` is not a registered function**, but the spec's null table and
  criterion 43 (`trim(null)` → `null`) use it. Criterion 43 cannot pass as
  written without adding `trim` or changing the criterion.
- **`keys`/`values` are not registered yet** (planned by this feature).
- **The draft replacement catalog** (`claudine/docs/schemas/partials/functions.yaml`)
  is missing `find_files`, `code_link`, `try_frontmatter` and lists
  `is_file_type`, which is out of scope and unregistered.
- **Converted-input fixture impossible for 34 functions**: 24 whose only
  parameter is `unknown | null` (7 type predicates, 14 date predicates,
  `file_exists`, `and`, `or`) and 10 taking `any[]` (`first`, `last`, eight
  `as_*`). `and`/`or` have no rejected fixture either. This is ruling R1;
  **criterion 10 stays blocked until the author rules.**
- **Ownership conflict:** `expr/functions/mod.rs` (6,041 lines) holds the
  dispatch, the binding types and 27 context handler bodies, so the Wave 5
  Pure and Context handler tasks would collide. Proposed: before Wave 5, the
  integrator moves the context bodies into their family files (move only).
  Unowned files: `schemas/resolve.rs` (→ Schema adapter) and
  `schemas/mod.rs` (→ integrator). The name `expr/binding.rs` is taken, so
  the Call binder needs another module name. Nine catalog consumers the plan
  does not name exist, including `claudine/gen/src/inputs.rs:292` and
  Claudine's grouping in `claudine/cli/src/commands/context/mod.rs:67`.
- **ConfigState drift (plan rule 8):** the plan's claim is only partly right.
  The `ConfigState` docs (`dmls/src/config/mod.rs:457`) already say the file
  layer is kept, matching `recompute`. The drift is in the test
  `test_semantic_tokens_malformed_value_keeps_previous` (`:1079–1081`), whose
  comment says the last good config survives. Left for the Wave 8 config task,
  which owns that file. Also: one invalid editor value discards *every*
  non-formatting overlay section (existing contract, kept).
- **`workspace/configuration` is never requested by DMLS** (documented). The
  spec's criterion 40 wording is read as the existing `didChangeConfiguration`
  channel (ruling R4).
- **Prerequisite comment drift (not fixed; owned by Phase 5):**
  `PredicateFact::FileExists` (`facts.rs:28`) says the handler rejects
  non-strings; it returns `false`.

### Numeric ingress: selected seam

A **number-spelling sidecar**: each loader keeps building today's
`serde_json::Value` and also records a JSON Pointer → written-number-text map
for numbers stored as `f64`, like the existing `MappingOrders`. One admission
function checks exactness from the spelling. `serde_json/arbitrary_precision`
was rejected (no effect on YAML/JSON5, workspace-wide feature unification
into 67 crates, and it breaks `Value(1.0) == Value(1.00)`). Darkmatter must
declare `serde_json` `raw_value` and `float_roundtrip` itself. Spike
recovered every spelling except YAML `99999999999999999999`, which fails the
whole frontmatter today. Four cell-level decisions for the author are in
`findings.md`.

### Existing defects found (not fixed in Phase 1; each is owned by a later task)

| Defect | Owner task |
|---|---|
| `standalone.rs:127-130` turns a YAML parse error into `Ok(None)`; DMLS then drops it (`named_types.rs:344`, overlay `:231`) | Grammar (Wave 2) |
| Duplicate-key and trailing-content errors in schema files reported as `AmbiguousReferenced` with no detail (`resolve.rs:759`, `:1152`) | Schema adapter (Wave 4) |
| `coerce.rs:597` rounds big integers silently; its comment is wrong | Numeric foundations (Wave 2) |
| `grammar.rs:279`, `:1948` parse type-expression numbers with bare `parse::<f64>()` | Grammar + Numeric (Wave 2) |
| YAML `.inf`/`.nan` and JSON5 `1e400`/`Infinity` become `null`; `1e-400` becomes `0`; YAML integer > u64 fails the whole frontmatter; enum members saturate through `i64`; `min(inf)` accepted | Numeric foundations (Wave 2) |
| `Markdown::from(String)` swallows frontmatter parse errors (`markdown/mod.rs:999-1003`) | not in this feature's scope; reported |
| `catalog.rs:425-438` silently skips a malformed schema-root catalog | not in this feature's scope; reported |
| `md set` stores malformed JSON as a string without warning | only if the numeric seam is routed through it |

### Matrix fixtures

Positive controls come from real producers (shipped `schemas/partials/timing.yaml`
and a pure-envelope shipped schema, `md schema detect --format json` output
and its YAML twin, `md get --yaml` output); keyword fixtures that `detect`
does not emit are hand-written and labeled so in `PROVENANCE.md`.

Tests (all `darkmatter` `l1`, prefix `schema_enhancements_matrix::`):

- `envelope::shipped_tagged_envelope_matrix`
- `envelope::shipped_pure_envelope_matrix`
- `raw_schema::raw_json_schema_keyword_matrix` (48 keywords, `.json` branch)
- `raw_schema::raw_yaml_schema_keyword_matrix` (48 keywords, YAML raw branch)
- `frontmatter::frontmatter_value_matrix`

Only cells current code already decides correctly are asserted. Where the
outcome is right but the error class is mislabeled (`AmbiguousReferenced`),
the test asserts only that loading fails; the fixing task tightens it.
Every other cell is assigned to its task or ruling in
`inventory/reader-matrix.md` → "Cell coverage". DMLS cells wait for the
`expressions` field (Wave 8). Observed contract: an absent optional property
composes as `null`, so absent and explicit null differ only in the source.

`darkmatter/lib/tests/l1/schema_yaml_spellings.rs` was edited only to make
`Edit` and `apply` `pub(crate)` for reuse.

Targeted verification:

- `cargo nextest run -p darkmatter --features effects-instrumentation --test l1 -E 'test(/schema_enhancements_matrix/) | test(/schema_yaml_spellings/) | test(/test_layout/)'` → 30 passed (slowest new test 0.63 s)
- `cd darkmatter && just lint` → clean

### Requirement-to-test mapping (Phase 1)

Phase 1 changes no behavior, so there is no regression test to add. The
behavior Phase 1 pins is the current decided matrix cells above. The plan's
criteria-to-test mapping for later phases is `inventory/criteria-map.md`.

### Gates

Scope calculator (run from the repository root after all edits):
`git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`
→ `packages: [darkmatter, test-toolkit, claudine]`, `change_class: package`.
The gates were therefore required, and all were run serially with no
subagent running:

| Gate | Result |
|---|---|
| `claudine`: `just test` | 10,691 passed, 9 skipped, 0 failed (332 s) |
| `claudine`: `just lint` | clean |
| `just check-tier-coverage claudine` | 0 stranded |
| `just check-tier-coverage darkmatter` | 0 stranded |
| `darkmatter`: `cargo nextest run -p darkmatter --features effects-instrumentation --test l1` (the binary this phase changed) | 2,073 passed, 0 failed |
| `darkmatter`: `just lint` | clean |

No failures, no load-sensitive reruns, no pre-existing failures observed.
Dependent packages: Phase 1 changed only test files and fixtures inside the
`darkmatter` `l1` binary, so no dependent package can observe the change.
Cross-OS check skipped: the new tests use in-memory fixtures and temp
directories, with no OS-specific path handling; CI covers the other OSes.
The `claudine` skill needed no update (no Claudine behavior changed).

## Author Decisions (2026-10-22)

Phase 1 paused for the decisions below. The author approved every
recommendation as written, so each one is decided:

- **Category names:** option A. A blank or repeated name in `categories:` is a
  load error at that entry; names compare exactly; list order is kept.
- **Numeric edge cells:** all four recommendations (a located per-field error
  for oversized YAML integers; no `1e3`/`.5` expression literals; JSON
  `1e400`'s load error counts as "not finite"; `json`-typed `1e400` stays
  malformed JSON).
- **`inventory/rulings.md` R1–R7:** each recommendation, as written there.
- **`trim`:** add it as a text transform, so criterion 43 holds as written.
- **Examples:** carry all 121.
- **`behavior-ledger.md`:** each row marked "ruling requested" takes its
  proposed behavior; `has_command` and its analogues take the criterion 44
  signature (`string | null`). `keys(null)` and `values(null)` return `[]` (signature
  `object | null`), so a missing map lists nothing.
- **Wrong kind of value to a yes/no check** (`has_key("hello", "a")`,
  `has_command(["git"])`): a type error, not `false`. `null` still gives
  `false`.
- **Behavior ledger (criterion 13):** the author reviewed and approved the
  whole ledger on 2026-10-22; the review is recorded at the top of
  `behavior-ledger.md`.

The items as they were raised:

- **Can a category name appear twice, or be empty? (needed before Phase 2's grammar work)**

  Phase 2 adds a `categories:` list to schema files, which groups functions for documentation. The spec does not say what happens when two entries have the same name, or when a name is blank. The grammar code written in Phase 2 has to pick one answer, and changing it later means rewriting that code and its tests.

  - **Option A — reject both (recommended).** A blank name, or a name that repeats an earlier one, makes the schema file fail to load with an error pointing at the entry. Names compare exactly ("Math" and "math" are different). List order is kept. *Pro:* matches how the old catalog already rejects blank categories and how the spec rejects duplicate parameter names; nothing is silently dropped. *Con:* a small authoring mistake stops the file loading.
  - **Option B — merge duplicates, first one wins.** *Pro:* forgiving. *Con:* no sensible rule when the two copies have different descriptions; hides mistakes.
  - **Option C — silently drop blank names.** *Con:* this is the kind of silent filtering the project's input rules forbid.

  I recommend **A**. Say if you would rather compare names case-insensitively, so "Math" and "math" count as the same name.
- **Four edge cases in how numbers are read (needed before Phase 2's number work)**

  Phase 1's investigation picked how numbers will be checked for exactness: each file reader keeps today's values and also remembers how each number was written, so `9007199254740993` (too large to store exactly) can be rejected instead of silently rounded. A workspace-wide alternative (`serde_json`'s `arbitrary_precision` switch) was rejected because it would change number behavior in 67 crates and still not help YAML. Details: `spikes/numeric-ingress/findings.md`. Four smaller cases need your answer:

  1. **Huge whole numbers in YAML, such as `99999999999999999999`.** Today they make the *whole* frontmatter fail to parse. *Recommended:* keep it an error, but report it on that one field with its location. *Alternative:* report it as "cannot be represented exactly", like the other big numbers. That is more uniform, but it needs a custom YAML read path.
  2. **Expression literals such as `1e3` or `.5`.** These are not valid in expressions today. *Recommended:* keep it that way; the spec does not add that syntax. *Alternative:* add it (new syntax, more scope).
  3. **JSON `1e400` (too large to store).** The JSON reader already fails with "number out of range". *Recommended:* accept that load error as the spec's "not finite" outcome. *Alternative:* re-word the error to say "not finite" (cosmetic, extra work).
  4. **Text declared as `json` that contains `1e400`.** It is rejected today as malformed JSON. *Recommended:* keep that.
- **Rulings needed later (not blocking Phase 2, but best decided now while you are reviewing)**

  Each has a recommendation in `inventory/rulings.md` or `inventory/function-registry.md`. Replying "approve the recommendations" settles them all.

  - **R1 — some functions cannot have a "converted input" test (blocks acceptance criterion 10, Phase 4).** 34 functions take any value unchanged (for example `is_number`, `file_exists`, `first`) or take no arguments, so a test showing the input being converted is impossible. *Recommended:* mark those cells "impossible", test instead that the input arrives unchanged, and cover conversion with test-only functions. *Alternatives:* weaken those functions' signatures so they convert (changes behavior; not recommended), or drop them from the criterion (forbidden by the plan).
  - **`trim` does not exist.** The spec's criterion 43 expects `trim(null)` to return `null`, but there is no `trim` function. Add `trim` (recommended, a small string function) or change the criterion.
  - **121 examples, not 117.** Three functions added four examples after the spec was written. Carry all 121 (recommended).
  - **R3 — duplicate keys in JSON.** Reject them in raw `.json` schema files and in `json`-typed text (recommended). Keep the editor-settings channel as it is, because an existing test depends on it.
  - **R4 — editor settings channel.** The spec says `workspace/configuration`, but the language server deliberately uses `didChangeConfiguration`. Read the spec as meaning the existing channel (recommended).
  - **R5 — some "should fail" cells in the plan contradict documented behavior.** For example `types.x: null` is documented as the `null` type. Keep the documented behavior for ordinary schema files, and apply the stricter rules only in the new function-catalog loader (recommended).
  - **R7 — two plan cells disagree with the JSON Schema standard (`prefixItems: []`, `enum: []`).** Follow the standard's validator (recommended).
  - **Ambiguous null behavior for about 20 functions** (for example `is_empty(null)`, `starts_with(null, …)`) and **`has_command(true)`'s draft signature**, which contradicts criterion 44. These are listed in `behavior-ledger.md`; the ledger review is a pre-merge requirement anyway.

## Phase 2

### Integrator step 1: `serde_json` features and `preserve_order` triage

`darkmatter/lib/Cargo.toml` now declares `serde_json` with `preserve_order`,
`raw_value`, and `float_roundtrip`. Before this change the darkmatter,
claudine, claudine-cli, darkmatter-cli and dmls build graphs did **not** have
`preserve_order` (only a whole-workspace build got it, through tauri-build and
tree-sitter); every package that depends on `darkmatter` now gets it.

Fallout measured with the full darkmatter (lib, CLI, DMLS) and claudine L1
suites, no fail-fast: 45 darkmatter and 3 claudine failures, every one a key
order change. Classification:

| Consumer | Contract | Resolution |
|---|---|---|
| `md schema validate --format json` | alphabetical keys, pinned byte-for-byte by `schema_validate_legacy_json_output_is_byte_identical` | explicit `sort_all_objects()` before printing |
| non-strict `md hash` frontmatter hash | persisted in documents; nested keys were sorted | nested values sorted before hashing; new unit test `non_strict_frontmatter_hash_ignores_nested_key_order`; benchmark fixture identities unchanged |
| `OPENCODE_CONFIG_CONTENT` / `KILO_CONFIG_CONTENT` merge | byte-identical whatever the insertion order | merged value sorted; test now asserts through `merge_overlay` |
| SimplifiedSchema → JSON Schema (`to_json_schema`) | none (diff-friendly output) | written order is intended; `$schema` now leads the document; 41 snapshots refreshed after a script confirmed each old/new pair is JSON-equal |
| `as_json5`, frontmatter `serde_json` maps, `WithOverlay::iter`, provider `serialized_field_list` | none; order was an artifact of `BTreeMap` | expectations now state written/declaration order; `WithOverlay::iter` doc comment was stale (said order was sorted) and was rewritten |

`dmls::l1 searched_candidates::a_watched_nearer_candidate_reselects_the_import_and_its_removal_falls_back`
failed once and passed on rerun alone and in the next full run: load-sensitive,
not an ordering effect.

Note: the `Cargo.toml` edit was reverted once by something outside this
session while the first test run was in flight (other sessions share this
worktree); it was reapplied and re-verified.

### Integrator step 2: numeric admission API (frozen for Wave 2)

New module `darkmatter::markdown::numeric` (`darkmatter/lib/src/markdown/numeric.rs`):
`admit_number_spelling` (loader/lexer spelling), `admit_number_text` (the one
lenient text parser), `admit_f64`, `admit_json_number`, `admit_integer`,
`parse_boolean_word`, `format_number`, `json_number`, and the error types
`NumericAdmissionError` / `NumberTextError`. Whole-number exactness is
decided by comparing the written digits with the exact decimal expansion of the
nearest `f64`, so no big-integer dependency is needed.

### Wave 2 execution

The two Wave 2 tasks ran in parallel as foreground subagents, each in its own
git worktree seeded with a patch of the integrator's uncommitted base (steps 1
and 2), so neither could break the other's build. Each returned a patch
against that base; both applied to this worktree with no conflicts.

- **Grammar descriptors** (`/tmp/phase2-grammar.patch`): tuples, functions,
  `|` and grouping, `category`, `categories:`, inline examples, duplicate
  overload detection (`simplified/signature.rs`), function rejection in
  document schemas, numeric admission of schema metadata, the standalone
  YAML-error defect, serialization round trips, and docs.
- **Numeric foundations** (`/tmp/phase2-numeric.patch`): load-time admission
  for frontmatter YAML, raw JSON/YAML schemas, and expression literals
  (`numeric/ingress.rs`); `f64`-only operators with non-finite errors; one
  number formatter; JSON5 non-finite errors; the `coerce.rs` silent-rounding
  defect; `darkmatter/docs/topics/numbers.md`.

Integration fixes after the merge:

- The grammar task made a standalone schema file that is not valid YAML a
  `SchemaDocument` "not valid YAML" error. That path ran before the numeric
  task's oversized-integer rescan, so a raw YAML schema with
  `99999999999999999999` lost its admission reason
  (`raw_yaml_schema_numeric_token_matrix` failed). `standalone.rs` now loads
  through `numeric::load_admitted_yaml`, so every schema-file loader admits
  numbers the same way and reports the admission reason.
- The grammar task narrowed `LARGE_INT_CASES` in `schemas_literal_expression.rs`
  to exact values; the numeric task's coercion test iterated it expecting
  inexact values. The test now names its inexact inputs explicitly.

Concurrency hazards met (recorded so the next phase avoids them):

- An outside process committed this phase's integrator work in this shared
  worktree mid-phase (`58e2a5790 feat(darkmatter): preserve authored order and
  validate numeric ingress`, `b3e647ae8` for the log and plan). While it did,
  working files were briefly rolled back, and one claudine test build picked
  up an old test name. The rerun on the settled tree passed.
- zsh `CDPATH` resolves a relative `cd claudine` from a subdirectory to
  `/Volumes/coding/personal/rusty-biscuit/claudine`, a different checkout. One
  claudine `just lint` and both `check-tier-coverage` runs went there; all three
  were rerun from absolute paths in this worktree. Use absolute paths.
- `git apply --3way` stages what it applies; the merged files were unstaged
  again with `git reset -q -- <files>` (index only).

### Design Decisions

Integrator decisions:

1. **`preserve_order` everywhere, sort only where a published contract needs it.**
   - Decision: declare `serde_json/preserve_order` in `darkmatter/lib/Cargo.toml`
     and sort explicitly at outputs whose alphabetical order is a contract.
   - Alternatives: sort every serialized map back to alphabetical (hides the
     feature the spec asks for); refresh every changed snapshot (silently
     changes byte-stable outputs and persisted hashes).
   - Why: the spec rules `preserve_order` in (decision 75); persisted hashes and
     byte-pinned JSON reports must not drift.
   - Downsides: four sort sites to keep; a new alphabetical-output contract must
     sort explicitly.
2. **`sniff-cli` sorts its JSON keys.**
   - Decision: `sniff-cli` sorts at its three JSON writers
     (`output::sort_json_keys`), and its snapshot tests compare with
     `sort_maps`; a new test pins sorted keys on real output.
   - Alternatives: accept the new key order and refresh the five snapshots.
   - Why: `sniff-cli` links `darkmatter`, so without sorting its output would
     depend on which packages a build unifies.
   - Downsides: struct-declaration order is not used for `sniff` JSON.
3. **`to_json_schema` puts `$schema` first.**
   - Alternatives: leave it last (where insertion order puts it).
   - Why: readers expect the dialect first; the 41 snapshot changes were checked
     to be order-only by comparing each old/new pair as JSON.
   - Downsides: none found.
4. **Numeric admission at ingress (load time).**
   - Decision: frontmatter YAML, raw JSON/YAML schemas, schema-file YAML,
     SimplifiedSchema metadata, and expression literals admit every number when
     loaded, from its written spelling; a failure is a located load error
     whatever the property's declared type. The Phase 1 spelling sidecar is
     used only inside the loaders (the guided second pass) rather than stored
     beside the value.
   - Alternatives: store a JSON Pointer → spelling sidecar on `Frontmatter` and
     check at schema validation (Wave 4).
   - Why: the spec applies the check "wherever a value becomes a number",
     including native YAML numbers; a value that no `f64` holds cannot be a
     Darkmatter value under any type; checking at load closes the silent
     `.inf` → `null` and `1e-400` → `0` defects now, needs no new public
     state, and criteria 30/59's native rows ("fails validation", "rejected")
     are met as a located load error.
   - Downsides: an untyped or `string` property holding such a number must
     quote it; whole numbers in exponent form that no `f64` holds (`1e308`,
     `1e300`) are rejected, as the spec's whole-number rule requires.
5. **Criterion 46's `1e308 * 10`** is exercised with `big` supplied through
   frontmatter, written as the full digits of `f64::MAX` (an exactly held whole
   number), because exponent literals are not expression syntax (author
   decision, numeric cell 2) and `1e308` itself is an inexact whole number.

Grammar descriptors decisions (subagent):

1. A trailing `-> description` on a `|` union describes the last option, as the
   list form does. Alternatives: reject it; copy it to every option. Downside:
   per-option descriptions need the YAML list.
2. An option inside a YAML-list union cannot itself be a `|` union. Alternative:
   flatten it. Why: one spelling per union; flattening breaks per-item source
   projection.
3. Grouping: a group without `[]` joins the enclosing union; `(A)[]` is `A[]`;
   lists of lists and a group with its own constraint list are rejected. Why:
   one representation (`TypeExpr::Union` with `is_array`).
4. `?` on a union tuple element follows TypeScript: `(A | B)?` is required;
   `A | B?` and `A? | B` are rejected.
5. `tuple` and `function` are reserved built-in keywords; a local type with
   either name is reached as `tuple@this`. Downside: a bare reference to such a
   local type now fails.
6. A spread must be written as an array type (`...T` naming an array type is
   rejected) and takes no list constraints. Why: the simplest rule matching
   "must name an array type".
7. Tuple constraints are limited to `required`, `default`, `generated`,
   file `example`, and `category`, checked at parse time, so types files that
   are never converted are still checked.
8. Functions take no `required`/`default` but may take `[]`; a trailing `;`
   and trailing commas in element/parameter lists are accepted.
9. Tuple elements, parameter types, and return types take no `-> description`.
10. `|` ends a `Name@file` reference everywhere; inside a group, tuple, or
    signature, `)`, `]`, `;`, and `?` also end it. Downside: a referenced path
    cannot contain `|`.
11. Binding contracts for duplicate overloads compare canonical parameter types
    and constraints (order-insensitive), optional and rest positions; they
    ignore names, descriptions, examples, categories, return types, and
    `fallible`. Checked structurally at parse time and again with local named
    types resolved (depth 16) when a types file loads. Downside: references
    into other files compare by name.
12. `to_json_schema` is the one place that rejects a function in a document
    schema, naming the path; it works because the resolver inlines references
    first.
13. `tuple([])` lowers to `minItems: 0, items: false` with no `prefixItems`
    (the 2020-12 meta-schema rejects an empty `prefixItems`, ruling R7).
14. `categories:` is accepted only in the tagged envelope; the global catalog
    loader is unchanged. An entry splits on its first `->`; `Name ->` with no
    text is an error; at most one `category(...)` per type.
15. The strict schema-document source locator now follows `|` and `>` block
    scalars, since `function` signatures are written as `|-` blocks. This moves
    a boundary documented as "frozen v1"; the pinning tests and docs were
    updated.
16. A schema file that is not valid YAML is a `SchemaDocument` error instead of
    a silent decline or the mislabeled `AmbiguousReferenced`. DMLS
    `standalone_lints` still returns `None` for unparseable text, so ordinary
    YAML with syntax errors does not show as a broken schema.
17. DMLS surfaces the YAML error where it was dropped: the overlay's named-type
    catalog records it and hover on a reference into that file shows it;
    `diagnostics/named_types.rs` attaches it as related information.
18. Schema numbers: `inf`/`nan` report "not finite"; quoted defaults stay text;
    word enum members are admitted only when spelled like numbers;
    `suggest(...)` candidates are unchanged.

Numeric foundations decisions (subagent):

1. Equality stays exact (integer-to-integer `==` does not round). Alternative:
   compare everything as `f64` (broke two pinned `expression_strict_equality`
   tests). Why: every admitted number is held exactly, so exact `==` agrees with
   ordering for every value a document, schema, or literal supplies. Downside:
   an unadmitted external integer (for example `--set` with
   `9007199254740993`) is unequal to `…992` while neither `<` nor `>` holds;
   arithmetic rejects it.
2. Oversized YAML integers: a failed first parse triggers a rescan that reports
   an inadmissible integer beyond `u64` as a located admission error. Downside:
   an exactly held integer beyond `u64` (2^64) keeps the old parse error
   (pinned by a unit test).
3. `coerce_to_number` keeps its spelling rule and leaves inexact text
   unconverted; `validate.rs` appends the admission reason to the type error.
   Alternative: an error channel in `coerce.rs` (a public struct Wave 4
   replaces). Behavior change: `"9223372036854775807"` and
   `"9007199254740993.0"` no longer convert.
4. The JSON5 check stays in Darkmatter (`Json5::deserialize_raw` plus a
   non-finite scan); `biscuit-file` is unchanged. `--state`/`--set` reject a
   non-finite number; transclusion `set.n=Infinity` is an error (quoted stays
   text); a `name=Infinity` shorthand setter stays text, matching its existing
   fallback.
5. Numeric text in arithmetic goes through `admit_number_text`; inadmissible
   numeric text is an error rather than a concatenation. Spellings whose result
   changed: `" 4 "` and `"1_000"` now convert; `"inf"`, `"NaN"`, `"5."`,
   `"1e400"`, `"1e-400"`, and inexact whole text no longer convert.
6. Raw schema admission failures reuse `SchemaError::SchemaDocument` (a new
   variant would ripple through exhaustive matches); JSON `1e400` keeps its
   existing load error (author cell 3).
7. `format_number` writes every digit of a large whole number (Rust's shortest
   form printed `2^60` as `1152921504606847000`, which reads back as a
   different number).

### Requirement-to-test mapping

| Requirement | Tests (darkmatter `l1` unless noted) |
|---|---|
| Criterion 1 (grammar forms and rejections) | `schema_grammar_descriptors::{tuple_forms_parse, function_type_with_every_constraint_parses, union_operator_parses_everywhere_a_type_is_written, categories_list_parses_in_order_with_optional_multi_line_descriptions, invalid_tuples_and_repeated_categories_are_rejected_at_the_offending_text, invalid_function_types_are_rejected_at_the_offending_text, a_union_inside_a_yaml_list_arm_and_a_question_mark_on_a_property_are_rejected}` |
| Criterion 2 (precedence) | `schema_grammar_descriptors::array_suffix_binds_tighter_than_the_union_operator` |
| Criterion 3 (JSON Schema lowering) | snapshots `schema_grammar_descriptors::{snapshot_fixed_tuple, snapshot_variadic_tuple_with_optional_position, snapshot_union_operator, snapshot_empty_tuple_and_category_annotation}`; `union_operator_and_yaml_list_are_the_same_definition` |
| Criterion 4 / 56 (functions in document schemas, indirect forms) | `schema_grammar_descriptors::{a_document_schema_that_reaches_a_function_fails_to_load_naming_the_path, tuples_validate_positions_omission_and_length}`; CLI `schema_grammar_types::schema_validate_rejects_a_function_typed_property_as_a_schema_error` |
| Criterion 5 (categories) | `schema_grammar_descriptors::{a_declared_categories_list_limits_category_names, a_category_on_a_data_type_is_an_annotation}`; `schema_enhancements_matrix::envelope::documented_categories_matrix` |
| Criterion 6 (duplicate overloads) | `schema_grammar_descriptors::overloads_with_one_binding_contract_fail_to_load`; unit `simplified::signature::tests` |
| Criterion 47 (`(A \| B)[]`) | `schema_grammar_descriptors::grouped_union_list_matches_a_named_union_list` (compiled JSON Schema rejects `[true]`; document-level rejection waits for Phase 3 coercion, which today turns `true` into `"true"`) |
| Round trip of new forms | `schema_grammar_descriptors::every_new_form_round_trips_through_serialization` |
| End to end through `md compose` | CLI `schema_grammar_types::{compose_accepts_a_document_matching_a_tuple_and_a_union, compose_rejects_a_document_whose_tuple_has_too_many_positions}` |
| Standalone YAML-error defect | `schema_grammar_descriptors::a_types_file_that_is_not_yaml_reports_its_yaml_error`; DMLS `named_type_references::a_reference_into_a_file_that_is_not_yaml_reports_the_yaml_error` (fails before the fix) |
| Schema metadata numbers | `schema_grammar_descriptors::numeric_metadata_rejects_what_an_f64_cannot_hold`; `schemas_literal_expression::literal_inexact_large_integer_is_a_schema_load_error` |
| Criterion 21 (literal part) | `numeric_admission::{an_inexact_expression_literal_is_rejected_at_the_literal, an_exact_expression_literal_evaluates_exactly}`; `expression_type_scope::literal_numbers_narrow_to_the_value_the_evaluator_compares` |
| Criteria 30/59 (native YAML rows, spellings) | `numeric_admission::{native_frontmatter_numbers_no_f64_holds_fail_to_load_with_location, admissible_frontmatter_numbers_load_exactly}`; lib unit `frontmatter::tests::{inadmissible_frontmatter_numbers_are_located_load_errors, inadmissible_numbers_are_not_hidden_by_expression_fallbacks, admissible_frontmatter_numbers_load_exactly}`; `numeric::tests::*`; `numeric::ingress::tests::yaml_*` |
| Quoted inexact text vs `number` (coerce defect) | `numeric_admission::quoted_inexact_text_against_a_number_fails_validation_with_the_reason`; lib unit `coerce::tests::large_integral_strings_are_admitted_before_coercion`; `schemas_literal_expression::literal_large_integer_coercion_admits_before_converting` |
| Criterion 46 (operators, presentation) | `numeric_admission::{non_finite_results_are_errors, equality_and_ordering_agree_on_represented_values, whole_numbers_never_pass_through_an_integer_cast, the_largest_finite_number_loads_and_renders_every_digit, numeric_text_converts_through_the_canonical_parser, an_out_of_range_index_is_none_rather_than_saturated}`; lib unit `ast::tests::display_number_literal_beyond_i64_never_saturates`, `numeric::tests::presentation_never_narrows` |
| Raw schema numeric tokens | `numeric_admission::{raw_schema_numbers_are_admitted_at_load, ordinary_raw_schema_bounds_still_load_and_validate}`; `schema_enhancements_matrix::raw_schema::{raw_json_schema_numeric_token_matrix, raw_yaml_schema_numeric_token_matrix}` |
| Frontmatter numeric matrix cells | `schema_enhancements_matrix::frontmatter::frontmatter_value_matrix` (18 numeric cells) |
| JSON5 non-finite | lib unit `numeric::ingress::tests::json5_non_finite_numbers_are_rejected`, `transclusion::parser::tests::set_property_non_finite_number_is_an_error_not_null`; CLI `compose_state_set::test_compose_non_finite_json5_number_is_an_error_not_null`, CLI unit `commands::compose::tests::parse_shorthand_value_keeps_a_non_finite_number_as_text_not_null` |
| Criterion 58 (map order) | lib unit `markdown::hash::tests::non_strict_frontmatter_hash_ignores_nested_key_order`, `frontmatter::tests::parsed_frontmatter_retains_nested_mapping_order_as_metadata`, `fn_list_formatting::as_json5_uses_the_formatter_object_idiom`; `benchmark_fixtures::benchmark_manifest_matches_recorded_identities`; CLI `schema_validate_baseline::schema_validate_legacy_json_output_is_byte_identical`; claudine `opencode_config::tests::yolo_permission_block_serializes_independent_of_insertion_order`, `action_shape_control::proxy_with_iteration_follows_authored_order_and_is_complete`; sniff-cli `snapshots::json_output_keys_are_sorted_at_every_depth` |

Existing tests whose expectations changed (each pinned behavior this phase
changes on purpose): the 41 conversion snapshots (order only);
`as_json5_uses_the_formatter_object_idiom`,
`parsed_frontmatter_retains_nested_mapping_order_as_metadata`,
`serialized_field_list_matches_catalog`,
`proxy_with_iteration_is_canonical_and_complete` (renamed), the OpenCode
insertion-order test (order); `coerce::tests::large_integral_strings_*`,
`schemas_literal_expression::literal_large_integer_*`,
`expression_type_scope::literal_numbers_narrow_to_the_value_the_evaluator_compares`
(numeric admission); `schema_yaml_spellings` (library and DMLS),
`meta_schema_phase6`, `schemas_union_postfix`, `shipped_schema_corpus`, and
`convert.rs` unit tests (grammar: block-scalar locator, YAML errors, new
keywords); sniff-cli JSON snapshots (now compared with `sort_maps`).

### Draft catalog under the new grammar

`claudine/docs/schemas/partials/functions.yaml` is still classed Draft. Its
only grammar failures are two uses of `min-length(...)`, plus its `behavior:`
envelope key. With those fixed, all 111 signatures parse and the file passes
the categories and overload checks. `mutations.yaml` signatures parse; it is
rejected for its `name:` key. Neither draft was edited (Phase 4 migrates them).

### Gates

Scope calculator, run from the repository root after all edits:
`packages: [claudine, claudine-gen, darkmatter, darkmatter-cli, dmls, sniff-cli, test-toolkit]`,
`change_class: package`, so every gate was required. All ran serially with no
subagent running:

| Gate | Result |
|---|---|
| `darkmatter`: `just test` (`--no-fail-fast` rerun) | 11,342 passed, 10 skipped |
| `darkmatter`: `just lint` | clean |
| `claudine`: `just test` | 10,691 passed, 9 skipped |
| `claudine`: `just lint` | clean |
| `just check-tier-coverage claudine` / `darkmatter` | 0 stranded / 0 stranded |
| Dependents, `cargo nextest run -p <pkg>` one at a time | messenger 226, messenger-cli 133, research 606, sniff-cli 976, playa-cli 25, biscuit-icon-cli 98, biscuit-speaks-cli 108, claudine-gen + claudine-contract 298: all passed |
| `sniff`: `just lint` | clean |

Load-sensitive: `dmls::l1 searched_candidates::a_watched_nearer_candidate_reselects_the_import_and_its_removal_falls_back`
failed under full-suite load in three runs (including one before any Phase 2
code, with only `preserve_order`) and passed alone every time. `main` already
relaxes this assertion (`43798e829 test(dmls): accept an intermediate watch
registration under load`), which `feat/pipeline` does not have yet.

Cross-OS: no OS-specific path, process, or console code changed (numeric
parsing, YAML/JSON loaders, grammar, key ordering), so no `just cross-check`
run was made; CI covers the other environments.

The `claudine` skill needed no change: Claudine's only visible effects are the
authored-order `with:` iteration (documented in Darkmatter) and the sorted
OpenCode/Kilo inline config (unchanged output). The `darkmatter` skill
(`expressions.md`, `frontmatter.md`, `schema.md`) was updated.

## Phase 3

Phase 3 builds the shared conversion engine: the rule core (Wave 3), then the
schema adapter and the call binder (Wave 4). Production function dispatch is
not switched in this phase (Phase 4 does that).

### Wave 3: conversion core

Started 2026-10-22 from a clean tree at `72cb04ab7`. The core is delegated to
one foreground subagent with the design below; the integrator owns
`markdown/mod.rs` registration.

The core landed as `darkmatter::markdown::coercion`
(`darkmatter/lib/src/markdown/coercion/{mod,shape,engine,union,error}.rs`):

- `Shape` is the neutral, rule-free target. Its kinds are `Any`, `Null`,
  `Number` (with `as_sent` for `numberlike`), `Boolean` (with `as_sent` for
  `boolish`), `Text` (lengths, pattern, members, and `scalar_text: false` for
  `alphanumeric`), `Literal`, `Content` (Json/Yaml), `Array`, `Tuple`,
  `Object`, `Union`, and `Assert`.
- `Object` covers properties, pattern slots with a `PatternScope`
  (`AllKeys` for JSON Schema, `UndeclaredKeys` for SimplifiedSchema), and
  `Additional`.
- `Assert` wraps a shape with a passive predicate. Translators use it for
  refinement formats and for assertions the core does not model.
- Entry points: `bind`, `bind_with` (`BindOptions { defer, max_depth }`),
  `check_exact` (keep-only, for return contracts), `accepts_exactly`, and
  `content_error` (the one strict JSON/YAML content check).
- `bind` returns `Bound::Kept(&original)` or `Bound::Converted(copy)`.
- `ConversionError` holds one `ValueFailure` per failing path. Each failure
  carries the path, the expected type with its constraints, the received
  value (`None` when missing), the converted value if any, and a typed
  `FailureReason`.
- The default depth bound is `MAX_DEPTH = 64`.
- Tests: `darkmatter` `l1` `coercion_core::*` (46 tests, 0.47 s total).

### Design Decisions

1. **Criterion 47's `[true]` rejection is the compiled schema's.**
   - Decision: the rule core converts `[true]` against `(string | number)[]`
     to `["true"]`, because boolean → text is always safe and only the
     `string` option is reachable. Criterion 47's "reject `[true]`" is read
     as the compiled JSON Schema's verdict, which `check_exact` agrees with.
   - Alternatives: make boolean → text unsafe inside unions (contradicts the
     conversion table and `s(val: string | string[])` given `2026`); add a
     list-specific exception (two engines).
   - Why: criterion 47's "How to validate" compares lowered JSON Schema. The
     spec's conversion rules and criterion 20 force the document-level
     result.
   - Downsides: a document with `tags: [true]` under that type now composes
     as `["true"]` instead of failing.
2. **Union ties are compared field by field at the deepest differing paths.**
   - Decision: each reading records which conversion produced each converted
     path. A reading wins only if, at every path where it differs from
     another reading, it holds number-over-boolean or JSON-over-YAML. Any
     other difference, including converted against kept, is an ambiguous
     union naming every reading.
   - Alternatives: rank whole options (order-sensitive); treat a kept field
     as beating a converted one (a third tie-breaker the spec forbids).
   - Why: spec "Unions: Choosing an Option" names exactly two tie-breakers
     and requires field-wise application.
   - Downsides: `{a: string, b: number} | {a: number, b: number}` given
     `{a: "4", b: "5"}` is ambiguous rather than keeping `a`.
3. **Overlapping declarations get their own reason** (`ConflictingDeclarations`).
   - Why: a JSON Schema key under a property and a matching pattern must
     satisfy both, so this is not a union choice.
   - Downsides: one more reason variant for consumers to render.
4. **Content text is checked for syntax, duplicates, and finiteness only.**
   - Decision: numbers inside JSON/YAML text are not checked for exactness.
     Text given to `json`/`yaml` is kept as written.
   - Alternatives: admit every number inside the content.
   - Why: the spec treats content text as already-encoded data that is kept
     unchanged. Author decision (numeric cell 4) already covers only `1e400`.
   - Downsides: `"9007199254740993"` inside a JSON string-typed config keeps
     its digits as text, which a later reader may round.
5. **`check_exact` reports a needed conversion as `ConversionNotAllowed`.**
   - Why: a return-contract failure must not say "no conversion from that
     type" when a conversion exists but is not permitted there.
6. **The depth bound is 64 value levels** (`BindOptions.max_depth`). The
   grammar limit of 32 bounds type nesting, not value nesting. Exceeding the
   bound is a `ResourceLimit` failure, never acceptance.

### Wave 4: schema adapter and call binder

The two tasks ran in parallel as foreground subagents, each in its own
detached worktree (`p3-adapter`, `p3-binder`) seeded with a tarball of the
Wave 3 base. They owned disjoint files. The integrator copied each task's
files back into this worktree and merged the shared `tests/l1/main.rs`
registrations and the `compose/expression/mod.rs` exports by hand, then
removed both worktrees.

**Schema adapter**

- New `schemas/json_shape.rs` with `shape_from_json_schema`: compiled or raw
  JSON Schema → `Shape`.
- `schemas/coerce.rs` is rewritten over the core. The old rule code is gone
  (`CoercionTarget`, `coercion_target`, `is_numberlike`, `coerce_to_number`,
  and the root-union and property-union pickers). The entry points keep
  their names.
- Other changes:
  - Custom formats `darkmatter-numberlike` and `darkmatter-boolish`; the
    lowering is now `{"type": ["number", "string"], "format": ...}`.
  - `darkmatter-json` and `darkmatter-yaml` use `coercion::content_error`.
  - `validate.rs` reports the core's reasons.
  - `rewrite.rs` root unions are order-independent.
  - `resolve.rs` reports duplicate-key, trailing-content, and overflow errors
    in raw `.json` schemas as located `SchemaDocument` errors. This fixes the
    Phase 1 `AmbiguousReferenced` defect.
- New tests: `frontmatter_coercion_engine` (criteria 28–34, 33a, 33b, 47
  element-wise, 59), `raw_schema_conversion` (33a raw, 55, `$id`/`$anchor`),
  CLI `compose_schema::{compose_frontmatter_writes_engine_conversions,
  compose_reports_the_engine_reason_for_a_failed_conversion}`, and matrix
  cells `raw_schema::{raw_json,raw_yaml}_schema_conversion_controls` and
  `frontmatter::frontmatter_conversion_matrix`.

**Call binder**

- New module `compose/expression/call_binding/{mod,translate,signature,registration,bind,error}.rs`.
- Grammar → `Shape` translator: `shape_from_property_def`,
  `property_shape_from_def`, `shape_from_simplified_schema`, with a
  `TypeResolver` for named types.
- Signatures and registrations: `CompiledSignature`, and
  `FunctionRegistration` with evaluation mode, source exclusions, and
  numeric recovery.
- Binding: `bind_call` with order-independent overloads, read-only
  `BoundArgs`, incremental `LazyCall`, and `check_return`. Errors are typed
  as type, arity, ambiguous overload, or implementation contract.
- Dispatch is not switched; that is Phase 4.
- Tests: `function_call_binding` (52 tests, criteria 12 binding level, 14–27,
  57 return contract, laziness, registration rules).

**Integrator work after the merge**

- **Translator conformance** (Wave 4 checkpoint): new
  `tests/l1/coercion_translator_conformance.rs`. It runs 40 grammar
  constructs, one `#[test]` each, binding 38 probe values as a document
  property through `call_binding::shape_from_simplified_schema` and through
  `to_json_schema` + `shape_from_json_schema`. The keep/convert/error
  outcomes must be identical. All 40 pass.
- **`number(integer)`'s 2^64 ceiling** (the adapter flagged the two
  translators disagreeing at or above 2^64): `convert.rs` now emits
  `x-darkmatter-integer: true` beside `"type": "integer"`. That custom
  keyword (`format.rs`, registered in `validate.rs`) enforces magnitude
  below 2^64 in the validator, and `json_shape.rs` maps it to
  `IntegerRule::WholeBelow2Pow64`. The `md schema about` description of
  `integer` is updated.
- **Cleanup rule reader**: the adapter had split
  `base_style_markdown_schema_agrees_with_the_rule_validator` and the DMLS
  `style_markdown_parity` test, because schema validation now converts
  `style.markdown` values (`"80"` → `80`, `5` → `"5"`) while
  `cleanup::rules` read the authored value. Instead,
  `cleanup/rules.rs::read_leaf` now binds each value through the engine
  against the rule's declaration in the shipped `style.yaml` partial
  (`RULE_SHAPES`). Every rule reader (`md clean`, the `style:` parser, DMLS
  `formatting.markdown`) therefore agrees with validation. Both tests are
  back to one agreed expectation per row. `schemas::embedded` became
  `pub(crate)` for this. The `style.yaml` comment claiming literal unions
  block coercion was stale and is rewritten.
- **Commit unit is the top-level property**: see Design Decision 9 below.
  `schemas/coerce.rs::property_wise` plus `partial_copy` now commit each
  property that converts cleanly. The adapter's three tests that pinned
  "one failure commits nothing" were updated (`raw_schema_conversion::failure_leaves_input_unchanged`,
  `schemas_core_type_values::alphanumeric_never_coerces_numbers_or_booleans`,
  and the unit test renamed `a_failing_property_keeps_its_siblings_conversions`).
  New test: `frontmatter_coercion_engine::a_missing_required_sibling_does_not_block_conversion`.
  This fixed claudine `composition::prepare::service::tests::a_deferred_read_still_coerces_declared_types`,
  which failed after the merge.
- **Lifecycle parity**: claudine `claudine_schemas::break_and_prep_shapes_agree_between_schema_and_parser`
  now parses the frontmatter the validated document holds (`Judged.held`,
  through `EffectiveSchema::coerce_frontmatter_with_pending`), as Claudine's
  preparation does. Two rows changed on purpose: `break: {code: '2'}` (now
  `2`) and `break: 5` (the reason `"5"`) are valid. A row with
  `code: 'two'` keeps the rejection case.
- **New tests**: `coercion_engine_parity` (criterion 35). It has 17 value/type
  tables, one `#[test]` each, comparing library composition of a temp-dir
  document with `FunctionRegistration::bind_call`, plus a control test showing
  both sides observe keep, convert, and reject, plus
  `old_frontmatter_rule_code_is_gone` (an `include_str!` scan of
  `schemas/coerce.rs`). Also `frontmatter_coercion_engine::integer_ceiling_applies_to_frontmatter`.
  Function-call parity through production dispatch arrives with Phase 4's
  dispatch switch.

### Performance (spec "Performance": one before/after sample)

After the merge, claudine `just test` took 643 s with 6 timeouts (Phase 1
took 332 s). Serial A/B on this host, with a HEAD worktree at `72cb04ab7`
(`p3-baseline`) against this tree, `-j 1`:

| Test | HEAD | after merge | final |
|---|---|---|---|
| `claudine_schemas::cancelled_dictionaries_reject_forbidden_actions` | 0.95 s | 10.9 s | 1.20 s |
| `claudine_schemas::task_cancelled_rejects_forbidden_actions_in_the_deepest_conditionals` | 1.42 s | 14.9 s | 1.74 s |
| `wrap_sequence_composition::sequence_composition_dry_run_for_every_provider` (CLI) | 9.7 s | 15.6 s | 8.6 s |

`sample` profiles named four causes, each fixed:

1. `schema_content_hash` serialized the whole schema to bytes and hashed it
   on every shape, relaxed-shape, and validator lookup, three or four times
   per validation, and Claudine's unrolled trigger schemas are very large.
   A `ConversionPlan` (shape, relaxed shape, and structural validator, each
   lazy) is now kept on `EffectiveSchema`. It is keyed by the digest
   `ValidatorCache::shared_schema_hash` already records for the same `Arc`,
   so a validation hashes nothing extra.
2. Before deciding to keep a value, the plan compiled a separate structural
   validator for the whole schema. Now `EffectiveSchema::accepts_as_is`
   checks the instance with the validators the schema already compiled, and
   a value the complete schema accepts is kept without any plan work.
3. `Shape::describe` rendered whole union and object types for every failure.
   It now stops at `DESCRIPTION_LIMIT` (240 bytes) and appends `…`.
4. `relax_presence` rebuilt the relaxed shape on every partial-copy pass. It
   is now built once per plan.

The remaining cost (about 1.2–1.26x on invalid documents) is the core
binding a large instance to find what converts. Valid documents (the CLI
test) are faster than HEAD, because the keep decision now reuses the
compiled validators.

### Design Decisions (Wave 4)

These continue the Wave 3 numbering.

7. **Raw-schema authority through assertions** (adapter).
   - Decision: keywords the translator does not model, `oneOf`'s
     exactly-one rule, `$ref` with constraining siblings, and Darkmatter's
     validating `x-` keywords become a lazily compiled `jsonschema` assertion
     on their subschema. A subschema holding a reference compiles inside a
     wrapper that embeds the whole document under its resolved `$id`. Cyclic
     references go validation-only below 4 levels, with at most 32 references
     per path and 4,096 per translation.
   - Alternatives: model every keyword (a second validator); ignore
     unmodeled keywords during conversion (they would become unconstrained
     success).
   - Why: spec "Raw JSON Schema Conversion Boundaries": the complete schema
     decides admissibility, and candidates are filtered by full validation.
   - Downsides: an assertion that cannot compile passes locally, and the
     final full validation decides.
8. **Text with only numeric keywords is kept as written** (adapter). A
   typeless subschema such as `{minimum: 0}` never converts text, because
   converting `0.25` to `"0.25"` would let it escape `minimum`.
9. **Each top-level property is its own commit unit** (integrator; replaces
   the adapter's whole-document rule).
   - Decision: when the whole converted document is not accepted, each
     top-level property whose conversion succeeds, and on which the complete
     schema then reports no problem, is written back. A failing property
     stays as written. A missing required property never blocks a sibling,
     because presence is the validator's verdict. A root union keeps the
     whole-document rule, since choosing its arm needs the whole document.
     Inside one property, conversion is still all-or-nothing.
   - Alternatives: commit nothing on any failure (the adapter's rule).
   - Why: spec "Three Actions per Value" names each frontmatter property as
     a value. Claudine's deferred reads (`SchemaStage::DeferToStabilizedReread`)
     must see converted types while a required field's verdict is
     postponed, and the old engine committed per property.
   - Downsides: a validate-only report and the write-back can each hold a
     partly converted document. This was already true before this feature.
10. **`number(integer)` emits `x-darkmatter-integer: true`** (integrator).
    - Decision: a custom validator keyword enforces the spec's "below 2^64"
      ceiling, and `json_shape.rs` maps it to `IntegerRule::WholeBelow2Pow64`.
      Raw `integer` keeps no ceiling.
    - Alternatives: `exclusiveMaximum: 2^64` (an awkward float literal in
      every snapshot, and indistinguishable from an authored bound); a
      translator-only marker (the validator and the engine would disagree).
    - Why: the compiled schema, the frontmatter path, and the call binder
      must reach the same verdict (conformance test, spec "Numbers").
    - Downsides: three snapshots and one inline lowering expectation gained
      the keyword. External validators ignore it, which is looser but
      acceptable.
11. **`style.markdown` frontmatter values bind through the engine; settings
    files do not** (integrator).
    - Decision: `cleanup::rules::bind_document_rules` and `validate_rule`
      bind a document's rule values against the shipped `style.yaml`
      declarations (`RULE_SHAPES`) before reading them. `md clean`'s document
      reader and the `style:` parser use it. DMLS `.dmls.toml` and editor
      `formatting.markdown` keep reading values as written.
    - Alternatives: bind in the shared `read_leaf` (tried first). That made
      the settings readers accept `"80"` and broke their established
      robustness matrices (`workspace_formatting_settings::{toml,json}_formatting_matrix`).
    - Why: the spec governs frontmatter and function calls. A frontmatter
      consumer must read the value the validated document holds, so
      validation and cleanup agree. Settings files are outside the spec.
    - Downsides: the same quoted `"80"` is valid in frontmatter and rejected
      in `.dmls.toml`. The module docs say so.
    - The `clean_rules` matrix's wrong-type values were changed to values
      that stay wrong after conversion (`"maybe"`, `"80.5"`, `"3"`,
      `{ lang: sh }`). New tests: `clean_rules::quoted_rule_values_bind_like_schema_validation`
      and `style::parse::tests::markdown_rule_values_bind_like_schema_validation`.
12. **Call-binder registration rules** (binder).
    - Lazy registrations have exactly one signature and no recovery
      (`InvalidLazy`).
    - Recovery requires a required subject, an optional `number` fallback,
      and a `number` return.
    - A recovered call never counts as exact.
    - Null is admitted only through `Shape::admits_null_explicitly`, checked
      after source exclusions.
    - Return checks use `check_exact`, with no parameter null rule.
    - Refinement formats reuse the structural JSON Schema format validators,
      so the two translators judge formats with the same code. `file` is
      always syntax-only.
13. **Translator conformance is a document-level comparison** (integrator).
    Each construct is the single property `v`, bound through
    `shape_from_simplified_schema` and through `to_json_schema` +
    `shape_from_json_schema`. That includes the optional-property null
    wrapper both translators apply.
14. **`numberlike`/`boolish` lowering** (adapter): `{"type": [number|boolean,
    "string"], "format": "darkmatter-numberlike|boolish"}`. Both formats
    are backed by the canonical parser. `md schema detect` never infers these
    types, so its JSON output and snapshot are unchanged, and no detect
    snapshot existed to update for criterion 33b.

### Requirement-to-test mapping (Phase 3)

All tests are `darkmatter` `l1` unless noted.

| Requirement | Tests |
|---|---|
| Core rule tables: conversions, rejections, exactness, constraints, preservation types, content, containers, unions in every option order, immutability, depth bound | `coercion_core::*` (46) |
| Criteria 14–27 at binding level (no dispatch switch yet) | `function_call_binding::*` (52; mapping in the Wave 4 summary above) |
| Criterion 12 recovery and criterion 57 return contract (binding level) | `function_call_binding::{recovery_*, a_supplied_fallback_is_always_validated, wrong_return_reports_implementation_contract_error}` |
| Criterion 53 laziness (binding level) | `function_call_binding::{lazy_binding_never_binds_unevaluated_operands, lazy_binding_reports_each_operand_as_it_binds}` |
| Criteria 28–34, 47 (element-wise), 59 | `frontmatter_coercion_engine::*`, plus `integer_ceiling_applies_to_frontmatter` and `a_missing_required_sibling_does_not_block_conversion` |
| Criterion 33a raw, 55 | `raw_schema_conversion::*` |
| Criterion 35 (one engine) | `coercion_engine_parity::*` (17 tables, a control, and the source scan) |
| Wave 4 checkpoint: both translators agree | `coercion_translator_conformance::*` (40) |
| End to end through `md compose` | CLI `compose_schema::{compose_frontmatter_writes_engine_conversions, compose_reports_the_engine_reason_for_a_failed_conversion}` |
| Reader matrix cells (raw schema, frontmatter conversion, located `.json` document errors) | `schema_enhancements_matrix::raw_schema::{raw_json,raw_yaml}_schema_conversion_controls`, `schema_enhancements_matrix::frontmatter::frontmatter_conversion_matrix` |
| `style.markdown` agreement | `base_schema_end_to_end::base_style_markdown_schema_agrees_with_the_rule_validator`, DMLS `style_markdown_parity::*`, `clean_rules::{yaml_matrix_walks_every_rule_and_shape, quoted_rule_values_bind_like_schema_validation}`, unit `style::parse::tests::markdown_rule_values_bind_like_schema_validation` |
| Lifecycle schema/parser agreement after conversion | claudine `claudine_schemas::break_and_prep_shapes_agree_between_schema_and_parser` |
| Deferred reads still convert | claudine unit `composition::prepare::service::tests::a_deferred_read_still_coerces_declared_types` (failed after the merge; passes) |
| `md schema about` text | CLI `schema_about::schema_about_verbose_prints_advanced_sections_as_readable_lists` |

Existing expectations changed on purpose (each a spec-intended behavior):

- `numberlike`/`boolish` are no longer normalized: `schemas_union_coerce` (5),
  CLI `compose_schema` (2), CLI `schema_union_pipeline` (1), and unit
  `inline_object_array_nested_coercion`.
- `"yes"` now converts to a boolean (two copies of
  `non_boolean_word_still_reports_type_problem`).
- Number → text through overlapping union arms: `schemas_union_clean`,
  `schemas_union_report`.
- Quoted `style.markdown` values are valid in frontmatter.
- Lifecycle rows `code: '2'` and `break: 5` are now valid.
- Matrix: the `$id` row uses `[5]`, and the `1e400` cell expects
  `SchemaDocument`.
- Snapshots: three convert snapshots (the new `numberlike`/`boolish`
  formats), and `snapshot_atom_number_integer_with_min` plus
  `suggest_constraint_phase3` (`x-darkmatter-integer`).

### Gates

Scope calculator, run from the repository root after every edit:
`git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`
gave `packages: [claudine, darkmatter, darkmatter-cli, dmls, test-toolkit]`
and `change_class: package`, so every gate was required. All gates ran
serially with no subagent running.

| Gate | Result |
|---|---|
| `claudine`: `just test` (final run) | 10,691 passed, 9 skipped, 0 failed (366 s) |
| `claudine`: `just lint` | clean (a pre-existing linker `__eh_frame` size note) |
| `darkmatter`: `just test` (final run) | 11,472 passed, 10 skipped, 1 failed: the known load-sensitive `dmls::l1 searched_candidates::a_watched_nearer_candidate_reselects_the_import_and_its_removal_falls_back`, which passed alone each of the three times it was rerun |
| `darkmatter`: `just lint` | clean |
| `just check-tier-coverage claudine` / `darkmatter` | 0 stranded / 0 stranded |
| `test-toolkit` (selected by the calculator) | 478 passed |
| Dependents: claudine-gen and claudine-contract, then messenger | 298 passed, then 226 passed |
| Dependents: research, sniff-cli, messenger-cli, playa-cli, biscuit-icon-cli, biscuit-speaks-cli | `cargo check --tests` clean. None of them uses the changed schema or conversion APIs; a grep found no use of the removed `CoercionTarget`/`coercion_target` outside a throwaway spike |

Slow tests that predate this phase (over 5 s on HEAD as well):
`wrap_sequence_composition::sequence_composition_dry_run_for_every_provider`
(9.7 s on HEAD, 8.6 s now) and several `claudine_schemas::*_cancelled_*`
tests. They were not changed.

Cross-OS: no path, process, console, or `#[cfg]` code changed. The work is
pure value conversion, one embedded-document lookup by a constant path, and
docs, so no `just cross-check` was run; CI covers the other environments.

The `claudine` skill needed no change. Claudine's only visible effect is that
frontmatter lifecycle values convert like any other frontmatter (for example
`break: {code: '2'}`), and the skill does not describe those value types. The
`darkmatter` skill (`schema.md`, `cleanup.md`) was updated.

## Phase 4

Phase 4 migrates every function and catalog consumer (Waves 5–6). Started
2026-10-22 from the Phase 3 tree. The integrator froze the shared shapes in
`phase4-design.md` before any parallel task started.

### Integrator preparation

- **Move-only:** the context-handler bodies (`absolute_fn` … `validate_schema_fn`
  and their helpers) moved from `functions/mod.rs` into `functions/fs.rs`,
  re-exported with `pub use fs::*`. No behavior change; `cargo check --tests`
  is clean.
- **Interim catalog:** `darkmatter/schemas/partials/functions.yaml` was first
  generated mechanically from the old catalog (old signatures, old examples)
  so the loader and dispatch could be built and tested while the final
  catalog was written in parallel.
- **Frozen shapes:** `phase4-design.md` (catalog file, descriptor API, handler
  signature, dispatch order, the `Call` error, test hooks).

### Execution

Three rounds of foreground subagents. Parallel tasks ran in throwaway
worktrees seeded from a tarball of the uncommitted tree and owned disjoint
files; the integrator copied each task's files back, merged the one shared
file (`tests/l1/main.rs`) by hand, and removed the worktrees.

1. **Round 1 (parallel).** *Catalog* wrote the final
   `darkmatter/schemas/partials/functions.yaml` (116 functions, 124
   overloads, 22 categories, 147 examples: 100 executable, 43 display-only,
   4 file-form; all 121 old examples carried, 26 added), moved `mutations.yaml`
   beside it, created `function-examples/*.yaml`, and deleted both drafts under
   `claudine/docs/schemas/partials/`. *Infrastructure* replaced the catalog
   loader and descriptors, built the registry and the one eager dispatch entry
   (`call_function`), the lazy path for `and`/`or`, `ExpressionError::Call`,
   and the test hooks, with transitional legacy handler variants so the old
   bodies kept compiling. The old catalog file and its parser were deleted.
2. **Integrator.** Moved the context unit tests into `functions/fs_tests.rs`
   (move-only) so the two handler tasks did not share a test module.
3. **Round 2 (parallel).** *Pure handlers* (bodies moved into their family
   files), *Context handlers* (`fs.rs` and the family files), and *Consumers*
   (DMLS, `md schema about`, `claudine context --expressions`, docs, the
   shipped-schema corpus). After the merge the integrator deleted the legacy
   handler variants, `args.rs`, `any_null`, `require_string`,
   `ExpressionError::{Arity, ArgType}`, `ArityBound`, and the evaluator's
   string-sniffing arity enrichment.
4. **Round 3.** Registry-wide criterion tests (below).

Integrator fixes after the merge (each an approved behavior change, not a
weakened test):

- `lifecycle::actions::tests::expression_function_signature_bracket_optional`:
  `number`'s second parameter is `fallback` (spec item 1).
- `compose_expression_failure_contract` (5) and CLI
  `compose_expression_failures` (4): `min(1)` now reports the binder's
  `min: no signature takes 1 argument` instead of handler text.
- `unknown_identifier_warning`: the row `length(x)` (missing `x` → `0`)
  became a type error by ledger row 18; replaced by `upper(x)` (null passes
  through, row 31) and `is_empty(trim(x))` (the spec's own example, now that
  `trim` exists).
- `lookup_parity` inventory: the catalog's fixture lookup moved to
  `catalog/tests.rs`; `FsLookup`/`MapLookup` no longer exist.
- `current_root_documentation_contract`: two skill phrases claimed one shared
  descriptor for `ctx.recent_commits`/`recent_commits()`; they now say the two
  share one rendering (see decision 7). The `recent_commits` catalog
  description carries the binding-time sentence the doc contract pins, and
  the generated table was regenerated.
- Docs: `darkmatter-expressions.md` "Function Contracts" rewritten (it still
  said every function propagates null), the `validate_schema(path, obj)` row
  (no longer "accepted for forward compatibility"), `docs/lsp/features.md`
  (`fallback`), and Claudine `lifecycle.md`'s pre-warm example (`date()` with no
  arguments is now an arity error).

### Design Decisions

1. **Handlers take the bound call.**
   - Decision: `PureFn = fn(&BoundCall) -> Result<Value, String>`,
     `ContextFn = fn(&BoundCall, &ResolutionContext) -> Result<Value, ExpressionError>`.
     Handlers read only `BoundArgs` accessors and `overload()`.
   - Alternatives: keep `&[Value]` and bind before the call (handlers could
     still re-check types); pass `BoundArgs` only (overloaded handlers would
     guess the overload from argument shapes).
   - Why: spec criterion 11; the selected overload is part of the binding.
   - Downsides: every handler and its unit tests changed.
2. **`ExpressionError::Call` and its severity.**
   - Decision: one variant carries the binder's `CallBindingError`. Only an
     implementation-contract error is authoring-fatal; type, arity, and
     ambiguous-overload errors keep the old arity/arg-type severity (demoted to
     a warning only in lenient body interpolation). `Arity`/`ArgType` are gone.
   - Alternatives: make every binding error fatal (changes the ratified
     fatality matrix for ordinary mistakes); keep `ArgType` beside `Call` (two
     ways to report one failure).
   - Why: the spec makes bad input a type error at binding; the old severity
     of type errors was a ratified contract.
   - Downsides: some inputs that were fatal `ContractViolation`/`FileReference`
     errors are now type errors and are demoted in lenient body interpolation:
     an unknown `has_agentic_cli` name, a malformed `ping` address, an
     out-of-range `recent_commits`/`pr_list` count, and `absolute("")`
     (`fatality_characterization` changed that one cell deliberately). Document
     bodies stay strict whatever `fail_fast` says, so composition still fails.
3. **Test hooks ride the `effects-instrumentation` feature.** Test-registered
   functions and handler-entry counters (`function_test_hooks`) compile under
   `cfg(test)` or that feature, which `just test` and CI already enable for the
   library and DMLS. Alternative: a new feature (one more CI feature set to
   keep consistent). Downside: the feature name is broader than effects.
4. **The display key keeps its old form** (`round(x, [fallback])`, `and(...)`).
   Every key-based consumer (`lifecycle::signatures::parse_signature`,
   suggestions, DMLS keys) keeps working; types live in `typed_signature()`
   and the new fields. Alternative: grammar spelling (`fallback?`), which would
   have changed every consumer's parser for no reader benefit.
5. **File-form examples are embedded.** `example(./function-examples/<name>.yaml)`
   files live in `darkmatter/schemas/partials/function-examples/`, are embedded
   with `include_str!`, and are validated with `validate_example_bytes` when the
   catalog loads, so the embedded catalog never reads disk.
6. **Named helper types live in the catalog.** `agentic-cli` (an enum pinned to
   the generated `AGENTIC_CLI_NAMES` by a test), `ip-address` (a pattern), and
   `recent-commits`. Non-function `types:` entries are helper types, never
   functions. Alternative: Rust-registered refinements (a second, invisible
   type vocabulary).
7. **`ctx.recent_commits` does not import `recent-commits`.**
   - Decision: the function returns the catalog's `recent-commits` type and the
     two descriptions name each other; `context-variables.yaml` keeps
     `string[]`.
   - Alternatives: add `functions.yaml` to the embedded shipped set and import
     it (tried twice).
   - Why: the import works, but every on-disk copy of the shipped partials
     (the `global_catalog` fixture matrix, content-policy baselines, two DMLS
     sessions, and a user's own disk copy) would then need `functions.yaml`
     beside them; about 20 existing tests failed for that reason alone.
   - Downsides: the shared type is enforced by review, not by one definition.
8. **Strict catalog loader.** A function without `category(...)`, or an
   overload set whose overloads name different categories, fails to load, so
   each function's overloads stay adjacent and `function_descriptors(name)`
   returns one slice. `Example.result` is the authored literal text.
9. **Catalog signatures where the ledger left room** (catalog task):
   - list formatters return `string | null` (the ledger's null rule returns
     `null`, which a plain `string` return would reject);
   - `date` and the date-arithmetic functions take `date | datetime`, `fmt` is
     an enum of the nine accepted spellings, and the duration is a pattern, so
     bad input is a type error (`date` lost `fallible`); the handlers accept
     every datetime spelling the binder admits;
   - `pr`/`cicd` take `number(integer; min(1)) | string(min(1))`: under the
     union rule `pr(0)` binds as the text `"0"` and the handler rejects it at
     runtime (recorded for review; the rule itself is correct);
   - `ping`'s `timeout` stays plain `number` (range checked in the handler);
   - `fallible` removed from functions that fail only by argument rejection
     (`min`, `max`, `abs`, `first`, `last`, `has_key`, `contains`, `length`,
     `number`, string predicates and transforms, `terminal`, `date`, list
     formatters) and added where a captured context can be missing (`ipv4`,
     `ipv6`, `has_agentic_cli`) or a bound value still fails (`has_skill`,
     `has_local_skill`); `try_frontmatter(file: file | null): object | null` is
     not fallible because every load error is inside its result.
10. **Unbound-argument defects surface as errors, not panics.** Pure handlers
    use a `bound()` helper and context handlers a `required_text` helper: if a
    handler and its signature ever disagree, the call fails with a domain or
    contract error instead of `expect`. Cost: a branch no caller can reach.
11. **`validate_schema(file, obj)` reuses `deep_merge_override`** (the `::file
    set=` merge the spec names) and validates the merged page in memory; only
    the boolean is returned, so lost source positions are invisible.
12. **Consumers.** DMLS completes and hovers aliases case-insensitively and
    shows every overload on hover; `claudine context --expressions` and
    `md schema about` group by `categories:` order and sort canonicals
    alphabetically; file-form example results render in JSON spelling on one
    line; the Claudine snapshot pins order and identity only, so a reworded
    description does not churn it. Typed signatures name helper types
    (`agent: agentic-cli`) instead of expanding them.
13. **Lifecycle verb lists include the new functions.** `keys`, `trim`, and
    `values` were added to the verb lists in `claudine/schemas/{action,lifecycle}.yaml`
    and `darkmatter/schemas/partials/{lifecycle,md-lifecycle}.yaml`, which the
    lifecycle parity tests derive from the function catalog.
14. **Fixed while migrating:** a huge duration count (`99999999999999999d`)
    panicked in `chrono::Duration::days` and is now a domain error; `ensure_*`
    decides whether its joined result is a number with the canonical numeric
    check, so a value that cannot be held exactly stays text.

### Requirement-to-test mapping (Phase 4)

All tests are `darkmatter` `l1` (`darkmatter/lib/tests/l1/`) unless noted.
"Counter" is `function_test_hooks::handler_entries`.

| Requirement | Tests |
|---|---|
| Criterion 7: catalog complete, only catalog, consumers read the API | `function_dispatch::the_embedded_catalog_and_the_registry_have_the_same_functions`, unit `functions::registration_tests::*` (parity both ways, every rule applies), `function_binding_registry::every_registered_function_has_a_row`; old file deleted (`git status`), CLI `context_command::*`, DMLS `lsp_session`/`overlay::expressions` tests read `function_descriptors` |
| Criterion 8: executable examples pass structurally (100 ≥ 72) | unit `catalog::tests::every_executable_example_evaluates_to_its_declared_result` and the structural-comparison unit tests beside it |
| Criterion 9: display-only and file-form examples shown, never run | unit `catalog::tests::{every_display_only_example_carries_a_non_empty_reason, examples_keep_their_authored_text_and_files_are_embedded}`; claudine-cli `commands::context::tests` example formatting; generated table in `darkmatter-expressions.md` (`narrative_doc_function_table_matches_catalog`) |
| Criterion 10: every function binds (accepted / rejected / converted or impossible) | `function_binding_registry::rows::<name>` (116, one per function, macro-generated) + guard `every_registered_function_has_a_row` |
| Criterion 11: no handler coerces | `function_binding_registry::no_handler_local_coercion_helpers_remain` (scans all 23 files of `functions/` and fails on an unscanned new file); the integrator reviewed every handler family in the merge (no handler inspects an argument's type to convert it, except `number`/`round`/`contains`/list rendering, whose purpose is conversion) |
| Criterion 12: `number`/`round` | `pure_function_contracts::*` (all listed cases, counters), catalog executable examples |
| Criteria 14, 15, 18, 19, 43 (pure), 44 (pure), 45 | `pure_function_contracts::*` |
| Criteria 16, 17, 19, 20, 21, 22, 24, 25, 26, 27 through dispatch | `function_binding_conversions::*` (test-registered functions named as in the spec, both union orders, counters 0 on every rejection) |
| Criterion 23 | `pull_requests`/`cicd` unit tests (both overload orders, counters), unit `compose::tests::provider_network` (fake provider request count) |
| Criteria 42, 43 (`file_exists(null)`), 44 (`validate_schema` M9, `has_command(true)`) | `context_function_contracts::*` |
| Criterion 35 dispatch side | `coercion_engine_parity::*` (each table now also runs through dispatch) |
| Criterion 53 laziness through dispatch | `function_dispatch::a_skipped_lazy_operand_is_never_entered` |
| Criterion 57 | `function_catalog_parity::{the_registered_aliases_are_the_inventoried_ones, aliases_resolve_as_before, canonical_names_resolve_case_insensitively, documentation_lists_canonicals_once}`, unit `catalog::tests::semantically_duplicate_overloads_fail_catalog_load`, `function_dispatch::a_wrong_return_is_an_implementation_contract_error_and_is_not_converted` |
| Criterion 39 | claudine-cli `context_command::context_expressions_function_order_snapshot` |
| Catalog loader rules | unit `catalog::tests::*` (missing `categories:`, named helper type, order, aliases, `typed_signature`, `agentic_cli_named_type_is_the_generated_table`) |
| End to end through `md compose` | darkmatter-cli `compose_schema_enhancements::{converted_calls_compose_to_their_results, an_unbindable_call_fails_compose_with_the_type_error}` |
| Shipped corpus | `shipped_schema_corpus::*` (`functions.yaml`/`mutations.yaml` classified as function catalogs that refuse to apply as document schemas; helper types import and lower; example files are not schemas) |

Impossible converted-input cells (ruling R1) are asserted as "arrives
unchanged" in their rows: the `unknown | null` predicates (`is_number("4")` is
`false`; `file_exists(2026)` is `false` beside a file named `2026`), the
`any[]` list functions, `keys`/`values`, and `and`/`or` (lazy, no handler,
any arity). For `has_agentic_cli` and the date family a converted number then
fails the refined type, so the row asserts the rejection with counter 0.
Conversion itself is covered by the test-registered functions.

Not automated here: criterion 13's human ledger review (recorded at the top of
`behavior-ledger.md`); a ledger-completeness test was not added because a
test must not read a feature directory that moves when the spec closes.

Shipped-document scan (round 3): no call relies on a removed result
(`round(x, default)`, one-argument `number`/`round` on text, `min`/`max` on
null, `has_key` on non-objects). `length(...)` calls in Claudine docs read
values that are always lists; they would become type errors only if the
value were `null`.

Lifecycle long-form keys follow catalog parameter names, so the renames in
this phase change authored long-form spellings: `number`/`round` `default` →
`fallback` (spec), `length` `x` → `val` (spec), `file_exists` `file` →
`value` (spec), `terminal` `string` → `content`. The Claudine fixture
`no_error_flag_is_accepted_on_every_action_category` now writes
`length: {val: …}`.

### Performance (spec "Performance": one before/after sample)

Release `md compose` on one host (best of three), against the installed
pre-feature `md` (built 2026-10-18): a document with 8,000 pure function
calls took 0.401 s before and 0.422 s after; a control document with 8,000
frontmatter reads took 0.377 s and 0.395 s. Function calls therefore cost
about 24 ms before and 27 ms after per 8,000 calls; the rest of the
difference is unrelated startup work present in the control. Binding
compiles each signature once (the registry is built once per process) and
`check_return` uses the already-compiled return shape, so no per-call schema
hashing or compilation was added.

### Gates

Scope calculator, run from the repository root after every edit:
`git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`
gave `packages: [claudine, claudine-cli, darkmatter, darkmatter-cli, dmls, test-toolkit]`
and `change_class: package`, so every gate was required. Gates ran serially
with no subagent running.

| Gate | Result |
|---|---|
| `darkmatter`: `just test` (final run, `--no-fail-fast`) | 11,571 passed, 10 skipped, 0 failed |
| `darkmatter`: `just lint` | clean |
| `claudine`: `just test` (`--no-fail-fast`) | 10,691 passed, 9 skipped, 1 failed: `action_shape_control::no_error_flag_is_accepted_on_every_action_category` (`length`'s parameter is now `val`); fixed and rerun alone: passes |
| `claudine`: `just lint` | clean |
| `just check-tier-coverage claudine` / `darkmatter` | 0 stranded / 0 stranded |
| `test-toolkit` (selected by the calculator) | 478 passed |
| Dependents: `messenger` | 226 passed |
| Dependents: research, messenger-cli, sniff-cli, playa-cli, biscuit-icon-cli, biscuit-speaks-cli | `cargo check --tests` clean; none uses the expression-function API |

The known load-sensitive DMLS test
`searched_candidates::a_watched_nearer_candidate_reselects_the_import_and_its_removal_falls_back`
failed in one intermediate full darkmatter run and passed in the final one.

Cross-OS: no path, process, console, or `#[cfg]` code changed. The file
handlers keep their existing `FileReference` resolution; the new tests anchor
in temp dirs and spawn `md`/`claudine` only through the existing fixtures. No
`just cross-check` was run; CI covers the other environments.

Skills: `darkmatter` (`library-surfaces.md`, `compose.md`) and `claudine`
(`cli-commands.md`, `cli-reference.md` — the `--expressions` report's order
and example column) were updated.


## Phase 5

Phase 5 adds passive checking and editor agreement (Waves 7–8). Started
2026-10-22 from the Phase 4 tree.

### Wave 7: library checking authority (integrator)

New module `darkmatter/lib/src/markdown/compose/expression/call_check.rs`,
re-exported from `expression`:

- `ArgumentCategory` (`Compatible` 1, `NullableMatch` 2, `Uncertain` 3,
  `Unknown` 4, `Incompatible` 5; `number()`), `ArgumentType`
  (`Literal(Value)` / `Typed(scope::Shape)` / `Unknown`, built by
  `ArgumentType::of(arg, &TypeScope)`).
- `classify_argument(&ArgumentType, &CompiledParameter)`: the pairwise query.
- `check_call(name, &[ArgumentType], call_span, argument_spans) -> Option<CallCheck>`:
  whole-call classification (arity, overloads, source exclusions, numeric
  recovery). `CallCheck::{category, is_rejected, findings, rejection_message}`;
  `CallFinding { span, category, argument, message }` (argument span, or the
  call span for `CallProblem::{Arity, Ambiguous, NoOverload}`).
- `check_expression_calls(&SpannedExpr, &TypeScope) -> Vec<CallCheck>`: every
  call in every branch, each argument typed at its own site through
  `branch_scope` (so `is_number(foo) ? min(foo, 5) : 0` narrows).
- `check_prepared_calls(&PreparedValue, &TypeScope) -> Vec<PreparedCallCheck>`:
  the same over a prepared frontmatter value, spans mapped onto the leaf.
- Projection: `DeclaredTypes::top_level_types() -> BTreeMap<String, Shape>`.
- `ExpressionError::CallCheck(Box<CallCheck>)`: authoring-fatal; its message
  is the binder's own type error for an all-literal call.
- Supporting: `CompiledParameter::{declared, binds}`,
  `CompiledSignature::recovery`, `functions::inspect_registration` (test
  functions first, as dispatch resolves them).

Tests: `darkmatter` `l1` `design_time_call_check::*` (23).

### Design Decisions

1. **Call checks read the declaration-only layer.**
   - Decision: the call check's `TypeScope` is built over
     `DeclaredTypes::new(schema)` with no static frontmatter.
   - Alternatives: reuse the static-frontmatter layer the presence checks use
     (a set `maybe: 3` would remove `null`, making row b category 1).
   - Why: the spec's projection table derives types from the schema alone,
     and criterion 40 sets every variable in frontmatter yet expects row b to
     be category 2. A frontmatter value is a runtime value: setters, caller
     inputs, and interpolation can replace it.
   - Downsides: a document that sets an optional property still gets the
     strict-mode category 2 warning.
2. **An all-literal call is classified by the runtime binder itself.**
   - Decision: when every argument is a literal, `check_call` runs
     `FunctionRegistration::bind_call` (passive; no handler) and maps its
     result; recovery and ambiguity come out exactly as at runtime, and the
     rejection message is the binder's type error.
   - Alternatives: classify literals one by one (recovery and ambiguity
     would need re-implementing).
   - Why: "Literals are checked by value through the same rules the runtime
     uses"; it also makes composition's message read like the runtime error.
   - Downsides: none found; mixed calls use the per-argument path with
     `CompiledParameter::binds` for their literals.
3. **Typed arguments: `is_subtype` first, then the engine's conversion
   families.** An exact fit is `is_subtype` against the parameter's declared
   grammar type; otherwise source and target are compared as coercion shapes
   by family (number/boolean → text always; text → number/boolean/content
   may; native → json/yaml always). Constraints and assertions never decide a
   type-level verdict; a literal alternative (narrowed) binds by value; a
   list → list whose elements do not fit is category 3 (an empty list binds).
   Alternative: compare grammar types only (cannot see resolved named types
   or conversions). Downside: the family table restates which conversions
   exist; it reads the same `Shape`s the engine walks.
4. **Overloads.** With several overloads that take the argument count, the
   verdict is the overload whose worst argument is least concerning; if every
   overload is impossible, the finding ranges the one argument they all
   reject, else the call (`NoOverload`). Typed arguments never produce an
   ambiguity finding (only literals can prove one).
5. **Recovery for typed arguments.** A subject that may fail is recovered
   when the fallback is certain to bind (category 1); otherwise it takes the
   fallback's category (it is recovered exactly when the fallback binds); an
   impossible fallback rejects the call on the fallback.
6. **Category order for "worst".** 5 > 3 > 4 > 2 > 1: a possible error
   outranks an unknown type, which outranks a possible null.
7. **The shared walk is at the expression level.** The library walks every
   call of a parsed expression with branch-aware narrowing; composition and
   DMLS each find their own expressions (composition from the document it
   composes, DMLS from its overlay with editor positions) and pass each one,
   under its enclosing block's guards, to the same walk. Alternative: a
   library document scanner returning document offsets (DMLS's frontmatter
   positions come from its own YAML AST, so it could not adopt one without a
   second position model). Downside: each consumer owns which expressions it
   finds; Wave 8's shared-corpus test pins that they agree.
8. **No batch API and no `may_bind`.** The spec allows a batch form only
   with measured need; `CallCheck::is_rejected` answers the boolean question.

After Wave 8 the integrator fixed one Wave 7 defect the DMLS task found: an
unknown argument (a call result, an untyped variable) passed to an
`unknown | null` parameter was category 4; the spec makes any source to
`unknown | null` category 1, so strict mode warned on every `and(…, f())`.
`classify_argument` now returns category 1 there (a plain `unknown`
parameter keeps category 4); `design_time_call_check::categories::null_is_accepted_only_by_an_explicit_null_arm`
pins it.

### Wave 8: consumer integration

Two foreground subagents ran in parallel with disjoint files: *Composition
precheck* in a throwaway worktree seeded from the uncommitted tree (its files
were copied back and the worktree removed; `tests/l1/main.rs` merged by
hand), *Editor diagnostics* in the main tree under `darkmatter/dmls/` only.
The integrator then added the shared-corpus agreement test.

**Composition precheck** (`darkmatter/lib/src/markdown/compose/call_precheck.rs`,
hooked in `pipeline/mod.rs::run_compose_pipeline_node` after schema
preparation and caller projection, before the pre-approval clone and
frontmatter interpolation pass 1). It checks, on the composed surface only:
frontmatter string leaves exactly as pass 1 would evaluate them (skipping
`exclude_keys` and data leaves; `--set` values are authored and checked);
`::block when=` conditions and each block body under its guards (when page
blocks run); `::file`/`::code` `when=` conditions (when transclusion runs);
body `{{ … }}` through the body composer's own scan (fences and `{{{ … }}}`
excluded unless `interpolate_code_blocks`). Each transcluded child is checked
by its own pipeline node against its own schema. `MarkdownError::rejected_call()`
finds the `CallCheck` in a rejection's cause chain.

**Editor diagnostics** (`darkmatter/dmls/src/diagnostics/call_check.rs`,
wired in `providers/dsl.rs` for body interpolations and every directive
`when=`, and in `diagnostics/frontmatter.rs` for expression-typed values,
lifecycle stacks/guards, and plain frontmatter `{{ … }}` values). Severity:
5 → error, 3 → warning, strict adds 2 and 4 as warnings, 1 never. Codes
`dm.expression.incompatible_call` (error) and `dm.expression.uncertain_call`
(warning). Setting `ExpressionsConfig { strict }` (`[expressions] strict` in
`.dmls.toml`, `dmls.expressions.strict` from the editor) through the same
serde validation and layering as `schema.strict`. The `ConfigState` comment
drift (it claimed an invalid merge keeps the previous effective config; the
code falls back to the file layer) was fixed in the comment and pinned by
`config::tests::expressions_strict::an_invalid_merge_falls_back_to_the_file_layer_not_the_previous_config`;
the existing test `test_semantic_tokens_malformed_value_keeps_previous` was
renamed `…_falls_back_to_file_layer` to say what it asserts.

Existing tests changed: `compose_expression_failure_contract::preflight_discovery_does_not_fail_on_a_region_composition_removes`
used `min(1)` inside a false block as a runtime failure discovery must
tolerate; that call is now rejected at composition by criterion 49, so the
test uses `min(missing, 5)` (not provably wrong, still fails at runtime).
`type_scope_ownership_guard.rs` gained a reasoned allowlist entry for the
precheck's one unguarded `TypeScope`.

### Design Decisions (Wave 8)

9. **Composition reuses the runtime's error variants.**
   - Decision: a precheck rejection is the variant the runtime raises at the
     same place (`MarkdownError::Interpolation` for frontmatter and body,
     `PageBlock(Condition(Eval))` for a block condition,
     `TransclusionError::ConditionEval` for a transclusion condition), each
     with `ExpressionError::CallCheck` as its cause.
   - Alternatives: a new `MarkdownError` variant.
   - Why: location, excerpt, and supplier attribution keep working unchanged.
   - Downsides: callers distinguish a precheck rejection through
     `MarkdownError::rejected_call()`.
10. **A rejected transcluded child fails its parent.**
    - Decision: `transclusion_failure_is_fatal` includes `rejected_call()`.
    - Alternatives: keep the tolerant "Could not transclude" placeholder.
    - Why: criterion 37 says a category 5 call is rejected at composition;
      it is an authoring defect no runtime value repairs, like an unavailable
      binding, which is already fatal. A placeholder would let `md compose`
      exit 0 on a document the editor marks as an error.
    - Downsides: a parent that previously tolerated a broken child now fails.
11. **Block-body narrowing applies only when page blocks run.** Otherwise a
    false block's body is evaluated unguarded, so it is checked unguarded.
12. **Not checked before evaluation:** `replace`-map insertions and `{{ }}`
    inside `::shell` lines when body interpolation is off. They exist only
    after a later stage or are not evaluated on that surface; a bad call there
    still fails at runtime. Unparsable expressions are skipped; the stage that
    evaluates them reports the parse error.
13. **DMLS checks plain frontmatter `{{ … }}` values and every directive
    `when=`.** Composition checks both, and the editor must report an error
    exactly when composition rejects. The shared corpus pins agreement.
14. **Two DMLS codes, split by proven and possible.** One error code
    (category 5, arity, ambiguity) and one warning code (3, plus 2 and 4 in
    strict mode); editors suppress by code. The category number is in the
    message. Alternative: one code per category (five codes for two
    severities).
15. **"`workspace/configuration`" is DMLS's existing settings channel**
    (`workspace/didChangeConfiguration` settings under `dmls`, as
    `schema.strict` uses). DMLS does not pull configuration; adding a pull
    only for this flag would give it a second transport.
16. **The DMLS no-handler test runs in-source on the test thread**: test
    function registration is thread-local, so the LSP fixture's server
    thread would not see a test `effect()`.

### Requirement-to-test mapping (Phase 5)

"lib" is `darkmatter` `l1`, "dmls" is `dmls` `l1`. Counter is
`function_test_hooks::handler_entries`.

| Requirement | Tests |
|---|---|
| Criterion 36 (category per pair) | lib `design_time_call_check::categories::{the_worked_example_rows, criterion_36_pairs, unions_quantify_over_their_options, containers_reason_about_their_elements}` |
| Criterion 37 (category 5 rejected at composition, every function) | lib `compose_call_precheck::rejection::{a_proven_bad_call_rejects_before_any_expression_runs, the_message_reads_like_the_type_error_and_names_the_key, a_bad_body_call_rejects_before_frontmatter_runs, every_registered_function_rejects_a_proven_bad_call}`; darkmatter-cli `compose_schema_enhancements::a_proven_bad_call_in_a_dead_branch_rejects_before_any_evaluation` (real `md compose`: exit 1, located type error, the whitelisted `$(touch …)` sentinel never created) |
| Criterion 38 | lib `compose_call_precheck::conversions::a_category_1_conversion_composes_and_fails_by_value_at_runtime`; `design_time_call_check::categories::three_letter_acronym_takes_a_typed_number` |
| Criterion 40 (DMLS, both settings, file and editor, reload) | dmls `expression_call_diagnostics::settings::{rows_a_to_e_by_default, rows_a_to_e_from_the_config_file_and_its_reload, rows_a_to_e_from_editor_settings_flipped_at_runtime}` |
| Criterion 41 (ranging, agreement) | dmls `expression_call_diagnostics::ranges::*`, `call_check_agreement::composition_and_the_editor_reject_the_same_documents` |
| Criterion 42 (`file_exists`) | lib `design_time_call_check::categories::null_is_accepted_only_by_an_explicit_null_arm`; dmls `verdicts::file_exists_takes_a_number_silently`, `config::tests::expressions_strict::strict_is_the_only_expressions_setting` |
| Criterion 44 (`length(true)` category 5) | lib `design_time_call_check::categories::source_exclusions_are_part_of_classification` |
| Criterion 48 (projection) | lib `design_time_call_check::projection::*` (projection takes only a schema reference: it has no file, value, or expression input to read) |
| Criterion 49 (dead branches) | lib `design_time_call_check::walk::branches_that_do_not_run_are_checked`, `compose_call_precheck::branches::*`; dmls `verdicts::branches_that_do_not_run_are_checked` |
| Criterion 50 (presence, `fm.` spellings) | lib `design_time_call_check::projection::presence_follows_the_shared_rule`, `walk::fm_spellings_reject_like_the_bare_name`, `compose_call_precheck::rejection::frontmatter_namespace_access_rejects_like_the_bare_name` |
| Criterion 51 (recovery) | lib `design_time_call_check::whole_calls::numeric_recovery`, `compose_call_precheck::conversions::numeric_recovery_composes_and_an_unrecoverable_call_is_rejected`; dmls `verdicts::numeric_recovery` |
| Criterion 52 (content formats) | lib `design_time_call_check::categories::content_formats` |
| Criterion 53 (laziness) | lib `design_time_call_check::walk::checking_never_enters_a_handler`, `compose_call_precheck::branches::lazy_operands_stay_lazy_and_are_still_checked`; dmls in-source `diagnostics::call_check::tests::diagnostics_never_enter_a_handler` |
| Criterion 54 (surface, ordering) | lib `compose_call_precheck::surface::{non_executable_syntax_is_not_checked, frontmatter_only_composition_leaves_an_invalid_body_untouched, frontmatter_only_composition_still_checks_frontmatter}`; sibling effect counters in every `rejected_before_evaluation` test; dmls `verdicts::fenced_code_and_literals_are_not_checked` |
| Rows f–i stay unrejected; g/h exact | lib `design_time_call_check::categories::{the_worked_example_rows, guards_narrow_the_argument}`, `compose_call_precheck::branches::conservative_rows_are_not_rejected`; dmls `verdicts::guards_narrow_rows_g_and_h` |
| Nested transclusion | lib `compose_call_precheck::transclusion::*` |
| `expressions.strict` input matrix | dmls `config::tests::expressions_strict::*`: absent and empty section → false; file true/false; editor overrides file both ways, whole-settings `null` clears the overlay; explicit null, wrong type (`"yes"`, `1`, `expressions = 5`) → error diagnostic and file-layer fallback; TOML duplicate key and duplicate table reject the file; malformed file recovers on reload |
| Shared-corpus agreement (checkpoint) | dmls `call_check_agreement::composition_and_the_editor_reject_the_same_documents` (24 documents) |

**Matrix review.** Field `expressions.strict` goes through the same serde
validator as `schema.strict`/`style.strict` (no `#[serde(default)]` on the
field read as a value; the section's default is `false` only when absent).
Every field shape above is covered at the config API, and wiring is covered
by the LSP-session tests. One quirk is recorded, not changed: an editor value
`"expressions": [true]` deserializes as a struct, as every existing section
does (serde's sequence-as-struct form).

### Gates

The scope calculator, run from the repository root after the last edit:
`git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`.
It gave `packages: [darkmatter, darkmatter-cli, dmls, test-toolkit]` and
`change_class: package`, so every gate was required. The gates ran serially,
with no subagent running.

| Gate | Result |
|---|---|
| `darkmatter`: `just lint` | clean |
| `darkmatter`: `just test` | library and CLI: every test passed; it stopped at 11,521 passed and 1 failed. The failure was the known load-sensitive `dmls::l1 searched_candidates::a_watched_nearer_candidate_reselects_the_import_and_its_removal_falls_back` (one watch registration too many, 3 instead of 2). It passes alone and in its module. |
| `dmls` full rerun (the tests fail-fast skipped) | 1,237 passed, 6 skipped. The same test failed again under full-crate load and passes alone; no code this phase changed touches watch registration. |
| `zed-dmls-cli` | 27 passed |
| `claudine`: `just lint` | clean (only the existing linker `__eh_frame` notice) |
| `claudine`: `just test` | 10,692 passed, 9 skipped, 0 failed |
| `just check-tier-coverage claudine` / `darkmatter` | 0 stranded / 0 stranded |
| `test-toolkit` (selected by the calculator) | 476 passed |
| Dependents: `messenger` | 738 passed |
| Dependents: research, messenger-cli, sniff-cli, playa-cli, biscuit-icon-cli, biscuit-speaks-cli, claudine-gen | `cargo check --tests` clean; `research` uses only `darkmatter::render` |

One subagent run used `--no-fail-fast` once (a scoped sweep of 1,353 tests),
against the brief; it caused no load problem.

**Cross-OS.** No path, process, console, or `#[cfg]` code changed. The new
tests compose in temp dirs and spawn `md` only through the existing CLI
fixture, so no `just cross-check` was run; CI covers the other environments.

**Docs and skills.** The tasks wrote a new page,
`darkmatter/docs/inline/call-check.md`, and added a stage to
`darkmatter-compose-pipeline.md`. `dmls/docs/diagnostics.md` gained the two
codes, the severity table, and the setting. Skills updated: `darkmatter`
(`SKILL.md` pipeline order, `expressions.md` call check) and `claudine`
(`SKILL.md`). Phase 6 owns the complete documentation pass.

## Phase 6

Phase 6 documents, audits, and verifies (Waves 9–10). Started 2026-10-22
from the uncommitted Phase 5 tree (Phase 5 is not yet committed, so the
working tree holds both phases; the Phase 6 file lists in the frontmatter
come from a before/after `git status` comparison plus each task's report).

### Wave 9: documentation and coverage audit

Three foreground subagents ran in parallel with disjoint files.

**Current documentation** (Darkmatter docs and the `darkmatter` skill).
`definition.md` "Type Coercion" is now the single rule list: two interfaces
of a type, three actions, conversion rules with the rejected conversions,
nested values (container rules), choosing a union option (renamed from
"Unions", whose anchor clashed with the grammar section), binding before
computing, type errors with real output, raw JSON Schema conversion, and
changes for existing documents. The `numberlike`/`boolish` rows say the
value is kept as written; the content-format coercion bullet and two stale
"Limitations" entries (no `(A | B)[]`, "no coercions beyond the matrix")
are gone; display-only and file-form examples and `fallible` are
documented. `authoring-schemas.md` got content for its empty "Union Types"
heading and allows `categories:` in the envelope. `inline/call-check.md` has
the five categories for authors, a Mermaid flow, runnable examples, and
"What the check cannot see" (each limit checked against the code).
`numbers.md` is the one numeric page; both dependency docs describe
`preserve_order` (insertion order is not source order everywhere; sort when
order must be stable; `md schema detect --format json` and `json` text keep
the written order; other validators ignore Darkmatter's formats). Every
claim was checked with a debug `md` on temp files.

**Consumer documentation** (DMLS and Claudine docs, the `claudine` skill,
`darkmatter/dmls.md`). `dmls/docs/diagnostics.md` has "Call diagnostics"
under Severity (example, category → severity table, proven versus possible,
types from `$schema` only, every branch, Darkmatter decides) and a "Fixing a
call diagnostic" table; Configuration has `expressions.strict` (file and
editor forms, precedence, `null` and invalid values, reload).
`docs/lsp/configuration.md` lists `[expressions]`. Several DMLS docs said
DMLS reads `workspace/configuration`; they now name `didChangeConfiguration`.
Four DMLS docs carried `features:` frontmatter naming specs and two editor
docs linked into `features/`; both removed (docs never name a spec). The
`claudine context --expressions` report already met criterion 39 and the
example forms, so the renderer is unchanged; `cargo run -p darkmatter
--example expression_doc_generator` matched the doc table.

**Coverage audit** (`inventory/criteria-map.md`, `behavior-ledger.md`). All
61 IDs resolve to existing, passing tests: 53 pass, 7 pass with a review
clause the audit performed (7, 11, 33b, 41, 42, 48, 58), and criterion 13
waits on the author. The audit added
`…catalog::tests::{executable_examples_number_at_least_72_and_display_only_ones_are_skipped_by_design, example_results_compare_structurally}`
(criteria 8 and 9 had no count or structural-comparison test: 100
executable, 43 display-only). The ledger had 113 rows for 116 registered
functions; rows 114–116 (`trim`, `keys`, `values`) were added with their
null outcomes, checked through `md compose`.

### Integrator follow-ups (after the audit)

The audit and the doc tasks reported gaps; each was fixed rather than filed:

- `example(number() => error)` added to the catalog (criterion 12's arity
  case had no catalog example); the generated table in
  `darkmatter-expressions.md` regenerated (only that row changed).
- Catalog loader: an entry written as `null`, `~`, or empty now fails with
  "`types.<name>` is null, not a function signature" (ruling R5). Tests
  `a_missing_null_or_wrong_kind_does_not_load` (absent, `null`, `~`, `""`,
  wrong string, number, duplicate key, second document, trailing invalid
  content) and `an_entry_written_as_null_does_not_load`.
- DMLS `expressions.strict` matrix: `empty_values_are_wrong_types_not_absent`,
  `a_list_section_is_read_positionally_as_every_section_is`,
  `a_syntactically_broken_file_is_rejected_whole`, `an_unknown_key_is_ignored`
  (R6).
- Type-error messages printed an expression literal `0` as `received number
  0.0` (`pr_list(0)`); they now present numbers as interpolation does.
  `function_call_binding::a_received_number_reads_as_interpolation_presents_it`.
- `inline/call-check.md` quoted the message as `argument 1 (a)`; the real
  text is ``argument 1 (`a`)``.
- Gate defect from Phase 5: `dmls::l1 type_scope_ownership_guard` failed on
  two unreviewed `TypeScope::unguarded` sites Phase 5 added
  (`providers/dsl.rs::expression_diagnostics`,
  `diagnostics/frontmatter.rs::FrontmatterCalls::new`). Both are correct
  (the body site narrows by `BodyGuards::scope_at`; frontmatter has no
  guard), so they got reasoned allowlist entries, a decision-site rule for
  the body site, and baseline snippets in the guard's self-test.

### Design Decisions

17. **The catalog loader rejects a null entry.**
    - Decision: `load_catalog` fails on a `types:` entry written as null,
      `~`, or empty; general schema files keep reading it as the `null` type.
    - Alternatives: keep the later registry-build error
      (`BindingWithoutCatalog`, "runtime binding has no catalog function").
    - Why: ruling R5 places this E in the loader; the later error names the
      wrong cause and stage. A null named type has no use in the catalog.
    - Downsides: none found.
18. **A missing canonical stays a registry-build error.**
    - Decision: the loader does not check registered names.
    - Alternatives: pass the registered canonicals into `load_catalog`.
    - Why: the loader is generic over fixtures and knows no registry; the
      shipped catalog is pinned both ways by
      `catalog_and_runtime_bindings_have_bidirectional_canonical_parity`.
    - Downsides: R5's "a missing canonical is E" is met at registry build,
      not at catalog load.
19. **Messages present a float number as interpolation does.**
    - Decision: `coercion::compact` (shared by every coercion and binding
      message) writes a float `Value::Number` through `format_number`;
      integer-stored numbers keep serde's exact text.
    - Alternatives: leave serde's `0.0`; convert every literal to an integer
      `Number` at parse time (wider change to evaluation values).
    - Why: the spec's numeric model presents `4.0` as `4`; the message is
      what authors read.
    - Downsides: a number nested in a list or object in a message keeps
      serde's spelling.
20. **The sequence-as-struct reading of a list section is pinned, not
    changed.** `"expressions": [true]` sets strict, as every DMLS section
    reads a list positionally (Phase 5 recorded the quirk). Changing it
    belongs to all sections at once.

### Departures the docs follow (code wins)

- A root union whose arms are all SimplifiedSchema still projects a
  property when every arm that declares it agrees; only a raw JSON Schema
  arm or disagreeing arms make variables unknown (the spec says a mixed
  root union projects everything as unknown, which holds for the mixed
  case).
- A native YAML number that cannot be represented exactly fails when the
  document loads, whatever the property's type (Phase 2 decision 4).
- `[true]` against `(string | number)[]` composes as `["true"]` (Phase 3
  decision 1); the spec's "rejects `[true]`" holds for the compiled JSON
  Schema validator, which the doc states.
- The editor setting arrives through `workspace/didChangeConfiguration`,
  not a `workspace/configuration` pull (decision 15). An invalid editor
  value drops all editor settings except `formatting`, not only
  `expressions.strict`.

### Requirement-to-test mapping (Phase 6)

The final map for criteria 1–59, 33a and 33b is
`inventory/criteria-map.md`. Phase 6 additions:

| Requirement | Tests |
|---|---|
| Criterion 8 (≥ 72 executable, structural comparison) | `darkmatter` lib `…expression::catalog::tests::{executable_examples_number_at_least_72_and_display_only_ones_are_skipped_by_design, example_results_compare_structurally}` |
| Criterion 9 (display-only skipped by design) | same first test |
| Criterion 12 (arity example) | catalog `number() => error`, run by `…catalog::tests::every_executable_example_evaluates_to_its_declared_result` |
| R5 catalog envelope cells | `…catalog::tests::{a_missing_null_or_wrong_kind_does_not_load, an_entry_written_as_null_does_not_load}` |
| `expressions.strict` remaining cells, R6 | `dmls` lib `config::tests::expressions_strict::{empty_values_are_wrong_types_not_absent, a_list_section_is_read_positionally_as_every_section_is, a_syntactically_broken_file_is_rejected_whole, an_unknown_key_is_ignored}` |
| Message presentation (criterion 16's received value) | `darkmatter` l1 `function_call_binding::a_received_number_reads_as_interpolation_presents_it` |
| Type-scope ownership of the call-check sites | `dmls` l1 `type_scope_ownership_guard::*` |

### Review package

- The old catalog `darkmatter/docs/schemas/expression-functions.yaml` is
  deleted and nothing reads it (criterion 7); the catalog compiles once
  (`LazyLock` in `expression_function_catalog`); production conversion has
  one owner (`coercion_engine_parity::old_frontmatter_rule_code_is_gone`).
- **Human ledger review is pending** (criterion 13): the author's
  2026-10-22 review covered rows 1–113; rows 114–116 (`trim`, `keys`,
  `values`) and the closing per-function table in `behavior-ledger.md`
  need the author's review before merge.
- Unrelated pre-existing changes in the tree were left untouched.

### Observed, not part of this feature

- `topics/schemas/schema-types.md`, `schema-constraints.md`,
  `schema-validation.md`, and `topics/expression-engine.md` are empty stub
  pages that predate this feature; links to them now point into
  `definition.md`.
- `md` keeps ANSI color in error output under `NO_COLOR=1`.
- The `pr_list`/`cicd_list` descriptions link to
  `darkmatter-expressions.md#provider-query-vocabulary`; in the terminal
  report that relative link resolves against the current directory.
- `.claude/skills/darkmatter/SKILL.md` is 217 lines (over the 200-line
  target before this phase, at 213).

### Gates

The scope calculator, run from the repository root after the last edit
(`git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`),
gave `packages: [darkmatter, darkmatter-cli, dmls, test-toolkit]` and
`change_class: package`, so every gate was required. Gates ran serially with
no subagent running.

| Gate | Result |
|---|---|
| `darkmatter`: `just lint` | clean (before and after the follow-up test edits) |
| `darkmatter`: `just test`, run 1 | stopped at 11,536 of 11,644: the known load-sensitive `dmls::l1 searched_candidates::a_watched_nearer_candidate_reselects_the_import_and_its_removal_falls_back`; passes alone |
| `darkmatter`: `just test`, run 2 | stopped at 11,565: `dmls::l1 type_scope_ownership_guard::dmls_reads_types_only_through_the_library_scope` (the Phase 5 defect above; fixed) |
| `darkmatter`: `just test`, run 3 | 11,644 passed, 10 skipped |
| `claudine`: `just lint` | clean (only the existing linker `__eh_frame` notice) |
| `claudine`: `just test` | 10,692 passed, 9 skipped |
| `just check-tier-coverage claudine` / `darkmatter` | 0 stranded / 0 stranded |
| Dependents (root `just test {pkg}`) | `test-toolkit` 476, `sniff-cli` 975, `messenger` 738, `messenger-cli` 133, `research` 606: all passed |
| Dependents (`cargo check --tests`) | `research-cli`, `biscuit-icon-cli`, `playa-cli`, `biscuit-speaks-cli`, `claudine-gen`: clean |

The dependent list is the audit's `preserve_order` consumer list
(`inventory/criteria-map.md` "Downstream gates"). No other package's tests
pin the changed message text (repository-wide search).

**Cross-OS.** Phase 6 changed message text, a YAML loader check, and tests;
no path, process, console, or `#[cfg]` code. No `just cross-check` was run.
Evidence is macOS only; Linux and macOS are proven by the pull request's CI,
native Windows and WSL2 by the nightly. No other environment is claimed.

## Pipeline Refresh

- `2026-10-29-recursive-schema-types`: changed. Spec names the real
  translators and limits (`json_shape.rs` `CYCLE_EXPANSIONS`/`MAX_REFERENCE_DEPTH`/`REFERENCE_BUDGET`,
  `call_binding/translate.rs` `TooDeep`, engine `max_depth`), adds
  `is_subtype`, `DeclaredTypes::top_level_types` and `call_check` as
  consumers that must terminate on recursion, extends the callable ban to
  `$defs`; three decisions recorded; plan Waves 1, 4, 5 own those files.
- `2026-10-30-claudine-recursive-schemas`: changed. Names the catalog API,
  the shared engine, `bind_call`, and `BindOptions.defer`; new
  reconciliation section (overload choice by `bind_call`, inline helper
  types, no generated schema for callback parameters); plan rule 9 and
  Waves 2–4 updated. Noted, not caused here: its plan maps only criteria
  1–9 of 11.
- `2026-10-31-type-aware-evaluation`: changed (spec, plan,
  `declarations-design.md`). Corrects `file_exists` to `unknown | null`
  (inspects, `file_exists(42)` is `false`), `is_file_type` to
  `unknown | null`, names the projection function, corrects the root-union
  projection, uses the real code `dm.expression.incompatible_call`; plan
  Waves 8 and 11 updated. Noted, not caused here: its plan's Phase 3 builds
  a narrowing engine its spec says `TypeScope` already provides.
- `2026-11-01-map-and-filter`: changed (Phase 1 done; only Phases 2–5
  edited). Names the loader, `RegistrationError::FunctionValued`, the
  example parser; decisions for callback-body call checking, non-array
  iterables, and verb-inventory exclusion.
- `2026-10-22-spec-dependency-gate`: checked, unchanged (names this spec
  only in a sample output that is still accurate).

No downstream decision meets the bar for the author (`decisions_for_author: []`).
