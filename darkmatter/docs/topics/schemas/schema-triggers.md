---
kind: canonical-documentation
---

# Schemas Applied by Triggers

A schema trigger applies a frontmatter schema to matching documents without
requiring each document to name it with `$schema`. Put the trigger in one of
the document's [schema roots](./definition.md#schema-roots). Its `match`
conditions select documents; its `$schema` supplies the types they must follow.
For condition syntax and path matching, start with
[Schema Triggers in Darkmatter](./schema-targeting.md).

## Invalid discovered triggers (planned)

**Planned:** an invalid automatically discovered trigger will be ignored
with a warning identifying its source and the reason. For example, a
`schemas/memory.yaml` with unsupported envelope keys or an empty `match:`
will produce a warning on CLI stderr while composition continues using
the other valid schemas. This also covers broken trigger payloads and
imports. A skipped trigger contributes no partial schema and continues to
shadow same-named files in later schema roots.

Warnings will be reported once per trigger per invocation. Explicit
document `$schema` load failures and validation failures under valid
schemas will remain errors. Today, invalid discovered triggers still
abort discovery; the behavior described here is not yet implemented.

## Applying a recursive type

A trigger payload can declare the same reusable and recursive types as a
document schema. For example, save this as `schemas/outline.yaml`:

```yaml
kind: schema-trigger
match:
    kind: literal(outline; required)
$schema:
    node: "{ title: string(required), children: node[](required) }"
    outline: node(required)
```

The trigger selects this document and validates every nested child:

```yaml
---
kind: outline
outline:
    title: Overview
    children:
        - title: Details
          children: []
---
```

Each node needs both `title` and `children`. An empty child list ends a
branch. A missing or invalid property deeper in the tree is reported at that
document value's path and source location. Completion and hover follow the
same type as the cursor moves into children.

The recursive type must eventually check a child value: an object property,
dictionary value, array item, or tuple slot. A cycle made only of aliases or
union alternatives is rejected. See
[Recursive Named Types](./definition.md#recursive-named-types) for the full
rule, including cycles across files and definition-versus-use-site behavior.

## Keeping document layers separate

The effective schema combines the baseline, matching triggers in their
established order, and the document's own schema. Later declarations win for
the same property. See [Merge Semantics](./definition.md#merge-semantics-baseline--document)
for the complete precedence rules.

A type belongs to the source that declares it. A `node` in this trigger and a
`node` in the document's own `$schema` remain different types. Recursive
definitions are stored once in JSON Schema's `$defs`, with references from
their uses. When layers bring different definitions with the same generated
key, Darkmatter renames the joining key and its references together. Literal
data containing `$ref` or `$defs`, such as a `default`, is not rewritten.

A raw JSON Schema trigger payload must remain a simple object schema. It may
have a root `$defs` with checked local references from its properties, but it
cannot use a root `$ref` or root union. The
[baseline restrictions](./definition.md#json-schema-baseline-restrictions)
also apply to trigger payloads. A malformed discovered payload remains an
error even when its trigger does not match the current document.

## Validation and runtime phases

Trigger matching and validation are passive: expressions and shell commands
are not executed, and document values are not changed. Composition can perform
its separately authorized normalization steps. Recursive children that are
absent are not created merely to traverse their type.

Launch and Completion honor each property's own `required`, `generated`, and
`eager` markers below recursive references. A shared definition can serve both
an eager and a deferred property without changing the other use. See
[Validation Phases](../../inline/schema-validation.md#validation-phases).

For slow validation, inspect [CLI stage timings](../../inline/schema-validation.md#stage-timings--vv)
or [DMLS timing logs](../../lsp/indexing.md#schema-validation-timing-lines).
