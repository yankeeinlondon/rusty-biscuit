use std::fmt;
use std::str::FromStr;

use serde::de::Error as SerdeError;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::Provider;

use crate::models::model_metadata::{Modality, ProviderModelMetadata};
use crate::rigging::providers::models::{
    anthropic::ProviderModelAnthropic, deepseek::ProviderModelDeepseek,
    gemini::ProviderModelGemini, groq::ProviderModelGroq, mistral::ProviderModelMistral,
    moonshotai::ProviderModelMoonshotAi, ollama::ProviderModelOllama, openai::ProviderModelOpenAi,
    openrouter::ProviderModelOpenRouter, xai::ProviderModelXai, zai::ProviderModelZai,
    zenmux::ProviderModelZenMux,
};

pub mod anthropic;
pub mod deepseek;
pub mod gemini;
pub mod groq;
pub mod mistral;
pub mod moonshotai;
pub mod ollama;
pub mod openai;
pub mod openrouter;
pub mod xai;
pub mod zai;
pub mod zenmux;

pub mod build;
mod metadata_generated;

/// Aggregated enumeration of all provider models.
///
/// This enum provides access to _all_ the models across _all_ the providers
/// which this repo supports. Each variant wraps a provider-specific enum.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ProviderModel {
    /// Anthropic models
    Anthropic(ProviderModelAnthropic),
    /// DeepSeek models
    Deepseek(ProviderModelDeepseek),
    /// Google Gemini models
    Gemini(ProviderModelGemini),
    /// Groq models
    Groq(ProviderModelGroq),
    /// Mistral AI models
    Mistral(ProviderModelMistral),
    /// Moonshot AI (Kimi) models
    MoonshotAi(ProviderModelMoonshotAi),
    /// Local Ollama models
    Ollama(ProviderModelOllama),
    /// OpenAI models
    OpenAi(ProviderModelOpenAi),
    /// OpenRouter aggregator models
    OpenRouter(ProviderModelOpenRouter),
    /// xAI (Grok) models
    Xai(ProviderModelXai),
    /// Zhipu AI (Z.ai) models
    Zai(ProviderModelZai),
    /// ZenMux aggregator models
    ZenMux(ProviderModelZenMux),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderModelParseError {
    InvalidFormat { input: String },
    UnknownProvider { provider: String, input: String },
    UnknownModel { provider: Provider, model: String },
}

impl fmt::Display for ProviderModelParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidFormat { input } => write!(
                f,
                "Invalid model string format: '{}' (expected 'provider/model-id')",
                input
            ),
            Self::UnknownProvider { provider, input } => {
                write!(
                    f,
                    "Unknown provider '{}' in model string '{}'",
                    provider, input
                )
            }
            Self::UnknownModel { provider, model } => {
                write!(f, "Unknown model '{}' for provider {:?}", model, provider)
            }
        }
    }
}

impl std::error::Error for ProviderModelParseError {}

impl ProviderModel {
    pub fn provider(&self) -> Provider {
        match self {
            Self::Anthropic(_) => Provider::Anthropic,
            Self::Deepseek(_) => Provider::Deepseek,
            Self::Gemini(_) => Provider::Gemini,
            Self::Groq(_) => Provider::Groq,
            Self::Mistral(_) => Provider::Mistral,
            Self::MoonshotAi(_) => Provider::MoonshotAi,
            Self::Ollama(_) => Provider::Ollama,
            Self::OpenAi(_) => Provider::OpenAi,
            Self::OpenRouter(_) => Provider::OpenRouter,
            Self::Xai(_) => Provider::Xai,
            Self::Zai(_) => Provider::Zai,
            Self::ZenMux(_) => Provider::ZenMux,
        }
    }

    pub fn wire_id(&self) -> String {
        format!("{}/{}", provider_slug(self.provider()), self.model_id())
    }

