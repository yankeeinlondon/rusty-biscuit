---
kind: canonical-documentation
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

The full list of _types_ can be found at: [`SimplifiedSchema` Types](../../../../simplified-schema/docs/topics/language/types.md#types) but for now here's a simple example of using _types_ to define a schema:

```yaml
$schema:
    name: string
    age: number
    registered: boolean
```

> **Note:** it's important to understand that by default, a type represents an _optional type_. That means that a property defined as `string` must be a string when defined but it is allowed to be undefined (aka, `null` in YAML). Another way to describe this is that types defined in a schema which are not explicitly declared as `required` (more on that in a moment) are represented in the type system as union of `{type} | null`.

#### Constraints

A **constraint** is the a way to further _constrain_ a base type. There are some constraints that are available to _all_ types and others which can only be used with a subset of types:

- `required` -  can be used for any property and indicates that the property is a required property (aka, can not have a `null` value); the exception is a type that itself admits `null` (`null`, `unknown`, or a union with such an arm), which stays optional (see [Null-Admitting Properties](../../../../simplified-schema/docs/topics/language/types.md#null-admitting-properties))
- `length` - can be used with _some_ types like `string` or any array type but would not be allowed with a type like `boolean`

For a full list of the _constraints_ available (and what _types_ can use them), you can read: [`SimplifiedSchema` Constraints](../../../../simplified-schema/docs/topics/language/types.md#universal-constraints) but here's an update of our example that takes advantage of constraints:

```yaml
$schema:
    name: string(required)
    age: number(min(1); max(150))
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
    age: number(min(1); max(150)) -> the employee's age, remember to check legal requirements before hiring 1 year olds
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
  $schema:
      name: string(required)
      category: enum(good food, yuck, bad before expiry)
  ```

In a moment of "recursive truth" the external schema definition is defined as a schema:

::code ^simplified-schema/schemas/schema.yaml

A schema file can export document properties and declare reusable types:

- **pure**: `$schema` holds the export, with an optional sibling `types` mapping
- **tagged**: `kind: schema` and `types`, with an optional `$schema` export

Either may also carry a `description` string and a `categories` list, and
accepts no other keys. When a file has both `$schema` and
`types`, only `$schema` defines document properties; `types` supplies local
names and named imports. Without separate export and type tables, every
top-level entry of the payload plays two roles. It is a property of the schema
when a document references the whole file (`$schema: ./address-types.yaml`),
and it is a **named type** that other definitions can reuse by name.

Inside the same file, refer to a named type by its bare name:

```yaml
kind: schema
types:
    address:
        street: string(required)
        city: string(required)
    letter: "{ recipient: address, sender: address }"
```

You can also write `address@this` if you prefer to make that explicit; both
forms mean the same thing. A type from another file needs its source:
`address@./address-types.yaml`.

Darkmatter's own reusable types are reached with the reserved source
`darkmatter:<stem>` instead of a path, so a schema in any repository can use
them without knowing where Darkmatter is installed:

```yaml
kind: schema
types:
    notice: communicate@darkmatter:lifecycle   # one say/info/warn/… action
```

See [Reserved Darkmatter References](../../../../simplified-schema/docs/topics/language/objects-and-named-types.md#reserved-darkmatter-references-namedarkmatterstem)
for the rules, and [Lifecycle Event Typing](./definition.md#lifecycle-event-typing)
for the lifecycle types Darkmatter ships.

To keep helper types out of document properties, put them in a sibling
`types` table or a separate file and import them by name. A
document that uses `recipient: address@./address-types.yaml` gains a
`recipient` property, not an `address` one.

Built-in names such as `string` keep their normal meaning. If you give a local
type the same name as a built-in, use `string@this` to select your definition.
A misspelled or missing type is an error at the reference, not a new type
created automatically. See [Local Type References](../../../../simplified-schema/docs/topics/language/local-type-references.md)
for where local names live and how they resolve.

#### Presence travels with the type

A named type brings **all** of its constraints with it, including `required`
and `generated`. If you define `code: string(required)`, every property that
uses `code`, `code@this`, or `code@./types.yaml` is required; you don't need to
write `code(required)` again. Use-site constraints add to the inherited ones
and never remove them. See
[Inheriting Constraints](../../../../simplified-schema/docs/topics/language/local-type-references.md#inheriting-constraints)
for how they combine.

Presence belongs to the value it is declared on. Requiring a field inside an
object type does not require the property that holds the object:

```yaml
# people.yaml
kind: schema
types:
    person: "{ name: string(required), email: email }"
```

A property declared as `author: person@./people.yaml` is optional. When
`author` is supplied, it must have a `name`.

There is no use-site syntax to make an inherited `required` optional again.
When you need both a required and an optional version of a type, declare the
optional one directly:

```yaml
# people.yaml
kind: schema
types:
    owner: "{ name: string(required), email: email }(required)"
    draft_owner: "{ name: string(required), email: email }"   # the optional variant
```

Do not define the variant as `draft_owner: owner`: an alias inherits
`required` again. A union with `null` does make it optional:
`draft_owner: [owner, null]` accepts `null`, an omitted value, or a valid
`owner` (see [Null-Admitting Properties](../../../../simplified-schema/docs/topics/language/types.md#null-admitting-properties)).

An inherited `generated` keeps its usual meaning. It does not create a value
provider. Checking an authored document, as `md schema validate` and DMLS do,
accepts a missing generated property even when it is required, because the host
supplies the value later. Runtime checks use two phases: launch requires an
inherited `required` value immediately only when it is also `eager`;
completion requires it after the work finishes. Supplied values are checked
at both phases. A type that admits `null` remains optional regardless of
these markers. See [Universal Constraints](../../../../simplified-schema/docs/topics/language/types.md#universal-constraints).

To make an exported schema apply automatically, place its definition in one of the five [schema roots](./definition.md#schema-roots), searched in this order:

1. the folder the `SCHEMAS_DIR` environment variable names, when it is set (the folder itself, which can be any folder you choose; no `schemas/` is appended), so a team or host override outranks every repository copy
2. in a monorepo, `{package-root}/schemas` of the document's package
3. in a monorepo, `{package-area-root}/schemas` of the document's package area
4. `{root}/schemas`
5. `~/schemas`

`{root}` is the document's file tree root: the root of its repository when it is in one, otherwise the tree it was opened in (often the document's own folder). `md` searches these folders only for a document in a repository; DMLS searches them for any document inside an open workspace folder. Package and package area are the document's, not those of the directory you launched from. `schemas/` folders anywhere else, such as `{package-root}/docs/schemas`, are not searched. An earlier folder shadows a file of the same name in every later one.

Today two kinds of file in these folders apply automatically: a `kind: schema-trigger` file to the Markdown documents it matches (see [Repository Trigger Schemas](./definition.md#repository-trigger-schemas)), and a `kind: yaml-schema` file to the YAML documents whose `kind` it claims (see [Defining a YAML Document Kind](#defining-a-yaml-document-kind)). Any other schema file there is used when a document names it with a bare `$schema`, such as `$schema: claudine.yaml`. The automatic application of an exported standalone schema described next is not implemented yet.

Recognized standalone schemas with an exported `$schema` augment the Darkmatter baseline within their discovery scope. Automatically discovered package-area and package schemas do not apply to sibling scopes. The `SCHEMAS_DIR` folder and `~/schemas` are in every document's roots, so their definitions can apply to documents in any repository; they add to the package, package-area, and `{root}` folders rather than replacing them. Types-only libraries remain available for imports without automatically applying a schema. Arbitrary YAML files are not schema inputs.

Direct, explicitly supplied always-on schemas remain supported independently of automatic discovery.

### Schema Triggers

Unlike an exported standalone schema, a `schema-trigger` document applies its schema only when its matching conditions pass. The canonical kind is `schema-trigger`; the earlier `schema-target` wording is not a second document kind.

Schema triggers let you choose a schema based on the document and its surroundings. Conditions can check:

- Frontmatter properties on the actively edited document
- The document's path (`$path`)

These conditions are **planned** and not yet part of the trigger grammar; a trigger that uses one is skipped as invalid:

- File existence or absence
- File contains text
- Binary found or absent from the hosts execution path
- Time of day (local)
- Time of day (UTC)
- Host timezone
- Host OS
- Repository membership or absence
- An `expression` over document values

Conditions belong under `match`. Top-level conditions combine with **AND**; conditions inside a `group` combine with **OR**. A trigger file holds only `kind`, `match`, and `$schema`:

```yaml
kind: schema-trigger
match:
    - group:
          - foo: string(required)
          - bar: string(required)
          - baz: string(required)
$schema:
    foo: string -> my favorite fake thing
    bar: string -> not bad, but no foo
    baz: string -> I wish we could leave him at home
```

This trigger applies when the document has at least one of `foo`, `bar`, or `baz`.

Just like a normal schema definition a schema trigger will be _active_ when it's located in one of the directories mentioned in the last section. However, in this case _active_ means that the matching is active. When there is a match then the schema will be merged into the base schema.

A trigger's schema can use named types, including [recursive ones](#recursive-types), like any other schema. Its stored definitions (recursive types, and [shared](../../../../simplified-schema/docs/topics/language/objects-and-named-types.md#shared-named-types) structured ones) are merged beside the document's own; a type named `item` in both stays two separate types.

#### Passive Activation Expressions

> **Planned.** The `expression`, file, and host conditions this section and
> the next describe are not yet part of the trigger grammar.

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

The consuming root is independent of schema-source location, including external `SCHEMAS_DIR` sources. It does not expand the applicability of automatically discovered nested schemas. Additional base selectors remain under design review; these examples describe the intended contract, not current support.

For the matching model and its implementation status, read [Schema Triggers in Darkmatter](./schema-targeting.md).

### Defining a YAML Document Kind

A schema trigger picks Markdown documents by their frontmatter. When you have
a family of **YAML files** that should all follow one contract (rosters,
service manifests, fixture files), write a `kind: yaml-schema` definition
instead. Each file then only has to say what it is, with `kind: roster`, and
the definition does the rest: no file carries its own copy of the schema, and
none can point at the wrong one.

Put this in your repository's `schemas/roster.yaml`:

```yaml
kind: yaml-schema
description: The platforms a team publishes to
types:
    platform:
        name: string(required)
$schema:
    kind: literal(roster; required) -> the document kind this file defines
    roster_version: literal(1; required)
    platforms: platform[](required)
    $constraints: { closed: true }
```

Every YAML document in the repository that says `kind: roster` is now
validated against that `$schema`:

```yaml
kind: roster
roster_version: 1
platforms:
    - name: slack
```

The rules for a definition:

- **`kind: yaml-schema` and a `$schema` are required.** `$schema` is the
  contract every document of the kind must meet. `description`, `categories`,
  and `types` are optional and mean what they mean in a `kind: schema` file;
  `types` entries are local names for `$schema`, not properties of a roster.
- **The `kind` property of `$schema` names the kinds you claim.** Declare it
  as `literal(roster; required)` or, to claim several,
  `enum(roster, team; required)`. It must be `required`: `literal(roster)` alone, or a plain
  `kind: string(required)`, claims nothing and the definition is rejected.
- **A root union claims every arm's kinds.** Each arm declares its own `kind`,
  and a document must satisfy one of the arms:

  ```yaml
  kind: yaml-schema
  $schema:
      - kind: literal(left; required)
        a: string
      - kind: enum(right, middle; required)
        b: string
  ```

- **Close the root.** `$constraints: { closed: true }` turns a misspelled key
  into an unknown-key problem instead of an ignored property. It also means a
  roster cannot carry a stray `$schema`: once a definition claims the kind, a
  root `$schema` is an ordinary property that the definition must declare.
- **A definition is never referenced.** Naming a `yaml-schema` file in a
  `$schema` or a `Name@file` import is an error. Put types that a definition
  and a Markdown schema share in a `kind: schema` file and import them from
  both (see [A `yaml-schema` Is Selected by `kind`, Never Referenced](../../../../simplified-schema/docs/topics/language/schema-documents.md#a-yaml-schema-is-selected-by-kind-never-referenced)).

Darkmatter's own definitions are worked examples of each form:

- [`yaml-schema.yaml`](../../../../simplified-schema/schemas/yaml-schema.yaml)
  defines `yaml-schema` itself, and validates itself. Its
  `property(kind, ...)` declaration option is what requires your `$schema` to
  declare a literal or enum `kind`.
- [`schema-trigger.yaml`](../../../schemas/schema-trigger.yaml) uses `types`
  for a recursive `match-expression` and a closed root.
- [`schema.yaml`](../../../../simplified-schema/schemas/schema.yaml) uses a
  root union whose two arms claim the same `schema` kind: one requires
  `$schema`, the other requires `types`.

#### Where to put a definition

Place the file directly in one of the five
[schema roots](./definition.md#schema-roots), not in a subfolder. Its reach
follows the root: a package's `schemas/` covers that package's documents, the
repository's `schemas/` covers the whole repository, and `~/schemas` or the
`SCHEMAS_DIR` folder covers every repository you work in. Definitions are only
read for documents inside a repository; elsewhere only the shipped `schema`,
`yaml-schema`, and `schema-trigger` kinds exist.

Unlike triggers, file names never shadow each other: what matters is the kinds
a file claims. Two definitions that claim one kind, in any roots and under any
names, make every document of that kind fail with an error naming both files.
That includes claiming a shipped kind such as `schema-trigger`. See
[Schema Discovery](./schema-discovery.md) for the full rules, including what a
broken definition does to the documents around it.

#### Changing a shipped definition

In this repository the shipped definitions are ordinary files in schema roots:
[`darkmatter/schemas/schema-trigger.yaml`](../../../schemas/schema-trigger.yaml)
sits in the `darkmatter` package area's root. Edit it in place and the edit
applies at once to documents whose roots include that folder, with no
rebuild, because a file at a shipped definition's repository-relative path
that claims the same kind replaces the compiled-in copy. Documents elsewhere
keep the compiled-in copy until Darkmatter is rebuilt. A copy saved under any
other path is a second claim, and therefore a duplicate.

#### Checking a definition

A definition is itself a YAML document of kind `yaml-schema`, so validating
it checks it against `yaml-schema.yaml` and then checks that its `$schema`
prepares and claims a finite set of kinds (a
[kind check](./definition.md#kind-checks)). Validate one with
`md schema validate schemas/roster.yaml`
([YAML documents on the command line](./definition.md#yaml-documents-on-the-command-line));
the library's entry point is
[`validate_yaml_document`](./definition.md#validating-from-the-library).

### Schemas for Global Variables

Every global variable has a schema, including `fm`, `ctx`, `current`, `env`,
`err`, and `timing`. Darkmatter's built-in definitions start in
[`darkmatter.yaml`](../../../schemas/darkmatter.yaml), which imports reusable
named types from its `partials` directory. `ctx` and `current` share the same
context type, so both expose properties such as `cwd` directly.

Frontmatter is the value of `fm`. Most expressions can omit that namespace:
`title` and `fm.title` refer to the same property. This shorthand does not change
how its schema is defined. A frontmatter property named `ctx`, accessed as
`fm.ctx`, is separate from the `ctx` global.

The schema describes what a value can contain. The application supplies the value
and determines when it is available. Knowing the fields of `err`, for example,
does not make an error available during initialization. Editor assistance uses
the schema and availability declarations without running value providers.

The layering rules below apply to the properties of `fm`. Authored document
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

### Make an Enum from a Data File

When the choices for a property already live in a data file (the operating
systems a project supports, the regions a service runs in), copying them into
the schema means two lists to keep in sync. A `{{ … }}` **slot** lets the
schema take them from the file instead:

```yaml
# schemas/provider.yaml, beside schemas/os.yaml
kind: schema
$schema:
    os: enum({{ load_data("./os.yaml", "string[]") }}; required)
```

A relative path in a slot resolves from the folder of the schema file the
slot is written in, not from the document being validated. A slot may sit
beside literal elements, so `enum(none, {{ … }})` adds `none` to the loaded
values.

A schema with a slot is a **template**. Reading it evaluates nothing: parsing,
querying, exporting, trigger matching, completion, and hover all see the slot
as text. The slot runs only when a command resolves the schema, and every
read it makes must be approved first. A clean, committed file in the schema's
own repository is approved automatically; anything else needs a grant or an
answer at a prompt. See
[When a slot is evaluated](./schema-slots.md#when-a-slot-is-evaluated) and
[What a slot may read](./schema-slots.md#what-a-slot-may-read).

A slot may be written in these positions, and each takes one shape of value:

| Position | Example | The slot must produce |
| --- | --- | --- |
| an `enum(...)` member | `enum(none, {{ list }})` | an array of strings, spliced in as members |
| a `suggest(...)` candidate | `string(suggest(a, {{ list }}))` | an array of strings, numbers, booleans, or `null`s, spliced in as candidates |
| `literal(...)` | `literal({{ revision }})` | one string, number, or boolean |
| `default(...)` | `number(default({{ n }}))` | one value other than `null` that the property's type accepts |
| `min(...)`, `max(...)` | `number(min({{ low }}))` | a number (a non-negative integer for a length or item count) |
| `pattern(...)` | `string(pattern({{ re }}))` | a string, within the pattern cap |

A list slot must return an array even for a single value: a bare scalar is a
wrong shape. A template constraint is the whole argument, so `default({{ d }} + 1)` is
a grammar error. A pattern from a slot is capped so that data can never slow
down validation: it may not use lookaround or a backreference, and it may be
at most 4096 bytes (see
[The pattern cap](../../../../simplified-schema/docs/topics/language/slots.md#the-pattern-cap)).

A **template type** is a slot that supplies a property's whole type
definition (`status: "{{ load_data('./status-type.yaml', 'type-definition') }}"`);
see [Template Types](../../../../simplified-schema/docs/topics/language/slots.md#template-types).

> **Planned:** Darkmatter does not yet grant the reads a returned definition
> causes, so a returned definition that imports a type fails with
> `dm.schema.slot_approval_required`.

The full grammar and every result rule are in
[Slots: Values Supplied by an Expression](../../../../simplified-schema/docs/topics/language/slots.md).
Darkmatter's side (which expressions a slot may contain, where slots may
appear, approvals and grants, caching, the editor, and failure codes) is in
[Schema Slots in Darkmatter](./schema-slots.md).

### Union Types

A union says a property may hold one of several types. Write it with `|`, as
in TypeScript, or as a YAML list; both mean the same thing:

```yaml
$schema:
    id: "string | number"
    width:
      - literal(auto)
      - number(min(1))
```

`[]` binds tighter than `|`, so `string | number[]` is a string or a list of
numbers; group with parentheses for a list whose items may be either:
`(string | number)[]`. A list whose positions each have their own type is a
`tuple`, such as `point: "tuple([number, number])"`.

A value that already matches an option is kept as written: `"4"` against
`string | number` stays the text `"4"`. Otherwise Darkmatter converts it into
the one option it can reach: `"1"` against `number(min(5)) | boolean` becomes
`true`, because `1` fails `min(5)`. The order of the options never matters.
The full rule, including how ties are settled, is in
[Choosing a Union Option](../../../../simplified-schema/docs/topics/conversion.md#choosing-a-union-option); the
grammar is in [The `|` Operator and Grouping](../../../../simplified-schema/docs/topics/language/unions.md#the--operator-and-grouping)
and [Tuples](../../../../simplified-schema/docs/topics/language/types.md#tuples).

### Recursive Types

Some data nests to a depth you cannot know in advance: a tree of sections, a
list of steps where a step can hold more steps. Describe it with a named type
that refers to itself:

```yaml
$schema:
    step: "{ say: string(required), then: step[](required) }"
    plan: step(required)
plan:
    say: start
    then:
        - say: check
          then: []      # an empty list ends this branch
```

Every level is checked against `step`, however deep the document goes, and
an error deep in the tree still points at its own line. Types can also refer
to each other, in one file or across files with `Name@file`.

One rule keeps recursion meaningful: each cycle must pass through a **child
value**, meaning an object property, a dictionary value, a list item, or a
tuple slot. `step` above recurses through the list items of `then`, so it is
fine. `a: b` with `b: a`, or `a: [a, string]`, never reaches a child value
and is a schema error that names each reference in the cycle.

The definition describes the value. Whether a property using it is
`required`, `eager`, or nullable is decided where the type is used, so
`plan: step(required)` and `draft: step` can share one definition. See
[Recursive Named Types](../../../../simplified-schema/docs/topics/language/objects-and-named-types.md#recursive-named-types) for the full
rules.

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
    
    A tagged schema file has `kind: schema`, a `types` mapping, an optional `$schema` export, and optionally a [`categories:` list](../../../../simplified-schema/docs/topics/language/schema-documents.md#categories) that names the documentation categories its types may use; any other key is a schema-document error. Without an explicit export, its `types` entries are both the schema's properties and its named types (see [External Schemas](#external-schemas)):
    
  ```yaml
  kind: schema
  types:
      name: string(required)
      category: enum(good food, yuck, bad before expiry)
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

A third kind, `kind: yaml-schema`, defines a kind of YAML document rather than a Markdown schema; see [Defining a YAML Document Kind](#defining-a-yaml-document-kind).

A conditional trigger supplies its schema under `$schema`; a `kind: schema` file supplies it under `types`. The trigger predicate catalog and complete envelope validation rules are still being finalized; these sketches do not establish additional required fields.

#### Schema Definition Files

#### Trigger Files

## CLI Grammar Support

### Validation

### Discovery

## Language Server (DMLS) Grammar Support
