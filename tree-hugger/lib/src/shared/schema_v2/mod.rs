use std::collections::BTreeMap;
use std::path::Path;

use biscuit_hash::blake3_hash;
use serde::{Deserialize, Serialize};

use super::{
    CodeRange, Diagnostic, FieldInfo, FunctionSignature, ParameterInfo, ProgrammingLanguage,
    SymbolInfo, SymbolKind, TypeMetadata, VariantInfo, Visibility,
};

mod facets;
mod kinds;
mod occurrences;
mod reexports;
mod types;

pub use facets::*;
pub use kinds::*;
pub use occurrences::*;
pub use reexports::*;
pub use types::*;

/// Top-level v2 symbol record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolRecord {
    pub schema_version: SchemaVersion,
    pub id: SymbolId,
    pub language: ProgrammingLanguage,
    pub kind: SymbolKindV2,
    pub scope: SymbolScope,
    pub identity: IdentityFacet,
    pub source: SourceFacet,
    pub visibility: VisibilityFacet,
    pub docs: DocsFacet,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attributes: Vec<Attribute>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub modifiers: Vec<Modifier>,
    #[serde(default)]
    pub type_info: TypeFacet,
    #[serde(default)]
    pub relations: RelationFacet,
    #[serde(default)]
    pub semantics: SemanticFacet,
    /// Present only when metrics were requested.
    #[serde(
        default,
        deserialize_with = "facets::present",
        skip_serializing_if = "Option::is_none"
    )]
    pub metrics: Option<MetricsFacet>,
    pub provenance: ProvenanceFacet,
    pub kind_data: SymbolKindData,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub extensions: BTreeMap<String, serde_json::Value>,
}

/// Import entry aligned to v2 relations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportRecord {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alias: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    pub span: TextSpan,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub statement_span: Option<TextSpan>,
    pub symbol: SymbolRef,
    /// Whether a `syntax` or `declared` re-export entry forwards this
    /// import. Meaningful only when `FileSymbolIndex::reexports` was
    /// computed; `false` for every record otherwise.
    pub reexported: bool,
}

/// Export entry aligned to v2 relations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportRecord {
    pub name: String,
    pub span: TextSpan,
    pub symbol: SymbolRef,
}

/// Per-file v2 analysis output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSymbolIndex {
    pub schema_version: SchemaVersion,
    pub file: std::path::PathBuf,
    pub language: ProgrammingLanguage,
    pub file_hash: String,
    #[serde(default)]
    pub completed_passes: Vec<AnalysisPass>,
    pub symbols: Vec<SymbolRecord>,
    #[serde(default)]
    pub imports: Vec<ImportRecord>,
    #[serde(default)]
    pub exports: Vec<ExportRecord>,
    #[serde(default)]
    pub diagnostics: Vec<Diagnostic>,
    /// Every reference in the file, whatever kinds were selected; empty
    /// unless `Capability::Occurrences` was computed.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub occurrences: Vec<Occurrence>,
    /// The coverage of each owning region of the file, beside
    /// `occurrences` (empty exactly when they were not computed).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reference_coverage: Vec<ReferenceCoverage>,
    /// The file's re-export report; `None` exactly when
    /// `Capability::Reexports` was not computed, which is not the same as a
    /// report with no entries.
    #[serde(default, deserialize_with = "facets::present", skip_serializing_if = "Option::is_none")]
    pub reexports: Option<ReexportReport>,
}

/// Deterministic stable symbol key used to derive `SymbolId`.
pub fn stable_symbol_key(
    language: ProgrammingLanguage,
    file: &Path,
    kind: SymbolKindV2,
    qualified_or_name: &str,
    sibling_ordinal: u32,
) -> String {
    stable_key_with_path(language, &stable_key_path(file), kind, qualified_or_name, sibling_ordinal)
}

/// The spelling of `file` a [`stable_symbol_key`] holds.
pub(crate) fn stable_key_path(file: &Path) -> String {
    file.to_string_lossy().replace('\\', "/").trim().to_string()
}

