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
reviewed_on: 2026-10-11
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
6. **Merge boundaries.** The library's [validate_simple_object_schema](../../../darkmatter/lib/src/markdown/schemas/resolve.rs) and [enforce_merge_compatible](../../../darkmatter/lib/src/markdown/schemas/triggers/assemble.rs) protect property-based baseline and trigger merging. Admit `$defs` and supported local references inside property schemas while retaining the simple object root: do not admit a root `$ref`, root union, or other root applicator merely by extending the allow-list. Keep document-over-baseline and trigger precedence unchanged. When combining definition tables, re-key colliding generated keys and all affected generated references; never merge unrelated bodies by last-writer-wins. The new admission of raw `$defs` at these merge boundaries is limited to checked `#/$defs/…` references without `$id` changes to their scope or external targets. Isolate their keys too. This does not tighten existing acyclic property schemas that were already accepted. Raw document schemas retain their existing Draft 2020-12 semantics and are not blindly rewritten as generated definitions. Root-union assembly, mixed raw/simplified arms, baseline merge into each arm, and synthetic sub-schemas must preserve each reference's original scope.
7. **JSON walkers.** Add shared reference lookup used by the library's [validate.rs](../../../darkmatter/lib/src/markdown/schemas/validate.rs), [clean.rs](../../../darkmatter/lib/src/markdown/schemas/clean.rs), [coerce.rs](../../../darkmatter/lib/src/markdown/schemas/coerce.rs), [rewrite.rs](../../../darkmatter/lib/src/markdown/schemas/rewrite.rs), and [discriminant.rs](../../../darkmatter/lib/src/markdown/schemas/discriminant.rs). Lookup resolves a reference without discarding sibling assertions or annotations. Missing targets are schema errors, never an unconstrained schema. Instance walkers may revisit a definition at a new instance path; a global visited-definition set would skip valid deeper children. Guard repeated visits to the same schema location at the same instance location instead. Synthetic validators receive the applicable root definitions and scope. Preserve each consumer's existing union policy: closest-arm reporting cannot authorize coercion, cleanup, or eager-file rewriting. Validation remains passive; composition performs only its existing authorized transformations in the captured request context.
8. **Typed-tree walkers and editor behavior.** Resolve references through the definition table in the library's [completion.rs](../../../darkmatter/lib/src/markdown/schemas/completion.rs), [frontmatter_shape.rs](../../../darkmatter/lib/src/markdown/schemas/frontmatter_shape.rs), [phase.rs](../../../darkmatter/lib/src/markdown/schemas/phase.rs), and [compose/context/catalog.rs](../../../darkmatter/lib/src/markdown/compose/context/catalog.rs); the CLI's [schema/assignment.rs](../../../darkmatter/cli/src/commands/schema/assignment.rs); and DMLS's [overlay/schema.rs](../../../darkmatter/dmls/src/overlay/schema.rs) and [providers/frontmatter.rs](../../../darkmatter/dmls/src/providers/frontmatter.rs). Audit `schemas/about.rs`, `schemas/detect.rs`, and `schemas/triggers/matcher.rs` for the new variant too; inference or unresolved-trigger paths need not start resolving files. Whole-schema operations visit each definition once and preserve references. Editor lookup descends along the finite cursor path and exposes properties for the next insertion even in an incomplete item. Whole-type hover and catalog display show named back-references instead of infinitely expanding them. Definition navigation retains the defining file and source span.
9. **Phase projection and conversion.** Project cyclic definitions once per validation phase, retaining finite references, and derive effective property presence/timing at each use site. Launch and Completion semantics must agree with the prerequisites even when a definition is shared between eager and deferred properties. The schema-enhancement conversion translator's support for local `$ref` is not proof of arbitrary-depth recursive conversion: its current spike inlines targets up to a fixed translation depth. Replace recursive expansion with reusable reference handles or equivalent instance-directed traversal so declared conversions continue at deeper finite document paths; preserve final validation and atomic failure behavior.
10. **Validator probe.** With the repository's pinned `jsonschema` version (currently 0.55, Draft 2020-12), first compile and validate a self-referencing schema at depth 8 and confirm that a bad nested value is rejected. Include mutual recursion and sibling constraints. This is a correctness test, not a dependency upgrade or performance spike.
11. **Portable fixture generator.** The generator in the spike for `2026-10-20-lifecycle-grammar` hardcodes another worktree and a former feature directory. Derive its repository root from its own location and write to its adjacent `spike/schemas/` directory, independent of launch CWD. Update the generator's usage text and any comments affected by the path change. Keep its other inputs unchanged; do not regenerate the historical measurement baseline from today's catalogs.

