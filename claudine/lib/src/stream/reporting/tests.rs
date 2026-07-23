use super::*;
use crate::provider_id::Provider;
use crate::stream::token_usage::NormalizedTokenUsage;

fn make_test_summary() -> StreamExecutionSummary {
    StreamExecutionSummary {
        provider: Provider::Claude,
        session_id: Some("sess-test".into()),
        model: Some("claude-sonnet-4-20250514".into()),
        assistant_text: "Hello".into(),
        provider_status: Some("end_turn".into()),
        exit_code: 0,
        is_error: false,
        error_kind: None,
        error_message: None,
        duration_ms: Some(12345),
        duration_api_ms: Some(11000),
        num_turns: Some(3),
        token_usage: Some(NormalizedTokenUsage {
            input: Some(1000),
            output: Some(500),
            total: Some(1500),
            cache_read: Some(200),
        }),
        cost_usd: Some(0.0042),
        tool_calls: Some(5),
        permission_prompts: None,
        user_input_prompts: None,
        rate_limit: None,
        context_usage: None,
        badges: Vec::new(),
        raw_summary: None,
        stderr_text: None,
        stderr_diagnostics: None,
    }
}

fn make_test_env() -> EnvironmentContext {
    EnvironmentContext::default()
}

#[test]
fn summary_to_event_meta_has_synthetic_markers() {
    let summary = make_test_summary();
    let meta = summary_to_event_meta(&summary, StreamProtocol::StreamJson, &make_test_env());

    assert_eq!(meta.event, AgenticEvent::SessionEnd);
    assert_eq!(meta.extra["synthetic"], Value::Bool(true));
    assert_eq!(
        meta.extra["synthetic_kind"],
        Value::String("stream_wrapper_summary".into())
    );
    assert_eq!(
        meta.extra["stream_protocol"],
        Value::String("stream-json".into())
    );
}

#[test]
fn summary_to_event_meta_maps_fields() {
    let summary = make_test_summary();
    let meta = summary_to_event_meta(&summary, StreamProtocol::StreamJson, &make_test_env());

    assert_eq!(meta.provider, Provider::Claude);
    assert_eq!(meta.session_id.as_deref(), Some("sess-test"));
    assert_eq!(
        meta.extra["model"],
        Value::String("claude-sonnet-4-20250514".into())
    );
    assert_eq!(meta.extra["duration_ms"], Value::Number(12345.into()));
    assert_eq!(meta.extra["duration_api_ms"], Value::Number(11000.into()));
    assert_eq!(meta.extra["exit_code"], Value::Number(0.into()));
    assert_eq!(
        meta.extra["provider_status"],
        Value::String("end_turn".into())
    );
    assert_eq!(meta.extra["tool_calls"], Value::Number(5.into()));
}

#[test]
fn summary_to_event_meta_maps_provider_summary_fields() {
    let mut summary = make_test_summary();
    summary.rate_limit = Some(crate::stream::summary::RateLimitInfo {
        is_throttled: Some(true),
        retry_after_ms: Some(1500),
        message: Some("Slow down".into()),
        reset_at: None,
    });
    summary.context_usage = Some(crate::stream::summary::ContextUsage {
        used: Some(90),
        total: Some(100),
        percent: Some(90.0),
    });
    summary.raw_summary = Some(serde_json::json!({"stop_reason":"end_turn"}));

    let meta = summary_to_event_meta(&summary, StreamProtocol::StreamJson, &make_test_env());

    assert_eq!(
        meta.extra["provider_summary"]["raw_summary"]["stop_reason"],
        Value::String("end_turn".into())
    );
    assert_eq!(
        meta.extra["provider_summary"]["rate_limit"]["is_throttled"],
        Value::Bool(true)
    );
    assert_eq!(
        meta.extra["provider_summary"]["context_usage"]["percent"],
        Value::from(90.0)
    );
}

