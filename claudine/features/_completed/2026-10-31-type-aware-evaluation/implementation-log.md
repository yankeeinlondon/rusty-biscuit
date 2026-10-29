---
spec: "/Volumes/coding/wt/rusty-biscuit/feat-pipeline/claudine/features/2026-10-31-type-aware-evaluation/spec.md"
plan: "claudine/features/2026-10-31-type-aware-evaluation/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages:
    - darkmatter
    - darkmatter-cli
    - dmls
    - claudine
    - biscuit-file
    - renderable
    - content-policy
human_review: false
source_files_during_phase_2:
    - darkmatter/lib/src/markdown/compose/expression/scope/shape.rs
    - darkmatter/lib/src/markdown/compose/expression/scope/projection.rs
    - darkmatter/lib/src/markdown/compose/expression/scope/raw.rs
    - darkmatter/lib/src/markdown/compose/expression/scope/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/call_check.rs
    - darkmatter/lib/src/markdown/compose/expression/call_binding/signature.rs
    - darkmatter/lib/src/markdown/compose/call_precheck.rs
    - darkmatter/lib/src/markdown/compose/directive_targets.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/site.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/template/typing.rs
    - darkmatter/lib/src/markdown/compose/preflight/collect.rs
    - darkmatter/lib/src/markdown/compose/schema_validation.rs
    - darkmatter/lib/src/markdown/compose/mod.rs
    - darkmatter/lib/src/markdown/schemas/mod.rs
    - darkmatter/lib/src/lifecycle/error.rs
    - darkmatter/lib/src/lifecycle/validate.rs
    - darkmatter/lib/src/lifecycle/mod.rs
    - darkmatter/lib/src/lifecycle/registry.rs
    - darkmatter/lib/src/lifecycle/shell_template.rs
    - darkmatter/lib/tests/l1/variable_type_projection.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/design_time_call_check.rs
    - darkmatter/lib/tests/l1/expression_type_scope.rs
    - darkmatter/lib/tests/l1/directive_target_analysis.rs
    - darkmatter/lib/tests/l1/type_scope_ownership_guard.rs
    - darkmatter/cli/src/commands/compose.rs
    - darkmatter/cli/src/commands/compose/lifecycle.rs
    - darkmatter/cli/tests/l1/compose_lifecycle.rs
    - darkmatter/dmls/src/diagnostics/call_check.rs
    - darkmatter/dmls/src/providers/dsl.rs
    - darkmatter/dmls/tests/l1/expression_call_diagnostics.rs
    - darkmatter/dmls/tests/l1/expression_scopes.rs
    - claudine/lib/src/composition/error/lifecycle_authoring.rs
    - claudine/lib/src/composition/error/mod.rs
    - claudine/lib/src/composition/error/render/lifecycle.rs
    - claudine/lib/src/composition/error/render/mod.rs
    - claudine/lib/src/composition/lifecycle/mod.rs
    - claudine/lib/src/composition/lifecycle/registry.rs
    - claudine/lib/src/composition/lifecycle/tests/mod.rs
    - claudine/lib/src/composition/lifecycle/tests/call_check.rs
    - claudine/lib/src/composition/prepare.rs
    - claudine/lib/src/composition/sequence/preflight/mod.rs
    - claudine/lib/tests/l1/type_scope_ownership_guard.rs
docs_updated_during_phase_2:
    - darkmatter/docs/inline/call-check.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
    - .claude/skills/darkmatter/expressions.md
    - .claude/skills/claudine/SKILL.md
source_files_during_phase_3:
    - darkmatter/lib/src/markdown/compose/expression/call_binding/registration.rs
    - darkmatter/lib/src/markdown/compose/expression/call_binding/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/paths.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/predicates.rs
    - darkmatter/lib/src/markdown/compose/expression/scope/facts.rs
    - darkmatter/lib/src/markdown/compose/expression/scope/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/scope/shape.rs
    - darkmatter/lib/src/markdown/compose/expression/scope/tests.rs
    - darkmatter/lib/src/lifecycle/actions.rs
    - darkmatter/lib/src/lifecycle/mod.rs
    - darkmatter/lib/src/lifecycle/source_map.rs
    - darkmatter/lib/src/lifecycle/validate.rs
    - darkmatter/lib/tests/l1/flow_sensitive_narrowing.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/type_scope_ownership_guard.rs
    - darkmatter/dmls/src/diagnostics/call_check.rs
    - darkmatter/dmls/src/diagnostics/frontmatter.rs
    - darkmatter/dmls/src/overlay/event_views.rs
    - darkmatter/dmls/tests/l1/expression_call_diagnostics.rs
    - darkmatter/dmls/tests/l1/type_scope_ownership_guard.rs
    - claudine/lib/src/composition/lifecycle/tests/call_check.rs
    - claudine/lib/tests/l1/type_scope_ownership_guard.rs
docs_updated_during_phase_3:
    - darkmatter/docs/topics/darkmatter-expressions.md
    - darkmatter/docs/inline/call-check.md
    - darkmatter/dmls/docs/diagnostics.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
    - .claude/skills/darkmatter/expressions.md
    - .claude/skills/claudine/SKILL.md
source_files_during_phase_4:
    - biscuit-file/lib/src/file_reference/parse.rs
    - biscuit-file/lib/src/file_reference/mod.rs
    - biscuit-file/lib/tests/l1/structural_validation.rs
    - biscuit-file/lib/tests/l1/main.rs
    - darkmatter/lib/src/effects/verbs.rs
    - darkmatter/lib/src/markdown/compose/call_precheck.rs
    - darkmatter/lib/src/markdown/compose/pipeline/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/call_check.rs
    - darkmatter/lib/src/markdown/compose/expression/resolve_ctx.rs
    - darkmatter/lib/src/markdown/compose/expression/call_binding/registration.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/fs.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/fs_tests.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/paths.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/predicates.rs
    - darkmatter/schemas/partials/functions.yaml
    - darkmatter/schemas/partials/lifecycle.yaml
    - darkmatter/schemas/partials/md-lifecycle.yaml
    - darkmatter/lib/tests/l1/file_predicates.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/function_binding_registry.rs
    - darkmatter/lib/tests/l1/lifecycle_schema_generation.rs
    - darkmatter/cli/tests/l1/compose_file_predicates.rs
    - darkmatter/cli/tests/l1/main.rs
    - darkmatter/dmls/src/diagnostics/call_check.rs
    - darkmatter/dmls/src/diagnostics/codes.rs
    - darkmatter/dmls/tests/l1/expression_call_diagnostics.rs
    - claudine/schemas/action.yaml
    - claudine/schemas/lifecycle.yaml
    - darkmatter/lib/src/markdown/compose/expression/scope/tests.rs
    - darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/darkmatter-schemas-darkmatter-schema-body.digest
    - darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/darkmatter-schemas-partials-fm.digest
    - darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/effective-baseline-inline.digest
    - darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/effective-baseline-root-union.digest
    - darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/effective-baseline-schema-file.digest
    - darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/embedded-darkmatter-base.digest
    - claudine/cli/tests/l1/snapshots/l1__context_command__context_expressions_function_order_snapshot.snap
docs_updated_during_phase_4:
    - darkmatter/docs/topics/darkmatter-expressions.md
    - darkmatter/dmls/docs/diagnostics.md
    - biscuit-file/docs/topics/file-references.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
    - .claude/skills/darkmatter/expressions.md
    - .claude/skills/claudine/SKILL.md
source_files_during_phase_5:
    - darkmatter/lib/src/markdown/compose/expression/host/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/host/artifact.rs
    - darkmatter/lib/src/markdown/compose/expression/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/test_hooks.rs
    - darkmatter/lib/src/markdown/compose/expression/call_check.rs
    - darkmatter/lib/src/markdown/compose/expression/absence.rs
    - darkmatter/lib/src/markdown/schemas/catalog.rs
    - darkmatter/lib/tests/l1/host_functions.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/fixtures/host_functions/project.yaml
    - darkmatter/lib/tests/fixtures/host_functions/project.facts.json
    - darkmatter/dmls/src/diagnostics/codes.rs
    - darkmatter/dmls/src/diagnostics/call_check.rs
    - darkmatter/dmls/src/diagnostics/frontmatter.rs
    - darkmatter/dmls/src/providers/dsl.rs
    - darkmatter/dmls/tests/l1/host_function_diagnostics.rs
    - darkmatter/dmls/tests/l1/main.rs
    - claudine/lib/src/composition/looping/config.rs
    - claudine/lib/src/composition/looping/config/tests.rs
    - claudine/lib/src/composition/reserved.rs
    - claudine/lib/src/composition/lifecycle/bindings.rs
    - claudine/lib/src/composition/lifecycle/bindings/tests.rs
    - claudine/lib/src/composition/lifecycle/bindings/rename.rs
    - claudine/lib/src/composition/lifecycle/bindings/rename/tests.rs
    - claudine/lib/src/composition/sequence/preflight/shape.rs
    - claudine/lib/src/composition/sequence/preflight/tests.rs
    - claudine/lib/tests/l1/type_scope_ownership_guard.rs
    - claudine/lib/tests/l1/host_catalog.rs
docs_updated_during_phase_5:
    - darkmatter/docs/topics/darkmatter-expressions.md
    - darkmatter/dmls/docs/diagnostics.md
docs_created_during_phase_5:
    - darkmatter/docs/topics/host-functions.md
skills_files_updated_during_phase_5:
    - .claude/skills/darkmatter/expressions.md
    - .claude/skills/claudine/SKILL.md
source_files_during_phase_6:
    - darkmatter/lib/src/markdown/compose/expression/diagnostics.rs
    - darkmatter/lib/src/markdown/compose/expression/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/absence.rs
    - darkmatter/lib/src/markdown/compose/expression/call_binding/registration.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/predicates.rs
    - darkmatter/lib/src/markdown/compose/call_precheck.rs
    - darkmatter/lib/src/markdown/compose/context/report.rs
    - darkmatter/lib/src/markdown/compose/frontmatter_interpolation.rs
    - darkmatter/lib/src/markdown/compose/mod.rs
    - darkmatter/lib/src/markdown/types.rs
    - darkmatter/lib/src/markdown/mod.rs
    - darkmatter/lib/src/lifecycle/validate.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/shared_diagnostics.rs
    - darkmatter/lib/tests/l1/unknown_identifier_warning.rs
    - darkmatter/lib/tests/l1/host_functions.rs
    - darkmatter/lib/tests/l1/variable_type_projection.rs
    - darkmatter/cli/src/commands/compose.rs
    - darkmatter/cli/src/commands/compose/lifecycle.rs
    - darkmatter/cli/tests/l1/compose_call_warnings.rs
    - darkmatter/cli/tests/l1/main.rs
    - darkmatter/dmls/src/diagnostics/call_check.rs
    - darkmatter/dmls/src/diagnostics/codes.rs
    - darkmatter/dmls/src/diagnostics/frontmatter.rs
    - darkmatter/dmls/src/providers/dsl.rs
    - darkmatter/dmls/tests/l1/expression_call_diagnostics.rs
    - darkmatter/dmls/tests/l1/expression_scopes.rs
    - darkmatter/dmls/tests/l1/lsp_session.rs
    - claudine/lib/src/composition/lifecycle/mod.rs
    - claudine/lib/src/composition/lifecycle/tests/call_check.rs
    - claudine/lib/src/composition/prepare.rs
docs_updated_during_phase_6:
    - darkmatter/docs/inline/call-check.md
    - darkmatter/docs/inline/interpolation.md
    - darkmatter/docs/topics/schemas/parsing/grammar.md
    - darkmatter/dmls/docs/diagnostics.md
docs_created_during_phase_6: []
skills_files_updated_during_phase_6:
    - .claude/skills/darkmatter/compose.md
    - .claude/skills/darkmatter/dmls.md
    - .claude/skills/darkmatter/expressions.md
    - .claude/skills/claudine/SKILL.md
source_files_during_phase_7:
    - darkmatter/cli/src/args/cli.rs
    - darkmatter/cli/src/args/command.rs
    - darkmatter/cli/src/args/mod.rs
    - darkmatter/cli/src/args/policy.rs
    - darkmatter/cli/src/args/target.rs
    - darkmatter/cli/src/artifact.rs
    - darkmatter/cli/src/commands/clean.rs
    - darkmatter/cli/src/commands/compose.rs
    - darkmatter/cli/src/commands/compose/lifecycle.rs
    - darkmatter/cli/src/commands/mod.rs
    - darkmatter/cli/src/commands/render.rs
    - darkmatter/cli/src/commands/schema/validate.rs
    - darkmatter/cli/src/main.rs
    - darkmatter/cli/src/render.rs
    - darkmatter/cli/tests/l1/claudine_schema_bundle.rs
    - darkmatter/cli/tests/l1/clean_schema.rs
    - darkmatter/cli/tests/l1/compose_base_schema.rs
    - darkmatter/cli/tests/l1/compose_call_warnings.rs
    - darkmatter/cli/tests/l1/compose_dasherized_identifiers.rs
    - darkmatter/cli/tests/l1/compose_icmp.rs
    - darkmatter/cli/tests/l1/compose_refs_and_missing.rs
    - darkmatter/cli/tests/l1/compose_remote_caching.rs
    - darkmatter/cli/tests/l1/compose_schema.rs
    - darkmatter/cli/tests/l1/compose_shell_template.rs
    - darkmatter/cli/tests/l1/compose_shell.rs
    - darkmatter/cli/tests/l1/compose_state_set.rs
    - darkmatter/cli/tests/l1/layout_fill.rs
    - darkmatter/cli/tests/l1/layout_style_frontmatter.rs
    - darkmatter/cli/tests/l1/literal_glob_hint.rs
    - darkmatter/cli/tests/l1/main.rs
    - darkmatter/cli/tests/l1/policy_verbs.rs
    - darkmatter/cli/tests/l1/schema_robustness_matrix.rs
    - darkmatter/cli/tests/l1/schema_trigger_rename.rs
    - darkmatter/cli/tests/l1/schema_triggers.rs
    - darkmatter/cli/tests/l1/schema_validate_baseline_selection.rs
    - darkmatter/cli/tests/l1/schema_validate_shipped_artifacts.rs
    - darkmatter/cli/tests/l1/schema_validate_timings.rs
    - darkmatter/cli/tests/l1/schema_validate.rs
    - darkmatter/cli/tests/l1/standalone_baseline.rs
    - darkmatter/lib/benches/schema_validation.rs
    - darkmatter/lib/src/markdown/block/hr_parser.rs
    - darkmatter/lib/src/markdown/compose/call_precheck.rs
    - darkmatter/lib/src/markdown/compose/context/options.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/network.rs
    - darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion/suffix.rs
    - darkmatter/lib/src/markdown/compose/icmp.rs
    - darkmatter/lib/src/markdown/compose/mod.rs
    - darkmatter/lib/src/markdown/compose/pipeline/mod.rs
    - darkmatter/lib/src/markdown/compose/pipeline/phases.rs
    - darkmatter/lib/src/markdown/compose/policy.rs
    - darkmatter/lib/src/markdown/compose/preflight/collect.rs
    - darkmatter/lib/src/markdown/compose/remote.rs
    - darkmatter/lib/src/markdown/compose/shell_blocks/types.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/mod.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/types.rs
    - darkmatter/lib/src/markdown/compose/transclusion/engine.rs
    - darkmatter/lib/src/markdown/errors/blocks.rs
    - darkmatter/lib/src/markdown/schemas/advisory.rs
    - darkmatter/lib/src/markdown/schemas/clean.rs
    - darkmatter/lib/src/markdown/types.rs
    - darkmatter/lib/src/style/parse.rs
    - darkmatter/lib/tests/l1/blocking_policy.rs
    - darkmatter/lib/tests/l1/clean_counters.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/schemas/partials/functions.yaml
docs_updated_during_phase_7:
    - content-policy/docs/topics/policy-lifecycle.md
    - darkmatter/cli/README.md
    - darkmatter/docs/cli/clean.md
    - darkmatter/docs/cli/compose.md
    - darkmatter/docs/cli/index.md
    - darkmatter/docs/inline/fm-shell-expansion.md
    - darkmatter/docs/inline/preflight-checks.md
    - darkmatter/docs/inline/schema-validation.md
    - darkmatter/docs/inline/shell-expansion.md
    - darkmatter/docs/lsp/features.md
    - darkmatter/docs/rendering/disclosure.md
    - darkmatter/docs/rendering/hr.md
    - darkmatter/docs/rendering/style.md
    - darkmatter/docs/topics/caching.md
    - darkmatter/docs/topics/darkmatter-expressions.md
    - darkmatter/docs/topics/layouts-and-style.md
    - darkmatter/docs/topics/markdown-formatting.md
    - darkmatter/docs/topics/remote-url-references.md
    - darkmatter/docs/topics/schemas/definition.md
    - darkmatter/docs/topics/state-management/context-variables.md
    - darkmatter/docs/topics/transclusion.md
    - darkmatter/docs/transclusion/block-transclusion.md
    - prompts/_add/add-expressions.md
    - renderable/docs/layout-and-style.md
    - renderable/README.md
docs_created_during_phase_7:
    - darkmatter/docs/cli/policy.md
skills_files_updated_during_phase_7:
    - .claude/skills/claudine/research-contracts.md
    - .claude/skills/darkmatter/SKILL.md
    - .claude/skills/darkmatter/compose.md
    - .claude/skills/darkmatter/frontmatter.md
    - .claude/skills/darkmatter/remote-cache.md
    - .claude/skills/darkmatter/schema.md
source_files_during_phase_8:
    - darkmatter/lib/src/markdown/compose/report_diagnostics.rs
    - darkmatter/lib/src/markdown/compose/mod.rs
    - darkmatter/lib/src/markdown/compose/context/report.rs
    - darkmatter/lib/src/markdown/compose/pipeline/mod.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/types.rs
    - darkmatter/lib/src/markdown/schemas/mod.rs
    - darkmatter/lib/tests/l1/report_diagnostics.rs
    - darkmatter/lib/tests/l1/variable_type_projection.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/dmls/src/diagnostics/codes.rs
    - darkmatter/dmls/tests/l1/expression_call_diagnostics.rs
    - darkmatter/cli/src/args/enums.rs
    - darkmatter/cli/src/args/mod.rs
    - darkmatter/cli/src/args/command.rs
    - darkmatter/cli/src/commands/mod.rs
    - darkmatter/cli/src/commands/compose.rs
    - darkmatter/cli/src/commands/compose/lifecycle.rs
    - darkmatter/cli/src/commands/compose/report.rs
    - darkmatter/cli/src/main.rs
    - darkmatter/cli/tests/l1/compose_report_json.rs
    - darkmatter/cli/tests/l1/main.rs
docs_updated_during_phase_8:
    - darkmatter/docs/cli/compose.md
    - darkmatter/docs/cli/index.md
docs_created_during_phase_8:
    - darkmatter/docs/cli/report-json.md
skills_files_updated_during_phase_8:
    - .claude/skills/darkmatter/SKILL.md
source_files_during_phase_9:
    - claudine/docs/research/agent-cli/_research.md
    - claudine/docs/research/non-interactive-sessions/_research.md
    - claudine/docs/research/reasoning-level/_research.md
    - claudine/docs/research/steering/_research.md
docs_updated_during_phase_9:
    - darkmatter/README.md
    - darkmatter/docs/cli/policy.md
    - darkmatter/docs/topics/darkmatter-expressions.md
    - darkmatter/docs/topics/compose-requests.md
    - darkmatter/docs/lsp/configuration.md
    - darkmatter/dmls/docs/diagnostics.md
    - claudine/docs/topics/schemas.md
docs_created_during_phase_9: []
skills_files_updated_during_phase_9:
    - .claude/skills/biscuit-file/SKILL.md
    - .claude/skills/claudine/SKILL.md
source_code:
    - biscuit-file/lib/src/file_reference/mod.rs
    - biscuit-file/lib/src/file_reference/parse.rs
    - biscuit-file/lib/tests/l1/main.rs
    - biscuit-file/lib/tests/l1/structural_validation.rs
    - claudine/cli/tests/l1/snapshots/l1__context_command__context_expressions_function_order_snapshot.snap
    - claudine/docs/research/agent-cli/_research.md
    - claudine/docs/research/non-interactive-sessions/_research.md
    - claudine/docs/research/reasoning-level/_research.md
    - claudine/docs/research/steering/_research.md
    - claudine/lib/src/composition/error/lifecycle_authoring.rs
    - claudine/lib/src/composition/error/mod.rs
    - claudine/lib/src/composition/error/render/lifecycle.rs
    - claudine/lib/src/composition/error/render/mod.rs
    - claudine/lib/src/composition/lifecycle/bindings.rs
    - claudine/lib/src/composition/lifecycle/bindings/rename.rs
    - claudine/lib/src/composition/lifecycle/bindings/rename/tests.rs
    - claudine/lib/src/composition/lifecycle/bindings/tests.rs
    - claudine/lib/src/composition/lifecycle/mod.rs
    - claudine/lib/src/composition/lifecycle/registry.rs
    - claudine/lib/src/composition/lifecycle/tests/call_check.rs
    - claudine/lib/src/composition/lifecycle/tests/mod.rs
    - claudine/lib/src/composition/looping/config.rs
    - claudine/lib/src/composition/looping/config/tests.rs
    - claudine/lib/src/composition/prepare.rs
    - claudine/lib/src/composition/reserved.rs
    - claudine/lib/src/composition/sequence/preflight/mod.rs
    - claudine/lib/src/composition/sequence/preflight/shape.rs
    - claudine/lib/src/composition/sequence/preflight/tests.rs
    - claudine/lib/tests/l1/host_catalog.rs
    - claudine/lib/tests/l1/type_scope_ownership_guard.rs
    - claudine/schemas/action.yaml
    - claudine/schemas/lifecycle.yaml
    - darkmatter/cli/src/args/cli.rs
    - darkmatter/cli/src/args/command.rs
    - darkmatter/cli/src/args/enums.rs
    - darkmatter/cli/src/args/mod.rs
    - darkmatter/cli/src/args/policy.rs
    - darkmatter/cli/src/args/target.rs
    - darkmatter/cli/src/artifact.rs
    - darkmatter/cli/src/commands/clean.rs
    - darkmatter/cli/src/commands/compose.rs
    - darkmatter/cli/src/commands/compose/lifecycle.rs
    - darkmatter/cli/src/commands/compose/report.rs
    - darkmatter/cli/src/commands/mod.rs
    - darkmatter/cli/src/commands/render.rs
    - darkmatter/cli/src/commands/schema/validate.rs
    - darkmatter/cli/src/main.rs
    - darkmatter/cli/src/render.rs
    - darkmatter/cli/tests/l1/claudine_schema_bundle.rs
    - darkmatter/cli/tests/l1/clean_schema.rs
    - darkmatter/cli/tests/l1/compose_base_schema.rs
    - darkmatter/cli/tests/l1/compose_call_warnings.rs
    - darkmatter/cli/tests/l1/compose_dasherized_identifiers.rs
    - darkmatter/cli/tests/l1/compose_file_predicates.rs
    - darkmatter/cli/tests/l1/compose_icmp.rs
    - darkmatter/cli/tests/l1/compose_lifecycle.rs
    - darkmatter/cli/tests/l1/compose_refs_and_missing.rs
    - darkmatter/cli/tests/l1/compose_remote_caching.rs
    - darkmatter/cli/tests/l1/compose_report_json.rs
    - darkmatter/cli/tests/l1/compose_schema.rs
    - darkmatter/cli/tests/l1/compose_shell.rs
    - darkmatter/cli/tests/l1/compose_shell_template.rs
    - darkmatter/cli/tests/l1/compose_state_set.rs
    - darkmatter/cli/tests/l1/layout_fill.rs
    - darkmatter/cli/tests/l1/layout_style_frontmatter.rs
    - darkmatter/cli/tests/l1/literal_glob_hint.rs
    - darkmatter/cli/tests/l1/main.rs
    - darkmatter/cli/tests/l1/policy_verbs.rs
    - darkmatter/cli/tests/l1/schema_robustness_matrix.rs
    - darkmatter/cli/tests/l1/schema_trigger_rename.rs
    - darkmatter/cli/tests/l1/schema_triggers.rs
    - darkmatter/cli/tests/l1/schema_validate.rs
    - darkmatter/cli/tests/l1/schema_validate_baseline_selection.rs
    - darkmatter/cli/tests/l1/schema_validate_shipped_artifacts.rs
    - darkmatter/cli/tests/l1/schema_validate_timings.rs
    - darkmatter/cli/tests/l1/standalone_baseline.rs
    - darkmatter/dmls/src/diagnostics/call_check.rs
    - darkmatter/dmls/src/diagnostics/codes.rs
    - darkmatter/dmls/src/diagnostics/frontmatter.rs
    - darkmatter/dmls/src/overlay/event_views.rs
    - darkmatter/dmls/src/providers/dsl.rs
    - darkmatter/dmls/tests/l1/expression_call_diagnostics.rs
    - darkmatter/dmls/tests/l1/expression_scopes.rs
    - darkmatter/dmls/tests/l1/host_function_diagnostics.rs
    - darkmatter/dmls/tests/l1/lsp_session.rs
    - darkmatter/dmls/tests/l1/main.rs
    - darkmatter/dmls/tests/l1/type_scope_ownership_guard.rs
    - darkmatter/lib/benches/schema_validation.rs
    - darkmatter/lib/src/effects/verbs.rs
    - darkmatter/lib/src/lifecycle/actions.rs
    - darkmatter/lib/src/lifecycle/error.rs
    - darkmatter/lib/src/lifecycle/mod.rs
    - darkmatter/lib/src/lifecycle/registry.rs
    - darkmatter/lib/src/lifecycle/shell_template.rs
    - darkmatter/lib/src/lifecycle/source_map.rs
    - darkmatter/lib/src/lifecycle/validate.rs
    - darkmatter/lib/src/markdown/block/hr_parser.rs
    - darkmatter/lib/src/markdown/compose/call_precheck.rs
    - darkmatter/lib/src/markdown/compose/context/options.rs
    - darkmatter/lib/src/markdown/compose/context/report.rs
    - darkmatter/lib/src/markdown/compose/directive_targets.rs
    - darkmatter/lib/src/markdown/compose/expression/absence.rs
    - darkmatter/lib/src/markdown/compose/expression/call_binding/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/call_binding/registration.rs
    - darkmatter/lib/src/markdown/compose/expression/call_binding/signature.rs
    - darkmatter/lib/src/markdown/compose/expression/call_check.rs
    - darkmatter/lib/src/markdown/compose/expression/diagnostics.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/fs.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/fs_tests.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/network.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/paths.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/predicates.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/test_hooks.rs
    - darkmatter/lib/src/markdown/compose/expression/host/artifact.rs
    - darkmatter/lib/src/markdown/compose/expression/host/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/resolve_ctx.rs
    - darkmatter/lib/src/markdown/compose/expression/scope/facts.rs
    - darkmatter/lib/src/markdown/compose/expression/scope/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/scope/projection.rs
    - darkmatter/lib/src/markdown/compose/expression/scope/raw.rs
    - darkmatter/lib/src/markdown/compose/expression/scope/shape.rs
    - darkmatter/lib/src/markdown/compose/expression/scope/tests.rs
    - darkmatter/lib/src/markdown/compose/frontmatter_interpolation.rs
    - darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion/suffix.rs
    - darkmatter/lib/src/markdown/compose/icmp.rs
    - darkmatter/lib/src/markdown/compose/mod.rs
    - darkmatter/lib/src/markdown/compose/pipeline/mod.rs
    - darkmatter/lib/src/markdown/compose/pipeline/phases.rs
    - darkmatter/lib/src/markdown/compose/policy.rs
    - darkmatter/lib/src/markdown/compose/preflight/collect.rs
    - darkmatter/lib/src/markdown/compose/remote.rs
    - darkmatter/lib/src/markdown/compose/report_diagnostics.rs
    - darkmatter/lib/src/markdown/compose/schema_validation.rs
    - darkmatter/lib/src/markdown/compose/shell_blocks/types.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/mod.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/site.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/template/typing.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/types.rs
    - darkmatter/lib/src/markdown/compose/transclusion/engine.rs
    - darkmatter/lib/src/markdown/errors/blocks.rs
    - darkmatter/lib/src/markdown/mod.rs
    - darkmatter/lib/src/markdown/schemas/advisory.rs
    - darkmatter/lib/src/markdown/schemas/catalog.rs
    - darkmatter/lib/src/markdown/schemas/clean.rs
    - darkmatter/lib/src/markdown/schemas/mod.rs
    - darkmatter/lib/src/markdown/types.rs
    - darkmatter/lib/src/style/parse.rs
    - darkmatter/lib/tests/fixtures/host_functions/project.facts.json
    - darkmatter/lib/tests/fixtures/host_functions/project.yaml
    - darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/darkmatter-schemas-darkmatter-schema-body.digest
    - darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/darkmatter-schemas-partials-fm.digest
    - darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/effective-baseline-inline.digest
    - darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/effective-baseline-root-union.digest
    - darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/effective-baseline-schema-file.digest
    - darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/embedded-darkmatter-base.digest
    - darkmatter/lib/tests/l1/blocking_policy.rs
    - darkmatter/lib/tests/l1/clean_counters.rs
    - darkmatter/lib/tests/l1/design_time_call_check.rs
    - darkmatter/lib/tests/l1/directive_target_analysis.rs
    - darkmatter/lib/tests/l1/expression_type_scope.rs
    - darkmatter/lib/tests/l1/file_predicates.rs
    - darkmatter/lib/tests/l1/flow_sensitive_narrowing.rs
    - darkmatter/lib/tests/l1/function_binding_registry.rs
    - darkmatter/lib/tests/l1/host_functions.rs
    - darkmatter/lib/tests/l1/lifecycle_schema_generation.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/report_diagnostics.rs
    - darkmatter/lib/tests/l1/shared_diagnostics.rs
    - darkmatter/lib/tests/l1/type_scope_ownership_guard.rs
    - darkmatter/lib/tests/l1/unknown_identifier_warning.rs
    - darkmatter/lib/tests/l1/variable_type_projection.rs
    - darkmatter/schemas/partials/functions.yaml
    - darkmatter/schemas/partials/lifecycle.yaml
    - darkmatter/schemas/partials/md-lifecycle.yaml
