pub mod loader;
mod matcher;
mod runner;
pub mod template;

use std::path::Path;

use serde_json::Value;
use tracing::{debug, info};

use crate::actions::HookResponse;
use crate::adapters::{self, AdapterError};
use crate::error::Result;
use crate::events::{EnvironmentContext, Provider, ResolvedHook};

/// Result of dispatching a single incoming provider event.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DispatchOutcome {
    /// Provider-native response payload for blocking hooks.
    pub response: Option<Value>,
    /// Optional process exit code for shell-based providers.
    pub exit_code: Option<i32>,
}

/// Main dispatch entry point.
///
/// Parses raw provider JSON, resolves configured actions, executes them,
/// and returns any provider-native blocking response.
pub async fn dispatch(
    raw: &Value,
    provider: Provider,
    env: &EnvironmentContext,
) -> Result<DispatchOutcome> {
    let adapter = adapters::adapter_for(provider);

    let (event, mut meta) = match adapter.parse_event(raw) {
        Ok(parsed) => parsed,
        Err(AdapterError::UnknownEvent(_)) => {
            debug!(%provider, "Adapter returned unknown event, skipping dispatch");
            return Ok(DispatchOutcome::default());
        }
        Err(error) => return Err(error.into()),
    };

    meta.env = env.clone();

    info!(%provider, %event, "Dispatching event");

    let config = match loader::load_runtime_config(None, runtime_repo_root(env)) {
        Ok(config) => config,
        Err(crate::error::ClaudineError::ConfigNotFound(_)) => {
            debug!("No .claudine config found, skipping dispatch");
            return Ok(DispatchOutcome::default());
        }
        Err(error) => return Err(error),
    };

    let binding = match config.get_binding(provider, &event) {
        Some(binding) => binding,
        None => {
            debug!(%event, %provider, "No binding found for event/provider, skipping");
            return Ok(DispatchOutcome::default());
        }
    };

    if !binding.enabled() {
        debug!(%event, %provider, "Binding disabled, skipping");
        return Ok(DispatchOutcome::default());
    }

    if binding.actions().is_empty() {
        debug!(%event, "No actions configured, skipping");
        return Ok(DispatchOutcome::default());
    }

    if !matcher::matches_with_regex(binding.matcher(), &meta) {
        debug!(%event, "Matcher did not match, skipping");
        return Ok(DispatchOutcome::default());
    }

    let resolved_hook = ResolvedHook {
        event,
        meta,
        provider,
        actions: binding.actions().to_vec(),
        can_block: adapter.can_block(&event),
    };

    info!(
        event = %resolved_hook.event,
        provider = %resolved_hook.provider,
        action_count = resolved_hook.actions.len(),
        can_block = resolved_hook.can_block,
        "Executing resolved hook"
    );

    let action_response = runner::execute_actions(
        &resolved_hook.actions,
        Some(binding.compiled_mappers()),
        &resolved_hook.meta,
        config.settings(),
        resolved_hook.can_block,
    )
    .await?;

    finalize_response(
        adapter,
        &resolved_hook.event,
        resolved_hook.can_block,
        action_response,
    )
}

fn runtime_repo_root(env: &EnvironmentContext) -> Option<&Path> {
    env.git
        .as_ref()
        .map(|git| git.repo_root.as_path())
        .or_else(|| env.repo.as_ref().map(|repo| repo.root.as_path()))
}

