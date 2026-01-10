//! Request execution with tracing instrumentation.
//!
//! This module provides the [`ApiClient`] struct for executing HTTP requests
//! against REST API endpoints with automatic auth handling and tracing.

use std::time::Duration;

use reqwest::header::{HeaderMap, HeaderName, HeaderValue, AUTHORIZATION};
use tracing::{instrument, Span};
use url::Url;

use crate::auth::ApiAuthMethod;
use crate::endpoint::Endpoint;
use crate::error::{ApiError, AuthError, ClientError};
use crate::response::ResponseFormat;

/// Default request timeout in seconds.
const DEFAULT_TIMEOUT_SECS: u64 = 30;

/// Builder for configuring an [`ApiClient`].
#[derive(Debug)]
pub struct ApiClientBuilder {
    base_url: Url,
    timeout: Duration,
    default_headers: HeaderMap,
    auth: Option<(ApiAuthMethod, String)>,
}

impl ApiClientBuilder {
    /// Creates a new builder with the specified base URL.
    fn new(base_url: Url) -> Self {
        Self {
            base_url,
            timeout: Duration::from_secs(DEFAULT_TIMEOUT_SECS),
            default_headers: HeaderMap::new(),
            auth: None,
        }
    }

    /// Sets the request timeout.
    ///
    /// ## Examples
    ///
    /// ```rust,ignore
    /// use std::time::Duration;
    ///
    /// let client = ApiClient::builder(base_url)
    ///     .timeout(Duration::from_secs(60))
    ///     .build()?;
    /// ```
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Adds a default header to all requests.
    ///
    /// ## Examples
    ///
    /// ```rust,ignore
    /// let client = ApiClient::builder(base_url)
    ///     .default_header("X-Custom-Header", "value")?
    ///     .build()?;
    /// ```
    ///
    /// ## Errors
    ///
    /// Returns an error if the header name or value is invalid.
    pub fn default_header(
        mut self,
        name: impl AsRef<str>,
        value: impl AsRef<str>,
    ) -> Result<Self, ApiError> {
        let name = HeaderName::try_from(name.as_ref())
            .map_err(|e| ClientError::Connection(format!("invalid header name: {e}")))?;
        let value = HeaderValue::try_from(value.as_ref())
            .map_err(|e| ClientError::Connection(format!("invalid header value: {e}")))?;
        self.default_headers.insert(name, value);
        Ok(self)
    }

    /// Sets the authentication method and API key.
    ///
    /// ## Examples
    ///
    /// ```rust,ignore
    /// use api::ApiAuthMethod;
    ///
    /// let client = ApiClient::builder(base_url)
    ///     .auth(ApiAuthMethod::BearerToken, "sk-xxx")
    ///     .build()?;
    /// ```
    pub fn auth(mut self, method: ApiAuthMethod, api_key: impl Into<String>) -> Self {
        self.auth = Some((method, api_key.into()));
        self
    }

    /// Builds the [`ApiClient`].
    ///
    /// ## Errors
    ///
    /// Returns an error if the HTTP client cannot be constructed.
    pub fn build(self) -> Result<ApiClient, ApiError> {
        let client = reqwest::Client::builder()
            .timeout(self.timeout)
            .default_headers(self.default_headers)
            .pool_max_idle_per_host(10)
            .build()
            .map_err(ClientError::Request)?;

        Ok(ApiClient {
            client,
            base_url: self.base_url,
            auth: self.auth,
        })
    }
}

/// Async HTTP client for executing API requests.
///
/// The client wraps `reqwest::Client` with connection pooling and provides
/// type-safe request execution with automatic auth handling.
///
/// ## Examples
///
/// ```rust,ignore
/// use api::{ApiClient, Endpoint, RestMethod};
/// use api::response::JsonFormat;
/// use url::Url;
///
/// #[derive(serde::Deserialize)]
/// struct User { id: u64, name: String }
///
/// let base_url = Url::parse("https://api.example.com")?;
/// let client = ApiClient::builder(base_url).build()?;
///
/// let endpoint: Endpoint<JsonFormat<User>> = Endpoint::builder()
///     .id("get_user")
///     .method(RestMethod::Get)
///     .path("/users/1")
///     .build();
///
/// let user = client.execute(&endpoint).await?;
/// println!("User: {}", user.name);
/// ```
#[derive(Debug)]
pub struct ApiClient {
    client: reqwest::Client,
    base_url: Url,
    auth: Option<(ApiAuthMethod, String)>,
}