**Reader's note:** `$ref` changes the representation of cyclic types intentionally. It does not authorize arbitrary reference-based baseline roots, erase constraints beside a reference, or change which union alternatives may safely transform data. These restrictions preserve the existing merge and mutation contracts while enabling nested types.

### The frozen measurement fixture

The “Frozen fixture size and proportional measurement” acceptance check uses the committed spike schemas, frozen so later catalog changes cannot move the baseline: the committed `spike/schemas/action.yaml` (the 143-verb action table) and depth-5 unrolled `lifecycle.yaml` from `2026-10-20-lifecycle-grammar` are copied byte-for-byte into a Darkmatter test fixture directory, and a hand-written `lifecycle-recursive.yaml` beside them expresses the same grammar with one recursive item type over the same `action.yaml`. Keep that spike's original approximations, rather than introducing the later fully typed Claudine grammar into this comparison. Compare equivalent valid and invalid shapes within the unrolled fixture's supported levels; only the recursive fixture checks deeper levels. The fixtures are test inputs, read by the tests through the repository-read forms the `rust-testing` skill lists.

## Work breakdown

From source, nothing built: **12–18 engineer-days, likeliest 15**. The array-of-union lift, the `Arc` type tables, the loader crash fixes, required-preservation, and diagnostic positions are no longer here; they are prerequisites and must be verified before this work begins.

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
- **`$ref` validation performance.** Small schemas must not regress; large ones must not compile 40 MB. The acyclic golden corpus guards output compatibility; the frozen fixture guards size. The one-host timing measurement against a re-baselined target, recorded in the implementation log, catches an unexpected cost without creating a CI timing gate.
- **DMLS cache unchanged.** The bundle stays keyed on document text; this removes the size, not per-keystroke re-resolution. A resolved-schema cache keyed on `$schema`, trigger registry, and dependency hashes is complementary and not part of this spec.

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

Automated behavior checks are L1 tests run with nextest through `just test` in `darkmatter/`, with `just lint` passing. Documentation review and the one-host timing measurement are recorded in the implementation log; no test focuses a terminal or browser window; every test is portable to macOS, Linux, native Windows, and WSL2; no CI cell is added.

