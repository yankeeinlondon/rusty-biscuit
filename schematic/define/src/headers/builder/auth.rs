use super::super::sensitive::SensitiveString;
use super::Headers;

impl Headers {
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
}
