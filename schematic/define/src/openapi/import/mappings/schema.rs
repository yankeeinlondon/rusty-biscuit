use openapiv3::{ReferenceOr, Schema};
use std::collections::HashSet;

use super::super::diagnostics::OpenApiDiagnostic;
use super::super::naming::{deconflict_name, sanitize_rust_field_ident, sanitize_rust_ident};
use super::super::resolver::RefResolver;
use crate::models::{
    EnumDef, EnumVariant, FieldDef, ModelCatalog, ModelDef, PrimitiveType, StructDef, TypeAlias,
    TypeRef,
};

pub struct SchemaMapResult {
    pub catalog: ModelCatalog,
    pub name_mapping: std::collections::HashMap<String, String>,
}

pub fn map_all_schemas(
    doc: &openapiv3::OpenAPI,
    resolver: &RefResolver,
    diagnostics: &mut Vec<OpenApiDiagnostic>,
    options: &super::super::OpenApiImportOptions,
    reserved_names: &HashSet<String>,
) -> SchemaMapResult {
    let mut types = Vec::new();
    // Final Rust name for every source schema, keyed by its ORIGINAL wire name.
    // Keying by the original (not the sanitized) name is what keeps two schemas
    // that sanitize identically (`user-name`, `user_name`) as distinct models
    // and lets each `TypeRef::Named` ref resolve to the correct target.
    let mut final_by_original: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();
    // Sanitized -> final, surfaced to the builder for request type-name remaps.
    let mut name_mapping: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();
    let mut used_names = reserved_names.clone();

    if let Some(ref components) = doc.components {
        for (name, _) in &components.schemas {
            let sanitized_name = sanitize_rust_ident(name);
            let final_name = deconflict_name(&sanitized_name, &used_names);
            used_names.insert(final_name.clone());
            if sanitized_name != final_name {
                name_mapping.insert(sanitized_name, final_name.clone());
            }
            final_by_original.insert(name.clone(), final_name);
        }

        for (name, schema_ref) in &components.schemas {
            let final_name = final_by_original
                .get(name)
                .cloned()
                .unwrap_or_else(|| sanitize_rust_ident(name));

            if let Some(mut model) =
                map_schema_to_model(&final_name, name, schema_ref, resolver, diagnostics)
            {
                remap_type_refs_in_model(&mut model, &final_by_original);
                types.push(model);
            }
        }
    }

    SchemaMapResult {
        catalog: ModelCatalog {
            module_path: options.module_path.clone(),
            types,
        },
        name_mapping,
    }
}

fn remap_type_refs_in_model(
    model: &mut ModelDef,
    mapping: &std::collections::HashMap<String, String>,
) {
    match model {
        ModelDef::Struct(s) => {
            for field in &mut s.fields {
                remap_type_ref(&mut field.field_type, mapping);
            }
            if let Some(ref mut ap) = s.additional_properties {
                remap_type_ref(ap, mapping);
            }
        }
        ModelDef::Alias(a) => {
            remap_type_ref(&mut a.target, mapping);
        }
        ModelDef::Enum(_) => {}
    }
}

/// Resolves each `TypeRef::Named` from its original wire name to the final Rust
/// name. `mapping` is keyed by original name; a name absent from it (e.g. a
/// dangling ref) falls back to plain sanitization so output stays valid Rust.
fn remap_type_ref(type_ref: &mut TypeRef, mapping: &std::collections::HashMap<String, String>) {
    match type_ref {
        TypeRef::Named(name) => {
            *name = mapping
                .get(name)
                .cloned()
                .unwrap_or_else(|| sanitize_rust_ident(name));
        }
        TypeRef::Array(inner) | TypeRef::Map(inner) | TypeRef::Optional(inner) => {
            remap_type_ref(inner, mapping);
        }
        TypeRef::OneOf(refs) | TypeRef::AnyOf(refs) | TypeRef::AllOf(refs) => {
            for r in refs {
                remap_type_ref(r, mapping);
            }
        }
        TypeRef::Primitive(_) | TypeRef::Unknown => {}
    }
}

