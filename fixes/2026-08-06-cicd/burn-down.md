# Burn-down — morning report after the authoritative full-scope run

Source: run **31366269515** (full scope on `a3b0e25cd`, completed overnight
2026-08-10). 595 jobs, 470 result cells: **352 PASS · 27 FAIL · 34 governed
POLICY GAP · 20 NOTHING TO RUN · 36 NOT SCHEDULED · 1 MISSING**.

Trajectory: 102 failed jobs (attempt 2) → **33**. The WSL guest column went
from 63 dead legs to 8 real test-level failures; darkmatter-cli and
biscuit-terminal are now fully green on wsl2 (the insta and archive fixes,
confirmed at scale). The L2 backend-evidence corrections landed clean: no
zero-fail L2 cells remain, and dmls's neovim tests executed in CI for the
first time (2 of 3 pass).

## Baseline: corrected and verified

The seven `baseline-now-passing` entries flagged by the verdict are pruned
(biscuit-terminal-cli windows+wsl2, darkmatter-cli wsl2, model_id windows,
sniff-cli ubuntu+macos, research windows). Every remaining entry was
confirmed against this run's package-keyed evidence — the Phase 4 candidate
baseline is now **verified**, 17 entries. Re-running the verdict locally
against this run's results with the corrected baseline yields exactly the 13
blockers below and zero baseline noise.

## The 13 remaining blockers

### Lint (3) — trivial clippy, introduced by the fix rounds

| Cell | Error |
|---|---|
| `biscuit-terminal/lint` | `variable does not need to be mutable` in test `level1_apple_terminal_prose` |
| `sniff/lint` | `unused import: super::*`, `unused variable: helpers`, `call to set_readonly(false)` in test `merge_conflict_prediction` |
| `sniff-cli/lint` | `this if statement can be collapsed` in test `snapshots` |

### sniff L1 (4 cells) — one cross-OS test + two env-specific

- `resolve_acting_binary_returns_none_when_binary_missing` fails on
  **ubuntu, windows, macos** (identical) — the runner environment satisfies a
  lookup the test expects to miss.
- macos additionally: `hardware::gpu::test_detect_gpus_on_macos`.
- wsl2: `list_worktrees_propagates_registry_error` + git_parity twin.

### Windows L1 (2 cells) — remaining functional work

- `rendezvous-daemon`: `the_pipe_dacl_names_this_user_and_nobody_else`,
  `private_dir::the_current_user_descriptor_names_this_account_and_nobody_else`
  (down from 10 in the previous round).
- `unchained-ai`: `pty_runner::test_ansi_stripping`, `test_run_echo_command`.

### WSL2 L1 (2 cells) — today's burn-down territory

- `claudine`: 4 tests (`shell_expansion_failed_via_real_markdown…`,
  `direct_composition_runs_shell_in_configured_working_directory`,
  `validate_permissions_readonly_file`,
  `non_repository_session_runs_shell_in_launch_cwd`).
- `claudine-contract`:
  `shadow_home_copy_failure_logs_error_and_returns_secret_free_message`.

(The baselined wsl2 entries — biscuit-speaks-cli 2, claudine-cli 17,
darkmatter 11, sniff-cli 1, model_id 1 — are excused but are also today's
material; fixing them prunes the baseline further.)

### First-execution (1 cell)

- `dmls/ubuntu/L2`:
  `level2_neovim_decodes_semantic_token_families_and_positions` — 1 of the 3
  neovim tests, first time ever executed in CI. The other two pass (including
  the tmux rendering test, which also proves the backend-evidence chain
  end-to-end).

### Infrastructure flake (1 cell)

- `biscuit-contract/windows/L1` **MISSING**: the job died at "Install protoc"
  with `socket hang up` — a transient download failure, nothing to fix. A
  re-run of that single job (or the next full run) clears it. MISSING blocked
  instead of passing, which is the guard working.

## Also green and worth knowing

- The POLICY GAP machinery renders all 34 gaps governed (tmux/browser/GUI
  backends), each with owner and expiry — none block.
- Synthetic baseline identities (`claudine-gen-drift`, `coverage`) report
  out-of-scope, neither blocking nor passing.
- Specialized legs (`playa-windows`, `biscuit-tui-captured-stdout`,
  `rendezvous native windows`, `coverage`) remain red outside the verdict —
  unchanged, covered by the retirement spec.
