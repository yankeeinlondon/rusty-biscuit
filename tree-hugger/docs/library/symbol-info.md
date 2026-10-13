Part of [Using the Tree Hugger Library](../library.md).

# The `SymbolInfo` Model

`SymbolInfo` is the simpler of the library's two symbol types: one flat
struct per declaration, returned by `TreeFile::symbols()`,
`exported_symbols()` and `local_symbols()`. It answers the common questions
(what is it, where is it, what is it inside, who can see it, what does it
take and return) without the schema record's facets. Use a
[`SymbolRecord`](../symbol-records.md) when you need a stable ID, relations,
attributes, modifiers, flags, metrics or anything from a
[`SymbolQuery`](../symbol-records/file-index.md#querying-from-the-library).

```rust
use tree_hugger::TreeFile;

let file = TreeFile::new("src/shapes.rs")?;
for symbol in file.symbols()?.iter().filter(|s| s.kind.is_function()) {
    let signature = symbol.signature.as_ref().expect("function-like symbols have one");
    println!("{} takes {} parameter(s), returns {:?}",
             symbol.qualified_name, signature.parameters.len(), signature.return_type);
}
```

## Fields

```rust
pub struct SymbolInfo {
    pub name: String,
    pub kind: SymbolKind,
    pub scope: SymbolScope,                  // Item, Member or Local
    pub qualified_name: String,              // in-file: `Point::norm`
    pub is_namespace_rooted: bool,           // starts at a declared package or namespace
    pub range: CodeRange,                    // 1-based lines and columns, plus byte offsets
    pub language: ProgrammingLanguage,
    pub file: PathBuf,                       // the file's identity (absolute)
    pub container_name: Option<String>,
    pub container_kind: Option<SymbolKind>,
    pub doc_comment: Option<String>,
    pub visibility: Option<Visibility>,      // None = unknown or not applicable
    pub is_static: bool,                     // member usable without an instance
    pub is_static_storage: bool,             // storage or linkage `static`
    pub is_async: bool,                      // declared `async`
    pub signature: Option<FunctionSignature>,
    pub type_metadata: Option<TypeMetadata>,
}
```

- **`scope`** is decided by the nearest enclosing declaration: a parameter
  and anything in a function body is `Local`, anything in a type, trait or
  `impl` body is `Member`, and the rest is `Item`.
- **`qualified_name`** joins the enclosing named declarations with the
  language's own separator. Unlike a query result, it carries no file
  prefix. See
  [Scope, Qualified Names and `async`](../symbol-kinds-and-visibility/scope-and-qualified-names.md)
  for how both are decided in each language.
- **`visibility`** is set for every kind, not only functions. It is the
  access declared or implied at the declaration itself, not the effective
  access through enclosing modules, and `None` where no access rules apply
  or the file cannot tell. It is the only place a symbol's access level is
  stored.
- **`is_static` and `is_static_storage`** are independent. A C++ `static`
  data member has `is_static`; a C `static` function or a Rust `static` item
  has `is_static_storage`.
- **`signature`** is `Some` for every `Function`, `Method` and
  `Constructor`, with an empty parameter list when there are none.
- **`type_metadata`** is filled for type-like kinds.

### `SymbolKind` helpers

`SymbolKind` has the same 19 variants as `SymbolKindV2`, and `Display`
prints them in lowercase words (`type alias`, `enum variant`).

| Method | True for |
|--------|----------|
| `is_function()` | `Function`, `Method`, `Constructor` |
| `is_type()` | `Type`, `TypeAlias`, `Class`, `Interface`, `Enum`, `Trait` |
| `is_class()` | `Class`, `Type` (kinds that can have members) |

### Visibility

```rust
pub enum Visibility {
    Public,
    Protected,
    Private,
    Internal,          // C# and Swift `internal`
    Package,           // unmarked Java member, Swift `package`, lowercase Go name
    Crate,             // Rust `pub(crate)`, `pub(super)`, `pub(in path)`
    ModulePrivate,     // C `static` linkage, Swift `fileprivate`, unexported module-level JS/TS
    ProtectedInternal, // C# `protected internal`
    PrivateProtected,  // C# `private protected`
}
```

`Visibility` matches the record's `VisibilityLevel` one to one, except that
`VisibilityLevel::Unknown` is `None` here.

### Signatures

```rust
pub struct FunctionSignature {
    pub parameters: Vec<ParameterInfo>,
    pub return_type: Option<String>,
    pub is_static: bool,                 // a copy of SymbolInfo::is_static
}

pub struct ParameterInfo {
    pub name: String,
    pub type_annotation: Option<String>,
    pub default_value: Option<String>,
    pub is_variadic: bool,
}
```

### Type metadata

```rust
pub struct TypeMetadata {
    pub fields: Vec<FieldInfo>,          // fields and properties, never methods
    pub variants: Vec<VariantInfo>,      // enum variants
    pub type_parameters: Vec<String>,    // generic parameters
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
    pub tuple_fields: Vec<String>,       // Variant(T1, T2)
    pub struct_fields: Vec<FieldInfo>,   // Variant { field: T }
    pub discriminant: Option<String>,    // `2` in `B = 2`, as written
    pub doc_comment: Option<String>,
}
```

A `FieldInfo`'s `visibility` and `is_static` are copied from the standalone
`Field`, `Property` or `Constant` symbol with the same name and container,
so the field list and the standalone symbols always agree.

## Converting between `SymbolInfo` and `SymbolRecord`

`SymbolRecord` implements `From<SymbolInfo>`, and `SymbolInfo` implements
`TryFrom<SymbolRecord>` (it fails only for a record with an empty name):

```rust
use tree_hugger::{SymbolInfo, SymbolRecord};

let record: SymbolRecord = info.clone().into();
let back: Result<SymbolInfo, &'static str> = SymbolInfo::try_from(record);
```

Both directions keep the kind, scope, qualified name, visibility, both
`static` flags, `async`, the container's kind and whether a signature is
present. Converting a record to `SymbolInfo` drops what the older model has
no field for: generic parameters and constraints, `extends`/`implements`,
attributes, flags, metrics and every modifier except `static` and `async`.
A record converted from a `SymbolInfo` is never `is_exported`, because
export status comes from export syntax, which `SymbolInfo` does not carry.

## File and package summaries

`FileSummary` and `PackageSummary` bundle the older model for JSON output.
`hug imports --json` and `hug lint --json` print this shape, with a
`schema_version` added at the top:

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

The declaration commands (`hug symbols`, `functions`, `types` and
`classes`) print `SymbolRecord`s instead; see
[Symbol Reports](../symbol-reports.md).
