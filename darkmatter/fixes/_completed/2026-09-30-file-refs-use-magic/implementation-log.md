---
spec: "/Volumes/coding/wt/rusty-biscuit/fix-magic-globs/darkmatter/fixes/2026-09-30-file-refs-use-magic/spec.md"
plan: "darkmatter/fixes/2026-09-30-file-refs-use-magic/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
source_files_during_phase_1:
    - darkmatter/lib/src/markdown/compose/context/options.rs
    - darkmatter/lib/src/markdown/compose/pipeline/mod.rs
    - darkmatter/lib/src/markdown/compose/preflight/collect.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/preflight_repository_sigils.rs
    - darkmatter/cli/tests/l1/compose_transclusion.rs
docs_updated_during_phase_1:
    - darkmatter/docs/inline/preflight-checks.md
docs_created_during_phase_1: []
skills_files_updated_during_phase_1:
    - .claude/skills/darkmatter/SKILL.md
    - .claude/skills/darkmatter/compose.md
packages:
    - darkmatter
    - darkmatter-cli
    - biscuit-file
    - sniff
    - dmls
    - claudine
    - claudine-gen
    - messenger
    - messenger-cli
source_files_during_phase_2:
    - biscuit-file/lib/src/file_reference/context.rs
    - biscuit-file/lib/src/file_reference/mod.rs
    - biscuit-file/lib/src/file_reference/resolve.rs
    - biscuit-file/lib/src/lib.rs
    - darkmatter/cli/src/commands/compose.rs
    - darkmatter/cli/src/commands/schema/triggers.rs
    - darkmatter/cli/src/commands/schema/validate.rs
    - darkmatter/lib/benches/compose_pipeline.rs
    - darkmatter/lib/benches/compose_schema_transclusion.rs
    - darkmatter/lib/benches/phase6_interpolation.rs
    - darkmatter/lib/benches/reference_graph.rs
    - darkmatter/lib/src/markdown/compose/cache/hashing.rs
    - darkmatter/lib/src/markdown/compose/context/capture/agent.rs
    - darkmatter/lib/src/markdown/compose/context/capture/mod.rs
    - darkmatter/lib/src/markdown/compose/context/checked.rs
    - darkmatter/lib/src/markdown/compose/context/current.rs
    - darkmatter/lib/src/markdown/compose/context/mod.rs
    - darkmatter/lib/src/markdown/compose/context/options.rs
    - darkmatter/lib/src/markdown/compose/context/request.rs
    - darkmatter/lib/src/markdown/compose/context/runtime.rs
    - darkmatter/lib/src/markdown/compose/expression/lint.rs
    - darkmatter/lib/src/markdown/compose/expression/resolve_ctx.rs
    - darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion/assignment.rs
    - darkmatter/lib/src/markdown/compose/interpolation/fatality_characterization.rs
    - darkmatter/lib/src/markdown/compose/mod.rs
    - darkmatter/lib/src/markdown/compose/pipeline/mod.rs
    - darkmatter/lib/src/markdown/compose/preflight/collect.rs
    - darkmatter/lib/src/markdown/compose/preflight/lifecycle.rs
    - darkmatter/lib/src/markdown/compose/preflight/mod.rs
    - darkmatter/lib/src/markdown/compose/schema_validation.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/mod.rs
    - darkmatter/lib/src/markdown/compose/tests/frontmatter.rs
    - darkmatter/lib/src/markdown/compose/tests/icmp.rs
    - darkmatter/lib/src/markdown/compose/tests/identity.rs
    - darkmatter/lib/src/markdown/compose/tests/lazy_roots.rs
    - darkmatter/lib/src/markdown/compose/tests/provider_network.rs
    - darkmatter/lib/src/markdown/compose/tests/rendering.rs
    - darkmatter/lib/src/markdown/compose/tests/schema.rs
    - darkmatter/lib/src/markdown/compose/tests/shell.rs
    - darkmatter/lib/src/markdown/compose/tests/transclusion.rs
    - darkmatter/lib/src/markdown/compose/toc_linking/mod.rs
    - darkmatter/lib/src/markdown/compose/transclusion/resolver.rs
    - darkmatter/lib/src/markdown/compose/type_tests.rs
    - darkmatter/lib/src/markdown/compose/util.rs
    - darkmatter/lib/src/markdown/reference/file_tree/mod.rs
    - darkmatter/lib/src/markdown/reference/graph.rs
    - darkmatter/lib/src/markdown/reference/mod.rs
    - darkmatter/lib/src/markdown/reference/types.rs
    - darkmatter/lib/src/markdown/reference/validate.rs
    - darkmatter/lib/src/markdown/schemas/clean.rs
    - darkmatter/lib/src/markdown/schemas/format.rs
    - darkmatter/lib/tests/l1/ambient_ctx_capture.rs
    - darkmatter/lib/tests/l1/array_rendering_json.rs
    - darkmatter/lib/tests/l1/backslash_escape_spans.rs
    - darkmatter/lib/tests/l1/compose_diagnostic_identity.rs
    - darkmatter/lib/tests/l1/compose_expression_failure_contract.rs
    - darkmatter/lib/tests/l1/compose_phase6.rs
    - darkmatter/lib/tests/l1/compose_reuse_phase5.rs
    - darkmatter/lib/tests/l1/context_functions.rs
    - darkmatter/lib/tests/l1/dasherized_identifier_compose.rs
    - darkmatter/lib/tests/l1/data_origin.rs
    - darkmatter/lib/tests/l1/declined_path_transclusion.rs
    - darkmatter/lib/tests/l1/disclosure_transclusion_integration.rs
    - darkmatter/lib/tests/l1/empty_package_area.rs
    - darkmatter/lib/tests/l1/expression_regression.rs
    - darkmatter/lib/tests/l1/feature_review_incident.rs
    - darkmatter/lib/tests/l1/file_tree_roots.rs
    - darkmatter/lib/tests/l1/find_files_and_try_frontmatter.rs
    - darkmatter/lib/tests/l1/frontmatter_surface_projection.rs
    - darkmatter/lib/tests/l1/git_context_integration.rs
    - darkmatter/lib/tests/l1/interpolation_literal_pipeline.rs
    - darkmatter/lib/tests/l1/link_interpolation_integration.rs
    - darkmatter/lib/tests/l1/literal_token.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/meta_schema_phase4.rs
    - darkmatter/lib/tests/l1/missing_ctx_capture.rs
    - darkmatter/lib/tests/l1/nested_composition.rs
    - darkmatter/lib/tests/l1/persistent_cache_disabled.rs
    - darkmatter/lib/tests/l1/predict_conflicts.rs
    - darkmatter/lib/tests/l1/preflight_child_state_parity.rs
    - darkmatter/lib/tests/l1/preflight_repository_sigils.rs
    - darkmatter/lib/tests/l1/reference_integration.rs
    - darkmatter/lib/tests/l1/request_context_builder.rs
    - darkmatter/lib/tests/l1/request_context_epoch.rs
    - darkmatter/lib/tests/l1/schemas_literal_expression.rs
    - darkmatter/lib/tests/l1/set_overlay_integration.rs
    - darkmatter/lib/tests/l1/shell_block_integration.rs
    - darkmatter/lib/tests/l1/shell_expansion_coordinates.rs
    - darkmatter/lib/tests/l1/shell_probe_preflight.rs
    - darkmatter/lib/tests/l1/shell_result_values.rs
    - darkmatter/lib/tests/l1/suggest_constraint_phase3.rs
    - darkmatter/lib/tests/l1/ternary_integration.rs
    - darkmatter/lib/tests/l1/transcluded_shell_failure.rs
    - darkmatter/lib/tests/l1/unknown_identifier_warning.rs
    - darkmatter/lib/tests/l1/url_root_identity.rs
    - darkmatter/lib/tests/level2/level2_render_tree_terminal/support/mod.rs
    - darkmatter/lib/tests/request_support/mod.rs
    - sniff/lib/src/filesystem/repo/glob.rs
    - sniff/lib/src/filesystem/repo/manifest_index.rs
    - sniff/lib/src/filesystem/repo/mod.rs
docs_updated_during_phase_2:
    - darkmatter/lib/README.md
    - darkmatter/docs/topics/magic-paths.md
    - darkmatter/docs/topics/schemas/definition.md
    - darkmatter/docs/inline/text-replacement.md
    - darkmatter/docs/inline/preflight-checks.md
    - biscuit-file/docs/topics/file-references.md
    - sniff/docs/sniff-library-architecture.md
docs_created_during_phase_2:
    - darkmatter/docs/topics/compose-requests.md
skills_files_updated_during_phase_2:
    - .claude/skills/darkmatter/SKILL.md
    - .claude/skills/darkmatter/compose.md
    - .claude/skills/biscuit-file/references/file-references.md
    - .claude/skills/sniff/remote-and-repository.md
source_files_during_phase_4:
    - darkmatter/dmls/Cargo.toml
    - Cargo.lock
    - darkmatter/dmls/src/context.rs
    - darkmatter/dmls/src/lib.rs
    - darkmatter/dmls/src/main.rs
    - darkmatter/dmls/src/router.rs
    - darkmatter/dmls/src/bench.rs
    - darkmatter/dmls/src/diagnostics/codes.rs
    - darkmatter/dmls/src/diagnostics/frontmatter.rs
    - darkmatter/dmls/src/diagnostics/frontmatter/severity_tests.rs
    - darkmatter/dmls/src/diagnostics/nested_span/nested_span_tests.rs
    - darkmatter/dmls/src/graph/arena.rs
    - darkmatter/dmls/src/graph/invalidate.rs
    - darkmatter/dmls/src/graph/mod.rs
    - darkmatter/dmls/src/overlay/mod.rs
    - darkmatter/dmls/src/overlay/schema.rs
    - darkmatter/dmls/src/providers/code_actions.rs
    - darkmatter/dmls/src/providers/completion.rs
    - darkmatter/dmls/src/providers/diagnostics.rs
    - darkmatter/dmls/src/providers/dsl.rs
    - darkmatter/dmls/src/providers/frontmatter.rs
    - darkmatter/dmls/src/providers/frontmatter/sequence_tests.rs
    - darkmatter/dmls/src/providers/hover.rs
    - darkmatter/dmls/src/providers/mod.rs
    - darkmatter/dmls/src/providers/semantic_tokens.rs
    - darkmatter/dmls/src/workspace/snapshot.rs
    - darkmatter/dmls/src/workspace/startup.rs
    - darkmatter/dmls/src/workspace/watch.rs
    - darkmatter/dmls/tests/common/mod.rs
    - darkmatter/dmls/tests/fixtures/mapping_only_corpus/baseline.json
    - darkmatter/dmls/tests/l1/level1_graph_index.rs
    - darkmatter/dmls/tests/l1/level1_wiki.rs
    - darkmatter/dmls/tests/l1/main.rs
    - darkmatter/dmls/tests/l1/mapping_only_corpus.rs
    - darkmatter/dmls/tests/l1/repository_contexts.rs
    - darkmatter/lib/src/markdown/compose/schema_validation.rs
    - darkmatter/lib/src/markdown/compose/tests/schema.rs
    - darkmatter/lib/src/markdown/schemas/format.rs
    - darkmatter/lib/src/markdown/schemas/mod.rs
    - darkmatter/lib/src/markdown/schemas/validate.rs
    - darkmatter/lib/tests/l1/error_snapshots/snapshots/l1__error_snapshots__reference__file_reference.snap
    - darkmatter/lib/tests/l1/error_snapshots/snapshots/l1__error_snapshots__transclusion__file_reference.snap
    - claudine/lib/src/composition/error/render/mod.rs
docs_updated_during_phase_4:
    - darkmatter/dmls/docs/diagnostics.md
    - darkmatter/dmls/docs/features.md
    - darkmatter/dmls/README.md
    - darkmatter/docs/dependencies.md
docs_created_during_phase_4:
    - darkmatter/dmls/docs/file-references.md
skills_files_updated_during_phase_4:
    - .claude/skills/darkmatter/SKILL.md
    - .claude/skills/darkmatter/dmls.md
    - .claude/skills/os/windows.md
source_files_during_phase_5:
    - claudine/gen/src/agent_errors_check.rs
    - claudine/gen/src/agent_errors_check/review6_tests.rs
    - claudine/gen/src/inputs.rs
    - claudine/gen/src/main.rs
    - claudine/gen/src/signals.rs
    - claudine/gen/src/steering_catalog.rs
    - claudine/gen/src/steering_check.rs
    - claudine/gen/tests/l1/agent_errors_check.rs
    - claudine/gen/tests/l1/drift.rs
    - claudine/gen/tests/l1/signals_validation.rs
    - claudine/gen/tests/l1/steering_activation.rs
    - darkmatter/cli/src/commands/clean/frontmatter_repair.rs
    - darkmatter/cli/src/commands/compose.rs
    - darkmatter/cli/src/commands/frontmatter.rs
    - darkmatter/cli/src/commands/graph.rs
    - darkmatter/cli/src/commands/hash.rs
    - darkmatter/cli/src/commands/mod.rs
    - darkmatter/cli/src/commands/schema/assignment.rs
    - darkmatter/cli/src/commands/schema/detect.rs
    - darkmatter/cli/src/commands/schema/triggers.rs
    - darkmatter/cli/src/commands/schema/validate.rs
    - darkmatter/cli/src/io/mod.rs
    - darkmatter/cli/src/request.rs
    - darkmatter/cli/tests/l1/compose_shell.rs
    - darkmatter/cli/tests/l1/compose_state_set.rs
    - darkmatter/dmls/src/overlay/mod.rs
    - darkmatter/dmls/src/overlay/schema.rs
    - darkmatter/dmls/src/providers/frontmatter/sequence_tests.rs
    - darkmatter/lib/benches/clean_hot_paths.rs
    - darkmatter/lib/benches/effective_schema_ownership.rs
    - darkmatter/lib/benches/schema_validation.rs
    - darkmatter/lib/src/markdown/compose/cache/hashing.rs
    - darkmatter/lib/src/markdown/compose/conditions.rs
    - darkmatter/lib/src/markdown/compose/context/authority.rs
    - darkmatter/lib/src/markdown/compose/context/capture/mod.rs
    - darkmatter/lib/src/markdown/compose/context/effective_state.rs
    - darkmatter/lib/src/markdown/compose/context/options.rs
    - darkmatter/lib/src/markdown/compose/context/request.rs
    - darkmatter/lib/src/markdown/compose/context/runtime.rs
    - darkmatter/lib/src/markdown/compose/expression/catalog/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/git.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/repository.rs
    - darkmatter/lib/src/markdown/compose/expression/path_projection.rs
    - darkmatter/lib/src/markdown/compose/expression/resolve_ctx.rs
    - darkmatter/lib/src/markdown/compose/expression/semantics.rs
    - darkmatter/lib/src/markdown/compose/frontmatter_interpolation.rs
    - darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs
    - darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion/assignment.rs
    - darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion/tests/execution_tests.rs
    - darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion/tests/tests.rs
    - darkmatter/lib/src/markdown/compose/inline/interpolation.rs
    - darkmatter/lib/src/markdown/compose/inline/page_blocks.rs
    - darkmatter/lib/src/markdown/compose/inline/replacement.rs
    - darkmatter/lib/src/markdown/compose/inline/shell_expansion.rs
    - darkmatter/lib/src/markdown/compose/interpolation/fatality_characterization.rs
    - darkmatter/lib/src/markdown/compose/link_normalization.rs
    - darkmatter/lib/src/markdown/compose/link_resolve.rs
    - darkmatter/lib/src/markdown/compose/mod.rs
    - darkmatter/lib/src/markdown/compose/nested.rs
    - darkmatter/lib/src/markdown/compose/pipeline/mod.rs
    - darkmatter/lib/src/markdown/compose/pipeline/phases.rs
    - darkmatter/lib/src/markdown/compose/preflight/collect.rs
    - darkmatter/lib/src/markdown/compose/preflight/lifecycle.rs
    - darkmatter/lib/src/markdown/compose/preflight/mod.rs
    - darkmatter/lib/src/markdown/compose/schema_validation.rs
    - darkmatter/lib/src/markdown/compose/shell_blocks/mod.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/mod.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/store.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/types.rs
    - darkmatter/lib/src/markdown/compose/tests/lazy_roots.rs
    - darkmatter/lib/src/markdown/compose/tests/schema.rs
    - darkmatter/lib/src/markdown/compose/transclusion/engine.rs
    - darkmatter/lib/src/markdown/compose/transclusion/resolver.rs
    - darkmatter/lib/src/markdown/compose/type_tests.rs
    - darkmatter/lib/src/markdown/compose/unknown_identifiers.rs
    - darkmatter/lib/src/markdown/compose/util.rs
    - darkmatter/lib/src/markdown/reference/file_tree/mod.rs
    - darkmatter/lib/src/markdown/reference/file_tree/model.rs
    - darkmatter/lib/src/markdown/reference/graph.rs
    - darkmatter/lib/src/markdown/reference/mod.rs
    - darkmatter/lib/src/markdown/reference/provenance.rs
    - darkmatter/lib/src/markdown/reference/types.rs
    - darkmatter/lib/src/markdown/reference/validate.rs
    - darkmatter/lib/src/markdown/schemas/clean.rs
    - darkmatter/lib/src/markdown/schemas/coerce.rs
    - darkmatter/lib/src/markdown/schemas/completion.rs
    - darkmatter/lib/src/markdown/schemas/detect.rs
    - darkmatter/lib/src/markdown/schemas/example.rs
    - darkmatter/lib/src/markdown/schemas/file_match.rs
    - darkmatter/lib/src/markdown/schemas/format.rs
    - darkmatter/lib/src/markdown/schemas/mod.rs
    - darkmatter/lib/src/markdown/schemas/resolve.rs
    - darkmatter/lib/src/markdown/schemas/rewrite.rs
    - darkmatter/lib/src/markdown/schemas/simplified/convert.rs
    - darkmatter/lib/src/markdown/schemas/simplified/lint.rs
    - darkmatter/lib/src/markdown/schemas/tests/clean_quoting.rs
    - darkmatter/lib/src/markdown/schemas/tests/mod.rs
    - darkmatter/lib/src/markdown/schemas/triggers/assemble.rs
    - darkmatter/lib/src/markdown/schemas/triggers/discovery.rs
    - darkmatter/lib/src/markdown/schemas/validate.rs
    - darkmatter/lib/tests/l1/ambient_ctx_capture.rs
    - darkmatter/lib/tests/l1/base_schema_end_to_end.rs
    - darkmatter/lib/tests/l1/clean_counters.rs
    - darkmatter/lib/tests/l1/dasherized_identifier_corpus.rs
    - darkmatter/lib/tests/l1/directive_target_analysis.rs
    - darkmatter/lib/tests/l1/expression_regression.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/meta_schema_phase1.rs
    - darkmatter/lib/tests/l1/meta_schema_phase3.rs
    - darkmatter/lib/tests/l1/meta_schema_phase4.rs
    - darkmatter/lib/tests/l1/meta_schema_phase5.rs
    - darkmatter/lib/tests/l1/meta_schema_reference_graph.rs
    - darkmatter/lib/tests/l1/meta_schema_repo_schemas.rs
    - darkmatter/lib/tests/l1/more_is_more_literals_and_indexes.rs
    - darkmatter/lib/tests/l1/predict_conflicts.rs
    - darkmatter/lib/tests/l1/prelude_exports.rs
    - darkmatter/lib/tests/l1/reference_integration.rs
    - darkmatter/lib/tests/l1/request_context_builder.rs
    - darkmatter/lib/tests/l1/required_context.rs
    - darkmatter/lib/tests/l1/schema_phase_validation.rs
    - darkmatter/lib/tests/l1/schema_quoting_safety.rs
    - darkmatter/lib/tests/l1/schemas_detect_table.rs
    - darkmatter/lib/tests/l1/schemas_literal_expression.rs
    - darkmatter/lib/tests/l1/schemas_required_count_matrix.rs
    - darkmatter/lib/tests/l1/schemas_source_projection.rs
    - darkmatter/lib/tests/l1/schemas_validate_table.rs
    - darkmatter/lib/tests/l1/shell_result_values.rs
    - darkmatter/lib/tests/l1/suggest_constraint_phase1.rs
    - darkmatter/lib/tests/l1/suggest_constraint_phase2.rs
    - darkmatter/lib/tests/l1/suggest_constraint_phase3.rs
    - darkmatter/lib/tests/l1/suggest_constraint_phase4.rs
    - darkmatter/lib/tests/request_support/mod.rs
    - messenger/lib/src/research/load.rs
    - messenger/lib/tests/research_corpus.rs
    - messenger/lib/tests/research_validation.rs
