# Authoring a Schema

A SimplifiedSchema describes the shape of YAML or JSON data in a few short
type strings. Write one when you want a document's frontmatter, a config
file, or a function's arguments checked: which keys must be present, what
type each value has, and which values are allowed. The library judges data
against it and, when asked, converts values that are almost right (the text
`"4"` for a `number`).

## What a Schema Looks Like

A schema is a YAML mapping from property names to **type-and-constraint**
strings:

```yaml
$schema:
    title: string(required) -> The page title
    tags: string[]
    status: enum(draft, published)
    priority: number(integer; min(1); max(5))
```

Read each line as `{type}({constraints}) -> {description}`; the constraints
and the description are optional. `title` must be present and be text, `tags`
is an optional list of text, `status` is one of two words, and `priority` is a
whole number from 1 to 5. Properties are optional unless `required` is
written.

The schema can live in two places:

- **Inline**, as the `$schema` key of the data it describes, such as a
  Markdown document's frontmatter. The other keys are the data:

  ```markdown
  ---
  $schema:
      title: string(required)
      tags: string[]
  title: Release notes
  tags: [changelog]
  ---
  ```

- **In a standalone file** that other documents reference by path. The file
  has the same `$schema` key at its root, and may add a `types` table of
  reusable definitions:

  ```yaml
  # schemas/page.yaml
  $schema:
      title: string(required)
      author: person
  types:
      person: "{ name: string(required), email: email }"
  ```

  A document then writes `$schema: ./schemas/page.yaml` in place of the
  mapping.

From Rust, the [`SimplifiedSchema` object](../simplified-schema-struct.md) builds from
either form and validates values:

```rust
use serde_json::json;
use simplified_schema::SimplifiedSchema;

let page = SimplifiedSchema::try_from("$schema:\n  title: string(required)\n  tags: string[]\n")?;
assert!(page.validate_strict(&json!({ "title": "Release notes", "tags": ["changelog"] })).is_ok());
assert!(page.validate_strict(&json!({ "tags": ["changelog"] })).is_err()); // `title` is required
```

## The Main Forms

| You want | Write | See |
| --- | --- | --- |
| a typed value | `name: string`, `age: number` | [Types](./types.md#types) |
| a constraint | `age: number(min(0); required)` | [Universal constraints](./types.md#universal-constraints) |
| an optional value that may not be `null` | `note: string(non-null)` | [Non-null properties](./types.md#non-null-properties) |
| a document with no undeclared keys | `$constraints: { closed: true }` in `$schema` | [Closed roots](./objects-and-named-types.md#closed-roots-closed) |
| a list | `tags: string[]` | [Array constraints](./types.md#array-constraints) |
| one of several values | `status: enum(draft, published)` | [Enumerations](./types.md#enumerations) |
| a nested object | `author: "{ name: string(required), email: email }"` | [Inline object literals](./objects-and-named-types.md#inline-object-literals) |
| a reusable or imported type | `author: person`, `owner: person@./types.yaml` | [Cross-file named-type imports](./objects-and-named-types.md#cross-file-named-type-imports-namefile) |
| a type that contains itself | `children: node[]` inside `node` | [Recursive named types](./objects-and-named-types.md#recursive-named-types) |
| either of two types | `id: "string \| number"` | [Unions](./unions.md#unions) |
| a whole document of one of several shapes | a YAML list at `$schema` | [Root-level unions](./unions.md#root-level-unions) |
| a path to a file | `source: file`, `source: file(eager)` | [Files](./types.md#files) |
| a callable signature | `function(parameters(a: number); returns(number))` | [Function signatures](./unions.md#function-signatures) |

## The Language Reference

- [Choosing Schema Types](./schema-types.md): a practical guide, with worked
  examples, to picking a type for a value.
- [Types and Constraints](./types.md): every type, the constraints each
  accepts, and how each lowers to Draft 2020-12 JSON Schema.
- [Objects and Named Types](./objects-and-named-types.md): inline objects,
  named types, cross-file imports, recursive and shared types, pattern keys,
  and key-count constraints.
- [Local Type References](./local-type-references.md): bare local names,
  where they resolve, and how reused types inherit constraints.
- [Unions and Function Signatures](./unions.md): property and root unions,
  the `|` operator, named union references, discriminated unions, and
  `function(...)` signatures.
- [Schema Resolution and Standalone Documents](./schema-documents.md): how a
  `$schema` reference finds its file, the envelopes a standalone schema
  file uses (`kind: schema`, `kind: yaml-schema`, and the informal one), and
  how each is checked against its definition.
- [Subtyping](./subtyping.md): whether one type accepts only values another
  type also accepts.
- [Slots](./slots.md): `{{ … }}` expressions that supply `enum`/`suggest`
  elements and `literal`, `default`, `min`, `max`, and `pattern` values, and
  what a schema holding one (a template) reports before it is resolved.
- [Conversion](../conversion.md): exactly which almost-right values soft
  validation converts, and how unions choose an arm.
- [Diagnostics](../diagnostics.md): the typed errors a schema or a value
  produces, and where each points.

## Limitations

These are limits of the language itself; a host built on it may add its own.

- **No remote references.** An `http://` or `https://` schema, import, or
  example reference is `SchemaError::RemoteUnsupported`. Download the schema
  and reference the local copy.
- **No authored `$ref`.** Reuse a definition through
  [named types](./objects-and-named-types.md#cross-file-named-type-imports-namefile).
  [Recursive](./objects-and-named-types.md#recursive-named-types) and
  [repeated structured](./objects-and-named-types.md#shared-named-types) named
  types lower to generated `$ref`s into the root `$defs`; every other use is
  copied in place. Which types are stored is decided automatically and
  cannot be requested or prevented.
- **Inline objects always reject extra keys.** There is no lenient opt-out
  for an inline object's `additionalProperties: false`. For looser typing use
  the opaque `object` type.
- **Root schemas allow extra keys unless closed.** A root, or one root-union
  arm, rejects undeclared keys only with
  [`$constraints: { closed: true }`](./objects-and-named-types.md#closed-roots-closed).
- **Inline objects nest at most 32 levels.** Deeper nesting is
  `SchemaError::Grammar`. Use a
  [recursive named type](./objects-and-named-types.md#recursive-named-types)
  for repeating structures, or reference a JSON Schema file for deeper
  typing.
- **No quoted property names inside an inline object.** Rename the key to a
  valid identifier (letters, digits, `-`, `_`; a leading digit is allowed) or
  use a JSON Schema file.
- **No commas inside inline object descriptions.** A description inside
  `{ ... }` ends at the next top-level comma or closing brace.
- **A fixed set of conversions.** Soft validation applies only the
  [conversion rules](../conversion.md#conversion-rules): no text parsed into a
  list or object, no value wrapped into a list, and no conversion across
  properties. The rules cannot be configured; to judge a value without any
  conversion, use [strict validation](../validation.md).
- **No cross-document constraints.** A schema judges one value; uniqueness
  across many documents is out of scope.
- **No parameterized `expression(…)`.** There is one expression grammar, so a
  positional value has nothing to select and is rejected; use bare
  `expression`.
- **No expression evaluation or return-type checking.** `expression` checks
  syntax only; whether a condition evaluates to a boolean is never checked.
