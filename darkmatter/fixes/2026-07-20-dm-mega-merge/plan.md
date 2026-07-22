---
agent: opencode/zai-coding-plan/glm-5.2
total_phases: 8
created: 2026-07-21
phase: 8
source_files_during_phase_1:
  - claudine/cli/src/commands/context/format.rs
  - claudine/lib/src/composition/lifecycle/executor.rs
  - claudine/lib/src/composition/lifecycle/executor/tests.rs
  - claudine/lib/src/composition/lifecycle/tests.rs
  - claudine/lib/src/composition/lifecycle/validate.rs
  - claudine/lib/src/composition/looping/config.rs
  - claudine/lib/src/composition/preflight.rs
  - claudine/lib/src/composition/schema/classify.rs
  - claudine/lib/src/dispatch/matcher.rs
  - darkmatter/cli/src/commands/compose.rs
  - darkmatter/cli/src/commands/schema/about.rs
  - darkmatter/cli/tests/level2_code_block_styling.rs
  - darkmatter/cli/tests/level2_errors.rs
  - darkmatter/cli/tests/schema_about.rs
  - darkmatter/dmls/src/diagnostics/codes.rs
  - darkmatter/dmls/src/diagnostics/frontmatter.rs
  - darkmatter/dmls/src/graph/substrate.rs
  - darkmatter/dmls/src/overlay/doc_links.rs
  - darkmatter/dmls/src/overlay/expressions.rs
  - darkmatter/dmls/src/overlay/frontmatter.rs
  - darkmatter/dmls/src/overlay/mod.rs
  - darkmatter/dmls/src/overlay/schema.rs
  - darkmatter/dmls/src/providers/dsl.rs
  - darkmatter/dmls/src/providers/frontmatter.rs
  - darkmatter/dmls/src/providers/mod.rs
  - darkmatter/dmls/tests/lsp_session.rs
  - darkmatter/dmls/tests/no_side_effects.rs
  - darkmatter/lib/src/markdown/compose/context/capture/git.rs
  - darkmatter/lib/src/markdown/compose/context/capture/groups.rs
  - darkmatter/lib/src/markdown/compose/context/capture/mod.rs
  - darkmatter/lib/src/markdown/compose/context/capture/snapshot.rs
  - darkmatter/lib/src/markdown/compose/context/catalog.rs
  - darkmatter/lib/src/markdown/compose/context/options.rs
  - darkmatter/lib/src/markdown/compose/expression/ast.rs
  - darkmatter/lib/src/markdown/compose/expression/catalog/ast.rs
  - darkmatter/lib/src/markdown/compose/expression/catalog/mod.rs
  - darkmatter/lib/src/markdown/compose/expression/catalog/parser.rs
  - darkmatter/lib/src/markdown/compose/expression/error.rs
  - darkmatter/lib/src/markdown/compose/expression/functions/cicd.rs
  - darkmatter/lib/src/markdown/compose/expression/functions/escape.rs
  - darkmatter/lib/src/markdown/compose/expression/functions/git.rs
  - darkmatter/lib/src/markdown/compose/expression/functions/mod.rs
  - darkmatter/lib/src/markdown/compose/expression/functions/paths.rs
  - darkmatter/lib/src/markdown/compose/expression/functions/provider.rs
  - darkmatter/lib/src/markdown/compose/expression/functions/pull_requests.rs
  - darkmatter/lib/src/markdown/compose/expression/lexer.rs
  - darkmatter/lib/src/markdown/compose/expression/mod.rs
  - darkmatter/lib/src/markdown/compose/expression/parser.rs
  - darkmatter/lib/src/markdown/compose/expression/resolve_ctx.rs
  - darkmatter/lib/src/markdown/compose/frontmatter_interpolation.rs
  - darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs
  - darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion/tests/tests.rs
  - darkmatter/lib/src/markdown/compose/interpolation/fatality_characterization.rs
  - darkmatter/lib/src/markdown/compose/pipeline/mod.rs
  - darkmatter/lib/src/markdown/compose/remote.rs
  - darkmatter/lib/src/markdown/compose/remote_fetch.rs
  - darkmatter/lib/src/markdown/compose/subtree.rs
  - darkmatter/lib/src/markdown/compose/tests/mod.rs
  - darkmatter/lib/src/markdown/compose/tests/provider_network.rs
  - darkmatter/lib/src/markdown/compose/tests/transclusion.rs
  - darkmatter/lib/src/markdown/errors/blocks.rs
  - darkmatter/lib/src/markdown/schemas/about.rs
  - darkmatter/lib/src/markdown/schemas/coerce.rs
  - darkmatter/lib/src/markdown/schemas/errors.rs
  - darkmatter/lib/src/markdown/schemas/format.rs
  - darkmatter/lib/src/markdown/schemas/mod.rs
  - darkmatter/lib/src/markdown/schemas/reference.rs
  - darkmatter/lib/src/markdown/schemas/resolve.rs
  - darkmatter/lib/src/markdown/schemas/simplified/convert.rs
  - darkmatter/lib/src/markdown/schemas/simplified/cursor.rs
  - darkmatter/lib/src/markdown/schemas/simplified/grammar.rs
  - darkmatter/lib/src/markdown/schemas/simplified/mod.rs
  - darkmatter/lib/src/markdown/schemas/simplified/source.rs
  - darkmatter/lib/src/markdown/schemas/simplified/standalone.rs
  - darkmatter/lib/src/markdown/schemas/simplified/types.rs
  - darkmatter/lib/src/markdown/schemas/simplified/yaml_scalar.rs
  - darkmatter/lib/src/markdown/schemas/triggers/matcher.rs
  - darkmatter/lib/src/markdown/schemas/validate.rs
  - darkmatter/lib/tests/git_context_integration.rs
  - darkmatter/lib/tests/level2_render_tree_terminal/images.rs
  - darkmatter/lib/tests/level2_render_tree_terminal/support/mod.rs
  - darkmatter/lib/tests/level3_image_painting.rs
  - darkmatter/lib/tests/meta_schema_phase1.rs
  - darkmatter/lib/tests/meta_schema_phase3.rs
  - darkmatter/lib/tests/meta_schema_phase4.rs
  - darkmatter/lib/tests/meta_schema_phase5.rs
  - darkmatter/lib/tests/meta_schema_phase6.rs
  - darkmatter/lib/tests/meta_schema_reference_graph.rs
  - darkmatter/lib/tests/meta_schema_repo_schemas.rs
  - darkmatter/lib/tests/more_is_more_literals_and_indexes.rs
  - darkmatter/lib/tests/predict_conflicts.rs
  - darkmatter/lib/tests/schemas_grammar_proptest.rs
  - darkmatter/lib/tests/schemas_source_projection.rs
  - darkmatter/lib/tests/suggest_constraint_phase4.rs
  - sniff/cli/tests/snapshots.rs
  - sniff/lib/src/credentials.rs
  - sniff/lib/src/error.rs
  - sniff/lib/src/filesystem/blast_radius.rs
  - sniff/lib/src/filesystem/formatting.rs
  - sniff/lib/src/filesystem/git/api.rs
  - sniff/lib/src/filesystem/git/merge_conflicts.rs
  - sniff/lib/src/filesystem/git/mod.rs
  - sniff/lib/src/filesystem/git/open.rs
  - sniff/lib/src/filesystem/git/recent_commits.rs
  - sniff/lib/src/filesystem/git/remote_observation.rs
  - sniff/lib/src/filesystem/git/remote_refresh.rs
  - sniff/lib/src/filesystem/git/remote_resolver.rs
  - sniff/lib/src/filesystem/git/status.rs
  - sniff/lib/src/filesystem/git/types.rs
  - sniff/lib/src/filesystem/git/worktree.rs
  - sniff/lib/src/filesystem/mod.rs
  - sniff/lib/src/filesystem/repo/area.rs
  - sniff/lib/src/filesystem/repo/identity.rs
  - sniff/lib/src/lib.rs
  - sniff/lib/src/network/mod.rs
  - sniff/lib/src/remote/focused.rs
  - sniff/lib/src/remote/mod.rs
  - sniff/lib/src/remote/provider.rs
  - sniff/lib/src/remote/provider_url.rs
  - sniff/lib/src/remote/types.rs
  - sniff/lib/src/remote/url_parser.rs
  - sniff/lib/src/remote/web_link.rs
  - sniff/lib/tests/focused_provider.rs
  - sniff/lib/tests/git_parity.rs
  - sniff/lib/tests/integration.rs
  - sniff/lib/tests/merge_conflict_prediction.rs
  - sniff/lib/tests/remote_observation.rs
  - sniff/lib/tests/remote_resolution.rs
