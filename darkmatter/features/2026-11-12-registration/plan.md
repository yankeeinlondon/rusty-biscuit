---
kind: plan
created: 2026-11-13
total_phases: 9
phase: 4
agent: codex/gpt-6.1-sol
yolo: true
source_files_during_phase_1:
- claudine/lib/Cargo.toml
- claudine/lib/generated/shipped.json
- claudine/lib/src/composition/schema/bundle.rs
- claudine/lib/src/composition/schema/findings.rs
- claudine/lib/src/composition/schema/shipped.rs
- claudine/lib/src/invocation_context/document_tests.rs
- claudine/lib/tests/l1/claudine_schemas_embedded.rs
- claudine/lib/tests/l1/claudine_schemas_matrix.rs
- claudine/lib/tests/l1/claudine_schemas.rs
- claudine/lib/tests/l1/claudine_trigger_selection_parity.rs
- claudine/lib/tests/l1/host_catalog.rs
- claudine/lib/tests/l1/invalid_trigger_findings.rs
- claudine/lib/tests/l1/shipped_artifacts.rs
- darkmatter/cli/src/commands/clean.rs
- darkmatter/cli/src/commands/schema/triggers.rs
- darkmatter/cli/src/commands/schema/validate.rs
- darkmatter/cli/src/commands/trigger_findings.rs
- darkmatter/cli/src/render_style.rs
- darkmatter/cli/tests/l1/schema_validate_yaml.rs
- darkmatter/dmls/src/diagnostics/frontmatter.rs
- darkmatter/dmls/src/overlay/mod.rs
- darkmatter/dmls/src/overlay/registry.rs
- darkmatter/dmls/src/overlay/schema.rs
- darkmatter/dmls/src/overlay/yaml_contract.rs
- darkmatter/dmls/src/overlay/yaml_document.rs
- darkmatter/dmls/src/router.rs
- darkmatter/dmls/src/slot_jobs.rs
- darkmatter/dmls/tests/l1/claudine_schema_discovery.rs
- darkmatter/dmls/tests/l1/schema_reference_parity.rs
- darkmatter/dmls/tests/l1/schema_roots_parity.rs
- darkmatter/dmls/tests/l1/yaml_document_editor.rs
- darkmatter/lib/benches/schema_validation.rs
- darkmatter/lib/Cargo.toml
- darkmatter/lib/generated/shipped.json
- darkmatter/lib/src/markdown/cleanup/document.rs
- darkmatter/lib/src/markdown/cleanup/frontmatter.rs
- darkmatter/lib/src/markdown/compose/context/report.rs
- darkmatter/lib/src/markdown/compose/expression/data/contract.rs
- darkmatter/lib/src/markdown/compose/expression/data/locate.rs
- darkmatter/lib/src/markdown/compose/expression/data/parse.rs
- darkmatter/lib/src/markdown/compose/schema_validation.rs
- darkmatter/lib/src/markdown/compose/tests/invalid_triggers.rs
- darkmatter/lib/src/markdown/output/command/clean.rs
- darkmatter/lib/src/markdown/output/command/data.rs
- darkmatter/lib/src/markdown/schemas/about.rs
- darkmatter/lib/src/markdown/schemas/catalog.rs
- darkmatter/lib/src/markdown/schemas/clean.rs
- darkmatter/lib/src/markdown/schemas/embedded.rs
- darkmatter/lib/src/markdown/schemas/mod.rs
- darkmatter/lib/src/markdown/schemas/roots.rs
- darkmatter/lib/src/markdown/schemas/schema_files.rs
- darkmatter/lib/src/markdown/schemas/shipped.rs
- darkmatter/lib/src/markdown/schemas/slots/codes.rs
- darkmatter/lib/src/markdown/schemas/tests/reset_process_caches.rs
- darkmatter/lib/src/markdown/schemas/triggers.rs
- darkmatter/lib/src/markdown/schemas/triggers/assemble.rs
- darkmatter/lib/src/markdown/schemas/triggers/discovery.rs
- darkmatter/lib/src/markdown/schemas/triggers/envelope.rs
- darkmatter/lib/src/markdown/schemas/triggers/finding.rs
- darkmatter/lib/src/markdown/schemas/triggers/grammar.rs
- darkmatter/lib/src/markdown/schemas/triggers/lint.rs
- darkmatter/lib/src/markdown/schemas/triggers/matcher.rs
- darkmatter/lib/src/markdown/schemas/triggers/mod.rs
- darkmatter/lib/src/markdown/schemas/triggers/payload_memo_tests.rs
- darkmatter/lib/src/markdown/schemas/triggers/prepared.rs
- darkmatter/lib/src/markdown/schemas/yaml_documents/load.rs
- darkmatter/lib/src/markdown/schemas/yaml_documents/marks.rs
- darkmatter/lib/src/markdown/schemas/yaml_documents/mod.rs
- darkmatter/lib/src/markdown/schemas/yaml_documents/registry.rs
- darkmatter/lib/src/markdown/schemas/yaml_documents/select.rs
- darkmatter/lib/src/markdown/schemas/yaml_documents/semantic.rs
- darkmatter/lib/src/work_counters.rs
- darkmatter/lib/tests/l1/base_schema_end_to_end.rs
- darkmatter/lib/tests/l1/deferred_positions.rs
- darkmatter/lib/tests/l1/directive_target_analysis.rs
- darkmatter/lib/tests/l1/kind_semantic_checks.rs
- darkmatter/lib/tests/l1/meta_schema_phase4.rs
- darkmatter/lib/tests/l1/prepared_trigger_bundles.rs
- darkmatter/lib/tests/l1/recursive_schema_golden.rs
- darkmatter/lib/tests/l1/recursive_schema_merge.rs
- darkmatter/lib/tests/l1/schema_cache_validity.rs
- darkmatter/lib/tests/l1/schema_embedded_trigger_bundles.rs
- darkmatter/lib/tests/l1/schema_language_ownership_guard.rs
- darkmatter/lib/tests/l1/schema_phase_validation.rs
- darkmatter/lib/tests/l1/schema_reference_parity.rs
- darkmatter/lib/tests/l1/schema_roots.rs
- darkmatter/lib/tests/l1/schemas_effective_presence.rs
- darkmatter/lib/tests/l1/schemas_literal_expression.rs
- darkmatter/lib/tests/l1/schemas_union_typed_consumers.rs
- darkmatter/lib/tests/l1/shared_acyclic_consumers.rs
- darkmatter/lib/tests/l1/shipped_artifacts.rs
- darkmatter/lib/tests/l1/shipped_schema_corpus.rs
- darkmatter/lib/tests/l1/trigger_discovery_recovery.rs
- darkmatter/lib/tests/l1/trigger_envelope_claim.rs
- darkmatter/lib/tests/l1/trigger_input_matrix.rs
- darkmatter/lib/tests/l1/trigger_match_grammar.rs
- darkmatter/lib/tests/l1/trigger_payload_load_errors.rs
- darkmatter/lib/tests/l1/trigger_selection_parity.rs
- darkmatter/lib/tests/l1/trigger_spelling_guard.rs
- darkmatter/lib/tests/l1/yaml_document_loading.rs
- darkmatter/lib/tests/l1/yaml_document_selection.rs
- darkmatter/lib/tests/l1/yaml_kind_registry.rs
- darkmatter/schemas/schema-trigger.yaml
- scripts/ci/test_affected_scope.py
- simplified-schema/lib/generated/shipped.json
- simplified-schema/lib/src/embedded.rs
- simplified-schema/lib/src/envelope.rs
- simplified-schema/lib/src/lib.rs
- simplified-schema/lib/src/roots.rs
- simplified-schema/lib/src/shipped/registry.rs
- simplified-schema/lib/src/slots.rs
- simplified-schema/lib/src/triggers/assemble.rs
- simplified-schema/lib/src/triggers/discovery.rs
- simplified-schema/lib/src/triggers/envelope.rs
- simplified-schema/lib/src/triggers/finding.rs
- simplified-schema/lib/src/triggers/grammar.rs
- simplified-schema/lib/src/triggers/lint.rs
- simplified-schema/lib/src/triggers/matcher.rs
- simplified-schema/lib/src/triggers/mod.rs
- simplified-schema/lib/src/triggers/payload_memo_tests.rs
- simplified-schema/lib/src/triggers/prepared.rs
- simplified-schema/lib/src/work_counters.rs
- simplified-schema/lib/src/yaml_documents/load.rs
- simplified-schema/lib/src/yaml_documents/marks.rs
- simplified-schema/lib/src/yaml_documents/mod.rs
- simplified-schema/lib/src/yaml_documents/registry.rs
- simplified-schema/lib/src/yaml_documents/select.rs
- simplified-schema/lib/src/yaml_documents/semantic.rs
- simplified-schema/lib/tests/l1/envelope_admission.rs
- simplified-schema/lib/tests/l1/shipped_artifacts.rs
- simplified-schema/lib/tests/l1/shipped_corpus.rs
- simplified-schema/schemas/schema-trigger.yaml
docs_updated_during_phase_1:
- darkmatter/docs/topics/schemas/authoring-schemas.md
- darkmatter/docs/topics/schemas/definition.md
- darkmatter/docs/topics/schemas/schema-discovery.md
- simplified-schema/README.md
- simplified-schema/docs/topics/features.md
- simplified-schema/docs/topics/language-core.md
- simplified-schema/docs/topics/language/schema-documents.md
- simplified-schema/docs/topics/shipped-artifacts.md
docs_created_during_phase_1: []
skills_files_updated_during_phase_1:
- .claude/skills/darkmatter/SKILL.md
- .claude/skills/darkmatter/cli.md
- .claude/skills/darkmatter/dmls.md
- .claude/skills/darkmatter/expressions.md
- .claude/skills/darkmatter/library-surfaces.md
- .claude/skills/darkmatter/schema.md
- .claude/skills/simplified-schema/SKILL.md
- .claude/skills/simplified-schema/build-and-guards.md
- .claude/skills/simplified-schema/schema-documents.md
packages:
- simplified-schema
- darkmatter
- darkmatter-cli
- darkmatter-lsp
- claudine
- sniff
- messenger
- biscuit-file
- claudine-cli
source_files_during_phase_2:
- claudine/lib/src/composition/schema/classify.rs
- claudine/lib/src/composition/schema/defaults_tests.rs
- claudine/lib/src/composition/schema/launch_defaults.rs
- claudine/lib/src/composition/schema/mod.rs
- claudine/lib/src/composition/schema/status_render.rs
- claudine/lib/tests/l1/claudine_schemas.rs
- darkmatter/cli/schemas/output/schema-validate.yaml
- darkmatter/cli/src/commands/schema/validate.rs
- darkmatter/cli/tests/l1/compose_schema.rs
- darkmatter/cli/tests/l1/output_commands.rs
- darkmatter/dmls/src/diagnostics/frontmatter.rs
- darkmatter/dmls/src/providers/frontmatter.rs
- darkmatter/dmls/tests/l1/main.rs
- darkmatter/dmls/tests/l1/required_defaults.rs
- darkmatter/lib/Cargo.toml
- darkmatter/lib/src/atomic_replace.rs
- darkmatter/lib/src/config/migration_receipt.rs
- darkmatter/lib/src/config/mod.rs
- darkmatter/lib/src/config/write.rs
- darkmatter/lib/src/effects/fs_write.rs
- darkmatter/lib/src/lib.rs
- darkmatter/lib/src/markdown/compose/expression/host/artifact.rs
- darkmatter/lib/src/markdown/compose/expression/scope/shape.rs
- darkmatter/lib/src/markdown/compose/pipeline/mod.rs
- darkmatter/lib/src/markdown/compose/schema_validation.rs
- darkmatter/lib/src/markdown/hash/mod.rs
- darkmatter/lib/src/markdown/hash/node_edit.rs
- darkmatter/lib/src/markdown/hash/path_edit.rs
- darkmatter/lib/src/markdown/schemas/mod.rs
- darkmatter/lib/src/markdown/schemas/yaml_documents/mod.rs
- darkmatter/lib/src/strict_json.rs
- darkmatter/lib/src/yaml_edit.rs
- darkmatter/lib/tests/l1/compose_setters.rs
- darkmatter/lib/tests/l1/composition_defaults.rs
- darkmatter/lib/tests/l1/config_migration_receipt.rs
- darkmatter/lib/tests/l1/config_write.rs
- darkmatter/lib/tests/l1/main.rs
- darkmatter/lib/tests/l1/schema_phase_validation.rs
- darkmatter/lib/tests/l1/validation_contract_parity.rs
- darkmatter/lib/tests/l1/yaml_edit.rs
- messenger/lib/src/research/load.rs
- messenger/lib/tests/research_corpus.rs
- simplified-schema/lib/examples/validate_person.rs
- simplified-schema/lib/src/coerce.rs
- simplified-schema/lib/src/defaults.rs
- simplified-schema/lib/src/errors.rs
- simplified-schema/lib/src/lib.rs
- simplified-schema/lib/src/outcome.rs
- simplified-schema/lib/src/prepared.rs
- simplified-schema/lib/src/triggers/finding.rs
- simplified-schema/lib/tests/l1/declaration_narrowing.rs
- simplified-schema/lib/tests/l1/envelope_admission.rs
- simplified-schema/lib/tests/l1/main.rs
- simplified-schema/lib/tests/l1/required_defaults.rs
- simplified-schema/lib/tests/l1/root_union_assertions.rs
- simplified-schema/lib/tests/l1/validation_contract.rs
- sniff/lib/src/error.rs
- sniff/lib/src/filesystem/git/committed_tree.rs
- sniff/lib/src/filesystem/git/mod.rs
- sniff/lib/src/filesystem/mod.rs
- sniff/lib/tests/l1/committed_tree.rs
- sniff/lib/tests/l1/main.rs
docs_updated_during_phase_2:
- claudine/docs/topics/composition.md
- claudine/docs/topics/frontmatter-properties.md
- darkmatter/dmls/docs/diagnostics.md
- darkmatter/dmls/docs/hover.md
- darkmatter/docs/cli/schema.md
- darkmatter/docs/inline/schema-validation.md
- darkmatter/docs/topics/darkmatter-expressions.md
- darkmatter/docs/topics/frontmatter.md
- darkmatter/docs/topics/schemas/authoring-schemas.md
- darkmatter/docs/topics/schemas/definition.md
- darkmatter/README.md
- simplified-schema/docs/topics/global-type-constraints.md
- simplified-schema/docs/topics/language/types.md
- simplified-schema/docs/topics/simplified-schema-struct.md
- simplified-schema/docs/topics/validation.md
- sniff/docs/sniff-library-architecture.md
docs_created_during_phase_2:
- darkmatter/docs/topics/config-writes.md
- darkmatter/docs/topics/yaml-editing.md
skills_files_updated_during_phase_2:
- .claude/skills/claudine/cli-commands.md
- .claude/skills/darkmatter/compose.md
- .claude/skills/darkmatter/dmls.md
- .claude/skills/darkmatter/frontmatter.md
- .claude/skills/darkmatter/library-surfaces.md
- .claude/skills/darkmatter/schema.md
- .claude/skills/darkmatter/SKILL.md
- .claude/skills/simplified-schema/SKILL.md
- .claude/skills/sniff/remote-and-repository.md
- .claude/skills/sniff/SKILL.md
source_files_during_phase_3:
- biscuit-file/lib/src/file_reference/context.rs
- biscuit-file/lib/src/file_reference/mod.rs
- biscuit-file/lib/src/file_reference/portable/mod.rs
- claudine/cli/src/commands/actions.rs
- claudine/cli/src/commands/compose/prep.rs
- claudine/cli/src/commands/config_tui/app.rs
- claudine/cli/src/commands/config_tui/mod.rs
- claudine/cli/src/commands/config_tui/persistence.rs
- claudine/cli/src/commands/config_tui/persistence_tests.rs
- claudine/cli/src/commands/config_tui/tabs/tts.rs
- claudine/cli/src/commands/hooks/list.rs
- claudine/cli/src/commands/init/mod.rs
- claudine/cli/src/commands/init/prompts.rs
- claudine/cli/src/commands/init_wizard.rs
- claudine/cli/src/commands/link_display.rs
- claudine/cli/src/commands/sequence.rs
- claudine/cli/src/commands/uninstall.rs
- claudine/cli/src/commands/wrap/composition/tests.rs
- claudine/cli/src/commands/wrap/env/mod.rs
- claudine/cli/src/commands/wrap/runaway_guard.rs
- claudine/cli/src/completion/engine/mod.rs
- claudine/cli/src/completion/engine/tests.rs
- claudine/cli/src/main.rs
- claudine/cli/src/request.rs
- claudine/cli/tests/common/mod.rs
- claudine/cli/tests/common/pty.rs
- claudine/cli/tests/common/review_screen.rs
- claudine/cli/tests/common/webhook_listener.rs
- claudine/cli/tests/common/wrap.rs
- claudine/cli/tests/l1/agent_cwd.rs
- claudine/cli/tests/l1/argv_normalization.rs
- claudine/cli/tests/l1/command_routing.rs
- claudine/cli/tests/l1/completion_ownership.rs
- claudine/cli/tests/l1/completion_perf.rs
- claudine/cli/tests/l1/config_yaml_wiring.rs
- claudine/cli/tests/l1/contamination_probes.rs
- claudine/cli/tests/l1/darkmatter_config_gate.rs
- claudine/cli/tests/l1/detached_audio.rs
- claudine/cli/tests/l1/handle_blocking_output.rs
- claudine/cli/tests/l1/handle_repo_config.rs
- claudine/cli/tests/l1/home_lookup_round_trip.rs
- claudine/cli/tests/l1/level1_compose_autocomplete_failure_pty.rs
- claudine/cli/tests/l1/level1_dry_run_pty.rs
- claudine/cli/tests/l1/level1_inline_compose_mismatch_pty.rs
- claudine/cli/tests/l1/level1_lifecycle_shell_terminal_pty.rs
- claudine/cli/tests/l1/main.rs
- claudine/cli/tests/l1/pi_managed_rpc.rs
- claudine/cli/tests/l1/protect_cli.rs
- claudine/cli/tests/l1/skills_integration.rs
- claudine/cli/tests/l1/tts_prewarm.rs
- claudine/cli/tests/l1/wrap_compose_agent.rs
- claudine/cli/tests/l1/wrap_compose_preflight.rs
- claudine/cli/tests/l1/wrap_ctrl_c_windows.rs
- claudine/cli/tests/level2/level2_auto_complete_chooser.rs
- claudine/cli/tests/level2/level2_auto_complete_operation_file.rs
- claudine/cli/tests/level2/level2_dry_run_approval_capture.rs
- claudine/cli/tests/level2/level2_dry_run_metadata_capture.rs
- claudine/cli/tests/level2/level2_explicit_operation_file_miss.rs
- claudine/cli/tests/level2/level2_file_resolution_capture.rs
- claudine/cli/tests/level2/level2_inline_compose_mismatch_capture.rs
- claudine/cli/tests/level2/level2_invalid_file_reference_capture.rs
- claudine/cli/tests/level2/level2_lifecycle_control.rs
- claudine/cli/tests/level2/level2_malformed_frontmatter_capture.rs
- claudine/cli/tests/level2/level2_perf_capture.rs
- claudine/cli/tests/level2/level2_prompt_reporting_capture.rs
- claudine/cli/tests/level2/level2_removed_validation_key_capture.rs
- claudine/cli/tests/level2/level2_schema_parse_capture.rs
- claudine/cli/tests/level2/level2_sequence_container_ctrl_c.rs
- claudine/cli/tests/level2/level2_sequence_task_stream_capture.rs
- claudine/cli/tests/level2/level2_typed_error_render_capture.rs
- claudine/docs/providers/dispatch-inventory.json
- claudine/lib/benches/runtime_hot_paths.rs
- claudine/lib/generated/shipped.json
- claudine/lib/src/composition/error/tests.rs
- claudine/lib/src/composition/lifecycle/executor.rs
- claudine/lib/src/composition/lifecycle/prewarm/worker.rs
- claudine/lib/src/composition/lifecycle/tests/tts_settings.rs
- claudine/lib/src/composition/mod.rs
- claudine/lib/src/composition/prepare.rs
- claudine/lib/src/composition/sequence/task/mod.rs
- claudine/lib/src/config/claudine_config.rs
- claudine/lib/src/config/claudine_config/tests.rs
- claudine/lib/src/config/json_conversion.rs
- claudine/lib/src/config/load.rs
- claudine/lib/src/config/messaging_block/tests.rs
- claudine/lib/src/config/mod.rs
- claudine/lib/src/config/persist.rs
- claudine/lib/src/config/persist_tests.rs
- claudine/lib/src/config/schema.rs
- claudine/lib/src/config/schema_tests.rs
- claudine/lib/src/config/tts.rs
- claudine/lib/src/dispatch/deps.rs
- claudine/lib/src/dispatch/loader.rs
- claudine/lib/src/dispatch/loader/tests.rs
- claudine/lib/src/dispatch/runner/speak.rs
- claudine/lib/src/dispatch/runner/tests.rs
- claudine/lib/src/dispatch/tests.rs
- claudine/lib/src/error.rs
- claudine/lib/src/events/config.rs
- claudine/lib/src/invocation_context.rs
- claudine/lib/src/messaging/config.rs
- claudine/lib/src/signals/harvest.rs
- claudine/lib/src/steering/automatic.rs
- claudine/lib/src/steering/automatic/tests.rs
- claudine/lib/src/system_prompt/prepare.rs
- claudine/lib/tests/config/repository-control.yaml
- claudine/lib/tests/config/user-control.json5
- claudine/lib/tests/config/user-control.yaml
- claudine/lib/tests/l1/claudine_schemas_embedded.rs
- claudine/lib/tests/l1/config_json_conversion.rs
- claudine/lib/tests/l1/config_schema_matrix.rs
- claudine/lib/tests/l1/main.rs
- claudine/lib/tests/l1/path_lookup_guard.rs
- claudine/schemas/claudine-config.yaml
- darkmatter/cli/src/commands/compose.rs
- darkmatter/cli/src/commands/graph.rs
- darkmatter/cli/src/commands/validate.rs
- darkmatter/cli/src/main.rs
- darkmatter/cli/src/output/command.rs
- darkmatter/cli/src/output/mod.rs
- darkmatter/cli/src/output/report.rs
- darkmatter/cli/src/request.rs
- darkmatter/cli/tests/common/fixture.rs
- darkmatter/cli/tests/l1/config_notices.rs
- darkmatter/cli/tests/l1/main.rs
- darkmatter/lib/generated/shipped.json
- darkmatter/lib/src/config/load.rs
- darkmatter/lib/src/config/load/entries.rs
- darkmatter/lib/src/config/load/error.rs
- darkmatter/lib/src/config/mod.rs
- darkmatter/lib/src/config/schema.rs
- darkmatter/lib/src/lifecycle/executor.rs
- darkmatter/lib/src/lifecycle/executor/shell.rs
- darkmatter/lib/src/lifecycle/executor/tests.rs
- darkmatter/lib/src/lifecycle/md_host.rs
- darkmatter/lib/src/markdown/compose/context/capture/mod.rs
- darkmatter/lib/src/markdown/compose/context/capture/observations.rs
- darkmatter/lib/src/markdown/compose/context/capture/snapshot.rs
- darkmatter/lib/src/markdown/compose/context/options.rs
- darkmatter/lib/src/markdown/compose/context/repository_scope.rs
- darkmatter/lib/src/markdown/compose/context/request.rs
- darkmatter/lib/src/markdown/compose/context/runtime.rs
- darkmatter/lib/src/markdown/compose/mod.rs
- darkmatter/lib/src/markdown/compose/report_diagnostics.rs
- darkmatter/lib/src/markdown/compose/shell_expansion/store.rs
- darkmatter/lib/src/markdown/schemas/embedded.rs
- darkmatter/lib/src/markdown/schemas/shipped.rs
- darkmatter/lib/src/yaml_edit.rs
- darkmatter/lib/tests/l1/config_fields.rs
- darkmatter/lib/tests/l1/config_load.rs
- darkmatter/lib/tests/l1/context_construction_guard.rs
- darkmatter/lib/tests/l1/empty_package_area.rs
- darkmatter/lib/tests/l1/expression_recovery.rs
- darkmatter/lib/tests/l1/file_tree_roots.rs
- darkmatter/lib/tests/l1/main.rs
- darkmatter/lib/tests/l1/path_function_contracts.rs
- darkmatter/lib/tests/l1/reference_integration.rs
- darkmatter/lib/tests/l1/request_config.rs
- darkmatter/lib/tests/l1/request_context_builder.rs
- darkmatter/lib/tests/l1/schemas_union_postfix.rs
- darkmatter/lib/tests/l1/unknown_identifier_warning.rs
- darkmatter/lib/tests/l1/yaml_edit.rs
- darkmatter/lib/tests/request_support/mod.rs
- darkmatter/schemas/darkmatter-config.yaml
- simplified-schema/lib/src/embedded.rs
- simplified-schema/lib/src/simplified/grammar.rs
- simplified-schema/lib/src/simplified/serialize.rs
- simplified-schema/lib/src/triggers/assemble.rs
- simplified-schema/lib/src/triggers/discovery.rs
- sniff/lib/src/filesystem/mod.rs
docs_updated_during_phase_3:
- biscuit-file/docs/topics/file-references.md
- claudine/README.md
- claudine/docs/pipeline.md
- claudine/docs/topics/automatic-steering.md
- claudine/docs/topics/completions/shell-completions.md
- claudine/docs/topics/composition.md
- claudine/docs/topics/configuring-actions.md
- claudine/docs/topics/execution-flow.md
- claudine/docs/topics/file-referencing.md
- claudine/docs/topics/flow-control/lifecycle.md
- claudine/docs/topics/protect-service.md
- claudine/schemas/README.md
- darkmatter/README.md
- darkmatter/docs/cli/data-contracts.md
- darkmatter/docs/cli/output-and-width.md
- darkmatter/docs/topics/compose-requests.md
- darkmatter/docs/topics/yaml-editing.md
- simplified-schema/docs/topics/language-core.md
- simplified-schema/docs/topics/language/objects-and-named-types.md
- simplified-schema/docs/topics/language/types.md
- simplified-schema/docs/topics/language/unions.md
docs_created_during_phase_3:
- claudine/docs/topics/configuration.md
- darkmatter/docs/topics/configuration.md
skills_files_updated_during_phase_3:
- .claude/skills/claudine/SKILL.md
- .claude/skills/claudine/architecture.md
- .claude/skills/claudine/cli-reference.md
- .claude/skills/claudine/hook-actions.md
- .claude/skills/claudine/unified-hooks.md
- .claude/skills/darkmatter/SKILL.md
- .claude/skills/darkmatter/frontmatter.md
- .claude/skills/darkmatter/library-surfaces.md
- .claude/skills/darkmatter/requests.md
- .claude/skills/simplified-schema/SKILL.md
---

