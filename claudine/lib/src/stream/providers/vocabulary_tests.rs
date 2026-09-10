use super::common::{ErrorKeywords, classify_error_by_keywords};
use super::semantic::SemanticErrorKind;
use crate::provider_id::Provider;

#[derive(Clone, Copy, Debug)]
enum Input {
    Kind(&'static str),
    Message(&'static str),
    Code(i32),
}

#[derive(Clone, Copy, Debug)]
struct Case {
    provider: &'static str,
    input: Input,
    expected: SemanticErrorKind,
}

fn provider(slug: &str) -> Provider {
    Provider::parse_cli_name(slug).unwrap_or_else(|| panic!("unknown test provider: {slug}"))
}

fn classify(case: Case) -> SemanticErrorKind {
    let vocabulary = super::vocabulary::error_keywords(provider(case.provider));
    match case.input {
        Input::Kind(kind) => classify_error_by_keywords(vocabulary, None, Some(kind), None),
        Input::Message(message) => {
            classify_error_by_keywords(vocabulary, None, None, Some(message))
        }
        Input::Code(code) => classify_error_by_keywords(vocabulary, Some(code), None, None),
    }
}

fn assert_declared(vocabulary: &ErrorKeywords, input: Input, expected: SemanticErrorKind) {
    let declared = match input {
        Input::Kind(needle) => vocabulary
            .kind_buckets
            .iter()
            .any(|(kind, needles)| *kind == expected && needles.contains(&needle)),
        Input::Message(needle) => vocabulary
            .msg_buckets
            .iter()
            .any(|(kind, needles)| *kind == expected && needles.contains(&needle)),
        Input::Code(code) => vocabulary
            .code_buckets
            .contains(&(code, expected)),
    };
    assert!(declared, "expected {input:?} in the {expected:?} bucket");
}

#[test]
fn accepted_research_additions_are_declared_and_classify() {
    use SemanticErrorKind::{AgentNative, ApiRemote, Configuration};

    let cases = [
        Case { provider: "claude", input: Input::Kind("oauth_org_not_allowed"), expected: Configuration },
        Case { provider: "claude", input: Input::Kind("invalid_request"), expected: Configuration },
        Case { provider: "claude", input: Input::Kind("model_not_found"), expected: Configuration },
        Case { provider: "claude", input: Input::Message("server is temporarily limiting requests"), expected: ApiRemote },
        Case { provider: "claude", input: Input::Message("request rejected (429)"), expected: ApiRemote },
        Case { provider: "claude", input: Input::Message("is temporarily unavailable, so auto mode cannot determine"), expected: ApiRemote },
        Case { provider: "claude", input: Input::Message("not logged in"), expected: Configuration },
        Case { provider: "claude", input: Input::Message("invalid api key"), expected: Configuration },
        Case { provider: "claude", input: Input::Message("could not resolve authentication method"), expected: Configuration },
        Case { provider: "claude", input: Input::Message("oauth token revoked"), expected: Configuration },
        Case { provider: "claude", input: Input::Message("oauth token has expired"), expected: Configuration },
        Case { provider: "claude", input: Input::Message("login expired"), expected: Configuration },
        Case { provider: "codex", input: Input::Message("overloaded"), expected: ApiRemote },
        Case { provider: "codex", input: Input::Message("selected model is at capacity"), expected: ApiRemote },
        Case { provider: "gemini", input: Input::Kind("forbidden"), expected: Configuration },
        Case { provider: "gemini", input: Input::Kind("unauthorized"), expected: Configuration },
        Case { provider: "gemini", input: Input::Kind("fatalturnlimitederror"), expected: AgentNative },
        Case { provider: "gemini", input: Input::Message("overloaded"), expected: ApiRemote },
        Case { provider: "gemini", input: Input::Message("resource_exhausted"), expected: ApiRemote },
        Case { provider: "gemini", input: Input::Message("no capacity available for model"), expected: ApiRemote },
        Case { provider: "kilo", input: Input::Message("server error"), expected: ApiRemote },
        Case { provider: "kilo", input: Input::Message("response decompression failed"), expected: ApiRemote },
        Case { provider: "kilo", input: Input::Message("please reauthenticate with the copilot provider"), expected: Configuration },
        Case { provider: "kilo", input: Input::Message("unauthorized:"), expected: Configuration },
        Case { provider: "kilo", input: Input::Message("forbidden:"), expected: Configuration },
        Case { provider: "kimi-code", input: Input::Code(-32000), expected: AgentNative },
        Case { provider: "kimi-code", input: Input::Code(-32001), expected: Configuration },
        Case { provider: "kimi-code", input: Input::Code(-32002), expected: Configuration },
        Case { provider: "kimi-code", input: Input::Code(-32003), expected: ApiRemote },
        Case { provider: "opencode", input: Input::Message("server error"), expected: ApiRemote },
        Case { provider: "opencode", input: Input::Message("connection reset by server"), expected: ApiRemote },
        Case { provider: "opencode", input: Input::Message("provider response headers timed out"), expected: ApiRemote },
        Case { provider: "opencode", input: Input::Message("response decompression failed"), expected: ApiRemote },
        Case { provider: "opencode", input: Input::Message("unauthorized:"), expected: Configuration },
        Case { provider: "opencode", input: Input::Message("forbidden:"), expected: Configuration },
        Case { provider: "pi", input: Input::Message("insufficient_quota"), expected: ApiRemote },
        Case { provider: "pi", input: Input::Message("out of budget"), expected: ApiRemote },
        Case { provider: "pi", input: Input::Message("quota exceeded"), expected: ApiRemote },
        Case { provider: "pi", input: Input::Message("too many requests"), expected: ApiRemote },
        Case { provider: "pi", input: Input::Message("service unavailable"), expected: ApiRemote },
        Case { provider: "pi", input: Input::Message("server error"), expected: ApiRemote },
        Case { provider: "pi", input: Input::Message("internal error"), expected: ApiRemote },
        Case { provider: "pi", input: Input::Message("provider returned error"), expected: ApiRemote },
        Case { provider: "pi", input: Input::Message("network error"), expected: ApiRemote },
        Case { provider: "pi", input: Input::Message("connection refused"), expected: ApiRemote },
        Case { provider: "pi", input: Input::Message("fetch failed"), expected: ApiRemote },
        Case { provider: "pi", input: Input::Message("reset before headers"), expected: ApiRemote },
        Case { provider: "pi", input: Input::Message("socket hang up"), expected: ApiRemote },
        Case { provider: "pi", input: Input::Message("websocket closed"), expected: ApiRemote },
        Case { provider: "pi", input: Input::Message("websocket error"), expected: ApiRemote },
        Case { provider: "pi", input: Input::Message("stream ended before message_stop"), expected: ApiRemote },
        Case { provider: "pi", input: Input::Message("http2 request did not get a response"), expected: ApiRemote },
        Case { provider: "pi", input: Input::Message("resourceexhausted"), expected: ApiRemote },
        Case { provider: "qwen-code", input: Input::Message("no auth type is selected"), expected: Configuration },
        Case { provider: "qwen-code", input: Input::Message("loop detection halted the run"), expected: AgentNative },
    ];

    assert_eq!(cases.len(), 55, "the C1 disposition accepted 55 rows");
    for case in cases {
        let vocabulary = super::vocabulary::error_keywords(provider(case.provider));
        assert_declared(vocabulary, case.input, case.expected);
        assert_eq!(classify(case), case.expected, "failed case: {case:?}");
    }
}

#[test]
fn accepted_additions_preserve_branch_code_and_bucket_precedence() {
    use SemanticErrorKind::{AgentNative, ApiRemote, Configuration};

    let cases = [
        ("gemini", None, Some("forbidden"), Some("overloaded"), Configuration),
        ("opencode", None, Some("auth_error"), Some("server error"), Configuration),
        ("kilo", None, Some("auth_error"), Some("server error"), Configuration),
        ("qwen-code", None, Some("rate_limited"), Some("no auth type is selected"), ApiRemote),
        ("kimi-code", Some(-32000), None, Some("invalid api key"), AgentNative),
        ("kimi-code", Some(-32001), None, Some("rate limit reached"), Configuration),
    ];

    for (provider_name, code, kind, message, expected) in cases {
        assert_eq!(
            classify_error_by_keywords(
                super::vocabulary::error_keywords(provider(provider_name)),
                code,
                kind,
                message,
            ),
            expected,
            "precedence changed for {provider_name}"
        );
    }

    for provider_name in ["opencode", "kilo"] {
        assert_eq!(
            classify_error_by_keywords(
                super::vocabulary::error_keywords(provider(provider_name)),
                None,
                None,
                Some("unauthorized: server error"),
            ),
            ApiRemote,
            "the earlier api_remote message bucket must win for {provider_name}"
        );
    }
}

#[test]
fn broad_additions_keep_representative_near_misses_native() {
    let controls = [
        ("claude", Input::Kind("invalid request")),
        ("claude", Input::Message("request 429 was accepted")),
        ("codex", Input::Message("capacity planning selected a model")),
        ("gemini", Input::Message("resources were exhausted")),
        ("opencode", Input::Message("the server recovered without an error")),
        ("kilo", Input::Message("the server recovered without an error")),
        ("pi", Input::Message("the internal operation completed without errors")),
        ("qwen-code", Input::Message("the loop guard completed normally")),
    ];

    for (provider, input) in controls {
        assert_eq!(
            classify(Case {
                provider,
                input,
                expected: SemanticErrorKind::AgentNative,
            }),
            SemanticErrorKind::AgentNative,
            "near-miss control unexpectedly classified for {provider:?}: {input:?}"
        );
    }
}
