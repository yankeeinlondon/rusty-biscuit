//! Provider discovery via official APIs and hardcoded lists

use super::cache::{acquire_fetch_lock, check_cache, write_cache};
use super::types::{LlmEntry, ProviderListFormat};
use reqwest::Client;
use serde::Deserialize;
use std::collections::HashSet;
use std::future::Future;
use std::time::Duration;
use thiserror::Error;
use tracing::{info, warn};

/// Rate limiting constants
const INITIAL_RETRY_DELAY: Duration = Duration::from_secs(1);
const MAX_RETRY_DELAY: Duration = Duration::from_secs(30);
const RETRY_MULTIPLIER: f64 = 2.0;
const MAX_RETRIES: u32 = 3;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
const MAX_RESPONSE_SIZE: usize = 10 * 1024 * 1024; // 10MB

#[derive(Debug, Error)]
pub enum ProviderError {
    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("Rate limit exceeded for provider {provider}")]
    RateLimitExceeded { provider: String },

    #[error("API authentication failed for {provider}")]
    AuthenticationFailed { provider: String },

    #[error("JSON serialization failed: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Timeout waiting for {provider}")]
    Timeout { provider: String },

    #[error("Cache error: {0}")]
    CacheError(#[from] super::cache::CacheError),

    #[error("Response too large from {provider}: {size} bytes")]
    ResponseTooLarge { provider: String, size: usize },

    #[error("Invalid model name for URL generation: {model}")]
    InvalidUrl { model: String },
}

/// OpenAI API response for /v1/models
#[derive(Debug, Deserialize)]
struct OpenAIModelsResponse {
    data: Vec<OpenAIModel>,
}

#[derive(Debug, Deserialize)]
struct OpenAIModel {
    id: String,
}

/// Hugging Face API response for /api/models
#[derive(Debug, Deserialize)]
struct HuggingFaceModel {
    #[serde(rename = "modelId")]
    model_id: String,
    #[serde(default)]
    tags: Vec<String>,
}

/// Check if an error is a rate limit error
fn is_rate_limit_error(error: &ProviderError) -> bool {
    match error {
        ProviderError::HttpError(e) => e
            .status()
            .map(|s| s.as_u16() == 429)
            .unwrap_or(false),
        ProviderError::RateLimitExceeded { .. } => true,
        _ => false,
    }
}

/// Fetch with exponential backoff retry logic
async fn fetch_with_retry<F, Fut, T>(
    fetch_fn: F,
    provider_name: &str,
) -> Result<T, ProviderError>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<T, ProviderError>>,
{
    let mut delay = INITIAL_RETRY_DELAY;

    for attempt in 0..=MAX_RETRIES {
        match tokio::time::timeout(REQUEST_TIMEOUT, fetch_fn()).await {
            Ok(Ok(result)) => return Ok(result),
            Ok(Err(e)) if is_rate_limit_error(&e) && attempt < MAX_RETRIES => {
                warn!(
                    "Rate limit hit for {}, retry {} after {:?}",
                    provider_name,
                    attempt + 1,
                    delay
                );
                tokio::time::sleep(delay).await;
                delay = std::cmp::min(
                    Duration::from_secs_f64(delay.as_secs_f64() * RETRY_MULTIPLIER),
                    MAX_RETRY_DELAY,
                );
            }
            Ok(Err(e)) => return Err(e),
            Err(_) => {
                return Err(ProviderError::Timeout {
                    provider: provider_name.to_string(),
                })
            }
        }
    }

    Err(ProviderError::RateLimitExceeded {
        provider: provider_name.to_string(),
    })
}

/// Fetch models from OpenAI API
async fn fetch_openai_models() -> Result<Vec<LlmEntry>, ProviderError> {
    let api_key = match std::env::var("OPENAI_API_KEY") {
        Ok(key) if !key.is_empty() => key,
        _ => {
            warn!("OPENAI_API_KEY not set, skipping OpenAI models");
            return Ok(vec![]);
        }
    };

    let client = Client::new();
    let response = client
        .get("https://api.openai.com/v1/models")
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await?;

    if response.status().as_u16() == 401 {
        return Err(ProviderError::AuthenticationFailed {
            provider: "openai".to_string(),
        });
    }

    if response.status().as_u16() == 429 {
        return Err(ProviderError::RateLimitExceeded {
            provider: "openai".to_string(),
        });
    }

    // Check response size
    if let Some(content_length) = response.content_length()
        && content_length as usize > MAX_RESPONSE_SIZE
    {
        return Err(ProviderError::ResponseTooLarge {
            provider: "openai".to_string(),
            size: content_length as usize,
        });
    }

    let data: OpenAIModelsResponse = response.json().await?;

    let count = data.data.len();
    let entries = data
        .data
        .into_iter()
        .map(|model| LlmEntry::new("openai", model.id))
        .collect();

    info!("Fetched {} OpenAI models", count);
    Ok(entries)
}