# Configuration, permissions, and host registration implementation plan

## Summary and Definition of Done

Implement `2026-11-12-registration` across Simplified Schema, Darkmatter, DMLS,
Claudine, Biscuit File, Sniff, and this repository's CI. First move schema-kind
infrastructure to Simplified Schema. Build shared default processing and safe
plain-YAML edits, then configuration loading, permissions, passive host
registration, editor integration, Claudine publication, and the CI widening
check. Keep executable host behavior under the executing program's control.

Implementation is complete and ready for review when:

- Both `.darkmatter.yaml` scopes and both `.claudine/config.yaml` scopes have
  shipped, closed schemas, located errors, schema-derived defaults, and the
  documented scope rules. `md init`, Claudine setup, migration, and approval
  persistence preserve text outside their owned edits and survive interruption.
- One captured configuration and repository revision determines a request's
  permissions. Repository allows intersect HEAD and working-copy decisions;
  scopes then combine by union with exclusions from either scope. Network
  uses the more restrictive setting, and the built-in shell blacklist wins
  over every approval path.
- Required defaults work through the public Simplified Schema API and the
  normal Darkmatter/Claudine composition routes. Strict validation remains
  read-only; soft validation and composition produce complete effective
  values. Editor typing recognizes required defaults without executing them.
- Registration supplies only passive schema, global, event, and function
  information. The motivating Claudine document works outside `claudine/`;
  shared events alone do not identify Claudine. `md compose` composes foreign
  documents with the prescribed notice and runs only its shared events.
