# Claudine CLI slow-test inventory

Baseline command:

```text
just _test claudine-cli --no-fail-fast
```

The 2026-08-01 Linux baseline ran 2,345 L1 tests across 112 binaries in
50.904 seconds: 2,342 passed, 56 crossed nextest's slow threshold, three
failed, and 239 were skipped. A second run with slow-only reporting completed
in 51.445 seconds and placed 61 tests over the threshold. The larger observed
set is the optimization inventory because tests close to five seconds move
across the boundary under concurrent load.

The three originally reported timeout cases did not time out in either local
baseline. They failed quickly and consistently because provider installation
validation ran before the expected retired-flag validation. They remain in
scope as failing/flaky cases.

## Slow tests

### `compose_header_first` (5)

- `compose_quiet_keeps_the_execution_header`
- `compose_execution_header_is_the_first_signal`
- `compose_silent_suppresses_the_execution_header`
- `inline_compose_execution_header_is_the_first_signal`
- `inline_compose_silent_suppresses_the_execution_header`

### `inline_compose_hash` (1)

- `inline_compose_writes_hash_that_passes_md_diff`

### `mcp_cli` (1)

- `gemini_and_opencode_wrapper_mcp_dry_run_show_provider_specific_injection`

### `prompt_reporting` (11)

- `compose_default_shows_summary_and_short_user_prompt`
- `compose_env_var_quiet_shows_summary_only`
- `compose_env_var_silent_suppresses_system_prompt`
- `compose_env_var_verbose_shows_full_system_prompt`
- `compose_long_user_prompt_uses_frontback_truncation`
- `compose_quiet_shows_system_summary_and_full_agent_prompt`
- `compose_silent_suppresses_all_prompt_reporting`
- `compose_system_prompt_summary_shows_token_count`
- `compose_user_prompt_at_40_lines_renders_full`
- `compose_user_prompt_at_41_lines_uses_frontback`
- `compose_verbose_shows_full_prompts`

### `sequence_perf` (1)

- `sequence_perf_propagates_startup_timings`

### `sequence_schema` (1)

- `sequence_per_step_step_timeout_override`

### `shipped_prompt_contract` (1)

- `feature_review_cli_preserves_numeric_iteration_and_dependent_paths`

### `wrap_compose_agent` (1)

- `agent_hint_resolved_early_in_non_tty`

### `wrap_compose_exec` (6)

- `compose_interactive_claude_seeds_prompt_as_positional_arg`
- `compose_interactive_kimi_seeds_prompt_with_prompt_flag`
- `compose_resolves_env_agent_in_body_template`
- `compose_supports_mcp_runtime_and_tag_cleanup`
- `compose_uses_wrapper_grade_execution`
- `explicit_provider_flag_bypasses_chooser`

### `wrap_compose_preflight` (1)

- `compose_interactive_preflight_with_whitelisted_command`

### `wrap_compose_validation` (2)

- `compose_success_when_evaluation_error_surfaces_before_finalize_marker`
- `no_cross_provider_retry_after_launch`

### `wrap_direct_argv` (1)

- `direct_wrap_opencode_argv`

### `wrap_inline_compose` (5)

- `inline_compose_no_overwrite_on_failure`
- `inline_compose_preserves_frontmatter`
- `inline_compose_rejects_empty_captured_output`
- `inline_compose_resolves_env_agent_in_prompt_template`
- `inline_compose_writes_only_final_response_not_narration`

### `wrap_inline_compose_interactive` (1)

- `inline_compose_interactive_codex_uses_captured_last_message`

### `wrap_opencode` (5)

- `compose_opencode_non_interactive_passes_prompt_as_positional_arg`
- `compose_opencode_serviceless_stderr_lines_are_consumed`
- `opencode_non_interactive_model_precedence_uses_env_overrides`
- `opencode_stderr_mixed_shapes_only_consume_classified_lines`
- `opencode_stderr_rate_limit_before_stdout_forces_early_termination`

### `wrap_opencode_models` (1)

- `compose_opencode_dry_run_calls_opencode_models_and_fails_with_test_double`

### `wrap_perf` (3)

- `compose_perf_emits_report_to_stderr`
- `inline_compose_perf_emits_report_to_stderr`
- `perf_arg_parsing_includes_clap_time`

### `wrap_structured_stream` (8)

- `codex_structured_compose_filters_stdin_banner`
- `codex_structured_compose_surfaces_live_tool_progress`
- `codex_structured_mode_reconstructs_stdout_and_writes_summary_event`
- `gemini_structured_success_suppresses_provider_stderr_noise`
- `structured_completion_summary_is_separated_on_stderr`
- `structured_quiet_verbose_uses_old_verbose_summary_renderer`
- `structured_verbose_summary_reports_no_tool_calls_when_absent`
- `structured_verbosity_controls_stream_stderr_lines`

### `wrap_watchdog_timeout` (6)

- `compose_non_harness_respects_cli_timeout`
- `inline_compose_non_harness_respects_cli_step_timeout`
- `watchdog_opencode_post_fanout_silence_does_not_kill_prematurely`
- `watchdog_stream_idle_timeout_after_tool_call_hang`
- `watchdog_subagent_hang_terminates_and_names_stuck_ids`
- `watchdog_wall_clock_timeout_terminates_active_stream`

## Failing or reported-flaky tests (3)