fn map_schema_to_model(
    rust_name: &str,
    original_name: &str,
    schema_ref: &ReferenceOr<Schema>,
    resolver: &RefResolver,
    diagnostics: &mut Vec<OpenApiDiagnostic>,
) -> Option<ModelDef> {
    let schema = match schema_ref {
        ReferenceOr::Item(s) => s,
        ReferenceOr::Reference { reference } => {
            if let Some(target) = reference.strip_prefix("#/components/schemas/") {
                return Some(ModelDef::Alias(TypeAlias {
                    name: rust_name.to_string(),
                    description: None,
                    // Raw original name; `remap_type_refs_in_model` resolves it.
                    target: TypeRef::Named(target.to_string()),
                }));
            }
            diagnostics.push(OpenApiDiagnostic::warn(
                reference.clone(),
                "Unexpected schema reference format".to_string(),
            ));
            return None;
        }
    };

    let description = schema.schema_data.description.clone();

    match &schema.schema_kind {
        openapiv3::SchemaKind::Type(openapiv3::Type::Object(obj)) => {
            let struct_def =
                map_object_to_struct(rust_name, description, obj, resolver, diagnostics);
            Some(ModelDef::Struct(struct_def))
        }
        openapiv3::SchemaKind::Type(openapiv3::Type::String(st))
            if st.enumeration.iter().any(|v| v.is_some()) =>
        {
            let enum_def = map_string_enum(rust_name, description, st);
            Some(ModelDef::Enum(enum_def))
        }
        openapiv3::SchemaKind::Type(openapiv3::Type::Array(arr)) => {
            let item_type = map_array_items(&arr.items, resolver, diagnostics);
            Some(ModelDef::Alias(TypeAlias {
                name: rust_name.to_string(),
                description,
                target: TypeRef::Array(Box::new(item_type)),
            }))
        }
        openapiv3::SchemaKind::Type(openapiv3::Type::String(_)) => {
            Some(ModelDef::Alias(TypeAlias {
                name: rust_name.to_string(),
                description,
                target: TypeRef::Primitive(PrimitiveType::String),
            }))
        }
        openapiv3::SchemaKind::Type(openapiv3::Type::Integer(_)) => {
            Some(ModelDef::Alias(TypeAlias {
                name: rust_name.to_string(),
                description,
                target: TypeRef::Primitive(PrimitiveType::Integer),
            }))
        }
        openapiv3::SchemaKind::Type(openapiv3::Type::Number(_)) => {
            Some(ModelDef::Alias(TypeAlias {
                name: rust_name.to_string(),
                description,
                target: TypeRef::Primitive(PrimitiveType::Number),
            }))
        }
        openapiv3::SchemaKind::Type(openapiv3::Type::Boolean(_)) => {
            Some(ModelDef::Alias(TypeAlias {
                name: rust_name.to_string(),
                description,
                target: TypeRef::Primitive(PrimitiveType::Boolean),
            }))
        }
        openapiv3::SchemaKind::OneOf { one_of } => {
            let variants: Vec<TypeRef> = one_of
                .iter()
                .map(|s| map_schema_ref_to_type_ref(s, resolver, diagnostics))
                .collect();
            Some(ModelDef::Alias(TypeAlias {
                name: rust_name.to_string(),
                description,
                target: TypeRef::OneOf(variants),
            }))
        }
        openapiv3::SchemaKind::AnyOf { any_of } => {
            let variants: Vec<TypeRef> = any_of
                .iter()
                .map(|s| map_schema_ref_to_type_ref(s, resolver, diagnostics))
                .collect();
            Some(ModelDef::Alias(TypeAlias {
                name: rust_name.to_string(),
                description,
                target: TypeRef::AnyOf(variants),
            }))
        }
        openapiv3::SchemaKind::AllOf { all_of } => {
            let variants: Vec<TypeRef> = all_of
                .iter()
                .map(|s| map_schema_ref_to_type_ref(s, resolver, diagnostics))
                .collect();
            Some(ModelDef::Alias(TypeAlias {
                name: rust_name.to_string(),
                description,
                target: TypeRef::AllOf(variants),
            }))
        }
        // JSON Schema (and so OpenAPI 3.1) lets a schema carry `properties`
        // without a `type: object`, which openapiv3 parses as `Any`. OpenAI's
        // `Model` and `OpenAIFile` are spelled that way; without this arm they
        // would erase to `serde_json::Value`.
        openapiv3::SchemaKind::Any(any) if !any.properties.is_empty() => {
            let object = openapiv3::ObjectType {
                properties: any.properties.clone(),
                required: any.required.clone(),
                additional_properties: any.additional_properties.clone(),
                min_properties: any.min_properties,
                max_properties: any.max_properties,
            };
            let struct_def =
                map_object_to_struct(rust_name, description, &object, resolver, diagnostics);
            Some(ModelDef::Struct(struct_def))
        }
        openapiv3::SchemaKind::Any(_) => Some(ModelDef::Alias(TypeAlias {
            name: rust_name.to_string(),
            description,
            target: TypeRef::Primitive(PrimitiveType::Json),
        })),
        openapiv3::SchemaKind::Not { .. } => {
            diagnostics.push(OpenApiDiagnostic::warn(
                format!("#/components/schemas/{}", original_name),
                "not schemas are not supported".to_string(),
            ));
            None
        }
    }
}

