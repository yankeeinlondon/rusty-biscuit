use super::*;

fn argv(tokens: &[&str]) -> Vec<String> {
    tokens.iter().map(|s| s.to_string()).collect()
}

#[test]
fn classifier_root_slot_at_position_1_empty() {
    let a = argv(&["claudine", ""]);
    assert_eq!(
        classify_completion_target(&a, 1),
        CompletionTarget::Root(RootPartial::Empty),
    );
}

#[test]
fn classifier_root_slot_at_position_1_partial_word() {
    let a = argv(&["claudine", "com"]);
    assert_eq!(
        classify_completion_target(&a, 1),
        CompletionTarget::Root(RootPartial::Word("com".to_string())),
    );
}

#[test]
fn classifier_root_slot_with_flag_partial_single_dash() {
    let a = argv(&["claudine", "-"]);
    assert_eq!(
        classify_completion_target(&a, 1),
        CompletionTarget::Root(RootPartial::FlagLike("-".to_string())),
    );
}

#[test]
fn classifier_root_slot_with_flag_partial_long_prefix() {
    let a = argv(&["claudine", "--h"]);
    assert_eq!(
        classify_completion_target(&a, 1),
        CompletionTarget::Root(RootPartial::FlagLike("--h".to_string())),
    );
}

#[test]
fn classifier_root_slot_after_global_plain_flag() {
    let a = argv(&["claudine", "--plain", ""]);
    assert_eq!(
        classify_completion_target(&a, 2),
        CompletionTarget::Root(RootPartial::Empty),
    );
}

#[test]
fn classifier_root_slot_after_global_verbose_flag() {
    let a = argv(&["claudine", "-v", "c"]);
    assert_eq!(
        classify_completion_target(&a, 2),
        CompletionTarget::Root(RootPartial::Word("c".to_string())),
    );
}

#[test]
fn classifier_root_slot_after_global_debug_with_value() {
    let a = argv(&["claudine", "--debug", "trace", ""]);
    assert_eq!(
        classify_completion_target(&a, 3),
        CompletionTarget::Root(RootPartial::Empty),
    );
}

#[test]
fn classifier_root_slot_after_debug_equals_form() {
    let a = argv(&["claudine", "--debug=info", ""]);
    assert_eq!(
        classify_completion_target(&a, 2),
        CompletionTarget::Root(RootPartial::Empty),
    );
}

#[test]
fn classifier_composition_flag_slot_routes_to_provider_flag() {
    let a = argv(&["claudine", "compose", "--ki"]);
    assert_eq!(
        classify_completion_target(&a, 2),
        CompletionTarget::CompositionProviderFlag {
            partial: "--ki".to_string(),
        },
    );
}

#[test]
fn composition_provider_flag_completer_is_catalog_derived() {
    // Every provider's switch is offered — including kilo/pi/antigravity,
    // which have no dedicated clap boolean field — so the derived surface
    // never drifts as providers are added.
    let a = argv(&["claudine", "compose", "--"]);
    let got = run_composition_provider_flag("--", &a, 2);
    for flag in ["--claude", "--kilo", "--pi", "--antigravity"] {
        assert!(got.iter().any(|c| c == flag), "{flag} missing: {got:?}");
    }
    // Prefix filtering keeps only matching switches. (argv[current_index]
    // must equal the partial — the clap merge reads the token from argv.)
    let narrowed_argv = argv(&["claudine", "compose", "--kil"]);
    let narrowed = run_composition_provider_flag("--kil", &narrowed_argv, 2);
    assert!(narrowed.iter().any(|c| c == "--kilo"));
    assert!(!narrowed.iter().any(|c| c == "--claude"));
}

#[test]
fn classifier_composition_positional_on_compose() {
    let a = argv(&["claudine", "compose", ""]);
    assert_eq!(
        classify_completion_target(&a, 2),
        CompletionTarget::CompositionPositional {
            mode: ComposeMode::Compose,
            partial: String::new(),
        },
    );
}

#[test]
fn classifier_composition_positional_on_inline_compose_with_partial() {
    let a = argv(&["claudine", "inline-compose", "pl"]);
    assert_eq!(
        classify_completion_target(&a, 2),
        CompletionTarget::CompositionPositional {
            mode: ComposeMode::InlineCompose,
            partial: "pl".to_string(),
        },
    );
}

#[test]
fn classifier_composition_positional_on_sequence() {
    let a = argv(&["claudine", "sequence", "@s"]);
    assert_eq!(
        classify_completion_target(&a, 2),
        CompletionTarget::CompositionPositional {
            mode: ComposeMode::Sequence,
            partial: "@s".to_string(),
        },
    );
}