docs_updated_during_phase_1:
  - .claudine/memory/commits.md
  - CLAUDE.md
  - darkmatter/dmls/docs/diagnostics.md
  - darkmatter/docs/topics/context-variables.md
  - darkmatter/docs/topics/darkmatter-expressions.md
  - darkmatter/docs/topics/schema-definition.md
  - darkmatter/features/2026-07-13-meta-schema/plan.md
  - darkmatter/features/2026-07-13-meta-schema/spec.md
  - darkmatter/features/2026-07-13-more-is-more/plan.md
  - darkmatter/features/2026-07-13-more-is-more/spec.md
  - darkmatter/features/2026-07-15-performance-followup/review-7.md
  - darkmatter/fixes/2026-07-20-dm-mega-merge/plan.md
  - docs/testing-strategy.md
  - sniff/docs/sniff-library-architecture.md
  - sniff/lib/README.md
docs_created_during_phase_1:
  - darkmatter/features/2026-07-13-meta-schema/log.md
  - darkmatter/features/2026-07-13-meta-schema/phase1-impact.md
  - darkmatter/features/2026-07-13-meta-schema/phase1-test-matrix.md
  - darkmatter/features/2026-07-13-meta-schema/phase2-test-map.md
  - darkmatter/features/2026-07-13-meta-schema/phase3-test-map.md
  - darkmatter/features/2026-07-13-meta-schema/phase4-test-map.md
  - darkmatter/features/2026-07-13-meta-schema/phase5-baseline-replay.md
  - darkmatter/features/2026-07-13-meta-schema/phase5-test-map.md
  - darkmatter/features/2026-07-13-meta-schema/phase6-test-map.md
  - darkmatter/features/2026-07-13-meta-schema/phase7-test-map.md
  - darkmatter/features/2026-07-13-meta-schema/review-10.md
  - darkmatter/features/2026-07-13-meta-schema/review-11.md
  - darkmatter/features/2026-07-13-meta-schema/review-12.md
  - darkmatter/features/2026-07-13-meta-schema/review-13.md
  - darkmatter/features/2026-07-13-meta-schema/review-14.md
  - darkmatter/features/2026-07-13-meta-schema/review-2.md
  - darkmatter/features/2026-07-13-meta-schema/review-3.md
  - darkmatter/features/2026-07-13-meta-schema/review-4.md
  - darkmatter/features/2026-07-13-meta-schema/review-5.md
  - darkmatter/features/2026-07-13-meta-schema/review-6.md
  - darkmatter/features/2026-07-13-meta-schema/review-7.md
  - darkmatter/features/2026-07-13-meta-schema/review-8.md
  - darkmatter/features/2026-07-13-meta-schema/review-9.md
  - darkmatter/features/2026-07-13-more-is-more/log.md
  - darkmatter/features/2026-07-13-more-is-more/review-15.md
  - darkmatter/features/2026-07-13-more-is-more/review-16.md
  - darkmatter/features/2026-07-13-more-is-more/review-17.md
  - darkmatter/features/2026-07-13-more-is-more/review-18.md
  - darkmatter/features/2026-07-13-more-is-more/review-19.md
  - darkmatter/features/2026-07-13-more-is-more/review-20.md
  - darkmatter/features/2026-07-13-more-is-more/review-21.md
  - darkmatter/features/2026-07-13-more-is-more/review-22.md
  - darkmatter/features/2026-07-13-more-is-more/review-23.md
  - darkmatter/features/2026-07-13-more-is-more/review-24.md
  - darkmatter/features/2026-07-13-more-is-more/review-25.md
  - darkmatter/features/2026-07-13-more-is-more/review-26.md
  - darkmatter/features/2026-07-13-more-is-more/review-27.md
  - darkmatter/features/2026-07-13-more-is-more/review-plan-19.md
  - darkmatter/fixes/2026-07-20-dm-mega-merge/resolution-record.md
  - prompts/_implement/implement-review-findings-plan.md
  - prompts/_implement/review-findings-plan.md
skills_files_updated_during_phase_1:
  - .claude/skills/darkmatter/SKILL.md
  - .claude/skills/rust-devops/SKILL.md
  - .claude/skills/rust-devops/gitoxide.md
  - .claude/skills/sniff/SKILL.md
source_files_during_phase_2: []
docs_updated_during_phase_2:
  - darkmatter/fixes/2026-07-20-dm-mega-merge/plan.md
  - darkmatter/fixes/2026-07-20-dm-mega-merge/resolution-record.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - darkmatter/cli/tests/level2_code_block_styling.rs
  - darkmatter/cli/tests/level2_errors.rs
docs_updated_during_phase_3:
  - darkmatter/fixes/2026-07-20-dm-mega-merge/plan.md
  - darkmatter/fixes/2026-07-20-dm-mega-merge/resolution-record.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
packages_during_phase_3:
  - darkmatter-cli
source_files_during_phase_4: []
docs_updated_during_phase_4:
  - .claudine/memory/commits.md
  - CLAUDE.md
  - darkmatter/features/2026-07-15-performance-followup/review-7.md
  - darkmatter/features/2026-07-15-performance-followup/review-10.md
  - darkmatter/fixes/2026-07-20-dm-mega-merge/plan.md
  - darkmatter/fixes/2026-07-20-dm-mega-merge/resolution-record.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
  - .claude/skills/darkmatter/SKILL.md
packages_during_phase_4: []
source_files_during_phase_5:
  - darkmatter/cli/tests/level2_schema_about.rs
docs_updated_during_phase_5:
  - darkmatter/fixes/2026-07-20-dm-mega-merge/plan.md
  - darkmatter/fixes/2026-07-20-dm-mega-merge/resolution-record.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5: []
packages_during_phase_5:
  - darkmatter-cli
source_files_during_phase_6:
  - claudine/cli/src/commands/wrap/harness_orch/loop_control.rs
  - claudine/cli/src/commands/wrap/harness_orch/prompt.rs
  - claudine/cli/tests/level2_context_capture.rs
  - sniff/cli/src/output/repo_json.rs
  - sniff/cli/tests/cli.rs
  - sniff/lib/src/lib.rs
  - sniff/lib/tests/integration.rs
docs_updated_during_phase_6:
  - claudine/docs/providers/dispatch-inventory.json
  - darkmatter/fixes/2026-07-20-dm-mega-merge/plan.md
  - darkmatter/fixes/2026-07-20-dm-mega-merge/resolution-record.md
docs_created_during_phase_6: []
skills_files_updated_during_phase_6: []
packages_during_phase_6:
  - claudine-cli
  - sniff
  - sniff-cli
source_files_during_phase_7: []
docs_updated_during_phase_7:
  - darkmatter/fixes/2026-07-20-dm-mega-merge/plan.md
  - darkmatter/fixes/2026-07-20-dm-mega-merge/resolution-record.md
docs_created_during_phase_7:
  - darkmatter/fixes/2026-07-20-dm-mega-merge/merge-report.md
skills_files_updated_during_phase_7:
  - .claude/skills/darkmatter/SKILL.md
