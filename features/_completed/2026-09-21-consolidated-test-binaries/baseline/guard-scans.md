---
kind: evidence
feature: 2026-09-21-consolidated-test-binaries
created: 2026-09-21
plan_phase: 1
revision: c0f911f5a43b869bccf4b822a9c83a6114f27dc1
host: macOS aarch64-apple-darwin
---

# Guard before-scans

The exact file sets three path-sensitive guards scan **before** any
migration. Later phases must reproduce each list, adjusted only by the
manifest's old→new path mapping (acceptance 7). A guard that passes while
scanning fewer files, or none, is a failure, not evidence.

## How these lists were produced

None of the guards prints its population, so each got a **temporary**,
uncommitted probe test appended to its own file. The probe called the
guard's own population functions (`collect_rust_files`, `governed_files`
with `governs_spawn` / `governs_isolation`, and
`test_toolkit::archive_guard::{collect_rust_files, is_eligible}`), which
avoids re-deriving any predicate. Each suite was then run with `cargo nextest
run -p <pkg> --test <guard> --no-capture`, and the three files were restored
with `git checkout --` (verified clean afterwards). Each real guard passed in
the same run:

| Guard | Suite result | Probe population | Guard's own census (`*-burn-down.jsonl`, `governed_files`) |
|---|---|---:|---:|
| claudine-cli `spawn_site_guard.rs`, spawn gate | 20/20 pass | 103 | 103 |
| claudine-cli `spawn_site_guard.rs`, isolation gate | (same run) | 88 | 88 |
| darkmatter-cli `spawn_site_guard.rs`, spawn gate | 24/24 pass | 42 | 42 |
| darkmatter-cli `spawn_site_guard.rs`, isolation gate | (same run) | 40 | 40 |
| `tools/test-toolkit/tests/archive_path_guard.rs` (the repository archive-path guard) | 3/3 pass | 348 files under the four `tests/` trees (348 eligible) | full tree: see summary below |

The probe count equals the guard's own census for every gate, so the lists
below are the populations the guards actually enforce.

Archive-path guard summary lines from the same run:

```text
archive-path guard: exemption maintenance checked 5 live form site(s) across the full tree
archive-path guard: mode=full-tree files-checked=3476 ineligible=0 violations=5
archive-path guard: scope reason — BISCUIT_ARCHIVE_GUARD_PLAN is unset; the standalone default checks the full tree
```

The plan also names `scripts/ci-build-archive-tests.rs`. Its relocation
fixture (`slow_real_package_archives_read_their_fixtures_from_the_consumers_checkout`)
relocates `test-toolkit` and `biscuit-file` only. It scans none of the four
packages' sources, so none of its file sets changes under this feature. The
source scan that applies to the four packages is the archive-path guard
above. Its allow-list (`ALLOWED` in `tools/test-toolkit/src/archive_guard.rs`)
names no file under the four `tests/` trees today.

## What changes after migration, and what must not

- **Spawn and isolation gates** key their exemptions and allow-lists on the
  path relative to `tests/` (`relative.starts_with("common/")`, the file-name
  `real_` prefix, and `SPAWN_ALLOWLIST` entries). After the move each entry
  becomes `<target>/<file>`. The **set of files** must map one-to-one through
  the manifest: same count, and each old path's image present. `common/`
  must stay excluded. A path that lands under a new directory not named
  `common/` stays governed, which is intended.
- **Walked sets** grow only by the new `main.rs` crate roots (one per
  consolidated target). Every other walked file must be the image of an old
  one.
- **Archive-path guard** eligibility is by path component (`SKIPPED_DIRS`)
  and file name. The new `tests/<target>/` directories (`l1`, `level2`,
  `level3`, `level3-terminal`, `level3-browser`, `browser`, `real`) collide
  with no skipped component, so eligibility must be unchanged for every
  mapped file.

## claudine-cli `spawn_site_guard.rs` (paths relative to `claudine/cli/tests/`)

### walked (148)

