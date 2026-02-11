//! Mapping functions for OpenAPI to Schematic type conversion.
//!
//! This module provides functions that transform OpenAPI types into
//! their Schematic equivalents.

use std::collections::HashSet;

use openapiv3::{
    AdditionalProperties, ObjectType, Operation, ReferenceOr, Schema, SchemaKind, SecurityScheme,
    Server, Type,
};

use super::diagnostics::OpenApiDiagnostic;
use super::naming::{deconflict_name, operation_id_fallback, sanitize_rust_ident, to_snake_case};
use super::resolver::RefResolver;
use super::{BaseUrlPolicy, ContentPreference, OpenApiImportOptions};
use crate::auth::{ApiKeyLocation, AuthStrategy};
use crate::models::{
    EnumDef, EnumVariant, FieldDef, ModelCatalog, ModelDef, PrimitiveType, StructDef, TypeAlias,
    TypeRef,
};
use crate::params::{EndpointParams, ParamDef, ParamStyle, QueryParamType};
use crate::request::ApiRequest;
use crate::response::ApiResponse;
use crate::types::{Endpoint, RestMethod};

/// Maps OpenAPI Info to (name, description).
pub fn map_info(info: &openapiv3::Info) -> (String, String) {
    let name = sanitize_rust_ident(&info.title);
    let description = info
        .description
        .clone()
        .unwrap_or_else(|| info.title.clone());
    (name, description)
}

/// Maps OpenAPI servers to a base URL string.
pub fn map_server(servers: &[Server], policy: &BaseUrlPolicy) -> String {
    match policy {
        BaseUrlPolicy::Override(url) => url.clone(),
        BaseUrlPolicy::FromServers => servers
            .first()
            .map(|s| s.url.clone())
            .unwrap_or_else(|| "https://api.example.com".to_string()),
    }
}

/// Maps an OpenAPI operation to a Schematic Endpoint.
pub fn map_operation(
    op: &Operation,
    path: &str,
    method: RestMethod,
    resolver: &RefResolver,
    used_names: &mut HashSet<String>,
    options: &OpenApiImportOptions,
) -> Result<(Endpoint, Vec<OpenApiDiagnostic>), String> {
    let mut diagnostics = Vec::new();

    // Determine operation ID
    let raw_id = match &op.operation_id {
        Some(id) => id.clone(),
        None => {
            let fallback = operation_id_fallback(&method.to_string(), path);
            diagnostics.push(OpenApiDiagnostic::info(
                format!(
                    "#/paths{}/{}",
                    path.replace('/', "~1"),
                    method.to_string().to_lowercase()
                ),
                format!("Missing operationId, using fallback: {}", fallback),
            ));
            fallback
        }
    };

    // Sanitize and deconflict the ID
    let sanitized_id = sanitize_rust_ident(&raw_id);
    let id = deconflict_name(&sanitized_id, used_names);
    used_names.insert(id.clone());

    // Map description
    let description = op
        .summary
        .clone()
        .or_else(|| op.description.clone())
        .unwrap_or_else(|| format!("{} {}", method, path));

    // Map request body
    let request = map_request_body(&op.request_body, resolver, &mut diagnostics, options);

    // Map response
    let response = map_responses(&op.responses, resolver, &mut diagnostics, options);

    // Map parameters
    let params = map_parameters(&op.parameters, resolver, &mut diagnostics);

    Ok((
        Endpoint {
            id,
            method,
            path: path.to_string(),
            description,
            request,
            response,
            headers: vec![],
            params: if params.query.is_empty()
                && params.header.is_empty()
                && params.cookie.is_empty()
            {
                None
            } else {
                Some(params)
            },
        },
        diagnostics,
    ))
}

/// Maps an OpenAPI security scheme to an AuthStrategy.
pub fn map_security_scheme(scheme: &SecurityScheme) -> Result<AuthStrategy, String> {
    match scheme {
        SecurityScheme::HTTP {
            scheme: auth_scheme,
            ..
        } => {
            let scheme_lower = auth_scheme.to_lowercase();
            match scheme_lower.as_str() {
                "bearer" => Ok(AuthStrategy::BearerToken { header: None }),
                "basic" => Ok(AuthStrategy::Basic),
                _ => Err(format!("Unsupported HTTP auth scheme: {}", auth_scheme)),
            }
        }
        SecurityScheme::APIKey { location, name, .. } => match location {
            openapiv3::APIKeyLocation::Header => Ok(AuthStrategy::ApiKey {
                header: name.clone(),
            }),
            openapiv3::APIKeyLocation::Query => Ok(AuthStrategy::ApiKeyParam {
                name: name.clone(),
                location: ApiKeyLocation::Query,
            }),
            openapiv3::APIKeyLocation::Cookie => Ok(AuthStrategy::ApiKeyParam {
                name: name.clone(),
                location: ApiKeyLocation::Cookie,
            }),
        },
        SecurityScheme::OAuth2 { .. } => {
            // OAuth2 is complex; default to bearer token behavior
            Err("OAuth2 not fully supported, consider using BearerToken with manual token management".to_string())
        }
        SecurityScheme::OpenIDConnect { .. } => Err("OpenID Connect not supported".to_string()),
    }
}

