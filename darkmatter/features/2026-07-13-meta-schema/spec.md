---
status: ready for review
reviewed: false
review_iterations: 0
rulings: `type-definition` and `schema` semantic types ruled by Ken 2026-07-13
inputs:
  - ../../docs/schemas/darkmatter.yaml
  - ../../docs/topics/schema-definition.md
  - ../../lib/src/markdown/schemas/simplified/types.rs
  - ../../lib/src/markdown/schemas/simplified/grammar.rs
  - ../../lib/src/markdown/schemas/simplified/convert.rs
  - ../../lib/src/markdown/schemas/simplified/serialize.rs
  - ../../lib/src/markdown/schemas/format.rs
  - ../../lib/src/markdown/schemas/resolve.rs
  - ../../dmls/src/providers/frontmatter.rs
related:
  - ../2026-07-12-literal-expression
  - ../_completed/2026-07-08-schema-plus
  - ../_completed/2026-07-04-dmls
---

# SimplifiedSchema Meta-Schema Types

**Status:** Ready for review. This spec introduces two first-class semantic
types for describing SimplifiedSchema syntax as data: `type-definition` for
one property definition and `schema` for a complete document schema
declaration. They replace imprecise `string`, `object`, and `any` annotations
where the value is actually interpreted by Darkmatter's schema grammar.

## Goal

1. Add **`type-definition`**, a grammar-backed type whose value must be one
   valid SimplifiedSchema `PropertyDef`.
2. Add **`schema`**, a grammar-backed type whose value must be one valid
   `$schema` declaration: an inline schema shape, a schema file reference, or
   a root union.
3. Make these semantic types available to schema authors as concise nominal
   shorthands instead of enumerating type-name literals or falling back to
   broad carrier types.
4. Give DMLS an explicit schema signal for completion, hover, diagnostics,
   and semantic classification inside inline and standalone SimplifiedSchema.
5. Replace the Darkmatter base schema's current `$schema: any` declaration
   with `$schema: schema`, so hover reports the language-level contract rather
   than an implementation compromise.

## Motivation

Darkmatter already distinguishes a value's YAML carrier from its semantic
language. An `expression` property is carried by a YAML string, but its value
is parsed under the Darkmatter expression grammar. Calling it merely `string`
would discard the information that validation and DMLS need.

Schema definitions have the same distinction:

```yaml
$schema:
    foo: string(required)
```

There are three separate facts here:

1. The schema entry `foo` contains a **type definition**.
2. Its authored carrier is a YAML string.
3. That particular definition declares that document values of `foo` are
   strings and that the property must be present.

Other definitions use different carriers and denote different instance types:

```yaml
$schema:
    foo:
        - string
        - bar: string
```

Here the definition's carrier is a YAML sequence and its denoted document type
is `string | object`. Neither `string`, `object`, nor `any` accurately names
the semantic role of the schema entry itself. `type-definition` does.

This distinction matters beyond terminology. DMLS currently sees the
Darkmatter-owned `$schema` property as `any`, so its hover faithfully displays
an unhelpful type even though the resolver enforces a precise language. A
grammar-backed type lets the editor activate schema intelligence for the same
reason `expression` activates expression intelligence.

## Terminology and Semantic Layers

This feature uses the following terms consistently:

| Term | Meaning | Existing model |
|------|---------|----------------|
| **Type definition** | One complete definition of a document property | `PropertyDef` |
| **Type expression** | A scalar definition such as `string(required)` | `PropertyAtom` / `TypeExpr` plus constraints |
| **Schema shape** | A mapping of property names to type definitions | `SchemaShape` |
| **Schema declaration** | The complete value accepted at `$schema` | inline `SimplifiedSchema`, file reference, or root union |
| **Denoted type** | The type accepted for the eventual document value | `string`, `object`, a union, and so on |
| **Carrier** | The YAML representation used to author the definition | string, mapping, or sequence |

`type-definition` is a nominal semantic type. It is not the denoted type and
it is not just an enum of primitive type keywords. It names the complete
grammar that produces a `PropertyDef`.

## Feature A — `type-definition`

### Authoring syntax

`type-definition` is a normal SimplifiedSchema type keyword:

```yaml
$schema:
    parameter:
        name: string(required)
        type: type-definition(required)

parameter:
    name: input
    type: string[]
```

It is the concise, extensible replacement for incomplete literal
enumerations such as:

```yaml
type: enum(string,number,boolean,object,any; required)
```

