# Schema Triggers in Darkmatter

A `schema-trigger` file defines a schema that applies only when its matching
conditions pass. Conditions are set in the `match` property. The filename of
this topic retains the earlier terminology; the document kind is
`schema-trigger`.

This topic describes the intended contract, not current implementation. The
agreed Boolean model is AND between top-level conditions and OR within a group.
The current implementation uses OR for a top-level list; existing definitions
require explicit migration to preserve their meaning. The full predicate family
listed in [Authoring Schemas](./authoring-schemas.md#schema-triggers) is in scope;
its exact syntax and observation/refresh contracts remain under design review.

Trigger matching is passive: it checks whether a schema applies without running
commands or starting runtime actions. Use expressions for document values and
dedicated conditions for files or host information. Read
[Passive Activation Expressions](./authoring-schemas.md#passive-activation-expressions)
for the authoring rules, or
[How Schema Activation Is Evaluated](./schema-activation.md) for the shared
evaluation and failure-handling details.

An activated trigger's schema takes precedence over always-on schema
definitions and the Darkmatter base schema, but the document's own `$schema`
takes precedence over the trigger. For each conflicting property, the winning
definition supplies both its type and its description. See
[Schema Layering](./authoring-schemas.md#schema-layering) for the shared ordering
and guidance on keeping active schema stacks small.

The `match` property is composed of 1:M _matchers_ which are either:

- a match expression, or 
- a _group_ of match expressions

_Grouped_ match expressions are evaluated with a logical OR operation but all others are joined with an AND operation.

Let's look at this through an example:

```yaml
kind: schema-trigger
name: example
description: an example
$schema:
    name: string
    quantity: number
match:
    - in_repo
    - expression: "is_number(quantity) && quantity > 5"
    - group:
        - expression: "name == 'Bob'"
        - can_execute: "netscape"
        - timezone: "America/Los_Angeles"
        - file_exists: "./path/to/file/found-it.md"
```

In the example:

- we have 6 match expressions to evaluate, 4 of which are in a group where only one of the members has to pass for it's leg to be valid
- looked at from the top level array's standpoint we have 3 match expressions which ALL must evaluate to `true` for the schema to be activated.


The draft schema for a `schema-trigger` is defined in `SimplifiedSchema` grammar
in [schema-target.yaml](../../schemas/schema-target.yaml). Its filename also
retains the earlier terminology; its unfinished predicate grammar is not an
implemented contract.

For unconditional exported schemas and importable types-only libraries, see
[External Schemas](./authoring-schemas.md#external-schemas).