documentation:
    - .claude/skills/biscuit-file/SKILL.md
    - .claude/skills/claudine/SKILL.md
    - .claude/skills/claudine/research-contracts.md
    - .claude/skills/darkmatter/SKILL.md
    - .claude/skills/darkmatter/compose.md
    - .claude/skills/darkmatter/dmls.md
    - .claude/skills/darkmatter/expressions.md
    - .claude/skills/darkmatter/frontmatter.md
    - .claude/skills/darkmatter/remote-cache.md
    - .claude/skills/darkmatter/schema.md
    - biscuit-file/docs/topics/file-references.md
    - claudine/docs/topics/schemas.md
    - content-policy/docs/topics/policy-lifecycle.md
    - darkmatter/README.md
    - darkmatter/cli/README.md
    - darkmatter/dmls/docs/diagnostics.md
    - darkmatter/docs/cli/clean.md
    - darkmatter/docs/cli/compose.md
    - darkmatter/docs/cli/index.md
    - darkmatter/docs/cli/policy.md
    - darkmatter/docs/cli/report-json.md
    - darkmatter/docs/inline/call-check.md
    - darkmatter/docs/inline/fm-shell-expansion.md
    - darkmatter/docs/inline/interpolation.md
    - darkmatter/docs/inline/preflight-checks.md
    - darkmatter/docs/inline/schema-validation.md
    - darkmatter/docs/inline/shell-expansion.md
    - darkmatter/docs/lsp/configuration.md
    - darkmatter/docs/lsp/features.md
    - darkmatter/docs/rendering/disclosure.md
    - darkmatter/docs/rendering/hr.md
    - darkmatter/docs/rendering/style.md
    - darkmatter/docs/topics/caching.md
    - darkmatter/docs/topics/compose-requests.md
    - darkmatter/docs/topics/darkmatter-expressions.md
    - darkmatter/docs/topics/host-functions.md
    - darkmatter/docs/topics/layouts-and-style.md
    - darkmatter/docs/topics/markdown-formatting.md
    - darkmatter/docs/topics/remote-url-references.md
    - darkmatter/docs/topics/schemas/definition.md
    - darkmatter/docs/topics/schemas/parsing/grammar.md
    - darkmatter/docs/topics/state-management/context-variables.md
    - darkmatter/docs/topics/transclusion.md
    - darkmatter/docs/transclusion/block-transclusion.md
    - prompts/_add/add-expressions.md
    - renderable/README.md
    - renderable/docs/layout-and-style.md
completed_phase: 9
implemented: true
pipeline_refreshed:
    - 2026-11-01-map-and-filter
decisions_for_author: []
---

# Implementation Log for 2026-10-31-type-aware-evaluation (9 phases)

## Phase 1

Phase 1 changes no product code. It records the dependency audit, the surface
and legacy-flag inventories, the criterion traceability table, the Spike S1
classification, and the "before" performance baseline. All findings were taken
from the worktree at `817792c35` (2026-10-24) on macOS.

### Dependency audit

**Verdict: passed.** Every seam the plan names from
`2026-10-28-schema-enhancements`, `2026-10-27-darkmatter-lifecycle-ownership`,
and `2026-10-29-recursive-schema-types` exists. Rule 2's stop condition does
not apply. `L` = `darkmatter/lib/src/markdown`.

| Seam | Status | Location | Signature / notes |
|---|---|---|---|
| Catalog `returns` | yes | `darkmatter/schemas/partials/functions.yaml` (`kind: schema`, `categories:`) | e.g. `is_array`: `function( parameters(x: unknown \| null); returns(boolean); category(Type Predicates); …)` |
| Catalog loader | yes | `L/compose/expression/catalog/mod.rs:221`, `:256` | `pub fn expression_function_catalog() -> &'static FunctionCatalog` (`LazyLock` over `include_str!`); entry field `pub returns: &'static str`; strict loader `pub(crate) fn load_catalog(` |
| `ArgumentCategory` | yes | `L/compose/expression/call_check.rs:55` | `pub enum ArgumentCategory { Compatible, NullableMatch, Uncertain, Unknown, Incompatible }`, numbered 1–5 by `number()` |
| `ArgumentType` / `::of` | yes | `call_check.rs:106`, `:122` | `pub enum ArgumentType { Literal(Value), Typed(ScopeShape), Unknown }`; `pub fn of(arg: &SpannedExpr, scope: &TypeScope<'_>) -> Self`. **Calls and operators type `Unknown` today** (only variables and constant paths are subjects). |
| `classify_argument` | yes | `call_check.rs:176`, `:182` | `pub fn classify_argument(source: &ArgumentType, parameter: &CompiledParameter) -> ArgumentCategory`; `classify_argument_with_definitions(…)` |
| `check_call` / `CallCheck` | yes | `call_check.rs:708`, `:719`, `:580` | `pub fn check_call(name, arguments: &[ArgumentType], span, argument_spans) -> Option<CallCheck>` delegates to `check_call_with_definitions(…, definitions: &DefinitionTable)` with an empty table |
| `check_expression_calls` / `check_prepared_calls` | yes | `call_check.rs:945`, `:978` | `pub fn check_expression_calls(expr: &SpannedExpr, site: &TypeScope<'_>) -> Vec<CallCheck>`; `pub fn check_prepared_calls(prepared: &PreparedValue, site: &TypeScope<'_>) -> Vec<PreparedCallCheck>` |
| Passive literal classification | yes | `call_check.rs:755–767`, `:839`; `call_binding/bind.rs:211` | all-literal calls go through `registration.bind_call(values)` |
| Composition rejection | yes | `L/compose/call_precheck.rs:56`, `:232`; `L/types.rs:457`; `L/compose/expression/error.rs:379` | `check_document(…) -> Result<(), PrecheckFailure>`; `MarkdownError::rejected_call()`; `ExpressionError::CallCheck(Box<CallCheck>)` |
| `[expressions] strict` | yes | `darkmatter/dmls/src/config/mod.rs:184`, `:285` | `pub struct ExpressionsConfig { pub strict: bool }` on `DmlsConfig.expressions` |
| DMLS call codes | yes | `darkmatter/dmls/src/diagnostics/codes.rs:164`, `:167` | `EXPRESSION_INCOMPATIBLE_CALL = "dm.expression.incompatible_call"`, `EXPRESSION_UNCERTAIN_CALL = "dm.expression.uncertain_call"` (DMLS-owned today; Phase 6 moves them) |
| DMLS call adapter | yes | `darkmatter/dmls/src/diagnostics/call_check.rs:36`, `:50`, `:59` | `severity(category, strict)`, `declared_types(ctx)`, `call_diagnostics(expr, site, strict)` |
| Projection | yes | `L/compose/expression/scope/projection.rs:153–192`, `:304` | `pub fn top_level_types(&self) -> BTreeMap<String, Shape>`; `new(Option<&SimplifiedSchema>)`; `with_definitions(&'s DefinitionTable)` (**a table, not a lookup**); `definitions()`; `with_static_frontmatter`; `with_host_globals(&[&str])`; `with_binding_view(&BindingView)` |
| `with_definitions` callers | yes | — | `L/compose/call_precheck.rs:64`; `darkmatter/dmls/src/diagnostics/call_check.rs:54`; `darkmatter/dmls/src/providers/dsl.rs:419`; `L/compose/directive_targets.rs:105`; `L/compose/shell_expansion/site.rs:516`; `L/compose/preflight/collect.rs:1378`; `claudine/lib/src/composition/sequence/preflight/mod.rs:1609`, `:2013`; `darkmatter/lib/src/lifecycle/shell_template.rs:195` |
| `lifecycle_declared_types` | yes (moved) | `darkmatter/lib/src/lifecycle/shell_template.rs:188` | `pub fn lifecycle_declared_types<'a>(schema: Option<&'a EffectiveSchema>, registry: &'a EventRegistry) -> DeclaredTypes<'a>`; Claudine re-exports it (`claudine/lib/src/composition/lifecycle/mod.rs:49`) |
| Registration facts | yes | `L/compose/expression/call_binding/registration.rs:14`, `:67`, `:136`, `:141` | `EvaluationMode { Eager, Lazy }`; builder `reject_sources(parameter, &[ValueKind])`, `numeric_recovery(subject, fallback)`; `FunctionRegistration { name, overloads, evaluation, recovery }` |
| Narrowing-fact registration slot | **different** | `L/compose/expression/scope/facts.rs:20`, `:34`, `:44` | No fact field on `FunctionRegistration`. Built-in facts are the private static table `PREDICATE_FACTS: &[(&str, PredicateFact)]` (`Family`, `Null`, `Empty`, `FileExists`) applied by `assume(..)`. See decision D1. |
| `BindingView` | yes | `L/compose/expression/binding.rs:247` | `builder(scope)`, `baseline()`, `global(root)`, `globals()`, `classify_root(root)` |
| Global catalogs | yes | `L/schemas/catalog.rs:185–265`; `L/schemas/embedded.rs:165` | `GlobalCatalog::darkmatter()`, `from_documents`, `from_file`, `from_host_file(path, &FileResolutionContext)` |
| Claudine host globals | yes | `claudine/lib/src/composition/schema/globals.rs:23`, `:37`, `:52` | `HOST_CATALOG_TEXT`, `claudine_globals()`, `host_global(name)`; DMLS `validation_view(event, catalogs)` (`darkmatter/dmls/src/overlay/event_views.rs:63`) |
| `TypeExpr::Ref(DefKey)` | yes | `L/schemas/simplified/types.rs:271` | — |
| `EffectiveSchema::definitions()` / `definition(&DefKey)` | yes | `L/schemas/mod.rs:1026`, `:1037` | `definitions()` returns `&Arc<DefinitionTable>` (the document table only); `definition(&DefKey) -> Option<&Definition>` searches every layer |
| `DefinitionLookup` | yes | `L/schemas/simplified/query.rs:346` | `fn lookup_definition(&self, key: &DefKey) -> Option<&Definition>`, implemented for `DefinitionTable` |
| Path / subtype helpers | yes | `L/schemas/frontmatter_shape.rs:116`; `L/schemas/simplified/subtype.rs:98` | `property_def_at_path_with_definitions(root, frontmatter, path, &DefinitionTable)`; `is_subtype_with_definitions(a, b, &DefinitionTable)` |
| `Shape::Ref` in call check | yes | `call_check.rs:350`, `:395`, `:478` | an unresolved target gives `Fit::Unknown` |
| `AbsenceScope` name list | yes | `L/compose/expression/absence.rs:50`, `:84`; `functions/mod.rs:674`; `functions/predicates.rs:16` | `ABSENCE_PREDICATES` = `is_null`/`isnull`, `is_empty`/`isempty` (the list Phase 6 removes) |
| `dm.expression.undeclared_property` | yes | `darkmatter/dmls/src/diagnostics/codes.rs:151` | warning |
| `file_exists_fn` / `fileexists` | yes | `L/compose/expression/functions/fs.rs:162`; `functions/paths.rs:6`; `resolve_ctx.rs:330`, `:342` | catalog `parameters(value: unknown \| null); returns(boolean); fallible;` |
| Host function registration | **absent** | `L/compose/expression/functions/mod.rs:150–172`, `:300`; `functions/test_hooks.rs:77` | The registry is a static `LazyLock` from the embedded catalog plus `BINDING_GROUPS`. The only runtime hook is test/feature-gated `test_hooks::register`. `load_catalog` is `pub(crate)`. Claudine registers no expression function today. See decision D1. |

### Surface inventory: consumers of expression analysis

Marks: **move** = a read-only scan that must move onto a Darkmatter library
API; **library** = already the shared library (a Phase 2–6 target);
**n/a** = not variable visibility or typing.

