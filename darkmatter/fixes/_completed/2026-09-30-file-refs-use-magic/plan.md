---
area: darkmatter
fix: 2026-09-30-file-refs-use-magic
total_phases: 8
created: 2026-10-01
phase: 2
agent: "claude/opus"
yolo: true
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

# Plan: File References Resolve From One Prepared Context

## Summary of the Work

The fix closes Incident 1 (`&` and `^` fail in `md compose` because
preflight never prepares a repository-aware context) and then removes the
design gap behind it, in the order Decision 2 fixes:

1. **Preflight fix** (commit 1). Preflight prepares its options the way the
   pipeline does, with a CLI regression test for `&` and `^` from a nested
   directory.
2. **Change 1, a prepared request is a type.** Darkmatter publishes one
   request snapshot type, one context builder that validates and returns a
   `Result`, and `ComposeRequest`, which holds a **required**
   `FileResolutionContext`. Every entry point moves onto it: preflight, the
   pipeline, schema validation, every `md` route, Claudine's composition and
   completion, claudine-gen, the messenger research loader, and DMLS. DMLS
   gets one lazily built context per repository, cache invalidation, a
   context-failure diagnostic, and untitled-buffer handling.
3. **Change 2, no silent fallback.** No `Option` context remains on a
   request path, `document_resolution_context` loses its
   `FileResolutionContext::new(cwd)` fallback, every listed current-directory
   read goes (except the two handed off to `2026-09-30-glob-reference`), and
   every `PortablePath` receives `with_ctx`.
4. **Enforcement.** Source-scan guard tests (construction, optional context,
   ambient process state) and the entry-point parity matrix (two tables, an
   exhaustive `EntryPoint` enum, errors compared by `ResolutionFailure`).

What the code survey (2026-10-01) found, which shapes the plan:

- **Magic paths are dropped today whenever a context exists.**
  `ComposeOptions::magic_paths` is applied only on
  `document_resolution_context`'s no-context branch
  (`compose/util.rs:83`). `ensure_file_resolution_context` never adds them,
  and the resolver's context branch (`transclusion/resolver.rs:167-178`)
  ignores them. No production code calls `with_magic_path`. This answers the
  spec's "confirm magic paths are not dropped": they are dropped for every
  source that has a context. The builder fixes that (Ruling R3).
- **`Option` contexts number about 71, not 52.** Darkmatter has 55: 17 in
  `compose/` and 38 in `schemas/`, the latter mostly `resolve.rs` (16),
  `rewrite.rs` (8), and `validate.rs` (6). Claudine has 16.
- **Construction sites.** Darkmatter has about 30 production
  `FileResolutionContext::new` / `from_snapshot` sites. Claudine has 7:
  `invocation_context.rs:2130`, `composition/resolve.rs:74,147`,
  `composition/sequence/source.rs:95`, `harness/resolve.rs:187`,
  `system_prompt/resolve.rs:289`, and `cli/src/completion/scopes.rs:240`.
  claudine-gen (`gen/src/inputs.rs:255`) and messenger
  (`research/load.rs:122`) have one each. The spec lists
  `cli/src/commands/sequence.rs`, `schema_interactive/supplied.rs`, and
  `composition/sequence/expr.rs` as production sites, but each is inside
  `#[cfg(test)]`.
- **`md` routes are mostly context-free.** `md compose` attaches its
  hand-built context only when `--set` overrides are present
  (`commands/compose.rs:360-364`); otherwise validation, preflight, and the
  pipeline each capture their own. `validate`, `graph`, `schema validate`,
  `schema triggers`, `schema detect`, and `clean` capture ad hoc.
  `hash`, `frontmatter get|set|rm`, `toc`, `delta`, and `code-block` resolve
  their document argument through the ambient `FileReference::resolve()`
  (`cli/src/io/mod.rs:66`).
- **DMLS has no repository discovery and no context.** Every lexical join
  (`graph/arena.rs:938` `normalize_join`, `providers/dsl.rs:1080`
  `resolve_local_path`, frontmatter `nav_targets`, the code action, and anchor
  completion) must move. Its save-triggered rescan ignores its own "changed"
  result (`router.rs:1017`), and it has no `didChangeWorkspaceFolders`
  handler: roots are fixed at initialize.
- **Darkmatter errors do not carry `ResolutionFailure`.** biscuit-file's
  classifier `classify_error` is private (`resolve.rs:312`), and `md` has no
  machine-readable compose error output. The parity matrix cannot compare by
  failure class until each entry point exposes it (Ruling R5).
- **Sniff's package manifest names are private.** Packages are detected from
  exactly `Cargo.toml`, `package.json`, `pyproject.toml`, and `go.mod`
  (`sniff/lib/src/filesystem/repo/glob.rs:32`, `const MANIFEST_FILES`).
- **The ambient-read census is large.** About 286 unfiltered
  `std::env::{current_dir,var,vars,var_os,home_dir}` / `dirs::home_dir`
  matches across the eight packages, most in Claudine. Many sit in
  `#[cfg(test)]` code the sanitizer blanks; the rest are classified one by
  one (Ruling R7).

### Definition of Success

- Acceptance Criteria 1–14 of the spec hold, each with the test named in
  this plan. Two exceptions are tracked: Acceptance Criterion 5's
  `resolve_boundary` and `admits` remain as allowlist entries marked
  **handed off** until `2026-09-30-glob-reference` deletes them on this
  branch.
- `md compose docs/use-claudine/SKILL.md` from `claudine/` succeeds (manual
  check recorded in the implementation log).
- `just test` and `just lint` pass in `darkmatter/`, `claudine/`, and
  `messenger/`, plus `biscuit-file/` and `sniff/` for their additive changes
  (R5, R10). `just test-l2` passes where DMLS or the CLI L2 suites are
  touched.
- `just cross-check <pkg> --os windows` passes for every package in the
  spec's `packages` list plus `biscuit-file` and `sniff`, recorded in the
  implementation log.
- `implementation-log.md` lists every changed output and every departure
  from the spec. The spec's `status` is set to "implementation complete,
  ready for review". The spec is not moved, and `just complete` is not run.
- Every commit is signed and passes `git verify-commit`. Commits follow
  Decision 2's order (Ruling R16).
- `docs/` topic pages and the `darkmatter`, `claudine`, `biscuit-file`, and
  `sniff` skills describe the new behavior. No `docs/` page names this fix.

### Conventions for Every Task

