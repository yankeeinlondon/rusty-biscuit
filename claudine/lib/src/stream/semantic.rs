//! Semantic stream event model.
//!
//! A provider-agnostic, typed event model that represents both output deltas
//! (assistant text, reasoning) and structured metadata (tool calls, subagents,
//! file changes, plan updates, diagnostics) emitted during a wrapped
//! non-interactive session.
//!
//! Every successfully-parsed JSONL line from a provider becomes exactly one
//! `SemanticEvent`. Typed variants carry their own fields plus an `extra`
//! [`serde_json::Value`] object for provider-specific spillover. Unknown but
//! parseable events become [`SemanticEvent::ProviderExtension`] so nothing is
//! dropped.
//!
//! Design context lives in `claudine/features/2026-04-14-more-meta-response/`.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::events::Provider;

use super::token_usage::NormalizedTokenUsage;

/// Typed cross-provider stream event.
///
/// See the module docs for the fidelity invariants. In short: every typed
/// variant carries an `extra` JSON object for provider-specific fields, and
/// [`SemanticEvent::ProviderExtension`] is the catch-all for kinds that have
/// not (yet) graduated to a typed variant.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SemanticEvent {
    SessionStart {
        session_id: Option<String>,
        model: Option<String>,
        extra: Value,
    },
    TurnStart {
        extra: Value,
    },
    TurnComplete {
        provider_status: Option<String>,
        token_usage: Option<NormalizedTokenUsage>,
        cost_usd: Option<f64>,
        duration_ms: Option<u64>,
        extra: Value,
    },
    OutputText {
        text: String,
        extra: Value,
    },
    Reasoning {
        text: String,
        extra: Value,
    },
    ToolCall {
        name: Option<String>,
        id: Option<String>,
        input: Option<Value>,
        extra: Value,
    },
    ToolResult {
        name: Option<String>,
        id: Option<String>,
        status: Option<String>,
        exit_code: Option<i32>,
        output: Option<Value>,
        extra: Value,
    },
    PermissionRequest {
        kind: Option<String>,
        tool_name: Option<String>,
        extra: Value,
    },
    SubagentStart {
        name: Option<String>,
        id: Option<String>,
        extra: Value,
    },
    SubagentStop {
        name: Option<String>,
        id: Option<String>,
        status: Option<String>,
        extra: Value,
    },
    FileChange {
        path: Option<String>,
        change_kind: Option<String>,
        extra: Value,
    },
    PlanUpdate {
        message: Option<String>,
        extra: Value,
    },
    Info {
        message: String,
        extra: Value,
    },
    Warning {
        message: String,
        extra: Value,
    },
    Error {
        message: String,
        terminal: bool,
        extra: Value,
    },
    ProviderExtension {
        provider: Provider,
        kind: String,
        payload: Value,
    },
}

impl SemanticEvent {
    /// Stable short identifier for the variant (used for logging, metrics,
    /// and the `extra.semantic_kind` tag in JSONL/SQLite reporting).
    pub fn kind_str(&self) -> &'static str {
        match self {
            SemanticEvent::SessionStart { .. } => "session_start",
            SemanticEvent::TurnStart { .. } => "turn_start",
            SemanticEvent::TurnComplete { .. } => "turn_complete",
            SemanticEvent::OutputText { .. } => "output_text",
            SemanticEvent::Reasoning { .. } => "reasoning",
            SemanticEvent::ToolCall { .. } => "tool_call",
            SemanticEvent::ToolResult { .. } => "tool_result",
            SemanticEvent::PermissionRequest { .. } => "permission_request",
            SemanticEvent::SubagentStart { .. } => "subagent_start",
            SemanticEvent::SubagentStop { .. } => "subagent_stop",
            SemanticEvent::FileChange { .. } => "file_change",
            SemanticEvent::PlanUpdate { .. } => "plan_update",
            SemanticEvent::Info { .. } => "info",
            SemanticEvent::Warning { .. } => "warning",
            SemanticEvent::Error { .. } => "error",
            SemanticEvent::ProviderExtension { .. } => "provider_extension",
        }
    }

    /// Whether this event counts as "activity" for heartbeat silence tracking.
    ///
    /// Session/turn envelope events are not counted; actual model work
    /// (output, reasoning, tools, subagents, diagnostics, extensions) is.
    pub fn is_activity(&self) -> bool {
        matches!(
            self,
            SemanticEvent::OutputText { .. }
                | SemanticEvent::Reasoning { .. }
                | SemanticEvent::ToolCall { .. }
                | SemanticEvent::ToolResult { .. }
                | SemanticEvent::SubagentStart { .. }
                | SemanticEvent::SubagentStop { .. }
                | SemanticEvent::FileChange { .. }
                | SemanticEvent::PlanUpdate { .. }
                | SemanticEvent::Info { .. }
                | SemanticEvent::Warning { .. }
                | SemanticEvent::Error { .. }
                | SemanticEvent::ProviderExtension { .. }
        )
    }
}

/// Sink interface for stream parsers.
///
/// Every successfully-parsed JSONL line produces exactly one [`SemanticEvent`]
/// delivered via `on_semantic_event`. Malformed JSON is surfaced as
/// [`SemanticEvent::Warning`] rather than propagated as an error.
pub trait SemanticEventSink: Send {
    fn on_semantic_event(&mut self, event: SemanticEvent);
}

/// No-op sink that discards all events.
pub struct NullSemanticSink;

