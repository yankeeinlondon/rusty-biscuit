---
kind: overview
type: schemas
---
# Schema Support in Claudine

Claudine inherits and extends the powerful schema system that [Darkmatter](^darkmatter/docs/topics/schemas/index.md) provides. This schema system allows you to get rich LSP support from [DMLS](^darkmatter/docs/topics/dmls.md) (aka, Darkmatter's LSP for Markdown and YAML).

This document will provide you a breadth wide overview of schemas but there is a lot of powerful features under the hood. If you want to dig into these details use the links [Schema Details](#schema-reference-docs) section.

## Defining Schemas

You can think of schema's as being defined at three levels:

1. Local to where you are working
2. Across the Repo
3. Base Darkmatter and Claudine schemas

### Local Schemas

You can use the `$schema` property of any Markdown document's Frontmatter to impose a schema constraint on the document:

```yaml
$schema:
    name: string(required) -> the name of the product
    description: string -> a description of the product
```

If you want to reuse a schema across several files you can create the schema in a YAML file:

```yaml
kind: schema
$schema:
    name: string(required) -> the name of the product
    description: string -> a description of the product
```

and then point any Markdown documents you want to be bound to that schema by adding the following to the frontmatter:

```yaml
$schema: ./path/to/schema.md
```

### Repo Wide Schemas

## Schema Reference Docs

- [Authoring Schemas](^darkmatter/docs/topics/schemas/authoring-schemas.md)
    - [Schema Types](^darkmatter/docs/topics/schemas/schema-types.md)
    - [Schema Constraints](^darkmatter/docs/topics/schemas)
    - [Schema Triggering](^darkmatter/docs/topics/schemas/)
- [CLI Validation]()
- [CLI Observation]()
- [DMLS Support for Schemas]()
- Schemas
    - Darkmatter ([all global namespaces](^darkmatter/schemas/darkmatter.yaml))
        - [Frontmatter Base Schema (`doc` namespace)](^darkmatter/schemas/partials/doc.yaml)
            - [Style Schema (for the `style` property in `doc` namespace)](^darkmatter/schemas/partials/tracking.yaml)
        - [Context Variables Schema (`ctx` and `current` namespaces)](^darkmatter/schemas/partials/context-variables.yaml)
        - [Error Schema (`err` namespace)](^darkmatter/schemas/partials/error.yaml)
        - [Tracking Schema (`tracking` namespace)](^darkmatter/schemas/partials/tracking.yaml)
    - Claudine
        -