/// Maps an OpenAPI request body to ApiRequest.
fn map_request_body(
    body: &Option<ReferenceOr<openapiv3::RequestBody>>,
    resolver: &RefResolver,
    diagnostics: &mut Vec<OpenApiDiagnostic>,
    options: &OpenApiImportOptions,
) -> Option<ApiRequest> {
    let body = match body {
        Some(ReferenceOr::Item(b)) => b,
        Some(ReferenceOr::Reference { reference }) => {
            diagnostics.push(OpenApiDiagnostic::warn(
                reference.clone(),
                "Request body references not supported".to_string(),
            ));
            return None;
        }
        None => return None,
    };

    // Select content type based on preference
    let content_types: Vec<_> = body.content.keys().collect();
    let selected = select_content_type(&content_types, options);

    if let Some(ct) = selected
        && let Some(media_type) = body.content.get(ct)
    {
        // Log alternate content types as info
        for other_ct in &content_types {
            if *other_ct != ct {
                diagnostics.push(OpenApiDiagnostic::info(
                    "request body".to_string(),
                    format!("Also supports content type: {}", other_ct),
                ));
            }
        }

        return map_media_type_to_request(ct, media_type, resolver, diagnostics);
    }

    None
}

/// Maps an OpenAPI MediaType to ApiRequest.
fn map_media_type_to_request(
    content_type: &str,
    media_type: &openapiv3::MediaType,
    resolver: &RefResolver,
    diagnostics: &mut Vec<OpenApiDiagnostic>,
) -> Option<ApiRequest> {
    let ct_lower = content_type.to_lowercase();

    if ct_lower.contains("json") {
        // JSON request body
        if let Some(ref schema) = media_type.schema {
            let type_name = extract_type_name_from_schema(schema, resolver, diagnostics);
            return Some(ApiRequest::json_type(&type_name));
        }
    } else if ct_lower.contains("octet-stream") || ct_lower.contains("binary") {
        return Some(ApiRequest::binary(content_type));
    } else if ct_lower.contains("text") {
        return Some(ApiRequest::text(content_type));
    } else if ct_lower.contains("form-data") {
        // Multipart - would need to parse schema for fields
        // For now, just note it
        diagnostics.push(OpenApiDiagnostic::info(
            "request body".to_string(),
            "Multipart form-data requires manual field mapping".to_string(),
        ));
        return None;
    } else if ct_lower.contains("x-www-form-urlencoded") {
        diagnostics.push(OpenApiDiagnostic::info(
            "request body".to_string(),
            "URL-encoded form requires manual field mapping".to_string(),
        ));
        return None;
    }

    None
}

/// Maps OpenAPI responses to ApiResponse.
pub fn map_responses(
    responses: &openapiv3::Responses,
    resolver: &RefResolver,
    diagnostics: &mut Vec<OpenApiDiagnostic>,
    _options: &OpenApiImportOptions,
) -> ApiResponse {
    // Priority: 200 > 201 > 204 > first 2xx > default
    let status_priority = ["200", "201", "204"];

    // Try priority statuses first
    for status in &status_priority {
        if let Some(resp) = responses
            .responses
            .get(&openapiv3::StatusCode::Code(status.parse().unwrap()))
        {
            if *status == "204" {
                return ApiResponse::Empty;
            }
            if let Some(response) = map_response_ref(resp, resolver, diagnostics) {
                return response;
            }
        }
    }

    // Try any 2xx response
    for (status, resp) in &responses.responses {
        if let openapiv3::StatusCode::Code(code) = status
            && (200..300).contains(&(*code as i32))
        {
            if *code == 204 {
                return ApiResponse::Empty;
            }
            if let Some(response) = map_response_ref(resp, resolver, diagnostics) {
                return response;
            }
        }
    }

    // Try default response
    if let Some(ref default) = responses.default
        && let Some(response) = map_response_ref(default, resolver, diagnostics)
    {
        return response;
    }

    // Fallback
    diagnostics.push(OpenApiDiagnostic::warn(
        "responses".to_string(),
        "No suitable response found, defaulting to Empty".to_string(),
    ));
    ApiResponse::Empty
}

