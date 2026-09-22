---
spec: /Volumes/coding/wt/rusty-biscuit/feat-dark-fixes/features/2026-09-21-consolidated-test-binaries/spec.md
plan: features/2026-09-21-consolidated-test-binaries/plan.md
implemented_by: claude/opus
started_phase: "1"
packages:
    - repo-deps
    - claudine-cli
    - claudine
    - darkmatter
    - test-toolkit
    - renderable
    - biscuit-file
    - darkmatter-cli
    - biscuit-terminal
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
packages_during_phase_5:
    - darkmatter-cli
    - darkmatter
    - claudine-cli
    - biscuit-terminal
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
---

# Implementation Log for 2026-09-21-consolidated-test-binaries (8 phases)

## Phase 1

Started and completed 2026-09-21 on macOS (`aarch64-apple-darwin`, rustc
1.98.1, cargo-nextest 0.9.136) at revision `c0f911f5a`.

Phase 1 is evidence only. The one production-file change is the spec
`status: draft-spec → planned` (R7). No package source, manifest, recipe,
config, or skill changed. The three guard files that got temporary probes
were restored with `git checkout --` and verified clean.

### What was produced

| Plan task | Artifact |
|---|---|
| R1–R8 (+ new R9) rulings | `rulings.md` |
| Spike S1 — Nextest identity | `spikes/s1-nextest-identity.md` (scratch crate in system temp) |
| Spike S2 — crate-global scan | `spikes/s2-crate-globals.md`, `spikes/s2-scan.py`, `spikes/s2-scan-output.txt` |
| Spike S3 — measurement dry-run | `spikes/s3-measurement.md`, `spikes/s3-measure.sh`, `spikes/s3/` |
| Spike S4 — snapshot mapping | `spikes/s4-snapshots.md` (scratch crate, insta 1.47.2) |
| Baseline inventory | `baseline/inventory.json` + `.md`, generated by `baseline/inventory.py` |
| `--test` consumer sweep | `baseline/test-selector-consumers.md`, generated by `baseline/consumer-sweep.py` |
| Guard before-scans | `baseline/guard-scans.md` |
| Before-listings | `baseline/listings/*.json.gz` (114) + `SHA256SUMS`, from `baseline/capture-listings.sh` |
| Disk and CI context | `baseline/disk-and-ci.md`, `baseline/deps-census.py` |
| Measurement protocol + before series | `measurements.md` |
| Phase 2 design re-read | "Adjustments from Phase 1 evidence" block added at the top of plan Phase 2 |

### Findings that changed the plan's assumptions

1. **Static identity consumers (R5 amended).** `.config/nextest.toml` holds
   exact-name overrides for two in-scope tests. After the move they stop
   matching silently, and the tests lose their extended `slow-timeout`.
   `claudine/docs/providers/dispatch-inventory.json` persists a
   `--test dispatch_inventory` selector that its test compares against.
2. **One neutral alias is required (R2).** darkmatter-cli
   `level2_harness_integrity` is gated on `terminal-tests`, but its 4 tests are
   L1-tier: CI's darkmatter-cli L1 count is 578 with the feature and 574
   without. As a module named `level2_harness_integrity` they would move to
   L2. The first inventory pass also flagged biscuit-terminal `perf_gate`. That
   was a false positive, because `perf_` is a marker only for
   `worktree-cli`, and the fix is in `inventory.py`.
3. **Identity-sensitive runtime code (R9).** Darkmatter's L2 support re-runs
   its own binary with `--exact public_entry_points::level2_render_probe_entrypoint`.
   `interpolation_literal_pipeline.rs` embeds `current_exe() --list` output,
   whose size grows from one file's tests to the whole L1 binary.
4. **Module-file resolution** (measured on the scratch crate). Bare `mod x;`
   in a non-root module file resolves under `<file-stem>/`. `#[path]` and
   `include_str!` resolve relative to the containing file's directory.
   `#[path = "../common/mod.rs"]` keeps `common`'s children resolving in
   place. An inner `#![cfg]` still works in a module. A crate-root-only
   attribute degrades to a warning.
5. **`slow_` premise (R4).** The four packages' integration targets hold zero
   `slow_` tests. The single difference between darkmatter's two slow-policy
   listings is a **lib unit test**, which is a built-in control.
6. **Spec facts.** `error_snapshots/main.rs` is auto-discovered, not declared.
   The claudine `mod common;` count is 129, not 131; the total of 187 is
   right. The spec's 51 packages / 523 targets are confirmed (74 workspace
   members in all).
7. **Pre-existing, preserved as-is (separate-defect candidates):**
   darkmatter `schema_phase_validation::real_shipped_inline_schema_uses_normal_resolution_and_phase_path`
   is `real_`-prefixed in a package with no real tier, so no recipe runs it.
   biscuit-terminal's 1,086 `layout_matrix__*` snapshots belong to an
   `#[ignore]`d test.

### Measurements (S3 before series, loaded host)

Clean build: 280.1 s wall, 3.63 GB peak RSS, 136 test targets, 2.78 GB of
executables (the spec observed 2.67 GB / 136). Edit-to-one-test: median
10.2 s, slowest 15.0 s. The host was **not idle** (1-minute load 10.8 at the
start, 57.1 at the end, from other sessions). Phase 3 must re-measure the
before side alternating with the after side, and needs an edit-only mode in
`s3-measure.sh` to do so (`measurements.md`).

### Requirement-to-evidence mapping

Phase 1 changes no behavior, so there is no regression test to add. Every
phase requirement maps to a re-runnable artifact instead:

| Requirement | Evidence (re-runnable) | Verified how |
|---|---|---|
| Exact Nextest identity shape, including nested modules, ignored, filter-excluded, cfg-absent, feature-gated-absent, and archive remap | S1 scratch crate listings | observed outputs recorded in `s1-nextest-identity.md` |
| Crate-global scan with dispositions | `spikes/s2-scan.py` → `s2-scan-output.txt` | every hit class dispositioned; `fn main` / `#[link]` hits read manually and confirmed to be string fixtures or imports |
| Measurement protocol works | `spikes/s3-measure.sh` | ran end to end; `--no-tests=fail` proves the edit filter selected the test every trial; target count matches the independent 136 |
| Snapshot rule and missed-mapping failure | S4 scratch crate | negative case (unmapped → both tests fail, no `.snap.new`) and positive case (byte-identical move passes under `INSTA_UPDATE=no` and `CI=true`) both observed; `INSTA_REQUIRE_FULL_MATCH` failure recorded and shown unused in the repository |
| Before-listings immutable and complete | `baseline/listings/` + `SHA256SUMS` | 114/114 non-empty; each decompressed file re-hashed and diffed against `SHA256SUMS` (clean) |
| Inventory merges metadata + manifest + listings | `baseline/inventory.py` | regenerated twice, byte-identical; workspace totals match the spec (51 / 523) |
| Consumer sweep complete and classified | `baseline/consumer-sweep.py` | regenerated, byte-identical; 30 active / 8 in-flight / 546 historical |
| Guard before-populations by path | `baseline/guard-scans.md` | probe counts equal each guard's own `governed_files` census (103/88, 42/40); all three suites passed in the same run |

Gates run: `shellcheck` on both shell scripts (clean) and `python3 -m
py_compile` on all four Python scripts (clean). `just test` / `just lint`
were **not run**. This phase changed no Rust source, manifest, or recipe in
any package area, so there is nothing for them to verify. Phase 2 builds
the toolkit with its own pytest suite under `scripts/ci`.

### Skipped or deferred

- **Cross-OS listings:** macOS only. The plan puts Linux/Windows captures in
  Phase 3 Wave 3 (per package, on-host), and platform-absent sets need them
  (S1). Three claudine-cli targets are cfg-absent on macOS. Their alias
  status is provisional, from a source scan.
- **Real-pair snapshot move:** S4 verified the mechanism on a scratch crate,
  because moving the real `claudine-cli` snapshot would be a production
  change. Phase 3 verifies the real pair.

### Unrelated working-tree changes observed (not made by this phase)

While this phase ran, `just/devops.just` (the `status` / `_status_recent`
recipes) and spec files under `tree-hugger/`, `unchained-ai/`, and
`worktree/` were modified by another process (mtime 16:59). They do not touch
`_tier_filter`, so the listings are unaffected. They were left untouched.

## Phase 2

Completed 2026-09-21 on macOS (`aarch64-apple-darwin`, cargo-nextest 0.9.136)
at revision `da6e3847d`. No file in the four package trees changed. This
phase builds and proves the toolkit only.

### Resumed work

