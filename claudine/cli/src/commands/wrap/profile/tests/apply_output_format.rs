use super::super::*;

fn profile(provider: Provider) -> &'static dyn WrapperProfile {
    profile_for_provider(provider).unwrap()
}

fn provider_output_format(format: OutputFormat) -> claudine::provider::OutputFormat {
    match format {
        OutputFormat::Json => claudine::provider::OutputFormat::Json,
        OutputFormat::Text => claudine::provider::OutputFormat::Text,
        OutputFormat::Stream => claudine::provider::OutputFormat::Stream,
    }
}

fn catalog_output_args(provider: Provider, format: OutputFormat) -> Option<Vec<String>> {
    let support = provider_info(provider)
        .output_formats
        .iter()
        .find(|support| support.format == provider_output_format(format))?;

    let args = match support.selector {
        OutputFormatSelector::Default | OutputFormatSelector::Positional { .. } => Vec::new(),
        OutputFormatSelector::Flag { flag } | OutputFormatSelector::TransportFlag { flag } => {
            vec![flag.to_string()]
        }
        OutputFormatSelector::FlagValue { flag } => {
            vec![flag.to_string(), support.native_name.to_string()]
        }
    };
    Some(args)
}

#[test]
fn apply_output_format_matches_provider_catalog_for_every_provider() {
    for provider in claudine::provider::PROVIDERS_DISPLAY_ORDER {
        let Some(profile) = profile_for_provider(provider) else {
            continue;
        };
        for format in [OutputFormat::Json, OutputFormat::Text, OutputFormat::Stream] {
            let mut args = Vec::new();
            let warning = profile.apply_output_format(&mut args, format);
            match catalog_output_args(provider, format) {
                Some(expected) => {
                    assert!(
                        warning.is_none(),
                        "{provider:?} should support cataloged output format {format}"
                    );
                    assert_eq!(args, expected, "{provider:?} --output {format}");
                }
                None => {
                    assert!(
                        warning.is_some(),
                        "{provider:?} should warn for uncataloged output format {format}"
                    );
                    assert!(args.is_empty(), "{provider:?} --output {format}");
                }
            }
        }
    }
}

#[test]
fn claude_output_format_supports_all_formats() {
    let p = profile(Provider::Claude);
    for format in [OutputFormat::Json, OutputFormat::Text, OutputFormat::Stream] {
        let mut args = Vec::new();
        let warning = p.apply_output_format(&mut args, format);
        assert!(warning.is_none(), "Claude should support --output {format}");
        assert!(!args.is_empty());
    }
}

#[test]
fn gemini_output_format_uses_output_format_flag_and_supports_stream_json() {
    let p = profile(Provider::Gemini);

    let mut json_args = Vec::new();
    assert!(
        p.apply_output_format(&mut json_args, OutputFormat::Json)
            .is_none()
    );
    assert_eq!(json_args, vec!["--output-format", "json"]);

    let mut text_args = Vec::new();
    assert!(
        p.apply_output_format(&mut text_args, OutputFormat::Text)
            .is_none()
    );
    assert_eq!(text_args, vec!["--output-format", "text"]);

    let mut stream_args = Vec::new();
    assert!(
        p.apply_output_format(&mut stream_args, OutputFormat::Stream)
            .is_none()
    );
    assert_eq!(stream_args, vec!["--output-format", "stream-json"]);
}

#[test]
fn codex_sandbox_adds_flag() {
    let p = profile(Provider::Codex);
    let mut args = Vec::new();
    let warning = p.apply_sandbox(&mut args);
    assert!(warning.is_none());
    assert_eq!(args, vec!["--sandbox"]);
}

#[test]
fn qwen_sandbox_adds_flag() {
    let p = profile(Provider::QwenCode);
    let mut args = Vec::new();
    let warning = p.apply_sandbox(&mut args);
    assert!(warning.is_none());
    assert_eq!(args, vec!["--sandbox"]);
}

#[test]
fn unsupported_sandbox_returns_warning() {
    let p = profile(Provider::Claude);
    let mut args = Vec::new();
    let warning = p.apply_sandbox(&mut args);
    assert!(warning.is_some());
    assert!(args.is_empty());
}

// The generic `MODEL` env var is part of Claudine's wrapper contract
// (alongside AGENT/YOLO/OPERATION/PACKAGE/PACKAGE_AREA) and must be
// exported for every provider, not just OpenCode. Providers deliver the
// model to the child CLI through their own mechanism (argv flag,
// GOOSE_MODEL, ...), but the generic `MODEL` must always be present.
#[test]
fn default_apply_model_sets_generic_model_env() {
    let p = profile(Provider::Claude);
    let mut args: Vec<String> = Vec::new();
    let mut env: Vec<(String, String)> = Vec::new();
    p.apply_model(&mut args, &mut env, "claude-opus-4-8");
    assert!(
        env.contains(&("MODEL".to_string(), "claude-opus-4-8".to_string())),
        "default apply_model must export the generic MODEL env var, got {env:?}"
    );
}

#[test]
fn goose_apply_model_sets_generic_model_env_alongside_goose_model() {
    let p = profile(Provider::Goose);
    let mut args: Vec<String> = Vec::new();
    let mut env: Vec<(String, String)> = Vec::new();
    p.apply_model(&mut args, &mut env, "gpt-4o");
    assert!(
        env.contains(&("GOOSE_MODEL".to_string(), "gpt-4o".to_string())),
        "goose must keep its provider-specific GOOSE_MODEL env, got {env:?}"
    );
    assert!(
        env.contains(&("MODEL".to_string(), "gpt-4o".to_string())),
        "goose must also export the generic MODEL env var, got {env:?}"
    );
}

#[test]
fn goose_model_uses_env_var() {
    let p = profile(Provider::Goose);
    let mut args = Vec::new();
    let mut env_overrides = Vec::new();

    let warning = p.apply_model(&mut args, &mut env_overrides, "gpt-4o");
    assert!(warning.is_none());
    assert!(args.is_empty());
    assert!(env_overrides.contains(&("GOOSE_MODEL".to_string(), "gpt-4o".to_string())));
}

/// Source guard: the catalog-driven default bodies for `apply_output_format`
/// and `apply_entrypoint` must read from the provider catalog. The bodies live
/// in `profile/apply.rs` (the trait keeps only thin delegators); if a future
/// change adds a raw `match provider` block inside either, this test catches
/// it. The `has_explicit_native_output_request` helper has its own source guard
/// in `wrap::flags`.
#[test]
fn output_detection_and_application_uses_catalog_not_raw_dispatch() {
    let source = include_str!("../apply.rs");

    for fn_signature in ["fn apply_output_format(", "fn apply_entrypoint("] {
        let start = source
            .find(fn_signature)
            .unwrap_or_else(|| panic!("{fn_signature} default body should exist in apply.rs"));
        let end = source[start..]
            .find("\npub(super) fn ")
            .or_else(|| source[start..].find("\n/// "))
            .map(|offset| start + offset)
            .unwrap_or(source.len());
        let body = &source[start..end];

        assert!(
            body.contains("provider_info(provider)"),
            "{fn_signature} default must read from provider catalog: {body}"
        );

        for variant in [
            "Provider::Claude",
            "Provider::Codex",
            "Provider::Gemini",
            "Provider::KimiCode",
            "Provider::OpenCode",
            "Provider::QwenCode",
            "Provider::Goose",
        ] {
            assert!(
                !body.contains(variant),
                "{fn_signature} default must not contain {variant} — use catalog data"
            );
        }
    }
}