#[test]
fn classifier_setter_name_after_first_positional_is_committed() {
    // Schemas Phase 4: once the file slot is committed, a plain
    // identifier-shaped token at the cursor is treated as a setter-
    // name partial. The schema-aware completer downstream decides
    // whether the name maps to a schema property; the classifier is
    // happy as long as the shape and file_arg are right.
    let a = argv(&["claudine", "compose", "plan.md", "more"]);
    assert_eq!(
        classify_completion_target(&a, 3),
        CompletionTarget::SetterName {
            partial: "more".to_string(),
            file_arg: "plan.md".to_string(),
        },
    );
}

#[test]
fn classifier_other_when_second_positional_has_non_identifier_chars() {
    // Tokens that look like paths or contain unsupported characters
    // are not setter-name partials and still route to `Other`.
    let a = argv(&["claudine", "compose", "plan.md", "more/stuff"]);
    assert_eq!(classify_completion_target(&a, 3), CompletionTarget::Other);
}

#[test]
fn classifier_setter_value_when_cursor_is_setter_shaped() {
    let a = argv(&["claudine", "compose", "key=val"]);
    assert_eq!(
        classify_completion_target(&a, 2),
        CompletionTarget::SetterValue {
            token: "key=val".to_string(),
            file_arg: None,
        },
    );
}

#[test]
fn classifier_setter_value_after_committed_positional() {
    // Phase 4 (improved-shell-completions) contract: a setter-shaped
    // cursor wins even when the file slot has already been filled.
    // The classifier must not return `Other` just because a positional
    // was observed earlier — and the schemas-Phase-4 work now also
    // surfaces the committed file as `file_arg` so the schema-aware
    // value completer can use it.
    let a = argv(&["claudine", "compose", "foo.md", "spec=@s"]);
    assert_eq!(
        classify_completion_target(&a, 3),
        CompletionTarget::SetterValue {
            token: "spec=@s".to_string(),
            file_arg: Some("foo.md".to_string()),
        },
    );
}

#[test]
fn classifier_setter_value_on_inline_compose() {
    let a = argv(&["claudine", "inline-compose", "foo.md", "ref=@d"]);
    assert_eq!(
        classify_completion_target(&a, 3),
        CompletionTarget::SetterValue {
            token: "ref=@d".to_string(),
            file_arg: Some("foo.md".to_string()),
        },
    );
}

#[test]
fn classifier_setter_value_on_sequence() {
    let a = argv(&["claudine", "sequence", "foo.md", "plan=@p"]);
    assert_eq!(
        classify_completion_target(&a, 3),
        CompletionTarget::SetterValue {
            token: "plan=@p".to_string(),
            file_arg: Some("foo.md".to_string()),
        },
    );
}

#[test]
fn classifier_other_on_setter_shaped_cursor_for_non_composition_sub() {
    // Wrappers and admin subcommands never route through the setter
    // classifier — they fall through to the legacy supplement.
    let a = argv(&["claudine", "claude", "spec=@s"]);
    assert_eq!(classify_completion_target(&a, 2), CompletionTarget::Other);
}

#[test]
fn classifier_skips_setter_earlier_still_reaches_positional() {
    // Earlier setter isn't a positional; cursor is still the first
    // positional for the composition completer.
    let a = argv(&["claudine", "compose", "key=val", "@plan"]);
    assert_eq!(
        classify_completion_target(&a, 3),
        CompletionTarget::CompositionPositional {
            mode: ComposeMode::Compose,
            partial: "@plan".to_string(),
        },
    );
}

#[test]
fn classifier_skips_value_of_preceding_value_bearing_flag() {
    let a = argv(&["claudine", "compose", "--model", "gpt-4", ""]);
    assert_eq!(
        classify_completion_target(&a, 4),
        CompletionTarget::CompositionPositional {
            mode: ComposeMode::Compose,
            partial: String::new(),
        },
    );
}

#[test]
fn classifier_other_on_wrapper_subcommand() {
    let a = argv(&["claudine", "claude", ""]);
    assert_eq!(classify_completion_target(&a, 2), CompletionTarget::Other);
}

#[test]
fn classifier_other_on_non_composition_subcommand() {
    let a = argv(&["claudine", "hooks", ""]);
    assert_eq!(classify_completion_target(&a, 2), CompletionTarget::Other);
}

#[test]
fn classifier_other_when_cursor_crosses_double_dash_separator() {
    let a = argv(&["claudine", "claude", "--", "--model"]);
    assert_eq!(classify_completion_target(&a, 3), CompletionTarget::Other,);
}

#[test]
fn classifier_other_when_current_index_is_zero() {
    let a = argv(&["claudine", ""]);
    assert_eq!(classify_completion_target(&a, 0), CompletionTarget::Other,);
}