docs_updated_during_phase_5:
    - darkmatter/dmls/docs/diagnostics.md
    - darkmatter/dmls/docs/file-references.md
    - darkmatter/docs/inline/preflight-checks.md
    - darkmatter/docs/inline/schema-validation.md
    - darkmatter/docs/structs/Markdown.md
    - darkmatter/docs/topics/caching.md
    - darkmatter/docs/topics/darkmatter-expressions.md
    - darkmatter/docs/topics/schemas/definition.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
    - .claude/skills/darkmatter/compose.md
    - .claude/skills/darkmatter/dmls.md
    - .claude/skills/darkmatter/SKILL.md
    - .claude/skills/os/windows.md
source_files_during_phase_6:
    - biscuit-file/lib/src/file_reference/mod.rs
    - biscuit-file/lib/src/file_reference/resolve.rs
    - biscuit-file/lib/tests/l1/resolution_context.rs
    - claudine/gen/Cargo.toml
    - claudine/gen/src/agent_errors_check.rs
    - claudine/gen/src/agent_errors_check/review6_tests.rs
    - claudine/gen/tests/l1/context_construction_guard.rs
    - claudine/gen/tests/l1/main.rs
    - darkmatter/cli/tests/common/context_guard.rs
    - darkmatter/cli/tests/common/source_scan.rs
    - darkmatter/cli/tests/l1/context_construction_guard.rs
    - darkmatter/cli/tests/l1/main.rs
    - darkmatter/dmls/Cargo.toml
    - darkmatter/dmls/src/context.rs
    - darkmatter/dmls/src/diagnostics/frontmatter.rs
    - darkmatter/dmls/src/graph/arena.rs
    - darkmatter/dmls/src/overlay/mod.rs
    - darkmatter/dmls/src/providers/completion.rs
    - darkmatter/dmls/src/providers/frontmatter.rs
    - darkmatter/dmls/src/providers/mod.rs
    - darkmatter/dmls/src/router.rs
    - darkmatter/dmls/tests/l1/context_construction_guard.rs
    - darkmatter/dmls/tests/l1/main.rs
    - darkmatter/lib/Cargo.toml
    - darkmatter/lib/src/markdown/compose/conditions.rs
    - darkmatter/lib/src/markdown/compose/context/capture/agent.rs
    - darkmatter/lib/src/markdown/compose/context/capture/mod.rs
    - darkmatter/lib/src/markdown/compose/context/capture/repo.rs
    - darkmatter/lib/src/markdown/compose/context/current.rs
    - darkmatter/lib/src/markdown/compose/context/request.rs
    - darkmatter/lib/src/markdown/compose/expression/ctx.rs
    - darkmatter/lib/src/markdown/compose/frontmatter_interpolation.rs
    - darkmatter/lib/src/markdown/compose/link_resolve.rs
    - darkmatter/lib/src/markdown/compose/pipeline/mod.rs
    - darkmatter/lib/src/markdown/compose/schema_validation.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/executor.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/mod.rs
    - darkmatter/lib/src/markdown/compose/unknown_identifiers.rs
    - darkmatter/lib/src/markdown/compose/util.rs
    - darkmatter/lib/src/markdown/schemas/resolve.rs
    - darkmatter/lib/tests/l1/context_construction_guard.rs
    - darkmatter/lib/tests/l1/expression_regression.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/required_context.rs
    - darkmatter/lib/tests/l1/semantic_results_never_persist.rs
    - darkmatter/lib/tests/request_support/mod.rs
    - messenger/cli/Cargo.toml
    - messenger/cli/src/research.rs
    - messenger/cli/tests/context_construction_guard.rs
    - messenger/cli/tests/research_cli.rs
    - messenger/lib/Cargo.toml
    - messenger/lib/tests/context_construction_guard.rs
    - messenger/lib/tests/research_corpus.rs
docs_updated_during_phase_6:
    - biscuit-file/docs/topics/file-references.md
    - darkmatter/docs/inline/shell-expansion.md
    - darkmatter/docs/topics/compose-requests.md
docs_created_during_phase_6: []
skills_files_updated_during_phase_6:
    - .claude/skills/biscuit-file/references/file-references.md
    - .claude/skills/darkmatter/SKILL.md
    - .claude/skills/darkmatter/compose.md
    - .claude/skills/darkmatter/dmls.md
    - .claude/skills/os/SKILL.md
source_files_during_phase_7:
    - darkmatter/lib/src/markdown/types.rs
    - darkmatter/lib/src/markdown/compose/context/options.rs
    - darkmatter/lib/src/markdown/compose/context/report.rs
    - darkmatter/lib/src/markdown/compose/context/request.rs
    - darkmatter/lib/src/markdown/compose/expression/resolve_ctx.rs
    - darkmatter/lib/src/markdown/compose/pipeline/phases.rs
    - darkmatter/lib/src/markdown/compose/schema_validation.rs
    - darkmatter/lib/src/markdown/compose/toc_linking/mod.rs
    - darkmatter/lib/src/markdown/compose/toc_linking/types.rs
    - darkmatter/lib/src/markdown/compose/tests/transclusion.rs
    - darkmatter/lib/src/markdown/errors/blocks.rs
    - darkmatter/lib/tests/common/entry_point_parity/mod.rs
    - darkmatter/lib/tests/l1/entry_point_parity.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/error_snapshots/toc_linking.rs
    - darkmatter/lib/tests/l1/error_snapshots/snapshots/l1__error_snapshots__markdown_error__schema_validation_format_failure_renders_block.snap
    - darkmatter/cli/Cargo.toml
    - darkmatter/cli/src/commands/compose.rs
    - darkmatter/cli/src/commands/schema/validate.rs
    - darkmatter/cli/tests/l1/entry_point_parity.rs
    - darkmatter/cli/tests/l1/main.rs
    - darkmatter/dmls/Cargo.toml
    - darkmatter/dmls/src/diagnostics/codes.rs
    - darkmatter/dmls/src/overlay/directives.rs
    - darkmatter/dmls/src/providers/dsl.rs
    - darkmatter/dmls/tests/l1/entry_point_parity.rs
    - darkmatter/dmls/tests/l1/main.rs
docs_updated_during_phase_7:
    - darkmatter/docs/inline/toc-linking.md
    - darkmatter/docs/topics/compose-requests.md
    - darkmatter/docs/topics/schemas/definition.md
    - darkmatter/dmls/docs/diagnostics.md
    - darkmatter/dmls/docs/features.md
    - darkmatter/dmls/docs/file-references.md
docs_created_during_phase_7: []
skills_files_updated_during_phase_7:
    - .claude/skills/darkmatter/SKILL.md
    - .claude/skills/os/windows.md
source_files_during_phase_8:
    - darkmatter/dmls/src/diagnostics/codes.rs
    - darkmatter/dmls/src/providers/definition.rs
    - darkmatter/lib/src/markdown/compose/tests/schema.rs
docs_updated_during_phase_8:
    - darkmatter/docs/topics/file-referencing.md
    - darkmatter/docs/topics/transclusion.md
    - darkmatter/docs/topics/darkmatter-expressions.md
    - darkmatter/docs/topics/dmls.md
    - darkmatter/docs/composition/index.md
    - darkmatter/docs/darkmatter-compose-pipeline.md
    - darkmatter/docs/errors/README.md
    - darkmatter/docs/lsp/architecture.md
    - darkmatter/docs/lsp/features.md
    - darkmatter/dmls/docs/diagnostics.md
docs_created_during_phase_8:
    - darkmatter/docs/errors/file-reference-failures.md
skills_files_updated_during_phase_8:
    - .claude/skills/darkmatter/SKILL.md
    - .claude/skills/darkmatter/compose.md
    - .claude/skills/darkmatter/library-surfaces.md
    - .claude/skills/os/windows.md
