---
ready: true
agent: codex/default
created: 2026-06-23T10:37:50
implemented: true
---

# Review: Remove Validations, Iteration 3

## Findings

No blocking findings.

The review-2 gap is addressed. The removed-key diagnostic now has Level 2
real-terminal coverage in `claudine/cli/tests/level2_removed_validation_key_capture.rs`:
the tmux test drives `claudine compose --goose` in a real terminal pane, captures
the rendered output, and asserts the `removed validation/handler key` diagnostic,
the offending `pre_checks` key, lifecycle replacement guidance, the authored YAML
frontmatter excerpt, and styled terminal output are all present.

## Verification Matrix

- Removed frontmatter keys reject with typed diagnostics: Level 1 present via
  scanner unit tests and `compose_removed_validation_keys` CLI integration tests.
- Provider is not launched when a removed key is present: Level 1 present in
  `compose_removed_validation_keys`.
- `compose`, `inline-compose`, and `sequence` user-boundary removed-key behavior:
  Level 1 present.
- Removed-key frontmatter excerpt/highlight in TTY-capable output: Level 2 present
  via `level2_tmux_removed_key_renders_styled_diagnostic_with_yaml`; WezTerm
  coverage is also present and skip-gated by backend availability.
- Shell-audit denial routes through lifecycle `blocked`/`finalize`: Level 2
  coverage remains present in the lifecycle dispatch tests from the prior review.
- Agent-failure recovery through lifecycle `failure` `Retry`: Level 2 coverage
  remains present in `level2_lifecycle_failure_retry_recovers_to_success`.

## Notes

- The removed implementation symbols remain absent from active lib/CLI code:
  `evaluate_pre_checks`, `evaluate_post_checks`, `capture_pre_run_snapshot`,
  `PreRunSnapshot`, `ValidationRule`, `ValidationKind`, `HandlerTable`,
  `resolve_handler`, `try_resolve_handler`, and the validation reporter harness
  were not found in active source.
- Remaining documentation hits for `pre_checks`, `post_checks`, `handle_*`,
  `handle`, and `deviate` are migration/removed-surface notes, not accepted
  authoring instructions.

## Verification Run

- `just test-library scan_rejects_pre_checks_removed_key scan_rejects_handle_timeout_removed_key scan_rejects_handle_inline_body_unchanged_removed_key`
  passed: 3 tests run, 3 passed.
- `cargo nextest run -p claudine-cli --test compose_removed_validation_keys --color never`
  passed: 8 tests run, 8 passed.
- `cargo nextest run -p claudine-cli --test level2_removed_validation_key_capture --color never -E 'test(level2_tmux_removed_key_renders_styled_diagnostic_with_yaml)'`
  passed: 1 Level 2 tmux test run, 1 passed.

## Production Readiness

Production ready. The implementation satisfies the removal contract and the
previously missing user-observable TTY diagnostic coverage now exists at the
required verification level.
