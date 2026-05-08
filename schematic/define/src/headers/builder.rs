#[allow(unused_imports)]
use super::ApiKeyEnv;
#[allow(unused_imports)]
use super::env::{EnvList, EnvMapping, resolve_env_list};
use super::error::HeaderError;
use super::sensitive::SensitiveString;

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
    authorization: Option<SensitiveString>,
    content_type: Option<String>,
    accept: Option<String>,
    user_agent: Option<String>,
    custom: Vec<(String, String)>,
    explicit_auth_headers: Vec<String>,
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

    /// Returns the configured environment-variable mapping.
    #[must_use]
    pub fn env_mapping(&self) -> &EnvMapping {
        &self.env_mapping
    }

    /// Set Bearer token authentication.
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

    /// Set bearer-token authentication using a custom header name.
    pub fn use_bearer_token_with_header(
        mut self,
        token: impl Into<String>,
        header_name: impl Into<String>,
    ) -> Self {
        let header_name = header_name.into();
        if header_name.eq_ignore_ascii_case("Authorization") {
            return self.use_bearer_token(token);
        }

        self.explicit_auth_headers
            .retain(|existing| existing != &header_name);
        self.explicit_auth_headers.push(header_name.clone());
        self = self.header(header_name, format!("Bearer {}", token.into()));
        self
    }

    /// Set Basic authentication credentials.
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
    pub fn use_api_key(mut self, key: impl Into<String>, header_name: impl Into<String>) -> Self {
        let header_name = header_name.into();
        self.explicit_auth_headers
            .retain(|existing| existing != &header_name);
        self.explicit_auth_headers.push(header_name.clone());
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

        self.custom.retain(|(k, _)| k != &name);
        self.custom.push((name, value));
        self
    }

    /// Remove a header by name.
    ///
    /// ## Examples
    ///
    /// ```
    /// use schematic_define::Headers;
    ///
    /// let headers = Headers::default()
    ///     .header("X-Custom-1", "value1")
    ///     .header("X-Custom-2", "value2")
    ///     .remove("X-Custom-1")
    ///     .build()
    ///     .unwrap();
    ///
    /// assert_eq!(headers.len(), 1);
    /// assert_eq!(headers[0].0, "X-Custom-2");
    /// assert_eq!(headers[0].1, "value2");
    /// ```
    pub fn remove(mut self, name: &str) -> Self {
        self.custom.retain(|(k, _)| k != name);
        self.explicit_auth_headers.retain(|header| header != name);

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
        let fallback = self.clone();
        self.from_env_internal(&mapping, false).unwrap_or(fallback)
    }

    /// Load credentials from environment variables (strict).
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

    #[allow(clippy::wrong_self_convention)]
    fn from_env_internal(
        mut self,
        mapping: &EnvMapping,
        strict: bool,
    ) -> Result<Self, HeaderError> {
        if self.has_explicit_auth() {
            return Ok(self);
        }

        if self.authorization.is_none() {
            if let Some(ref env_list) = mapping.bearer_token {
                if let Some(token) = resolve_env_list(env_list) {
                    self = self.use_bearer_token(token);
                } else if strict {
                    return Err(HeaderError::MissingCredential(env_list.names().to_vec()));
                }
            }

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

        if let Some(ref api_key_env) = mapping.api_key {
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

    /// Returns `true` if any explicit auth has been set programmatically.
    #[must_use]
    pub fn has_explicit_auth(&self) -> bool {
        self.authorization.is_some()
            || self
                .explicit_auth_headers
                .iter()
                .any(|header| self.custom.iter().any(|(name, _)| name == header))
    }

    /// Returns `true` if a header with the given name is present.
    #[must_use]
    pub fn has_header(&self, name: &str) -> bool {
        if name.eq_ignore_ascii_case("Authorization") {
            return self.authorization.is_some()
                || self
                    .custom
                    .iter()
                    .any(|(header_name, _)| header_name.eq_ignore_ascii_case("Authorization"));
        }

        self.custom
            .iter()
            .any(|(header_name, _)| header_name.eq_ignore_ascii_case(name))
            || match name {
                "Content-Type" => self.content_type.is_some(),
                "Accept" => self.accept.is_some(),
                "User-Agent" => self.user_agent.is_some(),
                _ => false,
            }
    }

    /// Build the final header list.
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

        if let Some(auth) = self.authorization {
            validate_header_name("Authorization")?;
            result.push(("Authorization".to_string(), auth.into_inner()));
        }

        if let Some(ct) = self.content_type {
            validate_header_name("Content-Type")?;
            result.push(("Content-Type".to_string(), ct));
        }

        if let Some(accept) = self.accept {
            validate_header_name("Accept")?;
            result.push(("Accept".to_string(), accept));
        }

        if let Some(ua) = self.user_agent {
            validate_header_name("User-Agent")?;
            result.push(("User-Agent".to_string(), ua));
        }

        for (name, value) in self.custom {
            validate_header_name(&name)?;
            result.push((name, value));
        }

        Ok(result)
    }
}

fn validate_header_name(name: &str) -> Result<(), HeaderError> {
    if !name.is_ascii() {
        return Err(HeaderError::InvalidHeaderName(name.to_string()));
    }

    for ch in name.chars() {
        if !ch.is_ascii_alphanumeric() && ch != '-' && ch != '_' {
            return Err(HeaderError::InvalidHeaderName(name.to_string()));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(result[0].1, "Basic dXNlcm5hbWU6cGFzc3dvcmQ=");
    }

    #[test]
    fn headers_basic_auth_special_chars() {
        let headers = Headers::default().use_basic_auth("user@example.com", "p@ssw0rd!");
        let result = headers.build().unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "Authorization");
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
        let headers = Headers::default()
            .use_bearer_token("token1")
            .use_basic_auth("user", "pass");

        let result = headers.build().unwrap();

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
        let headers = Headers::default().use_api_key("key", "X-API-Key");
        assert!(!headers.has_authorization());
    }

    #[test]
    fn headers_has_explicit_auth_true_after_api_key() {
        let headers = Headers::default().use_api_key("key", "X-API-Key");
        assert!(headers.has_explicit_auth());
    }

    #[test]
    fn headers_has_explicit_auth_false_after_api_key_remove() {
        let headers = Headers::default()
            .use_api_key("key", "X-API-Key")
            .remove("X-API-Key");
        assert!(!headers.has_explicit_auth());
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
        .use_bearer_token("sk-preset-token")
        .from_env();

        let result = headers.build().unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "Authorization");
        assert_eq!(result[0].1, "Bearer sk-preset-token");

        unsafe {
            std::env::remove_var("OPENAI_API_KEY");
        }
    }

    #[test]
    #[serial_test::serial]
    fn from_env_does_not_apply_bearer_fallback_when_api_key_is_explicit() {
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
        .use_api_key("hf-explicit", "X-API-Key")
        .from_env();

        let result = headers.build().unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "X-API-Key");
        assert_eq!(result[0].1, "hf-explicit");

        unsafe {
            std::env::remove_var("OPENAI_API_KEY");
        }
    }

    #[test]
    #[serial_test::serial]
    fn from_env_does_not_overwrite_explicit_api_key() {
        unsafe {
            std::env::set_var("HF_TOKEN", "hf-from-env");
        }

        let mapping = EnvMapping {
            bearer_token: None,
            basic_user: None,
            basic_pass: None,
            api_key: Some(ApiKeyEnv {
                names: EnvList::single("HF_TOKEN"),
                header: "X-API-Key".to_string(),
            }),
            ..Default::default()
        };

        let headers = Headers {
            env_mapping: mapping,
            ..Default::default()
        }
        .use_api_key("hf-explicit", "X-API-Key")
        .from_env();

        let result = headers.build().unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "X-API-Key");
        assert_eq!(result[0].1, "hf-explicit");

        unsafe {
            std::env::remove_var("HF_TOKEN");
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
