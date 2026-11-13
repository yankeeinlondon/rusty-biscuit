## Global Type Constraints

Every **type** in `SimplifiedSchema` can use the following _type constraints_:

## `required`

`required` says a property must be present. Without it, a property is
optional: it may be left out, and an explicit `null` counts as left out.

```yaml
$schema:
  name: string(required)   # must be present
  nickname: string         # may be omitted or null
```

| Value | `name: string(required)` |
| --- | --- |
| `{ name: Ada }` | passes |
| `{}` | fails: `name` is missing |
| `{ name: null }` | fails: `null` is not a string |

Two cases change what `required` means:

- **A null-admitting type is always optional.** `[string(required), null]`,
  `null`, and `unknown` accept omission whatever their markers say.
- **A default satisfies it.** A required property with a `default(...)`
  passes when absent; see the next section.

In exported JSON Schema, a required property is listed in its object's
`required` array.

## `eager`



## `default(value)`

`default(value)` declares the value a property takes when it is not written.
The value must be one the type accepts (`number(default(3))`,
`string(default(draft))`); a number that cannot be held exactly is refused
when the schema loads.

What a default does depends on whether the property is required:

- **Optional property** — the default is metadata. Validation never fills
  it in; tools such as editors and completion can show it.
- **Required property** — an absent value passes through its default.
  Validation judges the value as if the default were written, and the
  success lists each default it relied on (`ValidationSuccess::defaults`).
  Strict validation leaves the value unchanged; soft validation returns a
  copy with the defaults inserted.

```yaml
$schema:
  title: string(required)
  retries: number(required; default(3))
```

`{ title: Report }` passes, with `retries = 3` reported. `{ title: Report,
retries: null }` fails: an explicit `null` is a written value, judged by the
type, and is never replaced by the default.

A default is checked against the property's complete definition before it
is used, constraints and nested required children included.
`number(required; min(5); default(3))` is a schema error as soon as a value
needs the default. Nested properties, lists, unions, and slots each have a
rule; see
[Required properties with a default](./validation.md#required-properties-with-a-default).

### Limit: other JSON Schema validators

Exported JSON Schema keeps the property in `required` and carries the
default only as the `default` annotation, which JSON Schema validators do
not act on. A third-party validator given the exported schema therefore
rejects `{ title: Report }` as missing `retries`. Accepting an absent
required property through its default is a SimplifiedSchema rule, applied
by this crate and the tools built on it; supply the value explicitly when
the data must also pass an external validator.


## `suggest(v1,v2,v3,...)`