| File | Role | Mark |
|---|---|---|
| `L/compose/expression/absence.rs` | `AbsenceScope`, `static_variable_reads`, `branch_scope`; the catalog-name rule `call_argument → is_absence_predicate` | library (Phase 6 replaces the rule) |
| `L/compose/expression/functions/predicates.rs:16` | `ABSENCE_PREDICATES` name list | library (removed in Phase 6) |
| `L/compose/expression/binding.rs` | binding model, reserved namespaces | library |
| `L/compose/expression/semantics.rs` | operator/truthiness descriptor catalogs (no AST walk) | n/a |
| `L/compose/expression/lint.rs` | nested `{{…}}` in literals (syntax) | n/a |
| `L/compose/expression/prepared.rs` | `prepare_value`, `validate_prepared`, `validate_expression` | library |
| `L/compose/expression/path_projection.rs` | filesystem path projection for `relative`/`dirname` | n/a |
| `L/compose/expression/call_check.rs` | compatibility API; `ArgumentType::of` types calls/operators `unknown` | library (Phase 2) |
| `L/compose/expression/scope/{projection,mod,facts,shape}.rs` | `DeclaredTypes`, `TypeScope`, guard algebra | library (Phases 2, 3) |
| `L/compose/call_precheck.rs` | composition walk; category 5 only | library |
| `L/compose/interpolation/evaluator.rs` | runtime evaluator | n/a |
| `L/compose/directive_targets.rs` | target nullability via `TypeScope` (own `DeclaredTypes` build) | library |
| `L/compose/unknown_identifiers.rs` | absence candidates → advisories | library |
| `darkmatter/lib/src/lifecycle/validate.rs` | `walk_stack`, `walk_action`, `walk_with_value`, `event_surfaces` (successor of Claudine's deleted `validate.rs`) | library |
| `darkmatter/dmls/src/diagnostics/call_check.rs` | thin severity adapter | library adapter |
| `darkmatter/dmls/src/diagnostics/frontmatter.rs` | frontmatter expression diagnostics through the library walk | library adapter |
| `darkmatter/dmls/src/overlay/expressions.rs:1357` | `undeclared_property_findings` via `static_variable_reads`; local dash-key walk for a quick fix only | library adapter (dash walk n/a) |
| `darkmatter/dmls/src/providers/dsl.rs:406` | `with_site_scope`: own `DeclaredTypes` + `BodyGuards` | library adapter |
| `darkmatter/dmls/src/config/mod.rs` | `ExpressionsConfig.strict`; no `file_exists` setting | n/a |
| `claudine/lib/src/composition/looping/config.rs:393–498` | `extract_control_variables`, `collect_value_template_identifiers`, `collect_identifiers` (local `Expr::*` walk; lifts `fm.<head>`; local reserved-root test `is_reserved_identifier`) | **move** |
| `claudine/lib/src/composition/lifecycle/binding_checks.rs:42` | `validate_no_err_in_no_error_events` (library `prepare_value`/`validate_prepared`) | library adapter |
| `claudine/lib/src/composition/lifecycle/binding_checks.rs:95` + `bindings.rs:423–692` | `find_removed_loop_read`, `removed_loop_read*`, `reject_removed_loop_reads_*` (local `Expr` walk for retired loop names) | **move** |
| `claudine/lib/src/composition/lifecycle/bindings/rename.rs:23` | `read`/`static_path` (local walk for retired sequence globals) | **move** |
| `claudine/lib/src/composition/sequence/preflight/shape.rs:510`, `:522` | `first_target_identity_root`, `target_identity_path_in_expr` (local walk for agent/model late roots) | **move** |
| `claudine/lib/src/composition/lifecycle/bindings.rs:456` | `approval_diagnostics` (library reads filtered by late namespaces) | library adapter |
| `claudine/lib/src/composition/lifecycle/prewarm/classify.rs:114–262` | readiness classification; roots from `static_variable_reads`, local `contains_call` | n/a (decision D3) |
| `claudine/lib/src/composition/dispatch/matcher.rs:123` | condition-vs-regex heuristic | n/a |
| `claudine/lib/src/composition/sequence/preflight/containers.rs:27`, `sequence/task/mod.rs:1376–1410` | shell literal rewriting | n/a |

Places that build their own `DeclaredTypes` (they all switch to the
all-layer definition lookup in Phase 2): `L/compose/call_precheck.rs:62`,
`L/compose/directive_targets.rs:98`, `darkmatter/lib/src/lifecycle/shell_template.rs:188`,
`L/compose/shell_expansion/site.rs:515`, `L/compose/preflight/collect.rs:1373`,
`L/compose/shell_expansion/template/typing.rs:298` (`project_hole`: bare
top-level properties only, everything else `string`),
`L/compose/shell_expansion/template/grammar.rs:176` (`DeclaredTypes::new(None)`),
`L/compose/expression/absence.rs:43` (`UNTYPED`),
`darkmatter/dmls/src/diagnostics/call_check.rs:50`, `darkmatter/dmls/src/providers/dsl.rs:417`, `:489`,
`darkmatter/dmls/src/providers/frontmatter.rs:3120`,
`claudine/lib/src/composition/sequence/preflight/mod.rs:1606`, `:2010`.

**Criterion 8 gap.** No lifecycle action expression is call-checked today:
only `call_precheck.rs` and DMLS call `check_expression_calls`. See decision D2.

### Legacy flag inventory

| Flag | Occurrences | Files (all / live) | clap definition | Accepted by |
|---|---|---|---|---|
| `--allow-missing-hyperlinks` | 6 | 4 / 1 | `darkmatter/cli/src/args/command.rs:154` | compose |
| `--allow-missing-image-refs` | 4 | 4 / 1 | `command.rs:158` | compose |
| `--allow-missing-transclusions` | 8 | 4 / 1 | `command.rs:162` | compose |
| `--allow-any-missing-reference` | 9 | 5 / 1 | `command.rs:166` | compose |
| `--allow-ctx-override` | 23 | 9 / 4 | `command.rs:170` | compose |
| `--allow-invalid-frontmatter-assignment` | 19 | 9 / 3 | `command.rs:174` | compose |
| `--allow-reassigned-frontmatter-property` | 13 | 7 / 3 | `command.rs:178` | compose |
| `--allow-shell-timeout` | 41 | 24 / 13 | `command.rs:198` | compose |
| `--allow-host` | 100 | 42 / 22 | `command.rs:211` (`Vec<String>`) | compose |
| `--no-baseline-schema` | 115 | 54 / 23 | `command.rs:98` (clean), `:186` (compose), `args/target.rs:79` (schema validate) | clean, compose, schema validate |
| `--no-schema-triggers` | 67 | 31 / 24 | `command.rs:102`, `:190`, `target.rs:91` | clean, compose, schema validate |
| `--no-trigger-schemas` | 72 | 44 / 4 | not a clap arg; `RENAMED_FLAGS` in `darkmatter/cli/src/main.rs:136` | none (usage error already) |
| `--strict-style` | 89 | 34 / 12 | `darkmatter/cli/src/args/cli.rs:233` (top-level `Cli`) | default `md FILE` (`darkmatter/cli/src/render.rs:134`) |

"Live" excludes `*/features/**`, `*/fixes/**`, `*/reviews/**`, and
`docs/research/**`. Baseline conflicts: `command.rs:93` (clean), `:181`
(compose), `target.rs:73` (schema validate, on `--schema`). Explicit
`md render` (`command.rs:16–38`) declares none of these flags.

Live files by kind:

- **CLI definitions and commands:** `darkmatter/cli/src/args/{command,cli,target}.rs`,
  `darkmatter/cli/src/commands/{compose,clean}.rs`, `commands/schema/validate.rs`,
  `darkmatter/cli/src/main.rs`, `darkmatter/cli/src/render.rs`.
- **Completions, justfiles, scripts, examples:** none. Completions are
  generated by clap; `darkmatter/cli/tests/l1/schema_trigger_rename.rs::completions_offer_only_the_new_flag`
  is the only completion test.
- **Library messages/doc strings:** `L/types.rs`, `L/compose/shell_expansion/types.rs`,
  `L/compose/shell_blocks/types.rs`, `L/compose/frontmatter_shell_expansion/suffix.rs`,
  `L/compose/context/options.rs`, `L/compose/remote.rs`, `L/compose/icmp.rs`,
  `L/compose/expression/functions/network.rs`, `L/compose/pipeline/mod.rs`,
  `L/schemas/clean.rs`, `L/block/hr_parser.rs`, `L/style/parse.rs`,
  `darkmatter/lib/benches/schema_validation.rs`.
- **Schemas/fixtures:** `darkmatter/schemas/partials/functions.yaml`,
  `darkmatter/lib/tests/fixtures/recursive_schema_frozen/{action,lifecycle,lifecycle-recursive}.yaml`.
- **Docs:** `darkmatter/cli/README.md`; `darkmatter/docs/cli/{compose,clean}.md`;
  `darkmatter/docs/inline/{fm-shell-expansion,preflight-checks,schema-validation,shell-expansion}.md`;
  `darkmatter/docs/lsp/features.md`; `darkmatter/docs/rendering/{disclosure,hr,style}.md`;
  `darkmatter/docs/topics/{caching,darkmatter-expressions,layouts-and-style,markdown-formatting,remote-url-references,transclusion}.md`;
  `darkmatter/docs/topics/schemas/definition.md`;
  `darkmatter/docs/topics/state-management/context-variables.md`;
  `darkmatter/docs/transclusion/block-transclusion.md`.
- **Tests:** `darkmatter/cli/tests/l1/{claudine_schema_bundle,clean_schema,compose_base_schema,compose_dasherized_identifiers,compose_icmp,compose_refs_and_missing,compose_remote_caching,compose_schema,compose_shell_template,compose_shell,compose_state_set,layout_style_frontmatter,literal_glob_hint,schema_robustness_matrix,schema_trigger_rename,schema_triggers,schema_validate_baseline_selection,schema_validate_shipped_artifacts,schema_validate_timings,schema_validate,standalone_baseline}.rs`;
  `darkmatter/lib/tests/l1/{clean_counters,trigger_spelling_guard}.rs`.
- **Other packages and skills:** `.claude/skills/darkmatter/{SKILL,compose,frontmatter,remote-cache,schema}.md`,
  `.claude/skills/claudine/research-contracts.md`,
  `content-policy/docs/topics/policy-lifecycle.md`, `renderable/README.md`,
  `renderable/docs/layout-and-style.md`, `prompts/_add/add-expressions.md`
  (internal prompt folder).
- **Historical (not changed):** about 125 files under feature/fix/review
  snapshots and `docs/research/`.

### Traceability: acceptance criteria → tests

Abbreviations: `LT` = `darkmatter/lib/tests/l1/`, `DT` = `darkmatter/dmls/tests/l1/`,
`CT` = `darkmatter/cli/tests/l1/`, `CL` = `claudine/lib/src/composition/`.
Later phases add the new test names to the last column.

| # | Existing coverage today | Required addition (phase) | Added test(s) |
|---|---|---|---|
| 1 | `LT/design_time_call_check.rs::a_mixed_root_union_projects_nothing` (Phase B behavior; changes) | root-union fold, order independent (P2) | `LT/variable_type_projection.rs::root_unions::the_motivating_table_folds_every_arm`, `::root_unions::the_fold_is_independent_of_arm_and_key_order`, `::root_unions::arms_that_differ_fold_to_a_union_with_each_arms_optionality` |
| 2 | `LT/expression_type_scope.rs::type_predicates_narrow_a_declared_union_on_both_sides` (partial) | every normalization row (P2) | `LT/variable_type_projection.rs::normalization::every_row_of_the_table`, `::normalization::presence_markers_do_not_split_a_member`, `::normalization::a_property_level_union_normalizes_like_a_fold` |
| 3 | `LT/unknown_identifier_warning.rs::a_baseline_schema_property_is_known`, `::a_matched_trigger_schema_property_is_known` (presence only) | all-layer + raw JSON Schema projection (P2) | `LT/variable_type_projection.rs::layers::every_layer_of_the_effective_schema_projects`, `::layers::a_raw_json_schema_exposes_its_enumerable_properties`, `::layers::a_raw_root_union_folds_its_arms`, `::layers::a_referenced_raw_schema_types_its_properties` |
| 4 | `LT/expression_type_scope.rs::constant_paths_and_indices_are_subjects_and_computed_ones_are_not` (subjects only) | `meta.count` → `number \| null` (P2) | `LT/variable_type_projection.rs::nested_paths::a_member_of_an_optional_object_may_be_null` |
| 5 | `LT/global_catalog.rs::err_declares_exactly_the_runtime_fields`, `::timing_declares_the_runtime_fields_as_optional_integers`, `::ctx_and_current_share_one_shape_projected_by_the_context_descriptors`; `DT/host_globals.rs::host_global_hover_comes_from_the_host_catalog_file` | reserved/host global types in `DeclaredTypes`; unavailable → no type (P2) | `LT/variable_type_projection.rs::globals::reserved_roots_take_their_catalog_shapes`, `::globals::an_unavailable_global_has_no_type`, `::globals::a_host_global_takes_its_host_catalog_type`; `CL/lifecycle/tests/call_check.rs::claudine_globals_take_their_host_catalog_types` |
| 6 | `LT/design_time_call_check.rs::the_worked_example_rows`; `DT/expression_call_diagnostics.rs::rows_a_to_e_by_default` | rows f, i category 1 (P2) | `LT/variable_type_projection.rs::call_typing::worked_example_rows_f_and_i_are_exact`, `::call_typing::operators_and_calls_are_typed_from_their_operands`, `::call_typing::an_overloaded_call_unions_its_candidate_returns`; `LT/design_time_call_check.rs::categories::the_worked_example_rows` (rows f, i now category 1); `DT/expression_call_diagnostics.rs::settings::rows_f_and_i_are_silent_under_both_settings` |
| 7 | `LT/design_time_call_check.rs::projection_is_deterministic_and_reads_only_the_schema`; `LT/absent_property_contract.rs::*` | instrumented passivity + unchanged state (P2) | `LT/variable_type_projection.rs::passivity::projection_reads_no_file_and_runs_no_handler`, `::passivity::null_materialization_is_unchanged` |
| 8 | none (lifecycle actions are not call-checked) | lifecycle `min(flag, 5)` rejection parity (P2, decision D2) | `LT/variable_type_projection.rs::lifecycle_calls::a_lifecycle_call_is_rejected_exactly_like_the_body_call`, `::lifecycle_calls::a_compatible_or_untyped_lifecycle_call_is_not_rejected`; `CT/compose_lifecycle.rs::a_provably_wrong_lifecycle_call_rejects_before_any_event`; `CL/lifecycle/tests/call_check.rs::a_lifecycle_call_is_rejected_like_the_body_call` |
| 9 | `LT/design_time_call_check.rs::guards_narrow_the_argument`; `DT/expression_call_diagnostics.rs::guards_narrow_rows_g_and_h`; `DT/expression_scopes.rs::expression_branch_rows_agree_with_the_library` | composition assertion on extended scope (P3) | `LT/flow_sensitive_narrowing.rs::body_guards::composition_reads_the_same_narrowing`, `::registered_facts::a_true_implies_fact_narrows_only_its_true_side`; `DT/expression_call_diagnostics.rs::narrowing::the_editor_shows_exactly_the_library_categories_at_each_guarded_site` |
| 10 | `LT/expression_type_scope.rs::a_conjunction_proves_each_operand_on_its_true_side_only`, `::a_ternary_guard_joins_its_branch_routes`; `DT/expression_scopes.rs::hover_inside_a_body_block_guard_shows_the_library_type` | category 1/2 for `x ?`, `x &&`, `::block when` (P3) | `LT/flow_sensitive_narrowing.rs::compound_guards::short_circuit_operands_and_ternaries_see_only_justified_facts`, `::body_guards::a_block_guard_narrows_its_body`; `DT/expression_call_diagnostics.rs::narrowing::the_editor_shows_exactly_the_library_categories_at_each_guarded_site` |
| 11 | `LT/expression_type_scope.rs::type_predicates_narrow_a_declared_union_on_both_sides`; `DT/type_scope_ownership_guard.rs::*` | true-iff vs true-implies registry; `is_file_type`, host facts, shell hole (P3) | `LT/flow_sensitive_narrowing.rs::registered_facts::a_true_implies_fact_narrows_only_its_true_side`, `::registered_facts::a_registered_fact_narrows_a_shell_template_hole`, `::registered_facts::true_implies_and_false_implies_facts_use_their_type`, `::registered_facts::a_predicate_without_a_fact_narrows_nothing`, `::registered_facts::registration_rejects_invalid_facts`, `::recursive_arms::a_recursive_arm_is_classified_by_its_definition`; `LT/file_predicates.rs::guarded_file_arguments::an_is_file_type_guard_narrows_a_shell_template_hole` and `::an_is_file_type_guard_makes_the_file_argument_category_1` (`is_file_type` leg, P4); host-fact DMLS leg P5 (decision P3-D7) |
| 12 | `LT/expression_type_scope.rs::a_child_scope_shares_its_parent_and_never_changes_it` | before/after state fixture (P3) | `LT/flow_sensitive_narrowing.rs::body_guards::narrowing_never_changes_a_stored_value` |
| 13 | none (`is_file_type` absent); biscuit-file `parse.rs` unit tests `empty_string_rejected`, `https_url_detected_as_url_kind` | `is_file_type` corpus with failing stubs (P4) | `biscuit-file/lib/tests/l1/structural_validation.rs::every_supported_kind_is_well_formed_on_every_host`, `::malformed_references_are_rejected_by_construction`; `LT/file_predicates.rs::is_file_type::it_is_true_exactly_for_well_formed_reference_text_without_a_context` |
| 14 | `LT/path_function_contracts.rs::file_exists_inspects_its_argument_without_converting_it`; `LT/context_function_contracts.rs::file_exists_inspects_its_argument_without_converting_it` | `""`/`https://` false without I/O; `file://` corpus (P4) | `LT/file_predicates.rs::file_exists::malformed_text_is_false_and_well_formed_references_keep_their_outcomes`, `::file_exists::a_valid_remote_reference_without_a_runtime_is_still_an_error`; darkmatter unit `fs_tests::malformed_references_are_false_before_any_access` |
| 15 | `DT/expression_call_diagnostics.rs::file_exists_takes_a_number_silently` (adjacent) | always-false literal warning (P4) | `DT/expression_call_diagnostics.rs::file_predicates::a_malformed_literal_reference_warns_and_a_string_variable_does_not`; `LT/file_predicates.rs::always_false::only_a_malformed_literal_reference_is_always_false` |
| 16 | `LT/design_time_call_check.rs::the_worked_example_rows` (category 4 machinery) | guarded `frontmatter` category 1 (P4) | `LT/file_predicates.rs::guarded_file_arguments::an_is_file_type_guard_makes_the_file_argument_category_1`; `DT/expression_call_diagnostics.rs::file_predicates::under_strict_only_the_unguarded_file_argument_warns` |
| 17 | `LT/function_binding_registry.rs::every_registered_function_has_a_row` (built-ins) | host predicate, catalog, never-invoked counter (P5) | `LT/host_functions.rs::installed_host_functions_get_the_built_in_checks_and_run_only_when_evaluated`, `::registration_rejects_collisions_parity_gaps_and_invalid_facts_before_evaluation`; `DT/host_function_diagnostics.rs::the_host_catalog_and_its_artifact_check_host_calls_like_built_ins` (P5) |
| 18 | DMLS `config/mod.rs` unit tests (strict only) | assert no `file_exists` setting (P4) | `DT/expression_call_diagnostics.rs::file_predicates::the_configuration_has_no_file_exists_setting` |
| 19 | `DT/call_check_agreement.rs::composition_and_the_editor_reject_the_same_documents`; `DT/recursive_call_check.rs::the_editor_reports_exactly_the_recursive_call_composition_rejects` | identical fields across composition, report-json, DMLS (P6, P8) | `CT/compose_report_json.rs::a_rejected_call_reports_the_shared_record` (report == library `error_diagnostics`); `DT/expression_call_diagnostics.rs::shared_fields::a_rejected_call_reports_the_editor_fields`, `::a_category_3_warning_reports_the_editor_fields` (report records == editor, P8 leg) |
| 20 | none | category 3 warning; `--deny all` fails (P6, P7) | `LT/shared_diagnostics.rs::a_category_3_call_composes_with_a_located_warning`, `::a_lifecycle_surface_reports_the_same_located_warning`; `CT/compose_call_warnings.rs::*`, `::deny_all_fails_the_category_3_document` (P6, P7) |
| 21 | `LT/unknown_identifier_warning.rs::every_suppression_row_warns_exactly_where_evaluation_reads_an_unhandled_root`, `::a_handled_absence_is_silent_and_renders_the_fallback`; `DT/expression_scopes.rs::undeclared_property_findings_match_the_library_branch_facts`; `absence.rs` unit tests | `\| null` type rule; no name list (P6) | `LT/unknown_identifier_warning.rs::a_parameter_declaring_null_handles_a_direct_missing_argument`, `::a_non_null_parameter_handles_nothing_and_its_binding_error_stays_visible`, `::absence_handling_names_no_function`; `DT/expression_scopes.rs::undeclared_property_findings_follow_the_parameter_types` (P6) |
| 22 | `DT/no_side_effects.rs::expression_validation_runs_no_actions_shells_file_effects_or_lazy_providers` | instrumented report privacy/passivity (P6, P8) | `LT/report_diagnostics.rs::report_diagnostics_hold_no_value_and_read_nothing`; `CT/compose_report_json.rs::the_report_holds_no_value_or_file_content` |
| 23 | none | precedence and conflict usage errors (P7) | `CT/policy_verbs.rs::a_specific_denial_beats_allow_all_in_either_order`, `::a_specific_recovery_beats_deny_all_in_either_order`, `::opposite_selections_are_usage_errors_in_either_order_before_any_effect`, `::a_recovery_denied_by_its_code_conflicts_with_its_allowance` (P7) |
| 24 | `CT/compose_shell.rs::test_compose_allow_shell_timeout_emits_warning`; `CT/compose_refs_and_missing.rs::test_compose_scalar_ctx_with_allow_override_succeeds`; `CT/compose_state_set.rs::test_set_overlay_permissive_reassigned_warns_and_rightmost_wins`, `::test_set_overlay_permissive_invalid_warns`; `CT/compose_remote_caching.rs::test_compose_remote_allowed_host_fetches_url` (no missing-reference allow tests exist) | port each to `--allow <id>`; `--allow all` grants no host/shell (P7) | `CT/policy_verbs.rs::each_missing_reference_allowance_recovers_its_own_kind_only`, `::allow_all_recovers_a_shell_timeout_as_an_empty_string`, `::allow_all_grants_no_unnamed_host`, `::allow_all_authorizes_no_shell_command`; ported legacy tests (P7 mapping); `LT/blocking_policy.rs::each_recovery_reports_under_its_registered_code` (P7) |
| 25 | `CT/schema_trigger_rename.rs::the_old_flag_is_a_usage_error_naming_the_new_one`, `::completions_offer_only_the_new_flag`; `LT/trigger_spelling_guard.rs::no_active_file_spells_a_renamed_trigger_form` | every removed flag on every command; spelling guard (P7) | `CT/policy_verbs.rs::every_removed_flag_is_a_usage_error_naming_its_replacement`, `::completions_offer_the_registry_identifiers_per_command`; `CT/schema_trigger_rename.rs::the_old_flag_is_a_usage_error_naming_the_new_one`, `::completions_offer_only_the_new_flag`; legacy-spelling grep re-run in `## Phase 9` (P7, P9) |
| 26 | `CT/layout_style_frontmatter.rs::style_strict_style_fails_on_unknown_key`, `::style_strict_style_fails_on_deprecated_key`, `::style_strict_style_accepts_markdown_cleanup_rules` | `--deny style`; info never blocks (P7) | `CT/policy_verbs.rs::deny_style_and_deny_all_block_a_style_issue_on_every_render_path`; `CT/layout_style_frontmatter.rs::deny_style_fails_on_unknown_key`, `::deny_style_fails_on_deprecated_key`, `::deny_style_accepts_markdown_cleanup_rules`; darkmatter-cli unit `render::tests::a_denied_style_blocks_schema_issues_and_inline_rules_but_never_a_notice` (P7) |
| 27 | `LT/compose_call_precheck.rs::a_proven_bad_call_rejects_before_any_expression_runs`, `::a_bad_body_call_rejects_before_frontmatter_runs` | stop-before-effect / no rollback under `--deny` (P7) | `CT/policy_verbs.rs::a_passive_blocking_diagnostic_stops_before_initialize`, `::a_blocking_diagnostic_found_after_an_effect_stops_later_work_without_rollback`; `LT/blocking_policy.rs::a_blocking_call_warning_stops_the_document_before_any_expression_runs`, `::a_transcluded_child_blocks_before_its_own_expressions_run` (P7) |
| 28 | `CT/schema_trigger_rename.rs::compose_reports_an_invalid_trigger_field_by_location_and_composes_nothing` (partial) | no stdout document in all modes; no viewer/file publish (P7, P8) | `CT/compose_report_json.rs::deny_all_reports_the_blocking_warning_once_and_no_document` (report leg; human/json legs P7) |
| 29 | none | report-json success/failure schema (P8) | `CT/compose_report_json.rs::success_reports_the_document_and_leaves_other_outputs_unchanged`, `::a_category_3_warning_is_reported_once_with_every_field`, `::a_rejected_call_reports_the_shared_record`, `::an_unreadable_input_is_one_failure_with_null_fields` |
| 30 | this log (baseline below) | "after" sample + explanation (P8) | this log (`## Phase 8` → Performance "after"); `LT/variable_type_projection.rs::bounds::a_path_past_the_projection_depth_bound_stays_unknown` |
| 31 | `LT/expression_type_scope.rs::bare_and_fm_reads_share_one_document_binding_unless_a_host_root_shadows_it`, `::an_unavailable_global_stays_unavailable_under_any_guard`; `LT/host_globals_compose.rs::reading_an_unavailable_global_is_a_typed_binding_error_in_body_and_frontmatter` | type-level bare/`fm` agreement (P2) | `LT/variable_type_projection.rs::precedence::bare_and_fm_paths_agree_unless_a_host_global_shadows_the_bare_one` |
| 32 | `LT/expression_type_scope.rs::constant_paths_and_indices_are_subjects_and_computed_ones_are_not`; `LT/recursive_schema_typed_consumers.rs` | item type + null; terminating recursive path (P2) | `LT/variable_type_projection.rs::nested_paths::constant_indices_project_the_item_type`, `::nested_paths::a_path_through_a_recursive_type_terminates` |
| 33 | `LT/expression_type_scope.rs::a_disjunction_proves_what_both_routes_prove`, `::negation_swaps_sides_and_parentheses_keep_facts`; `LT/compose_call_precheck.rs::a_dead_ternary_branch_is_checked` | re-assert on extended scope (P3) | `LT/flow_sensitive_narrowing.rs::compound_guards::short_circuit_operands_and_ternaries_see_only_justified_facts`, `::compound_guards::an_impossible_branch_is_checked_with_the_incoming_type`, `::body_guards::composition_reads_the_same_narrowing` |
| 34 | `LT/expression_type_scope.rs::a_write_invalidates_its_aliases_ancestors_and_descendants_only`, `::a_join_restores_only_incoming_facts_no_branch_could_have_written`; `CL/lifecycle/executor/tests/guarded_shell_sites.rs::a_set_before_the_shell_ends_the_guard_proof`; `CL/lifecycle/executor/tests/scope_site_conformance.rs::lifecycle_set_shell_negated_guard` | lifecycle guard → `set` → later call, runtime and DMLS (P3) | `LT/flow_sensitive_narrowing.rs::lifecycle_fact_lifetime::a_set_ends_the_guard_and_events_never_share_facts`, `::lifecycle_fact_lifetime::overlapping_writes_end_facts_and_siblings_do_not`, `::lifecycle_fact_lifetime::the_lifecycle_rejection_reads_each_site`; `DT/expression_call_diagnostics.rs::narrowing::a_lifecycle_guard_ends_at_a_write_and_its_event`; `CL/lifecycle/tests/call_check.rs::a_guard_narrows_until_a_write_and_never_reaches_a_shadowing_host_global` |
| 35 | `CL/lifecycle/executor/tests/scope_site_conformance.rs::file_exists_alias`; `LT/scope_site_conformance.rs::file_exists_guards_are_analyzed_without_running_a_handler` | context-free `is_file_type`; passive validator; `fileexists` parity (P4) | `LT/file_predicates.rs::is_file_type::*`, `::file_exists::*`, `::always_false::*`, `::composition::composition_warns_for_each_always_false_call_and_composes_the_guarded_read`; `CT/compose_file_predicates.rs::compose_answers_file_predicates_and_warns_for_an_always_false_call`; `biscuit-file/lib/tests/l1/structural_validation.rs::*` |
| 36 | `LT/design_time_call_check.rs::criterion_36_pairs`, `::null_is_accepted_only_by_an_explicit_null_arm` | `\| null` suppression incl. host and overloads (P6) | `LT/unknown_identifier_warning.rs::only_an_explicit_null_in_every_overload_handles_a_missing_argument`, `::a_non_null_parameter_handles_nothing_and_its_binding_error_stays_visible`; `LT/host_functions.rs::a_host_parameter_declaring_null_handles_a_missing_argument` (P6) |
| 37 | `LT/function_binding_registry.rs::*`; `LT/function_catalog_parity.rs` | registration errors; `.facts.json` matrix (P5) | `LT/host_functions.rs::registration_rejects_collisions_parity_gaps_and_invalid_facts_before_evaluation`, `::the_committed_facts_artifact_is_fresh_and_deterministic`, `::the_facts_artifact_reader_walks_the_input_robustness_matrix`; `DT/host_function_diagnostics.rs::a_missing_host_facts_artifact_reports_incomplete_analysis`, `::a_host_facts_artifact_for_another_catalog_version_reports_incomplete_analysis`, `::a_host_facts_artifact_lacking_a_function_reports_it_incomplete` (P5) |
| 38 | `CT/clean_schema.rs::test_baseline_schema_flags_conflict`; `CT/schema_validate_baseline_selection.rs::no_baseline_schema_conflicts_with_schema_flag`; `CT/schema_trigger_rename.rs::the_new_flag_disables_triggers_on_compose_validate_and_clean` | registry mappings, unknown/repeated ids, host denial, precedence (P7) | `CT/policy_verbs.rs::unknown_and_empty_identifiers_are_usage_errors`, `::identifiers_a_verb_or_command_does_not_take_are_usage_errors`, `::verbs_a_command_does_not_declare_are_usage_errors`, `::repeated_identical_selections_are_harmless`, `::disabling_the_baseline_conflicts_with_an_explicit_baseline`, `::a_host_is_an_allowance_only`, `::a_disabled_baseline_stays_disabled_under_deny_all`, `::an_independently_denied_diagnostic_still_blocks_a_recovered_command`; `CT/clean_schema.rs::test_baseline_schema_flags_conflict`; `CT/schema_validate_baseline_selection.rs::disable_baseline_schema_conflicts_with_schema_flag` (P7) |
| 39 | none | report-json failure kinds, transcluded positions, non-compose rejection (P8) | `CT/compose_report_json.rs::*` (input-read, schema, YAML parse, runtime, transcluded, lifecycle, reference, non-compose rejection); `LT/report_diagnostics.rs::*` |

### Spike S1: biscuit-file structural-validation gap

Read from `biscuit-file/lib/src/file_reference/parse.rs` (**P**),
`resolve.rs` (**R**), `darkmatter/…/functions/fs.rs` (**FS**), and
`resolve_ctx.rs` (**RC**). Results were confirmed with the installed `md`
(built 2026-10-23, same code paths) in a scratch repository.
`FileReference::new` (`file_reference/mod.rs:615`) is `parse::parse` (P:9),
purely lexical with no I/O. `file_exists` today: a non-string is `false`; an
`is_remote_url` string (RC:330) is fetched; anything else goes through
`normalize_path_arg` (RC:342) → `FileReference::new` → resolve.

| Input | `FileReference::new` today | `file_exists` today | Required `is_file_type` | Gap |
|---|---|---|---|---|
| `./docs/spec.md` | Ok ExplicitRelative | local resolve | true | none |
| `/tmp/spec.md` | Ok Absolute | local (Windows: `ForeignAbsolutePath`, R:196) | true | none |
| `\Users\ken\data\foobar.json` | Ok ImplicitRelative (P:212) | local → false | true | none |
| `@docs/spec.md`, `%@spec.md` | Ok Magic (recursive for `%`) | local | true | none |
| `vault:notes/spec.md` | Ok Vault | `VaultNotConfigured` error (R:938) | true | none |
| `https://example.com/spec.md`, `http://…` | Ok Url (P:93) | remote fetch / policy | true | none |
| `42` (string), `hello world` | Ok ImplicitRelative | local → false | true | none |
| `""` | Err InvalidSyntax (P:12) | **error, not false** | false | darkmatter must return `false` (P4) |
| `https://`, `http://` | **Ok Url** (no authority check) | collapse turns it into `https:/` → UnsupportedScheme error | false | **URL authority check** |
| `&docs/spec.md`, `^docs/spec.md` | Ok RepositoryRoot / RepositoryScoped | local | true | none |
| `~/spec.md`, `~` | Ok Home | local | true | none |
| `{{HOME}}/x` (env template; grammar `{{NAME}}`, `NAME` in `[A-Z0-9_]+`) | Ok + EnvVar (P:248–291) | local; unset var errors | true | none |
| `${HOME}/x`, `$HOME/x` | Ok, literal text | local → false | true (legal file name) | none |
| `{{HOME/x`, `{{}}`, `{{home}}`, `{{env.X}}` | Err InvalidSyntax (P:260/266/273) | error | false | none |
| `a\0b.md` | **Ok ImplicitRelative** | reaches `fs::metadata` (R:363) → error | false | **NUL check** |
| `ftp://x/y`, `mailto:a@b`, `s3://b/k` | Err UnsupportedScheme | error | false | none |
| `file://foo/bar`, `file:///tmp/spec.md` | Err UnsupportedScheme | adapter strips `file://` → local | true (through adapter) | none (adapter stays in Darkmatter) |
| `file://` | Err UnsupportedScheme | adapter → `""` → error | false | none |
| `\\server\share\x`, `C:\x\y.md` | Ok Absolute | macOS: `ForeignAbsolutePath` | true | none |
| `//server/share/x` | Ok Absolute | **adapter collapses to `/server/share/x`** | true | adapter defect (Darkmatter, D5) |
| `C:x`, `\\?\C:\x` | Err | error | false | none |
| `@`, `&`, `^`, `@/`, `%`, `%%x`, `!x`, `~bob/x` | Err (P:19, 53, 112, 146–161, 197) | error | false | none |
| `vault:`, `vault::` | **Ok Vault, empty payload** | error / probes vault root | false | **empty vault payload check** |
| `vault:/etc/hosts` | **Ok Vault, rooted payload** | joins as absolute, escaping the vault root | false | **rooted vault payload check** (D4) |
| `@@x` | Ok Magic, payload `@x` (documented valid, `mod.rs:646`) | local | true | none |
| `%https://example.com/x` | **Ok Url, recursive** | `RemoteNotLocal` at resolve (R:88) | false | **recursive-URL check** |
| `https:///x` | Ok Url | url crate makes host `x` → fetch | false (D4) | covered by authority check |

**Structural checks to add to biscuit-file** (Phase 4, all in `parse::parse`
/ `detect_kind`, so `FileReference::new` itself is the passive validator):

1. **NUL rejection** at the top of `parse` (next to the empty check, P:12):
   `InvalidSyntax` for any `\0`, every kind.
2. **URL authority check** in the URL branch (P:93–95): a non-empty literal
   authority after `scheme://`, validated with the same `url::Url::parse` +
   host check resolve already does (R:407–417). An authority that is a
   `{{NAME}}` template is accepted when non-empty; variables are never read.
3. **Recursive URL rejection:** `%` combined with a URL (`%https://…`) is
   `InvalidSyntax` at parse time.
4. **Vault payload checks:** an empty payload (`vault:`, `vault::`) and a
   rooted payload (`is_rooted_magic_payload`, P:229) are `InvalidSyntax`.

**`file://` adapter** (`normalize_path_arg`, RC:342–360; an engine-local copy
at `darkmatter/lib/src/effects/verbs.rs:329–345`): strips one leading
`file://`, then collapses every run of `/`. The collapse violates spec §3's
"do not collapse separators in a way that changes UNC or URL meaning"
(`//server/share/x` and `file:////server/share/x` lose their UNC meaning).
`file_exists` routes URLs with Darkmatter's own `is_remote_url`, separately
from `FileReference`'s `Url` kind. The required Phase 4 order is:
adapter → `FileReference::new` (on `Err` return `false`) → route on the
reference's `Url` kind for the remote fetch, otherwise resolve locally.

### Performance baseline ("before")

- **Host:** Apple M4 Max, 16 cores, 128 GiB, macOS 27.2. Load average 7–9
  during measurement (other sessions on the host).
- **Build:** `cargo build --release -p darkmatter-cli --bin md -p dmls --bin dmls`
  at `817792c35`, binaries in `target/release/`.
- **Input:** `darkmatter/cli/tests/fixtures/frozen_prompts/prompts/plan.md`, a
  frozen copy of the repository's planning prompt (5.6 KB, 16 `{{ … }}`
  sites). It has a root-union `$schema` (the shape criterion 1 changes),
  `||` and `?:` expressions, and lifecycle stacks with expressions. It
  composes successfully with `spec=claudine/features/2026-10-31-type-aware-evaluation/spec.md`.
