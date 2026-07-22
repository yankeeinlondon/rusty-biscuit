pub(crate) mod deps;
pub mod expression;
pub mod loader;
pub mod matcher;
pub mod runner;
pub mod template;
mod logging;
mod protect_bridge;
mod wrapper_flags;

use std::sync::Arc;

use serde_json::Value;
use tracing::{debug, info, info_span};

use crate::actions::HookResponse;
use crate::hook_adapters::{self, AdapterError};
use crate::error::Result;
use crate::events::{AgenticEvent, EnvironmentContext, EventMeta, ResolvedHook};
use crate::protect::decision::ProtectDecision;
use crate::protect::observe::{extract_protect_request, ProtectObservation};
use crate::provider::Provider;

pub use logging::{log_dispatch_event, write_dispatch_event_to};
use logging::{prepare_meta_for_dispatch, tool_detail_for_log};
use protect_bridge::{evaluate_protect_observation, map_protect_block};
use wrapper_flags::runtime_repo_root;
#[cfg(test)]
use wrapper_flags::{wrapper_flag_from, wrapper_interactive_flag_from};

/// Wrapper-session-scoped dispatch runtime.
///
/// Holds the compiled runtime configuration for repeated dispatches within
/// a single wrapper process. `None` means no Claudine config was found.
#[derive(Debug, Clone, Default)]
pub struct DispatchRuntimeContext {
    canonical_config: Option<Arc<loader::CanonicalRuntimeConfig>>,
}

impl DispatchRuntimeContext {
    /// Load and compile the runtime config once for a specific environment.
    pub fn load_for_env(env: &EnvironmentContext) -> Result<Self> {
        let repo_root = runtime_repo_root(env);

        match loader::load_claudine_config(None, repo_root) {
            Ok(config) => {
                let runtime = loader::compile_canonical_runtime(config, repo_root)?;
                Ok(Self {
                    canonical_config: Some(Arc::new(runtime)),
                })
            }
            Err(crate::error::ClaudineError::ConfigNotFound(_)) => Ok(Self::default()),
            Err(error) => Err(error),
        }
    }

    /// Load and compile the canonical runtime config for a specific environment.
    ///
    /// Deprecated: use [`load_for_env`](Self::load_for_env) instead. This
    /// method is an identical alias kept for API compatibility.
    #[deprecated(note = "use load_for_env instead")]
    pub fn load_canonical_for_env(env: &EnvironmentContext) -> Result<Self> {
        let repo_root = runtime_repo_root(env);
        match loader::load_claudine_config(None, repo_root) {
            Ok(config) => {
                let runtime = loader::compile_canonical_runtime(config, repo_root)?;
                Ok(Self {
                    canonical_config: Some(Arc::new(runtime)),
                })
            }
            Err(crate::error::ClaudineError::ConfigNotFound(_)) => Ok(Self::default()),
            Err(e) => Err(e),
        }
    }

    pub fn has_config(&self) -> bool {
        self.canonical_config.is_some()
    }

