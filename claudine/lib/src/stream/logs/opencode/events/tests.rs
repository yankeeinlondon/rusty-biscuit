use super::*;
use chrono::Timelike;

#[test]
fn header_accepts_all_levels() {
    for (level_str, expected) in [
        ("DEBUG", LogLevel::Debug),
        ("INFO", LogLevel::Info),
        ("WARN", LogLevel::Warn),
        ("ERROR", LogLevel::Error),
    ] {
        let line = format!("{level_str} 2026-04-15T21:28:30 +5ms service=default msg=ok");
        match parse_line(&line) {
            ParsedOpenCodeStderrLine::Structured(record) => {
                assert_eq!(record.level, expected, "{level_str}");
                assert_eq!(record.delta_ms, 5);
                assert_eq!(
                    record.tags.get("service").map(String::as_str),
                    Some("default")
                );
                assert_eq!(record.tags.get("msg").map(String::as_str), Some("ok"));
            }
            other => panic!("expected Structured for {level_str}, got {other:?}"),
        }
    }
}

#[test]
fn header_rejects_non_matching_lines() {
    for line in [
        "not a log line at all",
        "    at processTicksAndRejections (native:7:39) fatal",
        "\u{1b}[91m\u{1b}[1mError: \u{1b}[0mUnexpected error",
        "",
    ] {
        match parse_line(line) {
            ParsedOpenCodeStderrLine::RawText(raw) => assert_eq!(raw, line),
            other => panic!("expected RawText for {line:?}, got {other:?}"),
        }
    }
}

#[test]
fn header_rejects_unknown_level() {
    match parse_line("TRACE 2026-04-15T21:28:30 +5ms service=default") {
        ParsedOpenCodeStderrLine::RawText(_) => {}
        other => panic!("expected RawText, got {other:?}"),
    }
}

#[test]
fn parses_simple_key_value_tags() {
    let line = "INFO 2026-04-15T21:28:30 +0ms service=default providerID=zai-coding-plan modelID=glm-5.1";
    let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
        panic!("expected Structured");
    };
    assert_eq!(
        record.tags.get("service").map(String::as_str),
        Some("default")
    );
    assert_eq!(
        record.tags.get("providerID").map(String::as_str),
        Some("zai-coding-plan")
    );
    assert_eq!(
        record.tags.get("modelID").map(String::as_str),
        Some("glm-5.1")
    );
    assert_eq!(record.message, "");
}

#[test]
fn parses_inline_json_tag() {
    let line = r#"ERROR 2026-04-15T19:26:02 +3054ms service=llm error={"error":{"name":"AI_RetryError","reason":"maxRetriesExceeded","errors":[{"name":"AI_APICallError","statusCode":429}]}}"#;
    let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
        panic!("expected Structured");
    };
    let error_value = record
        .tags
        .get("error")
        .expect("error tag should be captured");
    assert!(error_value.contains("AI_RetryError"), "{error_value}");
    assert!(error_value.contains("AI_APICallError"), "{error_value}");
}

#[test]
fn error_equals_is_terminal_to_end_of_line() {
    let line = "ERROR 2026-04-15T21:28:30 +1ms service=config error=some raw failure text here";
    let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
        panic!("expected Structured");
    };
    assert_eq!(
        record.tags.get("error").map(String::as_str),
        Some("some raw failure text here"),
    );
}

#[test]
fn err_captures_trailing_failed_to_load_command() {
    let line = "ERROR 2026-04-15T21:28:30 +315ms service=config command=/Users/ken/.config/opencode/commands/catalog.md err=ENOENT: no such file or directory, open '/Users/ken/.config/opencode/commands/catalog.md' failed to load command";
    let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
        panic!("expected Structured");
    };
    let err = record.tags.get("err").expect("err tag should be captured");
    assert!(err.ends_with("failed to load command"), "{err}");
    assert_eq!(
        record.tags.get("command").map(String::as_str),
        Some("/Users/ken/.config/opencode/commands/catalog.md"),
    );
}

#[test]
fn tolerates_unknown_tags() {
    let line = "ERROR 2026-04-15T21:28:30 +0ms service=default brand_new_tag=hello other=world";
    let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
        panic!("expected Structured");
    };
    assert_eq!(
        record.tags.get("brand_new_tag").map(String::as_str),
        Some("hello"),
    );
    assert_eq!(record.tags.get("other").map(String::as_str), Some("world"));
}

#[test]
fn preserves_raw_line() {
    let line = "ERROR 2026-04-15T21:28:30 +0ms service=default msg=ok";
    let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
        panic!("expected Structured");
    };
    assert_eq!(record.raw, line);
}

#[test]
fn parses_timestamp_as_utc() {
    let line = "ERROR 2026-04-15T21:28:30 +0ms service=default msg=ok";
    let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
        panic!("expected Structured");
    };
    assert_eq!(
        record.timestamp.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
        "2026-04-15T21:28:30Z",
    );
}

