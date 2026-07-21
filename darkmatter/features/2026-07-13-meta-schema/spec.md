---
status: ready for planning and implementation
reviewed: true
review_iterations: 10
reviewed_by: codex/default
reviewed_on: 2026-07-14
rulings: "`type-definition` and `schema` semantic types ruled by Ken 2026-07-13"
inputs:
  - ../../docs/schemas/darkmatter.yaml
  - ../../docs/topics/schema-definition.md
  - ../../lib/src/markdown/schemas/simplified/types.rs
  - ../../lib/src/markdown/schemas/simplified/grammar.rs
  - ../../lib/src/markdown/schemas/simplified/convert.rs
  - ../../lib/src/markdown/schemas/simplified/serialize.rs
  - ../../lib/src/markdown/schemas/simplified/source.rs
  - ../../lib/src/markdown/schemas/simplified/standalone.rs
  - ../../lib/src/markdown/schemas/format.rs
  - ../../lib/src/markdown/schemas/validate.rs
  - ../../lib/src/markdown/schemas/resolve.rs
  - ../../lib/src/markdown/schemas/about.rs
  - ../../dmls/src/overlay/mod.rs
  - ../../dmls/src/providers/frontmatter.rs
related:
  - ../_completed/2026-07-12-literal-expression
  - ../_completed/2026-07-08-schema-plus
  - ../_completed/2026-07-04-dmls
---

# SimplifiedSchema Meta-Schema Types

**Status:** Ready for planning and implementation. This spec introduces two
first-class semantic types for describing SimplifiedSchema syntax as data:
`type-definition` for one property definition and `schema` for a complete
document schema declaration. They replace imprecise `string`, `object`, and
`any` annotations where the value is actually interpreted by Darkmatter's
schema grammar.

> **Reader's note (2026-07-14 review):** The draft rejected
> `type-definition[]` and `schema[]` because their item type can itself use a
> sequence carrier. The reviewed design allows both. The declared outer `[]`
> already disambiguates an array of semantic values from one union-valued
> semantic value, preserves SimplifiedSchema's established "arrays of every
> type" rule, and avoids parser/serializer exceptions. This review also makes
> the passive source-map contract and recursive-parse limit explicit.

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

   Mapping definitions include the existing schema-object grammar: literal and
   pattern keys, recursively nested property definitions, and the reserved
   `$constraints` metadata entry. `$constraints` is structural authoring
   metadata, not a property whose value is itself a `type-definition`.

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

- Validation is **parse-only and side-effect-free**. Imported-type,
  `example(...)`, and other file-bearing reference text is retained exactly as
  the authoritative property-definition parser retains it; it is not resolved,
  opened, expanded, or subjected to resolver-only `FileReference` policy.
  Tightening that subsyntax belongs in the shared property-definition parser,
  never only in this semantic type. The owning schema resolver remains
  responsible for I/O and reference preparation when a definition is actually
  used as a schema.
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
Because SimplifiedSchema's `default(...)` grammar accepts only one scalar
argument, v1 can express only a scalar type-expression default; compound
mapping and sequence defaults remain unrepresentable, as they are for every
other type. All value-domain constraints (`min`, `max`, `pattern`, `suggest`,
`eager`, and so on) are rejected because they constrain the definition's
denoted values, not the definition artifact.

`type-definition[]` follows the normal array rule and means an array of
independent property definitions. The outer `[]` is declared in the schema, so
the carrier is unambiguous:

```yaml
$schema:
    one_union: type-definition
    many_definitions: type-definition[]

one_union: [string, number]
many_definitions:
    - string
    - number
    - [literal(auto), number]
```

The first flat sequence is one property union. The second flat sequence is an
array containing two scalar definitions and one union definition. Standard
array-level constraints remain available on the postfix surface.

### AST and public parser surface

- Add `SimplifiedType::TypeDefinition`, canonical keyword
  `"type-definition"`.
- Keep `PropertyDef` as the semantic parse product; do not create an
  isomorphic second AST.