packages_during_phase_7: []
source_files_during_phase_8: []
docs_updated_during_phase_8:
  - darkmatter/fixes/2026-07-20-dm-mega-merge/merge-report.md
  - darkmatter/fixes/2026-07-20-dm-mega-merge/plan.md
  - darkmatter/fixes/2026-07-20-dm-mega-merge/resolution-record.md
docs_created_during_phase_8: []
skills_files_updated_during_phase_8: []
packages_during_phase_8: []
source_code:
  - claudine/cli/src/commands/context/format.rs
  - claudine/cli/src/commands/wrap/harness_orch/loop_control.rs
  - claudine/cli/src/commands/wrap/harness_orch/prompt.rs
  - claudine/cli/tests/level2_context_capture.rs
  - claudine/lib/src/composition/lifecycle/executor.rs
  - claudine/lib/src/composition/lifecycle/executor/tests.rs
  - claudine/lib/src/composition/lifecycle/tests.rs
  - claudine/lib/src/composition/lifecycle/validate.rs
  - claudine/lib/src/composition/looping/config.rs
  - claudine/lib/src/composition/preflight.rs
  - claudine/lib/src/composition/schema/classify.rs
  - claudine/lib/src/dispatch/matcher.rs
  - darkmatter/cli/src/commands/compose.rs
  - darkmatter/cli/src/commands/schema/about.rs
  - darkmatter/cli/tests/level2_code_block_styling.rs
  - darkmatter/cli/tests/level2_errors.rs
  - darkmatter/cli/tests/level2_schema_about.rs
  - darkmatter/cli/tests/schema_about.rs
  - darkmatter/dmls/src/diagnostics/codes.rs
  - darkmatter/dmls/src/diagnostics/frontmatter.rs
  - darkmatter/dmls/src/graph/substrate.rs
  - darkmatter/dmls/src/overlay/doc_links.rs
  - darkmatter/dmls/src/overlay/expressions.rs
  - darkmatter/dmls/src/overlay/frontmatter.rs
  - darkmatter/dmls/src/overlay/mod.rs
  - darkmatter/dmls/src/overlay/schema.rs
  - darkmatter/dmls/src/providers/dsl.rs
  - darkmatter/dmls/src/providers/frontmatter.rs
  - darkmatter/dmls/src/providers/mod.rs
  - darkmatter/dmls/tests/lsp_session.rs
  - darkmatter/dmls/tests/no_side_effects.rs
  - darkmatter/lib/src/markdown/compose/context/capture/git.rs
  - darkmatter/lib/src/markdown/compose/context/capture/groups.rs
  - darkmatter/lib/src/markdown/compose/context/capture/mod.rs
  - darkmatter/lib/src/markdown/compose/context/capture/snapshot.rs
  - darkmatter/lib/src/markdown/compose/context/catalog.rs
  - darkmatter/lib/src/markdown/compose/context/options.rs
  - darkmatter/lib/src/markdown/compose/expression/ast.rs
  - darkmatter/lib/src/markdown/compose/expression/catalog/ast.rs
  - darkmatter/lib/src/markdown/compose/expression/catalog/mod.rs
  - darkmatter/lib/src/markdown/compose/expression/catalog/parser.rs
  - darkmatter/lib/src/markdown/compose/expression/error.rs
  - darkmatter/lib/src/markdown/compose/expression/functions/cicd.rs
  - darkmatter/lib/src/markdown/compose/expression/functions/escape.rs
  - darkmatter/lib/src/markdown/compose/expression/functions/git.rs
  - darkmatter/lib/src/markdown/compose/expression/functions/mod.rs
  - darkmatter/lib/src/markdown/compose/expression/functions/paths.rs
  - darkmatter/lib/src/markdown/compose/expression/functions/provider.rs
  - darkmatter/lib/src/markdown/compose/expression/functions/pull_requests.rs
  - darkmatter/lib/src/markdown/compose/expression/lexer.rs
  - darkmatter/lib/src/markdown/compose/expression/mod.rs
  - darkmatter/lib/src/markdown/compose/expression/parser.rs
  - darkmatter/lib/src/markdown/compose/expression/resolve_ctx.rs
  - darkmatter/lib/src/markdown/compose/frontmatter_interpolation.rs
  - darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs
  - darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion/tests/tests.rs
  - darkmatter/lib/src/markdown/compose/interpolation/fatality_characterization.rs
  - darkmatter/lib/src/markdown/compose/pipeline/mod.rs
  - darkmatter/lib/src/markdown/compose/remote.rs
  - darkmatter/lib/src/markdown/compose/remote_fetch.rs
  - darkmatter/lib/src/markdown/compose/subtree.rs
  - darkmatter/lib/src/markdown/compose/tests/mod.rs
  - darkmatter/lib/src/markdown/compose/tests/provider_network.rs
  - darkmatter/lib/src/markdown/compose/tests/transclusion.rs
  - darkmatter/lib/src/markdown/errors/blocks.rs
  - darkmatter/lib/src/markdown/schemas/about.rs
  - darkmatter/lib/src/markdown/schemas/coerce.rs
  - darkmatter/lib/src/markdown/schemas/errors.rs
  - darkmatter/lib/src/markdown/schemas/format.rs
  - darkmatter/lib/src/markdown/schemas/mod.rs
  - darkmatter/lib/src/markdown/schemas/reference.rs
  - darkmatter/lib/src/markdown/schemas/resolve.rs
  - darkmatter/lib/src/markdown/schemas/simplified/convert.rs
  - darkmatter/lib/src/markdown/schemas/simplified/cursor.rs
  - darkmatter/lib/src/markdown/schemas/simplified/grammar.rs
  - darkmatter/lib/src/markdown/schemas/simplified/mod.rs
  - darkmatter/lib/src/markdown/schemas/simplified/source.rs
  - darkmatter/lib/src/markdown/schemas/simplified/standalone.rs
  - darkmatter/lib/src/markdown/schemas/simplified/types.rs
  - darkmatter/lib/src/markdown/schemas/simplified/yaml_scalar.rs
  - darkmatter/lib/src/markdown/schemas/triggers/matcher.rs
  - darkmatter/lib/src/markdown/schemas/validate.rs
  - darkmatter/lib/tests/git_context_integration.rs
  - darkmatter/lib/tests/level2_render_tree_terminal/images.rs
  - darkmatter/lib/tests/level2_render_tree_terminal/support/mod.rs
  - darkmatter/lib/tests/level3_image_painting.rs
  - darkmatter/lib/tests/meta_schema_phase1.rs
  - darkmatter/lib/tests/meta_schema_phase3.rs
  - darkmatter/lib/tests/meta_schema_phase4.rs
  - darkmatter/lib/tests/meta_schema_phase5.rs
  - darkmatter/lib/tests/meta_schema_phase6.rs
  - darkmatter/lib/tests/meta_schema_reference_graph.rs
  - darkmatter/lib/tests/meta_schema_repo_schemas.rs
  - darkmatter/lib/tests/more_is_more_literals_and_indexes.rs
  - darkmatter/lib/tests/predict_conflicts.rs
  - darkmatter/lib/tests/schemas_grammar_proptest.rs
  - darkmatter/lib/tests/schemas_source_projection.rs
  - darkmatter/lib/tests/suggest_constraint_phase4.rs
  - sniff/cli/src/output/repo_json.rs
  - sniff/cli/tests/cli.rs
  - sniff/cli/tests/snapshots.rs
  - sniff/lib/src/credentials.rs
  - sniff/lib/src/error.rs
  - sniff/lib/src/filesystem/blast_radius.rs
  - sniff/lib/src/filesystem/formatting.rs
  - sniff/lib/src/filesystem/git/api.rs
  - sniff/lib/src/filesystem/git/merge_conflicts.rs
  - sniff/lib/src/filesystem/git/mod.rs
  - sniff/lib/src/filesystem/git/open.rs
  - sniff/lib/src/filesystem/git/recent_commits.rs
  - sniff/lib/src/filesystem/git/remote_observation.rs
  - sniff/lib/src/filesystem/git/remote_refresh.rs
  - sniff/lib/src/filesystem/git/remote_resolver.rs
  - sniff/lib/src/filesystem/git/status.rs
  - sniff/lib/src/filesystem/git/types.rs
  - sniff/lib/src/filesystem/git/worktree.rs
  - sniff/lib/src/filesystem/mod.rs
  - sniff/lib/src/filesystem/repo/area.rs
  - sniff/lib/src/filesystem/repo/identity.rs
  - sniff/lib/src/lib.rs
  - sniff/lib/src/network/mod.rs
  - sniff/lib/src/remote/focused.rs
  - sniff/lib/src/remote/mod.rs
  - sniff/lib/src/remote/provider.rs
  - sniff/lib/src/remote/provider_url.rs
  - sniff/lib/src/remote/types.rs
  - sniff/lib/src/remote/url_parser.rs
  - sniff/lib/src/remote/web_link.rs
  - sniff/lib/tests/focused_provider.rs
  - sniff/lib/tests/git_parity.rs
  - sniff/lib/tests/integration.rs
  - sniff/lib/tests/merge_conflict_prediction.rs
  - sniff/lib/tests/remote_observation.rs
  - sniff/lib/tests/remote_resolution.rs
