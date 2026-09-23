---
spec: /Volumes/coding/wt/rusty-biscuit/feat-dark-fixes/features/2026-09-22-consolidated-test-binaries-wave-2/spec.md
plan: features/2026-09-22-consolidated-test-binaries-wave-2/plan.md
implemented_by: claude/opus
started_phase: 1
source_files_during_phase_1:
    - features/2026-09-22-consolidated-test-binaries-wave-2/baseline/pre-existing.sh
    - features/2026-09-22-consolidated-test-binaries-wave-2/baseline/test-input-probe.py
    - features/2026-09-22-consolidated-test-binaries-wave-2/baseline/test-input-listings.py
    - features/2026-09-22-consolidated-test-binaries-wave-2/baseline/consumer-sweep.py
    - features/2026-09-22-consolidated-test-binaries-wave-2/spikes/s2-scan.py
docs_updated_during_phase_1:
    - features/2026-09-22-consolidated-test-binaries-wave-2/plan.md
    - features/2026-09-22-consolidated-test-binaries-wave-2/spec.md
docs_created_during_phase_1:
    - features/2026-09-22-consolidated-test-binaries-wave-2/rulings.md
    - features/2026-09-22-consolidated-test-binaries-wave-2/implementation-log.md
    - features/2026-09-22-consolidated-test-binaries-wave-2/spikes/s1-feature-sets.md
    - features/2026-09-22-consolidated-test-binaries-wave-2/spikes/s2-hazards.md
    - features/2026-09-22-consolidated-test-binaries-wave-2/spikes/s2-scan-output.txt
    - features/2026-09-22-consolidated-test-binaries-wave-2/spikes/s3-cross-check.md
    - features/2026-09-22-consolidated-test-binaries-wave-2/spikes/s4-deps.md
    - features/2026-09-22-consolidated-test-binaries-wave-2/baseline/pre-existing.md
    - features/2026-09-22-consolidated-test-binaries-wave-2/baseline/skip-baseline.md
    - features/2026-09-22-consolidated-test-binaries-wave-2/baseline/test-inputs.md
    - features/2026-09-22-consolidated-test-binaries-wave-2/baseline/test-selector-consumers.md
    - darkmatter/fixes/_unscheduled/proptest-regressions-after-consolidation/spec.md
skills_files_updated_during_phase_1:
    - .claude/skills/os/build-hosts.md
source_files_during_phase_2:
    - scripts/ci/consolidation.py
    - scripts/ci/test_consolidation.py
    - features/2026-09-22-consolidated-test-binaries-wave-2/selfproof/mutation-check.py
docs_updated_during_phase_2:
    - features/2026-09-22-consolidated-test-binaries-wave-2/plan.md
    - features/2026-09-22-consolidated-test-binaries-wave-2/implementation-log.md
    - features/2026-09-22-consolidated-test-binaries-wave-2/spec.md
docs_created_during_phase_2:
    - features/2026-09-22-consolidated-test-binaries-wave-2/selfproof/README.md
    - features/2026-09-22-consolidated-test-binaries-wave-2/selfproof/dry-run.md
    - features/2026-09-22-consolidated-test-binaries-wave-2/selfproof/noop-comparison.md
    - features/2026-09-22-consolidated-test-binaries-wave-2/selfproof/noop-comparison.json
    - features/2026-09-22-consolidated-test-binaries-wave-2/selfproof/inventory.md
    - features/2026-09-22-consolidated-test-binaries-wave-2/selfproof/inventory.json
    - features/2026-09-22-consolidated-test-binaries-wave-2/selfproof/mutation-check.txt
    - features/2026-09-22-consolidated-test-binaries-wave-2/selfproof/r18-proptest-scratch.txt
    - features/2026-09-22-consolidated-test-binaries-wave-2/selfproof/wave1-metadata-check.txt
    - features/2026-09-22-consolidated-test-binaries-wave-2/selfproof/wave1-darkmatter-proptest-check.txt
    - features/2026-09-22-consolidated-test-binaries-wave-2/selfproof/capture-b.SHA256SUMS
    - features/2026-09-22-consolidated-test-binaries-wave-2/selfproof/capture-a/
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
    - tree-hugger/lib/Cargo.toml
    - claudine/lib/Cargo.toml
    - sniff/lib/Cargo.toml
    - .config/nextest.toml
    - Cargo.lock
    - sniff/lib/benches/support/bench_ids.rs
    - claudine/lib/tests/l1/agent_errors_fleet.rs
    - claudine/lib/tests/l1/boundary_lint.rs
    - claudine/lib/tests/l1/canonical_dispatch.rs
    - claudine/lib/tests/l1/deprecated_compatibility.rs
    - claudine/lib/tests/l1/diagnostic_detail_conformance.rs
    - claudine/lib/tests/l1/kimi_wire.rs
    - claudine/lib/tests/l1/lifecycle_control_flow_spike.rs
    - claudine/lib/tests/l1/main.rs
    - claudine/lib/tests/l1/model_catalog_integration.rs
    - claudine/lib/tests/l1/opencode_stderr_lifecycle.rs
    - claudine/lib/tests/l1/protocol_fixture_replay.rs
    - claudine/lib/tests/l1/semantic_fidelity.rs
    - claudine/lib/tests/l1/strict_mode_provenance_spike.rs
    - claudine/lib/tests/l1/test_layout.rs
    - claudine/lib/tests/l1/tts_phase1_contract.rs
    - claudine/lib/tests/l1/tts_phase5_contract.rs
    - claudine/lib/tests/l1/typed_stream_protocols.rs
    - sniff/lib/tests/l1/bench_fixtures.rs
    - sniff/lib/tests/l1/bench_ids_sync.rs
    - sniff/lib/tests/l1/bench_plans.rs
    - sniff/lib/tests/l1/benchmark_workloads.rs
    - sniff/lib/tests/l1/focused_provider.rs
    - sniff/lib/tests/l1/git_parity.rs
    - sniff/lib/tests/l1/host_capability_cache.rs
    - sniff/lib/tests/l1/integration.rs
    - sniff/lib/tests/l1/main.rs
    - sniff/lib/tests/l1/merge_conflict_prediction.rs
    - sniff/lib/tests/l1/network_primitives.rs
    - sniff/lib/tests/l1/program_installable.rs
    - sniff/lib/tests/l1/program_serialization.rs
    - sniff/lib/tests/l1/recent_commits.rs
    - sniff/lib/tests/l1/remote_observation.rs
    - sniff/lib/tests/l1/remote_providers.rs
    - sniff/lib/tests/l1/remote_resolution.rs
    - sniff/lib/tests/l1/test_layout.rs
    - sniff/lib/tests/l1/uv_with_install_plan.rs
    - sniff/lib/tests/l1/windows_app_paths_orphan.rs
    - sniff/lib/tests/l1/windows_find_program_priority.rs
    - tree-hugger/lib/tests/l1/adapter_tests.rs
    - tree-hugger/lib/tests/l1/cache_tests.rs
    - tree-hugger/lib/tests/l1/corpus_tests.rs
    - tree-hugger/lib/tests/l1/lint_diagnostics.rs
    - tree-hugger/lib/tests/l1/main.rs
    - tree-hugger/lib/tests/l1/phase1_diagnostics.rs
    - tree-hugger/lib/tests/l1/phase6_neovim_query_reuse.rs
    - tree-hugger/lib/tests/l1/query_compile.rs
    - tree-hugger/lib/tests/l1/resolver_tests.rs
    - tree-hugger/lib/tests/l1/test_layout.rs
    - tree-hugger/lib/tests/l1/tree_file.rs
    - tree-hugger/lib/tests/l1/tree_package.rs
    - features/2026-09-22-consolidated-test-binaries-wave-2/measure/measure.sh
    - features/2026-09-22-consolidated-test-binaries-wave-2/measure/layout-guard.sh
    - features/2026-09-22-consolidated-test-binaries-wave-2/measure/test-input-check.py
docs_updated_during_phase_3:
    - tree-hugger/lib/README.md
    - docs/comment-quality.md
    - docs/dependencies.md
    - features/2026-09-22-consolidated-test-binaries-wave-2/plan.md
    - features/2026-09-22-consolidated-test-binaries-wave-2/implementation-log.md
    - features/2026-09-22-consolidated-test-binaries-wave-2/spec.md
docs_created_during_phase_3:
    - features/2026-09-22-consolidated-test-binaries-wave-2/measurements.md
    - features/2026-09-22-consolidated-test-binaries-wave-2/tree-hugger-migration.json
    - features/2026-09-22-consolidated-test-binaries-wave-2/claudine-migration.json
    - features/2026-09-22-consolidated-test-binaries-wave-2/sniff-migration.json
    - features/2026-09-22-consolidated-test-binaries-wave-2/tree-hugger/
    - features/2026-09-22-consolidated-test-binaries-wave-2/claudine/
    - features/2026-09-22-consolidated-test-binaries-wave-2/sniff/
    - features/2026-09-22-consolidated-test-binaries-wave-2/measure/tree-hugger-before.txt
    - features/2026-09-22-consolidated-test-binaries-wave-2/measure/tree-hugger-before-edit.log
    - features/2026-09-22-consolidated-test-binaries-wave-2/measure/tree-hugger-after.txt
    - features/2026-09-22-consolidated-test-binaries-wave-2/measure/tree-hugger-after-edit.log
    - features/2026-09-22-consolidated-test-binaries-wave-2/measure/claudine-before.txt
    - features/2026-09-22-consolidated-test-binaries-wave-2/measure/claudine-before-edit.log
    - features/2026-09-22-consolidated-test-binaries-wave-2/measure/claudine-after.txt
    - features/2026-09-22-consolidated-test-binaries-wave-2/measure/claudine-after-edit.log
    - features/2026-09-22-consolidated-test-binaries-wave-2/measure/sniff-before.txt
    - features/2026-09-22-consolidated-test-binaries-wave-2/measure/sniff-before-edit.log
    - features/2026-09-22-consolidated-test-binaries-wave-2/measure/sniff-after.txt
    - features/2026-09-22-consolidated-test-binaries-wave-2/measure/sniff-after-edit.log
skills_files_updated_during_phase_3:
    - .claude/skills/os/build-hosts.md
    - .claude/skills/sniff/network.md
    - .claude/skills/tree-hugger/query-system.md
packages:
    - tree-hugger
    - claudine
    - sniff
    - biscuit-file
    - schematic-gen
    - biscuit-terminal-cli
    - claudine-gen
    - dmls
    - biscuit-test-harness
    - renderable
    - sniff-cli
    - biscuit-tui-cli
    - test-toolkit
    - darkmatter
source_files_during_phase_4:
    - .config/nextest.toml
    - Cargo.lock
    - biscuit-file/lib/Cargo.toml
    - biscuit-file/lib/tests/l1-fetch/fetch_integration.rs
    - biscuit-file/lib/tests/l1-fetch/main.rs
    - biscuit-file/lib/tests/l1/completion_round_trip.rs
    - biscuit-file/lib/tests/l1/detailed_resolution.rs
    - biscuit-file/lib/tests/l1/finalized_reference_resolution.rs
    - biscuit-file/lib/tests/l1/implicit_relative.rs
    - biscuit-file/lib/tests/l1/main.rs
    - biscuit-file/lib/tests/l1/parse_count.rs
    - biscuit-file/lib/tests/l1/precedence_flip.rs
    - biscuit-file/lib/tests/l1/reference_grammar.rs
    - biscuit-file/lib/tests/l1/repository_scope_catalog.rs
    - biscuit-file/lib/tests/l1/resolution_context.rs
    - biscuit-file/lib/tests/l1/round_trip.rs
    - biscuit-file/lib/tests/l1/span_compat.rs
    - biscuit-file/lib/tests/l1/test_layout.rs
    - biscuit-file/lib/tests/l1/yaml_corpus.rs
    - biscuit-file/lib/tests/l1/yaml_mutation.rs
    - biscuit-file/lib/tests/l1/yaml_safety.rs
    - biscuit-file/lib/tests/proptest-regressions/yaml_mutation.txt
    - biscuit-terminal/cli/Cargo.toml
    - biscuit-terminal/cli/tests/l1/about.rs
    - biscuit-terminal/cli/tests/l1/dir_targets.rs
    - biscuit-terminal/cli/tests/l1/integration_test.rs
    - biscuit-terminal/cli/tests/l1/main.rs
    - biscuit-terminal/cli/tests/l1/snapshots/l1__integration_test__columns_snapshot.snap
    - biscuit-terminal/cli/tests/l1/snapshots/l1__integration_test__list_snapshot.snap
    - biscuit-terminal/cli/tests/l1/snapshots/l1__integration_test__padleft_snapshot.snap
    - biscuit-terminal/cli/tests/l1/snapshots/l1__integration_test__padright_snapshot.snap
    - biscuit-terminal/cli/tests/l1/snapshots/l1__integration_test__prose_snapshot.snap
    - biscuit-terminal/cli/tests/l1/snapshots/l1__integration_test__prose_styled_snapshot.snap
    - biscuit-terminal/cli/tests/l1/snapshots/l1__integration_test__quote_snapshot.snap
    - biscuit-terminal/cli/tests/l1/test_layout.rs
    - biscuit-terminal/cli/tests/level2/diagrams.rs
    - biscuit-terminal/cli/tests/level2/level2_apple_terminal_prose.rs
    - biscuit-terminal/cli/tests/level2/level2_container_fenced_code.rs
    - biscuit-terminal/cli/tests/level2/level2_cursor_and_hygiene.rs
    - biscuit-terminal/cli/tests/level2/level2_image.rs
    - biscuit-terminal/cli/tests/level2/level2_layout.rs
    - biscuit-terminal/cli/tests/level2/level2_prose_styling.rs
    - biscuit-terminal/cli/tests/level2/level2_render_tree_style.rs
    - biscuit-terminal/cli/tests/level2/level2_status_block.rs
    - biscuit-terminal/cli/tests/level2/level2_style_everywhere_matrix.rs
    - biscuit-terminal/cli/tests/level2/main.rs
    - biscuit-terminal/cli/tests/level2/prose_cells.rs
    - biscuit-test-harness/src/bin/broker.rs
    - claudine/gen/Cargo.toml
    - claudine/gen/tests/l1/agent_errors_check.rs
    - claudine/gen/tests/l1/drift.rs
    - claudine/gen/tests/l1/fixtures_provenance.rs
    - claudine/gen/tests/l1/generate_ux.rs
    - claudine/gen/tests/l1/main.rs
    - claudine/gen/tests/l1/pipeline.rs
    - claudine/gen/tests/l1/registry_coverage.rs
    - claudine/gen/tests/l1/signals_sidecar_mirror.rs
    - claudine/gen/tests/l1/signals_validation.rs
    - claudine/gen/tests/l1/steering_check.rs
    - claudine/gen/tests/l1/test_layout.rs
    - claudine/gen/tests/l1/vocabulary.rs
    - claudine/gen/tests/level2/level2_report_terminal.rs
    - claudine/gen/tests/level2/main.rs
    - claudine/lib/src/provider/tests.rs
    - darkmatter/dmls/Cargo.toml
    - darkmatter/dmls/src/overlay/expressions.rs
    - darkmatter/dmls/tests/fixtures/editor_neovim/init.lua
    - darkmatter/dmls/tests/fixtures/editor_neovim/probe.lua
    - darkmatter/dmls/tests/l1/level1_graph_index.rs
    - darkmatter/dmls/tests/l1/level1_wiki.rs
    - darkmatter/dmls/tests/l1/lsp_session.rs
    - darkmatter/dmls/tests/l1/main.rs
    - darkmatter/dmls/tests/l1/mapping_only_corpus.rs
    - darkmatter/dmls/tests/l1/no_side_effects.rs
    - darkmatter/dmls/tests/l1/packaging_contract.rs
    - darkmatter/dmls/tests/l1/stdio_subprocess.rs
    - darkmatter/dmls/tests/l1/strict_mode_recovery_spike.rs
    - darkmatter/dmls/tests/l1/suggest_constraint_phase1.rs
    - darkmatter/dmls/tests/l1/test_layout.rs
    - darkmatter/dmls/tests/l1/zed_extension_contract.rs
    - darkmatter/dmls/tests/level2/level2_editor_neovim.rs
    - darkmatter/dmls/tests/level2/main.rs
    - features/2026-09-22-consolidated-test-binaries-wave-2/measure/measure.sh
    - schematic/gen/Cargo.toml
    - schematic/gen/tests/l1/artifact_drift.rs
    - schematic/gen/tests/l1/e2e_generation.rs
    - schematic/gen/tests/l1/http_client.rs
    - schematic/gen/tests/l1/main.rs
    - schematic/gen/tests/l1/openapi_import_test.rs
    - schematic/gen/tests/l1/openapi_strict_completeness.rs
    - schematic/gen/tests/l1/path_substitution.rs
    - schematic/gen/tests/l1/postman_artifact_validation.rs
    - schematic/gen/tests/l1/postman_golden.rs
    - schematic/gen/tests/l1/postman_schema.rs
    - schematic/gen/tests/l1/postman_var_consistency.rs
    - schematic/gen/tests/l1/query_param_detection.rs
    - schematic/gen/tests/l1/query_params_codegen.rs
    - schematic/gen/tests/l1/test_layout.rs
    - schematic/gen/tests/l1/ws_codegen.rs
    - schematic/gen/tests/level2/main.rs
    - schematic/gen/tests/level2/terminal_capture.rs
    - schematic/justfile
    - scripts/ci/consolidation.py
    - scripts/ci/test_consolidation.py
