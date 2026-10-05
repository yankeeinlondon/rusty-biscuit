# Tree Hugger Library

Tree Hugger exposes Tree-sitter-based helpers for extracting symbols and diagnostics from files
and packages.

## TreeFile

The core API for file-level symbol extraction and diagnostics.

```rust
use tree_hugger::{Capability, SourceFile, SymbolKindV2, SymbolQuery, TreeFile};

let file = TreeFile::new("src/lib.rs")?;

// Symbol extraction
let symbols = file.symbols()?;           // All symbols combined
let v2_symbols = file.symbol_records()?; // Symbol Schema v2 parse records
let v2_index = file.symbol_index_v2()?;  // Complete index: every pass, diagnostics included
let imports = file.imported_symbols()?;  // Import statements
let exports = file.exported_symbols()?;  // Exported symbols
let locals = file.local_symbols()?;      // Local definitions
let refs = file.referenced_symbols()?;   // All identifier usages
let reexports = file.reexported_symbols()?; // Imports that are re-exported

// Selective analysis: only the kinds and work a caller needs
let query = SymbolQuery::new()
    .kinds([SymbolKindV2::Function, SymbolKindV2::Method])
    .capabilities([Capability::Containment]);
let result = SourceFile::read("src/lib.rs")?.query(&query)?; // or file.query(&query)
// result.index.symbols holds only functions and methods; no diagnostics ran

// Diagnostics
let lint = file.lint_diagnostics();      // Pattern-based lint rules
let syntax = file.syntax_diagnostics();  // Syntax/parse errors
let all = file.diagnostics();            // Unified diagnostic format
let dead = file.dead_code();             // Unreachable code detection
```

## TreePackage

```rust
use tree_hugger::TreePackage;

let package = TreePackage::new(".")?;
let modules = package.modules();
```

## SymbolKind

Symbols are categorized by kind, allowing consumers to distinguish between different type constructs.
The v1 `SymbolKind` and the schema `SymbolKindV2` have the same variants:

```rust
pub enum SymbolKind {
    Function,    // Free functions
    Method,      // Functions declared directly in a type, trait, impl, protocol or extension body
    Constructor, // TypeScript/JavaScript `constructor`
    Type,        // Structs, unions, records, Go types, Swift types
    TypeAlias,   // `type`/`typedef`/`using` aliases
    Class,       // Class definitions
    Interface,   // Interfaces, protocols
    Enum,        // Enumeration types
    EnumVariant, // Rust variants, Swift `case`
    Trait,       // Traits (Rust, Scala, PHP)
    Module,      // Rust `mod`, Perl `package`, Scala `object`
    Namespace,   // Go/Java `package`, C#/C++ `namespace`
    Variable,    // Variables, locals, Rust `static` items
    Parameter,   // Function parameters
    Field,       // Struct/class fields
    Property,    // JS/TS class-body properties, TS interface properties, C# properties
    Macro,       // Macro definitions
    Constant,    // Constants
    Unknown,     // Unrecognized symbols
}
```

`is_function()` includes `Constructor`, and `is_type()` includes `TypeAlias`.

### Language-Specific Mappings

| Language | Type | Class | Interface | Enum | Trait | TypeAlias |
|----------|:----:|:-----:|:---------:|:----:|:-----:|:---------:|
| Rust | struct, union | - | - | enum | trait | `type` |
| TypeScript | - | class | interface | enum | - | `type` |
| Java | record | class | interface | enum | - | - |
| C# | struct, record | class | interface | enum | - | - |
| C | struct, union | - | - | enum | - | `typedef` |
| C++ | struct, union | class | - | enum | - | `typedef`, `using` |
| Swift | struct, class, enum* | - | protocol | - | - | - |
| Scala | - | class | - | enum | trait | - |
| PHP | - | class | interface | enum | trait | - |
| Go | type | - | - | - | - | - |
| Python | - | class | - | - | - | - |

\* Swift structs, classes and enums are all captured as Type; see Known Limitations.

The per-language rules for kinds, containers, visibility and `static` are in
[Symbol Kinds, Visibility and `static`](../docs/symbol-kinds-and-visibility.md).

## Symbol Metadata

Symbols include rich metadata when available:

```rust
pub struct SymbolInfo {
    pub name: String,
    pub kind: SymbolKind,
    pub range: CodeRange,
    pub language: ProgrammingLanguage,
    pub file: PathBuf,
    pub container_name: Option<String>,
    pub container_kind: Option<SymbolKind>,
    pub doc_comment: Option<String>,
    pub visibility: Option<Visibility>, // None = unknown
    pub is_static: bool,                // member usable without an instance
    pub is_static_storage: bool,        // storage/linkage `static`
    pub signature: Option<FunctionSignature>,
    pub type_metadata: Option<TypeMetadata>,
}
```

- `visibility` is set for every kind, not only functions. It is the access
  declared or implied at the declaration (not effective access through
  enclosing modules), and `None` where no access rules apply or the file
  cannot tell.
