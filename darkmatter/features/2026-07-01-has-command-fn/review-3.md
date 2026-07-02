---
ready: false
implemented: true
agent: codex/default
created: 2026-07-01T22:56:31
---

# Review 3 - `has_command(cmd)`

## Findings

### Medium - The feature branch includes an unrelated global L2 test runner change

The `has_command` implementation surface in the spec is limited to the expression handler/dispatch table, descriptor catalog, generated/prose docs, tests, and the feature spec. The current diff also changes `darkmatter/justfile:92` onward to default `just test-l2` into parallel self-spawn mode. That is unrelated to `has_command(cmd)` and changes validation behavior for the whole darkmatter package area.

The new justfile comment says every darkmatter L2 test attaches via `WezTermHarness::shared_or_spawn()` (`darkmatter/justfile:92`), but the tree still has tmux-backed L2 tests such as `darkmatter/cli/tests/level2_schema_about.rs:10` and `darkmatter/cli/tests/level2_schema_about.rs:174`. The testing guidance says tmux can be parallel-safe when isolated, so this is not necessarily wrong as a standalone optimization. It is still not part of this feature, it is justified with an inaccurate statement, and it has not been validated by this review.

Impact: merging this feature would also merge a package-wide L2 harness policy change. If that change flakes or alters skip/hard-fail behavior, the regression would be attributed to `has_command` even though the command-existence feature does not need L2 at all.

Required fix: remove the `darkmatter/justfile` change from this feature branch, or split it into its own reviewed change with corrected comments and a `just darkmatter test-l2` validation pass.

Verification level: not a user-facing `has_command` behavior. This is a branch hygiene and test-infrastructure risk, not a Level 1/2/3 coverage mismatch for the expression function.

## Coverage Notes

For the `has_command(cmd)` behavior itself, Level 1 is the appropriate verification level: this is in-process expression evaluation plus host filesystem/PATH probing, not terminal rendering or OS keyboard input.

The implementation now covers the specified behavior with Level 1 tests:

- present command on `PATH` and missing command
- `null`, non-string, and empty-string inputs returning `false`
- absolute executable, absolute missing path, Unix non-executable path, and directory probes
- tilde and relative path gaps
- the prior regression where an existing executable under the process CWD made `./tool` or `bin/tool` return `true`
- canonical `has_command` and alias `hascommand` dispatch
- descriptor/catalog parity and generated docs parity

I ran:

```text
cargo nextest run --color=never -p darkmatter -E 'test(/has_command/) + test(/descriptor_signature_set_equals_dispatchable_signature_set/) + test(/narrative_doc_function_table_matches_catalog/)'
```

Result: 14 passed, 5033 skipped.

I did not run the full `just darkmatter test`, `just darkmatter lint`, or `just darkmatter test-l2` suite. The focused Level 1 suite is sufficient for the `has_command` user-facing requirements, but the unrelated `justfile` change should not ship without its own L2 validation.

## Open Questions

None.

## Production Readiness

Not ready for production as a branch because of the unrelated L2 runner change. The `has_command(cmd)` implementation itself appears complete against the spec after the review-2 relative-path fix.