An earlier run of this phase wrote `scripts/ci/consolidation.py` (all six
subcommands), `scripts/ci/test_consolidation.py`, `selfproof/mutation-check.py`,
and the first self-proof capture (`selfproof/capture-a/`). It ticked the four
Wave 1 tasks and "Toolkit tests", then stopped before logging anything. This
run re-verified that work instead of trusting the ticks:

- **Toolkit tests**: 62 of 62 pass. `mutation-check.py` reports 8 of 8 oracles
  red on their mutation and green on the original.
- **"Wire into the existing Python suite runner" was not done**, although the
  task was ticked. It is done now (next section).
- Only the second capture, the comparisons, and the records were missing.

### Suite wiring (the one behavior change outside the toolkit)

`test_consolidation.py` now runs in the `just ci-local` ci-infra self-test
loop (`just/ci-local.just`). The two `test_ci_local.py` fixtures that stub
every suite in that loop gained the new name, so the fixtures still model
the recipe exactly.

It is deliberately **not** in `affected_scope.py`'s `SUITE_REGISTRY`. That
registry schedules a CI companion cell on `ubuntu-latest`, and R1 rules that
no workflow runs this toolkit (spec §Out of scope). The comment in the recipe
says so, so a later "every `test_*.py` is a companion" sweep does not add it by
reflex. It costs about 4 s per `ci-local` run. The shipped-artifact tests read
frozen Phase 1 evidence (`baseline/inventory.json`, `baseline/listings/`), so
they do not drift as packages migrate.

Note: R1 and the plan say "pytest suite", but the suite is stdlib `unittest`
like every other `scripts/ci/test_*.py`. It runs as
`python3 scripts/ci/test_consolidation.py`. R1's intent, the house pattern,
holds.

### Self-proof (`selfproof/README.md`)

| Proof | Result |
|---|---|
| `capture` twice, then `compare --require-identical-digests` over all four packages | **identical**: 0 failures and 0 notes across 18 feature-set captures and 404 selector cells, every raw-listing digest equal (`selfproof/noop-comparison.{json,md}`) |
| Capture determinism | The two runs are byte-identical, so the second is kept only as `capture-b.SHA256SUMS` |
| Reproduces the Phase 1 listings | `compare --before baseline/listings --after capture-a --common-selectors --require-identical-digests`: 114 of 114 digests equal (locality fields excluded), zero identity differences |
| Reproduces the Phase 1 inventory | `inventory --listings baseline/listings` equals `baseline/inventory.json` except `generator` |

Platform-absent is `not-evaluated` in every cell. The captures are
single-host, and the tool refuses to report that set as empty.

### Requirement-to-test mapping

All in `scripts/ci/test_consolidation.py` unless noted. "Mutation" marks a
test that `selfproof/mutation-check.py` proves red against a broken tool.

| Requirement | Test(s) |
|---|---|
| Tier expressions come only from `_tier_filter`; every shipped filter parses | `ShippedFilterCorpusTests.test_every_tier_and_override_filter_parses` (passive corpus over the live `_tier_filter` and `.config/nextest.toml`), `…test_include_slow_is_captured_only_where_the_manifest_declares_it` |
| Filterset semantics (anchoring, binary is never the test path, precedence, matchers, refusal of unsupported predicates) | `FilterEvaluatorTests.*` (5) |
| Evaluator must agree with every Nextest verdict before planning | `PlanTests.test_the_evaluator_must_agree_with_every_nextest_verdict` (mutation), `…test_a_capture_taken_under_another_filter_is_refused` |
| Inventory merges metadata and manifest; fails on one-sided targets, undeclared targets, unbuilt nested roots | `InventoryTests.*` (5) |
| Contract grouping, R2 names, neutral alias, collisions, reserved names, R3 target-wide keys | `PlanTests.test_contracts_become_named_targets`, `…marker_name_that_would_move_tests_gets_a_neutral_alias` (mutation), `…alias_that_collides…`, `…reserved_names_custom_harness_and_target_wide_keys_are_refused`, `…unmarked_name_matched_by_an_unanchored_override_is_refused`, `…a_target_no_capture_lists_falls_back_to_a_source_scan` (cfg-absent targets) |
| Exact-name override → rewrite, not alias (R5) | `PlanTests.test_an_exact_name_override_is_a_rewrite_not_an_alias` |
| End to end over the real shipped artifacts | `ShippedArtifactPlanTests.test_darkmatter_cli_needs_exactly_the_one_known_alias` (reproduces `level2_harness_integrity` → `harness_integrity`), `…test_the_ruled_target_sets_for_every_package` |
| Capture folding, locality-free digest, same-build check, package-skipped binaries, empty listing ≠ zero tests, gzip round trip | `CaptureDocumentTests.*` (6; skipped-binary one is a mutation) |
| Four-way compare by exact identity, never counts | `CompareTests.test_identity_loss_masked_by_identity_gain_fails_with_the_identities` (mutation; silent count match), `…tier_move_by_module_name_fails`, `…ignore_flip_fails`, `…absent_consolidated_suite_is_lost_identities_not_zero` |
| Unmapped test / duplicate normalization fail | `CompareTests.test_an_unmapped_module_and_a_duplicate_normalization_fail` (mutation) |
| Platform-absent needs other hosts; single host is `not-evaluated` | `CompareTests.test_platform_absent_needs_other_hosts` (mutation) |
| Missing captures and one-sided selectors, tier-filter change, override rewrite noted, additions, digest requirement, CLI exit codes 0/1/2 | `CompareTests.*` (remaining 6) |
| No-op comparison identical | `CompareTests.test_an_unmigrated_tree_compares_identical_to_itself` plus the real-tree self-proof above |
| Inner attributes anywhere in the leading block; declarations with attributes | `SourceScanTests.*` (2) |
| Attribute checker: cfg moved to the declaration, cfg missing, dropped lint, crate-root-only, missing declaration, crate-global and identity detectors, `crate::`, path review | `CheckAttributesTests.*` (8; the cfg-missing one is a mutation) |
| Snapshot mapper: rule-derived paths, byte-identical move, missing row, disagreeing row, changed bytes / left-behind / `.snap.new`, unmoved changed, Insta path settings, unattributable, `#[ignore]`d readers stated per file, most-specific attribution | `CheckSnapshotsTests.*` (10; the missing-row one is a mutation) |
| Suite wiring is modeled exactly by the recipe fixtures | `scripts/ci/test_ci_local.py` (100 of 100 pass) |

No persisted value is written and read back beyond the capture documents.
Their round trip is `CaptureDocumentTests.test_round_trip_and_raw_listing_directories`,
and on the real tree the byte-identical recapture shows it.

### Gates run

- `python3 scripts/ci/test_consolidation.py`: 62 passed.
- `python3 selfproof/mutation-check.py`: 8 of 8 OK.
- `python3 scripts/ci/test_ci_local.py`: 100 passed (82 s).
- `just _lint repo-deps` (the owning package's lint; `scripts/` belongs to the
  `root` area, whose `just lint` fans out across the whole monorepo): exit 0.
- `just --list` parses. `py_compile` is clean on all changed Python.
- **Not run:** the root-area `just lint`/`just test` fan-out across every
  area. Only Python and a `just` recipe changed, and no Rust source.
- **Cross-OS:** not run in this phase. The toolkit is stdlib Python with
  `pathlib` / POSIX-normalized paths, and Phase 3 exercises it on-host on
  Linux and Windows (plan Phase 3 Wave 3). Capture's `host` is
  `platform.system().lower()` (`darwin`/`linux`/`windows`), which matches the
  baseline's file naming.

### Open for review

- Plan Phase 2 checkpoint "Reviewer sign-off that the manifest format is the
  single authoritative mapping" is left unticked. It is a human reviewer's
  call, and it is raised in the spec's `human_review_items`.

### Unrelated working-tree changes (not made by this phase)

`just/devops.just` (`_status_recent`) and
`darkmatter/features/2026-07-22-explicit-null/spec.md` (deleted) were already
modified when this run started. They were left untouched. Neither affects
`_tier_filter`.

## Phase 3

Worked 2026-09-21/22 on macOS (`aarch64-apple-darwin`, rustc 1.98.1,
cargo-nextest 0.9.136) against base revision `9621882ae`. `claudine/cli` was
unchanged since the Phase 2 capture, so `selfproof/capture-a` is a valid
before side (R8). The review packet is `pilot.md`, and this section records
how it was produced.

### What was done