#[test]
fn classifier_other_when_debug_value_slot_is_the_cursor() {
    // `claudine --debug <TAB>` — the cursor sits on the value slot of
    // `--debug`, not on the root subcommand slot. We treat this as
    // Other so the (currently no-op) value-slot completer handles it.
    let a = argv(&["claudine", "--debug", ""]);
    assert_eq!(classify_completion_target(&a, 2), CompletionTarget::Other,);
}

#[test]
fn classify_root_partial_detects_empty_and_word_and_flag() {
    assert_eq!(classify_root_partial(""), RootPartial::Empty);
    assert_eq!(
        classify_root_partial("com"),
        RootPartial::Word("com".to_string())
    );
    assert_eq!(
        classify_root_partial("-h"),
        RootPartial::FlagLike("-h".to_string())
    );
    assert_eq!(
        classify_root_partial("--help"),
        RootPartial::FlagLike("--help".to_string())
    );
}

#[test]
fn user_config_exists_detects_json_file() {
    let tmp = tempfile::TempDir::new().unwrap();
    let dot_claudine = tmp.path().join(".claudine");
    std::fs::create_dir_all(&dot_claudine).unwrap();
    std::fs::write(dot_claudine.join("config.json"), "{}").unwrap();
    assert!(user_config_exists(Some(tmp.path())));
}

#[test]
fn user_config_exists_detects_json5_file() {
    let tmp = tempfile::TempDir::new().unwrap();
    let dot_claudine = tmp.path().join(".claudine");
    std::fs::create_dir_all(&dot_claudine).unwrap();
    std::fs::write(dot_claudine.join("config.json5"), "{}").unwrap();
    assert!(user_config_exists(Some(tmp.path())));
}

#[test]
fn user_config_exists_returns_false_when_absent() {
    let tmp = tempfile::TempDir::new().unwrap();
    assert!(!user_config_exists(Some(tmp.path())));
}

#[test]
fn user_config_exists_returns_false_when_home_missing() {
    assert!(!user_config_exists(None));
}

#[test]
fn detect_repo_config_finds_repo_and_config() {
    let tmp = tempfile::TempDir::new().unwrap();
    std::fs::create_dir_all(tmp.path().join(".git")).unwrap();
    std::fs::create_dir_all(tmp.path().join(".claudine")).unwrap();
    std::fs::write(tmp.path().join(".claudine").join("config.json"), "{}").unwrap();
    let (has_cfg, in_repo) = detect_repo_config(tmp.path());
    assert!(in_repo);
    assert!(has_cfg);
}

#[test]
fn detect_repo_config_finds_repo_without_config() {
    let tmp = tempfile::TempDir::new().unwrap();
    std::fs::create_dir_all(tmp.path().join(".git")).unwrap();
    let (has_cfg, in_repo) = detect_repo_config(tmp.path());
    assert!(in_repo);
    assert!(!has_cfg);
}

#[test]
fn detect_repo_config_handles_no_repo() {
    let tmp = tempfile::TempDir::new().unwrap();
    let (has_cfg, in_repo) = detect_repo_config(tmp.path());
    assert!(!in_repo);
    assert!(!has_cfg);
}

// -- end-to-end dispatch --------------------------------------------

fn test_ctx(user: bool, repo: bool, in_repo: bool) -> RootContext {
    RootContext {
        user_config_exists: user,
        repo_config_exists: repo,
        in_repo,
    }
}

#[test]
fn run_with_context_emits_full_root_menu_on_bare_tab() {
    let a = argv(&["claudine", ""]);
    let got = run_with_context(&a, 1, &test_ctx(true, true, true));
    assert_eq!(got.first().map(String::as_str), Some("compose"));
    assert!(got.contains(&"config".to_string()));
    assert!(!got.contains(&"init".to_string()));
}

#[test]
fn run_with_context_includes_init_when_no_configs() {
    let a = argv(&["claudine", ""]);
    let got = run_with_context(&a, 1, &test_ctx(false, false, false));
    assert!(got.contains(&"init".to_string()));
}

#[test]
fn run_with_context_filters_by_word_prefix() {
    let a = argv(&["claudine", "com"]);
    let got = run_with_context(&a, 1, &test_ctx(true, true, true));
    assert_eq!(got, vec!["compose", "commands", "completions"]);
}

#[test]
fn run_with_context_emits_help_only_for_flag_partial() {
    let a = argv(&["claudine", "--h"]);
    let got = run_with_context(&a, 1, &test_ctx(true, true, true));
    assert_eq!(got, vec!["--help"]);
}

#[test]
fn run_with_context_handles_global_flag_before_cursor() {
    let a = argv(&["claudine", "--plain", "c"]);
    let got = run_with_context(&a, 2, &test_ctx(true, true, true));
    assert!(got.iter().any(|c| c == "compose"));
    assert!(got.iter().any(|c| c == "commands"));
}

