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
- `date` - an ISO Date of the format YYYY-MM-DD
- `datetime` - any valid ISO Datetime
- `time` - any valid ISO time representation in the form of `hh:mm{TZ}`, `hh:mm:ss{TZ}`, `hh:mm:ss.ms{TZ}` where TZ can be included or excluded but when included must be `Z` or `{+-}{##}:{##}`
- `number` - any numeric value
- `numberlike` - allows numeric values or string representations of a number (e.g., "4", "-13", etc.)
- `boolean` - any boolean value
- `boolish` - any boolean value or the strings "true" or "false"
- `object` - a dictionary is any object key/value shape
- `file` - a file reference which follows the conventions put in place by the `biscuit-file` library's `FileReference`
- `enum` - an enumerated set of values
- `any` - allows any type of value; also allows this to be _constrained_ as **required**

In addition we represent arrays of each of these types with a trailing `[]`.

## Type Constraints

All types allow constraints to be used to further _constrain_ the valid types. The type of constraints allowed vary by type
but all type allow for the `required` constraint:

```yaml
$schema:
    name: string(required)
    age: number
```

> makes the `name` property required while keeping `age` as optional (the default)

The parser/lexor ignores whitespace in Constraint strings giving users the freedom to add whitespace to increase visual clarity. In the example above, the constraint was `string(required)` but defining it as `"string( required )"` would have been equally as valid. 

> **Note:** when you include whitespace you will need to single or double quote the type's value so that it remains a valid YAML string.

Finally, for types which have more than just one constraint type, the `;` character will be used to delimit one constraint definition from the next.

#### Enumeration Constraints

- An enumeration is the only type which REQUIRES that it have constraints defined because it needs to express the elements of the enumeration! 
- In addition to defining it's own elements it allows specifying the enumeration as required

Here's an example:

```yaml
$schema:
    color: enum(red,green,blue;required)
```

#### Numbers

Numeric types offer the following constrains (in addition to `required`):

- max(#)
- min(#)
- integer

Example:

```yaml
$schema:
    opt_number: number
    req_positive_int: "number(min(0); integer; required)"
```

#### Strings

String types offer the following constrains (in addition to `required`):

- min(#) - _(minimum length)_
- max(#) - _(maximum length)_
- not-empty - _do not allow empty strings (including just whitespace)_

Example:

```yaml
$schema:
    name: string(not-empty;required)
    favorite_expression: string(min(5))
```

#### Files

- A **file** _type_ is a string representation of a file path. 
- The _validity_ of the **file** type is based on:
    - the FileReference struct can resolve the file reference to a file on the filesystem
- 

- It allows for magic paths which lead with `@` and all other path variants that the `FileReference` struct provides for but it there is no need to constrain the type to express this. There is, however, a set dimensions that file types can be constrained on:

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