- The whole session tree is **non-interactive**. Every subagent brief
  repeats this, never runs prompting commands, and never signs, pushes, or
  changes credentials.
- Use `nextest` through `just test` / `just test-l2` in the package area.
  Never run `cargo fmt`. Never run `just lint` concurrently with `just test`.
- Load the `darkmatter` skill for every task. Also load `biscuit-file` for
  context or derivation work, `sniff` for discovery, `claudine` for Claudine
  tasks, `rust-testing` for tests, `rust-devops` for `source-inputs`, and
  `os` before touching path comparison or `#[cfg(windows)]` code.
- Tests never mutate the test process's environment or current directory.
  A fixture `HOME` and environment reach code only through a request
  snapshot (or `CliProcessFixture` for `md`). L2 tests never take focus.
- A behavior change updates its `///` / `//!` docs in the same edit. If a
  comment disagrees with the code, assume the code is right, fix the
  comment, and record the drift in the implementation log.
- A departure from the spec goes into `implementation-log.md` and the
  `docs/` tree, never into `spec.md`.

## Phase 1: Rulings, Spikes, and the Preflight Fix

Goal: settle every planning-owned decision, retire the two real unknowns,
and land commit 1 (Incident 1 fixed, with its regression test) before any
signature changes.

### Necessary Rules

These are rulings made in planning. **R5's `md` output row and R7's
`current_env` row change user-visible behavior; the author may overturn
them before Phase 2 starts.** The rest settle what the spec delegated to
planning.

- **R1. Names.**
  - Prepared request: `ComposeRequest`.
  - Request snapshot: `RequestSnapshot`.
  - Process constructor: `RequestSnapshot::from_process()`.
  - Builder: `build_resolution_context(&RequestSnapshot) -> Result<FileResolutionContext, ContextBuildError>`,
    defined in `darkmatter/lib/src/markdown/compose/context/` and
    re-exported from `darkmatter::markdown::compose`.
  - DMLS diagnostic: code `dm.context.build_failure`, source
    `darkmatter.context`, both in `dmls/src/diagnostics/codes.rs`.
  - Guard tests: `context_construction_guard.rs`, one per package (R6).
  - Parity matrix: shared module `entry_point_parity`, with per-package
    runners named `entry_point_parity.rs` (R6).
- **R2. The snapshot's shape.**
  - `RequestSnapshot::new(request_dir)` names the request directory and
    starts with no `HOME` and an empty environment.
  - Builders add the rest: `with_home(Option<PathBuf>)`,
    `with_env(HashMap<String, String>)`,
    `with_magic_root(path, PathPosition)` for the caller's extra `@` roots,
    and `with_opening_reference(FileReference, PathBuf)`.
  - `at_request_dir(dir)` rebases a snapshot while keeping its home,
    environment, and extra roots. It is used for a source in another
    repository, a DMLS repository root, Claudine's `capture_at`, and
    messenger's workspace root.
  - `from_process()` reads the current directory plus home and environment
    through the same biscuit-file helpers `FileResolutionContext::new` uses
    today, so Windows `USERPROFILE` handling and non-UTF-8 behavior do not
    change. It is the only reader of process state.
- **R3. What the builder does, in order.**
  1. `from_snapshot(request_dir, home, env)`.
  2. Sniff repository discovery from the request directory. "No
     repository" is `Ok` with no repository; a discovery **error** is
     `Err`.
  3. `repository_scope_catalog`.
  4. The launch `@` scope.
  5. The snapshot's extra `@` roots.
  6. `for_source_reference` when an opening reference is present.
  7. `validate()`.
  8. One `tracing::debug!` naming the request directory and the `base_dir`
     origin.

  Magic paths enter **only** through the snapshot.
  `ComposeOptions::magic_paths`, `with_magic_path`, and
  `TransclusionOptions::magic_paths` are deleted; no production code sets
  them. `ContextBuildError` names the failing directory and exposes
  `resolution_failure() -> ResolutionFailure` (see R5).
- **R4. The `ComposeRequest` API.**
  - `ComposeRequest::prepare(options, &RequestSnapshot)` builds through the
    builder.
  - `ComposeRequest::with_context(options, FileResolutionContext)` is for
    callers that already hold a built or derived context: the `md` source
    derivation, Claudine, and DMLS.
  - Both perform today's repository observation, return `Result`, and hold
    the context by value. `ComposeOptions` loses its `Option` context field.
  - `Markdown::compose()` and `compose_mut()` (no request) are deleted;
    `compose_with(&ComposeRequest)` is the only form, so a library call
    names its request directory (Decision 4).
  - `ctx.*` process values (env, home, request directory) read from the
    request's `FileResolutionContext` (`env()`, `home_dir()`,
    `request_cwd()`), which the builder filled from the snapshot. That is
    "the same snapshot" without storing a second copy. `env.*` in
    `compose/conditions.rs:379` reads the same map.
  - The spec's `ComposeRequest::prepare(options)?` example becomes
    `prepare(options, &snapshot)?`. This is recorded as a naming-level
    departure.
- **R5. How each entry point exposes a `ResolutionFailure`.** The matrix
  cannot compare by class otherwise.

  | Entry point | Exposure |
  |---|---|
  | biscuit-file | Gains `FileReferenceError::resolution_failure()`, a public delegate to the private `classify_error`. This is additive, so the classifier is never re-derived. |
  | darkmatter | Every error raised from a file reference (the transclusion file-reference error, the schema file-value problem, and `ContextBuildError`) keeps the class and exposes `resolution_failure()`. |
  | `md` | A failed file reference's rendered error block gains a stable `failure` detail row in kebab case (for example `failure: invalid-reference`), documented in `darkmatter/docs/errors/`. The CLI runner reads only that row, never message text. **Author may overturn.** |
  | DMLS | File-reference diagnostics carry `data: {"resolution_failure": "<Class>"}`. A navigation feature's failure cell asserts no target **and** the class on that reference's diagnostic. |
  | Claudine | Composition errors already carry the class (`composition/error/mod.rs:3060`). Completion of a supplied value exposes it through the same accessor. |
