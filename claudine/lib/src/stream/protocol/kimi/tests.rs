use super::*;

// ------------------------------------------------------------------
// Wire-mode (JSON-RPC 2.0) protocol tests
// ------------------------------------------------------------------
//
// Fixture-corpus replay of the typed envelope model lives in the
// `protocol_fixture_replay` integration test; the cases below are
// inline-literal deserialization unit tests.

#[test]
fn envelope_classifies_notification() {
    let value = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "event",
        "params": {"type": "TurnEnd", "payload": {}},
    });
    let env = KimiEnvelope::classify(value).expect("classified");
    let KimiEnvelope::Notification(params) = env else {
        panic!("expected Notification");
    };
    assert_eq!(params.event_type, "TurnEnd");
}

#[test]
fn envelope_classifies_request() {
    let value = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "request",
        "id": "req-1",
        "params": {"type": "ApprovalRequest", "payload": {"id": "a"}},
    });
    let env = KimiEnvelope::classify(value).expect("classified");
    let KimiEnvelope::Request { id, params } = env else {
        panic!("expected Request");
    };
    assert_eq!(id.as_str(), Some("req-1"));
    assert_eq!(params.request_type, "ApprovalRequest");
}

#[test]
fn envelope_classifies_success_response() {
    let value = serde_json::json!({
        "jsonrpc": "2.0",
        "id": "prompt-2",
        "result": {"status": "finished"},
    });
    let env = KimiEnvelope::classify(value).expect("classified");
    let KimiEnvelope::SuccessResponse { id, result } = env else {
        panic!("expected SuccessResponse");
    };
    assert_eq!(id.as_str(), Some("prompt-2"));
    let parsed: KimiPromptResult = serde_json::from_value(result).unwrap();
    assert_eq!(
        parsed.status.as_deref(),
        Some(KimiPromptResult::STATUS_FINISHED)
    );
}

#[test]
fn envelope_classifies_error_response() {
    let value = serde_json::json!({
        "jsonrpc": "2.0",
        "id": "prompt-2",
        "error": {"code": -32004, "message": "Auth expired", "data": null},
    });
    let env = KimiEnvelope::classify(value).expect("classified");
    let KimiEnvelope::ErrorResponse { id, error } = env else {
        panic!("expected ErrorResponse");
    };
    assert_eq!(id.as_str(), Some("prompt-2"));
    assert_eq!(error.code, KimiJsonRpcError::AUTH_EXPIRED);
    assert_eq!(error.message, "Auth expired");
}

#[test]
fn envelope_returns_none_for_unknown_shape() {
    let value = serde_json::json!({"jsonrpc": "2.0"});
    assert!(KimiEnvelope::classify(value).is_none());
}

#[test]
fn notification_params_decodes_typed_event() {
    let params = KimiNotificationParams {
        event_type: "StepBegin".into(),
        payload: serde_json::json!({"n": 3}),
    };
    let event = params.into_event().expect("typed event");
    let KimiWireEvent::StepBegin(payload) = event else {
        panic!("expected StepBegin");
    };
    assert_eq!(payload.n, Some(3));
}

#[test]
fn notification_params_unknown_event_type_returns_none() {
    let params = KimiNotificationParams {
        event_type: "FuturisticEvent".into(),
        payload: Value::Null,
    };
    assert!(params.into_event().is_none());
}

#[test]
fn request_params_decodes_typed_request() {
    let params = KimiRequestParams {
        request_type: "ApprovalRequest".into(),
        payload: serde_json::json!({
            "id": "abc",
            "tool_call_id": "tool_x",
            "sender": "Shell",
            "action": "run command",
            "description": "Run command `ls`",
            "display": [{"type": "shell", "language": "bash", "command": "ls"}]
        }),
    };
    let request = params.into_request().expect("typed request");
    let KimiWireRequest::Approval(approval) = request else {
        panic!("expected Approval");
    };
    assert_eq!(approval.id.as_deref(), Some("abc"));
    assert_eq!(approval.tool_call_id.as_deref(), Some("tool_x"));
    assert_eq!(approval.shell_command(), Some("ls".into()));
}

