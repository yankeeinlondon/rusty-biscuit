//! ZenMux API client and Rig integration
//!
//! ZenMux is an AI gateway that provides unified access to multiple LLM providers
//! via OpenAI-compatible APIs. It supports models from OpenAI, Anthropic, and
//! Google Vertex AI using provider/model-name format (e.g., "openai/gpt-5").
//!
//! ## Authentication
//!
//! ZenMux uses standard Bearer token authentication via the `Authorization` header.
//! Set the `ZENMUX_API_KEY` or `ZEN_MUX_API_KEY` environment variable.
//!
//! ## API Compatibility
//!
//! ZenMux implements the OpenAI Chat Completions API. Note that the `/v1/models`
//! endpoint for automatic model discovery is not currently supported.
//!
//! ## Examples
//!
//! ```no_run
//! use shared::providers::zenmux::Client;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create client from environment variable
//!     let client = Client::from_env()?;
//!
//!     // Create an agent for a specific model
//!     let agent = client.agent("openai/gpt-4o")
//!         .preamble("You are a helpful assistant.")
//!         .build();
//!
//!     Ok(())
//! }
//! ```

use rig::client::CompletionClient;
use rig::providers::openai;
use rig::providers::openai::completion::CompletionModel;
use std::env;

use crate::providers::discovery::ProviderError;

/// ZenMux API base URL
pub const ZENMUX_API_BASE_URL: &str = "https://zenmux.ai/api/v1";

/// Primary environment variable for ZenMux API key
pub const ZENMUX_API_KEY_ENV: &str = "ZENMUX_API_KEY";

/// Alternative environment variable for ZenMux API key
pub const ZENMUX_API_KEY_ALT_ENV: &str = "ZEN_MUX_API_KEY";

/// ZenMux client wrapping the OpenAI-compatible Chat Completions API
pub struct Client {
    inner: openai::CompletionsClient,
}

impl Client {
    /// Create a new ZenMux client with the given API key
    ///
    /// ## Errors
    ///
    /// Returns `ProviderError::ClientBuildFailed` if the underlying OpenAI client
    /// cannot be constructed.
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

    /// Create a new ZenMux client from environment variables
    ///
    /// Checks `ZENMUX_API_KEY` first, then `ZEN_MUX_API_KEY` as a fallback.
    ///
    /// ## Errors
    ///
    /// Returns `ProviderError::MissingApiKey` if neither environment variable is set.
    /// Returns `ProviderError::ClientBuildFailed` if the client cannot be constructed.
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
        Self::new(&api_key)
    }

    /// Create a builder for more advanced configuration
    pub fn builder(api_key: String) -> ClientBuilder {
        ClientBuilder::new(api_key)
    }

    /// Get a completion model by name
    ///
    /// Use provider/model-name format for ZenMux (e.g., "openai/gpt-4o",
    /// "anthropic/claude-3-5-sonnet", "google/gemini-pro").
    pub fn completion_model(&self, model: &str) -> CompletionModel {
        self.inner.completion_model(model)
    }

    /// Get an agent builder for the given model
    ///
    /// Use provider/model-name format for ZenMux (e.g., "openai/gpt-4o",
    /// "anthropic/claude-3-5-sonnet", "google/gemini-pro").
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
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            base_url: ZENMUX_API_BASE_URL.to_string(),
        }
    }

    /// Set a custom base URL
    ///
    /// Use this for self-hosted ZenMux instances or alternative endpoints.
    pub fn base_url(mut self, base_url: String) -> Self {
        self.base_url = base_url;
        self
    }

    /// Build the ZenMux client
    ///
    /// ## Errors
    ///
    /// Returns `ProviderError::ClientBuildFailed` if the underlying OpenAI client
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
    #[serial_test::serial]
    fn test_from_env_missing_key() {
        // Ensure both env vars are unset
        unsafe {
            env::remove_var(ZENMUX_API_KEY_ENV);
            env::remove_var(ZENMUX_API_KEY_ALT_ENV);
        }

        let result = Client::from_env();
        assert!(result.is_err());

        match result {
            Err(ProviderError::MissingApiKey { provider, env_vars }) => {
                assert_eq!(provider, "ZenMux");
                assert!(env_vars.contains(&ZENMUX_API_KEY_ENV.to_string()));
                assert!(env_vars.contains(&ZENMUX_API_KEY_ALT_ENV.to_string()));
            }
            _ => panic!("Expected MissingApiKey error"),
        }
    }

    #[test]
    #[serial_test::serial]
    fn test_from_env_primary_key() {
        unsafe {
            env::remove_var(ZENMUX_API_KEY_ALT_ENV);
            env::set_var(ZENMUX_API_KEY_ENV, "test-primary-key");
        }

        let result = Client::from_env();
        assert!(result.is_ok());

        // Cleanup
        unsafe {
            env::remove_var(ZENMUX_API_KEY_ENV);
        }
    }

    #[test]
    #[serial_test::serial]
    fn test_from_env_fallback_key() {
        unsafe {
            env::remove_var(ZENMUX_API_KEY_ENV);
            env::set_var(ZENMUX_API_KEY_ALT_ENV, "test-fallback-key");
        }

        let result = Client::from_env();
        assert!(result.is_ok());

        // Cleanup
        unsafe {
            env::remove_var(ZENMUX_API_KEY_ALT_ENV);
        }
    }

    #[test]
    fn test_builder_default_base_url() {
        let builder = ClientBuilder::new("test-key".to_string());
        assert_eq!(builder.base_url, ZENMUX_API_BASE_URL);
    }

    #[test]
    fn test_builder_custom_base_url() {
        let builder = ClientBuilder::new("test-key".to_string())
            .base_url("https://custom.endpoint/v1".to_string());
        assert_eq!(builder.base_url, "https://custom.endpoint/v1");
    }

    #[test]
    fn test_new_with_valid_key() {
        let result = Client::new("test-api-key");
        assert!(result.is_ok());
    }
}
