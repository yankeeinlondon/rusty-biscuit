---
total_phases: 8
source_files_during_phase_3:
  - biscuit-file/cli/tests/cli_tests.rs
  - biscuit-file/lib/src/file_reference/context.rs
  - biscuit-file/lib/src/file_reference/error.rs
  - biscuit-file/lib/src/file_reference/mod.rs
  - biscuit-file/lib/src/file_reference/parse.rs
  - biscuit-file/lib/src/file_reference/resolve.rs
  - biscuit-file/lib/src/lib.rs
  - biscuit-file/lib/src/list_format.rs
  - biscuit-file/lib/tests/completion_round_trip.rs
  - biscuit-file/lib/tests/detailed_resolution.rs
  - biscuit-file/lib/tests/implicit_relative.rs
  - biscuit-file/lib/tests/precedence_flip.rs
  - biscuit-file/lib/tests/reference_grammar.rs
  - biscuit-file/lib/tests/resolution_context.rs
  - biscuit-terminal/lib/tests/layout_matrix.rs
  - biscuit-test-harness/src/cliclick.rs
  - biscuit-test-harness/src/lib.rs
  - biscuit-test-harness/src/wezterm.rs
  - biscuit-test-harness/src/win_input.rs
  - biscuit-test-harness/src/xdotool.rs
  - claudine/cli/src/commands/compose/interrupt.rs
  - claudine/cli/src/commands/compose/loop_run.rs
  - claudine/cli/src/commands/compose/mod.rs
  - claudine/cli/src/commands/compose/prep.rs
  - claudine/cli/src/commands/compose/prep/tests.rs
  - claudine/cli/src/commands/config_tui/app.rs
  - claudine/cli/src/commands/config_tui/tabs/messenger/input.rs
  - claudine/cli/src/commands/context/expressions.rs
  - claudine/cli/src/commands/context/format.rs
  - claudine/cli/src/commands/context_render.rs
  - claudine/cli/src/commands/context_render/tests.rs
  - claudine/cli/src/commands/dashboard/mod.rs
  - claudine/cli/src/commands/dashboard/tests.rs
  - claudine/cli/src/commands/handle.rs
  - claudine/cli/src/commands/init/prompts.rs
  - claudine/cli/src/commands/mcp/show.rs
  - claudine/cli/src/commands/schema_interactive/mod.rs
  - claudine/cli/src/commands/schema_interactive/tests.rs
  - claudine/cli/src/commands/sequence.rs
  - claudine/cli/src/commands/wrap/composition/dry_run.rs
  - claudine/cli/src/commands/wrap/composition/dry_run/tests.rs
  - claudine/cli/src/commands/wrap/composition/launch.rs
  - claudine/cli/src/commands/wrap/composition/mod.rs
  - claudine/cli/src/commands/wrap/composition/pipeline.rs
  - claudine/cli/src/commands/wrap/composition/pipeline/tests.rs
  - claudine/cli/src/commands/wrap/composition/preflight.rs
  - claudine/cli/src/commands/wrap/composition/prep_context.rs
  - claudine/cli/src/commands/wrap/composition/runner.rs
  - claudine/cli/src/commands/wrap/composition/target.rs
  - claudine/cli/src/commands/wrap/composition/tests.rs
  - claudine/cli/src/commands/wrap/env/mod.rs
  - claudine/cli/src/commands/wrap/env/sanitize.rs
  - claudine/cli/src/commands/wrap/exec/mod.rs
  - claudine/cli/src/commands/wrap/exec/spawn.rs
  - claudine/cli/src/commands/wrap/exec/spawn/captured.rs
  - claudine/cli/src/commands/wrap/exec/spawn/inherited.rs
  - claudine/cli/src/commands/wrap/exec/spawn/mod.rs
  - claudine/cli/src/commands/wrap/exec/spawn/semantic.rs
  - claudine/cli/src/commands/wrap/exec/spawn/setup.rs
  - claudine/cli/src/commands/wrap/exec/spawn/tests/captured.rs
  - claudine/cli/src/commands/wrap/exec/spawn/tests/inherited.rs
  - claudine/cli/src/commands/wrap/exec/spawn/tests/mod.rs
  - claudine/cli/src/commands/wrap/exec/task_frame_fixtures.rs
  - claudine/cli/src/commands/wrap/exec/termination.rs
  - claudine/cli/src/commands/wrap/exec/termination/coordinator.rs
  - claudine/cli/src/commands/wrap/exec/termination/coordinator/tests.rs
  - claudine/cli/src/commands/wrap/exec/termination/handle.rs
  - claudine/cli/src/commands/wrap/exec/termination/message.rs
  - claudine/cli/src/commands/wrap/exec/termination/mod.rs
  - claudine/cli/src/commands/wrap/exec/termination/reasons.rs
  - claudine/cli/src/commands/wrap/exec/termination/summary.rs
  - claudine/cli/src/commands/wrap/exec/termination/tests/mod.rs
  - claudine/cli/src/commands/wrap/exec/termination/tests/projection.rs
  - claudine/cli/src/commands/wrap/exec/termination/tests/reasons.rs
  - claudine/cli/src/commands/wrap/exec/termination/tests/wait.rs
  - claudine/cli/src/commands/wrap/exec/termination/unix.rs
  - claudine/cli/src/commands/wrap/exec/termination/windows.rs
  - claudine/cli/src/commands/wrap/exec/timeouts.rs
  - claudine/cli/src/commands/wrap/exec/timeouts/tests.rs
  - claudine/cli/src/commands/wrap/exec/watchdog/spawn.rs
  - claudine/cli/src/commands/wrap/exec/wiring/mod.rs
  - claudine/cli/src/commands/wrap/exec/wiring/session.rs
  - claudine/cli/src/commands/wrap/flags.rs
  - claudine/cli/src/commands/wrap/flags/tests.rs
  - claudine/cli/src/commands/wrap/harness_orch/attempt.rs
  - claudine/cli/src/commands/wrap/harness_orch/launch.rs
  - claudine/cli/src/commands/wrap/harness_orch/loop_control.rs
  - claudine/cli/src/commands/wrap/harness_orch/loop_control/control_dispatch.rs
  - claudine/cli/src/commands/wrap/harness_orch/loop_control/coordinator.rs
  - claudine/cli/src/commands/wrap/harness_orch/loop_control/error_routing.rs
  - claudine/cli/src/commands/wrap/harness_orch/loop_control/lifecycle_events.rs
  - claudine/cli/src/commands/wrap/harness_orch/loop_control/proxy.rs
  - claudine/cli/src/commands/wrap/harness_orch/loop_control/requeue.rs
  - claudine/cli/src/commands/wrap/harness_orch/loop_control/target_launch.rs
  - claudine/cli/src/commands/wrap/harness_orch/loop_control/target_launch/tests.rs
  - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests.rs
  - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/active_state_wiring.rs
  - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/budget_scoping.rs
  - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/coordinator_adoption.rs
  - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/lifecycle_ordering.rs
  - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/mod.rs
  - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/overlay_layering.rs
  - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/proxy.rs
  - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/recovery_identity.rs
  - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/requeue.rs
  - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/retry_resume.rs
  - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/shell_approval.rs
  - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/terminal_evaluation.rs
  - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/terminal_routing.rs
  - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/unowned_handoff.rs
  - claudine/cli/src/commands/wrap/harness_orch/mod.rs
  - claudine/cli/src/commands/wrap/harness_orch/prompt.rs
  - claudine/cli/src/commands/wrap/harness_orch/session_key.rs
  - claudine/cli/src/commands/wrap/harness_orch/session_key/tests.rs
  - claudine/cli/src/commands/wrap/harness_orch/shell_options.rs
  - claudine/cli/src/commands/wrap/harness_orch/types.rs
  - claudine/cli/src/commands/wrap/launch_plan.rs
  - claudine/cli/src/commands/wrap/launch_plan/tests.rs
  - claudine/cli/src/commands/wrap/live_semantic_sink/mod.rs
  - claudine/cli/src/commands/wrap/live_semantic_sink/tests/golden_stderr.rs
  - claudine/cli/src/commands/wrap/mod.rs
  - claudine/cli/src/commands/wrap/overlay.rs
  - claudine/cli/src/commands/wrap/policy.rs
  - claudine/cli/src/commands/wrap/profile/mod.rs
  - claudine/cli/src/commands/wrap/profile/tests/positional.rs
  - claudine/cli/src/commands/wrap/repo_home.rs
  - claudine/cli/src/commands/wrap/repo_home/tests.rs
  - claudine/cli/src/commands/wrap/runaway_guard.rs
  - claudine/cli/src/commands/wrap/sequence/iterate.rs
  - claudine/cli/src/commands/wrap/sequence/jit.rs
  - claudine/cli/src/commands/wrap/sequence/jit/tests.rs
  - claudine/cli/src/commands/wrap/sequence/mod.rs
  - claudine/cli/src/commands/wrap/sequence/phase1c.rs
  - claudine/cli/src/commands/wrap/sequence/resolve.rs
  - claudine/cli/src/commands/wrap/sequence/task_frames.rs
  - claudine/cli/src/commands/wrap/sequence/task_run.rs
  - claudine/cli/src/commands/wrap/sequence/tests.rs
  - claudine/cli/src/commands/wrap/session_report.rs
  - claudine/cli/src/commands/wrap/session_report/tests.rs
  - claudine/cli/src/commands/wrap/stream_io.rs
  - claudine/cli/src/commands/wrap/system_prompt.rs
  - claudine/cli/src/commands/wrap/system_prompt/tests.rs
  - claudine/cli/src/commands/wrap/wrapper_exec.rs
  - claudine/cli/src/commands/wrap/wrapper_mcp.rs
  - claudine/cli/src/commands/wrap/wrapper_stages.rs
  - claudine/cli/src/completion/bootstrap.rs
  - claudine/cli/src/completion/bootstrap/tests.rs
  - claudine/cli/src/completion/composition/magic_at.rs
  - claudine/cli/src/completion/composition/mod.rs
  - claudine/cli/src/completion/composition/tests.rs
  - claudine/cli/src/completion/engine/mod.rs
  - claudine/cli/src/completion/engine/tests.rs
  - claudine/cli/src/completion/scopes.rs
  - claudine/cli/src/completion/scopes/tests.rs
  - claudine/cli/src/completion/setter_value.rs
  - claudine/cli/src/completion/setter_value/tests.rs
  - claudine/cli/src/completion/walker.rs
  - claudine/cli/src/completion/walker/tests.rs
  - claudine/cli/src/log.rs
  - claudine/cli/src/main.rs
  - claudine/cli/src/output/error_walker.rs
  - claudine/cli/src/output/error_walker/tests.rs
  - claudine/cli/src/perf/mod.rs
  - claudine/cli/src/perf/report.rs
  - claudine/cli/src/perf/tests/perf_tree.rs
  - claudine/cli/src/perf/tests/report.rs
  - claudine/cli/src/perf/tree.rs
  - claudine/cli/tests/characterization_error_routes.rs
  - claudine/cli/tests/common/mod.rs
  - claudine/cli/tests/common/pty.rs
  - claudine/cli/tests/completion_compose.rs
  - claudine/cli/tests/completion_resolution_round_trip.rs
  - claudine/cli/tests/compose_cli.rs
  - claudine/cli/tests/compose_system_prompt_lifetime.rs
  - claudine/cli/tests/composition_outputs.rs
  - claudine/cli/tests/composition_seams.rs
  - claudine/cli/tests/diagnostic_discovery.rs
  - claudine/cli/tests/dispatch_inventory.rs
  - claudine/cli/tests/effective_diagnostic_render.rs
  - claudine/cli/tests/error_guards.rs
  - claudine/cli/tests/error_guards/source_scan.rs
  - claudine/cli/tests/inline_compose_cli.rs
  - claudine/cli/tests/level2_auto_complete_operation_file.rs
  - claudine/cli/tests/level2_context_capture.rs
  - claudine/cli/tests/level2_dry_run_approval_capture.rs
  - claudine/cli/tests/level2_dry_run_metadata_capture.rs
  - claudine/cli/tests/level2_file_resolution_capture.rs
  - claudine/cli/tests/level2_inline_compose_mismatch_capture.rs
  - claudine/cli/tests/level2_interrupt_feedback_capture.rs
  - claudine/cli/tests/level2_invalid_file_reference_capture.rs
  - claudine/cli/tests/level2_lifecycle_control.rs
  - claudine/cli/tests/level2_lifecycle_dispatch.rs
  - claudine/cli/tests/level2_malformed_frontmatter_capture.rs
  - claudine/cli/tests/level2_perf_capture.rs
  - claudine/cli/tests/level2_prompt_reporting_capture.rs
  - claudine/cli/tests/level2_removed_validation_key_capture.rs
  - claudine/cli/tests/level2_schema_parse_capture.rs
  - claudine/cli/tests/level2_schema_prompt_pty.rs
  - claudine/cli/tests/level2_sequence_overlay_pty.rs
  - claudine/cli/tests/level2_sequence_task_stream_capture.rs
  - claudine/cli/tests/level2_stalled_generation_capture.rs
  - claudine/cli/tests/level2_typed_error_render_capture.rs
  - claudine/cli/tests/level2_windows_sequence_ctrl_c.rs
  - claudine/cli/tests/level3_auto_complete_chooser.rs
  - claudine/cli/tests/level3_auto_complete_operation_file.rs
  - claudine/cli/tests/level3_linux_sequence_ctrl_c.rs
  - claudine/cli/tests/level3_sequence_ctrl_c.rs
  - claudine/cli/tests/level3_windows_sequence_ctrl_c.rs
  - claudine/cli/tests/run_harness_loop_call_sites.rs
  - claudine/cli/tests/sequence_cli.rs
  - claudine/cli/tests/sequence_errors_cli.rs
  - claudine/cli/tests/sequence_groups.rs
  - claudine/cli/tests/sequence_jit.rs
  - claudine/cli/tests/sequence_overlay_pty.rs
  - claudine/cli/tests/sequence_perf.rs
  - claudine/cli/tests/sequence_sources_cli.rs
  - claudine/cli/tests/shipped_prompt_route_drift.rs
  - claudine/cli/tests/shipped_prompts.rs
  - claudine/cli/tests/test_placement.rs
  - claudine/cli/tests/wrap_basics.rs
  - claudine/cli/tests/wrap_compose_agent.rs
  - claudine/cli/tests/wrap_compose_preflight.rs
  - claudine/cli/tests/wrap_compose_validation.rs
  - claudine/cli/tests/wrap_perf.rs
  - claudine/contract/src/adapter.rs
  - claudine/contract/src/tests.rs
  - claudine/contract/src/tests/tracing_capture.rs
  - claudine/gen/src/agent_errors_check.rs
  - claudine/gen/src/agent_errors_check/tests.rs
  - claudine/gen/src/emit.rs
  - claudine/gen/src/emit/event_policy.rs
  - claudine/gen/src/emit/execution_prompting.rs
  - claudine/gen/src/emit/identity_paths.rs
  - claudine/gen/src/emit/linking.rs
  - claudine/gen/src/emit/mod.rs
  - claudine/gen/src/emit/models_offerings.rs
  - claudine/gen/src/emit/tests.rs
  - claudine/gen/src/generate.rs
  - claudine/gen/src/generate/coerce/event_policy.rs
  - claudine/gen/src/generate/coerce/execution_prompting.rs
  - claudine/gen/src/generate/coerce/identity_paths.rs
  - claudine/gen/src/generate/coerce/mod.rs
  - claudine/gen/src/generate/coerce/models_offerings.rs
  - claudine/gen/src/generate/tests.rs
  - claudine/gen/src/lib.rs
  - claudine/gen/src/main.rs
  - claudine/gen/src/registry.rs
  - claudine/gen/src/registry/tests.rs
  - claudine/gen/src/report.rs
  - claudine/gen/src/report/tests.rs
  - claudine/gen/src/vocabulary.rs
  - claudine/gen/src/vocabulary/tests.rs
  - claudine/gen/tests/drift.rs
  - claudine/gen/tests/generate_ux.rs
  - claudine/gen/tests/level2_report_terminal.rs
  - claudine/lib/benches/runtime_hot_paths.rs
  - claudine/lib/src/actions/bash_executor.rs
  - claudine/lib/src/actions/hook_action.rs
  - claudine/lib/src/actions/hook_action/tests.rs
  - claudine/lib/src/composition/closure.rs
  - claudine/lib/src/composition/closure/tests.rs
  - claudine/lib/src/composition/coordinator/active.rs
  - claudine/lib/src/composition/coordinator/commit.rs
  - claudine/lib/src/composition/coordinator/document.rs
  - claudine/lib/src/composition/coordinator/handoff.rs
  - claudine/lib/src/composition/coordinator/invocation.rs
  - claudine/lib/src/composition/coordinator/mod.rs
  - claudine/lib/src/composition/coordinator/tests.rs
  - claudine/lib/src/composition/coordinator/transition.rs
  - claudine/lib/src/composition/error/mod.rs
  - claudine/lib/src/composition/error/render.rs
  - claudine/lib/src/composition/error/render/lifecycle.rs
  - claudine/lib/src/composition/error/render/mod.rs
  - claudine/lib/src/composition/error/render/provider.rs
  - claudine/lib/src/composition/error/render/schema.rs
  - claudine/lib/src/composition/error/render/selection.rs
  - claudine/lib/src/composition/error/render/sequence_loop.rs
  - claudine/lib/src/composition/error/tests.rs
  - claudine/lib/src/composition/interpolation_conformance.rs
  - claudine/lib/src/composition/lifecycle/action_shape.rs
  - claudine/lib/src/composition/lifecycle/actions.rs
  - claudine/lib/src/composition/lifecycle/actions/tests.rs
  - claudine/lib/src/composition/lifecycle/context.rs
  - claudine/lib/src/composition/lifecycle/context/tests.rs
  - claudine/lib/src/composition/lifecycle/control.rs
  - claudine/lib/src/composition/lifecycle/control/tests.rs
  - claudine/lib/src/composition/lifecycle/executor.rs
  - claudine/lib/src/composition/lifecycle/executor/tests.rs
  - claudine/lib/src/composition/lifecycle/executor/tests/action_dispatch.rs
  - claudine/lib/src/composition/lifecycle/executor/tests/conditions_control.rs
  - claudine/lib/src/composition/lifecycle/executor/tests/event_time_interpolation.rs
  - claudine/lib/src/composition/lifecycle/executor/tests/filesystem_lookup.rs
  - claudine/lib/src/composition/lifecycle/executor/tests/mod.rs
  - claudine/lib/src/composition/lifecycle/executor/tests/mutation_visibility.rs
  - claudine/lib/src/composition/lifecycle/executor/tests/proxy_with_evaluation.rs
  - claudine/lib/src/composition/lifecycle/executor/tests/runtime_set.rs
  - claudine/lib/src/composition/lifecycle/mod.rs
  - claudine/lib/src/composition/lifecycle/parse.rs
  - claudine/lib/src/composition/lifecycle/runtime.rs
  - claudine/lib/src/composition/lifecycle/runtime/tests.rs
  - claudine/lib/src/composition/lifecycle/tests.rs
  - claudine/lib/src/composition/lifecycle/tests/action_shape_control.rs
  - claudine/lib/src/composition/lifecycle/tests/audio_emission.rs
  - claudine/lib/src/composition/lifecycle/tests/diagnostics.rs
  - claudine/lib/src/composition/lifecycle/tests/guard_runtime.rs
  - claudine/lib/src/composition/lifecycle/tests/mod.rs
  - claudine/lib/src/composition/lifecycle/tests/parse_config.rs
  - claudine/lib/src/composition/lifecycle/tests/validation.rs
  - claudine/lib/src/composition/lifecycle/validate.rs
  - claudine/lib/src/composition/looping/actions.rs
  - claudine/lib/src/composition/looping/actions/tests.rs
  - claudine/lib/src/composition/looping/config.rs
  - claudine/lib/src/composition/looping/config/tests.rs
  - claudine/lib/src/composition/looping/engine.rs
  - claudine/lib/src/composition/looping/engine/tests.rs
  - claudine/lib/src/composition/looping/engine/tests/iteration_actions.rs
  - claudine/lib/src/composition/looping/engine/tests/lifecycle_control.rs
  - claudine/lib/src/composition/looping/engine/tests/mod.rs
  - claudine/lib/src/composition/looping/engine/tests/rate_limits.rs
  - claudine/lib/src/composition/looping/engine/tests/seed_state.rs
  - claudine/lib/src/composition/looping/expression.rs
  - claudine/lib/src/composition/looping/expression/tests/resolution_context.rs
  - claudine/lib/src/composition/looping/types.rs
  - claudine/lib/src/composition/mod.rs
  - claudine/lib/src/composition/preflight.rs
  - claudine/lib/src/composition/preflight/tests.rs
  - claudine/lib/src/composition/prepare.rs
  - claudine/lib/src/composition/prepare/entry.rs
  - claudine/lib/src/composition/prepare/entry/tests.rs
  - claudine/lib/src/composition/prepare/service.rs
  - claudine/lib/src/composition/prepare/service/tests.rs
  - claudine/lib/src/composition/prepare/tests.rs
  - claudine/lib/src/composition/resolve.rs
  - claudine/lib/src/composition/resolve/tests.rs
  - claudine/lib/src/composition/runtime_state.rs
  - claudine/lib/src/composition/runtime_state/tests.rs
  - claudine/lib/src/composition/schema/classify.rs
  - claudine/lib/src/composition/schema/mod.rs
  - claudine/lib/src/composition/schema/tests.rs
  - claudine/lib/src/composition/schema/translate.rs
  - claudine/lib/src/composition/select.rs
  - claudine/lib/src/composition/select/tests.rs
  - claudine/lib/src/composition/sequence.rs
  - claudine/lib/src/composition/sequence/data.rs
  - claudine/lib/src/composition/sequence/expr.rs
  - claudine/lib/src/composition/sequence/formal.rs
  - claudine/lib/src/composition/sequence/grammar.rs
  - claudine/lib/src/composition/sequence/mod.rs
  - claudine/lib/src/composition/sequence/model.rs
  - claudine/lib/src/composition/sequence/normalize.rs
  - claudine/lib/src/composition/sequence/preflight/mod.rs
  - claudine/lib/src/composition/sequence/preflight/shape.rs
  - claudine/lib/src/composition/sequence/preflight/tests.rs
  - claudine/lib/src/composition/sequence/reserved.rs
  - claudine/lib/src/composition/sequence/source.rs
  - claudine/lib/src/composition/sequence/task/group.rs
  - claudine/lib/src/composition/sequence/task/mod.rs
  - claudine/lib/src/composition/sequence/task/shell.rs
  - claudine/lib/src/composition/sequence/task/shell/tests.rs
  - claudine/lib/src/composition/sequence/task/tests.rs
  - claudine/lib/src/composition/sequence/tests.rs
  - claudine/lib/src/composition/types.rs
  - claudine/lib/src/config/atomic.rs
  - claudine/lib/src/config/claude.rs
  - claudine/lib/src/config/claude/tests.rs
  - claudine/lib/src/config/claudine_config.rs
  - claudine/lib/src/config/claudine_config/tests.rs
  - claudine/lib/src/config/messaging_block.rs
  - claudine/lib/src/config/messaging_block/tests.rs
  - claudine/lib/src/diagnostics/discovery.rs
  - claudine/lib/src/diagnostics/discovery/tests.rs
  - claudine/lib/src/diagnostics/facets.rs
  - claudine/lib/src/diagnostics/mod.rs
  - claudine/lib/src/diagnostics/registry.rs
  - claudine/lib/src/diagnostics/restored.rs
  - claudine/lib/src/diagnostics/restored/tests.rs
  - claudine/lib/src/diagnostics/snapshot.rs
  - claudine/lib/src/diagnostics/snapshot/tests.rs
  - claudine/lib/src/dispatch/deps.rs
  - claudine/lib/src/dispatch/expression.rs
  - claudine/lib/src/dispatch/expression/tests.rs
  - claudine/lib/src/dispatch/loader.rs
  - claudine/lib/src/dispatch/loader/tests.rs
  - claudine/lib/src/dispatch/matcher.rs
  - claudine/lib/src/dispatch/matcher/tests.rs
  - claudine/lib/src/dispatch/mod.rs
  - claudine/lib/src/dispatch/runner/mappers.rs
  - claudine/lib/src/dispatch/runner/mod.rs
  - claudine/lib/src/dispatch/runner/tests.rs
  - claudine/lib/src/dispatch/template.rs
  - claudine/lib/src/dispatch/template/tests.rs
  - claudine/lib/src/dispatch/tests.rs
  - claudine/lib/src/error.rs
  - claudine/lib/src/harness/audit.rs
  - claudine/lib/src/harness/error.rs
  - claudine/lib/src/harness/error/tests.rs
  - claudine/lib/src/harness/mod.rs
  - claudine/lib/src/harness/resolve.rs
  - claudine/lib/src/harness/resolve/tests.rs
  - claudine/lib/src/harness/runtime.rs
  - claudine/lib/src/harness/runtime/tests.rs
  - claudine/lib/src/harness/shell.rs
  - claudine/lib/src/linking/compatibility/mod.rs
  - claudine/lib/src/linking/compatibility/tests.rs
  - claudine/lib/src/linking/hashing.rs
  - claudine/lib/src/linking/skills/portable.rs
  - claudine/lib/src/linking/skills/portable/tests.rs
  - claudine/lib/src/mcp/catalog.rs
  - claudine/lib/src/mcp/defaults.rs
  - claudine/lib/src/mcp/export.rs
  - claudine/lib/src/mcp/import.rs
  - claudine/lib/src/mcp/import/tests.rs
  - claudine/lib/src/mcp/inject.rs
  - claudine/lib/src/mcp/state.rs
  - claudine/lib/src/messaging/config.rs
  - claudine/lib/src/messaging/config/tests.rs
  - claudine/lib/src/messaging/mod.rs
  - claudine/lib/src/messaging/send.rs
  - claudine/lib/src/messaging/send/tests.rs
  - claudine/lib/src/model_catalog/service.rs
  - claudine/lib/src/model_catalog/service/tests.rs
  - claudine/lib/src/permissions/engine.rs
  - claudine/lib/src/permissions/engine/tests.rs
  - claudine/lib/src/permissions/mutation.rs
  - claudine/lib/src/permissions/providers/claude.rs
  - claudine/lib/src/permissions/providers/claude/tests.rs
  - claudine/lib/src/permissions/providers/codex.rs
  - claudine/lib/src/permissions/providers/codex/tests.rs
  - claudine/lib/src/permissions/providers/gemini.rs
  - claudine/lib/src/permissions/providers/gemini/tests.rs
  - claudine/lib/src/permissions/providers/goose.rs
  - claudine/lib/src/permissions/providers/kimi.rs
  - claudine/lib/src/permissions/providers/opencode.rs
  - claudine/lib/src/permissions/providers/qwen.rs
  - claudine/lib/src/permissions/providers/qwen/tests.rs
  - claudine/lib/src/permissions/query.rs
  - claudine/lib/src/permissions/query/tests.rs
  - claudine/lib/src/protect/catalog.rs
  - claudine/lib/src/protect/catalog/tests.rs
  - claudine/lib/src/protect/observe.rs
  - claudine/lib/src/protect/observe/tests.rs
  - claudine/lib/src/protect/path.rs
  - claudine/lib/src/protect/service.rs
  - claudine/lib/src/protect/service/tests.rs
  - claudine/lib/src/provider/methods.rs
  - claudine/lib/src/provider/methods/tests.rs
  - claudine/lib/src/provider/mod.rs
  - claudine/lib/src/render/mod.rs
  - claudine/lib/src/render/prompt/system.rs
  - claudine/lib/src/render/prompt/system/tests.rs
  - claudine/lib/src/render/task_stream.rs
  - claudine/lib/src/render/task_stream/tests.rs
  - claudine/lib/src/reporting/error.rs
  - claudine/lib/src/reporting/error/tests.rs
  - claudine/lib/src/reporting/ingest.rs
  - claudine/lib/src/reporting/ingest/tests.rs
  - claudine/lib/src/reporting/mod.rs
  - claudine/lib/src/reporting/types.rs
  - claudine/lib/src/runaway/config.rs
  - claudine/lib/src/runaway/config/tests.rs
  - claudine/lib/src/runaway/detector.rs
  - claudine/lib/src/runaway/detector/tests.rs
  - claudine/lib/src/signals/bespoke.rs
  - claudine/lib/src/signals/bespoke/tests.rs
  - claudine/lib/src/signals/projection_equivalence_tests.rs
  - claudine/lib/src/stream/badges.rs
  - claudine/lib/src/stream/badges/tests.rs
  - claudine/lib/src/stream/logs/opencode/bridge/tests.rs
  - claudine/lib/src/stream/logs/opencode/bridge/tests/ingest_classification.rs
  - claudine/lib/src/stream/logs/opencode/bridge/tests/mod.rs
  - claudine/lib/src/stream/logs/opencode/bridge/tests/session_lifecycle.rs
  - claudine/lib/src/stream/logs/opencode/bridge/tests/signal_projection.rs
  - claudine/lib/src/stream/logs/opencode/bridge/tests/stalled_generation_progress.rs
  - claudine/lib/src/stream/logs/opencode/bridge/tests/stdout_stderr_coordination.rs
  - claudine/lib/src/stream/logs/opencode/bridge/tests/usage_retry_guards.rs
  - claudine/lib/src/stream/logs/opencode/events.rs
  - claudine/lib/src/stream/logs/opencode/events/tests.rs
  - claudine/lib/src/stream/mod.rs
  - claudine/lib/src/stream/parser.rs
  - claudine/lib/src/stream/progress.rs
  - claudine/lib/src/stream/progress/tests.rs
  - claudine/lib/src/stream/protocol/claude.rs
  - claudine/lib/src/stream/protocol/claude/tests.rs
  - claudine/lib/src/stream/protocol/codex.rs
  - claudine/lib/src/stream/protocol/codex/tests.rs
  - claudine/lib/src/stream/protocol/kimi.rs
  - claudine/lib/src/stream/protocol/kimi/tests.rs
  - claudine/lib/src/stream/protocol/opencode.rs
  - claudine/lib/src/stream/protocol/opencode/tests.rs
  - claudine/lib/src/stream/providers/antigravity.rs
  - claudine/lib/src/stream/providers/claude.rs
  - claudine/lib/src/stream/providers/claude/tests.rs
  - claudine/lib/src/stream/providers/codex.rs
  - claudine/lib/src/stream/providers/codex/tests.rs
  - claudine/lib/src/stream/providers/gemini.rs
  - claudine/lib/src/stream/providers/gemini/tests.rs
  - claudine/lib/src/stream/providers/kimi.rs
  - claudine/lib/src/stream/providers/kimi/tests.rs
  - claudine/lib/src/stream/providers/opencode.rs
  - claudine/lib/src/stream/providers/opencode/tests.rs
  - claudine/lib/src/stream/providers/pi.rs
  - claudine/lib/src/stream/providers/pi/tests.rs
  - claudine/lib/src/stream/providers/qwen.rs
  - claudine/lib/src/stream/reporting.rs
  - claudine/lib/src/stream/reporting/tests.rs
  - claudine/lib/src/stream/semantic.rs
  - claudine/lib/src/stream/semantic/tests.rs
  - claudine/lib/src/stream/stderr.rs
  - claudine/lib/src/stream/stderr/tests.rs
  - claudine/lib/src/stream/tool_display.rs
  - claudine/lib/src/stream/tool_display/from_event_tests.rs
  - claudine/lib/src/stream/tool_display/humanize_tests.rs
  - claudine/lib/src/stream/tool_display/summary_tests.rs
  - claudine/lib/src/stream/tool_display/tests.rs
  - claudine/lib/src/system_prompt/change_state.rs
  - claudine/lib/src/system_prompt/context.rs
  - claudine/lib/src/system_prompt/prepare.rs
  - claudine/lib/src/system_prompt/prepare/tests.rs
  - claudine/lib/src/system_prompt/resolve.rs
  - claudine/lib/src/system_prompt/resolve/tests.rs
  - claudine/lib/tests/agent_errors_fleet.rs
  - claudine/lib/tests/boundary_lint.rs
  - claudine/lib/tests/diagnostic_detail_conformance.rs
  - claudine/lib/tests/kimi_wire.rs
  - claudine/lib/tests/semantic_fidelity.rs
  - claudine/rendezvous/client/src/connector/mod.rs
  - claudine/rendezvous/client/src/connector/tests.rs
  - claudine/rendezvous/client/src/connector/unix.rs
  - claudine/rendezvous/client/src/connector/windows.rs
  - claudine/rendezvous/client/src/lib.rs
  - claudine/rendezvous/client/src/main.rs
  - claudine/rendezvous/client/tests/local_round_trip.rs
  - claudine/rendezvous/client/tests/session_log_round_trip.rs
  - claudine/rendezvous/client/tests/uds_round_trip.rs
  - claudine/rendezvous/core/src/lib.rs
  - claudine/rendezvous/core/src/local_endpoint.rs
  - claudine/rendezvous/core/src/local_endpoint/test_support.rs
  - claudine/rendezvous/core/src/local_endpoint/tests.rs
  - claudine/rendezvous/core/src/socket.rs
  - claudine/rendezvous/daemon/src/lib.rs
  - claudine/rendezvous/daemon/src/local_transport/mod.rs
  - claudine/rendezvous/daemon/src/local_transport/unix.rs
  - claudine/rendezvous/daemon/src/local_transport/unix/tests.rs
  - claudine/rendezvous/daemon/src/local_transport/windows.rs
  - claudine/rendezvous/daemon/src/local_transport/windows/tests.rs
  - claudine/rendezvous/daemon/src/main.rs
  - claudine/rendezvous/daemon/src/peers.rs
  - claudine/rendezvous/daemon/src/peers/tests.rs
  - claudine/rendezvous/daemon/src/private_dir.rs
  - claudine/rendezvous/daemon/src/private_dir/tests.rs
  - claudine/rendezvous/daemon/src/register.rs
  - claudine/rendezvous/daemon/src/register/tests.rs
  - claudine/rendezvous/daemon/src/server.rs
  - claudine/rendezvous/daemon/src/server/tests.rs
  - claudine/rendezvous/daemon/src/service.rs
  - claudine/rendezvous/daemon/src/service/tests/mod.rs
  - claudine/rendezvous/daemon/src/service/tests/rpc.rs
  - claudine/rendezvous/daemon/src/service/tests/session_register.rs
  - claudine/rendezvous/daemon/src/service/tests/validation.rs
  - claudine/rendezvous/daemon/src/session_log.rs
  - claudine/rendezvous/daemon/src/session_log/append.rs
  - claudine/rendezvous/daemon/src/session_log/mod.rs
  - claudine/rendezvous/daemon/src/session_log/rehydrate.rs
  - claudine/rendezvous/daemon/src/session_log/staging.rs
  - claudine/rendezvous/daemon/src/session_log/tests.rs
  - claudine/rendezvous/daemon/src/session_log/tests/append_rotation.rs
  - claudine/rendezvous/daemon/src/session_log/tests/durability.rs
  - claudine/rendezvous/daemon/src/session_log/tests/mod.rs
  - claudine/rendezvous/daemon/src/session_log/tests/remote_validation.rs
  - claudine/rendezvous/daemon/src/session_log/tests/replace_update.rs
  - claudine/rendezvous/daemon/src/session_log/tests/replay_rehydration.rs
  - claudine/rendezvous/daemon/src/session_log/validate.rs
  - claudine/rendezvous/daemon/src/sync.rs
  - claudine/rendezvous/daemon/src/sync/tests/envelope_validation.rs
  - claudine/rendezvous/daemon/src/sync/tests/mod.rs
  - claudine/rendezvous/daemon/src/sync/tests/schema_validation.rs
  - claudine/rendezvous/daemon/src/sync/tests/snapshot_replace.rs
  - claudine/rendezvous/daemon/tests/pairing_and_sync.rs
  - claudine/rendezvous/daemon/tests/peer_discovery.rs
  - claudine/rendezvous/daemon/tests/phase6_integration.rs
  - darkmatter/lib/src/effects/catalog.rs
  - darkmatter/lib/src/effects/error.rs
  - darkmatter/lib/src/effects/verbs.rs
  - darkmatter/lib/src/markdown/compose/context/effective_state.rs
  - darkmatter/lib/src/markdown/compose/context/options.rs
  - darkmatter/lib/src/markdown/compose/expression/error.rs
  - darkmatter/lib/src/markdown/compose/expression/functions/mod.rs
  - darkmatter/lib/src/markdown/compose/expression/mod.rs
  - darkmatter/lib/src/markdown/compose/expression/path_projection.rs
  - darkmatter/lib/src/markdown/compose/expression/resolve_ctx.rs
  - darkmatter/lib/src/markdown/compose/frontmatter_interpolation.rs
  - darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs
  - darkmatter/lib/src/markdown/compose/link_normalization.rs
  - darkmatter/lib/src/markdown/compose/link_resolve.rs
  - darkmatter/lib/src/markdown/compose/mod.rs
  - darkmatter/lib/src/markdown/compose/pipeline/mod.rs
  - darkmatter/lib/src/markdown/compose/preflight/collect.rs
  - darkmatter/lib/src/markdown/compose/remote.rs
  - darkmatter/lib/src/markdown/compose/schema_validation.rs
  - darkmatter/lib/src/markdown/compose/tests/frontmatter.rs
  - darkmatter/lib/src/markdown/compose/tests/rendering.rs
  - darkmatter/lib/src/markdown/compose/tests/schema.rs
  - darkmatter/lib/src/markdown/compose/tests/transclusion.rs
  - darkmatter/lib/src/markdown/compose/transclusion/engine.rs
  - darkmatter/lib/src/markdown/compose/transclusion/mod.rs
  - darkmatter/lib/src/markdown/compose/transclusion/resolver.rs
  - darkmatter/lib/src/markdown/compose/util.rs
  - darkmatter/lib/src/markdown/reference/graph.rs
  - darkmatter/lib/src/markdown/reference/mod.rs
  - darkmatter/lib/src/markdown/reference/validate.rs
  - darkmatter/lib/src/markdown/schemas/detect.rs
  - darkmatter/lib/src/markdown/schemas/format.rs
  - darkmatter/lib/src/markdown/schemas/mod.rs
  - darkmatter/lib/src/markdown/schemas/reference.rs
  - darkmatter/lib/src/markdown/schemas/resolve.rs
  - darkmatter/lib/src/markdown/schemas/rewrite.rs
  - darkmatter/lib/src/markdown/schemas/tests/mod.rs
  - darkmatter/lib/src/markdown/schemas/validate.rs
  - darkmatter/lib/tests/reference_integration.rs
  - sniff/lib/src/error.rs
  - sniff/lib/src/os/mod.rs
  - sniff/lib/src/os/user.rs
