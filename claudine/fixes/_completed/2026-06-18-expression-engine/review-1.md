---
ready: false
agent: codex
model: ""
created: 2026-06-18T21:39:37
---

# Review: Expression Engine Errors Must Not Leak Into Lifecycle Messages

## Findings

### High: undefined lifecycle variables still collapse to empty strings and can be dispatched

The implementation only rejects lifecycle strings when a raw interpolation span survives composition. `prepare_direct` and `prepare_inline` parse the rendered lifecycle config and then call `validate_no_interpolation_leaks` (`claudine/lib/src/composition/prepare.rs:165`, `claudine/lib/src/composition/prepare.rs:259`). That guard only scans the already-rendered string with `ExpressionFinder::find_all_plain` and errors on remaining spans (`claudine/lib/src/composition/lifecycle.rs:652`).

This does not implement the spec's undefined-variable contract:

> If a lifecycle message references an undefined bare variable, Claudine should warn or fail before dispatching the side effect.

I verified the gap with a temporary prompt:

```yaml
---
title: Test
start:
  message: "before {{ missing_lifecycle_var }} after"
---
Body
```

`cargo run -q -p claudine-cli --bin claudine -- compose <file> --dry-run` exited `0` and rendered resolved frontmatter as `message: before  after`, with no warning or preparation failure. In a non-dry run this message would be eligible for lifecycle dispatch with the missing variable silently removed.

Required fix: lifecycle/frontmatter interpolation needs access to Darkmatter diagnostics for unresolved bare variables, or Claudine needs to fail lifecycle strings when composition warnings identify unresolved variables in lifecycle keys. Add regression coverage for `start.message: "before {{ missing_lifecycle_var }} after"` asserting a non-zero preparation error or an explicit warning before any lifecycle side effect can dispatch.

Verification level: currently no valid level covers this requirement. The existing L1 guard tests cover malformed syntax that leaves delimiters, not resolved-empty undefined variables. L1 is sufficient for this requirement if it exercises `prepare_*` plus a fake lifecycle/messenger boundary.

### Medium: the "zero sends" regression test does not exercise dispatch

`guard_prevents_message_dispatch_on_leak` constructs a `RecordingEmitter`, calls `validate_no_interpolation_leaks`, and then asserts the emitter is empty (`claudine/lib/src/composition/lifecycle.rs:1528`). The emitter is never wired into a `LifecycleRunGuard` path after the validation error, so the assertion is tautological. It proves the helper returns an error, but not that the composition execution path prevents message/TTS/effect dispatch.

The prepare-layer wiring likely prevents real sends for malformed expressions because validation happens before `PreparedComposition` is returned, but the test named for dispatch should either exercise the actual lifecycle runner boundary or be replaced with a higher-value prepare/execution test using the existing fake emitter.

Verification level: L1 is appropriate; the current L1 test is too shallow for the user-observable "no external side effects" requirement.

## Covered Requirements

- The bundled prompt typo is fixed in `prompts/implement-suggestions.md`; it no longer contains `parent_dir(review))`, `interation`, `implemention`, or bare `{{area}}`.
- Malformed lifecycle parse errors now fail preparation before provider execution. A dry-run prompt with `{{ parent_dir(review)) }}` exits non-zero and reports `start.message`, the expression, and the parser reason.
- `prepare_direct` and `prepare_inline` both run the post-compose lifecycle leak guard.
- The rendered error uses the existing `CompositionError` status block path rather than ad hoc ANSI.

## Test Rigor Notes

- Raw malformed lifecycle interpolation: strongest coverage is L1 unit/CLI-style dry-run. That is appropriate because the behavior is pure preparation and does not depend on terminal encoder/decoder behavior.
- Lifecycle side-effect suppression: needs stronger L1 coverage at the fake emitter boundary. No L2/L3 is required because this is not terminal rendering or keyboard input behavior.
- Prompt fixture regression: L1 real-file preparation coverage is appropriate.

## Production Readiness

Not ready. The main raw-template leak is fixed, but the spec explicitly requires undefined lifecycle variables to warn or fail before dispatch, and they currently resolve to empty strings successfully.