/// Maps a response reference to ApiResponse.
fn map_response_ref(
    resp: &ReferenceOr<openapiv3::Response>,
    resolver: &RefResolver,
    diagnostics: &mut Vec<OpenApiDiagnostic>,
) -> Option<ApiResponse> {
    let response = match resp {
        ReferenceOr::Item(r) => r,
        ReferenceOr::Reference { reference } => match resolver.resolve_response(resp) {
            Ok(r) => r,
            Err(e) => {
                diagnostics.push(OpenApiDiagnostic::error(
                    reference.clone(),
                    format!("Failed to resolve response reference: {}", e),
                ));
                return None;
            }
        },
    };

    // Check content types
    for (ct, media_type) in &response.content {
        let ct_lower = ct.to_lowercase();

        if ct_lower.contains("json") {
            if let Some(ref schema) = media_type.schema {
                let type_name = extract_type_name_from_schema(schema, resolver, diagnostics);
                return Some(ApiResponse::json_type(&type_name));
            }
            return Some(ApiResponse::json_type("serde_json::Value"));
        } else if ct_lower.contains("octet-stream")
            || ct_lower.contains("binary")
            || ct_lower.contains("audio")
            || ct_lower.contains("image")
        {
            return Some(ApiResponse::Binary);
        } else if ct_lower.contains("text") {
            return Some(ApiResponse::Text);
        }
    }

    // No content
    if response.content.is_empty() {
        return Some(ApiResponse::Empty);
    }

    None
}

/// Extracts a type name from a schema or reference.
fn extract_type_name_from_schema(
    schema: &ReferenceOr<Schema>,
    _resolver: &RefResolver,
    diagnostics: &mut Vec<OpenApiDiagnostic>,
) -> String {
    match schema {
        ReferenceOr::Reference { reference } => {
            // Extract name from reference like "#/components/schemas/Pet"
            if let Some(name) = reference.strip_prefix("#/components/schemas/") {
                sanitize_rust_ident(name)
            } else {
                diagnostics.push(OpenApiDiagnostic::warn(
                    reference.clone(),
                    "Unexpected reference format".to_string(),
                ));
                "serde_json::Value".to_string()
            }
        }
        ReferenceOr::Item(_) => {
            // Inline schema - would need to generate a name
            "serde_json::Value".to_string()
        }
    }
}

/// Maps OpenAPI parameters to EndpointParams.
fn map_parameters(
    parameters: &[ReferenceOr<openapiv3::Parameter>],
    resolver: &RefResolver,
    diagnostics: &mut Vec<OpenApiDiagnostic>,
) -> EndpointParams {
    let mut params = EndpointParams::default();

    for param_ref in parameters {
        let param = match resolver.resolve_parameter(param_ref) {
            Ok(p) => p,
            Err(e) => {
                diagnostics.push(OpenApiDiagnostic::error(
                    "parameter".to_string(),
                    format!("Failed to resolve parameter: {}", e),
                ));
                continue;
            }
        };

        if let Some(param_def) = map_parameter(param, diagnostics) {
            match param {
                openapiv3::Parameter::Query { .. } => params.query.push(param_def),
                openapiv3::Parameter::Header { .. } => params.header.push(param_def),
                openapiv3::Parameter::Cookie { .. } => params.cookie.push(param_def),
                openapiv3::Parameter::Path { .. } => {
                    // Path parameters are handled via path template, skip
                }
            }
        }
    }

    params
}

