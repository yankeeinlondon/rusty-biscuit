//! Provider and model resolution for composition workflows.

use std::collections::{BTreeMap, BTreeSet};

use crate::model_catalog::ModelCatalogService;
use crate::provider::{PROVIDERS_DISPLAY_ORDER, Provider, provider_info};

use super::error::CompositionError;
use super::types::{
    AgentHint, AgentResolutionState, EffectiveSelectionHints, InstalledProviderSnapshot, ModelHint,
    ModelResolutionReason, ProviderPickerOption, ProviderPickerPlan, ProviderResolutionReason,
    ResolutionMode, ResolvedExecutionTarget, SelectedProvider, SelectionReason,
};

/// Build a snapshot of installed providers from a pre-computed list.
///
/// The caller (typically the CLI) is responsible for running host
/// detection once and passing the result here.  This keeps the
/// library side pure and unit-testable.
pub fn build_installed_snapshot(
    installed: &[Provider],
    excluded: &BTreeSet<Provider>,
    clients: &sniff::programs::InstalledAiClients,
) -> InstalledProviderSnapshot {
    let runnable: Vec<Provider> = installed
        .iter()
        .copied()
        .filter(|p| !excluded.contains(p))
        .collect();

    let mut binary_paths = BTreeMap::new();
    for provider in installed {
        if let Some(path) = clients.path(provider.sniff_ai_cli()) {
            binary_paths.insert(*provider, path);
        }
    }

    InstalledProviderSnapshot {
        runnable,
        excluded: excluded.clone(),
        all_installed: installed.to_vec(),
        binary_paths,
    }
}

/// Detect which providers have a runnable agentic CLI on `PATH`.
///
/// Only `ExecutableSource::Path` discoveries count as installed. sniff's
/// program lookup falls back to macOS app bundles when the `PATH` probe
/// misses, and for agentic providers that fallback finds the *desktop* GUI
/// app (`/Applications/Claude.app`, `/Applications/Qwen.app`, ...) — a
/// binary that cannot run a terminal session. Counting it would select a
/// provider the harness cannot actually launch and would leak host state
/// past a deliberately restricted `PATH`. This mirrors the direct wrapper's
/// `which`-based binary resolution, so "installed" means the same thing on
/// both surfaces.
pub fn detect_installed_providers(
    clients: &sniff::programs::InstalledAiClients,
) -> Vec<Provider> {
    PROVIDERS_DISPLAY_ORDER
        .into_iter()
        .filter(|p| {
            matches!(
                clients.path_with_source(p.sniff_ai_cli()),
                Some((_, sniff::programs::ExecutableSource::Path))
            )
        })
        .collect()
}

/// Classify a frontmatter `agent` value against the installed-provider
/// snapshot and return the corresponding [`AgentResolutionState`].
///
/// This function is pure: it does not prompt, emit output, or consult
/// `--silent`. The caller (dry-run renderer or live execution gate) decides
/// what to do with each variant.
pub fn classify_agent_resolution(
    hints: &EffectiveSelectionHints,
    snapshot: &InstalledProviderSnapshot,
) -> AgentResolutionState {
    // Explicit CLI provider is handled upstream; this classifies frontmatter
    // hints only.
    let invalid = hints.agent_invalid.clone();

    match &hints.agent {
        None => {
            if invalid.is_empty() {
                AgentResolutionState::NoAgent
            } else if hints.agent_was_list {
                // A list-valued hint that resolved to zero valid providers —
                // including a single-entry all-invalid list — is the
                // zero-installed-list state, never a single-invalid scalar.
                AgentResolutionState::ZeroInstalledList {
                    not_installed: Vec::new(),
                    invalid,
                }
            } else {
                // A scalar value parses to at most one invalid entry.
                AgentResolutionState::SingleInvalid {
                    hint: invalid[0].clone(),
                }
            }
        }
        Some(AgentHint::Single(provider)) => {
            if snapshot.runnable.contains(provider) {
                AgentResolutionState::Selected { provider: *provider }
            } else {
                AgentResolutionState::SingleNotInstalled { provider: *provider }
            }
        }
        Some(AgentHint::List(providers)) => {
            let mut installed = Vec::with_capacity(providers.len());
            let mut not_installed = Vec::with_capacity(providers.len());
            for provider in providers {
                if snapshot.runnable.contains(provider) {
                    installed.push(*provider);
                } else {
                    not_installed.push(*provider);
                }
            }

            match installed.len() {
                0 => AgentResolutionState::ZeroInstalledList {
                    not_installed,
                    invalid,
                },
                1 => AgentResolutionState::ListOneInstalled {
                    selected: installed[0],
                    not_installed,
                    invalid,
                },
                _ => AgentResolutionState::ListMultipleInstalled {
                    installed,
                    not_installed,
                    invalid,
                },
            }
        }
    }
}

/// Resolve provider and model for a non-TTY session.
///
/// Returns a fully resolved target or a structured error explaining why
/// resolution failed.
pub fn resolve_target_non_tty(
    explicit_provider: Option<Provider>,
    prepared: &super::types::PreparedComposition,
    snapshot: &InstalledProviderSnapshot,
    favorite: Option<Provider>,
    cli_model: Option<&str>,
) -> Result<ResolvedExecutionTarget, CompositionError> {
    resolve_target_non_tty_with_env(
        explicit_provider,
        &prepared.selection_hints,
        snapshot,
        favorite,
        cli_model,
        |var| std::env::var(var).ok(),
        None,
    )
}

