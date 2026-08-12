//! models.dev catalog client and metadata mapping.

#![allow(dead_code)]

use std::collections::BTreeMap;
use std::time::Duration;

use serde::Deserialize;
use thiserror::Error;
use tracing::{info, warn};
use unchained_ai::models::identity::ModelIdentity;
use unchained_ai::models::model_capability::canonical_models_dev_capability;
use unchained_ai::models::model_metadata::{Modality, ModelModalities, ProviderModelMetadata};
use unchained_ai::models::model_pricing::ModelPricing;
use unchained_ai::rigging::providers::Provider;
use unchained_ai_gen::catalog::identity_key;

/// models.dev API endpoint for model metadata.
const MODELS_DEV_API_URL: &str = "https://models.dev/api.json";

/// Timeout for models.dev API requests.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// Provider-keyed response from models.dev.
pub type ModelsDevResponse = BTreeMap<String, ModelsDevProvider>;

/// Provider-keyed model index used by the generator.
pub type ModelsDevIndex = BTreeMap<String, BTreeMap<String, ModelsDevModel>>;

/// Minimum plausible provider count for the live models.dev response.
pub const MODELS_DEV_MIN_PROVIDERS: usize = 50;

/// Provider buckets required for roster-critical enrichment.
pub const ROSTER_CRITICAL_MODELS_DEV_PROVIDERS: &[&str] = &[
    "anthropic",
    "google",
    "moonshotai",
    "openai",
    "openrouter",
    "xai",
    "zai",
    "deepseek",
    "groq",
    "mistral",
];

struct Candidate<'a> {
    id: &'a str,
    model: &'a ModelsDevModel,
    identity: ModelIdentity,
}

/// Error types for models.dev operations.
#[derive(Debug, Error)]
pub enum ModelsDevError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("failed to parse JSON response: {0}")]
    Json(#[from] serde_json::Error),

    #[error("API returned error status: {status}")]
    Api { status: u16 },

    #[error("models.dev response is implausibly thin: {provider_count} providers, expected at least {minimum}")]
    ThinResponse {
        provider_count: usize,
        minimum: usize,
    },

    #[error("models.dev response is missing roster-critical provider bucket: {provider}")]
    MissingProvider { provider: &'static str },

    #[error("models.dev roster-critical provider bucket is empty: {provider}")]
    EmptyProvider { provider: &'static str },

    #[error("models.dev release_date must be YYYY-MM-DD, got {value:?}")]
    InvalidReleaseDate { value: String },
}

/// Provider entry from models.dev.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ModelsDevProvider {
    /// Models keyed by provider-local model id.
    #[serde(default)]
    pub models: BTreeMap<String, ModelsDevModel>,
}

/// Model entry from models.dev.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ModelsDevModel {
    /// Human-readable model name.
    #[serde(default)]
    pub name: Option<String>,
    /// Model family.
    #[serde(default)]
    pub family: Option<String>,
    /// Token limits.
    #[serde(default)]
    pub limit: Option<ModelsDevLimit>,
    /// Pricing, expressed by models.dev per million tokens.
    #[serde(default)]
    pub cost: Option<ModelsDevCost>,
    /// Input/output modality strings.
    #[serde(default)]
    pub modalities: Option<ModelsDevModalities>,
    /// Knowledge cutoff date.
    #[serde(default)]
    pub knowledge: Option<String>,
    /// Release date.
    #[serde(default)]
    pub release_date: Option<String>,
    /// Whether the model supports tool/function calling.
    #[serde(default)]
    pub tool_call: bool,
    /// Whether the model supports structured output.
    #[serde(default)]
    pub structured_output: bool,
    /// Whether the model supports reasoning controls.
    #[serde(default)]
    pub reasoning: bool,
    /// Whether the model accepts file attachments.
    #[serde(default)]
    pub attachment: bool,
}

/// Token limits from models.dev.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ModelsDevLimit {
    /// Context window size in tokens.
    #[serde(default)]
    pub context: Option<u32>,
    /// Maximum output tokens.
    #[serde(default)]
    pub output: Option<u32>,
}