    pub fn parse_wire_id(input: &str) -> Result<Self, ProviderModelParseError> {
        let trimmed = input.trim();
        let (provider_raw, model_id) =
            trimmed
                .split_once('/')
                .ok_or_else(|| ProviderModelParseError::InvalidFormat {
                    input: trimmed.to_string(),
                })?;

        let provider_key = normalize_provider_key(provider_raw);
        let provider = match provider_key.as_str() {
            "anthropic" => Provider::Anthropic,
            "deepseek" => Provider::Deepseek,
            "gemini" => Provider::Gemini,
            "groq" => Provider::Groq,
            "mistral" => Provider::Mistral,
            "moonshotai" | "moonshot" => Provider::MoonshotAi,
            "ollama" => Provider::Ollama,
            "openai" => Provider::OpenAi,
            "openrouter" => Provider::OpenRouter,
            "xai" => Provider::Xai,
            "zai" => Provider::Zai,
            "zenmux" => Provider::ZenMux,
            _ => {
                return Err(ProviderModelParseError::UnknownProvider {
                    provider: provider_raw.trim().to_string(),
                    input: trimmed.to_string(),
                });
            }
        };

        let model_id = model_id.trim();
        if model_id.is_empty() {
            return Err(ProviderModelParseError::InvalidFormat {
                input: trimmed.to_string(),
            });
        }

        match provider {
            Provider::Anthropic => ProviderModelAnthropic::from_str(model_id)
                .map(Self::Anthropic)
                .map_err(|_| ProviderModelParseError::UnknownModel {
                    provider,
                    model: model_id.to_string(),
                }),
            Provider::Deepseek => ProviderModelDeepseek::from_str(model_id)
                .map(Self::Deepseek)
                .map_err(|_| ProviderModelParseError::UnknownModel {
                    provider,
                    model: model_id.to_string(),
                }),
            Provider::Gemini => ProviderModelGemini::from_str(model_id)
                .map(Self::Gemini)
                .map_err(|_| ProviderModelParseError::UnknownModel {
                    provider,
                    model: model_id.to_string(),
                }),
            Provider::Groq => ProviderModelGroq::from_str(model_id)
                .map(Self::Groq)
                .map_err(|_| ProviderModelParseError::UnknownModel {
                    provider,
                    model: model_id.to_string(),
                }),
            Provider::Mistral => ProviderModelMistral::from_str(model_id)
                .map(Self::Mistral)
                .map_err(|_| ProviderModelParseError::UnknownModel {
                    provider,
                    model: model_id.to_string(),
                }),
            Provider::MoonshotAi => ProviderModelMoonshotAi::from_str(model_id)
                .map(Self::MoonshotAi)
                .map_err(|_| ProviderModelParseError::UnknownModel {
                    provider,
                    model: model_id.to_string(),
                }),
            Provider::Ollama => ProviderModelOllama::from_str(model_id)
                .map(Self::Ollama)
                .map_err(|_| ProviderModelParseError::UnknownModel {
                    provider,
                    model: model_id.to_string(),
                }),
            Provider::OpenAi => ProviderModelOpenAi::from_str(model_id)
                .map(Self::OpenAi)
                .map_err(|_| ProviderModelParseError::UnknownModel {
                    provider,
                    model: model_id.to_string(),
                }),
            Provider::OpenRouter => ProviderModelOpenRouter::from_str(model_id)
                .map(Self::OpenRouter)
                .map_err(|_| ProviderModelParseError::UnknownModel {
                    provider,
                    model: model_id.to_string(),
                }),
            Provider::Xai => ProviderModelXai::from_str(model_id)
                .map(Self::Xai)
                .map_err(|_| ProviderModelParseError::UnknownModel {
                    provider,
                    model: model_id.to_string(),
                }),
            Provider::Zai => ProviderModelZai::from_str(model_id)
                .map(Self::Zai)
                .map_err(|_| ProviderModelParseError::UnknownModel {
                    provider,
                    model: model_id.to_string(),
                }),
            Provider::ZenMux => ProviderModelZenMux::from_str(model_id)
                .map(Self::ZenMux)
                .map_err(|_| ProviderModelParseError::UnknownModel {
                    provider,
                    model: model_id.to_string(),
                }),
            _ => Err(ProviderModelParseError::UnknownProvider {
                provider: provider_raw.trim().to_string(),
                input: trimmed.to_string(),
            }),
        }
    }
    /// Returns the canonical wire-format model ID.
    ///
    /// Delegates to the underlying provider-specific enum's `model_id()` method.
    #[must_use]
    pub fn model_id(&self) -> &str {
        match self {
            Self::Anthropic(m) => m.model_id(),
            Self::Deepseek(m) => m.model_id(),
            Self::Gemini(m) => m.model_id(),
            Self::Groq(m) => m.model_id(),
            Self::Mistral(m) => m.model_id(),
            Self::MoonshotAi(m) => m.model_id(),
            Self::Ollama(m) => m.model_id(),
            Self::OpenAi(m) => m.model_id(),
            Self::OpenRouter(m) => m.model_id(),
            Self::Xai(m) => m.model_id(),
            Self::Zai(m) => m.model_id(),
            Self::ZenMux(m) => m.model_id(),
        }
    }

