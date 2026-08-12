//! Per-step target resolution helpers for the sequence orchestrator.
//!
//! These mirror the direct-compose agent-resolution path
//! ([`super::super::composition`]) so a sequence resolves its shared
//! document-level `agent` hint exactly like `compose` does.

use color_eyre::eyre::{Result, WrapErr, eyre};

use claudine::composition;

/// Apply user `--set` overrides to raw selection hints.
///
/// Frontmatter `agent`/`model` can be overridden at the top level of
/// `--set`. The override is interpreted with the same parsing rules
/// (`agent` may be a string or array of strings; `model` may be a string
/// or array of strings).
pub(super) fn apply_user_set_to_hints(
    mut hints: claudine::composition::EffectiveSelectionHints,
    user_set_overrides: Option<&serde_json::Value>,
) -> Result<claudine::composition::EffectiveSelectionHints> {
    let Some(serde_json::Value::Object(map)) = user_set_overrides else {
        return Ok(hints);
    };

    if let Some(agent_value) = map.get("agent") {
        let mut fm = darkmatter::markdown::Frontmatter::new();
        fm.insert("agent", agent_value.clone())
            .wrap_err("invalid --set agent")?;
        let parsed = composition::parse_selection_hints_from_frontmatter(&fm)?;
        hints.agent = parsed.agent;
    }
    if let Some(model_value) = map.get("model") {
        let mut fm = darkmatter::markdown::Frontmatter::new();
        fm.insert("model", model_value.clone())
            .wrap_err("invalid --set model")?;
        let parsed = composition::parse_selection_hints_from_frontmatter(&fm)?;
        hints.model = parsed.model;
    }

    Ok(hints)
}

/// Whether a classified agent state resolves to a provider without prompting.
///
/// Only [`Selected`](claudine::composition::AgentResolutionState::Selected) and
/// [`ListOneInstalled`](claudine::composition::AgentResolutionState::ListOneInstalled)
/// auto-select; every other state is a prompting state that must show the
/// picker on a TTY or abort in a no-TTY session. Mirrors the selected-provider
/// branch of the direct compose path's `resolve_live_target_with_tty`.
pub(super) fn is_auto_selectable_state(state: &claudine::composition::AgentResolutionState) -> bool {
    use claudine::composition::AgentResolutionState;
    matches!(
        state,
        AgentResolutionState::Selected { .. } | AgentResolutionState::ListOneInstalled { .. }
    )
}

/// Resolve the per-step execution target for a `--dry-run` sequence.
///
/// Mirrors the dry-run branch of
/// [`eagerly_resolve_target`](super::super::composition::eagerly_resolve_target):
/// an explicit `--<provider>` flag and the auto-selectable frontmatter
/// states ([`Selected`](claudine::composition::AgentResolutionState::Selected),
/// [`ListOneInstalled`](claudine::composition::AgentResolutionState::ListOneInstalled))
/// resolve to a concrete target so `{{env.AGENT}}` interpolates and the
/// model resolves; every other agent-resolution state returns `None` so the
/// per-step dry-run seam renders it from the installed snapshot. The model
/// catalog is never consulted under `--dry-run` (catalog `None`), matching
/// the direct compose path.
///
/// Sequence steps share one source frontmatter `agent` hint, so the same
/// target applies to every step.
pub(super) fn dry_run_sequence_target(
    explicit_provider: Option<claudine::provider::Provider>,
    raw_hints: &claudine::composition::EffectiveSelectionHints,
    snapshot: &claudine::composition::InstalledProviderSnapshot,
    cli_model: Option<&str>,
) -> Option<claudine::composition::ResolvedExecutionTarget> {
    use claudine::composition::{
        AgentResolutionState, ProviderResolutionReason, ResolvedExecutionTarget,
        classify_agent_resolution, resolve_model_with_hints,
    };

    if let Some(provider) = explicit_provider {
        let (model, model_reason) = resolve_model_with_hints(provider, raw_hints, cli_model, None);
        return Some(ResolvedExecutionTarget {
            provider,
            provider_reason: ProviderResolutionReason::ExplicitFlag,
            model,
            model_reason,
        });
    }

    let (provider, provider_reason) = match classify_agent_resolution(raw_hints, snapshot) {
        AgentResolutionState::Selected { provider } => {
            (provider, ProviderResolutionReason::FrontmatterSingle)
        }
        AgentResolutionState::ListOneInstalled { selected, .. } => {
            (selected, ProviderResolutionReason::FrontmatterList)
        }
        _ => return None,
    };
    let (model, model_reason) = resolve_model_with_hints(provider, raw_hints, cli_model, None);
    Some(ResolvedExecutionTarget {
        provider,
        provider_reason,
        model,
        model_reason,
    })
}