/// Pricing from models.dev, in USD per million tokens.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ModelsDevCost {
    /// Input token cost per million tokens.
    #[serde(default, deserialize_with = "deserialize_optional_string_or_f64")]
    pub input: Option<f64>,
    /// Output token cost per million tokens.
    #[serde(default, deserialize_with = "deserialize_optional_string_or_f64")]
    pub output: Option<f64>,
    /// Cached input read cost per million tokens.
    #[serde(default, deserialize_with = "deserialize_optional_string_or_f64")]
    pub cache_read: Option<f64>,
}

/// Modalities from models.dev.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ModelsDevModalities {
    /// Input modalities.
    #[serde(default)]
    pub input: Vec<String>,
    /// Output modalities.
    #[serde(default)]
    pub output: Vec<String>,
}

/// Maps a generated provider to the models.dev provider key.
#[must_use]
pub fn models_dev_provider_key(provider: Provider) -> Option<&'static str> {
    match provider {
        Provider::Anthropic => Some("anthropic"),
        Provider::Deepseek => Some("deepseek"),
        Provider::Gemini => Some("google"),
        Provider::Groq => Some("groq"),
        Provider::HuggingFace => Some("huggingface"),
        Provider::Mistral => Some("mistral"),
        Provider::MoonshotAi => Some("moonshotai"),
        Provider::Ollama => None,
        Provider::OpenAi => Some("openai"),
        Provider::OpenRouter => Some("openrouter"),
        Provider::Xai => Some("xai"),
        Provider::Zai => Some("zai"),
        Provider::ZenMux => None,
    }
}

/// Deserializes a models.dev JSON document into the generator index.
pub fn parse_models_dev_response_str(input: &str) -> Result<ModelsDevIndex, ModelsDevError> {
    let response: ModelsDevResponse = serde_json::from_str(input)?;
    Ok(response
        .into_iter()
        .map(|(provider_key, provider)| (provider_key, provider.models))
        .collect())
}

/// Validates the live models.dev response against the anti-sunset guardrails.
pub fn validate_models_dev_index(index: &ModelsDevIndex) -> Result<(), ModelsDevError> {
    if index.len() < MODELS_DEV_MIN_PROVIDERS {
        return Err(ModelsDevError::ThinResponse {
            provider_count: index.len(),
            minimum: MODELS_DEV_MIN_PROVIDERS,
        });
    }

    validate_roster_critical_models_dev_providers(index)
}

/// Validates roster-critical provider buckets without applying the global floor.
pub fn validate_roster_critical_models_dev_providers(
    index: &ModelsDevIndex,
) -> Result<(), ModelsDevError> {
    for provider in ROSTER_CRITICAL_MODELS_DEV_PROVIDERS {
        let Some(models) = index.get(*provider) else {
            return Err(ModelsDevError::MissingProvider {
                provider,
            });
        };
        if models.is_empty() {
            return Err(ModelsDevError::EmptyProvider {
                provider,
            });
        }
    }
    Ok(())
}

/// Converts a models.dev model into runtime metadata.
///
/// ## Errors
///
/// Returns an error when a non-empty `release_date` is not an exact
/// `YYYY-MM-DD` date.
pub fn models_dev_to_metadata(
    model: &ModelsDevModel,
) -> Result<ProviderModelMetadata, ModelsDevError> {
    Ok(ProviderModelMetadata {
        display_name: model.name.clone(),
        family: model.family.clone(),
        context_window: model.limit.as_ref().and_then(|limit| limit.context),
        max_output_tokens: model.limit.as_ref().and_then(|limit| limit.output),
        modalities: model.modalities.as_ref().map(models_dev_modalities),
        capabilities: models_dev_capabilities(model),
        pricing: model.cost.as_ref().and_then(models_dev_pricing),
        knowledge_cutoff: model.knowledge.clone(),
        release_date: validated_release_date(model.release_date.as_deref())?,
        ..Default::default()
    })
}

