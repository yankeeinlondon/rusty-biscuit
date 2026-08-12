---
ready: false
agent: codex/default
created: 2026-07-09T07:58:11
implemented: true
---

# Schema Plus Review 2

Verdict: **not production ready**.

The review-1 fixes improve the implementation: target `returns` validation was added for `example(...)`, import/example dependency edges are exposed on `EffectiveSchema`, and the targeted Level 1 regression tests pass. I still found two production blockers.

## Findings

### High: `example()` target validation bypasses schema-plus `yaml` / `json` coercion

Spec requirement: `yaml` and `json` types must accept native YAML values through transient coercion without mutating validation-only callers, and the `example.yaml` invocation union must validate through that behavior (`spec.md:253-274`, `spec.md:444-446`). Review-1 also required the target-specific example check to validate examples against the annotated target.

The envelope path correctly does this: `validate_example_object` coerces a working copy before validating the artifact envelope (`darkmatter/lib/src/markdown/schemas/example.rs:160-172`). The new target-specific `returns` path does not. `validate_returns_against_target` builds a validator directly and checks the raw `returns` value (`darkmatter/lib/src/markdown/schemas/example.rs:221-235`), so a valid example attached to a `yaml` or `json` property with native `returns` data is rejected for not already being a string.

This violates Feature D's validation-only coercion contract and creates an inconsistent schema load path: `frontmatter: { ... }` in `invocation` works, but `returns: { ... }` for an annotated `yaml` property does not.

Verification level: **Level 1 is the correct tier**, because this is pure schema validation behavior. Existing Level 1 tests cover envelope coercion and scalar target mismatch, but they do not pair the target-specific example check with content-format coercion. Add tests where `example(...)` is attached to `yaml` and `json` properties and `returns` is a native mapping/sequence; validation should pass without mutating the parsed example object.

### High: DMLS schema cache invalidation still misses file-backed schema sources

Spec requirement: `Name@fileref` / `Name@this` resolution must provide dependency edges so DMLS invalidates cached schemas when referenced files change (`spec.md:145-153`, `spec.md:430-432`). The review-1 cache fix hashes `bundle.effective.dependencies()` (`darkmatter/dmls/src/overlay/mod.rs:152-164`, `darkmatter/dmls/src/overlay/mod.rs:182-195`), but that dependency list is only the document `$schema`'s import/example union (`darkmatter/lib/src/markdown/schemas/mod.rs:288-309`, `darkmatter/lib/src/markdown/schemas/mod.rs:376-399`).

Two file-backed schema sources are still omitted:

- the root schema file itself when a document uses `$schema: ./schema.yaml`; `resolve_reference` records the file only as `SchemaOrigin::referenced_file`, not as a dependency (`darkmatter/lib/src/markdown/schemas/resolve.rs:220-240`);
- DMLS extension baseline files and their import/example edges; `combined_baseline` loads them, merges only `resolved.json_schema`, and drops `resolved.imports` / `resolved.examples` (`darkmatter/dmls/src/overlay/schema.rs:89-107`).

Result: an open DMLS document can keep a stale `SchemaBundle` after editing the referenced schema file itself, or after editing an extension baseline/import/example file, as long as the Markdown document text and schema config are unchanged. That is the same user-visible stale diagnostics/completion/hover class review-1 called out, just through the file-backed schema source rather than the narrow imported-type happy path now tested.

Verification level: **Level 1 is the correct tier**. Add DMLS overlay cache tests for `$schema: ./schema.yaml` where the schema file's own type changes, and for a configured extension baseline whose source or imported type changes. Both should reassemble on the next `for_document` call with unchanged document text/config.

## Test Rigor

Schema-plus has no terminal-rendered, hotkey, paste, mouse, scroll, or OS-input behavior. Level 1 is the appropriate verification level for all user-observable requirements in this feature; no Level 2 or Level 3 tests are required by the stated taxonomy.

The strongest current tests are Level 1. They cover many parser/converter/validation paths, and the review-1 regression tests now pass, but the two findings above are requirements whose strongest test is currently missing.

## Verification Run

Ran targeted Level 1 nextest checks:

```sh
cd darkmatter
cargo nextest run --no-fail-fast -p darkmatter -p dmls \
  schema_cache_invalidates_on_dependency_change \
  dependencies_surface_import_and_example_edges \
  example_returns_wrong_target_type_is_a_schema_load_error \
  returns_wrong_target_type_is_rejected
```

Result: **4 passed**, 5558 skipped.

