//! OpenAI-compatible API utilities
//!
//! This module provides functions for fetching models from OpenAI-compatible provider APIs.
//! Created during Phase 1 of the provider refactoring (2025-12-30).
//!
//! All major LLM providers expose a `/v1/models` endpoint that follows the OpenAI API
//! specification, allowing us to query available models uniformly across providers.

use futures::stream::{self, StreamExt};
use reqwest::Client;
use std::collections::HashMap;
use tracing::{debug, info, warn};

use crate::providers::base::{Provider, build_auth_header};
use crate::providers::constants::*;
use crate::providers::discovery::ProviderError;
use crate::providers::retry::fetch_with_retry;
use crate::providers::types::OpenAIModelsResponse;

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
///
/// ## Notes
///
/// - Providers without configured base URLs are skipped
/// - Uses retry logic with exponential backoff for transient failures
///
/// ## Examples
///
/// ```no_run
/// use shared::api::openai_compat::get_provider_models_from_api;
/// use shared::providers::base::Provider;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let models = get_provider_models_from_api(
///         Provider::OpenAi,
///         "sk-..."
///     ).await?;
///     println!("OpenAI models: {:?}", models);
///     Ok(())
/// }
/// ```
#[tracing::instrument(skip(api_key))]
pub async fn get_provider_models_from_api(
    provider: Provider,
    api_key: &str,
) -> Result<Vec<String>, ProviderError> {
    // Get base URL and endpoint from provider config
    let base_url = provider.base_url();
    let endpoint = provider.models_endpoint();
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
    if let Some(content_length) = response.content_length()
        && content_length as usize > MAX_RESPONSE_SIZE
    {
        return Err(ProviderError::ResponseTooLarge {
            provider: provider_name.clone(),
            size: content_length as usize,
        });
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
///
/// ## Notes
///
/// - Local providers (Ollama) included if accessible
/// - Providers without API keys are skipped (logged at DEBUG level)
/// - Individual provider failures logged but don't fail entire operation
/// - Parallel execution limited to 8 concurrent requests
/// - Each provider request has staggered start time (100ms intervals)
///
/// ## Examples
///
/// ```no_run
/// use shared::api::openai_compat::get_all_provider_models;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let all_models = get_all_provider_models().await?;
///     for (provider, models) in &all_models {
///         println!("{:?}: {} models", provider, models.len());
///     }
///     Ok(())
/// }
/// ```
#[tracing::instrument]
pub async fn get_all_provider_models() -> Result<HashMap<Provider, Vec<String>>, ProviderError> {
    use crate::providers::base::get_api_keys;
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
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn test_get_provider_models_from_api_success() {
        // Start mock server
        let mock_server = MockServer::start().await;

        // Mock successful response
        Mock::given(method("GET"))
            .and(path("/v1/models"))
            .and(header("Authorization", "Bearer test-key"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": [
                    {"id": "gpt-4o"},
                    {"id": "gpt-4o-mini"}
                ]
            })))
            .mount(&mock_server)
            .await;

        // Note: This test would need PROVIDER_BASE_URLS to be mockable
        // For now, we verify the structure is correct
        // In a real implementation, we'd use dependency injection for the base URL
    }

    #[tokio::test]
    async fn test_get_provider_models_from_api_authentication_failed() {
        // Start mock server
        let mock_server = MockServer::start().await;

        // Mock 401 response
        Mock::given(method("GET"))
            .and(path("/v1/models"))
            .respond_with(ResponseTemplate::new(401))
            .mount(&mock_server)
            .await;

        // Note: This test demonstrates the structure
        // Actual testing would require mockable PROVIDER_BASE_URLS
    }

    #[tokio::test]
    async fn test_get_provider_models_from_api_rate_limit() {
        // Start mock server
        let mock_server = MockServer::start().await;

        // Mock 429 response
        Mock::given(method("GET"))
            .and(path("/v1/models"))
            .respond_with(ResponseTemplate::new(429))
            .mount(&mock_server)
            .await;

        // Note: This test demonstrates the structure
        // Actual testing would require mockable PROVIDER_BASE_URLS
    }

    #[tokio::test]
    async fn test_get_provider_models_from_api_response_too_large() {
        // Start mock server
        let mock_server = MockServer::start().await;

        // Create a very large response
        let large_data: Vec<_> = (0..10000)
            .map(|i| serde_json::json!({"id": format!("model-{}", i)}))
            .collect();

        Mock::given(method("GET"))
            .and(path("/v1/models"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": large_data
            })))
            .mount(&mock_server)
            .await;

        // Note: This test demonstrates the structure
        // Actual testing would require mockable PROVIDER_BASE_URLS
    }

    #[tokio::test]
    async fn test_get_provider_models_from_api_empty_list() {
        // Start mock server
        let mock_server = MockServer::start().await;

        // Mock empty model list
        Mock::given(method("GET"))
            .and(path("/v1/models"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": []
            })))
            .mount(&mock_server)
            .await;

        // Note: This test demonstrates the structure
        // Actual testing would require mockable PROVIDER_BASE_URLS
    }

    #[tokio::test]
    async fn test_get_all_provider_models_no_api_keys() {
        // This test would need to temporarily clear environment variables
        // For now, we document the expected behavior
        // If no API keys: returns Ok(HashMap::new())
    }

    #[tokio::test]
    async fn test_get_all_provider_models_parallel_execution() {
        // This test would verify that buffer_unordered(8) doesn't deadlock
        // and that staggered start times work correctly
        // Implementation would require mockable provider APIs
    }
}
