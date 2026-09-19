---
kind: index
area: schema
---
# Simplified Schemas with `SimplifiedSchema`

Darkmatter introduces an exciting new grammar called `SimplifiedSchema` which provides an ergonomic yet powerful way to provide schema support to your Markdown and YAML files. Now if you're not the type to get excited by a new standard let me tell you where you're wrong:

- `SimplifiedSchema` provides a superset of JSON Schema functionality (and be easily converted to it when you need)
- but unlike JSON Schema, `SimplifiedSchema` is ergonomic and easy to write and use
- the Darkmatter CLI provides full validation support of this grammar
- the Darkmatter Language Server (DMLS) provides rich visual support for this grammar

## Getting Started

A `SimplifiedSchema` is most likely to be found in the `$schema` property of Markdown files and looks something like this:


```yaml
$schema:
    name: |-
        string(required) -> the name of the person you're needlessly 
                            bothering about SimplifiedSchema
```

I'm sure you're ready to dig into the magic we call `SimplifiedSchema` so choose from the links below to get more details:


- **Using SimplifiedSchema:**

    - [Authoring SimplifiedSchema](./authoring-schemas.md)
        - describes the grammar primitives (types, constraints, )
        - shows how to _define_ useful schemas in your project in Markdown and YAML
        - describes what a `schema-trigger` is and how to leverage them to activate schemas dynamically
    - [Schema Validation](./schema-validation.md)
        - shows how Darkmatter provides _schema validation_
        - describes how this can be utilized  
            - from the terminal using the CLI 
            - as a library author calling into the Darkmatter library
    - [Schema Discovery](./schema-discovery.md)
        - extract _observed_ schema structure from a schemaless file
    - [Visual Support of Schemas from DMLS](dmls-schema-support.md)
        - DMLS provides a full Markdown/CommonMark language server that extends Markdown's grammar to support schemas
    - [Understanding the Schema's Role in Composition](./schema-in-composition.md)
        - this explores the interaction schemas have with _composable_ documents
        - it touches on eager and late binding variables and gives examples of the `eager` constraint in `SimplifiedSchema`

- **Standards:**

    - [Schema Standards and SimplifiedSchema's Position](./schema-standards.md)
    - describes how SimplifiedSchema tries to stay inline with any fledgling informal standards that exist
    - [SimplifiedSchema to JSON Schema (_and back_)](./json-schema.md)
        - JSON Schema is the "go to" schema for many projects and yet it's an ugly and non-ergonomic grammar
        - `SimplifiedSchema` attempts to provide ergonomics while also allowing simple movement between `SimplifiedSchema` and JSON Schemas.
    -
- **Related Topics**


- **Consuming Libraries**

    - [Claudine](@claudine/README.md) is the biggest consumer of Darkmatter _in general_ but also in terms of it's schema functionality
