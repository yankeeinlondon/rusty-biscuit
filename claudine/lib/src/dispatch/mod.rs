pub mod loader;
mod matcher;
mod runner;
pub mod template;

use std::path::Path;
use std::sync::Arc;

use serde_json::Value;
use tracing::{debug, info, info_span};

use crate::actions::{HookDecision, HookResponse};
use crate::adapters::{self, AdapterError};
use crate::error::Result;
use crate::events::{AgenticEvent, EnvironmentContext, EventMeta, Provider, ResolvedHook};
use crate::services::protect::decision::ProtectDecision;
use crate::services::protect::observe::extract_protect_request;
use crate::services::protect::report::format_blocked_message;

/// Wrapper-session-scoped dispatch runtime.
///
/// Holds the compiled runtime configuration for repeated dispatches within
/// a single wrapper process. `None` means no Claudine config was found.
#[derive(Debug, Clone, Default)]
pub struct DispatchRuntimeContext {
    config: Option<Arc<loader::RuntimeConfig>>,
}

impl DispatchRuntimeContext {
    /// Load and compile the runtime config once for a specific environment.
    pub fn load_for_env(env: &EnvironmentContext) -> Result<Self> {
        match loader::load_runtime_config(None, runtime_repo_root(env)) {
            Ok(config) => Ok(Self {
                config: Some(Arc::new(config)),
            }),
            Err(crate::error::ClaudineError::ConfigNotFound(_)) => Ok(Self { config: None }),
            Err(error) => Err(error),
        }
    }

    /// Build a cached runtime context from a preloaded runtime config.
    pub fn from_runtime_config(config: loader::RuntimeConfig) -> Self {
        Self {
            config: Some(Arc::new(config)),
        }
    }

    /// Return true when a compiled runtime config is available.
    pub fn has_config(&self) -> bool {
        self.config.is_some()
    }
}

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

/// Dispatch a normalized event using a cached wrapper-session runtime config.
///
/// This avoids reloading and recompiling Claudine config for every streamed
/// event within a single wrapper process.
pub async fn dispatch_event_meta_with_runtime(
    provider: Provider,
    event: AgenticEvent,
    mut meta: EventMeta,
    runtime: &DispatchRuntimeContext,
) -> Result<DispatchOutcome> {
    debug!(
        %provider,
        %event,
        has_cached_runtime = runtime.has_config(),
        "Dispatching event with wrapper-session runtime cache"
    );
    meta.provider = provider;
    meta.event = event;
    let env = meta.env.clone();
    prepare_meta_for_dispatch(&mut meta, &env);
    dispatch_preparsed_with_config(provider, event, meta, runtime.config.as_deref()).await
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
    let repo_root_path = runtime_repo_root(&meta.env);
    let repo_root = repo_root_path
        .map(|path| path.display().to_string())
        .unwrap_or_default();

    let config = match info_span!(
        "dispatch_config_load",
        provider = %provider,
        repo_root = %repo_root
    )
    .in_scope(|| loader::load_runtime_config(None, repo_root_path))
    {
        Ok(config) => config,
        Err(crate::error::ClaudineError::ConfigNotFound(_)) => {
            debug!("No .claudine config found, skipping dispatch");
            return Ok(DispatchOutcome::default());
        }
        Err(error) => return Err(error),
    };

    dispatch_preparsed_with_config(provider, event, meta, Some(&config)).await
}

fn tool_detail_for_log(event: AgenticEvent, meta: &EventMeta) -> Option<String> {
    match event {
        AgenticEvent::BeforeTool | AgenticEvent::PermissionRequest => meta
            .tool_input
            .as_ref()
            .map(|value| compact_value_for_log(value, 120)),
        AgenticEvent::AfterTool => {
            let mut parts = Vec::new();

            if let Some(tool_id) = meta.extra.get("tool_id").and_then(|value| value.as_str()) {
                parts.push(format!("id={tool_id}"));
            }
            if let Some(status) = meta.extra.get("status").and_then(|value| value.as_str()) {
                parts.push(format!("status={status}"));
            }
            let error = meta.error.clone().or_else(|| {
                meta.extra
                    .get("error")
                    .and_then(|value| compact_scalar_for_log(value, 80))
            });
            if let Some(error) = error {
                parts.push(format!("error={error}"));
            }
            if let Some(response) = meta.tool_response.as_ref() {
                parts.push(format!("result={}", compact_value_for_log(response, 120)));
            }

            (!parts.is_empty()).then(|| parts.join(" "))
        }
        AgenticEvent::ToolError => meta
            .error
            .as_deref()
            .map(|error| format!("error={}", truncate_for_log(error, 80))),
        _ => None,
    }
}

