---
area: darkmatter
status: draft-spec
created: 2026-09-28
owner: Ken Snyder <ken@ken.net>
depends-on:
    - 2026-09-21-lifecycle-ergonomics
related:
    - 2026-09-21-lifecycle-ergonomics
---

# Recursive Named Types in SimplifiedSchema

SimplifiedSchema is Darkmatter's YAML schema dialect for Markdown frontmatter. A
file may declare a `types:` table of **named types** and reference one from any
property as `Name@file` (`@this` for the current file, `@./x.yaml` for a
sibling). Every reference is **inlined**: the resolver copies the type's body
into the referencing property before lowering to JSON Schema, so the output
never contains `$ref` or `$defs`
(`darkmatter/docs/topics/schemas/definition.md:727`), and a type that
transitively references itself is a hard error (`SchemaError::ImportCycle`,
`darkmatter/lib/src/markdown/schemas/resolve.rs:1134-1140`). This spec records
the decision to lift that restriction and an effort estimate produced from
source.

## Problem

The clarification of `2026-09-21-lifecycle-ergonomics` needed a schema for a
grammar that is recursive by nature: a list item may carry a `then`/`else` body
that is itself a list of the same items. With no self-reference, the only
expressible schema **unrolls** the recursion to a fixed depth N with N named
types, and since each reference is inlined, every level duplicates every level
below it. The spike measured the cost
(`claudine/features/2026-09-21-lifecycle-ergonomics/spike-results.md:73-87`,
Finding 14 at `:162-166`; host under load, upper bounds):

| depth | resolved JSON | resolve | `md schema validate` | inlined copies of one 143-key table |
|---|---|---|---|---|
| 1 | 1.9 MB | 109 ms | 234 ms | 42 |
| 3 | 9.5 MB | 657 ms | 1.1 s | 210 |
| 5 | 39.9 MB | 2.7 s | 5.9 s | 882 |

Copies per event = 2^(N+2) − 2, times seven consumers; size and time roughly
double per level. The cost is entirely duplication, and no cache helps: DMLS
keys its bundle on full document text, so every keystroke re-resolves (Finding
15, `spike-results.md:167-170`; `darkmatter/dmls/src/overlay/mod.rs:384-411`),
and each expansion clones the whole type table (`resolve.rs:1175`, `:1240`,
`:1252`).

Recursion was consciously deferred: `2026-07-08-schema-plus` closed open item
O-B3 as "named types form a DAG; a self-referential type is a recursion error in
v1; true recursive/reference types deferred" (`:140-142`, `:398-401`). No design
existed until this estimate.

## Goal

A named type may reference itself, directly or through other named types, and
such a schema resolves in size and time proportional to the schema, not to the
depth of the documents it validates. Every existing schema that does not recurse
lowers to byte-identical JSON Schema.

## Non-goals

- New surface grammar. Recursion uses the existing `Name@this` spelling.
- `$defs` for acyclic types. Inlining stays the default; see Alternatives.
- Honoring `required` on a reused named type (Finding 10): a separate half-day
  decision.
- CI changes: pure Rust, L1 coverage.

## Design

1. **Reference graph.** The resolver builds the graph of named types across
   files and finds strongly connected components. `MAX_IMPORT_DEPTH`
   (`resolve.rs:899`) and the frame stack in `expand_import`
   (`resolve.rs:1102-1157`) become the cycle finder rather than the cycle
   rejecter.
2. **Acyclic types inline exactly as today**, so every existing schema lowers
   byte-identically. A golden test proves it over the nine real schema files
   using `@this`/`@./` (under `claudine/docs/schemas/`,
   `darkmatter/docs/schemas/`, `darkmatter/schemas/`, `messenger/docs/`), the
   built-in baseline, and the resolver fixtures.
3. **`TypeExpr::Ref(DefKey)`.** `TypeExpr`
   (`schemas/simplified/types.rs:183-200`) gains a variant keyed on canonical
   file path + type name. A type in a cycle becomes a `Ref`; so does a
   union-typed name used with `[]`, which `apply_import_postfix` rejects today
   (`resolve.rs:1350-1358`, Finding 4): a union cannot be inlined as one array
   item but can be referenced as one.
