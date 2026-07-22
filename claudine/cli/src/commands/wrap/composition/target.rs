//! Provider/target resolution for the composition executor.
//!
//! Resolves the execution target (provider + model) before composition
//! templates are rendered so `{{env.AGENT}}` resolves to the chosen
//! provider, applies the TTY-only agent picker gate, and assembles the
//! dispatch context lifted from the resolved target. Shared with the
//! `sequence` orchestrator through the `composition::` re-exports.

use std::collections::HashMap;
use std::io::IsTerminal;

use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::prelude::TerminalRenderable;
use claudine::composition::{
    AgentResolutionState, CompositionError, CompositionExecutionRequest, CompositionMode,
    ModelResolutionReason, ResolvedExecutionTarget, agent_state_breakdown,
    build_installed_snapshot, classify_agent_resolution, detect_installed_providers,
    invalid_agent_message,
};
use claudine::provider::Provider;
use color_eyre::eyre::{Result, WrapErr, eyre};
use sniff::programs::InstalledAiClients;

use super::super::wrap_terminal;
use super::{CompositionPrepContext, load_selection_config};
use crate::log;

pub(crate) fn composition_dispatch_context(
    request: &CompositionExecutionRequest,
    target: &ResolvedExecutionTarget,
) -> HashMap<String, serde_json::Value> {
    let mut context = HashMap::new();
    context.insert(
        "composition_file_ref".into(),
        serde_json::Value::String(request.file_ref.clone()),
    );
    context.insert(
        "composition_mode".into(),
        serde_json::Value::String(match request.mode {
            CompositionMode::InlineFrontmatterPrompt => "inline".to_string(),
            CompositionMode::ChainedDocument => "compose".to_string(),
        }),
    );
    context.insert(
        "composition_source_path".into(),
        serde_json::Value::String(request.prepared.resolved_path.display().to_string()),
    );
    context.insert(
        "provider_selection_reason".into(),
        serde_json::Value::String(format!("{:?}", target.provider_reason)),
    );
    context.insert(
        "resolved_model".into(),
        serde_json::Value::String(target.model.clone().unwrap_or_default()),
    );
    context.insert(
        "model_selection_reason".into(),
        serde_json::Value::String(format!("{:?}", target.model_reason)),
    );
    context.insert(
        "selection_mode".into(),
        serde_json::Value::String(
            if std::io::stdin().is_terminal() && std::io::stdout().is_terminal() {
                "tty"
            } else {
                "non-tty"
            }
            .to_string(),
        ),
    );
    context
}

