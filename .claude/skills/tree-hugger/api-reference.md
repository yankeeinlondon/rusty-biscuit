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

// Whole-file reference evidence: an `Occurrences` query selecting no kinds
fn reference_evidence(&self) -> Result<ReferenceEvidence, TreeHuggerError>
fn reexports(&self) -> Result<ReexportReport, TreeHuggerError> // whole-file forwarding report
```

`referenced_symbols()` is the older name-only usage list (diagnostics read
the same captures); it carries no owner, kind or confidence.

`ImportSymbol::is_reexport` on `imported_symbols()` is `true` for an import a
`syntax`/`declared` entry of `reexports()` forwards (all languages; Rust
`pub(self) use` is not one); `reexports()` itself has the full evidence.

#### Reference Evidence (`shared/schema_v2/occurrences.rs`)

```rust
pub struct ReferenceEvidence { pub occurrences: Vec<Occurrence>, pub reference_coverage: Vec<ReferenceCoverage> }
pub struct Occurrence {
    pub owner: Option<SymbolRef>,  // nearest callable record; None = file-level/anonymous/initializer
    pub owner_span: TextSpan,      // tells ownerless regions apart
    pub target: SymbolRef,         // name as written; `id` only for Scoped -> in-file record
    pub candidates: Vec<SymbolRef>,// NameOnly only
    pub span: TextSpan,
    pub kind: OccurrenceKind,      // Call | Usage
    pub confidence: Confidence,    // Scoped | NameOnly | Unresolved
}
pub struct ReferenceCoverage { pub owner: Option<SymbolRef>, pub span: TextSpan, pub state: CoverageState, pub reasons: Vec<LimitReason> }
// CoverageState: Complete | Limited
// LimitReason: UnsupportedLanguage, UnsupportedBindingForm, MissingCallCapture,
//              IndirectDispatch, UnexpandedMacro, OwnershipUnknown, ParseError, EmptyReferenceQuery
```

The same lists sit on `FileSymbolIndex::{occurrences, reference_coverage}`
under `Capability::Occurrences` (empty = not computed). Contract:
`tree-hugger/docs/references.md`; language tables:
`tree-hugger/docs/references/language-support.md`.

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
match (Windows 8.3 short names). `tree-hugger/docs/cache-design/entry-format.md#file-identity`.

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
// Both also come in a `_retaining` form that gives the SourceFile back
// (its text moved through any parse, never copied), for a caller that keeps
// the exact bytes only when the result calls for them (`hug refs`):
let (result, source) = SourceFile::read("src/lib.rs")?.query_retaining(&query)?;
let (cached, source) = cache.query_retaining(source, &query)?;
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
`coverage::project` and `SourceFile::analyze_retaining` return **unfinished** results
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
projection, shared by the prefix and the CLI; it replaces names that are
not valid Unicode, and `lossless_portable_relative` gives the same text or
`None` when it did (the ctags writer's navigation check).

| Capability | Implies | Work |
|---|---|---|
| `Declarations` | (always on) | parse pass: records, cheap metadata, imports, exports |
| `Containment` | Declarations | parent/container/members only, no reference query |
| `References` | Containment | full bind pass: resolves occurrences, derives the edges, drops the evidence |
| `Occurrences` | References | keeps `occurrences` + `reference_coverage` (whole file) in the index |
| `Semantics` | Occurrences | semantic pass (recursion read from the occurrences) |
| `StructuredDocs` | Declarations | docs pass |
| `Diagnostics` | Reexports | diagnostics pass (reads proven forwarding for the `unused-import` exemption) |
| `Flags` | Declarations | `visibility.is_test` / `is_deprecated` while records are built (`extract/flags.rs`, `attributes.rs`) |
| `Metrics` | Declarations | `metrics` facet while records are built (`extract/metrics.rs`) |
| `Reexports` | Declarations | `FileSymbolIndex::reexports` + `imports[].reexported` (`analysis/reexports.rs`); whole file, no reference captures; `None` = not computed |

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

`semantics.is_recursive` (functions, methods, constructors) is `Yes` only
for a `Scoped` call to the record's own ID, `No` only with a `Complete` own
region and no call that might be to itself (`may_call_itself`: a `NameOnly`
candidate, an `Unresolved` same-name call, an import whose original name is
the record's, a constructor's own class), else `Unknown`; `None` when not
requested (v1 `SymbolInfo` has no recursion field, so nothing to map).
`symbols_reference_summaries::RECURSION_MATRIX` is the row table to extend.

### Work Counters

`work_counters()` returns process-wide atomic counts (`diagnostics_runs`,
`parser_constructions`, `definition_captures`, `shared_context_steps`,
`relation_links`, `complexity_walks`, `reference_captures`,
`occurrence_resolutions`); diff two snapshots with `WorkCounters::since`.
`reference_captures` counts references-query runs (at most one per analysis,
shared by occurrences and diagnostics; none for declaration/containment
queries) and `occurrence_resolutions` the occurrence resolutions (one per
analysis that binds).
`relation_links` counts runs of the per-file relation linker (declaration
index build plus the Rust impl walk), one per analysis. `complexity_walks`
counts callable bodies walked for `metrics.cyclomatic`: one per measured
callable of a language with counting rules, none without `Capability::Metrics`
or for a language without rules. `shared_context_steps` counts the
derivation of, and lookups in, context a file's declarations share (file
namespace index, sibling scopes, inner `cfg(test)`, C++ access sections,
same-file traits/`static` prototypes/owners, container and export-target
lookups); it must grow linearly with declarations, repeated names included
(`symbols_shared_context`: a trait, owner or Lua table repeated per module,
namespaced and overloaded C++ functions, Scala block packages). Tests rely on nextest's process-per-test isolation.

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
an optional `metrics` facet (`MetricsFacet { parameters, has_docs, lines,
cyclomatic }`, each `Option` = unsupported, every key required (schema 4.1
added `lines`: a function, method or constructor's body lines, counted from
its `source.body_span`; schema 4.3 added `cyclomatic`: that body's
cyclomatic complexity, `None` = unavailable, never zero); absent facet = not
requested; `Capability::Metrics`).
None of them exists on `SymbolInfo`, so the adapter does not carry them;
nor does it carry `type_info.generic_params`/`constraints`,
`relations.extends`/`implements`, `attributes` or modifiers other than
`Static`/`StaticStorage`/`Async`
(`symbols_adapter::record_only_declaration_facts_are_lost_through_the_older_model`). The
cache refuses a stored facet or a `true` flag whose capability the entry does
not list, and a record without the facet under `metrics`. Every kind/visibility conversion is a
wildcard-free `match`: adding a variant on one side is a compile error until
the other side handles it. A converted record is never `is_exported` (export
comes from declaration syntax, set by `symbol_records()` and the bind pass),
and a function-like kind always converts back with `Some(signature)`.
Records, file indexes and analyzer fingerprints carry `SchemaVersion::CURRENT`
(5.1):
no timestamps in `provenance`, the container in `identity.container_name`
(formerly `module_path`).

