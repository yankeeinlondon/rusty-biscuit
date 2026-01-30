# Tree Hugger API Reference

## Core Types

### TreeFile

The primary API for file-level operations.

```rust
use tree_hugger_lib::TreeFile;

// Create from file path (auto-detects language)
let file = TreeFile::new("src/lib.rs")?;

// Create with explicit language override
let file = TreeFile::with_language("src/lib.rs", Some(ProgrammingLanguage::Rust))?;

// Properties
file.file       // PathBuf - file path
file.language   // ProgrammingLanguage
file.hash       // String - content hash
```

### TreeFile Methods

#### Symbol Extraction

```rust
// All public symbols (functions, types, methods, etc.)
fn symbols(&self) -> Result<Vec<SymbolInfo>, TreeHuggerError>

// Import statements
fn imported_symbols(&self) -> Result<Vec<ImportSymbol>, TreeHuggerError>

// Exported symbols (language-specific)
fn exported_symbols(&self) -> Result<Vec<SymbolInfo>, TreeHuggerError>

// Local definitions (file-scoped)
fn local_symbols(&self) -> Result<Vec<SymbolInfo>, TreeHuggerError>

// All identifier references in the file
fn referenced_symbols(&self) -> Result<Vec<ReferencedSymbol>, TreeHuggerError>

// Imports that are also exported
fn reexported_symbols(&self) -> Result<Vec<ImportSymbol>, TreeHuggerError>
```

#### Diagnostics

```rust
// Pattern-based lint rules (unwrap-call, dbg-macro, etc.)
fn lint_diagnostics(&self) -> Vec<LintDiagnostic>

// Syntax/parse errors
fn syntax_diagnostics(&self) -> Vec<SyntaxDiagnostic>

// Unified diagnostic format (lint + syntax)
fn diagnostics(&self) -> Vec<Diagnostic>

// Unreachable code after terminal statements
fn dead_code(&self) -> Vec<DeadCodeDiagnostic>
```

### TreePackage

Package-level operations across multiple files.

```rust
use tree_hugger_lib::{TreePackage, TreePackageConfig};

// Create with defaults (auto-detect git root)
let package = TreePackage::new(".")?;

// Create with config
let config = TreePackageConfig {
    language: Some(ProgrammingLanguage::Rust),
    ignore_patterns: vec!["**/target/**".to_string()],
};
let package = TreePackage::with_config(".", config)?;

// Get module names (Rust only)
let modules = package.modules();
```

## Symbol Types

### SymbolInfo

Main symbol representation.

```rust
pub struct SymbolInfo {
    pub name: String,
    pub kind: SymbolKind,
    pub range: CodeRange,
    pub language: ProgrammingLanguage,
    pub file: PathBuf,
    pub doc_comment: Option<String>,
    pub signature: Option<FunctionSignature>,    // For functions/methods
    pub type_metadata: Option<TypeMetadata>,     // For types/classes/enums
}
```

### SymbolKind

```rust
pub enum SymbolKind {
    Function,   // Regular functions
    Method,     // Functions with self/this
    Type,       // Structs, type aliases
    Class,      // Class definitions
    Interface,  // Interfaces, protocols
    Enum,       // Enumerations
    Trait,      // Traits (Rust, Scala, PHP)
    Module,     // Modules, namespaces
    Namespace,  // Package declarations
    Variable,   // Variables
    Parameter,  // Function parameters
    Field,      // Struct/class fields
    Macro,      // Macro definitions
    Constant,   // Constants
    Unknown,    // Unrecognized
}

impl SymbolKind {
    fn is_function(&self) -> bool  // Function or Method
    fn is_type(&self) -> bool      // Type, Class, Interface, Enum, Trait
    fn is_class(&self) -> bool     // Class only
}
```

### FunctionSignature

```rust
pub struct FunctionSignature {
    pub parameters: Vec<ParameterInfo>,
    pub return_type: Option<String>,
    pub visibility: Option<Visibility>,
    pub is_static: bool,
}

pub struct ParameterInfo {
    pub name: String,
    pub type_annotation: Option<String>,
    pub default_value: Option<String>,
    pub is_variadic: bool,
}

pub enum Visibility {
    Public,
    Protected,
    Private,
    Internal,
}
```

### TypeMetadata

```rust
pub struct TypeMetadata {
    pub fields: Vec<FieldInfo>,           // Struct/class fields
    pub variants: Vec<VariantInfo>,       // Enum variants
    pub type_parameters: Vec<String>,     // Generic parameters <T, U>
}

pub struct FieldInfo {
    pub name: String,
    pub type_annotation: Option<String>,
    pub visibility: Option<Visibility>,
    pub is_static: bool,
}

pub struct VariantInfo {
    pub name: String,
    pub tuple_fields: Vec<String>,        // Variant(T1, T2)
    pub struct_fields: Vec<FieldInfo>,    // Variant { field: T }
}
```