documentation:
  - .claudine/memory/commits.md
  - CLAUDE.md
  - claudine/docs/providers/dispatch-inventory.json
  - darkmatter/dmls/docs/diagnostics.md
  - darkmatter/docs/topics/context-variables.md
  - darkmatter/docs/topics/darkmatter-expressions.md
  - darkmatter/docs/topics/schema-definition.md
  - darkmatter/features/2026-07-13-meta-schema/log.md
  - darkmatter/features/2026-07-13-meta-schema/phase1-impact.md
  - darkmatter/features/2026-07-13-meta-schema/phase1-test-matrix.md
  - darkmatter/features/2026-07-13-meta-schema/phase2-test-map.md
  - darkmatter/features/2026-07-13-meta-schema/phase3-test-map.md
  - darkmatter/features/2026-07-13-meta-schema/phase4-test-map.md
  - darkmatter/features/2026-07-13-meta-schema/phase5-baseline-replay.md
  - darkmatter/features/2026-07-13-meta-schema/phase5-test-map.md
  - darkmatter/features/2026-07-13-meta-schema/phase6-test-map.md
  - darkmatter/features/2026-07-13-meta-schema/phase7-test-map.md
  - darkmatter/features/2026-07-13-meta-schema/plan.md
  - darkmatter/features/2026-07-13-meta-schema/review-10.md
  - darkmatter/features/2026-07-13-meta-schema/review-11.md
  - darkmatter/features/2026-07-13-meta-schema/review-12.md
  - darkmatter/features/2026-07-13-meta-schema/review-13.md
  - darkmatter/features/2026-07-13-meta-schema/review-14.md
  - darkmatter/features/2026-07-13-meta-schema/review-2.md
  - darkmatter/features/2026-07-13-meta-schema/review-3.md
  - darkmatter/features/2026-07-13-meta-schema/review-4.md
  - darkmatter/features/2026-07-13-meta-schema/review-5.md
  - darkmatter/features/2026-07-13-meta-schema/review-6.md
  - darkmatter/features/2026-07-13-meta-schema/review-7.md
  - darkmatter/features/2026-07-13-meta-schema/review-8.md
  - darkmatter/features/2026-07-13-meta-schema/review-9.md
  - darkmatter/features/2026-07-13-meta-schema/spec.md
  - darkmatter/features/2026-07-13-more-is-more/log.md
  - darkmatter/features/2026-07-13-more-is-more/plan.md
  - darkmatter/features/2026-07-13-more-is-more/review-15.md
  - darkmatter/features/2026-07-13-more-is-more/review-16.md
  - darkmatter/features/2026-07-13-more-is-more/review-17.md
  - darkmatter/features/2026-07-13-more-is-more/review-18.md
  - darkmatter/features/2026-07-13-more-is-more/review-19.md
  - darkmatter/features/2026-07-13-more-is-more/review-20.md
  - darkmatter/features/2026-07-13-more-is-more/review-21.md
  - darkmatter/features/2026-07-13-more-is-more/review-22.md
  - darkmatter/features/2026-07-13-more-is-more/review-23.md
  - darkmatter/features/2026-07-13-more-is-more/review-24.md
  - darkmatter/features/2026-07-13-more-is-more/review-25.md
  - darkmatter/features/2026-07-13-more-is-more/review-26.md
  - darkmatter/features/2026-07-13-more-is-more/review-27.md
  - darkmatter/features/2026-07-13-more-is-more/review-plan-19.md
  - darkmatter/features/2026-07-13-more-is-more/spec.md
  - darkmatter/features/2026-07-15-performance-followup/review-10.md
  - darkmatter/features/2026-07-15-performance-followup/review-7.md
  - darkmatter/fixes/2026-07-20-dm-mega-merge/merge-report.md
  - darkmatter/fixes/2026-07-20-dm-mega-merge/plan.md
  - darkmatter/fixes/2026-07-20-dm-mega-merge/resolution-record.md
  - docs/testing-strategy.md
  - prompts/_implement/implement-review-findings-plan.md
  - prompts/_implement/review-findings-plan.md
  - sniff/docs/sniff-library-architecture.md
  - sniff/lib/README.md
packages:
  - claudine
  - claudine-cli
  - darkmatter
  - darkmatter-cli
  - dmls
  - sniff
  - sniff-cli
yolo: "false"
reviewed: true
reviewed_by: codex/default
reviewed_on: 2026-07-21
---

# Execution Plan: Darkmatter and More-Is-More Integration Merge

## References

- **Specification**: `darkmatter/fixes/2026-07-20-dm-mega-merge/spec.md`
- **Conflict report**: `darkmatter/fixes/2026-07-20-dm-mega-merge/conflict-report.md`
- **`darkmatter` branch log**: `darkmatter/fixes/2026-07-20-dm-mega-merge/darkmatter-log.md`
- **`more-is-more` branch log**: `darkmatter/fixes/2026-07-20-dm-mega-merge/more-is-more-log.md`
- **Research workbook**: `darkmatter/fixes/2026-07-20-dm-mega-merge/_research.md`

## Pinned Inputs

These immutable object IDs are the validity envelope for the plan:

| Input | Revision |
|---|---|
| Merge base | `d672388dd0fed4196295e7f21514cac6fa59f0ae` |
| `darkmatter` parent | `14dd391f45206d58383ba9d84adbf53c65520534` |
| `more-is-more` parent | `0584d8297f57f5eb30b52d03b1241ba55184bb44` |

Moving branch names do not change these pins. Stop only if an object is missing,
the computed merge base differs, or an operator explicitly requests different
source commits. An intentional repin invalidates the branch deltas, overlap
inventory, conflict preview, scope record, and this plan until they are
regenerated and reviewed.

## Candidate Scope, Not Final Gate Scope

Package and package-area scope are different records. The minimum candidate
package-area scope from spec R2 is:

| Package area | Candidate packages | Reason |
|---|---|---|
| `biscuit-file` | `biscuit-file`, `biscuit-file-cli` | YAML source analysis and repair authority |
| `sniff` | `sniff`, `sniff-cli` | Git, worktree, remote, provider, and credential authority |
| `darkmatter` | `darkmatter`, `darkmatter-cli` | Compose, expressions, schemas, cleanup, references, and CLI |
| `darkmatter/dmls` | `dmls` | Passive consumers of expression, schema, and graph products |
| `claudine` | Discovered area members affected by the final graph | Container-expression and semantic-schema downstream behavior |

`biscuit-terminal` is an unchanged upstream boundary, not an unconditional gate
area. Its trees at both pins must be identical. Add its area gates only if the
actual merge changes that tree or the final Sniff/GitNexus evidence expands
scope to it. Darkmatter's own Level 2 suite remains mandatory for the terminal
integration seam.