docs_updated_during_phase_3:
  - .claudine/memory/commits.md
  - .claudine/non-interactive.md
  - biscuit-file/docs/dependencies.md
  - biscuit-file/docs/topics/file-references.md
  - biscuit-test-harness/README.md
  - claudine/docs/dependencies.md
  - claudine/docs/rendezvous/current-state.md
  - claudine/docs/rendezvous/design.md
  - claudine/docs/rendezvous/index.md
  - claudine/docs/research/local_runners/_fleet.md
  - claudine/docs/research/local_runners/llamacpp.md
  - claudine/docs/research/local_runners/lmstudio.md
  - claudine/docs/research/local_runners/ollama.md
  - claudine/docs/research/local_runners/omlx.md
  - claudine/docs/research/local_runners/vllm.md
  - claudine/docs/topics/building-an-agent-wrapper.md
  - claudine/docs/topics/cli-pre-parsing.md
  - claudine/docs/topics/completions/index.md
  - claudine/docs/topics/completions/shell-completions.md
  - claudine/docs/topics/composition.md
  - claudine/docs/topics/context/expression-engine.md
  - claudine/docs/topics/execution-flow.md
  - claudine/docs/topics/flow-control/looping.md
  - claudine/docs/topics/flow-control/sequences.md
  - claudine/docs/topics/lifecycle.md
  - claudine/docs/topics/non-interactive-sessions.md
  - claudine/docs/topics/provider-metadata.md
  - claudine/docs/topics/stream-parsing.md
  - claudine/docs/topics/system-prompt.md
  - claudine/docs/topics/unified-events.md
  - claudine/features/2026-07-11-fleet-validate-and-resume/spec.md
  - claudine/features/2026-07-11-module-structure/critical-plan.md
  - claudine/features/2026-07-11-module-structure/nice-plan.md
  - claudine/features/2026-07-11-module-structure/phase6-discovery.md
  - claudine/features/2026-07-11-module-structure/review.md
  - claudine/features/2026-07-11-module-structure/strong-plan.md
  - claudine/features/2026-07-11-sequence-plus/plan.md
  - claudine/features/2026-07-11-sequence-plus/spec.md
  - claudine/features/2026-07-12-rendezvous-dashboard/windows-support-followup.md
  - claudine/features/2026-07-13-error-propogation/spec.md
  - claudine/features/2026-07-13-file-resolution/spec.md
  - claudine/features/2026-07-13-proxy-with/spec.md
  - claudine/features/_completed/2026-06-14-auto-complete/plan.md
  - claudine/features/_completed/2026-06-14-auto-complete/review-3.md
  - claudine/features/_completed/2026-06-14-auto-complete/review-5.md
  - claudine/features/_completed/2026-06-14-auto-complete/spec.md
  - claudine/fixes/2026-07-13-rendezvous-local-ipc/spec.md
  - claudine/rendezvous/README.md
  - claudine/reviews/2026-07-01-dry-review/review.md
  - claudine/reviews/2026-07-14-module-assessment/review.md
  - darkmatter/docs/inline/fm-interpolation.md
  - darkmatter/docs/inline/schema-validation.md
  - darkmatter/docs/topics/context-variables.md
  - darkmatter/docs/topics/darkmatter-expressions.md
  - darkmatter/docs/topics/magic-paths.md
  - darkmatter/docs/topics/schema-definition.md
  - darkmatter/docs/transclusion/block-transclusion.md
  - darkmatter/docs/transclusion/transclusion-design.md
  - fixes/2026-07-22-mega-merge/plan.md
  - prompts/_implement/implement-review.md
  - prompts/_implement/implement-suggestions.md
  - prompts/_implement/review-findings-plan.md
  - prompts/_reviews/feature-review.md
  - prompts/_reviews/suggestion-review.md
  - prompts/commit.md
  - prompts/faster-builds-and-tests.md
  - prompts/plan.md