/// [`stable_symbol_key`] for a file whose [`stable_key_path`] is `file_part`,
/// so a file's records spell their path once.
pub(crate) fn stable_key_with_path(
    language: ProgrammingLanguage,
    file_part: &str,
    kind: SymbolKindV2,
    qualified_or_name: &str,
    sibling_ordinal: u32,
) -> String {
    format!(
        "{}::{}::{:?}::{}::{}",
        language.query_name(),
        file_part,
        kind,
        qualified_or_name,
        sibling_ordinal
    )
}

/// `provenance.extractor_version` of every record: this crate's version
/// and [`crate::file::EXTRACTOR_REVISION`].
fn extractor_version() -> &'static str {
    static VERSION: std::sync::LazyLock<String> = std::sync::LazyLock::new(|| {
        format!("{}+rev.{}", env!("CARGO_PKG_VERSION"), crate::file::EXTRACTOR_REVISION)
    });
    &VERSION
}

/// Converts a stable symbol key to a deterministic `SymbolId`.
pub fn symbol_id_from_stable_key(stable_key: &str) -> SymbolId {
    SymbolId(blake3_hash(stable_key))
}

// Every conversion below is a wildcard-free `match`, so a variant added on
// one side fails to compile until the other side handles it.
impl From<SymbolKind> for SymbolKindV2 {
    fn from(value: SymbolKind) -> Self {
        match value {
            SymbolKind::Function => Self::Function,
            SymbolKind::Method => Self::Method,
            SymbolKind::Constructor => Self::Constructor,
            SymbolKind::Type => Self::Type,
            SymbolKind::TypeAlias => Self::TypeAlias,
            SymbolKind::Class => Self::Class,
            SymbolKind::Interface => Self::Interface,
            SymbolKind::Enum => Self::Enum,
            SymbolKind::EnumVariant => Self::EnumVariant,
            SymbolKind::Trait => Self::Trait,
            SymbolKind::Module => Self::Module,
            SymbolKind::Namespace => Self::Namespace,
            SymbolKind::Variable => Self::Variable,
            SymbolKind::Parameter => Self::Parameter,
            SymbolKind::Field => Self::Field,
            SymbolKind::Property => Self::Property,
            SymbolKind::Macro => Self::Macro,
            SymbolKind::Constant => Self::Constant,
            SymbolKind::Unknown => Self::Unknown,
        }
    }
}

impl From<SymbolKindV2> for SymbolKind {
    fn from(value: SymbolKindV2) -> Self {
        match value {
            SymbolKindV2::Function => Self::Function,
            SymbolKindV2::Method => Self::Method,
            SymbolKindV2::Constructor => Self::Constructor,
            SymbolKindV2::Type => Self::Type,
            SymbolKindV2::TypeAlias => Self::TypeAlias,
            SymbolKindV2::Class => Self::Class,
            SymbolKindV2::Interface => Self::Interface,
            SymbolKindV2::Enum => Self::Enum,
            SymbolKindV2::EnumVariant => Self::EnumVariant,
            SymbolKindV2::Trait => Self::Trait,
            SymbolKindV2::Module => Self::Module,
            SymbolKindV2::Namespace => Self::Namespace,
            SymbolKindV2::Variable => Self::Variable,
            SymbolKindV2::Parameter => Self::Parameter,
            SymbolKindV2::Field => Self::Field,
            SymbolKindV2::Property => Self::Property,
            SymbolKindV2::Macro => Self::Macro,
            SymbolKindV2::Constant => Self::Constant,
            SymbolKindV2::Unknown => Self::Unknown,
        }
    }
}

impl From<Option<Visibility>> for VisibilityLevel {
    fn from(value: Option<Visibility>) -> Self {
        match value {
            Some(Visibility::Public) => Self::Public,
            Some(Visibility::Protected) => Self::Protected,
            Some(Visibility::Private) => Self::Private,
            Some(Visibility::Internal) => Self::Internal,
            Some(Visibility::Package) => Self::Package,
            Some(Visibility::Crate) => Self::Crate,
            Some(Visibility::ModulePrivate) => Self::ModulePrivate,
            Some(Visibility::ProtectedInternal) => Self::ProtectedInternal,
            Some(Visibility::PrivateProtected) => Self::PrivateProtected,
            None => Self::Unknown,
        }
    }
}

