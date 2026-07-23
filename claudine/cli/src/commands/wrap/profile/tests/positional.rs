use super::super::*;

fn profile(provider: Provider) -> &'static dyn WrapperProfile {
    profile_for_provider(provider).unwrap()
}

// -- PromptSource tests ------------------------------------------------

#[test]
fn prompt_source_as_inline_returns_text_for_inline_variant() {
    let source = PromptSource::Inline("hello".to_string());
    assert_eq!(source.as_inline(), Some("hello"));
}

#[test]
fn prompt_source_as_inline_returns_none_for_non_inline_variants() {
    assert_eq!(PromptSource::None.as_inline(), None);
    assert_eq!(PromptSource::InheritStdin.as_inline(), None);
}

#[test]
fn prompt_source_is_none_only_true_for_none_variant() {
    assert!(PromptSource::None.is_none());
    assert!(!PromptSource::Inline("hi".to_string()).is_none());
    assert!(!PromptSource::InheritStdin.is_none());
}

#[test]
fn prompt_source_has_prompt_or_stdin_accepts_inline_and_stdin() {
    assert!(!PromptSource::None.has_prompt_or_stdin());
    assert!(PromptSource::Inline("x".to_string()).has_prompt_or_stdin());
    assert!(PromptSource::InheritStdin.has_prompt_or_stdin());
}

#[test]
fn qwen_apply_non_interactive_flags_rejects_prompt_interactive() {
    let p = profile(Provider::QwenCode);
    let mut args = vec!["-i".to_string(), "task".to_string()];
    let err = p.apply_non_interactive_flags(&mut args).unwrap_err();
    assert!(err.to_string().contains("conflicts"));
}

#[test]
fn qwen_apply_non_interactive_flags_allows_empty_args_for_composition() {
    let p = profile(Provider::QwenCode);
    let mut args: Vec<String> = Vec::new();
    p.apply_non_interactive_flags(&mut args).unwrap();
    assert!(args.is_empty());
}

/// Regression test for the composition pipeline path: non-interactive
/// flag application must NOT bail when args are empty, because the
/// prompt arrives later via `prompt_delivery`.
#[test]
fn gemini_apply_non_interactive_flags_allows_empty_args_for_composition() {
    let p = profile(Provider::Gemini);
    let mut args: Vec<String> = Vec::new();
    p.apply_non_interactive_flags(&mut args).unwrap();
    assert!(args.is_empty());
}

#[test]
fn gemini_apply_non_interactive_flags_rejects_interactive_mode_flags() {
    let p = profile(Provider::Gemini);
    let mut args = vec!["-i".to_string()];
    let err = p.apply_non_interactive_flags(&mut args).unwrap_err();
    assert!(err.to_string().contains("conflicts"));
}

/// When a prompt starts with '-' some CLI parsers (notably yargs,
/// used by Gemini) interpret the value as a flag or end-of-options
/// marker, causing '--prompt ---...' to fail with "Not enough
/// arguments following: prompt". The fix uses '--prompt=---...'
/// syntax when the prompt starts with '-'.
#[test]
fn gemini_prompt_delivery_uses_equals_syntax_when_prompt_starts_with_dash() {
    let p = profile(Provider::Gemini);
    let args = Vec::new();
    let delivery = p.prompt_delivery(&args, "---\nfoo", true).unwrap();
    let mut applied = Vec::new();
    delivery.apply_to(&mut applied);
    assert_eq!(applied, vec!["--prompt=---\nfoo"]);
}

#[test]
fn gemini_prompt_delivery_uses_space_syntax_for_normal_prompt() {
    let p = profile(Provider::Gemini);
    let args = Vec::new();
    let delivery = p.prompt_delivery(&args, "hello world", true).unwrap();
    let mut applied = Vec::new();
    delivery.apply_to(&mut applied);
    assert_eq!(applied, vec!["--prompt", "hello world"]);
}

