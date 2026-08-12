---
clarified: codex/gpt-5
review_iterations: 3
---

# Suggested Values for SimplifiedSchema

## Status

This specification defines the functional contract for the `suggest(...)`
SimplifiedSchema constraint. It covers authoring, generated JSON Schema,
library linting, standalone schema recognition, and DMLS diagnostics and
completion.

## Purpose

`suggest(...)` lets a schema author provide representative string or number
values without restricting the set of valid document values. DMLS uses valid
suggestions for frontmatter completion and warns when a suggestion conflicts
with its declared type or sibling constraints.

Unlike `enum(...)`, suggestions are advisory. A document value is valid when it
satisfies the underlying type and constraints, whether or not it appears in the
suggestion list.

## Terminology

- **Candidate**: one authored argument to `suggest(...)`.
- **Decoded text**: an argument's text after the existing SimplifiedSchema
  quoting and escaping rules have been applied.
- **Interpreted candidate**: the string or number value derived from decoded
  text according to the annotated target type.
- **Canonical decimal text**: a simple decimal spelling normalized without
  converting through a machine number: redundant integer leading zeros are
  removed, trailing fractional zeros and an empty decimal point are removed,
  and every positive or negative zero spelling becomes `0`.
- **Losslessly representable number**: a simple decimal whose canonical decimal
  value survives conversion to the supported JSON numeric model and canonical
  JSON serialization with the same exact canonical decimal value.
- **Eligible type**: the exact SimplifiedSchema `string` or `number` type,
  including its array form.
- **Target schema**: the non-null JSON Schema fragment for the annotated scalar
  or array item, excluding the suggestion annotation itself.
- **Invalid candidate**: an interpreted candidate that does not validate
  against its target schema.
- **Authoring schema**: the inline or standalone SimplifiedSchema source that
  contains `suggest(...)`.

## Authoring Surface

`suggest(...)` is a constraint containing a comma-separated, non-empty list of
candidate arguments:

```yaml
$schema:
    color: string(suggest(red, green, "blue gray"))
    port: number(integer; suggest(80, 443))
    ratio: number(min(0); max(1); suggest(0.25, 0.5, 1))
```

The constraint is available only on exact `string` and `number` types.
`number(integer)` remains eligible because `integer` is a constraint on
`number`, not a separate type.

Array forms are eligible. Their candidates describe individual elements, not
whole arrays:

```yaml
$schema:
    tags: string(suggest(alpha, beta))[]
    retries: number(integer; suggest(1, 2, 3))[]
```

### Cardinality

A complete property definition may contain at most one `suggest(...)`
constraint. A second occurrence is a structural SimplifiedSchema grammar error.

This restriction applies across all atoms of a property-level union. The
following definition is invalid even though each atom contains only one
occurrence:

```yaml
$schema:
    value: [string(suggest(a)), string(suggest(b))]
```

Each property declaration in a separate root-level schema-union arm is a
separate complete property definition and may carry its own single suggestion
list.

`suggest()` with no candidates is a structural grammar error.

### Argument Grammar

Candidate arguments reuse the existing SimplifiedSchema argument delimiter,
quoting, and escaping grammar. The feature does not introduce a second string
literal syntax.

The parser retains the exact source span and decoded text for every candidate.
Interpretation is target-directed; arguments are not first assigned generic
YAML or JSON scalar types and then coerced.

### String Interpretation

Every syntactically valid argument to `string(suggest(...))` is interpreted as
a string. Bare spellings that resemble numbers, booleans, or null are still
strings:

```yaml
$schema:
    value: string(suggest(Orange, 12, true, null))
```

The interpreted values are `"Orange"`, `"12"`, `"true"`, and `"null"`.
Quotes delimit and escape an argument; they do not create a distinct candidate
type. Consequently, `12` and `"12"` are the same interpreted string.

### Number Interpretation

Bare and quoted arguments to `number(suggest(...))` are interpreted using the
same simple decimal syntax:

```text
optional `-` + one or more digits + optional (`.` + one or more digits)
```

The syntax accepts leading zeros and canonicalizes them. It does not accept:

- exponent notation such as `1e3`;
- a leading plus sign such as `+1`;
- a missing integer portion such as `.5`;
- a missing fractional portion such as `5.`; or
- leading or trailing whitespace as part of the decoded value.