impl From<VisibilityLevel> for Option<Visibility> {
    fn from(value: VisibilityLevel) -> Self {
        match value {
            VisibilityLevel::Public => Some(Visibility::Public),
            VisibilityLevel::Protected => Some(Visibility::Protected),
            VisibilityLevel::Private => Some(Visibility::Private),
            VisibilityLevel::Internal => Some(Visibility::Internal),
            VisibilityLevel::Package => Some(Visibility::Package),
            VisibilityLevel::Crate => Some(Visibility::Crate),
            VisibilityLevel::ModulePrivate => Some(Visibility::ModulePrivate),
            VisibilityLevel::ProtectedInternal => Some(Visibility::ProtectedInternal),
            VisibilityLevel::PrivateProtected => Some(Visibility::PrivateProtected),
            VisibilityLevel::Unknown => None,
        }
    }
}

fn text_span_from_range(range: &CodeRange) -> TextSpan {
    TextSpan {
        start: TextPoint {
            line: range.start_line as u32,
            column: range.start_column as u32,
        },
        end: TextPoint {
            line: range.end_line as u32,
            column: range.end_column as u32,
        },
        start_byte: range.start_byte as u32,
        end_byte: range.end_byte as u32,
    }
}

fn code_range_from_span(span: &TextSpan) -> CodeRange {
    CodeRange {
        start_line: span.start.line as usize,
        start_column: span.start.column as usize,
        end_line: span.end.line as usize,
        end_column: span.end.column as usize,
        start_byte: span.start_byte as usize,
        end_byte: span.end_byte as usize,
    }
}

fn function_kind_data(signature: &Option<FunctionSignature>) -> SymbolKindData {
    let mut data = FunctionData::default();

    if let Some(signature) = signature {
        data.parameters = signature
            .parameters
            .iter()
            .enumerate()
            .map(|(index, parameter)| ParameterData {
                name: parameter.name.clone(),
                position: index as u16,
                label: None,
                type_text: parameter.type_annotation.clone(),
                type_ast: None,
                default_value: parameter.default_value.clone(),
                is_optional: parameter.default_value.is_some(),
                is_variadic: parameter.is_variadic,
                is_keyword_only: false,
                is_positional_only: false,
                mutability: None,
                ownership_mode: None,
                docs: None,
                attributes: Vec::new(),
            })
            .collect();
        data.return_info = ReturnData {
            type_text: signature.return_type.clone(),
            ..ReturnData::default()
        };
        data.is_variadic = signature
            .parameters
            .iter()
            .any(|parameter| parameter.is_variadic);
        data.arity = data.parameters.len() as u16;
    }

    SymbolKindData::Function(data)
}

fn field_data_from_field(field: &FieldInfo) -> FieldData {
    FieldData {
        name: field.name.clone(),
        qualified_name: None,
        type_text: field.type_annotation.clone(),
        visibility: field.visibility.map(|visibility| Some(visibility).into()),
        is_static: field.is_static,
        is_readonly: false,
        is_mutable: true,
        is_optional: false,
        default_value: None,
        docs: field.doc_comment.clone(),
    }
}

fn type_kind_data(metadata: &Option<TypeMetadata>, kind: SymbolKind) -> SymbolKindData {
    let Some(metadata) = metadata else {
        return SymbolKindData::Unknown;
    };

    if kind == SymbolKind::Enum {
        return SymbolKindData::Enum(EnumData {
            backing_type: None,
            variants: metadata
                .variants
                .iter()
                .map(|variant| EnumVariantData {
                    name: variant.name.clone(),
                    discriminant: variant.discriminant.clone(),
                    tuple_fields: variant.tuple_fields.clone(),
                    struct_fields: variant
                        .struct_fields
                        .iter()
                        .map(field_data_from_field)
                        .collect(),
                    docs: variant.doc_comment.clone(),
                })
                .collect(),
        });
    }

    SymbolKindData::Type(TypeData {
        members: Vec::new(),
        fields: metadata.fields.iter().map(field_data_from_field).collect(),
        constructors: Vec::new(),
        associated_types: Vec::new(),
        associated_constants: Vec::new(),
        is_abstract: false,
        is_final: false,
        is_sealed: false,
        is_open: false,
        is_partial: false,
    })
}