4. **Definitions table.** Referenced definitions travel on `ResolvedSchema`,
   `EffectiveSchema`, and the DMLS bundle. Shared type tables move behind an
   `Arc`.
5. **Lowering.** `to_json_schema` (`simplified/convert.rs:79-96`) emits
   `{"$ref": "#/$defs/<key>"}` for a `Ref` plus a root `$defs` map, with
   `required`/`generated` markers stripped as inlining strips them today. The
   unresolved-import rejection (`convert.rs:348-359`) stays; phase re-lowering
   (`schemas/phase.rs:32`, `:90-147`) carries the context through.
6. **Merge gates.** `validate_simple_object_schema` (`resolve.rs:1847-1885`) and
   `enforce_merge_compatible` (`schemas/triggers/assemble.rs:251-271`) reject
   keys outside a fixed allow-list; they are relaxed to carry and combine
   `$defs`, keys namespaced by file so collisions cannot occur.
7. **JSON walkers** get one `deref(root, node)` helper: `validate.rs` (`:932`,
   `:998-1004`), `clean.rs` (`:491`, `:636-643`), `coerce.rs` (`:48`,
   `:151-240`), `rewrite.rs` (`:175`, `:209`, `:281-289`), `discriminant.rs`
   (`:107`). The four synthetic sub-schemas compiled alone
   (`validate.rs:761-806`, `coerce.rs`, `rewrite.rs`) must receive the root
   `$defs` or they fail to compile.
8. **Typed-tree walkers** use `defs.resolve(atom)`: `completion.rs:86-236`,
   `frontmatter_shape.rs:259-310`, `phase.rs`, `compose/context/catalog.rs`, the
   CLI's `schema/assignment.rs`, and in DMLS `overlay/schema.rs` (`:753`,
   `:800`, `:970`) and `providers/frontmatter.rs` (`:1526`, `:1766-1775`,
   `:1970-1974`). DMLS descends lazily, bounded by the document's own nesting.
9. **Validator.** `jsonschema 0.55.0` (Draft 2020-12,
   `darkmatter/lib/Cargo.toml:128`) supports recursive local `$ref`. A
   quarter-day probe test comes first.

## Work breakdown

From source, nothing built: **13–20 engineer-days, likeliest 16**; +1–1.5 days
for the array-of-union lift (item 3), cheap here and expensive alone. Twenty-two
consumer sites in fourteen files.

| Work | Days |
|---|---|
| Resolver: SCCs, `Ref`, definitions table, `Arc` for shared type tables | 2–3 |
| Lowering with `$defs` context; keep phase re-lowering working | 1.5–2 |
| Merge and synthetic-root paths: baseline gate, trigger merge, phase merge, four sub-schemas compiled alone | 1.5–2 |
| JSON walkers deref: validate, clean ×2, coerce, rewrite, discriminant | 1.5–2.5 |
| Library and CLI typed-tree walkers | 1–2 |
| DMLS completion, hover, navigation through `Ref` | 1.5–2.5 |
| Tests: unit, byte-identical golden corpus, recursive end-to-end at depth ≥ 6, DMLS below a recursion point, size/time regression | 2.5–3.5 |
| Docs: `definition.md`, `authoring-schemas.md`, `local-type-references.md`, `dmls-schema-support.md` matrix, `schema-triggers.md`, the `darkmatter` skill | 1–1.5 |
| CI | 0 |

High-end drivers: full DMLS completion and hover below a recursion point instead
of opaque-first; mapping a validation error raised inside a `$ref` back to its
property and source line; discriminant narrowing over reference members; and
whether a recursive schema can enforce a nesting limit (it cannot; the limit
becomes parser-only for consumers).

## Risks

- **Silent degradation in a missed JSON walker.** A walker that does not deref
  sees no `properties` and quietly returns nothing. Each walker needs a targeted
  test through a `$ref`.
- **`$ref` validation performance.** Small schemas must not regress; large ones
  must not compile 40 MB. The regression criterion guards both.
- **DMLS cache unchanged.** The bundle stays keyed on document text; this
  removes the size, not per-keystroke re-resolution (Alternative i).