/// Get hardcoded Anthropic models (no public API available)
async fn fetch_anthropic_models() -> Result<Vec<LlmEntry>, ProviderError> {
    Ok(vec![
        LlmEntry::new("anthropic", "claude-opus-4.5-20250929"),
        LlmEntry::new("anthropic", "claude-sonnet-4.5-20250929"),
        LlmEntry::new("anthropic", "claude-haiku-4.5-20250929"),
    ])
}

/// Fetch models from Hugging Face API
async fn fetch_huggingface_models() -> Result<Vec<LlmEntry>, ProviderError> {
    let api_token = match std::env::var("HUGGINGFACE_TOKEN") {
        Ok(token) if !token.is_empty() => token,
        _ => {
            warn!("HUGGINGFACE_TOKEN not set, skipping Hugging Face models");
            return Ok(vec![]);
        }
    };

    let client = Client::new();
    let response = client
        .get("https://huggingface.co/api/models")
        .header("Authorization", format!("Bearer {}", api_token))
        .query(&[("filter", "text-generation"), ("limit", "100")])
        .send()
        .await?;

    if response.status().as_u16() == 401 {
        return Err(ProviderError::AuthenticationFailed {
            provider: "huggingface".to_string(),
        });
    }

    if response.status().as_u16() == 429 {
        return Err(ProviderError::RateLimitExceeded {
            provider: "huggingface".to_string(),
        });
    }

    // Check response size
    if let Some(content_length) = response.content_length()
        && content_length as usize > MAX_RESPONSE_SIZE
    {
        return Err(ProviderError::ResponseTooLarge {
            provider: "huggingface".to_string(),
            size: content_length as usize,
        });
    }

    let models: Vec<HuggingFaceModel> = response.json().await?;

    let entries: Vec<LlmEntry> = models
        .into_iter()
        .filter(|m| m.tags.contains(&"text-generation".to_string()))
        .map(|model| LlmEntry::new("huggingface", model.model_id))
        .collect();

    info!("Fetched {} Hugging Face models", entries.len());
    Ok(entries)
}

/// Get hardcoded Google Gemini models (no easy public API)
async fn fetch_gemini_models() -> Result<Vec<LlmEntry>, ProviderError> {
    Ok(vec![
        LlmEntry::new("gemini", "gemini-3-flash-preview"),
        LlmEntry::new("gemini", "gemini-5-pro"),
    ])
}

/// Normalize provider name (lowercase, replace spaces/hyphens)
fn normalize_provider_name(name: &str) -> String {
    name.to_lowercase().replace([' ', '-'], "_")
}

/// Convert to Rust enum variant name
fn to_enum_variant(provider: &str, model: &str) -> String {
    let provider_part = capitalize_first(&normalize_provider_name(provider));
    let model_part = model.replace(['.', '-', '/'], "_");

    format!("{}_{}", provider_part, model_part)
}

/// Capitalize first character
fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().chain(chars).collect(),
    }
}

/// Normalize and deduplicate entries
fn normalize_and_dedupe(entries: Vec<LlmEntry>) -> Vec<LlmEntry> {
    let mut seen = HashSet::new();
    let mut normalized = Vec::new();

    for entry in entries {
        let normalized_entry = LlmEntry {
            provider: normalize_provider_name(&entry.provider),
            model: entry.model.clone(),
        };

        let key = (
            normalized_entry.provider.clone(),
            normalized_entry.model.clone(),
        );

        if seen.insert(key) {
            normalized.push(normalized_entry);
        }
    }

    normalized.sort_by(|a, b| {
        a.provider
            .cmp(&b.provider)
            .then(a.model.cmp(&b.model))
    });

    normalized
}

/// Fetch from all providers concurrently
///
/// This function now uses a curated model registry as the primary source,
/// with optional OpenAI API fetching to supplement with latest models.
pub async fn fetch_all_providers() -> Result<Vec<LlmEntry>, ProviderError> {
    // Double-checked locking pattern
    if let Some(cached) = check_cache()? {
        return Ok(cached);
    }

    let _guard = acquire_fetch_lock().await;

    // Check cache again after acquiring lock
    if let Some(cached) = check_cache()? {
        return Ok(cached);
    }

    info!("Loading models from curated registry ({})", super::curated::LAST_UPDATED);

    // Start with curated models (always available, no API keys needed)
    let mut all_entries = super::curated::get_curated_models();

    // Optionally fetch from OpenAI API to supplement with latest models
    // This is best-effort - if it fails, we still have the curated list
    if std::env::var("OPENAI_API_KEY").is_ok() {
        match fetch_with_retry(fetch_openai_models, "openai").await {
            Ok(openai_models) => {
                info!("Fetched {} models from OpenAI API", openai_models.len());
                // Remove curated OpenAI models to avoid duplicates
                all_entries.retain(|e| e.provider != "openai");
                // Add fresh OpenAI models from API
                all_entries.extend(openai_models);
            }
            Err(e) => {
                warn!("Failed to fetch from OpenAI API, using curated OpenAI models: {}", e);
                // Curated OpenAI models already in all_entries
            }
        }
    } else {
        info!("OPENAI_API_KEY not set, using curated OpenAI models");
    }

    // Note: We no longer fetch from Anthropic, Hugging Face, or Gemini APIs
    // because they either don't have public APIs or the curated list is more reliable

    let normalized = normalize_and_dedupe(all_entries);

    info!("Total models in registry: {} (from {} providers)",
        normalized.len(),
        super::curated::PROVIDER_COUNT
    );

    // Write to cache
    if let Err(e) = write_cache(&normalized) {
        warn!("Failed to write cache: {}", e);
    }

    Ok(normalized)
}