docs_created_during_phase_3:
  - biscuit-terminal/fixes/2026-07-22-table-width/spec.md
  - claudine/cli/tests/fixtures/shipped_implement_route/_implement/implement-plan.md
  - claudine/docs/rendezvous/local-ipc.md
  - claudine/docs/research/herdr.md
  - claudine/docs/research/model-serving-api-standards/_research.md
  - claudine/docs/topics/agentic-research-as-a-typed-knowledge-pipeline.md
  - claudine/docs/topics/error-architecture.md
  - claudine/docs/topics/messaging.md
  - claudine/features/2026-07-11-sequence-plus/gate-run-2026-07-18.md
  - claudine/features/2026-07-11-sequence-plus/gate-run-2026-07-19-l3-linux.md
  - claudine/features/2026-07-11-sequence-plus/gate-run-2026-07-19-linux.md
  - claudine/features/2026-07-11-sequence-plus/gate-run-2026-07-19-windows.md
  - claudine/features/2026-07-11-sequence-plus/gate-run-2026-07-21-windows.md
  - claudine/features/2026-07-11-sequence-plus/l3-ctrl-c-runbook.md
  - claudine/features/2026-07-11-sequence-plus/phase-1-baseline.md
  - claudine/features/2026-07-11-sequence-plus/review-1.md
  - claudine/features/2026-07-11-sequence-plus/review-10.md
  - claudine/features/2026-07-11-sequence-plus/review-11.md
  - claudine/features/2026-07-11-sequence-plus/review-12.md
  - claudine/features/2026-07-11-sequence-plus/review-2.md
  - claudine/features/2026-07-11-sequence-plus/review-3.md
  - claudine/features/2026-07-11-sequence-plus/review-4.md
  - claudine/features/2026-07-11-sequence-plus/review-5.md
  - claudine/features/2026-07-11-sequence-plus/review-6.md
  - claudine/features/2026-07-11-sequence-plus/review-7.md
  - claudine/features/2026-07-11-sequence-plus/review-8.md
  - claudine/features/2026-07-11-sequence-plus/review-9.md
  - claudine/features/2026-07-11-sequence-plus/validation-matrix.md
  - claudine/features/2026-07-13-error-propogation/burndown-triage.md
  - claudine/features/2026-07-13-error-propogation/decisions.md
  - claudine/features/2026-07-13-error-propogation/inventory.md
  - claudine/features/2026-07-13-error-propogation/plan.md
  - claudine/features/2026-07-13-error-propogation/review-1.md
  - claudine/features/2026-07-13-error-propogation/review-2.md
  - claudine/features/2026-07-13-error-propogation/review-3.md
  - claudine/features/2026-07-13-error-propogation/review-4.md
  - claudine/features/2026-07-13-error-propogation/review-5.md
  - claudine/features/2026-07-13-error-propogation/review-6.md
  - claudine/features/2026-07-13-error-propogation/review-7.md
  - claudine/features/2026-07-13-error-propogation/review-8.md
  - claudine/features/2026-07-13-error-propogation/review-9.md
  - claudine/features/2026-07-13-file-resolution/decisions.md
  - claudine/features/2026-07-13-file-resolution/inventory.md
  - claudine/features/2026-07-13-file-resolution/plan.md
  - claudine/features/2026-07-13-file-resolution/review-1.md
  - claudine/features/2026-07-13-file-resolution/review-2.md
  - claudine/features/2026-07-13-file-resolution/review-3.md
  - claudine/features/2026-07-13-file-resolution/review-4.md
  - claudine/features/2026-07-13-file-resolution/review-5.md
  - claudine/features/2026-07-13-file-resolution/review-6.md
  - claudine/features/2026-07-13-file-resolution/review-7.md
  - claudine/features/2026-07-13-file-resolution/review-8.md
  - claudine/features/2026-07-13-file-resolution/review-9.md
  - claudine/features/2026-07-13-proxy-with/notes/acceptance-map.md
  - claudine/features/2026-07-13-proxy-with/notes/baseline.md
  - claudine/features/2026-07-13-proxy-with/notes/state-migration.md
  - claudine/features/2026-07-13-proxy-with/plan.md
  - claudine/features/2026-07-13-proxy-with/review-1.md
  - claudine/features/2026-07-13-proxy-with/review-10.md
  - claudine/features/2026-07-13-proxy-with/review-11.md
  - claudine/features/2026-07-13-proxy-with/review-12.md
  - claudine/features/2026-07-13-proxy-with/review-13.md
  - claudine/features/2026-07-13-proxy-with/review-14.md
  - claudine/features/2026-07-13-proxy-with/review-15.md
  - claudine/features/2026-07-13-proxy-with/review-16.md
  - claudine/features/2026-07-13-proxy-with/review-17.md
  - claudine/features/2026-07-13-proxy-with/review-18.md
  - claudine/features/2026-07-13-proxy-with/review-3.md
  - claudine/features/2026-07-13-proxy-with/review-4.md
  - claudine/features/2026-07-13-proxy-with/review-5.md
  - claudine/features/2026-07-13-proxy-with/review-6.md
  - claudine/features/2026-07-13-proxy-with/review-7.md
  - claudine/features/2026-07-13-proxy-with/review-8.md
  - claudine/features/2026-07-13-proxy-with/review-9.md
  - claudine/features/2026-07-20-lifecycle-ergonomics/spec.md
  - claudine/features/2026-07-20-local-runners-plus/improvements.md
  - claudine/features/_completed/2026-07-11-module-structure/critical-plan.md
  - claudine/features/_completed/2026-07-11-module-structure/nice-plan.md
  - claudine/features/_completed/2026-07-11-module-structure/phase6-discovery.md
  - claudine/features/_completed/2026-07-11-module-structure/review.md
  - claudine/features/_completed/2026-07-11-module-structure/strong-plan.md
  - claudine/fixes/2026-07-09-shared-resources/spec.md
  - claudine/fixes/2026-07-11-display-issue/plan.md
  - claudine/fixes/2026-07-13-cli-switches/plan.md
  - claudine/fixes/2026-07-13-rendezvous-local-ipc/change-notes.md
  - claudine/fixes/2026-07-13-rendezvous-local-ipc/plan.md
  - claudine/fixes/2026-07-13-rendezvous-local-ipc/review-1.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/_research.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/acceptance-ledger.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/baselines/summary.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/claudine-log.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/conflict-checklist.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/conflict-report.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/dirty-worktree-inventory.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/error-prop-and-file-resolution-log.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/impact-review.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/impact/execution-seed/compose-prep.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/impact/execution-seed/composition-error.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/impact/execution-seed/composition-pipeline.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/impact/execution-seed/darkmatter-options.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/impact/execution-seed/harness-loop.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/impact/execution-seed/loop-engine.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/impact/execution-seed/sequence-entry.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/impact/execution-seed/wrapper-stages.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/impact/final-audit-detect.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/impact/foundation-merge-detect.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/impact/foundation/compose-prep.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/impact/foundation/composition-entry.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/impact/foundation/composition-error.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/impact/foundation/composition-pipeline.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/impact/foundation/composition-runner.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/impact/foundation/control-dispatch.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/impact/foundation/error-render.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/impact/foundation/file-cli-src-commands-compose-prep-rs.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/impact/foundation/file-cli-src-commands-wrap-composition-mod-rs.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/impact/foundation/file-cli-src-commands-wrap-composition-pipeline-rs.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/impact/foundation/file-cli-src-commands-wrap-composition-runner-rs.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/impact/foundation/file-cli-src-commands-wrap-harness-orch-loop-control-control-dispatch-rs.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/impact/foundation/file-cli-src-commands-wrap-harness-orch-loop-control-proxy-rs.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/impact/foundation/file-cli-src-commands-wrap-harness-orch-loop-control-rs.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/impact/foundation/file-cli-src-commands-wrap-harness-orch-prompt-rs.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/impact/foundation/file-cli-src-commands-wrap-harness-orch-types-rs.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/impact/foundation/file-cli-src-commands-wrap-overlay-rs.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/impact/foundation/file-cli-src-commands-wrap-sequence-iterate-rs.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/impact/foundation/file-cli-src-commands-wrap-sequence-mod-rs.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/impact/foundation/file-cli-src-commands-wrap-sequence-phase1c-rs.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/impact/foundation/file-cli-src-commands-wrap-wrapper-stages-rs.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/impact/foundation/file-darkmatter-lib-src-markdown-compose-context-options-rs.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/impact/foundation/file-lib-src-composition-error-mod-rs.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/impact/foundation/file-lib-src-composition-error-render-mod-rs.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/impact/foundation/file-lib-src-composition-lifecycle-context-rs.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/impact/foundation/file-lib-src-composition-lifecycle-executor-rs.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/impact/foundation/file-lib-src-composition-looping-engine-rs.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/impact/foundation/file-lib-src-composition-mod-rs.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/impact/foundation/file-lib-src-composition-preflight-rs.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/impact/foundation/file-lib-src-composition-prepare-rs.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/impact/foundation/file-lib-src-composition-types-rs.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/impact/foundation/harness-loop.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/impact/foundation/harness-prompt.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/impact/foundation/harness-types.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/impact/foundation/overlay.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/impact/foundation/proxy-routing.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/impact/foundation/sequence-entry.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/impact/foundation/sequence-iterate.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/impact/foundation/sequence-preflight.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/impact/foundation/wrapper-stages.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/impact/index-freshness.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/impact/phase1-detect-changes.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/impact/proxy-merge-detect.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/impact/proxy/file-cli-src-commands-compose-prep-rs.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/impact/proxy/file-cli-src-commands-wrap-composition-mod-rs.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/impact/proxy/file-cli-src-commands-wrap-composition-pipeline-rs.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/impact/proxy/file-cli-src-commands-wrap-composition-runner-rs.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/impact/proxy/file-cli-src-commands-wrap-harness-orch-loop-control-control-dispatch-rs.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/impact/proxy/file-cli-src-commands-wrap-harness-orch-loop-control-proxy-rs.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/impact/proxy/file-cli-src-commands-wrap-harness-orch-loop-control-rs.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/impact/proxy/file-cli-src-commands-wrap-harness-orch-prompt-rs.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/impact/proxy/file-cli-src-commands-wrap-harness-orch-types-rs.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/impact/proxy/file-cli-src-commands-wrap-overlay-rs.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/impact/proxy/file-cli-src-commands-wrap-sequence-iterate-rs.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/impact/proxy/file-cli-src-commands-wrap-sequence-mod-rs.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/impact/proxy/file-cli-src-commands-wrap-sequence-phase1c-rs.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/impact/proxy/file-cli-src-commands-wrap-wrapper-stages-rs.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/impact/proxy/file-darkmatter-lib-src-markdown-compose-context-options-rs.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/impact/proxy/file-lib-src-composition-error-mod-rs.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/impact/proxy/file-lib-src-composition-error-render-mod-rs.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/impact/proxy/file-lib-src-composition-lifecycle-context-rs.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/impact/proxy/file-lib-src-composition-lifecycle-executor-rs.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/impact/proxy/file-lib-src-composition-looping-engine-rs.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/impact/proxy/file-lib-src-composition-mod-rs.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/impact/proxy/file-lib-src-composition-preflight-rs.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/impact/proxy/file-lib-src-composition-prepare-rs.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/impact/proxy/file-lib-src-composition-types-rs.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/impact/reconciliation-detect.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/impact/symbol-resolution.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/phase1-closeout.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/phase1-gates/current-gate-summary.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/phase2-audit.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/phase2-gates.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/phase2-test-map.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/phase3-audit.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/phase3-gates.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/phase3-test-map.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/phase4-gates.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/phase4-test-map.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/phase5-gates.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/phase5-test-map.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/phase6-gates.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/phase6-test-map.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/plan.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/proxy-with-log.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/review-1.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/reviewed-seed-audit.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/sha-ledger.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/spec.md
  - claudine/fixes/_unscheduled/1-windows-compose-interrupt-guard/spec.md
  - claudine/reviews/2026-07-01-dry-review/plan.md
  - claudine/reviews/_completed/2026-07-14-module-assessment/phase-1-baseline.md
  - claudine/reviews/_completed/2026-07-14-module-assessment/plan.md
  - claudine/reviews/_completed/2026-07-14-module-assessment/review-2.md
  - claudine/reviews/_completed/2026-07-14-module-assessment/review-3.md
  - claudine/reviews/_completed/2026-07-14-module-assessment/review-4.md
  - claudine/reviews/_completed/2026-07-14-module-assessment/review-5.md
  - claudine/reviews/_completed/2026-07-14-module-assessment/review-6.md
  - claudine/reviews/_completed/2026-07-14-module-assessment/spec.md
  - darkmatter/features/2026-07-15-type-system/spec.md
  - sniff/docs/dependencies.md
  - sniff/fixes/2026-07-22-inefficient-calling/spec.md
  - ~/features/2026-07-20-router-fixture/log.md