/// Maps a single OpenAPI parameter to ParamDef.
pub fn map_parameter(
    param: &openapiv3::Parameter,
    diagnostics: &mut Vec<OpenApiDiagnostic>,
) -> Option<ParamDef> {
    let (name, required, description, schema, style, explode) = match param {
        openapiv3::Parameter::Query {
            parameter_data,
            allow_reserved: _,
            style,
            allow_empty_value: _,
        } => (
            &parameter_data.name,
            parameter_data.required,
            &parameter_data.description,
            get_param_schema(parameter_data),
            map_query_style(style),
            parameter_data.explode.unwrap_or(false),
        ),
        openapiv3::Parameter::Header {
            parameter_data,
            style,
        } => (
            &parameter_data.name,
            parameter_data.required,
            &parameter_data.description,
            get_param_schema(parameter_data),
            map_header_style(style),
            parameter_data.explode.unwrap_or(false),
        ),
        openapiv3::Parameter::Cookie {
            parameter_data,
            style,
        } => (
            &parameter_data.name,
            parameter_data.required,
            &parameter_data.description,
            get_param_schema(parameter_data),
            map_cookie_style(style),
            parameter_data.explode.unwrap_or(false),
        ),
        openapiv3::Parameter::Path { .. } => {
            // Path parameters are inferred from the path template
            return None;
        }
    };

    let param_type = match schema {
        Some(s) => map_schema_to_param_type(s, diagnostics),
        None => QueryParamType::String,
    };

    Some(ParamDef {
        name: name.clone(),
        required,
        description: description.clone(),
        param_type,
        explode,
        style,
    })
}

/// Gets the schema from parameter data.
fn get_param_schema(data: &openapiv3::ParameterData) -> Option<&ReferenceOr<Schema>> {
    match &data.format {
        openapiv3::ParameterSchemaOrContent::Schema(s) => Some(s),
        openapiv3::ParameterSchemaOrContent::Content(_) => None,
    }
}

/// Maps query parameter style.
fn map_query_style(style: &openapiv3::QueryStyle) -> ParamStyle {
    match style {
        openapiv3::QueryStyle::Form => ParamStyle::Form,
        openapiv3::QueryStyle::SpaceDelimited => ParamStyle::SpaceDelimited,
        openapiv3::QueryStyle::PipeDelimited => ParamStyle::PipeDelimited,
        openapiv3::QueryStyle::DeepObject => ParamStyle::DeepObject,
    }
}

/// Maps header parameter style.
fn map_header_style(style: &openapiv3::HeaderStyle) -> ParamStyle {
    match style {
        openapiv3::HeaderStyle::Simple => ParamStyle::Simple,
    }
}

/// Maps cookie parameter style.
fn map_cookie_style(style: &openapiv3::CookieStyle) -> ParamStyle {
    match style {
        openapiv3::CookieStyle::Form => ParamStyle::Form,
    }
}

/// Maps a schema to QueryParamType.
fn map_schema_to_param_type(
    schema: &ReferenceOr<Schema>,
    diagnostics: &mut Vec<OpenApiDiagnostic>,
) -> QueryParamType {
    match schema {
        ReferenceOr::Reference { reference } => {
            diagnostics.push(OpenApiDiagnostic::info(
                reference.clone(),
                "Parameter references schema component".to_string(),
            ));
            QueryParamType::Json
        }
        ReferenceOr::Item(s) => match &s.schema_kind {
            SchemaKind::Type(Type::String(st)) => {
                let values: Vec<String> = st.enumeration.iter().filter_map(|v| v.clone()).collect();
                if !values.is_empty() {
                    return QueryParamType::Enum(values);
                }
                QueryParamType::String
            }
            SchemaKind::Type(Type::Integer(_)) => QueryParamType::Integer,
            SchemaKind::Type(Type::Number(_)) => QueryParamType::Number,
            SchemaKind::Type(Type::Boolean(_)) => QueryParamType::Boolean,
            SchemaKind::Type(Type::Array(arr)) => {
                let item_type = if let Some(ref items) = arr.items {
                    Box::new(map_schema_to_param_type_boxed(items, diagnostics))
                } else {
                    Box::new(QueryParamType::String)
                };
                QueryParamType::Array(item_type)
            }
            _ => QueryParamType::Json,
        },
    }
}

/// Maps a boxed schema reference to QueryParamType.
fn map_schema_to_param_type_boxed(
    schema: &ReferenceOr<Box<Schema>>,
    diagnostics: &mut Vec<OpenApiDiagnostic>,
) -> QueryParamType {
    match schema {
        ReferenceOr::Reference { reference } => {
            diagnostics.push(OpenApiDiagnostic::info(
                reference.clone(),
                "Parameter references schema component".to_string(),
            ));
            QueryParamType::Json
        }
        ReferenceOr::Item(s) => match &s.schema_kind {
            SchemaKind::Type(Type::String(st)) => {
                let values: Vec<String> = st.enumeration.iter().filter_map(|v| v.clone()).collect();
                if !values.is_empty() {
                    return QueryParamType::Enum(values);
                }
                QueryParamType::String
            }
            SchemaKind::Type(Type::Integer(_)) => QueryParamType::Integer,
            SchemaKind::Type(Type::Number(_)) => QueryParamType::Number,
            SchemaKind::Type(Type::Boolean(_)) => QueryParamType::Boolean,
            SchemaKind::Type(Type::Array(arr)) => {
                let item_type = if let Some(ref items) = arr.items {
                    Box::new(map_schema_to_param_type_boxed(items, diagnostics))
                } else {
                    Box::new(QueryParamType::String)
                };
                QueryParamType::Array(item_type)
            }
            _ => QueryParamType::Json,
        },
    }
}

