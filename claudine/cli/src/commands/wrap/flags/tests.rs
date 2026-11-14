use super::*;

#[test]
fn native_output_codex_json_flag_is_detected_from_catalog() {
    assert!(has_explicit_native_output_request(
        Provider::Codex,
        &string_args(&["exec", "--json"])
    ));
}

#[test]
fn native_output_output_format_values_are_detected_from_catalog() {
    for provider in [Provider::Claude, Provider::Gemini, Provider::QwenCode] {
        assert!(
            has_explicit_native_output_request(
                provider,
                &string_args(&["--output-format", "json"])
            ),
            "{provider:?} should detect separated json output format"
        );
        assert!(
            has_explicit_native_output_request(
                provider,
                &string_args(&["--output-format=stream-json"])
            ),
            "{provider:?} should detect inline stream-json output format"
        );
    }
}

#[test]
fn native_output_opencode_format_json_is_detected_from_catalog() {
    assert!(has_explicit_native_output_request(
        Provider::OpenCode,
        &string_args(&["run", "--format", "json"])
    ));
}

#[test]
fn native_output_kimi_wire_transport_is_not_user_output_request() {
    assert!(!has_explicit_native_output_request(
        Provider::KimiCode,
        &string_args(&["--wire"])
    ));
}

#[test]
fn native_output_unknown_provider_flags_do_not_match_catalog() {
    assert!(!has_explicit_native_output_request(
        Provider::Codex,
        &string_args(&["--output-format", "json"])
    ));
    assert!(!has_explicit_native_output_request(
        Provider::OpenCode,
        &string_args(&["json"])
    ));
    assert!(!has_explicit_native_output_request(
        Provider::KimiCode,
        &string_args(&["--print"])
    ));
}

#[test]
fn native_output_detection_matches_provider_catalog() {
    for provider in claudine::provider::PROVIDERS_DISPLAY_ORDER {
        for format in provider_info(provider).output_formats {
            let args = match format.selector {
                OutputFormatSelector::Default | OutputFormatSelector::TransportFlag { .. } => {
                    continue;
                }
                OutputFormatSelector::Flag { flag } => string_args(&[flag]),
                OutputFormatSelector::FlagValue { flag } => {
                    vec![flag.to_string(), format.native_name.to_string()]
                }
                OutputFormatSelector::Positional { token } => string_args(&[token]),
            };

            assert!(
                has_explicit_native_output_request(provider, &args),
                "{provider:?} should detect catalog selector {:?} for {}",
                format.selector,
                format.native_name
            );
        }
    }
}

#[test]
fn native_output_detection_source_has_no_provider_branch() {
    let source = include_str!("../flags.rs");
    let start = source
        .find("fn has_explicit_native_output_request")
        .expect("helper should exist");
    let end = source[start..]
        .find("\nfn ")
        .or_else(|| source[start..].find("\npub(crate) fn "))
        .or_else(|| source[start..].find("\n#[allow"))
        .map(|offset| start + offset)
        .expect("helper should be followed by another fn definition");
    let helper_source = &source[start..end];

    assert!(
        !helper_source.contains("match provider")
            && !helper_source.contains("Provider::Claude")
            && !helper_source.contains("Provider::Codex")
            && !helper_source.contains("Provider::Gemini")
            && !helper_source.contains("Provider::KimiCode")
            && !helper_source.contains("Provider::OpenCode")
            && !helper_source.contains("Provider::QwenCode")
            && !helper_source.contains("Provider::Goose")
            && helper_source.contains("provider_info(provider)")
            && helper_source.contains(".output_formats"),
        "native-output detection must remain catalog-derived: {helper_source}"
    );
}

/// Parse `WrapperArgs` through a clap probe, exactly as the wrapper
/// subcommands flatten it.
fn try_parse_wrapper_args(extra: &[&str]) -> Result<WrapperArgs, clap::Error> {
    use clap::Parser;

    // `WrapperArgs` declares its own `help` field.
    #[derive(Debug, clap::Parser)]
    #[command(disable_help_flag = true)]
    struct Probe {
        #[command(flatten)]
        args: WrapperArgs,
    }

    let mut argv = vec!["probe"];
    argv.extend_from_slice(extra);
    Probe::try_parse_from(argv).map(|probe| probe.args)
}

/// `extra` after `claudine claude`, with the wrapper options hoisted as the
/// executable hoists them, then parsed.
fn parse_hoisted(extra: &[&str]) -> WrapperArgs {
    let mut argv: Vec<std::ffi::OsString> = vec!["claudine".into(), "claude".into()];
    argv.extend(extra.iter().map(std::ffi::OsString::from));
    let hoisted: Vec<String> = crate::argv::hoist_wrapper_options(argv)
        .unwrap()
        .into_iter()
        .map(|token| token.into_string().unwrap())
        .collect();
    let tokens: Vec<&str> = hoisted[2..].iter().map(String::as_str).collect();
    try_parse_wrapper_args(&tokens).unwrap()
}