- `is_static` and `is_static_storage` are independent: a C++ `static` data
  member has `is_static`, a C `static` function or a Rust `static` item has
  `is_static_storage`.
- Every function-like symbol (Function, Method, Constructor) has a
  `signature`, with an empty parameter list when there are none.

### Visibility

```rust
pub enum Visibility {
    Public,
    Protected,
    Private,
    Internal,          // C#/Swift `internal`
    Package,           // unmarked Java member, Swift `package`, lowercase Go name
    Crate,             // Rust `pub(crate)`, `pub(super)`, `pub(in path)`
    ModulePrivate,     // C `static` linkage, Swift `fileprivate`, unexported module-level JS/TS
    ProtectedInternal, // C# `protected internal`
    PrivateProtected,  // C# `private protected`
}
```

`Visibility` mirrors the schema's `VisibilityLevel` one-to-one, except that
`VisibilityLevel::Unknown` is `None` in v1.

### FunctionSignature

For functions, methods and constructors:

```rust
pub struct FunctionSignature {
    pub parameters: Vec<ParameterInfo>,
    pub return_type: Option<String>,
    pub is_static: bool, // copy of SymbolInfo::is_static
}
```

Visibility lives only on `SymbolInfo::visibility`; `FunctionSignature` has no
visibility field.

### TypeMetadata

For type definitions:

```rust
pub struct TypeMetadata {
    pub fields: Vec<FieldInfo>,           // Fields and properties only, never methods
    pub variants: Vec<VariantInfo>,       // Enum variants
    pub type_parameters: Vec<String>,     // Generic parameters
}
```

A `FieldInfo`'s `visibility` and `is_static` are copied from the standalone
Field, Property or Constant symbol of the same name and container, so type
metadata and standalone symbols agree.

### Schema records

`SymbolRecord` (schema 4.0) carries the same facts:

- `visibility.level` holds the level; `modifiers` holds `Static` for member
  `static` and `StaticStorage` for storage/linkage `static`.
- `identity.container_kind` holds the container's kind next to
  `identity.container_name` (the container name).
- `identity.keyword` holds the declaring keyword (`fn`, `struct`, `def`,
  `const`, ...) read from the syntax tree; it is absent when the construct
  has none, such as a Java method.
- `source.declaration_text` holds the declaration up to its body, keeping
  modifiers such as `pub(in crate::a)` that a level cannot express.
- `source.signature_text` holds the display header: `declaration_text`
  without leading annotations, initializers, accessors or a terminator
  (`pub const TABLE: [u8; 4] = [...];` has the header
  `pub const TABLE: [u8; 4]`).
- `relations.implements` lists the trait of a Rust trait-impl member.
- `visibility.is_exported` comes only from export syntax and the bind pass,
  never from a Public level; converting a `SymbolInfo` always yields
  `is_exported: false`.

The `From<SymbolInfo>` / `From<SymbolRecord>` adapters preserve kind,
visibility, both `static` flags, container kind and signature presence.

## JSON Summaries

The library provides `FileSummary` and `PackageSummary` structs for JSON output. These types are
used by the CLI but are also available to library consumers.

