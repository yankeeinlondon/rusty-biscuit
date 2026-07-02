//! Capability-based model selection.
//!
//! Resolves an abstract [`ModelCapability`] into a concrete [`ProviderModel`]
//! by walking a pre-defined stack of models and returning the first one whose
//! provider is runnable in the current environment.

use std::collections::HashMap;

use crate::models::model_capability::ModelCapability;
use crate::rigging::providers::models::{
    anthropic::ProviderModelAnthropic, deepseek::ProviderModelDeepseek,
    gemini::ProviderModelGemini, groq::ProviderModelGroq, mistral::ProviderModelMistral,
    moonshotai::ProviderModelMoonshotAi, ollama::ProviderModelOllama, openai::ProviderModelOpenAi,
    openrouter::ProviderModelOpenRouter, xai::ProviderModelXai, zai::ProviderModelZai,
    ProviderModel,
};
use crate::rigging::providers::provider_errors::ProviderError;

/// A view of the process environment used to determine whether a provider is
/// runnable. Production code uses [`StdEnvView`]; tests inject a
/// [`HashMapEnvView`] so they do not need to mutate `std::env`.
pub trait EnvView: Send + Sync {
    /// Returns the value of the environment variable, or `None` if it is
    /// missing or empty.
    fn var(&self, key: &str) -> Option<String>;
}

/// Production environment view backed by [`std::env::var`].
pub struct StdEnvView;

impl EnvView for StdEnvView {
    fn var(&self, key: &str) -> Option<String> {
        std::env::var(key).ok().filter(|v| !v.trim().is_empty())
    }
}

/// Test environment view backed by an in-memory map.
#[derive(Debug, Default, Clone)]
pub struct HashMapEnvView {
    vars: HashMap<String, String>,
}

impl HashMapEnvView {
    /// Creates a new view from the provided key/value pairs.
    #[must_use]
    pub fn new(vars: impl IntoIterator<Item = (&'static str, &'static str)>) -> Self {
        Self {
            vars: vars
                .into_iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        }
    }

    /// Inserts or updates an environment variable.
    pub fn set(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.vars.insert(key.into(), value.into());
    }
}

impl EnvView for HashMapEnvView {
    fn var(&self, key: &str) -> Option<String> {
        self.vars.get(key).cloned().filter(|v| !v.trim().is_empty())
    }
}

/// Resolves a capability to the first runnable model in its stack.
///
/// A provider is runnable when it is local (e.g. Ollama) or when at least one
/// of its configured credential environment variables is present and
/// non-empty in the provided [`EnvView`].
///
/// For [`ModelCapability::Specific`], the specific model is returned only if
/// its provider is runnable.
pub fn resolve_model(
    capability: &ModelCapability,
    env: &dyn EnvView,
) -> Result<ProviderModel, ProviderError> {
    let stack = stack_for(capability);

    stack
        .into_iter()
        .find(|model| is_runnable(model, env))
        .ok_or_else(|| ProviderError::NoRunnableModel {
            capability: capability_to_string(capability),
        })
}

fn capability_to_string(capability: &ModelCapability) -> String {
    match capability {
        ModelCapability::Specific(model) => format!("Specific({})", model.wire_id()),
        _ => format!("{capability:?}"),
    }
}

fn is_runnable(model: &ProviderModel, env: &dyn EnvView) -> bool {
    let provider = model.provider();
    let config = provider.config();

    if config.is_local {
        return true;
    }

    config
        .env_vars
        .iter()
        .any(|var| env.var(var).is_some())
}

