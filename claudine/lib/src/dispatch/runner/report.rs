use serde_json::{Map, Value};
use tracing::warn;

use crate::actions::{ReportFormat, ReportHandler};
use crate::dispatch::template::interpolate;
use crate::events::EventMeta;

pub(super) fn execute_report(handler: Option<&ReportHandler>, meta: &EventMeta, blocking: bool) {
    let output = match handler {
        Some(handler) => format_report(handler, meta),
        None => format!(
            "[{}] {} ({})",
            meta.event.as_pascal_case(),
            meta.tool_name.as_deref().unwrap_or("-"),
            meta.provider
        ),
    };
    // Route report output to stderr on blocking events to avoid corrupting
    // the machine-facing provider response payload on stdout.
    if blocking {
        eprintln!("{output}");
    } else {
        println!("{output}");
    }
}

pub(super) fn terminal_meta_json(meta: &EventMeta) -> String {
    let mut value = terminal_meta_value(meta);
    super::strip_nulls(&mut value);
    serde_json::to_string(&value).unwrap_or_else(|err| {
        warn!(%err, "serializing terminal event metadata failed; falling back to empty object");
        "{}".to_string()
    })
}

pub(super) fn terminal_meta_value(meta: &EventMeta) -> Value {
    let mut object = Map::new();

    object.insert(
        "provider".to_string(),
        Value::String(meta.provider.as_slug().to_string()),
    );
    object.insert(
        "event".to_string(),
        Value::String(meta.event.as_pascal_case().to_string()),
    );
    object.insert(
        "timestamp".to_string(),
        serde_json::to_value(meta.timestamp).unwrap_or_else(|err| {
            warn!(%err, "serializing event timestamp failed; substituting null");
            Value::Null
        }),
    );

    if let Some(value) = &meta.session_id {
        object.insert("session_id".to_string(), Value::String(value.clone()));
    }
    if let Some(value) = &meta.cwd {
        object.insert("cwd".to_string(), Value::String(value.clone()));
    }
    if let Some(value) = &meta.tool_name {
        object.insert("tool_name".to_string(), Value::String(value.clone()));
    }
    if let Some(value) = &meta.tool_input {
        object.insert("tool_input".to_string(), value.clone());
    }
    if let Some(value) = &meta.tool_response {
        object.insert("tool_response".to_string(), value.clone());
    }
    if let Some(value) = &meta.error {
        object.insert("error".to_string(), Value::String(value.clone()));
    }
    if let Some(value) = &meta.prompt {
        object.insert("prompt".to_string(), Value::String(value.clone()));
    }
    if let Some(value) = &meta.agent_type {
        object.insert("agent_type".to_string(), Value::String(value.clone()));
    }
    if let Some(value) = &meta.notification_type {
        object.insert(
            "notification_type".to_string(),
            Value::String(value.clone()),
        );
    }
    if let Some(value) = &meta.notification_message {
        object.insert(
            "notification_message".to_string(),
            Value::String(value.clone()),
        );
    }

    object.insert(
        "extra".to_string(),
        serde_json::to_value(&meta.extra).unwrap_or_else(|err| {
            warn!(%err, "serializing event `extra` map failed; substituting empty object");
            Value::Object(Map::new())
        }),
    );
    object.insert(
        "env".to_string(),
        serde_json::to_value(&meta.env).unwrap_or_else(|err| {
            warn!(%err, "serializing event `env` failed; substituting null");
            Value::Null
        }),
    );

    Value::Object(object)
}

pub(super) fn format_report(handler: &ReportHandler, meta: &EventMeta) -> String {
    if let Some(template) = &handler.template {
        let mut output = interpolate(template, meta);
        if handler.include_metadata {
            let json = terminal_meta_json(meta);
            output.push(' ');
            output.push_str(&json);
        }
        return output;
    }

    match handler.format {
        ReportFormat::Json => terminal_meta_json(meta),
        ReportFormat::Compact => format!(
            "[{}] {}",
            meta.event.as_pascal_case(),
            meta.tool_name.as_deref().unwrap_or("-")
        ),
        ReportFormat::Text => format!(
            "Event: {}, Provider: {}, Tool: {}",
            meta.event.as_pascal_case(),
            meta.provider,
            meta.tool_name.as_deref().unwrap_or("-")
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::{AgenticEvent, EnvironmentContext, EventMeta};
    use crate::provider::Provider;
    use chrono::Utc;

    use std::collections::HashMap;

    fn meta() -> EventMeta {
        EventMeta {
            provider: Provider::Claude,
            event: AgenticEvent::BeforeTool,
            timestamp: Utc::now(),
            session_id: Some("test-session".to_string()),
            cwd: Some("/tmp".to_string()),
            tool_name: Some("Bash".to_string()),
            tool_input: None,
            tool_response: None,
            error: None,
            prompt: None,
            agent_type: None,
            notification_type: None,
            notification_message: None,
            agent_pid: None,
            extra: HashMap::new(),
            env: EnvironmentContext::default(),
        }
    }

    #[test]
    fn terminal_meta_json_uses_pascal_case_and_omits_none() {
        let json = terminal_meta_json(&meta());
        let value: Value = serde_json::from_str(&json).unwrap();

        assert_eq!(value["event"], "BeforeTool");
        assert_eq!(value["provider"], "claude");
        assert!(value.get("tool_input").is_none());
        assert!(value.get("tool_response").is_none());
        assert!(value.get("error").is_none());
        assert!(value.get("prompt").is_none());
        assert!(value.get("notification_type").is_none());
        assert!(value.get("notification_message").is_none());
        assert!(value["env"].get("git").is_none());
        assert!(value["env"].get("repo").is_none());
        assert!(value["env"].get("primary_language").is_none());
    }

    #[test]
    fn report_json_uses_terminal_serialization() {
        let output = format_report(
            &ReportHandler {
                format: ReportFormat::Json,
                template: None,
                include_metadata: false,
            },
            &meta(),
        );

        let value: Value = serde_json::from_str(&output).unwrap();
        assert_eq!(value["event"], "BeforeTool");
        assert!(value.get("tool_input").is_none());
    }

    #[test]
    fn report_template_resolves_darkmatter_expressions() {
        // Verifies the runner-level template path uses the shared expression
        // engine (Darkmatter) end-to-end: a fallback expression and a simple
        // variable both resolve correctly via format_report.
        let output = format_report(
            &ReportHandler {
                format: ReportFormat::Text,
                template: Some("{{provider}} ran {{tool_name || \"unknown-tool\"}}".to_string()),
                include_metadata: false,
            },
            &meta(),
        );
        assert_eq!(output, "claude ran Bash");
    }
}