#[test]
fn request_params_unknown_request_type_returns_none() {
    let params = KimiRequestParams {
        request_type: "FuturisticRequest".into(),
        payload: Value::Null,
    };
    assert!(params.into_request().is_none());
}

#[test]
fn wire_event_unknown_type_fails_typed() {
    let envelope = serde_json::json!({"type": "FuturisticEvent", "payload": {}});
    let result: Result<KimiWireEvent, _> = serde_json::from_value(envelope);
    assert!(result.is_err());
}

#[test]
fn wire_request_unknown_type_fails_typed() {
    let envelope = serde_json::json!({"type": "FuturisticRequest", "payload": {}});
    let result: Result<KimiWireRequest, _> = serde_json::from_value(envelope);
    assert!(result.is_err());
}

#[test]
fn turn_begin_user_input_text_handles_string() {
    let envelope = serde_json::json!({
        "type": "TurnBegin",
        "payload": {"user_input": "Hi how are you?"},
    });
    let event: KimiWireEvent = serde_json::from_value(envelope).unwrap();
    let KimiWireEvent::TurnBegin(payload) = event else {
        panic!("expected TurnBegin");
    };
    assert_eq!(payload.user_input_text(), Some("Hi how are you?".into()));
}

#[test]
fn turn_begin_user_input_text_handles_array() {
    let envelope = serde_json::json!({
        "type": "TurnBegin",
        "payload": {"user_input": [
            {"type": "text", "text": "Hi "},
            {"type": "text", "text": "Bob"}
        ]},
    });
    let event: KimiWireEvent = serde_json::from_value(envelope).unwrap();
    let KimiWireEvent::TurnBegin(payload) = event else {
        panic!("expected TurnBegin");
    };
    assert_eq!(payload.user_input_text(), Some("Hi Bob".into()));
}

#[test]
fn content_part_text_resolves_text() {
    let envelope = serde_json::json!({
        "type": "ContentPart",
        "payload": {"type": "text", "text": "Hi Bob!"},
    });
    let event: KimiWireEvent = serde_json::from_value(envelope).unwrap();
    let KimiWireEvent::ContentPart(part) = event else {
        panic!("expected ContentPart");
    };
    assert!(part.is_text());
    assert!(!part.is_thinking());
    assert_eq!(part.resolved_text(), Some("Hi Bob!"));
}

#[test]
fn content_part_think_resolves_thinking() {
    let envelope = serde_json::json!({
        "type": "ContentPart",
        "payload": {"type": "think", "think": "The user...", "encrypted": null},
    });
    let event: KimiWireEvent = serde_json::from_value(envelope).unwrap();
    let KimiWireEvent::ContentPart(part) = event else {
        panic!("expected ContentPart");
    };
    assert!(part.is_thinking());
    assert!(!part.is_text());
    assert_eq!(part.resolved_text(), Some("The user..."));
}

#[test]
fn content_part_image_url_returns_no_inline_text() {
    let envelope = serde_json::json!({
        "type": "ContentPart",
        "payload": {"type": "image_url", "image_url": "https://example.test/x.png"},
    });
    let event: KimiWireEvent = serde_json::from_value(envelope).unwrap();
    let KimiWireEvent::ContentPart(part) = event else {
        panic!("expected ContentPart");
    };
    assert_eq!(part.resolved_text(), None);
}

#[test]
fn status_update_computes_percent_from_fraction() {
    let envelope = serde_json::json!({
        "type": "StatusUpdate",
        "payload": {
            "context_usage": 0.06,
            "context_tokens": 15969,
            "max_context_tokens": 262144,
            "token_usage": {"input_other": 10081, "output": 52, "input_cache_read": 5888, "input_cache_creation": 0},
            "message_id": "chatcmpl-abc",
            "plan_mode": false,
            "mcp_status": null
        }
    });
    let event: KimiWireEvent = serde_json::from_value(envelope).unwrap();
    let KimiWireEvent::StatusUpdate(status) = event else {
        panic!("expected StatusUpdate");
    };
    let pct = status.computed_context_percent().expect("percent");
    assert!((pct - 6.0).abs() < 0.01, "percent was {pct}");
    let usage = status.token_usage.as_ref().expect("token_usage");
    assert_eq!(usage.total_input(), Some(10081 + 5888));
    assert_eq!(usage.cache_read_input(), Some(5888));
    assert_eq!(status.message_id.as_deref(), Some("chatcmpl-abc"));
}

