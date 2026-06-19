use super::super::*;

fn detect_native_output(provider: Provider, args: &[&str]) -> bool {
    let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    super::super::super::has_explicit_native_output_request(provider, &args)
}

#[test]
fn codex_json_flag_disables_structured_parsing() {
    assert!(detect_native_output(Provider::Codex, &["--json"]));
}

#[test]
fn claude_output_format_json_disables_structured_parsing() {
    assert!(detect_native_output(
        Provider::Claude,
        &["--output-format", "json"]
    ));
}

#[test]
fn claude_output_format_equals_stream_json_disables_structured_parsing() {
    assert!(detect_native_output(
        Provider::Claude,
        &["--output-format=stream-json"]
    ));
}

#[test]
fn gemini_output_format_json_disables_structured_parsing() {
    assert!(detect_native_output(
        Provider::Gemini,
        &["--output-format", "json"]
    ));
}

#[test]
fn gemini_output_format_equals_stream_json_disables_structured_parsing() {
    assert!(detect_native_output(
        Provider::Gemini,
        &["--output-format=stream-json"]
    ));
}

#[test]
fn qwen_output_format_json_disables_structured_parsing() {
    assert!(detect_native_output(
        Provider::QwenCode,
        &["--output-format", "json"]
    ));
}

#[test]
fn qwen_output_format_equals_stream_json_disables_structured_parsing() {
    assert!(detect_native_output(
        Provider::QwenCode,
        &["--output-format=stream-json"]
    ));
}

#[test]
fn opencode_format_json_disables_structured_parsing() {
    assert!(detect_native_output(
        Provider::OpenCode,
        &["--format", "json"]
    ));
}

#[test]
fn opencode_format_equals_json_disables_structured_parsing() {
    assert!(detect_native_output(Provider::OpenCode, &["--format=json"]));
}

#[test]
fn kimi_wire_transport_flag_does_not_count_as_native_output_request() {
    assert!(!detect_native_output(Provider::KimiCode, &["--wire"]));
}

#[test]
fn unknown_provider_flags_do_not_match() {
    assert!(!detect_native_output(Provider::Claude, &["--unknown-flag"]));
    assert!(!detect_native_output(Provider::Codex, &["--unknown-flag"]));
    assert!(!detect_native_output(
        Provider::OpenCode,
        &["--unknown-flag"]
    ));
}

#[test]
fn goose_no_output_flags_always_false() {
    assert!(!detect_native_output(Provider::Goose, &[]));
    assert!(!detect_native_output(
        Provider::Goose,
        &["--some-random-flag"]
    ));
}

#[test]
fn empty_args_always_false() {
    for provider in claudine::provider::PROVIDERS_DISPLAY_ORDER {
        assert!(
            !detect_native_output(provider, &[]),
            "{provider:?}: empty args should not trigger native output detection"
        );
    }
}

#[test]
fn flags_after_double_dash_separator_are_ignored() {
    assert!(!detect_native_output(Provider::Codex, &["--", "--json"]));
    assert!(!detect_native_output(
        Provider::Claude,
        &["--", "--output-format", "json"]
    ));
}

// -- Parity: explicit_native_output_detection_matches_provider_catalog (Phase 3, Task 7)

#[test]
fn explicit_native_output_detection_matches_provider_catalog() {
    for provider in claudine::provider::PROVIDERS_DISPLAY_ORDER {
        let info = provider_info(provider);
        for support in info.output_formats {
            match support.selector {
                OutputFormatSelector::Flag { flag } => {
                    let args: Vec<String> = vec![flag.to_string()];
                    assert!(
                        super::super::super::has_explicit_native_output_request(provider, &args),
                        "{provider:?}: catalog Flag {flag} must be detected as native output"
                    );
                }
                OutputFormatSelector::FlagValue { flag } => {
                    let separated: Vec<String> =
                        vec![flag.to_string(), support.native_name.to_string()];
                    assert!(
                        super::super::super::has_explicit_native_output_request(provider, &separated),
                        "{provider:?}: catalog FlagValue {flag} {} must be detected as native output",
                        support.native_name
                    );
                    let inline: Vec<String> = vec![format!("{flag}={}", support.native_name)];
                    assert!(
                        super::super::super::has_explicit_native_output_request(provider, &inline),
                        "{provider:?}: catalog FlagValue {flag}={} (inline) must be detected as native output",
                        support.native_name
                    );
                }
                OutputFormatSelector::TransportFlag { flag } => {
                    let args: Vec<String> = vec![flag.to_string()];
                    assert!(
                        !super::super::super::has_explicit_native_output_request(provider, &args),
                        "{provider:?}: catalog TransportFlag {flag} must NOT be detected as native output"
                    );
                }
                OutputFormatSelector::Default | OutputFormatSelector::Positional { .. } => {}
            }
        }
    }
}
