//! Provider selection for composition workflows.

use std::collections::BTreeSet;

use crate::events::Provider;
use crate::linking::ranked_provider_preferences;

use super::error::CompositionError;
use super::types::{CompositionRequest, PreparedPrompt, SelectedProvider, SelectionReason};

/// Select a provider for composition execution.
///
/// Precedence:
/// 1. Explicit provider (from wrapper subcommand)
/// 2. `AGENT` environment variable
/// 3. Frontmatter `agent` hint
/// 4. Ranked provider preferences (with fallback)
pub fn select_provider(
    request: &CompositionRequest,
    prepared: &PreparedPrompt,
    installed: &[Provider],
    preference: &[Provider],
) -> Result<SelectedProvider, CompositionError> {
    let agent_env = std::env::var("AGENT").ok();
    select_provider_with_env(request, prepared, installed, preference, agent_env.as_deref())
}

/// Core selection logic with the AGENT env value passed explicitly (for testability).
pub fn select_provider_with_env(
    request: &CompositionRequest,
    prepared: &PreparedPrompt,
    installed: &[Provider],
    preference: &[Provider],
    agent_env: Option<&str>,
) -> Result<SelectedProvider, CompositionError> {
    let candidates = build_candidate_set(installed, &request.excluded);
    if candidates.is_empty() {
        return Err(CompositionError::NoRunnableProviders);
    }

    // 1. Explicit provider
    if let Some(provider) = request.explicit_provider {
        if candidates.contains(&provider) {
            return Ok(SelectedProvider {
                provider,
                reason: SelectionReason::ExplicitProvider,
            });
        }
        return Err(CompositionError::NoRunnableProviders);
    }

    // 2. AGENT env override
    if let Some(agent_val) = agent_env {
        let trimmed = agent_val.trim();
        if !trimmed.is_empty() {
            let matches = Provider::fuzzy_match_all(trimmed);
            let candidate_matches: Vec<Provider> = matches
                .into_iter()
                .filter(|p| candidates.contains(p))
                .collect();

            if candidate_matches.len() == 1 {
                return Ok(SelectedProvider {
                    provider: candidate_matches[0],
                    reason: SelectionReason::EnvironmentOverride,
                });
            }
            // Multiple or zero matches from env — fall through to next rule
        }
    }

    // 3. Frontmatter agent hint
    if let Some(ref hint) = prepared.source_agent_hint {
        return resolve_agent_hint(hint, &candidates, request.force_interactive_selection);
    }

    // 4. Force interactive
    if request.force_interactive_selection {
        return Err(CompositionError::InteractiveSelectionRequired);
    }

    // 5. Preference fallback
    let ranked = ranked_provider_preferences(&candidates, preference);
    if let Some(&provider) = ranked.first() {
        return Ok(SelectedProvider {
            provider,
            reason: SelectionReason::PreferenceFallback,
        });
    }

    Err(CompositionError::NoRunnableProviders)
}

/// Build the set of candidate providers by filtering installed providers.
///
/// Removes excluded providers and returns only those that have a wrapper profile.
pub fn build_candidate_set(
    installed: &[Provider],
    excluded: &BTreeSet<Provider>,
) -> Vec<Provider> {
    installed
        .iter()
        .copied()
        .filter(|p| !excluded.contains(p))
        // RooCode is a VS Code extension, not a wrappable CLI
        .filter(|p| *p != Provider::RooCode)
        .collect()
}