```text
agent_cwd.rs
argv_normalization.rs
characterization_error_routes.rs
cli_process_fixture.rs
command_routing.rs
common/completion.rs
common/host_tools.rs
common/incomplete_subagents.rs
common/mod.rs
common/pty.rs
common/review_router.rs
common/source_scan.rs
common/wrap.rs
completion_cli.rs
completion_compose.rs
completion_contract.rs
completion_inline_compose.rs
completion_perf.rs
completion_resolution_round_trip.rs
completion_sequence.rs
completion_setter.rs
compose_caller_file_provenance.rs
compose_cli.rs
compose_frontmatter_model.rs
compose_header_first.rs
compose_initialize_acceptance.rs
compose_initialize_staged_boot.rs
compose_interactive_timeout_cli.rs
compose_removed_validation_keys.rs
compose_repository_context.rs
compose_schema_cli.rs
compose_system_prompt_lifetime.rs
compose_ttff_perf.rs
composition_outputs.rs
composition_seams.rs
contamination_probes.rs
context_command.rs
contextual_errors.rs
ctx_launch_anchor.rs
detached_audio.rs
diagnostic_discovery.rs
dispatch_inventory.rs
effective_diagnostic_render.rs
error_guards.rs
error_guards/source_scan.rs
errors_command.rs
handle_blocking_output.rs
handle_deadline.rs
handle_repo_config.rs
hooks_cli.rs
inline_completion_lifecycle.rs
inline_compose_cli.rs
inline_compose_hash.rs
inline_compose_sequence_mismatch.rs
level1_compose_autocomplete_failure_pty.rs
level1_dry_run_pty.rs
level1_inline_compose_mismatch_pty.rs
level1_provided_partial_file_pty.rs
level1_provider_overlay_home.rs
level1_pty_wrapper_summary.rs
level1_review_router_partial_pty.rs
level1_schema_prompt_pty.rs
level1_structured_error_message.rs
level2_auto_complete_chooser.rs
level2_auto_complete_operation_file.rs
level2_context_capture.rs
level2_dry_run_approval_capture.rs
level2_dry_run_metadata_capture.rs
level2_explicit_operation_file_miss.rs
level2_file_resolution_capture.rs
level2_incomplete_subagents_capture.rs
level2_initialize_generated_transclusion.rs
level2_inline_compose_mismatch_capture.rs
level2_interrupt_feedback_capture.rs
level2_invalid_file_reference_capture.rs
level2_lifecycle_action_forms.rs
level2_lifecycle_control.rs
level2_lifecycle_dispatch.rs
level2_lifecycle_loop.rs
level2_malformed_frontmatter_capture.rs
level2_perf_capture.rs
level2_prompt_reporting_capture.rs
level2_provided_partial_file_capture.rs
level2_provider_overlay_capture.rs
level2_removed_validation_key_capture.rs
level2_schema_parse_capture.rs
level2_sequence_task_stream_capture.rs
level2_stalled_generation_capture.rs
level2_typed_error_render_capture.rs
level2_windows_provided_partial_file_capture.rs
level2_wrap_ctrl_c_loop_wedge_tmux.rs
level2_wrap_ctrl_c_tmux.rs
level3_auto_complete_chooser.rs
level3_linux_sequence_ctrl_c.rs
level3_sequence_ctrl_c.rs
level3_windows_sequence_ctrl_c.rs
level3_wrap_ctrl_c.rs
loop_cli.rs
loop_initialize_state.rs
mcp_cli.rs
prompt_reporting.rs
propagated_context_fixtures.rs
protect_cli.rs
provider_error_finalize.rs
real_inline_write_grant.rs
real_opencode_yolo_subagent.rs
real_pi_steering.rs
run_harness_loop_call_sites.rs
sequence_budget.rs
sequence_cli.rs
sequence_ctrl_c_windows.rs
sequence_errors_cli.rs
sequence_groups.rs
sequence_initialize_include_preflight.rs
sequence_jit.rs
sequence_magic_reference.rs
sequence_overlay_pty.rs
sequence_perf.rs
sequence_prompt_property.rs
sequence_schema.rs
sequence_sources_cli.rs
shipped_prompt_contract.rs
shipped_prompt_route_drift.rs
shipped_prompts.rs
skills_integration.rs
spawn_site_guard.rs
system_prompt_perf_bench.rs
test_placement.rs
wrap_antigravity_exit_signal.rs
wrap_basics.rs
wrap_compose_agent.rs
wrap_compose_exec.rs
wrap_compose_preflight.rs
wrap_compose_validation.rs
wrap_ctrl_c_windows.rs
wrap_direct_argv.rs
wrap_incomplete_subagents.rs
wrap_inline_compose.rs
wrap_inline_compose_interactive.rs
wrap_opencode.rs
wrap_opencode_models.rs
wrap_perf.rs
wrap_provider_flags.rs
wrap_sequence_composition.rs
wrap_sigint.rs
wrap_structured_stream.rs
wrap_watchdog_startup_stall.rs
wrap_watchdog_timeout.rs
```

### spawn gate — governed (103)