Simple decimal syntax alone is not sufficient to produce a numeric candidate.
Darkmatter first computes canonical decimal text using string operations, then
attempts to represent that value in its supported JSON numeric model. The value
becomes a JSON number only when that representation is lossless and
deterministic.

For this feature, losslessness is defined by observable canonical decimal
round-trip equality:

1. Normalize the decoded simple decimal to canonical decimal text without
   passing through a machine number.
2. Parse that canonical text into the supported JSON numeric model.
3. Canonically serialize the JSON number.
4. Convert any serializer exponent form to its exact decimal value for the
   comparison, normalize it by the same rules, and require byte-for-byte
   equality with the canonical text from step 1.

This contract tests the exact decimal value exposed by the JSON model; it does
not depend on a platform's internal floating-point formatting. A value that
parses but rounds, truncates, overflows, underflows, or serializes to a different
decimal value is not losslessly representable.

For example, `3`, `"3"`, `003`, and `3.0` all have canonical decimal text `3`
and interpret to the same numeric candidate `3`. Negative values follow the
same rules, and `-0` canonicalizes to `0`.

A syntactically valid simple decimal that is not losslessly representable is
retained as a JSON string containing its exact canonical decimal text. It is
invalid metadata: the library returns a structured lint problem that DMLS maps
to `dm.schema.invalid_suggestion`, DMLS omits it from completion, and schema
loading continues.

An argument whose decoded text does not use simple decimal syntax is retained
as its decoded JSON string solely so the invalid metadata remains available for
linting and exact diagnostics:

```yaml
$schema:
    count: number(suggest(1, many))
```

`many` does not make the schema fail to load. It remains invalid metadata,
produces a DMLS warning, and is omitted from completion. Syntax-invalid text is
preserved as decoded; representability-invalid simple decimals are preserved in
canonical decimal form.

### Numeric Boundary Expectations

Tests must derive the supported JSON numeric model's actual lossless boundary
rather than assume a platform word size or floating-point implementation:

- a large integer at the lossless boundary becomes a JSON number, while the
  first tested integer beyond that boundary is preserved as its canonical
  decimal string and warned;
- a long fractional value becomes a JSON number only when canonical decimal
  round-trip equality retains every significant digit; otherwise its full
  canonical decimal text is preserved and warned;
- negative boundary values follow the same decision as their positive
  magnitudes, except all zero spellings normalize to `0`;
- redundant leading integer zeros are removed before the lossless comparison,
  so they do not by themselves force fallback; and
- spellings that normalize to the same accepted number, including quoted and
  bare forms, leading-zero forms, trailing-fractional-zero forms, and negative
  zero, are duplicate candidates and fail at the later argument span.

The same input must produce the same numeric-versus-string decision and the
same generated metadata on macOS, Windows, and Linux.

### Uniqueness

Candidates must be unique after target-directed interpretation. A duplicate is
a structural SimplifiedSchema grammar error ranged at the later argument.

Therefore both examples below are invalid:

```yaml
$schema:
    label: string(suggest(12, "12"))
    count: number(suggest(3, "3"))
```

Uniqueness, linting, generated annotation output, and completion all operate on
interpreted candidates rather than authored spelling.

### Unsupported Targets

`suggest(...)` is not available on:

- `numberlike`;
- specialized string-valued types such as `date`, `datetime`, `time`, `url`,
  and `email`;
- `boolean` or `boolish`;
- `enum`;
- `any`;
- other SimplifiedSchema primitive types; or
- raw JSON Schema.

## Validation and Generated JSON Schema

`suggest(...)` is non-validating metadata. It must not alter whether a document
value satisfies its schema.

Darkmatter preserves interpreted suggestions in generated JSON Schema using
the custom `x-darkmatter-suggest` annotation. Authors never write this field;
they write only the SimplifiedSchema `suggest(...)` constraint.

```yaml
$schema:
    score: number(min(0); max(100); suggest(-1, 50, 101))
```

generates a property schema containing:

```json
{
  "type": "number",
  "minimum": 0,
  "maximum": 100,
  "x-darkmatter-suggest": [-1, 50, 101]
}
```

