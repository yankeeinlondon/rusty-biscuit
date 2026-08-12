---
ready: false
agent: codex/default
created: 2026-07-09T08:55:58
---

# Schema Plus Review 4

Verdict: **not production ready**.

The review-3 implementation improved `example()` validation by checking an example's `returns` value against the annotated property type, and the targeted Level 1 suite passes. One acceptance-criteria gap remains: `parameters` are still validated only as generic one-key maps, not against the inherited target signature required by the spec.

## Findings

### High: `example()` parameter validation still does not use the inherited target signature

Spec requirement: each referenced example is validated at schema-load time, and target-specific fields such as `parameters` validate against the annotated property or typed expression-function signature (`spec.md:61-65`). The acceptance criteria repeat this explicitly: `example(<file>, ...)` must validate target-specific `parameters` against the inherited target signature (`spec.md:426-429`). The example notes also say parameter type declarations are inherited from the annotated property or typed expression-function catalog to prevent drift (`spec.md:89-96`).

Implementation still validates `parameters` only against the generic `parameter[]` container shape: an array of single-key maps with `any` values (`darkmatter/lib/src/markdown/schemas/example.rs:16-18`, `darkmatter/lib/src/markdown/schemas/example.rs:182-199`). The module documentation explicitly marks typed expression-function parameter validation as deferred (`darkmatter/lib/src/markdown/schemas/example.rs:31-36`). The new `returns` validation covers the annotated property's result value, but it does not cover parameter names, arity, or per-parameter value types.

Result: a schema can still ship an example for a typed function-like target with the wrong parameter name, a missing or extra parameter, or a value incompatible with the eventual function signature, as long as the `parameters` block is an array of one-key maps. That is the drift the spec says inherited signature validation is meant to prevent.

Verification level: **Level 1 is the correct tier**, because this is pure schema-load validation behavior. Current Level 1 tests cover malformed envelopes, generic one-key parameter arity (`resolve.rs:1915-1939`), and target-typed `returns` mismatches (`resolve.rs:1942-1992`). They do not and cannot cover target-signature-aware parameter validation because the implementation still defers that model. Add negative Level 1 tests when the typed catalog/signature model is present: wrong parameter name, missing parameter, extra parameter, and wrong parameter value type.

## Test Rigor

Schema-plus has no terminal-rendered, hotkey, paste, mouse, scroll, or OS-keyboard behavior. Level 1 is appropriate for all user-observable requirements in this feature; no Level 2 or Level 3 tests are required by the taxonomy.

The strongest current tests are Level 1 and cover parsing, conversion, import resolution, pattern keys, content-format validation/coercion, fixture migration, `returns` target checks, and DMLS cache invalidation. The remaining production blocker is also a Level 1 gap tied to acceptance criterion 1.

## Verification Run

Ran targeted Level 1 nextest checks:

```sh
cargo nextest run --no-fail-fast -p darkmatter -p dmls \
  schema_plus \
  schema_cache_invalidates_on_dependency_change \
  schema_cache_invalidates_on_referenced_schema_file_change \
  schema_cache_invalidates_on_extension_baseline_change \
  returns_native_mapping_validates_against_yaml_target \
  returns_native_value_validates_against_json_target \
  example_validates_as_unordered_list_fm_fixture
```

Result: **66 passed**, 5509 skipped.
