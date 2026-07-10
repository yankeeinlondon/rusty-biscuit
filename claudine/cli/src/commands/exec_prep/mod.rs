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

use claudine::provider::{Provider, provider_info};
use color_eyre::eyre::Result;

use super::wrap::env::EnvPlan;
use super::wrap::policy::StructuredCodexOutput;
use super::wrap::profile::{
    OpenCodeEnvSnapshot, OpenCodeModelSource, WrapperProfile, apply_opencode_model_resolution,
};
use super::wrap::repo_home;

/// Failure from [`resolve_model_and_validate`], split so each pipeline
/// keeps its own presentation.
#[derive(Debug)]
pub(crate) enum ModelStageError {
    /// Non-interactive OpenCode launch with no model from any source
    /// (CLI switch, `OPENCODE_MODEL`, config default). The direct
    /// wrapper renders `AgentErrorReport::no_model_provided` and exits;
    /// composition propagates the message.
    NoOpenCodeModel(color_eyre::eyre::Error),
    /// Provider-trait non-interactive validation failure; both
    /// pipelines propagate it unchanged.
    Validation(color_eyre::eyre::Error),
}

impl ModelStageError {
    pub(crate) fn into_report(self) -> color_eyre::eyre::Error {
        match self {
            Self::NoOpenCodeModel(report) | Self::Validation(report) => report,
        }
    }
}

/// Resolve the session model onto the child argv/env and run the
/// provider's non-interactive validation.
///
/// Covers three formerly duplicated steps, in order:
///
/// 1. OpenCode model resolution via
///    [`apply_opencode_model_resolution`] (acts only when
///    non-interactive; the resolved source feeds the wrapper's
///    preflight preamble).
/// 2. The universal `--model` flag through `profile.apply_model` — for
///    non-OpenCode providers always, and for OpenCode only when
///    interactive (non-interactive OpenCode was handled by step 1).
/// 3. `profile.validate_non_interactive_requirements` for non-OpenCode
///    providers in non-interactive mode.
///
/// `has_model_env` is whether the caller's env plan already carries a
/// `MODEL` entry; env writes go through `env_sink` (the wrapper pushes
/// onto `env_overrides`, composition inserts into `env_plan.env`) and
/// unsupported-model warnings through `warn_sink` (deferred vs. logged
/// immediately).
#[allow(clippy::too_many_arguments)]
pub(crate) fn resolve_model_and_validate(
    provider: Provider,
    profile: &dyn WrapperProfile,
    child_args: &mut Vec<String>,
    model: Option<&str>,
    non_interactive: bool,
    has_model_env: bool,
    env_sink: &mut dyn FnMut(String, String),
    warn_sink: &mut dyn FnMut(String),
) -> Result<Option<OpenCodeModelSource>, ModelStageError> {
    // Gated on the catalog's `model_required_in_non_tty` (OpenCode is the
    // only provider setting it today, so semantics are identical). The
    // resolution helper itself is still OpenCode-flavored (OPENCODE_MODEL
    // env, `OpenCodeEnvSnapshot`) — a future provider setting this bool
    // needs the helper generalized first.
    let model_required_in_non_tty = provider_info(provider).model_required_in_non_tty;
    let opencode_model_source = if model_required_in_non_tty {
        apply_opencode_model_resolution(
            child_args,
            env_sink,
            has_model_env,
            model,
            non_interactive,
            &OpenCodeEnvSnapshot::from_system(),
        )
        .map_err(ModelStageError::NoOpenCodeModel)?
    } else {
        None
    };

    if (!model_required_in_non_tty || !non_interactive)
        && let Some(model) = model
    {
        let mut env_overrides = Vec::new();
        if let Some(warn) = profile.apply_model(child_args, &mut env_overrides, model) {
            warn_sink(warn);
        }
        for (key, value) in env_overrides {
            env_sink(key, value);
        }
    }

    if !model_required_in_non_tty && non_interactive {
        profile
            .validate_non_interactive_requirements(child_args)
            .map_err(ModelStageError::Validation)?;
    }

    Ok(opencode_model_source)
}

