---
kind: topic
area: schema
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
        
>       **Note:** the external URI referencing is planned but currently not implemented yet!

We also carry forward the idea from JSON Schema of "types" and "constraints" being used to define a schema and in the next two sections we'll discuss both.

## Authoring Simplified Schemas

In this section we will get you up to speed on how to _define_ a schema.

### The Primitives

#### Types

A **type** is the foundation primitive for defining a schema. Each property in a schema must be assigned to a **type**. To take some of the mystery out of it a type, types available include:

- `string`, `number`, `boolean`, `null`, `object`

These _types_ will feel familiar to most but the full list of types goes beyond this, capable but basic, starting point to include some _types_ that some JSON Schema purists will swear are actually "constraints" _not types_ but don't listen to them (they're types in `SimplifiedSchema`):

- `email`, `file`, etc.

Then there the more esoteric or "meta" types like:

- `schema` and `expression`

The full list of _types_ can be found at: [`SimplifiedSchema` Types](./schemas/schema-types.md) but for now here's a simple example of using _types_ to define a schema:

```yaml
$schema:
    name: string
    age: number
    registered: boolean
```

> **Note:** it's important to understand that by default, a type represents an _optional type_. That means that a property defined as `string` must be a string when defined but it is allowed to be undefined (aka, `null` in YAML). Another way to describe this is that types defined in a schema which are not explicitly declared as `required` (more on that in a moment) are represented in the type system as union of `{type} | null`.

#### Constraints

A **constraint** is the a way to further _constrain_ a base type. There are some constraints that are available to _all_ types and others which can only be used with a subset of types:

- `required` -  can be used for any property and indicates that the property is a required property (aka, can not have a `null` value)
- `length` - can be used with _some_ types like `string` or any array type but would not be allowed with a type like `boolean`

For a full list of the _constraints_ available (and what _types_ can use them), you can read: [`SimplifiedSchema` Constraints](./schema-constraints.md) but here's an update of our example that takes advantage of constraints:

```yaml
$schema:
    name: string(required)
    age: number(min(1),max(150))
    registered: boolean(required)
```

#### Defaults, optional values, and runtime presence

Properties are optional unless their definition includes `required`. An
unbound optional property therefore remains nullable at a use site, including
when its definition contains `default(...)`: schema defaults are metadata and
are not applied as runtime values. Only `required` or a concrete non-null
frontmatter/caller binding establishes that a property is non-null.

This distinction matters when an expression supplies a directive target. DMLS
warns about an unguarded whole-value target such as `::file {{log}}` when
`log` is optional. Bind `log`, declare it `file(required)`, or guard the use
with `::block when="file_exists(log)"`.

#### Descriptors

Schema's -- _by their very nature_ -- describe a data structure but by allowing a schema to describe itself in prose as well as it's innate rule based structure, it can add a tremendous amount of clarity to schemas. This clarity is not only available as documentation but can also be picked up by language servers too.

`SimplifiedSchema` provides an ergonomic `->` operator that was designed to take the work out of documenting and hopefully encourage people to document more. Here's an example of how you could add prose annotations to your schema:

```yaml
$schema:
    name: string(required) -> An employees name
    age: number(min(1),max(150)) -> the employee's age, remember to check legal requirements before hiring 1 year olds
    registered: boolean(required) -> Flag indicating whether this person is presently employed
```

### External Schemas