- `wrap_compose_validation::retired_compose_flag_rejected_in_wrapper`
- `wrap_compose_validation::retired_frontmatter_prompt_flag_rejected_in_wrapper`
- `wrap_compose_validation::retired_prompt_file_flag_rejected_in_wrapper`

## Results

Every inventoried test retained its real Claudine binary boundary and the
provider/file/parser/termination behavior named by its assertions. The common
optimization was to run synthetic fixtures from their temporary workspace
instead of inheriting the rusty-biscuit root, and to disable rendezvous session
reporting when the test did not cover rendezvous behavior.

| Group | Cases | Before | After |
|---|---:|---:|---:|
| Prompt and execution-header reporting | 16 | 5.0–6.8s each | 0.12–0.17s each |
| Inline compose and hash validation | 7 | 3.7–4.9s each | 0.13–0.23s each |
| Wrapper compose, agent selection, and preflight | 8 | 3.7–6.0s each | 0.13–0.23s each |
| MCP, OpenCode argv/model/parser paths | 8 | 24.663s serial total | 1.375s serial total |
| Structured streams | 8 | 28.031s serial total | 1.802s serial total |
| Watchdog and termination | 6 | 5.3–7.2s each | 1.2–3.2s each |
| Performance and sequence timeout | 5 | 20.348s serial total | 2.131s serial total |
| Shipped feature-review prompt | 1 | 5.659s | 2.044s |
| Lifecycle/cross-provider validation | 2 | 3.7–3.8s each | 0.14–0.21s each |
| Retired wrapper flags | 3 | Failed on every retry | 0.02s each, passing |

The final full L1 run completed 2,345 tests in 25.203 seconds: all 2,345
passed, 239 tier-gated tests were skipped, and nextest reported no slow,
flaky, failed, or timed-out tests. The comparable baselines took 50.904 and
51.445 seconds, with 56–61 slow tests and three failures.

## Follow-up timeout audit

A later report named three unique tests, with two names repeated. Two still
inherited ambient launch context that could become expensive under concurrent
suite load; the third was already fully isolated.

| Test | Focused before | Focused after | Change |
|---|---:|---:|---|
| `compose_system_prompt_shell_failure_renders_rich_block` | 0.433s | 0.025–0.034s | Isolated HOME/CWD and disabled unrelated rendezvous reporting |
| `compose_preflight_error_includes_source_provenance` | 0.456s | 0.024–0.028s | Isolated launch CWD |
| `compose_preflight_discovers_shell_inside_false_block` | 0.022–0.024s | 0.028s in the combined audit | No change; already isolated and deterministic |

The three-test combined run passed in 0.038 seconds. A subsequent full L1 run
completed all 2,345 tests in 25.332 seconds with no slow, flaky, failed, or
timed-out cases. The two modified tests retain the real CLI boundary and all
original assertions. The unchanged false-block test still proves
condition-blind static shell discovery and that preflight prevents provider
launch.

## Production concerns catalog

These findings were not changed in this test-performance pass.

1. **Large-repository startup discovery dominates wrapper and composition
   latency.** Controlled tests took roughly 2.3–4.8 seconds from the
   rusty-biscuit root and 0.13–0.25 seconds from an isolated directory.
   Prompted wrappers request Sniff Git summary and repository structure, while
   composition performs launch-repository/context discovery and source-repo
   preparation. Profile whether every synchronous request is required, then
   consider narrower requests, reuse, caching, or deferred discovery.
   The follow-up provenance test also showed that a deterministic preflight
   failure pays this launch-context cost before it can return its diagnostic.
2. **Root system-prompt discovery adds measurable composition work.** The
   shipped-prompt investigation attributed about 1.4 seconds of its original
   run to automatically discovering and composing the repository-root
   `system-prompt.md`. This may be intended behavior, but it belongs in a
   user-visible startup budget.
3. **Short step timeouts are enforced at the watchdog sampling cadence.** With
   the default five-second cadence, a one-second `step_timeout` completed in
   about 5.42 seconds. The optimized test uses a 100ms test-only cadence while
   retaining the real timeout evaluator and process-group termination.
4. **Rendezvous reporting is default-on for provider launches.** It is bounded
   and was not the dominant cost in controlled isolation, but it adds
   best-effort connection work to commands unrelated to the dashboard. Its
   missing-daemon and live-but-wedged latency contracts should remain part of
   startup profiling.
5. **One OpenCode model test has stale identity.**
   `compose_opencode_dry_run_calls_opencode_models_and_fails_with_test_double`
   neither passes `--dry-run` nor fails; its environment-selected model causes
   dynamic model refresh to be skipped. Rename and rewrite its comments in a
   separate comment/test-maintenance change.
6. **One rate-limit assertion is tautological.** The OpenCode early-termination
   test compares `row["error"]` with itself, so it does not prove the promised
   rate-limit message content even though the real parser and early child
   termination remain covered.
7. **Relative file-reference behavior differs by consumer.** In an isolated
   external workspace, a relative `spec=` reference resolved for `file_exists`
   but not for `frontmatter`, causing the shipped prompt's iteration to fall
   back to one. The test uses an absolute synthetic spec path to keep its
   numeric/dependent-path contract deterministic; the production inconsistency
   needs separate diagnosis.

The timeout test also contained a stale comment claiming a five-second
SIGTERM grace. The implementation uses a ten-second default and the fixture
exits on TERM before escalation, so the comment was corrected alongside the
test-only cadence change.
