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
        self.types
            .iter()
            .map(|(name, schema)| {
                let openapi_schema = convert_schema_to_openapi(schema);
                (name.clone(), openapi_schema)
            })
            .collect()
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
    /// use schematic_definitions::openai::{define_openai_api, Model, ListModelsResponse, DeleteModelResponse};
    ///
    /// let api = define_openai_api();
    ///
    /// // Complete registry passes validation
    /// let complete_registry = SchemaRegistry::new()
    ///     .register::<Model>("Model")
    ///     .register::<ListModelsResponse>("ListModelsResponse")
    ///     .register::<DeleteModelResponse>("DeleteModelResponse");
    ///
    /// assert!(complete_registry.validate_completeness(&api).is_ok());
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

        for endpoint in &api.endpoints {
            if let schematic_define::ApiResponse::Json(schema) = &endpoint.response {
                let type_name = &schema.type_name;
                if !self.types.contains_key(type_name) {
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

/// Returns the OpenAPI schema registry for the specified API.
///
/// This function provides a central lookup for all available API schema registries.
/// OpenAI and Samsung Smart TV currently have complete registries with
/// `JsonSchema` derives on their REST response types.
///
/// ## Arguments
///
/// * `api_name` - The name of the API (case-insensitive). Supported values:
///   - `"openai"` - OpenAI Models API registry
///   - `"samsung-smart-tv"` - Samsung Smart TV REST registry
///
/// ## Returns
///
/// `Some(SchemaRegistry)` if the API has a complete schema registry, `None` otherwise.
///
/// ## Examples
///
/// ```
/// use schematic_definitions::registry::get_registry;
///
/// let registry = get_registry("openai");
/// assert!(registry.is_some());
///
/// let unknown = get_registry("unknown-api");
/// assert!(unknown.is_none());
/// ```
#[must_use]
pub fn get_registry(api_name: &str) -> Option<SchemaRegistry> {
    match api_name.to_lowercase().as_str() {
        "anthropic" => Some(crate::anthropic::openapi_registry()),
        "bitbucket" => Some(crate::bitbucket::openapi_registry()),
        "elevenlabs" => Some(crate::elevenlabs::openapi_registry()),
        "emqx-basic" | "emqx-bearer" => Some(crate::emqx::openapi_registry()),
        "eversolo" => Some(crate::eversolo::openapi_registry()),
        "gitea" => Some(crate::gitea::openapi_registry()),
        "github" => Some(crate::github::openapi_registry()),
        "gitlab" => Some(crate::gitlab::openapi_registry()),
        "huggingface" => Some(crate::huggingface::openapi_registry()),
        "lmstudio" => Some(crate::lmstudio::openapi_registry()),
        "ollama-native" | "ollama-openai" => Some(crate::ollama::openapi_registry()),
        "openai" => Some(crate::openai::openapi_registry()),
        "samsung-smart-tv" => Some(crate::samsung_smart_tv::openapi_registry()),
        "unfolded-circle-core-rest" => {
            Some(crate::unfolded_circle::core_rest::openapi_registry())
        }
        _ => None,
    }
}

/// Converts a schemars `Schema` to an `openapiv3::Schema`.
///
/// This function handles the conversion from schemars JSON Schema format
/// to OpenAPI 3.0 schema format. The conversion preserves:
/// - Type information (string, number, object, array, etc.)
/// - Property definitions
/// - Required fields
/// - Descriptions from doc comments
fn convert_schema_to_openapi(schema: &Schema) -> openapiv3::Schema {
    // Get the underlying JSON value
    let json_value = schema.as_value();
    convert_json_schema_to_openapi(json_value)
}

/// Converts a JSON Schema value to an OpenAPI schema.
fn convert_json_schema_to_openapi(value: &serde_json::Value) -> openapiv3::Schema {
    use openapiv3::{ObjectType, Schema, SchemaData, SchemaKind, Type};

    // Handle boolean schemas
    if let Some(b) = value.as_bool() {
        return if b {
            // true = any schema
            Schema {
                schema_data: SchemaData::default(),
                schema_kind: SchemaKind::Any(openapiv3::AnySchema::default()),
            }
        } else {
            // false = never matches (we'll represent as object with no valid values)
            Schema {
                schema_data: SchemaData::default(),
                schema_kind: SchemaKind::Not {
                    not: Box::new(openapiv3::ReferenceOr::Item(Schema {
                        schema_data: SchemaData::default(),
                        schema_kind: SchemaKind::Any(openapiv3::AnySchema::default()),
                    })),
                },
            }
        };
    }

    let obj = match value.as_object() {
        Some(obj) => obj,
        None => {
            return Schema {
                schema_data: SchemaData::default(),
                schema_kind: SchemaKind::Any(openapiv3::AnySchema::default()),
            };
        }
    };

    // Extract schema data (metadata)
    let data = SchemaData {
        title: obj.get("title").and_then(|v| v.as_str()).map(String::from),
        description: obj
            .get("description")
            .and_then(|v| v.as_str())
            .map(String::from),
        ..Default::default()
    };

    // Handle $ref (references to definitions)
    if let Some(ref_value) = obj.get("$ref")
        && let Some(ref_str) = ref_value.as_str()
    {
        // Convert schemars $ref format to OpenAPI $ref format
        let openapi_ref = ref_str.replace("#/$defs/", "#/components/schemas/");
        let openapi_ref = openapi_ref.replace("#/definitions/", "#/components/schemas/");
        return Schema {
            schema_data: data,
            schema_kind: SchemaKind::AllOf {
                all_of: vec![openapiv3::ReferenceOr::Reference {
                    reference: openapi_ref,
                }],
            },
        };
    }

    // Determine the schema kind from type
    let schema_kind = if let Some(type_value) = obj.get("type") {
        match type_value.as_str() {
            Some("object") => {
                let properties = obj
                    .get("properties")
                    .and_then(|p| p.as_object())
                    .map(|props| {
                        props
                            .iter()
                            .map(|(k, v)| {
                                let prop_schema = convert_json_schema_to_openapi(v);
                                (
                                    k.clone(),
                                    openapiv3::ReferenceOr::Item(Box::new(prop_schema)),
                                )
                            })
                            .collect()
                    })
                    .unwrap_or_default();

                let required = obj
                    .get("required")
                    .and_then(|r| r.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();

                SchemaKind::Type(Type::Object(ObjectType {
                    properties,
                    required,
                    ..Default::default()
                }))
            }
            Some("array") => {
                let items = obj.get("items").map(|items_schema| {
                    let item_schema = convert_json_schema_to_openapi(items_schema);
                    openapiv3::ReferenceOr::Item(Box::new(item_schema))
                });
                SchemaKind::Type(Type::Array(openapiv3::ArrayType {
                    items,
                    min_items: None,
                    max_items: None,
                    unique_items: false,
                }))
            }
            Some("string") => {
                let format = obj
                    .get("format")
                    .and_then(|f| f.as_str())
                    .map(|s| openapiv3::VariantOrUnknownOrEmpty::Unknown(s.to_string()))
                    .unwrap_or(openapiv3::VariantOrUnknownOrEmpty::Empty);
                SchemaKind::Type(Type::String(openapiv3::StringType {
                    format,
                    ..Default::default()
                }))
            }
            Some("integer") => {
                let format = obj
                    .get("format")
                    .and_then(|f| f.as_str())
                    .map(|s| openapiv3::VariantOrUnknownOrEmpty::Unknown(s.to_string()))
                    .unwrap_or(openapiv3::VariantOrUnknownOrEmpty::Empty);
                SchemaKind::Type(Type::Integer(openapiv3::IntegerType {
                    format,
                    ..Default::default()
                }))
            }
            Some("number") => {
                let format = obj
                    .get("format")
                    .and_then(|f| f.as_str())
                    .map(|s| openapiv3::VariantOrUnknownOrEmpty::Unknown(s.to_string()))
                    .unwrap_or(openapiv3::VariantOrUnknownOrEmpty::Empty);
                SchemaKind::Type(Type::Number(openapiv3::NumberType {
                    format,
                    ..Default::default()
                }))
            }
            Some("boolean") => SchemaKind::Type(Type::Boolean(openapiv3::BooleanType::default())),
            _ => SchemaKind::Any(openapiv3::AnySchema::default()),
        }
    } else if obj.contains_key("properties") {
        // Treat as object if it has properties but no explicit type
        let properties = obj
            .get("properties")
            .and_then(|p| p.as_object())
            .map(|props| {
                props
                    .iter()
                    .map(|(k, v)| {
                        let prop_schema = convert_json_schema_to_openapi(v);
                        (
                            k.clone(),
                            openapiv3::ReferenceOr::Item(Box::new(prop_schema)),
                        )
                    })
                    .collect()
            })
            .unwrap_or_default();

        let required = obj
            .get("required")
            .and_then(|r| r.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        SchemaKind::Type(Type::Object(ObjectType {
            properties,
            required,
            ..Default::default()
        }))
    } else {
        SchemaKind::Any(openapiv3::AnySchema::default())
    };

    Schema {
        schema_data: data,
        schema_kind,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::openai::{DeleteModelResponse, ListModelsResponse, Model, define_openai_api};
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

        let registry = SchemaRegistry::new()
            .register::<Model>("Model")
            .register::<ListModelsResponse>("ListModelsResponse")
            .register::<DeleteModelResponse>("DeleteModelResponse");

        let result = registry.validate_completeness(&api);
        assert!(result.is_ok());
    }

    #[test]
    fn validate_completeness_fails_for_missing_types() {
        let api = define_openai_api();

        let registry = SchemaRegistry::new().register::<Model>("Model");
        // Missing: ListModelsResponse, DeleteModelResponse

        let result = registry.validate_completeness(&api);
        assert!(result.is_err());

        let missing = result.unwrap_err();
        assert!(missing.contains(&"ListModelsResponse".to_string()));
        assert!(missing.contains(&"DeleteModelResponse".to_string()));
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
        use super::get_registry;

        let registry = get_registry("openai");
        assert!(registry.is_some());

        let registry = registry.unwrap();
        assert!(registry.get("Model").is_some());
        assert!(registry.get("ListModelsResponse").is_some());
        assert!(registry.get("DeleteModelResponse").is_some());
    }

    #[test]
    fn get_registry_returns_samsung_smart_tv() {
        use super::get_registry;

        let registry = get_registry("samsung-smart-tv");
        assert!(registry.is_some());

        let registry = registry.unwrap();
        assert!(registry.get("SamsungDeviceInfoResponse").is_some());
        assert!(registry.get("SamsungDeviceInfo").is_some());
    }

    #[test]
    fn get_registry_case_insensitive() {
        use super::get_registry;

        assert!(get_registry("OpenAI").is_some());
        assert!(get_registry("OPENAI").is_some());
        assert!(get_registry("openai").is_some());
        assert!(get_registry("SAMSUNG-SMART-TV").is_some());
    }

    #[test]
    fn get_registry_unknown_api_returns_none() {
        use super::get_registry;

        assert!(get_registry("unknown-api").is_none());
        assert!(get_registry("nonexistent").is_none());
    }

    #[test]
    fn get_registry_validates_against_api() {
        use super::get_registry;

        let registry = get_registry("openai").unwrap();
        let api = define_openai_api();

        let result = registry.validate_completeness(&api);
        assert!(result.is_ok(), "Registry should be complete: {:?}", result);
    }

    #[test]
    fn get_registry_validates_against_samsung_api() {
        use super::get_registry;

        let registry = get_registry("samsung-smart-tv").unwrap();
        let api = define_samsung_smart_tv_api();

        let result = registry.validate_completeness(&api);
        assert!(result.is_ok(), "Registry should be complete: {:?}", result);
    }
}