> **Implementation status:** Some of the schema discovery and trigger behavior described below is not implemented yet. See [Schema Activation: Implementation Status](./schema-activation.md#implementation-status) for the current limitations.

In our examples so far we've used the _inline_ form of defining a schema. The inline form _can_ be very useful but when you're looking for better reuse the external schema is usually the better approach:

```yaml
$schema: ./food.yaml
```

In the example above:

- we replaced the key/value inline style of schema definition with a _reference_ to an external file
- the file being referenced must always be a YAML file, and
- the format of an external schema file might look something like this:
    
  ```yaml
  kind: schema
  $schema: 
      name: string(required)
      category: enum(good food, yuck, bad before expiry)
  ```

In a moment of "recursive truth" the external schema definition is defined as a schema:

::code ^darkmatter/docs/schemas/schema-definition.yaml

A schema document has two distinct payloads:

- `$schema` is the exported schema applied to a consuming document.
- `types` holds reusable named type definitions. Helper names do not become properties of the consuming document merely because they are declared here.

For example, a schema can export document properties while keeping its helper types separate:

```yaml
kind: schema
$schema:
    recipient: address
types:
    address:
        street: string(required)
        city: string(required)
```

The `address` type comes from the `types` section in the same file. You can also write `address@this` if you prefer to make that explicit; both forms mean the same thing. A reference to another file still needs its source, such as `address@./address-types.yaml`.

A named type also brings its constraints with it. If you define `code: string(required)` under `types`, using `code` elsewhere keeps it required; you don't need to write `code(required)` again. The same rule applies to `@this` and references to another file. See [Inheriting Constraints](./local-type-references.md#inheriting-constraints) for examples and the current implementation limitation.

Built-in names such as `string` keep their normal meaning. If you give a local type the same name as a built-in, use `string@this` to select your definition. A misspelled or missing local type is an error, not a new type created automatically.

> **Implementation status:** Bare local type names are not supported by the current parser yet. Until that support lands, use the explicit `@this` form. See [Local Type References](./local-type-references.md) for resolution and constraint details.

A types-only library omits `$schema`:

```yaml
kind: schema
types:
    address:
        street: string(required)
        city: string(required)
```

Other schemas can import `address` by name, for example with `address@./address-types.yaml`. A whole-file schema reference to this types-only file fails with a structured **no exported schema** error. It does not turn the helper names into document properties. Discovering or importing a type library does not automatically activate a document schema or lifecycle binding scope.

To make an exported schema apply automatically, place its definition in one of the accepted discovery directories:

- `{root}/schemas`
- in a monorepo:
    - `{package-area-root}/schemas`
    - `{package-root}/schemas`

- when the environment variable SCHEMA_DIR is set then it will be looked in as well

Note that `{root}` is the root of the repo you currently operating in if you're in a repo, if you're NOT in a repo then `{root}` is the current working directory (or the root folder in your editor in the case of DMLS).

Recognized standalone schemas with an exported `$schema` augment the Darkmatter baseline within their discovery scope. Automatically discovered package-area and package schemas do not apply to sibling scopes. `SCHEMA_DIR` adds explicitly selected definitions with workspace-wide applicability; it does not replace normal discovery. Types-only libraries remain available for imports without automatically applying a schema. Arbitrary YAML files are not schema inputs.

Direct, explicitly supplied always-on schemas remain supported independently of automatic discovery.

### Schema Triggers

Unlike an exported standalone schema, a `schema-trigger` document applies its schema only when its matching conditions pass. The canonical kind is `schema-trigger`; the earlier `schema-target` wording is not a second document kind.

Schema triggers let you choose a schema based on the document and its surroundings. Conditions can check:

- Frontmatter properties on the actively edited document
- File existence or absence
- File contains text
- Binary found or absent from the hosts execution path
- Time of day (local)
- Time of day (UTC)
- Host timezone
- Host OS
- Repository membership or absence

Conditions belong under `match`. Top-level conditions combine with **AND**; conditions inside a `group` combine with **OR**. An `expression` uses normal Darkmatter expression syntax to check document values:

```yaml
kind: schema-trigger
description: defines 'foo', 'bar', and 'baz' when one of them is present in the document
match:
    - expression: foo || bar || baz
$schema: 
    foo: string -> my favorite fake thing
    bar: string -> not bad, but no foo
    baz: string -> I wish we could leave him at home
```

Just like a normal schema definition a schema trigger will be _active_ when it's located in one of the directories mentioned in the last section. However, in this case _active_ means that the matching is active. When there is a match then the schema will be merged into the base schema.

#### Passive Activation Expressions

Choosing a schema should only answer a question: does this schema apply to this document? It should never run a command or start a runtime action. We call this _passive activation_.

For conditions about the document itself, use an `expression`. For example, `kind == "task"` checks whether the document's `kind` property is `task`. You can combine comparisons with `&&` and `||`, and use functions that work on the values you give them.

For conditions about files or the host computer, use a dedicated condition such as `file_exists` or `can_execute`. Give these conditions a literal path or program name rather than an expression that calculates one. Checking whether a program is available does **not** run it, and an expression cannot read a file or fetch a URL on its own.

This distinction also affects errors. If a file is missing, a file-existence condition simply does not match. If Darkmatter cannot check the file because access was denied, it reports the problem rather than pretending the file is missing. Likewise, a function that is not allowed in a trigger remains an error even if you put it after `false &&`.

For how Darkmatter gathers facts, evaluates conditions, and reports failures, see [How Schema Activation Is Evaluated](./schema-activation.md). Those details are shared by the CLI and DMLS; you do not need different trigger rules for each.

Time-based conditions can change which schema applies even when you haven't edited the document. DMLS checks again when a relevant time window starts or ends, and after sleep or clock changes. CLI commands check once each time you run them. See [Refreshing Time-Based Conditions](./schema-activation.md#refreshing-time-based-conditions) for the scheduling policy.

#### File predicate references

File predicates accept ordinary `biscuit-file` file-reference strings. An explicit-relative string resolves only beside the declaring schema:

```yaml
file_exists: "./config.yaml"
```

An optional explicit base selector distinguishes a consuming-workspace check:

```yaml
file_exists:
    base: workspace
    path: "./.example/config.yaml"
```

The explicit-base form requires an explicit-relative `path`, preventing a fallback search through other roots. Both forms use `FileReference` and captured resolution contexts, preserving typed failures. Schema imports retain their source origin. Existing sigils retain their meaning: `&` selects a repository root; `@` performs magic-path search and is not a workspace-root shorthand.

`base: workspace` selects the consuming document's repository/worktree root, even when the editor opened only a repository subdirectory. Outside a repository, it selects the nearest containing editor workspace folder or the captured CLI working directory. Nested repositories and linked worktrees use their own roots. Repository-discovery errors remain errors, not evidence that the document is outside a repository.

The consuming root is independent of schema-source location, including external `SCHEMA_DIR` sources. It does not expand the applicability of automatically discovered nested schemas. Additional base selectors remain under design review; these examples describe the intended contract, not current support.

For the matching model and its implementation status, read [Schema Triggers in Darkmatter](./schema-targeting.md).

### Schemas for Global Variables

Every global variable has a schema, including `doc`, `ctx`, `current`, `env`,
`err`, and `tracking`. Darkmatter's built-in definitions start in
[`darkmatter.yaml`](../../../schemas/darkmatter.yaml), which imports reusable
named types from its `partials` directory. `ctx` and `current` share the same
context type, so both expose properties such as `cwd` directly.

Frontmatter is the value of `doc`. Most expressions can omit that namespace:
`title` and `doc.title` refer to the same property. This shorthand does not change
how its schema is defined. A frontmatter property named `ctx`, accessed as
`doc.ctx`, is separate from the `ctx` global.

The schema describes what a value can contain. The application supplies the value
and determines when it is available. Knowing the fields of `err`, for example,
does not make an error available during initialization. Editor assistance uses
the schema and availability declarations without running value providers.

The layering rules below apply to the properties of `doc`. Authored document
schemas cannot redefine the other globals. The registered built-in global schema
is not automatically applied as a collection of frontmatter properties.

### Schema Layering

Schemas are so great we often end with too many of them:

- we _always_ get a base schema layer from Darkmatter
    - if you're using a library like Claudine (which is a big consumer of Darkmatter) then it will inject it's own schemas into the mix

- every document can define a schema in the `$schema` frontmatter property
- then we've just found out that `schema-definitions` and `schema-trigger`'s are a thing

That can be a lot of schemas all piled up on top of one another. What happens when two or more of these schema's disagree on what type the property `foo` is? Well fortunately we live in a law based society ... so we follow the rules.

The agreed precedence, from highest to lowest, is:

| Precedence | Source                                                                    |
|------------|---------------------------------------------------------------------------|
| Highest    | The consuming document's `$schema`                                        |
|            | Schemas activated by matching `schema-trigger` definitions                |
|            | Always-on schema definitions, including discovered `kind: schema` exports |
| Lowest     | The built-in Darkmatter base schema                                       |

Conflicts are resolved for each frontmatter property, not by rejecting either schema as a whole. If two schemas both declare `foo`, the higher-precedence definition of `foo` wins.

> **Note:** When there is a property which has a property conflict and two or more schemas are at the _same level_ of the precedence hierarchy the precedence hierarchy can't dictate who wins but at this level -- _one which shouldn't happen often or ever if you're being careful about your schema design_ -- an author will have no semantic sense as to which one is more important so the key design goal is simply to be consistent:
> 
> - Darkmatter library owns the resolution algorithm, DMLS and others resolve a layered schema via the Darkmatter library
> - the Darkmatter library uses precedence where possible but otherwise has a deterministic way to break conflicts at the same level of the precedence hierarchy

#### Best practices

Keep the active schema stack small. A typical Claudine document should need only the built-in Darkmatter schema plus the Claudine schema. Use `schema-trigger` definitions for specialized schemas so they participate only when document and environment conditions indicate that they apply. Avoid loading a large collection of unrelated always-on schemas and relying on precedence to sort out their differences.

### Modeling More Advanced Schemas

We've now seen how to define some basic schemas and been introduced to the basic building blocks of the grammar (type, constraint, descriptor) now we'll discuss how to model more advanced schemas:

- Enumerations (and Suggestions)
- Narrow Dictionary Definitions
- Arrays
- Union Types

### Enumerations

Being able to specify an _enumeration_ is considered critical infrastructure for any schema grammar and `SimplifiedSchema` makes it easy with examples like this:

```yaml
$schema:
    color: enum(red,blue,gree)
```

- the elements which make up the enumeration are comma delimited
- unless you quote the different options, the surrounding whitespace will be stripped:
    - `red,blue,green` and `red, blue, green` result in the same "red", "blue", and "green" choices

There is a _variant_ of an enumeration which we will call a "suggestion" which provides "guidance" but not rigid constraints like it's peer. This is far less commonly found in a schema grammar but as the exploding popularity of this type of schema in Typescript's type system demonstrates it's usefulness. Let's look at an example of this before we dive into how it works:

```yaml

```

#### Wide versus Nested Dictionaries

### Union Types

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

### External Grammar Files

There are two primary types of grammar files which are distinguished by their `kind` property:

- Schema Definition Files (`kind: schema`)
    
    An optional `$schema` exports the document schema. Optional `types` contains reusable named helpers. A types-only library has no exported document schema and does not automatically apply to discovered documents.
    
    This file type must follow the following schema definition:
    
  ```yaml
  kind: "const(schema,required)"
  $schema: "schema -> optional exported document schema; absent in a types-only library"
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

An applying standalone schema or conditional trigger supplies `$schema`. A types-only `kind: schema` library omits it. The trigger predicate catalog and complete envelope validation rules are still being finalized; these sketches do not establish additional required fields.

#### Schema Definition Files

#### Trigger Files

## CLI Grammar Support

### Validation

### Discovery

## Language Server (DMLS) Grammar Support