#[test]
fn status_update_computes_percent_from_token_counters() {
    let envelope = serde_json::json!({
        "type": "StatusUpdate",
        "payload": {"context_tokens": 100, "max_context_tokens": 200}
    });
    let event: KimiWireEvent = serde_json::from_value(envelope).unwrap();
    let KimiWireEvent::StatusUpdate(status) = event else {
        panic!("expected StatusUpdate");
    };
    assert_eq!(status.computed_context_percent(), Some(50.0));
}

#[test]
fn supported_versions_window_covers_both_wire_revisions() {
    assert_eq!(SUPPORTED_WIRE_PROTOCOL_VERSIONS, &["1.9", "1.10"]);
}

#[test]
fn step_retry_decodes_full_payload() {
    let envelope = serde_json::json!({
        "type": "StepRetry",
        "payload": {
            "n": 3,
            "next_attempt": 2,
            "max_attempts": 5,
            "wait_s": 1.5,
            "error_type": "APIEmptyResponseError",
            "status_code": 500
        }
    });
    let event: KimiWireEvent = serde_json::from_value(envelope).unwrap();
    let KimiWireEvent::StepRetry(retry) = event else {
        panic!("expected StepRetry");
    };
    assert_eq!(retry.n, Some(3));
    assert_eq!(retry.next_attempt, Some(2));
    assert_eq!(retry.max_attempts, Some(5));
    assert_eq!(retry.wait_s, Some(1.5));
    assert_eq!(retry.error_type.as_deref(), Some("APIEmptyResponseError"));
    assert_eq!(retry.status_code, Some(500));
}

#[test]
fn step_retry_decodes_empty_payload() {
    let envelope = serde_json::json!({"type": "StepRetry", "payload": {}});
    let event: KimiWireEvent = serde_json::from_value(envelope).unwrap();
    let KimiWireEvent::StepRetry(retry) = event else {
        panic!("expected StepRetry");
    };
    assert!(retry.error_type.is_none());
    assert!(retry.status_code.is_none());
}

#[test]
fn status_update_decodes_typed_mcp_status() {
    let envelope = serde_json::json!({
        "type": "StatusUpdate",
        "payload": {
            "context_tokens": 100,
            "max_context_tokens": 200,
            "mcp_status": {
                "loading": false,
                "connected": 2,
                "total": 3,
                "tools": 14,
                "servers": [
                    {"name": "weather", "status": "connected", "tools": ["forecast"]},
                    {"name": "broken", "status": "failed", "tools": []}
                ]
            }
        }
    });
    let event: KimiWireEvent = serde_json::from_value(envelope).unwrap();
    let KimiWireEvent::StatusUpdate(status) = event else {
        panic!("expected StatusUpdate");
    };
    let snapshot = status.mcp_status.expect("mcp_status");
    assert_eq!(snapshot.loading, Some(false));
    assert_eq!(snapshot.connected, Some(2));
    assert_eq!(snapshot.total, Some(3));
    assert_eq!(snapshot.tools, Some(14));
    let servers = snapshot.servers.expect("servers");
    assert_eq!(servers.len(), 2);
    assert_eq!(servers[0].name.as_deref(), Some("weather"));
    assert_eq!(servers[1].status.as_deref(), Some("failed"));
}

