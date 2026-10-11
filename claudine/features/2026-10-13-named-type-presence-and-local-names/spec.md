---
area: darkmatter
status: finalized-spec
created: 2026-10-10
owner: Ken Snyder <ken@ken.net>
$schema:
    status: |-
        enum(
            draft-spec,
            finalized-spec,
            planned,
            implemented,
            review-findings,
            human-in-the-loop,
            completed,
            on-hold,
            abandoned
        ) -> an indicator of progress for this specification
    reviewed: boolean -> indicates whether the specification file has been reviewed by another agent from the one which created the spec
    reviewed_by: string -> the agent and model used in the spec review
    reviewed_on: date -> the date the spec was reviewed
    review_iterations: number -> the number of implementation reviews have taken place in the review/fix cycle
    implemented: boolean -> indicates whether this spec's plan has been implemented
    implemented_by: string -> the agent who implemented the plan
    formerly: string -> the spec's previous `{date}-{name}`, when it was renamed in the 2026-10-10 restructure
    pipeline_step: number -> position in the claudine/docs/rollout-strategy.md pipeline
reviewed: false
human_review: false
pipeline_step: 3
related:
    - 2026-10-14-union-type-constraints
    - 2026-10-16-core-schema-types
    - 2026-10-17-content-policy-base-schema
---

# Named Types Keep Their Constraints; Local Types by Bare Name

SimplifiedSchema lets a schema define reusable **named types** and reference
them from properties. Two things make that harder than it should be today:

1. When a named type is reused, the resolver silently **strips** its
   top-level `required` and `generated` constraints, so `types: { req:
   expression(required) }` used as `when: req@this` makes `when` optional.
   Authors have to repeat `(required)` at every use site, and forgetting it is
   invisible.
2. A type defined in the same file must be referenced as `name@this`; a bare
   `name` is an "unknown type" error.

After this change a named type carries **all** of its constraints wherever it
is used, a same-file type is referenced by its bare name, and use-site
constraints merge with inherited ones under one predictable rule.

```yaml
kind: schema
types:
    person: "{ name: string(required), email: email }"
    code: string(required; generated)
    err: "{ code: code, author: person, reviewer: person@./people.yaml }"
    # `code` and `person` are same-file types referenced bare (no @this);
    # err.code inherits required and generated; @ is for other files.
```

## Decided

### 1. Presence constraints survive every reference

Decided 2026-10-10 (confirming the owner ruling of 2026-09-19): stripping
`required` and `generated` from a reused named type was an oversight, not a
design decision. Every constraint declared on a named type, including
`required` and `generated`, survives every reference to it: bare local
references, `@this`, cross-file `@./file.yaml` imports, and multi-hop chains
where one named type references another.

An inherited constraint behaves exactly as if it were written at the use site.
For example, `required` inherited onto `people: person[]` marks `people`
required, the same as writing `person(required)[]` does today, and a
`required` inherited into one arm of a property-level union is hoisted to the
property as `required` on any arm already is.

### 2. No use-site opt-out

There is no syntax to remove an inherited `required` (or `generated`) at a use
site. An omitted postfix never resets the inherited definition. An author who
needs the whole value to be optional writes a union with `null`:

```yaml
types:
    draft_owner: [person, null]   # optional: null or omitted, else a person
```

A variant whose inner field is optional is a different type and is declared as
one. The `null` type and the rule that a union containing a `null` arm is never
required land in `2026-10-16-core-schema-types`; the compact
`person | null` spelling lands in `2026-10-28-schema-enhancements`. This spec
only removes the stripping.

### 3. Bare local names

A type defined in the same file is referenced by its bare name: `author:
person`. The meaning is identical to `person@this`, which remains supported.
`@` is for types from other files. A bare reference survives renaming the
file, which `@./file.yaml` self-references do not.

Resolution rules:

- **Where local types live.** A bare name resolves in exactly the namespace
  `@this` resolves in today: the named types of the schema where the
  reference is written (the `types` table of a `kind: schema` file; the
  top-level `$schema` entries of a document or plain schema file).
- **Built-in keywords win.** A bare name that is a built-in type keyword
  (`string`, `email`, `file`, …) means the built-in. `string@this` selects a
  local type named `string`.
- **Unknown names are errors.** A bare name that is neither a built-in nor a
  local type is a typed resolution error naming the type and the schema file.
  It is never treated as `any`, never looked up in other active schemas,
  frontmatter, or runtime globals.
- **Resolution stays in the defining file.** If `address@./contacts.yaml`
  uses a bare `postcode`, that `postcode` comes from `contacts.yaml`, not from
  the importing document.
- **Same forms everywhere.** A bare reference accepts every postfix an
  explicit reference does: `person[]`, `person(required)`, `person[](min(1))`,
  `-> description`, and use inside union arms and inline objects. Forward
  references see the complete local table. Cycles are rejected exactly as for
  `@this` (`SchemaError::ImportCycle`).

### 4. Constraint merging

When a use site adds constraints to a named type, inherited and use-site
constraints combine. This is a merge within one reused type; it does not
change schema layering, where the winning tier's whole property definition
replaces lower tiers.

