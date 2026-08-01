# Phase 0 native Windows baseline

Captured on 2026-08-01 on native Windows (`x86_64-pc-windows-msvc`) before
production-code changes. The checkout was already dirty (`CLAUDE.md` modified
and `claudine/fixes/plan.md` untracked); neither was changed by this capture.

## Commands and outcomes

```powershell
# repository root
cargo build --tests -j 4 `
  -p claudine-catalog-types -p claudine -p claudine-contract `
  -p claudine-cli -p claudine-gen

# claudine/
just test --profile ci --no-fail-fast
```

| Command | Exit | Wall time | Outcome |
|---|---:|---:|---|
| five-package constrained build | 0 | 8m 34.503s | passed; Cargo reported 8m 33s |
| full L1 CI-profile baseline | 1 | 4m 12.059s | three packages passed; `claudine` and `claudine-cli` failed |

The build emitted pre-existing warnings, notably the Windows-only unused
`tracing::warn` import in `config/atomic.rs` and unreachable/unused code in
`linking/symlink.rs`. There were no compiler or linker errors.

The five-package run executed 6,136 tests: 5,988 passed and 148 failed. The
runner additionally reported 13 skipped tests and no timeouts. Package results:

| Package | Passed | Failed | Skipped | Timeout | Package wall time |
|---|---:|---:|---:|---:|---:|
| `claudine-catalog-types` | 21 | 0 | 0 | 0 | 22s recipe cell; 0.092s nextest rerun |
| `claudine` | 3,884 | 49 | 0 | 0 | 77s recipe cell; 62.649s XML rerun |
| `claudine-contract` | 47 | 0 | 4 | 0 | 7s recipe cell; 0.280s nextest rerun |
| `claudine-cli` | 1,884 | 99 | 8 | 0 | 72s recipe cell; 47.253s XML rerun |
| `claudine-gen` | 152 | 0 | 1 | 0 | 69s recipe cell |

The area recipe forwarded `--profile ci` but did not export
`NEXTEST_PROFILE`; `_stage_junit` therefore searched the wrong profile path and
recorded `report_present:false`. XML-backed reruns of only the two failed
package cells reproduced the same 49 and 99 failures. The passing-package
counts were confirmed by XML/list capture. The generator baseline completed
without a timeout, but that staging defect prevented recovery of its individual
test durations after the run; its three expensive drift tests therefore have
only the exact 69-second package-cell bound in this note.

## Per-test-binary totals

`claudine` ran 3,830/49/0 in its library binary (pass/fail/skip). Its integration
binaries were all green: `boundary_lint` 7, `canonical_dispatch` 7,
`deprecated_compatibility` 3, `agent_errors_fleet` 2,
`diagnostic_detail_conformance` 4, `kimi_wire` 6,
`model_catalog_integration` 2, `opencode_stderr_lifecycle` 6,
`protocol_fixture_replay` 9, `semantic_fidelity` 35, and
`typed_stream_protocols` 22.

`claudine-cli` binary totals (pass/fail/skip; omitted skip values are zero):

```text
argv_normalization 14/0           characterization_error_routes 7/0
command_routing 8/1              completion_cli 13/1
completion_compose 15/17          completion_contract 8/2
completion_perf 5/0/4             completion_inline_compose 6/2
completion_sequence 5/3           completion_setter 2/15
compose_schema_cli 12/3           composition_seams 17/0
completion_resolution_round_trip 2/0
context_command 27/0              diagnostic_discovery 2/0
dispatch_inventory 12/0           effective_diagnostic_render 5/0
error_guards 18/0                 contextual_errors 2/0
errors_command 5/0                handle_deadline 2/0
hooks_cli 7/1                     handle_repo_config 2/0
inline_compose_sequence_mismatch 13/0
mcp_cli 3/9                       run_harness_loop_call_sites 2/0
sequence_cli 8/1                  sequence_errors_cli 29/0
protect_cli 1/0                   sequence_sources_cli 21/1
provider_error_finalize 1/0       shipped_prompt_contract 1/0
shipped_prompt_route_drift 3/0    shipped_prompts 1/0
skills_integration 13/9           test_placement 9/0
sequence_ctrl_c_windows 1/0       wrap_basics 5/0
wrap_compose_validation 10/0      wrap_inline_compose 2/0
bin/claudine 1565/32              wrap_ctrl_c_windows 0/1
inline_compose_hash 0/1           system_prompt_perf_bench 0/0/3
level3_windows_sequence_ctrl_c 0/0/1 (excluded by the L1 filter)
```

