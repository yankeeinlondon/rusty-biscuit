---
kind: baseline
feature: 2026-09-22-consolidated-test-binaries-wave-2
created: 2026-09-23
rev: 208051f753aa9ce0fcc19622f64464d11d3570d2
generator: baseline/consumer-sweep.py
---

# Test-selector consumer sweep (before any wave-2 migration)

Every reference, in git-tracked text files, to a current integration-test
target of the ten wave-2 packages, in one of these forms:

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

## Targets in scope

| Package | Manifest | Targets | Names |
|---|---|---:|---|
| `tree-hugger` | `tree-hugger/lib` | 10 | `adapter_tests`, `cache_tests`, `corpus_tests`, `lint_diagnostics`, `phase1_diagnostics`, `phase6_neovim_query_reuse`, `query_compile`, `resolver_tests`, `tree_file`, `tree_package` |
| `claudine` | `claudine/lib` | 15 | `agent_errors_fleet`, `boundary_lint`, `canonical_dispatch`, `deprecated_compatibility`, `diagnostic_detail_conformance`, `kimi_wire`, `lifecycle_control_flow_spike`, `model_catalog_integration`, `opencode_stderr_lifecycle`, `protocol_fixture_replay`, `semantic_fidelity`, `strict_mode_provenance_spike`, `tts_phase1_contract`, `tts_phase5_contract`, `typed_stream_protocols` |
| `sniff` | `sniff/lib` | 20 | `bench_fixtures`, `bench_ids_sync`, `bench_plans`, `benchmark_workloads`, `fixtures`, `focused_provider`, `git_parity`, `host_capability_cache`, `integration`, `merge_conflict_prediction`, `network_primitives`, `program_installable`, `program_serialization`, `recent_commits`, `remote_observation`, `remote_providers`, `remote_resolution`, `uv_with_install_plan`, `windows_app_paths_orphan`, `windows_find_program_priority` |
| `biscuit-file` | `biscuit-file/lib` | 15 | `completion_round_trip`, `detailed_resolution`, `fetch_integration`, `finalized_reference_resolution`, `implicit_relative`, `parse_count`, `precedence_flip`, `reference_grammar`, `repository_scope_catalog`, `resolution_context`, `round_trip`, `span_compat`, `yaml_corpus`, `yaml_mutation`, `yaml_safety` |
| `schematic-gen` | `schematic/gen` | 14 | `artifact_drift`, `e2e_generation`, `http_client`, `openapi_import_test`, `openapi_strict_completeness`, `path_substitution`, `postman_artifact_validation`, `postman_golden`, `postman_schema`, `postman_var_consistency`, `query_param_detection`, `query_params_codegen`, `terminal_capture`, `ws_codegen` |
| `biscuit-terminal-cli` | `biscuit-terminal/cli` | 14 | `about`, `dir_targets`, `integration_test`, `level2_apple_terminal_prose`, `level2_container_fenced_code`, `level2_cursor_and_hygiene`, `level2_diagrams`, `level2_image`, `level2_layout`, `level2_prose_cells`, `level2_prose_styling`, `level2_render_tree_style`, `level2_status_block`, `level2_style_everywhere_matrix` |
| `claudine-gen` | `claudine/gen` | 11 | `agent_errors_check`, `drift`, `fixtures_provenance`, `generate_ux`, `level2_report_terminal`, `pipeline`, `registry_coverage`, `signals_sidecar_mirror`, `signals_validation`, `steering_check`, `vocabulary` |
| `dmls` | `darkmatter/dmls` | 11 | `level1_graph_index`, `level1_wiki`, `level2_editor_neovim`, `lsp_session`, `mapping_only_corpus`, `no_side_effects`, `packaging_contract`, `stdio_subprocess`, `strict_mode_recovery_spike`, `suggest_constraint_phase1`, `zed_extension_contract` |
| `sniff-cli` | `sniff/cli` | 11 | `cli`, `cli_process_fixture`, `install_interview_cli`, `install_plan`, `level2_cicd_styling`, `level2_git_status_styling`, `level2_perf_tree_rendering`, `level2_recent_commits_rendering`, `snapshots`, `spawn_site_guard`, `tty` |
| `biscuit-tui-cli` | `biscuit-tui/cli` | 15 | `boolean_switch_output`, `choose_cli`, `choose_many_output`, `choose_one_output`, `completions`, `completions_shell`, `exit_codes`, `help_contract`, `input_table_output`, `keyboard_protocol`, `level3_chord_select`, `real_terminal_render`, `text_area_input_output`, `text_input_output`, `windows_captured_stdout` |

| Class | Confirmed hits | Ambiguous hits | Files |
|---|---:|---:|---:|
| active | 27 | 0 | 16 |
| in-flight-spec | 2 | 1 | 2 |
| self | 0 | 0 | 0 |
| historical | 912 | 501 | 185 |

### Active confirmed hits per package

| Package | Hits |
|---|---:|
| `tree-hugger` | 0 |
| `claudine` | 1 |
| `sniff` | 3 |
| `biscuit-file` | 0 |
| `schematic-gen` | 6 |
| `biscuit-terminal-cli` | 4 |
| `claudine-gen` | 1 |
| `dmls` | 3 |
| `sniff-cli` | 0 |
| `biscuit-tui-cli` | 9 |

