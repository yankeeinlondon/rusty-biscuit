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

## What is prepared ahead and what runs per validation

The schemas compiled into Darkmatter are prepared before any process runs:
the baseline, the global catalog, and the shipped kind definitions are read
from Darkmatter's [shipped artifact](./definition.md#the-shipped-artifact),
and a host such as Claudine reads its [prepared trigger
bundle](./definition.md#prepared-bundles) from its own. A validation still
does the work only the document can decide: it finds and matches triggers,
resolves the document's own `$schema` and any trigger read from disk, merges
the layers, and compiles the merged schema into a validator.

That validator is cached for the rest of the process under the identity of
the layers it was merged from, so the next document with the same baseline,
triggers, and `$schema` reuses it instead of compiling again. For example,
`md schema validate notes/*.md` over fifty notes that all match one trigger
and declare no `$schema` compiles one validator, not fifty. See
[Validator Cache](./definition.md#validator-cache) for what makes two
identities differ.

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

Validating the schema file itself resolves it too, through the same grant
gate: every slot in the file runs (template constraints and template types,
including slots in named types the file's `$schema` does not use), and the
resolved schema is then checked. There is no mode that passes a template
without running it, because a template's slots decide whether it is valid:

```sh
md schema validate schemas/provider.yaml
# resolves every slot in provider.yaml, then checks the resolved schema
```

A template type whose data returns something that is not a type definition,
a refused or unapproved read, or a slot that fails makes the check
unavailable: exit `1`, `error: slots`. See
[`md schema validate`](../../cli/schema.md).

Which commands resolve, and with which grants, is in
[When a slot is evaluated](./schema-slots.md#when-a-slot-is-evaluated); the
diagnostic codes are in [Failures](./schema-slots.md#failures).
