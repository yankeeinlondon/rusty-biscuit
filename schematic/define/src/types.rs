//! Core types for REST API definitions.
//!
//! This module provides the fundamental types for defining REST APIs:
//!
//! - [`RestApi`] - The top-level API definition
//! - [`Endpoint`] - Individual API endpoint definitions
//! - [`RestMethod`] - HTTP method enumeration

use serde::{Deserialize, Serialize};
use strum::{Display, EnumIter, EnumString};

use crate::auth::{AuthPolicy, AuthStrategy, EnvAuthStrategy};
use crate::cross_origin::CrossOrigin;
use crate::headers::{EnvList, EnvMapping};
use crate::pagination::Strategy;
use crate::params::EndpointParams;
use crate::request::ApiRequest;
use crate::response::ApiResponse;

/// HTTP methods supported by REST APIs.
///
/// This enum represents standard HTTP methods used in REST APIs.
/// It derives several traits for serialization, display, and iteration.
///
/// ## Examples
///
/// Parse from string:
///
/// ```
/// use std::str::FromStr;
/// use schematic_define::RestMethod;
///
/// let method = RestMethod::from_str("GET").unwrap();
/// assert_eq!(method, RestMethod::Get);
/// ```
///
/// Display as uppercase:
///
/// ```
/// use schematic_define::RestMethod;
///
/// assert_eq!(RestMethod::Post.to_string(), "POST");
/// ```
///
/// Iterate over all methods:
///
/// ```
/// use schematic_define::RestMethod;
/// use strum::IntoEnumIterator;
///
/// let methods: Vec<_> = RestMethod::iter().collect();
/// assert_eq!(methods.len(), 7);
/// ```
#[non_exhaustive]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Display, EnumIter, EnumString,
)]
#[serde(rename_all = "UPPERCASE")]
#[strum(serialize_all = "UPPERCASE")]
pub enum RestMethod {
    /// HTTP GET - Retrieve a resource
    Get,
    /// HTTP POST - Create a new resource
    Post,
    /// HTTP PUT - Replace a resource entirely
    Put,
    /// HTTP PATCH - Partially update a resource
    Patch,
    /// HTTP DELETE - Remove a resource
    Delete,
    /// HTTP HEAD - Get headers only (no body)
    Head,
    /// HTTP OPTIONS - Get allowed methods for a resource
    Options,
}