`claudine-gen` binary counts were: library 92, `agent_errors_check` 10,
`drift` 6, `fixtures_provenance` 1, `generate_ux` 10, `pipeline` 17,
`registry_coverage` 3, `signals_sidecar_mirror` 1,
`signals_validation` 8 plus one L1-filtered test, and `vocabulary` 4. All
executed tests passed. `claudine-catalog-types` had one 21-test binary;
`claudine-contract` had one 47-test binary (the second discovered target had no
executed tests).

## Tests over five seconds

```text
PASS claudine::composition::preflight::tests::full_flow_blacklisted_command_aborts_preflight 5.483s
PASS claudine::composition::preflight::tests::full_flow_unapproved_command_rejected_at_compose_time 5.508s
PASS claudine::composition::preflight::tests::deny_returns_shell_command_denied_error 6.015s
PASS claudine::composition::preflight::tests::approval_request_carries_real_source_provenance 6.289s
PASS claudine::composition::preflight::tests::dry_run_no_handler_emits_cannot_dry_run_message 6.425s
PASS claudine::composition::preflight::tests::blacklisted_command_returns_error 6.780s
PASS claudine::composition::preflight::tests::discovers_commands_from_template 6.770s
PASS claudine::composition::preflight::tests::lifecycle_key_with_missing_file_ref_is_deferred_in_preflight 8.382s
PASS claudine::composition::preflight::tests::full_flow_template_with_whitelisted_commands 11.097s
PASS claudine::composition::preflight::tests::full_flow_shell_pending_dir_with_context_requiring_sibling_key 12.429s
PASS claudine-cli::context_command::context_no_markdown_parsing_artifacts 5.157s
PASS claudine-cli::context_command::context_footer_no_availability_claims 5.529s
PASS claudine-cli::context_command::context_values_exits_zero_and_produces_stdout 5.211s
PASS claudine-cli::context_command::context_values_writes_footer_to_stderr 5.140s
PASS claudine-cli::context_command::context_values_includes_every_descriptor 6.331s
PASS claudine-cli::context_command::context_values_renders_canonical_keys_non_null 6.083s
PASS claudine-cli::context_command::context_reports_preserve_all_columns_at_minimum_supported_width 13.681s
PASS claudine-cli::contextual_errors::compose_transclusion_cycle_renders_file_chain 6.096s
PASS claudine-cli::contextual_errors::compose_shell_execution_failure_renders_rich_block 6.299s
FAIL claudine-cli::wrap_ctrl_c_windows::ctrl_c_terminates_wrapped_child_on_windows 16.604s
FAIL claudine-cli::inline_compose_hash::inline_compose_writes_hash_that_passes_md_diff 31.037s
```

## Targeted security suites

Both commands used the CI profile, no retries, and no fail-fast behavior:

```powershell
cargo nextest run -p claudine -E 'test(/permissions::/)' --profile ci --no-fail-fast
cargo nextest run -p claudine -E 'test(/protect::/)' --profile ci --no-fail-fast
```

| Suite | Exit | Wall / nextest | Passed | Failed | Filter-skipped |
|---|---:|---:|---:|---:|---:|
| permissions | 100 | 3.880s / 0.513s | 83 | 9 | 3,841 |
| protect | 100 | 4.279s / 0.908s | 106 | 13 | 3,814 |

