//! ZenMux API client and Rig integration
//!
//! ZenMux is an AI gateway service providing access to multiple LLM providers
//! through a unified OpenAI-compatible API.
//!
//! ## Examples
//!
//! ```no_run
//! use ai_pipeline::rigging::providers::client_adaptors::zenmux::Client;
//!
//! // Create client from environment
//! let client = Client::from_env().expect("ZENMUX_API_KEY not set");
//!
//! // Get a completion model (use provider-prefixed model names)
//! let model = client.completion_model("openai/gpt-4o");
//! ```
//!
//! See: <https://zenmux.ai/docs>

use rig::client::CompletionClient;
use rig::providers::openai;
use rig::providers::openai::completion::CompletionModel;
use std::env;

use crate::rigging::providers::provider_errors::ProviderError;

/// ZenMux API base URL
pub const ZENMUX_API_BASE_URL: &str = "https://api.zenmux.ai/v1";

/// Environment variable for ZenMux API key
pub const ZENMUX_API_KEY_ENV: &str = "ZENMUX_API_KEY";

/// Alternative environment variable for ZenMux API key
pub const ZENMUX_API_KEY_ALT_ENV: &str = "ZEN_MUX_API_KEY";

/// ZenMux client wrapping the OpenAI-compatible API
pub struct Client {
    inner: openai::CompletionsClient,
}

impl Client {
    /// Create a new ZenMux client with the given API key
    ///
    /// ## Errors
    ///
    /// Returns `ProviderError::ClientBuildFailed` if the underlying HTTP client
    /// cannot be constructed (e.g., TLS initialization failure).
    pub fn new(api_key: &str) -> Result<Self, ProviderError> {
        let inner = openai::CompletionsClient::builder()
            .api_key(api_key)
            .base_url(ZENMUX_API_BASE_URL)
            .build()
            .map_err(|e| ProviderError::ClientBuildFailed {
                provider: "ZenMux".to_string(),
                reason: e.to_string(),
            })?;
        Ok(Self { inner })
    }

    /// Create a new ZenMux client from the ZENMUX_API_KEY environment variable
    ///
    /// Checks `ZENMUX_API_KEY` first, then `ZEN_MUX_API_KEY` as a fallback.
    ///
    /// ## Errors
    ///
    /// Returns `ProviderError::MissingApiKey` if neither environment variable is set
    /// or is empty. Returns `ProviderError::ClientBuildFailed` if the HTTP client
    /// cannot be built.
    pub fn from_env() -> Result<Self, ProviderError> {
        let api_key = env::var(ZENMUX_API_KEY_ENV)
            .or_else(|_| env::var(ZENMUX_API_KEY_ALT_ENV))
            .map_err(|_| ProviderError::MissingApiKey {
                provider: "ZenMux".to_string(),
                env_vars: vec![
                    ZENMUX_API_KEY_ENV.to_string(),
                    ZENMUX_API_KEY_ALT_ENV.to_string(),
                ],
            })?;

        if api_key.trim().is_empty() {
            return Err(ProviderError::MissingApiKey {
                provider: "ZenMux".to_string(),
                env_vars: vec![
                    ZENMUX_API_KEY_ENV.to_string(),
                    ZENMUX_API_KEY_ALT_ENV.to_string(),
                ],
            });
        }
        Self::new(&api_key)
    }

    /// Create a builder for more advanced configuration
    pub fn builder(api_key: &str) -> ClientBuilder {
        ClientBuilder::new(api_key)
    }

    /// Get a completion model by name
    ///
    /// ZenMux uses provider-prefixed model names like:
    /// - `openai/gpt-4o`
    /// - `anthropic/claude-3.5-sonnet`
    /// - `google/gemini-pro`
    pub fn completion_model(&self, model: &str) -> CompletionModel {
        self.inner.completion_model(model)
    }

    /// Get an agent builder for the given model
    pub fn agent(&self, model: &str) -> rig::agent::AgentBuilder<CompletionModel> {
        self.inner.agent(model)
    }
}

/// Builder for ZenMux client configuration
pub struct ClientBuilder {
    api_key: String,
    base_url: String,
}

impl ClientBuilder {
    /// Create a new builder with the given API key
    pub fn new(api_key: &str) -> Self {
        Self {
            api_key: api_key.to_string(),
            base_url: ZENMUX_API_BASE_URL.to_string(),
        }
    }

    /// Set a custom base URL
    pub fn base_url(mut self, base_url: &str) -> Self {
        self.base_url = base_url.to_string();
        self
    }

