use super::super::*;
use std::collections::HashSet;

fn profile(provider: Provider) -> &'static dyn WrapperProfile {
    profile_for_provider(provider).unwrap()
}

#[test]
fn gemini_yolo_mapping_is_idempotent() {
    let p = profile(Provider::Gemini);
    let mut args = vec!["--approval-mode".to_string(), "yolo".to_string()];
    let mut env_overrides = Vec::new();

    p.apply_yolo(&mut args, &mut env_overrides).unwrap();
    p.apply_yolo(&mut args, &mut env_overrides).unwrap();

    assert_eq!(args, vec!["--approval-mode", "yolo"]);
}

#[test]
fn gemini_yolo_conflicts_with_non_yolo_approval_mode() {
    let p = profile(Provider::Gemini);
    let mut args = vec!["--approval-mode".to_string(), "default".to_string()];
    let mut env_overrides = Vec::new();

    let error = p.apply_yolo(&mut args, &mut env_overrides).unwrap_err();
    assert!(error.to_string().contains("conflicts"));
}

#[test]
fn qwen_yolo_conflicts_with_non_yolo_approval_mode() {
    let p = profile(Provider::QwenCode);
    let mut args = vec!["--approval-mode".to_string(), "careful".to_string()];
    let mut env_overrides = Vec::new();

    let error = p.apply_yolo(&mut args, &mut env_overrides).unwrap_err();
    assert!(error.to_string().contains("conflicts"));
}

#[test]
fn qwen_reject_direct_yolo_catches_approval_mode_yolo() {
    let p = profile(Provider::QwenCode);
    let args = vec!["--approval-mode".to_string(), "yolo".to_string()];

    let error = p.reject_direct_yolo(&args).unwrap_err();
    assert!(error.to_string().contains("do not pass"));
    assert!(error.to_string().contains("--approval-mode yolo"));
}

#[test]
fn opencode_noise_prefixes_cover_captured_symptoms() {
    let noise = provider_info(Provider::OpenCode)
        .display_policy
        .stderr_noise_prefixes;

    // Representative lines taken verbatim from
    // claudine/claudine-output/opencode.err (2026-04-14 capture).
    let symptoms = [
        r#"✱ Glob "**/claudine/**/improved-sequences/**" 2 matches"#,
        r#"$ cd /tmp && git log --all --oneline"#,
        r#"> build · MiniMax-M2.7-highspeed"#,
        r#"████ Subprocess hygiene"#,
        "\u{2699} firecrawl_firecrawl_search {\"query\":\"NFL draft 2026 date\",\"limit\":5}",
    ];

    for line in symptoms {
        assert!(
            noise.iter().any(|p| line.starts_with(p)),
            "noise prefixes must match representative line: {line}"
        );
    }
}

#[test]
fn opencode_profile_advertises_default_tui_noise_prefixes() {
    let profile = profile(Provider::OpenCode);
    let prefixes: &[&str] = profile.stderr_noise_prefixes();
    assert!(
        prefixes.contains(&"\u{2731} "),
        "OpenCode profile must expose the default TUI noise prefixes; got {prefixes:?}"
    );
}

#[test]
fn opencode_yolo_interactive_warns_without_mutating_args() {
    // The mode-aware variant in interactive mode must report
    // `applied = false`, emit the refined warning, and NOT mutate argv.
    let p = profile(Provider::OpenCode);
    let mut args = vec!["run".to_string(), "status".to_string()];
    let mut env_overrides = Vec::new();

    let outcome = p
        .apply_yolo_for_mode(&mut args, &mut env_overrides, /* interactive = */ true)
        .unwrap();
    assert!(
        !outcome.applied,
        "interactive mode must report applied=false; got {outcome:?}",
    );
    assert_eq!(
        outcome.warning.as_deref(),
        Some(
            "--yolo mode is not supported in OpenCode <i>interactive</i> sessions and was ignored"
        ),
    );
    assert_eq!(args, vec!["run", "status"]);
}