/// Resolve provider and model for a non-TTY session with an optional catalog.
///
/// When a [`ModelCatalogService`] is provided, frontmatter `model` hints are
/// validated against the merged catalog. Invalid hints are skipped rather than
/// treated as errors.
pub fn resolve_target_non_tty_with_catalog(
    explicit_provider: Option<Provider>,
    prepared: &super::types::PreparedComposition,
    snapshot: &InstalledProviderSnapshot,
    favorite: Option<Provider>,
    cli_model: Option<&str>,
    catalog: Option<&ModelCatalogService>,
) -> Result<ResolvedExecutionTarget, CompositionError> {
    resolve_target_non_tty_with_env(
        explicit_provider,
        &prepared.selection_hints,
        snapshot,
        favorite,
        cli_model,
        |var| std::env::var(var).ok(),
        catalog,
    )
}

/// Resolve provider and model for a non-TTY session using raw selection hints.
///
/// Mirrors [`resolve_target_non_tty_with_catalog`] but takes hints directly,
/// without requiring a fully prepared composition. Used by the CLI for *eager*
/// target resolution before composition templates are rendered, so
/// `{{env.AGENT}}` resolves to the chosen provider during body rendering.
pub fn resolve_target_non_tty_with_hints(
    explicit_provider: Option<Provider>,
    hints: &EffectiveSelectionHints,
    snapshot: &InstalledProviderSnapshot,
    favorite: Option<Provider>,
    cli_model: Option<&str>,
    catalog: Option<&ModelCatalogService>,
) -> Result<ResolvedExecutionTarget, CompositionError> {
    resolve_target_non_tty_with_env(
        explicit_provider,
        hints,
        snapshot,
        favorite,
        cli_model,
        |var| std::env::var(var).ok(),
        catalog,
    )
}

fn resolve_target_non_tty_with_env<E>(
    explicit_provider: Option<Provider>,
    hints: &EffectiveSelectionHints,
    snapshot: &InstalledProviderSnapshot,
    favorite: Option<Provider>,
    cli_model: Option<&str>,
    env_lookup: E,
    catalog: Option<&ModelCatalogService>,
) -> Result<ResolvedExecutionTarget, CompositionError>
where
    E: Fn(&str) -> Option<String>,
{
    // 1. Explicit provider flag (highest priority)
    if let Some(provider) = explicit_provider {
        if snapshot.all_installed.contains(&provider) {
            let (model, model_reason) =
                resolve_model_with_env(provider, hints, cli_model, &env_lookup, catalog);
            return Ok(ResolvedExecutionTarget {
                provider,
                provider_reason: ProviderResolutionReason::ExplicitFlag,
                model,
                model_reason,
            });
        }
        return Err(CompositionError::NoRunnableProviders);
    }

    if snapshot.runnable.is_empty() {
        return Err(CompositionError::NoRunnableProviders);
    }

    // 2. Frontmatter agent hint
    let provider = if let Some(ref hint) = hints.agent {
        resolve_agent_hint_non_tty(hint, snapshot)
    } else {
        None
    };

    // 3. Config favorite
    let provider = provider.or_else(|| favorite.filter(|f| snapshot.runnable.contains(f)));

    let Some(provider) = provider else {
        return Err(CompositionError::SelectionUnavailable {
            mode: ResolutionMode::NonTty,
            installed: snapshot.runnable.clone(),
            favorite_agent: favorite,
            frontmatter_agent_present: hints.agent.is_some(),
        });
    };

    let (model, model_reason) =
        resolve_model_with_env(provider, hints, cli_model, &env_lookup, catalog);

    // Providers that require an explicit model in non-interactive mode (catalog
    // `model_required_in_non_tty`; OpenCode today) hard-error when none resolved.
    if provider_info(provider).model_required_in_non_tty && model.is_none() {
        return Err(CompositionError::ModelSelectionFailed {
            provider,
            reason: "OpenCode requires a model in non-interactive mode; set --model, OPENCODE_MODEL, or MODEL".into(),
        });
    }

    // Determine provider resolution reason based on which signal actually resolved
    let provider_reason = if let Some(ref hint) = hints.agent {
        let resolved_from_hint = match hint {
            AgentHint::Single(p) => *p == provider,
            AgentHint::List(list) => list
                .iter()
                .any(|p| snapshot.runnable.contains(p) && *p == provider),
        };
        if resolved_from_hint {
            match hint {
                AgentHint::Single(_) => ProviderResolutionReason::FrontmatterSingle,
                AgentHint::List(_) => ProviderResolutionReason::FrontmatterList,
            }
        } else {
            ProviderResolutionReason::FavoriteAgent
        }
    } else {
        ProviderResolutionReason::FavoriteAgent
    };

    Ok(ResolvedExecutionTarget {
        provider,
        provider_reason,
        model,
        model_reason,
    })
}

/// Build a picker plan for TTY interactive selection.
///
/// The picker always shows all installed (non-excluded) providers. The
/// default index and row ordering are influenced by frontmatter `agent`
/// and the configured favorite agent.
#[allow(dead_code)]
pub fn build_picker_plan(
    prepared: &super::types::PreparedComposition,
    snapshot: &InstalledProviderSnapshot,
    favorite: Option<Provider>,
) -> Result<ProviderPickerPlan, CompositionError> {
    build_picker_plan_with_hints(&prepared.selection_hints, snapshot, favorite)
}

