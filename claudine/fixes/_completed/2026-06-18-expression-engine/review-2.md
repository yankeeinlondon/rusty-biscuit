---
ready: false
agent: codex
model: ""
created: 2026-06-18T22:03:12
---

# Review: Expression Engine Errors Must Not Leak Into Lifecycle Messages

## Findings

### High: undefined variables inside lifecycle expressions still collapse to empty strings

The second-pass undefined-variable guard only rejects spans whose whole expression parses as `Expr::Variable` (`claudine/lib/src/composition/lifecycle.rs:747-749`). That covers `{{ missing }}`, but skips every expression that contains a bare undefined variable inside a larger expression, including function calls. Those values are still lifecycle side effects under the spec.

I verified this with a temporary prompt:

```yaml
---
start:
  message: "before {{ parent_dir(missing_review) }} after"
---
Body
```

`cargo run -q -p claudine-cli --bin claudine -- compose <file> --dry-run` exited `0` and rendered resolved frontmatter as:

```yaml
start:
  message: before  after
```

This violates the spec's undefined lifecycle variable contract. The original broken prompt used `parent_dir(review)`, so missing variables in function arguments are not an edge case for this feature; they are part of the lifecycle expression shape being fixed.

Required fix: inspect the parsed expression tree for bare variable references, not only expressions that are exactly `Expr::Variable`. The guard should reject undefined frontmatter-scoped variables in function arguments and other expression nodes unless the expression intentionally provides fallback semantics (`||`, ternary, or another explicitly tolerated form). Add L1 regression coverage for `start.message: "{{ parent_dir(missing_review) }}"` asserting preparation fails before lifecycle dispatch.

Verification level: no current test covers this requirement. L1 is sufficient because this is prepare-time expression validation with no dependency on a real terminal or OS keyboard encoder.

## Covered Requirements

- The bundled `prompts/implement-suggestions.md` typo is fixed; it no longer contains `parent_dir(review))`, `interation`, `implemention`, or bare `{{area}}` in the touched lifecycle block.
- Malformed lifecycle parse errors now fail preparation through `LifecycleInterpolationLeak`.
- Pure undefined lifecycle variables such as `{{ missing_lifecycle_var }}` now fail preparation through `LifecycleUndefinedVariable`.
- `prepare_direct` and `prepare_inline` both run the lifecycle leak guard and undefined-variable guard before returning `PreparedComposition`.

## Test Rigor Notes

- Focused L1 verification passed:
  `just -f claudine/justfile test-library composition::prepare::tests::undefined_lifecycle_variable_fails_preparation composition::prepare::tests::lifecycle_fallback_for_undefined_variable_passes_preparation composition::prepare::tests::implement_suggestions_prompt_composes_without_lifecycle_leak`
- Raw interpolation leaks and undefined lifecycle variables are prepare-layer behavior, so L1 is the appropriate verification level. No L2/L3 coverage is required unless the feature grows terminal-rendering or keyboard-input requirements.

## Production Readiness

Not ready. The direct raw-template leak is fixed, and the first review's pure-variable gap is partially addressed, but lifecycle expressions can still silently dispatch degraded messages when an undefined variable appears inside a function call.