#[test]
fn summary_to_event_meta_serializes_stderr_diagnostics_into_provider_summary() {
    use crate::stream::summary::StderrDiagnostics;
    use chrono::TimeZone;
    let mut summary = make_test_summary();
    summary.stderr_diagnostics = Some(StderrDiagnostics {
        log_records_parsed: 5,
        rate_limit_events: 1,
        malformed_asset_events: 2,
        api_failures: 0,
        auth_failures: 0,
        uncaught_errors: 1,
        rate_limit_reset_at: Some(Utc.with_ymd_and_hms(2026, 4, 16, 4, 18, 56).unwrap()),
    });

    let meta = summary_to_event_meta(&summary, StreamProtocol::StreamJson, &make_test_env());

    let diagnostics = &meta.extra["provider_summary"]["stderr_diagnostics"];
    assert_eq!(diagnostics["log_records_parsed"], Value::Number(5.into()));
    assert_eq!(diagnostics["rate_limit_events"], Value::Number(1.into()));
    assert_eq!(
        diagnostics["malformed_asset_events"],
        Value::Number(2.into())
    );
    assert_eq!(diagnostics["uncaught_errors"], Value::Number(1.into()));
    assert!(diagnostics["rate_limit_reset_at"].is_string());
}

#[test]
fn summary_to_event_meta_omits_stderr_diagnostics_when_none() {
    let summary = make_test_summary();
    let meta = summary_to_event_meta(&summary, StreamProtocol::StreamJson, &make_test_env());
    // No provider_summary fields at all should be present for the default summary.
    assert!(!meta.extra.contains_key("provider_summary"));
}

#[test]
fn summary_to_event_meta_provider_summary_preserves_raw_rate_and_context_with_diagnostics() {
    use crate::stream::summary::StderrDiagnostics;
    let mut summary = make_test_summary();
    summary.raw_summary = Some(serde_json::json!({"stop_reason":"end_turn"}));
    summary.rate_limit = Some(crate::stream::summary::RateLimitInfo {
        is_throttled: Some(true),
        retry_after_ms: Some(2500),
        message: Some("Slow down".into()),
        reset_at: None,
    });
    summary.context_usage = Some(crate::stream::summary::ContextUsage {
        used: Some(80),
        total: Some(100),
        percent: Some(80.0),
    });
    summary.stderr_diagnostics = Some(StderrDiagnostics {
        log_records_parsed: 1,
        malformed_asset_events: 1,
        ..Default::default()
    });

    let meta = summary_to_event_meta(&summary, StreamProtocol::StreamJson, &make_test_env());
    let provider_summary = meta.extra["provider_summary"].as_object().unwrap();
    assert!(provider_summary.contains_key("raw_summary"));
    assert!(provider_summary.contains_key("rate_limit"));
    assert!(provider_summary.contains_key("context_usage"));
    assert!(provider_summary.contains_key("stderr_diagnostics"));
    assert_eq!(
        provider_summary["stderr_diagnostics"]["malformed_asset_events"],
        Value::Number(1.into())
    );
}

#[test]
fn summary_to_event_meta_maps_token_usage() {
    let summary = make_test_summary();
    let meta = summary_to_event_meta(&summary, StreamProtocol::StreamJson, &make_test_env());

    let usage = meta.extra["token_usage"].as_object().unwrap();
    assert_eq!(usage["input"], Value::Number(1000.into()));
    assert_eq!(usage["output"], Value::Number(500.into()));
    assert_eq!(usage["total"], Value::Number(1500.into()));
    assert_eq!(usage["cache_read"], Value::Number(200.into()));
}

#[test]
fn summary_to_event_meta_maps_cost() {
    let summary = make_test_summary();
    let meta = summary_to_event_meta(&summary, StreamProtocol::StreamJson, &make_test_env());

    let cost = meta.extra["cost_usd"].as_f64().unwrap();
    assert!((cost - 0.0042).abs() < f64::EPSILON);
}

#[test]
fn missing_optional_fields_omitted() {
    let summary = StreamExecutionSummary::default();
    let meta = summary_to_event_meta(&summary, StreamProtocol::Ndjson, &make_test_env());

    assert!(!meta.extra.contains_key("model"));
    assert!(!meta.extra.contains_key("token_usage"));
    assert!(!meta.extra.contains_key("cost_usd"));
    assert!(!meta.extra.contains_key("provider_status"));
    assert!(!meta.extra.contains_key("tool_calls"));
    // These are always present
    assert!(meta.extra.contains_key("synthetic"));
    assert!(meta.extra.contains_key("exit_code"));
}