- **R6. Where the tests live.**
  - **Guards.** One `context_construction_guard.rs` per package, scanning
    that package's own `src/`. All share one engine,
    `darkmatter/cli/tests/common/context_guard.rs`, included by `#[path]`
    beside `source_scan.rs`. Every package other than darkmatter-cli
    declares both shared files in `[package.metadata.ci.tests] source-inputs`.
  - **Matrix.** The fixture builder, both tables, the `EntryPoint` enum, the
    exhaustive `owner()`/`rows()` matches, and the comparison helpers live
    in `darkmatter/lib/tests/common/entry_point_parity/`. Runners are in
    darkmatter (pipeline, preflight, schema validation), darkmatter-cli (`md`
    routes and arguments, which need `CARGO_BIN_EXE_md`), dmls (five
    features), and claudine-cli (composition and completion). Each
    non-owning package declares the shared files in `source-inputs`.
  - Each runner `match`es exhaustively over `EntryPoint`, so a new variant
    fails to compile in every runner. Each runner asserts it executes every
    variant whose `owner()` is its package. A shared assertion checks every
    variant has at least one row.
  - The expected-result type is an enum (`File`, `Failure`) that the glob
    feature extends with an ordered `Files` variant.
  - **Spelling the coupling (Spike S2, 2026-10-01).** A `#[path]` include
    alone does **not** satisfy a `source-inputs` declaration:
    `test_every_declared_source_input_is_read_by_its_declarer` fails with
    "no L1 test of it names the path", because the test-input index walks a
    `#[path]` module but does not count it as a reference. Every declaring
    runner and guard therefore also spells each shared file with
    `include_str!("<same relative path>")` **inside the test function** that
    depends on it. Inside a test function the planner narrows the cell to that
    test (`binary_id(claudine-cli::l1) & test(=<module>::<test>)`); at module
    level it widens to the whole binary. A `#[path]` include and its
    `include_str!` compiled and ran in both the `dmls::l1` and
    `claudine-cli::l1` binaries; darkmatter's `l1` already includes
    `cli/tests/common/source_scan.rs` this way. `claudine-cli::l1` holds both
    completion (`completion_*`) and composition (`compose_*`) tests, but
    several `compose_*` modules are `#[cfg(unix)]`, so a matrix runner there
    must not inherit that gate.
- **R7. Ambient-state gate breadth.**
  - The gate rejects `std::env::current_dir`, `var`, `vars`, `var_os`,
    `vars_os`, and `home_dir`, plus `dirs::home_dir` and `home::home_dir`.
    `var_os` and `vars_os` are added because they read the same state.
  - It also rejects biscuit-file's ambient entry points:
    `biscuit_file::home_dir`, its environment-capture helper,
    `FileReference::resolve()` without a context, and any `PortablePath`
    evaluation not preceded by `with_ctx`. The exact identifiers are fixed
    by the census.
  - Allowlist entries are `(file, identifier, count, one-line reason)`. A
    read may be allowlisted only if it feeds neither file resolution,
    `ctx.*`, nor `env.*`. Examples: rendering and theme flags, terminal
    detection, telemetry, `RUST_LOG`, and test-only hooks.
  - **`current_env` stays live** (`compose/context/current.rs:513`): it is
    not `ctx.*` and is documented as "the live process environment at
    reference time". It is allowlisted with that reason. **Author may
    overturn** by moving it onto the snapshot instead.
- **R8. `from_process()` call sites.** It is called exactly once per binary
    (`md`, `claudine`, `dmls`, `claudine-gen`, `messenger`), in `main` or the
    top-level dispatch, and the snapshot is threaded down. The construction
    gate counts `from_process`: one per binary crate, zero in library crates.
    Claudine's `InvocationContext::capture`, `capture_at`, and
    `capture_for_wrapper`, and completion's `ScopeContext::discover`, take
    the snapshot instead of reading the process.
- **R9. The Claudine and `md` split for sources in another repository.**
  - Both `md` (`compose.rs:281`) and Claudine
    (`derive_request_context_for_source`, `resolve.rs:124`) call the builder
    with `snapshot.at_request_dir(source_dir)`, then derive with
    `for_trusted_external_source_reference` (or `_source`).
  - A same-repository source keeps `for_source_reference`
    (`derive_composition_source`).
  - Claudine keeps:
    - the decision that a source is foreign;
    - its `@` prompt roots (`with_prompt_magic_roots`), passed as snapshot
      roots;
    - its error policy.
  - Claudine's own `capture_file_resolution_context`,
    `build_sequence_resolution_context`, harness `build_resolution_context`,
    and system-prompt `resolve_file_ref` become derivations (`for_cwd` /
    `for_source`) of the invocation's built context. They are not new
    builds unless the directory lies in another repository.
- **R10. DMLS details.**
  - **Watched manifests.** sniff exposes its package-manifest list as
    `pub const PACKAGE_MANIFEST_FILE_NAMES` (`Cargo.toml`, `package.json`,
    `pyproject.toml`, `go.mod`), replacing the private `MANIFEST_FILES`.
    DMLS registers `**/<name>` for each, so the list has one source.
  - **Cache keys.** The cache is keyed by repository root, or by the
    document's folder for a document in no repository. A watched or
    rescan-detected change drops every cached entry whose key is an
    ancestor of the changed path. A configuration change drops all entries.
  - **Untitled buffers.** Each workspace folder counts the repository that
    contains it, found by discovering upward from the folder. Repositories
    merely nested below a folder are not counted. If the distinct count is
    exactly 1, the buffer uses that context with `cwd` = repository root;
    otherwise it gets the context-failure diagnostic.
  - **Hover and anchor completion** also move onto the context, since they
    share `resolve_local_path` / `normalize_join`. They get no matrix rows.
  - **Rescan** reports the changed paths it found and includes manifest
    names in its scan.
- **R11. Builder-failure inputs** for Acceptance Criteria 6 and 13, settled
  by Spike S1 (2026-10-01, macOS, biscuit-file and sniff called directly):
  - **(a) Relative request directory.** `RequestSnapshot::new("relative/dir")`
    fails `validate()` with `FileReferenceError::RelativeContextDirectory
    { anchor: RequestDirectory, .. }`.
  - **(b) Opening reference outside the request tree.** A snapshot whose
    request directory is inside a repository, with an opening reference
    `~/notes/doc.md` (home captured in the snapshot) or `{{NOTES}}/doc.md`
    (`NOTES` in the snapshot environment) resolving outside that repository,
    fails `validate()` after `for_source_reference` with
    `RepositoryRootNotContainingSource`. The same holds for a request
    directory inside `VAULT` when the repository root is elsewhere. With no
    repository (fallback tree) the `~` anchor becomes the tree and validation
    passes, so the failing input must have a repository.
  - **(c) Discovery error DMLS can create and repair.** A syntactically
    corrupt `.git/config` (for example the two lines `[core` and
    `this is not = = valid`) makes both `sniff::filesystem::git::GitRepo::discover`
    and `biscuit_file::find_git_root` return `Err` (gix config parse error).
    Rewriting the file as a valid config (`[core]`,
    `repositoryformatversion = 0`, `bare = false`) makes discovery succeed
    again. These states are **not** errors; sniff reports "no repository":
    a `.git` file naming a missing or empty `gitdir`, a `.git` file with
    garbage content, an empty `.git` directory, a `.git` directory without
    `HEAD`, and an unreadable `.git` directory (Unix `chmod 000`). A malformed
    `HEAD`, `repositoryformatversion = 99`, and an unknown extension all
    discover successfully. The parser is pure gix, so the input is
    platform-neutral. **Consequence for Phase 4:** the repair edit is to
    `.git/config`, so DMLS's cache invalidation must see that path change
    (watch `**/.git/config`, or have the test repair through a watched path
    as well).
