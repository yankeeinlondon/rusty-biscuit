---
ready: false
agent: codex/default
created: 2026-06-25T04:41:52
implemented: true
---

# Review 2

## Findings

### High: Frontmatter mutations from one lifecycle event are not visible to later lifecycle events in the same provider attempt

The spec requires bare frontmatter references in lifecycle text to read "the current effective document state at the moment the event fires", specifically so lifecycle side effects that mutate frontmatter are visible to later event-time interpolation (`spec.md:77-82`, `spec.md:182-190`). The implementation only mirrors document-targeted `set_frontmatter`/`merge_frontmatter`/etc. into a per-stack `working` map:

- `claudine/lib/src/composition/lifecycle_executor.rs:510` seeds `working` from `self.frontmatter` for one stack execution.
- `claudine/lib/src/composition/lifecycle_executor.rs:790` mirrors mutations into that local `working` map.
- `claudine/lib/src/composition/lifecycle_executor.rs:794` documents the scope as "a later action in the same stack".

That fixes the intra-stack case, and the new Level 1 test at `claudine/lib/src/composition/lifecycle_executor.rs:2329` verifies exactly that. But the provider harness builds every lifecycle event context from the same immutable `MaterializedHarnessPrompt.frontmatter`:

- `claudine/cli/src/commands/wrap/harness_orch/prompt.rs:16` seeds materialized frontmatter from `prepared.effective_frontmatter`.
- `claudine/cli/src/commands/wrap/harness_orch/loop_control.rs:420` pulls `fm_map` directly from `materialized.frontmatter`.
- `claudine/cli/src/commands/wrap/harness_orch/loop_control.rs:242-257`, `339-360` reuse that same materialized frontmatter when running later `success`/`failure`/`finalize` events.

So a prompt like this will write `status = "running"` during `start`, but `success.message` and `finalize.message` still interpolate `{{status}}` from the original prepared frontmatter, not the live document state:

```yaml
status: pending
start:
  stack:
    - action: set_frontmatter('prompt.md', 'status', 'running')
success:
  message: "status={{status}}"
finalize:
  message: "final={{status}}"
```

This is the same user-facing binding surface the feature is about: lifecycle output can report stale state after a lifecycle side effect. The gap is especially visible for `start -> success/failure/finalize`, and for terminal-event stacks that mutate a status consumed by `finalize`.

Suggested fix: make the lifecycle runtime carry an owned, mutable effective-frontmatter state for the current attempt, and have document-targeted frontmatter side effects update that shared state, not only a stack-local copy. Alternatively, reload/re-materialize frontmatter before each lifecycle event, but be careful to preserve the raw deferred lifecycle subtree and not accidentally re-compose ordinary prompt body/frontmatter at the wrong time. Add Level 1 harness-orchestration coverage for `start.stack set_frontmatter` followed by `success.message`/`finalize.message`, plus a loop-engine test if loop gate mutations are expected to be visible to subsequent lifecycle concerns.

Verification level: strongest present is Level 1 executor-only coverage for same-stack visibility. The requirement is cross-event lifecycle state semantics, so Level 1 runtime/harness-orchestration coverage is the appropriate minimum. No Level 2 or Level 3 is required because this is not terminal rendering or keyboard input behavior.

## Requirement Coverage Notes

- The two Review 1 findings appear addressed: DM2 strict fallback now has direct Level 1 coverage for `{{ missing || 'default' }}`, and `when:` now fails closed on an unknown unguarded root with Level 1 executor coverage.
- Core late-binding surfaces have Level 1 coverage: top-level `failure.message`, stack `message(...)`, mixed early/late spans, known-empty handling, malformed/unknown-root fail-closed behavior, post-DM2 leak checks, and deferred effect validation.
- I did not find Level 1 coverage for cross-event lifecycle state after a lifecycle frontmatter mutation, and the runtime wiring currently appears to make that behavior stale.
- No Level 3 coverage is required for this feature; the spec does not define OS keyboard input behavior.

## Production Readiness

Not production ready. Most of the late-binding design is implemented, and the prior review blockers look fixed, but the implementation still does not satisfy the spec's "current effective document state at the moment the event fires" contract across lifecycle events in the same run.

I did not run the test suite during this review; findings are based on source inspection against the spec.
