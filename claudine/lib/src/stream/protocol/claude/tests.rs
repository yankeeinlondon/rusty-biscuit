use super::*;

fn parse(line: &str) -> ClaudeEvent {
    serde_json::from_str(line).expect("valid ClaudeEvent")
}

#[test]
fn claude_init_deserializes() {
    let event = parse(
        r#"{"type":"init","session_id":"sess-1","model":"claude-sonnet-4-20250514","apiKeySource":"ANTHROPIC_API_KEY"}"#,
    );
    let ClaudeEvent::Init(init) = event else {
        panic!("expected Init");
    };
    assert_eq!(init.session_id.as_deref(), Some("sess-1"));
    assert_eq!(init.model.as_deref(), Some("claude-sonnet-4-20250514"));
    assert_eq!(init.api_key_source.as_deref(), Some("ANTHROPIC_API_KEY"));
}

#[test]
fn claude_system_maps_to_init() {
    let event = parse(r#"{"type":"system","subtype":"init","session_id":"sess-2"}"#);
    let ClaudeEvent::System(init) = event else {
        panic!("expected System");
    };
    assert_eq!(init.session_id.as_deref(), Some("sess-2"));
    assert_eq!(init.subtype.as_deref(), Some("init"));
}

#[test]
fn claude_assistant_deserializes_flat_content() {
    let event = parse(r#"{"type":"assistant","content":[{"type":"text","text":"Hello"}]}"#);
    let ClaudeEvent::Assistant(assistant) = event else {
        panic!("expected Assistant");
    };
    assert!(assistant.message.is_none());
    let parts = assistant.content.expect("content");
    assert_eq!(parts.len(), 1);
    assert_eq!(parts[0].kind.as_deref(), Some("text"));
    assert_eq!(parts[0].text.as_deref(), Some("Hello"));
}

#[test]
fn claude_assistant_deserializes_with_message_key() {
    let event = parse(
        r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"Nested"}]}}"#,
    );
    let ClaudeEvent::Assistant(assistant) = event else {
        panic!("expected Assistant");
    };
    let message = assistant.message.expect("message");
    assert_eq!(message.role.as_deref(), Some("assistant"));
    let parts = message.content.expect("content");
    assert_eq!(parts[0].text.as_deref(), Some("Nested"));
}

#[test]
fn claude_assistant_deserializes_nested_tool_use_content() {
    let event = parse(
        r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"tool_use","id":"tu_1","name":"Bash","input":{"command":"ls -la"}}]}}"#,
    );
    let ClaudeEvent::Assistant(assistant) = event else {
        panic!("expected Assistant");
    };
    let message = assistant.message.expect("message");
    let part = message
        .content
        .expect("content")
        .into_iter()
        .next()
        .unwrap();
    let tool = part.into_tool_use().expect("tool_use");
    assert_eq!(tool.resolved_tool_id(), Some("tu_1"));
    assert_eq!(tool.resolved_tool_name(), Some("Bash"));
    assert_eq!(
        tool.input
            .as_ref()
            .and_then(|v| v.get("command"))
            .and_then(Value::as_str),
        Some("ls -la")
    );
}

