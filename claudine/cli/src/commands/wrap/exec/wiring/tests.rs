//! Kimi wire-protocol tests: JSON-RPC request builders, `WireWriter`
//! streaming, request classification, hook-event mapping, dispatch result
//! translation, and the high-level request dispatch glue.

use super::*;
use serde_json::json;

#[test]
fn build_initialize_request_sends_canonical_protocol_version() {
    let value = build_initialize_request(
        "claudine",
        "0.1.0",
        WireClientCapabilities::default_for_claudine(),
    );
    assert_eq!(value["jsonrpc"], "2.0");
    assert_eq!(value["id"], INITIALIZE_REQUEST_ID);
    assert_eq!(value["method"], "initialize");
    assert_eq!(value["params"]["protocol_version"], WIRE_PROTOCOL_VERSION);
    assert_eq!(value["params"]["client"]["name"], "claudine");
    assert_eq!(value["params"]["client"]["version"], "0.1.0");
    assert_eq!(value["params"]["capabilities"]["approvals"], true);
    assert_eq!(value["params"]["capabilities"]["supports_question"], false);
    assert_eq!(value["params"]["capabilities"]["hooks"], true);
    assert_eq!(value["params"]["capabilities"]["subagents"], true);
    assert_eq!(value["params"]["capabilities"]["supports_plan_mode"], false);
}

#[test]
fn build_prompt_request_uses_user_input_field() {
    let value = build_prompt_request("Hi how are you?");
    assert_eq!(value["jsonrpc"], "2.0");
    assert_eq!(value["id"], PROMPT_REQUEST_ID);
    assert_eq!(value["method"], "prompt");
    assert_eq!(value["params"]["user_input"], "Hi how are you?");
    assert!(value["params"].get("prompt").is_none());
}

#[test]
fn build_cancel_request_uses_cancel_method_and_distinct_id() {
    let value = build_cancel_request();
    assert_eq!(value["method"], "cancel");
    assert_eq!(value["id"], CANCEL_REQUEST_ID);
    assert!(value["id"] != PROMPT_REQUEST_ID);
    assert_eq!(value["params"], json!({}));
}

#[test]
fn build_approval_response_carries_approve_decision() {
    let value = build_approval_response(json!("req-1"));
    assert_eq!(value["id"], "req-1");
    assert_eq!(value["result"]["response"], "approve");
    assert!(value.get("error").is_none());
}

#[test]
fn build_question_response_returns_empty_answer() {
    let value = build_question_response(json!("req-2"));
    assert_eq!(value["id"], "req-2");
    assert_eq!(value["result"]["answer"], "");
}

#[test]
fn build_tool_call_unsupported_error_uses_method_not_found_code() {
    let value = build_tool_call_unsupported_error(json!("req-3"));
    assert_eq!(value["id"], "req-3");
    assert_eq!(value["error"]["code"], KimiJsonRpcError::METHOD_NOT_FOUND);
    assert!(
        value["error"]["message"]
            .as_str()
            .unwrap()
            .contains("not supported")
    );
}

#[test]
fn build_hook_response_maps_allow_to_approve() {
    let value = build_hook_response(
        json!("req-4"),
        HookOutcome::Allow {
            reason: Some("ok".to_string()),
        },
    );
    assert_eq!(value["result"]["decision"], "approve");
    assert_eq!(value["result"]["reason"], "ok");
}

#[test]
fn build_hook_response_maps_deny_to_reject() {
    let value = build_hook_response(json!("req-5"), HookOutcome::Deny { reason: None });
    assert_eq!(value["result"]["decision"], "reject");
    assert!(value["result"].get("reason").is_none());
}

#[test]
fn build_hook_response_maps_ask_to_ask() {
    let value = build_hook_response(json!("req-6"), HookOutcome::Ask { reason: None });
    assert_eq!(value["result"]["decision"], "ask");
}

#[test]
fn writer_serializes_with_trailing_newline_and_flushes() {
    struct CaptureWriter {
        buf: Arc<Mutex<Vec<u8>>>,
        flushed: Arc<AtomicBool>,
    }
    impl Write for CaptureWriter {
        fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
            self.buf.lock().unwrap().extend_from_slice(data);
            Ok(data.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            self.flushed.store(true, Ordering::Relaxed);
            Ok(())
        }
    }

    let buf = Arc::new(Mutex::new(Vec::new()));
    let flushed = Arc::new(AtomicBool::new(false));
    let writer = WireWriter::from_writer(Box::new(CaptureWriter {
        buf: Arc::clone(&buf),
        flushed: Arc::clone(&flushed),
    }));

    let value = json!({"hello": "world"});
    let serialized = writer.send_value(&value).expect("send_value succeeds");
    let captured = buf.lock().unwrap().clone();
    let captured_str = String::from_utf8(captured).unwrap();
    assert_eq!(captured_str, format!("{serialized}\n"));
    assert!(flushed.load(Ordering::Relaxed));
    assert!(captured_str.ends_with('\n'));
}

