# Test Map — Provider Overlay Contracts

Maps each numbered L1 contract in [`spec.md`](./spec.md) → Testing → L1 contract
tests to the tests that prove it. Paths are relative to `claudine/`.

The L1 tests drive the real `claudine` binary against fake providers.
`level1_provider_overlay_home.rs` is `#![cfg(unix)]` because its fakes are
shell stubs. Native Windows path forms are covered at unit level. Phase 11
records the L2 and cross-platform evidence below.

`L1` = `cli/tests/level1_provider_overlay_home.rs` unless another file is named.

## L1 contracts

| # | Contract | Tests | Level |
|---|---|---|---|
| 1 | Every overlay activation reason leaves the launch baseline's home variables unchanged | `L1::every_activation_reason_leaves_the_launch_home_variables_unchanged` (`repo_resources` × Claude, Codex, Gemini, Kimi, Pi, Qwen; Codex `repo_prompt`; `mcp` × Codex, Gemini, OpenCode inline; `compose --codex --repo`), `L1::codex_repo_overlay_uses_codex_home_and_leaves_the_user_home`, `L1::a_launch_without_overlay_reasons_sets_no_selector` | L1 |
| 2 | No launch path puts `/dev/null`, `NUL`, or an overlay path into a global home variable | `L1::every_activation_reason_leaves_the_launch_home_variables_unchanged` (`assert_no_home_sentinel` over the provider and every nested tool), `L1::a_failed_overlay_stops_the_launch_without_a_null_home`, `L1::an_unwritable_storage_root_stops_the_launch_with_the_typed_diagnostic`; writer and checker: `cli/src/commands/wrap/provider_overlay/tests.rs::{an_overlay_patch_never_writes_a_home_variable, home_identity_violation_flags_null_devices_and_overlay_paths}`, enforced on every debug spawn by `exec/spawn/setup.rs::debug_assert_child_env` | L1 + unit |
| 3 | Explicit provider-root overrides are source roots; the selector points at the right provider-visible overlay shape | `L1::an_explicit_provider_root_is_the_overlay_source_and_the_selector_names_the_overlay` (all six native-root providers, including Gemini's parent shape and Claude's credential-store pin); `L1::a_codex_to_opencode_transition_restores_explicit_ambient_codex_selectors`; unit: `lib/src/provider_overlay/tests.rs::{an_explicit_selector_value_is_the_source_and_never_the_destination, an_explicit_parent_shaped_value_resolves_through_its_child_segment}`, `cli/.../provider_overlay/tests.rs::an_explicit_source_root_is_read_from_not_written_to` | L1 + unit |
| 4 | Repo resources stay isolated at each provider's documented level | `L1::a_repo_overlay_hides_exactly_the_documented_resource_classes` (Claude, Codex, Gemini, Kimi, Qwen, against `docs/topics/repo-isolation.md`); unit: `lib/src/provider_overlay/tests.rs::the_repo_isolation_table_matches_the_documented_set`, `cli/.../provider_overlay/tests.rs::{repo_only_exclusions_come_from_the_shared_isolation_table, a_repo_overlay_omits_isolated_resources_and_keeps_settings}` | L1 + unit |
| 5 | Codex prompt and MCP overlays still work, and SQLite stays outside them | `cli/tests/wrap_basics.rs::codex_wrapper_uses_a_provider_overlay_for_repo_prompts_without_repo_flag` (renamed from `…uses_shadow_home…`), `L1::codex_mcp_injects_servers_into_the_config_codex_home_names`, `L1::codex_mcp_without_a_codex_root_injects_into_an_empty_overlay` (Phase 11 regression), `cli/tests/propagated_context_fixtures.rs::isolated_fixture_can_opt_in_to_provider_overlay_repo_resources`; unit: `cli/.../provider_overlay/tests.rs::{a_mutable_state_file_is_neither_linked_nor_copied, codex_overlay_uses_real_sqlite_directory_and_preserves_legacy_state}` | L1 + unit |
| 6 | Gemini MCP injection writes where `GEMINI_CLI_HOME` points, not a `HOME`-derived path | `L1::gemini_mcp_injects_servers_under_the_gemini_cli_home_root`, `L1::compose_gemini_mcp_injects_servers_under_the_gemini_cli_home_root`; unit: `lib/src/mcp/inject.rs::tests::gemini_writes_settings_json_directly_in_the_config_root` | L1 + unit |
| 7 | OpenCode inline MCP injection creates no overlay | `L1::opencode_mcp_injects_inline_without_an_overlay` (no `OPENCODE_CONFIG_DIR`, no storage on disk), the OpenCode row of `L1::every_activation_reason_…` | L1 |
| 8 | Unsupported provider/reason pairs fail before the fake provider records a spawn | `L1::every_refused_provider_and_reason_fails_before_the_provider_is_spawned` (audit refusal list rows 1–4: Antigravity, OpenCode, Kilo, Goose × `--repo`, direct and `compose`; `claude --mcp` keeps its export guidance), `L1::antigravity_refuses_only_the_repo_mode`; unit: `lib/src/provider_overlay/tests.rs::every_published_refusal_pair_refuses_before_a_plan_exists` | L1 + unit |
| 9 | Materialization failure gives the typed diagnostic and never falls back to a null home | `L1::a_failed_overlay_stops_the_launch_without_a_null_home` (file in the way), `L1::an_unwritable_storage_root_stops_the_launch_with_the_typed_diagnostic` (read-only storage root), `L1::a_missing_explicit_source_root_stops_the_launch_before_building_storage` (named `CLAUDE_CONFIG_DIR` that does not exist); unit: `cli/.../provider_overlay/tests.rs::{a_missing_explicit_source_root_fails_with_a_typed_error_before_building_storage, a_source_root_that_is_a_file_fails_with_a_typed_error}`, `L1::a_removed_api_key_and_an_overlay_failure_are_distinct_diagnostics` | L1 |
| 10 | Simulated nested `git`, `gpg`, and `gh` see the original home variables without touching real credential stores | `assert_home_preserved` in every `L1` launch through `recording_command`: stubs sit in a directory only the provider adds to its `PATH`, and each records `HOME`/`USERPROFILE`/`HOMEDRIVE`/`HOMEPATH` | L1 |
| 11 | Proxy/retry/resume transitions remove the prior selector, restore an explicit ambient value, and apply only the target's plan | `L1::{a_codex_to_opencode_transition_leaves_no_codex_selector_in_the_opencode_child, a_codex_to_opencode_transition_restores_explicit_ambient_codex_selectors, a_codex_to_gemini_transition_injects_into_the_gemini_overlay}` (each via proxy **and** retry); unit: `launch_plan::tests::{a_replay_onto_codex_applies_the_codex_overlay_plan, a_replay_away_from_codex_removes_every_codex_overlay_variable, a_same_provider_replay_keeps_the_invocation_overlay}`, `provider_overlay::tests::restoring_overlay_selectors_returns_every_overlay_variable_to_the_baseline`, `loop_control::tests::retry_resume::a_resume_whose_only_moved_facet_is_the_overlay_is_refused` | L1 + unit |