    /// Returns metadata for this model if available.
    ///
    /// Delegates to the underlying provider-specific enum's `metadata()` method.
    /// Metadata is enriched at build time and includes context window,
    /// modalities, pricing, capabilities, and source release dates when
    /// available.
    #[must_use]
    pub fn metadata(&self) -> Option<&'static ProviderModelMetadata> {
        match self {
            Self::Anthropic(m) => m.metadata(),
            Self::Deepseek(m) => m.metadata(),
            Self::Gemini(m) => m.metadata(),
            Self::Groq(m) => m.metadata(),
            Self::Mistral(m) => m.metadata(),
            Self::MoonshotAi(m) => m.metadata(),
            Self::Ollama(m) => m.metadata(),
            Self::OpenAi(m) => m.metadata(),
            Self::OpenRouter(m) => m.metadata(),
            Self::Xai(m) => m.metadata(),
            Self::Zai(m) => m.metadata(),
            Self::ZenMux(m) => m.metadata(),
        }
    }

    /// Returns all known wire IDs across all providers.
    ///
    /// Useful for shell completion and model discovery.
    #[must_use]
    pub fn all_wire_ids() -> Vec<&'static str> {
        let mut ids = Vec::new();

        for model in ProviderModelAnthropic::ALL {
            ids.push(&*Box::leak(
                ProviderModel::Anthropic(model.clone())
                    .wire_id()
                    .into_boxed_str(),
            ));
        }
        for model in ProviderModelDeepseek::ALL {
            ids.push(&*Box::leak(
                ProviderModel::Deepseek(model.clone())
                    .wire_id()
                    .into_boxed_str(),
            ));
        }
        for model in ProviderModelGemini::ALL {
            ids.push(&*Box::leak(
                ProviderModel::Gemini(model.clone())
                    .wire_id()
                    .into_boxed_str(),
            ));
        }
        for model in ProviderModelGroq::ALL {
            ids.push(&*Box::leak(
                ProviderModel::Groq(model.clone())
                    .wire_id()
                    .into_boxed_str(),
            ));
        }
        for model in ProviderModelMistral::ALL {
            ids.push(&*Box::leak(
                ProviderModel::Mistral(model.clone())
                    .wire_id()
                    .into_boxed_str(),
            ));
        }
        for model in ProviderModelMoonshotAi::ALL {
            ids.push(&*Box::leak(
                ProviderModel::MoonshotAi(model.clone())
                    .wire_id()
                    .into_boxed_str(),
            ));
        }
        for model in ProviderModelOllama::ALL {
            ids.push(&*Box::leak(
                ProviderModel::Ollama(model.clone())
                    .wire_id()
                    .into_boxed_str(),
            ));
        }
        for model in ProviderModelOpenAi::ALL {
            ids.push(&*Box::leak(
                ProviderModel::OpenAi(model.clone())
                    .wire_id()
                    .into_boxed_str(),
            ));
        }
        for model in ProviderModelOpenRouter::ALL {
            ids.push(&*Box::leak(
                ProviderModel::OpenRouter(model.clone())
                    .wire_id()
                    .into_boxed_str(),
            ));
        }
        for model in ProviderModelXai::ALL {
            ids.push(&*Box::leak(
                ProviderModel::Xai(model.clone()).wire_id().into_boxed_str(),
            ));
        }
        for model in ProviderModelZai::ALL {
            ids.push(&*Box::leak(
                ProviderModel::Zai(model.clone()).wire_id().into_boxed_str(),
            ));
        }
        for model in ProviderModelZenMux::ALL {
            ids.push(&*Box::leak(
                ProviderModel::ZenMux(model.clone())
                    .wire_id()
                    .into_boxed_str(),
            ));
        }

        ids
    }

    /// Returns the context window size if known.
    ///
    /// The context window is the maximum number of tokens the model can
    /// process in a single request (input + output).
    #[must_use]
    pub fn context_window(&self) -> Option<u32> {
        self.metadata().and_then(|m| m.context_window)
    }

    /// Returns the maximum output tokens if known.
    ///
    /// This is the maximum number of tokens the model can generate
    /// in a single response.
    #[must_use]
    pub fn max_output_tokens(&self) -> Option<u32> {
        self.metadata().and_then(|m| m.max_output_tokens)
    }

    /// Returns true if this model supports the given input modality.
    ///
    /// Returns false if the model's modalities are unknown.
    #[must_use]
    pub fn supports_input(&self, modality: Modality) -> bool {
        self.metadata()
            .map(|m| m.supports_input(modality))
            .unwrap_or(false)
    }

    /// Returns true if this model supports the given output modality.
    ///
    /// Returns false if the model's modalities are unknown.
    #[must_use]
    pub fn supports_output(&self, modality: Modality) -> bool {
        self.metadata()
            .map(|m| m.supports_output(modality))
            .unwrap_or(false)
    }

    /// Returns true if this model has the specified capability.
    ///
    /// Common capabilities include "function_calling", "structured_output",
    /// "vision", etc.
    #[must_use]
    pub fn has_capability(&self, capability: &str) -> bool {
        self.metadata()
            .map(|m| m.has_capability(capability))
            .unwrap_or(false)
    }
}