```text
agent_cwd.rs
argv_normalization.rs
characterization_error_routes.rs
cli_process_fixture.rs
command_routing.rs
completion_cli.rs
completion_compose.rs
completion_contract.rs
completion_inline_compose.rs
completion_perf.rs
completion_resolution_round_trip.rs
completion_sequence.rs
completion_setter.rs
compose_caller_file_provenance.rs
compose_cli.rs
compose_frontmatter_model.rs
compose_header_first.rs
compose_initialize_acceptance.rs
compose_initialize_staged_boot.rs
compose_interactive_timeout_cli.rs
compose_removed_validation_keys.rs
compose_repository_context.rs
compose_schema_cli.rs
compose_system_prompt_lifetime.rs
compose_ttff_perf.rs
composition_outputs.rs
composition_seams.rs
contamination_probes.rs
context_command.rs
contextual_errors.rs
ctx_launch_anchor.rs
detached_audio.rs
diagnostic_discovery.rs
dispatch_inventory.rs
effective_diagnostic_render.rs
error_guards.rs
error_guards/source_scan.rs
errors_command.rs
handle_blocking_output.rs
handle_deadline.rs
handle_repo_config.rs
hooks_cli.rs
inline_completion_lifecycle.rs
inline_compose_cli.rs
inline_compose_hash.rs
inline_compose_sequence_mismatch.rs
level1_compose_autocomplete_failure_pty.rs
level1_dry_run_pty.rs
level1_inline_compose_mismatch_pty.rs
level1_provided_partial_file_pty.rs
level1_provider_overlay_home.rs
level1_pty_wrapper_summary.rs
level1_review_router_partial_pty.rs
level1_schema_prompt_pty.rs
level1_structured_error_message.rs
loop_cli.rs
loop_initialize_state.rs
mcp_cli.rs
prompt_reporting.rs
propagated_context_fixtures.rs
protect_cli.rs
provider_error_finalize.rs
run_harness_loop_call_sites.rs
sequence_budget.rs
sequence_cli.rs
sequence_ctrl_c_windows.rs
sequence_errors_cli.rs
sequence_groups.rs
sequence_initialize_include_preflight.rs
sequence_jit.rs
sequence_magic_reference.rs
sequence_overlay_pty.rs
sequence_perf.rs
sequence_prompt_property.rs
sequence_schema.rs
sequence_sources_cli.rs
shipped_prompt_contract.rs
shipped_prompt_route_drift.rs
shipped_prompts.rs
skills_integration.rs
spawn_site_guard.rs
system_prompt_perf_bench.rs
test_placement.rs
wrap_antigravity_exit_signal.rs
wrap_basics.rs
wrap_compose_agent.rs
wrap_compose_exec.rs
wrap_compose_preflight.rs
wrap_compose_validation.rs
wrap_ctrl_c_windows.rs
wrap_direct_argv.rs
wrap_incomplete_subagents.rs
wrap_inline_compose.rs
wrap_inline_compose_interactive.rs
wrap_opencode.rs
wrap_opencode_models.rs
wrap_perf.rs
wrap_provider_flags.rs
wrap_sequence_composition.rs
wrap_sigint.rs
wrap_structured_stream.rs
wrap_watchdog_startup_stall.rs
wrap_watchdog_timeout.rs
```

### isolation gate — governed (88)

```text
agent_cwd.rs
argv_normalization.rs
characterization_error_routes.rs
cli_process_fixture.rs
command_routing.rs
completion_contract.rs
completion_perf.rs
completion_resolution_round_trip.rs
compose_caller_file_provenance.rs
compose_cli.rs
compose_frontmatter_model.rs
compose_header_first.rs
compose_initialize_acceptance.rs
compose_initialize_staged_boot.rs
compose_interactive_timeout_cli.rs
compose_removed_validation_keys.rs
compose_repository_context.rs
compose_schema_cli.rs
compose_system_prompt_lifetime.rs
compose_ttff_perf.rs
composition_outputs.rs
contamination_probes.rs
context_command.rs
contextual_errors.rs
ctx_launch_anchor.rs
detached_audio.rs
effective_diagnostic_render.rs
errors_command.rs
handle_blocking_output.rs
handle_deadline.rs
handle_repo_config.rs
hooks_cli.rs
inline_completion_lifecycle.rs
inline_compose_cli.rs
inline_compose_hash.rs
inline_compose_sequence_mismatch.rs
level1_compose_autocomplete_failure_pty.rs
level1_dry_run_pty.rs
level1_inline_compose_mismatch_pty.rs
level1_provided_partial_file_pty.rs
level1_provider_overlay_home.rs
level1_pty_wrapper_summary.rs
level1_review_router_partial_pty.rs
level1_schema_prompt_pty.rs
level1_structured_error_message.rs
loop_cli.rs
loop_initialize_state.rs
mcp_cli.rs
prompt_reporting.rs
propagated_context_fixtures.rs
protect_cli.rs
provider_error_finalize.rs
sequence_budget.rs
sequence_cli.rs
sequence_ctrl_c_windows.rs
sequence_errors_cli.rs
sequence_groups.rs
sequence_initialize_include_preflight.rs
sequence_jit.rs
sequence_magic_reference.rs
sequence_overlay_pty.rs
sequence_perf.rs
sequence_prompt_property.rs
sequence_schema.rs
sequence_sources_cli.rs
shipped_prompt_contract.rs
shipped_prompts.rs
skills_integration.rs
wrap_antigravity_exit_signal.rs
wrap_basics.rs
wrap_compose_agent.rs
wrap_compose_exec.rs
wrap_compose_preflight.rs
wrap_compose_validation.rs
wrap_ctrl_c_windows.rs
wrap_direct_argv.rs
wrap_incomplete_subagents.rs
wrap_inline_compose.rs
wrap_inline_compose_interactive.rs
wrap_opencode.rs
wrap_opencode_models.rs
wrap_perf.rs
wrap_provider_flags.rs
wrap_sequence_composition.rs
wrap_sigint.rs
wrap_structured_stream.rs
wrap_watchdog_startup_stall.rs
wrap_watchdog_timeout.rs
```

## darkmatter-cli `spawn_site_guard.rs` (paths relative to `darkmatter/cli/tests/`)

### walked (58)