#[test]
fn notification_decodes_1_10_payload() {
    let envelope = serde_json::json!({
        "type": "Notification",
        "payload": {
            "id": "notif-1",
            "category": "task",
            "type": "task.completed",
            "source_kind": "background_task",
            "source_id": "task-9",
            "title": "Background task finished",
            "body": "Task `lint` completed",
            "severity": "info",
            "created_at": 1751700000.25,
            "payload": {"exit_code": 0}
        }
    });
    let event: KimiWireEvent = serde_json::from_value(envelope).unwrap();
    let KimiWireEvent::Notification(notification) = event else {
        panic!("expected Notification");
    };
    assert_eq!(notification.category.as_deref(), Some("task"));
    assert_eq!(
        notification.notification_type.as_deref(),
        Some("task.completed")
    );
    assert_eq!(
        notification.source_kind.as_deref(),
        Some("background_task")
    );
    assert_eq!(notification.body.as_deref(), Some("Task `lint` completed"));
    assert_eq!(notification.severity.as_deref(), Some("info"));
    assert_eq!(notification.created_at, Some(1751700000.25));
    // 1.9 fields are absent on a pure 1.10 payload.
    assert!(notification.level.is_none());
    assert!(notification.message.is_none());
}

#[test]
fn notification_still_decodes_1_9_payload() {
    let envelope = serde_json::json!({
        "type": "Notification",
        "payload": {
            "level": "error",
            "source": "claudine",
            "message": "Hook dispatch failed: boom"
        }
    });
    let event: KimiWireEvent = serde_json::from_value(envelope).unwrap();
    let KimiWireEvent::Notification(notification) = event else {
        panic!("expected Notification");
    };
    assert_eq!(notification.level.as_deref(), Some("error"));
    assert_eq!(
        notification.message.as_deref(),
        Some("Hook dispatch failed: boom")
    );
    assert!(notification.severity.is_none());
    assert!(notification.body.is_none());
}

#[test]
fn tool_call_arguments_string_round_trip() {
    let envelope = serde_json::json!({
        "type": "ToolCall",
        "payload": {
            "type": "function",
            "id": "tool_x",
            "function": {"name": "Shell", "arguments": ""},
            "extras": null
        }
    });
    let event: KimiWireEvent = serde_json::from_value(envelope).unwrap();
    let KimiWireEvent::ToolCall(mut call) = event else {
        panic!("expected ToolCall");
    };
    assert_eq!(call.resolved_tool_id(), Some("tool_x"));
    assert_eq!(call.resolved_tool_name(), Some("Shell"));
    // Empty initial arguments — subsequent ToolCallPart deltas accumulate the body.
    assert_eq!(call.take_arguments_string().as_deref(), Some(""));
}