- DMLS selects both programs' configuration schemas, observes configuration
  and registration dependencies, and invalidates stale facts after edits,
  creation, deletion, and atomic replacement. It never migrates, prompts,
  publishes a host revision, or executes configuration or registration data.
- Claudine installs immutable, content-identified host revisions from its
  embedded texts and publishes its stable manifest last. Its runtime keeps
  using its in-process registry and embedded bundle.
- A trusted CI checker detects potential permission widening without running
  candidate code. The workflow conditionally requests the `permission-widening`
  deployment environment and feeds ordinary job results into `ci-gate`.
  A tested environment-configuration script and exact activation instructions
  are delivered for final review; no live approval or repository-settings
  mutation is required to execute this plan.
- Focused tests and the affected areas' canonical L1/lint checks pass. Minimal
  Simplified Schema builds remain supported. Cross-platform behavioral
  evidence covers the new filesystem, Git, and persistence boundaries on
  macOS, Linux, native Windows, and WSL2. Current docs, skills, ownership
  guards, and the regenerated artifact chain match the implementation.

All sessions are non-interactive. Execute every phase without a human gate.
A wave completes before the next begins. Tasks in a wave marked **parallel**
may use separate subagents with the listed file ownership; pass the
non-interactive constraint to each. The coordinating session owns shared
module declarations, manifests, generated artifacts, and integration edits.
Keep all changes in this session's starting worktree and branch. Do not
commit, push, activate a deployment, or move the feature to `_completed`.
Update the existing implementation log with completed work, validation, and
concrete departures from the spec; preserve its existing content.

## Phase 1: Move schema-kind ownership

### Necessary Rules

These are settled implementation decisions. Paths below are repository-relative;
new paths are explicitly called new. The dated specification remains a snapshot.

1. **Move the language infrastructure, retain the Markdown adapter.** Move
   `markdown/schemas/triggers/` and `roots.rs` to new
   `simplified-schema/lib/src/triggers/` and `roots.rs`. Move `KindRegistry`,
   kind selection, admitted whole-YAML loading/positions, and schema-kind
   semantic checks from `yaml_documents/` into new
   `simplified-schema/lib/src/yaml_documents/`. Keep
   `DarkmatterSchemas::validate_yaml_document`, `YamlValidation::effective`,
   and their `EffectiveSchema`/`ValidationReport` adaptation in Darkmatter.
   Move `EmbeddedSchemaBundle` into `simplified_schema::embedded`; Darkmatter's
   shipped catalog/text table stays in Darkmatter. Evidence:
   `yaml_documents/mod.rs` mixes host-independent loading/selection with
   `impl DarkmatterSchemas`, while `registry.rs` itself needs no Markdown.

2. **Supply host sources explicitly.** The moved trigger assembly, kind
   registry, and bundle preparation take `ShippedSources`, supplied shipped
   kind definitions, and the existing validator-cache/preparation inputs.
   Darkmatter passes `embedded::SHIPPED_SOURCES`; text-only callers pass
   `ShippedSources::NONE` or supplied texts. Move
   `darkmatter/schemas/schema-trigger.yaml` to
   `simplified-schema/schemas/schema-trigger.yaml` and add it to
   `envelope::ENVELOPE_DEFINITIONS`. Do not leave a duplicate implementation
   or definition at the old path. Update workspace imports directly; retain
   Darkmatter wrappers only for adding its shipped sources or adapting a
   Markdown result. Evidence: `triggers/assemble.rs` currently reaches
   Darkmatter's resolver wrappers/cache, `yaml_documents/semantic.rs` embeds
   Darkmatter's sources, and Simplified Schema already accepts `ShippedSources`.

3. **Preserve ownership-move behavior and feature boundaries.** Existing
   trigger grammar, match semantics, discovery order, shadowing, diagnostic
   codes, prepared payload meaning, and relative-import anchors remain.
   Gate filesystem enumeration, context-sensitive matching, and repository
   discovery behind Simplified Schema's `file` feature. Passive envelope
   parsing, lint, supplied-source preparation, and identification proof stay
   usable without filesystem discovery. Move the trigger/YAML work counters
   to Simplified Schema and fold them into Darkmatter's snapshot. Artifact
   payload meaning stays the same, although paths, ownership records, headers,
   and bytes necessarily change. Bump all three owners' preparation revisions
   for this move and regenerate together. Evidence: the three `shipped.rs` /
   `shipped/registry.rs` owners form a dependent artifact chain; the minimal
   feature contract excludes `sniff`, `gix`, and `walkdir`.

4. **Use one default engine.** Add new
   `simplified-schema/lib/src/defaults.rs`, consumed by `Prepared` and
   Darkmatter's `EffectiveSchema`, rather than routing Darkmatter through
   a second schema object or duplicating the algorithm. Expose
   `AppliedDefault { path: PropertyPath, value: Value }` and
   `DefaultPreparation<'input> { candidate: Cow<'input, Value>,
   defaults: Vec<AppliedDefault> }` result. Name the shared entry point
   `defaults::prepare_defaults`; pass the already-resolved schema, existing
   nested-template policy, and validator context explicitly, never recapture
   them. The implementation remains a single shared candidate walk.
   Keep JSON Schema `required` assertions in exports; validate a temporary
   complete candidate internally even for strict validation, returning the
   original borrowed value in strict success. Evidence: `Prepared::judge`
   and `EffectiveSchema::validate_instance` both use `CompiledValidators`
   independently; `validation_contract_parity.rs` pins their agreement.

5. **Default semantics are presence-sensitive and deterministic.** Fill only
   absent required properties with resolved defaults. Explicit null is
   present; nullable properties retain their existing optional semantics;
   optional defaults remain metadata. Process parent values before children,
   object properties in schema declaration order, and array indexes ascending;
   deduplicate paths by first occurrence. Validate defaults strictly against
   the complete contract without I/O or coercion. Report invalid defaults
   as located `SchemaError`s at the default declaration, and conflicting
   successful union candidates as a located ambiguity error. Judge each
   union's complete candidate independently, including discriminants and
   sibling assertions; collapse equal candidates. Reject recursive default
   expansion that repeats the same unresolved schema/value construction.
   Pending schema slots remain pending and never supply a placeholder default.
   Evidence: spec 3.2; `prepared::wrapped_root_arms` preserves root assertions,
   and `simplified/convert.rs::union_arm_fragments` already rejects conflicting
   property-level default annotations.

6. **Make changed-result reporting honest.** Add `ValidationSuccess::defaults`
   and `ValidationReport::defaults`. Rename `ValidationSuccess::is_converted`
   to `is_modified`: `Cow::Owned` now means coercion, default insertion, or
   both. Update every current call in `simplified-schema/lib/examples/validate_person.rs`,
   `tests/l1/{validation_contract,declaration_narrowing,envelope_admission,root_union_assertions}.rs`,
   and `darkmatter/lib/tests/l1/validation_contract_parity.rs`. Strict success
   remains borrowed even with defaults; soft success is borrowed only when
   no edit occurred. Propagate defaults through all report constructors and
   serializers. Evidence: repository search finds the sole success constructor
   in `Prepared::accepted` and the existing method checks only `Cow::Owned`;
   Claudine and DMLS consume Darkmatter reports rather than constructing
   `ValidationSuccess` directly.

7. **Insert defaults at the existing schema boundary.** In
   `compose/schema_validation.rs::run_with_registry`, apply reported defaults
   to effective frontmatter, revalidate the complete candidate, then continue
   frontmatter shell expansion, pass-2 interpolation, and body stages.
   Defaults inserted into a document never edit its source file. Preserve
   pending/deferred operands, caller origins, trigger re-evaluation, and
   schema-default provenance. Update `expression/scope/{projection,shape}.rs`
   (`DeclaredTypes::for_schema` and property presence) alongside writeback.
   Update Claudine's
   `composition/schema/{mod,classify,translate}.rs` so default-backed absence
   never enters interactive collection. Update expression declaration typing
   and DMLS analysis so required resolved defaults are present. Evidence:
   `run_with_registry` already owns coercion and writeback; Claudine's
   `categorize_problems` derives prompts from missing reports.

8. **Create one plain-YAML editing API.** Add new
   `darkmatter/lib/src/yaml_edit.rs` with
   `YamlEdit::{Set { path: PropertyPath, value: Value }, Remove { path: PropertyPath }}`
   and `edit_yaml_paths(source, edits) -> Result<String, YamlEditError>`.
   Extract the reusable node-range edit engine from
   `markdown/hash/path_edit.rs`/`write.rs`; let the Markdown adapter supply
   its envelope span and the YAML API use the actual whole document span.
   Support missing mapping parents, insertion into empty mappings, list
   replacement/append, BOM, LF/CRLF, and no terminal newline. Verify the
   reparsed value and every unnamed value. Refuse unsupported alias/tag/flow
   edits with a located error; never fall back to whole-file serialization
   for configuration writes. Evidence: `edit_frontmatter_paths` currently
   calls the Markdown loader and `edited_document_text` deliberately has a
   serialization fallback that would violate spec 2.8.