1. **Byte-identical golden corpus.** Every acyclic schema in the manifest, the built-in baseline, and resolver fixtures lowers to byte-identical serialized JSON, including stable key order. Capture and commit the golden after the prerequisites and before the recursion change. Include the passive shipped-artifact corpus; do not freeze pre-prerequisite behavior that was intentionally changed.
2. **Recursive validation.** A self-referencing named type (directly, and through a two-file cycle) accepts a conforming document at depth ≥ 6 and rejects a non-conforming one there, with the JSON path and the source line and column of the offending node.
3. **DMLS below a recursion point.** Completion and hover return the referenced definition's properties at recursion levels 1, 2, and 3 of a recursive schema, resolved lazily (a test proves lookup follows the cursor path and at most the next editable shape, without expanding unused recursive descendants).
4. **Frozen fixture size and proportional measurement.** `lifecycle-recursive.yaml` over the frozen `action.yaml` resolves to at most 2 MB of serialized JSON, asserted in L1. Assert that increasing the instance depth does not create additional definitions or change the resolved schema. The existing growth figures already establish why unrolling must be removed; no new feasibility spike is needed. Timing is re-baselined on one host: in the same session, with the release profile and the median of 5 warm resolves, measure the recursive fixture and the frozen unrolled depth-5 `lifecycle.yaml`, reporting resolve cost separately from I/O and validator compilation. The recursive time must not exceed 150 ms × (that host's unrolled time ÷ 2,683 ms), the spike's 150 ms target scaled to the host. Record the host description, both timings, and the result in the implementation log. The timing target is a recorded measurement, not an L1 or CI assertion, and needs no OS matrix.
5. **Per-walker `$ref` coverage.** Every JSON walker listed under “JSON walkers” in Design (validate, both clean walks, coerce and the conversion translator, rewrite, discriminant) and every synthetic sub-schema compiled alone has a test that exercises it through a `$ref`, and each test fails if the walker skips the dereference.
6. **Presence and a finite terminating case.** Use `item: "{ value: string(required), children: item[](required) }"`, with `children: []` as a valid leaf. A nested item missing `children`, or supplying null, fails at that node (including depth 4); an empty leaf list passes. A separate optional reference accepts missing/null but rejects null list items. Cover inherited required/generated/eager, nullable unions, use-site descriptions, conflicting constraints, array bounds, and two differently constrained uses of one shared definition. Do not require an endless non-null `next` chain with no valid finite leaf.
7. **Generator portability.** `gen-action-schema.py` contains no hardcoded absolute path and, run from a different CWD in a fresh checkout, writes into its own `spike/schemas/` directory. Its derived runtime paths may be absolute. Preserve the frozen inputs independently of generator output.
8. **Graph and merge safety.** Cover direct recursion, same-file mutual recursion, a two-file cycle, acyclic aliases into a cycle, repeated imports, and distinct inline/root-union/embedded namespaces. Same-named types from different files and identical generated keys from independently lowered roots cannot collide during baseline, trigger, or root-union assembly. Repeated resolution of unchanged inputs yields identical JSON without exposing absolute file paths in generated keys. Missing targets and unsupported merge roots fail with structured errors; existing non-type cycle errors and import-chain limits remain enforced. Include raw local definitions and mixed raw/simplified union arms to detect changed reference scopes.
9. **Phase, conversion, and normal invocation.** Test Launch and Completion presence/timing below recursive references, including a shared definition with different use-site timing. A conversion supported by the schema-enhancement engine still applies past its former translation cutoff within existing document resource limits. Failure leaves input unchanged. Exercise `md schema validate`, `md schema about`, and normal composition with a hermetic recursive fixture; passive validation cannot execute expressions or mutate input, and repeated successful normalization is idempotent. DMLS projects the same positioned validation and schema-resolution errors.
10. **Docs and consumer audit.** Update Darkmatter's schema definition, authoring, local-reference, trigger, and DMLS support pages and its skill. Replace the old ban on recursive types and describe definition/use-site scope, supported cycles, phase handling, and baseline restrictions with compact examples. Docs do not link to feature directories. Audit all typed and JSON consumers for the new variant; add focused behavior coverage where they consume resolved references rather than adding tests for unrelated inference paths.
11. **Child-value boundary rule.** Each of `{ children: item[] }` (array item), an object property, a dictionary value, and a tuple slot is accepted as a recursive edge. `a: b` plus `b: a` (alias cycle), `a: [a, string]` (self-union), and a cycle through a callable signature's parameter type are each rejected with the typed schema error, which names every reference in the cycle with its source span. A graph containing both a descending cycle and a non-descending cycle in the same recursive group is rejected with the error naming only the non-descending cycle's references; removing that cycle makes the same graph resolve. A two-file non-descending cycle reports spans in both files. DMLS projects the error as a schema diagnostic at those spans.

## Open questions

None.