## active — confirmed (27)

| File:line | Target | Package | Form | Why | Line |
|---|---|---|---|---|---|
| `.claude/skills/biscuit-test-harness/SKILL.md:166` | `level2_prose_styling` | biscuit-terminal-cli | --test | agent skill | `cargo test -p biscuit-terminal-cli --test level2_prose_styling` |
| `.claude/skills/biscuit-test-harness/SKILL.md:174` | `level2_prose_styling` | biscuit-terminal-cli | --test | agent skill | `KITTY_LISTEN_ON=unix:/tmp/kitty-l2 cargo test -p biscuit-terminal-cli --test level2_prose_styling` |
| `.claude/skills/os/wsl.md:91` | `steering_check` | claudine-gen | pkg::target | agent skill | ``claudine-gen::steering_check` through it fails with CI's exact panic, and the` |
| `.claude/skills/schematic-define/SKILL.md:173` | `e2e_generation` | schematic-gen | --test | agent skill | `cargo test -p schematic-gen --test e2e_generation binary_response_generates_request_bytes_method` |
| `.claude/skills/schematic-define/SKILL.md:174` | `e2e_generation` | schematic-gen | --test | agent skill | `cargo test -p schematic-gen --test e2e_generation text_response_generates_request_text_method` |
| `.claude/skills/schematic-define/SKILL.md:175` | `e2e_generation` | schematic-gen | --test | agent skill | `cargo test -p schematic-gen --test e2e_generation empty_response_generates_request_empty_method` |
| `.github/workflows/sniff-performance.yml:102` | `bench_ids_sync` | sniff | prose | CI workflow/config | `# `bench_ids_sync` integration test.` |
| `biscuit-test-harness/README.md:391` | `level2_prose_styling` | biscuit-terminal-cli | --test | documentation | `cargo test -p biscuit-terminal-cli --test level2_prose_styling` |
| `biscuit-test-harness/README.md:400` | `level2_prose_styling` | biscuit-terminal-cli | --test | documentation | `KITTY_LISTEN_ON=unix:/tmp/kitty-l2 cargo test -p biscuit-terminal-cli --test level2_prose_styling` |
| `biscuit-tui/cli/README.md:367` | `keyboard_protocol` | biscuit-tui-cli | --test | documentation | `cargo test -p biscuit-tui-cli --test keyboard_protocol` |
| `biscuit-tui/cli/README.md:371` | `real_terminal_render` | biscuit-tui-cli | --test | documentation | `cargo test -p biscuit-tui-cli --test real_terminal_render` |
| `biscuit-tui/cli/README.md:375` | `real_terminal_render` | biscuit-tui-cli | --test | documentation | `RUN_LEVEL3=1 cargo test -p biscuit-tui-cli --test real_terminal_render` |
| `biscuit-tui/cli/README.md:378` | `real_terminal_render` | biscuit-tui-cli | --test | documentation | `cargo test -p biscuit-tui-cli --test real_terminal_render \` |
| `biscuit-tui/cli/tests/choose_cli.rs:782` | `choose_cli` | biscuit-tui-cli | --test | source comment | `//     QUESTION_INTERACTIVE_PTY=1 cargo test -p biscuit-tui-cli --test choose_cli` |
| `biscuit-tui/cli/tests/completions_shell.rs:12` | `completions_shell` | biscuit-tui-cli | --test | source comment | `//!     RUN_SHELL_TESTS=1 cargo test -p biscuit-tui-cli --test completions_shell` |
| `biscuit-tui/justfile:127` | `keyboard_protocol` | biscuit-tui-cli | --test | recipe | `@RUN_PTY_TESTS=1 cargo test -p {{CLI}} --test keyboard_protocol -- --nocapture` |
| `biscuit-tui/justfile:128` | `completions_shell` | biscuit-tui-cli | --test | recipe | `@RUN_SHELL_TESTS=1 cargo test -p {{CLI}} --test completions_shell -- --nocapture` |
| `biscuit-tui/justfile:129` | `choose_cli` | biscuit-tui-cli | --test | recipe | `@QUESTION_INTERACTIVE_PTY=1 cargo test -p {{CLI}} --test choose_cli pty:: -- --nocapture` |
| `claudine/docs/topics/performance-testing.md:134` | `bench_ids_sync` | sniff | prose | documentation | `3. Run the `bench_ids_sync` integration test to validate the IDs file stays in sync.` |
| `claudine/lib/src/stream/protocol/kimi/tests.rs:8` | `protocol_fixture_replay` | claudine | prose | source comment | `// `protocol_fixture_replay` integration test; the cases below are` |
| `darkmatter/dmls/tests/common/mod.rs:2` | `lsp_session` | dmls | prose | source comment | `//! binaries: `lsp_session`, `no_side_effects`, and `suggest_constraint_phase1`.` |
| `darkmatter/dmls/tests/common/mod.rs:2` | `no_side_effects` | dmls | prose | source comment | `//! binaries: `lsp_session`, `no_side_effects`, and `suggest_constraint_phase1`.` |
| `darkmatter/dmls/tests/common/mod.rs:2` | `suggest_constraint_phase1` | dmls | prose | source comment | `//! binaries: `lsp_session`, `no_side_effects`, and `suggest_constraint_phase1`.` |
| `schematic/gen/tests/artifact_drift.rs:11` | `artifact_drift` | schematic-gen | --test | source comment | `//! cargo test -p schematic-gen --test artifact_drift` |
| `schematic/gen/tests/postman_golden.rs:18` | `postman_golden` | schematic-gen | --test | source comment | `//! BLESS_POSTMAN_GOLDEN=1 cargo test -p schematic-gen --test postman_golden` |
| `schematic/justfile:271` | `e2e_generation` | schematic-gen | --test | recipe | `@cargo test -p schematic-gen --test e2e_generation -- --ignored` |
| `sniff/lib/tests/remote_providers.rs:11` | `remote_providers` | sniff | --test | source comment | `//! cargo test -p sniff --features remote --test remote_providers` |

