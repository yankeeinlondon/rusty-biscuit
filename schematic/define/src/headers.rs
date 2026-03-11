//! HTTP header management for REST APIs.
//!
//! This module provides types for configuring dynamic HTTP headers based on
//! environment variables, including secure credential handling via [`SensitiveString`].
//!
//! ## Security
//!
//! - [`SensitiveString`] does NOT implement `PartialEq`/`Eq` to prevent timing attacks
//! - Debug output for sensitive values is redacted to `"***"`
//! - All credential types use `SensitiveString` internally
//!
//! ## Examples
//!
//! Create an environment variable list with fallback:
//!
//! ```
//! use schematic_define::EnvList;
//!
//! let env = EnvList::from_strs(&["OPENAI_API_KEY", "OPENAI_KEY"]);
//! assert_eq!(env.names().len(), 2);
//! ```
//!
//! Define API key header from environment:
//!
//! ```
//! use schematic_define::{ApiKeyEnv, EnvList};
//!
//! let api_key = ApiKeyEnv {
//!     names: EnvList::single("X_API_KEY"),
//!     header: "X-API-Key".to_string(),
//! };
//! ```

use std::fmt;
use thiserror::Error;

/// A string value that should be treated as sensitive (passwords, tokens, etc.).
///
/// This type provides secure handling of sensitive strings by:
/// - Redacting Debug output to prevent accidental logging
/// - NOT implementing PartialEq/Eq to prevent timing attacks in comparisons
///
/// ## Examples
///
/// ```
/// use schematic_define::SensitiveString;
///
/// let secret = SensitiveString::from("my-secret-token");
/// assert_eq!(format!("{:?}", secret), "SensitiveString(\"***\")");
/// ```
///
/// Access the inner value:
///
/// ```
/// use schematic_define::SensitiveString;
///
/// let secret = SensitiveString::from("my-secret-token");
/// assert_eq!(secret.as_str(), "my-secret-token");
/// ```
#[derive(Clone, Default)]
pub struct SensitiveString(String);

impl SensitiveString {
    /// Create a new sensitive string from any string-like value.
    ///
    /// ## Examples
    ///
    /// ```
    /// use schematic_define::SensitiveString;
    ///
    /// let secret = SensitiveString::new("my-token");
    /// assert_eq!(secret.as_str(), "my-token");
    /// ```
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Get the underlying string value as a string slice.
    ///
    /// ## Examples
    ///
    /// ```
    /// use schematic_define::SensitiveString;
    ///
    /// let secret = SensitiveString::from("my-token");
    /// assert_eq!(secret.as_str(), "my-token");
    /// ```
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consume the wrapper and return the inner String.
    ///
    /// ## Examples
    ///
    /// ```
    /// use schematic_define::SensitiveString;
    ///
    /// let secret = SensitiveString::from("my-token");
    /// let inner = secret.into_inner();
    /// assert_eq!(inner, "my-token");
    /// ```
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl fmt::Debug for SensitiveString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("SensitiveString").field(&"***").finish()
    }
}

impl From<String> for SensitiveString {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for SensitiveString {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

/// A list of environment variable names to check in priority order.
///
/// The first environment variable that is set will be used. This provides
/// a fallback mechanism for credentials when different environments may
/// use different variable names.
///
/// ## Examples
///
/// Create from a single variable:
///
/// ```
/// use schematic_define::EnvList;
///
/// let env = EnvList::single("API_KEY");
/// assert_eq!(env.names().len(), 1);
/// ```
///
/// Create from multiple variables (fallback chain):
///
/// ```
/// use schematic_define::EnvList;
///
/// let env = EnvList::from_strs(&["OPENAI_API_KEY", "OPENAI_KEY", "API_KEY"]);
/// assert_eq!(env.names().len(), 3);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct EnvList {
    /// Environment variable names in priority order (first match wins).
    names: Vec<String>,
}

impl EnvList {
    /// Create a new environment variable list.
    ///
    /// ## Examples
    ///
    /// ```
    /// use schematic_define::EnvList;
    ///
    /// let env = EnvList::new(vec!["KEY1".to_string(), "KEY2".to_string()]);
    /// assert_eq!(env.names().len(), 2);
    /// ```
    pub fn new(names: Vec<String>) -> Self {
        Self { names }
    }

    /// Create an environment list from a single variable name.
    ///
    /// ## Examples
    ///
    /// ```
    /// use schematic_define::EnvList;
    ///
    /// let env = EnvList::single("OPENAI_API_KEY");
    /// assert_eq!(env.names(), &["OPENAI_API_KEY"]);
    /// ```
    pub fn single(name: impl Into<String>) -> Self {
        Self {
            names: vec![name.into()],
        }
    }

    /// Create an environment list from a slice of string references.
    ///
    /// This is a convenience method for creating lists from string literals.
    ///
    /// ## Examples
    ///
    /// ```
    /// use schematic_define::EnvList;
    ///
    /// let env = EnvList::from_strs(&["KEY1", "KEY2", "KEY3"]);
    /// assert_eq!(env.names().len(), 3);
    /// ```
    pub fn from_strs(names: &[&str]) -> Self {
        Self {
            names: names.iter().map(|s| s.to_string()).collect(),
        }
    }

