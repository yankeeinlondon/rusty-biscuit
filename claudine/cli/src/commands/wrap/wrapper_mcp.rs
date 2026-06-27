//! MCP session composition for the direct provider wrapper.
//!
//! Lifted verbatim from `run_provider_wrapper_inner` so the wrapper
//! orchestrator stays readable. Resolves the effective MCP server set from
//! `--mcp` / `--use` plus prompt `#tags`, injects the servers into the
//! child env/argv, and returns the runtime info plus an injector-cleanup
//! handle the caller drops after the child exits.

use std::io::IsTerminal;

use color_eyre::eyre::{Result, eyre};

use claudine::provider::Provider;

use super::flags::WrapperArgs;
use super::{McpRuntimeInfo, bootstrap_mcp_state, env, inline, profile};

/// Compose the MCP session for a direct wrapper run.
///
/// Returns the `(runtime_info, injector_cleanup)` pair. The cleanup handle,
/// when `Some`, owns the injector and its [`InjectionResult`] so the caller
/// can run `injector.cleanup(...)` after the child process exits. Mutates
/// `env_plan`, `child_args`, and `prompt_source` in place (server env vars,
/// extra argv, and `#tag`-stripped prompt) and appends any warnings/messages
/// to the deferred buffers.
#[allow(clippy::type_complexity)]
// The eight parameters mirror the caller-owned structures this function
// mutates in place; grouping them would only add a context struct that
// every call site has to construct and destructure.
#[allow(clippy::too_many_arguments)]
pub(crate) fn compose_mcp_session(
    args: &WrapperArgs,
    provider: Provider,
    non_interactive_requested: bool,
    env_plan: &mut env::EnvPlan,
    child_args: &mut Vec<String>,
    prompt_source: &mut profile::PromptSource,
    deferred_warnings: &mut Vec<String>,
    deferred_messages: &mut Vec<String>,
) -> Result<(
    Option<McpRuntimeInfo>,
    Option<(
        Box<dyn claudine::mcp::inject::McpInjector>,
        claudine::mcp::inject::InjectionResult,
    )>,
)> {
    let mut mcp_runtime = None;
    let mut mcp_cleanup: Option<(
        Box<dyn claudine::mcp::inject::McpInjector>,
        claudine::mcp::inject::InjectionResult,
    )> = None;

    // MCP session composition
    if args.mcp || !args.mcp_use.is_empty() {
        use claudine::mcp::catalog::McpCatalogStore;
        use claudine::mcp::inject::injector_for_provider;
        use claudine::mcp::session::{compute_session_set, lex_tags};

        let repo_root_ref = env_plan.repo_root.as_deref();
        if bootstrap_mcp_state(repo_root_ref)? {
            deferred_messages.push(
                "MCP bootstrap: created Claudine MCP state from discoverable provider configs."
                    .to_string(),
            );
        }
        let catalog =
            McpCatalogStore::load().map_err(|e| eyre!("failed to load MCP catalog: {e}"))?;
        let (cleaned_prompt, prompt_tags) =
            inline::extract_tags_from_prompt(prompt_source.as_inline(), lex_tags);
        if let Some(ref cleaned) = cleaned_prompt {
            *prompt_source = profile::PromptSource::Inline(cleaned.clone());
        }
        let prompt_is_interactive =
            std::io::stdin().is_terminal() && std::io::stdout().is_terminal();
        let mut session = compute_session_set(
            &catalog,
            repo_root_ref,
            &args.mcp_use,
            &prompt_tags,
            |tag, _tier, candidates| {
                if args.strict || non_interactive_requested || !prompt_is_interactive {
                    return None;
                }
                inquire::Select::new(
                    &format!("`#{tag}` matched multiple MCP servers. Choose one:"),
                    candidates.to_vec(),
                )
                .prompt()
                .ok()
            },
        )
        .map_err(|e| eyre!("MCP session error: {e}"))?;
        session.cleaned_prompt = cleaned_prompt.clone();

        for warning in &session.warnings {
            deferred_warnings.push(warning.clone());
        }
        if !session.missing_tags.is_empty() {
            if args.strict {
                return Err(eyre!(
                    "unresolved MCP tag(s): {}",
                    session
                        .missing_tags
                        .iter()
                        .map(|tag| format!("#{tag}"))
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
            for tag in &session.missing_tags {
                deferred_warnings.push(format!("tag `#{tag}` was not found in the MCP catalog"));
            }
        }
        if !session.ambiguous_tags.is_empty() {
            if args.strict || non_interactive_requested {
                let message = session
                    .ambiguous_tags
                    .iter()
                    .map(|tag| format!("#{} -> {}", tag.tag, tag.candidates.join(", ")))
                    .collect::<Vec<_>>()
                    .join("; ");
                return Err(eyre!("ambiguous MCP tag(s): {message}"));
            }
            // Interactive non-strict: warn and drop ambiguous tags
            for tag in &session.ambiguous_tags {
                deferred_warnings.push(format!(
                    "tag `#{0}` is ambiguous ({1}); dropped from session",
                    tag.tag,
                    tag.candidates.join(", ")
                ));
            }
            session.ambiguous_tags.clear();
        }

        let mut runtime = McpRuntimeInfo {
            servers: session
                .servers
                .iter()
                .map(|server| server.id.clone())
                .collect(),
            default_servers: session.default_servers.clone(),
            explicit_servers: session.explicit_servers.clone(),
            tag_servers: session.tag_servers.clone(),
            resolved_tags: session
                .resolved_tags
                .iter()
                .map(|tag| format!("#{} -> {} ({:?})", tag.tag, tag.resolved_to, tag.match_tier))
                .collect(),
            missing_tags: session
                .missing_tags
                .iter()
                .map(|tag| format!("#{tag}"))
                .collect(),
            ambiguous_tags: session
                .ambiguous_tags
                .iter()
                .map(|tag| format!("#{} -> {}", tag.tag, tag.candidates.join(", ")))
                .collect(),
            cleaned_prompt: session.cleaned_prompt.clone(),
            ..McpRuntimeInfo::default()
        };

        if let Some(injector) = injector_for_provider(provider) {
            if !session.servers.is_empty() {
                let shadow = env_plan.shadow_home_path.as_deref();
                // Injector works with String env; bridge to OsString env plan
                let mut string_env = std::collections::HashMap::new();
                let result = injector
                    .inject(&session.servers, &mut string_env, shadow)
                    .map_err(|e| eyre!("MCP injection failed: {e}"))?;

                // Merge injected env vars into the OsString env plan. The
                // OpenCode inline config is shared with the system-prompt and
                // YOLO producers, so it must merge into any value already on the
                // plan rather than overwrite it; every other key is a plain set.
                for (k, v) in string_env {
                    if k == "OPENCODE_CONFIG_CONTENT" {
                        let current = env_plan
                            .env
                            .get(std::ffi::OsStr::new("OPENCODE_CONFIG_CONTENT"))
                            .and_then(|os| os.to_str());
                        let overlay = serde_json::from_str(&v).map_err(|e| {
                            eyre!("MCP injection produced invalid OPENCODE_CONFIG_CONTENT: {e}")
                        })?;
                        let merged = claudine::opencode_config::merge_overlay(current, overlay)
                            .map_err(|e| eyre!("failed to merge OPENCODE_CONFIG_CONTENT: {e}"))?;
                        env_plan.env.insert(k.into(), merged.into());
                    } else {
                        env_plan.env.insert(k.into(), v.into());
                    }
                }

                for arg in &result.extra_args {
                    child_args.push(arg.clone());
                }

                runtime.env_vars_set = result.env_vars_set.clone();
                runtime.temp_files = result.temp_files.clone();
                runtime.extra_args = result.extra_args.clone();

                mcp_cleanup = Some((injector, result));
            }
        } else {
            return Err(eyre!(
                "provider {} does not support runtime MCP injection.\n\
                 Use `claudine mcp export {} --apply` to write servers to its native config instead.",
                provider,
                provider.as_slug()
            ));
        }

        deferred_messages.push(if runtime.servers.is_empty() {
            "MCP: no active servers".to_string()
        } else {
            format!("MCP: {}", runtime.servers.join(", "))
        });
        if !runtime.resolved_tags.is_empty() {
            deferred_messages.push(format!("MCP tags: {}", runtime.resolved_tags.join(", ")));
        }
        mcp_runtime = Some(runtime);
    }

    Ok((mcp_runtime, mcp_cleanup))
}
