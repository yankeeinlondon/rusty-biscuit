---
kind: evidence
feature: 2026-09-21-consolidated-test-binaries
created: 2026-09-21
plan_phase: 3
host: macOS aarch64-apple-darwin
---

# Guard after-scans — `claudine-cli`

The file sets the three path-sensitive guards scan after the pilot move,
compared by path with `../baseline/guard-scans.md` (acceptance 7). Each
before path was mapped through `../claudine-cli-migration.json` (plus the
`error_guards/` helper directory, which moved beside `l1/error_guards.rs`
per R2). Produced the Phase 1 way: a temporary probe test calling each
guard's own population functions, run once, then removed (the files were
verified byte-identical to their pre-probe state).

| Guard | Before | After | Extra after mapping | Missing after mapping | Eligibility changed |
|---|---:|---:|---|---|---|
| `spawn_site_guard.rs` walked | 148 | 152 | the 4 new `main.rs` roots | none | — |
| spawn gate, governed | 103 | 107 | the 4 new `main.rs` roots | none | — |
| isolation gate, governed | 88 | 88 | none | none | — |
| archive-path guard, `claudine/cli/tests/**` | 148 | 152 | the 4 new `main.rs` roots | none | none (all `true`) |

The roots join the spawn population because they are neither under `common/`
nor `real_`-named, and they hold no spawn site. They stay out of the isolation
population because they never name `CliProcessFixture`. That is the growth
`../baseline/guard-scans.md` §"What changes after migration" anticipated.
The spawn gate census reads `governed_files: 107`, the isolation gate `88`.
The archive-path guard reports `files-checked=3480` against the baseline
run's 3476, the same four files.

## `spawn_site_guard.rs` walked (152, relative to `claudine/cli/tests/`)