/// Returns the ordered fallback stack for a capability.
///
/// The ordering is a best-effort default based on the capability's intent:
///
/// - **Fast / FastCheap** prefer small, fast models from US/global providers,
///   with Chinese providers and local options lower in the stack.
/// - **Normal / NormalCheap** prefer capable mid-tier models.
/// - **Smart** prefer flagship models.
/// - **Thinking / Ultrathink** variants select models known to expose
///   reasoning parameters or reasoning modes.
/// - **Creative** and **Literal** variants reuse the base tier stacks; the
///   execution surface applies the temperature adjustment (Phase 4).
fn stack_for(capability: &ModelCapability) -> Vec<ProviderModel> {
    match capability {
        ModelCapability::FastCheap => vec![
            openai(ProviderModelOpenAi::Gpt__3_5__Turbo),
            gemini(ProviderModelGemini::Gemini__2_0__Flash__Lite),
            groq(ProviderModelGroq::Llama__3_1__8b__Instant),
            mistral(ProviderModelMistral::Mistral__Small__Latest),
            openrouter(ProviderModelOpenRouter::Openai___Gpt__5__Nano),
            zai(ProviderModelZai::Glm__4_5__Air),
            ollama("llama3.2"),
        ],
        ModelCapability::Fast => vec![
            openai(ProviderModelOpenAi::Gpt__5_2),
            gemini(ProviderModelGemini::Gemini__2_5__Flash),
            anthropic(ProviderModelAnthropic::Claude__Haiku__4__5__20251001),
            groq(ProviderModelGroq::Llama__3_3__70b__Versatile),
            mistral(ProviderModelMistral::Mistral__Small__Latest),
            xai("grok-4.3"),
            zai(ProviderModelZai::Glm__4_7),
            openrouter(ProviderModelOpenRouter::Anthropic___Claude__Haiku__4_5),
            ollama("llama3.2"),
        ],
        ModelCapability::Normal => vec![
            anthropic(ProviderModelAnthropic::Claude__Sonnet__4__5__20250929),
            openai(ProviderModelOpenAi::Gpt__5_2),
            gemini(ProviderModelGemini::Gemini__2_5__Pro),
            mistral(ProviderModelMistral::Mistral__Large__Latest),
            xai("grok-4.3"),
            deepseek(ProviderModelDeepseek::Deepseek__V4__Pro),
            moonshotai(ProviderModelMoonshotAi::Kimi__K2_5),
            zai(ProviderModelZai::Glm__5),
            openrouter(ProviderModelOpenRouter::Anthropic___Claude__Sonnet__4_5),
            ollama("llama3.1"),
        ],
        ModelCapability::NormalCheap => vec![
            openai(ProviderModelOpenAi::Gpt__3_5__Turbo),
            gemini(ProviderModelGemini::Gemini__2_5__Flash),
            groq(ProviderModelGroq::Llama__3_3__70b__Versatile),
            mistral(ProviderModelMistral::Mistral__Small__Latest),
            zai(ProviderModelZai::Glm__4_7),
            anthropic(ProviderModelAnthropic::Claude__Haiku__4__5__20251001),
            xai("grok-4.3"),
            openrouter(ProviderModelOpenRouter::Openai___Gpt__5__Mini),
            deepseek(ProviderModelDeepseek::Deepseek__V4__Flash),
            moonshotai(ProviderModelMoonshotAi::Moonshot__V1__8k),
            ollama("llama3.1"),
        ],
        ModelCapability::NormalThinking => vec![
            openai(ProviderModelOpenAi::O3__Mini),
            deepseek(ProviderModelDeepseek::Deepseek__V4__Pro),
            anthropic(ProviderModelAnthropic::Claude__Sonnet__4__5__20250929),
            openrouter(ProviderModelOpenRouter::Anthropic___Claude__Sonnet__4_5),
            gemini(ProviderModelGemini::Gemini__2_5__Pro),
            zai(ProviderModelZai::Glm__5),
            ollama("qwq"),
        ],
        ModelCapability::NormalThinkingCheap => vec![
            openai(ProviderModelOpenAi::O3__Mini),
            deepseek(ProviderModelDeepseek::Deepseek__V4__Flash),
            zai(ProviderModelZai::Glm__4_7),
            groq(ProviderModelGroq::Llama__3_3__70b__Versatile),
            ollama("qwq"),
        ],
        ModelCapability::NormalUltrathink => vec![
            openai(ProviderModelOpenAi::O3),
            deepseek(ProviderModelDeepseek::Deepseek__V4__Pro),
            anthropic(ProviderModelAnthropic::Claude__Sonnet__4__5__20250929),
            ollama("qwq"),
        ],
        ModelCapability::NormalCheapUltrathink => vec![
            openai(ProviderModelOpenAi::O3__Mini),
            deepseek(ProviderModelDeepseek::Deepseek__V4__Flash),
            zai(ProviderModelZai::Glm__4_7),
            ollama("qwq"),
        ],
        ModelCapability::Smart => vec![
            anthropic(ProviderModelAnthropic::Claude__Opus__4__5__20251101),
            openai(ProviderModelOpenAi::O3),
            gemini(ProviderModelGemini::Gemini__2_5__Pro),
            xai("grok-4.3"),
            mistral(ProviderModelMistral::Mistral__Large__Latest),
            deepseek(ProviderModelDeepseek::Deepseek__V4__Pro),
            moonshotai(ProviderModelMoonshotAi::Kimi__K2_5),
            zai(ProviderModelZai::Glm__5_1),
            openrouter(ProviderModelOpenRouter::Anthropic___Claude__Opus__4_5),
            ollama("llama3.1"),
        ],
        ModelCapability::SmartCheap => vec![
            openai(ProviderModelOpenAi::Gpt__5_2),
            gemini(ProviderModelGemini::Gemini__2_5__Pro),
            zai(ProviderModelZai::Glm__5),
            anthropic(ProviderModelAnthropic::Claude__Sonnet__4__5__20250929),
            openrouter(ProviderModelOpenRouter::Openai___Gpt__5_2),
            ollama("llama3.1"),
        ],
        ModelCapability::SmartThink => vec![
            openai(ProviderModelOpenAi::O3),
            deepseek(ProviderModelDeepseek::Deepseek__V4__Pro),
            anthropic(ProviderModelAnthropic::Claude__Opus__4__5__20251101),
            xai("grok-4.20-0309-reasoning"),
            zai(ProviderModelZai::Glm__5_1),
            ollama("qwq"),
        ],
        ModelCapability::SmartCheapThink => vec![
            openai(ProviderModelOpenAi::O3__Mini),
            deepseek(ProviderModelDeepseek::Deepseek__V4__Flash),
            zai(ProviderModelZai::Glm__5),
            ollama("qwq"),
        ],
        ModelCapability::SmartUltrathink => vec![
            openai(ProviderModelOpenAi::O3),
            anthropic(ProviderModelAnthropic::Claude__Opus__4__5__20251101),
            deepseek(ProviderModelDeepseek::Deepseek__V4__Pro),
            xai("grok-4.20-0309-reasoning"),
            ollama("qwq"),
        ],
        ModelCapability::SmartCheapUltrathink => vec![
            openai(ProviderModelOpenAi::O3__Mini),
            deepseek(ProviderModelDeepseek::Deepseek__V4__Flash),
            zai(ProviderModelZai::Glm__5),
            ollama("qwq"),
        ],
        ModelCapability::CreativeFast => stack_for(&ModelCapability::Fast),
        ModelCapability::CreativeNormal => stack_for(&ModelCapability::Normal),
        ModelCapability::CreativeSmart => stack_for(&ModelCapability::Smart),
        ModelCapability::LiteralFast => stack_for(&ModelCapability::Fast),
        ModelCapability::LiteralNormal => stack_for(&ModelCapability::Normal),
        ModelCapability::LiteralSmart => stack_for(&ModelCapability::Smart),
        ModelCapability::Specific(model) => vec![model.clone()],
    }
}

