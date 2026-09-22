---
title: Consolidate compatible integration tests into shared binaries
spec: 2026-09-21-consolidated-test-binaries
created: 2026-09-21
phase: 7
total_phases: 8
agent: opencode/zai-coding-plan/glm-5.3
yolo: "true"
packages:
    - claudine-cli
    - claudine
    - darkmatter
    - darkmatter-cli
    - biscuit-terminal
    - repo-deps
    - test-toolkit
    - renderable
    - biscuit-file
    - biscuit-terminal-cli
source_files_during_phase_1:
    - features/2026-09-21-consolidated-test-binaries/baseline/capture-listings.sh
    - features/2026-09-21-consolidated-test-binaries/baseline/inventory.py
    - features/2026-09-21-consolidated-test-binaries/baseline/consumer-sweep.py
    - features/2026-09-21-consolidated-test-binaries/baseline/deps-census.py
    - features/2026-09-21-consolidated-test-binaries/spikes/s2-scan.py
    - features/2026-09-21-consolidated-test-binaries/spikes/s3-measure.sh
docs_updated_during_phase_1:
    - features/2026-09-21-consolidated-test-binaries/plan.md
    - features/2026-09-21-consolidated-test-binaries/spec.md
docs_created_during_phase_1:
    - features/2026-09-21-consolidated-test-binaries/rulings.md
    - features/2026-09-21-consolidated-test-binaries/measurements.md
    - features/2026-09-21-consolidated-test-binaries/implementation-log.md
    - features/2026-09-21-consolidated-test-binaries/spikes/s1-nextest-identity.md
    - features/2026-09-21-consolidated-test-binaries/spikes/s2-crate-globals.md
    - features/2026-09-21-consolidated-test-binaries/spikes/s3-measurement.md
    - features/2026-09-21-consolidated-test-binaries/spikes/s4-snapshots.md
    - features/2026-09-21-consolidated-test-binaries/baseline/inventory.md
    - features/2026-09-21-consolidated-test-binaries/baseline/test-selector-consumers.md
    - features/2026-09-21-consolidated-test-binaries/baseline/guard-scans.md
    - features/2026-09-21-consolidated-test-binaries/baseline/disk-and-ci.md
skills_files_updated_during_phase_1: []
packages_during_phase_1: []
source_files_during_phase_2:
    - scripts/ci/consolidation.py
    - scripts/ci/test_consolidation.py
    - scripts/ci/test_ci_local.py
    - just/ci-local.just
    - features/2026-09-21-consolidated-test-binaries/selfproof/mutation-check.py
docs_updated_during_phase_2:
    - features/2026-09-21-consolidated-test-binaries/plan.md
    - features/2026-09-21-consolidated-test-binaries/implementation-log.md
    - features/2026-09-21-consolidated-test-binaries/spec.md
docs_created_during_phase_2:
    - features/2026-09-21-consolidated-test-binaries/selfproof/README.md
    - features/2026-09-21-consolidated-test-binaries/selfproof/noop-comparison.md
skills_files_updated_during_phase_2: []
packages_during_phase_2:
    - repo-deps
source_files_during_phase_3:
    - .config/nextest.toml
    - claudine/cli/Cargo.toml
    - claudine/cli/src/budget/tests.rs
    - claudine/cli/src/commands/wrap/exec/termination/coordinator/tests.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/recovery_identity.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/unowned_handoff.rs
    - claudine/cli/tests/common/mod.rs
    - claudine/cli/tests/l1/agent_cwd.rs
    - claudine/cli/tests/l1/argv_normalization.rs
    - claudine/cli/tests/l1/characterization_error_routes.rs
    - claudine/cli/tests/l1/cli_process_fixture.rs
    - claudine/cli/tests/l1/command_routing.rs
    - claudine/cli/tests/l1/completion_cli.rs
    - claudine/cli/tests/l1/completion_compose.rs
    - claudine/cli/tests/l1/completion_contract.rs
    - claudine/cli/tests/l1/completion_inline_compose.rs
    - claudine/cli/tests/l1/completion_perf.rs
    - claudine/cli/tests/l1/completion_resolution_round_trip.rs
    - claudine/cli/tests/l1/completion_sequence.rs
    - claudine/cli/tests/l1/completion_setter.rs
    - claudine/cli/tests/l1/compose_caller_file_provenance.rs
    - claudine/cli/tests/l1/compose_cli.rs
    - claudine/cli/tests/l1/compose_frontmatter_model.rs
    - claudine/cli/tests/l1/compose_header_first.rs
    - claudine/cli/tests/l1/compose_initialize_acceptance.rs
    - claudine/cli/tests/l1/compose_initialize_staged_boot.rs
    - claudine/cli/tests/l1/compose_interactive_timeout_cli.rs
    - claudine/cli/tests/l1/compose_removed_validation_keys.rs
    - claudine/cli/tests/l1/compose_repository_context.rs
    - claudine/cli/tests/l1/compose_schema_cli.rs
    - claudine/cli/tests/l1/compose_system_prompt_lifetime.rs
    - claudine/cli/tests/l1/compose_ttff_perf.rs
    - claudine/cli/tests/l1/composition_outputs.rs
    - claudine/cli/tests/l1/composition_seams.rs
    - claudine/cli/tests/l1/contamination_probes.rs
    - claudine/cli/tests/l1/context_command.rs
    - claudine/cli/tests/l1/contextual_errors.rs
    - claudine/cli/tests/l1/ctx_launch_anchor.rs
    - claudine/cli/tests/l1/detached_audio.rs
    - claudine/cli/tests/l1/diagnostic_discovery.rs
    - claudine/cli/tests/l1/dispatch_inventory.rs
    - claudine/cli/tests/l1/effective_diagnostic_render.rs
    - claudine/cli/tests/l1/error_guards.rs
    - claudine/cli/tests/l1/error_guards/source_scan.rs
    - claudine/cli/tests/l1/errors_command.rs
    - claudine/cli/tests/l1/handle_blocking_output.rs
    - claudine/cli/tests/l1/handle_deadline.rs
    - claudine/cli/tests/l1/handle_repo_config.rs
    - claudine/cli/tests/l1/hooks_cli.rs
    - claudine/cli/tests/l1/inline_completion_lifecycle.rs
    - claudine/cli/tests/l1/inline_compose_cli.rs
    - claudine/cli/tests/l1/inline_compose_hash.rs
    - claudine/cli/tests/l1/inline_compose_sequence_mismatch.rs
    - claudine/cli/tests/l1/level1_compose_autocomplete_failure_pty.rs
    - claudine/cli/tests/l1/level1_dry_run_pty.rs
    - claudine/cli/tests/l1/level1_inline_compose_mismatch_pty.rs
    - claudine/cli/tests/l1/level1_provided_partial_file_pty.rs
    - claudine/cli/tests/l1/level1_provider_overlay_home.rs
    - claudine/cli/tests/l1/level1_pty_wrapper_summary.rs
    - claudine/cli/tests/l1/level1_review_router_partial_pty.rs
    - claudine/cli/tests/l1/level1_schema_prompt_pty.rs
    - claudine/cli/tests/l1/level1_structured_error_message.rs
    - claudine/cli/tests/l1/loop_cli.rs
    - claudine/cli/tests/l1/loop_initialize_state.rs
    - claudine/cli/tests/l1/main.rs
    - claudine/cli/tests/l1/mcp_cli.rs
    - claudine/cli/tests/l1/prompt_reporting.rs
    - claudine/cli/tests/l1/propagated_context_fixtures.rs
    - claudine/cli/tests/l1/protect_cli.rs
    - claudine/cli/tests/l1/provider_error_finalize.rs
    - claudine/cli/tests/l1/run_harness_loop_call_sites.rs
    - claudine/cli/tests/l1/sequence_budget.rs
    - claudine/cli/tests/l1/sequence_cli.rs
    - claudine/cli/tests/l1/sequence_ctrl_c_windows.rs
    - claudine/cli/tests/l1/sequence_errors_cli.rs
    - claudine/cli/tests/l1/sequence_groups.rs
    - claudine/cli/tests/l1/sequence_initialize_include_preflight.rs
    - claudine/cli/tests/l1/sequence_jit.rs
    - claudine/cli/tests/l1/sequence_magic_reference.rs
    - claudine/cli/tests/l1/sequence_overlay_pty.rs
    - claudine/cli/tests/l1/sequence_perf.rs
    - claudine/cli/tests/l1/sequence_prompt_property.rs
    - claudine/cli/tests/l1/sequence_schema.rs
    - claudine/cli/tests/l1/sequence_sources_cli.rs
    - claudine/cli/tests/l1/shipped_prompt_contract.rs
    - claudine/cli/tests/l1/shipped_prompt_route_drift.rs
    - claudine/cli/tests/l1/shipped_prompts.rs
    - claudine/cli/tests/l1/skills_integration.rs
    - claudine/cli/tests/l1/spawn_site_guard.rs
    - claudine/cli/tests/l1/system_prompt_perf_bench.rs
    - claudine/cli/tests/l1/test_placement.rs
    - claudine/cli/tests/l1/wrap_antigravity_exit_signal.rs
    - claudine/cli/tests/l1/wrap_basics.rs
    - claudine/cli/tests/l1/wrap_compose_agent.rs
    - claudine/cli/tests/l1/wrap_compose_exec.rs
    - claudine/cli/tests/l1/wrap_compose_preflight.rs
    - claudine/cli/tests/l1/wrap_compose_validation.rs
    - claudine/cli/tests/l1/wrap_ctrl_c_windows.rs
    - claudine/cli/tests/l1/wrap_direct_argv.rs
    - claudine/cli/tests/l1/wrap_incomplete_subagents.rs
    - claudine/cli/tests/l1/wrap_inline_compose.rs
    - claudine/cli/tests/l1/wrap_inline_compose_interactive.rs
    - claudine/cli/tests/l1/wrap_opencode.rs
    - claudine/cli/tests/l1/wrap_opencode_models.rs
    - claudine/cli/tests/l1/wrap_perf.rs
    - claudine/cli/tests/l1/wrap_provider_flags.rs
    - claudine/cli/tests/l1/wrap_sequence_composition.rs
    - claudine/cli/tests/l1/wrap_sigint.rs
    - claudine/cli/tests/l1/wrap_structured_stream.rs
    - claudine/cli/tests/l1/wrap_watchdog_startup_stall.rs
    - claudine/cli/tests/l1/wrap_watchdog_timeout.rs
    - claudine/cli/tests/level2/level2_auto_complete_chooser.rs
    - claudine/cli/tests/level2/level2_auto_complete_operation_file.rs
    - claudine/cli/tests/level2/level2_context_capture.rs
    - claudine/cli/tests/level2/level2_dry_run_approval_capture.rs
    - claudine/cli/tests/level2/level2_dry_run_metadata_capture.rs
    - claudine/cli/tests/level2/level2_explicit_operation_file_miss.rs
    - claudine/cli/tests/level2/level2_file_resolution_capture.rs
    - claudine/cli/tests/level2/level2_incomplete_subagents_capture.rs
    - claudine/cli/tests/level2/level2_initialize_generated_transclusion.rs
    - claudine/cli/tests/level2/level2_inline_compose_mismatch_capture.rs
    - claudine/cli/tests/level2/level2_interrupt_feedback_capture.rs
    - claudine/cli/tests/level2/level2_invalid_file_reference_capture.rs
    - claudine/cli/tests/level2/level2_lifecycle_action_forms.rs
    - claudine/cli/tests/level2/level2_lifecycle_control.rs
    - claudine/cli/tests/level2/level2_lifecycle_dispatch.rs
    - claudine/cli/tests/level2/level2_lifecycle_loop.rs
    - claudine/cli/tests/level2/level2_malformed_frontmatter_capture.rs
    - claudine/cli/tests/level2/level2_perf_capture.rs
    - claudine/cli/tests/level2/level2_prompt_reporting_capture.rs
    - claudine/cli/tests/level2/level2_provided_partial_file_capture.rs
    - claudine/cli/tests/level2/level2_provider_overlay_capture.rs
    - claudine/cli/tests/level2/level2_removed_validation_key_capture.rs
    - claudine/cli/tests/level2/level2_schema_parse_capture.rs
    - claudine/cli/tests/level2/level2_sequence_task_stream_capture.rs
    - claudine/cli/tests/level2/level2_stalled_generation_capture.rs
    - claudine/cli/tests/level2/level2_typed_error_render_capture.rs
    - claudine/cli/tests/level2/level2_windows_provided_partial_file_capture.rs
    - claudine/cli/tests/level2/level2_wrap_ctrl_c_loop_wedge_tmux.rs
    - claudine/cli/tests/level2/level2_wrap_ctrl_c_tmux.rs
    - claudine/cli/tests/level2/main.rs
    - claudine/cli/tests/level3/level3_auto_complete_chooser.rs
    - claudine/cli/tests/level3/level3_linux_sequence_ctrl_c.rs
    - claudine/cli/tests/level3/level3_sequence_ctrl_c.rs
    - claudine/cli/tests/level3/level3_windows_sequence_ctrl_c.rs
    - claudine/cli/tests/level3/level3_wrap_ctrl_c.rs
    - claudine/cli/tests/level3/main.rs
    - claudine/cli/tests/real/main.rs
    - claudine/cli/tests/real/real_inline_write_grant.rs
    - claudine/cli/tests/real/real_opencode_yolo_subagent.rs
    - claudine/cli/tests/real/real_pi_steering.rs
    - claudine/justfile
    - claudine/lib/src/provider/mod.rs
    - claudine/lib/src/provider/tests.rs
    - features/2026-09-21-consolidated-test-binaries/pilot/apply-move.py
    - features/2026-09-21-consolidated-test-binaries/spikes/s3-measure.sh
