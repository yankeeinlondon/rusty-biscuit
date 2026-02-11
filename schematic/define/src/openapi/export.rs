//! OpenAPI export functionality.
//!
//! This module provides the core export logic for transforming `RestApi` definitions
//! into OpenAPI 3.0.3 documents with vendor extensions for round-trip fidelity.
//!
//! ## Examples
//!
//! ```ignore
//! use schematic_define::openapi::{export, ExportOptions};
//! use schematic_definitions::openai::{define_openai_api, openapi_registry};
//!
//! let api = define_openai_api();
//! let registry = openapi_registry();
//! let options = ExportOptions::new().with_version("1.0.0");
//!
//! let openapi_doc = export(&api, &registry, &options)?;
//! ```

use indexmap::IndexMap;
use openapiv3::{
    MediaType, OpenAPI, Operation, PathItem, Paths, ReferenceOr, RequestBody, Response, Responses,
    SecurityRequirement, SecurityScheme, Server, StatusCode,
};

use super::OpenApiError;
use super::extensions::{SchematicDocExtension, SchematicOpExtension};
use super::options::ExportOptions;
use crate::auth::AuthStrategy;
use crate::request::{ApiRequest, FormField, FormFieldKind};
use crate::response::ApiResponse;
use crate::types::{Endpoint, RestApi, RestMethod};

/// Exports a `RestApi` definition to an OpenAPI 3.0.3 document.
///
/// This function transforms the Schematic API definition into a fully compliant
/// OpenAPI document, including vendor extensions (`x-schematic`) to preserve
/// Schematic-specific metadata for round-trip fidelity.
///
/// ## Parameters
///
/// - `api` - The REST API definition to export
/// - `registry` - Schema registry containing JSON schemas for response types
/// - `options` - Export configuration options
///
/// ## Returns
///
/// An OpenAPI 3.0.3 document.
///
/// ## Errors
///
/// Returns `OpenApiError` if the export fails due to validation issues.
pub fn export<R: SchemaRegistryLike>(
    api: &RestApi,
    registry: &R,
    options: &ExportOptions,
) -> Result<OpenAPI, OpenApiError> {
    let info = map_info(api, options);
    let servers = map_servers(api);
    let paths = map_paths(api, registry, options)?;
    let security = map_security_requirements(api);
    let components = map_components(api, registry, options);

    let mut extensions = IndexMap::new();
    if !options.skip_extensions {
        let doc_ext = SchematicDocExtension {
            module_path: api.module_path.clone(),
            request_suffix: api.request_suffix.clone(),
            env_mapping: api.env_mapping.clone(),
            headers: api.headers.clone(),
        };
        extensions.insert("x-schematic".to_string(), doc_ext.into());
    }

    Ok(OpenAPI {
        openapi: "3.0.3".to_string(),
        info,
        servers,
        paths,
        components: Some(components),
        security: if security.is_empty() {
            None
        } else {
            Some(security)
        },
        tags: vec![],
        external_docs: api
            .docs_url
            .as_ref()
            .map(|url| openapiv3::ExternalDocumentation {
                url: url.clone(),
                description: Some("API Documentation".to_string()),
                ..Default::default()
            }),
        extensions,
    })
}

/// Trait for schema registry abstraction.
///
/// This allows the export function to work with different registry implementations.
pub trait SchemaRegistryLike {
    /// Returns OpenAPI schemas indexed by name.
    fn to_openapi_schemas(&self) -> IndexMap<String, openapiv3::Schema>;
}

/// Maps API metadata to OpenAPI Info.
fn map_info(api: &RestApi, options: &ExportOptions) -> openapiv3::Info {
    openapiv3::Info {
        title: api.name.clone(),
        description: Some(api.description.clone()),
        version: options.version_or_default().to_string(),
        ..Default::default()
    }
}

/// Maps the base URL to OpenAPI servers.
fn map_servers(api: &RestApi) -> Vec<Server> {
    vec![Server {
        url: api.base_url.clone(),
        description: Some(format!("{} API Server", api.name)),
        ..Default::default()
    }]
}

