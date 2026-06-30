---
ready: false
agent: codex/default
created: 2026-06-22T17:56:09
implemented: true
---

# Review 6

Production ready: **false**.

## Findings

### High: post-`start` harness setup errors still bypass stack-aware `failure`/`finalize`

The review-5 blocker is fixed for the named re-entry sites: materialization errors, target lifecycle parse errors, and harness-plan parse errors now route through the stack-aware blocked/finalize helper, and there is L1 plus L2 coverage for those paths. However, the harness loop still has ordinary `?` exits after the `start` event has already fired and before a terminal lifecycle event is recorded.

Evidence:

- `start` is emitted through `run_lifecycle_event(...)`, including top-level communication and `start.stack`, before the launch path starts (`claudine/cli/src/commands/wrap/harness_orch/loop_control.rs:1579-1677`).
- After that point, `capture_pre_run_snapshot(...)`, `build_harness_launch(...)`, and `attempt_result` are still propagated with `?` at `claudine/cli/src/commands/wrap/harness_orch/loop_control.rs:1679-1698` and `:1730-1763`.
- If any of these return `Err`, the only lifecycle fallback is `LifecycleRunGuard::drop`, which emits `Blocked` or `Failure` through the legacy `emit_signal` path (`claudine/lib/src/composition/lifecycle.rs:704-713`).
- That legacy path emits only `stderr`, `message`, `notify`, and audio; it does not emit `info`/`warn`, never runs `blocked.stack`/`failure.stack`, and never emits `finalize` (`claudine/lib/src/composition/lifecycle.rs:655-700`).
- These are reachable user-facing errors, not just impossible internals. For example, `execute_harness_attempt(...)` can fail before spawning while resolving runaway guard inputs (`claudine/cli/src/commands/wrap/harness_orch/attempt.rs:104-111`), and `build_harness_launch(...)` can fail while constructing provider resume args or prompt delivery (`claudine/cli/src/commands/wrap/harness_orch/launch.rs:23-44`).

This violates the lifecycle contract that a run which has reached `start` then fails should reach `failure` and `finalize`, with `err` available to the terminal stacks. A prompt author can observe the bug by putting an `append_line(...)` in `failure.stack` or `finalize.stack` and triggering one of these setup errors; the marker will not be written.

Verification level: the current focused lifecycle suite has strong L2 coverage for normal success/failure/blocked order, loop behavior, proxy/retry/resume/requeue, and the review-5 re-entry fixes. I found no L1 or L2 test that injects an error after `start` but before terminal event recording and asserts `failure.stack` plus `finalize.stack` with `err`. This needs at least L1 coverage around the harness-loop helper/control path and L2 coverage through `claudine compose` for one real post-start setup failure.

## Notes

Review-5's finding appears materially addressed:

- The former `materialize_harness_prompt(...)`, target lifecycle parse, and harness-plan parse exits now use stack-aware blocked/finalize routing.
- L2 tests now prove proxy target harness-plan failure and target lifecycle parse failure reach the expected terminal stacks with `err`.
- Requeue now has a daemon-first path plus durable fallback coverage for daemon-unreachable cases.

Focused verification run:

```text
cargo nextest run --manifest-path claudine/cli/Cargo.toml -E 'test(/lifecycle/) | test(/requeue_fallback/) | test(/loop_gate/) | test(/loop_initialize/)' --color=never
34 tests run: 34 passed, 1765 skipped
```

The remaining issue is narrower than the prior reviews, but it is still a public lifecycle semantics gap for authored `failure`/`finalize` stacks. I would not mark this production-ready until post-`start` setup errors use the same stack-aware terminal/finalize path as ordinary provider failures.
