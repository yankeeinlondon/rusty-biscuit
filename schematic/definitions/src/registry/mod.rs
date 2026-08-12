//! Schema registry for OpenAPI export.
//!
//! This module provides a registry for collecting and managing JSON schemas
//! derived from Rust types using schemars. The registry can then be converted
//! to OpenAPI schema format for API documentation generation.
//!
//! ## Examples
//!
//! ```
//! use schematic_definitions::registry::SchemaRegistry;
//! use schematic_definitions::openai::{Model, ListModelsResponse, DeleteModelResponse};
//!
//! let registry = SchemaRegistry::new()
//!     .register::<Model>("Model")
//!     .register::<ListModelsResponse>("ListModelsResponse")
//!     .register::<DeleteModelResponse>("DeleteModelResponse");
//!
//! assert_eq!(registry.len(), 3);
//! assert!(registry.get("Model").is_some());
//! ```

mod conversion;
mod lookup;

pub use lookup::*;

use indexmap::IndexMap;
use schemars::generate::SchemaSettings;
use schemars::{JsonSchema, Schema};

/// A registry of JSON schemas for API response types.
///
/// The registry collects schemas generated from Rust types and can convert
/// them to OpenAPI schema format. It uses an `IndexMap` to preserve insertion
/// order, which is important for deterministic output.
#[derive(Debug, Default)]
pub struct SchemaRegistry {
    types: IndexMap<String, Schema>,
}

impl SchemaRegistry {
    /// Creates a new empty schema registry.
    ///
    /// ## Examples
    ///
    /// ```
    /// use schematic_definitions::registry::SchemaRegistry;
    ///
    /// let registry = SchemaRegistry::new();
    /// assert!(registry.is_empty());
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a type's JSON schema in the registry.
    ///
    /// The type must implement `schemars::JsonSchema`. The schema is generated
    /// using OpenAPI 3.0 compatible settings.
    ///
    /// ## Examples
    ///
    /// ```
    /// use schematic_definitions::registry::SchemaRegistry;
    /// use schematic_definitions::openai::Model;
    ///
    /// let registry = SchemaRegistry::new()
    ///     .register::<Model>("Model");
    ///
    /// assert!(registry.get("Model").is_some());
    /// ```
    #[must_use]
    pub fn register<T: JsonSchema>(mut self, name: &str) -> Self {
        // Use OpenAPI 3.0 settings for maximum compatibility
        let generator = SchemaSettings::openapi3().into_generator();
        let schema = generator.into_root_schema_for::<T>();
        self.types.insert(name.to_string(), schema);
        self
    }

