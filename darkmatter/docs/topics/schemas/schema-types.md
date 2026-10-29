---
kind: canonical-documentation
---
# Choosing Schema Types

A SimplifiedSchema type describes the values a frontmatter property accepts.
Use a concrete type when a consumer needs a particular shape, and `unknown`
when the value is deliberately unrestricted. This guide explains the main
choices; the [complete type reference](./definition.md#types) lists every
built-in type and its constraints.

```yaml
$schema:
  title: string(required)
  count: number(min(0))
  slug: alphanumeric(allow-dashes; min(1); max(40))
  payload: unknown
  retired: null
  build: shell
  tags: string[]
```

The schema above requires `title`. The other properties may be omitted.
`slug: release-2` is valid, while `slug: release/2` is invalid because slashes
are not allowed. `payload` may hold a scalar, list, mapping, or null. `retired`
may only be omitted or null. `build` holds nonblank command text; validating
it does not execute it.

## Pick the shape the consumer needs

| Need | Type | Example declaration |
| --- | --- | --- |
| Text | `string` | `title: string(required; min(1))` |
| A number or boolean | `number`, `boolean` | `attempts: number(integer; min(1))` |
| A numeric or boolean value that keeps its original spelling | `numberlike`, `boolish` | `enabled: boolish` |
| A fixed choice or exact scalar | `enum`, `literal` | `mode: enum(fast, thorough)` |
| A date, time, URL, or email address | `date`, `datetime`, `time`, `url`, `email` | `contact: email` |
| A local file reference | `file` | `input: file(required; eager)` |
| ASCII identifier text with chosen punctuation | `alphanumeric` | `slug: alphanumeric(allow-dashes; min(1))` |
| Command text | `shell` | `build: shell(required)` |
| Text containing structured data or an expression | `yaml`, `json`, `expression` | `condition: expression` |
| Arbitrary data, including null | `unknown` (alias `any`) | `payload: unknown` |
| A property that must be absent or null | `null` | `retired: null` |
| An object with any fields | `object` | `metadata: object` |
| An object with declared fields | Inline object or named type | `owner: "{ name: string(required) }"` |
| A list or a fixed sequence of positions | `T[]`, `tuple` | `tags: string[]` |

`function(...)`, `type-definition`, and `schema` serve schema authors and
library integrations rather than ordinary text fields. See the
[reference](./definition.md#types) for their placement and validation rules.
Named types reuse these shapes; a bare name selects a local definition unless
it is a built-in keyword. Use `shell@this`, for example, to select a local
type named `shell` rather than the built-in command type.

## Missing values and null

A concrete property is optional unless it carries `required`. Optional
properties also accept null as an absent value. A type that explicitly admits
null as a whole value is always optional, even if it carries or inherits
`required`:

| Declaration | Missing or null | Other accepted values |
| --- | --- | --- |
| `name: string(required)` | Rejected | Text, with ordinary scalar coercion |
| `payload: unknown(required)` | Accepted | Every value, with its native type preserved |
| `retired: null(required)` | Accepted | None |
| `name: [string(required; min(3)), null]` | Accepted | Text of at least three characters |
| `items: unknown[](required)` | Rejected | A list whose items may have any type, including null |
| `items: null[](required)` | Rejected | A list containing only null items |

`any` and `unknown` mean the same thing. Serialization and editor hover use
`unknown` as the canonical name. Neither spelling can enforce a property's
presence; choose a concrete type for that requirement. A native YAML null
(`retired: null`, `retired: ~`, or `retired:`) declares the null type at a
property-definition position. A null root schema is invalid.

A root union accepts one or more matching alternatives. To require exactly
one of two inputs, explicitly exclude the other in each alternative:

```yaml
$schema:
  - input: file(required)
    review: null
  - review: file(required)
    input: null
```

Exactly one file must be supplied; the other may be omitted or null. See
[Null-Admitting Properties](./definition.md#null-admitting-properties) and
[Root-Level Unions](./definition.md#root-level-unions) for the complete rules.

## Identifiers and command text

`alphanumeric` accepts ASCII letters and digits. Options such as
`allow-dashes`, `allow-underscores`, `allow-slashes`, `allow-periods`, and
`allow-whitespace` widen that set; `allow-whitespace` permits only an ASCII
space. Restrictions such as `leading-alpha` and `block-double-periods` narrow
it. Write comma-separated options before `;` and ordinary constraints after
it: `alphanumeric(allow-dashes, leading-alpha; min(1))`.

The empty string is allowed unless a constraint rejects it. Non-ASCII
letters, tabs, and newlines are never accepted. Native numbers and booleans
are type mismatches, so quote digit-only identifiers in YAML. Defaults and
suggestions must themselves satisfy the type. The
[alphanumeric reference](./definition.md#alphanumeric-strings) explains all
nine options, canonical ordering, and named-type compatibility.

`shell` accepts text containing at least one non-whitespace character. It
checks neither shell syntax nor authorization: `echo ready | cat` is valid
command text, and validation runs nothing. Native boolean and numeric values
use the same text coercion as `expression`. Exported JSON Schema uses the
custom format `darkmatter-shell`; external validators must enforce that
format to reject blank strings. See [Shell Commands](./definition.md#shell-commands).

## Inspect types from the CLI or editor

Run `md schema about` for the implementation's type and option catalog.
DMLS offers the same keywords while editing schemas, labels `any` as an
alias, and explains the effective character set of an `alphanumeric` type on
hover. See [DMLS Schema Support](./dmls-schema-support.md).

Library callers can compare resolved types with
[`is_subtype`](./definition.md#subtype-relation-is_subtype), for example to
ask whether an `alphanumeric` argument fits a rule allowing slashes. That
comparison ignores ordinary constraints and property presence; callers still
validate actual values and apply their own shell quoting and path policy.
