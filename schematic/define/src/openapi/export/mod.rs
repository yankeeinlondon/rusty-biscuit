//! OpenAPI export functionality.
//!
//! This module provides the core export logic for transforming `RestApi` definitions
//! into OpenAPI 3.0.3 documents with vendor extensions for round-trip fidelity.
//!
//! ## Examples
//!
//! ```text
//! use schematic_define::openapi::{export, ExportOptions};
//! use schematic_definitions::openai::define_openai_api;
//!
//! let api = define_openai_api();
//! let options = ExportOptions::new().with_version("1.0.0");
//!
//! let openapi_doc = export(&api, &options)?;
//! ```

mod components;
mod conversion;
mod info;
mod paths;
mod parameters;
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
/// - `api` - The REST API definition to export; `components.schemas` comes
///   from [`RestApi::schemas`]
/// - `options` - Export configuration options
///
/// ## Returns
///
/// An OpenAPI 3.0.3 document.
///
/// ## Errors
///
/// Returns `OpenApiError` if the export fails due to validation issues.
pub fn export(
    api: &RestApi,
    options: &ExportOptions,
) -> Result<OpenAPI, OpenApiError> {
    let info = map_info(api, options);
    let servers = map_servers(api);
    let mut components = map_components(api, options);
    // Endpoint auth registers override-only schemes after the API's own.
    let paths = map_paths(api, options, &mut components.security_schemes)?;
    let security = map_security_requirements(api);

    let mut extensions = IndexMap::new();
    if !options.skip_extensions {
        let doc_ext = SchematicDocExtension {
            module_path: api.module_path.clone(),
            request_suffix: api.request_suffix.clone(),
            env_mapping: api.env_mapping.clone(),
            headers: api.headers.clone(),
            rate_limit: api.rate_limit.clone(),
            retry_guidance: api.retry_guidance.clone(),
            errors: api.errors.clone(),
            cross_origin: api.cross_origin.clone(),
            request_id_header: api.request_id_header.clone(),
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
    use crate::cross_origin::CrossOrigin;
    use crate::headers::{EnvList, EnvMapping};
    use crate::request::ApiRequest;
    use crate::response::SuccessShape;
    use crate::types::{Endpoint, RestMethod};

    /// Give `api` one object component per name.
    fn with_components(mut api: RestApi, names: &[&str]) -> RestApi {
        api.schemas = names.iter().map(|name| crate::SchemaComponent {
            name: (*name).to_string(),
            rust_path: Some(format!("::models::{name}")),
            schema: serde_json::json!({"description": format!("{name} schema"), "type": "object"}),
        }).collect();
        api
    }

    fn create_test_api() -> RestApi {
        RestApi { rate_limit: None, retry_guidance: None, errors: vec![], graphql: None,
            name: "TestAPI".to_string(),
            description: "Test API description".to_string(),
            base_url: "https://api.test.com/v1".to_string(),
            docs_url: Some("https://docs.test.com".to_string()),
            auth: AuthStrategy::BearerToken { header: None },
            auth_policy: None,
            env_auth: vec!["TEST_API_KEY".to_string()],
            headers: vec![],
            endpoints: vec![],
            module_path: None,
            request_suffix: None,
            version: None,
            env_mapping: Some(EnvMapping {
                bearer_token: Some(EnvList::single("TEST_API_KEY")),
                ..Default::default()
            }),
            cross_origin: CrossOrigin::default(),
            request_id_header: None,
            secret_names: vec![],
            default_pagination: None,
            schemas: vec![],
            }
    }

    fn create_test_api_with_endpoints() -> RestApi {
        RestApi { rate_limit: None, retry_guidance: None, errors: vec![], graphql: None,
            name: "TestAPI".to_string(),
            description: "Test API description".to_string(),
            base_url: "https://api.test.com/v1".to_string(),
            docs_url: None,
            auth: AuthStrategy::BearerToken { header: None },
            auth_policy: None,
            env_auth: vec!["TEST_API_KEY".to_string()],
            headers: vec![],
            endpoints: vec![
                Endpoint { rate_limit: None, retry_guidance: None, deprecated: false, auth: None, errors: vec![], no_declared_errors: false,
                    id: "ListItems".to_string(),
                    method: RestMethod::Get,
                    path: "/items".to_string(),
                    description: "List all items".to_string(),
                    request: None,
                    response: vec![SuccessShape::json_type("ListResponse")],
                    headers: vec![],
                    params: None,
                    cross_origin: None, pagination: None,
                },
                Endpoint { rate_limit: None, retry_guidance: None, deprecated: false, auth: None, errors: vec![], no_declared_errors: false,
                    id: "GetItem".to_string(),
                    method: RestMethod::Get,
                    path: "/items/{item_id}".to_string(),
                    description: "Get a specific item".to_string(),
                    request: None,
                    response: vec![SuccessShape::json_type("Item")],
                    headers: vec![],
                    params: None,
                    cross_origin: None, pagination: None,
                },
            ],
            module_path: None,
            request_suffix: None,
            version: None,
            env_mapping: None,
            cross_origin: CrossOrigin::default(),
            request_id_header: None,
            secret_names: vec![],
            default_pagination: None,
            schemas: vec![],
            }
    }

    #[test]
    fn export_produces_valid_openapi_version() {
        let api = create_test_api();
        let options = ExportOptions::new();

        let doc = export(&api, &options).unwrap();
        assert_eq!(doc.openapi, "3.0.3");
    }

    #[test]
    fn export_includes_info() {
        let api = create_test_api();
        let options = ExportOptions::new();

        let doc = export(&api, &options).unwrap();
        assert_eq!(doc.info.title, "TestAPI");
    }

    #[test]
    fn export_includes_paths() {
        let api = create_test_api_with_endpoints();
        let api = with_components(api, &["ListResponse", "Item"]);
        let options = ExportOptions::new();

        let doc = export(&api, &options).unwrap();
        assert!(!doc.paths.paths.is_empty());
    }

    #[test]
    fn export_includes_components_schemas() {
        let api = create_test_api();
        let api = with_components(api, &["TestSchema", "AnotherSchema"]);
        let options = ExportOptions::new();

        let doc = export(&api, &options).unwrap();
        assert!(doc.components.is_some());

        let components = doc.components.unwrap();
        assert!(components.schemas.contains_key("TestSchema"));
        assert!(components.schemas.contains_key("AnotherSchema"));
    }

    #[test]
    fn export_includes_security_schemes() {
        let api = create_test_api();
        let options = ExportOptions::new();

        let doc = export(&api, &options).unwrap();
        let components = doc.components.unwrap();

        assert!(components.security_schemes.contains_key("bearerAuth"));
    }

    #[test]
    fn export_includes_vendor_extensions_by_default() {
        let api = create_test_api();
        let options = ExportOptions::new();

        let doc = export(&api, &options).unwrap();
        assert!(doc.extensions.contains_key("x-schematic"));
    }

    #[test]
    fn export_skips_extensions_when_requested() {
        let api = create_test_api();
        let options = ExportOptions::new().skip_extensions();

        let doc = export(&api, &options).unwrap();
        assert!(!doc.extensions.contains_key("x-schematic"));
    }

    #[test]
    fn export_includes_external_docs_when_present() {
        let api = create_test_api();
        let options = ExportOptions::new();

        let doc = export(&api, &options).unwrap();
        assert!(doc.external_docs.is_some());
        assert_eq!(doc.external_docs.unwrap().url, "https://docs.test.com");
    }

    #[test]
    fn export_preserves_api_metadata_in_extension() {
        let mut api = create_test_api();
        api.module_path = Some("mymodule".to_string());
        api.request_suffix = Some("Params".to_string());

        let options = ExportOptions::new();

        let doc = export(&api, &options).unwrap();
        let ext = doc.extensions.get("x-schematic").unwrap();

        assert!(ext.get("module_path").is_some());
        assert!(ext.get("request_suffix").is_some());
    }

    #[test]
    fn export_fails_on_unresolved_request_schema_ref() {
        let api = RestApi { rate_limit: None, retry_guidance: None, errors: vec![], graphql: None,
            name: "TestAPI".to_string(),
            description: "Test".to_string(),
            base_url: "https://api.test.com/v1".to_string(),
            docs_url: None,
            auth: AuthStrategy::None,
            auth_policy: None,
            env_auth: vec![],
            headers: vec![],
            endpoints: vec![Endpoint { rate_limit: None, retry_guidance: None, deprecated: false, auth: None, errors: vec![], no_declared_errors: false,
                id: "Create".to_string(),
                method: RestMethod::Post,
                path: "/create".to_string(),
                description: "Create something".to_string(),
                request: Some(ApiRequest::json_type("MissingBody")),
                response: vec![SuccessShape::empty()],
                headers: vec![],
                params: None,
                cross_origin: None, pagination: None,
            }],
            module_path: None,
            request_suffix: None,
            version: None,
            env_mapping: None,
            cross_origin: CrossOrigin::default(),
            request_id_header: None,
            secret_names: vec![],
            default_pagination: None,
            schemas: vec![],
            };
        let options = ExportOptions::new();

        let result = export(&api, &options);
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
        let api = RestApi { rate_limit: None, retry_guidance: None, errors: vec![], graphql: None,
            name: "TestAPI".to_string(),
            description: "Test".to_string(),
            base_url: "https://api.test.com/v1".to_string(),
            docs_url: None,
            auth: AuthStrategy::None,
            auth_policy: None,
            env_auth: vec![],
            headers: vec![],
            endpoints: vec![Endpoint { rate_limit: None, retry_guidance: None, deprecated: false, auth: None, errors: vec![], no_declared_errors: false,
                id: "Get".to_string(),
                method: RestMethod::Get,
                path: "/get".to_string(),
                description: "Get something".to_string(),
                request: None,
                response: vec![SuccessShape::json_type("MissingResponse")],
                headers: vec![],
                params: None,
                cross_origin: None, pagination: None,
            }],
            module_path: None,
            request_suffix: None,
            version: None,
            env_mapping: None,
            cross_origin: CrossOrigin::default(),
            request_id_header: None,
            secret_names: vec![],
            default_pagination: None,
            schemas: vec![],
            };
        let options = ExportOptions::new();

        let result = export(&api, &options);
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
        let api = RestApi { rate_limit: None, retry_guidance: None, errors: vec![], graphql: None,
            name: "TestAPI".to_string(),
            description: "Test".to_string(),
            base_url: "https://api.test.com".to_string(),
            docs_url: None,
            auth: AuthStrategy::None,
            auth_policy: None,
            env_auth: vec![],
            headers: vec![],
            endpoints: vec![Endpoint { rate_limit: None, retry_guidance: None, deprecated: false, auth: None, errors: vec![], no_declared_errors: false,
                id: "GetModel".to_string(),
                method: RestMethod::Get,
                path: "/models/{+repo_id}".to_string(),
                description: "Get a model".to_string(),
                request: None,
                response: vec![SuccessShape::json_type("Model")],
                headers: vec![],
                params: None,
                cross_origin: None, pagination: None,
            }],
            module_path: None,
            request_suffix: None,
            version: None,
            env_mapping: None,
            cross_origin: CrossOrigin::default(),
            request_id_header: None,
            secret_names: vec![],
            default_pagination: None,
            schemas: vec![],
            };
        let api = with_components(api, &["Model"]);
        let options = ExportOptions::new();

        let doc = export(&api, &options).unwrap();

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
        let api = RestApi { rate_limit: None, retry_guidance: None, errors: vec![], graphql: None,
            name: "TestAPI".to_string(),
            description: "Test".to_string(),
            base_url: "https://api.test.com/v1".to_string(),
            docs_url: None,
            auth: AuthStrategy::None,
            auth_policy: None,
            env_auth: vec![],
            headers: vec![],
            endpoints: vec![Endpoint { rate_limit: None, retry_guidance: None, deprecated: false, auth: None, errors: vec![], no_declared_errors: false,
                id: "Create".to_string(),
                method: RestMethod::Post,
                path: "/create".to_string(),
                description: "Create something".to_string(),
                request: Some(ApiRequest::json_type("RequestBody")),
                response: vec![SuccessShape::json_type("ResponseBody")],
                headers: vec![],
                params: None,
                cross_origin: None, pagination: None,
            }],
            module_path: None,
            request_suffix: None,
            version: None,
            env_mapping: None,
            cross_origin: CrossOrigin::default(),
            request_id_header: None,
            secret_names: vec![],
            default_pagination: None,
            schemas: vec![],
            };
        let api = with_components(api, &["RequestBody", "ResponseBody"]);
        let options = ExportOptions::new();

        let result = export(&api, &options);
        assert!(
            result.is_ok(),
            "export should succeed when all refs are resolved"
        );
    }

    /// Guards the wiring, not just the mapping: `map_parameters` had complete
    /// unit coverage while `map_operation` still hardcoded `parameters: vec![]`,
    /// so every exported spec shipped without query parameters.
    #[test]
    fn exported_operations_carry_declared_query_parameters() {
        use crate::params::{EndpointParams, ParamDef, ParamStyle, QueryParamType};
        use openapiv3::ReferenceOr;

        let mut api = create_test_api();
        api.endpoints = vec![Endpoint { rate_limit: None, retry_guidance: None, deprecated: false, auth: None, errors: vec![], no_declared_errors: false,
            id: "ListItems".to_string(),
            method: RestMethod::Get,
            path: "/items".to_string(),
            description: "List items".to_string(),
            request: None,
            response: vec![SuccessShape::empty()],
            headers: vec![],
            params: Some(EndpointParams {
                query: vec![ParamDef { deprecated: false, rust_name: None, secret: false, pager_managed: false,
                    name: "limit".to_string(),
                    required: false,
                    description: Some("Page size".to_string()),
                    param_type: QueryParamType::Integer,
                    explode: false,
                    style: ParamStyle::Form,
                }],
                ..Default::default()
            }),
            cross_origin: None, pagination: None,
        }];

        let doc = export(&api, &ExportOptions::new())
            .expect("export should succeed");

        let ReferenceOr::Item(path_item) = &doc.paths.paths["/items"] else {
            panic!("expected an inline path item");
        };
        let operation = path_item.get.as_ref().expect("GET operation");

        assert_eq!(
            operation.parameters.len(),
            1,
            "declared query parameters must reach the exported operation"
        );

        let ReferenceOr::Item(openapiv3::Parameter::Query { parameter_data, .. }) =
            &operation.parameters[0]
        else {
            panic!("expected an inline query parameter");
        };
        assert_eq!(parameter_data.name, "limit");
        assert_eq!(parameter_data.description.as_deref(), Some("Page size"));
    }

    /// OpenAPI forbids duplicate entries in a parameter list; two methods on one
    /// path each contribute the same path parameters.
    #[test]
    fn path_parameters_are_declared_once_per_path() {
        use openapiv3::ReferenceOr;

        let mut api = create_test_api();
        api.endpoints = vec![
            Endpoint { rate_limit: None, retry_guidance: None, deprecated: false, auth: None, errors: vec![], no_declared_errors: false,
                id: "GetItem".to_string(),
                method: RestMethod::Get,
                path: "/items/{item_id}".to_string(),
                description: "Get".to_string(),
                request: None,
                response: vec![SuccessShape::empty()],
                headers: vec![],
                params: None,
                cross_origin: None, pagination: None,
            },
            Endpoint { rate_limit: None, retry_guidance: None, deprecated: false, auth: None, errors: vec![], no_declared_errors: false,
                id: "DeleteItem".to_string(),
                method: RestMethod::Delete,
                path: "/items/{item_id}".to_string(),
                description: "Delete".to_string(),
                request: None,
                response: vec![SuccessShape::empty()],
                headers: vec![],
                params: None,
                cross_origin: None, pagination: None,
            },
        ];

        let doc = export(&api, &ExportOptions::new())
            .expect("export should succeed");

        let ReferenceOr::Item(path_item) = &doc.paths.paths["/items/{item_id}"] else {
            panic!("expected an inline path item");
        };

        assert_eq!(path_item.parameters.len(), 1, "item_id declared twice");
    }
    #[test]
    fn typed_builder_exports_named_wrapped_request_response_and_form_schemas() {
        use crate::{DefinitionContext, Success, FormField};
        use std::{collections::HashMap, time::Duration};
        let mut ctx = DefinitionContext::types_in("::models");
        let desc = ctx.register::<Vec<Duration>>().unwrap();
        let mut json = Endpoint::post("Json", "/json").describes("Json")
            .returns_json::<Option<Vec<Duration>>>(&mut ctx)
            .request_json::<HashMap<String, Duration>>(&mut ctx).build().unwrap();
        json.headers.push(("x-fixture".into(), "one".into()));
        let form = Endpoint::post("Form", "/form").describes("Form")
            .returns(Success::empty()).request_form(vec![FormField::json_desc("metadata", desc)
                .optional().with_description("metadata")]).build().unwrap();
        let api = RestApi::builder(&ctx, "Typed", "https://example.com").describes("Typed")
            .no_auth().endpoints([json, form]).build().unwrap();
        assert_eq!(api.schemas.iter().map(|c| c.name.as_str()).collect::<Vec<_>>(), ["Duration"]);
        let doc = export(&api, &ExportOptions::new()).unwrap();
        let value = serde_json::to_value(doc).unwrap();
        let response = &value["paths"]["/json"]["post"]["responses"]["200"]["content"]["application/json"]["schema"];
        assert_eq!(response["anyOf"][0]["type"], "array");
        assert_eq!(response["anyOf"][1]["nullable"], true);
        assert_eq!(response["anyOf"][0]["items"]["$ref"], "#/components/schemas/Duration");
        let request = &value["paths"]["/json"]["post"]["requestBody"]["content"]["application/json"]["schema"];
        assert_eq!(request["additionalProperties"]["$ref"], "#/components/schemas/Duration");
        let form = &value["paths"]["/form"]["post"]["requestBody"]["content"]["multipart/form-data"]["schema"];
        assert_eq!(form["properties"]["metadata"]["allOf"][0]["items"]["$ref"], "#/components/schemas/Duration");
        assert!(form["required"].as_array().is_none_or(|v| v.is_empty()));
        let mut without_components = api.clone();
        without_components.schemas.clear();
        assert!(matches!(export(&without_components, &ExportOptions::new()), Err(OpenApiError::UnresolvedRefs { .. })));
    }


    // Schema-only fixtures: nothing constructs or reads them.
    #[allow(dead_code)]
    mod composed {
        use schemars::JsonSchema;
        crate::definition_type!(#[derive(JsonSchema)] pub struct Cat { pub meows: bool });
        crate::definition_type!(#[derive(JsonSchema)] pub struct Dog { pub barks: u8 });
        crate::definition_type!(#[derive(JsonSchema)] #[serde(untagged)] pub enum Pet { Cat(Cat), Dog(Dog) });
        crate::definition_type!(#[derive(JsonSchema)] #[serde(untagged)] pub enum Either { Cat(Cat), Dog(Dog) });
        crate::definition_type!(#[derive(JsonSchema)] pub struct Both { pub meows: bool, pub barks: u8 });
    }

    #[test]
    fn composed_components_export_as_compositions_of_references() {
        use crate::DefinitionContext;
        use composed::*;
        let mut ctx = DefinitionContext::types_in("::models");
        let cat = ctx.register::<Cat>().unwrap();
        let dog = ctx.register::<Dog>().unwrap();
        ctx.one_of::<Pet>([cat.clone(), dog.clone()]).unwrap();
        ctx.any_of::<Either>([cat.clone(), dog.clone()]).unwrap();
        ctx.all_of::<Both>([cat, dog]).unwrap();
        let endpoint = Endpoint::post("Adopt", "/pets").describes("Adopt")
            .request_json::<Pet>(&mut ctx).returns_json::<Both>(&mut ctx).build().unwrap();
        let api = RestApi::builder(&ctx, "Pets", "https://example.com").describes("Pets").no_auth()
            .endpoints([endpoint]).build().unwrap();

        let doc = serde_json::to_value(export(&api, &ExportOptions::new()).unwrap()).unwrap();
        let schemas = &doc["components"]["schemas"];
        let refs = serde_json::json!([{"$ref": "#/components/schemas/Cat"}, {"$ref": "#/components/schemas/Dog"}]);
        assert_eq!(schemas["Pet"], serde_json::json!({"oneOf": refs}));
        assert_eq!(schemas["Either"], serde_json::json!({"anyOf": refs}));
        assert_eq!(schemas["Both"], serde_json::json!({"allOf": refs}));
        assert_eq!(schemas["Cat"]["type"], "object");
        let body = &doc["paths"]["/pets"]["post"]["requestBody"]["content"]["application/json"]["schema"];
        assert_eq!(body["$ref"], "#/components/schemas/Pet");
    }

    #[test]
    fn metadata_reaches_the_document_and_only_overriding_operations() {
        use crate::metadata::{Backoff, RateLimit, ResetUnit, RetryGuidance, StatusSelector};
        let api_limit = RateLimit::new("x-limit", "x-remaining", "x-reset", ResetUnit::EpochSeconds).unwrap();
        let own_retry = RetryGuidance::new([StatusSelector::Code(503)], false, 2, Backoff::None).unwrap();
        let ctx = crate::DefinitionContext::types_in("::models");
        let api = RestApi::builder(&ctx, "Meta", "https://example.com").describes("Meta").no_auth()
            .rate_limit(api_limit)
            .endpoints([
                Endpoint::get("Inherit", "/a").describes("A").returns_empty().build().unwrap(),
                Endpoint::get("Own", "/b").describes("B").returns_empty().retry_guidance(own_retry).build().unwrap(),
            ])
            .build().unwrap();

        let doc = serde_json::to_value(export(&api, &ExportOptions::new()).unwrap()).unwrap();
        assert_eq!(doc["x-schematic"]["rate_limit"]["reset_unit"], "epoch_seconds");
        assert!(doc["x-schematic"].get("retry_guidance").is_none());
        assert_eq!(doc["paths"]["/a"]["get"]["x-schematic"], serde_json::json!({"response": "Empty"}));
        assert_eq!(doc["paths"]["/b"]["get"]["x-schematic"]["retry_guidance"]["retry_on"], serde_json::json!([{"code": 503}]));
        assert!(doc["paths"]["/b"]["get"]["x-schematic"].get("rate_limit").is_none());

        let bare = serde_json::to_value(export(&api, &ExportOptions::new().skip_extensions()).unwrap()).unwrap();
        assert!(!bare.to_string().contains("x-schematic"));
    }
}
