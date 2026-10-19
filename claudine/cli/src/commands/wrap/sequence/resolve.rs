//! Selection-hint helpers for the sequence orchestrator: the caller's `--set`
//! over the sequence document's hints, and a planned target's model
//! re-selected under the map of the document that runs it. Per-task provider
//! selection is [`super::providers`].

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

/// What a step's planned target is re-selected from when the document that
/// runs it enters: the model catalog and `--model`, with the sequence's
/// selection hints (`--set` applied) for a step whose task has none recorded.
#[derive(Clone, Copy)]
pub(super) struct StepSelection<'a> {
    pub(super) raw_hints: &'a claudine::composition::EffectiveSelectionHints,
    pub(super) catalog: &'a claudine::model_catalog::ModelCatalogService,
    pub(super) cli_model: Option<&'a str>,
}

impl StepSelection<'_> {
    /// `planned` as the document captured under `environment` runs it, its
    /// model re-selected from `hints`: the task's own (or, for a body step,
    /// the sequence document's).
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
        hints: &claudine::composition::EffectiveSelectionHints,
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
            hints,
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