9. **Use shared locked transactions without a new dependency.** New
   `darkmatter::config::write` owns `ConfigTransaction`: resolve the real
   target, acquire a stable sibling lock with `std::fs::File::try_lock`, and
   hold it across re-read, edit, validation, source-byte comparison, and
   replacement. Use a 2-second acquisition deadline with 20-ms backoff,
   returning a typed lock error on expiry. Lock files are retained, never
   unlinked. For a missing target, canonicalize the nearest existing ancestor
   and append the remaining native components. Compare identities through
   `PathIdentity`; order multi-target locks by portable canonical path text.
   Reuse `NamedTempFile::persist` and the bounded Windows replacement behavior
   already implemented in `effects/fs_write.rs`; extract that replacement
   helper within Darkmatter for the writer's use. Preserve existing file
   permissions, sync the temp file, and clean it on failure. An external-byte
   change aborts that save without retry; current-run approval survives.
   Cooperating writers serialize; no claim is made about an uncooperative
   editor's final check/replace race. Evidence: Rust is pinned to 1.98.1,
   existing code uses standard file locks, and Claudine's `atomic_write`
   guarantees complete payloads but does not merge concurrent edits.

10. **Own configuration in Darkmatter, not composition internals.** Add new
    public `darkmatter::config` modules `schema`, `load`, `permissions`,
    `write`, `migration`, `init`, and `registration`. `ConfigSnapshot` holds
    user/repository raw observations, independent per-setting validation
    results, dependency observations, and diagnostics. Its loading entry point
    is `ConfigSnapshot::load(context: &FileResolutionContext,
    repository: Option<&CommittedTreeSnapshot>, mode: ConfigLoadMode)
    -> Result<Self, ConfigError>`. A repository-naming context requires the
    matching snapshot; a context without a repository passes None. Its
    permission accessor returns `Result<&EffectivePermissions, ConfigError>`; registration loading
    returns valid registrations plus findings. Whole-document syntax or an
    unknown root key fails loading; a well-formed invalid `permissions` blocks
    permission consumers, while invalid `register` entries warn and omit only
    those registrations. Never recover an invalid permission with defaults.
    Evidence: spec 1 separates top-level failures; existing `MdRequest` and
    `ComposeRequest` already centralize captured request inputs.

11. **Use one schema identity for permissions.** Put canonical named types
    in `darkmatter/schemas/darkmatter-config.yaml`; export them through the
    existing `darkmatter:` shipped namespace as `darkmatter:config` by adding
    a shipped alias for that document, rather than copying its text. The
    canonical host import is `permissions@darkmatter:config` (the spec's two
    example spellings are reconciled to this one). Export `user-config`,
    `repository-config`, `permissions`, `user-permissions`, and
    `repository-permissions`. Shared `permissions` defines syntax and field
    types; scope projections attach scope-specific file-read defaults. All
    persisted lists are required-with-default, non-null arrays. `permissions`
    itself is a required-with-default mapping-or-local-reference; `register`
    defaults to `[]`. No loader hard-codes the eleven extension patterns,
    shell empty list, or network default. Evidence: spec 1, 2.1, 2.8, and 9;
    `ShippedSources` currently resolves stems under `schemas/partials`, so
    add `ShippedSources::with_aliases(&[(&str, &str)])`, using static alias/path
    pairs, and supply `config` → `darkmatter/schemas/darkmatter-config.yaml`.
    Include alias mappings in preparation identity/freshness inputs; other
    reserved stems retain their existing meaning.

12. **Capture configuration and HEAD once, with typed Git errors.** Add
    `GitRepo::committed_tree() -> Result<CommittedTreeSnapshot>` in new
    `sniff/lib/src/filesystem/git/committed_tree.rs`. The Send + Sync snapshot
    pins the tree and commit IDs using the already-discovered gix handle and
    owns thread-safe object access. Expose `commit_id()`,
    `read_file(repo_relative_path)`, and `paths()` for the checker; return
    explicit absent/blob/symlink outcomes and errors, never run filters,
    external helpers, hooks, or Git subprocesses. An unborn HEAD is a valid
    empty committed view; corruption/read failures are errors.
    Retain this snapshot in `RepositoryObservation` while
    `ContextCapture::new` still has `git_handle`, preserve it through
    `CapturedObservations::with_repository`, and include it in supplied
    `ContextCaptureEvidence`. Do not use `GitRepo::head_id()` for policy: it
    calls `.ok()` and conflates errors with absence. Evidence:
    `repository_scope.rs::RepositoryObservation` currently holds only root
    and topology; `ContextCapture::new` has the sole discovery handle.

13. **Share snapshots across all request derivations.** Add
    `ComposeOptions::with_config_snapshot(Arc<ConfigSnapshot>)`; preparing a
    request without a supplied snapshot loads passively after its context
    and repository observation exist. Add an explicit `ConfigLoadMode`
    (`Passive`, `MigrateLegacy`) to the public configuration loader; only
    writing CLI startup uses `MigrateLegacy`, before the snapshot is finalized.
    `MdRequest` caches the snapshot by captured source-repository identity,
    sharing user bytes across its source requests; Claudine invocation context
    and DMLS document context supply the same public snapshot. A caller of
    `ComposeRequest::with_context` supplies its captured repository policy
    snapshot when the context names a repository; omission is a typed context
    error, never rediscovery or disabled enforcement. Update its workspace
    callers and test request helpers. Child composition keeps captured
    permissions and current-run grants; it derives only the reading document's
    path context. Standalone passive validators accept a supplied snapshot
    and otherwise load passively from their supplied context through the same
    preparation API. Evidence: `ComposeRequest::{prepare,with_context,derive}`,
    `MdRequest::document_context`, and the darkmatter request-context contract.

14. **Evaluate permission predicates, not textual entry intersections.**
    Define `PermissionVerdict::{Preapproved, ApprovalRequired}` and typed
    errors; explicit run denial remains in the approval state, and the built-in
    shell refusal remains a separate terminal verdict. For each file/command,
    repository pre-approval requires both HEAD and worktree positives and no
    exclusion in either. Combine that contribution with the user positive by
    OR; an exclusion from the user, HEAD, or worktree removes pre-approval
    globally. Current-run approval may override exclusions, never the built-in
    blacklist. Network is AND across HEAD/worktree and then applicable scopes;
    per-run exact hosts are added afterward. Missing scope files contribute
    their schema defaults; a missing explicit reference target is an error.
    Evidence: spec 2.4–2.6 explicitly defines semantic intersection and union.

15. **Keep path grammar and committed reads separate from disk probing.**
    Validate file-read entries with `GlobReference` plus the portable-variable
    catalog, reject absolute spellings on every OS, and judge relative globs
    in the document doing the read. Resolve configuration/manifest references
    with `FileReference` anchored to the file declaring them. Use
    `FileReference::candidate_plan` against the pinned committed view instead
    of `resolve_detailed_with_presence`: the latter probes disk first and
    would mix revisions. Repository reference candidates must remain inside
    that repository; traverse committed symlink blobs within that same tree,
    with cycle detection and a 40-link limit, rejecting escapes. For worktree
    paths use `canonicalize_simplified` and `PathIdentity`, checking existing
    ancestors for missing probes. Apply physical containment at actual read
    time too; an out-of-tree symlink never becomes a repo allow. References
    use ordinary local candidate forms; recursive search uses the committed
    path listing, never the worktree. Evidence: spec 2.1/2.6;
    `candidate_plan` is documented as filesystem-free, whereas presence
    resolution consults disk. Scope defaults and portability checks do not
    use process environment or hand-written sigil parsing.

16. **Do not require a folder grant for listings.** Extend the existing
    `expression::read_gate::ReadGate` with a listing-result admission hook;
    gate each returned file before exposing it to a schema slot, and drop
    uncovered files without prompting for the containing directory. Keep
    canonical containment and per-file observation checks, including before
    a data memo hit or schema import. `file_exists` is a file probe requiring
    the same file decision as content reads. Remove `grants/auto.rs`, active
    grant-store loading, directory grants, and persistent deny choices; retain
    only the current-run approval state needed by slot preflight and runtime.
    Evidence: spec 2.1/2.8 and `ReadGate::check`, which currently asks both
    before and after listing and cannot yet filter a returned set.

17. **Network allow is a FetchPolicy mode.** Add
    `biscuit_file::file_reference::fetch::FetchPolicy::allow_all()` backed by
    an explicit blanket-allow flag; retain deny-all as Default. Project the
    effective policy into both `RemoteFetchRuntime` and the slot session's
    runtime. Keep HTTP(S)-only transport, redirect checks, cache consent checks,
    and existing cache-control rules. Never emulate blanket access with a
    wildcard hostname. Evidence: `fetch.rs::FetchPolicy` currently contains
    only `Vec<HostPattern>`, and `Wildcard` matches subdomains rather than
    every host; spec 2.3 changes consent without changing safeguards.

18. **Retire the policy-root and blacklist APIs.** Replace
    `ShellExpansionOptions::policy_root`, `ShellPolicyPaths`,
    `with_shell_policy_root`, and Claudine's equivalent policy-root options
    with the captured permission snapshot and a user-scope persistence target.
    Extract strict single-entry parsing from `shell_expansion/store.rs` while
    retaining `exact`, `prefix`, `TemplateShape`, normalization, and matching.
    Empty payloads are errors; malformed configured rules are never skipped.
    Remove user-blacklist evaluation and `BlacklistPersist`; remove that
    prompt choice rather than rebranding it as a permanent denial. Update
    `shell_expansion/{site,executor,discovery,types}.rs`, preflight, lifecycle
    shell runners, `lifecycle/md_host.rs`, CLI `approval.rs`,
    `commands/compose.rs` and `compose/lifecycle.rs`, Claudine's harness APIs,
    `wrap/harness_orch/shell_options.rs`, and `system_prompt/prepare.rs`.
    Preserve graph membership, late-value/template checks, and builtin
    blacklist checks ahead of yolo/cached approvals. Evidence: these callers
    currently load the same rule store through different roots; spec 2.2/8.

19. **Persist exact user approvals without changing the captured policy.**
    Follow the user reference, materialize an omitted list's schema defaults,
    append/deduplicate the exact portable escaped file or approved shell rule,
    validate permissions, and save through `ConfigTransaction`. Do not save
    directory or site grants, widen `network`, remove exclusions, or edit repo
    policy. URL prompts offer per-run consent only; blanket network consent
    is configured explicitly. A still-matching exclusion or failed save emits
    a structured not-saved diagnostic while retaining the run's approval.
    Evidence: spec 2.8 requires exact portable file approvals and states that
    current-run exceptions do not reload configuration.

20. **Migrations are temporary writers with resumable receipts.** Keep all
    legacy reading in `config/migration.rs` and Claudine's temporary JSON5
    converter. Missing matching properties trigger migration; authored empty
    values do not. Convert whitelist and blacklist together, defaulting an
    omitted file-read list before adding migrated entries. Convert under-home
    allow-file entries to exact escaped `~/` references and allow-dir to
    `~/<dir>/**`; deny entries become equivalent exclusions. Report dropped
    outside-home/URL/site lines and changed blacklist semantics. Invalid
    legacy line syntax is a located migration error and retains the whole
    source. Store a content-addressed pending cleanup receipt outside the
    closed configuration document, beside its writer lock; save/re-read the
    new config before deleting sources whose bytes still equal the receipt.
    A later load completes only that recorded cleanup, never assumes an
    authored property was migrated. Remove the receipt after cleanup. This
    prevents a crash after save from suppressing legacy deletion forever.
    DMLS/passive loads report the repair command and never migrate. Evidence:
    spec 5 requires both deletion-after-verification and idempotence across
    interruption; property presence alone cannot distinguish that crash.

21. **Preserve Claudine configuration semantics, including inheritance.**
    Ship `claudine/schemas/claudine-config.yaml` exporting closed user and
    repository shapes; repository accepts only its existing override keys
    plus permissions. Import the one Darkmatter permissions type, applying
    its scope projection at the loader. Keep all current field names,
    provider/action variants and accepted aliases, nullable optional fields,
    merge rules, and reader-specific scope behavior. Encode current defaults
    in the schema; make typed configuration construction consume the schema
    projection before serde, with no duplicate file-loader defaults.
    Repository inheritance fields remain optional and are not inserted;
    their declared defaults describe inherited behavior. In particular,
    absent `active_messenger` inherits and explicit null disables, and
    `steering` null remains invalid. Keep runtime-only policy constants such
    as the bash action timeout outside this change. Evidence:
    `ClaudineConfig`, `RepoOverrideConfig`, `config/merge.rs`, and the
    `2026-09-30-configuration` inventory; spec 9 changes format, not scope.