The annotation is a public generated-output compatibility contract. It
preserves interpreted candidate order and scalar values. JSON Schema validators
ignore it for validation, consistent with Darkmatter's other non-validating
`x-darkmatter-*` annotations.

The annotation must not be lowered to the standard JSON Schema `examples`
annotation. It is also distinct from Darkmatter's `example(...)` artifact
constraint and `x-darkmatter-example` annotation.

A number candidate that does not use simple decimal syntax is emitted as its
decoded JSON string. A simple decimal that is not losslessly representable is
emitted as its exact canonical decimal string. Both fallback forms keep the
generated metadata complete and allow the library to report the defect without
blocking schema use.

A syntactically well-formed property definition containing target-invalid
candidates must continue through:

- SimplifiedSchema parsing and resolution;
- JSON Schema conversion and validator construction;
- frontmatter validation;
- Markdown composition; and
- schema-aware completion infrastructure.

Invalid candidates are metadata defects, not schema-load errors and not
frontmatter-validation errors. Outside DMLS, existing validation and
composition paths remain permissive and do not fail because of them.

Raw JSON Schema may contain an unknown field with the same spelling, but
Darkmatter and DMLS do not discover suggestions from raw JSON Schema. The
annotation is generated and consumed as part of the SimplifiedSchema contract.

## Candidate Linting

The Darkmatter library owns candidate checking and exposes structured, typed,
span-bearing suggestion-lint results. DMLS consumes those results and must not
independently reimplement candidate interpretation or SimplifiedSchema
validation semantics.

Each interpreted candidate is checked against the exact target schema:

- For a scalar property, the target is the property's non-null scalar schema.
- For `string[]` and `number[]`, the target is the array's item schema.
- `x-darkmatter-suggest` is excluded from the target before checking.
- Applicable number constraints include `min`, `max`, and `integer`.
- Applicable string constraints include minimum length, maximum length,
  `not-empty`, and `pattern`.
- `required`, `default`, `generated`, and `example(...)` do not constrain a
  candidate.

All authored candidates are linted, including candidates in root-level union
arms that do not provide completion. A number candidate that fails simple
decimal syntax or lossless JSON representation remains string metadata, fails
the number target's type requirement, and produces a structured lint problem.

Each lint problem retains the interpreted value, decoded text, failure reason,
and exact original argument span through parsing and schema resolution.

## Standalone SimplifiedSchema Documents

DMLS recognizes standalone SimplifiedSchema YAML by document content. Filename
patterns and configured schema-document globs are not part of recognition.

### Pure Envelope

A pure schema definition file is a YAML mapping whose only top-level key is
`$schema`. Its value is the SimplifiedSchema payload:

```yaml
$schema:
    name: string(suggest(Bob, Mary, Sam))
    age: number(integer; min(0); suggest(21, 30, 40))
```

A mapping payload is usable both as a whole-file schema and as the namespace
for named imports.

A sequence payload remains supported as a root-level schema union for
whole-file use. It supplies no named-import namespace.

### Tagged Envelope

A tagged schema definition file is a YAML mapping containing exactly
`kind: schema` and a `types` mapping:

```yaml
kind: schema
types:
    name: string(suggest(Bob, Mary, Sam))
    age: number(integer; min(0); suggest(21, 30, 40))
```

No other top-level keys are permitted. The `types` mapping is semantically
equivalent to a pure envelope's `$schema` mapping for whole-file use and named
imports.

### Whole-File References and Named Imports

Markdown frontmatter `$schema` remains one of:

- an inline SimplifiedSchema mapping;
- an inline SimplifiedSchema sequence/root union; or
- a `FileReference` to either standalone SimplifiedSchema envelope.

```yaml
---
$schema: ./schemas/person.yaml
name: Bob
age: 30
---
```

Referencing either mapping envelope as a whole validates the document against
that mapping's complete object shape.

The actual named-import syntax is `Name@fileref`; braces used in design
discussion were placeholders, not literal grammar:

```yaml
$schema:
    display-name: name@./schemas/person.yaml
    ages: age[]@./schemas/person.yaml
```

`Name@fileref` extracts `Name` from the same mapping payload and structurally
inlines it using the existing eager, bounded, cycle-checked import behavior.