#[test]
fn protocol_variants_serialized_correctly() {
    let summary = StreamExecutionSummary::default();
    let env = make_test_env();

    let meta = summary_to_event_meta(&summary, StreamProtocol::Ndjson, &env);
    assert_eq!(
        meta.extra["stream_protocol"],
        Value::String("ndjson".into())
    );

    let meta = summary_to_event_meta(&summary, StreamProtocol::Jsonl, &env);
    assert_eq!(meta.extra["stream_protocol"], Value::String("jsonl".into()));
}

#[test]
fn summary_with_context_merges_composition_metadata() {
    let summary = make_test_summary();
    let env = make_test_env();
    let mut context = HashMap::new();
    context.insert(
        "composition_file_ref".into(),
        Value::String("notes/weekly.md".into()),
    );
    context.insert("composition_mode".into(), Value::String("inline".into()));
    context.insert(
        "composition_source_path".into(),
        Value::String("/tmp/notes/weekly.md".into()),
    );

    let meta = summary_to_event_meta_with_context(
        &summary,
        StreamProtocol::StreamJson,
        &env,
        Some(&context),
        None,
        &[],
        None,
    );

    assert_eq!(meta.event, AgenticEvent::SessionEnd);
    assert_eq!(
        meta.extra["composition_file_ref"],
        Value::String("notes/weekly.md".into())
    );
    assert_eq!(
        meta.extra["composition_mode"],
        Value::String("inline".into())
    );
    assert_eq!(
        meta.extra["composition_source_path"],
        Value::String("/tmp/notes/weekly.md".into())
    );
    // Standard fields still present
    assert_eq!(meta.extra["synthetic"], Value::Bool(true));
    assert_eq!(meta.extra["exit_code"], Value::Number(0.into()));
}

#[test]
fn summary_with_no_context_matches_original() {
    let summary = make_test_summary();
    let env = make_test_env();

    let meta_plain = summary_to_event_meta(&summary, StreamProtocol::StreamJson, &env);
    let meta_none = summary_to_event_meta_with_context(
        &summary,
        StreamProtocol::StreamJson,
        &env,
        None,
        None,
        &[],
        None,
    );

    assert_eq!(meta_plain.extra.len(), meta_none.extra.len());
    assert!(!meta_none.extra.contains_key("composition_file_ref"));
}

#[test]
fn summary_to_event_meta_serializes_badges_when_present() {
    use crate::stream::badges::{BadgeCategory, BadgeSeverity, SessionBadge};
    let mut summary = make_test_summary();
    summary.badges = vec![SessionBadge {
        category: BadgeCategory::Billing,
        severity: BadgeSeverity::Error,
        label: "Billing".into(),
        message: "Insufficient credits".into(),
        remediation_url: Some("https://console.anthropic.com/settings/billing".into()),
    }];
    let meta = summary_to_event_meta(&summary, StreamProtocol::StreamJson, &make_test_env());
    let badges = meta.extra.get("badges").unwrap();
    let arr = badges.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["category"], Value::String("billing".into()));
    assert_eq!(arr[0]["severity"], Value::String("error".into()));
    assert_eq!(arr[0]["label"], Value::String("Billing".into()));
    assert_eq!(
        arr[0]["message"],
        Value::String("Insufficient credits".into())
    );
    assert_eq!(
        arr[0]["remediation_url"],
        Value::String("https://console.anthropic.com/settings/billing".into())
    );
}

#[test]
fn summary_to_event_meta_omits_badges_when_empty() {
    let summary = make_test_summary();
    let meta = summary_to_event_meta(&summary, StreamProtocol::StreamJson, &make_test_env());
    assert!(!meta.extra.contains_key("badges"));
}

/// Drained run signals land as `extra["signals"]` on the summary row
/// with `kind` hoisted beside the full tagged `event` payload.
#[test]
fn summary_to_event_meta_serializes_drained_signals() {
    use chrono::TimeZone;
    use claudine_catalog_types::{DriftObservation, SignalEvent, SignalSource};

    let signals = vec![crate::signals::ObservedSignal {
        event: SignalEvent::ModelCatalogDrift {
            unexpected: vec!["mystery-model".into()],
            missing: vec![],
            observed_via: DriftObservation::Listing,
        },
        source: SignalSource::Wrapper,
        first_seen: Utc.with_ymd_and_hms(2026, 7, 6, 12, 0, 0).unwrap(),
        occurrences: 2,
        context: Default::default(),
    }];

    let meta = summary_to_event_meta_with_context(
        &make_test_summary(),
        StreamProtocol::StreamJson,
        &make_test_env(),
        None,
        None,
        &signals,
        None,
    );

    let rows = meta.extra["signals"].as_array().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["kind"], "model_catalog_drift");
    assert_eq!(rows[0]["source"], "wrapper");
    assert_eq!(rows[0]["occurrences"], 2);
    assert_eq!(rows[0]["first_seen"], "2026-07-06T12:00:00Z");
    assert_eq!(rows[0]["event"]["kind"], "model_catalog_drift");
    assert_eq!(rows[0]["event"]["unexpected"][0], "mystery-model");
}