- **Composition:** `hyperfine -N --warmup 3 --runs 20 "target/release/md compose <input> spec=<spec>"`
  → **797.9 ms ± 5.5 ms** (range 787.1–807.1 ms; user 765 ms, system 219 ms).
  `--perf` prints no report for this document (it takes the lifecycle path),
  so the wall-clock figure includes context capture and I/O.
- **Schema stages (checking vs other work):** `md schema validate -vv <input>`
  (exits 1 without `spec`, but the timings are still reported): resolve
  8.3 ms, compile 28.5 ms, validate 48.3 ms, total **85.5 ms**.
- **DMLS analysis:** `python3 -I claudine/features/2026-10-31-type-aware-evaluation/perf/lsp_drive.py target/release/dmls <input> . 10`
  (stdio LSP; workspace root = repo; didOpen → `publishDiagnostics`; 0
  diagnostics). First publish: median **250.1 ms** (min 245.2, max 274.8).
  The second publish (schema compile cache hit, which is mostly the
  expression and diagnostic work) followed after **41.8–48.5 ms**. DMLS's own
  `schema validation complete` log for the first publish: resolve 27–29 ms,
  compile 27–30 ms, validate 45–48 ms, total **101–107 ms**.
- Phase 8 reruns exactly these three commands on the same input and host.

### Design Decisions

- **D1. The dependency gate passes; the fact slot and host registration are
  this spec's work.**
  - *Decision:* The static `PREDICATE_FACTS` table (`scope/facts.rs`) is
    `TypeScope`'s fact registry. Phase 3 turns it into facts carried on
    `FunctionRegistration`, and Phase 5 adds the public host registration
    path (function catalog loading through the strict `load_catalog`, plus
    registration). Work does not stop under Rule 2.
  - *Alternatives:* stop and report these as missing Phase B seams.
  - *Why:* spec §2 says new facts register "into the same registry built-ins
    use", which today is that table. Spec §5 makes host registration this
    spec's deliverable ("through the same Rust registration API the built-ins
    use"). Neither is a Phase B outcome; Phase B's deliverables (catalog
    `returns`, categories, call check, strict, projection) are all present.
  - *Downsides:* Phases 3 and 5 carry the registry-opening work the plan's
    audit list described as a pre-existing slot.
- **D2. Criterion 8 wires the call check into Darkmatter's lifecycle walk.**
  - *Decision:* Phase 2 runs `check_expression_calls` from
    `darkmatter/lib/src/lifecycle/validate.rs`'s action walk, under
    `lifecycle_declared_types`, so `md` and Claudine hosts both get it.
  - *Alternatives:* a Claudine-side check in its lifecycle parser.
  - *Why:* lifecycle validation moved into Darkmatter
    (`claudine/lib/src/composition/lifecycle/validate.rs` no longer exists),
    and rule 3 forbids a second implementation.
  - *Downsides:* Claudine lifecycle fixtures whose actions hold a provably
    bad call start failing; such a fixture holds a real bug.