docs_updated_during_phase_3:
    - claudine/docs/providers/dispatch-inventory.json
    - claudine/docs/topics/argv-normalization.md
    - claudine/docs/topics/cli-pre-parsing.md
    - claudine/docs/topics/completions/shell-completions.md
    - claudine/docs/topics/composition.md
    - claudine/docs/topics/error-architecture.md
    - claudine/docs/topics/flow-control/sequences.md
    - claudine/docs/topics/performance-testing.md
    - claudine/docs/topics/provider-metadata.md
    - claudine/docs/topics/signal-handling.md
    - claudine/prompts/create-new-provider.md
    - features/2026-09-21-consolidated-test-binaries/implementation-log.md
    - features/2026-09-21-consolidated-test-binaries/measurements.md
    - features/2026-09-21-consolidated-test-binaries/plan.md
    - features/2026-09-21-consolidated-test-binaries/spec.md
docs_created_during_phase_3:
    - features/2026-09-21-consolidated-test-binaries/claudine-cli-migration.json
    - features/2026-09-21-consolidated-test-binaries/pilot.md
    - features/2026-09-21-consolidated-test-binaries/pilot/attribute-check.json
    - features/2026-09-21-consolidated-test-binaries/pilot/comparison-darwin-linux.json
    - features/2026-09-21-consolidated-test-binaries/pilot/comparison-darwin-linux.md
    - features/2026-09-21-consolidated-test-binaries/pilot/comparison-darwin.json
    - features/2026-09-21-consolidated-test-binaries/pilot/comparison-darwin.md
    - features/2026-09-21-consolidated-test-binaries/pilot/comparison-linux.json
    - features/2026-09-21-consolidated-test-binaries/pilot/comparison-linux.md
    - features/2026-09-21-consolidated-test-binaries/pilot/guard-scans-after.md
    - features/2026-09-21-consolidated-test-binaries/pilot/linux-l1-summary.txt
    - features/2026-09-21-consolidated-test-binaries/pilot/snapshot-check.json
    - features/2026-09-21-consolidated-test-binaries/pilot/snapshot-mapping.json
skills_files_updated_during_phase_3:
    - .claude/skills/biscuit-test-harness/SKILL.md
    - .claude/skills/claudine/SKILL.md
    - .claude/skills/claudine/architecture.md
    - .claude/skills/claudine/argv-normalization.md
    - .claude/skills/claudine/cli-pre-parsing.md
    - .claude/skills/claudine/completions/shell-completions.md
    - .claude/skills/claudine/composition.md
    - .claude/skills/claudine/error-architecture.md
    - .claude/skills/os/build-hosts.md
packages_during_phase_3:
    - claudine-cli
    - claudine
source_files_during_phase_4:
    - .config/nextest.toml
    - Cargo.lock
    - biscuit-file/lib/tests/span_compat.rs
    - darkmatter/cli/tests/level2_code_block_styling.rs
    - darkmatter/cli/tests/level2_disclosure_blocks.rs
    - darkmatter/justfile
    - darkmatter/lib/Cargo.toml
    - darkmatter/lib/src/markdown/render_tree/inline_extension.rs
    - darkmatter/lib/src/markdown/render_tree/style_tree_parity_tests.rs
    - darkmatter/lib/tests/browser/browser_render.rs
    - darkmatter/lib/tests/browser/main.rs
    - darkmatter/lib/tests/l1/ambient_ctx_capture.rs
    - darkmatter/lib/tests/l1/array_rendering_json.rs
    - darkmatter/lib/tests/l1/as_block_error_registry.rs
    - darkmatter/lib/tests/l1/backslash_escape_spans.rs
    - darkmatter/lib/tests/l1/base_schema_end_to_end.rs
    - darkmatter/lib/tests/l1/benchmark_fixtures.rs
    - darkmatter/lib/tests/l1/blockquote_list_spacing.rs
    - darkmatter/lib/tests/l1/clean_counters.rs
    - darkmatter/lib/tests/l1/compose_phase6.rs
    - darkmatter/lib/tests/l1/compose_reuse_phase5.rs
    - darkmatter/lib/tests/l1/cutover_reference.rs
    - darkmatter/lib/tests/l1/debug_test.rs
    - darkmatter/lib/tests/l1/declined_path_transclusion.rs
    - darkmatter/lib/tests/l1/disclosure_render_targets.rs
    - darkmatter/lib/tests/l1/disclosure_transclusion_integration.rs
    - darkmatter/lib/tests/l1/effects_integration.rs
    - darkmatter/lib/tests/l1/error_snapshots/condition.rs
    - darkmatter/lib/tests/l1/error_snapshots/ctx_merge.rs
    - darkmatter/lib/tests/l1/error_snapshots/deferred_set.rs
    - darkmatter/lib/tests/l1/error_snapshots/editor.rs
    - darkmatter/lib/tests/l1/error_snapshots/file_tree.rs
    - darkmatter/lib/tests/l1/error_snapshots/helpers.rs
    - darkmatter/lib/tests/l1/error_snapshots/image_ref.rs
    - darkmatter/lib/tests/l1/error_snapshots/link.rs
    - darkmatter/lib/tests/l1/error_snapshots/markdown_error.rs
    - darkmatter/lib/tests/l1/error_snapshots/mermaid_theme.rs
    - darkmatter/lib/tests/l1/error_snapshots/mod.rs
    - darkmatter/lib/tests/l1/error_snapshots/normalization.rs
    - darkmatter/lib/tests/l1/error_snapshots/page_block.rs
    - darkmatter/lib/tests/l1/error_snapshots/reference.rs
    - darkmatter/lib/tests/l1/error_snapshots/shell_expansion.rs
    - darkmatter/lib/tests/l1/error_snapshots/stylesheet.rs
    - darkmatter/lib/tests/l1/error_snapshots/toc_linking.rs
    - darkmatter/lib/tests/l1/error_snapshots/transclusion.rs
    - darkmatter/lib/tests/l1/expression_regression.rs
    - darkmatter/lib/tests/l1/frontmatter_surface_projection.rs
    - darkmatter/lib/tests/l1/git_context_integration.rs
    - darkmatter/lib/tests/l1/horizontal_rule_integration.rs
    - darkmatter/lib/tests/l1/horizontal_rule_snapshots.rs
    - darkmatter/lib/tests/l1/html_inversion.rs
    - darkmatter/lib/tests/l1/image_pixel_classification.rs
    - darkmatter/lib/tests/l1/inline_document_text.rs
    - darkmatter/lib/tests/l1/inline_envelope_prototype.rs
    - darkmatter/lib/tests/l1/interpolation_literal_pipeline.rs
    - darkmatter/lib/tests/l1/layout_matrix.rs
    - darkmatter/lib/tests/l1/layout_snapshots.rs
    - darkmatter/lib/tests/l1/link_interpolation_integration.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/meta_schema_phase1.rs
    - darkmatter/lib/tests/l1/meta_schema_phase3.rs
    - darkmatter/lib/tests/l1/meta_schema_phase4.rs
    - darkmatter/lib/tests/l1/meta_schema_phase5.rs
    - darkmatter/lib/tests/l1/meta_schema_phase6.rs
    - darkmatter/lib/tests/l1/meta_schema_reference_graph.rs
    - darkmatter/lib/tests/l1/meta_schema_repo_schemas.rs
    - darkmatter/lib/tests/l1/more_is_more_literals_and_indexes.rs
    - darkmatter/lib/tests/l1/predict_conflicts.rs
    - darkmatter/lib/tests/l1/prelude_exports.rs
    - darkmatter/lib/tests/l1/prose_wrap_parity.rs
    - darkmatter/lib/tests/l1/reference_integration.rs
    - darkmatter/lib/tests/l1/render_comparison.rs
    - darkmatter/lib/tests/l1/render_invariants.rs
    - darkmatter/lib/tests/l1/render_tree_hr_snapshots.rs
    - darkmatter/lib/tests/l1/render_tree_roundtrip.rs
    - darkmatter/lib/tests/l1/schema_phase_validation.rs
    - darkmatter/lib/tests/l1/schema_quoting_safety.rs
    - darkmatter/lib/tests/l1/schemas_convert_snapshots.rs
    - darkmatter/lib/tests/l1/schemas_detect_table.rs
    - darkmatter/lib/tests/l1/schemas_grammar_proptest.rs
    - darkmatter/lib/tests/l1/schemas_literal_expression.rs
    - darkmatter/lib/tests/l1/schemas_required_count_matrix.rs
    - darkmatter/lib/tests/l1/schemas_source_projection.rs
    - darkmatter/lib/tests/l1/schemas_validate_table.rs
    - darkmatter/lib/tests/l1/set_overlay_integration.rs
    - darkmatter/lib/tests/l1/shell_block_integration.rs
    - darkmatter/lib/tests/l1/shell_expansion_coordinates.rs
    - darkmatter/lib/tests/l1/span_compat.rs
    - darkmatter/lib/tests/l1/style_features_baseline.rs
    - darkmatter/lib/tests/l1/style_features_phase5.rs
    - darkmatter/lib/tests/l1/style_frontmatter.rs
    - darkmatter/lib/tests/l1/style_frontmatter_parity.rs
    - darkmatter/lib/tests/l1/suggest_constraint_phase1.rs
    - darkmatter/lib/tests/l1/suggest_constraint_phase2.rs
    - darkmatter/lib/tests/l1/suggest_constraint_phase3.rs
    - darkmatter/lib/tests/l1/suggest_constraint_phase4.rs
    - darkmatter/lib/tests/l1/ternary_integration.rs
    - darkmatter/lib/tests/l1/test_layout.rs
    - darkmatter/lib/tests/l1/tree_features_characterization.rs
    - darkmatter/lib/tests/l1/yaml_block_parity.rs
    - darkmatter/lib/tests/level2/level2_render_tree_terminal.rs
    - darkmatter/lib/tests/level2/level2_render_tree_terminal/basic_spans.rs
    - darkmatter/lib/tests/level2/level2_render_tree_terminal/code_panel.rs
    - darkmatter/lib/tests/level2/level2_render_tree_terminal/file_links.rs
    - darkmatter/lib/tests/level2/level2_render_tree_terminal/images.rs
    - darkmatter/lib/tests/level2/level2_render_tree_terminal/layout_policy.rs
    - darkmatter/lib/tests/level2/level2_render_tree_terminal/public_entry_points.rs
    - darkmatter/lib/tests/level2/level2_render_tree_terminal/support/mod.rs
    - darkmatter/lib/tests/level2/main.rs
    - darkmatter/lib/tests/level3-browser/level3_popover.rs
    - darkmatter/lib/tests/level3-browser/main.rs
    - darkmatter/lib/tests/level3-terminal/level3_image_painting.rs
    - darkmatter/lib/tests/level3-terminal/main.rs
    - features/2026-09-21-consolidated-test-binaries/pilot/apply-move.py
    - renderable/justfile
    - tools/test-toolkit/Cargo.toml
    - tools/test-toolkit/src/lib.rs
    - tools/test-toolkit/src/test_layout.rs
    - tools/test-toolkit/src/test_layout/tests.rs
