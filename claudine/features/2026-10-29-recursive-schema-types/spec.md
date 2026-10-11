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
    implemented: boolean -> indicates whether this spec's plan has been implemented
    implemented_by: string -> the agent who implemented the plan
    formerly: string -> the spec's previous `{date}-{name}`, when it was renamed in the 2026-10-10 restructure
    pipeline_step: number -> position in the claudine/docs/rollout-strategy.md pipeline
reviewed: false
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

SimplifiedSchema is Darkmatter's YAML schema dialect for Markdown frontmatter. A file declares a `types:` table of **named types** and references one from any property: by bare name for a type in the same file (`author: person`, since `2026-10-13-named-type-presence-and-local-names`), or as `Name@file` for a type in another file. Every reference is **inlined**: the resolver copies the type's body into the referencing property before lowering to JSON Schema, so the output never contains `$ref` or `$defs` (`darkmatter/docs/topics/schemas/definition.md`, "composition … is the type system"), and a type that transitively references itself is a hard error (`SchemaError::ImportCycle`, raised in `ImportEngine::expand_import`, `darkmatter/lib/src/markdown/schemas/resolve.rs` ~:1089 and ~:1112 at the time of writing).

This spec lifts that restriction in the resolver: a named type may reference itself, and such a schema resolves in size and time proportional to the schema, not to the depth of the documents it validates. An author can finally describe a grammar that nests — a list item whose `then` body is a list of the same items — without unrolling it by hand. Switching Claudine's lifecycle schemas to recursion is `2026-10-30-claudine-recursive-schemas`.

## Problem

