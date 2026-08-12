---
ready: false
agent: codex/default
created: 2026-06-23T08:21:53
implemented: true
---

# Review: Remove Validations

## Findings

### High: Removed-key diagnostics are only scanner-unit-tested, not verified at the CLI/user boundary

The spec requires old prompt files declaring `pre_checks`, `post_checks`, `handle`,
`handle_*`, or `deviate` to fail with a typed, actionable
`CompositionError::RemovedValidationKey`, before generic lifecycle validation and
before the provider is launched. The implementation has the scanner in
`claudine/lib/src/composition/lifecycle.rs:993`, calls it from direct and inline
prepare paths in `claudine/lib/src/composition/prepare.rs:114` and
`claudine/lib/src/composition/prepare.rs:234`, and renders a typed diagnostic in
`claudine/lib/src/composition/error.rs:1307`.

The strongest tests I found are unit tests for `scan_removed_validation_keys`
(`claudine/lib/src/composition/lifecycle.rs:3302` onward). That proves key matching,
but it does not verify the user-facing contract:

- `claudine compose` / `inline-compose` / `sequence` exit non-zero with the typed
  diagnostic.
- The error happens before the provider stub is launched.
- The diagnostic survives the CLI render boundary and is not downgraded to
  prompt-shape, schema, or generic lifecycle validation errors.
- The frontmatter excerpt highlights the removed key when stderr is a TTY.

Verification level: current coverage is below Level 1 for the user-observable CLI
behavior. At minimum add Level 1 integration tests with prompt fixtures for direct,
inline, and sequence paths, asserting stderr contains `removed validation/handler key`,
the specific offending key, and the lifecycle replacement text, and that the provider
stub was not invoked. The TTY-gated frontmatter-highlight requirement should have a
Level 2 capture if the colored/highlighted rendering is part of the acceptance
contract.

### Medium: Public library README still advertises removed validation/handler surfaces

`claudine/lib/README.md:269` says composition preflight collects shell approval for
``shell_command` validations, and `deviate`/`handle` commands``. Those surfaces are
explicitly retired by this feature, and the implementation now routes lifecycle shell
preflight through lifecycle stacks instead. This is a public package-area README, so
leaving it stale undermines the migration story and can lead users to author removed
frontmatter keys that now fail.

Update that bullet to describe the kept surfaces only: source `::shell` directives,
frontmatter shell expansion, and lifecycle stack shell actions.

## Notes

- The core DSL types and evaluators appear to be removed from the current harness
  model: I did not find `ValidationRule`, `ValidationKind`, `HandlerTable`,
  `evaluate_pre_checks`, `evaluate_post_checks`, `capture_pre_run_snapshot`,
  `resolve_handler`, or the validation reporter harness in the active lib/CLI source.
- `harness/resolve.rs` remains, but it is still used by lifecycle `Proxy` target
  resolution (`claudine/lib/src/composition/lifecycle_control.rs:245`), so keeping the
  helper is reasonable. Its module docs should eventually be reworded away from
  "validation subjects".

## Verification Run

- `just test-library scan_rejects_pre_checks_removed_key scan_rejects_handle_timeout_removed_key`
  passed: 2 tests run, 2 passed.
- `just test-cli lifecycle_failure_retry_replaced_removed_handler_dsl` compiled and ran
  zero tests, so it did not provide useful verification for this review.

## Production Readiness

Not production ready. The implementation looks directionally correct, but the
production-facing compatibility diagnostic is not verified at the level required by
the spec.