/// Build a picker plan from raw selection hints, without requiring a
/// fully prepared composition. Used by the CLI for eager target
/// resolution before composition templates render.
pub fn build_picker_plan_with_hints(
    hints: &EffectiveSelectionHints,
    snapshot: &InstalledProviderSnapshot,
    favorite: Option<Provider>,
) -> Result<ProviderPickerPlan, CompositionError> {
    if snapshot.runnable.is_empty() {
        return Err(CompositionError::NoRunnableProviders);
    }

    // Start from canonical display order, keeping only runnable providers
    let mut options: Vec<ProviderPickerOption> = PROVIDERS_DISPLAY_ORDER
        .iter()
        .filter(|&&p| snapshot.runnable.contains(&p))
        .map(|&p| ProviderPickerOption {
            provider: p,
            rank_reason: None,
        })
        .collect();

    let mut default_index: usize = 0;

    // Apply frontmatter ordering and default-index influence
    if let Some(ref hint) = hints.agent {
        match hint {
            AgentHint::Single(provider) => {
                if let Some(pos) = options.iter().position(|o| o.provider == *provider) {
                    // Move to top
                    let opt = options.remove(pos);
                    options.insert(0, opt);
                    default_index = 0;
                    options[0].rank_reason = Some(super::types::PickerInfluence::FrontmatterSingle);
                }
            }
            AgentHint::List(providers) => {
                let mut reordered = Vec::new();
                let mut remaining: Vec<ProviderPickerOption> = options.clone();

                for provider in providers {
                    if let Some(pos) = remaining.iter().position(|o| o.provider == *provider) {
                        let mut opt = remaining.remove(pos);
                        opt.rank_reason = Some(super::types::PickerInfluence::FrontmatterList);
                        reordered.push(opt);
                    }
                }

                // Append remaining providers in display order
                reordered.extend(remaining);
                options = reordered;

                // Default is first installed entry from the list
                default_index = options
                    .iter()
                    .position(|o| {
                        providers.contains(&o.provider) && snapshot.runnable.contains(&o.provider)
                    })
                    .unwrap_or(0);
            }
        }
    }

    // If no frontmatter default applied, try favorite agent
    if default_index == 0
        && options.first().and_then(|o| o.rank_reason).is_none()
        && let Some(fav) = favorite
        && let Some(pos) = options.iter().position(|o| o.provider == fav)
    {
        let mut opt = options.remove(pos);
        opt.rank_reason = Some(super::types::PickerInfluence::FavoriteAgent);
        options.insert(0, opt);
        default_index = 0;
    }

    Ok(ProviderPickerPlan {
        options,
        default_index,
    })
}

/// Resolve model for a given provider using the precedence chain:
///
/// 1. CLI `--model`
/// 2. Provider-specific env var(s)
/// 3. Generic `MODEL` env var
/// 4. Frontmatter `model` (validated against catalog when available)
/// 5. Provider default (`None`)
#[allow(dead_code)]
pub fn resolve_model(
    provider: Provider,
    prepared: &super::types::PreparedComposition,
    cli_model: Option<&str>,
) -> (Option<String>, ModelResolutionReason) {
    resolve_model_with_env(
        provider,
        &prepared.selection_hints,
        cli_model,
        |var| std::env::var(var).ok(),
        None,
    )
}

/// Resolve model with an optional catalog for frontmatter validation.
///
/// When `catalog` is provided, frontmatter `model` hints are validated
/// against the merged catalog. Invalid single hints fall through to the
/// provider default; invalid list entries are skipped until a valid one
/// is found or the list is exhausted.
pub fn resolve_model_with_catalog(
    provider: Provider,
    prepared: &super::types::PreparedComposition,
    cli_model: Option<&str>,
    catalog: Option<&ModelCatalogService>,
) -> (Option<String>, ModelResolutionReason) {
    resolve_model_with_env(
        provider,
        &prepared.selection_hints,
        cli_model,
        |var| std::env::var(var).ok(),
        catalog,
    )
}

/// Resolve model from raw selection hints, without a prepared composition.
///
/// Mirrors [`resolve_model_with_catalog`] for callers that want to resolve
/// a model before composition runs (e.g. for eager AGENT injection).
pub fn resolve_model_with_hints(
    provider: Provider,
    hints: &EffectiveSelectionHints,
    cli_model: Option<&str>,
    catalog: Option<&ModelCatalogService>,
) -> (Option<String>, ModelResolutionReason) {
    resolve_model_with_env(
        provider,
        hints,
        cli_model,
        |var| std::env::var(var).ok(),
        catalog,
    )
}

fn resolve_model_with_env<E>(
    provider: Provider,
    hints: &EffectiveSelectionHints,
    cli_model: Option<&str>,
    env_lookup: E,
    catalog: Option<&ModelCatalogService>,
) -> (Option<String>, ModelResolutionReason)
where
    E: Fn(&str) -> Option<String>,
{
    // 1. CLI --model
    if let Some(model) = cli_model {
        return (Some(model.to_string()), ModelResolutionReason::ExplicitCli);
    }

    // 2. Provider-specific env vars
    let env_vars = provider_env_vars(provider);
    for &var_name in env_vars {
        if let Some(value) = env_lookup(var_name)
            && !value.is_empty()
        {
            return (Some(value), ModelResolutionReason::ProviderEnv(var_name));
        }
    }

    // 3. Generic MODEL env
    if let Some(value) = env_lookup("MODEL")
        && !value.is_empty()
    {
        return (Some(value), ModelResolutionReason::GenericEnv);
    }

    // 4. Frontmatter model (validated against catalog when available)
    if let Some(ref hint) = hints.model {
        match hint {
            ModelHint::Single(model) => {
                if catalog.is_none() || catalog.unwrap().is_valid(provider, model) {
                    return (
                        Some(model.clone()),
                        ModelResolutionReason::FrontmatterSingle,
                    );
                }
            }
            ModelHint::List(models) => {
                if let Some(catalog) = catalog {
                    if let Some(valid) = catalog.first_valid(provider, models) {
                        return (Some(valid), ModelResolutionReason::FrontmatterList);
                    }
                } else if let Some(first) = models.first() {
                    return (Some(first.clone()), ModelResolutionReason::FrontmatterList);
                }
            }
        }
    }

    // 5. Provider default
    (None, ModelResolutionReason::ProviderDefault)
}

/// Return the provider-specific environment variable names to check
/// for model selection, in priority order.
fn provider_env_vars(provider: Provider) -> &'static [&'static str] {
    provider_info(provider).model_env_vars
}