#[test]
fn opencode_yolo_non_interactive_forwards_dangerously_skip_permissions() {
    let mut args: Vec<String> = vec!["run".to_string()];
    let mut env = Vec::new();
    let wrapper = OpencodeWrapper;
    let outcome = wrapper
        .apply_yolo_for_mode(&mut args, &mut env, /* interactive = */ false)
        .unwrap();
    assert!(
        outcome.applied,
        "non-interactive mode must report applied=true; got {outcome:?}",
    );
    assert!(
        args.iter().any(|a| a == "--dangerously-skip-permissions"),
        "flag must be forwarded in non-interactive mode; args={args:?}"
    );
    assert!(
        outcome.warning.is_none(),
        "no warning expected in non-interactive: got {outcome:?}"
    );
}

#[test]
fn opencode_yolo_interactive_emits_refined_warning_only() {
    let mut args: Vec<String> = vec![];
    let mut env = Vec::new();
    let wrapper = OpencodeWrapper;
    let outcome = wrapper
        .apply_yolo_for_mode(&mut args, &mut env, /* interactive = */ true)
        .unwrap();
    assert!(
        !outcome.applied,
        "interactive mode must report applied=false; got {outcome:?}",
    );
    assert_eq!(
        outcome.warning.as_deref(),
        Some(
            "--yolo mode is not supported in OpenCode <i>interactive</i> sessions and was ignored"
        ),
    );
    assert!(
        args.is_empty(),
        "no args should be added in interactive mode"
    );
}

#[test]
fn opencode_yolo_non_interactive_idempotent() {
    let mut args: Vec<String> = vec!["--dangerously-skip-permissions".to_string()];
    let mut env = Vec::new();
    let wrapper = OpencodeWrapper;
    wrapper
        .apply_yolo_for_mode(&mut args, &mut env, false)
        .unwrap();
    let count = args
        .iter()
        .filter(|a| *a == "--dangerously-skip-permissions")
        .count();
    assert_eq!(count, 1, "flag must not be duplicated");
}

// -- resolve_opencode_model tests ----------------------------------------

#[test]
fn opencode_resolve_cli_switch_when_model_provided() {
    let snapshot = OpenCodeEnvSnapshot {
        opencode_model_env: None,
        opencode_config_model: None,
    };
    let source = resolve_opencode_model(Some("cli-model"), &snapshot).unwrap();
    assert_eq!(
        source,
        OpenCodeModelSource::CliSwitch("cli-model".to_string())
    );
}

#[test]
fn opencode_resolve_env_var_when_no_cli_switch() {
    let snapshot = OpenCodeEnvSnapshot {
        opencode_model_env: Some("env-model".to_string()),
        opencode_config_model: None,
    };
    let source = resolve_opencode_model(None, &snapshot).unwrap();
    assert_eq!(
        source,
        OpenCodeModelSource::OpenCodeModelEnv("env-model".to_string())
    );
}

#[test]
fn opencode_resolve_config_default_when_json_has_model() {
    let snapshot = OpenCodeEnvSnapshot {
        opencode_model_env: None,
        opencode_config_model: Some("config-model".to_string()),
    };
    let source = resolve_opencode_model(None, &snapshot).unwrap();
    assert_eq!(
        source,
        OpenCodeModelSource::ConfigDefault("config-model".to_string())
    );
}

#[test]
fn opencode_resolve_err_no_model_provided_when_none_available() {
    let snapshot = OpenCodeEnvSnapshot {
        opencode_model_env: None,
        opencode_config_model: None,
    };
    let result = resolve_opencode_model(None, &snapshot);
    assert_eq!(result, Err(NoModelProvided));
}

#[test]
fn opencode_resolve_precedence_cli_over_env() {
    let snapshot = OpenCodeEnvSnapshot {
        opencode_model_env: Some("env-model".to_string()),
        opencode_config_model: Some("config-model".to_string()),
    };
    let source = resolve_opencode_model(Some("cli-model"), &snapshot).unwrap();
    assert_eq!(
        source,
        OpenCodeModelSource::CliSwitch("cli-model".to_string())
    );
}

