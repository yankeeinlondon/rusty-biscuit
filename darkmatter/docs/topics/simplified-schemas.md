---
kind: documentation
tags:
    - schema
    - schema-triggers
    - schema-validation
    - schema-detection
    - types
    - constraints
---
# Simplified Schemas

## Overview

Darkmatter introduces an exciting new grammar called **Simplified Schemas** which provides an ergonomic yet powerful way to provide schema support to your Markdown and YAML files. In addition:

- the Darkmatter CLI provides not only provides **schema validation** but also **schema discovery**
- the Darkmatter Language Server (DMLS) fully understands and visually reinforces the schema rules defined by Simplified Schema

Now if that weren't already great news, Simplified Schema has been designed with a very clear JSON Schema mapping so that any schema defined in Simplified Schema can be converted to JSON schema easily.

Some of you may be thinking, well schema support in Markdown and YAML sounds great but why reinvent the wheel? Just use JSON Schema. Well, two things:

1. You _can_ use JSON Schema if you'd like to (more on that later)
2. Have you used JSON Schema before? Yuck. It's usefulness is that it's a standard and that many libraries support it but it is the _opposite_ of ergonomic.

## Grammar

### Trying to be Standards Based

We are trying to provide interested parties a handy way to introduce schemas into both your YAML and Markdown documents. For those of you deep in the ways of YAML, you may know that there have been informal attempts to provide schema support in YAML by giving the `$schema` root property of a document special meaning and pointing it toward a URI that exposes a JSON Schema. Sadly this remains an informal approach that has never been ratified or formalized.

With Markdown's Frontmatter, you will most typically see Frontmatter represented in YAML syntax though the Markdown standard actually also allows JSON and TOML frontmatter too (quite rarely used). Regardless, there are no known attempts -- _known by this author anyway_ -- to provide schema support to Markdown. Even semi-formal proposal's like we have in the case of YAML don't exist though arguably if YAML ever formalized a schema standard it probably could be brute forced into Markdown too.

In any case, our goal with **Simplified Schema** is to be as "standards based" as is possible in this landscape of half truths and informalities but build it in such a way that both the more accepted JSON Schema and the more ergonomic **Simplified Schema** can be used side-by-side with precisely the same referencing entry points:

- we support the idea of the `$schema` property being used to define a schema
- this can be done in either YAML or Frontmatter YAML
- The value of the `$schema` property can be:
    - an inline schema definition (Simplified Schema)
    - a local file reference to a YAML or JSON definition file (Simplified Schema _or_ JSON Schema)
    - a URI reference to an external schema file (Simplified Schema _or_ JSON Schema)

      > **Note:** the external URI referencing is planned but currently not implemented yet!

We also carry forward the idea from JSON Schema of "types" and "constraints" being used to define a schema and in the next two sections we'll discuss both.

### Types

A **type** is the base primitive for defining a property in a schema's type. It includes:

- things everyone will be familiar with like `string`, `number`, `boolean`, etc.
- but also includes some types that in JSON Schema might be viewed as a _constraint_ not a _type_ like `email`, `file`, etc.
- and then provides some handy "meta types" like `schema`

The full list of _types_ can be found at: [Simplified Schema Types](./simplified-schema-types.md).

### Constraints

A **constraint** is the a way to further _constrain_ a base type. There are some constraints that are available to _all_ types and others which can only be used with a subset of types:

- `required` -  can be used for any property and indicates that the property is a required property (aka, can not have a `null` value)
- `length` - can be used with _some_ types like `string` or any array type but would not be allowed with a type like `boolean`

For a full list of the _constraints_ available (and what _types_ can use them), you can read: [Simplified Schema Constraints](./simplified-schema-constraints.md).

### Descriptors

All schema properties can add a `-> {description}` at the end of their definition to provide a prose description of the property.

### Examples: using Inline Syntax

#### The Basics

In a Markdown file that is responsible for taking in a specification file and an optionally a tech design file and then composing a prompt for an LLM agent that looked like this:

```md
---
$schema:
    spec: file(required;eager)
    design: file
---
Your job is to review the specification file "{{spec}}".

::block when="design"
- there is also a technical design document to complement the spec found at: "{{design}}"
::end-block
```

- this example defines two **file** properties
    - `spec` and `design` are both **file** types
    - but only `spec` is considered a _required_ property
    - this means that `design` is valid when it's value is `null` (null and undefined are identical types) or when it's filepath
- we also specify that `spec` should be evaluated _eagerly_ (the default is _lazy_ evaluation)
- even though a filepath is a derivative of a string type it holds additional semantic meaning to define it as a **file** type
-


This meaning becomes even more pronounced when we add the `eager` constraint:

- by default all properties are `lazy` which means that the page can be _composed_ without


#### Enumerations and Union Types


#### Wide versus Nested Dictionaries





### External Grammar Files

There are two primary types of grammar files which are distinguished by their `kind` property:

- Schema Definition Files (`kind: schema`)

    This type of file defines types via the `$schema` property that can be referenced by as many Markdown (or YAML files as you wish). Unlike an inline schema definition, the external schema definition file has the benefit of being able to define _intermediate_ types in the optional `types` property.

    This file type must follow the following schema definition:

    ```yaml
    kind: "const(schema,required)"
    $schema: "schema(required)"
    description: "string -> a way to be more explicit about this schema's intended usage"
    types: "schema -> you may optionally define types in this area which then become accessible as _types_ in the `$schema` property."
    ```

- Schema Trigger Files (`kind: schema-trigger`)

    The schema _trigger_ file is used to establish a matching pattern that _when matched_ will trigger the schema definition on a given file.

    This file follows the following schema definition:

    ```yaml
    kind: "const(schema-trigger,required)"
    match: TODO
    $schema:
        - schema
        - file
    description: |-
        string -> allows you to describe the intent of this trigger in prose
    ```

Both types expect that you define the root-level `$schema` property

#### Schema Definition Files

#### Trigger Files


## CLI Grammar Support

### Validation

### Discovery

## Language Server (DMLS) Grammar Support
