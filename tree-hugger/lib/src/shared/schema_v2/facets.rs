use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::kinds::SymbolKindV2;
use super::types::{SymbolId, TextSpan};

/// Identity metadata for a symbol.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IdentityFacet {
    pub name: String,
    pub display_name: String,
    /// The in-file qualified name: the enclosing named declarations
    /// (namespaces, packages, modules, types, `impl` targets, functions)
    /// joined with the language's own separators, ending in `name`. Never
    /// empty, and built from this file's source alone, so it does not depend
    /// on where the file is read from. A query result prefixes it unless
    /// `is_namespace_rooted` ([`SymbolQueryResult::name_prefix`](crate::SymbolQueryResult::name_prefix));
    /// cache entries and symbol IDs keep the in-file form.
    #[serde(deserialize_with = "non_empty_string")]
    pub qualified_name: String,
    /// `qualified_name` starts at a namespace or package the source declares
    /// (Java/Scala/Go `package`, C#/C++/PHP `namespace`, Perl `package`), so
    /// it is complete without a file prefix. A Rust `mod` or a type does not
    /// root a name.
    pub is_namespace_rooted: bool,
    /// Name of the declaration that directly contains this one (a type,
    /// module, namespace or function); absent at file level.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub container_name: Option<String>,
    /// Kind of the container named by `container_name`; absent when there is
    /// no container or its kind cannot be determined from the file.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub container_kind: Option<SymbolKindV2>,
    /// The keyword token that introduces the declaration (`fn`, `struct`,
    /// `def`, `const`, `my`, ...), read from the syntax tree, never from
    /// source text; absent when the construct has none (a Java method, a C
    /// variable).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keyword: Option<String>,
    pub stable_key: String,
}

/// Source location and snippets for a symbol.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SourceFacet {
    pub file_path: PathBuf,
    pub declaration_span: TextSpan,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body_span: Option<TextSpan>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name_span: Option<TextSpan>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doc_span: Option<TextSpan>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_text: Option<String>,
    /// Evidence: the declaration node's text up to its body, including
    /// initializers and the annotations the grammar parses inside the node.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub declaration_text: Option<String>,
    /// Display header: a contiguous sub-range of `declaration_text` without
    /// leading in-node annotations, initializers, a terminator or a trailing
    /// `=`, `=>` or `:`; absent when the header cannot be isolated.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body_text: Option<String>,
}

/// Visibility level across languages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VisibilityLevel {
    Public,
    Protected,
    Private,
    Internal,
    Package,
    Crate,
    ModulePrivate,
    /// C# `protected internal`
    ProtectedInternal,
    /// C# `private protected`
    PrivateProtected,
    Unknown,
}

/// Visibility and exposure metadata.
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
    /// Test-only code: a declaration in a recognized test file, with a test
    /// attribute, or inside a Rust `#[cfg(test)]` item.
    pub is_test: bool,
}

impl Default for VisibilityFacet {
    fn default() -> Self {
        Self {
            level: VisibilityLevel::Unknown,
            is_exported: false,
            is_reexported: false,
            is_default_export: false,
            is_imported: false,
            is_external: false,
            is_generated: false,
            is_synthetic: false,
            is_deprecated: false,
            is_experimental: false,
            is_test: false,
        }
    }
}

/// Supported parsed comment kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CommentKind {
    Doc,
    Line,
    Block,
}

/// How a comment attaches to a symbol.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CommentAttachment {
    Leading,
    Trailing,
    Inline,
    Detached,
}

/// Raw and cleaned comment attached to a symbol.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttachedComment {
    pub kind: CommentKind,
    pub attachment: CommentAttachment,
    pub span: TextSpan,
    pub raw_text: String,
    pub cleaned_text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_distance: Option<u16>,
}

/// Parsed doc parameter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocParam {
    pub name: String,
    pub description: String,
}

/// Parsed free-form doc tag.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocTag {
    pub name: String,
    pub value: String,
}

/// Parsed docs payload.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ParsedDocs {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub params: Vec<DocParam>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub returns: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub tags: Vec<DocTag>,
}

/// Documentation facet preserving raw and parsed forms.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DocsFacet {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_doc: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub comments: Vec<AttachedComment>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parsed: Option<ParsedDocs>,
}

/// Structured attribute on a symbol.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attribute {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}

/// Modifier information kept language-agnostic.
///
/// The derived `Ord` is the serialized order of a record's `modifiers`
/// (variant declaration order, then `Custom` values lexically); reordering
/// the variants changes output. See [`normalize_modifiers`].
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum Modifier {
    /// Member usable without an instance.
    Static,
    /// Storage or linkage `static` (Rust `static` items, C/C++ file-scope
    /// and function-local `static`); never implies `Static`.
    StaticStorage,
    Async,
    Const,
    Readonly,
    Unsafe,
    Mut,
    Custom(String),
}