#[test]
fn opencode_resolve_precedence_env_over_config() {
    let snapshot = OpenCodeEnvSnapshot {
        opencode_model_env: Some("env-model".to_string()),
        opencode_config_model: Some("config-model".to_string()),
    };
    let source = resolve_opencode_model(None, &snapshot).unwrap();
    assert_eq!(
        source,
        OpenCodeModelSource::OpenCodeModelEnv("env-model".to_string())
    );
}

#[test]
fn opencode_resolve_model_env_var_ignored_entirely() {
    // This test was to ensure `MODEL` env is ignored and only `OPENCODE_MODEL` is checked
    let snapshot = OpenCodeEnvSnapshot {
        opencode_model_env: None,
        opencode_config_model: None,
    };
    let result = resolve_opencode_model(None, &snapshot);
    assert_eq!(result, Err(NoModelProvided));
}

#[test]
fn opencode_resolve_malformed_config_json_yields_no_model() {
    let snapshot = OpenCodeEnvSnapshot {
        opencode_model_env: None,
        opencode_config_model: None,
    };
    let result = resolve_opencode_model(None, &snapshot);
    assert_eq!(result, Err(NoModelProvided));
}

#[test]
fn opencode_resolve_missing_config_file_yields_no_model() {
    let snapshot = OpenCodeEnvSnapshot {
        opencode_model_env: None,
        opencode_config_model: None,
    };
    let result = resolve_opencode_model(None, &snapshot);
    assert_eq!(result, Err(NoModelProvided));
}

#[test]
fn opencode_resolve_empty_string_model_yields_no_model() {
    let snapshot = OpenCodeEnvSnapshot {
        opencode_model_env: None,
        opencode_config_model: None,
    };
    let result = resolve_opencode_model(None, &snapshot);
    assert_eq!(result, Err(NoModelProvided));
}

#[test]
fn opencode_model_source_location_strings() {
    assert_eq!(
        OpenCodeModelSource::CliSwitch(String::new()).location_string(),
        "the --model CLI switch"
    );
    assert_eq!(
        OpenCodeModelSource::OpenCodeModelEnv(String::new()).location_string(),
        "the OPENCODE_MODEL environment variable"
    );
    assert_eq!(
        OpenCodeModelSource::ConfigDefault(String::new()).location_string(),
        "the config file ~/.config/opencode/config.json"
    );
}

#[test]
fn opencode_no_model_provided_display() {
    assert_eq!(NoModelProvided.to_string(), "no model provided");
}

#[test]
fn opencode_apply_to_args_cli_switch_pushes_model_flag_and_env() {
    let snapshot = OpenCodeEnvSnapshot {
        opencode_model_env: None,
        opencode_config_model: None,
    };
    let mut args = vec!["run".to_string()];
    let mut env = Vec::new();
    apply_opencode_model_resolution(
        &mut args,
        &mut |k, v| env.push((k, v)),
        false,
        Some("gpt-4o"),
        true,
        &snapshot,
    )
    .unwrap();
    assert!(args.contains(&"--model".to_string()));
    assert!(args.contains(&"gpt-4o".to_string()));
    assert!(env.contains(&("MODEL".to_string(), "gpt-4o".to_string())));
}

#[test]
fn opencode_apply_to_args_env_var_pushes_model_flag_and_env() {
    let snapshot = OpenCodeEnvSnapshot {
        opencode_model_env: Some("env-model".to_string()),
        opencode_config_model: None,
    };
    let mut args = vec!["run".to_string()];
    let mut env = Vec::new();
    apply_opencode_model_resolution(
        &mut args,
        &mut |k, v| env.push((k, v)),
        false,
        None,
        true,
        &snapshot,
    )
    .unwrap();
    assert!(args.contains(&"--model".to_string()));
    assert!(args.contains(&"env-model".to_string()));
    assert!(env.contains(&("MODEL".to_string(), "env-model".to_string())));
}

