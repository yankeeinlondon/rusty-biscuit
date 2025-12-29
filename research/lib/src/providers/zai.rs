//! Z.ai API client and Rig integration
//!
//! Z.ai provides GLM models via an OpenAI-compatible API.
//! Uses the Chat Completions API (not the newer Responses API).
//! See: https://docs.z.ai/guides/llm/glm-4.7

use rig::client::CompletionClient;
use rig::providers::openai;
use rig::providers::openai::completion::CompletionModel;
use std::env;

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

/// Z.ai client wrapping the OpenAI-compatible Chat Completions API
pub struct Client {
    inner: openai::CompletionsClient,
}

impl Client {
    /// Create a new Z.ai client with the given API key
    pub fn new(api_key: &str) -> Self {
        let inner = openai::CompletionsClient::builder()
            .api_key(api_key)
            .base_url(ZAI_API_BASE_URL)
            .build()
            .expect("Failed to build Z.ai client");
        Self { inner }
    }

    /// Create a new Z.ai client from the ZAI_API_KEY environment variable
    pub fn from_env() -> Result<Self, String> {
        let api_key = env::var(ZAI_API_KEY_ENV)
            .map_err(|_| format!("{} environment variable not set", ZAI_API_KEY_ENV))?;
        Ok(Self::new(&api_key))
    }

    /// Create a new Z.ai client from the ZAI_API_KEY environment variable, panicking on error
    pub fn from_env_or_panic() -> Self {
        Self::from_env().expect("Failed to create Z.ai client from environment")
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
    pub fn build(self) -> Client {
        let inner = openai::CompletionsClient::builder()
            .api_key(&self.api_key)
            .base_url(&self.base_url)
            .build()
            .expect("Failed to build Z.ai client");

        Client { inner }
    }
}