- Extract the existing private property-definition entry point into a public,
  passive parser that accepts a YAML value and returns `PropertyDef` with
  structured `SchemaError` information.
- Add a source-aware companion that returns the same `PropertyDef` plus a
  sidecar source map keyed by structural schema paths. The sidecar records the
  authored spans of mapping keys, complete definitions, atoms, type keywords,
  constraints, arguments, import names/references, and union arms. It projects
  through plain, single-quoted, and double-quoted YAML scalars without
  normalizing line endings, using the existing `yaml_scalar` projection seam.
  Do not create an isomorphic spanned AST.
- The source-aware schema-declaration parser reuses the same sidecar model and
  adds outer declaration/file-reference spans. DMLS consumes these maps for
  hover, completion context, and diagnostics; it must not search decoded text
  to reconstruct ranges.

The exact public type and function names are planning decisions. There must be
one grammar authority with semantic-only and source-aware entry points, not a
DMLS-only parser.

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
`ConstraintViolation` classification used by other semantic formats. The
keyword's schema path identifies `x-darkmatter-type-definition`; DMLS uses that
identity plus the source-aware parser to replace the generic problem with its
more precise diagnostic rather than adding a new public
`ValidationProblemCode` variant.

The normal generic array lowering wraps this fragment under `items` for
`type-definition[]`; the custom keyword always validates exactly the item value
on which it appears.

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
- File-reference strings are trimmed, rejected when they use the resolver's
  unsupported HTTP(S) forms, and syntax-checked through
  `biscuit_file::FileReference`; they are not resolved, opened, discovered, or
  fetched. Bare schema-root names remain valid declaration syntax even though
  their existence cannot be established without discovery.
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
  plus the resolver's shared local-reference classifier and `FileReference`
  construction for reference syntax. Extract the HTTP(S), bare-name, and
  path-qualified classification policy from `resolve.rs`; do not duplicate it
  in a custom keyword or DMLS.
- Compile to the portable carrier domain plus a registered semantic keyword:

  ```json
  {
    "type": ["string", "object", "array"],
    "x-darkmatter-schema": true
  }
  ```

- `schema[]` follows ordinary array lowering and means an array of independent
  schema declarations. A root union stored as one array element uses a nested
  sequence. The declared outer array, not carrier guessing, determines which
  sequence is the collection boundary.
- Constraint applicability and coercion match `type-definition`.

As with `type-definition`, `x-darkmatter-schema` failures retain ordinary
`ConstraintViolation` classification and a distinguishing schema path. DMLS
replaces the generic diagnostic using the shared source-aware declaration
parser.

## Relationship Between the Types

The model is recursive but not circular in implementation:

```text
TypeDefinitionValue = TypeExpression
                    | SchemaShape
                    | PropertyUnion

SchemaShape    = map<PropertyKey, TypeDefinitionValue>
               + optional structural $constraints metadata

PropertyKey    = literal property key | pattern/dictionary key

PropertyUnion  = non-empty list<TypeExpression | SchemaShape>

SchemaValue    = SchemaShape
               | LocalSchemaReference
               | RootUnion

RootUnion      = non-empty list<SchemaShape | LocalSchemaReference>

Array<T>       = list<T> when the declaring atom carries the [] postfix
```

`type-definition` validates one node in the schema language. `schema`
validates the root declaration and its special file-reference forms. A
`SchemaShape` is shared by both, but the sequence grammars differ:
property-union string arms are type expressions, while root-union string arms
are file references.

That distinction must remain explicit. A single catch-all "schema-ish" parser
would incorrectly interpret the same string arm in two different contexts.
Likewise, the semantic parser validates one `TypeDefinitionValue` or
`SchemaValue`; generic array lowering is responsible for iterating values when
the declaring atom carries `[]`.

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

### Activation and source model

DMLS has two explicit activation paths:

1. In Markdown frontmatter, a value activates from its effective
   `PropertyDef` containing a `type-definition` or `schema` atom. The reserved
   `$schema` control value also activates from the Darkmatter base language
   contract while an effective schema is being prepared.