```text
clean.rs
clean_frontmatter.rs
clean_json.rs
clean_schema.rs
code_block.rs
common/fixture.rs
common/level2.rs
common/mod.rs
common/protected_env.rs
common/source_scan.rs
compose_array_rendering.rs
compose_base_schema.rs
compose_basic.rs
compose_interpolation.rs
compose_layout.rs
compose_page_blocks.rs
compose_perf.rs
compose_refs_and_missing.rs
compose_remote_caching.rs
compose_schema.rs
compose_schema_file_rewrite.rs
compose_shell.rs
compose_state_set.rs
compose_terminal_detection.rs
compose_transclusion.rs
delta.rs
get_set_rm.rs
graph.rs
hash.rs
hash_directory.rs
hash_kind_save_diff.rs
help.rs
layout_alignment.rs
layout_fill.rs
layout_flags.rs
layout_style_frontmatter.rs
level2_code_block_styling.rs
level2_disclosure_blocks.rs
level2_errors.rs
level2_frontmatter_images.rs
level2_frontmatter_tables.rs
level2_harness_integrity.rs
level2_horizontal_rules.rs
level2_layout_dimensions.rs
level2_ordered_lists.rs
level2_schema_about.rs
level2_schema_validate.rs
md_process_fixture.rs
render_basic.rs
rm.rs
schema_about.rs
schema_detect.rs
schema_triggers.rs
schema_validate.rs
schema_validate_baseline.rs
spawn_site_guard.rs
toc.rs
validate_refs.rs
```

### spawn gate — governed (42)

```text
clean.rs
clean_frontmatter.rs
clean_json.rs
clean_schema.rs
code_block.rs
compose_array_rendering.rs
compose_base_schema.rs
compose_basic.rs
compose_interpolation.rs
compose_layout.rs
compose_page_blocks.rs
compose_perf.rs
compose_refs_and_missing.rs
compose_remote_caching.rs
compose_schema.rs
compose_schema_file_rewrite.rs
compose_shell.rs
compose_state_set.rs
compose_terminal_detection.rs
compose_transclusion.rs
delta.rs
get_set_rm.rs
graph.rs
hash.rs
hash_directory.rs
hash_kind_save_diff.rs
help.rs
layout_alignment.rs
layout_fill.rs
layout_flags.rs
layout_style_frontmatter.rs
md_process_fixture.rs
render_basic.rs
rm.rs
schema_about.rs
schema_detect.rs
schema_triggers.rs
schema_validate.rs
schema_validate_baseline.rs
spawn_site_guard.rs
toc.rs
validate_refs.rs
```

### isolation gate — governed (40)

```text
clean.rs
clean_frontmatter.rs
clean_json.rs
clean_schema.rs
code_block.rs
compose_array_rendering.rs
compose_base_schema.rs
compose_basic.rs
compose_interpolation.rs
compose_layout.rs
compose_page_blocks.rs
compose_perf.rs
compose_refs_and_missing.rs
compose_remote_caching.rs
compose_schema.rs
compose_schema_file_rewrite.rs
compose_shell.rs
compose_state_set.rs
compose_terminal_detection.rs
compose_transclusion.rs
delta.rs
get_set_rm.rs
graph.rs
hash.rs
hash_directory.rs
hash_kind_save_diff.rs
help.rs
layout_fill.rs
layout_flags.rs
layout_style_frontmatter.rs
md_process_fixture.rs
render_basic.rs
rm.rs
schema_about.rs
schema_detect.rs
schema_triggers.rs
schema_validate.rs
schema_validate_baseline.rs
toc.rs
validate_refs.rs
```

## archive-path guard: files under the four `tests/` trees (repository-relative; `path<TAB>eligible`)

### walked (348)