docs_updated_during_phase_4:
    - biscuit-file/docs/dependencies.md
    - biscuit-terminal/docs/dependencies.md
    - biscuit-test-harness/README.md
    - claudine/docs/dependencies.md
    - claudine/docs/research/signals/fixtures/README.md
    - claudine/docs/research/signals/fixtures/provenance.yaml
    - claudine/docs/topics/provider-metadata.md
    - darkmatter/dmls/README.md
    - darkmatter/dmls/docs/editors/smoke-checklist.md
    - darkmatter/dmls/docs/features.md
    - darkmatter/dmls/docs/hover.md
    - docs/dependencies.md
    - features/2026-09-22-consolidated-test-binaries-wave-2/implementation-log.md
    - features/2026-09-22-consolidated-test-binaries-wave-2/measurements.md
    - features/2026-09-22-consolidated-test-binaries-wave-2/plan.md
    - features/2026-09-22-consolidated-test-binaries-wave-2/spec.md
    - renderable/docs/layout-and-style.md
    - schematic/docs/dependencies.md
    - schematic/gen/tests/fixtures/postman/README.md
docs_created_during_phase_4:
    - features/2026-09-22-consolidated-test-binaries-wave-2/biscuit-file-migration.json
    - features/2026-09-22-consolidated-test-binaries-wave-2/biscuit-file/
    - features/2026-09-22-consolidated-test-binaries-wave-2/biscuit-terminal-cli-migration.json
    - features/2026-09-22-consolidated-test-binaries-wave-2/biscuit-terminal-cli/
    - features/2026-09-22-consolidated-test-binaries-wave-2/claudine-gen-migration.json
    - features/2026-09-22-consolidated-test-binaries-wave-2/claudine-gen/
    - features/2026-09-22-consolidated-test-binaries-wave-2/dmls-migration.json
    - features/2026-09-22-consolidated-test-binaries-wave-2/dmls/
    - features/2026-09-22-consolidated-test-binaries-wave-2/measure/biscuit-file-after.txt
    - features/2026-09-22-consolidated-test-binaries-wave-2/measure/biscuit-file-before.txt
    - features/2026-09-22-consolidated-test-binaries-wave-2/measure/biscuit-terminal-cli-after.txt
    - features/2026-09-22-consolidated-test-binaries-wave-2/measure/biscuit-terminal-cli-before.txt
    - features/2026-09-22-consolidated-test-binaries-wave-2/measure/claudine-gen-after.txt
    - features/2026-09-22-consolidated-test-binaries-wave-2/measure/claudine-gen-before.txt
    - features/2026-09-22-consolidated-test-binaries-wave-2/measure/dmls-after.txt
    - features/2026-09-22-consolidated-test-binaries-wave-2/measure/dmls-before.txt
    - features/2026-09-22-consolidated-test-binaries-wave-2/measure/schematic-gen-after.txt
    - features/2026-09-22-consolidated-test-binaries-wave-2/measure/schematic-gen-before.txt
    - features/2026-09-22-consolidated-test-binaries-wave-2/schematic-gen-migration.json
    - features/2026-09-22-consolidated-test-binaries-wave-2/schematic-gen/
skills_files_updated_during_phase_4:
    - .claude/skills/biscuit-test-harness/SKILL.md
    - .claude/skills/os/macos.md
    - .claude/skills/schematic-define/SKILL.md
source_files_during_phase_5:
    - biscuit-tui/cli/Cargo.toml
    - biscuit-tui/cli/tests/l1/boolean_switch_output.rs
    - biscuit-tui/cli/tests/l1/choose_cli.rs
    - biscuit-tui/cli/tests/l1/choose_many_output.rs
    - biscuit-tui/cli/tests/l1/choose_one_output.rs
    - biscuit-tui/cli/tests/l1/completions.rs
    - biscuit-tui/cli/tests/l1/completions_shell.rs
    - biscuit-tui/cli/tests/l1/exit_codes.rs
    - biscuit-tui/cli/tests/l1/help_contract.rs
    - biscuit-tui/cli/tests/l1/input_table_output.rs
    - biscuit-tui/cli/tests/l1/keyboard_protocol.rs
    - biscuit-tui/cli/tests/l1/main.rs
    - biscuit-tui/cli/tests/l1/test_layout.rs
    - biscuit-tui/cli/tests/l1/text_area_input_output.rs
    - biscuit-tui/cli/tests/l1/text_input_output.rs
    - biscuit-tui/cli/tests/level2/main.rs
    - biscuit-tui/cli/tests/level2/terminal_render.rs
    - biscuit-tui/cli/tests/level2/windows_captured_stdout.rs
    - biscuit-tui/cli/tests/level3/level3_chord_select.rs
    - biscuit-tui/cli/tests/level3/main.rs
    - biscuit-tui/justfile
    - darkmatter/lib/tests/l1/context_functions.rs
    - features/2026-09-22-consolidated-test-binaries-wave-2/measure/test-input-check.py
    - scripts/ci/consolidation.py
    - scripts/ci/test_consolidation.py
    - sniff/cli/Cargo.toml
    - sniff/cli/tests/l1/cli.rs
    - sniff/cli/tests/l1/cli_process_fixture.rs
    - sniff/cli/tests/l1/install_interview_cli.rs
    - sniff/cli/tests/l1/install_plan.rs
    - sniff/cli/tests/l1/main.rs
    - sniff/cli/tests/l1/snapshots.rs
    - sniff/cli/tests/l1/snapshots/
    - sniff/cli/tests/l1/spawn_site_guard.rs
    - sniff/cli/tests/l1/test_layout.rs
    - sniff/cli/tests/l1/tty.rs
    - sniff/cli/tests/level2/level2_cicd_styling.rs
    - sniff/cli/tests/level2/level2_git_status_styling.rs
    - sniff/cli/tests/level2/level2_perf_tree_rendering.rs
    - sniff/cli/tests/level2/level2_recent_commits_rendering.rs
    - sniff/cli/tests/level2/main.rs
    - tools/test-toolkit/tests/ci_workflow_contracts.rs
docs_updated_during_phase_5:
    - biscuit-tui/cli/README.md
    - docs/dependencies.md
    - biscuit-tui/lib/README.md
    - features/2026-09-22-consolidated-test-binaries-wave-2/plan.md
    - features/2026-09-22-consolidated-test-binaries-wave-2/implementation-log.md
    - features/2026-09-22-consolidated-test-binaries-wave-2/spec.md
    - features/2026-09-22-consolidated-test-binaries-wave-2/measurements.md
docs_created_during_phase_5:
    - features/2026-09-22-consolidated-test-binaries-wave-2/sniff-cli-migration.json
    - features/2026-09-22-consolidated-test-binaries-wave-2/biscuit-tui-cli-migration.json
    - features/2026-09-22-consolidated-test-binaries-wave-2/metadata-check-all-ten.txt
    - features/2026-09-22-consolidated-test-binaries-wave-2/sniff-cli/
    - features/2026-09-22-consolidated-test-binaries-wave-2/biscuit-tui-cli/
    - features/2026-09-22-consolidated-test-binaries-wave-2/measure/sniff-cli-before.txt
    - features/2026-09-22-consolidated-test-binaries-wave-2/measure/sniff-cli-after.txt
    - features/2026-09-22-consolidated-test-binaries-wave-2/measure/biscuit-tui-cli-before.txt
    - features/2026-09-22-consolidated-test-binaries-wave-2/measure/biscuit-tui-cli-after.txt
skills_files_updated_during_phase_5:
    - .claude/skills/os/windows.md
    - .claude/skills/rust-testing/SKILL.md
    - .claude/skills/sniff/SKILL.md
source_files_during_phase_6:
    - claudine/lib/src/stream/protocol/kimi/tests.rs
    - darkmatter/dmls/tests/common/mod.rs
    - features/2026-09-22-consolidated-test-binaries-wave-2/baseline/consumer-sweep.py
    - features/2026-09-22-consolidated-test-binaries-wave-2/acceptance/final-sweep.sh
docs_updated_during_phase_6:
    - features/2026-09-22-consolidated-test-binaries-wave-2/plan.md
    - features/2026-09-22-consolidated-test-binaries-wave-2/spec.md
    - features/2026-09-22-consolidated-test-binaries-wave-2/implementation-log.md
docs_created_during_phase_6:
    - features/2026-09-22-consolidated-test-binaries-wave-2/acceptance.md
    - features/2026-09-22-consolidated-test-binaries-wave-2/ci-observations.md
    - features/2026-09-22-consolidated-test-binaries-wave-2/workspace-metadata.txt
    - features/2026-09-22-consolidated-test-binaries-wave-2/baseline/test-selector-consumers-after.md
    - features/2026-09-22-consolidated-test-binaries-wave-2/acceptance/final-sweep-logs/
skills_files_updated_during_phase_6:
    - .claude/skills/rust-testing/SKILL.md
    - .claude/skills/darkmatter/SKILL.md
source_code:
    - features/2026-09-22-consolidated-test-binaries-wave-2/baseline/pre-existing.sh
    - features/2026-09-22-consolidated-test-binaries-wave-2/baseline/test-input-probe.py
    - features/2026-09-22-consolidated-test-binaries-wave-2/baseline/test-input-listings.py
    - features/2026-09-22-consolidated-test-binaries-wave-2/baseline/consumer-sweep.py
    - features/2026-09-22-consolidated-test-binaries-wave-2/spikes/s2-scan.py
    - scripts/ci/consolidation.py
    - scripts/ci/test_consolidation.py
    - features/2026-09-22-consolidated-test-binaries-wave-2/selfproof/mutation-check.py
    - tree-hugger/lib/Cargo.toml
    - claudine/lib/Cargo.toml
    - sniff/lib/Cargo.toml
    - .config/nextest.toml
    - Cargo.lock
    - sniff/lib/benches/support/bench_ids.rs
    - claudine/lib/tests/l1/agent_errors_fleet.rs
    - claudine/lib/tests/l1/boundary_lint.rs
    - claudine/lib/tests/l1/canonical_dispatch.rs
    - claudine/lib/tests/l1/deprecated_compatibility.rs
    - claudine/lib/tests/l1/diagnostic_detail_conformance.rs
    - claudine/lib/tests/l1/kimi_wire.rs
    - claudine/lib/tests/l1/lifecycle_control_flow_spike.rs
    - claudine/lib/tests/l1/main.rs
    - claudine/lib/tests/l1/model_catalog_integration.rs
    - claudine/lib/tests/l1/opencode_stderr_lifecycle.rs
    - claudine/lib/tests/l1/protocol_fixture_replay.rs
    - claudine/lib/tests/l1/semantic_fidelity.rs
    - claudine/lib/tests/l1/strict_mode_provenance_spike.rs
    - claudine/lib/tests/l1/test_layout.rs
    - claudine/lib/tests/l1/tts_phase1_contract.rs
    - claudine/lib/tests/l1/tts_phase5_contract.rs
    - claudine/lib/tests/l1/typed_stream_protocols.rs
    - sniff/lib/tests/l1/bench_fixtures.rs
    - sniff/lib/tests/l1/bench_ids_sync.rs
    - sniff/lib/tests/l1/bench_plans.rs
    - sniff/lib/tests/l1/benchmark_workloads.rs
    - sniff/lib/tests/l1/focused_provider.rs
    - sniff/lib/tests/l1/git_parity.rs
    - sniff/lib/tests/l1/host_capability_cache.rs
    - sniff/lib/tests/l1/integration.rs
    - sniff/lib/tests/l1/main.rs
    - sniff/lib/tests/l1/merge_conflict_prediction.rs
    - sniff/lib/tests/l1/network_primitives.rs
    - sniff/lib/tests/l1/program_installable.rs
    - sniff/lib/tests/l1/program_serialization.rs
    - sniff/lib/tests/l1/recent_commits.rs
    - sniff/lib/tests/l1/remote_observation.rs
    - sniff/lib/tests/l1/remote_providers.rs
    - sniff/lib/tests/l1/remote_resolution.rs
    - sniff/lib/tests/l1/test_layout.rs
    - sniff/lib/tests/l1/uv_with_install_plan.rs
    - sniff/lib/tests/l1/windows_app_paths_orphan.rs
    - sniff/lib/tests/l1/windows_find_program_priority.rs
    - tree-hugger/lib/tests/l1/adapter_tests.rs
    - tree-hugger/lib/tests/l1/cache_tests.rs
    - tree-hugger/lib/tests/l1/corpus_tests.rs
    - tree-hugger/lib/tests/l1/lint_diagnostics.rs
    - tree-hugger/lib/tests/l1/main.rs
    - tree-hugger/lib/tests/l1/phase1_diagnostics.rs
    - tree-hugger/lib/tests/l1/phase6_neovim_query_reuse.rs
    - tree-hugger/lib/tests/l1/query_compile.rs
    - tree-hugger/lib/tests/l1/resolver_tests.rs
    - tree-hugger/lib/tests/l1/test_layout.rs
    - tree-hugger/lib/tests/l1/tree_file.rs
    - tree-hugger/lib/tests/l1/tree_package.rs
    - features/2026-09-22-consolidated-test-binaries-wave-2/measure/measure.sh
    - features/2026-09-22-consolidated-test-binaries-wave-2/measure/layout-guard.sh
    - features/2026-09-22-consolidated-test-binaries-wave-2/measure/test-input-check.py
    - biscuit-file/lib/Cargo.toml
    - biscuit-file/lib/tests/l1-fetch/fetch_integration.rs
    - biscuit-file/lib/tests/l1-fetch/main.rs
    - biscuit-file/lib/tests/l1/completion_round_trip.rs
    - biscuit-file/lib/tests/l1/detailed_resolution.rs
    - biscuit-file/lib/tests/l1/finalized_reference_resolution.rs
    - biscuit-file/lib/tests/l1/implicit_relative.rs
    - biscuit-file/lib/tests/l1/main.rs
    - biscuit-file/lib/tests/l1/parse_count.rs
    - biscuit-file/lib/tests/l1/precedence_flip.rs
    - biscuit-file/lib/tests/l1/reference_grammar.rs
    - biscuit-file/lib/tests/l1/repository_scope_catalog.rs
    - biscuit-file/lib/tests/l1/resolution_context.rs
    - biscuit-file/lib/tests/l1/round_trip.rs
    - biscuit-file/lib/tests/l1/span_compat.rs
    - biscuit-file/lib/tests/l1/test_layout.rs
    - biscuit-file/lib/tests/l1/yaml_corpus.rs
    - biscuit-file/lib/tests/l1/yaml_mutation.rs
    - biscuit-file/lib/tests/l1/yaml_safety.rs
    - biscuit-file/lib/tests/proptest-regressions/yaml_mutation.txt
    - biscuit-terminal/cli/Cargo.toml
    - biscuit-terminal/cli/tests/l1/about.rs
    - biscuit-terminal/cli/tests/l1/dir_targets.rs
    - biscuit-terminal/cli/tests/l1/integration_test.rs
    - biscuit-terminal/cli/tests/l1/main.rs
    - biscuit-terminal/cli/tests/l1/snapshots/l1__integration_test__columns_snapshot.snap
    - biscuit-terminal/cli/tests/l1/snapshots/l1__integration_test__list_snapshot.snap
    - biscuit-terminal/cli/tests/l1/snapshots/l1__integration_test__padleft_snapshot.snap
    - biscuit-terminal/cli/tests/l1/snapshots/l1__integration_test__padright_snapshot.snap
    - biscuit-terminal/cli/tests/l1/snapshots/l1__integration_test__prose_snapshot.snap
    - biscuit-terminal/cli/tests/l1/snapshots/l1__integration_test__prose_styled_snapshot.snap
    - biscuit-terminal/cli/tests/l1/snapshots/l1__integration_test__quote_snapshot.snap
    - biscuit-terminal/cli/tests/l1/test_layout.rs
    - biscuit-terminal/cli/tests/level2/diagrams.rs
    - biscuit-terminal/cli/tests/level2/level2_apple_terminal_prose.rs
    - biscuit-terminal/cli/tests/level2/level2_container_fenced_code.rs
    - biscuit-terminal/cli/tests/level2/level2_cursor_and_hygiene.rs
    - biscuit-terminal/cli/tests/level2/level2_image.rs
    - biscuit-terminal/cli/tests/level2/level2_layout.rs
    - biscuit-terminal/cli/tests/level2/level2_prose_styling.rs
    - biscuit-terminal/cli/tests/level2/level2_render_tree_style.rs
    - biscuit-terminal/cli/tests/level2/level2_status_block.rs
    - biscuit-terminal/cli/tests/level2/level2_style_everywhere_matrix.rs
    - biscuit-terminal/cli/tests/level2/main.rs
    - biscuit-terminal/cli/tests/level2/prose_cells.rs
    - biscuit-test-harness/src/bin/broker.rs
    - claudine/gen/Cargo.toml
    - claudine/gen/tests/l1/agent_errors_check.rs
    - claudine/gen/tests/l1/drift.rs
    - claudine/gen/tests/l1/fixtures_provenance.rs
    - claudine/gen/tests/l1/generate_ux.rs
    - claudine/gen/tests/l1/main.rs
    - claudine/gen/tests/l1/pipeline.rs
    - claudine/gen/tests/l1/registry_coverage.rs
    - claudine/gen/tests/l1/signals_sidecar_mirror.rs
    - claudine/gen/tests/l1/signals_validation.rs
    - claudine/gen/tests/l1/steering_check.rs
    - claudine/gen/tests/l1/test_layout.rs
    - claudine/gen/tests/l1/vocabulary.rs
    - claudine/gen/tests/level2/level2_report_terminal.rs
    - claudine/gen/tests/level2/main.rs
    - claudine/lib/src/provider/tests.rs
    - darkmatter/dmls/Cargo.toml
    - darkmatter/dmls/src/overlay/expressions.rs
    - darkmatter/dmls/tests/fixtures/editor_neovim/init.lua
    - darkmatter/dmls/tests/fixtures/editor_neovim/probe.lua
    - darkmatter/dmls/tests/l1/level1_graph_index.rs
    - darkmatter/dmls/tests/l1/level1_wiki.rs
    - darkmatter/dmls/tests/l1/lsp_session.rs
    - darkmatter/dmls/tests/l1/main.rs
    - darkmatter/dmls/tests/l1/mapping_only_corpus.rs
    - darkmatter/dmls/tests/l1/no_side_effects.rs
    - darkmatter/dmls/tests/l1/packaging_contract.rs
    - darkmatter/dmls/tests/l1/stdio_subprocess.rs
    - darkmatter/dmls/tests/l1/strict_mode_recovery_spike.rs
    - darkmatter/dmls/tests/l1/suggest_constraint_phase1.rs
    - darkmatter/dmls/tests/l1/test_layout.rs
    - darkmatter/dmls/tests/l1/zed_extension_contract.rs
    - darkmatter/dmls/tests/level2/level2_editor_neovim.rs
    - darkmatter/dmls/tests/level2/main.rs
    - schematic/gen/Cargo.toml
    - schematic/gen/tests/l1/artifact_drift.rs
    - schematic/gen/tests/l1/e2e_generation.rs
    - schematic/gen/tests/l1/http_client.rs
    - schematic/gen/tests/l1/main.rs
    - schematic/gen/tests/l1/openapi_import_test.rs
    - schematic/gen/tests/l1/openapi_strict_completeness.rs
    - schematic/gen/tests/l1/path_substitution.rs
    - schematic/gen/tests/l1/postman_artifact_validation.rs
    - schematic/gen/tests/l1/postman_golden.rs
    - schematic/gen/tests/l1/postman_schema.rs
    - schematic/gen/tests/l1/postman_var_consistency.rs
    - schematic/gen/tests/l1/query_param_detection.rs
    - schematic/gen/tests/l1/query_params_codegen.rs
    - schematic/gen/tests/l1/test_layout.rs
    - schematic/gen/tests/l1/ws_codegen.rs
    - schematic/gen/tests/level2/main.rs
    - schematic/gen/tests/level2/terminal_capture.rs
    - schematic/justfile
    - biscuit-tui/cli/Cargo.toml
    - biscuit-tui/cli/tests/l1/boolean_switch_output.rs
    - biscuit-tui/cli/tests/l1/choose_cli.rs
    - biscuit-tui/cli/tests/l1/choose_many_output.rs
    - biscuit-tui/cli/tests/l1/choose_one_output.rs
    - biscuit-tui/cli/tests/l1/completions.rs
    - biscuit-tui/cli/tests/l1/completions_shell.rs
    - biscuit-tui/cli/tests/l1/exit_codes.rs
    - biscuit-tui/cli/tests/l1/help_contract.rs
    - biscuit-tui/cli/tests/l1/input_table_output.rs
    - biscuit-tui/cli/tests/l1/keyboard_protocol.rs
    - biscuit-tui/cli/tests/l1/main.rs
    - biscuit-tui/cli/tests/l1/test_layout.rs
    - biscuit-tui/cli/tests/l1/text_area_input_output.rs
    - biscuit-tui/cli/tests/l1/text_input_output.rs
    - biscuit-tui/cli/tests/level2/main.rs
    - biscuit-tui/cli/tests/level2/terminal_render.rs
    - biscuit-tui/cli/tests/level2/windows_captured_stdout.rs
    - biscuit-tui/cli/tests/level3/level3_chord_select.rs
    - biscuit-tui/cli/tests/level3/main.rs
    - biscuit-tui/justfile
    - darkmatter/lib/tests/l1/context_functions.rs
    - sniff/cli/Cargo.toml
    - sniff/cli/tests/l1/cli.rs
    - sniff/cli/tests/l1/cli_process_fixture.rs
    - sniff/cli/tests/l1/install_interview_cli.rs
    - sniff/cli/tests/l1/install_plan.rs
    - sniff/cli/tests/l1/main.rs
    - sniff/cli/tests/l1/snapshots.rs
    - sniff/cli/tests/l1/snapshots/
    - sniff/cli/tests/l1/spawn_site_guard.rs
    - sniff/cli/tests/l1/test_layout.rs
    - sniff/cli/tests/l1/tty.rs
    - sniff/cli/tests/level2/level2_cicd_styling.rs
    - sniff/cli/tests/level2/level2_git_status_styling.rs
    - sniff/cli/tests/level2/level2_perf_tree_rendering.rs
    - sniff/cli/tests/level2/level2_recent_commits_rendering.rs
    - sniff/cli/tests/level2/main.rs
    - tools/test-toolkit/tests/ci_workflow_contracts.rs
    - claudine/lib/src/stream/protocol/kimi/tests.rs
    - darkmatter/dmls/tests/common/mod.rs
    - features/2026-09-22-consolidated-test-binaries-wave-2/acceptance/final-sweep.sh