/// A complete REST API definition.
///
/// This struct captures all the information needed to _generate_ a typed client
/// for a REST API, including the base URL, authentication strategy, and all
/// endpoint definitions.
///
/// ## Examples
///
/// Create a simple API with no authentication:
///
/// ```
/// use schematic_define::{DefinitionContext, RestApi, Endpoint};
///
/// let mut ctx = DefinitionContext::types_in("crate::types");
/// let endpoint = Endpoint::get("GetHealth", "/health")
///     .describes("Health check endpoint")
///     .returns_json::<serde_json::Value>(&mut ctx)
///     .build().unwrap();
/// let api = RestApi::builder(&ctx, "SimpleApi", "https://api.example.com/v1")
///     .describes("A simple REST API")
///     .no_auth()
///     .endpoints([endpoint])
///     .build().unwrap();
///
/// assert_eq!(api.name, "SimpleApi");
/// assert_eq!(api.endpoints.len(), 1);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub struct RestApi {
    /// Unique identifier for this API (used for generated type names).
    ///
    /// "OpenAI", for example, generates the client struct
    /// `struct OpenAI(schematic_runtime::Client<OpenAIApi>)` and its tag type
    /// `enum OpenAIApi {}`.
    pub name: String,
    /// Human-readable description of the API.
    pub description: String,
    /// Base URL for all endpoints (e.g., `https://api.openai.com/v1`).
    ///
    /// Endpoint paths are appended to this URL when making requests.
    pub base_url: String,
    /// Link to API documentation (optional).
    pub docs_url: Option<String>,
    /// Authentication strategy for this API.
    pub auth: AuthStrategy,
    /// Structured authentication policy for this API.
    ///
    /// Use this when the client should accept multiple explicit auth methods,
    /// such as "explicit OAuth token or explicit API key, with API-key env
    /// fallback". If `None`, the generator derives a backward-compatible policy
    /// from [`Self::auth`].
    pub auth_policy: Option<AuthPolicy>,
    /// Environment variable names for authentication credentials.
    ///
    /// For `BearerToken`, `ApiKey`, and `ApiKeyParam` auth strategies, this is
    /// a fallback chain. The first non-empty env var is used. If none is set,
    /// the request is sent anonymously.
    ///
    /// Example: `vec!["OPENAI_API_KEY".to_string(), "OPENAI_KEY".to_string()]`
    pub env_auth: Vec<String>,
    /// Environment variable for Basic auth username.
    ///
    /// Only used when `auth` is `AuthStrategy::Basic`. The password is read
    /// from the first element of `env_auth` (i.e., `env_auth[0]`).
    pub env_username: Option<String>,
    /// Default HTTP headers to include with every request.
    ///
    /// These headers are applied to all endpoints unless overridden by
    /// endpoint-specific headers. Keys are case-insensitive for merging.
    ///
    /// Example: `vec![("X-Api-Version".to_string(), "2024-01".to_string())]`
    pub headers: Vec<(String, String)>,
    /// All endpoints defined for this API.
    pub endpoints: Vec<Endpoint>,
    /// Custom module path for generated code (defaults to `api.name.to_lowercase()`).
    ///
    /// This allows APIs to specify a different module name than the default
    /// lowercase version of the API name. For example, an API named "HuggingFaceHub"
    /// might want to use "huggingface" instead of "huggingfacehub".
    pub module_path: Option<String>,
    /// Custom suffix for generated request structs (defaults to "Request").
    ///
    /// This allows APIs to customize the naming of request structs. For example,
    /// using "Params" would generate `ListModelsParams` instead of `ListModelsRequest`.
    pub request_suffix: Option<String>,
    /// Semantic version of this API definition (e.g., `"1.0.0"`).
    ///
    /// Used as the `info.version` field in exported OpenAPI documents.
    /// If `None`, the exporter falls back to `ExportOptions::version`, then `"1.0.0"`.
    pub version: Option<String>,
    /// Environment variable mapping for authentication credentials.
    ///
    /// This provides a structured way to configure which environment variables to check
    /// for authentication credentials. If `None`, the mapping is built from legacy fields
    /// (`env_auth`, `env_username`) for backward compatibility.
    ///
    /// ## Examples
    ///
    /// ```
    /// use schematic_define::{DefinitionContext, RestApi, EnvMapping, EnvList};
    ///
    /// let ctx = DefinitionContext::types_in("crate::types");
    /// let api = RestApi::builder(&ctx, "Example", "https://api.example.com")
    ///     .describes("Example API")
    ///     .bearer_env(["API_KEY", "TOKEN"])
    ///     .env_mapping(EnvMapping {
    ///         bearer_token: Some(EnvList::from_strs(&["API_KEY", "TOKEN"])),
    ///         ..Default::default()
    ///     })
    ///     .build().unwrap();
    /// ```
    pub env_mapping: Option<EnvMapping>,
    /// Governs redirects and followed next-page links that leave the base
    /// origin. Defaults to same-origin; endpoints cannot override it.
    pub cross_origin: CrossOrigin,
    /// Response header carrying the provider's request id, matched
    /// case-insensitively. `None` when the provider is not known to send one.
    pub request_id_header: Option<String>,
    /// Extra header names (matched case-insensitively) and query names
    /// (matched exactly) whose values are never stored in metadata or shown,
    /// beyond the built-in names and the API's authentication names.
    pub secret_names: Vec<String>,
    /// Strategy inherited by every list endpoint without an endpoint override.
    ///
    /// `None` leaves lists unpaginated. A GET endpoint whose response is an
    /// explicit `Vec<T>` is a list automatically; any other list needs an endpoint collection. See [`crate::pagination::resolve`].
    pub default_pagination: Option<Strategy>,
    /// Every schema component of this API's definition context, in
    /// registration order. JSON body references name these; see
    /// [`RestApi::validate_references`].
    pub schemas: Vec<crate::SchemaComponent>,
}