### Malformed Envelopes

Once a standalone SimplifiedSchema envelope is recognized, a missing or
malformed payload is a schema-document diagnostic. DMLS and the resolver must
not silently reinterpret that document as ordinary YAML or raw JSON Schema.

For the tagged envelope, `kind: schema` claims the document even when `types`
is missing, malformed, or accompanied by unsupported top-level keys. These are
schema-document errors.

### Raw JSON Schema

Existing raw JSON Schema reference support remains a distinct validation
format, including the currently supported referenced file formats. Raw JSON
Schema:

- does not provide `suggest(...)`;
- cannot supply a `Name@fileref` named-import namespace;
- does not receive SimplifiedSchema authoring diagnostics or completion; and
- does not enable suggestion discovery from a hand-authored
  `x-darkmatter-suggest` field.

## DMLS Diagnostics

DMLS emits one diagnostic for each invalid candidate:

- severity: `WARNING`;
- source: `darkmatter.schema`;
- stable code: `dm.schema.invalid_suggestion`;
- range: the exact original argument in the authoring schema.

The message identifies why the interpreted candidate fails its target, such as
invalid decimal syntax, unsupported lossless JSON numeric representation, range
violation, integer violation, string-length violation, or pattern mismatch.

Exact source diagnostics apply to:

- SimplifiedSchema authored inline in Markdown frontmatter; and
- either recognized standalone SimplifiedSchema YAML envelope.

DMLS recognizes an open YAML buffer from these content envelopes and publishes
diagnostics against that open authoring document. It does not place a
standalone-schema candidate warning on each consuming Markdown document's
`$schema` reference.

An invalid candidate must not cause DMLS to discard the effective schema or
disable otherwise valid schema completion.

## DMLS Completion

DMLS provides end-to-end completion from `suggest(...)` for:

- scalar `string` and `number` property values;
- eligible properties nested in inline-object schemas;
- elements in block-style YAML arrays; and
- elements in flow-style YAML arrays.

Completion behavior is:

- preserve interpreted candidate declaration order;
- prefix-filter using decoded candidate values;
- omit candidates identified as invalid by suggestion linting;
- always insert strings as YAML-safe double-quoted scalars;
- insert numbers using their canonical numeric spelling;
- for a property-level union, use its single suggestion-bearing eligible arm;
  and
- for a root-level schema union, use the first declaration-order arm containing
  an eligible suggestion-bearing definition for the property.

All candidates in all root-level arms remain linted even though later arms do
not contribute completion for that property.

For an eligible array, completion inserts one candidate element. It does not
replace the array with a collection of suggestions.

DMLS reads suggestions from the SimplifiedSchema representation. It does not
depend on discovering the generated annotation in raw JSON Schema.

## Failure Behavior

- Empty lists, repeated `suggest(...)` constraints within one complete
  property definition, duplicates after interpretation, and malformed argument
  syntax are structural SimplifiedSchema grammar errors.
- Target-invalid candidates, including number candidates with invalid decimal
  syntax or no lossless JSON representation, produce structured lint output
  and DMLS warnings while schema resolution continues.
- Failure to retain an exact source span for an authored candidate is an
  implementation defect; DMLS must not silently substitute a consumer
  document's `$schema` reference range.
- Candidate linting and completion must not perform filesystem discovery,
  execute composition directives or shell expressions, or cause other side
  effects.

## Scope Boundaries

This feature includes:

- SimplifiedSchema grammar and interpreted, span-bearing AST support for
  `suggest(...)`;
- generated `x-darkmatter-suggest` annotations;
- a library-owned structured lint API;
- content-based recognition of the two standalone SimplifiedSchema envelopes;
- DMLS diagnostics in inline and standalone authoring schemas; and
- DMLS completion at the scalar, nested-property, and array-item positions
  listed above.

This feature does not include:

- restricting document values to suggested candidates;
- applying document-value coercion rules to suggestion arguments;
- changing `enum(...)` semantics;
- replacing or extending `example(...)` artifacts;
- suggestion discovery from raw JSON Schema;
- diagnostics from `md schema validate` or the compose pipeline;
- filename- or glob-based standalone schema recognition;
- support for types other than exact `string` and `number`; or
- I/O, composition, or shell side effects during linting or completion.

