---
ready: false
agent: codex/default
created: 2026-06-23T10:16:05
implemented: true
---

# Review: Remove Validations, Iteration 2

## Findings

### High: Removed-key frontmatter excerpt/highlight is not verified at Level 2

The spec requires removed validation/handler keys to fail with a typed diagnostic
and to include a frontmatter excerpt/highlight in TTY-capable output. The
implementation does attach the offending removed key as the frontmatter highlight
target: `CompositionError::RemovedValidationKey` maps to `Some(Some(key.clone()))`
in `claudine/lib/src/composition/error.rs:1177`, and the excerpt renderer gates
display on TTY output in `claudine/lib/src/composition/frontmatter_excerpt.rs`.

The new L1 CLI tests in `claudine/cli/tests/compose_removed_validation_keys.rs`
verify the non-TTY/user-boundary diagnostic for `compose`, `inline-compose`, and
`sequence`, including provider-not-launched behavior. They do not verify the
TTY-gated excerpt or highlighted YAML line. I found no `level2_*` test that runs
a removed-key prompt in a real terminal and captures stderr/pane text containing
the frontmatter block and offending key line.

Verification level: strongest coverage for this user-observable requirement is
Level 1. The spec's TTY/highlight rendering requirement needs Level 2 capture
(tmux/WezTerm/Kitty) because it depends on the real terminal render boundary,
line wrapping, code-block rendering, and SGR/highlight output. Add a focused
`level2_*` test for a removed key such as `pre_checks` that asserts the captured
pane includes the `removed validation/handler key` diagnostic, the YAML
frontmatter block, and the offending key line. If color/highlight itself is part
of the acceptance contract, assert SGR is present around the highlighted YAML
line, following the existing dry-run metadata capture pattern.

## Verification Matrix

- Removed frontmatter keys reject with typed diagnostics: Level 1 present via
  scanner unit tests and `compose_removed_validation_keys` CLI integration tests.
- Provider is not launched when a removed key is present: Level 1 present in
  `compose_removed_validation_keys`.
- `compose`, `inline-compose`, and `sequence` user-boundary removed-key behavior:
  Level 1 present.
- Removed-key frontmatter excerpt/highlight in TTY-capable output: gap; no
  feature-specific Level 2 coverage found.
- Shell-audit denial routes through lifecycle `blocked`/`finalize`: Level 2
  present in `level2_lifecycle_dispatch`.
- Agent-failure recovery through lifecycle `failure` `Retry`: Level 2 present in
  `level2_lifecycle_failure_retry_recovers_to_success`.

## Notes

- The core removed symbols still appear absent from active lib/CLI code:
  `evaluate_pre_checks`, `evaluate_post_checks`, `capture_pre_run_snapshot`,
  `PreRunSnapshot`, `ValidationRule`, `ValidationKind`, `HandlerTable`,
  `resolve_handler`, and `try_resolve_handler` were not found in the active
  harness/wrap implementation.
- The review-1 README issue appears fixed in `claudine/lib/README.md`; the stale
  `shell_command` validation and `deviate`/`handle` wording was replaced with
  source `::shell` directives and lifecycle stack `shell` actions.

## Verification Run

- `just test-library scan_rejects_pre_checks_removed_key scan_rejects_handle_timeout_removed_key scan_rejects_handle_inline_body_unchanged_removed_key`
  passed: 3 tests run, 3 passed.
- `cargo nextest run -p claudine-cli --test compose_removed_validation_keys --color never`
  passed: 8 tests run, 8 passed.
- `just test-l2 level2_lifecycle_failure_retry_recovers_to_success` passed: 1
  Level 2 test run, 1 passed.
- `just test-cli compose_removed_validation_keys` was also tried first but ran
  zero tests because that recipe argument filtered by test name rather than test
  binary; I did not count it as verification.

## Production Readiness

Not production ready. The implementation satisfies the core removal and the
review-1 CLI-boundary gap, but the spec still has an explicit TTY/highlighted
diagnostic requirement whose strongest feature-specific verification is below
the required level.
