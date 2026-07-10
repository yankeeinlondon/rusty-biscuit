//! OpenRouter-specific metadata parser.
//!
//! Parses the rich model metadata from OpenRouter's `/v1/models` endpoint
//! into the unified `ProviderModelMetadata` type. OpenRouter provides 371+
//! models with per-model pricing, architecture, context windows, supported
//! parameters, and default generation parameters.

use unchained_ai::models::model_default_parameters::ModelDefaultParameters;
use unchained_ai::models::model_metadata::{Modality, ModelModalities, ProviderModelMetadata};
use unchained_ai::models::model_pricing::ModelPricing;

/// Parse an OpenRouter model JSON object into `ProviderModelMetadata`.
///
/// The input `value` is the full JSON object for a single model from
/// OpenRouter's `/v1/models` response `data` array.
///
/// Fields that OpenRouter doesn't provide (`family`, `capabilities`,
/// `release_date`) are left as defaults so an external catalog can fill them
/// during the merge phase.
pub fn parse_openrouter_model(value: &serde_json::Value) -> ProviderModelMetadata {
    let display_name = str_field(value, "name");
    let description = str_field(value, "description");
    let context_window = u32_field(value, "context_length");
    let knowledge_cutoff = value
        .get("knowledge_cutoff")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(String::from);
    let created = u32_field(value, "created");

    let pricing = value.get("pricing").and_then(parse_pricing);
    let modalities = value.get("architecture").and_then(parse_modalities);
    let supported_parameters = value
        .get("supported_parameters")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        });
    let default_parameters = value
        .get("default_parameters")
        .and_then(parse_default_parameters);
    let max_output_tokens = value
        .get("top_provider")
        .and_then(|tp| u32_field(tp, "max_completion_tokens"));

    ProviderModelMetadata {
        display_name,
        family: None,
        context_window,
        max_output_tokens,
        modalities,
        capabilities: vec![],
        description,
        pricing,
        supported_parameters,
        default_parameters,
        knowledge_cutoff,
        created,
        release_date: None,
    }
}

fn str_field(value: &serde_json::Value, key: &str) -> Option<String> {
    value.get(key).and_then(|v| v.as_str()).map(String::from)
}

fn u32_field(value: &serde_json::Value, key: &str) -> Option<u32> {
    value.get(key).and_then(|v| v.as_u64()).map(|v| v as u32)
}

/// Parse the OpenRouter `pricing` object.
///
/// OpenRouter uses field names (`prompt`, `completion`, `web_search`,
/// `input_cache_read`) that differ from the serde rename attributes on
/// `ModelPricing` (`request`, `cache_read`), so we parse manually rather
/// than deserializing.
fn parse_pricing(value: &serde_json::Value) -> Option<ModelPricing> {
    if value.is_null() {
        return None;
    }

    let prompt_per_token = value.get("prompt").and_then(json_to_f64);
    let completion_per_token = value.get("completion").and_then(json_to_f64);
    let web_search_per_request = value.get("web_search").and_then(json_to_f64);
    let input_cache_read_per_token = value.get("input_cache_read").and_then(json_to_f64);

    if prompt_per_token.is_none()
        && completion_per_token.is_none()
        && web_search_per_request.is_none()
        && input_cache_read_per_token.is_none()
    {
        return None;
    }

    Some(ModelPricing {
        prompt_per_token,
        completion_per_token,
        web_search_per_request,
        input_cache_read_per_token,
    })
}

/// Parse the OpenRouter `architecture` object into `ModelModalities`.
fn parse_modalities(value: &serde_json::Value) -> Option<ModelModalities> {
    let input = value
        .get("input_modalities")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().and_then(|s| s.parse().ok()))
                .collect::<Vec<Modality>>()
        })
        .unwrap_or_default();

    let output = value
        .get("output_modalities")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().and_then(|s| s.parse().ok()))
                .collect::<Vec<Modality>>()
        })
        .unwrap_or_default();

    if input.is_empty() && output.is_empty() {
        return None;
    }

    Some(ModelModalities { input, output })
}

/// Parse the OpenRouter `default_parameters` object.
///
/// OpenRouter fields map directly to `ModelDefaultParameters` fields.
/// The `repetition_penalty` field is present in OpenRouter but not in our
/// struct, so it is silently ignored.
fn parse_default_parameters(value: &serde_json::Value) -> Option<ModelDefaultParameters> {
    if value.is_null() {
        return None;
    }

    let temperature = value.get("temperature").and_then(json_to_f32);
    let top_p = value.get("top_p").and_then(json_to_f32);
    let top_k = value
        .get("top_k")
        .and_then(|v| v.as_u64())
        .map(|v| v as u32);
    let frequency_penalty = value.get("frequency_penalty").and_then(json_to_f32);
    let presence_penalty = value.get("presence_penalty").and_then(json_to_f32);

    if temperature.is_none()
        && top_p.is_none()
        && top_k.is_none()
        && frequency_penalty.is_none()
        && presence_penalty.is_none()
    {
        return None;
    }

    Some(ModelDefaultParameters {
        temperature,
        top_p,
        top_k,
        frequency_penalty,
        presence_penalty,
    })
}