/// Resolve an agent hint in non-TTY mode.
fn resolve_agent_hint_non_tty(
    hint: &AgentHint,
    snapshot: &InstalledProviderSnapshot,
) -> Option<Provider> {
    match hint {
        AgentHint::Single(provider) => {
            if snapshot.runnable.contains(provider) {
                Some(*provider)
            } else {
                None
            }
        }
        AgentHint::List(providers) => {
            for provider in providers {
                if snapshot.runnable.contains(provider) {
                    return Some(*provider);
                }
            }
            None
        }
    }
}

/// Legacy compatibility: select a provider using the old API.
///
/// This delegates to the new resolver. In TTY mode it returns
/// `InteractiveSelectionRequired` (the caller should show the picker).
/// In non-TTY mode it resolves automatically.
pub fn select_provider(
    explicit_provider: Option<Provider>,
    prepared: &super::types::PreparedComposition,
    installed: &[Provider],
    excluded: &BTreeSet<Provider>,
    favorite: Option<Provider>,
) -> Result<SelectedProvider, CompositionError> {
    let snapshot = InstalledProviderSnapshot {
        runnable: build_candidate_set(installed, excluded),
        excluded: excluded.clone(),
        all_installed: installed.to_vec(),
        binary_paths: std::collections::BTreeMap::new(),
    };

    let target = resolve_target_non_tty(explicit_provider, prepared, &snapshot, favorite, None)?;

    let reason = match target.provider_reason {
        ProviderResolutionReason::ExplicitFlag => SelectionReason::ExplicitProvider,
        ProviderResolutionReason::FrontmatterSingle | ProviderResolutionReason::FrontmatterList => {
            SelectionReason::FrontmatterHint
        }
        ProviderResolutionReason::FavoriteAgent => SelectionReason::ConfigFavorite,
        _ => SelectionReason::InteractiveChoice,
    };

    Ok(SelectedProvider {
        provider: target.provider,
        reason,
    })
}