source_code:
    - darkmatter/lib/src/markdown/compose/context/options.rs
    - darkmatter/lib/src/markdown/compose/pipeline/mod.rs
    - darkmatter/lib/src/markdown/compose/preflight/collect.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/preflight_repository_sigils.rs
    - darkmatter/cli/tests/l1/compose_transclusion.rs
    - biscuit-file/lib/src/file_reference/context.rs
    - biscuit-file/lib/src/file_reference/mod.rs
    - biscuit-file/lib/src/file_reference/resolve.rs
    - biscuit-file/lib/src/lib.rs
    - darkmatter/cli/src/commands/compose.rs
    - darkmatter/cli/src/commands/schema/triggers.rs
    - darkmatter/cli/src/commands/schema/validate.rs
    - darkmatter/lib/benches/compose_pipeline.rs
    - darkmatter/lib/benches/compose_schema_transclusion.rs
    - darkmatter/lib/benches/phase6_interpolation.rs
    - darkmatter/lib/benches/reference_graph.rs
    - darkmatter/lib/src/markdown/compose/cache/hashing.rs
    - darkmatter/lib/src/markdown/compose/context/capture/agent.rs
    - darkmatter/lib/src/markdown/compose/context/capture/mod.rs
    - darkmatter/lib/src/markdown/compose/context/checked.rs
    - darkmatter/lib/src/markdown/compose/context/current.rs
    - darkmatter/lib/src/markdown/compose/context/mod.rs
    - darkmatter/lib/src/markdown/compose/context/request.rs
    - darkmatter/lib/src/markdown/compose/context/runtime.rs
    - darkmatter/lib/src/markdown/compose/expression/lint.rs
    - darkmatter/lib/src/markdown/compose/expression/resolve_ctx.rs
    - darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion/assignment.rs
    - darkmatter/lib/src/markdown/compose/interpolation/fatality_characterization.rs
    - darkmatter/lib/src/markdown/compose/mod.rs
    - darkmatter/lib/src/markdown/compose/preflight/lifecycle.rs
    - darkmatter/lib/src/markdown/compose/preflight/mod.rs
    - darkmatter/lib/src/markdown/compose/schema_validation.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/mod.rs
    - darkmatter/lib/src/markdown/compose/tests/frontmatter.rs
    - darkmatter/lib/src/markdown/compose/tests/icmp.rs
    - darkmatter/lib/src/markdown/compose/tests/identity.rs
    - darkmatter/lib/src/markdown/compose/tests/lazy_roots.rs
    - darkmatter/lib/src/markdown/compose/tests/provider_network.rs
    - darkmatter/lib/src/markdown/compose/tests/rendering.rs
    - darkmatter/lib/src/markdown/compose/tests/schema.rs
    - darkmatter/lib/src/markdown/compose/tests/shell.rs
    - darkmatter/lib/src/markdown/compose/tests/transclusion.rs
    - darkmatter/lib/src/markdown/compose/toc_linking/mod.rs
    - darkmatter/lib/src/markdown/compose/transclusion/resolver.rs
    - darkmatter/lib/src/markdown/compose/type_tests.rs
    - darkmatter/lib/src/markdown/compose/util.rs
    - darkmatter/lib/src/markdown/reference/file_tree/mod.rs
    - darkmatter/lib/src/markdown/reference/graph.rs
    - darkmatter/lib/src/markdown/reference/mod.rs
    - darkmatter/lib/src/markdown/reference/types.rs
    - darkmatter/lib/src/markdown/reference/validate.rs
    - darkmatter/lib/src/markdown/schemas/clean.rs
    - darkmatter/lib/src/markdown/schemas/format.rs
    - darkmatter/lib/tests/l1/ambient_ctx_capture.rs
    - darkmatter/lib/tests/l1/array_rendering_json.rs
    - darkmatter/lib/tests/l1/backslash_escape_spans.rs
    - darkmatter/lib/tests/l1/compose_diagnostic_identity.rs
    - darkmatter/lib/tests/l1/compose_expression_failure_contract.rs
    - darkmatter/lib/tests/l1/compose_phase6.rs
    - darkmatter/lib/tests/l1/compose_reuse_phase5.rs
    - darkmatter/lib/tests/l1/context_functions.rs
    - darkmatter/lib/tests/l1/dasherized_identifier_compose.rs
    - darkmatter/lib/tests/l1/data_origin.rs
    - darkmatter/lib/tests/l1/declined_path_transclusion.rs
    - darkmatter/lib/tests/l1/disclosure_transclusion_integration.rs
    - darkmatter/lib/tests/l1/empty_package_area.rs
    - darkmatter/lib/tests/l1/expression_regression.rs
    - darkmatter/lib/tests/l1/feature_review_incident.rs
    - darkmatter/lib/tests/l1/file_tree_roots.rs
    - darkmatter/lib/tests/l1/find_files_and_try_frontmatter.rs
    - darkmatter/lib/tests/l1/frontmatter_surface_projection.rs
    - darkmatter/lib/tests/l1/git_context_integration.rs
    - darkmatter/lib/tests/l1/interpolation_literal_pipeline.rs
    - darkmatter/lib/tests/l1/link_interpolation_integration.rs
    - darkmatter/lib/tests/l1/literal_token.rs
    - darkmatter/lib/tests/l1/meta_schema_phase4.rs
    - darkmatter/lib/tests/l1/missing_ctx_capture.rs
    - darkmatter/lib/tests/l1/nested_composition.rs
    - darkmatter/lib/tests/l1/persistent_cache_disabled.rs
    - darkmatter/lib/tests/l1/predict_conflicts.rs
    - darkmatter/lib/tests/l1/preflight_child_state_parity.rs
    - darkmatter/lib/tests/l1/reference_integration.rs
    - darkmatter/lib/tests/l1/request_context_builder.rs
    - darkmatter/lib/tests/l1/request_context_epoch.rs
    - darkmatter/lib/tests/l1/schemas_literal_expression.rs
    - darkmatter/lib/tests/l1/set_overlay_integration.rs
    - darkmatter/lib/tests/l1/shell_block_integration.rs
    - darkmatter/lib/tests/l1/shell_expansion_coordinates.rs
    - darkmatter/lib/tests/l1/shell_probe_preflight.rs
    - darkmatter/lib/tests/l1/shell_result_values.rs
    - darkmatter/lib/tests/l1/suggest_constraint_phase3.rs
    - darkmatter/lib/tests/l1/ternary_integration.rs
    - darkmatter/lib/tests/l1/transcluded_shell_failure.rs
    - darkmatter/lib/tests/l1/unknown_identifier_warning.rs
    - darkmatter/lib/tests/l1/url_root_identity.rs
    - darkmatter/lib/tests/level2/level2_render_tree_terminal/support/mod.rs
    - darkmatter/lib/tests/request_support/mod.rs
    - sniff/lib/src/filesystem/repo/glob.rs
    - sniff/lib/src/filesystem/repo/manifest_index.rs
    - sniff/lib/src/filesystem/repo/mod.rs
    - darkmatter/dmls/Cargo.toml
    - Cargo.lock
    - darkmatter/dmls/src/context.rs
    - darkmatter/dmls/src/lib.rs
    - darkmatter/dmls/src/main.rs
    - darkmatter/dmls/src/router.rs
    - darkmatter/dmls/src/bench.rs
    - darkmatter/dmls/src/diagnostics/codes.rs
    - darkmatter/dmls/src/diagnostics/frontmatter.rs
    - darkmatter/dmls/src/diagnostics/frontmatter/severity_tests.rs
    - darkmatter/dmls/src/diagnostics/nested_span/nested_span_tests.rs
    - darkmatter/dmls/src/graph/arena.rs
    - darkmatter/dmls/src/graph/invalidate.rs
    - darkmatter/dmls/src/graph/mod.rs
    - darkmatter/dmls/src/overlay/mod.rs
    - darkmatter/dmls/src/overlay/schema.rs
    - darkmatter/dmls/src/providers/code_actions.rs
    - darkmatter/dmls/src/providers/completion.rs
    - darkmatter/dmls/src/providers/diagnostics.rs
    - darkmatter/dmls/src/providers/dsl.rs
    - darkmatter/dmls/src/providers/frontmatter.rs
    - darkmatter/dmls/src/providers/frontmatter/sequence_tests.rs
    - darkmatter/dmls/src/providers/hover.rs
    - darkmatter/dmls/src/providers/mod.rs
    - darkmatter/dmls/src/providers/semantic_tokens.rs
    - darkmatter/dmls/src/workspace/snapshot.rs
    - darkmatter/dmls/src/workspace/startup.rs
    - darkmatter/dmls/src/workspace/watch.rs
    - darkmatter/dmls/tests/common/mod.rs
    - darkmatter/dmls/tests/fixtures/mapping_only_corpus/baseline.json
    - darkmatter/dmls/tests/l1/level1_graph_index.rs
    - darkmatter/dmls/tests/l1/level1_wiki.rs
    - darkmatter/dmls/tests/l1/main.rs
    - darkmatter/dmls/tests/l1/mapping_only_corpus.rs
    - darkmatter/dmls/tests/l1/repository_contexts.rs
    - darkmatter/lib/src/markdown/schemas/mod.rs
    - darkmatter/lib/src/markdown/schemas/validate.rs
    - darkmatter/lib/tests/l1/error_snapshots/snapshots/l1__error_snapshots__reference__file_reference.snap
    - darkmatter/lib/tests/l1/error_snapshots/snapshots/l1__error_snapshots__transclusion__file_reference.snap
    - claudine/lib/src/composition/error/render/mod.rs
    - claudine/gen/src/agent_errors_check.rs
    - claudine/gen/src/agent_errors_check/review6_tests.rs
    - claudine/gen/src/inputs.rs
    - claudine/gen/src/main.rs
    - claudine/gen/src/signals.rs
    - claudine/gen/src/steering_catalog.rs
    - claudine/gen/src/steering_check.rs
    - claudine/gen/tests/l1/agent_errors_check.rs
    - claudine/gen/tests/l1/drift.rs
    - claudine/gen/tests/l1/signals_validation.rs
    - claudine/gen/tests/l1/steering_activation.rs
    - darkmatter/cli/src/commands/clean/frontmatter_repair.rs
    - darkmatter/cli/src/commands/frontmatter.rs
    - darkmatter/cli/src/commands/graph.rs
    - darkmatter/cli/src/commands/hash.rs
    - darkmatter/cli/src/commands/mod.rs
    - darkmatter/cli/src/commands/schema/assignment.rs
    - darkmatter/cli/src/commands/schema/detect.rs
    - darkmatter/cli/src/io/mod.rs
    - darkmatter/cli/src/request.rs
    - darkmatter/cli/tests/l1/compose_shell.rs
    - darkmatter/cli/tests/l1/compose_state_set.rs
    - darkmatter/lib/benches/clean_hot_paths.rs
    - darkmatter/lib/benches/effective_schema_ownership.rs
    - darkmatter/lib/benches/schema_validation.rs
    - darkmatter/lib/src/markdown/compose/conditions.rs
    - darkmatter/lib/src/markdown/compose/context/authority.rs
    - darkmatter/lib/src/markdown/compose/context/effective_state.rs
    - darkmatter/lib/src/markdown/compose/expression/catalog/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/git.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/repository.rs
    - darkmatter/lib/src/markdown/compose/expression/path_projection.rs
    - darkmatter/lib/src/markdown/compose/expression/semantics.rs
    - darkmatter/lib/src/markdown/compose/frontmatter_interpolation.rs
    - darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs
    - darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion/tests/execution_tests.rs
    - darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion/tests/tests.rs
    - darkmatter/lib/src/markdown/compose/inline/interpolation.rs
    - darkmatter/lib/src/markdown/compose/inline/page_blocks.rs
    - darkmatter/lib/src/markdown/compose/inline/replacement.rs
    - darkmatter/lib/src/markdown/compose/inline/shell_expansion.rs
    - darkmatter/lib/src/markdown/compose/link_normalization.rs
    - darkmatter/lib/src/markdown/compose/link_resolve.rs
    - darkmatter/lib/src/markdown/compose/nested.rs
    - darkmatter/lib/src/markdown/compose/pipeline/phases.rs
    - darkmatter/lib/src/markdown/compose/shell_blocks/mod.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/store.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/types.rs
    - darkmatter/lib/src/markdown/compose/transclusion/engine.rs
    - darkmatter/lib/src/markdown/compose/unknown_identifiers.rs
    - darkmatter/lib/src/markdown/reference/file_tree/model.rs
    - darkmatter/lib/src/markdown/reference/provenance.rs
    - darkmatter/lib/src/markdown/schemas/coerce.rs
    - darkmatter/lib/src/markdown/schemas/completion.rs
    - darkmatter/lib/src/markdown/schemas/detect.rs
    - darkmatter/lib/src/markdown/schemas/example.rs
    - darkmatter/lib/src/markdown/schemas/file_match.rs
    - darkmatter/lib/src/markdown/schemas/resolve.rs
    - darkmatter/lib/src/markdown/schemas/rewrite.rs
    - darkmatter/lib/src/markdown/schemas/simplified/convert.rs
    - darkmatter/lib/src/markdown/schemas/simplified/lint.rs
    - darkmatter/lib/src/markdown/schemas/tests/clean_quoting.rs
    - darkmatter/lib/src/markdown/schemas/tests/mod.rs
    - darkmatter/lib/src/markdown/schemas/triggers/assemble.rs
    - darkmatter/lib/src/markdown/schemas/triggers/discovery.rs
    - darkmatter/lib/tests/l1/base_schema_end_to_end.rs
    - darkmatter/lib/tests/l1/clean_counters.rs
    - darkmatter/lib/tests/l1/dasherized_identifier_corpus.rs
    - darkmatter/lib/tests/l1/directive_target_analysis.rs
    - darkmatter/lib/tests/l1/meta_schema_phase1.rs
    - darkmatter/lib/tests/l1/meta_schema_phase3.rs
    - darkmatter/lib/tests/l1/meta_schema_phase5.rs
    - darkmatter/lib/tests/l1/meta_schema_reference_graph.rs
    - darkmatter/lib/tests/l1/meta_schema_repo_schemas.rs
    - darkmatter/lib/tests/l1/more_is_more_literals_and_indexes.rs
    - darkmatter/lib/tests/l1/prelude_exports.rs
    - darkmatter/lib/tests/l1/required_context.rs
    - darkmatter/lib/tests/l1/schema_phase_validation.rs
    - darkmatter/lib/tests/l1/schema_quoting_safety.rs
    - darkmatter/lib/tests/l1/schemas_detect_table.rs
    - darkmatter/lib/tests/l1/schemas_required_count_matrix.rs
    - darkmatter/lib/tests/l1/schemas_source_projection.rs
    - darkmatter/lib/tests/l1/schemas_validate_table.rs
    - darkmatter/lib/tests/l1/suggest_constraint_phase1.rs
    - darkmatter/lib/tests/l1/suggest_constraint_phase2.rs
    - darkmatter/lib/tests/l1/suggest_constraint_phase4.rs
    - messenger/lib/src/research/load.rs
    - messenger/lib/tests/research_corpus.rs
    - messenger/lib/tests/research_validation.rs
    - biscuit-file/lib/tests/l1/resolution_context.rs
    - claudine/gen/Cargo.toml
    - claudine/gen/tests/l1/context_construction_guard.rs
    - claudine/gen/tests/l1/main.rs
    - darkmatter/cli/tests/common/context_guard.rs
    - darkmatter/cli/tests/common/source_scan.rs
    - darkmatter/cli/tests/l1/context_construction_guard.rs
    - darkmatter/cli/tests/l1/main.rs
    - darkmatter/dmls/tests/l1/context_construction_guard.rs
    - darkmatter/lib/Cargo.toml
    - darkmatter/lib/src/markdown/compose/context/capture/repo.rs
    - darkmatter/lib/src/markdown/compose/expression/ctx.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/executor.rs
    - darkmatter/lib/tests/l1/context_construction_guard.rs
    - darkmatter/lib/tests/l1/semantic_results_never_persist.rs
    - messenger/cli/Cargo.toml
    - messenger/cli/src/research.rs
    - messenger/cli/tests/context_construction_guard.rs
    - messenger/cli/tests/research_cli.rs
    - messenger/lib/Cargo.toml
    - messenger/lib/tests/context_construction_guard.rs
    - darkmatter/lib/src/markdown/types.rs
    - darkmatter/lib/src/markdown/compose/context/report.rs
    - darkmatter/lib/src/markdown/compose/toc_linking/types.rs
    - darkmatter/lib/src/markdown/errors/blocks.rs
    - darkmatter/lib/tests/common/entry_point_parity/mod.rs
    - darkmatter/lib/tests/l1/entry_point_parity.rs
    - darkmatter/lib/tests/l1/error_snapshots/toc_linking.rs
    - darkmatter/lib/tests/l1/error_snapshots/snapshots/l1__error_snapshots__markdown_error__schema_validation_format_failure_renders_block.snap
    - darkmatter/cli/Cargo.toml
    - darkmatter/cli/tests/l1/entry_point_parity.rs
    - darkmatter/dmls/src/overlay/directives.rs
    - darkmatter/dmls/tests/l1/entry_point_parity.rs
    - darkmatter/dmls/src/providers/definition.rs
documentation:
    - darkmatter/docs/inline/preflight-checks.md
    - darkmatter/lib/README.md
    - darkmatter/docs/topics/magic-paths.md
    - darkmatter/docs/topics/schemas/definition.md
    - darkmatter/docs/inline/text-replacement.md
    - biscuit-file/docs/topics/file-references.md
    - sniff/docs/sniff-library-architecture.md
    - darkmatter/docs/topics/compose-requests.md
    - darkmatter/dmls/docs/diagnostics.md
    - darkmatter/dmls/docs/features.md
    - darkmatter/dmls/README.md
    - darkmatter/docs/dependencies.md
    - darkmatter/dmls/docs/file-references.md
    - darkmatter/docs/inline/schema-validation.md
    - darkmatter/docs/structs/Markdown.md
    - darkmatter/docs/topics/caching.md
    - darkmatter/docs/topics/darkmatter-expressions.md
    - darkmatter/docs/inline/shell-expansion.md
    - darkmatter/docs/inline/toc-linking.md
    - darkmatter/docs/topics/file-referencing.md
    - darkmatter/docs/topics/transclusion.md
    - darkmatter/docs/topics/dmls.md
    - darkmatter/docs/composition/index.md
    - darkmatter/docs/darkmatter-compose-pipeline.md
    - darkmatter/docs/errors/README.md
    - darkmatter/docs/lsp/architecture.md
    - darkmatter/docs/lsp/features.md
    - darkmatter/docs/errors/file-reference-failures.md
completed_phase: 8
implemented: true
---

# Implementation Log for 2026-09-30-file-refs-use-magic (8 phases)

## Phase 1

### Wave 1

#### Reproduce Incident 1

From `claudine/` with the installed `md` (`md 0.1.0`, `~/.cargo/bin/md`):

```text
$ md compose docs/use-claudine/SKILL.md
⤫ TransclusionError: file reference failure
┃
┃ `^` repository reference requires a repository containing reference CWD
┃ `/Volumes/coding/wt/rusty-biscuit/fix-magic-globs/claudine/docs/use-
┃ claudine`
┃
┃ Check sigil usage: `@` magic, `&` repository root, `^` repository-scoped.
```

#### Spike S1: builder-failure inputs (recorded as R11 in the plan)

Ran with a throwaway crate (`/tmp/s1-spike`, path dependencies on
`biscuit-file` and `sniff`; nothing added to the repository).

- (a) `FileResolutionContext::from_snapshot("relative/dir", ..)` fails
  `validate()` with `RelativeContextDirectory { anchor: RequestDirectory }`.
- (b) A request directory inside a repository plus an opening reference
  through `~` or `{{VAR}}` that resolves outside it fails
  `validate()` after `for_source_reference` with
  `RepositoryRootNotContainingSource`. A request directory in `VAULT` with
  the repository root elsewhere fails the same way. With no repository the
  `~` anchor becomes the tree and validation passes.
- (c) A syntactically corrupt `.git/config` (`[core` unterminated) makes
  `GitRepo::discover` and `find_git_root` return `Err`. Rewriting the config
  as valid repairs it. Every other broken state tried (a `.git` file naming
  a missing or empty gitdir, a garbage `.git` file, an empty `.git`
  directory, a missing `HEAD`, an unreadable `.git` on Unix) is reported as
  **no repository**, not an error. A malformed `HEAD`, format version 99, and
  an unknown extension all discover successfully.
- Phase 4 consequence: DMLS must see a `.git/config` change to drop the
  failed entry (see `message_to_agent` in the spec).

#### Spike S2: matrix and guard topology (recorded under R6 in the plan)

Ran with temporary edits to `claudine/cli` (`Cargo.toml`
`source-inputs`, `tests/l1/s2_spike.rs`) and `darkmatter/dmls`
(`tests/l1/s2_spike.rs`), all reverted afterward.

- A `#[path]`-included copy of `darkmatter/cli/tests/common/source_scan.rs`
  compiled and ran in both `dmls::l1` and `claudine-cli::l1`. The darkmatter
  `l1` binary already does this (`semantic_results_never_persist.rs`).
- With only the `#[path]` include, the planner **refused** the declaration:
  `RealWorkspaceTestInputTests.test_every_declared_source_input_is_read_by_its_declarer`
  failed with "claudine-cli declares darkmatter/cli/tests/common/source_scan.rs
  but no L1 test of it names the path". The index walks `#[path]` modules but
  does not count them as references.
- Adding `include_str!("<same path>")` makes the test pass. At module level
  the planned cell is `claudine-cli ubuntu-latest L1 (binary_id(claudine-cli::l1))`.
  Inside the test function it narrows to
  `(binary_id(claudine-cli::l1) & test(=s2_spike::s2_spike_path_include_compiles))`.
  The R6 alternative: each declaring test spells the shared file with an
  in-function `include_str!` next to the `#[path]` include.
- `claudine-cli::l1` holds both completion and composition tests. Several
  `compose_*` modules there are `#[cfg(unix)]`.

### Wave 2

#### Regression tests (written first, confirmed red before the fix)