#[test]
fn writer_send_then_send_writes_two_lines() {
    struct CaptureWriter(Arc<Mutex<Vec<u8>>>);
    impl Write for CaptureWriter {
        fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(data);
            Ok(data.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let buf = Arc::new(Mutex::new(Vec::new()));
    let writer = WireWriter::from_writer(Box::new(CaptureWriter(Arc::clone(&buf))));
    writer.send_value(&json!({"a": 1})).unwrap();
    writer.send_value(&json!({"b": 2})).unwrap();
    let captured = buf.lock().unwrap().clone();
    let text = String::from_utf8(captured).unwrap();
    let lines: Vec<&str> = text.split_terminator('\n').collect();
    assert_eq!(lines.len(), 2);
    assert_eq!(
        serde_json::from_str::<Value>(lines[0]).unwrap(),
        json!({"a": 1})
    );
    assert_eq!(
        serde_json::from_str::<Value>(lines[1]).unwrap(),
        json!({"b": 2})
    );
}

#[test]
fn writer_clones_share_one_pipe() {
    struct CaptureWriter(Arc<Mutex<Vec<u8>>>);
    impl Write for CaptureWriter {
        fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(data);
            Ok(data.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let buf = Arc::new(Mutex::new(Vec::new()));
    let writer = WireWriter::from_writer(Box::new(CaptureWriter(Arc::clone(&buf))));
    let clone = writer.clone();
    writer.send_value(&json!({"a": 1})).unwrap();
    clone.send_value(&json!({"b": 2})).unwrap();
    let captured = buf.lock().unwrap().clone();
    let text = String::from_utf8(captured).unwrap();
    let lines: Vec<&str> = text.split_terminator('\n').collect();
    assert_eq!(lines.len(), 2);
}

#[test]
fn dispatch_for_request_classifies_each_kimi_variant() {
    use claudine::stream::protocol::kimi::{
        KimiApprovalRequest, KimiHookRequest, KimiQuestionRequest, KimiToolCallRequest,
    };
    assert!(matches!(
        dispatch_for_request(&KimiWireRequest::Approval(KimiApprovalRequest::default())),
        WireRequestDispatch::AutoApprove
    ));
    assert!(matches!(
        dispatch_for_request(&KimiWireRequest::Question(KimiQuestionRequest::default())),
        WireRequestDispatch::EmptyQuestionAnswer
    ));
    assert!(matches!(
        dispatch_for_request(&KimiWireRequest::ToolCall(KimiToolCallRequest::default())),
        WireRequestDispatch::UnsupportedToolCall
    ));
    assert!(matches!(
        dispatch_for_request(&KimiWireRequest::Hook(KimiHookRequest::default())),
        WireRequestDispatch::HookRequest(_)
    ));
}

#[test]
fn map_kimi_hook_event_covers_canonical_aliases() {
    assert_eq!(
        map_kimi_hook_event("PreToolUse"),
        Some(AgenticEvent::BeforeTool)
    );
    assert_eq!(
        map_kimi_hook_event("PostToolUse"),
        Some(AgenticEvent::AfterTool)
    );
    assert_eq!(
        map_kimi_hook_event("Stop"),
        Some(AgenticEvent::TurnComplete)
    );
    assert_eq!(
        map_kimi_hook_event("UserPromptSubmit"),
        Some(AgenticEvent::BeforePrompt)
    );
    assert!(map_kimi_hook_event("UnknownEvent").is_none());
}

#[test]
fn validate_initialize_response_accepts_negotiated_version() {
    let result = KimiInitializeResult {
        protocol_version: Some(WIRE_PROTOCOL_VERSION.to_string()),
        ..Default::default()
    };
    assert!(validate_initialize_response(&result).is_ok());
}

#[test]
fn validate_initialize_response_rejects_missing_version() {
    let result = KimiInitializeResult::default();
    let err = validate_initialize_response(&result).unwrap_err();
    assert!(matches!(err, WireInitError::MissingProtocolVersion));
}

#[test]
fn validate_initialize_response_rejects_unsupported_version() {
    let result = KimiInitializeResult {
        protocol_version: Some("0.9".to_string()),
        ..Default::default()
    };
    let err = validate_initialize_response(&result).unwrap_err();
    match err {
        WireInitError::UnsupportedProtocolVersion {
            negotiated,
            expected,
        } => {
            assert_eq!(negotiated, "0.9");
            assert_eq!(expected, WIRE_PROTOCOL_VERSION);
        }
        _ => panic!("expected UnsupportedProtocolVersion"),
    }
}

#[test]
fn outcome_to_hook_outcome_no_response_defaults_allow() {
    let outcome = claudine::dispatch::DispatchOutcome::default();
    assert!(matches!(
        outcome_to_hook_outcome(&outcome),
        HookOutcome::Allow { .. }
    ));
}

#[test]
fn outcome_to_hook_outcome_explicit_decisions() {
    let allow = claudine::dispatch::DispatchOutcome {
        response: Some(json!({"decision": "approve", "reason": "ok"})),
        ..Default::default()
    };
    match outcome_to_hook_outcome(&allow) {
        HookOutcome::Allow { reason } => assert_eq!(reason.as_deref(), Some("ok")),
        other => panic!("expected Allow, got {other:?}"),
    }

    let deny = claudine::dispatch::DispatchOutcome {
        response: Some(json!({"decision": "reject"})),
        ..Default::default()
    };
    assert!(matches!(
        outcome_to_hook_outcome(&deny),
        HookOutcome::Deny { reason: None }
    ));

    let ask = claudine::dispatch::DispatchOutcome {
        response: Some(json!({"decision": "ask"})),
        ..Default::default()
    };
    assert!(matches!(
        outcome_to_hook_outcome(&ask),
        HookOutcome::Ask { reason: None }
    ));
}

#[test]
fn outcome_to_hook_outcome_protect_block_maps_to_deny() {
    use claudine::protect::catalog::{RuleGroup, ScanSurface};
    use claudine::protect::decision::{ProtectDecision, ProtectMatch};
    let block = ProtectDecision::blocked(ProtectMatch {
        group: RuleGroup::FilesystemDestruction,
        rule_id: "test_block".to_string(),
        pattern: "test".to_string(),
        matched_text: "test".to_string(),
        surface: ScanSurface::BashCommand,
        target_path: None,
        config_key: "protect.rules.filesystem_destruction".to_string(),
    });
    let outcome = claudine::dispatch::DispatchOutcome {
        response: None,
        exit_code: None,
        protect_pre: Some(block),
        protect_post: None,
    };
    match outcome_to_hook_outcome(&outcome) {
        HookOutcome::Deny { reason } => {
            assert!(reason.unwrap().contains("Protect"));
        }
        other => panic!("expected Deny, got {other:?}"),
    }
}

#[test]
fn dispatch_hook_request_unknown_event_returns_allow() {
    let request = KimiHookRequest {
        event: Some("Mystery".into()),
        ..Default::default()
    };
    let runtime_context = claudine::dispatch::DispatchRuntimeContext::default();
    let result = dispatch_hook_request(
        &request,
        None,
        &runtime_context,
        &EnvironmentContext::default(),
        None,
    );
    assert!(matches!(result.outcome, HookOutcome::Allow { .. }));
    assert!(
        result.warning.is_none(),
        "unknown events are silent fallbacks, not user-facing warnings: {:?}",
        result.warning
    );
}

#[test]
fn dispatch_hook_request_missing_event_returns_allow() {
    let request = KimiHookRequest::default();
    let runtime_context = claudine::dispatch::DispatchRuntimeContext::default();
    let result = dispatch_hook_request(
        &request,
        None,
        &runtime_context,
        &EnvironmentContext::default(),
        None,
    );
    assert!(matches!(result.outcome, HookOutcome::Allow { .. }));
    assert!(result.warning.is_none());
}

/// Review-2 Finding — Kimi wire `HookRequest` dispatch must stamp both
/// `claudine_pid` (via `EnvironmentContext`) and `agent_pid` onto the
/// dispatched [`EventMeta`] after a successful spawn.
///
/// `build_hook_event_meta` is the meta-construction step
/// `dispatch_hook_request` runs immediately before
/// `dispatch_event_meta_with_runtime`, so asserting its output proves the
/// hook/action context, dispatch JSONL, and reporting ingest all carry the
/// PIDs. Both the typed fields and their `extra` mirrors are verified.
#[test]
fn build_hook_event_meta_stamps_pids_and_mirrors() {
    let request = KimiHookRequest {
        event: Some("PreToolUse".into()),
        context: Some(json!({
            "tool_name": "Bash",
            "session_id": "sess-kimi-pid",
        })),
        ..Default::default()
    };
    let env = EnvironmentContext {
        claudine_pid: Some(12_345),
        ..Default::default()
    };

    let meta = build_hook_event_meta(&request, AgenticEvent::BeforeTool, &env, Some(67_890));

    // Typed fields remain authoritative for JSONL and SQL ingest.
    assert_eq!(meta.env.claudine_pid, Some(12_345));
    assert_eq!(meta.agent_pid, Some(67_890));
    // Request context still flows through to the typed slots.
    assert_eq!(meta.tool_name.as_deref(), Some("Bash"));
    assert_eq!(meta.session_id.as_deref(), Some("sess-kimi-pid"));
    // `extra` mirrors expose both PIDs to templates and expressions.
    assert_eq!(
        meta.extra.get("claudine_pid").and_then(Value::as_u64),
        Some(12_345)
    );
    assert_eq!(
        meta.extra.get("agent_pid").and_then(Value::as_u64),
        Some(67_890)
    );
}

/// Without a spawned child the wire path has no `agent_pid` to report and
/// the env carries no `claudine_pid`; the meta must omit both rather than
/// fabricate them.
#[test]
fn build_hook_event_meta_omits_pids_when_unavailable() {
    let request = KimiHookRequest {
        event: Some("Stop".into()),
        ..Default::default()
    };

    let meta = build_hook_event_meta(
        &request,
        AgenticEvent::TurnComplete,
        &EnvironmentContext::default(),
        None,
    );

    assert!(meta.agent_pid.is_none());
    assert!(meta.env.claudine_pid.is_none());
    assert!(!meta.extra.contains_key("claudine_pid"));
    assert!(!meta.extra.contains_key("agent_pid"));
}

#[test]
fn handle_request_dispatch_writes_approval_for_approval_request() {
    struct CaptureWriter(Arc<Mutex<Vec<u8>>>);
    impl Write for CaptureWriter {
        fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(data);
            Ok(data.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let buf = Arc::new(Mutex::new(Vec::new()));
    let writer = WireWriter::from_writer(Box::new(CaptureWriter(Arc::clone(&buf))));
    let runtime_context = claudine::dispatch::DispatchRuntimeContext::default();

    let line = json!({
        "jsonrpc": "2.0",
        "method": "request",
        "id": "req-99",
        "params": {
            "type": "ApprovalRequest",
            "payload": {
                "id": "approval-1",
                "tool_call_id": "tool-1"
            }
        }
    });
    let trimmed = serde_json::to_string(&line).unwrap();
    let synthetic = handle_request_dispatch(
        &trimmed,
        &writer,
        None,
        &runtime_context,
        &EnvironmentContext::default(),
        None,
    );
    assert!(synthetic.is_none(), "approval requests are not diagnostics");

    let captured = buf.lock().unwrap().clone();
    let text = String::from_utf8(captured).unwrap();
    let response: Value = serde_json::from_str(text.trim()).unwrap();
    assert_eq!(response["id"], "req-99");
    assert_eq!(response["result"]["response"], "approve");
}

#[test]
fn handle_request_dispatch_ignores_notifications() {
    struct CaptureWriter(Arc<Mutex<Vec<u8>>>);
    impl Write for CaptureWriter {
        fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(data);
            Ok(data.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let buf = Arc::new(Mutex::new(Vec::new()));
    let writer = WireWriter::from_writer(Box::new(CaptureWriter(Arc::clone(&buf))));
    let runtime_context = claudine::dispatch::DispatchRuntimeContext::default();

    let line = json!({
        "jsonrpc": "2.0",
        "method": "event",
        "params": {"type": "TurnEnd", "payload": {}}
    });
    let trimmed = serde_json::to_string(&line).unwrap();
    let synthetic = handle_request_dispatch(
        &trimmed,
        &writer,
        None,
        &runtime_context,
        &EnvironmentContext::default(),
        None,
    );
    assert!(
        synthetic.is_none(),
        "notifications produce no synthetic envelopes"
    );
    assert!(buf.lock().unwrap().is_empty());
}

#[test]
fn handle_request_dispatch_writes_method_not_found_for_tool_call_request() {
    struct CaptureWriter(Arc<Mutex<Vec<u8>>>);
    impl Write for CaptureWriter {
        fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(data);
            Ok(data.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let buf = Arc::new(Mutex::new(Vec::new()));
    let writer = WireWriter::from_writer(Box::new(CaptureWriter(Arc::clone(&buf))));
    let runtime_context = claudine::dispatch::DispatchRuntimeContext::default();
    let line = json!({
        "jsonrpc": "2.0",
        "method": "request",
        "id": "req-tool",
        "params": {"type": "ToolCallRequest", "payload": {"id": "x"}}
    });
    let trimmed = serde_json::to_string(&line).unwrap();
    let synthetic = handle_request_dispatch(
        &trimmed,
        &writer,
        None,
        &runtime_context,
        &EnvironmentContext::default(),
        None,
    );
    assert!(
        synthetic.is_none(),
        "tool call rejections are not diagnostics"
    );
    let captured = buf.lock().unwrap().clone();
    let text = String::from_utf8(captured).unwrap();
    let response: Value = serde_json::from_str(text.trim()).unwrap();
    assert_eq!(response["id"], "req-tool");
    assert_eq!(
        response["error"]["code"],
        KimiJsonRpcError::METHOD_NOT_FOUND
    );
}

#[test]
fn build_synthetic_warning_envelope_round_trips_as_error_notification() {
    let envelope = build_synthetic_warning_envelope("Hook dispatch failed: boom");
    assert_eq!(envelope["jsonrpc"], "2.0");
    assert_eq!(envelope["method"], "event");
    assert_eq!(envelope["params"]["type"], "Notification");
    assert_eq!(envelope["params"]["payload"]["level"], "error");
    assert_eq!(envelope["params"]["payload"]["source"], "claudine");
    assert_eq!(
        envelope["params"]["payload"]["message"],
        "Hook dispatch failed: boom"
    );
    // Must classify as a Notification envelope so the parser routes it.
    let classified = KimiEnvelope::classify(envelope.clone());
    assert!(
        matches!(classified, Some(KimiEnvelope::Notification(_))),
        "synthetic envelope must classify as a wire Notification"
    );
}

#[test]
fn hook_dispatch_result_allow_with_warning_carries_message() {
    let result = HookDispatchResult::allow_with_warning("dispatch boom");
    assert!(matches!(result.outcome, HookOutcome::Allow { .. }));
    assert_eq!(result.warning.as_deref(), Some("dispatch boom"));
}

#[test]
fn hook_dispatch_result_default_is_silent_allow() {
    let result = HookDispatchResult::allow_default();
    assert!(matches!(result.outcome, HookOutcome::Allow { .. }));
    assert!(result.warning.is_none());
}

#[test]
fn is_prompt_response_line_matches_finished_status() {
    let line = format!(
        r#"{{"jsonrpc":"2.0","id":"{PROMPT_REQUEST_ID}","result":{{"status":"finished"}}}}"#
    );
    assert!(is_prompt_response_line(&line));
}

#[test]
fn is_prompt_response_line_matches_error_response() {
    let line = format!(
        r#"{{"jsonrpc":"2.0","id":"{PROMPT_REQUEST_ID}","error":{{"code":-32004,"message":"auth"}}}}"#
    );
    assert!(is_prompt_response_line(&line));
}

#[test]
fn is_prompt_response_line_rejects_other_ids() {
    let line = format!(
        r#"{{"jsonrpc":"2.0","id":"{INITIALIZE_REQUEST_ID}","result":{{"protocol_version":"1.9"}}}}"#
    );
    assert!(!is_prompt_response_line(&line));
}

#[test]
fn is_prompt_response_line_rejects_event_envelopes() {
    let line = r#"{"jsonrpc":"2.0","method":"event","params":{"type":"TurnEnd","payload":{}}}"#;
    assert!(!is_prompt_response_line(line));
}

#[test]
fn is_prompt_response_line_rejects_garbage() {
    assert!(!is_prompt_response_line("not json"));
}

#[test]
fn close_stdin_drops_underlying_writer_and_redirects_to_sink() {
    struct DropTracker(Arc<AtomicBool>);
    impl Write for DropTracker {
        fn write(&mut self, _data: &[u8]) -> std::io::Result<usize> {
            Ok(_data.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    impl Drop for DropTracker {
        fn drop(&mut self) {
            self.0.store(true, Ordering::Relaxed);
        }
    }

    let dropped = Arc::new(AtomicBool::new(false));
    let writer = WireWriter::from_writer(Box::new(DropTracker(Arc::clone(&dropped))));
    assert!(!dropped.load(Ordering::Relaxed));
    writer.close_stdin();
    assert!(
        dropped.load(Ordering::Relaxed),
        "close_stdin must drop the original ChildStdin so kimi sees EOF"
    );
    // Subsequent send_value calls succeed silently against the sink.
    writer
        .send_value(&json!({"after_close": true}))
        .expect("send_value after close_stdin should succeed against sink");
}