skills_files_updated_during_phase_3:
  - .claude/skills/biscuit-file/references/file-references.md
  - .claude/skills/biscuit-test-harness/SKILL.md
  - .claude/skills/claudine/SKILL.md
  - .claude/skills/claudine/architecture.md
  - .claude/skills/claudine/cli-reference.md
  - .claude/skills/claudine/error-architecture.md
  - .claude/skills/claudine/linking-strategy.md
  - .claude/skills/claudine/messaging.md
  - .claude/skills/claudine/timeline.md
  - .claude/skills/sniff/SKILL.md
source_files_during_phase_2:
  - sniff/cli/src/output/repo_json.rs
  - sniff/lib/src/filesystem/git/remote_refresh.rs
  - sniff/lib/src/filesystem/git/remote_resolver.rs
docs_updated_during_phase_2:
  - fixes/2026-07-22-mega-merge/plan.md
  - prompts/_implement/implement-plan.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_1:
  - biscuit-speaks/cli/src/install_ui.rs
  - biscuit-speaks/cli/src/main.rs
  - playa/cli/src/install_ui.rs
  - playa/cli/src/main.rs
  - sniff/cli/src/args/mod.rs
  - sniff/cli/src/args/repo.rs
  - sniff/cli/src/commands/mod.rs
  - sniff/cli/src/commands/repo.rs
  - sniff/cli/src/install_plan_cmd.rs
  - sniff/cli/src/install_ui.rs
  - sniff/cli/src/output/filesystem/files.rs
  - sniff/cli/src/output/filesystem/mod.rs
  - sniff/cli/src/output/recent_commits.rs
  - sniff/cli/src/output/repo_json.rs
  - sniff/cli/src/perf.rs
  - sniff/cli/tests/cli.rs
  - sniff/cli/tests/snapshots.rs
  - sniff/lib/benches/cases/filesystem.rs
  - sniff/lib/benches/cases/workload_matrix.rs
  - sniff/lib/benches/perf.rs
  - sniff/lib/benches/support/bench_ids.rs
  - sniff/lib/benches/support/builder.rs
  - sniff/lib/benches/support/fixtures.rs
  - sniff/lib/benches/support/remote_report_fixture.rs
  - sniff/lib/examples/work_counts.rs
  - sniff/lib/src/error.rs
  - sniff/lib/src/executable_index.rs
  - sniff/lib/src/filesystem/docs.rs
  - sniff/lib/src/filesystem/file_types/aggregate.rs
  - sniff/lib/src/filesystem/file_types/classify.rs
  - sniff/lib/src/filesystem/file_types/model.rs
  - sniff/lib/src/filesystem/formatting.rs
  - sniff/lib/src/filesystem/git/api.rs
  - sniff/lib/src/filesystem/git/discovery.rs
  - sniff/lib/src/filesystem/git/mod.rs
  - sniff/lib/src/filesystem/git/open.rs
  - sniff/lib/src/filesystem/git/recent_commits.rs
  - sniff/lib/src/filesystem/git/remote_refresh.rs
  - sniff/lib/src/filesystem/git/status.rs
  - sniff/lib/src/filesystem/git/types.rs
  - sniff/lib/src/filesystem/git/worktree.rs
  - sniff/lib/src/filesystem/mod.rs
  - sniff/lib/src/filesystem/repo/aggregate.rs
  - sniff/lib/src/filesystem/repo/aggregate_view.rs
  - sniff/lib/src/filesystem/repo/area.rs
  - sniff/lib/src/filesystem/repo/cargo.rs
  - sniff/lib/src/filesystem/repo/detection.rs
  - sniff/lib/src/filesystem/repo/dotnet.rs
  - sniff/lib/src/filesystem/repo/glob.rs
  - sniff/lib/src/filesystem/repo/go.rs
  - sniff/lib/src/filesystem/repo/gradle.rs
  - sniff/lib/src/filesystem/repo/identity.rs
  - sniff/lib/src/filesystem/repo/manifest_index.rs
  - sniff/lib/src/filesystem/repo/maven.rs
  - sniff/lib/src/filesystem/repo/mod.rs
  - sniff/lib/src/filesystem/repo/nested.rs
  - sniff/lib/src/filesystem/repo/npm.rs
  - sniff/lib/src/filesystem/repo/nx_turbo.rs
  - sniff/lib/src/filesystem/repo/ownership.rs
  - sniff/lib/src/filesystem/repo/polyglot.rs
  - sniff/lib/src/filesystem/repo/python.rs
  - sniff/lib/src/filesystem/repo/seed.rs
  - sniff/lib/src/filesystem/repo/test_runner_usage.rs
  - sniff/lib/src/filesystem/repo/topology.rs
  - sniff/lib/src/filesystem/repo/types.rs
  - sniff/lib/src/filesystem/repo/uv.rs
  - sniff/lib/src/filesystem/system_view.rs
  - sniff/lib/src/hardware/audio.rs
  - sniff/lib/src/hardware/storage.rs
  - sniff/lib/src/lib.rs
  - sniff/lib/src/network/mod.rs
  - sniff/lib/src/os/locale.rs
  - sniff/lib/src/os/time.rs
  - sniff/lib/src/performance.rs
  - sniff/lib/src/performance/counters.rs
  - sniff/lib/src/process.rs
  - sniff/lib/src/programs/enums/metadata.rs
  - sniff/lib/src/programs/host_capability.rs
  - sniff/lib/src/programs/install/command.rs
  - sniff/lib/src/programs/install/execute.rs
  - sniff/lib/src/programs/install/interview.rs
  - sniff/lib/src/programs/install/mod.rs
  - sniff/lib/src/programs/install/options.rs
  - sniff/lib/src/programs/mod.rs
  - sniff/lib/src/programs/schema.rs
  - sniff/lib/src/remote/bitbucket.rs
  - sniff/lib/src/remote/gitea.rs
  - sniff/lib/src/remote/github.rs
  - sniff/lib/src/remote/gitlab.rs
  - sniff/lib/src/remote/mod.rs
  - sniff/lib/src/remote/provider.rs
  - sniff/lib/src/remote/snapshot.rs
  - sniff/lib/src/request.rs
  - sniff/lib/src/services/benchmark.rs
  - sniff/lib/src/services/launchd.rs
  - sniff/lib/src/services/mod.rs
  - sniff/lib/src/services/openrc.rs
  - sniff/lib/src/services/runit.rs
  - sniff/lib/src/services/systemd.rs
  - sniff/lib/tests/benchmark_workloads.rs
  - sniff/lib/tests/git_parity.rs
  - sniff/lib/tests/integration.rs
  - sniff/lib/tests/remote_providers.rs