    /// Retrieves a schema by name.
    ///
    /// ## Returns
    ///
    /// `Some(&Schema)` if the schema exists, `None` otherwise.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&Schema> {
        self.types.get(name)
    }

    /// Returns the number of registered schemas.
    #[must_use]
    pub fn len(&self) -> usize {
        self.types.len()
    }

    /// Returns `true` if no schemas are registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.types.is_empty()
    }

    /// Returns an iterator over registered schema names.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.types.keys().map(String::as_str)
    }

    /// Merges another registry into this one.
    ///
    /// Entries from `other` are inserted into `self.types` using
    /// `IndexMap::insert`, which overwrites duplicates while preserving
    /// insertion order. This produces the union of schemas suitable for
    /// module-grouped OpenAPI export — identical schemas registered under the
    /// same name collapse to a single entry.
    pub fn extend(&mut self, other: SchemaRegistry) {
        for (name, schema) in other.types {
            self.types.insert(name, schema);
        }
    }

    /// Converts all registered schemas to OpenAPI 3.0 schema format.
    ///
    /// This method transforms the schemars `Schema` objects into
    /// `openapiv3::Schema` objects suitable for inclusion in an OpenAPI spec.
    ///
    /// ## Examples
    ///
    /// ```
    /// use schematic_definitions::registry::SchemaRegistry;
    /// use schematic_definitions::openai::Model;
    ///
    /// let registry = SchemaRegistry::new()
    ///     .register::<Model>("Model");
    ///
    /// let openapi_schemas = registry.to_openapi_schemas();
    /// assert!(openapi_schemas.contains_key("Model"));
    /// ```
    #[must_use]
    pub fn to_openapi_schemas(&self) -> IndexMap<String, openapiv3::Schema> {
        let mut result = IndexMap::new();

        for (name, schema) in &self.types {
            let openapi_schema = conversion::convert_schema_to_openapi(schema);
            result.insert(name.clone(), openapi_schema);

            // Recursively extract all nested $defs from the schemars JSON.
            let json_value = schema.as_value();
            Self::extract_defs_recursive(json_value, &mut result);
        }

        result
    }

    /// Recursively extracts all `$defs` from a JSON schema value and converts
    /// them to OpenAPI schemas, inserting them into `result`.
    fn extract_defs_recursive(
        value: &serde_json::Value,
        result: &mut IndexMap<String, openapiv3::Schema>,
    ) {
        if let Some(obj) = value.as_object() {
            // Extract $defs at this level
            if let Some(defs) = obj.get("$defs").and_then(|v| v.as_object()) {
                for (def_name, def_schema) in defs {
                    if !result.contains_key(def_name) {
                        result.insert(
                            def_name.clone(),
                            conversion::convert_json_schema_to_openapi(def_schema),
                        );
                        // Recurse into the def itself
                        Self::extract_defs_recursive(def_schema, result);
                    }
                }
            }

            // Extract components.schemas (used by schemars OpenAPI 3.0 settings)
            if let Some(components) = obj.get("components").and_then(|v| v.as_object())
                && let Some(schemas) = components.get("schemas").and_then(|v| v.as_object())
            {
                for (schema_name, schema_value) in schemas {
                    if !result.contains_key(schema_name) {
                        result.insert(
                            schema_name.clone(),
                            conversion::convert_json_schema_to_openapi(schema_value),
                        );
                        // Recurse into the schema itself
                        Self::extract_defs_recursive(schema_value, result);
                    }
                }
            }

            // Recurse into property values, array items, and allOf/anyOf/oneOf
            for (_key, val) in obj {
                Self::extract_defs_recursive(val, result);
            }
        } else if let Some(arr) = value.as_array() {
            for val in arr {
                Self::extract_defs_recursive(val, result);
            }
        }
    }

    /// Validates that all schema names referenced by an API are registered.
    ///
    /// This method checks the API definition to find all response schema names
    /// and verifies they exist in the registry.
    ///
    /// ## Returns
    ///
    /// `Ok(())` if all schemas are registered, `Err(Vec<String>)` containing
    /// the names of missing schemas otherwise.
    ///
    /// ## Examples
    ///
    /// ```
    /// use schematic_definitions::registry::SchemaRegistry;
    /// use schematic_definitions::openai::{define_openai_api, openapi_registry, Model};
    ///
    /// let api = define_openai_api();
    ///
    /// // The generated registry covers every type the endpoints name
    /// assert!(openapi_registry().validate_completeness(&api).is_ok());
    ///
    /// // Incomplete registry fails validation
    /// let incomplete_registry = SchemaRegistry::new()
    ///     .register::<Model>("Model");
    ///
    /// let result = incomplete_registry.validate_completeness(&api);
    /// assert!(result.is_err());
    /// let missing = result.unwrap_err();
    /// assert!(missing.contains(&"ListModelsResponse".to_string()));
    /// ```
    pub fn validate_completeness(
        &self,
        api: &schematic_define::RestApi,
    ) -> Result<(), Vec<String>> {
        let mut missing = Vec::new();

        // A path-shaped name (`serde_json::Value`) is a foreign type, not a
        // registrable local one: an imported spec uses it wherever a body had no
        // named schema to reference. Requiring registration for it would make
        // every such API permanently incomplete.
        let is_registrable = |name: &String| !name.contains("::");

        for endpoint in &api.endpoints {
            if let schematic_define::ApiResponse::Json(schema) = &endpoint.response {
                let type_name = &schema.type_name;
                if is_registrable(type_name) && !self.types.contains_key(type_name) {
                    missing.push(type_name.clone());
                }
            }
            if let Some(schematic_define::ApiRequest::Json(schema)) = &endpoint.request {
                let type_name = &schema.type_name;
                if is_registrable(type_name) && !self.types.contains_key(type_name) {
                    missing.push(type_name.clone());
                }
            }
        }

        if missing.is_empty() {
            Ok(())
        } else {
            // Deduplicate missing types
            missing.sort();
            missing.dedup();
            Err(missing)
        }
    }
}