fn map_object_to_struct(
    name: &str,
    description: Option<String>,
    obj: &openapiv3::ObjectType,
    resolver: &RefResolver,
    diagnostics: &mut Vec<OpenApiDiagnostic>,
) -> StructDef {
    let required_set: HashSet<_> = obj.required.iter().collect();
    let mut fields = Vec::new();

    for (field_name, schema_ref) in &obj.properties {
        let rust_field_name = sanitize_rust_field_ident(field_name);
        let serde_rename = if rust_field_name != *field_name {
            Some(field_name.clone())
        } else {
            None
        };

        let field_type = map_schema_to_type_ref(schema_ref, resolver, diagnostics);
        let required = required_set.contains(field_name);

        let field_desc = match schema_ref {
            ReferenceOr::Item(s) => s.schema_data.description.clone(),
            _ => None,
        };

        fields.push(FieldDef {
            name: rust_field_name,
            serde_rename,
            description: field_desc,
            required,
            field_type,
        });
    }

    let additional_properties = match &obj.additional_properties {
        Some(openapiv3::AdditionalProperties::Any(true)) => {
            Some(TypeRef::Primitive(PrimitiveType::Json))
        }
        Some(openapiv3::AdditionalProperties::Schema(schema)) => {
            Some(map_schema_ref_to_type_ref(schema, resolver, diagnostics))
        }
        _ => None,
    };

    StructDef {
        name: name.to_string(),
        description,
        fields,
        additional_properties,
    }
}

fn map_string_enum(
    name: &str,
    description: Option<String>,
    string_type: &openapiv3::StringType,
) -> EnumDef {
    // Values differing only by case/separators (`active`/`Active`) sanitize to
    // the same variant identifier; deconflict the identifier while keeping each
    // original wire value so serde still round-trips the source string.
    let mut used_variant_names: HashSet<String> = HashSet::new();
    let mut variants: Vec<EnumVariant> = Vec::new();
    for value in string_type.enumeration.iter().flatten() {
        let sanitized = sanitize_rust_ident(value);
        let variant_name = deconflict_name(&sanitized, &used_variant_names);
        used_variant_names.insert(variant_name.clone());
        variants.push(EnumVariant {
            name: variant_name,
            value: Some(value.clone()),
            description: None,
        });
    }

    EnumDef {
        name: name.to_string(),
        description,
        variants,
        untagged: false,
    }
}

fn map_array_items(
    items: &Option<ReferenceOr<Box<Schema>>>,
    resolver: &RefResolver,
    diagnostics: &mut Vec<OpenApiDiagnostic>,
) -> TypeRef {
    match items {
        Some(item_ref) => match item_ref {
            ReferenceOr::Reference { reference } => {
                if let Some(name) = reference.strip_prefix("#/components/schemas/") {
                    TypeRef::Named(name.to_string())
                } else {
                    diagnostics.push(OpenApiDiagnostic::warn(
                        reference.clone(),
                        "Unexpected array item reference format".to_string(),
                    ));
                    TypeRef::Unknown
                }
            }
            ReferenceOr::Item(boxed) => map_schema_to_type_ref_inner(boxed, resolver, diagnostics),
        },
        None => TypeRef::Primitive(PrimitiveType::Json),
    }
}

