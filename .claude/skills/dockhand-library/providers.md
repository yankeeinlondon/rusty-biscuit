# Provider System Architecture

The provider system in the shared library discovers and manages LLM models from multiple providers using a three-tier architecture.

## Three-Tier Architecture

### 1. Base Layer (`providers/base.rs`)

Environment-based API key management and provider configuration:

```rust
pub enum Provider {
    Anthropic,
    Deepseek,
    Gemini,
    MoonshotAI,
    Ollama,
    OpenAI,
    OpenRouter,
    ZAI,
    ZenMux,
}

// Check if provider has API key
if has_provider_api_key(Provider::OpenAI) {
    // Provider is available
}

// Get all available API keys
let keys = get_api_keys();
```

**Environment Variables**:
- `OPENAI_API_KEY` - OpenAI models
- `ANTHROPIC_API_KEY` - Claude models
- `GEMINI_API_KEY` or `GOOGLE_API_KEY` - Gemini models
- `DEEPSEEK_API_KEY` - Deepseek models
- `MOONSHOT_API_KEY` or `MOONSHOT_AI_API_KEY` - MoonshotAI
- `ZAI_API_KEY` or `Z_AI_API_KEY` - ZAI models
- `ZENMUX_API_KEY` or `ZEN_MUX_API_KEY` - ZenMux
- `OPEN_ROUTER_API_KEY` or `OPENROUTER_API_KEY` - OpenRouter

### 2. Curated Registry (`providers/curated.rs`)

Hardcoded model list with metadata:

```rust
// Get curated models
let models = get_curated_models();

// Check metadata
println!("Last updated: {}", LAST_UPDATED);
println!("Provider count: {}", PROVIDER_COUNT);
```

The curated list includes:
- Model IDs and display names
- Context windows and pricing
- Provider information
- Last update timestamp

### 3. Discovery Layer (`providers/discovery.rs`)

Dynamic API-based model discovery with caching:

```rust
use shared::providers::{generate_provider_list, ProviderListFormat};

// Generate as JSON array
let json = generate_provider_list(Some(ProviderListFormat::StringLiterals)).await?;

// Generate as Rust enum
let rust_enum = generate_provider_list(Some(ProviderListFormat::RustEnum)).await?;
```

Features:
- **24-hour caching**: Reduces API calls
- **Rate limiting**: Exponential backoff for retries
- **OpenAI-compatible**: Discovers models via `/v1/models` endpoint
- **Parallel fetching**: Concurrent API requests with `tokio::join!`

## Model Discovery Flow

1. Check environment for API keys
2. Fetch from provider APIs (with cache)
3. Fall back to curated list if API fails
4. Normalize and deduplicate results

## Provider-Specific Features

### OpenAI-Compatible Providers

Providers using the `/v1/models` endpoint:
- OpenAI
- Deepseek
- MoonshotAI
- OpenRouter
- ZenMux

### Custom API Providers

Providers with unique APIs:
- **Anthropic**: Custom models endpoint
- **Gemini**: Google AI Studio API
- **ZAI**: Custom discovery format

### Local Provider

**Ollama**: No API key needed, discovers local models

## Code Generation Integration

The provider system integrates with codegen for updating model enums:

```rust
// Update provider models binary
cargo run --bin update-provider-models

// This fetches latest models and updates src/model/providers.rs
```

## Error Handling

```rust
use shared::providers::ProviderError;

match generate_provider_list(None).await {
    Ok(models) => println!("{}", models),
    Err(ProviderError::ApiError { provider, error }) => {
        eprintln!("Failed to fetch from {}: {}", provider, error);
    }
    Err(ProviderError::NoApiKey(provider)) => {
        eprintln!("No API key for {}", provider);
    }
    // ... other error variants
}
```

## Best Practices

1. **Use curated list for stability**: API discovery can fail
2. **Cache results**: Provider APIs have rate limits
3. **Check API keys first**: Avoid unnecessary API calls
4. **Handle errors gracefully**: Fall back to curated data

## Testing

The module includes comprehensive tests:

```rust
#[cfg(test)]
mod tests {
    // Environment variable isolation with serial_test
    #[serial_test::serial]
    fn test_has_provider_api_key() {
        // Tests use ScopedEnv for cleanup
    }
}
```

## Tracing

All provider operations include tracing:

```rust
#[instrument(level = "debug", skip(client))]
async fn fetch_anthropic_models(client: &Client) -> Result<Vec<LlmEntry>> {
    // Logs API calls and results
}
```