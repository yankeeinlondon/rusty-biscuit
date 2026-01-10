use crate::models::model_metadata::{ModelMetadata, Modality};
use crate::rigging::providers::models::{
    anthropic::ProviderModelAnthropic,
    deepseek::ProviderModelDeepseek,
    gemini::ProviderModelGemini,
    groq::ProviderModelGroq,
    mistral::ProviderModelMistral,
    moonshotai::ProviderModelMoonshotAi,
    openai::ProviderModelOpenAi,
    openrouter::ProviderModelOpenRouter,
    xai::ProviderModelXai,
    zai::ProviderModelZai,
    zenmux::ProviderModelZenMux,
};

pub mod anthropic;
pub mod deepseek;
pub mod gemini;
pub mod groq;
pub mod mistral;
pub mod moonshotai;
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

impl ProviderModel {
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
            Self::OpenAi(m) => m.model_id(),
            Self::OpenRouter(m) => m.model_id(),
            Self::Xai(m) => m.model_id(),
            Self::Zai(m) => m.model_id(),
            Self::ZenMux(m) => m.model_id(),
        }
    }

    /// Returns metadata for this model if available.
    ///
    /// Metadata is fetched from the Parsera LLM Specs API at build time
    /// and includes context window, modalities, and capabilities.
    #[must_use]
    pub fn metadata(&self) -> Option<&'static ModelMetadata> {
        metadata_generated::MODEL_METADATA.get(self.model_id())
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
        let _anthropic = ProviderModel::Anthropic(ProviderModelAnthropic::Claude__Opus__4__5__20251101);
        let _deepseek = ProviderModel::Deepseek(ProviderModelDeepseek::Deepseek__Chat);
        let _gemini = ProviderModel::Gemini(ProviderModelGemini::Gemini__2_5__Pro);
        let _groq = ProviderModel::Groq(ProviderModelGroq::Llama__3_3__70b__Versatile);
        let _mistral = ProviderModel::Mistral(ProviderModelMistral::Bespoke("mistral-large".to_string()));
        let _moonshot = ProviderModel::MoonshotAi(ProviderModelMoonshotAi::Kimi__K2__Thinking);
        let _openai = ProviderModel::OpenAi(ProviderModelOpenAi::O3);
        let _openrouter = ProviderModel::OpenRouter(ProviderModelOpenRouter::Bespoke("test".to_string()));
        let _xai = ProviderModel::Xai(ProviderModelXai::Grok__3);
        let _zai = ProviderModel::Zai(ProviderModelZai::Glm__4_7);
        let _zenmux = ProviderModel::ZenMux(ProviderModelZenMux::Bespoke("test".to_string()));
    }

    /// Test metadata accessor methods return None for unknown models.
    ///
    /// With the empty placeholder metadata file, all models should return None.
    #[test]
    fn test_metadata_accessors_return_none_for_unknown() {
        let model = ProviderModel::OpenAi(ProviderModelOpenAi::Bespoke("unknown-model".to_string()));

        assert!(model.metadata().is_none());
        assert!(model.context_window().is_none());
        assert!(model.max_output_tokens().is_none());
        assert!(!model.supports_input(Modality::Text));
        assert!(!model.supports_output(Modality::Text));
        assert!(!model.has_capability("function_calling"));
    }
}