fn modifiers_from_symbol(symbol: &SymbolInfo) -> Vec<Modifier> {
    let mut modifiers = Vec::new();
    if symbol.is_static {
        modifiers.push(Modifier::Static);
    }
    if symbol.is_static_storage {
        modifiers.push(Modifier::StaticStorage);
    }
    if symbol.is_async {
        modifiers.push(Modifier::Async);
    }
    modifiers
}

fn declared_type_views(symbol: &SymbolInfo) -> TypeFacet {
    let mut facet = TypeFacet::default();

    if let Some(signature) = &symbol.signature
        && let Some(return_type) = &signature.return_type
    {
        facet.primary_type = Some(return_type.clone());
        facet.views.push(TypeView {
            kind: TypeViewKind::Declared,
            text: Some(return_type.clone()),
            ast: None,
            referenced_symbol_ids: Vec::new(),
        });
        facet.views.push(TypeView {
            kind: TypeViewKind::Canonical,
            text: Some(return_type.trim().to_string()),
            ast: None,
            referenced_symbol_ids: Vec::new(),
        });
    }

    facet
}

impl From<SymbolInfo> for SymbolRecord {
    fn from(value: SymbolInfo) -> Self {
        let qualified_name = record_qualified_name(&value);
        let stable_key = stable_symbol_key(value.language, &value.file, SymbolKindV2::from(value.kind), qualified_name, 0);
        Self::with_stable_key(value, stable_key)
    }
}

/// The qualified name a record of `value` carries: its own, else its name.
pub(crate) fn record_qualified_name(value: &SymbolInfo) -> &str {
    if value.qualified_name.is_empty() { &value.name } else { &value.qualified_name }
}

impl SymbolRecord {
    /// The record of `value` whose identity is `stable_key` (and the ID it
    /// hashes to): [`From<SymbolInfo>`] with the sibling ordinal the caller
    /// already knows, so no other key is built.
    pub(crate) fn with_stable_key(value: SymbolInfo, stable_key: String) -> Self {
        let kind = SymbolKindV2::from(value.kind);
        let declaration_span = text_span_from_range(&value.range);
        let qualified_name = record_qualified_name(&value).to_string();
        let level = VisibilityLevel::from(value.visibility);

        let mut comments = Vec::new();
        if let Some(raw_doc) = &value.doc_comment {
            comments.push(AttachedComment {
                kind: CommentKind::Doc,
                attachment: CommentAttachment::Leading,
                span: declaration_span.clone(),
                raw_text: raw_doc.clone(),
                cleaned_text: raw_doc.trim().to_string(),
                line_distance: None,
            });
        }

        let kind_data = if value.kind.is_function() {
            function_kind_data(&value.signature)
        } else if value.kind == SymbolKind::TypeAlias {
            SymbolKindData::TypeAlias(TypeAliasData::default())
        } else if value.kind.is_type() {
            type_kind_data(&value.type_metadata, value.kind)
        } else if matches!(value.kind, SymbolKind::Field | SymbolKind::Property) {
            SymbolKindData::Field(FieldData {
                name: value.name.clone(),
                qualified_name: None,
                type_text: None,
                visibility: Some(level),
                is_static: value.is_static,
                is_readonly: false,
                is_mutable: true,
                is_optional: false,
                default_value: None,
                docs: value.doc_comment.clone(),
            })
        } else if value.kind == SymbolKind::Parameter {
            SymbolKindData::Parameter(ParameterSymbolData {
                name: value.name.clone(),
                position: 0,
                type_text: None,
            })
        } else {
            SymbolKindData::Unknown
        };
        let raw_doc = value.doc_comment.clone();
        let type_info = declared_type_views(&value);
        let modifiers = modifiers_from_symbol(&value);

        Self {
            schema_version: SchemaVersion::CURRENT,
            id: symbol_id_from_stable_key(&stable_key),
            language: value.language,
            kind,
            scope: value.scope,
            identity: IdentityFacet {
                name: value.name.clone(),
                display_name: value.name,
                qualified_name,
                is_namespace_rooted: value.is_namespace_rooted,
                container_name: value.container_name,
                container_kind: value.container_kind.map(SymbolKindV2::from),
                keyword: None,
                stable_key,
            },
            source: SourceFacet {
                file_path: value.file,
                declaration_span: declaration_span.clone(),
                body_span: None,
                name_span: Some(declaration_span.clone()),
                doc_span: None,
                source_text: None,
                declaration_text: None,
                signature_text: None,
                body_text: None,
            },
            visibility: VisibilityFacet {
                level,
                // Export is decided by the declaration's syntax, never by its
                // access level; `symbol_records()` and the bind pass set it.
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
            },
            docs: DocsFacet {
                raw_doc,
                comments,
                parsed: None,
            },
            attributes: Vec::new(),
            modifiers,
            type_info,
            relations: RelationFacet::default(),
            semantics: SemanticFacet::default(),
            metrics: None,
            provenance: ProvenanceFacet {
                extractor: "tree-sitter".to_string(),
                extractor_version: extractor_version().to_string(),
                parse_pass: AnalysisPass::Parse.as_str().to_string(),
            },
            kind_data,
            extensions: BTreeMap::new(),
        }
    }
}