// Implement SchemaRegistryLike for integration with export functionality
impl schematic_define::openapi::SchemaRegistryLike for SchemaRegistry {
    fn to_openapi_schemas(&self) -> IndexMap<String, openapiv3::Schema> {
        self.to_openapi_schemas()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::openai::{
        DeleteModelResponse, ListModelsResponse, Model, define_openai_api,
        openapi_registry as openai_registry,
    };
    use crate::samsung_smart_tv::define_samsung_smart_tv_api;

    // =============================================
    // SchemaRegistry::new() tests
    // =============================================

    #[test]
    fn new_creates_empty_registry() {
        let registry = SchemaRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
    }

    // =============================================
    // SchemaRegistry::register() tests
    // =============================================

    #[test]
    fn register_adds_schema_to_registry() {
        let registry = SchemaRegistry::new().register::<Model>("Model");

        assert_eq!(registry.len(), 1);
        assert!(registry.get("Model").is_some());
    }

    #[test]
    fn register_returns_self_for_chaining() {
        let registry = SchemaRegistry::new()
            .register::<Model>("Model")
            .register::<ListModelsResponse>("ListModelsResponse")
            .register::<DeleteModelResponse>("DeleteModelResponse");

        assert_eq!(registry.len(), 3);
    }

    #[test]
    fn register_overwrites_existing_schema() {
        let registry = SchemaRegistry::new()
            .register::<Model>("Model")
            .register::<Model>("Model"); // Same name, should overwrite

        assert_eq!(registry.len(), 1);
    }

    // =============================================
    // SchemaRegistry::get() tests
    // =============================================

    #[test]
    fn get_returns_none_for_missing_schema() {
        let registry = SchemaRegistry::new();
        assert!(registry.get("NonExistent").is_none());
    }

    #[test]
    fn get_returns_schema_for_registered_type() {
        let registry = SchemaRegistry::new().register::<Model>("Model");

        let schema = registry.get("Model");
        assert!(schema.is_some());
    }

    // =============================================
    // SchemaRegistry::names() tests
    // =============================================

    #[test]
    fn names_returns_all_registered_names() {
        let registry = SchemaRegistry::new()
            .register::<Model>("Model")
            .register::<ListModelsResponse>("ListModelsResponse");

        let names: Vec<_> = registry.names().collect();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"Model"));
        assert!(names.contains(&"ListModelsResponse"));
    }

    // =============================================
    // SchemaRegistry::to_openapi_schemas() tests
    // =============================================

    #[test]
    fn to_openapi_schemas_converts_all_registered() {
        let registry = SchemaRegistry::new()
            .register::<Model>("Model")
            .register::<ListModelsResponse>("ListModelsResponse");

        let openapi_schemas = registry.to_openapi_schemas();

        assert_eq!(openapi_schemas.len(), 2);
        assert!(openapi_schemas.contains_key("Model"));
        assert!(openapi_schemas.contains_key("ListModelsResponse"));
    }

    #[test]
    fn to_openapi_schemas_produces_valid_schema_objects() {
        let registry = SchemaRegistry::new().register::<Model>("Model");

        let openapi_schemas = registry.to_openapi_schemas();
        let model_schema = openapi_schemas.get("Model").expect("Model schema missing");

        // Verify it's an object type with properties
        match &model_schema.schema_kind {
            openapiv3::SchemaKind::Type(openapiv3::Type::Object(obj)) => {
                // Model has: id, object, created, owned_by
                assert!(obj.properties.contains_key("id"));
                assert!(obj.properties.contains_key("object"));
                assert!(obj.properties.contains_key("created"));
                assert!(obj.properties.contains_key("owned_by"));
            }
            _ => panic!("Expected object schema kind"),
        }
    }

    // =============================================
    // SchemaRegistry::validate_completeness() tests
    // =============================================

    #[test]
    fn validate_completeness_passes_for_complete_registry() {
        let api = define_openai_api();
        let registry = openai_registry();

        let result = registry.validate_completeness(&api);
        assert!(result.is_ok(), "missing: {:?}", result.err());
    }

    #[test]
    fn validate_completeness_fails_for_missing_types() {
        let api = define_openai_api();

        // One registered type against the full endpoint surface.
        let registry = SchemaRegistry::new().register::<Model>("Model");

        let result = registry.validate_completeness(&api);
        assert!(result.is_err());

        let missing = result.unwrap_err();
        assert!(!missing.is_empty());
        assert!(!missing.contains(&"Model".to_string()));
    }

    #[test]
    fn validate_completeness_deduplicates_missing_types() {
        // Create an API with duplicate response types
        use schematic_define::{ApiResponse, Endpoint, RestApi, RestMethod};

        let api = RestApi {
            name: "Test".to_string(),
            description: "Test".to_string(),
            base_url: "https://test.com".to_string(),
            docs_url: None,
            auth: schematic_define::AuthStrategy::None,
            auth_policy: None,
            env_auth: vec![],
            env_username: None,
            headers: vec![],
            endpoints: vec![
                Endpoint {
                    id: "Get1".to_string(),
                    method: RestMethod::Get,
                    path: "/1".to_string(),
                    description: String::new(),
                    request: None,
                    response: ApiResponse::json_type("MissingType"),
                    headers: vec![],
                    params: None,
                    oauth_scopes: None,
                },
                Endpoint {
                    id: "Get2".to_string(),
                    method: RestMethod::Get,
                    path: "/2".to_string(),
                    description: String::new(),
                    request: None,
                    response: ApiResponse::json_type("MissingType"), // Same type
                    headers: vec![],
                    params: None,
                    oauth_scopes: None,
                },
            ],
            module_path: None,
            request_suffix: None,
            version: None,
            env_mapping: None,
        };

        let registry = SchemaRegistry::new();
        let result = registry.validate_completeness(&api);

        assert!(result.is_err());
        let missing = result.unwrap_err();
        // Should be deduplicated
        assert_eq!(missing.len(), 1);
        assert_eq!(missing[0], "MissingType");
    }

    #[test]
    fn validate_completeness_fails_for_missing_request_types() {
        use schematic_define::{ApiRequest, ApiResponse, Endpoint, RestApi, RestMethod};

        let api = RestApi {
            name: "Test".to_string(),
            description: "Test".to_string(),
            base_url: "https://test.com".to_string(),
            docs_url: None,
            auth: schematic_define::AuthStrategy::None,
            auth_policy: None,
            env_auth: vec![],
            env_username: None,
            headers: vec![],
            endpoints: vec![Endpoint {
                id: "Create".to_string(),
                method: RestMethod::Post,
                path: "/create".to_string(),
                description: String::new(),
                request: Some(ApiRequest::json_type("SomeBody")),
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

        let registry = SchemaRegistry::new();
        let result = registry.validate_completeness(&api);
        assert!(result.is_err());

        let missing = result.unwrap_err();
        assert!(missing.contains(&"SomeBody".to_string()));
    }

    #[test]
    fn validate_completeness_ignores_non_json_responses() {
        use schematic_define::{ApiResponse, Endpoint, RestApi, RestMethod};

        let api = RestApi {
            name: "Test".to_string(),
            description: "Test".to_string(),
            base_url: "https://test.com".to_string(),
            docs_url: None,
            auth: schematic_define::AuthStrategy::None,
            auth_policy: None,
            env_auth: vec![],
            env_username: None,
            headers: vec![],
            endpoints: vec![
                Endpoint {
                    id: "Binary".to_string(),
                    method: RestMethod::Get,
                    path: "/binary".to_string(),
                    description: String::new(),
                    request: None,
                    response: ApiResponse::Binary,
                    headers: vec![],
                    params: None,
                    oauth_scopes: None,
                },
                Endpoint {
                    id: "Text".to_string(),
                    method: RestMethod::Get,
                    path: "/text".to_string(),
                    description: String::new(),
                    request: None,
                    response: ApiResponse::Text,
                    headers: vec![],
                    params: None,
                    oauth_scopes: None,
                },
                Endpoint {
                    id: "Empty".to_string(),
                    method: RestMethod::Delete,
                    path: "/empty".to_string(),
                    description: String::new(),
                    request: None,
                    response: ApiResponse::Empty,
                    headers: vec![],
                    params: None,
                    oauth_scopes: None,
                },
            ],
            module_path: None,
            request_suffix: None,
            version: None,
            env_mapping: None,
        };

        let registry = SchemaRegistry::new();
        let result = registry.validate_completeness(&api);
        assert!(result.is_ok());
    }

    // =============================================
    // Doc comment to description tests
    // =============================================

    #[test]
    fn schema_includes_description_from_doc_comments() {
        let registry = SchemaRegistry::new().register::<Model>("Model");

        let schema = registry.get("Model").expect("Model schema missing");

        // The description should come from the doc comment on the Model struct
        // "An OpenAI model object."
        let description = schema
            .as_value()
            .as_object()
            .and_then(|obj| obj.get("description"))
            .and_then(|v| v.as_str());
        assert!(
            description.is_some(),
            "Schema should have description from doc comment"
        );
        assert!(
            description.unwrap().contains("OpenAI model"),
            "Description should contain 'OpenAI model': got {:?}",
            description
        );
    }

    #[test]
    fn openapi_schema_preserves_description() {
        let registry = SchemaRegistry::new().register::<Model>("Model");

        let openapi_schemas = registry.to_openapi_schemas();
        let model_schema = openapi_schemas.get("Model").expect("Model schema missing");

        // Check that the description is preserved in the OpenAPI schema
        assert!(
            model_schema.schema_data.description.is_some(),
            "OpenAPI schema should have description"
        );
    }

    // =============================================
    // get_registry() tests
    // =============================================

    #[test]
    fn get_registry_returns_openai() {
        use super::lookup::get_registry;

        let registry = get_registry("openai");
        assert!(registry.is_some());

        let registry = registry.unwrap();
        assert!(registry.get("Model").is_some());
        assert!(registry.get("ListModelsResponse").is_some());
        assert!(registry.get("DeleteModelResponse").is_some());
    }

    #[test]
    fn get_registry_returns_samsung_smart_tv() {
        use super::lookup::get_registry;

        let registry = get_registry("samsung-smart-tv");
        assert!(registry.is_some());

        let registry = registry.unwrap();
        assert!(registry.get("SamsungDeviceInfoResponse").is_some());
        assert!(registry.get("SamsungDeviceInfo").is_some());
    }

    #[test]
    fn get_registry_case_insensitive() {
        use super::lookup::get_registry;

        assert!(get_registry("OpenAI").is_some());
        assert!(get_registry("OPENAI").is_some());
        assert!(get_registry("openai").is_some());
        assert!(get_registry("SAMSUNG-SMART-TV").is_some());
    }

    #[test]
    fn get_registry_unknown_api_returns_none() {
        use super::lookup::get_registry;

        assert!(get_registry("unknown-api").is_none());
        assert!(get_registry("nonexistent").is_none());
    }

    #[test]
    fn get_registry_validates_against_api() {
        use super::lookup::get_registry;

        let registry = get_registry("openai").unwrap();
        let api = define_openai_api();

        let result = registry.validate_completeness(&api);
        assert!(result.is_ok(), "Registry should be complete: {:?}", result);
    }

    #[test]
    fn get_registry_validates_against_samsung_api() {
        use super::lookup::get_registry;

        let registry = get_registry("samsung-smart-tv").unwrap();
        let api = define_samsung_smart_tv_api();

        let result = registry.validate_completeness(&api);
        assert!(result.is_ok(), "Registry should be complete: {:?}", result);
    }

    // =============================================
    // SchemaRegistry::extend() tests
    // =============================================

    #[test]
    fn extend_unions_disjoint_registries() {
        let mut a = SchemaRegistry::new().register::<Model>("Model");
        let b = SchemaRegistry::new().register::<ListModelsResponse>("ListModelsResponse");

        a.extend(b);

        assert_eq!(a.len(), 2);
        assert!(a.get("Model").is_some());
        assert!(a.get("ListModelsResponse").is_some());
    }

    #[test]
    fn extend_overwrites_duplicate_names() {
        let mut a = SchemaRegistry::new().register::<Model>("Model");
        let b = SchemaRegistry::new().register::<Model>("Model");

        a.extend(b);

        // Identical schemas registered under the same name collapse to one.
        assert_eq!(a.len(), 1);
    }

    #[test]
    fn extend_preserves_insertion_order() {
        let mut a = SchemaRegistry::new()
            .register::<Model>("Model")
            .register::<ListModelsResponse>("ListModelsResponse");
        let b = SchemaRegistry::new().register::<DeleteModelResponse>("DeleteModelResponse");

        a.extend(b);

        let names: Vec<&str> = a.names().collect();
        assert_eq!(
            names,
            vec!["Model", "ListModelsResponse", "DeleteModelResponse"]
        );
    }

    // =============================================
    // registry_key_for() tests
    // =============================================

    #[test]
    fn registry_key_for_known_apis_matches_table() {
        // Mirrors the api_names table previously in schematic-gen/src/main.rs.
        let table = [
            ("Anthropic", "anthropic"),
            ("ArtificialAnalysisData", "artificial-analysis-data"),
            ("ArtificialAnalysisCritPt", "artificial-analysis-critpt"),
            ("OpenAI", "openai"),
            ("ElevenLabs", "elevenlabs"),
            ("Gitea", "gitea"),
            ("GitHub", "github"),
            ("GitLab", "gitlab"),
            ("HuggingFaceHub", "huggingface"),
            ("LmStudio", "lmstudio"),
            ("OllamaNative", "ollama-native"),
            ("OllamaOpenAI", "ollama-openai"),
            ("EmqxBasic", "emqx-basic"),
            ("EmqxBearer", "emqx-bearer"),
            ("Eversolo", "eversolo"),
            ("SamsungSmartTv", "samsung-smart-tv"),
            ("UnfoldedCircleCoreRest", "unfolded-circle-core-rest"),
            ("Bitbucket", "bitbucket"),
        ];

        for (api_name, expected_key) in table {
            assert_eq!(
                registry_key_for(api_name),
                expected_key,
                "registry_key_for({:?}) should yield {:?}",
                api_name,
                expected_key,
            );
            assert!(
                get_registry(&registry_key_for(api_name)).is_some(),
                "get_registry({:?}) should resolve to Some",
                expected_key,
            );
        }
    }

    #[test]
    fn registry_key_for_unknown_falls_back_to_lowercase() {
        assert_eq!(registry_key_for("MysteryApi"), "mysteryapi");
    }

    // =============================================
    // get_registries_for_module() tests
    // =============================================

    #[test]
    fn get_registries_for_module_returns_union_for_ollama() {
        use crate::ollama::{define_ollama_native_api, define_ollama_openai_api};

        let registry = get_registries_for_module("ollama").expect("ollama module should resolve");
        assert!(
            !registry.is_empty(),
            "merged ollama registry should be non-empty"
        );

        // Validates both member APIs against the merged registry.
        registry
            .validate_completeness(&define_ollama_native_api())
            .expect("native ollama API should validate");
        registry
            .validate_completeness(&define_ollama_openai_api())
            .expect("openai-compatible ollama API should validate");

        // The merged registry must be a strict superset of each per-API registry.
        let native = get_registry("ollama-native").unwrap();
        let openai = get_registry("ollama-openai").unwrap();
        for name in native.names() {
            assert!(
                registry.get(name).is_some(),
                "merged registry missing {} from ollama-native",
                name
            );
        }
        for name in openai.names() {
            assert!(
                registry.get(name).is_some(),
                "merged registry missing {} from ollama-openai",
                name
            );
        }
    }

    #[test]
    fn get_registries_for_module_returns_union_for_emqx() {
        use crate::emqx::{define_emqx_basic_api, define_emqx_bearer_api};

        let registry = get_registries_for_module("emqx").expect("emqx module should resolve");
        assert!(!registry.is_empty());

        registry
            .validate_completeness(&define_emqx_basic_api())
            .expect("emqx-basic should validate against merged registry");
        registry
            .validate_completeness(&define_emqx_bearer_api())
            .expect("emqx-bearer should validate against merged registry");

        let basic = get_registry("emqx-basic").unwrap();
        for name in basic.names() {
            assert!(registry.get(name).is_some());
        }
    }

    #[test]
    fn get_registries_for_module_returns_single_for_singleton_module() {
        let merged = get_registries_for_module("openai").expect("openai module should resolve");
        let direct = get_registry("openai").unwrap();

        // Compare key sets — for a singleton module, the merged registry equals
        // the per-API registry content-wise.
        let merged_names: std::collections::BTreeSet<&str> = merged.names().collect();
        let direct_names: std::collections::BTreeSet<&str> = direct.names().collect();
        assert_eq!(merged_names, direct_names);
    }

    #[test]
    fn get_registries_for_module_unknown_returns_none() {
        assert!(get_registries_for_module("not-a-real-module").is_none());
    }

    #[test]
    fn get_registries_for_module_is_case_insensitive() {
        assert!(get_registries_for_module("OLLAMA").is_some());
        assert!(get_registries_for_module("Ollama").is_some());
    }
}
