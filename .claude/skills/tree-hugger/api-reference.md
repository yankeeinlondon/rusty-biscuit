# Tree Hugger API Reference

## Core Types

### TreeFile

The primary API for file-level operations.

```rust
use tree_hugger::TreeFile;

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

// Re-exported imports
fn reexported_symbols(&self) -> Result<Vec<ImportSymbol>, TreeHuggerError>
```

#### Staged Analysis (schema v2 records)

```rust
// Parse-pass records only
fn symbol_records(&self) -> Result<Vec<SymbolRecord>, TreeHuggerError>

// Complete index: parse, bind, semantic, docs AND diagnostics
fn symbol_index_v2(&self) -> Result<FileSymbolIndex, TreeHuggerError>

// Requested passes after their prerequisites (parse -> bind -> semantic;
// parse -> docs; parse -> diagnostics), each once; diagnostics only when
// AnalysisPass::Diagnostics is requested
fn symbol_index_with_passes(&self, passes: &[AnalysisPass]) -> Result<FileSymbolIndex, TreeHuggerError>

// Selected kinds and capabilities through the same planner
fn query(&self, query: &SymbolQuery) -> Result<SymbolQueryResult, TreeHuggerError>
```

`completed_passes` lists only passes that ran: no `Diagnostics` entry means
diagnostics were not computed, not that the file is clean.

### SourceFile

Bytes read once, hashed, parsed only on demand; `hash()` always describes the
text `parse()` parses (the file is never reread). `TreeFile::new` /
`with_language` are wrappers over it and keep their error contract (`Io` for
unreadable or non-UTF-8 files, `UnsupportedLanguage`, `ParseFailed`).
`path()` is the file's `file_identity` (absolute, `.`/`..` and symlinks
resolved via `canonicalize_simplified`), not the spelling read: every
spelling of one file has the same path, records, IDs, cache key and entry,
and a symlink is its target. Detection reads the identity; `Io` and
`UnsupportedLanguage` name the spelling given. `scanner::collect_files` and
`scanner::collect_selected_files` (plus caller-resolved explicit files, listed
whatever `.gitignore`/fixture exclusions say, still subject to
`excluded_files`; with no inputs only they are listed) return identities, and `hug` canonicalizes its working directory so roots
match (Windows 8.3 short names). `tree-hugger/docs/cache-design.md#file-identity`.

```rust
let source = SourceFile::read("src/lib.rs")?;          // or read_with_language(path, Some(lang))
source.hash(); source.language(); source.source();
source.forced_language();                               // Some(lang) only when given (a cache key input)
let source = source.experimental_semantics(true);       // carried into the parsed TreeFile
let tree_file: TreeFile = source.parse()?;              // consumes the SourceFile
let result = SourceFile::read("src/lib.rs")?.query(&query)?;
// Same result through the persistent cache (parses only when the entry lacks
// a requested kind or capability):
let cached = SymbolCache::new(PersistentCache::default_cache("project", true))
    .query(SourceFile::read("src/lib.rs")?, &query)?;   // cached.result, cached.lookup, cached.write_error
```

### SymbolQuery and Capability

```rust
let query = SymbolQuery::new()                          // every kind, Declarations only
    .kinds([SymbolKindV2::Function, SymbolKindV2::Method])
    .capabilities([Capability::Containment]);
SymbolQuery::complete_index();                          // what symbol_index_v2() runs
SymbolQuery::for_passes(&[AnalysisPass::Bind]);         // what symbol_index_with_passes runs

// Predicates and naming inputs: applied after analysis and the cache
let query = SymbolQuery::new()
    .scopes([SymbolScope::Item, SymbolScope::Member])
    .visibility(VisibilityLevel::Public)                // repeatable, exact level
    .exported(true)
    .exclude_tests(true)                                // or .tests_only(true); last wins
    .undocumented(true)                                 // has_docs == Some(false)
    .deprecated(true)
    .is_async(true)
    .returns("Result<*")                                // glob_match: whole, case-sensitive
    .param_type("&str")
    .min_params(2)
    .in_container(NamePattern::new("TreeFile"))         // repeatable
    .name(NamePattern::new("parse"))                    // repeatable, smart case
    .exclude_name(NamePattern::exact("parse_all"))      // wins over inclusions
    .scan_root("tree-hugger")                           // prefix `lib/src/x.rs#`
    .naming_context(NamingContext::new("src", "crate")); // prefix `crate::x::`
