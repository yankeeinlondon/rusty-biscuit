//! `real_` tier: opt-in end-to-end tests against genuinely authenticated
//! remote providers.
//!
//! These tests call live provider APIs, so they are skipped unless
//! `UNCHAINED_AI_CONTRACT_REAL=1` is set **and** at least one non-local
//! provider has a credential present in the process environment. Each enabled
//! provider gets a prose request and a structured request; the structured
//! response is validated by the adapter's bundled JSON Schema engine.
//!
//! Run with:
//!
//! ```sh
//! UNCHAINED_AI_CONTRACT_REAL=1 cargo test -p unchained-ai-contract --test real_provider -- --nocapture
//! ```

use biscuit_contract::inference::{
    InferenceData, InferenceOutput, InferenceProfile, InferenceRequest,
};
use serde_json::json;
use unchained_ai::rigging::providers::models::{
    ProviderModel, anthropic::ProviderModelAnthropic, deepseek::ProviderModelDeepseek,
    gemini::ProviderModelGemini, groq::ProviderModelGroq, mistral::ProviderModelMistral,
    moonshotai::ProviderModelMoonshotAi, openai::ProviderModelOpenAi,
    openrouter::ProviderModelOpenRouter, xai::ProviderModelXai, zai::ProviderModelZai,
    zenmux::ProviderModelZenMux,
};
use unchained_ai::rigging::providers::Provider;
use unchained_ai_contract::UnchainedInferenceAdapter;

/// Non-local providers that can be exercised by the real tier.
const REAL_PROVIDERS: &[Provider] = &[
    Provider::Anthropic,
    Provider::OpenAi,
    Provider::Gemini,
    Provider::Groq,
    Provider::Mistral,
    Provider::MoonshotAi,
    Provider::Deepseek,
    Provider::Xai,
    Provider::OpenRouter,
    Provider::Zai,
    Provider::ZenMux,
];

/// Whether the real tier is enabled at all (the opt-in env gate).
///
/// `test_name` is only used to make the skip line unambiguous in cargo output,
/// where a skipped real test otherwise prints the same `... ok` as one that
/// genuinely exercised a provider.
fn real_enabled(test_name: &str) -> bool {
    if std::env::var("UNCHAINED_AI_CONTRACT_REAL").as_deref() != Ok("1") {
        eprintln!(
            "SKIP[real-provider] {test_name}: NO PROVIDER EXERCISED \
(set UNCHAINED_AI_CONTRACT_REAL=1 to enable the real tier)"
        );
        return false;
    }
    true
}

/// Returns the providers that have a non-empty credential environment variable.
fn authenticated_providers() -> Vec<Provider> {
    REAL_PROVIDERS
        .iter()
        .copied()
        .filter(|provider| {
            let config = provider.config();
            if config.is_local {
                return false;
            }
            let present = config
                .env_vars
                .iter()
                .any(|var| env_value_nonempty(var));
            if !present {
                eprintln!(
                    "skipping `{}` (no credential env var set)",
                    provider.display_name()
                );
            }
            present
        })
        .collect()
}

fn env_value_nonempty(var: &str) -> bool {
    std::env::var(var)
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false)
}

/// Picks a sensible predefined model for each provider.
fn default_model_for(provider: Provider) -> ProviderModel {
    match provider {
        Provider::Anthropic => {
            ProviderModel::Anthropic(ProviderModelAnthropic::Claude__Sonnet__4__5__20250929)
        }
        Provider::OpenAi => ProviderModel::OpenAi(ProviderModelOpenAi::Gpt__5_2),
        Provider::Gemini => ProviderModel::Gemini(ProviderModelGemini::Gemini__2_5__Pro),
        Provider::Groq => ProviderModel::Groq(ProviderModelGroq::Llama__3_3__70b__Versatile),
        Provider::Mistral => ProviderModel::Mistral(ProviderModelMistral::Mistral__Large__Latest),
        Provider::MoonshotAi => ProviderModel::MoonshotAi(ProviderModelMoonshotAi::Kimi__K2_5),
        Provider::Deepseek => ProviderModel::Deepseek(ProviderModelDeepseek::Deepseek__V4__Pro),
        Provider::Xai => ProviderModel::Xai(ProviderModelXai::Grok__4_3),
        Provider::OpenRouter => {
            ProviderModel::OpenRouter(ProviderModelOpenRouter::Anthropic___Claude__Sonnet__4_5)
        }
        Provider::Zai => ProviderModel::Zai(ProviderModelZai::Glm__5),
        Provider::ZenMux => {
            ProviderModel::ZenMux(ProviderModelZenMux::Anthropic___Claude__Sonnet__4_5)
        }
        other => panic!("provider `{}` is not supported by the real tier", other.display_name()),
    }
}

#[tokio::test]
async fn real_prose_request_completes_end_to_end() {
    if !real_enabled("real_prose_request_completes_end_to_end") {
        return;
    }

    let providers = authenticated_providers();
    if providers.is_empty() {
        eprintln!(
            "SKIP[real-provider] real_prose_request_completes_end_to_end: \
NO PROVIDER EXERCISED (UNCHAINED_AI_CONTRACT_REAL=1 set, but no provider credentials present)"
        );
        return;
    }

    for provider in providers {
        let adapter = UnchainedInferenceAdapter::new()
            .with_model(default_model_for(provider))
            .build();
        let request = InferenceRequest {
            prompt: "Reply with exactly the single word: pong".to_string(),
            output: InferenceOutput::Prose,
            profile: InferenceProfile::default(),
        };

        let response = adapter
            .infer(request)
            .await
            .unwrap_or_else(|err| panic!("prose inference for `{}` succeeds: {err:?}", provider.display_name()));

        match response.data {
            InferenceData::Prose(text) => {
                assert!(!text.trim().is_empty(), "`{}`: expected non-empty prose", provider.display_name());
            }
            other => panic!("`{}`: expected prose, got {other:?}", provider.display_name()),
        }
        assert_eq!(
            response.metadata.provider.as_deref(),
            Some(provider.display_name())
        );
    }
}

#[tokio::test]
async fn real_structured_request_validates_against_schema() {
    if !real_enabled("real_structured_request_validates_against_schema") {
        return;
    }

    let providers = authenticated_providers();
    if providers.is_empty() {
        eprintln!(
            "SKIP[real-provider] real_structured_request_validates_against_schema: \
NO PROVIDER EXERCISED (UNCHAINED_AI_CONTRACT_REAL=1 set, but no provider credentials present)"
        );
        return;
    }

    let schema = json!({
        "type": "object",
        "properties": {
            "language": { "type": "string" },
            "confidence": { "type": "number" }
        },
        "required": ["language", "confidence"],
        "additionalProperties": false
    });

    for provider in providers {
        let adapter = UnchainedInferenceAdapter::new()
            .with_model(default_model_for(provider))
            .build();
        let request = InferenceRequest {
            prompt: "Identify the language of this text and your confidence 0-1: 'Bonjour le monde'."
                .to_string(),
            output: InferenceOutput::Structured { schema: schema.clone() },
            profile: InferenceProfile::default(),
        };

        let response = adapter
            .infer(request)
            .await
            .unwrap_or_else(|err| panic!("structured inference for `{}` succeeds: {err:?}", provider.display_name()));

        match response.data {
            InferenceData::Structured(value) => {
                assert!(value.get("language").is_some(), "`{}`: schema field present", provider.display_name());
                assert!(value.get("confidence").is_some(), "`{}`: schema field present", provider.display_name());
            }
            other => panic!("`{}`: expected structured value, got {other:?}", provider.display_name()),
        }
    }
}
