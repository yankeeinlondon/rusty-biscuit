//! API type definitions
//!
//! This module contains types for representing API authentication methods
//! and endpoint configurations. Created during Phase 0 of the provider
//! refactoring (2025-12-30).

/// API authentication methods
///
/// Represents different ways to authenticate with provider APIs.
///
/// ## Examples
/// ```
/// use shared::api::types::ApiAuth;
///
/// // Bearer token authentication (most common)
/// let bearer = ApiAuth::Bearer("sk-test-key".to_string());
///
/// // Custom header authentication (e.g., Anthropic's x-api-key)
/// let custom_header = ApiAuth::HeaderKey("x-api-key".to_string(), "key-value".to_string());
///
/// // Query parameter authentication
/// let query = ApiAuth::QueryParam("api_key".to_string(), "key-value".to_string());
///
/// // No authentication (for local providers like Ollama)
/// let none = ApiAuth::None;
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApiAuth {
    /// Authorization: Bearer {token}
    Bearer(String),
    /// Custom header: {name}: {value}
    HeaderKey(String, String),
    /// URL query parameter: ?{name}={value}
    QueryParam(String, String),
    /// No authentication required
    None,
}

/// API endpoint configuration
///
/// Represents a complete API endpoint with base URL, path, and authentication.
///
/// ## Examples
/// ```
/// use shared::api::types::{ApiEndpoint, ApiAuth};
///
/// let endpoint = ApiEndpoint {
///     base_url: "https://api.openai.com".to_string(),
///     path: "/v1/models".to_string(),
///     auth: ApiAuth::Bearer("sk-test".to_string()),
/// };
/// ```
#[derive(Debug, Clone)]
pub struct ApiEndpoint {
    /// Base URL for the API (e.g., "https://api.openai.com")
    pub base_url: String,
    /// Path for the specific endpoint (e.g., "/v1/models")
    pub path: String,
    /// Authentication method for this endpoint
    pub auth: ApiAuth,
}

impl ApiEndpoint {
    /// Construct the full URL for this endpoint
    ///
    /// ## Returns
    /// Returns the full URL by concatenating base_url and path.
    ///
    /// ## Examples
    /// ```
    /// use shared::api::types::{ApiEndpoint, ApiAuth};
    ///
    /// let endpoint = ApiEndpoint {
    ///     base_url: "https://api.openai.com".to_string(),
    ///     path: "/v1/models".to_string(),
    ///     auth: ApiAuth::None,
    /// };
    ///
    /// assert_eq!(endpoint.full_url(), "https://api.openai.com/v1/models");
    /// ```
    pub fn full_url(&self) -> String {
        format!("{}{}", self.base_url, self.path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_auth_variants() {
        let bearer = ApiAuth::Bearer("token".to_string());
        assert!(matches!(bearer, ApiAuth::Bearer(_)));

        let header = ApiAuth::HeaderKey("name".to_string(), "value".to_string());
        assert!(matches!(header, ApiAuth::HeaderKey(_, _)));

        let query = ApiAuth::QueryParam("key".to_string(), "value".to_string());
        assert!(matches!(query, ApiAuth::QueryParam(_, _)));

        let none = ApiAuth::None;
        assert!(matches!(none, ApiAuth::None));
    }

    #[test]
    fn test_api_endpoint_full_url() {
        let endpoint = ApiEndpoint {
            base_url: "https://api.example.com".to_string(),
            path: "/v1/test".to_string(),
            auth: ApiAuth::None,
        };

        assert_eq!(endpoint.full_url(), "https://api.example.com/v1/test");
    }

    #[test]
    fn test_api_endpoint_full_url_no_leading_slash() {
        let endpoint = ApiEndpoint {
            base_url: "https://api.example.com".to_string(),
            path: "v1/test".to_string(),
            auth: ApiAuth::None,
        };

        assert_eq!(endpoint.full_url(), "https://api.example.comv1/test");
    }

    #[test]
    fn test_api_auth_equality() {
        let bearer1 = ApiAuth::Bearer("token".to_string());
        let bearer2 = ApiAuth::Bearer("token".to_string());
        let bearer3 = ApiAuth::Bearer("different".to_string());

        assert_eq!(bearer1, bearer2);
        assert_ne!(bearer1, bearer3);
    }
}
