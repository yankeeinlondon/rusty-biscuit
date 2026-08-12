//! MCP session composition for the direct provider wrapper.
//!
//! Lifted verbatim from `run_provider_wrapper_inner` so the wrapper
//! orchestrator stays readable. Resolves the effective MCP server set from
//! `--mcp` / `--use` plus prompt `#tags`, injects the servers into the
//! child env/argv, and returns the runtime info plus an injector-cleanup
//! handle the caller drops after the child exits.

use std::io::IsTerminal;

use color_eyre::eyre::{Result, WrapErr, eyre};

use claudine::provider::Provider;

use super::flags::WrapperArgs;
use super::{McpRuntimeInfo, bootstrap_mcp_state, env, inline, profile};

/// Fold the injector's String env vars into the child `EnvPlan`.
///
/// `OPENCODE_CONFIG_CONTENT` is the one inline config shared by the MCP,
/// system-prompt, and YOLO producers, so its injected value must **merge** into
/// whatever is already on the plan (via [`claudine::opencode_config::merge_overlay`])
/// rather than overwrite it; every other key is a plain set. Used by both the
/// direct wrapper and the composition MCP folds so the merge semantics stay in
/// one place.
///
/// ## Errors
///
/// Returns an error when the injected `OPENCODE_CONFIG_CONTENT` is not valid
/// JSON, or when merging it into the existing plan value fails (including an
/// existing plan value that is present but not valid UTF-8 or not a JSON object).
/// Error messages name `OPENCODE_CONFIG_CONTENT` but never echo its raw value (it
/// may carry credentials).
pub(crate) fn merge_injected_env_into_plan(
    string_env: std::collections::HashMap<String, String>,
    env_plan: &mut env::EnvPlan,
) -> Result<()> {
    const KEY: &str = "OPENCODE_CONFIG_CONTENT";
    for (k, v) in string_env {
        if k == KEY {
            let current = env_plan
                .env
                .get(std::ffi::OsStr::new(KEY))
                .map(|v| v.as_os_str());
            let overlay = serde_json::from_str(&v)
                .wrap_err("MCP injection produced invalid OPENCODE_CONFIG_CONTENT")?;
            let merged = claudine::opencode_config::merge_overlay(current, overlay)
                .wrap_err("failed to merge OPENCODE_CONFIG_CONTENT")?;
            env_plan.env.insert(k.into(), merged.into());
        } else {
            env_plan.env.insert(k.into(), v.into());
        }
    }
    Ok(())
}

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
            McpCatalogStore::load().wrap_err("failed to load MCP catalog")?;
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
        .wrap_err("MCP session error")?;
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
                    .wrap_err("MCP injection failed")?;

                // Merge injected env vars into the OsString env plan. The
                // OpenCode inline config is shared with the system-prompt and
                // YOLO producers, so it must merge into any value already on the
                // plan rather than overwrite it; every other key is a plain set.
                merge_injected_env_into_plan(string_env, env_plan)?;

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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    use std::collections::HashMap;

    const KEY: &str = "OPENCODE_CONFIG_CONTENT";

    fn env_plan_with(current: Option<&str>) -> env::EnvPlan {
        let mut plan = env::EnvPlan::default();
        if let Some(raw) = current {
            plan.env
                .insert(std::ffi::OsString::from(KEY), std::ffi::OsString::from(raw));
        }
        plan
    }

    fn config_value(plan: &env::EnvPlan) -> Option<Value> {
        plan.env
            .get(std::ffi::OsStr::new(KEY))
            .and_then(|os| os.to_str())
            .map(|raw| serde_json::from_str(raw).expect("config is valid JSON"))
    }

    #[test]
    fn merge_injected_env_plain_keys_are_set() {
        let mut plan = env::EnvPlan::default();
        let injected =
            HashMap::from([("CODEX_HOME".to_string(), "/tmp/shadow".to_string())]);
        merge_injected_env_into_plan(injected, &mut plan).unwrap();
        assert_eq!(
            plan.env.get(std::ffi::OsStr::new("CODEX_HOME")).unwrap(),
            std::ffi::OsStr::new("/tmp/shadow")
        );
    }

    #[test]
    fn merge_injected_env_merges_opencode_config_into_existing_value() {
        // The system-prompt fold already wrote `instructions`; the MCP fold must
        // merge its `mcp` block in, not clobber the existing config.
        let existing = r#"{"instructions":["/tmp/sp.md"]}"#;
        let mut plan = env_plan_with(Some(existing));
        let injected = HashMap::from([(
            KEY.to_string(),
            r#"{"mcp":{"srv":{"command":"x"}}}"#.to_string(),
        )]);
        merge_injected_env_into_plan(injected, &mut plan).unwrap();

        let config = config_value(&plan).unwrap();
        assert_eq!(config["instructions"], serde_json::json!(["/tmp/sp.md"]));
        assert_eq!(config["mcp"], serde_json::json!({ "srv": { "command": "x" } }));
    }

    #[test]
    fn merge_injected_env_merges_user_supplied_object_value() {
        // A user-supplied OPENCODE_CONFIG_CONTENT object must be preserved, not
        // replaced, when the MCP fold injects its block.
        let user = r#"{"theme":"dark","mcp":{"keep":{}}}"#;
        let mut plan = env_plan_with(Some(user));
        let injected =
            HashMap::from([(KEY.to_string(), r#"{"mcp":{"srv":{}}}"#.to_string())]);
        merge_injected_env_into_plan(injected, &mut plan).unwrap();

        let config = config_value(&plan).unwrap();
        assert_eq!(config["theme"], "dark");
        assert_eq!(config["mcp"]["keep"], serde_json::json!({}));
        assert_eq!(config["mcp"]["srv"], serde_json::json!({}));
    }

    #[test]
    fn merge_injected_env_rejects_invalid_injected_config_without_echoing_value() {
        let mut plan = env::EnvPlan::default();
        let injected =
            HashMap::from([(KEY.to_string(), "{not json secret=hunter2".to_string())]);
        let err = merge_injected_env_into_plan(injected, &mut plan).unwrap_err();
        let message = err.to_string();
        assert!(message.contains("OPENCODE_CONFIG_CONTENT"));
        assert!(!message.contains("hunter2"));
    }

    #[cfg(unix)]
    #[test]
    fn merge_injected_env_rejects_existing_non_utf8_plan_value() {
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStringExt;

        // An existing plan value that is present but not valid UTF-8 must be
        // rejected, never silently clobbered by the injected block (0x80 is an
        // invalid UTF-8 continuation byte).
        let mut plan = env::EnvPlan::default();
        plan.env
            .insert(OsString::from(KEY), OsString::from_vec(vec![0x66, 0x80]));
        let injected = HashMap::from([(KEY.to_string(), r#"{"mcp":{"srv":{}}}"#.to_string())]);
        let err = merge_injected_env_into_plan(injected, &mut plan).unwrap_err();
        assert!(err.to_string().contains("OPENCODE_CONFIG_CONTENT"));

        // The UTF-8 diagnosis lives on the typed cause rather than being
        // pre-flattened into the context line, so assert it through the chain —
        // which is what the renderer and `err.*` both read.
        let chain: Vec<String> = err.chain().map(|c| c.to_string()).collect();
        assert!(
            chain.iter().any(|c| c.contains("UTF-8")),
            "UTF-8 diagnosis lost from the cause chain: {chain:?}"
        );
    }
}