```text
common/completion.rs
common/host_tools.rs
common/incomplete_subagents.rs
common/mod.rs
common/pty.rs
common/review_router.rs
common/source_scan.rs
common/wrap.rs
l1/agent_cwd.rs
l1/argv_normalization.rs
l1/characterization_error_routes.rs
l1/cli_process_fixture.rs
l1/command_routing.rs
l1/completion_cli.rs
l1/completion_compose.rs
l1/completion_contract.rs
l1/completion_inline_compose.rs
l1/completion_perf.rs
l1/completion_resolution_round_trip.rs
l1/completion_sequence.rs
l1/completion_setter.rs
l1/compose_caller_file_provenance.rs
l1/compose_cli.rs
l1/compose_frontmatter_model.rs
l1/compose_header_first.rs
l1/compose_initialize_acceptance.rs
l1/compose_initialize_staged_boot.rs
l1/compose_interactive_timeout_cli.rs
l1/compose_removed_validation_keys.rs
l1/compose_repository_context.rs
l1/compose_schema_cli.rs
l1/compose_system_prompt_lifetime.rs
l1/compose_ttff_perf.rs
l1/composition_outputs.rs
l1/composition_seams.rs
l1/contamination_probes.rs
l1/context_command.rs
l1/contextual_errors.rs
l1/ctx_launch_anchor.rs
l1/detached_audio.rs
l1/diagnostic_discovery.rs
l1/dispatch_inventory.rs
l1/effective_diagnostic_render.rs
l1/error_guards.rs
l1/error_guards/source_scan.rs
l1/errors_command.rs
l1/handle_blocking_output.rs
l1/handle_deadline.rs
l1/handle_repo_config.rs
l1/hooks_cli.rs
l1/inline_completion_lifecycle.rs
l1/inline_compose_cli.rs
l1/inline_compose_hash.rs
l1/inline_compose_sequence_mismatch.rs
l1/level1_compose_autocomplete_failure_pty.rs
l1/level1_dry_run_pty.rs
l1/level1_inline_compose_mismatch_pty.rs
l1/level1_provided_partial_file_pty.rs
l1/level1_provider_overlay_home.rs
l1/level1_pty_wrapper_summary.rs
l1/level1_review_router_partial_pty.rs
l1/level1_schema_prompt_pty.rs
l1/level1_structured_error_message.rs
l1/loop_cli.rs
l1/loop_initialize_state.rs
l1/main.rs
l1/mcp_cli.rs
l1/prompt_reporting.rs
l1/propagated_context_fixtures.rs
l1/protect_cli.rs
l1/provider_error_finalize.rs
l1/run_harness_loop_call_sites.rs
l1/sequence_budget.rs
l1/sequence_cli.rs
l1/sequence_ctrl_c_windows.rs
l1/sequence_errors_cli.rs
l1/sequence_groups.rs
l1/sequence_initialize_include_preflight.rs
l1/sequence_jit.rs
l1/sequence_magic_reference.rs
l1/sequence_overlay_pty.rs
l1/sequence_perf.rs
l1/sequence_prompt_property.rs
l1/sequence_schema.rs
l1/sequence_sources_cli.rs
l1/shipped_prompt_contract.rs
l1/shipped_prompt_route_drift.rs
l1/shipped_prompts.rs
l1/skills_integration.rs
l1/spawn_site_guard.rs
l1/system_prompt_perf_bench.rs
l1/test_placement.rs
l1/wrap_antigravity_exit_signal.rs
l1/wrap_basics.rs
l1/wrap_compose_agent.rs
l1/wrap_compose_exec.rs
l1/wrap_compose_preflight.rs
l1/wrap_compose_validation.rs
l1/wrap_ctrl_c_windows.rs
l1/wrap_direct_argv.rs
l1/wrap_incomplete_subagents.rs
l1/wrap_inline_compose.rs
l1/wrap_inline_compose_interactive.rs
l1/wrap_opencode.rs
l1/wrap_opencode_models.rs
l1/wrap_perf.rs
l1/wrap_provider_flags.rs
l1/wrap_sequence_composition.rs
l1/wrap_sigint.rs
l1/wrap_structured_stream.rs
l1/wrap_watchdog_startup_stall.rs
l1/wrap_watchdog_timeout.rs
level2/level2_auto_complete_chooser.rs
level2/level2_auto_complete_operation_file.rs
level2/level2_context_capture.rs
level2/level2_dry_run_approval_capture.rs
level2/level2_dry_run_metadata_capture.rs
level2/level2_explicit_operation_file_miss.rs
level2/level2_file_resolution_capture.rs
level2/level2_incomplete_subagents_capture.rs
level2/level2_initialize_generated_transclusion.rs
level2/level2_inline_compose_mismatch_capture.rs
level2/level2_interrupt_feedback_capture.rs
level2/level2_invalid_file_reference_capture.rs
level2/level2_lifecycle_action_forms.rs
level2/level2_lifecycle_control.rs
level2/level2_lifecycle_dispatch.rs
level2/level2_lifecycle_loop.rs
level2/level2_malformed_frontmatter_capture.rs
level2/level2_perf_capture.rs
level2/level2_prompt_reporting_capture.rs
level2/level2_provided_partial_file_capture.rs
level2/level2_provider_overlay_capture.rs
level2/level2_removed_validation_key_capture.rs
level2/level2_schema_parse_capture.rs
level2/level2_sequence_task_stream_capture.rs
level2/level2_stalled_generation_capture.rs
level2/level2_typed_error_render_capture.rs
level2/level2_windows_provided_partial_file_capture.rs
level2/level2_wrap_ctrl_c_loop_wedge_tmux.rs
level2/level2_wrap_ctrl_c_tmux.rs
level2/main.rs
level3/level3_auto_complete_chooser.rs
level3/level3_linux_sequence_ctrl_c.rs
level3/level3_sequence_ctrl_c.rs
level3/level3_windows_sequence_ctrl_c.rs
level3/level3_wrap_ctrl_c.rs
level3/main.rs
real/main.rs
real/real_inline_write_grant.rs
real/real_opencode_yolo_subagent.rs
real/real_pi_steering.rs
```

## spawn gate — governed (107)

