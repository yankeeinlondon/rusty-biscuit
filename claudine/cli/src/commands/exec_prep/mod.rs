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
use super::wrap::profile::{ModelSource, WrapperProfile, no_model_error, resolve_model_source};
use super::wrap::repo_home;

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
///    provider's behalf: the catalog's `model_env_vars` in order (applied
///    exactly like step 1), then `profile.configured_default_model()` (which
///    sets only `MODEL`; the provider reads its own default). Nothing found
///    is [`ModelStageError::NoModel`], unless the caller's env plan or the
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
    env_sink: &mut dyn FnMut(String, String),
    warn_sink: &mut dyn FnMut(String),
) -> Result<Option<ModelSource>, ModelStageError> {
    let info = provider_info(provider);
    let must_find_model = non_interactive && info.model_required_in_non_tty;

    let source = match model {
        Some(model) => Some(ModelSource::CliSwitch(model.to_string())),
        None if must_find_model => resolve_model_source(
            info.model_env_vars,
            |var| std::env::var(var).ok(),
            || profile.configured_default_model(),
        ),
        None => None,
    };

    match &source {
        Some(ModelSource::CliSwitch(model) | ModelSource::ProviderEnv { model, .. }) => {
            let mut env_overrides = Vec::new();
            if let Some(warn) = profile.apply_model(child_args, &mut env_overrides, model) {
                warn_sink(warn);
            }
            for (key, value) in env_overrides {
                env_sink(key, value);
            }
        }
        Some(ModelSource::ConfigDefault(configured)) => {
            env_sink("MODEL".to_string(), configured.model.clone());
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

    /// Pin the process-level sources the stage reads on the provider's behalf
    /// — `OPENCODE_MODEL` and the OpenCode config directory — so a test never
    /// observes the developer's own configuration.
    fn isolated_opencode_sources(
        config: Option<&str>,
    ) -> (test_toolkit::EnvGuard, test_toolkit::EnvGuard, tempfile::TempDir) {
        let xdg = tempfile::tempdir().unwrap();
        if let Some(config) = config {
            let dir = xdg.path().join("opencode");
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join("opencode.jsonc"), config).unwrap();
        }
        let xdg_guard = test_toolkit::EnvGuard::set_safe("XDG_CONFIG_HOME", xdg.path());
        let env_guard = test_toolkit::EnvGuard::remove_safe("OPENCODE_MODEL");
        (xdg_guard, env_guard, xdg)
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
        // The model is optional for Claude, so the launch reports no source.
        assert!(source.is_none());
        assert!(args.windows(2).any(|w| w == ["--model", "opus"]));
        assert!(env.iter().any(|(k, v)| k == "MODEL" && v == "opus"));
        assert!(warnings.is_empty());
    }

    /// The 2026-09-08 regression shape: a composition-resolved model that is
    /// outside the compiled baseline must still reach argv and `MODEL`.
    #[test]
    fn explicit_model_is_applied_for_non_interactive_opencode() {
        let mut args = vec!["run".to_string()];
        let mut env: Vec<(String, String)> = Vec::new();
        let source = resolve_model_and_validate(
            Provider::OpenCode,
            opencode_profile(),
            &mut args,
            Some("minimax/MiniMax-M3"),
            true,
            false,
            &mut |k, v| env.push((k, v)),
            &mut |_| {},
        )
        .unwrap();
        assert_eq!(
            source,
            Some(ModelSource::CliSwitch("minimax/MiniMax-M3".to_string()))
        );
        assert!(args.windows(2).any(|w| w == ["--model", "minimax/MiniMax-M3"]));
        assert!(env.contains(&("MODEL".to_string(), "minimax/MiniMax-M3".to_string())));
    }

    #[test]
    fn explicit_model_is_applied_for_interactive_opencode() {
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
        // Interactive OpenCode does not require a model, so no source is
        // reported — but the explicit model still reaches argv.
        assert!(source.is_none());
        assert!(args.windows(2).any(|w| w == ["--model", "anthropic/claude-sonnet-4"]));
    }

    #[test]
    fn interactive_opencode_without_a_model_is_left_to_the_provider() {
        let _sources = isolated_opencode_sources(Some(r#"{"model": "cfg/model"}"#));
        let mut args = Vec::new();
        let mut env: Vec<(String, String)> = Vec::new();
        let source = resolve_model_and_validate(
            Provider::OpenCode,
            opencode_profile(),
            &mut args,
            None,
            false,
            false,
            &mut |k, v| env.push((k, v)),
            &mut |_| {},
        )
        .unwrap();
        assert!(source.is_none());
        assert!(args.is_empty());
        assert!(env.is_empty());
    }

    #[test]
    fn missing_model_is_tolerated_when_the_env_plan_already_carries_one() {
        // `has_model_env` covers a caller whose env plan already exports
        // `MODEL`; the stage must neither error nor invent a source.
        let _sources = isolated_opencode_sources(None);
        let mut args = vec!["run".to_string()];
        let mut env: Vec<(String, String)> = Vec::new();
        let source = resolve_model_and_validate(
            Provider::OpenCode,
            opencode_profile(),
            &mut args,
            None,
            true,
            true,
            &mut |k, v| env.push((k, v)),
            &mut |_| {},
        )
        .unwrap();
        assert!(source.is_none());
        assert!(env.is_empty());
    }

    #[test]
    fn required_model_comes_from_the_catalog_env_var_when_nothing_is_explicit() {
        let _sources = isolated_opencode_sources(Some(r#"{"model": "cfg/model"}"#));
        let _env = test_toolkit::EnvGuard::set_safe("OPENCODE_MODEL", "env/model");
        let mut args = vec!["run".to_string()];
        let mut env: Vec<(String, String)> = Vec::new();
        let source = resolve_model_and_validate(
            Provider::OpenCode,
            opencode_profile(),
            &mut args,
            None,
            true,
            false,
            &mut |k, v| env.push((k, v)),
            &mut |_| {},
        )
        .unwrap();
        assert_eq!(
            source,
            Some(ModelSource::ProviderEnv {
                var: "OPENCODE_MODEL",
                model: "env/model".to_string()
            })
        );
        assert!(args.windows(2).any(|w| w == ["--model", "env/model"]));
        assert!(env.contains(&("MODEL".to_string(), "env/model".to_string())));
    }

    #[test]
    fn required_model_falls_back_to_the_configured_default_without_a_flag() {
        let (_xdg, _env_guard, xdg) =
            isolated_opencode_sources(Some("{\n  // default\n  \"model\": \"cfg/model\",\n}\n"));
        let mut args = vec!["run".to_string()];
        let mut env: Vec<(String, String)> = Vec::new();
        let source = resolve_model_and_validate(
            Provider::OpenCode,
            opencode_profile(),
            &mut args,
            None,
            true,
            false,
            &mut |k, v| env.push((k, v)),
            &mut |_| {},
        )
        .unwrap();
        assert_eq!(
            source,
            Some(ModelSource::ConfigDefault(profile::ConfiguredModel {
                model: "cfg/model".to_string(),
                path: xdg.path().join("opencode").join("opencode.jsonc"),
            }))
        );
        assert!(!args.iter().any(|a| a == "--model"));
        assert_eq!(env, vec![("MODEL".to_string(), "cfg/model".to_string())]);
    }

    #[test]
    fn required_model_missing_everywhere_fails_before_launch() {
        let _sources = isolated_opencode_sources(None);
        let mut args = vec!["run".to_string()];
        let mut env: Vec<(String, String)> = Vec::new();
        let result = resolve_model_and_validate(
            Provider::OpenCode,
            opencode_profile(),
            &mut args,
            None,
            true,
            false,
            &mut |k, v| env.push((k, v)),
            &mut |_| {},
        );
        let Err(ModelStageError::NoModel(report)) = result else {
            panic!("expected the no-model error, got {result:?}");
        };
        assert!(report.to_string().contains("No model specified!"));
        assert!(env.is_empty());
    }

    #[test]
    fn model_free_provider_never_searches_for_a_model() {
        let mut args = Vec::new();
        let mut env: Vec<(String, String)> = Vec::new();
        let source = resolve_model_and_validate(
            Provider::Claude,
            profile::profile_for_provider(Provider::Claude).unwrap(),
            &mut args,
            None,
            true,
            false,
            &mut |k, v| env.push((k, v)),
            &mut |_| {},
        )
        .unwrap();
        assert!(source.is_none());
        assert!(args.is_empty());
        assert!(env.is_empty());
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