Extraction fills the record-only facets in `symbol_records()`:
`source.declaration_text` (declaration start up to its body, trailing
whitespace trimmed; keeps `pub(...)`, setter-only modifiers, annotations),
`source.signature_text` and `identity.keyword` (both from
`file/extract/header/`, see below), `type_info.generic_params` and
`type_info.constraints` (the declaration's own written parameter list,
`extract/generics.rs`; a list whose positions overflow `u16` is
`TreeHuggerError::TooManyGenericParameters`; see
`tree-hugger/docs/symbol-records/generic-parameters.md`), `attributes`
and the written `modifiers` (`Const`, `Unsafe`, `Readonly`, `Mut`,
`Custom("abstract" | "final" | "override" | "virtual" | "sealed" |
"mutating")`; see `tree-hugger/docs/symbol-records/fields.md#modifiers`) and,
`relations.extends`/`relations.implements` (each base the declaration writes,
as written, `id` and in-file `qualified_name` only when the path names exactly
one eligible same-file declaration; a Rust trait impl adds its trait as written
to each member and to the resolved self type; see
`tree-hugger/docs/type-relations.md`; C#/Swift/Scala retain unknown categories
in `relations.unclassified_bases`, and Swift local-owner extensions add
conformance with `SymbolRef::source_span` evidence). A lookup made inside one
of several same-named owners (two Rust modules, Scala objects, C# types that
are not fragments of one type) reads only that owner's members, imports,
wildcard/unrecorded bindings, bases and Swift raw-type shadowing; reopened
namespaces and the C# `partial` fragments of one type (same category, generic
arity and enclosing owner) share one lookup, and a C# name selects only the
declarations of the generic arity it is written with, at every step (final
target, owner members, outward search, qualified and `global::` paths), so
`Base<int>` beside `Base` names `Base<T>`. A Rust `impl` member's
`identity.container_name` is the implemented type's last path segment without
generics; its container kind is the kind of the declaration the self type
resolves to by the relation resolver (`relations::impl_self_types`, called
from `definitions.rs::resolve_container_kind`, so `symbols()` agrees),
absent when it does not resolve (`impl ::ext::B`, an enclosing generic `T`,
a type declared elsewhere, a path through two same-named modules). Under
Containment its `relations.container` is the self type resolved by full written
path, or the self type as written with no ID. A Swift extension member's
`relations.container` follows the extension's conformance decision: the one
resolved owner, or the extended type as written with no ID and no parent
(external, aliased, imported or ambiguous owners). Every other container kind
is the kind extracted for the container's own declaration.

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
(`cli/src/report.rs`) of schema 5.1 `SymbolRecord`s, described in
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