The enum only lists today's bare type names. `type-definition` also recognizes
constraints, arrays, inline objects, imports, literals, expressions, and future
backward-compatible additions to the SimplifiedSchema definition grammar.

### Accepted values

A `type-definition` value accepts exactly the shapes already accepted by the
authoritative property-definition parser:

1. **String type expression**

   ```yaml
   definition: string(required)
   ```

2. **Mapping object definition**

   ```yaml
   definition:
       name: string(required)
       enabled: boolish
   ```

3. **Non-empty sequence union**, whose arms are string type expressions or
   mapping object definitions

   ```yaml
   definition:
       - literal(auto)
       - number(min(1))
       - width: number(required)
   ```

The type does not invent a second grammar. Validation must route through the
same parser used when these shapes appear as property definitions inside an
authored SimplifiedSchema. A syntax accepted in one position and rejected in
the other is a parity bug.

### Semantics

- Validation is **parse-only and side-effect-free**. Imports, `example(...)`
  references, and file-bearing constraints are syntax-checked but are not
  resolved or read. The owning schema resolver remains responsible for I/O
  when a definition is actually used as a schema.
- The value is preserved exactly as authored. There is no native-to-string
  coercion: a mapping stays a mapping and a sequence stays a sequence.
- YAML boolean, number, and null scalars are invalid because none is a valid
  property-definition carrier. Authors quote a scalar when they intend a type
  expression.
- The semantic result is a `PropertyDef`. DMLS and other passive consumers
  should use the shared parse product rather than re-tokenizing the source.

### Constraints and arrays

`type-definition` permits the universal `required`, `default(...)`, and
`generated` constraints. A default must itself be a valid type definition.
All value-domain constraints (`min`, `max`, `pattern`, `suggest`, `eager`, and
so on) are rejected because they constrain the definition's denoted values,
not the definition artifact.

The `type-definition[]` postfix is rejected in v1. A sequence is already the
carrier for one property-level union, so an array of independent definitions
would be ambiguous at the YAML boundary. A concrete driver may introduce an
explicit outer-array form later without overloading the existing sequence.

### AST and public parser surface

- Add `SimplifiedType::TypeDefinition`, canonical keyword
  `"type-definition"`.
- Keep `PropertyDef` as the semantic parse product; do not create an
  isomorphic second AST.
- Extract the existing private property-definition entry point into a public,
  passive parser that accepts a YAML value and returns `PropertyDef` with
  structured `SchemaError` information.
- Span-aware consumers must receive or be able to derive the same authored
  spans used by the schema-source parser. DMLS must not reconstruct ranges
  from decoded strings.

The exact public function name is a planning decision; there must be one
library authority, not a DMLS-only parser.

### JSON Schema lowering

Unlike `expression`, this semantic type accepts native mappings and sequences,
so a string-only JSON Schema `format` is insufficient. The compiler emits a
portable outer shape plus a Darkmatter keyword:

```json
{
  "type": ["string", "object", "array"],
  "x-darkmatter-type-definition": true
}
```

The registered custom keyword validates the complete instance with the shared
property-definition parser. External JSON Schema validators that ignore the
extension still enforce the carrier domain; Darkmatter validators enforce the
full grammar. The normal optional-nullable wrapper remains outside this
fragment, and `required` retains its existing parent-object behavior.

The custom keyword must return ordinary structured validation failures so
`ValidationProblem` retains the instance path, source position, and
constraint-class reporting used by other semantic formats.

## Feature B — `schema`

### Authoring syntax

`schema` is the whole-declaration counterpart to `type-definition`:

```yaml
$schema:
    embedded_schema: schema(required)

embedded_schema:
    title: string(required)
    tags: string[]
```

The Darkmatter base schema uses it directly:

```yaml
$schema:
    "$schema": "schema -> Declares an inline schema, referenced schema file,
      or root union for this document. Document schemas override baseline
      properties on conflict."
```

This replaces the current `any` declaration. Raw JSON Schema remains accepted
through the referenced-file branch; an inline mapping is always interpreted as
SimplifiedSchema, matching the existing resolver.

### Accepted values

A `schema` value accepts exactly the existing `$schema` declaration shapes:

1. **Inline schema shape** — a mapping of property names to
   `type-definition` values.
2. **File reference** — a string accepted by
   `biscuit_file::FileReference`, subject to the existing local-only schema
   reference policy.
3. **Non-empty root union** — a sequence whose arms are inline schema shapes
   or file-reference strings.