This table is only a starting point. Phase 0 records packages, package areas,
workspace members, and dependency edges separately. Phase 5 freezes the final
gate scope after the merged symbol set is known. A package or area may be
removed only with both Sniff dependency evidence and GitNexus impact evidence.

## Evidence Storage and Control-Artifact Disposition

The reviewed spec and its inputs include dirty or newly added source-worktree
content that is not present in both pinned commits. This creates a bootstrap
constraint: the integration worktree must be clean when the merge starts, so
the preflight record cannot initially live inside it.

Use two evidence locations:

1. **External preflight ledger** — create a dedicated path outside every Git
   worktree with `mktemp -d`; record its exact absolute path and retain it
   through handoff. Phase 0 writes a preflight manifest there; Phase 1 freezes
   and hashes it before conflict edits. Later raw logs may be added as separate
   files without changing that manifest.
2. **Authoritative integration record** — after the no-commit merge starts,
   create
   `darkmatter/fixes/2026-07-20-dm-mega-merge/resolution-record.md` in the
   integration worktree. Import a concise Phase 0 summary and record the
   external ledger path plus the immutable manifest's content hash. The
   specification itself authorizes this new evidence artifact.

Before Phase 1, every existing control artifact must have an explicit
disposition in the external ledger:

- `external-only`: readable evidence, never added to the integration index; or
- `authorized-documentation-delta`: copied only after conflict resolution,
  verified against its frozen working-content hash, and separately identified
  in the staged-diff audit.

Do not infer authorization from the artifact being staged, modified, or
untracked in a source worktree. A missing disposition blocks final handoff, not
the read-only preflight.

## Cross-Cutting Controls

- Set `GIT_TERMINAL_PROMPT=0` for every Git command. All Git operations are
  one-shot and non-interactive.
- One operator owns the integration index and all edits. Read-only audits may
  be prepared independently, but concurrent edits or staging in the same
  worktree are forbidden.
- One validation owner runs all build/test/lint commands from the integration
  worktree. No other worktree may run competing Rust or terminal gates.
- Whole-file `ours`/`theirs` resolution is forbidden for production files. It
  is allowed elsewhere only after a path-level audit proves one side is
  intentionally identical or obsolete and records that proof.
- Process discovery is read-only. Stop a process only when it is attributable
  to this integration and stopping it is separately authorized. Otherwise
  wait, isolate the target directory, or report the deferred gate.
- Record and export one dedicated `CARGO_TARGET_DIR`, `CARGO_BUILD_JOBS`, and
  `NEXTEST_TEST_THREADS` budget. Also record `BISCUIT_L2_THREADS` wherever an
  area recipe uses owned-pane Level 2 concurrency.
- Invoke real-terminal tests only through the owning area's `just test-l2`
  recipe. Preserve the recipe's shared-pane or owned-pane policy; never run
  Level 2 tests directly through Nextest or concurrently from another
  worktree.
- Focused Level 1 Rust tests use Nextest. `cargo test` is forbidden. Final
  gates use the affected package area's `just build`, `just test`,
  `just test-l2`, and `just lint` recipes.
- Never use a bare root Cargo build/check/test, `--workspace`, an unscoped root
  lifecycle recipe, or write-mode `cargo fmt`/`rustfmt`.
- Keep edits surgical and match surrounding formatting by hand. Any behavior
  change includes a pass over its docs and comments; do not mix unrelated
  cleanup or stale-comment narration into the merge.
- Every Cargo inspection or execution uses the existing lockfile. A
  `Cargo.lock` content change is a failure until explicitly explained and
  reviewed.
- Remote/provider acceptance tests are loopback-fixture-only. Sanitize ambient
  provider-token and credential-helper influence without recording secret
  values; record only the names of variables/config channels neutralized.
- Generated metadata is refreshed once, after all gates and corrective edits:
  Darkmatter skill hash first, GitNexus index/counts second.
- Any test or gate failure returns to the owning resolution phase. Do not patch
  forward in Phase 5, 6, or 7 without updating impact/scope evidence and
  invalidating any later evidence already collected.

## Resolution Record Contract

Maintain one path-resolution entry for each of the ten shared net-change paths:

1. `.claude/skills/darkmatter/SKILL.md` — textual conflict;
2. `.claudine/memory/commits.md` — textual conflict;
3. `CLAUDE.md` — textual conflict;
4. `darkmatter/cli/tests/level2_code_block_styling.rs` — textual conflict;
5. `darkmatter/cli/tests/level2_errors.rs` — textual conflict;
6. `darkmatter/features/2026-07-15-performance-followup/review-8.md` — modify/delete;
7. `darkmatter/lib/Cargo.toml` — auto-merge audit;
8. `darkmatter/lib/src/markdown/schemas/mod.rs` — auto-merge audit;
9. `darkmatter/lib/src/markdown/schemas/validate.rs` — auto-merge audit;
10. `darkmatter/cli/src/commands/compose.rs` — auto-merge audit.

Each entry records the conflict/audit type, both parent contributions, chosen
merged structure, governing requirement, authority boundary, GitNexus symbols
and flows where applicable, focused evidence IDs, and deferred follow-up. New
overlaps, conflicts, or snapshot changes get entries before work continues.

Path decisions are append-only after closure. Later test results live in a
separate evidence ledger section and refer back to the path entry by ID.

---

## Phase 0 — Freeze Inputs, Scope Evidence, and Provision a Clean Worktree

**Goal**: Establish immutable inputs, recoverability, exact source-worktree
state, a clean integration worktree, and pre-edit impact evidence without
changing either source worktree. (Spec Phase 0; R1, R2, R11, R12.)

**Depends on**: nothing.

**Checkpoint**: Both source worktrees are unchanged; all pins and the merge
base are proven; backup refs exist; the integration worktree is clean at the
pinned `darkmatter` parent; the external ledger contains scope, resource, and
control-artifact evidence.

- [x] Create the external preflight directory with `mktemp -d`, record its
  absolute path, designate the single integration/validation owner, and name
  the immutable preflight manifest that will be hashed before Phase 1 edits.
- [x] With `GIT_TERMINAL_PROMPT=0`, prove each pin is a commit using
  `git cat-file -e <sha>^{commit}` and record `git show -s --format=fuller` for
  each object.
- [x] Run `git merge-base <darkmatter-pin> <more-is-more-pin>` and require the
  exact pinned merge base. Record `git rev-list --left-right --count` as a
  corroborating branch-delta check.
- [x] Recompute the ten-path intersection from the two merge-base deltas and
  run the read-only three-tree `git merge-tree <base> <ours> <theirs>` preview.
  Require the expected ten shared paths and six predicted conflicts. Do not use
  a preview mode that writes a tree object.
- [x] Compare the `biscuit-terminal` subtree object at both pins and require
  byte-identical trees; otherwise add the area to candidate scope and update
  the conflict/overlap audit before proceeding.
- [x] For the spec and every listed input, record:
  - [x] absolute path and `git status --short -- <path>`;
  - [x] HEAD blob ID when present;
  - [x] index blob ID when present; and
  - [x] exact working-content ID from `git hash-object --no-filters <path>`.
- [x] Record an `external-only` or `authorized-documentation-delta`
  disposition for each control artifact. Stop before Phase 1 if the operator
  has not made this decision.
- [x] Record byte-for-byte `git status --short` output for both source
  worktrees and an identity hash for every dirty/untracked file that must be
  preserved. Do not rely on a prose list of today's known dirty files.
- [x] Inventory potentially interfering Cargo, rustc, linker, Nextest,
  Criterion, and terminal-harness processes. Record PID/command/worktree
  attribution without secrets. Apply the process rule above; do not issue a
  blanket termination command.
- [x] Record the relevant host capacity with Sniff and choose conservative,
  fixed values for `CARGO_BUILD_JOBS`, `NEXTEST_TEST_THREADS`, and any
  `BISCUIT_L2_THREADS` override.