22. **Replace all whole-config saves with explicit edits.** Replace
    `save_claudine_config` and `save_repo_override_config` for existing files
    with per-path change sets through Darkmatter's transaction API. The TUI
    keeps an opening baseline and computes recursively changed leaves,
    replacing a changed sequence as one owned node; apply only these edits
    to the freshly read file. Programmatic setters supply their one path;
    init supplies its managed paths. Fresh-file creation uses the schema
    projection plus its explicit initialization values. Migrate actual
    Claudine paths in `dispatch/loader.rs`, `wrap/runaway_guard.rs`, init,
    `init_wizard.rs`, `config_tui/`, `link_display.rs`, uninstall, main's
    init check, and completion. Remove the unread `config.json5` completion
    candidate. Do not change providers' own `config.json` files or
    `.docker/config.json`. Evidence: current TUI saves the entire struct,
    and the repository search distinguishes these files from provider config.

23. **Setup is ordered and non-destructive.** `md init` creates the user file;
    `--repo` additionally creates the captured repository file and fails
    before any write outside a repository. `--inline` applies to scopes this
    invocation initializes, preserving a readable referenced block or using
    explicitly reported schema defaults for an unreadable one. Ordinary init
    preserves existing values and references. Claudine transfers inline
    permissions only when its own block is absent or semantically equal;
    differing authored blocks leave both untouched and return a located
    conflict. Take target locks in the same order, write Claudine first,
    then the reference, keeping the original inline policy active on failure.
    An arbitrary existing third-party reference remains in place: Claudine
    does not seize it. Register Claudine once by manifest identity after its
    manifest is published. Evidence: spec 2.9; preserving existing authored
    references follows its idempotence requirement.

24. **Use a closed v1 manifest and all-or-nothing profiles.** New
    `darkmatter/schemas/darkmatter-host.yaml` defines exactly `kind`,
    `format_version`, `host`, `version`, `identify`, `triggers`, `globals`,
    `lifecycle`, and `functions`. Require every field, including explicit
    `functions: []`; allow empty trigger/function lists. Require literal
    kind `darkmatter-host`, integer version 1, nonempty string release and
    host ID, and local nonempty references. Each function item requires only
    `catalog` and `facts`. Load every member and transitive dependency once
    into a retained read set before publishing a `RegisteredHostProfile`.
    Reuse `GlobalCatalog::from_host_file`, `HostLifecycleProfile` loading,
    and `HostFunctions::analysis_only`; add explicit companion-path inputs
    so a manifest can name the artifact rather than relying on its stem.
    Invalid/missing artifacts refuse the registration as a whole. Validate
    the manifest host ID against the lifecycle profile's host name. No handler
    registry is installed. Evidence: spec 6 and existing strict catalog/hash
    readers in `catalog.rs`, `lifecycle/host_profile.rs`, and
    `expression/host/artifact.rs`.

25. **Prove identifying breadth structurally.** Add
    `simplified_schema::triggers::identify::{admit_identifying_trigger, identifies}`
    with a caller-supplied set of md-owned top-level names. A positive
    required non-null property atom on a name outside that set supplies a
    witness. `All` needs one witnessing child; every `Group` alternative
    must witness; `MinMatch(n, children)` witnesses only when n is greater
    than the number of children without witnesses. `None` and `$path`
    supply no witness. An empty or unprovable expression is refused with
    `dm.host.identify_too_broad`. For identification only, each positive
    required property atom tests non-null presence regardless of its declared
    value type; normal trigger matching remains unchanged. Generate Claudine's
    identify trigger as a group of the nine specified keys, each with a
    required non-null atom; its payload is `{}`. Pass the names from the
    resolved Darkmatter fm baseline plus md events/globals, not a duplicate
    hard-coded list. Evidence: `MatchExpr` has exactly All/Group/None/MinMatch/
    Property/Path, and normal `property_matches` includes type checks that
    would otherwise lose malformed host documents.

26. **Select once and layer registered sources separately.** New
    `config::registration::HostSelection` has `Md`, `Registered(profile)`,
    `Legacy(profile)`, and `Ambiguous(hosts)` results. Concatenate user then
    repository entries,
    deduplicate manifest identities, and compare distinct same-host claims
    by ordered manifest meaning plus member/dependency content digests.
    Equal claims collapse; conflicting claims are refused with
    `dm.host.registration` and cannot win selection. A document identified by
    a conflicting claim gets the internal Ambiguous refusal and no legacy
    fallback; same-host conflicts use the registration code rather than
    claiming two different hosts matched. Multiple matching valid host IDs
    produce `dm.host.ambiguous` and md analysis with no legacy fallback.
    A single match supplies all host catalogs, functions, lifecycle, and
    matching triggers together. Only the no-match case uses legacy
    root-catalog fallback, through a central Legacy view preserving that
    fallback's existing globals/functions and first usable lifecycle profile.
    A known installed host whose identifying trigger did not match is excluded
    from legacy fallback too: otherwise its root catalog would immediately
    undo identification and make shared-only/unrelated documents foreign.
    Other unregistered root hosts retain compatibility fallback. Layer
    baseline, registered triggers in manifest order, normal roots, then document schema. Deduplicate an
    identical root/registered trigger by resolved identity and content,
    retaining its registered occurrence; unrelated same-name files remain
    distinct. Extend origins/layer identities with the registered manifest
    identity and content revision; keep definition/import anchors intact.
    Evidence: spec 6; `LifecycleHost::first_usable` and providers' independent
    `host_catalogs`/`host_function_catalogs` currently select separately.
    The same-host fallback suppression follows spec 6's requirement that
    registered globals/events apply only to identified documents; record
    this precise reading of its broad fallback wording in the implementation
    log and test it inside the Claudine package area as well as outside it.

27. **Analysis profiles never replace runtime authority.** Library
    composition gets a passive selected-host view for validation/typing.
    `md` continues using `md_host` bindings and parse rules for only
    `initialize`, `cancelled`, and `finalize` on a registered foreign document;
    build an execution-only frontmatter projection omitting foreign events
    and md's renamed `composed`/`failed` keys. Do not feed the full foreign
    schema into a generated md `composed` event. Shared expressions demanding
    unavailable host globals fail normally. Emit `dm.host.foreign_document`
    once per root request. Claudine's `claudine_registry()` and embedded
    runtime options remain authoritative and opt out of passive replacement.
    Evidence: spec's explicit `md compose` contract and the existing
    `commands/compose/lifecycle.rs` runtime adapter.

28. **DMLS consumes one selected view and rejects stale revisions.** Add
    configuration/registration dependencies to its preparation/install model;
    include revisions in `AnalysisIdentity` and document analysis snapshots.
    Route lifecycle parse, globals, function facts, diagnostics, completion,
    and hover through the same selection. Extend existing parent-directory
    watches to missing configs, references, manifests, schema imports, and
    artifacts, including JSON files; observe HEAD changes for the commit
    rule. On invalid replacement invalidate the old host view and report the
    new failure, never silently serve its last-good facts. An unsaved config
    buffer is validation/completion input and may narrow permission analysis;
    it never authorizes effects or a committed widening. Emit
    `dm.lifecycle.unknown_host` only for a known host-only event from a loaded
    installed profile whose identification failed, naming that host. Unrelated
    application keys remain data. Evidence: `TriggerRegistries` already tracks
    dependencies and generation numbers, `workspace/watch.rs` already watches
    missing paths from parents, and `LifecycleHost::for_document` currently
    bypasses that cache to scan root catalogs.

29. **Publish complete immutable Claudine revisions.** New
    `claudine/lib/src/composition/registration.rs` publishes under
    `~/.claudine/darkmatter-host/<version>-<content-digest>/schemas/` and
    atomically replaces `~/.claudine/darkmatter-host.yaml` last. Compute
    `biscuit_hash::blake3_hash_json` over ordered role metadata and a canonical
    map of portable relative filenames to exact embedded texts, including
    all trigger libraries, globals, lifecycle artifact, config schema, and
    identify trigger. Keep `functions: []` until Claudine ships a function
    catalog. Validate an existing revision's bytes before reuse; publish via
    a sibling staging directory, never mutate a live revision. Retain all
    old revisions; no cleanup is added. Refresh on substantive CLI launches
    after help/completion early exits, without writing `.darkmatter.yaml`.
    Refresh failure warns and keeps the last complete revision; init failure
    returns an error. Evidence: `composition/schema/bundle.rs` embeds eight
    schema texts, `globals.rs` embeds the globals, and `shipped.rs` provides
    their prepared/lifecycle data; spec 6 requires immutable publication.

30. **Diagnostics use existing result channels.** Configuration load errors
    use `dm.config.invalid`, reference failures use `dm.config.reference`,
    persistence failures use `dm.config.not_saved`, and temporary conversions
    use `dm.config.migration`; retain every code/severity specified in the
    spec, including the host codes and `dm.config.missing`. Carry both source
    and target locations for references. Integrate with `CommandRun`,
    `ReportSink`, and library diagnostic projection instead of tracing-only
    reporting. Emit missing-user-config once per invocation, including
    non-composition md commands after help/completion; DMLS sends one session
    notification. Repository absence is normal. Formatting uses
    `TerminalRenderable` components; data outputs keep stderr empty.
    Evidence: `output/{command,report}.rs` already has these envelopes and
    deduplication; spec 4/5/6 requires notices and warnings there.

