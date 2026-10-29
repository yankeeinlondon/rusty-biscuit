---
area: darkmatter
status: finalized-spec
created: 2026-10-10
owner: Ken Snyder <ken@ken.net>
$schema:
    status: |-
        enum(
            draft-spec,
            finalized-spec,
            planned,
            implemented,
            review-findings,
            human-in-the-loop,
            completed,
            on-hold,
            abandoned
        ) -> an indicator of progress for this specification
    reviewed: boolean -> indicates whether the specification file has been reviewed by another agent from the one which created the spec
    reviewed_by: string -> the agent and model used in the spec review
    reviewed_on: date -> the date the spec was reviewed
    review_iterations: number -> the number of implementation reviews have taken place in the review/fix cycle
    clarified: boolean -> indicates whether the specification was built -- _in part_ -- with the 'clarify.md' prompt
    implemented: boolean -> indicates whether this spec's plan has been implemented
    implemented_by: string -> the agent who implemented the plan
    formerly: string -> the spec's previous `{date}-{name}`, when it was renamed in the 2026-10-10 restructure
    pipeline_step: number -> position in the claudine/docs/rollout-strategy.md pipeline
reviewed: true
reviewed_by: codex/gpt-6.1-sol
reviewed_on: 2026-10-11
review_iterations: 1
completed: true
human_review: false
message_to_agent: |-
    All 3 phases are complete; implementation complete, ready for review. See implementation-log.md "## Phase 3" for the acceptance-criteria-to-evidence table and diff audit.
    - Only darkmatter/lib/src/markdown/schemas/resolve.rs (Arc-shared named-type tables + 6 `shared_type_tables` L1 tests) and darkmatter/docs/topics/schemas/definition.md changed.
    - Local macOS gates: darkmatter `just test` 9160/9160, `just lint` clean; claudine `just test` 8673/8673, `just lint` clean. No cross-OS run; existing CI cells cover Linux/macOS/Windows/WSL2.
implemented: true
pipeline_step: 2
canonical_docs:
    - darkmatter/docs/topics/schemas/definition.md
related:
    - 2026-10-29-recursive-schema-types
---

# Shared Named-Type Tables in the Schema Resolver

The Darkmatter schema resolver expands every `Name@file` (and `Name@this`)
reference by looking the name up in the target file's table of named types.
Today returning that table makes a **deep copy of every definition**, even
when the import needs only one. A schema with many references to a large type
library therefore copies that library once per reference. This spec shares
each table behind an `Arc`, Rust's reference-counted shared pointer, so
returning the table only increments a reference count. Claudine's lifecycle
schemas are the motivating case; the implementation belongs to Darkmatter.

For example, `first: A@./types.yaml` and `second: B@./types.yaml` read one
shared table from `types.yaml`. Each property still receives its own expanded
definition. File-reference resolution, copying the selected definition, and
expanding it retain their existing costs; this change removes only repeated
whole-table copies. It does not promise a fixed end-to-end speedup.

This is a performance refactor only. **No behavior changes**: every schema
resolves to the same JSON Schema, including ordering and byte-identical output
when serialized in the same way, with identical errors and dependency edges.
The parsed schema, origin information, referenced files, examples, and
advisories returned by the resolver also remain unchanged.

## Decided

### Ownership and sharing

- In Darkmatter, [Namespace and CachedNamespace](../../../darkmatter/lib/src/markdown/schemas/resolve.rs)
  hold their named-type tables as `Arc<IndexMap<String, PropertyDef>>`.
  These private structures supply definitions and relative-file context to
  import expansion. Keep `IndexMap` so declaration order is preserved; no
  public type or API changes and no new crate dependency are needed.
- Move the table returned by Darkmatter's
  [load_named_types](../../../darkmatter/lib/src/markdown/schemas/resolve.rs)
  into one `Arc` when a file is first loaded. The cache entry and the namespace
  returned for that first import must share that allocation. Cache hits and
  `@this` also clone only the shared pointer. All three existing full-table
  copy sites in Darkmatter's
  [ImportEngine::resolve_namespace](../../../darkmatter/lib/src/markdown/schemas/resolve.rs),
  the method that selects the namespace for an import, are removed.
- Tables are immutable after construction. Expansion clones only the selected
  `PropertyDef`, the definition of one named type, before applying use-site
  constraints, descriptions, or array modifiers. Never mutate the shared
  table through a lock, interior mutability, or `Arc::make_mut`: two imports
  of the same type must remain independent.