fn normalize_provider_key(value: &str) -> String {
    value
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

fn provider_slug(provider: Provider) -> &'static str {
    match provider {
        Provider::Anthropic => "anthropic",
        Provider::Deepseek => "deepseek",
        Provider::Gemini => "gemini",
        Provider::Groq => "groq",
        Provider::Mistral => "mistral",
        Provider::MoonshotAi => "moonshotai",
        Provider::Ollama => "ollama",
        Provider::OpenAi => "openai",
        Provider::OpenRouter => "openrouter",
        Provider::Xai => "x-ai",
        Provider::Zai => "z-ai",
        Provider::ZenMux => "zenmux",
        Provider::HuggingFace => "unsupported",
    }
}

impl Serialize for ProviderModel {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let wire_id = self.wire_id();
        serializer.serialize_str(&wire_id)
    }
}

impl<'de> Deserialize<'de> for ProviderModel {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let input = String::deserialize(deserializer)?;
        Self::parse_wire_id(&input).map_err(SerdeError::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Regression test: ProviderModel must delegate model_id() to underlying enums.
    ///
    /// The aggregated ProviderModel doesn't use the ModelId derive macro,
    /// so it needs a manual implementation that correctly delegates.
    #[test]
    fn test_provider_model_delegates_model_id() {
        // Test Anthropic delegation
        let model = ProviderModel::Anthropic(ProviderModelAnthropic::Claude__Opus__4__5__20251101);
        assert_eq!(model.model_id(), "claude-opus-4-5-20251101");

        // Test OpenAI delegation
        let model = ProviderModel::OpenAi(ProviderModelOpenAi::O3);
        assert_eq!(model.model_id(), "o3");

        // Test Bespoke variant delegation
        let model = ProviderModel::Gemini(ProviderModelGemini::Bespoke("custom-model".to_string()));
        assert_eq!(model.model_id(), "custom-model");
    }

    /// Test that all provider variants can be wrapped in ProviderModel.
    #[test]
    fn test_all_provider_variants_constructible() {
        // This test ensures the enum variants compile and are accessible
        let _anthropic =
            ProviderModel::Anthropic(ProviderModelAnthropic::Claude__Opus__4__5__20251101);
        let _deepseek = ProviderModel::Deepseek(ProviderModelDeepseek::Deepseek__V4__Flash);
        let _gemini = ProviderModel::Gemini(ProviderModelGemini::Gemini__2_5__Pro);
        let _groq = ProviderModel::Groq(ProviderModelGroq::Llama__3_3__70b__Versatile);
        let _mistral =
            ProviderModel::Mistral(ProviderModelMistral::Bespoke("mistral-large".to_string()));
        let _moonshot = ProviderModel::MoonshotAi(ProviderModelMoonshotAi::Kimi__K2_5);
        let _ollama = ProviderModel::Ollama(ProviderModelOllama::Llama__3_1);
        let _openai = ProviderModel::OpenAi(ProviderModelOpenAi::O3);
        let _openrouter =
            ProviderModel::OpenRouter(ProviderModelOpenRouter::Bespoke("test".to_string()));
        let _xai = ProviderModel::Xai(ProviderModelXai::Bespoke("grok-4.3".to_string()));
        let _zai = ProviderModel::Zai(ProviderModelZai::Glm__4_7);
        let _zenmux = ProviderModel::ZenMux(ProviderModelZenMux::Bespoke("test".to_string()));
    }

    #[test]
    fn test_wire_id_uses_canonical_provider_slugs() {
        let xai = ProviderModel::Xai(ProviderModelXai::Grok__4_3);
        assert_eq!(xai.wire_id(), "x-ai/grok-4.3");

        let zai = ProviderModel::Zai(ProviderModelZai::Glm__4_5);
        assert_eq!(zai.wire_id(), "z-ai/glm-4.5");
    }

    #[test]
    fn test_parse_wire_id_accepts_legacy_provider_slugs() {
        let xai = ProviderModel::parse_wire_id("xai/grok-4.3").unwrap();
        assert_eq!(xai.wire_id(), "x-ai/grok-4.3");

        let zai = ProviderModel::parse_wire_id("zai/glm-4.5").unwrap();
        assert_eq!(zai.wire_id(), "z-ai/glm-4.5");
    }

    #[test]
    fn test_aggregator_namespaces_decode_with_single_hyphens() {
        let openrouter = ProviderModel::OpenRouter(ProviderModelOpenRouter::X__Ai___Grok__4_3);
        assert_eq!(openrouter.wire_id(), "openrouter/x-ai/grok-4.3");

        let zenmux = ProviderModel::ZenMux(ProviderModelZenMux::Z__Ai___Glm__4_5);
        assert_eq!(zenmux.wire_id(), "zenmux/z-ai/glm-4.5");
    }

    #[test]
    fn test_generated_model_ids_do_not_emit_double_hyphen_provider_namespaces() {
        for wire_id in ProviderModel::all_wire_ids() {
            assert!(
                !wire_id.contains("x--ai") && !wire_id.contains("z--ai"),
                "wire_id contains a double-hyphen provider namespace: {wire_id}"
            );
        }
    }

    /// Test metadata accessor methods return None for unknown models.
    ///
    /// Bespoke model IDs do not have generated metadata entries.
    #[test]
    fn test_metadata_accessors_return_none_for_unknown() {
        let model =
            ProviderModel::OpenAi(ProviderModelOpenAi::Bespoke("unknown-model".to_string()));

        assert!(model.metadata().is_none());
        assert!(model.context_window().is_none());
        assert!(model.max_output_tokens().is_none());
        assert!(!model.supports_input(Modality::Text));
        assert!(!model.supports_output(Modality::Text));
        assert!(!model.has_capability("function_calling"));
    }

    /// Test that individual provider model enums have direct metadata() access.
    ///
    /// The `#[model_id_metadata]` attribute on provider enums generates a
    /// `metadata()` method that looks up in the static MODEL_METADATA table.
    #[test]
    fn test_individual_provider_model_metadata() {
        // Test Gemini - a model with known metadata
        let gemini = ProviderModelGemini::Gemini__2_0__Flash;
        let meta = gemini.metadata();
        assert!(meta.is_some(), "Gemini 2.0 Flash should have metadata");

        let meta = meta.unwrap();
        assert!(meta.context_window.is_some());
        assert!(meta.context_window.unwrap() > 0);

        // Test OpenAI O3 - a model with known metadata
        let o3 = ProviderModelOpenAi::O3;
        let meta = o3.metadata();
        assert!(meta.is_some(), "OpenAI O3 should have metadata");
    }

    #[test]
    fn test_direct_anthropic_models_have_priced_metadata() {
        for model in ProviderModelAnthropic::ALL {
            let metadata = model
                .metadata()
                .unwrap_or_else(|| panic!("{} is missing metadata", model.model_id()));
            let pricing = metadata
                .pricing
                .as_ref()
                .unwrap_or_else(|| panic!("{} is missing pricing", model.model_id()));
            assert!(
                pricing.prompt_per_token.is_some() || pricing.completion_per_token.is_some(),
                "{} is missing per-token pricing",
                model.model_id()
            );
        }
    }

    /// Test that Bespoke variants return None for metadata on individual enums.
    #[test]
    fn test_individual_provider_model_bespoke_no_metadata() {
        let bespoke = ProviderModelOpenAi::Bespoke("custom-model-xyz".to_string());
        assert!(
            bespoke.metadata().is_none(),
            "Bespoke models should not have metadata"
        );

        let bespoke_gemini = ProviderModelGemini::Bespoke("my-custom-gemini".to_string());
        assert!(
            bespoke_gemini.metadata().is_none(),
            "Bespoke models should not have metadata"
        );
    }
}
