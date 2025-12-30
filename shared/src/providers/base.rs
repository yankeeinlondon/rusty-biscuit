use std::collections::{HashMap, BTreeMap};
use lazy_static::lazy_static;
use serde_json::Value;
use url::Url;
use strum_macros::EnumIter;
use strum::IntoEnumIterator;


#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, EnumIter, PartialOrd, Ord)]
pub enum Provider {
    Anthropic,
    Deepseek,
    Gemini,
    MoonshotAi,
    Ollama,
    OpenAi,
    OpenRouter,
    Zai,
    ZenMux
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum ApiAuthMethod {
    BearerToken,
    ApiKey(String),
    None
}


lazy_static! {
    /// A lookup table which provides a list of ENV variables in which the given provider's
    /// API Key may be found.
    static ref PROVIDER_ENV_VARIABLES: HashMap<Provider, Vec<&'static str>> = {
        let mut m = HashMap::new();
        m.insert(Provider::Anthropic, vec!["ANTHROPIC_API_KEY"]);
        m.insert(Provider::Deepseek, vec!["DEEPSEEK_API_KEY"]);
        m.insert(Provider::Gemini, vec!["GEMINI_API_KEY", "GOOGLE_API_KEY"]);
        m.insert(Provider::MoonshotAi, vec!["MOONSHOT_API_KEY", "MOONSHOT_AI_API_KEY"]);
        m.insert(Provider::OpenAi, vec!["OPENAI_API_KEY"]);
        m.insert(Provider::OpenRouter, vec!["OPEN_ROUTER_API_KEY", "OPENROUTER_API_KEY"]);
        m.insert(Provider::Zai, vec!["ZAI_API_KEY", "Z_AI_API_KEY"]);
        m.insert(Provider::ZenMux, vec!["ZENMUX_API_KEY", "ZEN_MUX_API_KEY"]);

        m
    };
}

lazy_static! {
    /// A lookup table which provides the default authentication method for a given provider.
    static ref PROVIDER_AUTH: HashMap<Provider, ApiAuthMethod> = {
        let mut auth = HashMap::new();

        auth.insert(Provider::Anthropic, ApiAuthMethod::ApiKey("x-api-key".to_string()));
        auth.insert(Provider::Deepseek, ApiAuthMethod::BearerToken);
        auth.insert(Provider::Gemini, ApiAuthMethod::BearerToken);
        auth.insert(Provider::MoonshotAi, ApiAuthMethod::BearerToken);
        auth.insert(Provider::Ollama, ApiAuthMethod::None);
        auth.insert(Provider::OpenAi, ApiAuthMethod::BearerToken);
        auth.insert(Provider::OpenRouter, ApiAuthMethod::BearerToken);
        auth.insert(Provider::Zai, ApiAuthMethod::BearerToken);
        auth.insert(Provider::ZenMux, ApiAuthMethod::None);

        auth
    };
}

