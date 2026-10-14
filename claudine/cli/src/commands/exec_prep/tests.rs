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
        &claudine::invocation_context::DocumentEnvironment::default(),
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
        &claudine::invocation_context::DocumentEnvironment::default(),
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
        &claudine::invocation_context::DocumentEnvironment::default(),
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
        &claudine::invocation_context::DocumentEnvironment::default(),
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
        &claudine::invocation_context::DocumentEnvironment::default(),
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
    let environment: claudine::invocation_context::DocumentEnvironment =
        [("OPENCODE_MODEL", "env/model")].into_iter().collect();
    let mut args = vec!["run".to_string()];
    let mut env: Vec<(String, String)> = Vec::new();
    let source = resolve_model_and_validate(
        Provider::OpenCode,
        opencode_profile(),
        &mut args,
        None,
        true,
        false,
        &environment,
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

/// A configured default is delivered on argv, not left for the provider to
/// rediscover: the child's environment is not the one the file was read in.
#[test]
fn required_model_falls_back_to_the_configured_default_on_argv() {
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
        &claudine::invocation_context::DocumentEnvironment::default(),
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
    assert!(args.windows(2).any(|w| w == ["--model", "cfg/model"]));
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
        &claudine::invocation_context::DocumentEnvironment::default(),
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
        &claudine::invocation_context::DocumentEnvironment::default(),
        &mut |k, v| env.push((k, v)),
        &mut |_| {},
    )
    .unwrap();
    assert!(source.is_none());
    assert!(args.is_empty());
    assert!(env.is_empty());
}

/// Off the gate, or for Claude, which delivers MCP without a
/// Claudine-provided config root and must not plan an overlay it would refuse.
#[test]
fn ensure_provider_overlay_is_a_no_op_off_the_gate_or_without_a_root_requiring_injector() {
    let no_home = HomeBaseline::from_parts(None, Default::default());
    for (provider, needs_overlay) in [(Provider::Codex, false), (Provider::Claude, true)] {
        let mut plan = EnvPlan::default();
        ensure_provider_overlay(provider, needs_overlay, &mut plan, &no_home, &EnvBaseline::default())
            .unwrap();
        assert!(plan.overlay.is_none() && plan.env.is_empty(), "{provider}");
    }
}

#[test]
fn ensure_provider_overlay_keeps_an_existing_overlay() {
    use claudine::provider_overlay::{OverlayPlanner, OverlayReasons};
    let scratch = tempfile::tempdir().unwrap();
    let home = scratch.path().join("home");
    let home_baseline = HomeBaseline::from_parts(Some(home.clone()), Default::default());
    let env = EnvBaseline::default();
    let existing = OverlayPlanner::new(&home_baseline, &env)
        .plan(Provider::Codex, OverlayReasons::single(OverlayReason::Mcp))
        .unwrap();
    let mut plan = EnvPlan { overlay: Some(existing.clone()), ..EnvPlan::default() };
    ensure_provider_overlay(Provider::Codex, true, &mut plan, &home_baseline, &env).unwrap();
    assert_eq!(plan.overlay, Some(existing));
    assert!(plan.env.is_empty());
    assert!(!home.exists(), "an existing overlay is not rebuilt");
}