impl RestApi {
    /// Get the environment variable mapping for this API.
    ///
    /// If `env_mapping` is set, returns a clone. Otherwise, builds a mapping
    /// from the legacy fields (`env_auth`, `env_username`) for backward compatibility.
    ///
    /// ## Returns
    ///
    /// An `EnvMapping` struct containing the environment variable configuration.
    ///
    /// ## Examples
    ///
    /// With explicit env_mapping:
    ///
    /// ```
    /// use schematic_define::{DefinitionContext, RestApi, EnvMapping, EnvList};
    ///
    /// let ctx = DefinitionContext::types_in("crate::types");
    /// let api = RestApi::builder(&ctx, "Example", "https://api.example.com")
    ///     .describes("Example API")
    ///     .bearer_env(["CUSTOM_TOKEN"])
    ///     .env_mapping(EnvMapping {
    ///         bearer_token: Some(EnvList::from_strs(&["CUSTOM_TOKEN"])),
    ///         ..Default::default()
    ///     })
    ///     .build().unwrap();
    ///
    /// let mapping = api.default_env_mapping();
    /// assert!(mapping.bearer_token.is_some());
    /// ```
    ///
    /// With a bearer environment fallback:
    ///
    /// ```
    /// use schematic_define::{DefinitionContext, RestApi};
    ///
    /// let ctx = DefinitionContext::types_in("crate::types");
    /// let api = RestApi::builder(&ctx, "Example", "https://api.example.com")
    ///     .describes("Example API")
    ///     .bearer_env(["OPENAI_API_KEY"])
    ///     .build().unwrap();
    ///
    /// let mapping = api.default_env_mapping();
    /// assert!(mapping.bearer_token.is_some());
    /// assert_eq!(mapping.bearer_token.unwrap().names(), &["OPENAI_API_KEY"]);
    /// ```
    pub fn default_env_mapping(&self) -> EnvMapping {
        // If env_mapping is explicitly set, use it
        if let Some(ref mapping) = self.env_mapping {
            return mapping.clone();
        }

        Self::legacy_env_mapping_for(&self.auth, &self.env_auth, self.env_username.as_deref())
    }

    /// Returns the effective authentication policy for this API.
    ///
    /// If [`Self::auth_policy`] is set, it is used directly. Otherwise a
    /// backward-compatible policy is derived from [`Self::auth`].
    #[must_use]
    pub fn effective_auth_policy(&self) -> AuthPolicy {
        self.auth_policy
            .clone()
            .unwrap_or_else(|| AuthPolicy::from_auth_strategy(&self.auth))
    }

    /// Builds an [`EnvMapping`] from the legacy auth fields.
    ///
    /// This is used by the generator to preserve the existing `env_auth` /
    /// `env_username` authoring API while routing runtime behavior through
    /// [`EnvMapping`].
    ///
    /// An [`AuthStrategy::ApiKeyParam`] strategy yields an empty mapping:
    /// [`EnvMapping`] only describes header credentials. For an
    /// [`EnvAuthStrategy::ApiKeyParam`] fallback the generator reads the
    /// variables from `env_auth` directly and gives them to the runtime's
    /// environment fallback with a query or cookie placement.
    #[must_use]
    pub fn legacy_env_mapping_for(
        auth: &AuthStrategy,
        env_auth: &[String],
        env_username: Option<&str>,
    ) -> EnvMapping {
        match AuthPolicy::from_auth_strategy(auth).env_fallback {
            Some(EnvAuthStrategy::BearerToken { .. }) => EnvMapping {
                bearer_token: (!env_auth.is_empty()).then(|| EnvList::new(env_auth.to_vec())),
                ..Default::default()
            },
            Some(EnvAuthStrategy::ApiKey { header, .. }) => EnvMapping {
                api_key: (!env_auth.is_empty()).then(|| crate::headers::ApiKeyEnv {
                    names: EnvList::new(env_auth.to_vec()),
                    header,
                }),
                ..Default::default()
            },
            // See the doc comment: parameter keys are not header credentials.
            Some(EnvAuthStrategy::ApiKeyParam { .. }) => EnvMapping::default(),
            Some(EnvAuthStrategy::Basic) => EnvMapping {
                basic_user: env_username.map(EnvList::single),
                basic_pass: env_auth.first().cloned().map(EnvList::single),
                ..Default::default()
            },
            None => EnvMapping::default(),
        }
    }
}