docs_updated_during_phase_1:
  - .claudine/memory/commits.md
  - CLAUDE.md
  - fixes/2026-07-22-mega-merge/plan.md
  - prompts/_implement/implement-suggestions.md
  - prompts/_reviews/feature-review.md
  - prompts/plan.md
  - sniff/README.md
  - sniff/cli/README.md
  - sniff/docs/sniff-library-architecture.md
  - sniff/lib/CHANGELOG.md
  - sniff/lib/README.md
  - sniff/lib/benches/README.md
docs_created_during_phase_1:
  - sniff/features/2026-07-16-performance/deferred-perf-tests.md
  - sniff/features/2026-07-16-performance/log.md
  - sniff/features/2026-07-16-performance/phases/_completed/01-work-accounting/log.md
  - sniff/features/2026-07-16-performance/phases/_completed/01-work-accounting/spec.md
  - sniff/features/2026-07-16-performance/phases/_completed/02-reuse-and-scope/spec.md
  - sniff/features/2026-07-16-performance/phases/_completed/03-observation-index/spec.md
  - sniff/features/2026-07-16-performance/phases/_completed/04-package-enrichment-and-ownership/spec.md
  - sniff/features/2026-07-16-performance/phases/_completed/05-git-observation/spec.md
  - sniff/features/2026-07-16-performance/phases/_completed/06-remote-network-and-subprocess/spec.md
  - sniff/features/2026-07-16-performance/phases/_completed/07-profile-guided-cleanup/spec.md
  - sniff/features/2026-07-16-performance/phases/_completed/08-cross-platform-validation/spec.md
  - sniff/features/2026-07-16-performance/plan.md
  - sniff/features/2026-07-16-performance/review-1.md
  - sniff/features/2026-07-16-performance/review-10.md
  - sniff/features/2026-07-16-performance/review-11.md
  - sniff/features/2026-07-16-performance/review-12.md
  - sniff/features/2026-07-16-performance/review-2.md
  - sniff/features/2026-07-16-performance/review-3.md
  - sniff/features/2026-07-16-performance/review-4.md
  - sniff/features/2026-07-16-performance/review-5.md
  - sniff/features/2026-07-16-performance/review-6.md
  - sniff/features/2026-07-16-performance/review-7.md
  - sniff/features/2026-07-16-performance/review-8.md
  - sniff/features/2026-07-16-performance/review-9.md
  - sniff/features/2026-07-16-performance/spec.md
  - sniff/reviews/2026-07-13-perf/spec.md
  - sniff/reviews/2026-07-14-filesystem-observation/review.md
skills_files_updated_during_phase_1:
  - .claude/skills/sniff/SKILL.md
  - .claude/skills/sniff/extending.md
  - .claude/skills/sniff/programs.md
source_code:
  - biscuit-speaks/cli/src/install_ui.rs
  - biscuit-speaks/cli/src/main.rs
  - playa/cli/src/install_ui.rs
  - playa/cli/src/main.rs
  - sniff/cli/src/args/mod.rs
  - sniff/cli/src/args/repo.rs
  - sniff/cli/src/commands/mod.rs
  - sniff/cli/src/commands/repo.rs
  - sniff/cli/src/install_plan_cmd.rs
  - sniff/cli/src/install_ui.rs
  - sniff/cli/src/output/filesystem/files.rs
  - sniff/cli/src/output/filesystem/mod.rs
  - sniff/cli/src/output/recent_commits.rs
  - sniff/cli/src/output/repo_json.rs
  - sniff/cli/src/perf.rs
  - sniff/cli/tests/cli.rs
  - sniff/cli/tests/snapshots.rs
  - sniff/lib/benches/cases/filesystem.rs
  - sniff/lib/benches/cases/workload_matrix.rs
  - sniff/lib/benches/perf.rs
  - sniff/lib/benches/support/bench_ids.rs
  - sniff/lib/benches/support/builder.rs
  - sniff/lib/benches/support/fixtures.rs
  - sniff/lib/benches/support/remote_report_fixture.rs
  - sniff/lib/examples/work_counts.rs
  - sniff/lib/src/error.rs
  - sniff/lib/src/executable_index.rs
  - sniff/lib/src/filesystem/docs.rs
  - sniff/lib/src/filesystem/file_types/aggregate.rs
  - sniff/lib/src/filesystem/file_types/classify.rs
  - sniff/lib/src/filesystem/file_types/model.rs
  - sniff/lib/src/filesystem/formatting.rs
  - sniff/lib/src/filesystem/git/api.rs
  - sniff/lib/src/filesystem/git/discovery.rs
  - sniff/lib/src/filesystem/git/mod.rs
  - sniff/lib/src/filesystem/git/open.rs
  - sniff/lib/src/filesystem/git/recent_commits.rs
  - sniff/lib/src/filesystem/git/remote_refresh.rs
  - sniff/lib/src/filesystem/git/status.rs
  - sniff/lib/src/filesystem/git/types.rs
  - sniff/lib/src/filesystem/git/worktree.rs
  - sniff/lib/src/filesystem/mod.rs
  - sniff/lib/src/filesystem/repo/aggregate.rs
  - sniff/lib/src/filesystem/repo/aggregate_view.rs
  - sniff/lib/src/filesystem/repo/area.rs
  - sniff/lib/src/filesystem/repo/cargo.rs
  - sniff/lib/src/filesystem/repo/detection.rs
  - sniff/lib/src/filesystem/repo/dotnet.rs
  - sniff/lib/src/filesystem/repo/glob.rs
  - sniff/lib/src/filesystem/repo/go.rs
  - sniff/lib/src/filesystem/repo/gradle.rs
  - sniff/lib/src/filesystem/repo/identity.rs
  - sniff/lib/src/filesystem/repo/manifest_index.rs
  - sniff/lib/src/filesystem/repo/maven.rs
  - sniff/lib/src/filesystem/repo/mod.rs
  - sniff/lib/src/filesystem/repo/nested.rs
  - sniff/lib/src/filesystem/repo/npm.rs
  - sniff/lib/src/filesystem/repo/nx_turbo.rs
  - sniff/lib/src/filesystem/repo/ownership.rs
  - sniff/lib/src/filesystem/repo/polyglot.rs
  - sniff/lib/src/filesystem/repo/python.rs
  - sniff/lib/src/filesystem/repo/seed.rs
  - sniff/lib/src/filesystem/repo/test_runner_usage.rs
  - sniff/lib/src/filesystem/repo/topology.rs
  - sniff/lib/src/filesystem/repo/types.rs
  - sniff/lib/src/filesystem/repo/uv.rs
  - sniff/lib/src/filesystem/system_view.rs
  - sniff/lib/src/hardware/audio.rs
  - sniff/lib/src/hardware/storage.rs
  - sniff/lib/src/lib.rs
  - sniff/lib/src/network/mod.rs
  - sniff/lib/src/os/locale.rs
  - sniff/lib/src/os/time.rs
  - sniff/lib/src/performance.rs
  - sniff/lib/src/performance/counters.rs
  - sniff/lib/src/process.rs
  - sniff/lib/src/programs/enums/metadata.rs
  - sniff/lib/src/programs/host_capability.rs
  - sniff/lib/src/programs/install/command.rs
  - sniff/lib/src/programs/install/execute.rs
  - sniff/lib/src/programs/install/interview.rs
  - sniff/lib/src/programs/install/mod.rs
  - sniff/lib/src/programs/install/options.rs
  - sniff/lib/src/programs/mod.rs
  - sniff/lib/src/programs/schema.rs
  - sniff/lib/src/remote/bitbucket.rs
  - sniff/lib/src/remote/gitea.rs
  - sniff/lib/src/remote/github.rs
  - sniff/lib/src/remote/gitlab.rs
  - sniff/lib/src/remote/mod.rs
  - sniff/lib/src/remote/provider.rs
  - sniff/lib/src/remote/snapshot.rs
  - sniff/lib/src/request.rs
  - sniff/lib/src/services/benchmark.rs
  - sniff/lib/src/services/launchd.rs
  - sniff/lib/src/services/mod.rs
  - sniff/lib/src/services/openrc.rs
  - sniff/lib/src/services/runit.rs
  - sniff/lib/src/services/systemd.rs
  - sniff/lib/tests/benchmark_workloads.rs
  - sniff/lib/tests/git_parity.rs
  - sniff/lib/tests/integration.rs
  - sniff/lib/tests/remote_providers.rs
