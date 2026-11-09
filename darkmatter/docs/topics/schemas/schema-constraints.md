---
kind: topic
area: schema
---

# Schema Constraints

A **constraint** narrows what a property's type accepts: `required`,
`enum(...)`, `min(...)`, `pattern(...)`, and the others written inside a
type's parentheses, as in `age: number(required; min(0))`. This page covers
how Darkmatter treats constraints whose values come from data. The complete
list of constraints, and which types accept each, is in
[`SimplifiedSchema` Constraints](../../../../simplified-schema/docs/topics/language/types.md#universal-constraints).

## Constraints that take a slot

Most constraints are written as literal values, but a few can take their value
from a `{{ … }}` [slot](./schema-slots.md), an expression Darkmatter evaluates
when the schema is resolved:

```yaml
kind: schema
$schema:
    os: enum({{ load_data("./os.yaml", "string[]") }}; required)
```

Only these positions accept a slot:

- an element of an `enum(...)` or `suggest(...)` list;
- the whole argument of `literal(...)`, `default(...)`, `min(...)`,
  `max(...)`, or `pattern(...)`.

Each position takes one shape of value (a list slot an array, `min` a
number, and so on), and a pattern from a slot is capped. The shapes and the
cap are listed in
[What a Slot's Value Must Be](../../../../simplified-schema/docs/topics/language/slots.md#what-a-slots-value-must-be),
and authoring guidance is in
[Make an Enum from a Data File](./authoring-schemas.md#make-an-enum-from-a-data-file).
Every other constraint, such as `required` or `not-empty`, is written
literally, and so is a `null` default.

A list slot can read an optional file with `try`, as in
`enum(macOS, {{ try(load_data("./extra.yaml", "string[]"), []) }})`: a
missing file adds no candidates, but the final list must still not be empty.
What `try` recovers from inside a slot, and what it never does, is in
[An optional data file](./schema-slots.md#an-optional-data-file).