#[test]
fn header_body_is_optional() {
    let line = "INFO 2026-04-15T21:28:30 +0ms";
    let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
        panic!("expected Structured");
    };
    assert!(record.tags.is_empty());
    assert_eq!(record.message, "");
}

#[test]
fn parses_tags_with_dots_and_hyphens() {
    let line = "ERROR 2026-04-15T21:28:30 +33ms service=llm provider-id=zai session.id=s1 model=k2p6 message";
    let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
        panic!("expected Structured");
    };
    assert_eq!(record.tags.get("provider-id").unwrap(), "zai");
    assert_eq!(record.tags.get("session.id").unwrap(), "s1");
    assert_eq!(record.tags.get("model").unwrap(), "k2p6 message");
    assert_eq!(record.message, "");
}

#[test]
fn new_format_parses_info_level() {
    let line = "timestamp=2026-06-10T16:11:27.352Z level=INFO service=default run=abc message=tracking";
    let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
        panic!("expected Structured");
    };

    assert_eq!(record.level, LogLevel::Info);
    assert_eq!(
        record.timestamp.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string(),
        "2026-06-10T16:11:27.352Z",
    );
    assert_eq!(record.delta_ms, 0);
    assert_eq!(
        record.tags.get("service").map(String::as_str),
        Some("default"),
    );
    assert_eq!(record.tags.get("run").map(String::as_str), Some("abc"));
    assert_eq!(
        record.tags.get("message").map(String::as_str),
        Some("tracking"),
    );
}

#[test]
fn new_format_parses_all_levels() {
    for (level_str, expected) in [
        ("DEBUG", LogLevel::Debug),
        ("INFO", LogLevel::Info),
        ("WARN", LogLevel::Warn),
        ("ERROR", LogLevel::Error),
    ] {
        let line = format!(
            "timestamp=2026-06-10T16:11:27.352Z level={level_str} service=default"
        );
        let ParsedOpenCodeStderrLine::Structured(record) = parse_line(&line) else {
            panic!("expected Structured for {level_str}");
        };
        assert_eq!(record.level, expected, "{level_str}");
    }
}

#[test]
fn new_format_timestamp_includes_millis() {
    let line = "timestamp=2026-06-10T16:11:27.352Z level=INFO service=default";
    let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
        panic!("expected Structured");
    };

    assert_eq!(record.timestamp.timestamp_subsec_millis(), 352);
    assert_eq!(record.timestamp.nanosecond(), 352_000_000);
}

#[test]
fn new_format_preserves_raw_line() {
    let line = "timestamp=2026-06-10T16:11:27.352Z level=WARN service=default run=abc";
    let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
        panic!("expected Structured");
    };

    assert_eq!(record.raw, line);
}

#[test]
fn new_format_extracts_tags() {
    let line = "timestamp=2026-06-10T16:11:27.352Z level=INFO run=abc service=session id=ses_123 parentID=ses_parent title=My task";
    let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
        panic!("expected Structured");
    };

    assert_eq!(record.tags.get("run").map(String::as_str), Some("abc"));
    assert_eq!(
        record.tags.get("service").map(String::as_str),
        Some("session"),
    );
    assert_eq!(record.tags.get("id").map(String::as_str), Some("ses_123"));
    assert_eq!(
        record.tags.get("parentID").map(String::as_str),
        Some("ses_parent"),
    );
    assert_eq!(
        record.tags.get("title").map(String::as_str),
        Some("My task"),
    );
}

#[test]
fn new_format_message_tag_captured() {
    let line = "timestamp=2026-06-10T16:11:27.352Z level=INFO message=tracking hash=abc123";
    let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
        panic!("expected Structured");
    };

    assert_eq!(
        record.tags.get("message").map(String::as_str),
        Some("tracking"),
    );
    assert_eq!(record.tags.get("hash").map(String::as_str), Some("abc123"));
}

#[test]
fn new_format_rejects_non_matching() {
    for line in [
        "timestamp=not-a-date level=INFO service=default",
        "time=2026-06-10T16:11:27.352Z level=INFO service=default",
        "timestamp=2026-06-10T16:11:27.352Z severity=INFO service=default",
    ] {
        match parse_line(line) {
            ParsedOpenCodeStderrLine::RawText(raw) => assert_eq!(raw, line),
            other => panic!("expected RawText for {line:?}, got {other:?}"),
        }
    }
}

#[test]
fn new_format_without_message_tag() {
    let line = "timestamp=2026-06-10T16:11:27.352Z level=INFO run=abc service=session id=ses_123 parentID=ses_parent";
    let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
        panic!("expected Structured");
    };

    assert_eq!(
        record.tags.get("service").map(String::as_str),
        Some("session"),
    );
    assert_eq!(record.tags.get("id").map(String::as_str), Some("ses_123"));
    assert!(!record.tags.contains_key("message"));
    assert_eq!(record.message, "");
}