- **D3. Which Claudine scans move (spec §5's last paragraph).**
  - *Decision:* Phase 5 moves every Claudine walk that decides which roots
    or paths an expression reads onto Darkmatter's reads API
    (`static_variable_reads` / `binding::is_reserved_namespace`, extended
    there if needed): `looping/config.rs::collect_identifiers` and its
    callers; `lifecycle/bindings.rs::removed_loop_read*` with
    `binding_checks.rs::find_removed_loop_read` (the successor of the
    deleted `validate.rs` scan); `bindings/rename.rs::read`/`static_path`;
    and `sequence/preflight/shape.rs::target_identity_path_in_expr`.
    `prewarm/classify.rs` already takes its roots from
    `static_variable_reads`, and `dispatch/matcher.rs` is a grammar heuristic,
    so both stay.
  - *Alternatives:* move only the two files the spec names.
  - *Why:* the spec's rule is "Claudine keeps no independent definition of
    variable visibility or types", and the spec's two files have since
    moved or split. Every walk listed defines visibility locally.
  - *Downsides:* a wider Claudine diff in Phase 5.
- **D4. Structural verdicts for edge spellings S1 surfaced.**
  - *Decision:*
    - `https:///x` (empty literal authority) is malformed, so `is_file_type`
      is `false`.
    - `vault:`, `vault::`, and rooted vault payloads (`vault:/etc/hosts`)
      are malformed.
    - `%https://…` is malformed.
    - `@@x` stays valid (it is documented grammar).
    - `${X}`/`$X` stay literal file-name text.
  - *Alternatives:* accept `https:///x` because the `url` crate normalizes
    it; leave rooted vault payloads to resolve.
  - *Why:* spec §3 makes malformed reference syntax, including an empty URL
    authority, `false`. Reserved malformed forms are false rather than
    implicit local paths. A rooted vault payload escapes its root, as a
    rooted magic payload does, and the parser already rejects that one.
  - *Downsides:* `vault:/x` and `%https://…` become parse errors where they
    used to fail at resolution; error text changes for those inputs.
- **D5. The `file://` adapter stops collapsing separators.**
  - *Decision:* Phase 4 changes `normalize_path_arg` (and its copy in
    `effects/verbs.rs`) to strip the `file://` prefix only. The structural
    check then runs on the unmodified remainder.
  - *Alternatives:* keep the collapse and special-case UNC.
  - *Why:* spec §3 forbids collapsing in a way that changes UNC or URL
    meaning. The collapse is also why `https://` reaches `UnsupportedScheme`
    today instead of a structural rejection.
  - *Downsides:* a path with doubled slashes (`a//b`) is no longer
    normalized by Darkmatter. Resolution treats it the same on every OS.
- **D6. Baseline input and method.**
  - *Decision:* the frozen `plan.md` prompt, release builds, hyperfine for
    composition, and the checked-in stdio driver
    (`perf/lsp_drive.py` in this feature directory) for DMLS.
  - *Alternatives:* a live `prompts/` file (it may change between phases); a
    Criterion bench (does not cover DMLS).
  - *Why:* a frozen input guarantees the same bytes in Phase 8. It exercises
    root unions, `||`, `?:`, and lifecycle expressions. The driver measures
    the editor path without a client.
  - *Downsides:* composition time is dominated by context capture and I/O;
    the checking share is visible only through the schema stage timings and
    DMLS's cache-hit republish.
- **D7. Legacy-flag migration has no static completion files.**
  - *Decision:* Phase 7 updates clap definitions only for completions, and
    extends `completions_offer_only_the_new_flag`-style tests to every
    migrated command. `--strict-style` lives on the top-level `Cli`
    (default `md FILE` only), and `md render` declares none of the flags
    today, so `--deny style` and the verbs are added there by the registry.
  - *Alternatives:* none.
  - *Why:* the inventory found no static completion, justfile, script, or
    example uses.
  - *Downsides:* none.

### Gates

- **Scope calculator:** `git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`
  → `packages: ["darkmatter"]`, `change_class: package`. `plan.md` and this log
  are test inputs of two `darkmatter` L1 tests (`--resolved-plan` filter:
  `shipped_schema_corpus::shipped_markdown_inline_schemas_parse_passively`,
  `shipped_schema_corpus::shipped_markdown_named_type_references_resolve`).
  `perf/lsp_drive.py` alone selects nothing (`documentation`). The gates
  were therefore **not** skipped.
- **Narrowed darkmatter cell:** `cargo nextest run -p darkmatter --test l1 -E 'test(=shipped_schema_corpus::shipped_markdown_inline_schemas_parse_passively) | test(=shipped_schema_corpus::shipped_markdown_named_type_references_resolve)'`
  → 2 passed.
- **`claudine/` `just test`:** 11445 passed (13 slow), 9 skipped, 296.6 s.
- **`claudine/` `just lint`:** passed.
- **`just check-tier-coverage claudine`:** passed, 0 stranded.
- No pre-existing or load-sensitive failures. No cross-OS check: Phase 1
  changed no product code.
- **Requirement-to-test mapping:** Phase 1 changes no behavior, so it adds
  no tests. The traceability table above maps every criterion to the
  existing tests and to the phase that adds the missing ones.
- **Non-product file created:** `claudine/features/2026-10-31-type-aware-evaluation/perf/lsp_drive.py`
  (measurement driver for criterion 30; not package source).

## Phase 2

Phase 2 completes the variable-type projection (spec §1; criteria 1–8, 31,
32). Everything lands in the one projection, `DeclaredTypes`
(`darkmatter/lib/src/markdown/compose/expression/scope/projection.rs`), and
in `TypeScope`'s `Shape`. No consumer walks schema syntax itself.

### What changed

- **Union normalization** (`scope/shape.rs`): `Shape::union` is the one
  operation every projection, root-union fold, scope join, and operator type
  uses. `unknown` absorbs (including `null`). Members are deduplicated by
  type identity: presence markers, timing, annotations, and descriptions are
  stripped from top-level members, because a shape carries presence in
  `nullable`. Members are sorted canonically by their serialized type text.
  `Shape::of_type` is a type's values with no optionality (a `returns(...)`);
  `Shape::from_definition` adds `null` for an optional property;
  `Shape::to_definition` converts back for `DeclaredType::Property`.
- **Layers** (`projection.rs`): `DeclaredTypes::for_schema(Option<&EffectiveSchema>)`
  reads the authored SimplifiedSchema, then the trigger shapes (later
  wins), then `RawDeclarations` of the merged JSON Schema, which carries the
  baseline and every raw layer. A root union folds per arm; an arm that does
  not declare a key takes the lower layers' declaration, else `unknown`.
  Arms that agree exactly keep the authored definition (constraints
  included) for shell-hole validators.
- **Definitions across layers**: `EffectiveSchema::layered_definitions()`
  merges the document's table with each trigger layer's (`DefinitionTable::absorb`,
  cached in a `OnceLock`). `DeclaredTypes::with_definitions` keeps its
  `&DefinitionTable` parameter (decision P2-D2).
- **Raw JSON Schema** (`scope/raw.rs`, `RawDeclarations`, cached by
  `EffectiveSchema::raw_declarations()`): literal `properties` through
  root `anyOf`/`oneOf` (folded) and `allOf` (intersected); `type`, `const`,
  `enum`, `items`, the two file formats; `$ref` and everything else are
  `unknown`.
- **Nested paths**: `DeclaredTypes::projected` walks constant members and
  indices through inline objects, arrays, tuples, and recursive references
  (one definition opened per segment, alias chains bounded by the table
  size). `DeclaredType::NestedProperty` is removed: a typed path is
  `Property(def)`, an untyped one `NoProjection`.
- **Reserved roots and host globals**: `ctx`/`env`/`err`/`timing` take
  Darkmatter's catalog shapes; a host global takes its host catalog's shape
  (`DeclaredTypes::with_global_catalog`, `EventRegistry::register_global_catalog`,
  registered by Claudine's `claudine_registry()` and set on the sequence
  preflight sites). An available global's root is present, an
  execution-dependent one nullable, and an unavailable one has no type.
  `ctx` members keep their descriptor presence.
- **Call and operator typing** (`call_check.rs::expression_type`, public):
  call = union of the `declared_returns` (new `CompiledSignature` field) of
  every overload the argument count selects; `x || y` and `c ? a : b` as the
  spec states, with operands typed under `TypeScope::assume`; anything else
  `unknown`. `ArgumentType::of` reads it.
- **Lifecycle call check (criterion 8, decision D2)**:
  `darkmatter::lifecycle::reject_incompatible_calls` checks every surface
  `walk_stack` yields, from authored text, with each event's binding view.
  The error is the new `LifecycleError::LifecycleCallRejected` (with
  `rejected_call()`), and its Claudine twin is
  `CompositionError::LifecycleCallRejected` (mapped, rendered, coded
  `composition.lifecycle_invalid`, and in the JSON-detail and property
  groups). md calls it from `md compose` before the lifecycle run, against
  `darkmatter::markdown::compose::effective_schema_for` (new public helper,
  the pipeline's own schema assembly). Claudine calls it in `prepare.rs`
  right after the launch schema resolves (direct and inline preparation).
- **Callers moved to `for_schema`**: composition precheck, directive
  targets, shell-site discovery (`preflight/collect.rs`) and fill
  (`shell_expansion/site.rs`, which now stores the boxed `EffectiveSchema` so
  discovery and fill type identically), `lifecycle_declared_types`, DMLS
  call diagnostics and site scopes, and Claudine's sequence preflight.

### Design Decisions

- **P2-D1. Canonical member order is the serialized type text.**
  - *Decision:* `Shape::union` sorts members by `serialize_property_atom` of
    the stripped member, so messages print `number | string`.
  - *Alternatives:* keep authored order (not order independent, which
    criterion 2 requires); rank by value family first.
  - *Why:* the annex requires one canonical order. Text order is total,
    deterministic, and needs no second ranking table.
  - *Downsides:* messages and hovers no longer echo the authored order. Ten
    existing tests that pinned `string | number` were updated.
- **P2-D2. Layered definitions are one merged table, not a lookup trait
  object.**
  - *Decision:* `EffectiveSchema::layered_definitions()` returns one
    `DefinitionTable` holding the document's definitions and every trigger
    layer's. `with_definitions` keeps `&DefinitionTable`.
  - *Alternatives:* widen `with_definitions`, `check_call_with_definitions`,
    `classify_argument_with_definitions`, subtyping, and the call binder's
    translator to `&dyn DefinitionLookup`.
  - *Why:* a `DefKey` names one definition wherever it is reached, so the
    merge resolves exactly what `EffectiveSchema::definition(&DefKey)` does
    (spec §11's requirement). It needs no change to the call-binding and
    subtype APIs, and it is computed once per schema.
  - *Downsides:* one table clone per effective schema with trigger
    definitions (none when no trigger has any).
- **P2-D3. The baseline is projected from the merged JSON Schema.**
  - *Decision:* layers that `EffectiveSchema` holds only as JSON (the
    baseline, raw document or trigger schemas) project through
    `RawDeclarations` of `json_schema`.
  - *Alternatives:* store the baseline's SimplifiedSchema on
    `EffectiveSchema`.
  - *Why:* a configured baseline may itself be raw JSON. The merged JSON is
    "the final EffectiveSchema after resolution and merging", which the spec
    names, and it needs no new stored state.
  - *Downsides:* baseline types are the conservative raw projection (for
    example `date` is `string`). A sound supertype never proves a call
    falsely valid or invalid.
- **P2-D4. A root-union arm that lacks a property takes the lower layers'
  declaration.**
  - *Decision:* an arm with no declaration of a key contributes the
    trigger's or the raw layer's declaration of it, and only otherwise
    `unknown`.
  - *Alternatives:* always `unknown` for a missing arm declaration.
  - *Why:* the merged schema merges the baseline and triggers into every
    arm, so the arm does declare the property at runtime.
  - *Downsides:* the raw fallback is the fold over every merged arm, which
    is a supertype of that arm's own.
- **P2-D5. Lifecycle rejection ignores guards.**
  - *Decision:* `reject_incompatible_calls` types each surface at an
    unguarded site. It has an allowlist entry in
    `type_scope_ownership_guard.rs`.
  - *Alternatives:* narrow each surface by its enclosing `when`.
  - *Why:* narrowing only removes alternatives, and an impossible branch
    keeps its incoming type. A call that is category 5 unguarded is category
    5 under every guard, so the verdict is identical. Guard-sensitive
    warnings (categories 2 to 4) on lifecycle surfaces are Phase 3/6 work.
  - *Downsides:* none for rejection.
- **P2-D6. A new lifecycle twin rather than reusing `MarkdownError`.**
  - *Decision:* `LifecycleError::LifecycleCallRejected` carries the
    `ExpressionError::CallCheck` as its source, and Claudine's
    `CompositionError` twin mirrors it.
  - *Alternatives:* raise a `MarkdownError::Interpolation` from the
    lifecycle layer.
  - *Why:* lifecycle validation errors are the `LifecycleError` family. The
    twin rule (`lifecycle_authoring.rs`) keeps one mapping, and the verdict
    and message are the body's (`rejected_call()`).
  - *Downsides:* one more twin pair to render.
- **P2-D7. An available host global's root is present.**
  - *Decision:* `Availability::Available` removes `null` from the root's
    type, `ExecutionDependent` keeps it, and `Unavailable` has no type.
  - *Alternatives:* use the catalog definition's own optionality, which is
    always optional.
  - *Why:* availability is the host's statement that the binding exists in
    that scope.
  - *Downsides:* `err.msg` now proves present in failure events; presence
    consumers see that evidence.
- **P2-D8. `SiteScope` boxes its schema.**
  - *Decision:* `shell_expansion::site::SiteScope` holds
    `Option<Box<EffectiveSchema>>`.
  - *Why:* storing the effective schema inline grew the pipeline's
    stage-dispatch frame (`run_inline_pre_operation`) by 1 KB per
    nested-composition level. With that,
    `nested_composition::self_composition_reaches_the_depth_limit` overflowed
    a 2 MiB debug test thread. Measured with lldb: HEAD already needed 2.0–2.1
    MB for that test, so it sits within 5% of the default stack. That margin
    is a pre-existing risk, recorded here, not introduced by this phase.
  - *Downsides:* one allocation per document with shell blocks.

### Requirement-to-test mapping

See the traceability table in Phase 1 ("Added test(s)" column, criteria 1–8,
31, 32). New file `darkmatter/lib/tests/l1/variable_type_projection.rs`
(declared in `tests/l1/main.rs` under `effects-instrumentation`, which CI
enables for the library), plus
`darkmatter/cli/tests/l1/compose_lifecycle.rs::a_provably_wrong_lifecycle_call_rejects_before_any_event`,
`darkmatter/dmls/tests/l1/expression_call_diagnostics.rs::settings::rows_f_and_i_are_silent_under_both_settings`,
and `claudine/lib/src/composition/lifecycle/tests/call_check.rs` (two tests).
Passivity (criterion 7) is proven with a deleted fixture tree as a failing
file-access stub and handler-entry counters, not return values alone.

Updated existing tests (behavior changed by design):
`design_time_call_check.rs` (rows f, i now category 1; bare `env` is an
`object`, so `min(env, 5)` is category 5; canonical order),
`expression_type_scope.rs` (canonical order; `NestedProperty` removed),
`directive_target_analysis.rs` (a raw-JSON-only and a disagreeing root-union
`optional` are now typed, so nullable), `dmls/tests/l1/expression_scopes.rs`
(canonical order), `type_scope_ownership_guard.rs` (allowlist and baseline
snippet for the lifecycle site), and `lifecycle_authoring.rs` (one more
twin).

### Gates

- **Scope calculator:** `git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`
  → `packages: ["claudine", "darkmatter", "darkmatter-cli", "dmls", "test-toolkit", "claudine-cli"]`,
  `change_class: package`. Gates were run.
- **darkmatter library** (`cargo nextest run -p darkmatter --features effects-instrumentation`, L1 filter):
  9514 passed, 4 skipped (before the last lifecycle test additions; those
  and the module they extend were rerun and pass).
- **darkmatter CLI** (`-p darkmatter-cli --features test-fixtures`, L1): 1250 passed.
- **DMLS** (`-p dmls --features effects-instrumentation`, L1): 1375 run, one
  canonical-order expectation updated, then `expression_scopes` and
  `expression_call_diagnostics` (22) rerun and pass.
- **`claudine/` `just test`:** 11447 passed (14 slow), 9 skipped.
- **`claudine/` `just lint`:** passed. **`darkmatter/` `just lint`:** passed.
- **`just check-tier-coverage claudine`** and **`darkmatter`:** 0 stranded.
- **Workspace compile:** `cargo check --workspace` clean. The other
  darkmatter dependents (`research`, `messenger`, the `*-cli` crates) do not
  compose documents, so no further dependent tests were selected.
- **Failures found and fixed during the phase:** the stack overflow
  (P2-D8), Claudine's `type_scope_ownership_guard` (its declared-layer
  matcher now recognizes `DeclaredTypes::for_schema` as well as `new`, and
  the lifecycle site has a reviewed `Unguarded` entry), and
  `test_placement::repository_test_placement` (inline tests in
  `lifecycle_authoring.rs` exceeded 300 lines; the new helper was compacted).
  No load-sensitive or pre-existing failures.
- **Cross-OS:** not run. The changes are pure type computation over
  in-memory schema values plus temp-dir fixtures that follow existing
  portable patterns, with no path comparison or OS-specific code.

## Phase 3

Phase 3 delivers flow-sensitive narrowing (spec §2; criteria 9–12, 33, 34)
by extending the existing `TypeScope` engine; no second engine exists.
Composition, the call check, lifecycle surfaces, DMLS, and shell template
holes all read the same scope.

### What changed

- **Facts live on registrations** (`call_binding/registration.rs`):
  `NarrowingFact { parameter, index, kind, ty }` with
  `FactKind::{TrueIff, TrueImplies, FalseImplies}` and
  `FactType::{NonNull, Type(text)}`, set with
  `RegistrationBuilder::narrows` and read with
  `FunctionRegistration::narrowing_fact[_for(count)]`. Built-ins declare
  theirs on their `FunctionBinding` (`.narrowing(...)`): `is_string`,
  `is_number`, `is_array` (`unknown[]`), `is_object`, `is_null` (`null`) are
  true-iff; `file_exists` is true-implies `string`; `is_empty` is
  false-implies non-null. The static `PREDICATE_FACTS` table is gone and
  retired in both ownership guards.
- **`scope/facts.rs` reads the registry** through
  `functions::inspect_registration` (a thread's test function first, as
  dispatch resolves it), so a test or host predicate narrows exactly like a
  built-in. `and`/`or` are recognized through `lazy_function`.
- **Recursive arms** (`scope/shape.rs::arm_families`): a `TypeExpr::Ref` arm
  is classified by the top level of its definition body in
  `DeclaredTypes::definitions()`, one level, never expanded; without the
  definition it keeps every family (the delivered result).
  `Shape::of_family`/`without_family` take the definition table, and
  `Shape::suitable_for` implements true-implies.
- **Lifecycle call checks are guard-aware**
  (`lifecycle/validate.rs::lifecycle_call_checks`, `walk_stack_scoped`):
  each surface is typed at its site, narrowed by the enclosing `when:` items,
  facts ended by `action_writes`, never across events.
  `reject_incompatible_calls` is its first category 5. The fact entry and exit
  rules are one pair of functions (`actions.rs::branch_scopes`,
  `scope_after_action`) shared with the mutating `walk_scoped`.
- **Authored locations** (`lifecycle/source_map.rs`): the source map records
  each scalar's frontmatter location (`AuthoredSegment` keys/indices), and
  every `LifecycleSurfaceCalls` carries it.
- **DMLS** (`diagnostics/frontmatter.rs::GuardedStackCalls`): md lifecycle
  stack scalars take the library's guarded checks, matched by location and
  ranged with `StackExpression::scalar_range`; a scalar the library does not
  walk (a `set` shell value), or a lifecycle that does not parse, keeps the
  previous unguarded per-expression check. `call_check::check_diagnostics`
  maps already-made checks to editor findings.

### Design Decisions

- **P3-D1. A narrowing fact is a field of `FunctionRegistration`.**
  - *Decision:* facts are declared with the registration's builder and stored
    on it (`NarrowingFact`); analysis reads them from whatever registration a
    call resolves to.
  - *Alternatives:* keep the static name-keyed table; a separate fact
    registry keyed by function name.
  - *Why:* Phase 1 decision D1 and spec §5 ("the same Rust registration API
    the built-ins use"); a name-keyed table cannot serve host or test
    functions, and Phase 5's artifact needs per-function kind, type, and
    parameter position, which the field holds.
  - *Downsides:* each predicate guard costs one registry lookup.
- **P3-D2. A third kind, false-implies, for `is_empty`.**
  - *Decision:* `FactKind::FalseImplies` proves its type on the false side
    only; `is_empty` registers false-implies non-null.
  - *Alternatives:* keep `is_empty` as a special case in `facts.rs`.
  - *Why:* no fact may be keyed on a name; `is_empty`'s fact is neither
    true-iff nor true-implies.
  - *Downsides:* Phase 5's artifact records three kinds, not two.
- **P3-D3. "Suitable for a type" semantics.**
  - *Decision:* for the bare type of one family, true-implies is
    `of_family` (as before). Otherwise an arm that is a subtype of the target
    stays, an arm sharing a family with it becomes the target, other arms are
    dropped; `unknown` becomes the target; `null` is removed.
  - *Alternatives:* compute an exact type intersection (no such operation
    exists); always replace with the target.
  - *Why:* sound (the result contains every value that satisfies both), never
    invents members, items, or existence, and makes spec §4's example
    (`unknown` → `file`) category 1.
  - *Downsides:* a constrained arm (`string(min(3))`) narrowed by
    true-implies `file` loses its constraint in the narrowed type.
- **P3-D4. Facts are validated at registration now.**
  - *Decision:* `build` rejects a fact on a function that does not return
    `boolean`, on a parameter no overload declares (`UnknownParameter`
    `narrows`), on a rest parameter or one at different positions, a union
    type, and a true-iff type that is not `null` or a bare family.
  - *Alternatives:* leave all validation to Phase 5.
  - *Why:* a fact the engine cannot apply must not reach it; the checks are
    the fact's own shape. Phase 5 still owns name/alias collisions, host
    registration, and criterion 37's matrix.
  - *Downsides:* none found.
- **P3-D5. Lifecycle call checks read the guarded site (supersedes P2-D5).**
  - *Decision:* `lifecycle_call_checks` types every surface under its
    enclosing guards; `reject_incompatible_calls` rejects its first category
    5.
  - *Alternatives:* keep the unguarded rejection and add a separate guarded
    walk for warnings.
  - *Why:* Wave 6 requires lifecycle expressions to use the same engine as
    composition; one walk serves rejection, DMLS, and the later Phase 6
    warnings. An impossible guard keeps the incoming type, so P2-D5's
    guarantee (a category 5 call is rejected under any guard) still holds.
  - *Downsides:* a lifecycle call that is wrong only on its guard's true side
    is now rejected, as the same call in a `::block` already was.
- **P3-D6. DMLS joins the library's surfaces by authored location.**
  - *Decision:* the lifecycle source map records each scalar's frontmatter
    path; DMLS parses the md lifecycle (`md_host::parse_lifecycle`) and
    matches each stack scalar to the library's surface by that path.
  - *Alternatives:* walk the YAML AST with guards in DMLS (a second engine);
    match by expression text (ambiguous for repeated text).
  - *Why:* the editor reports exactly the library's verdict per site.
  - *Downsides:* DMLS parses the lifecycle once per diagnostics pass; a
    lifecycle with a parse error falls back to unguarded per-expression
    checks (the parser's error is the primary finding there).
- **P3-D7. Criterion 11's `is_file_type` and editor host-fact legs land with
  those features.**
  - *Decision:* Phase 3 proves criterion 11 for test-registered predicates in
    composition's scope and the shell template hole. `is_file_type`
    registers its fact in Phase 4 (Wave 8), and host facts reach DMLS through
    Phase 5's `.facts.json` artifact, whose plan tasks already require the
    composition/DMLS agreement tests.
  - *Alternatives:* none; `is_file_type` does not exist yet, and a
    thread-local test function cannot reach a DMLS server.
  - *Why:* the plan sequences those features after Phase 3; the fact slot they
    register into is delivered here.
  - *Downsides:* criterion 11 is complete only after Phases 4 and 5.
- **P3-D8. The shadowed-`fm` case is tested under Claudine.**
  - *Decision:* `claudine/lib/src/composition/lifecycle/tests/call_check.rs`
    covers it at a sequence site, where `seq` is a host global shadowing a
    document `seq`.
  - *Alternatives:* a darkmatter-only registry with an invented global.
  - *Why:* `md`'s events declare only reserved namespaces, which cannot be
    document properties; Claudine is the real host that shadows one. In a
    plain document event `seq` is not a host global, so it is the property.
  - *Downsides:* none.
- **P3-D9. "Event re-entry" means a fresh, identical walk per event.**
  - *Decision:* every event's walk starts from the declarations alone, and
    re-reading a stack recomputes identical results; both are asserted.
  - *Why:* static analysis has no run history; a re-entered event (retry,
    loop pass) is analyzed as its own stack.
  - *Downsides:* none.

### Requirement-to-test mapping

| Behavior | Boundary / tier | Test |
|---|---|---|
| true-implies narrows true side only; true-iff both; no handler runs (11, 9) | library public API, F1, L1 | `darkmatter/lib/tests/l1/flow_sensitive_narrowing.rs::registered_facts::a_true_implies_fact_narrows_only_its_true_side` |
| no fact → no narrowing | library, L1 | `::registered_facts::a_predicate_without_a_fact_narrows_nothing` |
| true-implies `file` (`unknown` → `file`, category 4 → 1), false-implies non-null | library, L1 | `::registered_facts::true_implies_and_false_implies_facts_use_their_type` |
| invalid fact registrations rejected (non-boolean, unknown parameter, union, non-family true-iff) | library, L1 | `::registered_facts::registration_rejects_invalid_facts` |
| shell template hole narrowed by a registered fact (11) | library, L1 | `::registered_facts::a_registered_fact_narrows_a_shell_template_hole` |
| recursive `Ref` arm classified by its definition | library + schema files on disk, L1 | `::recursive_arms::a_recursive_arm_is_classified_by_its_definition` |
| short-circuit, negation, compound, calls-as-subjects (10, 33) | library, L1 | `::compound_guards::short_circuit_operands_and_ternaries_see_only_justified_facts` |
| impossible branch checked with incoming type (33) | library, L1 | `::compound_guards::an_impossible_branch_is_checked_with_the_incoming_type` |
| `::block when` narrows, nested blocks compose (10) | `BodyGuards` (composition's and DMLS's scope), L1 | `::body_guards::a_block_guard_narrows_its_body` |
| composition: guarded calls compose; impossible ternary/block rejects (9, 33) | `Markdown::compose_with`, temp dir, L1 | `::body_guards::composition_reads_the_same_narrowing` |
| no stored value changes; authored bytes unchanged (12) | analysis + composition, L1 | `::body_guards::narrowing_never_changes_a_stored_value` |
| guard → set → later call; events never share facts; recompute identical (34) | `lifecycle_call_checks`, md registry, L1 | `::lifecycle_fact_lifetime::a_set_ends_the_guard_and_events_never_share_facts` |
| ancestor/alias write ends fact; sibling does not; `fm` alias (34) | same | `::lifecycle_fact_lifetime::overlapping_writes_end_facts_and_siblings_do_not` |
| lifecycle rejection reads the guarded site | same | `::lifecycle_fact_lifetime::the_lifecycle_rejection_reads_each_site` |
| editor = library at each body site, strict severities (9, 10, 33, parity checkpoint) | real LSP session, L1 | `darkmatter/dmls/tests/l1/expression_call_diagnostics.rs::narrowing::the_editor_shows_exactly_the_library_categories_at_each_guarded_site` |
| editor lifecycle guard ends at set and event (34) | real LSP session, L1 | `::narrowing::a_lifecycle_guard_ends_at_a_write_and_its_event` |
| Claudine registry: set ends fact; shadowing host global unaffected (34) | Claudine lib, L1 | `claudine/lib/src/composition/lifecycle/tests/call_check.rs::a_guard_narrows_until_a_write_and_never_reaches_a_shadowing_host_global` |
| built-in facts are on registrations | library unit | `darkmatter/lib/src/markdown/compose/expression/scope/tests.rs::the_built_in_narrowing_facts_are_registered_on_one_argument_predicates` |

Updated existing tests: both `type_scope_ownership_guard.rs` files in
darkmatter (lib, DMLS) and Claudine's (retired `PREDICATE_FACTS`/
`PredicateFact`; `NarrowingFact`/`FactKind` owned by `registration.rs`; the
lifecycle allowlist entries renamed to `lifecycle_call_checks` and
`branch_scopes`).

The Input Robustness Matrix does not apply: no file format or configuration
reader changed (the lifecycle source map gained location bookkeeping only).

### Gates

- **Scope calculator:** `git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`
  → `packages: ["claudine", "darkmatter", "dmls", "test-toolkit"]`,
  `change_class: package`. Gates were run.
- **`darkmatter/` `just test`** (library, CLI, DMLS, zed CLI): 11985 run,
  11984 passed, 1 failed:
  `dmls::l1 registry_preparation::a_held_preparation_leaves_edits_cancellation_and_other_requests_running`.
  Rerun alone three times: passed each time. Recorded as
  **load-sensitive**; it exercises request concurrency in the LSP server,
  not anything this phase changed.
- **`darkmatter/` `just lint`:** passed.
- **`claudine/` `just test`:** 11448 passed (386 slow), 9 skipped.
- **`claudine/` `just lint`:** passed (the `__eh_frame` linker note on the
  `claudine` binary is pre-existing and not an error).
- **`just check-tier-coverage claudine`** and **`darkmatter`:** 0 stranded.
- **Workspace compile:** `cargo check --workspace` clean; the other
  darkmatter dependents do not use the changed lifecycle or fact APIs.
- **Targeted runs during the phase:** the 393 existing narrowing/call-check
  tests (`expression_type_scope|design_time_call_check|scope::tests|variable_type_projection|compose_call_precheck|scope_site_conformance|call_binding|function_binding_registry`)
  passed after the fact-model change; the 14 new library tests, the 2 new
  DMLS tests, the Claudine `call_check` module, and all three ownership
  guards pass.
- **Cross-OS:** not run. The changes are type computation over in-memory
  values plus temp-dir fixtures in existing portable patterns; no path
  comparison or OS-specific code was touched.

## Phase 4

Phase 4 delivers the file predicates and guarded file arguments (spec §3, §4;
criteria 13–16, 18, 35, and criterion 11's `is_file_type` leg).
`biscuit-file`'s `FileReference::new` is the one structural validator;
Darkmatter adds only the `file://` input adapter in front of it.

### What changed

- **`biscuit-file` structural checks** (`file_reference/parse.rs`), the four
  Spike S1 named: NUL rejection, a URL host check (literal authority parsed by
  `url::Url`; a `{{NAME}}` authority accepted unread), `%` on a URL rejected,
  and empty or rooted vault payloads rejected. All are `InvalidSyntax`.
  `FileReference::new`'s docs now state it is the passive validator.
- **`file://` adapter** (`resolve_ctx.rs::normalize_path_arg`, and its copy in
  `effects/verbs.rs`): strips one `file://` prefix and no longer collapses
  `/` runs (Phase 1 D5), so `file:////server/share/x` keeps its UNC meaning.
  `is_well_formed_reference(raw)` = adapter + `FileReference::new(..).is_ok()`.
- **`is_file_type(value: unknown | null): boolean`** — catalog entry
  (Filesystem, four inline examples), pure handler in `predicates.rs`, and
  `.narrowing("value", TrueImplies, Some("file"))` registered together.
  Lifecycle verb regenerated (`regen-lifecycle-schema`), name added to both
  `md-lifecycle.yaml` patterns, and `-short`/`-long@darkmatter:lifecycle`
  references added to `claudine/schemas/action.yaml` (category and both
  bundles) and `claudine/schemas/lifecycle.yaml` (loop keys). Function table
  regenerated (`regen-expr-doc`).
- **`file_exists`** parses the adapted text once: malformed → `false` before
  any lookup; `Url` kind → remote fetch (unchanged outcomes); otherwise local
  resolution (unchanged outcomes and errors). Catalog description updated.
- **Always-false analysis**: `RegistrationBuilder::false_for_malformed_reference(parameter)`
  (validated: boolean result, one non-rest position in every overload;
  `RegistrationError::InvalidReferenceRule`), `FunctionRegistration::malformed_reference_parameter`,
  `FunctionBinding::false_for_malformed_reference` on `file_exists` only.
  `CallCheck::always_false` / `always_false_finding()` (`AlwaysFalseFinding`)
  set for a non-rejected call whose literal text argument at that position is
  malformed. Code `dm.expression.always_false_call`
  (`expression::ALWAYS_FALSE_CALL_CODE`, defined once in the library).
  `check_prepared_calls` keeps always-false checks.
- **Composition** (`call_precheck.rs`): the existing walk records each
  always-false check (frontmatter leaves, body, `::block`/`::file` `when=`)
  and `check_document` returns them as coded `ComposeWarning`s (file and
  line in the Prose message; identity per expression), added to the report
  in `pipeline/mod.rs`.
- **DMLS** (`call_check.rs::check_diagnostics`, `codes.rs`): every surface
  (body, frontmatter, conditions, md lifecycle stacks) shows the warning
  under both strictness settings, ranged at the literal.

### Design Decisions

- **P4-D1. A denied or failed remote fetch stays an error.**
  - *Decision:* `file_exists` keeps its existing remote outcomes: a fetched
    URL is `true`; a denied, failed, or local-only fetch is an error.
  - *Alternatives:* return `false` for a denied/failed fetch, as spec §3's
    parenthetical says.
  - *Why:* spec §3's rule is "valid references keep their existing
    outcomes". The parenthetical describes the outcomes wrongly: the
    existing code and its tests (`file_exists_errors_for_policy_denied_remote`,
    "a denial says nothing about existence") make a denial an error, and the
    catalog documents it. Keeping the rule keeps the intent.
  - *Downsides:* a document that wants `false` for an unreachable host must
    guard the read itself.
- **P4-D2. The always-false rule is registration metadata.**
  - *Decision:* `false_for_malformed_reference(parameter)` on the
    registration; the call check reads it, never a function name. Only
    `file_exists` declares it.
  - *Alternatives:* match `file_exists`/`fileexists` by name; derive it from
    the narrowing fact.
  - *Why:* Phase 3's rule (never key analysis on a name); a host predicate
    (Phase 5) can declare it through the same builder. `is_file_type` does
    not declare it: spec §3 limits the warning to `file_exists`, and
    `is_file_type("https://") => false` is the predicate's own documented
    example.
  - *Downsides:* Phase 5's `.facts.json` must carry this metadata too.
- **P4-D3. The new code is defined once, in the library.**
  - *Decision:* `ALWAYS_FALSE_CALL_CODE` in `expression/call_check.rs`;
    DMLS's `code::EXPRESSION_ALWAYS_FALSE_CALL` is that constant.
  - *Alternatives:* define it in DMLS like the other call codes.
  - *Why:* Phase 1's default (a new `dm.expression.*` code in the shared
    library diagnostics) and Phase 6's direction (codes move to the library).
  - *Downsides:* until Phase 6 the call codes live in two places.
- **P4-D4. Composition reports the warning on the precheck surfaces; its
  lifecycle-action leg rides Phase 6's warning channel.**
  - *Decision:* composition warns for frontmatter, body, and `when=`
    conditions now. DMLS warns on every surface, md lifecycle stacks
    included. Composition's lifecycle actions report through
    `lifecycle_call_checks` when Phase 6 adds composition warnings for that
    list (the plan's Phase 6 "CLI severities" task now says so).
  - *Alternatives:* add a lifecycle warning channel to `md compose` and
    Claudine's `prepare.rs` now.
  - *Why:* lifecycle rejection today returns only the first category 5;
    Phase 6 builds the warning path for that same list (category 3 there
    too). Building it twice would make two channels.
  - *Downsides:* until Phase 6, a malformed literal in an md lifecycle
    action warns in the editor but not at `md compose`.
- **P4-D5. No alias for `is_file_type`.**
  - *Decision:* no `isfiletype` alias (and no `is_file`).
  - *Alternatives:* add `isfiletype` like the older functions.
  - *Why:* the spec lists no alias; the alias inventory test pins the 75
    aliases the registry held before the migration.
  - *Downsides:* `isFileType` is not accepted.
- **P4-D6. A malformed reference fails a `file` parameter at binding.**
  - *Decision:* accepted as a consequence: the `file` type's format check is
    `FileReference::new`, so text such as `"http://[::1"` or
    `"https://exa mple.com"` passed to `link`/`code_link` is now a binding
    type error ("fails file") rather than an invalid-file-path error.
  - *Alternatives:* exempt the binder from the new checks.
  - *Why:* one grammar; a malformed reference is not a file.
  - *Downsides:* the error text for those inputs changed
    (`fs_tests::code_link_errors_name_code_link_with_the_link_cause` updated).
- **P4-D7. A "file value" is text.**
  - *Decision:* `is_file_type` is `true` only for strings that pass the
    validator; runtime has no separate file value.
  - *Why:* `file`-typed properties are strings at runtime.
  - *Downsides:* none.
- **P4-D8. How "failing stubs" prove no access.**
  - *Decision:* `is_file_type` is evaluated with a lookup that has no
    resolution context (nothing reachable) and unset template variables;
    effect-engine and network counters are unchanged. `file_exists` runs
    against a deny-all remote runtime (a fetch would error; the mock server
    records no request) and a working directory that is a regular file (a
    probe would error; armed on Unix only, where probing under a file is an
    I/O error, while Windows reports NotFound).
  - *Alternatives:* add filesystem counters to `biscuit-file`.
  - *Why:* each tripwire turns an access into a visible error, so a `false`
    result proves the access never happened, without new instrumentation.
  - *Downsides:* the filesystem tripwire's arming check is Unix-only; the
    outcome assertion runs everywhere.

### Requirement-to-test mapping

| Behavior | Boundary / tier | Test |
|---|---|---|
| structural corpus: every kind well-formed, malformed set rejected, identical on every OS, no context (13, 35) | `biscuit-file` public API, L1 | `biscuit-file/lib/tests/l1/structural_validation.rs::every_supported_kind_is_well_formed_on_every_host`, `::malformed_references_are_rejected_by_construction`, `::the_new_structural_checks_are_syntax_errors`, `::a_url_error_names_the_missing_host` |
| `file://` adapter strips only its prefix; UNC kept | Darkmatter unit | `resolve_ctx::tests::the_file_scheme_adapter_strips_only_its_prefix` |
| `is_file_type` corpus, non-text false, no context, no engine/network (13, 35) | Darkmatter public evaluator, L1 | `darkmatter/lib/tests/l1/file_predicates.rs::is_file_type::it_is_true_exactly_for_well_formed_reference_text_without_a_context`, `::is_file_type::it_never_changes_its_argument` |
| `file_exists`/`fileexists` parity; malformed false; existing/missing/`file://` outcomes kept; null false (14, 35) | public evaluator + temp dir, L1 | `::file_exists::malformed_text_is_false_and_well_formed_references_keep_their_outcomes` |
| valid remote reference without runtime still errors (14) | public evaluator, L1 | `::file_exists::a_valid_remote_reference_without_a_runtime_is_still_an_error` |
| malformed false before any access, failing remote + filesystem stubs (14) | handler dispatch, unit | `functions::fs_tests::...::malformed_references_are_false_before_any_access` |
| always-false only for malformed literals; alias; not for `string` var or `is_file_type` (15, 35) | call check, L1 | `::always_false::only_a_malformed_literal_reference_is_always_false` |
| rule is registration metadata; validated; handler never runs | registration + call check, L1 | `::always_false::the_rule_is_registered_and_validated` |
| guarded `frontmatter` category 1, unguarded 4, false side nothing (16) | call check, L1 | `::guarded_file_arguments::an_is_file_type_guard_makes_the_file_argument_category_1` |
| `is_file_type` narrows a shell template hole (11) | `BodyGuards` + `project_hole`, L1 | `::guarded_file_arguments::an_is_file_type_guard_narrows_a_shell_template_hole` |
| composition warnings with lines, alias, none for `string` var, guarded read composes, no network (35, 16) | `Markdown::compose_with`, temp dir, L1 | `::composition::composition_warns_for_each_always_false_call_and_composes_the_guarded_read` |
| `md compose` end to end: predicates, guarded read, one warning with file:line on stderr | CLI binary (`CliProcessFixture`), L1 | `darkmatter/cli/tests/l1/compose_file_predicates.rs::compose_answers_file_predicates_and_warns_for_an_always_false_call` |
| editor warning on body, `::block` condition, lifecycle action, alias; none for `string` var; both settings (15, 35) | real LSP session, L1 | `darkmatter/dmls/tests/l1/expression_call_diagnostics.rs::file_predicates::a_malformed_literal_reference_warns_and_a_string_variable_does_not` |
| editor: only the unguarded call warns under strict (16) | real LSP session, L1 | `::file_predicates::under_strict_only_the_unguarded_file_argument_warns` |
| no `file_exists` setting in DMLS config (18) | DMLS config model, L1 | `::file_predicates::the_configuration_has_no_file_exists_setting` |
| `is_file_type` row in registry / generated verb / doc table | L1 + unit | `function_binding_registry::every_registered_function_has_a_row` (new row), `lifecycle_schema_generation::the_generated_verbs_are_every_catalog_function_and_side_effect_but_set` (129), `catalog::tests::narrative_doc_function_table_matches_catalog`, `claudine_schemas::generated_verb_keys_equal_the_darkmatter_catalogs` |

Updated existing tests: `fs_tests::absolute_and_file_exists_resolve_relative_to_base_dir`
(`"\0bad"` is now `false`), `fs_tests::code_link_errors_name_code_link_with_the_link_cause`
(P4-D6), `resolve_ctx::tests` (adapter), `function_binding_registry` (row),
`lifecycle_schema_generation` (verb count 128 → 129).

The Input Robustness Matrix does not apply: the reference grammar is a text
grammar, and no file-format or configuration reader changed.

Further updated by the full-suite gates (expected inventory changes from the
new function): `scope/tests.rs::the_built_in_narrowing_facts_are_registered_on_one_argument_predicates`
(adds `is_file_type`), the six `recursive_schema_golden` digests (re-blessed
with `DARKMATTER_BLESS_RECURSIVE_GOLDEN=1`; each embedding grows by the
`is_file_type` verb types, +390 bytes), and Claudine's
`context_expressions_function_order_snapshot` (adds `is_file_type(value)`).

### Gates

- **Scope calculator:** `git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`
  → `packages: ["biscuit-file", "claudine", "darkmatter", "darkmatter-cli", "dmls", "test-toolkit", "claudine-cli"]`,
  `change_class: package` (the tree still holds Phase 3's uncommitted
  changes). Gates were run.
- **`biscuit-file/` `just test`:** 1151 passed. **`just lint`:** passed.
- **`darkmatter/` `just test`:** 12199 passed (10 skipped) after the two
  expected inventory updates above. **`just lint`:** passed. The first lint
  run caught a stranded `#[cfg(feature = "effects-instrumentation")]` from
  registering `file_predicates` in `tests/l1/main.rs` (it had moved off
  `flow_sensitive_narrowing`); both modules are now gated.
- **`claudine/` `just test`:** the first two fail-fast runs each ended on
  two 30 s timeouts in different CLI tests
  (`compose_schema_cli::compose_unresolved_caller_file_fails_before_initialize_when_non_interactive`,
  `compose_schema_cli::compose_late_caller_file_verdict_names_the_launch_directory`,
  then `agent_text_is_data::malformed_structured_agent_values_are_refused_not_saved_as_text`,
  `authored_text_rendering::header_document_label_and_model_render_exactly_as_authored`)
  while the host load average was 55–130 from other sessions. The first
  pair passed alone. Recorded as **load-sensitive**. A
  `--no-fail-fast` run then completed: 11447 passed, 1 failed (the expected
  snapshot above), 9 skipped; the snapshot test and its module pass after
  the update. **`just lint`:** passed (the `__eh_frame` linker note is
  pre-existing).
- **`just check-tier-coverage`** for `claudine`, `darkmatter`, and
  `biscuit-file`: 0 stranded.
- **Dependents of `biscuit-file`** (the only package whose shared behavior
  changed for other crates): no test in another package passes a newly
  rejected form (NUL, a host-less URL, `%` + URL, an empty or rooted vault
  path) to `FileReference` (searched). The file-reference tests of
  `content-policy` and `biscuit-terminal` (529) and of `tree-hugger` and
  `messenger` (100) pass.
- **Cross-OS:** not run. The new checks are lexical and host-independent
  (the corpus has no `cfg`). The one OS-sensitive assertion, the filesystem
  tripwire's arming check, is `cfg(unix)` by design (P4-D8). The adapter
  change preserves separators, so it can only keep UNC paths intact on
  Windows.

## Phase 5

Phase 5 delivers one function contract for host extensions (spec §5;
criteria 17, 37): host functions registered through the built-ins'
registration API, the generated `.facts.json` companion artifact, its DMLS
reader, and the removal of Claudine's independent expression scans.

### What changed

- **Host functions** (`darkmatter/lib/src/markdown/compose/expression/host/`):
  `HostFunctionCatalog::load` reads a host's signature catalog with the
  built-ins' strict loader and records its version (BLAKE3 hex of the raw
  bytes, `biscuit-hash`). `HostFunctions::builder(catalog).function(name,
  aliases, configure, handler).build()` compiles every registration with
  `FunctionRegistration::builder_from_types` + `build_with` (the built-ins'
  API) and rejects, before any evaluation: duplicate canonical names or
  aliases, a collision with a built-in name or alias, a catalog function
  without a registration and the reverse, a lazy host function, and every
  existing `RegistrationError` (a fact on a nonexistent parameter, a
  narrowing fact on a non-boolean function, …).
- **Two ways to make them visible**: `install_host_functions` (process-wide,
  additive, collision-checked; the runtime host's path) and
  `HostFunctions::enter` (a thread-scoped, `!Send` guard; the editor's
  per-document path). `functions/mod.rs`'s lookups (`inspect_registration`,
  `is_dispatchable`, `call_function`, `is_context_function`,
  `lazy_function`) consult test functions → entered sets → installed sets →
  built-ins, so host calls bind, type-check, and narrow exactly as built-ins.
- **Facts artifact**: `HostFunctions::facts_artifact()` writes
  `<stem>.facts.json` (`facts_artifact_path`): `format_version` 1,
  `catalog_hash`, and per function `aliases`, `facts` (parameter, position,
  kind, type), `lazy`, `conversion_recovery`, `source_exclusions`, and
  `malformed_reference` (Phase 4's always-false rule). Keys sorted at every
  level; deterministic bytes.
- **Reader**: `HostFunctions::analysis_only(catalog, FactsArtifact)` parses
  with a duplicate-key-rejecting visitor, rejects trailing content and
  unknown fields, walks the value by hand (absent ≠ null ≠ wrong type), and
  rebuilds each registration through the same builder. A function whose
  metadata cannot be trusted is *incomplete*: dispatchable, but it has no
  registration for analysis (no category, no narrowing), and
  `incomplete_host_calls(expr)` lists its calls
  (`dm.expression.incomplete_analysis`, `host::INCOMPLETE_ANALYSIS_CODE`).
- **Discovery**: `schemas::catalog::host_function_catalogs(context)` finds
  function catalogs (`kind: schema` + `categories:` + `types:`) in the same
  schema roots as host global catalogs and reads the artifact beside each by
  name; results are cached by path, catalog hash, and artifact bytes (the
  loader leaks).
- **DMLS**: enters each document's host sets around diagnostics, reports
  incomplete calls as warnings on every call-diagnostic surface (including
  guarded lifecycle stacks), and reports a function catalog that fails to
  load as `dm.schema.host_catalog`.
- **Claudine scans moved** onto one new Darkmatter API,
  `expression::static_access_reads(&Expr) -> Vec<StaticAccessRead>`
  (`absence.rs`: variable, typed `AccessStep`s, `static_path()`):
  `looping/config.rs` (`collect_identifiers`,
  `collect_value_template_identifiers`, `is_reserved_identifier` deleted;
  `reserved.rs::EXPRESSION_RESERVED_ROOTS` deleted),
  `lifecycle/bindings.rs::removed_loop_read`, `bindings/rename.rs::read`, and
  `sequence/preflight/shape.rs` (`target_identity_path_in_expr`,
  `static_member_path`, `member_root` deleted). What remains in Claudine only
  filters the library's reads by Claudine's own names.
  `claudine/lib/tests/l1/type_scope_ownership_guard.rs` now fails on a new
  `Expr::Variable(`/`SpannedExprKind::Variable(` match in `claudine/lib/src`
  without a reviewed entry (four entries: `dispatch/matcher.rs`,
  `dispatch/template.rs`, `prewarm/classify.rs` ×2).
- **Claudine host functions**: none exist; see P5-D1. A guard test keeps
  `claudine/schemas/` free of function catalogs and artifacts until one is
  wired.
- **Docs**: new `darkmatter/docs/topics/host-functions.md`, linked from
  `darkmatter-expressions.md`; `dmls/docs/diagnostics.md` lists the new code.

### Design Decisions

- **P5-D1. Claudine ships no host function catalog or artifact.**
  - *Decision:* Claudine registers no host expression function, so it ships
    no `functions.yaml`/`.facts.json` and installs nothing. The guard
    `claudine/lib/tests/l1/host_catalog.rs::claudine_ships_no_host_function_catalog_and_no_facts_artifact`
    fails if a function catalog or an artifact appears in `claudine/schemas/`
    without its registration and freshness test.
  - *Alternatives:* invent a Claudine function to exercise the path; ship an
    empty catalog and artifact.
  - *Why:* the Phase 1 audit found no Claudine expression function, and the
    spec asks Claudine for none (§5 describes how a host *may* extend the
    language). The registration, generator, freshness, and reader paths are
    proven end to end by the test host (`is_project_file`, `project_label`)
    in Darkmatter and DMLS. An invented or empty catalog would be discovered
    by DMLS for every Claudine document and add nothing.
  - *Downsides:* the plan's "emit the artifact in the Claudine build" has no
    Claudine artifact to emit until Claudine gains a function; the guard
    makes that addition fail until it is wired.
- **P5-D2. Runtime install is process-wide; editor analysis is thread-scoped.**
  - *Decision:* `install_host_functions` for a runtime host (group workers
    and other threads see it), `HostFunctions::enter` for DMLS, which
    analyzes documents from different schema roots in one process.
  - *Alternatives:* thread a host set through every analysis and evaluation
    API; one mechanism for both.
  - *Why:* every consumer already resolves functions by name through
    `functions/mod.rs`; a lookup layer there reaches all of them without
    changing any signature, as the test-function layer already does.
  - *Downsides:* two installation paths; a set entered on one thread is not
    visible on another (the guard is `!Send`, so it cannot cross an await).
- **P5-D3. Artifact encoding details** (subagent sub-decisions adopted):
  host aliases are a registration fact recorded in the artifact, not in the
  catalog; the non-null fact type is the string `"<non-null>"`
  (`NON_NULL_FACT_TYPE`); `facts` is a list but more than one element makes
  the function incomplete (a registration holds one fact; no silent
  last-wins); `conversion_recovery`, `malformed_reference`, and `facts`
  distinguish absent (incomplete) from `null` (none); unknown fields are
  rejected (top level → whole artifact, in an entry → that function), so an
  additive change needs a `format_version` bump; an artifact entry for a
  function missing from the catalog, or an artifact alias colliding with a
  built-in, invalidates the whole artifact.
  - *Alternatives:* `null` for non-null (conflicts with "null is an error"),
    ignoring unknown fields.
  - *Why:* every load-bearing shape has exactly one outcome, per the plan's
    matrix.
  - *Downsides:* stricter than a forward-compatible reader.
- **P5-D4. An incomplete function dispatches by canonical name only.**
  - *Decision:* aliases are not salvaged from a bad artifact entry.
  - *Why:* the aliases are part of the untrusted metadata.
  - *Downsides:* calling an incomplete function through an alias reports an
    unknown function rather than incomplete analysis.
- **P5-D5. Among function catalogs, an earlier schema root wins a name.**
  - *Decision:* same file-name shadowing as host global catalogs; DMLS
    enters sets so the earlier root wins a function name. Duplicate names
    across two different catalog files are not reported.
  - *Downsides:* a duplicate across catalogs is silent in the editor.
- **P5-D6. One read inventory API for Claudine, on the unspanned `Expr`.**
  - *Decision:* `static_access_reads` returns each read with typed selector
    steps; consumers stop where they need to. Parentheses are looked through
    everywhere, so `(fm)['_loop_count']` is now `RemovedGrammar` (it passed
    silently before; the documented rule says any read fails). Loop control
    variables exclude `is_reserved_namespace` roots (fm, ctx, env, current)
    rather than the old local list (true/false/fm/env), so a bare `ctx.*`
    read no longer seeds a frontmatter key named `ctx`.
  - *Alternatives:* extend `static_variable_reads` (spanned, absence-aware);
    a joined-string path (loses `ctx.agent[0]` and `fm['a.b']`).
  - *Why:* D3; walkers' surfaces carry `Expr`. A test asserts the two read
    inventories visit the same variables in the same order.
  - *Downsides:* two read inventories exist in the library; the two widened
    edge cases above.
- **P5-D7. `md compose` gets no host-function end-to-end test.**
  - *Decision:* composition is proven in-process with an installed test
    host; the `md` binary has no host functions to register.
  - *Why:* a host function exists only in a host's process.
  - *Downsides:* none; the grammar did not change.
- **Inventory correction (Phase 1 surface table):**
  `prewarm/classify.rs::frontmatter_reads` is a hand walk over `fm` reads,
  not only `static_variable_reads` roots. It stays out of scope per D3 and
  carries a reviewed guard entry naming it as a candidate for
  `static_access_reads`.

### Requirement-to-test mapping

Lib: `darkmatter/lib/tests/l1/host_functions.rs`; DMLS:
`darkmatter/dmls/tests/l1/host_function_diagnostics.rs`.

| Behavior | Boundary / tier | Test |
|---|---|---|
| registration rejects duplicates, alias/built-in collisions, parity gaps both ways, fact on nonexistent parameter, non-boolean narrowing, lazy, installed collision — before evaluation (37) | library public API, L1 | `host_functions::registration_rejects_collisions_parity_gaps_and_invalid_facts_before_evaluation` |
| generator output equals the committed artifact byte for byte; deterministic | library + fixture file, L1 | `host_functions::the_committed_facts_artifact_is_fresh_and_deterministic` |
| Input Robustness Matrix: control row + 50 one-edit cells (absent/null/wrong-type per load-bearing field, element and every-element, `facts: []` valid, `functions: {}`, duplicate key, trailing content, hash mismatch, unknown version, missing file, extra function) → incomplete, reason distinguishes absent/null, no category, no narrowing, 0 handler entries | library reader, L1 | `host_functions::the_facts_artifact_reader_walks_the_input_robustness_matrix` |
| strict parser, sorted keys, artifact path | library unit | `host::artifact::tests::*` |
| installed host: guarded `frontmatter` cat 1, unguarded cat 4, `project_label(42)` cat 5 and rejected by `compose_with`, real note composes, counters 0 after analysis and only evaluated calls enter (17, 37) | compatibility API + `Markdown::compose_with`, temp dir, L1 | `host_functions::installed_host_functions_get_the_built_in_checks_and_run_only_when_evaluated` |
| function catalog never a global catalog, trigger, or document schema | library, temp schema root, L1 | `host_functions::a_function_catalog_in_a_schema_root_is_never_a_document_schema` |
| editor control (strict): guarded silent, unguarded uncertain, `project_label(42)` incompatible, no incomplete (17, 37) | real LSP session, L1 | `host_function_diagnostics::the_host_catalog_and_its_artifact_check_host_calls_like_built_ins` |
| editor: missing artifact / hash mismatch / artifact lacking a function → incomplete analysis, no category or narrowing (37) | real LSP session, L1 | `::a_missing_host_facts_artifact_reports_incomplete_analysis`, `::a_host_facts_artifact_for_another_catalog_version_reports_incomplete_analysis`, `::a_host_facts_artifact_lacking_a_function_reports_it_incomplete` |
| editor: function catalog load failure reported | real LSP session, L1 | `::a_host_function_catalog_that_fails_to_load_is_reported` |
| `static_access_reads` paths, positions, parity with absence reads | library unit | `absence::tests::access_reads_keep_each_static_path…`, `::access_reads_visit_every_position_in_source_order`, `::access_reads_and_absence_reads_visit_the_same_variables` |
| loop control variables over every read shape (written first, passed on the old code) | Claudine lib unit | `looping/config/tests.rs::extract_control_variables_visits_every_read_shape` (+ 8 existing `extract_control_variables_*`) |
| retired loop read in every position, incl. `(fm)` | Claudine lib unit | `lifecycle/bindings/tests.rs::a_retired_loop_read_is_found_in_every_read_position` |
| retired sequence globals: static prefix of the first read | Claudine lib unit | `bindings/rename/tests.rs::the_removed_path_is_the_static_prefix_of_the_first_read` |
| target identity paths in every read position | Claudine lib unit | `sequence/preflight/tests.rs::target_identity_paths_are_found_in_every_read_position` |
| no new hand walk of variable reads in Claudine | source guard, L1 | `claudine/lib/tests/l1/type_scope_ownership_guard.rs` (new rule + self-test) |
| Claudine ships no unwired function catalog/artifact | repository files, L1 | `claudine/lib/tests/l1/host_catalog.rs::claudine_ships_no_host_function_catalog_and_no_facts_artifact` |

The reader was mutation-checked: disabling the hash check makes the matrix
test fail. The reader grep (`serde(default)`, conflated `Option`,
`filter_map(.. as_str())`, `unwrap_or_default()`, `.ok()`) finds nothing on
a load-bearing field.

### Gates

- **Scope calculator:** `git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`
  → `packages: ["claudine", "darkmatter", "dmls", "test-toolkit"]`,
  `change_class: package`. Gates were run.
- **`darkmatter/` `just test`** (library, CLI, DMLS, zed CLI): 12216 passed,
  10 skipped. **`just lint`:** passed.
- **`claudine/` `just test`:** 11454 passed (51 slow), 9 skipped.
  **`just lint`:** passed.
- **`just check-tier-coverage`** for `claudine` and `darkmatter`: 0 stranded.
- **Dependents:** `cargo check --workspace` is clean. The Darkmatter changes
  add APIs and a lookup layer that is empty unless a host installs or enters
  a set, so the other dependents' behavior is unchanged. Claudine (the
  dependent that uses the changed read APIs) passed its full suite.
- **Cross-OS:** `just cross-check dmls --os windows` passed (1382 tests),
  covering schema-root discovery of function catalogs and their artifacts.
  The fixtures are byte-compared, and `.gitattributes` (`* text=auto eol=lf`)
  keeps them LF on Windows.
- **Targeted runs during the phase** (subagents, scoped):
  `cargo nextest run -p darkmatter --features effects-instrumentation --test l1 -E 'test(/host_functions|function_binding_registry|flow_sensitive_narrowing|file_predicates|design_time_call_check|type_scope_ownership_guard/)'`
  (229 passed); the lib unit filter `functions::|call_binding|catalog::|scope::|host::|absence`
  (368 + 17 passed); DMLS `expression_call_diagnostics|host|type_scope_ownership_guard|call_check`
  (38 passed); Claudine lib `looping|loop_config|bindings|rename|binding_checks|removed|shape|target_identity`
  (411 passed) and `preflight|looping|prepare|lifecycle::bindings|prewarm`
  (699 passed); Claudine CLI `compose_removed_grammar|loop_gate_ambient|sequence_errors_cli|…`
  (93 passed). No failures in the full runs; nothing was recorded as
  load-sensitive.

## Phase 6

Phase 6 delivers the shared diagnostics model (spec §6; criteria 19–22, 36):
one library-owned record for every call finding, category 3 warnings at
composition (body, frontmatter, conditions, and lifecycle stacks), the
type-based absence rule that replaces the function-name list, and privacy.

### What changed

- **Shared record** (`darkmatter/lib/src/markdown/compose/expression/diagnostics.rs`,
  new): `INCOMPATIBLE_CALL_CODE` and `UNCERTAIN_CALL_CODE` moved here from
  DMLS unchanged; `Severity` (`error`/`warning`/`info`); `call_severity`, the
  one category→severity ladder (moved from `dmls/.../call_check.rs::severity`);
  `ExpressionDiagnostic` (code, severity, message, span relative to the
  analyzed expression, expected/actual type) from `CallCheck::diagnostics(strict)`,
  `IncompleteHostCall::diagnostic()`, and `expression_call_diagnostics`; the
  located `Diagnostic` / `DiagnosticSource` (file, one-based line, one-based
  character column), already in the report shape spec §8 fixes and
  `Serialize`; `one_based_position`.
- **Composition** (`call_precheck.rs`): the precheck records every finding
  `CallCheck::diagnostics(false)` shows (category 3, always-false) plus
  incomplete host calls, and turns each into a `ComposeWarning` whose new
  `diagnostic` field (boxed, P6-D8) is the record located at the authored argument
  (frontmatter through `FrontmatterExpressionLocation::project`, body through
  `BodyOrigin`, conditions by their text after the directive). The old
  always-false-only path is gone; the always-false origin was the `{{`, not
  the expression, and now is the argument (`expression_start`).
  `MarkdownError::call_diagnostic()` gives a rejection's record (first
  category 5 finding) located in its anchored `{{ … }}` or condition.
- **Condition rejections now carry file lines.** The precheck's
  `condition_rejection` used the body-only context and a body-relative
  line (`doc.md` line 2 for a block on file line 7, in the rendered excerpt
  too). It now uses `full_source_context_for_errors()` and
  `frontmatter_line_count() + line`. Runtime condition evaluation errors are
  unchanged (outside this phase's surface).
- **Lifecycle surfaces** (`lifecycle/validate.rs`): `reject_incompatible_calls`
  gained `authored: Option<&str>` (the loaded text) and returns
  `Ok(Vec<ComposeWarning>)`, the surfaces' located warnings;
  `LifecycleSurfaceCalls` gained `text` (the scalar the spans index). `md
  compose` threads them through both paths (plain and `lifecycle::run`,
  `LifecycleCompose.call_warnings`) ahead of composition's warnings; Claudine's
  `prepare.rs` passes `source.original_text` and puts them first in
  `PreparedComposition.warnings` (both preparation paths).
  `Markdown::loaded_text()` is new.
- **DMLS** (`diagnostics/call_check.rs`): now an adapter. `CallDiagnostic` is
  the library's `ExpressionDiagnostic`; `lsp_diagnostic` maps severity and
  puts `{expected_type, actual_type}` in the LSP `data`. `codes.rs` reads both
  call codes from the library.
- **Absence by type** (`absence.rs`, `call_binding/registration.rs`):
  `AbsenceScope::call_argument(function, index, count)` asks
  `FunctionRegistration::handles_absence_at(index, count)`; the
  `ABSENCE_PREDICATES` group and `functions::is_absence_predicate` are
  removed. The evaluator computes it per argument and only when observing.
- **Docs**: `docs/inline/call-check.md` (category 3 warns at composition,
  "One diagnostic, everywhere" table, lifecycle leg), `docs/inline/interpolation.md`
  (the type rule with its three limits), `docs/topics/schemas/parsing/grammar.md`,
  `dmls/docs/diagnostics.md` (`data` fields, the absence rule).

### Design Decisions

- **P6-D1. A `| null` parameter handles absence only if the function's
  result does not declare `null`.**
  - *Decision:* `handles_absence_at` requires, in every overload taking that
    many arguments, an explicit `null` in the parameter **and** a return type
    without an explicit `null`.
  - *Alternatives:* the bare spec rule (any `| null` parameter); change
    `trim`/`lower`'s signatures to non-null.
  - *Why:* criterion 21 requires `is_empty(trim(x))` to keep warning for `x`,
    but `trim` declares `text: string | null`, so the bare rule would silence
    it (and `lower(x)`). A function that returns `null` for `null` passes the
    absence on to its caller rather than handling it; this is the reading
    that meets every literal criterion. Changing signatures would make
    `trim(null)` a binding error.
  - *Downsides:* a host function `f(x: string | null): string | null` does
    not suppress; documented in `interpolation.md`.
- **P6-D2. `is_number(x)` is silent, as its signature dictates.**
  - *Decision:* the plan lists `is_number(x)` among the calls that warn, but
    the catalog declares `is_number(x: unknown | null): boolean`, and the
    spec says `is_number(x)` warns "unless its signature declares `| null`".
    It does, so it is silent. The plain-`unknown` row is covered by a
    test-registered `unknown_arg(v: unknown)`.
  - *Alternatives:* change `is_number` to `unknown` (then `is_number(null)`
    would not bind; a breaking runtime change).
  - *Why:* the spec's own conditional; criterion 36 needs only "a plain
    `unknown` parameter", which no built-in has.
  - *Downsides:* none beyond the plan text.
- **P6-D3. The rule applies uniformly, including `&&`/`||` in the condition
  grammar.** `and`/`or` declare `unknown | null` operands and return
  `boolean`, so `x && x.name` and a frontmatter `known || fm_rhs` (condition
  grammar) no longer warn for the direct operand. The interpolation
  grammar's `known || rhs_var` is a fallback that may return `null`, so it
  still warns. The spec says the rule is "broader than absence predicates by
  design"; three existing tests' rows were updated (`unknown_identifier_warning`,
  DMLS `expression_scopes`, `lsp_session` operand doc), keeping their
  branch-fact intent by moving the covered read inside `lower(…)`.
  - *Downsides:* a misspelled `&&` operand is no longer flagged, exactly as a
    misspelled ternary condition already was not.
- **P6-D4. The located record is computed where the position is known, not
  stored in `CallCheck`.** Warnings are located by the precheck/lifecycle
  walk; a rejection is located by `MarkdownError::call_diagnostic()` from the
  error's existing anchor (`SourceRef::OnDiskSpan`, or the condition's line).
  A construct whose authored bytes differ from its expression (an escape
  inside the `{{ … }}`) is located at the construct's start.
  - *Alternatives:* a position field on `CallCheck`; a new error variant.
  - *Why:* the anchors already exist and are authored-frame; no analysis type
    gains surface state.
  - *Downsides:* an unanchored rejection (no on-disk source) has
    `source: None`.
- **P6-D5. The shared message is the finding's message; the warning row
  keeps a location prefix.** `Diagnostic.message` equals DMLS's message
  (for a literal call it is the finding message, not the binder's error,
  which the rendered rejection still shows). `ComposeWarning.message` is
  `` `fn(…)` at file:line: <finding text> `` for the terminal.
- **P6-D6. `Diagnostic.source.file` is the document path as its source
  names it** (`ComposeSource::File`, `SourceContext::display`), the Phase 1
  ruling's "path as given". Columns are one-based characters; DMLS's
  position encoding is the adapter's concern (the parity tests use ASCII).
- **P6-D8. `ComposeWarning.diagnostic` is `Option<Box<Diagnostic>>`.**
  - *Decision:* the record is boxed.
  - *Alternatives:* inline `Option<Diagnostic>`.
  - *Why:* inline, it made `darkmatter::l1 nested_composition::self_composition_reaches_the_depth_limit`
    overflow its stack (reproduced alone, not load-related): `ComposeWarning`
    is held by value in frames that recur once per `as_markdown` nesting
    level, so its size multiplies by the 16-level depth limit. Boxed, the
    test passes again. Most warnings carry no record.
  - *Downsides:* one allocation per call warning; consumers read it with
    `as_deref()`.
- **P6-D7. Lifecycle warnings lead the composition's warnings** (their check
  runs before composition), in `LifecycleSignal::ALL` then authored order.

