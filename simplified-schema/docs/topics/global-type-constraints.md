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

`eager` says **when** a supplied value is checked, not whether it must be
supplied. A host that runs a document in phases (Claudine validates a prompt
just before it launches the agent, and again when the run completes) checks
an `eager` value at launch; anything else is checked when the run completes,
so an actor inside the run can still produce it.

```yaml
$schema:
  spec: file(required; eager)   # must exist before the agent starts
  notes: file(eager)            # optional; checked at launch when supplied
  summary: string(required)     # may be written by the run itself
```

| Declaration | At launch | At completion |
| --- | --- | --- |
| `eager` | optional; a supplied value is checked | optional |
| `required` | optional | required |
| `required; eager` | required | required |

Validation without a phase (`validate_strict`, `validate_soft`) ignores
`eager` for presence: `required` alone decides. `eager` adds nothing to exported JSON Schema except on `file`.

On `file`, `eager` also changes what a value must be. A plain `file` is
**lazy**: it only has to parse as a file reference. `file(eager)` must also
resolve to a file that exists, which needs a file context (see
[Host Capabilities](./host-capabilities.md)). It lowers to the
`darkmatter-file` format instead of `darkmatter-file-reference`.

Checking an eager value never changes it. The validation methods return the
value as written (soft validation only converts types and inserts
defaults). Darkmatter rewrites present `file(eager)` values to resolved
paths only after a composition has passed validation; a document with
validation problems is never rewritten. A `default(...)` is checked against
its definition without file access, so a `file` default is judged by syntax
there; once it stands in for an absent value it is judged like any other
value.

The full timing rules, including unions, nested objects, and `null`, are in
[Universal Constraints](./language/types.md#universal-constraints); `file`
resolution is in [Files](./language/types.md#files).

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
union arms, are a schema error before any value is judged (a key selected
only by patterns or catch-alls is the exception, judged when that key needs
its default). Nested properties, lists, `null` defaults, unions, and slots each have a rule; see
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

`suggest(...)` lists values an editor offers when completing a property. It
is **advisory**: a value outside the list is still valid if the type accepts
it. Use `enum(...)` when only the listed values are allowed.

```yaml
$schema:
  color: string(suggest(red, green, "blue gray"))
  port: number(integer; suggest(80, 443))
  tags: string(suggest(alpha, beta))[]   # candidates for each item
```

- Only `string`, `number`, and `alphanumeric` (and their lists) accept it.
- A property may carry one `suggest(...)`, across all arms of a property
  union, and it needs at least one candidate.
- Candidates of a `string` are always strings (`12` is the text `"12"`);
  candidates of a `number` must be plain decimals (`3`, `-0.5`, not `1e3`).

Exported JSON Schema carries the list as the `x-darkmatter-suggest`
annotation, which validators ignore. The parsing rules are in
[Advisory Suggestions](./language/types.md#advisory-suggestions-suggest).