documentation:
    - features/2026-09-22-consolidated-test-binaries-wave-2/plan.md
    - features/2026-09-22-consolidated-test-binaries-wave-2/spec.md
    - features/2026-09-22-consolidated-test-binaries-wave-2/rulings.md
    - features/2026-09-22-consolidated-test-binaries-wave-2/implementation-log.md
    - features/2026-09-22-consolidated-test-binaries-wave-2/spikes/s1-feature-sets.md
    - features/2026-09-22-consolidated-test-binaries-wave-2/spikes/s2-hazards.md
    - features/2026-09-22-consolidated-test-binaries-wave-2/spikes/s2-scan-output.txt
    - features/2026-09-22-consolidated-test-binaries-wave-2/spikes/s3-cross-check.md
    - features/2026-09-22-consolidated-test-binaries-wave-2/spikes/s4-deps.md
    - features/2026-09-22-consolidated-test-binaries-wave-2/baseline/pre-existing.md
    - features/2026-09-22-consolidated-test-binaries-wave-2/baseline/skip-baseline.md
    - features/2026-09-22-consolidated-test-binaries-wave-2/baseline/test-inputs.md
    - features/2026-09-22-consolidated-test-binaries-wave-2/baseline/test-selector-consumers.md
    - darkmatter/fixes/_unscheduled/proptest-regressions-after-consolidation/spec.md
    - .claude/skills/os/build-hosts.md
    - features/2026-09-22-consolidated-test-binaries-wave-2/selfproof/README.md
    - features/2026-09-22-consolidated-test-binaries-wave-2/selfproof/dry-run.md
    - features/2026-09-22-consolidated-test-binaries-wave-2/selfproof/noop-comparison.md
    - features/2026-09-22-consolidated-test-binaries-wave-2/selfproof/noop-comparison.json
    - features/2026-09-22-consolidated-test-binaries-wave-2/selfproof/inventory.md
    - features/2026-09-22-consolidated-test-binaries-wave-2/selfproof/inventory.json
    - features/2026-09-22-consolidated-test-binaries-wave-2/selfproof/mutation-check.txt
    - features/2026-09-22-consolidated-test-binaries-wave-2/selfproof/r18-proptest-scratch.txt
    - features/2026-09-22-consolidated-test-binaries-wave-2/selfproof/wave1-metadata-check.txt
    - features/2026-09-22-consolidated-test-binaries-wave-2/selfproof/wave1-darkmatter-proptest-check.txt
    - features/2026-09-22-consolidated-test-binaries-wave-2/selfproof/capture-b.SHA256SUMS
    - features/2026-09-22-consolidated-test-binaries-wave-2/selfproof/capture-a/
    - tree-hugger/lib/README.md
    - docs/comment-quality.md
    - docs/dependencies.md
    - features/2026-09-22-consolidated-test-binaries-wave-2/measurements.md
    - features/2026-09-22-consolidated-test-binaries-wave-2/tree-hugger-migration.json
    - features/2026-09-22-consolidated-test-binaries-wave-2/claudine-migration.json
    - features/2026-09-22-consolidated-test-binaries-wave-2/sniff-migration.json
    - features/2026-09-22-consolidated-test-binaries-wave-2/tree-hugger/
    - features/2026-09-22-consolidated-test-binaries-wave-2/claudine/
    - features/2026-09-22-consolidated-test-binaries-wave-2/sniff/
    - features/2026-09-22-consolidated-test-binaries-wave-2/measure/tree-hugger-before.txt
    - features/2026-09-22-consolidated-test-binaries-wave-2/measure/tree-hugger-before-edit.log
    - features/2026-09-22-consolidated-test-binaries-wave-2/measure/tree-hugger-after.txt
    - features/2026-09-22-consolidated-test-binaries-wave-2/measure/tree-hugger-after-edit.log
    - features/2026-09-22-consolidated-test-binaries-wave-2/measure/claudine-before.txt
    - features/2026-09-22-consolidated-test-binaries-wave-2/measure/claudine-before-edit.log
    - features/2026-09-22-consolidated-test-binaries-wave-2/measure/claudine-after.txt
    - features/2026-09-22-consolidated-test-binaries-wave-2/measure/claudine-after-edit.log
    - features/2026-09-22-consolidated-test-binaries-wave-2/measure/sniff-before.txt
    - features/2026-09-22-consolidated-test-binaries-wave-2/measure/sniff-before-edit.log
    - features/2026-09-22-consolidated-test-binaries-wave-2/measure/sniff-after.txt
    - features/2026-09-22-consolidated-test-binaries-wave-2/measure/sniff-after-edit.log
    - .claude/skills/sniff/network.md
    - .claude/skills/tree-hugger/query-system.md
    - biscuit-file/docs/dependencies.md
    - biscuit-terminal/docs/dependencies.md
    - biscuit-test-harness/README.md
    - claudine/docs/dependencies.md
    - claudine/docs/research/signals/fixtures/README.md
    - claudine/docs/research/signals/fixtures/provenance.yaml
    - claudine/docs/topics/provider-metadata.md
    - darkmatter/dmls/README.md
    - darkmatter/dmls/docs/editors/smoke-checklist.md
    - darkmatter/dmls/docs/features.md
    - darkmatter/dmls/docs/hover.md
    - renderable/docs/layout-and-style.md
    - schematic/docs/dependencies.md
    - schematic/gen/tests/fixtures/postman/README.md
    - features/2026-09-22-consolidated-test-binaries-wave-2/biscuit-file-migration.json
    - features/2026-09-22-consolidated-test-binaries-wave-2/biscuit-file/
    - features/2026-09-22-consolidated-test-binaries-wave-2/biscuit-terminal-cli-migration.json
    - features/2026-09-22-consolidated-test-binaries-wave-2/biscuit-terminal-cli/
    - features/2026-09-22-consolidated-test-binaries-wave-2/claudine-gen-migration.json
    - features/2026-09-22-consolidated-test-binaries-wave-2/claudine-gen/
    - features/2026-09-22-consolidated-test-binaries-wave-2/dmls-migration.json
    - features/2026-09-22-consolidated-test-binaries-wave-2/dmls/
    - features/2026-09-22-consolidated-test-binaries-wave-2/measure/biscuit-file-after.txt
    - features/2026-09-22-consolidated-test-binaries-wave-2/measure/biscuit-file-before.txt
    - features/2026-09-22-consolidated-test-binaries-wave-2/measure/biscuit-terminal-cli-after.txt
    - features/2026-09-22-consolidated-test-binaries-wave-2/measure/biscuit-terminal-cli-before.txt
    - features/2026-09-22-consolidated-test-binaries-wave-2/measure/claudine-gen-after.txt
    - features/2026-09-22-consolidated-test-binaries-wave-2/measure/claudine-gen-before.txt
    - features/2026-09-22-consolidated-test-binaries-wave-2/measure/dmls-after.txt
    - features/2026-09-22-consolidated-test-binaries-wave-2/measure/dmls-before.txt
    - features/2026-09-22-consolidated-test-binaries-wave-2/measure/schematic-gen-after.txt
    - features/2026-09-22-consolidated-test-binaries-wave-2/measure/schematic-gen-before.txt
    - features/2026-09-22-consolidated-test-binaries-wave-2/schematic-gen-migration.json
    - features/2026-09-22-consolidated-test-binaries-wave-2/schematic-gen/
    - .claude/skills/biscuit-test-harness/SKILL.md
    - .claude/skills/os/macos.md
    - .claude/skills/schematic-define/SKILL.md
    - biscuit-tui/cli/README.md
    - biscuit-tui/lib/README.md
    - features/2026-09-22-consolidated-test-binaries-wave-2/sniff-cli-migration.json
    - features/2026-09-22-consolidated-test-binaries-wave-2/biscuit-tui-cli-migration.json
    - features/2026-09-22-consolidated-test-binaries-wave-2/metadata-check-all-ten.txt
    - features/2026-09-22-consolidated-test-binaries-wave-2/sniff-cli/
    - features/2026-09-22-consolidated-test-binaries-wave-2/biscuit-tui-cli/
    - features/2026-09-22-consolidated-test-binaries-wave-2/measure/sniff-cli-before.txt
    - features/2026-09-22-consolidated-test-binaries-wave-2/measure/sniff-cli-after.txt
    - features/2026-09-22-consolidated-test-binaries-wave-2/measure/biscuit-tui-cli-before.txt
    - features/2026-09-22-consolidated-test-binaries-wave-2/measure/biscuit-tui-cli-after.txt
    - .claude/skills/os/windows.md
    - .claude/skills/rust-testing/SKILL.md
    - .claude/skills/sniff/SKILL.md
    - features/2026-09-22-consolidated-test-binaries-wave-2/acceptance.md
    - features/2026-09-22-consolidated-test-binaries-wave-2/ci-observations.md
    - features/2026-09-22-consolidated-test-binaries-wave-2/workspace-metadata.txt
    - features/2026-09-22-consolidated-test-binaries-wave-2/baseline/test-selector-consumers-after.md
    - features/2026-09-22-consolidated-test-binaries-wave-2/acceptance/final-sweep-logs/
    - .claude/skills/darkmatter/SKILL.md
completed_phase: 6
implemented: true
implementation_1: "2026-09-23T16:27:50-07:00"
---

# Implementation Log for 2026-09-22-consolidated-test-binaries-wave-2 (6 phases)

## Phase 1

Phase 1 (rulings, spikes, baselines) ran at `208051f75`. No production source
changed. The spec `status` moved `draft-spec → planned`. Nothing was
committed, per the phase instructions.

### What was produced