    /// Get a reference to the environment variable names.
    ///
    /// ## Examples
    ///
    /// ```
    /// use schematic_define::EnvList;
    ///
    /// let env = EnvList::from_strs(&["KEY1", "KEY2"]);
    /// assert_eq!(env.names(), &["KEY1", "KEY2"]);
    /// ```
    pub fn names(&self) -> &[String] {
        &self.names
    }
}

/// API key header configuration with environment variable source.
///
/// Defines where to read an API key from environment variables and which
/// HTTP header to place it in.
///
/// ## Examples
///
/// ```
/// use schematic_define::{ApiKeyEnv, EnvList};
///
/// let api_key = ApiKeyEnv {
///     names: EnvList::from_strs(&["HUGGINGFACE_API_KEY", "HF_TOKEN"]),
///     header: "Authorization".to_string(),
/// };
/// ```
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ApiKeyEnv {
    /// Environment variables to check (in priority order).
    pub names: EnvList,
    /// HTTP header name to set (e.g., "X-API-Key", "Authorization").
    pub header: String,
}

/// Environment variable mapping for authentication credentials.
///
/// This struct provides a comprehensive way to configure all authentication
/// credentials via environment variables, with fallback chains for each type.
///
/// ## Examples
///
/// Bearer token only:
///
/// ```
/// use schematic_define::{EnvMapping, EnvList};
///
/// let mapping = EnvMapping {
///     bearer_token: Some(EnvList::single("OPENAI_API_KEY")),
///     basic_user: None,
///     basic_pass: None,
///     api_key: None,
///     ..Default::default()
/// };
/// ```
///
/// Basic auth with username and password:
///
/// ```
/// use schematic_define::{EnvMapping, EnvList};
///
/// let mapping = EnvMapping {
///     bearer_token: None,
///     basic_user: Some(EnvList::single("API_USERNAME")),
///     basic_pass: Some(EnvList::single("API_PASSWORD")),
///     api_key: None,
///     ..Default::default()
/// };
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct EnvMapping {
    /// Environment variables for bearer token (e.g., for Authorization: Bearer TOKEN).
    pub bearer_token: Option<EnvList>,
    /// Environment variables for basic auth username.
    pub basic_user: Option<EnvList>,
    /// Environment variables for basic auth password.
    pub basic_pass: Option<EnvList>,
    /// Environment variables for API key with custom header.
    pub api_key: Option<ApiKeyEnv>,
    /// Environment variables for OAuth2 client ID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub oauth_client_id: Option<EnvList>,
    /// Environment variables for OAuth2 client secret.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub oauth_client_secret: Option<EnvList>,
    /// Environment variables for OAuth2 redirect URI.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub oauth_redirect_uri: Option<EnvList>,
}

/// Errors that can occur when working with headers.
///
/// ## Examples
///
/// ```
/// use schematic_define::HeaderError;
///
/// let error = HeaderError::InvalidHeaderName("Inválid-Héader".to_string());
/// assert!(error.to_string().contains("Invalid header name"));
/// ```
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum HeaderError {
    /// The header name contains invalid characters.
    ///
    /// Header names must be ASCII and follow HTTP header field name rules.
    #[error("Invalid header name: {0}")]
    InvalidHeaderName(String),

    /// The header value contains invalid characters.
    ///
    /// Header values must not contain control characters (except HTAB).
    #[error("Invalid header value for header '{0}'")]
    InvalidHeaderValue(String),

    /// Required environment variable was not set.
    #[error("Environment variable not set: {0}")]
    MissingEnv(String),

    /// Required credential is missing (no env vars were set).
    #[error("Missing credential: none of the environment variables were set: {0:?}")]
    MissingCredential(Vec<String>),
}

/// HTTP headers builder with support for authentication strategies.
///
/// This builder provides a fluent API for constructing HTTP headers, including
/// specialized methods for common authentication patterns (Bearer tokens, Basic auth,
/// API keys). It validates header names during the build phase.
///
/// ## Examples
///
/// Bearer token authentication:
///
/// ```
/// use schematic_define::Headers;
///
/// let headers = Headers::default()
///     .use_bearer_token("my-secret-token")
///     .accept_json()
///     .build()
///     .unwrap();
///
/// assert!(headers.contains(&("Authorization".to_string(), "Bearer my-secret-token".to_string())));
/// ```
///
/// Basic authentication:
///
/// ```
/// use schematic_define::Headers;
///
/// let headers = Headers::default()
///     .use_basic_auth("username", "password")
///     .build()
///     .unwrap();
///
/// // Authorization header contains Base64-encoded credentials
/// assert_eq!(headers[0].0, "Authorization");
/// assert!(headers[0].1.starts_with("Basic "));
/// ```
///
/// Custom headers:
///
/// ```
/// use schematic_define::Headers;
///
/// let headers = Headers::default()
///     .header("X-API-Key", "my-key")
///     .header("X-Request-ID", "12345")
///     .user_agent("MyClient/1.0")
///     .build()
///     .unwrap();
///
/// assert_eq!(headers.len(), 3);
/// ```
#[derive(Debug, Clone, Default)]
pub struct Headers {
    /// Pre-encoded authorization header value (e.g., "Bearer TOKEN" or "Basic BASE64").
    authorization: Option<SensitiveString>,
    /// Content-Type header value.
    content_type: Option<String>,
    /// Accept header value.
    accept: Option<String>,
    /// User-Agent header value.
    user_agent: Option<String>,
    /// Custom headers (name, value pairs).
    custom: Vec<(String, String)>,
    /// Environment variable mapping for dynamic credential resolution.
    env_mapping: EnvMapping,
}

impl Headers {
    /// Create a new empty headers builder.
    ///
    /// ## Examples
    ///
    /// ```
    /// use schematic_define::Headers;
    ///
    /// let headers = Headers::new();
    /// assert!(headers.build().unwrap().is_empty());
    /// ```
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the environment variable mapping for dynamic credential resolution.
    ///
    /// This configures which environment variables to check when calling
    /// [`from_env`](Self::from_env) or [`try_from_env`](Self::try_from_env).
    ///
    /// ## Examples
    ///
    /// ```
    /// use schematic_define::{Headers, EnvMapping, EnvList};
    ///
    /// let mapping = EnvMapping {
    ///     bearer_token: Some(EnvList::single("API_TOKEN")),
    ///     basic_user: None,
    ///     basic_pass: None,
    ///     api_key: None,
    ///     ..Default::default()
    /// };
    ///
    /// let headers = Headers::default()
    ///     .with_env_mapping(mapping);
    /// ```
    pub fn with_env_mapping(mut self, mapping: EnvMapping) -> Self {
        self.env_mapping = mapping;
        self
    }

    /// Set Bearer token authentication.
    ///
    /// This sets the Authorization header to "Bearer {token}".
    /// If called multiple times, the last value wins.
    ///
    /// ## Examples
    ///
    /// ```
    /// use schematic_define::Headers;
    ///
    /// let headers = Headers::default()
    ///     .use_bearer_token("my-token")
    ///     .build()
    ///     .unwrap();
    ///
    /// assert_eq!(headers[0].1, "Bearer my-token");
    /// ```
    pub fn use_bearer_token(mut self, token: impl Into<String>) -> Self {
        self.authorization = Some(SensitiveString::new(format!("Bearer {}", token.into())));
        self
    }