#[test]
fn qwen_prompt_delivery_uses_equals_syntax_when_prompt_starts_with_dash() {
    let p = profile(Provider::QwenCode);
    let args = Vec::new();
    let delivery = p.prompt_delivery(&args, "---\nfoo", true).unwrap();
    let mut applied = Vec::new();
    delivery.apply_to(&mut applied);
    assert_eq!(applied, vec!["--prompt=---\nfoo"]);
}

#[test]
fn claude_interactive_prompt_starting_with_dash_is_separated_with_end_of_options() {
    let p = profile(Provider::Claude);
    let args = Vec::new();
    let delivery = p
        .prompt_delivery(&args, "- review the spec\n- ask questions", false)
        .unwrap();
    let mut applied = Vec::new();
    delivery.apply_to(&mut applied);
    assert_eq!(
        applied,
        vec![
            "--".to_string(),
            "- review the spec\n- ask questions".to_string(),
        ]
    );
}

#[test]
fn claude_interactive_prompt_without_dash_keeps_plain_positional_arg() {
    let p = profile(Provider::Claude);
    let args = Vec::new();
    let delivery = p.prompt_delivery(&args, "review the spec", false).unwrap();
    let mut applied = Vec::new();
    delivery.apply_to(&mut applied);
    assert_eq!(applied, vec!["review the spec"]);
}

#[test]
fn opencode_non_interactive_prompt_body_uses_positional_arg() {
    let p = profile(Provider::OpenCode);
    let mut args = vec!["run".to_string()];
    let stdin_seed = p
        .prompt_delivery(&args, "summarize staged files", true)
        .unwrap()
        .apply_to(&mut args);
    assert_eq!(stdin_seed, None);
    assert_eq!(args, vec!["run", "--", "summarize staged files"]);
}

#[test]
fn opencode_non_interactive_prompt_starting_with_dash_is_separated_with_end_of_options() {
    // Regression: OpenCode's yargs parser prints help and exits when a
    // positional prompt begins with `-`. Claudine must emit `--` before
    // the prompt so composed bullet-list prompts are delivered intact.
    let p = profile(Provider::OpenCode);
    let mut args = vec![
        "run".to_string(),
        "--format".to_string(),
        "json".to_string(),
    ];
    p.prompt_delivery(&args, "- implement the plan\n- use the skill", true)
        .unwrap()
        .apply_to(&mut args);
    let sep_index = args
        .iter()
        .position(|a| a == "--")
        .expect("`--` separator must be present");
    let prompt_index = args
        .iter()
        .position(|a| a == "- implement the plan\n- use the skill")
        .expect("prompt must be present as a positional");
    assert!(
        sep_index < prompt_index,
        "`--` must precede the prompt: {args:?}"
    );
}

#[test]
fn opencode_interactive_prompt_starting_with_dash_uses_attached_prompt_flag() {
    // Regression: OpenCode's yargs parser treats a `--prompt` value beginning
    // with `-` (a Markdown bullet) as an option, prints its help, and exits 1.
    // The attached `--prompt=<value>` form binds the value to the flag.
    let p = profile(Provider::OpenCode);
    let args = Vec::new();
    let delivery = p
        .prompt_delivery(&args, "- review the spec\n- ask questions", false)
        .unwrap();
    let mut applied = Vec::new();
    delivery.apply_to(&mut applied);
    assert_eq!(
        applied,
        vec!["--prompt=- review the spec\n- ask questions".to_string()]
    );
}

#[test]
fn opencode_interactive_prompt_without_dash_keeps_space_separated_flag() {
    let p = profile(Provider::OpenCode);
    let args = Vec::new();
    let delivery = p.prompt_delivery(&args, "review the spec", false).unwrap();
    let mut applied = Vec::new();
    delivery.apply_to(&mut applied);
    assert_eq!(applied, vec!["--prompt".to_string(), "review the spec".to_string()]);
}

