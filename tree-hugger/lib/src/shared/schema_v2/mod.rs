use std::collections::BTreeMap;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use biscuit_hash::blake3_hash;
use serde::{Deserialize, Serialize};

use super::{
    CodeRange, Diagnostic, FieldInfo, FunctionSignature, ParameterInfo, ProgrammingLanguage,
    SymbolInfo, SymbolKind, TypeMetadata, VariantInfo, Visibility,
};

mod facets;
mod kinds;
mod types;

pub use facets::*;
pub use kinds::*;
pub use types::*;

/// Top-level v2 symbol record.
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
}

/// Deterministic stable symbol key used to derive `SymbolId`.
pub fn stable_symbol_key(
    language: ProgrammingLanguage,
    file: &Path,
    kind: SymbolKindV2,
    qualified_or_name: &str,
    sibling_ordinal: u32,
) -> String {
    let file_part = file.to_string_lossy().replace('\\', "/").trim().to_string();
    format!(
        "{}::{}::{:?}::{}::{}",
        language.query_name(),
        file_part,
        kind,
        qualified_or_name,
        sibling_ordinal
    )
}

/// Converts a stable symbol key to a deterministic `SymbolId`.
pub fn symbol_id_from_stable_key(stable_key: &str) -> SymbolId {
    SymbolId(blake3_hash(stable_key))
}

/// Returns current unix timestamp in milliseconds.
pub fn now_epoch_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

impl From<SymbolKind> for SymbolKindV2 {
    fn from(value: SymbolKind) -> Self {
        match value {
            SymbolKind::Function => Self::Function,
            SymbolKind::Method => Self::Method,
            SymbolKind::Type => Self::Type,
            SymbolKind::Class => Self::Class,
            SymbolKind::Interface => Self::Interface,
            SymbolKind::Enum => Self::Enum,
            SymbolKind::Trait => Self::Trait,
            SymbolKind::Module => Self::Module,
            SymbolKind::Namespace => Self::Namespace,
            SymbolKind::Variable => Self::Variable,
            SymbolKind::Parameter => Self::Parameter,
            SymbolKind::Field => Self::Field,
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
            SymbolKindV2::Type => Self::Type,
            SymbolKindV2::Class => Self::Class,
            SymbolKindV2::Interface => Self::Interface,
            SymbolKindV2::Enum => Self::Enum,
            SymbolKindV2::Trait => Self::Trait,
            SymbolKindV2::Module => Self::Module,
            SymbolKindV2::Namespace => Self::Namespace,
            SymbolKindV2::Variable => Self::Variable,
            SymbolKindV2::Parameter => Self::Parameter,
            SymbolKindV2::Field => Self::Field,
            SymbolKindV2::Macro => Self::Macro,
            SymbolKindV2::Constant => Self::Constant,
            _ => Self::Unknown,
        }
    }
}

fn visibility_to_level(visibility: Option<Visibility>) -> VisibilityLevel {
    match visibility {
        Some(Visibility::Public) => VisibilityLevel::Public,
        Some(Visibility::Protected) => VisibilityLevel::Protected,
        Some(Visibility::Private) => VisibilityLevel::Private,
        Some(Visibility::Internal) => VisibilityLevel::Internal,
        None => VisibilityLevel::Unknown,
    }
}

