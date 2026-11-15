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