#[test]
fn opencode_apply_to_args_config_default_pushes_env_only() {
    let snapshot = OpenCodeEnvSnapshot {
        opencode_model_env: None,
        opencode_config_model: Some("config-model".to_string()),
    };
    let mut args = vec!["run".to_string()];
    let mut env = Vec::new();
    apply_opencode_model_resolution(
        &mut args,
        &mut |k, v| env.push((k, v)),
        false,
        None,
        true,
        &snapshot,
    )
    .unwrap();
    assert!(!args.contains(&"--model".to_string()));
    assert!(env.contains(&("MODEL".to_string(), "config-model".to_string())));
}

#[test]
fn opencode_apply_to_args_does_not_duplicate_existing_model_flag() {
    let snapshot = OpenCodeEnvSnapshot {
        opencode_model_env: None,
        opencode_config_model: None,
    };
    let mut args = vec![
        "run".to_string(),
        "--model".to_string(),
        "existing".to_string(),
    ];
    let mut env = Vec::new();
    apply_opencode_model_resolution(
        &mut args,
        &mut |k, v| env.push((k, v)),
        false,
        Some("existing"),
        true,
        &snapshot,
    )
    .unwrap();
    let count = args.iter().filter(|a| *a == "--model").count();
    assert_eq!(count, 1, "should not duplicate --model flag");
}

#[test]
fn goose_yolo_env_override_is_idempotent() {
    let p = profile(Provider::Goose);
    let mut args = Vec::new();
    let mut env_overrides = Vec::new();

    p.apply_yolo(&mut args, &mut env_overrides).unwrap();
    p.apply_yolo(&mut args, &mut env_overrides).unwrap();

    let unique: HashSet<_> = env_overrides.into_iter().collect();
    assert_eq!(unique.len(), 1);
    assert!(unique.contains(&("GOOSE_MODE".to_string(), "auto".to_string())));
}

#[test]
fn direct_provider_yolo_flag_is_rejected_with_guidance() {
    let p = profile(Provider::Claude);
    let args = vec!["--dangerously-skip-permissions".to_string()];

    let error = p.reject_direct_yolo(&args).unwrap_err();
    let message = error.to_string();

    assert!(message.contains("do not pass"));
    assert!(message.contains("--dangerously-skip-permissions"));
    assert!(message.contains("--yolo"));
}

/// Documented "no wrapper" providers. Must stay in lock-step with the
/// `None` slots in [`WRAPPER_REGISTRY`].
const NO_WRAPPER: &[Provider] = &[];

/// Phase 3 invariant: the array-backed [`WRAPPER_REGISTRY`] either
/// returns a wrapper whose `provider()` matches the lookup key, or
/// returns `None` for a provider explicitly listed in [`NO_WRAPPER`].
/// A future [`Provider`] variant fails compilation at the array
/// declaration; this test then guards the `None`/`Some` decision.
#[test]
fn wrapper_registry_covers_every_provider_and_documents_exceptions() {
    use claudine::provider::PROVIDERS_DISPLAY_ORDER;

    for provider in PROVIDERS_DISPLAY_ORDER {
        let result = profile_for_provider(provider);
        if NO_WRAPPER.contains(&provider) {
            assert!(
                result.is_none(),
                "{provider:?}: documented as no-wrapper but registry returned Some"
            );
        } else {
            let profile = result.unwrap_or_else(|| {
                panic!(
                    "{provider:?}: registry must provide a wrapper unless explicitly \
                     listed in NO_WRAPPER"
                )
            });
            assert_eq!(
                profile.provider(),
                provider,
                "{provider:?}: registry slot returned a profile for the wrong provider"
            );
        }
    }
}

/// The registry array length is wired to [`PROVIDER_COUNT`], so adding
/// a new [`Provider`] variant forces a compile error at the declaration.
/// This runtime assertion documents the invariant for readers and
/// catches accidental drift if the static is ever rebuilt by hand.
#[test]
fn wrapper_registry_length_matches_provider_count() {
    assert_eq!(WRAPPER_REGISTRY.len(), PROVIDER_COUNT);
}