    /// Set Basic authentication credentials.
    ///
    /// This encodes the username and password as Base64 and sets the
    /// Authorization header to "Basic {encoded}".
    /// If called multiple times, the last value wins.
    ///
    /// ## Examples
    ///
    /// ```
    /// use schematic_define::Headers;
    ///
    /// let headers = Headers::default()
    ///     .use_basic_auth("user", "pass")
    ///     .build()
    ///     .unwrap();
    ///
    /// assert!(headers[0].1.starts_with("Basic "));
    /// ```
    pub fn use_basic_auth(
        mut self,
        username: impl Into<String>,
        password: impl Into<String>,
    ) -> Self {
        use base64::Engine;
        let credentials = format!("{}:{}", username.into(), password.into());
        let encoded = base64::engine::general_purpose::STANDARD.encode(credentials);
        self.authorization = Some(SensitiveString::new(format!("Basic {}", encoded)));
        self
    }

    /// Set API key authentication with a custom header.
    ///
    /// This adds a custom header with the specified name and API key value.
    ///
    /// ## Examples
    ///
    /// ```
    /// use schematic_define::Headers;
    ///
    /// let headers = Headers::default()
    ///     .use_api_key("my-key", "X-API-Key")
    ///     .build()
    ///     .unwrap();
    ///
    /// assert_eq!(headers[0].0, "X-API-Key");
    /// assert_eq!(headers[0].1, "my-key");
    /// ```
    pub fn use_api_key(self, key: impl Into<String>, header_name: impl Into<String>) -> Self {
        self.header(header_name, key)
    }

    /// Set the User-Agent header.
    ///
    /// ## Examples
    ///
    /// ```
    /// use schematic_define::Headers;
    ///
    /// let headers = Headers::default()
    ///     .user_agent("MyClient/1.0")
    ///     .build()
    ///     .unwrap();
    ///
    /// assert_eq!(headers[0].0, "User-Agent");
    /// ```
    pub fn user_agent(mut self, agent: impl Into<String>) -> Self {
        self.user_agent = Some(agent.into());
        self
    }

    /// Set the Content-Type header.
    ///
    /// ## Examples
    ///
    /// ```
    /// use schematic_define::Headers;
    ///
    /// let headers = Headers::default()
    ///     .content_type("application/json")
    ///     .build()
    ///     .unwrap();
    ///
    /// assert_eq!(headers[0].0, "Content-Type");
    /// ```
    pub fn content_type(mut self, content_type: impl Into<String>) -> Self {
        self.content_type = Some(content_type.into());
        self
    }

    /// Set the Accept header.
    ///
    /// ## Examples
    ///
    /// ```
    /// use schematic_define::Headers;
    ///
    /// let headers = Headers::default()
    ///     .accept("application/json")
    ///     .build()
    ///     .unwrap();
    ///
    /// assert_eq!(headers[0].0, "Accept");
    /// ```
    pub fn accept(mut self, accept: impl Into<String>) -> Self {
        self.accept = Some(accept.into());
        self
    }

    /// Convenience method to set Accept: application/json.
    ///
    /// ## Examples
    ///
    /// ```
    /// use schematic_define::Headers;
    ///
    /// let headers = Headers::default()
    ///     .accept_json()
    ///     .build()
    ///     .unwrap();
    ///
    /// assert_eq!(headers[0].1, "application/json");
    /// ```
    pub fn accept_json(self) -> Self {
        self.accept("application/json")
    }

    /// Convenience method to set Content-Type: application/json.
    ///
    /// ## Examples
    ///
    /// ```
    /// use schematic_define::Headers;
    ///
    /// let headers = Headers::default()
    ///     .content_type_json()
    ///     .build()
    ///     .unwrap();
    ///
    /// assert_eq!(headers[0].0, "Content-Type");
    /// assert_eq!(headers[0].1, "application/json");
    /// ```
    pub fn content_type_json(self) -> Self {
        self.content_type("application/json")
    }

    /// Add a custom header.
    ///
    /// This can be used to add any additional headers beyond the standard ones.
    /// If the same header name is added multiple times, the last value wins.
    ///
    /// ## Examples
    ///
    /// ```
    /// use schematic_define::Headers;
    ///
    /// let headers = Headers::default()
    ///     .header("X-Request-ID", "12345")
    ///     .header("X-Custom", "value")
    ///     .build()
    ///     .unwrap();
    ///
    /// assert_eq!(headers.len(), 2);
    /// ```
    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        let name = name.into();
        let value = value.into();

        // Remove existing header with this name (last value wins)
        self.custom.retain(|(k, _)| k != &name);
        self.custom.push((name, value));
        self
    }

    /// Remove a header by name.
    ///
    /// This removes both custom headers and standard headers (Content-Type, Accept, User-Agent).
    /// Does nothing if the header doesn't exist.
    ///
    /// ## Examples
    ///
    /// ```
    /// use schematic_define::Headers;
    ///
    /// let headers = Headers::default()
    ///     .header("X-Custom", "value")
    ///     .content_type("text/plain")
    ///     .remove("X-Custom")
    ///     .build()
    ///     .unwrap();
    ///
    /// assert_eq!(headers.len(), 1);
    /// assert_eq!(headers[0].0, "Content-Type");
    /// ```
    pub fn remove(mut self, name: &str) -> Self {
        // Remove from custom headers
        self.custom.retain(|(k, _)| k != name);

        // Remove from standard headers
        match name {
            "Content-Type" => self.content_type = None,
            "Accept" => self.accept = None,
            "User-Agent" => self.user_agent = None,
            "Authorization" => self.authorization = None,
            _ => {}
        }

        self
    }

    /// Load credentials from environment variables (permissive).
    ///
    /// This method attempts to resolve credentials from environment variables
    /// according to the `env_mapping` configuration. If environment variables
    /// are not set, it silently skips them. Pre-set values are NOT overwritten.
    ///
    /// The first non-empty value in each fallback chain wins.
    ///
    /// ## Examples
    ///
    /// ```no_run
    /// use schematic_define::{Headers, EnvMapping, EnvList};
    ///
    /// let mapping = EnvMapping {
    ///     bearer_token: Some(EnvList::from_strs(&["OPENAI_API_KEY", "OPENAI_KEY"])),
    ///     basic_user: None,
    ///     basic_pass: None,
    ///     api_key: None,
    ///     ..Default::default()
    /// };
    ///
    /// let headers = Headers::default()
    ///     .with_env_mapping(mapping)
    ///     .from_env();
    /// ```
    pub fn from_env(self) -> Self {
        let mapping = self.env_mapping.clone();
        let fallback = self.clone();
        self.from_env_internal(&mapping, false).unwrap_or(fallback)
    }