```yaml
# Inline shape
$schema:
    title: string(required)

# Referenced SimplifiedSchema or raw JSON Schema
$schema: ./schemas/post.yaml

# Root union
$schema:
    - ./schemas/spec.yaml
    - kind: literal(review)
      findings: object(required)
```

Boolean, number, null, empty sequence, and invalid union arms fail the semantic
type even though `any` previously allowed them at the baseline-schema layer.
This is not a new runtime restriction: the `$schema` resolver already rejects
those shapes before document validation.

### Resolution boundary

The `schema` type validates a declaration; it does not resolve it:

- Inline shapes are parsed fully and recursively.
- File-reference strings are syntax-checked through `FileReference` but are
  not opened, discovered, or fetched.
- Root-union file arms follow the same syntax-only rule.
- Remote references retain the existing `$schema` policy and are rejected.

Actual `$schema` preparation continues through `resolve_schema_with_roots`,
which performs file loading, bare-name schema-root lookup, import expansion,
example validation, and raw-JSON-Schema disambiguation. A user property typed
`schema` never causes validation or DMLS to perform I/O.

### AST and JSON Schema lowering

- Add `SimplifiedType::Schema`, canonical keyword `"schema"`.
- Introduce a passive schema-declaration parse/classification surface that
  reuses `parse_yaml_schema` for inline mappings and root-union mapping arms,
  plus `FileReference` for reference syntax. It must not duplicate the
  resolving loader.
- Compile to the portable carrier domain plus a registered semantic keyword:

  ```json
  {
    "type": ["string", "object", "array"],
    "x-darkmatter-schema": true
  }
  ```

- `schema[]` is rejected for the same reason as `type-definition[]`: a
  sequence already denotes one root union.
- Constraint applicability and coercion match `type-definition`.

## Relationship Between the Types

The model is recursive but not circular in implementation:

```text
TypeDefinition = TypeExpression
               | SchemaShape
               | PropertyUnion

SchemaShape    = map<string, TypeDefinition>

PropertyUnion  = non-empty list<TypeExpression | SchemaShape>

Schema         = SchemaShape
               | FileReference
               | RootUnion

RootUnion      = non-empty list<SchemaShape | FileReference>
```

`type-definition` validates one node in the schema language. `schema`
validates the root declaration and its special file-reference forms. A
`SchemaShape` is shared by both, but the sequence grammars differ:
property-union string arms are type expressions, while root-union string arms
are file references.

That distinction must remain explicit. A single catch-all "schema-ish" parser
would incorrectly interpret the same string arm in two different contexts.

## Meta-Schema Status

These two grammar-backed types form Darkmatter's **semantic meta-schema**. No
parallel, hand-maintained JSON document may redefine the SimplifiedSchema
grammar. The Rust parser remains authoritative, while the emitted
`x-darkmatter-*` keywords expose that authority through compiled JSON Schema.

This feature supersedes the narrow ruling in schema-plus O-C2 that no
`type-expr` meta-primitive would be introduced. The earlier ruling correctly
kept `Name@file` imports as structural inline expansion and prevented a second
type system from being built for parameter metadata. Those decisions remain.
What changes is that Darkmatter now has a concrete editor and self-description
driver for naming the existing `PropertyDef` grammar as a semantic value type.

`type-definition` does not participate in import expansion and does not replace
named types. It describes a definition artifact when that artifact appears as
document data.

## DMLS Behavior

The main consumer benefit is schema-driven editor intelligence. DMLS must use
the effective schema's semantic type; it must not activate these behaviors from
key-name heuristics, except that the reserved `$schema` control key is always
known by the Darkmatter language itself.

### Hover

Hover on the document control key becomes:

```md
**`$schema`**

Type: **schema**

Declares an inline schema, referenced schema file, or root union for this
document. Document schemas override baseline properties on conflict.
```

Within a schema shape, hover distinguishes the artifact's semantic role from
the type it denotes. Given:

```yaml
$schema:
    foo:
        - string
        - bar: string
```

hover for `foo` reports, in equivalent wording:

```md
Type: **type-definition**

Declares: **string | object**
```

For `foo: string(required)`, `Declares` is `string` and the parsed constraint
summary includes `Required`. The existing schema-hover renderer must render
all union arms; using the first arm as a representative is not sufficient for
a type definition.

### Completion

Inside a `type-definition` value, completion is driven by the existing typed
descriptor catalogs and parser state:

- type keywords from `schema_type_descriptors()`;
- constraints valid for the selected type;
- `[]`, inline-object, union, and `Name@file` scaffolds where syntactically
  valid;
- referenced named types when the containing schema has an available passive
  namespace.

Inside a `schema` value, completion offers the appropriate outer scaffolds and
then delegates each inline property value to `type-definition` completion.
File-reference completion uses the existing passive path-completion machinery.

Completion must support inline `$schema` blocks and standalone
SimplifiedSchema documents recognized by `parse_standalone_schema_document`.
Tagged schema documents (`kind: schema`, `types:`) receive the same intelligence
inside their `types` mapping. Raw JSON Schema files remain outside this
provider.

### Diagnostics

- Invalid property definitions produce a dedicated
  `dm.schema.invalid_type_definition` diagnostic at the smallest reliable
  authored range.
- Invalid outer declarations retain `dm.schema.invalid_schema_shape`.
- File-reference syntax and resolution remain distinguishable: the semantic
  type can report invalid syntax without I/O; the existing preparation path can
  separately report an unresolved file.
- Specialized diagnostics replace, rather than duplicate, a generic custom-
  keyword constraint failure for the same span.
- All analysis is passive. Schema intelligence must not load references,
  expand imports, validate examples, compose directives, execute expressions,
  or access the network on a keystroke.

### Semantic tokens

The deferred frontmatter semantic-token family gains two schema-driven
classifications:

- a complete property definition may be classified as a semantic type;
- type keywords, constraints, literal values, import names, and file-reference
  portions may receive finer tokens in a later phase.

Fine-grained meta-schema semantic tokens are not required for this feature.
The spec records the typed activation signal so a future token phase does not
rediscover schema regions heuristically.

## Validation, Composition, and Trigger Behavior

- Both types are parse-only. They never execute or resolve their contents.
- Compose-time schema validation may validate user properties typed with these
  types, but it preserves their original YAML representation and performs no
  normalization.
- Pending `$(...)` and `{{ }}` values follow the existing `PendingPolicy`
  behavior before semantic parsing.
- Both types are pure and may be used in trigger match schemas for ordinary
  document properties. The reserved `$schema` control key remains absent from
  the trigger-matching instance, as today.
- `md schema detect` never infers either type. A string that happens to parse as
  a type definition or file reference is still inferred from its carrier type;
  semantic intent cannot be inferred reliably.
- `md schema about` and the public descriptor catalogs list both types,
  including their accepted carriers, constraints, parse-only behavior, and
  DMLS meaning.

## Compatibility and Migration

This is an additive grammar change: `type-definition` and `schema` become
reserved type keywords in type-expression position. Existing schemas using
those words as imported named-type identifiers must rename those helpers.

The base-schema edit from:

```yaml
"$schema": "any -> Declares an inline schema, referenced schema file, root
  union, or raw JSON Schema for this document."
```

to:

```yaml
"$schema": "schema -> Declares an inline schema, referenced schema file, or
  root union for this document."
```

does not narrow real accepted behavior. Invalid scalar shapes already fail in
the resolver. It aligns the authored baseline, compiled schema metadata, DMLS
hover, and runtime contract.

The description must stop implying that raw JSON Schema can be authored as an
inline mapping. Raw JSON Schema is supported through a referenced YAML or JSON
file and remains a resolver concern.

Schemas that currently enumerate type names may migrate when they intend the
full definition grammar:

```yaml
# Before: closed and immediately stale when the vocabulary grows
type: enum(string,number,boolean,date,time,datetime,any; required)

# After: grammar-backed and automatically follows compatible additions
type: type-definition(required)
```

This migration is opt-in. An enum remains correct when the consumer genuinely
accepts only a closed subset of bare names rather than arbitrary definitions.

## Implementation Surface Map

| Surface | Required change |
|---------|-----------------|
| `simplified/types.rs` | Add `TypeDefinition` and `Schema` variants and canonical keywords |
| `simplified/grammar.rs` | Accept the new keywords; expose the existing `PropertyDef` parser through one passive public entry point |
| `simplified/serialize.rs` | Canonically serialize both keywords and reject `[]` postfix |
| `simplified/convert.rs` | Emit carrier types plus `x-darkmatter-type-definition` / `x-darkmatter-schema` |
| `format.rs` / validator construction | Register custom keyword validators backed by shared passive parsers |
| `coerce.rs` / normalization | Explicitly preserve all accepted carriers; no semantic-type coercion |
| `triggers/matcher.rs` | Pure parse-based matching for both variants |
| `about.rs` | Add authoritative descriptors used by `md schema about` and DMLS |
| `resolve.rs` | Reuse declaration classification without moving I/O into semantic validation |
| `docs/schemas/darkmatter.yaml` | Retype `$schema` from `any` to `schema` and correct raw-JSON wording |
| `docs/topics/schema-definition.md` | Document the semantic meta-types and carrier/denoted-type distinction |
| DMLS frontmatter/overlay providers | Hover, completion, diagnostics, standalone-schema activation, and union-aware declared-type rendering |
| Darkmatter skill | Record the two types and the passive parser authority |