docs_updated_during_phase_4:
    - darkmatter/docs/errors/README.md
    - darkmatter/docs/rendering/popover.md
    - darkmatter/features/2026-07-15-performance-followup/benchmarks/manifest.yaml
    - darkmatter/lib/README.md
    - darkmatter/lib/tests/fixtures/mermaid/README.md
    - docs/dependencies.md
    - features/2026-09-21-consolidated-test-binaries/implementation-log.md
    - features/2026-09-21-consolidated-test-binaries/measurements.md
    - features/2026-09-21-consolidated-test-binaries/plan.md
    - features/2026-09-21-consolidated-test-binaries/spec.md
    - tools/test-toolkit/README.md
docs_created_during_phase_4:
    - features/2026-09-21-consolidated-test-binaries/darkmatter-migration.json
    - features/2026-09-21-consolidated-test-binaries/darkmatter/attribute-check.json
    - features/2026-09-21-consolidated-test-binaries/darkmatter/capture-after/
    - features/2026-09-21-consolidated-test-binaries/darkmatter/capture-before/
    - features/2026-09-21-consolidated-test-binaries/darkmatter/capture-linux-after/
    - features/2026-09-21-consolidated-test-binaries/darkmatter/capture-linux-before/
    - features/2026-09-21-consolidated-test-binaries/darkmatter/comparison-darwin-linux.json
    - features/2026-09-21-consolidated-test-binaries/darkmatter/comparison-darwin-linux.md
    - features/2026-09-21-consolidated-test-binaries/darkmatter/comparison-darwin.json
    - features/2026-09-21-consolidated-test-binaries/darkmatter/comparison-darwin.md
    - features/2026-09-21-consolidated-test-binaries/darkmatter/comparison-linux.json
    - features/2026-09-21-consolidated-test-binaries/darkmatter/comparison-linux.md
    - features/2026-09-21-consolidated-test-binaries/darkmatter/guard-scans-after.md
    - features/2026-09-21-consolidated-test-binaries/darkmatter/linux-l1-summary.txt
    - features/2026-09-21-consolidated-test-binaries/darkmatter/snapshot-check.json
    - features/2026-09-21-consolidated-test-binaries/darkmatter/snapshot-mapping.json
skills_files_updated_during_phase_4:
    - .claude/skills/darkmatter/errors.md
    - .claude/skills/darkmatter/structure.md
packages_during_phase_4:
    - darkmatter
    - test-toolkit
    - renderable
    - biscuit-file
    - darkmatter-cli
source_files_during_phase_5:
    - biscuit-terminal/lib/tests/level2_terminal_osc_wezterm.rs
    - claudine/cli/tests/level2/level2_invalid_file_reference_capture.rs
    - darkmatter/cli/Cargo.toml
    - darkmatter/cli/tests/clean.rs
    - darkmatter/cli/tests/clean_frontmatter.rs
    - darkmatter/cli/tests/clean_json.rs
    - darkmatter/cli/tests/clean_schema.rs
    - darkmatter/cli/tests/code_block.rs
    - darkmatter/cli/tests/compose_array_rendering.rs
    - darkmatter/cli/tests/compose_base_schema.rs
    - darkmatter/cli/tests/compose_basic.rs
    - darkmatter/cli/tests/compose_interpolation.rs
    - darkmatter/cli/tests/compose_layout.rs
    - darkmatter/cli/tests/compose_page_blocks.rs
    - darkmatter/cli/tests/compose_perf.rs
    - darkmatter/cli/tests/compose_refs_and_missing.rs
    - darkmatter/cli/tests/compose_remote_caching.rs
    - darkmatter/cli/tests/compose_schema.rs
    - darkmatter/cli/tests/compose_schema_file_rewrite.rs
    - darkmatter/cli/tests/compose_shell.rs
    - darkmatter/cli/tests/compose_state_set.rs
    - darkmatter/cli/tests/compose_terminal_detection.rs
    - darkmatter/cli/tests/compose_transclusion.rs
    - darkmatter/cli/tests/delta.rs
    - darkmatter/cli/tests/get_set_rm.rs
    - darkmatter/cli/tests/graph.rs
    - darkmatter/cli/tests/hash.rs
    - darkmatter/cli/tests/hash_directory.rs
    - darkmatter/cli/tests/hash_kind_save_diff.rs
    - darkmatter/cli/tests/help.rs
    - darkmatter/cli/tests/l1/clean.rs
    - darkmatter/cli/tests/l1/clean_frontmatter.rs
    - darkmatter/cli/tests/l1/clean_json.rs
    - darkmatter/cli/tests/l1/clean_schema.rs
    - darkmatter/cli/tests/l1/code_block.rs
    - darkmatter/cli/tests/l1/compose_array_rendering.rs
    - darkmatter/cli/tests/l1/compose_base_schema.rs
    - darkmatter/cli/tests/l1/compose_basic.rs
    - darkmatter/cli/tests/l1/compose_interpolation.rs
    - darkmatter/cli/tests/l1/compose_layout.rs
    - darkmatter/cli/tests/l1/compose_page_blocks.rs
    - darkmatter/cli/tests/l1/compose_perf.rs
    - darkmatter/cli/tests/l1/compose_refs_and_missing.rs
    - darkmatter/cli/tests/l1/compose_remote_caching.rs
    - darkmatter/cli/tests/l1/compose_schema.rs
    - darkmatter/cli/tests/l1/compose_schema_file_rewrite.rs
    - darkmatter/cli/tests/l1/compose_shell.rs
    - darkmatter/cli/tests/l1/compose_state_set.rs
    - darkmatter/cli/tests/l1/compose_terminal_detection.rs
    - darkmatter/cli/tests/l1/compose_transclusion.rs
    - darkmatter/cli/tests/l1/delta.rs
    - darkmatter/cli/tests/l1/get_set_rm.rs
    - darkmatter/cli/tests/l1/graph.rs
    - darkmatter/cli/tests/l1/hash.rs
    - darkmatter/cli/tests/l1/hash_directory.rs
    - darkmatter/cli/tests/l1/hash_kind_save_diff.rs
    - darkmatter/cli/tests/l1/help.rs
    - darkmatter/cli/tests/l1/layout_alignment.rs
    - darkmatter/cli/tests/l1/layout_fill.rs
    - darkmatter/cli/tests/l1/layout_flags.rs
    - darkmatter/cli/tests/l1/layout_style_frontmatter.rs
    - darkmatter/cli/tests/l1/main.rs
    - darkmatter/cli/tests/l1/md_process_fixture.rs
    - darkmatter/cli/tests/l1/render_basic.rs
    - darkmatter/cli/tests/l1/rm.rs
    - darkmatter/cli/tests/l1/schema_about.rs
    - darkmatter/cli/tests/l1/schema_detect.rs
    - darkmatter/cli/tests/l1/schema_triggers.rs
    - darkmatter/cli/tests/l1/schema_validate.rs
    - darkmatter/cli/tests/l1/schema_validate_baseline.rs
    - darkmatter/cli/tests/l1/spawn_site_guard.rs
    - darkmatter/cli/tests/l1/test_layout.rs
    - darkmatter/cli/tests/l1/toc.rs
    - darkmatter/cli/tests/l1/validate_refs.rs
    - darkmatter/cli/tests/layout_alignment.rs
    - darkmatter/cli/tests/layout_fill.rs
    - darkmatter/cli/tests/layout_flags.rs
    - darkmatter/cli/tests/layout_style_frontmatter.rs
    - darkmatter/cli/tests/level2/harness_integrity.rs
    - darkmatter/cli/tests/level2/level2_code_block_styling.rs
    - darkmatter/cli/tests/level2/level2_disclosure_blocks.rs
    - darkmatter/cli/tests/level2/level2_errors.rs
    - darkmatter/cli/tests/level2/level2_frontmatter_images.rs
    - darkmatter/cli/tests/level2/level2_frontmatter_tables.rs
    - darkmatter/cli/tests/level2/level2_horizontal_rules.rs
    - darkmatter/cli/tests/level2/level2_layout_dimensions.rs
    - darkmatter/cli/tests/level2/level2_ordered_lists.rs
    - darkmatter/cli/tests/level2/level2_schema_about.rs
    - darkmatter/cli/tests/level2/level2_schema_validate.rs
    - darkmatter/cli/tests/level2/main.rs
    - darkmatter/cli/tests/level2_code_block_styling.rs
    - darkmatter/cli/tests/level2_disclosure_blocks.rs
    - darkmatter/cli/tests/level2_errors.rs
    - darkmatter/cli/tests/level2_frontmatter_images.rs
    - darkmatter/cli/tests/level2_frontmatter_tables.rs
    - darkmatter/cli/tests/level2_harness_integrity.rs
    - darkmatter/cli/tests/level2_horizontal_rules.rs
    - darkmatter/cli/tests/level2_layout_dimensions.rs
    - darkmatter/cli/tests/level2_ordered_lists.rs
    - darkmatter/cli/tests/level2_schema_about.rs
    - darkmatter/cli/tests/level2_schema_validate.rs
    - darkmatter/cli/tests/md_process_fixture.rs
    - darkmatter/cli/tests/render_basic.rs
    - darkmatter/cli/tests/rm.rs
    - darkmatter/cli/tests/schema_about.rs
    - darkmatter/cli/tests/schema_detect.rs
    - darkmatter/cli/tests/schema_triggers.rs
    - darkmatter/cli/tests/schema_validate.rs
    - darkmatter/cli/tests/schema_validate_baseline.rs
    - darkmatter/cli/tests/spawn_site_guard.rs
    - darkmatter/cli/tests/toc.rs
    - darkmatter/cli/tests/validate_refs.rs
    - darkmatter/justfile
    - darkmatter/lib/tests/l1/clean_counters.rs
