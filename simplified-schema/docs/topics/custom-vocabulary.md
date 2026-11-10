# Custom Vocabulary

A SimplifiedSchema lowers to Draft 2020-12 JSON Schema, but several of its
types mean more than standard JSON Schema can say: a `file` that must exist,
a `datetime` whose offset is optional, text that must parse as YAML. The
lowered schema expresses these with custom `format` values named
`darkmatter-*` and custom keywords named `x-darkmatter-*`. This page lists
each one, the type that emits it, and what checks it.

Use it when you export a schema (`SimplifiedSchema::to_json_schema`,
`to_json_schema`) and hand it to another tool, or when you write raw JSON
Schema that this library will judge.

## The Contract

The library's validators (`SimplifiedSchema`, `Prepared`, and the builders in
`validate`) register every format and keyword below and assert formats. A
third-party JSON Schema validator does not know them. Draft 2020-12 treats an
unknown `format` as an annotation and ignores an unknown keyword, so such a
validator:

- **accepts more** wherever a check below is the only thing standing between
  a value and acceptance: any string passes `format: darkmatter-yaml`, a
  missing file passes `format: darkmatter-file`, and `2026-13-99T00:00:00`
  passes `format: darkmatter-datetime`;
- **accepts less** where the library uses an annotation to *relax* a check:
  a `{{ … }}` template under `x-darkmatter-deferred` is judged as text against
  the declared type, so it fails a `number`;