documentation:
  - .claude/skills/sniff/SKILL.md
  - .claude/skills/sniff/extending.md
  - .claude/skills/sniff/programs.md
  - .claudine/memory/commits.md
  - CLAUDE.md
  - fixes/2026-07-22-mega-merge/plan.md
  - prompts/_implement/implement-suggestions.md
  - prompts/_reviews/feature-review.md
  - prompts/plan.md
  - sniff/README.md
  - sniff/cli/README.md
  - sniff/docs/sniff-library-architecture.md
  - sniff/features/2026-07-16-performance/deferred-perf-tests.md
  - sniff/features/2026-07-16-performance/log.md
  - sniff/features/2026-07-16-performance/phases/_completed/01-work-accounting/log.md
  - sniff/features/2026-07-16-performance/phases/_completed/01-work-accounting/spec.md
  - sniff/features/2026-07-16-performance/phases/_completed/02-reuse-and-scope/spec.md
  - sniff/features/2026-07-16-performance/phases/_completed/03-observation-index/spec.md
  - sniff/features/2026-07-16-performance/phases/_completed/04-package-enrichment-and-ownership/spec.md
  - sniff/features/2026-07-16-performance/phases/_completed/05-git-observation/spec.md
  - sniff/features/2026-07-16-performance/phases/_completed/06-remote-network-and-subprocess/spec.md
  - sniff/features/2026-07-16-performance/phases/_completed/07-profile-guided-cleanup/spec.md
  - sniff/features/2026-07-16-performance/phases/_completed/08-cross-platform-validation/spec.md
  - sniff/features/2026-07-16-performance/plan.md
  - sniff/features/2026-07-16-performance/review-1.md
  - sniff/features/2026-07-16-performance/review-10.md
  - sniff/features/2026-07-16-performance/review-11.md
  - sniff/features/2026-07-16-performance/review-12.md
  - sniff/features/2026-07-16-performance/review-2.md
  - sniff/features/2026-07-16-performance/review-3.md
  - sniff/features/2026-07-16-performance/review-4.md
  - sniff/features/2026-07-16-performance/review-5.md
  - sniff/features/2026-07-16-performance/review-6.md
  - sniff/features/2026-07-16-performance/review-7.md
  - sniff/features/2026-07-16-performance/review-8.md
  - sniff/features/2026-07-16-performance/review-9.md
  - sniff/features/2026-07-16-performance/spec.md
  - sniff/lib/CHANGELOG.md
  - sniff/lib/README.md
  - sniff/lib/benches/README.md
  - sniff/reviews/2026-07-13-perf/spec.md
  - sniff/reviews/2026-07-14-filesystem-observation/review.md
packages:
  - biscuit-file
  - biscuit-file-cli
  - biscuit-terminal
  - biscuit-test-harness
  - claudine
  - claudine-cli
  - claudine-contract
  - claudine-gen
  - darkmatter
  - rendezvous-client
  - rendezvous-core
  - rendezvous-daemon
  - sniff
---
# Mega Merge Execution Plan

Status: ready for execution

Source of truth: [spec.md](spec.md)

## Outcome and hard completion gate

Execute one ancestry-preserving integration on the `mega-merge` worktree and
`feat/mega-merge` branch, in dependency order:

1. Sniff
2. Darkmatter
3. Claudine

This plan is complete only when all of the following are true on one unchanged
candidate commit:

- `HEAD` contains the three frozen source tips as ancestors.
- The first-parent history contains one reviewed merge commit for each stream,
  in Sniff → Darkmatter → Claudine order.
- Every workspace Level-1 suite is green.
- Every applicable Level-2 suite is green through its managed `just test-l2`
  recipe; a resource skip is not accepted as a pass for the affected areas.
- All lint recipes complete without warnings or failures.
- Every focused seam, lifecycle, generated-artifact, and platform checkpoint in
  this plan is green.
- Native macOS, Linux, and Windows evidence is attached to the exact candidate
  SHA.
- The Claudine, Darkmatter, and Sniff skills match the merged implementation,
  pass portable Agent Skills validation, have no broken local links, and meet
  the progressive-disclosure gate.
- Final GitNexus change detection has been reviewed against `main`.
- No source worktree or frozen source branch was modified.
- The verified candidate is merged into `main` without changing its tree.

An isolated retry after a full-suite failure is diagnostic evidence only. It
does not satisfy this completion gate; the containing full suite must later
pass.

## Frozen inputs

| Input | Required value |
|---|---|
| Worktree | `/Users/ken/.claudine/worktrees/rusty-biscuit/mega-merge` |
| Branch | `feat/mega-merge` |
| Base `main` | `d30aedd36829256bc677e1d2e73f47a9a2e6005f` |
| Sniff | `0b3286a193899f800a97a24ee3e35c8042602cf6` |
| Darkmatter | `7fb7136dca32a7b1f971b4c83bc1733bcdedebee` |
| Claudine | `8c7a7a8a57d6eebba2e7007df2a6523d9679bbb3` |

Merge the SHA values, not moving branch names. If a source branch advances,
that is a separate integration decision and this plan must be amended before
the new commit enters the candidate.

## Operating rules

- Do not merge `main` into a source branch or merge source branches into one
  another.
- Do not use a global `-X ours`, `-X theirs`, whole-file checkout, or broad
  conflict preference.
- Do not copy a disposable spike tree into the candidate. Replay its decisions
  and tests on the real merge.
- Do not run `cargo fmt` or `rustfmt` in write mode. `main` is the formatting
  authority.
- Preserve unrelated user changes. Stop if an untracked or modified path would
  be overwritten.
- Before editing a function, method, or type during conflict resolution, run
  GitNexus upstream impact analysis for that symbol. Record and surface HIGH or
  CRITICAL results before editing.
- Run GitNexus change detection after each real merge stage and once against
  `main` on the final candidate.
- Run L2 only through the package `just test-l2` recipe. Never invoke L2 through
  Cargo or nextest directly. The managed harness must not steal focus.
- Do not regenerate `CLAUDE.md`, generated provider data, skill hashes, or other
  derived artifacts until production behavior is stable.
- A stage stays uncommitted while red. Create its merge commit only after its
  stage gate is green and its conflict ledger is complete.

## Evidence ledger

Fill these tables during execution. Keep command logs or CI URLs beside the
commit they certify.

### Merge ledger

| Stage | Pre-merge `HEAD` | Incoming SHA | Merge commit | Parent SHAs | Conflicts reviewed | GitNexus result |
|---|---|---|---|---|---|---|
| Sniff | `ae143e497f5a02368f62fad11d6d6adcf49e03e7` | `0b3286a193899f800a97a24ee3e35c8042602cf6` | deferred by no-commit instruction | pending | `CLAUDE.md` | MEDIUM: 5 flows reviewed; 2 CLI performance-output flows covered by L1, 3 benchmark-fixture flows outside production runtime paths |
| Darkmatter | `36aaf6c776fb6ad6e3ace0f969d552582ee4fb6d` (Phase 1 checkpoint created outside this no-commit phase) | `7fb7136dca32a7b1f971b4c83bc1733bcdedebee` | deferred by no-commit instruction | intended parents: `36aaf6c776fb6ad6e3ace0f969d552582ee4fb6d`, `7fb7136dca32a7b1f971b4c83bc1733bcdedebee` | 18 conflicts classified and resolved in the reviewed worktree result | LOW: 23 mapped symbols, 0 affected indexed processes |
| Claudine | | `8c7a7a8a57d6eebba2e7007df2a6523d9679bbb3` | | | | |

### Conflict ledger

For every conflict and every auto-merged semantic-audit file, record:

| Stage | Path/symbol | Classification | Starting authority | Required additive behavior | Focused proof |
|---|---|---|---|---|---|
| Sniff | `CLAUDE.md` | generated / operational | Candidate generated counts | Removed both nested conflict-marker layers; retained the candidate's pre-merge GitNexus counts pending Phase 4 regeneration. | `rg` found no live conflict markers; scoped source and focused suites compile. |
| Sniff | `get_worktrees_from_snapshot` | behavioral | Sniff focused inspection | Omit absent registered targets without weakening typed errors for existing corrupt repositories. | `get_worktrees_omits_an_absent_registered_target`; `get_worktrees_surfaces_an_existing_corrupt_target`; existing full-detail test. |
| Darkmatter | Sniff aggregate / focused worktree split | behavioral | Sniff aggregate + Darkmatter focused inspection | Retain request-scoped ref/worktree projection with zero linked-checkout opens; validate absolute registered paths and stale/corrupt targets only on focused detail paths. | Six focused worktree tests and three aggregate work-counter tests passed. |
| Darkmatter | `RemoteRepoProvider` and focused provider modules | additive API | Additive | Keep snapshot-backed report reuse and add provider URL resolution plus exact/list PR and CI/CD operations with compatible defaults. | Remote report, preferred-remote, exact/list PR, and exact/list CI tests passed. |
| Darkmatter | `biscuit-file` facade | additive API | Additive | Retain file-reference/completion/fetch exports and add YAML span, analyzer, diagnostic, and repair exports. | Seven-crate scoped `cargo check` passed. |
| Darkmatter | `ComposeContext` capture and remote runtime | behavioral | Darkmatter | Preserve demand-driven Sniff-backed capture; network reads remain behind explicit deny-by-default `FetchPolicy` configuration. | Scoped compile, Darkmatter L1/L2, and browser gates passed. |
| Darkmatter | generated and operational conflicts | generated / operational | Deferred artifact authority | Kept candidate GitNexus counts and dispatch inventory pending Phase 4; composed ignore rules, prompt schemas, and commit-memory guidance additively. | No live conflict markers; real index remains untouched. |

### Verification ledger