Permissions failures: `effective_snapshot_queries_work`,
`query_result_has_explanation`, `snapshot_path_read_query`,
`snapshot_path_write_deny`, `claude_backend_queries_paths_and_commands`,
`claude_local_override_round_trip_changes_query_result`,
`codex_backend_models_workspace_write`,
`codex_full_auto_cli_override_is_effective`, and
`path_queries_are_normalized_relative_to_cwd`.

Protect failures: `absolute_allow_respects_boundary`,
`absolute_sensitive_paths_are_detected`,
`exact_home_sensitive_directory_roots_are_detected`,
`exact_sensitive_directory_roots_are_detected`,
`home_credential_paths_are_sensitive`,
`home_relative_sensitive_paths_are_detected`,
`macos_system_path_is_sensitive`,
`tilde_exact_sensitive_directory_roots_are_detected`,
`relative_path_traversal_to_etc_is_blocked`,
`relative_path_traversal_to_ssh_is_blocked`,
`write_paths_array_blocks_when_any_path_is_sensitive`,
`write_to_non_allowed_sensitive_path_is_still_blocked`, and
`write_to_ssh_config_is_blocked`.

## Atomic-write evidence

`claudine::config::atomic::tests::concurrent_writers_produce_intact_payload`
failed at the first product location `claudine/lib/src/config/atomic.rs:144:49`.
Two writer panics exposed raw Windows OS codes:

```text
Os { code: 5, kind: PermissionDenied, message: "Access is denied." }
Os { code: 2, kind: NotFound, message: "The system cannot find the file specified." }
```

The coordinator then panicked while joining at `config/atomic.rs:148:22`.

## Failure inventory and first emitted product locations

The runner did not enable `RUST_BACKTRACE`, so “frame” below means the first
Claudine source location emitted by the panic. When an `assert_cmd` assertion
emitted only Rust's `core::ops::function.rs:250:5`, that absence is recorded
instead of inventing a Claudine frame.

### Library (49)