| Plan task | Result | Artifact |
|---|---|---|
| Execute migration | 139 targets → 4 (`l1` 102 modules, `level2` 29, `level3` 5, `real` 3). `autotests = false`, 4 explicit `[[test]]`. No alias needed. | `claudine-cli-migration.json`, `pilot/apply-move.py` |
| Compile and list on macOS | all 5 feature sets list. `check-attributes`: 0 failures, 1 recorded disposition (a `current_exe`/`--list` false positive) | `pilot/attribute-check.json` |
| Snapshot moves | 3 moved (`tests/l1/snapshots/l1__wrap_basics__*`), 4 unaffected, `INSTA_UPDATE=no` green, 0 `.snap.new` | `pilot/snapshot-{mapping,check}.json` |
| Identity comparison (macOS) | identical, 0 failures across 5 feature sets × 22 selectors | `pilot/comparison-darwin.{json,md}` |
| Test-placement guard | layout gate added inside `l1` (`test_placement.rs`) | 4 new tests, declared as manifest `additions` |
| Area recipe/doc pass | `lint-transport` recipe, area docs, provider prompt, R9 strings, moved-path references | list below |
| macOS full validation | `just test` 7,334 passed; `just test-l2` 231+3 passed; `just lint` exit 0; tier-coverage and canonical pass | — |
| Linux | on-host before/after captures identical. L1: 2,760 passed, 3 pre-existing failures | `pilot/comparison-linux.*`, `pilot/linux-l1-summary.txt` |
| Native Windows | **not done**: cross-compile only (compile evidence) | `pilot.md` §Per-host |
| WSL2 | **pending**, recorded with the exact missing evidence | `pilot.md` §Per-host |
| After measurement series | alternating pair, kache off | `measurements.md`, `pilot/measure/` |
| Skip-baseline check | `.github/ci/ci-baseline.toml` holds only `schema_version = 3`, empty | — |
| Guardrail decision | one binary per contract (+0.30 s median edit latency) | `measurements.md` §Seam decision |
| Pilot report | done | `pilot.md` |

### Decisions made during the move

