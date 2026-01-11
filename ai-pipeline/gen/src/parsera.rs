//! Parsera LLM Specs API client.
//!
//! Fetches model metadata from the Parsera API at build time to enrich
//! generated model enums with context window, modalities, and capabilities.

use std::collections::HashMap;
use std::time::Duration;

use serde::Deserialize;
use thiserror::Error;
use tracing::{debug, info, warn};

/// Parsera API endpoint for LLM specifications.
const PARSERA_API_URL: &str = "https://api.parsera.org/v1/llm-specs";

/// Timeout for API requests.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// Error types for Parsera API operations.
#[derive(Debug, Error)]
pub enum ParseraError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Failed to parse JSON response: {0}")]
    Json(#[from] serde_json::Error),

    #[error("API returned error status: {status}")]
    Api { status: u16 },
}

/// Input/output modalities from Parsera API.
#[derive(Debug, Clone, Deserialize)]
pub struct ParseraModalities {
    /// Input modalities (e.g., ["text", "image"])
    #[serde(default)]
    pub input: Vec<String>,
    /// Output modalities (e.g., ["text"])
    #[serde(default)]
    pub output: Vec<String>,
}

/// Model specification from Parsera API.
#[derive(Debug, Clone, Deserialize)]
pub struct ParseraModel {
    /// Model ID (e.g., "gpt-4o-mini")
    pub id: String,

    /// Human-readable name (e.g., "GPT-4o mini")
    pub name: String,

    /// Provider name (e.g., "openai", "anthropic")
    pub provider: String,

    /// Model family (e.g., "gpt-4o-mini")
    #[serde(default)]
    pub family: Option<String>,

    /// Context window size in tokens
    #[serde(default)]
    pub context_window: Option<u32>,

    /// Maximum output tokens
    #[serde(default)]
    pub max_output_tokens: Option<u32>,

    /// Input/output modalities
    #[serde(default)]
    pub modalities: Option<ParseraModalities>,

    /// Capabilities (e.g., ["function_calling", "structured_output"])
    #[serde(default)]
    pub capabilities: Option<Vec<String>>,
}

/// Fetches all LLM specs from the Parsera API.
///
/// Returns a vector of model specifications. On failure, logs a warning
/// and returns an empty vector for graceful degradation.
///
/// ## Errors
///
/// Returns `ParseraError` on HTTP failures, timeouts, or JSON parse errors.
pub async fn fetch_parsera_specs() -> Result<Vec<ParseraModel>, ParseraError> {
    let client = reqwest::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .build()?;

    info!("Fetching model specs from Parsera API");

    let response = client.get(PARSERA_API_URL).send().await?;

    let status = response.status();
    if !status.is_success() {
        return Err(ParseraError::Api {
            status: status.as_u16(),
        });
    }

    let models: Vec<ParseraModel> = response.json().await?;
    info!("Fetched {} model specs from Parsera", models.len());

    Ok(models)
}

/// Fetches Parsera specs with one retry on failure.
///
/// On first failure, waits 2 seconds and retries once. If both attempts
/// fail, returns an empty HashMap for graceful degradation.
pub async fn fetch_parsera_specs_with_retry() -> HashMap<String, ParseraModel> {
    match fetch_parsera_specs().await {
        Ok(models) => index_by_id(models),
        Err(e) => {
            warn!("First Parsera fetch failed: {e}, retrying in 2s...");
            tokio::time::sleep(Duration::from_secs(2)).await;

            match fetch_parsera_specs().await {
                Ok(models) => index_by_id(models),
                Err(e) => {
                    warn!("Parsera API unavailable after retry: {e}");
                    HashMap::new()
                }
            }
        }
    }
}

/// Indexes models by their ID for fast lookup.
///
/// Creates a HashMap mapping model ID to the full model specification.
pub fn index_by_id(models: Vec<ParseraModel>) -> HashMap<String, ParseraModel> {
    let mut index = HashMap::with_capacity(models.len());
    for model in models {
        debug!("Indexing model: {} ({})", model.id, model.provider);
        index.insert(model.id.clone(), model);
    }
    index
}

/// Attempts to find Parsera metadata for a given model ID.
///
/// Uses a multi-step matching strategy:
/// 1. Exact match on model ID
/// 2. Strip date suffix (e.g., "claude-3-5-haiku-20241022" -> "claude-3-5-haiku")
/// 3. Match via family field
pub fn find_parsera_metadata<'a>(
    model_id: &str,
    index: &'a HashMap<String, ParseraModel>,
) -> Option<&'a ParseraModel> {
    // 1. Exact match
    if let Some(model) = index.get(model_id) {
        return Some(model);
    }

    // 2. Strip date suffix (YYYYMMDD pattern at end)
    let stripped = strip_date_suffix(model_id);
    if stripped != model_id
        && let Some(model) = index.get(stripped) {
            debug!(
                "Matched {} via date-stripped ID: {}",
                model_id, stripped
            );
            return Some(model);
        }

    // 3. Match via family field
    for model in index.values() {
        if let Some(family) = &model.family
            && (family == model_id || family == stripped) {
                debug!("Matched {} via family: {}", model_id, family);
                return Some(model);
            }
    }

    None
}

