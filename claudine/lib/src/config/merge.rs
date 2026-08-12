//! Repo-vs-user configuration merge logic.

use crate::config::claudine_config::{ClaudineConfig, RepoOverrideConfig};

/// Merge a repo-level [`RepoOverrideConfig`] into a user-level config.
///
/// Merge rules:
/// - `canonical_provider`: repo overrides user if repo has `Some`.
/// - `actions`: per-event replacement — if repo defines actions for an event,
///   that vector fully replaces the user's entry for the same event.
/// - `matchers`: per-event replacement — same as `actions`.
/// - `active_messenger`: repo overrides user's active config key only.
/// - `exit_expressions`: repo replaces user's declaration (the resolver
///   in `runaway::resolve_exit_expressions` interprets the repo's combine
///   mode; here we just hand the repo's value through as the resolved
///   "user-facing" view). If the repo did not declare `exit_expressions`,
///   the user value is preserved.
/// - `guard_settings`: repo fully replaces the user's scalar settings if
///   the repo declares them.
pub(crate) fn merge_repo_override(user: &mut ClaudineConfig, repo: &RepoOverrideConfig) {
    // canonical_provider: repo overrides user if set
    if repo.canonical_provider.is_some() {
        user.canonical_provider = repo.canonical_provider;
    }

    // actions: per-event replacement
    for (event, repo_actions) in &repo.actions {
        user.actions.insert(*event, repo_actions.clone());
    }

    // matchers: per-event replacement
    for (event, repo_matcher) in &repo.matchers {
        user.matchers.insert(*event, repo_matcher.clone());
    }

    // active_messenger: repo overrides the active config key only
    if let Some(override_value) = &repo.active_messenger
        && let Some(messenger) = &mut user.messenger
    {
        messenger.active_config = override_value.clone();
    }

    // exit_expressions: repo declaration replaces user's when present.
    // (The three-layer resolver interprets the combine mode; the merge
    // here only collapses user+repo into a single user-facing view that
    // later phases treat as "the user layer" — frontmatter is added on
    // top at run time.)
    if repo.exit_expressions.is_some() {
        user.exit_expressions = repo.exit_expressions.clone();
    }

    // guard_settings: repo fully replaces user's scalar settings.
    if let Some(repo_guards) = &repo.guard_settings {
        user.guard_settings = repo_guards.clone();
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::HookAction;
    use crate::config::messaging_block::{ClaudineMessengerConfig, MessengerProviderConfig};
    use crate::events::AgenticEvent;
    use crate::provider::Provider;
    use std::collections::HashMap;

    #[test]
    fn merge_repo_canonical_provider_overrides_user() {
        let mut user = ClaudineConfig {
            canonical_provider: Some(Provider::Claude),
            ..ClaudineConfig::default()
        };
        let repo = RepoOverrideConfig {
            canonical_provider: Some(Provider::Gemini),
            ..RepoOverrideConfig::default()
        };
        merge_repo_override(&mut user, &repo);
        assert_eq!(user.canonical_provider, Some(Provider::Gemini));
    }

    #[test]
    fn merge_repo_actions_replace_user_per_event() {
        let mut user = ClaudineConfig::default();
        user.actions.insert(
            AgenticEvent::SessionStart,
            vec![HookAction::SoundEffect {
                effect: "user-sound".to_string(),
                volume: 1.0,
                speed: 1.0,
                when: None,
            }],
        );
        user.actions.insert(
            AgenticEvent::TurnComplete,
            vec![HookAction::Report {
                handler: None,
                when: None,
            }],
        );

        let repo = RepoOverrideConfig {
            actions: std::collections::HashMap::from([(
                AgenticEvent::SessionStart,
                vec![HookAction::SoundEffect {
                    effect: "repo-sound".to_string(),
                    volume: 0.5,
                    speed: 1.0,
                    when: None,
                }],
            )]),
            ..RepoOverrideConfig::default()
        };

        merge_repo_override(&mut user, &repo);

        let session_start = &user.actions[&AgenticEvent::SessionStart];
        assert_eq!(session_start.len(), 1);
        if let HookAction::SoundEffect { effect, .. } = &session_start[0] {
            assert_eq!(effect, "repo-sound");
        } else {
            panic!("Expected SoundEffect");
        }

        assert!(user.actions.contains_key(&AgenticEvent::TurnComplete));
    }

    #[test]
    fn merge_repo_override_applies_active_messenger() {
        let mut user = ClaudineConfig {
            messenger: Some(ClaudineMessengerConfig {
                active_config: Some("personal".to_string()),
                configurations: {
                    let mut m = std::collections::HashMap::new();
                    m.insert(
                        "personal".to_string(),
                        MessengerProviderConfig::Discord {
                            channel_id: "123".to_string(),
                            bot_token_env: "TOKEN".to_string(),
                        },
                    );
                    m.insert(
                        "work".to_string(),
                        MessengerProviderConfig::Slack {
                            channel_id: "C456".to_string(),
                            bot_token_env: "SLACK_URL".to_string(),
                        },
                    );
                    m
                },
            }),
            ..Default::default()
        };

        let repo = RepoOverrideConfig {
            active_messenger: Some(Some("work".to_string())),
            ..Default::default()
        };

        merge_repo_override(&mut user, &repo);

        assert_eq!(
            user.messenger.as_ref().unwrap().active_config.as_deref(),
            Some("work"),
        );
    }

    #[test]
    fn merge_repo_override_disables_messenger_with_null() {
        let mut user = ClaudineConfig {
            messenger: Some(ClaudineMessengerConfig {
                active_config: Some("personal".to_string()),
                configurations: std::collections::HashMap::new(),
            }),
            ..Default::default()
        };

        let repo = RepoOverrideConfig {
            active_messenger: Some(None),
            ..Default::default()
        };

        merge_repo_override(&mut user, &repo);

        assert_eq!(user.messenger.as_ref().unwrap().active_config, None);
    }

    #[test]
    fn merge_repo_matchers_replace_user_per_event() {
        let mut user = ClaudineConfig::default();
        user.matchers
            .insert(AgenticEvent::BeforeTool, "Bash".to_string());
        user.matchers
            .insert(AgenticEvent::AfterTool, "Edit".to_string());

        let repo = RepoOverrideConfig {
            matchers: HashMap::from([(
                AgenticEvent::BeforeTool,
                "tool_name == 'Bash' && git.branch == 'main'".to_string(),
            )]),
            ..RepoOverrideConfig::default()
        };

        merge_repo_override(&mut user, &repo);

        assert_eq!(
            user.matchers.get(&AgenticEvent::BeforeTool),
            Some(&"tool_name == 'Bash' && git.branch == 'main'".to_string()),
        );
        assert_eq!(
            user.matchers.get(&AgenticEvent::AfterTool),
            Some(&"Edit".to_string()),
        );
    }
}