```text
l1/agent_cwd.rs
l1/argv_normalization.rs
l1/characterization_error_routes.rs
l1/cli_process_fixture.rs
l1/command_routing.rs
l1/completion_cli.rs
l1/completion_compose.rs
l1/completion_contract.rs
l1/completion_inline_compose.rs
l1/completion_perf.rs
l1/completion_resolution_round_trip.rs
l1/completion_sequence.rs
l1/completion_setter.rs
l1/compose_caller_file_provenance.rs
l1/compose_cli.rs
l1/compose_frontmatter_model.rs
l1/compose_header_first.rs
l1/compose_initialize_acceptance.rs
l1/compose_initialize_staged_boot.rs
l1/compose_interactive_timeout_cli.rs
l1/compose_removed_validation_keys.rs
l1/compose_repository_context.rs
l1/compose_schema_cli.rs
l1/compose_system_prompt_lifetime.rs
l1/compose_ttff_perf.rs
l1/composition_outputs.rs
l1/composition_seams.rs
l1/contamination_probes.rs
l1/context_command.rs
l1/contextual_errors.rs
l1/ctx_launch_anchor.rs
l1/detached_audio.rs
l1/diagnostic_discovery.rs
l1/dispatch_inventory.rs
l1/effective_diagnostic_render.rs
l1/error_guards/source_scan.rs
l1/error_guards.rs
l1/errors_command.rs
l1/handle_blocking_output.rs
l1/handle_deadline.rs
l1/handle_repo_config.rs
l1/hooks_cli.rs
l1/inline_completion_lifecycle.rs
l1/inline_compose_cli.rs
l1/inline_compose_hash.rs
l1/inline_compose_sequence_mismatch.rs
l1/level1_compose_autocomplete_failure_pty.rs
l1/level1_dry_run_pty.rs
l1/level1_inline_compose_mismatch_pty.rs
l1/level1_provided_partial_file_pty.rs
l1/level1_provider_overlay_home.rs
l1/level1_pty_wrapper_summary.rs
l1/level1_review_router_partial_pty.rs
l1/level1_schema_prompt_pty.rs
l1/level1_structured_error_message.rs
l1/loop_cli.rs
l1/loop_initialize_state.rs
l1/main.rs
l1/mcp_cli.rs
l1/prompt_reporting.rs
l1/propagated_context_fixtures.rs
l1/protect_cli.rs
l1/provider_error_finalize.rs
l1/run_harness_loop_call_sites.rs
l1/sequence_budget.rs
l1/sequence_cli.rs
l1/sequence_ctrl_c_windows.rs
l1/sequence_errors_cli.rs
l1/sequence_groups.rs
l1/sequence_initialize_include_preflight.rs
l1/sequence_jit.rs
l1/sequence_magic_reference.rs
l1/sequence_overlay_pty.rs
l1/sequence_perf.rs
l1/sequence_prompt_property.rs
l1/sequence_schema.rs
l1/sequence_sources_cli.rs
l1/shipped_prompt_contract.rs
l1/shipped_prompt_route_drift.rs
l1/shipped_prompts.rs
l1/skills_integration.rs
l1/spawn_site_guard.rs
l1/system_prompt_perf_bench.rs
l1/test_placement.rs
l1/wrap_antigravity_exit_signal.rs
l1/wrap_basics.rs
l1/wrap_compose_agent.rs
l1/wrap_compose_exec.rs
l1/wrap_compose_preflight.rs
l1/wrap_compose_validation.rs
l1/wrap_ctrl_c_windows.rs
l1/wrap_direct_argv.rs
l1/wrap_incomplete_subagents.rs
l1/wrap_inline_compose.rs
l1/wrap_inline_compose_interactive.rs
l1/wrap_opencode.rs
l1/wrap_opencode_models.rs
l1/wrap_perf.rs
l1/wrap_provider_flags.rs
l1/wrap_sequence_composition.rs
l1/wrap_sigint.rs
l1/wrap_structured_stream.rs
l1/wrap_watchdog_startup_stall.rs
l1/wrap_watchdog_timeout.rs
level2/main.rs
level3/main.rs
real/main.rs
```

## isolation gate — governed (88)