/// Puts a declaration's modifiers in their one serialized order and drops
/// repeats, so extractors may push them in any order and more than once.
pub(crate) fn normalize_modifiers(modifiers: &mut Vec<Modifier>) {
    modifiers.sort();
    modifiers.dedup();
}

/// Per-symbol measurements, present on a record only when requested. Inside
/// the facet each metric is required: `None` (serialized `null`) means the
/// metric is unsupported for this declaration, never "not computed".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MetricsFacet {
    /// Declared parameters, excluding a separate receiver; a variadic
    /// parameter counts once.
    #[serde(deserialize_with = "nullable")]
    pub parameters: Option<u32>,
    /// Non-empty attached documentation.
    #[serde(deserialize_with = "nullable")]
    pub has_docs: Option<bool>,
    /// Physical lines intersecting a Function, Method or Constructor's
    /// `source.body_span`; `Some(0)` for an empty span, `None` for other kinds
    /// and for a callable without a body.
    #[serde(deserialize_with = "nullable")]
    pub lines: Option<u32>,
    /// Cyclomatic complexity of a Function, Method or Constructor's body
    /// (the one `lines` counts): 1 plus one per decision the language's
    /// reviewed rules count, nested callables and types excluded. `None`
    /// means unavailable for this declaration: another kind, no body, a
    /// language without reviewed rules, a syntax error in the counted body,
    /// or a total beyond `u32`. Never read `None` as zero.
    #[serde(deserialize_with = "nullable")]
    pub cyclomatic: Option<u32>,
}

/// A required value that may be `null`; unlike a plain `Option` field, an
/// absent key is an error rather than `None`.
fn nullable<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer)
}

/// A present facet; `null` is an error rather than `None`. Pair with
/// `#[serde(default)]` so an absent key reads as `None`.
pub(crate) fn present<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    T::deserialize(deserializer).map(Some)
}

fn non_empty_string<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<String, D::Error> {
    let value = String::deserialize(deserializer)?;
    if value.is_empty() {
        return Err(serde::de::Error::invalid_length(0, &"a non-empty string"));
    }
    Ok(value)
}

/// Different projections for type strings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TypeViewKind {
    Declared,
    Canonical,
    Resolved,
    Expanded,
}

/// A concrete type view.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeView {
    pub kind: TypeViewKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ast: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub referenced_symbol_ids: Vec<SymbolId>,
}

/// Generic parameter kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GenericParamKind {
    Type,
    Lifetime,
    Const,
}

/// Generic parameter details.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenericParam {
    pub name: String,
    pub kind: GenericParamKind,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub constraints: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variance: Option<String>,
    pub position: u16,
}

/// Type facet shared across symbol kinds.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TypeFacet {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primary_type: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub views: Vec<TypeView>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub generic_params: Vec<GenericParam>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub constraints: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nullability: Option<String>,
}

/// Reference to another symbol.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SymbolRef {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<SymbolId>,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub qualified_name: Option<String>,
    /// Written relation location, when needed to distinguish a declaration's
    /// header from conformance added by a Swift extension in the same file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_span: Option<TextSpan>,
}

/// Cross-symbol relation facet.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RelationFacet {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<SymbolRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub container: Option<SymbolRef>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub members: Vec<SymbolRef>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub extends: Vec<SymbolRef>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub implements: Vec<SymbolRef>,
    /// Written bases whose inheritance or conformance category is unknown.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub unclassified_bases: Vec<SymbolRef>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub overrides: Vec<SymbolRef>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub references: Vec<SymbolRef>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub referenced_by: Vec<SymbolRef>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub dependencies: Vec<SymbolRef>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub related: Vec<SymbolRef>,
}

/// Three-state inference for semantic fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TriState {
    Yes,
    No,
    Unknown,
}

/// Semantic facet.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticFacet {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_pure: Option<TriState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_side_effects: Option<TriState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub may_throw: Option<TriState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub may_panic: Option<TriState>,
    /// Direct recursion: `Yes` only for a `Scoped` call to the callable's
    /// own ID, `No` only with complete call coverage and no call that might
    /// be to itself, otherwise `Unknown`; `None` when semantics were not
    /// computed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_recursive: Option<TriState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mutates_self: Option<TriState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mutates_arguments: Option<TriState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reads_global_state: Option<TriState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub writes_global_state: Option<TriState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allocates: Option<TriState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocks: Option<TriState>,
}

/// Extractor provenance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvenanceFacet {
    pub extractor: String,
    pub extractor_version: String,
    pub parse_pass: String,
}
