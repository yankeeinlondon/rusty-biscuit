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

#[test]
fn extract_wrapper_flags_lifts_reserved_aliases_from_passthrough() {
    let mut args = vec![
        "--json".to_string(),
        "-i".to_string(),
        "task".to_string(),
        "-y".to_string(),
    ];

    let extracted = extract_wrapper_flags_from_passthrough(&mut args).unwrap();

    assert!(extracted.yolo);
    assert!(extracted.interactive);
    assert_eq!(args, vec!["--json", "task"]);
}

#[test]
fn extract_wrapper_flags_lifts_interactive_long_form() {
    let mut args = vec!["--interactive".to_string(), "do something".to_string()];

    let extracted = extract_wrapper_flags_from_passthrough(&mut args).unwrap();

    assert!(extracted.interactive);
    assert_eq!(args, vec!["do something"]);
}

#[test]
fn extract_wrapper_flags_lifts_edit_long_form() {
    let mut args = vec!["do something".to_string(), "--edit".to_string()];

    let extracted = extract_wrapper_flags_from_passthrough(&mut args).unwrap();

    assert!(extracted.edit);
    assert_eq!(args, vec!["do something"]);
}

#[test]
fn old_non_interactive_flags_pass_through_to_provider() {
    let mut args = vec![
        "-n".to_string(),
        "--non-interactive".to_string(),
        "--ni".to_string(),
        "task".to_string(),
    ];

    let extracted = extract_wrapper_flags_from_passthrough(&mut args).unwrap();

    // Old flags should NOT be consumed by Claudine
    assert!(!extracted.interactive);
    assert_eq!(args, vec!["-n", "--non-interactive", "--ni", "task"]);
}

#[test]
fn extract_wrapper_flags_lifts_operation_from_passthrough() {
    let mut args = vec![
        "do something".to_string(),
        "--op".to_string(),
        "commit".to_string(),
    ];

    let extracted = extract_wrapper_flags_from_passthrough(&mut args).unwrap();

    assert_eq!(extracted.operation.as_deref(), Some("commit"));
    assert_eq!(args, vec!["do something"]);
}

#[test]
fn extract_wrapper_flags_lifts_operation_equals_form() {
    let mut args = vec!["do something".to_string(), "--operation=deploy".to_string()];

    let extracted = extract_wrapper_flags_from_passthrough(&mut args).unwrap();

    assert_eq!(extracted.operation.as_deref(), Some("deploy"));
    assert_eq!(args, vec!["do something"]);
}

#[test]
fn extract_wrapper_flags_lifts_op_equals_form() {
    let mut args = vec!["do something".to_string(), "--op=review".to_string()];

    let extracted = extract_wrapper_flags_from_passthrough(&mut args).unwrap();

    assert_eq!(extracted.operation.as_deref(), Some("review"));
    assert_eq!(args, vec!["do something"]);
}

#[test]
fn extract_wrapper_flags_lifts_perf_from_passthrough() {
    let mut args = vec!["do something".to_string(), "--perf".to_string()];

    let extracted = extract_wrapper_flags_from_passthrough(&mut args).unwrap();

    assert!(extracted.perf);
    assert_eq!(args, vec!["do something"]);
}

#[test]
fn extract_wrapper_flags_respects_double_dash_in_passthrough() {
    // User typed: claudine claude prompt -- --silent -y
    //
    // clap collects the tail verbatim because `trailing_var_arg` began
    // capturing at `prompt`, so the passthrough literally contains `--`.
    // Anything at or after that `--` must be opaque to Claudine.
    let mut args = vec![
        "prompt".to_string(),
        "--".to_string(),
        "--silent".to_string(),
        "-y".to_string(),
    ];

    let extracted = extract_wrapper_flags_from_passthrough_with_boundary(&mut args, 1).unwrap();

    assert!(!extracted.silent);
    assert!(!extracted.yolo);
    assert_eq!(args, vec!["prompt", "--", "--silent", "-y"]);
}

#[test]
fn extract_wrapper_flags_respects_double_dash_consumed_by_clap() {
    // User typed: claudine claude -- prompt --silent
    //
    // clap consumed `--` as the positional separator so it is absent from
    // the passthrough vector. Boundary detection must recover the tail
    // count from the raw process arguments.
    let mut args = vec!["prompt".to_string(), "--silent".to_string()];

    let raw = vec![
        "claudine".to_string(),
        "claude".to_string(),
        "--".to_string(),
        "prompt".to_string(),
        "--silent".to_string(),
    ];
    let boundary = find_passthrough_dash_boundary_with_raw(&args, &raw).unwrap();
    let extracted =
        extract_wrapper_flags_from_passthrough_with_boundary(&mut args, boundary).unwrap();

    assert!(!extracted.silent);
    assert_eq!(args, vec!["prompt", "--silent"]);
}