- `darkmatter/cli/tests/l1/compose_transclusion.rs::test_compose_repository_sigils_from_a_nested_document`.
  This is a git repository built with `CliProcessFixture::initialize_repository_at`.
  `md` is launched from `repo/pkg` (`ambient_context`) with the relative
  argument `docs/guide/doc.md`. The document holds `::file &amp-target.md`,
  `::file ^caret-target.md`, and `::file ^pkg/docs/cli/index.md`, which is
  the shape of the original report. The test asserts exit 0, all three
  bodies in stdout, no leaked `::file`, and no "requires a repository
  containing reference CWD" message. Before the fix it failed with exactly
  that message for `&`.
- `darkmatter/lib/tests/l1/preflight_repository_sigils.rs` uses the same
  layout through `gix::init`. Each test runs two request shapes that carry
  no file-resolution context: `ComposeOptions::for_document(launch_dir, ..)`,
  which is what `md` builds (the source is inside the request repository),
  and `ComposeOptions::new()` (the source is outside the process's
  repository). The tests:
  - `compose_preflight_resolves_repository_sigils_from_a_nested_document`
    checks the approval set holds each target's command and the graph edges
    resolve to the three canonical target paths, in order;
  - `collect_shell_commands_resolves_repository_sigils_from_a_nested_document`
    checks the entries match the targets' commands, in order;
  - `compose_preflight_approvals_resolves_repository_sigils_from_a_nested_document`
    checks the pre-approved set and the discovered count;
  - `compose_preflight_reports_a_missing_repository_target_as_not_found`
    is the negative case: a missing `&` target still fails, names the
    target, and is no longer the missing-repository precondition.

  All four failed before the fix with "`&` repository reference requires a
  repository containing reference CWD".

#### Preflight fix

- `ComposeOptions::prepare_root(&mut self, &Markdown)` (`pub(crate)`,
  `compose/context/options.rs`) runs, in order, `extend_context_for`,
  `establish_repository_observation`, and `ensure_file_resolution_context`.
  `prepared_root(&self, ..)` is the borrowing form, which clones.
- `run_compose_pipeline` calls `prepare_root` in place of the three inline
  calls.
- Pre-flight calls `prepared_root` at the top of `collect_effects` and
  `collect_frontmatter_shell_commands`. **Departure from the plan's wording:**
  the plan lists `compose_preflight`, `compose_preflight_approvals`, and the
  public `collect_*` entries. Every one of those reaches the walk through
  `collect_effects` (`compose_preflight_approvals` goes through
  `compose_preflight`; `collect_shell_commands` goes through
  `collect_shell_commands_with_graph`), except
  `collect_frontmatter_shell_commands`, which is prepared separately. So two
  call sites cover all five entries, plus the pipeline's
  `validate_pre_approved` and `nested.rs`'s use of it.
- The ambient repository observation lives behind an `Arc<OnceLock>` that
  clones share, so preparing a clone in pre-flight does not add a second
  discovery when the pipeline later runs with the caller's options.
- Doc drift fixed: `establish_repository_observation`'s doc said the "root
  pipeline entry" calls it. It now names `prepare_root`.

#### Docs and skill

- `darkmatter/docs/inline/preflight-checks.md`: collection resolves
  transclusion targets as composition does, so `&` and `^` reach the same
  file from any launch directory.
- `.claude/skills/darkmatter/SKILL.md`: pre-flight is a root entry and
  prepares through `prepare_root`.
- `.claude/skills/darkmatter/compose.md`: the "fixed by the root pipeline
  entry" line now names `prepare_root` and both of its callers.

#### Incident check after the fix

Running `target/debug/md compose docs/use-claudine/SKILL.md` from `claudine/`
with the worktree build exits 0 and prints 88 lines with no unresolved
`::file`.

#### Gates

- `just test` (darkmatter): 8749 passed, 12 skipped, exit 0. The four new
  library tests and the new CLI test ran in it.
- `just lint` (darkmatter): exit 0.
- `just test` (claudine, a downstream caller of `compose_preflight`): 8072
  passed, 9 skipped, exit 0.
- No pre-existing failures seen.
- No cross-OS run in this phase. The change adds no path comparison and no
  `#[cfg]` code, and the tests compare canonicalized paths on both sides.
  Windows evidence is gathered in Phase 8 under the plan's Definition of
  Success.
- Not done, per the phase prompt: Commit 1 and `git verify-commit`. The plan's
  Checkpoint asks for them; they are left to the separate commit process.

#### Requirement-to-test mapping

| Requirement | Test |
|---|---|
| `md compose` resolves `&` from a nested directory | `compose_transclusion::test_compose_repository_sigils_from_a_nested_document` |
| `md compose` resolves `^` (bare and `^pkg/...`, the incident shape) | same test |
| `compose_preflight` collects `&`/`^` targets with no context (both request shapes) | `preflight_repository_sigils::compose_preflight_resolves_repository_sigils_from_a_nested_document` |
| `collect_shell_commands` likewise | `preflight_repository_sigils::collect_shell_commands_resolves_repository_sigils_from_a_nested_document` |
| `compose_preflight_approvals` likewise | `preflight_repository_sigils::compose_preflight_approvals_resolves_repository_sigils_from_a_nested_document` |
| A missing `&` target still fails, as not-found, not as the repository precondition | `preflight_repository_sigils::compose_preflight_reports_a_missing_repository_target_as_not_found` |

The Input Robustness Matrix does not apply (R15): no file-format reader was
added or changed.

## Phase 2

### Wave 3

#### biscuit-file accessor (subagent)

- `FileReferenceError::resolution_failure()` (`biscuit-file/lib/src/file_reference/resolve.rs`)
  delegates to the private `classify_error`, which stays the one place the
  classification is decided.
- **Departure from the plan:** the plan asks for one test per
  `ResolutionFailure` variant. `classify_error` never returns `NoMatch` (a
  no-match is a result, not an error), so there is no input for it. Four
  variant tests plus `resolution_failure_never_no_match` were added instead:
  `resolution_failure_invalid_reference`, `resolution_failure_missing_context`,
  `resolution_failure_io`, `resolution_failure_unsupported_remote`.
- Also made public (not in the plan): `biscuit_file::capture_env()`, the
  environment reader `FileResolutionContext::new` uses. `RequestSnapshot::from_process()`
  needs it so R2's "same helpers" holds (non-UTF-8 variables skipped,
  `USERPROFILE` via `home_dir()`). Documented in
  `biscuit-file/docs/topics/file-references.md` and the `biscuit-file` skill.
- `just test` (1016 passed) and `just lint` in `biscuit-file/`: exit 0.

#### sniff manifest list (subagent)

- `sniff::filesystem::repo::PACKAGE_MANIFEST_FILE_NAMES` replaces the private
  `MANIFEST_FILES` in `glob.rs`; the membership check in `manifest_index.rs`
  (~691) uses it.
- **Departure:** the two `manifest_index.rs` sites at ~459 and ~501 map each
  name to a `ManifestKind` with a `match` on literals and cannot be written
  over a slice. They were left, and
  `manifest_index::tests::manifest_index_recognizes_every_package_manifest_file_name`
  keeps the ~501 copy in step with the constant (the ~459 walker copy is not
  covered). `glob::tests::package_manifest_file_names_are_pinned` pins the
  contents and order.
- `just test` (3121 passed, 32 skipped) and `just lint` in `sniff/`: exit 0.

#### `RequestSnapshot` and `ContextBuildError`

New module `darkmatter/lib/src/markdown/compose/context/request.rs`,
re-exported from `darkmatter::markdown::compose`:

- `RequestSnapshot::new(dir)` (no home, empty environment), `from_process()`
  (returns `io::Result`; the current-directory read can fail),
  `with_home`, `with_env`, `with_magic_root`, `with_magic_root_tier`
  (Claudine's `~/.claudine` roots need `MagicPathTier::User`; R2 named only
  the inferred form), `with_opening_reference`, `at_request_dir` (keeps home,
  environment, and roots; drops the opening reference, which was resolved for
  the old directory), and accessors.
- `ContextBuildError::{Discovery, Invalid}` with `request_dir()` and
  `resolution_failure()`. **Ruling made here:** a discovery failure classifies
  as `MissingContext`, the class biscuit-file gives its own `Git` errors, so
  the two never disagree.

### Wave 4

#### Builder

`build_resolution_context(&RequestSnapshot)` follows R3: `from_snapshot`,
discovery, scope catalog (which also syncs the launch `@` scope), the
snapshot's extra roots, `for_source_reference` for an opening reference,
`validate()`, then one `tracing::debug!` with `request_dir`, `base_dir`, and
`base_dir_origin`.

- Discovery runs through the same `Repo`-group capture `ctx.repo*` uses
  (`capture_runtime_context_for_groups`), so the observation can be shared.
  With only that group requested, the capture's sole `git` diagnostic is the
  discovery failure, which becomes `ContextBuildError::Discovery`. A relative
  request directory skips discovery (validation rejects it with a typed error
  instead of discovering from the process directory).
- A topology-walk failure under a found repository still degrades to "no
  packages", as before. Only the Git discovery itself is an error.
- `capture_file_resolution_context` and its helper are deleted, with the
  re-exports. Its library callers (`schemas/clean.rs`, `util.rs`
  `source_link_context`'s fallback, tests) and the CLI's
  `schema validate` / `schema triggers` now call the builder.
- Magic paths: `ComposeOptions::magic_paths`, `with_magic_path`, and
  `TransclusionOptions::magic_paths` are deleted (R3). The cache and graph
  identities already encode the context's tier-aware registrations, so their
  explicit `magic_paths` fields were dropped and the identity tests now vary
  the context's roots. `ResolutionContext::magic_paths` (expression side) and
  `document_resolution_context`'s parameter are kept for the no-context
  fallback, which Phase 5 removes; every caller now passes `&[]`.

#### `ComposeRequest`

- `ComposeRequest { options, context }`. `prepare(options, &snapshot)` and
  `with_context(options, context)` (which validates) both return
  `Result<_, ContextBuildError>`. `options()`, `context()`, and
  `map_options()` (for settings decided after preparation, such as
  pre-flight's approval set) are public; `root_options()` (options with the
  context attached) is crate-private.
- `prepare` keeps "one discovery per request" (the `lazy_roots` tests pin
  it): a `ComposeOptions::new()` context that captured only date/time is
  re-anchored on the request directory (`ComposeContext::with_anchor`); the
  request's existing observation is reused when it contains the request
  directory; otherwise the builder's discovery is installed as the request's
  observation (`CurrentAuthority::adopt_ambient_repository`) when the request
  observes its own repository from that directory.
- `ensure_file_resolution_context`, `prepare_root`, and `prepared_root` are
  deleted. `establish_repository_observation` became the module-private
  (`pub(super)`) `establish_request_repository`, called only by
  `ComposeOptions::for_document` and `ComposeRequest`.
- `ComposeOptions::with_file_resolution_context` is now `pub(crate)`: outside
  the crate the only way to attach a context is `ComposeRequest`. The internal
  `Option` field stays until Phase 5, which already lists
  `file_resolution_context()` and `source_file_resolution_context`.
  **Departure from R4's wording** ("`ComposeOptions` loses its `Option`
  context field"): the field is now internal transport between a request and
  its derived child options; Phase 5 makes it required.
- Public entry points on `&ComposeRequest`: `Markdown::compose_with`,
  `compose_preflight`, `compose_preflight_approvals`, `collect_shell_commands`,
  `collect_shell_commands_with_graph`, `collect_frontmatter_shell_commands`,
  `transclusions_with_options`, `ReferenceGraphOptions::with_compose`,
  `shell_expansion::execute_directive`, and `execute_resolved_shell_values`.
  `Markdown::compose()` and `compose_mut()` are deleted.
- Not moved, recorded as departures: `normalize_links` is `pub` inside the
  crate-private `link_normalization` module and `link_resolve` is
  `#[cfg(test)]`; neither is reachable from outside the crate, and both run on
  derived child options inside the pipeline. `validate_pre_approved` is
  `pub(crate)` and also runs on child options (`nested.rs`); it now calls the
  internal `collect_effects` / `frontmatter_shell_commands`.
- Internal inline passes that compose over derived options (pre-flight's
  inline pass, reference-graph content preparation) use the new crate-private
  `Markdown::compose_with_options`, and `ReferenceGraphOptions::from_compose_options`
  replaces their `with_compose` calls. `run_compose_pipeline(&ComposeRequest)`
  delegates to `run_root_pipeline(options)`.
- Added `ComposeOptions::source()` (public getter) so test helpers outside the
  crate can choose a request directory.

#### `ctx.*` from the request

- `ComposeRequest` aligns the `ComposeContext` environment with the request
  context's (`align_context_environment`), so `{{ env.X }}`, `ctx.agent`, and
  `ctx.model` read the snapshot. Extension never overwrites the environment
  (`extend_missing_with` keeps `inner.env`), so later group captures keep it.
- `ResolutionContext::agent()` falls back to the request context's `AGENT`
  before the process.
- Home on the request path already came from the context
  (`options.rs` `expression_resolution_context`), so no change was needed
  there; the `dirs::home_dir()` fallbacks now run only without a context.
- **Residue left for Phase 5** (its "anything left in `runtime.rs`" item):
  `ComposeContext::capture()` / `capture_minimal()` still read the current
  directory and environment, and `capture/mod.rs`'s capture still calls
  `std::env::vars()`. On a request path the environment is replaced and a
  minimal context re-anchored, so neither reaches `ctx.*`; non-request users
  (DMLS validation, `local_expression_resolution_context` without a context)
  still depend on them.
- **Not changed, recorded:** `conditions.rs:379` (`env.*`) belongs to the
  standalone `evaluate_condition_against(data, work_dir)` shortcut API, which
  has no request. It is not on the compose path. Phase 5/6 must classify it.
- `capture/agent.rs`: the two `populate_agent` tests now pass an environment
  map instead of mutating the process environment. The third
  (`capture_runtime_context_includes_agent_group`) still mutates it under
  `serial_test`.

#### Migrating darkmatter's tests

- About 900 call sites in `darkmatter/lib` were moved with a balanced-paren
  rewrite onto two helpers that reproduce the request directory the deleted
  fallback chose (a file source's directory, else the context's absolute
  anchor, else the process directory), with process home and the options'
  own context environment:
  - unit tests: `crate::markdown::compose::test_request(options)`
    (`context/request.rs` `test_support`), which keeps a context the options
    already hold through `with_context`;
  - integration tests: `crate::request_support::{request, request_at}`
    (`lib/tests/request_support/mod.rs`, declared in `l1/main.rs`).
- Tests whose subject is the context moved to `ComposeRequest::with_context`
  (`file_tree_roots`, `link_interpolation_integration`,
  `reference_integration::reference_options`, `unknown_identifier_warning`).
- Behavior surfaced by the migration, now asserted rather than masked: a
  reference graph whose request directory is in this repository rejects a
  document in a temp directory (`RepositoryRootNotContainingSource`) instead
  of silently re-anchoring on the document. Graph tests now anchor the
  request at their fixture (`request_at`).
- `schema_validation::undecided_root_union_resolves_a_caller_file_from_the_launch_area`
  now prepares its request at the launch directory, which is what it tests.
- `transclusion::resolver::resolves_magic_path_prepended` no longer changes
  the process directory; it builds its context from a snapshot root.
- `test_compose_mut_modifies_in_place` was deleted with `compose_mut`.
- Benches and the level2 terminal support were moved to `ComposeRequest`.
- `lib/tests/l1/declined_path_transclusion.rs` is `#[cfg(windows)]`; macOS
  never compiled it. The Windows cross-check caught it.

#### New tests

| Requirement | Test |
|---|---|
| `new()` reads nothing from the process | `context::request::tests::a_new_snapshot_reads_nothing_from_the_process` |
| `at_request_dir` keeps home, env, roots; drops the opening | `context::request::tests::at_request_dir_keeps_home_environment_and_roots_but_not_the_opening` |
| One `debug` event naming request dir and `base_dir` origin (scoped `tracing_test` subscriber) | `context::request::tests::a_successful_build_emits_one_debug_event` |
| A failed build emits none | `context::request::tests::a_failed_build_emits_no_debug_event` |
| AC 6 (a): relative request directory rejected, by builder and `prepare` | `request_context_builder::a_relative_request_directory_is_rejected` |
| AC 6 (b): opening reference through `~` and `{{VAR}}` outside the repository rejected; control builds | `request_context_builder::an_opening_reference_outside_the_request_repository_is_rejected` |
| (b) negative control: no repository, same opening builds | `request_context_builder::an_opening_reference_builds_when_the_request_has_no_repository` |
| R11 (c): corrupt `.git/config` is `Discovery`, `MissingContext`; repair builds | `request_context_builder::a_corrupt_git_config_is_a_discovery_error_until_repaired` |
| Magic-path finding: a snapshot root resolves `@` in the context and through `compose_with`; without it, nothing | `request_context_builder::a_snapshot_magic_root_resolves_an_at_reference` |
| `{{ env.X }}`, `ctx.agent`, and a `{{X}}` file reference read the snapshot while the process lacks `X` | `request_context_builder::an_expression_and_a_file_reference_read_the_same_snapshot_environment` |
| A transclusion's `{{X}}` target reads the snapshot | `request_context_builder::a_transcluded_variable_reference_reads_the_snapshot_environment` |
| `with_context` validates | `request_context_builder::with_context_rejects_an_invalid_context` |
| One discovery per request survives preparation | existing `compose::tests::lazy_roots::ambient_repository::*` (unchanged, passing) |
| Incident 1 stays fixed | existing `preflight_repository_sigils::*` (now through `request_support`) and `compose_transclusion::test_compose_repository_sigils_from_a_nested_document` |

The Input Robustness Matrix does not apply (R15).

#### `md` CLI (compile bridge only; Phase 3 Track A owns the redesign)

- `md compose`: with `--set`, the request is `with_context` over the
  document context it already derived; without `--set` it is `prepare` at the
  document's directory (the launch directory for stdin) with
  `RequestSnapshot::from_process()`. That reproduces the deleted fallback.
  Validation, pre-flight, and compose share the request, and pre-flight's
  approvals are applied with `map_options`.
- `schema validate` / `schema triggers` find their trigger boundary through
  the builder. In `schema validate` a build failure is reported as the file's
  `ParseError` outcome (no better outcome exists yet).
- `from_process()` is called inside `compose.rs`, not once in `main`; Phase 3
  moves it.

#### Docs and skills

- New `darkmatter/docs/topics/compose-requests.md` (snapshot, builder steps,
  failure table, one environment per request, Mermaid flow).
- `darkmatter/docs/topics/magic-paths.md` rewritten for snapshot roots; the
  Claudine example is marked **planned** (Claudine still builds its own
  context until Phase 3).
- Stale `compose_with(options)` / `.compose()` examples fixed in
  `darkmatter/lib/README.md`, `docs/topics/schemas/definition.md`,
  `docs/inline/text-replacement.md`, `docs/inline/preflight-checks.md`.
- `darkmatter` skill (`SKILL.md`, `compose.md`) describes `ComposeRequest`,
  the test helpers, and `compose_with_options`; `prepare_root` is gone.

#### Gates

- `just test` (darkmatter: lib, cli, dmls): 8757 passed, 12 skipped, exit 0.
- `just lint` (darkmatter): exit 0.
- `cargo test --doc -p darkmatter`: 189 passed, 1 failed. The failure,
  `compose::expression::evaluate` (calls an undefined `evaluate_expr`), is
  pre-existing; the file was not touched. Four stale doctests this phase broke
  were fixed. Doctests are not part of `just test`.
- `just cross-check darkmatter --os windows` (filtered to the new and changed
  tests): first run failed to compile `declined_path_transclusion.rs`
  (Windows-only), then three new tests failed on verbatim `\\?\` spellings
  from `canonicalize`. Fixed with `biscuit_file::canonicalize_simplified` and
  portable `{{VAR}}` values; final run 12/12 passed. Linux: 18/18 passed.
  (The `just` wrapper mangles `-E '…(…)…'`; plain substring filters through
  `./scripts/cross-check.sh` work.)
- Not done: the full darkmatter suite on Windows/Linux (Phase 8).

#### Downstream compile errors (Phase 3's input)

messenger, messenger-cli, and claudine-gen still compile. `claudine` does
not, so `claudine-cli` is not checked yet. Per R16, Phase 2 cannot stand
alone, so Phases 2 and 3 land as one commit. `cargo check -p claudine
--all-targets` errors (rustc stops at these; more will follow in
`claudine-cli`):

- lib: `composition/lifecycle/executor.rs:337,344`, `composition/mod.rs:213`,
  `composition/preflight.rs:82,139`,
  `composition/prepare.rs:341,567,678,883,929`,
  `system_prompt/prepare.rs:173,198`. All are
  `with_file_resolution_context` (now crate-private) or an entry point that
  now takes `&ComposeRequest`.
- lib tests: `composition/preflight/tests.rs:324,384,431`,
  `composition/runtime_state/tests.rs:251`, `composition/sequence/tests.rs:761`,
  `invocation_context/tests.rs:306` (`capture_file_resolution_context`).

#### Re-verification (2026-10-01)

A second Phase 2 run found every Phase 2 task already checked and the work
present in the tree, so nothing was re-implemented. Gates re-run on macOS:
`just test` in `darkmatter/` gave 8757 passed and 12 skipped, and `just lint`
exited 0. The plan, log, and spec frontmatter were already complete, so they
were left unchanged.

## Phase 4

Phase 4 ran on top of the committed Phase 2 work and the Phase 3 commits
already on the branch (`14a6bf301` claudine-gen, `b15db8151` md CLI,
`003ffdd3d` messenger). Phase 3's plan tasks are still unchecked and
Claudine (Track B) does not compile yet; nothing in this phase depends on it.

### Wave 7: snapshot and cache

- `RunOptions` gained `snapshot: RequestSnapshot` and lost `Default`
  (`RunOptions::new(snapshot)`). `main.rs` calls
  `RequestSnapshot::from_process()` once, after `--gen-corpus` and before
  `--bench-index` and the server, and passes it to both.
- New `dmls/src/context.rs`: `RepositoryContexts` caches one
  `build_resolution_context` result per key, **failures included**. The key is
  `biscuit_file::find_git_root(folder)`, or the folder itself when there is no
  repository or discovery fails (the builder then reports the same failure
  for that folder). A build uses `snapshot.at_request_dir(key)`, so the request
  directory is the repository root. `for_document(path)` derives with
  `for_source`. A folder → key memo avoids rediscovering per request.
- `DocumentResolution` carries the derived context or a `ContextFailure`
  (`Build(ContextBuildError)` or `UntitledWorkspace`) plus a generation
  number for cache keys. `DocumentContext.resolution` carries it to every
  provider, with `file_context()` and `resolve_reference(raw)` helpers.
- **Departure (counter):** the plan asked for a `work-counters` counter. A
  feature-gated counter would need a self dev-dependency or a new feature in
  every recipe, and a test gated on a feature that `just test` forgets would
  silently vanish. `context_build_count()` is an always-compiled process-wide
  `AtomicUsize`; tests assert deltas (nextest runs one test per process).
- The fixture (`tests/common/mod.rs`) passes `LspFixture::fixture_snapshot`
  (workspace as request dir, `HOME` = `<workspace>/home`, empty environment);
  `start_with_snapshot` lets a test supply its own.

### Wave 8: schema validation, graph, providers

- `overlay/schema.rs::assemble` takes the document's context and calls
  `DarkmatterSchemas::with_file_resolution_context`;
  `with_file_ref_fallback_dir` is gone. The overlay's schema cache key mixes
  in the resolution's generation, so a rebuilt context re-validates.
- **Library fix found here (darkmatter lib):** when a `file(eager)` value
  failed validation, `validate.rs` re-ran resolution *without* the validator's
  request context to build the diagnostic, so a failing `&missing.md` would
  have been reported as a missing-context failure. `FileRefAnchors` now carries
  the context and the re-run uses `resolve_file_reference_in_context`. The
  context-free `resolve_file_reference` is now `#[cfg(test)]` (only its own
  tests call it).
