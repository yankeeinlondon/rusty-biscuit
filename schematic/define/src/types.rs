//! Core types for REST API definitions.
//!
//! This module provides the fundamental types for defining REST APIs:
//!
//! - [`RestApi`] - The top-level API definition
//! - [`Endpoint`] - Individual API endpoint definitions
//! - [`RestMethod`] - HTTP method enumeration

use serde::{Deserialize, Serialize};
use strum::{Display, EnumIter, EnumString};

use crate::auth::AuthStrategy;
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
/// use schematic_define::{RestApi, Endpoint, RestMethod, AuthStrategy, ApiResponse};
///
/// let api = RestApi {
///     name: "SimpleApi".to_string(),
///     description: "A simple REST API".to_string(),
///     base_url: "https://api.example.com/v1".to_string(),
///     docs_url: None,
///     auth: AuthStrategy::None,
///     env_auth: vec![],
///     env_username: None,
///     headers: vec![],
///     endpoints: vec![
///         Endpoint {
///             id: "GetHealth".to_string(),
///             method: RestMethod::Get,
///             path: "/health".to_string(),
///             description: "Health check endpoint".to_string(),
///             request: None,
///             response: ApiResponse::json_type("HealthResponse"),
///             headers: vec![],
///         },
///     ],
/// };
///
/// assert_eq!(api.name, "SimpleApi");
/// assert_eq!(api.endpoints.len(), 1);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestApi {
    /// Unique identifier for this API (used for generated struct/enum names).
    ///
    /// This becomes the generated client struct name (e.g., "OpenAI" generates
    /// `struct OpenAI` and `enum OpenAIRequest`).
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
    /// Environment variable names for authentication credentials.
    ///
    /// For `BearerToken` and `ApiKey` auth strategies, this is a fallback chain.
    /// The first env var that is set will be used. If none are set, the request
    /// will fail with a `MissingCredential` error.
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
/// use schematic_define::{Endpoint, RestMethod, ApiResponse};
///
/// let endpoint = Endpoint {
///     id: "GetUser".to_string(),
///     method: RestMethod::Get,
///     path: "/users/{user_id}".to_string(),
///     description: "Retrieve a user by ID".to_string(),
///     request: None,
///     response: ApiResponse::json_type("User"),
///     headers: vec![],
/// };
///
/// assert!(endpoint.path.contains("{user_id}"));
/// ```
///
/// A POST endpoint with a JSON request body:
///
/// ```
/// use schematic_define::{Endpoint, RestMethod, ApiResponse, ApiRequest};
///
/// let endpoint = Endpoint {
///     id: "CreateUser".to_string(),
///     method: RestMethod::Post,
///     path: "/users".to_string(),
///     description: "Create a new user".to_string(),
///     request: Some(ApiRequest::json_type("CreateUserRequest")),
///     response: ApiResponse::json_type("User"),
///     headers: vec![],
/// };
///
/// assert!(endpoint.request.is_some());
/// ```
///
/// A POST endpoint with multipart form-data:
///
/// ```
/// use schematic_define::{Endpoint, RestMethod, ApiResponse, ApiRequest, FormField};
///
/// let endpoint = Endpoint {
///     id: "UploadFile".to_string(),
///     method: RestMethod::Post,
///     path: "/files".to_string(),
///     description: "Upload a file".to_string(),
///     request: Some(ApiRequest::form_data(vec![
///         FormField::file_accept("document", vec!["application/pdf".into()]),
///         FormField::text("title").optional(),
///     ])),
///     response: ApiResponse::json_type("FileUploadResponse"),
///     headers: vec![],
/// };
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
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
    /// Use [`ApiRequest::json_type`] for JSON bodies, [`ApiRequest::form_data`]
    /// for multipart uploads, or other variants as needed.
    pub request: Option<ApiRequest>,
    /// Expected response type for this endpoint.
    pub response: ApiResponse,
    /// HTTP headers specific to this endpoint.
    ///
    /// These headers are merged with API-level headers, with endpoint headers
    /// taking precedence for matching keys (case-insensitive comparison).
    ///
    /// Example for Anthropic beta endpoints:
    /// ```ignore
    /// headers: vec![("anthropic-beta".to_string(), "message-batches-2024-09-24".to_string())]
    /// ```
    pub headers: Vec<(String, String)>,
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
}