docs_updated_during_phase_5:
    - darkmatter/README.md
    - features/2026-09-21-consolidated-test-binaries/implementation-log.md
    - features/2026-09-21-consolidated-test-binaries/measurements.md
    - features/2026-09-21-consolidated-test-binaries/plan.md
    - features/2026-09-21-consolidated-test-binaries/spec.md
docs_created_during_phase_5:
    - features/2026-09-21-consolidated-test-binaries/darkmatter-cli-migration.json
    - features/2026-09-21-consolidated-test-binaries/darkmatter-cli/attribute-check.json
    - features/2026-09-21-consolidated-test-binaries/darkmatter-cli/capture-after/
    - features/2026-09-21-consolidated-test-binaries/darkmatter-cli/capture-before/
    - features/2026-09-21-consolidated-test-binaries/darkmatter-cli/capture-linux-after/
    - features/2026-09-21-consolidated-test-binaries/darkmatter-cli/capture-linux-before/
    - features/2026-09-21-consolidated-test-binaries/darkmatter-cli/comparison-darwin-linux.json
    - features/2026-09-21-consolidated-test-binaries/darkmatter-cli/comparison-darwin-linux.md
    - features/2026-09-21-consolidated-test-binaries/darkmatter-cli/comparison-darwin.json
    - features/2026-09-21-consolidated-test-binaries/darkmatter-cli/comparison-darwin.md
    - features/2026-09-21-consolidated-test-binaries/darkmatter-cli/comparison-linux.json
    - features/2026-09-21-consolidated-test-binaries/darkmatter-cli/comparison-linux.md
    - features/2026-09-21-consolidated-test-binaries/darkmatter-cli/guard-scans-after.md
    - features/2026-09-21-consolidated-test-binaries/darkmatter-cli/linux-l1-summary.txt
    - features/2026-09-21-consolidated-test-binaries/darkmatter-cli/snapshot-check.json
    - features/2026-09-21-consolidated-test-binaries/darkmatter-cli/snapshot-mapping.json
skills_files_updated_during_phase_5:
    - .claude/skills/darkmatter/SKILL.md
source_files_during_phase_6:
    - biscuit-terminal/lib/Cargo.toml
    - biscuit-terminal/lib/examples/discovery_probe.rs
    - biscuit-terminal/lib/src/discovery/osc_queries/query.rs
    - biscuit-terminal/lib/tests/compose_parity.rs
    - biscuit-terminal/lib/tests/filesystem_parity.rs
    - biscuit-terminal/lib/tests/graph_expression_parity.rs
    - biscuit-terminal/lib/tests/horizontal_rule_parity.rs
    - biscuit-terminal/lib/tests/html_page_example.rs
    - biscuit-terminal/lib/tests/inline_content_matrix.rs
    - biscuit-terminal/lib/tests/integration.rs
    - biscuit-terminal/lib/tests/l1/compose_parity.rs
    - biscuit-terminal/lib/tests/l1/filesystem_parity.rs
    - biscuit-terminal/lib/tests/l1/graph_expression_parity.rs
    - biscuit-terminal/lib/tests/l1/horizontal_rule_parity.rs
    - biscuit-terminal/lib/tests/l1/html_page_example.rs
    - biscuit-terminal/lib/tests/l1/inline_content_matrix.rs
    - biscuit-terminal/lib/tests/l1/integration.rs
    - biscuit-terminal/lib/tests/l1/layout_matrix.rs
    - biscuit-terminal/lib/tests/l1/level1_apple_terminal_prose.rs
    - biscuit-terminal/lib/tests/l1/level1_clipboard.rs
    - biscuit-terminal/lib/tests/l1/level1_cursor.rs
    - biscuit-terminal/lib/tests/l1/level1_mode_2027.rs
    - biscuit-terminal/lib/tests/l1/level1_osc_queries.rs
    - biscuit-terminal/lib/tests/l1/level1_terminal_init.rs
    - biscuit-terminal/lib/tests/l1/level1_terminal_osc_cache.rs
    - biscuit-terminal/lib/tests/l1/list_parity.rs
    - biscuit-terminal/lib/tests/l1/main.rs
    - biscuit-terminal/lib/tests/l1/mermaid_parity.rs
    - biscuit-terminal/lib/tests/l1/metrics_tree_parity.rs
    - biscuit-terminal/lib/tests/l1/ordered_list_parity.rs
    - biscuit-terminal/lib/tests/l1/parity_helpers.rs
    - biscuit-terminal/lib/tests/l1/perf_gate.rs
    - biscuit-terminal/lib/tests/l1/prelude_exports.rs
    - biscuit-terminal/lib/tests/l1/progress_parity.rs
    - biscuit-terminal/lib/tests/l1/prose_cells_parity.rs
    - biscuit-terminal/lib/tests/l1/render_comparison.rs
    - biscuit-terminal/lib/tests/l1/render_tree_code_context.rs
    - biscuit-terminal/lib/tests/l1/render_tree_component_parity.rs
    - biscuit-terminal/lib/tests/l1/section_parity.rs
    - biscuit-terminal/lib/tests/l1/status_block_parity.rs
    - biscuit-terminal/lib/tests/l1/status_parity.rs
    - biscuit-terminal/lib/tests/l1/table_parity.rs
    - biscuit-terminal/lib/tests/l1/terminal_image_parity.rs
    - biscuit-terminal/lib/tests/l1/test_layout.rs
    - biscuit-terminal/lib/tests/l1/text_block_parity.rs
    - biscuit-terminal/lib/tests/l1/todo_parity.rs
    - biscuit-terminal/lib/tests/l1/tree_layout.rs
    - biscuit-terminal/lib/tests/l1/two_column_parity.rs
    - biscuit-terminal/lib/tests/l1/unordered_list_parity.rs
    - biscuit-terminal/lib/tests/layout_matrix.rs
    - biscuit-terminal/lib/tests/level1_apple_terminal_prose.rs
    - biscuit-terminal/lib/tests/level1_clipboard.rs
    - biscuit-terminal/lib/tests/level1_cursor.rs
    - biscuit-terminal/lib/tests/level1_mode_2027.rs
    - biscuit-terminal/lib/tests/level1_osc_queries.rs
    - biscuit-terminal/lib/tests/level1_terminal_init.rs
    - biscuit-terminal/lib/tests/level1_terminal_osc_cache.rs
    - biscuit-terminal/lib/tests/level2/level2_terminal_osc_wezterm.rs
    - biscuit-terminal/lib/tests/level2/main.rs
    - biscuit-terminal/lib/tests/level2_terminal_osc_wezterm.rs
    - biscuit-terminal/lib/tests/list_parity.rs
    - biscuit-terminal/lib/tests/mermaid_parity.rs
    - biscuit-terminal/lib/tests/metrics_tree_parity.rs
    - biscuit-terminal/lib/tests/ordered_list_parity.rs
    - biscuit-terminal/lib/tests/parity_helpers.rs
    - biscuit-terminal/lib/tests/perf_gate.rs
    - biscuit-terminal/lib/tests/prelude_exports.rs
    - biscuit-terminal/lib/tests/progress_parity.rs
    - biscuit-terminal/lib/tests/prose_cells_parity.rs
    - biscuit-terminal/lib/tests/render_comparison.rs
    - biscuit-terminal/lib/tests/render_tree_code_context.rs
    - biscuit-terminal/lib/tests/render_tree_component_parity.rs
    - biscuit-terminal/lib/tests/section_parity.rs
    - biscuit-terminal/lib/tests/status_block_parity.rs
    - biscuit-terminal/lib/tests/status_parity.rs
    - biscuit-terminal/lib/tests/table_parity.rs
    - biscuit-terminal/lib/tests/terminal_image_parity.rs
    - biscuit-terminal/lib/tests/text_block_parity.rs
    - biscuit-terminal/lib/tests/todo_parity.rs
    - biscuit-terminal/lib/tests/tree_layout.rs
    - biscuit-terminal/lib/tests/two_column_parity.rs
    - biscuit-terminal/lib/tests/unordered_list_parity.rs
    - claudine/cli/tests/common/pty.rs
    - renderable/justfile
    - "biscuit-terminal/lib/tests/snapshots/*.snap → biscuit-terminal/lib/tests/l1/snapshots/l1__*.snap (1,086 byte-identical moves; see biscuit-terminal/snapshot-mapping.json)"