### Requirement-to-test mapping

| Behavior | Boundary / tier | Test |
|---|---|---|
| 19: same category 5 call → identical code, severity, message, line/column, expected/actual from composition and DMLS (body and frontmatter) | real LSP session + in-process compose of the same file, L1 | `dmls::l1 expression_call_diagnostics::shared_fields::a_rejected_call_reports_the_editor_fields` |
| 19: category 3 warnings identical in composition and DMLS | same | `::shared_fields::a_category_3_warning_reports_the_editor_fields` |
| 19: rejection record located on body, frontmatter, `::block when`, escaped scalars (outside/inside the construct) | `compose_with`, temp dir, L1 | `darkmatter::l1 shared_diagnostics::a_rejected_call_yields_its_located_diagnostic_on_every_surface` |
| 20 (severity): category 3 composes with located warnings (frontmatter, body) | `compose_with`, L1 | `shared_diagnostics::a_category_3_call_composes_with_a_located_warning` |
| 20: `md compose` exits 0 with one stderr warning per category 3 argument, body and lifecycle `when`, with file:line; category 5 fails with empty stdout | spawned `md`, `CliProcessFixture`, L1 | `darkmatter-cli::l1 compose_call_warnings::*` |
| lifecycle surface warning located inside the event scalar | library API, L1 | `shared_diagnostics::a_lifecycle_surface_reports_the_same_located_warning` |
| Claudine receives the located lifecycle warning | Claudine lib unit | `composition::lifecycle::tests::call_check::a_category_3_lifecycle_call_returns_its_located_warning` |
| transcluded child warning located in the child | `compose_with`, L1 | `shared_diagnostics::a_transcluded_child_warning_is_located_in_the_child` |
| 21: `is_null`/`is_empty`/`file_exists`/alias silent; `is_empty(trim(x))`, `lower(x)`, bare read warn (composition) | `compose_with`, L1 | `darkmatter::l1 unknown_identifier_warning::a_parameter_declaring_null_handles_a_direct_missing_argument` |
| 21/36: `min(x, 5)` not suppressed; its binding error stays visible; arity error case | `compose_with` + static reads, L1 | `unknown_identifier_warning::a_non_null_parameter_handles_nothing_and_its_binding_error_stays_visible` |
| 36: explicit null / plain `unknown` / optional non-null / mixed overloads / all-null overloads, static and runtime | test-registered functions, L1 | `unknown_identifier_warning::only_an_explicit_null_in_every_overload_handles_a_missing_argument` |
| 36: host function with `unknown \| null` suppresses; `project_label(name: string)` does not | installed test host, L1 | `darkmatter::l1 host_functions::a_host_parameter_declaring_null_handles_a_missing_argument` |
| 21: DMLS agrees | real LSP session, L1 | `dmls::l1 expression_scopes::undeclared_property_findings_follow_the_parameter_types` |
| 21: `AbsenceScope` has no function-name list | source guard, L1 | `unknown_identifier_warning::absence_handling_names_no_function` |
| absence rule unit rows | lib unit | `absence::tests::only_arguments_bound_to_a_parameter_declaring_null_are_handled`, `::static_reads_apply_the_suppression_rules_to_every_branch` |
| 22: no frontmatter, env, or file-content value in any record; no handler entered (dead-branch call included); no network attempt | `compose_with` + test hook counters + effects counter, L1 | `shared_diagnostics::diagnostics_hold_no_evaluated_value_and_run_nothing` |
| severity ladder, one-based positions | lib unit | `expression::diagnostics::tests::*` |

