---
$schema: feature-review.yaml
ready: true
findings: []
observations:
    - Documentation describes sharing over an entire resolution
human_review: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: "2026-10-11T08:37:37-07:00"
spec: 2026-10-12-shared-type-tables/spec.md
implemented: false
description: "A **feature** review of `2026-10-12-shared-type-tables/spec.md`"
feature: 2026-10-12-shared-type-tables/review-1.md
---

# Review of Shared Type Tables

The feature is production ready. No blocking defects or missing acceptance coverage were found. The change removes repeated copies of whole named-type tables while keeping each imported definition independent.

## Scope and implementation

Reviewed the specification, plan, implementation log, and the implementation commits `8a70498cd` and `4178c41ad`. The implementation boundary is Darkmatter's private [schema resolver](../../../darkmatter/lib/src/markdown/schemas/resolve.rs) and its [schema definition documentation](../../../darkmatter/docs/topics/schemas/definition.md). Existing test expectations were not changed.

Checked every table-return path in Darkmatter's [ImportEngine::resolve_namespace](../../../darkmatter/lib/src/markdown/schemas/resolve.rs:1152):

| Path | Implementation and verification |
| --- | --- |
| First file load | The loaded map moves into one `Arc`; the cache and returned namespace share it. The test compares these allocations immediately after the first lookup. |
| File cache hit | Only the pointer is cloned. Two later lookups, with different reference spellings and different selected names, retain the first allocation. |
| Inline-root `@this` | Two calls to the real namespace resolver share the original root table. |
| Imported-file `@this` | Two calls share the loaded file's table, which is also asserted to differ from the root table. Both supported file envelopes are exercised. |

Darkmatter's [expand_document_imports](../../../darkmatter/lib/src/markdown/schemas/resolve.rs:942) keeps the intentional one-time root-properties copy after the no-import early return. Darkmatter's [ImportEngine::expand_import](../../../darkmatter/lib/src/markdown/schemas/resolve.rs:1089) still copies the selected definition before expansion and use-site modification. No shared-table mutation, global cache, public API change, dependency change, or CI change was introduced.

The diff preserves declaration order, namespace identity, canonical cache keys, relative-directory context, dependency recording before cache lookup, cycle detection, depth limits, classification, and error text. Conversion and the projection of parsed schemas, origins, examples, referenced files, and advisories are unchanged.

## Acceptance criteria and verification levels

| Requirement | Evidence | Level |
| --- | --- | --- |
| Existing resolution behavior remains unchanged | Existing resolver, import-cycle, envelope, dependency, and shipped-schema corpus tests; unchanged conversion and error paths in the diff | L1 |
| First load, cache hits, and both forms of `@this` share allocations | Three private `shared_type_tables` tests call the actual namespace resolver and assert `Arc::ptr_eq`, including first-load/cache identity and root/file separation | L1 |
| Repeated imports remain independent | Three `shared_type_tables` tests assert public resolution output for cross-file reuse, inline-root `@this`, and `@this` inside an imported file; constraints and descriptions affect only one use | L1 |
| Both supported mapping envelopes continue to work | New file-backed tests loop over pure `$schema` and `kind: schema` / `types`; existing envelope and nested-import tests remain unchanged | L1 |
| Whole-table copies are removed without sharing expanded definitions | All three copy sites audited; selected-definition copying retained; allocation assertions provide the specified performance check | Source review and L1 |
| Lint and portable test construction | Area lint recipe; temporary directories, request-scoped contexts, production canonical-path handling, and no environment mutation in the new tests | Static review and local checks |
| Comments and implementation note describe the change | Resolver comments cover immutable tables, both envelopes, and per-expansion lifetime; documentation includes an independent-use example | Source review, with one nonblocking observation below |

The six new tests compile in the library's `#[cfg(test)]` module and were selected and passed by the normal L1 recipe. They require no optional feature or terminal resource, and each ran in under 0.04 seconds in this review. L1 is the appropriate level: the feature changes private ownership and schema-resolution results, with no terminal rendering or keyboard-input requirement. No L2 or L3 verification is needed.

The input robustness matrix does not apply: no format parser, configuration reader, deserializer, or loader classification changed. This work changes the ownership of already parsed tables.

Local macOS verification performed for this review:

- `just test` in `darkmatter/`: 9,160 passed, 11 skipped by the existing selection, exit 0. Test execution took 37.6 seconds.
- `just lint` in `darkmatter/`: passed, exit 0, including the Zed WASM extension compile check.

Cross-OS execution remains with the existing CI coverage and is not a readiness blocker. No human-only validation or unresolved design decision is required.

## Findings

None.

## Observations

### Documentation describes sharing over an entire resolution

The new [implementation note in Darkmatter's schema definition documentation](../../../darkmatter/docs/topics/schemas/definition.md:894) says that each target file is parsed once “within one schema resolution.” Sharing actually lasts for one import-expansion operation: Darkmatter's [inline root-union resolver](../../../darkmatter/lib/src/markdown/schemas/resolve.rs:196) and [standalone root-union resolver](../../../darkmatter/lib/src/markdown/schemas/resolve.rs:759) create separate engines for separate arms. A union whose two arms import the same file can therefore load two tables during one top-level resolution. The implementation correctly preserves the specification's engine boundaries. Narrow the note to “within one import-expansion operation,” and mention that union arms may expand separately; extending the cache would add unnecessary scope. This is a documentation observation, not a requirement for stronger sharing or a blocker for this review.
