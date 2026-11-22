---
kind: feature
created: 2026-11-22
status: draft-spec
area: darkmatter
owner: Ken Snyder <ken@ken.net>
related:
    - 2026-11-22-host-registration-v2
    - 2026-11-22-claudine-doctor
depends-on:
    - 2026-11-22-claudine-doctor
---

# Expression and mutation schemas

A short spec: it records what was decided and leaves the design for when the
work is scheduled.

## Why this exists

Two things Darkmatter and its hosts provide are described in code, or in
special-purpose files, rather than as schema files of their own kind:

- the **expression functions** any expression scope can call
  (`dasherize(name)`, `load_data(...)`, …), today Darkmatter's built-in
  catalog plus a host manifest's `functions:` (a catalog file and a generated
  facts file per set);
- the **mutation functions** (lifecycle verbs) a lifecycle event may run,
  today defined in `darkmatter/schemas/partials/lifecycle.yaml` and referenced
  by hosts through the built-in `darkmatter:` namespace.

## Decisions

Two new kinds, alongside `namespace-schema` from
`2026-11-22-host-registration-v2`:

```yaml
kind: expression-schema
$schema: { ... }
types: { ... }
```

```yaml
kind: mutation-schema
$schema: { ... }
types: { ... }
```

- Darkmatter defines its own built-in functions and verbs with these kinds.
- A host may register its own files of these kinds to extend Darkmatter. This
  replaces the manifest's `functions:` field.
- Signatures use SimplifiedSchema's existing `function(...)` type
  (`simplified-schema/docs/topics/language/unions.md`, "Function Signatures"),
  which is already legal in a schema file's `types:`.

## To settle when scheduled

- **The line between the two kinds**, in one sentence: a reasonable starting
  point is that an expression function returns a value and changes nothing,
  while a mutation function is an action a lifecycle event runs, with effects.
- Where an expression function's implementation lives, given that a
  registration is passive and never runs host code (today: the facts file
  describes the function, and the host runs it).
- Whether a `when` on these kinds (and on `namespace-schema`) should say
  which events or scopes may use them, replacing the availability rules
  currently held in code and in generated lifecycle artifacts.
- Migration of Darkmatter's built-in catalog and lifecycle verb partials
  into the new kinds.
