---
spec: /Volumes/coding/wt/rusty-biscuit/feat-osc-9-4-protocol/claudine/features/2026-10-24-sequence-containers/spec.md
plan: claudine/features/2026-10-24-sequence-containers/plan.md
implemented_by: opencode/zai-coding-plan/glm-5.3
started_phase: 1
human_review: false
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
    - messenger/lib/src/research/refresh/runner.rs
    - messenger/lib/src/research/refresh/state.rs
    - messenger/lib/src/research/refresh/prepare.rs
    - messenger/lib/src/research/refresh/check.rs
    - messenger/lib/src/research/refresh/promote.rs
    - messenger/lib/src/research/refresh/select.rs
    - messenger/lib/src/research/refresh/cleanup.rs
    - messenger/lib/src/research/refresh/mod.rs
    - messenger/cli/src/research.rs
    - messenger/cli/src/research_lifecycle.rs
    - messenger/justfile
    - messenger/lib/tests/research_refresh.rs
    - messenger/cli/tests/research_lifecycle_cli.rs
    - claudine/cli/src/commands/sequence.rs
    - claudine/cli/src/commands/mod.rs
    - claudine/cli/src/args.rs
    - claudine/cli/src/main.rs
    - claudine/cli/src/telemetry.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/control_dispatch.rs
    - claudine/cli/src/commands/wrap/sequence/iterate.rs
    - claudine/cli/src/commands/wrap/sequence/mod.rs
    - claudine/cli/src/commands/wrap/exec/termination/windows.rs
    - claudine/cli/src/commands/wrap/exec/spawn/captured.rs
    - claudine/cli/src/commands/wrap/exec/spawn/inherited.rs
    - claudine/cli/src/commands/wrap/exec/spawn/retained.rs
    - claudine/cli/src/commands/wrap/exec/spawn/semantic.rs
    - claudine/cli/src/commands/wrap/exec/wiring/session.rs
    - claudine/cli/src/completion/engine/tests.rs
    - claudine/cli/tests/l1/sequence_errors_cli.rs
    - claudine/cli/tests/l1/main.rs
    - claudine/cli/tests/l1/completion_ownership.rs
    - claudine/cli/tests/l1/provider_tail_ownership.rs
    - claudine/cli/tests/common/owned_value_options.rs
    - claudine/cli/tests/l1/error_guards/transport-allow.toml
docs_updated_during_phase_2:
    - claudine/docs/cli/index.md
    - claudine/docs/cli/sequence.md
    - claudine/README.md
    - claudine/docs/topics/completions/shell-completions.md
    - claudine/docs/dependencies.md
    - docs/dependencies.md
    - messenger/cli/README.md
    - messenger/docs/user-guide.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
    - .claude/skills/messenger/research-contract.md
    - .claude/skills/messenger/SKILL.md
    - .claude/skills/claudine/cli-commands.md
    - .claude/skills/claudine/timeline.md
packages:
    - claudine
    - claudine-cli
    - messenger
    - messenger-cli
    - darkmatter-cli
    - darkmatter
    - biscuit-tui
    - reaper
    - schematic-define
    - claudine-gen

message_to_agent: "Final verification is complete; implementation is ready for review. Required local gates and the 31-case shipped-prompt checkpoint pass. The Darkmatter documentation guard and sibling context/skill guidance now describe concurrent groups. Preserve separate seq/group bindings, caller provenance, cancellation scopes, and RuntimeSnapshot.proxy_chain. Review the Phase 9 log for 11 Messenger and 5 Darkmatter/DMLS tests over the L1 speed budget, three inherited failures outside the required opt-in prompt checkpoint, ignored cases, the existing local-runner freshness defect, and native Windows Ctrl-C/Windows-WSL2 L2 coverage gaps. Only macOS was exercised in Phase 9. No formatting, staging, commit, or lifecycle move was performed."
source_files_during_phase_3:
    - claudine/cli/src/argv/partition.rs
    - claudine/cli/src/argv/partition/tests.rs
    - claudine/cli/src/commands/compose/prep.rs
    - claudine/cli/src/commands/sequence.rs
    - claudine/cli/src/commands/wrap/sequence/jit.rs
    - claudine/cli/src/commands/wrap/sequence/review.rs
    - claudine/cli/src/commands/wrap/sequence/review/tests.rs
    - claudine/cli/src/commands/wrap/sequence/task_run.rs
    - claudine/cli/src/commands/wrap/sequence/task_run/tests.rs
    - claudine/cli/src/completion/engine/tests.rs
    - claudine/cli/src/completion/schema_completion/tests.rs
    - claudine/cli/tests/common/owned_value_options.rs
    - claudine/cli/tests/fixtures/frozen_prompts/prompts/review-loop.md
    - claudine/cli/tests/l1/agent_text_is_data.rs
    - claudine/cli/tests/l1/completion_ownership.rs
    - claudine/cli/tests/l1/composition_outputs.rs
    - claudine/cli/tests/l1/ctx_per_run.rs
    - claudine/cli/tests/l1/document_environment_routes.rs
    - claudine/cli/tests/l1/handoff_owners.rs
    - claudine/cli/tests/l1/launch_trigger_schemas.rs
    - claudine/cli/tests/l1/lifecycle_downgrade_outcome.rs
    - claudine/cli/tests/l1/lifecycle_mixed_span_validation.rs
    - claudine/cli/tests/l1/loop_cli.rs
    - claudine/cli/tests/l1/provider_tail_notice.rs
    - claudine/cli/tests/l1/sequence_cli.rs
    - claudine/cli/tests/l1/sequence_ctrl_c_windows.rs
    - claudine/cli/tests/l1/sequence_errors_cli.rs
    - claudine/cli/tests/l1/sequence_groups.rs
    - claudine/cli/tests/l1/sequence_jit.rs
    - claudine/cli/tests/l1/sequence_perf.rs
    - claudine/cli/tests/l1/sequence_shell_templates.rs
    - claudine/cli/tests/l1/sequence_sources_cli.rs
    - claudine/cli/tests/l1/tts_prewarm.rs
    - claudine/lib/src/composition/error/mod.rs
    - claudine/lib/src/composition/lifecycle/actions.rs
    - claudine/lib/src/composition/lifecycle/audio.rs
    - claudine/lib/src/composition/lifecycle/bindings.rs
    - claudine/lib/src/composition/lifecycle/executor/tests/event_time_interpolation.rs
    - claudine/lib/src/composition/lifecycle/mod.rs
    - claudine/lib/src/composition/lifecycle/parse.rs
    - claudine/lib/src/composition/lifecycle/prewarm/sequence/tests.rs
    - claudine/lib/src/composition/lifecycle/runtime.rs
    - claudine/lib/src/composition/lifecycle/tests/grammar.rs
    - claudine/lib/src/composition/lifecycle/tests/nested_span.rs
    - claudine/lib/src/composition/lifecycle/tests/parse_config.rs
    - claudine/lib/src/composition/sequence/formal.rs
    - claudine/lib/src/composition/sequence/grammar.rs
    - claudine/lib/src/composition/sequence/mod.rs
    - claudine/lib/src/composition/sequence/model.rs
    - claudine/lib/src/composition/sequence/normalize.rs
    - claudine/lib/src/composition/sequence/preflight/mod.rs
    - claudine/lib/src/composition/sequence/preflight/runtime_fixture.rs
    - claudine/lib/src/composition/sequence/preflight/shape.rs
    - claudine/lib/src/composition/sequence/preflight/tests.rs
    - claudine/lib/src/composition/sequence/reserved.rs
    - claudine/lib/src/composition/sequence/source.rs
    - claudine/lib/src/composition/sequence/task/tests.rs
    - claudine/lib/src/composition/sequence/tests.rs
    - claudine/lib/tests/l1/claudine_schemas.rs
    - claudine/lib/tests/l1/lifecycle_lift_ready_guard.rs
    - claudine/schemas/action.yaml
    - claudine/schemas/claudine.yaml
    - claudine/schemas/lifecycle.yaml
    - darkmatter/cli/tests/l1/claudine_schema_bundle.rs
    - darkmatter/lib/tests/l1/shipped_schema_corpus.rs

docs_updated_during_phase_3:
    - claudine/README.md
    - claudine/docs/cli/sequence.md
    - claudine/docs/topics/flow-control/lifecycle.md
    - claudine/docs/topics/flow-control/sequences.md
    - claudine/features/2026-10-24-sequence-containers/implementation-log.md
    - claudine/features/2026-10-24-sequence-containers/plan.md
    - claudine/schemas/README.md

docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
    - .claude/skills/claudine/SKILL.md
source_files_during_phase_4:
    - claudine/cli/src/commands/compose/prep.rs
    - claudine/cli/src/commands/sequence.rs
    - claudine/cli/src/commands/wrap/composition/pipeline.rs
    - claudine/cli/src/commands/wrap/composition/preflight.rs
    - claudine/cli/src/commands/wrap/composition/staged_boot.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/lifecycle_events.rs
    - claudine/cli/src/commands/wrap/harness_orch/shell_options.rs
    - claudine/cli/src/commands/wrap/sequence/iterate.rs
    - claudine/cli/src/commands/wrap/sequence/jit.rs
    - claudine/cli/src/commands/wrap/sequence/jit/tests.rs
    - claudine/cli/src/commands/wrap/sequence/task_run.rs
    - claudine/cli/tests/common/drain_interrupt.rs
    - claudine/cli/tests/fixtures/frozen_prompts/prompts/_implement/implement-plan.md
    - claudine/cli/tests/fixtures/frozen_prompts/shipped_implement_route/_implement/implement-plan.md
    - claudine/cli/tests/l1/agent_text_is_data.rs
    - claudine/cli/tests/l1/compose_caller_file_provenance.rs
    - claudine/cli/tests/l1/compose_frontmatter_model.rs
    - claudine/cli/tests/l1/compose_removed_validation_keys.rs
    - claudine/cli/tests/l1/composition_outputs.rs
    - claudine/cli/tests/l1/handoff_owners.rs
    - claudine/cli/tests/l1/inline_completion_lifecycle.rs
    - claudine/cli/tests/l1/lifecycle_message_drain.rs
    - claudine/cli/tests/l1/lifecycle_message_drain_interrupt.rs
    - claudine/cli/tests/l1/loop_cli.rs
    - claudine/cli/tests/l1/preflight_execution_parity.rs
    - claudine/cli/tests/l1/sequence_cli.rs
    - claudine/cli/tests/l1/sequence_errors_cli.rs
    - claudine/cli/tests/l1/sequence_groups.rs
    - claudine/cli/tests/l1/sequence_jit.rs
    - claudine/cli/tests/l1/sequence_magic_reference.rs
    - claudine/cli/tests/l1/sequence_overlay_pty.rs
    - claudine/cli/tests/l1/sequence_perf.rs
    - claudine/cli/tests/l1/sequence_prompt_property.rs
    - claudine/cli/tests/l1/sequence_schema.rs
    - claudine/cli/tests/l1/sequence_shell_templates.rs
    - claudine/cli/tests/l1/sequence_sources_cli.rs
    - claudine/cli/tests/l1/wrap_compose_validation.rs
    - claudine/lib/src/composition/error/render/lifecycle.rs
    - claudine/lib/src/composition/error/tests.rs
    - claudine/lib/src/composition/lifecycle/bindings.rs
    - claudine/lib/src/composition/lifecycle/bindings/containers.rs
    - claudine/lib/src/composition/lifecycle/bindings/rename.rs
    - claudine/lib/src/composition/lifecycle/bindings/rename/tests.rs
    - claudine/lib/src/composition/lifecycle/bindings/tests.rs
    - claudine/lib/src/composition/lifecycle/context/tests.rs
    - claudine/lib/src/composition/lifecycle/executor.rs
    - claudine/lib/src/composition/lifecycle/executor/tests/filesystem_lookup.rs
    - claudine/lib/src/composition/lifecycle/executor/tests/mod.rs
    - claudine/lib/src/composition/lifecycle/executor/tests/runtime_set.rs
    - claudine/lib/src/composition/lifecycle/prewarm/classify/tests.rs
    - claudine/lib/src/composition/lifecycle/prewarm/sequence/tests.rs
    - claudine/lib/src/composition/lifecycle/prewarm/tests.rs
    - claudine/lib/src/composition/lifecycle/tests/audio_emission.rs
    - claudine/lib/src/composition/lifecycle/tests/guard_runtime.rs
    - claudine/lib/src/composition/looping/actions/tests.rs
    - claudine/lib/src/composition/looping/engine.rs
    - claudine/lib/src/composition/looping/expression.rs
    - claudine/lib/src/composition/preflight.rs
    - claudine/lib/src/composition/preflight/tests.rs
    - claudine/lib/src/composition/prepare.rs
    - claudine/lib/src/composition/prepare/tests.rs
    - claudine/lib/src/composition/reserved.rs
    - claudine/lib/src/composition/runtime_state.rs
    - claudine/lib/src/composition/runtime_state/tests.rs
    - claudine/lib/src/composition/sequence/mod.rs
    - claudine/lib/src/composition/sequence/model.rs
    - claudine/lib/src/composition/sequence/preflight/mod.rs
    - claudine/lib/src/composition/sequence/preflight/tests.rs
    - claudine/lib/src/composition/sequence/reserved.rs
    - claudine/lib/src/composition/sequence/task/group.rs
    - claudine/lib/src/composition/sequence/task/mod.rs
    - claudine/lib/src/composition/sequence/task/templates.rs
    - claudine/lib/src/composition/sequence/task/tests.rs
    - claudine/lib/src/composition/sequence/tests.rs
    - claudine/lib/src/composition/types.rs
    - claudine/lib/tests/l1/agent_errors_fleet.rs
    - darkmatter/lib/src/markdown/compose/context/effective_state.rs
    - darkmatter/lib/src/markdown/compose/expression/binding.rs
    - darkmatter/lib/src/markdown/compose/frontmatter_interpolation.rs
docs_updated_during_phase_4:
    - claudine/README.md
    - claudine/docs/cli/sequence.md
    - claudine/docs/topics/flow-control/lifecycle.md
    - claudine/docs/topics/flow-control/sequences.md
    - claudine/features/2026-10-24-sequence-containers/implementation-log.md
    - claudine/features/2026-10-24-sequence-containers/plan.md
    - darkmatter/docs/topics/darkmatter-expressions.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
    - .claude/skills/claudine/SKILL.md
    - .claude/skills/darkmatter/compose.md
source_files_during_phase_5:
    - claudine/cli/src/commands/sequence.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/retry_resume.rs
    - claudine/cli/src/commands/wrap/sequence/container.rs
    - claudine/cli/src/commands/wrap/sequence/iterate.rs
    - claudine/cli/src/commands/wrap/sequence/jit/tests.rs
    - claudine/cli/src/commands/wrap/sequence/mod.rs
    - claudine/cli/src/commands/wrap/sequence/task_run.rs
    - claudine/cli/tests/l1/agent_text_is_data.rs
    - claudine/cli/tests/l1/codex_app_server.rs
    - claudine/cli/tests/l1/composition_outputs.rs
    - claudine/cli/tests/l1/document_environment_routes.rs
    - claudine/cli/tests/l1/ctx_per_run.rs
    - claudine/cli/tests/l1/handoff_owners.rs
    - claudine/cli/tests/l1/inline_completion_lifecycle.rs
    - claudine/cli/tests/l1/lifecycle_message_drain.rs
    - claudine/cli/tests/l1/main.rs
    - claudine/cli/tests/l1/sequence_cli.rs
    - claudine/cli/tests/l1/sequence_containers.rs
    - claudine/cli/tests/l1/sequence_errors_cli.rs
    - claudine/cli/tests/l1/sequence_groups.rs
    - claudine/cli/tests/l1/sequence_jit.rs
    - claudine/cli/tests/l1/sequence_schema.rs
    - claudine/lib/src/composition/error/mod.rs
    - claudine/lib/src/composition/lifecycle/prewarm/sequence.rs
    - claudine/lib/src/composition/lifecycle/runtime.rs
    - claudine/lib/src/composition/lifecycle/tests/container_runtime.rs
    - claudine/lib/src/composition/lifecycle/tests/mod.rs
    - claudine/lib/src/composition/runtime_state.rs
    - claudine/lib/src/composition/runtime_state/tests.rs
    - claudine/lib/src/composition/sequence/data.rs
    - claudine/lib/src/composition/sequence/preflight/mod.rs
    - claudine/lib/src/composition/sequence/preflight/shape.rs
    - claudine/lib/src/composition/sequence/preflight/tests.rs
    - claudine/lib/src/composition/sequence/task/container_regressions.rs
    - claudine/lib/src/composition/sequence/task/group.rs
    - claudine/lib/src/composition/sequence/task/mod.rs
    - claudine/lib/src/composition/sequence/task/sequence.rs
    - claudine/lib/src/composition/sequence/task/tests.rs
docs_updated_during_phase_5:
    - claudine/docs/providers/dispatch-inventory.json
    - claudine/README.md
    - claudine/docs/topics/flow-control/lifecycle.md
    - claudine/docs/topics/flow-control/sequences.md
    - claudine/features/2026-10-24-sequence-containers/implementation-log.md
    - claudine/features/2026-10-24-sequence-containers/plan.md
docs_created_during_phase_5:
    - claudine/docs/examples/sequence-containers.md
skills_files_updated_during_phase_5:
    - .claude/skills/claudine/SKILL.md
source_files_during_phase_6:
    - claudine/cli/src/commands/compose/prep.rs
    - claudine/cli/src/commands/sequence.rs
    - claudine/cli/src/commands/wrap/composition/pipeline.rs
    - claudine/cli/src/commands/wrap/composition/pipeline/tests.rs
    - claudine/cli/src/commands/wrap/composition/staged_boot.rs
    - claudine/cli/src/commands/wrap/exec/termination/unix.rs
    - claudine/cli/src/commands/wrap/exec/termination/windows.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/error_routing.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/lifecycle_events.rs
    - claudine/cli/src/commands/wrap/sequence/container.rs
    - claudine/cli/src/commands/wrap/sequence/mod.rs
    - claudine/cli/tests/common/review_screen.rs
    - claudine/cli/tests/common/terminal_interrupt.rs
    - claudine/cli/tests/l1/sequence_containers.rs
    - claudine/cli/tests/l1/wrap_sigint.rs
    - claudine/cli/tests/level2/level2_auto_complete_operation_file.rs
    - claudine/cli/tests/level2/level2_lifecycle_control.rs
    - claudine/cli/tests/level2/level2_perf_capture.rs
    - claudine/cli/tests/level2/level2_sequence_container_ctrl_c.rs
    - claudine/cli/tests/level2/level2_sequence_task_stream_capture.rs
    - claudine/cli/tests/level2/main.rs
    - claudine/lib/src/composition/error/mod.rs
    - claudine/lib/src/composition/error/render/mod.rs
    - claudine/lib/src/composition/lifecycle/cancellation.rs
    - claudine/lib/src/composition/lifecycle/executor.rs
    - claudine/lib/src/composition/lifecycle/mod.rs
    - claudine/lib/src/composition/lifecycle/runtime.rs
    - claudine/lib/src/composition/lifecycle/structured_shell.rs
    - claudine/lib/src/composition/lifecycle/tests/container_runtime.rs
    - claudine/lib/src/composition/looping/engine.rs
    - claudine/lib/src/composition/looping/engine/tests/lifecycle_control.rs
    - claudine/lib/src/composition/sequence/task/container_regressions.rs
    - claudine/lib/src/composition/sequence/task/group.rs
    - claudine/lib/src/composition/sequence/task/mod.rs
    - claudine/lib/src/composition/sequence/task/sequence.rs
    - claudine/lib/src/composition/sequence/task/shell.rs
    - claudine/lib/src/diagnostics/registry.rs
docs_updated_during_phase_6:
    - claudine/README.md
    - claudine/docs/providers/dispatch-inventory.json
    - claudine/docs/topics/error-architecture.md
    - claudine/docs/topics/flow-control/lifecycle.md
    - claudine/docs/topics/flow-control/sequences.md
    - claudine/features/2026-10-24-sequence-containers/implementation-log.md
    - claudine/features/2026-10-24-sequence-containers/plan.md
docs_created_during_phase_6: []
skills_files_updated_during_phase_6:
    - .claude/skills/claudine/SKILL.md
source_files_during_phase_7:
    - claudine/lib/src/composition/preflight.rs
    - claudine/lib/src/composition/types.rs
    - claudine/lib/src/harness/shell.rs
    - claudine/lib/src/composition/sequence/preflight/mod.rs
    - claudine/lib/src/composition/sequence/preflight/shape.rs
    - claudine/lib/src/composition/sequence/preflight/containers.rs
    - claudine/lib/src/composition/sequence/preflight/tests.rs
    - claudine/lib/src/composition/sequence/task/sequence.rs
    - claudine/lib/src/composition/sequence/task/tests.rs
    - claudine/cli/src/commands/compose/prep.rs
    - claudine/cli/src/commands/wrap/composition/pipeline.rs
    - claudine/cli/src/commands/wrap/sequence/container.rs
    - claudine/cli/src/commands/wrap/sequence/mod.rs
    - claudine/cli/src/commands/wrap/sequence/iterate.rs
    - claudine/cli/src/commands/wrap/sequence/task_run.rs
    - claudine/cli/tests/l1/sequence_containers.rs
docs_updated_during_phase_7:
    - claudine/docs/topics/flow-control/sequences.md
    - claudine/docs/providers/dispatch-inventory.json
    - claudine/features/2026-10-24-sequence-containers/plan.md
    - claudine/features/2026-10-24-sequence-containers/implementation-log.md
docs_created_during_phase_7: []
skills_files_updated_during_phase_7:
    - .claude/skills/claudine/SKILL.md
    - .claude/skills/os/windows.md
source_files_during_phase_8:
    - claudine/cli/tests/fixtures/frozen_prompts/prompts/_implement/implement-plan.md
    - claudine/cli/tests/fixtures/frozen_prompts/prompts/make-it-markdown.md
    - claudine/cli/tests/fixtures/frozen_prompts/shipped_implement_route/_implement/implement-plan.md
    - claudine/cli/tests/fixtures/shipped_implement_route/_implement/implement-plan.md
    - claudine/cli/tests/fixtures/shipped_implement_route/shipped-hashes.json
    - claudine/cli/tests/l1/main.rs
    - claudine/cli/tests/l1/research_sequence_artifacts.rs
    - claudine/cli/tests/l1/sequence_schema.rs
    - claudine/cli/tests/level3/level3_linux_sequence_ctrl_c.rs
    - claudine/cli/tests/level3/level3_sequence_ctrl_c.rs
    - claudine/cli/tests/level3/level3_windows_sequence_ctrl_c.rs
    - claudine/cli/tests/prompts/prompt_guide_defects.rs
    - claudine/cli/tests/prompts/shipped_implement_router.rs
    - claudine/cli/tests/prompts/shipped_prompt_contract.rs
    - claudine/cli/tests/prompts/shipped_prompts.rs
    - claudine/gen/tests/l1/agent_errors_check.rs
    - claudine/lib/src/composition/error/mod.rs
    - claudine/lib/src/composition/lifecycle/prewarm/sequence.rs
    - claudine/lib/src/composition/prepare.rs
    - claudine/lib/src/composition/sequence/mod.rs
    - claudine/lib/src/composition/sequence/task/tests.rs
    - claudine/lib/src/composition/types.rs
    - claudine/lib/tests/l1/agent_errors_fleet.rs
    - claudine/justfile
docs_updated_during_phase_8:
    - biscuit-tui/docs/components/_build.md
    - biscuit-tui/docs/style-guide/_style-guide.md
    - claudine/docs/getting-started/index.md
    - claudine/docs/research/_TEMPLATE.md
    - claudine/docs/research/acp/_fleet.md
    - claudine/docs/research/agent-cli/_fleet.md
    - claudine/docs/research/agent-errors/_fleet.md
    - claudine/docs/research/agent-logging/_fleet.md
    - claudine/docs/research/agent-models/_fleet.md
    - claudine/docs/research/agent-permissions/_fleet.md
    - claudine/docs/research/hooks/_fleet.md
    - claudine/docs/research/local_runners/_fleet.md
    - claudine/docs/research/mcp/_fleet.md
    - claudine/docs/research/memory/_fleet.md
    - claudine/docs/research/model-config/_fleet.md
    - claudine/docs/research/model-serving-api-standards/_research.md
    - claudine/docs/research/non-interactive-sessions/_fleet.md
    - claudine/docs/research/plugins/_fleet.md
    - claudine/docs/research/reasoning-level/_fleet.md
    - claudine/docs/research/resume/_fleet.md
    - claudine/docs/research/signals/_fleet.md
    - claudine/docs/research/skills/_fleet.md
    - claudine/docs/research/slash-commands/_fleet.md
    - claudine/docs/research/steering/_fleet.md
    - claudine/docs/research/subagents/_fleet.md
    - claudine/docs/research/summary/acp.md
    - claudine/docs/research/summary/agent-cli.md
    - claudine/docs/research/summary/agent-logging.md
    - claudine/docs/research/summary/agent-models.md
    - claudine/docs/research/summary/agent-permissions.md
    - claudine/docs/research/summary/agent-plugins.md
    - claudine/docs/research/summary/agent-skills.md
    - claudine/docs/research/summary/hooks.md
    - claudine/docs/research/summary/local-runners.md
    - claudine/docs/research/summary/model-config.md
    - claudine/docs/research/summary/non-interactive-sessions.md
    - claudine/docs/research/summary/session-resumption.md
    - claudine/docs/research/summary/slash-commands.md
    - claudine/docs/research/summary/steering.md
    - claudine/docs/research/summary/subagents.md
    - claudine/docs/research/summary/system-prompt.md
    - claudine/docs/research/system-prompt/_fleet.md
    - claudine/docs/research/usage/_fleet.md
    - claudine/docs/topics/composition.md
    - claudine/docs/topics/execution-flow.md
    - claudine/docs/topics/flow-control/flow-control-reference.md
    - claudine/docs/topics/flow-control/flow-control.md
    - claudine/docs/topics/flow-control/lifecycle.md
    - claudine/docs/topics/flow-control/looping.md
    - claudine/docs/topics/flow-control/sequences.md
    - claudine/docs/topics/frontmatter-properties.md
    - claudine/docs/topics/pre-flight-checks.md
    - claudine/docs/topics/schemas.md
    - claudine/docs/topics/timeouts.md
    - claudine/features/2026-10-24-sequence-containers/implementation-log.md
    - claudine/features/2026-10-24-sequence-containers/plan.md
    - darkmatter/docs/inline/preflight-checks.md
    - darkmatter/docs/research/parsing/overview.md
    - prompts/_implement/implement-plan.md
    - prompts/_prompt.md
    - prompts/_reviews/feature-review.md
    - prompts/review-loop.md
    - reaper/docs/research/schemas.md
    - schematic/docs/research/project-mgmt/canonical.md
    - claudine/docs/research/summary/mcp.md
    - claudine/docs/research/summary/plugins.md
docs_created_during_phase_8:
    - claudine/docs/research/_local-runner-tasks.yaml
    - claudine/docs/research/_provider-tasks.yaml
    - claudine/docs/research/acp/_research.md
    - claudine/docs/research/agent-cli/_research.md
    - claudine/docs/research/agent-errors/_research.md
    - claudine/docs/research/agent-logging/_research.md
    - claudine/docs/research/agent-models/_research.md
    - claudine/docs/research/agent-permissions/_research.md
    - claudine/docs/research/hooks/_research.md
    - claudine/docs/research/local_runners/_research.md
    - claudine/docs/research/mcp/_research.md
    - claudine/docs/research/memory/_research.md
    - claudine/docs/research/model-config/_research.md
    - claudine/docs/research/non-interactive-sessions/_research.md
    - claudine/docs/research/plugins/_research.md
    - claudine/docs/research/reasoning-level/_research.md
    - claudine/docs/research/resume/_research.md
    - claudine/docs/research/signals/_research.md
    - claudine/docs/research/skills/_research.md
    - claudine/docs/research/slash-commands/_research.md
    - claudine/docs/research/steering/_research.md
    - claudine/docs/research/subagents/_research.md
    - claudine/docs/research/system-prompt/_research.md
    - claudine/docs/research/usage/_research.md
skills_files_updated_during_phase_8:
    - .claude/skills/claudine/SKILL.md
    - .claude/skills/claudine/architecture.md
    - .claude/skills/claudine/cli-commands.md
    - .claude/skills/claudine/cli-reference.md
    - .claude/skills/claudine/research-contracts.md
    - .claude/skills/claudine/timeline.md
source_files_during_phase_9:
    - claudine/lib/src/composition/sequence/model.rs
    - claudine/lib/src/composition/sequence/task/mod.rs
    - claudine/cli/src/commands/wrap/sequence/jit.rs
    - claudine/cli/src/commands/wrap/sequence/iterate.rs
    - claudine/cli/src/commands/wrap/sequence/mod.rs
    - darkmatter/lib/tests/l1/current_root_documentation_contract.rs
docs_updated_during_phase_9:
    - claudine/features/2026-10-24-sequence-containers/plan.md
    - claudine/features/2026-10-24-sequence-containers/implementation-log.md
    - claudine/features/2026-10-24-sequence-containers/spec.md
    - claudine/docs/topics/state-management/context-variables.md
docs_created_during_phase_9: []
skills_files_updated_during_phase_9:
    - .claude/skills/messenger/research-contract.md
    - .claude/skills/claudine/cli-commands.md