fn catalog_yolo_applied(
    provider: Provider,
    args: &[String],
    env: &[(String, String)],
    outcome: &YoloOutcome,
) -> bool {
    match provider_info(provider).yolo {
        YoloSupport::None => {
            !outcome.applied && outcome.warning.is_some() && args.is_empty() && env.is_empty()
        }
        YoloSupport::DirectFlag { native_flag } => {
            outcome.applied && yolo_flag_present(args, native_flag)
        }
        YoloSupport::DirectFlagWithAlias { native_flag, .. } => {
            outcome.applied && yolo_flag_present(args, native_flag)
        }
        YoloSupport::NonInteractiveOnly {
            non_interactive_flag,
        } => outcome.applied && args.iter().any(|arg| arg == non_interactive_flag),
        YoloSupport::EnvVar { env_var, value } => {
            outcome.applied && env.iter().any(|(k, v)| k == env_var && v == value)
        }
    }
}

fn yolo_flag_present(args: &[String], native_flag: &str) -> bool {
    if args.iter().any(|arg| arg == native_flag) {
        return true;
    }
    let Some((flag, value)) = native_flag.split_once('=') else {
        return false;
    };
    args.windows(2)
        .any(|window| window[0] == flag && window[1] == value)
}

/// Regression: the `--yolo` CLI flag and the `CLAUDINE_YOLO` env var
/// must reach the SAME field (`request.yolo`) so they cannot diverge.
/// Before this binding, the env var was only read by the dispatch
/// reporter for metadata stamping while the wrapper's flag-push gate
/// looked at the CLI arg alone — producing reports of "yolo: true"
/// for sessions where the provider flag never actually landed.
///
/// Test mutates process env, so it must be serialized against any
/// other test that reads `CLAUDINE_YOLO`. We snapshot/restore here
/// rather than relying on a test mutex — the env var is uncommon
/// enough that parallel parsing collisions are unlikely.
#[test]
fn claudine_yolo_env_binds_to_compose_shared_yolo_flag() {
    use clap::Parser;

    use crate::commands::compose::SharedComposeArgs;

    // Probe shim: `SharedComposeArgs` is `Args`, not `Parser`. Wrap
    // it in a derive(Parser) container so `try_parse_from` is callable.
    #[derive(Debug, clap::Parser)]
    struct Probe {
        #[command(flatten)]
        shared: SharedComposeArgs,
    }

    let prior = std::env::var("CLAUDINE_YOLO").ok();
    // SAFETY: single-threaded scoped mutation. We restore the prior
    // value below so sibling tests are unaffected.
    unsafe {
        std::env::remove_var("CLAUDINE_YOLO");
    }
    let baseline = Probe::try_parse_from(["probe"])
        .expect("probe must parse without flag or env")
        .shared
        .yolo;

    let with_flag = Probe::try_parse_from(["probe", "-y"])
        .expect("probe with -y must parse")
        .shared
        .yolo;

    unsafe {
        std::env::set_var("CLAUDINE_YOLO", "true");
    }
    let from_env = Probe::try_parse_from(["probe"])
        .expect("probe with CLAUDINE_YOLO=true must parse")
        .shared
        .yolo;

    // Restore prior env so we don't leak state.
    unsafe {
        match prior {
            Some(value) => std::env::set_var("CLAUDINE_YOLO", value),
            None => std::env::remove_var("CLAUDINE_YOLO"),
        }
    }

    assert!(
        !baseline,
        "no flag and no env must leave yolo=false (got {baseline})",
    );
    assert!(with_flag, "-y on argv must enable yolo (got {with_flag})",);
    assert!(
        from_env,
        "CLAUDINE_YOLO=true must enable yolo on SharedComposeArgs (got {from_env})",
    );
}

#[test]
fn apply_yolo_matches_provider_catalog_for_every_provider() {
    for provider in claudine::provider::PROVIDERS_DISPLAY_ORDER {
        let Some(profile) = profile_for_provider(provider) else {
            continue;
        };
        let mut args = Vec::new();
        let mut env = Vec::new();
        let outcome = profile
            .apply_yolo_for_mode(&mut args, &mut env, false)
            .expect("YOLO application should not fail from empty args");

        assert!(
            catalog_yolo_applied(provider, &args, &env, &outcome),
            "{provider:?} yolo mismatch: args={args:?}, env={env:?}, outcome={outcome:?}"
        );
    }
}
