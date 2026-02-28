use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::types::{SymbolId, TextSpan};

/// Identity metadata for a symbol.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IdentityFacet {
    pub name: String,
    pub display_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub qualified_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub module_path: Option<String>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub declaration_text: Option<String>,
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
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum Modifier {
    Static,
    Async,
    Const,
    Readonly,
    Unsafe,
    Mut,
    Custom(String),
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolRef {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<SymbolId>,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub qualified_name: Option<String>,
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
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SemanticFacet {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_pure: Option<TriState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_side_effects: Option<TriState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub may_throw: Option<TriState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub may_panic: Option<TriState>,
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
    pub created_at_epoch_ms: u64,
    pub updated_at_epoch_ms: u64,
}