## Edge matrix

| Edge | Tests |
|---|---|
| Absent variables | `HOMEDRIVE`/`HOMEPATH` absent in every row of `L1::every_activation_reason_…`; absent `USERPROFILE` in `L1::absent_and_non_utf8_home_variables_pass_through_an_overlay_launch_verbatim`; absent `HOME` on Windows forms in `cli/src/commands/wrap/env/tests.rs::build_child_env_carries_native_windows_home_forms_through_an_overlay_launch`; `lib/src/invocation_context/tests.rs::an_absent_home_is_captured_as_absent_and_is_not_synthesized` |
| Non-UTF-8 Unix values | `L1::absent_and_non_utf8_home_variables_pass_through_an_overlay_launch_verbatim` (provider and nested `git`, byte-exact); unit: `env::tests::sanitize_process_env_preserves_non_utf8_values`, `provider_overlay::tests::{home_identity_violation_accepts_a_non_utf8_home, restoring_overlay_selectors_keeps_a_non_utf8_ambient_value}` |
| Paths containing spaces | `USERPROFILE=<home>/profile with space` on every `recording_command` launch; `explicit <provider> root` in the contract 3 test; ambient `my codex` / `my sqlite` in the transition tests; `home with space` in the Windows-forms unit test |
| Native Windows path forms | `env::tests::build_child_env_carries_native_windows_home_forms_through_an_overlay_launch` (drive-letter and UNC `USERPROFILE`, split `HOMEDRIVE`/`HOMEPATH`); `provider_overlay::tests::home_identity_violation_flags_null_devices_and_overlay_paths` (`NUL`/`nul`) |

