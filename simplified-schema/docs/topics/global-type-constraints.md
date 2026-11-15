## Global Type Constraints

Every **type** in `SimplifiedSchema` can use the following _type constraints_:

## `required`

`required` says a property's key must be written. Without it, a property is
optional: it may be left out, and an explicit `null` is the same as leaving
it out.

```yaml
$schema:
  name: string(required)   # must be written, as text
  nickname: string         # may be omitted or null
  note: unknown(required)  # must be written; null is a value it accepts
```

| Value | `name: string(required)` | `note: unknown(required)` |
| --- | --- | --- |
| `{ name: Ada, note: x }` | passes | passes |
| `{}` | fails: `name` is missing | fails: `note` is missing |
| `{ name: null, note: null }` | fails: `null` is not a string | passes |

`required` only asks whether the key was written; the type then judges the
value, so a written `null` passes only a type that admits it. A required
property with a `default(...)` also passes when the key is missing or
`null`; see the next section.

In exported JSON Schema, a required property is listed in its object's
`required` array.

## `eager`



## `default(value)`

`default(value)` declares the value a property takes when its key is missing
or its value is `null`, whether or not the property is `required`. The value
must be one the type accepts (`number(default(3))`, `string(default(draft))`);
a number that cannot be held exactly is refused when the schema loads.

```yaml
$schema:
  title: string(required)
  retries: number(default(3))
```

`{ title: Report }` and `{ title: Report, retries: null }` both pass, with
`retries = 3` applied. A written value always wins: `retries: 0` stays `0`.

Applying defaults is a step of its own, before validation. Validation then
judges the completed value and reports each default applied
(`ValidationSuccess::defaults`). Strict validation leaves your value
unchanged; soft validation returns a copy with the defaults in place. On a
`required` property a default also meets the presence contract, so
`retries: number(required; default(3))` passes `{ title: Report }` too.

A default is checked against the property's complete definition when a
value needs it, constraints and nested required children included:
`number(min(5); default(3))` is a schema error as soon as a value leaves
`retries` unset. Two different defaults for one property, for example on two
union arms, are a schema error before any value is judged. Nested
properties, lists, `null` defaults, unions, and slots each have a rule; see
[Applying defaults](./validation.md#applying-defaults).

### Limit: other JSON Schema validators

Exported JSON Schema keeps a required property in `required` and carries the
default only as the `default` annotation, which JSON Schema validators do
not act on. A third-party validator given the exported schema therefore
rejects `{ title: Report }` when `retries` is `required`, and leaves an
optional `retries: null` as `null`. Applying defaults is a SimplifiedSchema
step, performed by this crate and the tools built on it; supply the value
explicitly when the data must also pass an external validator.


## `suggest(v1,v2,v3,...)`