query.matches(&record);                                 // the one record filter

pub struct SymbolQueryResult {
    pub index: FileSymbolIndex,                         // selected records passing the predicates
    pub completed_capabilities: Vec<Capability>,        // canonical order
    pub selected_kinds: Option<Vec<SymbolKindV2>>,      // None = every kind
    // private: the applied prefix and predicates
}
result.name_prefix();              // prefix on every name not namespace-rooted
result.narrowed(&narrower_query);  // Some(= a fresh narrower query now) or None: analyze again
```

Predicates push down only what they read (`file/filter.rs::RecordFilter`,
`SymbolQuery::settle`): a shape predicate (`is_async`, `returns`,
`param_type`, `min_params`) intersects the kinds with Function/Method/
Constructor, test/deprecation predicates add `Flags`, `undocumented` and
`min_params` add `Metrics`; the effective kinds/capabilities are what the
planner and cache see, whatever the builder order. `run_query`,
`coverage::project` and `SourceFile::analyze` return **unfinished** results
(in-file names, no predicate); `SymbolQueryResult::finish` (called by
`TreeFile::query`, `SymbolCache::query` and `narrowed`) applies the prefix
once, then the predicates. The cache stores only unfinished indexes, so an
entry never depends on a predicate or scan root. A filtered result narrows
only to the same predicates under the same prefix; an unfiltered one is
re-prefixed (`without_prefix`). `NamePattern` (`file/pattern.rs`): smart case,
substring unless `*` (whole-string glob) or exact (`exact`/trailing `!`, `*`
literal); qualified when it holds `#` or the language's separator, then it
matches the displayed `qualified_name` (anchored at the file prefix). A
qualified `in_container` pattern matches the record's qualified name minus
its own name. `portable_relative` (`file/naming.rs`) is the one path
projection, shared by the prefix and the CLI.

| Capability | Implies | Work |
|---|---|---|
| `Declarations` | (always on) | parse pass: records, cheap metadata, imports, exports |
| `Containment` | Declarations | parent/container/members only, no reference query |
| `References` | Containment | full bind pass |
| `Semantics` | References | semantic pass |
| `StructuredDocs` | Declarations | docs pass |
| `Diagnostics` | Declarations | diagnostics pass |
| `Flags` | Declarations | `visibility.is_test` / `is_deprecated` while records are built (`extract/flags.rs`, `attributes.rs`) |
| `Metrics` | Declarations | `metrics` facet while records are built (`extract/metrics.rs`) |

Flags and metrics are filled in `TreeFile::records_from` (`RecordFacts`) for
wanted kinds only; without the capability the flags are `false` and the facet
absent, and `coverage::project` strips them. Both need the syntax tree, so a
cache widening by either parses (`WidenedCapabilities { parsed: true }`).
`symbols_query::flags_and_metrics_run_with_the_declarations_only` pins that
neither runs another pass.

Unselected kinds are still extracted light (identity, spans, container) and
bound, then dropped; their metadata is skipped. Selecting a type kind keeps
Field/Property/Constant metadata (`align_field_metadata` needs it). A narrow
query's records equal the same records of a full query with the same
capabilities (`symbols_query::narrow_queries_equal_the_filtered_records_of_a_full_query`).

### Caches

`cache::SymbolCache` (persistent, one entry per file, computed key) is the
only cache the CLI uses; see `tree-hugger/docs/cache-design.md`.
`InProcessCache`, `InMemorySymbolCache`, `SymbolSnapshot` and
`AnalyzerFingerprint` are still exported for library callers but are off the
CLI path.

`semantics.is_recursive` is `Some(TriState::Unknown)` whenever the semantic
pass runs and `None` when it was not requested (v1 `SymbolInfo` has no
recursion field, so nothing to map).

### Work Counters