#[test]
fn wrapper_args_accept_edit_with_interactive_in_every_spelling() {
    for argv in [
        &["--edit", "--interactive"][..],
        &["--edit", "-i"][..],
        &["-i", "--edit"][..],
        &["--interactive", "--edit", "review this repository"][..],
    ] {
        let args = try_parse_wrapper_args(argv)
            .unwrap_or_else(|err| panic!("{argv:?} must parse, got: {err}"));
        assert!(args.edit, "{argv:?} must set edit");
        assert!(args.interactive, "{argv:?} must set interactive");
    }
}

#[test]
fn options_after_the_prompt_mean_what_they_mean_before_it() {
    let after = parse_hoisted(&["review this repository", "--edit", "-i", "-m", "opus", "--op=commit", "-o", "json"]);
    let before = parse_hoisted(&["--edit", "-i", "-m", "opus", "--op=commit", "-o", "json", "review this repository"]);

    for args in [&after, &before] {
        assert!(args.edit && args.interactive);
        assert_eq!(args.model.as_deref(), Some("opus"));
        assert_eq!(args.operation.as_deref(), Some("commit"));
        assert_eq!(args.output.as_deref(), Some("json"));
        assert_eq!(args.passthrough, vec!["review this repository"]);
    }
}

#[test]
fn old_non_interactive_flags_pass_through_to_provider() {
    let args = parse_hoisted(&["-n", "--non-interactive", "--ni", "task"]);

    assert!(!args.interactive);
    assert_eq!(args.passthrough, vec!["-n", "--non-interactive", "--ni", "task"]);
}

#[test]
fn the_separator_after_the_prompt_is_consumed_and_the_tail_kept() {
    // User typed: claudine claude prompt -- --silent -y
    //
    // clap collects the tail verbatim because `trailing_var_arg` began
    // capturing at `prompt`, so the passthrough literally contains `--`.
    // The `--` is Claudine's boundary, so it is not forwarded.
    let mut args = string_args(&["prompt", "--", "--silent", "-y"]);

    let boundary = consume_passthrough_separator_at(&mut args, Some(DashBoundary::Literal(1)));

    assert_eq!(boundary, Some(1));
    assert_eq!(args, vec!["prompt", "--silent", "-y"]);
}

#[test]
fn a_separator_clap_consumed_leaves_the_passthrough_unchanged() {
    // User typed: claudine claude -- prompt --silent
    let mut args = string_args(&["prompt", "--silent"]);
    let raw = string_args(&["claudine", "claude", "--", "prompt", "--silent"]);

    let boundary = consume_passthrough_separator_with_raw(&mut args, &raw);

    assert_eq!(boundary, Some(0));
    assert_eq!(args, vec!["prompt", "--silent"]);
}

#[test]
fn a_tail_after_the_separator_is_never_read_as_a_wrapper_option() {
    // User typed: claudine codex prompt -- --edit --yolo --output json --help
    let args = parse_hoisted(&["prompt", "--", "--edit", "--yolo", "--output", "json", "--help"]);
    assert!(!args.edit && !args.yolo && !args.help);
    assert_eq!(args.output, None);

    let mut passthrough = args.passthrough;
    let raw = string_args(&["claudine", "codex", "prompt", "--", "--edit", "--yolo", "--output", "json", "--help"]);
    consume_passthrough_separator_with_raw(&mut passthrough, &raw);
    assert_eq!(passthrough, vec!["prompt", "--edit", "--yolo", "--output", "json", "--help"]);
}

#[test]
fn only_the_user_separator_is_consumed() {
    // User typed: claudine pi -i hello -- --offline -- "- item"
    //
    // The first `--` is Claudine's boundary and is dropped; the second belongs
    // to the provider and is forwarded with the rest of the tail.
    let mut args = string_args(&["hello", "--", "--offline", "--", "- item"]);
    let raw = string_args(&["claudine", "pi", "-i", "hello", "--", "--offline", "--", "- item"]);

    consume_passthrough_separator_with_raw(&mut args, &raw);

    assert_eq!(args, vec!["hello", "--offline", "--", "- item"]);
}

#[test]
fn a_provider_separator_after_a_consumed_one_is_kept() {
    // User typed: claudine pi -- hello -- --offline
    //
    // clap consumed the first `--`, so the literal one in the passthrough is
    // the provider's own separator and must survive.
    let mut args = string_args(&["hello", "--", "--offline"]);
    let raw = string_args(&["claudine", "pi", "--", "hello", "--", "--offline"]);

    consume_passthrough_separator_with_raw(&mut args, &raw);

    assert_eq!(args, vec!["hello", "--", "--offline"]);
}

#[test]
fn find_passthrough_dash_boundary_detects_literal_separator() {
    let passthrough = vec!["prompt".to_string(), "--".to_string(), "rest".to_string()];
    let raw = vec![
        "claudine".to_string(),
        "claude".to_string(),
        "prompt".to_string(),
        "--".to_string(),
        "rest".to_string(),
    ];

    assert_eq!(
        find_passthrough_dash_boundary_with_raw(&passthrough, &raw),
        Some(DashBoundary::Literal(1))
    );
}