    /// Load credentials from environment variables with custom mapping (permissive).
    ///
    /// Like [`from_env`](Self::from_env) but uses a custom mapping instead of
    /// the builder's `env_mapping` field.
    ///
    /// ## Examples
    ///
    /// ```no_run
    /// use schematic_define::{Headers, EnvMapping, EnvList};
    ///
    /// let custom_mapping = EnvMapping {
    ///     bearer_token: Some(EnvList::single("CUSTOM_TOKEN")),
    ///     basic_user: None,
    ///     basic_pass: None,
    ///     api_key: None,
    ///     ..Default::default()
    /// };
    ///
    /// let headers = Headers::default().from_env_with(custom_mapping);
    /// ```
    pub fn from_env_with(self, mapping: EnvMapping) -> Self {
        // Clone self for fallback before moving into from_env_internal
        let fallback = self.clone();
        self.from_env_internal(&mapping, false).unwrap_or(fallback)
    }

    /// Load credentials from environment variables (strict).
    ///
    /// Like [`from_env`](Self::from_env) but returns an error if any required
    /// environment variable chain is not set. A chain is "required" if it is
    /// configured in the mapping but none of the variables in the chain are set.
    ///
    /// ## Returns
    ///
    /// The updated headers builder.
    ///
    /// ## Errors
    ///
    /// Returns [`HeaderError::MissingCredential`] if any required environment
    /// variable chain has no value set.
    ///
    /// ## Examples
    ///
    /// ```no_run
    /// use schematic_define::{Headers, EnvMapping, EnvList};
    ///
    /// let mapping = EnvMapping {
    ///     bearer_token: Some(EnvList::single("REQUIRED_TOKEN")),
    ///     basic_user: None,
    ///     basic_pass: None,
    ///     api_key: None,
    ///     ..Default::default()
    /// };
    ///
    /// let headers = Headers::default()
    ///     .with_env_mapping(mapping)
    ///     .try_from_env()?;
    /// # Ok::<(), schematic_define::HeaderError>(())
    /// ```
    pub fn try_from_env(self) -> Result<Self, HeaderError> {
        let mapping = self.env_mapping.clone();
        self.from_env_internal(&mapping, true)
    }

    /// Internal implementation for environment variable resolution.
    ///
    /// ## Parameters
    ///
    /// - `mapping`: The environment variable mapping to use
    /// - `strict`: If true, returns error on missing credentials; if false, silently skips
    ///
    /// ## Returns
    ///
    /// The updated headers builder.
    ///
    /// ## Errors
    ///
    /// Returns [`HeaderError::MissingCredential`] if `strict` is true and any
    /// required credential is missing.
    #[allow(clippy::wrong_self_convention)]
    fn from_env_internal(
        mut self,
        mapping: &EnvMapping,
        strict: bool,
    ) -> Result<Self, HeaderError> {
        // Only load ENV values if authorization is not already set
        if self.authorization.is_none() {
            // Try bearer token
            if let Some(ref env_list) = mapping.bearer_token {
                if let Some(token) = resolve_env_list(env_list) {
                    self = self.use_bearer_token(token);
                } else if strict {
                    return Err(HeaderError::MissingCredential(env_list.names().to_vec()));
                }
            }

            // Try basic auth (only if bearer token not set)
            if self.authorization.is_none()
                && let (Some(user_list), Some(pass_list)) =
                    (&mapping.basic_user, &mapping.basic_pass)
            {
                let user = resolve_env_list(user_list);
                let pass = resolve_env_list(pass_list);

                match (user, pass) {
                    (Some(u), Some(p)) => {
                        self = self.use_basic_auth(u, p);
                    }
                    (None, _) if strict => {
                        return Err(HeaderError::MissingCredential(user_list.names().to_vec()));
                    }
                    (_, None) if strict => {
                        return Err(HeaderError::MissingCredential(pass_list.names().to_vec()));
                    }
                    _ => {}
                }
            }
        }

        // Try API key (custom header, not part of authorization)
        if let Some(ref api_key_env) = mapping.api_key {
            // Check if custom header is already set
            let header_exists = self
                .custom
                .iter()
                .any(|(name, _)| name == &api_key_env.header);

            if !header_exists {
                if let Some(key) = resolve_env_list(&api_key_env.names) {
                    self = self.header(&api_key_env.header, key);
                } else if strict {
                    return Err(HeaderError::MissingCredential(
                        api_key_env.names.names().to_vec(),
                    ));
                }
            }
        }

        Ok(self)
    }

    /// Returns `true` if an authorization header has been set programmatically.
    ///
    /// This includes headers set via [`use_bearer_token`](Self::use_bearer_token),
    /// [`use_basic_auth`](Self::use_basic_auth), or loaded from environment variables.
    ///
    /// Use this to check whether authentication will be handled by the `Headers`
    /// builder, allowing you to skip redundant environment-based auth checks.
    ///
    /// ## Examples
    ///
    /// ```
    /// use schematic_define::Headers;
    ///
    /// let headers = Headers::default();
    /// assert!(!headers.has_authorization());
    ///
    /// let headers = Headers::default().use_bearer_token("token");
    /// assert!(headers.has_authorization());
    /// ```
    #[must_use]
    pub fn has_authorization(&self) -> bool {
        self.authorization.is_some()
    }

    /// Build the final header list.
    ///
    /// This validates all header names are ASCII and returns a vector of
    /// (name, value) tuples ready for use with HTTP clients.
    ///
    /// ## Returns
    ///
    /// A vector of header name-value pairs.
    ///
    /// ## Errors
    ///
    /// Returns [`HeaderError::InvalidHeaderName`] if any header name contains
    /// non-ASCII characters or invalid characters for HTTP header names.
    ///
    /// ## Examples
    ///
    /// ```
    /// use schematic_define::Headers;
    ///
    /// let headers = Headers::default()
    ///     .use_bearer_token("token")
    ///     .accept_json()
    ///     .build()
    ///     .unwrap();
    ///
    /// assert_eq!(headers.len(), 2);
    /// ```
    pub fn build(self) -> Result<Vec<(String, String)>, HeaderError> {
        let mut result = Vec::new();

        // Add authorization if present
        if let Some(auth) = self.authorization {
            validate_header_name("Authorization")?;
            result.push(("Authorization".to_string(), auth.into_inner()));
        }

        // Add content-type if present
        if let Some(ct) = self.content_type {
            validate_header_name("Content-Type")?;
            result.push(("Content-Type".to_string(), ct));
        }

        // Add accept if present
        if let Some(accept) = self.accept {
            validate_header_name("Accept")?;
            result.push(("Accept".to_string(), accept));
        }

        // Add user-agent if present
        if let Some(ua) = self.user_agent {
            validate_header_name("User-Agent")?;
            result.push(("User-Agent".to_string(), ua));
        }

        // Add custom headers
        for (name, value) in self.custom {
            validate_header_name(&name)?;
            result.push((name, value));
        }

        Ok(result)
    }
}

