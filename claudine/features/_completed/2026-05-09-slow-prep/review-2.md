---
ready: false
agent: codex
model: ""
---

# Review: 2026-05-09 Slow Compose Prep

## Findings

### High: `sequence` still blocks on dynamic catalog refresh when env vars override frontmatter model

The direct `compose` / `inline-compose` path now probes `ModelResolutionReason` before refreshing, but the `sequence` path still uses the older gate:

- `claudine/cli/src/commands/wrap/sequence.rs:194`
- `claudine/cli/src/commands/wrap/sequence.rs:199`

That means a selected dynamic provider with any frontmatter `model` hint still calls `catalog.refresh_provider_blocking(provider)` whenever `--model` is absent, even if `OPENCODE_MODEL`, `QWEN_MODEL`, or generic `MODEL` will override the frontmatter value. This violates the spec requirement that provider-specific env vars and `MODEL` resolve without blocking on dynamic refresh unless the selected model actually requires catalog validation. It also leaves the slow `opencode models` subprocess on the pre-launch path for `claudine sequence --opencode` / `--qwen` cases.

Verification level: missing Level 1 CLI coverage. Add a sequence dry-run test with frontmatter `model`, `OPENCODE_MODEL` or `MODEL` set, and a failing/sleeping `opencode models` test double on `PATH`.

### High: Executor still redoes provider/config discovery after upstream target resolution

`compose`, `inline-compose`, and `sequence` now pass `resolved_target: Some(...)` into `execute_composition_request_inner`, but the executor still builds a fresh `InstalledAiClients` snapshot and calls `load_selection_config(...)` before checking whether `request.resolved_target` is already present:

- `claudine/cli/src/commands/wrap/composition/mod.rs:521`
- `claudine/cli/src/commands/wrap/composition/mod.rs:529`

`load_selection_config()` itself calls `sniff::filesystem::git::detect_git`, so this keeps an extra repo discovery on the hot path after `CompositionPrepContext` already loaded the selection config. This conflicts with the spec's `CompositionPrepContext` requirement and with the acceptance check of at most one source repo-root discovery outside `biscuit-file`'s required resolution.

Verification level: missing Level 1/perf assertion. The new tests prove no unrelated `opencode models` call for some compose cases, but they do not assert the remaining duplicate repo/config discovery. The executor should use the precomputed context when a target is supplied, or skip installed-provider/config/catalog setup entirely when `request.resolved_target` is `Some`.

### High: Ctrl+C during prep test is not a reliable slow-prep verification

`compose_sigint_during_prep_exits_130_with_notice` intends to slow prep by making `opencode models` sleep, but the command under test has no frontmatter `model` hint:

- `claudine/cli/tests/wrap_commands.rs:5339`
- `claudine/cli/tests/wrap_commands.rs:5361`

The refresh gate returns early when `hints.model.is_none()`, so the sleeping `opencode models` path is not required to run. The test can pass or fail based on incidental local prep timing rather than a deterministic blocked prep phase. This does not satisfy the acceptance requirement for SIGINT during the slow-prep window.

Verification level: Level 1 is appropriate for signal delivery here, but the current Level 1 test setup does not reliably hold the process in prep. Add a frontmatter `model` hint for a dynamic selected provider, or introduce an explicit test-only prep delay, then assert exit 130 and the clean notice.

### Medium: Cached environment context does not cover the common repo-subdir case

`CompositionPrepContext` computes `prep_env_context` from a shared launch-CWD sniff scan, but the executor only reuses it when `env_detect_root == launch_cwd`:

- `claudine/cli/src/commands/wrap/composition/mod.rs:1280`
- `claudine/cli/src/commands/wrap/composition/mod.rs:1290`

For the common case where the command is launched from a package/subdirectory and the source repo root is the enclosing repo, `env_detect_root` is the repo root while `launch_cwd` is the subdirectory. That still falls back to `detect_environment_fast(env_detect_root)`, leaving another pre-launch sniff scan in ordinary monorepo usage. If the intended decision is "instrument only and defer lazy environment work," this reuse path should not claim to remove the duplicate scan for common repo runs.

Verification level: missing Level 1/perf coverage. Add a test or trace assertion for a launch CWD below the repo root.

### Medium: Interrupt test cleanup clears only the CLI flag, not the lib-side flag

`mark_user_interrupted()` sets both the CLI-local flag and `claudine::interrupt`:

- `claudine/cli/src/output/mod.rs:651`
- `claudine/cli/src/output/mod.rs:653`

But `clear_user_interrupt_for_tests()` only clears the CLI-local flag:

- `claudine/cli/src/output/mod.rs:664`

The new `sigint_during_prep_sets_interrupt_flag_and_renders_notice` unit test calls that cleanup after raising SIGINT, leaving the lib-side flag set for any later in-process test that checks lifecycle side effects through `claudine::interrupt::interrupted()`.

Verification level: Level 1 unit cleanup gap. Clear both flags in the helper or use a guard that restores both process-wide flags.

## Coverage Notes

- Provider/model refresh: Level 1 unit coverage now exists for direct compose env-var gating and catalog dedup, but sequence has no matching env-var override coverage.
- No-`opencode models` acceptance: Level 1 CLI coverage exists for explicit `compose --claude`, `inline-compose --claude`, and `compose --codex`; sequence is not covered.
- Ctrl+C during prep: Level 1 signal testing is the right boundary for this feature, but the current integration test does not deterministically create the slow-prep condition it claims to exercise.
- Terminal rendering: no Level 2 requirement is central to the performance feature unless exact color/glyph rendering of the interrupt notice is considered part of acceptance.

## Validation

Ran targeted validation:

```sh
cargo test -p claudine-cli compose_claude_dry_run_does_not_call_opencode_models --no-default-features
```

This passed. The filter also matched `inline_compose_claude_dry_run_does_not_call_opencode_models`, so both explicit Claude no-`opencode models` tests passed.

## Readiness

Not ready for production. The direct compose path is closer, but `sequence` still violates the dynamic-refresh rules, the executor still repeats hot-path discovery after prep context resolution, and the Ctrl+C acceptance test is not a reliable verification of the user-visible interrupt behavior during slow prep.