## Alternatives considered

- **(i) DMLS resolved-schema cache** keyed on `$schema` + trigger registry +
  dependency hashes, 2–3 days. Fixes re-resolution only; first load and every
  schema edit still pay the duplicated size. Complementary, not a substitute.
- **(ii) `$defs` without recursion**, 7–10 days lean or 11–15 with full DMLS.
  Removes duplication but not the unroll, and a poor waypoint: adding recursion
  later costs another 4–7 days at the same consumer sites.

## Related defects

Non-blocking; tracked here until filed as fixes.

- **Finding 4** (`spike-results.md:121-126`): `Name[]@file` over a union-typed
  named type is rejected (`resolve.rs:1350-1358`). Lifted by item 3.
- **Finding 8** (`:137-141`): diagnostics carry the JSON path of the offending
  node but the line/column of the top-level key.
- **Finding 10** (`:144-148`): `required` is stripped from a reused named type;
  direct `expression(required)` in a mapping type is kept. Deliberate; reversing
  it is ~0.5 day.
- **Finding 11** (`:149-154`): indentless block sequences and plain scalars
  folded across lines crash the standalone loader ("could not project
  SimplifiedSchema expression spans").

## Pre-work recommended regardless

- Share type tables behind an `Arc` instead of cloning per expansion
  (`resolve.rs:1175`, `:1240`, `:1252`), ~0.5 day; directly cuts resolve time
  for the interim unrolled shape.
- Fix the Finding 11 loader crashes; they will bite the first hand-edited
  schema.

## Sequencing

Runs **after** `2026-09-21-lifecycle-ergonomics`, which is its `depends-on`.
That feature ships Claudine's lifecycle schemas in an interim unrolled shape:
three fully typed levels, a loose list-or-map below them, and the generated
side-effect and expression-function verbs admitted through a single-key
catch-all. It also leaves behind the generator (`gen-action-schema.py` in its
spike directory) and the spike fixtures this feature measures against.

Updating that schema to take advantage of recursion is a task of **this**
feature, not of the lifecycle work: once the resolver lands, this feature
switches Claudine's generator to a recursive `item[]@this` item type, restores
full typing of the generated verbs, removes the unroll, and re-derives the
fixtures. Claudine's nesting limit of five then lives only in its parser; the
schema no longer encodes a depth.

## Acceptance criteria

1. **Byte-identical golden corpus.** The nine-file corpus, the built-in
   baseline, and the resolver fixtures lower to JSON identical to pre-change
   output.
2. **Recursive validation.** A self-referencing named type accepts a conforming
   document at depth ≥ 6 and rejects a non-conforming one there with a path to
   the offending node.
3. **DMLS below a recursion point.** Completion and hover return the referenced
   definition's properties at least two recursion levels deep.
4. **Size/time regression.** The lifecycle grammar from
   `2026-09-21-lifecycle-ergonomics`, fully typed with recursion, resolves to ≤
   2 MB in ≤ 150 ms warm on the spike host (Apple M4 Max,
   `spike-results.md:10-21`), against 39.9 MB / 2.7 s unrolled to depth 5.
5. **Claudine schema switched.** `claudine/schemas/action.yaml` and
   `lifecycle.yaml` are regenerated in the recursive shape with the generated
   verbs fully typed and no unrolled levels; the Claudine spike fixtures,
   the DMLS parity test, and `md schema validate` over the shipped prompt
   corpus pass; the Claudine `docs/` schema pages and the `claudine` skill
   describe the recursive shape and drop the interim wording.
5. Every JSON walker in Design item 7 has a test that exercises it through a
   `$ref`.

## Open questions

- **Nesting limit: schema or parser?** A recursive schema cannot bound depth.
  Either the consumer's parser owns the limit alone, or SimplifiedSchema grows a
  depth constraint on `Ref` that lowers to an unrolled `$defs` chain. The
  estimate assumes parser-only.
- **Opaque-first DMLS.** Treat a `Ref` as opaque in completion and hover on
  first landing (low end) and add lazy descent later, or ship full descent at
  once (high end)?