```text
composition::lifecycle::control::tests::resolve_proxy_target_package_reference_prefers_authoring_package_area -> composition/lifecycle/control/tests.rs:386:5
composition::lifecycle::executor::tests::filesystem_lookup::lifecycle_reuses_prepared_snapshot_for_prompt_outside_launch_area -> composition/lifecycle/executor/tests/filesystem_lookup.rs:256:5
composition::lifecycle::executor::tests::filesystem_lookup::ctx_capture_follows_ctx_base_dir_not_base_dir -> composition/lifecycle/executor/tests/filesystem_lookup.rs:164:5
composition::prepare::tests::direct_composition_runs_shell_in_configured_working_directory -> composition/prepare/tests.rs:777:5
composition::sequence::preflight::tests::loading::nested_references_reuse_all_request_resolution_inputs -> composition/sequence/preflight/tests.rs:154:10
composition::sequence::task::tests::shell_tasks::the_system_shell_kills_a_backgrounded_descendant_holding_stdout -> composition/sequence/task/tests.rs:1176:9
composition::sequence::task::tests::shell_tasks::the_system_shell_times_out_a_nested_tree -> composition/sequence/task/tests.rs:1246:9
composition::sequence::tests::tilde_reference_expands_against_home_directory -> composition/sequence/tests.rs:640:77
config::atomic::tests::concurrent_writers_produce_intact_payload -> config/atomic.rs:144:49
config::claude::tests::already_in_sync_skips_registration -> config/claude/tests.rs:377:5
config::claude::tests::re_register_adds_new_events -> config/claude/tests.rs:302:5
config::claude::tests::re_register_removes_stale_events -> config/claude/tests.rs:337:5
config::codex::tests::sync_detects_missing_wrapper_file_as_out_of_sync -> config/codex.rs:542:67
harness::resolve::tests::implicit_no_match_projects_ordered_candidate_detail -> harness/resolve/tests.rs:261:5
messaging::resolve::tests::absolute_path_unchanged -> messaging/resolve.rs:379:9
model_catalog::provider_sources::tests::fetch_shell_command_models_spawns_and_parses -> model_catalog/provider_sources.rs:290:9
permissions::engine::tests::effective_snapshot_queries_work -> permissions/engine/tests.rs:376:5
permissions::engine::tests::query_result_has_explanation -> permissions/engine/tests.rs:393:5
permissions::engine::tests::snapshot_path_read_query -> permissions/engine/tests.rs:217:5
permissions::engine::tests::snapshot_path_write_deny -> permissions/engine/tests.rs:231:5
permissions::providers::claude::tests::claude_backend_queries_paths_and_commands -> permissions/providers/claude/tests.rs:66:5
permissions::providers::claude::tests::claude_local_override_round_trip_changes_query_result -> permissions/providers/claude/tests.rs:182:5
permissions::providers::codex::tests::codex_backend_models_workspace_write -> permissions/providers/codex/tests.rs:48:5
permissions::providers::codex::tests::codex_full_auto_cli_override_is_effective -> permissions/providers/codex/tests.rs:118:5
permissions::query::tests::path_queries_are_normalized_relative_to_cwd -> permissions/query/tests.rs:46:5
protect::path::tests::absolute_allow_respects_boundary -> protect/path.rs:388:9
protect::path::tests::absolute_sensitive_paths_are_detected -> protect/path.rs:292:9
protect::path::tests::exact_home_sensitive_directory_roots_are_detected -> protect/path.rs:425:9
protect::path::tests::exact_sensitive_directory_roots_are_detected -> protect/path.rs:404:9
protect::path::tests::home_credential_paths_are_sensitive -> protect/path.rs:478:9
protect::path::tests::home_relative_sensitive_paths_are_detected -> protect/path.rs:313:9
protect::path::tests::macos_system_path_is_sensitive -> protect/path.rs:304:9
protect::path::tests::tilde_exact_sensitive_directory_roots_are_detected -> protect/path.rs:438:9
protect::service::tests::relative_path_traversal_to_etc_is_blocked -> protect/service/tests.rs:219:5
protect::service::tests::relative_path_traversal_to_ssh_is_blocked -> protect/service/tests.rs:206:5
protect::service::tests::write_paths_array_blocks_when_any_path_is_sensitive -> protect/service/tests.rs:75:5
protect::service::tests::write_to_non_allowed_sensitive_path_is_still_blocked -> protect/service/tests.rs:305:5
protect::service::tests::write_to_ssh_config_is_blocked -> protect/service/tests.rs:61:5
render::prompt::system::tests::display_label_nerd_font_in_base_uses_glyph_with_path -> render/prompt/system/tests.rs:180:5
render::prompt::system::tests::display_label_no_nerd_font_in_base_uses_relative -> render/prompt/system/tests.rs:194:5
render::prompt::system::tests::summary_emits_osc8_for_file_link -> render/prompt/system/tests.rs:269:5
stream::path_link::tests::cwd_preferred_over_home_when_both_could_match -> stream/path_link.rs:197:9
stream::path_link::tests::home_preferred_when_cwd_is_not_prefix -> stream/path_link.rs:185:9
stream::path_link::tests::long_path_truncates_visible_text_but_keeps_full_href -> stream/path_link.rs:206:9
stream::path_link::tests::path_inside_cwd_renders_relative_osc8_link -> stream/path_link.rs:121:9
stream::path_link::tests::path_inside_home_renders_tilde_prefix -> stream/path_link.rs:133:9
stream::path_link::tests::prose_metacharacters_in_path_are_escaped -> stream/path_link.rs:162:9
system_prompt::resolve::tests::explicit_append_tilde_resolves_against_home -> system_prompt/resolve/tests.rs:159:33
system_prompt::resolve::tests::non_interactive_candidates_prefer_repo_then_home_then_builtin -> system_prompt/resolve/tests.rs:512:5
```

