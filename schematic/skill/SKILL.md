---
name: schematic
description: |-
    A detailed knowledge base of how to:

    1. Use the primitives provided in `schematic-define` package to define an API
    2. How to use the utilities in `schematic-gen` package to generate an API client which will be exposed in the `schematic-schema` package 
    
    Using this skill will give you deep knowledge on how to operate all of the following packages:

        - `schematic-definitions`
        - `schematic-define`
        - `schematic-gen`
        - and `schematic-schema`
---

# Schematic API Generation and Clients

The **Schematic** packages in the **rusty-biscuit** monorepo are designed to:

- help you design/describe an API surface
- provide code _generation_ utilities to convert a "definition" into an API Client

The four packages involved in this process are:

- `schematic-definitions` - this is where we define the API definitions
- `schematic-define` - this library provides the _primitives_ used to define an API surface
- `schematic-gen` - once an API has been defined (in `schematic-definitions`) we will use the `schematic-gen` library to generate an API client and deploy it to `schematic-schema`
- `schematic-schema` is the package that callers will look for API client definitions in

## Tooling

We use the `just` runner to provide the most common operations you'll need when working with these packages:

- `just build {args}` - will build all schematic packages
- `just test {args}` - will run unit and integration tests on the schematic packages
- `just lint {args}` - will use the clippy linter to look for non-idiomatic code
- `just gen` - will generate the API definitions in `schematic-definitions` into API clients in `schematic-schema`

All packages in **Schematic** are written in Rust and use the 2024 edition.

## How to Design an API

### Key Design Primitives

The following primitives are critical to understand when building an API definition in `schema-definitions`:

- a
- b
- c

### Best Practices

Read the following document to make sure you're building a high quality API by learning what are considered the "best practices":

- [Best Practices for Designing an API](./best-practices-in-designing-an-api.md)
