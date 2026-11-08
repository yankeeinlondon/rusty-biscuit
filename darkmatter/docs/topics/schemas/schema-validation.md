---
kind: topic
area: schema
---

# Schema Validation

Schema validation checks a document's frontmatter, or a whole YAML document,
against the schema that applies to it: the baseline, the document's own
`$schema`, and any schema trigger that matches it. `md schema validate`
reports the result on the command line, and the `DarkmatterSchemas` library
API does the same for a program. How the effective schema is assembled, the
command's flags, output formats, and exit codes are described in
[Schema Definition](./definition.md#cli-md-schema-validate).

## Validating against a schema with slots

A schema whose values come from `{{ … }}` [slots](./schema-slots.md) is a
**template**, and a template is never what a document is validated against.
Validation first resolves it, evaluating each slot under the read approvals,
and checks the document against the **resolved** schema:

```sh
md schema validate provider.md
# resolves the slots of provider.md's schema, then checks provider.md
```

When the template cannot be resolved (a read nobody approved, a denied read,
a slot that failed), the document gets the slot's `dm.schema.slot_*`
diagnostics instead of a verdict, and `md schema validate` exits `1` with the
file's check unavailable (`error: slots`). A schema missing its loaded values
never makes a document look valid.

To check a template itself without running any slot, validate the schema
file:

```sh
md schema validate schemas/provider.yaml
# checks the template's syntax and shape, reports its slot count, resolves nothing
```

Which commands resolve, and with which grants, is in
[When a slot is evaluated](./schema-slots.md#when-a-slot-is-evaluated); the
diagnostic codes are in [Failures](./schema-slots.md#failures).