docs_updated_during_phase_6:
    - biscuit-terminal/README.md
    - biscuit-terminal/docs/dependencies.md
    - features/2026-09-21-consolidated-test-binaries/implementation-log.md
    - features/2026-09-21-consolidated-test-binaries/measurements.md
    - features/2026-09-21-consolidated-test-binaries/plan.md
    - features/2026-09-21-consolidated-test-binaries/spec.md
docs_created_during_phase_6:
    - features/2026-09-21-consolidated-test-binaries/biscuit-terminal-migration.json
    - features/2026-09-21-consolidated-test-binaries/biscuit-terminal/attribute-check.json
    - features/2026-09-21-consolidated-test-binaries/biscuit-terminal/capture-after/
    - features/2026-09-21-consolidated-test-binaries/biscuit-terminal/capture-before/
    - features/2026-09-21-consolidated-test-binaries/biscuit-terminal/capture-linux-after/
    - features/2026-09-21-consolidated-test-binaries/biscuit-terminal/capture-linux-before/
    - features/2026-09-21-consolidated-test-binaries/biscuit-terminal/comparison-darwin-linux.json
    - features/2026-09-21-consolidated-test-binaries/biscuit-terminal/comparison-darwin-linux.md
    - features/2026-09-21-consolidated-test-binaries/biscuit-terminal/comparison-darwin.json
    - features/2026-09-21-consolidated-test-binaries/biscuit-terminal/comparison-darwin.md
    - features/2026-09-21-consolidated-test-binaries/biscuit-terminal/comparison-linux.json
    - features/2026-09-21-consolidated-test-binaries/biscuit-terminal/comparison-linux.md
    - features/2026-09-21-consolidated-test-binaries/biscuit-terminal/guard-scans-after.md
    - features/2026-09-21-consolidated-test-binaries/biscuit-terminal/linux-l1-summary.txt
    - features/2026-09-21-consolidated-test-binaries/biscuit-terminal/snapshot-check.json
    - features/2026-09-21-consolidated-test-binaries/biscuit-terminal/snapshot-mapping.json
skills_files_updated_during_phase_6:
    - .claude/skills/biscuit-terminal/SKILL.md
    - .claude/skills/renderable/tree.md
packages_during_phase_6:
    - biscuit-terminal
    - renderable
    - claudine-cli
source_files_during_phase_7:
    - claudine/cli/tests/l1/spawn_site_guard.rs
    - claudine/cli/tests/l1/dispatch_inventory.rs
    - claudine/cli/tests/l1/error_guards/transport-allow.toml
    - claudine/cli/tests/level2/level2_lifecycle_control.rs
    - claudine/docs/providers/dispatch-inventory.json
    - biscuit-terminal/cli/tests/level2_style_everywhere_matrix.rs
    - features/2026-09-21-consolidated-test-binaries/baseline/consumer-sweep.py
docs_updated_during_phase_7:
    - biscuit-terminal/README.md
    - docs/testing-strategy.md
    - prompts/_prompt.md
    - claudine/features/2026-09-08-steering/verification/README.md
    - darkmatter/features/2026-07-15-performance-followup/benchmarks/README.md
    - features/2026-09-21-consolidated-test-binaries/baseline/test-selector-consumers.md
    - features/2026-09-21-consolidated-test-binaries/plan.md
    - features/2026-09-21-consolidated-test-binaries/implementation-log.md
    - features/2026-09-21-consolidated-test-binaries/spec.md
docs_created_during_phase_7:
    - features/2026-09-21-consolidated-test-binaries/baseline/test-selector-consumers-after.md
skills_files_updated_during_phase_7:
    - .claude/skills/rust-testing/SKILL.md
    - .claude/skills/rust-testing/integration-tests.md
    - .claude/skills/rust-testing/cli-output-testing.md
    - .claude/skills/rust-testing/nextest.md
    - .claude/skills/claudine/completions/shell-completions.md
    - .claude/skills/cli/cliclick.md
packages_during_phase_7:
    - claudine-cli
    - biscuit-terminal-cli
    - biscuit-terminal
---

# Implementation Plan — Consolidated Test Binaries

## Summary of Work and Definition of Success

### What this change actually is

One sentence: **each migrated package replaces one-executable-per-`tests/*.rs`-file
with one test binary per compatible execution contract — tier × exact
`required-features` × harness mode × irreducible target-wide settings — declared
explicitly under `autotests = false`, with every existing test kept in the same
tier, behind the same Cargo features and OS conditions, with the same
assertions.**

The work decomposes into five tracks:

1. **Tooling track (Phase 2).** A checked, local migration toolkit — inventory
   generator, migration manifest, before/after Nextest identity comparator,
   crate-attribute checker, and snapshot-mapping checker — that reads `cargo
   metadata` **and** each package manifest, shells out to `just _tier_filter`
   for every tier expression (never re-derives them), and compares exact
   normalized test identities, never counts.
2. **Pilot track (Phase 3).** Migrate `claudine-cli` (139 integration-test
   targets → expected 4: L1, Level 2, Level 3, real-provider) and run the full
   measurement series — clean build time, warm edit-to-one-test latency (five
   alternating trials), peak compiler memory, target count, on-disk size —
   under the matched conditions in spec §8, then take the one-contract vs
   subject-group seam decision against the stated guardrail.
3. **Rollout track (Phases 4–6).** Migrate `darkmatter` (74 targets; keeps
   `browser-tests` and `terminal-tests` Level 3 contracts in **separate**
   binaries), then `darkmatter-cli` (53), then `biscuit-terminal` (38) — one
   package at a time, each with its own recreated inventory and verification
   evidence, per spec §Sequencing.
4. **Workflow and documentation track (Phase 7).** Update the `rust-testing`
   skill (consolidated layout, positional name filtering, feature boundary,
   Nextest process-isolation dependency, `cargo test` non-equivalence) and
   sweep every **active** recipe, doc, and skill away from `--test
   <old-binary>` selectors for migrated targets; historical records stay
   untouched.
5. **Closeout track (Phase 8).** Record archive size, file count, packing
   time, and Cargo build time from the next **ordinary** CI runs that select
   each migrated package (no run is triggered solely for this), and walk all
   twelve acceptance criteria into an acceptance document.

Grounding against the current tree:

| Package | Crate directory | Test files | Cargo targets | Expected consolidated targets |
|---|---|---:|---:|---|
| `claudine-cli` | `claudine/cli` | 139 | 139 | 4 (`l1`, `level2`, `level3`, `real`) |
| `darkmatter` | `darkmatter/lib` | 73 top-level + `error_snapshots/main.rs` | 74 | ~5 (L1; L2×`terminal-tests`; L3×`terminal-tests`; L3×`browser-tests`; browser×`browser-tests`) — inventory is authoritative |
| `darkmatter-cli` | `darkmatter/cli` | 53 | 53 | 2–4 — inventory is authoritative |
| `biscuit-terminal` | `biscuit-terminal/lib` | 38 | 38 | 2 (L1; L2×`terminal-tests`) — inventory is authoritative |

131 of `claudine-cli`'s 139 test files declare `mod common;` today (187 across
the four package trees), so the shared-`common` consolidation is where most of
the compile-time win and most of the mechanical edit risk both live.

### Non-negotiable constraints (from the spec)

- **Structural edits only** (spec §3): module-ization attribute moves, helper
  import rewiring, path repairs, and crate-namespace collision fixes. Test
  bodies, inputs, assertions, timeouts, skip decisions, tier markers, and
  feature requirements do not change. A test needing behavior change to
  survive consolidation stops that package's migration and files a separate
  defect.
- **Canonical tier expressions come only from `just _tier_filter`**
  (`just/devops.just`). No implementation or verification script re-creates
  them. OS `cfg` conditions stay module conditions; they never create a new
  binary.
- **Identity comparison is exact and four-way** — selected, excluded-as-other-
  tier, ignored, and platform-absent — per package, per supported feature set,
  per platform. Counts are never sufficient.
- **One package at a time**, pilot first; the pilot lands **before** work on
  `2026-09-21-ci-build-feature-divergence` begins (spec §Sequencing).
- **No new CI run, matrix cell, or gate** for this feature (spec §Out of
  scope). Discovery runs locally and on the standing build hosts
  (`just cross-check`); CI observations come from the next ordinary run.
- **Snapshots move mechanically** with contents preserved, suites run under
  `INSTA_UPDATE=no`, and no `.snap.new` file may exist. A missed mapping
  fails; regeneration to go green is forbidden.
- **Storage keys do not change.** Completion records and artifacts stay keyed
  by `{package, environment, tier}`. The skip baseline
  (`.github/ci/ci-baseline.toml`) is confirmed empty before each package
  migration; if non-empty, identities are translated via the migration
  manifest, never deleted or broadened.
- **Benchmarks, examples, and real custom-harness tests do not move.**
  Darkmatter's 16 `harness = false` entries are `[[bench]]` targets and stay.
- **Terminal and browser verification never takes host focus** (repo testing
  rules for L2/L3 suites).

### What success looks like

Complete when all twelve acceptance criteria of the spec hold and are
demonstrated by named, re-runnable artifacts — not by inspection:

- **Explicit targets** — each migrated package has `autotests = false`, a
  reviewed `[[test]]` list, a committed migration manifest mapping every old
  target to exactly one consolidated target and module (or recorded neutral
  alias), and a structural guard inside its consolidated Level 1 binary that
  rejects undeclared test crate roots and modules.
- **Identity preserved** — for every package, feature set, and platform, the
  four-way before/after identity comparison is identical; the comparison
  artifact (tool output) is committed under this feature directory.
- **Platform proof** — macOS, Linux, and native-Windows compilation and
  listings pass per package (`just cross-check <package>`); WSL2 archive
  portability is recorded when ordinary evidence exists and reported
  `pending` otherwise.
