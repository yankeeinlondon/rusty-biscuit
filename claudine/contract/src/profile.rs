//! Mapping [`InferenceProfile`] onto provider reasoning and model selection.
//!
//! The profile is a best-effort preference, never a guarantee. v1 maps
//! [`ReasoningEffort`] onto the provider's typed [`ReasoningSupport`], emitting
//! it onto argv where the provider exposes a verified non-interactive
//! config-override (e.g. Codex `-c model_reasoning_effort=…`). It honors an
//! explicit model override; absent one it lets the provider choose its own
//! default, because Claudine's static model catalog is an untiered id list with
//! no cost/latency/quality metadata to map an [`InferencePriority`] onto. See
//! the crate README for the rationale.
//!
//! [`InferencePriority`]: biscuit_contract::inference::InferencePriority
//!
//! [`InferenceProfile`]: biscuit_contract::inference::InferenceProfile

use biscuit_contract::inference::ReasoningEffort;
use claudine::provider::{ProviderInfo, ReasoningSupport};

/// A resolved reasoning control: the provider's flag/key and the chosen value.
///
/// Recorded on the [`SessionPlan`](crate::session::SessionPlan) and emitted onto argv
/// where the provider has a verified non-interactive config-override (Codex).
/// For a provider whose non-interactive reasoning wiring is not verifiable, the
/// preference is recorded but not emitted, since an unrecognized flag could
/// fail an otherwise valid session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ResolvedReasoning {
    pub flag: String,
    pub value: String,
}

/// Resolve a reasoning preference against the provider's typed capability.
///
/// `ReasoningEffort::None` disables reasoning where the provider models an
/// explicit off state and otherwise resolves to `None` (omitted). Providers
/// without a reasoning control return `None` — an approximation, not an error.
pub(crate) fn resolve_reasoning(
    info: &'static ProviderInfo,
    effort: ReasoningEffort,
) -> Option<ResolvedReasoning> {
    match &info.reasoning {
        ReasoningSupport::NamedLevels { flag, levels } => {
            let value = pick_named_level(levels, effort)?;
            Some(ResolvedReasoning {
                flag: (*flag).to_string(),
                value: value.to_string(),
            })
        }
        ReasoningSupport::NumericBudget { flag, min, max, default } => {
            let value = pick_numeric_budget(*min, *max, *default, effort);
            Some(ResolvedReasoning {
                flag: (*flag).to_string(),
                value: value.to_string(),
            })
        }
        ReasoningSupport::BinaryToggle { flag, on, off } => {
            let value = if matches!(effort, ReasoningEffort::None) { off } else { on };
            Some(ResolvedReasoning {
                flag: (*flag).to_string(),
                value: (*value).to_string(),
            })
        }
        ReasoningSupport::NotSupported
        | ReasoningSupport::NotDocumented
        | ReasoningSupport::ProviderSpecific(_) => None,
    }
}

/// Pick a named level proportional to the requested effort.
///
/// `None` → no level (off). `Low`/`Medium`/`High` map to the first, middle,
/// and last documented levels respectively, so a provider exposing
/// `["low", "medium", "high"]` round-trips and a finer-grained list still
/// spans its range.
fn pick_named_level(levels: &[&'static str], effort: ReasoningEffort) -> Option<&'static str> {
    if levels.is_empty() {
        return None;
    }
    let index = match effort {
        ReasoningEffort::None => return None,
        ReasoningEffort::Low => 0,
        ReasoningEffort::Medium => levels.len() / 2,
        ReasoningEffort::High => levels.len() - 1,
    };
    Some(levels[index])
}

/// Pick a numeric reasoning budget proportional to the requested effort.
fn pick_numeric_budget(min: u32, max: u32, default: Option<u32>, effort: ReasoningEffort) -> u32 {
    match effort {
        ReasoningEffort::None => min,
        ReasoningEffort::Low => min + (max - min) / 4,
        ReasoningEffort::Medium => default.unwrap_or(min + (max - min) / 2),
        ReasoningEffort::High => max,
    }
}