31. **Deliver the CI gate without a human implementation checkpoint.** Use
    a trusted base checkout for the checker and its Cargo.lock/dependencies;
    candidate trees are data only. Add `permission-scope` and
    `permission-widening` jobs to `ci.yml`; the first classifies, the second
    references the deployment environment only for potential widening.
    Extend `ci-gate`'s needs/result list only. Missing trusted checker during
    bootstrap is conservatively potential widening, never success. The
    environment settings are an author-owned final activation artifact,
    matching spec 2.7/Phase 7; do not create or approve an environment during
    implementation. Deliver a dry-run-by-default script targeting
    `yankeeinlondon/rusty-biscuit`, reviewer `User:980757` (`yankeeinlondon`),
    `prevent_self_review: false`, zero wait timer, and no admin bypass.
    The live GET at planning time returned no environments, and ruleset
    `protect-your-bacon` requires `ci-gate`. Before a widening job can be
    requested, verify the environment exists with these protections; absent
    or weaker protection fails closed with setup instructions. This avoids
    GitHub auto-creating an unprotected environment. Evidence: read-only
    repository/environment/ruleset API observations on 2026-11-13 and
    [GitHub's environment API](https://docs.github.com/en/rest/deployments/environments)
    and [review behavior](https://docs.github.com/en/actions/how-tos/deploy/configure-and-manage-deployments/review-deployments).
    Runtime deployment approval is the delivered product behavior, not an
    implementation phase gate. Live activation is assessed after all phases.

32. **No spike or unrelated cleanup.** No runtime measurement is needed:
    the spec expressly says none, and the open questions above were answered
    by source/API reads. Keep existing caching and work counters; add no
    benchmark suite or wider performance survey. Do not implement Claudine's
    separate scope/editor-coverage redesign, revamp dry run, fold in
    `.dmls.toml`, add recursive permission references, broaden network
    granularity, or add a dependency just to replace available standard APIs.
    Update behavior-changing comments/docs with their code; preserve unrelated
    local changes. Evidence: spec non-goals and verification criteria.

33. **Compile configuration contracts into DMLS without a host dependency.**
    Add new `darkmatter/dmls/src/config_contracts.rs` and embed Claudine's
    single authoritative `claudine/schemas/claudine-config.yaml` with
    `include_str!`, preparing it once from supplied text and the Darkmatter
    shipped sources. Keep its named config types in that one schema file;
    do not import runtime Claudine code or duplicate schema text. Select only
    the exact captured user/repository config paths, through `PathIdentity`,
    and use explicit configuration-contract routing before ordinary YAML
    kind/root-schema routing. Add a `Route::Configuration` adapter outcome
    for providers to distinguish this route. Configuration files have no
    mandatory `kind`, so `KindRegistry` alone cannot select them. This
    intentional compile-time source coupling lets DMLS edit a brand-new
    Claudine config even before Claudine is registered; it also works after
    installation without a checkout. Add the embedded schema to DMLS's
    freshness/wiring test inputs and preserve the single permissions import.
    Evidence: `overlay/yaml_document.rs` currently selects only kind or
    root `$schema`; neither unkinded config supplies one. The v1 host manifest
    has no config-schema member, and spec 9 requires both paths to be typed.

34. **Keep recovery receipts strict and confined.** New
    `config/migration_receipt.rs` owns the one receipt reader/writer for both
    legacy-policy migration and Claudine JSON conversion. A receipt is a
    closed JSON object with `format_version: 1`, `target` (resolved canonical
    target identity), `published_hash` (BLAKE3 of verified target bytes), and
    `sources` (a nonempty list of `{ path, hash, action }`; action is `delete`
    or `backup`, with `backup_path` required only for backup). All fields are
    required and non-null. Store it at the target's stable sibling path
    `<target-name>.darkmatter-migration.json`; take the target lock for every
    receipt operation. Before cleanup, match source/backup paths against the
    captured scope's exact known legacy filenames and the target identity,
    re-read the published target, and require its hash and all source hashes
    to match. A missing already-cleaned source completes that item; a changed
    source or destination, unreadable/malformed receipt, or unknown path
    retains source data and returns a located migration error. Write the
    receipt before publication; an unpublished target never authorizes
    cleanup. Expose receipt operations to Claudine through Darkmatter's API.
    Extract the existing strict JSON parser into new crate-private
    `darkmatter/lib/src/strict_json.rs`, retaining the lifecycle/facts
    entry points and error wording as wrappers using it. Implement the
    receipt module with the Phase 2 writer foundation, before the Phase 3
    JSON5 converter consumes it; its public cleanup API accepts the exact
    allowed legacy-source paths from the captured caller, never arbitrary
    deletion authority from a receipt.
    Evidence: both migrations require recovery across a publish/cleanup
    interruption, and `host/artifact.rs::parse_strict` already rejects
    duplicate keys and trailing content. This adds no second JSON parser.

### Wave 1 — Ownership extraction (sequential)

- [x] **Move schema core**
  - Move the files/types in Rules 1–3, adapting source/cache inputs before
    introducing registration. Keep parser/matcher and envelope admission
    available from supplied data; put disk operations behind `file`.
  - Split the existing whole-YAML adapter at the `EffectiveSchema` boundary;
    retain `DarkmatterSchemas` APIs as delegation/adaptation, with no trigger
    or kind-selection implementation left there.
  - Update imports in Darkmatter lib/CLI/DMLS, Claudine's bundle/shipped
    code, and existing tests. Move the trigger definition's compiled source
    path and all real code/document references to its new owner.
  - Complete module exports and manifest feature wiring in this task so the
    next wave works against compiling public types.

### Wave 2 — Ownership checks (parallel)

- [x] **Pin boundary tests**
  - Move/retain trigger discovery, matcher, root search, kind-selection,
    prepared-bundle memo, and YAML admission regression cases with the
    implementation. Preserve exact `md schema triggers` origins and errors.
  - Update `schema_language_ownership_guard`, dependency-direction,
    context-construction, path-lookup and glob guards in both packages.
    Add the moved owned symbols and source-input declarations, including
    `schema-trigger.yaml`'s new path.
  - Run focused nextest ownership/parity cases and Simplified Schema's
    minimal-feature checks. No filesystem dependency may enter its text-only
    dependency graph.

- [x] **Document new ownership**
  - Update Simplified Schema language-core/features/kind docs and Darkmatter's
    schema ownership docs; update `.claude/skills/{simplified-schema,darkmatter}/`
    routing and boundary descriptions. Describe the final delegation boundary.
  - Own docs/skills only in this wave; the coordinator owns code and manifests.

### Validation checkpoint

- [x] **Regenerate moved artifacts**
  - Coordinator task after Wave 2: bump preparation revisions,
    update freshness-test source inputs, and run root `just shipped-schemas`.
    Regenerate the three artifacts as one chain, not per-file hand edits.
  - Checkpoint: `just test` and `just lint` in `simplified-schema/`; focused
    Darkmatter/Claudine shipped-corpus and layering tests. Review the generated
    diff for ownership/path changes without altered schema semantics.

## Phase 2: Build defaults and YAML transaction foundations

### Wave 3 — Independent foundations (parallel)

- [x] **Implement default candidates**
  - Own Simplified Schema `defaults.rs`, `prepared.rs`, `outcome.rs`, exports,
    default/schema diagnostics, and shared-library tests. Implement Rules 4–6
    over existing resolved schema/definition data and validators.
  - Test strict borrowing with reported defaults; soft borrowing versus owned
    insertion; nested parents and arrays; explicit null; invalid default type
    and constraints; missing nested required children; resolved/pending slots;
    recursive references; union ambiguity/equal candidates/discriminants and
    sibling assertions; stable escaped property paths.
  - Update the standalone example and conversion-reporting tests. Verify
    external exported JSON Schema retains ordinary annotation semantics.

- [x] **Implement YAML edits**
  - Own `yaml_edit.rs`, extracted node edit internals, and existing Markdown
    path-editor regressions. Implement Rule 8 with no fabricated envelope.
  - Test empty roots, missing parents, nested mapping/list edits, explicit
    empty collections, comments/quotes/order outside edited nodes, BOM,
    CRLF, and no final newline. A refused alias/tag/flow mutation returns
    the original bytes unchanged.

- [x] **Capture committed trees**
  - Own new Sniff `committed_tree.rs`, its exports, and focused public tests.
    Implement Rule 12's pinned tree/object view and symlink outcomes.
  - Use library-created fixture commits; compare results to read-only Git
    commands as the repository's Git API parity pattern requires. Cover an
    unborn HEAD, deleted worktree files still present at HEAD, staged versus
    unstaged edits, linked worktrees, missing objects, and ref movement after
    capture. The snapshot must continue reading the original tree.

### Wave 4 — Foundation integration (parallel)

- [x] **Apply composition defaults**
  - Own Darkmatter `EffectiveSchema` reports, composition schema writeback,
    expression presence typing, and their tests. Consume the shared default
    engine before downstream stages; retain pending/caller/default origins.
  - Revalidate after insertion and trigger re-evaluation. Assert an inserted
    default can feed pass-2 and body interpolation without an authored-file
    edit; assert default errors never invent source positions.
  - Audit every `ValidationReport` constructor and serialization consumer,
    including no-schema paths, YAML results, and report parity tests.

- [x] **Implement locked writes**
  - Own new `config/{write,migration_receipt}.rs`, the shared atomic
    replacement helper, and `strict_json.rs`. Implement Rules 9/34 using the
    YAML editor; accept a caller-supplied passive validation callback so both
    programs share the transaction mechanism. Finish the receipt API and its
    public strict-input tests here; Phase 3's converter depends on it.
  - Test receipt publish/cleanup interruption, wrong-path refusal, invalid
    receipt shapes, hash mismatch, and completed-cleanup idempotence through
    the public transaction result, using a supplied known-source allowlist.
  - Test two processes editing different leaves through symlink aliases,
    lock deadline, deterministic multi-file lock ordering, changed input,
    unsupported edit, validation failure, replacement failure, retained
    source modes, and no temp leaks. Inject replacement failures instead
    of depending on privileged filesystem permissions.

- [x] **Update default consumers**
  - Own Claudine's missing-property classification/collection tests and DMLS
    declaration narrowing/hover tests. Required resolved defaults count as
    present; optional defaults stay metadata; invalid defaults block rather
    than becoming prompts.
  - Synchronize at the shared default-engine API established by Wave 3;
    do not edit Darkmatter's schema writeback files in this task.

### Validation checkpoint

- [x] **Verify foundations**
  - Coordinator checkpoint after Wave 4: run focused public default/parity,
    YAML edit, Sniff snapshot, and transaction tests. Run `just test` in
    Simplified Schema and Sniff, plus affected Darkmatter/Claudine L1 filters.
  - Regenerate artifacts after default preparation changes, including higher
    owners; record the changed preparation/merge identity revisions and tests.

## Phase 3: Load and type both configuration formats

### Wave 5 — Configuration contracts (parallel)

- [x] **Load Darkmatter config**
  - Own the shipped Darkmatter configuration schema/namespace alias and new
    `config/{schema,load}.rs`. Use the moved admitted-YAML loader and
    schema-only validation; never use repo triggers, slots, network, or the
    policy being loaded to admit a config.
  - Expose `ConfigSnapshot`, located per-setting outcomes, dependency records,
    and the scope projections in Rules 10–13. Materialize defaults through
    the shared engine and keep authored presence alongside effective values.
  - Validate reference syntax and field types here; actual referenced-block
    permission evaluation lands in Phase 4. Invalid references stay errors
    rather than being represented as an empty policy.

- [x] **Type Claudine config**
  - Own `claudine/schemas/claudine-config.yaml`, config schema embedding, and
    typed user/repository validation in `dispatch/loader.rs` and config types.
    Add permissions as shared Darkmatter data, never a second grammar.
  - Cover every existing top-level field: tts, messenger, logging, protect,
    actions, matchers, preferred_agent/favorite_agent, canonical_provider,
    models, default_sounds, prompt_for_missing, harvest_unmatched,
    exit_expressions, guard_settings, steering, and repo active_messenger.
    Preserve the inventory's accepted nested variants and per-field scope.
  - Move defaults for these configuration values into the shipped schema;
    apply user runtime defaults there and preserve repository omission for
    inheritance. Keep `ClaudineConfig::validate` semantic checks for sound
    names, regexes, active messenger references, and Protect rules after
    schema validation. Do not silently coerce configuration field types.

### Wave 6 — Reader and writer integration (parallel)

- [x] **Migrate Claudine YAML**
  - Own the temporary JSON5 converter and all concrete path readers listed
    in Rule 22. Run `biscuit_file::json5_scan` before conversion to reject
    duplicate decoded keys/numeric admission failures, then parse with the
    existing `Json5` loader and validate the corresponding YAML config shape.
  - JSON-only: publish and re-read valid YAML, then rename unchanged JSON
    to `config.json.bak`; emit the comments-loss notice. Both files: YAML
    wins with an ignored-JSON warning. YAML-only: no conversion. Legacy
    per-provider JSON: retain existing backup-and-ignore behavior.
  - Do not overwrite an existing backup: an identical existing backup permits
    removal of the unchanged duplicate source; a differing backup is a
    located error retaining both. Use a pending conversion receipt for the
    publish-before-rename crash window. Changed or invalid source is retained.
  - An explicitly supplied noncanonical configuration path is read at that
    exact path as YAML; only the known legacy `config.json` scope paths trigger
    automatic conversion. Rename custom-path test fixtures to `.yaml`.
  - Completion and passive tooling do not migrate. Stop invalid loads before
    dispatching any hook, agent, provider, or composition command.

- [x] **Preserve host edits**
  - Own Claudine config editor/init save paths. Replace whole-struct saves
    with Rule 22's explicit per-key edits through `ConfigTransaction`.
  - Record original TUI user/repository values and compute changed leaves at
    save; two sessions changing different keys preserve both edits and all
    unmanaged text, including permissions written by Darkmatter meanwhile.
  - Update non-interactive setters, init_wizard, and all old save-function
    callers; retain existing field coverage and interaction behavior.

- [x] **Bind request config**
  - Coordinator owns `RepositoryObservation`, capture/evidence plumbing,
    `ComposeOptions`, `ComposeRequest`, `MdRequest`, and Claudine invocation
    snapshot adapters. Implement Rules 12–13, and update all supplied-context
    callers, including tests, without recapturing CWD/repository downstream.
  - Add run-level diagnostic plumbing through `CommandRun`/`ReportSink` and
    Claudine's logger/result adapter. Missing user config emits the specified
    diagnostic once, while missing repo config remains normal.
  - Keep shell/file/network enforcement changes for Phase 4; no temporary
    permissive adapter may interpret invalid configuration as authorization.

### Input robustness matrix

Every invalid shape produces a located public error, launches nothing, and
publishes nothing. A host-registration error is the specified exception: it
warns and omits the entire affected host, leaving other valid registrations
and permissions usable. A well-formed config's permission error blocks only
consumers needing permissions, while DMLS reports all findings. Configuration
validation is strict about field types; soft coercion is for document data.

Use a real generated default/configuration fixture as each control, with
exactly one edit per negative cell. Test the public `ConfigSnapshot` accessors,
Claudine loader, registered-profile loader, and public validation outcomes;
never count a private parser assertion as the matrix result.

- **Shared validators:** test absent, explicit null, wrong whole-field type,
  and empty for scalar/reference, mapping, boolean, enum, and numeric
  validators. For sequence/dictionary validators also test one wrongly typed
  element, all wrongly typed elements, and empty. Apply the full shape walk
  once per actual validator/union resolver; every field using it gets a
  positive control and one invalid wiring case at its own located path.
- **Darkmatter fields:** cover permissions mapping/reference, file-read and
  shell-commands lists and their strings, network, register, and each register
  reference. Absent fields take scope defaults. `{}` permissions expands
  child defaults; `[]` is an authored empty permission/registration list.
  Null, blank references, empty rules/patterns, unknown keys, and reference
  chains are errors. A valid list of exclusions only is allowed and grants
  nothing. A target's absent/null/string permissions is an error, distinct
  from the source config's absent property default.
- **Claudine fields:** exercise every top-level field enumerated in Wave 5
  and the nested schema paths for voice selections, messenger route variants,
  Protect rules/custom patterns, event action variants/mappers, model overrides,
  sound categories, exit-expression rules, repetition/volume settings, and
  steering. The schema's presence/default metadata supplies each absent
  outcome; keep the special absent/null behavior of active_messenger,
  nullable existing Option fields, and invalid-null steering. Preserve exact
  user-versus-repo allowed-key sets. A collection's empty shape is explicit,
  never treated as absence. Existing semantic constraints remain errors,
  including invalid active routes, regexes, and provider scopes.
- **Manifest fields:** cover kind, format_version, host, version, identify,
  triggers and each reference, globals, lifecycle, functions, and each
  catalog/facts pair. Every absent/null/wrong-typed required member is an
  error; only triggers/functions may be empty collections. Unknown fields,
  remote references, inconsistent host IDs, missing pairs, and malformed
  loaded members refuse the host as a unit.
- **Document loaders:** the moved admitted-YAML loader serves `.yaml` and
  `.yml` configs/manifests/targets, so one loader matrix covers duplicate keys
  at multiple depths, trailing garbage, multiple documents, non-string keys,
  invalid numbers, malformed syntax, BOM/CRLF, YAML `k:` null, legal comments,
  and start/end markers. An empty config file is not a mapping and errors;
  an explicit `{}` config is valid. Run each other entry point's control
  plus one invalid document to prove it uses that loader.
- **JSON5 conversion loader:** test decoded duplicate keys, trailing garbage,
  invalid/non-finite numbers, wrong root shape, and allowed comments/trailing
  commas. Preserve unconverted bytes and existing YAML on every failure.
- **Artifact and receipt loaders:** lifecycle/facts and the new receipt use
  the one extracted strict JSON loader. Retain its duplicate/trailing/syntax
  matrix once. Test receipt format_version, target, published_hash, sources,
  path/hash/action, and backup_path through public migration outcomes for
  absent/null/wrong/empty shapes; collections include mixed/all invalid
  elements. All are required except backup_path on delete; an empty sources
  list is invalid. Add lifecycle/facts explicit-path controls and hash-mismatch
  refusals through the host loader. Invalid receipts clean up nothing.
- **CLI wiring only:** spawn md/Claudine for a control and one field-shape
  and one syntax failure per changed loader route. Assert file/line in the
  appropriate diagnostics channel, empty stderr in data mode, and an empty
  argv-recorder/effect log. The in-process matrix owns the remaining cells.
- **Completion audit:** search changed loaders for `#[serde(default)]`,
  `Option<T>`, `filter_map` with `as_str`, `unwrap_or_default()`, and `.ok()`.
  Remove invalid-shape swallowing. Retain serde defaults only behind the
  validated schema projection or for documented presence-sensitive fields;
  use a present deserializer/`Option<Option<T>>` for absent versus null.
  Record each retained case and its public test. A review finding expands
  to the complete validator/loader row and column, not one added cell.

### Validation checkpoint

- [x] **Verify config matrices**
  - Coordinator checkpoint after Wave 6: execute the Darkmatter/Claudine
    configuration and JSON5 matrix through public APIs, conversion
    interruption tests, writer concurrency tests, and hermetic CLI wiring.
    The receipt validator matrix runs in Phase 2, JSON conversion recovery
    in this phase, policy migration recovery in Phase 4, and manifest/artifact
    integration in Phase 5, alongside their owning implementations. Verify all
    current Claudine configuration fixtures
    still describe the same values/scopes after changing their file syntax.
  - Run affected area L1 filters and regenerate schemas. The two configuration
    loaders must be usable before implementing setup and registration.

## Phase 4: Unify permission evaluation, persistence, and setup

### Wave 7 — Permission mechanisms (parallel)

- [x] **Evaluate scope policies**
  - Own `config/permissions.rs` and reference resolution, using the pinned
    Sniff view and Rules 14–15. Implement whole-block references with one hop,
    independent HEAD/worktree reads, user/repo defaults, semantic intersections,
    exclusions, and scope union.
  - Test missing scope files, omitted/empty lists, different equivalent
    allows, exclusions in every layer, uncommitted additions/removals,
    inline/reference conversion and redirection, target edits/deletion,
    unborn HEAD, Git failures, and ref movement after capture.
  - Test portable variables, absolute Unix/Windows paths, escaped exact
    globs, home rejection at repo scope, lexical traversal, symlink escape,
    committed symlink chains/cycles, and missing-file probes. Use native path
    fixtures and shared identity APIs rather than slash-replacement helpers.

- [x] **Enforce fetch consent**
  - Own Biscuit File's FetchPolicy change and Darkmatter's remote policy
    projection. Implement Rule 17 without changing transport/cache rules.
  - Use loopback HTTP fixtures to prove allow, deny, exact-host override,
    redirect reauthorization, cached-body denial, and slot/composition parity.
    No live internet API or real-tier test is needed.
  - Update Biscuit File fetch docs and the affected skill/API description;
    keep default construction deny-all.

- [x] **Prepare legacy migration**
  - Own `config/migration.rs` and policy migration recovery tests. Implement
    Rule 20 on top of the Phase 2 transactions/receipts, using strict legacy
    parsing. Report every dropped or reinterpreted line through returned
    diagnostics. Reuse Rule 34's receipt API without a second parser.
  - Test mixed whitelist/blacklist conversion as one unit, explicit authored
    empty properties, home-portable patterns, source changes, failed writes,
    crash windows, successful repeat, and no migration of document-directory
    fallback policy files. Do not remove unconverted sources.

### Wave 8 — Enforcement adapters (parallel)

- [x] **Unify shell decisions**
  - Own Darkmatter shell policy/preflight/execution and md lifecycle adapters.
    Replace old stores/paths with shared permission decisions and transaction
    persistence; remove user blacklist and permanent-deny APIs. The coordinator
    integrates both prompt changes in the shared CLI `approval.rs` after the
    parallel library tasks, before Wave 9.
  - Preserve exact/prefix/template matching, late graph membership, replay
    checks, and chain-level behavior. Any excluded segment requires approval;
    approval of a chain cannot bypass a built-in refusal in another segment.
  - Test built-in refusal under whitelist-equivalent allows, once approval,
    cached approval, templates, yolo, and lifecycle execution; test exclusions
    prompt or return a noninteractive approval-required error.

- [x] **Unify slot decisions**
  - Own slot grants/gates/session and file listing filtering. The coordinator
    owns `darkmatter/cli/src/approval.rs` for both shell and schema prompt
    edits after these parallel library tasks; neither worker edits that shared
    file. Implement Rules 16/19; remove the automatic clean-file
    grant, active store reads, folder grants, and always-deny choices.
  - Test content/probe/import/schema checks before memo access, listings
    returning only permitted files, no directory prompt, current-run overrides,
    exclusion persistence refusal, portable exact-file saves, and read →
    approve → read preserving host bytes. URL reads use effective network
    consent and per-run approvals without a retired second store.

- [ ] **Integrate host permissions**
  - Own Claudine's shell harness/options and system-prompt paths plus DMLS's
    `overlay/shell.rs` and retired `shell.policy_path` setting diagnostic.
    All use the shared captured permission result; remove policy-directory
    fallback logic, never substitute a permissive empty ruleset on an error.
  - Extend parity fixtures through md, Claudine preparation, and passive DMLS
    classification. DMLS emits no prompt or effect and supplies no approvals.

### Wave 9 — Setup completion (sequential)

- [ ] **Implement setup handshake**
  - Add `md init` command/args/output, new `config/init.rs`, and Claudine's
    shared setup service. Implement Rule 23, with noninteractive services
    behind the existing wizard so tests need no terminal or human input.
  - Repair reads raw admitted config/reference observations before invoking
    the failing permissions accessor; a broken target cannot block explicit
    `--inline`. Require a captured home for init/persistence; without a home,
    ordinary commands use user defaults with one `dm.config.missing` warning
    explaining the unavailable path, and writes fail without creating files.
  - Finalize writing-CLI migration before configuration capture. Finish the
    coordinator's shared approval prompt edits from Wave 8. Connect
    saved shell/file approvals to the user reference; test save failure keeps
    the grant for this run and emits `dm.config.not_saved` correctly.
  - Cover both setup orders, repeated init, authored values/comments,
    conflicting blocks, third-party references, broken reference repair,
    interrupted two-file transfer, and `--repo` outside a repository with
    zero writes. Claudine register-entry publication is connected in Phase 7.

- [ ] **Verify permission parity**
  - Exercise the same file, command, template, URL, and exclusion fixtures
    through all three consumers; assert equivalent structured decisions.
    Verify every retired file/API has only temporary migration/test references.
  - Checkpoint: `just test`/`just lint` in biscuit-file, sniff, darkmatter,
    simplified-schema, and claudine for the code changed so far. Reuse passing
    unchanged foundation runs; rerun suites affected since those runs.

## Phase 5: Load registered hosts and compose foreign documents

### Wave 10 — Registration core (parallel)

- [ ] **Prove host identity**
  - Own Simplified Schema identifying-trigger admission/matching and tests.
    Implement Rule 25 over the moved AST without changing ordinary triggers.
  - Cover every combinator, shared-only/optional-only/path-only/empty cases,
    min-match bounds, nullable/unknown atoms, malformed host values, and
    no filesystem work during proof/matching. Demonstrate Claudine's nine
    keys identify at every non-null YAML type, while shared events alone do not.

- [ ] **Load host profiles**
  - Own manifest schema and `config/registration` member loader plus explicit
    lifecycle/facts companion-path adapters. Implement Rule 24 and the input
    matrix, preserving retained reads/import anchors and all hash checks.
  - Add `RegisteredHostProfile`, manifest/dependency content identity, typed
    registration diagnostics, duplicate manifest deduplication, and same-host
    agreement/conflict detection. Failed profiles expose no partial globals,
    functions, lifecycle events, or trigger payloads.

### Wave 11 — Registration consumers (parallel)

- [ ] **Layer host schemas**
  - Own Darkmatter host selection and `DarkmatterSchemas` registered-layer
    integration. Implement Rule 26 with origin inspection and cache identity.
  - Test a same-basename root file overriding an actual declaration without
    shadowing the registered trigger; test identical file/content dedup,
    manifest order, authored schema override, import anchors, no-match legacy
    fallback, conflicts, ambiguity, and unrelated-document isolation.
  - Thread selection into compose, validate, clean, and schema inspection
    preparation paths; preserve executable host options supplied by Claudine.

- [ ] **Compose foreign documents**
  - Own md analysis/runtime separation and CLI notice projection. Implement
    Rule 27 and add an in-process execution/effect recorder fixture.
  - Prove body composition and selected host validation using a foreign
    document with literal shared-event operands. The motivating document's
    `seq.name` in initialize is clean in passive analysis and fails normally
    at md runtime without seq bindings; it is not an expected successful md
    execution. Shared lifecycle stacks run with md bindings, foreign-only
    stacks/tasks/agents/functions do not run, and unavailable host values fail normally in shared stacks.
    Prove `composed`/`failed` are rejected by the host schema without an
    internal md lifecycle collision.
  - Cover formatted and data outputs and once-per-request notices, including
    nested transclusion, without launching an actual agent.

### Validation checkpoint

- [ ] **Verify registered behavior**
  - Coordinator checkpoint after Wave 11: run registration public matrix,
    layering and CLI contract tests, plus shipped-artifact checks. Verify
    registration performs no install/write/network/prompt/handler effects.

## Phase 6: Make DMLS registration-aware

### Wave 12 — Editor preparation (parallel)

- [ ] **Select editor contracts**
  - Own explicit configuration path selection in `overlay/yaml_document.rs`,
    `overlay/yaml_contract.rs`, and schema preparation/providers. Select the
    shipped user/repo config types by captured path identity before general
    kind/root schema discovery; ignore authored `$schema` overrides there.
  - Add new `dmls/src/config_contracts.rs` as Rule 33's static path-contract
    adapter. Select the scope-specific Darkmatter contract for
    `.darkmatter.yaml` and the embedded Claudine contract for
    `.claudine/config.yaml`, including an empty newly created buffer. No
    host registration or installed revision is needed to edit a configuration.
    Preserve schema defaults, docs, and errors on hover, completion, and
    diagnostics.
  - Remove active `shell.policy_path` semantics and return a located retired
    setting diagnostic naming `.darkmatter.yaml`/`permissions.shell-commands`.

- [ ] **Cache selected hosts**
  - Own `TriggerRegistries`, `AnalysisIdentity`, `LifecycleHost`, and
    provider host-view plumbing. Implement the one coherent selected view
    in Rules 26/28 across `providers/{dsl,frontmatter}.rs`, event views,
    expression checking, hover, completion, and named function typing.
  - Remove independent registered-host rescans from provider calls. Keep the
    legacy fallback only through the central selection result; ambiguity
    never invokes it. Add precise unknown-host diagnostics for known events.

### Wave 13 — Editor invalidation (sequential)

- [ ] **Watch passive inputs**
  - Extend `workspace/watch.rs` and backend invalidation to configuration,
    references, manifests, revision directories, all schema/catalog/artifact
    dependencies, and captured HEAD. Watch missing paths from existing parents
    and atomic replacements through the same dependency preparation model.
  - Preserve prepare/install generation checks and reuse unchanged profiles.
    Invalid new revisions clear selected host facts and diagnose the defect;
    delete/add/recreate must update all document projections without restart.
  - Emit `dm.config.missing` as one session notification. Passive configuration
    and legacy-migration diagnostics never launch setup or write files.

- [ ] **Verify editor behavior**
  - Use existing in-process/provider and hermetic stdio LSP fixtures; add no
    real editor, browser, or OS-focus test. Cover the motivating sequence/start
    example outside the package area, shared-events-only md files, wrong-typed
    identifying values, unrelated keys, duplicate/conflicting hosts, pending
    defaults, config errors, missing targets, invalid artifacts, and recovery.
  - Assert diagnostics, hover, completion, and lifecycle analysis agree after
    atomic replacement/deletion. Checkpoint: DMLS L1 through `just test` in
    `darkmatter/`, with current config/schema artifacts accepted.

## Phase 7: Publish and maintain Claudine registration

### Wave 14 — Claudine publication (parallel)

- [ ] **Publish host revisions**
  - Own new Claudine registration service and embedded revision source table.
    Implement Rule 29 and export the passive config schema alongside the
    manifest members/dependencies. Keep the stable manifest fields exactly
    v1; its member paths name the immutable revision directory.
  - Publish the config schema as an auxiliary embedded source for inspecting
    the installed revision, never as a schema-trigger or kind-based selector.
    DMLS's built-in configuration contract adapter is Rule 33; the v1 manifest
    gains no undisclosed configuration metadata or selection field.
  - Test first publish, no-op refresh, version-only/content-only changes,
    concurrent publishers, corrupt preexisting revision, failed staging,
    failed stable-manifest replacement, and readers holding the old manifest.
    Retain old revisions and prove every published manifest sees one complete
    revision. No manual copied-schema fixture is an installation mechanism.

- [ ] **Narrow Claudine triggers**
  - Own `claudine/schemas/claudine-document.yaml`, `claudine.yaml` match
    changes, and corresponding bundle/corpus tests. Remove initialize,
    cancelled, and finalize as sole root-trigger selectors; retain their
    payload definitions and all host-only selectors.
  - Generate identification with sequence/start/blocked/success/failure/loop/
    prompt/agent/model. Update bundle/shipped sources to include new texts
    without adding runtime execution through the identifying trigger.

### Wave 15 — Launch and setup integration (sequential)

- [ ] **Connect Claudine setup**
  - Connect init to publication, permission transfer, and user registration
    insertion in that order. Preserve all existing register entries and
    deduplicate by resolved stable manifest identity. Add repository setup
    only for the repository scope explicitly requested by existing init.
  - Connect launch refresh after help/completion exits and before document
    preparation. Refresh only Claudine's install; it never changes Darkmatter
    registrations. Ordinary runtime still uses `claudine_registry()` and the
    embedded bundle even with a hostile repo registration for the same host.
  - Return refresh diagnostics through the existing output path without
    launching init or asking a question in noninteractive operation.

- [ ] **Verify installed integration**
  - In temporary home/repository fixtures, exercise init in both orders,
    repeat setup, launch refresh, old-revision readers, and failure recovery.
    Run md and DMLS against the published revision outside `claudine/`.
    Assert a binary refresh updates facts without any `~/schemas` copies.
  - Regenerate the full shipped chain after trigger/config additions and
    update freshness-test source-input lists. Checkpoint: Claudine and
    Darkmatter L1/lint, source/bundle parity, guards, and shipped corpus.

## Phase 8: Detect permission widening in trusted CI

### Wave 16 — Conservative comparison (parallel)

- [ ] **Compare resolved policies**
  - Add new `darkmatter::config::widening` and
    `darkmatter/lib/examples/permission_widening.rs`, a narrow noninteractive
    checker using the same pinned-tree/config/default/reference code. Inputs
    are base/head commit IDs and known repository roots; output is JSON with
    changed inputs, resolved before/after policies, reasons, and
    `potential_widening`. Candidate schema text is passive data, never code.
  - Enumerate `.darkmatter.yaml` and `.claudine/config.yaml` policy blocks in
    both revisions, references and targets, and the shipped default/schema
    inputs. Pure comparator: normalize equivalent exact rules/glob spellings,
    deduplicate order-insensitive entries, resolve unchanged inline/reference
    moves, and compare effective scope defaults. Prove narrowing only for
    unchanged policies, removed positive entries, added identical-semantic
    exclusions, and allow → deny network transitions. Other glob/template
    rewrites are conservatively widening; do not attempt general containment.
  - Changes to defaults, reference targets, deleted files/properties restoring
    defaults, or the checker/dependency inputs cannot evade comparison. A
    changed default schema not understood by trusted code is potential
    widening. Parse/Git/reference errors fail the checker, never mean no change.
  - Test equal inline/reference policy, missing/default restores, allow
    replacements of the same length, removed exclusions, mixed add/remove,
    network, symlink/reference target changes, and hostile candidate executable
    files. Tests call the public comparator with fixture revisions.

- [ ] **Prepare environment setup**
  - Add new `scripts/ci/permission_environment.py`, dry-run by default, with
    explicit `--apply` to configure Rule 31's exact environment. Use GitHub's
    REST API through the existing noninteractive `gh` authentication; never
    log credentials or approve a deployment. Verify the resulting settings.
  - Add mocked request/response tests for idempotence, exact reviewer set,
    self-review enabled, admin bypass disabled, API rejection, and weak/missing
    settings. Provide the final command and payload in CI docs and the final
    review checklist. Do not execute `--apply` in implementation.

### Wave 17 — Workflow integration (sequential)

- [ ] **Gate widening results**
  - Add the two jobs in Rule 31. `permission-scope` uses PR base/head IDs;
    pushes compare event-before/head; scheduled/manual runs with no branch
    delta produce no widening work. Fetch immutable objects without running
    candidate scripts. Build/run checker code and lockfile only from the
    trusted base checkout under `$RUNNER_TEMP/permissions-trusted`, outside
    the candidate checkout and its ancestor `.cargo` configuration. Candidate
    trees are read-only object data; no candidate build script, workflow
    helper, configuration import, or executable runs.
  - Determine input changes from both revisions' policy/reference/default
    dependency closure. The lightweight scope job may inspect paths on every
    PR/push; compilation/comparison and the approval job run only for changed
    policy inputs/checker inputs. Bootstrap absence is potential widening.
    Include trusted checker dependencies and workflow/configuration guard
    files among the checker inputs, so an attempted bypass is visible.
  - Verify environment protection through a read-only API call before
    requesting it. `permission-widening` has a job-level condition on
    potential widening and `environment: permission-widening`; a nonwidening
    change skips it and never waits. Set no job timeout shorter than the
    intended approval period; approval itself happens before runner execution.
  - Add both jobs to `ci-gate`'s `needs` and ordinary result list, keeping its
    fold policy-free. A failed classifier blocks even when approval skips.
    Do not alter canonical package/environment/gate scheduling, add an OS
    matrix, or duplicate test suites in preflight.
  - Update `tools/test-toolkit/tests/ci_workflow_contracts.rs` exact job/gated
    lists, trust-boundary assertions, and workflow fixtures; exercise them in
    their existing L1 owner. Add fake-GitHub workflow decision tests for
    unchanged/narrowing/widening/error/missing-environment/bootstrap cases.

- [ ] **Verify CI contract**
  - Run test-toolkit's CI contract tests and the checker/environment tests
    locally through existing owners. Inspect `just ci-local --plan` without
    pushing; confirm changed source/default inputs schedule their owners and
    the new gate adds no package OS cells.
  - Keep live environment activation and deployment approval in the final
    review artifacts. Do not launch a full-scope workflow to test GitHub UI
    behavior or fabricate a passing deployment observation.

## Phase 9: Complete documentation and acceptance evidence

### Wave 18 — Current documentation (parallel)

- [ ] **Document user behavior**
  - Own Darkmatter docs: new `topics/configuration.md` and
    `topics/permissions.md`, replacing obsolete grant-store/whitelist guidance
    in `darkmatter/docs/topics/schemas/schema-slots.md` and
    `claudine/docs/topics/pre-flight-checks.md` (owned by this task).
    Document defaults,
    overrides, exclusions, union/intersection order, reference repair, init
    orders, temporary migration notices, and the cloned-repository risk.
  - Update CLI policy/data/output/init reference, `host-artifact.md`,
    `dmls-schema-support.md`, configuration schema editing, and foreign
    composition behavior. Use compact examples and Mermaid flows for request
    capture, HEAD/worktree policy, setup transfer, and immutable publication.
    Keep READMEs to an overview and links into current docs.

- [ ] **Document host behavior**
  - Own Simplified Schema validation/defaults/global-type-constraints docs,
    Darkmatter `schemas/authoring-schemas.md`, and Claudine configuration/
    schema/setup docs. Fill the required/eager/
    default/suggest sections, explain external JSON Schema's annotation limit,
    preserve nullable/optional rules, and remove manual `~/schemas` install
    instructions. Document YAML conversion and its future removal notice.
  - Update Claudine/Darkmatter/Simplified Schema/Sniff/Biscuit File skills for
    final ownership, snapshots, editing, policy, and registration APIs.
    Update CI docs and root conventions for the two added jobs and final
    environment setup. Documentation never links to dated feature/fix specs.

### Wave 19 — Acceptance and handoff (sequential)

- [ ] **Run acceptance checks**
  - Run the affected areas' `just test` and `just lint` recipes against the
    final state: simplified-schema, biscuit-file, sniff, darkmatter, and
    claudine; run root `just test test-toolkit` for CI contracts. Preserve
    previous qualifying results only for code unchanged since their run.
    Add compile checks for changed examples and directly affected unchanged
    workspace callers; do not run a full workspace suite by default.
  - Use nextest/Test Toolkit and consolidated test roots. In tests, locate
    fixture/binary paths through `manifest_dir!`/`bin_exe!`; all new policy,
    migration, host, LSP, and concurrency tests are L1. Exercise changed prompt
    choices with scripted handlers at L1; run the existing focused
    `just test-l2` approval tests for terminal wiring without focus changes.
    Do not add real-browser or L3 tests for passive analysis.
  - Run cross-platform L1 evidence for the changed filesystem/Git packages
    with `just cross-check <package> --os linux|windows|wsl` on the configured
    BUILD hosts; macOS evidence is the local run. Include biscuit-file,
    sniff, simplified-schema, darkmatter, darkmatter-cli, dmls, claudine,
    and claudine-cli for changed behavior/caller coverage. Reuse qualifying
    unchanged evidence per cell; report actual platform results separately.
    Every direct SSH command uses `-o BatchMode=yes`; WSL evidence does not
    count as native Windows. Follow the os skill's archive/path conventions.
  - Run shipped artifact freshness/corpus checks after the final regeneration,
    source-guard checks, comment-drift audit, and the input-matrix smell search.
    Do not run `cargo fmt` or unrelated formatting. Fix discovered defects
    in the owning phase's implementation and rerun only affected checks.

- [ ] **Prepare review evidence**
  - Update this plan's completed checkboxes and the existing implementation
    log. Include the exact acceptance command/results, public matrix coverage,
    installed-host fixture output, policy before/after examples, and all
    concrete spec departures. Do not rewrite the spec's decisions or move it.
  - Include the environment setup dry-run output and exact final activation
    command, with the observed live environment status. Distinguish the
    tested workflow/checker from a live deployment protection rule; no claim
    of live enforcement is made before final activation.
  - Re-read docs and code for the retired active policy files, blacklist/root
    APIs, JSON config reader names, automatic grants, and copied-schema setup.
    Remaining references must describe temporary migration or historical
    fixtures, not an alternate active configuration path.
  - End with “implementation complete, ready for review” and the concrete
    final-review artifacts. No human sign-off, permission prompt, commit,
    push, deployment approval, or feature completion move occurs in this plan.