| Plan task | Output | Result |
|---|---|---|
| Record rulings | `rulings.md` | R1–R13 decided, plus new R14–R19 from Phase 1 evidence |
| S1 | `spikes/s1-feature-sets.md` | Census matches the plan: 236 workspace targets, 136 in scope. `dmls` needs a 4th feature set, `(terminal-tests)`. `claudine` is `()`. |
| S2 | `spikes/s2-hazards.md`, `s2-scan.py`, `s2-scan-output.txt` | 144 files scanned with `consolidation.py`'s detectors. Zero crate-global constructs or crate-only inner attributes. 95 path repairs, 2 self-exec identities, 3 `crate::` false positives, a proptest relocation hazard, and a cross-package guard. Done by a subagent; I verified the proptest and guard claims independently. |
| S3 | `spikes/s3-cross-check.md` | `cross-check` ships the working tree (not `HEAD`) and builds the CI feature union, so `windows_captured_stdout` is compiled. Per-test PASS lines appear live only. |
| S4 | `spikes/s4-deps.md` | No cycle. Five packages need an unconditional `test-toolkit` dev-dependency, not two. The added crates are all already in `Cargo.lock`. |
| Pre-existing state | `baseline/pre-existing.md`, `pre-existing.sh`, `pre-existing-logs/` | 8 areas × `test`, `test-l2`, `lint`, `check-tier-coverage`, plus `check-canonical`: all green except 2 environment-induced `claudine-cli` failures. 0 stranded tests. |
| Skip baseline | `baseline/skip-baseline.md` | `ci-baseline.toml` has no entries at all |
| Test-input probes | `baseline/test-inputs.md`, `test-input-probe.py`, `test-input-listings.py`, `*-before.json` | 9 probes, 1 per package (`biscuit-tui-cli` has 0 references). Identities are confirmed derived from the `mod` walk. |
| Consumer sweep | `baseline/test-selector-consumers.md`, `consumer-sweep.py` | 27 active selector hits and 41 file-path references. One is load-bearing and crosses packages (R17). Done by a subagent. |

### Findings that changed the plan (written into the plan's table and F9–F14)

- **F9 / R14:** all four `sniff-cli` `level2_*` files are
  `#![cfg(feature = "test-fixtures")]`, including
  `level2_recent_commits_rendering`. So spec hazard 3's "no features"
  contract is empty without the feature. `sniff-cli` becomes 2 targets, and
  the in-scope total is 18. The author may restore the split before Phase 5.
- **F10 / R7:** `schematic-gen`, `biscuit-terminal-cli`, and `claudine-gen`
  have `test-toolkit` only as an optional dependency, so their feature-less
  `l1` layout gate needs an extra unconditional dev-dependency.
- **F11 / R18:** proptest 1.11 `SourceParallel` resolves to
  `tests/proptest-regressions/<stem>.txt` once `tests/<target>/main.rs`
  exists. I verified this from `proptest-1.11.0/src/test_runner/failure_persistence/file.rs:77-81`
  and `:336-367`. **This is a wave-1 defect:** darkmatter's 5 seeds under
  `darkmatter/lib/tests/l1/*.proptest-regressions` are no longer replayed,
  and `darkmatter/lib/tests/proptest-regressions/` does not exist. Filed as
  `darkmatter/fixes/_unscheduled/proptest-regressions-after-consolidation/spec.md`.
- **F12 / R17:** `tools/test-toolkit/tests/ci_workflow_contracts.rs:7053`
  reads `windows_captured_stdout.rs` by path, and `:7067` asserts its inner
  `#![cfg(windows)]`. R4's disposition is therefore an outer cfg on the `mod`
  line **and** the inner attribute kept in the file.
- **F13 / R15:** `test_inputs.py`'s `NON_L1_BINARIES` never narrows into a
  `level2*` binary. That already applies to F3's old targets, and
  `biscuit-tui-cli` has no references, so nothing new is hidden. No tool
  change.
- **F14 / R13, R16:** `cross-check` ships the working tree. Remote legs must
  finish before the next package's structural edit starts. Windows evidence
  is `tee`d PASS lines, and `--features` must never be passed.

### Pre-existing failures and gaps (not attributable to the migration)

- `claudine-cli::l1 shipped_prompt_route_drift::{fixture_body_matches_the_shipped_body, shipped_implement_prompts_have_not_drifted_from_their_fixture}`
  fail because this worktree carries the author's uncommitted
  `prompts/_implement/*.md` edits. `claudine-cli` is outside the ten. With
  `--no-fail-fast`, 7316 passed and 2 failed.
- F3's 49 L1-tier tests in `biscuit-terminal-cli` run in no local recipe;
  they run only in CI's L1 cell. F5's `windows_captured_stdout` runs only on
  Windows with `terminal-tests`. Neither counts as "stranded" under
  `check-tier-coverage`. Both must be preserved exactly.

### Other things noticed (out of scope, not acted on)

- `claudine/cli/claudine/cli/tests/snapshots/` is a tracked, doubly nested
  directory holding two `wrap_commands__*.snap` files. It looks like a stray
  insta write. It is outside the ten packages.
- The `os` skill's `build-hosts.md` said `cross-check` routes feature flags
  into the archive build. The code does the opposite: any build flag switches
  every host to native mode. **Drift was detected, the code was taken as
  correct, and the skill was corrected.**

### Gates run in this phase

- `just test`, `just test-l2`, `just lint`, and `just check-tier-coverage <area>`
  for all eight areas, plus `just check-canonical` over the eight. Results are
  in `baseline/pre-existing.md`. This phase changed no Rust, justfile, or
  manifest, so these runs are both the baseline and this phase's own gate.
- `python3 -m py_compile` over the five new evidence scripts.
- The three frontmatters (spec, plan, log) parse as YAML.
- There is no requirement-to-test mapping, because Phase 1 changes no
  behavior. Its outputs are evidence documents, and each claim cites file
  and line or a committed log.

## Phase 2

Phase 2 (toolkit extension) ran at `9d44e7988`. No file in the ten packages
changed. Nothing was committed, per the phase instructions. R12's signed
toolkit commit is left to the separate commit step.

### What changed in `scripts/ci/consolidation.py`

| Plan task / ruling | Change |
|---|---|
| Package tables (R1) | `PACKAGES` gained the ten packages. `PACKAGE_FEATURE_SETS` holds S1's reviewed sets, including `dmls`'s fourth set `(terminal-tests)`. `capture` refuses an unlisted package **before** any subprocess (it used to run `cargo metadata` first), and `inventory --package <unlisted>` now refuses too (exit 2). |
| `move` (R2, R17, R18) | Ports `pilot/apply-move.py`, generalized: it refuses before touching anything (missing source, existing destination, undeclared module, unresolved `mod`). It resolves every `include_str!`/`include_bytes!`/`#[path]` literal against the old location and rewrites it only when the resolution would change, which also covers nested-root children. `mod common;`, bare or `#[path]` to `tests/common/mod.rs`, becomes `use crate::common;` with its other attributes kept. Any other bare `mod x;` gains a `#[path]` to its old file (sniff's `fixtures`). The row flag `keep_inner_cfg` keeps the inner cfg (R17). Roots declare `common` only where a module uses it, and list modules in rustfmt order (byte order, verified against `rustfmt --edition 2024`). Seed files move to `proptest_regression_path` (R18). It prints the `[[test]]` entries for `Cargo.toml`. |
| `check-proptest` (R18) | New. Seed content must survive byte for byte, and every seed file must be the resolution of some test source. A file proptest never reads fails. |
| `check-metadata` | Ports `acceptance/metadata-check.py` with `--manifest` (repeatable) and `--metadata`. Output on wave 1's four manifests is identical to the original's. |
| `body-diff` | Ports `acceptance/body-diff.py` with `--manifest`, `--base-rev`, `--after-rev` (default: the working tree), and `--markdown`. It now pairs nested-root children, and counts a line that differs only by a path literal as structural. **It exits 1** on any `other` line or a missing side, because wave 2 allows structural edits only. |
| `plan`: `RULED_TARGETS` (R4, R14, R19; new finding F16) | A reviewed table places `schematic-gen`'s `terminal_capture` and `biscuit-tui-cli`'s `real_terminal_render` and `windows_captured_stdout` in `level2`, places `sniff-cli`'s `level2_recent_commits_rendering` in `level2` (`test-fixtures`), and rules `sniff`'s `fixtures` a helper. `plan` fails if an entry names no target, if a helper lists tests, if the target would compile where it never did, or if it joins a target with more features while listing tests without them. |
| `plan`/`compare`: shared `common` tests (new finding F15) | Tests inside the shared `common` module are recorded as `shared_tests` and not projected per module. `compare` folds their copies into the consolidated identity and fails if the copies disagree. |

### Self-proof (all in `selfproof/`; see its `README.md`)

- **No-op:** `capture` ran twice for the ten packages (21 feature sets).
  `compare --require-identical-digests` gave `identical`: 0 failures and 0
  notes across 487 selector cells, and the two runs were byte-identical.
  `capture-a/` is committed as the before-side for Phases 3–5, with
  `inventory.json`.
- **`plan` over `capture-a`** gives exactly the ruled table: 18 targets; the
  aliases `prose_cells`, `diagrams`, and `terminal_render` only; R10's two
  rewrites; and `sniff` with 19 modules plus the `fixtures` helper.
  `ShippedWave2PlanTests` pins this against the frozen evidence and the
  filters each capture recorded.
- **`move` dry run** of all ten packages in a throwaway worktree: 0 `other`
  body lines, 0 `check-proptest` failures, and the `biscuit-file` seed
  relocated. The 7 `check-attributes` failures are exactly the S2/R19
  dispositions. `tree-hugger` and `biscuit-file` were then built and compared
  against `capture-a`: `identical` (`dry-run.md`).
- **R18:** a scratch crate with a failing property wrote
  `tests/proptest-regressions/always_fails.txt`, exactly as predicted.
  `check-proptest` is red on wave 1's darkmatter seeds (the filed defect).
- **Red then green:** `mutation-check.py` gave 17 of 17 `OK`. Wave 1's
  `mutation-check.py` still gives 8 of 8.

### Requirement-to-test mapping (`scripts/ci/test_consolidation.py`, 86 → 96 tests)

| Behavior | Tests |
|---|---|
| Ruled feature sets; unlisted package refused (capture and inventory) | `PackageTableTests.*`; shipped: `ShippedPackageTableTests` (each wave-2 set against the real `Cargo.toml`, CI union listed) |
| proptest path resolution | `ProptestPathTests` |
| `move`: moves, path repairs (include, `#[path]`, bare `mod`, nested child), `common` rewrite, alias as module name, rustfmt order, common only where used, `keep_inner_cfg`, seed relocation, `[[test]]` entries, refusal leaves the tree untouched, CLI | `MoveTests.*` (10 tests), including `test_a_generated_root_is_already_rustfmt_clean` and `test_the_moved_tree_passes_every_after_check` |
| `check-proptest` | `CheckProptestTests.*`: the wave-1 defect shape, changed or lost seeds, correct relocation |
| `check-metadata` | `CheckMetadataTests.*`: missing target, extra target, feature mismatch, missing `autotests`, double mapping, unknown target, CLI exit codes; shipped: `ShippedWave1MetadataTests` |
| `body-diff` | `BodyDiffTests.*`: structural-only passes, body change fails with the line, missing side fails |
| `RULED_TARGETS`, `shared_tests` in `plan` | `SharedAndRuledPlanTests.*` (5 tests); shipped: `ShippedWave2PlanTests` (3 tests) |
| Shared-copy fold in `compare` | `CompareTests.test_shared_common_copies_fold_into_the_consolidated_identity`, `test_shared_copies_that_disagree_fail` |

### Gates run

- `python3 scripts/ci/test_consolidation.py`: 96 tests, OK. No shipped class
  skipped.
- `just ci-local`'s Python leg (its ten suites, run as `ci-local` runs them):
  all pass.
- Lint: the repository has no Python lint recipe, and `scripts/` is not a
  `just` area, so `just lint` (Rust areas) does not cover this surface. Run
  instead: `python3 -W error -m py_compile` and
  `uvx ruff check --select F,E9,B` on the three changed Python files, both
  clean. The base file was clean under the same rules; my three findings
  (B905 ×2, B023) were fixed.
- No Rust, manifest, or justfile changed, so area `just test`/`just lint` are
  unaffected by this phase. Phase 1's baseline stands.

### Findings (written into the plan as F15–F17)

- **F15:** `biscuit-terminal-cli`'s `tests/common/pane_geometry.rs` has 10 unit
  tests. Each of 9 `level2_*` binaries compiles a copy (90 identities). They
  are L1-tier and run in CI's L1 cell with `terminal-tests`. Phase 1 (F3, S2)
  missed them because they are not in a target file. Under one `common` per
  root they fold to 10. **Open for Phase 4:** record the fold as that
  package's disposition (recommended: it follows first-spec §3 and
  `rust-testing`'s "declared once" rule, while wave 1's per-module
  `parity_helpers` copies are labeled "legacy shape, not a pattern"), or keep
  per-module copies. The toolkit supports the fold. Keeping copies would need
  `plan` to project `common::` tests per module again.
- **F16:** without `RULED_TARGETS`, `plan` gave `schematic-gen: l1-terminal`,
  `biscuit-tui-cli: l1-terminal + real`, and
  `sniff-cli: level2 + level2-test-fixtures`, which contradict R4 and R14.
- **F17:** wave 1's `claudine-cli` row in `PACKAGE_FEATURE_SETS` is stale. Its
  CI union now includes `test-fixtures`, so `capture --package claudine-cli`
  refuses. It is outside scope and was not changed. `ShippedPackageTableTests`
  checks wave-2 rows only, and says why.
- **S2 miscount:** S2 says 12 `#[path]` sites in scope. There are 11 (its
  `sniff` row says ×7 and lists 6). All 11 are handled.
- **Still manual in Phases 3–5** (the mover does not guess these):
  - `Cargo.toml`, including removing stale `[[test]]` entries such as
    `biscuit-file`'s `fetch_integration`;
  - `dispositions` for the 7 `check-attributes` hits;
  - `keep_inner_cfg: true` on `biscuit-tui-cli`'s `windows_captured_stdout`
    row (R17);
  - the snapshot moves;
  - the `spawn_site_guard` key; and
  - the self-exec `--exact` strings.

## Phase 3

Phase 3 (single-contract packages) ran at `c02e691d9` on the macOS dev host.
It migrated `tree-hugger` (10 → 1), `claudine` (15 → 1), and `sniff` (20 → 1)
into one `l1` binary each. Nothing was committed, per the phase instructions.
R12's commit series (manifest and evidence, structural move, area docs) is
left to the separate commit step, one package at a time.

### Before-side

- Fresh macOS captures (SPP 1) are byte-identical to `selfproof/capture-a`
  (`compare --require-identical-digests`: identical for all three). Nothing in
  the three crates, their justfiles, or `.config/nextest.toml` changed between
  `9d44e7988` and `c02e691d9`.
- Linux before-captures came from a private `--shared` scratch clone on
  `build-linux` (`os` skill procedure): the standing clone at `7685d1ac2`,
  plus a bundle `7685d1ac2..HEAD`, checked out at `c02e691d9`, with
  `RUSTC_WRAPPER="" KACHE_AUTO=0`. One base serves all three packages because
  they share it. The clone, bundle, and patch were deleted afterward.
- Each `plan` output equals the one from `capture-a` (checked for `sniff` with
  a JSON diff). No alias was emitted: the two `*_spike` targets carry no tier
  marker, as projected.

### Per package

| Step | `tree-hugger` | `claudine` | `sniff` |
|---|---|---|---|
| `move` | 10 modules, 41 `include_str!` repairs in 2 files | 15 modules, 2 repairs (`#[path]` into `darkmatter/features/…/spike/model.rs`, one fixture literal) | 19 modules, 6 bench-support `#[path]` repairs, `integration`'s `mod fixtures;` → `#[path = "../fixtures.rs"]` (R19) |
| `Cargo.toml` | `autotests = false`, `[[test]] l1`, **new** unconditional `test-toolkit` dev-dependency (R7) | `autotests = false`, `[[test]] l1` (`test-toolkit` was already a dev-dependency) | `autotests = false`, `[[test]] l1` (already had `test-toolkit`) |
| Module `cfg`s | none | none | `feature = "remote"` ×2, `feature = "network"`, `target_os = "windows"` ×2, all carried onto the `mod` lines |
| Layout gate | `tests/l1/test_layout.rs`, ≥ 12 files | `tests/l1/test_layout.rs`, ≥ 17 files | `tests/l1/test_layout.rs`, ≥ 22 files, names `tests/fixtures.rs` and both Windows modules |
| Red/green | `layout-guard.md`: red (stray), red (orphan), green | same | same |
| `check-attributes` / `check-proptest` / `check-metadata` | 0 failures (2 reviewed path-sensitive files, both compile) / clean / PASS | 0 / clean / PASS | 0 / clean / PASS |
| `body-diff` (other lines) | 0 (82 structural) | 0 (4 structural) | 0 (23 structural, 2 comment) |
| `compare` darwin / linux / darwin-linux | identical / identical / identical | identical / identical / identical | identical / identical / identical (all 3 feature sets; platform-absent evaluated) |
| Test-input probe | 89 → 89 (`binary_id(tree-hugger::l1) & test(/^tree_file::/)`) | 7 + 1 → 7 + 1 | 3 → 3 |
| Overrides | none | `claudine-l1` group (`override-ci-1`): identical 4342-test identity set | R10 rewrite in `.config/nextest.toml`: `override-rewrite.txt` shows the new filter matches exactly 1 test and the old spelling 0; `sniff-windows-l1` (`override-ci-2`) identical |
| Windows (`cross-check`) | 484/484 | 4300/4302: 2 lib unit-test failures (below); every `claudine::l1` test passed | 1937/1937, **both Windows-only modules named PASS** |
| WSL2 (`cross-check`) | 484/484 | 4343/4343 | 1946/1946 |
| Linux L1 (scratch clone) | 484/484 | 4339/4343: 4 lib unit-test failures, proven pre-existing (below) | 1946/1946 (`remote`) |

