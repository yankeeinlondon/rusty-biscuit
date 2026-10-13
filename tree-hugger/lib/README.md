# Tree Hugger Library

`tree-hugger` reads source files in 16 languages with
[Tree-sitter](https://tree-sitter.github.io/) and reports what they declare,
import, export and reference, along with lint and syntax diagnostics. It is
the engine behind the [`hug` CLI](../cli/README.md).

- **Symbols** with kind, location, container, visibility, signature and type
  metadata, as a flat `SymbolInfo` or a full schema `SymbolRecord`.
- **Selective analysis**: a `SymbolQuery` asks for only the kinds and work it
  needs, and a persistent cache answers repeat queries without parsing.
- **References** resolved through lexical scopes in Rust, TypeScript and
  JavaScript, each with a confidence.
- **Diagnostics**, ignore comments and oversized-file ("god file") analysis.

## Install

The crate is not published to crates.io; depend on it from Git:

```toml
[dependencies]
tree-hugger = { git = "https://github.com/yankeeinlondon/rusty-biscuit" }
```

## Example

```rust
use tree_hugger::{TreeFile, TreeHuggerError};

fn main() -> Result<(), TreeHuggerError> {
    let file = TreeFile::new("src/shapes.rs")?;
    for symbol in file.symbols()? {
        println!("line {:<3} {} {}", symbol.range.start_line, symbol.kind, symbol.qualified_name);
    }
    Ok(())
}
```

```text
line 4   type Point
line 5   field Point::x
line 11  method Point::norm
line 22  function origin
```

The output is trimmed; [the guide](../docs/library.md#quick-start) shows the full run.

## Documentation

- [Tree Hugger Overview](../docs/overview.md): what Tree Hugger does and
  where to read next
- [Using the Tree Hugger Library](../docs/library.md): the guide to this
  crate, from the first example to queries, the cache, references,
  diagnostics, god files and errors.
- [Symbol Records](../docs/symbol-records.md): every field of a
  `SymbolRecord`.
- [Symbol Kinds, Visibility and `static`](../docs/symbol-kinds-and-visibility.md):
  how each language's declarations map to kinds and access levels.
- [Scope-Aware References](../docs/references.md): occurrences, confidence
  and coverage.
- [Tree Hugger's Persistent Symbol Cache](../docs/cache-design.md): keys,
  entries and invalidation.

Run the tests with `just test` from `tree-hugger/`.