/// Guarantee `env_plan.shadow_home_path` before provider-config
/// injection when the session needs a shadow HOME.
///
/// `env::build_child_env_with_launch` already attempts the shadow-home
/// build whenever its `force_shadow_home` flag is set, and a successful
/// build always records `shadow_home_path` — so this stage is a no-op
/// on the happy path. It is reachable only when that builder degraded
/// to a warning (`failed to create shadow HOME`, HOME=/dev/null). Here
/// the retry's failure is a hard error rather than another warning:
/// the caller is about to inject provider config that must land in the
/// shadow HOME, not the user's real one.
pub(crate) fn ensure_shadow_home(
    provider: Provider,
    needs_shadow_home: bool,
    env_plan: &mut EnvPlan,
) -> Result<()> {
    if !needs_shadow_home || env_plan.shadow_home_path.is_some() {
        return Ok(());
    }
    let (shadow_env, shadow_path, _) = repo_home::build_repo_home_env(
        provider,
        env_plan.child_cwd.as_path(),
        false,
        false,
        Some(env_plan.child_cwd.as_path()),
    )?;
    for (key, value) in shadow_env {
        env_plan.env.insert(key, value);
    }
    env_plan.shadow_home_path = shadow_path;
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
mod tests {
    use super::*;
    use crate::commands::wrap::profile;

    fn opencode_profile() -> &'static dyn WrapperProfile {
        profile::profile_for_provider(Provider::OpenCode).unwrap()
    }

    #[test]
    fn codex_structured_output_prepares_for_structured_runs() {
        let mut args = vec!["exec".to_string()];
        let output = prepare_codex_structured_output(Provider::Codex, true, false, &mut args);
        assert!(output.is_some());
        assert!(args.iter().any(|a| a == "--output-last-message"));
    }

    #[test]
    fn codex_structured_output_prepares_for_inline_interactive() {
        let mut args = Vec::new();
        let output = prepare_codex_structured_output(Provider::Codex, false, true, &mut args);
        assert!(output.is_some());
        assert!(args.iter().any(|a| a == "--output-last-message"));
    }

    #[test]
    fn codex_structured_output_is_a_no_op_off_the_gate() {
        let mut args = Vec::new();
        assert!(
            prepare_codex_structured_output(Provider::Codex, false, false, &mut args).is_none()
        );
        assert!(prepare_codex_structured_output(Provider::Claude, true, true, &mut args).is_none());
        assert!(args.is_empty());
    }

    #[test]
    fn resolve_model_applies_universal_model_for_non_opencode() {
        let claude = profile::profile_for_provider(Provider::Claude).unwrap();
        let mut args = Vec::new();
        let mut env: Vec<(String, String)> = Vec::new();
        let mut warnings: Vec<String> = Vec::new();
        let source = resolve_model_and_validate(
            Provider::Claude,
            claude,
            &mut args,
            Some("opus"),
            false,
            false,
            &mut |k, v| env.push((k, v)),
            &mut |w| warnings.push(w),
        )
        .unwrap();
        assert!(source.is_none());
        assert!(args.windows(2).any(|w| w == ["--model", "opus"]));
        assert!(env.iter().any(|(k, v)| k == "MODEL" && v == "opus"));
        assert!(warnings.is_empty());
    }

    #[test]
    fn resolve_model_uses_opencode_resolution_when_non_interactive() {
        let mut args = vec!["run".to_string()];
        let mut env: Vec<(String, String)> = Vec::new();
        let source = resolve_model_and_validate(
            Provider::OpenCode,
            opencode_profile(),
            &mut args,
            Some("anthropic/claude-sonnet-4"),
            true,
            false,
            &mut |k, v| env.push((k, v)),
            &mut |_| {},
        )
        .unwrap();
        assert!(matches!(source, Some(OpenCodeModelSource::CliSwitch(_))));
        assert!(args.iter().any(|a| a == "--model"));
        assert!(env.iter().any(|(k, _)| k == "MODEL"));
    }

    #[test]
    fn resolve_model_keeps_interactive_opencode_on_apply_model_path() {
        let mut args = Vec::new();
        let mut env: Vec<(String, String)> = Vec::new();
        let source = resolve_model_and_validate(
            Provider::OpenCode,
            opencode_profile(),
            &mut args,
            Some("anthropic/claude-sonnet-4"),
            false,
            false,
            &mut |k, v| env.push((k, v)),
            &mut |_| {},
        )
        .unwrap();
        // Interactive OpenCode skips resolution (no source) but still
        // receives the universal --model flag.
        assert!(source.is_none());
        assert!(args.iter().any(|a| a == "--model"));
    }

    #[test]
    fn ensure_shadow_home_is_a_no_op_off_the_gate() {
        let mut plan = EnvPlan::default();
        ensure_shadow_home(Provider::Codex, false, &mut plan).unwrap();
        assert!(plan.shadow_home_path.is_none());
        assert!(plan.env.is_empty());
    }

    #[test]
    fn ensure_shadow_home_keeps_existing_path() {
        let mut plan = EnvPlan {
            shadow_home_path: Some(std::path::PathBuf::from("/tmp/shadow")),
            ..EnvPlan::default()
        };
        ensure_shadow_home(Provider::Codex, true, &mut plan).unwrap();
        assert_eq!(
            plan.shadow_home_path.as_deref(),
            Some(std::path::Path::new("/tmp/shadow"))
        );
        assert!(plan.env.is_empty());
    }
}