```text
biscuit-terminal/lib/tests/common/mod.rs	true
biscuit-terminal/lib/tests/common/pty.rs	true
biscuit-terminal/lib/tests/compose_parity.rs	true
biscuit-terminal/lib/tests/filesystem_parity.rs	true
biscuit-terminal/lib/tests/graph_expression_parity.rs	true
biscuit-terminal/lib/tests/horizontal_rule_parity.rs	true
biscuit-terminal/lib/tests/html_page_example.rs	true
biscuit-terminal/lib/tests/inline_content_matrix.rs	true
biscuit-terminal/lib/tests/inline_content_matrix_support/mod.rs	true
biscuit-terminal/lib/tests/integration.rs	true
biscuit-terminal/lib/tests/layout_matrix.rs	true
biscuit-terminal/lib/tests/layout_matrix_support/mod.rs	true
biscuit-terminal/lib/tests/level1_apple_terminal_prose.rs	true
biscuit-terminal/lib/tests/level1_clipboard.rs	true
biscuit-terminal/lib/tests/level1_cursor.rs	true
biscuit-terminal/lib/tests/level1_mode_2027.rs	true
biscuit-terminal/lib/tests/level1_osc_queries.rs	true
biscuit-terminal/lib/tests/level1_terminal_init.rs	true
biscuit-terminal/lib/tests/level1_terminal_osc_cache.rs	true
biscuit-terminal/lib/tests/level2_terminal_osc_wezterm.rs	true
biscuit-terminal/lib/tests/list_parity.rs	true
biscuit-terminal/lib/tests/mermaid_parity.rs	true
biscuit-terminal/lib/tests/metrics_tree_parity.rs	true
biscuit-terminal/lib/tests/ordered_list_parity.rs	true
biscuit-terminal/lib/tests/parity_helpers.rs	true
biscuit-terminal/lib/tests/perf_gate.rs	true
biscuit-terminal/lib/tests/prelude_exports.rs	true
biscuit-terminal/lib/tests/progress_parity.rs	true
biscuit-terminal/lib/tests/prose_cells_parity.rs	true
biscuit-terminal/lib/tests/render_comparison.rs	true
biscuit-terminal/lib/tests/render_tree_code_context.rs	true
biscuit-terminal/lib/tests/render_tree_component_parity.rs	true
biscuit-terminal/lib/tests/section_parity.rs	true
biscuit-terminal/lib/tests/status_block_parity.rs	true
biscuit-terminal/lib/tests/status_parity.rs	true
biscuit-terminal/lib/tests/table_parity.rs	true
biscuit-terminal/lib/tests/terminal_image_parity.rs	true
biscuit-terminal/lib/tests/text_block_parity.rs	true
biscuit-terminal/lib/tests/todo_parity.rs	true
biscuit-terminal/lib/tests/tree_layout.rs	true
biscuit-terminal/lib/tests/two_column_parity.rs	true
biscuit-terminal/lib/tests/unordered_list_parity.rs	true
claudine/cli/tests/agent_cwd.rs	true
claudine/cli/tests/argv_normalization.rs	true
claudine/cli/tests/characterization_error_routes.rs	true
claudine/cli/tests/cli_process_fixture.rs	true
claudine/cli/tests/command_routing.rs	true
claudine/cli/tests/common/completion.rs	true
claudine/cli/tests/common/host_tools.rs	true
claudine/cli/tests/common/incomplete_subagents.rs	true
claudine/cli/tests/common/mod.rs	true
claudine/cli/tests/common/pty.rs	true
claudine/cli/tests/common/review_router.rs	true
claudine/cli/tests/common/source_scan.rs	true
claudine/cli/tests/common/wrap.rs	true
claudine/cli/tests/completion_cli.rs	true
claudine/cli/tests/completion_compose.rs	true
claudine/cli/tests/completion_contract.rs	true
claudine/cli/tests/completion_inline_compose.rs	true
claudine/cli/tests/completion_perf.rs	true
claudine/cli/tests/completion_resolution_round_trip.rs	true
claudine/cli/tests/completion_sequence.rs	true
claudine/cli/tests/completion_setter.rs	true
claudine/cli/tests/compose_caller_file_provenance.rs	true
claudine/cli/tests/compose_cli.rs	true
claudine/cli/tests/compose_frontmatter_model.rs	true
claudine/cli/tests/compose_header_first.rs	true
claudine/cli/tests/compose_initialize_acceptance.rs	true
claudine/cli/tests/compose_initialize_staged_boot.rs	true
claudine/cli/tests/compose_interactive_timeout_cli.rs	true
claudine/cli/tests/compose_removed_validation_keys.rs	true
claudine/cli/tests/compose_repository_context.rs	true
claudine/cli/tests/compose_schema_cli.rs	true
claudine/cli/tests/compose_system_prompt_lifetime.rs	true
claudine/cli/tests/compose_ttff_perf.rs	true
claudine/cli/tests/composition_outputs.rs	true
claudine/cli/tests/composition_seams.rs	true
claudine/cli/tests/contamination_probes.rs	true
claudine/cli/tests/context_command.rs	true
claudine/cli/tests/contextual_errors.rs	true
claudine/cli/tests/ctx_launch_anchor.rs	true
claudine/cli/tests/detached_audio.rs	true
claudine/cli/tests/diagnostic_discovery.rs	true
claudine/cli/tests/dispatch_inventory.rs	true
claudine/cli/tests/effective_diagnostic_render.rs	true
claudine/cli/tests/error_guards.rs	true
claudine/cli/tests/error_guards/source_scan.rs	true
claudine/cli/tests/errors_command.rs	true
claudine/cli/tests/handle_blocking_output.rs	true
claudine/cli/tests/handle_deadline.rs	true
claudine/cli/tests/handle_repo_config.rs	true
claudine/cli/tests/hooks_cli.rs	true
claudine/cli/tests/inline_completion_lifecycle.rs	true
claudine/cli/tests/inline_compose_cli.rs	true
claudine/cli/tests/inline_compose_hash.rs	true
claudine/cli/tests/inline_compose_sequence_mismatch.rs	true
claudine/cli/tests/level1_compose_autocomplete_failure_pty.rs	true
claudine/cli/tests/level1_dry_run_pty.rs	true
claudine/cli/tests/level1_inline_compose_mismatch_pty.rs	true
claudine/cli/tests/level1_provided_partial_file_pty.rs	true
claudine/cli/tests/level1_provider_overlay_home.rs	true
claudine/cli/tests/level1_pty_wrapper_summary.rs	true
claudine/cli/tests/level1_review_router_partial_pty.rs	true
claudine/cli/tests/level1_schema_prompt_pty.rs	true
claudine/cli/tests/level1_structured_error_message.rs	true
claudine/cli/tests/level2_auto_complete_chooser.rs	true
claudine/cli/tests/level2_auto_complete_operation_file.rs	true
claudine/cli/tests/level2_context_capture.rs	true
claudine/cli/tests/level2_dry_run_approval_capture.rs	true
claudine/cli/tests/level2_dry_run_metadata_capture.rs	true
claudine/cli/tests/level2_explicit_operation_file_miss.rs	true
claudine/cli/tests/level2_file_resolution_capture.rs	true
claudine/cli/tests/level2_incomplete_subagents_capture.rs	true
claudine/cli/tests/level2_initialize_generated_transclusion.rs	true
claudine/cli/tests/level2_inline_compose_mismatch_capture.rs	true
claudine/cli/tests/level2_interrupt_feedback_capture.rs	true
claudine/cli/tests/level2_invalid_file_reference_capture.rs	true
claudine/cli/tests/level2_lifecycle_action_forms.rs	true
claudine/cli/tests/level2_lifecycle_control.rs	true
claudine/cli/tests/level2_lifecycle_dispatch.rs	true
claudine/cli/tests/level2_lifecycle_loop.rs	true
claudine/cli/tests/level2_malformed_frontmatter_capture.rs	true
claudine/cli/tests/level2_perf_capture.rs	true
claudine/cli/tests/level2_prompt_reporting_capture.rs	true
claudine/cli/tests/level2_provided_partial_file_capture.rs	true
claudine/cli/tests/level2_provider_overlay_capture.rs	true
claudine/cli/tests/level2_removed_validation_key_capture.rs	true
claudine/cli/tests/level2_schema_parse_capture.rs	true
claudine/cli/tests/level2_sequence_task_stream_capture.rs	true
claudine/cli/tests/level2_stalled_generation_capture.rs	true
claudine/cli/tests/level2_typed_error_render_capture.rs	true
claudine/cli/tests/level2_windows_provided_partial_file_capture.rs	true
claudine/cli/tests/level2_wrap_ctrl_c_loop_wedge_tmux.rs	true
claudine/cli/tests/level2_wrap_ctrl_c_tmux.rs	true
claudine/cli/tests/level3_auto_complete_chooser.rs	true
claudine/cli/tests/level3_linux_sequence_ctrl_c.rs	true
claudine/cli/tests/level3_sequence_ctrl_c.rs	true
claudine/cli/tests/level3_windows_sequence_ctrl_c.rs	true
claudine/cli/tests/level3_wrap_ctrl_c.rs	true
claudine/cli/tests/loop_cli.rs	true
claudine/cli/tests/loop_initialize_state.rs	true
claudine/cli/tests/mcp_cli.rs	true
claudine/cli/tests/prompt_reporting.rs	true
claudine/cli/tests/propagated_context_fixtures.rs	true
claudine/cli/tests/protect_cli.rs	true
claudine/cli/tests/provider_error_finalize.rs	true
claudine/cli/tests/real_inline_write_grant.rs	true
claudine/cli/tests/real_opencode_yolo_subagent.rs	true
claudine/cli/tests/real_pi_steering.rs	true
claudine/cli/tests/run_harness_loop_call_sites.rs	true
claudine/cli/tests/sequence_budget.rs	true
claudine/cli/tests/sequence_cli.rs	true
claudine/cli/tests/sequence_ctrl_c_windows.rs	true
claudine/cli/tests/sequence_errors_cli.rs	true
claudine/cli/tests/sequence_groups.rs	true
claudine/cli/tests/sequence_initialize_include_preflight.rs	true
claudine/cli/tests/sequence_jit.rs	true
claudine/cli/tests/sequence_magic_reference.rs	true
claudine/cli/tests/sequence_overlay_pty.rs	true
claudine/cli/tests/sequence_perf.rs	true
claudine/cli/tests/sequence_prompt_property.rs	true
claudine/cli/tests/sequence_schema.rs	true
claudine/cli/tests/sequence_sources_cli.rs	true
claudine/cli/tests/shipped_prompt_contract.rs	true
claudine/cli/tests/shipped_prompt_route_drift.rs	true
claudine/cli/tests/shipped_prompts.rs	true
claudine/cli/tests/skills_integration.rs	true
claudine/cli/tests/spawn_site_guard.rs	true
claudine/cli/tests/system_prompt_perf_bench.rs	true
claudine/cli/tests/test_placement.rs	true
claudine/cli/tests/wrap_antigravity_exit_signal.rs	true
claudine/cli/tests/wrap_basics.rs	true
claudine/cli/tests/wrap_compose_agent.rs	true
claudine/cli/tests/wrap_compose_exec.rs	true
claudine/cli/tests/wrap_compose_preflight.rs	true
claudine/cli/tests/wrap_compose_validation.rs	true
claudine/cli/tests/wrap_ctrl_c_windows.rs	true
claudine/cli/tests/wrap_direct_argv.rs	true
claudine/cli/tests/wrap_incomplete_subagents.rs	true
claudine/cli/tests/wrap_inline_compose.rs	true
claudine/cli/tests/wrap_inline_compose_interactive.rs	true
claudine/cli/tests/wrap_opencode.rs	true
claudine/cli/tests/wrap_opencode_models.rs	true
claudine/cli/tests/wrap_perf.rs	true
claudine/cli/tests/wrap_provider_flags.rs	true
claudine/cli/tests/wrap_sequence_composition.rs	true
claudine/cli/tests/wrap_sigint.rs	true
claudine/cli/tests/wrap_structured_stream.rs	true
claudine/cli/tests/wrap_watchdog_startup_stall.rs	true
claudine/cli/tests/wrap_watchdog_timeout.rs	true
darkmatter/cli/tests/clean.rs	true
darkmatter/cli/tests/clean_frontmatter.rs	true
darkmatter/cli/tests/clean_json.rs	true
darkmatter/cli/tests/clean_schema.rs	true
darkmatter/cli/tests/code_block.rs	true
darkmatter/cli/tests/common/fixture.rs	true
darkmatter/cli/tests/common/level2.rs	true
darkmatter/cli/tests/common/mod.rs	true
darkmatter/cli/tests/common/protected_env.rs	true
darkmatter/cli/tests/common/source_scan.rs	true
darkmatter/cli/tests/compose_array_rendering.rs	true
darkmatter/cli/tests/compose_base_schema.rs	true
darkmatter/cli/tests/compose_basic.rs	true
darkmatter/cli/tests/compose_interpolation.rs	true
darkmatter/cli/tests/compose_layout.rs	true
darkmatter/cli/tests/compose_page_blocks.rs	true
darkmatter/cli/tests/compose_perf.rs	true
darkmatter/cli/tests/compose_refs_and_missing.rs	true
darkmatter/cli/tests/compose_remote_caching.rs	true
darkmatter/cli/tests/compose_schema.rs	true
darkmatter/cli/tests/compose_schema_file_rewrite.rs	true
darkmatter/cli/tests/compose_shell.rs	true
darkmatter/cli/tests/compose_state_set.rs	true
darkmatter/cli/tests/compose_terminal_detection.rs	true
darkmatter/cli/tests/compose_transclusion.rs	true
darkmatter/cli/tests/delta.rs	true
darkmatter/cli/tests/get_set_rm.rs	true
darkmatter/cli/tests/graph.rs	true
darkmatter/cli/tests/hash.rs	true
darkmatter/cli/tests/hash_directory.rs	true
darkmatter/cli/tests/hash_kind_save_diff.rs	true
darkmatter/cli/tests/help.rs	true
darkmatter/cli/tests/layout_alignment.rs	true
darkmatter/cli/tests/layout_fill.rs	true
darkmatter/cli/tests/layout_flags.rs	true
darkmatter/cli/tests/layout_style_frontmatter.rs	true
darkmatter/cli/tests/level2_code_block_styling.rs	true
darkmatter/cli/tests/level2_disclosure_blocks.rs	true
darkmatter/cli/tests/level2_errors.rs	true
darkmatter/cli/tests/level2_frontmatter_images.rs	true
darkmatter/cli/tests/level2_frontmatter_tables.rs	true
darkmatter/cli/tests/level2_harness_integrity.rs	true
darkmatter/cli/tests/level2_horizontal_rules.rs	true
darkmatter/cli/tests/level2_layout_dimensions.rs	true
darkmatter/cli/tests/level2_ordered_lists.rs	true
darkmatter/cli/tests/level2_schema_about.rs	true
darkmatter/cli/tests/level2_schema_validate.rs	true
darkmatter/cli/tests/md_process_fixture.rs	true
darkmatter/cli/tests/render_basic.rs	true
darkmatter/cli/tests/rm.rs	true
darkmatter/cli/tests/schema_about.rs	true
darkmatter/cli/tests/schema_detect.rs	true
darkmatter/cli/tests/schema_triggers.rs	true
darkmatter/cli/tests/schema_validate.rs	true
darkmatter/cli/tests/schema_validate_baseline.rs	true
darkmatter/cli/tests/spawn_site_guard.rs	true
darkmatter/cli/tests/toc.rs	true
darkmatter/cli/tests/validate_refs.rs	true
darkmatter/lib/tests/ambient_ctx_capture.rs	true
darkmatter/lib/tests/array_rendering_json.rs	true
darkmatter/lib/tests/as_block_error_registry.rs	true
darkmatter/lib/tests/backslash_escape_spans.rs	true
darkmatter/lib/tests/base_schema_end_to_end.rs	true
darkmatter/lib/tests/benchmark_fixtures.rs	true
darkmatter/lib/tests/blockquote_list_spacing.rs	true
darkmatter/lib/tests/browser_render.rs	true
darkmatter/lib/tests/clean_counters.rs	true
darkmatter/lib/tests/compose_phase6.rs	true
darkmatter/lib/tests/compose_reuse_phase5.rs	true
darkmatter/lib/tests/cutover_reference.rs	true
darkmatter/lib/tests/debug_test.rs	true
darkmatter/lib/tests/declined_path_transclusion.rs	true
darkmatter/lib/tests/disclosure_render_targets.rs	true
darkmatter/lib/tests/disclosure_transclusion_integration.rs	true
darkmatter/lib/tests/effects_integration.rs	true
darkmatter/lib/tests/error_snapshots/condition.rs	true
darkmatter/lib/tests/error_snapshots/ctx_merge.rs	true
darkmatter/lib/tests/error_snapshots/deferred_set.rs	true
darkmatter/lib/tests/error_snapshots/editor.rs	true
darkmatter/lib/tests/error_snapshots/file_tree.rs	true
darkmatter/lib/tests/error_snapshots/helpers.rs	true
darkmatter/lib/tests/error_snapshots/image_ref.rs	true
darkmatter/lib/tests/error_snapshots/link.rs	true
darkmatter/lib/tests/error_snapshots/main.rs	true
darkmatter/lib/tests/error_snapshots/markdown_error.rs	true
darkmatter/lib/tests/error_snapshots/mermaid_theme.rs	true
darkmatter/lib/tests/error_snapshots/normalization.rs	true
darkmatter/lib/tests/error_snapshots/page_block.rs	true
darkmatter/lib/tests/error_snapshots/reference.rs	true
darkmatter/lib/tests/error_snapshots/shell_expansion.rs	true
darkmatter/lib/tests/error_snapshots/stylesheet.rs	true
darkmatter/lib/tests/error_snapshots/toc_linking.rs	true
darkmatter/lib/tests/error_snapshots/transclusion.rs	true
darkmatter/lib/tests/expression_regression.rs	true
darkmatter/lib/tests/frontmatter_surface_projection.rs	true
darkmatter/lib/tests/git_context_integration.rs	true
darkmatter/lib/tests/horizontal_rule_integration.rs	true
darkmatter/lib/tests/horizontal_rule_snapshots.rs	true
darkmatter/lib/tests/html_inversion.rs	true
darkmatter/lib/tests/image_pixel_classification.rs	true
darkmatter/lib/tests/image_test_support/mod.rs	true
darkmatter/lib/tests/inline_document_text.rs	true
darkmatter/lib/tests/inline_envelope_prototype.rs	true
darkmatter/lib/tests/interpolation_literal_pipeline.rs	true
darkmatter/lib/tests/layout_matrix.rs	true
darkmatter/lib/tests/layout_matrix_support/mod.rs	true
darkmatter/lib/tests/layout_snapshots.rs	true
darkmatter/lib/tests/level2_render_tree_terminal.rs	true
darkmatter/lib/tests/level2_render_tree_terminal/basic_spans.rs	true
darkmatter/lib/tests/level2_render_tree_terminal/code_panel.rs	true
darkmatter/lib/tests/level2_render_tree_terminal/file_links.rs	true
darkmatter/lib/tests/level2_render_tree_terminal/images.rs	true
darkmatter/lib/tests/level2_render_tree_terminal/layout_policy.rs	true
darkmatter/lib/tests/level2_render_tree_terminal/public_entry_points.rs	true
darkmatter/lib/tests/level2_render_tree_terminal/support/mod.rs	true
darkmatter/lib/tests/level3_image_painting.rs	true
darkmatter/lib/tests/level3_popover.rs	true
darkmatter/lib/tests/link_interpolation_integration.rs	true
darkmatter/lib/tests/meta_schema_phase1.rs	true
darkmatter/lib/tests/meta_schema_phase3.rs	true
darkmatter/lib/tests/meta_schema_phase4.rs	true
darkmatter/lib/tests/meta_schema_phase5.rs	true
darkmatter/lib/tests/meta_schema_phase6.rs	true
darkmatter/lib/tests/meta_schema_reference_graph.rs	true
darkmatter/lib/tests/meta_schema_repo_schemas.rs	true
darkmatter/lib/tests/more_is_more_literals_and_indexes.rs	true
darkmatter/lib/tests/predict_conflicts.rs	true
darkmatter/lib/tests/prelude_exports.rs	true
darkmatter/lib/tests/prose_wrap_parity.rs	true
darkmatter/lib/tests/reference_integration.rs	true
darkmatter/lib/tests/render_comparison.rs	true
darkmatter/lib/tests/render_invariants.rs	true
darkmatter/lib/tests/render_tree_hr_snapshots.rs	true
darkmatter/lib/tests/render_tree_roundtrip.rs	true
darkmatter/lib/tests/schema_phase_validation.rs	true
darkmatter/lib/tests/schema_quoting_safety.rs	true
darkmatter/lib/tests/schemas_convert_snapshots.rs	true
darkmatter/lib/tests/schemas_detect_table.rs	true
darkmatter/lib/tests/schemas_grammar_proptest.rs	true
darkmatter/lib/tests/schemas_literal_expression.rs	true
darkmatter/lib/tests/schemas_required_count_matrix.rs	true
darkmatter/lib/tests/schemas_source_projection.rs	true
darkmatter/lib/tests/schemas_validate_table.rs	true
darkmatter/lib/tests/set_overlay_integration.rs	true
darkmatter/lib/tests/shell_block_integration.rs	true
darkmatter/lib/tests/shell_expansion_coordinates.rs	true
darkmatter/lib/tests/span_compat.rs	true
darkmatter/lib/tests/style_features_baseline.rs	true
darkmatter/lib/tests/style_features_phase5.rs	true
darkmatter/lib/tests/style_frontmatter.rs	true
darkmatter/lib/tests/style_frontmatter_parity.rs	true
darkmatter/lib/tests/suggest_constraint_phase1.rs	true
darkmatter/lib/tests/suggest_constraint_phase2.rs	true
darkmatter/lib/tests/suggest_constraint_phase3.rs	true
darkmatter/lib/tests/suggest_constraint_phase4.rs	true
darkmatter/lib/tests/ternary_integration.rs	true
darkmatter/lib/tests/tree_features_characterization.rs	true
darkmatter/lib/tests/yaml_block_parity.rs	true
```

