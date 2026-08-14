---
kind: failing-catalog
created: 2026-08-14
updated: 2026-08-14
run: 31753281913
commit: a00ea7c08
branch: fix/ctx-launch-anchor
status: complete
producer_jobs: 25
failing_occurrences: 61
distinct_test_identities: 55
zero_identity_diagnostics: 11
description: Complete failed-producer catalog for CI run 31753281913
---

# Failing producers — run 31753281913 (`a00ea7c08`)

The completed run contains 603 jobs: 459 successful, 118 skipped, 25 failed
producers, and one expected downstream failed `ci-verdict`. The producer
catalog contains 23 JUnit-backed cells with 61 failing cell-occurrences (55
distinct test identities) and two lint cells with 11 normalized diagnostics.

The reproducible inventory, artifact IDs and digests, current `main` comparison,
and independent recount are in
[`../2026-08-14-ci-failure-triage/evidence.md`](../2026-08-14-ci-failure-triage/evidence.md).
The comparison control is completed `main` run `31588186544` at `e03bafee9`.

## Disposition ledger

`Equal` and `subset` describe exact current branch-versus-main evidence, not a
claim based on package ownership. Phase 2 still owns matched-host attribution
for the Windows and Level-2 rows. The mixed Claudine CLI WSL2 row is unresolved
and blocks baseline acceptance pending Phase 4.

| Cell | Job | Failed/total | Baseline | Main relation | Phase disposition |
| --- | ---: | ---: | --- | --- | --- |
| `biscuit-speaks-cli/wsl2-ubuntu/L1` | `94638865003` | 1/99 | no | subset (1/2) | main-side handoff |
| `biscuit-terminal-cli/macos-latest/L2` | `94635748562` | 1/1 | yes | equal | Phase 2 attribution |
| `biscuit-terminal-cli/ubuntu-latest/L2` | `94635748552` | 1/14 | yes | equal | Phase 2 attribution |
| `biscuit-tui-cli/windows-latest/L1` | `94624612538` | 1/391 | yes | equal | main-side handoff |
| `claudine/wsl2-ubuntu/L1` | `94643818956` | 3/4043 | no | equal | Phase 4 fixture handoff |
| `claudine-cli/macos-latest/L2` | `94630530915` | 3/28 | yes | equal | Phase 2 attribution |
| `claudine-cli/ubuntu-latest/L2` | `94630530916` | 4/29 | yes | equal | Phase 2 attribution |
| `claudine-cli/wsl2-ubuntu/L1` | `94639713054` | 21/2389 | yes | mixed: 17 shared, 4 added, 9 removed | unresolved Phase 4 block |
| `darkmatter/wsl2-ubuntu/L1` | `94640870498` | 7/6258 | yes | equal | Phase 4 fixture handoff |
| `darkmatter-cli/macos-latest/L2` | `94634206790` | 1/3 | yes | equal | Phase 2 attribution |
| `darkmatter-cli/ubuntu-latest/L2` | `94634206852` | 1/68 | yes | equal | Phase 2 attribution |
| `darkmatter-cli/windows-latest/L1` | `94624611717` | 1/655 | yes | equal | Phase 2 byte attribution |
| `dmls/ubuntu-latest/L2` | `94627543991` | 1/1 | no | equal | main-side handoff |
| `messenger/wsl2-ubuntu/L1` | `94637189813` | 1/457 | no | equal | main-side handoff |
| `model_id/wsl2-ubuntu/L1` | `94638098240` | 1/2 | yes | equal | main-side handoff |
| `rendezvous-daemon/windows-latest/L1` | `94624612212` | 2/155 | no | equal | main-side handoff |
| `sniff/macos-latest/L1` | `94624610764` | 2/1817 | no | equal | main-side handoff |
| `sniff/ubuntu-latest/L1` | `94624610810` | 1/1796 | no | equal | main-side handoff |
| `sniff/ubuntu-latest/lint` | `94624610721` | 0/0 | no | equal diagnostics | main-side handoff |
| `sniff/windows-latest/L1` | `94624610798` | 1/1791 | no | equal | main-side handoff |
| `sniff/wsl2-ubuntu/L1` | `94637944679` | 1/1796 | no | equal | main-side handoff |
| `sniff-cli/ubuntu-latest/lint` | `94624610329` | 0/0 | no | equal diagnostics | main-side handoff |
| `sniff-cli/windows-latest/L1` | `94624610505` | 3/785 | yes | equal | main-side handoff |
| `sniff-cli/wsl2-ubuntu/L1` | `94637640639` | 1/789 | yes | equal | main-side handoff |
| `unchained-ai/windows-latest/L1` | `94624613091` | 2/227 | no | equal | main-side handoff |

## Exact JUnit identity ledger

### `biscuit-speaks-cli/wsl2-ubuntu/L1`

- `biscuit-speaks-cli::cli_test::test_cli_loud_flag`

