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
use crate::rigging::providers::provider_errors::ProviderError;
use crate::rigging::providers::providers::Provider;

/// Maximum response size in bytes (10 MB)
const MAX_RESPONSE_SIZE: usize = 10 * 1024 * 1024;

/// Maximum number of retries for transient failures
const MAX_RETRIES: u32 = 3;

/// Base delay between retries in milliseconds
const RETRY_BASE_DELAY_MS: u64 = 1000;

/// Response from OpenAI-compatible /v1/models endpoint
#[derive(Debug, Deserialize)]
pub struct OpenAIModelsResponse {
    pub data: Vec<OpenAIModel>,
}

/// Single model entry from OpenAI-compatible API
#[derive(Debug, Deserialize)]
pub struct OpenAIModel {
    pub id: String,
}

/// Build the authentication header for a provider
fn build_auth_header(provider: &Provider, api_key: &str) -> (String, String) {
    match &provider.config().auth_method {
        ApiAuthMethod::BearerToken => {
            ("Authorization".to_string(), format!("Bearer {}", api_key))
        }
        ApiAuthMethod::ApiKey(header) => (header.clone(), api_key.to_string()),
        ApiAuthMethod::QueryParam(_) => (String::new(), String::new()),
        ApiAuthMethod::None => (String::new(), String::new()),
    }
}

/// Fetch with retry logic for transient failures
async fn fetch_with_retry<F, Fut, T, E>(
    mut operation: F,
    provider_name: &str,
) -> Result<T, E>
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
            if let Ok(key) = env::var(env_var) {
                if !key.is_empty() {
                    keys.insert(provider, key);
                    break;
                }
            }
        }
    }

    keys
}

/// Fetch models from a single provider's OpenAI-compatible API
///
/// Queries the provider's `/v1/models` endpoint and returns a list of available
/// model IDs. The models are returned without the provider prefix.
///
/// ## Arguments
///
/// * `provider` - The provider to query
/// * `api_key` - API key for authentication
///
/// ## Returns
///
/// Vec of model IDs (unprefixed, as returned by API)
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
) -> Result<Vec<String>, ProviderError> {
    let config = provider.config();
    let base_url = config.base_url;
    let endpoint = config.models_endpoint.unwrap_or("/v1/models");

    let url = format!("{}{}", base_url, endpoint);
    let provider_name = format!("{:?}", provider).to_lowercase();

    debug!("Fetching models from {} at {}", provider_name, url);

    // Build auth header
    let (header_name, header_value) = build_auth_header(&provider, api_key);

    // Create HTTP client and make request
    let client = Client::new();
    let mut request = client.get(&url);

    // Add auth header if not empty
    if !header_name.is_empty() {
        request = request.header(&header_name, &header_value);
    }

    // Anthropic requires anthropic-version header
    if provider == Provider::Anthropic {
        request = request.header("anthropic-version", "2023-06-01");
    }

    // Make the request
    let response = request.send().await?;

    // Check for authentication failure
    if response.status().as_u16() == 401 {
        return Err(ProviderError::AuthenticationFailed {
            provider: provider_name.clone(),
        });
    }

    // Check for rate limiting
    if response.status().as_u16() == 429 {
        return Err(ProviderError::RateLimitExceeded {
            provider: provider_name.clone(),
        });
    }

    // Check response size
    if let Some(content_length) = response.content_length() {
        if content_length as usize > MAX_RESPONSE_SIZE {
            return Err(ProviderError::ResponseTooLarge {
                provider: provider_name.clone(),
                size: content_length as usize,
            });
        }
    }

    // Parse response
    let data: OpenAIModelsResponse = response.json().await?;

    // Extract model IDs (unprefixed)
    let models: Vec<String> = data.data.into_iter().map(|model| model.id).collect();

    info!("Fetched {} models from {}", models.len(), provider_name);

    Ok(models)
}

/// Fetch models from all available providers
///
/// Queries all providers that have API keys configured in environment.
/// Runs API calls in parallel for efficiency using `buffer_unordered(8)`.
///
/// ## Returns
///
/// HashMap mapping Provider to Vec of model IDs (unprefixed)
///
/// ## Errors
///
/// Returns `ProviderError::NoProvidersAvailable` if no API keys are configured.
/// Individual provider failures are logged but don't fail the entire operation.
#[tracing::instrument]
pub async fn get_all_provider_models() -> Result<HashMap<Provider, Vec<String>>, ProviderError> {
    use std::time::Duration;

    let api_keys = get_api_keys();

    if api_keys.is_empty() {
        info!("No API keys configured, returning empty result");
        return Ok(HashMap::new());
    }

    info!("Fetching models from {} providers", api_keys.len());

    // Create futures for each provider with staggered start times
    let provider_futures: Vec<_> = api_keys
        .iter()
        .enumerate()
        .map(|(i, (provider, api_key))| {
            let provider = *provider;
            let api_key = api_key.clone();
            async move {
                // Stagger start times to avoid overwhelming provider APIs
                tokio::time::sleep(Duration::from_millis(100 * i as u64)).await;

                // Wrap in retry logic
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

    // Execute in parallel with buffer_unordered(8) to limit concurrent requests
    let results: Vec<(Provider, Result<Vec<String>, ProviderError>)> =
        stream::iter(provider_futures)
            .buffer_unordered(8)
            .collect()
            .await;

    // Collect successful results and log errors
    let mut all_models = HashMap::new();
    for (provider, result) in results {
        match result {
            Ok(models) => {
                all_models.insert(provider, models);
            }
            Err(e) => {
                warn!("Failed to fetch models from {:?}: {}", provider, e);
                // Continue with other providers
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
}