A grammar that is recursive by nature (Claudine's `when`/`then`/`else` lifecycle items) can only be expressed today by **unrolling** the recursion to a fixed depth N with N named types, and since each reference is inlined, every level duplicates every level below it. The schema spike measured the cost (`claudine/features/2026-10-20-lifecycle-grammar/spike-results.md`, "Resolved JSON Schema size and resolve time", Finding 14; host under load, upper bounds):

| depth | resolved JSON | resolve | `md schema validate` | inlined copies of one 143-key table |
|---|---|---|---|---|
| 1 | 1.9 MB | 109 ms | 234 ms | 42 |
| 3 | 9.5 MB | 657 ms | 1.1 s | 210 |
| 5 | 39.9 MB | 2.7 s | 5.9 s | 882 |

Copies per event = 2^(N+2) − 2, times seven consumers; size and time roughly double per level. The cost is entirely duplication, and no cache helps: DMLS keys its bundle on full document text, so every keystroke re-resolves (Finding 15; `darkmatter/dmls/src/overlay/mod.rs`).

Recursion was consciously deferred: `2026-07-08-schema-plus` closed open item O-B3 as "named types form a DAG; a self-referential type is a recursion error in v1; true recursive/reference types deferred".

## Decided

- **Recursion is allowed.** A named type may reference itself directly, or through other named types in the same file or in other files. A cycle is no longer an error; it lowers to a JSON Schema `$ref` into a root `$defs` map.
- **No new surface grammar.** Recursion uses the existing reference spellings: a bare local name, `@this`, or `Name@file`.

  ```yaml
  kind: schema
  types:
      item: |-
          { say: string, when: expression, then: item[], else: item[] }
  ```

- **Acyclic types are unchanged.** Every schema that does not recurse lowers to byte-identical JSON Schema; inlining stays the default.
- **Depth is not a schema concern** (decided 2026-10-10). A recursive schema cannot bound nesting depth, and SimplifiedSchema gains no depth constraint. A consumer that needs a nesting limit enforces it in its own parser only.
- **Editor descent** (decided 2026-10-10). DMLS completion and hover descend through a recursive reference lazily, resolving one level at a time as the cursor's document position requires, and reach at least two recursion levels deep. A recursive reference is never shown as opaque.
- **Presence markers ride the reference.** `required` and `generated` on a property whose type is a recursive reference apply to that property exactly as `2026-10-13-named-type-presence-and-local-names` established for inlined references; they are never stripped.
- **Validation errors stay positioned.** A validation error raised inside a `$ref` target maps back to the offending document node's JSON path and source line/column, as `2026-10-15-diagnostic-node-positions` requires for inlined schemas.

## Design

Line numbers are as of 2026-10-10 and will drift as steps 1–5 of the pipeline land; the symbol names are authoritative.

1. **Reference graph.** The resolver builds the graph of named types across files and finds strongly connected components. `MAX_IMPORT_DEPTH` (`resolve.rs` ~:875) and the frame stack in `ImportEngine::expand_import` (~:1078) become the cycle finder rather than the cycle rejecter. `MAX_IMPORT_DEPTH` keeps bounding acyclic import chains.
2. **Acyclic types inline exactly as today**, so every existing schema lowers byte-identically. A golden test proves it over every real schema file that uses named-type references (under `claudine/schemas/`, `claudine/docs/schemas/`, `darkmatter/docs/schemas/`, `darkmatter/schemas/`, `messenger/docs/`, and `content-policy/schemas/`), the built-in baseline, and the resolver fixtures. The corpus is enumerated by a test that finds every `kind: schema` file with a reference, so a file added later joins it.
3. **`TypeExpr::Ref(DefKey)`.** `TypeExpr` (`schemas/simplified/types.rs` ~:183) gains a variant keyed on canonical file identity + type name (the same identity `NamespaceKey` gives a file, including the embedded-origin identity the built-in baseline uses). Only a type that belongs to a cycle becomes a `Ref`. A list of a union-typed name is **not** a `Ref`: `2026-10-14-union-type-constraints` already lowers it inline to `items: { anyOf }`; when that union is itself part of a cycle, the `Ref` wraps the whole union and the list-of-union representation is unchanged.
4. **Definitions table.** Referenced definitions travel on `ResolvedSchema`, `EffectiveSchema`, and the DMLS bundle, sharing the `Arc`-backed type tables that `2026-10-12-shared-type-tables` introduced.
5. **Lowering.** `to_json_schema` (`simplified/convert.rs` ~:81) emits `{"$ref": "#/$defs/<key>"}` for a `Ref` and a root `$defs` map holding each referenced definition once. Presence markers apply at the referencing property per the Decided section. The unresolved-import rejection in the converter (`TypeExpr::Imported` arm, ~:386) stays. Phase re-lowering (`schemas/phase.rs`) carries the `$defs` context through.
6. **Merge gates.** `validate_simple_object_schema` (`resolve.rs` ~:1815) and `enforce_merge_compatible` (`schemas/triggers/assemble.rs` ~:274) reject keys outside a fixed allow-list; they are relaxed to carry and combine `$defs` and `$ref`, with `$defs` keys namespaced by file identity so two files' definitions can never collide. The baseline's "no `$ref`" load rule (`definition.md`, baseline section) admits a local `$ref` into the merged `$defs` and nothing else.
7. **JSON walkers** get one `deref(root, node)` helper that follows a local `$ref` to its `$defs` entry: `schemas/validate.rs`, `schemas/clean.rs` (both `items` walks), `schemas/coerce.rs` (and the coercion engine's JSON Schema translator from `2026-10-28-schema-enhancements`, which already translates local `$ref`/`$defs`), `schemas/rewrite.rs`, and `schemas/discriminant.rs`. Every synthetic sub-schema compiled on its own (in `validate.rs`, `coerce.rs`, `rewrite.rs`) receives the root `$defs`, or it fails to compile.
8. **Typed-tree walkers** use `defs.resolve(atom)`: `schemas/completion.rs`, `schemas/frontmatter_shape.rs`, `schemas/phase.rs`, `compose/context/catalog.rs`, the CLI's `commands/schema/assignment.rs`, and in DMLS `overlay/schema.rs` (`effective_shape`, `load_extension_schema`) and `providers/frontmatter.rs` (`known_shape` and its completion and hover paths). DMLS descends lazily, bounded by the document's own nesting.
9. **Validator.** `jsonschema` 0.55 (Draft 2020-12, `darkmatter/lib/Cargo.toml`) supports recursive local `$ref`. A probe test that compiles and validates a self-referencing schema at depth 8 lands first, before any other change.
10. **The spike generator's paths.** `claudine/features/2026-10-20-lifecycle-grammar/spike/gen-action-schema.py` hardcodes another worktree (`REPO = Path("/Volumes/coding/wt/rusty-biscuit/feat-schema-enhancement")`) and an output directory under the lifecycle spec's former name. It is fixed to derive the repository root from its own location and to write into its own `spike/schemas/` directory, so it runs from any checkout on any OS. Its other inputs are unchanged here.

### The frozen measurement fixture

Criterion 4 measures against the committed spike schemas, frozen so later catalog changes cannot move the baseline: `claudine/features/2026-10-20-lifecycle-grammar/spike/schemas/action.yaml` (the 143-verb action table) and the depth-5 unrolled `lifecycle.yaml` are copied byte-for-byte into a Darkmatter test fixture directory, and a hand-written `lifecycle-recursive.yaml` beside them expresses the same grammar with one recursive item type over the same `action.yaml`. The fixtures are test inputs, read by the tests through the repository-read forms the `rust-testing` skill lists.

## Work breakdown

From source, nothing built: **12–18 engineer-days, likeliest 15**. The array-of-union lift, the `Arc` type tables, the loader crash fixes, required-preservation, and diagnostic positions are no longer here; they landed in steps 1–5.

| Work | Days |
|---|---|
| Probe test; resolver: SCCs, `Ref`, definitions table | 1.5–2.5 |
| Lowering with `$defs` context; keep phase re-lowering working | 1.5–2 |
| Merge and synthetic-root paths: baseline gate, trigger merge, phase merge, sub-schemas compiled alone | 1.5–2 |
| JSON walkers deref: validate, clean ×2, coerce and the engine translator, rewrite, discriminant | 1.5–2.5 |
| Library and CLI typed-tree walkers | 1–2 |
| DMLS completion, hover, navigation through `Ref`, lazy descent | 1.5–2.5 |
| Tests: unit, byte-identical golden corpus, recursive end-to-end at depth ≥ 6, DMLS below a recursion point, frozen fixture size/time, per-walker `$ref` tests | 2.5–3.5 |
| Docs: `definition.md`, `authoring-schemas.md`, `local-type-references.md`, the `dmls-schema-support.md` matrix, `schema-triggers.md`, the `darkmatter` skill; generator path fix and fixture freeze | 1–1.5 |
| CI | 0 |

High-end drivers: mapping a validation error raised inside a `$ref` back to its source line; discriminant narrowing over reference members.

## Risks

- **Silent degradation in a missed JSON walker.** A walker that does not deref sees no `properties` and quietly returns nothing. Criterion 5 gives each walker a targeted test through a `$ref`.
- **`$ref` validation performance.** Small schemas must not regress; large ones must not compile 40 MB. Criteria 1 and 4 guard both.
- **DMLS cache unchanged.** The bundle stays keyed on document text; this removes the size, not per-keystroke re-resolution. A resolved-schema cache keyed on `$schema`, trigger registry, and dependency hashes is complementary and not part of this spec.

## Alternatives considered

- **(i) DMLS resolved-schema cache** (2–3 days). Fixes re-resolution only; first load and every schema edit still pay the duplicated size. Complementary, not a substitute.
- **(ii) `$defs` without recursion** (7–15 days). Removes duplication but not the unroll; adding recursion later costs another 4–7 days at the same consumer sites.

## Out of scope

| Work | Owner |
|---|---|
| Lists of a union-typed name (`items: { anyOf }`) | `2026-10-14-union-type-constraints` (landed earlier) |
| `required`/`generated` surviving references; bare local names | `2026-10-13-named-type-presence-and-local-names` |
| Shared type tables behind `Arc` | `2026-10-12-shared-type-tables` |
| Loader crashes on valid YAML spellings | `2026-10-11-schema-loader-yaml-spellings` |
| Diagnostic positions at the offending node | `2026-10-15-diagnostic-node-positions` |
| Switching Claudine's `action.yaml`/`lifecycle.yaml` to recursion and re-deriving its fixtures | `2026-10-30-claudine-recursive-schemas` |
| A depth constraint in SimplifiedSchema | not done (decided 2026-10-10: nesting limits are parser-only) |

## Acceptance criteria

All criteria are L1 tests run with nextest through `just test` in `darkmatter/`, with `just lint` passing; no test focuses a terminal or browser window; every test is portable to macOS, Linux, native Windows, and WSL2; no CI cell is added.

1. **Byte-identical golden corpus.** Every schema file in the enumerated corpus, the built-in baseline, and the resolver fixtures lower to JSON identical to the pre-change output (captured before the resolver change and committed as the golden).
2. **Recursive validation.** A self-referencing named type (directly, and through a two-file cycle) accepts a conforming document at depth ≥ 6 and rejects a non-conforming one there, with the JSON path and the source line and column of the offending node.
3. **DMLS below a recursion point.** Completion and hover return the referenced definition's properties at recursion levels 1, 2, and 3 of a recursive schema, resolved lazily (a test proves the definition is not expanded beyond the document's own nesting).
4. **Size and time against the frozen fixture.** `lifecycle-recursive.yaml` over the frozen `action.yaml` resolves to at most 2 MB of JSON. Its warm resolve time, and the frozen unrolled depth-5 `lifecycle.yaml`'s warm resolve time, are measured in the same session on one host (release profile, median of 5); the recursive time must not exceed 150 ms × (that host's unrolled time ÷ 2,683 ms), the spike's 150 ms target scaled to the host. The size bound is asserted in an L1 test; the timing comparison is a measurement recorded in the implementation log with the host description, not a CI assertion.
5. **Per-walker `$ref` coverage.** Every JSON walker in Design item 7 (validate, both clean walks, coerce and the engine translator, rewrite, discriminant) and every synthetic sub-schema compiled alone has a test that exercises it through a `$ref`, and each test fails if the walker skips the dereference.
6. **Presence on a reference.** `next: item(required)` where `item` is recursive makes `next` required at every level; omitting it at depth 4 fails validation at that node.
7. **Generator portability.** `gen-action-schema.py` contains no absolute path and, run from a fresh checkout, writes into its own `spike/schemas/` directory.

## Open questions

None.