/// No signals → no `signals` key; a `family_latest` stamp lands under
/// its own key.
#[test]
fn summary_to_event_meta_omits_signals_and_carries_family_latest_stamp() {
    let stamp = crate::model_catalog::FamilyLatestStamp {
        alias: "opus".into(),
        identity_key: "anthropic/claude-opus@4.5".into(),
        family_key: "anthropic/claude-opus".into(),
        artifact_generated_at: "2026-07-06T22:44:37.286219+00:00".into(),
        stale: false,
        age_days: None,
    };

    let meta = summary_to_event_meta_with_context(
        &make_test_summary(),
        StreamProtocol::StreamJson,
        &make_test_env(),
        None,
        None,
        &[],
        Some(&stamp),
    );

    assert!(!meta.extra.contains_key("signals"));
    let value = &meta.extra["family_latest"];
    assert_eq!(value["alias"], "opus");
    assert_eq!(value["identity_key"], "anthropic/claude-opus@4.5");
    assert_eq!(value["stale"], false);
}

#[test]
fn summary_event_includes_permission_counters_when_populated() {
    let summary = StreamExecutionSummary {
        provider: Provider::Codex,
        permission_prompts: Some(2),
        user_input_prompts: Some(1),
        ..Default::default()
    };
    let env = EnvironmentContext::default();
    let meta = summary_to_event_meta(&summary, StreamProtocol::Jsonl, &env);
    assert_eq!(meta.extra["permission_prompts"], Value::Number(2.into()));
    assert_eq!(meta.extra["user_input_prompts"], Value::Number(1.into()));
}

#[test]
fn summary_event_omits_permission_counters_when_absent() {
    let summary = StreamExecutionSummary::default();
    let env = EnvironmentContext::default();
    let meta = summary_to_event_meta(&summary, StreamProtocol::Jsonl, &env);
    assert!(!meta.extra.contains_key("permission_prompts"));
    assert!(!meta.extra.contains_key("user_input_prompts"));
}

mod semantic_event_to_event_meta_tests {
    use super::*;
    use crate::stream::semantic::SemanticErrorKind;
    use serde_json::json;

    fn env() -> EnvironmentContext {
        EnvironmentContext::default()
    }

    #[test]
    fn tool_call_maps_to_before_tool_and_carries_name_input() {
        let event = SemanticEvent::ToolCall {
            name: Some("bash".into()),
            id: Some("t1".into()),
            input: Some(json!({"cmd": "ls"})),
            extra: json!({"provider": "claude"}),
        };
        let meta = semantic_event_to_event_meta(&event, Provider::Claude, &env(), None);
        assert_eq!(meta.event, AgenticEvent::BeforeTool);
        assert_eq!(meta.tool_name.as_deref(), Some("bash"));
        assert_eq!(meta.session_id.as_deref(), Some("t1"));
        assert_eq!(meta.tool_input, Some(json!({"cmd": "ls"})));
        assert_eq!(meta.extra["synthetic"], Value::Bool(true));
        assert_eq!(
            meta.extra["synthetic_kind"],
            Value::String("stream_semantic_event".into())
        );
        assert_eq!(
            meta.extra["semantic_kind"],
            Value::String("tool_call".into())
        );
        assert!(meta.extra.contains_key("semantic_event"));
    }

    #[test]
    fn tool_result_maps_to_after_tool_with_output() {
        let event = SemanticEvent::ToolResult {
            name: Some("bash".into()),
            id: Some("t1".into()),
            status: Some("success".into()),
            exit_code: Some(0),
            output: Some(json!("done")),
            extra: json!({}),
        };
        let meta = semantic_event_to_event_meta(&event, Provider::Claude, &env(), None);
        assert_eq!(meta.event, AgenticEvent::AfterTool);
        assert_eq!(meta.tool_response, Some(json!("done")));
    }