## active — ambiguous (confirm before rewriting) (0)

None.

## in-flight-spec (3)

| File:line | Target | Package | Form | Why | Note | Line |
|---|---|---|---|---|---|---|
| `claudine/fixes/2026-07-13-rendezvous-local-ipc/plan.md:565` | `drift` | claudine-gen | pkg::target | active dated spec/plan |  | ``claudine-gen::drift committed_generated_artifacts_match_phase_1_byte_baseline`` |
| `claudine/fixes/2026-07-13-rendezvous-local-ipc/plan.md:826` | `drift` | claudine-gen | pkg::target | active dated spec/plan |  | ``claudine-gen::drift committed_generated_artifacts_match_phase_1_byte_baseline`` |
| `fixes/2026-09-22-test-input-blind-spot/spec.md:335` | `boundary_lint` | claudine | pkg::target | active dated spec/plan | also a Rust path spelling; confirm it is a binary id | ``claudine::boundary_lint` walking the claudine tree). Measured: including` |

## self (0)

None.

## active — file-path references (informational, 41)

Not selectors: `tests/<name>.rs` paths in active files. They go stale when
the file moves under a consolidated binary's directory, so they belong on the
same worklist, but they are not counted in the selector totals above.

| File:line | Target | Package | Why | Line |
|---|---|---|---|---|
| `.claude/skills/os/windows.md:219` | `windows_captured_stdout` | biscuit-tui-cli | agent skill | ``biscuit-tui/cli/tests/windows_captured_stdout.rs` is ordinary `windows-latest`` |
| `.claude/skills/rust-testing/SKILL.md:140` | `windows_captured_stdout` | biscuit-tui-cli | agent skill | ``biscuit-tui/cli/tests/windows_captured_stdout.rs` had all three at once: an` |
| `.claude/skills/sniff/SKILL.md:206` | `spawn_site_guard` | sniff-cli | agent skill | `naming the tool observed or the absence proved — `cli/tests/spawn_site_guard.rs`` |
| `.claude/skills/sniff/network.md:64` | `network_primitives` | sniff | agent skill | `- Real round trips live in `lib/tests/network_primitives.rs` as `real_` tests` |
| `.claude/skills/tree-hugger/query-system.md:243` | `tree_file` | tree-hugger | agent skill | `- Add test in `lib/tests/tree_file.rs`` |
| `biscuit-test-harness/src/bin/broker.rs:187` | `level2_apple_terminal_prose` | biscuit-terminal-cli | source comment | `// (`biscuit-terminal/cli/tests/level2_apple_terminal_prose.rs`),` |
| `biscuit-tui/cli/Cargo.toml:45` | `windows_captured_stdout` | biscuit-tui-cli | documentation | `# Test-only, target-scoped: in `tests/windows_captured_stdout.rs` the F2` |
| `biscuit-tui/cli/README.md:348` | `real_terminal_render` | biscuit-tui-cli | documentation | `and a `cliclick` helper. Tests in `cli/tests/real_terminal_render.rs` use them.` |
| `biscuit-tui/lib/README.md:201` | `real_terminal_render` | biscuit-tui-cli | documentation | ``cli/tests/real_terminal_render.rs::level2_wezterm_bare_ctrl_kitty_bytes_reveal_badges`.` |
| `claudine/docs/research/signals/fixtures/README.md:40` | `fixtures_provenance` | claudine-gen | documentation | `CI-enforced machine record; `gen/tests/fixtures_provenance.rs` asserts the` |
| `claudine/docs/research/signals/fixtures/provenance.yaml:2` | `fixtures_provenance` | claudine-gen | documentation | `# path from this directory). CI-enforced by `gen/tests/fixtures_provenance.rs`:` |
| `claudine/docs/topics/provider-metadata.md:186` | `registry_coverage` | claudine-gen | documentation | `(`gen/tests/registry_coverage.rs` and its lib twin` |
| `claudine/gen/Cargo.toml:67` | `level2_report_terminal` | claudine-gen | documentation | `# CI policy: this package owns real L2 tests (`tests/level2_report_terminal.rs`` |
| `claudine/lib/src/provider/tests.rs:183` | `registry_coverage` | claudine-gen | source comment | `/// (`gen/tests/registry_coverage.rs`): the serialized `--describe` key` |
| `claudine/lib/src/provider/tests.rs:254` | `registry_coverage` | claudine-gen | **string literal in code** | `here AND in gen/tests/registry_coverage.rs, and extend the mapping registry"` |
| `darkmatter/dmls/Cargo.toml:72` | `level2_editor_neovim` | dmls | documentation | `# Level-2 editor tests (tests/level2_editor_neovim.rs): tier gating and the` |
| `darkmatter/dmls/README.md:25` | `no_side_effects` | dmls | documentation | `[`tests/no_side_effects.rs`](tests/no_side_effects.rs) (spec acceptance` |
| `darkmatter/dmls/README.md:25` | `no_side_effects` | dmls | documentation | `[`tests/no_side_effects.rs`](tests/no_side_effects.rs) (spec acceptance` |
| `darkmatter/dmls/README.md:147` | `lsp_session` | dmls | documentation | `The in-process JSON-RPC session tests (`tests/lsp_session.rs`) are L1. The L2` |
| `darkmatter/dmls/README.md:148` | `level2_editor_neovim` | dmls | documentation | `tier (`tests/level2_editor_neovim.rs`) drives Neovim's real LSP client against` |
| `darkmatter/dmls/docs/editors/smoke-checklist.md:4` | `lsp_session` | dmls | documentation | `steps that remain OUTSTANDING. The automated `darkmatter/dmls/tests/lsp_session.rs`` |
| `darkmatter/dmls/docs/editors/smoke-checklist.md:9` | `level2_editor_neovim` | dmls | documentation | `**Automated for Neovim:** `tests/level2_editor_neovim.rs` (run via` |
| `darkmatter/dmls/docs/editors/smoke-checklist.md:96` | `no_side_effects` | dmls | documentation | ``tests/no_side_effects.rs`).` |
| `darkmatter/dmls/docs/features.md:38` | `no_side_effects` | dmls | documentation | `([`tests/no_side_effects.rs`](../tests/no_side_effects.rs)).` |
| `darkmatter/dmls/docs/hover.md:113` | `lsp_session` | dmls | documentation | `- **Integration** (`tests/lsp_session.rs`): a full   `initialize → didOpen → textDocument/hover` session over the in-memory   connection …` |
| `darkmatter/dmls/src/overlay/expressions.rs:1566` | `lsp_session` | dmls | source comment | `/// LSP response boundary; `tests/lsp_session.rs` proves that end of it.` |
| `darkmatter/lib/tests/l1/context_functions.rs:234` | `cli` | sniff-cli | source comment | `/// `sniff/cli/tests/cli.rs::test_repo_recent_commits_plain_is_the_concatenated_per_commit_blocks`` |
| `docs/comment-quality.md:293` | `canonical_dispatch` | claudine | documentation | ``claudine/lib/tests/canonical_dispatch.rs`, which exercises the full` |
| `renderable/docs/layout-and-style.md:405` | `level2_render_tree_style` | biscuit-terminal-cli | documentation | ``biscuit-terminal/cli/tests/level2_render_tree_style.rs` continue to` |
| `schematic/gen/tests/fixtures/postman/README.md:10` | `postman_schema` | schematic-gen | documentation | `Used by `schematic/gen/tests/postman_schema.rs` to validate every emitted` |
| `sniff/docs/cli/repo_recent-commits.md:30` | `bench_ids_sync` | sniff | documentation | `- modified: sniff/lib/tests/bench_ids_sync.rs` |
| `sniff/docs/cli/repo_recent-commits.md:31` | `uv_with_install_plan` | sniff | documentation | `- added: sniff/lib/tests/uv_with_install_plan.rs` |
| `sniff/docs/cli/repo_source-code-changes.md:47` | `bench_ids_sync` | sniff | documentation | `- modified: sniff/lib/tests/bench_ids_sync.rs` |
| `sniff/docs/cli/repo_source-code-changes.md:48` | `uv_with_install_plan` | sniff | documentation | `- added: sniff/lib/tests/uv_with_install_plan.rs` |
| `sniff/lib/benches/support/bench_ids.rs:7` | `bench_ids_sync` | sniff | source comment | `//! `sniff/lib/tests/bench_ids_sync.rs` asserts the two stay in sync.` |
| `sniff/lib/benches/support/bench_ids.rs:11` | `bench_ids_sync` | sniff | source comment | `//! integration test at `sniff/lib/tests/bench_ids_sync.rs` (where every` |
| `sniff/lib/benches/support/bench_ids.rs:74` | `bench_ids_sync` | sniff | source comment | `// Unit tests for this module live in `sniff/lib/tests/bench_ids_sync.rs`,` |
| `tools/test-toolkit/tests/ci_workflow_contracts.rs:7053` | `windows_captured_stdout` | biscuit-tui-cli | **string literal in code** | `let test = read("biscuit-tui/cli/tests/windows_captured_stdout.rs");` |
| `tree-hugger/lib/README.md:144` | `lint_diagnostics` | tree-hugger | documentation | `4. **Diagnostics coverage** - Add regression tests for semantic lint rules and syntax diagnostics in `tests/lint_diagnostics.rs` and `tes…` |
| `tree-hugger/lib/README.md:144` | `tree_file` | tree-hugger | documentation | `4. **Diagnostics coverage** - Add regression tests for semantic lint rules and syntax diagnostics in `tests/lint_diagnostics.rs` and `tes…` |
| `tree-hugger/lib/README.md:154` | `tree_file` | tree-hugger | documentation | `Tests are in `tests/tree_file.rs` and follow this pattern:` |

