---
ready: false
agent: codex/default
created: 2026-06-22T18:20:54
implemented: true
---

# Review 7

Production ready: **false**.

## Findings

### Medium: `loop.stdout` is rejected through the generic loop parser, not the lifecycle stdout diagnostic

The implementation correctly rejects `stdout` fields on ordinary lifecycle event blocks and rejects `stdout(...)` actions in lifecycle stacks, but the shared `loop:` block has a different parse path. `parse_lifecycle_config` extracts only keys listed in `LIFECYCLE_CONCERN_KEYS` from the `loop:` object before calling `parse_event_block`, so a top-level `loop.stdout` key never reaches the `LifecycleStdoutRejected` check in `parse_event_block` (`claudine/lib/src/composition/lifecycle.rs:1023`, `:1044`, `:1096`). It is later rejected by `resolve_loop_config` as an unknown loop key through `CompositionError::LoopInvalid` (`claudine/lib/src/composition/loop_config.rs:288`).

That still prevents stdout output, but it violates the feature's typed parse-time contract: lifecycle `stdout` fields and `stdout(...)` actions should fail as `CompositionError::LifecycleStdoutRejected`, consistently across every event surface. It also misses the dedicated diagnostic text in `CompositionError::LifecycleStdoutRejected` that tells authors stdout is reserved for pipeable output and suggests `stderr`, `info`, `warn`, or file/frontmatter side effects instead.

Verification level: this is an L1 parser/diagnostic requirement. Existing L1 coverage exercises `stdout` rejection for normal event fields and stack actions, but I found no test for `loop: { stdout: ... }` asserting the `LifecycleStdoutRejected` variant. Add a unit test for `parse_lifecycle_config`/prepare on `loop.stdout`, and route that key through the same typed rejection path before `resolve_loop_config` falls back to `LoopInvalid`.

## Notes

The review-6 blocker appears addressed:

- Post-`start` snapshot, launch, and pre-spawn attempt failures now call `emit_failure_finalize_with_err`.
- There is L1 helper coverage and L2 CLI coverage using an invalid runaway regex to prove `start` fires, the provider does not spawn, and `failure.stack` plus `finalize.stack` observe `err`.

Focused verification run:

```text
cargo nextest run --manifest-path claudine/cli/Cargo.toml -E 'test(/lifecycle/) | test(/requeue_fallback/) | test(/loop_gate/) | test(/loop_initialize/)' --color=never
35 tests run: 35 passed (1 flaky), 1767 skipped
```

The flaky retry was `level2_lifecycle_failure_resume_without_session_surfaces_typed_error`; nextest reported a first-attempt leak-handle failure even though the test body passed, then it passed on retry. That does not look like a lifecycle semantics failure, but it should be watched because leak-handle flakiness can hide cleanup regressions in L2 tests.