## Non-Goals

- No expression or schema evaluation.
- No file loading, import expansion, example loading, or raw JSON Schema
  inspection as part of semantic-type validation.
- No replacement for `PropertyDef`, `SchemaShape`, `SimplifiedSchema`, named
  types, or `Name@file` structural expansion.
- No general recursive user-defined types. The meta-types describe the
  existing bounded grammar; they do not remove its recursion limits.
- No static type-checking of expressions against a `type-definition`.
- No inference of semantic types from ordinary strings, mappings, or
  sequences.
- No requirement to implement fine-grained meta-schema semantic tokens in this
  feature.
- No change to raw JSON Schema authoring or resolution rules.

## Acceptance Criteria

1. `type-definition` is a canonical SimplifiedSchema keyword backed by
   `SimplifiedType::TypeDefinition`; it round-trips through parse/serialize and
   appears in the descriptor catalog and `md schema about`.
2. It accepts scalar type expressions, nested mapping object definitions, and
   non-empty property unions exactly when the existing `PropertyDef` parser
   accepts them. Invalid scalars, empty unions, invalid arms, and malformed
   constraints fail with structured validation problems.
3. `schema` is a canonical keyword backed by `SimplifiedType::Schema`; it
   accepts inline schema shapes, valid local `FileReference` strings, and
   non-empty root unions of shapes/references without performing I/O.
4. `schema` rejects invalid scalar shapes, remote references, empty root
   unions, and invalid arms with the same semantic rules as `$schema`
   preparation, excluding resolution/existence failures.
5. Both compiled fragments expose `string | object | array` as their portable
   carrier domain and register grammar-backed custom keywords. Validation-only
   callers are not mutated, and compose write-back preserves the authored
   representation.
6. `type-definition[]` and `schema[]` fail at schema-load time with an error
   explaining that sequence syntax already represents a union.
7. The public passive parser is the shared authority for schema authoring,
   custom-keyword validation, and DMLS. Parser-parity tests prove representative
   definitions cannot diverge between those entry points.
8. The Darkmatter base schema declares `$schema: schema`; existing valid inline,
   referenced, root-union, and referenced-raw-JSON documents continue to
   prepare successfully, while DMLS hover displays `Type: schema` instead of
   `Type: any`.
9. DMLS provides parser-state completion and precise diagnostics inside inline
   and standalone SimplifiedSchema. Hover identifies entries as
   `type-definition` and renders the complete denoted union rather than only
   its first arm.
10. DMLS analysis of these values is side-effect-free: no reference loading,
    import/example expansion, composition, expression execution, shell
    execution, or network access occurs.
11. Existing schemas that do not use the two new reserved keywords parse,
    compile, validate, and render diagnostics byte-identically except for the
    intentional `$schema` hover/type metadata correction.
12. L1 and L2 coverage passes through `just test` and `just test-l2`; the
    implementation remains portable across macOS, Windows, and Linux.

## Ruled Design Questions

- **Q1 — Is a definition merely its YAML carrier type?** **Ruled no
  (2026-07-13).** A definition has the nominal semantic type
  `type-definition`; its carrier and denoted document type are separate facts.
- **Q2 — Is this just an enum of primitive names?** **Ruled no
  (2026-07-13).** It is shorthand for the complete existing `PropertyDef`
  grammar, so it remains accurate as compatible definition forms are added.
- **Q3 — What types the complete `$schema` value?** **Ruled `schema`
  (2026-07-13).** `type-definition` describes one property definition;
  `schema` describes the outer inline/reference/root-union declaration.
- **Q4 — Does the meta-type replace structural imports or create a second
  type system?** **Ruled no (2026-07-13).** Existing ASTs, parsers, and
  structural `Name@file` expansion remain authoritative. The feature names and
  validates those artifacts when they appear as data.