- **R12. Spec drift is logged, not edited.** This covers the test-only
  Claudine sites, the `Option` count, the magic-path finding, and the
  `trigger_boundary` doc comment (`dmls/src/overlay/schema.rs:982`), which
  claims the git root narrows the boundary when the code uses the workspace
  folder. The comment is fixed in Phase 4 and logged.
- **R13. messenger research roots.** `Loader::new` forces
  `with_repository_root(workspace.repo_root())` today. If a research
  workspace is not a git repository, the builder will not find a
  repository there. Phase 3 checks this; if research workspaces can be
  non-repositories, stop and ask the author rather than re-adding an
  explicit root.
- **R14. No performance spike.** The spec names no cost. DMLS rebuilding a
  context on any watched event in a repository costs one sniff discovery,
  run lazily. If the author wants a measurement, it is a new ruling, not a
  spike.
- **R15. Input Robustness Matrix: not applicable.** This fix adds no
  file-format reader. DMLS watches manifests but does not parse them, and
  the snapshot is in memory.
- **R16. Commits.** Commit 1 is Phase 1. Change 1 is Phases 2–4, and
  change 2 is Phase 5. Guards (Phase 6), matrix (Phase 7), and docs
  (Phase 8) follow. Every commit builds and tests all in-scope packages; if
  Phase 2 cannot stand alone, Phases 2 and 3 land as one commit. Each
  commit is signed and passes `git verify-commit HEAD`, with no agent
  attribution trailers.

### Spikes

- **S1. Builder-failure inputs** (one host, quick). Using biscuit-file and
  sniff directly, find:
  - (a) that a relative request directory fails `validate()`;
  - (b) a request directory outside a tree root named by the snapshot (an
    opening reference through `~` or `{{VAR}}`, or `VAULT`) that fails;
  - (c) a **filesystem** state that DMLS can create and then repair at
    runtime, which sniff discovery reports as an error rather than "no
    repository" (for example a `.git` file naming a missing `gitdir`).

  Record the inputs as R11. If (c) has no answer, stop and ask the author
  to choose between a DMLS configuration input and test-only fault
  injection.
- **S2. Matrix and guard topology** (one host, quick). Prove that:
  - a `#[path]`-included shared module compiles in the darkmatter, dmls, and
    claudine-cli L1 binaries;
  - a `source-inputs` entry naming it is accepted by
    `scripts/ci/test_affected_scope.py` and narrows as `docs/cicd/test-inputs.md`
    describes;
  - claudine-cli can reach both Claudine composition and completion from
    one test binary.

  If a declaration is refused, record the alternative in R6 before Phase 6.

### Tasks

#### Wave 1 (parallel)

- [x] **Reproduce Incident 1**
  - From `claudine/`, run `md compose docs/use-claudine/SKILL.md` with the
    installed or worktree `md` and record the failure text in
    `implementation-log.md`.
- [x] **Spike S1** (above)
- [x] **Spike S2** (above)

#### Wave 2 (sequential; commit 1)

- [x] **CLI regression test**
  - In `darkmatter/cli/tests/l1/`, using `CliProcessFixture` and
    `initialize_repository`, build a temporary git repository with
    `target.md` at its root and a document two directories down containing
    `::file &target.md` and `::file ^target.md`.
  - Run `md compose` from the nested directory and assert both bodies
    appear. Confirm the test fails before the fix.
- [x] **Library regression test**
  - A darkmatter L1 test calls `compose_preflight` with options that carry
    no context and asserts `&` and `^` transclusions are collected without
    a file-reference failure.
- [x] **Preflight fix**
  - Extract the pipeline's preparation (`run_compose_pipeline`,
    `pipeline/mod.rs:36-42`: `extend_context_for`,
    `establish_repository_observation`, `ensure_file_resolution_context`)
    into one `pub(crate)` function.
  - Call it from the pipeline and at the top of `compose_preflight`,
    `compose_preflight_approvals`, and the public `collect_*` entry points
    (`preflight/collect.rs:137,152,231`). This is deliberately minimal;
    Phase 2 replaces it.
- [x] **Checkpoint** (test, lint, and the incident check done; the commit is left to the separate commit process, per the phase prompt)
  - `just test` and `just lint` in `darkmatter/`.
  - `md compose docs/use-claudine/SKILL.md` from `claudine/` succeeds;
    record it.
  - Commit 1 (`fix(darkmatter): prepare repository context in compose preflight`),
    signed and verified.

## Phase 2: Request Snapshot, Context Builder, and `ComposeRequest`

Goal: change 1 inside darkmatter's library. The builder exists and
validates, `ComposeRequest` holds a required context, and the three
standalone preparation functions are gone (Acceptance Criteria 4 and 6).

### Wave 3 (parallel; disjoint crates)

- [x] **biscuit-file accessor**
  - Add `FileReferenceError::resolution_failure()`, delegating to
    `classify_error`, with a unit test per `ResolutionFailure` variant.
  - Update the `biscuit-file` skill. Run `just test` and `just lint` in
    `biscuit-file/`.
- [x] **sniff manifest list**
  - Make `PACKAGE_MANIFEST_FILE_NAMES` public (R10) and use it in
    `glob.rs` and the hard-coded copies in `manifest_index.rs`
    (`:459-462`, `:501-504`, `:691`).
  - Update the `sniff` skill. Run `just test` and `just lint` in `sniff/`.
- [x] **`RequestSnapshot` and `ContextBuildError`**
  - Add the snapshot type in darkmatter with R2's constructors, and the
    error type with directory and `resolution_failure()`.
  - Unit tests: `new()` has no home and an empty environment;
    `at_request_dir` keeps home, environment, and roots.