2. In standalone YAML, content-based pure or tagged envelopes activate through
   `parse_standalone_schema_document`. While an open buffer is temporarily
   malformed, a lexical envelope claim (a sole top-level `$schema` key or
   top-level `kind: schema`) retains the last-good parsed schema/source map and
   exposes current parser errors. File extension or directory location alone
   never activates SimplifiedSchema intelligence, so raw JSON Schema and
   ordinary YAML remain outside this provider.

`DocumentOverlay` must carry the parsed standalone schema and its source map;
the existing `SuggestionState::Standalone` marker is not a sufficient semantic
model. The last-good behavior mirrors frontmatter's current last-good AST
contract so completion and hover do not disappear during an incomplete edit.
The current buffer always owns diagnostics, and stale semantic data must not be
used to claim that malformed current text is valid.

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
For `type-definition[]` and `schema[]`, completion first identifies the outer
array item at the cursor and then applies the scalar semantic completion to that
item; nested sequences remain valid union-valued items.

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
- Scalar grammar errors use their projected token span. Mapping/sequence shape
  errors use the smallest key, value, or arm span available from the sidecar;
  only a missing structural element falls back to its parent mapping span.
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

### Parse limits and failure behavior

Semantic validation processes untrusted document values, so recursion limits
are part of the contract rather than an implementation detail:

- Reuse `MAX_INLINE_OBJECT_DEPTH` as the shared maximum across both string-form
  inline objects and YAML-native mapping definitions. Root-declaration parsing
  starts at depth zero; every nested schema object increments the same counter,
  including mapping arms nested beneath property unions.
- Array iteration and union parsing remain linear in the supplied YAML value.
  No resolver, filesystem, network, compose, or expression work is permitted
  from a custom keyword.
- Exceeding the shared depth limit returns a structured `SchemaError::Grammar`
  and a normal validation failure; it must never panic or overflow the stack.

Applying the existing depth limit to YAML-native mappings intentionally rejects
pathological schemas deeper than the documented grammar limit. Ordinary
accepted schemas are unaffected, and using one limit prevents semantic-type
validation from becoming a less-bounded second parser.

## Compatibility and Migration

This is an additive grammar change: `type-definition` and `schema` become
reserved primitive keywords only when they occupy the complete primitive-type
position. Existing imported named types do **not** need renaming:
`schema@file`, `type-definition@file`, and their postfix forms remain imports
because the established grammar recognizes terminal `Name@file` syntax before
primitive-keyword lookup. Named definitions are mapping keys and are not
otherwise reserved.

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

does not narrow the set of documents accepted by complete `$schema`
preparation. Invalid scalar shapes already fail in the resolver. It does make
validation-only use of the Darkmatter base schema reject malformed `$schema`
declarations at the validation stage instead of accepting them until a later
resolver call. That earlier failure and its grammar-specific message are
intentional; tests must distinguish acceptance parity for valid declarations
from the changed failure stage for invalid declarations. The edit aligns the
authored baseline, compiled schema metadata, DMLS hover, and runtime contract.

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
| `simplified/grammar.rs` / `simplified/mod.rs` | Accept the new keywords; expose the existing `PropertyDef` parser; apply the shared depth limit to YAML-native mappings |
| `simplified/source.rs` | Generalize the existing scalar projection seam into the structural sidecar source map used by both passive parsers |
| `simplified/serialize.rs` | Canonically serialize both keywords, including ordinary `[]` postfix forms |
| `simplified/convert.rs` | Emit carrier types plus `x-darkmatter-type-definition` / `x-darkmatter-schema` |
| `format.rs` / `validate.rs` | Register custom keyword validators backed by shared passive parsers; preserve keyword schema paths for diagnostic specialization |
| `coerce.rs` / normalization | Explicitly preserve all accepted carriers; no semantic-type coercion |
| `triggers/matcher.rs` | Pure parse-based matching for both variants |
| `about.rs` | Add authoritative descriptors used by `md schema about` and DMLS |
| `resolve.rs` | Extract and reuse local schema-reference classification without moving I/O into semantic validation |
| `docs/schemas/darkmatter.yaml` | Retype `$schema` from `any` to `schema` and correct raw-JSON wording |
| `docs/topics/schema-definition.md` | Document the semantic meta-types and carrier/denoted-type distinction |
| DMLS overlay/frontmatter providers | Retain standalone parsed schema/source maps, hover, completion, precise diagnostics, last-good standalone activation, and union-aware declared-type rendering |
| Darkmatter skill | Record the two types and the passive parser authority |

