---
created: 2026-06-10
status: draft
---

# Inline Nested Object Schemas in SimplifiedSchema

SimplifiedSchema v1 treats `object` as opaque — it accepts any JSON/YAML object shape. This is a deliberate simplicity trade-off, but it forces authors who need typed object arrays (e.g. an array of `{ foo: string, bar: string }`) to drop down to a referenced JSON Schema file.

This feature adds an **inline object literal** syntax to the SimplifiedSchema grammar so authors can declare the shape of nested objects directly inside a type expression, without leaving the single-line grammar or referencing an external file.

## Goals & Non-Goals

**Goals**

- Let authors declare typed object arrays inline: `entries: "{ foo: string(required), bar: string }[]"`.
- Allow arbitrary nesting: `{ outer: { inner: string } }`.
- Support inline objects in all positions where a type expression is valid: single properties, union arms, and array item types.
- Compile inline object literals to the same Draft 2020-12 JSON Schema as a hand-written `type: object` with `properties` and `required`.
- Make whitespace inside braces insignificant for readability.
- Add a descriptor-backed `md schema about` command so users can discover the schema language from the CLI without relying on hand-maintained prose alone.
- Expose the same schema-language documentation surface to library callers, following the descriptor-catalog pattern already used for context variables, expression functions, and side effects.

**Non-Goals (this feature)**