/// Validate that a header name is valid ASCII and follows HTTP header field name rules.
///
/// ## Errors
///
/// Returns [`HeaderError::InvalidHeaderName`] if the name contains non-ASCII
/// characters, spaces, or other invalid characters.
fn validate_header_name(name: &str) -> Result<(), HeaderError> {
    // Header names must be ASCII
    if !name.is_ascii() {
        return Err(HeaderError::InvalidHeaderName(name.to_string()));
    }

    // Header names cannot contain spaces or control characters
    // Valid characters: alphanumeric, hyphen, underscore
    for ch in name.chars() {
        if !ch.is_ascii_alphanumeric() && ch != '-' && ch != '_' {
            return Err(HeaderError::InvalidHeaderName(name.to_string()));
        }
    }

    Ok(())
}

/// Resolve an environment variable from a list of candidates.
///
/// Returns the first non-empty value found, or None if all fail.
/// This implements the fallback chain behavior where the first matching
/// environment variable wins.
fn resolve_env_list(env_list: &EnvList) -> Option<String> {
    for name in env_list.names() {
        if let Ok(value) = std::env::var(name)
            && !value.is_empty()
        {
            return Some(value);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================
    // SensitiveString Tests
    // ========================================

    #[test]
    fn sensitive_string_debug_redacts_value() {
        let secret = SensitiveString::from("super-secret-token");
        let debug_output = format!("{:?}", secret);

        assert_eq!(debug_output, "SensitiveString(\"***\")");
        assert!(!debug_output.contains("super-secret-token"));
    }

    #[test]
    fn sensitive_string_as_str_returns_value() {
        let secret = SensitiveString::from("my-token");
        assert_eq!(secret.as_str(), "my-token");
    }

    #[test]
    fn sensitive_string_into_inner_returns_value() {
        let secret = SensitiveString::from("my-token");
        assert_eq!(secret.into_inner(), "my-token");
    }

    #[test]
    fn sensitive_string_default_is_empty() {
        let secret = SensitiveString::default();
        assert_eq!(secret.as_str(), "");
    }

    #[test]
    fn sensitive_string_clone_works() {
        let secret = SensitiveString::from("token");
        let cloned = secret.clone();
        assert_eq!(secret.as_str(), cloned.as_str());
    }

    #[test]
    fn sensitive_string_from_string() {
        let secret = SensitiveString::from(String::from("token"));
        assert_eq!(secret.as_str(), "token");
    }

    #[test]
    fn sensitive_string_from_str() {
        let secret = SensitiveString::from("token");
        assert_eq!(secret.as_str(), "token");
    }

    #[test]
    fn sensitive_string_new_method() {
        let secret = SensitiveString::new("token");
        assert_eq!(secret.as_str(), "token");
    }

    // ========================================
    // EnvList Tests
    // ========================================

    #[test]
    fn env_list_new_creates_list() {
        let env = EnvList::new(vec!["KEY1".to_string(), "KEY2".to_string()]);
        assert_eq!(env.names().len(), 2);
        assert_eq!(env.names()[0], "KEY1");
        assert_eq!(env.names()[1], "KEY2");
    }

    #[test]
    fn env_list_single_creates_single_element() {
        let env = EnvList::single("OPENAI_API_KEY");
        assert_eq!(env.names().len(), 1);
        assert_eq!(env.names()[0], "OPENAI_API_KEY");
    }

    #[test]
    fn env_list_from_strs_convenience() {
        let env = EnvList::from_strs(&["KEY1", "KEY2", "KEY3"]);
        assert_eq!(env.names().len(), 3);
        assert_eq!(env.names()[0], "KEY1");
        assert_eq!(env.names()[1], "KEY2");
        assert_eq!(env.names()[2], "KEY3");
    }

    #[test]
    fn env_list_names_returns_slice() {
        let env = EnvList::from_strs(&["A", "B"]);
        let names = env.names();
        assert_eq!(names, &["A", "B"]);
    }

    #[test]
    fn env_list_clone_works() {
        let env = EnvList::from_strs(&["KEY1", "KEY2"]);
        let cloned = env.clone();
        assert_eq!(env, cloned);
    }

    #[test]
    fn env_list_debug_impl() {
        let env = EnvList::single("TEST_KEY");
        let debug = format!("{:?}", env);
        assert!(debug.contains("EnvList"));
        assert!(debug.contains("TEST_KEY"));
    }

    // ========================================
    // ApiKeyEnv Tests
    // ========================================

    #[test]
    fn api_key_env_construction() {
        let api_key = ApiKeyEnv {
            names: EnvList::from_strs(&["HF_TOKEN", "HUGGINGFACE_TOKEN"]),
            header: "Authorization".to_string(),
        };

        assert_eq!(api_key.names.names().len(), 2);
        assert_eq!(api_key.header, "Authorization");
    }

    #[test]
    fn api_key_env_clone() {
        let api_key = ApiKeyEnv {
            names: EnvList::single("API_KEY"),
            header: "X-API-Key".to_string(),
        };

        let cloned = api_key.clone();
        assert_eq!(api_key, cloned);
    }

    #[test]
    fn api_key_env_debug() {
        let api_key = ApiKeyEnv {
            names: EnvList::single("KEY"),
            header: "X-Key".to_string(),
        };

        let debug = format!("{:?}", api_key);
        assert!(debug.contains("ApiKeyEnv"));
        assert!(debug.contains("KEY"));
        assert!(debug.contains("X-Key"));
    }

    // ========================================
    // EnvMapping Tests
    // ========================================

    #[test]
    fn env_mapping_default_is_empty() {
        let mapping = EnvMapping::default();
        assert!(mapping.bearer_token.is_none());
        assert!(mapping.basic_user.is_none());
        assert!(mapping.basic_pass.is_none());
        assert!(mapping.api_key.is_none());
    }

    #[test]
    fn env_mapping_bearer_token_only() {
        let mapping = EnvMapping {
            bearer_token: Some(EnvList::single("OPENAI_API_KEY")),
            basic_user: None,
            basic_pass: None,
            api_key: None,
        ..Default::default()
        };

        assert!(mapping.bearer_token.is_some());
        assert!(mapping.basic_user.is_none());
    }

    #[test]
    fn env_mapping_basic_auth() {
        let mapping = EnvMapping {
            bearer_token: None,
            basic_user: Some(EnvList::single("API_USER")),
            basic_pass: Some(EnvList::single("API_PASS")),
            api_key: None,
        ..Default::default()
        };

        assert!(mapping.basic_user.is_some());
        assert!(mapping.basic_pass.is_some());
        assert!(mapping.bearer_token.is_none());
    }

    #[test]
    fn env_mapping_api_key() {
        let mapping = EnvMapping {
            bearer_token: None,
            basic_user: None,
            basic_pass: None,
            api_key: Some(ApiKeyEnv {
                names: EnvList::from_strs(&["HF_TOKEN", "HUGGINGFACE_API_KEY"]),
                header: "Authorization".to_string(),
            }),
        ..Default::default()
        };

        assert!(mapping.api_key.is_some());
        assert!(mapping.bearer_token.is_none());
    }

    #[test]
    fn env_mapping_clone() {
        let mapping = EnvMapping {
            bearer_token: Some(EnvList::single("TOKEN")),
            basic_user: None,
            basic_pass: None,
            api_key: None,
        ..Default::default()
        };

        let cloned = mapping.clone();
        assert_eq!(mapping, cloned);
    }

    #[test]
    fn env_mapping_debug() {
        let mapping = EnvMapping::default();
        let debug = format!("{:?}", mapping);
        assert!(debug.contains("EnvMapping"));
    }

    // ========================================
    // HeaderError Tests
    // ========================================

    #[test]
    fn header_error_invalid_header_name() {
        let error = HeaderError::InvalidHeaderName("Inválid-Héader".to_string());
        let msg = error.to_string();

        assert!(msg.contains("Invalid header name"));
        assert!(msg.contains("Inválid-Héader"));
    }

    #[test]
    fn header_error_invalid_header_value() {
        let error = HeaderError::InvalidHeaderValue("X-Custom-Header".to_string());
        let msg = error.to_string();

        assert!(msg.contains("Invalid header value"));
        assert!(msg.contains("X-Custom-Header"));
    }

    #[test]
    fn header_error_missing_env() {
        let error = HeaderError::MissingEnv("API_KEY".to_string());
        let msg = error.to_string();

        assert!(msg.contains("Environment variable not set"));
        assert!(msg.contains("API_KEY"));
    }

    #[test]
    fn header_error_missing_credential() {
        let error = HeaderError::MissingCredential(vec![
            "OPENAI_API_KEY".to_string(),
            "OPENAI_KEY".to_string(),
        ]);
        let msg = error.to_string();

        assert!(msg.contains("Missing credential"));
        assert!(msg.contains("OPENAI_API_KEY"));
        assert!(msg.contains("OPENAI_KEY"));
    }

    #[test]
    fn header_error_clone() {
        let error = HeaderError::MissingEnv("KEY".to_string());
        let cloned = error.clone();
        assert_eq!(error, cloned);
    }

    #[test]
    fn header_error_debug() {
        let error = HeaderError::InvalidHeaderName("Test".to_string());
        let debug = format!("{:?}", error);
        assert!(debug.contains("InvalidHeaderName"));
        assert!(debug.contains("Test"));
    }

    #[test]
    fn header_error_eq() {
        let e1 = HeaderError::MissingEnv("KEY".to_string());
        let e2 = HeaderError::MissingEnv("KEY".to_string());
        let e3 = HeaderError::MissingEnv("OTHER".to_string());

        assert_eq!(e1, e2);
        assert_ne!(e1, e3);
    }

    // ========================================
    // Headers Builder Tests
    // ========================================

    #[test]
    fn headers_default_is_empty() {
        let headers = Headers::default();
        let result = headers.build().unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn headers_bearer_token_formatting() {
        let headers = Headers::default().use_bearer_token("my-secret-token");
        let result = headers.build().unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "Authorization");
        assert_eq!(result[0].1, "Bearer my-secret-token");
    }

    #[test]
    fn headers_basic_auth_encoding() {
        let headers = Headers::default().use_basic_auth("username", "password");
        let result = headers.build().unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "Authorization");

        // "username:password" in Base64 is "dXNlcm5hbWU6cGFzc3dvcmQ="
        assert_eq!(result[0].1, "Basic dXNlcm5hbWU6cGFzc3dvcmQ=");
    }

    #[test]
    fn headers_basic_auth_special_chars() {
        let headers = Headers::default().use_basic_auth("user@example.com", "p@ssw0rd!");
        let result = headers.build().unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "Authorization");

        // "user@example.com:p@ssw0rd!" in Base64 is "dXNlckBleGFtcGxlLmNvbTpwQHNzdzByZCE="
        assert_eq!(result[0].1, "Basic dXNlckBleGFtcGxlLmNvbTpwQHNzdzByZCE=");
    }

    #[test]
    fn headers_api_key_custom_header() {
        let headers = Headers::default().use_api_key("my-api-key", "X-API-Key");
        let result = headers.build().unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "X-API-Key");
        assert_eq!(result[0].1, "my-api-key");
    }

    #[test]
    fn headers_builder_chaining() {
        let headers = Headers::default()
            .use_bearer_token("token123")
            .content_type("application/json")
            .accept("application/json")
            .user_agent("MyClient/1.0");

        let result = headers.build().unwrap();

        assert_eq!(result.len(), 4);

        // Check all headers are present (order may vary, so we check individually)
        assert!(result.contains(&("Authorization".to_string(), "Bearer token123".to_string())));
        assert!(result.contains(&("Content-Type".to_string(), "application/json".to_string())));
        assert!(result.contains(&("Accept".to_string(), "application/json".to_string())));
        assert!(result.contains(&("User-Agent".to_string(), "MyClient/1.0".to_string())));
    }

    #[test]
    fn headers_accept_json_convenience() {
        let headers = Headers::default().accept_json();
        let result = headers.build().unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "Accept");
        assert_eq!(result[0].1, "application/json");
    }

    #[test]
    fn headers_content_type_json_convenience() {
        let headers = Headers::default().content_type_json();
        let result = headers.build().unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "Content-Type");
        assert_eq!(result[0].1, "application/json");
    }

    #[test]
    fn headers_custom_header() {
        let headers = Headers::default()
            .header("X-Custom-1", "value1")
            .header("X-Custom-2", "value2");

        let result = headers.build().unwrap();

        assert_eq!(result.len(), 2);
        assert!(result.contains(&("X-Custom-1".to_string(), "value1".to_string())));
        assert!(result.contains(&("X-Custom-2".to_string(), "value2".to_string())));
    }

    #[test]
    fn headers_remove_header() {
        let headers = Headers::default()
            .header("X-Custom-1", "value1")
            .header("X-Custom-2", "value2")
            .remove("X-Custom-1");

        let result = headers.build().unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "X-Custom-2");
        assert_eq!(result[0].1, "value2");
    }

    #[test]
    fn headers_remove_standard_header() {
        let headers = Headers::default()
            .content_type("text/plain")
            .accept("text/html")
            .remove("Content-Type");

        let result = headers.build().unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "Accept");
        assert_eq!(result[0].1, "text/html");
    }

    #[test]
    fn headers_remove_nonexistent_does_nothing() {
        let headers = Headers::default()
            .header("X-Custom", "value")
            .remove("X-NonExistent");

        let result = headers.build().unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "X-Custom");
    }

    #[test]
    fn headers_invalid_header_name_non_ascii() {
        let headers = Headers::default().header("Inválid-Héader", "value");
        let result = headers.build();

        assert!(result.is_err());
        match result {
            Err(HeaderError::InvalidHeaderName(name)) => {
                assert_eq!(name, "Inválid-Héader");
            }
            _ => panic!("Expected InvalidHeaderName error"),
        }
    }

    #[test]
    fn headers_invalid_header_name_with_space() {
        let headers = Headers::default().header("Invalid Header", "value");
        let result = headers.build();

        assert!(result.is_err());
        match result {
            Err(HeaderError::InvalidHeaderName(name)) => {
                assert_eq!(name, "Invalid Header");
            }
            _ => panic!("Expected InvalidHeaderName error"),
        }
    }

    #[test]
    fn headers_valid_ascii_header_names() {
        let headers = Headers::default()
            .header("X-Custom-123", "value")
            .header("Accept-Language", "en-US")
            .header("cache-control", "no-cache");

        let result = headers.build();
        assert!(result.is_ok());
    }

    #[test]
    fn headers_multiple_auth_strategies_last_wins() {
        // If someone sets multiple auth strategies, the last one should win
        let headers = Headers::default()
            .use_bearer_token("token1")
            .use_basic_auth("user", "pass");

        let result = headers.build().unwrap();

        // Should only have one Authorization header (basic auth)
        let auth_headers: Vec<_> = result
            .iter()
            .filter(|(k, _)| k == "Authorization")
            .collect();

        assert_eq!(auth_headers.len(), 1);
        assert!(auth_headers[0].1.starts_with("Basic "));
    }

    #[test]
    fn headers_override_content_type() {
        let headers = Headers::default()
            .content_type("text/plain")
            .content_type("application/json");

        let result = headers.build().unwrap();

        let ct_headers: Vec<_> = result.iter().filter(|(k, _)| k == "Content-Type").collect();

        assert_eq!(ct_headers.len(), 1);
        assert_eq!(ct_headers[0].1, "application/json");
    }

    #[test]
    fn headers_has_authorization_false_by_default() {
        let headers = Headers::default();
        assert!(!headers.has_authorization());
    }

    #[test]
    fn headers_has_authorization_true_after_bearer_token() {
        let headers = Headers::default().use_bearer_token("my-token");
        assert!(headers.has_authorization());
    }

    #[test]
    fn headers_has_authorization_true_after_basic_auth() {
        let headers = Headers::default().use_basic_auth("user", "pass");
        assert!(headers.has_authorization());
    }

    #[test]
    fn headers_has_authorization_false_after_api_key() {
        // API key uses custom header, not Authorization
        let headers = Headers::default().use_api_key("key", "X-API-Key");
        assert!(!headers.has_authorization());
    }

    #[test]
    fn headers_has_authorization_false_after_remove() {
        let headers = Headers::default()
            .use_bearer_token("token")
            .remove("Authorization");
        assert!(!headers.has_authorization());
    }

    #[test]
    fn headers_from_string_conversions() {
        let headers = Headers::default()
            .use_bearer_token(String::from("token"))
            .content_type(String::from("text/plain"))
            .header(String::from("X-Custom"), String::from("value"));

        let result = headers.build().unwrap();
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn headers_empty_values_allowed() {
        let headers = Headers::default().header("X-Empty", "").content_type("");

        let result = headers.build();
        assert!(result.is_ok());
    }

    // ========================================
    // ENV Loading Tests
    // ========================================

    #[test]
    #[serial_test::serial]
    fn from_env_resolves_bearer_token_from_first_matching_var() {
        unsafe {
            std::env::set_var("OPENAI_API_KEY", "sk-first-token");
            std::env::set_var("OPENAI_KEY", "sk-second-token");
        }

        let mapping = EnvMapping {
            bearer_token: Some(EnvList::from_strs(&["OPENAI_API_KEY", "OPENAI_KEY"])),
            basic_user: None,
            basic_pass: None,
            api_key: None,
        ..Default::default()
        };

        let headers = Headers {
            env_mapping: mapping,
            ..Default::default()
        }
        .from_env();

        let result = headers.build().unwrap();

        // First env var wins
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "Authorization");
        assert_eq!(result[0].1, "Bearer sk-first-token");

        unsafe {
            std::env::remove_var("OPENAI_API_KEY");
            std::env::remove_var("OPENAI_KEY");
        }
    }

    #[test]
    #[serial_test::serial]
    fn from_env_skips_when_no_env_vars_set() {
        unsafe {
            std::env::remove_var("MISSING_KEY_1");
            std::env::remove_var("MISSING_KEY_2");
        }

        let mapping = EnvMapping {
            bearer_token: Some(EnvList::from_strs(&["MISSING_KEY_1", "MISSING_KEY_2"])),
            basic_user: None,
            basic_pass: None,
            api_key: None,
        ..Default::default()
        };

        let headers = Headers {
            env_mapping: mapping,
            ..Default::default()
        }
        .from_env();

        let result = headers.build().unwrap();

        // Should be empty, no panic
        assert_eq!(result.len(), 0);
    }

    #[test]
    #[serial_test::serial]
    fn from_env_respects_fallback_chain_order() {
        unsafe {
            std::env::remove_var("PRIMARY_KEY");
            std::env::set_var("SECONDARY_KEY", "fallback-token");
            std::env::set_var("TERTIARY_KEY", "third-token");
        }

        let mapping = EnvMapping {
            bearer_token: Some(EnvList::from_strs(&[
                "PRIMARY_KEY",
                "SECONDARY_KEY",
                "TERTIARY_KEY",
            ])),
            basic_user: None,
            basic_pass: None,
            api_key: None,
        ..Default::default()
        };

        let headers = Headers {
            env_mapping: mapping,
            ..Default::default()
        }
        .from_env();

        let result = headers.build().unwrap();

        // First non-empty value wins (SECONDARY_KEY)
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "Authorization");
        assert_eq!(result[0].1, "Bearer fallback-token");

        unsafe {
            std::env::remove_var("SECONDARY_KEY");
            std::env::remove_var("TERTIARY_KEY");
        }
    }

    #[test]
    #[serial_test::serial]
    fn from_env_does_not_overwrite_preset_authorization() {
        unsafe {
            std::env::set_var("OPENAI_API_KEY", "sk-from-env");
        }

        let mapping = EnvMapping {
            bearer_token: Some(EnvList::single("OPENAI_API_KEY")),
            basic_user: None,
            basic_pass: None,
            api_key: None,
        ..Default::default()
        };

        let headers = Headers {
            env_mapping: mapping,
            ..Default::default()
        }
        .use_bearer_token("sk-preset-token") // Pre-set value
        .from_env();

        let result = headers.build().unwrap();

        // Pre-set value should be preserved
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "Authorization");
        assert_eq!(result[0].1, "Bearer sk-preset-token");

        unsafe {
            std::env::remove_var("OPENAI_API_KEY");
        }
    }

    #[test]
    #[serial_test::serial]
    fn from_env_with_uses_custom_mapping() {
        unsafe {
            std::env::set_var("CUSTOM_TOKEN_VAR", "custom-token");
        }

        let custom_mapping = EnvMapping {
            bearer_token: Some(EnvList::single("CUSTOM_TOKEN_VAR")),
            basic_user: None,
            basic_pass: None,
            api_key: None,
        ..Default::default()
        };

        let headers = Headers::default().from_env_with(custom_mapping);

        let result = headers.build().unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "Authorization");
        assert_eq!(result[0].1, "Bearer custom-token");

        unsafe {
            std::env::remove_var("CUSTOM_TOKEN_VAR");
        }
    }

    #[test]
    #[serial_test::serial]
    fn try_from_env_returns_error_when_required_vars_missing() {
        unsafe {
            std::env::remove_var("REQUIRED_KEY_1");
            std::env::remove_var("REQUIRED_KEY_2");
        }

        let mapping = EnvMapping {
            bearer_token: Some(EnvList::from_strs(&["REQUIRED_KEY_1", "REQUIRED_KEY_2"])),
            basic_user: None,
            basic_pass: None,
            api_key: None,
        ..Default::default()
        };

        let headers = Headers {
            env_mapping: mapping,
            ..Default::default()
        };

        let result = headers.try_from_env();

        assert!(result.is_err());
        match result {
            Err(HeaderError::MissingCredential(vars)) => {
                assert_eq!(vars.len(), 2);
                assert!(vars.contains(&"REQUIRED_KEY_1".to_string()));
                assert!(vars.contains(&"REQUIRED_KEY_2".to_string()));
            }
            _ => panic!("Expected MissingCredential error"),
        }
    }

    #[test]
    #[serial_test::serial]
    fn try_from_env_succeeds_when_all_required_vars_present() {
        unsafe {
            std::env::set_var("API_TOKEN", "valid-token");
        }

        let mapping = EnvMapping {
            bearer_token: Some(EnvList::single("API_TOKEN")),
            basic_user: None,
            basic_pass: None,
            api_key: None,
        ..Default::default()
        };

        let headers = Headers {
            env_mapping: mapping,
            ..Default::default()
        };

        let result = headers.try_from_env();

        assert!(result.is_ok());
        let headers = result.unwrap();
        let built = headers.build().unwrap();

        assert_eq!(built.len(), 1);
        assert_eq!(built[0].0, "Authorization");
        assert_eq!(built[0].1, "Bearer valid-token");

        unsafe {
            std::env::remove_var("API_TOKEN");
        }
    }

    #[test]
    #[serial_test::serial]
    fn from_env_basic_auth_resolution() {
        unsafe {
            std::env::set_var("API_USERNAME", "myuser");
            std::env::set_var("API_PASSWORD", "mypass");
        }

        let mapping = EnvMapping {
            bearer_token: None,
            basic_user: Some(EnvList::single("API_USERNAME")),
            basic_pass: Some(EnvList::single("API_PASSWORD")),
            api_key: None,
        ..Default::default()
        };

        let headers = Headers {
            env_mapping: mapping,
            ..Default::default()
        }
        .from_env();

        let result = headers.build().unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "Authorization");
        assert!(result[0].1.starts_with("Basic "));

        unsafe {
            std::env::remove_var("API_USERNAME");
            std::env::remove_var("API_PASSWORD");
        }
    }

    #[test]
    #[serial_test::serial]
    fn from_env_api_key_resolution() {
        unsafe {
            std::env::set_var("HF_TOKEN", "hf-api-key-12345");
        }

        let mapping = EnvMapping {
            bearer_token: None,
            basic_user: None,
            basic_pass: None,
            api_key: Some(ApiKeyEnv {
                names: EnvList::from_strs(&["HF_TOKEN", "HUGGINGFACE_KEY"]),
                header: "X-API-Key".to_string(),
            }),
        ..Default::default()
        };

        let headers = Headers {
            env_mapping: mapping,
            ..Default::default()
        }
        .from_env();

        let result = headers.build().unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "X-API-Key");
        assert_eq!(result[0].1, "hf-api-key-12345");

        unsafe {
            std::env::remove_var("HF_TOKEN");
        }
    }
}
