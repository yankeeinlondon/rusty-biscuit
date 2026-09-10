use std::sync::{Arc, Mutex};

use super::*;
use serde_json::json;

struct Recording {
    events: Arc<Mutex<Vec<SemanticEvent>>>,
}
impl SemanticEventSink for Recording {
    fn on_semantic_event(&mut self, event: SemanticEvent) {
        self.events.lock().unwrap().push(event);
    }
}

fn new_parser() -> (
    Arc<Mutex<Vec<SemanticEvent>>>,
    Box<KimiSemanticStreamParser<Recording>>,
) {
    let events = Arc::new(Mutex::new(Vec::new()));
    let sink = Recording {
        events: events.clone(),
    };
    (events, Box::new(KimiSemanticStreamParser::new(sink)))
}

fn kinds(events: &[SemanticEvent]) -> Vec<&'static str> {
    events.iter().map(|e| e.kind_str()).collect()
}

fn feed_initialize(parser: &mut KimiSemanticStreamParser<Recording>) {
    parser
        .feed_line(
            r#"{"jsonrpc":"2.0","id":"init-1","result":{"protocol_version":"1.9","server":{"name":"Kimi Code CLI","version":"1.38.0"},"slash_commands":[],"hooks":[],"capabilities":{"supports_question":true}}}"#,
        );
}

#[test]
fn initialize_response_emits_session_start() {
    let (events, mut parser) = new_parser();
    feed_initialize(&mut parser);
    let collected = events.lock().unwrap().clone();
    assert!(matches!(collected[0], SemanticEvent::SessionStart { .. }));
    if let SemanticEvent::SessionStart { extra, model, .. } = &collected[0] {
        assert_eq!(model.as_deref(), Some("Kimi Code CLI"));
        assert_eq!(
            extra.get("protocol_version").and_then(Value::as_str),
            Some("1.9")
        );
    }
}

#[test]
fn initialize_response_accepts_wire_1_10() {
    let (events, mut parser) = new_parser();
    parser
        .feed_line(
            r#"{"jsonrpc":"2.0","id":"init-1","result":{"protocol_version":"1.10","server":{"name":"Kimi Code CLI","version":"1.47.0"},"slash_commands":[],"hooks":[],"capabilities":{"supports_question":true}}}"#,
        );
    let collected = events.lock().unwrap().clone();
    assert!(matches!(collected[0], SemanticEvent::SessionStart { .. }));
    if let SemanticEvent::SessionStart { extra, .. } = &collected[0] {
        assert_eq!(
            extra.get("protocol_version").and_then(Value::as_str),
            Some("1.10")
        );
    }
    assert!(
        !collected
            .iter()
            .any(|e| matches!(e, SemanticEvent::Error { .. })),
        "1.10 is inside the supported window and must not error"
    );
}

#[test]
fn initialize_response_unknown_version_emits_terminal_configuration_error() {
    let (events, mut parser) = new_parser();
    parser
        .feed_line(
            r#"{"jsonrpc":"2.0","id":"init-1","result":{"protocol_version":"2.0","server":{"name":"Kimi Code CLI","version":"9.9.9"}}}"#,
        );
    let collected = events.lock().unwrap().clone();
    let err = collected
        .iter()
        .find_map(|e| match e {
            SemanticEvent::Error {
                message,
                kind,
                terminal,
                extra,
            } => Some((message.clone(), *kind, *terminal, extra.clone())),
            _ => None,
        })
        .expect("terminal error");
    assert!(err.2, "unsupported version must be terminal");
    assert_eq!(err.1, SemanticErrorKind::Configuration);
    assert!(err.0.contains("2.0"), "message names the negotiated version");
    assert!(err.0.contains("1.9") && err.0.contains("1.10"));
    assert!(
        err.0.contains("Upgrade Claudine") && err.0.contains("kimi --version"),
        "message carries remediation: {}",
        err.0
    );
    assert_eq!(
        err.3.get("protocol_version").and_then(Value::as_str),
        Some("2.0")
    );
    assert!(
        !collected
            .iter()
            .any(|e| matches!(e, SemanticEvent::SessionStart { .. })),
        "no SessionStart after a failed handshake"
    );
    let summary = parser.finish(1);
    assert!(summary.is_error);
    assert_eq!(
        summary.error_kind.as_deref(),
        Some("unsupported_protocol_version")
    );
}

#[test]
fn initialize_response_missing_version_stays_lenient() {
    let (events, mut parser) = new_parser();
    parser
        .feed_line(
            r#"{"jsonrpc":"2.0","id":"init-1","result":{"server":{"name":"Kimi Code CLI","version":"1.38.0"}}}"#,
        );
    let collected = events.lock().unwrap().clone();
    assert!(matches!(collected[0], SemanticEvent::SessionStart { .. }));
    assert!(
        !collected
            .iter()
            .any(|e| matches!(e, SemanticEvent::Error { .. })),
        "absent protocol_version must not be fatal"
    );
}