```text
l1/agent_cwd.rs
l1/argv_normalization.rs
l1/characterization_error_routes.rs
l1/cli_process_fixture.rs
l1/command_routing.rs
l1/completion_contract.rs
l1/completion_perf.rs
l1/completion_resolution_round_trip.rs
l1/compose_caller_file_provenance.rs
l1/compose_cli.rs
l1/compose_frontmatter_model.rs
l1/compose_header_first.rs
l1/compose_initialize_acceptance.rs
l1/compose_initialize_staged_boot.rs
l1/compose_interactive_timeout_cli.rs
l1/compose_removed_validation_keys.rs
l1/compose_repository_context.rs
l1/compose_schema_cli.rs
l1/compose_system_prompt_lifetime.rs
l1/compose_ttff_perf.rs
l1/composition_outputs.rs
l1/contamination_probes.rs
l1/context_command.rs
l1/contextual_errors.rs
l1/ctx_launch_anchor.rs
l1/detached_audio.rs
l1/effective_diagnostic_render.rs
l1/errors_command.rs
l1/handle_blocking_output.rs
l1/handle_deadline.rs
l1/handle_repo_config.rs
l1/hooks_cli.rs
l1/inline_completion_lifecycle.rs
l1/inline_compose_cli.rs
l1/inline_compose_hash.rs
l1/inline_compose_sequence_mismatch.rs
l1/level1_compose_autocomplete_failure_pty.rs
l1/level1_dry_run_pty.rs
l1/level1_inline_compose_mismatch_pty.rs
l1/level1_provided_partial_file_pty.rs
l1/level1_provider_overlay_home.rs
l1/level1_pty_wrapper_summary.rs
l1/level1_review_router_partial_pty.rs
l1/level1_schema_prompt_pty.rs
l1/level1_structured_error_message.rs
l1/loop_cli.rs
l1/loop_initialize_state.rs
l1/mcp_cli.rs
l1/prompt_reporting.rs
l1/propagated_context_fixtures.rs
l1/protect_cli.rs
l1/provider_error_finalize.rs
l1/sequence_budget.rs
l1/sequence_cli.rs
l1/sequence_ctrl_c_windows.rs
l1/sequence_errors_cli.rs
l1/sequence_groups.rs
l1/sequence_initialize_include_preflight.rs
l1/sequence_jit.rs
l1/sequence_magic_reference.rs
l1/sequence_overlay_pty.rs
l1/sequence_perf.rs
l1/sequence_prompt_property.rs
l1/sequence_schema.rs
l1/sequence_sources_cli.rs
l1/shipped_prompt_contract.rs
l1/shipped_prompts.rs
l1/skills_integration.rs
l1/wrap_antigravity_exit_signal.rs
l1/wrap_basics.rs
l1/wrap_compose_agent.rs
l1/wrap_compose_exec.rs
l1/wrap_compose_preflight.rs
l1/wrap_compose_validation.rs
l1/wrap_ctrl_c_windows.rs
l1/wrap_direct_argv.rs
l1/wrap_incomplete_subagents.rs
l1/wrap_inline_compose.rs
l1/wrap_inline_compose_interactive.rs
l1/wrap_opencode.rs
l1/wrap_opencode_models.rs
l1/wrap_perf.rs
l1/wrap_provider_flags.rs
l1/wrap_sequence_composition.rs
l1/wrap_sigint.rs
l1/wrap_structured_stream.rs
l1/wrap_watchdog_startup_stall.rs
l1/wrap_watchdog_timeout.rs
```

## archive-path guard (152, repository-relative; `path<TAB>eligible`)