#[test]
fn new_format_without_millis() {
    let line = "timestamp=2026-06-10T16:11:27Z level=INFO service=default";
    let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
        panic!("expected Structured");
    };

    assert_eq!(
        record.timestamp.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
        "2026-06-10T16:11:27Z",
    );
    assert_eq!(record.timestamp.timestamp_subsec_millis(), 0);
}

#[test]
fn provider_limit_signal_payload_carries_kind_and_optional_fields() {
    let classification = LogClassification::ProviderLimit {
        status_code: Some(429),
        kind: ProviderLimitKind::UsageCap,
        reset_at: Some(
            Utc.with_ymd_and_hms(2026, 4, 16, 4, 18, 56)
                .single()
                .expect("valid timestamp"),
        ),
        provider_id: Some("zai-coding-plan".to_string()),
        model_id: None,
        provider_error: "Usage limit reached".to_string(),
    };
    assert_eq!(
        classification.to_signal_payload(),
        serde_json::json!({
            "classification": "ProviderLimit",
            "kind": "UsageCap",
            "status_code": 429,
            "reset_at": "2026-04-16T04:18:56Z",
            "provider_id": "zai-coding-plan",
            "provider_error": "Usage limit reached",
        }),
    );
}

// Absent optional fields must be OMITTED (not null): the engine's
// `exists` operator fires on any non-null value, so a `null` slot would
// change detection semantics.
#[test]
fn signal_payload_omits_absent_optionals() {
    let classification = LogClassification::ProviderLimit {
        status_code: None,
        kind: ProviderLimitKind::RateLimited,
        reset_at: None,
        provider_id: None,
        model_id: None,
        provider_error: "429".to_string(),
    };
    let payload = classification.to_signal_payload();
    let object = payload.as_object().expect("payload is an object");
    assert_eq!(object.get("kind"), Some(&serde_json::json!("RateLimited")));
    for absent in ["status_code", "reset_at", "provider_id", "model_id"] {
        assert!(!object.contains_key(absent), "{absent} must be omitted");
    }
}

#[test]
fn classification_keyed_signal_payloads_use_variant_names() {
    let auth = LogClassification::AuthFailure {
        message: "AuthenticationError: Invalid API key".to_string(),
    };
    assert_eq!(
        auth.to_signal_payload(),
        serde_json::json!({
            "classification": "AuthFailure",
            "message": "AuthenticationError: Invalid API key",
        }),
    );

    let boot = LogClassification::BootBanner {
        version: "1.14.48".to_string(),
    };
    assert_eq!(
        boot.to_signal_payload(),
        serde_json::json!({
            "classification": "BootBanner",
            "version": "1.14.48",
        }),
    );

    let call = LogClassification::LlmCall {
        provider_id: "kimi-for-coding".to_string(),
        model_id: "k2p6".to_string(),
        mode: "primary".to_string(),
        is_stream: true,
    };
    assert_eq!(
        call.to_signal_payload(),
        serde_json::json!({
            "classification": "LlmCall",
            "provider_id": "kimi-for-coding",
            "model_id": "k2p6",
            "mode": "primary",
            "is_stream": true,
        }),
    );

    assert_eq!(
        LogClassification::Unclassified.to_signal_payload(),
        serde_json::json!({ "classification": "Unclassified" }),
    );
}

#[test]
fn mixed_format_stream_parses_both() {
    let lines = [
        (
            "INFO 2026-04-15T21:28:30 +5ms service=default msg=legacy",
            LogLevel::Info,
            "2026-04-15T21:28:30Z",
            5,
            "default",
        ),
        (
            "timestamp=2026-06-10T16:11:27.352Z level=WARN service=session id=ses_new",
            LogLevel::Warn,
            "2026-06-10T16:11:27Z",
            0,
            "session",
        ),
        (
            "ERROR 2026-04-15T21:28:31 +42ms service=llm providerID=legacy modelID=model",
            LogLevel::Error,
            "2026-04-15T21:28:31Z",
            42,
            "llm",
        ),
        (
            "timestamp=2026-06-10T16:11:28Z level=DEBUG service=permission permission=task",
            LogLevel::Debug,
            "2026-06-10T16:11:28Z",
            0,
            "permission",
        ),
    ];

    for (line, expected_level, expected_ts, expected_delta_ms, expected_service) in lines {
        let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
            panic!("expected Structured for {line}");
        };

        assert_eq!(record.level, expected_level, "{line}");
        assert_eq!(
            record.timestamp.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
            expected_ts,
            "{line}",
        );
        assert_eq!(record.delta_ms, expected_delta_ms, "{line}");
        assert_eq!(
            record.tags.get("service").map(String::as_str),
            Some(expected_service),
            "{line}",
        );
    }
}