`work_counters()` returns process-wide atomic counts (`diagnostics_runs`,
`parser_constructions`, `definition_captures`); diff two snapshots with
`WorkCounters::since`. Tests rely on nextest's process-per-test isolation.

#### Diagnostics

```rust
// Pattern + semantic lint rules. Infallible, best-effort: a failing analysis
// stage contributes nothing rather than aborting the others.
fn lint_diagnostics(&self) -> Vec<LintDiagnostic>

// Fallible variant: surfaces query-compile/extraction failures instead of
// hiding them behind an empty list. Use when "clean file" must be
// distinguished from "analyzer could not run".
fn try_lint_diagnostics(&self) -> Result<Vec<LintDiagnostic>, TreeHuggerError>

// Syntax/parse errors
fn syntax_diagnostics(&self) -> Vec<SyntaxDiagnostic>

// Unified diagnostic format (lint + syntax)
fn diagnostics(&self) -> Vec<Diagnostic>

// Fallible unified diagnostics (used by the v2 analysis pipeline and CLI)
fn try_diagnostics(&self) -> Result<Vec<Diagnostic>, TreeHuggerError>

// Unreachable code blocks after terminal statements
fn dead_code(&self) -> Vec<CodeBlock>
```

### TreePackage

Package-level operations across multiple files.

```rust
use tree_hugger::{TreePackage, TreePackageConfig};

// Create with defaults (auto-detect git root)
let package = TreePackage::new(".")?;

// Create with config
let config = TreePackageConfig {
    language: Some(ProgrammingLanguage::Rust),
    ignores: vec!["**/target/**".to_string()],
};
let package = TreePackage::with_config(".", config)?;

// Get module names (Rust only, requires &mut self)
let modules = package.modules();
```

## Symbol Types

### SymbolInfo

Main symbol representation.

```rust
pub struct SymbolInfo {
    pub name: String,
    pub kind: SymbolKind,
    pub scope: SymbolScope,                      // Item | Member | Local (record: `scope`)
    pub qualified_name: String,                  // in-file, native separators; drives the ID
    pub is_namespace_rooted: bool,               // starts at a declared package/namespace
    pub range: CodeRange,
    pub language: ProgrammingLanguage,
    pub file: PathBuf,
    pub container_name: Option<String>,
    pub container_kind: Option<SymbolKind>,
    pub doc_comment: Option<String>,
    pub visibility: Option<Visibility>,          // None = Unknown; the only home of a symbol's access level
    pub is_static: bool,                         // Member usable without an instance
    pub is_static_storage: bool,                 // Storage/linkage `static` (Rust static, C/C++); never implies is_static
    pub is_async: bool,                          // Modifier::Async on the record
    pub signature: Option<FunctionSignature>,    // For functions/methods/constructors
    pub type_metadata: Option<TypeMetadata>,     // For types/classes/enums
}
```

The v1 ↔ v2 adapter (`From<SymbolInfo> for SymbolRecord`,
`TryFrom<SymbolRecord> for SymbolInfo`) is lossless for kind, visibility,
`is_static` (`Modifier::Static`), `is_static_storage`
(`Modifier::StaticStorage`), container kind
(`IdentityFacet::container_kind`), `scope`, `is_async` (`Modifier::Async`),
`qualified_name` and `is_namespace_rooted` (an empty v1 `qualified_name`
converts as the bare name). Records also carry `visibility.is_test` and
`is_deprecated` (`Capability::Flags`; `false` = not computed without it) and
an optional `metrics` facet (`MetricsFacet { parameters, has_docs }`, each
`Option` = unsupported; absent facet = not requested; `Capability::Metrics`).
None of them exists on `SymbolInfo`, so the adapter does not carry them. The
cache refuses a stored facet or a `true` flag whose capability the entry does
not list, and a record without the facet under `metrics`. Every kind/visibility conversion is a
wildcard-free `match`: adding a variant on one side is a compile error until
the other side handles it. A converted record is never `is_exported` (export
comes from declaration syntax, set by `symbol_records()` and the bind pass),
and a function-like kind always converts back with `Some(signature)`.
Records, file indexes and analyzer fingerprints carry `SchemaVersion::V4_0`:
no timestamps in `provenance`, the container in `identity.container_name`
(formerly `module_path`).

