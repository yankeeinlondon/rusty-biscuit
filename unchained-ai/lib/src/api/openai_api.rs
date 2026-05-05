//! OpenAI-compatible API utilities
//!
//! This module provides functions for fetching models from OpenAI-compatible provider APIs.
//! All major LLM providers expose a `/v1/models` endpoint that follows the OpenAI API
//! specification, allowing us to query available models uniformly across providers.

use std::collections::HashMap;
use std::env;

use futures::stream::{self, StreamExt};
use reqwest::Client;
use serde::Deserialize;
use tracing::{debug, info, warn};

use crate::api::auth::ApiAuthMethod;
use crate::rigging::providers::Provider;
use crate::rigging::providers::provider_errors::ProviderError;

/// Maximum response size in bytes (10 MB)
const MAX_RESPONSE_SIZE: usize = 10 * 1024 * 1024;

/// Maximum number of retries for transient failures
const MAX_RETRIES: u32 = 3;

/// Base delay between retries in milliseconds
const RETRY_BASE_DELAY_MS: u64 = 1000;

/// Response from provider /v1/models endpoint.
///
/// Preserves the full JSON value for each model so that provider-native
/// metadata (pricing, architecture, etc.) is available to downstream consumers
/// rather than being silently discarded during deserialization.
#[derive(Debug, Deserialize)]
pub struct ProviderModelsResponse {
    /// Standard OpenAI-style `data` array. Each element is kept as a raw
    /// [`serde_json::Value`] so provider-specific fields are not lost.
    #[serde(default)]
    pub data: Vec<serde_json::Value>,

    /// Gemini uses `"models"` instead of `"data"`.
    #[serde(default)]
    pub models: Vec<GeminiModel>,
}

/// A parsed model entry with its raw provider-native metadata preserved.
///
/// The `id` field is always populated. `raw_metadata` is `Some` when the
/// provider response included additional fields beyond `id`.
#[derive(Debug, Clone)]
pub struct ProviderModelEntry {
    /// Model identifier (unprefixed).
    pub id: String,
    /// The full JSON object for this model from the provider response, or
    /// `None` when only an ID was available (e.g. Gemini `name` field).
    pub raw_metadata: Option<serde_json::Value>,
}

/// Single model entry from Gemini API.
#[derive(Debug, Deserialize)]
pub struct GeminiModel {
    /// Gemini uses "name" like "models/gemini-1.5-flash"
    pub name: String,
}

#[deprecated(
    since = "0.2.0",
    note = "Use ProviderModelsResponse instead. The old type discarded provider-native metadata."
)]
pub type OpenAIModelsResponse = ProviderModelsResponse;

#[deprecated(
    since = "0.2.0",
    note = "Use ProviderModelEntry instead. The old type discarded provider-native metadata."
)]
pub type OpenAIModel = ProviderModelEntry;

/// Build the authentication header for a provider
fn build_auth_header(provider: &Provider, api_key: &str) -> (String, String) {
    match &provider.config().auth_method {
        ApiAuthMethod::BearerToken => ("Authorization".to_string(), format!("Bearer {}", api_key)),
        ApiAuthMethod::ApiKey(header) => (header.clone(), api_key.to_string()),
        ApiAuthMethod::QueryParam(_) => (String::new(), String::new()),
        ApiAuthMethod::None => (String::new(), String::new()),
    }
}

/// Fetch with retry logic for transient failures
async fn fetch_with_retry<F, Fut, T, E>(mut operation: F, provider_name: &str) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
    E: std::fmt::Display,
{
    let mut last_error = None;

    for attempt in 0..MAX_RETRIES {
        match operation().await {
            Ok(result) => return Ok(result),
            Err(e) => {
                let delay = RETRY_BASE_DELAY_MS * 2u64.pow(attempt);
                warn!(
                    "Attempt {} failed for {}: {}. Retrying in {}ms",
                    attempt + 1,
                    provider_name,
                    e,
                    delay
                );
                tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
                last_error = Some(e);
            }
        }
    }

    Err(last_error.unwrap())
}