- [x] Record discovery separately from the repository root:
  - [x] `sniff repo packages --json`;
  - [x] `sniff repo package-areas --json`;
  - [x] `sniff repo package-dependencies --json`; and
  - [x] `cargo metadata --locked --no-deps --format-version 1`.
- [x] Summarize the workspace member count and affected manifest paths from
  Cargo metadata; never hardcode the member count or infer membership from
  directory names. Record the `Cargo.lock` content ID before inspection.
- [x] Freeze the retained benchmark artifacts, performance evidence, fixture
  hashes, snapshots, and review-chain files named by the source feature specs.
  Record content identities rather than copying them into the integration
  worktree.
- [x] Verify the GitNexus indexes used for both pinned parents name the exact
  pinned commits. Use `query`/`context` to resolve ambiguous symbols to UIDs,
  then record upstream `impact` with `includeTests: true` for the affected
  public functions/types in the four auto-merged production paths and the R8
  seam directories. Record direct callers, processes, modules, risk, and both
  parent perspectives where a symbol exists on only one parent.
- [x] Surface every HIGH/CRITICAL impact result to the authorizing operator
  before Phase 2 edits and attach a focused-regression obligation. If a later
  edit touches an unrecorded symbol, run impact for that UID before editing it.
- [x] Create backup refs without overwriting an existing different target.
  If a proposed ref exists, proceed only after proving it already names the
  same pin; otherwise choose a new name.
- [x] Choose a new integration branch name and worktree path. If either exists,
  do not delete or reuse it automatically; prove it is the intended clean
  object or choose a collision-free name.
- [x] Create the integration worktree from the exact `darkmatter` pin and
  require `git status --porcelain` to be empty and `HEAD` to equal that pin.
- [x] Create a dedicated Cargo target directory with `mktemp -d`, record the
  absolute path, and retain it through handoff.

---

## Phase 1 — Create and Inventory the No-Commit Merge

**Goal**: Enter the unresolved merge state, verify actual conflicts against the
preview, and bootstrap the authoritative integration record before editing any
conflict. (Spec Phase 1; R1, R6, R12.)

**Depends on**: Phase 0.

**Checkpoint**: `HEAD` and `MERGE_HEAD` prove the intended parents; every
unmerged path is mapped to a requirement; no conflict marker has been edited;
the resolution record contains the frozen preflight summary.

- [x] From the clean integration worktree run, with
  `GIT_TERMINAL_PROMPT=0`,
  `git merge --no-commit --no-ff 0584d8297f57f5eb30b52d03b1241ba55184bb44`.
  Do not commit.
- [x] Record `git status --short`, `git diff --name-only --diff-filter=U`, and
  `git ls-files -u` in the external ledger. Do not create ad hoc status files
  inside the integration worktree.
- [x] Require the six predicted conflicts, including the Review 8
  modify/delete case. If the set differs, stop, update the inventory, and
  explain whether the discrepancy invalidates the preview.
- [x] Prove `HEAD` is the pinned `darkmatter` parent and `MERGE_HEAD` is the
  pinned `more-is-more` parent.
- [x] Freeze the external preflight manifest and compute its content identity;
  subsequent raw logs use separate files.
- [x] Create `resolution-record.md` in the integration worktree, summarize
  Phase 0, and record the external ledger path and immutable-manifest hash.
- [x] Create a stub record for all ten shared paths plus any unexpected path.
  Do not edit conflict content until every unmerged path has a governing
  requirement.

---

## Phase 2 — Resolve and Audit Production Authority Seams

**Goal**: Semantically audit the four auto-merged production paths and the
cross-file unions before resolving tests or documentation. (Spec Phase 2; R5,
R7, R8.)

**Depends on**: Phase 1.

**Checkpoint**: All production seams preserve both parents' behavior; locked
metadata succeeds without lockfile drift; any corrective production edit is
staged and linked to pre-edit impact evidence.

### 2a — `darkmatter/lib/Cargo.toml`

- [x] Confirm `sniff` retains the `remote` feature, `git2` remains dev-only,
  and `clean_hot_paths` remains a benchmark target.
- [x] Confirm no second production Git implementation was introduced.
- [x] Run `cargo metadata --locked --no-deps --format-version 1` from the repo
  root and verify the `Cargo.lock` content ID is unchanged.

### 2b — `darkmatter/lib/src/markdown/schemas/mod.rs`

- [x] Use GitNexus UIDs rather than ambiguous names for the clean facade,
  `effective_for_with_override`, raw-validation entry points,
  `SchemaReference`, and source-aware parser/cursor/span exports.
- [x] Preserve the extracted test-module layout, clean-analysis exports,
  schema override/raw-validation seams, deterministic ordering, bounded schema
  references, and every DMLS source-aware export.
- [x] Do not restore the removed inline god-test module.

### 2c — `darkmatter/lib/src/markdown/schemas/validate.rs`

- [x] Preserve helper visibility required by schema-clean analysis.
- [x] Preserve the URL-scheme, `type-definition`, and `schema` custom
  validator registrations.
- [x] Keep raw and coercing validation as distinct testable paths.

### 2d — `darkmatter/cli/src/commands/compose.rs`

- [x] Preserve the shared `env_disables_baseline_schema` rule used by clean.
- [x] Preserve removal of obsolete approval-error bindings.
- [x] Preserve focused provider/approval error classification and rendering.

### 2e — Directory-level propagation audit

- [x] `markdown/compose/context`: demand-driven `ctx.branch`, `ctx.worktree`,
  and `ctx.merge_conflicts`; shared repository discovery; independent
  degradation.
- [x] `markdown/compose/expression`: structured literals, postfix indexing,
  indexed-file functions, and runtime-policy propagation through root and
  nested compose paths.
- [x] `markdown/schemas`: one grammar/AST authority, clean analysis,
  references/cycles/depth, nominal validators, source products, recursion
  limit, serializer/descriptors, and base `$schema: schema` declaration.
- [x] DMLS graph/overlay/providers/diagnostics: catalog-driven passive
  behavior only; no shell, network, repository mutation, or composition.
- [x] Darkmatter clean/compose CLI: baseline-disable parity, output behavior,
  idempotency, and save/stdout equivalence.
- [x] Sniff Git/remote: caller-anchored direction-correct read-only conflict
  prediction; preferred remote/provider/flavor/credential authority.
- [x] Claudine lifecycle/classification/rendering: traversal and validation of
  container expressions and nominal schema values.
- [x] Check propagation across frontmatter, body, `$()` branches, nested
  subtrees, CLI commands, and passive DMLS projections.

### 2f — Closeout

- [x] For every corrective edit, update behavior docs/comments in scope and
  stage the reviewed path with `git add -- <path>`. Auto-merged paths that need
  no edit remain staged by the merge.
- [x] Scan changed production text for conflict markers and investigate every
  match; do not use a repository-wide scan that mistakes committed fixtures
  for unresolved conflicts.
- [x] Re-run locked metadata and the lockfile identity check.
- [x] Close the four production path records with planned focused evidence
  IDs. Test results are appended later in the evidence ledger.

---

## Phase 3 — Resolve Test Conflicts and Audit Harness Topology

**Goal**: Resolve the two Level 2 test conflicts around the centralized helper
without restoring obsolete harness code. (Spec Phase 3; R3.4, R6.)

**Depends on**: Phase 2.

**Checkpoint**: Both conflicted test paths are marker-free and prepared for the
operator's separate staging step; no duplicate terminal harness/parser/
validator/formatter authority was restored; focused test commands are
identified, with the L1 helper-integrity selector executed in this phase.

- [x] Resolve `level2_code_block_styling.rs` using the centralized
  `tests/common/level2.rs` helper.
  - [x] Do not restore the local tmux harness, sentinel loop, fixture writer,
    or `run_md_in_tmux` helper.
  - [x] Port only unique incoming build-shim or terminal-discovery coverage
    not already represented by the shared helper.
  - [x] Preserve the area recipe's serialization/resource policy.
- [x] Resolve `level2_errors.rs` with one canonically ordered `md_shim` import
  and the current shared build shim.