fn anthropic(model: ProviderModelAnthropic) -> ProviderModel {
    ProviderModel::Anthropic(model)
}

fn deepseek(model: ProviderModelDeepseek) -> ProviderModel {
    ProviderModel::Deepseek(model)
}

fn gemini(model: ProviderModelGemini) -> ProviderModel {
    ProviderModel::Gemini(model)
}

fn groq(model: ProviderModelGroq) -> ProviderModel {
    ProviderModel::Groq(model)
}

fn mistral(model: ProviderModelMistral) -> ProviderModel {
    ProviderModel::Mistral(model)
}

fn moonshotai(model: ProviderModelMoonshotAi) -> ProviderModel {
    ProviderModel::MoonshotAi(model)
}

fn openai(model: ProviderModelOpenAi) -> ProviderModel {
    ProviderModel::OpenAi(model)
}

fn openrouter(model: ProviderModelOpenRouter) -> ProviderModel {
    ProviderModel::OpenRouter(model)
}

fn xai(model_id: &str) -> ProviderModel {
    ProviderModel::Xai(ProviderModelXai::Bespoke(model_id.to_string()))
}

fn zai(model: ProviderModelZai) -> ProviderModel {
    ProviderModel::Zai(model)
}

fn ollama(model_id: &str) -> ProviderModel {
    ProviderModel::Ollama(ProviderModelOllama::Bespoke(model_id.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rigging::providers::Provider;

    #[test]
    fn test_xai_defaults_are_in_generated_catalog() {
        let known: std::collections::HashSet<&str> = ProviderModelXai::ALL
            .iter()
            .map(ProviderModelXai::model_id)
            .collect();
        let capabilities = [
            ModelCapability::FastCheap,
            ModelCapability::Fast,
            ModelCapability::Normal,
            ModelCapability::NormalCheap,
            ModelCapability::NormalThinking,
            ModelCapability::NormalThinkingCheap,
            ModelCapability::NormalUltrathink,
            ModelCapability::NormalCheapUltrathink,
            ModelCapability::Smart,
            ModelCapability::SmartCheap,
            ModelCapability::SmartThink,
            ModelCapability::SmartCheapThink,
            ModelCapability::SmartUltrathink,
            ModelCapability::SmartCheapUltrathink,
            ModelCapability::CreativeFast,
            ModelCapability::CreativeNormal,
            ModelCapability::CreativeSmart,
            ModelCapability::LiteralFast,
            ModelCapability::LiteralNormal,
            ModelCapability::LiteralSmart,
        ];

        for capability in capabilities {
            for model in stack_for(&capability) {
                if let ProviderModel::Xai(model) = model {
                    assert!(
                        known.contains(model.model_id()),
                        "{capability:?} selects xAI model `{}` missing from generated catalog",
                        model.model_id()
                    );
                }
            }
        }
    }

    #[test]
    fn test_resolver_prefers_first_runnable_model() {
        let env = HashMapEnvView::new([("OPENAI_API_KEY", "sk-test")]);
        let model = resolve_model(&ModelCapability::Normal,&env,
        )
        .expect("expected a runnable model");

        assert_eq!(model.provider(), Provider::OpenAi);
        assert_eq!(model.model_id(), "gpt-5.2");
    }

    #[test]
    fn test_resolver_falls_through_stack() {
        // No OpenAI key, so the first OpenAI entry is skipped; Gemini is next.
        let env = HashMapEnvView::new([("GEMINI_API_KEY", "sk-test")]);
        let model = resolve_model(&ModelCapability::Normal,&env,
        )
        .expect("expected a runnable model");

        assert_eq!(model.provider(), Provider::Gemini);
        assert_eq!(model.model_id(), "gemini-2.5-pro");
    }

    #[test]
    fn test_local_provider_is_always_runnable() {
        let env = HashMapEnvView::default();
        let model = resolve_model(&ModelCapability::Fast,&env,
        )
        .expect("expected a runnable model");

        assert_eq!(model.provider(), Provider::Ollama);
    }

    #[test]
    fn test_specific_model_requires_runnable_provider() {
        let model = ProviderModel::OpenAi(ProviderModelOpenAi::O3);
        let capability = ModelCapability::Specific(model.clone());

        let env = HashMapEnvView::new([("OPENAI_API_KEY", "sk-test")]);
        let resolved = resolve_model(&capability, &env).unwrap();
        assert_eq!(resolved, model);

        let env = HashMapEnvView::default();
        let result = resolve_model(&capability, &env);
        assert!(matches!(result, Err(ProviderError::NoRunnableModel { .. })));
    }

    #[test]
    fn test_no_runnable_model_returns_error() {
        let env = HashMapEnvView::default();
        let model = ProviderModel::OpenAi(ProviderModelOpenAi::O3);
        let result = resolve_model(&ModelCapability::Specific(model), &env);
        assert!(matches!(
            result,
            Err(ProviderError::NoRunnableModel { .. })
        ));
    }

    #[test]
    fn test_empty_env_value_is_not_runnable() {
        let mut env = HashMapEnvView::default();
        env.set("OPENAI_API_KEY", "   ");
        let model = ProviderModel::OpenAi(ProviderModelOpenAi::O3);
        let result = resolve_model(&ModelCapability::Specific(model), &env);
        assert!(matches!(
            result,
            Err(ProviderError::NoRunnableModel { .. })
        ));
    }
}
