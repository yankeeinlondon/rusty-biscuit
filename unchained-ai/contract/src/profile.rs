//! Profile-driven model selection and reasoning-parameter translation.
//!
//! Converts a provider-neutral [`InferenceProfile`] into a
//! [`ModelCapability`], resolves that capability to a concrete [`ProviderModel`]
//! using an injected [`EnvView`], and best-effort translates
//! [`ReasoningEffort`] into a provider-specific generation parameter when the
//! model's metadata advertises support.

use biscuit_contract::inference::{InferenceProfile, InferencePriority, ReasoningEffort};
use unchained_ai::execution::ResolvedParameters;
use unchained_ai::models::model_capability::ModelCapability;
use unchained_ai::models::selection::{EnvView, resolve_model};
use unchained_ai::rigging::providers::models::ProviderModel;
use unchained_ai::rigging::providers::provider_errors::ProviderError;

/// Maps an [`InferenceProfile`] to the capability stack that best approximates
/// its intent.
///
/// Reasoning effort selects a `*Thinking` or `*Ultrathink` variant when one
/// exists; otherwise it is dropped to the nearest available capability (e.g.
/// `Latency` always maps to `Fast` because no thinking variants exist in that
/// tier).
pub(crate) fn profile_to_capability(profile: InferenceProfile) -> ModelCapability {
    match profile.priority {
        InferencePriority::Cost => match profile.reasoning {
            ReasoningEffort::None => ModelCapability::NormalCheap,
            ReasoningEffort::Low | ReasoningEffort::Medium => ModelCapability::NormalThinkingCheap,
            ReasoningEffort::High => ModelCapability::NormalCheapUltrathink,
        },
        InferencePriority::Latency => {
            // No thinking variants exist in the Fast tier; drop reasoning.
            let _ = profile.reasoning;
            ModelCapability::Fast
        }
        InferencePriority::Quality => match profile.reasoning {
            ReasoningEffort::None => ModelCapability::Smart,
            ReasoningEffort::Low => ModelCapability::SmartCheapThink,
            ReasoningEffort::Medium => ModelCapability::SmartThink,
            ReasoningEffort::High => ModelCapability::SmartUltrathink,
        },
        InferencePriority::Balanced => match profile.reasoning {
            ReasoningEffort::None => ModelCapability::Normal,
            ReasoningEffort::Low | ReasoningEffort::Medium => ModelCapability::NormalThinking,
            ReasoningEffort::High => ModelCapability::NormalUltrathink,
        },
    }
}

/// Resolves the concrete model for a profile using the supplied environment
/// view.
pub(crate) fn resolve_profile_model(
    profile: InferenceProfile,
    env: &dyn EnvView,
) -> Result<ProviderModel, ProviderError> {
    let capability = profile_to_capability(profile);
    resolve_model(&capability, env)
}

/// Best-effort translation of [`ReasoningEffort`] into provider-specific extra
/// parameters.
///
/// Only emits a parameter when the model's metadata explicitly lists a known
/// reasoning parameter (`reasoning_effort` for OpenAI-style providers,
/// `thinking` for Anthropic). Otherwise returns an empty parameter set so the
/// model's default reasoning behavior is used.
pub(crate) fn parameters_for_reasoning(
    model: &ProviderModel,
    reasoning: ReasoningEffort,
) -> ResolvedParameters {
    let mut params = ResolvedParameters::new();

    if reasoning == ReasoningEffort::None {
        return params;
    }

    let Some(metadata) = model.metadata() else {
        return params;
    };

    let Some(supported) = &metadata.supported_parameters else {
        return params;
    };

    let mut extra = serde_json::Map::new();

    if supported
        .iter()
        .any(|p| p.eq_ignore_ascii_case("reasoning_effort"))
    {
        let value = match reasoning {
            ReasoningEffort::Low => "low",
            ReasoningEffort::Medium => "medium",
            ReasoningEffort::High => "high",
            ReasoningEffort::None => return params,
        };
        extra.insert(
            "reasoning_effort".to_string(),
            serde_json::Value::String(value.to_string()),
        );
    } else if supported.iter().any(|p| p.eq_ignore_ascii_case("thinking")) {
        let budget = match reasoning {
            ReasoningEffort::Low => 1_024u32,
            ReasoningEffort::Medium => 4_096,
            ReasoningEffort::High => 16_384,
            ReasoningEffort::None => return params,
        };
        extra.insert(
            "thinking".to_string(),
            serde_json::json!({ "budget_tokens": budget }),
        );
    }

    if !extra.is_empty() {
        params.extra = Some(extra);
    }

    params
}

#[cfg(test)]
mod tests {
    use super::*;
    use unchained_ai::models::selection::HashMapEnvView;
    use unchained_ai::rigging::providers::Provider;

    #[test]
    fn cost_without_reasoning_maps_to_normal_cheap() {
        let profile = InferenceProfile {
            priority: InferencePriority::Cost,
            reasoning: ReasoningEffort::None,
        };
        assert_eq!(profile_to_capability(profile), ModelCapability::NormalCheap);
    }

    #[test]
    fn cost_with_medium_reasoning_maps_to_normal_thinking_cheap() {
        let profile = InferenceProfile {
            priority: InferencePriority::Cost,
            reasoning: ReasoningEffort::Medium,
        };
        assert_eq!(
            profile_to_capability(profile),
            ModelCapability::NormalThinkingCheap
        );
    }

    #[test]
    fn latency_drops_reasoning_to_fast() {
        let profile = InferenceProfile {
            priority: InferencePriority::Latency,
            reasoning: ReasoningEffort::High,
        };
        assert_eq!(profile_to_capability(profile), ModelCapability::Fast);
    }

    #[test]
    fn quality_with_high_reasoning_maps_to_smart_ultrathink() {
        let profile = InferenceProfile {
            priority: InferencePriority::Quality,
            reasoning: ReasoningEffort::High,
        };
        assert_eq!(
            profile_to_capability(profile),
            ModelCapability::SmartUltrathink
        );
    }

    #[test]
    fn balanced_with_low_reasoning_maps_to_normal_thinking() {
        let profile = InferenceProfile {
            priority: InferencePriority::Balanced,
            reasoning: ReasoningEffort::Low,
        };
        assert_eq!(
            profile_to_capability(profile),
            ModelCapability::NormalThinking
        );
    }

    #[test]
    fn resolve_profile_prefers_credential_backed_model() {
        let env = HashMapEnvView::new([("OPENAI_API_KEY", "sk-test")]);
        let profile = InferenceProfile {
            priority: InferencePriority::Balanced,
            reasoning: ReasoningEffort::None,
        };
        let model = resolve_profile_model(profile, &env).expect("expected a runnable model");
        assert_eq!(model.provider(), Provider::OpenAi);
    }

    #[test]
    fn resolve_profile_falls_back_to_local_provider() {
        let env = HashMapEnvView::default();
        let profile = InferenceProfile {
            priority: InferencePriority::Balanced,
            reasoning: ReasoningEffort::None,
        };
        let model = resolve_profile_model(profile, &env).expect("expected a runnable model");
        assert_eq!(model.provider(), Provider::Ollama);
    }
}