### Wave 4 (sequential; depends on Wave 3)

- [x] **Builder**
  - Implement `build_resolution_context` per R3, replacing
    `capture_file_resolution_context` (`context/capture/mod.rs:74`, deleted
    with its re-export at `compose/mod.rs:162`) and the body of
    `ensure_file_resolution_context`.
  - L1 tests (Acceptance Criterion 6):
    - a relative request directory is rejected;
    - a request directory outside a snapshot-named tree root (S1 input) is
      rejected;
    - a successful build emits exactly one `debug` event naming the request
      directory and `base_dir` origin, captured with a scoped subscriber;
    - a snapshot magic root resolves an `@` reference (the magic-path
      finding).
- [x] **`ComposeRequest`**
  - Add `prepare` and `with_context` (R4). Delete
    `establish_repository_observation` and `ensure_file_resolution_context`
    as callable items; their logic lives in `prepare` and the builder.
  - Move `run_compose_pipeline`, `compose_preflight`,
    `compose_preflight_approvals`, `validate_pre_approved`, the `collect_*`
    entry points, `normalize_links`, `link_resolve`,
    `transclusions_with_options`, `ReferenceGraphOptions::with_compose`,
    `shell_expansion::execute_directive`, and
    `execute_resolved_shell_values` onto `&ComposeRequest` (or the context
    it owns). The Phase 1 helper is deleted.
  - Delete `Markdown::compose()` / `compose_mut()`; `compose_with` takes
    `&ComposeRequest`.
  - Delete `magic_paths` from `ComposeOptions` and `TransclusionOptions`
    (R3).
  - Nested sources keep deriving through `SourceOpening` /
    `source_file_context`.
- [x] **`ctx.*` from the request**
  - Feed the following from the request's context (R4) instead of the
    process:
    - `RuntimeContext::capture` / `capture_minimal`
      (`context/runtime.rs:140,160,182`);
    - the `env` capture in `context/capture/mod.rs:131`;
    - the `AGENT` reads (`capture/agent.rs:40,66,85`,
      `expression/resolve_ctx.rs:263`);
    - home (`resolve_ctx.rs:275`, `options.rs:1547,1601`);
    - `env.*` (`conditions.rs:379`).
  - Test that an expression `{{ ctx.env.X }}` and a `{{X}}` file reference
    in one request see the same snapshot value while the process value
    differs.
- [x] **Migrate darkmatter's own tests**
  - Callers of `compose()`, `compose_mut()`, `with_magic_path`, and
    `capture_file_resolution_context` in `darkmatter/lib` tests move to
    `RequestSnapshot::new(fixture_dir)` + `ComposeRequest::prepare`.
- [x] **Checkpoint**
  - `just test` and `just lint` in `darkmatter/` (lib).
  - Downstream compile errors are expected and are Phase 3's input. List
    them in the implementation log.

## Phase 3: Consumers on the Builder

Goal: every non-DMLS consumer gets its context from the builder, and every
binary calls `from_process()` exactly once (R8). Acceptance Criteria 3 and 7
hold, apart from DMLS. **Phase 4 depends only on Phase 2 and may run
concurrently with this phase.**

### Wave 5 (parallel; one subagent per track, disjoint crates)