/// Strips a date suffix (YYYYMMDD or -YYYYMMDD) from a model ID.
fn strip_date_suffix(model_id: &str) -> &str {
    // Pattern: ends with -YYYYMMDD (8 digits after hyphen)
    if model_id.len() > 9 {
        let suffix_start = model_id.len() - 9;
        if model_id.as_bytes()[suffix_start] == b'-' {
            let suffix = &model_id[suffix_start + 1..];
            if suffix.len() == 8 && suffix.chars().all(|c| c.is_ascii_digit()) {
                return &model_id[..suffix_start];
            }
        }
    }

    // Pattern: ends with @YYYYMMDD (some providers use @)
    if model_id.len() > 9 {
        let suffix_start = model_id.len() - 9;
        if model_id.as_bytes()[suffix_start] == b'@' {
            let suffix = &model_id[suffix_start + 1..];
            if suffix.len() == 8 && suffix.chars().all(|c| c.is_ascii_digit()) {
                return &model_id[..suffix_start];
            }
        }
    }

    model_id
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_date_suffix() {
        assert_eq!(
            strip_date_suffix("claude-3-5-haiku-20241022"),
            "claude-3-5-haiku"
        );
        assert_eq!(
            strip_date_suffix("gpt-4-turbo-2024-04-09"),
            "gpt-4-turbo-2024-04-09" // Not stripped - not YYYYMMDD format
        );
        assert_eq!(strip_date_suffix("gpt-4o"), "gpt-4o");
        assert_eq!(strip_date_suffix("model@20241022"), "model");
    }

    #[test]
    fn test_index_by_id() {
        let models = vec![
            ParseraModel {
                id: "gpt-4o".to_string(),
                name: "GPT-4o".to_string(),
                provider: "openai".to_string(),
                family: Some("gpt-4o".to_string()),
                context_window: Some(128000),
                max_output_tokens: Some(16384),
                modalities: None,
                capabilities: None,
            },
            ParseraModel {
                id: "claude-3-5-sonnet".to_string(),
                name: "Claude 3.5 Sonnet".to_string(),
                provider: "anthropic".to_string(),
                family: Some("claude-3-5".to_string()),
                context_window: Some(200000),
                max_output_tokens: Some(8192),
                modalities: None,
                capabilities: None,
            },
        ];

        let index = index_by_id(models);
        assert_eq!(index.len(), 2);
        assert!(index.contains_key("gpt-4o"));
        assert!(index.contains_key("claude-3-5-sonnet"));
    }

    #[test]
    fn test_find_parsera_metadata_exact_match() {
        let models = vec![ParseraModel {
            id: "gpt-4o".to_string(),
            name: "GPT-4o".to_string(),
            provider: "openai".to_string(),
            family: None,
            context_window: Some(128000),
            max_output_tokens: None,
            modalities: None,
            capabilities: None,
        }];

        let index = index_by_id(models);
        let result = find_parsera_metadata("gpt-4o", &index);
        assert!(result.is_some());
        assert_eq!(result.unwrap().name, "GPT-4o");
    }

    #[test]
    fn test_find_parsera_metadata_date_stripped() {
        let models = vec![ParseraModel {
            id: "claude-3-5-haiku".to_string(),
            name: "Claude 3.5 Haiku".to_string(),
            provider: "anthropic".to_string(),
            family: None,
            context_window: Some(200000),
            max_output_tokens: None,
            modalities: None,
            capabilities: None,
        }];

        let index = index_by_id(models);
        let result = find_parsera_metadata("claude-3-5-haiku-20241022", &index);
        assert!(result.is_some());
        assert_eq!(result.unwrap().name, "Claude 3.5 Haiku");
    }

    #[test]
    fn test_find_parsera_metadata_family_match() {
        let models = vec![ParseraModel {
            id: "claude-3-5-sonnet-latest".to_string(),
            name: "Claude 3.5 Sonnet".to_string(),
            provider: "anthropic".to_string(),
            family: Some("claude-3-5-sonnet".to_string()),
            context_window: Some(200000),
            max_output_tokens: None,
            modalities: None,
            capabilities: None,
        }];

        let index = index_by_id(models);
        let result = find_parsera_metadata("claude-3-5-sonnet", &index);
        assert!(result.is_some());
        assert_eq!(result.unwrap().name, "Claude 3.5 Sonnet");
    }

    #[test]
    fn test_find_parsera_metadata_not_found() {
        let index = HashMap::new();
        let result = find_parsera_metadata("unknown-model", &index);
        assert!(result.is_none());
    }
}
