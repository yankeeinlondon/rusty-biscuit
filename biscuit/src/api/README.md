# API Module

This module provides utilities for working with LLM provider APIs, with a focus on OpenAI-compatible endpoints.

## Overview

The `api` module was created during the provider/model refactoring (Phase 1, 2025-12-30) to eliminate code duplication between `providers/base.rs` and `providers/discovery.rs`. It centralizes OpenAI-compatible API utilities and provides type-safe model references.

## Architecture

### Module Structure

```
api/
├── mod.rs              - Module exports and public interface
├── openai_compat.rs    - OpenAI-compatible /v1/models endpoint utilities
├── types.rs            - API-specific type definitions
└── README.md           - This file
```

### Design Principles

**1. DRY (Don't Repeat Yourself):**

- Eliminates duplicate HTTP request code from `base.rs` and `discovery.rs`
- Single source of truth for OpenAI-compatible API interactions

**2. Type Safety:**

- Uses `ProviderModel` enum instead of raw strings
- Compile-time guarantees for provider/model combinations

**3. Instrumentation:**

- All public functions use `#[tracing::instrument]`
- Follows OpenTelemetry semantic conventions
- API keys never logged (always use `skip(api_key)`)

**4. Error Handling:**

- Rich error types from `providers::discovery::ProviderError`
- No `unwrap()` or `expect()` in production code
- Proper Result propagation

## Components

### openai_compat.rs

Utilities for fetching models from OpenAI-compatible provider APIs.

**Key Functions:**

```rust
/// Fetch models from a single provider
#[tracing::instrument(skip(api_key))]
pub async fn get_provider_models_from_api(
    provider: Provider,
    api_key: &str,
) -> Result<Vec<String>, ProviderError>

/// Fetch models from all configured providers (parallel)
pub async fn get_all_provider_models() -> Result<Vec<String>, ProviderError>
```

**Features:**

- Parallel fetching with `futures::stream`
- Retry logic with exponential backoff (via `providers::retry`)
- Rate limiting handling (429 responses)
- Response size limits (10MB max)
- Timeout enforcement (30 seconds)

**Supported Providers:**

- Anthropic (`/v1/models`)
- Deepseek (`/v1/models`)
- Gemini (`/v1/models`)
- MoonshotAI (`/v1/models`)
- Ollama (local, `/v1/models`)
- OpenAI (`/v1/models`)
- OpenRouter (`/v1/models`)
- ZAI (`/v1/models`)

**Skipped Providers:**

- ZenMux (no `/v1/models` endpoint support)

### types.rs

Type definitions specific to API operations.

**Key Types:**

```rust
/// List of provider/model combinations from all providers
pub struct ProviderModelList {
    pub models: Vec<String>,
}
```

**Shared Types:**

- `OpenAIModelsResponse` - Defined in `providers::types`, used here
- `ProviderModel` - Defined in `providers::types`, type-safe model enum

## Usage Examples

### Fetch Models from Single Provider

```rust
use shared::api::openai_compat::get_provider_models_from_api;
use shared::providers::base::Provider;

async fn fetch_openai_models(api_key: &str) -> Result<Vec<String>, ProviderError> {
    get_provider_models_from_api(Provider::OpenAI, api_key).await
}
```

### Fetch Models from All Providers

```rust
use shared::api::openai_compat::get_all_provider_models;

async fn fetch_all() -> Result<Vec<String>, ProviderError> {
    // Fetches in parallel from all providers with configured API keys
    get_all_provider_models().await
}
```

### Error Handling

```rust
use shared::api::openai_compat::get_provider_models_from_api;
use shared::providers::discovery::ProviderError;
use shared::providers::base::Provider;

async fn safe_fetch(api_key: &str) -> Result<Vec<String>, ProviderError> {
    match get_provider_models_from_api(Provider::Anthropic, api_key).await {
        Ok(models) => Ok(models),
        Err(ProviderError::RateLimitExceeded { .. }) => {
            // Handle rate limiting
            Err(ProviderError::RateLimitExceeded {
                provider: "anthropic".to_string(),
                retry_after: None,
            })
        }
        Err(e) => Err(e),
    }
}
```

## Integration with Provider System

The API module works closely with the provider system:

**Dependencies:**

- `providers::base` - Provider enum, base URLs, auth headers
- `providers::types` - Shared types (OpenAIModelsResponse, ProviderModel)
- `providers::discovery` - Error types, retry logic
- `providers::retry` - Exponential backoff with jitter

**Flow:**

```
1. User calls get_all_provider_models()
2. Module checks for API keys (providers::base::has_provider_api_key)
3. Parallel fetch from each provider with API key
4. Each fetch uses get_provider_models_from_api()
5. HTTP request built with providers::base::build_auth_header()
6. Response parsed as OpenAIModelsResponse (providers::types)
7. Results aggregated and returned
```

## Tracing & Observability

All functions emit structured tracing events:

**Spans:**

```rust
#[tracing::instrument(skip(api_key))]
pub async fn get_provider_models_from_api(...)
```

**Events:**

- `debug!` - URL construction, request building
- `info!` - Successful fetches, model counts
- `warn!` - Skipped providers, missing API keys

**Fields (OpenTelemetry):**

- `provider` - Provider name
- `http.url` - Request URL (redacted API keys)
- `http.status_code` - Response status
- `model_count` - Number of models fetched

**Security:**

- API keys NEVER logged (always use `skip(api_key)`)
- Sensitive headers redacted in debug output

## Error Types

From `providers::discovery::ProviderError`:

```rust
pub enum ProviderError {
    /// 429 Too Many Requests
    RateLimitExceeded { provider: String, retry_after: Option<u64> },

    /// 401/403 Authentication Failed
    AuthenticationFailed { provider: String },

    /// Request timeout
    Timeout { provider: String },

    /// Response exceeds size limit
    ResponseTooLarge { provider: String, size: usize },

    /// HTTP request error
    RequestFailed { provider: String, error: String },

    /// JSON parsing error
    ParseError { provider: String, error: String },
}
```

## Testing

**Test Coverage:**

- Single provider fetching
- Parallel multi-provider fetching
- Error handling (rate limits, auth failures, timeouts)
- Response size limits
- Retry logic

**Running Tests:**

```bash
# All API module tests
cargo test -p shared --lib api

# Specific test
cargo test -p shared --lib api::openai_compat::tests::test_get_provider_models

# With output
cargo test -p shared --lib api -- --nocapture
```

**Test Strategy:**

- Uses `wiremock` for HTTP mocking
- Tests both success and failure cases
- Verifies tracing instrumentation

## Performance Considerations

**Parallel Fetching:**

- Uses `futures::stream::StreamExt` for concurrent requests
- Default concurrency: 8 providers in parallel
- Each request has 30-second timeout

**Caching:**

- API module does NOT cache (caching handled by `providers::cache`)
- Allows fresh data when cache expires

**Rate Limiting:**

- Respects `Retry-After` headers
- Exponential backoff with jitter (via `providers::retry`)
- Plans: free (1/sec), base (20/sec), pro (50/sec)

## Migration Notes

This module was created during the provider/model refactoring to address code duplication identified in the code review (`.ai/code-reviews/20251230.provider-base-implementation.md`).

**Before (duplicated code):**

- `providers/base.rs` had inline HTTP request code
- `providers/discovery.rs` had duplicate HTTP request code
- `OpenAIModelsResponse` defined in both files

**After (centralized):**

- `api/openai_compat.rs` contains single HTTP implementation
- `providers/types.rs` has single `OpenAIModelsResponse` definition
- Both `base.rs` and `discovery.rs` use `api` module

**Migration Guide:** See `.ai/docs/provider-model-migration.md`

## Future Enhancements

Potential areas for expansion:

1. **Additional API Standards:**
   - Anthropic native API (`/v1/messages`)
   - Gemini native API (REST API)

2. **Rate Limit Management:**
   - Centralized rate limiter per provider
   - Automatic backoff coordination

3. **Response Caching:**
   - Short-term cache (5 min) for frequent requests
   - Separate from 24-hour provider cache

4. **Health Checks:**
   - Provider availability testing
   - API endpoint monitoring

## Related Documentation

- **Provider Module:** `../providers/README.md` (if exists)
- **Migration Guide:** `.ai/docs/provider-model-migration.md`
- **Architecture Docs:** `/research/docs/architecture.md`
- **Tracing Docs:** `/docs/tracing.md`
- **Code Review:** `.ai/code-reviews/20251230.provider-base-implementation.md`