    /// Build the ZenMux client
    ///
    /// ## Errors
    ///
    /// Returns `ProviderError::ClientBuildFailed` if the underlying HTTP client
    /// cannot be constructed.
    pub fn build(self) -> Result<Client, ProviderError> {
        let inner = openai::CompletionsClient::builder()
            .api_key(&self.api_key)
            .base_url(&self.base_url)
            .build()
            .map_err(|e| ProviderError::ClientBuildFailed {
                provider: "ZenMux".to_string(),
                reason: e.to_string(),
            })?;

        Ok(Client { inner })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constants() {
        assert_eq!(ZENMUX_API_BASE_URL, "https://api.zenmux.ai/v1");
        assert_eq!(ZENMUX_API_KEY_ENV, "ZENMUX_API_KEY");
        assert_eq!(ZENMUX_API_KEY_ALT_ENV, "ZEN_MUX_API_KEY");
    }

    #[test]
    #[serial_test::serial]
    fn test_from_env_missing_key() {
        unsafe {
            std::env::remove_var("ZENMUX_API_KEY");
            std::env::remove_var("ZEN_MUX_API_KEY");
        }
        let result = Client::from_env();
        assert!(result.is_err());
        match result {
            Err(ProviderError::MissingApiKey { provider, env_vars }) => {
                assert_eq!(provider, "ZenMux");
                assert!(env_vars.contains(&"ZENMUX_API_KEY".to_string()));
                assert!(env_vars.contains(&"ZEN_MUX_API_KEY".to_string()));
            }
            _ => panic!("Expected MissingApiKey error"),
        }
    }

    #[test]
    #[serial_test::serial]
    fn test_from_env_empty_key() {
        unsafe {
            std::env::set_var("ZENMUX_API_KEY", "");
            std::env::remove_var("ZEN_MUX_API_KEY");
        }
        let result = Client::from_env();
        assert!(result.is_err());
        match result {
            Err(ProviderError::MissingApiKey { provider, .. }) => {
                assert_eq!(provider, "ZenMux");
            }
            _ => panic!("Expected MissingApiKey error"),
        }
        unsafe { std::env::remove_var("ZENMUX_API_KEY") };
    }

    #[test]
    #[serial_test::serial]
    fn test_from_env_whitespace_key() {
        unsafe {
            std::env::set_var("ZENMUX_API_KEY", "   ");
            std::env::remove_var("ZEN_MUX_API_KEY");
        }
        let result = Client::from_env();
        assert!(result.is_err());
        match result {
            Err(ProviderError::MissingApiKey { provider, .. }) => {
                assert_eq!(provider, "ZenMux");
            }
            _ => panic!("Expected MissingApiKey error"),
        }
        unsafe { std::env::remove_var("ZENMUX_API_KEY") };
    }

    #[test]
    #[serial_test::serial]
    fn test_from_env_fallback_key() {
        unsafe {
            std::env::remove_var("ZENMUX_API_KEY");
            std::env::set_var("ZEN_MUX_API_KEY", "test-fallback-key");
        }

        let result = Client::from_env();
        assert!(result.is_ok());

        // Cleanup
        unsafe {
            std::env::remove_var("ZEN_MUX_API_KEY");
        }
    }

    #[test]
    #[serial_test::serial]
    fn test_from_env_success() {
        unsafe {
            std::env::set_var("ZENMUX_API_KEY", "test-api-key");
        }

        let result = Client::from_env();
        assert!(result.is_ok());

        // Cleanup
        unsafe {
            std::env::remove_var("ZENMUX_API_KEY");
        }
    }

    #[test]
    fn test_builder_base_url() {
        let builder = ClientBuilder::new("test-key").base_url("https://custom.api.com");
        assert_eq!(builder.base_url, "https://custom.api.com");
    }

    #[test]
    fn test_provider_error_display() {
        let err = ProviderError::MissingApiKey {
            provider: "ZenMux".to_string(),
            env_vars: vec!["ZENMUX_API_KEY".to_string()],
        };
        let display = err.to_string();
        assert!(display.contains("ZenMux"));
        assert!(display.contains("ZENMUX_API_KEY"));

        let err = ProviderError::ClientBuildFailed {
            provider: "ZenMux".to_string(),
            reason: "Connection error".to_string(),
        };
        let display = err.to_string();
        assert!(display.contains("ZenMux"));
        assert!(display.contains("Connection error"));
    }

    #[test]
    fn test_new_client() {
        let result = Client::new("test-api-key");
        assert!(result.is_ok());
    }

    #[test]
    fn test_builder_build() {
        let result = ClientBuilder::new("test-key")
            .base_url("https://api.example.com")
            .build();
        assert!(result.is_ok());
    }
}