- **`mod common;` → `use crate::common;`** in each moved file, with any
  attribute or doc line above it kept. Each root declares `common` once through
  `#[path = "../common/mod.rs"]`. `use common::…` statements then resolve
  through the import, with no further edits. Three imports needed the `cfg`
  their uses already had, because an unused import warns where an unused
  `mod` (under `common`'s `#![allow(dead_code)]`) did not. That showed up on
  macOS for `wrap_ctrl_c_windows.rs`, and in the Windows cross-compile for
  `loop_initialize_state.rs` and `shipped_prompts.rs`.
- **Inner `#![cfg]` removed from the module file** and carried only on the
  declaration. `check-attributes` would also accept a duplicate, but "becomes
  an outer attribute" (spec §4) reads as a move.
- **`#[path = "common/…"] mod x;` copies** (`source_scan` in the two guards,
  `host_tools` in the perf bench) became `use crate::common::x;`. That is the
  §3 "replace repeated helper-module declarations" edit. Their comments
  explained why a per-file binary avoided `mod common`, and they were
  rewritten because they no longer applied.
- **`error_guards/` moved beside `l1/error_guards.rs`** (R2). Its `#[path]`
  became a plain `mod source_scan;`, which resolves there, and the two
  area-relative allowlist constants were repaired.
- **The layout gate** (acceptance 12) walks the Rust module graph from the
  `[[test]]` roots `Cargo.toml` declares. It requires every `.rs` under
  `tests/` (except `fixtures/`, `snapshots/`) to be reached. One rule covers a
  stray top-level file, an undeclared `tests/<x>/main.rs`, and an undeclared
  module, including one inside a helper directory. It also requires
  `autotests = false` and an explicit `path` on every `[[test]]`. A
  `cfg`-gated declaration counts as declared, and `mod x;` in prose or a
  string does not.
- **Narrow-test guidance** is `just test-cli <module>::` (run in `claudine/`).
  Verified: nextest intersects a positional filter with `-E`, so
  `-E <L1> error_guards::` lists exactly the 8 `error_guards` tests. For a
  compile-only check of a Windows target, `--test level3` names the new target
  (`signal-handling.md`), which acceptance 11 allows.
- **`dispatch-inventory.json`**: re-blessing through the new command also
  rewrote about 50 `line` fields that were already stale at `HEAD` (the compare
  ignores lines). To honor R9 ("the committed diff must be exactly the
  `regenerate` line"), the file was restored and only that line edited. The
  test passes, and fails with the old line.

### Requirement-to-test mapping

| Changed behavior | Test / check (targeted) | Red-then-green shown |
|---|---|---|
| Every old test keeps one identity, tier, ignore state, and platform presence | `consolidation.py compare` macOS, Linux, macOS+Linux | the toolkit's own oracles (Phase 2 mutation check), identical here |
| Override `test(=…)` still selects its test | `compare` selector `override-ci-5` / `override-default-0` (1 selected before and after) | Phase 2 `test_an_exact_name_override_is_a_rewrite_not_an_alias` |
| Former file-level `cfg` on the declaration | `check-attributes` | Phase 2 mutation |
| Snapshots found at the new path | `INSTA_UPDATE=no just test-cli wrap_basics::` + `check-snapshots` | yes: an unmoved name makes `help_lists_wrapper_subcommands` fail |
| Acceptance 12 layout gate | `test_placement::every_test_source_is_compiled_by_a_declared_target`, `layout_gate_rejects_stray_roots_and_undeclared_modules`, `layout_gate_requires_explicit_targets`, `module_declarations_read_attributes_visibility_and_path` | yes: planted `tests/stray_probe.rs` + `tests/l1/orphan_probe.rs` → red naming both, then green |
| Guards scan the same files | temporary probes, `pilot/guard-scans-after.md` | populations compared by path; the only additions are the 4 roots |
| Guard assertions follow the move | `spawn_site_guard::*` (20), `error_guards::*` (8), `test_placement::*` | the renamed-path assertions are positive (`contains`), so a wrong path fails |
| Persisted `regenerate` selector | `dispatch_inventory::dispatch_inventory_matches_committed_file` | yes: the old line fails it |
| `lint-transport` recipe | `just lint` (runs `just _test claudine-cli error_guards::`: 8 tests) | — |
| L3 / real stay opt-in | L3 and real filters without the opt-in env | every test printed its skip reason |
| Cross-OS module graph | Windows cross-compile of all 4 targets; Linux on-host build and listings | Windows cross-compile warnings found and fixed (2 imports) |

"Passive corpus" and "shipped artifact end-to-end" are covered by the layout
gate reading the real `Cargo.toml` and tree, and by `compare` over real
captures. No persisted value is written and read back.

### Gates run

- `cargo check --tests` and `cargo clippy --all-targets -D warnings` with
  `terminal-tests,real-tests`: clean. `_lint` itself uses no features, so it
  skips the three feature-gated targets. That was also true of their
  per-file predecessors.
- `just lint` (claudine): exit 0. `just test` (claudine): 7,334 passed,
  9 skipped. `just test-l2`: 231 + 3 passed.
- `just check-tier-coverage claudine`, `just check-canonical claudine`: pass.
- `python3 scripts/ci/test_consolidation.py`: 62 passed (unchanged toolkit).
- `shellcheck spikes/s3-measure.sh`: clean.
- GitNexus `detect-changes --scope all`: risk low, 0 affected processes.
- Not run: `just test-l3` and `just test-real`. Both take focus or bill real
  providers, and their opt-in was verified instead. Also not run: the
  root-area fan-out.

### Pre-existing failures (not caused by this phase)

- **Linux:** `sequence_groups::{parallel_body_lines_carry_their_own_tasks_bar_color,
  parallel_group_members_are_attributed_across_both_channels,
  serial_and_parallel_group_frames_share_one_left_edge}` fail on
  `build-linux` on the unmigrated base as well, identically. They pass on
  macOS. Separate-defect candidate.
- **Windows cross-compile warning** `GENERATED_MARKER` never used
  (`sequence_initialize_include_preflight.rs`): the constant is only read by
  a unix-gated test, so the old per-file crate warned too.
- Orphan snapshots and a stale `--test level2_pty_tests` doc line: `pilot.md`
  §Defects.

### Host problems met (recorded, not worked around silently)

- **kache was on in the Phase 1 measurement and in the first Phase 3
  attempts.** `unset RUSTC_WRAPPER` does not beat
  `~/.cargo/config.toml`'s `rustc-wrapper = "kache"`, and `build-linux`'s
  `/usr/local/bin/cargo` shim enables kache on any cold target dir. The first
  after "clean" build took 33.8 s, and the first Linux run hit kache's
  read-only hardlink error. Fixed with an explicitly empty
  `RUSTC_WRAPPER=""` (plus removing the kache `cc` shims locally). Both runs
  were discarded and re-done. Recorded in the `os` skill
  (`build-hosts.md` §Compiler cache) and in `measurements.md` §Correction.
- **`build-linux` cross-check lock** has been held since 2026-09-14 by another
  branch's `nightly-reward-spike`. Linux evidence came from a private
  `--shared` clone plus a bundle, per the `os` skill. The scratch clone was
  deleted afterward, and the lock was left alone.
- **`build-win-native` `W:` has 43 GB free**, under the 50 GiB preflight, and
  **`build-win` (WSL) resets SSH at key exchange**, the `os` skill's full-`W:`
  signature. No Windows build was attempted. Freeing `W:` is the owner's
  call.

### Changed outside the four target directories

`.config/nextest.toml` (R5 override), `claudine/justfile`
(`lint-transport`), `claudine/cli/Cargo.toml`,
`claudine/cli/tests/common/mod.rs` (one stale comment),
`claudine/docs/providers/dispatch-inventory.json` (one line), area docs
(`claudine/docs/topics/*` ×9, `claudine/prompts/create-new-provider.md`),
comment-only path updates in `claudine/cli/src` ×4 and
`claudine/lib/src/provider` ×2, skills (`claudine` ×7 path references,
`biscuit-test-harness` ×1, `os/build-hosts.md`), and this feature's
`spikes/s3-measure.sh` (build/edit modes, kache really off).

Comment-only and doc-only changes should go in a separate commit from the
structural move (AGENTS.md scope discipline). The source-file comment edits
are 1:1 path swaps in `//!`/`///` lines.

### Unrelated working-tree changes

None observed. The tree was clean at `9621882ae` when this phase started.

## Phase 4

Worked 2026-09-21 on macOS (`aarch64-apple-darwin`, rustc 1.98.1,
cargo-nextest 0.9.136) against base revision `048e44f7a`, with a clean tree
at start. All evidence is under `darkmatter/` in this feature directory,
plus `darkmatter-migration.json`.

### What was done

| Plan task | Result | Artifact |
|---|---|---|
| Manifest and move | 74 targets → 5: `l1` (70 modules, 0 features), `level2` (`terminal-tests`), `level3-terminal` (`terminal-tests`), `level3-browser` (`browser-tests`), `browser` (`browser-tests`). Browser and terminal Level 3 are separate targets (acceptance 3). `autotests = false`, 5 explicit `[[test]]`. No alias needed. 821 tests mapped. | `darkmatter-migration.json` |
| `error_snapshots` disposition | L1 contract (no required features), so it joins `l1`: `tests/error_snapshots/main.rs` → `tests/l1/error_snapshots/mod.rs`, with its 17 children unchanged apart from the namespace repair below. It is not a remaining target. | manifest row `nested_root: true` |
| Snapshot moves | 200 moved byte-for-byte (129 `tests/snapshots/*` → `tests/l1/snapshots/l1__*`, 71 `error_snapshots/snapshots/*` → `l1/error_snapshots/snapshots/l1__error_snapshots__*`), 1 unaffected (`src/layout/page/snapshots`). Every moved snapshot is read by a running test. `INSTA_UPDATE=no` runs green, and there are 0 `.snap.new` files. | `darkmatter/snapshot-{mapping,check}.json` |
| Identity comparison, macOS | **identical**, 0 failures: 6 feature sets × 23 selectors, including `L1` and `L1-include-slow` (R4). The one-test slow-policy control holds (6,494 vs 6,495 selected). The 12 notes are the R5 override rewrite. | `darkmatter/comparison-darwin.{json,md}` |
| Structural guard | `tests/l1/test_layout.rs` calls the new shared `test_toolkit::test_layout` | manifest `additions` (1 test) |
| Area recipe/doc pass | selectors, moved paths, `renderable` drift-report loop, stale justfile comment | list below |
| macOS suites | `just lint` exit 0. `just test` 7,947/7,948. `just test-l2` 90/90. `just test-browser` 43/43. L3 skips without `RUN_LEVEL3`. `check-tier-coverage darkmatter` red. The two reds are pre-existing (below). `check-canonical darkmatter` passes. | — |
| Linux | on-host before/after captures **identical**. L1 6,492/6,494, 2 pre-existing failures | `darkmatter/comparison-linux.*`, `darkmatter/linux-l1-summary.txt` |
| macOS + Linux platform-absent | **identical**, evaluated on both hosts | `darkmatter/comparison-darwin-linux.*` |
| Native Windows | **pending**: cross-compile of all 5 targets for `x86_64-pc-windows-gnu` is clean (0 warnings; this compiles the `#[cfg(windows)]` `declined_path_transclusion` module). There are no on-host listings, for the same reason as Phase 3. | — |
| WSL2 | **pending**: same reason | — |
| Lightweight observations | 74 → 5 targets, 5.01 → 0.51 GB (−90%), all three features | `measurements.md` §Rollout |
| Skip-baseline check | `.github/ci/ci-baseline.toml` holds only `schema_version = 3`, empty | — |
| Guard file sets | archive-path guard over `darkmatter/lib/tests/**`: 100 → 106. The only additions are the 5 roots and `l1/test_layout.rs`. Nothing is missing, and no file's eligibility changed. | `darkmatter/guard-scans-after.md` |

### Decisions made during the move

- **Fresh before-side, not `capture-a`.** `plan` refused `selfproof/capture-a`
  because Phase 3 rewrote claudine's override string in `.config/nextest.toml`.
  `darkmatter/lib` was unchanged since `capture-a` (`git log da6e3847d..HEAD
  -- darkmatter/lib` is empty), so the tree was re-captured unmigrated
  (`darkmatter/capture-before`). A `compare` against `capture-a` is identical
  (0 failures; the 12 notes are the claudine override), so the new before-side
  describes the same tree (R8).
- **Mover generalized, not copied.** `pilot/apply-move.py` now takes the package
  name from the manifest, and declares `common` only where `tests/common/mod.rs`
  exists. It moves a `nested_root` crate as a whole directory (`main.rs` →
  `mod.rs`), and takes the layout-gate module name as an argument (default
  `test_placement.rs`). Re-rendering the four claudine roots from their manifest
  reproduces the committed files byte-for-byte, so the pilot's record still holds.
- **Helpers (R2).** `image_test_support/` (used by `l1` and `level3-terminal`)
  and `layout_matrix_support/` (two `l1` modules, plus
  `examples/layout_matrix.rs`) stay at `tests/`. Each root that uses one
  declares it once by `#[path]`. `level3-terminal`'s declaration carries the
  same `#[cfg(target_os = "macos")]` as its only user, because the old crate
  was empty off macOS. The files' own declarations became `use crate::…;`.
  `level2_render_tree_terminal/` moved beside its file under `level2/`, so its
  seven `#[path = "level2_render_tree_terminal/…"]` lines became plain `mod x;`.
- **Namespace repair.** The 16 `error_snapshots` children that imported
  `crate::helpers::…` (the nested crate's root) now import `super::helpers::…`.
  That is spec §3's "crate-namespace collision fixes".
- **Proptest regression files moved** beside their modules
  (`tests/l1/{schema_quoting_safety,schemas_grammar_proptest}.proptest-regressions`).
  Proptest resolves them relative to the source file. Left behind, the
  persisted seeds would silently stop replaying.
- **Inner `#![allow(deprecated)]`** in five files stays in the module file. It
  is valid there and covers the same code, and `check-attributes` accepts it.
  No moved helper needed it (all-features clippy with `-D warnings` is clean).
- **R9 self-re-exec identity**
  (`level2/level2_render_tree_terminal/support/mod.rs`,
  `--exact public_entry_points::level2_render_probe_entrypoint`) now carries
  the `level2_render_tree_terminal::` prefix. **Load-bearing, shown red first:**
  with the old string, `level2` ran 17/18, and
  `layout_policy::level2_matched_layout_policy_matches_no_policy_capabilities_in_real_terminal`
  panicked with "table header row missing from real-terminal capture". The
  re-exec'd probe selected no test. With the repair, 18/18 pass.
- **R5 override** `test(=every_catalog_variable_survives_ambient_options)` →
  `test(=ambient_ctx_capture::…)` in both profiles. It still selects its one
  test (`override-ci-7`, `override-default-2`).
- **`check-attributes` disposition** (manifest `dispositions`):
  `l1/interpolation_literal_pipeline.rs` runs its own executable with `--list`.
  That is not load-bearing: the test compares compose's shell expansion against
  that same executable's listing, never a test path. The listing is longer now
  and the test passes.
- **Shared layout gate.** Claudine's acceptance-12 gate (Phase 3) lived in
  `claudine/cli/tests/l1/test_placement.rs`. Three more packages need it, so
  its logic, with a self-contained sanitizer, is now
  `tools/test-toolkit/src/test_layout.rs` (`collect_test_sources`,
  `layout_violations`), with 10 unit tests. test-toolkit was already a
  dev-dependency of darkmatter and claudine-cli. It gains `toml = "1.0"` (the
  lockfile adds one edge to an existing `toml`, no new crate). **Claudine's own
  copy was left unchanged** so Phase 3's recorded evidence (its manifest
  `additions`) stays valid. Switching it to the shared module is a follow-up
  (see `message_to_agent`).
- **`renderable/justfile` `drift-report`** looped `cargo test -p $crate --test
  render_comparison` over biscuit-terminal and darkmatter. darkmatter now uses
  `--test l1 render_comparison::`. biscuit-terminal keeps the old selector
  until Phase 6. Verified: the recipe runs, and the darkmatter selector runs
  exactly 1 test that prints the ledger markers (`0 entries`, a real result, not
  a missed filter).
- **Persisted manifest header.** `benchmark_fixtures.rs`'s emit-mode header
  (R9 row 3) and the committed
  `darkmatter/features/2026-07-15-performance-followup/benchmarks/manifest.yaml`
  header comment were changed together. What `DM_BENCH_EMIT=1` writes still
  equals the committed file (the verify path ignores comments).

### Requirement-to-test mapping

| Changed behavior | Test / check (targeted) | Red-then-green shown |
|---|---|---|
| Every old test keeps identity, tier, ignore state, platform presence (all 6 feature sets, both slow-policy states) | `consolidation.py compare` macOS, Linux, macOS+Linux | toolkit oracles (Phase 2 mutation check); identical here |
| Exact-name override still selects its test | `compare` selectors `override-ci-7` / `override-default-2` (1 before, 1 after) | Phase 2 `test_an_exact_name_override_is_a_rewrite_not_an_alias` |
| Former file-level `cfg` moved to the declaration (`declined_path_transclusion` windows, `level3_*` macOS) | `check-attributes` (0 failures); Windows cross-compile; Linux listing (level3 absent, platform-absent evaluated) | Phase 2 mutation |
| Snapshots found at their new paths | `INSTA_UPDATE=no` L1 + L3 runs, `check-snapshots` | an unmapped move makes the reading test fail (S4, Phase 3) |
| Acceptance 12, darkmatter | `test_layout::every_test_source_is_compiled_by_a_declared_target` | yes: planted `tests/stray_probe.rs`, `tests/l1/orphan_probe.rs`, `tests/level9/main.rs` → red naming all three, then green |
| Shared gate logic | `test_toolkit::test_layout::tests::*` (10): stray file, undeclared root, undeclared modules (beside root, in helper dir, prose/string only), `autotests`/`path` rules, unparseable manifest, rustc resolution incl. `mod.rs` crate roots, sanitizer desync cases, data-dir skipping | the stray/root/module/manifest cases are the negative inputs |
| R9 re-exec identity | `level2` suite | yes (above) |
| `drift-report` recipe | `just drift-report` (renderable) + direct selector run | — |
| Guard file sets | temporary probe, `darkmatter/guard-scans-after.md` | populations compared by path |
| L3 opt-in kept | `level3-terminal`, `level3-browser` without `RUN_LEVEL3` | every test printed its skip reason |

"Passive corpus" and "shipped artifact end-to-end" are covered by the layout
gate reading the real `Cargo.toml` and tree, and by `compare` over real
captures. The one persisted value, the manifest header, is written by the
emit path and is not read back by any test.

### Gates run

- `cargo check --tests` and `cargo clippy --all-targets -D warnings` for
  darkmatter with `terminal-tests,browser-tests,effects-instrumentation`:
  clean. test-toolkit clippy: clean.
- `just lint` (darkmatter): exit 0. Its `_lint` uses no features, so it skips
  the four feature-gated targets. That was also true of their per-file
  predecessors.
- `just test --no-fail-fast` (darkmatter area): 7,947 passed, 1 failed
  (pre-existing), 7 skipped. `just test-l2`: 3 + 18 + 69 passed.
  `just test-browser`: 43 passed.
- `cargo nextest run -p test-toolkit`: 339 passed, including the archive-path
  guard.
- `x86_64-pc-windows-gnu` cross-compile of darkmatter (all features) and
  test-toolkit tests: clean.
- `just check-canonical darkmatter`: pass. `just check-tier-coverage
  darkmatter`: fails, pre-existing (below).
- `python3 scripts/ci/test_consolidation.py`: OK (toolkit unchanged).
- GitNexus `detect-changes --scope all`: risk low, 0 affected processes.
- Not run: `just test-l3` with `RUN_LEVEL3=1`. It takes focus (headed Chrome,
  cliclick), and its opt-in was verified instead. Also not run: the root-area
  fan-out.

### Pre-existing failures (proven on the unmigrated base `048e44f7a`)

- **All hosts:**
  `schema_phase_validation::public_docs_and_skill_describe_required_and_eager_as_independent_axes`
  reads `darkmatter/docs/topics/schema-definition.md`, which `aa1f03c70`
  deleted ("remove orphaned schema-definition.md after rename"). `main` has the
  same test and the same missing file. The base-tree run fails identically as
  its old per-file binary. It is not fixed here, because spec §3 forbids
  test-body edits in a migration. Separate-defect candidate: point it at
  `docs/topics/schemas/definition.md`.
- **All hosts:** `just check-tier-coverage darkmatter` reports
  `schema_phase_validation::real_shipped_inline_schema_uses_normal_resolution_and_phase_path`
  as a stranded `real` test. Its name starts with `real_`, and darkmatter's
  `test-real` is "not applicable". It fails identically on the base worktree.
  Separate-defect candidate: rename the test, or give the area a `test-real`.
- **Linux only:**
  `horizontal_rule_integration::tests::test_custom_weight_thick_differs_from_thin`
  sees identical ASCII `-` rules for thin and thick. It fails 3/3 on the base
  tree as its old binary on `build-linux` (an environment/terminal-detection
  difference over SSH). It passes on macOS.

### Host problems met

- **`build-win-native` `W:` still has 43 GB free**, under the 50 GiB preflight,
  and **`build-win` still resets SSH at key exchange**. Both are unchanged
  since Phase 3. No Windows build was attempted. Native-Windows listings and
  WSL2 archive portability stay pending for darkmatter as well as claudine-cli.
- **`build-linux` cross-check lock** is still held by `feat-nightly-perf`
  (since 2026-09-14). Linux evidence came from a private `--shared` clone, a
  bundle of `3efdbe450..HEAD`, and a binary patch of the working tree built
  with a temporary `GIT_INDEX_FILE` (the local index was untouched). The
  scratch clone and files were deleted afterward.

### Changed outside `darkmatter/lib/tests`

`darkmatter/lib/Cargo.toml`, `.config/nextest.toml` (R5), `Cargo.lock` (one
edge), `tools/test-toolkit/{Cargo.toml,README.md,src/lib.rs,src/test_layout.rs,src/test_layout/tests.rs}`,
`docs/dependencies.md`, `renderable/justfile`, `darkmatter/justfile` (comment),
`darkmatter/features/2026-07-15-performance-followup/benchmarks/manifest.yaml`
(header comment), and this feature's `pilot/apply-move.py` and
`measurements.md`. Comment-only path swaps: `biscuit-file/lib/tests/span_compat.rs`,
`darkmatter/cli/tests/level2_{code_block_styling,disclosure_blocks}.rs`,
`darkmatter/lib/src/markdown/render_tree/{inline_extension,style_tree_parity_tests}.rs`.
Doc-only: `darkmatter/docs/errors/README.md`, `darkmatter/docs/rendering/popover.md`,
`darkmatter/lib/README.md`, `darkmatter/lib/tests/fixtures/mermaid/README.md`,
and skills `.claude/skills/darkmatter/{errors,structure}.md`.

Commit split, per R7 and the AGENTS.md scope discipline:

1. manifest and evidence
2. structural move (the `tests/` tree, `Cargo.toml`, `.config/nextest.toml`,
   the R9 re-exec repair)
3. the shared test-toolkit gate, with the darkmatter `test_layout.rs` and its
   manifest `addition`
4. docs, comments, and the recipe pass, including the `benchmark_fixtures.rs`
   header string and `manifest.yaml`

### Unrelated working-tree changes

None observed.

## Phase 5

Worked 2026-09-21 on macOS (`aarch64-apple-darwin`, rustc 1.98.1,
cargo-nextest 0.9.136) against base revision `cb9a3d38b`, with a clean tree
at start. All evidence is under `darkmatter-cli/` in this feature directory,
plus `darkmatter-cli-migration.json`.

### What was done

| Plan task | Result | Artifact |
|---|---|---|
| Fresh before side | re-captured unmigrated (`capture-a` is refused after the Phase 3/4 override rewrites). `compare` against `selfproof/capture-a`: identical, and the 8 notes are the claudine/darkmatter override rewrites | `darkmatter-cli/capture-before` |
| Manifest and move | 53 targets → 2: `l1` (42 modules, no features) and `level2` (11 modules, `terminal-tests`). `autotests = false`, 2 explicit `[[test]]`. The planner produced the one known alias, `level2_harness_integrity` → `harness_integrity` (R2). 647 integration tests mapped, plus 1 addition (the 784 listed with `terminal-tests` include the lib and bin unit tests) | `darkmatter-cli-migration.json` |
| `check-attributes` | 0 failures. 2 review items, both the reviewed `#[path = "../common/source_scan.rs"]` repair below | `darkmatter-cli/attribute-check.json` |
| Snapshot moves | none needed: the package tracks no Insta snapshots or proptest regression files. `check-snapshots` with the rule-derived (empty) mapping: 0 moved, 0 failures, 0 `.snap.new` | `darkmatter-cli/snapshot-{mapping,check}.json` |
| Identity comparison, macOS | **identical**, 0 failures, 0 notes: 2 feature sets × 23 selectors (incl. `L1-include-slow`, R4) | `darkmatter-cli/comparison-darwin.{json,md}` |
| Structural guard | `tests/l1/test_layout.rs` calls the shared `test_toolkit::test_layout` | manifest `additions` (1 test) |
| Spawn-site guard | runs inside `l1`, 24/24 pass. Populations by path: walked 58 → 61, spawn 42 → 44, isolation 40 → 40. The only additions are the 2 roots and `l1/test_layout.rs` (spawn: `l1/main.rs` and `l1/test_layout.rs`). Nothing is missing | `darkmatter-cli/guard-scans-after.md` |
| Archive-path guard | `darkmatter/cli/tests/**` 58 → 61, same three additions, eligibility unchanged, `files-checked=3491` | same file |
| Area recipe/doc pass | `lint-files` accepted-exception paths, README, skill, 3 cross-area comments | list below |
| macOS suites | `just test` 7,948/7,949 (the 1 red is pre-existing, below). `just test-l2` 18 + 69 + 3 passed. `just lint` exit 0. `check-tier-coverage darkmatter` red for the pre-existing stranded `real_` test only. `check-canonical darkmatter` passes | — |
| Linux | on-host before/after captures **identical**. L1 712/712 passed | `darkmatter-cli/comparison-linux.*`, `darkmatter-cli/linux-l1-summary.txt` |
| macOS + Linux platform-absent | **identical**, evaluated on both hosts | `darkmatter-cli/comparison-darwin-linux.*` |
| Native Windows | **pending**. `just cross-check darkmatter-cli --os windows` refused at storage preflight: `W:` has 37.9 GiB free (down from 43), after the recipe's automatic reclaim. The cross-compile of both targets with `terminal-tests` for `x86_64-pc-windows-gnu` is clean (0 warnings). That is compile evidence only | — |
| WSL2 | **pending**: its VHDX lives on the same full `W:` | — |
| Lightweight observations | 53 → 2 targets, 396.6 MB → 148.7 MB (−62%) | `measurements.md` §Rollout |
| Skip-baseline check | `.github/ci/ci-baseline.toml` holds only `schema_version = 3` (0 `[[skip]]`) | — |

### Decisions made during the move

- **Spawn-guard exclusion follows the tier directory.** `excluded` dropped
  Level 2 files by their `level2_` file-name prefix. After the move, the
  aliased `level2/harness_integrity.rs` no longer carries it, and neither does
  `level2/main.rs`. Both would have joined the spawn population silently. The
  harness-integrity file has no detected site today, so the guard would still
  have passed, but it would have scanned a file the baseline excludes. A first
  path segment equal to a prefix without its `_` (`level2/`, `level3/`,
  `browser/`, `real/`, `slow/`) now excludes too. Only a whole leading segment
  counts (`level2x/…` and `l1/level2/…` stay governed; both asserted). The
  module docs were updated. **Shown red first:** with the new clause
  disabled, `tier_naming_decides_what_the_guard_governs` and
  `the_spawn_gate_reads_a_real_population_and_still_finds_a_planted_site`
  fail on `level2/harness_integrity.rs`.
- **Guard assertions moved with the paths.** `ISOLATION_ALLOWLIST`'s entry is
  now `l1/md_process_fixture.rs`, and the population assertions name
  `l1/…`/`level2/…`. Left alone, `!names.contains("level2_errors.rs")` would
  have passed vacuously. The two failure messages that tell a developer where
  to add an allow-list entry name `cli/tests/l1/spawn_site_guard.rs`.
- **`source_scan` keeps a `#[path]`, `protected_env` becomes an import.**
  `common/mod.rs` declares `protected_env` (the fixture builder uses it) but
  not `source_scan`, whose only user is this guard. So the guard's
  `#[path = "common/protected_env.rs"] mod protected_env;` copy became
  `use crate::common::protected_env::…` (it would otherwise compile twice in
  `l1`). The sanitizer stays a guard-local `#[path = "../common/source_scan.rs"]`
  instead of widening `common` into `level2`. The old comment's reason (this
  binary avoided `common`) no longer applied and was rewritten.
- **`level2/main.rs` module order** is alphabetical (`harness_integrity`
  first), so a future `rustfmt` does not reorder it. The mover emitted it in
  manifest order.
- **`lint-files` (darkmatter/justfile)** keyed four accepted over-cap
  exceptions on `./cli/tests/<file>.rs`. After the move, the four files showed
  up as unaccepted (the recipe is advisory, exit 0, not in `lint`). They are
  re-keyed to `./cli/tests/l1/…`. `md_process_fixture.rs`'s reason lost its
  "one binary so mod common compiles once" clause, which no longer holds.
- **Comment drift** in `l1/clean_frontmatter.rs`'s header: "this target" →
  "this module" (twice).
- **Before-side size build in its own target dir.** Phase 4's base worktree
  (`/tmp/p4/base`) shared the main target dir. `zed-dmls-cli`'s build script
  kept that path, and the first `just test` here failed with
  `NotFound` in that build script. `cargo clean -p zed-dmls-cli` fixed it, and
  the before-side build for the size row used `--target-dir /tmp/p5/target-before`,
  since deleted. Recorded in `measurements.md`.

### Requirement-to-test mapping

| Changed behavior | Test / check (targeted) | Red-then-green shown |
|---|---|---|
| Every old test keeps identity, tier, ignore state, platform presence (both feature sets, both slow-policy states) | `consolidation.py compare` macOS, Linux, macOS+Linux | toolkit oracles (Phase 2 mutation check); identical here |
| Neutral alias keeps `harness_integrity` tests in L1 under `terminal-tests` | same `compare` (`L1`/`L2` selectors, `terminal-tests`: 715/69 before and after) | Phase 2 alias oracle |
| Acceptance 12, darkmatter-cli | `test_layout::every_test_source_is_compiled_by_a_declared_target` | yes: planted `tests/stray_probe.rs`, `tests/level2/orphan_probe.rs`, `tests/level9/main.rs` → red naming all three, then green |
| Spawn/isolation guards keep their intended population | `spawn_site_guard::tier_naming_decides_what_the_guard_governs` (new `level2/…`, `l1/…`, and segment-boundary cases), `the_spawn_gate_reads_a_real_population_…`, `the_isolation_gate_governs_…`, plus a temporary probe diffed by path against the baseline | yes (above) |
| Guard file sets (acceptance 7) | temporary probes, `darkmatter-cli/guard-scans-after.md` | populations compared by path |
| Level 2 opt-in unchanged | `just test` (no `terminal-tests`) builds no `level2`. `just test-l2` runs 69 | — |
| `lint-files` exceptions | `just lint-files` shows the four `l1/…` files as accepted again | the unfixed run listed them as unaccepted |

"Passive corpus" and "shipped artifact end-to-end": the layout gate reads the
real `Cargo.toml` and tree, and `compare` runs over real captures. No value is
persisted and read back.

### Gates run

- `cargo check --tests` (none, `terminal-tests`) and `cargo clippy
  --all-targets --features terminal-tests -- -D warnings` for darkmatter-cli:
  clean.
- `just lint` (darkmatter): exit 0.
- `just test --no-fail-fast` (darkmatter area): 7,948 passed, 1 failed
  (pre-existing), 7 skipped. `just test-l2`: 18 + 69 + 3 passed.
- `just check-canonical darkmatter`: pass. `just check-tier-coverage
  darkmatter`: fails, pre-existing (below).
- `x86_64-pc-windows-gnu` cross-compile of darkmatter-cli tests with
  `terminal-tests`: clean.
- Linux (`build-linux`, private `--shared` clone at `cb9a3d38b` + bundle +
  working-tree patch, `RUSTC_WRAPPER=""`): before/after captures, and L1
  712/712. The scratch clone and uploads were deleted.
- `cargo nextest run -p test-toolkit --test archive_path_guard`: 3/3 pass
  (with the temporary probe, since removed).
- GitNexus `detect-changes --scope all`: risk low, 0 affected processes.
- Not run: the root-area fan-out, and `just test-l3` (darkmatter-cli has no
  Level 3 tests).

### Pre-existing failures (the same as Phase 4, proven there on the unmigrated base)

- `darkmatter::l1 schema_phase_validation::public_docs_and_skill_describe_required_and_eager_as_independent_axes`
  (reads a doc deleted by `aa1f03c70`).
- `check-tier-coverage darkmatter`: stranded
  `darkmatter::l1::schema_phase_validation::real_shipped_inline_schema_uses_normal_resolution_and_phase_path`.
- Neither is in darkmatter-cli. Every darkmatter-cli test passed on macOS and
  Linux.

### Host problems met

- **`build-win-native` `W:` has 37.9 GiB free**, after the recipe's own
  automatic Cargo reclaim, under the 50 GiB preflight. It had 43 GB in
  Phase 4. No override was used. Native-Windows listings and WSL2 are pending
  for all three migrated packages.
- **`build-linux` cross-check lock** is still held by `feat-nightly-perf`
  (since 2026-09-14). Linux evidence came from the private-clone procedure in
  the `os` skill.

### Left alone (noted for Phase 7)

- `claudine/cli/tests/l1/spawn_site_guard.rs` failure messages still name
  `cli/tests/spawn_site_guard.rs` (stale since Phase 3; claudine is not this
  phase's package).
- `.claude/skills/rust-testing/SKILL.md:673` says "each area's
  `cli/tests/spawn_site_guard.rs`". Two of its three areas are now under
  `cli/tests/l1/` (sniff's is not migrated). That is the Phase 7 skill rewrite.
- `l1/layout_style_frontmatter.rs` cites `darkmatter/cli/tests/level2_layout.rs`,
  which did not exist before this phase either.

### Changed outside `darkmatter/cli/tests`

`darkmatter/cli/Cargo.toml`, `darkmatter/justfile` (`lint-files`),
`darkmatter/README.md`, `.claude/skills/darkmatter/SKILL.md`, comment-only path
swaps in `biscuit-terminal/lib/tests/level2_terminal_osc_wezterm.rs`,
`claudine/cli/tests/level2/level2_invalid_file_reference_capture.rs`, and
`darkmatter/lib/tests/l1/clean_counters.rs`, and this feature's
`measurements.md`. `.config/nextest.toml` was not touched, because no override
names a darkmatter-cli test.

Commit split, following Phase 4: (1) manifest and evidence; (2) structural
move (the `tests/` tree, `Cargo.toml`, the spawn-guard path repair, and the
layout gate with its manifest `addition`); (3) docs, comments, and the
`lint-files` recipe.

### Unrelated working-tree changes

None observed.

## Phase 6

Worked 2026-09-22 on macOS (`aarch64-apple-darwin`) against base revision
`beca6c368`, with a clean tree at start. All evidence is under
`biscuit-terminal/` in this feature directory, plus
`biscuit-terminal-migration.json`. The Phase 5 human-review items (free `W:`
before Phase 6; confirm the manifest as sole authority) were still open. This
phase went ahead on the prompt's instruction and re-tried Windows first; it is
still blocked (below).

### What was done

| Plan task | Result | Artifact |
|---|---|---|
| Fresh before side | re-captured unmigrated, 5 feature sets (`none`, `image`, `terminal-tests`, `browser-tests`, all three). `compare` against `selfproof/capture-a`: identical, and the 20 notes are the claudine/darkmatter override rewrites | `biscuit-terminal/capture-before` |
| Manifest and move | 38 targets → 2: `l1` (37 modules, no features) and `level2` (1 module, `terminal-tests`). `autotests = false`, 2 explicit `[[test]]`. No alias and no override rewrite: `.config/nextest.toml`'s `test(=level2_render_tree_style_in_wezterm)` belongs to `biscuit-terminal-cli`, which is not migrated. 1,121 tests mapped (432 of them the per-module `parity_helpers` copies), plus 1 addition (the layout gate) | `biscuit-terminal-migration.json` |
| `check-attributes` | 0 failures. The review items are the reviewed `#[path]` repairs below | `biscuit-terminal/attribute-check.json` |
| Snapshot moves | 1,086 moved byte-for-byte, `tests/snapshots/*` → `tests/l1/snapshots/l1__*` (60 `inline_content_matrix`, 1,026 `layout_matrix`). 117 unaffected (`src/**/snapshots`). 744 are read by running tests; 342 only by the `#[ignore]`d `layout_matrix_snapshots`, which `check-snapshots` states. `INSTA_UPDATE=no` suites green, 0 `.snap.new` | `biscuit-terminal/snapshot-{mapping,check}.json` |
| Identity comparison, macOS | **identical**, 0 failures, 0 notes: 5 feature sets × 22 selectors | `biscuit-terminal/comparison-darwin.{json,md}` |
| Structural guard | `tests/l1/test_layout.rs` calls the shared `test_toolkit::test_layout` | manifest `additions` (1 test) |
| Guard file sets | archive-path guard over `biscuit-terminal/lib/tests/**`: 42 → 45, the only additions are the 2 roots and `l1/test_layout.rs`. Nothing missing, eligibility unchanged, `files-checked=3494`. The package has no spawn/isolation guard | `biscuit-terminal/guard-scans-after.md` |
| Area recipe/doc pass | `renderable` `drift-report`, 3 module-doc regeneration selectors, path comments, README, 2 skills, area `docs/dependencies.md` | list below |
| macOS suites | `just test` 3,263/3,263. `just test-l2` lib 2/2 + CLI 76/76 (after one transient CLI timeout, below). `just test-browser` 54/54. `just lint` exit 0. `check-tier-coverage biscuit-terminal` and `check-canonical biscuit-terminal` pass | — |
| Linux | on-host before/after captures **identical**. `just test` (lib + CLI L1) 3,257/3,257 | `biscuit-terminal/comparison-linux.*`, `biscuit-terminal/linux-l1-summary.txt` |
| macOS + Linux platform-absent | **identical**, evaluated on both hosts for all 5 feature sets | `biscuit-terminal/comparison-darwin-linux.*` |
| Native Windows | **pending**. `just cross-check biscuit-terminal --os windows` refused at storage preflight: `W:` has **36 GiB** free (37.9 in Phase 5) after the recipe's automatic reclaim cleaned nothing. The `Ubuntu-26.04 ext4.vhdx` is 130.8 GiB, 47% of the volume. Cross-compile for `x86_64-pc-windows-gnu`, clippy `--tests -D warnings`, with all three features and with none: clean. Compile evidence only | — |
| WSL2 | **pending**: its VHDX is on the same `W:`. Not attempted, because a full `W:` has reset SSH to both Windows hosts before | — |
| Lightweight observations | 38 → 2 targets, 1.35 GB → 0.10 GB (−93%) | `measurements.md` §Rollout |
| Skip-baseline check | `.github/ci/ci-baseline.toml` holds only `schema_version = 3` (0 `[[skip]]`) | — |

### Decisions made during the move

- **`parity_helpers` keeps one private copy per parity module.** It is both a
  test target (24 unit tests in its `mod tests`) and a helper that 18 parity
  files declared with `mod parity_helpers;`, so each old binary ran its own
  copy of those 24 tests (`<parity_file>::parity_helpers::tests::*`). The
  planner kept those identities, so they are kept: `parity_helpers` is an
  `l1` module, and each parity module declares
  `#[allow(clippy::duplicate_mod)] #[path = "parity_helpers.rs"] mod
  parity_helpers;` (a path repair). Replacing the copies with `use
  crate::parity_helpers;` would have removed 432 test identities, a behavior
  change that spec §3 does not allow. `clippy -D warnings` rejects the
  duplicate module without the `allow`, which sits only on the 18 copies. The
  root declaration keeps the lint, and a comment there says why. Whether to
  deduplicate is a follow-up question for the author, not a structural edit.
- **Helpers (R2).** `layout_matrix_support/` (two `l1` modules, plus
  `examples/layout_matrix.rs`) stays at `tests/` and is declared once at the
  `l1` root by `#[path]`. Its two users now `use crate::layout_matrix_support;`,
  as in darkmatter. `inline_content_matrix_support/` has one user whose file
  name differs from the directory, so moving it "beside" the file would not
  have kept `mod inline_content_matrix_support;` resolving anyway. It stays at
  `tests/` and its user declares it with
  `#[path = "../inline_content_matrix_support/mod.rs"]`.
- **`level2` root declares no `common`.** The mover adds
  `#[path = "../common/mod.rs"] mod common;` to every root when
  `tests/common/` exists. `level2_terminal_osc_wezterm` never used it, so it
  was removed from `level2/main.rs` rather than widening that binary.
- **`common` stays `#[cfg(unix)]`-free at the root.** `level1_terminal_osc_cache`
  declared `#[cfg(unix)] mod common;`, and it is now `#[cfg(unix)] use
  crate::common;`. `level1_apple_terminal_prose` already declared it
  unconditionally, so `common` compiled on Windows before as well (Windows
  cross-compile clean).
- **The layout gate needs a dev-dependency.** `test-toolkit` was only an
  optional `terminal-tests` dependency, so it is now also an unconditional
  `[dev-dependencies]` entry. It adds no crate to the graph (`Cargo.lock`
  unchanged, `--locked` builds pass): its dependencies are already built by
  the test build. Recorded in `biscuit-terminal/docs/dependencies.md`.
- **Nothing path-keyed needed repair.** Every fixture path goes through
  `manifest_dir!()`. `level2_terminal_osc_wezterm`'s `discovery_probe_path()`
  climbs from `current_exe()` in `target/<profile>/deps/`, which the move
  does not change (Level 2 passes).
- **The Phase 5 note about snapshots was partly wrong.** It said all 1,086
  `layout_matrix__*` snapshots belong to the `#[ignore]`d test. Only 342 do.
  684 are read by the running `layout_matrix_markdown_snapshots` and
  `layout_matrix_browser_snapshots`, and those pass with `INSTA_UPDATE=no` at
  the new paths.
- **`drift-report` no longer branches per crate.** Both crates now use
  `--test l1 render_comparison::`, so the `case` became one inline selector.
  Verified: the recipe runs, and the biscuit-terminal selector runs exactly
  1 test that prints both `KNOWN_DRIFT` ledger markers.
- **Before-side size build in its own worktree and target dir**
  (`/tmp/p6/base`, `/tmp/p6/target-before`), both deleted afterward.

### Requirement-to-test mapping

| Changed behavior | Test / check (targeted) | Red-then-green shown |
|---|---|---|
| Every old test keeps identity, tier, ignore state, platform presence (5 feature sets; biscuit-terminal sets no `l1-include-slow`, so there is no second slow-policy selector, and R4 found no `slow_` tests) | `consolidation.py compare` macOS, Linux, macOS+Linux | toolkit oracles (Phase 2 mutation check); identical here |
| 432 `parity_helpers::tests::*` copies kept | same `compare` (each is a manifest row) | a dedup would fail `compare` as unmapped-missing |
| Former file-level `cfg` moved to the declaration (`unix` ×3, `image` ×3) | `check-attributes` (0 failures); `compare` on the `none` vs `image` sets; Windows cross-compile | Phase 2 mutation |
| Snapshots found at their new paths | `INSTA_UPDATE=no just test`, `check-snapshots` | an unmapped move makes the reading test fail (S4, Phase 3) |
| Acceptance 12, biscuit-terminal | `test_layout::every_test_source_is_compiled_by_a_declared_target` | yes: planted `tests/stray_probe.rs`, `tests/l1/orphan_probe.rs`, `tests/level9/main.rs` → red naming all three, then green |
| Guard file sets (acceptance 7) | temporary probe, `biscuit-terminal/guard-scans-after.md` | populations compared by path |
| Level 2 opt-in unchanged | `just test` (no `terminal-tests`) builds no `level2`; `just test-l2` runs its 2 | — |
| Browser tier still selected inside `l1` | `just test-browser` 54/54, plus `compare`'s `browser` selector | — |
| `drift-report` recipe | `just drift-report` (renderable) + direct selector run | — |

"Passive corpus" and "shipped artifact end-to-end": the layout gate reads the
real `Cargo.toml` and tree, and `compare` runs over real captures. No value is
persisted and read back.

### Gates run

- `cargo clippy -p biscuit-terminal --all-targets -- -D warnings` with no
  features, with `image`, and with `image,terminal-tests,browser-tests`: clean.
- `x86_64-pc-windows-gnu` clippy of the test targets, all features and none:
  clean.
- `just lint` (biscuit-terminal): exit 0.
- `INSTA_UPDATE=no just test --no-fail-fast` (biscuit-terminal area): 3,263
  passed, 0 failed, 55 skipped.
- `just test-l2`: first run, lib 2/2 and CLI 47/48, with
  `biscuit-terminal-cli::level2_prose_cells level2_prose_cells_in_wezterm`
  failing on `send_command_with_env ... command timed out after 10s`. That
  package is not migrated or touched. The test passed alone (6.96 s) and the
  whole recipe then passed: lib 2/2, CLI 76/76. Recorded as a transient
  WezTerm timeout.
- `just test-browser`: 54/54.
- `just check-tier-coverage biscuit-terminal`, `just check-canonical
  biscuit-terminal`: pass.
- `cargo nextest run -p test-toolkit --test archive_path_guard`: 3/3 (with the
  temporary probe, since removed; the file was restored byte-for-byte).
- Linux (`build-linux`, private `--shared` clone at `beca6c368` + bundle
  `3efdbe450..HEAD` + working-tree patch, `RUSTC_WRAPPER="" KACHE_AUTO=0`):
  before/after captures, and `just test` 3,257/3,257. The lock is still held
  by `feat-nightly-perf` (since 2026-09-14). The scratch clone and uploads
  were deleted.
- GitNexus `detect-changes --scope all`: risk low, 0 affected processes.
- Not run: the root-area fan-out, `just test-l3` (biscuit-terminal has no
  Level 3 tests).

### Pre-existing failures

None in this package. The two darkmatter-library failures recorded in Phases 4
and 5 are outside this area and were not re-run.

### Host problems met

- **`build-win-native` `W:` has 36 GiB free**, still going down (43 → 37.9 →
  36 across Phases 4–6). The recipe's automatic reclaim cleaned nothing. No
  override was used. Native-Windows listings and WSL2 are pending for **all
  four** migrated packages.
- **`build-linux` cross-check lock** is still held by `feat-nightly-perf`.

### Changed outside `biscuit-terminal/lib/tests`

`biscuit-terminal/lib/Cargo.toml`; comment-only path updates in
`biscuit-terminal/lib/examples/discovery_probe.rs`,
`biscuit-terminal/lib/src/discovery/osc_queries/query.rs`, and
`claudine/cli/tests/common/pty.rs`; `renderable/justfile` (`drift-report`);
`biscuit-terminal/README.md`; `biscuit-terminal/docs/dependencies.md`;
`.claude/skills/biscuit-terminal/SKILL.md` (new layout bullet);
`.claude/skills/renderable/tree.md` (`tests/l1/perf_gate.rs`); and this
feature's `measurements.md` and `plan.md`. `.config/nextest.toml` was not
touched.

Suggested commit split, following Phases 4–5: (1) manifest and evidence;
(2) structural move (the `tests/` tree including snapshots, `Cargo.toml`, the
`parity_helpers`/helper path repairs, and the layout gate with its manifest
`addition`); (3) docs, comments, the skills, and the `drift-report` recipe.

### Left for Phase 7

- The Phase 5 list still stands (rust-testing SKILL.md:673 spawn-guard path,
  claudine's `l1/spawn_site_guard.rs` failure messages, claudine's own
  layout-gate copy → `test_toolkit::test_layout`).
- `biscuit-terminal/README.md`'s test table lists only the CLI's Level 2
  location (`cli/tests/level2_*.rs`). The library's Level 2 now lives in
  `lib/tests/level2/`. The row predates this phase and is left for the sweep.

### Unrelated working-tree changes

None observed.
