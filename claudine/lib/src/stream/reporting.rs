use std::collections::HashMap;
use std::io::Write;

use chrono::Utc;
use serde_json::Value;

use super::StreamProtocol;
use super::semantic::SemanticEvent;
use super::summary::StreamExecutionSummary;
use crate::events::{AgenticEvent, EnvironmentContext, EventMeta};
use crate::model_catalog::FamilyLatestStamp;
use crate::reporting::paths;
use crate::signals::ObservedSignal;

/// Convert a `StreamExecutionSummary` into a synthetic `SessionEnd`
/// [`EventMeta`] for JSONL logging.
pub fn summary_to_event_meta(
    summary: &StreamExecutionSummary,
    protocol: StreamProtocol,
    env: &EnvironmentContext,
) -> EventMeta {
    summary_to_event_meta_with_context(summary, protocol, env, None, None, &[], None)
}

/// Convert a `StreamExecutionSummary` into an `EventMeta` with optional
/// composition context merged into `extra`.
///
/// `agent_pid` carries the immediate child PID captured by the wrapper
/// after a successful spawn. Pass `None` for failed-spawn paths or paths
/// that never spawn a provider child. The typed `EventMeta.agent_pid`
/// field is the authoritative location; this builder also mirrors both
/// `claudine_pid` (read from `env`) and `agent_pid` into `extra` so
/// templates and expressions can resolve them alongside the existing
/// stringly-typed wrapper context keys.
///
/// `signals` is the run's drained [`ObservedSignal`] collection; when
/// non-empty it lands as `extra["signals"]` on this summary row ONLY —
/// deliberately not on the per-event `context_extra` channel, which is
/// mirrored onto every live semantic tool row. `family_latest` likewise
/// lands as `extra["family_latest"]` when the run's requested model was
/// a marked rolling alias.
#[allow(clippy::too_many_arguments)]
pub fn summary_to_event_meta_with_context(
    summary: &StreamExecutionSummary,
    protocol: StreamProtocol,
    env: &EnvironmentContext,
    context_extra: Option<&HashMap<String, Value>>,
    agent_pid: Option<u32>,
    signals: &[ObservedSignal],
    family_latest: Option<&FamilyLatestStamp>,
) -> EventMeta {
    let mut extra = HashMap::new();

    if let Some(ctx) = context_extra {
        for (key, value) in ctx {
            extra.insert(key.clone(), value.clone());
        }
    }

    extra.insert("synthetic".into(), Value::Bool(true));
    extra.insert(
        "synthetic_kind".into(),
        Value::String("stream_wrapper_summary".into()),
    );

    let protocol_str = match protocol {
        StreamProtocol::StreamJson => "stream-json",
        StreamProtocol::Ndjson => "ndjson",
        StreamProtocol::Jsonl => "jsonl",
        StreamProtocol::WireJsonRpc => "wire-json-rpc",
        StreamProtocol::Json => "json",
    };
    extra.insert("stream_protocol".into(), Value::String(protocol_str.into()));

    if let Some(model) = &summary.model {
        extra.insert("model".into(), Value::String(model.clone()));
    }

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

    if let Some(cost) = summary.cost_usd
        && let Some(n) = serde_json::Number::from_f64(cost)
    {
        extra.insert("cost_usd".into(), Value::Number(n));
    }

    if let Some(ms) = summary.duration_ms {
        extra.insert("duration_ms".into(), Value::Number(ms.into()));
    }
    if let Some(ms) = summary.duration_api_ms {
        extra.insert("duration_api_ms".into(), Value::Number(ms.into()));
    }

    extra.insert("exit_code".into(), Value::Number(summary.exit_code.into()));

    if let Some(kind) = &summary.error_kind {
        extra.insert("exit_reason".into(), Value::String(kind.clone()));
    }

    if let Some(status) = &summary.provider_status {
        extra.insert("provider_status".into(), Value::String(status.clone()));
    }

    if let Some(tc) = summary.tool_calls {
        extra.insert("tool_calls".into(), Value::Number(tc.into()));
    }

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

    if !signals.is_empty() {
        extra.insert(
            "signals".into(),
            Value::Array(signals.iter().map(observed_signal_value).collect()),
        );
    }

    if let Some(stamp) = family_latest
        && let Ok(value) = serde_json::to_value(stamp)
    {
        extra.insert("family_latest".into(), value);
    }

    // Mirror `claudine_pid` (read from the wrapper-supplied environment)
    // and `agent_pid` into `extra` so templates and expressions can
    // resolve them alongside the existing composition/extra keys. The
    // typed `EnvironmentContext.claudine_pid` and `EventMeta.agent_pid`
    // fields remain authoritative for JSONL and SQL ingest; these
    // mirrors exist only for template/expression bridging.
    if let Some(pid) = env.claudine_pid {
        extra.entry("claudine_pid".to_string()).or_insert(Value::Number(
            serde_json::Number::from(pid),
        ));
    }
    if let Some(pid) = agent_pid {
        extra.entry("agent_pid".to_string()).or_insert(Value::Number(
            serde_json::Number::from(pid),
        ));
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
        agent_pid,
    }
}

/// Project one [`ObservedSignal`] into the summary-row JSONL shape:
/// `kind` is hoisted next to `source`/`occurrences`/`first_seen` so SQL
/// ingest can address it without descending into the payload, while
/// `event` keeps the full kind-tagged [`crate::signals::SignalEvent`].
fn observed_signal_value(signal: &ObservedSignal) -> Value {
    serde_json::json!({
        "kind": <&'static str>::from(signal.event.kind()),
        "source": signal.source,
        "occurrences": signal.occurrences,
        "first_seen": signal.first_seen,
        "event": signal.event,
    })
}

/// Convert a [`SemanticEvent`] into a synthetic [`EventMeta`] for JSONL logging.
///
/// The envelope event is mapped per variant rather than reusing `SessionEnd`
/// so semantic rows don't collide with the canonical summary row. The full
/// serialized event is preserved under `extra["semantic_event"]` so downstream
/// consumers retain [`SemanticEvent::ProviderExtension::payload`] fidelity,
/// while the typed `EventMeta` slots are populated where they fit so existing
/// JSONL/SQLite ingest columns stay queryable.
pub fn semantic_event_to_event_meta(
    event: &SemanticEvent,
    provider: crate::provider_id::Provider,
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
        agent_pid: None,
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
        .map_err(std::io::Error::other)?;

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
mod tests;