- **Snapshots and guards** — every tracked snapshot has a checked old→new
  mapping or a proof of unaffectedness; the Claudine/Darkmatter-CLI
  spawn-site guards and the repository archive-path guard
  (`scripts/ci-build-archive.rs` suite) report the same intended file sets
  before and after, listed by path.
- **Local suites green** — `just test`, `just test-l2`, `just lint` pass in
  every migrated area; `just test-browser` passes for `darkmatter`;
  `just check-tier-coverage` passes for every migrated area; Level 3 and
  real-provider tests keep their opt-in.
- **Pilot measured** — `measurements.md` in this directory records the
  before/after series and the seam decision under the spec §8 guardrail
  (split only if median edit latency rises by both >50% **and** >5 s, or the
  target cannot compile reliably within runner memory).
- **Docs honest** — no active recipe, doc, or skill recommends `--test
  <old-binary>` for migrated targets; the `rust-testing` skill documents the
  new contract including the Nextest process-isolation dependency.
- **CI observed** — archive size, file count, packing time, and Cargo time
  recorded from ordinary runs, reported as observations.

## Phase 1 — Rulings, Spikes, and Baselines

No production file changes in this phase except the specification's `status`
moving to `planned` once rulings are recorded. Everything else produces
committed evidence under `features/2026-09-21-consolidated-test-binaries/`
(`rulings.md`, `spikes/`, `baseline/`).

### Necessary Rules

These rulings must be decided (with the author where judgment is involved)
before Phase 2 begins. Each carries a recommended default so implementation is
never blocked; a ruling task closes by writing its decision and consequences
into `rulings.md`.

- [x] **R1 — Toolkit home and language.** Where does the migration toolkit
      live: Python beside the existing CI scripts (`scripts/ci/`) or a Rust
      binary under `scripts/`?
      *Recommended:* Python — `scripts/ci/consolidation.py` with subcommands
      (`inventory`, `plan`, `capture`, `compare`, `check-attributes`,
      `check-snapshots`) and a pytest suite `scripts/ci/test_consolidation.py`,
      matching the `affected_scope.py`/`completion.py` house pattern. It runs
      only locally and on build hosts; no CI workflow consumes it, so the
      `check`-cell and lint surface of `scripts/ci` applies unchanged.
- [x] **R2 — Target names, directory layout, and neutral module aliases.**
      Confirm binary names (`l1`, `level2`, `level3`, `browser`, `real`),
      directory layout (`tests/<tier>/main.rs` + sibling modules,
      `tests/common/mod.rs` per package or per-tier `common`), the rule that a
      tier with multiple feature contracts gets feature-suffixed names
      (darkmatter: `level3-terminal` vs `level3-browser`), and the neutral
      alias mechanism: a former target name becomes the module name **unless**
      that segment would newly match or stop matching a
      `(^|::)<marker>` tier predicate — then a neutral alias is chosen and
      recorded in the migration manifest.
      *Recommended:* adopt exactly that; the alias comparison is mechanical
      (apply each `_tier_filter` expression to the projected new path for
      every test in the target) and the comparator re-proves it afterward.
- [x] **R3 — "Target-wide settings" criterion.** What qualifies as a
      target-wide setting that cannot safely move to a module (and therefore
      forces a separate binary)?
      *Recommended:* any per-`[[test]]` manifest key Cargo honors other than
      `name`/`path` — `harness`, `required-features`, `edition` — plus any
      discovered crate-root-only construct. Expected inventory outcome: none
      beyond `harness = false` (which is benches-only today and out of scope);
      the inventory must prove this, not assume it.
- [x] **R4 — `slow_` Level 1 tests.** Confirm `slow_` tests stay inside the
      L1 consolidated binary in every package (selection is filter-based, not
      feature-based), including `claudine-cli`'s `wrap_sigint::slow_*` case
      and darkmatter's `l1-include-slow = true` policy. For darkmatter, the
      before/after listing capture runs under both
      `BISCUIT_L1_INCLUDE_SLOW` states.
- [x] **R5 — Static identity consumers.** Confirm the only *static* test-
      identity store is the CI skip baseline (`.github/ci/ci-baseline.toml`):
      expected manifests and completion records are derived per run by
      `_expected_manifest` from the live tree, and JUnit artifacts are
      historical. Therefore no translation layer is built; the per-package
      checkpoint is "confirm the baseline is still empty (or translate each
      entry via the manifest)". No historical JUnit artifact is rewritten.
- [x] **R6 — Measurement protocol.** Fix the matched conditions and record
      them in `measurements.md`: pinned toolchain from
      `rust-toolchain.toml`, host triple, cargo profile, feature set, linker,
      worker count, `RUSTC_WRAPPER` unset (kache off) and fresh separate
      `--target-dir`s for clean builds, otherwise-idle macOS host; peak RSS
      via `/usr/bin/time -l`; Sniff host/load snapshot once per series
      (`sniff runtime`); five alternating before/after edit-one-test trials
      after warm-up, reporting median and slowest; the §8 two-part latency
      guardrail and its subject-group fallback.
- [x] **R7 — Sequencing and status.** Spec `status` moves
      `draft-spec → planned` in the same commit as this plan; the
      `claudine-cli` pilot lands before any implementation of
      `2026-09-21-ci-build-feature-divergence`; each package migration is its
      own reviewable commit series (inventory + manifest, structural move,
      verification evidence, docs); the agent never moves the spec to
      `_completed` (author-only, after review closes).
- [x] **R8 — Parallel-rollout boundary.** Confirm "one package at a time"
      means the *migration and its review* are sequential, while **baseline
      capture for the next package may be prepared during the previous
      package's remote-verification wait** (listings are read-only evidence
      against an untouched tree). No two packages are ever mid-migration in
      the same working tree simultaneously.

### Wave 1 — Rulings and ruling-independent evidence (parallel)

- [x] **Record rulings** — work R1–R8 to written decisions in `rulings.md`
      (lead/author task; blocks only the Wave 2 items that depend on R1, R2,
      R6). While open, everything else in this wave proceeds.
- [x] **Spike S1 — Nextest identity fidelity** (`spikes/s1-nextest-identity.md`).
      On a scratch crate (no CI, no package change), prove with
      `nextest list --message-format json`: the exact matched path shape for a
      test inside a nested module vs a top-level module; that `test(...)`
      never matches the binary id; how `#[ignore]`d, filter-excluded, and
      `cfg`-removed tests are reported; how two former binaries sharing a test
      name collides once consolidated (and that nextest disambiguates only by
      binary, which the migration manifest must therefore forbid or alias);
      and listing from an archive with `--workspace-remap`. Reuse the
      fixtures and learnings of
      `features/2026-09-19-direct-cell-execution/spikes/s2-nextest-list.md`
      rather than building a new harness. Output: the exact JSON shape
      `consolidation.py capture/compare` will parse, plus confirmation (or
      correction) of the neutral-alias mechanics in R2.
- [x] **Spike S2 — Crate-global construct scan** (`spikes/s2-crate-globals.md`).
      Scan all four package trees for crate-root-only constructs: inner
      attributes (`#![...]`), `#[macro_export]`, `#[global_allocator]`,
      `no_main`/`no_std`, `#[link_section]`/`#[used]`/`#[ctor]`-style startup,
      and duplicated crate-root symbol names that would collide in a shared
      crate namespace. Record every hit with file, construct, and disposition
      (safe-to-modularize vs forces-separate-target vs blocks-migration).
      This is the §3 pre-migration scan; its method becomes the
      `check-attributes` subcommand's detector list.
- [x] **Baseline inventory** (`baseline/inventory.json` + human-readable
      summary). Generate, from `cargo metadata --no-deps` **and** parsing each
      of the four `Cargo.toml`s, the per-package table of every current test
      target: name, source path, `required-features`, harness mode, and
      tier-by-marker. Neither source alone is sufficient (metadata hides some
      manifest settings); the merged table is the future migration manifest's
      `before` side. Include the 47-package / 523-target workspace context row
      for the record.
- [x] **Active `--test` consumer sweep** (`baseline/test-selector-consumers.md`).
      Grep every justfile, `*.md` doc, `just.md`, README, and
      `.claude/skills/` + `.opencode/skill/` file for `--test <name>` usage
      against the four packages' targets. Classify each hit **active**
      (current recipe/doc/skill guidance — must be updated in Phase 7 or the
      package phase) vs **historical** (completed specs, review logs, dated
      records — never rewritten). This inventory is Phase 7's worklist.
- [x] **Guard before-scans** (`baseline/guard-scans.md`). Run and record the
      exact scanned-file lists produced today by: `claudine/cli/tests/
      spawn_site_guard.rs`, `darkmatter/cli/tests/spawn_site_guard.rs`, and
      the repository archive-path guard suite
      (`scripts/ci-build-archive-tests.rs`, the archive-relocation fixtures).
      After-state comparisons in later phases must reproduce these lists
      exactly; "passing while scanning zero files" is explicitly not
      evidence.

### Wave 2 — Ruling-dependent spikes and before-evidence (parallel; needs R1/R2/R6 closed)

- [x] **Spike S3 — Measurement harness dry-run** (`spikes/s3-measurement.md`).
      Script and validate the R6 protocol against **current** `claudine-cli`
      (no migration yet): clean-build timing in a fresh target dir, warm
      edit-one-test loop (touch one L1 test, run it through the canonical
      recipe), peak-RSS capture, and disk-size accounting of produced test
      executables. The successful dry-run's numbers become the pilot's
      recorded **before** series in `measurements.md`. Also record the
      command list so the **after** series is byte-for-byte the same recipe.
- [x] **Spike S4 — Snapshot mapping probe** (`spikes/s4-snapshots.md`). For
      the pilot's proposed layout (R2), enumerate which Insta snapshots move:
      those under `claudine/cli/tests/**` whose assertion directory or
      `module_path!()`-derived filename changes when a file becomes a module
      under `tests/l1/` etc. Derive the mechanical old→new mapping rule,
      verify on one moved pair that contents survive and the suite passes
      with `INSTA_UPDATE=no`, and estimate the per-package mapping-table
      shape the `check-snapshots` subcommand must validate.
- [x] **Before-listings capture** (`baseline/listings/`). Using the exact
      commands S1 validated, capture per package × supported feature set the
      `nextest list --message-format json` evidence on macOS under every
      canonical tier expression (via `just _tier_filter <tier> [<pkg>]`),
      including darkmatter under both `BISCUIT_L1_INCLUDE_SLOW` states.
      These files are the immutable `before` side of every later identity
      comparison.