fn compact_value_for_log(value: &Value, max_chars: usize) -> String {
    let rendered = serde_json::to_string(value).unwrap_or_else(|_| value.to_string());
    truncate_for_log(&rendered, max_chars)
}

fn compact_scalar_for_log(value: &Value, max_chars: usize) -> Option<String> {
    match value {
        Value::String(text) => Some(truncate_for_log(text, max_chars)),
        Value::Null | Value::Bool(_) | Value::Number(_) => {
            Some(compact_value_for_log(value, max_chars))
        }
        _ => None,
    }
}

fn truncate_for_log(value: &str, max_chars: usize) -> String {
    let count = value.chars().count();
    if count <= max_chars {
        return value.to_string();
    }

    let truncated: String = value.chars().take(max_chars.saturating_sub(3)).collect();
    format!("{truncated}...")
}

async fn dispatch_preparsed_with_config(
    provider: Provider,
    event: AgenticEvent,
    meta: EventMeta,
    config: Option<&loader::RuntimeConfig>,
) -> Result<DispatchOutcome> {
    let adapter = adapters::adapter_for(provider);
    let can_block = adapter.can_block(&event);
    let repo_root = runtime_repo_root(&meta.env)
        .map(|path| path.display().to_string())
        .unwrap_or_default();
    let session_id = meta.session_id.clone().unwrap_or_default();
    let tool_name = meta.tool_name.clone().unwrap_or_default();
    let tool_detail = tool_detail_for_log(event, &meta);
    let _dispatch_span = info_span!(
        "dispatch_event",
        provider = %provider,
        event = %event,
        session_id = %session_id,
        tool_name = %tool_name,
        can_block,
        repo_root = %repo_root,
    )
    .entered();

    info!(
        %provider,
        %event,
        tool_name = %tool_name,
        tool_detail = tool_detail.as_deref().unwrap_or(""),
        "Dispatching event"
    );

    let Some(config) = config else {
        debug!("No cached .claudine config found, skipping dispatch");
        return Ok(DispatchOutcome::default());
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
        can_block,
    };

    let protect_service = config.protect_service();

    let protect_pre = protect_service.and_then(|service| {
        let request = extract_protect_request(&resolved_hook.event, &resolved_hook.meta)?;
        let decision = service.evaluate(&request);
        if decision.is_blocked() {
            Some(decision)
        } else {
            None
        }
    });

    if let Some(ref decision) = protect_pre {
        let response = map_protect_block(decision);
        return finalize_response(
            adapter,
            &resolved_hook.event,
            resolved_hook.can_block,
            Some(response),
            protect_pre.clone(),
            None,
        );
    }

    info!(
        event = %resolved_hook.event,
        provider = %resolved_hook.provider,
        tool_name = resolved_hook.meta.tool_name.as_deref().unwrap_or(""),
        tool_detail = tool_detail.as_deref().unwrap_or(""),
        action_count = resolved_hook.actions.len(),
        can_block = resolved_hook.can_block,
        "Executing resolved hook"
    );

    let action_response = runner::execute_actions(
        &resolved_hook.actions,
        Some(binding.compiled_mappers()),
        &resolved_hook.meta,
        config.settings(),
        config.messaging(),
        resolved_hook.can_block,
        protect_pre.as_ref(),
    )
    .await?;

    let protect_post = protect_service.and_then(|service| {
        if !matches!(
            resolved_hook.event,
            AgenticEvent::AfterTool | AgenticEvent::TurnComplete | AgenticEvent::SubagentStop
        ) {
            return None;
        }
        let request = extract_protect_request(&resolved_hook.event, &resolved_hook.meta)?;
        let decision = service.evaluate(&request);
        if decision.is_blocked() {
            Some(decision)
        } else {
            None
        }
    });

    let action_response = if let Some(ref decision) = protect_post {
        Some(map_protect_block(decision))
    } else {
        action_response
    };

    finalize_response(
        adapter,
        &resolved_hook.event,
        resolved_hook.can_block,
        action_response,
        protect_pre,
        protect_post,
    )
}