#[test]
fn extract_wrapper_flags_extracts_before_dash_but_not_after() {
    // User typed: claudine claude -y prompt -- --yolo
    //
    // `-y` BEFORE the prompt is consumed by clap (not present in
    // passthrough). The trailing `--yolo` after `--` must remain untouched
    // so it can collide with an agent-owned flag without being stolen.
    let mut args = vec!["prompt".to_string(), "--".to_string(), "--yolo".to_string()];

    let extracted = extract_wrapper_flags_from_passthrough_with_boundary(&mut args, 1).unwrap();

    assert!(!extracted.yolo);
    assert_eq!(args, vec!["prompt", "--", "--yolo"]);
}

#[test]
fn extract_wrapper_flags_extracts_edit_before_dash_but_not_after() {
    let mut args = vec!["prompt".to_string(), "--".to_string(), "--edit".to_string()];

    let extracted = extract_wrapper_flags_from_passthrough_with_boundary(&mut args, 1).unwrap();

    assert!(!extracted.edit);
    assert_eq!(args, vec!["prompt", "--", "--edit"]);
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
        Some(1)
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
        Some(0)
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
fn extract_wrapper_flags_errors_on_dangling_operation_flag() {
    let mut args = vec!["prompt".to_string(), "--operation".to_string()];
    let boundary = args.len();

    let err =
        extract_wrapper_flags_from_passthrough_with_boundary(&mut args, boundary).unwrap_err();
    let message = err.to_string();
    assert!(
        message.contains("--operation"),
        "expected error to mention --operation, got: {message}"
    );
    assert!(
        message.to_lowercase().contains("missing value"),
        "expected error to describe the missing value, got: {message}"
    );
}

#[test]
fn extract_wrapper_flags_errors_on_dangling_op_alias() {
    let mut args = vec!["prompt".to_string(), "--op".to_string()];
    let boundary = args.len();

    let err =
        extract_wrapper_flags_from_passthrough_with_boundary(&mut args, boundary).unwrap_err();
    let message = err.to_string();
    assert!(
        message.contains("--op"),
        "expected error to mention --op, got: {message}"
    );
}

#[test]
fn extract_wrapper_flags_errors_when_operation_value_is_dash_separator() {
    // User typed: claudine claude --operation -- prompt
    //
    // `--operation` would otherwise greedily consume `--` as its value,
    // which is nonsensical. Require a real value before the separator.
    let mut args = vec![
        "--operation".to_string(),
        "--".to_string(),
        "prompt".to_string(),
    ];

    let err = extract_wrapper_flags_from_passthrough_with_boundary(&mut args, 1).unwrap_err();
    let message = err.to_string();
    assert!(
        message.contains("--operation"),
        "expected error to mention --operation, got: {message}"
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
        fn proptest_extract_wrapper_flags_preserves_others(
            flags in prop::collection::vec("-y|--yolo|-i|--interactive|--edit|-q|--quiet|--silent", 0..5),
            others in prop::collection::vec("[a-z0-9]+", 0..10)
        ) {
            let mut args = Vec::new();
            for o in &others {
                args.push(o.clone());
            }
            for f in &flags {
                args.push(f.clone());
            }

            // Shuffle manually or just accept order for now
            // Pass a boundary equal to args.len() so std::env::args() is
            // not consulted inside the proptest runner.
            let boundary = args.len();
            let extracted =
                extract_wrapper_flags_from_passthrough_with_boundary(&mut args, boundary)
                    .unwrap();

            // All 'others' should still be there
            assert_eq!(args.len(), others.len());
            for o in others {
                assert!(args.contains(&o));
            }

            if flags.iter().any(|f| f == "-y" || f == "--yolo") {
                assert!(extracted.yolo);
            }
            if flags.iter().any(|f| f == "-i" || f == "--interactive") {
                assert!(extracted.interactive);
            }
            if flags.iter().any(|f| f == "--edit") {
                assert!(extracted.edit);
            }
        }
    }
}
