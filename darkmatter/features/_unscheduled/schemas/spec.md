# Schemas in Darkmatter

Darkmatter brings in the ability to define, detect and evaluate schemas for Frontmatter.


## Schema Definition

There is no formalized spec for schemas in Frontmatter nor in YAML but with YAML there is a strong _informal_ approach of setting
the `$schema` property as a reference to a JSON-Schema. We will leverage that informal starting point but we want to ensure that also design a solution where non-technical people can easily create a schema easily and not deal with the non-ergonomic nature of
JSON Schemas.

- **Decision #1** the `$schema` property will be reserved for schema definitions in Darkmatter
- **Decision #2** the _value_ of `$schema` can point to a URI of a JSON Schema but we will also allow it to be defined in what we'll call **SimplifiedSchema**
- **Decision #3** all valid `SimplifiedSchema` definitions must be able to efficiently be converted into a JSON schema
- **Decision #4** all schema validation will be done against JSON Schemas

## SimplifiedSchema Definitions

A `SimplifiedSchema` definition might look something like:

```yaml
$schema:
    name: string
    age: number
```

- the simplicity of this grammar makes it easily usable by a large set of people and keeps friction low.
- the syntax is:
    - `{property}: {type}` 
    - `{property}: {type}({constraints})`
    - or `{property}: {type} -> {description}` / `{property}: {type}({constraints}) -> {description}`
- all properties defined are _optional_ unless constrained to be **required**

### Types

The types which `SimplifiedSchema` provides are:

- `string` - any string value
- `number` - any numeric value
- `numberlike` - allows numeric values or string representations of a number (e.g., "4", "-13", etc.)
- `boolean` - any boolean value
- `boolish` - any boolean value or the strings "true" or "false"
- `object` - a dictionary is any object key/value shape
- `file` - a file reference which follows the conventions put in place by the `biscuit-file` library's `FileReference`
- `enum` - an enumerated set of values
- `any` - allows any type of value; also allows this to be _constrained_ as **required**

In addition we represent arrays of each of these types with a trailing `[]`.

### Type Constraints

All types allow constraints to be used to further _constrain_ the valid types. The type of constraints allowed vary by type
but all type allow for the `required` constraint:

```yaml
$schema:
    name: string(required)
    age: number
```

> makes the `name` property required

#### Enumerations

An enumeration by definition is an enumerated set of potential values. To represent an **enum** type we therefore _must_ define the enumerated list that it represents:

```yaml
color: "enum(red,blue,' green', yellow)"
```

The comma is used to separate elements and by default all exterior whitespace is removed. That means that in the example above:

- the color green preserves a leading space because it explicitly used quotes to express that,
- by contract the color yellow's leading space is removed which is the default behavior
- we only support the definition of string values for enumerations but you can use a _number-like_ enumeration like:

```yaml
tier: "enum(1,2,3)"
```

This represents the string values "1", "2", and "3" as elements in the enumerated set.

#### Numbers

A numeric type does not need to be constrained at all but can be by the following dimensions:

- max(#)
- min(#)
- integer (aka, _not float_)

To illustrate this here are some example definitions:

```yaml
some_number: number
positive_int: "number(min(0), integer)"
```

#### Strings

A string, like numbers, offers a few ways to constrain the type:

- min(#) - _(minimum length)_
- max(#) - _(maximum length)_
- date - enforces `YYYY-MM-DD` ISO format
- datetime - enforces `YYYY-MM-DD hh:mm:(ss).(ms)Z(offset)`

#### Files

A file type is a string representation of a file path. It allows for magic paths which lead with `@` and all other path variants that the `FileReference` struct provides for but it there is no need to constrain the type to express this. There is, however, a set dimensions that file types can be constrained on:

- ext([file-extension])
- path([paths])

To help illustrate how you might configure this, here's an example:

```yaml
doc: "file(ext(md,txt))"
source_code: "file(ext(rs,ts,js))"
```

## Schema Detection

Before we go into schema definition, let's cover the topic of "detection". As we all know a single record is not going to
truly represent a "schema" but it's still useful to be able to report on a Markdown document's observed schema or shape.

The Darkmatter schema will make checking a document's perceived "schema" with the `schema` subcommand.

```sh
md schema document.md
```

Will evaluate the specified document and returns a YAML representation of the
