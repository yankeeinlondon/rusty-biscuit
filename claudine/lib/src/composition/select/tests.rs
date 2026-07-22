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
        schema_verdict_deferred: false,
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
        input_layers: Default::default(),
        entry: crate::composition::DocumentEntryReason::Direct,
        compose_context: darkmatter::markdown::compose::ComposeContext::capture_for_content(std::path::Path::new("."), ""),
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
