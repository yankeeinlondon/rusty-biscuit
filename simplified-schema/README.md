# simplified-schema

`simplified-schema` is the home of the **SimplifiedSchema** language: the
compact, type-string schema notation (`name: string`, `age: number(min(0))`)
that Darkmatter, Claudine, and Messenger use to describe and check YAML and
JSON data. The library has no dependency on any Markdown host; Darkmatter
builds its document policy on top of it.

Most callers need one type, `SimplifiedSchema`: build it from text or a file,
validate values strictly or softly, layer other schemas into it, query its
types, and export it. Underneath, the crate holds the language and everything
that judges data with it:

- `simplified`: the AST (`SchemaRoot`, `SchemaShape`, `PropertyDef`, …), the
  type grammar, source maps, cursors, queries, serializers, subtyping, and
  lowering to Draft 2020-12 JSON Schema;
- `resolve`: named-type imports, cycle checks, use-site merges, and layering,
  over sources the caller supplies;
- `validate`, `format`, `file_match`: compiled validators with the
  `darkmatter-*` formats and `x-darkmatter-*` keywords;
- `coercion`, `coerce`: the conversion engine and frontmatter conversion;
- `numeric`, `span`, `expression` (passive `{{ … }}` syntax), and `pending`
  (the `{{!data:v1:…}}` literal-token codec): the shared foundations.

Darkmatter keeps its own policy (its baseline, schema roots, trigger
discovery, and which values are still pending) and calls in here.

## Install

Inside this monorepo, add a path dependency:

```toml
[dependencies]
simplified-schema = { path = "../../simplified-schema/lib" }
```

Disable default features (`default-features = false`) for a build that parses
and checks supplied text only, without file resolution or repository
discovery.

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

- [Authoring a schema](./docs/topics/language/index.md): what a schema looks
  like, its main forms, the language reference, and its limitations.
- [Conversion](./docs/topics/conversion.md): which almost-right values soft
  validation converts.
- [Diagnostics](./docs/topics/diagnostics.md): typed errors, where each
  points, and where a schema came from.
- [Custom vocabulary](./docs/topics/custom-vocabulary.md): the
  `darkmatter-*` formats and `x-darkmatter-*` keywords in a lowered schema,
  and what other JSON Schema validators miss.
- [Host capabilities](./docs/topics/host-capabilities.md): which checks need
  a file system, and when the library reads the running process.
- [The `SimplifiedSchema` object](./docs/topics/schema-object.md): building,
  validating, layering, querying, and exporting a schema.
- [Language core](./docs/topics/language-core.md): the modules, the
  resolver's supplied sources, and what stays in a host.
- [Foundations](./docs/topics/foundations.md): the numeric model, spans, the
  expression scan, and the literal-token codec.
- [Expression syntax](./docs/topics/expression-syntax.md): parsing
  expressions without evaluating them.
- [Strict and soft validation](./docs/topics/validation.md): judging
  values with a prepared schema, with or without conversion.
- [Cargo features](./docs/topics/features.md)
- [Dependencies](./docs/dependencies.md)
