# Symbol Schema Design for Tree Hugger

## Status

Draft design for a major refactor.

## Context

`tree-hugger` currently extracts symbols, imports/exports, and diagnostics from multiple languages via Tree-sitter. The existing symbol model is useful but too narrow for high-fidelity static analysis, API diffing, semantic search, and richer documentation workflows.

This design proposes a new Rust schema that treats symbol metadata as composable facets with kind-specific payloads. It is intentionally broader than the current `SymbolInfo`/`TypeMetadata` model and is designed for staged adoption.

## Goals

1. Preserve syntax fidelity from Tree-sitter.
2. Support semantic enrichment in later passes without schema churn.
3. Model documentation as structured data, not only plain text.
4. Represent type information in multiple views (declared, canonical, resolved, expanded).
5. Scale across all supported languages without making Rust-specific assumptions mandatory.
6. Provide stable IDs and relationship edges for graph-based analysis.
7. Keep JSON serialization deterministic and backward-compatible via schema versioning.

## Non-Goals

1. Full type-checker parity for every language in v1.
2. Complete inter-file semantic resolution in the parse pass.
3. Perfect purity/side-effect inference in v1.

## Design Principles

1. Facet composition over giant optional structs.
2. Loss-minimizing capture: keep raw text and structured forms.
3. Incremental enrichment: parse -> bind -> infer -> doc parse.
4. Language portability with language-specific extension slots.
5. Stable public schema with explicit version field.

## High-Level Model

Each symbol record has:

1. Common facets for all symbols.
2. A discriminated `kind_data` payload with symbol-kind-specific fields.
3. Optional enrichment facets populated by later analysis passes.

```text
SymbolRecord
├── identity
├── source
├── visibility
├── docs
├── attributes
├── modifiers
├── type_info
├── relations
├── semantics
├── provenance
└── kind_data (Function | Type | Enum | Module | ...)
```

## Proposed Rust Types

