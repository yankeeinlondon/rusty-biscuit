---
ready: false
agent: codex
model: ""
---

# Review 2

## Findings

### High: boolish subset `anyOf` schemas can still be coerced into false acceptances

- Location: `darkmatter/lib/src/markdown/schemas/coerce.rs:117`
- Requirement: coercion is allowed only for the explicit matrix and for the boolish/numberlike shapes Darkmatter emits. Values outside that boundary must be left untouched for strict JSON Schema validation.
- Current behavior: iteration 2 tightened the recognizer for non-boolish enum members, but `is_boolish_enum` now accepts any non-empty subset of `BOOLISH_VALUES`. That is broader than the design's "matches exactly the shapes it emits" contract and broader than the module comment at `coerce.rs:88`.
- Impact: a raw JSON Schema like `{"anyOf":[{"type":"boolean"},{"enum":["true"]}]}` is not Darkmatter's boolish shape. With instance `"false"`, strict validation should fail because the value is a string and the enum only allows `"true"`. The current recognizer classifies the property as `ToBoolean`, coerces `"false"` to `false`, and validation then succeeds against the boolean arm. This is another false acceptance outside the feature matrix.
- Suggested fix: require the enum arm to match the full emitted boolish set, order-independent, with no missing or extra members. Add a regression test for subset enums such as `["true"]` and `["true","false"]` if the design keeps the full six-spelling shape as the only recognized boolish fragment. Also update the comments so they no longer disagree with the code.

### High: root-union compose coercion is blocked by shell-pending fields in the same arm

- Location: `darkmatter/lib/src/markdown/compose/schema_validation.rs:102` and `darkmatter/lib/src/markdown/schemas/coerce.rs:145`
- Requirement: values still holding `$(...)` must be skipped at the pre-shell stage, not coerced, not written back, and not errored. Non-shell values in the same frontmatter should still be coerced and written back so downstream conditions see real booleans/numbers.
- Current behavior: compose records the shell-pending keys, but still inserts those values into the instance passed to `coerce_frontmatter`. For root unions, `coerce_frontmatter` commits an arm only if the full candidate validates. A shell-pending typed property such as `n: "$(echo 1)"` keeps the candidate invalid, so no arm is committed and non-shell fields in that arm are left as strings. The later validation pass then reports the non-shell field as a composition-independent type problem.
- Impact: a root-union document with one arm declaring `n: number` and `flag: boolean`, plus frontmatter `n: "$(echo 1)"` and `flag: "false"`, should defer `n` and write `flag: false` before shell expansion. Instead, the unresolved `n` prevents the arm from validating, `flag` remains the string `"false"`, and compose can fail before shell expansion on `/flag`.
- Suggested fix: make the compose write-back path honor the pending-key set before root-union arm success is decided. One workable approach is to select/accept an arm when all remaining validation problems are attributable to shell-pending keys, then write back only non-pending coerced fields. Add a Level 1 compose test covering a root union with both a shell-pending typed field and a non-shell bool/number field.

## Verification Level Review

- Schema recognizer and scalar coercion: strongest coverage is Level 1 unit tests in `schemas::coerce`. Level 1 is appropriate for pure JSON data behavior, but the boolish subset false-acceptance case is not covered.
- Compose write-back and serialized `md compose --frontmatter` output: strongest coverage is Level 1 in-process tests plus Level 1 CLI integration tests. Level 1 is appropriate because this is data-flow and serialization behavior.
- Shell deferral: strongest coverage is Level 1 in-process compose tests for non-union shell-pending values. The root-union mixed pending/non-pending case above is not covered.
- `md schema validate` and `md compose` parity: strongest coverage is Level 1 CLI integration. Level 1 is appropriate.
- No Level 2 or Level 3 verification is required for this feature. The spec does not define terminal rendering, hotkey, modifier-key, paste, IME, mouse, scrolling, or OS keyboard-injection behavior.

## Local Verification

- Attempted: `cargo test --color=never -p darkmatter unrelated_boolean_enum_union_is_none`
- Result: not completed. The command was still compiling dependencies after 60 seconds in the non-interactive session, so I terminated the Cargo process.

## Production Readiness

Not ready for production. The boolish recognizer can still accept raw JSON Schema values outside the coercion matrix, and root-union compose coercion can fail the shell-deferral contract when pending and non-pending typed fields appear in the same arm.