/// Finds the models.dev model row for a generated model id within its provider bucket.
pub fn find_models_dev_metadata<'a>(
    model_id: &str,
    provider: Provider,
    bucket: &'a BTreeMap<String, ModelsDevModel>,
) -> Option<&'a ModelsDevModel> {
    if let Some(model) = bucket.get(model_id) {
        return Some(model);
    }

    let provider_key = models_dev_provider_key(provider)?;
    let query_wire_id = format!("{provider_key}/{model_id}");
    let query_identity = ModelIdentity::parse(&query_wire_id);
    let Some(query_key) = identity_key(&query_identity) else {
        warn!("models.dev no-match for {:?}/{model_id}: identity-less id", provider);
        return None;
    };

    let candidates: Vec<Candidate<'a>> = bucket
        .iter()
        .filter_map(|(candidate_id, model)| {
            let candidate_wire_id = format!("{provider_key}/{candidate_id}");
            let identity = ModelIdentity::parse(&candidate_wire_id);
            if identity_key(&identity).as_deref() == Some(query_key.as_str()) {
                Some(Candidate {
                    id: candidate_id,
                    model,
                    identity,
                })
            } else {
                None
            }
        })
        .collect();

    select_identity_candidate(model_id, provider, &query_identity, &candidates)
}

/// Fetches models.dev with one retry on failure.
pub async fn fetch_models_dev_with_retry() -> Result<ModelsDevIndex, ModelsDevError> {
    match fetch_models_dev().await {
        Ok(index) => Ok(index),
        Err(e) => {
            warn!("First models.dev fetch failed: {e}, retrying in 2s...");
            tokio::time::sleep(Duration::from_secs(2)).await;
            fetch_models_dev().await
        }
    }
}

async fn fetch_models_dev() -> Result<ModelsDevIndex, ModelsDevError> {
    let client = reqwest::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .build()?;

    info!("Fetching model specs from models.dev");

    let response = client.get(MODELS_DEV_API_URL).send().await?;
    let status = response.status();
    if !status.is_success() {
        return Err(ModelsDevError::Api {
            status: status.as_u16(),
        });
    }

    let body = response.text().await?;
    parse_models_dev_response_str(&body)
}

fn select_identity_candidate<'a>(
    model_id: &str,
    provider: Provider,
    query_identity: &ModelIdentity,
    candidates: &[Candidate<'a>],
) -> Option<&'a ModelsDevModel> {
    if candidates.is_empty() {
        warn!("models.dev no-match for {:?}/{model_id}", provider);
        return None;
    }

    let exact_date_pin: Vec<_> = candidates
        .iter()
        .filter(|candidate| candidate.identity.date_pin == query_identity.date_pin)
        .collect();
    if exact_date_pin.len() == 1 {
        return Some(exact_date_pin[0].model);
    }

    let unpinned: Vec<_> = candidates
        .iter()
        .filter(|candidate| candidate.identity.date_pin.is_empty())
        .collect();
    if unpinned.len() == 1 {
        return Some(unpinned[0].model);
    }

    let candidate_ids = candidates
        .iter()
        .map(|candidate| candidate.id)
        .collect::<Vec<_>>()
        .join(", ");
    warn!(
        "models.dev ambiguous identity match for {:?}/{model_id}: {candidate_ids}",
        provider
    );
    None
}

fn models_dev_modalities(modalities: &ModelsDevModalities) -> ModelModalities {
    ModelModalities {
        input: modalities
            .input
            .iter()
            .filter_map(|modality| modality.parse::<Modality>().ok())
            .collect(),
        output: modalities
            .output
            .iter()
            .filter_map(|modality| modality.parse::<Modality>().ok())
            .collect(),
    }
}

fn models_dev_capabilities(model: &ModelsDevModel) -> Vec<String> {
    [
        ("tool_call", model.tool_call),
        ("structured_output", model.structured_output),
        ("reasoning", model.reasoning),
        ("attachment", model.attachment),
    ]
    .into_iter()
    .filter(|(_, enabled)| *enabled)
    .filter_map(|(field, _)| canonical_models_dev_capability(field))
    .map(str::to_string)
    .collect()
}

fn models_dev_pricing(cost: &ModelsDevCost) -> Option<ModelPricing> {
    let pricing = ModelPricing {
        prompt_per_token: cost.input.map(per_million_to_per_token),
        completion_per_token: cost.output.map(per_million_to_per_token),
        web_search_per_request: None,
        input_cache_read_per_token: cost.cache_read.map(per_million_to_per_token),
    };

    if pricing.prompt_per_token.is_some()
        || pricing.completion_per_token.is_some()
        || pricing.input_cache_read_per_token.is_some()
    {
        Some(pricing)
    } else {
        None
    }
}

fn per_million_to_per_token(value: f64) -> f64 {
    value / 1_000_000.0
}