- R5 darkmatter row, schema half: `FileReferenceDiagnostic::ResolutionFailed`
  gained `failure: ResolutionFailure`, and the enum gained
  `resolution_failure()`. All 8 construction sites in
  `compose/schema_validation.rs` pass the class from their error (or the
  fixed class for their non-error cases). One Claudine pattern
  (`composition/error/render/mod.rs:289`) needed `{ raw, .. }`; this was the
  only Claudine edit and it adds no new compile error to Claudine's existing
  Phase 3 list.
- DMLS diagnostics carry R5's `data: {"resolution_failure": "<Class>"}` (the
  `Debug` variant name) on `dm.links.broken_path`,
  `dm.transclusion.broken_path`, `dm.schema.invalid_file_reference`, and
  `dm.context.build_failure`. `LinkDiagnostic::BrokenPath` now carries the
  class (`NoMatch`, or why planning failed).
- Graph: `WorkspaceGraph::build`/`build_with_roots`/`assemble` take
  `&dyn DocumentContexts`, and `WorkspaceIndex::new`/`from_indices` hold an
  `Arc<dyn DocumentContexts>`. Links, transclusions, `$schema`, and file uses
  resolve to the **first `candidate_plan` path that is an indexed document**:
  the graph stays disk-free, and unsaved open buffers count. Contexts are
  fetched once per document folder per rebuild. `diagnose_unresolved` takes
  the document's context. `NoContexts` (resolve nothing) and `FixedContext`
  (one request context for every document) are the non-server sources.
  `WorkspaceIndex::relink()` rebuilds after contexts drop.
- Providers on the context: DSL `resolve_local_path` (hover, definition,
  transclusion links, transclusion diagnostics) now returns a
  `ReferenceTarget` from `resolve_in_context`; frontmatter `nav_targets`
  navigates to the found file, else the first planned candidate (as
  `md compose` absolutizes a link to a missing file); the create-missing-file
  code action creates at the first candidate; anchor completion resolves its
  path through the graph-style candidate plan. `normalize_join` had no
  callers left and was **deleted**, with its test.
- R12: the `trigger_boundary` doc comment claimed the Git root narrows the
  boundary; the code uses the nearest workspace folder only. The comment was
  corrected (code is right).

### Wave 9: invalidation, failure diagnostic, untitled buffers

- Watchers: `watch::context_input_globs()` adds `**/<name>` for each of
  sniff's `PACKAGE_MANIFEST_FILE_NAMES` plus `**/.git/config` (R11's repair
  path); 4 → 9 watchers, 2 → 7 without include globs. `is_context_input`
  keeps those paths out of the document index (before this, every watched
  path was handed to `set_document`).
