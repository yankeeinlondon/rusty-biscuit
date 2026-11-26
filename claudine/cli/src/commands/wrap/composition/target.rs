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
    EffectiveSelectionHints, ModelResolutionReason, ResolvedExecutionTarget,
    agent_state_breakdown, build_installed_snapshot, classify_agent_resolution,
    detect_installed_providers, invalid_agent_message, resolve_model_with_hints_from,
};
use claudine::invocation_context::DocumentEnvironment;
use claudine::model_catalog::ModelCatalogService;
use claudine::provider::Provider;
use color_eyre::eyre::{Result, WrapErr, eyre};
use sniff::programs::InstalledAiClients;

use super::super::selection_trace::{LinkState, SelectionTrace, classified, hints_agent_value};
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
        serde_json::Value::String(biscuit_file::to_forward_slash_string(
            &request.prepared.resolved_path,
        )),
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

/// The environment of the document a composition request launches: its
/// prepared epoch's, else its invocation view's.
pub(crate) fn request_environment(
    request: &CompositionExecutionRequest,
) -> DocumentEnvironment {
    match (&request.prepared.document_epoch, &request.invocation_context) {
        (Some(epoch), _) => epoch.environment().clone(),
        (None, Some(invocation)) => invocation.document_environment().clone(),
        (None, None) => DocumentEnvironment::default(),
    }
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

    let mode = ModelResolveMode::live(request.silent);
    let environment = request_environment(request);
    if let Some(provider) = request.explicit_provider {
        let (model, model_reason) = resolve_document_model(
            &catalog,
            provider,
            &request.prepared.selection_hints,
            request.model.as_deref(),
            mode,
            &environment,
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
        let (model, model_reason) = resolve_document_model(
            &catalog,
            provider,
            &request.prepared.selection_hints,
            request.model.as_deref(),
            mode,
            &environment,
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
        let (model, model_reason) = resolve_document_model(
            &catalog,
            provider,
            &request.prepared.selection_hints,
            request.model.as_deref(),
            mode,
            &environment,
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
/// The explicit flag wins, then the frontmatter `agent` hint is classified
/// and acted on according to the TTY-only gate ([`decide_target`]).
pub(crate) fn eagerly_resolve_target(
    ctx: &CompositionPrepContext,
    hints: &claudine::composition::EffectiveSelectionHints,
    explicit_provider: Option<Provider>,
    cli_model: Option<&str>,
    silent: bool,
    source_path: &std::path::Path,
) -> Result<Option<ResolvedExecutionTarget>> {
    // Phase 2 (2026-05-09-slow-prep): the installed-provider snapshot and
    // selection config are pre-built on the shared `CompositionPrepContext`
    // so this function no longer rediscovers the source repo root, reloads
    // the claudine config, or re-runs host detection.
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
    let inputs = TargetInputs {
        hints,
        snapshot: &ctx.installed_snapshot,
        explicit_provider,
        favorite: selection_config.and_then(|c| c.favorite),
        cli_model,
        catalog: &catalog,
        source_path,
        is_tty: std::io::stderr().is_terminal(),
        mode: ModelResolveMode::live(silent),
        environment: ctx.invocation.document_environment(),
    };
    let decision = decide_target(&inputs, &mut SelectionTrace::default());
    resolve_decided(decision, &inputs).map(Some)
}

/// What a direct or inline composition's target resolver reads.
pub(crate) struct TargetInputs<'a> {
    pub(crate) hints: &'a EffectiveSelectionHints,
    pub(crate) snapshot: &'a claudine::composition::InstalledProviderSnapshot,
    pub(crate) explicit_provider: Option<Provider>,
    /// The configured `preferred_agent`, which a picker defaults to.
    pub(crate) favorite: Option<Provider>,
    pub(crate) cli_model: Option<&'a str>,
    pub(crate) catalog: &'a ModelCatalogService,
    pub(crate) source_path: &'a std::path::Path,
    /// Whether stderr is a terminal, which decides whether a run asks.
    pub(crate) is_tty: bool,
    pub(crate) mode: ModelResolveMode,
    pub(crate) environment: &'a DocumentEnvironment,
}

/// What the resolver decides before any picker shows.
#[derive(Debug)]
pub(crate) enum TargetDecision {
    Chosen(ResolvedExecutionTarget),
    /// A run on a terminal shows the picker scoped to this state.
    Picker(AgentResolutionState),
    /// A run without a terminal refuses with this error.
    Refused(CompositionError),
}

/// Decide a document's target: the provider flag, else its `agent` hint as
/// [`classify_agent_resolution`] reads it, recording each source consulted
/// in `trace`. Pure: it shows no picker and refreshes no catalog when
/// `inputs.mode` says not to.
pub(crate) fn decide_target(inputs: &TargetInputs<'_>, trace: &mut SelectionTrace) -> TargetDecision {
    if let Some(provider) = inputs.explicit_provider {
        trace.push("--provider flag", LinkState::Chose(provider.as_slug().to_string()));
        let (model, model_reason) = resolve_document_model(
            inputs.catalog, provider, inputs.hints, inputs.cli_model, inputs.mode, inputs.environment,
        );
        return TargetDecision::Chosen(ResolvedExecutionTarget {
            provider,
            provider_reason: claudine::composition::ProviderResolutionReason::ExplicitFlag,
            model,
            model_reason,
        });
    }
    trace.push("--provider flag", LinkState::NotGiven);
    decide_classified(classify_agent_resolution(inputs.hints, inputs.snapshot), inputs, trace)
}

/// [`decide_target`] for an already classified `agent` hint.
fn decide_classified(state: AgentResolutionState, inputs: &TargetInputs<'_>, trace: &mut SelectionTrace) -> TargetDecision {
    use claudine::composition::ProviderResolutionReason;

    let source = format!(
        "{} agent",
        inputs.source_path.file_name().map_or_else(
            || biscuit_file::to_forward_slash_string(inputs.source_path),
            |name| name.to_string_lossy().into_owned(),
        )
    );
    trace.push(source, match state {
        AgentResolutionState::NoAgent => LinkState::NotSet,
        _ => classified(&hints_agent_value(inputs.hints), &state),
    });
    // The non-TTY gate is the shared authority for both branches: its `Ok` is the
    // auto-selectable state, and its `Err` is exactly what a no-operator session
    // aborts with. Deriving both from one call is what keeps the picker-less
    // rebuild in `target_launch` from drifting away from this route.
    match provider_for_state_non_tty(&state, inputs.snapshot, inputs.source_path) {
        Ok(provider) => {
            let (model, model_reason) = resolve_document_model(
                inputs.catalog, provider, inputs.hints, inputs.cli_model, inputs.mode, inputs.environment,
            );
            let provider_reason = match state {
                AgentResolutionState::Selected { .. } => ProviderResolutionReason::FrontmatterSingle,
                _ => ProviderResolutionReason::FrontmatterList,
            };
            TargetDecision::Chosen(ResolvedExecutionTarget { provider, provider_reason, model, model_reason })
        }
        Err(_) if inputs.is_tty => TargetDecision::Picker(state),
        Err(error) => TargetDecision::Refused(error),
    }
}

/// Act on a decision: a chosen target as is, a refusal as the error, and a
/// picker by asking.
fn resolve_decided(decision: TargetDecision, inputs: &TargetInputs<'_>) -> Result<ResolvedExecutionTarget> {
    match decision {
        TargetDecision::Chosen(target) => Ok(target),
        TargetDecision::Refused(error) => Err(error.into()),
        TargetDecision::Picker(state) => {
            let provider =
                prompt_for_agent_state(&state, inputs.hints, inputs.snapshot, inputs.favorite, inputs.source_path)?;
            let (model, model_reason) = resolve_document_model(
                inputs.catalog, provider, inputs.hints, inputs.cli_model, inputs.mode, inputs.environment,
            );
            Ok(ResolvedExecutionTarget {
                provider,
                provider_reason: claudine::composition::ProviderResolutionReason::InteractivePicker,
                model,
                model_reason,
            })
        }
    }
}

/// [`decide_target`] and its resolution for an already-classified state, so
/// unit tests drive each state without building hints that classify to it.
#[cfg(test)]
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
    mode: ModelResolveMode,
    environment: &DocumentEnvironment,
) -> Result<ResolvedExecutionTarget> {
    let inputs = TargetInputs {
        hints, snapshot, explicit_provider: None, favorite, cli_model, catalog, source_path, is_tty, mode, environment,
    };
    let decision = decide_classified(state, &inputs, &mut SelectionTrace::default());
    resolve_decided(decision, &inputs)
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
/// The text is the single source of truth shared with the no-TTY abort body (via [`agent_state_breakdown`] /
/// [`invalid_agent_message`]) so the three surfaces cannot drift. Returns
/// Prose **markup**; callers render it with their own terminal.
///
/// The `sequence` orchestrator reuses this before its review screen so an
/// invalid-scalar or zero-installed-list sequence shows the same pre-prompt
/// message direct compose shows.
pub(crate) fn agent_prompt_message(
    state: &AgentResolutionState,
    source_path: &std::path::Path,
) -> Option<String> {
    match state {
        AgentResolutionState::SingleInvalid { hint } => {
            let file_label = biscuit_file::to_forward_slash_string(source_path);
            let file_link = crate::cli_utils::file_url(source_path).map_or_else(
                || file_label.clone(),
                |file_href| format!("<a href=\"{file_href}\">{file_label}</a>"),
            );
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
        log::message(&claudine::composition::agent_message_prose(markup).render(&term));
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
    let mut plan = claudine::composition::build_picker_plan_with_hints(hints, snapshot, favorite)?;

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

/// How a route wants a document's model resolved by
/// [`resolve_document_model`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ModelResolveMode {
    /// Warm the provider's listing cache as drift-channel input. Off for a
    /// dry run, which must start no subprocess.
    pub(crate) refresh: bool,
    /// Emit the `[model]` warning for a frontmatter model the catalog does
    /// not recognize. Off under `--silent`, and off for the per-attempt
    /// launch rebuild, which re-reads a document that already warned.
    pub(crate) warn: bool,
}

impl ModelResolveMode {
    pub(crate) fn live(silent: bool) -> Self {
        Self {
            refresh: true,
            warn: !silent,
        }
    }

    /// A dry run's resolution: no listing refresh, no warning (the report
    /// carries what it found).
    pub(crate) const PASSIVE: Self = Self {
        refresh: false,
        warn: false,
    };

    /// Retry/resume re-entry: the invocation already refreshed and warned.
    pub(crate) const REBUILD: Self = Self {
        refresh: false,
        warn: false,
    };
}

/// The one place the CLI turns (provider, frontmatter hints, `--model`,
/// catalog) into the model a document launches with.
///
/// Every route — compose, inline-compose, sequence review, the dry-run
/// check, and the retry/resume rebuild — resolves here, so the library
/// chain in `composition::select` is the only model precedence rule and the
/// catalog's role (order list hints, warn on a miss, never demote) cannot
/// drift between routes.
pub(crate) fn resolve_document_model(
    catalog: &ModelCatalogService,
    provider: Provider,
    hints: &EffectiveSelectionHints,
    cli_model: Option<&str>,
    mode: ModelResolveMode,
    // The document's captured environment: model precedence steps 2 and 3
    // (provider-specific variables, then `MODEL`) read it, never the process.
    environment: &DocumentEnvironment,
) -> (Option<String>, ModelResolutionReason) {
    let env_lookup = |var: &str| environment.lookup(var);
    if mode.refresh {
        // Probe without the catalog so the refresh gate can see whether a
        // CLI/env source already decided the model.
        let (_, probe_reason) =
            resolve_model_with_hints_from(provider, hints, cli_model, None, env_lookup);
        refresh_for_model_validation(catalog, provider, hints, Some(&probe_reason));
    }
    let (model, reason) =
        resolve_model_with_hints_from(provider, hints, cli_model, Some(catalog), env_lookup);
    if mode.warn
        && let Some(notice) = model_catalog_notice(catalog, provider, model.as_deref(), &reason)
    {
        log::warn(&notice);
    }
    (model, reason)
}

/// The `[model]` prepare-time warning for a frontmatter model outside the
/// provider's expected offerings, or `None` when the model came from a
/// CLI/env source or the catalog recognizes it.
pub(crate) fn model_catalog_notice(
    catalog: &ModelCatalogService,
    provider: Provider,
    model: Option<&str>,
    reason: &ModelResolutionReason,
) -> Option<String> {
    let model = model?;
    if !matches!(
        reason,
        ModelResolutionReason::FrontmatterSingle | ModelResolutionReason::FrontmatterList
    ) || catalog.is_valid(provider, model)
    {
        return None;
    }
    Some(format!(
        "[model] frontmatter model '{model}' is outside Claudine's expected offerings for {}; \
         forwarding it unchanged",
        crate::output::capitalize_provider(provider)
    ))
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