## Non-Goals

- No expression or schema evaluation.
- No file loading, import expansion, example loading, or raw JSON Schema
  inspection as part of semantic-type validation.
- No replacement for `PropertyDef`, `SchemaShape`, `SimplifiedSchema`, named
  types, or `Name@file` structural expansion.
- No general recursive user-defined types. The meta-types describe the
  existing bounded grammar; they align YAML-native mappings with its recursion
  limit rather than removing that limit.
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
6. `type-definition[]` and `schema[]` parse, serialize, and validate through
   ordinary array lowering. Flat outer sequences are arrays of independent
   semantic values; a union-valued item uses a nested sequence. Item and array
   constraints retain the established postfix semantics.
7. The public passive parsers are the shared authority for schema authoring,
   custom-keyword validation, and DMLS. Their source-aware companions return
   the same semantic AST plus structural sidecar spans. Parser-parity and span-
   projection tests prove representative definitions cannot diverge across
   plain/double/single-quoted YAML, CRLF, UTF-8, nested mappings, and unions.
8. The Darkmatter base schema declares `$schema: schema`; existing valid inline,
   referenced, root-union, and referenced-raw-JSON documents continue to
   prepare successfully, while DMLS hover displays `Type: schema` instead of
   `Type: any`.
9. DMLS provides parser-state completion and precise diagnostics inside inline
   and standalone SimplifiedSchema. Standalone activation is content-based,
   retains last-good semantic data during malformed edits, and never activates
   ordinary YAML or raw JSON Schema from filename alone. Hover identifies
   entries as `type-definition` and renders the complete denoted union rather
   than only its first arm.
10. DMLS analysis of these values is side-effect-free: no reference loading,
    import/example expansion, composition, expression execution, shell
    execution, or network access occurs.
11. Schema parsing and semantic validation enforce the same
    `MAX_INLINE_OBJECT_DEPTH` across string-form and YAML-native nested objects;
    over-limit input returns a structured error without panic or stack
    overflow.
12. Existing schemas that do not use the two new reserved keywords parse,
    compile, validate, and render diagnostics byte-identically except for the
    intentional `$schema` hover/type metadata correction and earlier
    validation-only rejection of malformed `$schema` declarations. Existing
    imports named `schema` or `type-definition` continue to parse.
13. L1 and L2 coverage passes through `just test` and `just test-l2`; the
    implementation remains portable across macOS, Windows, and Linux.
    Review cycle 9 restored the canonical Level-2 gate to 90/90 by restaging
    the three terminal-mode tests below on tmux; no scope exception is needed.

### AC13 Level-2 gate closure

**Status: RESOLVED — the canonical gate is green and no exception is needed.**

Review cycle 9 restaged the three affected tests on a focused tmux runner,
where OSC-11 is unanswered and the staged `COLORFGBG` value remains
authoritative. Their existing luma and foreground assertions were not changed:

- `level2_code_block_inverts_to_light_in_dark_terminal`
- `level2_default_code_block_inverts_background_and_foreground`
- `level2_code_block_clears_inherited_dim_before_theme_colors`

The final canonical `just test-l2 --no-fail-fast` run passed Darkmatter 18/18,
Darkmatter CLI 69/69, and DMLS 3/3 — 90/90 total. The focused runner continues
to execute the Cargo-built `md` through the compile-time `CARGO_BIN_EXE_md`
shim, and hosts without tmux skip through the standard Level-2 gate.

#### Historical pre-repair diagnosis

Before review cycle 9, `just test` was green while `just test-l2` was red on
exactly three tests, all in
`darkmatter/cli/tests/level2_code_block_styling.rs`:

