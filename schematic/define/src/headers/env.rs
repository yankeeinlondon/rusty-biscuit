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
    pub names: EnvList,
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
    pub bearer_token: Option<EnvList>,
    pub basic_user: Option<EnvList>,
    pub basic_pass: Option<EnvList>,
    pub api_key: Option<ApiKeyEnv>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub oauth_client_id: Option<EnvList>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub oauth_client_secret: Option<EnvList>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub oauth_redirect_uri: Option<EnvList>,
}

pub(super) fn resolve_env_list(env_list: &EnvList) -> Option<String> {
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
}
