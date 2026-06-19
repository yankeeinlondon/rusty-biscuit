---
ready: true
agent: codex
model: ""
created: "2026-06-19T07:19:47"
---

# Review 2 - Optional Schema Properties

## Findings

### Low - Required enum null rejection is implemented but not durably covered

The spec's acceptance matrix says every typed primitive, including `enum`, should accept `null` when optional and reject `null` when required. The current converter test covers optional enum null acceptance, but then explicitly skips the required enum case with a comment saying the grammar does not accept required enum syntax.

That comment is inaccurate. The grammar supports `enum(draft, published; required)` at [grammar.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/w-schema-props/darkmatter/lib/src/markdown/schemas/simplified/grammar.rs:1464), and `enum_fragment` permits `Constraint::Required` at [convert.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/w-schema-props/darkmatter/lib/src/markdown/schemas/simplified/convert.rs:567). I also confirmed with the CLI that:

```yaml
$schema:
  req: "enum(red,green; required)"
req: null
```

is rejected with `null is not one of "red" or "green"`.

Fix direction: add `enum(red,green; required)` to the required-null regression coverage in [convert.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/w-schema-props/darkmatter/lib/src/markdown/schemas/simplified/convert.rs:1326), and remove the misleading comment.

Verification level: Level 1 is appropriate because this is schema conversion and validation, not terminal rendering or keyboard input. Current Level 1 coverage verifies the behavior manually only through this review run, not as a committed regression test.

## Test Coverage Review

The requirements in this fix are non-terminal behavior: schema conversion, validation, coercion, and Claudine prepare-time composition. Level 1 is the right verification level for the feature. No Level 2 or Level 3 coverage is required because the spec does not assert terminal rendering, keybindings, paste, IME, mouse behavior, scrolling, or OS keyboard-event encoding.

Observed Level 1 coverage is present for:

- Optional primitives, objects, inline objects, arrays, property-level unions, and `file` accepting `null`.
- Optional `file` retaining the empty-string sentinel.
- Required scalar atoms preserving their pre-change JSON Schema shape.
- Required unions with a `file` arm rejecting both `null` and `""`, closing the review 1 production blocker.
- Coercion through nullable wrappers for non-null values, while leaving `null` untouched.
- Claudine direct and inline composition when an optional string template resolves to `null`.

Remaining Level 1 gap:

- Required `enum(...; required)` rejecting `null` should be added as a unit regression test.

## Commands Run

- `cargo nextest run -p darkmatter --lib -E 'test(required_union_with_file_arm_rejects_empty_string) or test(optional_union_property_is_single_level_nullable_any_of) or test(optional_file_accepts_null_and_empty_as_absent) or test(nullable_wrapper_string_yields_to_string) or test(optional_property_level_union_runs_per_arm_coercion)'`
- `cargo nextest run -p claudine --lib -E 'test(optional_string_resolved_to_null_passes_direct) or test(optional_string_resolved_to_null_passes_inline) or test(required_string_resolved_to_null_fails_schema_validation)'`
- `cargo run -q -p darkmatter-cli -- schema validate --format json <temp-required-enum-null.md>`

## Production Readiness

Ready for production. The review 1 behavior gap is fixed, the core acceptance paths have appropriate Level 1 coverage, and the only remaining issue is a small missing regression test for a working required-enum edge case.