fn map_schema_to_type_ref(
    schema_ref: &ReferenceOr<Box<Schema>>,
    resolver: &RefResolver,
    diagnostics: &mut Vec<OpenApiDiagnostic>,
) -> TypeRef {
    match schema_ref {
        ReferenceOr::Reference { reference } => {
            if let Some(name) = reference.strip_prefix("#/components/schemas/") {
                TypeRef::Named(name.to_string())
            } else {
                diagnostics.push(OpenApiDiagnostic::warn(
                    reference.clone(),
                    "Unexpected reference format".to_string(),
                ));
                TypeRef::Unknown
            }
        }
        ReferenceOr::Item(schema) => map_schema_to_type_ref_inner(schema, resolver, diagnostics),
    }
}

pub(super) fn map_schema_ref_to_type_ref(
    schema_ref: &ReferenceOr<Schema>,
    resolver: &RefResolver,
    diagnostics: &mut Vec<OpenApiDiagnostic>,
) -> TypeRef {
    match schema_ref {
        ReferenceOr::Reference { reference } => {
            if let Some(name) = reference.strip_prefix("#/components/schemas/") {
                TypeRef::Named(name.to_string())
            } else {
                diagnostics.push(OpenApiDiagnostic::warn(
                    reference.clone(),
                    "Unexpected reference format".to_string(),
                ));
                TypeRef::Unknown
            }
        }
        ReferenceOr::Item(schema) => map_schema_to_type_ref_inner(schema, resolver, diagnostics),
    }
}

fn map_schema_to_type_ref_inner(
    schema: &Schema,
    resolver: &RefResolver,
    diagnostics: &mut Vec<OpenApiDiagnostic>,
) -> TypeRef {
    match &schema.schema_kind {
        openapiv3::SchemaKind::Type(openapiv3::Type::String(_)) => {
            TypeRef::Primitive(PrimitiveType::String)
        }
        openapiv3::SchemaKind::Type(openapiv3::Type::Integer(_)) => {
            TypeRef::Primitive(PrimitiveType::Integer)
        }
        openapiv3::SchemaKind::Type(openapiv3::Type::Number(_)) => {
            TypeRef::Primitive(PrimitiveType::Number)
        }
        openapiv3::SchemaKind::Type(openapiv3::Type::Boolean(_)) => {
            TypeRef::Primitive(PrimitiveType::Boolean)
        }
        openapiv3::SchemaKind::Type(openapiv3::Type::Array(arr)) => {
            let item_type = map_array_items(&arr.items, resolver, diagnostics);
            TypeRef::Array(Box::new(item_type))
        }
        openapiv3::SchemaKind::Type(openapiv3::Type::Object(_)) => {
            TypeRef::Primitive(PrimitiveType::Json)
        }
        openapiv3::SchemaKind::OneOf { one_of } => {
            let variants: Vec<TypeRef> = one_of
                .iter()
                .map(|s| map_schema_ref_to_type_ref(s, resolver, diagnostics))
                .collect();
            TypeRef::OneOf(variants)
        }
        openapiv3::SchemaKind::AnyOf { any_of } => {
            let variants: Vec<TypeRef> = any_of
                .iter()
                .map(|s| map_schema_ref_to_type_ref(s, resolver, diagnostics))
                .collect();
            TypeRef::AnyOf(variants)
        }
        openapiv3::SchemaKind::AllOf { all_of } => {
            let variants: Vec<TypeRef> = all_of
                .iter()
                .map(|s| map_schema_ref_to_type_ref(s, resolver, diagnostics))
                .collect();
            TypeRef::AllOf(variants)
        }
        openapiv3::SchemaKind::Any(_) => TypeRef::Primitive(PrimitiveType::Json),
        openapiv3::SchemaKind::Not { .. } => TypeRef::Unknown,
    }
}