#[test]
fn codex_interactive_prompt_starting_with_dash_is_separated_with_end_of_options() {
    // Regression: Codex's clap parser rejects a leading-`-` positional prompt
    // ("unexpected argument '- ' found"). The prompt must follow a `--`
    // end-of-options marker placed after every flag.
    let p = profile(Provider::Codex);
    let args = vec!["--model".to_string(), "gpt-5".to_string()];
    let delivery = p
        .prompt_delivery(&args, "- review the spec\n- ask questions", false)
        .unwrap();
    let mut applied = args.clone();
    delivery.apply_to(&mut applied);
    assert_eq!(
        applied,
        vec![
            "--model".to_string(),
            "gpt-5".to_string(),
            "--".to_string(),
            "- review the spec\n- ask questions".to_string(),
        ]
    );
}

#[test]
fn codex_interactive_prompt_without_dash_keeps_leading_positional() {
    let p = profile(Provider::Codex);
    let args = vec!["--model".to_string(), "gpt-5".to_string()];
    let delivery = p.prompt_delivery(&args, "review the spec", false).unwrap();
    let mut applied = args.clone();
    delivery.apply_to(&mut applied);
    assert_eq!(
        applied,
        vec![
            "review the spec".to_string(),
            "--model".to_string(),
            "gpt-5".to_string(),
        ]
    );
}

fn catalog_entrypoint_args(provider: Provider, non_interactive: bool) -> Vec<String> {
    let info = provider_info(provider);
    let target_mode = if non_interactive {
        EntrypointMode::NonInteractive
    } else {
        EntrypointMode::Interactive
    };
    let mut args = Vec::new();
    if let Some(ep) = info
        .entrypoints
        .iter()
        .find(|ep| matches!(ep.mode, EntrypointMode::Both) || ep.mode == target_mode)
    {
        if let Some(sub) = ep.subcommand {
            args.push(sub.to_string());
        }
        args.extend(ep.required_flags.iter().map(|flag| (*flag).to_string()));
    }
    args
}

#[test]
fn apply_entrypoint_matches_provider_catalog_for_every_provider() {
    for provider in claudine::provider::PROVIDERS_DISPLAY_ORDER {
        let Some(profile) = profile_for_provider(provider) else {
            continue;
        };
        let mut args = Vec::new();
        profile.apply_entrypoint(&mut args, true);
        assert_eq!(
            args,
            catalog_entrypoint_args(provider, true),
            "{provider:?} non-interactive entrypoint"
        );
    }
}

// -- PromptArgConventions tests -----------------------------------------

#[test]
fn prompt_arg_conventions_claude_uses_defaults() {
    let conv = profile(Provider::Claude).prompt_arg_conventions();
    assert!(conv.prompt_flags.is_empty());
    assert_eq!(conv.entrypoint, None);
}

#[test]
fn prompt_arg_conventions_codex_uses_exec_entrypoint() {
    let conv = profile(Provider::Codex).prompt_arg_conventions();
    assert_eq!(conv.entrypoint, Some("exec"));
    assert!(conv.prompt_flags.is_empty());
}

#[test]
fn prompt_arg_conventions_gemini_uses_prompt_flags() {
    let conv = profile(Provider::Gemini).prompt_arg_conventions();
    assert_eq!(conv.prompt_flags, &["-p", "--prompt"]);
    assert_eq!(conv.entrypoint, None);
}

#[test]
fn prompt_arg_conventions_goose_uses_run_entrypoint_and_text_flags() {
    let conv = profile(Provider::Goose).prompt_arg_conventions();
    assert_eq!(conv.entrypoint, Some("run"));
    assert_eq!(conv.prompt_flags, &["-t", "--text"]);
}

#[test]
fn prompt_arg_conventions_kimi_uses_long_prompt_flag_only() {
    let conv = profile(Provider::KimiCode).prompt_arg_conventions();
    assert_eq!(conv.prompt_flags, &["--prompt"]);
    assert_eq!(conv.entrypoint, None);
}