fn level_to_visibility(level: VisibilityLevel) -> Option<Visibility> {
    match level {
        VisibilityLevel::Public => Some(Visibility::Public),
        VisibilityLevel::Protected => Some(Visibility::Protected),
        VisibilityLevel::Private => Some(Visibility::Private),
        VisibilityLevel::Internal => Some(Visibility::Internal),
        _ => None,
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
        visibility: field
            .visibility
            .map(|visibility| visibility_to_level(Some(visibility))),
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
                    discriminant: None,
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

fn modifiers_from_signature(signature: &Option<FunctionSignature>) -> Vec<Modifier> {
    let mut modifiers = Vec::new();
    if signature
        .as_ref()
        .map(|value| value.is_static)
        .unwrap_or(false)
    {
        modifiers.push(Modifier::Static);
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
        let kind = SymbolKindV2::from(value.kind);
        let declaration_span = text_span_from_range(&value.range);
        let qualified_name = value
            .container_name
            .as_ref()
            .map(|container| format!("{container}::{}", value.name));
        let stable_key = stable_symbol_key(
            value.language,
            &value.file,
            kind,
            qualified_name.as_deref().unwrap_or(&value.name),
            0,
        );
        let now = now_epoch_ms();

        let level = value
            .signature
            .as_ref()
            .and_then(|signature| signature.visibility)
            .map_or(VisibilityLevel::Unknown, |visibility| {
                visibility_to_level(Some(visibility))
            });

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
        } else if value.kind.is_type() {
            type_kind_data(&value.type_metadata, value.kind)
        } else if value.kind == SymbolKind::Field {
            SymbolKindData::Field(FieldData {
                name: value.name.clone(),
                qualified_name: None,
                type_text: None,
                visibility: Some(level),
                is_static: false,
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

        Self {
            schema_version: SchemaVersion::V2_0,
            id: symbol_id_from_stable_key(&stable_key),
            language: value.language,
            kind,
            identity: IdentityFacet {
                name: value.name.clone(),
                display_name: value.name,
                qualified_name,
                module_path: value.container_name,
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
                is_exported: level == VisibilityLevel::Public,
                is_reexported: false,
                is_default_export: false,
                is_imported: false,
                is_external: false,
                is_generated: false,
                is_synthetic: false,
                is_deprecated: false,
                is_experimental: false,
            },
            docs: DocsFacet {
                raw_doc,
                comments,
                parsed: None,
            },
            attributes: Vec::new(),
            modifiers: modifiers_from_signature(&value.signature),
            type_info,
            relations: RelationFacet::default(),
            semantics: SemanticFacet::default(),
            provenance: ProvenanceFacet {
                extractor: "tree-sitter".to_string(),
                extractor_version: env!("CARGO_PKG_VERSION").to_string(),
                parse_pass: AnalysisPass::Parse.as_str().to_string(),
                created_at_epoch_ms: now,
                updated_at_epoch_ms: now,
            },
            kind_data,
            extensions: BTreeMap::new(),
        }
    }
}

fn signature_from_function_data(function: FunctionData) -> Option<FunctionSignature> {
    if function.parameters.is_empty() && function.return_info.type_text.is_none() {
        return None;
    }

    Some(FunctionSignature {
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
        visibility: None,
        is_static: false,
    })
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
                    visibility: field.visibility.and_then(level_to_visibility),
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
                            visibility: field.visibility.and_then(level_to_visibility),
                            is_static: field.is_static,
                        })
                        .collect(),
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

        let signature = match value.kind_data.clone() {
            SymbolKindData::Function(function) => signature_from_function_data(function),
            _ => None,
        };

        let type_metadata = type_metadata_from_kind_data(value.kind_data);

        Ok(Self {
            name,
            kind: SymbolKind::from(value.kind),
            range: code_range_from_span(&value.source.declaration_span),
            language: value.language,
            file: value.source.file_path,
            container_name: value.identity.module_path,
            container_kind: None,
            doc_comment: value.docs.raw_doc,
            signature,
            type_metadata,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn symbol_record_round_trip_via_adapter() {
        let symbol = SymbolInfo {
            name: "hello".to_string(),
            kind: SymbolKind::Function,
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
            signature: None,
            type_metadata: None,
        };

        let record: SymbolRecord = symbol.into();
        let json = serde_json::to_string(&record).expect("serialize");
        let parsed: SymbolRecord = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed.schema_version, SchemaVersion::V2_0);
        assert_eq!(parsed.identity.name, "hello");
    }
}
