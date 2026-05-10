---
ready: false
agent: codex
model: ""
---

# Review 1 - Expression Syntax

## Verdict

Not ready for production.

The implementation covers most of the parser/evaluator shape from the spec, but there are observable semantic gaps in numeric coercion and bracket access, plus test coverage gaps for user-facing date helper dispatch and arithmetic error reporting.

## Findings

### High - Booleans are accepted as numeric operands

Spec/design requirement: arithmetic operators must reject non-numeric operands; the spec explicitly includes booleans in the invalid set for arithmetic type errors, and math helpers must error on type mismatches.

Implementation:

- `darkmatter/lib/src/markdown/compose/expression/mod.rs:198`-`202` converts `Value::Bool(true)` to `1.0` and `false` to `0.0`.
- `evaluate_binary` then uses that conversion for `+`, `-`, `*`, `/`, `%` at `darkmatter/lib/src/markdown/compose/expression/mod.rs:377`-`378`.
- `darkmatter/lib/src/markdown/compose/expression/functions.rs:44`-`50` also accepts booleans for `min`, `max`, and `abs`.

Impact: `{{ true + 1 }}` evaluates to `2`, `{{ false * 5 }}` evaluates to `0`, `items[true]` indexes as `items[1]`, and `min(true, 5)` evaluates as `1`. Those results contradict the spec's type-mismatch contract.

Verification level: wrong Level 1 coverage. Unit tests cover string/array/object mismatches, but there is no Level 1 unit or compose test asserting that booleans are rejected for arithmetic/math helper domains.

Recommendation: split numeric coercion into context-specific helpers. Comparisons and legacy `number()` may keep boolean coercion if desired, but arithmetic operators, array indexes, and `min`/`max`/`abs` should require JSON numbers, plus numeric strings only if that is explicitly intended for arithmetic. Add tests for boolean operands in every arithmetic operator and math helper.

### High - Object bracket access accepts non-string keys

Design requirement: invalid bracket access returns `null`, including "non-string key on object" (`design-notes.md:148`-`149`). The spec describes object access as `foo["key"]` string keys.

Implementation: `darkmatter/lib/src/markdown/compose/expression/mod.rs:420`-`425` converts any non-null object index through `scalar_string`, so `obj[0]`, `obj[true]`, and `obj[1.5]` look up `"0"`, `"true"`, and `"1.5"` instead of returning `null`.

Impact: documents can accidentally depend on unsupported object indexing forms. This is also inconsistent with the stricter numeric-dot-access rejection.

Verification level: missing Level 1 coverage. There are positive tests for `config["key"]`, but no unit/integration test for invalid object index expressions returning `null`.

Recommendation: for `Value::Object`, return a key only for `Value::String`; return `null` for all other index value types. Add tests for `obj[0]`, `obj[true]`, `obj[null]`, and missing string keys.

### High - Date helper user-facing dispatch is under-tested

Spec requirement: all date/date-time validators and UTC variants are part of the expression language: `IsDateUtc`, `IsDateTimeUtc`, `IsToday`, `IsYesterday`, `IsTomorrow`, `IsThisMonth`, `IsThisYear`, and all `*Utc` relative variants.

Current tests:

- Direct helper tests cover pure date functions in `functions.rs`.
- The integration regression suite only exercises `IsDate(...)` and `IsDateTime(...)` through document interpolation (`expression_regression.rs:209`-`222`).

Gap: the user-facing expression path is parser -> evaluator -> function dispatch -> compose/condition behavior. Most date helper names and UTC variants are not exercised through that path, so a dispatch typo or mode-specific issue would pass.

Verification level: insufficient Level 1 for user-facing expression requirements. Pure helper unit tests are useful, but they do not verify expression names are reachable from interpolation, `when=`, and `evaluate_condition_against`.

Recommendation: add Level 1 tests that evaluate every date helper through `expression::parse`/`evaluate`, interpolation compose, and at least representative `when=`/`evaluate_condition_against` conditions. Keep pure frozen-date helper tests for deterministic date math.

### Medium - Arithmetic error reporting tests are weak

Spec/design requirement: division/remainder by zero and non-numeric arithmetic produce clear evaluator errors.

Current regression coverage: `expression_regression.rs:352`-`363` composes `numerator / denominator` with zero but only asserts the output is not `inf`. It does not assert the warning/error message, fail-fast behavior, or that the original expression remains intentionally unreplaced when fail-fast is disabled.

Verification level: incomplete Level 1. Evaluator unit tests check division/remainder-by-zero errors, but the user-facing compose path does not verify the reported warning/error contract.

Recommendation: add compose tests for default warning behavior and `ComposeOptions::with_fail_fast(true)` behavior for division by zero, remainder by zero, and boolean/object/array arithmetic type mismatches.

## Coverage Notes

Level 2 and Level 3 terminal testing are not required for this feature because the reviewed requirements are expression parser/evaluator semantics, not terminal emulator rendering or OS keyboard input. The appropriate floor is Level 1 in-process tests through the public expression and compose surfaces.

Implemented areas with reasonable Level 1 coverage include parser precedence/associativity, `<=`, positive arithmetic, positive bracket access, string helpers, collection helpers, type predicates, and page-block conditions. The gaps above block production readiness because they change or fail to verify user-observable expression behavior.