- `level2_code_block_inverts_to_light_in_dark_terminal`
- `level2_default_code_block_inverts_background_and_foreground`
- `level2_code_block_clears_inherited_dim_before_theme_colors`

The proposed exception covered **these three named tests only**. It was never
ratified and is now obsolete because the tests and the full gate pass.

**Why they were outside meta-schema execution paths.** This feature changes
schema parsing, lowering, validation, and DMLS semantic paths. Before the
review-cycle-9 test repair, `git diff main...HEAD` reported zero changes under
`darkmatter/lib/src/markdown/render/`, and
`darkmatter/cli/tests/level2_code_block_styling.rs` was byte-identical to
`main`. The three failures reproduced on `main` and were pre-existing.

Corrected 2026-07-20 (review-4 implementation): this paragraph previously also
claimed zero changes under `darkmatter/lib/src/markdown/highlighting/`. That is
no longer accurate — the unrelated perf commit `864521fae` ("borrow syntax
themes and write escapes directly") touches `highlighting/{mod,prose,themes}.rs`
on this branch. It is not a meta-schema change and it is not the cause: the
observed failure is a *dark* panel (luma 44) where a light one is required,
i.e. the terminal was detected light despite `COLORFGBG='15;0'`. That is the
staging mechanism described below, not a theme-resolution regression.

**Why they are a test-staging defect, not a product defect.** All three assert
the same contract — a dark terminal must render a light (inverted) code panel —
and all three stage that terminal with `COLORFGBG='15;0'` under the **WezTerm**
harness. That staging no longer works. `query_osc_color_with_timeout`
(`biscuit-terminal/lib/src/discovery/osc_queries/query.rs`) attempts a live
OSC-11 query first for `TerminalApp::Wezterm` when no multiplexer is present,
and returns on success — so `COLORFGBG` is never consulted in a WezTerm pane.
Under tmux the OSC query is skipped and `COLORFGBG` is honored. The same file
already documents this for the mirror direction and abandoned its
light-terminal test for exactly this reason; the dark direction has since
become invalid the same way.

**Evidence the contract itself is green.** The inversion rule is verified at
two independent levels that both pass:

- L1: `resolve_for_surface_inverts_default_dark_terminal_to_light_panel` and
  `resolve_for_surface_inverts_default_light_terminal_to_dark_panel`
  (`darkmatter/lib/src/markdown/highlighting/resolve.rs`).
- L2 under the tmux harness, with exact OneHalf RGB assertions in both
  directions: `level2_schema_about_dark_terminal_uses_light_code_theme` and
  `level2_schema_about_light_terminal_uses_dark_code_theme`.
- L2 in the library tier: `level2_page_code_panel_is_contiguous_inverted_rectangle`.

**Evidence the meta-schema-relevant L2 slices pass** (2026-07-19, this host):

- Darkmatter library L2 — 18/18 passed.
- `schema about` CLI L2 — 3/3 passed.
- DMLS L2 — 3/3 passed.
- Canonical `just test-l2`, `--no-fail-fast`: library 18/18; CLI 66/69;
  DMLS 3/3 (run separately — the canonical run aborts in the CLI tier before
  reaching it). Total 87/90, with the only 3 failures being those named above.

**Reconfirmed after the `level2_errors` repair** (2026-07-20, review-6
implementation): library 18/18, CLI 66/69, DMLS 3/3 — total 87/90, unchanged.
The 8 `level2_errors` tests now execute `CARGO_BIN_EXE_md`, and the exception's
scope is once again exactly coextensive with the remaining failure set: the
three named code-block tests and no others. Each L2 tier is run through
`just _test_l2 <crate>` because the CLI tier's failure aborts the canonical
recipe before it reaches DMLS. Note that `just test-l2 -- --no-fail-fast` is
rejected by the harness; the flag must be passed bare as
`just test-l2 --no-fail-fast`. Under heavy host load (load average >25) the
library tier can produce a spurious single-test failure that clears on rerun at
lower load; verify `uptime` before treating an L2 failure as real.

**Reconfirmed again after the review-7 DMLS hover/activation work** (2026-07-20,
review-7 implementation): library 18/18, CLI 66/69, DMLS 3/3 — total 87/90,
unchanged for the second consecutive cycle. The failure set is still exactly the
three named code-block tests. That cycle changed only DMLS hover routing,
pattern-key region projection, and the standalone envelope recognizer, and added
five L1 tests (dmls 616 → 621), so the L2 total holding steady is the expected
result rather than evidence of anything new. Host load averaged 36–95 across the
three tiers; the three failures remained deterministic value mismatches
(`got luma 44`), not timeouts, so load does not explain them.

**Reconfirmed a third time after the review-8 DMLS hardening work** (2026-07-20,
review-8 implementation): library 18/18, CLI 66/69, DMLS 3/3 — total 87/90,
unchanged for the third consecutive cycle, with the failure set still exactly the
three named code-block tests. That cycle hardened the standalone envelope
recognizer against escaped quotes, removed an `expect_err` panic, taught semantic
completion to locate a cursor structurally inside flow mappings, and made
activated standalone schema regions win hover arbitration over the Markdown
substrate — adding four L1 tests (dmls 621 → 625) and touching no rendering code,
so a steady L2 total is again the expected result. Host load averaged 157 during
this run; the three failures remained deterministic value mismatches rather than
timeouts, so load does not explain them.

**Repair implemented in review cycle 9.** The three tests use a focused tmux
runner local to `level2_code_block_styling.rs`. The shared
`run_md_env` / `run_md_after_shell_prefix` helpers and the remainder of the
69-test CLI Level-2 corpus remain on WezTerm, avoiding the broad helper port
that earlier reviews rejected as disproportionate.

**Previously noted as a latent hazard — repaired 2026-07-20 (review-6
implementation).** `darkmatter/cli/tests/level2_errors.rs` ran `md compose` as a
bare command through the pane `PATH` (lines 98, 135, 180) instead of the
`md_shim` that `common/level2.rs` adopted so Level 2 can never pass against a
stale host-installed `md`. The file was the only `level2_*` test file in
`darkmatter/cli/tests/` with no `mod common;` declaration, so it had no access
to the shared helper.

The hazard was active, not latent, and it failed in the more dangerous
direction: a stale `md` (2026-07-14, predating this branch's meta-schema
commits) was on the host `PATH`, so all 8 tests passed **green while verifying
the wrong binary**. Its output happened to be byte-identical to the workspace
binary across the four fixtures checked, which is why the drift survived
undetected. Review 6 observed the opposite symptom — `bash: md: command not
found` — on a host with no installed `md`; both symptoms are the same defect.

The three command builders now route through `md_shim()`, inheriting its
symlink → hard-link → copy fallback ladder and the `assert_shim_resolves_to_built`
integrity check. Non-vacuity was proven by temporarily neutering a rendered
error headline in `darkmatter/lib/src/markdown/errors/blocks.rs` and confirming
2 of the 8 tests went red — a break the pre-repair tests would have passed
through unchanged.

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
- **Q5 — Are arrays of the semantic types allowed?** **Ruled yes
  (2026-07-14 review).** The explicit `[]` postfix disambiguates the outer
  collection from a sequence-carried union, preserves the grammar's uniform
  array rule, and uses the existing generic lowering. A union-valued array item
  is represented by a nested sequence.
- **Q6 — How are source spans exposed without duplicating `PropertyDef`?**
  **Ruled sidecar source map (2026-07-14 review).** Semantic-only and
  source-aware entry points share one parser and AST. The latter adds
  structural-path-keyed spans projected through the existing YAML scalar seam;
  no second spanned AST or DMLS tokenizer is introduced.
- **Q7 — What bounds recursive native mappings?** **Ruled the existing shared
  depth limit (2026-07-14 review).** `MAX_INLINE_OBJECT_DEPTH` applies to both
  string-form inline objects and YAML-native mapping definitions. Semantic
  validation must not expose a less-bounded recursive path.