## Missing source roots (Phase 11)

| Case | Behavior | Tests |
|---|---|---|
| Default root absent (provider never run) | Empty overlay; launch proceeds; the user's root is not created | `L1::a_missing_default_source_root_launches_with_an_empty_overlay`, `L1::codex_mcp_without_a_codex_root_injects_into_an_empty_overlay`; unit: `provider_overlay::tests::a_missing_default_source_root_builds_an_empty_overlay`; L2: `level2_lifecycle_control::{level2_lifecycle_resume_refuses_when_refresh_changes_mcp_server_set, level2_lifecycle_resume_refuses_when_refresh_changes_an_interpolated_mcp_tag}` |
| Explicit root absent | `provider.overlay_failed` at `source_root`, before storage | contract 9 row above |
| Root is a file | `provider.overlay_failed` at `source_root` | `provider_overlay::tests::a_source_root_that_is_a_file_fails_with_a_typed_error` |

## L2 and cross-platform evidence

`L2` = `cli/tests/level2_provider_overlay_capture.rs`. It runs `claudine codex
--repo` with no prompt inside a real terminal, so the provider inherits the
terminal as an interactive session, which piped L1 stdio never reaches. It
proves, for the provider and nested `git`/`gpg`/`gh`: stdin and stdout are TTYs;
the home variables are unchanged and carry no sentinel; `CODEX_HOME` names a
filesystem-backed overlay that carries settings and a directory two levels deep
and hides `skills`; SQLite stays at `~/.codex`.

| Environment | (a) home preservation + (b) filesystem overlay | Evidence | Status |
|---|---|---|---|
| macOS (local, 2026-09-16) | `L2::unix::level2_tmux_codex_repo_overlay_keeps_the_user_home_in_an_interactive_launch` (tmux, headless); all `L1` contracts | `just test-l2 --no-fail-fast`: 224/226, the 2 failures are unrelated (below); 20/20 `--stress-count` iterations at `-j 14`; `just test`: 7140/7140 after the inventory refresh | **met** |
| Linux (`build-linux`, scratch clone, 2026-09-16) | same L2 test with `BISCUIT_TEST_REQUIRED_BACKENDS=tmux`; overlay L1 and unit filter | L2 binaries `level2_provider_overlay_capture` + `level2_lifecycle_control`: 95/97 (same 2 unrelated failures); overlay L1/unit filter: 123/123 | **met** |
| WSL2 | L1 contracts run in CI's `wsl2-ubuntu` L1 cell (`#![cfg(unix)]`) | none: `build-win` resets SSH connections (its VHDX is on `W:`, 0 GB free); CI's `claudine-cli/wsl2-ubuntu/L2` cell is an `accepted-gap` | **unmet** |
| Native Windows | `L2::windows::level2_wezterm_windows_codex_repo_overlay_copies_nested_directories_and_keeps_the_user_home` (WezTerm `cmd.exe`, rustc-built recorder, write-through copy proof) | compile only: `cargo check -p claudine-cli --tests --features terminal-tests --target x86_64-pc-windows-gnu`, clean. Not run: `build-win-native` `W:` has 0 GB free. The test's own premise also fails there by design: Claudine's overlay home is `dirs::home_dir()`, the known-folder profile, so a fixture home cannot hold the overlay (human review). CI's `windows-latest/L2` cell is an `accepted-gap`; the Phase 5 copy-mode unit tests are the only native Windows materialization evidence (Phase 5 log, 26 passed on `build-win-native`) | **unmet** |

Non-vacuity of `L2` (each corruption applied alone, rebuilt, restored): `HOME`
moved to the overlay parent with the spawn guard off → "provider saw a moved
HOME"; `--repo` exclusions ignored → "must hide the user's skills"; SQLite
mirrored → "SQLite state was mirrored"; provider stdin not inherited → "did not
inherit the terminal".

Unrelated L2 failures on both macOS and Linux:
`level2_lifecycle_control::{level2_shipped_implement_plan_supplied_commit_message_runs_exact_commit_branch,
level2_shipped_implement_plan_unset_commit_message_reaches_provider_and_auto_branch}`.
The working tree's uncommitted `shipped_implement_route/_implement/implement-plan.md`
fixture edit (other in-flight work) still uses a bare `{{area}}` in its
`success`/`failure` messages, which strict subtree compose rejects: "unknown
root 'area'". No overlay is involved.