lazy_static! {
    /// A list of providers who are "local" (versus a cloud provider)
    static ref LOCAL_PROVIDERS: Vec<Provider> = {
        let local_providers = vec![
            Provider::Ollama
        ];

        local_providers
    };
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct ModelArchitecture {
    modality: String,
    input_modalities: Vec<String>,
    output_modalities: Vec<String>,
    tokenizer: String,
    instruct_type: Option<String> 
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct NumericString(String);

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct ModelPricing {
    prompt: NumericString,
    completion: NumericString,
    request: NumericString,
    image: NumericString,
    web_search: NumericString,
    internal_reasoning: NumericString
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct ModelTopProvider {
    context_length: u32,
    max_completion_tokens: u32,
    is_moderated: bool 
}

#[derive(Debug, PartialEq, Clone)]
pub struct ModelDefaultParameters {
    temperature: Option<f32>,
    top_p: Option<f32>,
    frequency_penalty: Option<Value>
}

#[derive(Debug, PartialEq, Clone)]
pub struct ModelPermission {
    created: u32,
    id: String,
    object: String,
    organization: String,
    group: String,
    is_blocking: bool
}


/// The shape of a provider's model when returned from the 
/// OpenAI API `/models` endpoint.
#[derive(Debug, PartialEq, Clone)]
pub struct ModelDefinition {
    id: String,

    /// only provided on OpenRouter
    canonical_slug: Option<String>,
    /// only provided on OpenRouter
    hugging_face_id: Option<String>,
    /// only provided on OpenRouter
    name: Option<String>,
    /// only provided on OpenRouter
    description: Option<String>,
    /// only provided on OpenRouter and Moonshot AI
    context_length: Option<u32>,
    /// only provided on OpenRouter
    architecture: Option<ModelArchitecture>,
    /// only provided on OpenRouter
    pricing: Option<ModelPricing>,
    /// only provided on OpenRouter
    top_provider: Option<ModelTopProvider>,
    /// only provided on OpenRouter
    supported_parameters: Option<Vec<String>>,
    /// only provided on OpenRouter
    default_parameters: Option<ModelDefaultParameters>,

    /// only provided on Moonshot AI
    root: Option<String>,
    /// only provided on Moonshot AI
    parent: Option<String>,
    /// only provided on Moonshot AI
    permission: Option<Vec<ModelPermission>>,

    object: String,
    created: u32,
    owned_by: String,
    display_name: Option<String>
}

/// Checks whether the passed in provider has an API Key derived from ENV variables
pub fn has_provider_api_key(provider: &Provider) -> bool {
    // Local providers don't require API keys
    if LOCAL_PROVIDERS.contains(provider) {
        return false;
    }

    // Look up the environment variable names for this provider
    let Some(env_vars) = PROVIDER_ENV_VARIABLES.get(provider) else {
        return false;
    };

    // Check if ANY of the environment variables is set and non-empty
    env_vars.iter().any(|var_name| {
        std::env::var(var_name)
            .ok()
            .filter(|v| !v.trim().is_empty())
            .is_some()
    })
}

/// Looks up all known ENV variables which might contain AI API Keys and returns a
/// BTreeMap of available API Keys in the current environment.
///
/// The BTreeMap is used instead of HashMap to provide deterministic ordering.
/// Priority is given to the first environment variable in the list for each provider.
pub fn get_api_keys() -> BTreeMap<Provider, String> {
    let mut api_keys = BTreeMap::new();

    // Iterate through all providers
    for provider in Provider::iter() {
        // Skip local providers
        if LOCAL_PROVIDERS.contains(&provider) {
            continue;
        }

        // Get the list of environment variables for this provider
        let Some(env_vars) = PROVIDER_ENV_VARIABLES.get(&provider) else {
            continue;
        };

        // Check each environment variable in priority order (first match wins)
        for var_name in env_vars {
            if let Some(api_key) = std::env::var(var_name)
                .ok()
                .filter(|v| !v.trim().is_empty())
            {
                api_keys.insert(provider, api_key);
                break; // First match wins
            }
        }
    }

    api_keys
}


/// Uses the OpenAI-compatible API's provided by the Providers
/// which we have API Keys for.
pub async fn get_provider_models() -> Vec<String> {
    // will provide a list of models provided with the PROVIDER prefixed before it.
    // for aggregators like OpenRouter and ZenMux, we have two levels of abstraction:
    // first the aggregator, then the _underlying_ provider, and then the model id/name
    
    // NOTE: the aggregator's do NOT include their own names in the abstraction when 
    // calling the `/models` endpoint on the OpenAI compatible API's

    // NOTE: if we have the list of models from an aggregator we can supplement the list
    // with not only the fully qualified `{aggregator}/{underlying provider}/{model}` but
    // we can infer the `{underlying provider}/{model}` if we do NOT have the API key for
    // that provider.

    todo!()
}

/// Provides the full URL to the evaluation of the given model on the Artificial Analysis
/// website.
pub fn artificial_analysis_url(model: &str) -> Url {
    // it would appear the base url of `https://artificialanalysis.ai/model/{model_name}` is
    // the correct url but to get the model_name we need to:
    //   - strip off any `-preview` at the end of the model
    //   - strip off any text after a `:` character is found
    //   - strip off any provider info at the front
    todo!()
}

// ============================================================================
// ZENMUX API COMPATIBILITY RESEARCH (Phase 0)
// ============================================================================
// Research Date: 2025-12-30
// Base URL: https://zenmux.ai/api/v1
// Authentication: Bearer token (OpenAI-compatible)
// /v1/models endpoint: NOT CONFIRMED in official documentation
//
// ZenMux uses provider/model-name format (e.g., "openai/gpt-5") and supports
// OpenAI, Anthropic, and Google Vertex AI protocols. However, the /v1/models
// endpoint for automatic model discovery is not documented.
//
// Related Issue: https://github.com/sst/opencode/issues/2901
// - OpenCode Zen (related project) does not expose /v1/models endpoint
// - This prevents automatic model discovery in OpenAI-compatible clients
//
// CONCLUSION: ZenMux likely does NOT support /v1/models endpoint.
// - Mark as UNSUPPORTED for model discovery in get_provider_models()
// - Can still be used with explicitly configured model names
// ============================================================================

#[cfg(test)]
mod test_helpers {
    use std::env;
    use std::collections::HashMap;

    /// RAII-based environment variable setter that automatically restores
    /// the original value (or removes the variable) when dropped.
    ///
    /// This ensures test isolation when modifying environment variables.
    ///
    /// # Example
    ///
    /// ```
    /// use test_helpers::ScopedEnv;
    ///
    /// {
    ///     let _env = ScopedEnv::new("OPENAI_API_KEY", "test-key");
    ///     assert_eq!(std::env::var("OPENAI_API_KEY").unwrap(), "test-key");
    /// } // _env dropped here, original value restored
    /// ```
    pub struct ScopedEnv {
        key: String,
        original: Option<String>,
    }

    impl ScopedEnv {
        /// Sets an environment variable and stores the original value for restoration.
        ///
        /// # Safety
        ///
        /// This function is safe for test use with `#[serial_test::serial]` attribute,
        /// which ensures tests run sequentially and don't race on environment variables.
        pub fn new(key: &str, value: &str) -> Self {
            let original = env::var(key).ok();
            unsafe {
                env::set_var(key, value);
            }
            Self {
                key: key.to_string(),
                original,
            }
        }

        /// Sets multiple environment variables at once.
        pub fn new_multi(vars: HashMap<&str, &str>) -> Vec<Self> {
            vars.into_iter()
                .map(|(k, v)| Self::new(k, v))
                .collect()
        }
    }

    impl Drop for ScopedEnv {
        fn drop(&mut self) {
            match &self.original {
                Some(val) => unsafe { env::set_var(&self.key, val) },
                None => unsafe { env::remove_var(&self.key) },
            }
        }
    }

    #[cfg(test)]
    mod scoped_env_tests {
        use super::*;

        #[test]
        #[serial_test::serial]
        fn test_scoped_env_restores_original() {
            // Set a baseline value
            unsafe { env::set_var("TEST_VAR", "original") };

            {
                let _scoped = ScopedEnv::new("TEST_VAR", "temporary");
                assert_eq!(env::var("TEST_VAR").unwrap(), "temporary");
            }

            // Should be restored
            assert_eq!(env::var("TEST_VAR").unwrap(), "original");

            // Cleanup
            unsafe { env::remove_var("TEST_VAR") };
        }

        #[test]
        #[serial_test::serial]
        fn test_scoped_env_removes_if_not_set() {
            // Ensure var is not set
            unsafe { env::remove_var("TEST_VAR_2") };

            {
                let _scoped = ScopedEnv::new("TEST_VAR_2", "temporary");
                assert_eq!(env::var("TEST_VAR_2").unwrap(), "temporary");
            }

            // Should be removed
            assert!(env::var("TEST_VAR_2").is_err());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::discovery::ProviderError;

    // ========================================================================
    // Phase 3: artificial_analysis_url() Tests
    // ========================================================================

    #[test]
    fn test_artificial_analysis_url_basic_model() {
        let result = artificial_analysis_url("gpt-4o");
        assert!(result.is_ok());
        let url = result.unwrap();
        assert_eq!(url.as_str(), "https://artificialanalysis.ai/models/gpt-4o");
    }

    #[test]
    fn test_artificial_analysis_url_with_provider_prefix() {
        let result = artificial_analysis_url("openai/gpt-4o");
        assert!(result.is_ok());
        let url = result.unwrap();
        assert_eq!(url.as_str(), "https://artificialanalysis.ai/models/gpt-4o");
    }

    #[test]
    fn test_artificial_analysis_url_with_preview_suffix() {
        let result = artificial_analysis_url("claude-opus-4.5-20250929-preview");
        assert!(result.is_ok());
        let url = result.unwrap();
        assert_eq!(url.as_str(), "https://artificialanalysis.ai/models/claude-opus-4.5-20250929");
    }

    #[test]
    fn test_artificial_analysis_url_with_colon_separator() {
        let result = artificial_analysis_url("gpt-4:turbo");
        assert!(result.is_ok());
        let url = result.unwrap();
        assert_eq!(url.as_str(), "https://artificialanalysis.ai/models/gpt-4");
    }

    #[test]
    fn test_artificial_analysis_url_all_transformations() {
        let result = artificial_analysis_url("openai/gpt-4-preview:turbo");
        assert!(result.is_ok());
        let url = result.unwrap();
        assert_eq!(url.as_str(), "https://artificialanalysis.ai/models/gpt-4");
    }

    #[test]
    fn test_artificial_analysis_url_empty_string() {
        let result = artificial_analysis_url("");
        assert!(result.is_err());
        match result {
            Err(ProviderError::InvalidUrl { model }) => {
                assert_eq!(model, "");
            }
            _ => panic!("Expected InvalidUrl error"),
        }
    }

    #[test]
    fn test_artificial_analysis_url_unicode_characters() {
        let result = artificial_analysis_url("model-名前");
        assert!(result.is_ok());
        let url = result.unwrap();
        assert_eq!(url.as_str(), "https://artificialanalysis.ai/models/model-%E5%90%8D%E5%89%8D");
    }

    #[test]
    fn test_artificial_analysis_url_multiple_slashes() {
        let result = artificial_analysis_url("provider/subprovider/gpt-4o");
        assert!(result.is_ok());
        let url = result.unwrap();
        // Should only strip the first slash, keeping "subprovider/gpt-4o"
        assert_eq!(url.as_str(), "https://artificialanalysis.ai/models/subprovider/gpt-4o");
    }

    // Property-based tests with proptest
    #[cfg(test)]
    mod property_tests {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            #[test]
            fn prop_never_panics(model in "\\PC*") {
                let _ = artificial_analysis_url(&model);
            }

            #[test]
            fn prop_idempotent_transformations(model in "[a-zA-Z0-9/_:-]{1,100}") {
                if let Ok(url1) = artificial_analysis_url(&model) {
                    // Extract the model name from the first URL
                    let model_name = url1.as_str().strip_prefix("https://artificialanalysis.ai/models/").unwrap();
                    // Apply transformation again
                    if let Ok(url2) = artificial_analysis_url(model_name) {
                        assert_eq!(url1.as_str(), url2.as_str(), "Transformation should be idempotent");
                    }
                }
            }

            #[test]
            fn prop_all_successful_urls_start_with_prefix(model in "[a-zA-Z0-9/_:-]{1,100}") {
                if let Ok(url) = artificial_analysis_url(&model) {
                    assert!(url.as_str().starts_with("https://artificialanalysis.ai/models/"));
                }
            }
        }
    }
}
