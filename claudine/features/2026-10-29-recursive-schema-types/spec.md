---
area: darkmatter
status: finalized-spec
created: 2026-09-28
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
reviewed_on: 2026-10-16
review_iterations: 0
human_review: false
pipeline_step: 19
formerly: 2026-09-28-recursive-schema-types
depends-on:
    - 2026-10-14-union-type-constraints
    - 2026-10-28-schema-enhancements
related:
    - 2026-10-12-shared-type-tables
    - 2026-10-13-named-type-presence-and-local-names
    - 2026-10-15-diagnostic-node-positions
    - 2026-10-20-lifecycle-grammar
    - 2026-10-30-claudine-recursive-schemas
    - 2026-10-24-sequence-containers
---

# Recursive Named Types in SimplifiedSchema

SimplifiedSchema is the `darkmatter` library's YAML schema dialect for Markdown frontmatter. Authors declare reusable named types in a `types:` table. With the prerequisite features implemented, a property can use a bare local name (`author: person`), an explicit local reference (`person@this`), or a cross-file reference (`person@./people.yaml`). Today the resolver copies each referenced body into the property before compiling it to JSON Schema. A type that refers back to itself is rejected by the library's [ImportEngine::expand_import](../../../darkmatter/lib/src/markdown/schemas/resolve.rs), which expands named references and detects import cycles.

This spec allows named types to describe nested structures without a fixed unroll. Cyclic definitions are stored once in JSON Schema's `$defs` table and used through `$ref`, a reference to a definition in that table. The generated schema's size does not depend on the nesting depth of a document. Existing acyclic definitions still inline, so this is not a general guarantee of output size proportional to authored schema size.

**Starting point:** implement this feature after the union-reference and schema-enhancement prerequisites in `depends-on`. The union-reference dependency supplies local names and inherited presence rules; the schema-enhancement dependency supplies the newer core types and conversion engine. The earlier rollout steps for shared type tables and diagnostic positions are also required starting-state contracts, even though they are listed under `related` rather than as transitive dependencies here. References below describe the required behavior at that point, not a claim that those drafts are already implemented in this checkout. Adopting recursion in Claudine's lifecycle schemas is separate work, identified by `2026-10-30-claudine-recursive-schemas`.

## Problem

