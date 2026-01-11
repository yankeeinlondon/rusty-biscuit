//! Z.ai API client and Rig integration
//!
//! Z.ai provides GLM models via an OpenAI-compatible API.
//! Uses the Chat Completions API (not the newer Responses API).
//!
//! ## Examples
//!
//! ```no_run
//! use shared::providers::zai::{Client, GLM_4_7};
//!
//! // Create client from environment
//! let client = Client::from_env().expect("ZAI_API_KEY not set");
//!
//! // Get a completion model
//! let model = client.completion_model(GLM_4_7);
//! ```
//!
//! See: <https://docs.z.ai/guides/llm/glm-4.7>

use rig::client::CompletionClient;
use rig::providers::openai;
use rig::providers::openai::completion::CompletionModel;
use std::env;
use thiserror::Error;

/// Z.ai API base URL
pub const ZAI_API_BASE_URL: &str = "https://api.z.ai/api/paas/v4";

/// Z.ai China API base URL (alternative)
pub const ZAI_CN_API_BASE_URL: &str = "https://open.bigmodel.cn/api/paas/v4";

/// Environment variable for Z.ai API key
pub const ZAI_API_KEY_ENV: &str = "ZAI_API_KEY";

// Model constants
pub const GLM_4_5: &str = "glm-4.5";
pub const GLM_4_6: &str = "glm-4.6";
pub const GLM_4_7: &str = "glm-4.7";

/// Errors that can occur when creating a Z.ai client
#[derive(Debug, Error)]
pub enum ZaiError {
    /// Environment variable not set or empty
    #[error("{0} environment variable not set")]
    EnvVarNotSet(String),

    /// Failed to build the underlying HTTP client
    #[error("Failed to build Z.ai client: {0}")]
    ClientBuildFailed(String),
}

/// Z.ai client wrapping the OpenAI-compatible Chat Completions API
pub struct Client {
    inner: openai::CompletionsClient,
}

impl Client {
    /// Create a new Z.ai client with the given API key
    ///
    /// ## Errors
    ///
    /// Returns `ZaiError::ClientBuildFailed` if the underlying HTTP client
    /// cannot be constructed (e.g., TLS initialization failure).
    pub fn new(api_key: &str) -> Result<Self, ZaiError> {
        let inner = openai::CompletionsClient::builder()
            .api_key(api_key)
            .base_url(ZAI_API_BASE_URL)
            .build()
            .map_err(|e| ZaiError::ClientBuildFailed(e.to_string()))?;
        Ok(Self { inner })
    }

    /// Create a new Z.ai client from the ZAI_API_KEY environment variable
    ///
    /// ## Errors
    ///
    /// Returns `ZaiError::EnvVarNotSet` if the environment variable is not set
    /// or is empty. Returns `ZaiError::ClientBuildFailed` if the HTTP client
    /// cannot be built.
    pub fn from_env() -> Result<Self, ZaiError> {
        let api_key = env::var(ZAI_API_KEY_ENV)
            .map_err(|_| ZaiError::EnvVarNotSet(ZAI_API_KEY_ENV.to_string()))?;
        if api_key.trim().is_empty() {
            return Err(ZaiError::EnvVarNotSet(ZAI_API_KEY_ENV.to_string()));
        }
        Self::new(&api_key)
    }

    /// Create a builder for more advanced configuration
    pub fn builder(api_key: &str) -> ClientBuilder {
        ClientBuilder::new(api_key)
    }

    /// Get a completion model by name
    pub fn completion_model(&self, model: &str) -> CompletionModel {
        self.inner.completion_model(model)
    }

    /// Get an agent builder for the given model
    pub fn agent(&self, model: &str) -> rig::agent::AgentBuilder<CompletionModel> {
        self.inner.agent(model)
    }
}

/// Builder for Z.ai client configuration
pub struct ClientBuilder {
    api_key: String,
    base_url: String,
}