/// Resolve the execution target for a composition request, reusing an
/// upstream-resolved target when present and otherwise running the full
/// provider/config/catalog discovery and TTY-gated picker.
///
/// This is the executor's target-resolution stage. When
/// `request.resolved_target` is already set (sequence review, non-TTY
/// preflight, eager resolution via `CompositionPrepContext`) it is reused
/// verbatim, skipping a duplicate installed-client PATH scan and a
/// redundant `load_selection_config`.
pub(crate) fn resolve_execution_target(
    request: &CompositionExecutionRequest,
    launch_cwd: &std::path::Path,
    source_repo_root: Option<&std::path::Path>,
) -> Result<ResolvedExecutionTarget> {
    if let Some(ref t) = request.resolved_target {
        return Ok(t.clone());
    }

    let snapshot = match request.installed_snapshot {
        Some(ref s) => s.clone(),
        None => {
            let clients = InstalledAiClients::new();
            let installed = detect_installed_providers(&clients);
            build_installed_snapshot(&installed, &request.excluded, &clients)
        }
    };

    let selection_config = load_selection_config(source_repo_root.unwrap_or(launch_cwd));
    let catalog = match &selection_config {
        Some(cfg) => claudine::model_catalog::ModelCatalogService::with_overrides(
            cfg.model_overrides.clone(),
        ),
        None => claudine::model_catalog::ModelCatalogService::new(),
    };
    let favorite = selection_config.as_ref().and_then(|c| c.favorite);

    if let Some(provider) = request.explicit_provider {
        let (_, probe_reason) = claudine::composition::resolve_model_with_catalog(
            provider,
            &request.prepared,
            request.model.as_deref(),
            None,
        );
        refresh_for_prepared_model_validation(
            &catalog,
            provider,
            &request.prepared,
            Some(&probe_reason),
        );
        let (model, model_reason) = claudine::composition::resolve_model_with_catalog(
            provider,
            &request.prepared,
            request.model.as_deref(),
            Some(&catalog),
        );
        return Ok(ResolvedExecutionTarget {
            provider,
            provider_reason: claudine::composition::ProviderResolutionReason::ExplicitFlag,
            model,
            model_reason,
        });
    }

    let state = classify_agent_resolution(&request.prepared.selection_hints, &snapshot);
    let selected_provider = match state {
        AgentResolutionState::Selected { provider } => Some(provider),
        AgentResolutionState::ListOneInstalled { selected, .. } => Some(selected),
        _ => None,
    };

    if let Some(provider) = selected_provider {
        let (_, probe_reason) = claudine::composition::resolve_model_with_catalog(
            provider,
            &request.prepared,
            request.model.as_deref(),
            None,
        );
        refresh_for_prepared_model_validation(
            &catalog,
            provider,
            &request.prepared,
            Some(&probe_reason),
        );
        let (model, model_reason) = claudine::composition::resolve_model_with_catalog(
            provider,
            &request.prepared,
            request.model.as_deref(),
            Some(&catalog),
        );
        let provider_reason = match state {
            AgentResolutionState::Selected { .. } => {
                claudine::composition::ProviderResolutionReason::FrontmatterSingle
            }
            _ => claudine::composition::ProviderResolutionReason::FrontmatterList,
        };
        return Ok(ResolvedExecutionTarget {
            provider,
            provider_reason,
            model,
            model_reason,
        });
    }

    let is_tty = std::io::stderr().is_terminal();
    if is_tty {
        let provider = prompt_for_agent_state(
            &state,
            &request.prepared.selection_hints,
            &snapshot,
            favorite,
            &request.prepared.resolved_path,
        )?;
        let (_, probe_reason) = claudine::composition::resolve_model_with_catalog(
            provider,
            &request.prepared,
            request.model.as_deref(),
            None,
        );
        refresh_for_prepared_model_validation(
            &catalog,
            provider,
            &request.prepared,
            Some(&probe_reason),
        );
        let (model, model_reason) = claudine::composition::resolve_model_with_catalog(
            provider,
            &request.prepared,
            request.model.as_deref(),
            Some(&catalog),
        );
        Ok(ResolvedExecutionTarget {
            provider,
            provider_reason: claudine::composition::ProviderResolutionReason::InteractivePicker,
            model,
            model_reason,
        })
    } else {
        Err(CompositionError::AgentResolutionFailed {
            source_path: request.prepared.resolved_path.clone(),
            state,
            installed: snapshot.runnable.clone(),
        }
        .into())
    }
}

/// Resolve the execution target *before* composition templates are
/// rendered, so `{{env.AGENT}}` in the body or inline `prompt` resolves
/// to the chosen provider.
///
/// This is the Phase 3 live-path entry point: explicit flag wins, then
/// the frontmatter `agent` hint is classified and acted on according to
/// the TTY-only gate. In `--dry-run` mode the function returns
/// `Ok(None)` for any unresolved state so the dry-run renderer can
/// report it without prompting; for auto-selectable states it still
/// returns the selected target so `AGENT` interpolation works.
pub(crate) fn eagerly_resolve_target(
    ctx: &CompositionPrepContext,
    hints: &claudine::composition::EffectiveSelectionHints,
    explicit_provider: Option<Provider>,
    cli_model: Option<&str>,
    dry_run: bool,
    source_path: &std::path::Path,
) -> Result<Option<ResolvedExecutionTarget>> {
    // Phase 2 (2026-05-09-slow-prep): the installed-provider snapshot and
    // selection config are pre-built on the shared `CompositionPrepContext`
    // so this function no longer rediscovers the source repo root, reloads
    // the claudine config, or re-runs host detection.
    let snapshot = &ctx.installed_snapshot;
    let selection_config = ctx.selection_config.as_ref();
    let catalog = match selection_config {
        Some(cfg) => claudine::model_catalog::ModelCatalogService::with_overrides(
            cfg.model_overrides.clone(),
        ),
        None => claudine::model_catalog::ModelCatalogService::new(),
    };
    // Phase 1 (2026-05-09-slow-prep): no global `refresh_blocking()` here.
    // Catalog refresh is deferred until *after* a provider is selected and
    // is scoped to that provider only. Provider selection itself never
    // touches the catalog.
    let favorite = selection_config.and_then(|c| c.favorite);

    if let Some(provider) = explicit_provider {
        // Probe model resolution without catalog to determine whether
        // an env var override makes refresh unnecessary.
        let (_, probe_reason) =
            claudine::composition::resolve_model_with_hints(provider, hints, cli_model, None);
        if !dry_run {
            refresh_for_model_validation(&catalog, provider, hints, Some(&probe_reason));
        }
        let catalog_ref = if dry_run { None } else { Some(&catalog) };
        let (model, model_reason) =
            claudine::composition::resolve_model_with_hints(provider, hints, cli_model, catalog_ref);
        return Ok(Some(ResolvedExecutionTarget {
            provider,
            provider_reason: claudine::composition::ProviderResolutionReason::ExplicitFlag,
            model,
            model_reason,
        }));
    }

    let state = classify_agent_resolution(hints, snapshot);

    if dry_run {
        // Dry-run only needs a concrete target when the state auto-selects.
        // Otherwise the renderer reports the unresolved state.
        let (provider, provider_reason) = match state {
            AgentResolutionState::Selected { provider } => (
                provider,
                claudine::composition::ProviderResolutionReason::FrontmatterSingle,
            ),
            AgentResolutionState::ListOneInstalled { selected, .. } => (
                selected,
                claudine::composition::ProviderResolutionReason::FrontmatterList,
            ),
            _ => return Ok(None),
        };
        let (model, model_reason) =
            claudine::composition::resolve_model_with_hints(provider, hints, cli_model, None);
        return Ok(Some(ResolvedExecutionTarget {
            provider,
            provider_reason,
            model,
            model_reason,
        }));
    }

    resolve_live_target(state, hints, snapshot, favorite, cli_model, &catalog, source_path)
        .map(Some)
}

