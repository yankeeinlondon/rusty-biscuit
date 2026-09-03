# Darkmatter Schemas

Use this reference for SimplifiedSchema parsing, validation, triggers, imports,
and DMLS schema behavior.

## Contents

- [Document kinds](#document-kinds)
- [Composition and validation](#composition-and-validation)
- [Imports and dictionaries](#imports-and-dictionaries)
- [Special types](#special-types)
- [Suggestions and DMLS](#suggestions-and-dmls)
- [Testing](#testing)

## Document kinds

Darkmatter recognizes two standalone SimplifiedSchema shapes:

```yaml
$schema:
  title: string(required)
```

```yaml
kind: schema
types:
  Person:
    name: string(required)
```

`parse_standalone_schema_document` is the passive authority. A pure schema must
have `$schema` as its only root key. A kinded schema must declare `kind: schema`
and provide a `types` mapping. Raw JSON Schema stays distinct.

Schema-trigger documents use `kind: schema-trigger`. Preserve the kinded root
declaration so Darkmatter and Claudine can select the right schema formally.

## Composition and validation

SimplifiedSchema compiles to Draft 2020-12 JSON Schema. Composition validates
after initial frontmatter interpolation and before shell expansion, then
revalidates values deferred because they contained pending shell syntax.

Validation-only APIs are passive and read-only. Composition may coerce declared
scalar types and normalize a successful eager `file(eager)` value to its
repo-relative path. At the same schema seam, after first-pass frontmatter
interpolation and before coercion, composition materializes an absent optional,
no-default top-level property as `null` only when the winning declaration comes
from the document's inline or referenced SimplifiedSchema. Baseline and trigger
properties, raw JSON Schema, root unions, nested properties, required
properties, and defaulted properties do not materialize. A compose run with no
effective schema and all validation-only APIs remain non-mutating. Repeated
schema passes are idempotent, and present values are preserved exactly.

Caller records retain an immutable raw value and file-resolution origin per
property. Before frontmatter interpolation pass 1, an exactly selected eager or
non-recursive lazy file arm materializes that value from its caller origin.
Eager local files must exist; lazy local files bind their first ordered
candidate without a probe, lazy HTTP(S) values remain remote, and recursive
lazy values fail because they have no single unprobed identity. This prelude
does not validate or mutate document-authored values. Markdown body
interpolation reads a separate portable presentation value, including through
static member and index selection; path operations, comparisons, frontmatter
expressions, and lifecycle state keep the native semantic identity. The raw
record remains unchanged for fresh preparation against another active schema.

`ValidationProblem` retains the public message plus typed code, JSON-pointer
instance path, optional schema path, offending property, source position, and
file-reference diagnostics. `ValidationOptions` controls pending values and
excluded keys without executing anything.

## Imports and dictionaries

- `Name@file` and `Name@this` import named types eagerly with dependency and
  cycle tracking.
- Root unions can compose schema arms without erasing each arm's origin.
- Pattern dictionary keys lower to `additionalProperties` or
  `patternProperties`; literal keys take precedence.
- `min-keys` and `max-keys` constrain dictionaries.
- Examples are documentation artifacts validated at schema-load time and
  emitted through `x-darkmatter-example`.

## Special types

- `literal(value)` lowers to JSON Schema `const`. Bare YAML bool/number values
  are typed; quoted values are strings; bare null is rejected. Only `required`
  and an equal default are allowed.
- `expression` is a parse-only string format. Native bool/number values coerce
  to strings, but no expression is evaluated.
- `yaml` and `json` accept string or native structured values and validate the
  encoded content format.
- `type-definition` validates one property definition.
- `schema` validates one complete `$schema` declaration.

The meta-types delegate to the same passive parser used by authoring and DMLS.
They do not perform imports, I/O, matching side effects, or rewrites.

## Suggestions and DMLS

`suggest(...)` attaches advisory completion candidates without changing
validation. `lint_suggestions()` reports malformed or misplaced suggestions;
`suggestions_for_path()` supplies structured completion items.

Literal discriminants use one presentation-neutral union-arm selector shared
by library validation and DMLS. Expression-typed values enable expression
completion, hover, and `dm.expression.*` diagnostics.

## Testing

For grammar or schema changes, cover:

- Native and quoted YAML representations.
- Missing, explicit null, valid, malformed, and boundary values.
- Passive parsing across every shipped schema and trigger artifact.
- An end-to-end `md schema` or normal compose invocation using the real shipped
  artifact.
- Imported dependency/cycle errors and source spans.
- Read/write/read repetition when composition persists normalized values.
