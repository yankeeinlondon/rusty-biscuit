use std::collections::HashMap;

use lazy_static::lazy_static;
use strum::EnumIter;

use crate::api::auth::ApiAuthMethod;

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, EnumIter, PartialOrd, Ord)]
pub enum Provider {
    Anthropic,
    Deepseek,
    Gemini,
    Groq,
    HuggingFace,
    Mistral,
    MoonshotAi,
    Ollama,
    OpenAi,
    OpenRouter,
    Xai,
    Zai,
    ZenMux,
}

impl Provider {
    /// Returns the configuration for this provider.
    ///
    /// ## Panics
    ///
    /// Panics if the provider is not found in the configuration map.
    /// This should never happen as all providers must have configuration.
    pub fn config(&self) -> &'static ProviderConfig {
        PROVIDER_CONFIG
            .get(self)
            .expect("All providers must have config")
    }

    /// Returns the base URL for this provider's API.
    pub fn base_url(&self) -> &'static str {
        self.config().base_url
    }

    /// Returns the models endpoint for this provider.
    ///
    /// Returns the custom endpoint if configured, otherwise "/v1/models".
    pub fn models_endpoint(&self) -> &'static str {
        self.config().models_endpoint.unwrap_or("/v1/models")
    }

    /// Returns whether this is a local provider (no API key required).
    pub fn is_local(&self) -> bool {
        self.config().is_local
    }
}

/// Configuration for a single LLM provider.
///
/// Consolidates all provider-specific settings in one struct for easier
/// maintenance and reduced hash lookups.
#[derive(Debug, Clone)]
pub struct ProviderConfig {
    /// Environment variables that may contain the API key (first match wins)
    pub env_vars: &'static [&'static str],
    /// Authentication method for API requests
    pub auth_method: ApiAuthMethod,
    /// Base URL for the provider's API
    pub base_url: &'static str,
    /// Custom models endpoint (None = use standard /v1/models)
    /// for the provider's OpenAI compatible API
    pub models_endpoint: Option<&'static str>,
    /// Whether this is a local provider (no API key required)
    pub is_local: bool,
}

lazy_static! {
    /// Unified configuration for all providers.
    ///
    /// Single source of truth for provider settings, replacing multiple
    /// separate lookup tables.
    pub static ref PROVIDER_CONFIG: HashMap<Provider, ProviderConfig> = {
        let mut m = HashMap::new();

        m.insert(Provider::Anthropic, ProviderConfig {
            env_vars: &["ANTHROPIC_API_KEY"],
            auth_method: ApiAuthMethod::ApiKey("x-api-key".to_string()),
            base_url: "https://api.anthropic.com",
            models_endpoint: None,
            is_local: false,
        });

        m.insert(Provider::Deepseek, ProviderConfig {
            env_vars: &["DEEPSEEK_API_KEY"],
            auth_method: ApiAuthMethod::BearerToken,
            base_url: "https://api.deepseek.com",
            models_endpoint: None,
            is_local: false,
        });

        m.insert(Provider::Gemini, ProviderConfig {
            env_vars: &["GEMINI_API_KEY", "GOOGLE_API_KEY"],
            auth_method: ApiAuthMethod::QueryParam("key".to_string()),
            base_url: "https://generativelanguage.googleapis.com",
            models_endpoint: Some("/v1beta/models"),
            is_local: false,
        });

        m.insert(Provider::MoonshotAi, ProviderConfig {
            env_vars: &["MOONSHOT_API_KEY", "MOONSHOT_AI_API_KEY"],
            auth_method: ApiAuthMethod::BearerToken,
            base_url: "https://api.moonshot.ai/v1",  // Also works: api.moonshot.cn/v1
            models_endpoint: Some("/models"),  // base_url already has /v1
            is_local: false,
        });

        m.insert(Provider::Ollama, ProviderConfig {
            env_vars: &[],
            auth_method: ApiAuthMethod::None,
            base_url: "http://localhost:11434",
            models_endpoint: None,
            is_local: true,
        });

        m.insert(Provider::OpenAi, ProviderConfig {
            env_vars: &["OPENAI_API_KEY"],
            auth_method: ApiAuthMethod::BearerToken,
            base_url: "https://api.openai.com",
            models_endpoint: None,
            is_local: false,
        });

        m.insert(Provider::OpenRouter, ProviderConfig {
            env_vars: &["OPEN_ROUTER_API_KEY", "OPENROUTER_API_KEY"],
            auth_method: ApiAuthMethod::BearerToken,
            base_url: "https://openrouter.ai/api",
            models_endpoint: None,
            is_local: false,
        });

        m.insert(Provider::Xai, ProviderConfig {
            env_vars: &["XAI_API_KEY", "X_AI_API_KEY"],
            auth_method: ApiAuthMethod::BearerToken,
            base_url: "https://api.x.ai/v1",
            models_endpoint: Some("/models"),  // base_url already has /v1
            is_local: false,
        });


        m.insert(Provider::Zai, ProviderConfig {
            env_vars: &["ZAI_API_KEY", "Z_AI_API_KEY"],
            auth_method: ApiAuthMethod::BearerToken,
            base_url: "https://open.bigmodel.cn/api/paas/v4",  // ZhipuAI's actual domain
            models_endpoint: Some("/models"),  // base_url already has /v4
            is_local: false,
        });

        m.insert(Provider::ZenMux, ProviderConfig {
            env_vars: &["ZENMUX_API_KEY", "ZEN_MUX_API_KEY"],
            auth_method: ApiAuthMethod::None,
            base_url: "https://zenmux.ai/api",
            models_endpoint: None,
            is_local: false,
        });

        m.insert(Provider::Groq, ProviderConfig {
            env_vars: &["GROQ_API_KEY"],
            auth_method: ApiAuthMethod::BearerToken,
            base_url: "https://api.groq.com/openai",
            models_endpoint: None,
            is_local: false,
        });

        m.insert(Provider::Mistral, ProviderConfig {
            env_vars: &["MISTRAL_API_KEY"],
            auth_method: ApiAuthMethod::BearerToken,
            base_url: "https://api.mistral.ai",
            models_endpoint: None,
            is_local: false,
        });

        m.insert(Provider::HuggingFace, ProviderConfig {
            env_vars: &["HF_TOKEN", "HUGGINGFACE_TOKEN", "HUGGING_FACE_TOKEN"],
            auth_method: ApiAuthMethod::BearerToken,
            base_url: "https://huggingface.co/api",
            models_endpoint: Some("/models"),
            is_local: false,
        });

        m
    };
}