The compare runs report `override-ci-3 was rewritten` as a note wherever the
after capture used the new `.config/nextest.toml`. That is R10's rewrite, and
the identity it selects is unchanged.

### Hazards and dispositions

- **`claudine` `boundary_lint`**: not a path-keyed test guard. It reads
  production files by fixed paths under `claudine/lib/src`, `claudine/cli/src`,
  and `darkmatter/lib/src`, and never scans `tests/`, so the move changes its
  scan set by nothing. There is no `guard-scans-after.md` for any Phase 3
  package, because none has a path guard over test files.
- **`claudine-cli`'s `test_placement.rs`** walks only `claudine/cli/tests` for
  the layout rule, and it runs only when `claudine-cli` is tested. So it was
  not extended: `claudine` got its own gate in its own `l1`. Otherwise a
  `claudine`-only change would never run it.
- **`sniff` Windows-only files**: `check-attributes` confirmed both
  `#![cfg(target_os = "windows")]` moved onto the declarations. The Windows
  leg names both tests as run (`sniff/cross-check-windows.txt`).
- **`sniff` `bench_*`**: ordinary test targets. They moved like the rest.
- **`sniff` `real_` tests** stay inside L1 modules (R6). The area's
  `check-tier-coverage` reports 0 stranded.

### Suites (macOS; compared with `baseline/pre-existing.md`)

| Area | `just test` | `just test-l2` | `just lint` | `check-tier-coverage` | `check-canonical` |
|---|---|---|---|---|---|
| `tree-hugger` | ✅ 586 passed (585 + gate) | ✅ 3 | ✅ | ✅ 0 stranded | ✅ |
| `claudine` | 7317 passed, 2 failed: the **same two** pre-existing `claudine-cli::l1 shipped_prompt_route_drift` failures (baseline 7316 + 2) | ✅ 237 + 3 | ✅ | ✅ 0 stranded | ✅ |
| `sniff` | ✅ 2825 passed (2824 + gate) | ✅ 6 | ✅ | ✅ 0 stranded | ✅ |

All suites ran with `INSTA_UPDATE=no`. No `.snap.new` exists. Logs are
gzipped under each package's `suites/`.

### Failures not attributable to the migration

- **`claudine` on `build-linux`, 4 lib unit tests**
  (`composition::sequence::task::tests::group_framing::{a_members_body_output_lands_on_the_data_channel_not_the_status_one, a_serial_group_uses_the_invisible_bar_at_the_same_left_edge, every_member_task_opens_and_closes_exactly_one_stream, every_body_line_carries_its_own_tasks_bar}`).
  **Proven pre-existing**: with the patch stashed in the scratch clone (0
  dirty paths, `c02e691d9`), the same four fail (`claudine/linux-l1-summary.txt`).
  They pass on macOS and WSL2, so they look specific to that host's
  non-interactive SSH session.
- **`claudine` on native Windows, 2 lib unit tests**
  (`composition::schema::tests::shipped_implement_plan_prepares_with_unset_optional_commit_message`:
  expected `B:\…\src`, got `B:/…/src`;
  `composition::sequence::task::tests::side_effect_tasks::a_failed_nested_mapping_set_projects_one_path_and_commits_nothing`:
  an `unknown_root` error). **Not proven on the base.** Both live in the
  `claudine` lib-test binary, which is compiled only from `claudine/lib/src`
  and its dependencies. `git status claudine/lib/src` is empty, and the test
  targets the move changes are not part of that binary, so the move cannot
  reach them. A base run from a clean worktree
  (`claudine/cross-check-windows-base.txt`) first failed with `os error 1455`
  (paging file exhausted while the WSL guest on the same machine was
  building). The retry then failed on the poisoned `target\` (`E0463`). The
  lesson is now in the `os` skill (`build-hosts.md`, Remote-process hygiene).
  A later leg pruned the stale clone. **For the author:** both look like real
  Windows path-spelling defects in `claudine`. They are outside this feature's
  scope and are not filed.

### Docs (SPP 9)

- `tree-hugger/lib/README.md`: test paths now `tests/l1/…`, plus one sentence
  saying a new file must be declared in `tests/l1/main.rs`.
- `.claude/skills/tree-hugger/query-system.md`, `.claude/skills/sniff/network.md`:
  moved test paths.
- `docs/comment-quality.md:293`: `claudine/lib/tests/l1/canonical_dispatch.rs`.
- `sniff/lib/tests/l1/remote_providers.rs:11`: `--test remote_providers` →
  `--test l1 remote_providers::`. It selects the same 70 tests.
- `sniff/lib/benches/support/bench_ids.rs:7,11,74`: comment paths.
- `docs/dependencies.md`: `tree-hugger`'s dev-only `test-toolkit` (R7).
  `Cargo.lock` gains only that edge, and no new crate. `tree-hugger` has no
  area dependency doc, and none was created (R7).
- Left as history: `playa/docs/{plans,specs}/2026-04-29-*` (dated plans),
  `tree-hugger/reviews/…`, `sniff/docs/cli/repo_*.md` (sample CLI output of
  past commits), and `fixes/2026-09-22-test-input-blind-spot/spec.md:335`
  (another active spec's measurement record, which names
  `claudine::boundary_lint`). Module-name prose (`bench_ids_sync`,
  `protocol_fixture_replay`) is still correct.

### Measurements

See `measurements.md`. Test executables went from 45 to 3, and from
1,488.8 MB to 362.6 MB. The warm edit changed by +0.21 s, +0.38 s, and
+0.56 s, so R8's trigger was never reached.

### Deviations from the SPP, and why

- **No commits** (phase instructions). So R13's "remote legs against a
  committed revision" could not hold literally. Each `cross-check` shipped the
  working tree at its start. Every leg's banner names its synthetic revision.
  A leg's package does not depend on the other two packages' test files:
  `claudine` depends only on `sniff`'s library, and test targets are not part
  of a library build. So a later package's edits could not change an earlier
  leg's evidence. No structural edit started while a leg was being bundled.
- **One Linux before/after pair for all three packages**, taken from one
  scratch clone. The after patch covered `tree-hugger/lib`, `claudine/lib`,
  `sniff/lib`, and `.config/nextest.toml`. Cargo regenerated the same
  `Cargo.lock` edge there.
- **`cross-check` output was redirected, not `tee`d** for `claudine` and
  `sniff`. The files are the same evidence.

### Requirement-to-test mapping

| Changed behavior | Evidence / test |
|---|---|
| Every former test keeps its identity, tier, feature, and platform reachability | `compare` four-way, darwin/linux/darwin-linux, every selector and feature set (`<pkg>/comparison-*.md`) |
| A `tests/` file no root declares fails the build gate | `<pkg>::l1 test_layout::every_test_source_is_compiled_by_a_declared_target`, shown red on a stray root and an undeclared module, then green (`<pkg>/layout-guard.md`) |
| Windows-only modules still compile and run on Windows | `sniff/cross-check-windows.txt` PASS lines for `windows_app_paths_orphan::orphaned_hkcu_entry_is_filtered` and `windows_find_program_priority::path_wins_over_fallbacks_for_cmd` |
| CI's `threads-required` override still hits its test | `sniff/override-rewrite.txt` (1 match; old spelling 0) plus `compare`'s `override-ci-3` row |
| CI's test-input narrowing still selects the same tests | `<pkg>/test-inputs.md` (the planner's own index, re-run) |
| Manifests match Cargo | `check-metadata` PASS (`<pkg>/metadata-check.txt`) |
| Only structural edits | `body-diff` 0 `other` lines (`<pkg>/body-diff.md`) |

### New files (evidence tooling)

- `measure/measure.sh`: count, bytes, and warm edit (R8).
- `measure/layout-guard.sh`: red/green proof.
- `measure/test-input-check.py`: the after probe with the manifest mapping.

All three are reusable unchanged in Phases 4–5. `shellcheck` and
`ruff --select F,E9,B` are clean.

## Phase 4

Phase 4 (two-contract packages) ran at `5a396aeb5` on the macOS dev host, with
Phase 3's R12 series already committed. It migrated `biscuit-file` (15 → 2),
`schematic-gen` (14 → 2), `biscuit-terminal-cli` (14 → 2), `claudine-gen`
(11 → 2), and `dmls` (11 → 2), in the spec's order. Nothing was committed, per
the phase instructions. Each package's R12 series (manifest and evidence,
structural move, area docs) is left to the separate commit step. So is the
toolkit fix below, which R12 makes its own commit.

### Before-side

- Fresh macOS captures of all five packages were identical to
  `selfproof/capture-a` (`compare --require-identical-digests`). The only notes
  were Phase 3's `sniff` override rewrite.
- Linux before-captures came from one private `--shared` scratch clone on
  `build-linux`: the standing clone at `7685d1ac2`, plus a bundle
  `7685d1ac2..5a396aeb5`, with `RUSTC_WRAPPER="" KACHE_AUTO=0`.
- **Recaptured after `biscuit-terminal-cli`'s R10 rewrite.** `plan` refuses a
  capture whose recorded override filters differ from the live
  `.config/nextest.toml`. So `claudine-gen` and `dmls` were captured again on
  both hosts after that edit (Linux: the scratch clone plus that one file).
  Their "before" follows `biscuit-terminal-cli`'s move, which is R13's order
  anyway. Old and new captures compare identical, and the only notes are the
  two rewritten overrides.
- The before-measurements ran in a detached worktree of `5a396aeb5`, which was
  removed afterward.

### Per package

| Step | `biscuit-file` | `schematic-gen` | `biscuit-terminal-cli` | `claudine-gen` | `dmls` |
|---|---|---|---|---|---|
| Targets | `l1`, `l1-fetch` (`fetch`) | `l1`, `level2` (`terminal-tests`) | `l1`, `level2` (`terminal-tests`) | `l1`, `level2` (`terminal-tests`) | `l1`, `level2` (`terminal-tests`) |
| `plan` extras | none | `ruled_targets`: `terminal_capture` → `level2` (R4, F2) | aliases `prose_cells`, `diagrams` **derived by `plan`**; both R10 rewrites; 10 `shared_tests` (F15) | none | none (`level1_*` projected as non-markers) |
| `move` | 15 modules; seed `yaml_mutation.proptest-regressions` → `tests/proptest-regressions/yaml_mutation.txt` | 14 modules; 3 `include_bytes!` repairs | 14 modules; 9 `mod common;` → `use crate::common;`; `level2` declares `common` | 11 modules; `#![cfg(unix)]` → `#[cfg(unix)] mod level2_report_terminal;` | 11 modules; 4 `common` rewrites; 7 `include_str!` repairs |
| Manual edits | `Cargo.toml`; stale `fetch_integration` `[[test]]` removed | `Cargo.toml`; `test-e2e` recipe (R19) | `Cargo.toml`; 7 snapshots moved (`snapshot-mapping.json`); R10 in both profiles | `Cargo.toml`; `--exact` identity repair (R19) | `Cargo.toml`; `--exact` identity repair (R19) |
| `test-toolkit` dev-dep (R7) | new | new (optional one kept) | new (optional one kept) | new (optional one kept) | already present |
| Layout gate | ≥ 18 files | ≥ 17 | ≥ 19 (names `common/`) | ≥ 14 | ≥ 15 (names `common/`) |
| Red/green | `layout-guard.md`: red, red, green | same | same | same | same |
| `check-attributes` | 0 | 0 (2 `crate_path` dispositions, R19) | 0 | 0 (`crate_path` + `exact_path_string` dispositions) | 0 (`exact_arg` disposition) |
| `check-proptest` / `check-snapshots` | 1 seed relocated byte for byte / n/a | clean / n/a | clean / 7 of 7 | clean / n/a | clean / n/a |
| `check-metadata` | PASS | PASS | PASS | PASS | PASS |
| `body-diff` (other lines) | 0 (15 byte-identical) | 0 (6 structural, 4 comment) | 0 (18 structural) | 0 (4 structural, 7 comment) | 0 (24 structural) |
| `compare` darwin / linux / darwin-linux | identical ×3 | identical ×3 | identical ×3 (R10 notes only) | identical ×3 | identical ×3 (every set, both slow states) |
| Test-input probe | 13 → 13 | 14 → 14 (golden `complex_auth.json`) | 1 → 1 | 10 → 10 | 2 → 2 |
| Linux L1 (scratch clone, CI features) | 778/778 | 559/559 | 377/377 | 181/181 | 740/740 |

`check-metadata` over all eight migrated manifests together: PASS.

### Remote legs (R16: `cross-check`, no `--features`, one leg at a time)

| Package | Windows (`build-win-native`) | WSL2 |
|---|---|---|
| `biscuit-file` | 784/784 | 778/778 |
| `schematic-gen` | 559/559 | 559/559 |
| `biscuit-terminal-cli` | 374/374 | 377/377 |
| `claudine-gen` | 181/181 | 181/181 |
| `dmls` | 742/742 | 740/740 |

Every leg ran in archive mode with the package's CI feature union, and had 0
`FAIL` lines. The Windows and WSL2 legs never overlapped (`os` skill, 1455
lesson). The count differences are platform `cfg`s in files the move left
byte-identical. `biscuit-terminal-cli`'s three missing Windows tests are its
three `#[cfg(not(windows))]` tests (`about::test_about_warp_json_…`,
`integration_test::test_actual_terminal_query_integration`,
`integration_test::test_graph_expression_meta_outputs_render_metadata_in_a_pty`).
`claudine-gen`'s `level2` module is `#[cfg(unix)]`.

Each leg shipped the working tree as a synthetic commit over
`origin/fix/ci-build-feature-divergence` (`1879dd880`), named in its
`cross-check:` banner. No source file changed after the first leg started;
only evidence under this feature directory did. So the ten legs describe one
source tree. As in Phase 3, no receipt was published, because the tree is not
a committed head.

### Hazards and dispositions

- **F2 (`schematic-gen` `terminal_capture`)**: `plan`'s `ruled_targets` puts it
  in `level2` with `terminal-tests`. All three of its tests are `level2_*`, so
  the disposition is **moot, no L1 test**. `compare` confirms the L1 set is
  unchanged.
- **F3 (`biscuit-terminal-cli`)**: `plan` derived both aliases from the
  projection (its manifest records the tests that would have changed tier).
  `biscuit-terminal-cli/l1-tier-in-level2.txt` lists the L1-tier tests in the
  `level2` binary under `terminal-tests`: 59 = 43 `prose_cells` + 6 `diagrams`
  + 10 `common::pane_geometry`.
- **F15 (`common::pane_geometry`, 90 → 10)**: accepted as recommended. One
  `common` per root, so the nine per-binary copies fold into 10 `level2`
  identities, and `compare` checks the copies agreed.
- **R10**: `biscuit-terminal-cli/override-rewrite.txt` shows the new filter
  matching exactly one test in `default` and in `ci`, and the old spelling
  matching none.
- **R19 recipes**: `schematic/justfile` `test-e2e` →
  `--test l1 -- --ignored e2e_generation::`. It selects the same three ignored
  tests before and after (`schematic-gen/recipe-rewrite.txt`), and one run
  passed 3/3 (`test-e2e-run.log.gz`).
- **R19 `crate::` in string literals**: recorded as `crate_path` dispositions
  (`e2e_generation.rs:214`, `http_client.rs:471`, `pipeline.rs:166`).
