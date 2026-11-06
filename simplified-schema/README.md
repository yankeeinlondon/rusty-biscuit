# Simplified Schema (`SimplifiedSchema`)

The simplified schema was initially designed to provide a compact and ergonomic way for authors to create a JSON schema using YAML syntax. Today it not only provides that _ergonomic_ goal but has increased in power to include many features that plain old JSON Schema does not, however, every simplified schema retains the ability to be converted to JSON schema on demand (even if it might be lossy when the advanced features of simplified schema are employed).

When you use the [Darkmatter](darkmatter/README.md), [Claudine](claudine/README.md), or the [DMLS](darkmatter/dmls/README.md) LSP in your editor you can take full advantage of simplified schemas either for the Frontmatter of your Markdown documents or on YAML files too.

## Types and Constraints

The `SimplifiedSchema` uses both **types** and **constraints** (_of types_) in a manner that is structurally identical to how JSON Schema's use types and constraints:

- every property in the schema must have a **type**
- each **type** can be further narrowed with **constraints**

### Types

The **types** which `SimplifiedSchema` provides to authors overlaps with types found JSON Schema but is not exactly the same. You can refer to the [Simplified Schema's Types](simplified-schema/docs/topics/types.md) documentation to get a full list of _types_ you can use but there are many that are semantically obvious by name alone like:

- `string`
- `number`
- `boolean`

There are some other types, however, which have been added to `SimplifiedSchema` that were added for ergonomic benefits but that JSON schema would have required using "constraints" for, these include types like:

- `integer` / `float` - _while these are indeed **constraints** of a numeric type, they are so common to schemas that it was felt they should be included as a base type_
- `email` - _email is indeed a **constraint** of the `string` type but again it is just so common in schema requirements that it was felt that it should just be included as a type_
- `enum(a,b,c)` - _and enumeration of allowed values_

There are plenty more that fit into this category so please refer to the 

## A Simple Example

Defining a schema in Markdown is as easy as defining the `$schema` Frontmatter property and either pointing it as a YAML schema file or defining it inline like this:

```yaml
$schema:
    name: string(required) -> the employee's name
    age: number(required) -> the employee's age
    employed-since: date -> the date the employee's employment started
    favorite-color: enum(red,blue,green;required;eager) -> the employee's favorite color
```

> In this example we see: 
> 
> - the `name` and `age` properties were defined to be a "string" and "number" respectively 
>   - these two properties are **required** at _runtime_ (where _runtime_ means when the markdown file is composed using libraries like **Darkmatter** and **Claudine**)
> - the `employed-since` property must be a **date** type _if_ defined but since it wasn't declared as "required" it is _optional_ by default
> - the `favorite-color` property to be an **enumerated** types and by declaring both **required** and **eager** we are saying that it MUST be set _before_ composition can take place

## Types, Composition and Lifecycle

Unlike most static type grammars, the `SimplifiedSchema` grammar has a sense of a _lifecycle_:

1. Documents are authored at what we will call "design time"
2. Documents are then _composed_ using the Darkmatter library and go through a lifecycle:

    - this lifecycle is characterized by lifecycle events and whether you're using **Darkmatter**, **Claudine**, or **DMLS** the first lifecycle event is the `initialize` event.

In the example we saw not only how to define the Frontmatter property types but also we 


`simplified-schema` is the home of the **SimplifiedSchema** language: the compact, type-string schema notation (`name: string`, `age: number(min(0))`) that Darkmatter, Claudine, and Messenger use to describe and check YAML and JSON data. The library has no dependency on any Markdown host; Darkmatter builds its document policy on top of it.

Most callers need one type, `SimplifiedSchema`: build it from text or a file, validate values strictly or softly, layer other schemas into it, query its types, and export it. Underneath, the crate holds the language and everything that judges data with it:

- `simplified`: the AST (`SchemaRoot`, `SchemaShape`, `PropertyDef`, …), the type grammar, source maps, cursors, queries, serializers, subtyping, and lowering to Draft 2020-12 JSON Schema;
- `resolve`: named-type imports, cycle checks, use-site merges, and layering, over sources the caller supplies;
- `validate`, `format`, `file_match`: compiled validators with the `darkmatter-*` formats and `x-darkmatter-*` keywords;
- `coercion`, `coerce`: the conversion engine and frontmatter conversion;
- `numeric`, `span`, `expression` (passive `{{ … }}` syntax), and `pending` (the `{{!data:v1:…}}` literal-token codec): the shared foundations.

Darkmatter keeps its own policy (its baseline, schema roots, trigger discovery, and which values are still pending) and calls in here.

The crate also owns the definitions of `kind: schema` and `kind: yaml-schema` documents (`schemas/`); see [Schema documents](./docs/topics/language/schema-documents.md).

## Install

Inside this monorepo, add a path dependency:

```toml
[dependencies]
simplified-schema = { path = "../../simplified-schema/lib" }
```

Disable default features (`default-features = false`) for a build that parses and checks supplied text only, without file resolution or repository discovery.

## Example

```rust
use serde_json::json;
use simplified_schema::SimplifiedSchema;

let person = SimplifiedSchema::try_from("$schema:\n  name: string(required)\n  age: number(min(0))\n")?;
assert_eq!(person.query("age")?.unwrap().type_text(), "number(min(0))");
assert!(person.validate_strict(&json!({ "name": "Ada", "age": 36 })).is_ok());
assert!(person.validate_strict(&json!({ "age": -1 })).is_err());
```

The lower-level pieces are public too:

```rust
use std::sync::Arc;
use biscuit_file::FileResolutionContext;
use serde_json::json;
use simplified_schema::{Prepared, parse_yaml_schema, to_json_schema};

let yaml: serde_yaml_ng::Value = serde_yaml_ng::from_str("name: string(required)\nage: number(min(0))").unwrap();
let schema = to_json_schema(&parse_yaml_schema(&yaml).unwrap()).unwrap();
let prepared = Prepared::new(Arc::new(schema), None, FileResolutionContext::new("/work")).unwrap();
assert!(prepared.validate_strict(&json!({ "name": "Ada", "age": 36 })).is_ok());
assert!(prepared.validate_strict(&json!({ "age": -1 })).is_err());
```

The foundations work on their own:

```rust
use simplified_schema::numeric::admit_number_spelling;
use simplified_schema::pending::{decode_leaf, encode};

assert_eq!(admit_number_spelling("0.1"), Ok(0.1));
assert!(admit_number_spelling("9007199254740993").is_err());

let token = encode("{{ kept as data }}");
assert_eq!(decode_leaf(&token), Some(Ok("{{ kept as data }}".to_string())));
```

## Documentation

- [Authoring a schema](./docs/topics/language/index.md): what a schema looks like, its main forms, the language reference, and its limitations.
- [Conversion](./docs/topics/conversion.md): which almost-right values soft validation converts.
- [Diagnostics](./docs/topics/diagnostics.md): typed errors, where each points, and where a schema came from.
- [Selecting a schema file](./docs/topics/selecting-schema-files.md): which file a schema reference names, what was probed, where a missing file would be created, and whether the file is a valid `$schema` target.
- [Custom vocabulary](./docs/topics/custom-vocabulary.md): the `darkmatter-*` formats and `x-darkmatter-*` keywords in a lowered schema, and what other JSON Schema validators miss.
- [Host capabilities](./docs/topics/host-capabilities.md): which checks need a file system, and when the library reads the running process.
- [The `SimplifiedSchema` object](docs/topics/simplified-schema-struct.md): building, validating, layering, querying, and exporting a schema.
- [Language core](./docs/topics/language-core.md): the modules, the resolver's supplied sources, and what stays in a host.
- [Foundations](./docs/topics/foundations.md): the numeric model, spans, the expression scan, and the literal-token codec.
- [Expression syntax](./docs/topics/expression-syntax.md): parsing expressions without evaluating them.
- [Strict and soft validation](./docs/topics/validation.md): judging values with a prepared schema, with or without conversion.
- [Cargo features](./docs/topics/features.md)
- [Dependencies](./docs/dependencies.md)