- [ ] **Track A: `md` CLI**
  - **Snapshot.** `main` calls `RequestSnapshot::from_process()` once and
    threads it through the command context.
  - **`compose.rs`.**
    - The launch context (`:210`) becomes a builder call.
    - The foreign-repository context (`:281`) becomes
      `builder(snapshot.at_request_dir(..))` plus the trusted-external
      derivation (`:297-301`), per R9.
    - The derivation at `:250` stays.
    - The context is **always** attached through
      `ComposeRequest::with_context`, not only with `--set` (`:360-364`),
      and validation, preflight, and the pipeline share it.
    - Remove the ambient read at `:193`.
  - **Other routes.** `validate`, `graph` (`FileTree` from the request
    instead of `from_markdown`'s current directory), `schema
    validate|triggers|detect`, and `clean` take the built context.
  - **Document arguments.** `resolve_file_path` (`io/mod.rs:66`) takes
    `&FileResolutionContext`. That covers `hash`, `frontmatter get|set|rm`,
    `toc`, `delta`, and `code-block` (Table 2 "md arguments"). Remove the
    reads at `frontmatter.rs:258-277` and `code_block.rs:222`.
  - **Failure row.** Add R5's `failure` detail row to the rendered
    file-reference error.
  - **Tests.**
    - Acceptance Criterion 7: from inside a package, `md compose` of
      `::file ^pkg-only.md` resolves.
    - The `failure` row appears for a missing `&` target.
    - Every existing `cli/tests` suite passes.
- [ ] **Track B: Claudine (lib, then cli; one subagent, sequential)**
  - **Snapshot.** `InvocationContext` captures from a `RequestSnapshot`.
    `EnvBaseline` and `HomeBaseline` are built from it, and `capture()`,
    `capture_at`, and `capture_for_wrapper` take it (R8).
    `build_file_resolution_context` (`invocation_context.rs:2130`) calls the
    builder, passing `with_prompt_magic_roots`' roots as snapshot roots.
    `absolutize` (`:2234`) uses the snapshot's request directory.
  - **Composition.** Apply R9 in `composition/resolve.rs:62,124`,
    `composition/sequence/source.rs:90`, `harness/resolve.rs:171`, and
    `system_prompt/resolve.rs:278`. Composition and preflight
    (`composition/preflight.rs:82,139`, `prepare.rs:881`) use
    `ComposeRequest::with_context`.
  - **Binary.** `main` calls `from_process()` once. Every
    `InvocationContext::capture*` and `ScopeContext::discover` call site
    takes the threaded snapshot:
    - `commands/sequence.rs:267`
    - `compose/prep.rs:199`
    - `wrap/composition/prep_context.rs:161`
    - `providers.rs:454`
    - `wrap/overlay.rs:177`
    - `wrap/harness_orch/shell_options.rs:276,298`
    - `wrap/env/mod.rs:52`
    - `completion/engine/mod.rs:190,207,216`
    - `completion/operation_file.rs:231`
    - `schema_interactive/mod.rs:284,639`
  - **Completion.** `completion/scopes.rs:240` calls the builder.
    `resolve_provided_file_reference` (`schema_interactive/mod.rs:278`) and
    `resolve_file_value` (`:741-743`, ambient `reference.resolve()`) resolve
    through the built context.
  - **Checks.** `just test` and `just lint` in `claudine/`. Update the
    `claudine` skill's architecture notes.
- [ ] **Track C: claudine-gen and messenger**
  - **claudine-gen.** `main` calls `from_process()` once.
    `resolve_area`'s `current_dir` read (`gen/src/main.rs:489`) uses the
    snapshot. `generator_schemas` (`inputs.rs:243`) calls the builder with
    `snapshot.at_request_dir(area)`.
  - **messenger.** The binary calls `from_process()` once. `research.rs`
    `execute` builds through `snapshot.at_request_dir(root)` and passes the
    result to `Loader::new`, whose signature takes the context. Remove the
    forced `with_repository_root` and check R13 first.
  - **Checks.** `just test` and `just lint` in `claudine/` (gen) and
    `messenger/`.

### Wave 6 (after Wave 5)

- [ ] **Checkpoint**
  - Every in-scope package builds and passes `just test`.
  - `rg 'FileResolutionContext::(new|from_snapshot)'` outside tests shows
    only the builder. Record the residue (should be none) in the
    implementation log.

## Phase 4: DMLS on One Context per Repository

Goal: change 1 for DMLS. Acceptance Criteria 8, 9 (outside the matrix), 12,
13, and 14 hold. This phase depends on Phase 2 (and on sniff's list from
Wave 3).

### Wave 7 (sequential foundation)

- [x] **Snapshot at startup**
  - `RunOptions` gains `snapshot: RequestSnapshot`, and `main.rs` fills it
    with `from_process()` once.
  - The test fixture (`tests/common/mod.rs` `LspFixture::start`) passes a
    fixture snapshot with its own `HOME`.
- [x] **Repository context cache**
  - Add a cache to `ServerState` (or `OverlayCache`), keyed per R10 and
    storing `Result<FileResolutionContext, ContextBuildError>`. On a miss it
    discovers the repository from the document's folder and calls the
    builder with `snapshot.at_request_dir(repo_root)` (or the folder when
    there is no repository).
  - Each document derives with `for_source(doc)`. `DocumentContext`
    (`providers/mod.rs:49`) gains the derived context or the cached error.
  - Count builds with a `work-counters` counter so tests can assert reuse.

### Wave 8 (parallel; disjoint files; depends on Wave 7)

- [x] **Schema validation and diagnostics**
  - `overlay/schema.rs` `assemble` passes the derived context through
    `DarkmatterSchemas::with_file_resolution_context` and drops
    `with_file_ref_fallback_dir`.
  - File-reference diagnostics carry R5's `data`.
  - Fix the `trigger_boundary` comment drift (R12).
- [x] **Link graph**
  - `graph/arena.rs` transclusion edges (`:361`), `diagnose_unresolved`
    (`:677`), `resolve_link` (`:737`), and `resolve_file_edge` (`:771`)
    parse a `FileReference` and resolve through the document's context.
  - `normalize_join` remains only for non-reference joins (anchors), or is
    deleted if unused.
- [x] **Providers**
  - Move the following from `normalize_join` / `resolve_local_path` onto
    the context:
    - frontmatter `nav_targets` (`providers/frontmatter.rs:1399`), used by
      definition and document links;
    - DSL `resolve_local_path` (`providers/dsl.rs:1080`), used by hover,
      definition, transclusion links, and transclusion diagnostics;
    - the code action `create_missing_markdown_file`
      (`code_actions.rs:124`);
    - `anchor_completions` (`completion.rs:107`).

### Wave 9 (parallel; depends on Wave 8)

- [x] **Invalidation**
  - `workspace/watch.rs` registers `**/<name>` for each
    `PACKAGE_MANIFEST_FILE_NAMES` entry; update the watcher-count tests.
  - `apply_watched_changes` (`router.rs:1090`) drops the containing
    repository's cache entries.
  - `reload_config` (`:385`) drops all entries.
  - `rescan_workspace` (`:350`) returns changed paths, including manifests,
    which drop entries the same way.
  - Each drop refreshes the diagnostics of affected open documents and
    re-resolves their graph edges.
- [x] **Context-failure diagnostic**
  - On a cached `Err`, publish exactly one ERROR diagnostic at 0:0 with
    `dm.context.build_failure`, a message naming the typed failure and the
    directory, and `data` carrying the class. Log it with
    `tracing::error!`.
  - Document links, go-to-definition, and file-value validation return
    nothing for that document; features that need no resolution are
    unaffected. There is no fallback context.
- [x] **Untitled buffers**
  - Let `untitled:` documents through `with_document` (today
    `uri_to_file_path` returns `None`, `workspace/mod.rs:29`).
  - Count repositories across workspace folders per R10. Exactly one
    repository gives that repository's context with `cwd` = its root;
    otherwise the context-failure diagnostic.

### Wave 10 (parallel tests; depend on Wave 9)

- [x] **Acceptance Criterion 8**
  - Two documents in one repository produce one build, observed through
    the counter.
  - The request directory is the repository root even when the workspace
    folder is above the repository.
  - A document in no repository resolves against its own folder.
- [x] **Acceptance Criterion 12**
  - **Watched client** (`watched_initialize_params`): `^pkg-only.md` fails,
    the test adds a package, sends `didChangeWatchedFiles` naming only its
    manifest, and the reference resolves.
  - **Rescan client** (`neovim_like_initialize_params`): the same change is
    detected on rescan with no notification.
  - `didChangeConfiguration` drops every entry.
  - A snapshot `HOME` differing from the test process's `HOME` is the one
    used (startup-only environment).
- [x] **Acceptance Criterion 13**
  - Using R11's filesystem input: exactly one diagnostic at 0:0 with the
    code, the typed failure, and the directory, plus the log line.
  - Links, definition, and validation return nothing.
  - After the cause is fixed and the entry dropped, the diagnostic clears
    and the features return.
- [x] **Acceptance Criterion 14**
  - With one repository across the workspace folders, an untitled
    `&root-only.md` resolves.
  - With folders in two repositories, or in none, the diagnostic appears
    and nothing resolves.
- [x] **Checkpoint** (dmls test, L2, lint, and Windows/Linux cross-checks pass; the 7 darkmatter-cli test and 3 clippy failures predate this phase (Phase 3 commits, verified on a clean `HEAD`); the commit is left to the separate commit process)
  - `just test`, `just test-l2`, and `just lint` in `darkmatter/` (dmls).
  - Commit(s) for change 1 per R16.

## Phase 5: Required Context and No Ambient Reads (Change 2)

Goal: Acceptance Criteria 2 and 5 hold in code; the guards in Phase 6 then
lock them. Depends on Phases 3 and 4.

### Wave 11 (sequential; darkmatter lib, one subagent)

- [x] **`schemas/` to required context** (`with_file_ref_fallback_dir` kept: it anchors `match()` globs; see the log)
  - Replace every `Option<FileResolutionContext>` /
    `Option<&FileResolutionContext>` in `format.rs` (3), `resolve.rs` (16,
    including the `ImportEngine` field), `rewrite.rs` (8), `validate.rs` (6:
    `ValidatorCache`, `CacheEntry`, `lookup`, `insert`,
    `build_validator_in_context`, `canonical_hash`), `mod.rs` (2:
    `DarkmatterSchemas`, `EffectiveSchema`), and `file_match.rs` (3).
  - In `file_match.rs`, **only the context parameter** of `admits`,
    `match_keyword_factory`, and `MatchKeyword` changes. Their separate
    `base_dir: Option<&Path>` and the `current_dir` fallback at `:185` stay
    for the glob feature.
  - Delete `with_file_ref_fallback_dir` (`schemas/mod.rs:293-301`, which
    has no effect on resolution) once DMLS no longer calls it.
  - `DarkmatterSchemas` without a context is no longer constructible on a
    request path.
- [x] **`compose/` to required context**
  - `document_resolution_context` (`util.rs:72`) takes
    `&FileResolutionContext` and loses the `FileResolutionContext::new(cwd)`
    fallback. `source_link_context` changes the same way.
  - `ShellExpansionOptions` (`types.rs:428`) and `TransclusionOptions`
    (`options.rs:2142`) change, as do `file_resolution_context()`,
    `source_file_resolution_context`, and `encode_file_resolution_context`.
  - In `expression/`, `ResolutionContext` (`resolve_ctx.rs:101`) and its
    four functions, `path_projection.rs` (3), and `path_display_components`
    change.
  - `reference/mod.rs` `options_with_reference_resolution_context` goes.
- [x] **Remaining ambient reads in darkmatter** (residue: `capture/mod.rs` `std::env::vars()`; see the log)
  - Remove the reads listed in change 2:
    - `schemas/format.rs:425` (`resolved_from`);
    - anything left in `runtime.rs`;
    - `schemas/detect.rs:146`, `schemas/mod.rs:1752` (`base_dir_for`);
    - `reference/file_tree/mod.rs:181`;
    - `path_projection.rs:80`.
  - Leave `resolve_boundary` and `admits` (handed off).
- [x] **`PortablePath` with context**
  - `link_normalization.rs:194` always calls `with_ctx` with the request's
    context; the no-context branch (`:196-198`) goes.
  - Test that a non-file source's links normalize against the request
    context.

### Wave 12 (parallel; depends on Wave 11)

- [ ] **Claudine `Option` contexts** (BLOCKED: Claudine does not compile until Phase 3 Track B lands; see the log)
  - Make the 16 sites required:
    - lib: `composition/types.rs:518`, `prepare.rs:146,485`,
      `sequence/preflight/mod.rs:214,260,269`, `composition/mod.rs:194`,
      `preflight.rs:326`, `looping/engine.rs:72,591`,
      `schema/mod.rs:422`, `sequence/mod.rs:205`;
    - cli: `compose/loop_run.rs:181`,
      `wrap/composition/preflight.rs:140`,
      `wrap/harness_orch/types.rs:108,149`.
  - Run `just test` in `claudine/`.
- [x] **CLI and DMLS fallout** (plus claudine-gen, messenger)
  - Fix any compile fallout in darkmatter-cli, dmls, claudine-gen, and
    messenger. No new constructions.
- [ ] **Checkpoint** (darkmatter, messenger, and claudine-gen tests and lint pass; Claudine is blocked as above; commits are left to the separate commit process)
  - Run `just test` and `just lint` in every touched area.
  - Commit(s) for change 2 per R16.

## Phase 6: Source-Scan Guards

Goal: Acceptance Criteria 2, 3, and 5 are enforced by guard tests with exact
allowlists (Decisions 18 and 21). **May run concurrently with Phase 7.**

### Wave 13 (sequential)

- [x] **Shared guard engine**
  - Create `darkmatter/cli/tests/common/context_guard.rs` on top of
    `source_scan.rs` (comments, strings, and `#[cfg(test)]` items blanked;
    identifier-boundary matching; `#[cfg(test)] mod x;` files excluded the
    way `semantic_results_never_persist.rs` does).
  - It runs three gates over a crate's `src/`:
    - **Construction:** `FileResolutionContext::new`, `::from_snapshot`,
      and the `from_process` call count from R8.
    - **Optional context:** `Option<FileResolutionContext>` and
      `Option<&FileResolutionContext>`, any spacing or path prefix, with an
      empty allowlist.
    - **Ambient state:** the R7 identifiers.
  - Each allowlist entry is `(path, identifier, count, reason)`. An unused
    entry, a moved count, or a new site fails.
  - Self-tests: a seeded source string with each violation is rejected,
    and a seeded stale entry is rejected.

