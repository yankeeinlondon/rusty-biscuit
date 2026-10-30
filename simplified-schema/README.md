# simplified-schema

`simplified-schema` is the home of the **SimplifiedSchema** language: the
compact, type-string schema notation (`name: string`, `age: number(min(0))`)
that Darkmatter, Claudine, and Messenger use to describe and check YAML and
JSON data. The library has no dependency on any Markdown host; Darkmatter
builds its document policy on top of it.

The crate is being extracted from Darkmatter in stages. Today it holds the
shared foundations every schema module builds on:

- `numeric`: admission of written numbers into finite `f64`s;
- `span`: byte spans over the exact supplied text;
- `expression`: passive expression syntax, never evaluated: the `{{ … }}` /
  `{{{ … }}}` scanner, the token lexer, and the parser and AST;
- `pending`: the `{{!data:v1:…}}` literal-token codec.

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
use simplified_schema::numeric::admit_number_spelling;
use simplified_schema::pending::{decode_leaf, encode};

assert_eq!(admit_number_spelling("0.1"), Ok(0.1));
assert!(admit_number_spelling("9007199254740993").is_err());

let token = encode("{{ kept as data }}");
assert_eq!(decode_leaf(&token), Some(Ok("{{ kept as data }}".to_string())));
```

## Documentation

- [Foundations](./docs/topics/foundations.md): the numeric model, spans, the
  expression scan, and the literal-token codec.
- [Expression syntax](./docs/topics/expression-syntax.md): parsing
  expressions without evaluating them.
- [Cargo features](./docs/topics/features.md)
- [Dependencies](./docs/dependencies.md)