- [x] **Disk and CI-context baseline** (`baseline/disk-and-ci.md`). Record
      the current on-disk size/count of each package's built test executables
      on this host (the direct `claudine-cli` observation the spec cites,
      re-measured), the skip-baseline emptiness snapshot of
      `.github/ci/ci-baseline.toml`, and pointers to the pull-request-92
      producer-job numbers the spec quotes (they stay labeled
      single-observation, not baseline).

### Checkpoint

- [x] `rulings.md` records R1–R8 with consequences; no ruling is left at
      "recommended" without a decision.
- [x] S1–S4 have written results; the baseline inventory, consumer sweep,
      guard scans, before-listings, and disk/CI context are committed under
      this feature directory.
- [x] The only production-file change in this phase is the spec's `status`.
- [x] Phase 2's tool design is re-read against S1/S2/S4 findings and adjusted
      where a spike contradicted an assumption.

## Phase 2 — Migration Toolkit and Oracles

Build the Phase 1-ruled toolkit as production code with tests. Nothing here
touches the four packages; the toolkit proves itself against the **unmigrated**
tree (a no-op comparison must be identical) before the pilot uses it.

**Adjustments from Phase 1 evidence** (`rulings.md`, `spikes/`). These
supersede the task text below where they differ:

- `capture` also lists under **every `filter` in `.config/nextest.toml`
  overrides** (read verbatim), and `compare` requires each override to select
  the same normalized identities (R5). Two in-scope exact-name overrides
  (`compose_loop_rate_limit_pause_waits_then_continues`,
  `every_catalog_variable_survives_ambient_options`) would otherwise stop
  matching silently.
- `plan` projects module names against the `_tier_filter` expressions **and**
  those override filters (some are unanchored regexes). The alias form strips
  the marker prefix (R2). The projection must reproduce the one alias known
  today: darkmatter-cli `level2_harness_integrity` → `harness_integrity`.
- `compare`'s platform-absent set is the union of the macOS, Linux, and
  Windows captures minus the host's own. It needs the on-host captures
  (S1 §3). A single-host run reports that set as `not-evaluated`, never empty.
- A required-features target that is not enabled is **absent** from
  `rust-suites` (S1 §4). `capture` records the feature set, and `compare`
  never reads an absent suite as zero tests.
- `inventory` treats `tests/*/main.rs` as an auto-discovered root. Darkmatter's
  `error_snapshots` is auto-discovered, not declared. `perf_` is a marker
  only for `worktree-cli`.
- `check-attributes` uses S2's detector list, including the identity-sensitive
  detectors (`--exact <path>` strings, `current_exe()` + `--list`). It finds
  inner attributes anywhere in the leading doc/attribute block, not only on
  line 1.
- `check-snapshots` derives expected paths from S4's rule, not from a
  hand-written table. It must state explicitly when a mapped snapshot is read
  by no running test (biscuit-terminal's 1,086 `layout_matrix__*` belong to
  an `#[ignore]`d test).
- The Phase 1 prototypes (`baseline/capture-listings.sh`,
  `baseline/inventory.py`, `baseline/consumer-sweep.py`, `spikes/s2-scan.py`)
  are folded in. The no-op self-proof reproduces `baseline/inventory.json`
  and the listing SHA-256s (`baseline/listings/SHA256SUMS`) on the
  unmigrated tree, excluding the locality fields S1 names.

### Wave 1 — Independent modules (parallel)

- [x] **Inventory generator** — `consolidation.py inventory`: merges
      `cargo metadata --no-deps` with manifest parsing (per R3's key list)
      into the checked per-package table; fails on a target present in one
      source and absent from the other, on an undeclared nested crate root,
      and on any `[[test]]` whose source path does not exist.
- [x] **Contract planner** — `consolidation.py plan`: groups the inventory by
      execution-contract key (tier × required-features × harness ×
      target-wide settings), assigns consolidated target names and module
      names per R2, applies the neutral-alias rule by projecting every test's
      new path against every `_tier_filter` expression (shelling out to
      `just`; never embedding the expressions), and emits the migration
      manifest JSON (old target → consolidated target, module name or
      recorded alias, per-test projected path).
- [x] **Attribute checker** — `consolidation.py check-attributes`: compares
      each moved file's former file-level crate attributes against its new
      module declaration attributes (`#[cfg(unix)] mod x;` etc.), using S2's
      detector list; reports any crate-global construct that survived
      modularization.
- [x] **Snapshot mapper** — `consolidation.py check-snapshots`: validates a
      committed old→new snapshot mapping table — every tracked snapshot
      affected by the move is mapped, mapped files are byte-identical after
      the move, no `.snap.new` exists, and unmoved snapshots are proven
      unaffected (path hash unchanged).

### Wave 2 — Comparator and self-proof (needs Wave 1's manifest format)

- [x] **Identity comparator** — `consolidation.py capture` + `compare`:
      capture runs `nextest list --message-format json` per tier expression
      (again via `just _tier_filter`) and feature set; `compare` loads
      before/after captures plus the migration manifest, normalizes only the
      intentional binary→module identity change, and diffs the four sets —
      selected, excluded-as-other-tier, ignored, platform-absent — by exact
      identity. Any difference, any unmapped test, or any count-only output
      fails with a named, actionable error.
- [x] **Toolkit tests** — `scripts/ci/test_consolidation.py`: unit fixtures
      for each subcommand (synthetic metadata/manifest/listing JSON), plus
      the failure oracle for every guard above (missing mapping, drifted
      attribute, identity loss masked by identity gain, silent count match).
      Wire into the existing Python suite runner for `scripts/ci`.
- [x] **No-op self-proof** — run `capture` twice against the current
      (unmigrated) tree for all four packages and `compare` the captures:
      identical, all tests mapped trivially. This is the toolkit's own
      acceptance gate before any source file moves.

### Checkpoint

- [x] `just lint` and the `scripts/ci` Python suites pass; every failure
      oracle demonstrated red-then-green in the suite.
- [x] The no-op comparison artifact for all four packages is committed under
      this feature directory as the toolkit's self-proof.
- [ ] Reviewer sign-off that the manifest format is the single authoritative
      mapping — filenames were never authoritative, and now nothing else is
      either.

## Phase 3 — `claudine-cli` Pilot Migration

The pilot exercises every hazard (features, platform `cfg`, shared `common`,
path guards, Level 2, Level 3, real-provider, snapshots) and carries the full
measurement series and the seam decision. Expected result: 139 targets → 4
(`l1`, `level2` ×`terminal-tests`, `level3` ×`terminal-tests`, `real`
×`real-tests`); the migration manifest is authoritative if it says otherwise.

### Wave 1 — Structural move (sequential; one working tree)

- [x] **Execute migration** — generate the pilot manifest
      (`plan` output, reviewed and committed as
      `claudine-cli-migration.json` under this feature directory); create
      `claudine/cli/tests/{l1,level2,level3,real}/main.rs` and shared
      `common`; set `autotests = false` and declare the four `[[test]]`
      entries with explicit `path` and exact `required-features`; move every
      file per the manifest; apply only the §3 structural edits (module-ize,
      `#[cfg]`/attribute moves to module declarations, `crate::common`
      imports replacing 131 `mod common;` declarations, `mod`/`#[path]`/
      `include_*`/fixture/snapshot path repairs, crate-namespace collision
      fixes). Any test that needs a behavior change halts the pilot and
      becomes a separate defect record.
- [x] **Compile and list on macOS** — `cargo nextest list` succeeds for the
      package under each feature set; `check-attributes` reports zero
      unrepresented crate-level conditions; S2's crate-global hits are all
      dispositioned.

### Wave 2 — Pilot fix-ups (parallel once macOS compiles)

- [x] **Snapshot moves** — apply the S4-derived mapping mechanically; run the
      affected suites with `INSTA_UPDATE=no`; `check-snapshots` green; zero
      `.snap.new` files in the tree.
- [x] **Identity comparison** — run `capture` after-state and `compare`
      against `baseline/listings/` for every feature set: four-way identity
      equality on macOS. Any diff is a manifest or move defect, never a
      filter change.
- [x] **Test-placement guard** — extend `claudine/cli/tests/test_placement.rs`
      so it rejects an unexpected test crate root or undeclared module under
      the new layout (acceptance 12), keeping it inside the consolidated L1
      binary — no new test target, no CI gate.
- [x] **Area recipe/doc pass** — update anything **active** in
      `claudine/justfile`, `claudine/just.md`, and area docs that names the
      old per-file targets or recommends `--test <old-binary>`; narrow-test
      guidance switches to Nextest positional filtering with the tier's
      canonical `-E` expression retained (spec §6). Historical records stay
      untouched.

### Wave 3 — Cross-host verification and measurements (parallel across hosts)

- [x] **macOS full validation** — `just test claudine`, `just test-l2`,
      `just lint` for the area; L3 terminal and real-provider suites run per
      their opt-in and the no-focus rule; `just check-tier-coverage claudine`
      and `just check-canonical claudine` pass.
- [ ] **Linux and native Windows** — `just cross-check claudine-cli --os
      linux` and `--os windows`: compile, per-feature-set listings, and the
      identity comparison re-run **on each host** (a macOS source scan cannot
      prove the Windows module graph); Windows `cfg` module coverage
      confirmed from the host's listing, not from inspection.
- [x] **WSL2 archive portability** — when the standing WSL2 host is
      reachable, run the archive-path check per the `os` skill; otherwise
      record `pending` with the exact missing evidence (acceptance 4 allows
      pending, never a silent pass).
- [x] **After measurement series** — on the idle macOS host, run the exact
      S3 command list against the migrated tree: clean-build time, five
      alternating edit-one-test trials (median + slowest), peak compiler
      RSS, produced target count, on-disk test executable size. Append to
      `measurements.md` beside the before series with the Sniff host/load
      snapshots for both series.
- [x] **Skip-baseline check** — confirm `.github/ci/ci-baseline.toml` is
      still empty (or translate entries via the pilot manifest per R5).

### Wave 4 — Seam decision and pilot close

- [x] **Guardrail decision** — apply the §8 thresholds to the measurement
      series. Default: keep one binary per contract. Only if median edit
      latency rises by both >50% and >5 s, or the consolidated target cannot
      compile reliably / exceeds runner memory: split into **stable subject
      groups** (same tier and feature contract, named product areas — never
      numbered shards), re-run Waves 1–3 for the split, and record why.
- [x] **Pilot report** — `pilot.md` under this feature directory: manifest
      summary, before/after identity artifacts, per-host results, measurement
      table, seam decision, defects filed (if any). This is the review packet
      for Phases 4–6 proceeding.

### Checkpoint

- [ ] All Wave 3 evidence committed; four-way identity equality on macOS,
      Linux, and native Windows; WSL2 recorded as passed or pending-with-
      reason.
- [x] `measurements.md` complete with matched conditions stated; seam
      decision recorded against the guardrail.
- [x] `claudine-cli` shows 4 declared test targets, `autotests = false`, no
      undeclared crate roots (guard test proves it), and the area suites
      named above are green.

## Phase 4 — `darkmatter` Migration

Recreate inventory and evidence for this package; do not assume the pilot's
module or feature shapes apply (spec §Sequencing). Distinct hazards: the
dual Level 3 feature contracts, the declared nested `error_snapshots` target,
the `effects-instrumentation` feature dimension, the `l1-include-slow`
policy, and the heaviest snapshot population in the four packages. Benches
(`harness = false`) do not move.

### Wave 1 — Structural move (sequential)

- [x] **Darkmatter manifest and move** — fresh `inventory` + `plan` for
      `darkmatter/lib` (committed as `darkmatter-migration.json`); the
      browser-backed Level 3 tests (`browser_render`, `level3_popover`) and
      terminal-backed Level 3 test (`level3_image_painting`) land in
      **separate** consolidated targets with their exact `required-features`
      (acceptance 3); `error_snapshots/main.rs`'s nested crate joins its
      contract's target per the manifest (flattened to modules, or — if its
      contract genuinely differs — recorded as a deliberate remaining
      target); `autotests = false`; structural edits only per §3.