impl ApiClient {
    /// Creates a new builder for configuring an API client.
    ///
    /// ## Arguments
    ///
    /// * `base_url` - The base URL for all API requests.
    pub fn builder(base_url: Url) -> ApiClientBuilder {
        ApiClientBuilder::new(base_url)
    }

    /// Creates a new API client with default settings.
    ///
    /// ## Arguments
    ///
    /// * `base_url` - The base URL for all API requests.
    ///
    /// ## Errors
    ///
    /// Returns an error if the HTTP client cannot be constructed.
    pub fn new(base_url: Url) -> Result<Self, ApiError> {
        Self::builder(base_url).build()
    }

    /// Returns the base URL for this client.
    pub fn base_url(&self) -> &Url {
        &self.base_url
    }

    /// Executes an API request and parses the response.
    ///
    /// This method builds the request from the endpoint definition, applies
    /// authentication if configured, sends the request, and parses the response
    /// using the endpoint's response format.
    ///
    /// ## Type Parameters
    ///
    /// * `F` - The [`ResponseFormat`] for parsing the response.
    ///
    /// ## Arguments
    ///
    /// * `endpoint` - The endpoint definition to execute.
    ///
    /// ## Errors
    ///
    /// Returns an error if:
    /// - The request fails (network, timeout, etc.)
    /// - The server returns a non-success status code
    /// - The response cannot be parsed
    #[instrument(
        name = "api_request",
        skip(self, endpoint),
        fields(
            http.method = tracing::field::Empty,
            http.url = tracing::field::Empty,
            http.status_code = tracing::field::Empty,
            otel.kind = "client",
            otel.status_code = tracing::field::Empty,
        )
    )]
    pub async fn execute<F>(&self, endpoint: &Endpoint<F>) -> Result<F::Output, ApiError>
    where
        F: ResponseFormat,
    {
        // Record the method in the span
        Span::current().record("http.method", endpoint.method().to_string().as_str());
        let full_url = endpoint
            .full_url(&self.base_url)
            .map_err(|e| ClientError::Connection(format!("invalid URL: {e}")))?;

        // Record the full URL in the span
        Span::current().record("http.url", full_url.as_str());

        let mut request = self
            .client
            .request(endpoint.method().to_reqwest(), full_url.clone());

        // Apply authentication
        request = self.apply_auth(request, &full_url)?;

        // Send request
        let response = request.send().await.map_err(ClientError::Request)?;

        let status = response.status();
        let status_code = status.as_u16();

        // Record status in span
        Span::current().record("http.status_code", status_code);

        if !status.is_success() {
            // Try to extract error message from response body
            let message = response
                .text()
                .await
                .unwrap_or_else(|_| status.to_string());

            let otel_status = if status.is_server_error() {
                "ERROR"
            } else {
                "UNSET"
            };
            Span::current().record("otel.status_code", otel_status);

            // Check for auth-related status codes
            if status_code == 401 {
                return Err(AuthError::AuthenticationFailed { message }.into());
            }
            if status_code == 403 {
                return Err(AuthError::InsufficientPermissions {
                    operation: endpoint.id().to_string(),
                }
                .into());
            }

            return Err(ClientError::HttpStatus {
                status: status_code,
                message,
            }
            .into());
        }

        Span::current().record("otel.status_code", "OK");

        // Parse response body
        let body = response.bytes().await.map_err(ClientError::Request)?;
        let parsed = F::parse(body).await.map_err(ApiError::Validation)?;

        Ok(parsed)
    }

    /// Executes an API request with path parameter substitution.
    ///
    /// ## Arguments
    ///
    /// * `endpoint` - The endpoint definition to execute.
    /// * `params` - Path parameters to substitute in the URL template.
    ///
    /// ## Examples
    ///
    /// ```rust,ignore
    /// let user = client
    ///     .execute_with_params(&get_user_endpoint, &[("id", "123")])
    ///     .await?;
    /// ```
    #[instrument(
        name = "api_request",
        skip(self, endpoint),
        fields(
            http.method = tracing::field::Empty,
            http.url = tracing::field::Empty,
            http.status_code = tracing::field::Empty,
            otel.kind = "client",
            otel.status_code = tracing::field::Empty,
        )
    )]
    pub async fn execute_with_params<F>(
        &self,
        endpoint: &Endpoint<F>,
        params: &[(&str, &str)],
    ) -> Result<F::Output, ApiError>
    where
        F: ResponseFormat,
    {
        // Record the method in the span
        Span::current().record("http.method", endpoint.method().to_string().as_str());
        let substituted_path = endpoint.substitute_params(params);
        let full_url = self
            .base_url
            .join(&substituted_path)
            .map_err(|e| ClientError::Connection(format!("invalid URL: {e}")))?;

        // Record the full URL in the span
        Span::current().record("http.url", full_url.as_str());

        let mut request = self
            .client
            .request(endpoint.method().to_reqwest(), full_url.clone());

        // Apply authentication
        request = self.apply_auth(request, &full_url)?;

        // Send request
        let response = request.send().await.map_err(ClientError::Request)?;

        let status = response.status();
        let status_code = status.as_u16();

        // Record status in span
        Span::current().record("http.status_code", status_code);

        if !status.is_success() {
            let message = response
                .text()
                .await
                .unwrap_or_else(|_| status.to_string());

            let otel_status = if status.is_server_error() {
                "ERROR"
            } else {
                "UNSET"
            };
            Span::current().record("otel.status_code", otel_status);

            if status_code == 401 {
                return Err(AuthError::AuthenticationFailed { message }.into());
            }
            if status_code == 403 {
                return Err(AuthError::InsufficientPermissions {
                    operation: endpoint.id().to_string(),
                }
                .into());
            }

            return Err(ClientError::HttpStatus {
                status: status_code,
                message,
            }
            .into());
        }

        Span::current().record("otel.status_code", "OK");

        let body = response.bytes().await.map_err(ClientError::Request)?;
        let parsed = F::parse(body).await.map_err(ApiError::Validation)?;

        Ok(parsed)
    }

    /// Applies authentication to a request builder based on the configured auth method.
    fn apply_auth(
        &self,
        request: reqwest::RequestBuilder,
        url: &Url,
    ) -> Result<reqwest::RequestBuilder, ApiError> {
        let Some((method, api_key)) = &self.auth else {
            return Ok(request);
        };

        match method {
            ApiAuthMethod::BearerToken => {
                let header_value = format!("Bearer {api_key}");
                Ok(request.header(AUTHORIZATION, header_value))
            }
            ApiAuthMethod::ApiKey(header_name) => {
                let name = HeaderName::try_from(header_name.as_str())
                    .map_err(|_| AuthError::InvalidKeyFormat)?;
                Ok(request.header(name, api_key.as_str()))
            }
            ApiAuthMethod::QueryParam(param_name) => {
                // Append API key as query parameter
                let mut url_with_key = url.clone();
                url_with_key
                    .query_pairs_mut()
                    .append_pair(param_name, api_key);

                // Rebuild the request with the new URL
                // Note: We need to get the method from the original request
                // Since we can't easily modify just the URL, we'll use query() on reqwest
                Ok(request.query(&[(param_name.as_str(), api_key.as_str())]))
            }
            ApiAuthMethod::None => Ok(request),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ValidationError;
    use crate::method::RestMethod;
    use crate::response::JsonFormat;
    use wiremock::matchers::{header, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize)]
    struct TestResponse {
        id: u64,
        name: String,
    }

    #[tokio::test]
    async fn test_execute_get_json() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/users/1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(TestResponse {
                id: 1,
                name: "Alice".to_string(),
            }))
            .mount(&mock_server)
            .await;

        let base_url = Url::parse(&mock_server.uri()).unwrap();
        let client = ApiClient::new(base_url).unwrap();

        let endpoint: Endpoint<JsonFormat<TestResponse>> = Endpoint::builder()
            .id("get_user")
            .method(RestMethod::Get)
            .path("/users/1")
            .build();

        let result = client.execute(&endpoint).await.unwrap();
        assert_eq!(result.id, 1);
        assert_eq!(result.name, "Alice");
    }

    #[tokio::test]
    async fn test_execute_with_path_params() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/users/42"))
            .respond_with(ResponseTemplate::new(200).set_body_json(TestResponse {
                id: 42,
                name: "Bob".to_string(),
            }))
            .mount(&mock_server)
            .await;

        let base_url = Url::parse(&mock_server.uri()).unwrap();
        let client = ApiClient::new(base_url).unwrap();

        let endpoint: Endpoint<JsonFormat<TestResponse>> = Endpoint::builder()
            .id("get_user")
            .method(RestMethod::Get)
            .path("/users/{id}")
            .build();

        let result = client
            .execute_with_params(&endpoint, &[("id", "42")])
            .await
            .unwrap();
        assert_eq!(result.id, 42);
        assert_eq!(result.name, "Bob");
    }

    #[tokio::test]
    async fn test_bearer_token_auth() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/protected"))
            .and(header("authorization", "Bearer test-token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(TestResponse {
                id: 1,
                name: "Protected".to_string(),
            }))
            .mount(&mock_server)
            .await;

        let base_url = Url::parse(&mock_server.uri()).unwrap();
        let client = ApiClient::builder(base_url)
            .auth(ApiAuthMethod::BearerToken, "test-token")
            .build()
            .unwrap();

        let endpoint: Endpoint<JsonFormat<TestResponse>> = Endpoint::builder()
            .id("get_protected")
            .method(RestMethod::Get)
            .path("/protected")
            .build();

        let result = client.execute(&endpoint).await.unwrap();
        assert_eq!(result.name, "Protected");
    }

    #[tokio::test]
    async fn test_api_key_header_auth() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/api-key-protected"))
            .and(header("x-api-key", "my-secret-key"))
            .respond_with(ResponseTemplate::new(200).set_body_json(TestResponse {
                id: 2,
                name: "ApiKey".to_string(),
            }))
            .mount(&mock_server)
            .await;

        let base_url = Url::parse(&mock_server.uri()).unwrap();
        let client = ApiClient::builder(base_url)
            .auth(ApiAuthMethod::ApiKey("X-API-Key".to_string()), "my-secret-key")
            .build()
            .unwrap();

        let endpoint: Endpoint<JsonFormat<TestResponse>> = Endpoint::builder()
            .id("get_api_key_protected")
            .method(RestMethod::Get)
            .path("/api-key-protected")
            .build();

        let result = client.execute(&endpoint).await.unwrap();
        assert_eq!(result.name, "ApiKey");
    }

    #[tokio::test]
    async fn test_query_param_auth() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/query-auth"))
            .and(query_param("key", "gemini-key"))
            .respond_with(ResponseTemplate::new(200).set_body_json(TestResponse {
                id: 3,
                name: "QueryAuth".to_string(),
            }))
            .mount(&mock_server)
            .await;

        let base_url = Url::parse(&mock_server.uri()).unwrap();
        let client = ApiClient::builder(base_url)
            .auth(ApiAuthMethod::QueryParam("key".to_string()), "gemini-key")
            .build()
            .unwrap();

        let endpoint: Endpoint<JsonFormat<TestResponse>> = Endpoint::builder()
            .id("get_query_auth")
            .method(RestMethod::Get)
            .path("/query-auth")
            .build();

        let result = client.execute(&endpoint).await.unwrap();
        assert_eq!(result.name, "QueryAuth");
    }

    #[tokio::test]
    async fn test_http_error_401() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/unauthorized"))
            .respond_with(ResponseTemplate::new(401).set_body_string("Invalid token"))
            .mount(&mock_server)
            .await;

        let base_url = Url::parse(&mock_server.uri()).unwrap();
        let client = ApiClient::new(base_url).unwrap();

        let endpoint: Endpoint<JsonFormat<TestResponse>> = Endpoint::builder()
            .id("get_unauthorized")
            .method(RestMethod::Get)
            .path("/unauthorized")
            .build();

        let result = client.execute(&endpoint).await;
        assert!(matches!(
            result,
            Err(ApiError::Auth(AuthError::AuthenticationFailed { .. }))
        ));
    }

    #[tokio::test]
    async fn test_http_error_403() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/forbidden"))
            .respond_with(ResponseTemplate::new(403).set_body_string("Forbidden"))
            .mount(&mock_server)
            .await;

        let base_url = Url::parse(&mock_server.uri()).unwrap();
        let client = ApiClient::new(base_url).unwrap();

        let endpoint: Endpoint<JsonFormat<TestResponse>> = Endpoint::builder()
            .id("get_forbidden")
            .method(RestMethod::Get)
            .path("/forbidden")
            .build();

        let result = client.execute(&endpoint).await;
        assert!(matches!(
            result,
            Err(ApiError::Auth(AuthError::InsufficientPermissions { .. }))
        ));
    }

    #[tokio::test]
    async fn test_http_error_500() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/server-error"))
            .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
            .mount(&mock_server)
            .await;

        let base_url = Url::parse(&mock_server.uri()).unwrap();
        let client = ApiClient::new(base_url).unwrap();

        let endpoint: Endpoint<JsonFormat<TestResponse>> = Endpoint::builder()
            .id("get_server_error")
            .method(RestMethod::Get)
            .path("/server-error")
            .build();

        let result = client.execute(&endpoint).await;
        assert!(matches!(
            result,
            Err(ApiError::Client(ClientError::HttpStatus { status: 500, .. }))
        ));
    }

    #[tokio::test]
    async fn test_json_parse_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/invalid-json"))
            .respond_with(ResponseTemplate::new(200).set_body_string("not valid json"))
            .mount(&mock_server)
            .await;

        let base_url = Url::parse(&mock_server.uri()).unwrap();
        let client = ApiClient::new(base_url).unwrap();

        let endpoint: Endpoint<JsonFormat<TestResponse>> = Endpoint::builder()
            .id("get_invalid_json")
            .method(RestMethod::Get)
            .path("/invalid-json")
            .build();

        let result = client.execute(&endpoint).await;
        assert!(matches!(
            result,
            Err(ApiError::Validation(ValidationError::JsonParse(_)))
        ));
    }

    #[tokio::test]
    async fn test_custom_timeout() {
        let base_url = Url::parse("https://example.com").unwrap();
        let client = ApiClient::builder(base_url)
            .timeout(Duration::from_secs(60))
            .build()
            .unwrap();

        // Just verify the client was built successfully
        assert_eq!(client.base_url().as_str(), "https://example.com/");
    }

    #[tokio::test]
    async fn test_default_header() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/with-header"))
            .and(header("x-custom-header", "custom-value"))
            .respond_with(ResponseTemplate::new(200).set_body_json(TestResponse {
                id: 1,
                name: "CustomHeader".to_string(),
            }))
            .mount(&mock_server)
            .await;

        let base_url = Url::parse(&mock_server.uri()).unwrap();
        let client = ApiClient::builder(base_url)
            .default_header("X-Custom-Header", "custom-value")
            .unwrap()
            .build()
            .unwrap();

        let endpoint: Endpoint<JsonFormat<TestResponse>> = Endpoint::builder()
            .id("get_with_header")
            .method(RestMethod::Get)
            .path("/with-header")
            .build();

        let result = client.execute(&endpoint).await.unwrap();
        assert_eq!(result.name, "CustomHeader");
    }

    #[tokio::test]
    async fn test_no_auth() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/no-auth"))
            .respond_with(ResponseTemplate::new(200).set_body_json(TestResponse {
                id: 1,
                name: "NoAuth".to_string(),
            }))
            .mount(&mock_server)
            .await;

        let base_url = Url::parse(&mock_server.uri()).unwrap();
        let client = ApiClient::builder(base_url)
            .auth(ApiAuthMethod::None, "ignored")
            .build()
            .unwrap();

        let endpoint: Endpoint<JsonFormat<TestResponse>> = Endpoint::builder()
            .id("get_no_auth")
            .method(RestMethod::Get)
            .path("/no-auth")
            .build();

        let result = client.execute(&endpoint).await.unwrap();
        assert_eq!(result.name, "NoAuth");
    }
}
