# Tree Hugger Library

Tree Hugger exposes Tree-sitter-based helpers for extracting symbols and diagnostics from files
and packages.

## TreeFile

```rust
use tree_hugger_lib::TreeFile;

let file = TreeFile::new("src/lib.rs")?;
let symbols = file.symbols()?;
```

## TreePackage

```rust
use tree_hugger_lib::TreePackage;

let package = TreePackage::new(".")?;
let modules = package.modules();
```

## JSON Summaries

The library provides `FileSummary` and `PackageSummary` structs for JSON output. These types are
used by the CLI but are also available to library consumers.