### Wave 14 (parallel; one subagent per group; depends on Wave 13)

- [x] **darkmatter group** (lib, cli, dmls)
  - Add a `context_construction_guard.rs` per package.
  - Allowlists:
    - Construction: the builder only, plus `from_process` once in each of
      `md` and `dmls`.
    - Ambient state: the census, classified per R7. Include
      `resolve_boundary` (`file_links/discovery.rs:79`) and `admits`
      (`file_match.rs:185`), each with reason `handed off to
      2026-09-30-glob-reference`.
  - Declare the shared files in `source-inputs` for lib and dmls.
- [ ] **Claudine group** (lib, cli, gen) (claudine-gen done; claudine lib and cli blocked: the `claudine` crate does not compile until Phase 3 Track B lands)
  - The same three guards.
  - Seed the ambient census for the roughly 200 Claudine reads, each with
    a one-line reason. Reads that feed resolution or `ctx.*` must already be
    gone (Phase 3); finding one here is a defect to fix, not to allowlist.
  - Declare `source-inputs`.
- [x] **messenger group** (lib, cli)
  - The same three guards, the census, and `source-inputs`.
- [ ] **Checkpoint** (darkmatter, messenger, biscuit-file, and claudine-gen pass `test`/`lint`; `ci-local --plan` and `RealWorkspaceTestInputTests` accept every declaration; the `claudine/` area cannot run)
  - `just test` in each area.
  - `just ci-local --plan` shows the declared `source-inputs` accepted
    (S2).

