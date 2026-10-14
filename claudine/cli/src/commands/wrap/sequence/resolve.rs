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
pub(super) fn is_auto_selectable_state(
    state: &claudine::composition::AgentResolutionState,
) -> bool {
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
/// resolves through the shared [`resolve_document_model`] under the caller's
/// dry-run mode (compiled baseline, no listing refresh), matching the direct
/// compose path.
///
/// Sequence steps share one source frontmatter `agent` hint, so the same
/// target applies to every step.
pub(super) fn dry_run_sequence_target(
    explicit_provider: Option<claudine::provider::Provider>,
    raw_hints: &claudine::composition::EffectiveSelectionHints,
    snapshot: &claudine::composition::InstalledProviderSnapshot,
    cli_model: Option<&str>,
    catalog: &claudine::model_catalog::ModelCatalogService,
    mode: super::super::composition::ModelResolveMode,
    environment: &claudine::invocation_context::DocumentEnvironment,
) -> Option<claudine::composition::ResolvedExecutionTarget> {
    use claudine::composition::{
        AgentResolutionState, ProviderResolutionReason, ResolvedExecutionTarget,
        classify_agent_resolution,
    };

    use super::super::composition::resolve_document_model;

    if let Some(provider) = explicit_provider {
        let (model, model_reason) =
            resolve_document_model(catalog, provider, raw_hints, cli_model, mode, environment);
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
    let (model, model_reason) =
        resolve_document_model(catalog, provider, raw_hints, cli_model, mode, environment);
    Some(ResolvedExecutionTarget {
        provider,
        provider_reason,
        model,
        model_reason,
    })
}

/// What a step's planned target is re-selected from when the document that
/// runs it enters: the sequence's selection hints (with `--set` applied), its
/// model catalog, and `--model`.
#[derive(Clone, Copy)]
pub(super) struct StepSelection<'a> {
    pub(super) raw_hints: &'a claudine::composition::EffectiveSelectionHints,
    pub(super) catalog: &'a claudine::model_catalog::ModelCatalogService,
    pub(super) cli_model: Option<&'a str>,
}

impl StepSelection<'_> {
    /// `planned` as the document captured under `environment` runs it.
    ///
    /// Preflight plans every step's target under the launch map, before any
    /// later document has captured its own. The provider never depends on the
    /// environment, so it stands; the model's environment steps (the
    /// provider-specific variables, then `MODEL`) are re-read from the
    /// document's map, and the re-selected model is the one that runs. A
    /// model the user confirmed on the review screen, or passed as `--model`,
    /// is a decision rather than a reading of the environment and stands too.
    pub(super) fn for_document(
        &self,
        planned: claudine::composition::ResolvedExecutionTarget,
        environment: &claudine::invocation_context::DocumentEnvironment,
    ) -> claudine::composition::ResolvedExecutionTarget {
        use claudine::composition::ModelResolutionReason;

        if matches!(
            planned.model_reason,
            ModelResolutionReason::ExplicitCli | ModelResolutionReason::SequenceReview
        ) {
            return planned;
        }
        let (model, model_reason) = super::super::composition::resolve_document_model(
            self.catalog,
            planned.provider,
            self.raw_hints,
            self.cli_model,
            super::super::composition::ModelResolveMode::REBUILD,
            environment,
        );
        claudine::composition::ResolvedExecutionTarget {
            model,
            model_reason,
            ..planned
        }
    }
}