- still enforces everything standard the lowering emits beside them: `type`,
  `pattern`, bounds, `required`, `enum`, `const`, `prefixItems`,
  `additionalProperties`, and so on. A
  [`non-null`](./language/types.md#non-null-properties) property and a
  [closed root](./language/objects-and-named-types.md#closed-roots-closed)
  lower to standard JSON Schema only, so any validator enforces them.

```json
{
  "type": "object",
  "properties": {
    "notes": { "type": "string", "format": "darkmatter-yaml" },
    "when":  { "type": "string", "format": "darkmatter-datetime" }
  }
}
```

With this library, `{ "notes": "a: [", "when": "2026-07-10T15:05:34" }` fails
on `notes` (not valid YAML) and passes on `when` (an offset-less datetime is
allowed). A validator that ignores the formats passes both.

Only the validity of the data is at stake: the extra entries never make the
exported schema invalid JSON Schema.

## Formats

Each format is registered as a `format` validator; it sees only the string
value. The constants live in `simplified_schema::format`.

| Format | Emitted for | The library checks |
| --- | --- | --- |
| `darkmatter-file` | `file(eager)` | The value parses as a file reference, resolves in the schema's file context, and names an existing file. Needs a file context; see [Host Capabilities](./host-capabilities.md). |
| `darkmatter-file-reference` | `file` | The value parses as a file reference. Syntax only: nothing is resolved and existence is not checked. |
| `darkmatter-yaml` | `yaml` | The text parses as YAML with no duplicate key (JSON passes too). |
| `darkmatter-json` | `json` | The text parses as strict JSON: no YAML-only syntax, no duplicate key, no number outside `f64` (`1e400`). |
| `darkmatter-numberlike` | `numberlike` | A string is text the [numeric model](./foundations.md#numbers-numeric) admits as a number, exactness included. Emitted beside `"type": ["number", "string"]`. |
| `darkmatter-boolish` | `boolish` | A string is a boolean word (`true`/`false`, `yes`/`no`, `on`/`off`, `1`/`0`, any case). Emitted beside `"type": ["boolean", "string"]`. |
| `darkmatter-expression` | `expression` | The text parses as an [expression](./expression-syntax.md). It is never evaluated; a value still holding a `$(...)` or `{{ … }}` placeholder is accepted as pending. |
| `darkmatter-shell` | `shell` | The text has at least one non-whitespace character. It is never parsed or run. |
| `darkmatter-datetime` | `datetime` | An ISO 8601 datetime with an optional offset, range-checked (`2026-13-99T00:00:00` fails). The built-in `date-time` format requires an offset. |
| `darkmatter-time` | `time` | A time of day with an optional offset. The built-in `time` format requires one. |

## Keywords That Assert

These are custom keywords: they decide validity in this library and are
ignored elsewhere.

| Keyword | Emitted for | The library checks |
| --- | --- | --- |
| `x-darkmatter-url-scheme` | `url(scheme(...))` | The URL's scheme is one of the listed schemes, compared case-insensitively. Emitted beside `format: uri`. |
| `x-darkmatter-integer` | `number(integer)` | The whole number lies below 2^64 in magnitude. Emitted beside `"type": "integer"`, which has no such ceiling. Its only legal value is `true`. |
| `x-darkmatter-type-definition` | `type-definition`, `type-definition(forms(...); declared-required; container)` | The value (a string, mapping, or sequence) is a valid SimplifiedSchema property definition that satisfies the payload's options. Parse-only; nothing is resolved. See [Declaration-option payloads](#declaration-option-payloads). |
| `x-darkmatter-schema` | `schema`, `schema(mapping; property(...))` | The value is a valid `$schema` declaration that satisfies the payload's options. Parse-only; referenced files are not read. See [Declaration-option payloads](#declaration-option-payloads). |
| `x-darkmatter-match` | `file(match(...))` on a file property of a [root-union](./language/unions.md#root-level-unions) arm | An existing file named by the value matches the arm's globs, so a file outside them rules the arm out. On an ordinary property `match(...)` only suggests and is not emitted. Needs a file context. |
| `x-darkmatter-match-property` | beside `x-darkmatter-match`, inside an arm's copy of a recursive definition | Names the top-level property the copy types (`null` below a pattern key). It carries data for `x-darkmatter-match` and checks nothing itself. |

An `x-darkmatter-match` value is one group of patterns (`["*.md", "!_*.md"]`)
or several groups (`[["*.md"], ["guide*"]]`); a path must satisfy every
group. It is generated metadata, not something to author by hand.

### Declaration-Option Payloads

The payload of `x-darkmatter-type-definition` and `x-darkmatter-schema` is
`true` when the meta-type has no
[declaration options](./language/schema-types.md#narrowing-a-declaration),
and an options object when it has some. This declaration:

```yaml
$schema:
    contract: schema(required; mapping; property(kind, type-definition(forms(literal, enum); declared-required)))
```

lowers `contract` to:

```json
{
  "x-darkmatter-schema": {
    "mapping": true,
    "property": [
      { "name": "kind", "declaration": { "forms": ["literal", "enum"], "declaredRequired": true } }
    ]
  }
}
```

| Payload key | Of | From | The library checks |
| --- | --- | --- | --- |
| `mapping: true` | `x-darkmatter-schema` | `mapping` | The declaration is written as a mapping, not a list or a reference. |
| `property: [{ "name", "declaration" }]` | `x-darkmatter-schema` | each `property(NAME, type-definition(...))` | Every inline mapping, or inline root-union arm, declares `name`, and its declaration satisfies `declaration`, itself a type-definition payload (`true` for a bare `type-definition`). Reference carriers and reference arms are skipped. |
| `forms: ["literal", ...]` | `x-darkmatter-type-definition`, or a `declaration` | `forms(literal, ...)` | Every top-level alternative is a non-array atom with a listed type keyword; a `literal` or `enum` one denotes non-empty strings, and a named-type alternative passes. |
| `declaredRequired: true` | `x-darkmatter-type-definition`, or a `declaration` | `declared-required` | The definition is `required`, with no alternative that admits null. |
| `container: true` | `x-darkmatter-type-definition`, or a `declaration` | `container` | Every non-null alternative is a list, tuple, object, or dictionary, and at least one exists; a named-type alternative passes. |

Raw JSON Schema may write the payload directly and is judged exactly like a
lowered one. An absent or `false` flag (`declaredRequired`, `container`)
leaves its check off. A malformed payload (`false`, an unknown key such as `closed`, a
non-boolean `mapping`, `declaredRequired`, or `container` (`null` included), a
`property` that is not an array) makes the schema
fail to prepare, with an error naming the keyword.

## Annotations

These never decide validity in any validator. The library's conversion engine,
schema views, and completion read them; a host may read them too.

| Annotation | Emitted for | What reads it |
| --- | --- | --- |
| `x-darkmatter-deferred` | `deferred` | Validation accepts a `{{ … }}` template at the marked node unjudged. Raw JSON Schema may write `x-darkmatter-deferred: true` to the same effect. |
| `x-darkmatter-operands` | `operands(<verb>-short)` / `operands(<verb>-long)` | A host that knows the verb (Darkmatter's lifecycle) binds the operands through the verb's registration. |
| `x-darkmatter-uncounted-keys` | `uncounted-keys` | Conversion reads it in place of the `allOf` conditions that enforce the key count; those conditions are standard and every validator enforces them. |
| `x-darkmatter-type`, `x-darkmatter-options` | `alphanumeric(...)` | Conversion keeps a native number or boolean from becoming text; the type's `pattern` does the checking. |
| `x-darkmatter-generated` | `generated` | Marks a value the host supplies later; the property is left out of the static `required` list. |
| `x-darkmatter-example` | `example(...)` | The resolved example objects, for documentation and editor hovers. |
| `x-darkmatter-suggest` | `suggest(...)` | The interpreted suggestion candidates, in order. |
| `x-darkmatter-category` | `category(Name)` | Documentation grouping. |

## Related

- [Types and Constraints](./language/types.md): the types that emit these.
- [Host Capabilities](./host-capabilities.md): which checks need a file
  system, and what happens in a build without one.
- [Strict and Soft Validation](./validation.md): the validators that apply
  this vocabulary.