    #[test]
    fn provider_extension_preserves_payload_and_uses_inline_provider() {
        let payload = json!({"deep": {"nested": [1, 2, 3]}});
        let event = SemanticEvent::ProviderExtension {
            provider: Provider::Codex,
            kind: "item.updated".into(),
            payload: payload.clone(),
        };
        // Caller passes Claude, but ProviderExtension carries Codex.
        let meta = semantic_event_to_event_meta(&event, Provider::Claude, &env(), None);
        assert_eq!(meta.provider, Provider::Codex);
        let roundtrip = meta.extra["semantic_event"].clone();
        // Full semantic event is preserved.
        let decoded: SemanticEvent = serde_json::from_value(roundtrip).unwrap();
        match decoded {
            SemanticEvent::ProviderExtension {
                payload: decoded_payload,
                ..
            } => assert_eq!(decoded_payload, payload),
            other => panic!("expected ProviderExtension, got {other:?}"),
        }
    }

    #[test]
    fn error_terminal_maps_to_turn_error() {
        let event = SemanticEvent::Error {
            message: "billing failed".into(),
            terminal: true,
            kind: SemanticErrorKind::Unknown,
            extra: json!({}),
        };
        let meta = semantic_event_to_event_meta(&event, Provider::Claude, &env(), None);
        assert_eq!(meta.event, AgenticEvent::TurnError);
        assert_eq!(meta.error.as_deref(), Some("billing failed"));
    }

    #[test]
    fn warning_maps_to_notification_with_error_field() {
        let event = SemanticEvent::Warning {
            message: "rate limited".into(),
            extra: json!({}),
        };
        let meta = semantic_event_to_event_meta(&event, Provider::Claude, &env(), None);
        assert_eq!(meta.event, AgenticEvent::Notification);
        assert_eq!(meta.error.as_deref(), Some("rate limited"));
        assert_eq!(meta.notification_type.as_deref(), Some("warning"));
    }

    #[test]
    fn subagent_events_map_correctly() {
        let start = SemanticEvent::SubagentStart {
            name: Some("researcher".into()),
            id: Some("sa1".into()),
            extra: json!({}),
        };
        let stop = SemanticEvent::SubagentStop {
            name: Some("researcher".into()),
            id: Some("sa1".into()),
            status: Some("success".into()),
            extra: json!({}),
        };
        let m1 = semantic_event_to_event_meta(&start, Provider::Claude, &env(), None);
        let m2 = semantic_event_to_event_meta(&stop, Provider::Claude, &env(), None);
        assert_eq!(m1.event, AgenticEvent::SubagentStart);
        assert_eq!(m2.event, AgenticEvent::SubagentStop);
        assert_eq!(m1.tool_name.as_deref(), Some("researcher"));
    }

    #[test]
    fn context_extra_merges() {
        let event = SemanticEvent::Info {
            message: "x".into(),
            extra: json!({}),
        };
        let mut ctx: HashMap<String, Value> = HashMap::new();
        ctx.insert(
            "composition_file_ref".into(),
            Value::String("notes.md".into()),
        );
        let meta = semantic_event_to_event_meta(&event, Provider::Claude, &env(), Some(&ctx));
        assert_eq!(
            meta.extra["composition_file_ref"],
            Value::String("notes.md".into())
        );
    }

    #[test]
    fn round_trip_fidelity_via_semantic_event_slot() {
        for event in [
            SemanticEvent::OutputText {
                text: "hi".into(),
                extra: json!({"k": "v"}),
            },
            SemanticEvent::FileChange {
                path: Some("a.rs".into()),
                change_kind: Some("modified".into()),
                extra: json!({}),
            },
            SemanticEvent::PlanUpdate {
                message: Some("step 1".into()),
                extra: json!({}),
            },
        ] {
            let meta = semantic_event_to_event_meta(&event, Provider::Claude, &env(), None);
            let roundtrip = meta.extra["semantic_event"].clone();
            let decoded: SemanticEvent = serde_json::from_value(roundtrip.clone()).unwrap();
            assert_eq!(
                serde_json::to_value(&decoded).unwrap(),
                serde_json::to_value(&event).unwrap()
            );
        }
    }
}