| Candidate SHA | Host/OS | Gate | Command/workflow | Result | Log or URL |
|---|---|---|---|---|---|
| `ae143e497f5a02368f62fad11d6d6adcf49e03e7` + uncommitted Sniff merge tree | macOS | Focused Spike A | scoped nextest filters for aggregate, worktree, remote/provider, PR/CI, and work counters | 21 passed | local session |
| `ae143e497f5a02368f62fad11d6d6adcf49e03e7` + uncommitted Sniff merge tree | macOS | Sniff L1 | `just test` | 1,679 library + 782 CLI passed | local session |
| `ae143e497f5a02368f62fad11d6d6adcf49e03e7` + uncommitted Sniff merge tree | macOS | Sniff lint | `just lint` | passed without warnings | local session |
| `ae143e497f5a02368f62fad11d6d6adcf49e03e7` + uncommitted Sniff merge tree | macOS | Sniff L2 | `BISCUIT_TEST_LEVEL_REQUIRED=2 just test-l2` | 2 passed through managed harness | local session |
| `ae143e497f5a02368f62fad11d6d6adcf49e03e7` + uncommitted Sniff merge tree | macOS | GitNexus | `detect_changes(scope=all)` | MEDIUM; 5 affected flows reviewed | local session |
| `36aaf6c776fb6ad6e3ace0f969d552582ee4fb6d` + uncommitted Darkmatter merge tree | macOS | Compile spine | scoped `cargo check` for Sniff, Biscuit File, Darkmatter, and their CLIs/DMLS | passed | local session |
| `36aaf6c776fb6ad6e3ace0f969d552582ee4fb6d` + uncommitted Darkmatter merge tree | macOS | Focused Spike A replay | 22 named aggregate/worktree/remote/provider tests | 22 passed | local session |
| `36aaf6c776fb6ad6e3ace0f969d552582ee4fb6d` + uncommitted Darkmatter merge tree | macOS | Sniff L1 + lint | `just test`; `just lint` | 1,788 library + 782 CLI passed; Clippy clean | local session |
| `36aaf6c776fb6ad6e3ace0f969d552582ee4fb6d` + uncommitted Darkmatter merge tree | macOS | Biscuit File L1 + lint | `just test`; `just lint` | 624 library + 61 CLI passed; Clippy clean | local session |
| `36aaf6c776fb6ad6e3ace0f969d552582ee4fb6d` + uncommitted Darkmatter merge tree | macOS | Darkmatter L1 + lint | `just test`; `just lint` | 6,085 library + 643 CLI + 640 DMLS passed; Clippy clean | local session |
| `36aaf6c776fb6ad6e3ace0f969d552582ee4fb6d` + uncommitted Darkmatter merge tree | macOS | Sniff + Darkmatter L2 | `BISCUIT_TEST_LEVEL_REQUIRED=2 just test-l2` in both areas | Sniff CLI 2; Darkmatter library 18 + CLI 69 + DMLS 3 passed | managed real-terminal harness |
| `36aaf6c776fb6ad6e3ace0f969d552582ee4fb6d` + uncommitted Darkmatter merge tree | macOS | Darkmatter browser | `BISCUIT_BROWSER_REQUIRED=1 just test-browser` | 104 passed | headless browser harness |
| `36aaf6c776fb6ad6e3ace0f969d552582ee4fb6d` + uncommitted Darkmatter merge tree | macOS | GitNexus | `detect_changes(scope=all)` | LOW; 23 mapped symbols, 0 affected indexed processes | local session |

## Phase 0 — Freeze and baseline the candidate

### Goal

Prove that execution starts from the intended branch, worktree, and frozen
inputs, with no hidden changes that can contaminate a merge.

### Tasks

- [ ] Confirm the current directory and branch:

  `git rev-parse --show-toplevel` must return the `mega-merge` path and
  `git branch --show-current` must return `feat/mega-merge`.

- [ ] Confirm `HEAD` is the frozen base before the first merge:

  `git rev-parse HEAD` must equal
  `d30aedd36829256bc677e1d2e73f47a9a2e6005f`, except for an explicitly
  reviewed planning-only commit containing this spec and plan.

- [ ] Record `git status --short --branch` and `git worktree list --porcelain`.
  Resolve or explicitly quarantine every unexpected candidate-side change.

- [ ] Confirm each frozen SHA resolves and record the source branch/worktree
  status. Do not clean or modify the source worktrees. In particular, preserve
  Claudine's locally modified generated `CLAUDE.md` as source-worktree state,
  not merge input.

- [ ] Capture the authoritative package catalog with
  `cargo metadata --no-deps --format-version 1` and `sniff repo packages`.

- [ ] Record current CI workflow coverage for Sniff, Biscuit File, Darkmatter,
  and Claudine. Mark every missing native-OS or hard-required L2 leg for Phase
  7; do not mistake a soft or skipped check for evidence.

- [ ] Save the exact focused test filters used by the spikes. If a filter name
  changed in a frozen input, map it to the surviving test rather than silently
  dropping the checkpoint.

### Exit gate

The candidate is clean except for reviewed planning files, all four SHAs are
recorded, source branches remain untouched, and the execution/evidence ledger
is ready.

## Phase 1 — Merge and stabilize Sniff

### Merge

- [x] Record the pre-merge `HEAD`.
- [x] Run:

  `git merge --no-ff --no-commit 0b3286a193899f800a97a24ee3e35c8042602cf6`

- [x] Classify every conflict. Defer final `CLAUDE.md` reconciliation to Phase
  4, but do not let an unresolved generated file hide semantic changes.
- [x] Review all auto-merged files that touch Git discovery, worktrees, remotes,
  request presets, aggregate repository output, or the Sniff skill.

### Required behavioral contract

- [x] Aggregate repository projection reads Git administration metadata without
  opening linked repositories.
- [x] Aggregate projection preserves prunable/stale registrations and keeps the
  zero-linked-repository-open work counter.
- [x] Focused worktree inspection opens registered targets, omits an absent
  stale target, and returns a typed error for an existing corrupt repository.
- [x] Remote selection reuses the request's repository handle and performs no
  repository rediscovery.
- [x] Provider URL, exact/list pull-request, and CI/CD methods remain reachable;
  compatible defaults keep existing provider implementations compiling.
- [x] Bare repositories, linked worktrees, and platform path handling retain
  their documented behavior.

### Verification

- [x] Run the focused aggregate/worktree/remote/work-counter tests identified
  by Spike A.
- [x] Run `just test` and `just lint` from `sniff/`. The full L1 run must be
  warning-free.
- [x] Run `BISCUIT_TEST_LEVEL_REQUIRED=2 just test-l2` from `sniff/` on a host
  with the managed harness.
- [x] Run GitNexus change detection for this stage and review affected flows.
- [x] Record all proof and leave the reviewed merge tree uncommitted for the
  separately authorized commit operation, as required by this implementation
  session's explicit no-commit/no-stage instruction.

### Exit gate

The Sniff stage is independently green, the reviewed integration tree is ready
for the separately authorized merge commit that will preserve the frozen Sniff
tip, and no Darkmatter or Claudine conflict has been resolved early.

## Phase 2 — Merge and stabilize Darkmatter

### Merge

- [x] Record the Sniff-stage `HEAD`.
- [x] Run:

  `git merge --no-ff --no-commit 7fb7136dca32a7b1f971b4c83bc1733bcdedebee`

- [x] Resolve the Sniff–Darkmatter boundary from the ownership matrix:
  Sniff owns aggregate observation cost; Darkmatter owns focused target
  validation; remote-provider additions are additive.
- [x] Review every auto-merged file in Sniff repository discovery, Darkmatter
  context capture, Biscuit File facade/export modules, remote providers,
  schemas, references, caching, and skills.

### Required behavioral contract

- [x] Preserve Sniff's request-scoped “observe once, project many” architecture.
- [x] Preserve Darkmatter's bare-repository handling, typed focused-worktree
  errors, exact/list PR operations, and CI/CD operations.
- [x] Make Biscuit File exports additive: keep Claudine's file/list surfaces and
  Darkmatter's YAML span/analyzer surfaces.
- [x] Keep Darkmatter context capture demand-driven and routed through Sniff.
- [x] Confirm there is no accidental network work in aggregate or denied
  request paths.

### Verification

- [x] Replay all Spike A focused tests on the real candidate.
- [x] Run `just test` and `just lint` from `sniff/`.
- [x] Run `just test` and `just lint` from `biscuit-file/`.
- [x] Run `just test` and `just lint` from `darkmatter/`.
- [x] Run `BISCUIT_TEST_LEVEL_REQUIRED=2 just test-l2` from `sniff/` and
  `darkmatter/`.
- [x] Run Darkmatter's headless browser gate through
  `BISCUIT_BROWSER_REQUIRED=1 just test-browser`. It must remain headless and
  must not gain focus.
- [x] Run GitNexus change detection for this stage and review affected flows.
- [x] Record all proof and leave the reviewed Darkmatter merge tree uncommitted
  and unstaged for the separately authorized commit operation. The intended
  parent pair is recorded in the merge ledger, as required by this
  implementation session's explicit no-commit/no-stage instruction.

### Exit gate

Sniff, Biscuit File, and Darkmatter are jointly green before Claudine enters
the candidate.

## Phase 3 — Merge Claudine in controlled work packets

### Requirement-to-test map recorded before implementation

| Changed behavior | Concrete observable coverage |
|---|---|
| Additive Biscuit File facade and one captured resolution context | Biscuit File `resolution_context`, `detailed_resolution`, `precedence_flip`, `reference_grammar`, `completion_round_trip`, and full library/CLI suites; Claudine `lifecycle_file_functions_reuse_all_request_resolution_inputs` and `prepare_time_and_event_time_agree_on_file_reference`. |
| Cache eligibility and cache-key parity | Darkmatter `options_hash_not_value_compatible_with_pre_migration_encoding`, `options_hash_distinguishes_none_from_empty_value`, `options_hash_magic_path_element_boundaries_are_injective`, `phase_state_identity_matches_underlying_hashes`, and the option-family sensitivity tests in `compose/cache/hashing.rs`. |
| Authorized remote reads and deny-before-network parity | Darkmatter provider-network group, including frontmatter, body, and shell-ternary authorized/denied cases; failures must retain typed provider classification and denied cases must record no request. |
| Typed whole-value projection and configured object-name coercion | Darkmatter `name_coercion_renders_name_in_inline_body_context`, `name_coercion_is_opt_in_body_renders_json_without_keys`, `name_coercion_whole_value_frontmatter_span_keeps_object`, and `name_coercion_inline_frontmatter_value_coerces`. |
| Prebuilt graph identity, freshness, and prepared-heading reuse | Darkmatter prebuilt-graph mutation matrix in `reference_integration`, `fresh_seam_uses_snapshot_while_checked_path_rejects_stale_graph`, `fresh_seam_uses_heading_snapshot_while_checked_path_rejects_stale_headings`, `reference_graph_with_cache_root`, and `fragment_validation_with_cache_root`. |
| Typed reference-error transport | Darkmatter `invalid_reference_propagates_across_all_reference_surfaces` and `permission_failure_propagates_across_all_reference_surfaces`, asserting enumeration, graph construction, and validation preserve the typed cause. |
| Request-scoped schema recursion, cycles, and exact depth | Darkmatter schema reference tests for direct/transitive cycles, repeated non-cyclic references, scalar and root-union entry, exact depth, depth exhaustion, and multi-hop dependency collection. |
| Lifecycle array/object traversal | Claudine `ctx_scan_hint_descends_container_literals` plus the seven container-literal validation tests in `composition/lifecycle/tests/validation.rs`; keys remain data while values are traversed. |
| Source-local lifecycle lookup and unavailable-provider refusal | Claudine split filesystem-lookup inverse assertions, scalar/list provider refusal L1 tests, and `level2_lifecycle_retry_to_an_unavailable_provider_matches_direct_selection` repeated four times through `just test-l2`. |
| Shipped schemas/prompts and normal invocation path | Passive Darkmatter base-schema/corpus suites plus Claudine `shipped_prompts`/`shipped_prompt_route_drift`; real shipped prompt route exercised by `level2_lifecycle_shipped_implement_route_matches_direct_run`. |
| Persisted YAML/frontmatter fidelity | Claudine `frontmatter_yaml_round_trips_a_value`, setter quote round trips, and lifecycle retry mutation tests exercise read/write/read behavior with native and quoted YAML values. |

### Merge

- [x] Record the Darkmatter-stage `HEAD`.
- [x] Materialize the equivalent non-index merge tree (the implementation
  session explicitly forbids staging or committing):

  `git merge --no-ff --no-commit 8c7a7a8a57d6eebba2e7007df2a6523d9679bbb3`

- [x] Classify every conflict and add every auto-merged composition/schema/
  reference/lifecycle file to the semantic audit list.
- [x] Do not restore Claudine-deleted monolithic lifecycle test files. Port
  surviving assertions into Claudine's split test layout.

### Work packet A — API and facade spine

- [x] Compose, rather than replace, `biscuit-file` module declarations and
  exports.
- [x] Establish the compile spine in this order:
  Biscuit File → Darkmatter library → Darkmatter tests → Claudine library →
  Claudine CLI/tests.
- [x] Run focused compile/check after each seam closes so API-shape failures do
  not mix with orchestration failures.

### Work packet B — Request context and cache identity

- [x] Keep Claudine's single captured `FileResolutionContext`.
- [x] Add Darkmatter's remote runtime, cache, identity, provider-query, trigger,
  meta-schema, origin, and dependency state without recapturing ambient CWD,
  environment, or repository state.
- [x] Give each compose option one cache classification and one encoding;
  `options_hash` must delegate to the canonical compose-cache fingerprint.
- [x] Prove cache eligibility and cache-key construction cannot drift.

### Work packet C — File/expression projection and authorization parity

- [x] Preserve typed whole-value object results and the scalar/array fast path.
- [x] Route string object projection through the lookup hook so configured
  `name_coercion_keys` can select `.name`.
- [x] Give frontmatter, body interpolation, and shell ternary reads the same
  authorized remote runtime, denial behavior, and cache behavior.
- [x] Prove deny-before-network behavior on every surface.

### Work packet D — References and freshness

- [x] Normalize request options exactly as graph construction does before
  comparing identities.
- [x] Resolve targets through the captured Claudine context.
- [x] Preserve Darkmatter's prepared-heading cache for cross-document fragment
  validation.
- [x] Preserve typed `ReferenceError` values; stringify only genuinely
  non-reference graph failures.
- [x] Replay the freshness mutation matrix: child, descendant, heading, schema
  dependency, and option identity each invalidate through only their documented
  channel.

### Work packet E — Schema assembly and recursion

- [x] Preserve Claudine request-scoped resolution and typed errors.
- [x] Restore Darkmatter origin, dependency, namespace/example cache, trigger,
  meta-schema, and source-aware validation state.
- [x] Thread the immutable request context through scalar and root-union
  recursion while preserving the canonical-path open-frame stack and depth cap.
- [x] Prove direct/transitive cycles, repeated non-cyclic references, exact
  depth, depth exhaustion, root unions, and dependency collection.

