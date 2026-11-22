---
kind: feature
created: 2026-11-22
status: draft-spec
area: simplified-schema
owner: Ken Snyder <ken@ken.net>
related:
    - 2026-11-22-host-registration-v2
next-spec: 2026-11-22-host-registration-v2
---

# Object composition: `...spread` and `$extends`

## Why this exists

SimplifiedSchema has no way to build one object type out of another. A schema
that needs the same set of properties in several places (Claudine's lifecycle
events, a sequence task and a group leg, a namespace and a sub-namespace)
copies them, and the copies drift. This spec adds two ways to say "this object
has all of that object's properties, plus these":

- **spread**, inside the string grammar for inline objects:
  `"{ ...events, id: string }"`
- **`$extends`**, a reserved key in a YAML mapping:

  ```yaml
  $schema:
      $extends: events
      foo: string
      bar: number
  types:
      events: "{ id: string, price: number }"
  ```

Both forms mean the same thing and resolve to an ordinary object: after
resolution nothing records that a property came from a spread.

## Decisions

### Spread (`...name`)

A spread item is allowed wherever an inline object lists a property:

```yaml
types:
    lifecycle: "{ ...initialize, ...start, ...blocked, ...success, ...failure, ...finalize }"
    priced: "{ ...events, price: number(min(0)) }"
    remote: "{ ...events@./events.yaml, url: url }"
```

- The target is a **named type**: a local `types:` name, `Name@./file.yaml`,
  or `Name@this`. Anything a named reference can name today, a spread can
  name.
- **Last one wins.** Items apply left to right, and a later item replaces an
  earlier property of the same name, whether that later item is a spread or
  a property written in place. So in `{ id: number, ...events }`, `id` comes
  from `events`, and in `{ ...events, id: number }`, `id` is a `number`.
- A spread carries no constraints of its own: `...events(required)` is a
  grammar error. Constraints belong to the properties being copied, or to the
  property that holds the whole object (`"{ ...events }(required)"`).

### `$extends`

`$extends` is a reserved key in any YAML mapping that describes an object:
the root of `$schema`, each arm of a root `$schema` union, a `types:` entry
written as a mapping, and a nested property mapping.

```yaml
types:
    order:
        $extends: [events, audit@./audit.yaml]
        total: number
```

- The value is one named type or a list of them.
- **Order:** the extended types apply first, in list order, then the
  mapping's own properties. Last one wins throughout, so `audit` replaces a
  property of `events`, and the mapping's own `total` replaces either. Where
  `$extends` appears among the mapping's keys does not matter.
- `$extends: [a, b]` plus own keys is exactly
  `"{ ...a, ...b, <own keys> }"`.
- **Reserved:** a schema can no longer declare a property named `$extends`.
  An existing declaration of that name becomes a load error that names this
  rule. (No schema in the repository declares one today; the implementation
  confirms that.)

### Errors

Each of these is a schema load error naming the spread or `$extends` site
and the target:

| Case | Example |
|---|---|
| the target is not an object type | `...name` where `name: string` |
| the target is a union, even of objects | `...shape` where `shape: ["{ a: string }", "{ b: number }"]` |
| the target is an array of objects | `...rows` where `rows: "{ a: string }[]"` |
| the target is the open `object` type, which lists no properties | `...meta` where `meta: object` |
| the target cannot be resolved | `...missing`, or a file that does not load |
| the composition is cyclic | `a: "{ ...b }"`, `b: "{ ...a }"` |

A target that is an alias of an object (`events: base`, `base: "{ … }"`) is
followed to the object.

### What is copied

- Each property is copied whole: its type, its constraints, its
  `required` and default, its description, and its examples. An inherited
  `example(...)` reference still resolves from the file that declared it, as
  it does for named-type imports today.
- Pattern keys (`"<string>": unknown`) are copied like any other key, and
  last one wins applies to them as well.
- Whether the resulting object is open or closed is the **receiving**
  object's setting, never the target's.

### Resolution and output

- Composition is resolved when named types are resolved, before conversion
  to JSON Schema. The converted schema is a flat object with the merged
  `properties` and `required`.
- It is **never** converted to `allOf`. With `additionalProperties: false` on
  each branch, `allOf` rejects every property the other branch declares, so
  a flattened object is the only correct output.
- Diagnostics about a copied property point at the declaration in the target
  file, the same way named-type import diagnostics do today.
- `is_subtype` compares the resolved, flattened objects, so a composed
  object and the same object written out by hand are the same type.

### Editor support (DMLS)

- After `...` and in a `$extends` value, completion offers the type names in
  scope, including `Name@` import forms.
- Hover and go-to-definition on a spread target behave as they do for any
  named-type reference.

## Out of scope

- Removing a property from a composed object ("everything in `events`
  except `id`"). Last one wins can override a property but not delete it.
- Spreading arrays or unions.
- Any change to how triggers or document layers merge. Those are separate
  layers with their own rules (see `2026-11-22-host-registration-v2`).

## Documentation to update

- `simplified-schema/docs/topics/language/objects-and-named-types.md`: a
  section on composition (both forms, last one wins, the error table, the
  `allOf` note).
- `simplified-schema/docs/topics/language/index.md` and the grammar
  references that list inline-object syntax.
- `simplified-schema/docs/topics/diagnostics.md`: the new error codes.
- The `simplified-schema` repo skill and the `use-dmls` caller skill (schema
  authoring), then `just publish-skills` in the areas that own them.

## Tests

- Spread at the start, middle, and end of an inline object, with a property
  of the same name before and after it.
- `$extends` with one name, a list, and an import; with own keys that
  override; with `$extends` written after the own keys (same result).
- `$extends` at the `$schema` root, in a root-union arm, in a `types:`
  mapping, and in a nested property mapping.
- Each row of the error table.
- A declared property named `$extends` is a load error.
- The converted JSON Schema is flat (no `allOf`) and validates a document
  exactly as the hand-written equivalent does.
- `is_subtype` treats a composed object and its hand-written equivalent as
  the same type.
- DMLS completion after `...` and in `$extends`.