fn signature_from_function_data(function: FunctionData, is_static: bool) -> FunctionSignature {
    FunctionSignature {
        parameters: function
            .parameters
            .into_iter()
            .map(|parameter| ParameterInfo {
                name: parameter.name,
                type_annotation: parameter.type_text,
                default_value: parameter.default_value,
                is_variadic: parameter.is_variadic,
            })
            .collect(),
        return_type: function.return_info.type_text,
        is_static,
    }
}

fn type_metadata_from_kind_data(kind_data: SymbolKindData) -> Option<TypeMetadata> {
    match kind_data {
        SymbolKindData::Type(data) => Some(TypeMetadata {
            fields: data
                .fields
                .into_iter()
                .map(|field| FieldInfo {
                    name: field.name,
                    type_annotation: field.type_text,
                    doc_comment: field.docs,
                    visibility: field.visibility.and_then(Option::<Visibility>::from),
                    is_static: field.is_static,
                })
                .collect(),
            variants: Vec::new(),
            type_parameters: Vec::new(),
        }),
        SymbolKindData::Enum(data) => Some(TypeMetadata {
            fields: Vec::new(),
            variants: data
                .variants
                .into_iter()
                .map(|variant| VariantInfo {
                    name: variant.name,
                    tuple_fields: variant.tuple_fields,
                    struct_fields: variant
                        .struct_fields
                        .into_iter()
                        .map(|field| FieldInfo {
                            name: field.name,
                            type_annotation: field.type_text,
                            doc_comment: field.docs,
                            visibility: field.visibility.and_then(Option::<Visibility>::from),
                            is_static: field.is_static,
                        })
                        .collect(),
                    discriminant: variant.discriminant,
                    doc_comment: variant.docs,
                })
                .collect(),
            type_parameters: Vec::new(),
        }),
        _ => None,
    }
}

impl TryFrom<SymbolRecord> for SymbolInfo {
    type Error = &'static str;