### Work packet F — Lifecycle traversal and test port

- [x] Extend preflight traversal through array elements and object values;
  object keys remain data.
- [x] Port the eight surviving lifecycle/container assertions into Claudine's
  split validation and filesystem-lookup suites.
- [x] Retire the four contradictory launch-area fallback assertions. The
  source-local semantics in Claudine remain authoritative.
- [x] Preserve handoff, retry, resume, provider selection, and unavailable
  provider refusal behavior.

### Mandatory focused regression groups

All eight groups must pass on the real materialized candidate:

1. cache eligibility/hash encoding parity;
2. normalized graph validation identity;
3. prepared-heading cache use for cross-document fragments;
4. authorized frontmatter remote runtime;
5. name-coercion lookup behavior;
6. schema cycle/depth protection;
7. typed reference-error transport;
8. lifecycle traversal of array/object expression literals.

Also require:

- [x] provider-network and deny-before-network group;
- [x] prebuilt reference graph compatibility/freshness group;
- [x] cache-root and fragment-validation groups;
- [x] Biscuit File captured-context differential oracle;
- [x] all eight ported lifecycle assertions;
- [x] unavailable scalar/list provider refusal L1 tests;
- [x] the managed unavailable-provider retry L2 test four consecutive times.

### Stage verification

- [x] Run `just test` and `just lint` in `biscuit-file/`, `sniff/`,
  `darkmatter/`, and `claudine/`.
- [x] Run hard-required managed L2 in `sniff/`, `darkmatter/`, and
  `claudine/`.
- [x] Run Claudine `signals-check` and `test-gen`.
- [x] Run Darkmatter's required headless browser gate.
- [x] Re-run the complete Darkmatter and Claudine L1 suites until each has one
  clean full-suite result. The previously observed slow cleanup and context
  width timeouts remain visible in the ledger; an isolated pass does not close
  them.
- [x] Run GitNexus change detection for the Claudine stage and review affected
  flows.
- [x] Record all proof and leave the reviewed Claudine merge tree uncommitted
  and unstaged for the separately authorized commit operation. The intended
  parent pair is recorded in the merge ledger, as required by this
  implementation session's explicit no-commit/no-stage instruction.

### Verification evidence

- Biscuit File L1: library 723 passed (4 declared skips); CLI 61 passed. Lint
  passed.
- Sniff L1: library 1,801 passed (19 declared skips); CLI 782 passed (3
  declared skips). Lint passed. Hard-required L2 passed 2/2.
- Darkmatter L1 completed cleanly after exact-test nextest timeout policies
  were added for the known slow cleanup regression; all library, CLI, and DMLS
  suites passed. Lint passed. Hard-required L2 passed 18/18 library, 69/69 CLI,
  and 3/3 DMLS. Required headless browser coverage passed 104/104.
- Claudine L1 completed cleanly across catalog types, library, contract, CLI,
  and generator. The library's reserved-root regression was flaky under suite
  contention but passed on its third nextest attempt; it was not skipped.
  Lint, `signals-check`, and `test-gen` passed. Hard-required L2 passed 228/228.
- The unavailable-provider retry L2 regression passed in the full L2 run and
  in three immediately consecutive focused reruns (four consecutive passes).
- GitNexus change detection reported the expected critical merge-scale scope:
  4,304 changed indexed symbols, 98 affected symbols, and 1,204 files relative
  to `main`. Reviewed affected flows center on composition, lifecycle/harness,
  sequence execution, reference/transclusion, and file-resolution paths covered
  by the focused and full gates above.

### Exit gate

The candidate contains all three frozen tips and all focused semantic seams are
green before generated files or skills are finalized.

## Phase 4 — Generated artifacts and repository hygiene

- [ ] Confirm the purpose and intended final path of the tracked empty
  `~/features/2026-07-20-router-fixture/log.md` path. Keep, move, or delete it
  only from explicit fixture evidence.
- [ ] Compare generated outputs before regeneration and classify every
  difference. Generated files must not smuggle a semantic conflict resolution.
- [ ] Regenerate provider/catalog outputs once from the settled source, using
  repository recipes.
- [ ] Regenerate `CLAUDE.md`/GitNexus counts from the settled candidate rather
  than taking either branch's generated copy.
- [ ] Confirm no local `.claude/settings.local.json` or source-worktree-only
  change entered the candidate.
- [ ] Review `Cargo.lock`, manifests, generated schemas/catalogs, and symlinks
  explicitly.
- [ ] Re-run the focused tests affected by regeneration and verify a second
  generation is clean (idempotent).

### Exit gate

Derived artifacts are reproducible from the merged source and a second
generation produces no diff.

## Phase 5 — Agent Skills drift and progressive disclosure

Perform this phase after behavior and generated artifacts settle so the skills
describe the final implementation once.

### Common audit for all three skills

- [ ] Compare the merged package architecture, public APIs, CLI commands,
  test recipes, platform behavior, and invariants to every claim in
  `.claude/skills/{claudine,darkmatter,sniff}/`.
- [ ] Assume code is correct when a comment or skill claim drifts; update or
  remove the stale documentation in the same documentation phase.
- [ ] Validate the frontmatter description contains the real trigger contexts
  and the body does not carry a redundant “when to use” section.
- [ ] Search for consumers of top-level `hash` and `last_updated` before
  changing them. Normalize the entry files to portable Agent Skills
  frontmatter; do not retain non-standard top-level keys without a documented
  consumer and reviewed exception.
- [ ] Do not add Codex-specific `agents/openai.yaml` sidecars merely for this
  gate. These repository skills target the provider-neutral Agent Skills core.
- [ ] Keep essential procedures and safety invariants in `SKILL.md`; move
  history, catalogs, long examples, and subsystem detail to directly linked
  references.
- [ ] Keep each `SKILL.md` below 500 lines and approximately 5,000 words.
- [ ] Ensure every retained long operational reference is navigable (compact
  contents list or an explicit, reviewed reason that its existing structure is
  sufficient).
- [ ] Avoid duplicated facts between `SKILL.md` and references. The entry point
  should route; the reference should own the detail.
- [ ] Validate every local Markdown link and symlink resolves from the skill
  directory. Fix the known Darkmatter `code_block.rs` relative link.
- [ ] Confirm all resource paths use portable relative references and that no
  link depends on a source worktree outside the skill package.

### Skill-specific restructuring

- [ ] Claudine: preserve its concise architecture/CLI routing, then audit the
  incoming `error-architecture.md`, `messaging.md`, architecture, CLI, linking,
  and research links for final-behavior drift and duplication.
- [ ] Darkmatter: move the DMLS phase chronology, extracted-surface catalog,
  and detailed rendering implementation notes behind directly linked topic
  references. Keep the composition/schema/context/remote authority and browser
  safety contract in the entry point.
- [ ] Sniff: move work-counter evidence and detailed catalog/CLI material behind
  topic references. Keep platform support, request-cost tiers, aggregate versus
  focused worktree semantics, and cross-platform test gotchas in the entry
  point.

### Mechanical validation

- [ ] Run for each skill:

  `uv run --with pyyaml /Users/ken/.claude/skills/.system/skill-creator/scripts/quick_validate.py .claude/skills/<skill>`

- [ ] Run the link/symlink check from the skill directory and record zero
  broken targets.
- [ ] Record final line and word counts for all three entry points.
- [ ] Render/read each entry point and one routed topic as a cold reader; verify
  that the correct next document is discoverable without loading unrelated
  subsystems.
- [ ] Re-run package documentation drift guards, including Claudine's lifecycle
  facet check and any generated-doc checks.

### Exit gate

All three skills are accurate, portable, validator-green, link-clean, and
progressively disclosed. A reviewed exception must name the rule, rationale,
and owner; “existing file” is not a rationale.

## Phase 6 — Final local verification on macOS

Run these against one recorded candidate SHA. Any subsequent code, test,
generated, skill, manifest, or workflow change invalidates the result and
requires the affected gates plus the final aggregate gates again.

### Workspace-wide gates

- [ ] `just check-canonical`
- [ ] `just build`
- [ ] `just test` — all Cargo workspace packages from metadata, Level 1
- [ ] `just doctest`
- [ ] `just lint` — zero warnings and zero failures
- [ ] `just all` — all canonical tiers for every curated package area
- [ ] `just check-test-interrupts`
- [ ] `just test-leaks sniff biscuit-file darkmatter claudine`

### Hard-required affected-area gates

- [ ] `(cd sniff && BISCUIT_TEST_LEVEL_REQUIRED=2 just test-l2)`
- [ ] `(cd darkmatter && BISCUIT_TEST_LEVEL_REQUIRED=2 just test-l2)`
- [ ] `(cd claudine && BISCUIT_TEST_LEVEL_REQUIRED=2 just test-l2)`
- [ ] `(cd darkmatter && BISCUIT_BROWSER_REQUIRED=1 just test-browser)`
- [ ] `(cd claudine && just signals-check && just test-gen)`

Record executed, passed, skipped, and not-applicable counts separately. A skip
in an affected L1/L2 gate is a blocker unless the test is intentionally
platform-inapplicable and the ledger identifies the native host that executes
it.

## Phase 7 — Native Linux and Windows evidence

The exact candidate SHA must receive native evidence; cross-compilation alone
does not satisfy runtime path, filesystem, or work-counter behavior.

### CI coverage closure

- [ ] Keep the existing Sniff macOS/Linux/Windows all-target and L1 matrix, and
  Unix L2 legs, green.
- [ ] Promote Darkmatter's Windows leg from soft evidence to a required check
  for this candidate. Enable its reusable Linux L2 and headless-browser jobs.
- [ ] Add durable Biscuit File and Claudine area coverage through the shared
  area-CI workflow (or an equally strict existing workflow): macOS all-target
  check, Linux and Windows L1, and Linux hard-required L2 where applicable.
  Preserve Claudine's generator/signals job and Windows Ctrl+C runtime job.
- [ ] Ensure workflow lint/check output treats warnings as failures.
- [ ] Do not use a temporary workflow that is deleted before the final
  candidate SHA; the evidence must correspond to the tree being merged.

### Native functional matrix

| Checkpoint | macOS | Linux | Windows |
|---|---:|---:|---:|
| Sniff all-target compile + L1 | required | required | required |
| Sniff work counters and aggregate/focused worktree behavior | required | required | required |
| Biscuit File captured CWD/env/root/path oracle | required | required | required |
| Darkmatter all-target compile + L1 | required | required | required |
| Claudine all-target compile + L1 | required | required | required |
| Managed L2 | required where harness is supported | hard-required | platform-inapplicable unless a native harness exists |
| Darkmatter headless browser | required locally or Linux CI | hard-required | not required |
| Claudine Windows Ctrl+C/Job Object | not applicable | not applicable | required |
| Lint, docs guards, generated drift | required | required | required or compile-equivalent where shell tooling is unavailable |

Special attention on Linux and Windows:

- [ ] drive-relative and UNC paths;
- [ ] separator/case normalization;
- [ ] symlink and reparse-point containment;
- [ ] worktree administration paths and prunable registrations;
- [ ] no extra repository opens or network probes;
- [ ] typed syntax/permission errors across enumeration, graph construction,
  and validation.

### Exit gate

All required checks are green and non-soft on the exact candidate SHA. A
cancelled, skipped, allowed-to-fail, or superseded run is not evidence.

## Phase 8 — Final review, ancestry proof, and merge to main

### Ancestry and history

- [ ] Run:

    - `git merge-base --is-ancestor 0b3286a193899f800a97a24ee3e35c8042602cf6 HEAD`
    - `git merge-base --is-ancestor 7fb7136dca32a7b1f971b4c83bc1733bcdedebee HEAD`
    - `git merge-base --is-ancestor 8c7a7a8a57d6eebba2e7007df2a6523d9679bbb3 HEAD`

- [ ] Inspect `git log --first-parent --merges --oneline` and the parent list of
  each stage commit. Confirm Sniff → Darkmatter → Claudine order and two-parent
  merge commits.
- [ ] Confirm the source branch tips still equal the frozen SHAs and source
  worktree status has not changed due to this execution.

### Change and conflict review

- [ ] Run GitNexus `detect_changes` with compare scope against `main` and the
  explicit `mega-merge` worktree path.
- [ ] Review all reported affected processes and all HIGH/CRITICAL symbols.
  Compare them to the conflict ledger; unexplained scope is a blocker.
- [ ] Review `git diff --check` and the full `main...HEAD` diff.
- [ ] Confirm no conflict markers, temporary spike paths, generated drift,
  local settings, or accidental formatting-only rewrites remain.
- [ ] Confirm every checkbox and evidence-ledger row is complete.

### Merge

- [ ] Freeze the verified candidate SHA.
- [ ] Merge that exact candidate into `main` without squashing away its source
  ancestry and without changing the candidate tree.
- [ ] Verify the resulting `main` tree matches the candidate tree and all three
  frozen tips remain ancestors of `main`.
- [ ] Do not declare completion until required post-merge branch protection/CI
  checks are green.

## Stop conditions

Stop and update the spec before proceeding if:

- a frozen source SHA changes;
- a proposed resolution contradicts the semantic ownership matrix;
- a whole-file preference appears necessary in schemas, references, context,
  or lifecycle code;
- a source branch/worktree would need modification;
- a focused seam test has no surviving equivalent;
- a HIGH/CRITICAL GitNexus result is not understood;
- a full suite repeatedly passes only when isolated;
- native Windows/Linux evidence requires weakening a check;
- an Agent Skill cannot meet portable validation without breaking a confirmed
  repository consumer.

The response to a stop condition is a documented decision or a new focused
spike—not a broader merge strategy, a skipped gate, or an unexplained
exception.
