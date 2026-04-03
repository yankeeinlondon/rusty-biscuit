//! Provider selection for composition workflows.

use std::collections::BTreeSet;

use crate::events::Provider;

use super::error::CompositionError;
use super::types::{PreparedComposition, SelectedProvider, SelectionReason};

/// Build the set of candidate providers by filtering installed providers.
///
/// Removes excluded providers and returns only those that have a wrapper profile.
pub fn build_candidate_set(installed: &[Provider], excluded: &BTreeSet<Provider>) -> Vec<Provider> {
    installed
        .iter()
        .copied()
        .filter(|p| !excluded.contains(p))
        // RooCode is a VS Code extension, not a wrappable CLI
        .filter(|p| *p != Provider::RooCode)
        .collect()
}

/// Select a provider for composition execution.
///
/// Precedence:
/// 1. Explicit provider (from `--claude`, `--codex`, etc.)
/// 2. Single installed candidate after exclusion
/// 3. Effective frontmatter `agent` hint (from composed state)
/// 4. Config favorite (`settings.linking.preference[0]`)
/// 5. Error: interactive selection required
pub fn select_provider(
    explicit_provider: Option<Provider>,
    prepared: &PreparedComposition,
    installed: &[Provider],
    excluded: &BTreeSet<Provider>,
    favorite: Option<Provider>,
) -> Result<SelectedProvider, CompositionError> {
    let candidates = build_candidate_set(installed, excluded);
    let runnable_installed = build_candidate_set(installed, &BTreeSet::new());
    if runnable_installed.is_empty() {
        return Err(CompositionError::NoRunnableProviders);
    }

    // 1. Explicit provider
    if let Some(provider) = explicit_provider {
        if runnable_installed.contains(&provider) {
            return Ok(SelectedProvider {
                provider,
                reason: SelectionReason::ExplicitProvider,
            });
        }
        return Err(CompositionError::NoRunnableProviders);
    }

    if candidates.is_empty() {
        return Err(CompositionError::NoRunnableProviders);
    }

    // 2. Single installed candidate
    if candidates.len() == 1 {
        return Ok(SelectedProvider {
            provider: candidates[0],
            reason: SelectionReason::SingleInstalled,
        });
    }

    // 3. Effective frontmatter agent hint
    if let Some(ref hint) = prepared.effective_agent_hint
        && let Some(selected) = resolve_agent_hint(hint, &candidates)?
    {
        return Ok(selected);
    }

    // 4. Config favorite
    if let Some(fav) = favorite
        && candidates.contains(&fav)
    {
        return Ok(SelectedProvider {
            provider: fav,
            reason: SelectionReason::ConfigFavorite,
        });
    }

    // 5. No automatic selection possible
    Err(CompositionError::InteractiveSelectionRequired)
}

fn resolve_agent_hint(
    hint: &serde_json::Value,
    candidates: &[Provider],
) -> Result<Option<SelectedProvider>, CompositionError> {
    match hint {
        serde_json::Value::String(s) => {
            let lower = s.to_lowercase();
            if lower == "interactive" {
                return Err(CompositionError::InteractiveSelectionRequired);
            }

            let matches = Provider::fuzzy_match_all(s);
            if matches.is_empty() {
                return Err(CompositionError::AgentHintInvalid(s.clone()));
            }
            let candidate_matches: Vec<Provider> = matches
                .into_iter()
                .filter(|p| candidates.contains(p))
                .collect();

            match candidate_matches.len() {
                0 => Ok(None),
                1 => Ok(Some(SelectedProvider {
                    provider: candidate_matches[0],
                    reason: SelectionReason::FrontmatterHint,
                })),
                _ => Err(CompositionError::AgentHintAmbiguous {
                    hint: s.clone(),
                    matches: candidate_matches
                        .iter()
                        .map(|p| p.to_string())
                        .collect::<Vec<_>>()
                        .join(", "),
                    providers: candidate_matches,
                }),
            }
        }
        serde_json::Value::Bool(true) => Err(CompositionError::InteractiveSelectionRequired),
        other => Err(CompositionError::AgentHintInvalid(format!(
            "unexpected type: {}",
            json_type_name(other)
        ))),
    }
}