#[test]
fn kimi_non_interactive_uses_wire_protocol_and_wire_rpc_delivery() {
    let p = profile(Provider::KimiCode);
    assert_eq!(p.stream_protocol(), Some(StreamProtocol::WireJsonRpc));

    let mut args: Vec<String> = Vec::new();
    p.apply_entrypoint(&mut args, true);
    assert!(args.contains(&"--wire".to_string()));
    assert!(!args.contains(&"--print".to_string()));

    let mut structured_args: Vec<String> = Vec::new();
    p.apply_structured_stream(&mut structured_args);
    assert_eq!(structured_args, vec!["--wire".to_string()]);

    let delivery = p
        .prompt_delivery(&args, "hello kimi", true)
        .expect("kimi prompt_delivery should succeed");
    match delivery {
        PromptDelivery::WireRpc(prompt) => assert_eq!(prompt, "hello kimi"),
        other => panic!("expected WireRpc delivery, got {other:?}"),
    }
}

#[test]
fn kimi_interactive_continues_using_prompt_argv_flag() {
    let p = profile(Provider::KimiCode);
    let mut args: Vec<String> = Vec::new();
    p.apply_entrypoint(&mut args, false);
    assert!(args.is_empty(), "interactive must not append --wire");

    let delivery = p
        .prompt_delivery(&args, "hello", false)
        .expect("kimi prompt_delivery should succeed in interactive mode");
    match delivery {
        PromptDelivery::AppendArgs(extra) => {
            assert_eq!(extra, vec!["--prompt".to_string(), "hello".to_string()]);
        }
        other => panic!("expected AppendArgs delivery, got {other:?}"),
    }
}

#[test]
fn kimi_resume_uses_wire_flag() {
    let p = profile(Provider::KimiCode);
    let resume = p.build_resume_args("session-123").unwrap();
    assert_eq!(
        resume,
        vec![
            "kimi".to_string(),
            "--resume".to_string(),
            "session-123".to_string(),
            "--wire".to_string(),
        ]
    );
    assert!(!resume.contains(&"--print".to_string()));
}

#[test]
fn gemini_resume_uses_explicit_session_id() {
    let p = profile(Provider::Gemini);
    let resume = p.build_resume_args("abc-123").unwrap();
    assert_eq!(
        resume,
        vec![
            "gemini".to_string(),
            "--resume".to_string(),
            "abc-123".to_string(),
        ]
    );
}

#[test]
fn goose_resume_uses_run_with_explicit_session_id() {
    let p = profile(Provider::Goose);
    let resume = p.build_resume_args("20260704_1").unwrap();
    assert_eq!(
        resume,
        vec![
            "goose".to_string(),
            "run".to_string(),
            "--resume".to_string(),
            "--session-id".to_string(),
            "20260704_1".to_string(),
        ]
    );
    // `prompt_delivery` finds the `run` entrypoint in the resume argv, so
    // the follow-up prompt lands as `run -t <prompt> ...`.
    let delivery = p
        .prompt_delivery(&resume[1..], "follow up", true)
        .expect("goose prompt_delivery should succeed on resume argv");
    match delivery {
        PromptDelivery::InsertArgs { index, args } => {
            assert_eq!(index, 1);
            assert_eq!(args, vec!["-t".to_string(), "follow up".to_string()]);
        }
        other => panic!("expected InsertArgs delivery, got {other:?}"),
    }
}

#[test]
fn opencode_resume_uses_run_with_explicit_session() {
    let p = profile(Provider::OpenCode);
    let resume = p.build_resume_args("ses_abc123").unwrap();
    assert_eq!(
        resume,
        vec![
            "opencode".to_string(),
            "run".to_string(),
            "--session".to_string(),
            "ses_abc123".to_string(),
        ]
    );
    assert!(!resume.contains(&"--continue".to_string()));
}