- Invalidation rule (`RepositoryContexts::invalidate`): drop every entry
  keyed at an ancestor of the changed path, plus, for a path inside `.git/`,
  every entry keyed inside that repository (so a repaired `.git/config` drops
  the failure cached under the document's folder). The folder memo is cleared
  whenever anything drops or a `.git` path changes.
- `apply_watched_changes` invalidates for every event, then re-indexes only
  non-context-input, non-open paths. `reload_config` clears every entry
  (relinking once, folded into the reload's own rebuild).
  `rescan_workspace` (`&mut self` now) returns the changed document paths
  from `reconcile_disk` (which now returns `Vec<PathBuf>`) plus manifests
  whose xxHash changed (`scan_manifests` / `changed_manifests`, baseline taken
  at startup for rescan clients only), and invalidates them.
- **Ordering bug found and fixed:** clearing contexts at the top of
  `reload_config` first published diagnostics with the *old* config, which
  `lsp_session::config_reload_reindexes_wiki_roots` caught. There is now one
  refresh, after the reload.
- Failure diagnostic: `providers/diagnostics.rs` publishes one ERROR at 0:0,
  code `dm.context.build_failure`, source `darkmatter.context` (both new in
  `codes.rs`), message `file references are not resolved in this document
  (<Class>): <error naming the directory>`. The build logs once at `error`
  (`directory`, `failure` fields); a cached failure is not logged again.
  With a failure: graph edges, document links, definition, hover targets,
  transclusion diagnostics, and file-value problems all return nothing.
  **Departure:** the library still evaluates file formats during validation
  without a context; DMLS *suppresses* their problems for that document
  rather than preventing the evaluation. Phase 5 (required context in the
  library) removes the no-context path.
- Untitled buffers: `with_document` accepts `untitled:` URIs. The buffer is
  analyzed at `<repository root>/<buffer name>` and uses that repository's
  built context (cwd = root) when the folders lie in exactly one repository;
  otherwise it carries `ContextFailure::UntitledWorkspace` (class
  `MissingContext`). Untitled buffers are not added to the graph.

### Wave 10: tests

| Requirement | Test |
|---|---|
| AC 8: two documents and the startup index share one build; workspace folder above the repository; `&` anchors at the root | `repository_contexts::documents_in_one_repository_share_one_context_built_at_its_root` |
| AC 8: request dir = repository root, derived cwd = document folder | `context::tests::documents_in_one_repository_share_one_build` |
| AC 8: no repository → own folder; `&` there is `MissingContext` | `repository_contexts::a_document_in_no_repository_resolves_against_its_own_folder` |
| AC 9: `&`, `^`, `@` (snapshot magic root) in schema validation (`file(eager)`), document links (frontmatter, transclusion, Markdown links), definition, code action | `repository_contexts::repository_sigils_resolve_in_every_feature` |
| AC 9 (graph): `&` link, transclusion, and file use resolve; none without a context | `graph::arena::tests::test_repository_sigils_resolve_through_the_document_context` |
| Broken link carries its class | `graph::arena::tests::test_unplannable_link_reports_its_failure_class` |
| AC 12: watched client, only the manifest named | `repository_contexts::a_watched_manifest_change_rebuilds_the_repository_context` |
| AC 12: rescan client, no notification | `repository_contexts::a_rescan_detected_manifest_change_rebuilds_the_repository_context` |
| AC 12: configuration change drops every entry | `repository_contexts::a_configuration_change_drops_every_context` |
| AC 12: snapshot `HOME`, not the process's | `repository_contexts::home_comes_from_the_startup_snapshot` |
| AC 13: one 0:0 error with code, class, directory; no links/definition/validation; clears after repair | `repository_contexts::a_failed_context_is_one_diagnostic_and_no_resolution_until_repaired` |
| AC 13: the log line, once per cached failure | `context::tests::a_failed_build_logs_one_error_naming_the_directory_and_class` |
| Invalidation rule | `context::tests::invalidation_drops_ancestor_and_owned_entries_only`, `a_git_config_change_drops_the_folder_entries_of_its_repository`, `a_corrupt_git_config_is_a_cached_failure_until_invalidated` |
| AC 14: one repository across two folders | `repository_contexts::an_untitled_buffer_uses_the_single_repository_across_the_workspace_folders` |
| AC 14: two repositories / none | `repository_contexts::an_untitled_buffer_fails_with_folders_in_two_repositories`, `…_in_no_repository`, `context::tests::untitled_buffers_need_exactly_one_repository` |
| Watchers and manifest scan | `workspace::watch::tests::test_registration_covers_include_globs`, `test_context_inputs_are_manifests_and_git_files`, `test_manifest_scan_reports_created_changed_and_deleted` |
| `reconcile_disk` reports exact paths | `graph::invalidate::tests::test_reconcile_disk_adopts_created_and_changed_drops_deleted` |
| `untitled:` name parsing | `router::tests::test_untitled_name_reads_only_untitled_uris` |

All new tests are L1 (no tier markers), in `dmls::l1` (declared in
`tests/l1/main.rs`) or the `dmls` lib unit tests; `just check-tier-coverage
darkmatter` reports nothing stranded. The Input Robustness Matrix does not
apply (R15: no file-format reader; manifests are hashed, never parsed).

Test-fixture facts learned (recorded in the `darkmatter` skill): sniff
recognizes a package only under a workspace manifest and counts a declared
member whose directory exists even without its own manifest, so the AC 12
fixture writes the workspace and member manifests together and notifies only
the member's. Inline `$schema` `file` is lazy (syntax only); validation of a
reference needs `file(eager)`.

Existing tests adjusted: unit tests that built graphs or contexts over
rootless `/w` paths now go through `context::test_support::abs` (a drive on
Windows), because a context rejects a relative directory. The first Windows
cross-check caught three frontmatter navigation tests that still used a
rootless path (recorded in the `os` skill, Windows path spelling item 14).
The mapping-only corpus baseline was re-blessed; the only change is the new
`data` payload on 11 diagnostics.

### Gates

- `cargo nextest run -p dmls --features effects-instrumentation`: 774 passed,
  6 skipped.
- `just test` in `darkmatter/`: 8778 run, 8770 passed, 8 failed, 12 skipped.
  **All 8 failures are pre-existing on `HEAD` (`003ffdd3d`), proven by running
  them in a clean `HEAD` worktree**, where the same 8 fail:
  - `darkmatter::l1 error_snapshots::{reference,transclusion}::file_reference_shows_hint`:
    insta snapshots not updated for the `failure:` row `b15db8151` added (R5).
    Both snapshots were accepted in this phase; they now pass.
  - `darkmatter-cli::l1`: `clean_schema::{test_no_trigger_schemas_disables_discovery,
    test_matching_trigger_schema_drives_quoting,
    test_save_shorthand_uses_default_schema_state_and_repairs,
    test_stdin_does_not_discover_repository_trigger_schemas}`,
    `compose_state_set::test_compose_shorthand_numeric_leading_key_is_treated_as_input_path`,
    `compose_transclusion::{test_compose_link_relative_same_repo,
    test_compose_link_transcluded_child}`. These are Phase 3 Track A (`md`
    CLI) behavior changes and are left to Phase 3.
- `just lint` in `darkmatter/`: `darkmatter`, `dmls`, `zed-dmls-cli`, and
  `check-zed` pass. `darkmatter-cli` fails on three pre-existing Phase 3
  clippy errors (`too_many_arguments` in `commands/frontmatter.rs:12` and
  `commands/hash.rs:29`, `ptr_arg` in `io/mod.rs:77`); this phase touched none
  of those files.
- `just test-l2` in `darkmatter/`: 18 + 69 + 3 passed.
- Downstream: `messenger`, `messenger-cli`, and `claudine-gen` compile
  (`--all-targets`). `claudine` fails with exactly Phase 2's recorded list.
- `just cross-check dmls --os windows` (full dmls L1, `effects-instrumentation`):
  first run 773/776, with the 3 rootless-path test failures described above.
  Rerun results follow below.
- Windows rerun after the fix (full dmls L1, `effects-instrumentation`, on
  `build-win-native`): **776 passed, 6 skipped**.
- `just cross-check dmls --os linux` (same scope, `build-linux`): **774
  passed, 6 skipped** (the two-test difference is Windows-only tests).
- Not run here: a Windows cross-check of the darkmatter library for this
  phase's two library edits (a new enum field and passing an existing
  context through). Both are platform-neutral, and Phase 8 runs the full
  per-package Windows cross-checks.

## Phase 5

Phase 5 started on top of the uncommitted Phase 4 work. **Phase 3 Track B
(Claudine) is still not done**: `claudine` does not compile against the
Phase 2 API (13 lib errors at the start of this phase), so Wave 12's Claudine
task cannot be done until it is. Wave 11 (darkmatter lib) and the
darkmatter-cli / dmls / claudine-gen / messenger fallout are done here.

### Wave 11: required context in the darkmatter library

#### Design: the pipeline runs on `ComposeRequest`

- Phase 2 left `ComposeOptions.file_resolution_context: Option<…>` (R4 said
  the field would go). `ComposeOptions` stays the public settings builder,
  and the field is **deleted**. Every pipeline-internal function that took
  `&ComposeOptions` / `ComposeOptions` now takes `&ComposeRequest` /
  `ComposeRequest`, which holds the context by value.
- `ComposeRequest` implements `Deref`/`DerefMut<Target = ComposeOptions>`, so
  stages read settings unchanged. Context-dependent views moved onto the
  request (`transclusion_options`, `source_file_resolution_context`,
  `expression_resolution_context`, `frontmatter_resolution_context`,
  `local_expression_resolution_context`, `with_accepted_source_file`,
  `extended_for`, `compose_cache_fingerprint`); the `ComposeOptions` halves are
  crate-private `*_in(&context)` helpers. Builder chains inside the pipeline
  use the crate-private `ComposeRequest::derive(|options| …)`.
- **Rename:** `ComposeRequest::context()` became `resolution_context()`.
  Through `Deref`, `context()` would have shadowed `ComposeOptions::context()`
  (the captured `ctx.*`), so every stage calling `options.context()` would
  have silently received the wrong type of context.
- `root_options()` is gone; the pipeline consumes the request itself.
  `Markdown::compose_with_options` (internal inline passes) takes a
  `ComposeRequest`.
- `ReferenceGraphOptions.compose` is a `ComposeRequest`; its `Default` (and
  `ReferenceValidationOptions`'s) are deleted, since neither had a context.
  `Markdown::transclusions()` (the ambient compatibility facade) and
  `options_with_reference_resolution_context` are deleted;
  `transclusions_with_options(&request)` remains.
- `FileTree::new(path, &request)` / `from_markdown(md, &request)` replace the
  `std::env::current_dir()` capture (`reference/file_tree/mod.rs:181`); the
  request's `ComposeContext` is extended for the document's `ctx.*` groups.

#### `schemas/`

- `DarkmatterSchemas::new(context)`, `CleanSchemaConfig::new(context)`; no
  `Default`, and `CleanSchemaConfig::with_file_resolution_context` is gone.
  `DarkmatterSchemas::with_file_resolution_context` remains as a
  **replacement** setter (one configuration, such as a CLI baseline, validating
  documents that each carry their own context); it is no longer `Option`.
- `EffectiveSchema`, `ImportEngine`, `rewrite.rs`, `format.rs`,
  `file_match.rs`, `resolve.rs`, and `validate.rs` take the context by value or
  reference. The public `resolve_schema*`, `resolve_yaml_schema*`,
  `rewrite_eager_file_values`, `triggers::scan`, `detect_schema`, and
  `detect_from_document` take it; their `*_in_context` twins were merged in,
  and `detect_from_document_with_context` was removed.
- **Structural validators (new).** A context-free validator used to resolve
  `file` values against the process CWD (`reference.resolve()`). Callers with
  no request — coercion probes, `example(...)` checks, suggestion lint — now
  use `build_structural_validator` / `ValidatorCache::structural_validator_for`:
  a `darkmatter-file` value is judged by `FileReference::new` syntax alone and
  `match()` admits every value, so they never touch the filesystem. The cache
  keeps structural and context-bound entries apart (`JudgedIn`). Diagnostics
  carry `FileRefAnchors::{Syntax, Resolved{..}}`.
- Ambient reads removed: `format.rs` `resolved_from` (now the base directory,
  else the context's `cwd`); `schemas/mod.rs` and `schemas/detect.rs`
  `base_dir_for` (a pathless document uses the context's `cwd`).
- **Departure: `with_file_ref_fallback_dir` is kept.** The plan said it had no
  effect on resolution. It has none on `darkmatter-file`, but the launch-area
  directory is a glob anchor inside `match()`'s `admits`, which the
  glob-reference feature replaces; deleting it now would change `match()`
  results for Claudine and `md compose`. Only the context parameter of
  `admits`/`match_keyword_factory`/`MatchKeyword` changed, as planned, and the
  handed-off `current_dir` fallback in `admits` stays.

#### `compose/` and `expression/`

- `ResolutionContext::new(context)` requires the context (cwd, repository,
  package area come from it). `magic_paths` and the `home_dir` field (with its
  `dirs::home_dir()` fallback) are gone; `agent()` reads only the context's
  `AGENT` (the process fallback is gone). `resolve_document_file_ref*` and
  `resolve_document_directory` take `&FileResolutionContext`;
  `path_projection` (incl. the `dirs::home_dir()` read at `:80`) and
  `path_display_components` too.
- `TransclusionOptions.file_resolution_context` is required.
  `ShellExpansionOptions` (a public settings struct) **lost** its context
  field instead; `resolve_policy_paths(opts, source, &context)` takes it, which
  also removed its two `current_dir` reads (a pathless source uses the request
  directory).
- `util::document_resolution_context` and `source_link_context` were deleted:
  with a required context both were `for_source`/`for_cwd`.
- `PortablePath` task: `link_normalization` always calls `with_ctx` with the
  source's derived request context; the no-context branch is gone.
  `link_resolve` likewise uses the request's derived context only.
- `conditions::evaluate_condition_against(expr, data, &context)`: `env.*`
  reads the context's environment (was `std::env::var`), read-side functions
  resolve through it.
- `runtime.rs`: the CWD-based `ComposeContext::capture()` was deleted (only
  tests called it); `capture_minimal()` is anchored at an empty path instead
  of `current_dir()`. `EffectiveStateBuilder::build()` without a supplied
  context now uses `capture_minimal()` (date/time only) instead of the deleted
  full CWD capture. Production callers all pass a context; Claudine's
  `wrap/sequence/task_run.rs:77` builder passes none and will see only
  date/time `ctx.*` there (recorded for the Claudine track).
- **Residue (not removed this phase):** `context/capture/mod.rs:108`
  `std::env::vars()` still seeds every ambient `ComposeContext` capture
  (`capture_for_dir/content/document`, `CtxLookup` group captures). A request
  replaces that environment with the snapshot's, so it does not reach `ctx.*`
  on a request path, but it is not allowlistable under R7. Removing it means
  the capture API takes the environment (or the request context) from its
  caller; see the message to the next agent.

#### Test fallout

About 690 test compile errors (darkmatter lib unit and `l1` tests, benches)
were fixed by five parallel subagents with disjoint file sets, following one
rule: a test that used the context-free API gets the context the old fallback
built (`FileResolutionContext::new(<document dir>)`: process home and
environment, no repository), or the explicit context it already set, or a
request anchored where its documents live. New helpers:
`compose::test_request_in(options, context)` (unit) and
`request_support::{context_at, cwd_context}` (`l1`). Tests whose premise was
an ambient fallback were rewritten to assert the context-based behavior, never
weakened:

- schemas `CwdGuard` tests (launch-fallback, file-property, `format.rs`
  resolution) now pass a context whose `cwd` is the directory they used to
  `chdir` into; the process directory is no longer changed.
- `context/options.rs` `ambient_context_upgrade_keeps_the_request_launch_anchor`
  became `request_context_upgrade_keeps_the_request_launch_anchor` (prepares
  a request instead of `chdir`).
- `conditions::tests::shortcut_env_lookup` puts its variable in the context's
  environment instead of the process's.
- Reference-graph tests anchor their request at the fixture directory: a
  graph's request context is authoritative, and a document outside the
  request's repository fails with `RepositoryRootNotContainingSource` (the
  Phase 2 behavior), where the deleted default built a per-document context.
- `ambient_ctx_capture.rs` uses `capture_for_dir(current_dir)` for the full
  capture the deleted `capture()` took.

**Structural validators, refined.** The first version judged every `file`
value by syntax and let `match()` admit everything. That broke
`contested_glob_decides_which_arm_coerces_siblings`: root-union coercion picks
an arm through `match()` judging the absolute path a caller value was
projected to. The rule is now: a value whose meaning needs a context (relative,
`&`, `^`, `@`, `~`, vault, `{{VAR}}`) is judged by syntax and admitted by
`match()`; an absolute path needs none, so it must exist and `match()` judges
its full path. Nothing reads the process's directory, home, or environment
(`format::context_free_path`).

### Wave 12: consumers

- **Claudine `Option` contexts: not done (blocked).** Claudine still fails to
  compile against the Phase 2 API (Phase 3 Track B). This phase adds to its
  list: `DarkmatterSchemas::new()`/`with_file_ref_fallback_dir` callers,
  `ResolutionContext::new(cwd)`, `CtxLookup`, `ComposeContext::capture()` in
  tests, `evaluate_condition_against`, `resolve_policy_paths`,
  `ComposeOptions::local_expression_resolution_context` (now only on
  `ComposeRequest`), and `ComposeRequest::context()` (renamed
  `resolution_context()`).
- **darkmatter-cli:** `md schema detect` takes the request (launch context);
  `md schema validate` builds its `DarkmatterSchemas` at the launch context
  and re-anchors per file, and reads `BASELINE_SCHEMA` from the snapshot
  instead of `std::env::var`; `md clean` passes the document's (or, for stdin,
  the launch) context to `CleanSchemaConfig::new`; `md graph` uses
  `FileTree::from_markdown(md, &request)`.
- **Phase 3 Track A residue fixed (7 CLI tests, 3 clippy errors that predated
  Phase 4).** Root cause of 6 tests: `MdRequest::document_context` kept the
  launch derivation whenever it validated, so a document launched from a
  directory in no repository never found its own repository (trigger schemas
  missed, link targets left absolute). It now keeps the launch derivation
  only when the launch context has a repository containing the document, or
  when the document's own build finds none; otherwise it builds at the
  document's directory and derives as a trusted external source (R9). The
  7th (`compose_state_set` numeric shorthand) asserted the removed
  "Failed to load" text; it now asserts R5's `failure: no-match` row. Clippy:
  `#[allow(clippy::too_many_arguments)]` on `run_get`/`run_hash` (repo
  convention) and `&Path` for `resolve_file_path`, `load_markdown_text`,
  `run_graph`.
- **Shell policy for stdin (`md compose -`).** Pre-flight used to drop the
  context, so a stdin document's policy files came from the process
  directory, while execution already used the context's home. With the
  context required, both took the library order (repository root, home,
  request directory), and on Windows the home is the real profile even in a
  fixture (`dirs::home_dir()` ignores `USERPROFILE`), which turned 11 CLI
  tests red on build-win-native. `md` now pins `policy_root` to the launch
  directory for stdin, as it already pinned it beside a file document, so
  pre-flight and execution name the same file and never the user's home.
  `compose_shell::test_compose_stdin_unapproved_command_fails_with_guidance`
  keeps expecting the launch directory; the comment explains why. Recorded in
  the `os` skill (Windows item 2).
- **dmls:** `overlay::schema::assemble` takes `&FileResolutionContext`; a
  document whose context failed gets no schema bundle (its `$schema` and
  trigger payloads cannot resolve without one), replacing Phase 4's
  "suppress file-value problems" workaround. Trigger scanning takes the
  document's context. Docs updated (`file-references.md`, `diagnostics.md`).
- **claudine-gen** (subagent): `load_validated_frontmatter(path, &context)`;
  a context per area built once through the new
  `inputs::area_resolution_context(area, &snapshot)` (the old
  `generator_schemas` body) and threaded into signals, steering catalog and
  check, and agent-errors check. `main` already holds the one snapshot.
- **messenger:** `Loader::new` uses `DarkmatterSchemas::new(context)`; two
  tests build their context with the builder.

### Requirement-to-test mapping

| Requirement | Test |
|---|---|
| A string document's eager `file` value resolves from the request directory; the miss names it (was process CWD, `format.rs` `resolved_from` + `base_dir_for`) | `l1 required_context::a_string_document_resolves_file_values_from_the_request_directory` |
| `env.*` in the condition API reads the context (was `std::env::var`) | `required_context::a_condition_reads_env_from_the_context_not_the_process` |
| Condition read-side functions resolve from the context directory | `required_context::a_condition_resolves_files_from_the_context_directory` |
| Detection infers `file` from the context directory (was `detect.rs` `base_dir_for`) | `required_context::detection_infers_file_values_from_the_context_directory` |
| Structural validators read no process state; absolute paths judged, `match()` on absolute paths | `required_context::structural_validators_resolve_only_context_free_file_values`, `compose::tests::schema::…::contested_glob_decides_which_arm_coerces_siblings` |
| Pathless source's shell policy paths use the request directory (was `store.rs` `current_dir`) | `required_context::a_pathless_source_keeps_shell_policy_in_the_request_directory`, CLI `compose_shell::test_compose_stdin_unapproved_command_fails_with_guidance` |
| `ComposeOptions::new` captures no process directory (`capture_minimal`) | `required_context::new_options_are_anchored_only_by_their_request` |
| `PortablePath` with context: a non-file source's links normalize against the request context | `required_context::a_string_documents_links_normalize_against_the_request_context` |
| Required context (AC2) | compile-time: no `Option<FileResolutionContext>` remains in darkmatter lib/cli, messenger, claudine-gen (Phase 6's guard locks it) |
| `md` documents launched outside any repository use their own repository | CLI `clean_schema::*` (4), `compose_transclusion::test_compose_link_{relative_same_repo,transcluded_child}` |

Each `required_context` test except the link-normalization one fails on the
pre-phase code (it read the process directory or environment); the
link-normalization test pins behavior that already held on the request path.
All are L1 (`darkmatter::l1`, declared in `tests/l1/main.rs`), no tier
markers. The Input Robustness Matrix does not apply (R15).

### Gates

- `just test` in `darkmatter/`: **8786 passed, 12 skipped** (lib, cli, dmls).
- `just test-l2` in `darkmatter/`: 3 + 18 + 69 passed.
- `just lint` in `darkmatter/`: passes.
- `messenger`: `cargo nextest run -p messenger -p messenger-cli` 692 passed,
  2 skipped; `just lint` passes.
- `claudine-gen`: 194 passed; `cargo clippy -p claudine-gen --all-targets -D
  warnings` clean. `just lint`/`just test` in `claudine/` cannot run: the
  `claudine` crate does not compile (Phase 3 Track B).
- Cross-OS (`scripts/cross-check.sh`, local tree):
  - Linux (`build-linux`), darkmatter: **7190 passed**, 67 skipped.
  - Windows (`build-win-native`), darkmatter: first run 7147 passed, 7
    failed. Fixed: `validate_fail_fast` and
    `resolve_file_reference_no_match_for_missing_absolute_path` used `/tmp`
    paths (not absolute on Windows); the four
    `lazy_roots::ambient_repository` tests (file unchanged this phase)
    canonicalized their fixture root with `canonicalize()`, whose verbatim
    `\\?\` spelling never matched the discovered root, now
    `canonicalize_simplified`. Rerun of those plus `required_context`: 17 of
    18 pass. **Remaining, pre-existing:**
    `compose::tests::schema::…::schema_number_increment_survives_quoted_persistence_round_trips`
    builds `dirname(spec) + '/review-2.md'` with `dirname` = `""`, i.e.
    `/review-2.md`, which is a foreign absolute path on Windows. Neither the
    test nor that code path changed this phase; it is left for Phase 8's
    Windows pass.
  - Windows, darkmatter-cli: first run 11 failed (stdin shell policy reached
    the real profile, above); after the `policy_root` fix **794 passed**.
- Final `just test` in `darkmatter/` after those fixes: **8786 passed**,
  12 skipped; `just lint` passes.

## Phase 6

Phase 6 started on top of the uncommitted Phase 5 work. **Claudine (lib and
cli) still does not compile** (20 errors; Phase 3 Track B was never done and
the spec's `human_review` question is unanswered), so its two guards cannot
run. Everything else in the phase is done: the shared engine, the darkmatter
(lib, cli, dmls), messenger (lib, cli), and claudine-gen guards, and every
`source-inputs` declaration.

### Wave 13: the shared engine

- `darkmatter/cli/tests/common/context_guard.rs` runs the three gates over a
  crate's `src/`. Construction matches `FileResolutionContext :: new` /
  `from_snapshot` (any spacing or path prefix) and any `<qualifier>::from_process`
  call. Optional context matches `Option<FileResolutionContext>` and
  `Option<&['a] [mut] FileResolutionContext>` with any path prefix, and the
  gate **rejects any allowlist entry** (AC 2's empty allowlist is enforced by
  the engine, not by convention). Ambient state matches
  `env::{current_dir, var, vars, var_os, vars_os, home_dir}` (including a
  `use std::env::{..}` group), `dirs::home_dir`, `home::home_dir`, a free
  `home_dir()` / `capture_env()` call (biscuit-file's), the context-free
  `FileReference` resolvers (`.resolve()`, `.resolve_from(..)`,
  `.resolve_target()`), and a `PortablePath::from_path|from_reference` whose
  expression has no `.with_ctx(..)`.
- Allowlist entries are `Allowance { gate, path, identifier, count, reason }`.
  An unlisted site, a moved count, an unused entry, a duplicate entry, and an
  entry with an empty reason or zero count all fail. A failure for a new site
  prints the `Allowance` literal to paste, so a reviewer sees exactly what is
  being admitted.
- **Scope rule moved, not copied.** The `#[cfg(test)]` blanking and test-module
  exclusion lived privately in `lib/tests/l1/semantic_results_never_persist.rs`.
  It moved into the shared `cli/tests/common/source_scan.rs`
  (`production_sources`, `ident_offsets`, `matching_brace`), which that guard
  now imports, so the two source-scan gates cannot drift apart. Its own
  self-test still covers the scope rule. `source_scan.rs` gained
  `#![allow(dead_code)]` because each including binary uses a subset.
- Self-tests: `darkmatter-cli::l1
  context_construction_guard::the_engine_rejects_planted_violations_and_stale_entries`
  plants every gate's violation (plus comment, string, method, definition,
  `#[cfg(test)]` item, and `#[cfg(test)] mod hidden;` file decoys) and every
  kind of bad entry, and asserts the exact problem list.

### Wave 14: the census and what it found

Each guard was first run with an empty allowlist to produce the census. The
plan said Reads that feed resolution or `ctx.*` are defects to fix, not to
allowlist; eight were found and fixed:

| Site | What it read | Fix |
|---|---|---|
| `compose/context/capture/mod.rs` | `std::env::vars()` seeded every ambient `ComposeContext` capture (`ctx.env`, `ctx.agent`, `ctx.model`) | the capture takes its environment from the caller: the public `capture_for_*` start empty (a request installs the snapshot's), `CtxLookup::new(dir, env)` takes the condition context's environment, and `AnchoredRefresh` keeps the request's |
| `compose/shell_expansion/executor.rs` `resolve_working_directory` | `std::env::current_dir()` as the last fallback for a pathless source | takes `&FileResolutionContext` and falls back to `context.cwd()`; `execute_command` (public) and the internal chain take it too |
| `compose/link_resolve.rs` `resolve_absolute` | `FileReference::resolve()` (ambient) for a source with no path | `resolve_in_context(request.resolution_context())` |
| `compose/schema_validation.rs` caller lazy URL | `FileReference::resolve_target()` builds an ambient context, so a URL's `{{VAR}}` came from the **process** environment | new additive biscuit-file `FileReference::resolve_target_in_context(&ctx)` |
| `compose/util.rs` `abbreviate_path` | `dirs::home_dir()` for `~/` in warning text | takes the request's `home_dir()` |
| `schemas/resolve.rs` `try_bare_name_in_roots` | `FileResolutionContext::from_snapshot` per schema root (a construction outside the builder, AC 3) | the bare name is spelled `./name` and resolved in `request_context.for_trusted_external_cwd(root)`; an explicit relative reference has one candidate, so ordering is unchanged |
| `messenger/cli` `research validate` | `std::env::current_dir()` for relative document arguments | uses the snapshot's request directory |
| `claudine-gen` `agent_errors_check` fixture check | `FileReference::resolve_from(dir)` (reads live home/env) | `FixtureBase` holds the area context derived onto the topic directory; `resolve_in_context` |

DMLS's eight `Option<…FileResolutionContext>` sites (Phase 5's message) became
`Result<_, ContextFailure>`: `DocumentResolution::context()`,
`DocumentContext::file_context()`, `DocumentContexts::context_for`,
`WorkspaceGraph::diagnose_unresolved`, and the resolver's `by_folder` cache.
`NoContexts` returns the new `ContextFailure::NotProvided`
(`ResolutionFailure::MissingContext`).

#### Classified allowlists

- **darkmatter (lib)**: the builder's one `from_snapshot`; `from_process`'s
  three reads; `current_env` (R7, live by design); the two handed-off reads
  (`file_links/discovery.rs`, `schemas/file_match.rs`, reason "handed off to
  2026-09-30-glob-reference"); and rendering, tuning, or tool-selection reads
  (`EDITOR`, `PREFER_ITALICS`, `DARKMATTER_REMOTE_CONCURRENCY`, `SHELL` for
  alias lookup, highlighting and code themes, image metadata policy,
  `DARKMATTER_SCHEMA_CACHE_SIZE`).
- **Two rendering-tier CWD reads are allowlisted, not fixed** (judgment call,
  recorded for review): `output/terminal.rs` `ImageRenderer::new(None)` (no
  production caller passes `None`) and `style/bespoke.rs`, where a
  `style.page.stylesheet` of a document with no path (stdin) reads relative to
  the launch directory. Neither is a `FileReference` nor `ctx.*`, and both run
  after composition; fixing bespoke means threading a base directory through
  `md`'s three render entry points.
- **darkmatter-cli**: `from_process` once in `main.rs`; `RUST_LOG`,
  `MD_DRY_RUN`, `DARKMATTER_NO_BASELINE_SCHEMA` (a switch, not a path),
  `HASH_PROPERTY` / `HASH_IGNORE_PROPERTIES`, `TERMINAL_IMAGES`.
- **dmls**: `from_process` once in `main.rs`; nothing else.
- **messenger**: `%APPDATA%` (Windows shortcut). **messenger-cli**:
  `from_process` once; `RUST_LOG` and a route secret's variable; its own
  `~/.messenger.json` and `~/.messenger/receipts`; notification helpers;
  `%APPDATA%`.
- **claudine-gen**: `from_process` once; `FORCE_COLOR` / `CLICOLOR_FORCE` /
  `NO_COLOR`.

#### Placement and CI

- darkmatter lib, darkmatter-cli, dmls, claudine-gen: `tests/l1/context_construction_guard.rs`,
  declared in each `tests/l1/main.rs` (`autotests = false`). messenger lib and
  cli have `autotests` on, so each guard is its own `tests/context_construction_guard.rs`
  binary. No tier marker; all L1.
- `source-inputs` names both shared files in darkmatter lib, dmls, messenger,
  messenger-cli (which gained a `[package.metadata.ci.tests]` table; absent
  keys keep their defaults), and claudine-gen. Each guard spells both files
  with `include_str!` inside its test function (S2).
  `python3 -m unittest test_affected_scope.RealWorkspaceTestInputTests` (from
  `scripts/ci/`) passes with every declaration.

#### Test fallout

- `context/capture/agent.rs` and `expression/ctx.rs` agent tests mutated the
  process `AGENT`/`MODEL`; they now pass the environment in (no
  `serial_test`, no `unsafe`).
- The unit and `l1` request helpers (`test_support::request`,
  `request_support::request_at`) take the snapshot environment from the
  options' own context, which used to be the process environment only because
  capture read it. Their code is unchanged and their docs now say the
  environment is empty unless the test put values there. A first attempt
  overlaid the process environment (to imitate `md`); it let this session's
  real `AGENT` into two `transclusion_tests` that need an empty environment,
  so it was reverted: the helpers stay hermetic.
- Tests whose premise was the leak were rewritten, never weakened:
  `frontmatter_interpolation::seed_state_tests::env_resolves` puts `HOME` in
  the context; `expression_regression::regression_ctx_agent_in_interpolation`
  and `regression_page_block_with_has_skill` put `AGENT` on the options'
  context (`options_with_agent`, Darkmatter-owned authority) instead of
  mutating the process environment, so both lost `serial_test` and `unsafe`.
- `executor.rs` tests pass a request context; `working_directory_resolution_priority`
  and the renamed `bare_filename_source_falls_through_to_the_request_directory`
  assert the request directory instead of "not empty".

### Departures

- **Behavior change: a standalone `ComposeContext::capture_for_*` has an empty
  `env`.** `ctx.env`, `ctx.agent`, and `ctx.model` on a request always came
  from the snapshot (Phase 5's `align_context_environment`); now nothing else
  can leak the process environment into them. `current.agent` (the `current`
  mirror) reads the request's environment instead of re-reading the process.
  `current_env.NAME` stays live (R7). Documented in
  `docs/topics/compose-requests.md`.
- **Public API**: `execute_command` and `resolve_working_directory` take a
  `&FileResolutionContext`; `CtxLookup::new` takes the environment. Claudine
  does not call any of them.
- **biscuit-file gained `FileReference::resolve_target_in_context`**
  (additive, like Phase 2's `resolution_failure()`); `resolve_target()` is
  unchanged.

### Windows residue from Phase 5, fixed

`messenger/lib/tests/research_corpus.rs` `schemas()` (Phase 5) built its
context from the compile-time `env!("CARGO_MANIFEST_DIR")` at `messenger/lib`.
In an archive run (the cross-check's native-Windows consumer, and CI's WSL2
leg) the extracted workspace has no `.git`, so the request directory became
the tree root and every fixture's `$schema: ../../…/messenger/docs/…` was
refused as an escape (10 `research_corpus` failures on `build-win-native`).
It now anchors at the run-time repository root (`messenger_dir().parent()`),
as the same file's `research_loader` already did. Recorded in the `os` skill.

### Requirement-to-test mapping

| Requirement | Test |
|---|---|
| AC 2: no `Option<[&]FileResolutionContext>`, empty allowlist | the optional-context gate in every `context_construction_guard` (darkmatter, darkmatter-cli, dmls, messenger, messenger-cli, claudine-gen); engine rejects an allowlist entry for it (self-test) |
| AC 3: one builder; `from_process` once per binary, zero in libraries | the construction gate in each guard (lib: only `build_resolution_context_with_catalog`'s `from_snapshot`; each binary: one `from_process` in `main.rs`) |
| AC 5: no ambient process state except allowlisted, reasoned reads | the ambient-state gate in each guard |
| Engine rejects each violation, ignores comments/strings/test code/methods/definitions, and rejects stale, moved, duplicate, and unreasoned entries | `darkmatter-cli::l1 context_construction_guard::the_engine_rejects_planted_violations_and_stale_entries` |
| Shared scope rule still holds after the move | `darkmatter::l1 semantic_results_never_persist::the_guard_catches_planted_violations_and_honors_the_scope_rule` |
| A pathless source's shell command runs in the request directory | `darkmatter::l1 required_context::a_string_documents_shell_command_runs_in_the_request_directory` (unix), unit `shell_expansion::executor::tests::working_directory_resolution_priority`, `bare_filename_source_falls_through_to_the_request_directory` |
| A string document's relative link resolves from the request directory; a miss stays authored | `required_context::a_string_documents_relative_link_resolves_from_the_request_directory` |
| A standalone capture reads no process environment; `ctx.agent` comes from the snapshot, `"unknown"` without one | `required_context::ctx_environment_comes_from_the_request_snapshot_only`, unit `capture::agent::tests::capture_runtime_context_includes_agent_group_from_the_supplied_environment`, `expression::ctx::tests::ctx_lookup_resolves_agent_and_model` |
| A caller's lazy remote `file` value fills `{{VAR}}` from the request environment | unit `schema_validation::tests::caller_remote_file_value_interpolates_from_the_request_environment` |
| `resolve_target_in_context` reads only the context (URL env, local dir, invalid context) | `biscuit-file::l1 resolution_context::resolve_target_in_context_reads_only_the_context` |
| Schema-root bare names still resolve nearest-first without a construction | existing schema-root tests (`test(/bare|schema_root|roots/)`, all pass) |
| messenger `research validate` takes relative documents from the launch directory; a document absent there fails | `messenger-cli::research_cli validate_takes_a_relative_document_from_the_launch_directory` |
| claudine-gen fixtures resolve through the area context | existing `agent_errors_check::review6_tests` (now through `FixtureBase`) |
| DMLS: a document with no supplied context resolves nothing and gets no link diagnostic | `graph::arena` test using `NoContexts` / `Err(&ContextFailure::NotProvided)` |

Each new behavior test was confirmed red on the pre-phase code (the four
`required_context`/`schema_validation` tests by temporarily restoring the old
read; the biscuit-file test asserts the ambient twin's different result in the
same body). The messenger CLI test pins behavior that already held (the
process directory and the snapshot's request directory are the same for a
binary). The Input Robustness Matrix does not apply (R15): no file-format
reader changed.

### Gates

- `just test` in `darkmatter/`: **8794 passed**, 12 skipped. `just lint`:
  passes (after `#[allow(clippy::duplicate_mod)]` on the engine's
  `source_scan` include: the lib and cli `l1` binaries also load it directly).
- `just test` in `messenger/`: **695 passed**, 2 skipped; `just lint` passes.
- `just test` in `biscuit-file/`: **1017 passed**; `just lint` passes.
- claudine-gen: `cargo nextest run -p claudine-gen` **195 passed**;
  `cargo clippy -p claudine-gen --all-targets -D warnings` clean. The
  `claudine/` area recipes cannot run (the `claudine` crate does not compile).
- `just check-tier-coverage darkmatter` / `messenger`: nothing stranded.
- `just ci-local --plan`: exit 0, no refused declaration;
  `python3 -m unittest test_affected_scope.RealWorkspaceTestInputTests`
  (from `scripts/ci/`): 6 passed with every new `source-inputs` entry.
- Windows (`just cross-check <pkg> --os windows`, local tree):
  darkmatter **7157 of 7158** (the one failure is the pre-existing
  `schema_number_increment_survives_quoted_persistence_round_trips`, recorded
  in Phase 5 and untouched here), darkmatter-cli 796, dmls 777,
  messenger-cli 126, claudine-gen 195, biscuit-file 958 all pass; messenger
  593 pass after the `research_corpus` fix above.
- Linux (`just cross-check <pkg> --os linux`): darkmatter **7195 passed**,
  messenger **598 passed**.

## Phase 7

Phase 7 started on the uncommitted Phase 5 and 6 work. **Claudine (lib, cli)
still does not compile** (22 errors, `cargo check -p claudine`), so the
claudine-cli runner was not written. Its two `EntryPoint` variants
(`ClaudineComposition`, `ClaudineCompletion`) and their rows are in the shared
tables, so the runner can be added later without editing them.

### Wave 15: the shared fixture and tables

`darkmatter/lib/tests/common/entry_point_parity/mod.rs` (`#![allow(dead_code)]`,
included by `#[path]` from each runner):

- **Fixture** (`ParityFixture::create(root)`): `repo/` is a hand-built git
  repository (no `git` needed) and a Cargo workspace whose member
  `area/pkg` makes `area` a package area. Depths are `area/pkg`,
  `area/pkg/docs`, and `area/pkg/docs/guide`, each with its own `sibling.md`
  and `beside.md`. `home/` (the fixture `HOME`) holds `magic-doc.md` and
  `notes/beside.md`, and `outside.md` sits at the root, above `HOME` and
  outside every repository. Every target's only content is
  `## TARGET <id>` (its `/`-spelled path below the root), so `::file`,
  `::code`, and `::toc-linking` output all name the file they resolved.
- **One reference per document.** Each Table 1 cell writes its own document,
  so a failing reference cannot mask another. A `SchemaFile` cell declares
  `target: file(eager)` and renders `stored-value={{ target }}`; the stored
  value is decoded back to a file (`~/` under `HOME`, absolute, or relative
  to the document's tree root).
- **`EntryPoint`** has 13 variants with exhaustive `owner()` and `rows()`
  (no `_` arm); `every_entry_point_has_a_row` asserts each has a row.
  `rows_for(owner)` feeds each runner, and `ParityReport` records every cell,
  then fails naming every mismatch and every owned variant that ran no cell.
- **Comparison**: files by `canonicalize_simplified` then `PathIdentity`,
  reported with `to_portable_string`; failures by `ResolutionFailure` only.

### Wave 16: runners

| Runner | Entry points | Cells |
|---|---|---|
| `darkmatter::l1 entry_point_parity::darkmatter_entry_points_agree_on_every_reference` | `compose_with`, `compose_preflight`, `DarkmatterSchemas::validate` + `normalize_frontmatter` | 194 |
| `darkmatter-cli::l1 entry_point_parity::md_entry_points_agree_on_every_reference` | `md compose <doc>`, `md schema validate <doc>`, `md compose <value>` from the repository root and the package | 154 |
| `dmls::l1 entry_point_parity::dmls_entry_points_agree_on_every_reference` | diagnostics, document links, link graph, definition, code actions (one LSP session) | 312 |
| claudine-cli | not written: Claudine does not compile | 0 |

The darkmatter runner builds `RequestSnapshot::new(repo).with_home(fixture
HOME)` and an empty environment. The `md` runner runs its spawns on eight
threads (about 3 s) and reads a failure only from the `failure: <name>` row.
The DMLS runner was written by a subagent; it opens, queries, and closes one
document at a time (opening all 130 first made each open republish every
document's diagnostics, 7.5 to 12.8 s).

### Wave 17: failing cells, fixed at the entry point

First runs: darkmatter 24 of 194 cells failed, `md` 11 of 154, DMLS 52 of 312.
Each failure was fixed in the entry point:

| Cells | Root cause | Fix |
|---|---|---|
| pipeline, pre-flight, schema validation: a document under `HOME` opened by absolute path from a request at the repository root (row (b)) gave `MissingContext` | the root source of `ComposeRequest::prepare` always used the ordinary `for_source` derivation, which is invalid outside the request's tree; only a transcluded child was admitted as trusted-external (`with_accepted_source_file_in`), and `md` worked around it in its own `document_context` | `source_derivation_for` (one rule) now also admits the root source, in `ComposeRequest::assemble` (`ComposeOptions::admit_source_in`) |
| pipeline and pre-flight schema `file` values in a child opened through `~`, and standalone `DarkmatterSchemas` on a document outside the request's tree: `MissingContext` | (1) the schema stage built `DarkmatterSchemas` over the request's context, not the source's derived one; (2) `resolve_ctx::document_file_context` reused a derived context only when its `cwd` was spelled identically, and a `~`-opened child keeps the `/var` spelling while its canonical path is `/private/var`, so it re-derived with `for_cwd`, which drops the `~` tree root and falls outside it | the schema stage uses `source_file_resolution_context()`; `document_file_context` recognizes the derived context by canonical directory, and a `cwd` outside the tree derives trusted-external when only that is valid (the rule compose applies to sources) |
| a tolerated failure (a nested child's `../../outside.md`, any failed `::toc-linking`) showed a notice with no class | the tolerated path turned the error into a message-only `ComposeWarning` | `ComposeWarning::resolution_failure` (new public field, `with_resolution_failure`), set from the new `MarkdownError::resolution_failure()`, which walks the cause chain (nested children, toc errors, schema `file` values) |
| `::toc-linking` tree escape: no class at all | the chain discarded each target's error (`Err(_) => continue`) and raised `FileNotFound`, so an escape read as a miss | `TocLinkingError::FileNotFound` became `Unresolved { path, line, failure }` carrying the first (authored) target's class, with `resolution_failure()`; its block prints `failure:` |
| `md`: schema `file` failure (compose block and `md schema validate`) and tolerated-failure warnings had no `failure:` row (R5) | the three renderers did not print the class | the compose schema block, `md schema validate`'s problem bullet, and `md compose`'s warning lines print `failure: <kebab-class>` |
| DMLS: every `::toc-linking` cell for document links and definition (52) | the provider handled links, definition, broken-path diagnostics, and hover only for `is_transclusion` kinds (`::file`/`::code`), although the graph records `::toc-linking <file>` | `directives::has_file_target(kind)` (transclusions plus `TocLinking`) at those four places; a broken toc target now gets `dm.transclusion.broken_path` ("broken `::toc-linking` target: …") |

The DMLS runner and its fix were written by a subagent and reviewed here.

### Departures and decisions

- **`@` target lives in the fixture `HOME`, not a configured magic root.**
  `md` has no way to configure an extra `@` root (only snapshot callers do),
  so a configured root could not give every entry point one answer. The
  `@` row exercises the chain's home tier, which every entry point receives
  through its snapshot. Configured snapshot roots stay covered by
  `repository_contexts::repository_sigils_resolve_in_every_feature` and
  `request_context_builder`.
- **Pre-flight rows cover `::file` and schema `file` values only.**
  Pre-flight resolves only targets that can hold shell commands
  (`collect.rs` skips `::code` and `::toc-linking` by design) and reports no
  value for a schema `file` (`Observed::Accepted`, which satisfies a file
  cell; its failures still compare by class).
- **Table 2's caller-supplied `../` is expected to resolve, not fail.** The
  spec (AC 10) says the tree-escape row yields `InvalidReference` at every
  entry point. `md compose ../outside.md` resolves: `resolve_file_path` opts
  in with `allow_external_relative()` (added in Phase 2, documented as "the
  caller's own path, not a document-authored one"), and the released `md`
  (built from `main`) resolves it too. Enforcing the boundary would break
  `md compose ../other/doc.md` from inside any repository. The table now
  expects the file for a caller-supplied value; document-authored `../`
  still expects `InvalidReference` everywhere. Raised for human review.
- **`md` on Windows skips the 38 fixture-`HOME` rows.** `RequestSnapshot::from_process()`
  reads the profile known folder, which ignores `USERPROFILE` (R2 keeps
  that; `os` skill, Windows trap 2), so a test cannot give an `md` child the
  fixture `HOME` there. The runner skips exactly the `@`, `~`, and
  through-`~` rows on Windows (count asserted) and decodes a stored `~/…`
  value against the real profile. The darkmatter and dmls runners cover
  those forms on Windows.
- **DMLS link graph reads the indexed workspace, not the disk.** The DMLS
  runner uses the fixture root as the workspace folder so `home/` and
  `outside.md` are indexed; with only `repo/` open, a Markdown link to an
  `@`, `~`, or outside target would most likely be reported broken. Not
  changed here; recorded for review.
- **Test placement.** The shared module is at the plan's
  `lib/tests/common/entry_point_parity/`; each runner includes it inside its
  own file (`#[path] mod matrix;`) and spells it with `include_str!` inside
  the test function. darkmatter-cli and dmls declare it in `source-inputs`.

## Phase 8

Phase 8 started on the committed Phase 7 tree (`875429e43`; 48 commits since
`e23d0c2b3`). **Claudine (lib, cli) still does not compile** (22 errors,
macOS and Windows alike), so every Claudine item in this phase is undone, and
the human-review item on the spec (Claudine's Phase 3 Track B) stays open.

### Wave 18: docs

Darkmatter (most library pages already used `ComposeRequest` and
`RequestSnapshot` from Phases 2 to 7; `topics/compose-requests.md` already
held the builder, snapshot, Mermaid flow, failure classes, `ContextBuildError`
table, and the live `current_env` rule):

- `docs/errors/file-reference-failures.md` (new): the `failure: <class>` row.
  It leads with what to do per class and shows real `md` output for each
  surface (argument, pre-composition transclusion check, compose error block,
  `md schema validate`, tolerated warning), all captured from the built `md`.
  Linked from `docs/errors/README.md`.
- `docs/topics/file-referencing.md`: was an unfinished stub ending in empty
  bullets. Now: the reference forms and where each starts, "one answer per
  request" with a Mermaid flow, the tree rule with an example, and the
  caller-supplied exception.
- `docs/topics/transclusion.md`, `docs/composition/index.md`,
  `docs/darkmatter-compose-pipeline.md`: a short request section each
  (snapshot → builder → `ComposeRequest`, Mermaid on the last two), linking
  to `compose-requests.md`.
- `docs/topics/darkmatter-expressions.md`: the `env.*` row says it reads the
  request snapshot's environment, the map `{{VAR}}` references read.
- `docs/topics/magic-paths.md` already said magic roots enter only through
  the snapshot; unchanged.

DMLS (written by a subagent, checked against `dmls/src/` and spot-read here):
`docs/topics/dmls.md` (was empty), `docs/lsp/architecture.md` (was a title;
now has "File resolution contexts" with the spec's flowchart adapted to the
code), `docs/lsp/features.md` (§1.4 file reference resolution, `::toc-linking`
parity, watched manifests and `.git/config`), `dmls/docs/diagnostics.md`
(`dm.context.build_failure` behavior; `dm.transclusion.broken_path` no longer
claims prologue/epilogue, which no code emits).

Where the code goes beyond the plan (docs follow the code; recorded for
review, none is a defect):

1. A document outside any repository gets one cached context per folder; on
   a discovery failure the key is the document's folder, so the message names
   that folder.
2. Invalidation is wider than "ancestor of the changed path": it also drops
   entries keyed inside the changed path's owning directory (for a path in
   `.git/`, the repository), and any watched document change inside a
   repository drops that repository's context.
3. **Rescan mode does not read `.git/config`**; it compares documents and
   manifests only. Without a file watcher, a repaired `.git/config` clears
   the failure only after a later change under that folder, a configuration
   change, or a restart. R11 named watching `**/.git/config`, which watched
   mode does. Documented as such.
4. Untitled buffers fail with `ContextFailure::UntitledWorkspace` (class
   `MissingContext`); `NotProvided` comes only from `NoContexts` graphs.
5. A directive target containing `{{ … }}` gets links and definition but never
   `dm.transclusion.broken_path`.
6. The context-failure diagnostic carries its own `data` payload.

Stale comments fixed (comment-only): `dmls/src/diagnostics/codes.rs`
(`TRANSCLUSION_BROKEN_PATH` no longer lists prologue/epilogue) and
`dmls/src/providers/definition.rs` (`document_links` targets come from the
context-resolved link graph, not "lexical path joins").

### Wave 18: skills

- darkmatter `SKILL.md`: **drift fixed.** "Composition authority" opened with
  "`ComposeOptions` is the request authority. It carries the captured
  resolution context", contradicting the code and the same section's later
  "`ComposeOptions` … holds no context". Now names `ComposeRequest` and links
  the three user docs. The `failure:` row sentence points at the new errors
  page.
- `library-surfaces.md`: same drift, same fix.
- `compose.md`: `prepare`'s `ContextBuildError` cases and the
  `resolution_failure()` rule.
- `dmls.md`: already current from Phase 4/7; unchanged.
- biscuit-file (R5 `FileReferenceError::resolution_failure()`) and sniff
  (R10 `PACKAGE_MANIFEST_FILE_NAMES`) skills already carried both from
  earlier phases; unchanged.
- **Not done: the `claudine` skill (`architecture.md`) and
  `claudine/docs/topics/{composition,system-prompt,completions/shell-completions}.md`.**
  They describe Claudine's own context capture, and that code has not
  changed, so they are still accurate. They change with the Claudine port.
- `os` skill `windows.md`: trap 15 (below).

### Wave 18: Windows evidence (`just cross-check <pkg> --os windows`, `build-win-native`)

| Package | Result |
|---|---|
| biscuit-file | pass |
| sniff | pass (2210 passed, 23 skipped) |
| darkmatter | **FAIL, 1 of 7160**, fixed, re-run below |
| darkmatter-cli | pass |
| dmls | pass |
| claudine-gen | pass |
| messenger | pass (593 passed) |
| messenger-cli | pass |
| claudine | FAIL: does not compile (unported, see above) |
| claudine-cli | FAIL: does not compile (depends on claudine) |

The darkmatter failure:
`markdown::compose::tests::schema::schema_validation_integration::schema_number_increment_survives_quoted_persistence_round_trips`
failed at iteration 2 with `FileReference { function: "decrement_file_index",
reference: "/review-2.md", source: ForeignAbsolutePath }`.

- **Cause.** The test runs only the interpolation stages, so the schema stage
  never makes `spec: file(required;eager)` absolute. `dirname("spec.md")` is
  `""`, and `dirname(spec) + '/review-' + …` builds `/review-2.md`, a
  filesystem-root path. Printed on macOS it is `/review-2.md` there too; the
  test passed on Unix only because its `ends_with("review-2.md")` accepted
  it. On Windows, biscuit-file rejects a `/`-rooted reference as
  `ForeignAbsolutePath`, which is correct. Whether the test also failed on
  `main` was not established (`main` parses through `FileReference::new` too,
  and its recent push runs are red for other reasons); the fixture was wrong
  on every OS either way.
- **Fix (test only).** The spec now lives in `specs/` (`spec: specs/spec.md`),
  so `dirname` is `specs`, and the assertions require exactly
  `specs/review-N.md` and `specs/review-{N-1}.md` instead of a suffix.
- **Finding, not fixed (pre-existing, every OS):** `dirname` of a bare file
  name returns `""`, so the shipped `dirname(spec) + '/…'` pattern builds a
  root path whenever `spec` is a bare name that no schema stage normalized.
  Shipped prompts declare `spec` as an eager `file`, so the full pipeline
  never meets it. Returning `.` would remove the trap; that is a behavior
  change to an expression function and is left to review.

Re-run after the fix: darkmatter on Windows **pass, 7160 passed, 67
skipped**. The other packages' Windows runs covered only comment and doc
changes since, so they were not re-run. `os` skill `windows.md` gained trap
15 for this.

### Wave 19: status and final checkpoint

Every changed output and departure across the fix, for the reviewer:

- **Changed outputs.** `md`: a `failure: <kebab-class>` row on every failed
  file-reference block, `md schema validate` problem, and tolerated-failure
  warning (R5; documented in `docs/errors/file-reference-failures.md`).
  DMLS: file-reference diagnostics carry `data: {"resolution_failure":
  "<Class>"}`, a new `dm.context.build_failure` diagnostic (source
  `darkmatter.context`), `::toc-linking <file>` targets get links,
  definition, and `dm.transclusion.broken_path`, and the watcher registers
  sniff's package manifests and `.git/config`.
- **Deleted APIs.** `Markdown::compose()`/`compose_mut()`, the `Option`
  context on `ComposeOptions` and its setter, `ComposeOptions::magic_paths`/
  `with_magic_path`, `TransclusionOptions::magic_paths`,
  `establish_repository_observation`/`ensure_file_resolution_context` as
  standalone calls (Phases 2 and 5 logs list each).
- **Departures.** R4: `prepare(options, &snapshot)` instead of the spec's
  `prepare(options)`. R5: the `md` `failure` row (author may overturn). R7:
  `current_env` stays live, allowlisted. R12: spec drift logged, not edited.
  S1: builder-failure inputs settled as R11. Phase 7: caller-supplied `../`
  resolves (raised for review); `@` matrix target lives in the fixture
  `HOME`. Phase 8: DMLS rescan mode does not read `.git/config` (above).
- **Spec status.** Set to `human-in-the-loop`, not `implemented`: the
  status enum has no "implementation complete, ready for review" value, and
  the branch cannot be reviewed as complete while Claudine does not compile.
  `implemented: true` is set as the phase protocol requires. The spec was not
  moved and `just complete` was not run.

Final checkpoint (macOS, final tree):

| Area | `just test` | `just lint` |
|---|---|---|
| darkmatter (lib, cli, dmls, zed-dmls) | 8798 passed, 12 skipped | pass |
| biscuit-file | 1017 passed | pass |
| sniff | 3121 passed, 32 skipped | pass |
| messenger | 695 passed, 2 skipped | pass |
| claudine-gen (`cargo nextest run -p claudine-gen`, clippy `-D warnings`) | 195 passed | pass |
| claudine area | not runnable: `claudine` does not compile | not runnable |

`git verify-commit` on all 48 commits `e23d0c2b3..HEAD`: 48 verified, 0
unverified; author Ken Snyder, no attribution trailers. Phase 8's own changes
are uncommitted, per instructions.

`just ci-local --plan` was not run in this phase (Phase 7's open checkpoint
item); the plan stays red on Claudine until it compiles.

### Hand-off

- **Next on `fix/magic-globs`: Claudine's port (Phase 3 Track B, plus Phase 5's
  Claudine task, Phase 6's two Claudine guards, Phase 7's claudine-cli
  runner, and the Claudine docs and skill named in Wave 18).** The spec's
  human-review item still asks for that decision.
- **Then the feature `2026-09-30-glob-reference`.** It must delete the two
  handed-off allowlist entries (`file_links/discovery.rs`,
  `schemas/file_match.rs`, reason "handed off to 2026-09-30-glob-reference")
  and add its rows and `EntryPoint` variants. The branch is not merged until
  both are done (Decision 16). Watching the post-merge push to `main` is the
  merger's step.