fn map_protect_block(decision: &ProtectDecision) -> HookResponse {
    let reason = decision
        .blocked
        .as_ref()
        .map(format_blocked_message)
        .unwrap_or_else(|| "protect: blocked".to_string());

    HookResponse {
        decision: Some(HookDecision::Deny),
        reason: Some(reason),
        updated_input: None,
        additional_context: None,
        raw: Some(serde_json::json!({
            "protect": {
                "outcome": "block",
                "group": decision.blocked.as_ref().map(|m| m.group.config_key()),
                "rule_id": decision.blocked.as_ref().map(|m| &m.rule_id),
            }
        })),
    }
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
    if !can_block {
        return Ok(DispatchOutcome {
            response: adapter.non_blocking_ack(),
            exit_code: None,
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
    let exit_code = adapter.exit_code(event, &response);

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
        // Exercises the "no runtime config" branch in `dispatch_preparsed_with_config`
        // without depending on the absence of a user-level `~/.claudine/config.json`
        // on the machine running the test.
        let raw = json!({
            "hook_event_name": "SessionStart",
            "session_id": "test-123"
        });
        let adapter = adapters::adapter_for(Provider::Claude);
        let (event, mut meta) = adapter.parse_event(&raw).unwrap();
        meta.env = EnvironmentContext::default();

        let outcome = dispatch_preparsed_with_config(Provider::Claude, event, meta, None)
            .await
            .unwrap();
        assert_eq!(outcome, DispatchOutcome::default());
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

    #[test]
    fn tool_detail_for_log_formats_before_tool_input() {
        let mut meta = EventMeta::new(Provider::Codex, AgenticEvent::BeforeTool);
        meta.tool_name = Some("shell".into());
        meta.tool_input = Some(json!({"cmd": "git status"}));

        assert_eq!(
            tool_detail_for_log(AgenticEvent::BeforeTool, &meta).as_deref(),
            Some(r#"{"cmd":"git status"}"#)
        );
    }

    #[test]
    fn tool_detail_for_log_formats_after_tool_metadata() {
        let mut meta = EventMeta::new(Provider::Gemini, AgenticEvent::AfterTool);
        meta.tool_name = Some("search".into());
        meta.tool_response = Some(json!({"hits": 3}));
        meta.extra.insert("tool_id".into(), json!("tool-1"));
        meta.extra.insert("status".into(), json!("success"));

        assert_eq!(
            tool_detail_for_log(AgenticEvent::AfterTool, &meta).as_deref(),
            Some(r#"id=tool-1 status=success result={"hits":3}"#)
        );
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
                messaging: None,
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
                messaging: None,
            },
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

    #[tokio::test]
    async fn cached_runtime_context_reuses_loaded_config_after_file_removal() {
        let repo = tempfile::tempdir().unwrap();
        let log_path = repo.path().join("cached-runtime-events.jsonl");

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
                messaging: None,
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

        let runtime = DispatchRuntimeContext::load_for_env(&env).unwrap();
        assert!(runtime.has_config());

        std::fs::remove_file(&config_path).unwrap();

        let first = EventMeta {
            provider: Provider::Claude,
            event: AgenticEvent::SessionStart,
            timestamp: chrono::Utc::now(),
            session_id: Some("cached-1".to_string()),
            cwd: None,
            tool_name: None,
            tool_input: None,
            tool_response: None,
            error: None,
            prompt: None,
            agent_type: None,
            notification_type: None,
            notification_message: None,
            extra: HashMap::new(),
            env: env.clone(),
        };
        let second = EventMeta {
            session_id: Some("cached-2".to_string()),
            ..first.clone()
        };

        let first_outcome = dispatch_event_meta_with_runtime(
            Provider::Claude,
            AgenticEvent::SessionStart,
            first,
            &runtime,
        )
        .await
        .unwrap();
        let second_outcome = dispatch_event_meta_with_runtime(
            Provider::Claude,
            AgenticEvent::SessionStart,
            second,
            &runtime,
        )
        .await
        .unwrap();

        assert_eq!(
            first_outcome.response,
            Some(Value::Object(Default::default()))
        );
        assert_eq!(
            second_outcome.response,
            Some(Value::Object(Default::default()))
        );

        let content = std::fs::read_to_string(log_path).unwrap();
        assert!(content.contains("cached-1"));
        assert!(content.contains("cached-2"));
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