#[test]
fn every_provider_profile_supports_resume() {
    // Ratified end-state (2026-07-04): provider-native resume support means
    // Claudine lifecycle-resume support. Every compiled provider has
    // first-class non-interactive resume per the session-resumption research,
    // so every profile must implement the `supports_resume` + `build_resume_args`
    // pair; a `false` is only ever a not-yet-implemented gap.
    for provider in claudine::provider::PROVIDERS_DISPLAY_ORDER {
        let p = profile(provider);
        assert!(
            p.supports_resume(),
            "{provider:?}: profile must support lifecycle resume"
        );
        assert!(
            p.build_resume_args("session-id").is_ok(),
            "{provider:?}: build_resume_args must produce a resume argv"
        );
    }
}

#[test]
fn prompt_arg_conventions_opencode_uses_run_entrypoint() {
    let conv = profile(Provider::OpenCode).prompt_arg_conventions();
    assert_eq!(conv.entrypoint, Some("run"));
    assert!(conv.prompt_flags.is_empty());
}

#[test]
fn prompt_arg_conventions_qwen_uses_prompt_flags() {
    let conv = profile(Provider::QwenCode).prompt_arg_conventions();
    assert_eq!(conv.prompt_flags, &["-p", "--prompt"]);
    assert_eq!(conv.entrypoint, None);
}

// -- extract_prompt_source_from_passthrough ----------------------------

fn extract(
    provider: Provider,
    passthrough: &[&str],
    has_piped_stdin: bool,
) -> (Vec<String>, PromptSource) {
    let args: Vec<String> = passthrough.iter().map(|s| s.to_string()).collect();
    extract_prompt_source_from_passthrough(profile(provider), &args, has_piped_stdin)
        .expect("extract_prompt_source_from_passthrough should succeed")
}

#[test]
fn extract_claude_no_args_yields_none() {
    let (args, source) = extract(Provider::Claude, &[], false);
    assert!(args.is_empty());
    assert_eq!(source, PromptSource::None);
}

#[test]
fn extract_claude_bare_positional_yields_inline() {
    let (args, source) = extract(Provider::Claude, &["hello"], false);
    assert!(args.is_empty());
    assert_eq!(source, PromptSource::Inline("hello".to_string()));
}

#[test]
fn extract_claude_piped_stdin_yields_inherit_stdin() {
    let (args, source) = extract(Provider::Claude, &[], true);
    assert!(args.is_empty());
    assert_eq!(source, PromptSource::InheritStdin);
}

#[test]
fn extract_claude_flag_before_positional_is_preserved() {
    let (args, source) = extract(Provider::Claude, &["--model", "opus", "fix the bug"], false);
    assert_eq!(args, vec!["--model", "opus"]);
    assert_eq!(source, PromptSource::Inline("fix the bug".to_string()));
}

#[test]
fn extract_codex_skips_exec_entrypoint() {
    let (args, source) = extract(Provider::Codex, &["exec", "do it"], false);
    assert_eq!(args, vec!["exec"]);
    assert_eq!(source, PromptSource::Inline("do it".to_string()));
}

#[test]
fn extract_codex_without_exec_still_finds_positional() {
    let (args, source) = extract(Provider::Codex, &["--json", "task"], false);
    assert_eq!(args, vec!["--json"]);
    assert_eq!(source, PromptSource::Inline("task".to_string()));
}

#[test]
fn extract_gemini_long_prompt_flag() {
    let (args, source) = extract(Provider::Gemini, &["--prompt", "hi"], false);
    assert!(args.is_empty());
    assert_eq!(source, PromptSource::Inline("hi".to_string()));
}

#[test]
fn extract_gemini_short_prompt_flag() {
    let (args, source) = extract(Provider::Gemini, &["-p", "hi"], false);
    assert!(args.is_empty());
    assert_eq!(source, PromptSource::Inline("hi".to_string()));
}

#[test]
fn extract_gemini_inline_prompt_flag() {
    let (args, source) = extract(Provider::Gemini, &["--prompt=hi"], false);
    assert!(args.is_empty());
    assert_eq!(source, PromptSource::Inline("hi".to_string()));
}