### Core Primitives

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SymbolId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TypeId(pub String);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchemaVersion {
    pub major: u16,
    pub minor: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextPoint {
    pub line: u32,   // 1-based
    pub column: u32, // 1-based UTF-8 column
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextSpan {
    pub start: TextPoint,
    pub end: TextPoint,
    pub start_byte: u32,
    pub end_byte: u32,
}
```

### Top-Level Symbol Record

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolRecord {
    pub schema_version: SchemaVersion,
    pub id: SymbolId,
    pub language: ProgrammingLanguage,
    pub kind: SymbolKindV2,
    pub identity: IdentityFacet,
    pub source: SourceFacet,
    pub visibility: VisibilityFacet,
    pub docs: DocsFacet,
    pub attributes: Vec<Attribute>,
    pub modifiers: Vec<Modifier>,
    pub type_info: TypeFacet,
    pub relations: RelationFacet,
    pub semantics: SemanticFacet,
    pub provenance: ProvenanceFacet,
    pub kind_data: SymbolKindData,
    pub extensions: std::collections::BTreeMap<String, serde_json::Value>,
}
```

### Symbol Kind and Payload

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SymbolKindV2 {
    Function,
    Method,
    Constructor,
    Type,
    Class,
    Interface,
    Enum,
    Trait,
    Module,
    Namespace,
    Field,
    Property,
    Variable,
    Constant,
    Parameter,
    TypeAlias,
    EnumVariant,
    Macro,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind_data_type", content = "data")]
pub enum SymbolKindData {
    Function(FunctionData),
    Type(TypeData),
    TypeAlias(TypeAliasData),
    Enum(EnumData),
    Field(FieldData),
    Module(ModuleData),
    Variable(VariableData),
    Parameter(ParameterData),
    Unknown,
}
```

## Facets

### Identity Facet

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityFacet {
    pub name: String,
    pub display_name: String,
    pub qualified_name: Option<String>,
    pub module_path: Option<String>,
    pub stable_key: String, // language + file + kind + lexical path
}
```

`stable_key` is deterministic and used to produce `SymbolId`.

### Source Facet

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceFacet {
    pub file_path: std::path::PathBuf,
    pub declaration_span: TextSpan,
    pub body_span: Option<TextSpan>,
    pub name_span: Option<TextSpan>,
    pub doc_span: Option<TextSpan>,
    pub source_text: Option<String>,
    pub declaration_text: Option<String>,
    pub signature_text: Option<String>,
    pub body_text: Option<String>,
}
```

`source_text` and `body_text` should be feature-gated or size-limited for large-scale indexing.

### Visibility Facet

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VisibilityLevel {
    Public,
    Protected,
    Private,
    Internal,
    Package,
    Crate,
    ModulePrivate,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisibilityFacet {
    pub level: VisibilityLevel,
    pub is_exported: bool,
    pub is_reexported: bool,
    pub is_default_export: bool,
    pub is_imported: bool,
    pub is_external: bool,
    pub is_generated: bool,
    pub is_synthetic: bool,
    pub is_deprecated: bool,
    pub is_experimental: bool,
}
```

### Documentation Facet

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CommentKind {
    Doc,
    Line,
    Block,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CommentAttachment {
    Leading,
    Trailing,
    Inline,
    Detached,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttachedComment {
    pub kind: CommentKind,
    pub attachment: CommentAttachment,
    pub span: TextSpan,
    pub raw_text: String,
    pub cleaned_text: String,
    pub line_distance: Option<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocParam {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocTag {
    pub name: String, // param, returns, throws, deprecated, see, example, ...
    pub value: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ParsedDocs {
    pub summary: Option<String>,
    pub description: Option<String>,
    pub params: Vec<DocParam>,
    pub returns: Option<String>,
    pub throws: Vec<String>,
    pub examples: Vec<String>,
    pub remarks: Option<String>,
    pub deprecated_message: Option<String>,
    pub since: Option<String>,
    pub tags: Vec<DocTag>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DocsFacet {
    pub comments: Vec<AttachedComment>,
    pub raw_doc: Option<String>,
    pub parsed: ParsedDocs,
}
```

This preserves raw and parsed docs to avoid information loss.

### Attributes and Modifiers

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attribute {
    pub name: String,
    pub arguments: Vec<String>,
    pub raw_text: Option<String>,
    pub span: Option<TextSpan>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Modifier {
    Async,
    Static,
    Const,
    Abstract,
    Final,
    Sealed,
    Open,
    Virtual,
    Override,
    Unsafe,
    Extern,
    Readonly,
    Mutable,
    Generator,
}
```

### Type Facet

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TypeViewKind {
    Declared,
    Canonical,
    Resolved,
    Expanded,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeView {
    pub kind: TypeViewKind,
    pub text: Option<String>,
    pub ast: Option<serde_json::Value>,
    pub referenced_symbol_ids: Vec<SymbolId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GenericParamKind {
    Type,
    Lifetime,
    Const,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenericParam {
    pub name: String,
    pub kind: GenericParamKind,
    pub constraints: Vec<String>,
    pub default: Option<String>,
    pub variance: Option<String>,
    pub position: u16,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TypeFacet {
    pub primary_type: Option<String>,
    pub views: Vec<TypeView>,
    pub generic_params: Vec<GenericParam>,
    pub constraints: Vec<String>,
    pub nullability: Option<String>,
}
```

`ast` can begin as lightweight JSON from Tree-sitter nodes and evolve later to a strongly typed internal AST.

### Relation Facet

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolRef {
    pub id: Option<SymbolId>,   // unresolved references keep this None
    pub name: String,
    pub qualified_name: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RelationFacet {
    pub parent: Option<SymbolRef>,
    pub container: Option<SymbolRef>,
    pub members: Vec<SymbolRef>,
    pub extends: Vec<SymbolRef>,
    pub implements: Vec<SymbolRef>,
    pub overrides: Vec<SymbolRef>,
    pub references: Vec<SymbolRef>,
    pub referenced_by: Vec<SymbolRef>,
    pub dependencies: Vec<SymbolRef>,
    pub related: Vec<SymbolRef>,
}
```

### Semantic Facet

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TriState {
    Yes,
    No,
    Unknown,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SemanticFacet {
    pub is_pure: Option<TriState>,
    pub has_side_effects: Option<TriState>,
    pub may_throw: Option<TriState>,
    pub may_panic: Option<TriState>,
    pub is_recursive: Option<TriState>,
    pub mutates_self: Option<TriState>,
    pub mutates_arguments: Option<TriState>,
    pub reads_global_state: Option<TriState>,
    pub writes_global_state: Option<TriState>,
    pub allocates: Option<TriState>,
    pub blocks: Option<TriState>,
}
```

`Option<TriState>` intentionally distinguishes "not analyzed" from "analyzed unknown".

### Provenance Facet

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvenanceFacet {
    pub extractor: String, // e.g. "tree-sitter"
    pub extractor_version: String,
    pub parse_pass: String, // parse, bind, semantic, docs
    pub created_at_epoch_ms: u64,
    pub updated_at_epoch_ms: u64,
}
```

## Kind-Specific Payloads

### Function and Method Data

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReceiverKind {
    Value,
    Ref,
    MutRef,
    This,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceiverInfo {
    pub name: String,
    pub kind: ReceiverKind,
    pub type_text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterData {
    pub name: String,
    pub position: u16,
    pub label: Option<String>,
    pub type_text: Option<String>,
    pub type_ast: Option<serde_json::Value>,
    pub default_value: Option<String>,
    pub is_optional: bool,
    pub is_variadic: bool,
    pub is_keyword_only: bool,
    pub is_positional_only: bool,
    pub mutability: Option<String>,
    pub ownership_mode: Option<String>,
    pub docs: Option<String>,
    pub attributes: Vec<Attribute>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReturnData {
    pub type_text: Option<String>,
    pub type_ast: Option<serde_json::Value>,
    pub error_type: Option<String>,
    pub yields_type: Option<String>,
    pub never_returns: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionData {
    pub receiver: Option<ReceiverInfo>,
    pub parameters: Vec<ParameterData>,
    pub return_info: ReturnData,
    pub abi: Option<String>,
    pub calling_convention: Option<String>,
    pub is_variadic: bool,
    pub arity: u16,
    pub overload_set: Vec<SymbolRef>,
}
```

### Type/Class/Interface/Trait Data

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldData {
    pub name: String,
    pub qualified_name: Option<String>,
    pub type_text: Option<String>,
    pub visibility: Option<VisibilityLevel>,
    pub is_static: bool,
    pub is_readonly: bool,
    pub is_mutable: bool,
    pub is_optional: bool,
    pub default_value: Option<String>,
    pub docs: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeData {
    pub members: Vec<SymbolRef>,  // methods, fields, nested types
    pub fields: Vec<FieldData>,   // denormalized convenience
    pub constructors: Vec<SymbolRef>,
    pub associated_types: Vec<String>,
    pub associated_constants: Vec<String>,
    pub is_abstract: bool,
    pub is_final: bool,
    pub is_sealed: bool,
    pub is_open: bool,
    pub is_partial: bool,
}
```

### Enum and Type Alias Data

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnumVariantData {
    pub name: String,
    pub discriminant: Option<String>,
    pub tuple_fields: Vec<String>,
    pub struct_fields: Vec<FieldData>,
    pub docs: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnumData {
    pub backing_type: Option<String>,
    pub variants: Vec<EnumVariantData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeAliasData {
    pub aliased_type_text: Option<String>,
    pub aliased_type_ast: Option<serde_json::Value>,
    pub is_recursive: bool,
}
```

### Module and Variable Data

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleData {
    pub members: Vec<SymbolRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariableData {
    pub type_text: Option<String>,
    pub initializer_text: Option<String>,
    pub is_const: bool,
}
```

## Package-Level Container

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSymbolIndex {
    pub file: std::path::PathBuf,
    pub language: ProgrammingLanguage,
    pub file_hash: String,
    pub symbols: Vec<SymbolRecord>,
    pub imports: Vec<ImportRecord>,
    pub exports: Vec<ExportRecord>,
    pub diagnostics: Vec<Diagnostic>,
}
```

`ImportRecord` and `ExportRecord` should also use `SymbolRef` to align with `RelationFacet`.

## ID Strategy

1. Compute `stable_key` as: `<language>::<normalized_file>::<kind>::<qualified_name_or_path>::<declaration_span_start_byte>`.
2. Hash `stable_key` (BLAKE3 recommended) and encode as lowercase hex.
3. Keep both hashed `id` and unhashed `stable_key` for debugging and diff tooling.

## Extraction Pipeline Contract

The schema is designed for four passes:

1. Parse pass (Tree-sitter): identity, source, modifiers, declared signatures, raw comments.
2. Binding pass: symbol linking, parent/container, references, import/export edges.
3. Semantic pass: inferred flags (purity, mutation, throws/panics, recursion).
4. Docs pass: parse doc comments into structured `ParsedDocs`.

Each pass updates `provenance.parse_pass` and timestamps.

## Serialization and Compatibility

1. Derive `Serialize`/`Deserialize` on all public schema structs.
2. Use `#[serde(default)]` for forward-compatible additions.
3. Use `#[serde(skip_serializing_if = "...")]` for optional large fields (`source_text`, `body_text`, AST blobs).
4. Version schema with `SchemaVersion` and bump:
   1. `minor` for additive fields.
   2. `major` for breaking changes (renames, semantic changes, enum variant removals).

## Migration from Current Model

### Mapping from existing fields

1. `SymbolInfo.name` -> `identity.name`.
2. `SymbolInfo.kind` -> `kind`.
3. `SymbolInfo.range` -> `source.declaration_span`.
4. `SymbolInfo.doc_comment` -> `docs.raw_doc`.
5. `FunctionSignature` -> `kind_data: FunctionData` (+ `modifiers`).
6. `TypeMetadata` -> `kind_data: TypeData` or `EnumData`.

### Recommended migration phases

1. Introduce `SymbolRecord` and emit it behind a feature flag while keeping existing JSON output.
2. Add adapters:
   1. `impl From<SymbolInfo> for SymbolRecord`.
   2. `impl TryFrom<SymbolRecord> for SymbolInfo` (best effort).
3. Update CLI `--json` to opt into `schema_version = 2.x`.
4. Remove v1 schema after one release cycle.

## Testing Strategy for the New Schema

1. Fixture-based cross-language tests for all `SymbolKindV2` variants that can be emitted today.
2. Golden JSON tests for `--json` output stability.
3. Round-trip serde tests for every schema type.
4. Migration tests from v1 `SymbolInfo` to v2 `SymbolRecord`.
5. Precision tests for doc attachment modes (leading/trailing/inline/detached).
6. Type-view tests ensuring declared/canonical/resolved/expanded fields can coexist without overwrite.

## Open Decisions

1. Whether `type_ast` should remain `serde_json::Value` long-term or move to a typed internal AST enum.
2. Size controls for raw snippets in very large files (hard cap vs opt-in).
3. Whether `SymbolRef.name` should include original spelling and normalized spelling separately.
4. Whether relation edges should be kept only per symbol or also emitted as a package-level edge table.

## Recommended Initial Cut (v2.0)

Ship first with:

1. `SymbolRecord` core facets.
2. `FunctionData`, `TypeData`, `EnumData`, `FieldData`.
3. `DocsFacet` raw + parsed summary/params/returns.
4. `TypeFacet` with declared and canonical views.
5. `RelationFacet` parent/container/members/references.

Defer to v2.1+:

1. Fully resolved and expanded type views.
2. Rich semantic inference fields for all languages.
3. Package-level global relationship table.