    fn try_from(value: SymbolRecord) -> Result<Self, Self::Error> {
        let name = value.identity.name;
        if name.is_empty() {
            return Err("symbol record name is empty");
        }

        let kind = SymbolKind::from(value.kind);
        let is_static = value
            .modifiers
            .iter()
            .any(|modifier| matches!(modifier, Modifier::Static));
        let is_static_storage = value
            .modifiers
            .iter()
            .any(|modifier| matches!(modifier, Modifier::StaticStorage));
        let is_async = value
            .modifiers
            .iter()
            .any(|modifier| matches!(modifier, Modifier::Async));

        // Every function-like kind carries a signature, even with no
        // parameters, so renderers print `tick()` rather than `tick`.
        let signature = match (&value.kind_data, kind.is_function()) {
            (SymbolKindData::Function(function), true) => {
                Some(signature_from_function_data(function.clone(), is_static))
            }
            (_, true) => Some(FunctionSignature {
                is_static,
                ..FunctionSignature::new()
            }),
            (_, false) => None,
        };

        let type_metadata = type_metadata_from_kind_data(value.kind_data);

        Ok(Self {
            name,
            kind,
            scope: value.scope,
            qualified_name: value.identity.qualified_name,
            is_namespace_rooted: value.identity.is_namespace_rooted,
            range: code_range_from_span(&value.source.declaration_span),
            language: value.language,
            file: value.source.file_path,
            container_name: value.identity.container_name,
            container_kind: value.identity.container_kind.map(SymbolKind::from),
            doc_comment: value.docs.raw_doc,
            visibility: value.visibility.level.into(),
            is_static,
            is_static_storage,
            is_async,
            signature,
            type_metadata,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Any push order and repeats normalize to Static, StaticStorage, Async,
    /// Const, Readonly, Unsafe, Mut, then distinct Custom values lexically,
    /// and serialize in that order.
    #[test]
    fn modifiers_normalize_to_one_order_without_repeats() {
        let custom = |value: &str| Modifier::Custom(value.to_string());
        let mut modifiers = vec![
            custom("virtual"),
            Modifier::Mut,
            custom("abstract"),
            Modifier::Unsafe,
            Modifier::Async,
            custom("virtual"),
            Modifier::Readonly,
            Modifier::Static,
            Modifier::Const,
            Modifier::StaticStorage,
            custom("final"),
            Modifier::Async,
            custom("override"),
        ];
        normalize_modifiers(&mut modifiers);
        let expected = vec![
            Modifier::Static,
            Modifier::StaticStorage,
            Modifier::Async,
            Modifier::Const,
            Modifier::Readonly,
            Modifier::Unsafe,
            Modifier::Mut,
            custom("abstract"),
            custom("final"),
            custom("override"),
            custom("virtual"),
        ];
        assert_eq!(modifiers, expected);
        assert_eq!(
            serde_json::to_string(&modifiers).expect("serialize"),
            concat!(
                r#"[{"type":"Static"},{"type":"StaticStorage"},{"type":"Async"},{"type":"Const"},"#,
                r#"{"type":"Readonly"},{"type":"Unsafe"},{"type":"Mut"},"#,
                r#"{"type":"Custom","value":"abstract"},{"type":"Custom","value":"final"},"#,
                r#"{"type":"Custom","value":"override"},{"type":"Custom","value":"virtual"}]"#
            )
        );

        let mut empty = Vec::new();
        normalize_modifiers(&mut empty);
        assert!(empty.is_empty());
    }

    /// Symbol IDs digest only the stable key (language, file, kind, qualified
    /// name, ordinal), never the schema version or extractor revision, so a
    /// version bump keeps every ID. Pinned literally across the 4.0 → 4.1 and
    /// revision 17 → 18 bumps.
    #[test]
    fn symbol_ids_survive_schema_and_revision_bumps() {
        let key = stable_symbol_key(
            ProgrammingLanguage::Rust,
            Path::new("src/lib.rs"),
            SymbolKindV2::Function,
            "hello",
            0,
        );
        assert_eq!(key, "rust::src/lib.rs::Function::hello::0");
        assert_eq!(symbol_id_from_stable_key(&key).0, "7d883dc4d6d958aff41b8871016c3cdea132b3b661b1a5da1a1923b30c86a7a4");
    }

    /// The key built from a file's spelled path once is the key built from
    /// the path itself, separators, edge spaces and non-ASCII included; a
    /// record converted with it holds that key and its ID.
    #[test]
    fn stable_key_with_path_is_stable_symbol_key() {
        for (path, kind, name, ordinal) in [
            ("src/lib.rs", SymbolKindV2::Function, "hello", 0),
            ("C:\\work\\src\\lib.rs", SymbolKindV2::Method, "S::m", 3),
            ("  dir/with space/ü.rs  ", SymbolKindV2::Field, "S.f", 1),
            ("", SymbolKindV2::Unknown, "", 0),
        ] {
            let path = Path::new(path);
            let expected = stable_symbol_key(ProgrammingLanguage::Rust, path, kind, name, ordinal);
            assert_eq!(stable_key_with_path(ProgrammingLanguage::Rust, &stable_key_path(path), kind, name, ordinal), expected);
        }
        let symbol = sample_symbol();
        let key = stable_symbol_key(symbol.language, &symbol.file, SymbolKindV2::from(symbol.kind), record_qualified_name(&symbol), 2);
        let record = SymbolRecord::with_stable_key(symbol.clone(), key.clone());
        assert_eq!(record.identity.stable_key, key);
        assert_eq!(record.id, symbol_id_from_stable_key(&key));
        let converted: SymbolRecord = symbol.into();
        assert_eq!(converted.identity.stable_key, stable_symbol_key(record.language, &record.source.file_path, record.kind, &record.identity.qualified_name, 0));
        assert_eq!(converted.provenance.extractor_version, record.provenance.extractor_version);
        assert_eq!(record.provenance.extractor_version, format!("{}+rev.{}", env!("CARGO_PKG_VERSION"), crate::file::EXTRACTOR_REVISION));
    }

    #[test]
    fn stable_key_and_id_are_deterministic() {
        let key = stable_symbol_key(
            ProgrammingLanguage::Rust,
            Path::new("src/lib.rs"),
            SymbolKindV2::Function,
            "hello",
            0,
        );
        let first = symbol_id_from_stable_key(&key);
        let second = symbol_id_from_stable_key(&key);
        assert_eq!(first, second);
        assert_eq!(first.0.len(), 64);
    }

    fn sample_symbol() -> SymbolInfo {
        SymbolInfo {
            name: "m".to_string(),
            kind: SymbolKind::Method,
            scope: SymbolScope::Member,
            qualified_name: String::new(),
            is_namespace_rooted: false,
            range: CodeRange { start_line: 2, start_column: 5, end_line: 2, end_column: 6, start_byte: 14, end_byte: 15 },
            language: ProgrammingLanguage::Rust,
            file: Path::new("src\\a b.rs").to_path_buf(),
            container_name: Some("S".to_string()),
            container_kind: None,
            doc_comment: None,
            visibility: None,
            is_static: false,
            is_static_storage: false,
            is_async: false,
            signature: Some(FunctionSignature::new()),
            type_metadata: None,
        }
    }

    #[test]
    fn symbol_record_round_trip_via_adapter() {
        let symbol = SymbolInfo {
            name: "hello".to_string(),
            kind: SymbolKind::Function,
            scope: SymbolScope::Item,
            qualified_name: "hello".to_string(),
            is_namespace_rooted: false,
            range: CodeRange {
                start_line: 1,
                start_column: 1,
                end_line: 1,
                end_column: 10,
                start_byte: 0,
                end_byte: 9,
            },
            language: ProgrammingLanguage::Rust,
            file: Path::new("src/lib.rs").to_path_buf(),
            container_name: None,
            container_kind: None,
            doc_comment: Some("Says hi".to_string()),
            visibility: None,
            is_static: false,
            is_static_storage: false,
            is_async: false,
            signature: Some(FunctionSignature::new()),
            type_metadata: None,
        };

        let record: SymbolRecord = symbol.clone().into();
        let back = SymbolInfo::try_from(record).expect("adapter should succeed");
        assert_eq!(back.name, symbol.name);
        assert_eq!(back.kind, symbol.kind);
        assert_eq!(back.language, symbol.language);
    }

    #[test]
    fn symbol_record_serialization_round_trip() {
        let symbol = SymbolInfo {
            name: "hello".to_string(),
            kind: SymbolKind::Function,
            scope: SymbolScope::Item,
            qualified_name: "hello".to_string(),
            is_namespace_rooted: false,
            range: CodeRange {
                start_line: 1,
                start_column: 1,
                end_line: 1,
                end_column: 10,
                start_byte: 0,
                end_byte: 9,
            },
            language: ProgrammingLanguage::Rust,
            file: Path::new("src/lib.rs").to_path_buf(),
            container_name: None,
            container_kind: None,
            doc_comment: Some("Says hi".to_string()),
            visibility: None,
            is_static: false,
            is_static_storage: false,
            is_async: false,
            signature: None,
            type_metadata: None,
        };

        let record: SymbolRecord = symbol.into();
        let json = serde_json::to_string(&record).expect("serialize");
        let parsed: SymbolRecord = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed.schema_version, SchemaVersion::CURRENT);
        assert_eq!(parsed.identity.name, "hello");
    }
}
