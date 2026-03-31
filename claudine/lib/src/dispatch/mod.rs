pub mod loader;
mod matcher;
mod runner;
pub mod template;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde_json::Value;
use tracing::{debug, info};

use crate::actions::HookResponse;
use crate::adapters::{self, AdapterError};
use crate::error::{ClaudineError, Result};
use crate::events::{AgenticEvent, EnvironmentContext, EventMeta, Provider, ResolvedHook};
use crate::permissions::{PolicyContext, PolicyEngine};
use crate::services::{
    ProtectCliContext, ProtectDecision, ProtectOutcome, ProtectService, ProtectSessionContext,
    ProviderProtectProfiles,
};

/// Result of dispatching a single incoming provider event.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DispatchOutcome {
    /// Provider-native response payload for blocking hooks.
    pub response: Option<Value>,
    /// Optional process exit code for shell-based providers.
    pub exit_code: Option<i32>,
    /// Optional protect evaluation before actions.
    pub protect_pre: Option<ProtectDecision>,
    /// Optional protect evaluation after actions.
    pub protect_post: Option<ProtectDecision>,
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

    prepare_meta_for_dispatch(&mut meta, env);

    dispatch_preparsed(provider, event, meta).await
}

/// Dispatch a normalized event that has already been mapped into Claudine's
/// shared event vocabulary.
///
/// This is used by wrapper-managed structured streams, which already parse
/// provider output into coarse lifecycle events and therefore do not need to
/// round-trip back through a provider adapter.
pub async fn dispatch_event_meta(
    provider: Provider,
    event: AgenticEvent,
    mut meta: EventMeta,
) -> Result<DispatchOutcome> {
    meta.provider = provider;
    meta.event = event;
    let env = meta.env.clone();
    prepare_meta_for_dispatch(&mut meta, &env);
    dispatch_preparsed(provider, event, meta).await
}

fn prepare_meta_for_dispatch(meta: &mut EventMeta, env: &EnvironmentContext) {
    meta.env = env.clone();

    // If the wrapper injected a session ID and the adapter didn't extract one
    // from the payload, use the wrapper's session ID for consistent grouping.
    if meta.session_id.is_none()
        && let Ok(wrapper_sid) = std::env::var("CLAUDINE_SESSION_ID")
        && !wrapper_sid.trim().is_empty()
    {
        meta.session_id = Some(wrapper_sid);
    }

    // Propagate wrapper interactivity flag into extra for reporting.
    if let Some(interactive) = wrapper_interactive_flag() {
        meta.extra
            .entry("interactive".to_string())
            .or_insert_with(|| Value::String(interactive));
    }
    if let Some(yolo) = wrapper_yolo_flag() {
        meta.extra
            .entry("yolo".to_string())
            .or_insert_with(|| Value::String(yolo));
    }
}