/// Get API keys from environment for all providers that have them configured.
///
/// Returns a HashMap mapping Provider to API key string.
pub fn get_api_keys() -> HashMap<Provider, String> {
    use strum::IntoEnumIterator;

    let mut keys = HashMap::new();

    for provider in Provider::iter() {
        let config = provider.config();

        // Skip local providers
        if config.is_local {
            continue;
        }

        // Try each env var for this provider
        for env_var in config.env_vars {
            if let Ok(key) = env::var(env_var)
                && !key.is_empty()
            {
                keys.insert(provider, key);
                break;
            }
        }
    }

    keys
}

/// Fetch models from a single provider's OpenAI-compatible API.
///
/// Queries the provider's `/v1/models` endpoint and returns model entries that
/// preserve the full provider-native JSON metadata alongside each model ID.
///
/// ## Arguments
///
/// * `provider` - The provider to query
/// * `api_key` - API key for authentication
///
/// ## Returns
///
/// Vec of [`ProviderModelEntry`] (unprefixed IDs with optional raw metadata)
///
/// ## Errors
///
/// - `ProviderError::RateLimitExceeded` - 429 response from provider
/// - `ProviderError::AuthenticationFailed` - 401/403 response
/// - `ProviderError::Timeout` - Request exceeded timeout
/// - `ProviderError::ResponseTooLarge` - Response size exceeds limit
#[tracing::instrument(skip(api_key))]
pub async fn get_provider_models_from_api(
    provider: Provider,
    api_key: &str,
) -> Result<Vec<ProviderModelEntry>, ProviderError> {
    let config = provider.config();
    let base_url = config.base_url;
    let endpoint = config.models_endpoint.unwrap_or("/v1/models");

    // Build URL with query param auth if needed
    let url = match &config.auth_method {
        ApiAuthMethod::QueryParam(param_name) => {
            format!("{}{}?{}={}", base_url, endpoint, param_name, api_key)
        }
        _ => format!("{}{}", base_url, endpoint),
    };

    let provider_name = format!("{:?}", provider).to_lowercase();

    debug!(
        "Fetching models from {} at {}",
        provider_name,
        url.split('?').next().unwrap_or(&url)
    );

    let (header_name, header_value) = build_auth_header(&provider, api_key);

    let client = Client::new();
    let mut request = client.get(&url);

    if !header_name.is_empty() {
        request = request.header(&header_name, &header_value);
    }

    if provider == Provider::Anthropic {
        request = request.header("anthropic-version", "2023-06-01");
    }

    let response = request.send().await?;

    if response.status().as_u16() == 401 || response.status().as_u16() == 403 {
        return Err(ProviderError::AuthenticationFailed {
            provider: provider_name.clone(),
        });
    }

    if response.status().as_u16() == 429 {
        return Err(ProviderError::RateLimitExceeded {
            provider: provider_name.clone(),
        });
    }

    if let Some(content_length) = response.content_length()
        && content_length as usize > MAX_RESPONSE_SIZE
    {
        return Err(ProviderError::ResponseTooLarge {
            provider: provider_name.clone(),
            size: content_length as usize,
        });
    }

    let data: ProviderModelsResponse = response.json().await?;

    // Extract model entries — handle both OpenAI format (data) and Gemini format (models)
    let entries: Vec<ProviderModelEntry> = if !data.data.is_empty() {
        data.data
            .into_iter()
            .map(|value| {
                let id = value
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("<unknown>")
                    .to_string();
                ProviderModelEntry {
                    id,
                    raw_metadata: Some(value),
                }
            })
            .collect()
    } else if !data.models.is_empty() {
        data.models
            .into_iter()
            .map(|model| {
                let id = model
                    .name
                    .strip_prefix("models/")
                    .unwrap_or(&model.name)
                    .to_string();
                ProviderModelEntry {
                    id,
                    raw_metadata: None,
                }
            })
            .collect()
    } else {
        vec![]
    };

    info!(
        "Fetched {} models from {}",
        entries.len(),
        provider_name
    );

    Ok(entries)
}

