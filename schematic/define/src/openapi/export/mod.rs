//! OpenAPI export functionality.
//!
//! This module provides the core export logic for transforming `RestApi` definitions
//! into OpenAPI 3.0.3 documents with vendor extensions for round-trip fidelity.
//!
//! ## Examples
//!
//! ```text
//! use schematic_define::openapi::{export, ExportOptions};
//! use schematic_definitions::openai::{define_openai_api, openapi_registry};
//!
//! let api = define_openai_api();
//! let registry = openapi_registry();
//! let options = ExportOptions::new().with_version("1.0.0");
//!
//! let openapi_doc = export(&api, &registry, &options)?;
//! ```

mod components;
mod info;
mod paths;
mod request_body;
mod responses;
mod security;
mod validate;

use indexmap::IndexMap;
use openapiv3::OpenAPI;

use super::OpenApiError;
use super::extensions::SchematicDocExtension;
use super::options::ExportOptions;
use crate::types::RestApi;

use components::map_components;
use info::{map_info, map_servers};
use paths::map_paths;
use security::map_security_requirements;
use validate::validate_ref_closure;

pub use components::SchemaRegistryLike;
pub use paths::extract_path_params;
pub use security::map_security;

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

    let openapi = OpenAPI {
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
    };

    validate_ref_closure(&openapi)?;

    Ok(openapi)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::AuthStrategy;
    use crate::headers::{EnvList, EnvMapping};
    use crate::request::ApiRequest;
    use crate::response::ApiResponse;
    use crate::types::{Endpoint, RestMethod};

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

    fn create_test_api() -> RestApi {
        RestApi {
            name: "TestAPI".to_string(),
            description: "Test API description".to_string(),
            base_url: "https://api.test.com/v1".to_string(),
            docs_url: Some("https://docs.test.com".to_string()),
            auth: AuthStrategy::BearerToken { header: None },
            auth_policy: None,
            env_auth: vec!["TEST_API_KEY".to_string()],
            env_username: None,
            headers: vec![],
            endpoints: vec![],
            module_path: None,
            request_suffix: None,
            version: None,
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
            auth_policy: None,
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
                    oauth_scopes: None,
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
                    oauth_scopes: None,
                },
            ],
            module_path: None,
            request_suffix: None,
            version: None,
            env_mapping: None,
        }
    }

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
        let api = create_test_api();
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
        let api = create_test_api();
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

    #[test]
    fn export_fails_on_unresolved_request_schema_ref() {
        let api = RestApi {
            name: "TestAPI".to_string(),
            description: "Test".to_string(),
            base_url: "https://api.test.com/v1".to_string(),
            docs_url: None,
            auth: AuthStrategy::None,
            auth_policy: None,
            env_auth: vec![],
            env_username: None,
            headers: vec![],
            endpoints: vec![Endpoint {
                id: "Create".to_string(),
                method: RestMethod::Post,
                path: "/create".to_string(),
                description: "Create something".to_string(),
                request: Some(ApiRequest::json_type("MissingBody")),
                response: ApiResponse::Empty,
                headers: vec![],
                params: None,
                oauth_scopes: None,
            }],
            module_path: None,
            request_suffix: None,
            version: None,
            env_mapping: None,
        };
        let registry = TestRegistry::new();
        let options = ExportOptions::new();

        let result = export(&api, &registry, &options);
        assert!(result.is_err());
        let err = result.unwrap_err();
        let err_str = err.to_string();
        assert!(
            err_str.contains("MissingBody"),
            "error should mention MissingBody: {}",
            err_str
        );
    }

    #[test]
    fn export_fails_on_unresolved_response_schema_ref() {
        let api = RestApi {
            name: "TestAPI".to_string(),
            description: "Test".to_string(),
            base_url: "https://api.test.com/v1".to_string(),
            docs_url: None,
            auth: AuthStrategy::None,
            auth_policy: None,
            env_auth: vec![],
            env_username: None,
            headers: vec![],
            endpoints: vec![Endpoint {
                id: "Get".to_string(),
                method: RestMethod::Get,
                path: "/get".to_string(),
                description: "Get something".to_string(),
                request: None,
                response: ApiResponse::json_type("MissingResponse"),
                headers: vec![],
                params: None,
                oauth_scopes: None,
            }],
            module_path: None,
            request_suffix: None,
            version: None,
            env_mapping: None,
        };
        let registry = TestRegistry::new();
        let options = ExportOptions::new();

        let result = export(&api, &registry, &options);
        assert!(result.is_err());
        let err = result.unwrap_err();
        let err_str = err.to_string();
        assert!(
            err_str.contains("MissingResponse"),
            "error should mention MissingResponse: {}",
            err_str
        );
    }

    #[test]
    fn export_strips_reserved_marker_from_path_and_params() {
        let api = RestApi {
            name: "TestAPI".to_string(),
            description: "Test".to_string(),
            base_url: "https://api.test.com".to_string(),
            docs_url: None,
            auth: AuthStrategy::None,
            auth_policy: None,
            env_auth: vec![],
            env_username: None,
            headers: vec![],
            endpoints: vec![Endpoint {
                id: "GetModel".to_string(),
                method: RestMethod::Get,
                path: "/models/{+repo_id}".to_string(),
                description: "Get a model".to_string(),
                request: None,
                response: ApiResponse::json_type("Model"),
                headers: vec![],
                params: None,
                oauth_scopes: None,
            }],
            module_path: None,
            request_suffix: None,
            version: None,
            env_mapping: None,
        };
        let registry = TestRegistry::new().with_schema("Model");
        let options = ExportOptions::new();

        let doc = export(&api, &registry, &options).unwrap();

        // The path key renders as valid `{repo_id}`, never `{+repo_id}`.
        assert!(doc.paths.paths.contains_key("/models/{repo_id}"));
        assert!(!doc.paths.paths.contains_key("/models/{+repo_id}"));

        // No `{+` may leak anywhere in the serialized document.
        let json = serde_json::to_string(&doc).unwrap();
        assert!(!json.contains("{+"), "reserved marker leaked: {json}");

        if let openapiv3::ReferenceOr::Item(path_item) = &doc.paths.paths["/models/{repo_id}"] {
            if let openapiv3::Parameter::Path { parameter_data, .. } =
                path_item.parameters[0].as_item().unwrap()
            {
                assert_eq!(parameter_data.name, "repo_id");
            } else {
                panic!("expected a path parameter");
            }
        } else {
            panic!("expected an inline path item");
        }
    }

    #[test]
    fn export_succeeds_when_all_refs_are_resolved() {
        let api = RestApi {
            name: "TestAPI".to_string(),
            description: "Test".to_string(),
            base_url: "https://api.test.com/v1".to_string(),
            docs_url: None,
            auth: AuthStrategy::None,
            auth_policy: None,
            env_auth: vec![],
            env_username: None,
            headers: vec![],
            endpoints: vec![Endpoint {
                id: "Create".to_string(),
                method: RestMethod::Post,
                path: "/create".to_string(),
                description: "Create something".to_string(),
                request: Some(ApiRequest::json_type("RequestBody")),
                response: ApiResponse::json_type("ResponseBody"),
                headers: vec![],
                params: None,
                oauth_scopes: None,
            }],
            module_path: None,
            request_suffix: None,
            version: None,
            env_mapping: None,
        };
        let registry = TestRegistry::new()
            .with_schema("RequestBody")
            .with_schema("ResponseBody");
        let options = ExportOptions::new();

        let result = export(&api, &registry, &options);
        assert!(
            result.is_ok(),
            "export should succeed when all refs are resolved"
        );
    }
}