### ImportSymbol

```rust
pub struct ImportSymbol {
    pub name: String,                      // Imported name (or alias)
    pub original_name: Option<String>,     // Original name if aliased
    pub alias: Option<String>,             // Alias if renamed
    pub source: Option<String>,            // Module/package source
    pub range: CodeRange,                  // Symbol location
    pub statement_range: Option<CodeRange>, // Full statement range
    pub language: ProgrammingLanguage,
    pub file: PathBuf,
}
```

### ReferencedSymbol

```rust
pub struct ReferencedSymbol {
    pub name: String,
    pub range: CodeRange,
    pub is_qualified: bool,      // e.g., foo.bar, Foo::bar
    pub qualifier: Option<String>, // The qualifier (foo, Foo)
}
```

## Diagnostic Types

### LintDiagnostic

```rust
pub struct LintDiagnostic {
    pub rule: String,            // e.g., "unwrap-call", "dbg-macro"
    pub message: String,
    pub severity: DiagnosticSeverity,
    pub range: CodeRange,
    pub context: Option<SourceContext>,
}
```

### SyntaxDiagnostic

```rust
pub struct SyntaxDiagnostic {
    pub message: String,
    pub range: CodeRange,
    pub context: Option<SourceContext>,
}
```

### Diagnostic (Unified)

```rust
pub struct Diagnostic {
    pub kind: DiagnosticKind,
    pub severity: DiagnosticSeverity,
    pub rule: Option<String>,
    pub message: String,
    pub range: CodeRange,
    pub context: Option<SourceContext>,
}

pub enum DiagnosticKind {
    Lint,      // Pattern-based rules
    Semantic,  // undefined-symbol, unused-import, etc.
    Syntax,    // Parse errors
}

pub enum DiagnosticSeverity {
    Error,
    Warning,
    Info,
}
```

## Utility Functions

### Builtins

```rust
use tree_hugger_lib::{is_builtin, ProgrammingLanguage};

// Check if identifier is a language builtin
is_builtin(ProgrammingLanguage::Rust, "Option")      // true
is_builtin(ProgrammingLanguage::Python, "print")     // true
is_builtin(ProgrammingLanguage::JavaScript, "Array") // true
```

### Dead Code Detection

```rust
use tree_hugger_lib::{is_terminal_statement, find_dead_code_after};

// Check if a statement is terminal (never returns)
is_terminal_statement(node, source, ProgrammingLanguage::Rust)

// Find dead code after terminal statements
find_dead_code_after(tree, source, ProgrammingLanguage::Rust)
```

### Ignore Directives

```rust
use tree_hugger_lib::IgnoreDirectives;

// Parse ignore directives from source
let directives = IgnoreDirectives::parse(&source);

// Check if a rule is ignored at a line
directives.is_ignored("unwrap-call", 42)  // line-level
directives.is_file_ignored("unused-import")  // file-level
```

Comment formats supported:
- `// tree-hugger-ignore: rule1, rule2`
- `// tree-hugger-ignore` (all rules)
- `// tree-hugger-ignore-file: rule1`
- Also: `#`, `--`, `;` comment styles

## Error Handling

```rust
pub enum TreeHuggerError {
    Io { path: PathBuf, source: std::io::Error },
    UnsupportedLanguage { extension: String },
    ParseFailed { path: PathBuf, reason: String },
    MissingQuery { language: ProgrammingLanguage, kind: QueryKind },
    MissingVendorQuery { language: ProgrammingLanguage },
    QueryError { message: String },
    QueryCachePoisoned,
    GitRootNotFound { path: PathBuf },
    NoSourceFiles { path: PathBuf },
    Ignore(ignore::Error),
}
```

## JSON Output Types

For serialization (used by CLI `--json` output):

```rust
pub struct PackageSummary {
    pub root_dir: PathBuf,
    pub language: ProgrammingLanguage,
    pub files: Vec<FileSummary>,
}

pub struct FileSummary {
    pub file: PathBuf,
    pub language: ProgrammingLanguage,
    pub hash: String,
    pub symbols: Vec<SymbolInfo>,
    pub imports: Vec<ImportSymbol>,
    pub exports: Vec<SymbolInfo>,
    pub locals: Vec<SymbolInfo>,
    pub lint: Vec<LintDiagnostic>,
    pub syntax: Vec<SyntaxDiagnostic>,
}
```
