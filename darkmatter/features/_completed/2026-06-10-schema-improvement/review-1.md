---
agent: codex
model: ""
ready: false
---

# Review: Inline Nested Object Schemas in SimplifiedSchema

## Findings

### High: inline-object coercion stops entirely when any sibling field is outside the coercion matrix

The spec says compose-time coercion recurses into inline object fields and inline object arrays whenever the matching schema path is unambiguous. A path such as `/config/enabled` is still unambiguous when a sibling field is an opaque `object`, `enum`, `any`, or another non-coercible schema.

The implementation instead makes `inline_object_target` all-or-nothing: it returns `None` as soon as any child property has no coercion target, which disables coercion for every other child in that inline object. See `darkmatter/lib/src/markdown/schemas/coerce.rs:110`, especially the `?` at `darkmatter/lib/src/markdown/schemas/coerce.rs:120`. The current test `inline_object_with_unrecognised_property_is_not_recognised` explicitly locks this in at `darkmatter/lib/src/markdown/schemas/coerce.rs:1053`.

This breaks valid schemas like:

```yaml
$schema:
  config: "{ enabled: boolean, metadata: object }"
config:
  enabled: "true"
  metadata: { source: user }
```

Expected: `enabled` is coerced to `true`, while `metadata` is left alone.
Actual: the whole inline object is outside the coercion path, so `enabled` stays `"true"` and later validation fails against `boolean`.

It also weakens property-level union coercion for inline object arms with discriminators or opaque siblings, because `coerce_property_union` only gets an object candidate if `coercion_target(arm)` succeeds (`darkmatter/lib/src/markdown/schemas/coerce.rs:344`).

Suggested fix: build `CoercionTarget::Object` from the recognized child properties only, return `None` only when no child has a target, and leave unrecognized siblings untouched in `coerce_inline_object`.

Verification level: Level 1 is appropriate. Add unit/integration coverage for mixed inline objects, inline object arrays with mixed fields, and a property-level union arm where one nested field coerces while a discriminator/opaque sibling remains unchanged.

### High: the parser accepts inline-object item constraints before `[]`, which the spec forbids

Decision #10 and the descriptor catalog both state that constraints before `[]` are not valid for inline object arrays; array constraints must appear after `[]`, e.g. `{ name: string }[](min(1); required)`. The catalog says this directly at `darkmatter/lib/src/markdown/schemas/about.rs:333`.

However, `parse_postfix_after_type` parses an optional parenthesized constraint list first and then still accepts a following `[]` (`darkmatter/lib/src/markdown/schemas/simplified/grammar.rs:453` through `darkmatter/lib/src/markdown/schemas/simplified/grammar.rs:479`). That means `{ host: string }(required)[]` is accepted even though the feature says it must be a grammar error.

This is not just syntax tolerance: `atom_to_schema` scans both `atom.constraints` and `atom.array_constraints` for `required`/`default`, so a forbidden pre-array `required` can still affect the containing property.

Suggested fix: when parsing an inline object postfix, reject `[]` if `item_constraints` is non-empty, or split primitive and inline-object postfix parsing so inline arrays only accept `[]` followed by array constraints.

Verification level: Level 1 is appropriate. Add parser tests that reject `{ foo: string }(required)[]`, `{ foo: string }(default({}))[]`, and any non-universal pre-array constraint before conversion.

## Test Notes

No Level 2 or Level 3 coverage is required for this feature. The user-observable behavior is schema parsing, conversion, validation, CLI text output, and compose-time frontmatter coercion; Level 1 unit/integration tests are the right verification level.

Focused checks run:

- `cargo test --color=never -p darkmatter schemas::coerce --lib` passed: 46 tests.
- `cargo test --color=never -p darkmatter about --lib` passed: 7 tests.

Those checks do not clear the findings above because the current suite either lacks the forbidden-inline-array regression or explicitly asserts the all-or-nothing inline-object coercion behavior.

## Production Readiness

Not ready for production. The core inline-object validation/conversion surface is substantially covered, but compose-time coercion does not meet the spec for mixed inline object shapes, and the grammar accepts a form the spec and descriptor catalog declare invalid.
