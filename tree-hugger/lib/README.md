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

## SymbolKind

Symbols are categorized by kind, allowing consumers to distinguish between different type constructs:

```rust
pub enum SymbolKind {
    Function,   // Regular functions
    Method,     // Methods (functions with self/this)
    Type,       // Structs, type aliases
    Class,      // Class definitions
    Interface,  // Interfaces, protocols
    Enum,       // Enumeration types
    Trait,      // Traits (Rust, Scala, PHP)
    Module,     // Modules, namespaces
    Namespace,  // Package/namespace declarations
    Variable,   // Variables, locals
    Parameter,  // Function parameters
    Field,      // Struct/class fields
    Macro,      // Macro definitions
    Constant,   // Constants
    Unknown,    // Unrecognized symbols
}
```

### Language-Specific Mappings

| Language | Type | Class | Interface | Enum | Trait |
|----------|:----:|:-----:|:---------:|:----:|:-----:|
| Rust | struct | - | - | enum | trait |
| TypeScript | type alias | class | interface | enum | - |
| Java | - | class | - | enum | - |
| C# | struct, record | class | interface | enum | - |
| C/C++ | struct | class (C++) | - | enum | - |
| Swift | struct | class | protocol | enum* | - |
| Scala | - | class | - | enum | trait |
| PHP | - | class | interface | enum | trait |
| Go | type | - | - | - | - |
| Python | - | class | - | - | - |

\* Swift enum is captured as Type due to grammar limitations; see Known Limitations.

## Symbol Metadata

Symbols include rich metadata when available:

```rust
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub range: CodeRange,
    pub language: ProgrammingLanguage,
    pub file: PathBuf,
    pub doc_comment: Option<String>,
    pub signature: Option<FunctionSignature>,
    pub type_metadata: Option<TypeMetadata>,
}
```

### FunctionSignature

For functions and methods:

```rust
pub struct FunctionSignature {
    pub parameters: Vec<ParameterInfo>,
    pub return_type: Option<String>,
}
```

### TypeMetadata

For type definitions:

```rust
pub struct TypeMetadata {
    pub fields: Vec<FieldInfo>,           // Struct fields
    pub variants: Vec<EnumVariantInfo>,   // Enum variants
    pub type_parameters: Vec<String>,     // Generic parameters
}
```

## JSON Summaries

The library provides `FileSummary` and `PackageSummary` structs for JSON output. These types are
used by the CLI but are also available to library consumers.

## Testing

### Test Coverage Philosophy

Tree Hugger prioritizes comprehensive cross-language test coverage. When working on this library:

1. **Every language with type constructs must have type distinction tests** - If a language supports multiple type kinds (struct/enum, class/interface), tests must verify correct `SymbolKind` assignment.

2. **Fixture files for all supported languages** - Each language should have `sample.*` and `types.*` fixtures exercising its features.

3. **Regression tests are mandatory** - Bug fixes must include tests that would fail without the fix.

### Running Tests

```bash
cargo test -p tree-hugger-lib
```

### Test Structure

Tests are in `tests/tree_file.rs` and follow this pattern:

```rust
#[test]
fn distinguishes_rust_struct_from_enum() -> Result<(), TreeHuggerError> {
    let tree_file = TreeFile::new(fixture_path("types.rs"))?;
    let symbols = tree_file.symbols()?;

    let point = symbols.iter().find(|s| s.name == "Point");
    assert_eq!(point.unwrap().kind, SymbolKind::Type);

    let message = symbols.iter().find(|s| s.name == "Message");
    assert_eq!(message.unwrap().kind, SymbolKind::Enum);

    Ok(())
}
```

## Known Limitations

### Swift Type Distinction

Swift's tree-sitter grammar uses a single `class_declaration` node for struct, class, and enum declarations, differentiated by a `declaration_kind` field. Query-level predicates to distinguish these don't work reliably, so all are currently captured as `SymbolKind::Type`. Protocols are correctly captured as `SymbolKind::Interface`.

### Go Interface Detection

Go uses a single `type_declaration` for both struct and interface types. The library captures all as `SymbolKind::Type` since Go's type system treats them uniformly.