fn validated_release_date(raw: Option<&str>) -> Result<Option<String>, ModelsDevError> {
    let Some(value) = raw else {
        return Ok(None);
    };
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    if is_yyyy_mm_dd_date(trimmed) {
        return Ok(Some(trimmed.to_string()));
    }
    Err(ModelsDevError::InvalidReleaseDate {
        value: value.to_string(),
    })
}

fn is_yyyy_mm_dd_date(value: &str) -> bool {
    let bytes = value.as_bytes();
    value.len() == 10
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes
            .iter()
            .enumerate()
            .all(|(index, byte)| index == 4 || index == 7 || byte.is_ascii_digit())
        && chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d").is_ok()
}

fn deserialize_optional_string_or_f64<'de, D>(deserializer: D) -> Result<Option<f64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::{self, Visitor};
    use std::fmt;

    struct StringOrF64Visitor;

    impl<'de> Visitor<'de> for StringOrF64Visitor {
        type Value = Option<f64>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a string, a number, or null")
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(None)
        }

        fn visit_unit<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(None)
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            if v.is_empty() {
                return Ok(None);
            }
            v.parse::<f64>()
                .map(Some)
                .map_err(|_| de::Error::custom(format!("invalid float string: '{v}'")))
        }

        fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(Some(v))
        }

        fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(Some(v as f64))
        }

        fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(Some(v as f64))
        }
    }

    deserializer.deserialize_any(StringOrF64Visitor)
}

#[cfg(test)]
mod tests {
    use super::*;
    use unchained_ai::models::model_capability::{
        CAPABILITY_FILE_INPUT, CAPABILITY_FUNCTION_CALLING, CAPABILITY_REASONING,
        CAPABILITY_STRUCTURED_OUTPUT,
    };

    const FIXTURE: &str = include_str!("../tests/fixtures/models-dev.json");

    fn fixture_index() -> ModelsDevIndex {
        parse_models_dev_response_str(FIXTURE).expect("fixture should deserialize")
    }

    fn fixture_model(provider: &str, model_id: &str) -> ModelsDevModel {
        fixture_index()
            .get(provider)
            .and_then(|models| models.get(model_id))
            .cloned()
            .expect("fixture model should exist")
    }

    fn fixture_bucket(provider: &str) -> BTreeMap<String, ModelsDevModel> {
        fixture_index()
            .get(provider)
            .cloned()
            .expect("fixture provider should exist")
    }

    fn minimal_model(name: &str) -> ModelsDevModel {
        ModelsDevModel {
            name: Some(name.to_string()),
            ..Default::default()
        }
    }

    #[test]
    fn test_provider_key_mapping_all_providers() {
        assert_eq!(models_dev_provider_key(Provider::Anthropic), Some("anthropic"));
        assert_eq!(models_dev_provider_key(Provider::Deepseek), Some("deepseek"));
        assert_eq!(models_dev_provider_key(Provider::Gemini), Some("google"));
        assert_eq!(models_dev_provider_key(Provider::Groq), Some("groq"));
        assert_eq!(models_dev_provider_key(Provider::HuggingFace), Some("huggingface"));
        assert_eq!(models_dev_provider_key(Provider::Mistral), Some("mistral"));
        assert_eq!(
            models_dev_provider_key(Provider::MoonshotAi),
            Some("moonshotai")
        );
        assert_eq!(models_dev_provider_key(Provider::Ollama), None);
        assert_eq!(models_dev_provider_key(Provider::OpenAi), Some("openai"));
        assert_eq!(models_dev_provider_key(Provider::OpenRouter), Some("openrouter"));
        assert_eq!(models_dev_provider_key(Provider::Xai), Some("xai"));
        assert_eq!(models_dev_provider_key(Provider::Zai), Some("zai"));
        assert_eq!(models_dev_provider_key(Provider::ZenMux), None);
    }

    #[test]
    fn test_deserialize_fixture_roster_providers() {
        let index = fixture_index();
        for provider in [
            "anthropic",
            "google",
            "openai",
            "openrouter",
            "xai",
            "zai",
            "deepseek",
            "groq",
            "mistral",
            "moonshotai",
        ] {
            assert!(
                index.get(provider).is_some_and(|models| models.len() >= 2),
                "{provider} should have at least two fixture models"
            );
        }
    }

