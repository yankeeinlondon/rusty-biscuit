//! Provider-neutral prep stages shared by the direct-wrapper and
//! composition execution pipelines.
//!
//! Each stage function takes `provider` (plus mode flags) and gates
//! internally — a no-op off the gate — following the pattern set by
//! `wrap::wrapper_stages::apply_opencode_yolo_config_overlay`. Both
//! pipelines call the same function so resolution semantics cannot
//! drift between them. Design authority:
//! `features/2026-07-02-provider-metadata/design/pipeline-dry.md`
//! (workstream 0).

use claudine::invocation_context::{EnvBaseline, HomeBaseline};
use claudine::provider::{Provider, provider_info};
use claudine::provider_overlay::{OverlayCapability, OverlayReason};
use color_eyre::eyre::Result;

use super::wrap::env::EnvPlan;
use super::wrap::policy::StructuredCodexOutput;
use super::wrap::profile::{ModelSource, WrapperProfile, no_model_error, resolve_model_source};
use super::wrap::provider_overlay;

/// Failure from [`resolve_model_and_validate`], split so each pipeline
/// keeps its own presentation.
#[derive(Debug)]
pub(crate) enum ModelStageError {
    /// Non-interactive launch of a provider whose catalog requires a model
    /// (`model_required_in_non_tty`) with no model from any source. The
    /// direct wrapper renders `AgentErrorReport::no_model_provided` and
    /// exits; composition propagates the message.
    NoModel(color_eyre::eyre::Error),
    /// Provider-trait non-interactive validation failure; both
    /// pipelines propagate it unchanged.
    Validation(color_eyre::eyre::Error),
}

impl ModelStageError {
    pub(crate) fn into_report(self) -> color_eyre::eyre::Error {
        match self {
            Self::NoModel(report) | Self::Validation(report) => report,
        }
    }
}

/// Resolve the session model onto the child argv/env and run the
/// provider's non-interactive validation. One path for every provider:
///
/// 1. An explicit `model` (the composition's resolved model, or the direct
///    wrapper's `--model`) goes through `profile.apply_model` — the universal
///    `--model` mapping plus the `MODEL` env export.
/// 2. With no explicit model, in non-interactive mode, and only when the
///    catalog says `model_required_in_non_tty`, Claudine finds one on the
///    provider's behalf: the catalog's `model_env_vars` in order, then
///    `profile.configured_default_model()`. Nothing found is
///    [`ModelStageError::NoModel`], unless the caller's env plan or the
///    passthrough argv already carries a model.
/// 3. `profile.validate_non_interactive_requirements` in non-interactive
///    mode.
///
/// `has_model_env` is whether the caller's env plan already carries a
/// `MODEL` entry; env writes go through `env_sink` (the wrapper pushes
/// onto `env_overrides`, composition inserts into `env_plan.env`) and
/// unsupported-model warnings through `warn_sink` (deferred vs. logged
/// immediately). The returned [`ModelSource`] feeds the wrapper's preflight
/// preamble and error attribution, and is reported only for a provider
/// that had to have a model (`must_find_model`): where the model is
/// optional, the launch says nothing about it, as before.
#[allow(clippy::too_many_arguments)]
pub(crate) fn resolve_model_and_validate(
    provider: Provider,
    profile: &dyn WrapperProfile,
    child_args: &mut Vec<String>,
    model: Option<&str>,
    non_interactive: bool,
    has_model_env: bool,
    // The launching document's captured environment, which the catalog's
    // provider model variables (`OPENCODE_MODEL`) are read from.
    environment: &claudine::invocation_context::DocumentEnvironment,
    env_sink: &mut dyn FnMut(String, String),
    warn_sink: &mut dyn FnMut(String),
) -> Result<Option<ModelSource>, ModelStageError> {
    let info = provider_info(provider);
    let must_find_model = non_interactive && info.model_required_in_non_tty;

    let source = match model {
        Some(model) => Some(ModelSource::CliSwitch(model.to_string())),
        None if must_find_model => resolve_model_source(
            info.model_env_vars,
            |var| environment.lookup(var),
            || profile.configured_default_model(),
        ),
        None => None,
    };

    match &source {
        // Every source is delivered explicitly, including a value read from the
        // provider's own configuration. Provider overlays and MCP modes can
        // redirect provider config, so rediscovery could select a different
        // model — or none — than the one Claudine reports.
        Some(resolved) => {
            let mut env_overrides = Vec::new();
            if let Some(warn) = profile.apply_model(child_args, &mut env_overrides, resolved.model())
            {
                warn_sink(warn);
            }
            for (key, value) in env_overrides {
                env_sink(key, value);
            }
        }
        None => {
            let has_model_arg = child_args
                .iter()
                .any(|arg| arg == "--model" || arg == "-m" || arg.starts_with("--model="));
            if must_find_model && !has_model_arg && !has_model_env {
                return Err(ModelStageError::NoModel(no_model_error(provider)));
            }
        }
    }

    if non_interactive {
        profile
            .validate_non_interactive_requirements(child_args)
            .map_err(ModelStageError::Validation)?;
    }

    Ok(source.filter(|_| must_find_model))
}

/// Guarantee a materialized provider overlay before provider-config injection
/// writes into it.
///
/// `env::build_child_env_with_launch` builds the overlay whenever the caller's
/// reason set asks for one, and a build failure already stopped the launch, so
/// on the direct path this is a no-op. It does work only when the launch was
/// planned without an `Mcp` reason — a composition session whose MCP servers
/// were resolved after the child environment was built. A failure here is a
/// typed pre-spawn error, like the builder's.
///
/// Also a no-op for a provider whose MCP delivery needs no Claudine-provided
/// config root: its overlay plan has no `Mcp` reason to satisfy.
pub(crate) fn ensure_provider_overlay(
    provider: Provider,
    needs_overlay: bool,
    env_plan: &mut EnvPlan,
    home_baseline: &HomeBaseline,
    env_baseline: &EnvBaseline,
) -> Result<()> {
    if !needs_overlay
        || env_plan.overlay_visible_root().is_some()
        || provider.overlay_capability(OverlayReason::Mcp) != OverlayCapability::NativeRoot
    {
        return Ok(());
    }
    let repo_resources = env_plan
        .overlay
        .as_ref()
        .is_some_and(|plan| plan.reasons().contains(OverlayReason::RepoResources));
    let reasons =
        provider_overlay::overlay_reasons(provider, repo_resources, true, &env_plan.child_cwd);
    let (plan, _) = provider_overlay::build_overlay(
        provider,
        reasons,
        env_plan.child_cwd.as_path(),
        false,
        Some(env_plan.child_cwd.as_path()),
        home_baseline,
        env_baseline,
    )?;
    provider_overlay::apply_overlay_env(&mut env_plan.env, &plan);
    env_plan.overlay = Some(plan);
    Ok(())
}

/// Attach Codex `--output-last-message` capture when Claudine must
/// recover the final assistant message from a file.
///
/// Gated on Codex plus a session shape that hides the final message
/// from stdout: a structured-stream run (`use_structured`) or an
/// interactive inline composition (`inline_interactive`, where the TUI
/// owns the terminal). A no-op for every other provider or mode.
pub(crate) fn prepare_codex_structured_output(
    provider: Provider,
    use_structured: bool,
    inline_interactive: bool,
    child_args: &mut Vec<String>,
) -> Option<StructuredCodexOutput> {
    if provider != Provider::Codex || !(use_structured || inline_interactive) {
        return None;
    }
    Some(StructuredCodexOutput::prepare(child_args))
}

#[cfg(test)]
mod tests;
