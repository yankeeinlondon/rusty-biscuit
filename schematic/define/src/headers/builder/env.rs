use super::super::env::{EnvMapping, resolve_env_list};
use super::super::error::HeaderError;
use super::Headers;

impl Headers {
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
        from_env_internal(self, &mapping, false).unwrap_or(fallback)
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
        from_env_internal(self, &mapping, false).unwrap_or(fallback)
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
        from_env_internal(self, &mapping, true)
    }
}

#[allow(clippy::wrong_self_convention)]
fn from_env_internal(
    mut headers: Headers,
    mapping: &EnvMapping,
    strict: bool,
) -> Result<Headers, HeaderError> {
    if headers.has_explicit_auth() {
        return Ok(headers);
    }

    if headers.authorization.is_none() {
        if let Some(ref env_list) = mapping.bearer_token {
            if let Some(token) = resolve_env_list(env_list) {
                headers = headers.use_bearer_token(token);
            } else if strict {
                return Err(HeaderError::MissingCredential(env_list.names().to_vec()));
            }
        }

        if headers.authorization.is_none()
            && let (Some(user_list), Some(pass_list)) =
                (&mapping.basic_user, &mapping.basic_pass)
        {
            let user = resolve_env_list(user_list);
            let pass = resolve_env_list(pass_list);

            match (user, pass) {
                (Some(u), Some(p)) => {
                    headers = headers.use_basic_auth(u, p);
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
        let header_exists = headers
            .custom
            .iter()
            .any(|(name, _)| name == &api_key_env.header);

        if !header_exists {
            if let Some(key) = resolve_env_list(&api_key_env.names) {
                headers = headers.header(&api_key_env.header, key);
            } else if strict {
                return Err(HeaderError::MissingCredential(
                    api_key_env.names.names().to_vec(),
                ));
            }
        }
    }

    Ok(headers)
}