/// Maps all endpoints to OpenAPI paths.
fn map_paths<R: SchemaRegistryLike>(
    api: &RestApi,
    registry: &R,
    options: &ExportOptions,
) -> Result<Paths, OpenApiError> {
    let mut paths = IndexMap::new();

    for endpoint in &api.endpoints {
        let operation = map_operation(endpoint, registry, options)?;
        let path_params = extract_path_params(&endpoint.path);

        // Get or create PathItem for this path
        let path_item = paths
            .entry(endpoint.path.clone())
            .or_insert_with(PathItem::default);

        // Add operation to the appropriate method
        match endpoint.method {
            RestMethod::Get => path_item.get = Some(operation),
            RestMethod::Post => path_item.post = Some(operation),
            RestMethod::Put => path_item.put = Some(operation),
            RestMethod::Patch => path_item.patch = Some(operation),
            RestMethod::Delete => path_item.delete = Some(operation),
            RestMethod::Head => path_item.head = Some(operation),
            RestMethod::Options => path_item.options = Some(operation),
        }

        // Add path parameters at the path level
        for param in path_params {
            path_item.parameters.push(ReferenceOr::Item(param));
        }
    }

    Ok(Paths {
        paths: paths
            .into_iter()
            .map(|(k, v)| (k, ReferenceOr::Item(v)))
            .collect(),
        ..Default::default()
    })
}

/// Maps a single endpoint to an OpenAPI operation.
fn map_operation<R: SchemaRegistryLike>(
    endpoint: &Endpoint,
    _registry: &R,
    options: &ExportOptions,
) -> Result<Operation, OpenApiError> {
    let request_body = endpoint
        .request
        .as_ref()
        .map(|req| ReferenceOr::Item(map_request_body(req)));

    let responses = map_responses(&endpoint.response);

    let mut extensions = IndexMap::new();
    if !options.skip_extensions {
        let op_ext = SchematicOpExtension {
            request: endpoint.request.clone(),
            response: endpoint.response.clone(),
            headers: endpoint.headers.clone(),
        };
        extensions.insert("x-schematic".to_string(), op_ext.into());
    }

    Ok(Operation {
        operation_id: Some(endpoint.id.clone()),
        summary: Some(endpoint.description.clone()),
        description: Some(endpoint.description.clone()),
        request_body,
        responses,
        parameters: vec![],
        extensions,
        ..Default::default()
    })
}

/// Maps authentication strategy to OpenAPI security schemes.
pub fn map_security(auth: &AuthStrategy) -> Option<(String, SecurityScheme)> {
    match auth {
        AuthStrategy::None => None,
        AuthStrategy::BearerToken { header } => {
            // If custom header is specified, use apiKey scheme
            if let Some(header_name) = header {
                Some((
                    "bearerAuth".to_string(),
                    SecurityScheme::APIKey {
                        location: openapiv3::APIKeyLocation::Header,
                        name: header_name.clone(),
                        description: Some("Bearer token authentication".to_string()),
                        extensions: IndexMap::new(),
                    },
                ))
            } else {
                // Standard Authorization: Bearer
                Some((
                    "bearerAuth".to_string(),
                    SecurityScheme::HTTP {
                        scheme: "bearer".to_string(),
                        bearer_format: Some("JWT".to_string()),
                        description: Some("Bearer token authentication".to_string()),
                        extensions: IndexMap::new(),
                    },
                ))
            }
        }
        AuthStrategy::ApiKey { header } => Some((
            "apiKeyAuth".to_string(),
            SecurityScheme::APIKey {
                location: openapiv3::APIKeyLocation::Header,
                name: header.clone(),
                description: Some("API Key authentication".to_string()),
                extensions: IndexMap::new(),
            },
        )),
        AuthStrategy::Basic => Some((
            "basicAuth".to_string(),
            SecurityScheme::HTTP {
                scheme: "basic".to_string(),
                bearer_format: None,
                description: Some("Basic HTTP authentication".to_string()),
                extensions: IndexMap::new(),
            },
        )),
        AuthStrategy::ApiKeyParam { name, location } => {
            let api_key_location = match location {
                crate::auth::ApiKeyLocation::Query => openapiv3::APIKeyLocation::Query,
                crate::auth::ApiKeyLocation::Cookie => openapiv3::APIKeyLocation::Cookie,
            };
            Some((
                "apiKeyAuth".to_string(),
                SecurityScheme::APIKey {
                    location: api_key_location,
                    name: name.clone(),
                    description: Some("API Key authentication".to_string()),
                    extensions: IndexMap::new(),
                },
            ))
        }
    }
}

