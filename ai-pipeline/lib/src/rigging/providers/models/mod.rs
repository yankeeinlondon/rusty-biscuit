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
}