#[test]
fn extract_gemini_positional_prompt_after_model_flag() {
    let (args, source) = extract(
        Provider::Gemini,
        &["--model", "flash", "explain this"],
        false,
    );
    assert_eq!(args, vec!["--model", "flash"]);
    assert_eq!(source, PromptSource::Inline("explain this".to_string()));
}

#[test]
fn extract_gemini_positional_skips_approval_mode_value() {
    let (args, source) = extract(
        Provider::Gemini,
        &["--approval-mode", "yolo", "explain this"],
        false,
    );
    assert_eq!(args, vec!["--approval-mode", "yolo"]);
    assert_eq!(source, PromptSource::Inline("explain this".to_string()));
}

#[test]
fn extract_goose_text_flag() {
    let (args, source) = extract(Provider::Goose, &["run", "-t", "hello"], false);
    assert_eq!(args, vec!["run"]);
    assert_eq!(source, PromptSource::Inline("hello".to_string()));
}

#[test]
fn extract_kimi_prompt_flag() {
    let (args, source) = extract(Provider::KimiCode, &["--prompt", "hi"], false);
    assert!(args.is_empty());
    assert_eq!(source, PromptSource::Inline("hi".to_string()));
}

#[test]
fn extract_opencode_skips_run_entrypoint() {
    let (args, source) = extract(Provider::OpenCode, &["run", "build it"], false);
    assert_eq!(args, vec!["run"]);
    assert_eq!(source, PromptSource::Inline("build it".to_string()));
}

#[test]
fn extract_qwen_long_prompt_flag() {
    let (args, source) = extract(Provider::QwenCode, &["--prompt", "hi"], false);
    assert!(args.is_empty());
    assert_eq!(source, PromptSource::Inline("hi".to_string()));
}

#[test]
fn extract_flags_only_returns_none_when_no_piped_stdin() {
    let (args, source) = extract(Provider::Codex, &["exec", "--json"], false);
    assert_eq!(args, vec!["exec", "--json"]);
    assert_eq!(source, PromptSource::None);
}

#[test]
fn extract_flags_only_with_piped_stdin_returns_inherit_stdin() {
    let (args, source) = extract(Provider::Codex, &["exec", "--json"], true);
    assert_eq!(args, vec!["exec", "--json"]);
    assert_eq!(source, PromptSource::InheritStdin);
}

#[test]
fn extract_dangling_prompt_flag_returns_error() {
    // Regression test: a prompt flag with no following value must
    // surface as an error rather than silently falling through to the
    // positional / stdin / None branches. Silent fall-through is the
    // original DRY-providers bug this refactor exists to prevent.
    let args: Vec<String> = vec!["--prompt".to_string()];
    let err = extract_prompt_source_from_passthrough(profile(Provider::Gemini), &args, false)
        .expect_err("dangling --prompt must return an error");
    let message = err.to_string();
    assert!(
        message.contains("--prompt"),
        "error message should mention the flag: {message}"
    );
    assert!(
        message.contains("requires a value"),
        "error message should mention missing value: {message}"
    );
}

#[test]
fn extract_positional_with_equals_is_not_mistaken_for_flag() {
    // Regression test: a positional argument containing `=` (e.g.
    // an env-var-style token like `KEY=VALUE`) must not be mistaken
    // for a value-taking flag by `find_positional_prompt_index`.
    // `KEY` is not in the known value-taking flag list, so `KEY=VALUE`
    // must be treated as the first positional prompt (not skipped), and
    // the second positional remains in args.
    let (args, source) = extract(Provider::Claude, &["KEY=VALUE", "the actual prompt"], false);
    assert_eq!(args, vec!["the actual prompt".to_string()]);
    assert_eq!(source, PromptSource::Inline("KEY=VALUE".to_string()));
}

// -- require_prompt_present tests -----------------------------------------

#[test]
fn require_prompt_present_passes_in_interactive_mode_with_no_source() {
    require_prompt_present("claude", false, &PromptSource::None).unwrap();
}