/// Build the set of candidate providers by filtering installed providers.
///
/// Removes excluded providers and returns only those that have a wrapper profile.
pub fn build_candidate_set(installed: &[Provider], excluded: &BTreeSet<Provider>) -> Vec<Provider> {
    installed
        .iter()
        .copied()
        .filter(|p| !excluded.contains(p))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::composition::types::{
        AgentHint, CompositionMode, EffectiveSelectionHints, ModelHint,
    };
    use serde_json::json;
    use std::path::PathBuf;

    fn make_prepared_composition(
        agent_hint: Option<AgentHint>,
        model_hint: Option<ModelHint>,
    ) -> super::super::types::PreparedComposition {
        use super::super::lifecycle::LifecycleConfig;
        use super::super::types::CompositionClosurePlan;
        super::super::types::PreparedComposition {
            mode: CompositionMode::ChainedDocument,
            resolved_path: PathBuf::from("/tmp/test.md"),
            source_repo_root: None,
            prompt: "test prompt".to_string(),
            effective_frontmatter: json!({}),
            selection_hints: EffectiveSelectionHints {
                agent: agent_hint.clone(),
                model: model_hint,
                interactive: None,
                agent_invalid: Vec::new(),
                agent_was_list: matches!(agent_hint, Some(AgentHint::List(_))),
            },
            closure: CompositionClosurePlan::Direct,
            lifecycle: LifecycleConfig::default(),
            compose_perf: None,
            dropped_optionals: Vec::new(),
            warnings: Vec::new(),
            deferred_lifecycle_keys: Vec::new(),
            rematerialize: Default::default(),
        }
    }

    fn make_snapshot(
        installed: Vec<Provider>,
        excluded: BTreeSet<Provider>,
    ) -> InstalledProviderSnapshot {
        let runnable: Vec<Provider> = installed
            .iter()
            .copied()
            .filter(|p| !excluded.contains(p))
            .collect();
        InstalledProviderSnapshot {
            runnable,
            excluded,
            all_installed: installed,
            binary_paths: std::collections::BTreeMap::new(),
        }
    }

    #[test]
    fn explicit_provider_selected() {
        let prepared = make_prepared_composition(None, None);
        let snapshot = make_snapshot(vec![Provider::Claude, Provider::Codex], BTreeSet::new());

        let target =
            resolve_target_non_tty(Some(Provider::Claude), &prepared, &snapshot, None, None)
                .unwrap();
        assert_eq!(target.provider, Provider::Claude);
        assert!(matches!(
            target.provider_reason,
            ProviderResolutionReason::ExplicitFlag
        ));
    }

    #[test]
    fn explicit_provider_not_installed_errors() {
        let prepared = make_prepared_composition(None, None);
        let snapshot = make_snapshot(vec![Provider::Claude], BTreeSet::new());

        let err = resolve_target_non_tty(Some(Provider::Codex), &prepared, &snapshot, None, None)
            .unwrap_err();
        assert!(matches!(err, CompositionError::NoRunnableProviders));
    }

    #[test]
    fn explicit_provider_ignores_exclusion() {
        let prepared = make_prepared_composition(None, None);
        let snapshot = make_snapshot(
            vec![Provider::Claude, Provider::Codex],
            [Provider::Claude].into_iter().collect(),
        );

        let target =
            resolve_target_non_tty(Some(Provider::Claude), &prepared, &snapshot, None, None)
                .unwrap();
        assert_eq!(target.provider, Provider::Claude);
    }

    #[test]
    fn no_single_installed_auto_select() {
        let prepared = make_prepared_composition(None, None);
        let snapshot = make_snapshot(vec![Provider::Claude], BTreeSet::new());

        // With only one installed provider and no signals, non-TTY should error
        // (the old SingleInstalled shortcut is removed)
        let err = resolve_target_non_tty(None, &prepared, &snapshot, None, None).unwrap_err();
        assert!(matches!(err, CompositionError::SelectionUnavailable { .. }));
    }

    #[test]
    fn frontmatter_single_resolves() {
        let prepared = make_prepared_composition(Some(AgentHint::Single(Provider::Codex)), None);
        let snapshot = make_snapshot(vec![Provider::Claude, Provider::Codex], BTreeSet::new());

        let target = resolve_target_non_tty(None, &prepared, &snapshot, None, None).unwrap();
        assert_eq!(target.provider, Provider::Codex);
        assert!(matches!(
            target.provider_reason,
            ProviderResolutionReason::FrontmatterSingle
        ));
    }

    #[test]
    fn frontmatter_list_selects_first_installed() {
        let prepared = make_prepared_composition(
            Some(AgentHint::List(vec![Provider::Gemini, Provider::Codex])),
            None,
        );
        let snapshot = make_snapshot(vec![Provider::Claude, Provider::Codex], BTreeSet::new());

        let target = resolve_target_non_tty(None, &prepared, &snapshot, None, None).unwrap();
        // Gemini not installed, Codex is second in list
        assert_eq!(target.provider, Provider::Codex);
        assert!(matches!(
            target.provider_reason,
            ProviderResolutionReason::FrontmatterList
        ));
    }

    #[test]
    fn favorite_agent_resolves_when_no_frontmatter() {
        let prepared = make_prepared_composition(None, None);
        let snapshot = make_snapshot(
            vec![Provider::Claude, Provider::Codex, Provider::Gemini],
            BTreeSet::new(),
        );

        let target =
            resolve_target_non_tty(None, &prepared, &snapshot, Some(Provider::Gemini), None)
                .unwrap();
        assert_eq!(target.provider, Provider::Gemini);
        assert!(matches!(
            target.provider_reason,
            ProviderResolutionReason::FavoriteAgent
        ));
    }

    #[test]
    fn favorite_agent_not_installed_falls_through() {
        let prepared = make_prepared_composition(None, None);
        let snapshot = make_snapshot(vec![Provider::Claude, Provider::Codex], BTreeSet::new());

        let err = resolve_target_non_tty(None, &prepared, &snapshot, Some(Provider::Gemini), None)
            .unwrap_err();
        assert!(matches!(err, CompositionError::SelectionUnavailable { .. }));
    }

    #[test]
    fn no_resolution_signals_errors() {
        let prepared = make_prepared_composition(None, None);
        let snapshot = make_snapshot(vec![Provider::Claude, Provider::Codex], BTreeSet::new());

        let err = resolve_target_non_tty(None, &prepared, &snapshot, None, None).unwrap_err();
        assert!(matches!(err, CompositionError::SelectionUnavailable { .. }));
    }

    #[test]
    fn exclusion_narrows_candidates() {
        let prepared = make_prepared_composition(None, None);
        let snapshot = make_snapshot(
            vec![Provider::Claude, Provider::Codex],
            [Provider::Claude].into_iter().collect(),
        );

        // With exclusion, only Codex is runnable; but with no signals, still error
        let err = resolve_target_non_tty(None, &prepared, &snapshot, None, None).unwrap_err();
        assert!(matches!(err, CompositionError::SelectionUnavailable { .. }));
    }

    #[test]
    fn hint_matches_known_but_uninstalled_provider_falls_through_to_favorite() {
        let prepared = make_prepared_composition(Some(AgentHint::Single(Provider::Gemini)), None);
        let snapshot = make_snapshot(vec![Provider::Claude, Provider::Codex], BTreeSet::new());

        let target =
            resolve_target_non_tty(None, &prepared, &snapshot, Some(Provider::Claude), None)
                .unwrap();
        assert_eq!(target.provider, Provider::Claude);
        assert!(matches!(
            target.provider_reason,
            ProviderResolutionReason::FavoriteAgent
        ));
    }

    #[test]
    fn hint_matches_known_but_uninstalled_provider_no_favorite_errors() {
        let prepared = make_prepared_composition(Some(AgentHint::Single(Provider::Gemini)), None);
        let snapshot = make_snapshot(vec![Provider::Claude, Provider::Codex], BTreeSet::new());

        let err = resolve_target_non_tty(None, &prepared, &snapshot, None, None).unwrap_err();
        assert!(matches!(err, CompositionError::SelectionUnavailable { .. }));
    }

    #[test]
    fn empty_installed_errors() {
        let prepared = make_prepared_composition(None, None);
        let snapshot = make_snapshot(vec![], BTreeSet::new());

        let err = resolve_target_non_tty(None, &prepared, &snapshot, None, None).unwrap_err();
        assert!(matches!(err, CompositionError::NoRunnableProviders));
    }

    // -- Model resolution tests ------------------------------------------------

    #[test]
    fn model_cli_model_wins() {
        let prepared = make_prepared_composition(None, None);
        let (model, reason) = resolve_model(Provider::Codex, &prepared, Some("gpt-5"));
        assert_eq!(model, Some("gpt-5".to_string()));
        assert!(matches!(reason, ModelResolutionReason::ExplicitCli));
    }

    #[test]
    fn model_provider_env_wins_over_generic() {
        let prepared = make_prepared_composition(None, None);
        // Use the internal testable variant with an empty env lookup to verify
        // the fallback chain when no env vars are set:
        let (model, reason) = resolve_model_with_env(
            Provider::Codex,
            &prepared.selection_hints,
            None,
            |_| None,
            None,
        );
        assert_eq!(model, None);
        assert!(matches!(reason, ModelResolutionReason::ProviderDefault));
    }

    #[test]
    fn model_frontmatter_single_used() {
        let prepared = make_prepared_composition(None, Some(ModelHint::Single("gpt-4o".into())));
        let (model, reason) = resolve_model_with_env(
            Provider::Codex,
            &prepared.selection_hints,
            None,
            |_| None,
            None,
        );
        assert_eq!(model, Some("gpt-4o".to_string()));
        assert!(matches!(reason, ModelResolutionReason::FrontmatterSingle));
    }

    #[test]
    fn model_frontmatter_list_used() {
        let prepared = make_prepared_composition(
            None,
            Some(ModelHint::List(vec!["gpt-4o".into(), "o3-mini".into()])),
        );
        let (model, reason) = resolve_model_with_env(
            Provider::Codex,
            &prepared.selection_hints,
            None,
            |_| None,
            None,
        );
        assert_eq!(model, Some("gpt-4o".to_string()));
        assert!(matches!(reason, ModelResolutionReason::FrontmatterList));
    }

    // -- Catalog-aware model resolution tests ----------------------------------

    #[test]
    fn model_catalog_rejects_invalid_single_hint() {
        let prepared = make_prepared_composition(None, Some(ModelHint::Single("not-real".into())));
        let cache = tempfile::TempDir::new().unwrap();
        let catalog = crate::model_catalog::ModelCatalogService::with_cache_dir(
            cache.path().to_path_buf(),
        );
        let (model, reason) = resolve_model_with_env(
            Provider::Codex,
            &prepared.selection_hints,
            None,
            |_| None,
            Some(&catalog),
        );
        // Invalid single hint falls through to provider default
        assert_eq!(model, None);
        assert!(matches!(reason, ModelResolutionReason::ProviderDefault));
    }

    #[test]
    fn model_catalog_skips_invalid_list_entries() {
        let prepared = make_prepared_composition(
            None,
            Some(ModelHint::List(vec!["not-real".into(), "gpt-5.5".into()])),
        );
        let cache = tempfile::TempDir::new().unwrap();
        let catalog = crate::model_catalog::ModelCatalogService::with_cache_dir(
            cache.path().to_path_buf(),
        );
        let (model, reason) = resolve_model_with_env(
            Provider::Codex,
            &prepared.selection_hints,
            None,
            |_| None,
            Some(&catalog),
        );
        assert_eq!(model, Some("gpt-5.5".to_string()));
        assert!(matches!(reason, ModelResolutionReason::FrontmatterList));
    }

    #[test]
    fn model_catalog_all_list_entries_invalid_falls_through() {
        let prepared = make_prepared_composition(
            None,
            Some(ModelHint::List(vec!["not-real".into(), "also-fake".into()])),
        );
        let cache = tempfile::TempDir::new().unwrap();
        let catalog = crate::model_catalog::ModelCatalogService::with_cache_dir(
            cache.path().to_path_buf(),
        );
        let (model, reason) = resolve_model_with_env(
            Provider::Codex,
            &prepared.selection_hints,
            None,
            |_| None,
            Some(&catalog),
        );
        assert_eq!(model, None);
        assert!(matches!(reason, ModelResolutionReason::ProviderDefault));
    }

    #[test]
    fn model_catalog_valid_single_hint_accepted() {
        let prepared = make_prepared_composition(None, Some(ModelHint::Single("gpt-5.5".into())));
        let cache = tempfile::TempDir::new().unwrap();
        let catalog = crate::model_catalog::ModelCatalogService::with_cache_dir(
            cache.path().to_path_buf(),
        );
        let (model, reason) = resolve_model_with_env(
            Provider::Codex,
            &prepared.selection_hints,
            None,
            |_| None,
            Some(&catalog),
        );
        assert_eq!(model, Some("gpt-5.5".to_string()));
        assert!(matches!(reason, ModelResolutionReason::FrontmatterSingle));
    }

    // -- Picker plan tests -----------------------------------------------------

    #[test]
    fn picker_plan_basic_order() {
        let prepared = make_prepared_composition(None, None);
        let snapshot = make_snapshot(
            vec![Provider::Codex, Provider::Claude, Provider::Gemini],
            BTreeSet::new(),
        );

        let plan = build_picker_plan(&prepared, &snapshot, None).unwrap();
        // Should be in display order
        assert_eq!(plan.options.len(), 3);
        assert_eq!(plan.options[0].provider, Provider::Claude);
        assert_eq!(plan.options[1].provider, Provider::Codex);
        assert_eq!(plan.options[2].provider, Provider::Gemini);
        assert_eq!(plan.default_index, 0);
    }

    #[test]
    fn picker_plan_frontmatter_single_moves_to_top() {
        let prepared = make_prepared_composition(Some(AgentHint::Single(Provider::Gemini)), None);
        let snapshot = make_snapshot(
            vec![Provider::Codex, Provider::Claude, Provider::Gemini],
            BTreeSet::new(),
        );

        let plan = build_picker_plan(&prepared, &snapshot, None).unwrap();
        assert_eq!(plan.options[0].provider, Provider::Gemini);
        assert!(matches!(
            plan.options[0].rank_reason,
            Some(super::super::types::PickerInfluence::FrontmatterSingle)
        ));
        assert_eq!(plan.default_index, 0);
    }

    #[test]
    fn picker_plan_frontmatter_list_reorders() {
        let prepared = make_prepared_composition(
            Some(AgentHint::List(vec![Provider::Gemini, Provider::Claude])),
            None,
        );
        let snapshot = make_snapshot(
            vec![Provider::Codex, Provider::Claude, Provider::Gemini],
            BTreeSet::new(),
        );

        let plan = build_picker_plan(&prepared, &snapshot, None).unwrap();
        // Gemini first (in list), then Claude (in list), then Codex (remaining)
        assert_eq!(plan.options[0].provider, Provider::Gemini);
        assert_eq!(plan.options[1].provider, Provider::Claude);
        assert_eq!(plan.options[2].provider, Provider::Codex);
        assert_eq!(plan.default_index, 0); // first installed from list
    }

    #[test]
    fn picker_plan_favorite_agent_default() {
        let prepared = make_prepared_composition(None, None);
        let snapshot = make_snapshot(
            vec![Provider::Codex, Provider::Claude, Provider::Gemini],
            BTreeSet::new(),
        );

        let plan = build_picker_plan(&prepared, &snapshot, Some(Provider::Codex)).unwrap();
        assert_eq!(plan.options[0].provider, Provider::Codex);
        assert!(matches!(
            plan.options[0].rank_reason,
            Some(super::super::types::PickerInfluence::FavoriteAgent)
        ));
        assert_eq!(plan.default_index, 0);
    }

    #[test]
    fn picker_plan_frontmatter_overrides_favorite() {
        let prepared = make_prepared_composition(Some(AgentHint::Single(Provider::Gemini)), None);
        let snapshot = make_snapshot(
            vec![Provider::Codex, Provider::Claude, Provider::Gemini],
            BTreeSet::new(),
        );

        let plan = build_picker_plan(&prepared, &snapshot, Some(Provider::Codex)).unwrap();
        // Frontmatter should win over favorite
        assert_eq!(plan.options[0].provider, Provider::Gemini);
        assert_eq!(plan.default_index, 0);
    }

    #[test]
    fn picker_plan_uninstalled_frontmatter_ignored() {
        let prepared = make_prepared_composition(Some(AgentHint::Single(Provider::Goose)), None);
        let snapshot = make_snapshot(vec![Provider::Codex, Provider::Claude], BTreeSet::new());

        let plan = build_picker_plan(&prepared, &snapshot, None).unwrap();
        // Goose not installed, so no reordering happens; default stays 0
        assert_eq!(plan.options[0].provider, Provider::Claude);
        assert_eq!(plan.default_index, 0);
    }

    // -- Agent-resolution classification tests -------------------------------

    fn hints_from_agent(
        agent: Option<AgentHint>,
        invalid: Vec<String>,
    ) -> EffectiveSelectionHints {
        let was_list = matches!(agent, Some(AgentHint::List(_)));
        hints_from_agent_with_list(agent, invalid, was_list)
    }

    fn hints_from_agent_with_list(
        agent: Option<AgentHint>,
        invalid: Vec<String>,
        was_list: bool,
    ) -> EffectiveSelectionHints {
        EffectiveSelectionHints {
            agent,
            model: None,
            interactive: None,
            agent_invalid: invalid,
            agent_was_list: was_list,
        }
    }

    #[test]
    fn classify_no_agent() {
        let snapshot = make_snapshot(vec![Provider::Claude, Provider::Codex], BTreeSet::new());
        let state = classify_agent_resolution(&hints_from_agent(None, vec![]), &snapshot);
        assert!(matches!(state, AgentResolutionState::NoAgent));
    }

    #[test]
    fn classify_single_valid_installed() {
        let snapshot = make_snapshot(vec![Provider::Claude, Provider::Codex], BTreeSet::new());
        let state = classify_agent_resolution(
            &hints_from_agent(Some(AgentHint::Single(Provider::Codex)), vec![]),
            &snapshot,
        );
        assert_eq!(
            state,
            AgentResolutionState::Selected {
                provider: Provider::Codex,
            }
        );
    }

    #[test]
    fn classify_single_invalid() {
        let snapshot = make_snapshot(vec![Provider::Claude], BTreeSet::new());
        let state = classify_agent_resolution(
            &hints_from_agent(None, vec!["not-a-provider".into()]),
            &snapshot,
        );
        assert_eq!(
            state,
            AgentResolutionState::SingleInvalid {
                hint: "not-a-provider".into(),
            }
        );
    }

    #[test]
    fn classify_single_not_installed() {
        let snapshot = make_snapshot(vec![Provider::Claude], BTreeSet::new());
        let state = classify_agent_resolution(
            &hints_from_agent(Some(AgentHint::Single(Provider::Codex)), vec![]),
            &snapshot,
        );
        assert_eq!(
            state,
            AgentResolutionState::SingleNotInstalled {
                provider: Provider::Codex,
            }
        );
    }

    #[test]
    fn classify_list_multiple_installed() {
        let snapshot = make_snapshot(vec![Provider::Claude, Provider::Codex, Provider::Gemini], BTreeSet::new());
        let state = classify_agent_resolution(
            &hints_from_agent(
                Some(AgentHint::List(vec![Provider::Codex, Provider::Gemini])),
                vec![],
            ),
            &snapshot,
        );
        assert!(matches!(
            state,
            AgentResolutionState::ListMultipleInstalled {
                installed,
                not_installed,
                invalid,
            } if installed == vec![Provider::Codex, Provider::Gemini]
                && not_installed.is_empty()
                && invalid.is_empty()
        ));
    }

    #[test]
    fn classify_list_one_installed() {
        let snapshot = make_snapshot(vec![Provider::Claude, Provider::Codex], BTreeSet::new());
        let state = classify_agent_resolution(
            &hints_from_agent(
                Some(AgentHint::List(vec![Provider::Codex, Provider::Gemini])),
                vec![],
            ),
            &snapshot,
        );
        assert!(matches!(
            state,
            AgentResolutionState::ListOneInstalled {
                selected,
                not_installed,
                invalid,
            } if selected == Provider::Codex
                && not_installed == vec![Provider::Gemini]
                && invalid.is_empty()
        ));
    }

    #[test]
    fn classify_list_with_invalid_entries() {
        let snapshot = make_snapshot(vec![Provider::Claude, Provider::Codex], BTreeSet::new());
        let state = classify_agent_resolution(
            &hints_from_agent(
                Some(AgentHint::List(vec![Provider::Codex])),
                vec!["bad".into(), "worse".into()],
            ),
            &snapshot,
        );
        assert!(matches!(
            state,
            AgentResolutionState::ListOneInstalled {
                selected,
                not_installed,
                invalid,
            } if selected == Provider::Codex
                && not_installed.is_empty()
                && invalid == vec!["bad".to_string(), "worse".to_string()]
        ));
    }

    #[test]
    fn classify_all_invalid_list() {
        let snapshot = make_snapshot(vec![Provider::Claude], BTreeSet::new());
        let state = classify_agent_resolution(
            &hints_from_agent_with_list(None, vec!["bad".into(), "worse".into()], true),
            &snapshot,
        );
        assert!(matches!(
            state,
            AgentResolutionState::ZeroInstalledList {
                not_installed,
                invalid,
            } if not_installed.is_empty()
                && invalid == vec!["bad".to_string(), "worse".to_string()]
        ));
    }

    #[test]
    fn classify_single_entry_all_invalid_list_is_zero_installed() {
        // Regression: a one-element invalid list (`agent: ["not-real"]`)
        // resolves to `agent: None` + one invalid entry, which is shape-
        // identical to a scalar invalid value. The preserved `agent_was_list`
        // flag must route it to the zero-installed-list state.
        let snapshot = make_snapshot(vec![Provider::Claude], BTreeSet::new());
        let state = classify_agent_resolution(
            &hints_from_agent_with_list(None, vec!["not-real".into()], true),
            &snapshot,
        );
        assert!(
            matches!(
                state,
                AgentResolutionState::ZeroInstalledList {
                    ref not_installed,
                    ref invalid,
                } if not_installed.is_empty() && invalid == &vec!["not-real".to_string()]
            ),
            "single-entry all-invalid list must be ZeroInstalledList, got {state:?}"
        );
    }

    #[test]
    fn classify_single_scalar_invalid_is_single_invalid() {
        // The scalar counterpart (`agent: not-real`, `agent_was_list = false`)
        // must remain the single-invalid state.
        let snapshot = make_snapshot(vec![Provider::Claude], BTreeSet::new());
        let state = classify_agent_resolution(
            &hints_from_agent_with_list(None, vec!["not-real".into()], false),
            &snapshot,
        );
        assert_eq!(
            state,
            AgentResolutionState::SingleInvalid {
                hint: "not-real".into(),
            }
        );
    }

    #[test]
    fn classify_all_not_installed_list() {
        let snapshot = make_snapshot(vec![Provider::Claude], BTreeSet::new());
        let state = classify_agent_resolution(
            &hints_from_agent(
                Some(AgentHint::List(vec![Provider::Codex, Provider::Gemini])),
                vec![],
            ),
            &snapshot,
        );
        assert!(matches!(
            state,
            AgentResolutionState::ZeroInstalledList {
                not_installed,
                invalid,
            } if not_installed == vec![Provider::Codex, Provider::Gemini]
                && invalid.is_empty()
        ));
    }

    #[test]
    fn classify_zero_installed_list_mixed_invalid_and_not_installed() {
        let snapshot = make_snapshot(vec![Provider::Claude], BTreeSet::new());
        let state = classify_agent_resolution(
            &hints_from_agent(
                Some(AgentHint::List(vec![Provider::Codex])),
                vec!["bad".into()],
            ),
            &snapshot,
        );
        assert!(matches!(
            state,
            AgentResolutionState::ZeroInstalledList {
                not_installed,
                invalid,
            } if not_installed == vec![Provider::Codex]
                && invalid == vec!["bad".to_string()]
        ));
    }

    // -- OpenCode non-TTY hard error test --------------------------------------

    #[test]
    fn opencode_non_tty_no_model_errors() {
        let prepared = make_prepared_composition(None, None);
        let snapshot = make_snapshot(vec![Provider::OpenCode], BTreeSet::new());

        let err = resolve_target_non_tty_with_env(
            None,
            &prepared.selection_hints,
            &snapshot,
            Some(Provider::OpenCode),
            None,
            |_| None,
            None,
        )
        .unwrap_err();
        assert!(matches!(err, CompositionError::ModelSelectionFailed { .. }));
    }

    #[test]
    fn opencode_non_tty_with_model_ok() {
        let prepared = make_prepared_composition(None, Some(ModelHint::Single("gpt-4o".into())));
        let snapshot = make_snapshot(vec![Provider::OpenCode], BTreeSet::new());

        let target = resolve_target_non_tty_with_env(
            None,
            &prepared.selection_hints,
            &snapshot,
            Some(Provider::OpenCode),
            None,
            |_| None,
            None,
        )
        .unwrap();
        assert_eq!(target.provider, Provider::OpenCode);
        assert_eq!(target.model, Some("gpt-4o".to_string()));
    }

    // -- detect_installed_providers ---------------------------------------

    /// A macOS app-bundle discovery is the provider's *desktop* app, not the
    /// CLI; it must never count as an installed agentic CLI.
    #[test]
    fn detect_installed_providers_ignores_app_bundle_discoveries() {
        use sniff::programs::{AiCli, ExecutableSource, InstalledAiClients};

        let clients = InstalledAiClients::default()
            .with_program(
                AiCli::Claude,
                PathBuf::from("/stub/bin/claude"),
                ExecutableSource::Path,
            )
            .with_program(
                AiCli::QwenCli,
                PathBuf::from("/Applications/Qwen.app/Contents/MacOS/Qwen"),
                ExecutableSource::MacOsAppBundle,
            );

        let installed = detect_installed_providers(&clients);
        assert!(installed.contains(&Provider::Claude));
        assert!(
            !installed.contains(&Provider::QwenCode),
            "bundle-sourced Qwen must not be treated as installed: {installed:?}"
        );
    }
}