Criterion 22's report leg and criterion 19's `report-json` leg are Phase 8's;
the `Diagnostic` record already serializes in the report's field names.
Criterion 20's `--deny all` leg is Phase 7's.

The Input Robustness Matrix does not apply: no file-format reader changed.

### Gates

- **Scope calculator:** `git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`
  → `packages: ["claudine", "darkmatter", "darkmatter-cli", "dmls", "test-toolkit"]`,
  `change_class: package` (the tree still holds Phase 5's uncommitted work
  too). Gates were run.
- **`darkmatter/` `just test`** (library, CLI, DMLS, zed CLI): 12231 passed,
  10 skipped. The first run stopped on
  `nested_composition::self_composition_reaches_the_depth_limit` (stack
  overflow, reproduced alone); fixed by P6-D8, then the full rerun passed.
  **`just lint`:** passed.
- **`claudine/` `just test`:** 11455 passed (33 slow), 9 skipped.
  **`just lint`:** passed.
- **`just check-tier-coverage`** for `claudine` and `darkmatter`: 0 stranded.
- **Dependents:** `cargo check --workspace` is clean. Only `darkmatter-cli`
  and Claudine call `reject_incompatible_calls` or read compose warnings'
  call diagnostics; both suites passed in full. No other package reads
  `ComposeWarning` or the absence advisory.
- **Cross-OS:** not run. The change adds no path, process, or OS-specific
  code; positions count `\n` (a CRLF line ends in `\r`, which does not move a
  column), and fixtures are written by the tests with LF.
- **Targeted runs during the phase:** darkmatter `absence|undeclared|warn|identifier|missing`
  (lib + L1), `shared_diagnostics`, `compose_call_precheck|file_predicates|design_time_call_check|host_functions|flow_sensitive`;
  DMLS `shared_fields|undeclared|expression`; CLI `compose_call_warnings|compose_lifecycle|compose_file_predicates`;
  Claudine `lifecycle::tests::call_check`. Nothing was load-sensitive.

## Phase 7

Phase 7 delivers the CLI policy verbs (spec §7; criteria 20, 23–28, 38):
one registry of `--allow` / `--deny` / `--disable` identifiers that drives
validation, help, and completions on `md compose`, `md render`, default
`md FILE`, `md clean`, and `md schema validate`; the library seam that gives
every recovery a diagnostic code and lets a blocking policy stop a document;
and the removal of every legacy spelling.

### What changed

- **Registry** (`darkmatter/cli/src/args/policy.rs`, new): `POLICY_REGISTRY`
  records each identifier's kind (recovery, host, feature, diagnostics), the
  commands that accept it, the codes a recovery governs, and its summary.
  `PolicyValueParser` (a clap `TypedValueParser`) rejects unknown, empty,
  unsupported, and wrong-verb values at parse time; a recovery's code given
  to `--deny` resolves to that recovery. `ResolvedPolicy::resolve` /
  `resolve_for` reject `--allow all --deny all`, the same recovery allowed
  and denied (any spelling), and `--disable baseline-schema` beside
  `--baseline-schema PATH` / `--schema PATH`, as clap `ArgumentConflict`
  errors (exit 2) before the input is read. `long_help` and `complete` read
  the same registry. `main.rs` exits a `clap::Error` found after parsing as
  clap does.