Schema output (version 4.0) is exposed via `SymbolRecord` and `FileSymbolIndex`, including `schema_version`
and `completed_passes` metadata for staged analysis. A `SymbolQuery` result adds
`completed_capabilities`. Pass dependencies, capabilities and kind selection
are described in [Symbol Schema Design](../docs/symbol-schema-design.md#extraction-pipeline-contract).

## Testing

### Test Coverage Philosophy

Tree Hugger prioritizes comprehensive cross-language test coverage. When working on this library:

1. **Every language with type constructs must have type distinction tests** - If a language supports multiple type kinds (struct/enum, class/interface), tests must verify correct `SymbolKind` assignment.

2. **Fixture files for all supported languages** - Each language should have `sample.*` and `types.*` fixtures exercising its features.

3. **Regression tests are mandatory** - Bug fixes must include tests that would fail without the fix.

4. **Diagnostics coverage** - Add regression tests for semantic lint rules and syntax diagnostics in `tests/l1/lint_diagnostics.rs` and `tests/l1/tree_file.rs`.

### Running Tests

```bash
cargo test -p tree-hugger-lib
```

### Test Structure

Integration tests compile into one `l1` test binary. `Cargo.toml` sets
`autotests = false`, so a new file under `tests/l1/` runs only once
`tests/l1/main.rs` declares it with `mod`; the `test_layout` gate fails
otherwise. Language tests are in `tests/l1/tree_file.rs` and follow this
pattern:

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

## Lint Diagnostics

Tree Hugger provides semantic and pattern-based lint diagnostics for all 16 supported languages.

### Getting Diagnostics

```rust
use tree_hugger::TreeFile;

let file = TreeFile::new("src/main.rs")?;
let diagnostics = file.lint_diagnostics();

for d in diagnostics {
    println!("{}: {} ({})", d.range.start_line, d.message, d.severity);
}
```

### Semantic Rules

These rules analyze symbol definitions, imports, and references:

| Rule | Severity | Description | Default |
|------|----------|-------------|---------|
| `undefined-symbol` | Error | Reference to a symbol not defined, imported, or builtin | experimental |
| `undefined-module` | Warning | Reference to undefined module or namespace | experimental |
| `unused-symbol` | Warning | Symbol defined but never referenced or exported | experimental |
| `unused-import` | Warning | Imported symbol never used in the file | off (low confidence) |
| `dead-code` | Warning | Code after unconditional return/throw/panic | on |

`unused-import` is off by default: without trait/type resolution a syntactic scan
cannot see imports used only through trait methods (`Write` for `.write_all()`) or
macros, so it has an irreducible false-positive rate. Enable with `--warn unused-import`.
The `experimental` rules require `--experimental-semantics`.

### Pattern Rules

Language-specific pattern matching rules:

| Language | Rule | Description | Default |
|----------|------|-------------|---------|
| **Rust** | `unwrap-call` | Explicit `.unwrap()` call | off (`restriction`) |
| **Rust** | `expect-call` | Explicit `.expect()` call | off (`restriction`) |
| **Rust** | `dbg-macro` | Debug macro `dbg!()` usage | on |
| **JavaScript/TypeScript** | `debugger-statement` | `debugger;` statement | on |
| **JavaScript/TypeScript** | `eval-call` | Usage of `eval()` | on |
| **Python** | `eval-call` | Usage of `eval()` | on |
| **Python** | `exec-call` | Usage of `exec()` | on |
| **Python** | `breakpoint-call` | Usage of `breakpoint()` | on |
| **PHP** | `eval-call` | Usage of `eval()` | on |

`unwrap-call` and `expect-call` are `restriction`-category rules: the construct is
valid and deliberate, so they are silent by default. Enable them with `--warn <rule>`,
`--deny <rule>`, or `--deny category:restriction`.

### Ignore Directives

Suppress diagnostics with comments:

```rust
// tree-hugger-ignore: unwrap-call
let value = option.unwrap();  // This unwrap is not reported

// tree-hugger-ignore
let risky = data.unwrap();    // All rules ignored on next line

// tree-hugger-ignore-file: unused-import
// Ignores unused-import for the entire file
```

Supported comment styles: `//`, `#`, `--`, `;`

### Builtin Detection

Semantic rules avoid false positives by recognizing language builtins:

```rust
use tree_hugger::{is_builtin, ProgrammingLanguage};

assert!(is_builtin(ProgrammingLanguage::Rust, "Option"));
assert!(is_builtin(ProgrammingLanguage::Python, "print"));
assert!(is_builtin(ProgrammingLanguage::JavaScript, "console"));
```

## Builtins

The library maintains builtin symbol databases for all 16 languages to avoid false positive "undefined symbol" diagnostics:

```rust
use tree_hugger::{is_builtin, ProgrammingLanguage};

assert!(is_builtin(ProgrammingLanguage::Rust, "Option"));
assert!(is_builtin(ProgrammingLanguage::Python, "print"));
assert!(is_builtin(ProgrammingLanguage::JavaScript, "console"));
assert!(is_builtin(ProgrammingLanguage::Go, "make"));
```

## Dead Code Detection

Control flow analysis detects unreachable code after terminal statements:

```rust
use tree_hugger::{is_terminal_statement, find_dead_code_after, ProgrammingLanguage};

// Detects terminal statements per language
// Rust: panic!, unreachable!, todo!, unimplemented!, process::exit(), process::abort()
// Go: panic(), os.Exit()
// C/C++: exit(), abort(), _exit(), _Exit(), quick_exit()
// Swift: fatalError(), preconditionFailure(), assertionFailure()
// Perl: die, exit
// Lua: error(), os.exit()
```

## Known Limitations

### Swift Type Distinction

Swift's tree-sitter grammar uses a single `class_declaration` node for struct, class, and enum declarations, differentiated by a `declaration_kind` field. Query-level predicates to distinguish these don't work reliably, so all are currently captured as `SymbolKind::Type`. Protocols are correctly captured as `SymbolKind::Interface`.

### Go Interface Detection

Go uses a single `type_declaration` for both struct and interface types. The library captures all as `SymbolKind::Type` since Go's type system treats them uniformly.

`type A = B` aliases are not reported, and struct fields are not reported as
standalone symbols.

### Unreported Instance Attributes

Python attributes assigned only through `self.x`, and JavaScript properties
assigned only through `this.x`, are not reported; declarations in the class
body are. TypeScript never reported `this.x`-only properties.

### Perl

Perl coverage is narrow: named subs (Function), `package` (Module) and names
declared by `my`, `our` and `state` (Variable, keeping their sigil). `local`
is never reported, and visibility is always unknown.

The full list, including unresolved visibility cases, is in
[Symbol Kinds, Visibility and `static`](../docs/symbol-kinds-and-visibility.md#current-limitations).