- [x] Prepare each resolved test path for explicit staging. Per the operator's
  no-staging instruction, the working files are marker-free and byte-identical
  to the selected parent blobs, while their unmerged index entries are an
  explicitly deferred handoff item.
- [x] Audit auto-merged tests for schema/clean, expressions/context,
  references, DMLS, Sniff Git/remote/credentials, and Claudine downstream
  behavior. Map existing test binaries/cases to every Phase 5 seam.
- [x] Confirm no duplicate tmux harness, YAML parser/validator, formatter, Git
  implementation, terminal discovery, or remote executor was restored.
- [x] Close the two test-conflict records with their focused evidence IDs.

---

## Phase 4 — Resolve Documentation and Policy; Defer Generated Values

**Goal**: Resolve the remaining conflicts and support-file unions, while
deferring derived hashes/counts until all testing and corrective edits are
complete. (Spec Phase 4; R6, R9.)

**Depends on**: Phase 3.

**Checkpoint**: Every remaining conflict has marker-free working content
prepared for the operator's separate staging step; changed text has no
unexplained conflict markers; policy and review history are coherent; skill
hash and GitNexus counts are explicitly marked pending final refresh.

### 4a — Darkmatter skill

- [x] Merge both bodies semantically: cleanup/reference/invalid-frontmatter
  guidance from `darkmatter`; Git context/literals/providers/meta-schema
  guidance from `more-is-more`.
- [x] Reconcile contradictions against the Phase 2 merged code and update
  `last_updated`.
- [x] Prepare the resolved path for explicit staging with the Darkmatter-parent
  hash recorded as temporary and pending; per the operator's no-staging
  instruction, do not claim that value describes the merged body.

### 4b — Commit guidance

- [x] Preserve non-interactive signing/pinentry safety, the prohibition on
  bypassing hooks, and `--only` plus `-F -` argument ordering.
- [x] Prepare the marker-free resolved path for the operator's separate staging
  step.

### 4c — Performance review chain

- [x] Keep the `darkmatter` Review 8 rather than accepting deletion.
- [x] Restore and verify Review 7 -> 8 -> 9 -> 10 links.
- [x] Preserve the open quiet-host evidence status.
- [x] Prepare Review 8 and the intentional link repairs for the operator's
  separate staging step.

### 4d — `CLAUDE.md` and repository support files

- [x] Resolve the count conflict with a clearly recorded temporary parent
  value; final merged-tree counts are written only in Phase 7.
- [x] Preserve both parents' non-generated guidance without importing dirty
  source-worktree edits.
- [x] Review workflow, testing-strategy, review-schema, prompt, skill, and
  public-doc unions for reduced OS coverage, duplicate jobs, or weakened
  process rules.
- [x] Verify historical fixture hashes, benchmark samples, reviews, and open
  invalid-frontmatter evidence gaps were not rewritten or relabeled.
- [x] Prepare every reviewed resolution/support-file edit for the operator's
  separate staging step.

### 4e — Closeout

- [x] Verify every unmerged index path has marker-free resolved working
  content. Per the operator's no-staging instruction, retain the six index
  entries for the separate staging step rather than requiring
  `git ls-files -u` to be empty in this session.
- [x] Scan only changed text files for marker triples and investigate every
  match, with explicit allowance for intentional conflict fixtures.
- [x] Run `git diff --check` and locked Cargo metadata; verify no lockfile
  change.
- [x] Record every path prepared for separate staging in this phase and keep
  the skill-hash and GitNexus-count entries open until Phase 7.

---

## Phase 5 — Freeze Scope and Run Focused Convergence Evidence

**Goal**: Convert the semantic audit into exact, attributable focused tests,
then freeze the package/package-area scope for final gates. (Spec Phase 5; R2,
R3, R4.)

**Depends on**: Phase 4.

**Checkpoint**: Every semantic seam has focused passing evidence; the final
gate scope and any reductions are recorded with both Sniff and GitNexus
evidence; no generated metadata has been refreshed yet.

- [x] Build a focused-test manifest before running tests. Each row records
  package, test binary/case or area recipe, tier, exact command, environment
  budget, expected invariant, and resolution-record evidence ID.
- [x] Use exact Nextest selectors for Level 1 tests. Invoke every Level 2 case
  only through the owning area's `just test-l2`; record recipe, backend,
  passed/skipped counts, and skip reasons instead of inventing a raw Nextest
  filter.
- [x] Run the schema/meta-schema seam first: exports; raw vs. coercing
  validation; nominal keyword registration; references/cycles/depth; source
  projection; deterministic clean ordering; DMLS schema consumers.
- [x] Run invalid-frontmatter after meta-schema: ratified repair/report-only
  matrix; spans/delimiters/line endings/BOM/UTF-8; JSON envelope; flags and
  trigger isolation; fenced-body protection; pinned YAML Test Suite,
  mutation/property guards, zero-work, and parse-count invariants.
- [x] Run compose/provider seams: literals/postfix indexing; indexed-file
  endpoints; demand-driven Git context; nested/frontmatter/body/`$()` runtime
  propagation; fatal provider errors; deny-by-default exact-host policy.
- [x] Run Sniff Git/remote seams: git2 parity oracle; direction-sensitive
  conflict prediction; before/after repository-state proof; preferred remotes;
  provider queries/filter/bounds/order; credential isolation; loopback-only
  fixtures.
- [x] Run reference trust seams: fresh paths skip redundant verification;
  `FileTree::ensure_built`; stale/mismatched prebuilt rejection; changed-child
  behavior; `PreparedHeadingSnapshot` coherence.
- [x] Run cleanup/formatting seams: default/preserve/fixed-width list matrix;
  Unicode widths and hanging prefixes; opaque directives/shell/code/table/HTML
  boundaries; idempotency; library/compose/CLI stdout/CLI save/DMLS parity.
- [x] Run DMLS passive suites: no side effects, completion, hover,
  diagnostics, links, graph integration, LSP sessions, and last-good recovery.
- [x] Run Claudine downstream suites: container traversal and nominal-schema
  classification, validation, lifecycle, rendering, and CLI formatting.
- [x] Run retained performance/compatibility mechanism guards without claiming
  new benchmark evidence: directory-hash membership, no-NTP Darkmatter
  capture, terminal-query caching, shell ordering/timeouts, graph ownership,
  recursion limits, and the redundant-walk mechanism/no-regression guards.
- [x] Run the Darkmatter terminal seam through `just test-l2`: centralized
  helper, build shim, single terminal discovery, stable code-block rendering.
- [x] After focused tests, rerun Sniff package/dependency discovery against the
  integration tree and reconcile it with the recorded impact results. Freeze
  exact packages, package areas, area recipes, and conditional boundaries for
  Phase 6.
- [x] If any test fails, diagnose and return to Phase 2, 3, or 4. Run impact
  before a new symbol edit, update the resolution entry, rerun affected focused
  tests, and re-freeze scope.

---

## Phase 6 — Run Scoped macOS Area Gates Serially

**Goal**: Pass build, Level 1, Level 2, and lint gates for the frozen affected
scope with one owner and one isolated target directory. (Spec Phase 6; R2,
R10, R11.)

**Depends on**: Phase 5.

**Checkpoint**: Every retained package area has passing recorded gates; any
inapplicable canonical recipe reports its intentional no-op; source and lockfile
content remain unchanged except for reviewed corrective edits.

For every command record working directory, exact command, selected area,
`CARGO_TARGET_DIR`, job/thread settings, exit status, pass/fail/skip counts, and
log location. Pass `--locked` through every recipe that accepts trailing Cargo
arguments. For a no-argument recipe, verify the lockfile identity immediately
before and after. Run areas in dependency/authority order:

1. [x] `biscuit-file`: `just build`, `just test`, `just test-l2`, `just lint`.
   Record the canonical Level 2 no-op as such.