    #[test]
    fn test_field_mapping_and_pricing_conversion() {
        let model = fixture_model("anthropic", "claude-opus-4.5");
        let metadata = models_dev_to_metadata(&model).expect("fixture date should be valid");

        assert_eq!(metadata.display_name.as_deref(), Some("Claude Opus 4.5"));
        assert_eq!(metadata.family.as_deref(), Some("claude-opus"));
        assert_eq!(metadata.context_window, Some(200000));
        assert_eq!(metadata.max_output_tokens, Some(32000));
        assert_eq!(metadata.knowledge_cutoff.as_deref(), Some("2025-01"));
        assert_eq!(metadata.release_date.as_deref(), Some("2025-11-01"));

        let pricing = metadata.pricing.expect("pricing should map");
        assert_close(pricing.prompt_per_token, Some(0.000015));
        assert_close(pricing.completion_per_token, Some(0.000075));
        assert_close(pricing.input_cache_read_per_token, Some(0.0000015));
    }

    #[test]
    fn test_canonical_capability_mapping() {
        let model = fixture_model("anthropic", "claude-opus-4.5");
        let metadata = models_dev_to_metadata(&model).expect("fixture date should be valid");

        assert_eq!(
            metadata.capabilities,
            vec![
                CAPABILITY_FUNCTION_CALLING.to_string(),
                CAPABILITY_STRUCTURED_OUTPUT.to_string(),
                CAPABILITY_REASONING.to_string(),
                CAPABILITY_FILE_INPUT.to_string(),
            ]
        );
    }

    #[test]
    fn test_modality_parsing() {
        let model = fixture_model("google", "gemini-2.5-pro");
        let metadata = models_dev_to_metadata(&model).expect("fixture date should be valid");
        let modalities = metadata.modalities.expect("modalities should map");

        assert_eq!(modalities.input, vec![Modality::Text, Modality::Image]);
        assert_eq!(modalities.output, vec![Modality::Text]);
    }

    #[test]
    fn test_unknown_fields_are_ignored() {
        let input = r#"{
            "openai": {
                "unknown_provider_field": true,
                "models": {
                    "gpt-4.1": {
                        "name": "GPT-4.1",
                        "unknown_model_field": "ignored",
                        "cost": {
                            "input": "2.00",
                            "unknown_cost_field": "ignored"
                        }
                    }
                }
            }
        }"#;

        let index = parse_models_dev_response_str(input).expect("unknown fields should be ignored");
        let model = index
            .get("openai")
            .and_then(|models| models.get("gpt-4.1"))
            .expect("model should deserialize");
        let metadata = models_dev_to_metadata(model).expect("no release date should be valid");