- **R19 self-exec identity strings**:
  - `claudine-gen`: with the old string, both tmux tests fail (1 passed,
    2 failed); with the new one, 3/3 pass (`exact-{old,new}-string.log.gz`).
  - `dmls`: **the miss would be silent, not loud** (S2 predicted "fails
    loudly"). With the old string the consolidated binary selects 0 tests, the
    probe never starts, and `child_guard_reaps_process_during_unwind` still
    passes. Its `catch_unwind` swallows the "cancellation probe did not start"
    panic as if it were the simulated one. `dmls/exact-repair.txt` shows both
    sides with `--no-capture`. The repair is made. The test's inability to tell
    the two panics apart is a pre-existing weakness, outside this feature's
    structural-only scope (recorded for the author below).
- **`biscuit-file` `test-minimal`**: passes. It runs only lib `path_text` tests,
  so the move cannot reach it.
- **`fetch_integration`**: keeps exactly `required-features = ["fetch"]` as
  `l1-fetch`.

### Toolkit fix: `body-diff` and R19 identity repairs

`body-diff` exits 1 on any `other` line. R19 permits the self-exec `--exact`
repair as structural, but `body-diff` counted it as `other`, so `claudine-gen`
could not pass. `dmls` would have hit the same. Waiving the gate would have hidden
real body changes in those files. Instead, `body_diff` now masks the module's
own `<module>::` prefix on each line, **only** in files whose
identity-sensitive construct has a manifest disposition. Any other change in
those files still fails.

- `scripts/ci/consolidation.py`: `_body_normalize`, `diff_body`, `body_diff`,
  and the report header.
- `scripts/ci/test_consolidation.py` (96 → 99 tests), written red first:
  - `test_a_dispositioned_identity_repair_is_structural`;
  - `test_an_identity_repair_without_a_disposition_fails`;
  - `test_a_dispositioned_file_still_fails_on_a_body_change`.
- All ten `ci-local` Python suites pass. `py_compile -W error` and
  `ruff --select F,E9,B` are clean.

### Evidence tooling fix: `measure/measure.sh`

It counted 0 test executables for `dmls`. Cargo writes a package ID as
`…/darkmatter/dmls#0.1.0` when the directory name equals the package name, and
the script matched only `#dmls@`. It now matches both spellings (`shellcheck`
clean). `dmls` was re-measured.

### Suites (macOS; compared with `baseline/pre-existing.md`)

| Area | `just test` | `just test-l2` | `just lint` | `check-tier-coverage` | `check-canonical` |
|---|---|---|---|---|---|
| `biscuit-file` | ✅ 840 (839 + gate); `test-minimal` ✅ | n/a (stub) | ✅ | ✅ 0 stranded | ✅ |
| `schematic` | ✅ 1700 (1699 + gate) | ✅ 3 (tmux) | ✅ | ✅ 0 stranded | ✅ |
| `biscuit-terminal` | ✅ 3264 (3263 + gate) | ✅ 2 + 76 | ✅ | ✅ 0 stranded | ✅ |
| `claudine` | 7318 passed (7317 + gate), 2 failed: the **same two** pre-existing `claudine-cli::l1 shipped_prompt_route_drift` failures | ✅ 237 + 3 | ✅ | ✅ 0 stranded | ✅ |
| `darkmatter` | ✅ 8491 (8490 + gate) | ✅ 18 + 69 + 3 | ✅ | ✅ 0 stranded | ✅ |

All suites ran with `INSTA_UPDATE=no`, and no `.snap.new` exists. The first
`claudine` `lint` and `check-tier-coverage` runs failed only because they
overlapped the `dmls` move, before `dmls/Cargo.toml` named the new roots. The
reruns above are clean. Logs are gzipped under each package's `suites/`.

**`biscuit-terminal-cli` L2 backends.** `just test-l2` passed 76/76, but a
missing backend skips while printing PASS. So the L2 set was run again with
`BISCUIT_TEST_REQUIRED_BACKENDS`:

- With `tmux,wezterm,apple-terminal` required: 75 passed, 1 failed.
  `level2_apple_terminal_harness_lifecycle` captured only the host's `bash-3.2$`
  prompt. It fails the same way in isolation on the unmigrated base (twice),
  and it passed inside the full run. This is the `os` skill's "shell prompt
  inside a captured L2 frame" (host state, not a repo defect).
- Kitty has no remote-control instance on this host. A background instance
  started without focus (`open -g`) gave a pane a few columns wide: 20 of 25
  failed after, and 19 of 25 on the unmigrated base, with a varying set. So
  Kitty gives no discriminating evidence here. Identity for all 25 Kitty tests
  is proven by `compare`. See `suites/test-l2-kitty-README.txt`.

### Docs (SPP 9)

- `schematic`: `artifact_drift.rs` and `postman_golden.rs` module docs,
  `tests/fixtures/postman/README.md`, `docs/dependencies.md`, and
  `.claude/skills/schematic-define/SKILL.md` (three `--test l1
  e2e_generation::…` commands).
- `biscuit-terminal-cli`: `biscuit-test-harness/README.md` and
  `.claude/skills/biscuit-test-harness/SKILL.md` (four commands),
  `biscuit-test-harness/src/bin/broker.rs:187` (comment),
  `renderable/docs/layout-and-style.md:405`, and
  `biscuit-terminal/docs/dependencies.md`. The old commands omitted
  `--features terminal-tests`, which that target has always required, so they
  already failed. The new ones name it, and they list the 15
  `level2_prose_styling::` tests.
- `claudine-gen`: `Cargo.toml` CI-policy comment,
  `docs/research/signals/fixtures/{README.md,provenance.yaml}` (a YAML comment;
  the test parses the file), `docs/topics/provider-metadata.md`,
  `lib/src/provider/tests.rs:183,254` (R17's prose reference), and
  `claudine/docs/dependencies.md`.
- `dmls`: `Cargo.toml` comment, `README.md`, `docs/{features,hover}.md`,
  `docs/editors/smoke-checklist.md`, `src/overlay/expressions.rs:1566`, and the
  header comments of `tests/fixtures/editor_neovim/{init,probe}.lua`.
- `biscuit-file`: `Cargo.toml` CI-policy comment, and an area
  `docs/dependencies.md` "Development Only" entry.
- Repository `docs/dependencies.md`: one line each for `biscuit-file`,
  `biscuit-terminal-cli`, `claudine-gen`, and `schematic-gen`. `Cargo.lock`
  gains only the dev edges and no new crate.
- Left as history: dated plans and reviews,
  `claudine/fixes/2026-09-17-remove-strict-mode/*`,
  `darkmatter/fixes/2026-09-20-alias-projection-port/spec.md:117-143` (another
  spec's record of test names), the `os` skill's dated `claudine-gen::steering_check`
  record, and each moved snapshot's `source:` header (the snapshots move byte
  for byte).

- `os` skill (`macos.md`): a background, minimized Kitty is not an L2 host
  (the Kitty finding above), and how to stop one.

### Measurements

See `measurements.md`. Test executables went from 65 to 10, and from
941.8 MB to 371.0 MB. The largest warm-edit change was +0.89 s
(`biscuit-file`), so R8's trigger was never reached.

### For the author (outside scope, not acted on)

- `schematic/justfile` `check-drift` (`cargo test -p schematic-gen
  artifact_drift -- --ignored`) selects **no test**, before and after,
  because none of `artifact_drift`'s tests is `#[ignore]`
  (`schematic-gen/recipe-rewrite.txt`). S2 called it "still works". The drift
  tests do run in ordinary L1.
- `dmls` `child_guard_reaps_process_during_unwind` cannot tell a probe that
  never started from the simulated panic (see above).

### Deviations from the SPP, and why

- **No commits** (phase instructions). As in Phase 3, the remote legs ran
  against the working tree. R13's "N+1's structural move waits for N's remote
  legs" was relaxed the same way Phase 3 relaxed it. The five packages share
  no test file, and every leg ran after the last structural edit, so each leg
  saw the final tree for its package.
- **One Linux before/after pair for all five packages**, from one scratch
  clone. `claudine-gen` and `dmls` were re-captured before their plan (see
  Before-side). The after patch was built with a temporary `GIT_INDEX_FILE`
  (120 paths). The clone, bundle, and patch were deleted afterward.
- **`cross-check` output was redirected, not `tee`d.** The files are the same
  evidence.

### Requirement-to-test mapping

| Changed behavior | Evidence / test |
|---|---|
| Every former test keeps its identity, tier, feature, and platform reachability | `compare` four-way, darwin/linux/darwin-linux, every selector and feature set, both `dmls` slow states (`<pkg>/comparison-*.md`) |
| A `tests/` file no root declares fails the build gate | `<pkg>::l1 test_layout::every_test_source_is_compiled_by_a_declared_target`, red on a stray root and an undeclared module, then green (`<pkg>/layout-guard.md`) |
| F3's 49 unmarked tests stay L1 in CI's L1 cell | `biscuit-terminal-cli/l1-tier-in-level2.txt` (59 = 43 + 6 + 10), plus `compare`'s L1 row under `(terminal-tests)` |
| R10 override still hits its test | `biscuit-terminal-cli/override-rewrite.txt` (1 match per profile; old spelling 0) |
| Self-exec probes still select their probe | `claudine-gen/exact-{old,new}-string.log.gz` (red, then green); `dmls/exact-repair.txt` (0 tests, then 1) |
| Recipes that named an old target | `schematic-gen/recipe-rewrite.txt` (same 3 tests), `test-e2e-run.log.gz` (3/3) |
| Snapshots and proptest seeds are still read | `biscuit-terminal-cli/snapshot-check.json` (7/7, byte-identical, and `l1` passes); `biscuit-file/proptest-check.json` |
| CI's test-input narrowing selects the same tests | `<pkg>/test-inputs.md` |
| Manifests match Cargo | `check-metadata` PASS (`<pkg>/metadata-check.txt`; eight manifests together) |
| Only structural edits | `body-diff` 0 `other` lines (`<pkg>/body-diff.md`) |
| `body-diff` accepts exactly the dispositioned identity repair | `BodyDiffTests.test_a_dispositioned_identity_repair_is_structural`, `test_an_identity_repair_without_a_disposition_fails`, `test_a_dispositioned_file_still_fails_on_a_body_change` |
| Windows and WSL2 still build and pass | `<pkg>/cross-check-{windows,wsl}.txt` |

## Phase 5

Phase 5 (hazard packages) ran at `8255ee228` on the macOS dev host, with Phase 4's R12
series already committed. It migrated `sniff-cli` (11 → 2) and `biscuit-tui-cli` (15 → 3),
in the spec's order. Nothing was committed, per the phase instructions. Each package's R12
series (manifest and evidence, structural move, area docs) is left to the separate commit
step. So are the two toolkit changes below, which R12 makes their own commit, landing before
`sniff-cli`'s evidence commit (its `body-diff` needs the first one).

### Before-side

- Fresh macOS captures of both packages compare identical to `selfproof/capture-a`
  (`compare --require-identical-digests`). The only notes are Phases 3–4's three R10
  override rewrites, which `plan` accepts because the captures were taken after them.
- Linux before-captures came from a private `--shared` scratch clone on `build-linux`: the
  standing clone at `7685d1ac2`, plus a bundle `7685d1ac2..8255ee228`, with
  `RUSTC_WRAPPER="" KACHE_AUTO=0`. The after side applied one patch (57 paths) built with a
  temporary `GIT_INDEX_FILE`. The clone, bundle, and patch were deleted afterward.
- The before-measurements ran in a detached worktree of `8255ee228`, removed afterward.
- **R14 held.** Without `test-fixtures`, `sniff-cli::level2_recent_commits_rendering` lists
  0 tests on both hosts, and `plan`'s inventory records its inner
  `#![cfg(feature = "test-fixtures")]` as the file's leading attribute, so it gates every
  item. The ruled two-target shape stands.

### Per package

| Step | `sniff-cli` | `biscuit-tui-cli` |
|---|---|---|
| Targets | `l1`, `level2` (`test-fixtures`) | `l1`, `level2` (`terminal-tests`), `level3` (`terminal-tests`) |
| `plan` extras | `ruled_targets`: `level2_recent_commits_rendering` → `level2` (R14) | alias `real_terminal_render` → `terminal_render` **derived by `plan`** (5 `real` verdicts would change); `ruled_targets`: `real_terminal_render`, `windows_captured_stdout` → `level2` (R4) |
| `move` | 11 modules; 10 `mod common;` → `use crate::common;` (`tty`'s keeps `#[cfg(unix)]`); `#[path]` to `../common/source_scan.rs`; 4 `test-fixtures` cfgs on the `level2` declarations | 15 modules; `#[cfg(unix)] mod terminal_render;`, `#[cfg(windows)] mod windows_captured_stdout;` (inner cfg kept, `keep_inner_cfg`), `#[cfg(target_os = "macos")] mod level3_chord_select;` |
| Manual edits | `Cargo.toml`; 12 snapshots moved byte for byte (`snapshot-mapping.json`); `spawn_site_guard` key → `"l1/spawn_site_guard.rs"` (R19) | `Cargo.toml` (+ unconditional `biscuit-test-harness` dev-dependency, below); `test-pty` recipe (R19); `choose_cli`/`keyboard_protocol` file-local `common` removed; `#[allow(clippy::module_inception)]` on `mod keyboard_protocol;`; R17 path in `test-toolkit` |
| Layout gate | ≥ 16 files (names `common/source_scan.rs`) | ≥ 22 files (names all three roots and `common/real_terminal/mod.rs`) |
| Red/green | `layout-guard.md`: red, red, green | same |
| `check-attributes` | 0 | 0 (2 `crate_path` dispositions) |
| `check-proptest` / `check-snapshots` | clean / 12 of 12, byte-identical | clean / n/a |
| `check-metadata` | PASS (`level2` requires exactly `test-fixtures`, R14) | PASS |
| `body-diff` (other lines) | 0 (32 structural; needs the `path_key` disposition) | 0 (27 structural, 4 byte-identical) |
| `compare` darwin / linux / darwin-linux | identical ×3 | identical ×3 (platform-absent evaluated) |
| Test-input probe | 8 → 8 (renamed snapshot probe) | 0 → 0 (control; plus the new gate's own `Cargo.toml` row) |
| Linux L1 (scratch clone, CI features) | 857/857 | 412/412 |

`check-metadata` over all ten manifests together: PASS, 18 targets
(`metadata-check-all-ten.txt`).

### Remote legs (R16: `cross-check`, no `--features`, one leg at a time)

| Package | Windows (`build-win-native`) | WSL2 |
|---|---|---|
| `sniff-cli` | 853/853 | 857/857 |
| `biscuit-tui-cli` | 393/393, **`windows_captured_stdout` named PASS** | 412/412 |

Every leg ran in archive mode with the package's CI feature union, and had 0 `FAIL` lines.
The Windows and WSL2 legs never overlapped. `sniff-cli`'s Windows run lacks four Unix-only
integration tests (`tty::os_subcommand_runs_in_pty`, `install_plan::install_plan_force_rebuilds_cache`,
`install_plan::install_plan_populates_cache_file`, and
`cli_process_fixture::canonical_checkout_containment_rejects_symlink_spelling`), in files the
move changed only structurally. Every other test name appears in both runs. The `sniff-cli` legs ran before `biscuit-tui-cli`'s move started; no
`sniff-cli` file changed after them. The `biscuit-tui-cli` legs ran after its last source
edit. No leg published a receipt, because the tree is not a committed head.

### Hazards and dispositions

- **Spec hazard 3 / R14 (`sniff-cli`'s L2 contracts):** one `level2` target with exactly
  `required-features = ["test-fixtures"]`. The `()` comparison shows
  `level2_recent_commits_rendering` with 0 tests before and after on both hosts, and the
  `(test-fixtures)` comparison shows its 2 tests unchanged. The plan's "`level2` with no
  `required-features`" check is superseded by R14 (annotated in the plan).
- **`spawn_site_guard` (path-keyed guard):** `guard-scans-after.md` lists the scan set by
  path. The six former files are the same once their `l1/` directory is removed. The only
  additions are the two roots and the layout gate, which spawn nothing. Guard output is
  unchanged: 0 spawn sites, the same 4 PATH-escape sites at the same lines. **The old key
  would have failed silently, not loudly:** with it temporarily restored, the guard scans
  itself and still passes with identical totals, because its sanitizer blanks its own
  string-literal fixtures. The repair keeps the guard's stated rule.
- **`sniff-cli` snapshots:** `check-snapshots --emit-mapping`, then a byte-for-byte move
  (sha256 checked), then `--mapping`: 12 of 12. Every snapshot test passes under
  `INSTA_UPDATE=no`, and no `.snap.new` exists.
- **`sniff-windows-l1` (`override-ci-2`):** identical identity set in every comparison.
- **F4 (`real_terminal_render` → `terminal_render`):** `plan` derived the alias. The `real`
  selector picks 0 tests before and after on both hosts, so no test is newly selected by
  the stub `real` tier, and `check-tier-coverage biscuit-tui` reports 0 stranded. A `//`
  comment on the declaration says why the module is not named after its old target.
- **Spec hazard 2 (`level3_chord_select`):** keeps its name, since all four of its tests
  carry `level3_`. It stays `#[cfg(target_os = "macos")]` in `level3`, and its tests are
  listed only on darwin under `terminal-tests`, before and after. L3 was not run: it takes
  desktop focus and is opt-in only.
- **F5/R4 (`windows_captured_stdout`):** joins `level2` as a `#[cfg(windows)]` module and
  keeps its inner `#![cfg(windows)]` (R17). `windows-captured-stdout.md` shows it absent
  from every macOS and Linux listing, before and after, and the Windows leg's PASS line
  `biscuit-tui-cli::level2 windows_captured_stdout::captured_stdout_receives_only_value_no_tui_bytes`,
  selected by the L1 filter. That is spec criterion 1 for this hazard.
- **F6 (seven `level2_*` tests in feature-less files):** they stay in `l1` (R6), and
  `compare` shows the L1 and L2 sets unchanged (L2 under `()` still selects the same 7).
- **R17 (`test-toolkit` reads the moved file):**
  `tools/test-toolkit/tests/ci_workflow_contracts.rs:7053` now reads
  `biscuit-tui/cli/tests/level2/windows_captured_stdout.rs`. Its test passes, and
  `test-toolkit`'s whole L1 passes (347/347).
- **R19 `test-pty`:** `recipe-rewrite.txt` shows each rewritten line listing the same tests
  as before (4, 15, 4). One run of the recipe: `completions_shell::` 15/15 and
  `choose_cli::pty::` 4/4 pass. `keyboard_protocol::` has 3 of 4 failing, **the same three
  failing identically on the unmigrated base** (`test-pty-keyboard-base.log.gz`). They are
  env-gated (`RUN_PTY_TESTS=1`) and never run in CI. See "For the author".
- **`crate::common` in `choose_cli`/`keyboard_protocol`:** the mover turned each file's
  `#[path = "common/mod.rs"] mod common;` into `use crate::common;`. Both files name
  `common` only as `crate::common::pty::…`, so the import was unused (a warning, and an
  error under the lint's `-D warnings`). The import is removed. That matches S2 ("the
  files' own `mod common;` must be removed"). `crate::common` now resolves to the `l1`
  root's single copy. Both are recorded as `crate_path` dispositions.
- **`clippy::module_inception`:** `keyboard_protocol.rs` nests `mod keyboard_protocol { … }`.
  As a module named `keyboard_protocol`, that trips the lint under `just lint`. Renaming would
  change 2 test identities, so `#[allow(clippy::module_inception)]` goes on the declaration
  in `l1/main.rs`, with a comment saying why. The file body is untouched. The repository
  precedent is `biscuit-terminal/lib/src/components/{prose,table}/mod.rs`.
- **Layout gate without the harness:** `biscuit-tui-cli` had `biscuit-test-harness` only as
  an optional dependency behind `terminal-tests`, so the feature-less `l1` could not use
  `manifest_dir!()`. `env!("CARGO_MANIFEST_DIR")` names the producer's checkout in an
  archive run (`rust-testing`), so it adds an unconditional dev-dependency, keeping the
  optional one (R7's pattern). `Cargo.lock` is unchanged, because the edge already existed.
  It is documented in `Cargo.toml` and `docs/dependencies.md`. `biscuit-tui` has no area
  dependency doc, and none was created for one line (R7's `tree-hugger` precedent).

### Toolkit change 1: `body-diff` accepts a dispositioned path-key repair

`sniff-cli`'s R19 self-exclusion key (`"spawn_site_guard.rs"` → `"l1/spawn_site_guard.rs"`)
is structural by ruling, but `body-diff` counted it as `other` and exited 1. Waiving the gate
would hide real body changes in that file. Instead, following Phase 4's identity-repair
pattern:

- `scripts/ci/consolidation.py`: `PATH_KEY_DISPOSITION = "path_key"`, a disposition-only kind
  (no detector finds it). In a file with that disposition, `_body_normalize` masks exactly
  the module's own `"<target>/` prefix at the start of a string literal. `diff_body`,
  `body_diff`, and the report header carry it.
- `scripts/ci/test_consolidation.py` (99 → 103 tests), with the acceptance test written red
  first:
  - `test_a_dispositioned_path_key_repair_is_structural`;
  - `test_a_path_key_repair_without_a_disposition_fails`;
  - `test_a_path_key_disposition_rejects_another_targets_prefix`;
  - `test_a_path_key_disposition_still_fails_on_a_changed_path`.

### Toolkit change 2: the suite's host-tool gates use `tool_guard.require_tools`

`just test test-toolkit` failed one test that was **already failing at `8255ee228`**
(checked in the base worktree):
`ci_workflow_contracts::no_ci_python_suite_gates_a_host_tool_outside_the_shared_guard`. It
rejects `skipUnless(shutil.which(…))`, and Phase 2's `test_consolidation.py` (`ebd261004`)
had three such gates, plus two `skipUnless(JUST …)` gates that dodged the check only in
spelling. All five now call `require_tools(…, enforced_by=CI_LOCAL)`. `CI_LOCAL` names
`just ci-local`'s ci-infra self-tests, the only place the suite runs. Non-tool conditions
(the wave-1 manifests and the Phase 1 baseline) stay as plain `skipUnless`. On this host
nothing skips before or after. All ten ci-local Python suites pass, and `py_compile -W error`
and `ruff --select F,E9,B` are clean.

### Evidence tooling fix: `measure/test-input-check.py`

It assumed the probe file does not move. `sniff-cli`'s probe is a snapshot the move renames,
so the script takes an optional after-path (from `snapshot-mapping.json`). The probe also
enumerates `git ls-files`, and the moved files are untracked, so both packages' probes ran
against a temporary `GIT_INDEX_FILE` (the checkout's index was untouched). Both
`test-inputs.md` files say so.

### Suites (macOS; compared with `baseline/pre-existing.md`)

| Area | `just test` | `just test-l2` | `just lint` | `check-tier-coverage` | `check-canonical` |
|---|---|---|---|---|---|
| `sniff` | ✅ 2826 (2825 + gate) | ✅ 6 (tmux required: 6) | ✅ | ✅ 0 stranded | ✅ |
| `biscuit-tui` | ✅ 992 (991 + gate) | ✅ 21 (tmux + WezTerm required: 21) | ✅ (after the `module_inception` allow) | ✅ 0 stranded | ✅ |
| `test-toolkit` (R17) | ✅ 347 (`just test test-toolkit`, after change 2) | — | — | — | — |

All suites ran with `INSTA_UPDATE=no`, and no `.snap.new` exists. Kitty has no usable
instance on this host (`os` skill, `macos.md`), so it is not claimed. Identity for its tests
is proven by `compare`. Logs are gzipped under each package's `suites/`.

### Docs (SPP 9)

- `sniff-cli`: `.claude/skills/sniff/SKILL.md:206` and `.claude/skills/rust-testing/SKILL.md:777`
  (the guard path), and `darkmatter/lib/tests/l1/context_functions.rs:234` (a comment naming
  a `sniff-cli` test by file).
- `biscuit-tui-cli`: `cli/README.md` (harness paragraph and five commands, which now name
  `--features terminal-tests`, required by those targets all along; the old
  "Level 3 … `--test real_terminal_render`" command now points at `level3`, where the L3
  tests have lived since `36656dc34`), `lib/README.md:201`, the `choose_cli` and
  `completions_shell` run-hint comments, the `test-l3` recipe comment in
  `biscuit-tui/justfile`, and `Cargo.toml`'s `windows` dev-dependency comment.
- `os` skill (`windows.md`): the moved path, plus one sentence on why an L1 test lives in
  the `level2` binary (R4).
- Left as history: `.claude/skills/rust-testing/SKILL.md:140` (a past-tense account of the
  file's old `#[ignore]` era), dated reviews and plans under `features/` and `_completed/`,
  `tree-hugger/fixes/2026-08-03-portable-paths-at-machine-boundaries/spec.md:179` (a dated
  survey that also lists wave 1's old paths), and
  `darkmatter/features/_unscheduled/wezterm-sgr-race-test-fixes/spec.md:199` (another spec's
  record). `common/pty.rs:48` names `choose_cli.rs` and `keyboard_protocol.rs`, which still
  exist under those names.

### Measurements

See `measurements.md`. Test executables went from 26 to 5, and from 108.1 MB to 61.0 MB.
The largest warm-edit change was +0.08 s, so R8's trigger was never reached. Across the
wave's ten packages, the test executables went from 136 to 18.

### For the author (outside scope, not acted on)

- `biscuit-tui`'s `just test-pty`: three `keyboard_protocol` PTY tests
  (`bare_ctrl_shows_hotkey_badges`, `chord_fallback_still_works`,
  `dumb_terminal_chord_fallback_works`) fail on this host before and after the move. The
  child produces no selection output, or only reset sequences. They run only with
  `RUN_PTY_TESTS=1`, so nothing in CI sees them. The recipe also stops at its first failing
  line, so its other two lines never run after a failure.
- `spawn_site_guard`'s self-exclusion is untested: a wrong key passes silently (above).

### Deviations from the SPP, and why

- **No commits** (phase instructions). As in Phases 3–4, the remote legs ran against the
  working tree. `biscuit-tui-cli`'s structural move started after `sniff-cli`'s Windows leg
  had shipped its bundle and started building remotely, not after its commit. The two
  packages share no file, and no `sniff-cli` file changed after its legs.
- **One Linux before/after pair for both packages**, from one scratch clone.
- **Toolkit change 2 fixes a failure this feature introduced in Phase 2** and that Phase 4's
  suites did not run. It is in scope because the toolkit is this feature's, and
  `test-toolkit`'s L1 has to pass for R17.

### Requirement-to-test mapping

| Changed behavior | Evidence / test |
|---|---|
| Every former test keeps its identity, tier, feature, and platform reachability | `compare` four-way, darwin/linux/darwin-linux, every selector and both feature sets (`<pkg>/comparison-*.md`) |
| A `tests/` file no root declares fails the build gate | `<pkg>::l1 test_layout::every_test_source_is_compiled_by_a_declared_target`, red on a stray root and on an undeclared module, then green (`<pkg>/layout-guard.md`) |
| R14: one `level2` contract, with no test compiled where it was not before | `()` comparison (`level2_recent_commits_rendering`: 0 before and after), `check-metadata` |
| F4: no test enters the stub `real` tier | `real` selector 0 → 0 in every comparison; `check-tier-coverage biscuit-tui` 0 stranded |
| F5: the Windows-only L1 test still compiles and runs on Windows only | `biscuit-tui-cli/windows-captured-stdout.md` (absent on macOS and Linux; Windows PASS line) |
| R17: the cross-package reader still finds the file | `test-toolkit::ci_workflow_contracts the_windows_captured_stdout_test_is_discoverable_as_ordinary_l1`, and `test-toolkit` L1 347/347 |
| R19: the path-keyed guard scans the same files | `sniff-cli/guard-scans-after.md`; guard output identical (0 spawn sites, 4 escapes) |
| R19: the `test-pty` recipe selects the same tests | `biscuit-tui-cli/recipe-rewrite.txt` (4, 15, 4); `test-pty-run.log.gz` |
| Snapshots are still read | `sniff-cli/snapshot-check.json` (12/12, byte-identical); all 14 `snapshots::` tests pass |
| CI's test-input narrowing selects the same tests | `<pkg>/test-inputs.md` (8 → 8; 0 → 0) |
| Manifests match Cargo | `check-metadata` PASS per package and across all ten (`metadata-check-all-ten.txt`) |
| Only structural edits | `body-diff` 0 `other` lines (`<pkg>/body-diff.md`) |
| `body-diff` accepts exactly a dispositioned path-key repair | `BodyDiffTests.test_a_dispositioned_path_key_repair_is_structural`, `test_a_path_key_repair_without_a_disposition_fails`, `test_a_path_key_disposition_rejects_another_targets_prefix`, `test_a_path_key_disposition_still_fails_on_a_changed_path` |
| The suite's host-tool gates use the shared guard | `test-toolkit::ci_workflow_contracts no_ci_python_suite_gates_a_host_tool_outside_the_shared_guard` (red at `8255ee228`, green now) |
| Windows, WSL2, and Linux still build and pass | `<pkg>/cross-check-{windows,wsl}.txt`, `<pkg>/linux-l1-summary.txt` |

## Phase 6

Phase 6 (documentation, acceptance, and closeout) ran at `35295e442` on the macOS dev host.
By then the separate commit step had landed the whole R12 series for Phases 2–5: 58 commits,
every one with a good signature (`git log --format=%G?` is `G`). The only `E` entries in
`origin/fix/ci-build-feature-divergence..HEAD` are GitHub's merges of PRs #94 and #95, which
are not this feature's. No commit message has an agent-attribution trailer. Nothing in
Phase 6 was committed, per the phase instructions.

### Wave 1 — cross-cutting updates

**Active-doc sweep.** `baseline/consumer-sweep.py` gained `--after`, following wave 1's
script. Cargo no longer has the old per-file target names, so they come from the ten
`*-migration.json` manifests. Other packages keep their own targets, so a name another
package shares still marks a hit ambiguous. `--after` also adds:

- a record date for each historical file (its dated directory, else its last commit);
- a `self` summary by file, instead of 1,230 rows;
- `AFTER_DISPOSITIONS`, a reviewed table saying why each remaining active or in-flight hit
  stays. A hit without an entry prints as **undispositioned**.

The first run found 7 active hits, 3 in-flight hits, and 5 active file-path references:

| Hit | Disposition |
|---|---|
| `darkmatter/dmls/tests/common/mod.rs:2,5,26–28` called the three LSP modules "binaries" and "targets" | **Fixed** (comment-only). It now says "the `l1` integration binary's LSP modules". The `dead_code` rationale ("each consuming binary uses a different subset") no longer held in one binary, so it was reworded. The allow is still needed: removed, `cargo check -p dmls --tests` warns on `LspFixture::workspace` and `workspace_path`. |
| `claudine/lib/src/stream/protocol/kimi/tests.rs:8` "the `protocol_fixture_replay` integration test" | **Fixed** (comment-only): "the `l1` integration binary's `protocol_fixture_replay` module" |
| `.claude/skills/os/wsl.md:91` `claudine-gen::steering_check` | Historical: a dated account ("Proven non-vacuous on 2026-09-09") |
| `.github/workflows/sniff-performance.yml:102`, `claudine/docs/topics/performance-testing.md:134` "the `bench_ids_sync` integration test" | Accurate: it is still a test module, not a selector. Left unchanged, so no CI-infra cell is scheduled for a comment. The claudine doc's claim that this test covers claudine's benches is pre-existing drift, outside this feature |
| `.claude/skills/rust-testing/SKILL.md:140` (old `windows_captured_stdout.rs` path) | Historical: a past-tense account (Phase 5's call) |
| `sniff/docs/cli/repo_{recent-commits,source-code-changes}.md` (old `sniff/lib/tests/*.rs` paths) | Historical: sample command output quoting an old commit's file list |
| `claudine/fixes/2026-07-13-rendezvous-local-ipc/plan.md:565,826`, `fixes/2026-09-22-test-input-blind-spot/spec.md:335` | In-flight spec records; their authors own them at landing |

Result: `baseline/test-selector-consumers-after.md`. It has 0 `--test <old>` selectors
outside historical records and this feature's own evidence, 0 undispositioned hits, and 0
ambiguous active hits. It lists 912 + 500 historical hits in 185 files, each with its record
date. `py_compile -W error` and `ruff --select F,E9,B` are clean.

**Skill drift.**

- `rust-testing/SKILL.md`, "Consolidated Integration-Test Binaries": the section said only
  wave 1's four packages were consolidated ("Other packages still use Cargo's per-file
  discovery"). That was stale. It now names all fourteen packages and says
  `autotests = false` is the marker. It adds the rules this wave ruled:
  - target names, and where an L1 test that needs a feature goes (R3, R4, with
    `l1-fetch` and `windows_captured_stdout` as examples);
  - neutral aliases (R5: `terminal_render`, `prose_cells`);
  - proptest seeds' new location (R18).
- `darkmatter/SKILL.md`: three wave-1 test paths (`cli/tests/help.rs`,
  `lib/tests/semantic_results_never_persist.rs`,
  `lib/tests/dasherized_identifier_corpus.rs`) now point into `l1/`. Each target file and
  the named test exist.
- Area skills for the ten packages: a scan of every `tests/*.rs` path in the `sniff`,
  `claudine`, `biscuit-file`, `tree-hugger`, `biscuit-tui`, `biscuit-terminal`,
  `darkmatter`, `os`, and `rust-testing` skills found no other path that fails to exist.
  Exceptions:
  - `claudine/timeline.md:28` is a dated entry;
  - `rust-testing/SKILL.md:140` is historical;
  - `rust-testing/integration-tests.md` and `SKILL.md:609–610` are generic examples;
  - `rust-testing/wezterm-harness-pitfalls.md:121` names
    `darkmatter/cli/tests/level2_layout.rs`, which was deleted by `0ec6b3af5` before either
    wave. That is pre-existing drift, left alone.
- `os` skill: Phase 6 ran no remote leg, so it has no new Windows or WSL2 fact.

**Dependencies.** All five of R7's `test-toolkit` entries are in `docs/dependencies.md` and
are accurate against each `Cargo.toml`. So is Phase 5's `biscuit-tui-cli`
`biscuit-test-harness` line. The area docs (`biscuit-file`, `schematic`, `biscuit-terminal`,
`claudine`) carry theirs. No change was needed.

### Wave 2 — acceptance

- **Workspace metadata** (`workspace-metadata.txt`): 118 integration-test targets (236 at
  `208051f75`). No package has 10 or more. The largest are `biscuit-clipboard-cli` and
  `worktree-cli` (8 each). The ten packages hold 18 targets. This matches the plan's
  prediction exactly.
- **Final sweep** (`acceptance/final-sweep.sh`, the baseline script's recipes, with
  `INSTA_UPDATE=no`): all 40 runs pass except `claudine`'s `just test`. Its failures are the
  baseline's two `claudine-cli::l1 shipped_prompt_route_drift` tests. The `--no-fail-fast`
  re-run gives 7,318 passed and the same 2 failed. The cause is unchanged: the shipped
  prompt's `_test-tiers` transclusion, now committed as `9d44e7988`, without a fixture or pin
  refresh. Every L1 count is the baseline plus one layout gate per migrated package. Every
  L2 count equals the baseline. `check-tier-coverage` reports 0 stranded in all eight areas,
  and `check-canonical` passes 8 of 8. No `.snap.new` file exists. The sweep ran with the two
  comment edits above already in place, so both areas' `test` and `lint` cover them.
- **`acceptance.md`**: walks first-feature criteria 1–8 and 10–12 and new criteria 4–6, and
  links each one to its evidence. Criterion 10 is pending. `ci-observations.md` holds the
  table and the harvest procedure. No run was triggered: the branch is 60 commits ahead of
  its remote, and the latest `ci` run (`35900378816`) predates the migrations.
- **Status**: the spec is now `status: implemented`, `implemented: true`, and
  `implemented_by: claude/opus`. It stays in `features/`.
- **Plan checkboxes**: the Phase 2–5 "commit" boxes left open by earlier phases are now
  checked. Each note names the evidence and move commits, with good signatures.

### Requirement-to-test mapping

Phase 6 changes no behavior. The two source edits are comment-only. Each requirement maps to
a check that was run:

| Requirement | Check |
|---|---|
| No active `--test <old>` selector; every remaining hit has a disposition | `consumer-sweep.py --after` → `baseline/test-selector-consumers-after.md` (0 undispositioned) |
| The comment edits compile and lint clean | `darkmatter` and `claudine` `just test` / `just lint` in the final sweep |
| The `dead_code` allow in dmls `common` is still needed (so the reworded rationale is true) | `cargo check -p dmls --tests` with the allow removed: 2 warnings (then restored) |
| New criterion 5: no package with ≥ 10 targets | `cargo metadata` → `workspace-metadata.txt` |
| New criterion 6 and first-feature criterion 8 | `acceptance/final-sweep-logs/summary.tsv` and logs |
| Commits signed | `git log --format=%G?` over the branch's unpushed range |

Skipped: Level 3 (it takes focus and is opt-in) and Kitty L2 (no usable instance on this host).

Pre-existing failures:
- `claudine-cli`'s two `shipped_prompt_route_drift` tests (above).
- `claudine`'s two native-Windows lib tests (Phase 3). Phase 6 did not re-run them.

### Suggested commit split (R12; the commit step decides)

1. `docs(skills): …`: `.claude/skills/rust-testing/SKILL.md` and
   `.claude/skills/darkmatter/SKILL.md`.
2. `docs(darkmatter): …` and `docs(claudine): …` (comment-only):
   `darkmatter/dmls/tests/common/mod.rs` and
   `claudine/lib/src/stream/protocol/kimi/tests.rs`. They can be one comment-only commit.
3. `planning(repo): …`: this feature directory. That is `consumer-sweep.py --after`,
   `baseline/test-selector-consumers-after.md`, `workspace-metadata.txt`, `acceptance.md`,
   `acceptance/`, `ci-observations.md`, `plan.md`, `spec.md`, and this log.

### Handoff note for the reviewer

- **Read first:** `acceptance.md`. It is one page per criterion and links every artifact.
  The per-package evidence directories share one layout, described in R9.
- **What to check by reading, not by tool:**
  - Criterion 5: the `body-diff.md` "structural" lines. Each non-trivial one is a manifest
    disposition, listed in `acceptance.md` §5.
  - The three neutral aliases (R5).
  - The `rust-testing` skill's new paragraphs.
- **Pending:** first-feature criterion 10 (CI observations). Fill `ci-observations.md` from
  the first ordinary run that selects these packages.
- **For the author, outside scope:**
  - Refresh the `claudine-cli` shipped-prompt fixture and pin for `9d44e7988`.
  - `claudine`'s two Windows path-spelling unit tests.
  - `biscuit-tui`'s `test-pty` keyboard failures (Phase 5).
  - `spawn_site_guard`'s untested self-exclusion (Phase 5).
  - Phase 4's `schematic` `check-drift` and `dmls` `child_guard_reaps_process_during_unwind`
    notes.
  - The stale `wezterm-harness-pitfalls.md:121` path.
  - The `claudine` performance doc's `bench_ids_sync` step.
  - The `sniff-cli` move commit's subject says "3 binaries"; it has 2 (R14).
- **Not done by the agent, by rule:** moving the spec to `_completed`, or running
  `just complete`.

## Implementation of Review Findings #1

> **started at:** 2026-09-23T16:27:50-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/feat-dark-fixes/features/2026-09-22-consolidated-test-binaries-wave-2/review-1.md'
- this is iteration 1 of the review-to-implement cycle
- starting the work on 'The Claudine Level 1 area gate still fails' at 16:28:04
    - discovery: `git show 9d44e7988` adds only `::file ../_test-tiers.md` (after the spec NOTE block, line 121) to `prompts/_implement/implement-plan.md`; the fixture already carried the earlier `227e18c59` transclusion at the Completion section, so the shipped prompt now transcludes `_test-tiers.md` twice (lines 121 and 187) and the commit message's claim that the fixture "already mirrors the new line" is wrong
    - mirrored the line-121 transclusion into `claudine/cli/tests/fixtures/shipped_implement_route/_implement/implement-plan.md`; `diff` against the shipped file now shows only the documented `say:`/`effect:`/`shell:` removals
    - refreshed `claudine/cli/tests/fixtures/shipped_implement_route/shipped-hashes.json` via the documented `CLAUDINE_UPDATE_SHIPPED_PROMPT_HASHES=1 just test-cli shipped_prompt_route_drift::` recipe; the frontmatter hash is unchanged, and the body hash is now `9086e47509578e0a`, matching `md hash prompts/_implement/implement-plan.md`
    - completed: `just test` in `claudine/` is green: 7,320 passed, 0 failed, 9 skipped; `just lint` in `claudine/` exits 0 (its only warning is the macOS linker `__eh_frame` note)
    - updated `acceptance.md` criterion 8 (heading, `claudine` row, and the failure paragraph, which now records the refresh) and the `message_to_agent` known-failures sentence in `spec.md`
    - open for the author (not changed): the shipped `implement-plan.md` now transcludes `_test-tiers.md` twice (lines 121 and 187); dropping the line-187 copy would need the fixture and pin refreshed again
- work completed for 'The Claudine Level 1 area gate still fails' at 16:30:41
- starting the work on 'CI input narrowing assumes a level2 binary contains no Level 1 tests' at 16:30:41
    - discovery: a narrowed unit is always intersected with the L1 tier expression (`_tier_filter` ANDs `BISCUIT_TEST_NARROW` onto it), so a binary-wide unit in a mixed-tier binary cannot pull `level2_*` tests into an L1 cell. What the binary-name shortcut actually prevented was the opposite failure: narrowed cells run with `--no-tests=fail`, so a unit that selects nothing turns the cell red
    - discovery: a narrowed cell's package record takes `test_args` from `feature_args` (`[package.metadata.ci.tests]` `features`/`all-features`). Every live `level2` target's `required-features` is in its package's CI features today. `claudine-cli::real` and the two `real_provider` targets are not, and only the removed binary-name prefix kept them from getting units
    - completed: removed `NON_L1_BINARIES` from `scripts/ci/test_inputs.py`. `_unit` now reads tier from the test path alone, and returns `None` for a target whose `required-features` the CI feature contract (plus `default`, closed over the feature table) does not enable. That target's embeds still count as product source
    - completed: a module-wide reference is dropped when all the tests it can reach are outside L1. It can reach the tests under its module, or, for a helper module with no tests, every test in its binary. Only targets with such a reference are tokenized a second time. Planning one document costs +0.01–0.05 s; a scan of all 9,024 references costs +1.3 s
    - live delta over every tracked non-source file, old vs new: the only change is that 1,118 `darkmatter-cli::level2` `common::baseline`/`common::layout` references now get units. That binary holds L1 tests (`harness_integrity`), so this is correct. The `biscuit-terminal-cli` `level2_image`/`level2_cursor_and_hygiene` controls still resolve to no unit
    - completed: 8 regression tests in `scripts/ci/test_affected_scope.py`: 7 in `TestInputIndexTests` (a mixed-tier `level2` binary, a crate-root helper in it and in an all-Level 2 binary, required-features enabled or left off, and an embed in an unbuilt bin) plus 1 planner test in `TestInputSelectionTests`. 4 fail against the old module. Removing the helper-module check fails 2, and removing the `built_for_l1` check fails 1
    - completed: every `scripts/ci/test_*.py` suite passes (14 suites). `just _test repo-deps`: 449 passed. `ci_workflow_contracts::the_ci_documentation_states_the_implemented_behavior`, which reads the edited skill, passes. No Python linter is configured for `scripts/ci`
    - completed: added a paragraph to `docs/cicd/test-inputs.md`, a sentence to the `rust-testing` skill, and a dated superseded note to `baseline/test-inputs.md`. `rulings.md` R15 was left alone as a historical ruling
    - open (pre-existing, not changed): a helper module with no tests of its own, such as `common::layout`, gets `test(/^common::layout::/)`, which selects nothing. Its callers in other modules are not selected. Today another unit keeps those cells from being empty
- work completed for 'CI input narrowing assumes a level2 binary contains no Level 1 tests' at 16:45:05
- starting the work on 'Two consolidated crate roots describe the wrong test tier' at 16:45:05
    - discovery: `diagrams` and `prose_cells` are aliases of the former `level2_diagrams` and `level2_prose_cells` targets (`biscuit-terminal-cli-migration.json`), and `windows_captured_stdout` is an unmarked L1 module; the two `//!` headers wrongly called every module Level 2 and said each kept its old target name
    - the orchestrator made the edit directly, since the change is comment-only (19 lines)
    - rewrote `biscuit-tui/cli/tests/level2/main.rs` and `biscuit-terminal/cli/tests/level2/main.rs` crate-root docs: each binary states its `terminal-tests` feature contract, says tier selection follows the test path, and names its aliases
    - completed: `just test` in `biscuit-tui` (992 passed, 7 skipped) and `biscuit-terminal` (3,264 passed, 55 skipped); `just lint` is clean in both
- work completed for 'Two consolidated crate roots describe the wrong test tier' at 16:45:47

### Successful Completion

The implementation of review cycle 1 has completed successfully in about 20 minutes (16:27:50 to 16:45:47). During this implementation all 3 review findings were evaluated to see if they could be fixed as a part of this implementation cycle: 3 were fixed, 0 were deferred (see reasons below):

- no findings were deferred
- open item for the author, found while fixing finding 1 but outside its scope: `prompts/_implement/implement-plan.md` now transcludes `_test-tiers.md` twice (lines 121 and 187)
- existing gap noted while fixing finding 2 and left unchanged: a unit narrowed to a helper module that has no tests of its own (e.g. `common::layout`) selects nothing, so tests in other modules that call it are not selected; today another unit always keeps those cells from being empty

The files changed in this cycle are:

- `claudine/cli/tests/fixtures/shipped_implement_route/_implement/implement-plan.md`
- `claudine/cli/tests/fixtures/shipped_implement_route/shipped-hashes.json`
- `scripts/ci/test_inputs.py`
- `scripts/ci/test_affected_scope.py`
- `docs/cicd/test-inputs.md`
- `.claude/skills/rust-testing/SKILL.md`
- `biscuit-tui/cli/tests/level2/main.rs`
- `biscuit-terminal/cli/tests/level2/main.rs`
- `features/2026-09-22-consolidated-test-binaries-wave-2/acceptance.md`
- `features/2026-09-22-consolidated-test-binaries-wave-2/baseline/test-inputs.md`
- `features/2026-09-22-consolidated-test-binaries-wave-2/spec.md` (`message_to_agent`)

## Implementation of Review Findings #2

> **started at:** 2026-09-23T16:52:16-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/feat-dark-fixes/features/2026-09-22-consolidated-test-binaries-wave-2/review-2.md'
- this is iteration 2 of the review-to-implement cycle
- starting the work on 'CI input narrowing drops Level 1 callers of a shared helper' at 16:52:30
        - discovery: the module-scoped unit is unsound for any helper, not only one whose module tests are all Level 2. A `pub` helper, or a private one wrapped by a `pub` one, can be called from any module of its binary, so `test(/^render::/)` also misses `captured::uses_guide` when `render` holds L1 tests of its own. Only a call graph could narrow it, so the scan now gives every reference outside a test function a binary-wide unit
        - completed: `scripts/ci/test_inputs.py`: `_unit` returns `binary_id(<binary>)` for any reference outside a test function (the module-name tier check is gone too). `scan` drops the unit only when the binary holds no L1 test (`_has_l1_test`, replacing `_unreachable_from_l1`), so a unit still always selects at least one L1 test. This also closes the pre-existing `common::layout` gap from iteration 1
        - completed: 5 regression tests in `scripts/ci/test_affected_scope.py`, all failing against the pre-fix module. `TestInputIndexTests` gains review-2's shape (`render::guide()` reading `docs/guide.md`, `render::level2_uses_guide`, and the L1 caller `captured::uses_guide`), which now resolves to `binary_id(pkg::level2)`, and a helper in a binary with no tests, which gets no unit. `TestInputSelectionTests` gains the planner regression: a `docs/guide.md` change schedules one `ubuntu-latest` L1 cell. A new `narrowed_selection` helper evaluates the cell's `test_filter` against a listing, intersected with the L1 tier, and shows that it selects `captured::uses_guide` and nothing else
        - completed: 2 existing tests changed. `test_a_literal_in_a_helper_names_its_module` became `…_names_its_binary`: the unit is now the binary, and the fixture gained an L1 caller in another module, because a binary with no tests now gets no unit. `test_level2_tests_in_a_mixed_tier_binary_are_still_not_units` now expects `binary_id(pkg::level2)` for the `render` helper: the binary's L1 tests may call it
        - live delta over all 8,809 tracked non-source files (9,024 references, old vs new): no reference loses its unit. 5,617 module-scoped units become binary-wide, and 10 `biscuit-terminal-cli` references in `level2_cursor_and_hygiene` go from no unit to `binary_id(biscuit-terminal-cli::level2)`. The widest cells are unit-test helpers in the `darkmatter` (1,059 references, about 6,000 L1 tests) and `claudine` (24, about 4,200) library binaries. The cost is test run time in one Linux cell; the archive build is the same
        - completed: `cargo nextest list` with the L1 tier filter confirms both live widened units select tests: `binary_id(biscuit-terminal-cli::level2)` selects 59 and `binary_id(darkmatter-cli::level2)` selects 4 (`harness_integrity`)
        - completed: updated the unit description in `docs/cicd/test-inputs.md` and the `rust-testing` skill, and added a dated review-2 superseded note to `baseline/test-inputs.md`. The per-package `test-inputs.md` evidence tables and `fixes/2026-09-22-test-input-blind-spot/spec.md` still show `test(/^<module>::/)` units. They are left as historical records
