use std::collections::HashMap;
use std::io::Write;

use chrono::Utc;
use serde_json::Value;

use super::StreamProtocol;
use super::semantic::SemanticEvent;
use super::summary::StreamExecutionSummary;
use crate::events::{AgenticEvent, EnvironmentContext, EventMeta};
use crate::reporting::paths;

/// Convert a `StreamExecutionSummary` into an `EventMeta` suitable for JSONL logging.
///
/// The resulting `EventMeta` has:
/// - `event = SessionEnd`
/// - `extra.synthetic = true`
/// - `extra.synthetic_kind = "stream_wrapper_summary"`
/// - `extra.stream_protocol` set to the protocol name
/// - Token usage, cost, duration, and other fields mapped into `extra`
pub fn summary_to_event_meta(
    summary: &StreamExecutionSummary,
    protocol: StreamProtocol,
    env: &EnvironmentContext,
) -> EventMeta {
    summary_to_event_meta_with_context(summary, protocol, env, None)
}

/// Convert a `StreamExecutionSummary` into an `EventMeta` with optional
/// composition context merged into `extra`.
pub fn summary_to_event_meta_with_context(
    summary: &StreamExecutionSummary,
    protocol: StreamProtocol,
    env: &EnvironmentContext,
    context_extra: Option<&HashMap<String, Value>>,
) -> EventMeta {
    let mut extra = HashMap::new();

    // Merge caller-provided context (e.g. composition metadata).
    if let Some(ctx) = context_extra {
        for (key, value) in ctx {
            extra.insert(key.clone(), value.clone());
        }
    }

    // Synthetic markers
    extra.insert("synthetic".into(), Value::Bool(true));
    extra.insert(
        "synthetic_kind".into(),
        Value::String("stream_wrapper_summary".into()),
    );

    // Protocol
    let protocol_str = match protocol {
        StreamProtocol::StreamJson => "stream-json",
        StreamProtocol::Ndjson => "ndjson",
        StreamProtocol::Jsonl => "jsonl",
        StreamProtocol::WireJsonRpc => "wire-json-rpc",
    };
    extra.insert("stream_protocol".into(), Value::String(protocol_str.into()));

    // Model
    if let Some(model) = &summary.model {
        extra.insert("model".into(), Value::String(model.clone()));
    }

    // Token usage
    if let Some(usage) = &summary.token_usage {
        let mut usage_map = serde_json::Map::new();
        if let Some(v) = usage.input {
            usage_map.insert("input".into(), Value::Number(v.into()));
        }
        if let Some(v) = usage.output {
            usage_map.insert("output".into(), Value::Number(v.into()));
        }
        if let Some(v) = usage.total {
            usage_map.insert("total".into(), Value::Number(v.into()));
        }
        if let Some(v) = usage.cache_read {
            usage_map.insert("cache_read".into(), Value::Number(v.into()));
        }
        extra.insert("token_usage".into(), Value::Object(usage_map));
    }

    // Cost
    if let Some(cost) = summary.cost_usd
        && let Some(n) = serde_json::Number::from_f64(cost)
    {
        extra.insert("cost_usd".into(), Value::Number(n));
    }

    // Duration
    if let Some(ms) = summary.duration_ms {
        extra.insert("duration_ms".into(), Value::Number(ms.into()));
    }
    if let Some(ms) = summary.duration_api_ms {
        extra.insert("duration_api_ms".into(), Value::Number(ms.into()));
    }

    // Exit code
    extra.insert("exit_code".into(), Value::Number(summary.exit_code.into()));

    // Provider status
    if let Some(status) = &summary.provider_status {
        extra.insert("provider_status".into(), Value::String(status.clone()));
    }

    // Tool calls
    if let Some(tc) = summary.tool_calls {
        extra.insert("tool_calls".into(), Value::Number(tc.into()));
    }

    // Permission activity counters (Codex today; omitted when absent)
    if let Some(pp) = summary.permission_prompts {
        extra.insert("permission_prompts".into(), Value::Number(pp.into()));
    }
    if let Some(uip) = summary.user_input_prompts {
        extra.insert("user_input_prompts".into(), Value::Number(uip.into()));
    }

    let mut provider_summary = serde_json::Map::new();
    if let Some(raw_summary) = &summary.raw_summary {
        provider_summary.insert("raw_summary".into(), raw_summary.clone());
    }
    if let Some(rate_limit) = &summary.rate_limit
        && let Ok(value) = serde_json::to_value(rate_limit)
    {
        provider_summary.insert("rate_limit".into(), value);
    }
    if let Some(context_usage) = &summary.context_usage
        && let Ok(value) = serde_json::to_value(context_usage)
    {
        provider_summary.insert("context_usage".into(), value);
    }
    if let Some(stderr_diagnostics) = &summary.stderr_diagnostics
        && let Ok(value) = serde_json::to_value(stderr_diagnostics)
    {
        provider_summary.insert("stderr_diagnostics".into(), value);
    }
    if !provider_summary.is_empty() {
        extra.insert("provider_summary".into(), Value::Object(provider_summary));
    }

    if !summary.badges.is_empty()
        && let Ok(value) = serde_json::to_value(&summary.badges)
    {
        extra.insert("badges".into(), value);
    }

    EventMeta {
        provider: summary.provider,
        event: AgenticEvent::SessionEnd,
        timestamp: Utc::now(),
        session_id: summary.session_id.clone(),
        cwd: None,
        tool_name: None,
        tool_input: None,
        tool_response: None,
        error: summary.error_message.clone(),
        prompt: None,
        agent_type: None,
        notification_type: None,
        notification_message: None,
        extra,
        env: env.clone(),
    }
}