impl SemanticEventSink for NullSemanticSink {
    fn on_semantic_event(&mut self, _event: SemanticEvent) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn all_variants() -> Vec<SemanticEvent> {
        vec![
            SemanticEvent::SessionStart {
                session_id: Some("s1".into()),
                model: Some("claude".into()),
                extra: json!({"provider": "claude"}),
            },
            SemanticEvent::TurnStart {
                extra: json!({}),
            },
            SemanticEvent::TurnComplete {
                provider_status: Some("ok".into()),
                token_usage: Some(NormalizedTokenUsage {
                    input: Some(10),
                    output: Some(20),
                    total: Some(30),
                    cache_read: None,
                }),
                cost_usd: Some(0.001),
                duration_ms: Some(1500),
                extra: json!({}),
            },
            SemanticEvent::OutputText {
                text: "hello".into(),
                extra: json!({}),
            },
            SemanticEvent::Reasoning {
                text: "thinking".into(),
                extra: json!({}),
            },
            SemanticEvent::ToolCall {
                name: Some("bash".into()),
                id: Some("t_1".into()),
                input: Some(json!({"cmd": "ls"})),
                extra: json!({}),
            },
            SemanticEvent::ToolResult {
                name: Some("bash".into()),
                id: Some("t_1".into()),
                status: Some("success".into()),
                exit_code: Some(0),
                output: Some(json!({"stdout": "file.txt"})),
                extra: json!({}),
            },
            SemanticEvent::PermissionRequest {
                kind: Some("shell".into()),
                tool_name: Some("bash".into()),
                extra: json!({}),
            },
            SemanticEvent::SubagentStart {
                name: Some("researcher".into()),
                id: Some("sa_1".into()),
                extra: json!({}),
            },
            SemanticEvent::SubagentStop {
                name: Some("researcher".into()),
                id: Some("sa_1".into()),
                status: Some("success".into()),
                extra: json!({}),
            },
            SemanticEvent::FileChange {
                path: Some("src/lib.rs".into()),
                change_kind: Some("modified".into()),
                extra: json!({}),
            },
            SemanticEvent::PlanUpdate {
                message: Some("Step 2 of 5".into()),
                extra: json!({}),
            },
            SemanticEvent::Info {
                message: "rate limit reset".into(),
                extra: json!({}),
            },
            SemanticEvent::Warning {
                message: "rate limited".into(),
                extra: json!({}),
            },
            SemanticEvent::Error {
                message: "quota exceeded".into(),
                terminal: true,
                extra: json!({}),
            },
            SemanticEvent::ProviderExtension {
                provider: Provider::Codex,
                kind: "web_search".into(),
                payload: json!({"query": "rust"}),
            },
        ]
    }

    #[test]
    fn kind_str_is_unique_per_variant() {
        let kinds: Vec<&str> = all_variants().iter().map(|e| e.kind_str()).collect();
        let mut sorted = kinds.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), kinds.len(), "kind_str must be unique per variant");
    }

    #[test]
    fn is_activity_excludes_envelope_events() {
        assert!(!SemanticEvent::SessionStart {
            session_id: None,
            model: None,
            extra: json!({}),
        }
        .is_activity());
        assert!(!SemanticEvent::TurnStart { extra: json!({}) }.is_activity());
        assert!(!SemanticEvent::TurnComplete {
            provider_status: None,
            token_usage: None,
            cost_usd: None,
            duration_ms: None,
            extra: json!({}),
        }
        .is_activity());
        assert!(!SemanticEvent::PermissionRequest {
            kind: None,
            tool_name: None,
            extra: json!({}),
        }
        .is_activity());
    }

    #[test]
    fn is_activity_includes_work_events() {
        assert!(SemanticEvent::OutputText {
            text: "x".into(),
            extra: json!({}),
        }
        .is_activity());
        assert!(SemanticEvent::ToolCall {
            name: None,
            id: None,
            input: None,
            extra: json!({}),
        }
        .is_activity());
        assert!(SemanticEvent::ProviderExtension {
            provider: Provider::Claude,
            kind: "x".into(),
            payload: json!({}),
        }
        .is_activity());
    }

    #[test]
    fn round_trip_serde_preserves_value_for_every_variant() {
        for event in all_variants() {
            let value = serde_json::to_value(&event).expect("serialize");
            let decoded: SemanticEvent =
                serde_json::from_value(value.clone()).expect("deserialize");
            let value2 = serde_json::to_value(&decoded).expect("re-serialize");
            assert_eq!(
                value, value2,
                "round-trip lost fidelity for kind {}",
                event.kind_str()
            );
        }
    }

    #[test]
    fn provider_extension_preserves_arbitrary_payload() {
        let payload = json!({
            "deeply": {
                "nested": ["values", 1, 2.5, null, true],
                "with_weird_keys": {"kebab-key": "ok"}
            }
        });
        let event = SemanticEvent::ProviderExtension {
            provider: Provider::Codex,
            kind: "item.updated".into(),
            payload: payload.clone(),
        };
        let value = serde_json::to_value(&event).unwrap();
        let decoded: SemanticEvent = serde_json::from_value(value).unwrap();
        match decoded {
            SemanticEvent::ProviderExtension {
                payload: decoded_payload,
                ..
            } => assert_eq!(decoded_payload, payload),
            other => panic!("expected ProviderExtension, got {other:?}"),
        }
    }

    #[test]
    fn null_sink_is_send() {
        fn assert_send<T: Send>() {}
        assert_send::<NullSemanticSink>();
    }
}