#[test]
fn require_prompt_present_passes_with_inline_prompt() {
    require_prompt_present("claude", true, &PromptSource::Inline("x".to_string())).unwrap();
}

#[test]
fn require_prompt_present_passes_with_inherit_stdin() {
    require_prompt_present("claude", true, &PromptSource::InheritStdin).unwrap();
}

#[test]
fn require_prompt_present_fails_non_interactive_with_no_source() {
    let err = require_prompt_present("codex", true, &PromptSource::None).unwrap_err();
    let message = err.to_string();
    assert!(message.contains("codex"));
    assert!(message.contains("requires a prompt"));
}

// -- Pipeline Order Regression Tests (Issue 2026-04-15) -----------------

fn run_direct_wrap_pipeline_simulation(
    provider: Provider,
    cli_args: &[&str],
    prompt: &str,
) -> Vec<String> {
    let profile = profile(provider);
    let mut child_args: Vec<String> = cli_args.iter().map(|s| s.to_string()).collect();
    let mut env_overrides: Vec<(String, String)> = Vec::new();

    // 1. apply_yolo
    let _ = profile.apply_yolo_for_mode(&mut child_args, &mut env_overrides, false);
    // 2. apply_entrypoint
    profile.apply_entrypoint(&mut child_args, true);
    // 3. apply_non_interactive
    let _ = profile.apply_non_interactive_flags(&mut child_args);

    // 4. Model resolution (specifically OpenCode simulation)
    if provider == Provider::OpenCode {
        let snapshot = OpenCodeEnvSnapshot {
            opencode_model_env: None,
            opencode_config_model: None,
        };
        let _ = apply_opencode_model_resolution(
            &mut child_args,
            &mut |k, v| env_overrides.push((k, v)),
            false,
            Some("test-model"),
            true,
            &snapshot,
        );
    }

    // 5. Output format (simulation of --format stream-json)
    let _ = profile.apply_output_format(&mut child_args, OutputFormat::Stream);

    // 6. apply_structured_stream
    profile.apply_structured_stream(&mut child_args);

    // 7. prompt_delivery (NEW CORRECT ORDER)
    let _ = profile
        .prompt_delivery(&child_args, prompt, true)
        .unwrap()
        .apply_to(&mut child_args);

    child_args
}

#[test]
fn test_opencode_non_interactive_args_order() {
    let args = run_direct_wrap_pipeline_simulation(Provider::OpenCode, &[], "do the thing");

    // We want to verify that flags appear before any positional arguments,
    // specifically before the `--` separator if there is one.
    if let Some(pos) = args.iter().position(|a| a == "--") {
        for arg in args.iter().skip(pos + 2) {
            assert!(
                !arg.starts_with('-'),
                "Flag {:?} appears after -- separator in argv: {:?}",
                arg,
                args
            );
        }
    } else {
        // No separator, check the end
    }
}

#[test]
fn test_goose_non_interactive_no_duplicate_run() {
    let args = run_direct_wrap_pipeline_simulation(Provider::Goose, &[], "run this");
    let run_count = args.iter().filter(|a| *a == "run").count();
    assert_eq!(
        run_count, 1,
        "Goose pipeline should contain exactly one 'run' entrypoint, found: {:?}",
        args
    );
}

#[test]
fn test_all_providers_flags_before_double_dash() {
    for provider in [
        Provider::Claude,
        Provider::Codex,
        Provider::Gemini,
        Provider::KimiCode,
        Provider::QwenCode,
        Provider::OpenCode,
        Provider::Goose,
    ] {
        let args = run_direct_wrap_pipeline_simulation(
            provider,
            &[],
            "some generic prompt --with-flag",
        );
        if let Some(pos) = args.iter().position(|a| a == "--") {
            for arg in &args[pos + 1..] {
                if arg != "some generic prompt --with-flag" {
                    assert!(
                        !arg.starts_with('-'),
                        "[{:?}] Flag {:?} appears after -- separator in argv: {:?}",
                        provider,
                        arg,
                        args
                    );
                }
            }
        }
    }
}