### CLI (99)

The 99 exact names are grouped by their first emitted product file. Each line's
parenthesized number is the emitted line; names without a number emitted only
the `assert_cmd` framework frame described above.

```text
tests/command_routing.rs: actions_command_routes_and_reports_configured_events (103)
tests/completion_cli.rs: empty_input_lists_curated_markdown_inside_repo (86)
tests/completion_compose.rs: compose_empty_partial_surfaces_plain_markdown (40), compose_empty_partial_renders_repo_claudine_scope (60), compose_word_partial_renders_repo_claudine_scope (80), compose_empty_partial_renders_user_global_scope (101), compose_word_partial_renders_user_global_scope (122), compose_package_prompt_renders_repo_relative_path (140), compose_package_area_prompt_renders_repo_relative_path (158), compose_empty_partial_skips_prompt_frontmatter_files (180), compose_short_prefix_matches_filenames_and_dirs_with_prefix (252), compose_long_prefix_includes_directories (275), compose_magic_surfaces_user_global_only_filename (409), compose_hides_underscore_prefixed_files (433), compose_honors_gitignore_at_nested_depth (451), compose_rejects_oversized_markdown (479), compose_rejects_non_utf8_markdown (502), compose_plain_git_non_magic_renders_repo_claudine_relative (575), compose_magic_short_prefix_surfaces_no_dirs (687)
tests/completion_contract.rs: complete_subcommand_drives_dynamic_completion (74), yaml_sequence_candidates_surface_in_sequence_completion (239)
tests/completion_inline_compose.rs: inline_compose_surfaces_files_with_prompt_key (37), inline_compose_long_prefix_surfaces_directories (120)
tests/completion_sequence.rs: sequence_surfaces_markdown_with_sequence_key (34), sequence_surfaces_yaml_files (58), sequence_long_prefix_includes_directories (127)
tests/completion_setter.rs: setter_at_sigil_triggers_file_completion (40), setter_double_quote_is_normalized_to_single_quote (94), setter_single_quote_round_trips_to_single_quote (107), setter_scope_covers_all_four_subdirs (127), setter_scope_anchors_on_cwd_excludes_parent_dirs (156), setter_scope_renders_package_docs_relative_to_cwd (176), setter_value_fires_on_inline_compose (193), setter_value_fires_on_sequence (206), setter_value_wins_after_committed_positional (228), setter_scope_excludes_underscore_prefixed (251), setter_excludes_non_markdown_files (275), setter_accepts_uppercase_md_and_markdown (304), setter_scope_honors_gitignore (330), setter_resolves_nested_feature_directory_path (362), setter_resolves_in_plain_git_checkout (389)
tests/compose_schema_cli.rs: completion_file_match_emits_matching_files_only (1527), completion_file_match_emits_path_qualified_glob_matches (1567), completion_file_match_honors_negated_path_qualified_glob (1610)
tests/hooks_cli.rs: hooks_invalid_sound_effect_escapes_prose_characters (155)
tests/mcp_cli.rs: effective_defaults_repo_replaces_user (585), mcp_list_outside_repo_returns_no_repo_defaults (415), mcp_export_reports_unresolved_defaults_and_uses_native_name (269), mcp_check_json_reports_invalid_servers (180); no product frame: mcp_config_json_uses_new_command_name, mcp_remove_alias_reports_owner_and_remaining, mcp_show_json_includes_provenance, mcp_remove_cascades_to_user_defaults, mcp_remove_cascades_to_repo_defaults
tests/sequence_cli.rs: no product frame: sequence_resolves_a_data_file_with_offset_and_operator
tests/sequence_sources_cli.rs: no product frame: file_references_resolve_from_the_authoring_document
tests/skills_integration.rs: skills_detail_view_shows_filesystem (300), skills_footer_shows_filter_hint (358), skills_not_in_git_repo_shows_user_only_hint (407), skills_negation_with_dash_prefix_excludes_match (435), skills_negation_with_bang_prefix_excludes_match (469), skills_combined_positive_and_negation (537); no product frame: skills_fix_shows_summary, skills_lists_user_scoped_skill, skills_verbose_shows_descriptions
src/commands/wrap/composition/tests.rs: load_selection_config_returns_both_favorite_and_overrides (192)
src/commands/wrap/exec/spawn/tests/captured.rs: run_child_capture_captures_agent_pid_after_successful_spawn (93), run_child_capture_propagates_claudine_pid_to_child_environment (170), consecutive_spawns_produce_distinct_agent_pids (211)
src/commands/wrap/exec/spawn/tests/inherited.rs: run_child_captures_agent_pid_after_successful_spawn (29)
src/commands/wrap/composition/prep_context.rs: prep_context_loads_cwd_config_when_source_repo_root_is_none (453)
src/commands/wrap/exec/termination/tests/wait.rs: non_unix_wait_loop_returns_on_child_exit (175), windows_completion_termination_uses_job_object_path (214)
src/commands/wrap/harness_orch/loop_control/tests/overlay_layering.rs: a_file_valued_overlay_property_resolves_through_the_targets_own_context (714)
src/commands/wrap/live_semantic_sink/tests/provider_extension_and_opencode.rs: read_tool_call_renders_path_as_cwd_relative_blue_link (740)
src/commands/wrap/repo_home/tests.rs: codex_sqlite_home_defaults_to_pre_shadow_codex_home (89)
src/completion/composition/tests.rs: compose_long_prefix_surfaces_directories (155), compose_word_matches_while_typing_extension (240)
src/completion/operation_file.rs: format_relative_insert_prefers_repo_root (475)
src/completion/schema_completion/tests.rs: property_value_offers_files_for_root_union_arm (138), property_value_match_pattern_excludes_underscore_dirs_and_files (335), property_value_match_pattern_filters_by_path_substring (373), property_value_match_pattern_anchors_on_cwd_not_repo_root (412)
src/completion/setter_value/tests.rs: run_surfaces_repo_docs_under_at_sigil (161), run_wraps_value_in_single_quotes_even_when_user_typed_double (185), run_wraps_value_in_single_quotes_when_user_typed_single (200), run_empty_at_surfaces_every_file_in_scope (217), run_cwd_at_repo_root_only_repo_scope (230), run_cwd_inside_package_area_renders_relative_to_cwd (244), run_cwd_inside_package_renders_relative_to_cwd (261), run_only_surfaces_files_under_cwd_not_repo_root (283), setter_value_skips_txt_files (330), setter_value_accepts_uppercase_md (376), setter_value_accepts_uppercase_markdown (390), run_excludes_underscore_prefixed_files (424), run_orders_subdirs_then_path_within_cwd (441)
src/telemetry.rs: shorten_source_path_strips_repo_root_prefix (591)
tests/wrap_ctrl_c_windows.rs: ctrl_c_terminates_wrapped_child_on_windows (171)
no product frame: inline_compose_writes_hash_that_passes_md_diff
```

## Baseline classification

- Security matching is confirmed, not inferred: 9 permission-policy failures
  and 13 protect failures reproduce, including fail-open deny and sensitive-path
  behavior.
- Atomic replacement is a separate Windows product defect with measured OS
  codes 5 and 2.
- Portable rendering/file-URI work is directly represented by library prompt
  and path-link failures plus the large completion cluster that returns `\`
  where the public contract expects `/`.
- Residual Windows failures include shell/batch invocation, home/config
  discovery, Ctrl+C/job-object handling, MCP/skills filesystem projections, and
  two system-prompt candidate tests. These remain unclassified pending the
  later plan phase; none is hidden behind a Unix-only assumption here.
- The generator package passed the original CI-profile baseline without a
  timeout. Individual generator timing remains the one evidence gap caused by
  the profile/staging mismatch described above.