/// Maps request body to OpenAPI RequestBody.
fn map_request_body(request: &ApiRequest) -> RequestBody {
    let content = match request {
        ApiRequest::Json(schema) => {
            let mut content = IndexMap::new();
            content.insert(
                "application/json".to_string(),
                MediaType {
                    schema: Some(ReferenceOr::Reference {
                        reference: format!("#/components/schemas/{}", schema.type_name),
                    }),
                    ..Default::default()
                },
            );
            content
        }
        ApiRequest::FormData { fields } => {
            let mut content = IndexMap::new();
            content.insert(
                "multipart/form-data".to_string(),
                MediaType {
                    schema: Some(ReferenceOr::Item(map_form_fields_to_schema(fields))),
                    ..Default::default()
                },
            );
            content
        }
        ApiRequest::UrlEncoded { fields } => {
            let mut content = IndexMap::new();
            content.insert(
                "application/x-www-form-urlencoded".to_string(),
                MediaType {
                    schema: Some(ReferenceOr::Item(map_form_fields_to_schema(fields))),
                    ..Default::default()
                },
            );
            content
        }
        ApiRequest::Text { content_type } => {
            let mut content = IndexMap::new();
            content.insert(
                content_type.clone(),
                MediaType {
                    schema: Some(ReferenceOr::Item(openapiv3::Schema {
                        schema_data: openapiv3::SchemaData::default(),
                        schema_kind: openapiv3::SchemaKind::Type(openapiv3::Type::String(
                            openapiv3::StringType::default(),
                        )),
                    })),
                    ..Default::default()
                },
            );
            content
        }
        ApiRequest::Binary { content_type } => {
            let mut content = IndexMap::new();
            content.insert(
                content_type.clone(),
                MediaType {
                    schema: Some(ReferenceOr::Item(openapiv3::Schema {
                        schema_data: openapiv3::SchemaData::default(),
                        schema_kind: openapiv3::SchemaKind::Type(openapiv3::Type::String(
                            openapiv3::StringType {
                                format: openapiv3::VariantOrUnknownOrEmpty::Unknown(
                                    "binary".to_string(),
                                ),
                                ..Default::default()
                            },
                        )),
                    })),
                    ..Default::default()
                },
            );
            content
        }
    };

    RequestBody {
        description: None,
        content,
        required: true,
        ..Default::default()
    }
}

