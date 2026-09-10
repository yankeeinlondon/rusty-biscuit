use super::*;
use super::text_util::extract_status_code;
use crate::stream::logs::opencode::events::*;

    #[test]
    fn classifies_malformed_command() {
        let line = "ERROR 2026-04-15T21:28:30 +315ms service=config command=/tmp/foo.md err=ENOENT: no such file or directory failed to load command";
        let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
            panic!("expected Structured");
        };
        match classify(&record) {
            LogClassification::MalformedAsset {
                asset_type,
                path,
                error,
            } => {
                assert_eq!(asset_type, AssetType::Command);
                assert_eq!(path.as_deref(), Some("/tmp/foo.md"));
                assert!(error.contains("failed to load command"));
            }
            other => panic!("expected MalformedAsset, got {other:?}"),
        }
    }

    #[test]
    fn classifies_malformed_skill_and_agent() {
        let skill_line = "ERROR 2026-04-15T21:28:30 +0ms service=config skill=/tmp/s.md err=ENOENT failed to load skill";
        let agent_line = "ERROR 2026-04-15T21:28:30 +0ms service=config agent=/tmp/a.md err=ENOENT failed to load agent";

        for (line, expected) in [
            (skill_line, AssetType::Skill),
            (agent_line, AssetType::Agent),
        ] {
            let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
                panic!("expected Structured for {line}");
            };
            match classify(&record) {
                LogClassification::MalformedAsset { asset_type, .. } => {
                    assert_eq!(asset_type, expected);
                }
                other => panic!("expected MalformedAsset, got {other:?}"),
            }
        }
    }

    #[test]
    fn classifies_usage_cap_with_reset_time() {
        let line = r#"ERROR 2026-04-15T19:26:02 +3054ms service=llm providerID=zai-coding-plan modelID=glm-5.1 error={"error":{"name":"AI_RetryError","reason":"maxRetriesExceeded","errors":[{"name":"AI_APICallError","statusCode":429,"responseBody":"{\"error\":{\"code\":\"1308\",\"message\":\"Usage limit reached. Your limit will reset at 2026-04-16 04:18:56\"}}"}]}}"#;
        let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
            panic!("expected Structured");
        };
        match classify(&record) {
            LogClassification::ProviderLimit {
                status_code,
                kind,
                reset_at,
                ..
            } => {
                assert_eq!(status_code, Some(429));
                assert_eq!(kind, ProviderLimitKind::UsageCap);
                let reset = reset_at.expect("reset_at should be parsed");
                assert_eq!(
                    reset.format("%Y-%m-%d %H:%M:%S").to_string(),
                    "2026-04-16 04:18:56"
                );
            }
            other => panic!("expected ProviderLimit, got {other:?}"),
        }
    }

    // OpenCode 1.17.8 emits stream failures as `message="stream error"` with no
    // `service=` tag and the payload nested under `error.error="<string>"`.
    // Regression for fixes/2026-06-21-opencode-log-fix (session ses_1127ec2f).
    #[test]
    fn classifies_1178_stream_error_usage_cap() {
        let line = r#"timestamp=2026-06-22T04:07:15.161Z level=ERROR run=da37e0dd message="stream error" providerID=zai-coding-plan modelID=glm-5.2 session.id=ses_1127ec2fdffepaJc2kEnX093eo small=false agent=build mode=primary error.error="AI_APICallError: Usage limit reached for 5 hour. Your limit will reset at 2026-06-22 13:59:38""#;
        let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
            panic!("expected Structured");
        };
        match classify(&record) {
            LogClassification::ProviderLimit {
                kind,
                reset_at,
                provider_id,
                model_id,
                provider_error,
                ..
            } => {
                assert_eq!(kind, ProviderLimitKind::UsageCap);
                let reset = reset_at.expect("reset_at should be parsed");
                assert_eq!(
                    reset.format("%Y-%m-%d %H:%M:%S").to_string(),
                    "2026-06-22 13:59:38"
                );
                assert_eq!(provider_id.as_deref(), Some("zai-coding-plan"));
                assert_eq!(model_id.as_deref(), Some("glm-5.2"));
                assert!(provider_error.contains("Usage limit reached"));
                // The flat-string payload's surrounding quotes are stripped.
                assert!(!provider_error.starts_with('"'));
            }
            other => panic!("expected ProviderLimit, got {other:?}"),
        }
    }

    // The matching call-start line must still classify as a stream start, not an
    // error — the `message="stream"` vs `message="stream error"` split is the
    // only signal distinguishing them in the 1.17.8 format.
    #[test]
    fn classifies_1178_stream_start_as_llm_call() {
        let line = r#"timestamp=2026-06-22T04:05:31.166Z level=INFO run=da37e0dd message=stream providerID=zai-coding-plan modelID=glm-5.2 session.id=ses_x small=false agent=build mode=primary"#;
        let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
            panic!("expected Structured");
        };
        match classify(&record) {
            LogClassification::LlmCall {
                provider_id,
                model_id,
                is_stream,
                ..
            } => {
                assert_eq!(provider_id, "zai-coding-plan");
                assert_eq!(model_id, "glm-5.2");
                assert!(is_stream);
            }
            other => panic!("expected LlmCall, got {other:?}"),
        }
    }

    #[test]
    fn classifies_api_failure_when_not_rate_limited() {
        let line = r#"ERROR 2026-04-15T19:26:02 +100ms service=llm error={"error":{"name":"AI_APICallError","message":"upstream boom","statusCode":500}}"#;
        let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
            panic!("expected Structured");
        };
        match classify(&record) {
            LogClassification::ApiFailure {
                status_code,
                error_name,
                message,
                ..
            } => {
                assert_eq!(status_code, Some(500));
                assert_eq!(error_name, "AI_APICallError");
                assert_eq!(
                    message,
                    "AI_APICallError (500: Internal Server Error): upstream boom"
                );
            }
            other => panic!("expected ApiFailure, got {other:?}"),
        }
    }

    #[test]
    fn api_failure_message_strips_request_body() {
        let line = r#"ERROR 2026-04-21T19:30:26 +49275ms service=llm providerID=zai-coding-plan modelID=glm-5.1 session.id=ses_24e7a9448ffeyo7E2zcOHvsiOn small=false agent=explore mode=subagent error={"error":{"name":"AI_APICallError","url":"https://api.z.ai/api/coding/paas/v4/chat/completions","requestBodyValues":{"model":"glm-5.1","max_tokens":32000,"thinking":{"type":"enabled","clear_thinking":false},"messages":[{"role":"system","content":"You are a file search specialist with a very long system prompt that goes on and on"}]},"statusCode":400,"responseBody":"{\"error\":{\"code\":\"invalid_request\",\"message\":\"model does not support thinking\"}}"}}"#;
        let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
            panic!("expected Structured");
        };
        match classify(&record) {
            LogClassification::ApiFailure {
                status_code,
                error_name,
                message,
                ..
            } => {
                assert_eq!(status_code, Some(400));
                assert_eq!(error_name, "AI_APICallError");
                assert!(
                    !message.contains("system prompt"),
                    "message should not contain the request body: {message}"
                );
                assert!(
                    !message.contains("requestBodyValues"),
                    "message should not contain requestBodyValues: {message}"
                );
                assert!(
                    message.contains("model does not support thinking"),
                    "message should contain provider error: {message}"
                );
                assert_eq!(
                    message,
                    "AI_APICallError (400: Bad Request): model does not support thinking"
                );
            }
            other => panic!("expected ApiFailure, got {other:?}"),
        }
    }

    #[test]
    fn auth_failure_message_is_concise() {
        let line = r#"ERROR 2026-04-15T19:26:02 +5ms service=llm error={"error":{"name":"AuthenticationError","message":"Invalid API key"}}"#;
        let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
            panic!("expected Structured");
        };
        match classify(&record) {
            LogClassification::AuthFailure { message } => {
                assert_eq!(message, "AuthenticationError: Invalid API key");
            }
            other => panic!("expected AuthFailure, got {other:?}"),
        }
    }

    #[test]
    fn classifies_uncaught_from_type_error_record() {
        let line = r#"ERROR 2026-04-15T21:28:30 +33ms service=default name=TypeError message=U.split is not a function stack=TypeError: U.split is not a function fatal"#;
        let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
            panic!("expected Structured");
        };
        match classify(&record) {
            LogClassification::UncaughtError { .. } => {}
            other => panic!("expected UncaughtError, got {other:?}"),
        }
    }

    #[test]
    fn classifies_unknown_records_as_unclassified() {
        let line = "INFO 2026-04-15T21:28:30 +0ms service=default msg=hello";
        let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
            panic!("expected Structured");
        };
        assert_eq!(classify(&record), LogClassification::Unclassified);
    }

    #[test]
    fn classify_raw_matches_ansi_error_prefix() {
        let ansi_line = "\u{1b}[91m\u{1b}[1mError: \u{1b}[0mUnexpected error, check log file";
        match classify_raw(ansi_line) {
            LogClassification::UncaughtError { raw_text } => assert_eq!(raw_text, ansi_line),
            other => panic!("expected UncaughtError, got {other:?}"),
        }
    }

    #[test]
    fn classify_raw_ignores_plain_text() {
        assert_eq!(
            classify_raw("just some chatter"),
            LogClassification::Unclassified,
        );
    }

    fn parse_new_format_record(line: &str) -> OpenCodeLogRecord {
        let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
            panic!("expected Structured for {line}");
        };
        record
    }

    #[test]
    fn new_format_classifies_session_created() {
        let record = parse_new_format_record(
            "timestamp=2026-06-10T16:11:27.352Z level=INFO service=session id=ses_primary title=Primary session created",
        );

        match classify(&record) {
            LogClassification::SessionCreated { id, parent_id } => {
                assert_eq!(id, "ses_primary");
                assert_eq!(parent_id, None);
            }
            other => panic!("expected SessionCreated, got {other:?}"),
        }
    }

    #[test]
    fn new_format_classifies_session_created_subagent() {
        let record = parse_new_format_record(
            "timestamp=2026-06-10T16:11:28.000Z level=INFO service=session id=ses_child parentID=ses_parent title=Child task created",
        );

        match classify(&record) {
            LogClassification::SessionCreated { id, parent_id } => {
                assert_eq!(id, "ses_child");
                assert_eq!(parent_id.as_deref(), Some("ses_parent"));
            }
            other => panic!("expected SessionCreated, got {other:?}"),
        }
    }

    #[test]
    fn new_format_classifies_llm_call() {
        let record = parse_new_format_record(
            "timestamp=2026-06-10T16:11:29.000Z level=INFO service=llm providerID=kimi-for-coding modelID=k2p6 session.id=ses_primary small=false agent=build mode=primary stream",
        );

        match classify(&record) {
            LogClassification::LlmCall {
                provider_id,
                model_id,
                mode,
                is_stream,
            } => {
                assert_eq!(provider_id, "kimi-for-coding");
                assert_eq!(model_id, "k2p6");
                assert_eq!(mode, "primary");
                assert!(is_stream);
            }
            other => panic!("expected LlmCall, got {other:?}"),
        }
    }

    #[test]
    fn new_format_classifies_step_loop() {
        let record = parse_new_format_record(
            "timestamp=2026-06-10T16:11:30.000Z level=INFO service=session.prompt session.id=ses_primary step=2 logSpan.http.span.4=55ms message=loop",
        );

        match classify(&record) {
            LogClassification::StepLoop { session_id, step } => {
                assert_eq!(session_id, "ses_primary");
                assert_eq!(step, 2);
            }
            other => panic!("expected StepLoop, got {other:?}"),
        }
    }

    #[test]
    fn new_format_classifies_step_exit() {
        let record = parse_new_format_record(
            "timestamp=2026-06-10T16:11:31.000Z level=INFO service=session.prompt session.id=ses_primary logSpan.http.span.4=7437ms message=\"exiting loop\"",
        );

        match classify(&record) {
            LogClassification::StepExit { session_id } => {
                assert_eq!(session_id, "ses_primary");
            }
            other => panic!("expected StepExit, got {other:?}"),
        }
    }

    #[test]
    fn new_format_classifies_permission_evaluated() {
        let record = parse_new_format_record(
            r#"timestamp=2026-06-10T16:11:32.000Z level=INFO service=permission permission=task pattern=general action={"permission":"*","action":"allow","pattern":"*"} message=evaluated"#,
        );

        match classify(&record) {
            LogClassification::PermissionEvaluated {
                permission,
                pattern,
                action,
            } => {
                assert_eq!(permission, "task");
                assert_eq!(pattern, "general");
                assert_eq!(
                    action,
                    r#"{"permission":"*","action":"allow","pattern":"*"}"#
                );
            }
            other => panic!("expected PermissionEvaluated, got {other:?}"),
        }
    }

    #[test]
    fn new_format_serviceless_classifies_step_loop() {
        let record = parse_new_format_record(
            "timestamp=2026-06-10T16:11:27.460Z level=INFO run=df5a9474 message=loop session.id=ses_14db step=1",
        );

        match classify(&record) {
            LogClassification::StepLoop { session_id, step } => {
                assert_eq!(session_id, "ses_14db");
                assert_eq!(step, 1);
            }
            other => panic!("expected StepLoop, got {other:?}"),
        }
    }

    #[test]
    fn new_format_serviceless_classifies_llm_call() {
        let record = parse_new_format_record(
            "timestamp=2026-06-10T16:11:27.574Z level=INFO run=df5a9474 message=stream providerID=zai-coding-plan modelID=glm-5.1",
        );

        match classify(&record) {
            LogClassification::LlmCall {
                provider_id,
                model_id,
                mode,
                is_stream,
            } => {
                assert_eq!(provider_id, "zai-coding-plan");
                assert_eq!(model_id, "glm-5.1");
                assert_eq!(mode, "");
                assert!(is_stream);
            }
            other => panic!("expected LlmCall, got {other:?}"),
        }
    }

    #[test]
    fn new_format_serviceless_classifies_permission_evaluated() {
        let record = parse_new_format_record(
            "timestamp=2026-06-10T16:11:31.461Z level=INFO run=df5a9474 message=evaluated permission=glob pattern=general action=allow",
        );

        match classify(&record) {
            LogClassification::PermissionEvaluated {
                permission,
                pattern,
                action,
            } => {
                assert_eq!(permission, "glob");
                assert_eq!(pattern, "general");
                assert_eq!(action, "allow");
            }
            other => panic!("expected PermissionEvaluated, got {other:?}"),
        }
    }

    #[test]
    fn new_format_serviceless_classifies_session_created() {
        let record = parse_new_format_record(
            "timestamp=2026-06-10T16:11:28.000Z level=INFO run=df5a9474 message=created id=ses_primary title=Primary session",
        );

        match classify(&record) {
            LogClassification::SessionCreated { id, parent_id } => {
                assert_eq!(id, "ses_primary");
                assert_eq!(parent_id, None);
            }
            other => panic!("expected SessionCreated, got {other:?}"),
        }
    }

    #[test]
    fn new_format_serviceless_classifies_boot_banner() {
        let record = parse_new_format_record(
            "timestamp=2026-06-10T16:11:27.352Z level=INFO run=df5a9474 version=1.14.48 message=opencode",
        );

        match classify(&record) {
            LogClassification::BootBanner { version } => {
                assert_eq!(version, "1.14.48");
            }
            other => panic!("expected BootBanner, got {other:?}"),
        }
    }

    #[test]
    fn new_format_serviceless_classifies_step_exit() {
        let record = parse_new_format_record(
            "timestamp=2026-06-10T16:11:31.462Z level=INFO run=df5a9474 message=\"exiting loop\" session.id=ses_primary",
        );

        match classify(&record) {
            LogClassification::StepExit { session_id } => {
                assert_eq!(session_id, "ses_primary");
            }
            other => panic!("expected StepExit, got {other:?}"),
        }
    }

    #[test]
    fn new_format_classifies_tracking_as_unclassified() {
        let record = parse_new_format_record(
            "timestamp=2026-06-10T16:11:33.000Z level=INFO service=session message=tracking hash=abc123",
        );

        assert_eq!(classify(&record), LogClassification::Unclassified);
    }

    #[test]
    fn new_format_quoted_message_value() {
        let record = parse_new_format_record(
            "timestamp=2026-06-10T16:11:34.000Z level=INFO service=llm message=\"llm runtime selected\" providerID=kimi-for-coding modelID=k2p6",
        );

        assert_eq!(
            record.tags.get("message").map(String::as_str),
            Some("\"llm runtime selected\""),
        );
        assert_eq!(classify(&record), LogClassification::Unclassified);
    }

    #[test]
    fn new_format_error_with_inline_json() {
        let record = parse_new_format_record(
            r#"timestamp=2026-06-10T16:11:35.000Z level=ERROR service=llm providerID=kimi-for-coding modelID=k2p6 error={"error":{"name":"AI_APICallError","message":"upstream boom","statusCode":500}}"#,
        );

        match classify(&record) {
            LogClassification::ApiFailure {
                status_code,
                error_name,
                message,
                is_fatal,
            } => {
                assert_eq!(status_code, Some(500));
                assert_eq!(error_name, "AI_APICallError");
                assert_eq!(message, "AI_APICallError (500: Internal Server Error): upstream boom");
                assert!(!is_fatal);
            }
            other => panic!("expected ApiFailure, got {other:?}"),
        }
    }

    /// Trailing bare tokens after the last tag become part of that tag's
    /// value rather than a separate `message`. The classifier still picks
    /// up the `fatal` keyword either way.
    #[test]
    fn trailing_bare_token_stays_with_last_tag_value() {
        let line = "ERROR 2026-04-15T21:28:30 +0ms service=default name=TypeError fatal";
        let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
            panic!("expected Structured");
        };
        assert_eq!(
            record.tags.get("name").map(String::as_str),
            Some("TypeError fatal"),
        );
        assert_eq!(record.message, "");
        assert!(matches!(
            classify(&record),
            LogClassification::UncaughtError { .. }
        ));
    }

    #[test]
    fn rate_limit_without_reset_still_classifies() {
        let line = r#"ERROR 2026-04-15T19:26:02 +10ms service=llm error={"error":{"name":"AI_RetryError","reason":"maxRetriesExceeded","errors":[]}}"#;
        let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
            panic!("expected Structured");
        };
        match classify(&record) {
            LogClassification::ApiFailure { is_fatal, .. } => {
                assert!(is_fatal);
            }
            other => panic!("expected ApiFailure, got {other:?}"),
        }
    }

    /// Integration smoke test over the bundled fixtures - guards against
    /// the `err=... failed to load command` nuance regressing silently.
    #[test]
    fn fixture_malformed_assets_each_line_classifies() {
        let fixture = include_str!("../../../../../tests/fixtures/logs/opencode-malformed-assets.txt");
        let mut counts = (0usize, 0usize, 0usize);
        for line in fixture.lines() {
            let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
                panic!("fixture line did not parse: {line}");
            };
            match classify(&record) {
                LogClassification::MalformedAsset {
                    asset_type: AssetType::Command,
                    ..
                } => counts.0 += 1,
                LogClassification::MalformedAsset {
                    asset_type: AssetType::Skill,
                    ..
                } => counts.1 += 1,
                LogClassification::MalformedAsset {
                    asset_type: AssetType::Agent,
                    ..
                } => counts.2 += 1,
                other => panic!("expected MalformedAsset, got {other:?}"),
            }
        }
        assert_eq!(counts, (4, 1, 1));
    }

    #[test]
    fn fixture_usage_cap_classifies() {
        let fixture = include_str!("../../../../../tests/fixtures/logs/opencode-rate-limit.txt");
        let line = fixture
            .lines()
            .next()
            .expect("rate limit fixture has at least one line");
        let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
            panic!("rate limit fixture failed to parse");
        };
        match classify(&record) {
            LogClassification::ProviderLimit { kind, reset_at, .. } => {
                assert_eq!(kind, ProviderLimitKind::UsageCap);
                let reset = reset_at.expect("reset_at should be parsed from fixture");
                assert_eq!(
                    reset.format("%Y-%m-%d %H:%M:%S").to_string(),
                    "2026-04-16 04:18:56",
                );
            }
            other => panic!("expected ProviderLimit, got {other:?}"),
        }
    }

    #[test]
    fn fixture_429_overload_classifies() {
        let fixture = include_str!("../../../../../tests/fixtures/logs/opencode-429-overload.txt");
        let line = fixture
            .lines()
            .next()
            .expect("overload fixture has at least one line");
        let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
            panic!("overload fixture failed to parse");
        };
        match classify(&record) {
            LogClassification::ProviderLimit { kind, status_code, .. } => {
                assert_eq!(status_code, Some(429));
                assert_eq!(kind, ProviderLimitKind::Overloaded);
            }
            other => panic!("expected ProviderLimit, got {other:?}"),
        }
    }

    #[test]
    fn classifies_429_overload_as_overloaded() {
        let line = r#"ERROR 2026-05-15T19:26:02 +3054ms service=llm providerID=kimi-for-coding modelID=k2p6 error={"error":{"name":"AI_APICallError","statusCode":429,"responseBody":"{\"error\":{\"type\":\"rate_limit_error\",\"message\":\"The engine is currently overloaded, please try again later\"}}","isRetryable":true}}"#;
        let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
            panic!("expected Structured");
        };
        match classify(&record) {
            LogClassification::ProviderLimit {
                status_code,
                kind,
                ..
            } => {
                assert_eq!(status_code, Some(429));
                assert_eq!(kind, ProviderLimitKind::Overloaded);
            }
            other => panic!("expected ProviderLimit, got {other:?}"),
        }
    }

    #[test]
    fn classifies_429_throttled_as_rate_limited() {
        let line = r#"ERROR 2026-05-15T19:26:02 +100ms service=llm error={"error":{"name":"AI_APICallError","statusCode":429,"message":"Too many requests"}}"#;
        let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
            panic!("expected Structured");
        };
        match classify(&record) {
            LogClassification::ProviderLimit {
                status_code,
                kind,
                ..
            } => {
                assert_eq!(status_code, Some(429));
                assert_eq!(kind, ProviderLimitKind::RateLimited);
            }
            other => panic!("expected ProviderLimit, got {other:?}"),
        }
    }

    #[test]
    fn classifies_max_retries_exhausted_as_retries_exhausted() {
        let line = r#"ERROR 2026-05-15T19:26:02 +100ms service=llm error={"error":{"name":"AI_RetryError","reason":"maxRetriesExceeded","errors":[{"name":"AI_APICallError","statusCode":429}]}}"#;
        let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
            panic!("expected Structured");
        };
        match classify(&record) {
            LogClassification::ProviderLimit {
                status_code,
                kind,
                ..
            } => {
                assert_eq!(status_code, Some(429));
                assert_eq!(kind, ProviderLimitKind::RetriesExhausted);
            }
            other => panic!("expected ProviderLimit, got {other:?}"),
        }
    }

    /// Kimi reports its billing-cycle usage cap as HTTP 403 with
    /// `type: permission_error` and the phrase "reached your usage limit
    /// for this billing cycle" — a dialect none of the ZAI-style cap needles
    /// (`code 1308`, `exceeded_current_quota_error`, `Usage limit reached`)
    /// match. It must still classify as a terminal usage cap, not a raw
    /// `AI_APICallError` dump.
    #[test]
    fn classifies_kimi_403_billing_cycle_cap_as_usage_cap() {
        let line = r#"ERROR 2026-06-09T18:21:00 +4200ms service=llm providerID=kimi-for-coding modelID=k2p6 error={"error":{"name":"AI_APICallError","statusCode":403,"responseBody":"{\"error\":{\"type\":\"permission_error\",\"message\":\"You've reached your usage limit for this billing cycle. Your quota will be refreshed in the next cycle. Upgrade to get more: https://www.kimi.com/code/console?from=quota-upgrade\"}}","isRetryable":false}}"#;
        let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
            panic!("expected Structured");
        };
        match classify(&record) {
            LogClassification::ProviderLimit {
                status_code,
                kind,
                ..
            } => {
                assert_eq!(status_code, Some(403));
                assert_eq!(kind, ProviderLimitKind::UsageCap);
            }
            other => panic!("expected ProviderLimit, got {other:?}"),
        }
    }

    #[test]
    fn classifies_exceeded_quota_as_usage_cap() {
        let line = r#"ERROR 2026-05-15T19:26:02 +100ms service=llm error={"error":{"name":"AI_APICallError","statusCode":429,"responseBody":"{\"error\":{\"type\":\"exceeded_current_quota_error\",\"message\":\"Quota exceeded\"}}"}}"#;
        let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
            panic!("expected Structured");
        };
        match classify(&record) {
            LogClassification::ProviderLimit {
                status_code,
                kind,
                ..
            } => {
                assert_eq!(status_code, Some(429));
                assert_eq!(kind, ProviderLimitKind::UsageCap);
            }
            other => panic!("expected ProviderLimit, got {other:?}"),
        }
    }

    #[test]
    fn cap_phrase_without_error_tag_is_advisory_api_failure() {
        let line = "ERROR 2026-05-15T19:26:02 +100ms service=llm dummy={} Usage limit reached for k2p6";
        let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
            panic!("expected Structured");
        };
        match classify(&record) {
            LogClassification::ApiFailure {
                status_code,
                message,
                is_fatal,
                ..
            } => {
                assert_eq!(status_code, None);
                assert_eq!(message, "Usage limit reached for k2p6");
                assert!(!is_fatal);
            }
            other => panic!("expected ApiFailure, got {other:?}"),
        }
    }

    #[test]
    fn cap_wins_over_retries_exhausted() {
        let line = r#"ERROR 2026-05-15T19:26:02 +100ms service=llm error={"error":{"name":"AI_RetryError","reason":"maxRetriesExceeded","errors":[{"name":"AI_APICallError","statusCode":429,"responseBody":"{\"error\":{\"code\":\"1308\",\"message\":\"Usage limit reached\"}}"}]}}"#;
        let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
            panic!("expected Structured");
        };
        match classify(&record) {
            LogClassification::ProviderLimit {
                status_code,
                kind,
                ..
            } => {
                assert_eq!(status_code, Some(429));
                assert_eq!(kind, ProviderLimitKind::UsageCap);
            }
            other => panic!("expected ProviderLimit, got {other:?}"),
        }
    }

    #[test]
    fn fixture_uncaught_error_classifies_first_line() {
        let fixture = include_str!("../../../../../tests/fixtures/logs/opencode-uncaught-error.txt");
        let first = fixture
            .lines()
            .next()
            .expect("uncaught error fixture has at least one line");
        let ParsedOpenCodeStderrLine::Structured(record) = parse_line(first) else {
            panic!("first line did not parse: {first}");
        };
        match classify(&record) {
            LogClassification::UncaughtError { .. } => {}
            other => panic!("expected UncaughtError, got {other:?}"),
        }
    }

    #[test]
    fn fixture_mixed_handles_all_shapes() {
        let fixture = include_str!("../../../../../tests/fixtures/logs/opencode-mixed.txt");
        let mut rate_limit_seen = false;
        let mut malformed_seen = false;
        let mut raw_error_seen = false;
        let mut raw_passthrough_seen = false;

        for line in fixture.lines() {
            match parse_line(line) {
                ParsedOpenCodeStderrLine::Structured(record) => match classify(&record) {
                    LogClassification::ProviderLimit { .. } => rate_limit_seen = true,
                    LogClassification::MalformedAsset { .. } => malformed_seen = true,
                    _ => {}
                },
                ParsedOpenCodeStderrLine::RawText(raw) => match classify_raw(&raw) {
                    LogClassification::UncaughtError { .. } => raw_error_seen = true,
                    LogClassification::Unclassified => raw_passthrough_seen = true,
                    _ => {}
                },
            }
        }

        assert!(rate_limit_seen, "rate limit line should classify");
        assert!(malformed_seen, "malformed asset line should classify");
        assert!(raw_error_seen, "raw ANSI error line should classify");
        assert!(
            raw_passthrough_seen,
            "unstructured chatter should be unclassified raw text",
        );
    }

    #[test]
    fn merge_rate_limit_prefers_throttled_and_latest_reset() {
        use chrono::TimeZone;
        let older = Utc.with_ymd_and_hms(2026, 4, 16, 1, 0, 0).unwrap();
        let newer = Utc.with_ymd_and_hms(2026, 4, 16, 4, 0, 0).unwrap();
        let existing = RateLimitInfo {
            is_throttled: Some(false),
            retry_after_ms: Some(1000),
            message: Some("old".into()),
            reset_at: Some(older),
        };
        let incoming = RateLimitInfo {
            is_throttled: Some(true),
            retry_after_ms: Some(5000),
            message: Some("new".into()),
            reset_at: Some(newer),
        };
        let merged = merge_rate_limit(Some(existing), incoming);
        assert_eq!(merged.is_throttled, Some(true));
        assert_eq!(merged.retry_after_ms, Some(5000));
        assert_eq!(merged.message.as_deref(), Some("new"));
        assert_eq!(merged.reset_at, Some(newer));
    }

    #[test]
    fn extract_status_code_finds_json_variant() {
        assert_eq!(extract_status_code(r#""statusCode":429"#), Some(429));
        assert_eq!(extract_status_code(r#""statusCode":500"#), Some(500));
    }

    #[test]
    fn extract_status_code_finds_key_value_variant() {
        assert_eq!(extract_status_code("statusCode=429"), Some(429));
        assert_eq!(extract_status_code("statusCode=503"), Some(503));
    }

    #[test]
    fn extract_status_code_prefers_first_match() {
        // When both patterns appear, the first (JSON) match wins
        let haystack = r#""statusCode":200 statusCode=500"#;
        assert_eq!(extract_status_code(haystack), Some(200));
    }

    #[test]
    fn classifies_api_failure_with_status_description() {
        let line = r#"ERROR 2026-04-15T19:26:02 +100ms service=llm error={"error":{"name":"AI_APICallError","statusCode":502}}"#;
        let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
            panic!("expected Structured");
        };
        match classify(&record) {
            LogClassification::ApiFailure { message, .. } => {
                assert_eq!(message, "AI_APICallError (502: Bad Gateway)");
            }
            other => panic!("expected ApiFailure, got {other:?}"),
        }
    }

    #[test]
    fn overflow_status_code_does_not_produce_bogus_description() {
        let line = r#"ERROR 2026-04-15T19:26:02 +100ms service=llm error={"error":{"name":"AI_APICallError","statusCode":70000,"message":"unknown error"}}"#;
        let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
            panic!("expected Structured");
        };
        match classify(&record) {
            LogClassification::ApiFailure { message, .. } => {
                assert_eq!(message, "AI_APICallError (70000): unknown error");
                assert!(
                    !message.contains("Bad Request"),
                    "overflow status should not pick up a u16 description: {message}"
                );
            }
            other => panic!("expected ApiFailure, got {other:?}"),
        }
    }

    #[test]
    fn extracts_zai_code_from_response_body() {
        let line = r#"ERROR 2026-04-15T19:26:02 +100ms service=llm error={"error":{"name":"AI_APICallError","responseBody":"{\"code\":1301,\"message\":\"internal server error\"}"}}"#;
        let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
            panic!("expected Structured");
        };
        match classify(&record) {
            LogClassification::ApiFailure { message, .. } => {
                assert_eq!(message, "AI_APICallError: internal server error");
            }
            other => panic!("expected ApiFailure, got {other:?}"),
        }
    }

    #[test]
    fn extracts_zai_description_when_message_missing() {
        let line = r#"ERROR 2026-04-15T19:26:02 +100ms service=llm error={"error":{"name":"AI_APICallError","responseBody":"{\"code\":1305}"}}"#;
        let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
            panic!("expected Structured");
        };
        match classify(&record) {
            LogClassification::ApiFailure { message, .. } => {
                assert_eq!(message, "AI_APICallError: 1305: Request timeout");
            }
            other => panic!("expected ApiFailure, got {other:?}"),
        }
    }

    #[test]
    fn preserves_non_json_error_tag() {
        let line = "ERROR 2026-04-15T19:26:02 +100ms service=llm error=AI_APICallError: something went wrong on the server";
        let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
            panic!("expected Structured");
        };
        match classify(&record) {
            LogClassification::ApiFailure { message, .. } => {
                assert_eq!(
                    message,
                    "AI_APICallError: something went wrong on the server"
                );
            }
            other => panic!("expected ApiFailure, got {other:?}"),
        }
    }

    #[test]
    fn malformed_response_body_falls_back_to_raw() {
        let line = r#"ERROR 2026-04-15T19:26:02 +100ms service=llm error={"error":{"name":"AI_APICallError","responseBody":"not json"}}"#;
        let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
            panic!("expected Structured");
        };
        match classify(&record) {
            LogClassification::ApiFailure { message, .. } => {
                assert_eq!(message, "AI_APICallError: not json");
            }
            other => panic!("expected ApiFailure, got {other:?}"),
        }
    }

    #[test]
    fn api_failure_falls_back_to_record_message() {
        let line = "ERROR 2026-04-15T19:26:02 +100ms service=llm dummy=tag AI_APICallError: connection reset";
        let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
            panic!("expected Structured");
        };
        match classify(&record) {
            LogClassification::ApiFailure { message, .. } => {
                assert_eq!(message, "tag AI_APICallError: connection reset");
            }
            other => panic!("expected ApiFailure, got {other:?}"),
        }
    }

    #[test]
    fn extract_status_code_returns_none_for_missing_code() {
        assert_eq!(extract_status_code("no status here"), None);
        assert_eq!(extract_status_code(""), None);
        assert_eq!(extract_status_code("statusCode=99"), None); // too short
        // JSON variant: a 4-digit code must not match the first 3 digits.
        assert_eq!(extract_status_code(r#""statusCode":4291"#), None);
        assert_eq!(extract_status_code(r#""statusCode":9999"#), None);
        // Key-value variant: a 4+ digit run must not match the first 3 digits.
        assert_eq!(extract_status_code("statusCode=4299"), None);
        assert_eq!(extract_status_code("statusCode=9999"), None);
        // Valid 3-digit key-value codes still match (end-of-string or non-digit boundary).
        assert_eq!(extract_status_code("statusCode=429"), Some(429));
        assert_eq!(extract_status_code("statusCode=503 retrying"), Some(503));
        assert_eq!(extract_status_code("other=500"), None); // wrong key
    }

    // ------------------------------------------------------------------
    // Phase-2 lifecycle classifications
    // ------------------------------------------------------------------

    #[test]
    fn classifies_boot_banner() {
        let line = "INFO  2026-05-12T20:00:11 +97ms service=default version=1.14.48 args=[\"run\",\"--format\",\"json\"] process_role=main run_id=48277674-19e5-40b6-b2b5-efa7577f08ea opencode";
        let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
            panic!("expected Structured");
        };
        match classify(&record) {
            LogClassification::BootBanner { version } => {
                assert_eq!(version, "1.14.48");
            }
            other => panic!("expected BootBanner, got {other:?}"),
        }
    }

    #[test]
    fn classifies_session_created_primary() {
        let line = "INFO  2026-05-12T20:00:12 +20ms service=session id=ses_1e23972b3ffe8QLhzuFpWS5bzd slug=happy-panda version=1.14.48 projectID=global directory=/private/tmp/oc-test path=private/tmp/oc-test title=New session created";
        let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
            panic!("expected Structured");
        };
        match classify(&record) {
            LogClassification::SessionCreated { id, parent_id } => {
                assert_eq!(id, "ses_1e23972b3ffe8QLhzuFpWS5bzd");
                assert_eq!(parent_id, None);
            }
            other => panic!("expected SessionCreated, got {other:?}"),
        }
    }

    #[test]
    fn classifies_session_created_subagent() {
        let line = "INFO  2026-05-12T20:05:26 +1ms service=session id=ses_1e234a70dffeOCARJZRL9dhpHT slug=lucky-orchid version=1.14.48 projectID=global directory=/private/tmp/oc-test path=private/tmp/oc-test parentID=ses_1e234af48ffeViMPs5pMk6UhYk title=Count letters in 'banana' created";
        let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
            panic!("expected Structured");
        };
        match classify(&record) {
            LogClassification::SessionCreated { id, parent_id } => {
                assert_eq!(id, "ses_1e234a70dffeOCARJZRL9dhpHT");
                assert_eq!(parent_id, Some("ses_1e234af48ffeViMPs5pMk6UhYk".into()));
            }
            other => panic!("expected SessionCreated, got {other:?}"),
        }
    }

    #[test]
    fn classifies_llm_call_primary() {
        let line = "INFO  2026-05-12T20:00:12 +0ms service=llm providerID=kimi-for-coding modelID=k2p6 session.id=ses_1e23972b3ffe8QLhzuFpWS5bzd small=false agent=build mode=primary stream";
        let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
            panic!("expected Structured");
        };
        match classify(&record) {
            LogClassification::LlmCall {
                provider_id,
                model_id,
                mode,
                is_stream,
            } => {
                assert_eq!(provider_id, "kimi-for-coding");
                assert_eq!(model_id, "k2p6");
                assert_eq!(mode, "primary");
                assert!(is_stream);
            }
            other => panic!("expected LlmCall, got {other:?}"),
        }
    }

    #[test]
    fn classifies_llm_call_subagent() {
        let line = "INFO  2026-05-12T20:05:26 +1ms service=llm providerID=opencode modelID=claude-haiku-4-5 session.id=ses_1e234a70dffeOCARJZRL9dhpHT small=false agent=general mode=subagent stream";
        let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
            panic!("expected Structured");
        };
        match classify(&record) {
            LogClassification::LlmCall { mode, .. } => {
                assert_eq!(mode, "subagent");
            }
            other => panic!("expected LlmCall, got {other:?}"),
        }
    }

    #[test]
    fn llm_stream_error_still_classifies_as_api_failure() {
        // "stream error" must NOT be classified as LlmCall; the existing
        // LLM-failure path must win.
        let line = r#"ERROR 2026-05-12T20:02:20 +1967ms service=llm providerID=kimi-for-coding modelID=k2p6 session.id=ses_1e237a304ffeqwr10bXJSRYGHJ small=false agent=build mode=primary error={"error":{"name":"AI_APICallError","statusCode":429}} stream error"#;
        let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
            panic!("expected Structured");
        };
        match classify(&record) {
            LogClassification::ProviderLimit { .. } | LogClassification::ApiFailure { .. } => {}
            other => panic!("expected ApiFailure/ProviderLimit, got {other:?}"),
        }
    }

    #[test]
    fn classifies_step_loop() {
        let line = "INFO  2026-05-12T20:00:12 +0ms service=session.prompt session.id=ses_1e23972b3ffe8QLhzuFpWS5bzd step=0 logSpan.http.span.4=55ms loop";
        let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
            panic!("expected Structured");
        };
        match classify(&record) {
            LogClassification::StepLoop { session_id, step } => {
                assert_eq!(session_id, "ses_1e23972b3ffe8QLhzuFpWS5bzd");
                assert_eq!(step, 0);
            }
            other => panic!("expected StepLoop, got {other:?}"),
        }
    }

    #[test]
    fn classifies_step_exit() {
        let line = "INFO  2026-05-12T20:00:19 +1ms service=session.prompt session.id=ses_1e23972b3ffe8QLhzuFpWS5bzd logSpan.http.span.4=7437ms exiting loop";
        let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
            panic!("expected Structured");
        };
        match classify(&record) {
            LogClassification::StepExit { session_id } => {
                assert_eq!(session_id, "ses_1e23972b3ffe8QLhzuFpWS5bzd");
            }
            other => panic!("expected StepExit, got {other:?}"),
        }
    }

    #[test]
    fn classifies_permission_evaluated() {
        let line = r#"INFO  2026-05-12T20:05:26 +160ms service=permission permission=task pattern=general action={"permission":"*","action":"allow","pattern":"*"} evaluated"#;
        let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
            panic!("expected Structured");
        };
        match classify(&record) {
            LogClassification::PermissionEvaluated {
                permission,
                pattern,
                action,
            } => {
                assert_eq!(permission, "task");
                assert_eq!(pattern, "general");
                assert_eq!(
                    action,
                    r#"{"permission":"*","action":"allow","pattern":"*"}"#
                );
            }
            other => panic!("expected PermissionEvaluated, got {other:?}"),
        }
    }

    #[test]
    fn classifies_http_response() {
        let line = "INFO  2026-05-12T20:05:54 +0ms service=default http.method=POST http.url=/session/ses_1e2343b5cffeGOb3bcdTjvh1wZ/message http.status=500 logSpan.http.span.4=99ms Sent HTTP response";
        let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
            panic!("expected Structured");
        };
        match classify(&record) {
            LogClassification::HttpResponse {
                method,
                url,
                status,
                duration_ms,
            } => {
                assert_eq!(method, "POST");
                assert_eq!(url, "/session/ses_1e2343b5cffeGOb3bcdTjvh1wZ/message");
                assert_eq!(status, 500);
                assert_eq!(duration_ms, 99);
            }
            other => panic!("expected HttpResponse, got {other:?}"),
        }
    }

    #[test]
    fn http_response_without_span_tag_zeroes_duration() {
        let line = "INFO  2026-05-12T20:05:54 +0ms service=default http.method=GET http.url=/health http.status=200 Sent HTTP response";
        let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
            panic!("expected Structured");
        };
        match classify(&record) {
            LogClassification::HttpResponse { duration_ms, .. } => {
                assert_eq!(duration_ms, 0);
            }
            other => panic!("expected HttpResponse, got {other:?}"),
        }
    }

    #[test]
    fn unknown_default_service_is_unclassified() {
        // A service=default line that is neither boot banner nor HTTP response
        let line = "INFO  2026-05-12T20:00:11 +0ms service=default foo=bar some other text";
        let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
            panic!("expected Structured");
        };
        assert_eq!(classify(&record), LogClassification::Unclassified);
    }

    #[test]
    fn non_json_error_tag_truncation_respects_char_boundaries() {
        use std::collections::BTreeMap;

        // 496 ASCII bytes, then a 4-byte emoji, then more ASCII. Byte 497
        // falls inside the emoji, so a byte-index slice at 497 would panic.
        let prefix = "x".repeat(496);
        let error_tag = format!("{prefix}😀AI_APICallError: something went wrong");
        assert!(error_tag.len() > 500);
        let raw = error_tag.clone();
        let record = OpenCodeLogRecord {
            level: LogLevel::Error,
            timestamp: Utc::now(),
            delta_ms: 0,
            tags: BTreeMap::from([
                ("service".into(), "llm".into()),
                ("error".into(), error_tag),
            ]),
            message: String::new(),
            raw,
        };
        match classify(&record) {
            LogClassification::ApiFailure { message, .. } => {
                assert!(message.starts_with(&prefix));
                assert!(message.contains("😀"));
                assert!(message.ends_with("..."));
            }
            other => panic!("expected ApiFailure, got {other:?}"),
        }
    }

    /// Captured verbatim from OpenCode 1.18.4 (2026-07-27) when a prompt
    /// pinned a provider id that no longer exists. The stdout NDJSON reported
    /// only `UnknownError` / "Unexpected server error"; this stderr record was
    /// the sole carrier of the actionable text and used to be dropped.
    #[test]
    fn new_format_failed_record_surfaces_provider_error() {
        let record = parse_new_format_record(
            r#"timestamp=2026-07-28T00:36:54.753Z level=ERROR run=3d79f440 message=failed ref=err_58916eca error="ProviderModelNotFoundError: Model not found: minimax-coding-plan/MiniMax-M3. Did you mean: MiniMax-M3?" cause="ProviderModelNotFoundError: Model not found: minimax-coding-plan/MiniMax-M3. Did you mean: MiniMax-M3?\n    at <anonymous> (/$bunfs/root/chunk-2nxwk333.js:439:92342)\n    at SessionPrompt.getModel (/$bunfs/root/chunk-a7dkkw6a.js:1096:11482)""#,
        );

        match classify(&record) {
            LogClassification::UnclassifiedError { error, reference } => {
                assert_eq!(
                    error,
                    "ProviderModelNotFoundError: Model not found: minimax-coding-plan/MiniMax-M3. Did you mean: MiniMax-M3?"
                );
                assert_eq!(reference.as_deref(), Some("err_58916eca"));
            }
            other => panic!("expected UnclassifiedError, got {other:?}"),
        }
    }

    /// A payload with no stack and no `cause=` must survive intact.
    #[test]
    fn error_headline_keeps_a_short_payload_whole() {
        let record = parse_new_format_record(
            r#"timestamp=2026-07-28T00:36:54.753Z level=ERROR run=3d79f440 message=failed error="Token refresh failed: 401""#,
        );

        match classify(&record) {
            LogClassification::UnclassifiedError { error, reference } => {
                assert_eq!(error, "Token refresh failed: 401");
                assert_eq!(reference, None);
            }
            other => panic!("expected UnclassifiedError, got {other:?}"),
        }
    }

    /// The cap counts chars, not bytes — a multi-byte payload must not panic
    /// on a split codepoint.
    #[test]
    fn error_headline_cap_respects_char_boundaries() {
        let payload = "😀".repeat(400);
        let record = parse_new_format_record(&format!(
            "timestamp=2026-07-28T00:36:54.753Z level=ERROR run=3d79f440 message=failed error={payload}"
        ));

        match classify(&record) {
            LogClassification::UnclassifiedError { error, .. } => {
                assert!(error.ends_with("..."), "expected truncation marker: {error}");
                assert_eq!(error.chars().count(), 300);
            }
            other => panic!("expected UnclassifiedError, got {other:?}"),
        }
    }

    /// The backstop must not hijack records a dedicated classifier owns: a
    /// `failed` record wrapping a usage cap stays terminal.
    #[test]
    fn failed_record_with_usage_cap_envelope_stays_a_provider_limit() {
        let record = parse_new_format_record(
            r#"timestamp=2026-07-28T00:36:54.753Z level=ERROR run=3d79f440 message=failed ref=err_c0ffee error="AI_APICallError: Usage limit reached for 5 hour.""#,
        );

        match classify(&record) {
            LogClassification::ProviderLimit { kind, .. } => {
                assert_eq!(kind, ProviderLimitKind::UsageCap);
            }
            other => panic!("expected ProviderLimit, got {other:?}"),
        }
    }

    /// `error=Aborted` is OpenCode's cancellation sentinel — it fires on every
    /// Ctrl+C and would otherwise emit a warning for ordinary control flow.
    #[test]
    fn error_level_abort_sentinel_stays_unclassified() {
        let record = parse_new_format_record(
            "timestamp=2026-07-28T00:36:54.753Z level=ERROR run=3d79f440 message=process session.id=ses_a error=Aborted",
        );

        assert!(matches!(classify(&record), LogClassification::Unclassified));
    }

    /// The backstop is gated on `ERROR`; a WARN record carrying the same tag
    /// stays quiet.
    #[test]
    fn warn_level_error_payload_stays_unclassified() {
        let record = parse_new_format_record(
            r#"timestamp=2026-07-28T00:36:54.753Z level=WARN run=3d79f440 message=retrying error="transient blip""#,
        );

        assert!(matches!(classify(&record), LogClassification::Unclassified));
    }
