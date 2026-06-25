---
ready: false
agent: codex/default
created: 2026-06-22T16:37:39
implemented: true
---

# Review 5

Production ready: **false**.

## Findings

### High: retry/proxy rematerialization failures bypass `blocked.stack`, `finalize.stack`, and `err`

The review-4 blockers are mostly addressed for the common composition-preflight, provider-failure, and loop-control paths, but the harness loop still has early blocked exits that use the legacy top-level-only guard API after the lifecycle has already started.

Evidence:

- `materialize_harness_prompt(...)` errors are mapped through `lifecycle_guard.emit_blocked_or_err(e)` at `claudine/cli/src/commands/wrap/harness_orch/loop_control.rs:1027-1037`.
- Target lifecycle parse errors after a proxy hand-off also return through `lifecycle_guard.emit_blocked_or_err(e.into())` at `claudine/cli/src/commands/wrap/harness_orch/loop_control.rs:1048-1054`.
- Harness-plan parse errors in that same per-attempt path use `lifecycle_guard.emit_blocked_or_err(e)` at `claudine/cli/src/commands/wrap/harness_orch/loop_control.rs:1119-1132`.
- `emit_blocked_or_err` delegates to `emit_blocked_or_failure`, which calls `emit_terminal`, and `emit_terminal` only calls `emit_signal` (`claudine/lib/src/composition/lifecycle.rs:421-449`). `emit_signal` renders only the legacy top-level subset (`stderr`, `message`, `notify`, audio) and never runs the typed stack or `finalize` (`claudine/lib/src/composition/lifecycle.rs:655-700`).

These are not purely theoretical setup errors. They are reachable after a lifecycle control action re-enters the harness loop with a changed source: for example, `failure.stack` or `blocked.stack` can `proxy(...)` to a target, the target then enters the harness loop, and a read/compose/frontmatter materialization or target lifecycle parse error occurs before provider launch. The spec requires a blocked pre-provider run to route through `blocked` and then `finalize`, with `err` available to the error-bearing stacks. On these paths the user-authored stacks are skipped, `finalize` does not run, and `err.msg`/`err.kind` are unavailable.

Verification level: current L2 coverage now proves `err` for normal provider failure, preflight blocked, and finalize-after-error paths, and proves proxy success/error flows. It does not cover rematerialization or target lifecycle parse failures after retry/proxy re-entry. This needs at least L1 coverage for the helper/control path and Level 2 coverage proving a real `claudine compose` proxy/retry failure emits `blocked.stack` and `finalize.stack` with an `err` payload.

## Notes

Review-4's main issues are materially improved:

- L2 tests now cover `failure.stack` observing `err` on provider failure, `blocked.stack` observing `err` on preflight failure, and `finalize.stack` observing `err` after failed terminal outcomes.
- Loop-mode `initialize` controls are now handled, with L1 and L2 coverage for `skip`, `error`, and proxy hand-off behavior.
- Loop-gate explicit `error(...)` now fails and exits the loop, with L1 and L2 coverage.

Focused verification run:

```text
cargo nextest run --manifest-path claudine/cli/Cargo.toml -E 'test(/lifecycle/) | test(/loop_initialize/) | test(/loop_gate/)' --color=never
30 tests run: 30 passed, 1764 skipped
```

The remaining finding is an edge path, but it is still part of the public lifecycle semantics for blocked pre-provider execution after runtime control-flow re-entry. I would not mark this production-ready until those legacy `emit_blocked_or_err` exits are routed through the same stack-aware blocked/finalize machinery as the other preflight failures.