### `biscuit-terminal-cli/macos-latest/L2`

- `biscuit-terminal-cli::level2_apple_terminal_prose::level2_apple_terminal_double_underline_plain_text_visible`

### `biscuit-terminal-cli/ubuntu-latest/L2`

- `biscuit-terminal-cli::level2_diagrams::level2_diagram_fallback_when_no_image_protocol`

### `biscuit-tui-cli/windows-latest/L1`

- `biscuit-tui-cli::bin/question::completions::tests::bash_script_passes_syntax_check`

### `claudine/wsl2-ubuntu/L1`

- `claudine::composition::error::tests::shell_expansion_failed_via_real_markdown_preserves_rich_diagnostic`
- `claudine::composition::prepare::tests::direct_composition_runs_shell_in_configured_working_directory`
- `claudine::system_prompt::prepare::tests::non_repository_session_runs_shell_in_launch_cwd`

### `claudine-cli/macos-latest/L2`

- `claudine-cli::level2_context_capture::level2_context_default_at_140_fills_cap_in_tmux`
- `claudine-cli::level2_context_capture::level2_context_default_caps_at_140_in_wide_tmux`
- `claudine-cli::level2_context_capture::level2_context_default_narrow_preserves_type_and_wraps_in_tmux`

### `claudine-cli/ubuntu-latest/L2`

- `claudine-cli::level2_context_capture::level2_context_default_at_140_fills_cap_in_tmux`
- `claudine-cli::level2_context_capture::level2_context_default_caps_at_140_in_wide_tmux`
- `claudine-cli::level2_context_capture::level2_context_default_narrow_preserves_type_and_wraps_in_tmux`
- `claudine-cli::level2_context_capture::level2_context_default_preserves_columns_at_min_width_in_tmux`

### `claudine-cli/wsl2-ubuntu/L1`

- `claudine-cli::command_routing::agents_and_commands_route_to_empty_state_messages`
- `claudine-cli::context_command::context_reports_preserve_all_columns_at_minimum_supported_width`
- `claudine-cli::contextual_errors::compose_shell_execution_failure_renders_rich_block`
- `claudine-cli::ctx_launch_anchor_baseline::cli_loop_reuses_launch_context_for_root_and_package_prompt_copies`
- `claudine-cli::ctx_launch_anchor_baseline::cli_uses_launch_context_across_launch_source_matrix`
- `claudine-cli::ctx_launch_anchor_baseline::inline_cli_uses_launch_context_across_launch_source_matrix`
- `claudine-cli::handle_blocking_output::handle_flushes_blocking_payload_before_nonzero_exit`
- `claudine-cli::inline_compose_hash::inline_compose_writes_hash_that_passes_md_diff`
- `claudine-cli::loop_cli::compose_loop_rate_limit_pause_waits_then_continues`
- `claudine-cli::sequence_groups::a_parallel_group_overlaps_its_members`
- `claudine-cli::sequence_groups::max_parallel_bounds_the_overlap`
- `claudine-cli::sequence_overlay_pty::pty_sequence_prompt_dedupes_and_launches_all_steps`
- `claudine-cli::sequence_overlay_pty::pty_sequence_step_overlay_satisfies_required_property`
- `claudine-cli::sequence_schema::sequence_per_step_step_timeout_override`
- `claudine-cli::shipped_prompt_contract::shipped_context_prompt_renders_its_package_area_list_through_the_cli`
- `claudine-cli::shipped_prompts::shipped_implement_prompt_runs_real_router_target`
- `claudine-cli::wrap_compose_validation::compose_dry_run_quiet_and_silent_are_no_op`
- `claudine-cli::wrap_compose_validation::compose_initialize_error_with_failure_raise_surfaces_failure_evaluation_error`
- `claudine-cli::wrap_compose_validation::compose_initialize_when_evaluation_error_exits_non_zero`
- `claudine-cli::wrap_inline_compose::inline_compose_dry_run_quiet_and_silent_are_no_op`
- `claudine-cli::wrap_opencode::opencode_stderr_stream_error_cap_1_17_8_forces_early_termination`

### `darkmatter/wsl2-ubuntu/L1`

- `darkmatter::ambient_ctx_capture::every_catalog_variable_survives_ambient_options`
- `darkmatter::interpolation_literal_pipeline::frontmatter_literal_survives_shell_bracketed_interpolation_passes`
- `darkmatter::markdown::compose::frontmatter_shell_expansion::tests::detects_no_cache_suffix`
- `darkmatter::markdown::compose::frontmatter_shell_expansion::tests::no_cache_combines_with_timeout_either_order`
- `darkmatter::markdown::compose::frontmatter_shell_expansion::tests::no_cache_defaults_false_without_suffix`
- `darkmatter::shell_expansion_coordinates::shell_block_execution_failed_renders_inner_diagnostic`
- `darkmatter::shell_expansion_coordinates::shell_block_origin_counts_lines_not_bytes_with_crlf`