async fn dispatch_preparsed(
    provider: Provider,
    event: AgenticEvent,
    meta: EventMeta,
) -> Result<DispatchOutcome> {
    let adapter = adapters::adapter_for(provider);

    info!(%provider, %event, "Dispatching event");

    let config = match loader::load_runtime_config(None, runtime_repo_root(&meta.env)) {
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
        debug!(
            %event,
            %provider,
            "No actions configured; protect evaluation may still apply"
        );
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

    let engine = Arc::new(PolicyEngine::new());
    let mut protect_service = config.settings().protect.clone().map(|protect| {
        let mut profiles = ProviderProtectProfiles::defaults();
        profiles.insert(provider, adapter.protect_capabilities());
        ProtectService::with_profiles(engine.clone(), protect, profiles)
    });

    // Build session context
    let session_ctx = build_session_context(provider, &resolved_hook.meta);

    let protect_pre = if let Some(service) = protect_service.as_mut() {
        service
            .evaluate_event_structured(
                provider,
                resolved_hook.event,
                &resolved_hook.meta,
                &session_ctx,
                adapter,
            )
            .ok()
            .flatten()
    } else {
        None
    };

    let protect_pre_decision = protect_pre.as_ref().map(|e| e.decision.clone());

    if let Some(eval) = protect_pre.as_ref()
        && should_short_circuit_on_protect(&eval.decision.outcome)
    {
        let response = adapter
            .map_protect_outcome(&resolved_hook.event, &eval.decision)
            .map_err(|error| {
                ClaudineError::ProtectEnforcementMapping(format!(
                    "provider={provider} event={} pre-action: {error}",
                    resolved_hook.event
                ))
            })?;

        return finalize_response(
            adapter,
            &resolved_hook.event,
            resolved_hook.can_block,
            Some(response),
            protect_pre_decision,
            None,
        );
    }

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
        protect_pre_decision.as_ref(),
    )
    .await?;

    let protect_post = if let Some(service) = protect_service.as_mut() {
        // Only run post-action evaluation for relevant events
        if matches!(
            resolved_hook.event,
            AgenticEvent::AfterTool | AgenticEvent::TurnComplete | AgenticEvent::SubagentStop
        ) {
            service
                .evaluate_event_structured(
                    provider,
                    resolved_hook.event,
                    &resolved_hook.meta,
                    &session_ctx,
                    adapter,
                )
                .ok()
                .flatten()
        } else {
            None
        }
    } else {
        None
    };

    let protect_post_decision = protect_post.as_ref().map(|e| e.decision.clone());

    // Apply redaction from post-action evaluation
    let action_response = if let Some(eval) = protect_post.as_ref() {
        if let Some(plan) = &eval.redaction {
            apply_redaction(action_response, plan)
        } else if should_short_circuit_on_protect(&eval.decision.outcome) {
            Some(
                adapter
                    .map_protect_outcome(&resolved_hook.event, &eval.decision)
                    .map_err(|error| {
                        ClaudineError::ProtectEnforcementMapping(format!(
                            "provider={provider} event={} post-action: {error}",
                            resolved_hook.event
                        ))
                    })?,
            )
        } else {
            action_response
        }
    } else {
        action_response
    };

    finalize_response(
        adapter,
        &resolved_hook.event,
        resolved_hook.can_block,
        action_response,
        protect_pre_decision,
        protect_post_decision,
    )
}

fn build_session_context(provider: Provider, meta: &EventMeta) -> ProtectSessionContext {
    let cwd = meta
        .cwd
        .as_ref()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));

    let mut policy_ctx = PolicyContext::new(cwd);
    if let Some(ref git) = meta.env.git {
        policy_ctx = policy_ctx.with_repo_root(git.repo_root.clone());
    }
    if let Some(home) = dirs::home_dir() {
        policy_ctx = policy_ctx.with_home_dir(home);
    }

    let cli_ctx = std::env::var("AGENT_PARAMS")
        .ok()
        .map(|params| {
            let argv: Vec<String> = params
                .split_whitespace()
                .map(String::from)
                .collect();
            ProtectCliContext::Argv(argv)
        })
        .unwrap_or(ProtectCliContext::None);

    ProtectSessionContext {
        provider,
        policy_context: policy_ctx,
        cli: cli_ctx,
        interactive: std::env::var("INTERACTIVE")
            .ok()
            .map_or(false, |v| v == "1" || v == "true")
            || std::env::var("CLAUDINE_INTERACTIVE")
                .ok()
                .map_or(false, |v| v == "1" || v == "true"),
        yolo: std::env::var("YOLO")
            .ok()
            .map_or(false, |v| v == "1" || v == "true")
            || std::env::var("CLAUDINE_YOLO")
                .ok()
                .map_or(false, |v| v == "1" || v == "true"),
        session_id: meta
            .session_id
            .clone()
            .or_else(|| std::env::var("CLAUDINE_SESSION_ID").ok()),
    }
}

fn apply_redaction(
    response: Option<HookResponse>,
    plan: &crate::services::ProtectRedactionPlan,
) -> Option<HookResponse> {
    use crate::services::ProtectRedactionPlan;

    let mut response = response.unwrap_or_default();

    match plan {
        ProtectRedactionPlan::ReplaceText(redaction) => {
            response.additional_context = Some(redaction.text.clone());
        }
        ProtectRedactionPlan::ReplaceJson(redaction) => {
            response.updated_input = Some(redaction.value.clone());
        }
        ProtectRedactionPlan::BlockPayload { reason } => {
            response.additional_context = None;
            response.updated_input = None;
            response.reason = Some(reason.clone());
        }
    }

    Some(response)
}