Extraction fills the record-only facets in `symbol_records()`:
`source.declaration_text` (declaration start up to its body, trailing
whitespace trimmed; keeps `pub(...)`, setter-only modifiers, annotations),
`source.signature_text` and `identity.keyword` (both from
`file/extract/header/`, see below) and,
for members of a Rust `impl Trait for Type`, one `relations.implements` entry
named after the trait's last path segment (`id` only when exactly one trait of
that name is declared in the file). A Rust `impl` member's container is the
implemented type's last path segment without generics; its container kind is
that type's same-file kind, absent when declared elsewhere. Every other
container kind is the kind extracted for the container's own declaration.

`source.signature_text` is the display header: a contiguous part of
`declaration_text` with leading in-node annotations, initializers (first
`=`/`:=`, also a type alias's target and a macro's replacement), accessor
lists, the `=`/`=>`/`:` that introduces a body and a `;`/Lua `end`
terminator removed; an annotation between modifiers stays. It is `None` when
no contiguous part names the symbol alone (C/C++ type with an inline body,
a later name cut off by an earlier initializer). A type in a grouped Go
`type ( … )` is declared by its own spec (`Second int`).
`identity.keyword` is one anonymous grammar token from a per-language
`(language, declaration kind) → site` table (`header/keyword.rs`): a direct
child, a token inside a named wrapper (Perl `scope`, Swift
`value_binding_pattern`), a marker node (PHP `var_modifier` → `var`), or the
wrapping statement (Go `const`/`var` specs, Lua and shell `local`). Neither
is carried by the v1 adapter.

`tree_hugger::EXTRACTOR_REVISION` names the extraction rules; bump it in any
change that alters extracted records, unreleased branches included. It is stamped into
`provenance.extractor_version` (`<crate version>+rev.<revision>`) and the
persistent cache's records fingerprint, so a warm cache never serves records
produced by older rules.

### SymbolKind

Per-language kind, container, visibility and `static` rules (user-facing):
[symbol-kinds-and-visibility.md](../../../tree-hugger/docs/symbol-kinds-and-visibility.md).

```rust
pub enum SymbolKind {
    Function,     // Regular functions
    Method,       // Functions declared directly in a type/trait/impl body
    Constructor,  // Constructors / initializers
    Type,         // Structs, records, unions
    TypeAlias,    // Type aliases
    Class,        // Class definitions
    Interface,    // Interfaces, protocols
    Enum,         // Enumerations
    EnumVariant,  // Enum members / cases
    Trait,        // Traits (Rust, Scala, PHP)
    Module,       // Rust `mod`, Perl `package`, Scala `object`
    Namespace,    // Go/Java `package`, C#/C++/TS/PHP `namespace` (not unified with Module)
    Variable,     // Variables
    Parameter,    // Function parameters
    Field,        // Struct/class fields
    Property,     // Properties (C#, TS/JS class fields)
    Macro,        // Macro definitions
    Constant,     // Constants
    Unknown,      // Unrecognized
}

impl SymbolKind {
    fn is_function(&self) -> bool  // Function, Method or Constructor
    fn is_type(&self) -> bool      // Type, TypeAlias, Class, Interface, Enum, Trait
    fn is_class(&self) -> bool     // Class or Type
}
```

### FunctionSignature

```rust
pub struct FunctionSignature {
    pub parameters: Vec<ParameterInfo>,
    pub return_type: Option<String>,
    pub is_static: bool,                 // Copy of SymbolInfo::is_static
}

pub struct ParameterInfo {
    pub name: String,
    pub type_annotation: Option<String>,
    pub default_value: Option<String>,
    pub is_variadic: bool,
}

// Mirrors v2 VisibilityLevel one-to-one; Unknown is `None`.
pub enum Visibility {
    Public,
    Protected,
    Private,
    Internal,           // C# / Swift `internal`
    Package,            // Java default, Swift `package`, lowercase Go name
    Crate,              // Rust `pub(crate)`, `pub(super)`, `pub(in path)`
    ModulePrivate,      // Swift `fileprivate`, C `static` linkage, unexported JS/TS in a module file
    ProtectedInternal,  // C# `protected internal`
    PrivateProtected,   // C# `private protected`
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
    pub doc_comment: Option<String>,
    pub visibility: Option<Visibility>,
    pub is_static: bool,
}

pub struct VariantInfo {
    pub name: String,
    pub tuple_fields: Vec<String>,        // Variant(T1, T2)
    pub struct_fields: Vec<FieldInfo>,    // Variant { field: T }
    pub discriminant: Option<String>,     // `2` in `B = 2`, as written
    pub doc_comment: Option<String>,
}
```