The raw XML confirms all seven are also red on the selected `main` control.
`frontmatter_literal_survives_shell_bracketed_interpolation_passes` predates
this branch. The three `no_cache` tests consult `PATH` while classifying the
bare `rustc` token; the interpolation and coordinate tests execute `rustc`.
The seventh identity, `every_catalog_variable_survives_ambient_options`, was
omitted from the provisional catalog and must be retained for Phase 4.

### `darkmatter-cli/macos-latest/L2`

- `darkmatter-cli::level2_code_block_styling::level2_code_block_clears_inherited_dim_before_theme_colors`

### `darkmatter-cli/ubuntu-latest/L2`

- `darkmatter-cli::level2_schema_about::level2_schema_about_light_terminal_uses_dark_code_theme`

### `darkmatter-cli/windows-latest/L1`

- `darkmatter-cli::schema_validate_baseline::schema_validate_legacy_pretty_output_is_byte_identical`

### `dmls/ubuntu-latest/L2`

- `dmls::level2_editor_neovim::level2_neovim_decodes_semantic_token_families_and_positions`

### `messenger/wsl2-ubuntu/L1`

- `messenger::provider::desktop::linux::tests::native_fallback_delivers_when_no_helpers_installed`

### `model_id/wsl2-ubuntu/L1`

- `model_id::ui::ui`

### `rendezvous-daemon/windows-latest/L1`

- `rendezvous-daemon::local_transport::windows::tests::the_pipe_dacl_names_this_user_and_nobody_else`
- `rendezvous-daemon::private_dir::tests::the_current_user_descriptor_names_this_account_and_nobody_else`

### `sniff/macos-latest/L1`

- `sniff::filesystem::repo::standard::tests::resolve_acting_binary_returns_none_when_binary_missing`
- `sniff::hardware::gpu::tests::test_detect_gpus_on_macos`

### `sniff/ubuntu-latest/L1`

- `sniff::filesystem::repo::standard::tests::resolve_acting_binary_returns_none_when_binary_missing`

### `sniff/windows-latest/L1`

- `sniff::filesystem::repo::standard::tests::resolve_acting_binary_returns_none_when_binary_missing`

### `sniff/wsl2-ubuntu/L1`

- `sniff::integration::test_detect_completes_in_reasonable_time`

### `sniff-cli/windows-latest/L1`

- `sniff-cli::cli::test_repo_worktree_verbose_includes_path`
- `sniff-cli::cli::test_repo_worktrees_verbose_output`
- `sniff-cli::snapshots::repo_aggregate_json_snapshot`

### `sniff-cli/wsl2-ubuntu/L1`

- `sniff-cli::snapshots::repo_aggregate_json_snapshot`

### `unchained-ai/windows-latest/L1`

- `unchained-ai::primitives::services::pty_runner::tests::test_ansi_stripping`
- `unchained-ai::primitives::services::pty_runner::tests::test_run_echo_command`

## Exact zero-identity diagnostic ledger

### `sniff/ubuntu-latest/lint`

- `unused_imports` (`use super::*`) — `sniff/lib/src/services/launchd.rs:40:9`
- `unused_variables` (`helpers`) — `sniff/lib/src/programs/notification_helpers.rs:169:13`
- `permissions_set_readonly_false` — `sniff/lib/tests/merge_conflict_prediction.rs:480:5`
- `items_after_test_module` — `sniff/lib/src/hardware/storage.rs:218:1`
- `items_after_test_module` — `sniff/lib/src/programs/enums/metadata.rs:3324:1`
- `zombie_processes` — `sniff/lib/src/process.rs:845:9`
- `zombie_processes` — `sniff/lib/src/process.rs:863:26`
- `zombie_processes` — `sniff/lib/src/process.rs:907:9`
- `zombie_processes` — `sniff/lib/src/process.rs:937:26`
- `zombie_processes` — `sniff/lib/src/process.rs:989:26`

### `sniff-cli/ubuntu-latest/lint`

- `clippy::collapsible_if` — `sniff/cli/tests/snapshots.rs:672:17`

## Corrections to the provisional catalog

- The run completed with 603 jobs and 25 failed producers, not 601 jobs and 24
  failures. `ci-verdict` is a separate downstream failure.
- The final JUnit inventory is 61 failing cell-occurrences and 55 distinct test
  identities, not 51.
- `claudine-cli/wsl2-ubuntu/L1` is present with 21 identities. It was the one
  still-running producer omitted from the provisional capture.
- `darkmatter/wsl2-ubuntu/L1` has seven identities, not six. All seven are red
  on the selected `main` control; none is classified as branch-owned in Phase
  1.
- `biscuit-speaks-cli/wsl2-ubuntu/L1` has the recoverable identity
  `test_cli_loud_flag`; it is not a zero-identity failure.
- The two lint cells remain the only zero-identity producers and are compared
  by normalized diagnostics, not empty test sets.