#[test]
fn claude_content_block_delta_text() {
    let event =
        parse(r#"{"type":"content_block_delta","delta":{"type":"text_delta","text":"chunk"}}"#);
    let ClaudeEvent::ContentBlockDelta(d) = event else {
        panic!("expected ContentBlockDelta");
    };
    let delta = d.delta.expect("delta");
    assert_eq!(delta.kind.as_deref(), Some("text_delta"));
    assert_eq!(delta.text.as_deref(), Some("chunk"));
}

#[test]
fn claude_content_block_delta_thinking() {
    let event = parse(
        r#"{"type":"content_block_delta","delta":{"type":"thinking_delta","thinking":"reasoning"}}"#,
    );
    let ClaudeEvent::ContentBlockDelta(d) = event else {
        panic!("expected ContentBlockDelta");
    };
    let delta = d.delta.expect("delta");
    assert_eq!(delta.kind.as_deref(), Some("thinking_delta"));
    assert_eq!(delta.thinking.as_deref(), Some("reasoning"));
}

#[test]
fn claude_error_event_deserializes() {
    let event =
        parse(r#"{"type":"error","error":{"type":"billing_error","message":"no credits"}}"#);
    let ClaudeEvent::Error(err) = event else {
        panic!("expected Error");
    };
    let detail = err.error.expect("detail");
    assert_eq!(detail.kind.as_deref(), Some("billing_error"));
    assert_eq!(detail.message.as_deref(), Some("no credits"));
}

#[test]
fn claude_assistant_error_event_deserializes() {
    let event = parse(
        r#"{"type":"assistant.error","error":{"type":"rate_limit","message":"slow down"}}"#,
    );
    let ClaudeEvent::AssistantError(err) = event else {
        panic!("expected AssistantError");
    };
    let detail = err.error.expect("detail");
    assert_eq!(detail.kind.as_deref(), Some("rate_limit"));
}

#[test]
fn claude_result_deserializes() {
    let event = parse(
        r#"{"type":"result","duration_ms":12345,"duration_api_ms":11000,"num_turns":1,"stop_reason":"end_turn","cost_usd":0.0042,"usage":{"input_tokens":1000,"output_tokens":500,"cache_read_input_tokens":200}}"#,
    );
    let ClaudeEvent::Result(result) = event else {
        panic!("expected Result");
    };
    assert_eq!(result.duration_ms, Some(12345));
    assert_eq!(result.duration_api_ms, Some(11000));
    assert_eq!(result.num_turns, Some(1));
    assert_eq!(result.stop_reason.as_deref(), Some("end_turn"));
    assert_eq!(result.effective_cost_usd(), Some(0.0042));
    let usage = result.usage.expect("usage");
    assert_eq!(usage.input_tokens, Some(1000));
    assert_eq!(usage.output_tokens, Some(500));
    assert_eq!(usage.cache_read_input_tokens, Some(200));
    assert!(
        result.extra.is_empty(),
        "known result fields must not land in extra; extra={:?}",
        result.extra
    );
}

#[test]
fn claude_result_round_trips_through_json() {
    let line = r#"{"type":"result","duration_ms":12345,"duration_api_ms":11000,"num_turns":1,"stop_reason":"end_turn","total_cost_usd":0.0042,"usage":{"input_tokens":1000,"output_tokens":500,"cache_read_input_tokens":200}}"#;
    let event = parse(line);
    let serialized = serde_json::to_string(&event).unwrap();
    let reparsed: ClaudeEvent = serde_json::from_str(&serialized).unwrap();
    assert_eq!(
        serde_json::to_string(&reparsed).unwrap(),
        serialized,
        "parse -> serialize -> parse should be stable for a known event"
    );
}

#[test]
fn claude_result_total_cost_usd_preferred() {
    let event =
        parse(r#"{"type":"result","duration_ms":0,"total_cost_usd":0.185,"cost_usd":0.5}"#);
    let ClaudeEvent::Result(result) = event else {
        panic!("expected Result");
    };
    // total_cost_usd wins over cost_usd
    assert_eq!(result.effective_cost_usd(), Some(0.185));
}

#[test]
fn claude_rate_limit_deserializes() {
    let event = parse(
        r#"{"type":"rate_limit_event","is_throttled":true,"retry_after_ms":5000,"message":"slow"}"#,
    );
    let ClaudeEvent::RateLimit(rl) = event else {
        panic!("expected RateLimit");
    };
    assert_eq!(rl.is_throttled, Some(true));
    assert_eq!(rl.retry_after_ms, Some(5000));
    assert_eq!(rl.message.as_deref(), Some("slow"));
    assert!(
        rl.extra.is_empty(),
        "known rate-limit fields must not land in extra; extra={:?}",
        rl.extra
    );
}

#[test]
fn claude_rate_limit_nested_metadata_deserializes() {
    let event = parse(
        r#"{"type":"rate_limit_event","rate_limit_info":{"status":"approaching_limit","resetsAt":1712000000,"rateLimitType":"usage","overageStatus":"allowed"}}"#,
    );
    let ClaudeEvent::RateLimit(rl) = event else {
        panic!("expected RateLimit");
    };
    assert_eq!(rl.resolved_status(), Some("approaching_limit"));
    assert_eq!(rl.resolved_rate_limit_type(), Some("usage"));
    assert_eq!(rl.resolved_overage_status(), Some("allowed"));
    assert_eq!(rl.resolved_is_throttled(), Some(false));
    assert_eq!(
        rl.resolved_reset_at().map(|dt| dt.timestamp()),
        Some(1712000000)
    );
}

#[test]
fn claude_tool_use_deserializes() {
    let event =
        parse(r#"{"type":"tool_use","id":"tu-1","name":"bash","input":{"command":"ls"}}"#);
    let ClaudeEvent::ToolUse(tu) = event else {
        panic!("expected ToolUse");
    };
    assert_eq!(tu.id.as_deref(), Some("tu-1"));
    assert_eq!(tu.name.as_deref(), Some("bash"));
    assert_eq!(
        tu.input
            .as_ref()
            .and_then(|v| v.get("command"))
            .and_then(Value::as_str),
        Some("ls")
    );
}

#[test]
fn claude_tool_result_deserializes() {
    let event =
        parse(r#"{"type":"tool_result","tool_use_id":"tu-1","content":"file contents"}"#);
    let ClaudeEvent::ToolResult(tr) = event else {
        panic!("expected ToolResult");
    };
    assert_eq!(tr.resolved_tool_id(), Some("tu-1"));
    assert_eq!(
        tr.response().and_then(|v| v.as_str().map(String::from)),
        Some("file contents".into())
    );
}

#[test]
fn claude_content_block_start_tool_use() {
    let event = parse(
        r#"{"type":"content_block_start","content_block":{"type":"tool_use","id":"tu-2","name":"bash","input":{"command":"ls -la"}}}"#,
    );
    let ClaudeEvent::ContentBlockStart(cbs) = event else {
        panic!("expected ContentBlockStart");
    };
    let block = cbs.content_block.expect("content_block");
    assert_eq!(block.kind.as_deref(), Some("tool_use"));
    let tu = block.into_tool_use();
    assert_eq!(tu.id.as_deref(), Some("tu-2"));
    assert_eq!(tu.name.as_deref(), Some("bash"));
}

#[test]
fn claude_init_tolerates_unknown_fields() {
    let event = parse(
        r#"{"type":"init","session_id":"sess","model":"m","tools":[{"name":"a"}],"mcp_servers":["x"]}"#,
    );
    let ClaudeEvent::Init(init) = event else {
        panic!("expected Init");
    };
    assert_eq!(init.session_id.as_deref(), Some("sess"));
}

#[test]
fn claude_unknown_event_type_fails_typed_deserialization() {
    let err = serde_json::from_str::<ClaudeEvent>(
        r#"{"type":"some_unknown_event","text":"ignored"}"#,
    );
    assert!(err.is_err());
}

#[test]
fn claude_missing_type_fails_typed_deserialization() {
    let err = serde_json::from_str::<ClaudeEvent>(r#"{"session_id":"sess"}"#);
    assert!(err.is_err());
}

#[test]
fn claude_user_event_deserializes_with_tool_result_content() {
    let line = r#"{"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"tu_1","content":"hello","is_error":false}]},"session_id":"s1"}"#;
    let event: ClaudeEvent = serde_json::from_str(line).expect("valid event");
    let ClaudeEvent::User(user) = event else {
        panic!("expected ClaudeEvent::User, got {event:?}");
    };
    let content = user.message.and_then(|m| m.content).expect("content");
    assert!(
        content
            .iter()
            .any(|c: &Value| c.get("type").and_then(Value::as_str) == Some("tool_result"))
    );
}

#[test]
fn claude_system_hook_subtypes_deserialize() {
    for subtype in ["hook_started", "hook_response"] {
        let line = format!(
            r#"{{"type":"system","subtype":"{subtype}","session_id":"s1","hook_name":"SessionStart"}}"#
        );
        let event: ClaudeEvent = serde_json::from_str(&line).expect("valid event");
        assert!(
            matches!(event, ClaudeEvent::System(_)),
            "subtype {subtype} must parse as System event"
        );
    }
}

#[test]
fn claude_assistant_error_field_preserved() {
    let line = r#"{"type":"assistant","message":{"model":"<synthetic>","content":[{"type":"text","text":"Credit balance is too low"}]},"session_id":"s1","error":"billing_error"}"#;
    let event: ClaudeEvent = serde_json::from_str(line).expect("valid event");
    let ClaudeEvent::Assistant(a) = event else {
        panic!("expected Assistant")
    };
    assert_eq!(a.error.as_deref(), Some("billing_error"));
}

#[test]
fn claude_result_fields_billing_error_surface() {
    let line = r#"{"type":"result","subtype":"success","is_error":true,"result":"Credit balance is too low","session_id":"s1","permission_denials":[],"terminal_reason":"completed","fast_mode_state":"off","total_cost_usd":0,"usage":{"input_tokens":0,"output_tokens":0,"cache_creation_input_tokens":0,"cache_read_input_tokens":0,"service_tier":"standard"},"modelUsage":{}}"#;
    let event: ClaudeEvent = serde_json::from_str(line).expect("valid event");
    let ClaudeEvent::Result(r) = event else {
        panic!("expected Result")
    };
    assert_eq!(r.is_error, Some(true));
    assert_eq!(r.result.as_deref(), Some("Credit balance is too low"));
}