/// Convert a [`SemanticEvent`] into an [`EventMeta`] suitable for JSONL logging.
///
/// The resulting `EventMeta`:
///
/// - Uses [`AgenticEvent::Notification`] as the envelope event so the row
///   doesn't collide with the canonical `SessionEnd` summary row.
/// - Uses `provider` for the [`EventMeta::provider`] slot (semantic typed
///   variants don't carry the provider inline; callers already know which
///   provider's stream they're reading).
/// - Stores the full serialized semantic event under `extra["semantic_event"]`,
///   preserving [`SemanticEvent::ProviderExtension::payload`] untouched for
///   downstream fidelity.
/// - Marks the row with `extra.synthetic = true`,
///   `extra.synthetic_kind = "stream_semantic_event"`, and
///   `extra.semantic_kind = <kind_str>`.
/// - Populates one-to-one `EventMeta` slots where they fit
///   (`tool_name`, `tool_input`, `tool_response`, `session_id`, `error`,
///   `notification_type`, `notification_message`) so existing JSONL/SQLite
///   ingest columns stay queryable.
/// - Merges `context_extra` (composition metadata, etc.) into `extra` the
///   same way [`summary_to_event_meta_with_context`] does.
pub fn semantic_event_to_event_meta(
    event: &SemanticEvent,
    provider: crate::events::Provider,
    env: &EnvironmentContext,
    context_extra: Option<&HashMap<String, Value>>,
) -> EventMeta {
    let mut extra: HashMap<String, Value> = HashMap::new();

    if let Some(ctx) = context_extra {
        for (key, value) in ctx {
            extra.insert(key.clone(), value.clone());
        }
    }

    extra.insert("synthetic".into(), Value::Bool(true));
    extra.insert(
        "synthetic_kind".into(),
        Value::String("stream_semantic_event".into()),
    );
    extra.insert(
        "semantic_kind".into(),
        Value::String(event.kind_str().into()),
    );

    let serialized = serde_json::to_value(event).unwrap_or(Value::Null);
    extra.insert("semantic_event".into(), serialized);

    let parts = destructure_semantic(event);

    // ProviderExtension carries its own provider; prefer that over the caller-
    // supplied value when present. All other variants use the caller's provider.
    let resolved_provider = match event {
        SemanticEvent::ProviderExtension { provider: p, .. } => *p,
        _ => provider,
    };

    EventMeta {
        provider: resolved_provider,
        event: parts.event,
        timestamp: Utc::now(),
        session_id: parts.session_id,
        cwd: None,
        tool_name: parts.tool_name,
        tool_input: parts.tool_input,
        tool_response: parts.tool_response,
        error: parts.error,
        prompt: None,
        agent_type: None,
        notification_type: parts.notification_type,
        notification_message: parts.notification_message,
        extra,
        env: env.clone(),
    }
}

/// Slotted projection of a [`SemanticEvent`] onto the [`EventMeta`] columns.
struct EventMetaSlots {
    event: AgenticEvent,
    session_id: Option<String>,
    tool_name: Option<String>,
    tool_input: Option<Value>,
    tool_response: Option<Value>,
    error: Option<String>,
    notification_type: Option<String>,
    notification_message: Option<String>,
}

impl EventMetaSlots {
    fn simple(event: AgenticEvent) -> Self {
        Self {
            event,
            session_id: None,
            tool_name: None,
            tool_input: None,
            tool_response: None,
            error: None,
            notification_type: None,
            notification_message: None,
        }
    }
}

