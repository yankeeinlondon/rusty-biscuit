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
mod tests;