fn json_type_name(value: &serde_json::Value) -> &'static str {
    match value {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "boolean",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::String(_) => "string",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Object(_) => "object",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::composition::types::CompositionMode;
    use serde_json::json;
    use std::path::PathBuf;

    fn make_prepared_composition(agent_hint: Option<serde_json::Value>) -> PreparedComposition {
        use super::super::lifecycle::LifecycleConfig;
        use super::super::types::CompositionClosurePlan;
        PreparedComposition {
            mode: CompositionMode::ChainedDocument,
            resolved_path: PathBuf::from("/tmp/test.md"),
            source_repo_root: None,
            prompt: "test prompt".to_string(),
            effective_frontmatter: json!({}),
            effective_agent_hint: agent_hint,
            closure: CompositionClosurePlan::Direct,
            lifecycle: LifecycleConfig::default(),
        }
    }

    #[test]
    fn exclusion_filters_providers() {
        let candidates = build_candidate_set(
            &[Provider::Claude, Provider::Codex, Provider::Gemini],
            &[Provider::Codex].iter().copied().collect(),
        );
        assert_eq!(candidates, vec![Provider::Claude, Provider::Gemini]);
    }

    #[test]
    fn roocode_excluded_from_candidates() {
        let candidates =
            build_candidate_set(&[Provider::Claude, Provider::RooCode], &BTreeSet::new());
        assert_eq!(candidates, vec![Provider::Claude]);
    }

    #[test]
    fn explicit_provider_selected() {
        let prepared = make_prepared_composition(None);
        let installed = vec![Provider::Claude, Provider::Codex];

        let result = select_provider(
            Some(Provider::Claude),
            &prepared,
            &installed,
            &BTreeSet::new(),
            None,
        )
        .unwrap();
        assert_eq!(result.provider, Provider::Claude);
        assert_eq!(result.reason, SelectionReason::ExplicitProvider);
    }

    #[test]
    fn explicit_provider_ignores_exclusion() {
        let prepared = make_prepared_composition(None);
        let installed = vec![Provider::Claude, Provider::Codex];
        let excluded: BTreeSet<Provider> = [Provider::Claude].into_iter().collect();

        let result = select_provider(
            Some(Provider::Claude),
            &prepared,
            &installed,
            &excluded,
            None,
        )
        .unwrap();

        assert_eq!(result.provider, Provider::Claude);
        assert_eq!(result.reason, SelectionReason::ExplicitProvider);
    }

    #[test]
    fn single_installed_selected() {
        let prepared = make_prepared_composition(None);
        let installed = vec![Provider::Claude];

        let result = select_provider(None, &prepared, &installed, &BTreeSet::new(), None).unwrap();
        assert_eq!(result.provider, Provider::Claude);
        assert_eq!(result.reason, SelectionReason::SingleInstalled);
    }

    #[test]
    fn frontmatter_hint_from_effective() {
        let prepared = make_prepared_composition(Some(json!("codex")));
        let installed = vec![Provider::Claude, Provider::Codex];

        let result = select_provider(None, &prepared, &installed, &BTreeSet::new(), None).unwrap();
        assert_eq!(result.provider, Provider::Codex);
        assert_eq!(result.reason, SelectionReason::FrontmatterHint);
    }

    #[test]
    fn config_favorite_selected() {
        let prepared = make_prepared_composition(None);
        let installed = vec![Provider::Claude, Provider::Codex, Provider::Gemini];

        let result = select_provider(
            None,
            &prepared,
            &installed,
            &BTreeSet::new(),
            Some(Provider::Gemini),
        )
        .unwrap();
        assert_eq!(result.provider, Provider::Gemini);
        assert_eq!(result.reason, SelectionReason::ConfigFavorite);
    }

    #[test]
    fn config_favorite_not_installed_falls_through() {
        let prepared = make_prepared_composition(None);
        let installed = vec![Provider::Claude, Provider::Codex];

        // Favorite is Gemini, but it's not installed
        let err = select_provider(
            None,
            &prepared,
            &installed,
            &BTreeSet::new(),
            Some(Provider::Gemini),
        )
        .unwrap_err();
        assert!(matches!(
            err,
            CompositionError::InteractiveSelectionRequired
        ));
    }

    #[test]
    fn installed_but_excluded_hint_falls_through_to_favorite() {
        let prepared = make_prepared_composition(Some(json!("codex")));
        let installed = vec![Provider::Claude, Provider::Codex, Provider::Gemini];
        let excluded: BTreeSet<Provider> = [Provider::Codex].into_iter().collect();

        let result = select_provider(
            None,
            &prepared,
            &installed,
            &excluded,
            Some(Provider::Claude),
        )
        .unwrap();

        assert_eq!(result.provider, Provider::Claude);
        assert_eq!(result.reason, SelectionReason::ConfigFavorite);
    }

    #[test]
    fn no_automatic_selection_requires_interactive() {
        let prepared = make_prepared_composition(None);
        let installed = vec![Provider::Claude, Provider::Codex];

        let err = select_provider(None, &prepared, &installed, &BTreeSet::new(), None).unwrap_err();
        assert!(matches!(
            err,
            CompositionError::InteractiveSelectionRequired
        ));
    }

    #[test]
    fn ambiguous_hint_errors() {
        // "c" matches Claude and Codex
        let prepared = make_prepared_composition(Some(json!("c")));
        let installed = vec![Provider::Claude, Provider::Codex];

        let err = select_provider(None, &prepared, &installed, &BTreeSet::new(), None).unwrap_err();
        assert!(matches!(err, CompositionError::AgentHintAmbiguous { .. }));
    }

    #[test]
    fn exclusion_narrows_to_single() {
        let prepared = make_prepared_composition(None);
        let installed = vec![Provider::Claude, Provider::Codex];
        let excluded: BTreeSet<Provider> = [Provider::Claude].into_iter().collect();

        let result = select_provider(None, &prepared, &installed, &excluded, None).unwrap();
        assert_eq!(result.provider, Provider::Codex);
        assert_eq!(result.reason, SelectionReason::SingleInstalled);
    }

    #[test]
    fn hint_matches_known_but_uninstalled_provider_falls_through_to_favorite() {
        // Hint is "gemini" which is a known provider, but Gemini is not installed.
        let prepared = make_prepared_composition(Some(json!("gemini")));
        let installed = vec![Provider::Claude, Provider::Codex];

        // Falls through hint (no candidate match) → config favorite
        let result = select_provider(
            None,
            &prepared,
            &installed,
            &BTreeSet::new(),
            Some(Provider::Claude),
        )
        .unwrap();
        assert_eq!(result.provider, Provider::Claude);
        assert_eq!(result.reason, SelectionReason::ConfigFavorite);
    }

    #[test]
    fn hint_matches_known_but_uninstalled_provider_no_favorite_requires_interactive() {
        // Hint is "gemini" (known but not installed), no favorite configured.
        let prepared = make_prepared_composition(Some(json!("gemini")));
        let installed = vec![Provider::Claude, Provider::Codex];

        let err = select_provider(None, &prepared, &installed, &BTreeSet::new(), None).unwrap_err();
        assert!(matches!(
            err,
            CompositionError::InteractiveSelectionRequired
        ));
    }
}