source_code:
    - claudine/cli/src/args.rs
    - claudine/cli/src/argv/partition.rs
    - claudine/cli/src/argv/partition/tests.rs
    - claudine/cli/src/commands/compose/prep.rs
    - claudine/cli/src/commands/mod.rs
    - claudine/cli/src/commands/sequence.rs
    - claudine/cli/src/commands/wrap/composition/pipeline.rs
    - claudine/cli/src/commands/wrap/composition/pipeline/tests.rs
    - claudine/cli/src/commands/wrap/composition/preflight.rs
    - claudine/cli/src/commands/wrap/composition/staged_boot.rs
    - claudine/cli/src/commands/wrap/exec/spawn/captured.rs
    - claudine/cli/src/commands/wrap/exec/spawn/inherited.rs
    - claudine/cli/src/commands/wrap/exec/spawn/retained.rs
    - claudine/cli/src/commands/wrap/exec/spawn/semantic.rs
    - claudine/cli/src/commands/wrap/exec/termination/unix.rs
    - claudine/cli/src/commands/wrap/exec/termination/windows.rs
    - claudine/cli/src/commands/wrap/exec/wiring/session.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/control_dispatch.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/error_routing.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/lifecycle_events.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/retry_resume.rs
    - claudine/cli/src/commands/wrap/harness_orch/shell_options.rs
    - claudine/cli/src/commands/wrap/sequence/container.rs
    - claudine/cli/src/commands/wrap/sequence/iterate.rs
    - claudine/cli/src/commands/wrap/sequence/jit.rs
    - claudine/cli/src/commands/wrap/sequence/jit/tests.rs
    - claudine/cli/src/commands/wrap/sequence/mod.rs
    - claudine/cli/src/commands/wrap/sequence/review.rs
    - claudine/cli/src/commands/wrap/sequence/review/tests.rs
    - claudine/cli/src/commands/wrap/sequence/task_run.rs
    - claudine/cli/src/commands/wrap/sequence/task_run/tests.rs
    - claudine/cli/src/completion/engine/tests.rs
    - claudine/cli/src/completion/schema_completion/tests.rs
    - claudine/cli/src/main.rs
    - claudine/cli/src/telemetry.rs
    - claudine/cli/tests/common/drain_interrupt.rs
    - claudine/cli/tests/common/owned_value_options.rs
    - claudine/cli/tests/common/review_screen.rs
    - claudine/cli/tests/common/terminal_interrupt.rs
    - claudine/cli/tests/fixtures/frozen_prompts/prompts/_implement/implement-plan.md
    - claudine/cli/tests/fixtures/frozen_prompts/prompts/make-it-markdown.md
    - claudine/cli/tests/fixtures/frozen_prompts/prompts/review-loop.md
    - claudine/cli/tests/fixtures/frozen_prompts/shipped_implement_route/_implement/implement-plan.md
    - claudine/cli/tests/fixtures/shipped_implement_route/_implement/implement-plan.md
    - claudine/cli/tests/fixtures/shipped_implement_route/shipped-hashes.json
    - claudine/cli/tests/l1/agent_text_is_data.rs
    - claudine/cli/tests/l1/codex_app_server.rs
    - claudine/cli/tests/l1/completion_ownership.rs
    - claudine/cli/tests/l1/compose_caller_file_provenance.rs
    - claudine/cli/tests/l1/compose_frontmatter_model.rs
    - claudine/cli/tests/l1/compose_removed_validation_keys.rs
    - claudine/cli/tests/l1/composition_outputs.rs
    - claudine/cli/tests/l1/ctx_per_run.rs
    - claudine/cli/tests/l1/document_environment_routes.rs
    - claudine/cli/tests/l1/error_guards/transport-allow.toml
    - claudine/cli/tests/l1/handoff_owners.rs
    - claudine/cli/tests/l1/inline_completion_lifecycle.rs
    - claudine/cli/tests/l1/launch_trigger_schemas.rs
    - claudine/cli/tests/l1/lifecycle_downgrade_outcome.rs
    - claudine/cli/tests/l1/lifecycle_message_drain.rs
    - claudine/cli/tests/l1/lifecycle_message_drain_interrupt.rs
    - claudine/cli/tests/l1/lifecycle_mixed_span_validation.rs
    - claudine/cli/tests/l1/loop_cli.rs
    - claudine/cli/tests/l1/main.rs
    - claudine/cli/tests/l1/preflight_execution_parity.rs
    - claudine/cli/tests/l1/provider_tail_notice.rs
    - claudine/cli/tests/l1/provider_tail_ownership.rs
    - claudine/cli/tests/l1/research_sequence_artifacts.rs
    - claudine/cli/tests/l1/sequence_cli.rs
    - claudine/cli/tests/l1/sequence_containers.rs
    - claudine/cli/tests/l1/sequence_ctrl_c_windows.rs
    - claudine/cli/tests/l1/sequence_errors_cli.rs
    - claudine/cli/tests/l1/sequence_groups.rs
    - claudine/cli/tests/l1/sequence_jit.rs
    - claudine/cli/tests/l1/sequence_magic_reference.rs
    - claudine/cli/tests/l1/sequence_overlay_pty.rs
    - claudine/cli/tests/l1/sequence_perf.rs
    - claudine/cli/tests/l1/sequence_prompt_property.rs
    - claudine/cli/tests/l1/sequence_schema.rs
    - claudine/cli/tests/l1/sequence_shell_templates.rs
    - claudine/cli/tests/l1/sequence_sources_cli.rs
    - claudine/cli/tests/l1/tts_prewarm.rs
    - claudine/cli/tests/l1/wrap_compose_validation.rs
    - claudine/cli/tests/l1/wrap_sigint.rs
    - claudine/cli/tests/level2/level2_auto_complete_operation_file.rs
    - claudine/cli/tests/level2/level2_lifecycle_control.rs
    - claudine/cli/tests/level2/level2_perf_capture.rs
    - claudine/cli/tests/level2/level2_sequence_container_ctrl_c.rs
    - claudine/cli/tests/level2/level2_sequence_task_stream_capture.rs
    - claudine/cli/tests/level2/main.rs
    - claudine/cli/tests/level3/level3_linux_sequence_ctrl_c.rs
    - claudine/cli/tests/level3/level3_sequence_ctrl_c.rs
    - claudine/cli/tests/level3/level3_windows_sequence_ctrl_c.rs
    - claudine/cli/tests/prompts/prompt_guide_defects.rs
    - claudine/cli/tests/prompts/shipped_implement_router.rs
    - claudine/cli/tests/prompts/shipped_prompt_contract.rs
    - claudine/cli/tests/prompts/shipped_prompts.rs
    - claudine/gen/tests/l1/agent_errors_check.rs
    - claudine/justfile
    - claudine/lib/src/composition/error/mod.rs
    - claudine/lib/src/composition/error/render/lifecycle.rs
    - claudine/lib/src/composition/error/render/mod.rs
    - claudine/lib/src/composition/error/tests.rs
    - claudine/lib/src/composition/lifecycle/actions.rs
    - claudine/lib/src/composition/lifecycle/audio.rs
    - claudine/lib/src/composition/lifecycle/bindings.rs
    - claudine/lib/src/composition/lifecycle/bindings/containers.rs
    - claudine/lib/src/composition/lifecycle/bindings/rename.rs
    - claudine/lib/src/composition/lifecycle/bindings/rename/tests.rs
    - claudine/lib/src/composition/lifecycle/bindings/tests.rs
    - claudine/lib/src/composition/lifecycle/cancellation.rs
    - claudine/lib/src/composition/lifecycle/context/tests.rs
    - claudine/lib/src/composition/lifecycle/executor.rs
    - claudine/lib/src/composition/lifecycle/executor/tests/event_time_interpolation.rs
    - claudine/lib/src/composition/lifecycle/executor/tests/filesystem_lookup.rs
    - claudine/lib/src/composition/lifecycle/executor/tests/mod.rs
    - claudine/lib/src/composition/lifecycle/executor/tests/runtime_set.rs
    - claudine/lib/src/composition/lifecycle/mod.rs
    - claudine/lib/src/composition/lifecycle/parse.rs
    - claudine/lib/src/composition/lifecycle/prewarm/classify/tests.rs
    - claudine/lib/src/composition/lifecycle/prewarm/sequence.rs
    - claudine/lib/src/composition/lifecycle/prewarm/sequence/tests.rs
    - claudine/lib/src/composition/lifecycle/prewarm/tests.rs
    - claudine/lib/src/composition/lifecycle/runtime.rs
    - claudine/lib/src/composition/lifecycle/structured_shell.rs
    - claudine/lib/src/composition/lifecycle/tests/audio_emission.rs
    - claudine/lib/src/composition/lifecycle/tests/container_runtime.rs
    - claudine/lib/src/composition/lifecycle/tests/grammar.rs
    - claudine/lib/src/composition/lifecycle/tests/guard_runtime.rs
    - claudine/lib/src/composition/lifecycle/tests/mod.rs
    - claudine/lib/src/composition/lifecycle/tests/nested_span.rs
    - claudine/lib/src/composition/lifecycle/tests/parse_config.rs
    - claudine/lib/src/composition/looping/actions/tests.rs
    - claudine/lib/src/composition/looping/engine.rs
    - claudine/lib/src/composition/looping/engine/tests/lifecycle_control.rs
    - claudine/lib/src/composition/looping/expression.rs
    - claudine/lib/src/composition/preflight.rs
    - claudine/lib/src/composition/preflight/tests.rs
    - claudine/lib/src/composition/prepare.rs
    - claudine/lib/src/composition/prepare/tests.rs
    - claudine/lib/src/composition/reserved.rs
    - claudine/lib/src/composition/runtime_state.rs
    - claudine/lib/src/composition/runtime_state/tests.rs
    - claudine/lib/src/composition/sequence/data.rs
    - claudine/lib/src/composition/sequence/formal.rs
    - claudine/lib/src/composition/sequence/grammar.rs
    - claudine/lib/src/composition/sequence/mod.rs
    - claudine/lib/src/composition/sequence/model.rs
    - claudine/lib/src/composition/sequence/normalize.rs
    - claudine/lib/src/composition/sequence/preflight/containers.rs
    - claudine/lib/src/composition/sequence/preflight/mod.rs
    - claudine/lib/src/composition/sequence/preflight/runtime_fixture.rs
    - claudine/lib/src/composition/sequence/preflight/shape.rs
    - claudine/lib/src/composition/sequence/preflight/tests.rs
    - claudine/lib/src/composition/sequence/reserved.rs
    - claudine/lib/src/composition/sequence/source.rs
    - claudine/lib/src/composition/sequence/task/container_regressions.rs
    - claudine/lib/src/composition/sequence/task/group.rs
    - claudine/lib/src/composition/sequence/task/mod.rs
    - claudine/lib/src/composition/sequence/task/sequence.rs
    - claudine/lib/src/composition/sequence/task/shell.rs
    - claudine/lib/src/composition/sequence/task/templates.rs
    - claudine/lib/src/composition/sequence/task/tests.rs
    - claudine/lib/src/composition/sequence/tests.rs
    - claudine/lib/src/composition/types.rs
    - claudine/lib/src/diagnostics/registry.rs
    - claudine/lib/src/harness/shell.rs
    - claudine/lib/tests/l1/agent_errors_fleet.rs
    - claudine/lib/tests/l1/claudine_schemas.rs
    - claudine/lib/tests/l1/lifecycle_lift_ready_guard.rs
    - claudine/schemas/action.yaml
    - claudine/schemas/claudine.yaml
    - claudine/schemas/lifecycle.yaml
    - darkmatter/cli/tests/l1/claudine_schema_bundle.rs
    - darkmatter/lib/src/markdown/compose/context/effective_state.rs
    - darkmatter/lib/src/markdown/compose/expression/binding.rs
    - darkmatter/lib/src/markdown/compose/frontmatter_interpolation.rs
    - darkmatter/lib/tests/l1/current_root_documentation_contract.rs
    - darkmatter/lib/tests/l1/shipped_schema_corpus.rs
    - messenger/cli/src/research.rs
    - messenger/cli/src/research_lifecycle.rs
    - messenger/cli/tests/research_lifecycle_cli.rs
    - messenger/justfile
    - messenger/lib/src/research/refresh/check.rs
    - messenger/lib/src/research/refresh/cleanup.rs
    - messenger/lib/src/research/refresh/mod.rs
    - messenger/lib/src/research/refresh/prepare.rs
    - messenger/lib/src/research/refresh/promote.rs
    - messenger/lib/src/research/refresh/runner.rs
    - messenger/lib/src/research/refresh/select.rs
    - messenger/lib/src/research/refresh/state.rs
    - messenger/lib/tests/research_refresh.rs
documentation:
    - biscuit-tui/docs/components/_build.md
    - biscuit-tui/docs/style-guide/_style-guide.md
    - claudine/README.md
    - claudine/docs/cli/index.md
    - claudine/docs/cli/sequence.md
    - claudine/docs/dependencies.md
    - claudine/docs/examples/sequence-containers.md
    - claudine/docs/getting-started/index.md
    - claudine/docs/providers/dispatch-inventory.json
    - claudine/docs/research/_TEMPLATE.md
    - claudine/docs/research/_local-runner-tasks.yaml
    - claudine/docs/research/_provider-tasks.yaml
    - claudine/docs/research/acp/_fleet.md
    - claudine/docs/research/acp/_research.md
    - claudine/docs/research/agent-cli/_fleet.md
    - claudine/docs/research/agent-cli/_research.md
    - claudine/docs/research/agent-errors/_fleet.md
    - claudine/docs/research/agent-errors/_research.md
    - claudine/docs/research/agent-logging/_fleet.md
    - claudine/docs/research/agent-logging/_research.md
    - claudine/docs/research/agent-models/_fleet.md
    - claudine/docs/research/agent-models/_research.md
    - claudine/docs/research/agent-permissions/_fleet.md
    - claudine/docs/research/agent-permissions/_research.md
    - claudine/docs/research/hooks/_fleet.md
    - claudine/docs/research/hooks/_research.md
    - claudine/docs/research/local_runners/_fleet.md
    - claudine/docs/research/local_runners/_research.md
    - claudine/docs/research/mcp/_fleet.md
    - claudine/docs/research/mcp/_research.md
    - claudine/docs/research/memory/_fleet.md
    - claudine/docs/research/memory/_research.md
    - claudine/docs/research/model-config/_fleet.md
    - claudine/docs/research/model-config/_research.md
    - claudine/docs/research/model-serving-api-standards/_research.md
    - claudine/docs/research/non-interactive-sessions/_fleet.md
    - claudine/docs/research/non-interactive-sessions/_research.md
    - claudine/docs/research/plugins/_fleet.md
    - claudine/docs/research/plugins/_research.md
    - claudine/docs/research/reasoning-level/_fleet.md
    - claudine/docs/research/reasoning-level/_research.md
    - claudine/docs/research/resume/_fleet.md
    - claudine/docs/research/resume/_research.md
    - claudine/docs/research/signals/_fleet.md
    - claudine/docs/research/signals/_research.md
    - claudine/docs/research/skills/_fleet.md
    - claudine/docs/research/skills/_research.md
    - claudine/docs/research/slash-commands/_fleet.md
    - claudine/docs/research/slash-commands/_research.md
    - claudine/docs/research/steering/_fleet.md
    - claudine/docs/research/steering/_research.md
    - claudine/docs/research/subagents/_fleet.md
    - claudine/docs/research/subagents/_research.md
    - claudine/docs/research/summary/acp.md
    - claudine/docs/research/summary/agent-cli.md
    - claudine/docs/research/summary/agent-logging.md
    - claudine/docs/research/summary/agent-models.md
    - claudine/docs/research/summary/agent-permissions.md
    - claudine/docs/research/summary/agent-plugins.md
    - claudine/docs/research/summary/agent-skills.md
    - claudine/docs/research/summary/hooks.md
    - claudine/docs/research/summary/local-runners.md
    - claudine/docs/research/summary/mcp.md
    - claudine/docs/research/summary/model-config.md
    - claudine/docs/research/summary/non-interactive-sessions.md
    - claudine/docs/research/summary/plugins.md
    - claudine/docs/research/summary/session-resumption.md
    - claudine/docs/research/summary/slash-commands.md
    - claudine/docs/research/summary/steering.md
    - claudine/docs/research/summary/subagents.md
    - claudine/docs/research/summary/system-prompt.md
    - claudine/docs/research/system-prompt/_fleet.md
    - claudine/docs/research/system-prompt/_research.md
    - claudine/docs/research/usage/_fleet.md
    - claudine/docs/research/usage/_research.md
    - claudine/docs/topics/completions/shell-completions.md
    - claudine/docs/topics/composition.md
    - claudine/docs/topics/error-architecture.md
    - claudine/docs/topics/execution-flow.md
    - claudine/docs/topics/flow-control/flow-control-reference.md
    - claudine/docs/topics/flow-control/flow-control.md
    - claudine/docs/topics/flow-control/lifecycle.md
    - claudine/docs/topics/flow-control/looping.md
    - claudine/docs/topics/flow-control/sequences.md
    - claudine/docs/topics/frontmatter-properties.md
    - claudine/docs/topics/pre-flight-checks.md
    - claudine/docs/topics/schemas.md
    - claudine/docs/topics/state-management/context-variables.md
    - claudine/docs/topics/timeouts.md
    - claudine/features/2026-10-24-sequence-containers/implementation-log.md
    - claudine/features/2026-10-24-sequence-containers/plan.md
    - claudine/features/2026-10-24-sequence-containers/spec.md
    - claudine/schemas/README.md
    - darkmatter/docs/inline/preflight-checks.md
    - darkmatter/docs/research/parsing/overview.md
    - darkmatter/docs/topics/darkmatter-expressions.md
    - docs/dependencies.md
    - messenger/cli/README.md
    - messenger/docs/user-guide.md
    - prompts/_implement/implement-plan.md
    - prompts/_prompt.md
    - prompts/_reviews/feature-review.md
    - prompts/review-loop.md
    - reaper/docs/research/schemas.md
    - schematic/docs/research/project-mgmt/canonical.md
completed_phase: 9
implemented: true
---

# Implementation Log for 2026-10-24-sequence-containers (9 phases)

## Phase 1

Preconditions, rulings, and spikes. No product code changes in this phase;
entries below record the dependency status, spike results, rulings, and the
pre-change baseline.

### Task 1.1 — Dependency check

All three dependencies are **implemented in the working tree** (dependency 2
substantially via uncommitted changes, which count as "in the tree"). No
blockers under Rule 1.

**2026-10-20-lifecycle-grammar — IMPLEMENTED (committed; `implemented: true`;
log ends "Implementation complete, ready for review")**

- `LifecycleSignal { Initialize, Start, Success, Blocked, Failure, Finalize,
  Loop }` at `claudine/lib/src/composition/lifecycle/mod.rs:273-297`; scope ids
  `claudine.lifecycle.*` at `bindings.rs:122-133`.
- `loop` root is a host-declared global (`bindings.rs:360-398`,
  `composition_host_globals`); retired `_loop_*` spellings map to replacements
  (`composition/reserved.rs:10-16`, `RETIRED_LOOP_NAMES`) and are rejected at
  `bindings.rs:593,624`.
- Loop-gate grammar (`gate:` list vs loose verbs) parsed at
  `lifecycle/parse.rs:138,329-340`; `initialize` shell prohibition at
  `parse.rs:147-163`.
- Caveat for later phases: the member set is `loop.count, loop.is_first,
  loop.is_last, loop.last_output, loop.last_exit_code` (`looping/expression.rs:50-51`)
  — there is **no `loop.index`**; the 1-based member is `loop.count`.

**2026-10-23-shell-template-approval — IMPLEMENTED (spec `implemented: true`;
final phases uncommitted in the working tree: `lifecycle/shell_template.rs`,
`structured_shell.rs`, preflight shape/reapproval, and their tests are all
present as `A`/`AM` files)**

- Late-value classification: `lifecycle/shell_template.rs:39-41`
  `LIFECYCLE_HOST_GLOBALS = ["loop", "err", "timing", "group"]`;
  `sequence/preflight/mod.rs:1186` documents the late set (`outputs`, `loop`,
  `group`, step values derived from them).
