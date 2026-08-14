---
kind: failing-catalog
created: 2026-08-14
run: 31753281913
commit: a00ea7c08
branch: fix/ctx-launch-anchor
status: in_progress
description: Every failing test identity in CI run 31753281913, grouped by whether this branch is a plausible cause
---

# Failing tests — run 31753281913 (`a00ea7c08`)

601 jobs: 459 passed, 118 skipped, **24 failed**, 1 still running when this was
captured. The run had not finished, so this catalog may be incomplete.

51 failing test identities. Grouped below by **whether this branch can plausibly
have caused them** — not by package. Suspicion is stated as a claim to be
checked, not a conclusion.

Local hardware results for the same commit, for contrast:

| Suite | Windows 11 | Linux | macOS | WSL2 |
| --- | --- | --- | --- | --- |
| claudine + claudine-cli L1 | 6050/6050 | pre-existing only | 6432/6432 | 5/5 pkgs green |
| darkmatter + darkmatter-cli + dmls L1 | 7542/7542 | 7083/7084 | 7088/7088 | not run |

Nothing below was caught by those runs, because **every local run was Level 1**.
`just test` does not run Level 2. Six of the failing jobs are Level 2.

---

## A. Plausibly caused by this branch — investigate first

### A1. `darkmatter-cli / test (darkmatter-cli on windows-latest)`

- `darkmatter-cli::schema_validate_baseline schema_validate_legacy_pretty_output_is_byte_identical`

A byte-identical output baseline. This branch changed how interpolated values
are escaped, which is exactly the class of change that moves such a baseline.
**Directly contradicts local evidence**: the same package passed 655/655 on a
native Windows 11 host at this commit. The contradiction itself is the finding —
either the CI environment differs in a way that matters, or the local run did
not cover this test.

### A2. `darkmatter / wsl2 (darkmatter) / test (darkmatter on wsl2-ubuntu)`

- `darkmatter::interpolation_literal_pipeline frontmatter_literal_survives_shell_bracketed_interpolation_passes`
- `darkmatter markdown::compose::frontmatter_shell_expansion::tests::detects_no_cache_suffix`
- `darkmatter markdown::compose::frontmatter_shell_expansion::tests::no_cache_combines_with_timeout_either_order`
- `darkmatter markdown::compose::frontmatter_shell_expansion::tests::no_cache_defaults_false_without_suffix`
- `darkmatter::shell_expansion_coordinates shell_block_execution_failed_renders_inner_diagnostic`
- `darkmatter::shell_expansion_coordinates shell_block_origin_counts_lines_not_bytes_with_crlf`

**Highest suspicion in the whole run.** The first name is a test this branch
introduced, and the cluster sits squarely on interpolation and shell expansion —
the two things changed here. This cell is **not** in the main-drift candidate
list, so it has no prior evidence of being red on main. Darkmatter was never run
on the WSL2 host locally, so this is an untested combination.

---

## B. Environment-sensitive Level 2 — likely not this branch, but unverified

None of these were exercised locally. The specification treats terminal
rendering as out of scope here, and the code-block colour-mode contract on CI
tmux is already a recorded main-side follow-up — but that is an argument, not
evidence.

### B1. `claudine-cli / test-l2` (macos-latest **and** ubuntu-latest)

- `level2_context_capture level2_context_default_at_140_fills_cap_in_tmux`
- `level2_context_capture level2_context_default_caps_at_140_in_wide_tmux`
- `level2_context_capture level2_context_default_narrow_preserves_type_and_wraps_in_tmux`
- `level2_context_capture level2_context_default_preserves_columns_at_min_width_in_tmux` (ubuntu only)

Failing on both operating systems, which weakens a pure-flake explanation.

### B2. `darkmatter-cli / test-l2`

- macos-latest — `level2_code_block_styling level2_code_block_clears_inherited_dim_before_theme_colors`
- ubuntu-latest — `level2_schema_about level2_schema_about_light_terminal_uses_dark_code_theme`

