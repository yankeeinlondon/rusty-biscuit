# Schemas in Darkmatter

Darkmatter brings in the ability to define and evaluate schema's for Frontmatter.

## Types

The types which Darkmatter provides are:

- `string` - any string value
- `number` - any numeric value
- `numberlike` - allows numeric values or string representations of a number (e.g., "4", "-13", etc.)
- `boolean` - any boolean value
- `boolish` - any boolean value or the strings "true" or "false"
- `object` - a dictionary is any object key/value shape
- `file` - a file reference which follows the conventions put in place by the `biscuit-file` library's `FileReference`
- `enum` - an enumerated set of values
- `any` - can represent any type which by itself is not terribly useful as this is what the _type_ of any undefined Frontmatter property is anyway. It most commonly used in conjunction with the array modifier as `any[]` to generically represent an array of any type.

In addition we represent arrays of each of these types with a trailing `[]`.

### Type Constraints

Of the types above the `enum` type is the only one which _requires_ that constraints be defined:

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
