use std::collections::HashMap;
use std::io::Write;

use chrono::Utc;
use serde_json::Value;

use super::StreamProtocol;
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
    let mut extra = HashMap::new();

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
    if !provider_summary.is_empty() {
        extra.insert("provider_summary".into(), Value::Object(provider_summary));
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
            rate_limit: None,
            context_usage: None,
            raw_summary: None,
            stderr_text: None,
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
}