### ImportSymbol

```rust
pub struct ImportSymbol {
    pub name: String,                      // Imported name (or alias)
    pub original_name: Option<String>,     // Original name if aliased
    pub alias: Option<String>,             // Alias if renamed
    pub range: CodeRange,                  // Symbol location
    pub statement_range: Option<CodeRange>, // Full statement range (serde skipped)
    pub language: ProgrammingLanguage,
    pub file: PathBuf,
    pub source: Option<String>,            // Module/package source
}
```

### ReferencedSymbol

```rust
pub struct ReferencedSymbol {
    pub name: String,
    pub range: CodeRange,
    pub language: ProgrammingLanguage,
    pub file: PathBuf,
    pub is_qualified: bool,      // e.g., foo.bar, Foo::bar
    pub qualifier: Option<String>, // The qualifier (foo, Foo)
}
```

## Diagnostic Types

### LintDiagnostic

```rust
pub struct LintDiagnostic {
    pub message: String,
    pub range: CodeRange,
    pub severity: DiagnosticSeverity,
    pub rule: Option<String>,    // e.g., "unwrap-call", "dbg-macro"
    pub context: Option<SourceContext>,
}
```

### SyntaxDiagnostic

```rust
pub struct SyntaxDiagnostic {
    pub message: String,
    pub range: CodeRange,
    pub severity: DiagnosticSeverity,
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
use tree_hugger::{is_builtin, ProgrammingLanguage};

// Check if identifier is a language builtin
is_builtin(ProgrammingLanguage::Rust, "Option")      // true
is_builtin(ProgrammingLanguage::Python, "print")     // true
is_builtin(ProgrammingLanguage::JavaScript, "Array") // true
```

### Dead Code Detection

```rust
use tree_hugger::{is_terminal_statement, find_dead_code_after};

// Check if a statement is terminal (never returns)
is_terminal_statement(node, ProgrammingLanguage::Rust, source)

// Find dead code sibling nodes after a terminal statement
find_dead_code_after(terminal_node, ProgrammingLanguage::Rust)
```

### Ignore Directives

```rust
use tree_hugger::IgnoreDirectives;

// Tree-sitter-based parsing (preferred)
let directives = IgnoreDirectives::parse_with_tree(&source, &tree, language);

// Line-based fallback parsing
let directives = IgnoreDirectives::parse(&source);

// Check if a diagnostic should be suppressed
directives.should_ignore(42, Some("unwrap-call"))  // specific rule at line 42
directives.should_ignore(42, None)                  // any rule at line 42
directives.has_directives()                         // true if any directives found
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
    UnsupportedLanguage { path: PathBuf },
    ParseFailed { path: PathBuf },
    MissingQuery { language: ProgrammingLanguage, kind: QueryKind },
    MissingVendorQuery { name: String },
    QueryError { language: ProgrammingLanguage, kind: QueryKind, source: tree_sitter::QueryError },
    QueryCachePoisoned,
    GitRootNotFound { path: PathBuf },
    NoSourceFiles { path: PathBuf },
    Ignore(ignore::Error),
}
```

## JSON Output Types

The declaration commands (`functions`, `types`, `symbols`, `classes`) do not
use these types: their `--json` report is a CLI-local envelope
(`cli/src/report.rs`) of schema 4.0 `SymbolRecord`s, described in
`tree-hugger/docs/symbol-reports.md`. `PackageSummary` remains the shape of
`hug imports --json` and `hug lint --json` and of library callers:

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