### Wave 2 — Fix-ups (parallel)

- [x] **Snapshot moves** — the S4 rule applied to darkmatter's snapshot
      directories (`tests/snapshots`, `error_snapshots` payloads); mechanical
      moves, `INSTA_UPDATE=no`, `check-snapshots` green, zero `.snap.new`.
- [x] **Identity comparison** — `compare` against `baseline/listings/` for
      every darkmatter feature set **and** both `BISCUIT_L1_INCLUDE_SLOW`
      states (R4), on macOS.
- [x] **Structural guard** — add the undeclared-crate-root/module check
      inside darkmatter's consolidated Level 1 target (acceptance 12; not a
      new binary, not a CI gate).
- [x] **Area recipe/doc pass** — active `darkmatter/justfile`, `just.md`,
      and docs updated off per-file `--test` selectors; browser-suite
      guidance keeps its opt-in and no-focus rule.

### Wave 3 — Cross-host verification (parallel)

- [x] **macOS suites** — `just test darkmatter`, `just test-l2`,
      `just lint`, `just test-browser` (no focus); `just check-tier-coverage
      darkmatter` passes.
- [ ] **Linux / Windows listings + comparison on host**; **WSL2** archive
      portability when available, else `pending` with reason.
- [x] **Lightweight observations** — target count and on-disk executable
      size before/after on this host recorded into `measurements.md`'s
      rollout table (no full measurement series; that was pilot-only).
- [x] **Skip-baseline check** per R5.

### Checkpoint

- [ ] Identity equality four-way on macOS/Linux/Windows for every feature set
      and both slow-policy states; browser and terminal Level 3 contracts
      still separate targets; `error_snapshots` disposition recorded.
- [x] Area suites green; guard test live; snapshot mapping table committed.

## Phase 5 — `darkmatter-cli` Migration

Same shape as Phase 4. Distinct hazards: the spawn-site guard
(`darkmatter/cli/tests/spawn_site_guard.rs`) and its file-set invariants, and
the `terminal-tests` feature across Level 2.

### Wave 1 — Structural move (sequential)

- [x] **darkmatter-cli manifest and move** — fresh inventory/plan (committed
      as `darkmatter-cli-migration.json`); expected `l1` + `level2`
      (×`terminal-tests`) with the inventory authoritative; `autotests =
      false`; structural edits only.

### Wave 2 — Fix-ups (parallel)

- [x] **Identity comparison** on macOS across feature sets; snapshot moves if
      the inventory finds any; structural guard added inside the consolidated
      L1 target; active recipe/doc pass for `darkmatter/cli` area docs.
- [x] **Spawn-site guard** — re-run `darkmatter/cli/tests/spawn_site_guard.rs`
      (now inside its consolidated target) and diff its scanned-file list
      against `baseline/guard-scans.md`: identical intended coverage, by
      path, not by count.

### Wave 3 — Cross-host verification (parallel)

- [ ] **macOS suites** (`just test darkmatter-cli` equivalents via the area's
      canonical recipes, `test-l2`, `lint`, `check-tier-coverage`); Linux /
      Windows on-host listings + comparison; WSL2 when available else
      `pending`; rollout observations appended; skip-baseline check per R5.
      *Phase 5 status:* everything here is done except the native-Windows
      on-host listings. The `build-win-native` `W:` volume has 37.9 GiB free,
      below the 50 GiB preflight. Windows has cross-compile evidence only, and
      WSL2 is `pending` (see the log's Phase 5 section).

### Checkpoint

- [ ] Identity equality on three OSes; spawn-site guard file list identical
      to baseline; area suites green; guard test live.
      *Phase 5 status:* identity is identical on macOS and Linux, and on both
      together with platform-absent evaluated. The guard file lists match by
      path. Area suites are green apart from pre-existing failures, and the
      layout gate is live. Native Windows is pending (disk space).

## Phase 6 — `biscuit-terminal` Migration

Same shape as Phase 4/5. Distinct hazards: smallest package (38), the
`level2_terminal_osc_wezterm` WezTerm-backed contract, and benches
(`rendering`, `render_tree`) staying untouched.

### Wave 1 — Structural move (sequential)

- [x] **biscuit-terminal manifest and move** — fresh inventory/plan
      (committed as `biscuit-terminal-migration.json`); expected `l1` +
      `level2` (×`terminal-tests`); `autotests = false`; structural edits
      only; benches unchanged.

### Wave 2 — Fix-ups (parallel)

- [x] **Identity comparison** on macOS; snapshot moves if any; structural
      guard added inside the consolidated L1 target; active recipe/doc pass
      for the `biscuit-terminal` area.

### Wave 3 — Cross-host verification (parallel)

- [ ] **macOS suites** (L1, L2 WezTerm backend per the no-focus rule, lint,
      `check-tier-coverage`); Linux / Windows on-host listings +
      comparison; WSL2 when available else `pending`; rollout observations
      appended; skip-baseline check per R5.
      *Phase 6 status:* everything here is done except the native-Windows
      on-host listings. `just cross-check biscuit-terminal --os windows`
      refused at storage preflight: `W:` has 36 GiB free, under the 50 GiB
      floor. Windows has cross-compile evidence only (`x86_64-pc-windows-gnu`,
      all features and none, clippy `-D warnings` clean), and WSL2 is
      `pending` (see the log's Phase 6 section).

### Checkpoint

- [ ] Identity equality on three OSes; area suites green; guard test live;
      all four packages now migrated and the four migration manifests plus
      evidence sets are complete and committed.
      *Phase 6 status:* identity is identical on macOS, on Linux, and on both
      together with platform-absent evaluated. Area suites are green, the
      layout gate is live, and all four manifests and evidence sets are in
      this directory. Native Windows and WSL2 are pending (disk space), now
      for all four packages.

## Phase 7 — Developer Workflow and Documentation Sweep

Cross-cutting updates that must reflect all four migrations. The Phase 1
consumer sweep (`baseline/test-selector-consumers.md`) is the worklist; only
its **active** entries change.

### Wave 1 — Documentation surfaces (parallel)

- [x] **`rust-testing` skill update** — document the consolidated layout
      (`tests/<tier>/` + `common`), positional test-name filtering with the
      tier's canonical `-E` expression retained, the feature boundary
      (consolidation never unions features), and both sides of the
      process-isolation contract: Nextest runs each case in its own process
      so consolidation changes nothing under canonical recipes, while
      `cargo test` runs cases from one binary in one process and must never
      be presented as an equivalent way to run a migrated suite
      (acceptance 11, spec §7).
- [x] **Active-doc sweep** — update every active entry from the consumer
      inventory (root and area `just.md`s, READMEs, skill files that
      recommend `--test <old-binary>`) to the positional-filter pattern;
      re-verify each remaining grep hit is a historical record and annotate
      the inventory file saying so.
- [x] **Drift-maintenance pass** — per AGENTS.md: area READMEs where public
      behavior changed, `.claude/skills/` where architecture or workflows
      changed (beyond `rust-testing`: `rust-devops` and `os` skills only if
      they describe per-file test binaries or archive composition), and this
      feature's `spec.md` notes if any ruling changed a documented behavior.

### Checkpoint

- [x] A grep for `--test ` across justfiles, active docs, and skills returns
      zero references to migrated targets; the consumer inventory records
      every remaining hit as historical with its record date.
- [x] The `rust-testing` skill renders correctly (darkmatter `md` render) and
      a reviewer confirms the process-isolation statement is present and
      two-sided.

## Phase 8 — CI Observations, Acceptance, and Closeout

No CI run is triggered solely for this feature (spec §Out of scope); this
phase harvests the next ordinary runs and closes the acceptance record.

### Wave 1 — Evidence harvest (parallel as runs become available)

- [ ] **Producer observations** — from the first ordinary producer job that
      selects each migrated package after its migration, record archive
      size, archive file count, packing time, and Cargo build time into
      `ci-observations.md`, each labeled as single observations with run
      URLs (acceptance 10). Compare against the pull-request-92 numbers only
      as context, never as pass/fail.
- [ ] **WSL2 closure** — re-check any Phase 3–6 `pending` WSL2 items against
      ordinary-run or standing-host evidence now available; anything still
      unproven is reported pending in the acceptance document, not silently
      passed.

### Wave 2 — Acceptance and final validation (sequential)

- [ ] **Acceptance document** — `acceptance.md` walking all twelve spec
      acceptance criteria, each linked to its committed evidence artifact
      (manifests, identity comparisons, guard scans, measurement and
      observation files, skill diff).
- [ ] **Final validation sweep** — `just test claudine darkmatter
      biscuit-terminal`, `just lint` for the four areas, `just
      check-canonical`, `just check-tier-coverage` for migrated areas, and a
      workspace `cargo metadata` check that the four packages report exactly
      their declared consolidated test targets.
- [ ] **Status transition** — spec `status` → `implemented` with
      `implemented_by` recorded; the working tree is left at
      "implementation complete, ready for review" — moving the spec to
      `_completed` is the author's action after the review cycle closes, per
      AGENTS.md.

### Checkpoint

- [ ] All twelve acceptance criteria linked to evidence; any pending item
      (WSL2) explicitly named with what evidence is missing.
- [ ] Final sweep green; spec status updated; handoff note written for the
      reviewer.