fn resolve_agent_hint(
    hint: &serde_json::Value,
    candidates: &[Provider],
    _force_interactive: bool,
) -> Result<SelectedProvider, CompositionError> {
    match hint {
        serde_json::Value::String(s) => {
            let lower = s.to_lowercase();
            if lower == "interactive" {
                return Err(CompositionError::InteractiveSelectionRequired);
            }

            let matches = Provider::fuzzy_match_all(s);
            let candidate_matches: Vec<Provider> = matches
                .into_iter()
                .filter(|p| candidates.contains(p))
                .collect();

            match candidate_matches.len() {
                0 => Err(CompositionError::AgentHintInvalid(s.clone())),
                1 => Ok(SelectedProvider {
                    provider: candidate_matches[0],
                    reason: SelectionReason::FrontmatterHint,
                }),
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
        serde_json::Value::Bool(true) => {
            Err(CompositionError::InteractiveSelectionRequired)
        }
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
    use crate::composition::types::{CompositionMode, PreparedPrompt};
    use serde_json::json;
    use std::path::PathBuf;

    fn make_prepared(agent_hint: Option<serde_json::Value>) -> PreparedPrompt {
        PreparedPrompt {
            mode: CompositionMode::ChainedDocument,
            resolved_path: PathBuf::from("/tmp/test.md"),
            prompt: "test prompt".to_string(),
            source_agent_hint: agent_hint,
        }
    }

    fn make_request(
        explicit: Option<Provider>,
        excluded: &[Provider],
        interactive: bool,
    ) -> CompositionRequest {
        CompositionRequest {
            mode: CompositionMode::ChainedDocument,
            file_ref: "test.md".to_string(),
            explicit_provider: explicit,
            excluded: excluded.iter().copied().collect(),
            force_interactive_selection: interactive,
        }
    }

    #[test]
    fn explicit_provider_selected() {
        let request = make_request(Some(Provider::Claude), &[], false);
        let prepared = make_prepared(None);
        let installed = vec![Provider::Claude, Provider::Codex];

        let result = select_provider_with_env(&request, &prepared, &installed, &[], None).unwrap();
        assert_eq!(result.provider, Provider::Claude);
        assert_eq!(result.reason, SelectionReason::ExplicitProvider);
    }

    #[test]
    fn explicit_provider_excluded_fails() {
        let request = make_request(Some(Provider::Claude), &[Provider::Claude], false);
        let prepared = make_prepared(None);
        let installed = vec![Provider::Claude, Provider::Codex];

        let err = select_provider_with_env(&request, &prepared, &installed, &[], None).unwrap_err();
        assert!(matches!(err, CompositionError::NoRunnableProviders));
    }

    #[test]
    fn frontmatter_hint_selects_provider() {
        let request = make_request(None, &[], false);
        let prepared = make_prepared(Some(json!("claude")));
        let installed = vec![Provider::Claude, Provider::Codex];

        let result = select_provider_with_env(&request, &prepared, &installed, &[], None).unwrap();
        assert_eq!(result.provider, Provider::Claude);
        assert_eq!(result.reason, SelectionReason::FrontmatterHint);
    }

    #[test]
    fn frontmatter_hint_invalid() {
        let request = make_request(None, &[], false);
        let prepared = make_prepared(Some(json!("nonexistent")));
        let installed = vec![Provider::Claude, Provider::Codex];

        let err = select_provider_with_env(&request, &prepared, &installed, &[], None).unwrap_err();
        assert!(matches!(err, CompositionError::AgentHintInvalid(_)));
    }

    #[test]
    fn frontmatter_true_requires_interactive() {
        let request = make_request(None, &[], false);
        let prepared = make_prepared(Some(json!(true)));
        let installed = vec![Provider::Claude, Provider::Codex];

        let err = select_provider_with_env(&request, &prepared, &installed, &[], None).unwrap_err();
        assert!(matches!(
            err,
            CompositionError::InteractiveSelectionRequired
        ));
    }

    #[test]
    fn preference_fallback_uses_ranked_order() {
        let request = make_request(None, &[], false);
        let prepared = make_prepared(None);
        let installed = vec![Provider::Claude, Provider::Codex, Provider::Gemini];
        let preference = vec![Provider::Codex, Provider::Claude];

        let result =
            select_provider_with_env(&request, &prepared, &installed, &preference, None).unwrap();
        assert_eq!(result.provider, Provider::Codex);
        assert_eq!(result.reason, SelectionReason::PreferenceFallback);
    }

    #[test]
    fn no_providers_fails() {
        let request = make_request(None, &[], false);
        let prepared = make_prepared(None);
        let installed: Vec<Provider> = Vec::new();

        let err = select_provider_with_env(&request, &prepared, &installed, &[], None).unwrap_err();
        assert!(matches!(err, CompositionError::NoRunnableProviders));
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
        let candidates = build_candidate_set(
            &[Provider::Claude, Provider::RooCode],
            &BTreeSet::new(),
        );
        assert_eq!(candidates, vec![Provider::Claude]);
    }

    #[test]
    fn force_interactive_with_no_hint() {
        let request = make_request(None, &[], true);
        let prepared = make_prepared(None);
        let installed = vec![Provider::Claude, Provider::Codex];

        let err = select_provider_with_env(&request, &prepared, &installed, &[], None).unwrap_err();
        assert!(matches!(
            err,
            CompositionError::InteractiveSelectionRequired
        ));
    }
}
