---
ready: true
agent: codex
model: ""
created: 2026-06-18T22:13:25
---

# Review: Expression Engine Errors Must Not Leak Into Lifecycle Messages

## Findings

No blocking findings.

The implementation now covers the gaps from the previous reviews:

- The bundled `prompts/implement-suggestions.md` lifecycle strings no longer contain `parent_dir(review))`, `interation`, `implemention`, or a bare `{{area}}` reference in the touched block.
- `prepare_direct` and `prepare_inline` parse composed lifecycle metadata and reject surviving interpolation spans before a `LifecycleRunGuard` can dispatch any side effect.
- The raw-frontmatter undefined-variable guard now walks the parsed expression tree, so `{{ parent_dir(missing_review) }}` fails the same way as a top-level `{{ missing_review }}`.
- The guard intentionally allows fallback and ternary expressions, matching the documented lifecycle contract for optional values.

## Verification Levels

All user-observable behavior in this feature is preparation-time command behavior: malformed lifecycle expressions must fail before provider execution and before external side effects. It does not require real terminal glyph/style capture or OS keyboard input, so Level 1 is the appropriate verification level.

Verified at Level 1:

- Malformed lifecycle interpolation fails preparation with `LifecycleInterpolationLeak`.
- Undefined lifecycle variables fail preparation with `LifecycleUndefinedVariable`.
- Undefined variables inside function-call arguments fail preparation.
- The real `prompts/implement-suggestions.md` fixture composes without unresolved `{{` / `}}` in lifecycle metadata.
- A CLI `compose --dry-run` smoke test with `start.message: "{{ parent_dir(review)) }}"` exited `1`, reported `start.message` and `parent_dir(review))`, and did not launch the provider stub.

No Level 2 or Level 3 tests are required by the review rubric for this feature because there is no terminal-rendering fidelity requirement and no keyboard-input behavior.

## Production Readiness

Ready for production. The implemented boundary guard prevents raw expression syntax and silently-collapsed undefined lifecycle variables from reaching lifecycle side effects, and the focused Level 1 verification matches the required behavior.