2. [x] `sniff`: `just build`, `just test`, `just test-l2`, `just lint`.
3. [x] `darkmatter`: `just build`, `just test`, `just test-l2`, `just lint`.
   Recheck the merged justfile first; the pinned recipe covers `darkmatter`,
   `darkmatter-cli`, and `dmls`, so do not duplicate a separate DMLS gate unless
   the merged recipe no longer does.
4. [x] `claudine`: `just build`, `just test`, `just test-l2`, `just lint` for
   all members enumerated by its merged area recipe.
5. [x] `biscuit-terminal`, only if Phase 5 scope evidence requires it:
   `just build`, `just test`, `just test-l2`, `just lint`.
   Phase 5 evidence did not activate this conditional area.

Do not pass a cargo `-p` argument to a multi-invocation area recipe and assume
it narrows every invocation. Use a narrower route only when the merged justfile
explicitly exposes and documents one; otherwise run the full affected area.

After each area:

- [x] verify `Cargo.lock`, snapshots, fixtures, baselines, and authored docs
  did not change unexpectedly;
- [x] verify no competing source-worktree process appeared; and
- [x] on failure, return to the owning phase rather than continuing to the next
  area.

Native Windows/Linux runs, Level 3 tests, quiet-host benchmarks, and
invalid-frontmatter timing captures are not completion gates. Review the merged
code/tests/CI for path separators, line endings, executable discovery,
credentials, shell assumptions, and `cfg` coverage, and keep existing platform
evidence intact.

---

## Phase 7 — Refresh Derived Metadata Once, Audit the Result, and Hand Off

**Goal**: Generate metadata from the final tested tree, prove the intended
two-parent no-commit topology and scope, produce the final report, and leave the
result unstaged and uncommitted under the implementation prompt's explicit
no-staging override. (Spec Phase 7; R9, R12.)

**Depends on**: Phase 6.

**Checkpoint**: The skill hash matches the tested tree; change intelligence
matches the frozen scope; intended working-tree resolutions are marker-free;
the unresolved index and stale GitNexus counts are explicitly recorded as
handoff blockers; source worktrees are unchanged; no file is staged.

### 7a — Final generated metadata

- [x] Confirm no code, test, or authored skill-body edit remains pending.
- [x] From the integration repository root run the merged CLI with the lockfile:
  `cargo run --locked -p darkmatter-cli --bin md -- hash .claude/skills/darkmatter/SKILL.md --save`.
- [x] Verify immediately by replacing `--save` with `--diff` and require exit
  status 0. Review the intended skill frontmatter delta.
- [x] Record the intended skill frontmatter delta without staging it, as
  required by the implementation prompt.
- [x] Attempt the bounded GitNexus refresh from the integration worktree and
  record the command, indexed commit/worktree, timeout, partial generated paths,
  and the stale-index disposition.
- [x] Retain and label the temporary `CLAUDE.md` counts rather than inventing
  merged-tree counts from an incomplete refresh; review the complete file and
  leave it unstaged.

### 7b — Change intelligence and three-way preservation

- [x] Run GitNexus `detect_changes` with `scope: "all"` and the explicit
  integration worktree path so staged/unstaged merge changes are covered.
- [x] Run the repository-required
  `detect_changes({scope: "compare", base_ref: "main", worktree: <integration>})`.
  Reconcile affected symbols/processes/modules with the frozen Phase 5 scope.
- [x] Compare the working result separately with:
  - [x] pinned `darkmatter` parent (incoming integration delta);
  - [x] pinned `more-is-more` parent (preservation of later Darkmatter work);
  - [x] pinned merge base; and
  - [x] `main` (repository-wide change-detection contract).
- [x] Repeat the four comparisons against the final unstaged working result;
  staging is prohibited by the implementation prompt.
- [x] Investigate every path/symbol outside the recorded scope and every
  expected contribution missing from either-parent comparisons.

### 7c — Final report and index audit

- [x] Create `merge-report.md` with pins, actual conflicts, path resolutions,
  focused evidence, area gates and budgets, generated metadata, control-
  artifact dispositions, and carried-forward gaps. State explicitly that no
  Level 3 or native Windows/Linux result is required or claimed.
- [x] Leave the final resolution record, report, and authorized documentation
  delta unstaged, and record that no `external-only` artifact entered the
  integration worktree.
- [x] Require `git diff --check` to pass and locked Cargo metadata plus the
  original `Cargo.lock` identity to hold.
- [x] Record the six expected unresolved index paths and verify their resolved
  working-tree contents are marker-free; clearing the index would require the
  prohibited staging operation.
- [x] Scan changed text for conflict markers with intentional fixtures
  accounted for. Review every snapshot delta individually.
- [x] Inspect all three views end-to-end:
  - [x] `git diff` (documented unstaged handoff content is intentionally
    retained);
  - [x] `git diff --cached` (records the incomplete merge index without adding
    entries); and
  - [x] `git status --short` (reviewed working-tree and index state only).
- [x] Walk every specification completion criterion and record pass/fail plus
  evidence ID.
- [x] Prove `HEAD`, `MERGE_HEAD`, and the computed merge base still equal the
  three pins. Prove both backup refs remain.
- [x] Re-run source-worktree status and frozen dirty-file identity checks.
- [x] Require Phase 0 byte-for-byte identity after excluding only the explicitly
  authorized phase-plan progress delta.
- [x] Confirm the quiet-host performance-followup gap and invalid-frontmatter
  timing/native-platform gaps remain open and honestly labeled.
- [x] Hand off the integration path, branch, target directory, external ledger,
  resolution record, and merge report. Do not commit, tag, push, delete a
  worktree, or update either source branch.

---

## Failure Loop

```text
Phase 0  freeze/provision
   -> Phase 1  no-commit merge/inventory
   -> Phase 2  production seams
   -> Phase 3  tests/harness topology
   -> Phase 4  docs/policy (derived values pending)
   -> Phase 5  focused tests + final scope
   -> Phase 6  serial area gates
   -> Phase 7  one metadata refresh + final audit

Any failure or corrective edit
   -> return to the owning Phase 2/3/4 task
   -> update impact and resolution evidence
   -> rerun affected Phase 5 focused tests
   -> rerun affected and downstream Phase 6 gates
   -> perform Phase 7 only after the tree is final again
```

There is no parallel editing lane. Nextest and the area recipes may use their
own bounded internal concurrency, but index mutation, area gates, generated
metadata, and final audit remain serial.

## Completion Criteria

The result is ready for separate commit authorization only when:

1. All three pinned objects and the computed merge base are proven.
2. Both source worktrees, dirty files, source refs, and backup refs remain
   intact.
3. `HEAD`/`MERGE_HEAD` prove the intended no-commit two-parent topology.
4. All six conflicts and ten shared paths have requirement-linked records.
5. R3/R4 behavior and every R5 authority boundary have focused evidence.
6. No unmerged index entry or unexplained marker remains.
7. Locked metadata succeeds and `Cargo.lock` is byte-identical.
8. The final Sniff/GitNexus-derived gate scope is recorded, including every
   downstream consumer of changed public types.
9. Focused tests pass through Nextest-backed or owning area commands.
10. Every retained area passes build, Level 1, Level 2, and lint gates on the
    macOS host; intentional no-op tiers are recorded.
11. Cross-platform design/CI evidence is preserved without requiring native
    Windows/Linux execution.
12. No Level 3 result is used.
13. Skill hash and GitNexus counts describe the final tested tree.
14. GitNexus `all` and `compare main` results plus comparisons with both
    parents match the recorded scope.
15. The staged diff contains only source-parent integration, reviewed conflict
    resolutions, generated metadata, required evidence artifacts, and
    separately authorized control-artifact deltas.
16. Historical performance/platform gaps remain visible and non-blocking.
17. `resolution-record.md` and `merge-report.md` are complete, and no commit,
    tag, push, source-branch update, or unsupported readiness claim was made.

Compilation alone is insufficient; completion requires preservation evidence
for behavior, safety, passivity, history, ownership, and repository state.
