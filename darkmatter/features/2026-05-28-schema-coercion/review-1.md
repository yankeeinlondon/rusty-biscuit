---
ready: false
agent: codex
model: ""
---

# Review 1

## Findings

### High: unrelated `anyOf` schemas can be coerced into false acceptances

- Location: `darkmatter/lib/src/markdown/schemas/coerce.rs:89`
- Requirement: coercion must only add acceptances for the explicit, unambiguous matrix: strict scalar types plus Darkmatter's boolish/numberlike shapes. Anything outside the matrix must be left untouched and reported by the existing strict validator.
- Current behavior: `target_from_any_of` recognizes boolish whenever one `anyOf` arm has `type: boolean` and any other arm has an `enum` key, without checking that the enum is the boolish set. It recognizes numberlike whenever one arm has `type: number` and another string arm has any `pattern`, without checking that the pattern is the shared `^-?\d+(\.\d+)?$` numberlike pattern.
- Impact: raw JSON Schema property unions outside the feature matrix can now pass when they should fail. For example, `{"anyOf":[{"type":"boolean"},{"enum":["auto"]}]}` with instance `"true"` is not valid JSON Schema input, but this implementation coerces it to `true` and validation succeeds. Likewise, `{"anyOf":[{"type":"number"},{"type":"string","pattern":"^[A-Z]+$"}]}` with `"42"` can be coerced to `42` and accepted even though the string arm intentionally rejects digits.
- Suggested fix: make the recognizer exact. For boolish, require the enum values to match `BOOLISH_VALUES` (or at least be a subset/equivalent shape that only represents the boolish spellings). For numberlike, require the string arm pattern to equal `NUMBERLIKE_PATTERN`. Add regression tests proving unrelated boolean/enum and number/string-pattern unions remain invalid.

### Medium: oversized integral numeric strings violate the documented coercion matrix

- Location: `darkmatter/lib/src/markdown/schemas/coerce.rs:220`
- Requirement: a string matching `^-?\d+(\.\d+)?$` against `number` or `integer` is coerced to a real number; `"42"` validates as integer, while `"3.14"` coerces then fails the integer check normally.
- Current behavior: integral strings are parsed only as `i64`; values larger than `i64::MAX` are deliberately left as strings and then fail with a type error. The unit test at `coerce.rs:445` codifies that behavior, but the spec does not define an overflow exception.
- Impact: a schema-declared `number` field containing a valid JSON numeric literal such as `"9223372036854775808"` fails type validation instead of being normalized. This is a narrow edge case, but it is a spec/implementation mismatch.
- Suggested fix: either support larger JSON numbers consistently with the validator's numeric model, or update the spec/design to state the exact supported numeric range and why out-of-range integral strings remain uncoerced.

### Medium: user-facing compose write-back lacks CLI integration coverage

- Location: `darkmatter/lib/src/markdown/compose/schema_validation.rs:123`
- Requirement: the success criteria require stored frontmatter values to be real booleans/numbers/strings in the composed document, not merely accepted by validation.
- Current behavior: write-back is covered by Level 1 in-process tests against `schema_validation::run`, but I did not find an `md compose --frontmatter` integration test that verifies the actual CLI output serializes coerced values after the full compose pipeline.
- Impact: a future pipeline-ordering or CLI serialization regression could keep the library unit tests green while user-visible composed frontmatter regresses.
- Suggested fix: add Level 1 CLI tests for `md compose --frontmatter` covering boolean, number, string reverse-coercion, boolish/numberlike normalization, typed arrays, and root-union write-back. No Level 2/Level 3 harness is required because this is data-flow/serialization behavior, not terminal emulator rendering or keyboard input.

## Verification Level Review

- Schema validation acceptance/rejection and library validation parity: strongest present coverage is Level 1 unit tests in `schemas::coerce` and `schemas::mod`. Level 1 is appropriate because this is pure data validation.
- Compose write-back before downstream stages: strongest present coverage is Level 1 in-process tests in `compose::schema_validation`. Level 1 is appropriate, but CLI serialization coverage is incomplete as noted above.
- `md schema validate` and `md compose` parity: existing Level 1 CLI integration coverage is present for general schema parity. This feature needs additional coercion-specific CLI cases.
- Terminal verification: no Level 2 or Level 3 coverage is required for the core feature. The spec does not define terminal rendering, modifier-key, hotkey, paste, IME, mouse, or OS keyboard injection behavior.

## Local Verification

- Attempted: `cargo test --color=never -p darkmatter coerce`
- Result: not completed. The command first waited on Cargo's artifact-directory lock, then moved into a broad rebuild and exceeded the non-interactive review time budget, so I terminated that specific Cargo process.

## Production Readiness

Not ready for production. The broad `anyOf` recognizer can incorrectly accept raw JSON Schema values outside the coercion matrix, which violates the feature's correctness boundary.
