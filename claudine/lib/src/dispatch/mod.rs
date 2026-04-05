pub mod loader;
mod matcher;
mod runner;
pub mod template;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde_json::Value;
use tracing::{debug, info, info_span};

use crate::actions::HookResponse;
use crate::adapters::{self, AdapterError};
use crate::error::{ClaudineError, Result};
use crate::events::{AgenticEvent, EnvironmentContext, EventMeta, Provider, ResolvedHook};
use crate::permissions::{PolicyContext, PolicyEngine, ProjectTrustContext, TrustSource};
use crate::services::{
    ProtectCliContext, ProtectDecision, ProtectOutcome, ProtectService, ProtectSessionContext,
};

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

    let engine = Arc::new(PolicyEngine::new());
    let mut protect_service = config.settings().protect.clone().map(|protect| {
        ProtectService::with_capabilities(
            engine.clone(),
            protect,
            provider,
            adapter.protect_capabilities(),
        )
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

    // Post-action: blocking outcomes take priority over redaction.
    let action_response = if let Some(eval) = protect_post.as_ref() {
        if should_short_circuit_on_protect(&eval.decision.outcome) {
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
        } else if let Some(plan) = &eval.redaction {
            apply_redaction(action_response, plan)
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

    // Derive project trust from event metadata and wrapper state.
    let trust = derive_trust_context(meta);
    policy_ctx = policy_ctx.with_trust(trust);

    let cli_ctx = std::env::var("AGENT_PARAMS")
        .ok()
        .map(|params| {
            let argv: Vec<String> = params.split_whitespace().map(String::from).collect();
            ProtectCliContext::Argv(argv)
        })
        .unwrap_or(ProtectCliContext::None);

    ProtectSessionContext {
        provider,
        policy_context: policy_ctx,
        cli: cli_ctx,
        interactive: std::env::var("INTERACTIVE")
            .ok()
            .is_some_and(|v| v == "1" || v == "true")
            || std::env::var("CLAUDINE_INTERACTIVE")
                .ok()
                .is_some_and(|v| v == "1" || v == "true"),
        yolo: std::env::var("YOLO")
            .ok()
            .is_some_and(|v| v == "1" || v == "true")
            || std::env::var("CLAUDINE_YOLO")
                .ok()
                .is_some_and(|v| v == "1" || v == "true"),
        session_id: meta
            .session_id
            .clone()
            .or_else(|| std::env::var("CLAUDINE_SESSION_ID").ok()),
    }
}

/// Derive project trust from event metadata extras and environment.
///
/// Trust sources in priority order:
/// 1. Explicit `is_trusted` field in event extra (set by wrapper preflight)
/// 2. `CLAUDINE_TRUST` environment variable
/// 3. Provider permission mode (e.g., Claude's `acceptEdits` implies trusted)
fn derive_trust_context(meta: &EventMeta) -> ProjectTrustContext {
    // 1. Explicit trust flag from wrapper or event payload
    if let Some(trusted) = meta.extra.get("is_trusted").and_then(Value::as_bool) {
        return ProjectTrustContext {
            is_trusted: Some(trusted),
            source: TrustSource::ExplicitInput,
        };
    }

    // 2. CLAUDINE_TRUST environment variable
    if let Ok(val) = std::env::var("CLAUDINE_TRUST") {
        let lowered = val.trim().to_ascii_lowercase();
        if lowered == "1" || lowered == "true" {
            return ProjectTrustContext {
                is_trusted: Some(true),
                source: TrustSource::ExplicitInput,
            };
        } else if lowered == "0" || lowered == "false" {
            return ProjectTrustContext {
                is_trusted: Some(false),
                source: TrustSource::ExplicitInput,
            };
        }
    }

    // 3. Infer from provider permission mode (Claude-specific: acceptEdits implies trusted)
    if let Some(mode) = meta.extra.get("permission_mode").and_then(Value::as_str) {
        let lowered = mode.to_ascii_lowercase();
        if lowered.contains("accept") || lowered.contains("trust") {
            return ProjectTrustContext {
                is_trusted: Some(true),
                source: TrustSource::ProviderConfig,
            };
        }
    }

    ProjectTrustContext::default()
}

fn apply_redaction(
    response: Option<HookResponse>,
    plan: &crate::services::ProtectRedactionPlan,
) -> Option<HookResponse> {
    use crate::actions::HookDecision;
    use crate::services::ProtectRedactionPlan;

    let mut response = response.unwrap_or_default();

    match plan {
        ProtectRedactionPlan::ReplaceText(redaction) => {
            response.additional_context = Some(redaction.text.clone());
            // Preserve existing decision or set Allow so formatters know
            // there is an active response to serialize.
            if response.decision.is_none() {
                response.decision = Some(HookDecision::Allow);
            }
        }
        ProtectRedactionPlan::ReplaceJson(redaction) => {
            response.updated_input = Some(redaction.value.clone());
            if response.decision.is_none() {
                response.decision = Some(HookDecision::Allow);
            }
        }
        ProtectRedactionPlan::BlockPayload { reason } => {
            // BlockPayload must produce an enforceable deny.
            response.additional_context = None;
            response.updated_input = None;
            response.decision = Some(HookDecision::Deny);
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

    #[test]
    fn derive_trust_from_explicit_extra_field() {
        let mut meta = EventMeta {
            provider: Provider::Claude,
            event: AgenticEvent::BeforeTool,
            timestamp: chrono::Utc::now(),
            session_id: None,
            cwd: Some("/tmp".to_string()),
            tool_name: None,
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

        meta.extra.insert("is_trusted".to_string(), json!(true));

        let trust = derive_trust_context(&meta);
        assert_eq!(trust.is_trusted, Some(true));
        assert_eq!(trust.source, TrustSource::ExplicitInput);
    }

    #[test]
    fn derive_trust_from_permission_mode() {
        let mut meta = EventMeta {
            provider: Provider::Claude,
            event: AgenticEvent::BeforeTool,
            timestamp: chrono::Utc::now(),
            session_id: None,
            cwd: Some("/tmp".to_string()),
            tool_name: None,
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

        meta.extra
            .insert("permission_mode".to_string(), json!("acceptEdits"));

        let trust = derive_trust_context(&meta);
        assert_eq!(trust.is_trusted, Some(true));
        assert_eq!(trust.source, TrustSource::ProviderConfig);
    }

    #[test]
    fn derive_trust_unknown_without_signals() {
        let meta = EventMeta {
            provider: Provider::Claude,
            event: AgenticEvent::BeforeTool,
            timestamp: chrono::Utc::now(),
            session_id: None,
            cwd: Some("/tmp".to_string()),
            tool_name: None,
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

        let trust = derive_trust_context(&meta);
        assert_eq!(trust.is_trusted, None);
        assert_eq!(trust.source, TrustSource::Unknown);
    }

    #[test]
    fn apply_redaction_block_payload_sets_deny() {
        use crate::services::ProtectRedactionPlan;

        let plan = ProtectRedactionPlan::BlockPayload {
            reason: "blocked".to_string(),
        };

        let result = apply_redaction(None, &plan);
        assert!(result.is_some());
        let response = result.unwrap();
        assert_eq!(response.decision, Some(HookDecision::Deny));
        assert_eq!(response.reason.as_deref(), Some("blocked"));
        assert!(response.additional_context.is_none());
        assert!(response.updated_input.is_none());
    }

    #[test]
    fn apply_redaction_replace_text_sets_allow() {
        use crate::services::{McpTextRedaction, ProtectRedactionPlan};

        let plan = ProtectRedactionPlan::ReplaceText(McpTextRedaction {
            text: "redacted content".to_string(),
            redacted: true,
            blocked_instruction_payload: false,
            redactions_applied: 1,
        });

        let result = apply_redaction(None, &plan);
        assert!(result.is_some());
        let response = result.unwrap();
        assert_eq!(response.decision, Some(HookDecision::Allow));
        assert_eq!(
            response.additional_context.as_deref(),
            Some("redacted content")
        );
    }

    #[test]
    fn apply_redaction_replace_json_sets_allow() {
        use crate::services::{McpJsonRedaction, ProtectRedactionPlan};

        let plan = ProtectRedactionPlan::ReplaceJson(McpJsonRedaction {
            value: json!({"key": "[REDACTED]"}),
            redacted: true,
            blocked_instruction_payload: false,
            redactions_applied: 1,
        });

        let result = apply_redaction(None, &plan);
        assert!(result.is_some());
        let response = result.unwrap();
        assert_eq!(response.decision, Some(HookDecision::Allow));
        assert_eq!(response.updated_input, Some(json!({"key": "[REDACTED]"})));
    }

    #[test]
    fn build_session_context_includes_trust() {
        let mut meta = EventMeta {
            provider: Provider::Claude,
            event: AgenticEvent::BeforeTool,
            timestamp: chrono::Utc::now(),
            session_id: Some("s1".to_string()),
            cwd: Some("/tmp/project".to_string()),
            tool_name: None,
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

        meta.extra.insert("is_trusted".to_string(), json!(true));

        let ctx = build_session_context(Provider::Claude, &meta);
        assert_eq!(ctx.policy_context.trust.is_trusted, Some(true));
    }
}