/// Maps form fields to an OpenAPI schema.
fn map_form_fields_to_schema(fields: &[FormField]) -> openapiv3::Schema {
    let mut properties = IndexMap::new();
    let mut required = Vec::new();

    for field in fields {
        let field_schema = match &field.kind {
            FormFieldKind::Text => openapiv3::Schema {
                schema_data: openapiv3::SchemaData {
                    description: field.description.clone(),
                    ..Default::default()
                },
                schema_kind: openapiv3::SchemaKind::Type(openapiv3::Type::String(
                    openapiv3::StringType::default(),
                )),
            },
            FormFieldKind::File { accept: _ } => openapiv3::Schema {
                schema_data: openapiv3::SchemaData {
                    description: field.description.clone(),
                    ..Default::default()
                },
                schema_kind: openapiv3::SchemaKind::Type(openapiv3::Type::String(
                    openapiv3::StringType {
                        format: openapiv3::VariantOrUnknownOrEmpty::Unknown("binary".to_string()),
                        ..Default::default()
                    },
                )),
            },
            FormFieldKind::Files {
                accept: _,
                min: _,
                max: _,
            } => openapiv3::Schema {
                schema_data: openapiv3::SchemaData {
                    description: field.description.clone(),
                    ..Default::default()
                },
                schema_kind: openapiv3::SchemaKind::Type(openapiv3::Type::Array(
                    openapiv3::ArrayType {
                        items: Some(ReferenceOr::Item(Box::new(openapiv3::Schema {
                            schema_data: Default::default(),
                            schema_kind: openapiv3::SchemaKind::Type(openapiv3::Type::String(
                                openapiv3::StringType {
                                    format: openapiv3::VariantOrUnknownOrEmpty::Unknown(
                                        "binary".to_string(),
                                    ),
                                    ..Default::default()
                                },
                            )),
                        }))),
                        min_items: None,
                        max_items: None,
                        unique_items: false,
                    },
                )),
            },
            FormFieldKind::Json(schema) => openapiv3::Schema {
                schema_data: openapiv3::SchemaData {
                    description: field.description.clone(),
                    ..Default::default()
                },
                schema_kind: openapiv3::SchemaKind::AllOf {
                    all_of: vec![ReferenceOr::Reference {
                        reference: format!("#/components/schemas/{}", schema.type_name),
                    }],
                },
            },
        };

        properties.insert(
            field.name.clone(),
            ReferenceOr::Item(Box::new(field_schema)),
        );

        if field.required {
            required.push(field.name.clone());
        }
    }

    openapiv3::Schema {
        schema_data: openapiv3::SchemaData::default(),
        schema_kind: openapiv3::SchemaKind::Type(openapiv3::Type::Object(openapiv3::ObjectType {
            properties,
            required,
            ..Default::default()
        })),
    }
}

/// Maps response type to OpenAPI Responses.
fn map_responses(response: &ApiResponse) -> Responses {
    let mut responses = IndexMap::new();

    let (status_code, content_type, schema) = match response {
        ApiResponse::Json(schema_def) => (
            "200",
            "application/json",
            Some(ReferenceOr::Reference {
                reference: format!("#/components/schemas/{}", schema_def.type_name),
            }),
        ),
        ApiResponse::Text => (
            "200",
            "text/plain",
            Some(ReferenceOr::Item(openapiv3::Schema {
                schema_data: Default::default(),
                schema_kind: openapiv3::SchemaKind::Type(openapiv3::Type::String(
                    openapiv3::StringType::default(),
                )),
            })),
        ),
        ApiResponse::Binary => (
            "200",
            "application/octet-stream",
            Some(ReferenceOr::Item(openapiv3::Schema {
                schema_data: Default::default(),
                schema_kind: openapiv3::SchemaKind::Type(openapiv3::Type::String(
                    openapiv3::StringType {
                        format: openapiv3::VariantOrUnknownOrEmpty::Unknown("binary".to_string()),
                        ..Default::default()
                    },
                )),
            })),
        ),
        ApiResponse::Empty => ("204", "", None),
    };

    let mut content = IndexMap::new();
    if let Some(schema) = schema {
        content.insert(
            content_type.to_string(),
            MediaType {
                schema: Some(schema),
                ..Default::default()
            },
        );
    }

    let response_obj = Response {
        description: match response {
            ApiResponse::Json(_) => "Successful JSON response".to_string(),
            ApiResponse::Text => "Successful text response".to_string(),
            ApiResponse::Binary => "Successful binary response".to_string(),
            ApiResponse::Empty => "No content".to_string(),
        },
        content: if content.is_empty() {
            IndexMap::new()
        } else {
            content
        },
        ..Default::default()
    };

    responses.insert(
        StatusCode::Code(status_code.parse().unwrap_or(200)),
        ReferenceOr::Item(response_obj),
    );

    Responses {
        responses,
        ..Default::default()
    }
}