#[test]
fn find_passthrough_dash_boundary_uses_raw_tail_when_clap_strips_dash() {
    let passthrough = vec!["prompt".to_string(), "--silent".to_string()];
    let raw = vec![
        "claudine".to_string(),
        "claude".to_string(),
        "--".to_string(),
        "prompt".to_string(),
        "--silent".to_string(),
    ];

    assert_eq!(
        find_passthrough_dash_boundary_with_raw(&passthrough, &raw),
        Some(DashBoundary::Consumed(0))
    );
}

#[test]
fn find_passthrough_dash_boundary_returns_none_without_dash() {
    let passthrough = vec!["prompt".to_string()];
    let raw = vec![
        "claudine".to_string(),
        "claude".to_string(),
        "prompt".to_string(),
    ];

    assert_eq!(
        find_passthrough_dash_boundary_with_raw(&passthrough, &raw),
        None
    );
}

#[test]
fn model_value_from_args_supports_short_and_long_forms() {
    let long_inline = vec!["--model=foo".to_string()];
    let short_next = vec!["-m".to_string(), "bar".to_string()];

    assert_eq!(model_value_from_args(&long_inline), Some("foo".to_string()));
    assert_eq!(model_value_from_args(&short_next), Some("bar".to_string()));
}

fn string_args(args: &[&str]) -> Vec<String> {
    args.iter().map(|arg| (*arg).to_string()).collect()
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn proptest_wrapper_options_anywhere_are_claudine_s_and_others_keep_order(
            tokens in prop::collection::vec(
                prop_oneof!["-y|--yolo|-i|--interactive|--edit|-q|--quiet", "[a-z0-9]+"],
                0..12,
            )
        ) {
            // clap refuses a repeated option, so each family appears at most once.
            let family = |token: &str| token.trim_start_matches('-').chars().next();
            let mut seen = std::collections::HashSet::new();
            let tokens: Vec<String> = tokens
                .into_iter()
                .filter(|token| !token.starts_with('-') || seen.insert(family(token)))
                .collect();
            let args = parse_hoisted(&tokens.iter().map(String::as_str).collect::<Vec<_>>());

            let others: Vec<String> = tokens.iter().filter(|t| !t.starts_with('-')).cloned().collect();
            prop_assert_eq!(&args.passthrough, &others);
            prop_assert_eq!(args.yolo, tokens.iter().any(|t| t == "-y" || t == "--yolo"));
            prop_assert_eq!(args.interactive, tokens.iter().any(|t| t == "-i" || t == "--interactive"));
            prop_assert_eq!(args.edit, tokens.iter().any(|t| t == "--edit"));
            prop_assert_eq!(args.quiet, tokens.iter().any(|t| t == "-q" || t == "--quiet"));
        }
    }
}

#[test]
fn the_consumed_separator_reports_where_the_tail_starts() {
    // User typed: claudine codex -y -c x=y prompt -- --native z
    // (`-y` is clap's; `-c x=y` and the prompt are the passthrough.)
    let mut args = string_args(&["-c", "x=y", "prompt", "--", "--native", "z"]);
    assert_eq!(consume_passthrough_separator_at(&mut args, Some(DashBoundary::Literal(3))), Some(3));
    assert_eq!(args, string_args(&["-c", "x=y", "prompt", "--native", "z"]));

    // clap consumed the `--` itself.
    let mut args = string_args(&["-c", "x", "--native"]);
    assert_eq!(consume_passthrough_separator_at(&mut args, Some(DashBoundary::Consumed(2))), Some(2));
    assert_eq!(args, string_args(&["-c", "x", "--native"]));

    let mut args = string_args(&["-c", "x"]);
    assert_eq!(consume_passthrough_separator_at(&mut args, None), None);
}

#[test]
fn passthrough_provider_tail_splits_at_the_boundary() {
    let args = string_args(&["-c", "x=y", "--native", "z"]);

    let mixed = passthrough_provider_tail(&args, Some(2));
    assert_eq!(mixed.launch_args(), args.as_slice());
    assert_eq!(mixed.implicit_args(), string_args(&["-c", "x=y"]));
    assert_eq!(mixed.opaque_args().unwrap(), string_args(&["--native", "z"]));

    let implicit = passthrough_provider_tail(&args, None);
    assert_eq!(implicit.boundary(), None);
    assert_eq!(implicit.launch_args(), args.as_slice());

    // A boundary past the end (every token after `--` was the prompt)
    // leaves an authored, empty suffix.
    let empty_suffix = passthrough_provider_tail(&args, Some(9));
    assert_eq!(empty_suffix.boundary(), Some(4));
    assert_eq!(empty_suffix.opaque_args().unwrap(), [] as [String; 0]);
}