/// Generate provider list in the specified format
pub async fn generate_provider_list(
    format: Option<ProviderListFormat>,
) -> Result<String, ProviderError> {
    let entries = fetch_all_providers().await?;

    match format.unwrap_or_default() {
        ProviderListFormat::StringLiterals => {
            let literals: Vec<String> = entries.iter().map(|e| e.identifier()).collect();
            Ok(serde_json::to_string_pretty(&literals)?)
        }
        ProviderListFormat::RustEnum => {
            let mut enum_str = String::from("pub enum ModelProvider {\n");
            for entry in entries {
                let variant = to_enum_variant(&entry.provider, &entry.model);
                enum_str.push_str(&format!("    {},\n", variant));
            }
            enum_str.push_str("}\n");
            Ok(enum_str)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_provider_name_lowercase() {
        assert_eq!(normalize_provider_name("OpenAI"), "openai");
    }

    #[test]
    fn normalize_provider_name_spaces() {
        assert_eq!(normalize_provider_name("Hugging Face"), "hugging_face");
    }

    #[test]
    fn normalize_provider_name_hyphens() {
        assert_eq!(normalize_provider_name("my-provider"), "my_provider");
    }

    #[test]
    fn to_enum_variant_formats_correctly() {
        let variant = to_enum_variant("openai", "gpt-5.2");
        assert_eq!(variant, "Openai_gpt_5_2");
    }

    #[test]
    fn to_enum_variant_handles_slashes() {
        let variant = to_enum_variant("huggingface", "meta-llama/Llama-3.3-70B");
        assert_eq!(variant, "Huggingface_meta_llama_Llama_3_3_70B");
    }

    #[test]
    fn capitalize_first_works() {
        assert_eq!(capitalize_first("hello"), "Hello");
        assert_eq!(capitalize_first(""), "");
        assert_eq!(capitalize_first("a"), "A");
    }

    #[test]
    fn normalize_and_dedupe_removes_duplicates() {
        let entries = vec![
            LlmEntry::new("OpenAI", "gpt-4"),
            LlmEntry::new("openai", "gpt-4"),
            LlmEntry::new("Anthropic", "claude"),
        ];

        let result = normalize_and_dedupe(entries);

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].provider, "anthropic");
        assert_eq!(result[1].provider, "openai");
    }

    #[test]
    fn normalize_and_dedupe_sorts() {
        let entries = vec![
            LlmEntry::new("openai", "gpt-4"),
            LlmEntry::new("anthropic", "claude"),
            LlmEntry::new("anthropic", "opus"),
        ];

        let result = normalize_and_dedupe(entries);

        assert_eq!(result[0].provider, "anthropic");
        assert_eq!(result[0].model, "claude");
        assert_eq!(result[1].provider, "anthropic");
        assert_eq!(result[1].model, "opus");
        assert_eq!(result[2].provider, "openai");
    }

    #[tokio::test]
    async fn fetch_anthropic_models_returns_hardcoded() {
        let result = fetch_anthropic_models().await.unwrap();
        assert_eq!(result.len(), 3);
        assert!(result.iter().all(|e| e.provider == "anthropic"));
    }

    #[tokio::test]
    async fn fetch_gemini_models_returns_hardcoded() {
        let result = fetch_gemini_models().await.unwrap();
        assert_eq!(result.len(), 2);
        assert!(result.iter().all(|e| e.provider == "gemini"));
    }

    #[tokio::test]
    async fn generate_provider_list_string_literals() {
        // This will use hardcoded models since we don't have API keys in tests
        let result = generate_provider_list(Some(ProviderListFormat::StringLiterals))
            .await
            .unwrap();

        assert!(result.contains("anthropic/claude"));
        assert!(result.contains("gemini/gemini"));
        assert!(result.starts_with('['));
        assert!(result.ends_with(']'));
    }

    #[tokio::test]
    async fn generate_provider_list_rust_enum() {
        let result = generate_provider_list(Some(ProviderListFormat::RustEnum))
            .await
            .unwrap();

        assert!(result.contains("pub enum ModelProvider"));
        assert!(result.contains("Anthropic_"));
        assert!(result.contains("Gemini_"));
    }

    #[tokio::test]
    #[ignore] // Run with: cargo test show_provider_list_output -- --ignored --nocapture
    async fn show_provider_list_output() {
        println!("\n=== STRING LITERALS FORMAT ===");
        let json_result = generate_provider_list(Some(ProviderListFormat::StringLiterals))
            .await
            .unwrap();
        println!("{}", json_result);

        println!("\n=== RUST ENUM FORMAT ===");
        let enum_result = generate_provider_list(Some(ProviderListFormat::RustEnum))
            .await
            .unwrap();
        println!("{}", enum_result);
    }
}
