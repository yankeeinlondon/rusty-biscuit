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
    Box<GeminiSemanticStreamParser<Recording>>,
) {
    let events = Arc::new(Mutex::new(Vec::new()));
    let sink = Recording {
        events: events.clone(),
    };
    (events, Box::new(GeminiSemanticStreamParser::new(sink)))
}

fn kinds(events: &[SemanticEvent]) -> Vec<&'static str> {
    events.iter().map(|e| e.kind_str()).collect()
}

#[test]
fn init_emits_session_start() {
    let (events, mut parser) = new_parser();
    parser
        .feed_line(r#"{"type":"init","session_id":"gem-1","model":"gemini-2.5-pro"}"#);
    let collected = events.lock().unwrap().clone();
    assert!(matches!(
        collected[0],
        SemanticEvent::SessionStart { ref session_id, .. }
            if session_id.as_deref() == Some("gem-1")
    ));
}

#[test]
fn assistant_message_emits_output_text() {
    let (events, mut parser) = new_parser();
    parser
        .feed_line(r#"{"type":"message","role":"assistant","content":"Hello"}"#);
    let collected = events.lock().unwrap().clone();
    assert!(matches!(
        collected[0],
        SemanticEvent::OutputText { ref text, .. } if text == "Hello\n"
    ));
    let summary = parser.finish(0);
    assert_eq!(summary.assistant_text, "Hello");
}

#[test]
fn gemini_non_assistant_message_emits_no_provider_extension() {
    let (events, mut parser) = new_parser();
    parser
        .feed_line(
            r#"{"type":"message","content":"Hi how are you?","role":"user","timestamp":"2026-04-14T00:00:00Z"}"#,
        );

    let captured = events.lock().unwrap().clone();
    assert!(
        !captured.iter().any(|e| matches!(
            e,
            SemanticEvent::ProviderExtension { kind, .. } if kind == "message.non_assistant"
        )),
        "non-assistant messages must be dropped silently, got {captured:?}"
    );
    assert!(
        captured.is_empty(),
        "no semantic events should be emitted for user-role messages, got {captured:?}"
    );
}

#[test]
fn gemini_assistant_message_still_routes_to_output_text() {
    let (events, mut parser) = new_parser();
    parser
        .feed_line(r#"{"type":"message","content":"response text","role":"assistant"}"#);

    let captured = events.lock().unwrap().clone();
    assert!(
        captured
            .iter()
            .any(|e| matches!(e, SemanticEvent::OutputText { .. })),
        "assistant message must still route to OutputText"
    );
}

#[test]
fn tool_use_and_result_emit_typed_events() {
    let (events, mut parser) = new_parser();
    parser
        .feed_line(
            r#"{"type":"tool_use","tool_id":"t1","tool_name":"search","parameters":{"q":"rust"}}"#,
        );
    parser
        .feed_line(
            r#"{"type":"tool_result","tool_id":"t1","status":"success","output":{"hits":3}}"#,
        );
    let collected = events.lock().unwrap().clone();
    assert_eq!(kinds(&collected), vec!["tool_call", "tool_result"]);
    match &collected[1] {
        SemanticEvent::ToolResult {
            name,
            id,
            status,
            output,
            ..
        } => {
            assert_eq!(name.as_deref(), Some("search"));
            assert_eq!(id.as_deref(), Some("t1"));
            assert_eq!(status.as_deref(), Some("success"));
            assert_eq!(*output, Some(json!({"hits": 3})));
        }
        other => panic!("expected ToolResult, got {other:?}"),
    }
}

#[test]
fn error_severity_warning_emits_warning() {
    let (events, mut parser) = new_parser();
    parser
        .feed_line(r#"{"type":"error","severity":"warning","message":"Loop detected"}"#);
    let collected = events.lock().unwrap().clone();
    assert!(matches!(
        collected[0],
        SemanticEvent::Warning { ref message, .. } if message == "Loop detected"
    ));
    let summary = parser.finish(0);
    assert!(!summary.is_error);
}

#[test]
fn error_fatal_severity_emits_error() {
    let (events, mut parser) = new_parser();
    parser
        .feed_line(r#"{"type":"error","severity":"fatal","message":"Catastrophe"}"#);
    let collected = events.lock().unwrap().clone();
    assert!(matches!(
        collected[0],
        SemanticEvent::Error { terminal: true, .. }
    ));
}

#[test]
fn result_status_success_emits_turn_complete() {
    let (events, mut parser) = new_parser();
    parser
        .feed_line(
            r#"{"type":"result","status":"success","stats":{"input_tokens":500,"output_tokens":250,"cached":100,"duration_ms":8000,"tool_calls":2}}"#,
        );
    let collected = events.lock().unwrap().clone();
    match &collected[0] {
        SemanticEvent::TurnComplete {
            provider_status,
            duration_ms,
            token_usage,
            ..
        } => {
            assert_eq!(provider_status.as_deref(), Some("success"));
            assert_eq!(*duration_ms, Some(8000));
            let tu = token_usage.as_ref().unwrap();
            assert_eq!(tu.input, Some(500));
            assert_eq!(tu.output, Some(250));
            assert_eq!(tu.total, Some(750));
            assert_eq!(tu.cache_read, Some(100));
        }
        other => panic!("expected TurnComplete, got {other:?}"),
    }
    let summary = parser.finish(0);
    assert_eq!(summary.tool_calls, Some(2));
}

#[test]
fn result_status_error_emits_error() {
    let (events, mut parser) = new_parser();
    parser
        .feed_line(
            r#"{"type":"result","status":"error","error":{"type":"FatalTurnLimited","message":"max turns"}}"#,
        );
    let collected = events.lock().unwrap().clone();
    assert!(matches!(
        collected[0],
        SemanticEvent::Error { terminal: true, .. }
    ));
    let summary = parser.finish(1);
    assert!(summary.is_error);
    assert_eq!(summary.error_kind.as_deref(), Some("FatalTurnLimited"));
}

#[test]
fn unknown_event_type_becomes_provider_extension() {
    let (events, mut parser) = new_parser();
    parser
        .feed_line(r#"{"type":"some_unknown","data":"x"}"#);
    let collected = events.lock().unwrap().clone();
    assert!(matches!(
        collected[0],
        SemanticEvent::ProviderExtension { ref kind, .. } if kind == "some_unknown"
    ));
}

#[test]
fn malformed_json_emits_warning() {
    let (events, mut parser) = new_parser();
    parser.feed_line("garbage");
    assert!(matches!(
        events.lock().unwrap()[0],
        SemanticEvent::Warning { .. }
    ));
}

#[test]
fn tool_input_string_fallback_parses_without_panic() {
    let (events, mut parser) = new_parser();
    parser
        .feed_line(r#"{"type":"tool_use","tool_id":"t","tool_name":"bash","input":"ls -la"}"#);
    let collected = events.lock().unwrap().clone();
    assert_eq!(kinds(&collected), vec!["tool_call"]);
    match &collected[0] {
        SemanticEvent::ToolCall { input, .. } => {
            assert_eq!(input.as_ref().and_then(Value::as_str), Some("ls -la"));
        }
        other => panic!("expected ToolCall, got {other:?}"),
    }
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
            assert_eq!(*provider, Provider::Gemini);
            assert_eq!(kind, "");
            assert_eq!(payload.get("payload"), Some(&json!({"k": 1})));
        }
        other => panic!("expected ProviderExtension, got {other:?}"),
    }
}

#[test]
fn streamed_markdown_list_emits_contiguous_items() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/providers/gemini-markdown-list.ndjson");
    let raw = std::fs::read_to_string(&path).expect("fixture exists");
    let (events, mut parser) = new_parser();
    for line in raw.lines() {
        parser.feed_line(line);
    }
    let text: String = events
        .lock()
        .unwrap()
        .iter()
        .filter_map(|e| match e {
            SemanticEvent::OutputText { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect();
    // No bullet item should appear split mid-content (i.e. emitted twice
    // across two OutputText events — joined, no awkward internal split).
    let bullet_lines: Vec<&str> = text
        .lines()
        .filter(|l| l.trim_start().starts_with("- ") || l.trim_start().starts_with("* "))
        .collect();
    assert!(
        !bullet_lines.is_empty(),
        "fixture must include bullet items"
    );
    for line in &bullet_lines {
        assert!(
            line.len() > 5,
            "bullet item appears truncated or split: {line:?}\nfull text:\n{text}"
        );
    }
    // No three-or-more consecutive newlines (would indicate stray blank
    // lines from per-chunk emission).
    assert!(
        !text.contains("\n\n\n"),
        "unexpected triple-newline in:\n{text}"
    );
}

#[test]
fn delta_false_message_bypasses_buffer() {
    // Non-delta messages must emit immediately, not be held back.
    let (events, mut parser) = new_parser();
    parser
        .feed_line(r#"{"type":"init","session_id":"g1","model":"gemini-2.5"}"#);
    parser
        .feed_line(r#"{"type":"message","role":"assistant","content":"one-shot answer"}"#);
    let kinds: Vec<&'static str> = events
        .lock()
        .unwrap()
        .iter()
        .map(|e| e.kind_str())
        .collect();
    assert!(
        kinds.contains(&"output_text"),
        "non-delta message must emit output_text immediately; got {kinds:?}"
    );
}

#[test]
fn pending_delta_flushed_on_non_text_event() {
    // Buffered text from a delta must be flushed when a non-text event
    // (e.g. turn completion) arrives, even without an explicit blank line.
    let (events, mut parser) = new_parser();
    parser
        .feed_line(r#"{"type":"init","session_id":"g1","model":"gemini-2.5"}"#);
    // Partial delta: no trailing \n\n
    parser
        .feed_line(r#"{"type":"message","role":"assistant","delta":true,"content":"partial "}"#);
    parser
        .feed_line(r#"{"type":"message","role":"assistant","delta":true,"content":"more"}"#);
    // Turn completes — buffer must flush.
    parser
        .feed_line(r#"{"type":"result","usage":{"input_tokens":10,"output_tokens":20}}"#);
    let text: String = events
        .lock()
        .unwrap()
        .iter()
        .filter_map(|e| match e {
            SemanticEvent::OutputText { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect();
    assert!(
        text.contains("partial more"),
        "buffered delta content must be flushed on terminal event; got {text:?}"
    );
}

#[test]
fn round_trip_fidelity_mixed_fixture() {
    let (events, mut parser) = new_parser();
    for line in [
        r#"{"type":"init","session_id":"g","model":"m"}"#,
        r#"{"type":"message","role":"assistant","content":"hi"}"#,
        r#"{"type":"tool_use","tool_id":"t","tool_name":"s","parameters":{}}"#,
        r#"{"type":"tool_result","tool_id":"t","status":"success","output":"ok"}"#,
        r#"{"type":"error","severity":"warning","message":"loop"}"#,
        r#"{"type":"future.unknown","x":1}"#,
        r#"{"type":"result","status":"success","stats":{"duration_ms":1}}"#,
    ] {
        parser.feed_line(line);
    }
    for event in events.lock().unwrap().iter() {
        let v = serde_json::to_value(event).unwrap();
        let decoded: SemanticEvent = serde_json::from_value(v.clone()).unwrap();
        assert_eq!(v, serde_json::to_value(&decoded).unwrap());
    }
}
