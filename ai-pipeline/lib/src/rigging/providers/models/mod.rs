use crate::rigging::providers::models::{
  anthropic::ProviderModelAnthropic, deepseek::ProviderModelDeepseek, gemini::ProviderModelGemini, groq::ProviderModelGroq, mistral::ProviderModelMistral, moonshotai::ProviderModelMoonshotAi, openrouter::ProviderModelOpenRouter, xai::ProviderModelXai, zenmux::ProviderModelZenMux
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

/// The `ProviderModel` enumeration provides access to _all_ the
/// models across _all_ the providers which this repo supports.
pub enum ProviderModel {
  /// Anthropic models
  Anthropic(ProviderModelAnthropic),
  /// Deepseek models
  Deepseek(ProviderModelDeepseek),
  /// Gemini models
  Gemini(ProviderModelGemini),
  /// Groq models
  Groq(ProviderModelGroq),
  /// Mistral
  Mistral(ProviderModelMistral),
  /// MoonshotAI models (aggregator)
  MoonshotAi(ProviderModelMoonshotAi),
  /// OpenRouter models
  OpenRouter(ProviderModelOpenRouter),
  /// xAI models
  Xai(ProviderModelXai),
  /// Z.ai models
  Zai(ProviderModelXai),
  /// ZenMux models (aggregator)
  ZenMux(ProviderModelZenMux)
}
