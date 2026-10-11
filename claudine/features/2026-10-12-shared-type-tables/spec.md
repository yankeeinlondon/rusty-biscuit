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
    implemented: boolean -> indicates whether this spec's plan has been implemented
    implemented_by: string -> the agent who implemented the plan
    formerly: string -> the spec's previous `{date}-{name}`, when it was renamed in the 2026-10-10 restructure
    pipeline_step: number -> position in the claudine/docs/rollout-strategy.md pipeline
reviewed: false
human_review: false
pipeline_step: 2
related:
    - 2026-10-29-recursive-schema-types
---

# Shared Named-Type Tables in the Schema Resolver

The schema resolver expands every `Name@file` (and `Name@this`) reference by
looking the name up in the target file's table of named types. Today each
lookup hands back a **deep clone of the whole table**, so a schema with many
references to a large type library pays for copying that library once per
reference. This spec shares each table behind an `Arc` so a lookup costs a
reference-count increment. Schema authors see faster resolves of large,
heavily cross-referenced schemas (Claudine's lifecycle schemas are the
motivating case) and no other difference.

This is a performance refactor only. **No behavior changes**: every schema
resolves to byte-identical JSON Schema with identical errors and dependency
edges.

## Decided

- Named-type tables are immutable after load and are shared, never cloned, by
  every expansion that reads them.
- The three per-expansion clone sites in
  `darkmatter/lib/src/markdown/schemas/resolve.rs` go away:
  - `resolve_namespace` for `@this` (`named_types: current.named_types.clone()`,
    ~:1151);
  - the namespace-cache hit (`cached.named_types.clone()`, ~:1212);
  - the namespace-cache insert, which clones a freshly loaded table to keep a
    copy (~:1224).
- `Namespace` and `CachedNamespace` hold the table as a shared pointer
  (`Arc<IndexMap<String, PropertyDef>>` or an equivalent the implementer
  chooses). Looking up one definition may still clone that one `PropertyDef`,
  because expansion consumes it.
- The one-time copy of the root document's properties in
  `expand_document_imports` (~:944) may stay or become shared; either is
  acceptable because it runs once per resolve.
- Cycle detection, the depth cap (`MAX_IMPORT_DEPTH`), dependency recording,
  and error text are untouched.

## Out of scope

- Emitting `$defs`/`$ref` or recursion: `2026-10-29-recursive-schema-types`.
- Caching resolved schemas across calls or in DMLS.
- Any change to presence constraints or bare local names:
  `2026-10-13-named-type-presence-and-local-names`.

## Acceptance criteria

1. No behavior change: the existing Darkmatter suite (`just test` in
   `darkmatter/`) passes unmodified, including the resolver, import-cycle,
   and shipped-schema corpus tests. No test expectation is edited to make it
   pass.
2. A unit test in `resolve.rs` proves sharing: resolving a schema that
   references two names from the same file (and two names via `@this`)
   observes that every expansion of that file reads the same table allocation
   (`Arc::ptr_eq`, or an equivalent strong-count / allocation check), so a
   reintroduced deep clone fails the test.
3. `just lint` passes in `darkmatter/`. Tests are L1, need no terminal or
   browser, and are portable to macOS, Linux, Windows, and WSL2. No new CI
   cells.

## Open questions

None.