| Situation | Result | Example |
|---|---|---|
| Different constraints | Both apply | `label: string(min(3))`, use `label(max(80))` → 3..80 characters |
| Repeated lower bound | The larger (stricter) wins | `min(3)` + `min(5)` → `min(5)` |
| Repeated upper bound | The smaller (stricter) wins | `max(80)` + `max(50)` → `max(50)` |
| Contradictory bounds | Structured schema error | `min(10)` + `max(5)` |
| Flags (`not-empty`, `integer`, `unique`, `eager`, `required`, `generated`) | Idempotent | `required` twice = `required` |
| Repeated `pattern(…)` / `match(…)` | All apply (a value must satisfy each) | |
| Repeated `scheme(…)` | Intersection; an empty intersection is a contradiction error | `scheme(http,https)` + `scheme(https)` → `scheme(https)` |
| Repeated `default(…)` | Equal values are fine; different values are a contradiction error | |
| Repeated `suggest(…)` | Error, as today (at most one `suggest`) | |

Bounds are compared per kind: string length `min`/`max`, number `min`/`max`,
array `min`/`max` (item counts), and `min-keys`/`max-keys`. Array-element
constraints and array-container constraints keep their separate meanings and
merge only with their own kind.

A contradiction error is a structured `SchemaError` that names the property,
the conflicting constraints, and where each came from (the named type's
definition and the use site), so DMLS can point at both.

Preserving `generated` does not invent a value provider and does not change
its validation-phase semantics.

## Requirements

- Delete `strip_top_level_presence` and its call in `expand_import`
  (`darkmatter/lib/src/markdown/schemas/resolve.rs`), and replace the plain
  `extend` in `apply_import_postfix` with the merge rule above.
- The type-expression parser keeps a bare non-keyword identifier, with its
  authored span, as a local reference; the resolver expands it through the
  same named-import path as `@this`. Bare and explicit references must agree
  for scalar, array, union and constrained types, forward references, missing
  types, cyclic imports, descriptions, and defining-file origins.
- Runtime resolution, validation, generated JSON Schema, and DMLS use the same
  parser and resolver. DMLS completion offers same-file names bare (today it
  offers `Name@this`), and hover / go-to-definition work on bare references.
- **Test-suite audit.** Remove or update every test and comment that requires
  presence constraints to be stripped on reuse.
- **Shipped-schema audit.** Find every shipped schema (under `*/schemas/`,
  `*/docs/schemas/`, the built-in baseline, and inline `$schema` blocks in
  shipped documents) whose named types carry `required` or `generated` and are
  referenced elsewhere. For each, confirm the inherited presence is intended;
  where a reference relied on stripping to be optional, express that
  optionality explicitly in the schema (an optional variant type or, once
  `2026-10-16-core-schema-types` lands, a `null` union). Record the audit
  result (files checked, changes made) in the implementation log.
- **Docs.** In `darkmatter/docs/topics/schemas/local-type-references.md` and
  `authoring-schemas.md`, remove the "Implementation status" notes saying bare
  names are unsupported and stripping still happens, and correct the length
  constraint spelling in the examples to the grammar's actual `min(n)` /
  `max(n)` (the docs currently write `min-length` / `max-length`, which the
  grammar does not accept). In `definition.md`, update the Cross-File
  Named-Type Imports section to say constraints merge (with the table above)
  and that same-file types may be referenced bare. Update
  `dmls-schema-support.md` where it says names are offered as `Name@this`.
  Update the `darkmatter` skill's schema notes.

## Out of scope

- `[]`, list constraints, and `required` on a reference to a **union-typed**
  named type: `2026-10-14-union-type-constraints`.
- The `null` type and union-with-`null` optionality:
  `2026-10-16-core-schema-types`.
- The `|` union spelling: `2026-10-28-schema-enhancements`.

## Acceptance criteria

1. Given `types: { code: string(required; generated) }`, the properties
   `a: code`, `b: code@this`, and `c: code@./self.yaml` (a cross-file import of
   the same definition) all resolve to required-and-generated properties, with
   identical JSON Schema.
2. The spike case: `types: { req: expression(required) }` used as
   `when: req` inside a mapping type, and as a top-level property, makes the
   property required in both positions.
3. Coverage: required-only, generated-only, and combined constraints;
   optional (unconstrained) definitions; multi-hop local and cross-file
   chains; nested objects, arrays (`person[]` inherits `required` onto the
   property), and union arms.
4. Merge: additional use-site constraints apply; repeated bounds keep the
   stricter value for string length, number, array item count, and
   `min-keys`/`max-keys`; `min(10)` + `max(5)` yields a structured error naming
   both origins; repeated `pattern` requires both; `scheme` intersects;
   conflicting `default`s error.
5. Bare names: a bare local reference and its `@this` form produce identical
   resolved schemas for every form in Decided 3; a bare `string` with a local
   type named `string` resolves to the built-in while `string@this` selects
   the local type; an unknown bare name yields the typed resolution error
   naming the type and file; a bare name inside an imported definition
   resolves in its defining file.
6. DMLS: completion offers a same-file type bare; hover on a bare reference
   shows the local definition; a contradiction error is reported as a schema
   diagnostic.
7. The test-suite and shipped-schema audits are done and recorded; no test
   still asserts stripping.
8. `just test` and `just lint` pass in `darkmatter/`, plus `just test` in any
   package area whose shipped schema the audit changed. Tests are L1, use no
   terminal or browser windows, and are portable to macOS, Linux, Windows, and
   WSL2. No new CI cells.

## Open questions

None.