/// Result of mapping schemas, including the catalog and any name remapping.
pub struct SchemaMapResult {
    /// The generated model catalog.
    pub catalog: ModelCatalog,
    /// Mapping from original sanitized names to deconflicted names (only contains entries that changed).
    pub name_mapping: std::collections::HashMap<String, String>,
}

/// Maps all component schemas to a ModelCatalog.
///
/// ## Arguments
///
/// * `doc` - The OpenAPI document
/// * `resolver` - Reference resolver for $ref resolution
/// * `diagnostics` - Collected import diagnostics
/// * `options` - Import options
/// * `reserved_names` - Names reserved by generated request structs (to avoid collisions)
///
/// ## Returns
///
/// A `SchemaMapResult` containing both the `ModelCatalog` and a name mapping
/// for any schemas that were renamed due to collisions.
pub fn map_all_schemas(
    doc: &openapiv3::OpenAPI,
    resolver: &RefResolver,
    diagnostics: &mut Vec<OpenApiDiagnostic>,
    options: &OpenApiImportOptions,
    reserved_names: &HashSet<String>,
) -> SchemaMapResult {
    let mut types = Vec::new();

    // Build name mapping first to handle deconfliction consistently
    // Maps sanitized schema name -> final Rust name (only entries that differ)
    let mut name_mapping: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();
    let mut used_names = reserved_names.clone();

    if let Some(ref components) = doc.components {
        // First pass: compute all final names and build rename mapping
        for (name, _) in &components.schemas {
            let sanitized_name = sanitize_rust_ident(name);
            let final_name = deconflict_name(&sanitized_name, &used_names);
            used_names.insert(final_name.clone());
            // Only store mapping if name was changed
            if sanitized_name != final_name {
                name_mapping.insert(sanitized_name, final_name);
            }
        }

        // Second pass: process schemas and use final names
        let mut processed_names = reserved_names.clone();
        for (name, schema_ref) in &components.schemas {
            let sanitized_name = sanitize_rust_ident(name);
            let final_name = name_mapping
                .get(&sanitized_name)
                .cloned()
                .unwrap_or(sanitized_name);
            processed_names.insert(final_name.clone());

            if let Some(mut model) =
                map_schema_to_model(&final_name, name, schema_ref, resolver, diagnostics)
            {
                // Post-process TypeRefs to use renamed types
                remap_type_refs_in_model(&mut model, &name_mapping);
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

/// Remaps TypeRef::Named values in a ModelDef using the provided mapping.
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
        ModelDef::Enum(_) => {
            // Enums don't have TypeRef fields in their variants
        }
    }
}

/// Remaps a TypeRef using the provided name mapping.
fn remap_type_ref(type_ref: &mut TypeRef, mapping: &std::collections::HashMap<String, String>) {
    match type_ref {
        TypeRef::Named(name) => {
            if let Some(new_name) = mapping.get(name) {
                *name = new_name.clone();
            }
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

/// Maps a single schema to a ModelDef.
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
            // This is a reference to another schema - create an alias
            if let Some(target) = reference.strip_prefix("#/components/schemas/") {
                return Some(ModelDef::Alias(TypeAlias {
                    name: rust_name.to_string(),
                    description: None,
                    target: TypeRef::Named(sanitize_rust_ident(target)),
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
        SchemaKind::Type(Type::Object(obj)) => {
            let struct_def =
                map_object_to_struct(rust_name, description, obj, resolver, diagnostics);
            Some(ModelDef::Struct(struct_def))
        }
        SchemaKind::Type(Type::String(st)) if st.enumeration.iter().any(|v| v.is_some()) => {
            let enum_def = map_string_enum(rust_name, description, st);
            Some(ModelDef::Enum(enum_def))
        }
        SchemaKind::Type(Type::Array(arr)) => {
            // Array type - create alias
            let item_type = map_array_items(&arr.items, resolver, diagnostics);
            Some(ModelDef::Alias(TypeAlias {
                name: rust_name.to_string(),
                description,
                target: TypeRef::Array(Box::new(item_type)),
            }))
        }
        SchemaKind::Type(Type::String(_)) => Some(ModelDef::Alias(TypeAlias {
            name: rust_name.to_string(),
            description,
            target: TypeRef::Primitive(PrimitiveType::String),
        })),
        SchemaKind::Type(Type::Integer(_)) => Some(ModelDef::Alias(TypeAlias {
            name: rust_name.to_string(),
            description,
            target: TypeRef::Primitive(PrimitiveType::Integer),
        })),
        SchemaKind::Type(Type::Number(_)) => Some(ModelDef::Alias(TypeAlias {
            name: rust_name.to_string(),
            description,
            target: TypeRef::Primitive(PrimitiveType::Number),
        })),
        SchemaKind::Type(Type::Boolean(_)) => Some(ModelDef::Alias(TypeAlias {
            name: rust_name.to_string(),
            description,
            target: TypeRef::Primitive(PrimitiveType::Boolean),
        })),
        SchemaKind::OneOf { one_of } => {
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
        SchemaKind::AnyOf { any_of } => {
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
        SchemaKind::AllOf { all_of } => {
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
        SchemaKind::Any(_) => Some(ModelDef::Alias(TypeAlias {
            name: rust_name.to_string(),
            description,
            target: TypeRef::Primitive(PrimitiveType::Json),
        })),
        SchemaKind::Not { .. } => {
            diagnostics.push(OpenApiDiagnostic::warn(
                format!("#/components/schemas/{}", original_name),
                "not schemas are not supported".to_string(),
            ));
            None
        }
    }
}

/// Maps an OpenAPI object schema to a StructDef.
fn map_object_to_struct(
    name: &str,
    description: Option<String>,
    obj: &ObjectType,
    resolver: &RefResolver,
    diagnostics: &mut Vec<OpenApiDiagnostic>,
) -> StructDef {
    let required_set: HashSet<_> = obj.required.iter().collect();
    let mut fields = Vec::new();

    for (field_name, schema_ref) in &obj.properties {
        let rust_field_name = to_snake_case(field_name);
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
        Some(AdditionalProperties::Any(true)) => Some(TypeRef::Primitive(PrimitiveType::Json)),
        Some(AdditionalProperties::Schema(schema)) => {
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

/// Maps a string enum to EnumDef.
fn map_string_enum(
    name: &str,
    description: Option<String>,
    string_type: &openapiv3::StringType,
) -> EnumDef {
    let variants: Vec<EnumVariant> = string_type
        .enumeration
        .iter()
        .filter_map(|v| {
            v.as_ref().map(|s| {
                let variant_name = sanitize_rust_ident(s);
                EnumVariant {
                    name: variant_name,
                    value: Some(s.clone()),
                    description: None,
                }
            })
        })
        .collect();

    EnumDef {
        name: name.to_string(),
        description,
        variants,
        untagged: false,
    }
}

/// Maps array items to TypeRef.
fn map_array_items(
    items: &Option<ReferenceOr<Box<Schema>>>,
    resolver: &RefResolver,
    diagnostics: &mut Vec<OpenApiDiagnostic>,
) -> TypeRef {
    match items {
        Some(item_ref) => {
            // Convert ReferenceOr<Box<Schema>> to work with our function
            match item_ref {
                ReferenceOr::Reference { reference } => {
                    if let Some(name) = reference.strip_prefix("#/components/schemas/") {
                        TypeRef::Named(sanitize_rust_ident(name))
                    } else {
                        diagnostics.push(OpenApiDiagnostic::warn(
                            reference.clone(),
                            "Unexpected array item reference format".to_string(),
                        ));
                        TypeRef::Unknown
                    }
                }
                ReferenceOr::Item(boxed) => {
                    map_schema_to_type_ref_inner(boxed, resolver, diagnostics)
                }
            }
        }
        None => TypeRef::Primitive(PrimitiveType::Json),
    }
}

/// Maps a boxed schema reference (used in properties) to TypeRef.
fn map_schema_to_type_ref(
    schema_ref: &ReferenceOr<Box<Schema>>,
    resolver: &RefResolver,
    diagnostics: &mut Vec<OpenApiDiagnostic>,
) -> TypeRef {
    match schema_ref {
        ReferenceOr::Reference { reference } => {
            if let Some(name) = reference.strip_prefix("#/components/schemas/") {
                TypeRef::Named(sanitize_rust_ident(name))
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

/// Maps a non-boxed schema reference (used in oneOf/anyOf/allOf) to TypeRef.
fn map_schema_ref_to_type_ref(
    schema_ref: &ReferenceOr<Schema>,
    resolver: &RefResolver,
    diagnostics: &mut Vec<OpenApiDiagnostic>,
) -> TypeRef {
    match schema_ref {
        ReferenceOr::Reference { reference } => {
            if let Some(name) = reference.strip_prefix("#/components/schemas/") {
                TypeRef::Named(sanitize_rust_ident(name))
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

/// Maps a schema to TypeRef.
fn map_schema_to_type_ref_inner(
    schema: &Schema,
    resolver: &RefResolver,
    diagnostics: &mut Vec<OpenApiDiagnostic>,
) -> TypeRef {
    match &schema.schema_kind {
        SchemaKind::Type(Type::String(st)) => {
            if st.enumeration.iter().any(|v| v.is_some()) {
                // Inline enum - could generate a name or use unknown
                TypeRef::Primitive(PrimitiveType::String) // Simplified
            } else {
                TypeRef::Primitive(PrimitiveType::String)
            }
        }
        SchemaKind::Type(Type::Integer(_)) => TypeRef::Primitive(PrimitiveType::Integer),
        SchemaKind::Type(Type::Number(_)) => TypeRef::Primitive(PrimitiveType::Number),
        SchemaKind::Type(Type::Boolean(_)) => TypeRef::Primitive(PrimitiveType::Boolean),
        SchemaKind::Type(Type::Array(arr)) => {
            let item_type = map_array_items(&arr.items, resolver, diagnostics);
            TypeRef::Array(Box::new(item_type))
        }
        SchemaKind::Type(Type::Object(_)) => {
            // Inline object - would need to generate a struct
            TypeRef::Primitive(PrimitiveType::Json)
        }
        SchemaKind::OneOf { one_of } => {
            let variants: Vec<TypeRef> = one_of
                .iter()
                .map(|s| map_schema_ref_to_type_ref(s, resolver, diagnostics))
                .collect();
            TypeRef::OneOf(variants)
        }
        SchemaKind::AnyOf { any_of } => {
            let variants: Vec<TypeRef> = any_of
                .iter()
                .map(|s| map_schema_ref_to_type_ref(s, resolver, diagnostics))
                .collect();
            TypeRef::AnyOf(variants)
        }
        SchemaKind::AllOf { all_of } => {
            let variants: Vec<TypeRef> = all_of
                .iter()
                .map(|s| map_schema_ref_to_type_ref(s, resolver, diagnostics))
                .collect();
            TypeRef::AllOf(variants)
        }
        SchemaKind::Any(_) => TypeRef::Primitive(PrimitiveType::Json),
        SchemaKind::Not { .. } => TypeRef::Unknown,
    }
}

/// Selects a content type based on preference.
fn select_content_type<'a>(
    content_types: &[&'a String],
    options: &OpenApiImportOptions,
) -> Option<&'a String> {
    match options.content_preference {
        ContentPreference::PreferJson => {
            // Prefer JSON
            for ct in content_types {
                if ct.contains("json") {
                    return Some(ct);
                }
            }
            // Then text
            for ct in content_types {
                if ct.contains("text") {
                    return Some(ct);
                }
            }
            // Then binary
            for ct in content_types {
                if ct.contains("octet-stream") || ct.contains("binary") {
                    return Some(ct);
                }
            }
            // Fall back to first
            content_types.first().copied()
        }
        ContentPreference::FirstAvailable => content_types.first().copied(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========== map_info Tests ==========

    #[test]
    fn map_info_extracts_title_and_description() {
        let info = openapiv3::Info {
            title: "My API".to_string(),
            description: Some("A test API".to_string()),
            version: "1.0".to_string(),
            ..Default::default()
        };

        let (name, description) = map_info(&info);

        assert_eq!(name, "MyApi");
        assert_eq!(description, "A test API");
    }

    #[test]
    fn map_info_uses_title_as_fallback_description() {
        let info = openapiv3::Info {
            title: "My API".to_string(),
            description: None,
            version: "1.0".to_string(),
            ..Default::default()
        };

        let (_, description) = map_info(&info);
        assert_eq!(description, "My API");
    }

    // ========== map_server Tests ==========

    #[test]
    fn map_server_uses_override() {
        let servers = vec![Server {
            url: "https://original.com".to_string(),
            ..Default::default()
        }];
        let policy = BaseUrlPolicy::Override("https://custom.com".to_string());

        let result = map_server(&servers, &policy);
        assert_eq!(result, "https://custom.com");
    }

    #[test]
    fn map_server_uses_first_server() {
        let servers = vec![
            Server {
                url: "https://first.com".to_string(),
                ..Default::default()
            },
            Server {
                url: "https://second.com".to_string(),
                ..Default::default()
            },
        ];
        let policy = BaseUrlPolicy::FromServers;

        let result = map_server(&servers, &policy);
        assert_eq!(result, "https://first.com");
    }

    #[test]
    fn map_server_default_when_empty() {
        let servers = vec![];
        let policy = BaseUrlPolicy::FromServers;

        let result = map_server(&servers, &policy);
        assert_eq!(result, "https://api.example.com");
    }

    // ========== map_security_scheme Tests ==========

    #[test]
    fn map_security_http_bearer() {
        let scheme = SecurityScheme::HTTP {
            scheme: "bearer".to_string(),
            bearer_format: None,
            description: None,
            extensions: Default::default(),
        };

        let result = map_security_scheme(&scheme).unwrap();
        assert!(matches!(result, AuthStrategy::BearerToken { .. }));
    }

    #[test]
    fn map_security_http_basic() {
        let scheme = SecurityScheme::HTTP {
            scheme: "basic".to_string(),
            bearer_format: None,
            description: None,
            extensions: Default::default(),
        };

        let result = map_security_scheme(&scheme).unwrap();
        assert!(matches!(result, AuthStrategy::Basic));
    }

    #[test]
    fn map_security_api_key_header() {
        let scheme = SecurityScheme::APIKey {
            location: openapiv3::APIKeyLocation::Header,
            name: "X-API-Key".to_string(),
            description: None,
            extensions: Default::default(),
        };

        let result = map_security_scheme(&scheme).unwrap();
        match result {
            AuthStrategy::ApiKey { header } => {
                assert_eq!(header, "X-API-Key");
            }
            _ => panic!("Expected ApiKey"),
        }
    }

    #[test]
    fn map_security_api_key_query() {
        let scheme = SecurityScheme::APIKey {
            location: openapiv3::APIKeyLocation::Query,
            name: "api_key".to_string(),
            description: None,
            extensions: Default::default(),
        };

        let result = map_security_scheme(&scheme).unwrap();
        match result {
            AuthStrategy::ApiKeyParam { name, location } => {
                assert_eq!(name, "api_key");
                assert!(matches!(location, ApiKeyLocation::Query));
            }
            _ => panic!("Expected ApiKeyParam"),
        }
    }

    #[test]
    fn map_security_api_key_cookie() {
        let scheme = SecurityScheme::APIKey {
            location: openapiv3::APIKeyLocation::Cookie,
            name: "session".to_string(),
            description: None,
            extensions: Default::default(),
        };

        let result = map_security_scheme(&scheme).unwrap();
        match result {
            AuthStrategy::ApiKeyParam { name, location } => {
                assert_eq!(name, "session");
                assert!(matches!(location, ApiKeyLocation::Cookie));
            }
            _ => panic!("Expected ApiKeyParam"),
        }
    }

    #[test]
    fn map_security_unsupported_http_scheme() {
        let scheme = SecurityScheme::HTTP {
            scheme: "digest".to_string(),
            bearer_format: None,
            description: None,
            extensions: Default::default(),
        };

        let result = map_security_scheme(&scheme);
        assert!(result.is_err());
    }

    // ========== select_content_type Tests ==========

    #[test]
    fn select_content_type_prefers_json() {
        let xml_str = "application/xml".to_string();
        let json_str = "application/json".to_string();
        let text_str = "text/plain".to_string();
        let types = vec![&xml_str, &json_str, &text_str];
        let options = OpenApiImportOptions::default();

        let result = select_content_type(&types, &options);
        assert_eq!(result.map(|s| s.as_str()), Some("application/json"));
    }

    #[test]
    fn select_content_type_falls_back_to_text() {
        let json_str = "text/plain".to_string();
        let xml_str = "application/xml".to_string();
        let types = vec![&xml_str, &json_str];
        let options = OpenApiImportOptions::default();

        let result = select_content_type(&types, &options);
        assert_eq!(result.map(|s| s.as_str()), Some("text/plain"));
    }
}