## Reviewer notes

- `tools/test-toolkit/tests/ci_workflow_contracts.rs` reads
  `biscuit-tui/cli/tests/windows_captured_stdout.rs` at run time; moving that
  file breaks a test in another package, not just a comment.
- `.claude/skills/rust-testing/SKILL.md:777` names `cli/tests/spawn_site_guard.rs`
  "in sniff"; the path rule cannot attribute a bare `cli/` prefix from a
  cross-area skill, so it is listed here instead of in the table above.
- `tools/test-audit/fixtures/claudine-compat/families.json` holds sixteen
  `claudine::<target>` suite ids. It is a frozen replay fixture whose README
  forbids live edits, so it is classified historical.
- Selector-free consumers were checked and hold no target names:
  `.config/nextest.toml` filters these packages only by `package(...)` and
  tier-prefix `test(/(^|::)level2_/)` patterns, which survive module nesting;
  no justfile under `just/` and no `.github/` workflow names a target with
  `binary(...)` or `binary_id(...)`.

## historical (by file; never rewritten)

| File | Hits |
|---|---:|
| `biscuit-file/features/_completed/2026-04-11-implicit-relative-path/plan.md` | 3 |
| `biscuit-icon/features/_completed/2026-06-07-kickoff/plan.md` | 1 |
| `biscuit-terminal/features/2026-06-29-app-metadata/review-1.md` | 1 |
| `biscuit-terminal/features/2026-06-29-app-metadata/review-2.md` | 1 |
| `biscuit-terminal/features/2026-06-29-app-metadata/review-3.md` | 1 |
| `biscuit-terminal/features/2026-06-29-app-metadata/review-4.md` | 1 |
| `biscuit-terminal/features/2026-06-29-app-metadata/review-5.md` | 1 |
| `biscuit-terminal/features/2026-06-29-app-metadata/review-6.md` | 1 |
| `biscuit-terminal/features/2026-06-29-app-metadata/review-7.md` | 1 |
| `biscuit-terminal/features/2026-06-29-app-metadata/review-8.md` | 1 |
| `biscuit-terminal/features/2026-06-29-app-metadata/review-9.md` | 1 |
| `biscuit-terminal/features/_completed/2026-05-02-apple-terminal/plan.md` | 2 |
| `biscuit-terminal/features/_completed/2026-05-02-apple-terminal/review-1.md` | 5 |
| `biscuit-terminal/features/_completed/2026-05-02-apple-terminal/review-2.md` | 2 |
| `biscuit-terminal/features/_completed/2026-05-02-apple-terminal/review-5.md` | 1 |
| `biscuit-terminal/features/_completed/2026-05-02-apple-terminal/review-plan-1.md` | 2 |
| `biscuit-terminal/features/_completed/2026-05-02-apple-terminal/review-plan-2.md` | 5 |
| `biscuit-terminal/features/_completed/2026-05-02-apple-terminal/review-plan-3.md` | 2 |
| `biscuit-terminal/features/_completed/2026-05-02-apple-terminal/review-plan-4.md` | 3 |
| `biscuit-terminal/features/_completed/2026-05-02-l1-l2-tests/plan.md` | 2 |
| `biscuit-terminal/features/_completed/2026-05-02-l1-l2-tests/review-1.md` | 5 |
| `biscuit-terminal/features/_completed/2026-05-02-l1-l2-tests/review-2.md` | 5 |
| `biscuit-terminal/features/_completed/2026-05-02-l1-l2-tests/review-plan-1.md` | 11 |
| `biscuit-terminal/features/_completed/2026-05-02-l1-l2-tests/review-plan-2.md` | 12 |
| `biscuit-terminal/features/_completed/2026-05-05-prose-plus/plan.md` | 3 |
| `biscuit-tui/features/_completed/2026-04-23-choose-cli/plan.md` | 5 |
| `biscuit-tui/features/_completed/2026-04-23-choose-cli/review-1.md` | 1 |
| `biscuit-tui/features/_completed/2026-04-23-choose-cli/review-plan-1.md` | 2 |
| `biscuit-tui/features/_completed/2026-04-23-choose-cli/review-plan-2.md` | 3 |
| `biscuit-tui/features/_completed/2026-04-28-choose-one-improvements/review-10.md` | 3 |
| `biscuit-tui/features/_completed/2026-04-28-choose-one-improvements/review-7.md` | 4 |
| `biscuit-tui/features/_completed/2026-04-28-choose-one-improvements/review-8.md` | 2 |
| `biscuit-tui/features/_completed/2026-04-28-choose-one-improvements/review-plan-10.md` | 6 |
| `biscuit-tui/features/_completed/2026-04-28-choose-one-improvements/review-plan-4.md` | 2 |
| `biscuit-tui/features/_completed/2026-04-28-choose-one-improvements/review-plan-7.md` | 4 |
| `biscuit-tui/features/_completed/2026-04-28-choose-one-improvements/review-plan-8.md` | 2 |
| `biscuit-tui/features/_completed/2026-04-28-choose-one-improvements/review-plan-9.md` | 2 |
| `biscuit-tui/features/_completed/2026-06-19-review-findings/review-3.md` | 3 |
| `biscuit-tui/features/_completed/2026-06-19-review-findings/review-4.md` | 3 |
| `biscuit-tui/features/_completed/2026-06-19-review-findings/review-5.md` | 2 |
| `biscuit-tui/features/_completed/2026-06-19-review-findings/windows-captured-stdout-repro.md` | 2 |
| `claudine/features/_completed/2026-04-18-opencode-reporting-improvements/plan.md` | 1 |
| `claudine/features/_completed/2026-04-26-centralized-providers/review-plan-3.md` | 3 |
| `claudine/features/_completed/2026-04-26-fix-kimi/plan.md` | 4 |
| `claudine/features/_completed/2026-04-26-fix-kimi/review-plan-1.md` | 2 |
| `claudine/features/_completed/2026-05-08-testing-setup-teardown/plan.md` | 1 |
| `claudine/features/_completed/2026-06-28-real-errors/review-11.md` | 1 |
| `claudine/features/_completed/2026-06-28-real-errors/review-5.md` | 1 |
| `claudine/features/_completed/2026-06-28-real-errors/review-6.md` | 1 |
| `claudine/features/_completed/2026-07-13-error-propogation/plan.md` | 1 |
| `claudine/features/_completed/2026-07-13-proxy-with/notes/baseline.md` | 1 |
| `claudine/features/_completed/2026-07-13-proxy-with/plan.md` | 7 |
| `claudine/features/_completed/2026-07-13-proxy-with/review-3.md` | 1 |
| `claudine/fixes/2026-09-17-remove-strict-mode/spike-results.md` | 4 |
| `claudine/fixes/_completed/2026-04-08-different-configs/review.md` | 1 |
| `claudine/fixes/_completed/2026-05-12-opencode-stderr-returns/review-1.md` | 1 |
| `claudine/fixes/_completed/2026-06-10-error-suggest-format/review-3.md` | 1 |
| `claudine/fixes/_completed/2026-06-10-error-suggest-format/review-4.md` | 1 |
| `claudine/fixes/_completed/2026-07-20-claudine-mega-merge/phase4-test-map.md` | 2 |
| `claudine/fixes/_completed/2026-07-31-claudine-win/implementation-notes/phase-0-native-windows-baseline.md` | 3 |
| `claudine/fixes/_completed/2026-07-31-claudine-win/spec.md` | 5 |
| `claudine/fixes/_completed/2026-08-01-cli-slow-tests/log.md` | 3 |
| `claudine/fixes/_completed/2026-09-03-tts-not-finishing/log.md` | 1 |
| `claudine/fixes/_completed/2026-09-03-tts-not-finishing/phase-1-baseline.md` | 1 |
| `claudine/fixes/_completed/2026-09-03-tts-not-finishing/plan.md` | 1 |
| `claudine/fixes/_completed/2026-09-05-inline-flow-and-validations/plan.md` | 2 |
| `claudine/fixes/_completed/2026-09-05-inline-flow-and-validations/review-1.md` | 1 |
| `claudine/fixes/_completed/2026-09-05-inline-flow-and-validations/review-2.md` | 1 |
| `claudine/fixes/_completed/2026-09-07-faster-claudine-tests/deferred-performance-measurement.md` | 2 |
| `claudine/fixes/_completed/2026-09-07-faster-claudine-tests/families.json` | 24 |
| `claudine/fixes/_completed/2026-09-07-faster-claudine-tests/inventory.md` | 6 |
| `claudine/fixes/_completed/2026-09-07-faster-claudine-tests/log.md` | 7 |
| `claudine/fixes/_completed/2026-09-07-faster-claudine-tests/plan.md` | 1 |
| `claudine/fixes/_completed/2026-09-07-faster-claudine-tests/results.md` | 4 |
| `claudine/fixes/_completed/2026-09-07-faster-claudine-tests/review-1.md` | 1 |
| `claudine/fixes/_completed/2026-09-07-faster-claudine-tests/review-2.md` | 2 |
| `claudine/fixes/_completed/2026-09-12-shadow-home/implementation-log.md` | 6 |
| `claudine/fixes/_completed/2026-09-12-shadow-home/review-2.md` | 1 |
| `claudine/fixes/_completed/2026-09-13-better-static-analysis/implementation-log.md` | 2 |
| `claudine/fixes/_unscheduled/test-suite-residuals/spec.md` | 2 |
| `darkmatter/features/2026-09-22-lifecycle-events/spike-results.md` | 2 |
| `darkmatter/features/_completed/2026-06-01-url-referencing/review-9.md` | 1 |
| `darkmatter/features/_completed/2026-07-13-meta-schema/phase5-baseline-replay.md` | 1 |
| `darkmatter/features/_completed/2026-07-13-more-is-more/log.md` | 2 |
| `darkmatter/features/_completed/2026-07-13-more-is-more/review-18.md` | 3 |
| `darkmatter/features/_completed/2026-07-13-more-is-more/review-19.md` | 1 |
| `darkmatter/features/_completed/2026-07-13-more-is-more/review-27.md` | 1 |
| `darkmatter/features/_completed/2026-07-14-invalid-frontmatter/log.md` | 5 |
| `darkmatter/features/_completed/2026-07-14-invalid-frontmatter/review-2.md` | 4 |
| `darkmatter/features/_completed/2026-07-14-invalid-frontmatter/review-3.md` | 4 |
| `darkmatter/features/_completed/2026-09-09-more-context/implementation-log.md` | 8 |
| `darkmatter/fixes/_completed/2026-07-20-dm-mega-merge/resolution-record.md` | 17 |
| `darkmatter/fixes/_completed/2026-09-03-dmls-regression/log.md` | 1 |
| `darkmatter/fixes/_completed/2026-09-07-faster-darkmatter-tests/attribution/ci-macos-latest-L1.json` | 5 |
| `darkmatter/fixes/_completed/2026-09-07-faster-darkmatter-tests/attribution/ci-macos-latest-L2.json` | 1 |
| `darkmatter/fixes/_completed/2026-09-07-faster-darkmatter-tests/attribution/ci-ubuntu-latest-L1.json` | 5 |
| `darkmatter/fixes/_completed/2026-09-07-faster-darkmatter-tests/attribution/ci-ubuntu-latest-L2.json` | 1 |
| `darkmatter/fixes/_completed/2026-09-07-faster-darkmatter-tests/attribution/ci-windows-latest-L1.json` | 5 |
| `darkmatter/fixes/_completed/2026-09-07-faster-darkmatter-tests/attribution/ci-wsl2-ubuntu-L1.json` | 5 |
| `darkmatter/fixes/_completed/2026-09-07-faster-darkmatter-tests/attribution/local-l1-by-binary.json` | 13 |
| `darkmatter/fixes/_completed/2026-09-07-faster-darkmatter-tests/attribution/local-l2-by-binary.json` | 2 |
| `darkmatter/fixes/_completed/2026-09-07-faster-darkmatter-tests/families.json` | 9 |
| `darkmatter/fixes/_completed/2026-09-07-faster-darkmatter-tests/inventory.md` | 2 |
| `darkmatter/fixes/_completed/2026-09-07-faster-darkmatter-tests/log.md` | 9 |
| `darkmatter/fixes/_completed/2026-09-07-faster-darkmatter-tests/review-1.md` | 3 |
| `darkmatter/fixes/_completed/2026-09-07-faster-darkmatter-tests/review-2.md` | 3 |
| `darkmatter/fixes/_completed/2026-09-16-content-policy-no-cache/implementation-log.md` | 3 |
| `darkmatter/fixes/_completed/2026-09-16-content-policy-no-cache/review-3.md` | 1 |
| `features/2026-09-21-consolidated-test-binaries/implementation-log.md` | 2 |
| `fixes/_complete/2026-09-13-cicd-redundancies/acceptance.md` | 2 |
| `fixes/_complete/2026-09-13-cicd-redundancies/plan.md` | 1 |
| `fixes/_complete/2026-09-13-cicd-redundancies/rulings.md` | 4 |
| `fixes/_completed/2026-09-18-tmux-flake/spec.md` | 1 |
| `playa/docs/plans/2026-04-29-playa-stuck-device-resilience.md` | 2 |
| `renderable/features/_completed/2026-04-17-layout-and-style/review-3.md` | 1 |
| `renderable/features/_completed/2026-04-17-layout-and-style/review-4.md` | 1 |
| `renderable/features/_completed/2026-04-17-layout-and-style/review-5.md` | 2 |
| `renderable/features/_completed/2026-04-17-layout-and-style/review-6.md` | 1 |
| `renderable/features/_completed/2026-05-17-prose-cross-target/review-3.md` | 3 |
| `renderable/features/_completed/2026-05-17-prose-cross-target/review-4.md` | 2 |
| `renderable/features/_completed/2026-05-17-prose-cross-target/review-5.md` | 2 |
| `renderable/features/_completed/2026-05-17-prose-cross-target/review-6.md` | 2 |
| `renderable/features/_completed/2026-05-17-prose-cross-target/review-7.md` | 2 |
| `renderable/features/_completed/2026-05-17-prose-cross-target/review-8.md` | 1 |
| `renderable/features/_completed/2026-05-19-pushing-toward-ir/stage1-and-2/StatusBlock-review-1.md` | 1 |
| `renderable/features/_completed/2026-05-19-pushing-toward-ir/stage3-plan.md` | 2 |
| `renderable/features/_completed/2026-06-04-renderer-folds/review-2.md` | 1 |
| `renderable/features/_completed/2026-06-04-renderer-folds/review-3.md` | 1 |
| `renderable/features/_completed/2026-06-04-renderer-folds/review-4.md` | 1 |
| `renderable/features/_completed/2026-06-08-prose-cells/review-3.md` | 1 |
| `renderable/features/_completed/2026-06-08-prose-cells/review-4.md` | 1 |
| `renderable/features/_completed/2026-06-08-prose-cells/review-6.md` | 1 |
| `renderable/features/_completed/2026-06-08-prose-cells/review-7.md` | 2 |
| `renderable/features/_completed/2026-06-08-prose-cells/review-8.md` | 2 |
| `renderable/features/_completed/2026-06-08-prose-cells/review-9.md` | 2 |
| `schematic/features/_completed/2026-05-07-artificial-analysis/review-1.md` | 1 |
| `schematic/features/_completed/2026-05-07-artificial-analysis/review-2.md` | 1 |
| `schematic/features/_completed/2026-05-07-artificial-analysis/review-3.md` | 1 |
| `schematic/features/_completed/2026-05-07-artificial-analysis/review-plan-2.md` | 1 |
| `schematic/features/_completed/ergonomics-and-postman-projects/review-3.md` | 4 |
| `schematic/features/_completed/ergonomics-and-postman-projects/review-plan-1.md` | 3 |
| `schematic/features/_completed/ergonomics-and-postman-projects/review-plan-2.md` | 3 |
| `schematic/features/_completed/ergonomics-and-postman-projects/review-plan-3.md` | 10 |
| `sniff/features/_completed/2026-04-22-package-areas/review-plan-1.md` | 3 |
| `sniff/features/_completed/2026-04-22-package-areas/review-plan-2.md` | 2 |
| `sniff/features/_completed/2026-04-25-repo-pr/plan.md` | 1 |
| `sniff/features/_completed/2026-04-25-repo-pr/review-plan-1.md` | 1 |
| `sniff/features/_completed/2026-04-28-incorrect-json/review-plan-1.md` | 12 |
| `sniff/features/_completed/2026-04-28-incorrect-json/review-plan-2.md` | 11 |
| `sniff/features/_completed/2026-04-28-incorrect-json/review-plan-4.md` | 1 |
| `sniff/features/_completed/2026-05-27-scope-complete-json/plan.md` | 2 |
| `sniff/features/_completed/2026-06-02-repo-remote-improvements/review-2.md` | 1 |
| `sniff/features/_completed/2026-06-09-git-status-touch-up/review-4.md` | 1 |
| `sniff/features/_completed/2026-06-12-git-identity-request/review-2.md` | 1 |
| `sniff/features/_completed/2026-06-14-more-repo/baseline-repo.json` | 2 |
| `sniff/features/_completed/2026-06-14-more-repo/review-4.md` | 2 |
| `sniff/features/_completed/2026-06-14-more-repo/review-5.md` | 2 |
| `sniff/features/_completed/2026-06-14-more-repo/review-6.md` | 5 |
| `sniff/features/_completed/2026-06-15-improved-monorepo-capture/review-2.md` | 1 |
| `sniff/features/_completed/2026-06-15-improved-monorepo-capture/review-3.md` | 1 |
| `sniff/features/_completed/2026-06-15-improved-monorepo-capture/review-4.md` | 1 |
| `sniff/features/_completed/2026-06-15-improved-monorepo-capture/review-5.md` | 2 |
| `sniff/features/_completed/2026-06-20-faster-package-list/review-1.md` | 4 |
| `sniff/features/_completed/2026-06-20-faster-package-list/review-2.md` | 4 |
| `sniff/features/_completed/2026-06-20-faster-package-list/review-3.md` | 4 |
| `sniff/features/_completed/2026-07-16-performance/phases/_completed/01-work-accounting/spec.md` | 1 |
| `sniff/features/_completed/2026-07-16-performance/phases/_completed/02-reuse-and-scope/spec.md` | 2 |
| `sniff/features/_completed/2026-07-16-performance/phases/_completed/03-observation-index/spec.md` | 1 |
| `sniff/features/_completed/2026-09-15-recent-commits/implementation-log.md` | 11 |
| `sniff/features/_completed/2026-09-15-recent-commits/review-1.md` | 2 |
| `sniff/features/_completed/2026-09-15-recent-commits/review-2.md` | 1 |
| `sniff/features/_completed/2026-09-15-recent-commits/review-3.md` | 1 |
| `sniff/fixes/_completed/2026-06-17-repo-version/review-2.md` | 1 |
| `sniff/fixes/_completed/2026-06-17-repo-version/review-4.md` | 2 |
| `sniff/fixes/_completed/2026-09-07-faster-sniff-tests/families.json` | 58 |
| `sniff/fixes/_completed/2026-09-07-faster-sniff-tests/family-members.json` | 793 |
| `sniff/fixes/_completed/2026-09-07-faster-sniff-tests/inventory.md` | 2 |
| `sniff/fixes/_completed/2026-09-07-faster-sniff-tests/log.md` | 23 |
| `sniff/fixes/_completed/2026-09-07-faster-sniff-tests/results.md` | 1 |
| `sniff/fixes/_completed/2026-09-07-faster-sniff-tests/review-1.md` | 2 |
| `sniff/fixes/_completed/2026-09-07-faster-sniff-tests/review-2.md` | 2 |
| `sniff/fixes/_completed/2026-09-14-corrected-perf-flag/implementation-log.md` | 4 |
| `sniff/reviews/_completed/2026-05-02-performance-review/follow-up-plan.md` | 1 |
| `sniff/reviews/_completed/2026-05-02-performance-review/plan.md` | 3 |
| `tools/test-audit/fixtures/claudine-compat/families.json` | 24 |