fn finalize_response(
    adapter: &dyn adapters::ProviderAdapter,
    event: &crate::events::AgenticEvent,
    can_block: bool,
    response: Option<HookResponse>,
) -> Result<DispatchOutcome> {
    if !can_block {
        return Ok(DispatchOutcome::default());
    }

    let Some(response) = response else {
        return Ok(DispatchOutcome::default());
    };

    let payload = adapter.format_response(event, &response)?;
    let exit_code = adapter.exit_code(event, &response);

    let response_payload = if payload.is_null() {
        None
    } else {
        Some(payload)
    };

    Ok(DispatchOutcome {
        response: response_payload,
        exit_code,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::*;
    use crate::events::*;
    use serde_json::json;
    use std::collections::HashMap;

    #[tokio::test]
    async fn dispatch_returns_default_for_unknown_event() {
        let raw = json!({"hook_event_name": "CompletelyNewEvent"});
        let env = EnvironmentContext::default();

        let outcome = dispatch(&raw, Provider::Claude, &env).await.unwrap();
        assert_eq!(outcome, DispatchOutcome::default());
    }

    #[tokio::test]
    async fn dispatch_returns_default_when_no_config() {
        let raw = json!({
            "hook_event_name": "SessionStart",
            "session_id": "test-123"
        });
        let env = EnvironmentContext::default();

        let outcome = dispatch(&raw, Provider::Claude, &env).await.unwrap();
        assert_eq!(outcome, DispatchOutcome::default());
    }

    #[tokio::test]
    async fn dispatch_loads_repo_scoped_config_from_environment_context() {
        let repo = tempfile::tempdir().unwrap();
        let log_path = repo.path().join("repo-events.jsonl");

        let mut claude_config = ProviderConfig::default();
        claude_config.events.insert(
            AgenticEvent::SessionStart,
            EventBinding {
                enabled: true,
                actions: vec![HookAction::Log {
                    target: LogTarget::File {
                        path: Some(log_path.clone()),
                        rotate_daily: false,
                    },
                }],
                matcher: None,
            },
        );

        let config = HookerConfig {
            version: "1.0".to_string(),
            settings: GlobalSettings {
                default_log_target: None,
                tts: None,
                linking: Some(LinkingSettings {
                    preference: vec![],
                    canonical_provider: CanonicalProviderSettings {
                        repo_skill: Some(Provider::Claude),
                        ..CanonicalProviderSettings::default()
                    },
                }),
            },
            providers: {
                let mut providers = HashMap::new();
                providers.insert(Provider::Claude, claude_config);
                providers
            },
        };

        let config_path = repo.path().join(".claudine/config.json");
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&config_path, serde_json::to_string(&config).unwrap()).unwrap();

        let raw = json!({
            "hook_event_name": "SessionStart",
            "session_id": "repo-scoped-123"
        });
        let env = EnvironmentContext {
            git: Some(GitContext {
                repo_root: repo.path().to_path_buf(),
                branch: None,
                is_dirty: false,
                staged_count: 0,
                unstaged_count: 0,
                untracked_count: 0,
                head_sha: None,
                head_message: None,
                user_name: None,
                user_email: None,
                remote_name: None,
                remote_url: None,
                hosting_provider: None,
                repo_name: None,
                repo_org: None,
            }),
            ..EnvironmentContext::default()
        };

        let outcome = dispatch(&raw, Provider::Claude, &env).await.unwrap();
        assert_eq!(outcome, DispatchOutcome::default());

        let content = std::fs::read_to_string(log_path).unwrap();
        assert!(content.contains("repo-scoped-123"));
    }

    #[test]
    fn loader_with_explicit_path() {
        let tmp = tempfile::tempdir().unwrap();

        let mut claude_config = ProviderConfig::default();
        claude_config.events.insert(
            AgenticEvent::SessionStart,
            EventBinding {
                enabled: true,
                actions: vec![HookAction::Report { handler: None }],
                matcher: None,
            },
        );

        let config = HookerConfig {
            version: "1.0".to_string(),
            settings: GlobalSettings::default(),
            providers: {
                let mut providers = HashMap::new();
                providers.insert(Provider::Claude, claude_config);
                providers
            },
        };

        let path = tmp.path().join(".claudine/config.json");
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&path, serde_json::to_string(&config).unwrap()).unwrap();

        let loaded = loader::load_config(Some(&path), None).unwrap();
        assert!(loaded.providers.contains_key(&Provider::Claude));
        assert_eq!(loaded.providers[&Provider::Claude].events.len(), 1);
    }

    #[test]
    fn matcher_with_pattern() {
        let meta = EventMeta {
            provider: Provider::Claude,
            event: AgenticEvent::BeforeTool,
            timestamp: chrono::Utc::now(),
            session_id: None,
            cwd: None,
            tool_name: Some("Bash".to_string()),
            tool_input: None,
            tool_response: None,
            error: None,
            prompt: None,
            agent_type: None,
            notification_type: None,
            notification_message: None,
            extra: HashMap::new(),
            env: EnvironmentContext::default(),
        };

        assert!(matcher::matches_with_pattern(Some("Bash|Edit"), &meta));
        assert!(!matcher::matches_with_pattern(Some("Read"), &meta));
    }
}
