use serde_json::{Value, json};

use super::{AdapterError, ProviderAdapter, str_field};
use crate::actions::{HookDecision, HookResponse};
use crate::events::{AgenticEvent, EventMeta};
use crate::provider::Provider;

/// Hook-dispatch adapter for Antigravity.
///
/// agy calls `claudine handle <event> --provider antigravity` for each
/// configured hook, passing a camelCase JSON payload on stdin and reading a
/// JSON decision from stdout. The payload carries **no** event-name field, so
/// the event is inferred from which fields are present (`toolCall` →
/// PreToolUse, `fullyIdle`/`terminationReason` → Stop, `invocationNum` →
/// Pre/PostInvocation, a bare `stepIdx` → PostToolUse). PreInvocation and
/// PostInvocation share an identical payload shape and cannot be distinguished
/// from the payload alone; the ambiguous case is mapped to `before_model`
/// (documented limitation — the loop-control PostInvocation is the rarer case
/// and print-mode hook execution is itself pending confirmation).
pub(crate) struct AntigravityAdapter;

impl ProviderAdapter for AntigravityAdapter {
    fn provider(&self) -> Provider {
        Provider::Antigravity
    }

    fn parse_event(&self, raw: &Value) -> Result<(AgenticEvent, EventMeta), AdapterError> {
        let event = infer_event(raw)?;
        let mut meta = EventMeta::new(Provider::Antigravity, event);
        meta.session_id = str_field(raw, "conversationId");
        meta.cwd = raw
            .get("workspacePaths")
            .and_then(Value::as_array)
            .and_then(|a| a.first())
            .and_then(Value::as_str)
            .map(str::to_string);
        meta.tool_name = raw.get("toolCall").and_then(|t| str_field(t, "name"));
        meta.tool_input = raw.get("toolCall").and_then(|t| t.get("args").cloned());
        meta.error = str_field(raw, "error").filter(|e| !e.is_empty());
        // Preserve agy-specific correlation fields on the normalized payload.
        for key in ["conversationId", "transcriptPath", "artifactDirectoryPath"] {
            if let Some(v) = raw.get(key) {
                meta.extra.insert(key.to_string(), v.clone());
            }
        }
        Ok((event, meta))
    }

    fn can_block(&self, event: &AgenticEvent) -> bool {
        // agy's blocking hooks: PreToolUse (decision), PreInvocation /
        // PostInvocation (injectSteps / terminationBehavior), Stop (continue).
        // PostToolUse is observational.
        matches!(
            event,
            AgenticEvent::BeforeTool
                | AgenticEvent::BeforeModel
                | AgenticEvent::AfterModel
                | AgenticEvent::TurnComplete
        )
    }

    fn non_blocking_ack(&self) -> Option<Value> {
        // PostToolUse expects `{}` on stdout.
        Some(json!({}))
    }

    fn format_response(
        &self,
        event: &AgenticEvent,
        response: &HookResponse,
    ) -> Result<Value, AdapterError> {
        if let Some(raw) = &response.raw {
            return Ok(raw.clone());
        }
        if !self.can_block(event) {
            return Ok(json!({}));
        }
        let reason = response.reason.clone();
        match event {
            // PreToolUse decision vocabulary: allow / deny / ask.
            AgenticEvent::BeforeTool => match response.decision.unwrap_or(HookDecision::Allow) {
                HookDecision::Deny => Ok(json!({ "decision": "deny", "reason": reason })),
                HookDecision::Ask => Ok(json!({ "decision": "ask", "reason": reason })),
                _ => Ok(json!({ "decision": "allow" })),
            },
            // Stop: `decision:"continue"` prevents the stop and re-enters the
            // loop; anything else allows the stop.
            AgenticEvent::TurnComplete => match response.decision.unwrap_or(HookDecision::Allow) {
                HookDecision::Deny | HookDecision::Continue => {
                    Ok(json!({ "decision": "continue", "reason": reason }))
                }
                _ => Ok(json!({})),
            },
            // PreInvocation / PostInvocation: loop-control via injectSteps /
            // terminationBehavior. A Deny maps to forcing termination.
            AgenticEvent::AfterModel if matches!(response.decision, Some(HookDecision::Deny)) => {
                Ok(json!({ "terminationBehavior": "terminate" }))
            }
            _ => Ok(json!({})),
        }
    }

