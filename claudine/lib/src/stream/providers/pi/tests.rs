use std::sync::{Arc, Mutex};

use super::*;

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
    Box<PiSemanticStreamParser<Recording>>,
) {
    let events = Arc::new(Mutex::new(Vec::new()));
    let sink = Recording {
        events: events.clone(),
    };
    (
        events,
        Box::new(PiSemanticStreamParser::new(sink, Some("claude-opus-4-8".into()))),
    )
}

fn kinds(events: &[SemanticEvent]) -> Vec<&'static str> {
    events.iter().map(|e| e.kind_str()).collect()
}

#[test]
fn session_header_emits_session_start_with_cwd() {
    let (events, mut parser) = new_parser();
    parser
        .feed_line(r#"{"type":"session","version":3,"id":"s-1","cwd":"/work"}"#);
    let collected = events.lock().unwrap().clone();
    match &collected[0] {
        SemanticEvent::SessionStart {
            session_id,
            model,
            extra,
        } => {
            assert_eq!(session_id.as_deref(), Some("s-1"));
            assert_eq!(model.as_deref(), Some("claude-opus-4-8"));
            assert_eq!(extra.get("cwd"), Some(&Value::from("/work")));
        }
        other => panic!("expected SessionStart, got {other:?}"),
    }
}

#[test]
fn text_delta_emits_output_text_and_accumulates() {
    let (events, mut parser) = new_parser();
    parser
        .feed_line(
            r#"{"type":"message_update","assistantMessageEvent":{"type":"text_delta","delta":"Hello "}}"#,
        );
    parser
        .feed_line(
            r#"{"type":"message_update","assistantMessageEvent":{"type":"text_delta","delta":"world"}}"#,
        );
    assert_eq!(
        kinds(&events.lock().unwrap()),
        vec!["output_text", "output_text"]
    );
    let summary = parser.finish(0);
    assert_eq!(summary.assistant_text, "Hello world");
}

#[test]
fn thinking_delta_emits_reasoning() {
    let (events, mut parser) = new_parser();
    parser
        .feed_line(
            r#"{"type":"message_update","assistantMessageEvent":{"type":"thinking_delta","delta":"weighing options"}}"#,
        );
    let collected = events.lock().unwrap().clone();
    match &collected[0] {
        SemanticEvent::Reasoning { text, .. } => assert_eq!(text, "weighing options"),
        other => panic!("expected Reasoning, got {other:?}"),
    }
}

#[test]
fn block_boundary_and_toolcall_deltas_are_silent() {
    let (events, mut parser) = new_parser();
    for line in [
        r#"{"type":"message_update","assistantMessageEvent":{"type":"start"}}"#,
        r#"{"type":"message_update","assistantMessageEvent":{"type":"text_start"}}"#,
        r#"{"type":"message_update","assistantMessageEvent":{"type":"text_end"}}"#,
        r#"{"type":"message_update","assistantMessageEvent":{"type":"toolcall_delta","delta":"{\"a\":1}"}}"#,
        r#"{"type":"message_update","assistantMessageEvent":{"type":"done"}}"#,
    ] {
        parser.feed_line(line);
    }
    assert!(
        events.lock().unwrap().is_empty(),
        "boundary/toolcall deltas must not emit; got {:?}",
        events.lock().unwrap()
    );
}

#[test]
fn tool_execution_lifecycle_emits_call_then_result() {
    let (events, mut parser) = new_parser();
    parser
        .feed_line(
            r#"{"type":"tool_execution_start","toolCallId":"t1","toolName":"bash","args":{"command":"ls"}}"#,
        );
    parser
        .feed_line(
            r#"{"type":"tool_execution_end","toolCallId":"t1","toolName":"bash","result":"file.txt","isError":false}"#,
        );
    let collected = events.lock().unwrap().clone();
    assert_eq!(kinds(&collected), vec!["tool_call", "tool_result"]);
    match &collected[0] {
        SemanticEvent::ToolCall { name, id, input, .. } => {
            assert_eq!(name.as_deref(), Some("bash"));
            assert_eq!(id.as_deref(), Some("t1"));
            assert_eq!(
                input.as_ref().and_then(|v| v.get("command")).and_then(Value::as_str),
                Some("ls")
            );
        }
        other => panic!("expected ToolCall, got {other:?}"),
    }
    match &collected[1] {
        SemanticEvent::ToolResult { status, output, .. } => {
            assert_eq!(status.as_deref(), Some("success"));
            assert_eq!(output.as_ref().and_then(Value::as_str), Some("file.txt"));
        }
        other => panic!("expected ToolResult, got {other:?}"),
    }
}

#[test]
fn tool_error_marks_result_error_status() {
    let (events, mut parser) = new_parser();
    parser
        .feed_line(
            r#"{"type":"tool_execution_end","toolCallId":"t9","toolName":"bash","result":"boom","isError":true}"#,
        );
    match &events.lock().unwrap()[0] {
        SemanticEvent::ToolResult { status, .. } => assert_eq!(status.as_deref(), Some("error")),
        other => panic!("expected ToolResult, got {other:?}"),
    }
}

#[test]
fn tool_execution_update_is_silent() {
    let (events, mut parser) = new_parser();
    parser
        .feed_line(
            r#"{"type":"tool_execution_update","toolCallId":"t1","partialResult":"partial output so far"}"#,
        );
    assert!(
        events.lock().unwrap().is_empty(),
        "accumulated progress must not emit a delta"
    );
}

#[test]
fn message_end_accumulates_usage_and_cost() {
    let (events, mut parser) = new_parser();
    parser
        .feed_line(
            r#"{"type":"message_end","message":{"stopReason":"stop","usage":{"input":1200,"output":150,"cacheRead":300,"totalTokens":1650,"cost":{"total":0.00594}}}}"#,
        );
    parser
        .feed_line(
            r#"{"type":"message_end","message":{"stopReason":"stop","usage":{"input":100,"output":50,"totalTokens":150,"cost":{"total":0.001}}}}"#,
        );
    // message_end alone emits nothing user-visible.
    assert!(events.lock().unwrap().is_empty());
    let summary = parser.finish(0);
    let usage = summary.token_usage.expect("usage accumulated");
    assert_eq!(usage.input, Some(1300));
    assert_eq!(usage.output, Some(200));
    assert_eq!(usage.total, Some(1800));
    assert_eq!(usage.cache_read, Some(300));
    assert!((summary.cost_usd.unwrap() - 0.00694).abs() < 1e-9);
}

#[test]
fn message_end_error_stop_reason_emits_error() {
    let (events, mut parser) = new_parser();
    parser
        .feed_line(
            r#"{"type":"message_end","message":{"stopReason":"error","errorMessage":"Provider returned error: 503 service unavailable"}}"#,
        );
    let collected = events.lock().unwrap().clone();
    match &collected[0] {
        SemanticEvent::Error {
            terminal,
            kind,
            message,
            ..
        } => {
            assert!(!terminal, "an assistant-message error is not the terminal record");
            assert_eq!(*kind, SemanticErrorKind::ApiRemote);
            assert!(message.contains("503"));
        }
        other => panic!("expected Error, got {other:?}"),
    }
    let summary = parser.finish(1);
    assert!(summary.is_error);
    assert_eq!(summary.error_kind.as_deref(), Some("api_remote"));
}

#[test]
fn agent_end_emits_terminal_turn_complete() {
    let (events, mut parser) = new_parser();
    parser
        .feed_line(
            r#"{"type":"message_end","message":{"stopReason":"stop","usage":{"input":10,"output":5,"totalTokens":15}}}"#,
        );
    parser
        .feed_line(r#"{"type":"agent_end","willRetry":false}"#);
    let collected = events.lock().unwrap().clone();
    assert_eq!(kinds(&collected), vec!["turn_complete"]);
    match &collected[0] {
        SemanticEvent::TurnComplete {
            provider_status,
            token_usage,
            ..
        } => {
            assert_eq!(provider_status.as_deref(), Some("stop"));
            assert_eq!(token_usage.as_ref().and_then(|u| u.input), Some(10));
        }
        other => panic!("expected TurnComplete, got {other:?}"),
    }
    let summary = parser.finish(0);
    assert_eq!(summary.num_turns, Some(1));
}

#[test]
fn auto_retry_start_emits_info_and_end_failure_emits_error() {
    let (events, mut parser) = new_parser();
    parser
        .feed_line(
            r#"{"type":"auto_retry_start","attempt":3,"maxAttempts":3,"errorMessage":"503 service unavailable"}"#,
        );
    parser
        .feed_line(
            r#"{"type":"auto_retry_end","success":false,"attempt":3,"finalError":"provider overloaded"}"#,
        );
    let collected = events.lock().unwrap().clone();
    assert_eq!(kinds(&collected), vec!["info", "error"]);
    match &collected[0] {
        SemanticEvent::Info { message, .. } => assert!(message.contains("attempt 3/3")),
        other => panic!("expected Info, got {other:?}"),
    }
    assert!(parser.finish(1).is_error);
}

#[test]
fn successful_auto_retry_end_is_silent() {
    let (events, mut parser) = new_parser();
    parser
        .feed_line(r#"{"type":"auto_retry_end","success":true,"attempt":2}"#);
    assert!(events.lock().unwrap().is_empty());
}

#[test]
fn failed_compaction_emits_warning() {
    let (events, mut parser) = new_parser();
    parser
        .feed_line(
            r#"{"type":"compaction_end","result":null,"aborted":false,"errorMessage":"compaction model unavailable"}"#,
        );
    match &events.lock().unwrap()[0] {
        SemanticEvent::Warning { message, .. } => assert!(message.contains("compaction failed")),
        other => panic!("expected Warning, got {other:?}"),
    }
}

#[test]
fn silent_lifecycle_events_emit_nothing() {
    let (events, mut parser) = new_parser();
    for line in [
        r#"{"type":"agent_start"}"#,
        r#"{"type":"turn_start"}"#,
        r#"{"type":"message_start"}"#,
        r#"{"type":"turn_end","message":{}}"#,
        r#"{"type":"compaction_start"}"#,
        r#"{"type":"queue_update"}"#,
        r#"{"type":"entry_appended"}"#,
        r#"{"type":"session_info_changed"}"#,
        r#"{"type":"thinking_level_changed","level":"high"}"#,
    ] {
        parser.feed_line(line);
    }
    assert!(
        events.lock().unwrap().is_empty(),
        "recognized lifecycle events must be silent; got {:?}",
        events.lock().unwrap()
    );
}

#[test]
fn unknown_event_becomes_provider_extension() {
    let (events, mut parser) = new_parser();
    parser
        .feed_line(r#"{"type":"some_future_event","x":1}"#);
    match &events.lock().unwrap()[0] {
        SemanticEvent::ProviderExtension { provider, kind, .. } => {
            assert_eq!(*provider, Provider::Pi);
            assert_eq!(kind, "some_future_event");
        }
        other => panic!("expected ProviderExtension, got {other:?}"),
    }
}

#[test]
fn malformed_json_emits_warning() {
    let (events, mut parser) = new_parser();
    parser.feed_line("not json");
    assert!(matches!(
        events.lock().unwrap()[0],
        SemanticEvent::Warning { .. }
    ));
}

#[test]
fn blank_lines_are_skipped() {
    let (events, mut parser) = new_parser();
    parser.feed_line("");
    parser.feed_line("   ");
    assert!(events.lock().unwrap().is_empty());
}

#[test]
fn round_trip_serialization_fidelity() {
    let (events, mut parser) = new_parser();
    for line in [
        r#"{"type":"session","id":"s","cwd":"/w"}"#,
        r#"{"type":"message_update","assistantMessageEvent":{"type":"thinking_delta","delta":"hmm"}}"#,
        r#"{"type":"message_update","assistantMessageEvent":{"type":"text_delta","delta":"hi"}}"#,
        r#"{"type":"tool_execution_start","toolCallId":"t","toolName":"read","args":{"path":"a"}}"#,
        r#"{"type":"tool_execution_end","toolCallId":"t","toolName":"read","result":"x","isError":false}"#,
        r#"{"type":"message_end","message":{"stopReason":"stop","usage":{"input":1,"output":2,"totalTokens":3,"cost":{"total":0.01}}}}"#,
        r#"{"type":"agent_end","willRetry":false}"#,
        r#"{"type":"future.kind","x":1}"#,
    ] {
        parser.feed_line(line);
    }
    for event in events.lock().unwrap().iter() {
        let v = serde_json::to_value(event).unwrap();
        let decoded: SemanticEvent = serde_json::from_value(v.clone()).unwrap();
        assert_eq!(v, serde_json::to_value(&decoded).unwrap());
    }
    let summary = parser.finish(0);
    assert_eq!(summary.assistant_text, "hi");
    assert_eq!(summary.tool_calls, Some(1));
    assert_eq!(summary.num_turns, Some(1));
}

#[test]
fn classify_error_covers_categories() {
    assert_eq!(classify_error("rate limit exceeded"), SemanticErrorKind::ApiRemote);
    assert_eq!(classify_error("no API key found for anthropic"), SemanticErrorKind::Configuration);
    assert_eq!(classify_error("run aborted by user"), SemanticErrorKind::Interrupted);
    assert_eq!(classify_error("something odd"), SemanticErrorKind::AgentNative);
}