- Keep the one-time copy of the root document's properties in Darkmatter's
  [expand_document_imports](../../../darkmatter/lib/src/markdown/schemas/resolve.rs),
  which sets up one import-expansion operation, then wrap that copy in an
  `Arc`. The original properties are consumed while producing the expanded
  schema, so the unexpanded table must remain available for `@this`.
  Schemas without imports retain the existing early return without allocating
  a shared table.

### Lifetime and unchanged resolution rules

- Sharing lasts for one existing import-expansion operation, not necessarily
  an entire top-level resolve. Root-union arms and separately expanded schema
  files may create separate engines today; do not merge those engines or
  extend their cache lifetime. A later operation loads its own tables as it
  does today. Shared ownership adds no global cache or retained cross-call
  state.
- Preserve the existing canonical-file cache keys, namespace identities,
  relative-import base directories, request-scoped file-resolution context,
  and schema-root lookup. An `@this` reference inside an imported file still
  selects that imported file's table, not the root document's table.
- Keep dependency recording before the cache lookup, including on cache hits.
  Cycle detection, the depth cap (`MAX_IMPORT_DEPTH`, currently 32), load
  errors, and error text are untouched. Sharing a table does not share an
  expanded definition or bypass the expansion stack.
- Both supported standalone mapping envelopes continue to supply named
  types: a pure root `$schema` mapping and `kind: schema` with a `types`
  mapping. Raw JSON Schema, root-union documents, and whole-file-reference
  documents retain their existing rejection behavior as named-import targets.

> **Reader's note:** This deliberately preserves today's stripping of
> top-level `required` and `generated` when a named type is imported.
> Preserving those constraints and accepting bare local names are intended
> behavior changes in `2026-10-13-named-type-presence-and-local-names`.
> Combining them with this ownership change would prevent unchanged tests
> from establishing that sharing alone preserves behavior.

## Out of scope

- Emitting `$defs`/`$ref` or recursion: `2026-10-29-recursive-schema-types`.
- Caching resolved schemas across calls or in DMLS.
- Any change to presence constraints or bare local names:
  `2026-10-13-named-type-presence-and-local-names`.
- Sharing expanded definitions, reducing the fully expanded output size, or
  changing the loader, file-reference rules, or public resolver APIs.

## Acceptance criteria

1. No behavior change: the existing Darkmatter suite (`just test` in
   `darkmatter/`) passes unmodified, including the resolver, import-cycle,
   and shipped-schema corpus tests. No test expectation is edited to make it
   pass.
2. Focused unit tests beside the private resolver implementation prove
   allocation identity using `Arc::ptr_eq`:
   - A first file lookup returns the same table allocation held by the cache;
     subsequent lookups for two names from that file retain that allocation.
     Comparing only two cache hits is insufficient: it would miss a copy made
     during insertion.
   - Two `@this` lookups share the original current namespace's allocation,
     both for the inline root and for a loaded file. The loaded file's table
     is distinct from the root table.
   Exercise the actual namespace-resolution method; a test that merely
   clones a manually constructed `Arc` proves nothing about the resolver.
   These private tests need no public inspection API, runtime counters, or
   observer callbacks. Reference counts alone are insufficient because two
   independent allocations can have the same count.
3. A small resolution regression test imports one type twice, with a
   constraint or description applied to only one use. Assert the expected
   output for both properties, demonstrating that changing one use does not
   affect the other. Cover cross-file reuse and `@this`, including `@this`
   authored inside an imported file; cover both supported mapping envelopes.
   Reuse existing tests where they already prove these behaviors. Use
   hermetic temporary files and the repository's test-toolkit conventions.
4. Review the diff to confirm all three full-table copies are replaced by
   shared-pointer clones and only the selected definition is copied during
   expansion. Allocation-identity assertions are the performance regression
   check; no timing threshold, benchmark suite, or performance spike is
   required for this bounded ownership change.
5. `just lint` passes in `darkmatter/`. Tests are L1, need no terminal or
   browser, and are portable to macOS, Linux, native Windows, and WSL2. Use
   the existing nextest-backed recipes and CI coverage; no new CI cells.
6. Alongside implementation, update relevant resolver field comments to
   describe immutable shared tables and their lifetime. The existing comment
   that describes named tables as only `$schema` entries omits the supported
   `kind: schema` / `types` envelope; correct that wording without changing
   classification. Add a short implementation note to
   [Darkmatter's schema definition documentation](../../../darkmatter/docs/topics/schemas/definition.md)
   explaining per-expansion sharing and independent use-site definitions.
   Keep current behavior documentation free of feature-directory references.

## Open questions

None.