## Phase 7: Entry-Point Parity Matrix

Goal: Acceptance Criteria 10 and 11, plus Acceptance Criterion 9 across
every DMLS feature. Depends on Phases 4 and 5; may start alongside Phase 6.

### Wave 15 (sequential)

- [x] **Shared fixture and tables**
  - Build the fixture in `darkmatter/lib/tests/common/entry_point_parity/`
    in a temporary directory:
    - a git monorepo with a package area and a package;
    - documents at three depths;
    - `sibling.md`, `beside.md`, `root-only.md`, `area-doc.md`,
      `pkg-only.md`, and a configured `@` magic root holding
      `magic-doc.md`;
    - a fixture `HOME` outside the repository with `notes/doc.md` and
      `notes/beside.md`;
    - `outside.md` above `HOME` and outside any repository.
  - Define `EntryPoint` (one variant per Table 1 and Table 2 entry point),
    with exhaustive `owner()` and `rows()` matches (no `_` arm). Table 1
    covers form × consumer × depth; Table 2 covers form × launch directory
    (repository root and a nested package).
  - Model rows (a) and (b) per the spec: row (a) only at the pipeline and
    preflight `::file ~/notes/doc.md` and the quoted `md` argument.
  - Expected results: `Expected::File(path)` or
    `Expected::Failure(ResolutionFailure)` (R6).
  - Comparison: files with `canonicalize_simplified` and then
    `PathIdentity`, reported with `to_portable_string`; errors by
    `ResolutionFailure` only.
  - Add the every-variant-has-a-row assertion.

### Wave 16 (parallel; one runner per package; depends on Wave 15)

- [x] **darkmatter runner**
  - Covers the compose pipeline, preflight, and schema validation, with
    the consumers `::file`, `::code`, `::toc-linking <filename>`, and
    schema `file` values. Uses `RequestSnapshot::new` with the fixture
    `HOME` and environment.
- [x] **darkmatter-cli runner**
  - Covers `md` routes through `CliProcessFixture` (fixture `HOME`), and
    Table 2 `md` arguments from both launch directories, including the
    quoted `'~/notes/doc.md'`. Failures are read from R5's `failure` row.
- [x] **dmls runner**
  - Covers schema validation and diagnostics, document links, the link
    graph, go-to-definition, and code actions, through `LspFixture` with a
    fixture snapshot.
  - Row (b) only for the `~` document. Failures are observed per R5.
- [ ] **claudine-cli runner**
  - Covers Claudine composition (Table 1) and Claudine completion of a
    schema `file` value (Table 2), with snapshots injected per R8.

### Wave 17 (after Wave 16)

- [ ] **Fix failing cells**
  - A failing cell is a defect in the entry point, not in the table.
  - Fix it at the entry point, re-run, and log each cell fixed.
- [ ] **Checkpoint**
  - Every runner passes on macOS locally.
  - `just ci-local --plan` reviewed.

## Phase 8: Docs, Skills, Cross-Platform Evidence, and Hand-off

Goal: drift rules satisfied and the fix ready for review. Wave 18 tasks run
in parallel.

### Wave 18 (parallel)

- [x] **darkmatter docs**
  - Update `darkmatter/docs/topics/file-referencing.md`,
    `magic-paths.md`, `transclusion.md`, `darkmatter-compose-pipeline.md`,
    and `composition/index.md` to cover:
    - request preparation (`ComposeRequest`);
    - the snapshot and builder, and that a library call names its request
      directory;
    - magic roots entering only through the snapshot;
    - `ctx.*` and `env.*` read from the snapshot (and `current_env` live,
      per R7).
  - Add the `failure` row to `darkmatter/docs/errors/`. Lead with what the
    reader can do and give compact examples.
  - Add a Mermaid diagram for snapshot → builder → `ComposeRequest`.
- [x] **DMLS docs**
  - Update `darkmatter/docs/topics/dmls.md`, `docs/lsp/architecture.md`,
    `docs/lsp/features.md`, and `dmls/docs/diagnostics.md` to cover:
    - one context per repository, with the repository root as the request
      directory;
    - the environment is the editor's, fixed for the server's lifetime
      (restart to change; GUI launches on macOS may lack shell variables);
    - invalidation, including manifests and rescan;
    - the `dm.context.build_failure` diagnostic;
    - untitled buffers.
  - Include the spec's flowchart, adapted.
- [x] **Skills** (Claudine skill and `claudine/docs` left as-is: Claudine's code was not ported, so they still describe it; see the Phase 8 log)
  - Update the `darkmatter` skill (`SKILL.md` "Composition authority",
    `compose.md`, `dmls.md`, `library-surfaces.md`), the `claudine` skill
    (`architecture.md`), the `biscuit-file` skill (R5 accessor), and the
    `sniff` skill (R10 const).
  - Update `claudine/docs/topics/composition.md`, `system-prompt.md`, and
    `completions/shell-completions.md` where they describe context
    capture.
- [x] **Windows evidence** (all pass after one test-fixture fix; claudine and claudine-cli fail only because they do not compile)
  - Run `just cross-check <pkg> --os windows` for darkmatter,
    darkmatter-cli, dmls, claudine, claudine-cli, claudine-gen, messenger,
    messenger-cli, biscuit-file, and sniff.
  - Record each result in the implementation log. Fix any failure, loading
    the `os` skill first.

### Wave 19 (sequential)

- [x] **Implementation log and status** (status set to `human-in-the-loop`, not a ready-for-review value: Claudine is unported)
  - Complete `implementation-log.md`: every changed output (`md` error
    `failure` row, DMLS diagnostic and `data`, watcher registration,
    deleted APIs) and every departure (R4 signature, R5, R7 `current_env`,
    R12 drift, any S1 outcome).
  - Set the spec's `status` to "implementation complete, ready for
    review". Do not move the spec and do not run `just complete`.
- [x] **Final checkpoint** (every compiling area green; the claudine area cannot run until Claudine compiles)
  - `just test` and `just lint` in every touched area.
  - `git verify-commit` for each commit on the branch since `e23d0c2b3`.
  - Hand-off note: the feature `2026-09-30-glob-reference` is next on
    `fix/magic-globs`. It must delete the two handed-off allowlist entries
    and add its rows and `EntryPoint` variants. The branch is not merged
    until both are done (Decision 16). Watching the post-merge push to
    `main` is the merger's step.
