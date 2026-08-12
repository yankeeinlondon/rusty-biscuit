---
ready: false
agent: codex/default
created: 2026-06-22T08:04:53
implemented: true
---

# Review 1

Production ready: **false**.

## Findings

### Critical: `success` / `blocked` stack actions execute twice when no `Error` control is present

`execute_terminal_event` previews the `success` or `blocked` stack with `run_lifecycle_stack_only` before committing the terminal event, then calls `run_lifecycle_event` again for the same signal when the preview did not return an explicit `Error`.

- Evidence: `execute_terminal_event` previews at `claudine/cli/src/commands/wrap/harness_orch/loop_control.rs:42-52`, then executes the same signal at `:66-75`.
- `run_lifecycle_stack_only` is not a dry-run; it calls `ctx.execute_stack_for_signal(guard.config())` at `:128-140`.
- `execute_stack` performs real communication, shell, side-effect, and expression-function actions at `claudine/lib/src/composition/lifecycle_executor.rs:340-454`.

This violates the ordered stack contract. A prompt with:

```yaml
success:
  stack:
    - action: "append_line('@events.log', 'done')"
```

will append twice. A `success.stack` `message(...)`, `notify(...)`, `shell(...)`, or `http_post(...)` likewise fires twice. This is a user-visible and externally mutating regression.

The runtime should decide explicit `Error` transitions without executing side effects twice. Options include recording the terminal signal only after running the stack once, adding a pure classifier for whether a stack can produce terminal `Error`, or handling `StackControl::Error` after a single committed `run_lifecycle_event`.

Verification required: at least L1 for side-effect/action count and L2 for real terminal/status ordering where stderr/info/warn output is involved.

### High: setup stack action errors can bypass lifecycle state recording, preventing `finalize`

When `start` stack processing returns an unintentional action error, the failure path builds a failure context and calls `lifecycle_guard.run_event_stack(LifecycleSignal::Failure, ctx)` directly at `claudine/cli/src/commands/wrap/harness_orch/loop_control.rs:606-624`. That helper does not call `record_event_emission`; the state update only happens in `run_lifecycle_event` at `:95-110`.

The immediate next call tries to emit `finalize` via `run_lifecycle_event(... LifecycleSignal::Finalize ...)` at `:636-642`, but `record_event_emission(Finalize)` is a no-op unless `terminal_emitted` is already true. Because the direct `run_event_stack` call did not mark the failure terminal, `finalize` can be skipped.

This violates the acceptance criterion that `finalize` fires after terminal `failure`, and it means cleanup stacks can silently not run for a `start.stack` setup error.

Verification required: L1 around guard state and an integration path proving `start.stack` failing action emits `failure` then `finalize`.

### High: `Proxy`, `Retry`, `Resume`, and `Requeue` parse but are not wired into runtime control flow

The spec defines runtime behavior for `Proxy`, `Retry`, `Resume`, and `Requeue` and explicitly lists L2 coverage for these control flows. The implementation resolves these controls into `StackControl` variants, but the composition runtime does not implement them.

- `initialize` `Proxy` returns `lifecycle proxy is not yet implemented` at `claudine/cli/src/commands/wrap/composition/mod.rs:1453-1457`.
- `Retry`, `Resume`, and `Requeue` are treated as invalid at initialize at `:1458-1464`; expected there, but repo-wide search found no runtime match arms implementing these variants for `blocked` or `failure`.
- The executor module states it does not perform runtime control flow for `Proxy` / `Retry` / `Resume` / `Requeue`; that wiring is expected elsewhere (`claudine/lib/src/composition/lifecycle_executor.rs:17-20`).

This leaves several accepted lifecycle actions as parse-only. Authors can write valid lifecycle blocks that pass parse-time validation but fail or do nothing at runtime.

Verification required: L1 for each control transition and L2 for the user-observable blocked/failure recovery paths specified in the test strategy.

### High: stack-only `timing` and `current` globals are modeled but never populated

The spec says lifecycle stack expressions gain access to late-bound `timing` and `current` globals. The lookup supports them only when a snapshot is attached (`claudine/lib/src/composition/lifecycle_context.rs:290-310`), but every production context I found passes `timing: None` and `current: None`:

- Direct composition initialize context: `claudine/cli/src/commands/wrap/composition/mod.rs:1408-1423`
- Harness lifecycle context: `claudine/cli/src/commands/wrap/harness_orch/loop_control.rs:165-169`
- Loop lifecycle context: `claudine/lib/src/composition/loop_engine.rs:837-854`

As a result, `current.env.FOO`, `current.ctx.agent`, and `timing.total_ms` do not provide the promised late-bound state; they fall through as ordinary frontmatter lookups and evaluate missing unless the author happened to define those keys in frontmatter.

Verification required: L1 for lookup population and at least one integration test showing a stack `when:` clause reacts to an environment value changed after prepare.

### High: required L2 lifecycle coverage is missing

The feature's test strategy requires L2 verification for end-to-end lifecycle dispatch ordering, loop gate ordering, control-flow actions, and the blocked-first-iteration edge case. I found many L1 parser/executor tests under `composition/lifecycle*.rs`, but no `level2_*` test dedicated to this lifecycle feature. Existing `level2_*` tests cover unrelated surfaces such as perf, prompt reporting, dry-run, schema prompts, context rendering, and Ctrl+C.

Because lifecycle output and stack behavior are user-observable in the terminal and can mutate external state, L1-only parser/executor coverage is not sufficient to mark the feature production-ready under the requested rigor model.

Concrete missing requirement mappings:

- All seven event dispatch order: strongest observed coverage is L1; required L2.
- Loop gate concerns-before-condition-before-mutation and per-iteration `finalize` count: strongest observed coverage is L1; required L2.
- `Proxy` / `Retry` / `Resume` / `Requeue` control flow: no meaningful runtime coverage; required L2.
- Blocked-first-iteration `blocked` -> `finalize` behavior: strongest observed coverage is L1 or absent for runtime edge paths; required L2.

## Notes

The parser/model layer is substantially present: all seven events exist, `say_first` conflict and `stdout` rejection are typed, lifecycle action placement/cardinality is validated, and the `err` scan has a clear implementation path. The blocking issues are in runtime semantics and verification depth, not in the basic data model.