    pub fn canonical_config(&self) -> Option<&loader::CanonicalRuntimeConfig> {
        self.canonical_config.as_deref()
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
    let _span = info_span!("dispatch_event", %provider).entered();

    let adapter = hook_adapters::adapter_for(provider);

    let (event, mut meta) = match adapter.parse_event(raw) {
        Ok(parsed) => parsed,
        Err(AdapterError::UnknownEvent(reason)) => {
            debug!(%provider, %reason, "adapter returned unknown event, skipping dispatch");
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

    if let Some(canonical) = runtime.canonical_config.as_deref() {
        return dispatch_canonical_with_runtime(provider, event, meta, canonical).await;
    }

    Ok(DispatchOutcome::default())
}

/// High-level canonical dispatch entry point.
///
/// Parses raw provider JSON, loads the new
/// [`ClaudineConfig`](crate::config::ClaudineConfig), compiles the
/// canonical runtime, and delegates to [`dispatch_canonical_with_runtime`].
pub async fn dispatch_canonical(
    raw: &Value,
    provider: Provider,
    env: &EnvironmentContext,
) -> Result<DispatchOutcome> {
    let adapter = hook_adapters::adapter_for(provider);

    let (event, mut meta) = {
        let _span = info_span!("dispatch_adapter_parse", %provider).entered();
        match adapter.parse_event(raw) {
            Ok(parsed) => parsed,
            Err(AdapterError::UnknownEvent(_)) => {
                debug!(%provider, "Adapter returned unknown event, skipping canonical dispatch");
                return Ok(DispatchOutcome::default());
            }
            Err(error) => {
                let _fail_span = info_span!(
                    "dispatch_adapter_parse_failed",
                    %provider,
                    error = %error,
                )
                .entered();
                return Err(error.into());
            }
        }
    };

    prepare_meta_for_dispatch(&mut meta, env);

    let repo_root = runtime_repo_root(env);

    let config = {
        let _span = info_span!("dispatch_load_config").entered();
        match loader::load_claudine_config(None, repo_root) {
            Ok(config) => config,
            Err(crate::error::ClaudineError::ConfigNotFound(_)) => {
                debug!("No .claudine config found, skipping canonical dispatch");
                return Ok(DispatchOutcome::default());
            }
            Err(error) => return Err(error),
        }
    };

    let runtime = {
        let _span = info_span!("dispatch_compile_runtime").entered();
        loader::compile_canonical_runtime(config, repo_root)?
    };
    dispatch_canonical_with_runtime(provider, event, meta, &runtime).await
}

/// Core canonical dispatch logic using the flat event→actions config.
///
/// Looks up bindings by canonical event only (not provider+event).
pub async fn dispatch_canonical_with_runtime(
    provider: Provider,
    event: AgenticEvent,
    meta: EventMeta,
    runtime: &loader::CanonicalRuntimeConfig,
) -> Result<DispatchOutcome> {
    let adapter = hook_adapters::adapter_for(provider);
    let can_block = adapter.can_block(&event);
    let repo_root_display = meta
        .env
        .git
        .as_ref()
        .map(|g| g.repo_root.display().to_string())
        .or_else(|| meta.env.repo.as_ref().map(|r| r.root.display().to_string()))
        .unwrap_or_default();
    let session_id = meta.session_id.clone().unwrap_or_default();
    let tool_name = meta.tool_name.clone().unwrap_or_default();
    let tool_detail = tool_detail_for_log(event, &meta);
    let _dispatch_span = info_span!(
        "dispatch_canonical_event",
        provider = %provider,
        event = %event,
        session_id = %session_id,
        tool_name = %tool_name,
        can_block,
        repo_root = %repo_root_display,
    )
    .entered();

    info!(
        %provider,
        %event,
        tool_name = %tool_name,
        tool_detail = tool_detail.as_deref().unwrap_or(""),
        "Dispatching canonical event"
    );

    // --- Protect pre-evaluation ---
    let protect_service = runtime.protect_service();
    let protect_pre = {
        let _span = info_span!("dispatch_protect_pre").entered();
        protect_service.and_then(|service| {
            evaluate_protect_observation(
                service,
                extract_protect_request(&event, &meta),
                meta.tool_name.as_deref().unwrap_or(""),
            )
        })
    };

    if let Some(ref decision) = protect_pre {
        let response = map_protect_block(decision);
        return finalize_response(
            adapter,
            &event,
            can_block,
            Some(response),
            protect_pre.clone(),
            None,
        );
    }

    // --- Binding lookup by canonical event only ---
    let binding = runtime.get_binding(&event);

    // --- Execute actions if binding exists and is valid ---
    let action_response = if let Some(binding) = binding {
        if !binding.enabled() {
            debug!(%event, "Canonical binding disabled, skipping actions");
            None
        } else if !matcher::matches(binding.matcher(), &meta) {
            debug!(%event, "Matcher did not match in canonical binding, skipping actions");
            None
        } else {
            if binding.actions().is_empty() {
                debug!(
                    %event,
                    "No actions configured in canonical binding; protect evaluation may still apply"
                );
            }

            let resolved_hook = ResolvedHook {
                event,
                meta: meta.clone(),
                provider,
                actions: binding.actions().to_vec(),
                can_block,
            };

            info!(
                event = %resolved_hook.event,
                provider = %resolved_hook.provider,
                tool_name = resolved_hook.meta.tool_name.as_deref().unwrap_or(""),
                tool_detail = tool_detail.as_deref().unwrap_or(""),
                action_count = resolved_hook.actions.len(),
                can_block = resolved_hook.can_block,
                "Executing resolved canonical hook"
            );

            let _span = info_span!(
                "dispatch_execute_actions",
                action_count = resolved_hook.actions.len(),
            )
            .entered();
            runner::execute_actions(
                &resolved_hook.actions,
                Some(binding.compiled_mappers()),
                &resolved_hook.meta,
                runner::DispatchConfig::Canonical(runtime.config()),
                runtime.messaging(),
                resolved_hook.can_block,
                protect_pre.as_ref(),
            )
            .await?
        }
    } else {
        debug!(%event, "No canonical binding found for event, skipping actions");
        None
    };

    // --- JSONL event logging (independent of binding) ---
    if runtime.config().logging {
        let _span = info_span!("dispatch_log_event").entered();
        log_dispatch_event(&meta);
    }

    // --- Protect post-evaluation (independent of binding) ---
    let protect_post = {
        let _span = info_span!("dispatch_protect_post").entered();
        protect_service.and_then(|service| {
            if !matches!(
                event,
                AgenticEvent::AfterTool | AgenticEvent::TurnComplete | AgenticEvent::SubagentStop
            ) {
                return None;
            }
            match extract_protect_request(&event, &meta) {
                ProtectObservation::Request(request) => {
                    let decision = service.evaluate(&request);
                    if decision.is_blocked() {
                        Some(decision)
                    } else {
                        None
                    }
                }
                _ => None,
            }
        })
    };

    let action_response = if let Some(ref decision) = protect_post {
        Some(map_protect_block(decision))
    } else {
        action_response
    };

    // --- Default sounds ---
    let was_blocked = protect_pre.is_some() || protect_post.is_some();
    {
        let _span = info_span!("dispatch_default_sound").entered();
        runner::play_default_sound_for_event(&event, runtime.config(), was_blocked);
    }

    finalize_response(
        adapter,
        &event,
        can_block,
        action_response,
        protect_pre,
        protect_post,
    )
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
    .in_scope(|| loader::load_claudine_config(None, repo_root_path))
    {
        Ok(config) => config,
        Err(crate::error::ClaudineError::ConfigNotFound(_)) => {
            debug!("No .claudine config found, skipping dispatch");
            return Ok(DispatchOutcome::default());
        }
        Err(error) => return Err(error),
    };

    let runtime = loader::compile_canonical_runtime(config, repo_root_path)?;
    dispatch_canonical_with_runtime(provider, event, meta, &runtime).await
}

fn finalize_response(
    adapter: &dyn hook_adapters::ProviderAdapter,
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
mod tests;
