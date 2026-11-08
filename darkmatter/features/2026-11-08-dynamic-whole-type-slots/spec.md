---
kind: feature
created: 2026-11-08
status: draft-spec
area: darkmatter
related:
    - 2026-11-05-dynamic-schema-types
---

# Whole-type schema slots

## Summary

`2026-11-05-dynamic-schema-types` delivered `{{ … }}` slots in value
positions: `enum(...)` and `suggest(...)` elements, and the whole argument of
`literal(...)`, `default(...)`, `min(...)`/`max(...)`, and `pattern(...)`.
Ruling Q16 sequenced the remaining position, a slot that supplies a
property's **whole type definition**, into this follow-up. Deferring it was a
matter of implementation cost, not of budget or safety: investigation S8
found it well defined once the import base was ruled, and Q16 ruled it.

```yaml
$schema:
    status: "{{ load_data('./status-type.yaml', 'type-definition') }}"
    owner: "{{ load_data('./owner-type.yaml', 'type-definition') }}(required)"
```

Everything the value-position feature built (passive parsing, admission,
the approval gate and grant store, slot sessions, the process cache, the
DMLS worker) is reused unchanged. This spec adds only what a whole type
needs on top of it.

## What S8 found

- **Result shape.** A slot yields a `type-definition` value, the same domain
  as the `type-definition` type, parsed by the existing
  `parse_type_definition_value` (a string or the YAML list form), producing a
  `PropertyDef`.
- **Placement.** Wherever a property definition or a union arm stands; the
  slot is the whole YAML value or arm. A postfix (`{{ … }}(required)`,
  `{{ … }}[]`) reuses the use-site merge of `Name@file(postfix)`: until
  resolution the slot behaves like a named reference whose body is computed
  (a `TypeExpr::Slot` that acts like `TypeExpr::Imported`).
- **Admission of the returned definition.** It must contain no `{{ … }}`
  (returned data never introduces another evaluation), must pass
  `non_null_problem` and the other definition checks, and is expanded with
  the normal named-type resolver.
- **YAML hazard.** An unquoted value starting with `{{` is a YAML flow
  mapping, not a scalar. When a value is a mapping shaped like
  `{{…}: null}`, the parser must report "quote the slot" rather than a
  generic type error.
- **Editor.** Much weaker than value positions. Until resolution the
  property's type, presence (`is_required`), structure, and nested keys are
  unknown, so key completion, hover, `md schema` assignment, and phase
  projection all need a "pending type" answer. There are no literal parts to
  show while pending.
- **Cost.** High: a new `TypeExpr` variant meets about 44 files of exhaustive
  matches (18 in `simplified-schema`, 24 in Darkmatter's lib, CLI, and DMLS,
  2 in Claudine), plus subtyping, narrowing, frontmatter shape, and lint,
  each of which needs a pending case.

## Rulings carried in

- **Q16 import rule.** An import inside a returned type definition
  (`Name@./x.yaml`) resolves against the **slot's defining schema file**,
  consistent with Q3's "the schema's context". The data file the value came
  from is not the base; the resolver returns a value, not a
  provenance-tagged definition, so that base could not be known.
  `Name@this` in a returned definition is rejected, mirroring
  `load_data`'s `ThisImportRejected`.
- Q3 (bindings), Q4/Q15 (no slots in kind definitions, the baseline, global
  catalogs, or the DMLS extension layer), Q5/Q17/Q18/Q21 (approval), Q6/Q7
  (pending triggers, re-resolution after writes), Q14 (only the effective
  schema's slots resolve), and Q19 (cross-command `stat` re-check) apply
  unchanged.

## Open questions

- Should a whole-type slot be allowed inside a trigger payload, or only in a
  document's `$schema` and schema files, given that its keys are unknown
  until resolution and trigger layering is whole-property?
- What does `md schema validate <schema file>` report for a pending property
  whose type is unknown: only the slot count, or also "pending type" per
  property?
- Does Claudine's CLI completion need a distinct "type pending" message, or
  does the existing unresolved-template report cover it?

## Out of scope

- Slots anywhere a value-position slot is refused today.
- Any change to the approval model, grant store, or cache.