#[test]
fn parse_arguments_string_decodes_valid_json() {
    let parsed = KimiToolCall::parse_arguments_string(r#"{"command":"ls"}"#).unwrap();
    let value = parsed.expect("parsed");
    assert_eq!(value.get("command").and_then(Value::as_str), Some("ls"));
}

#[test]
fn parse_arguments_string_returns_none_for_empty() {
    let parsed = KimiToolCall::parse_arguments_string("").unwrap();
    assert!(parsed.is_none());
    let parsed = KimiToolCall::parse_arguments_string("   ").unwrap();
    assert!(parsed.is_none());
}

#[test]
fn parse_arguments_string_passthrough_on_malformed_json() {
    let err = KimiToolCall::parse_arguments_string("{not json").unwrap_err();
    assert_eq!(err, "{not json");
}

#[test]
fn tool_call_part_carries_argument_delta() {
    let envelope = serde_json::json!({
        "type": "ToolCallPart",
        "payload": {"arguments_part": "{\"command\":"}
    });
    let event: KimiWireEvent = serde_json::from_value(envelope).unwrap();
    let KimiWireEvent::ToolCallPart(part) = event else {
        panic!("expected ToolCallPart");
    };
    assert_eq!(part.arguments_part.as_deref(), Some("{\"command\":"));
}

#[test]
fn tool_result_returns_status_and_output() {
    let envelope = serde_json::json!({
        "type": "ToolResult",
        "payload": {
            "tool_call_id": "tool_x",
            "return_value": {
                "is_error": false,
                "output": "hello-from-kimi\n",
                "message": "Command executed successfully.",
                "display": [],
                "extras": null
            }
        }
    });
    let event: KimiWireEvent = serde_json::from_value(envelope).unwrap();
    let KimiWireEvent::ToolResult(mut result) = event else {
        panic!("expected ToolResult");
    };
    assert_eq!(result.resolved_tool_id(), Some("tool_x"));
    assert_eq!(result.derived_status(), "success");
    let output = result.take_output().expect("output");
    assert_eq!(output.as_str(), Some("hello-from-kimi\n"));
}

#[test]
fn tool_result_marks_errors() {
    let envelope = serde_json::json!({
        "type": "ToolResult",
        "payload": {
            "tool_call_id": "tool_y",
            "return_value": {"is_error": true, "output": "permission denied"}
        }
    });
    let event: KimiWireEvent = serde_json::from_value(envelope).unwrap();
    let KimiWireEvent::ToolResult(result) = event else {
        panic!("expected ToolResult");
    };
    assert!(result.is_error());
    assert_eq!(result.derived_status(), "error");
}

#[test]
fn approval_request_extracts_shell_command() {
    let envelope = serde_json::json!({
        "type": "ApprovalRequest",
        "payload": {
            "id": "appr-1",
            "tool_call_id": "tool_x",
            "sender": "Shell",
            "action": "run command",
            "description": "Run command `echo hi`",
            "source_kind": "foreground_turn",
            "source_id": "src-1",
            "display": [{"type": "shell", "language": "bash", "command": "echo hi"}]
        }
    });
    let request: KimiWireRequest = serde_json::from_value(envelope).unwrap();
    let KimiWireRequest::Approval(approval) = request else {
        panic!("expected Approval");
    };
    assert_eq!(approval.shell_command(), Some("echo hi".into()));
    assert_eq!(approval.action.as_deref(), Some("run command"));
}

#[test]
fn approval_request_no_shell_returns_none() {
    let envelope = serde_json::json!({
        "type": "ApprovalRequest",
        "payload": {
            "id": "appr-2",
            "display": [{"type": "markdown", "body": "..."}]
        }
    });
    let request: KimiWireRequest = serde_json::from_value(envelope).unwrap();
    let KimiWireRequest::Approval(approval) = request else {
        panic!("expected Approval");
    };
    assert_eq!(approval.shell_command(), None);
}

#[test]
fn subagent_event_decodes_nested_event() {
    let envelope = serde_json::json!({
        "type": "SubagentEvent",
        "payload": {
            "parent_tool_call_id": "tool_p",
            "agent_id": "ab1",
            "subagent_type": "explore",
            "event": {
                "type": "StepBegin",
                "payload": {"n": 1}
            }
        }
    });
    let event: KimiWireEvent = serde_json::from_value(envelope).unwrap();
    let KimiWireEvent::SubagentEvent(sub) = event else {
        panic!("expected SubagentEvent");
    };
    assert_eq!(sub.subagent_type.as_deref(), Some("explore"));
    let nested = sub.nested_event().expect("nested");
    let KimiWireEvent::StepBegin(payload) = nested else {
        panic!("expected nested StepBegin");
    };
    assert_eq!(payload.n, Some(1));
}

#[test]
fn approval_response_event_round_trip() {
    let envelope = serde_json::json!({
        "type": "ApprovalResponse",
        "payload": {"request_id": "abc", "response": "approve", "feedback": ""}
    });
    let event: KimiWireEvent = serde_json::from_value(envelope).unwrap();
    let KimiWireEvent::ApprovalResponse(echo) = event else {
        panic!("expected ApprovalResponse");
    };
    assert_eq!(echo.request_id.as_deref(), Some("abc"));
    assert_eq!(echo.response.as_deref(), Some("approve"));
}

#[test]
fn initialize_result_decodes_capabilities() {
    let result = serde_json::json!({
        "protocol_version": "1.9",
        "server": {"name": "Kimi Code CLI", "version": "1.38.0"},
        "slash_commands": [],
        "hooks": [],
        "capabilities": {"supports_question": true, "supports_plan_mode": false}
    });
    let parsed: KimiInitializeResult = serde_json::from_value(result).unwrap();
    assert_eq!(parsed.protocol_version.as_deref(), Some("1.9"));
    let server = parsed.server.expect("server");
    assert_eq!(server.name.as_deref(), Some("Kimi Code CLI"));
    let caps = parsed.capabilities.expect("capabilities");
    assert_eq!(caps.supports_question, Some(true));
    assert_eq!(caps.supports_plan_mode, Some(false));
}

#[test]
fn prompt_result_recognizes_known_statuses() {
    for status in [
        KimiPromptResult::STATUS_FINISHED,
        KimiPromptResult::STATUS_CANCELLED,
        KimiPromptResult::STATUS_MAX_STEPS_REACHED,
        KimiPromptResult::STATUS_STEERED,
    ] {
        let v = serde_json::json!({"status": status});
        let parsed: KimiPromptResult = serde_json::from_value(v).unwrap();
        assert_eq!(parsed.status.as_deref(), Some(status));
        assert_eq!(parsed.steps, None);
    }
}

#[test]
fn prompt_result_decodes_steps_from_max_steps_fixture() {
    // Wire sample from the signals corpus (kimi.md record
    // `stream-turn_limit_reached-max_steps`).
    const MAX_STEPS_LINE: &str = include_str!(
        "../../../../../docs/research/signals/fixtures/kimi/wire-max-steps-reached.jsonl"
    );
    let value: Value = serde_json::from_str(MAX_STEPS_LINE.trim()).unwrap();
    let env = KimiEnvelope::classify(value).expect("classified");
    let KimiEnvelope::SuccessResponse { id, result } = env else {
        panic!("expected SuccessResponse");
    };
    assert_eq!(id.as_str(), Some("prompt-2"));
    let parsed: KimiPromptResult = serde_json::from_value(result).unwrap();
    assert_eq!(
        parsed.status.as_deref(),
        Some(KimiPromptResult::STATUS_MAX_STEPS_REACHED)
    );
    assert_eq!(parsed.steps, Some(100));
}

#[test]
fn question_request_decodes_current_nested_shape() {
    // Wire sample from the signals corpus (kimi.md record
    // `stream-human_input_requested-question_request`, Wire >= 1.4).
    const QUESTION_LINE: &str = include_str!(
        "../../../../../docs/research/signals/fixtures/kimi/wire-question-request.jsonl"
    );
    let value: Value = serde_json::from_str(QUESTION_LINE.trim()).unwrap();
    let env = KimiEnvelope::classify(value).expect("classified");
    let KimiEnvelope::Request { id, params } = env else {
        panic!("expected Request");
    };
    // The synthetic empty-answer response must be keyed on the JSON-RPC
    // envelope id, not the payload's `id` or `tool_call_id`.
    assert_eq!(id.as_str(), Some("question-1"));
    let Some(KimiWireRequest::Question(question)) = params.into_request() else {
        panic!("expected QuestionRequest");
    };
    assert_eq!(question.id.as_deref(), Some("q-1"));
    assert_eq!(question.tool_call_id.as_deref(), Some("toolu-question"));
    let items = question.questions.as_ref().expect("questions");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].header.as_deref(), Some("Direction"));
    assert_eq!(items[0].multi_select, Some(false));
    assert_eq!(
        question.primary_question(),
        Some("Choose an implementation direction.")
    );
}

#[test]
fn question_request_tolerates_legacy_flat_shape() {
    let payload = serde_json::json!({
        "id": "q-9",
        "question": "Pick one",
        "options": ["a", "b"],
    });
    let parsed: KimiQuestionRequest = serde_json::from_value(payload).unwrap();
    assert_eq!(parsed.primary_question(), Some("Pick one"));
    assert!(parsed.questions.is_none());
    assert!(parsed.tool_call_id.is_none());
}