/// Fetch models from all available providers
///
/// Queries all providers that have API keys configured in environment.
/// Runs API calls in parallel for efficiency using `buffer_unordered(8)`.
///
/// ## Returns
///
/// HashMap mapping Provider to Vec of [`ProviderModelEntry`]
///
/// ## Errors
///
/// Returns `ProviderError::NoProvidersAvailable` if no API keys are configured.
/// Individual provider failures are logged but don't fail the entire operation.
#[tracing::instrument]
pub async fn get_all_provider_models(
) -> Result<HashMap<Provider, Vec<ProviderModelEntry>>, ProviderError> {
    use std::time::Duration;

    let api_keys = get_api_keys();

    if api_keys.is_empty() {
        info!("No API keys configured, returning empty result");
        return Ok(HashMap::new());
    }

    info!("Fetching models from {} providers", api_keys.len());

    let provider_futures: Vec<_> = api_keys
        .iter()
        .enumerate()
        .map(|(i, (provider, api_key))| {
            let provider = *provider;
            let api_key = api_key.clone();
            async move {
                tokio::time::sleep(Duration::from_millis(100 * i as u64)).await;

                let provider_name = format!("{:?}", provider).to_lowercase();
                let result = fetch_with_retry(
                    || get_provider_models_from_api(provider, &api_key),
                    &provider_name,
                )
                .await;

                (provider, result)
            }
        })
        .collect();

    let results: Vec<(Provider, Result<Vec<ProviderModelEntry>, ProviderError>)> =
        stream::iter(provider_futures)
            .buffer_unordered(8)
            .collect()
            .await;

    let mut all_models = HashMap::new();
    for (provider, result) in results {
        match result {
            Ok(models) => {
                all_models.insert(provider, models);
            }
            Err(e) => {
                warn!("Failed to fetch models from {:?}: {}", provider, e);
            }
        }
    }

    info!(
        "Successfully fetched models from {} providers",
        all_models.len()
    );

    Ok(all_models)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_auth_header_bearer() {
        let (name, value) = build_auth_header(&Provider::OpenAi, "test-key");
        assert_eq!(name, "Authorization");
        assert_eq!(value, "Bearer test-key");
    }

    #[test]
    fn test_build_auth_header_api_key() {
        let (name, value) = build_auth_header(&Provider::Anthropic, "test-key");
        assert_eq!(name, "x-api-key");
        assert_eq!(value, "test-key");
    }

    #[test]
    fn test_build_auth_header_none() {
        let (name, value) = build_auth_header(&Provider::Ollama, "");
        assert!(name.is_empty());
        assert!(value.is_empty());
    }

    #[test]
    fn test_provider_models_response_deserializes_openai_format() {
        let json = r#"{"data":[{"id":"gpt-4o","owned_by":"openai"}]}"#;
        let resp: ProviderModelsResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.data.len(), 1);
        let entry = &resp.data[0];
        assert_eq!(entry["id"].as_str(), Some("gpt-4o"));
        assert_eq!(entry["owned_by"].as_str(), Some("openai"));
    }

    #[test]
    fn test_provider_models_response_deserializes_openrouter_format() {
        let json = r#"{"data":[{"id":"x-ai/grok-4.3","name":"xAI: Grok 4.3","pricing":{"prompt":"0.00000125","completion":"0.0000025"}}]}"#;
        let resp: ProviderModelsResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.data.len(), 1);
        let entry = &resp.data[0];
        assert_eq!(entry["id"].as_str(), Some("x-ai/grok-4.3"));
        assert_eq!(entry["pricing"]["prompt"].as_str(), Some("0.00000125"));
    }

    #[test]
    fn test_provider_models_response_deserializes_gemini_format() {
        let json = r#"{"models":[{"name":"models/gemini-1.5-flash"}]}"#;
        let resp: ProviderModelsResponse = serde_json::from_str(json).unwrap();
        assert!(resp.data.is_empty());
        assert_eq!(resp.models.len(), 1);
        assert_eq!(resp.models[0].name, "models/gemini-1.5-flash");
    }

    #[test]
    fn test_provider_models_response_empty() {
        let json = r#"{}"#;
        let resp: ProviderModelsResponse = serde_json::from_str(json).unwrap();
        assert!(resp.data.is_empty());
        assert!(resp.models.is_empty());
    }

    #[test]
    fn test_provider_model_entry_from_openai_data() {
        let value: serde_json::Value =
            serde_json::from_str(r#"{"id":"gpt-4o","owned_by":"openai","created":1700000000}"#)
                .unwrap();
        let entry = ProviderModelEntry {
            id: value["id"].as_str().unwrap().to_string(),
            raw_metadata: Some(value),
        };
        assert_eq!(entry.id, "gpt-4o");
        assert!(entry.raw_metadata.is_some());
        assert_eq!(
            entry.raw_metadata.as_ref().unwrap()["created"].as_i64(),
            Some(1700000000)
        );
    }
}