/// A single API endpoint definition.
///
/// Endpoints define how to make a specific API call, including the HTTP method,
/// URL path (with optional parameters), and request/response schemas.
///
/// ## Path Parameters
///
/// Paths support template parameters using curly braces: `/models/{model}`.
/// These become fields in the generated request struct.
///
/// ## Examples
///
/// A GET endpoint with a path parameter:
///
/// ```
/// use schematic_define::{DefinitionContext, Endpoint};
///
/// let mut ctx = DefinitionContext::types_in("crate::types");
/// let endpoint = Endpoint::get("GetUser", "/users/{user_id}")
///     .describes("Retrieve a user by ID")
///     .returns_json::<serde_json::Value>(&mut ctx)
///     .build().unwrap();
///
/// assert!(endpoint.path.contains("{user_id}"));
/// ```
///
/// A POST endpoint with a JSON request body:
///
/// ```
/// use schematic_define::{DefinitionContext, Endpoint};
///
/// let mut ctx = DefinitionContext::types_in("crate::types");
/// let endpoint = Endpoint::post("CreateUser", "/users")
///     .describes("Create a new user")
///     .request_json::<serde_json::Value>(&mut ctx)
///     .returns_json::<serde_json::Value>(&mut ctx)
///     .build().unwrap();
///
/// assert!(endpoint.request.is_some());
/// ```
///
/// A POST endpoint with multipart form-data:
///
/// ```
/// use schematic_define::{DefinitionContext, Endpoint, FormField};
///
/// let mut ctx = DefinitionContext::types_in("crate::types");
/// let endpoint = Endpoint::post("UploadFile", "/files")
///     .describes("Upload a file")
///     .request_form(vec![
///         FormField::file_accept("document", vec!["application/pdf".into()]),
///         FormField::text("title").optional(),
///     ])
///     .returns_json::<serde_json::Value>(&mut ctx)
///     .build().unwrap();
/// ```
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub struct Endpoint {
    /// Identifier for this endpoint (becomes the enum variant and struct name).
    ///
    /// Should be PascalCase (e.g., "ListModels", "CreateUser").
    pub id: String,
    /// HTTP method for this endpoint.
    pub method: RestMethod,
    /// Path template (e.g., "/models/{model}").
    ///
    /// Parameters in curly braces become fields in the generated request struct.
    pub path: String,
    /// Human-readable description of what this endpoint does.
    pub description: String,
    /// Request body definition (typically `None` for GET/DELETE requests).
    ///
    /// Author JSON bodies with [`crate::EndpointBuilder::request_json`] and
    /// multipart uploads with [`crate::EndpointBuilder::request_form`].
    pub request: Option<ApiRequest>,
    /// Expected response type for this endpoint.
    pub response: ApiResponse,
    /// HTTP headers specific to this endpoint.
    ///
    /// These headers are merged with API-level headers, with endpoint headers
    /// taking precedence for matching keys (case-insensitive comparison).
    ///
    /// Example for Anthropic beta endpoints:
    /// ```text
    /// headers: vec![("anthropic-beta".to_string(), "message-batches-2024-09-24".to_string())]
    /// ```
    pub headers: Vec<(String, String)>,
    /// Query, header, and cookie parameters for this endpoint.
    ///
    /// These parameters are imported from API specifications like OpenAPI and
    /// define additional request parameters beyond the body and path parameters.
    ///
    /// If `None`, the endpoint has no imported parameters (backwards compatible
    /// with existing endpoint definitions).
    pub params: Option<EndpointParams>,
    /// Replaces the API policy for this endpoint; callers may only tighten it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cross_origin: Option<crate::CrossOrigin>,
    /// Explicit collection shape and pagination policy, overriding API inference.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pagination: Option<crate::pagination::Collection>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;
    use strum::IntoEnumIterator;

    #[test]
    fn rest_method_display_uppercase() {
        assert_eq!(RestMethod::Get.to_string(), "GET");
        assert_eq!(RestMethod::Post.to_string(), "POST");
        assert_eq!(RestMethod::Put.to_string(), "PUT");
        assert_eq!(RestMethod::Patch.to_string(), "PATCH");
        assert_eq!(RestMethod::Delete.to_string(), "DELETE");
        assert_eq!(RestMethod::Head.to_string(), "HEAD");
        assert_eq!(RestMethod::Options.to_string(), "OPTIONS");
    }

    #[test]
    fn rest_method_from_str_uppercase() {
        assert_eq!(RestMethod::from_str("GET").unwrap(), RestMethod::Get);
        assert_eq!(RestMethod::from_str("POST").unwrap(), RestMethod::Post);
        assert_eq!(RestMethod::from_str("PUT").unwrap(), RestMethod::Put);
        assert_eq!(RestMethod::from_str("PATCH").unwrap(), RestMethod::Patch);
        assert_eq!(RestMethod::from_str("DELETE").unwrap(), RestMethod::Delete);
        assert_eq!(RestMethod::from_str("HEAD").unwrap(), RestMethod::Head);
        assert_eq!(
            RestMethod::from_str("OPTIONS").unwrap(),
            RestMethod::Options
        );
    }

    #[test]
    fn rest_method_from_str_invalid() {
        assert!(RestMethod::from_str("INVALID").is_err());
        assert!(RestMethod::from_str("get").is_err()); // Case-sensitive
        assert!(RestMethod::from_str("").is_err());
    }

    #[test]
    fn rest_method_iter_all_variants() {
        let variants: Vec<_> = RestMethod::iter().collect();
        assert_eq!(variants.len(), 7);
        assert!(variants.contains(&RestMethod::Get));
        assert!(variants.contains(&RestMethod::Post));
        assert!(variants.contains(&RestMethod::Put));
        assert!(variants.contains(&RestMethod::Patch));
        assert!(variants.contains(&RestMethod::Delete));
        assert!(variants.contains(&RestMethod::Head));
        assert!(variants.contains(&RestMethod::Options));
    }

    #[test]
    fn rest_method_serde_roundtrip() {
        let method = RestMethod::Post;
        let serialized = serde_json::to_string(&method).unwrap();
        assert_eq!(serialized, "\"POST\"");

        let deserialized: RestMethod = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, method);
    }

    // ========================================
    // RestApi::default_env_mapping Tests
    // ========================================

    #[test]
    fn default_env_mapping_returns_explicit_mapping_when_set() {
        let explicit_mapping = EnvMapping {
            bearer_token: Some(EnvList::from_strs(&["CUSTOM_TOKEN", "FALLBACK_TOKEN"])),
            basic_user: None,
            basic_pass: None,
            api_key: None,
            ..Default::default()
        };

        let api = RestApi {
            name: "TestApi".to_string(),
            description: "Test API".to_string(),
            base_url: "https://api.test.com".to_string(),
            docs_url: None,
            auth: AuthStrategy::BearerToken { header: None },
            auth_policy: None,
            env_auth: vec!["IGNORED_ENV".to_string()],
            env_username: None,
            headers: vec![],
            endpoints: vec![],
            module_path: None,
            request_suffix: None,
            version: None,
            env_mapping: Some(explicit_mapping.clone()),
            cross_origin: CrossOrigin::default(),
            request_id_header: None,
            secret_names: vec![],
            default_pagination: None,
            schemas: vec![],
            };

        let mapping = api.default_env_mapping();
        assert_eq!(mapping, explicit_mapping);
        assert_eq!(
            mapping.bearer_token.unwrap().names(),
            &["CUSTOM_TOKEN", "FALLBACK_TOKEN"]
        );
    }

    #[test]
    fn default_env_mapping_builds_from_env_auth_when_none() {
        let api = RestApi {
            name: "TestApi".to_string(),
            description: "Test API".to_string(),
            base_url: "https://api.test.com".to_string(),
            docs_url: None,
            auth: AuthStrategy::BearerToken { header: None },
            auth_policy: None,
            env_auth: vec!["OPENAI_API_KEY".to_string(), "OPENAI_KEY".to_string()],
            env_username: None,
            headers: vec![],
            endpoints: vec![],
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

        let mapping = api.default_env_mapping();
        assert!(mapping.bearer_token.is_some());
        assert_eq!(
            mapping.bearer_token.unwrap().names(),
            &["OPENAI_API_KEY", "OPENAI_KEY"]
        );
        assert!(mapping.basic_user.is_none());
        assert!(mapping.basic_pass.is_none());
        assert!(mapping.api_key.is_none());
    }

    #[test]
    fn default_env_mapping_builds_basic_auth_from_legacy_fields() {
        let api = RestApi {
            name: "TestApi".to_string(),
            description: "Test API".to_string(),
            base_url: "https://api.test.com".to_string(),
            docs_url: None,
            auth: AuthStrategy::Basic,
            auth_policy: None,
            env_auth: vec!["API_PASSWORD".to_string()],
            env_username: Some("API_USERNAME".to_string()),
            headers: vec![],
            endpoints: vec![],
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

        let mapping = api.default_env_mapping();
        assert!(mapping.basic_user.is_some());
        assert_eq!(mapping.basic_user.unwrap().names(), &["API_USERNAME"]);
        assert!(mapping.basic_pass.is_some());
        assert_eq!(mapping.basic_pass.unwrap().names(), &["API_PASSWORD"]);
        assert!(mapping.bearer_token.is_none());
        assert!(mapping.api_key.is_none());
    }

    #[test]
    fn default_env_mapping_empty_when_no_env_fields() {
        let api = RestApi {
            name: "TestApi".to_string(),
            description: "Test API".to_string(),
            base_url: "https://api.test.com".to_string(),
            docs_url: None,
            auth: AuthStrategy::None,
            auth_policy: None,
            env_auth: vec![],
            env_username: None,
            headers: vec![],
            endpoints: vec![],
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

        let mapping = api.default_env_mapping();
        assert!(mapping.bearer_token.is_none());
        assert!(mapping.basic_user.is_none());
        assert!(mapping.basic_pass.is_none());
        assert!(mapping.api_key.is_none());
    }

    #[test]
    fn default_env_mapping_basic_auth_without_password_env() {
        let api = RestApi {
            name: "TestApi".to_string(),
            description: "Test API".to_string(),
            base_url: "https://api.test.com".to_string(),
            docs_url: None,
            auth: AuthStrategy::Basic,
            auth_policy: None,
            env_auth: vec![], // Empty env_auth
            env_username: Some("API_USERNAME".to_string()),
            headers: vec![],
            endpoints: vec![],
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

        let mapping = api.default_env_mapping();
        assert!(mapping.basic_user.is_some());
        assert_eq!(mapping.basic_user.unwrap().names(), &["API_USERNAME"]);
        assert!(mapping.basic_pass.is_none()); // No password env available
        assert!(mapping.bearer_token.is_none());
    }

    #[test]
    fn default_env_mapping_api_key_only_in_explicit_mapping() {
        use crate::headers::ApiKeyEnv;

        let explicit_mapping = EnvMapping {
            bearer_token: None,
            basic_user: None,
            basic_pass: None,
            api_key: Some(ApiKeyEnv {
                names: EnvList::from_strs(&["HF_TOKEN", "HUGGINGFACE_TOKEN"]),
                header: "Authorization".to_string(),
            }),
            ..Default::default()
        };

        let api = RestApi {
            name: "TestApi".to_string(),
            description: "Test API".to_string(),
            base_url: "https://api.test.com".to_string(),
            docs_url: None,
            auth: AuthStrategy::ApiKey {
                header: "Authorization".to_string(),
                value_prefix: None,
            },
            auth_policy: None,
            env_auth: vec![],
            env_username: None,
            headers: vec![],
            endpoints: vec![],
            module_path: None,
            request_suffix: None,
            version: None,
            env_mapping: Some(explicit_mapping.clone()),
            cross_origin: CrossOrigin::default(),
            request_id_header: None,
            secret_names: vec![],
            default_pagination: None,
            schemas: vec![],
            };

        let mapping = api.default_env_mapping();
        assert_eq!(mapping, explicit_mapping);
        assert!(mapping.api_key.is_some());
        let api_key = mapping.api_key.unwrap();
        assert_eq!(api_key.names.names(), &["HF_TOKEN", "HUGGINGFACE_TOKEN"]);
        assert_eq!(api_key.header, "Authorization");
    }

    #[test]
    fn default_env_mapping_clone_independence() {
        let explicit_mapping = EnvMapping {
            bearer_token: Some(EnvList::single("TOKEN")),
            basic_user: None,
            basic_pass: None,
            api_key: None,
            ..Default::default()
        };

        let api = RestApi {
            name: "TestApi".to_string(),
            description: "Test API".to_string(),
            base_url: "https://api.test.com".to_string(),
            docs_url: None,
            auth: AuthStrategy::BearerToken { header: None },
            auth_policy: None,
            env_auth: vec![],
            env_username: None,
            headers: vec![],
            endpoints: vec![],
            module_path: None,
            request_suffix: None,
            version: None,
            env_mapping: Some(explicit_mapping),
            cross_origin: CrossOrigin::default(),
            request_id_header: None,
            secret_names: vec![],
            default_pagination: None,
            schemas: vec![],
            };

        let mapping1 = api.default_env_mapping();
        let mapping2 = api.default_env_mapping();

        // Should be equal but independent clones
        assert_eq!(mapping1, mapping2);
    }
}