/// Convert a JSON value (string or number) to `f64`.
fn json_to_f64(value: &serde_json::Value) -> Option<f64> {
    match value {
        serde_json::Value::String(s) => s.parse().ok(),
        serde_json::Value::Number(n) => n.as_f64(),
        _ => None,
    }
}

/// Convert a JSON value (string or number) to `f32`.
fn json_to_f32(value: &serde_json::Value) -> Option<f32> {
    json_to_f64(value).map(|v| v as f32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use unchained_ai::models::model_metadata::Modality;

    fn grok_fixture() -> serde_json::Value {
        serde_json::json!({
            "id": "x-ai/grok-4.3",
            "name": "xAI: Grok 4.3",
            "description": "Grok 4.3 is a reasoning model from xAI.",
            "context_length": 1000000,
            "pricing": {
                "prompt": "0.00000125",
                "completion": "0.0000025",
                "web_search": "0.005",
                "input_cache_read": "0.0000002"
            },
            "architecture": {
                "modality": "text+image->text",
                "input_modalities": ["text", "image"],
                "output_modalities": ["text"],
                "tokenizer": "Grok",
                "instruct_type": null
            },
            "supported_parameters": [
                "frequency_penalty", "include_reasoning", "max_tokens",
                "temperature", "tools", "top_p"
            ],
            "default_parameters": {
                "temperature": 0.7,
                "top_p": 0.9,
                "top_k": null,
                "frequency_penalty": null,
                "presence_penalty": null,
                "repetition_penalty": null
            },
            "top_provider": {
                "context_length": 1000000,
                "max_completion_tokens": 32768,
                "is_moderated": false
            },
            "canonical_slug": "x-ai/grok-4.3-20260430",
            "created": 1777591821,
            "knowledge_cutoff": "2025-04-30"
        })
    }

    #[test]
    fn test_parse_display_name() {
        let meta = parse_openrouter_model(&grok_fixture());
        assert_eq!(meta.display_name.as_deref(), Some("xAI: Grok 4.3"));
    }

    #[test]
    fn test_parse_description() {
        let meta = parse_openrouter_model(&grok_fixture());
        assert_eq!(
            meta.description.as_deref(),
            Some("Grok 4.3 is a reasoning model from xAI.")
        );
    }

    #[test]
    fn test_parse_context_window() {
        let meta = parse_openrouter_model(&grok_fixture());
        assert_eq!(meta.context_window, Some(1_000_000));
    }

    #[test]
    fn test_parse_created() {
        let meta = parse_openrouter_model(&grok_fixture());
        assert_eq!(meta.created, Some(1_777_591_821));
    }

    #[test]
    fn test_parse_knowledge_cutoff() {
        let meta = parse_openrouter_model(&grok_fixture());
        assert_eq!(meta.knowledge_cutoff.as_deref(), Some("2025-04-30"));
    }

    #[test]
    fn test_parse_max_output_tokens_from_top_provider() {
        let meta = parse_openrouter_model(&grok_fixture());
        assert_eq!(meta.max_output_tokens, Some(32768));
    }

    #[test]
    fn test_parse_pricing() {
        let meta = parse_openrouter_model(&grok_fixture());
        let pricing = meta.pricing.as_ref().expect("pricing should be present");
        assert_eq!(pricing.prompt_per_token, Some(0.00000125));
        assert_eq!(pricing.completion_per_token, Some(0.0000025));
        assert_eq!(pricing.web_search_per_request, Some(0.005));
        assert_eq!(pricing.input_cache_read_per_token, Some(0.0000002));
    }

    #[test]
    fn test_parse_modalities() {
        let meta = parse_openrouter_model(&grok_fixture());
        let mods = meta
            .modalities
            .as_ref()
            .expect("modalities should be present");
        assert_eq!(mods.input, vec![Modality::Text, Modality::Image]);
        assert_eq!(mods.output, vec![Modality::Text]);
    }

    #[test]
    fn test_parse_supported_parameters() {
        let meta = parse_openrouter_model(&grok_fixture());
        let params = meta
            .supported_parameters
            .as_ref()
            .expect("supported_parameters should be present");
        assert!(params.contains(&"temperature".to_string()));
        assert!(params.contains(&"tools".to_string()));
        assert!(params.contains(&"max_tokens".to_string()));
        assert_eq!(params.len(), 6);
    }

    #[test]
    fn test_parse_default_parameters() {
        let meta = parse_openrouter_model(&grok_fixture());
        let dp = meta
            .default_parameters
            .as_ref()
            .expect("default_parameters should be present");
        assert_eq!(dp.temperature, Some(0.7));
        assert_eq!(dp.top_p, Some(0.9));
        assert_eq!(dp.top_k, None);
        assert_eq!(dp.frequency_penalty, None);
        assert_eq!(dp.presence_penalty, None);
    }

    #[test]
    fn test_family_and_capabilities_default() {
        let meta = parse_openrouter_model(&grok_fixture());
        assert!(meta.family.is_none());
        assert!(meta.capabilities.is_empty());
    }

    #[test]
    fn test_parse_minimal_model() {
        let value = serde_json::json!({
            "id": "test/model",
            "name": "Test Model"
        });
        let meta = parse_openrouter_model(&value);
        assert_eq!(meta.display_name.as_deref(), Some("Test Model"));
        assert!(meta.pricing.is_none());
        assert!(meta.modalities.is_none());
        assert!(meta.description.is_none());
        assert!(meta.context_window.is_none());
        assert!(meta.supported_parameters.is_none());
        assert!(meta.default_parameters.is_none());
        assert!(meta.knowledge_cutoff.is_none());
        assert!(meta.created.is_none());
        assert!(meta.max_output_tokens.is_none());
    }

    #[test]
    fn test_parse_numeric_pricing() {
        let value = serde_json::json!({
            "id": "test/model",
            "name": "Test",
            "pricing": {
                "prompt": 0.000005,
                "completion": 0.000015
            }
        });
        let meta = parse_openrouter_model(&value);
        let pricing = meta.pricing.as_ref().unwrap();
        assert_eq!(pricing.prompt_per_token, Some(0.000005));
        assert_eq!(pricing.completion_per_token, Some(0.000015));
        assert_eq!(pricing.web_search_per_request, None);
        assert_eq!(pricing.input_cache_read_per_token, None);
    }

    #[test]
    fn test_parse_empty_knowledge_cutoff() {
        let value = serde_json::json!({
            "id": "test/model",
            "name": "Test",
            "knowledge_cutoff": ""
        });
        let meta = parse_openrouter_model(&value);
        assert!(meta.knowledge_cutoff.is_none());
    }

    #[test]
    fn test_parse_null_knowledge_cutoff() {
        let value = serde_json::json!({
            "id": "test/model",
            "name": "Test",
            "knowledge_cutoff": null
        });
        let meta = parse_openrouter_model(&value);
        assert!(meta.knowledge_cutoff.is_none());
    }

    #[test]
    fn test_parse_null_top_provider_max_completion_tokens() {
        let value = serde_json::json!({
            "id": "test/model",
            "name": "Test",
            "top_provider": {
                "context_length": 128000,
                "max_completion_tokens": null,
                "is_moderated": false
            }
        });
        let meta = parse_openrouter_model(&value);
        assert!(meta.max_output_tokens.is_none());
    }

    #[test]
    fn test_parse_no_top_provider() {
        let value = serde_json::json!({"id": "test", "name": "Test"});
        let meta = parse_openrouter_model(&value);
        assert!(meta.max_output_tokens.is_none());
    }

    #[test]
    fn test_parse_all_null_pricing() {
        let value = serde_json::json!({
            "id": "test/model",
            "name": "Test",
            "pricing": {
                "prompt": null,
                "completion": null,
                "web_search": null,
                "input_cache_read": null
            }
        });
        let meta = parse_openrouter_model(&value);
        assert!(meta.pricing.is_none());
    }

    #[test]
    fn test_parse_all_null_default_parameters() {
        let value = serde_json::json!({
            "id": "test/model",
            "name": "Test",
            "default_parameters": {
                "temperature": null,
                "top_p": null,
                "top_k": null,
                "frequency_penalty": null,
                "presence_penalty": null
            }
        });
        let meta = parse_openrouter_model(&value);
        assert!(meta.default_parameters.is_none());
    }

    #[test]
    fn test_parse_empty_modalities_arrays() {
        let value = serde_json::json!({
            "id": "test/model",
            "name": "Test",
            "architecture": {
                "input_modalities": [],
                "output_modalities": []
            }
        });
        let meta = parse_openrouter_model(&value);
        assert!(meta.modalities.is_none());
    }

    #[test]
    fn test_parse_no_architecture() {
        let value = serde_json::json!({"id": "test", "name": "Test"});
        let meta = parse_openrouter_model(&value);
        assert!(meta.modalities.is_none());
    }

    #[test]
    fn test_parse_free_model_pricing() {
        let value = serde_json::json!({
            "id": "test/free-model",
            "name": "Free Model",
            "pricing": {
                "prompt": "0",
                "completion": "0"
            }
        });
        let meta = parse_openrouter_model(&value);
        let pricing = meta.pricing.as_ref().unwrap();
        assert_eq!(pricing.prompt_per_token, Some(0.0));
        assert_eq!(pricing.completion_per_token, Some(0.0));
    }
}
