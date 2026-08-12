---
ready: false
agent: codex/default
created: 2026-06-22T18:35:34
implemented: true
---

# Review 8

Production ready: **false**.

## Findings

### High: blocked-side lifecycle controls are not verified at Level 2

The implementation now has runtime wiring for terminal controls from `blocked.stack`: the blocked pre-flight branch calls `dispatch_terminal_control(...)`, and the dispatch path treats `Retry` from `blocked` as a pre-flight/start re-entry while also allowing `Proxy` and `Requeue` (`claudine/cli/src/commands/wrap/harness_orch/loop_control.rs:1568`). However, the real-terminal lifecycle control suite only exercises the user-observable control actions from `failure` plus `Proxy` from `initialize`; its own coverage list names `Retry from failure`, `Resume from failure`, `Requeue from failure`, `Proxy from failure`, and `Proxy from initialize`, but not `Retry`/`Proxy`/`Requeue` from `blocked` (`claudine/cli/tests/level2_lifecycle_control.rs:11`).

That leaves a mismatch with the spec's L2 requirement for lifecycle control flow. `Retry` from `blocked` must re-run pre-flight/start after a stack side effect fixes the blocked condition, `Proxy` from `blocked` must hand off to the target document without invoking the source provider, and `Requeue` from `blocked` must persist the deferred prompt and then finalize. These are externally observable CLI behaviors and need Level 2 coverage through `claudine compose` in a real terminal, not just parser or pure decision tests.

Strongest verification found:

- `blocked.retry` parses at L1 in `parses_retry_with_count_in_blocked` (`claudine/lib/src/composition/lifecycle.rs:4636`).
- Pure control dispatch marks `Retry` from `blocked` as `from_blocked: true` at L1 in `retry_from_blocked_re_enters_preflight` (`claudine/lib/src/composition/lifecycle_control.rs:319`).
- `Proxy` validity for `blocked` is covered only by pure decision logic, and `Requeue` is covered only generically/from `failure`; I found no `level2_*` test whose document has a `blocked.stack` ending in `retry(...)`, `proxy(...)`, or `requeue`.

Add focused L2 cases in `claudine/cli/tests/level2_lifecycle_control.rs`: one blocked pre-flight that becomes valid after a `blocked.stack` side effect then `retry`, one blocked `proxy` that runs the target lifecycle, and one blocked `requeue` that records the deferred prompt without launching the provider. Until those pass, this feature does not meet the review's stated production-readiness bar.

## Notes

The review-7 issue appears fixed: `loop.stdout` now returns `CompositionError::LifecycleStdoutRejected` before the generic loop parser can classify it as an unknown loop key (`claudine/lib/src/composition/lifecycle.rs:1044`), with L1 coverage in `rejects_stdout_field_on_loop_block` (`claudine/lib/src/composition/lifecycle.rs:4975`).

Focused verification run:

```text
cargo nextest run --manifest-path claudine/lib/Cargo.toml -E 'test(/rejects_stdout_field_on_loop_block/) | test(/retry_from_blocked_re_enters_preflight/) | test(/requeue_carries_delay_and_reason/) | test(/proxy_swaps_target_at_valid_signals/)' --color=never
4 tests run: 4 passed, 2949 skipped
```