### B3. `biscuit-terminal-cli / test-l2`

- ubuntu-latest — `level2_diagrams level2_diagram_fallback_when_no_image_protocol`
- macos-latest — `level2_apple_terminal_prose level2_apple_terminal_double_underline_plain_text_visible`

This branch does not touch `biscuit-terminal`, though it does depend on
darkmatter.

### B4. `dmls / test-l2 (ubuntu-latest)`

- `dmls::level2_editor_neovim level2_neovim_decodes_semantic_token_families_and_positions`

Recorded main-side follow-up (Neovim provisioning).

---

## C. Confirmed or near-certain main drift — not this branch

### C1. `sniff` / `sniff-cli` lint (ubuntu-latest)

Clippy errors, no test identities (lint emits producer status only):

- `sniff-cli` — `this if statement can be collapsed`
- `sniff` — `call to set_readonly with argument false`; `items after a test
  module`; `spawned process is never wait()ed on`; `unused import: super::*`;
  `unused variable: helpers`

Both cells verified byte-identical against main. **Caveat worth remembering:**
a lint cell carries zero test identities, so once baselined, a *new* clippy
error there is invisible to the gate.

### C2. `sniff / test` — all four environments

- macos-latest — `hardware::gpu::tests::test_detect_gpus_on_macos`,
  `filesystem::repo::standard::tests::resolve_acting_binary_returns_none_when_binary_missing`
- windows-latest — `resolve_acting_binary_returns_none_when_binary_missing`
- ubuntu-latest — `resolve_acting_binary_returns_none_when_binary_missing`
- wsl2-ubuntu — `sniff::integration test_detect_completes_in_reasonable_time`

### C3. `sniff-cli / test`

- windows-latest — `cli test_repo_worktree_verbose_includes_path`,
  `cli test_repo_worktrees_verbose_output`, `snapshots repo_aggregate_json_snapshot`
- wsl2-ubuntu — `snapshots repo_aggregate_json_snapshot`

### C4. `rendezvous-daemon / test (windows-latest)`

- `private_dir::tests::the_current_user_descriptor_names_this_account_and_nobody_else`
- `local_transport::windows::tests::the_pipe_dacl_names_this_user_and_nobody_else`

### C5. `unchained-ai / test (windows-latest)`

- `primitives::services::pty_runner::tests::test_ansi_stripping`
- `primitives::services::pty_runner::tests::test_run_echo_command`

### C6. `biscuit-tui-cli / test (windows-latest)`

- `bin/question completions::tests::bash_script_passes_syntax_check`

### C7. `claudine / wsl2 (wsl2-ubuntu)`

- `system_prompt::prepare::tests::non_repository_session_runs_shell_in_launch_cwd`
- `composition::prepare::tests::direct_composition_runs_shell_in_configured_working_directory`
- `composition::error::tests::shell_expansion_failed_via_real_markdown_preserves_rich_diagnostic`

Exactly three identities, matching the three recorded against main for this cell.
All three are shell-execution-under-WSL2 tests. Note the same package passed
fully on the local WSL2 host at this commit, which supports the reading that
this is specific to the CI WSL2 environment.

### C8. `messenger / wsl2`

- `provider::desktop::linux::tests::native_fallback_delivers_when_no_helpers_installed`

### C9. `model_id / wsl2`

- `model_id::ui ui`

### C10. `biscuit-speaks-cli / wsl2`

No per-test identity recovered; the job reports only `test run failed`.

---

## What this changes

The branch cannot be declared clean on the strength of the local Level-1 runs.
Section A has to be resolved before any CI baseline entry is written, and
section B has to be attributed rather than assumed.

Section A2 in particular undercuts an earlier conclusion recorded in
`ci-baseline-evidence.md`: that no proposed baseline cell hides a branch
regression. That analysis diffed run 31651014023, which predates every fix on
this branch. `darkmatter/wsl2-ubuntu` was never a candidate cell and is failing
on tests this branch owns.