    fn exit_code(&self, _event: &AgenticEvent, _response: &HookResponse) -> Option<i32> {
        // agy reads the decision from stdout JSON, not the exit code (exit-code
        // semantics are undocumented), so leave the exit code to the harness.
        None
    }
}

/// Infer the Claudine event from an agy hook payload's field shape.
fn infer_event(raw: &Value) -> Result<AgenticEvent, AdapterError> {
    if raw.get("toolCall").is_some() {
        return Ok(AgenticEvent::BeforeTool);
    }
    if raw.get("fullyIdle").is_some()
        || raw.get("terminationReason").is_some()
        || raw.get("executionNum").is_some()
    {
        return Ok(AgenticEvent::TurnComplete);
    }
    if raw.get("invocationNum").is_some() {
        // PreInvocation and PostInvocation are indistinguishable by payload;
        // default to the pre-model boundary.
        return Ok(AgenticEvent::BeforeModel);
    }
    if raw.get("stepIdx").is_some() {
        return Ok(AgenticEvent::AfterTool);
    }
    Err(AdapterError::MissingField("toolCall|invocationNum|stepIdx|fullyIdle"))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn pretooluse_payload_maps_before_tool() {
        let adapter = AntigravityAdapter;
        let raw = json!({
            "conversationId": "c-1",
            "workspacePaths": ["/work"],
            "toolCall": { "name": "run_command", "args": { "CommandLine": "echo hi" } },
            "stepIdx": 0
        });
        let (event, meta) = adapter.parse_event(&raw).unwrap();
        assert_eq!(event, AgenticEvent::BeforeTool);
        assert_eq!(meta.session_id.as_deref(), Some("c-1"));
        assert_eq!(meta.cwd.as_deref(), Some("/work"));
        assert_eq!(meta.tool_name.as_deref(), Some("run_command"));
    }

    #[test]
    fn stop_payload_maps_turn_complete() {
        let adapter = AntigravityAdapter;
        let raw = json!({ "conversationId": "c-2", "executionNum": 1, "terminationReason": "model_stop", "fullyIdle": true });
        let (event, _) = adapter.parse_event(&raw).unwrap();
        assert_eq!(event, AgenticEvent::TurnComplete);
    }

    #[test]
    fn invocation_payload_maps_before_model() {
        let adapter = AntigravityAdapter;
        let raw = json!({ "conversationId": "c-3", "invocationNum": 0, "initialNumSteps": 2 });
        let (event, _) = adapter.parse_event(&raw).unwrap();
        assert_eq!(event, AgenticEvent::BeforeModel);
    }

    #[test]
    fn posttooluse_payload_maps_after_tool() {
        let adapter = AntigravityAdapter;
        let raw = json!({ "conversationId": "c-4", "stepIdx": 3, "error": "boom" });
        let (event, meta) = adapter.parse_event(&raw).unwrap();
        assert_eq!(event, AgenticEvent::AfterTool);
        assert_eq!(meta.error.as_deref(), Some("boom"));
    }

    #[test]
    fn unrecognized_payload_errors() {
        let adapter = AntigravityAdapter;
        assert!(adapter.parse_event(&json!({ "conversationId": "x" })).is_err());
    }

    #[test]
    fn deny_before_tool_formats_agy_decision() {
        let adapter = AntigravityAdapter;
        let response = HookResponse {
            decision: Some(HookDecision::Deny),
            reason: Some("blocked by policy".to_string()),
            ..Default::default()
        };
        let out = adapter
            .format_response(&AgenticEvent::BeforeTool, &response)
            .unwrap();
        assert_eq!(out["decision"], "deny");
        assert_eq!(out["reason"], "blocked by policy");
    }

    #[test]
    fn after_tool_is_not_blocking() {
        let adapter = AntigravityAdapter;
        assert!(!adapter.can_block(&AgenticEvent::AfterTool));
        assert!(adapter.can_block(&AgenticEvent::BeforeTool));
    }

    #[test]
    fn provider_is_antigravity() {
        assert_eq!(AntigravityAdapter.provider(), Provider::Antigravity);
    }
}
