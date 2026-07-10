//! Provider-specific metadata parsing dispatcher.
//!
//! Routes raw provider JSON to the appropriate parser based on provider type.
//! Only providers with rich native metadata (e.g., OpenRouter) get special
//! parsing; others return `None` and rely on source enrichment data.

pub mod openrouter;

use serde_json::Value;
use unchained_ai::models::model_metadata::ProviderModelMetadata;
use unchained_ai::rigging::providers::Provider;

/// Parse provider-native metadata from a raw JSON model entry.
///
/// Returns `Some(ProviderModelMetadata)` when the provider has a dedicated
/// parser, or `None` for providers that return minimal metadata and should
/// rely on source enrichment data instead.
pub fn parse_provider_metadata(provider: Provider, value: &Value) -> Option<ProviderModelMetadata> {
    match provider {
        Provider::OpenRouter => Some(openrouter::parse_openrouter_model(value)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dispatch_openrouter() {
        let value = serde_json::json!({
            "id": "openai/gpt-4o",
            "name": "OpenAI: GPT-4o",
            "context_length": 128000,
            "pricing": { "prompt": "0.000005", "completion": "0.000015" },
            "architecture": {
                "modality": "text+image->text",
                "input_modalities": ["text", "image"],
                "output_modalities": ["text"]
            }
        });
        let result = parse_provider_metadata(Provider::OpenRouter, &value);
        assert!(result.is_some());
        assert_eq!(
            result.unwrap().display_name.as_deref(),
            Some("OpenAI: GPT-4o")
        );
    }

    #[test]
    fn test_dispatch_unknown_provider_returns_none() {
        let value = serde_json::json!({"id": "gpt-4o"});
        assert!(parse_provider_metadata(Provider::OpenAi, &value).is_none());
        assert!(parse_provider_metadata(Provider::Anthropic, &value).is_none());
        assert!(parse_provider_metadata(Provider::Gemini, &value).is_none());
    }
}