/// Resolve a classified agent state for a live (non-dry-run) run.
///
/// Applies the TTY-only gate per the Phase 3 spec:
/// - auto-selectable states return the target directly,
/// - TTY prompting states show the scoped picker (and any required
///   pre-prompt message),
/// - no-TTY prompting states abort with a structured error.
fn resolve_live_target(
    state: AgentResolutionState,
    hints: &claudine::composition::EffectiveSelectionHints,
    snapshot: &claudine::composition::InstalledProviderSnapshot,
    favorite: Option<Provider>,
    cli_model: Option<&str>,
    catalog: &claudine::model_catalog::ModelCatalogService,
    source_path: &std::path::Path,
) -> Result<ResolvedExecutionTarget> {
    resolve_live_target_with_tty(
        state,
        hints,
        snapshot,
        favorite,
        cli_model,
        catalog,
        source_path,
        std::io::stderr().is_terminal(),
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn resolve_live_target_with_tty(
    state: AgentResolutionState,
    hints: &claudine::composition::EffectiveSelectionHints,
    snapshot: &claudine::composition::InstalledProviderSnapshot,
    favorite: Option<Provider>,
    cli_model: Option<&str>,
    catalog: &claudine::model_catalog::ModelCatalogService,
    source_path: &std::path::Path,
    is_tty: bool,
) -> Result<ResolvedExecutionTarget> {
    use claudine::composition::ProviderResolutionReason;

    // The non-TTY gate is the shared authority for both branches: its `Ok` is the
    // auto-selectable state, and its `Err` is exactly what a no-operator session
    // aborts with. Deriving both from one call is what keeps the picker-less
    // rebuild in `target_launch` from drifting away from this route.
    let selected_provider = provider_for_state_non_tty(&state, snapshot, source_path);

    if let Ok(&provider) = selected_provider.as_ref() {
        let (_, probe_reason) =
            claudine::composition::resolve_model_with_hints(provider, hints, cli_model, None);
        refresh_for_model_validation(catalog, provider, hints, Some(&probe_reason));
        let (model, model_reason) =
            claudine::composition::resolve_model_with_hints(provider, hints, cli_model, Some(catalog));
        let provider_reason = match state {
            AgentResolutionState::Selected { .. } => ProviderResolutionReason::FrontmatterSingle,
            _ => ProviderResolutionReason::FrontmatterList,
        };
        return Ok(ResolvedExecutionTarget {
            provider,
            provider_reason,
            model,
            model_reason,
        });
    }

    if is_tty {
        let provider =
            prompt_for_agent_state(&state, hints, snapshot, favorite, source_path)?;
        let (_, probe_reason) =
            claudine::composition::resolve_model_with_hints(provider, hints, cli_model, None);
        refresh_for_model_validation(catalog, provider, hints, Some(&probe_reason));
        let (model, model_reason) =
            claudine::composition::resolve_model_with_hints(provider, hints, cli_model, Some(catalog));
        Ok(ResolvedExecutionTarget {
            provider,
            provider_reason: ProviderResolutionReason::InteractivePicker,
            model,
            model_reason,
        })
    } else {
        Err(selected_provider
            .expect_err("the Ok arm returned above")
            .into())
    }
}

/// The provider a classified agent state resolves to with no operator present.
///
/// Non-TTY selection has no picker to fall through to, so every state that would
/// have prompted is the route's typed [`CompositionError::AgentResolutionFailed`]
/// carrying the state itself — `SingleNotInstalled` for an unavailable scalar,
/// `ZeroInstalledList` for a list with no runnable member.
///
/// Shared with the per-attempt launch rebuild
/// (`harness_orch::loop_control::target_launch`): a retry or resume has no
/// operator either, so routing its refreshed `agent:` through this same gate is
/// what makes a refreshed document select — and refuse — exactly as a direct
/// non-interactive invocation of that document does (review-11 finding 2).
/// Before it did, the rebuild accepted any scalar and substituted a bare
/// executable name, so an unavailable provider failed at spawn instead.
#[allow(clippy::result_large_err)]
pub(crate) fn provider_for_state_non_tty(
    state: &AgentResolutionState,
    snapshot: &claudine::composition::InstalledProviderSnapshot,
    source_path: &std::path::Path,
) -> Result<Provider, CompositionError> {
    match state {
        AgentResolutionState::Selected { provider } => Ok(*provider),
        AgentResolutionState::ListOneInstalled { selected, .. } => Ok(*selected),
        _ => Err(CompositionError::AgentResolutionFailed {
            source_path: source_path.to_path_buf(),
            state: state.clone(),
            installed: snapshot.runnable.clone(),
        }),
    }
}

/// The styled pre-prompt message a TTY run shows before the `choose_one`
/// picker, or `None` for states that go straight to the picker.
///
/// The text is the single source of truth shared with the dry-run table cell
/// and the no-TTY abort body (via [`agent_state_breakdown`] /
/// [`invalid_agent_message`]) so the three surfaces cannot drift. Returns
/// Prose **markup**; callers render it with their own terminal.
///
/// The `sequence` orchestrator reuses this before its review screen so an
/// invalid-scalar or zero-installed-list sequence shows the same pre-prompt
/// message direct compose shows and the dry-run table predicts.
pub(crate) fn agent_prompt_message(
    state: &AgentResolutionState,
    source_path: &std::path::Path,
) -> Option<String> {
    match state {
        AgentResolutionState::SingleInvalid { hint } => {
            let file_href = format!("file://{}", source_path.display());
            let file_label = source_path.display().to_string();
            let file_link = format!("<a href=\"{file_href}\">{file_label}</a>");
            Some(invalid_agent_message(hint, &file_link))
        }
        AgentResolutionState::ZeroInstalledList { .. } => Some(agent_state_breakdown(state)),
        _ => None,
    }
}

/// Which installed providers the picker is scoped to for a given state.
///
/// Only [`AgentResolutionState::ListMultipleInstalled`] narrows the picker to
/// its suggested installed providers; every other prompting state offers all
/// installed agents (`None`), matching the spec's per-state scoping rules.
pub(crate) fn picker_scope_for_state(state: &AgentResolutionState) -> Option<&[Provider]> {
    match state {
        AgentResolutionState::ListMultipleInstalled { installed, .. } => Some(installed.as_slice()),
        _ => None,
    }
}

/// Build the picker plan for a prompting state, applying the per-state scope.
///
/// Pure (no I/O): the live path and L1 tests both go through here so the
/// scope contract is verifiable without a TTY. The `sequence` review screen
/// reuses this so its picker narrows `ListMultipleInstalled` to the same
/// installed-from-list subset the direct compose picker offers.
#[allow(clippy::result_large_err)]
pub(crate) fn scoped_picker_plan_for_state(
    state: &AgentResolutionState,
    hints: &claudine::composition::EffectiveSelectionHints,
    snapshot: &claudine::composition::InstalledProviderSnapshot,
    favorite: Option<Provider>,
) -> Result<claudine::composition::ProviderPickerPlan, CompositionError> {
    build_scoped_picker_plan(hints, snapshot, favorite, picker_scope_for_state(state))
}

/// Emit the pre-prompt message for TTY states that require one, then
/// show the `choose_one` picker and return the user-selected provider.
fn prompt_for_agent_state(
    state: &AgentResolutionState,
    hints: &claudine::composition::EffectiveSelectionHints,
    snapshot: &claudine::composition::InstalledProviderSnapshot,
    favorite: Option<Provider>,
    source_path: &std::path::Path,
) -> Result<Provider> {
    if let Some(markup) = agent_prompt_message(state, source_path) {
        let term = wrap_terminal();
        log::message(&Prose::new(markup).render(&term));
    }

    let plan = scoped_picker_plan_for_state(state, hints, snapshot, favorite)?;
    super::super::selection_ui::prompt_one_shot_provider(plan)
        .wrap_err("provider selection cancelled")
}

/// Build a picker plan scoped to a subset of installed providers.
///
/// When `scope` is `Some`, only providers in that list are shown. The
/// ordering and default index are inherited from the unscoped plan so
/// frontmatter/favorite influence is preserved.
#[allow(clippy::result_large_err)]
fn build_scoped_picker_plan(
    hints: &claudine::composition::EffectiveSelectionHints,
    snapshot: &claudine::composition::InstalledProviderSnapshot,
    favorite: Option<Provider>,
    scope: Option<&[Provider]>,
) -> Result<claudine::composition::ProviderPickerPlan, CompositionError> {
    let mut plan =
        claudine::composition::build_picker_plan_with_hints(hints, snapshot, favorite)?;

    if let Some(scope) = scope {
        let scope_set: std::collections::BTreeSet<Provider> = scope.iter().copied().collect();
        plan.options.retain(|o| scope_set.contains(&o.provider));
        if plan.options.is_empty() {
            return Err(CompositionError::NoRunnableProviders);
        }
        plan.default_index = plan.default_index.min(plan.options.len() - 1);
    }

    Ok(plan)
}

/// Refresh a single provider's dynamic listing only when a frontmatter
/// `model` hint is in play.
///
/// Validation itself is baseline-fed (expected offerings) and never
/// reads the listing; the refresh keeps the on-disk listing cache warm
/// as drift-channel input, scoped to the runs that exercise model
/// validation. CLI `--model`, provider-specific environment variables,
/// and the generic `MODEL` env var all win over the frontmatter hint,
/// so when one of those is supplied we skip the refresh entirely.
pub(crate) fn refresh_for_model_validation(
    catalog: &claudine::model_catalog::ModelCatalogService,
    provider: Provider,
    hints: &claudine::composition::EffectiveSelectionHints,
    resolved_model_reason: Option<&ModelResolutionReason>,
) {
    if hints.model.is_none() {
        return;
    }
    if matches!(
        resolved_model_reason,
        Some(
            ModelResolutionReason::ExplicitCli
                | ModelResolutionReason::ProviderEnv(_)
                | ModelResolutionReason::GenericEnv
        )
    ) {
        return;
    }
    let _span =
        tracing::info_span!("compose_prep.model_catalog", provider = %provider.as_slug()).entered();
    // W3: always non-blocking — the current run never needs the
    // subprocess result (`opencode models` feeds the drift channel, not
    // validation), so the refresh detaches even on a cold cache.
    catalog.refresh_provider_async(provider);
}

/// Same gating as [`refresh_for_model_validation`] but reads the
/// `model` hint from a fully prepared composition.
fn refresh_for_prepared_model_validation(
    catalog: &claudine::model_catalog::ModelCatalogService,
    provider: Provider,
    prepared: &claudine::composition::PreparedComposition,
    resolved_model_reason: Option<&ModelResolutionReason>,
) {
    refresh_for_model_validation(
        catalog,
        provider,
        &prepared.selection_hints,
        resolved_model_reason,
    );
}

/// Inject `AGENT` into the supplied `env_overrides` map so composition
/// templates and downstream system-prompt rendering see the chosen provider's
/// slug. The wrapper no longer mutates the parent process env for AGENT;
/// child processes receive it through `build_child_env_with_launch` and
/// composition contexts receive it through `env_overrides`.
pub(crate) fn install_agent_env_for_composition(
    target: &ResolvedExecutionTarget,
    yolo: bool,
    env_overrides: &mut std::collections::BTreeMap<String, String>,
) {
    let slug = target.provider.as_slug().to_string();
    env_overrides.insert("AGENT".to_string(), slug);
    if let Some(ref model) = target.model {
        env_overrides.insert("MODEL".to_string(), model.clone());
    }
    env_overrides.insert("YOLO".to_string(), yolo.to_string());
}