/// Extracts path parameters from a path template.
///
/// Parses `{param}` segments from the path and returns OpenAPI parameter definitions.
pub fn extract_path_params(path: &str) -> Vec<openapiv3::Parameter> {
    let mut params = Vec::new();

    // Find all {param} patterns in the path
    let mut chars = path.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '{' {
            let mut param_name = String::new();
            while let Some(&next) = chars.peek() {
                if next == '}' {
                    chars.next(); // consume '}'
                    break;
                }
                param_name.push(chars.next().unwrap());
            }

            if !param_name.is_empty() {
                params.push(openapiv3::Parameter::Path {
                    parameter_data: openapiv3::ParameterData {
                        name: param_name,
                        description: None,
                        required: true,
                        format: openapiv3::ParameterSchemaOrContent::Schema(ReferenceOr::Item(
                            openapiv3::Schema {
                                schema_data: Default::default(),
                                schema_kind: openapiv3::SchemaKind::Type(openapiv3::Type::String(
                                    openapiv3::StringType::default(),
                                )),
                            },
                        )),
                        deprecated: None,
                        example: None,
                        examples: IndexMap::new(),
                        explode: None,
                        extensions: IndexMap::new(),
                    },
                    style: openapiv3::PathStyle::Simple,
                });
            }
        }
    }

    params
}

/// Maps security requirements for the API.
fn map_security_requirements(api: &RestApi) -> Vec<SecurityRequirement> {
    match &api.auth {
        AuthStrategy::None => vec![],
        AuthStrategy::BearerToken { header: _ } => {
            let mut req = IndexMap::new();
            req.insert("bearerAuth".to_string(), vec![]);
            vec![req]
        }
        AuthStrategy::ApiKey { header: _ } => {
            let mut req = IndexMap::new();
            req.insert("apiKeyAuth".to_string(), vec![]);
            vec![req]
        }
        AuthStrategy::Basic => {
            let mut req = IndexMap::new();
            req.insert("basicAuth".to_string(), vec![]);
            vec![req]
        }
        AuthStrategy::ApiKeyParam { .. } => {
            let mut req = IndexMap::new();
            req.insert("apiKeyAuth".to_string(), vec![]);
            vec![req]
        }
    }
}