#[test]
fn clap_fallback_completes_providers_flags() {
    let a = argv(&["claudine", "providers", "--des"]);
    let got = run_with_context(&a, 2, &test_ctx(true, true, true));
    assert!(
        got.iter().any(|c| c == "--describe"),
        "expected --describe in candidates, got {got:?}"
    );
}

#[test]
fn clap_fallback_completes_providers_format_flag() {
    let a = argv(&["claudine", "providers", "--format"]);
    let got = run_with_context(&a, 2, &test_ctx(true, true, true));
    assert!(
        got.iter().any(|c| c == "--format"),
        "expected --format in candidates, got {got:?}"
    );
}

// -- SetterName classification (schemas Phase 4) -----------------------

#[test]
fn classifier_setter_name_on_inline_compose() {
    let a = argv(&["claudine", "inline-compose", "plan.md", "tit"]);
    assert_eq!(
        classify_completion_target(&a, 3),
        CompletionTarget::SetterName {
            partial: "tit".to_string(),
            file_arg: "plan.md".to_string(),
        },
    );
}

#[test]
fn classifier_setter_name_on_sequence() {
    let a = argv(&["claudine", "sequence", "plan.md", "stat"]);
    assert_eq!(
        classify_completion_target(&a, 3),
        CompletionTarget::SetterName {
            partial: "stat".to_string(),
            file_arg: "plan.md".to_string(),
        },
    );
}

#[test]
fn classifier_setter_name_with_empty_cursor_after_committed_file() {
    // Empty cursor token after `plan.md spec=foo <TAB>` is treated as
    // an unfiltered setter-name lookup — show every (unsupplied)
    // property.
    let a = argv(&["claudine", "compose", "plan.md", "spec=foo", ""]);
    // Empty token has no shape, so `is_setter_name_partial` returns
    // false: classifier returns Other (the value slot is fully filled).
    assert_eq!(classify_completion_target(&a, 4), CompletionTarget::Other);
}

#[test]
fn classifier_setter_name_only_when_file_arg_present() {
    // Without a committed file_arg, an identifier-shaped cursor stays
    // at the composition positional slot — there's no schema to load
    // yet.
    let a = argv(&["claudine", "compose", "tit"]);
    assert_eq!(
        classify_completion_target(&a, 2),
        CompletionTarget::CompositionPositional {
            mode: ComposeMode::Compose,
            partial: "tit".to_string(),
        },
    );
}

#[test]
fn is_setter_name_partial_accepts_identifiers() {
    assert!(is_setter_name_partial("foo"));
    assert!(is_setter_name_partial("_bar"));
    assert!(is_setter_name_partial("a1-b2"));
    assert!(is_setter_name_partial("snake_case"));
}

#[test]
fn is_setter_name_partial_rejects_non_identifier_shapes() {
    assert!(!is_setter_name_partial(""));
    assert!(!is_setter_name_partial("1abc"));
    assert!(!is_setter_name_partial("foo/bar"));
    assert!(!is_setter_name_partial("foo=bar"));
    assert!(!is_setter_name_partial("foo.bar"));
}

#[test]
fn scan_committed_positional_finds_file_arg_among_setters_and_flags() {
    let a = argv(&[
        "claudine", "compose", "--model", "gpt-4", "spec=val", "plan.md", "",
    ]);
    let (file, seen) = scan_committed_positional(&a, 2, 6);
    assert_eq!(file.as_deref(), Some("plan.md"));
    assert!(seen);
}

#[test]
fn scan_committed_positional_returns_none_when_no_positional() {
    let a = argv(&["claudine", "compose", "spec=val", "--model", "gpt", ""]);
    let (file, seen) = scan_committed_positional(&a, 2, 5);
    assert!(file.is_none());
    assert!(!seen);
}

#[test]
fn collect_supplied_setter_names_skips_cursor_token() {
    let a = argv(&["claudine", "compose", "plan.md", "title=hi", "des"]);
    let got = collect_supplied_setter_names(&a, 4);
    assert!(got.contains("title"));
    assert!(!got.contains("des"));
}

#[test]
fn split_setter_handles_typical_cases() {
    assert_eq!(split_setter("title=hi"), Some(("title", "hi")));
    assert_eq!(split_setter("a_b-c=v=w"), Some(("a_b-c", "v=w")));
    assert!(split_setter("=missing-name").is_none());
    assert!(split_setter("no-equals").is_none());
    assert!(split_setter("9bad=v").is_none());
}

#[test]
fn clap_fallback_completes_providers_format_values() {
    let a = argv(&["claudine", "providers", "--format", ""]);
    let got = run_with_context(&a, 3, &test_ctx(true, true, true));
    assert!(
        got.iter().any(|c| c == "text"),
        "expected 'text' format value, got {got:?}"
    );
    assert!(
        got.iter().any(|c| c == "json"),
        "expected 'json' format value, got {got:?}"
    );
}
