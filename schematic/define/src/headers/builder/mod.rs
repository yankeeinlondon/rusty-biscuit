mod auth;
mod env;
mod validate;

#[cfg(test)]
mod tests;

use super::env::EnvMapping;
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
    pub(crate) authorization: Option<SensitiveString>,
    pub(crate) content_type: Option<String>,
    pub(crate) accept: Option<String>,
    pub(crate) user_agent: Option<String>,
    pub(crate) custom: Vec<(String, String)>,
    pub(crate) explicit_auth_headers: Vec<String>,
    pub(crate) env_mapping: EnvMapping,
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
            validate::validate_header_name("Authorization")?;
            result.push(("Authorization".to_string(), auth.into_inner()));
        }

        if let Some(ct) = self.content_type {
            validate::validate_header_name("Content-Type")?;
            result.push(("Content-Type".to_string(), ct));
        }

        if let Some(accept) = self.accept {
            validate::validate_header_name("Accept")?;
            result.push(("Accept".to_string(), accept));
        }

        if let Some(ua) = self.user_agent {
            validate::validate_header_name("User-Agent")?;
            result.push(("User-Agent".to_string(), ua));
        }

        for (name, value) in self.custom {
            validate::validate_header_name(&name)?;
            result.push((name, value));
        }

        Ok(result)
    }
}