impl ClientBuilder {
    /// Create a new builder with the given API key
    pub fn new(api_key: &str) -> Self {
        Self {
            api_key: api_key.to_string(),
            base_url: ZAI_API_BASE_URL.to_string(),
        }
    }

    /// Set a custom base URL (e.g., for China endpoint)
    pub fn base_url(mut self, base_url: &str) -> Self {
        self.base_url = base_url.to_string();
        self
    }

    /// Use the China API endpoint
    pub fn china_endpoint(mut self) -> Self {
        self.base_url = ZAI_CN_API_BASE_URL.to_string();
        self
    }

    /// Build the Z.ai client
    ///
    /// ## Errors
    ///
    /// Returns `ZaiError::ClientBuildFailed` if the underlying HTTP client
    /// cannot be constructed.
    pub fn build(self) -> Result<Client, ZaiError> {
        let inner = openai::CompletionsClient::builder()
            .api_key(&self.api_key)
            .base_url(&self.base_url)
            .build()
            .map_err(|e| ZaiError::ClientBuildFailed(e.to_string()))?;

        Ok(Client { inner })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constants() {
        assert_eq!(ZAI_API_BASE_URL, "https://api.z.ai/api/paas/v4");
        assert_eq!(ZAI_CN_API_BASE_URL, "https://open.bigmodel.cn/api/paas/v4");
        assert_eq!(ZAI_API_KEY_ENV, "ZAI_API_KEY");
        assert_eq!(GLM_4_5, "glm-4.5");
        assert_eq!(GLM_4_6, "glm-4.6");
        assert_eq!(GLM_4_7, "glm-4.7");
    }

    #[test]
    #[serial_test::serial]
    fn test_from_env_missing_key() {
        unsafe { std::env::remove_var("ZAI_API_KEY") };
        let result = Client::from_env();
        assert!(result.is_err());
        match result {
            Err(ZaiError::EnvVarNotSet(var)) => {
                assert_eq!(var, "ZAI_API_KEY");
            }
            _ => panic!("Expected EnvVarNotSet error"),
        }
    }

    #[test]
    #[serial_test::serial]
    fn test_from_env_empty_key() {
        unsafe { std::env::set_var("ZAI_API_KEY", "") };
        let result = Client::from_env();
        assert!(result.is_err());
        match result {
            Err(ZaiError::EnvVarNotSet(var)) => {
                assert_eq!(var, "ZAI_API_KEY");
            }
            _ => panic!("Expected EnvVarNotSet error"),
        }
        unsafe { std::env::remove_var("ZAI_API_KEY") };
    }

    #[test]
    #[serial_test::serial]
    fn test_from_env_whitespace_key() {
        unsafe { std::env::set_var("ZAI_API_KEY", "   ") };
        let result = Client::from_env();
        assert!(result.is_err());
        match result {
            Err(ZaiError::EnvVarNotSet(var)) => {
                assert_eq!(var, "ZAI_API_KEY");
            }
            _ => panic!("Expected EnvVarNotSet error"),
        }
        unsafe { std::env::remove_var("ZAI_API_KEY") };
    }

    #[test]
    fn test_builder_base_url() {
        let builder = ClientBuilder::new("test-key")
            .base_url("https://custom.api.com");
        assert_eq!(builder.base_url, "https://custom.api.com");
    }

    #[test]
    fn test_builder_china_endpoint() {
        let builder = ClientBuilder::new("test-key")
            .china_endpoint();
        assert_eq!(builder.base_url, ZAI_CN_API_BASE_URL);
    }

    #[test]
    fn test_zai_error_display() {
        let err = ZaiError::EnvVarNotSet("ZAI_API_KEY".to_string());
        assert_eq!(err.to_string(), "ZAI_API_KEY environment variable not set");

        let err = ZaiError::ClientBuildFailed("TLS error".to_string());
        assert_eq!(err.to_string(), "Failed to build Z.ai client: TLS error");
    }
}
