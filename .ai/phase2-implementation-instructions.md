# Phase 2: Provider Model Discovery Implementation

## Context
You are implementing Phase 2 of the Provider Base Implementation Plan. This phase adds the `get_provider_models()` function to fetch models from OpenAI-compatible APIs.

## Skills Required
- rust
- rust-testing
- rust-logging
- tokio
- thiserror
- reqwest

## File to Edit
`/Volumes/coding/personal/dockhand/shared/src/providers/base.rs`

## Implementation Requirements

### 1. Add PROVIDER_BASE_URLS HashMap (~20 lines)
```rust
lazy_static! {
    static ref PROVIDER_BASE_URLS: HashMap<Provider, &'static str> = {
        let mut urls = HashMap::new();
        urls.insert(Provider::Anthropic, "https://api.anthropic.com");
        urls.insert(Provider::Deepseek, "https://api.deepseek.com");
        urls.insert(Provider::Gemini, "https://generativelanguage.googleapis.com");
        urls.insert(Provider::MoonshotAi, "https://api.moonshot.cn");
        urls.insert(Provider::Ollama, "http://localhost:11434");
        urls.insert(Provider::OpenAi, "https://api.openai.com");
        urls.insert(Provider::OpenRouter, "https://openrouter.ai/api");
        urls.insert(Provider::Zai, "https://api.zai.chat");
        // ZenMux marked as unsupported (no /v1/models endpoint)
        urls
    };
}
```

### 2. Add Optional PROVIDER_MODELS_ENDPOINT HashMap
For providers with non-standard endpoints (like Gemini which may use `/v1beta/models`):
```rust
lazy_static! {
    static ref PROVIDER_MODELS_ENDPOINT: HashMap<Provider, &'static str> = {
        let mut endpoints = HashMap::new();
        // Default is "/v1/models" - only add exceptions here
        // endpoints.insert(Provider::Gemini, "/v1beta/models");  // if needed
        endpoints
    };
}
```

### 3. Add build_auth_header() Helper
```rust
/// Builds the authentication header for a provider's API request
fn build_auth_header(provider: &Provider, api_key: &str) -> (String, String) {
    match PROVIDER_AUTH.get(provider) {
        Some(ApiAuthMethod::BearerToken) => {
            ("Authorization".to_string(), format!("Bearer {}", api_key))
        }
        Some(ApiAuthMethod::ApiKey(header_name)) => {
            (header_name.clone(), api_key.to_string())
        }
        Some(ApiAuthMethod::None) | None => {
            // For local providers or unknown auth, no header needed
            ("".to_string(), "".to_string())
        }
    }
}
```

### 4. Implement get_provider_models() Function

**Signature:**
```rust
pub async fn get_provider_models() -> Result<Vec<String>, ProviderError>
```

**Implementation Steps:**
1. Call `get_api_keys()` to get available providers
2. For each provider with an API key:
   - Get base URL from `PROVIDER_BASE_URLS`
   - Get endpoint (default "/v1/models" or from `PROVIDER_MODELS_ENDPOINT`)
   - Build full URL: `{base_url}{endpoint}`
   - Build auth header using `build_auth_header()`
   - Make HTTP request using reqwest
   - Parse OpenAI-compatible response (reuse `OpenAIModelsResponse` from discovery.rs)
   - Prefix each model ID with provider name: `{provider}/{model_id}`
3. Run API calls in parallel using `futures::stream::iter().buffer_unordered(8)`
4. Handle errors: rate limiting (429), auth failures (401), timeouts
5. Return Vec of fully-qualified model strings

**Parallel Execution Pattern:**
```rust
use futures::stream::{self, StreamExt};

let api_keys = get_api_keys();
let provider_futures: Vec<_> = api_keys
    .iter()
    .enumerate()
    .map(|(i, (provider, api_key))| {
        let provider = *provider;
        let api_key = api_key.clone();
        async move {
            // Stagger start times
            tokio::time::sleep(Duration::from_millis(100 * i as u64)).await;
            fetch_models_for_provider(provider, &api_key).await
        }
    })
    .collect();

let results: Vec<Result<Vec<String>, ProviderError>> = stream::iter(provider_futures)
    .buffer_unordered(8)
    .collect()
    .await;
```

**Error Handling:**
- Use `fetch_with_retry` from discovery.rs for rate limiting
- Return `ProviderError::AuthenticationFailed` for 401
- Return `ProviderError::RateLimitExceeded` for 429 (after retries)
- Return `ProviderError::Timeout` for timeouts
- Return `ProviderError::ResponseTooLarge` if response > MAX_RESPONSE_SIZE

### 5. Unit Tests (Minimum 5)

Add these tests in `#[cfg(test)] mod tests`:

1. **test_get_provider_models_single_provider**: Test with one provider, mocked HTTP response
2. **test_get_provider_models_multiple_providers**: Test parallel execution with multiple providers
3. **test_get_provider_models_rate_limiting**: Test retry logic on 429 error
4. **test_get_provider_models_auth_failure**: Test 401 error handling
5. **test_get_provider_models_malformed_json**: Test invalid JSON response

Use `wiremock::MockServer` for HTTP mocking.

### 6. Integration Test

Add `test_get_provider_models_parallel_execution` to verify:
- Multiple providers are called in parallel
- Execution time is < (num_providers * individual_time) (proves parallelism)
- Use `tokio::time::Instant` to measure timing

## Important Notes

1. **ZenMux**: Skip this provider in `get_provider_models()` - it doesn't support `/v1/models` endpoint
2. **Ollama**: Include if running (local provider with no auth)
3. **Gemini**: Verify endpoint - may need `/v1beta/models` instead of `/v1/models`
4. **Error Types**: All error types already exist in `ProviderError` (discovery.rs)
5. **Response Type**: Reuse `OpenAIModelsResponse` from discovery.rs (lines 48-57)
6. **Rate Limiting**: Reuse `fetch_with_retry` from discovery.rs (lines 81-119)
7. **Constants**: Use `MAX_RESPONSE_SIZE`, `REQUEST_TIMEOUT` from discovery.rs

## Dependencies Already Added
- `reqwest` (HTTP client)
- `wiremock` (dev-dependency for testing)
- `futures` (for buffer_unordered)
- `tokio` (async runtime)
- `serial_test` (test isolation)

## Acceptance Criteria
- [ ] PROVIDER_BASE_URLS HashMap added (~20 lines)
- [ ] PROVIDER_MODELS_ENDPOINT HashMap added
- [ ] build_auth_header() function implemented
- [ ] get_provider_models() implementation exists (no todo!())
- [ ] Returns Result<Vec<String>, ProviderError>
- [ ] Calls get_api_keys()
- [ ] Makes HTTP requests to /v1/models
- [ ] Uses correct authentication headers
- [ ] Handles rate limiting with retry
- [ ] Prefixes model IDs with provider name
- [ ] Runs API calls in parallel (buffer_unordered(8))
- [ ] Minimum 5 unit tests with mocked HTTP
- [ ] Integration test for parallel execution
- [ ] All tests pass: `cargo test --lib providers::base`

## Logging
Use `#[tracing::instrument(skip(api_key))]` on functions that handle API keys to avoid logging secrets.

Log at:
- `info!` level: Successful operations (e.g., "Fetched 12 models from OpenAI")
- `warn!` level: Expected failures (e.g., "Provider API call failed, skipping")
- `debug!` level: Detailed execution flow

## Output
Report back with:
1. Summary of changes made
2. List of files modified with line counts
3. Test results (count, pass/fail)
4. Any issues encountered
