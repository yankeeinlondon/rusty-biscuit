---
ready: false
agent: codex/default
created: 2026-07-09T08:48:11
---

# Schema Plus Review 3

Verdict: **not production ready**.

The review-2 blockers appear to be addressed: `example()` target `returns` validation now uses transient `yaml` / `json` coercion, and DMLS schema cache invalidation now hashes referenced schema files, imports, examples, and extension-baseline dependencies. The targeted Level 1 regression suite passes.

I found one remaining acceptance-criteria gap.

## Findings

### High: `example()` parameter validation is still generic, not target-signature-aware

Spec requirement: `example(...)` validation must validate target-specific fields such as `parameters` against the annotated property or typed expression-function signature (`spec.md:61-65`, `spec.md:89-96`), and acceptance criterion 1 requires target-specific `parameters` validation against the inherited target signature (`spec.md:426-429`).

Implementation currently validates `parameters` only against a generic `parameter[]` shape: an array of one-key maps with `any` values (`darkmatter/lib/src/markdown/schemas/example.rs:82-100`, `darkmatter/lib/src/markdown/schemas/example.rs:182-199`). The module docs explicitly mark signature-aware parameter validation as deferred (`darkmatter/lib/src/markdown/schemas/example.rs:31-36`). The expression function descriptor catalog currently exposes display signatures as strings, but no structured parameter names/types or return type model for this validation layer (`darkmatter/lib/src/markdown/compose/expression/catalog.rs:10-23`).

Result: schema load catches malformed parameter containers, but it cannot reject drift between an example and the function it demonstrates. A well-formed example for `as_unordered_list(list)` can use the wrong parameter name, wrong arity, or a value outside the eventual typed function signature and still pass schema-plus validation. That is exactly the drift the spec says inherited target validation is meant to prevent.

Verification level: **Level 1 is the correct tier**, because this is pure schema-load validation behavior. Existing Level 1 tests cover envelope validation, generic one-key parameter maps, fixture acceptance, and `returns` target mismatch. They do not cover function-signature-aware parameter validation because the implementation does not expose the required typed signature model yet. Add negative Level 1 tests once the catalog has structured signatures, for example: wrong parameter name, extra/missing parameter map, and wrong value type for a typed expression-function example.

## Test Rigor

Schema-plus has no terminal-rendered, hotkey, paste, mouse, scroll, or OS-input behavior. Level 1 is appropriate for all user-observable requirements in this feature; no Level 2 or Level 3 tests are required by the stated taxonomy.

The strongest current tests are Level 1 and are broad across parsing, conversion, validation, fixture migration, and DMLS cache invalidation. The remaining gap is also a Level 1 gap: acceptance criterion 1 is not fully implemented because parameter validation is not yet target-signature-aware.

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