        assert_eq!(metadata.display_name.as_deref(), Some("GPT-4.1"));
        assert_close(
            metadata.pricing.and_then(|pricing| pricing.prompt_per_token),
            Some(0.000002),
        );
    }

    #[test]
    fn test_release_date_rejects_partial_dates() {
        let model = ModelsDevModel {
            release_date: Some("2026-01".to_string()),
            ..Default::default()
        };

        let err = models_dev_to_metadata(&model).expect_err("partial dates should fail loudly");

        assert!(
            matches!(err, ModelsDevError::InvalidReleaseDate { value } if value == "2026-01")
        );
    }

    #[test]
    fn test_release_date_omits_empty_values() {
        let model = ModelsDevModel {
            release_date: Some("   ".to_string()),
            ..Default::default()
        };

        let metadata = models_dev_to_metadata(&model).expect("blank date should be omitted");

        assert_eq!(metadata.release_date, None);
    }

    #[test]
    fn test_matcher_exact_hit() {
        let bucket = fixture_bucket("anthropic");

        let matched = find_models_dev_metadata("claude-opus-4.5", Provider::Anthropic, &bucket)
            .expect("exact match should hit");

        assert_eq!(matched.name.as_deref(), Some("Claude Opus 4.5"));
    }

    #[test]
    fn test_matcher_identity_hit_across_dash_dot() {
        let mut bucket = fixture_bucket("anthropic");
        bucket.remove("claude-opus-4-5-20251101");

        let matched = find_models_dev_metadata(
            "claude-opus-4-5-20251101",
            Provider::Anthropic,
            &bucket,
        )
        .expect("identity match should bridge dash and dot version spelling");

        assert_eq!(matched.name.as_deref(), Some("Claude Opus 4.5"));
    }

    #[test]
    fn test_matcher_prefers_exact_date_pin() {
        let bucket = fixture_bucket("anthropic");

        let matched = find_models_dev_metadata(
            "claude-opus-4-5-20251101",
            Provider::Anthropic,
            &bucket,
        )
        .expect("date-pinned match should hit");

        assert_eq!(
            matched.name.as_deref(),
            Some("Claude Opus 4.5 20251101")
        );
    }

    #[test]
    fn test_matcher_prefers_unpinned_when_date_pin_differs() {
        let mut bucket = BTreeMap::new();
        bucket.insert(
            "claude-opus-4.5".to_string(),
            minimal_model("Claude Opus 4.5"),
        );
        bucket.insert(
            "claude-opus-4-5-20251212".to_string(),
            minimal_model("Claude Opus 4.5 20251212"),
        );

        let matched = find_models_dev_metadata(
            "claude-opus-4-5-20251101",
            Provider::Anthropic,
            &bucket,
        )
        .expect("unpinned candidate should win after date-pin mismatch");

        assert_eq!(matched.name.as_deref(), Some("Claude Opus 4.5"));
    }

    #[test]
    fn test_matcher_refuses_cross_provider() {
        let bucket = fixture_bucket("openai");

        let matched = find_models_dev_metadata("claude-opus-4.5", Provider::Anthropic, &bucket);

        assert!(matched.is_none());
    }

    #[test]
    fn test_matcher_refuses_ambiguity() {
        let mut bucket = BTreeMap::new();
        bucket.insert(
            "claude-opus-4-5-20251212".to_string(),
            minimal_model("Claude Opus 4.5 20251212"),
        );
        bucket.insert(
            "claude-opus-4-5-20251213".to_string(),
            minimal_model("Claude Opus 4.5 20251213"),
        );

        let matched = find_models_dev_metadata(
            "claude-opus-4-5-20251101",
            Provider::Anthropic,
            &bucket,
        );

        assert!(matched.is_none());
    }

    #[test]
    fn test_matcher_no_match() {
        let bucket = fixture_bucket("anthropic");

        let matched = find_models_dev_metadata("not-a-real-model", Provider::Anthropic, &bucket);

        assert!(matched.is_none());
    }

    #[test]
    fn test_validate_models_dev_index_rejects_thin_response() {
        let index = fixture_index();

        let err = validate_models_dev_index(&index).expect_err("fixture is intentionally thin");

        assert!(matches!(err, ModelsDevError::ThinResponse { .. }));
    }

    #[test]
    fn test_validate_roster_critical_rejects_missing_provider() {
        let mut index = fixture_index();
        index.remove("mistral");

        let err = validate_roster_critical_models_dev_providers(&index)
            .expect_err("missing provider should fail");

        assert!(matches!(
            err,
            ModelsDevError::MissingProvider {
                provider: "mistral"
            }
        ));
    }

    #[test]
    fn test_validate_roster_critical_rejects_empty_provider() {
        let mut index = fixture_index();
        index.insert("mistral".to_string(), BTreeMap::new());

        let err = validate_roster_critical_models_dev_providers(&index)
            .expect_err("empty provider should fail");

        assert!(matches!(
            err,
            ModelsDevError::EmptyProvider {
                provider: "mistral"
            }
        ));
    }

    #[test]
    fn test_validate_roster_critical_accepts_fixture() {
        let index = fixture_index();

        validate_roster_critical_models_dev_providers(&index)
            .expect("fixture has all roster-critical providers");
    }

    #[test]
    fn test_no_degraded_mode_guard_failures_are_errors() {
        let thin = fixture_index();
        assert!(validate_models_dev_index(&thin).is_err());

        let mut missing = fixture_index();
        missing.remove("anthropic");
        assert!(validate_roster_critical_models_dev_providers(&missing).is_err());

        let mut empty = fixture_index();
        empty.insert("anthropic".to_string(), BTreeMap::new());
        assert!(validate_roster_critical_models_dev_providers(&empty).is_err());
    }

    fn assert_close(actual: Option<f64>, expected: Option<f64>) {
        match (actual, expected) {
            (Some(actual), Some(expected)) => {
                assert!((actual - expected).abs() < 0.000000000001);
            }
            (actual, expected) => assert_eq!(actual, expected),
        }
    }
}