A grammar that is recursive by nature (Claudine's `when`/`then`/`else` lifecycle items) can only be expressed today by **unrolling** the recursion to a fixed depth N with N named types, and since each reference is inlined, every level duplicates every level below it. The schema spike measured the cost (the “Resolved JSON Schema size and resolve time” section in the spike results for `2026-10-20-lifecycle-grammar`; host under load, upper bounds):

| depth | resolved JSON | resolve | `md schema validate` | inlined copies of one 143-key table |
|---|---|---|---|---|
| 1 | 1.9 MB | 109 ms | 234 ms | 42 |
| 3 | 9.5 MB | 657 ms | 1.1 s | 210 |
| 5 | 39.9 MB | 2.7 s | 5.9 s | 882 |

Copies per event = 2^(N+2) − 2, times seven consumers; size and time roughly double per level. The measured size growth comes from duplication; caching cannot remove that expanded schema: DMLS keys its bundle on full document text, so a changed document text can cause re-resolution. The DMLS package implements this bundle cache in [overlay/mod.rs](../../../darkmatter/dmls/src/overlay/mod.rs).

Recursion was consciously deferred: `2026-07-08-schema-plus` recorded the decision that named types could not form a cycle, and recursive reference types were deferred beyond the first version.

## Decided

- **Recursion is allowed.** A named type may reference itself directly, or through other named types in the same file or in other files. A supported cycle lowers to a JSON Schema `$ref` into a root `$defs` map.
- **Every cycle must cross a child-value boundary.** `item: "{ children: item[] }"` evaluates the recursive type on a child value, so it is supported. In contrast, `a: b` plus `b: a`, or `a: [a, string]`, would revisit the same schema and instance without making progress; that is not a nesting grammar, a naive validator or editor walker could loop forever on it, and [JSON Schema's recursion guidance](https://json-schema.org/draft/2020-12/json-schema-core#section-9.4.1) leaves its evaluation undefined. The resolver marks the reference edges that descend into a child value: an object property, a dictionary value, an array item, or a tuple slot. Nothing else is a descending edge; aliases and union arms resolve without descent, and references inside a callable signature are not on the list, so they do not count either. Removing every descending edge must leave an acyclic graph; finding one descending edge somewhere in a recursive group is not enough. Any remaining cycle is a typed schema error that names the references forming the cycle and their source spans. This rejects self-unions such as `a: [a, string]` even though another arm could accept a value. Resource guards are not an authored nesting-depth constraint and never turn exhausted traversal into a successful validation or a silently unconstrained conversion.
- **No new surface grammar.** Recursion uses the existing reference spellings: a bare local name, `Name@this`, or `Name@file`.

  ```yaml
  kind: schema
  types:
      item: |-
          { say: string, when: expression, then: item[], else: item[] }
  ```

- **Acyclic types are unchanged.** Compared with the tree after the prerequisites are implemented, every schema that does not recurse lowers to byte-identical JSON Schema; inlining stays the default.
- **Depth is not a schema concern** (decided 2026-10-10). SimplifiedSchema gains no authored depth constraint. Recursion describes all finite levels permitted by the existing document parser and resource guards. A consumer that needs a nesting limit enforces it in its own parser only.
- **Editor descent** (decided 2026-10-10). DMLS completion and hover descend through a recursive reference lazily, resolving one level at a time as the cursor's document position requires, and reach at least two recursion levels deep. A recursive reference is never shown as opaque.
- **Presence markers ride the reference.** `required`, `generated`, and universal `eager` on a property whose type is a recursive reference follow the same inheritance and scope rules as inlined references. `eager` does not imply required presence, and `generated` does not create a value. The core-type prerequisite's nullable-union rules still override required presence where applicable.
- **Validation errors stay positioned.** A validation error raised inside a `$ref` target maps back to the offending document node's JSON path and source line/column, as `2026-10-15-diagnostic-node-positions` requires for inlined schemas.

## Design

The implementation changes the `darkmatter` library resolver, compiled-schema consumers, `darkmatter-cli`, and the DMLS language server together. A reference must remain useful for validation, transformations, and editor assistance; introducing `$ref` only in the converter is insufficient.

1. **Reference graph.** Build a graph of reachable named definitions across files, retaining the scope and source location of each reference. Group definitions that can reach each other into strongly connected components (groups of mutually recursive types). Discover each definition and dependency once; recognize a back-edge before applying a chain-depth check. Preserve the existing acyclic import-chain limit in [resolve.rs](../../../darkmatter/lib/src/markdown/schemas/resolve.rs), rather than allowing graph discovery to bypass it. Unknown names, unreadable imports, unsupported remote references, trigger-payload cycles, and root schema-file cycles remain errors. Only supported named-type recursion changes. Classify each edge as descending or not and reject cycles that survive removal of the descending edges, as described under Decided.
2. **Acyclic types inline exactly as today.** Capture the compatibility baseline after all prerequisites are implemented and before this resolver change. Cover shipped schema and trigger artifacts, the built-in baseline, and resolver fixtures, including pure `$schema` documents and inline/root-union references as well as `kind: schema` files. Keep a stable, explicit test-input manifest; use the repository-read forms required by the `rust-testing` skill. Artifact discovery must not treat all files under an area as schemas or silently miss non-kinded declarations.
3. **Typed references.** The library's [TypeExpr](../../../darkmatter/lib/src/markdown/schemas/simplified/types.rs), which represents a property's type, gains `Ref(DefKey)` for cyclic named definitions. Every use of a cyclic definition is a reference, not just the edge that closes the cycle. An acyclic alias may inline a reference to a cyclic target. Reuse the prerequisite's array-of-union representation; a recursive union used as `U[]` means an array whose items reference the whole union. Inspect the resolved outer shape to preserve the existing rejection of adding `[]` to an already-array type.
4. **Definition identity and storage.** Internally, `DefKey` combines the namespace identity and type name. File-backed namespaces use the existing resolved file identity; inline roots, separate root-union arms, and embedded schemas need distinct identities rather than a shared root sentinel. Retain each definition's origin and imported-file dependencies, including a file reached only on a cycle-closing edge. Definitions travel with `ResolvedSchema`, `EffectiveSchema`, and DMLS bundles using the prerequisite's shared tables behind `Arc`, Rust's reference-counted pointer. Handles point into tables; definitions must not create an owning `Arc` cycle that leaks memory. Generated JSON keys are deterministic opaque identifiers allocated in stable traversal order, not absolute paths or process-random hashes. Carry an explicit identity-to-key map and re-key generated references when combining independently lowered roots. If hashing is needed, use `biscuit-hash` under its skill; hashing is not required for this design.
5. **Lowering and reference-site metadata.** The library's [to_json_schema](../../../darkmatter/lib/src/markdown/schemas/simplified/convert.rs), which compiles the typed model, emits local `#/$defs/<key>` references and one body per reachable cyclic definition. Escape keys as JSON Pointer tokens. Definitions contain the reusable value shape; missing/null handling belongs to the property use site, so an optional object property may accept null while a list item of the same object type does not. Merge inherited and use-site constraints under the existing rules before lowering. Keep list item constraints distinct from array-container constraints, preserve descriptions/defaults/suggestions, and report contradictory or unsupported constraints with definition and use-site provenance. Two properties using the same definition with different constraints must not mutate that shared definition. `$ref` siblings remain applicable under [Draft 2020-12 reference semantics](https://json-schema.org/draft/2020-12/json-schema-core#section-8.2.3.1). The converter still rejects unresolved authored imports.
6. **Merge boundaries.** The library's [validate_simple_object_schema](../../../darkmatter/lib/src/markdown/schemas/resolve.rs) and [enforce_merge_compatible](../../../darkmatter/lib/src/markdown/schemas/triggers/assemble.rs) protect property-based baseline and trigger merging. Admit `$defs` and supported local references inside property schemas while retaining the simple object root: do not admit a root `$ref`, root union, or other root applicator merely by extending the allow-list. Keep document-over-baseline and trigger precedence unchanged. When combining definition tables, re-key colliding generated keys and all affected generated references; never merge unrelated bodies by last-writer-wins. The new admission of raw `$defs` at these merge boundaries is limited to checked `#/$defs/…` references without `$id` changes to their scope or external targets. Isolate their keys too. This does not tighten existing acyclic property schemas that were already accepted. Raw document schemas retain their existing Draft 2020-12 semantics and are not blindly rewritten as generated definitions. Re-key only schema-bearing locations: literal data under `const`, `enum`, `default`, `examples`, and suggestion metadata must remain byte-for-byte unchanged even if it contains keys named `$ref` or `$defs`. Resolve reference identities before moving an arm; a reference to the original root must not accidentally bind to the assembled root. Root-union assembly, mixed raw/simplified arms, baseline merge into each arm, and synthetic sub-schemas must preserve each reference's original scope.
7. **JSON walkers.** Add shared reference lookup used by the library's [validate.rs](../../../darkmatter/lib/src/markdown/schemas/validate.rs), [clean.rs](../../../darkmatter/lib/src/markdown/schemas/clean.rs), [coerce.rs](../../../darkmatter/lib/src/markdown/schemas/coerce.rs), [rewrite.rs](../../../darkmatter/lib/src/markdown/schemas/rewrite.rs), and [discriminant.rs](../../../darkmatter/lib/src/markdown/schemas/discriminant.rs). Lookup resolves a reference without discarding sibling assertions or annotations. Missing targets are schema errors, never an unconstrained schema. Instance walkers may revisit a definition at a new instance path; a global visited-definition set would skip valid deeper children. Guard repeated visits to the same schema location at the same instance location instead. Synthetic validators receive the applicable root definitions and scope. Preserve each consumer's existing union policy: closest-arm reporting cannot authorize coercion, cleanup, or eager-file rewriting. Validation remains passive; composition performs only its existing authorized transformations in the captured request context. Walk existing instance values and values introduced by already-supported normalization rules; do not materialize absent recursive children merely to traverse their type. Resource exhaustion returns the existing typed failure and leaves the input unchanged.
8. **Typed-tree walkers and editor behavior.** Resolve references through the definition table in the library's [completion.rs](../../../darkmatter/lib/src/markdown/schemas/completion.rs), [frontmatter_shape.rs](../../../darkmatter/lib/src/markdown/schemas/frontmatter_shape.rs), [phase.rs](../../../darkmatter/lib/src/markdown/schemas/phase.rs), and [compose/context/catalog.rs](../../../darkmatter/lib/src/markdown/compose/context/catalog.rs); the CLI's [schema/assignment.rs](../../../darkmatter/cli/src/commands/schema/assignment.rs); and DMLS's [overlay/schema.rs](../../../darkmatter/dmls/src/overlay/schema.rs) and [providers/frontmatter.rs](../../../darkmatter/dmls/src/providers/frontmatter.rs). Audit `schemas/about.rs`, `schemas/detect.rs`, and `schemas/triggers/matcher.rs` for the new variant too; inference or unresolved-trigger paths need not start resolving files. Whole-schema operations visit each definition once and preserve references. Editor lookup descends along the finite cursor path and exposes properties for the next insertion even in an incomplete item. Whole-type hover and catalog display show named back-references instead of infinitely expanding them. Definition navigation retains the defining file and source span.
9. **Phase projection and conversion.** Project cyclic definitions once per validation phase, retaining finite references, and derive effective property presence/timing at each use site. Launch and Completion semantics must agree with the prerequisites even when a definition is shared between eager and deferred properties. The schema-enhancement conversion translator's support for local `$ref` is not proof of arbitrary-depth recursive conversion: its current spike inlines targets up to a fixed translation depth. Replace recursive expansion with reusable reference handles or equivalent instance-directed traversal so declared conversions continue at deeper finite document paths; preserve final validation and atomic failure behavior. This intentionally extends the prerequisite’s validation-only treatment of cyclic shapes for supported recursive named types and the equivalent generated local references. Raw cycles outside that supported shape retain their validation-only behavior. Callable types still cannot appear anywhere reachable from a document schema, including through a recursive reference; recursion does not relax the prerequisite’s schema-load error.
10. **Validator probe.** With the repository's pinned `jsonschema` version (currently 0.55, Draft 2020-12), first compile and validate a self-referencing schema at depth 8 and confirm that a bad nested value is rejected. Include mutual recursion and sibling constraints. This is a correctness test, not a dependency upgrade or performance spike.
11. **Portable fixture generator.** The generator in the spike for `2026-10-20-lifecycle-grammar` hardcodes another worktree and a former feature directory. Derive its repository root from its own location and write to its adjacent `spike/schemas/` directory, independent of launch CWD. Update the generator's usage text and any comments affected by the path change. Keep its other inputs unchanged; do not regenerate the historical measurement baseline from today's catalogs.

**Reader's note:** `$ref` changes the representation of cyclic types intentionally. It does not authorize arbitrary reference-based baseline roots, erase constraints beside a reference, or change which union alternatives may safely transform data. These restrictions preserve the existing merge and mutation contracts while enabling nested types.

### The frozen measurement fixture

The “Frozen fixture size and proportional measurement” acceptance check uses the committed spike schemas, frozen so later catalog changes cannot move the baseline: the committed `spike/schemas/action.yaml` (the 143-verb action table) and depth-5 unrolled `lifecycle.yaml` from `2026-10-20-lifecycle-grammar` are copied byte-for-byte into a Darkmatter test fixture directory, and a hand-written `lifecycle-recursive.yaml` beside them expresses the same grammar with one recursive item type over the same `action.yaml`. Keep that spike's original approximations, rather than introducing the later fully typed Claudine grammar into this comparison. Compare equivalent valid and invalid shapes within the unrolled fixture's supported levels; only the recursive fixture checks deeper levels. The fixtures are test inputs, read by the tests through the repository-read forms the `rust-testing` skill lists.

## Part A: Measurement and validation waste

*(amended 2026-10-16)* Part A needs neither recursion nor
`2026-10-28-schema-enhancements`. It is implemented **first and early**, in
parallel with pipeline steps 15–16 and before lifecycle ownership moves in
step 17, and reviewed on its own. The recursion design above is Part B. The frontmatter prerequisites
gate Part B; they do not prevent Part A from landing independently. Scheduling
is described in
[Claudine’s rollout strategy](../../docs/rollout-strategy.md).

Recursion removes the duplicated size, but it does not remove the work Darkmatter
does around compiling, and today that work dominates. Measured 2026-10-15/16 on
the dev Mac (release `md schema validate --no-baseline-schema --format json`,
hyperfine 15–20 runs, host load 10–30, Claudine's bundle under `SCHEMAS_DIR` in
a scratch repository):

| Case | Mean |
| --- | --- |
| `md --version` (process start) | 7 ms |
| Claudine-shaped document, `--no-trigger-schemas` | 12 ms |
| Document **no trigger matches** (`title: x` only) | **138 ms** |
| Claudine document, shipped `lifecycle.yaml` | 203–263 ms |
| Same, container events typed four conditional levels deep (inlined) | 758 ms, 588 MB |

A `sample` profile of the slowest variant (one run; inclusive shares overlap)
attributed about 60% to `jsonschema` compilation, about 50% to `serde_json`
serializing a schema `Value` for the validator cache's content hash
(`schema_content_hash`, `structural_validator_hashed`), a large share to deep
`BTreeMap<String, Value>` clones and drops, and 10–20% to SimplifiedSchema
resolution and conversion. The profile also observed a validator recompile for
each newly opened document (about 270 ms, attributed at measurement time to the
source-path cache key). DMLS hashes `format!("{registry:?}")` on every request
(about 25 ms,
`dmls/src/overlay/mod.rs`). The existing `just bench-schema` bench uses a
five-field inline schema with a warm cache and sees none of this.

These observations describe the measured tree, not a requirement to repeat
optimizations already present. The current Darkmatter library’s
[ValidatorCache](../../../darkmatter/lib/src/markdown/schemas/validate.rs)
already memoizes shared-schema digests and reuses a structural validator across
documents when the schema judges neither file formats nor `match()` values.
Retain those paths, measure their actual callers, and record an item as already
addressed when appropriate. A schema containing `match()` is context-sensitive
even if it contains no `file` property.

Part A is a first pass and a learning exercise: it measures, removes waste,
and records results. It adds **no** wall-clock test or failing time budget,
because host load alone moved these numbers by tens of milliseconds. It changes
no verdict, diagnostic, completion, or hover.

1. **Stage timings.** Split one validation into `discover`, `match`, `resolve`,
   `compile` (cache hits count as zero), and `validate`. DMLS logs one
   `tracing` event at `warn` when a validation's total exceeds a threshold
   (default 200 ms, configurable in DMLS settings), carrying the document path,
   matched trigger names, each stage's time, and whether `compile` hit the
   cache; faster validations log at `debug`. Use `schema.slow_validation_ms`
   (nonnegative integer milliseconds; zero logs every completed validation at
   `warn`) under the existing DMLS file/editor-overlay precedence. Invalid
   values follow the existing configuration-diagnostic behavior. `md schema validate -vv` prints
   the same breakdown to stderr, using biscuit-terminal components. Preserve
   the CLI verbosity count through dispatch so one `-v` keeps its existing
   behavior and two enable the timing report; do not depend on `--debug` or
   `RUST_LOG` to display it. Stdout and exit codes do not change.

   Measure elapsed durations with a monotonic clock. Stages are non-overlapping
   accumulated work within one request: discovery includes registry freshness
   checks; matching includes matcher validation; resolution includes lowering
   and merge/phase preparation; compilation includes every validator built by
   those stages; validation is instance checking. Extract compilation time
   from its enclosing stage rather than counting it twice. Include total time
   and unassigned overhead; cached stages report zero work. If any validator
   was built, the request is a compile miss; no validator lookup is “not
   requested,” distinct from a hit. Report partial timings on failure without
   replacing the existing error. Do not log document values or schema bodies.
2. **Benchmark.** Add cold-cache Criterion cases to `just bench-schema`: a
   document no trigger matches with a Claudine-shaped bundle present, a
   Claudine document with document-level events only, and one with a nested
   sequence and a group, each with events. Benches read a frozen copy of
   Claudine's schema files under `lib/benches/fixtures/`, never
   `claudine/schemas/`. Before the first change, save a schema-only baseline
   with `just bench-schema --save-baseline recursive-schema-before`; the
   broader `just bench-baseline` recipe runs unrelated benchmarks too. Each cold-cache iteration resets the
   relevant in-process schema, payload, phase, and validator caches outside
   the timed work; ordinary filesystem caches may remain warm. A fresh
   `DarkmatterSchemas` wrapper alone does not make process-wide caches cold.
   Keep the existing warm-cache cases. Part B reuses these cases where applicable.
3. **Waste to investigate and remove.** Each is a hypothesis from the profile:
   confirm it, fix it where the fix is local, or record why not.
    - Unmatched triggers appear fully built: defer payload resolution and
      conversion until a trigger’s `match:` succeeds only where existing error
      behavior permits. Keep trigger discovery, match-definition validation,
      root-read errors, duplicate/precedence rules, and last-good-registry
      behavior unchanged. Compare matched and unmatched malformed payloads
      before and after: if delaying a payload would hide an error currently
      reported for an unmatched trigger, defer that optimization and record
      the reason rather than silently changing the contract.
    - The cache key appears to serialize the whole schema: key it from the
      immutable effective schema, or from a complete input identity, rather
      than serializing it at each lookup. Source-file fingerprints plus the
      authored `$schema` alone are insufficient: they omit embedded sources,
      matched triggers, merge order, baseline selection, and phase projection.
    - Check whether one document builds the same validator more than once
      (structural and context validators, merged arms) and share the build
      where the verdict is identical.
    - Replace whole-schema `Value` clones on the hot path with shared
      references.
    - DMLS: build the registry key from file fingerprints, not a `Debug` dump.
    - DMLS: confirm that opening a second document with the same schema
      reaches the existing structural-validator cache when `digest.judges_files`
      is false. That flag excludes both file formats and `match()`. Fix callers
      that rebuild equivalent schemas or bypass this path; retain context keys
      wherever the verdict depends on them.

    **Cache validity is a correctness requirement.** A reusable identity must
    distinguish authored inline schemas, disk and embedded source identities
    and revisions, transitive imports and example dependencies, selected
    trigger order, baseline/extensions and precedence, phase, and validator
    options. Keep resolution context in resolution/matching caches, including
    ordered search roots and registry membership: creating a nearer schema
    can change resolution even when an old dependency is unchanged. Only the
    compiled structural validator may omit document context when its verdict
    is context-independent. File formats and `match()` below `$ref` or union
    arms must still select the context-sensitive path. A failed dependency
    read must not serve a cached success. Retain the existing registry's
    explicitly supported last-good behavior separately from validator reuse.

    Prefer the existing shared-schema digest memoization when a complete
    input identity would be more complex; it hashes a newly prepared schema
    once and reuses that result. Retain bounded cache lifetimes and the inputs
    that establish identity; do not use a freed allocation’s address as a
    reusable key. Reused validators must not retain another document’s source
    positions: attach diagnostics to the current document on each validation.
    Use `biscuit-hash` for any new hashing.

## Work breakdown

From source, nothing built: **12–18 engineer-days, likeliest 15** for Part B.
Part A adds **3–5 engineer-days** (stage timings and logging 1–1.5, benchmark
cases 0.5–1, waste items 1.5–2.5). The array-of-union lift, the `Arc` type tables, the loader crash fixes, required-preservation, and diagnostic positions are no longer here; they are prerequisites and must be verified before this work begins.

| Work | Days |
|---|---|
| Probe test; resolver: mutually recursive groups, `Ref`, definitions table | 1.5–2.5 |
| Lowering with `$defs` context; keep phase re-lowering working | 1.5–2 |
| Merge and synthetic-root paths: baseline gate, trigger merge, phase merge, sub-schemas compiled alone | 1.5–2 |
| JSON walkers follow references: validate, clean ×2, coerce and the engine translator, rewrite, discriminant | 1.5–2.5 |
| Library and CLI typed-tree walkers | 1–2 |
| DMLS completion, hover, navigation through `Ref`, lazy descent | 1.5–2.5 |
| Tests: unit, byte-identical golden corpus, recursive end-to-end at depth ≥ 6, DMLS below a recursion point, frozen fixture size and the one-host timing measurement, per-walker `$ref` tests | 2.5–3.5 |
| Docs: `definition.md`, `authoring-schemas.md`, `local-type-references.md`, the `dmls-schema-support.md` matrix, `schema-triggers.md`, the `darkmatter` skill; generator path fix and fixture freeze | 1–1.5 |
| CI | 0 |

High-end drivers: mapping a validation error raised inside a `$ref` back to its source line; discriminant narrowing over reference members; replacing bounded inline expansion in the conversion translator.

## Risks

- **Silent degradation in a missed JSON walker.** A walker that does not follow references sees no `properties` and quietly returns nothing. The per-walker reference-coverage acceptance check gives each walker a targeted test through a `$ref`.
- **`$ref` validation performance.** Small schemas must not regress; large ones must not compile 40 MB. The acyclic golden corpus guards output compatibility; the frozen fixture guards size. The quick one-host before/after sample, recorded in the implementation log, makes an unexpected cost visible without creating a timing gate.
- **DMLS cache.** Part B removes the size, not per-keystroke re-resolution. Part A investigates the registry key and any remaining per-document validator recompiles; already-shared validators stay shared. A full resolved-schema cache keyed on `$schema`, trigger registry, and dependency hashes is still not part of this spec; Part A's slow-validation log shows whether it is needed.

## Alternatives considered

- **(i) DMLS resolved-schema cache** (2–3 days). Fixes re-resolution only; first load and every schema edit still pay the duplicated size. Complementary, not a substitute.
- **(ii) `$defs` without recursion** (7–15 days). Removes duplication but not the unroll; adding recursion later costs another 4–7 days at the same consumer sites.

## Out of scope

| Work | Owner |
|---|---|
| Lists of a union-typed name (`items: { anyOf }`) | `2026-10-14-union-type-constraints` (required before this work) |
| `required`/`generated` surviving references; bare local names | `2026-10-13-named-type-presence-and-local-names` |
| Shared type tables behind `Arc` | `2026-10-12-shared-type-tables` |
| Loader crashes on valid YAML spellings | `2026-10-11-schema-loader-yaml-spellings` |
| Diagnostic positions at the offending node | `2026-10-15-diagnostic-node-positions` |
| Switching Claudine's `action.yaml`/`lifecycle.yaml` to recursion and re-deriving its fixtures | `2026-10-30-claudine-recursive-schemas` |
| A depth constraint in SimplifiedSchema | not done (decided 2026-10-10: nesting limits are parser-only) |

## Acceptance criteria

Automated behavior checks are L1 tests run with nextest through `just test` in `darkmatter/`, with `just lint` passing. Documentation review and the one-host timing measurement are recorded in the implementation log; no test focuses a terminal or browser window; every test is portable to macOS, Linux, native Windows, and WSL2; no CI cell is added. The area recipe must include `darkmatter`, `darkmatter-cli`, and `dmls`; run any missing package’s L1/lint through its existing recipe. Cache and editor changes require DMLS coverage, not only library tests.

1. **Byte-identical golden corpus.** Every acyclic schema in the manifest, the built-in baseline, and resolver fixtures lowers to byte-identical serialized JSON, including stable key order. Capture and commit the golden after the prerequisites and before the recursion change. Include the passive shipped-artifact corpus; do not freeze pre-prerequisite behavior that was intentionally changed.
2. **Recursive validation.** A self-referencing named type (directly, and through a two-file cycle) accepts a conforming document at depth ≥ 6 and rejects a non-conforming one there, with the JSON path and the source line and column of the offending node.
3. **DMLS below a recursion point.** Completion and hover return the referenced definition's properties at recursion levels 1, 2, and 3 of a recursive schema, resolved lazily (a test proves lookup follows the cursor path and at most the next editable shape, without expanding unused recursive descendants).
4. **Frozen fixture size and proportional measurement.** `lifecycle-recursive.yaml` over the frozen `action.yaml` resolves to at most 2 MB of serialized JSON, asserted in L1. Assert that increasing the instance depth does not create additional definitions or change the resolved schema. The existing growth figures already establish why unrolling must be removed; no new feasibility spike is needed. Record one quick before/after sample on one host using the release profile and these fixtures, separating resolution from I/O and validator compilation. Record the host description, both timings, and an explanation of any visible regression in the implementation log. The existing growth figures answer the feasibility question; no scaled timing threshold, prescribed repetition count, or OS matrix is required.
5. **Per-walker `$ref` coverage.** Every JSON walker listed under “JSON walkers” in Design (validate, both clean walks, coerce and the conversion translator, rewrite, discriminant) and every synthetic sub-schema compiled alone has a test that exercises it through a `$ref`, and each test fails if the walker skips the dereference.
6. **Presence and a finite terminating case.** Use `item: "{ value: string(required), children: item[](required) }"`, with `children: []` as a valid leaf. A nested item missing `children`, or supplying null, fails at that node (including depth 4); an empty leaf list passes. A separate optional reference accepts missing/null but rejects null list items. Missing optional recursive children remain absent after normalization; traversal does not synthesize an endless child chain. Cover inherited required/generated/eager, nullable unions, use-site descriptions, conflicting constraints, array bounds, and two differently constrained uses of one shared definition. Do not require an endless non-null `next` chain with no valid finite leaf.
7. **Generator portability.** `gen-action-schema.py` contains no hardcoded absolute path and, run from a different CWD in a fresh checkout, writes into its own `spike/schemas/` directory. Its derived runtime paths may be absolute. Preserve the frozen inputs independently of generator output.
8. **Graph and merge safety.** Cover direct recursion, same-file mutual recursion, a two-file cycle, acyclic aliases into a cycle, repeated imports, and distinct inline/root-union/embedded namespaces. Same-named types from different files and identical generated keys from independently lowered roots cannot collide during baseline, trigger, or root-union assembly. Repeated resolution of unchanged inputs yields identical JSON without exposing absolute file paths in generated keys. Missing targets and unsupported merge roots fail with structured errors; existing non-type cycle errors and import-chain limits remain enforced. Include raw local definitions and mixed raw/simplified union arms to detect changed reference scopes. Re-keying must leave literal `$ref`/`$defs` data under `const`, defaults, and examples unchanged. Cover embedded self/mutual recursion without filesystem dependencies; editing or deleting a disk import reached only through a cycle-closing edge must invalidate a cached result.
9. **Phase, conversion, and normal invocation.** Test Launch and Completion presence/timing below recursive references, including a shared definition with different use-site timing. A conversion supported by the schema-enhancement engine still applies past its former translation cutoff within existing document resource limits. Failure leaves input unchanged. Exercise `md schema validate`, `md schema about`, and normal composition with a hermetic recursive fixture; passive validation cannot execute expressions or mutate input, and repeated successful normalization is idempotent. DMLS projects the same positioned validation and schema-resolution errors. A function type reachable only through a recursive definition remains a document schema-load error.
10. **Docs and consumer audit.** Update Darkmatter's schema definition, authoring, local-reference, trigger, and DMLS support pages and its skill. Replace the old ban on recursive types and describe definition/use-site scope, supported cycles, phase handling, and baseline restrictions with compact examples. Docs do not link to feature directories. Audit all typed and JSON consumers for the new variant; add focused behavior coverage where they consume resolved references rather than adding tests for unrelated inference paths.
11. **Child-value boundary rule.** Each of `{ children: item[] }` (array item), an object property, a dictionary value, and a tuple slot is accepted as a recursive edge. `a: b` plus `b: a` (alias cycle), `a: [a, string]` (self-union), and a cycle through a callable signature's parameter type are each rejected with the typed schema error, which names every reference in the cycle with its source span. A graph containing both a descending cycle and a non-descending cycle in the same recursive group is rejected with the error naming only the non-descending cycle's references; removing that cycle makes the same graph resolve. A two-file non-descending cycle reports spans in both files. DMLS projects the error as a schema diagnostic at those spans.
12. **Part A: stage timings.** DMLS logs the slow-validation event with every field listed in Part A, and `md schema validate -vv` prints the stage breakdown. An L1 test drives each with a fixture and asserts the stage names and the presence of timings, not their values. Cover zero/default/invalid timing configuration, a full cache hit, a mixed hit/miss, no requested compilation, and an early failure; verify that disabled verbose output adds nothing to stdout and a single `-v` retains its existing behavior.
13. **Part A: benchmark.** `just bench-schema` includes the three cold-cache cases, reading only frozen fixtures, with process-wide cache reset outside the measured work. Confirm the cold path compiles validators when required and the warm path reuses them; do not assert elapsed times.
14. **Part A: recorded results.** The implementation log records each waste item's disposition (fixed, already fine, or deferred with the reason) and a quick one-host before/after sample from the existing representative benchmark cases, with host load noted. The historical `hyperfine` measurements are context, not a second required workload. Every existing Darkmatter and Claudine schema test passes unchanged. Success is a measured improvement whose causes are understood; no particular time is a criterion.
15. **Part A: cache and trigger behavior.** Focused L1 tests distinguish inline-only schemas, embedded bundles, baseline choices, matched trigger order, and Launch/Completion projections. On the same long-lived cache, change a transitive disk dependency and add/remove a nearer schema candidate; no stale verdict is reused. Two documents with a structural-only schema reuse compilation while diagnostics use the current document’s positions; file and `match()` schemas, including references to them, keep the correct request context. Preserve current discovery and malformed-trigger errors for matched and unmatched triggers. These are behavior checks, not performance thresholds.

## Open questions

None. Cache identity, preservation of errors when payload work is delayed,
and the supported recursive-conversion boundary are decided above.