/// Maps API components (schemas and security schemes).
fn map_components<R: SchemaRegistryLike>(
    api: &RestApi,
    registry: &R,
    _options: &ExportOptions,
) -> openapiv3::Components {
    let mut security_schemes = IndexMap::new();

    if let Some((name, scheme)) = map_security(&api.auth) {
        security_schemes.insert(name, ReferenceOr::Item(scheme));
    }

    let schemas = registry
        .to_openapi_schemas()
        .into_iter()
        .map(|(name, schema)| (name, ReferenceOr::Item(schema)))
        .collect();

    openapiv3::Components {
        security_schemes,
        schemas,
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::AuthStrategy;
    use crate::headers::{EnvList, EnvMapping};
    use crate::request::FormField;

    // =============================================
    // Test SchemaRegistry implementation
    // =============================================

    /// A minimal test registry implementation.
    struct TestRegistry {
        schemas: IndexMap<String, openapiv3::Schema>,
    }

    impl TestRegistry {
        fn new() -> Self {
            Self {
                schemas: IndexMap::new(),
            }
        }

        fn with_schema(mut self, name: &str) -> Self {
            self.schemas.insert(
                name.to_string(),
                openapiv3::Schema {
                    schema_data: openapiv3::SchemaData {
                        description: Some(format!("{} schema", name)),
                        ..Default::default()
                    },
                    schema_kind: openapiv3::SchemaKind::Type(openapiv3::Type::Object(
                        openapiv3::ObjectType::default(),
                    )),
                },
            );
            self
        }
    }

    impl SchemaRegistryLike for TestRegistry {
        fn to_openapi_schemas(&self) -> IndexMap<String, openapiv3::Schema> {
            self.schemas.clone()
        }
    }

    // =============================================
    // map_info tests
    // =============================================

    #[test]
    fn map_info_sets_title_from_api_name() {
        let api = create_test_api();
        let options = ExportOptions::new();
        let info = map_info(&api, &options);

        assert_eq!(info.title, "TestAPI");
    }

    #[test]
    fn map_info_sets_description_from_api() {
        let api = create_test_api();
        let options = ExportOptions::new();
        let info = map_info(&api, &options);

        assert_eq!(info.description, Some("Test API description".to_string()));
    }

    #[test]
    fn map_info_uses_version_from_options() {
        let api = create_test_api();
        let options = ExportOptions::new().with_version("2.5.0");
        let info = map_info(&api, &options);

        assert_eq!(info.version, "2.5.0");
    }

    #[test]
    fn map_info_uses_default_version_when_none() {
        let api = create_test_api();
        let options = ExportOptions::new();
        let info = map_info(&api, &options);

        assert_eq!(info.version, "1.0.0");
    }

    // =============================================
    // map_servers tests
    // =============================================

    #[test]
    fn map_servers_creates_single_server() {
        let api = create_test_api();
        let servers = map_servers(&api);

        assert_eq!(servers.len(), 1);
    }

    #[test]
    fn map_servers_uses_base_url() {
        let api = create_test_api();
        let servers = map_servers(&api);

        assert_eq!(servers[0].url, "https://api.test.com/v1");
    }

    #[test]
    fn map_servers_sets_description() {
        let api = create_test_api();
        let servers = map_servers(&api);

        assert!(servers[0].description.is_some());
        assert!(servers[0].description.as_ref().unwrap().contains("TestAPI"));
    }

    // =============================================
    // map_security tests
    // =============================================

    #[test]
    fn map_security_none_returns_none() {
        let auth = AuthStrategy::None;
        assert!(map_security(&auth).is_none());
    }

    #[test]
    fn map_security_bearer_token_returns_http_bearer() {
        let auth = AuthStrategy::BearerToken { header: None };
        let (name, scheme) = map_security(&auth).unwrap();

        assert_eq!(name, "bearerAuth");
        match scheme {
            SecurityScheme::HTTP { scheme, .. } => {
                assert_eq!(scheme, "bearer");
            }
            _ => panic!("Expected HTTP scheme"),
        }
    }

    #[test]
    fn map_security_bearer_token_with_custom_header_returns_api_key() {
        let auth = AuthStrategy::BearerToken {
            header: Some("X-Auth-Token".to_string()),
        };
        let (name, scheme) = map_security(&auth).unwrap();

        assert_eq!(name, "bearerAuth");
        match scheme {
            SecurityScheme::APIKey { location, name, .. } => {
                assert!(matches!(location, openapiv3::APIKeyLocation::Header));
                assert_eq!(name, "X-Auth-Token");
            }
            _ => panic!("Expected APIKey scheme"),
        }
    }

    #[test]
    fn map_security_api_key_returns_api_key_scheme() {
        let auth = AuthStrategy::ApiKey {
            header: "X-API-Key".to_string(),
        };
        let (name, scheme) = map_security(&auth).unwrap();

        assert_eq!(name, "apiKeyAuth");
        match scheme {
            SecurityScheme::APIKey { location, name, .. } => {
                assert!(matches!(location, openapiv3::APIKeyLocation::Header));
                assert_eq!(name, "X-API-Key");
            }
            _ => panic!("Expected APIKey scheme"),
        }
    }

    #[test]
    fn map_security_basic_returns_http_basic() {
        let auth = AuthStrategy::Basic;
        let (name, scheme) = map_security(&auth).unwrap();

        assert_eq!(name, "basicAuth");
        match scheme {
            SecurityScheme::HTTP { scheme, .. } => {
                assert_eq!(scheme, "basic");
            }
            _ => panic!("Expected HTTP scheme"),
        }
    }

    // =============================================
    // map_request_body tests
    // =============================================

    #[test]
    fn map_request_body_json() {
        let request = ApiRequest::json_type("CreateUserBody");
        let body = map_request_body(&request);

        assert!(body.content.contains_key("application/json"));
        assert!(body.required);
    }

    #[test]
    fn map_request_body_form_data() {
        let request =
            ApiRequest::form_data(vec![FormField::file("document"), FormField::text("name")]);
        let body = map_request_body(&request);

        assert!(body.content.contains_key("multipart/form-data"));
    }

    #[test]
    fn map_request_body_url_encoded() {
        let request = ApiRequest::url_encoded(vec![
            FormField::text("username"),
            FormField::text("password"),
        ]);
        let body = map_request_body(&request);

        assert!(
            body.content
                .contains_key("application/x-www-form-urlencoded")
        );
    }

    #[test]
    fn map_request_body_text() {
        let request = ApiRequest::text("text/csv");
        let body = map_request_body(&request);

        assert!(body.content.contains_key("text/csv"));
    }

    #[test]
    fn map_request_body_binary() {
        let request = ApiRequest::binary("application/octet-stream");
        let body = map_request_body(&request);

        assert!(body.content.contains_key("application/octet-stream"));
    }

    // =============================================
    // map_responses tests
    // =============================================

    #[test]
    fn map_responses_json() {
        let response = ApiResponse::json_type("UserResponse");
        let responses = map_responses(&response);

        assert!(responses.responses.contains_key(&StatusCode::Code(200)));
    }

    #[test]
    fn map_responses_text() {
        let response = ApiResponse::Text;
        let responses = map_responses(&response);

        let resp = responses.responses.get(&StatusCode::Code(200)).unwrap();
        if let ReferenceOr::Item(r) = resp {
            assert!(r.content.contains_key("text/plain"));
        }
    }

    #[test]
    fn map_responses_binary() {
        let response = ApiResponse::Binary;
        let responses = map_responses(&response);

        let resp = responses.responses.get(&StatusCode::Code(200)).unwrap();
        if let ReferenceOr::Item(r) = resp {
            assert!(r.content.contains_key("application/octet-stream"));
        }
    }

    #[test]
    fn map_responses_empty() {
        let response = ApiResponse::Empty;
        let responses = map_responses(&response);

        assert!(responses.responses.contains_key(&StatusCode::Code(204)));
    }

    // =============================================
    // extract_path_params tests
    // =============================================

    #[test]
    fn extract_path_params_no_params() {
        let params = extract_path_params("/models");
        assert!(params.is_empty());
    }

    #[test]
    fn extract_path_params_single_param() {
        let params = extract_path_params("/models/{model}");
        assert_eq!(params.len(), 1);

        if let openapiv3::Parameter::Path { parameter_data, .. } = &params[0] {
            assert_eq!(parameter_data.name, "model");
            assert!(parameter_data.required);
        }
    }

    #[test]
    fn extract_path_params_multiple_params() {
        let params = extract_path_params("/users/{user_id}/posts/{post_id}");
        assert_eq!(params.len(), 2);

        let names: Vec<_> = params
            .iter()
            .map(|p| {
                if let openapiv3::Parameter::Path { parameter_data, .. } = p {
                    parameter_data.name.clone()
                } else {
                    String::new()
                }
            })
            .collect();

        assert!(names.contains(&"user_id".to_string()));
        assert!(names.contains(&"post_id".to_string()));
    }

    #[test]
    fn extract_path_params_typed_as_string() {
        let params = extract_path_params("/items/{item_id}");

        if let openapiv3::Parameter::Path { parameter_data, .. } = &params[0] {
            if let openapiv3::ParameterSchemaOrContent::Schema(ReferenceOr::Item(schema)) =
                &parameter_data.format
            {
                match &schema.schema_kind {
                    openapiv3::SchemaKind::Type(openapiv3::Type::String(_)) => {}
                    _ => panic!("Expected string type"),
                }
            }
        }
    }

    // =============================================
    // export integration tests
    // =============================================

    #[test]
    fn export_produces_valid_openapi_version() {
        let api = create_test_api();
        let registry = TestRegistry::new();
        let options = ExportOptions::new();

        let doc = export(&api, &registry, &options).unwrap();
        assert_eq!(doc.openapi, "3.0.3");
    }

    #[test]
    fn export_includes_info() {
        let api = create_test_api();
        let registry = TestRegistry::new();
        let options = ExportOptions::new();

        let doc = export(&api, &registry, &options).unwrap();
        assert_eq!(doc.info.title, "TestAPI");
    }

    #[test]
    fn export_includes_paths() {
        let api = create_test_api_with_endpoints();
        let registry = TestRegistry::new()
            .with_schema("ListResponse")
            .with_schema("Item");
        let options = ExportOptions::new();

        let doc = export(&api, &registry, &options).unwrap();
        assert!(!doc.paths.paths.is_empty());
    }

    #[test]
    fn export_includes_components_schemas() {
        let api = create_test_api();
        let registry = TestRegistry::new()
            .with_schema("TestSchema")
            .with_schema("AnotherSchema");
        let options = ExportOptions::new();

        let doc = export(&api, &registry, &options).unwrap();
        assert!(doc.components.is_some());

        let components = doc.components.unwrap();
        assert!(components.schemas.contains_key("TestSchema"));
        assert!(components.schemas.contains_key("AnotherSchema"));
    }

    #[test]
    fn export_includes_security_schemes() {
        let api = create_test_api(); // Uses BearerToken
        let registry = TestRegistry::new();
        let options = ExportOptions::new();

        let doc = export(&api, &registry, &options).unwrap();
        let components = doc.components.unwrap();

        assert!(components.security_schemes.contains_key("bearerAuth"));
    }

    #[test]
    fn export_includes_vendor_extensions_by_default() {
        let api = create_test_api();
        let registry = TestRegistry::new();
        let options = ExportOptions::new();

        let doc = export(&api, &registry, &options).unwrap();
        assert!(doc.extensions.contains_key("x-schematic"));
    }

    #[test]
    fn export_skips_extensions_when_requested() {
        let api = create_test_api();
        let registry = TestRegistry::new();
        let options = ExportOptions::new().skip_extensions();

        let doc = export(&api, &registry, &options).unwrap();
        assert!(!doc.extensions.contains_key("x-schematic"));
    }

    #[test]
    fn export_includes_external_docs_when_present() {
        let api = create_test_api(); // Has docs_url
        let registry = TestRegistry::new();
        let options = ExportOptions::new();

        let doc = export(&api, &registry, &options).unwrap();
        assert!(doc.external_docs.is_some());
        assert_eq!(doc.external_docs.unwrap().url, "https://docs.test.com");
    }

    #[test]
    fn export_preserves_api_metadata_in_extension() {
        let mut api = create_test_api();
        api.module_path = Some("mymodule".to_string());
        api.request_suffix = Some("Params".to_string());

        let registry = TestRegistry::new();
        let options = ExportOptions::new();

        let doc = export(&api, &registry, &options).unwrap();
        let ext = doc.extensions.get("x-schematic").unwrap();

        assert!(ext.get("module_path").is_some());
        assert!(ext.get("request_suffix").is_some());
    }

    // =============================================
    // Helper functions
    // =============================================

    fn create_test_api() -> RestApi {
        RestApi {
            name: "TestAPI".to_string(),
            description: "Test API description".to_string(),
            base_url: "https://api.test.com/v1".to_string(),
            docs_url: Some("https://docs.test.com".to_string()),
            auth: AuthStrategy::BearerToken { header: None },
            env_auth: vec!["TEST_API_KEY".to_string()],
            env_username: None,
            headers: vec![],
            endpoints: vec![],
            module_path: None,
            request_suffix: None,
            env_mapping: Some(EnvMapping {
                bearer_token: Some(EnvList::single("TEST_API_KEY")),
                ..Default::default()
            }),
        }
    }

    fn create_test_api_with_endpoints() -> RestApi {
        RestApi {
            name: "TestAPI".to_string(),
            description: "Test API description".to_string(),
            base_url: "https://api.test.com/v1".to_string(),
            docs_url: None,
            auth: AuthStrategy::BearerToken { header: None },
            env_auth: vec!["TEST_API_KEY".to_string()],
            env_username: None,
            headers: vec![],
            endpoints: vec![
                Endpoint {
                    id: "ListItems".to_string(),
                    method: RestMethod::Get,
                    path: "/items".to_string(),
                    description: "List all items".to_string(),
                    request: None,
                    response: ApiResponse::json_type("ListResponse"),
                    headers: vec![],
                    params: None,
                },
                Endpoint {
                    id: "GetItem".to_string(),
                    method: RestMethod::Get,
                    path: "/items/{item_id}".to_string(),
                    description: "Get a specific item".to_string(),
                    request: None,
                    response: ApiResponse::json_type("Item"),
                    headers: vec![],
                    params: None,
                },
            ],
            module_path: None,
            request_suffix: None,
            env_mapping: None,
        }
    }
}