- First-task resolved-value approval: `sequence/preflight/shape.rs:97-121`
  `TaskPosition::{First, Later}` with `child()` — already group-aware ("a
  group's legs take the group's position"); `preflight/mod.rs:1183-1195`.
- Template (shape) approval distinct from exact bytes:
  `shell_template.rs:44-55` `LifecycleShellMode::{ResolvedBytes, Template}`;
  `shape.rs:136-148` `SiteMode`; re-approval of resolved text at
  `reapproval.rs:1-11`.
- Caveats that are **this feature's in-scope work, not blockers**: the current
  late root is `outputs`, not `seq.outputs` (rename is spec §7/requirement 3);
  nested sequences and container terminal events have **no runtime surface**
  yet; `TaskPosition::child` already encodes the all-first-step ancestry rule
  the spec describes — reuse it.

**2026-10-18-fm-namespace — IMPLEMENTED (committed, in `_completed`)**

- `fm` root + resolution order at
  `darkmatter/lib/src/markdown/compose/expression/binding.rs:842-858`
  (reserved namespace → `fm.*` document path → runtime host global → document
  property); `RESERVED_ROOTS` catalog pinned to `["fm", "ctx", "env",
  "current"]` (`expression/catalog/roots.rs:62-92,104-107`).
- Claudine side: `composition/reserved.rs:19` `EXPRESSION_RESERVED_ROOTS`;
  `fm.loop` reads the `loop:` config even beside the runtime root
  (`bindings.rs:350-353`).
- Material caveat: the sequence-containers spec's "reserved by
  2026-10-18-fm-namespace" phrasing overstates the current state — `loop` and
  `group` are **Claudine host-declared globals** (`bindings.rs:197,203,223`),
  not Darkmatter reserved roots, and **`seq` does not exist anywhere yet**.
  The reservation *mechanism* (root wins over frontmatter; `fm.<name>` escape)
  is present and tested; building the `seq` root in the binding catalog is
  this feature's own work (task 4.1), as the spec itself says.

### Task 1.2 — Messenger lock inventory

**Lock crate choice (Ruling 6 default satisfied with zero new dependencies):**
reuse the standard library's advisory lock through Messenger's own
`PublicationLock` — `messenger/lib/src/research/publish/fsutil.rs:116-147`
(`try_acquire`; "flock on Unix, LockFileEx on Windows; the OS releases it when
the process dies"). `cleanup.rs:54-63` already probes liveness the same way
(`file.try_lock().is_err()`). The new fleet lock is
`PublicationLock::try_acquire(state.fleet_lock())` held for the whole
execution. If a crate is ever preferred, `fs4 v0.13.1` is already a runtime
dependency of `playa` and `biscuit-clipboard-service` (Cargo.lock:4349;
`docs/dependencies.md:149,267` blesses it); `fs2` (queue only) and `lock_api`
(in-memory only) are ruled out.

**Ledger readers to delete (Phase 2, Wave 2 inventory):**

- `messenger/lib/src/research/refresh/state.rs` — `LedgerView` 205-219,
  `Allowance` 221-225, `StateArea::ledger()` 369-383, `apply_ledger()` 386-410,
  `RunRecord::ledger_runs` 181-184, `StateArea::fleet_lock()` 273-276 (path to
  repurpose), module doc 4-7, test `only_an_active_run_takes_the_ledger_resting_state`
  450-478.
- `prepare.rs` — `resume()` ledger apply 149-150, `RefreshError::Ledger`
  guidance 172-183, `ledger_runs` update 193; `prepared()` emits
  `claudine budget init … --exclusive-lock ../../fleet.lock` 254-263 and passes
  `--budget-ledger` 267.
- `check.rs` — `Evaluation::ledger` 50, `state.ledger(record)` 117,
  `apply_ledger` in `check_run` 366, `Exhausted|Interrupted` in `passed` 389.
- `promote.rs` — `apply_ledger` 88-91, `RefreshError::Ledger` 100-108,
  `reject()` ledger read 281-282.
- `select.rs` — `blocking_runs()` ledger read 187-191.
- `cleanup.rs` — `ledger_lock_held()` 53-63 and its use at 88 (replace with
  fleet-lock liveness), doc 5-6.
- `mod.rs` — `RefreshError::Ledger` 64-65.
- `messenger/cli/src/research_lifecycle.rs` — `LedgerView` import 22,
  `RefreshError::Ledger` arm 155, `RunRow.ledger` 341/359/374, ledger render
  399-407, "within its existing budget" resume help 53-54;
  `messenger/cli/src/research.rs:116` ("budget ledgers" line).
- Tests: `messenger/lib/tests/research_refresh.rs` 572-576, 966-991, 1024-1057,
  1059-1110, 1169-1187, 1205.
- Docs: `messenger/docs/user-guide.md` 764, 811, 813, 825.

**Must keep (Phase 2 guard list):** run-record validation (`state.rs:339-367,
412-420`); stage checks (`check.rs:1-13,97-203,246-333,357-364`); resume limit
(`MAX_RECOVERY_ATTEMPTS` `state.rs:191`, `prepare.rs:159-161`,
`RefreshError::RecoveryLimit` `mod.rs:62-63`, one-open-run-per-platform
`select.rs:175-197`, `prepare.rs:162-171`); preparation lock (`prepare.rs:69,147,
231-237`, `RefreshError::PrepareBusy`); promotion refusal (`promote.rs:92-135,
225-270,283-290`); cleanup protections (`cleanup.rs:84-90,146-158`).

### Task 1.3 — Spike A result: runtime-state branching

**Verdict: YES for branch/merge (mechanism exists, purely in-memory); PARTIAL
for nested-sequence legs** — the mutation merge carries a nested-sequence leg
unchanged, but the *result* path does not: leg results are captured as
`stdout: String` and the group commits `Value::Array(Vec<String>)`, so a
nested sequence's native array (and the leg-name-keyed dictionary of §8) needs
a new carrier through `TaskOutcome`/`PrimaryOutcome`, not a new merge type.

**Branch/merge API to use in Phase 5 (all exists today):**

- `RuntimeState` (`claudine/lib/src/composition/runtime_state.rs:89`) is a
  `Mutex<RuntimeSnapshot>` (deliberately not `Clone`); `RuntimeSnapshot`
  (`:73-78`, `Clone`) holds `mutations: FrontmatterMap` (IndexMap) and
  `outputs: Vec<Value>` — no disk, no `Arc` internals.
- Branch: `RuntimeState::snapshot()` (`:112`) + `from_snapshot()` (`:105`);
  precedent `group.rs:298-315` (one private cell per leg:
  `Arc::new(RuntimeState::from_snapshot(base.clone()))`).
- Second channel that must branch too: the live document cell —
  `StackExecutionContext::live_frontmatter_snapshot()` (`executor.rs:931`),
  `live_cells` (`group.rs:324-328`), attached per leg via
  `with_private_cells` (`executor.rs:915-925`, called `group.rs:384-385`);
  `TaskExecution.runtime` swapped to the leg buffer (`group.rs:391`), and
  `prompt_request` propagates that `Arc` into the launched document
  (`sequence/task/mod.rs:775`).
- Merge: there is no state-level merge method — the merge is a **replay of
  recorded mutation deltas**: `TaskExecution::mutation_delta`
  (`task/mod.rs:1311-1326`) diffs the private cell into
  `Vec<RuntimeMutation>` (`sequence/model.rs:367-374`, carried on
  `TaskOutcome.mutations` `task/mod.rs:171`), replayed in declaration order by
  `TaskExecution::merge_parallel_mutations` (`group.rs:494-532`) via
  `runtime.set(...)` over `group.tasks.iter().zip(runs)`. Later-declared leg
  wins; contested keys `emit_warn` (`group.rs:507-516`). Works for a
  nested-sequence leg unchanged.

**Nested-sequence extension points (Phase 5.2/5.3):**

1. `TaskOutcome.stdout: String` (`task/mod.rs:167`) is the only result field
   `fold_parallel_results` reads (`group.rs:446` builds
   `Value::String(run.outcome.stdout)`, commits
   `PrimaryOutcome::with_group_output(Value::Array(entries))` `group.rs:477`)
   — becomes a dictionary keyed by leg name holding native results.
2. `PrimaryOutcome.group_output: Option<Value>` (`task/mod.rs:1398`),
   `StageOutcome.group_output` (`:1367`), commit at `run_validated`
   (`task/mod.rs:459-472`) — the plumbing is type-general; a
   sequence-executable task's `Vec<Value>` result must reach
   `RuntimeState::append_output_entry` (`runtime_state.rs:189`) instead of the
   `Value::String(stdout)` fallback (`task/mod.rs:470`).
3. `PreflightAction` (`sequence/preflight/shape.rs:270-294`) has no `Sequence`
   variant — add one plus dispatch in `TaskExecution::run_primary`.
4. `OutputEntry` (`sequence/model.rs:345-362`) is string-only
   (`Single(String) | Nested(Vec<String>)`).

**Risks/gotchas:**

- `serde_json::Map` is **not insertion-ordered** here (no `preserve_order`
  feature anywhere) — build §8 dictionaries with `indexmap::IndexMap` (as
  `FrontmatterMap` does) or keep order recoverable from `group.tasks`.
- Isolation is memory-only: `set_frontmatter` reaches disk and prompt sources
  are re-read at preparation boundaries (`group.rs:319-323`) — docs must state
  this (requirement 2).
- Never derive `Clone` on `RuntimeState`; branch only via
  snapshot/from_snapshot. Mutex poisoning is `expect`ed (`runtime_state.rs:47`,
  `group.rs:65`) — keep scoped-thread panic propagation.
- An interrupted parallel group commits neither mutations nor outputs today
  (`group.rs:467-470`); the cancelled/`fail_fast` staging ("retain private leg
  state until settlement") extends this.
- Ordering is structural (index-keyed `slots` + `zip(group.tasks, runs)`;
  merge on one thread after `thread::scope` joins) — preserve both when
  generalizing to container legs.

### Task 1.4 — Spike B result: guard and runtime re-entry

**Verdict: CONFIRMED** — the shared runtime already re-enters on `retry`/
`resume` without requesting `finalize`. `decide_lifecycle_transition`
(`claudine/lib/src/composition/lifecycle/runtime.rs:371-382`) returns
`Reenter(dispatch)` for Retry/Resume; `Finalize` is requested only by
`terminal_or_finalize` for `Success|Blocked|Failure` when `!finalize_emitted`
(`runtime.rs:421-438`) or an evaluation error (`:360-369`). The CLI consumer
makes it explicit: `drive_terminal_recovery` dispatches terminal control
*before* finalizing — "Dispatch it before finalizing so a re-entry skips
finalize for this iteration" (`cli/src/commands/wrap/harness_orch/loop_control/control_dispatch.rs:433-435`);
on `Reenter` it resets the guard (`reset_for_next_iteration`, `:156,196`) and
never reaches `run_finalize_with_recovery` (`:290-350`). Exactly-once lives in
`LifecycleRunGuard::record_event_emission` (`lifecycle/mod.rs:619-624`).

**Guard facts:** `LifecycleRunGuard` (`lifecycle/mod.rs:486-532`) holds
`config: Cow` (repointable for proxy `:850-852`), ctx, emitter, five dedup
booleans + `terminal_signal` (terminals mutually exclusive `:563-570,610-618`);
drop safety net emits `Blocked`/`Failure` when `start_emitted &&
!terminal_emitted` (`:921-932`), `defuse` suppresses (`:804-806`). Constructed
at `cli composition/pipeline.rs:1739`, `wrapper_stages.rs:602`,
`compose/prep.rs:1910`. One guard per primitive, not serializable.

**Change list for `cancelled`:**

(a) Lift-ready parser slice (Phase 3.1), guarded by
`lib/tests/l1/lifecycle_lift_ready_guard.rs`:

- `mod.rs:273-297` `LifecycleSignal` += `Cancelled`; `ALL` `[Self; 7]` → 8
  (`:305-313`, append at end per `:302-304`).
- `mod.rs:213-221` `LIFECYCLE_EVENT_KEYS` += `"cancelled"` (consumed by
  `prepare.rs:426`, `bindings.rs:663`, `executor.rs:2406`, cli
  `sequence/mod.rs:132`).
- `mod.rs:248-263` `LifecycleStacks` += `cancelled` field; `stack()`/`stack_mut()`
  arms (`:983-1010`).
- `parse.rs:97-104` event loop += `Cancelled` (normal stack, not shell-free —
  `refuse_initialize_shells` stays initialize-only).
- `audio.rs:19-29,43-52` += cancelled; **`can_carry_error` += `Cancelled`
  (`:62-64`)** (cancelled reads `err`); leave
  `routes_action_error_to_failure` out (cleanup errors are secondary).
- **`actions.rs:250-255` `is_valid_for` — the cancelled action-set
  restriction**: permits only `Stop`; rejects `Error`, `Proxy`, `Retry`,
  `Resume`, `Defer`, `Skip` (plus `prep`/`break` when that slice lands them —
  they do not exist yet, `signatures.rs:245-250`). Typed error:
  `CompositionError::LifecycleActionPlacement` (`parse.rs:897-910`).
- `validate.rs` walkers iterate `LifecycleSignal::ALL` (`:28,256,281,317,372,381`)
  — auto-include; `err` legality follows `can_carry_error` via
  `bindings.rs:140-146`.

(b) Runtime/guard (Phase 3/6):

- `mod.rs:610-618` terminal arm += `Cancelled`; `:668-675`
  `TerminalLifecycleScope` grace window += `Cancelled`; drop-net (`:921-932`)
  must not double-fire after cancellation — the cancellation path claims the
  terminal slot explicitly.
- `runtime.rs:212-215` `needs_finalize` += `Cancelled` (finalize still runs);
  `runtime.rs:421-455` `terminal_or_finalize` += `Cancelled` in both arms —
  needs a cancelled outcome representation end-to-end (today only
  `TerminalSuccess`/`TerminalFailure` exist, `runtime.rs:84-89`; also
  `TerminalRecovery` CLI-side).
- NEW guard: `decide_lifecycle_transition` (`runtime.rs:354-419`) must refuse
  `Retry`/`Resume`/`Proxy` when the terminal slot is `Cancelled` (typed
  cancellation-recovery error) — `decide_control` is deliberately
  event-agnostic (`control.rs:93-101`).
- `executor.rs:197` `routes_to_failure` — cancelled excluded, so action errors
  do not reroute to `failure`.

(c) Call sites (Phase 5/6): document pipeline `pipeline.rs:1739` +
`route_initialize` `:1493-1605` (model for pre-start cancellation cleanup with
shells disabled — spec §4 "entered-but-not-started");
`control_dispatch.rs:50-275` (cancelled outcome/exit-130 path);
`composition/preflight.rs:201`; loop mirror `looping/engine.rs:167-190`;
sequence/group runner under `cli/src/commands/wrap/sequence/` (legs parse
lifecycle via `mod.rs:132` excludes); `prewarm/sequence.rs`; lib
`sequence/preflight/mod.rs`. Shell gate: `mod.rs:679-681` (`!start_emitted` ⇒
`DisabledShellRunner`, `executor.rs:437-439`, error `BeforePreflight`) —
cancelled inherits it, matching spec §4. Ctrl-C entry:
`claudine/lib/src/interrupt.rs` (`interrupted()` `:24`,
`TerminalLifecycleScope` `:40`, `emit_signal` short-circuit `:895-897`) and
`cli/src/commands/wrap/exec/termination/{coordinator,handle,unix,windows}.rs`;
ladder documented in `claudine/docs/topics/signal-handling.md`.

(d) Docs/schemas/bindings: `docs/topics/flow-control/lifecycle.md:34-47` event
table; `flow-control.md:750` ("every directive valid in every event except
skip" needs the cancelled exception); `flow-control-reference.md`;
`claudine/schemas/claudine.yaml:86-91` += `cancelled: stack@./lifecycle.yaml`;
`docs/schemas/partials/lifecycle.yaml:6-13` (pre-existing `finalized` typo
there — drift, code is authoritative); `bindings.rs:105-117` scope `ALL` → 12
+= `Event(Cancelled)`, `id()` `:120-134`; test corpora
`lib/tests/l1/claudine_schemas{,_matrix,_embedded}.rs`, darkmatter
`shipped_schema_corpus.rs:262-270` and
`darkmatter/tests/fixtures/schema-triggers/schemas/claudine.yaml`.

**Risks/gotchas:** size-typed const arrays + exhaustive matches (compiler
drives, but `ALL` order changes validator diagnostic order); `Cancelled` is
the second placement-restricted directive context after `Skip` — update
"is_valid_for" docs and the three "skip only in initialize" schema comments in
lockstep; drop-net can synthesize a wrong terminal if cancellation has not
claimed the slot before a panic/drop; availability tables are triplicated
(schemas/, docs/schemas/partials/, darkmatter corpus); cancelled needs an
outcome representation end-to-end; comment drift found —
`looping/engine.rs:173-176` claims initialize `retry`/`resume`/`defer` are
rejected at parse time but `pipeline.rs:1580-1589` rejects `retry` at runtime
(`LifecycleSetupPhaseRecoveryUnsupported`); code is authoritative.

### Task 1.5 — Baseline

Host: macOS. Run before any change by this feature.

- `claudine/` `just test`: **9659 tests run: 9659 passed (1 slow), 9 skipped**
  (Summary 124.861s). No failures.
- `claudine/` `just lint`: **passed** (claudine, claudine-contract,
  claudine-cli, claudine-gen all clean).
- `messenger/` `just test`: **710 tests run: 710 passed (11 slow), 2 skipped**
  (Summary 21.292s). No failures. Two tests above 15s are pre-existing
  recovery simulations (`a_generation_interrupted_before_selection_recovers_to_the_old_snapshot`
  19.0s, `an_interrupted_promotion_recovers_to_one_consistent_snapshot_and_completes_on_retry`
  15.1s).
- `messenger/` `just lint`: **passed** (1m 04s).

Pre-existing unrelated uncommitted changes in the working tree (noted, not
touched): the uncommitted final phases of `2026-10-23-shell-template-approval`
(lifecycle `shell_template.rs`/`structured_shell.rs`, sequence preflight
shape/reapproval, shell-approval tests, docs) plus darkmatter compose docs and
skill files. HEAD is `005cb5fda planning(claudine): record Phase 2 and Phase 3
progress for shell-template-approval`. Any later failure in files those
changes touch is attributable to them, not this feature.

### Task 1.6 — Rulings (plan `yolo: true` → each recorded as **assumed**)

1. **[Phase 3-5] Dependency specs** — *assumed resolved*: Task 1.1 verified all
   three are implemented in the working tree. No absorption needed.
2. **[Phase 3] `params:` spelling** — *assumed*: fixtures use `params:`;
   spec `with:` examples are read as `params:`. Container `retry.with` values
   enter runtime state under the container's `params:` binding with no
   per-key schema; "validated afresh" = existing value-shape validation only.
3. **[Phase 5] Non-cancellable leg outcome after group failure** — *assumed*:
   record its own outcome (`succeeded`/`failed`), not `cancelled`; only
   opt-in-cancelled legs and never-admitted legs are `cancelled`.
4. **[Phase 5] Skipped container as step/leg** — *assumed*: appends nothing to
   a sequence; maps to `null` in a group dictionary;
   `group.legs.<leg>.outcome` is `skipped`.
5. **[Phase 6] Windows Ctrl-C / exit 130** — *assumed*: assert `130` on Unix;
   on Windows assert the value the existing signal ladder already produces
   (see `2026-07-11-signal-assurance-and-handling` docs and the `os` skill),
   recorded in one `docs/` table; if a console interrupt cannot be delivered
   without focus, the L2 test is skipped there with an explicit reported skip.
6. **[Phase 2] Messenger fleet lock** — *assumed*: no new dependency — reuse
   the std advisory lock through Messenger's existing `PublicationLock`
   (`research/publish/fsutil.rs:116-147`), the same pattern `cleanup.rs:54-63`
   already probes with; `fs4` is the in-graph fallback if a crate is required.
7. **[Phase 2] "Recovered explicitly"** — *assumed*: a new explicit flag on
   the existing `messenger research` cleanup/resume path (no new top-level
   command) marks a lockless `active` record `interrupted`; status commands
   only report a stale run.
8. **[Phase 2] Ledger-era run records** — *assumed*: `exhausted` stays a
   readable, resumable-like-`interrupted` status that is never promotable and
   never shown successful; an unreadable `budget.json` in a run directory is
   ignored (not deleted) with a one-line note.
9. **[Phase 5] Container wall-clock timeout** — *assumed*: containers carry
   no `timeout` of their own; a leg's `timeout` applies to that leg only.

Checkpoint 1: dependency status recorded; spike results recorded; rulings
recorded; baseline recorded.

## Phase 2

Budget ledger removal and Messenger run protection (spec section 10,
requirement 8, criteria 15 and 22). Deleted source files this phase:
`claudine/cli/src/budget/{mod,model,run,store,error,tests}.rs`,
`claudine/cli/src/commands/budget.rs`, `claudine/cli/tests/l1/sequence_budget.rs`,
`claudine/docs/cli/budget.md`.

### Recovery note

An earlier Phase 2 run had already written Wave 1 uncommitted (the runner
module, the state fields, recovery, refusals, and most tests) but ended
before logging or verifying anything. This phase inherited that tree,
finished and fixed it, then executed Wave 2 in full. Fixes applied to the
inherited work: a swapped backtick/paren in an expected note string, a
JSON-splice helper that indexed pretty bytes with compact-JSON lengths
(replaced with parse-modify-write), a missing `--json` in a test's runner
invocation (refusals go to stderr as human text without it), two clippy
failures (`bool_assert_comparison`, `useless_format`), and a compile error
(`&Path`/`PathBuf` mismatch in `join_paths`).

### Task 2.1 — Runner lock

- `messenger/lib/src/research/refresh/runner.rs`: `execute(loader, run_id,
  extra)` / `execute_with(loader, run_id, command)`. Takes
  `StateArea::fleet_lock()` (`runs/fleet.lock`, std `File::try_lock` via
  `PublicationLock` — Ruling 6, no new dependency) for the whole execution;
  second runner refused with `RefreshError::RunnerLive`. Marks the record
  `launched`, spawns from the repository root, enforces the run's
  `RunLimits::max_seconds` elapsed cap (250 ms poll, kill on overrun), and
  records `RunnerOutcome::{finished,interrupted,stopped}` in `run.json`:
  exit 0 keeps the check-decided status; 130/`0xC000013A`/signal →
  `interrupted`; any other exit or spawn failure → `failed` (only if still
  active — a failed check's own reason wins).
- `state.rs`: `RunnerOutcome` string_enum; `RunRecord.launched: bool` and
  `RunRecord.runner: Option<RunnerOutcome>` (`#[serde(default)]`, so
  ledger-era records load); `fleet_lock_path()`; `fleet_lock_held()`.
- `prepare.rs`: `prepared()` prints exactly one command,
  `messenger research run <run_id>`; `resume` resets `launched`/`runner`.
- CLI: new `messenger research run RUN_ID [--json] [-- extra-sequence-args]`
  subcommand (`RunArgs`, `last = true` so `--json` still parses); the exit
  follows the observation (0 finished, 130 interrupted, 1 stopped). Kept:
  per-refresh cap on agent runs (limits now enforced by the runner's
  elapsed-time stop), run-record validation, stage checks, resume limit,
  preparation lock, promotion refusal.

### Task 2.2 — Stale-run recovery and refusals

- `cleanup::plan` and `apply` refuse while a runner is live
  (`refuse_while_live`); `promote::promote` refuses likewise.
- `cleanup::recover(state, run_id)`: a launched `active` record whose runner
  is gone is marked `interrupted` with a "recovered" stop reason — explicit
  only. Refused when a runner is live (`RunnerLive`), not active
  (`WrongStatus`), or never launched (`NotLaunched`).
- `messenger research runs` reports each active record's state: a live
  fleet lock, "prepared, not yet run", or "stale: … recover it with
  `messenger research cleanup --recover <RUN_ID>`"; `cleanup --recover
  RUN_ID` flag (conflicts with `--apply`/`--older-than`).

### Task 2.3 — Tests (criterion 22 + run-record matrix)

In `messenger/lib/tests/research_refresh.rs`:
`a_live_runner_refuses_a_second_run`,
`the_runner_records_finished_interrupted_and_stopped`,
`the_runner_stops_a_sequence_that_exceeds_the_elapsed_time_limit`,
`a_stale_active_run_is_recovered_explicitly_and_never_silently`,
`cleanup_and_promotion_are_refused_during_a_live_run`,
`a_ledger_era_exhausted_run_is_readable_resumable_and_never_promotable`
(check-run on an exhausted record reports `passed: false` and keeps the
status — never shown successful), and the input-robustness matrix
`the_run_record_matrix_is_typed_for_every_load_bearing_field` (control row
plus absent/null/wrong-type/one-element/all-elements/empty/duplicate/
trailing cells for `status`, `recovery_attempts`, `stages`, `run_id`,
`platform_id`, walked from a real fixture record through the public load).
In `messenger/cli/tests/research_lifecycle_cli.rs`:
`the_runner_command_refuses_a_live_fleet_lock_and_reports_stale_runs` and
(unix-only, fake `claudine` on a private PATH)
`the_runner_runs_the_sequence_command_and_maps_its_exit`.

**Checkpoint 2a passed:** `just test` (719 → later 718 tests, all passing)
and `just lint` green in `messenger/` with the ledger readers still in the
tree.

### Task 2.4 — Messenger ledger removal

Deleted per the Task 1.2 inventory: `LedgerView`/`Allowance`,
`StateArea::ledger()`, `apply_ledger()` and its call sites in
`check.rs`/`promote.rs`/`select.rs`/`research_lifecycle.rs`; the
`RefreshError::Ledger` variant and resume guidance; the
`stopped_by_exhaustion` exhausted-summary wording; `ledger_lock_held` in
cleanup (superseded by the fleet-lock refusal); the ledger row render in
`runs`. `RunRecord::ledger_runs` became a `#[serde(skip_serializing,
default)]` read-only field so ledger-era records (which carry it) still load
under `deny_unknown_fields` — existing records stay readable per spec
section 10; new saves never write it (guarded by a state.rs unit test).
`RunStatus::Exhausted` stays readable/resumable/never-promotable (Ruling 8).
A leftover `budget.json` is never read; `runs` reports it as a one-line note
(`legacy_ledger` row field + human fact) and nothing deletes it
(`StateArea::has_legacy_ledger`). Ledger-subject tests were deleted or
rewritten runner-driven: `recovery_is_bounded` (renamed),
`a_sequence_that_stops_before_its_checks_leaves_a_resumable_or_rejectable_run`
now drives states through `runner::execute_with` exit codes instead of
ledger files.

### Task 2.5 — Claudine CLI ledger removal

Deleted `cli/src/budget/` (6 files), `cli/src/commands/budget.rs`,
`tests/l1/sequence_budget.rs`, the `Budget` clap variant + dispatch +
telemetry arm, the `sequence --budget-ledger` option and its
`run_with_ledger` wrapper, and the runtime hooks the compiler then exposed
beyond the spec's list: `admit_launch`/`settle_launch` in `loop_control.rs`,
`enter_stage`/`stopped_by_exhaustion` in `sequence/iterate.rs`,
`attach_stop_flag` in `sequence/mod.rs`, `cap_wait` in
`control_dispatch.rs`, and five `crate::budget::record_child` spawn-site
calls (captured/inherited/retained/semantic/session). Ledger comments in
`wrap/exec/termination/windows.rs` updated. The `error_guards`
transport-allowlist entry for the deleted `budget::run_with_ledger` was
removed (its every-entry-matches-a-live-site guard failed until then).
Exit codes 76/77 no longer exist anywhere.

### Task 2.6 — Completions

`--budget-ledger` removed from the owned-value option table
(`tests/common/owned_value_options.rs`, 18 → 17 with its guard test),
`completion_ownership.rs` (candidate list 5 → 4, both sequence macro
invocations, COVERED list), `provider_tail_ownership.rs` budget help rows,
and the engine test comment. The generated scripts are dynamic (they shell
out to `claudine __complete`), so removing the clap surface cleans them;
verified by generating all five shells and grepping — zero `budget` hits,
and the engine's root menu offers none.

### Task 2.7 — Docs and skills

Deleted `claudine/docs/cli/budget.md`; removed the budget row from
`docs/cli/index.md`, the option row from `docs/cli/sequence.md`, the README
bullet's ledger sentence, the `--budget-ledger` example in
`shell-completions.md` (its unrelated candidate/wall-clock "budget" wording
stays). `claudine/docs/dependencies.md`'s "Budget Ledger Process Identity"
section became "Process-Start Identity" (the `sysinfo` edge stays —
`cli_utils::process_start` and descendant enumeration still use it), and
root `docs/dependencies.md` was reworded to match. `messenger/cli/README.md`
and `messenger/docs/user-guide.md` now describe the runner, `run`,
`cleanup --recover`, stale reporting, and the elapsed-time cap; exit-76/77
recovery rows removed. Skills: `messenger/research-contract.md` rewritten
("Refresh runner and fleet lock" replaces "Refresh budget"; recipes and
gotchas updated), `messenger/SKILL.md` index line, `claudine/cli-commands.md`
budget rows removed, dated timeline entry added (hash/last_updated
re-stamped; `md set` was used only on this 2-key frontmatter after
verifying it did not reorder the block).

### Task 2.8 — Unused dependencies

`cargo machete` over `claudine/cli`, `messenger/lib`, `messenger/cli`:
none unused. `sysinfo` retained for `cli_utils::process_start` and
descendant enumeration. No Cargo.toml changes.

### Checkpoint 2b

- Criterion-15 L1 tests added in
  `claudine/cli/tests/l1/sequence_errors_cli.rs`:
  `the_budget_subcommand_is_an_unknown_subcommand` (clap exit 2,
  "unrecognized subcommand … budget") and
  `the_budget_ledger_option_is_an_unknown_option` (the pre-clap partition
  refuses the now-unowned switch before the file with exit 1, usage
  guidance, nothing runs, no file created).
- The criterion-15 rg search returns exactly three hits, all inside the
  criterion-15 test itself, which must spell the removed option to assert
  its refusal. **Deviation from a literal "no hit" reading, recorded here
  for the Phase 9 audit / author:** the test's own literals are the
  assertion, not residue; if a strict zero is required, build the two
  strings dynamically in the test.
- `just test` + `just lint` green in both areas (claudine 9638 passed;
  messenger 718 passed); `messenger research prepare` prints only
  `messenger research run <RUN_ID>`.
- Cross-OS: `just cross-check messenger --os linux` green (618 passed).
  Windows/WSL2 left to CI (pull requests prove Linux/macOS); the runner's
  Windows exit mapping (`0xC000013A`) and the cfg-branched test commands
  follow existing portable patterns.

### Requirement-to-test mapping (this phase)

- Criterion 22 (all five clauses): the six lib tests + two CLI tests above.
- Criterion 15 (phase-2 scope: usage errors, no exit 76/77, no budget.md,
  clean completions, green gates, prepare emission): the two
  sequence_errors_cli tests; exit-code and doc removals verified by search;
  completion scripts verified by generation + grep; prepare emission
  asserted in `a_run_goes_from_preparation_to_promotion_through_the_binary`
  (commands == `messenger research run <id>`, resume prints the same).
- Input-robustness matrix for `run.json`: one table-driven test from a real
  fixture record with a control row (`the_run_record_matrix_…`), plus the
  state.rs ledger-era load test; no `#[serde(default)]` on load-bearing
  `status`/`stages`/`recovery_attempts`, no `unwrap_or_default()`/`.ok()`
  on their parses (grepped).

### Completion audit — invocation cap regression

A repeat Phase 2 invocation found the existing implementation and checked
Phase 2 tasks already in the working tree. Auditing the retained protections
found `max_invocations` was stored but never read to decide admission after
the ledger removal. Task 2.1 is reopened until that regression is covered.
Before implementation, the behavior-to-test mapping is:

- Agent admission up to the exact cap, refusal beyond it, persisted count,
  and preservation across resume: new library regression
  `agent_admission_preserves_the_cap_across_resumptions`.
- Normal CLI admission and refusal with downstream record unchanged:
  `agent_admission_cli_refuses_when_the_cap_is_spent`.
- Every generated prompt has an admission hook: the existing prepared-input
  corpus assertion, extended to all four prompts.
- The generated hook actually prevents provider launch: a Claudine CLI L1
  regression executing a generated prompt against a refusing Messenger stub.
- The new persisted count's absent legacy value, null, wrong type, negative,
  empty, duplicate and trailing input: extend the real-record JSON matrix.

### Completion audit — implementation and targeted verification

The invocation cap now belongs entirely to Messenger. `runner::admit_agent`
reserves and atomically saves one admission under the preparation lock before
each generated prompt starts. The hidden `research admit-agent RUN_ID`
command returns exit 1 at the exact cap or for a non-active run; refusals
leave the run record unchanged. A count remains spent when launch later
fails, so interruptions cannot create extra allowance. Resume preserves the
count. Each of the four prompts has a `start` shell hook, so continuing past
a denied task cannot bypass the cap at the next task. Before execution, the
runner upgrades ledger-era prepared prompts; a repeated execution adds no
second frontmatter block. The old record's absent count explicitly migrates
to zero; present null, string (including empty), boolean, fractional,
negative, collection, and duplicate values fail through the public loader.
The existing JSON matrix also rejects trailing content and restores its
control record.

Targeted requirement mapping and evidence:

- `agent_admission_preserves_the_cap_across_resumptions`: first admission,
  exact-cap refusal, identical persisted state after refusal, wrong-status
  refusal, resume/save/read retaining the count, and refusal after resume.
- `agent_admission_cli_refuses_when_the_cap_is_spent`: executes the actual
  hook extracted from a generated prompt using the real Messenger binary,
  then repeats it to prove exit 1 and unchanged downstream `run.json`.
- `discovery_inputs_hold_no_previous_research_or_curated_sources`: passive
  generated corpus check now verifies all four prompt admission hooks.
- `the_runner_upgrades_legacy_prompt_hooks_only_once`: runs a prepared
  ledger-era prompt twice and checks the resulting document is identical.
- `the_run_record_matrix_is_typed_for_every_load_bearing_field`: expanded
  for the admission counter; final targeted run passed (0.510 s).
- `a_refused_messenger_admission_prevents_every_provider_launch`: new
  declared Claudine CLI L1 test runs the hook shape with a refusing Messenger
  stub and a recording provider, including a following prompt task.
  Its first run exposed a fixture error (unnamed steps); names were added.
  This is distinct from the real-artifact Messenger CLI test above.
- Criterion 15's removed-option test now uses `concat!("--budget",
  "-ledger")` and is named `the_removed_ledger_option_is_an_unknown_option`.
  The exact obsolete argument remains covered, while the strict repository
  migration search no longer needs the earlier test-literal exception.

The first invocation-cap test build failed on the missing admission API and
counter, confirming the missing implementation before it was added. The
first full Messenger run after the final production change passed 721/721
(2 existing skips). New/expanded admission tests took 2.3–2.7 s in that run;
none exceeded 5 s. Task 2.1 is checked again after these checks. The only
additional changes after that full run expand malformed counter test rows.

### Completion audit — final gates and known limits

- macOS `just test` in `claudine/`: **9,639 passed, 9 existing skips**;
  the admission/provider-launch regression passed in 1.444 s and the renamed
  removed-option regression passed in 0.042 s.
- macOS `just test` in `messenger/`: **721 passed, 2 existing skips**.
  The last matrix expansion was then verified by the targeted recipe
  `just test the_run_record_matrix` (1 passed; other tests filtered out).
- `just lint` passed in both areas after the source changes. These recipes
  run Clippy and the area's guards; no `cargo fmt` was run.
- `just check-tier-coverage messenger`: zero stranded tests. The new tests
  are compiled by existing auto-discovered Messenger targets and the
  existing declared Claudine `l1` target/module, and have L1-selected names.
- The strict criterion-15 repository search has **zero hits**, and
  `claudine/docs/cli/budget.md` is absent. All Phase 2 plan tasks are checked.
- No CI cell, dependency, or Git staging/commit operation
  was added. Existing staged/unrelated work was preserved.
- No extra OS runs in this audit: the new production logic uses the existing
  portable file-lock and atomic-save path. The previously logged Linux
  cross-check predates this cap fix. Windows/WSL2 remain untested locally.
- Existing Messenger ignored tests: `integration::smoke_test_discord_send`
  and `integration::smoke_test_slack_send` (live-provider smoke tests).
  The recording-provider Claudine regression is Unix-only because its stub
  is a shell script; the real Messenger CLI admission test is portable.
- No unresolved test failures. The new fixture failure noted above is fixed.
  The macOS linker emitted its existing large-unwind-table warning.

Existing tests exceeded the 5 s L1 budget during the final runs (none of the
new or expanded admission regressions did). These are pre-existing speed
violations outside this phase's changes, not waived or renamed here:

- `messenger tests::desktop_helpers::notify_send_stub::notice_only_timeout_maps_to_timeout_error`: 5.014 s.
- `messenger tests::desktop_helpers::snoretoast_stub::notice_only_timeout_maps_to_timeout_error`: 5.019 s.
- `messenger tests::desktop_helpers::terminal_notifier_stub::notice_only_timeout_maps_to_timeout_error`: 5.023 s.
- `messenger tests::desktop_helpers::burnttoast_stub::notice_only_timeout_maps_to_timeout_error`: 10.053 s.
- `messenger::research_publication a_generation_interrupted_after_selection_recovers_to_the_new_snapshot`: 7.611 s.
- `messenger::research_refresh a_reviewed_change_publishes_with_its_review_record_and_changelog_entry`: 5.378 s.
- `messenger::research_refresh an_impossible_evidence_date_cannot_qualify_an_unchanged_renewal`: 5.648 s.
- `messenger::research_refresh a_verified_unchanged_renewal_is_accepted_without_a_changelog_entry`: 6.250 s.
- `messenger::research_refresh cleanup_previews_exact_removals_and_protects_open_runs`: 5.148 s.
- `messenger::research_refresh decisions_name_a_trimmed_maintainer_and_invalid_persisted_names_do_not_load`: 5.182 s.
- `messenger::research_refresh an_initial_publication_promotes_every_platform_together`: 7.867 s.
- `messenger::research_refresh an_unreadable_run_record_blocks_its_platform_and_names_its_path`: 7.443 s.
- `messenger::research_refresh automatic_renewal_is_refused_unless_everything_substantive_is_unchanged_and_rechecked`: 7.492 s.
- `messenger::research_refresh selection_names_every_due_reason_and_skips_current_or_open_platforms`: 5.413 s.
- `messenger-cli::research_lifecycle_cli a_run_goes_from_preparation_to_promotion_through_the_binary`: 5.252 s.
- `messenger-cli::research_lifecycle_cli prepared_runs_name_no_host_path_and_their_checks_resolve_any_root`: 5.051 s.
- `messenger::research_publication a_generation_interrupted_before_selection_recovers_to_the_old_snapshot`: 27.226 s.
- `messenger::research_refresh an_interrupted_promotion_recovers_to_one_consistent_snapshot_and_completes_on_retry`: 22.284 s.
- `claudine-cli::l1 agent_text_is_data::malformed_structured_agent_values_are_refused_not_saved_as_text`: 5.701 s.
- `claudine-cli::l1 compose_initialize_staged_boot::early_catch_shells_cannot_run_or_enable_another_catch_shell`: 5.686 s.
- `claudine-cli::l1 compose_schema_cli::compose_unresolved_caller_file_fails_before_initialize_when_non_interactive`: 6.241 s.
- `claudine-cli::l1 ctx_launch_anchor::loop_route_keeps_launch_area_for_root_and_package_documents`: 5.079 s.
- `claudine-cli::l1 provider_tail_launch::other_failures_are_not_attributed_to_the_tail`: 6.180 s.
- `claudine-cli::l1 provider_tail_launch::embedded_credentials_reach_the_child_and_no_display_surface`: 6.699 s.
- `claudine-cli::l1 provider_tail_notice::a_switch_whose_record_is_typed_unknown_is_not_called_known`: 7.414 s.
- `claudine-cli::l1 shell_missing_values::a_missing_value_fails_pre_flight_on_every_shell_surface`: 5.021 s.
- `claudine-cli::l1 tts_prewarm::every_accepted_playa_dry_run_spelling_prepares_nothing`: 6.659 s.
- `claudine-gen::l1 generate_ux::roster_alias_rename_is_regenerated_until_check_converges`: 5.088 s.

The final `just check-tier-coverage claudine` also passed with zero stranded
tests. Claudine's nine existing L1 skips are its opt-in diagnostics:

- `compose_ttff_perf::compose_emits_first_stderr_byte_within_budget`
- `completion_perf::perf_compose_empty_partial_meets_target`
- `completion_perf::perf_compose_long_prefix_meets_target`
- `completion_perf::perf_inline_compose_empty_partial_meets_target`
- `completion_perf::perf_enter_compose_partial_meets_target`
- `system_prompt_perf_bench::bench_system_prompt_resolution_cold_and_warm`
- `system_prompt_perf_bench::bench_resolve_and_prepare_step_by_step`
- `system_prompt_perf_bench::bench_request_topology_probe_and_reuse`
- `system_prompt_perf_bench::bench_raw_darkmatter_compose_passes`

No L2/L3 or live-provider test was added or needed for this phase's hermetic
CLI/filesystem/persistence behavior. No window was opened or focused.

Source files changed by this completion audit (the frontmatter preserves the
full cumulative Phase 2 inventory, including inherited implementation):

- `messenger/lib/src/research/refresh/runner.rs`
- `messenger/lib/src/research/refresh/state.rs`
- `messenger/lib/src/research/refresh/prepare.rs`
- `messenger/lib/src/research/refresh/mod.rs`
- `messenger/cli/src/research.rs`
- `messenger/cli/src/research_lifecycle.rs`
- `messenger/lib/tests/research_refresh.rs`
- `messenger/cli/tests/research_lifecycle_cli.rs`
- `claudine/cli/tests/l1/sequence_errors_cli.rs`

Documentation updates in this audit: Messenger's user guide and research
contract skill, plus this append-only log and the plan's input matrix and
checklist. Cumulative Phase 2 file lists and the four-package list remain in
both frontmatter blocks. `human_review` is false; the replacement handoff
message calls out admission hooks for the next phase. Frontmatter values were
read back with `md get`, and `git diff --check` passed.

**Implementation complete, ready for review.** Later phases remain unchecked;
the feature stays in its active directory. Nothing was formatted, staged, or
committed by this session.

## Phase 3

### Behavior-to-test mapping before implementation

- `cancelled` event parsing, allowed verbs and forbidden control verbs: lifecycle grammar tests; existing lift-ready guard extended and deliberately challenged with a temporary forbidden import.
- Inline nested sequence exclusivity, removed execution/setup/teardown/document fail_fast, named legs and reserved keys: public sequence-plan/preflight regression tests plus authored YAML robustness matrices (Markdown, formal sequence, group/catalog). Container execution remains deferred to Phase 5.
- Removed CLI `--fail-fast`: CLI refusal tests for boolean and malformed values, before source load or any shell/provider; hidden migration receiver gives the typed replacement diagnostic. CLI help/completions must not advertise it.
- Sequence no longer injects `CLAUDINE_FAIL_FAST`: actual provider invocation checks both child environment and composed prompt, with no ambient value. Loop-local fail_fast belongs to its separate dependency and stays unchanged.
- Schema and completion nested containers/cancelled: passive shipped schema corpus and normal shipped schema discovery/completion tests. Canonical schema is `claudine/schemas/claudine.yaml`; the planned Darkmatter path does not exist.

The plan explicitly requests concurrent Wave 1 work. Bounded agents own lifecycle, sequence grammar, and schemas; the primary agent owns CLI and integration gates. Existing staged changes from earlier work are preserved. No formatting, staging, or commits are authorized.

### Lifecycle implementation details

### Tasks 3.1 and 3.2 — cancellation grammar and lift-ready boundary

Read Claudine, rust-testing, and OS skills. Before implementation, added three regressions that fail because `cancelled` was silently ignored: retained cleanup action paths, null refusal, forbidden recovery controls. `just test-library cancelled` proved all three red against the baseline (other four existing cancellation tests passed).

Added `LifecycleSignal::Cancelled`, the event key/storage/accessors, canonical property/status/error-binding behavior, parsing and placement restrictions. Cleanup accepts communications, `set`, side effects, expression functions, shells, conditionals and `stop`; recovery controls fail placement, and unsupported `prep`/`break` fail the existing typed unknown-verb path. Null is a typed stack shape error; empty list/dictionary remain valid. Walkers automatically include the event through `ALL`, including conditional shell discovery and static err validation. Small exhaustive-match/guard/finalize plumbing accompanies the enum so it compiles; cancellation admission, descendant cleanup, and interrupt execution remain for later runtime phases.

Existing criterion 14 guard already covered the five named modules plus source_map. Strengthened forbidden runtime-type names and added `criterion_14_deliberate_runtime_import_fails_the_guard`, which injects `use crate::config::GlobalSettings;` into the guard assertion and checks that it really panics. No temporary product import remains.

Targeted new tests in lifecycle/tests/grammar.rs:
- cancelled_parses_cleanup_actions_and_preserves_authored_paths
- cancelled_rejects_null_instead_of_silently_opting_in
- cancelled_rejects_recovery_and_other_control_actions
- cancelled_accepts_expression_families_and_validates_err_reads
- cancelled_empty_stacks_and_invalid_shapes_are_distinct
- cancelled_terminal_reserves_finalize_and_carries_the_cause

First five passed in `just test-library cancelled` (9/9 overall). Last test added afterward; combined parent gate must verify it and guard test. Broader `just test-library lifecycle` initially blocked on another agent's in-progress Sequence enum consumer; latest compile caught a missing test-only LifecycleErrorInfo import, corrected to its qualified public name. No subsequent build per coordination request. The macOS linker warns that __eh_frame exceeds 16 MB; no functional failure from it. Plan checkboxes 3.1/3.2 remain for parent to mark after combined verification.

### CLI fixture migrations

Migrated sequence_shell_templates.rs whole-value set/template/byte-grammar coverage to actual referenced prompt `start`/`success` lifecycles. Removed retired task setup scaffolding from unrelated late-binding/barrier cases, preserving downstream argv assertions. Serial-execution cases now verify typed removal before earlier steps run; task-stack group matrices similarly verify removal and no shell execution. Group snapshots use concurrent groups with a prompt writer and prove sibling mutations stay private (probe reads `before`). No test is ignored or removed.

Migrated tts_prewarm.rs sequence's early phrase to the sequence document's initialize stack, mixed playback-config coverage to prompt lifecycles, and obsolete task setup/teardown cases to refusal before speech preparation/provider launch. Removed execution keys from unrelated one-member fixtures. Removed explicit execution: parallel in agent_text_is_data.rs. task_run/tests.rs and jit/tests.rs already construct internal models directly, so no net edit required.

All added tests are in already-declared L1 modules. `git diff --check` passed for this slice. Full CLI fixture verification remains with parent's coordinated test run.

Exact changed source/test files:
- claudine/lib/src/composition/lifecycle/mod.rs
- claudine/lib/src/composition/lifecycle/actions.rs
- claudine/lib/src/composition/lifecycle/audio.rs
- claudine/lib/src/composition/lifecycle/bindings.rs
- claudine/lib/src/composition/lifecycle/parse.rs
- claudine/lib/src/composition/lifecycle/runtime.rs
- claudine/lib/src/composition/lifecycle/executor/tests/event_time_interpolation.rs
- claudine/lib/src/composition/lifecycle/tests/grammar.rs
- claudine/lib/src/composition/lifecycle/tests/parse_config.rs
- claudine/lib/tests/l1/lifecycle_lift_ready_guard.rs
- claudine/cli/tests/l1/tts_prewarm.rs
- claudine/cli/tests/l1/sequence_shell_templates.rs
- claudine/cli/tests/l1/agent_text_is_data.rs

No doc/skill/frontmatter edits by this subtask. No formatting, staging, commits, terminal focus, or remote OS runs.


### Schemas implementation details

#### Phase 3 schema/completion work

Behavior-to-test mapping before implementation:
- `cancelled` selects the shipped Claudine trigger, accepts communication/expression/shell/stop, and rejects disallowed flow: new `sequence_container_schema_contract` test through shipped bundle discovery, plus existing full shipped schema corpus.
- Public sequence task/group schema exposes nested `sequence`, container lifecycle fields and group-only fail_fast: same discovery test inspects compiled public schema and validates nested fixtures.
- Removed generated overlays and document fail_fast disappear from catalog: same test checks effective property names.
- Completion reads shipped artifact via normal schema resolver and offers cancelled/sequence, omitting retired overlays: new schema_completion test using copied shipped bundle library.
- Existing `md schema validate` bundle CLI tests supply end-to-end normal invocation coverage; add a cancelled-only fixture to that invocation.

Location departure: plan references absent darkmatter/docs/schemas/claudine.yaml; canonical live payload is claudine/schemas/claudine.yaml. Existing bundle has no group/task files, so named container types will be added to lifecycle.yaml, without sequence.yaml.

Implemented canonical schema/catalog changes, dedicated cancelled action/conditional types, exported task/group types, and targeted schema/completion/`md` CLI tests. `sequence_container_schema_contract` passed in 1.452 s. Broader schema suite build was interrupted by concurrent lifecycle test missing import (reported to parent). Per parent coordination, stopped further builds; pending final coordinated verification: `claudine_schemas::`, `shipped_container_schema_offers_nested_container_keys`, `claudine_schema_bundle`.

Additional parent assignment: legacy internal task runtime fixture migration. Added `preflight/runtime_fixture.rs` as a test-only inert-model constructor. Parses sanitized current grammar and explicitly restores old runtime stacks/scheduler on resulting nodes, including shell authorization collection; production parser remains unchanged. Adjusted `task/tests.rs` Fixture builder to use it. Parent must run these tests after coordinated edits finish.


### Grammar implementation details

Phase 3 library sequence/group grammar

Implemented:
- Added sixth executable `sequence` (list/file reference; offset/operator suffixes use existing grammar); excludes other executable keys.
- Empty authored sequence lists now normalize to empty plans.
- Removed task setup/teardown/execution/fail_fast with CompositionError::RemovedGrammar and migration guidance.
- Removed sequence-document fail_fast including externally referenced formal documents (actual author path retained).
- Seven lifecycle event keys allowed only beside inline task sequence lists, with lifecycle parser validation; referenced sequences/other tasks reject event placement.
- Group objects reject execution of every type/value, default to Parallel, validate fail_fast boolean (missing defaults runtime policy later), variables mapping/null, and reserved outcome variable names.
- Group legs require unique nonempty string names, including referencing task legs; referencing-site name retained in the graph.
- Reserved run_id added alongside existing reserved generated step keys.
- Removed old nested-group rejection.
- Nested sequence grammar intentionally yields explicit SequenceUnsupportedConstruct at execution preflight until Phase 5; no placeholder execution.

New targeted tests (compiled library unit target, ordinary L1 paths):
composition::sequence::tests::container_grammar::nested_sequence_and_removed_task_keys
composition::sequence::tests::container_grammar::sequence_task_shape_matrix_and_exclusivity
composition::sequence::tests::container_yaml_matrix::shipped_sequence_corpus_uses_normal_resolution_path
composition::sequence::tests::container_yaml_matrix::markdown_and_sequence_yaml_fields_matrix
composition::sequence::preflight::tests::container_grammar::group_fields_matrix
composition::sequence::preflight::tests::container_grammar::group_legs_require_unique_nonempty_names_and_execution_is_removed
composition::sequence::preflight::tests::container_yaml_matrix::group_and_catalog_file_fields_matrix

Requirement mapping:
- nested executable, exclusivity, inline [] preservation, removed task keys, misplaced events -> first two tests.
- reserved step keys -> sequence_task_shape_matrix_and_exclusivity (all 9 names specified by section 7).
- sequence root removal, YAML representation robustness, duplicate/trailing malformed rejection -> markdown_and_sequence_yaml_fields_matrix.
- group type/absence/null/empty/bad elements/unique names/concurrent default/reserved variables -> group_fields_matrix and group_legs... plus raw group_and_catalog_file_fields_matrix.
- shipped artifacts -> corpus reads actual docs/local-runners.yaml and docs/providers.yaml through include_str and resolves copied artifacts through ordinary sequence reference loader, including real template normalization.
- native vs quoted fail_fast, absent/null distinct, duplicate keys -> both raw YAML matrix tests.
- variables values remain arbitrary JSON by existing contract, so wrong-type element cells are inapplicable; wrong whole mapping and reserved key names are covered.
- no persistence writer changed, so repeated persisted read/write/read has no applicable new writer behavior.

Regression evidence:
Before production edits, both initial targeted tests FAILED: setup:null was accepted as task stack option; sequence:null silently became authored state. /tmp/phase3-grammar-before.log.
After edits, 7/7 new tests PASSED, 0.217 seconds combined, slowest 0.200 seconds: /tmp/phase3-new-matrices2.log.
Initial new YAML tests incorrectly used Markdown-only fixture source resolver and failed NotMarkdown; fixed to actual public YAML document loader / external artifact reference path, then passed.

Legacy runtime/approval coverage:
Old setup/teardown and serial scheduler tests use explicit internal RuntimeFixture construction (schema agent authored helper) rather than production cfg(test) acceptance. New parser matrices exclusively use public normal parsing/preflight. Old empty-sequence/fail_fast tests updated to current grammar. Legacy preflight file fixtures supply leg names centrally to retain focus on reference/approval contracts. Parser-specific execution-removal/nested-group tests use public preflight.

Files owned/changed by grammar agent:
claudine/lib/src/composition/sequence/grammar.rs
claudine/lib/src/composition/sequence/model.rs
claudine/lib/src/composition/sequence/normalize.rs
claudine/lib/src/composition/sequence/reserved.rs
claudine/lib/src/composition/sequence/mod.rs
claudine/lib/src/composition/sequence/formal.rs
claudine/lib/src/composition/sequence/source.rs
claudine/lib/src/composition/sequence/preflight/mod.rs
claudine/lib/src/composition/sequence/preflight/tests.rs
claudine/lib/src/composition/sequence/tests.rs
claudine/lib/src/composition/error/mod.rs (RemovedGrammar docs expanded)

Full library verification is currently running under parent /tmp/phase3-library-full.log; grammar agent will resolve its sequence/preflight failures before final return. No cargo fmt, staging, or commits.

Full-library migration audit:
The parent's full library run reached all sequence tests. Seven new grammar tests passed in that run. Three preflight fixture failures were corrected afterward:
- zero_max_parallel_is_rejected: removed retired execution spelling so the test reaches zero-cap validation.
- default_concurrent_group_rejects_reused_inline_target (renamed obsolete serial-sharing test): public parser now asserts SequenceWriteBackCollision for default concurrent group.
- stack_set_shell_values_are_collected_and_approved: compares approval membership; direct legacy model constructor restores stack sites after the primary graph so constructor-only order differs.
Two task runtime fixture failures were assigned back to schema agent: immutable reference options test should patch an option instead of the now-required leg name; external task leg fixture should author its expected leg name.
Expanded targeted exclusivity test additionally checks all 9 reserved step keys and initialize event placement. Parent owns final combined rerun.


### Group Cli implementation details

Migrated only claudine/cli/tests/l1/sequence_groups.rs and sequence_errors_cli.rs.

- Ordering and output-chain regressions now use root sequence tasks.
- Concurrent group/cap/output/attribution/provider-session regressions retain runtime coverage with implicit concurrency and named legs; file/catalog bundle side-effect comparisons ignore concurrent arrival order.
- Former document fail_fast true/false runtime tests retain authored flag inputs and assert explicit refusal before side effects.
- Former contested-state task setup runtime fixture retains setup and asserts removed grammar plus replacement advice and zero provider launches. State merge requires later container lifecycle runtime work rather than illegal old syntax.
- Failed concurrent member test retains sibling settlement and verifies default sequence stop, removing retired document fail_fast false.
- Former serial-vs-parallel frame comparison now checks concurrency caps 1 and 2 preserve geometry.
- Error fixtures include required leg names; nested group fixture indentation repaired.
- Serial execution rejection now asserts removed grammar and sequence advice; added explicit parallel execution rejection with omit-key advice. Other error fixtures omit old execution field so their intended errors remain reachable.
- Existing removed_fail_fast tests at end preserved.

No builds run here as requested; parent owns full CLI gate. No plan edits, staging, commits, or cargo fmt.


### Docs implementation details

Updated claudine/docs/topics/flow-control/sequences.md only.

- Removed the historical disclaimer and all advertised task setup/teardown and execution mode authoring examples.
- Added nested sequence task entry to executable table and reserved task-key list.
- Corrected group examples to omit execution and name each leg uniquely; documented concurrent defaults, max_parallel, and boolean fail_fast.
- Replaced task-stage reference with prompt-document lifecycle migration example and immutable task-reference semantics.
- Replaced serial group runtime section with ordered-work replacement guidance.
- Removed document fail_fast continuation semantics and CLAUDINE_FAIL_FAST injection claims.
- Marked nested sequence execution, container lifecycle recovery, sibling cancellation, and cancelled dispatch as planned; described current runner behavior without implying those behaviors shipped.
- Preserved state/previous/next/outputs namespace and existing detailed preflight, output, environment, source, and shell reference content.
- Corrected unrelated inaccurate count of reserved roots while touching its context.

No tests run: documentation-only edit. No cargo fmt, staging, or commit.


### Prewarm implementation details

#### Prewarm fixture repair

Changed only `claudine/lib/src/composition/lifecycle/prewarm/sequence/tests.rs`.

The three red retained-runtime tests now build through the existing test-only `RuntimeFixture`: it removes retired authoring fields before the public grammar resolves, then installs legacy task stacks and serial/parallel execution choices directly on internal preflight models. Their original authored inputs, phrase counts, deduplication assertions, captured input checks, and playback configuration assertions remain intact. The other two tests continue using the public resolver/preflight route unchanged.

Tests deliberately not run because the parent owns the active full library/CLI runs. Targeted names:
- `every_early_phrase_of_three_prompts_is_queued_before_any_step_runs`
- `task_stacks_use_task_playback_and_prompt_lifecycles_use_the_user_configuration`
- `task_stacks_resolve_with_the_tasks_captured_inputs`

Observed an additional full-run failure `composition::lifecycle::tests::nested_span::single_pass_inventory_matches_the_authored_claudine_schema`; this is outside this narrow task and was not modified.


### Shell Cli implementation details

#### Shell CLI fixture regression fixes

Changed only `claudine/cli/tests/l1/sequence_shell_templates.rs`.

Requirement-to-test mapping:

- Groups execute concurrently with no leg completion ordering guarantee: `a_first_group_offers_resolved_bytes_for_every_member` and `a_later_group_keeps_template_mode_for_every_member` still assert every exact recorded argument vector; compare the vectors without ordering among concurrent legs. The later-group case separately asserts the predecessor remains first. `exact_rules_cannot_authorize_a_later_groups_members` continues asserting refusal and no effects.
- Prompt document lifecycle values resolve against the prompt document: `byte_grammar_{first,group}_prompt_{start,success}` now author `tool: argv-recorder` in child.md, keeping the exact `{{ tool }} probe` command and the command sequence assertions. The associated `_set` rejection cases use the same now-valid child field and continue requiring executable-grammar rejection before effects.
- `body_steps_fill_their_current_value_without_preflight_spawns` remains unchanged: failure is schema rejection of named value-only body steps, which parent reported fixing in the schema independently. Its current recorded argument assertions preserve original body-step semantics.

No new tests, tier markers, ignored cases, or production changes. The existing test file is declared in `tests/l1/main.rs` line 189, so all affected names remain compiled L1 tests. No build or test was launched while the parent full CLI run owns its build artifacts. Parent must rerun these targets after rebuilding.


### Integration decisions and regressions

The final CLI design removes the clap flag entirely. A typed `PartitionError::RemovedSequenceFailFast` rejects the exact flag, attached values, and missing values during ownership scanning, before a document read. Owned option values and an explicit provider `--` suffix retain their original ownership. Three CLI regressions first failed against baseline, then passed (true, false, malformed); help omits the flag. `step_owned_env` no longer adds the sequence policy to a child. Internal scheduler fields remain until runtime migration.

The first broad runs found stale fixtures plus a new schema defect: named task objects omitted existing operation/flow/timeout and application state. The schema now declares those task options and permits arbitrary step state via the existing catch-all type grammar; reserved/exclusive authoring remains the public task parser's responsibility. A regression row covers task options plus arbitrary state. Fully expanding the task lifecycle schemas in every root sequence also imposed unnecessary repeated schema work and changed which unrelated controls were judged in formerly plain sequence documents. The trigger therefore retains its existing opaque sequence-array payload and trigger-selection rules; exported named task/group schema types provide explicit container schemas/completions, while public sequence parsing validates every nested item. No new schema format or sequence.yaml was added.

Task `setup`/`teardown` and serial group public CLI scenarios were migrated to prompt lifecycles, root sequence steps, bounded concurrent groups, or explicit refusal regressions, according to their observable purpose. Existing internal runtime tests construct inert preflight models through a test-only RuntimeFixture; public grammar tests never use that constructor. This preserves runtime contracts until the later runner refactor without reopening removed syntax.

The input matrix's variables-element cells are not type errors: variable values are arbitrary JSON values by the existing contract. Whole-field null/wrong types, duplicate keys, reserved names, and invalid documents are rejected. No new persistence writer was introduced, so read/write/read persistence round trips do not apply to this phase.

Targeted new grammar/corpus/YAML matrices passed 7/7 (0.217 s total); lifecycle cancelled regressions passed, including retained cause/finalize behavior in the broader library run. Plan tasks 3.1 and 3.3 checked as their functionality passes targeted tests; broader integration checkpoint remains pending.

### Final integration audit

The complete area run reached 9,657 tests: all new grammar matrices, cancelled
cases, schema contract (0.832 s), lift-ready guard, and prior runtime model
coverage passed. Eight fixture/assertion failures remained and were repaired:
native environment shell lists no longer interpolate captured values into
commands approved against the launch map (separate side-effect tests still
cover expression reads); nested groups are accepted rather than rejected;
collision/valid-graph fixtures have correct YAML indentation and leg names;
removed-execution tests witness an untouched output file rather than confusing
the initial sequence status line with a task launch; completion checks public
validation results instead of a compiled JSON Schema layout.

The schema completion test now imports `task` as an authored parameter type:
`$schema: { nested: task@./lifecycle.yaml }`. A root `$schema` string is a file
reference, so using a named-type spelling there was a test/documentation error,
not a product defect. The README example is corrected.

The frozen review-loop fixture drops its retired document `fail_fast` key;
the live internal prompt is left for the later migration phase. Prewarm CLI
fixtures put never-firing phrases on prompt documents, since root container
initialization execution is still planned. The mixed prewarm configuration
fixture caps group concurrency at one because its recording provider assigns
launch-file numbers serially; its subject is configuration propagation, not
parallel launch behavior (covered separately).

The final matrix audit uses control documents and one field edit per row,
adds field-specific malformed YAML, absent task sequence/cancelled/variables,
arbitrary JSON variable values, and downstream graph assertions. No test was
ignored, tier-renamed to evade a gate, or given a longer timeout.

### Container schema completion refinement

To keep nested editor completion available without duplicating the full
action catalog for every container, the trigger's array payload uses the
exported `step` type. Task/group event properties use the existing shallow
list/dictionary body shape, with the lifecycle parser retaining semantic
authority; cancellation retains its restricted schema type. Root document
events keep their existing deep action schemas. Trigger selection is unchanged
(sequence alone does not newly select the Claudine control catalog). This
refines the earlier integration decision: exported container types now also
provide nested property hints through the trigger payload, with bounded
validation work.

The schema refinement exposed a measurable cost in the existing embedded JSON
trigger-combinator test: 3.145 s with opaque tasks versus 6.327 s with fully
expanded cancellation schemas on every task/group. Container `cancelled` now
uses the same shallow event-body schema as the other container events; the
root document event retains the restricted cancellation schema. Public
lifecycle parsing enforces cancellation action restrictions at every primitive.
The completion regression rejects a malformed container event body through
the public validator, while root schema and parser tests cover forbidden
control actions. This keeps nested property hints without multiplying the
full action catalog.

### Grammar final audit

Phase 3 library sequence/group grammar

Implemented:
- Added sixth executable `sequence` (list/file reference; offset/operator suffixes use existing grammar); excludes other executable keys.
- Empty authored sequence lists now normalize to empty plans.
- Removed task setup/teardown/execution/fail_fast with CompositionError::RemovedGrammar and migration guidance.
- Removed sequence-document fail_fast including externally referenced formal documents (actual author path retained).
- Seven lifecycle event keys allowed only beside inline task sequence lists, with lifecycle parser validation; referenced sequences/other tasks reject event placement.
- Group objects reject execution of every type/value, default to Parallel, validate fail_fast boolean (missing defaults runtime policy later), variables mapping/null, and reserved outcome variable names.
- Group legs require unique nonempty string names, including referencing task legs; referencing-site name retained in the graph.
- Reserved run_id added alongside existing reserved generated step keys.
- Removed old nested-group rejection.
- Nested sequence grammar intentionally yields explicit SequenceUnsupportedConstruct at execution preflight until Phase 5; no placeholder execution.

New targeted tests (compiled library unit target, ordinary L1 paths):
composition::sequence::tests::container_grammar::nested_sequence_and_removed_task_keys
composition::sequence::tests::container_grammar::sequence_task_shape_matrix_and_exclusivity
composition::sequence::tests::container_yaml_matrix::shipped_sequence_corpus_uses_normal_resolution_path
composition::sequence::tests::container_yaml_matrix::markdown_and_sequence_yaml_fields_matrix
composition::sequence::preflight::tests::container_grammar::group_fields_matrix
composition::sequence::preflight::tests::container_grammar::group_legs_require_unique_nonempty_names_and_execution_is_removed
composition::sequence::preflight::tests::container_yaml_matrix::group_and_catalog_file_fields_matrix

Requirement mapping:
- nested executable, exclusivity, inline [] preservation, removed task keys, misplaced events -> first two tests.
- reserved step keys -> sequence_task_shape_matrix_and_exclusivity (all 9 names specified by section 7).
- sequence root removal, YAML representation robustness, duplicate/trailing malformed rejection -> markdown_and_sequence_yaml_fields_matrix.
- group type/absence/null/empty/bad elements/unique names/concurrent default/reserved variables -> group_fields_matrix and group_legs... plus raw group_and_catalog_file_fields_matrix.
- shipped artifacts -> corpus reads actual docs/local-runners.yaml and docs/providers.yaml through include_str and resolves copied artifacts through ordinary sequence reference loader, including real template normalization.
- native vs quoted fail_fast, absent/null distinct, duplicate keys -> both raw YAML matrix tests.
- variables values remain arbitrary JSON by existing contract, so wrong-type element cells are inapplicable; wrong whole mapping and reserved key names are covered.
- no persistence writer changed, so repeated persisted read/write/read has no applicable new writer behavior.

Regression evidence:
Before production edits, both initial targeted tests FAILED: setup:null was accepted as task stack option; sequence:null silently became authored state. /tmp/phase3-grammar-before.log.
After edits, 7/7 new tests PASSED, 0.217 seconds combined, slowest 0.200 seconds: /tmp/phase3-new-matrices2.log.
Initial new YAML tests incorrectly used Markdown-only fixture source resolver and failed NotMarkdown; fixed to actual public YAML document loader / external artifact reference path, then passed.

Legacy runtime/approval coverage:
Old setup/teardown and serial scheduler tests use explicit internal RuntimeFixture construction (schema agent authored helper) rather than production cfg(test) acceptance. New parser matrices exclusively use public normal parsing/preflight. Old empty-sequence/fail_fast tests updated to current grammar. Legacy preflight file fixtures supply leg names centrally to retain focus on reference/approval contracts. Parser-specific execution-removal/nested-group tests use public preflight.

Files owned/changed by grammar agent:
claudine/lib/src/composition/sequence/grammar.rs
claudine/lib/src/composition/sequence/model.rs
claudine/lib/src/composition/sequence/normalize.rs
claudine/lib/src/composition/sequence/reserved.rs
claudine/lib/src/composition/sequence/mod.rs
claudine/lib/src/composition/sequence/formal.rs
claudine/lib/src/composition/sequence/source.rs
claudine/lib/src/composition/sequence/preflight/mod.rs
claudine/lib/src/composition/sequence/preflight/tests.rs
claudine/lib/src/composition/sequence/tests.rs
claudine/lib/src/composition/error/mod.rs (RemovedGrammar docs expanded)

Full library verification is currently running under parent /tmp/phase3-library-full.log; grammar agent will resolve its sequence/preflight failures before final return. No cargo fmt, staging, or commits.

Full-library migration audit:
The parent's full library run reached all sequence tests. Seven new grammar tests passed in that run. Three preflight fixture failures were corrected afterward:
- zero_max_parallel_is_rejected: removed retired execution spelling so the test reaches zero-cap validation.
- default_concurrent_group_rejects_reused_inline_target (renamed obsolete serial-sharing test): public parser now asserts SequenceWriteBackCollision for default concurrent group.
- stack_set_shell_values_are_collected_and_approved: compares approval membership; direct legacy model constructor restores stack sites after the primary graph so constructor-only order differs.
Two task runtime fixture failures were assigned back to schema agent: immutable reference options test should patch an option instead of the now-required leg name; external task leg fixture should author its expected leg name.
Expanded targeted exclusivity test additionally checks all 9 reserved step keys and initialize event placement. Parent owns final combined rerun.


### Task Fixtures final audit

#### Task execution fixture repairs

Changed only `claudine/lib/src/composition/sequence/task/tests.rs`.

- `outcome_contract::a_referencing_site_override_on_an_external_task_is_rejected`: kept the negative external-task patch contract, but used an actual executable-option patch (`timeout: "10s"`). A referencing leg's `name` is now valid grammar, so the earlier name-only input correctly succeeded. Updated the comment to reflect this contract. The assertion still requires the typed `SequenceReferenceInvalid` rejection naming a meaningless task option.
- `serial_groups::an_external_task_inside_a_group_resolves_from_the_group_document`: supplied the required referencing-leg name `external` explicitly. This preserves the existing command-resolution and downstream member-name assertions rather than relying on the old file task name or the fixture's synthetic `fixture-leg-0` name. The shared runtime fixture already restores serial execution on the external group.

No tests or builds launched, as requested; parent owns verification. No formatting, staging, or commits.


### Matrix Audit final audit

#### Phase 3 YAML matrix audit

Changed only:
- claudine/lib/src/composition/sequence/tests.rs, final container_yaml_matrix module
- claudine/lib/src/composition/sequence/preflight/tests.rs, final container_yaml_matrix module

Both matrices now derive field edits from a valid control document, instead of listing independent whole documents. The sequence matrix runs through Markdown and kind:sequence YAML entry forms. The group matrix runs through kind:group and kind:group-catalog external references, then public preflight. No RuntimeFixture changes or production changes.

The sequence matrix covers task sequence absence (headed step), empty list, explicit null, wrong whole type, one bad/all bad elements, duplicate key, malformed value; cancelled absence, empty, null, wrong whole, one bad/all bad elements, duplicate, malformed; task name absence/empty/null/wrong whole/duplicate/malformed; exact sequence:null and setup:null regressions; document fail_fast native/quoted/null rejection; empty sequence root, duplicate root sequence, invalid trailing document. Successful results assert plan size, name, executable presence, and no-op root. The shipped docs/local-runners.yaml and docs/providers.yaml corpus test is retained unchanged.

The group matrix covers required tasks absence/empty/null/wrong whole/one bad/all bad/duplicate/malformed; optional fail_fast absence/native true/native false/quoted true/quoted false/null/wrong whole/empty collection/duplicate/malformed; variables absence/empty/null/wrong whole/duplicate/malformed and arbitrary JSON values (mixed list/all numeric list/null/quoted scalar). Arbitrary JSON values are accepted deliberately, so bad-element types are inapplicable for variables. All four reserved outcome variable names are tested. cancelled covers absent/empty/null/wrong whole/one bad/all bad/duplicate/malformed. Group leg names cover absent/empty/null/wrong type/duplicate YAML key/malformed/nonunique. Trailing invalid document is tested. Successful public graph outputs assert concurrency, leg count/name, outer step count, and variables presence/absence. fail_fast is validated through public acceptance and dependent group shape; it has no persisted preflight property yet.

No builds/tests were run, as requested while the parent was running the package gates. Parent must run targeted container_yaml_matrix tests and broader gates after these edits.


### Sources Cli final audit

#### Phase 3 sequence source CLI migration

Updated `claudine/cli/tests/l1/sequence_sources_cli.rs` only.

- `external_task_group_and_catalog_references_all_load_and_run`: external group and group-catalog members now each declare `name: render`, matching Phase 3 required leg identity. Reference forms, authoring paths, composed state, and `["a", "b", "c"]` assertions are unchanged. Each group has a single member, so implicit concurrency does not change output order.
- `a_formal_document_normalizes_identically_through_both_entry_paths`: original failure is `/sequence/1` schema rejection of arbitrary authored step state (`rank: 3`). Parent has already restored root sequence schema to `unknown[]`; fixture must remain unchanged to preserve parity/state coverage.
- `a_foreign_source_coerces_scalars_and_names_nameless_objects`: original failure likewise is `/sequence/1` schema rejection of a nameless foreign object. Parent schema correction covers this; fixture remains unchanged so scalar and nameless-object regression coverage is retained.

The modified existing test is declared by `cli/tests/l1/main.rs` (`mod sequence_sources_cli`) and has no tier marker, so it remains selected by L1. No parser/config production code changed; no new robustness matrix is needed for this fixture migration.

No builds or tests were launched, per parent coordination. Parent final gates must verify all three tests against the corrected schema. No formatting, staging, or commits performed.


### Lifecycle Docs final audit

Updated `claudine/docs/topics/flow-control/lifecycle.md` only.

- Added parsed `cancelled` event, cleanup action restrictions, `err` cancellation cause availability, and explicit planned dispatch/container execution boundary.
- Removed stale public task `setup:`/`teardown:` guidance from grammar, binding, shell approval, and TTS prewarm sections; documented preceding task/group finalize replacement.
- Preserved prompt lifecycle shell authorization, document TTS prewarm, and current sequence overlay namespace documentation.
- Corrected blanket resume/directive event claims for cancellation restrictions.

Read the claudine skill. Inspected diff and remaining setup/teardown occurrences (now only removal guidance). No source changes, tests, builds, formatting, staging, or commits. Parent owns verification and plan/log metadata.

### Completion audit: empty cancellation opt-in

The completion audit caught an omitted-versus-empty distinction missing from
the lifecycle model: both parsed to empty action vectors. Before fixing it,
`an_empty_cancelled_stack_retains_the_cancellation_opt_in` failed through the
public configuration classification (`is_empty`). `LifecycleConfig` now
retains `cancelled_declared`; the parser sets it only for an authored, valid
stack, including `[]`/`{}`. An explicit opt-in is nonempty configuration even
with no cleanup actions, and survives cloning into runtime state. Absent
remains empty; null remains a typed error. Cancellation dispatch itself is
still deferred. The model documents this invariant and the regression checks
classification, empty actions, and retained cloned state.

The first lint pass found one useless `format!` left after removing a test
fixture's policy placeholder. It now uses a literal string with ordinary
braces and `.to_string()`. No formatting command was used.

The same audit strengthened group matrices to require downstream retention of
`fail_fast` and container lifecycle state. The new public graph assertions
initially failed to compile because those fields did not exist: validation
had discarded them. `PreflightGroup` now retains the boolean policy (absent
means true) and typed `LifecycleConfig`, including an empty cancellation
opt-in. Existing runtime scanning explicitly leaves these inert container
fields to the forthcoming runner. The group/catalog matrix verifies defaults,
false, declaration presence, cloned state, variables, and named members through
the public preflight graph. This supersedes the earlier handoff caveat that
policy/event storage was deferred; only execution remains deferred.

Clippy then identified that storing all eight lifecycle vectors inline made
the group action variant too large. The group owns its lifecycle through a
`Box`, keeping the action enum compact without suppressing the lint or adding
shared mutable ownership. The public graph assertions use the retained typed
configuration unchanged.

### Darkmatter shipped-schema corpus alignment

The root `just test darkmatter` recipe runs the Darkmatter area (library, CLI,
and DMLS). Its passive shipped-schema corpus found an old assertion requiring
Claudine's generated `state`/`previous`/`next`/`outputs`/`sequence_id` overlays
in the authored catalog. The regression was `no state`, not a parser failure.
The assertion now verifies that those overlays and document `fail_fast` are
absent, and that the new optional `cancelled` definition does not admit null.
The test still checks required side-effect input presence, passive effect
counters, and unchanged artifact fingerprints. This is a test-only alignment
with the schema removal required by this phase.

### Verification checkpoints

- Claudine `NEXTEST_TEST_THREADS=4 just test`: 9,658 passed, 9 existing
  skips, 305.681 s; no test exceeded 5 s. This includes the empty cancellation
  opt-in regression. The subsequent retained group-model fields and boxing
  are covered by a final exact-tree area rerun below.
- Group/catalog public graph matrix after retention: 1 passed in 0.118 s.
- Claudine `NEXTEST_TEST_THREADS=4 just lint`: passed after fixing the
  redundant test format and boxing retained group lifecycle data. The recipe
  runs Clippy, transport guards, and the lifecycle documentation guard; it
  does not run cargo fmt.
- Root `NEXTEST_TEST_THREADS=4 just test darkmatter`: 10,114 passed,
  10 existing skips, 98.524 s after aligning the shipped-schema corpus. This
  recipe covers Darkmatter's library, CLI, DMLS and editor adapter packages;
  no monorepo-wide run was performed. The real `md` shipped-bundle CLI tests
  pass, including the added cancellation fixture.

The earlier default-concurrency area runs passed their assertions but showed
load-sensitive >5 s cases. Four local Nextest workers retained the complete
L1 selection and brought every Claudine test under the 5 s budget; no test
timeout, deadline, retry, ignore attribute, or tier was changed for speed.
No Linux/native-Windows/WSL2 runs were added: the changes use existing portable
YAML/file-reference paths and introduce no OS-specific runtime logic. The
Windows signal fixture changed only its authored group grammar. Public
grammar matrices run on every OS; existing Unix shell recorders retain their
platform guard. Other-OS behavioral proof remains with CI. No L2/L3 resource
was required by this phase, and no window was opened or focused.

### Existing skips and earlier timing violations

Claudine's nine unchanged ignored diagnostics:

- `compose_ttff_perf::compose_emits_first_stderr_byte_within_budget`
- `completion_perf::perf_compose_empty_partial_meets_target`
- `completion_perf::perf_compose_long_prefix_meets_target`
- `completion_perf::perf_inline_compose_empty_partial_meets_target`
- `completion_perf::perf_enter_compose_partial_meets_target`
- `system_prompt_perf_bench::bench_system_prompt_resolution_cold_and_warm`
- `system_prompt_perf_bench::bench_resolve_and_prepare_step_by_step`
- `system_prompt_perf_bench::bench_request_topology_probe_and_reuse`
- `system_prompt_perf_bench::bench_raw_darkmatter_compose_passes`

Darkmatter area's ten unchanged ignored cases:

- `f35_6_rhythm_raw_samples` (measurement harness)
- `f23_code_surface_raw_samples` (measurement harness)
- `f35_7_link_policy_raw_samples` (measurement harness)
- `resolve_alias_ll` (requires the user's login-shell alias)
- `a_malformed_alias_target_is_parsed_once_and_reported_by_the_rejecting_alias` (DMLS alias projection pending)
- `undeclared_properties_in_block_and_multi_line_expression_values_warn_at_their_own_ranges` (DMLS projection pending)
- `tagged_anchored_and_aliased_expression_values_are_diagnosed_once_where_authored` (DMLS projection pending)
- `an_aliased_dash_separated_key_fix_avoids_the_defining_scalars_quote` (DMLS projection pending)
- `a_dash_separated_key_behind_an_unprovable_alias_offers_no_fix` (DMLS projection pending)
- `aliases_are_analyzed_in_the_definition_the_yaml_parser_resolved` (DMLS projection pending)

Earlier default-concurrency timing violations from the last such Claudine run
(all assertions passed; the subsequent four-worker run had none):

- `claudine::l1 claudine_schemas_embedded::embedded_json_trigger_combinator`: 5.513 s.
- `claudine::l1 claudine_schemas_embedded::embedded_yaml_trigger_combinator`: 5.430 s.
- `claudine-cli::l1 agent_text_is_data::malformed_structured_agent_values_are_refused_not_saved_as_text`: 7.697 s.
- `claudine-cli::l1 compose_initialize_staged_boot::early_catch_shells_cannot_run_or_enable_another_catch_shell`: 5.835 s.
- `claudine-cli::l1 compose_schema_cli::compose_late_caller_file_verdict_names_the_launch_directory`: 5.063 s.
- `claudine-cli::l1 compose_schema_cli::compose_unresolved_caller_file_fails_before_initialize_when_non_interactive`: 6.632 s.
- `claudine-cli::l1 ctx_launch_anchor::loop_route_keeps_launch_area_for_root_and_package_documents`: 5.241 s.
- `claudine-cli::l1 provider_tail_launch::embedded_credentials_reach_the_child_and_no_display_surface`: 5.821 s.
- `claudine-cli::l1 provider_tail_launch::other_failures_are_not_attributed_to_the_tail`: 5.540 s.
- `claudine-cli::l1 provider_tail_notice::a_switch_whose_record_is_typed_unknown_is_not_called_known`: 6.987 s.
- `claudine-cli::l1 tts_prewarm::every_accepted_playa_dry_run_spelling_prepares_nothing`: 5.838 s.

All implementation/build/fixture failures encountered in this phase are
recorded above and corrected. No pre-existing assertion or lint failure is
being waived. The only compiler warning observed was the existing macOS
large unwind-table linker warning.

### Final requirement-to-test index

- Cancellation action vocabulary, authored paths, expression/side-effect
  families and error bindings: lifecycle grammar's six `cancelled_*` tests.
  Explicit empty opt-in classification and clone retention:
  `an_empty_cancelled_stack_retains_the_cancellation_opt_in` (red before fix,
  then green in 0.018 s).
- Criterion 14: `lifecycle_lift_ready_guard`'s module checks and
  `criterion_14_deliberate_runtime_import_fails_the_guard`; the deliberate
  forbidden import triggers the real guard assertion in an isolated string.
- Nested executable, six-way exclusivity, removed keys, reserved step keys
  and event placement: `container_grammar::nested_sequence_and_removed_task_keys`
  and `sequence_task_shape_matrix_and_exclusivity`.
- YAML representation robustness and downstream plan/graph state:
  `container_yaml_matrix::markdown_and_sequence_yaml_fields_matrix` and
  `group_and_catalog_file_fields_matrix`; the latter now also asserts retained
  false/default policy and cancellation declaration after cloning the model.
- Groups: `container_grammar::group_fields_matrix` and
  `group_legs_require_unique_nonempty_names_and_execution_is_removed`; CLI
  removed-execution tests witness no output file, and concurrency tests retain
  overlap, caps, isolated mutations and declaration-ordered publication.
- Criterion 2 CLI policy removal: the three `criterion_2_fail_fast_*` tests,
  `removed_fail_fast_is_absent_from_sequence_help`, and partition's
  `removed_sequence_policy_is_typed_before_any_source_read` (attached/missing
  values, owned values and explicit opaque tails). Existing task-stack
  regression matrices now prove typed refusal and no shell/provider/prewarm.
- Environment removal: `sequence_does_not_inject_fail_fast_into_child_env_or_prompt`
  and `sequence_does_not_publish_a_fail_fast_env`, plus operation propagation
  across handoffs and captured native shell-environment tests.
- Shipped artifacts and schemas: `shipped_sequence_corpus_uses_normal_resolution_path`,
  `sequence_container_schema_contract`, the full existing schema corpus/matrix,
  `shipped_container_schema_resolves_for_authored_parameter_completion`, and
  Darkmatter CLI's `claudine_schema_bundle` module using real shipped files
  through `md schema validate`. The passive shipped definition test now
  asserts retired overlays are absent and cancellation rejects null.

All new/renamed tests are in declared library unit targets or existing L1
modules; no new test file was stranded under an autotests=false manifest, no
new feature gate was introduced, and no new CI cell was added.

### Final exact-tree gates and closing state

- macOS `NEXTEST_TEST_THREADS=4 just test` in Claudine: **9,658 passed**,
  9 unchanged ignored diagnostics, 304.767 s; **no test exceeded 5 s**. This
  run includes the final boxed group lifecycle, retained false/default policy,
  cancellation opt-in, and every test fixture migration.
- `NEXTEST_TEST_THREADS=4 just lint` in Claudine: **passed**, including
  Clippy on all five primary area crates, transport guards and documentation
  guard. No cargo fmt command was run.
- Root `NEXTEST_TEST_THREADS=4 just test darkmatter`: **10,114 passed**,
  10 unchanged skips, 98.524 s; **no test exceeded 5 s**. The shipped-artifact
  CLI/corpus and DMLS coverage passed.
- Root `just check-tier-coverage claudine`: **0 stranded tests**. New tests
  use existing declared targets and running L1 tiers/features.
- `git diff --check`: passed. Frontmatter was read back with `md get` for
  every requested key; inventories include all 65 source/test/schema inputs,
  seven updated documents (including plan/log), no created documents, and the
  one updated Claudine skill. The cumulative package list preserves earlier
  packages and adds Darkmatter's library and CLI.

All Phase 3 tasks and checkpoint boxes are checked. Later phases and the
overall feature checklist remain open. The specification snapshot was not
changed, and the feature remains in its active directory. Earlier staged
work was preserved; this session performed no staging or Git commits.
`human_review` is false, and the replacement handoff message points to Phase 4
and calls out the retained container data and explicit cancellation presence.

**Implementation complete, ready for review.**

## Phase 4

### Requirement-to-test map before implementation

- Binding catalog and criterion 9: extend the existing public EvaluationSession binding matrix for `seq`, cancellation, standalone outputs, step boundaries, whole-root shadow/restore, authored `fm` access, and string versus expression presentation.
- Group outcomes and criterion 8's fixture-testable contract: exercise typed group result construction through public expression evaluation for every event, declaration-ordered summaries, skipped/unadmitted legs, partial failure output, and leg-local variable-only views.
- Hard rename and criteria 10/11: walk every section 7 rename row through parsed body/frontmatter/condition/lifecycle/shell reads, with dotted/static-index forms and authored/supplied null controls. Assert typed replacement errors and that preparation never reaches effects.
- Generated overlays: inspect the existing producer/consumer boundary before removal; the plan requires replacement production and removal in the same change. Preserve ordinary authored properties and verify inline persistence never writes runtime roots.
- Regression coverage uses the existing declared library unit targets and CLI L1 integration modules. No terminal/browser resource is needed. Run targeted regressions first, then `just test`, `just lint`, and the tier coverage audit sequentially.

The checkout contains staged work from prior phases; it will be preserved. No formatting, staging, or commits are authorized.

### Binding and transport checkpoint

The new binding regressions passed in the first area run: complete `seq`
shadow/restore, standalone output-only root, cancellation error availability,
and all group event timings. Group results retain declaration order and
structured partial output/cancellation causes; leg-local views contain only
variables. Task 4.2 is checked immediately.

The existing runtime now supplies `seq` alongside override layers rather than
injecting generated frontmatter. `CallerInputLayers` and `PrepareOptions`
carry that separate value across JIT rereads. Runtime output publication no
longer inserts an `outputs` frontmatter property. Existing tests that assumed
those old overlays are being migrated; the initial broad run reports those
expected contract changes rather than being accepted as final proof.

### Scope and integration checkpoint

Task 4.1 is checked: the public preparation regression verifies first,
middle, and last step roots, duplicate-name IDs, shallow neighbors, typed
whole-row expressions, name shorthand, and authored `fm.seq` preservation.
The catalog matrix now includes cancellation. The original error matrix
walks every rename row, including static `state['sequence_id']`, and every
presence control (null, false, zero, empty string/list/object).

The second broad run passed the library suite and isolated remaining CLI
fixtures that assembled old expression text from macro arguments, plus raw
loop/when predicates. Those inputs are migrated rather than adding aliases.
The loop condition/gate now reads runtime outputs directly, and sequence
configuration remains deferred during source composition. Step-scope proxy
adoption retains sequence bindings alongside caller overrides and lexical
group variables.

A shared-interface extension is necessary for transcluded documents:
Darkmatter's explicit `HostGlobals::with_document_fallbacks` lets a reading
document declare its own retired-named property, including null, while an
undeclared read retains the registered typed migration error. Normal lexical
roots still shadow complete roots. Frontmatter and body lookups share the
rule, and it participates in compose identity. Name presentation now honors
the existing configured coercion for globals without changing typed values.

Task 4.3 is checked after the parsed-read unit matrix, normal-preparation
presence controls, and all 13 CLI rename cases passed. Checks now run before
CLI dynamic source expansion as well as in document preparation and external
task/group loading. Conditions and operands use parsed lifecycle surfaces;
shell/interpolated values use parsed spans. Static selectors are preserved in
replacement selection; `fm.*`, `ctx.state`, literal strings, and code blocks
retain their documented meanings. The transcluded-document regression passes
for an authored value, authored null, and an undeclared read that must fail.

The runtime-root audit also removed group-variable publication into prompt
overrides: `PromptTaskRequest.lifecycle_group` carries that scope, while
`fm.group` remains authored data. Sequence/group roots are not stored in live
frontmatter or persisted by the inline closure.

### Timeout regression found by the full gate

The first three area runs each timed out in
`lifecycle_message_drain_interrupt::a_second_ctrl_c_during_the_sequence_drain_exits_130`.
The shared `tests/common/drain_interrupt.rs` document still read the exact old
input `{{ state.name }}`. Preparation correctly rejected it, so no message
reached the listener the test awaited. The fixture now uses `seq.name`.
`shared_drain_documents_pass_the_binding_catalog_without_starting_a_listener`
passively checks all three document-producing drain fixtures through the
public parser/catalog, catching this failure without opening a listener or
waiting on the production drain budget. No timeout, retry, tier, or deadline
was relaxed. The fourth in-progress run predates this fixture correction;
the final run must include it.

Earlier broad run evidence (all failures are being corrected, not waived):
9555 passed / 108 failed / 1 timeout; then 9625 / 54 / 1; then 9679 / 2 / 1.
The remaining two assertions in the third run were exact-byte approval of a
now-late sequence root and a schema relying on an injected state property.
They now use explicit template approval and an authored typed row projection.

Task 4.4 is checked after the fourth run passed the runtime-layer, body,
loop accumulation, proxy, group-scope, and inline persistence assertions.
`inline_compose_success_sees_this_runs_output` now performs two disk
read/write/read cycles and preserves an authored `seq` property while
asserting that no generated roots appear. The source-level audit found no
remaining implicit `fm.state`/`fm.outputs` access in current implementation
code. Internal live prompt migration remains Phase 8 work.

The plan's 4.4/Phase 5 coordination note was resolved by supplying `seq` from
the existing step runtime in this change, so overlay removal did not leave a
producer gap. Nested container execution and its own event dispatch remain
Phase 5; no nested runner was added prematurely.

### Final requirement-to-test mapping

| Requirement | Observable proof |
| --- | --- |
| 4.1 / criterion 9: row fields, IDs, boundaries, typed objects, presentation, authored namespaces | `prepare::tests::sequence_binding_composes_typed_rows_shorthand_and_authored_namespaces`; the binding matrix's `sequence_roots_shadow_completely_and_restore_without_merging` and `standalone_sequence_has_only_outputs_and_cancelled_has_an_error_binding` |
| 4.1: read-only roots, including loop actions | `runtime_state::tests::set_rejects_every_reserved_root_key`, `lifecycle::executor::tests::runtime_set::set_refuses_every_reserved_root_key`, `looping::actions::tests::set_rejects_reserved_properties` |
| 4.2: group records, declaration-ordered summaries, empty early events, private leg views | `bindings::tests::group_results_obey_event_timing_and_declaration_order`; the existing public group/catalog YAML matrix retains its reserved-name and malformed-input checks |
| 4.3 / criteria 10 and 11: every rename, static indexes, source presence and null | `bindings::rename::tests::every_rename_row_is_typed_and_property_presence_preserves_data`, `parsed_surfaces_include_conditions_operands_and_shell_values`, `explicit_frontmatter_unrelated_members_literals_and_code_are_not_renames`, and `prepare::tests::rename_presence_controls_apply_through_normal_preparation` |
| Parsed body conditions in inactive/nested branches | `bindings::rename::tests::body_directive_conditions_are_checked_in_inactive_and_nested_branches`; uses Darkmatter's public directive/page-block parsers, never textual name matching |
| Reading-document ownership through transclusion | `prepare::tests::transcluded_rename_reads_respect_the_reading_documents_property_presence` (authored value, authored null, undeclared error) |
| Normal CLI refusal for every table root | `sequence_errors_cli::{rename_state, rename_state_member, rename_state_static_index, rename_previous, rename_previous_state, rename_next, rename_next_state, rename_outputs, rename_sequence_id, rename_step, rename_total_steps, rename_is_first, rename_is_last}`; `rename_cli_matrix_covers_the_catalog` guards the list |
| Refusal precedes dynamic source execution | `sequence_errors_cli::a_rename_error_precedes_dynamic_sequence_source_effects`, using the exact input `sequence: '$(must-not-run)'` with body `{{ outputs }}` |
| 4.4: generated roots stay outside frontmatter and disk state | Runtime-layer precedence/origin tests, the typed-row preparation test, and `composition_outputs::inline_compose_success_sees_this_runs_output` with two read/write/read cycles and an authored `seq` object |
| Existing consumers retain the same outputs and scope | Sequence/group/JIT/handoff suites; `composition_outputs::a_loop_accumulates_outputs_and_retains_mutations_across_iterations`; the committed-accumulator lifecycle test now reads `seq.outputs` directly from the runtime cell without seeding a frontmatter alias |
| Public schema behavior after removing overlays | `sequence_overlay_pty::pty_sequence_authored_row_projection_satisfies_required_property`: explicit `state: '{{ seq }}'` satisfies its schema; no generated frontmatter is implied |
| Passive shipped artifacts and normal invocation | Updated `container_yaml_matrix::shipped_sequence_corpus_uses_normal_resolution_path` covers both shipped sequence data files; `sequence_sources_cli::shipped_runner_sequence_renders_lexical_rows_through_the_cli` uses the real shipped local-runner document through normal CLI dry-run; the complete shipped schema corpus remains in the area gates |
| Shared generated test documents remain valid | `lifecycle_message_drain_interrupt::shared_drain_documents_pass_the_binding_catalog_without_starting_a_listener`, plus the existing real signal/drain regression |

No configuration field or serialization format was added. The presence
matrix treats every ordinary authored JSON shape as data: omitted names
raise the migration error; null/false/zero/empty string/list/object preserve
ordinary properties, both authored and supplied. Existing YAML/formal-source
robustness matrices cover malformed documents, duplicates, trailing content,
and strict container fields. Runtime-only structs are constructed by the
runner, not deserialized from user configuration.

All new tests are compiled by existing declared library modules or CLI L1
modules. No tier marker, required feature, ignore, retry, or CI cell was
added. No terminal emulator or browser window was opened or focused; the changed
existing PTY test remains a non-GUI L1 test.

### Verification corrections

The first scoped lint found two unnecessary borrows and a single-element
loop left by a fixture migration; these were corrected. A subsequent lint
was accidentally invoked from the repository root, began unrelated package
Clippy checks, and was terminated with its descendants. It was not accepted
as validation; the correctly scoped Claudine lint passed afterward. No
formatting, staging, or commit command was run.

The metadata read-back caught a newline error in the append helper before
continuing. The helper and affected boundaries were corrected immediately;
`md get` successfully read every requested field afterward. Prior-phase
metadata, cumulative packages, and earlier log entries remain intact.

### Complete Phase 4 source/test inventory

```text
claudine/cli/src/commands/compose/prep.rs
claudine/cli/src/commands/sequence.rs
claudine/cli/src/commands/wrap/composition/pipeline.rs
claudine/cli/src/commands/wrap/composition/preflight.rs
claudine/cli/src/commands/wrap/composition/staged_boot.rs
claudine/cli/src/commands/wrap/harness_orch/loop_control.rs
claudine/cli/src/commands/wrap/harness_orch/loop_control/lifecycle_events.rs
claudine/cli/src/commands/wrap/harness_orch/shell_options.rs
claudine/cli/src/commands/wrap/sequence/iterate.rs
claudine/cli/src/commands/wrap/sequence/jit.rs
claudine/cli/src/commands/wrap/sequence/jit/tests.rs
claudine/cli/src/commands/wrap/sequence/task_run.rs
claudine/cli/tests/common/drain_interrupt.rs
claudine/cli/tests/fixtures/frozen_prompts/prompts/_implement/implement-plan.md
claudine/cli/tests/fixtures/frozen_prompts/shipped_implement_route/_implement/implement-plan.md
claudine/cli/tests/l1/agent_text_is_data.rs
claudine/cli/tests/l1/compose_caller_file_provenance.rs
claudine/cli/tests/l1/compose_frontmatter_model.rs
claudine/cli/tests/l1/compose_removed_validation_keys.rs
claudine/cli/tests/l1/composition_outputs.rs
claudine/cli/tests/l1/handoff_owners.rs
claudine/cli/tests/l1/inline_completion_lifecycle.rs
claudine/cli/tests/l1/lifecycle_message_drain.rs
claudine/cli/tests/l1/lifecycle_message_drain_interrupt.rs
claudine/cli/tests/l1/loop_cli.rs
claudine/cli/tests/l1/preflight_execution_parity.rs
claudine/cli/tests/l1/sequence_cli.rs
claudine/cli/tests/l1/sequence_errors_cli.rs
claudine/cli/tests/l1/sequence_groups.rs
claudine/cli/tests/l1/sequence_jit.rs
claudine/cli/tests/l1/sequence_magic_reference.rs
claudine/cli/tests/l1/sequence_overlay_pty.rs
claudine/cli/tests/l1/sequence_perf.rs
claudine/cli/tests/l1/sequence_prompt_property.rs
claudine/cli/tests/l1/sequence_schema.rs
claudine/cli/tests/l1/sequence_shell_templates.rs
claudine/cli/tests/l1/sequence_sources_cli.rs
claudine/cli/tests/l1/wrap_compose_validation.rs
claudine/lib/src/composition/error/render/lifecycle.rs
claudine/lib/src/composition/error/tests.rs
claudine/lib/src/composition/lifecycle/bindings.rs
claudine/lib/src/composition/lifecycle/bindings/containers.rs
claudine/lib/src/composition/lifecycle/bindings/rename.rs
claudine/lib/src/composition/lifecycle/bindings/rename/tests.rs
claudine/lib/src/composition/lifecycle/bindings/tests.rs
claudine/lib/src/composition/lifecycle/context/tests.rs
claudine/lib/src/composition/lifecycle/executor.rs
claudine/lib/src/composition/lifecycle/executor/tests/filesystem_lookup.rs
claudine/lib/src/composition/lifecycle/executor/tests/mod.rs
claudine/lib/src/composition/lifecycle/executor/tests/runtime_set.rs
claudine/lib/src/composition/lifecycle/prewarm/classify/tests.rs
claudine/lib/src/composition/lifecycle/prewarm/sequence/tests.rs
claudine/lib/src/composition/lifecycle/prewarm/tests.rs
claudine/lib/src/composition/lifecycle/tests/audio_emission.rs
claudine/lib/src/composition/lifecycle/tests/guard_runtime.rs
claudine/lib/src/composition/looping/actions/tests.rs
claudine/lib/src/composition/looping/engine.rs
claudine/lib/src/composition/looping/expression.rs
claudine/lib/src/composition/preflight.rs
claudine/lib/src/composition/preflight/tests.rs
claudine/lib/src/composition/prepare.rs
claudine/lib/src/composition/prepare/tests.rs
claudine/lib/src/composition/reserved.rs
claudine/lib/src/composition/runtime_state.rs
claudine/lib/src/composition/runtime_state/tests.rs
claudine/lib/src/composition/sequence/mod.rs
claudine/lib/src/composition/sequence/model.rs
claudine/lib/src/composition/sequence/preflight/mod.rs
claudine/lib/src/composition/sequence/preflight/tests.rs
claudine/lib/src/composition/sequence/reserved.rs
claudine/lib/src/composition/sequence/task/group.rs
claudine/lib/src/composition/sequence/task/mod.rs
claudine/lib/src/composition/sequence/task/templates.rs
claudine/lib/src/composition/sequence/task/tests.rs
claudine/lib/src/composition/sequence/tests.rs
claudine/lib/src/composition/types.rs
claudine/lib/tests/l1/agent_errors_fleet.rs
darkmatter/lib/src/markdown/compose/context/effective_state.rs
darkmatter/lib/src/markdown/compose/expression/binding.rs
darkmatter/lib/src/markdown/compose/frontmatter_interpolation.rs
```


### Final fixture and override audit

The first final run stopped on two frozen implementation-router inputs:
their optional application flag was written as bare `is_last` in a directive
condition. Both frozen copies now read `fm.is_last`, preserving the ordinary
frontmatter flag, including its absent/null behavior; they were not changed
to a generated sequence flag. The three shipped-router acceptance tests
passed afterward (1.446 s total). Live internal prompts remain untouched.

The next full Claudine run passed **9,685 tests**, with nine existing ignored
cases and no test over five seconds (303.039 s). The previously timing-out
sequence drain case passed in **2.169 s**, and its new passive corpus test
passed in **0.017 s**.

`prepare::tests::rename_validation_uses_active_authored_overrides_and_never_scans_runtime_data`
then passed in **0.242 s**. It proves that an overridden authored template is
not a live read, a runtime-data value containing the exact old syntax
`{{ state.name }}` stays literal, and an authored override reading that name
in an inactive branch is still rejected. Source checks use the effective
authored layer while preserving supplied-property presence; data layers are
not parsed as templates. The sequence preflight retains the plan's already
resolved sequence configuration when layering caller properties.

All five Phase 4 task boxes are now checked. Final validation below includes
the last override regression and the removal of obsolete group-state
parameters; later-phase task boxes remain open.


### Final Phase 4 gates and handoff

All final checks passed on macOS, using `NEXTEST_TEST_THREADS=4` for area
runs. These results include the last override regression and group-parameter
cleanup:

| Gate | Final result |
| --- | --- |
| Claudine `just test` | **9,686 passed**, nine existing skips; 302.629 s |
| Claudine `just lint` | Passed for all five area packages, transport guards, and documentation facets |
| `just check-tier-coverage claudine` | Passed; zero stranded tests |
| Darkmatter `just test` | **10,114 passed**, ten existing skips; 101.285 s |
| `just _lint darkmatter` | Passed; 25.46 s |

No test exceeded five seconds in either final area run. Claudine's longest
was 4.550 s; Darkmatter's longest was 3.364 s. The targeted tests and
requirement mapping are listed above, including every rename row, parsed
conditions and lifecycle operands, source-presence controls, override origins,
whole-root shadowing, group result timing, shipped artifacts, and persistence
round trips. Earlier failures were corrected; no unresolved failure is waived.


Claudine's nine unchanged ignored diagnostics:

- `compose_ttff_perf::compose_emits_first_stderr_byte_within_budget`
- `completion_perf::perf_compose_empty_partial_meets_target`
- `completion_perf::perf_compose_long_prefix_meets_target`
- `completion_perf::perf_inline_compose_empty_partial_meets_target`
- `completion_perf::perf_enter_compose_partial_meets_target`
- `system_prompt_perf_bench::bench_system_prompt_resolution_cold_and_warm`
- `system_prompt_perf_bench::bench_resolve_and_prepare_step_by_step`
- `system_prompt_perf_bench::bench_request_topology_probe_and_reuse`
- `system_prompt_perf_bench::bench_raw_darkmatter_compose_passes`

Darkmatter area's ten unchanged ignored cases:

- `f35_6_rhythm_raw_samples` (measurement harness)
- `f23_code_surface_raw_samples` (measurement harness)
- `f35_7_link_policy_raw_samples` (measurement harness)
- `resolve_alias_ll` (requires the user's login-shell alias)
- `a_malformed_alias_target_is_parsed_once_and_reported_by_the_rejecting_alias` (DMLS alias projection pending)
- `undeclared_properties_in_block_and_multi_line_expression_values_warn_at_their_own_ranges` (DMLS projection pending)
- `tagged_anchored_and_aliased_expression_values_are_diagnosed_once_where_authored` (DMLS projection pending)
- `an_aliased_dash_separated_key_fix_avoids_the_defining_scalars_quote` (DMLS projection pending)
- `a_dash_separated_key_behind_an_unprovable_alias_offers_no_fix` (DMLS projection pending)
- `aliases_are_analyzed_in_the_definition_the_yaml_parser_resolved` (DMLS projection pending)


No new ignore, tier marker, CI cell, or relaxed deadline was introduced. L2/L3
and remote OS runs were not needed: the changes affect expression/catalog
values and their existing in-process/CLI/file persistence paths, with no new
OS-specific implementation. The full existing L1 PTY tests ran without
opening or focusing a terminal window. The unchanged macOS linker unwind
warning remains non-fatal.

The complete source/test inventory is above and in both documents' Phase 4
frontmatter: 80 files (78 Rust files and two frozen prompt inputs). Seven
documentation files and two skill files were updated; no documentation file
was created. Both frontmatter blocks were read back with `md get`, preserving
prior-phase fields and the cumulative package list. The spec was not changed.

Implementation complete, ready for review. All Phase 4 tasks are checked;
Phase 5's container runner and later phases remain open. `human_review` is
false, and `message_to_agent` contains the replacement Phase 5 handoff. No
formatting, staging, or commit command was run.


## Phase 5

### Requirement-to-test map before implementation

- Root event order, initialization before preflight, blocked cleanup, skip, retry exhaustion and runtime overlays: new hermetic CLI `sequence_containers` tests, plus the shared runner's in-process event recorder tests.
- Body-only steps inherit root events without firing them per step: CLI fake-provider recording test.
- Native nested arrays and group dictionaries, lexical sequence restoration, shared run ID, empty nested sequences and by-reference execution: nested runner tests and normal CLI sequence invocations.
- Atomic group state, tolerant groups, declaration-order bounded admission, unadmitted outcomes and terminal summaries: injected group-runner tests with deterministic barriers and CLI downstream-state assertions.
- Failure propagation and exactly-once finalize, including terminal/finalize errors: lifecycle runner tests and nested CLI event traces. Timeouts retain failure routing.
- Retry clears abandoned child outputs while retaining state and commits one final result: shared runner/nested/group tests and root CLI retry trace.
- Recursive dry-run performs shell work without events/provider launch/write-back: CLI regression coverage.

The phase changes runtime behavior rather than adding configuration fields; existing Phase 3 robustness matrices remain the parser contract. New container variants will be covered through the normal preflight and CLI paths. L1 hermetic subprocess tests cover filesystem/CLI boundaries; no real terminal is needed for this phase's non-cancellation behavior.

### Foundation and native results

- Added the shared guarded `ContainerRunner`, native task result values, and lexical accumulator restoration/clearing. Foundation nextest filter `container_runtime` passed 4/4. The native-result CLI regression passed with an outer string, nested array, and named group dictionary.
- Root initialize now precedes static preflight; empty roots enter their lifecycle. Blocked cleanup, skip before invalid preflight, typed container resume refusal, finalize failure, and recursive dry-run targeted CLI cases passed. Two remaining targeted cases exposed body-step execution and a test recovery-input issue; investigation continues before checking off the runner.
- The original targeted attempt encountered temporarily incomplete parallel edits at compile time. A later run completed 8 tests (2 passing, 6 failing); fixture provider/skip corrections and dry-run suppression led to 7/9 passing. These were development failures, not accepted gaps.
- Existing ordinary document retry/resume dispatch already re-enters before finalize. Added event-recorder regressions at that dispatch boundary to protect it.

### Nested sequences

- Inline, referenced data, and referenced Markdown containers now preserve native arrays, shared run identity, unique local IDs and shallow neighbors, lexical outputs, and enclosing bindings on return. The existing Markdown parser is reused for referenced Markdown; no field semantics changed.
- Nested sources are reread before start and retry. New child commands or prompt targets introduced after approval closes are refused before start. The source file's original authoring context remains authoritative.
- Targeted preflight/runtime nextest filters passed 11/11 in 0.284 s, covering positions, source suffixes/origin, cycles, body-only Markdown, native results/restoration, empty arrays, retry retention, finalize failure, and no publication on failure.

### Groups and just-in-time preparation

- Group legs now admit in declaration order under the concurrency cap, use isolated runtime branches, and merge in declaration order only after successful group cleanup. Failed atomic groups withhold all staged leg state/results. Tolerant groups keep failed-leg mutations and expose structured terminal summaries.
- Seven `container_group_contract` regressions pass, including deterministic Condvar synchronization, retry ordering/state, skipped legs, terminal summaries, and finalize failure.
- Eight `group_preparation_contract` regressions pass for inline, referenced group and catalog rereads; retry rereads; unapproved command/template/new prompt refusal; malformed reread blocked cleanup. Every test completed below 0.1 s.
- Preparation rereads source bytes. Unchanged documents reuse the already audited graph nodes while variables resolve afresh; changed sources are reparsed and compared with the original approvals. This also preserves the intentional internal legacy stage-fixture models without allowing retired public grammar.
- Ordinary retry/resume event recordings and the full area L1 gate remain in verification. No formatting, staging, or commits have run.

### Additional public runtime regressions

- Before editing tests, mapped acceptance criteria 4, 6, 11, 16, 18, 20, and 23 to `TaskExecution::run` through current-grammar preflight fixtures. Added a declared child unit-test module, `task/container_regressions.rs`; its unmarked test names remain selected by L1 and compile through the library's existing unit-test target.
- Deterministic concurrent nested-sequence legs use a bounded Condvar signal, with no sleeps or process launches. Leg A mutates its private state, waits until leg B writes an upcoming referenced source, then observes its own mutation and the changed file while retaining the original sibling value. Parent mutations and native outputs are asserted after settlement.
- Added nested failure propagation with innermost-first cleanup and no later child launch, and a timed-out leg that retries through failure without dispatching cancellation.
- Added the combined four-container lexical-binding shape, successful proxy cleanup ownership, nested concurrent inline-write collision refusal, and two named three-child/two-retry cases covering retained writes, cleared attempt outputs, exhaustion, exactly-once finalize, source-file immutability, and final publication.
- The initial targeted development run exposed two fixture mistakes (a communication verb under `side_effect`, and the shell fake requiring a stdout mapping for its configured failure). Those fixtures were corrected. The next targeted run passed the failure-order and timeout cases; the remaining case's semantic assertions passed and its expected side-effect result was corrected to the public prior-value JSON output. Final verification of the settled eight-test module is delegated to the phase's sequential area gate.

### Root execution and failure cleanup verification

- The declared `sequence_containers` CLI module passed 13/13 in 1.416 s. It exercises normal invocation, native nested results, initialization before preflight, blocked cleanup with shells disabled, skip, no-session resume, body-only steps, root retry, finalize failure, empty roots, recursive dry-run, and proxy cleanup ownership.
- Added the executable documentation artifact `docs/examples/sequence-containers.md`, a passive corpus test walking every shipped example's container lifecycle, and an end-to-end invocation of that exact artifact. The CLI fixture asserts source-file immutability and no audio spool publication.
- A full-suite regression exposed missing transitive inline-write collision checks. The checker now walks each concurrent leg's nested groups/sequences, deduplicates ordered writes within a leg, and rejects collisions across legs. The exact nested two-leg failing input is retained in `nested_sequence_legs_reject_shared_inline_writeback_during_preflight`.
- Added regressions for body preparation failures reaching enclosing failure/finalize and for cleanup errors remaining secondary to an original sequence/group failure. Shared runner diagnostics and nested propagation now retain both channels.
- Migrated the remaining group consumers from positional array access to named dictionary members. A legacy sibling-completion test now uses a bounded Condvar to ensure both siblings are admitted before failure, replacing its timing assumption.
- The area gate also found the pre-existing working-tree Codex test `composition_yolo_frontmatter_disables_the_codex_sandbox` invoking a file as a root subcommand. Added its missing `compose` argument without changing the surrounding unrelated implementation.
- Root lifecycle tests are green; migration of older per-body-step lifecycle fixtures and the final full gates remain in progress. The plan's implementation tasks are checked, while its full-gate checkpoints remain open.

### Acceptance criteria and concrete verification

All new tests are L1: hermetic CLI/filesystem tests or injected public execution tests. The new CLI file is declared by `cli/tests/l1/main.rs`; library child modules are declared by the existing library unit-test target. No new feature-gated or terminal/browser target was introduced.

| Criterion | Concrete tests |
| --- | --- |
| 1: native nesting, references, concurrent ordered legs | `root_events_wrap_all_children_once_and_expose_native_results`; `referenced_sequence_suffix_and_origin_are_preserved`; `nested_group_legs_keep_memory_private_but_reread_shared_files_between_steps` |
| 3: container event order, blocked and skip | `root_initialize_skip_precedes_invalid_static_preflight`; `preflight_failure_emits_blocked_then_finalize_without_start`; `group_events_see_settled_summaries_and_finalize_before_publication`; `initialize_skip_runs_no_children_or_finalize_and_records_skipped_leg` |
| 4: failure propagation and recovery | `unhandled_nested_failure_finalizes_innermost_first_and_never_starts_later_children`; `root_resume_reports_typed_no_session_error_and_cleans_up`; `retry_restarts_legs_without_intermediate_finalize_or_duplicate_result` |
| 5, non-cancellation: atomic and tolerant groups | `default_fail_fast_withholds_mutations_outputs_and_unadmitted_legs`; `tolerated_failed_legs_merge_mutations_and_publish_native_named_results` |
| 6: timeout routes through failure | `timed_out_nested_leg_uses_failure_retry_and_never_cancellation` |
| 8: lexical group variables and terminal summaries | `group_events_see_settled_summaries_and_finalize_before_publication`; existing group-name/reserved-variable grammar matrix |
| 9: complete lexical sequence positions | `nested_prompt_observes_shared_run_and_shallow_neighbors`; `inline_sequence_preflight_preserves_nested_positions_and_duplicates`; Phase 4 binding tests |
| 11: innermost roots and restoration | `inner_group_and_sequence_roots_are_lexical_and_restored_after_return`; `child_results_are_native_and_parent_position_is_restored` |
| 12: body steps have no own lifecycle | `body_only_steps_run_root_success_once`; `referenced_markdown_body_step_has_no_document_lifecycle` |
| 13: publication after finalize only | `finalize_failure_discards_nested_native_result`; `group_finalize_error_discards_leg_mutations_and_result` |
| 16: proxy cleanup ownership | `root_failure_proxy_runs_target_cleanup_without_source_cleanup`; `container_failure_proxy_skips_source_finalize_and_runs_target_finalize`; same-thread and group-thread proxy-cycle tests |
| 18: empty, skipped, cycles and collisions | `empty_root_sequence_runs_one_lifecycle_with_empty_outputs`; `empty_sequence_returns_an_empty_native_array`; `sequence_reference_cycle_reports_the_active_chain`; `nested_sequence_legs_reject_shared_inline_writeback_during_preflight` |
| 19, non-cancellation: cleanup and state | `blocked_cleanup_cannot_run_shell_commands`; `body_preparation_failure_runs_enclosing_failure_and_finalize`; `cleanup_errors_stay_secondary_to_the_original_container_failure`; atomic/tolerant group cases above |
| 20: repeated retries, read timing and event adjacency | `two_retries_retain_mutations_and_publish_exactly_three_children_once`; `exhausting_two_retries_keeps_failure_and_publishes_nothing`; `nested_group_legs_keep_memory_private_but_reread_shared_files_between_steps`; `document_retry_reenters_directly_without_finalizing_failed_attempt`; `document_resume_reenters_directly_without_finalizing_failed_attempt` |
| 23: runtime-only retry values | `root_retry_retains_writes_clears_abandoned_outputs_and_finalizes_once`; `retry_clears_child_outputs_and_finalizes_once_before_publication`; group retry and source-immutability assertions |

Additional negative coverage includes invalid retry state, unapproved command/template/prompt introductions, malformed group rereads, source cycles, and finalizer failures. The shipped example corpus is read passively and the exact example is run through `claudine sequence`. Existing schema/grammar corpus and robustness matrix tests remain in the full gate; this phase adds no new load-bearing configuration field. Markdown source support reuses the existing frontmatter parser rather than introducing a second parser.

### Documentation and comment drift

- Replaced planned-runner markers in the current sequence topic, README overview, and Claudine skill with implemented behavior. Cancellation remains explicitly planned; container shell discovery and approval-position work remain assigned to Phase 7.
- Added lifecycle/recovery and failure-propagation Mermaid diagrams, native result examples, atomic in-memory state limits, and the real-effects dry-run warning.
- Corrected the prompt-versus-sequence diagnostic: a prompt leaf pointing at a sequence now names `sequence: <file>` rather than claiming all nesting is unsupported. Updated its existing message assertions.
- Updated the task module documentation to describe native container dispatch; legacy setup/teardown models remain internal fixture machinery, never supported authoring grammar.
- Preserved the root proxy action's authored location from the evaluated control instead of synthesizing an initialize location for every handoff.


### Full-suite fixture migration and runtime guards

- Migrated `composition_outputs`, `sequence_jit`, `document_environment_routes`, and `inline_completion_lifecycle` fixtures that previously attached per-step events to the root. Explicit prompt tasks now own those events; completion failure still proves that step three never launches after step two fails.
- Updated `handoff_owners::sequence_start_proxy_replaces_the_container_before_any_body_step` to assert container-level proxy ownership: target cleanup runs, the source never finalizes, and no source child starts.
- Added `ctx_per_run::container_events_refresh_current_after_nested_child_work`: root, group, and leg start events observe no staged files; their successful terminal events observe the child shell's staged file. The normal CLI test passed in 1.118 s under the area run.
- Added `invalid_root_retry_state_still_finalizes_once`; invalid retry overlays now enter blocked cleanup after clearing abandoned output. Recursive dry-run also asserts that real shell work occurs while providers and lifecycle events remain suppressed.
- The production error guard caught two nested-runner error string conversions. Both now preserve typed lifecycle diagnostics through `from_error_or_action`; no guard exemption was added. The subsequent full run passed that guard.
- Refreshed the generated provider dispatch inventory using its existing update mode after the guard reported 1,853 current sites versus 1,851 recorded sites. The 12 inventory tests pass; no dispatch allowlist was expanded.
- Audited removed overlay names in both production source trees. Remaining matches are authored `sequence` grammar or test assertions; runtime output access uses the separate sequence binding. `git diff --check` passed at this checkpoint.


### Phase 5 source inventory

The following source and test files were created or updated during this phase. Existing unrelated working-tree edits are preserved and excluded from this phase inventory.

- `claudine/cli/src/commands/sequence.rs`
- `claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/retry_resume.rs`
- `claudine/cli/src/commands/wrap/sequence/container.rs`
- `claudine/cli/src/commands/wrap/sequence/iterate.rs`
- `claudine/cli/src/commands/wrap/sequence/jit/tests.rs`
- `claudine/cli/src/commands/wrap/sequence/mod.rs`
- `claudine/cli/src/commands/wrap/sequence/task_run.rs`
- `claudine/cli/tests/l1/agent_text_is_data.rs`
- `claudine/cli/tests/l1/codex_app_server.rs`
- `claudine/cli/tests/l1/composition_outputs.rs`
- `claudine/cli/tests/l1/document_environment_routes.rs`
- `claudine/cli/tests/l1/ctx_per_run.rs`
- `claudine/cli/tests/l1/handoff_owners.rs`
- `claudine/cli/tests/l1/inline_completion_lifecycle.rs`
- `claudine/cli/tests/l1/lifecycle_message_drain.rs`
- `claudine/cli/tests/l1/main.rs`
- `claudine/cli/tests/l1/sequence_cli.rs`
- `claudine/cli/tests/l1/sequence_containers.rs`
- `claudine/cli/tests/l1/sequence_errors_cli.rs`
- `claudine/cli/tests/l1/sequence_groups.rs`
- `claudine/cli/tests/l1/sequence_jit.rs`
- `claudine/cli/tests/l1/sequence_schema.rs`
- `claudine/lib/src/composition/error/mod.rs`
- `claudine/lib/src/composition/lifecycle/prewarm/sequence.rs`
- `claudine/lib/src/composition/lifecycle/runtime.rs`
- `claudine/lib/src/composition/lifecycle/tests/container_runtime.rs`
- `claudine/lib/src/composition/lifecycle/tests/mod.rs`
- `claudine/lib/src/composition/runtime_state.rs`
- `claudine/lib/src/composition/runtime_state/tests.rs`
- `claudine/lib/src/composition/sequence/data.rs`
- `claudine/lib/src/composition/sequence/preflight/mod.rs`
- `claudine/lib/src/composition/sequence/preflight/shape.rs`
- `claudine/lib/src/composition/sequence/preflight/tests.rs`
- `claudine/lib/src/composition/sequence/task/container_regressions.rs`
- `claudine/lib/src/composition/sequence/task/group.rs`
- `claudine/lib/src/composition/sequence/task/mod.rs`
- `claudine/lib/src/composition/sequence/task/sequence.rs`
- `claudine/lib/src/composition/sequence/task/tests.rs`


- The complete no-fail-fast area run exercised all 9,751 tests and found three remaining failures. The completion-verdict fixture now uses a direct prompt document (its earlier `prompt:` declaration accidentally required an inline file update); the message-drain fixture now uses explicit step documents and retains its two-delivery assertion. The proxy schema regression identified a real root-adapter omission: handoffs must retain the existing caller file-input records. Both root proxy paths now pass those records into canonical target preparation, preserving typed validation before target initialize.


### Platform scope and remaining phases

- Verification in this phase runs on macOS. The new runners use portable Rust synchronization and the existing request-scoped file resolution/canonicalization; no platform-specific process or terminal behavior was added. The `os` skill was consulted. Linux, native Windows, and WSL2 behavioral runs were not performed locally; their CI coverage remains required.
- Cancellation dispatch and Ctrl-C cleanup remain Phase 6. Shell discovery/approval-position completion remains Phase 7; live prompt migration remains Phase 8. These are planned boundaries, not accepted failures of Phase 5.
- Existing unrelated worktree changes remain intact. No formatting, staging, commits, focus-changing terminal/browser tests, or lifecycle-directory moves were performed.


### Final verification

- `just --justfile claudine/justfile test --no-fail-fast`: **9,751 passed**, 0 failed, 9 existing ignored tests, in 204.222 s. This is the area `just test` recipe with fail-fast disabled. Every new or changed test completed below 5 s. The 17 warning-threshold crossings were unchanged test cases:
  - `claudine::l1 claudine_schemas_embedded::embedded_json_trigger_combinator`
  - `claudine::l1 claudine_schemas_embedded::embedded_yaml_trigger_combinator`
  - `claudine-cli::l1 agent_text_is_data::malformed_structured_agent_values_are_refused_not_saved_as_text`
  - `claudine-cli::l1 compose_initialize_staged_boot::early_catch_shells_cannot_run_or_enable_another_catch_shell`
  - `claudine-cli::l1 compose_schema_cli::compose_and_inline_bare_sidecar_advisory_render_once_and_silent_suppresses_it`
  - `claudine-cli::l1 compose_schema_cli::compose_late_caller_file_verdict_names_the_launch_directory`
  - `claudine-cli::l1 compose_schema_cli::compose_unresolved_caller_file_fails_before_initialize_when_non_interactive`
  - `claudine-cli::l1 contamination_probes::an_uncontaminated_run_matches_the_expectations_every_probe_asserts`
  - `claudine-cli::l1 ctx_launch_anchor::loop_route_keeps_launch_area_for_root_and_package_documents`
  - `claudine-cli::l1 inline_completion_lifecycle::a_proxy_with_overlay_satisfies_the_targets_completion_schema`
  - `claudine-cli::l1 inline_completion_lifecycle::sequence_step_params_satisfy_the_referenced_documents_completion_schema`
  - `claudine-cli::l1 provider_tail_launch::embedded_credentials_reach_the_child_and_no_display_surface`
  - `claudine-cli::l1 provider_tail_launch::other_failures_are_not_attributed_to_the_tail`
  - `claudine-cli::l1 provider_tail_notice::a_switch_whose_record_is_typed_unknown_is_not_called_known`
  - `claudine-cli::l1 sequence_sources_cli::every_string_list_form_classifies_through_the_real_invocation_path`
  - `claudine-cli::l1 sequence_sources_cli::yaml_json_json5_jsonl_and_ndjson_sources_produce_the_same_plan`
  - `claudine-cli::l1 wrap_sequence_composition::sequence_composition_dry_run_for_every_provider`

The nine pre-existing ignored performance tests were `compose_ttff_perf::compose_emits_first_stderr_byte_within_budget`; `completion_perf::{perf_compose_empty_partial_meets_target, perf_compose_long_prefix_meets_target, perf_inline_compose_empty_partial_meets_target, perf_enter_compose_partial_meets_target}`; and `system_prompt_perf_bench::{bench_system_prompt_resolution_cold_and_warm, bench_resolve_and_prepare_step_by_step, bench_request_topology_probe_and_reuse, bench_raw_darkmatter_compose_passes}`. No test was newly ignored or moved to an unrun tier. Earlier failures described above were corrected; none remain in the final test gate.

The macOS linker reported the existing large `__eh_frame` compact-unwind warning; it did not fail compilation or tests.


- The first lint pass passed transport and lifecycle documentation guards, then reported two `collapsible_if` findings in the new group runner. Collapsed those conditions manually with no formatting command or lint exemption. The lint gate and focused group regressions are being rerun after that equivalent control-flow edit.


- Final `just --justfile claudine/justfile lint`: **passed** for `claudine-catalog-types`, `claudine`, `claudine-contract`, `claudine-cli`, and `claudine-gen`; transport guard 9/9 and lifecycle documentation facet guard passed. Besides the two collapsed conditions, removed one redundant reference in the CLI sequence loop. No lint exemptions or formatting command were used.


- The post-lint focused run exposed a synchronization gap in the new shared-file regression: the writer could truncate `next.yaml` while the reader was still doing its initial recursive preparation, producing a null-root source error. Added a bounded two-way Condvar handshake: the publisher waits until the reader reaches its first child, writes the complete source, then releases the reader. The original scenario and all downstream state/native-output assertions remain. No sleeps, retries, ignore, or production workaround were added.


### Settled-tree verification and closeout

- Focused post-lint regressions (`container_group_contract`, `container_regressions`, `sequence_containers`, `container_runtime`): **35/35 passed** in 2.089 s after fixing the deterministic handshake.
- Settled-tree `just --justfile claudine/justfile test --no-fail-fast`: **9,751/9,751 passed** in 200.119 s, with the same nine existing ignored performance tests. No new or changed test exceeded 5 s. These 14 unchanged cases emitted slow warnings:
  - `claudine::l1 claudine_schemas_embedded::embedded_json_trigger_combinator`
  - `claudine::l1 claudine_schemas_embedded::embedded_yaml_trigger_combinator`
  - `claudine-cli::l1 agent_text_is_data::malformed_structured_agent_values_are_refused_not_saved_as_text`
  - `claudine-cli::l1 compose_initialize_staged_boot::early_catch_shells_cannot_run_or_enable_another_catch_shell`
  - `claudine-cli::l1 compose_schema_cli::compose_late_caller_file_verdict_names_the_launch_directory`
  - `claudine-cli::l1 compose_schema_cli::compose_unresolved_caller_file_fails_before_initialize_when_non_interactive`
  - `claudine-cli::l1 inline_completion_lifecycle::sequence_step_params_satisfy_the_referenced_documents_completion_schema`
  - `claudine-cli::l1 provider_tail_launch::embedded_credentials_reach_the_child_and_no_display_surface`
  - `claudine-cli::l1 provider_tail_launch::other_failures_are_not_attributed_to_the_tail`
  - `claudine-cli::l1 provider_tail_notice::a_switch_whose_record_is_typed_unknown_is_not_called_known`
  - `claudine-cli::l1 sequence_sources_cli::every_string_list_form_classifies_through_the_real_invocation_path`
  - `claudine-cli::l1 sequence_sources_cli::yaml_json_json5_jsonl_and_ndjson_sources_produce_the_same_plan`
  - `claudine-cli::l1 tts_prewarm::every_accepted_playa_dry_run_spelling_prepares_nothing`
  - `claudine-cli::l1 wrap_sequence_composition::sequence_composition_dry_run_for_every_provider`

All observed development failures have been resolved. The final proxy schema regression asserts the typed diagnostic independent of its sentence capitalization, and still requires the target/property/partial path, actionable reason, no target initialize evaluation, and no provider launch.

- Settled-tree `just --justfile claudine/justfile lint`: **passed** again, including the nine transport guards, documentation facets, and Clippy for all five area packages. All Phase 5 tasks and both Checkpoint 5 items are checked.

- `just check-tier-coverage claudine`: **passed**, zero stranded tests. The area was listed with its declared library/CLI/contract/generator and rendezvous targets; no new test depends on a stubbed tier.
- `git diff --check`: passed. Both plan and log metadata were read back with `md get`; all requested keys parse, the source inventory contains 38 unique paths, and the cumulative package list is preserved. Updated documentation includes the current sequence/lifecycle topics, README, executable example, generated dispatch inventory, plan/log, and Claudine skill.

Implementation complete, ready for review. `human_review` is false. The replacement `message_to_agent` identifies the cancellation entry points for Phase 6 and the binding, source-reread, proxy, and caller-record invariants to preserve. No outstanding Phase 5 functionality or failing gate remains. No formatting, staging, commits, or feature-completion moves were performed.


## Phase 6

### Test mapping before implementation

- 6.1: public lifecycle driver tests will assert first-cause retention and exactly-once cancelled/finalize routing.
- 6.2: deterministic group regressions will coordinate failure with admitted opt-in (including empty stacks), non-opt-in, nested descendants, and unadmitted legs; assert terminal summaries, no publication, and cleanup order.
- 6.3: the normal CLI in a non-focusing L2 terminal will receive Ctrl-C while both types of leg run; assert exit 130, descendant settlement, innermost-first cleanup, and no subsequent step.
- 6.4: driver and nested execution regressions will assert recovery never launches work and cleanup errors stay secondary to cancellation, including cleanup timeout.
- No new file-format fields or parser changes are planned; the configuration robustness matrix does not apply. Existing shipped-artifact corpus coverage remains applicable.

Implementation uses the existing portable process-tree and signal machinery. No formatting, staging, commits, or focus-taking tests are authorized.

### Cancellation driver and concurrent settlement

- Added a host-neutral cancellation cause/token and a first-cause `ContainerRunner::cancel` transition. Cancellation owns the terminal result, refuses retry/resume/proxy/defer recovery, and retains cleanup failures separately. Sibling diagnostics carry the originating group/leg and original diagnostic.
- Group admission now stops on atomic failure, cancels only admitted opt-in legs (including empty handlers), and settles all workers before group terminal events. Parent cancellation crosses worker boundaries and reaches all nested descendants. Non-opted siblings complete; unadmitted legs run no events.
- Nested sequence and group cleanup now runs before restoring lexical scopes. Root Ctrl-C cleans up and returns 130; prompt/provider cancellation and lifecycle-shell cancellation are being checked at all preparation boundaries.
- `container_runtime`: 6 tests passed after correcting an emitter assertion to use actual authored emissions. `container_regressions`: 12 passed, including coordinated sibling cancellation and interrupted cleanup/retry refusal; all new cases took under 0.2 s.
- The new non-focusing L2 criterion-7 test passed in 1.061 s. It checks both types of running leg, dead child PIDs, no later step, exact-once innermost-first cleanup, and exit 130. Native Windows explicitly reports its Unix-harness capability skip per Ruling 5.
- The first broad run stopped on the new fixture's omitted step names; the fixture was corrected. The next broad run reached 6,439 tests and exposed a sequence drain regression: keeping the root interrupt guard in shutdown changed its drain notice. The root guard now has invocation lifetime, preserving the existing drain-specific handler and notice. This remains scheduled for rerun.
- Review identified two additional paths: lifecycle shells used blocking waits, and pre-provider prompt cancellation could enter failure recovery. Lifecycle shells now reuse task shells' portable owned-tree execution; prompt preparation routing is being corrected with focused CLI coverage.

### Prompt setup and terminal grace

- Lifecycle exact and structured shell actions now share task shells' owned process trees and cooperative cancellation, with inherited streams preserved. No new platform-specific tree implementation was introduced.
- Prompt preparation checks cancellation around initialize/start and before provider execution. Cancellation-aware error routing also handles materialization, bootstrap, plan, and launch errors; cleanup uses the entered guard so pre-start shells remain disabled.
- Consolidated prompt cleanup bypasses failure recovery, preserves exit 130 even when cleanup fails, and reports typed recovery refusals through the existing terminal emitter.
- `wrap_sigint` targeted suite: **15/15 passed**, total 3.102 s, slowest 3.095 s. Added `compose_cancelled_start_runs_cancelled_finalize_without_failure_retry`: the original blocked-start input receives SIGINT, exposes `provider.interrupted`, runs its cleanup shell/finalize once, never runs failure/retry, and never launches a provider.
- Migrated interrupted orphan-teardown expectations to cancelled/finalize while retaining ordinary failure cases. All terminal events preserve the existing repeat-interrupt grace; a first interrupt does not kill an already-running terminal action. Fixed the success/blocked adapter bypass so those events use the same guarded execution.
- Expanded driver regressions cover retry/resume/proxy/defer refusal, inherited first-cause diagnostics, scope restoration, and cleanup timeout. **42 focused container tests passed** in 1.021 s, including schema, corpus, and nested execution tests.
- Current sequence/lifecycle docs and the Claudine skill now describe implemented cancellation, with a Mermaid flow and explicit OS exit table. No parser/configuration fields were added.

### Gate progress

- Area L1 passed **9,759/9,759** in 152.577 s with nine pre-existing ignored performance tests. The sequence drain regression is resolved. The only slow warnings were unchanged embedded schema combinator tests.
- Added a final CLI regression for sibling failure cancelling a running provider, with the normal `sequence` invocation, source prompt cleanup, structured group/leg cause, refused finalize retry, stopped provider PID, no completion, and no later step. A settled-tree L1 run includes this new test.
- Preserved interrupted child stdout while keeping native publication withheld. Group summaries now derive an interrupt diagnostic even for injected interruption flags without a process-global signal.

### Cross-platform checks

- `just cross-check claudine --os linux --features test-fixtures container_`: **42/42 passed** in 0.632 s. This is filtered native Linux behavioral evidence, not a CI receipt.
- Native Windows compiled the CLI and library successfully. The first `sequence_containers` run passed 12/14 and exposed two Phase 5 fixtures installing only a POSIX Goose script. Added the corresponding `goose.cmd` fixtures in the same touched test file, including the dry-run stub for consistency; the targeted Windows suite is rerunning.
- Initial cross-check attempts with a nextest expression hit the just recipe's shell interpolation before execution; reran using ordinary positional test filters. No test was skipped to mask a failure.

### Requirement-to-test mapping

| Requirement | Targeted tests |
| --- | --- |
| 6.1: first structured cause and exactly-once cleanup | `cancellation_keeps_first_cause_and_refuses_finalize_recovery`; `cancellation_scope_inherits_first_cause_and_restores_its_caller` |
| 6.2: opted-in siblings, all descendants, non-opted siblings, no new admission | `sibling_failure_cancels_opted_container_descendants_and_preserves_other_legs`; `sibling_failure_terminates_an_opted_prompt_and_refuses_its_finalize_retry`; existing `markdown_and_sequence_yaml_fields_matrix` retains empty-vs-absent grammar coverage |
| 6.3: Ctrl-C, pre-start restrictions, cleanup order, exit 130 and repeat force-exit | `level2_ctrl_c_cancels_group_descendants_and_finalizes_innermost_first`; `compose_cancelled_start_runs_cancelled_finalize_without_failure_retry`; `cancellation_before_start_runs_cleanup_with_shells_disabled`; all 15 `wrap_sigint` cases |
| 6.4: terminal cancellation and secondary diagnostics | `cancellation_refuses_every_recovery_and_retains_cleanup_timeout`; `interrupted_sequence_keeps_cleanup_errors_secondary_and_never_retries`; `cleanup_shell_timeout_stays_secondary_to_cancellation`; both new CLI cancellation regressions |
| 6.5: criteria 5, 7 and 19, plus timeout distinction | Tests above and existing `timed_out_nested_leg_uses_failure_retry_and_never_cancellation` |

The new L1 tests compile in the existing library unit target and the declared CLI `l1` target. The L2 file is declared by `cli/tests/level2/main.rs` and is selected by the live `test-l2` recipe. No new feature gate, ignore, timeout extension, focus-taking operation, parser, persisted value, or shipped prompt was introduced. Existing passive corpus and real shipped-example invocation tests remain in the full L1 gate.

### Phase 6 source inventory

- `claudine/cli/src/commands/compose/prep.rs`
- `claudine/cli/src/commands/sequence.rs`
- `claudine/cli/src/commands/wrap/composition/pipeline.rs`
- `claudine/cli/src/commands/wrap/composition/pipeline/tests.rs`
- `claudine/cli/src/commands/wrap/composition/staged_boot.rs`
- `claudine/cli/src/commands/wrap/exec/termination/unix.rs`
- `claudine/cli/src/commands/wrap/exec/termination/windows.rs`
- `claudine/cli/src/commands/wrap/harness_orch/loop_control.rs`
- `claudine/cli/src/commands/wrap/harness_orch/loop_control/error_routing.rs`
- `claudine/cli/src/commands/wrap/harness_orch/loop_control/lifecycle_events.rs`
- `claudine/cli/src/commands/wrap/sequence/container.rs`
- `claudine/cli/src/commands/wrap/sequence/mod.rs`
- `claudine/cli/tests/common/review_screen.rs`
- `claudine/cli/tests/common/terminal_interrupt.rs`
- `claudine/cli/tests/l1/sequence_containers.rs`
- `claudine/cli/tests/l1/wrap_sigint.rs`
- `claudine/cli/tests/level2/level2_auto_complete_operation_file.rs`
- `claudine/cli/tests/level2/level2_lifecycle_control.rs`
- `claudine/cli/tests/level2/level2_perf_capture.rs`
- `claudine/cli/tests/level2/level2_sequence_container_ctrl_c.rs`
- `claudine/cli/tests/level2/level2_sequence_task_stream_capture.rs`
- `claudine/cli/tests/level2/main.rs`
- `claudine/lib/src/composition/error/mod.rs`
- `claudine/lib/src/composition/error/render/mod.rs`
- `claudine/lib/src/composition/lifecycle/cancellation.rs`
- `claudine/lib/src/composition/lifecycle/executor.rs`
- `claudine/lib/src/composition/lifecycle/mod.rs`
- `claudine/lib/src/composition/lifecycle/runtime.rs`
- `claudine/lib/src/composition/lifecycle/structured_shell.rs`
- `claudine/lib/src/composition/lifecycle/tests/container_runtime.rs`
- `claudine/lib/src/composition/looping/engine.rs`
- `claudine/lib/src/composition/looping/engine/tests/lifecycle_control.rs`
- `claudine/lib/src/composition/sequence/task/container_regressions.rs`
- `claudine/lib/src/composition/sequence/task/group.rs`
- `claudine/lib/src/composition/sequence/task/mod.rs`
- `claudine/lib/src/composition/sequence/task/sequence.rs`
- `claudine/lib/src/composition/sequence/task/shell.rs`
- `claudine/lib/src/diagnostics/registry.rs`

### Final gate findings and repairs

- The final normal CLI container suite passed **15/15** in 1.813 s, including the running-provider cancellation regression and its assertion that sibling failure is not reported as a user interrupt.
- Native Windows CLI container suite passed **14/14** in 4.803 s after installing the existing compiled provider relay. A `.cmd` attempt hit Windows' already documented multiline-argument restriction; the final fixture uses `.exe`, without an interpreter wrapper around the prompt argument. Native Windows library container coverage also passed **42/42** in 3.167 s.
- Added the catalog code `composition.cancelled` for sibling cancellation, with declared `group`, `leg`, and `cause` detail, unrecoverable disposition and warning severity. The cause regression asserts both the projected values and exact catalog key set. Ctrl-C keeps `provider.interrupted`. This avoids attaching undeclared fields to the generic composition-failure payload.
- Lint passed all five area packages and diagnostic/documentation guards before the final catalog/fixture updates; its settled-tree rerun is queued after tests.
- Full L2 first found three stale `state.name` YAML fixtures. The complete diagnostic run then exercised **287 tests: 274 passed, 13 failed**. The new cancellation test passed in 1.337 s. The remaining failures were older fixture/helper drift: six lifecycle tests accepted an echoed, terminal-wrapped exit marker before execution; two tests still attached per-step proxy behavior to the root sequence; four review-screen tests used retired group grammar; one proxy/merge stream test needed its pre-proxy mutation at the enclosing leg-container scope.
- Corrected the helper marker to use split printf arguments and require actual completion. Migrated per-step proxy fixtures to explicit prompt documents, retained all target-count/loop assertions, migrated implicit-concurrency/serial-shell stream fixtures, and replaced retired task setup with leg-container initialize handlers. The tolerant-group expectation now matches the current success contract while retaining failed-leg attribution and downstream merge assertions.
- Review-screen targeted verification passed **4/4**, preserving catalog/free-text model selection, Escape, Ctrl-C, and no-work cancellation assertions. No test was dropped, ignored, moved to another tier, or given a longer deadline.
- A settled-tree full L2 → L1 → lint → tier audit sequence is running after these repairs.

### L2 completion

- Settled-tree `just --justfile claudine/justfile test-l2 --no-fail-fast`: **287/287 CLI tests passed**, plus **3/3 generator L2 tests passed**. All earlier L2 failures are resolved. The new criterion-7 test passed in 1.463 s, including terminal exit 130 and refused root-finalize retry; the proxy/merge stream regression passed in 3.338 s.
- Non-focusing tmux and background terminal harnesses were used; no keyboard-focus test or L3 recipe ran. The canonical tier filter excluded other tiers; no selected macOS L2 test was skipped. Native Windows' criterion-7 function retains its explicit Unix-backend skip under Ruling 5.
- All five Phase 6 implementation/test tasks are now checked. Final L1, lint, and tier audit remain running sequentially before closeout.

### Staged initialize cancellation audit

- Added the cancellation checkpoint to the shared initialize router and staged-read adapters before skip/proxy/error recovery is committed. Cancelled/finalize cleanup remains shell-disabled before start, and terminal exit remains 130 even when cleanup fails.
- Three targeted regressions passed: `cancelled_initialize_skip_keeps_exit_130_and_refuses_cleanup_shell`, `cancelled_initialize_proxy_never_commits_handoff_or_cleanup_recovery`, and `cancelled_initialize_error_never_dispatches_failure_catch`. They assert event order, no handoff/start/failure catch, visible secondary diagnostics, and no cleanup shell side effect.
- The first compilation caught three fixture type/import mismatches; those are corrected and the targeted run passed 3/3 in 0.059 s. Full settled-code gates are rerunning.

### Loop cancellation audit

- Inspection found that the existing prompt-loop coordinator could run its gate or another iteration after an executor reported cancellation, and initialization could route skip/proxy/error before observing cancellation. Moved the shared cancellation cleanup into `LifecycleRunGuard::cancel_if_requested`; both staged single prompts and loops now use it. Staged loop reread failure also checks cancellation before entering blocked catch.
- Added five declared L1 regressions: `cancelled_loop_initialize_skip_has_priority_over_routing`, `cancelled_loop_initialize_proxy_has_priority_over_routing`, `cancelled_loop_initialize_error_has_priority_over_routing`, `cancelled_loop_executor_stops_without_gate_or_mutation`, and `cancelled_loop_executor_error_stops_even_when_fail_fast_is_false`. All five passed in 0.049 s. They preserve partial output and structured sibling cause, refuse cleanup recovery/shells before start, and prove no handoff, gate, mutation, or second iteration even with `fail_fast: false`.
- Added normal-invocation `compose_cancelled_loop_start_never_runs_another_iteration`, using the same blocked start/SIGINT input as the single-prompt regression with `loop: { while: 'true', fail_fast: false }` and two permitted iterations. This extends requirements 6.3/6.4 to looping prompts.
- A full L1 run after staged-initialize changes stopped at the expected generated dispatch-inventory drift: 6,032 passed and one inventory mismatch, with 3,730 not run because of fail-fast. The prescribed inventory regeneration is included before the final full rerun; no behavioral test failed in that run.

### Final L1 result and existing exclusions

- Final area `just test`: **9,769/9,769 passed** in 204.093 s. The new CLI SIGINT tests passed in 1.605/1.687 s, the running-provider sibling-cancellation regression in 1.549 s, and the three staged-initialize tests in 0.045–0.065 s. No new or changed cancellation test exceeded the L1 five-second budget.
- Nine pre-existing opt-in performance tests were excluded by their existing ignore attributes: `compose_emits_first_stderr_byte_within_budget`; `bench_system_prompt_resolution_cold_and_warm`; `bench_resolve_and_prepare_step_by_step`; `bench_request_topology_probe_and_reuse`; `bench_raw_darkmatter_compose_passes`; `perf_compose_empty_partial_meets_target`; `perf_compose_long_prefix_meets_target`; `perf_inline_compose_empty_partial_meets_target`; `perf_enter_compose_partial_meets_target`. None was added or changed in this phase.
- The run reported 14 existing slow tests in unchanged test files (all passed; no timeout/ignore/tier change was used):

- `claudine::l1 claudine_schemas_embedded::embedded_json_trigger_combinator`
- `claudine::l1 claudine_schemas_embedded::embedded_yaml_trigger_combinator`
- `claudine-cli::l1 agent_text_is_data::malformed_structured_agent_values_are_refused_not_saved_as_text`
- `claudine-cli::l1 compose_initialize_staged_boot::early_catch_shells_cannot_run_or_enable_another_catch_shell`
- `claudine-cli::l1 compose_schema_cli::compose_late_caller_file_verdict_names_the_launch_directory`
- `claudine-cli::l1 compose_schema_cli::compose_unresolved_caller_file_fails_before_initialize_when_non_interactive`
- `claudine-cli::l1 inline_completion_lifecycle::sequence_step_params_satisfy_the_referenced_documents_completion_schema`
- `claudine-cli::l1 provider_tail_launch::embedded_credentials_reach_the_child_and_no_display_surface`
- `claudine-cli::l1 provider_tail_launch::other_failures_are_not_attributed_to_the_tail`
- `claudine-cli::l1 provider_tail_notice::a_switch_whose_record_is_typed_unknown_is_not_called_known`
- `claudine-cli::l1 sequence_sources_cli::every_string_list_form_classifies_through_the_real_invocation_path`
- `claudine-cli::l1 sequence_sources_cli::yaml_json_json5_jsonl_and_ndjson_sources_produce_the_same_plan`
- `claudine-cli::l1 tts_prewarm::every_accepted_playa_dry_run_spelling_prepares_nothing`
- `claudine-cli::l1 wrap_sequence_composition::sequence_composition_dry_run_for_every_provider`

### Final L2 capture boundary repair

- Final full L2 ran all 287 CLI cases: 286 passed, including the new cancellation test in 1.427 s; one existing WezTerm perf capture failed because the dominant `HOT` leaf scrolled above its viewport under host load. This is a capture-boundary failure, not a missing production marker: the captured report began mid-tree.
- The WezTerm perf test now captures the complete report from scrollback after completion. Its command clears shared-pane history as well as the viewport, preserving exactly-one-marker and no-stale-duration assertions. No timing threshold, retry, styling assertion, backend gate, or production rendering behavior changed. The existing three perf-capture cases provide targeted regression coverage.

### Final L2 result

- The corrected perf-capture target passed **3/3** in 2.008 s. Full area `just test-l2 --no-fail-fast` then passed **287/287 CLI cases** in 74.635 s and **3/3 generator cases** in 0.830 s. The new Ctrl-C container test passed in 1.361 s. All selected macOS L2 tests passed; canonical tier filters exclude other tiers, and no capability skip was reported on this host.
- The earlier perf viewport failure is resolved without weakening assertions. The complete WezTerm styling case also passed under full-suite load (5.843 s; L2 resource tier).
- Native Windows evidence remains the earlier targeted container suites (14 CLI and 42 library cases), and Linux evidence remains 42 targeted library cases. Those preceded the host-neutral initialize/loop audit refinements; they are targeted behavioral evidence, not final-tree CI receipts. WSL2 was not run. Native Windows console Ctrl-C is not claimed: the Unix-terminal criterion-7 test has the explicit capability skip required by Ruling 5.

### Final lint result

- Final area `just lint` passed all five packages: `claudine-catalog-types`, `claudine`, `claudine-contract`, `claudine-cli`, and `claudine-gen`. All nine diagnostic transport/catalog guards passed in 2.205 s, and the lifecycle documentation facet guard passed.
- No dependency, parser, configuration-field, persisted-format, or live prompt changes were added. No robustness-matrix or persistence-round-trip additions were required. Existing passive corpus, schema, and shipped-artifact invocation tests passed in the full L1 run.

### Phase 6 closeout

- `just check-tier-coverage claudine` passed: the area was listed and **zero stranded tests** were found. Added tests compile through the existing declared targets, and the new L2 file is declared in its tier binary. The browser recipe remains an existing stub with no stranded cases.
- All tasks 6.1–6.5 are checked. Required gates are green: **9,769 L1**, **290 L2**, area lint, diagnostic/documentation guards, and tier coverage. All earlier reported test failures were repaired; only the explicitly recorded pre-existing performance exclusions, slow warnings, linker warning, and platform coverage limits remain.
- Both plan and log inventories list all **38 source/test files**, **seven updated documentation files**, **no created documentation files**, and the updated Claudine skill. Cumulative `packages` retains prior phases and includes the two packages changed here (`claudine`, `claudine-cli`). Requested metadata was read back successfully with `md get`; no duplicate frontmatter key was introduced.
- Implementation complete, ready for review. `human_review` remains false; `message_to_agent` contains the replacement Phase 6 handoff and Phase 7 boundaries. No formatting, staging, branch commits, or lifecycle-directory moves were performed. Final diff whitespace and unchanged-index checks passed.

## Phase 7

### Requirement-to-test map before implementation

- 7.1 / criteria 1, 18: retain recursive graph and collision tests; add container lifecycle shell discovery (short/long shell, conditional branches, shell-valued set), malformed lifecycle rejection, and nested cross-leg collision coverage.
- 7.2 / criterion 18: add a mixed sequence/task/catalog cycle with its complete chain, distinct catalog entries within one active catalog, and repeated sequence reuse. Existing origin-relative FileReference tests remain applicable.
- 7.3 / criterion 21: add all-first ancestry and all-leg approval assertions, terminal-event and group-read template assertions, illegal err/timing/current reads, and CLI filesystem assertions for event-time values.
- 7.4: retain existing nested reread/proxy refusal tests and extend refusal to newly introduced container event shells. L1 hermetic CLI subprocess tests cover the invocation/filesystem boundary; no terminal interaction is introduced. Existing grammar robustness matrices cover unchanged input loading; this phase changes discovery and approval policy, not file-format parsing.

Earlier-phase changes are already present in the worktree and will be preserved. The plan names CLI preflight paths; the implementation lives in `lib/src/composition/sequence/preflight/`.

### Discovery and cycle implementation

- The first targeted run reproduced both regressions: distinct catalog entries were rejected as a file cycle, and a group finalize command was absent from discovery (two candidates instead of three). After implementation both passed, followed by all seven `preflight::tests::phase7` tests (0.043–0.051 s each).
- Added `preflight/containers.rs` to discover and prepare parsed container actions without evaluating lifecycle conditions. Terminal placeholders require templates; group event roots stay late. Shell-value commands retain normalized approval keys. Root guards receive the audited lifecycle; fresh nested lifecycles settle against the original authorization.
- Reference ancestry now includes a catalog entry selector. Mixed cycles report the root, sequence, task, catalog entry, and repeated sequence; two distinct active entries and repeated independent sequence references are legal. Existing request-scoped FileReference resolution remains unchanged.
- The recursive graph, retained source snapshots, authoring origins, and nested write-back collision checks are preserved. Tasks 7.1 and 7.2 are checked.
- CLI container coverage passed 17/17, including the existing shipped-example corpus and normal invocation. An initial new CLI fixture incorrectly used the array `group.succeeded` as one shell argument; that correctly failed. The success fixture now uses `first(group.succeeded)`, and a separate negative regression retains the original input and asserts failed publication.
- Root proxies were found to create fresh approval coordinators. A closed approval membership now travels through root/step proxy scope and is enforced before policy or prompts; `--yolo` cannot authorize newly introduced commands. This is a Phase 7 repair to the prior implementation, not a grammar change.

### Targeted tests and broad verification

New declared L1 tests (all compiled and selected by the full area recipe):

| Requirement | Exact tests added |
| --- | --- |
| 7.1 transitive discovery and collisions | `preflight::tests::phase7::container_shell_discovery_walks_both_branches_and_shell_values`; `preflight::tests::phase7::nested_inline_write_back_collision_is_rejected` |
| 7.2 active reference identity and full chain | `preflight::tests::phase7::distinct_groups_in_one_active_catalog_are_legal`; `preflight::tests::phase7::mixed_reference_cycle_names_every_active_reference`; `preflight::tests::phase7::sequence_reuse_is_legal_and_only_all_first_descendants_offer_bytes` |
| 7.3 position, late roots, illegal roots | `preflight::tests::phase7::container_events_are_discovered_and_terminal_commands_require_templates`; `preflight::tests::phase7::container_shells_refuse_unavailable_roots_even_beside_late_values`; `sequence_containers::container_shells_use_event_time_values_and_lexical_group_outcomes`; `sequence_containers::container_outcome_collection_cannot_be_used_as_one_shell_argument`; `sequence_containers::nested_event_command_is_rejected_before_any_child_effect` |
| 7.3/7.4 closed approval | `task::tests::group_preparation_contract::new_container_event_command_is_refused`; `task::tests::group_preparation_contract::new_container_event_template_is_refused`; `sequence_containers::root_proxy_refuses_a_new_shell_after_approval_closes_even_with_yolo` |

Existing complementary coverage retained: `a_first_group_qualifies_with_every_leg_and_group_readers_are_templates`, nested sequence position tests, source-origin resolution tests, changed nested source refusal, proxy cycle/refusal tests, and persisted scripted approval choice round trips. No new persisted format or parser field was added, so the input robustness matrix is unchanged. The existing passive `shipped_container_example_corpus_has_parseable_lifecycles` and end-to-end `shipped_container_example_runs_through_the_normal_sequence_command` now cover an actual shipped terminal-event shell template (`completed-3`).

- Full `just --justfile claudine/justfile test`: **9,782/9,782 passed** in **111.853 s**, across all five area packages. New preflight/refusal unit tests took 0.020–0.023 s; the four new CLI regressions took 0.322–0.839 s. No new/changed L1 test exceeded five seconds.
- Nine pre-existing ignored performance tests remain excluded: `compose_emits_first_stderr_byte_within_budget`; `bench_system_prompt_resolution_cold_and_warm`; `bench_resolve_and_prepare_step_by_step`; `bench_request_topology_probe_and_reuse`; `bench_raw_darkmatter_compose_passes`; `perf_compose_empty_partial_meets_target`; `perf_compose_long_prefix_meets_target`; `perf_inline_compose_empty_partial_meets_target`; `perf_enter_compose_partial_meets_target`. No test was dropped, retried, ignored, or moved to another tier in this phase. No unresolved test failure was observed.
- Targeted preflight run passed 7/7; group-preparation run passed 10/10; final CLI container run passed 19/19. The root-proxy regression first reproduced an unexpected success, then passed after carrying closed membership into the wrapper's rebuilt shell options as well as the coordinator's audit.
- Dispatch inventory was refreshed through its prescribed `CLAUDINE_UPDATE_INVENTORY=1 just test-cli dispatch_inventory::` route; all 12 inventory tests passed. Existing linker warnings about the large macOS unwind section appeared during builds; they did not fail tests.
- `git diff --check` passed. No formatter, staging operation, branch commit, feature lifecycle move, or focusing terminal/browser harness was used.
- Supplemental Linux/library and native-Windows/CLI checks are running through `just cross-check`. The first invocation failed before execution because `just` expands its variadic recipe arguments through a shell; preserving quotes around the nextest expression corrected it. The current runs use identical source trees and focused nextest selections. Lint and tier audit results follow below.

### Final audit repairs and platform findings

- Added `sequence_containers::root_retry_keeps_approval_closed_when_a_referenced_task_changes`. The original input rewrites a referenced task with `set_frontmatter`, then fails and retries the root. It failed before the fix because root graph approval reopened; it now passes because root retries pass the original membership into graph approval, including the empty-graph fast path. The test observes the rewritten source, refusal, and absence of the unapproved shell's file. The settled CLI container selection passes **20/20**.
- The passive shipped-example test now builds each example's public sequence plan and preflight graph in addition to walking every parsed container lifecycle. The shipped example's existing normal-invocation test remains in place.
- Native Windows ran 18 container tests and found two failures in newly authored `echo` template inputs. This was fixture drift from the existing direct-template contract: Windows `echo` is a `cmd.exe` builtin; template dispatch correctly requires a native executable. The successful event-time test now uses the compiled `claudine-argv-recorder`, checks all five exact argument vectors and downstream group output, and stays portable. The shipped example's added `echo` template was removed, leaving its original portable output behavior; it is therefore absent from this phase's changed-document inventory. The earlier log's `completed-3` statement describes the abandoned example edit, not the final implementation.
- Added this OS-specific lesson to `.claude/skills/os/windows.md`, as required by the repository's OS instructions. User docs now use a native Git example and explicitly name the native-executable requirement. No interpreter fallback was added.
- Supplemental Linux archive run passed **7/7** targeted preflight tests in **0.057 s**. The first Linux/Windows expression-filter attempts compiled successfully but failed in the shared runner's shell argument expansion before tests; positional test-name filters avoid that defect. The Windows fixture failures above are being rechecked with the final native fixture and root-retry regression.
- `just lint` passed all five area packages and both documentation/transport guard recipes. `just check-tier-coverage claudine` passed with **zero stranded tests**. The 13 initial new tests plus the root-retry regression are in existing declared L1 targets; no new target, feature, or CI cell was introduced.
- The full rerun after strengthening the corpus passed **9,782/9,782** in **286.194 s** with the same nine existing ignored tests. It reported 46 slow tests in unchanged code, versus no slow report in the earlier 111.853 s run. None of the new/changed container tests exceeded five seconds. The settled-source full L1 and lint reruns follow below, after the root-retry repair and portable fixture correction.
- Comment drift repaired in touched preflight symbols: `outputs` became `seq.outputs`; first position now explicitly includes all first-group legs; the outdated count of executable shapes was removed; ancestry documentation now includes catalog entry identity. Plan/log frontmatter properties were read back with `md get` after every edit; cumulative package membership was preserved. No spec status or lifecycle location was changed.

### Settled-source results

- Final area `just test`: **9,783/9,783 passed** in **241.301 s**, with the same nine pre-existing ignored performance tests. Every Phase 7 test passed, including the native-recorder lifecycle test, passive shipped corpus, and root-retry regression. No changed container test exceeded five seconds.
- Native Windows final archive run: **19/19 passed** in **4.850 s** (individual tests 1.181–3.548 s); Linux targeted archive run: **7/7 passed**. Native Windows excludes the existing Unix-only running-provider cancellation test at compilation; no selected test was skipped. These are local source-snapshot results, not reusable branch-head CI receipts: cross-check correctly declined to publish a Windows observation because the uncommitted snapshot differs from branch HEAD. WSL2 was not exercised in this phase and relies on the repository's scheduled CI; no WSL-specific implementation changed.
- Final L1 reported 26 slow tests in unchanged functions. Their passing results do not erase the repository's five-second performance debt. No timeout, retry, ignore, tier move, or deadline increase was used. Nextest counted 26 slow tests; 27 completed durations exceeded five seconds (including one threshold-boundary result). All 27 are listed below.

| Existing test | Seconds |
| --- | ---: |
| `claudine::l1 claudine_schemas_embedded::embedded_json_trigger_combinator` | 6.757 |
| `claudine::l1 claudine_schemas_embedded::embedded_yaml_trigger_combinator` | 6.590 |
| `claudine-cli::l1 agent_text_is_data::inline_agent_values_are_stored_as_data_and_read_back_exactly` | 6.433 |
| `claudine-cli::l1 authored_text_rendering::header_document_label_and_model_render_exactly_as_authored` | 5.850 |
| `claudine-cli::l1 agent_text_is_data::malformed_structured_agent_values_are_refused_not_saved_as_text` | 9.666 |
| `claudine-cli::l1 compose_initialize_staged_boot::early_catch_shells_cannot_run_or_enable_another_catch_shell` | 7.008 |
| `claudine-cli::l1 compose_schema_cli::compose_and_inline_bare_sidecar_advisory_render_once_and_silent_suppresses_it` | 5.179 |
| `claudine-cli::l1 compose_schema_cli::compose_late_caller_file_verdict_names_the_launch_directory` | 6.436 |
| `claudine-cli::l1 compose_schema_cli::compose_unresolved_caller_file_fails_before_initialize_when_non_interactive` | 8.322 |
| `claudine-cli::l1 ctx_launch_anchor::caller_file_anchor_survives_direct_and_proxy_success_guards` | 5.160 |
| `claudine-cli::l1 ctx_launch_anchor::loop_route_keeps_launch_area_for_root_and_package_documents` | 6.118 |
| `claudine-cli::l1 document_environment_routes::re_entering_a_document_reads_its_map_without_capturing_again` | 5.277 |
| `claudine-cli::l1 entry_point_parity::claudine_entry_points_agree_on_every_reference_shard_1` | 5.002 |
| `claudine-cli::l1 entry_point_parity::claudine_entry_points_agree_on_every_reference` | 5.036 |
| `claudine-cli::l1 inline_completion_lifecycle::a_proxy_with_overlay_satisfies_the_targets_completion_schema` | 5.170 |
| `claudine-cli::l1 inline_completion_lifecycle::sequence_step_params_satisfy_the_referenced_documents_completion_schema` | 5.730 |
| `claudine-cli::l1 provider_tail_launch::embedded_credentials_reach_the_child_and_no_display_surface` | 7.367 |
| `claudine-cli::l1 provider_tail_launch::other_failures_are_not_attributed_to_the_tail` | 6.909 |
| `claudine-cli::l1 provider_tail_notice::a_switch_whose_record_is_typed_unknown_is_not_called_known` | 8.582 |
| `claudine-cli::l1 sequence_sources_cli::the_three_operators_each_produce_a_usable_name` | 5.297 |
| `claudine-cli::l1 sequence_sources_cli::every_string_list_form_classifies_through_the_real_invocation_path` | 8.244 |
| `claudine-cli::l1 shell_missing_values::a_missing_value_fails_pre_flight_on_every_shell_surface` | 5.013 |
| `claudine-cli::l1 sequence_sources_cli::yaml_json_json5_jsonl_and_ndjson_sources_produce_the_same_plan` | 8.178 |
| `claudine-cli::l1 tts_prewarm::every_accepted_playa_dry_run_spelling_prepares_nothing` | 5.981 |
| `claudine-cli::l1 wrap_compose_validation::predicates_operands_and_proxy_with_values_are_rejected_before_launch` | 5.084 |
| `claudine-cli::l1 wrap_sequence_composition::sequence_composition_dry_run_for_every_provider` | 8.444 |
| `claudine-cli::l1 wrap_watchdog_startup_stall::watchdog_startup_bytes_move_the_deadline` | 5.035 |

### Phase 7 closeout

- Settled-source `just lint` passed all five area packages, transport guards, and lifecycle documentation guards. The tier audit passed with zero stranded tests, and the final full L1 selected and passed the added root-retry test in the existing declared target.
- All Phase 7 tasks (7.1–7.4) are checked. The requirement-to-test mapping, initial failures, fixes, skipped tests, existing slow tests, platform results, source inventory, and handoff are recorded above and in frontmatter. No unresolved Phase 7 implementation or test failure remains.
- All requested plan/log frontmatter keys parse and retain a single occurrence. The cumulative `packages` list is unchanged because `claudine` and `claudine-cli` were already present. `human_review` is false. The spec's status is left for its author.
- Implementation complete, ready for review. Phase 8 owns the remaining migration sweep. No formatting, staging, branch commit, or feature lifecycle move was performed.


## Phase 8

### Requirement-to-test map before edits

- 8.1 migration audit: repository-wide searches include hidden skills, current docs, shipped artifacts, fixtures, Rust tests, and `fm.outputs`/`fm.state`; classify behavioral reads rather than renaming identifiers mechanically.
- 8.2 shipped prompts: opt-in `shipped_prompt_contract`, `shipped_prompt_route_drift`, and `shipped_prompts` cover passive parsing, lifecycle walking, fixture drift, and normal hermetic CLI invocation. Existing L1 `shipped_lifecycle_artifacts_use_mapping_only_set` and the container example corpus/normal invocation cover shipped non-prompt artifacts. Prompt-specific additions stay in the declared opt-in target.
- 8.3/8.4 documentation and skills: compare guidance with the parser, binding catalog, recursive runtime, cancellation, and closed shell approval implementation; run lifecycle-doc lint, all area L1 tests, lint, and tier coverage sequentially. No new parser, persisted representation, or production behavior is planned, so the file-format input matrix and persistence round trips are unchanged.

Parallel scopes follow the plan: prompts/fixtures/tests, seven primary topic pages, and skill files. The primary agent owns the repository classification, supplementary topic drift, final gates, and metadata. No terminal/browser resource is needed for prose migration.

### Repository-wide classification (8.1)

Searches covered hidden skill files and the whole checkout, including research workflows outside the internal prompt folders. Broad token searches were narrowed by expression spans, block predicates, YAML keys, and command usage, then the surrounding documents and implementations were read. `fm.state`/`fm.outputs` have no remaining active runtime-global use; explicit authored-property documentation is retained.

| Hits | Classification and action |
| --- | --- |
| Internal `prompts/review-loop.md`, `_prompt.md`, `_reviews/feature-review.md`, `_implement/implement-plan.md` | Migrate removed root failure policy, run-token guidance, unbound `next` input, and stale loop boundary read. Preserve explicitly authored report `previous`/`next` fields. Re-derive implement route fixture and hash when its prompt changes. |
| Research fleets, summary prompt block predicates, research template and model-serving prompt under `claudine/docs/research/` | Migrate runtime row reads to `seq`; root lifecycle hooks need per-row prompt ownership, not just textual replacement. Remove root `fail_fast`. |
| `biscuit-tui` component/style templates, Schematic project-management prompt, Reaper schema prompt, Darkmatter parsing overview | Migrate row/current/adjacent/boundary reads; remove schemas claiming generated frontmatter globals. Component terminal messages describe container completion. |
| Current Claudine topic pages, getting-started guide, skill architecture/CLI references | Migrate retired bindings, serial group selectors, task stacks, sequence continuation policy, and outdated planned markers. Current docs own behavior, without links to feature snapshots. |
| Three L3 sequence Ctrl-C fixtures | Remove `execution: parallel`; concurrency is implicit. Preserve their existing live tier and platform guards. |
| Claudine rename/removed-grammar tests, literal agent-data tests, `loop_gate_ambient` authored `next`, lifecycle authored-state fixtures | Preserve: these assert rejection, literal transport, or ordinary authored properties. Old names in rename catalogs and parser errors remain intentional. |
| Darkmatter template-discovery/execution/policy tests, frontmatter name-coercion tests, schema generated-object tests | Preserve: Darkmatter has a host-defined binding catalog; these explicitly supply their own `outputs`/`state` values or validate authored schema properties. They do not use Claudine's removed runtime globals. Darkmatter's Claudine-facing preflight examples migrate to `seq.outputs`. |
| Internal Rust `sequence_id`, `total_steps`, retained task stack fields and `GroupExecution` | Preserve internal model/API identifiers; migrate misleading comments about accepted user grammar. Runtime binding projection already removes `sequence_id` and publishes `run_id`. |
| `features/`, `fixes/`, `timeline.md`, old `plans/`/`reviews/`, captured `agent-output` and provider NDJSON | Historical snapshot or recorded provider data; preserve verbatim (apart from the new dated timeline entry). |
| GitHub Actions `steps.*.outputs`, workflow `execution` names, Rust child-process `setup` module, GraphQL `state`, OpenAPI `total_steps`, cache outputs, loop-local `fail_fast`, benchmark `--fail-fast` | Unrelated grammar/code or unchanged document-loop policy; preserve. |

The diagnostic regression `a_param_targeting_a_reserved_key_is_rejected_before_the_provider_runs` failed with the original `params: { seq: hijacked }` input because its public error still called the retired roots executor-owned. The message now names `seq`, `group`, and `loop`; the test retains the failed status and no-provider-launch assertions. Source edits otherwise repair comment drift, not runtime behavior.

### Artifact migration and structural correction (8.2)

- Migrated internal prompt grammar and row bindings, plus the three L3 Ctrl-C fixture declarations. The implementation prompt's stale root boundary read became `loop.is_last`; optional handoff reads and shell inputs are guarded/defaulted so strict preflight succeeds. Route/frozen copies were re-derived and the pinned route hash refreshed through the existing Darkmatter/biscuit-hash API. Authored report `previous`/`next` fields remain intact.
- Research fleets needed a structural migration: their root hooks previously ran per body row, while container hooks now run once. Twenty-two `_fleet.md` coordinators now select shared formal task projections and per-topic `_research.md` prompt documents. Each row prompt owns its freshness and success checks. Two shared task projections retain canonical provider/runner rows and defaults; a parity test makes roster drift fail rather than silently forking metadata. No parser or task-source syntax was added. The three removed sequence `fail_fast: false` settings now follow mandatory unhandled-failure propagation; coordinators tell callers to correct failures and rerun, skipping current rows. This is a migration consequence, not a new failure-policy option.
- Migrated 16 summary prompt predicates, the research authoring standard/model-serving prompt, biscuit-tui templates, Schematic's project-management prompt, Reaper's research prompt, and Darkmatter's parsing overview and Claudine-facing preflight examples. Component root terminal messages now describe container completion. The source graph corpus stages documents in a private repository and freezes its output-style include, so normal L1 never depends on live internal prompts.
- Expanded docs beyond the seven required topic pages where the repository sweep found active drift: execution flow, schema behavior, preflight, looping, and getting started. Skills now document coordinator/row ownership and roster parity. Loaded the drift review; the skill audit's `sniff docs --package claudine` inventory succeeded.
- Preserved the pre-existing local-runner prompt's unconditional initialization skip. It is an independent freshness-policy defect, not removed container grammar; no unrelated behavior change was introduced.

Required opt-in prompt checkpoint: **31/31 passed**, including all `shipped_prompt_contract` and `shipped_prompt_route_drift` tests, current-grammar/retired-read/sequence corpus, actual shipped review-loop failure, optional `next_spec` present/absent, guide rules, persisted handoff, and mapping-only validation. Seven log-state cases were split into declared individual tests with a table-coverage guard; each targeted case took 2.5–2.9 seconds.

The full opt-in prompt run also exposed three pre-existing stale completion-contract tests: `shipped_implement_prompts_transclude_finish_before_exit` expects a missing snippet; `shipped_implement_plan_final_phase_delivers_completion_flags` expects older instructions; `shipped_implement_plan_warns_when_final_phase_exits_without_flags` expects an absent warning branch. These are outside the required Phase 8 checkpoint and remain failures in the full opt-in prompt suite. Seven unchanged `pr_flow_rehearsal` cases took 5–7 seconds under load. Normal L1 excludes this opt-in binary by design.

### Exact targeted tests and coverage

| Requirement | Added or strengthened tests |
| --- | --- |
| Removed diagnostic vocabulary | `composition::sequence::task::tests::prompt_tasks::a_param_targeting_a_reserved_key_is_rejected_before_the_provider_runs` (failed before correction; passed afterward in 0.034 s) |
| Passive shipped internal prompts | `shipped_prompts::internal_lifecycle_corpus_parses_under_current_grammar`, `shipped_prompts::shipped_prompt_corpus_parses_frontmatter`, `shipped_prompts::shipped_prompts_use_mapping_only_set`; existing full `shipped_prompt_contract` and `shipped_prompt_route_drift` modules |
| Public prompt failure and optional input routes | `shipped_prompts::shipped_review_loop_stops_after_its_first_failed_prompt`; `shipped_prompt_contract::feature_review_next_spec_input_drives_refresh_paths`; `shipped_prompt_contract::feature_review_without_next_spec_omits_refresh_instructions`; `prompt_guide_defects::shipped_guide_teaches_lexical_sequence_bindings_and_containers` |
| Handoff log variants, with one spawn per test | `implement_plan_logging_starts_missing_log`, `implement_plan_logging_starts_empty_log`, `implement_plan_logging_starts_whitespace_log`, `implement_plan_logging_preserves_handoff_and_routes`, `implement_plan_logging_preserves_frontmatter_only`, `implement_plan_logging_preserves_body_only`, `implement_plan_logging_preserves_first_phase`, `every_implement_log_case_has_a_test` in `shipped_prompt_contract` |
| Research/template/summary corpus and row ownership | `research_sequence_artifacts::shipped_research_sequence_corpus_uses_lexical_row_bindings`; `research_sequence_artifacts::shipped_fleet_graphs_resolve_per_row_prompt_lifecycles` |
| Canonical roster consistency | `research_sequence_artifacts::research_task_rosters_preserve_canonical_rows_and_templates` |
| Actual shipped artifacts through normal CLI | `research_sequence_artifacts::shipped_component_sequence_renders_rows_through_the_normal_cli`; `research_sequence_artifacts::shipped_research_fleet_runs_row_freshness_and_success_through_normal_cli`; `research_sequence_artifacts::shipped_research_fleet_refuses_zero_exit_without_a_research_document` |

The six research tests passed **6/6** (0.029–2.160 s). Their normal invocations observe rendered row names/paths, output files, repeat-run freshness skips, provider launch counts, negative exit status, and no audio publication. The missing-output regression preserves the original no-write/zero-exit input and asserts its existing invalid-path failure rather than inventing a new success path. The corpus calls the public removed-read validator, preserving ordinary prose such as “state.”; the graph checks executable row membership rather than incorrectly treating deduplicated prompt references as one reference per row.

The first full L1 run found lifecycle tests still reading row hooks from `_fleet.md`; those tests and a generator gate test now read `_research.md`. The library execution fixture supplies `seq` through `StackExecutionContext.seq` instead of authored frontmatter. This keeps the actual row recovery/finalization policy under test. No failing assertions or cases were removed.

New L1 tests are declared by `cli/tests/l1/main.rs`; prompt additions are compiled by the existing opt-in `prompts` target and `prompt-tests` feature. Existing L3 fixture tests retain their names, feature gates, and live `test-l3` recipe. No new terminal/browser test, target, feature, CI scope, or gate was added.

### Final corpus and recipe audit

- Found that `test-prompts` was documented but absent from the area justfile. Added the opt-in recipe for the existing `prompts` binary with `prompt-tests,test-fixtures`; ordinary L1 and CI remain unchanged. `just test-prompts shipped_prompt_route_drift::` passed **4/4** through the new recipe.
- Strengthened the artifact corpus to inspect decoded `prompt:` text as well as the document body. This reproduced retired `state.name` predicates in the quoted YAML prompts `summary/mcp.md` and `summary/plugins.md`; both now use `seq.name`. All **18** summary prompts are migrated (the earlier 16 count covered multiline YAML only). A directory-to-list guard prevents added topic/summary artifacts from silently missing corpus coverage.
- Lint identified two test-only Markdown conversions as infallible `TryFrom` calls. Replaced them with `Markdown::try_from_content`, preserving the corpus's promise to reject malformed frontmatter instead of forgiving it. No formatter was invoked.
- Full area L1 passed **9,789/9,789** in **153.456 s** before the final corpus strengthening; the changed corpus and artifacts are rechecked below. The six new L1 tests took **0.041–3.419 s** in the full run. Five slow reports belonged to unchanged tests.

### Final verification and handoff

- Final strengthened research corpus/CLI selection: **6/6 passed** in **1.863 s**, all under two seconds. The quoted/native YAML prompt variants and original missing-output input are covered.
- Area `just lint`: **passed** all five packages, diagnostic transport guards, and lifecycle documentation facets after correcting the test constructors. The shared lint recipe runs Clippy; no formatter was called.
- `just test-prompts shipped_prompt_route_drift::`: **4/4 passed** through the newly added canonical wrapper. The broader required prompt checkpoint was **31/31 passed**. Three unrelated pre-existing failures in the full opt-in prompt suite remain explicitly listed above; they are not hidden by the normal L1 result.
- Nine pre-existing ignored L1 tests remain: `compose_emits_first_stderr_byte_within_budget`, `bench_system_prompt_resolution_cold_and_warm`, `bench_resolve_and_prepare_step_by_step`, `bench_request_topology_probe_and_reuse`, `bench_raw_darkmatter_compose_passes`, `perf_compose_empty_partial_meets_target`, `perf_compose_long_prefix_meets_target`, `perf_inline_compose_empty_partial_meets_target`, and `perf_enter_compose_partial_meets_target`. No ignore, retry, timeout increase, tier move, or dropped case was introduced.
- The full L1 run's unchanged slow-test identities were:
  - `claudine-cli::l1 compose_initialize_staged_boot::early_catch_shells_cannot_run_or_enable_another_catch_shell`
  - `claudine-cli::l1 compose_schema_cli::compose_unresolved_caller_file_fails_before_initialize_when_non_interactive`
  - `claudine-cli::l1 provider_tail_notice::a_switch_whose_record_is_typed_unknown_is_not_called_known`
  - `claudine-cli::l1 sequence_sources_cli::every_string_list_form_classifies_through_the_real_invocation_path`
  - `claudine-cli::l1 sequence_sources_cli::yaml_json_json5_jsonl_and_ndjson_sources_produce_the_same_plan`

- Reviewed the OS skill. This phase adds no platform-specific runtime behavior; the three OS-specific L3 fixtures only drop retired YAML selectors. Linux/native Windows/WSL2, terminal L2, and OS-injection L3 were not rerun for this prose/artifact/diagnostic migration. Hermetic CLI/filesystem L1 covers its observable behavior, without focusing any window. Cross-platform execution remains part of normal repository CI and Phase 9's final scope assessment.
- `md get` read back every edited frontmatter key across **70** documents before the final two quoted-summary edits, and read back those two `prompt` keys afterward. Plan/log inventories include all **24** source/test/recipe inputs, **66** updated documents, **24** created documents, and **6** updated skill files; cumulative package membership is preserved.

- Final `just check-tier-coverage claudine`: **passed, zero stranded tests**. `git diff --check` passed. Both plan/log inventory sets were compared against every changed/created file and matched exactly; all edited metadata keys parsed through `md get`.
- All Phase 8 tasks (8.1–8.4) are checked. Implementation is complete, ready for review and Phase 9 final verification, with the pre-existing opt-in prompt failures, slow-test debt, and local-runner freshness defect recorded explicitly. `human_review` remains false because none requires a human decision before Phase 9. The handoff message replaces Phase 7's message. No formatting, staging, commit, or lifecycle move was performed.

## Phase 9

### Verification scope and requirement-to-test mapping before edits

Phase 9 verifies existing behavior; no production, parser, configuration, or persistence change is planned. Existing public CLI, filesystem, runtime, grammar-matrix, and terminal tests are the starting regression coverage. Criteria 1–23 will be mapped to concrete test identities below. Required gates: Claudine L1/L2/lint, Messenger L1/lint, Darkmatter L1, tier coverage, and the CI plan. Gates run sequentially where rebuilding could replace Claudine fixture-enabled binaries. Prior phase changes and unrelated worktree edits are preserved. No formatting, staging, commit, or lifecycle move is authorized.

Started `just test` in `claudine/`; full output is retained in `/tmp/sequence-phase9-claudine-test.log`. Prior opt-in prompt failures and slow tests remain explicit audit inputs, not passing evidence.

### Robustness grep audit (9.3)

Ran the named smell searches over Messenger refresh state/runner/prepare/select and Claudine sequence grammar/formal/normalize/source/model/task/group plus lifecycle parse/action-shape/actions. No fallback silently empties the matrix's required inputs. Messenger `RunRecord.status`, `recovery_attempts`, and `stages` are required; `invocations` defaults only for the explicitly specified legacy migration. `ledger_runs` is retained read-only legacy data; `launched` and `runner` are backward-compatible runner observations. The `.ok()` reads in `select::renewals` explicitly ignore unreadable renewal records (reported by cleanup), not run records; `prepare` reads an optional published baseline. Lifecycle defaults supply an absent suggestion suffix or omitted retry/proxy overlay after shape validation. Task defaults supply absent internal runtime state/deferred parameters/group outcome bindings; its `filter_map` projects known runtime keys, not malformed input elements. Lifecycle `filter_map` sites select action kinds or reorder validated entries. Required sequence/group/cancelled fields are validated before these execution paths.

Existing matrix tests are `container_yaml_matrix::markdown_and_sequence_yaml_fields_matrix`, `sequence_task_shape_matrix_and_exclusivity`, `group_fields_matrix` and `group_and_catalog_file_fields_matrix` in sequence preflight tests, and Messenger `the_run_record_matrix_is_typed_for_every_load_bearing_field`. The latter exercises real persisted run records through public loading, including legacy admission counts, duplicates, null, invalid elements, and trailing content. No input format or load-bearing field changes in Phase 9; no new matrix or persistence semantics are introduced.

### Acceptance-criteria audit (9.2)

The table uses exact test function names, searchable in the declared Claudine L1 binary/library tests, L2 binary, opt-in prompts binary, and Messenger research integration target. No criterion lacks a test. Platform execution gaps are separate from missing tests (see 9.6).

| Criterion | Concrete test identities |
| --- | --- |
| 1: nesting, references, cycles, ordered concurrent legs | `shipped_container_example_runs_through_the_normal_sequence_command`; `referenced_sequence_suffix_and_origin_are_preserved`; `sequence_reference_cycle_reports_the_active_chain`; `nested_group_legs_keep_memory_private_but_reread_shared_files_between_steps` |
| 2: removed keys refused before effects | `nested_sequence_and_removed_task_keys`; `group_legs_require_unique_nonempty_names_and_execution_is_removed`; `sequence_task_shape_matrix_and_exclusivity`; CLI `criterion_2_fail_fast_*` tests |
| 3: event order, blocked, skip, once-only cleanup | `root_events_wrap_all_children_once_and_expose_native_results`; `root_initialize_skip_precedes_invalid_static_preflight`; `preflight_failure_emits_blocked_then_finalize_without_start`; `initialize_skip_runs_no_children_or_finalize_and_records_skipped_leg`; `group_events_see_settled_summaries_and_finalize_before_publication` |
| 4: failure propagation, retry, no-session resume | `unhandled_nested_failure_finalizes_innermost_first_and_never_starts_later_children`; `retry_restarts_legs_without_intermediate_finalize_or_duplicate_result`; `root_resume_reports_typed_no_session_error_and_cleans_up` |
| 5: atomic/tolerant groups and cancellation opt-in | `default_fail_fast_withholds_mutations_outputs_and_unadmitted_legs`; `tolerated_failed_legs_merge_mutations_and_publish_native_named_results`; `sibling_failure_cancels_opted_container_descendants_and_preserves_other_legs`; `sibling_failure_terminates_an_opted_prompt_and_refuses_its_finalize_retry` |
| 6: timeouts remain failures | `timed_out_nested_leg_uses_failure_retry_and_never_cancellation` |
| 7: Ctrl-C and descendant cleanup (L2) | `level2_ctrl_c_cancels_group_descendants_and_finalizes_innermost_first` |
| 8: group outcome timing, names, reserved variables | `group_events_see_settled_summaries_and_finalize_before_publication`; `group_fields_matrix`; `group_and_catalog_file_fields_matrix`; `group_legs_require_unique_nonempty_names_and_execution_is_removed` |
| 9: sequence positions, shorthand, IDs and reserved keys | `sequence_binding_composes_typed_rows_shorthand_and_authored_namespaces`; `nested_prompt_observes_shared_run_and_shallow_neighbors`; `inline_sequence_preflight_preserves_nested_positions_and_duplicates`; `object_step_extracts_state_and_rejects_reserved_state_key` |
| 10: hard rename and authored-property controls | `every_rename_row_is_typed_and_property_presence_preserves_data`; `parsed_surfaces_include_conditions_operands_and_shell_values`; `rename_presence_controls_apply_through_normal_preparation` |
| 11: innermost roots and restoration | `inner_group_and_sequence_roots_are_lexical_and_restored_after_return`; `child_results_are_native_and_parent_position_is_restored`; `sequence_roots_shadow_completely_and_restore_without_merging` |
| 12: body steps own no lifecycle | `body_only_steps_run_root_success_once`; `referenced_markdown_body_step_has_no_document_lifecycle` |
| 13: finalization gates publication | `finalize_failure_discards_nested_native_result`; `group_finalize_error_discards_leg_mutations_and_result` |
| 14: lift-ready parser | `criterion_14_deliberate_runtime_import_fails_the_guard` and lifecycle lift-ready import guard |
| 15: removed ledger and Messenger command | `the_budget_subcommand_is_an_unknown_subcommand`; budget-option/completion absence checks in `sequence_errors_cli`; Messenger research CLI preparation tests; full Messenger gates |
| 16: proxy cleanup ownership | `root_failure_proxy_runs_target_cleanup_without_source_cleanup`; `container_failure_proxy_skips_source_finalize_and_runs_target_finalize` |
| 17: shipped prompt migration | `shipped_prompt_contract::*`; `shipped_prompt_route_drift::*`; `internal_lifecycle_corpus_parses_under_current_grammar`; `shipped_research_sequence_corpus_uses_lexical_row_bindings`; `shipped_fleet_graphs_resolve_per_row_prompt_lifecycles` |
| 18: empty, skipped, reuse, cycles, write collisions | `empty_root_sequence_runs_one_lifecycle_with_empty_outputs`; `initialize_skip_runs_no_children_or_finalize_and_records_skipped_leg`; `sequence_reuse_is_legal_and_only_all_first_descendants_offer_bytes`; `mixed_reference_cycle_names_every_active_reference`; `nested_inline_write_back_collision_is_rejected` |
| 19: cleanup, state isolation, cancellation terminality | `blocked_cleanup_cannot_run_shell_commands`; `group_finalize_error_discards_leg_mutations_and_result`; atomic/tolerant tests above; `interrupted_sequence_keeps_cleanup_errors_secondary_and_never_retries`; `cleanup_shell_timeout_stays_secondary_to_cancellation` |
| 20: retry retention, adjacency, rereads | `two_retries_retain_mutations_and_publish_exactly_three_children_once`; `exhausting_two_retries_keeps_failure_and_publishes_nothing`; `retry_clears_child_outputs_and_finalizes_once_before_publication`; `document_retry_reenters_directly_without_finalizing_failed_attempt`; `document_resume_reenters_directly_without_finalizing_failed_attempt`; deterministic nested-leg test from criterion 1 |
| 21: nested and closed approvals | `a_first_group_qualifies_with_every_leg_and_group_readers_are_templates`; `sequence_reuse_is_legal_and_only_all_first_descendants_offer_bytes`; `container_events_are_discovered_and_terminal_commands_require_templates`; `container_shells_use_event_time_values_and_lexical_group_outcomes`; `root_retry_keeps_approval_closed_when_a_referenced_task_changes`; `root_proxy_refuses_a_new_shell_after_approval_closes_even_with_yolo` |
| 22: Messenger locks, interruption, stale and legacy records | `a_live_runner_refuses_a_second_run`; `the_runner_records_finished_interrupted_and_stopped`; `a_stale_active_run_is_recovered_explicitly_and_never_silently`; `cleanup_and_promotion_are_refused_during_a_live_run`; `a_ledger_era_exhausted_run_is_readable_resumable_and_never_promotable` |
| 23: runtime-only retry overlay | `root_retry_retains_writes_clears_abandoned_outputs_and_finalizes_once`; `retry_clears_child_outputs_and_finalizes_once_before_publication`; group retry tests |

No tests are added or renamed in Phase 9: its source edits correct comments only. Public behavior and all existing positive/negative/representation cases remain unchanged. The existing grammar, shipped-artifact corpus, normal CLI invocation, persisted-record tests, and L2 terminal boundary remain the relevant verification levels.

### Comment and current-document drift pass (9.4)

Reviewed the changed container grammar/model, preflight, lifecycle/binding/cancellation/runtime state, recursive task/group/sequence execution, root CLI orchestration, Messenger refresh/run-record ownership, and their current topic/skill guidance. Preserved internal legacy task-stack/serial-dispatch descriptions where the implementation still has those branches; those descriptions do not authorize retired authoring grammar. Separate `seq`/`group` channels, caller provenance, cancellation scopes, and `RuntimeSnapshot.proxy_chain` are retained.

Corrected these stale comments in favor of the implementation:

- `sequence/model.rs`: removed an obsolete future-phase narrative and stale feature link; described the shared typed model and separate lexical `seq` channel.
- CLI `sequence/jit.rs`: override documentation no longer claims `seq` is frontmatter; mid-run compose errors enter container failure handling rather than the removed sequence `fail_fast` policy.
- CLI `sequence/iterate.rs`: updated module output names and failure handling, pre-warm layer description, and late-required-property failure documentation.
- CLI `sequence/mod.rs`: static preflight errors block the root before children; removed the obsolete comparison with sequence `fail_fast`.
- `sequence/task/mod.rs`: runtime cell documentation now describes concurrent legs rather than presenting serial groups as current authoring behavior.

These five files have comment-only Phase 9 edits. No behavior, format string, constant, test, target, dependency, or skill contract changes. No new targeted test is needed for the comment corrections; the required broad gates exercise the unchanged implementation. Current topic docs already describe implemented container execution, in-memory isolation, real-effect dry runs, cancellation, and closed approvals. No additional skill update is warranted.

### Gate evidence (in progress)

- Claudine `just test`: **9,789 passed, 9 pre-existing skips**, 151.575 s, exit 0. No failures or slow-test reports. Output: `/tmp/sequence-phase9-claudine-test.log`.
- `just ci-local --plan`: exit 0. Output: `/tmp/sequence-phase9-ci-plan.log`. The plan contains existing package/environment/tier cells and governed gaps; Phase 9 changes no manifest, feature, tier, workflow, environment catalog, or CI policy. The five comment-only source files already belong to packages selected by earlier implementation work, so this phase introduces no new CI cell. The printed multi-environment plan is a scope preview, not evidence those tests executed. Windows/WSL2 L2 remain governed accepted gaps, not passing observations.
- Started Claudine `just test-l2`; output: `/tmp/sequence-phase9-claudine-l2.log`. The criterion-7 fixture uses a headless tmux pane; other terminal fixtures use existing background spawn defaults. No focusing L3 recipe is run.

### Other-OS assessment (9.6)

Loaded the `os` skill and its build-host instructions. This final phase changes only Rust comments and implementation records; it adds no OS-specific code, process behavior, path handling, terminal behavior, or test fixtures. All required local gates run on macOS. No new Linux, native Windows, or WSL2 rig run is needed for those comment-only edits. Earlier Phase 6 evidence includes targeted Linux library cases and native Windows library/CLI container cases; those precede later host-neutral refinements and are **not final-tree receipts**. WSL2 has no local execution evidence for this feature.

Final Linux/macOS execution beyond this local macOS evidence relies on ordinary PR CI. Native Windows and WSL2 rely on scoped nightly CI (or an explicitly requested all-OS run). Native Windows criterion-7 console Ctrl-C coverage remains unmet: its test explicitly reports the unavailable Unix tmux backend, and provisioning a non-focusing native console-control harness is the required follow-up. Windows/WSL2 L2 policy gaps are reported as gaps, never as passes. The spec's requirement is unchanged. No cross-check invocation, remote mutation, or remote evidence claim is made in Phase 9.

- Claudine `just test-l2`: exit 0; CLI **287/287 passed** (46.075 s), generator **3/3 passed** (0.777 s). Criterion-7 Ctrl-C cleanup passed in 1.000 s. The 3,724 CLI and 250 generator entries excluded by the L2 filter are other-tier tests, not unexecuted L2 cases. No selected macOS L2 failure or capability skip was reported. Output: `/tmp/sequence-phase9-claudine-l2.log`.
- Started Claudine `just lint`; output: `/tmp/sequence-phase9-claudine-lint.log`.

### Final search correction

The criterion-15 whole-repository search (including hidden skills, excluding feature/fix snapshots and timelines) found one remaining hit in `.claude/skills/messenger/research-contract.md`: a historical note literally named the removed commands. Loaded the Messenger skill and reworded that note to describe the removed workflow without the prohibited spellings. Legacy exhausted-record and leftover-file guidance is preserved. This is a skill-text correction, not parser/runtime behavior. The exact search was rerun after the edit; no source test or CI cell is added. Phase 9's skill inventory includes this correction; Claudine's skill architecture remains accurate.

- Claudine `just lint`: **passed**, exit 0. Includes all five package Clippy checks, diagnostic transport checks (**9/9 passed**), and lifecycle documentation facets. No formatter was invoked. Output: `/tmp/sequence-phase9-claudine-lint.log`.
- Started Messenger `just test`; output: `/tmp/sequence-phase9-messenger-test.log`.
- Phase 9 inventory metadata was written surgically to plan/log and every edited key read back with `md get`: five comment-only Rust source inputs, three updated implementation documents, no created docs, one updated Messenger skill file. Cumulative inventories union all prior phase lists: **192 source inputs**, **104 documentation files**. All listed paths exist; cumulative package membership is preserved.

A surrounding-text review then found two stale Messenger skill bullets still describing ledger-owned recovery and `state::apply_ledger`, which no longer exists. Replaced them with the implemented `prepare::resume`/runner status behavior, durable invocation counts, explicit stale recovery, and the distinction between zero exit and completed research. Verified against `prepare.rs`, `runner.rs`, and the existing `recovery_is_bounded`, `the_runner_records_finished_interrupted_and_stopped`, and `a_stale_active_run_is_recovered_explicitly_and_never_silently` tests. This resolves guidance drift in favor of code; no behavior change or new test is introduced.

- Messenger `just test`: Nextest gate **passed**, exit 0; **721/721 passed**, **2 pre-existing skips**, 23.171 s. Output: `/tmp/sequence-phase9-messenger-test.log`. **11 unchanged tests exceeded the five-second L1 budget**; a passing Nextest result does not satisfy the separate speed requirement for those cases. No timeout override, ignore, retry, tier move, case deletion, or performance experiment is introduced. Desktop timeout fixtures wait for the production 3,000 ms timeout and use serialized environment guards; research fixtures perform repeated filesystem publication. Their speed debt is outside these comment/skill corrections and remains unresolved:

  - `messenger tests::desktop_helpers::notify_send_stub::notice_only_timeout_maps_to_timeout_error`: 5.019 s.
  - `messenger tests::desktop_helpers::snoretoast_stub::notice_only_timeout_maps_to_timeout_error`: 5.024 s.
  - `messenger tests::desktop_helpers::terminal_notifier_stub::notice_only_timeout_maps_to_timeout_error`: 5.025 s.
  - `messenger::research_publication a_generation_interrupted_after_selection_recovers_to_the_new_snapshot`: 6.159 s.
  - `messenger tests::desktop_helpers::burnttoast_stub::notice_only_timeout_maps_to_timeout_error`: 10.026 s.
  - `messenger::research_refresh a_verified_unchanged_renewal_is_accepted_without_a_changelog_entry`: 5.187 s.
  - `messenger::research_refresh an_initial_publication_promotes_every_platform_together`: 6.207 s.
  - `messenger::research_refresh an_unreadable_run_record_blocks_its_platform_and_names_its_path`: 5.924 s.
  - `messenger::research_refresh automatic_renewal_is_refused_unless_everything_substantive_is_unchanged_and_rechecked`: 6.270 s.
  - `messenger::research_publication a_generation_interrupted_before_selection_recovers_to_the_old_snapshot`: 20.266 s.
  - `messenger::research_refresh an_interrupted_promotion_recovers_to_one_consistent_snapshot_and_completes_on_retry`: 15.772 s.

- Started Messenger `just lint`; output: `/tmp/sequence-phase9-messenger-lint.log`.

### Existing exclusions and limitations

- Claudine's nine ignored L1 performance cases are unchanged: `compose_emits_first_stderr_byte_within_budget`, `bench_system_prompt_resolution_cold_and_warm`, `bench_resolve_and_prepare_step_by_step`, `bench_request_topology_probe_and_reuse`, `bench_raw_darkmatter_compose_passes`, `perf_compose_empty_partial_meets_target`, `perf_compose_long_prefix_meets_target`, `perf_inline_compose_empty_partial_meets_target`, and `perf_enter_compose_partial_meets_target`.
- Messenger's two ignored credentialed integration cases are `smoke_test_slack_send` and `smoke_test_discord_send`; no messages are sent.
- The full opt-in prompt binary still has the three previously recorded completion-contract failures: `shipped_implement_prompts_transclude_finish_before_exit`, `shipped_implement_plan_final_phase_delivers_completion_flags`, and `shipped_implement_plan_warns_when_final_phase_exits_without_flags`. These are outside ordinary L1 and the required shipped-prompt checkpoint. Its unchanged slow PR rehearsal cases and local-runner unconditional initialize skip remain as recorded in Phase 8. No unrelated prompt behavior is changed here.
- OS-native keyboard-injection L3 and live-provider tests are not requested gates and are not run. Selected L2 uses headless/background harnesses. Native Windows Ctrl-C and governed Windows/WSL2 L2 gaps remain explicitly unmet, as detailed above.

- Messenger `just lint`: **passed**, exit 0, library with `desktop,research` and CLI checks. Output: `/tmp/sequence-phase9-messenger-lint.log`.
- Started root `just test darkmatter` for the schema changes; output: `/tmp/sequence-phase9-darkmatter-test.log`.

### Darkmatter gate regression and targeted correction

The initial full `just test darkmatter` run reproduced this failure before edits:

```text
FAIL darkmatter::l1 current_root_documentation_contract::every_required_page_explains_the_binding_time_model
AC29 documentation contract:
  claudine/docs/topics/composition.md: missing "**`ctx` is evaluated once for each composition run.** ... each sequence step (each task of a serial group; a parallel group shares one capture until a sibling re-enters) ..."
```

Requirement-to-test map before correction: the existing public documentation-contract test is the failing regression; its sibling `the_check_reports_each_missing_phrase_and_tolerates_reflow` verifies the guard cannot silently pass missing text. The declared `darkmatter/lib/tests/l1/main.rs` includes the module, its unmarked names run in L1, and there is no new test or feature gate. This is a documentation-assertion update, not a parser or configuration change.

Loaded the Darkmatter skill. The composition page correctly describes concurrent groups, matching `open_group_run`/`open_leg_run`; the test's exact paragraph and the context-variables page/Claudine CLI skill still described the retired serial-group form. Updated those three copies to the composition page's concurrent-group wording. Preserved every other binding-time clause and the exact-paragraph guard. This resolves drift in favor of code rather than restoring removed user grammar. Phase 9 now includes one test assertion change, one additional current topic page, and one Claudine skill page beyond the earlier inventories. No production behavior changes.

- Initial root `just test darkmatter`: **10,113 passed, 1 failed, 10 skipped**, 38.677 s, exit 100. The failure output and correction are recorded above; no other assertion failed.
- Post-correction root invocation (`just test darkmatter current_root_documentation_contract`): the root recipe treats arguments as package/area selectors, so it selected the same four Darkmatter packages and ran the **entire** L1 gate, not a test-name filter. **10,114/10,114 passed**, 10 pre-existing skips, 64.279 s, exit 0. Output: `/tmp/sequence-phase9-darkmatter-regression.log`. A separate package-area `just test current_root_documentation_contract` is used for the exact targeted guard run below.
- The five unchanged Darkmatter/DMLS tests over the five-second L1 budget in the passing full run were:

  - `darkmatter markdown::compose::shell_expansion::probe::tests::installed_shells::injection_shaped_names_never_execute`: 5.926 s.
  - `dmls::l1 claudine_schema_discovery::trigger_payload_items_and_values_complete_without_a_schema`: 5.843 s.
  - `dmls::l1 claudine_schema_discovery::trigger_payload_keys_complete_without_a_schema`: 6.864 s.
  - `dmls::l1 claudine_schema_discovery::trigger_payload_properties_hover_without_a_schema`: 6.902 s.
  - `dmls::l1 claudine_schema_discovery::an_authored_redeclaration_wins_over_the_trigger_per_property`: 6.949 s.

The corrected documentation-contract test itself passed in 0.073 s. No performance-related code or test policy was changed.

- Exact targeted Darkmatter regression: package-area `just test current_root_documentation_contract` **2/2 passed**, 0.053 s, exit 0. This selects both the actual-page contract and the negative/reflow guard. Output: `/tmp/sequence-phase9-darkmatter-targeted.log`.
- Darkmatter's ten unchanged ignored cases are `f35_6_rhythm_raw_samples`, `f23_code_surface_raw_samples`, `f35_7_link_policy_raw_samples` (measurement harnesses), `resolve_alias_ll` (user login-shell alias), and six DMLS alias-projection cases: `a_malformed_alias_target_is_parsed_once_and_reported_by_the_rejecting_alias`, `undeclared_properties_in_block_and_multi_line_expression_values_warn_at_their_own_ranges`, `tagged_anchored_and_aliased_expression_values_are_diagnosed_once_where_authored`, `an_aliased_dash_separated_key_fix_avoids_the_defining_scalars_quote`, `a_dash_separated_key_behind_an_unprovable_alias_offers_no_fix`, and `aliases_are_analyzed_in_the_definition_the_yaml_parser_resolved`.

- `just check-tier-coverage claudine`: **passed**, exit 0; zero stranded tests. Output: `/tmp/sequence-phase9-tier.log`. The changed Darkmatter guard is already compiled by the declared L1 target and its targeted run proves tier selection.
- Required opt-in shipped-prompt selection: `just test-prompts shipped_prompt_contract:: shipped_prompt_route_drift:: internal_lifecycle_corpus_parses_under_current_grammar shipped_prompt_corpus_parses_frontmatter shipped_prompts_use_mapping_only_set shipped_review_loop_stops_after_its_first_failed_prompt shipped_guide_teaches_lexical_sequence_bindings_and_containers`: **30/30 passed**, 2.487 s, exit 0. The 52 excluded cases are outside this name selection, including the three known opt-in failures listed above. Output: `/tmp/sequence-phase9-prompts.log`. A separate persisted-handoff case completes the prior phase's 31-case checkpoint.
- Final `just ci-local --plan`: **passed**, exit 0. Compared canonical `{package, environment, gate}` identities with the initial preview: **73 cells before, 73 after, zero added or removed**. Output: `/tmp/sequence-phase9-ci-plan-final.log`. This includes all inherited implementation scope, not just Phase 9; no new cell, target, feature, workflow, environment, or policy was introduced.

### Closing verification and final state

- Persisted-handoff shipped prompt: **1/1 passed**, 1.156 s, exit 0 (`just test-prompts shipped_implement_plan_reads_persisted_handoff_before_provider_launch`). Combined required shipped-prompt checkpoint: **31/31 passed**. Output: `/tmp/sequence-phase9-prompt-handoff.log`.
- All six Phase 9 tasks and the plan's overall definition-of-done checkboxes are checked. The criteria audit names concrete coverage for all 23 criteria. No new or renamed test exists; the targeted regression is the existing Darkmatter documentation contract, reproduced red before correction, then green alone and in the full gate.
- Final Phase 9 inventory: **six Rust source/test files** (five comment-only changes and one documentation assertion), **four updated documentation files**, **no created docs**, and **two skill files**. Cumulative `source_code` is **193** entries and `documentation` is **105**; inventories preserve all earlier phases and package membership. The updated source/test files are listed in both plan/log frontmatter.
- Set plan/log `completed_phase: 9` and `implemented: true`; spec `implemented: true` and `implemented_by: codex/gpt-6.1-sol`. Spec `status` remains the author's `finalized-spec`. `human_review` is false because no decision is needed before another implementation phase; this is the final phase. The handoff message replaces Phase 8's message and records review limitations.
- Required local Nextest and lint gates pass after the documentation correction. This does **not** claim all L1 speed budgets or all-OS acceptance coverage are satisfied: the 16 unchanged slow cases, ignored cases, three inherited opt-in failures, local-runner freshness defect, and native Windows/WSL2 limitations are explicitly retained above.
- Implementation complete, ready for review. No formatting, staging, commit, lifecycle move, or `just complete` was performed.

Final integrity checks passed: `md get` parsed every edited plan/log/spec key; plan/log inventories match, contain unique existing paths, and preserve cumulative packages; no unchecked plan task or duplicate top-level frontmatter key remains. Spec status is unchanged. `git diff --check` passed, the index remains empty, and the final removed-budget-command search returned zero hits. All test/build/plan sessions have finished.
