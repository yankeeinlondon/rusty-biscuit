pub mod loader;
mod matcher;
mod resolver;
mod runner;
pub mod template;

use serde_json::Value;
use tracing::{debug, info};

use crate::adapters;
use crate::error::Result;
use crate::events::{EnvironmentContext, Provider};

/// Main dispatch entry point.
///
/// Parses raw provider JSON, matches against config bindings,
/// resolves actions (including provider overrides), and executes them.
///
/// ## Flow
///
/// 1. Select adapter for provider
/// 2. Parse raw JSON into normalized event + metadata
/// 3. Load hooker config (user + repo, merged)
/// 4. Look up event binding
/// 5. Resolve actions (with provider overrides)
/// 6. Check matcher against event metadata
/// 7. Execute resolved actions
///
/// ## Errors
///
/// Returns errors from config loading or action execution.
/// Unknown events or missing bindings return `Ok(())` silently.
pub async fn dispatch(
    raw: &Value,
    provider: Provider,
    env: &EnvironmentContext,
) -> Result<()> {
    // 1. Get adapter for this provider
    let adapter = adapters::adapter_for(provider);

    // 2. Parse the raw event payload
    let (event, meta) = match adapter.parse_event(raw, env) {
        Some(parsed) => parsed,
        None => {
            debug!(%provider, "Adapter returned None for raw event, skipping");
            return Ok(());
        }
    };

    info!(%provider, %event, "Dispatching event");

    // 3. Load merged config (user + repo)
    let config = match loader::load_config(None, None) {
        Ok(c) => c,
        Err(crate::error::ClaudineError::ConfigNotFound(_)) => {
            debug!("No hooker config found, skipping dispatch");
            return Ok(());
        }
        Err(e) => return Err(e),
    };

    // 4. Look up the event binding
    let binding = match config.events.get(&event) {
        Some(b) => b,
        None => {
            debug!(%event, "No binding found for event, skipping");
            return Ok(());
        }
    };

    // 5. Resolve actions (check overrides)
    let (enabled, actions, matcher_override) =
        resolver::resolve_actions(binding, provider);

    if !enabled {
        debug!(%event, %provider, "Binding disabled, skipping");
        return Ok(());
    }

    if actions.is_empty() {
        debug!(%event, "No actions configured, skipping");
        return Ok(());
    }

    // 6. Check matcher (use override matcher if present, otherwise binding matcher)
    let effective_matcher = matcher_override.or(binding.matcher.as_deref());
    if !matcher::matches_with_pattern(effective_matcher, &meta) {
        debug!(%event, "Matcher did not match, skipping");
        return Ok(());
    }

    // 7. Collect owned actions for execution
    let action_refs: Vec<_> = actions.into_iter().cloned().collect();

    info!(
        %event,
        %provider,
        action_count = action_refs.len(),
        "Executing actions"
    );

    // 8. Execute
    runner::execute_actions(&action_refs, &meta, &config.settings).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::*;
    use serde_json::json;
    use std::collections::HashMap;

    #[tokio::test]
    async fn dispatch_returns_ok_for_unknown_event() {
        let raw = json!({"hook_event_name": "CompletelyNewEvent"});
        let env = EnvironmentContext::default();

        let result = dispatch(&raw, Provider::Claude, &env).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn dispatch_returns_ok_when_no_config() {
        let raw = json!({
            "hook_event_name": "SessionStart",
            "session_id": "test-123"
        });
        let env = EnvironmentContext::default();

        // This will likely fail to find config, but should not error fatally
        let result = dispatch(&raw, Provider::Claude, &env).await;
        assert!(result.is_ok());
    }

    #[test]
    fn loader_with_explicit_path() {
        let tmp = tempfile::tempdir().unwrap();
        let config = HookerConfig {
            version: "1.0".to_string(),
            settings: GlobalSettings::default(),
            events: {
                let mut m = HashMap::new();
                m.insert(
                    AgenticEvent::SessionStart,
                    EventBinding {
                        enabled: true,
                        actions: vec![EventAction::Report { handler: None }],
                        matcher: None,
                        overrides: HashMap::new(),
                    },
                );
                m
            },
        };
        let path = tmp.path().join(".hooker");
        std::fs::write(&path, serde_json::to_string(&config).unwrap()).unwrap();

        let loaded = loader::load_config(Some(&path), None).unwrap();
        assert_eq!(loaded.events.len(), 1);
    }

    #[test]
    fn resolver_integration() {
        let binding = EventBinding {
            enabled: true,
            actions: vec![EventAction::Speak {
                message: "default".to_string(),
            }],
            matcher: None,
            overrides: {
                let mut m = HashMap::new();
                m.insert(
                    Provider::Claude,
                    ProviderOverride {
                        enabled: true,
                        actions: vec![EventAction::Speak {
                            message: "claude override".to_string(),
                        }],
                        matcher: Some("Bash".to_string()),
                    },
                );
                m
            },
        };

        let (enabled, actions, matcher) =
            resolver::resolve_actions(&binding, Provider::Claude);
        assert!(enabled);
        assert_eq!(actions.len(), 1);
        assert_eq!(matcher, Some("Bash"));
    }

    #[test]
    fn matcher_with_override_pattern() {
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