- Changing the root-schema default of `additionalProperties: true`. Inline objects default to `additionalProperties: false` (see Decision #7); a future feature may add an opt-in `lenient` constraint to relax this.
- `$ref` or reusable schema fragments. Inline objects are anonymous; reuse still requires an external JSON Schema file.
- Extending `md schema detect` to synthesize inline object schemas from sample values. Detection continues to emit `object` for object-typed values; inline object detection is deferred.
- Remote or file-referenced sub-schemas inside the inline object body. Every property inside `{ ... }` must be a SimplifiedSchema type expression, not a `$schema: ./foo.yaml` reference.
- Quoted or arbitrary inline object property keys. Inline object keys use the existing unquoted string-layer identifier scanner only.
- Escaping or quoting commas inside inline property descriptions. Descriptions containing commas are out of scope unless a future quoting/escaping feature is added.
- Adding caller CLI commands outside Darkmatter. This feature exposes the schema-language documentation catalog through Darkmatter; callers such as Claudine can add their own CLI surfaces in a future spec.
- Adding machine-readable `md schema about --format json` output. Callers that need structured data should use the public descriptor API directly in this feature.

## Foundational Decisions

- **Decision #1** — Inline object literals use curly-brace delimiters with comma-separated property definitions: `{ prop: type-expr, prop: type-expr }`. This mirrors JSON/YAML object syntax and is visually distinct from the constraint parenthesis syntax.
- **Decision #2** — Spaces and line breaks inside `{ ... }` are ignored. The parser strips optional whitespace after `{`, around `:`, around `,`, and before `}` so authors can format multi-line inline objects without fighting the grammar.
- **Decision #3** — Property definitions inside an inline object follow the same four syntax forms as top-level SimplifiedSchema properties: bare type, constrained type, type with description, and constrained type with description. The description arrow `->` is allowed per property.
- **Decision #4** — Inline objects can be nested recursively. The grammar parser descends into nested `{ ... }` blocks and produces nested `SchemaShape` AST nodes.
- **Decision #5** — Inline objects support the `[]` array suffix just like primitive types. `{ foo: string }[]` compiles to an array whose items are objects with a `foo` string property.
- **Decision #6** — Inline objects can appear as arms in property-level unions. A property may accept either an inline object shape or a primitive: `data: [ "{ foo: string }", "string" ]`.
- **Decision #7** — Inline objects default to `additionalProperties: false` in the generated JSON Schema. This differs from the root schema default (`true`) because the purpose of declaring an inline object is to restrict shape; silently accepting extra keys would defeat the author's intent. A future `lenient` constraint on the inline object body may opt back to `true`.
- **Decision #8** — Inline object property names use the current string-layer identifier scanner: unquoted ASCII alphanumeric characters plus `-` and `_`, including leading digits. Accepted examples: `name`, `foo_id`, `x-custom`, `api2_version`, `123abc`. Rejected examples: `display name`, `@type`, `x.custom`, and quoted property names such as `"x-custom"`.
- **Decision #9** — Inline object property descriptions terminate at the next top-level comma or closing brace in the current inline object body. This preserves per-property `->` descriptions inside `{ ... }`, but commas inside inline descriptions are not supported by this feature.
- **Decision #10** — Inline object atoms support the same postfix constraints as primitive atoms. For a single inline object, `{ host: string }(required)` attaches `required` to the containing property value. For an inline object array, `{ name: string }[](min(1); required)` attaches `min(1)` and `required` to the array property, while constraints on nested properties remain inside the nested object schema.
- **Decision #11** — Inline object parsing enforces a hard maximum nesting depth of 32 inline object levels. Exceeding that depth is a grammar error.
- **Decision #12** — Schema-language documentation is generated from typed descriptor catalogs, not duplicated as unrelated CLI prose. `md schema about` renders those descriptors, and library callers can consume the same descriptors to render their own reports. Tests must keep the descriptor catalog in parity with the implemented type and constraint surface so documentation drift is caught during development.

## Grammar Specification

### Updated EBNF

```text
type_expr_string := type_expr ( "->" description )?
type_expr        := simple_type
                  | inline_object
simple_type      := type_name ( "(" item_constraints ")" )?
                               ( "[]" ( "(" arr_constraints ")" )? )?
inline_object    := "{" ws* property_list? ws* "}"
                    ( "(" item_constraints ")"
                    | "[]" ( "(" arr_constraints ")" )?
                    )?
property_list    := property_def ( "," ws* property_def )* ","?
property_def     := identifier ws* ":" ws* type_expr_string
ws               := <whitespace or line-break>
identifier       := ( ASCII_ALNUM | "-" | "_" )+
type_name        := "string" | "date" | "datetime" | "time" | "number"
                  | "numberlike" | "boolean" | "boolish" | "object"
                  | "file" | "enum" | "url" | "email" | "any"
item_constraints := constraint ( ";" constraint )*
arr_constraints  := constraint ( ";" constraint )*
constraint       := IDENT
                  | IDENT "(" arglist ")"
arglist          := arg ( "," arg )*
arg              := NUMBER | BARE_WORD | SQUOTED | DQUOTED
description      := <rest-of-top-level-field, trimmed>
```

At the top level, `description` keeps the existing behavior: it consumes the rest of the scalar string after `->`. Inside an `inline_object`, `description` consumes text until the next comma or closing brace at the current object nesting level. Commas inside inline descriptions are not supported unless a future quoting/escaping feature adds a way to disambiguate them.

### Syntax Examples

```yaml
$schema:
    # Array of typed objects — the motivating case
    entries: "{ foo: string(required), bar: string }[]"

    # Optional single typed object
    config: "{ host: string(required), port: number(default(8080)) }"

    # Required single typed object — the postfix constraint applies to config
    config_required: "{ host: string }(required)"

    # Multi-line for readability (whitespace inside braces is ignored)
    endpoints: "{
        url: url(scheme(https); required),
        method: enum(GET, POST, PUT, DELETE; required),
        timeout: number(default(30))
    }[]"

    # Nested objects
    database: "{
        primary: { host: string(required), port: number(default(5432)) },
        replicas: { host: string, port: number }[]
    }"

    # Required non-empty array — postfix constraints after [] apply to replicas
    replicas: "{ host: string }[](min(1); required)"

    # Inline object as a union arm
    payload:
      - "{ type: enum(foo; required), foo_id: string(required) }"
      - "{ type: enum(bar; required), bar_count: number(required) }"

    # Mixed: inline object array or a plain string fallback
    metadata:
      - "{ key: string(required), value: string(required) }[]"
      - "string"
```

Inline object property names are always unquoted identifiers. `name`, `foo_id`, `x-custom`, `api2_version`, and `123abc` are valid. `display name`, `@type`, `x.custom`, and `"x-custom"` are invalid in this feature.

### Whitespace Rules

Inside an `inline_object` body, the following whitespace is **ignored** by the parser:

- Any whitespace immediately after the opening `{`
- Any whitespace immediately before the closing `}`
- Any whitespace around property-definition commas `,`
- Any whitespace around property-name colons `:`
- Any whitespace at the start or end of a `type_expr_string` that is a property value

This allows the multi-line `endpoints` example above to parse identically to its single-line form.

### Parsing Strategy

The inline object literal is parsed by the **string-layer grammar parser** (`simplified/grammar.rs`), not by the YAML-shape layer. The parser:

1. Observes `{` after reading an optional type name (or at the start of a type expression).
2. Enters an **object-body parsing mode** that reads property definitions separated by commas.
3. For each property definition, reads an unquoted string-layer identifier, a colon, and then recursively calls `parse_type_expr` to read the property's type expression.
4. Handles nested `{ ... }` by recursion — the object-body parser is re-entrant.
5. Stops at the matching `}` (braces are not allowed inside constraint argument lists, so there is no nesting ambiguity with constraint syntax).
6. Rejects inputs that exceed 32 inline object nesting levels with `SchemaError::Grammar`.
7. After closing `}`, processes either single-object constraints or the optional `[]` suffix with array constraints. For a single inline object, item constraints apply to the containing property's object value. For an inline object array, constraints after `[]` apply to the containing array property. Constraints before `[]` are not valid for inline object arrays.
8. When `->` appears inside an inline object property, reads the description until the next comma or closing brace at the current object nesting level.

## Type System Changes

### AST Additions

```rust
// darkmatter/lib/src/markdown/schemas/simplified/types.rs

/// A type expression is either a primitive type or an inline object shape.
#[derive(Debug, Clone, PartialEq)]
pub enum TypeExpr {
    /// Built-in primitive type (string, number, object, etc.).
    Primitive(SimplifiedType),
    /// Inline object literal: `{ foo: string, bar: number }`.
    InlineObject(SchemaShape),
}

/// Updated PropertyAtom to use TypeExpr instead of SimplifiedType.
pub struct PropertyAtom {
    /// The type expression for this atom (primitive or inline object).
    pub ty: TypeExpr,
    pub is_array: bool,
    pub constraints: Vec<Constraint>,
    pub array_constraints: Vec<Constraint>,
    pub description: Option<String>,
}
```

`SimplifiedType` remains unchanged and continues to include the `Object` variant (opaque object). The new `TypeExpr::InlineObject(SchemaShape)` is a separate construct that carries a fully-typed `SchemaShape`.

### Backward Compatibility

`SimplifiedType` keeps its `Copy` derive. `TypeExpr` is `Clone` but not `Copy`. Call sites that previously matched on `atom.ty` (a `SimplifiedType`) now match on `atom.ty` (a `TypeExpr`) and handle the `Primitive` / `InlineObject` arms.

The `SimplifiedType::Object` variant is **not** removed — `"object"` and `"object[]"` continue to parse and compile to `{ "type": "object" }` with no inner schema. This preserves all existing schemas.

## YAML-Shape Layer Changes

The YAML-shape layer (`simplified/mod.rs`) currently rejects mapping property values with the message *"mapping property values are reserved for future nested object schemas"*. That message is updated to avoid implying that YAML mapping values are part of this feature. Mapping values at property positions are parsed as inline object literals only when they appear inside a string scalar that the grammar parser recognizes as an `inline_object`.

The YAML-shape layer itself does not change its shape rules:

| YAML shape at a property value | Interpretation |
|---|---|
| Scalar (string) | Single `PropertyAtom` — parse the string per EBNF (now including inline object syntax) |
| Sequence whose items are all scalars | Property-level union (`PropertyDef::Union`) |
| Sequence containing any non-scalar | Error |
| Mapping | Error — still reserved, but for a different future (YAML-native inline schemas, not string literals) |

## JSON Schema Conversion

### Inline Object Fragment

An inline object `{ foo: string(required), bar: number }` compiles to:

```json
{
  "type": "object",
  "properties": {
    "foo": { "type": "string" },
    "bar": { "type": "number" }
  },
  "required": ["foo"],
  "additionalProperties": false
}
```

Postfix constraints on a single inline object are `PropertyAtom` constraints on the containing property. For example, `config: "{ host: string }(required)"` compiles the object fragment for `config` and hoists `required` to the parent object's `required` array. The nested `host` property remains optional because its own atom has no `required` constraint.

### Array of Inline Objects

`{ foo: string }[]` compiles to:

```json
{
  "type": "array",
  "items": {
    "type": "object",
    "properties": {
      "foo": { "type": "string" }
    },
    "additionalProperties": false
  }
}
```

Postfix constraints after `[]` are array-property constraints. For example, `replicas: "{ host: string }[](min(1); required)"` compiles `replicas` as a required array with `minItems: 1`; the array `items` schema is the inline object fragment. Constraints on nested properties inside the braces remain nested inside the `items` object schema.

### Nested Inline Objects

`{ outer: { inner: string } }` compiles to:

```json
{
  "type": "object",
  "properties": {
    "outer": {
      "type": "object",
      "properties": {
        "inner": { "type": "string" }
      },
      "additionalProperties": false
    }
  },
  "additionalProperties": false
}
```

### Inline Object in a Union Arm

`[ "{ foo: string }", "string" ]` compiles to:

```json
{
  "anyOf": [
    {
      "type": "object",
      "properties": {
        "foo": { "type": "string" }
      },
      "additionalProperties": false
    },
    { "type": "string" }
  ]
}
```

### Conversion Algorithm

`convert::type_fragment` gains a new branch for `TypeExpr::InlineObject(shape)`:

1. Recursively convert each property in `shape.properties` using the existing `property_def_to_schema` helper.
2. Collect property names that have `required` hoisted from their definition.
3. Emit `{ "type": "object", "properties": { ... }, "required": [ ... ], "additionalProperties": false }`.
4. Apply non-array `PropertyAtom.constraints` to the object property when the inline object is not array-suffixed. This is the same constraint attachment point used by primitive atoms; `required` affects the parent object's `required` array.
5. If the inline object is array-suffixed, wrap in `{ "type": "array", "items": <object fragment>, ...array constraints... }`. Constraints after `[]` apply to this array property, not to the object `items` schema.

The conversion of `TypeExpr::Primitive(existing_type)` delegates to the existing per-type fragment builders (`string_fragment`, `number_fragment`, etc.).

## Compose Schema Coercion

Schema-driven coercion continues to run after `--set` / `--state` and interpolation, before shell expansion, and writes coerced values back into frontmatter.

Coercion now recurses into inline object fields and inline object arrays when the matching schema path is unambiguous. Nested `boolean`, `boolish`, `number`, `numberlike`, and string-shaped scalar coercions behave the same as top-level fields. For example, a value at `/authors/0/active` is coerced from `"true"` to `true` when the schema path is `authors: "{ active: boolean }[]"`.

For property-level unions, coercion is attempted per arm. The compose stage builds a coerced candidate value for each union arm using that arm's schema path, validates each candidate against that arm, and commits the coerced value only when exactly one arm validates after coercion. If zero arms validate after per-arm coercion, or if multiple arms validate, the original value is left uncoerced and normal validation/reporting proceeds from that original value. This avoids guessing across ambiguous inline object paths while still allowing unambiguous union arms to coerce nested scalars.

## Schema About Report

The schema language is a user-facing DSL and should have a discoverable, implementation-bound reference surface. This feature adds `md schema about`, a documentation-only command that reports how SimplifiedSchema is defined and evaluated.

The command renders from a typed descriptor catalog in `darkmatter::markdown::schemas`, following the existing Darkmatter pattern used by:

- `context_variable_descriptors()` for runtime context variables
- `expression_function_descriptors()` for expression functions
- `effect_descriptors()` for side-effect capabilities

The schema descriptor surface must be available to library callers so downstream tools can render their own schema-language reports without scraping CLI output. A future Claudine spec may add a caller-side CLI command using this same Darkmatter API; this feature only adds the Darkmatter API and the `md schema about` command.

### CLI Behavior

`md schema about` prints a human-readable report covering:

- SimplifiedSchema shape: inline mapping, root-level union, file reference, and property-level union.
- Type vocabulary: `string`, `date`, `datetime`, `time`, `number`, `numberlike`, `boolean`, `boolish`, `object`, `file`, `enum`, `url`, `email`, and `any`.
- Constraint vocabulary, including which constraints apply to which types and which constraints apply at array level.
- Inline object syntax, including postfix constraints, arrays, nesting depth limit, identifier rules, description termination, and `additionalProperties: false`.
- Validation behavior: required/default hoisting, root-schema `additionalProperties: true`, opaque `object`, schema detection limits, and compose-time coercion.
- Security/operational notes that affect schema authors, such as `file` resolution at validation time and the fact that `md schema about` is documentation-only.

The command has no input files and no format flags in this feature. It must not parse user documents, load schema files, capture context, resolve `file` references, construct an `EffectEngine`, or perform network access. It is a pure metadata report.

### Library API

Darkmatter exposes a public schema-language descriptor API with stable traversal order. The exact Rust names are implementation details, but the surface must let callers enumerate at least:

- supported schema shapes and grammar forms
- supported type keywords and their display descriptions
- supported constraints, their argument forms, their applicable target types, and their JSON Schema effect
- inline object grammar rules and limits
- coercion rules and non-coercion cases

The descriptors are the authoritative source for the `md schema about` report. Hand-written prose in `schema-definition.md` may explain the concepts, but it must not be the only source used by the CLI report.

### Drift Prevention

The descriptor catalog must be tied to the implementation with tests comparable to the existing expression and side-effect catalog tests:

1. Every `SimplifiedType` keyword has exactly one type descriptor, and every type descriptor maps to a parseable implemented type.
2. Every implemented constraint has descriptor coverage for its accepted target types and argument form.
3. Descriptor examples parse successfully, except examples explicitly marked invalid.
4. The descriptor traversal order is deterministic and descriptor keys/signatures are unique.
5. Accessing the descriptor catalog and rendering `md schema about` performs no document parsing, context capture, effect-engine construction, or network access.

## Schema Detection

**Unchanged in this feature.** `md schema detect` continues to emit `object` for any object-typed value, even when sample values have a consistent shape. A future feature may enhance detection to infer and emit inline object schemas when object shapes are uniform across the corpus.

## Examples

### Example 1: Blog post with tags

```yaml
---
$schema:
    title: "string(required)"
    tags: "string[](min(1))"
    authors: "{ name: string(required), email: email }[]"
---
```

### Example 2: API endpoint specification

```yaml
---
$schema:
    base_url: "url(scheme(https); required)"
    routes: "{
        path: string(pattern(^/[a-z0-9-/]+$); required),
        method: enum(GET, POST, PUT, PATCH, DELETE; required),
        params: { name: string(required), type: enum(query, path, body; required), required: boolean(default(true)) }[]
    }[]"
---
```

### Example 3: Union of shapes (discriminated by enum)

```yaml
---
$schema:
    notification:
      - "{ channel: enum(email; required), to: email(required), subject: string(required) }"
      - "{ channel: enum(slack; required), webhook: url(required), message: string(required) }"
---
```

## Module Layout & Touchpoints

**Modified files:**

```
darkmatter/lib/src/markdown/schemas/simplified/
├── types.rs          # Add TypeExpr enum; update PropertyAtom.ty field type
├── grammar.rs        # Add inline_object parsing to Parser; recursive property_def reader
├── convert.rs        # Handle TypeExpr::InlineObject in type_fragment and atom_to_schema
└── mod.rs            # Update mapping-value error wording for the string-only inline object scope
darkmatter/lib/src/markdown/schemas/
└── about.rs          # Add descriptor-backed schema-language documentation catalog
darkmatter/cli/src/
├── args.rs           # Add `md schema about`
└── commands/schema/  # Render the schema about report from the descriptor catalog
```

The new syntax is valid inside any `$schema` string and is compiled/validated by the existing `md schema validate` and `md compose` pipelines. The only user-facing CLI addition is the documentation-only `md schema about` command.

## Documentation Updates

- `darkmatter/docs/topics/schema-definition.md` must be updated when this feature lands to document inline object syntax, postfix constraints, identifier rules, description comma limits, nested coercion, and the 32-level nesting limit.
- `darkmatter/docs/topics/schema-definition.md` should also point users to `md schema about` as the implementation-bound CLI reference for the schema language.

## Testing Strategy

### Grammar parser unit tests

- Parse bare inline object: `"{ foo: string, bar: number }"` → `TypeExpr::InlineObject` with two properties.
- Parse inline object array: `"{ foo: string }[]"` → `is_array: true`, inner `TypeExpr::InlineObject`.
- Parse required single inline object: `"{ host: string }(required)"` → `is_array: false`, inner `TypeExpr::InlineObject`, atom constraint `required`.
- Parse constrained inline object array: `"{ name: string }[](min(1); required)"` → `is_array: true`, array constraints `min(1)` and `required`.
- Parse nested inline object: `"{ outer: { inner: string } }"` → two levels of `SchemaShape`.
- Parse inline object at depth 32 → accepted.
- Parse inline object at depth 33 → `SchemaError::Grammar`.
- Whitespace tolerance: `"{  foo : string , bar : number  }"` parses identically to compact form.
- Multi-line inline object: string containing newlines inside `{ ... }` parses correctly.
- Inline object with constraints on properties: `"{ foo: string(required; not-empty), bar: number(min(0)) }"`.
- Inline object with descriptions: `"{ foo: string(required) -> The foo, bar: number -> The bar }"`.
- Inline object descriptions terminate at the next top-level comma or closing brace, preserving `The foo` and `The bar` as separate property descriptions in the example above.
- Inline object description with an unescaped comma: `"{ foo: string -> first, second }"` → treats `second` as the next property start and fails because descriptions with commas are out of scope.
- Accepted identifiers: `name`, `foo_id`, `x-custom`, `api2_version`, and `123abc`.
- Rejected identifiers: `display name`, `@type`, `x.custom`, and `"x-custom"`.
- Inline object as union arm: `"[ '{ foo: string }', 'string' ]"` (YAML sequence of strings).
- Empty inline object: `"{}"` → `SchemaShape` with empty `properties`, compiles to `{ "type": "object", "additionalProperties": false }`.
- Missing closing brace: `"{ foo: string"` → `SchemaError::Grammar`.
- Missing colon: `"{ foo string }"` → `SchemaError::Grammar`.
- Trailing comma: `"{ foo: string, }"` → accepted.

### Conversion snapshot tests

- One snapshot per example in the Examples section above.
- Verify `additionalProperties: false` is present on every inline object.
- Verify `required` arrays are correctly populated from property-level `required` constraints.
- Verify single inline object postfix `required` is hoisted to the parent object's `required` array, while nested properties without `required` remain optional.
- Verify array-level constraints (`minItems`, `maxItems`, `uniqueItems`) are applied when `[]` suffix is present.
- Verify inline object array postfix constraints after `[]` apply to the array property, not to the `items` object schema.

### Validation integration tests

- Valid document: `authors: [ { name: "Ada", email: "ada@example.com" } ]` against `authors: "{ name: string(required), email: email }[]"` → passes.
- Missing required nested property: `authors: [ { email: "ada@example.com" } ]` → fails with path `/authors/0/name`.
- Wrong nested type: `authors: [ { name: 42 } ]` → fails with path `/authors/0/name`.
- Extra nested property rejected: `authors: [ { name: "Ada", extra: true } ]` against inline object → fails with path `/authors/0` (additionalProperties: false).
- Valid against opaque object: `authors: [ { name: "Ada", extra: true } ]` against `authors: "object[]"` → passes (existing behavior preserved).

### Compose coercion integration tests

- Nested scalar coercion: `config: { enabled: "true", retries: "3" }` against `config: "{ enabled: boolean, retries: number }"` → writes `enabled: true` and `retries: 3`.
- Nested inline object array coercion: `authors: [ { active: "true", score: "4.5" } ]` against `authors: "{ active: boolish, score: numberlike }[]"` → coerces each array item field at its nested path.
- Nested string-shaped scalar coercion: a date/url/email/file-shaped scalar inside an inline object follows the same coercion behavior as the equivalent top-level field.
- Unambiguous union coercion: build coerced candidates for each union arm; when exactly one candidate validates, commit that candidate and recurse through that arm.
- Zero-match union coercion: when no coerced candidate validates, leave the original value uncoerced and let normal validation report the failure.
- Ambiguous union coercion: when multiple coerced candidates validate, leave the original value uncoerced and let normal validation/reporting proceed without coercion guessing.

### Schema about tests

- `md schema about` renders successfully and includes sections for schema shapes, type vocabulary, constraints, inline object rules, validation behavior, and coercion.
- The schema about report is rendered from the public descriptor catalog rather than hand-maintained command-local prose.
- Descriptor traversal order is deterministic and descriptor keys/signatures are unique.
- Every `SimplifiedType` keyword has exactly one descriptor and every type descriptor maps to a parseable implemented type.
- Every implemented constraint has descriptor coverage for its accepted target types and argument form.
- Descriptor examples parse successfully, except examples explicitly marked invalid.
- Rendering `md schema about` performs no document parsing, context capture, effect-engine construction, file-reference resolution, or network access.
- Library callers can access the same schema-language descriptors used by `md schema about`.

### Backward-compatibility tests

- All existing v1 SimplifiedSchema test fixtures continue to parse, convert, and validate identically.
- `object` (opaque) still compiles to `{ "type": "object" }` without `additionalProperties: false`.

## Risks

- **Grammar ambiguity.** The `{` character is new in the type-expression grammar. It was previously an error at the start of a type expression. Inside constraint argument lists, `{` and `}` are still errors (they are not valid arg characters), so there is no ambiguity with constraint syntax.
- **Parser recursion depth.** Deeply nested inline objects (e.g. `{ a: { b: { c: { d: string } } } }`) require recursive descent in the grammar parser. The parser enforces a hard maximum of 32 inline object levels and returns `SchemaError::Grammar` when an input exceeds it.
- **`additionalProperties: false` surprise.** Authors used to the root schema's `additionalProperties: true` default may be surprised that inline objects reject extra keys. This is documented as Decision #7 and is the intended behavior; a future `lenient` constraint can address feedback.
- **Detection drift.** Because detection does not synthesize inline objects, `md schema detect` followed by manual adoption of the detected schema will still emit `object` for object properties. Authors must hand-write inline object schemas. This is accepted as a non-goal.
- **Property-level union + inline object interaction.** When an inline object arm carries `required` on some of its properties and another arm (e.g. `string`) does not, the hoisting rules must not incorrectly hoist the inline object's inner `required` to the property level. The existing hoisting logic operates on `PropertyAtom` constraints, not on nested `SchemaShape` properties, so this risk is minimal — inner `required` stays inside the inline object fragment.

## Related Work

- `darkmatter/features/_completed/2026-05-11-schemas/spec.md` — the base schemas subsystem.
- `darkmatter/features/_completed/2026-05-23-compose-schema/spec.md` — compose pipeline schema validation, which consumes the output of this feature without changes.
- `darkmatter/docs/topics/schema-definition.md` — public documentation that should be updated once this feature lands.