fn runtime_repo_root(env: &EnvironmentContext) -> Option<&Path> {
    env.git
        .as_ref()
        .map(|git| git.repo_root.as_path())
        .or_else(|| env.repo.as_ref().map(|repo| repo.root.as_path()))
}

fn wrapper_interactive_flag() -> Option<String> {
    wrapper_interactive_flag_from(|key| std::env::var(key).ok())
}

fn wrapper_yolo_flag() -> Option<String> {
    wrapper_flag_from(&["YOLO", "CLAUDINE_YOLO"], |key| std::env::var(key).ok())
}

fn wrapper_interactive_flag_from<F>(lookup: F) -> Option<String>
where
    F: Fn(&str) -> Option<String>,
{
    wrapper_flag_from(&["INTERACTIVE", "CLAUDINE_INTERACTIVE"], lookup)
}

fn wrapper_flag_from<F>(keys: &[&str], lookup: F) -> Option<String>
where
    F: Fn(&str) -> Option<String>,
{
    keys.iter()
        .copied()
        .filter_map(lookup)
        .map(|value| value.trim().to_string())
        .find(|value| !value.is_empty())
}

fn finalize_response(
    adapter: &dyn adapters::ProviderAdapter,
    event: &crate::events::AgenticEvent,
    can_block: bool,
    response: Option<HookResponse>,
    protect_pre: Option<ProtectDecision>,
    protect_post: Option<ProtectDecision>,
) -> Result<DispatchOutcome> {
    let stop_session =
        has_stop_session(protect_pre.as_ref()) || has_stop_session(protect_post.as_ref());

    if !can_block {
        return Ok(DispatchOutcome {
            response: adapter.non_blocking_ack(),
            exit_code: stop_session.then_some(2),
            protect_pre,
            protect_post,
        });
    }

    let Some(response) = response else {
        return Ok(DispatchOutcome {
            response: None,
            exit_code: None,
            protect_pre,
            protect_post,
        });
    };

    let payload = adapter.format_response(event, &response)?;
    let exit_code = if stop_session {
        Some(2)
    } else {
        adapter.exit_code(event, &response)
    };

    let response_payload = if payload.is_null() {
        None
    } else {
        Some(payload)
    };

    Ok(DispatchOutcome {
        response: response_payload,
        exit_code,
        protect_pre,
        protect_post,
    })
}

fn should_short_circuit_on_protect(outcome: &ProtectOutcome) -> bool {
    matches!(
        outcome,
        ProtectOutcome::AskThenAllowOrStop { .. }
            | ProtectOutcome::StopCurrent { .. }
            | ProtectOutcome::StopSession { .. }
    )
}

fn has_stop_session(decision: Option<&ProtectDecision>) -> bool {
    decision.is_some_and(|decision| matches!(decision.outcome, ProtectOutcome::StopSession { .. }))
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
        // Claude adapter returns {} ack for non-blocking events
        assert_eq!(outcome.response, Some(Value::Object(Default::default())));
        assert_eq!(outcome.exit_code, None);
    }

    #[test]
    fn wrapper_interactive_flag_prefers_canonical_interactive_env() {
        let value = wrapper_interactive_flag_from(|key| match key {
            "INTERACTIVE" => Some("true".to_string()),
            "CLAUDINE_INTERACTIVE" => Some("false".to_string()),
            _ => None,
        });

        assert_eq!(value.as_deref(), Some("true"));
    }

    #[test]
    fn wrapper_interactive_flag_falls_back_to_legacy_claudine_env() {
        let value = wrapper_interactive_flag_from(|key| match key {
            "CLAUDINE_INTERACTIVE" => Some("false".to_string()),
            _ => None,
        });

        assert_eq!(value.as_deref(), Some("false"));
    }

    #[test]
    fn wrapper_yolo_flag_prefers_canonical_yolo_env() {
        let value = wrapper_flag_from(&["YOLO", "CLAUDINE_YOLO"], |key| match key {
            "YOLO" => Some("true".to_string()),
            "CLAUDINE_YOLO" => Some("false".to_string()),
            _ => None,
        });

        assert_eq!(value.as_deref(), Some("true"));
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
                protect: None,
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
        // Claude adapter returns {} ack for non-blocking events
        assert_eq!(outcome.response, Some(Value::Object(Default::default())));
        assert_eq!(outcome.exit_code, None);

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
