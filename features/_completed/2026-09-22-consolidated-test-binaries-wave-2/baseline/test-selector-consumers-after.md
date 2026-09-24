---
kind: evidence
feature: 2026-09-22-consolidated-test-binaries-wave-2
created: 2026-09-23
rev: 35295e442339eb1cec50424c79c3ffc1ce25ffa0
plan_phase: 6
generator: baseline/consumer-sweep.py --after
---

# Test-selector consumer re-sweep (after all ten migrations)

Every reference, in git-tracked text files, to an old per-file
integration-test target of the ten wave-2 packages, as recorded in the ten
`*-migration.json` manifests, in one of these forms:

- `--test <name>` (cargo test, cargo nextest, cargo check, just pass-through)
- nextest `binary(<name>)` (also `=`/`~` matchers)
- nextest `binary_id(<package>::<name>)`
- nextest binary id in prose, `<package>::<name>`
- prose: a backticked `<name>` on a line that says test binary / test target / integration test / binary

A line whose `-p`/`--package` (on the line or the `\`-continued line above)
names another package is dropped. A bare name that another workspace package
also uses is kept and listed under **ambiguous** unless the file sits in the
owning package's area. Prose naming a single-word target (`drift`,
`pipeline`, `fixtures`, `about`, `tty`, ...) and a bare `<package>::<name>`
for a dash-free package (`claudine::x` is also a Rust path) are listed under
**ambiguous**. Prose naming a name several packages share (`cli`,
`integration`) is not matched at all.

Classification rules, first match wins:

- this feature's own directory → **self**
- `(^|/)(features|fixes)/_complete(d)?/` → **historical** (completed spec record); never rewritten
- `(^|/)reviews?/` → **historical** (review record); never rewritten
- `(^|/)(implementation-log|review-log|CHANGELOG)[^/]*\.md$` → **historical** (dated log / changelog); never rewritten
- `(^|/)(features|fixes)/[^/]+/(review-\d+|log|[^/]*results|deferred-[^/]*|phase\d+-[^/]*)\.md$` → **historical** (spec-directory record (log, review, results)); never rewritten
- `(^|/)(docs/(superpowers/)?(plans|specs)|\.ai/plans)/\d{4}-\d{2}-\d{2}[^/]*\.md$` → **historical** (dated plan/spec record); never rewritten
- `^\.claudine/memory/` → **historical** (agent memory log); never rewritten
- `(^|/)(features|fixes)/[^/]+/baseline/` → **historical** (another feature's baseline evidence); never rewritten
- `(^|/)(features|fixes)/_unscheduled/` → **historical** (unscheduled spec); never rewritten
- `^tools/test-audit/fixtures/claudine-compat/` → **historical** (frozen replay fixture (its README forbids live edits)); never rewritten
- other dated `features/`/`fixes/` directories → **in-flight-spec**; reviewed at landing
- everything else (recipes, skills, docs, prompts, config, scripts, source comments) → **active**; the migration worklist

## Old targets in scope (from the migration manifests)

| Package | Manifest | Targets | Names |
|---|---|---:|---|
| `tree-hugger` | `tree-hugger/lib` | 10 | `adapter_tests`, `cache_tests`, `corpus_tests`, `lint_diagnostics`, `phase1_diagnostics`, `phase6_neovim_query_reuse`, `query_compile`, `resolver_tests`, `tree_file`, `tree_package` |
| `claudine` | `claudine/lib` | 15 | `agent_errors_fleet`, `boundary_lint`, `canonical_dispatch`, `deprecated_compatibility`, `diagnostic_detail_conformance`, `kimi_wire`, `lifecycle_control_flow_spike`, `model_catalog_integration`, `opencode_stderr_lifecycle`, `protocol_fixture_replay`, `semantic_fidelity`, `strict_mode_provenance_spike`, `tts_phase1_contract`, `tts_phase5_contract`, `typed_stream_protocols` |
| `sniff` | `sniff/lib` | 19 | `bench_fixtures`, `bench_ids_sync`, `bench_plans`, `benchmark_workloads`, `focused_provider`, `git_parity`, `host_capability_cache`, `integration`, `merge_conflict_prediction`, `network_primitives`, `program_installable`, `program_serialization`, `recent_commits`, `remote_observation`, `remote_providers`, `remote_resolution`, `uv_with_install_plan`, `windows_app_paths_orphan`, `windows_find_program_priority` |
| `biscuit-file` | `biscuit-file/lib` | 15 | `completion_round_trip`, `detailed_resolution`, `fetch_integration`, `finalized_reference_resolution`, `implicit_relative`, `parse_count`, `precedence_flip`, `reference_grammar`, `repository_scope_catalog`, `resolution_context`, `round_trip`, `span_compat`, `yaml_corpus`, `yaml_mutation`, `yaml_safety` |
| `schematic-gen` | `schematic/gen` | 14 | `artifact_drift`, `e2e_generation`, `http_client`, `openapi_import_test`, `openapi_strict_completeness`, `path_substitution`, `postman_artifact_validation`, `postman_golden`, `postman_schema`, `postman_var_consistency`, `query_param_detection`, `query_params_codegen`, `terminal_capture`, `ws_codegen` |
| `biscuit-terminal-cli` | `biscuit-terminal/cli` | 14 | `about`, `dir_targets`, `integration_test`, `level2_apple_terminal_prose`, `level2_container_fenced_code`, `level2_cursor_and_hygiene`, `level2_diagrams`, `level2_image`, `level2_layout`, `level2_prose_cells`, `level2_prose_styling`, `level2_render_tree_style`, `level2_status_block`, `level2_style_everywhere_matrix` |
| `claudine-gen` | `claudine/gen` | 11 | `agent_errors_check`, `drift`, `fixtures_provenance`, `generate_ux`, `level2_report_terminal`, `pipeline`, `registry_coverage`, `signals_sidecar_mirror`, `signals_validation`, `steering_check`, `vocabulary` |
| `dmls` | `darkmatter/dmls` | 11 | `level1_graph_index`, `level1_wiki`, `level2_editor_neovim`, `lsp_session`, `mapping_only_corpus`, `no_side_effects`, `packaging_contract`, `stdio_subprocess`, `strict_mode_recovery_spike`, `suggest_constraint_phase1`, `zed_extension_contract` |
| `sniff-cli` | `sniff/cli` | 11 | `cli`, `cli_process_fixture`, `install_interview_cli`, `install_plan`, `level2_cicd_styling`, `level2_git_status_styling`, `level2_perf_tree_rendering`, `level2_recent_commits_rendering`, `snapshots`, `spawn_site_guard`, `tty` |
| `biscuit-tui-cli` | `biscuit-tui/cli` | 15 | `boolean_switch_output`, `choose_cli`, `choose_many_output`, `choose_one_output`, `completions`, `completions_shell`, `exit_codes`, `help_contract`, `input_table_output`, `keyboard_protocol`, `level3_chord_select`, `real_terminal_render`, `text_area_input_output`, `text_input_output`, `windows_captured_stdout` |

| Class | Confirmed hits | Ambiguous hits | Files |
|---|---:|---:|---:|
| active | 4 | 0 | 4 |
| in-flight-spec | 2 | 1 | 2 |
| self | 1156 | 74 | 37 |
| historical | 912 | 500 | 185 |

### Active confirmed hits per package

| Package | Hits |
|---|---:|
| `tree-hugger` | 0 |
| `claudine` | 1 |
| `sniff` | 2 |
| `biscuit-file` | 0 |
| `schematic-gen` | 0 |
| `biscuit-terminal-cli` | 0 |
| `claudine-gen` | 1 |
| `dmls` | 0 |
| `sniff-cli` | 0 |
| `biscuit-tui-cli` | 0 |

## active — confirmed (4)

| File:line | Target | Package | Form | Why | Disposition | Line |
|---|---|---|---|---|---|---|
| `.claude/skills/os/wsl.md:91` | `steering_check` | claudine-gen | pkg::target | agent skill | historical: a dated account ("Proven non-vacuous on 2026-09-09") of the binary as it was then | ``claudine-gen::steering_check` through it fails with CI's exact panic, and the` |
| `.github/workflows/sniff-performance.yml:102` | `bench_ids_sync` | sniff | prose | CI workflow/config | accurate: names the test module (`l1::bench_ids_sync`), not a selector | `# `bench_ids_sync` integration test.` |
| `claudine/docs/topics/performance-testing.md:134` | `bench_ids_sync` | sniff | prose | documentation | accurate: names the test module (`sniff`'s `l1::bench_ids_sync`), not a selector; the step's claim that it covers claudine's benches is pre-existing drift, out of scope | `3. Run the `bench_ids_sync` integration test to validate the IDs file stays in sync.` |
| `claudine/lib/src/stream/protocol/kimi/tests.rs:8` | `protocol_fixture_replay` | claudine | prose | source comment | accurate: rewritten in Phase 6 to name the `l1` binary's module | `// integration binary's `protocol_fixture_replay` module; the cases below are` |

## active — ambiguous (confirm before rewriting) (0)

None.

## in-flight-spec (3)

| File:line | Target | Package | Form | Why | Note | Disposition | Line |
|---|---|---|---|---|---|---|---|
| `claudine/fixes/2026-07-13-rendezvous-local-ipc/plan.md:565` | `drift` | claudine-gen | pkg::target | active dated spec/plan |  | in-flight spec record (2026-07-13); its author owns the command at landing time | ``claudine-gen::drift committed_generated_artifacts_match_phase_1_byte_baseline`` |
| `claudine/fixes/2026-07-13-rendezvous-local-ipc/plan.md:826` | `drift` | claudine-gen | pkg::target | active dated spec/plan |  | in-flight spec record (2026-07-13); its author owns the command at landing time | ``claudine-gen::drift committed_generated_artifacts_match_phase_1_byte_baseline`` |
| `fixes/2026-09-22-test-input-blind-spot/spec.md:335` | `boundary_lint` | claudine | pkg::target | active dated spec/plan | also a Rust path spelling; confirm it is a binary id | in-flight spec record (2026-09-22): a measurement taken against the old binary | ``claudine::boundary_lint` walking the claudine tree). Measured: including` |

## self (1230, by file)

| File | Hits |
|---|---:|
| `features/2026-09-22-consolidated-test-binaries-wave-2/baseline/consumer-sweep.py` | 1 |
| `features/2026-09-22-consolidated-test-binaries-wave-2/baseline/test-input-probe-before.json` | 579 |
| `features/2026-09-22-consolidated-test-binaries-wave-2/baseline/test-inputs-listings-before.json` | 159 |
| `features/2026-09-22-consolidated-test-binaries-wave-2/baseline/test-inputs.md` | 11 |
| `features/2026-09-22-consolidated-test-binaries-wave-2/baseline/test-selector-consumers.md` | 44 |
| `features/2026-09-22-consolidated-test-binaries-wave-2/biscuit-file-migration.json` | 15 |
| `features/2026-09-22-consolidated-test-binaries-wave-2/biscuit-file/test-inputs.json` | 17 |
| `features/2026-09-22-consolidated-test-binaries-wave-2/biscuit-file/test-inputs.md` | 3 |
| `features/2026-09-22-consolidated-test-binaries-wave-2/biscuit-terminal-cli-migration.json` | 106 |
| `features/2026-09-22-consolidated-test-binaries-wave-2/biscuit-terminal-cli/test-inputs.json` | 3 |
| `features/2026-09-22-consolidated-test-binaries-wave-2/biscuit-terminal-cli/test-inputs.md` | 1 |
| `features/2026-09-22-consolidated-test-binaries-wave-2/biscuit-tui-cli-migration.json` | 15 |
| `features/2026-09-22-consolidated-test-binaries-wave-2/biscuit-tui-cli/test-inputs.md` | 1 |
| `features/2026-09-22-consolidated-test-binaries-wave-2/claudine-gen-migration.json` | 11 |
| `features/2026-09-22-consolidated-test-binaries-wave-2/claudine-gen/test-inputs.json` | 12 |
| `features/2026-09-22-consolidated-test-binaries-wave-2/claudine-gen/test-inputs.md` | 1 |
| `features/2026-09-22-consolidated-test-binaries-wave-2/claudine-migration.json` | 15 |
| `features/2026-09-22-consolidated-test-binaries-wave-2/claudine/test-inputs.json` | 12 |
| `features/2026-09-22-consolidated-test-binaries-wave-2/claudine/test-inputs.md` | 2 |
| `features/2026-09-22-consolidated-test-binaries-wave-2/dmls-migration.json` | 11 |
| `features/2026-09-22-consolidated-test-binaries-wave-2/dmls/test-inputs.json` | 5 |
| `features/2026-09-22-consolidated-test-binaries-wave-2/dmls/test-inputs.md` | 2 |
| `features/2026-09-22-consolidated-test-binaries-wave-2/implementation-log.md` | 5 |
| `features/2026-09-22-consolidated-test-binaries-wave-2/schematic-gen-migration.json` | 14 |
| `features/2026-09-22-consolidated-test-binaries-wave-2/schematic-gen/test-inputs.json` | 19 |
| `features/2026-09-22-consolidated-test-binaries-wave-2/schematic-gen/test-inputs.md` | 3 |
| `features/2026-09-22-consolidated-test-binaries-wave-2/sniff-cli-migration.json` | 11 |
| `features/2026-09-22-consolidated-test-binaries-wave-2/sniff-cli/guard-scans-after.md` | 1 |
| `features/2026-09-22-consolidated-test-binaries-wave-2/sniff-cli/test-inputs.json` | 10 |
| `features/2026-09-22-consolidated-test-binaries-wave-2/sniff-cli/test-inputs.md` | 1 |
| `features/2026-09-22-consolidated-test-binaries-wave-2/sniff-migration.json` | 20 |
| `features/2026-09-22-consolidated-test-binaries-wave-2/sniff/test-inputs.json` | 5 |
| `features/2026-09-22-consolidated-test-binaries-wave-2/sniff/test-inputs.md` | 1 |
| `features/2026-09-22-consolidated-test-binaries-wave-2/spikes/s2-hazards.md` | 10 |
| `features/2026-09-22-consolidated-test-binaries-wave-2/tree-hugger-migration.json` | 10 |
| `features/2026-09-22-consolidated-test-binaries-wave-2/tree-hugger/test-inputs.json` | 92 |
| `features/2026-09-22-consolidated-test-binaries-wave-2/tree-hugger/test-inputs.md` | 2 |

## active — file-path references (informational, 5)

Not selectors: `tests/<name>.rs` paths in active files. They go stale when
the file moves under a consolidated binary's directory, so they belong on the
same worklist, but they are not counted in the selector totals above.

| File:line | Target | Package | Why | Disposition | Line |
|---|---|---|---|---|---|
| `.claude/skills/rust-testing/SKILL.md:140` | `windows_captured_stdout` | biscuit-tui-cli | agent skill | historical: a past-tense account of the file's old `#[ignore]` era (Phase 5) | ``biscuit-tui/cli/tests/windows_captured_stdout.rs` had all three at once: an` |
| `sniff/docs/cli/repo_recent-commits.md:30` | `bench_ids_sync` | sniff | documentation | historical: sample command output quoting an old commit's file list | `- modified: sniff/lib/tests/bench_ids_sync.rs` |
| `sniff/docs/cli/repo_recent-commits.md:31` | `uv_with_install_plan` | sniff | documentation | historical: sample command output quoting an old commit's file list | `- added: sniff/lib/tests/uv_with_install_plan.rs` |
| `sniff/docs/cli/repo_source-code-changes.md:47` | `bench_ids_sync` | sniff | documentation | historical: sample command output quoting an old commit's file list | `- modified: sniff/lib/tests/bench_ids_sync.rs` |
| `sniff/docs/cli/repo_source-code-changes.md:48` | `uv_with_install_plan` | sniff | documentation | historical: sample command output quoting an old commit's file list | `- added: sniff/lib/tests/uv_with_install_plan.rs` |

## historical (by file; never rewritten)

| File | Hits | Record date |
|---|---:|---|
| `biscuit-file/features/_completed/2026-04-11-implicit-relative-path/plan.md` | 3 | 2026-04-11 |
| `biscuit-icon/features/_completed/2026-06-07-kickoff/plan.md` | 1 | 2026-06-07 |
| `biscuit-terminal/features/2026-06-29-app-metadata/review-1.md` | 1 | 2026-06-29 |
| `biscuit-terminal/features/2026-06-29-app-metadata/review-2.md` | 1 | 2026-06-29 |
| `biscuit-terminal/features/2026-06-29-app-metadata/review-3.md` | 1 | 2026-06-29 |
| `biscuit-terminal/features/2026-06-29-app-metadata/review-4.md` | 1 | 2026-06-29 |
| `biscuit-terminal/features/2026-06-29-app-metadata/review-5.md` | 1 | 2026-06-29 |
| `biscuit-terminal/features/2026-06-29-app-metadata/review-6.md` | 1 | 2026-06-29 |
| `biscuit-terminal/features/2026-06-29-app-metadata/review-7.md` | 1 | 2026-06-29 |
| `biscuit-terminal/features/2026-06-29-app-metadata/review-8.md` | 1 | 2026-06-29 |
| `biscuit-terminal/features/2026-06-29-app-metadata/review-9.md` | 1 | 2026-06-29 |
| `biscuit-terminal/features/_completed/2026-05-02-apple-terminal/plan.md` | 2 | 2026-05-02 |
| `biscuit-terminal/features/_completed/2026-05-02-apple-terminal/review-1.md` | 5 | 2026-05-02 |
| `biscuit-terminal/features/_completed/2026-05-02-apple-terminal/review-2.md` | 2 | 2026-05-02 |
| `biscuit-terminal/features/_completed/2026-05-02-apple-terminal/review-5.md` | 1 | 2026-05-02 |
| `biscuit-terminal/features/_completed/2026-05-02-apple-terminal/review-plan-1.md` | 2 | 2026-05-02 |
| `biscuit-terminal/features/_completed/2026-05-02-apple-terminal/review-plan-2.md` | 5 | 2026-05-02 |
| `biscuit-terminal/features/_completed/2026-05-02-apple-terminal/review-plan-3.md` | 2 | 2026-05-02 |
| `biscuit-terminal/features/_completed/2026-05-02-apple-terminal/review-plan-4.md` | 3 | 2026-05-02 |
| `biscuit-terminal/features/_completed/2026-05-02-l1-l2-tests/plan.md` | 2 | 2026-05-02 |
| `biscuit-terminal/features/_completed/2026-05-02-l1-l2-tests/review-1.md` | 5 | 2026-05-02 |
| `biscuit-terminal/features/_completed/2026-05-02-l1-l2-tests/review-2.md` | 5 | 2026-05-02 |
| `biscuit-terminal/features/_completed/2026-05-02-l1-l2-tests/review-plan-1.md` | 11 | 2026-05-02 |
| `biscuit-terminal/features/_completed/2026-05-02-l1-l2-tests/review-plan-2.md` | 12 | 2026-05-02 |
| `biscuit-terminal/features/_completed/2026-05-05-prose-plus/plan.md` | 3 | 2026-05-05 |
| `biscuit-tui/features/_completed/2026-04-23-choose-cli/plan.md` | 5 | 2026-04-23 |
| `biscuit-tui/features/_completed/2026-04-23-choose-cli/review-1.md` | 1 | 2026-04-23 |
| `biscuit-tui/features/_completed/2026-04-23-choose-cli/review-plan-1.md` | 2 | 2026-04-23 |
| `biscuit-tui/features/_completed/2026-04-23-choose-cli/review-plan-2.md` | 3 | 2026-04-23 |
| `biscuit-tui/features/_completed/2026-04-28-choose-one-improvements/review-10.md` | 3 | 2026-04-28 |
| `biscuit-tui/features/_completed/2026-04-28-choose-one-improvements/review-7.md` | 4 | 2026-04-28 |
| `biscuit-tui/features/_completed/2026-04-28-choose-one-improvements/review-8.md` | 2 | 2026-04-28 |
| `biscuit-tui/features/_completed/2026-04-28-choose-one-improvements/review-plan-10.md` | 6 | 2026-04-28 |
| `biscuit-tui/features/_completed/2026-04-28-choose-one-improvements/review-plan-4.md` | 2 | 2026-04-28 |
| `biscuit-tui/features/_completed/2026-04-28-choose-one-improvements/review-plan-7.md` | 4 | 2026-04-28 |
| `biscuit-tui/features/_completed/2026-04-28-choose-one-improvements/review-plan-8.md` | 2 | 2026-04-28 |
| `biscuit-tui/features/_completed/2026-04-28-choose-one-improvements/review-plan-9.md` | 2 | 2026-04-28 |
| `biscuit-tui/features/_completed/2026-06-19-review-findings/review-3.md` | 3 | 2026-06-19 |
| `biscuit-tui/features/_completed/2026-06-19-review-findings/review-4.md` | 3 | 2026-06-19 |
| `biscuit-tui/features/_completed/2026-06-19-review-findings/review-5.md` | 2 | 2026-06-19 |
| `biscuit-tui/features/_completed/2026-06-19-review-findings/windows-captured-stdout-repro.md` | 2 | 2026-06-19 |
| `claudine/features/_completed/2026-04-18-opencode-reporting-improvements/plan.md` | 1 | 2026-04-18 |
| `claudine/features/_completed/2026-04-26-centralized-providers/review-plan-3.md` | 3 | 2026-04-26 |
| `claudine/features/_completed/2026-04-26-fix-kimi/plan.md` | 4 | 2026-04-26 |
| `claudine/features/_completed/2026-04-26-fix-kimi/review-plan-1.md` | 2 | 2026-04-26 |
| `claudine/features/_completed/2026-05-08-testing-setup-teardown/plan.md` | 1 | 2026-05-08 |
| `claudine/features/_completed/2026-06-28-real-errors/review-11.md` | 1 | 2026-06-28 |
| `claudine/features/_completed/2026-06-28-real-errors/review-5.md` | 1 | 2026-06-28 |
| `claudine/features/_completed/2026-06-28-real-errors/review-6.md` | 1 | 2026-06-28 |
| `claudine/features/_completed/2026-07-13-error-propogation/plan.md` | 1 | 2026-07-13 |
| `claudine/features/_completed/2026-07-13-proxy-with/notes/baseline.md` | 1 | 2026-07-13 |
| `claudine/features/_completed/2026-07-13-proxy-with/plan.md` | 7 | 2026-07-13 |
| `claudine/features/_completed/2026-07-13-proxy-with/review-3.md` | 1 | 2026-07-13 |
| `claudine/fixes/2026-09-17-remove-strict-mode/spike-results.md` | 4 | 2026-09-17 |
| `claudine/fixes/_completed/2026-04-08-different-configs/review.md` | 1 | 2026-04-08 |
| `claudine/fixes/_completed/2026-05-12-opencode-stderr-returns/review-1.md` | 1 | 2026-05-12 |
| `claudine/fixes/_completed/2026-06-10-error-suggest-format/review-3.md` | 1 | 2026-06-10 |
| `claudine/fixes/_completed/2026-06-10-error-suggest-format/review-4.md` | 1 | 2026-06-10 |
| `claudine/fixes/_completed/2026-07-20-claudine-mega-merge/phase4-test-map.md` | 2 | 2026-07-20 |
| `claudine/fixes/_completed/2026-07-31-claudine-win/implementation-notes/phase-0-native-windows-baseline.md` | 3 | 2026-07-31 |
| `claudine/fixes/_completed/2026-07-31-claudine-win/spec.md` | 5 | 2026-07-31 |
| `claudine/fixes/_completed/2026-08-01-cli-slow-tests/log.md` | 3 | 2026-08-01 |
| `claudine/fixes/_completed/2026-09-03-tts-not-finishing/log.md` | 1 | 2026-09-03 |
| `claudine/fixes/_completed/2026-09-03-tts-not-finishing/phase-1-baseline.md` | 1 | 2026-09-03 |
| `claudine/fixes/_completed/2026-09-03-tts-not-finishing/plan.md` | 1 | 2026-09-03 |
| `claudine/fixes/_completed/2026-09-05-inline-flow-and-validations/plan.md` | 2 | 2026-09-05 |
| `claudine/fixes/_completed/2026-09-05-inline-flow-and-validations/review-1.md` | 1 | 2026-09-05 |
| `claudine/fixes/_completed/2026-09-05-inline-flow-and-validations/review-2.md` | 1 | 2026-09-05 |
| `claudine/fixes/_completed/2026-09-07-faster-claudine-tests/deferred-performance-measurement.md` | 2 | 2026-09-07 |
| `claudine/fixes/_completed/2026-09-07-faster-claudine-tests/families.json` | 24 | 2026-09-07 |
| `claudine/fixes/_completed/2026-09-07-faster-claudine-tests/inventory.md` | 6 | 2026-09-07 |
| `claudine/fixes/_completed/2026-09-07-faster-claudine-tests/log.md` | 7 | 2026-09-07 |
| `claudine/fixes/_completed/2026-09-07-faster-claudine-tests/plan.md` | 1 | 2026-09-07 |
| `claudine/fixes/_completed/2026-09-07-faster-claudine-tests/results.md` | 4 | 2026-09-07 |
| `claudine/fixes/_completed/2026-09-07-faster-claudine-tests/review-1.md` | 1 | 2026-09-07 |
| `claudine/fixes/_completed/2026-09-07-faster-claudine-tests/review-2.md` | 2 | 2026-09-07 |
| `claudine/fixes/_completed/2026-09-12-shadow-home/implementation-log.md` | 6 | 2026-09-12 |
| `claudine/fixes/_completed/2026-09-12-shadow-home/review-2.md` | 1 | 2026-09-12 |
| `claudine/fixes/_completed/2026-09-13-better-static-analysis/implementation-log.md` | 2 | 2026-09-13 |
| `claudine/fixes/_unscheduled/test-suite-residuals/spec.md` | 2 | 2026-09-10 |
| `darkmatter/features/2026-09-22-lifecycle-events/spike-results.md` | 2 | 2026-09-22 |
| `darkmatter/features/_completed/2026-06-01-url-referencing/review-9.md` | 1 | 2026-06-01 |
| `darkmatter/features/_completed/2026-07-13-meta-schema/phase5-baseline-replay.md` | 1 | 2026-07-13 |
| `darkmatter/features/_completed/2026-07-13-more-is-more/log.md` | 2 | 2026-07-13 |
| `darkmatter/features/_completed/2026-07-13-more-is-more/review-18.md` | 3 | 2026-07-13 |
| `darkmatter/features/_completed/2026-07-13-more-is-more/review-19.md` | 1 | 2026-07-13 |
| `darkmatter/features/_completed/2026-07-13-more-is-more/review-27.md` | 1 | 2026-07-13 |
| `darkmatter/features/_completed/2026-07-14-invalid-frontmatter/log.md` | 5 | 2026-07-14 |
| `darkmatter/features/_completed/2026-07-14-invalid-frontmatter/review-2.md` | 4 | 2026-07-14 |
| `darkmatter/features/_completed/2026-07-14-invalid-frontmatter/review-3.md` | 4 | 2026-07-14 |
| `darkmatter/features/_completed/2026-09-09-more-context/implementation-log.md` | 8 | 2026-09-09 |
| `darkmatter/fixes/_completed/2026-07-20-dm-mega-merge/resolution-record.md` | 17 | 2026-07-20 |
| `darkmatter/fixes/_completed/2026-09-03-dmls-regression/log.md` | 1 | 2026-09-03 |
| `darkmatter/fixes/_completed/2026-09-07-faster-darkmatter-tests/attribution/ci-macos-latest-L1.json` | 5 | 2026-09-07 |
| `darkmatter/fixes/_completed/2026-09-07-faster-darkmatter-tests/attribution/ci-macos-latest-L2.json` | 1 | 2026-09-07 |
| `darkmatter/fixes/_completed/2026-09-07-faster-darkmatter-tests/attribution/ci-ubuntu-latest-L1.json` | 5 | 2026-09-07 |
| `darkmatter/fixes/_completed/2026-09-07-faster-darkmatter-tests/attribution/ci-ubuntu-latest-L2.json` | 1 | 2026-09-07 |
| `darkmatter/fixes/_completed/2026-09-07-faster-darkmatter-tests/attribution/ci-windows-latest-L1.json` | 5 | 2026-09-07 |
| `darkmatter/fixes/_completed/2026-09-07-faster-darkmatter-tests/attribution/ci-wsl2-ubuntu-L1.json` | 5 | 2026-09-07 |
| `darkmatter/fixes/_completed/2026-09-07-faster-darkmatter-tests/attribution/local-l1-by-binary.json` | 13 | 2026-09-07 |
| `darkmatter/fixes/_completed/2026-09-07-faster-darkmatter-tests/attribution/local-l2-by-binary.json` | 2 | 2026-09-07 |
| `darkmatter/fixes/_completed/2026-09-07-faster-darkmatter-tests/families.json` | 9 | 2026-09-07 |
| `darkmatter/fixes/_completed/2026-09-07-faster-darkmatter-tests/inventory.md` | 2 | 2026-09-07 |
| `darkmatter/fixes/_completed/2026-09-07-faster-darkmatter-tests/log.md` | 9 | 2026-09-07 |
| `darkmatter/fixes/_completed/2026-09-07-faster-darkmatter-tests/review-1.md` | 3 | 2026-09-07 |
| `darkmatter/fixes/_completed/2026-09-07-faster-darkmatter-tests/review-2.md` | 3 | 2026-09-07 |
| `darkmatter/fixes/_completed/2026-09-16-content-policy-no-cache/implementation-log.md` | 3 | 2026-09-16 |
| `darkmatter/fixes/_completed/2026-09-16-content-policy-no-cache/review-3.md` | 1 | 2026-09-16 |
| `features/2026-09-21-consolidated-test-binaries/implementation-log.md` | 2 | 2026-09-21 |
| `fixes/_complete/2026-09-13-cicd-redundancies/acceptance.md` | 2 | 2026-09-13 |
| `fixes/_complete/2026-09-13-cicd-redundancies/plan.md` | 1 | 2026-09-13 |
| `fixes/_complete/2026-09-13-cicd-redundancies/rulings.md` | 4 | 2026-09-13 |
| `fixes/_completed/2026-09-18-tmux-flake/spec.md` | 1 | 2026-09-18 |
| `playa/docs/plans/2026-04-29-playa-stuck-device-resilience.md` | 2 | 2026-04-29 |
| `renderable/features/_completed/2026-04-17-layout-and-style/review-3.md` | 1 | 2026-04-17 |
| `renderable/features/_completed/2026-04-17-layout-and-style/review-4.md` | 1 | 2026-04-17 |
| `renderable/features/_completed/2026-04-17-layout-and-style/review-5.md` | 2 | 2026-04-17 |
| `renderable/features/_completed/2026-04-17-layout-and-style/review-6.md` | 1 | 2026-04-17 |
| `renderable/features/_completed/2026-05-17-prose-cross-target/review-3.md` | 3 | 2026-05-17 |
| `renderable/features/_completed/2026-05-17-prose-cross-target/review-4.md` | 2 | 2026-05-17 |
| `renderable/features/_completed/2026-05-17-prose-cross-target/review-5.md` | 2 | 2026-05-17 |
| `renderable/features/_completed/2026-05-17-prose-cross-target/review-6.md` | 2 | 2026-05-17 |
| `renderable/features/_completed/2026-05-17-prose-cross-target/review-7.md` | 2 | 2026-05-17 |
| `renderable/features/_completed/2026-05-17-prose-cross-target/review-8.md` | 1 | 2026-05-17 |
| `renderable/features/_completed/2026-05-19-pushing-toward-ir/stage1-and-2/StatusBlock-review-1.md` | 1 | 2026-05-19 |
| `renderable/features/_completed/2026-05-19-pushing-toward-ir/stage3-plan.md` | 2 | 2026-05-19 |
| `renderable/features/_completed/2026-06-04-renderer-folds/review-2.md` | 1 | 2026-06-04 |
| `renderable/features/_completed/2026-06-04-renderer-folds/review-3.md` | 1 | 2026-06-04 |
| `renderable/features/_completed/2026-06-04-renderer-folds/review-4.md` | 1 | 2026-06-04 |
| `renderable/features/_completed/2026-06-08-prose-cells/review-3.md` | 1 | 2026-06-08 |
| `renderable/features/_completed/2026-06-08-prose-cells/review-4.md` | 1 | 2026-06-08 |
| `renderable/features/_completed/2026-06-08-prose-cells/review-6.md` | 1 | 2026-06-08 |
| `renderable/features/_completed/2026-06-08-prose-cells/review-7.md` | 2 | 2026-06-08 |
| `renderable/features/_completed/2026-06-08-prose-cells/review-8.md` | 2 | 2026-06-08 |
| `renderable/features/_completed/2026-06-08-prose-cells/review-9.md` | 2 | 2026-06-08 |
| `schematic/features/_completed/2026-05-07-artificial-analysis/review-1.md` | 1 | 2026-05-07 |
| `schematic/features/_completed/2026-05-07-artificial-analysis/review-2.md` | 1 | 2026-05-07 |
| `schematic/features/_completed/2026-05-07-artificial-analysis/review-3.md` | 1 | 2026-05-07 |
| `schematic/features/_completed/2026-05-07-artificial-analysis/review-plan-2.md` | 1 | 2026-05-07 |
| `schematic/features/_completed/ergonomics-and-postman-projects/review-3.md` | 4 | 2026-05-05 |
| `schematic/features/_completed/ergonomics-and-postman-projects/review-plan-1.md` | 3 | 2026-05-05 |
| `schematic/features/_completed/ergonomics-and-postman-projects/review-plan-2.md` | 3 | 2026-05-05 |
| `schematic/features/_completed/ergonomics-and-postman-projects/review-plan-3.md` | 10 | 2026-05-05 |
| `sniff/features/_completed/2026-04-22-package-areas/review-plan-1.md` | 3 | 2026-04-22 |
| `sniff/features/_completed/2026-04-22-package-areas/review-plan-2.md` | 2 | 2026-04-22 |
| `sniff/features/_completed/2026-04-25-repo-pr/plan.md` | 1 | 2026-04-25 |
| `sniff/features/_completed/2026-04-25-repo-pr/review-plan-1.md` | 1 | 2026-04-25 |
| `sniff/features/_completed/2026-04-28-incorrect-json/review-plan-1.md` | 12 | 2026-04-28 |
| `sniff/features/_completed/2026-04-28-incorrect-json/review-plan-2.md` | 11 | 2026-04-28 |
| `sniff/features/_completed/2026-04-28-incorrect-json/review-plan-4.md` | 1 | 2026-04-28 |
| `sniff/features/_completed/2026-05-27-scope-complete-json/plan.md` | 2 | 2026-05-27 |
| `sniff/features/_completed/2026-06-02-repo-remote-improvements/review-2.md` | 1 | 2026-06-02 |
| `sniff/features/_completed/2026-06-09-git-status-touch-up/review-4.md` | 1 | 2026-06-09 |
| `sniff/features/_completed/2026-06-12-git-identity-request/review-2.md` | 1 | 2026-06-12 |
| `sniff/features/_completed/2026-06-14-more-repo/baseline-repo.json` | 2 | 2026-06-14 |
| `sniff/features/_completed/2026-06-14-more-repo/review-4.md` | 2 | 2026-06-14 |
| `sniff/features/_completed/2026-06-14-more-repo/review-5.md` | 2 | 2026-06-14 |
| `sniff/features/_completed/2026-06-14-more-repo/review-6.md` | 5 | 2026-06-14 |
| `sniff/features/_completed/2026-06-15-improved-monorepo-capture/review-2.md` | 1 | 2026-06-15 |
| `sniff/features/_completed/2026-06-15-improved-monorepo-capture/review-3.md` | 1 | 2026-06-15 |
| `sniff/features/_completed/2026-06-15-improved-monorepo-capture/review-4.md` | 1 | 2026-06-15 |
| `sniff/features/_completed/2026-06-15-improved-monorepo-capture/review-5.md` | 2 | 2026-06-15 |
| `sniff/features/_completed/2026-06-20-faster-package-list/review-1.md` | 4 | 2026-06-20 |
| `sniff/features/_completed/2026-06-20-faster-package-list/review-2.md` | 4 | 2026-06-20 |
| `sniff/features/_completed/2026-06-20-faster-package-list/review-3.md` | 4 | 2026-06-20 |
| `sniff/features/_completed/2026-07-16-performance/phases/_completed/01-work-accounting/spec.md` | 1 | 2026-07-16 |
| `sniff/features/_completed/2026-07-16-performance/phases/_completed/02-reuse-and-scope/spec.md` | 2 | 2026-07-16 |
| `sniff/features/_completed/2026-07-16-performance/phases/_completed/03-observation-index/spec.md` | 1 | 2026-07-16 |
| `sniff/features/_completed/2026-09-15-recent-commits/implementation-log.md` | 11 | 2026-09-15 |
| `sniff/features/_completed/2026-09-15-recent-commits/review-1.md` | 2 | 2026-09-15 |
| `sniff/features/_completed/2026-09-15-recent-commits/review-2.md` | 1 | 2026-09-15 |
| `sniff/features/_completed/2026-09-15-recent-commits/review-3.md` | 1 | 2026-09-15 |
| `sniff/fixes/_completed/2026-06-17-repo-version/review-2.md` | 1 | 2026-06-17 |
| `sniff/fixes/_completed/2026-06-17-repo-version/review-4.md` | 2 | 2026-06-17 |
| `sniff/fixes/_completed/2026-09-07-faster-sniff-tests/families.json` | 58 | 2026-09-07 |
| `sniff/fixes/_completed/2026-09-07-faster-sniff-tests/family-members.json` | 793 | 2026-09-07 |
| `sniff/fixes/_completed/2026-09-07-faster-sniff-tests/inventory.md` | 1 | 2026-09-07 |
| `sniff/fixes/_completed/2026-09-07-faster-sniff-tests/log.md` | 23 | 2026-09-07 |
| `sniff/fixes/_completed/2026-09-07-faster-sniff-tests/results.md` | 1 | 2026-09-07 |
| `sniff/fixes/_completed/2026-09-07-faster-sniff-tests/review-1.md` | 2 | 2026-09-07 |
| `sniff/fixes/_completed/2026-09-07-faster-sniff-tests/review-2.md` | 2 | 2026-09-07 |
| `sniff/fixes/_completed/2026-09-14-corrected-perf-flag/implementation-log.md` | 4 | 2026-09-14 |
| `sniff/reviews/_completed/2026-05-02-performance-review/follow-up-plan.md` | 1 | 2026-05-02 |
| `sniff/reviews/_completed/2026-05-02-performance-review/plan.md` | 3 | 2026-05-02 |
| `tools/test-audit/fixtures/claudine-compat/families.json` | 24 | 2026-09-09 |