- **Commands**: `md compose` takes all three verbs (`--deny` also takes
  `style` and the library's `COMPOSE_DIAGNOSTIC_CODES`); `md render` and the
  top-level `md FILE` take `--deny style|all`; `md clean` and `md schema
  validate` take `--disable`. `ComposeAllowFlags` is gone; `run_compose`
  takes the `ResolvedPolicy`. The style strictness travels as a
  `strict_style` argument through `apply_style_frontmatter`,
  `render_terminal_output`, and the HTML/MarkdownPlus artifacts
  (`denied_style_issues` is the one partition: schema issues plus inline HR
  deprecations block, informational notices never).
- **Removed spellings**: `REMOVED_FLAGS` in `main.rs` (was `RENAMED_FLAGS`)
  lists all 13 legacy flags with their replacement; each is a usage error
  naming it, checked before parsing so completion never offers a hidden
  alias.
- **Library seam** (`darkmatter/lib/src/markdown/compose/policy.rs`, new):
  the recovery codes (`dm.reference.missing_hyperlink|missing_image|missing_transclusion`,
  `dm.context.ctx_override`, `dm.transclusion.invalid_frontmatter_assignment`,
  `dm.transclusion.reassigned_frontmatter_property`, `dm.shell.timeout_recovered`),
  `missing_reference_code`, the `COMPOSE_DIAGNOSTIC_CODES` catalog, and
  `BlockingPolicy` (`deny_all`, `deny_code`, `exempt_code`, `blocks`,
  `blocking`). `ComposeOptions::with_blocking_policy` (graph and cache
  identity both encode it). The pipeline's call precheck returns the new
  `MarkdownError::BlockedDiagnostics { blocking, earlier }` before any
  expression of that document runs; `transclusion_failure_is_fatal` treats a
  child's as fatal (`MarkdownError::blocked_diagnostics()` walks the chain).
  The recovery warnings now carry their codes (ctx override, invalid and
  reassigned `set=`, shell timeout, tolerated missing transclusion).
- **Missing-transclusion recovery** (`ComposeOptions::with_allow_missing_transclusions`,
  P7-D4): a `::file` whose target does not exist is removed with a warning in
  the transclusion engine (body and frontmatter prologue/epilogue) and
  skipped by the preflight collector.
- **CLI blocking points** (`commands/compose.rs`, `compose/lifecycle.rs`):
  the lifecycle call warnings are checked before `initialize`;
  `check_references` refuses (exit 2) when a reported reference issue blocks,
  and now prints the warning rows with the view; composition's remaining
  warnings are checked after `compose_with` and before the document is
  written, in both the plain path and the lifecycle host's `prepare`.
- **Stragglers**: library doc comments and messages that named old flags
  (`icmp.rs`'s ICMP warning, the shell-timeout hints, `functions.yaml`'s
  `ping` description, `suffix.rs`, `schemas/clean.rs`, `style/parse.rs`,
  `hr_parser.rs`, `types.rs`, a bench doc), and the internal
  `prompts/_add/add-expressions.md` line.
- **Docs**: new `darkmatter/docs/cli/policy.md` (verbs, per-command table,
  identifiers and codes, precedence, a Mermaid flow of the stop points,
  removed-flag table); `docs/cli/index.md` links it; 20 darkmatter doc pages,
  `darkmatter/cli/README.md`, `renderable/README.md`,
  `renderable/docs/layout-and-style.md`, and
  `content-policy/docs/topics/policy-lifecycle.md` use the new verbs.
- **Skills**: `.claude/skills/darkmatter/{SKILL,compose,frontmatter,remote-cache,schema}.md`
  and `.claude/skills/claudine/research-contracts.md`.

### Design Decisions

- **P7-D1. `style` is a deny-only "diagnostics" identifier, also on
  `md compose`.**
  - *Decision:* `style` is accepted by `--deny` on `md render`, `md FILE`, and
    `md compose`; `--deny all` on those commands implies it.
  - *Alternatives:* render-only (`md compose --output html` would lose the
    strict check the old top-level `md --strict-style compose …` gave it);
    an `--allow style` recovery (there is no recovery contract).
  - *Why:* `md compose --output html|markdown-plus` applies `style:` through
    the same `apply_style_frontmatter`; spec §7 says each command accepts the
    identifiers meaningful to it.
  - *Downsides:* `md compose --deny style` with Markdown output checks
    nothing (Markdown output applies no style).
- **P7-D2. Codes come from the library; the CLI only maps recoveries onto
  them.**
  - *Decision:* every code `--deny` accepts on `md compose` is in
    `darkmatter::markdown::compose::policy::COMPOSE_DIAGNOSTIC_CODES`; each
    registry recovery lists the codes it governs, and a registry test
    asserts they are catalog codes. Uncoded warnings block only under
    `--deny all`.
  - *Alternatives:* a CLI-side code list (two owners of meaning); accept any
    `dm.*` string (unknown codes could not be usage errors).
  - *Why:* spec §7 makes unknown identifiers usage errors and the library
    owns diagnostic meaning.
  - *Downsides:* a new coded warning family must add its code to the catalog
    to be deniable by name.
- **P7-D3. Where blocking is decided.**
  - *Decision:* the library decides only at the call precheck (passive, per
    document, before evaluation); the CLI decides at its own passive points
    (lifecycle call check, reference validation) and once after composing,
    before writing. A blocking reference issue keeps the reference refusal's
    exit status 2; other blocking diagnostics exit 1 through the shared
    `BlockedDiagnostics` error block.
  - *Alternatives:* a check inside `ComposeReport::add_warning` (it cannot
    return an error, and would stop mid-stage); CLI-only checking (call
    warnings would be found only after shells ran, violating criterion 27).
  - *Why:* criterion 27 asks to stop before the next effectful step when
    passive inputs establish the problem, and not to reorder the pipeline.
  - *Downsides:* a warning found mid-composition (an undeclared property, a
    recovered timeout) stops only the write; earlier shells ran. That is the
    spec's "no rollback" leg.
- **P7-D4. `--allow missing-transclusions` now removes the directive in
  composition.**
  - *Decision:* new `ComposeOptions::with_allow_missing_transclusions`,
    applied only to a `NoMatch` resolution failure, in the engine and the
    preflight collector.
  - *Alternatives:* reuse `ignore_invalid_references` (it also skips URL
    transclusions and every other resolution failure); keep the old
    behavior.
  - *Why:* the legacy flag only relaxed reference validation; composition
    then failed on the same missing target, contradicting its documented
    contract ("remove directive") and the spec's "behavior retained" column.
    Found by the new end-to-end test.
  - *Downsides:* none known; the old behavior was a failure.
- **P7-D5. The baseline `ctx` type check is not a ctx-override recovery.**
  - *Decision:* `--allow ctx-override` keeps its old scope (the merge-time
    downgrade). With the default baseline, `ctx: 5` is still a schema
    failure; the tests pass `--disable baseline-schema`, as the existing
    legacy test did.
  - *Alternatives:* make `ctx-override` suppress the baseline's `ctx` type
    problem (a new recovery contract, which the spec forbids adding without
    documentation).
  - *Why:* spec §7: "behavior retained".
  - *Downsides:* `--allow ctx-override` alone does not recover a scalar
    `ctx` under the default baseline; documented in `policy.md` only by
    example.
- **P7-D6. A top-level `--deny` beside a subcommand is the existing
  top-level conflict error (exit 1), not a usage error.**
  - *Decision:* `validate_subcommand_usage` lists `--deny` with `--output`,
    `--show`, `--save`; `md --save --deny …` is a clap conflict.
  - *Alternatives:* make every top-level conflict exit 2 (changes unrelated
    flags' status).
  - *Why:* surgical; every top-level option already behaves this way.
  - *Downsides:* `md --deny style compose` exits 1 rather than 2.
- **P7-D7. No shipped style key is informational today.** Every known
  `style:` leaf is wired, so no document produces a `KnownButInactive`
  notice end to end. Criterion 26's "a notice never blocks" leg is tested at
  the one partition seam (`render::tests::a_denied_style_blocks_schema_issues_and_inline_rules_but_never_a_notice`)
  with a synthesized notice, plus `BlockingPolicy`'s `Severity::Info` rule.
- **P7-D8. The precheck itself decides blocking; the policy is shared, not
  inline.**
  - *Decision:* `call_precheck::check_document` returns
    `PrecheckFailure::Blocked { blocking, rest }`, which the pipeline's
    existing `map_err` closure maps to `BlockedDiagnostics`;
    `ComposeOptions.blocking_policy` is an `Arc<BlockingPolicy>`; the new
    warning constructors in the pipeline and its transclusion phase are
    out-of-line helpers.
  - *Alternatives:* the check inline in the pipeline after the precheck (the
    first version).
  - *Why:* `darkmatter::l1 nested_composition::self_composition_reaches_the_depth_limit`
    overflowed the 2 MiB test stack (reproduced alone; passed at
    `RUST_MIN_STACK=2110000`, so under 13 KB over across 16 levels): the
    pipeline's frame recurs once per nested composition in debug builds,
    and the inline check's temporaries grew it. With the check in the
    precheck's own frame the test passes at the default stack. Same
    sensitivity as P6-D8.
  - *Downsides:* `earlier` in a precheck stop holds the report's prior
    warnings plus the precheck's non-blocking ones, assembled in the closure.

### Requirement-to-test mapping

| Behavior | Boundary / tier | Test |
|---|---|---|
| 23: `--allow all --deny <id>` and the reverse order deny it | spawned `md`, L1 | `darkmatter-cli::l1 policy_verbs::a_specific_denial_beats_allow_all_in_either_order` |
| 23: `--deny all --allow <id>` in either order recovers (warning reported, not blocking) | spawned, L1 | `policy_verbs::a_specific_recovery_beats_deny_all_in_either_order` |
| 23: `--allow x --deny x`, `--allow all --deny all`, both orders: exit 2, no stdout, no `initialize` effect | spawned, L1 | `policy_verbs::opposite_selections_are_usage_errors_in_either_order_before_any_effect` |
| 23/38: a recovery spelled as its code conflicts | spawned, L1 | `policy_verbs::a_recovery_denied_by_its_code_conflicts_with_its_allowance` |
| 24: missing hyperlink / image / transclusion allowances recover their kind only; directive removed | spawned, L1 | `policy_verbs::each_missing_reference_allowance_recovers_its_own_kind_only` |
| 24: ctx-override, invalid / reassigned assignment, shell timeout, host (ported legacy tests) | spawned, L1 | `compose_refs_and_missing::test_compose_scalar_ctx_with_allow_override_succeeds`, `compose_state_set::test_set_overlay_permissive_invalid_warns`, `::test_set_overlay_permissive_reassigned_warns_and_rightmost_wins`, `compose_shell::test_compose_allow_shell_timeout_emits_warning`, `compose_remote_caching::test_compose_remote_allowed_host_fetches_url` |
| 24: `--allow all` recovers a timeout as an empty string; grants no host (0 requests); authorizes no shell | spawned + mock HTTP, L1 | `policy_verbs::allow_all_recovers_a_shell_timeout_as_an_empty_string`, `::allow_all_grants_no_unnamed_host`, `::allow_all_authorizes_no_shell_command` |
| 24: each recovery's warning carries its registered code; missing transclusion fails without and is removed with the recovery | library `compose_with`, L1 | `darkmatter::l1 blocking_policy::each_recovery_reports_under_its_registered_code` |
| 25: every removed flag on every migrated command is a usage error naming its replacement | spawned, L1 | `policy_verbs::every_removed_flag_is_a_usage_error_naming_its_replacement`, `schema_trigger_rename::the_old_flag_is_a_usage_error_naming_the_new_one` |
| 25: completions offer only the new verbs and registry values | spawned `COMPLETE=fish`, L1 | `schema_trigger_rename::completions_offer_only_the_new_flag`, `policy_verbs::completions_offer_the_registry_identifiers_per_command` |
| 25: no legacy spelling in live scripts, docs, examples | `git grep` (log, below) | — |
| 26: `--deny style` / `--deny all` block a style issue on `md FILE`, `md render`, `md compose --output html` | spawned, L1 | `policy_verbs::deny_style_and_deny_all_block_a_style_issue_on_every_render_path`, `layout_style_frontmatter::deny_style_fails_on_unknown_key`, `::deny_style_fails_on_deprecated_key`, `::deny_style_accepts_markdown_cleanup_rules` |
| 26: informational notices never block; inline HR deprecations do | unit | `render::tests::a_denied_style_blocks_schema_issues_and_inline_rules_but_never_a_notice` |
| 20: `--deny all` fails the category 3 document (lifecycle and plain paths) | spawned, L1 | `compose_call_warnings::deny_all_fails_the_category_3_document` |
| 27: passive lifecycle finding stops before `initialize` (marker absent; control present) | spawned, L1 | `policy_verbs::a_passive_blocking_diagnostic_stops_before_initialize` |
| 27: body finding after `initialize` stops later work (`composed` effect absent), no rollback (`initialize` effect present) | spawned, L1 | `policy_verbs::a_blocking_diagnostic_found_after_an_effect_stops_later_work_without_rollback` |
| 27: the precheck stops before any expression runs (handler entries 0); a child blocks before its own expressions; severity unchanged | library + test hooks, L1 | `blocking_policy::a_blocking_call_warning_stops_the_document_before_any_expression_runs`, `::a_transcluded_child_blocks_before_its_own_expressions_run` |
| 28: no stdout in markdown, json, html; no viewer launch or temp file under `--show` | spawned, `MD_DRY_RUN`, L1 | `policy_verbs::deny_all_fails_a_category_3_document_with_no_document_in_any_mode` |
| 38: unknown, empty, unsupported identifiers; verbs a command does not declare | spawned, L1 | `policy_verbs::unknown_and_empty_identifiers_are_usage_errors`, `::identifiers_a_verb_or_command_does_not_take_are_usage_errors`, `::verbs_a_command_does_not_declare_are_usage_errors` |
| 38: repeated identical options harmless | spawned, L1 | `policy_verbs::repeated_identical_selections_are_harmless` |
| 38: baseline-file conflicts on compose, clean, schema validate | spawned, L1 | `policy_verbs::disabling_the_baseline_conflicts_with_an_explicit_baseline`, `clean_schema::test_baseline_schema_flags_conflict`, `schema_validate_baseline_selection::disable_baseline_schema_conflicts_with_schema_flag` |
| 38: rejected host denials | spawned, L1 | `policy_verbs::a_host_is_an_allowance_only`; `--deny all` keeps host consent: `::deny_all_keeps_explicit_host_consent` |
| 38: disabled checks stay disabled under `--deny all`, not reported as passing | spawned, L1 | `policy_verbs::a_disabled_baseline_stays_disabled_under_deny_all` |
| 38: independently denied diagnostic blocks a recovered command | spawned, L1 | `policy_verbs::an_independently_denied_diagnostic_still_blocks_a_recovered_command` |
| registry invariants, code→recovery resolution, exemptions | unit | `args::policy::tests::*`, `compose::policy::tests::*`, `args::cli::tests::compose_*` |
| help explains allowance vs denial and lists identifiers and codes | spawned, L1 | `policy_verbs::help_explains_that_an_allowance_controls_recovery_only` |

Criterion 28's `report-json` leg is Phase 8's. The Input Robustness Matrix
does not apply: no file-format reader changed (the verbs are argv values,
validated by the registry tests above).

Renamed tests (legacy names): `clean_schema::test_default_baseline_is_active_and_no_baseline_schema_disables_it`
→ `…_and_disable_baseline_schema_turns_it_off`, `::test_no_baseline_schema_suppresses_quoting`
→ `test_disable_baseline_schema_suppresses_quoting`, `::test_no_schema_triggers_disables_discovery`
→ `test_disable_schema_triggers_disables_discovery`;
`compose_base_schema::compose_no_baseline_schema_disables_default_baseline`
→ `compose_disable_baseline_schema_disables_default_baseline`;
`layout_style_frontmatter::style_strict_style_*` → `deny_style_*`,
`::style_fixture_strict_style_passes_on_schema_clean_doc` →
`style_fixture_deny_style_passes_on_schema_clean_doc`;
`schema_validate_baseline_selection::no_baseline_schema_conflicts_with_schema_flag`
→ `disable_baseline_schema_conflicts_with_schema_flag`,
`::no_schema_triggers_keeps_the_embedded_baseline` →
`disable_schema_triggers_keeps_the_embedded_baseline`. The Phase 1
traceability table above keeps the names it recorded.

### Gates

- **Scope calculator:** `git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`
  → `packages: ["darkmatter", "darkmatter-cli", "test-toolkit", "claudine-cli", "dmls"]`,
  `change_class: package`. Gates were run.
- **`darkmatter/` `just test`:** 12267 passed (5 slow), 10 skipped, after
  three fixes found by earlier runs: the `ping` row of the generated
  function table in `docs/topics/darkmatter-expressions.md` had gained
  backticks the catalog lacks (`catalog::tests::narrative_doc_function_table_matches_catalog`);
  the depth-limit stack overflow (P7-D8, deterministic, reproduced alone);
  `policy.md` spelled `--no-trigger-schemas`, which `trigger_spelling_guard`
  forbids in active docs (removed from its table); and `policy_verbs` set
  `MD_DRY_RUN` with a raw `.env`, which `spawn_site_guard` rejects (now
  `application_input`). **`just lint`:** passed (after gating the new
  `blocking_policy` L1 module on `effects-instrumentation`, like its
  neighbors). **`just check-tier-coverage darkmatter`:** 0 stranded.
- **`claudine/` `just test`:** 11455 passed (11 slow), 9 skipped.
  **`just lint`:** passed. **`just check-tier-coverage claudine`:** 0 stranded.
- **Dependents:** `cargo check --workspace --tests` is clean. Of darkmatter's
  reverse dependencies, only `darkmatter-cli`, `dmls`, and Claudine read
  compose warnings or `MarkdownError`; all three suites passed in full. The
  others (`messenger`, `research`, `sniff-cli`, `playa-cli`, `biscuit-icon-cli`,
  `biscuit-speaks-cli`, `claudine-gen`) reference none of the changed symbols.
- **Targeted runs during the phase:** CLI unit `args::|render::tests|policy`
  (77); `policy_verbs|compose_call_warnings|compose_refs_and_missing|compose_transclusion`
  (45); the migration agent's 21 modules (149 + 194); library
  `blocking_policy|shared_diagnostics` (8), `catalog::tests` (61),
  `trigger_spelling_guard` (2). Nothing was load-sensitive.
- **Cross-OS:** not run. The change is argv parsing, messages, and a
  transclusion recovery keyed on a resolution-failure class; no path,
  process, or OS-specific code. The one shell test mirrors the existing
  `compose_shell::test_compose_allow_shell_timeout_emits_warning`.
- **Stragglers:** `git grep` for every legacy spelling outside
  `features/`, `fixes/`, `reviews/`, `docs/research/`, and the frozen
  `recursive_schema_frozen` fixtures finds only `main.rs::REMOVED_FLAGS`, the
  tests that assert rejection, `policy.md`'s removed-flag table, and
  `trigger_spelling_guard.rs`'s own fixture lines. The internal
  `prompts/_add/add-expressions.md` line was updated too.

## Phase 8

Phase 8 delivers `md compose --output report-json` (spec §8; criteria 19,
22, 28, 29, 39) and the performance "after" sample with a bounds regression
test (spec §9; criterion 30).

### What changed

- **Library: shared failure diagnostics** (`darkmatter/lib/src/markdown/compose/report_diagnostics.rs`,
  new): `warning_diagnostic` (a warning's own record, else its code or
  `dm.compose.warning`, no source), `error_diagnostics` (any error chain: a
  policy stop's earlier then blocking warnings, a rejected call's record, one
  record per schema problem, else one record classified by the innermost
  typed error, located for YAML parse, fence, and on-disk interpolation
  errors), `reference_issue_diagnostic`, `lifecycle_failure_diagnostics`, and
  the family codes (P8-D4). `ValidationProblemCode::diagnostic_code` and the
  `SCHEMA_*_CODE` constants in `schemas/mod.rs`; DMLS's `codes.rs` now reads
  those, `FRONTMATTER_PARSE_CODE`, and `UNKNOWN_FUNCTION_CODE` from the
  library (strings unchanged).
- **Library: warnings survive a failure** (P8-D6):
  `Markdown::compose_with_warnings` → `Result<_, Box<ComposeFailure { error, warnings }>>`
  (boxed for clippy's `result_large_err`);
  `PipelineRuntime.failed_warnings` (shared with child runtimes) receives each
  failed document's warnings, the outer document's first;
  `run_root_pipeline_collecting` returns them. `compose_with` is unchanged.
  The pipeline node's report is boxed and built/finished out of line
  (`new_document_report`, `finish_document`; P8-D11).
- **CLI** (`darkmatter/cli`): `ComposeOutputFormat` (compose's own `--output`
  enum, adding `report-json`); `commands/compose/report.rs` (new):
  `ReportSink` (collects diagnostics in discovery order and the document),
  `ReportedExit` (the status `main` exits with silently), `ClassifiedFailure`
  (a CLI-worded shell failure keeping its typed diagnostics);
  `reject_report_conflicts` (`--show`, `--shell`). `run_compose` and the
  lifecycle host record warnings, reference issues, the document, and
  failures into the sink instead of printing; the reference refusal returns
  `ReportedExit(2)` and a failed lifecycle run `ReportedExit(code)` instead of
  `process::exit`; lifecycle `stdout` actions and command output go to stderr
  in report mode. `compose_error` / `preflight_approval_error` wrap shell
  failures in `ClassifiedFailure` (human text unchanged).
- **Docs**: new `darkmatter/docs/cli/report-json.md` (shape, exit table,
  streams, Mermaid flow, order, codes, privacy, no rollback);
  `docs/cli/compose.md` (`--output` values, output behavior, lifecycle
  output under report mode, example); `docs/cli/index.md` link.
- **Skill**: `.claude/skills/darkmatter/SKILL.md` (report-json ownership).

### Design Decisions

- **P8-D1. `report-json` is a value of compose's own output enum.**
  - *Decision:* `md compose --output` takes `ComposeOutputFormat` (the old
    values plus `report-json`); `md render` and `md FILE` keep `OutputFormat`,
    so clap rejects `report-json` there as an invalid value (exit 2); `md
    clean` has no `--output` (unknown argument, exit 2); `md schema validate
    --format report-json` is an invalid value.
  - *Alternatives:* add `ReportJson` to the shared `OutputFormat` and refuse
    it at run time in each other command.
  - *Why:* the refusal happens at parse time, before any input is read, with
    no per-command code; help and completions show the value only on
    compose.
  - *Downsides:* the rejection message is clap's ("invalid value"), not a
    bespoke "unsupported output format".
- **P8-D2. The report's `document` is the Markdown output** (as
  `--output markdown`, honoring `--frontmatter`).
  - *Alternatives:* a selectable inner format. *Why:* the spec says "the
    composed document as a string"; Markdown is compose's default.
  - *Downsides:* no HTML-in-report mode.
- **P8-D3. `--show` and `--shell` with `report-json` are usage errors** (clap
  `ArgumentConflict`, exit 2, through `policy::usage_error`). `--show` has no
  artifact to open; `--shell` prints its own report to stdout, which would
  break "stdout carries only the object".
- **P8-D4. "An existing typed code" means a code chosen by the failure's
  typed error.**
  - *Decision:* call, schema, YAML-parse, unknown-function, expression
    parse/evaluation, and recovery codes reuse the existing taxonomy (the
    DMLS/library strings, unchanged); a typed error with no code yet gets a
    family code in the same `dm.<family>.<kind>` form (`dm.input.read_failure`,
    `dm.directive.parse_failure`, `dm.shell.failure`, `dm.transclusion.failure`,
    `dm.reference.failure`, `dm.compose.parameter_binding`,
    `dm.lifecycle.failure`), and anything untyped is `dm.compose.failure`.
    The codes live in the library; the CLI only maps its own
    `DocumentArgumentError` onto `INPUT_READ_CODE`.
  - *Alternatives:* one catch-all code for every failure (loses the
    classification a script needs); a code per `MarkdownError` variant (30
    codes, most never distinguished by a consumer).
  - *Why:* the spec forbids inventing ad-hoc strings and requires library
    ownership; one code per failure family matches what DMLS already does.
  - *Downsides:* these failure codes are not `--deny` identifiers (they are
    errors, which already block), so they are not in `COMPOSE_DIAGNOSTIC_CODES`.
- **P8-D5. `source` is `null` unless the error holds an authored position.**
  Located: call findings, schema problems with a line, YAML parse (YAML line
  mapped to the file line as the error block does), fence mismatch, and
  interpolation errors anchored `OnDiskSpan`. Reference issues record a line
  but no column and their span indexes the scanned text, so they are
  unlocated rather than given a fabricated column. The file is the root as
  given and a transcluded child by its resolved path (P6-D6).
- **P8-D6. Earlier warnings travel beside the error, not inside it.**
  - *Decision:* `compose_with_warnings` returns `ComposeFailure { error,
    warnings }`; each failed document's pipeline puts its report's warnings
    into the runtime's shared stash, the outer document's first (it fails
    after its child, and its warnings were reported first).
  - *Alternatives:* a new `MarkdownError` wrapper variant (every caller that
    matches error variants would have to unwrap it); an observer on
    `ComposeReport::add_warning` (children's warnings are merged into their
    parent and would be seen twice).
  - *Why:* no existing error type changes and `compose_with` is untouched.
  - *Downsides:* when two concurrently resolved sibling transclusions both
    fail, their warnings appear in completion order. A failure raised by the
    pre-flight walk (for example a transcluded child's rejected call, found
    before composition) carries no earlier warnings: nothing had been
    reported. Documented in `report-json.md`.
- **P8-D7. The sink records diagnostics as the run reports them and skips a
  failure's records it already holds** (multiset subtraction by equality), so
  a policy stop raised by the CLI, which repeats the warnings it was given,
  lists each warning once, in discovery order.
- **P8-D8. In report mode a lifecycle `stdout:` action and lifecycle command
  output go to stderr.** Stdout must hold only the object; dropping the
  output would lose authored effects.
- **P8-D9. Exit statuses are unchanged by report mode:** 2 for a refused
  reference, the lifecycle run's own status, 1 otherwise; usage errors
  precede report mode and keep clap's stderr and exit 2.
- **P8-D10. A CLI-worded shell failure keeps its typed diagnostics**
  (`ClassifiedFailure`, Display = the old message, so human output is byte
  identical).
- **P8-D11. The pipeline node's report is boxed and built and finished out of
  line.** Moving the report out of the closure overflowed
  `nested_composition::self_composition_reaches_the_depth_limit` (alone,
  deterministic; passed at `RUST_MIN_STACK=2150000`); boxing alone still
  overflowed (the `ComposeReport` temporaries stayed in the frame);
  `new_document_report` / `finish_document` (`#[inline(never)]`) fixed it at
  the default stack. Same sensitivity as P6-D8 and P7-D8.

### Performance "after" (criterion 30)

Same host (Apple M4 Max, macOS 27.2), same input
(`darkmatter/cli/tests/fixtures/frozen_prompts/prompts/plan.md`, with
`spec=…/spec.md`), same three commands as the Phase 1 baseline, release
binaries built from this working tree (HEAD `032fab2ae` plus Phases 7–8
uncommitted). Load average 13–21 during the run (7–9 at baseline; other
sessions on the host).

| Measurement | Before | After |
|---|---|---|
| `md compose` (hyperfine, 20 runs) | 797.9 ms ± 5.5 | **782.0 ms ± 4.9** (773.1–796.4) |
| `md schema validate -vv` stages | resolve 8.3, compile 28.5, validate 48.3, total 85.5 ms | resolve 7.8, compile 26.8, validate 43.4, total **78.3 ms** |
| DMLS first publish (median of 10) | 250.1 ms (245.2–274.8) | **269.9 ms** (267.3–320.2); rerun 276.2 ms |
| DMLS second (cache-hit) publish | 41.8–48.5 ms | 47.7–55.4 ms |
| DMLS schema stages, first publish | total 101–107 ms | total 101–103 ms (one 116 ms first run) |

- **Composition:** no regression (−16 ms, within noise of the busier host).
- **DMLS:** first publish +~20 ms (8%), cache-hit publish +~6 ms. Both are
  outside schema validation, whose stages are unchanged, so the cost is the
  expression analysis this feature added to every published document:
  call diagnostics and narrowing at every expression site (Phases 2–4, 6) and
  the host catalog and facts artifact lookup at open (Phase 5). The load
  average was about twice the baseline's, which inflates both figures; the
  cache-hit delta (6 ms on 26 + 36 × 4 expressions) is the closer measure of
  the analysis itself. Judged acceptable: no per-keystroke path regressed by
  more than a few milliseconds.
- **Bounds:** the projection's depth bound (`scope/raw.rs::DEPTH_LIMIT`, 32)
  and named-type translation bound (`MAX_REFERENCE_DEPTH`, 32) are unchanged.
  `variable_type_projection::bounds::a_path_past_the_projection_depth_bound_stays_unknown`
  proves a 40-deep raw schema path is category 4 (unknown: neither rejected
  nor proven valid) while the same leaf 3 deep is rejected.

### Requirement-to-test mapping

| Behavior | Boundary / tier | Test |
|---|---|---|
| 29: success is one object `{outcome, diagnostics, document}`; human Markdown and AST JSON unchanged | spawned `md`, L1 | `darkmatter-cli::l1 compose_report_json::success_reports_the_document_and_leaves_other_outputs_unchanged` |
| 29/39: a warning carries all six fields, located; no stderr footer repeats it | spawned, L1 | `::a_category_3_warning_is_reported_once_with_every_field` |
| 19 (report leg): the rejected call's record equals the library's `error_diagnostics`, field for field | spawned + in-process, L1 | `::a_rejected_call_reports_the_shared_record` |
| 19: the library's report records equal DMLS's (rejection and warnings) | real LSP session, L1 | `dmls::l1 expression_call_diagnostics::shared_fields::a_rejected_call_reports_the_editor_fields`, `::a_category_3_warning_reports_the_editor_fields` |
| 39: input-read failure, null source/types | spawned, L1 | `::an_unreadable_input_is_one_failure_with_null_fields` |
| 39: schema and YAML parse failures, located | spawned, L1 | `::schema_and_parse_failures_are_located_in_the_frontmatter` |
| 39: earlier warning survives a runtime failure, in order | spawned, L1 | `::an_earlier_warning_survives_a_runtime_failure` |
| 39: transcluded warning and rejection located in the child | spawned, L1 | `::transcluded_findings_are_located_in_the_child` |
| 28 (report leg): `--deny all` fails with `document: null`, each warning once with its severity (plain and lifecycle paths) | spawned, `MD_DRY_RUN`, L1 | `::deny_all_reports_the_blocking_warning_once_and_no_document` |
| lifecycle: stdout holds only the report; a failed event is one diagnostic | spawned, L1 | `::a_lifecycle_run_writes_only_the_report_to_stdout` |
| refused reference keeps exit 2 | spawned, L1 | `::an_unresolved_reference_keeps_its_exit_status` |
| 22: no frontmatter value or transcluded content in the report | spawned, L1 | `::the_report_holds_no_value_or_file_content` |
| 39: `render`, `md FILE`, `clean`, `schema validate` reject report-json; `--show`/`--shell` conflict; exit 2, no stdout, no `--save` effect | spawned, L1 | `::report_conflicts_and_other_commands_are_usage_errors_before_any_work` |
| failure classification by typed error, located | library, L1 | `darkmatter::l1 report_diagnostics::each_failure_is_classified_by_its_typed_error`, `::a_rejected_call_reports_its_shared_record`, `::a_foreign_error_takes_a_family_code_and_no_source` |
| warning record / code / uncoded | library, L1 | `::a_warning_keeps_its_record_or_its_code` |
| `compose_with_warnings` keeps earlier warnings; `compose_with` unchanged | library, L1 | `::a_failed_composition_keeps_the_warnings_it_reported` |
| policy stop: earlier then blocking, own severities | library, L1 | `::a_policy_stop_reports_every_warning_with_its_own_severity` |
| 22: no fm/env/file value; no handler; no network; same records after the files are deleted (no read) | library + hooks + effects counter, L1 | `::report_diagnostics_hold_no_value_and_read_nothing` |
| 30: depth bound conservative | library, L1 | `variable_type_projection::bounds::a_path_past_the_projection_depth_bound_stays_unknown` |

The Input Robustness Matrix does not apply: no file-format reader or
configuration loader changed (the report is a writer).

### Gates

- **Scope calculator:** `git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`
  → `packages: ["darkmatter", "darkmatter-cli", "dmls", "test-toolkit", "claudine-cli"]`,
  `change_class: package` (the tree still holds Phase 7's uncommitted work).
  Gates were run.
- **`darkmatter/` `just test`:** 12287 passed (7 slow), 10 skipped.
  **`just lint`:** passed after two fixes (`ComposeFailure` boxed for
  `result_large_err`; `ClassifiedFailure::new` renamed `report` for
  `new_ret_no_self`); the touched tests were rerun after them
  (`report_diagnostics|nested_composition|bounds::` 26 passed;
  `compose_report_json|compose_shell|policy_verbs` 69 passed).
  **`just check-tier-coverage darkmatter`:** 0 stranded.
- **`claudine/` `just test`:** 11455 passed (12 slow), 9 skipped.
  **`just lint`:** passed. **`just check-tier-coverage claudine`:** 0 stranded.
- **Dependents:** `cargo check --workspace --tests` is clean. The phase adds
  public API only (`compose_with_warnings`, `report_diagnostics`,
  `ValidationProblemCode::diagnostic_code`, schema code constants) and leaves
  `compose_with` and every existing signature unchanged; DMLS and Claudine
  suites passed in full above.
- **Load-sensitive:** none. The one failure during the phase,
  `nested_composition::self_composition_reaches_the_depth_limit`, was a
  deterministic stack overflow (P8-D11), fixed before the full runs.
- **Cross-OS:** not run. No path, process, or OS-specific code changed; the
  report's `file` is `Path::display()` of the paths the existing diagnostics
  already carry, and the new CLI tests compare against the paths the fixture
  wrote (root) or `canonicalize_simplified` (children), never raw
  `canonicalize`.


## Phase 9

Documentation, skills, final traceability, and gates. No Rust source changed.

### What changed

- **Traceability.** Rows 17, 20, 21, 23–27, 36–38 of the Phase 1 table now
  name their tests (copied from the Phase 5, 6, and 7 mapping tables). Every
  criterion 1–39 maps to at least one test; criterion 30 also maps to this
  log's Phase 8 "after" sample.
- **Darkmatter docs.**
  - `docs/topics/darkmatter-expressions.md`: a Mermaid diagram of how a
    site's type scope is built (declarations → enclosing guards → writes →
    the consumers that read it).
  - `docs/cli/policy.md`: a Mermaid diagram of the precedence rules (the
    conflict check first, then specific beats `all`).
  - `docs/topics/compose-requests.md`: "Keeping the warnings of a failed
    composition", documenting `Markdown::compose_with_warnings` and
    `ComposeFailure` with an example.
  - `docs/lsp/configuration.md` (`[expressions]`): the two call warnings that
    ignore `strict` (`always_false_call`, `incomplete_analysis`) and that no
    `file_exists` setting exists.
  - `dmls/docs/diagnostics.md`: drift fixed. It said "there is no shared
    cross-crate constant module", but `codes.rs` reads the YAML parse,
    `dm.schema.*`, and expression call codes from the library (Phase 8). It
    now says shared codes come from the library and how to add one.
  - `README.md`: guards and host functions, the policy verbs, and
    `report-json`, each linking its topic page.
- **Claudine docs.** `docs/topics/schemas.md` gains "What Claudine adds to
  expression checking": the `globals.yaml` host catalog types `seq`/`loop`/
  `group` for the call check, Claudine registers no expression function (and
  the three pieces adding one requires, with the guard that enforces it), and
  Claudine's read inventory now comes from Darkmatter's
  `static_access_reads`/`static_variable_reads` instead of hand walks.
- **Live research contracts fixed.** Four Claudine research fleet documents
  (`claudine/docs/research/{agent-cli,non-interactive-sessions,reasoning-level,steering}/_research.md`)
  still ran `md schema validate '{{file}}' --no-trigger-schemas`, which is a
  usage error, so those contracts' shape checks could never pass. They now
  use `--disable schema-triggers`. See P9-D1.
- **biscuit-file.** The public passive validation API did not change in this
  phase; Phase 4 documented it in `docs/topics/file-references.md`. The
  skill gains a line naming `FileReference::new(s).is_ok()` as the passive
  structural validator and Darkmatter's one entry point to it
  (`resolve_ctx::is_well_formed_reference`).
- **Skills.** `.claude/skills/claudine/SKILL.md` links the new Claudine docs
  section; `.claude/skills/biscuit-file/SKILL.md` as above. The darkmatter
  skill was already current (Phases 2–8 kept it so); `rust-testing` needs no
  change (no new test pattern); no repo-wide convention changed, so
  `CLAUDE.md` is untouched.
- **Dependencies.** No `Cargo.toml` or `Cargo.lock` changed during this
  feature (`git log 6c1322cde~1..HEAD -- '*Cargo.toml' Cargo.lock` is
  empty), so `docs/dependencies.md` needs no change.

### Checks run for Wave 17

- **Legacy spellings.** `git grep` for every removed flag outside
  `features/`, `fixes/`, `reviews/`, root `docs/research/`, and the frozen
  `recursive_schema_frozen` fixtures finds only `darkmatter/cli/src/main.rs`
  (`REMOVED_FLAGS`), the tests that assert rejection (`policy_verbs.rs`,
  `schema_trigger_rename.rs`, `trigger_spelling_guard.rs`), and `policy.md`'s
  removed-flag table, after the four research contracts above were fixed.
- **Facts artifacts.** The only committed `.facts.json` is
  `darkmatter/lib/tests/fixtures/host_functions/project.facts.json`; its
  freshness test (`host_functions::the_committed_facts_artifact_is_fresh_and_deterministic`)
  passes in the gate run below. Claudine ships none (P5-D1).
- **Matrix reader smells.** `host/artifact.rs` and `host/mod.rs` have no
  `#[serde(default)]`, no conflated `Option` (absent and `null` go through
  `required`/`nullable`, which differ), no `unwrap_or_default()`, and no
  `filter_map(.. as_str())`; the one `.ok()` converts an already-validated
  `u64` to `usize` inside an error-returning `position`.
- **Docs name no feature or fix.** The pages created by this feature
  (`policy.md`, `report-json.md`, `host-functions.md`) and this phase's
  edits name no `{date}-{name}`. Older references in pages that Phase 7
  touched only for flag spellings (`fm-shell-expansion.md`, `caching.md`,
  `style.md`, `renderable/docs/layout-and-style.md`) predate this feature and
  were left alone (Rule 3).

### Design Decisions

- **P9-D1. Fix the live research contracts' flag; do not widen the spelling
  guard.**
  - *Decision:* replace `--no-trigger-schemas` with `--disable
    schema-triggers` in the four Claudine research contracts and leave
    `trigger_spelling_guard.rs`'s scope unchanged.
  - *Alternatives:* leave them (historical research); widen the guard to scan
    `claudine/docs/research/*/_research.md` for every removed flag.
  - *Why:* these files are not history: they are the executed fleet
    contracts (`.claude/skills/claudine/research-contracts.md` documents the
    same command with `--disable schema-triggers`), and the removed flag is a
    usage error, so their shape gate always failed. The guard deliberately
    skips every `research/` folder and `_research.md` file because most of
    that material is historical; widening it would need a per-file
    live/historical split that belongs to the research workflow, not this
    feature. A wrong flag in a contract fails loudly the first time the fleet
    runs.
  - *Downsides:* nothing stops a removed flag from reappearing in a research
    contract until a fleet run catches it.
- **P9-D2. Keep the two "planned" narrowing items.**
  - *Decision:* `darkmatter-expressions.md`'s "Planned" list (narrowing an
    object from a guard on one of its members; DMLS narrowing of Claudine
    lifecycle `then:`/`else:` bodies) stays as written.
  - *Alternatives:* remove the markers.
  - *Why:* neither is built (Phase 3 recorded both as outside this spec's
    criteria), so the docs correctly mark them planned.
  - *Downsides:* none.

### Requirement-to-test mapping

No behavior changed in this phase, so it adds no test. Wave 17's mapping of
every criterion 1–39 to its tests is the Phase 1 traceability table, now
complete. The four research-contract edits change a command string in
documents that Claudine's research parity tests read; those tests passed in
the gate run below.

### Gates

- **Scope calculator** (run after every edit, frontmatter included):
  `git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`
  → `packages: ["claudine", "claudine-cli", "claudine-gen", "darkmatter", "darkmatter-cli", "dmls"]`,
  `change_class: package`. Gates were run.
- **`darkmatter/` `just test`:** 12287 passed (2 slow), 10 skipped.
  **`just lint`:** passed.
- **`claudine/` `just test`:** 11455 passed (8 slow), 9 skipped.
  **`just lint`:** passed. **`just check-tier-coverage claudine`:** 0 stranded.
- **`biscuit-file/` `just test`:** 1151 passed. **`just lint`:** passed.
- **`just test-l2`:** not run; no L2 test reads anything this phase changed.
- **Dependents:** no Rust source changed in this phase; the full suites of
  every package the calculator scheduled passed above.
- **Load-sensitive / pre-existing failures:** none.
- **Cross-OS:** not run. Documentation and four command strings only; no
  path, process, or `#[cfg]` code. No `#[cfg(windows)]` behavior was added
  anywhere in this feature without a path-comparison review (the Phase 4
  structural validator is host-independent by design and its corpus runs on
  every host).

## Final Summary

**Implementation complete, ready for review.** All nine phases are done and
every acceptance criterion 1–39 is traceable to passing tests (Phase 1
table).

- **Departures from the spec**, each recorded where it was made:
  Claudine ships no host function catalog or facts artifact, because it has
  no host function; the path is proven by Darkmatter's test host instead
  (P5-D1). `md compose` has no host-function end-to-end test, because the `md`
  binary registers none (P5-D7). Narrowing an object from a guard on one of
  its members, and DMLS narrowing of Claudine lifecycle bodies, remain
  documented as planned (P3, P9-D2).
- **Rulings taken:** the design decisions under each phase (P2-D… through
  P9-D2) settle every ambiguity. None needs the author (`decisions_for_author: []`).
- **Drift found and resolved in this phase:** `dmls/docs/diagnostics.md`
  still said no cross-crate code module existed (fixed to match `codes.rs`).
  Four live Claudine research contracts used a flag that has been a usage
  error since the trigger rename (fixed; P9-D1). The Phase 1 traceability
  table had 11 empty rows (filled).
- **Not done by design:** nothing committed, the feature directory not
  moved, and `just complete` not run.

## Pipeline Refresh

This spec is step 21, the last step of `claudine/docs/rollout-strategy.md`'s
pipeline, and no active spec names it in `depends-on`. One active spec names
it under `related` and builds on what it delivered, so it was checked too:

- `2026-11-01-map-and-filter`: **changed.** Its plan has started (Phase 1
  done), so only unstarted Phase 2 changed. The spec's Decisions gains a
  reconciliation entry, and the plan gains a reconciliation section before
  Wave 3: the callback local must fit the flow-sensitive `TypeScope` (no
  document fact reaches a same-named local; local facts end with the body);
  `static_access_reads` (now Claudine's only read inventory) must skip
  locals as well as `static_variable_reads`; new codes are defined in the
  library and added to `COMPOSE_DIAGNOSTIC_CODES` so `--deny`, `--help`,
  `report-json`, and DMLS share them; the type-based absence rule makes
  `map(missing, …)` silent; callback parameters stay `unknown`. No outcome
  was dropped or narrowed.

## Review 1

### Design Decisions

- **Decision:** no implementation decision requires human review. Keep the
  existing empty-catalog and test-host choices: neither Claudine nor `md`
  registers host expression functions, so their empty artifacts and an
  impossible shipped-host CLI case add no evidence.
  - **Alternatives considered:** generate empty catalogs/artifacts; add a
    production host function solely for a test; ask the author to approve
    the existing recorded decisions.
  - **Why:** the public library and separately running editor already prove
    the promised host registration and artifact delivery. No irreversible
    contract choice lacks a recommendation.
  - **Downsides:** there is no built-in application example exercising a
    nonempty host function registry; the test host remains that example.

- **Decision:** retain the recorded remote `file_exists` behavior, including
  errors for denied or failed fetches, and leave member-discriminant object
  refinement outside this feature. Complete Claudine lifecycle narrowing in
  DMLS within this feature rather than leaving it planned.
  - **Alternatives considered:** reverse existing remote outcomes; require
    object-discriminant inference; retain the planned editor limitation.
  - **Why:** the remote section contradicts its own preserve-existing rule,
    which the implementation already resolved with evidence. Stable-subject
    narrowing does not require refining a parent object from a member.
    Runtime/editor agreement and criterion 34 do require the host lifecycle
    editor path, so that deferral cannot stand.
  - **Downsides:** denied remote access still cannot be used as a false
    existence result. DMLS needs passive host lifecycle information rather
    than assuming every document uses the Markdown host.

- **Decision:** preserve nullable-parameter suppression for direct arguments
  regardless of return type, while keeping the required warning for a
  variable nested inside another call such as `is_empty(trim(x))`. Represent
  that distinction with call-site nesting context, not return-type exclusion.
  - **Alternatives considered:** retain the Phase 6 blanket exclusion of
    nullable-returning functions; suppress every nested null-admitting call;
    change `trim`/`lower` signatures.
  - **Why:** the explicit any-function scope and criterion 36 require direct
    nullable arguments to be handled; criterion 21 separately preserves the
    nested example. A global return-type restriction narrows the former
    promise to solve the latter example and is therefore undelivered scope,
    not a decision requiring author approval.
  - **Downsides:** absence analysis must retain whether the variable belongs
    to the expression's direct argument or a nested call. Return nullability
    alone is no longer a sufficient implementation shortcut.