```text
claudine/cli/tests/common/completion.rs	true
claudine/cli/tests/common/host_tools.rs	true
claudine/cli/tests/common/incomplete_subagents.rs	true
claudine/cli/tests/common/mod.rs	true
claudine/cli/tests/common/pty.rs	true
claudine/cli/tests/common/review_router.rs	true
claudine/cli/tests/common/source_scan.rs	true
claudine/cli/tests/common/wrap.rs	true
claudine/cli/tests/l1/agent_cwd.rs	true
claudine/cli/tests/l1/argv_normalization.rs	true
claudine/cli/tests/l1/characterization_error_routes.rs	true
claudine/cli/tests/l1/cli_process_fixture.rs	true
claudine/cli/tests/l1/command_routing.rs	true
claudine/cli/tests/l1/completion_cli.rs	true
claudine/cli/tests/l1/completion_compose.rs	true
claudine/cli/tests/l1/completion_contract.rs	true
claudine/cli/tests/l1/completion_inline_compose.rs	true
claudine/cli/tests/l1/completion_perf.rs	true
claudine/cli/tests/l1/completion_resolution_round_trip.rs	true
claudine/cli/tests/l1/completion_sequence.rs	true
claudine/cli/tests/l1/completion_setter.rs	true
claudine/cli/tests/l1/compose_caller_file_provenance.rs	true
claudine/cli/tests/l1/compose_cli.rs	true
claudine/cli/tests/l1/compose_frontmatter_model.rs	true
claudine/cli/tests/l1/compose_header_first.rs	true
claudine/cli/tests/l1/compose_initialize_acceptance.rs	true
claudine/cli/tests/l1/compose_initialize_staged_boot.rs	true
claudine/cli/tests/l1/compose_interactive_timeout_cli.rs	true
claudine/cli/tests/l1/compose_removed_validation_keys.rs	true
claudine/cli/tests/l1/compose_repository_context.rs	true
claudine/cli/tests/l1/compose_schema_cli.rs	true
claudine/cli/tests/l1/compose_system_prompt_lifetime.rs	true
claudine/cli/tests/l1/compose_ttff_perf.rs	true
claudine/cli/tests/l1/composition_outputs.rs	true
claudine/cli/tests/l1/composition_seams.rs	true
claudine/cli/tests/l1/contamination_probes.rs	true
claudine/cli/tests/l1/context_command.rs	true
claudine/cli/tests/l1/contextual_errors.rs	true
claudine/cli/tests/l1/ctx_launch_anchor.rs	true
claudine/cli/tests/l1/detached_audio.rs	true
claudine/cli/tests/l1/diagnostic_discovery.rs	true
claudine/cli/tests/l1/dispatch_inventory.rs	true
claudine/cli/tests/l1/effective_diagnostic_render.rs	true
claudine/cli/tests/l1/error_guards/source_scan.rs	true
claudine/cli/tests/l1/error_guards.rs	true
claudine/cli/tests/l1/errors_command.rs	true
claudine/cli/tests/l1/handle_blocking_output.rs	true
claudine/cli/tests/l1/handle_deadline.rs	true
claudine/cli/tests/l1/handle_repo_config.rs	true
claudine/cli/tests/l1/hooks_cli.rs	true
claudine/cli/tests/l1/inline_completion_lifecycle.rs	true
claudine/cli/tests/l1/inline_compose_cli.rs	true
claudine/cli/tests/l1/inline_compose_hash.rs	true
claudine/cli/tests/l1/inline_compose_sequence_mismatch.rs	true
claudine/cli/tests/l1/level1_compose_autocomplete_failure_pty.rs	true
claudine/cli/tests/l1/level1_dry_run_pty.rs	true
claudine/cli/tests/l1/level1_inline_compose_mismatch_pty.rs	true
claudine/cli/tests/l1/level1_provided_partial_file_pty.rs	true
claudine/cli/tests/l1/level1_provider_overlay_home.rs	true
claudine/cli/tests/l1/level1_pty_wrapper_summary.rs	true
claudine/cli/tests/l1/level1_review_router_partial_pty.rs	true
claudine/cli/tests/l1/level1_schema_prompt_pty.rs	true
claudine/cli/tests/l1/level1_structured_error_message.rs	true
claudine/cli/tests/l1/loop_cli.rs	true
claudine/cli/tests/l1/loop_initialize_state.rs	true
claudine/cli/tests/l1/main.rs	true
claudine/cli/tests/l1/mcp_cli.rs	true
claudine/cli/tests/l1/prompt_reporting.rs	true
claudine/cli/tests/l1/propagated_context_fixtures.rs	true
claudine/cli/tests/l1/protect_cli.rs	true
claudine/cli/tests/l1/provider_error_finalize.rs	true
claudine/cli/tests/l1/run_harness_loop_call_sites.rs	true
claudine/cli/tests/l1/sequence_budget.rs	true
claudine/cli/tests/l1/sequence_cli.rs	true
claudine/cli/tests/l1/sequence_ctrl_c_windows.rs	true
claudine/cli/tests/l1/sequence_errors_cli.rs	true
claudine/cli/tests/l1/sequence_groups.rs	true
claudine/cli/tests/l1/sequence_initialize_include_preflight.rs	true
claudine/cli/tests/l1/sequence_jit.rs	true
claudine/cli/tests/l1/sequence_magic_reference.rs	true
claudine/cli/tests/l1/sequence_overlay_pty.rs	true
claudine/cli/tests/l1/sequence_perf.rs	true
claudine/cli/tests/l1/sequence_prompt_property.rs	true
claudine/cli/tests/l1/sequence_schema.rs	true
claudine/cli/tests/l1/sequence_sources_cli.rs	true
claudine/cli/tests/l1/shipped_prompt_contract.rs	true
claudine/cli/tests/l1/shipped_prompt_route_drift.rs	true
claudine/cli/tests/l1/shipped_prompts.rs	true
claudine/cli/tests/l1/skills_integration.rs	true
claudine/cli/tests/l1/spawn_site_guard.rs	true
claudine/cli/tests/l1/system_prompt_perf_bench.rs	true
claudine/cli/tests/l1/test_placement.rs	true
claudine/cli/tests/l1/wrap_antigravity_exit_signal.rs	true
claudine/cli/tests/l1/wrap_basics.rs	true
claudine/cli/tests/l1/wrap_compose_agent.rs	true
claudine/cli/tests/l1/wrap_compose_exec.rs	true
claudine/cli/tests/l1/wrap_compose_preflight.rs	true
claudine/cli/tests/l1/wrap_compose_validation.rs	true
claudine/cli/tests/l1/wrap_ctrl_c_windows.rs	true
claudine/cli/tests/l1/wrap_direct_argv.rs	true
claudine/cli/tests/l1/wrap_incomplete_subagents.rs	true
claudine/cli/tests/l1/wrap_inline_compose.rs	true
claudine/cli/tests/l1/wrap_inline_compose_interactive.rs	true
claudine/cli/tests/l1/wrap_opencode.rs	true
claudine/cli/tests/l1/wrap_opencode_models.rs	true
claudine/cli/tests/l1/wrap_perf.rs	true
claudine/cli/tests/l1/wrap_provider_flags.rs	true
claudine/cli/tests/l1/wrap_sequence_composition.rs	true
claudine/cli/tests/l1/wrap_sigint.rs	true
claudine/cli/tests/l1/wrap_structured_stream.rs	true
claudine/cli/tests/l1/wrap_watchdog_startup_stall.rs	true
claudine/cli/tests/l1/wrap_watchdog_timeout.rs	true
claudine/cli/tests/level2/level2_auto_complete_chooser.rs	true
claudine/cli/tests/level2/level2_auto_complete_operation_file.rs	true
claudine/cli/tests/level2/level2_context_capture.rs	true
claudine/cli/tests/level2/level2_dry_run_approval_capture.rs	true
claudine/cli/tests/level2/level2_dry_run_metadata_capture.rs	true
claudine/cli/tests/level2/level2_explicit_operation_file_miss.rs	true
claudine/cli/tests/level2/level2_file_resolution_capture.rs	true
claudine/cli/tests/level2/level2_incomplete_subagents_capture.rs	true
claudine/cli/tests/level2/level2_initialize_generated_transclusion.rs	true
claudine/cli/tests/level2/level2_inline_compose_mismatch_capture.rs	true
claudine/cli/tests/level2/level2_interrupt_feedback_capture.rs	true
claudine/cli/tests/level2/level2_invalid_file_reference_capture.rs	true
claudine/cli/tests/level2/level2_lifecycle_action_forms.rs	true
claudine/cli/tests/level2/level2_lifecycle_control.rs	true
claudine/cli/tests/level2/level2_lifecycle_dispatch.rs	true
claudine/cli/tests/level2/level2_lifecycle_loop.rs	true
claudine/cli/tests/level2/level2_malformed_frontmatter_capture.rs	true
claudine/cli/tests/level2/level2_perf_capture.rs	true
claudine/cli/tests/level2/level2_prompt_reporting_capture.rs	true
claudine/cli/tests/level2/level2_provided_partial_file_capture.rs	true
claudine/cli/tests/level2/level2_provider_overlay_capture.rs	true
claudine/cli/tests/level2/level2_removed_validation_key_capture.rs	true
claudine/cli/tests/level2/level2_schema_parse_capture.rs	true
claudine/cli/tests/level2/level2_sequence_task_stream_capture.rs	true
claudine/cli/tests/level2/level2_stalled_generation_capture.rs	true
claudine/cli/tests/level2/level2_typed_error_render_capture.rs	true
claudine/cli/tests/level2/level2_windows_provided_partial_file_capture.rs	true
claudine/cli/tests/level2/level2_wrap_ctrl_c_loop_wedge_tmux.rs	true
claudine/cli/tests/level2/level2_wrap_ctrl_c_tmux.rs	true
claudine/cli/tests/level2/main.rs	true
claudine/cli/tests/level3/level3_auto_complete_chooser.rs	true
claudine/cli/tests/level3/level3_linux_sequence_ctrl_c.rs	true
claudine/cli/tests/level3/level3_sequence_ctrl_c.rs	true
claudine/cli/tests/level3/level3_windows_sequence_ctrl_c.rs	true
claudine/cli/tests/level3/level3_wrap_ctrl_c.rs	true
claudine/cli/tests/level3/main.rs	true
claudine/cli/tests/real/main.rs	true
claudine/cli/tests/real/real_inline_write_grant.rs	true
claudine/cli/tests/real/real_opencode_yolo_subagent.rs	true
claudine/cli/tests/real/real_pi_steering.rs	true
```