#[test]
fn turn_begin_emits_turn_start() {
    let (events, mut parser) = new_parser();
    feed_initialize(&mut parser);
    parser
        .feed_line(
            r#"{"jsonrpc":"2.0","method":"event","params":{"type":"TurnBegin","payload":{"user_input":"Hi Bob"}}}"#,
        );
    let collected = events.lock().unwrap().clone();
    assert!(
        collected
            .iter()
            .any(|e| matches!(e, SemanticEvent::TurnStart { .. }))
    );
}

#[test]
fn content_part_text_emits_output_text() {
    let (events, mut parser) = new_parser();
    feed_initialize(&mut parser);
    parser
        .feed_line(
            r#"{"jsonrpc":"2.0","method":"event","params":{"type":"ContentPart","payload":{"type":"text","text":"Hello "}}}"#,
        );
    parser
        .feed_line(
            r#"{"jsonrpc":"2.0","method":"event","params":{"type":"ContentPart","payload":{"type":"text","text":"world"}}}"#,
        );
    let collected = events.lock().unwrap().clone();
    let texts: Vec<&str> = collected
        .iter()
        .filter_map(|e| match e {
            SemanticEvent::OutputText { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(texts, vec!["Hello ", "world"]);
    let summary = parser.finish(0);
    assert_eq!(summary.assistant_text, "Hello world");
}

#[test]
fn content_part_think_emits_reasoning() {
    let (events, mut parser) = new_parser();
    feed_initialize(&mut parser);
    parser
        .feed_line(
            r#"{"jsonrpc":"2.0","method":"event","params":{"type":"ContentPart","payload":{"type":"think","think":"pondering"}}}"#,
        );
    // Thinking tokens are accumulated; a turn boundary triggers the flush.
    parser
        .feed_line(
            r#"{"jsonrpc":"2.0","method":"event","params":{"type":"TurnEnd","payload":{}}}"#,
        );
    assert!(events.lock().unwrap().iter().any(|e| matches!(e,
            SemanticEvent::Reasoning { text, .. } if text == "pondering")));
}

#[test]
fn content_part_think_chunks_coalesce_into_one_reasoning_event() {
    let (events, mut parser) = new_parser();
    feed_initialize(&mut parser);
    for chunk in ["The", " user", " said", " hi"] {
        let line = format!(
            r#"{{"jsonrpc":"2.0","method":"event","params":{{"type":"ContentPart","payload":{{"type":"think","think":"{chunk}"}}}}}}"#,
        );
        parser.feed_line(&line);
    }
    // No flush yet — accumulator still buffering.
    let mid = events.lock().unwrap().clone();
    assert!(
        !mid.iter()
            .any(|e| matches!(e, SemanticEvent::Reasoning { .. })),
        "thinking tokens must not emit per-token Reasoning events; got {mid:?}"
    );
    // A non-think content part flushes the accumulator.
    parser
        .feed_line(
            r#"{"jsonrpc":"2.0","method":"event","params":{"type":"ContentPart","payload":{"type":"text","text":"Hi!"}}}"#,
        );
    let collected = events.lock().unwrap().clone();
    let reasoning_texts: Vec<&str> = collected
        .iter()
        .filter_map(|e| match e {
            SemanticEvent::Reasoning { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(
        reasoning_texts,
        vec!["The user said hi"],
        "consecutive think chunks must coalesce into a single Reasoning event"
    );
}

#[test]
fn pending_thinking_flushes_on_finish() {
    let (events, mut parser) = new_parser();
    feed_initialize(&mut parser);
    parser
        .feed_line(
            r#"{"jsonrpc":"2.0","method":"event","params":{"type":"ContentPart","payload":{"type":"think","think":"trailing thoughts"}}}"#,
        );
    let _ = parser.finish(0);
    assert!(
        events
            .lock()
            .unwrap()
            .iter()
            .any(|e| matches!(e, SemanticEvent::Reasoning { text, .. } if text == "trailing thoughts")),
        "finish must flush any pending thinking accumulator"
    );
}

#[test]
fn status_update_above_threshold_emits_warning() {
    let (events, mut parser) = new_parser();
    feed_initialize(&mut parser);
    parser
        .feed_line(
            r#"{"jsonrpc":"2.0","method":"event","params":{"type":"StatusUpdate","payload":{"context_usage":0.85,"context_tokens":110000,"max_context_tokens":128000}}}"#,
        );
    let collected = events.lock().unwrap().clone();
    assert!(
        collected
            .iter()
            .any(|e| matches!(e, SemanticEvent::Warning { message, .. } if message.contains("Context window pressure"))),
        "kinds = {:?}",
        kinds(&collected)
    );
}

#[test]
fn status_update_below_threshold_no_warning() {
    let (events, mut parser) = new_parser();
    feed_initialize(&mut parser);
    parser
        .feed_line(
            r#"{"jsonrpc":"2.0","method":"event","params":{"type":"StatusUpdate","payload":{"context_usage":0.06,"context_tokens":15969,"max_context_tokens":262144,"token_usage":{"input_other":10000,"input_cache_read":5000,"output":50}}}}"#,
        );
    let collected = events.lock().unwrap().clone();
    assert!(
        !collected
            .iter()
            .any(|e| matches!(e, SemanticEvent::Warning { .. }))
    );
}

#[test]
fn step_retry_emits_warning_with_retry_observability() {
    let (events, mut parser) = new_parser();
    feed_initialize(&mut parser);
    parser
        .feed_line(
            r#"{"jsonrpc":"2.0","method":"event","params":{"type":"StepRetry","payload":{"n":1,"next_attempt":2,"max_attempts":5,"wait_s":1.5,"error_type":"APIEmptyResponseError","status_code":500}}}"#,
        );
    let collected = events.lock().unwrap().clone();
    let warning = collected
        .iter()
        .find_map(|e| match e {
            SemanticEvent::Warning { message, extra } => Some((message.clone(), extra.clone())),
            _ => None,
        })
        .expect("warning");
    assert!(warning.0.contains("attempt 2/5"), "message: {}", warning.0);
    assert!(warning.0.contains("APIEmptyResponseError"));
    assert!(warning.0.contains("HTTP 500"));
    assert_eq!(
        warning.1.get("kind").and_then(Value::as_str),
        Some("step_retry")
    );
    assert_eq!(warning.1.get("n").and_then(Value::as_u64), Some(1));
    assert_eq!(warning.1.get("wait_s").and_then(Value::as_f64), Some(1.5));
    assert_eq!(
        warning.1.get("error_type").and_then(Value::as_str),
        Some("APIEmptyResponseError")
    );
    assert_eq!(
        warning.1.get("status_code").and_then(Value::as_i64),
        Some(500)
    );
}

#[test]
fn step_retry_with_empty_payload_still_emits_warning() {
    let (events, mut parser) = new_parser();
    feed_initialize(&mut parser);
    parser
        .feed_line(
            r#"{"jsonrpc":"2.0","method":"event","params":{"type":"StepRetry","payload":{}}}"#,
        );
    assert!(events.lock().unwrap().iter().any(|e| matches!(e,
            SemanticEvent::Warning { message, .. } if message == "Step retry")));
}

#[test]
fn notification_1_10_shape_resolves_body_and_severity() {
    let (events, mut parser) = new_parser();
    feed_initialize(&mut parser);
    parser
        .feed_line(
            r#"{"jsonrpc":"2.0","method":"event","params":{"type":"Notification","payload":{"id":"notif-1","category":"task","type":"task.completed","source_kind":"background_task","source_id":"task-9","title":"Background task finished","body":"Task `lint` completed","severity":"warning","created_at":1751700000.25,"payload":{"exit_code":1}}}}"#,
        );
    let collected = events.lock().unwrap().clone();
    let warning = collected
        .iter()
        .find_map(|e| match e {
            SemanticEvent::Warning { message, extra } => Some((message.clone(), extra.clone())),
            _ => None,
        })
        .expect("severity=warning must surface as Warning");
    assert_eq!(warning.0, "Task `lint` completed", "message prefers body");
    assert_eq!(
        warning.1.get("category").and_then(Value::as_str),
        Some("task")
    );
    assert_eq!(
        warning.1.get("notification_type").and_then(Value::as_str),
        Some("task.completed")
    );
    assert_eq!(
        warning.1.get("source_kind").and_then(Value::as_str),
        Some("background_task")
    );
    assert_eq!(
        warning.1.get("severity").and_then(Value::as_str),
        Some("warning")
    );
    assert_eq!(
        warning.1.get("created_at").and_then(Value::as_f64),
        Some(1751700000.25)
    );
    assert_eq!(warning.1.get("payload"), Some(&json!({"exit_code": 1})));
}

#[test]
fn notification_1_10_info_severity_emits_info() {
    let (events, mut parser) = new_parser();
    feed_initialize(&mut parser);
    parser
        .feed_line(
            r#"{"jsonrpc":"2.0","method":"event","params":{"type":"Notification","payload":{"id":"notif-2","category":"task","type":"task.completed","source_kind":"background_task","source_id":"task-10","title":"Background task finished","body":"Task `build` completed","severity":"info","created_at":1751700001.0}}}"#,
        );
    let collected = events.lock().unwrap().clone();
    assert!(collected.iter().any(|e| matches!(e,
            SemanticEvent::Info { message, .. } if message == "Task `build` completed")));
    assert!(
        !collected
            .iter()
            .any(|e| matches!(e, SemanticEvent::Warning { .. }))
    );
}

#[test]
fn status_update_pressure_warning_carries_mcp_status() {
    let (events, mut parser) = new_parser();
    feed_initialize(&mut parser);
    parser
        .feed_line(
            r#"{"jsonrpc":"2.0","method":"event","params":{"type":"StatusUpdate","payload":{"context_usage":0.9,"context_tokens":118000,"max_context_tokens":131072,"mcp_status":{"loading":false,"connected":1,"total":2,"tools":4,"servers":[{"name":"weather","status":"connected","tools":["forecast"]},{"name":"broken","status":"failed"}]}}}}"#,
        );
    let collected = events.lock().unwrap().clone();
    let extra = collected
        .iter()
        .find_map(|e| match e {
            SemanticEvent::Warning { extra, .. } => Some(extra.clone()),
            _ => None,
        })
        .expect("pressure warning");
    let snapshot = extra.get("mcp_status").expect("mcp_status in extra");
    assert_eq!(snapshot.get("connected").and_then(Value::as_u64), Some(1));
    assert_eq!(snapshot.get("total").and_then(Value::as_u64), Some(2));
    assert_eq!(
        snapshot
            .get("servers")
            .and_then(Value::as_array)
            .map(Vec::len),
        Some(2)
    );
}

#[test]
fn tool_call_with_streamed_arguments_decodes() {
    let (events, mut parser) = new_parser();
    feed_initialize(&mut parser);
    parser
        .feed_line(
            r#"{"jsonrpc":"2.0","method":"event","params":{"type":"ToolCall","payload":{"type":"function","id":"tool_1","function":{"name":"Shell","arguments":""}}}}"#,
        );
    for delta in ["{\\\"", "command", "\\\":", " \\\"ls", "\\\"}"] {
        let line = format!(
            r#"{{"jsonrpc":"2.0","method":"event","params":{{"type":"ToolCallPart","payload":{{"arguments_part":"{delta}"}}}}}}"#
        );
        parser.feed_line(&line);
    }
    parser
        .feed_line(
            r#"{"jsonrpc":"2.0","method":"event","params":{"type":"ToolResult","payload":{"tool_call_id":"tool_1","return_value":{"is_error":false,"output":"ok"}}}}"#,
        );
    let collected = events.lock().unwrap().clone();
    let tool_call = collected
        .iter()
        .find_map(|e| match e {
            SemanticEvent::ToolCall {
                name, id, input, ..
            } => Some((name.clone(), id.clone(), input.clone())),
            _ => None,
        })
        .expect("tool_call");
    assert_eq!(tool_call.0.as_deref(), Some("Shell"));
    assert_eq!(tool_call.1.as_deref(), Some("tool_1"));
    let input = tool_call.2.expect("input");
    assert_eq!(input.get("command").and_then(Value::as_str), Some("ls"));
    assert!(
        collected
            .iter()
            .any(|e| matches!(e, SemanticEvent::ToolResult { .. }))
    );
}

#[test]
fn malformed_tool_arguments_pass_through_as_string() {
    let (events, mut parser) = new_parser();
    feed_initialize(&mut parser);
    parser
        .feed_line(
            r#"{"jsonrpc":"2.0","method":"event","params":{"type":"ToolCall","payload":{"id":"t1","function":{"name":"Shell","arguments":"{not json"}}}}"#,
        );
    parser
        .feed_line(
            r#"{"jsonrpc":"2.0","method":"event","params":{"type":"TurnEnd","payload":{}}}"#,
        );
    let collected = events.lock().unwrap().clone();
    let input = collected
        .iter()
        .find_map(|e| match e {
            SemanticEvent::ToolCall { input, .. } => input.clone(),
            _ => None,
        })
        .expect("tool input");
    assert_eq!(input.as_str(), Some("{not json"));
}

#[test]
fn approval_request_emits_auto_approved_info() {
    let (events, mut parser) = new_parser();
    feed_initialize(&mut parser);
    parser
        .feed_line(
            r#"{"jsonrpc":"2.0","method":"request","id":"req-1","params":{"type":"ApprovalRequest","payload":{"id":"req-1","tool_call_id":"t1","sender":"Shell","action":"run command","description":"Run command `ls`","display":[{"type":"shell","language":"bash","command":"ls"}]}}}"#,
        );
    let collected = events.lock().unwrap().clone();
    let info = collected
        .iter()
        .find_map(|e| match e {
            SemanticEvent::Info { message, extra } => Some((message.clone(), extra.clone())),
            _ => None,
        })
        .expect("info");
    assert!(info.0.contains("Auto-approved"));
    assert_eq!(
        info.1.get("kind").and_then(Value::as_str),
        Some("auto_approved")
    );
    assert_eq!(
        info.1.get("shell_command").and_then(Value::as_str),
        Some("ls")
    );
}

#[test]
fn unexpected_question_legacy_flat_shape_emits_warning() {
    // Pre-Wire-1.4 flat payload; kept as the legacy-tolerance test.
    let (events, mut parser) = new_parser();
    feed_initialize(&mut parser);
    parser
        .feed_line(
            r#"{"jsonrpc":"2.0","method":"request","id":"q-1","params":{"type":"QuestionRequest","payload":{"id":"q-1","question":"What now?"}}}"#,
        );
    let collected = events.lock().unwrap().clone();
    assert!(
        collected.iter().any(|e| matches!(e,
            SemanticEvent::Warning { message, .. }
                if message.contains("Unexpected question from agent: What now?")))
    );
}

#[test]
fn unexpected_question_current_nested_shape_emits_warning() {
    // Wire >= 1.4 sample from the signals corpus (kimi.md record
    // `stream-human_input_requested-question_request`).
    const QUESTION_LINE: &str = include_str!(
        "../../../../../docs/research/signals/fixtures/kimi/wire-question-request.jsonl"
    );
    let (events, mut parser) = new_parser();
    feed_initialize(&mut parser);
    parser.feed_line(QUESTION_LINE.trim());
    let collected = events.lock().unwrap().clone();
    let warning = collected
        .iter()
        .find_map(|e| match e {
            SemanticEvent::Warning { message, extra } => Some((message.clone(), extra.clone())),
            _ => None,
        })
        .expect("warning");
    assert!(warning.0.contains("Choose an implementation direction."));
    assert_eq!(
        warning.1.get("tool_call_id").and_then(Value::as_str),
        Some("toolu-question")
    );
    assert_eq!(
        warning.1.get("request_id").and_then(Value::as_str),
        Some("question-1")
    );
}

#[test]
fn external_tool_call_request_emits_warning() {
    let (events, mut parser) = new_parser();
    feed_initialize(&mut parser);
    parser
        .feed_line(
            r#"{"jsonrpc":"2.0","method":"request","id":"x-1","params":{"type":"ToolCallRequest","payload":{"id":"x-1","tool_call":{"name":"external"}}}}"#,
        );
    let collected = events.lock().unwrap().clone();
    assert!(collected.iter().any(|e| matches!(e,
            SemanticEvent::Warning { message, .. } if message.contains("external tool"))));
}

#[test]
fn hook_request_emits_info() {
    let (events, mut parser) = new_parser();
    feed_initialize(&mut parser);
    parser
        .feed_line(
            r#"{"jsonrpc":"2.0","method":"request","id":"h-1","params":{"type":"HookRequest","payload":{"id":"h-1","event":"PreToolUse","context":{"foo":"bar"}}}}"#,
        );
    let collected = events.lock().unwrap().clone();
    let info = collected
        .iter()
        .find_map(|e| match e {
            SemanticEvent::Info { message, extra } => Some((message.clone(), extra.clone())),
            _ => None,
        })
        .expect("info");
    assert!(info.0.contains("Hook request"));
    assert_eq!(
        info.1.get("event").and_then(Value::as_str),
        Some("PreToolUse")
    );
}

#[test]
fn prompt_finished_response_sets_status() {
    let (events, mut parser) = new_parser();
    feed_initialize(&mut parser);
    parser
        .feed_line(r#"{"jsonrpc":"2.0","id":"prompt-2","result":{"status":"finished"}}"#);
    let collected = events.lock().unwrap().clone();
    assert!(collected.iter().any(|e| matches!(e,
            SemanticEvent::Info { extra, .. } if extra.get("kind").and_then(Value::as_str) == Some("prompt_status"))));
    let summary = parser.finish(0);
    assert_eq!(summary.provider_status.as_deref(), Some("finished"));
    assert!(!summary.is_error);
}

#[test]
fn prompt_max_steps_response_surfaces_steps() {
    // Wire sample from the signals corpus (kimi.md record
    // `stream-turn_limit_reached-max_steps`).
    const MAX_STEPS_LINE: &str = include_str!(
        "../../../../../docs/research/signals/fixtures/kimi/wire-max-steps-reached.jsonl"
    );
    let (events, mut parser) = new_parser();
    feed_initialize(&mut parser);
    parser.feed_line(MAX_STEPS_LINE.trim());
    let collected = events.lock().unwrap().clone();
    let info = collected
        .iter()
        .find_map(|e| match e {
            SemanticEvent::Info { extra, .. }
                if extra.get("kind").and_then(Value::as_str) == Some("prompt_status") =>
            {
                Some(extra.clone())
            }
            _ => None,
        })
        .expect("prompt_status info");
    assert_eq!(
        info.get("status").and_then(Value::as_str),
        Some("max_steps_reached")
    );
    assert_eq!(info.get("steps").and_then(Value::as_u64), Some(100));
    let summary = parser.finish(0);
    assert_eq!(summary.provider_status.as_deref(), Some("max_steps_reached"));
}

#[test]
fn prompt_cancelled_response_emits_terminal_error() {
    let (events, mut parser) = new_parser();
    feed_initialize(&mut parser);
    parser
        .feed_line(r#"{"jsonrpc":"2.0","id":"prompt-2","result":{"status":"cancelled"}}"#);
    let collected = events.lock().unwrap().clone();
    let err = collected
        .iter()
        .find_map(|e| match e {
            SemanticEvent::Error {
                message,
                kind,
                terminal,
                ..
            } => Some((message.clone(), *kind, *terminal)),
            _ => None,
        })
        .expect("error");
    assert!(err.2);
    assert_eq!(err.1, SemanticErrorKind::Interrupted);
    assert!(err.0.contains("cancelled"));
    let summary = parser.finish(0);
    assert!(summary.is_error);
    assert_eq!(summary.provider_status.as_deref(), Some("cancelled"));
}

#[test]
fn auth_expired_error_response_classifies_configuration() {
    let (events, mut parser) = new_parser();
    feed_initialize(&mut parser);
    parser
        .feed_line(
            r#"{"jsonrpc":"2.0","id":"prompt-2","error":{"code":-32004,"message":"Authentication expired; please re-authenticate via `kimi login`","data":null}}"#,
        );
    let collected = events.lock().unwrap().clone();
    let err = collected
        .iter()
        .find_map(|e| match e {
            SemanticEvent::Error { kind, message, .. } => Some((*kind, message.clone())),
            _ => None,
        })
        .expect("error");
    assert_eq!(err.0, SemanticErrorKind::Configuration);
    assert!(err.1.contains("Authentication"));
}

#[test]
fn chat_provider_error_classifies_api_remote() {
    let kind = classify_jsonrpc_error(KimiJsonRpcError::CHAT_PROVIDER_ERROR, "boom");
    assert_eq!(kind, SemanticErrorKind::ApiRemote);
}

#[test]
fn classify_jsonrpc_error_message_keywords() {
    assert_eq!(
        classify_jsonrpc_error(0, "Rate limit exceeded"),
        SemanticErrorKind::ApiRemote
    );
    assert_eq!(
        classify_jsonrpc_error(0, "user cancelled"),
        SemanticErrorKind::Interrupted
    );
    assert_eq!(
        classify_jsonrpc_error(0, "invalid api key"),
        SemanticErrorKind::Configuration
    );
}

#[test]
fn classify_jsonrpc_error_code_wins_over_message() {
    // A known numeric code is matched before the message vocabulary: the
    // AUTH_EXPIRED code classifies as Configuration even though the message
    // text alone ("rate limit") would otherwise resolve to ApiRemote.
    assert_eq!(
        classify_jsonrpc_error(KimiJsonRpcError::AUTH_EXPIRED, "rate limit exceeded"),
        SemanticErrorKind::Configuration,
        "numeric code_buckets must take precedence over the message branch"
    );
}

#[test]
fn classify_jsonrpc_error_unknown_code_falls_through_to_message() {
    // An unrecognized numeric code carries no bucket, so classification
    // falls through to the message vocabulary rather than defaulting early.
    assert_eq!(
        classify_jsonrpc_error(12345, "billing quota exceeded"),
        SemanticErrorKind::ApiRemote
    );
    // With neither a known code nor a matching needle, the fallthrough is
    // the AgentNative default.
    assert_eq!(
        classify_jsonrpc_error(12345, "something inscrutable"),
        SemanticErrorKind::AgentNative
    );
}

#[test]
fn unknown_envelope_shape_falls_back_to_provider_extension() {
    let (events, mut parser) = new_parser();
    parser.feed_line(r#"{"jsonrpc":"2.0"}"#);
    assert!(matches!(
        events.lock().unwrap()[0],
        SemanticEvent::ProviderExtension { .. }
    ));
}

#[test]
fn missing_discriminator_falls_through_to_provider_extension() {
    let (events, mut parser) = new_parser();
    parser.feed_line(r#"{"payload":{"k":1}}"#);
    let collected = events.lock().unwrap().clone();
    assert_eq!(collected.len(), 1);
    match &collected[0] {
        SemanticEvent::ProviderExtension {
            provider,
            kind,
            payload,
        } => {
            assert_eq!(*provider, Provider::KimiCode);
            assert_eq!(kind, "");
            assert_eq!(payload.get("payload"), Some(&json!({"k": 1})));
        }
        other => panic!("expected ProviderExtension, got {other:?}"),
    }
}

#[test]
fn notification_with_error_level_emits_warning() {
    // Used by wire_io for hook dispatch failures: a synthetic
    // Notification envelope with `level: "error"` must surface as a
    // Warning so the user sees the diagnostic on the live stderr
    // surface, not as a quiet Info line.
    let (events, mut parser) = new_parser();
    feed_initialize(&mut parser);
    parser
        .feed_line(
            r#"{"jsonrpc":"2.0","method":"event","params":{"type":"Notification","payload":{"level":"error","source":"claudine","message":"Hook dispatch failed: boom"}}}"#,
        );
    let collected = events.lock().unwrap().clone();
    assert!(
        collected.iter().any(|e| matches!(e,
            SemanticEvent::Warning { message, .. } if message.contains("Hook dispatch failed"))),
        "error-level notification must surface as Warning; got {:?}",
        kinds(&collected)
    );
    assert!(
        !collected.iter().any(|e| matches!(e,
            SemanticEvent::Info { message, .. } if message.contains("Hook dispatch failed"))),
        "error-level notification must NOT also emit Info"
    );
}

#[test]
fn notification_with_warn_level_emits_warning() {
    let (events, mut parser) = new_parser();
    feed_initialize(&mut parser);
    parser
        .feed_line(
            r#"{"jsonrpc":"2.0","method":"event","params":{"type":"Notification","payload":{"level":"warn","message":"Approaching limit"}}}"#,
        );
    let collected = events.lock().unwrap().clone();
    assert!(collected.iter().any(|e| matches!(e,
            SemanticEvent::Warning { message, .. } if message.contains("Approaching limit"))));
}

#[test]
fn notification_with_info_level_emits_info() {
    // Default behavior is preserved: info/no-level notifications
    // remain Info events.
    let (events, mut parser) = new_parser();
    feed_initialize(&mut parser);
    parser
        .feed_line(
            r#"{"jsonrpc":"2.0","method":"event","params":{"type":"Notification","payload":{"level":"info","message":"Hello"}}}"#,
        );
    let collected = events.lock().unwrap().clone();
    assert!(collected.iter().any(|e| matches!(e,
            SemanticEvent::Info { message, .. } if message.contains("Hello"))));
    assert!(
        !collected
            .iter()
            .any(|e| matches!(e, SemanticEvent::Warning { .. })),
        "info-level notification must not surface as Warning"
    );
}

#[test]
fn known_wire_events_do_not_leak_as_provider_extension() {
    // Phase 5 contract: every Kimi wire event covered by the typed
    // protocol catalog must route through the typed semantic surface
    // (OutputText / Reasoning / ToolCall / ToolResult / Info /
    // Warning / PlanUpdate / TurnStart / TurnComplete / SessionStart),
    // never through the `ProviderExtension` fallback path. Unknown
    // events still fall back, which is verified separately above.
    let (events, mut parser) = new_parser();
    feed_initialize(&mut parser);
    for line in [
        r#"{"jsonrpc":"2.0","method":"event","params":{"type":"TurnBegin","payload":{"user_input":"hi"}}}"#,
        r#"{"jsonrpc":"2.0","method":"event","params":{"type":"StepBegin","payload":{"n":1}}}"#,
        r#"{"jsonrpc":"2.0","method":"event","params":{"type":"ContentPart","payload":{"type":"think","think":"pondering"}}}"#,
        r#"{"jsonrpc":"2.0","method":"event","params":{"type":"ContentPart","payload":{"type":"text","text":"hello"}}}"#,
        r#"{"jsonrpc":"2.0","method":"event","params":{"type":"ToolCall","payload":{"id":"t1","function":{"name":"Shell","arguments":"{\"command\":\"ls\"}"}}}}"#,
        r#"{"jsonrpc":"2.0","method":"event","params":{"type":"ToolCallPart","payload":{"arguments_part":""}}}"#,
        r#"{"jsonrpc":"2.0","method":"event","params":{"type":"ToolResult","payload":{"tool_call_id":"t1","return_value":{"is_error":false,"output":"ok"}}}}"#,
        r#"{"jsonrpc":"2.0","method":"event","params":{"type":"PlanDisplay","payload":{"plan":["step a"]}}}"#,
        r#"{"jsonrpc":"2.0","method":"event","params":{"type":"Notification","payload":{"message":"hi"}}}"#,
        r#"{"jsonrpc":"2.0","method":"event","params":{"type":"StatusUpdate","payload":{"context_usage":0.05,"context_tokens":100,"max_context_tokens":1000}}}"#,
        r#"{"jsonrpc":"2.0","method":"event","params":{"type":"CompactionBegin","payload":{}}}"#,
        r#"{"jsonrpc":"2.0","method":"event","params":{"type":"CompactionEnd","payload":{"tokens_saved":42}}}"#,
        r#"{"jsonrpc":"2.0","method":"event","params":{"type":"MCPLoadingBegin","payload":{"server":"weather"}}}"#,
        r#"{"jsonrpc":"2.0","method":"event","params":{"type":"MCPLoadingEnd","payload":{"server":"weather","status":"ok"}}}"#,
        r#"{"jsonrpc":"2.0","method":"event","params":{"type":"SubagentEvent","payload":{"subagent_type":"explore","event":{}}}}"#,
        r#"{"jsonrpc":"2.0","method":"event","params":{"type":"BtwBegin","payload":{"topic":"hi"}}}"#,
        r#"{"jsonrpc":"2.0","method":"event","params":{"type":"BtwEnd","payload":{}}}"#,
        r#"{"jsonrpc":"2.0","method":"event","params":{"type":"DiffDisplayBlock","payload":{"diff":"+a","is_summary":false}}}"#,
        r#"{"jsonrpc":"2.0","method":"event","params":{"type":"ApprovalResponse","payload":{"request_id":"r1","response":"approve"}}}"#,
        r#"{"jsonrpc":"2.0","method":"event","params":{"type":"SteerInput","payload":{}}}"#,
        r#"{"jsonrpc":"2.0","method":"event","params":{"type":"StepInterrupted","payload":{"reason":"cancel"}}}"#,
        r#"{"jsonrpc":"2.0","method":"event","params":{"type":"StepRetry","payload":{"n":1,"next_attempt":2,"max_attempts":5,"wait_s":0.5,"error_type":"APIEmptyResponseError","status_code":500}}}"#,
        r#"{"jsonrpc":"2.0","method":"event","params":{"type":"TurnEnd","payload":{}}}"#,
        r#"{"jsonrpc":"2.0","method":"request","id":"r1","params":{"type":"ApprovalRequest","payload":{"id":"r1"}}}"#,
        r#"{"jsonrpc":"2.0","method":"request","id":"q1","params":{"type":"QuestionRequest","payload":{"question":"?"}}}"#,
        r#"{"jsonrpc":"2.0","method":"request","id":"x1","params":{"type":"ToolCallRequest","payload":{"id":"x1"}}}"#,
        r#"{"jsonrpc":"2.0","method":"request","id":"h1","params":{"type":"HookRequest","payload":{"event":"PreToolUse"}}}"#,
    ] {
        parser.feed_line(line);
    }
    let collected = events.lock().unwrap().clone();
    let leaks: Vec<_> = collected
        .iter()
        .filter_map(|e| match e {
            SemanticEvent::ProviderExtension { kind, .. } => Some(kind.clone()),
            _ => None,
        })
        .collect();
    assert!(
        leaks.is_empty(),
        "known wire events must not surface as ProviderExtension; got leaks: {leaks:?}"
    );
}

#[test]
fn unknown_event_type_emits_provider_extension_with_event_kind() {
    // Unknown event types must still surface so operators can see
    // protocol drift, but the live sink's silent allowlist will
    // suppress the high-volume ones (see live_semantic_sink.rs).
    let (events, mut parser) = new_parser();
    parser
        .feed_line(
            r#"{"jsonrpc":"2.0","method":"event","params":{"type":"FutureKimiEvent","payload":{"x":1}}}"#,
        );
    let collected = events.lock().unwrap().clone();
    let ext_kind = collected
        .iter()
        .find_map(|e| match e {
            SemanticEvent::ProviderExtension { kind, .. } => Some(kind.clone()),
            _ => None,
        })
        .expect("ProviderExtension fallback must fire for unknown event types");
    assert_eq!(ext_kind, "event:FutureKimiEvent");
}

#[test]
fn malformed_json_emits_warning() {
    let (events, mut parser) = new_parser();
    parser.feed_line("x");
    assert!(matches!(
        events.lock().unwrap()[0],
        SemanticEvent::Warning { .. }
    ));
}

#[test]
fn turn_end_increments_num_turns_and_emits_turn_complete() {
    let (events, mut parser) = new_parser();
    feed_initialize(&mut parser);
    parser
        .feed_line(
            r#"{"jsonrpc":"2.0","method":"event","params":{"type":"TurnEnd","payload":{}}}"#,
        );
    let collected = events.lock().unwrap().clone();
    assert!(
        collected
            .iter()
            .any(|e| matches!(e, SemanticEvent::TurnComplete { .. }))
    );
    let summary = parser.finish(0);
    assert_eq!(summary.num_turns, Some(1));
}

#[test]
fn round_trip_fidelity_for_emitted_events() {
    let (events, mut parser) = new_parser();
    for line in [
        r#"{"jsonrpc":"2.0","id":"init-1","result":{"protocol_version":"1.9","server":{"name":"Kimi Code CLI","version":"1.38.0"},"slash_commands":[],"hooks":[],"capabilities":{"supports_question":true}}}"#,
        r#"{"jsonrpc":"2.0","method":"event","params":{"type":"TurnBegin","payload":{"user_input":"hi"}}}"#,
        r#"{"jsonrpc":"2.0","method":"event","params":{"type":"ContentPart","payload":{"type":"text","text":"hi"}}}"#,
        r#"{"jsonrpc":"2.0","method":"event","params":{"type":"TurnEnd","payload":{}}}"#,
        r#"{"jsonrpc":"2.0","id":"prompt-2","result":{"status":"finished"}}"#,
    ] {
        parser.feed_line(line);
    }
    for event in events.lock().unwrap().iter() {
        let v = serde_json::to_value(event).unwrap();
        let decoded: SemanticEvent = serde_json::from_value(v.clone()).unwrap();
        assert_eq!(v, serde_json::to_value(&decoded).unwrap());
    }
}