fn destructure_semantic(event: &SemanticEvent) -> EventMetaSlots {
    match event {
        SemanticEvent::SessionStart {
            session_id, model, ..
        } => EventMetaSlots {
            event: AgenticEvent::SessionStart,
            session_id: session_id.clone(),
            notification_type: Some("session_start".into()),
            notification_message: model.clone(),
            ..EventMetaSlots::simple(AgenticEvent::SessionStart)
        },
        SemanticEvent::TurnStart { .. } => EventMetaSlots::simple(AgenticEvent::BeforePrompt),
        SemanticEvent::TurnComplete { .. } => EventMetaSlots::simple(AgenticEvent::TurnComplete),
        SemanticEvent::OutputText { .. } | SemanticEvent::Reasoning { .. } => {
            EventMetaSlots::simple(AgenticEvent::Notification)
        }
        SemanticEvent::ToolCall {
            id, name, input, ..
        } => EventMetaSlots {
            event: AgenticEvent::BeforeTool,
            session_id: id.clone(),
            tool_name: name.clone(),
            tool_input: input.clone(),
            ..EventMetaSlots::simple(AgenticEvent::BeforeTool)
        },
        SemanticEvent::ToolResult {
            id, name, output, ..
        } => EventMetaSlots {
            event: AgenticEvent::AfterTool,
            session_id: id.clone(),
            tool_name: name.clone(),
            tool_response: output.clone(),
            ..EventMetaSlots::simple(AgenticEvent::AfterTool)
        },
        SemanticEvent::PermissionRequest {
            kind, tool_name, ..
        } => EventMetaSlots {
            event: AgenticEvent::PermissionRequest,
            tool_name: tool_name.clone(),
            notification_type: kind.clone(),
            ..EventMetaSlots::simple(AgenticEvent::PermissionRequest)
        },
        SemanticEvent::SubagentStart { id, name, .. } => EventMetaSlots {
            event: AgenticEvent::SubagentStart,
            session_id: id.clone(),
            tool_name: name.clone(),
            ..EventMetaSlots::simple(AgenticEvent::SubagentStart)
        },
        SemanticEvent::SubagentStop { id, name, .. } => EventMetaSlots {
            event: AgenticEvent::SubagentStop,
            session_id: id.clone(),
            tool_name: name.clone(),
            ..EventMetaSlots::simple(AgenticEvent::SubagentStop)
        },
        SemanticEvent::FileChange { path, .. } => EventMetaSlots {
            event: AgenticEvent::Notification,
            notification_type: Some("file_change".into()),
            notification_message: path.clone(),
            ..EventMetaSlots::simple(AgenticEvent::Notification)
        },
        SemanticEvent::PlanUpdate { message, .. } => EventMetaSlots {
            event: AgenticEvent::Notification,
            notification_type: Some("plan_update".into()),
            notification_message: message.clone(),
            ..EventMetaSlots::simple(AgenticEvent::Notification)
        },
        SemanticEvent::Info { message, .. } => EventMetaSlots {
            event: AgenticEvent::Notification,
            notification_type: Some("info".into()),
            notification_message: Some(message.clone()),
            ..EventMetaSlots::simple(AgenticEvent::Notification)
        },
        SemanticEvent::Warning { message, .. } => EventMetaSlots {
            event: AgenticEvent::Notification,
            error: Some(message.clone()),
            notification_type: Some("warning".into()),
            notification_message: Some(message.clone()),
            ..EventMetaSlots::simple(AgenticEvent::Notification)
        },
        SemanticEvent::Error {
            message, terminal, ..
        } => {
            let agentic = if *terminal {
                AgenticEvent::TurnError
            } else {
                AgenticEvent::Notification
            };
            EventMetaSlots {
                event: agentic,
                error: Some(message.clone()),
                notification_type: Some("error".into()),
                notification_message: Some(message.clone()),
                ..EventMetaSlots::simple(agentic)
            }
        }
        SemanticEvent::ProviderExtension { kind, .. } => EventMetaSlots {
            event: AgenticEvent::Notification,
            notification_type: Some(format!("provider_extension:{kind}")),
            ..EventMetaSlots::simple(AgenticEvent::Notification)
        },
    }
}

/// Write a single `EventMeta` to the Claudine JSONL log.
///
/// Uses the same date-partitioned path as dispatch Log actions.
/// This function is for synthetic summary events only — it must NOT
/// trigger user-configured hooks.
pub fn write_summary_event(meta: &EventMeta) -> Result<(), std::io::Error> {
    let path = paths::resolve_file_log_path(None, true)
        .map_err(|e| std::io::Error::other(e.to_string()))?;

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let mut line = serde_json::to_string(meta)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    line.push('\n');

    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)?
        .write_all(line.as_bytes())?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::Provider;
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
        let meta_none =
            summary_to_event_meta_with_context(&summary, StreamProtocol::StreamJson, &env, None);

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
}
