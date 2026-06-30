---
ready: false
agent: codex/default
created: 2026-06-25T02:53:37
implemented: true
---

# Review 1

## Findings

### High: Fallback syntax for intentionally optional lifecycle values is rejected in strict DM2

The spec explicitly preserves fallback tolerance for optional lifecycle values and documents `{{ maybe || '' }}` as the migration path for unknown optional names (`spec.md:213-217`, `spec.md:228-236`). The implementation's strict pre-pass rejects every unknown root before evaluation:

- `darkmatter/lib/src/markdown/compose/subtree.rs:478` parses every span and checks all collected roots.
- `darkmatter/lib/src/markdown/compose/subtree.rs:523` descends into both sides of `Expr::Fallback`.

That means `message: "{{ missing_lifecycle_var || 'default' }}"` fails before the fallback branch can run, even though `darkmatter::compose::expression::evaluate` implements fallback short-circuiting at `expression/mod.rs:400-406`. There is a prepare-only test proving the span is deferred (`claudine/lib/src/composition/prepare.rs:895`), but no event-time test asserting it actually resolves. This is a functional mismatch, not just a test gap.

Suggested fix: make strict root validation follow expression evaluation semantics for fallback and ternary operands, or move unknown-root detection into an evaluator mode that can distinguish "actually evaluated unknown" from "guarded optional unknown". Add Level 1 DM2 and Claudine executor tests for `{{ missing || 'default' }}` and a ternary with an unchosen unknown branch.

Verification level: strongest present is Level 1 prepare-only, which does not verify the user-facing event-time behavior. Level 1 executor coverage would be appropriate here because this is expression semantics, not terminal encoder/renderer behavior.

### High: `when:` unknown roots do not fail closed

Acceptance criterion 11 requires event-time failures, including unknown roots and malformed/illegal expressions, to fail closed before lifecycle side effects dispatch (`spec.md:380-383`). The guard rework also names `when:` as a lifecycle expression surface (`spec.md:209-217`). The current `when_matches` path does not apply strict DM2/root validation:

- `claudine/lib/src/composition/lifecycle_executor.rs:540` calls `eval_expr`.
- `darkmatter/lib/src/markdown/compose/expression/mod.rs:361` evaluates an unknown variable as `Value::Null`, not an error.
- `claudine/lib/src/composition/lifecycle_executor.rs:545` treats that null as false and silently skips the stack item.

So a typo such as `when: "spec_fil"` silently disables the action instead of failing closed. This can hide lifecycle recovery, messaging, or file-mutating actions and is the same class of operational silence the feature is meant to remove.

Suggested fix: add a strict validation/evaluation mode for lifecycle expression surfaces (`when:`, control operands, multi-argument action expressions) that rejects unknown roots when they are not protected by the documented fallback/ternary tolerance. Change `when_matches` to return a result so a malformed/unknown guard produces an action error instead of `false`.

Verification level: no focused Level 1 test was found for unknown-root `when:` fail-closed behavior. Level 1 executor coverage is appropriate; add one test proving a typo in `when:` produces `action_error` and no side effect, plus one proving guarded optional fallback remains allowed.

## Requirement Coverage Notes

- DM1 exclude keys, DM1a deferred-key cross-reference rejection, DM1b schema exclusion, DM2 eager/lazy globals, strict known-empty handling, top-level/stack `err` interpolation, just-in-time frontmatter mutation, shell early-binding, deferred effect validation, and post-DM2 leak checks all have relevant Level 1 coverage.
- Existing Level 2 lifecycle tests cover several real-terminal lifecycle flows, but the two findings above are expression/evaluation semantics. They do not need Level 2 or Level 3 to be production-ready; they need correct Level 1 executor/DM2 tests.
- No Level 3 coverage is required for this feature because the spec does not define keyboard-input behavior.

## Production Readiness

Not production ready. The implementation covers much of the late-binding design, but the strictness layer currently breaks the documented fallback escape hatch and allows `when:` typos to fail open by silently skipping actions.

I did not run the test suite during this review; findings are based on source inspection against the spec.
