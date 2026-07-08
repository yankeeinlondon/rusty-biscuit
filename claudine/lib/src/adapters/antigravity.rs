use serde_json::Value;

use super::{AdapterError, ProviderAdapter, str_field};
use crate::actions::HookResponse;
use crate::events::{AgenticEvent, EventMeta};
use crate::provider::Provider;

/// Hook-dispatch adapter for Antigravity.
///
/// agy has no headless hook system, so this adapter is never reached through
/// hook registration — agy's live output is consumed by
/// [`crate::stream::providers::antigravity::AntigravitySemanticStreamParser`]
/// instead. It exists to satisfy the per-provider [`ProviderAdapter`] contract
/// and to give `claudine handle --provider antigravity` a best-effort
/// normalization of the single `--output-format json` result envelope.
pub(crate) struct AntigravityAdapter;

impl ProviderAdapter for AntigravityAdapter {
    fn provider(&self) -> Provider {
        Provider::Antigravity
    }

    fn parse_event(&self, raw: &Value) -> Result<(AgenticEvent, EventMeta), AdapterError> {
        // agy's print-mode envelope carries no `type` discriminator; its
        // presence of a `status`/`response` field marks it as the terminal
        // result record. Anything else is not a recognizable agy event.
        if raw.get("status").is_none() && raw.get("response").is_none() {
            return Err(AdapterError::MissingField("status"));
        }
        let mut meta = EventMeta::new(Provider::Antigravity, AgenticEvent::TurnComplete);
        meta.session_id = str_field(raw, "conversation_id");
        meta.error = str_field(raw, "error").or_else(|| match raw.get("status") {
            Some(Value::String(s)) if !s.eq_ignore_ascii_case("success") => Some(s.clone()),
            _ => None,
        });
        if let Some(usage) = raw.get("usage") {
            meta.extra.insert("usage".to_string(), usage.clone());
        }
        Ok((AgenticEvent::TurnComplete, meta))
    }

    fn can_block(&self, _event: &AgenticEvent) -> bool {
        // agy exposes no permission events and no hook-approval path.
        false
    }

    fn non_blocking_ack(&self) -> Option<Value> {
        None
    }

    fn format_response(
        &self,
        _event: &AgenticEvent,
        _response: &HookResponse,
    ) -> Result<Value, AdapterError> {
        Ok(Value::Null)
    }

    fn exit_code(&self, _event: &AgenticEvent, _response: &HookResponse) -> Option<i32> {
        None
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn success_envelope_maps_turn_complete() {
        let adapter = AntigravityAdapter;
        let raw = json!({"conversation_id": "c-1", "status": "SUCCESS", "response": "hi"});
        let (event, meta) = adapter.parse_event(&raw).unwrap();
        assert_eq!(event, AgenticEvent::TurnComplete);
        assert_eq!(meta.session_id.as_deref(), Some("c-1"));
        assert!(meta.error.is_none());
    }

    #[test]
    fn error_status_surfaces_error() {
        let adapter = AntigravityAdapter;
        let raw = json!({"status": "ERROR", "error": "quota exhausted"});
        let (_, meta) = adapter.parse_event(&raw).unwrap();
        assert_eq!(meta.error.as_deref(), Some("quota exhausted"));
    }

    #[test]
    fn payload_without_status_or_response_errors() {
        let adapter = AntigravityAdapter;
        assert!(adapter.parse_event(&json!({"foo": "bar"})).is_err());
    }

    #[test]
    fn nothing_is_blocking() {
        let adapter = AntigravityAdapter;
        assert!(!adapter.can_block(&AgenticEvent::TurnComplete));
        assert!(!adapter.can_block(&AgenticEvent::PermissionRequest));
    }

    #[test]
    fn provider_is_antigravity() {
        assert_eq!(AntigravityAdapter.provider(), Provider::Antigravity);
    }
}