## Acceptance Criteria

1. Exact `string` and `number` properties parse one non-empty
   `suggest(...)`; unsupported target types do not gain the constraint.
2. `number(integer; suggest(...))`, `string(suggest(...))[]`, and
   `number(suggest(...))[]` are eligible.
3. A second suggestion constraint anywhere in one complete property
   definition, including another property-union atom, is a structural error.
4. String arguments are interpreted as decoded strings regardless of bare or
   quoted spelling.
5. Number arguments use the specified simple decimal language; leading zeros
   canonicalize, while exponent, plus, `.5`, and `5.` forms remain invalid
   metadata rather than schema-load errors.
6. A simple decimal becomes a JSON number only when canonical decimal
   round-trip comparison proves that the supported JSON numeric model preserves
   its exact normalized value deterministically.
7. A simple decimal outside that lossless representation boundary is preserved
   as its exact canonical decimal JSON string, produces
   `dm.schema.invalid_suggestion`, is omitted from completion, and never blocks
   schema loading or validation.
8. Duplicate interpreted candidates are rejected at the later argument span,
   including duplicates created by leading-zero, fractional-zero, quoted/bare,
   or negative-zero normalization.
9. Conversion emits public, non-validating `x-darkmatter-suggest` metadata in
   declaration order and never uses JSON Schema `examples` for this feature.
10. A frontmatter value validates based only on its underlying type and
   constraints, regardless of the suggestion list.
11. Invalid candidates do not fail schema resolution, validator construction,
   frontmatter validation, composition, or surrounding completion
   infrastructure.
12. The library returns one structured lint problem per invalid candidate with
    its decoded text, interpreted value, reason, and exact source span.
13. Candidate checking honors applicable type, numeric, and string constraints
    while ignoring `required`, `default`, `generated`, and `example(...)` as
    candidate restrictions.
14. DMLS publishes `WARNING` diagnostics with source `darkmatter.schema` and
    code `dm.schema.invalid_suggestion` on exact argument ranges.
15. Exact diagnostics work for inline schemas and both standalone YAML
    envelopes, including malformed recognized-envelope diagnostics.
16. The pure and tagged mapping payloads behave identically for whole-file
    references and `Name@fileref` imports; pure sequence payloads work only as
    whole-file root unions.
17. Existing raw JSON Schema validation remains available without gaining
    SimplifiedSchema suggestions, named imports, or authoring intelligence.
18. DMLS completes lint-valid suggestions in declaration order and
    prefix-filters them for scalar values, nested inline-object properties,
    block-array items, and flow-array items.
19. Completion inserts YAML-safe double-quoted strings and canonical numbers;
    invalid candidates are absent while valid siblings remain available.
20. Property unions use their single suggestion-bearing eligible arm; root
    unions use the first suggestion-bearing eligible arm for completion while
    linting all arms.
21. Array suggestions insert individual elements rather than whole arrays.
22. Raw JSON Schema `x-darkmatter-suggest` annotations do not enable this
    completion path.
23. Boundary tests cover losslessly representable and fallback cases for large
    integers, long fractional values, negative values, leading zeros, negative
    zero, and duplicates created by decimal normalization.
24. Unit and DMLS integration tests cover these requirements without
    OS-specific path, separator, or newline assumptions.

## Definition of Done

The feature is done when:

- the grammar, AST, serializer, converter, public descriptor catalog, and
  schema documentation describe the same contract;
- generated JSON Schema compatibility tests cover interpreted suggestions,
  invalid-syntax fallback metadata, and lossless-boundary fallback metadata;
- numeric boundary tests prove deterministic canonical round trips for large
  integers, long fractions, negatives, leading zeros, negative zero, and
  normalization-created duplicates;
- the Darkmatter lint API and DMLS mapping are implemented without duplicating
  interpretation or validation semantics;
- DMLS recognizes both standalone envelopes and provides exact candidate
  ranges in open schema documents;
- completion works at every position and union behavior listed above;
- raw JSON Schema regression coverage confirms its distinct existing behavior;
- relevant Darkmatter unit tests and in-memory DMLS integration tests pass with
  `just test`; and
- the package-area lint recipe passes with `just lint`.
