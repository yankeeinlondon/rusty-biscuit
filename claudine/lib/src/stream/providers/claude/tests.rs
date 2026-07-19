use std::sync::{Arc, Mutex};

use super::*;
use serde_json::json;

/// Recording sink that collects every emitted semantic event.
struct RecordingSink {
    events: Arc<Mutex<Vec<SemanticEvent>>>,
}

impl RecordingSink {
    fn new() -> Self {
        Self {
            events: Arc::new(Mutex::new(Vec::new())),
        }
    }
    fn snapshot(&self) -> Vec<SemanticEvent> {
        self.events.lock().unwrap().clone()
    }
    fn kinds(&self) -> Vec<&'static str> {
        self.events
            .lock()
            .unwrap()
            .iter()
            .map(|e| e.kind_str())
            .collect()
    }
}

impl SemanticEventSink for RecordingSink {
    fn on_semantic_event(&mut self, event: SemanticEvent) {
        self.events.lock().unwrap().push(event);
    }
}

fn new_parser() -> (
    RecordingSink,
    Box<ClaudeSemanticStreamParser<RecordingSink>>,
) {
    let sink = RecordingSink::new();
    let sink_shared = RecordingSink {
        events: sink.events.clone(),
    };
    let parser = Box::new(ClaudeSemanticStreamParser::new(sink_shared));
    (sink, parser)
}

#[test]
fn init_emits_session_start() {
    let (sink, mut parser) = new_parser();
    parser
        .feed_line(
            r#"{"type":"init","session_id":"s1","model":"claude","apiKeySource":"none"}"#,
        );
    let events = sink.snapshot();
    assert_eq!(events.len(), 1);
    match &events[0] {
        SemanticEvent::SessionStart {
            session_id,
            model,
            extra,
        } => {
            assert_eq!(session_id.as_deref(), Some("s1"));
            assert_eq!(model.as_deref(), Some("claude"));
            assert_eq!(
                extra.get("api_key_source").and_then(Value::as_str),
                Some("none")
            );
        }
        other => panic!("expected SessionStart, got {other:?}"),
    }
}

#[test]
fn assistant_text_emits_output_text_and_accumulates() {
    let (sink, mut parser) = new_parser();
    parser
        .feed_line(r#"{"type":"assistant","content":[{"type":"text","text":"Hello"}]}"#);
    parser
        .feed_line(r#"{"type":"assistant","content":[{"type":"text","text":", world"}]}"#);
    let kinds = sink.kinds();
    assert_eq!(kinds, vec!["output_text", "output_text"]);
    let summary = parser.finish(0);
    assert_eq!(summary.assistant_text, "Hello, world");
}

#[test]
fn thinking_delta_emits_reasoning() {
    let (sink, mut parser) = new_parser();
    parser
        .feed_line(
            r#"{"type":"content_block_delta","delta":{"type":"thinking_delta","thinking":"pondering"}}"#,
        );
    let events = sink.snapshot();
    assert!(matches!(
        events[0],
        SemanticEvent::Reasoning { ref text, .. } if text == "pondering"
    ));
    // Thinking must NOT contribute to assistant_text.
    let summary = parser.finish(0);
    assert_eq!(summary.assistant_text, "");
}

#[test]
fn text_delta_emits_output_text() {
    let (sink, mut parser) = new_parser();
    parser
        .feed_line(
            r#"{"type":"content_block_delta","delta":{"type":"text_delta","text":"Hi"}}"#,
        );
    let events = sink.snapshot();
    assert!(matches!(
        events[0],
        SemanticEvent::OutputText { ref text, .. } if text == "Hi"
    ));
}

#[test]
fn tool_use_and_result_emit_typed_events() {
    let (sink, mut parser) = new_parser();
    parser
        .feed_line(r#"{"type":"tool_use","id":"t1","name":"bash","input":{"cmd":"ls"}}"#);
    parser
        .feed_line(r#"{"type":"tool_result","tool_use_id":"t1","content":"ok"}"#);

    let events = sink.snapshot();
    assert_eq!(events.len(), 2);
    match &events[0] {
        SemanticEvent::ToolCall {
            name, id, input, ..
        } => {
            assert_eq!(name.as_deref(), Some("bash"));
            assert_eq!(id.as_deref(), Some("t1"));
            assert_eq!(input, &Some(json!({"cmd": "ls"})));
        }
        other => panic!("expected ToolCall, got {other:?}"),
    }
    match &events[1] {
        SemanticEvent::ToolResult {
            name, id, output, ..
        } => {
            assert_eq!(name.as_deref(), Some("bash"));
            assert_eq!(id.as_deref(), Some("t1"));
            assert_eq!(output, &Some(json!("ok")));
        }
        other => panic!("expected ToolResult, got {other:?}"),
    }

    let summary = parser.finish(0);
    assert_eq!(summary.tool_calls, Some(1));
}

#[test]
fn content_block_start_tool_use_dispatches_as_tool_call() {
    let (sink, mut parser) = new_parser();
    parser
        .feed_line(
            r#"{"type":"content_block_start","content_block":{"type":"tool_use","id":"t2","name":"bash","input":{"cmd":"ls -la"}}}"#,
        );
    let events = sink.snapshot();
    assert!(matches!(events[0], SemanticEvent::ToolCall { .. }));
}

#[test]
fn content_block_start_and_delta_emit_tool_call_with_merged_input() {
    let (sink, mut parser) = new_parser();
    parser
        .feed_line(
            r#"{"type":"content_block_start","content_block":{"type":"tool_use","id":"t3","name":"bash"}}"#,
        );
    parser
        .feed_line(
            r#"{"type":"content_block_delta","delta":{"type":"input_json_delta","partial_json":"{\"command\":\"ls -la\"}"}}"#,
        );
    let events = sink.snapshot();
    assert_eq!(events.len(), 1);
    match &events[0] {
        SemanticEvent::ToolCall {
            name, id, input, ..
        } => {
            assert_eq!(name.as_deref(), Some("bash"));
            assert_eq!(id.as_deref(), Some("t3"));
            assert_eq!(input, &Some(json!({"command": "ls -la"})));
        }
        other => panic!("expected ToolCall, got {other:?}"),
    }
}

#[test]
fn assistant_envelope_tool_use_preface_becomes_reasoning_then_tool_call() {
    let (sink, mut parser) = new_parser();
    parser
        .feed_line(
            r#"{"type":"assistant","message":{"content":[{"type":"text","text":"Checking tests."},{"type":"tool_use","id":"tu_1","name":"Bash","input":{"command":"cargo test -p claudine"}}]}}"#,
        );
    let events = sink.snapshot();
    assert_eq!(events.len(), 2);
    match &events[0] {
        SemanticEvent::Reasoning { text, extra } => {
            assert_eq!(text, "Checking tests.");
            assert_eq!(
                extra.get("reasoning_source").and_then(Value::as_str),
                Some("assistant_tool_preface")
            );
        }
        other => panic!("expected Reasoning, got {other:?}"),
    }
    match &events[1] {
        SemanticEvent::ToolCall {
            name, id, input, ..
        } => {
            assert_eq!(name.as_deref(), Some("Bash"));
            assert_eq!(id.as_deref(), Some("tu_1"));
            assert_eq!(input, &Some(json!({"command": "cargo test -p claudine"})));
        }
        other => panic!("expected ToolCall, got {other:?}"),
    }
    let summary = parser.finish(0);
    assert_eq!(
        summary.assistant_text, "",
        "pre-tool Claude narration must not leak into final stdout"
    );
}

#[test]
fn rate_limit_emits_warning_with_original_message() {
    let (sink, mut parser) = new_parser();
    parser
        .feed_line(
            r#"{"type":"rate_limit_event","is_throttled":true,"retry_after_ms":5000,"message":"Rate limited"}"#,
        );
    let events = sink.snapshot();
    match &events[0] {
        SemanticEvent::Warning { message, extra } => {
            assert_eq!(message, "Rate limited");
            assert_eq!(extra.get("retry_after_ms"), Some(&json!(5000)));
        }
        other => panic!("expected Warning, got {other:?}"),
    }
    let summary = parser.finish(0);
    assert!(summary.rate_limit.is_some());
}

#[test]
fn approaching_rate_limit_without_message_renders_reset_window_warning() {
    let (sink, mut parser) = new_parser();
    parser
        .feed_line(
            r#"{"type":"rate_limit_event","rate_limit_info":{"status":"approaching_limit","resetsAt":1712000000,"rateLimitType":"usage","overageStatus":"allowed"}}"#,
        );
    let events = sink.snapshot();
    match &events[0] {
        SemanticEvent::Warning { message, extra } => {
            assert!(message.contains("rate limit warning"));
            assert!(message.contains("approaching the cap"));
            assert!(message.contains("Window resets on"));
            assert_eq!(
                extra.get("rate_limit_status").and_then(Value::as_str),
                Some("approaching_limit")
            );
            assert_eq!(
                extra.get("reset_at").and_then(Value::as_str),
                Some("2024-04-01T19:33:20+00:00")
            );
        }
        other => panic!("expected Warning, got {other:?}"),
    }
    let summary = parser.finish(0);
    let rate_limit = summary.rate_limit.expect("rate limit summary");
    assert_eq!(rate_limit.is_throttled, Some(false));
    let msg = rate_limit.message.as_deref().expect("rate limit message");
    assert!(
        msg.starts_with(
            "Claude rate limit warning: your current rate limit window is approaching the cap"
        ),
        "unexpected message: {msg}"
    );
    assert!(
        msg.contains("Window resets on"),
        "unexpected message: {msg}"
    );
    assert_eq!(
        rate_limit.reset_at.map(|dt| dt.timestamp()),
        Some(1712000000)
    );
}

#[test]
fn allowed_warning_status_renders_soft_notice_with_correct_window() {
    let (sink, mut parser) = new_parser();
    parser
        .feed_line(
            r#"{"type":"rate_limit_event","rate_limit_info":{"status":"allowed_warning","resetsAt":1712000000,"rateLimitType":"seven_day","overageStatus":"allowed"}}"#,
        );
    let events = sink.snapshot();
    match &events[0] {
        SemanticEvent::Warning { message, extra } => {
            assert!(
                message.starts_with("Claude rate limit notice"),
                "expected soft notice, got: {message}"
            );
            assert!(
                message.contains("weekly usage window"),
                "expected weekly window label, got: {message}"
            );
            assert!(
                !message.contains("capped soon") && !message.contains("almost fully"),
                "allowed_warning must not use cap-imminent language: {message}"
            );
            assert_eq!(
                extra.get("rate_limit_status").and_then(Value::as_str),
                Some("allowed_warning")
            );
            assert_eq!(
                extra.get("rate_limit_type").and_then(Value::as_str),
                Some("seven_day")
            );
        }
        other => panic!("expected Warning, got {other:?}"),
    }
}

#[test]
fn non_throttled_rate_limit_without_message_or_status_emits_no_warning() {
    let (sink, mut parser) = new_parser();
    parser
        .feed_line(r#"{"type":"rate_limit_event","is_throttled":false}"#);
    assert!(sink.snapshot().is_empty());
    let summary = parser.finish(0);
    assert_eq!(summary.rate_limit.and_then(|rl| rl.message), None);
}

#[test]
fn error_event_emits_terminal_error() {
    let (sink, mut parser) = new_parser();
    parser
        .feed_line(
            r#"{"type":"error","error":{"type":"billing_error","message":"Insufficient credits"}}"#,
        );
    let events = sink.snapshot();
    match &events[0] {
        SemanticEvent::Error {
            message,
            terminal,
            kind,
            extra,
        } => {
            assert_eq!(message, "Insufficient credits");
            assert!(*terminal);
            assert_eq!(*kind, SemanticErrorKind::ApiRemote);
            assert_eq!(extra.get("error_kind"), Some(&json!("billing_error")));
        }
        other => panic!("expected Error, got {other:?}"),
    }
    let summary = parser.finish(1);
    assert!(summary.is_error);
    assert_eq!(summary.error_kind.as_deref(), Some("billing_error"));
}

#[test]
fn classify_error_billing_kind_maps_to_api_remote() {
    assert_eq!(
        classify_error(Some("billing_error"), Some("Insufficient credits")),
        SemanticErrorKind::ApiRemote,
    );
}

#[test]
fn classify_error_authentication_kind_maps_to_configuration() {
    assert_eq!(
        classify_error(Some("authentication_error"), None),
        SemanticErrorKind::Configuration,
    );
}

#[test]
fn classify_error_unknown_kind_with_no_message_falls_back_to_agent_native() {
    assert_eq!(
        classify_error(Some("weird_kind"), None),
        SemanticErrorKind::AgentNative,
    );
}

#[test]
fn result_emits_turn_complete_and_populates_summary() {
    let (sink, mut parser) = new_parser();
    parser
        .feed_line(
            r#"{"type":"result","duration_ms":12345,"num_turns":1,"stop_reason":"end_turn","cost_usd":0.0042,"usage":{"input_tokens":1000,"output_tokens":500}}"#,
        );
    let events = sink.snapshot();
    match &events[0] {
        SemanticEvent::TurnComplete {
            provider_status,
            cost_usd,
            duration_ms,
            token_usage,
            ..
        } => {
            assert_eq!(provider_status.as_deref(), Some("end_turn"));
            assert_eq!(*cost_usd, Some(0.0042));
            assert_eq!(*duration_ms, Some(12345));
            let tu = token_usage.as_ref().unwrap();
            assert_eq!(tu.input, Some(1000));
            assert_eq!(tu.output, Some(500));
            assert_eq!(tu.total, Some(1500));
        }
        other => panic!("expected TurnComplete, got {other:?}"),
    }
    let summary = parser.finish(0);
    assert_eq!(summary.duration_ms, Some(12345));
    assert_eq!(summary.cost_usd, Some(0.0042));
}

#[test]
fn malformed_json_emits_warning() {
    let (sink, mut parser) = new_parser();
    parser.feed_line("not json {{{");
    let events = sink.snapshot();
    assert!(matches!(events[0], SemanticEvent::Warning { .. }));
}

#[test]
fn unknown_event_becomes_provider_extension() {
    let (sink, mut parser) = new_parser();
    parser
        .feed_line(r#"{"type":"some_future_event","foo":"bar"}"#);
    let events = sink.snapshot();
    match &events[0] {
        SemanticEvent::ProviderExtension {
            provider,
            kind,
            payload,
        } => {
            assert_eq!(*provider, Provider::Claude);
            assert_eq!(kind, "some_future_event");
            assert_eq!(payload.get("foo"), Some(&json!("bar")));
        }
        other => panic!("expected ProviderExtension, got {other:?}"),
    }
}

#[test]
fn task_started_becomes_subagent_start() {
    let (sink, mut parser) = new_parser();
    parser
        .feed_line(r#"{"type":"task_started","task_id":"sa_1","name":"researcher"}"#);
    let events = sink.snapshot();
    match &events[0] {
        SemanticEvent::SubagentStart { name, id, .. } => {
            assert_eq!(name.as_deref(), Some("researcher"));
            assert_eq!(id.as_deref(), Some("sa_1"));
        }
        other => panic!("expected SubagentStart, got {other:?}"),
    }
}

#[test]
fn task_completed_becomes_subagent_stop() {
    let (sink, mut parser) = new_parser();
    parser
        .feed_line(
            r#"{"type":"task_completed","task_id":"sa_1","name":"researcher","status":"success"}"#,
        );
    let events = sink.snapshot();
    match &events[0] {
        SemanticEvent::SubagentStop {
            name, id, status, ..
        } => {
            assert_eq!(name.as_deref(), Some("researcher"));
            assert_eq!(id.as_deref(), Some("sa_1"));
            assert_eq!(status.as_deref(), Some("success"));
        }
        other => panic!("expected SubagentStop, got {other:?}"),
    }
}

#[test]
fn task_progress_becomes_info() {
    let (sink, mut parser) = new_parser();
    parser
        .feed_line(r#"{"type":"task_progress","message":"working on it"}"#);
    let events = sink.snapshot();
    match &events[0] {
        SemanticEvent::Info { message, .. } => {
            assert_eq!(message, "working on it");
        }
        other => panic!("expected Info, got {other:?}"),
    }
}

#[test]
fn empty_and_whitespace_lines_emit_nothing() {
    let (sink, mut parser) = new_parser();
    parser.feed_line("");
    parser.feed_line("  ");
    parser.feed_line("\t");
    assert!(sink.snapshot().is_empty());
}

#[test]
fn multi_turn_concatenation() {
    let (sink, mut parser) = new_parser();
    parser
        .feed_line(r#"{"type":"assistant","content":[{"type":"text","text":"First. "}]}"#);
    parser
        .feed_line(r#"{"type":"assistant","content":[{"type":"text","text":"Second."}]}"#);
    assert_eq!(sink.kinds(), vec!["output_text", "output_text"]);
    let summary = parser.finish(0);
    assert_eq!(summary.assistant_text, "First. Second.");
}

#[test]
fn large_init_arrays_not_stored_in_raw_summary() {
    let (_, mut parser) = new_parser();
    parser
        .feed_line(r#"{"type":"init","session_id":"s","model":"m","tools":[{"name":"a"}]}"#);
    parser
        .feed_line(r#"{"type":"result","duration_ms":1,"tools":["a"],"skills":["s"],"agents":["x"],"mcp_servers":["m"]}"#);
    let summary = parser.finish(0);
    let raw = summary.raw_summary.unwrap();
    assert!(raw.get("tools").is_none());
    assert!(raw.get("skills").is_none());
    assert!(raw.get("agents").is_none());
    assert!(raw.get("mcp_servers").is_none());
    assert!(raw.get("duration_ms").is_some());
}

#[test]
fn badges_derived_on_billing_error() {
    let (_, mut parser) = new_parser();
    parser
        .feed_line(r#"{"type":"init","session_id":"s","model":"m"}"#);
    parser
        .feed_line(
            r#"{"type":"error","error":{"type":"billing_error","message":"Insufficient credits"}}"#,
        );
    let summary = parser.finish(1);
    assert_eq!(summary.badges.len(), 1);
    assert_eq!(
        summary.badges[0].category,
        crate::stream::badges::BadgeCategory::Billing
    );
}

#[test]
fn user_event_routes_tool_result_to_semantic_tool_result() {
    let (sink, mut parser) = new_parser();
    parser
        .feed_line(
            r#"{"type":"assistant","message":{"content":[{"type":"tool_use","id":"tu_1","name":"Bash","input":{"command":"echo hello"}}]}}"#,
        );
    parser
        .feed_line(r#"{"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"tu_1","content":"hello","is_error":false}]},"session_id":"s1"}"#);
    let events = sink.snapshot();
    assert!(matches!(events[1], SemanticEvent::ToolResult { .. }));
    let SemanticEvent::ToolResult { name, status, .. } = &events[1] else {
        panic!("expected ToolResult, got {:?}", events[1]);
    };
    assert_eq!(name.as_deref(), Some("Bash"));
    assert_eq!(status.as_deref(), Some("success"));
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, SemanticEvent::ProviderExtension { .. })),
        "user event must not leak as ProviderExtension"
    );
}

#[test]
fn billing_error_on_assistant_surfaces_terminal_error_not_rate_limit() {
    let (sink, mut parser) = new_parser();
    parser
        .feed_line(r#"{"type":"assistant","message":{"model":"<synthetic>","content":[{"type":"text","text":"Credit balance is too low"}]},"session_id":"s1","error":"billing_error"}"#);
    let events = sink.snapshot();
    let terminal_errors: Vec<_> = events
        .iter()
        .filter(|e| matches!(e, SemanticEvent::Error { terminal: true, .. }))
        .collect();
    assert_eq!(
        terminal_errors.len(),
        1,
        "expected exactly one terminal Error; got {terminal_errors:?}"
    );
    if let SemanticEvent::Error { message, extra, .. } = terminal_errors[0] {
        let lower = message.to_lowercase();
        assert!(
            lower.contains("billing") || lower.contains("credit"),
            "billing error message must mention billing/credit: {message:?}"
        );
        assert_eq!(
            extra.get("error_kind").and_then(|v| v.as_str()),
            Some("billing_error")
        );
    }
    let summary = parser.finish(1);
    assert_eq!(summary.error_kind.as_deref(), Some("billing_error"));
    let billing = summary
        .badges
        .iter()
        .find(|b| b.category == crate::stream::badges::BadgeCategory::Billing);
    assert!(
        billing.is_some(),
        "summary must carry a Billing badge, not a RateLimit one; got {:?}",
        summary.badges
    );
    assert!(
        !summary
            .badges
            .iter()
            .any(|b| b.category == crate::stream::badges::BadgeCategory::RateLimit),
        "billing_error must NOT produce a RateLimit badge; got {:?}",
        summary.badges
    );
}

#[test]
fn hook_events_without_init_do_not_fabricate_session_start() {
    // Without an `init` event, hook_* system subtypes must NOT be
    // promoted to a `SessionStart`. They stay buffered until a real
    // init arrives (or flush inline once the buffer saturates). In
    // either case, nothing should synthesize a session_start.
    let (sink, mut parser) = new_parser();
    parser
        .feed_line(r#"{"type":"system","subtype":"hook_started","hook_name":"SessionStart:startup","session_id":"s1"}"#);
    parser
        .feed_line(r#"{"type":"system","subtype":"hook_response","hook_id":"x","output":"ok","exit_code":0,"session_id":"s1"}"#);
    let kinds = sink.kinds();
    assert!(
        !kinds.contains(&"session_start"),
        "hook_* subtypes must not emit SessionStart; got {kinds:?}"
    );
}

#[test]
fn hook_events_emitted_after_session_start() {
    let (sink, mut parser) = new_parser();
    parser
        .feed_line(r#"{"type":"system","subtype":"hook_started","hook_name":"SessionStart:startup","session_id":"s1"}"#);
    parser
        .feed_line(r#"{"type":"system","subtype":"hook_response","hook_id":"x","output":"ok","exit_code":0,"session_id":"s1"}"#);
    parser
        .feed_line(r#"{"type":"init","session_id":"s1","model":"claude-opus-4-6"}"#);
    let kinds: Vec<&'static str> = sink.kinds();
    let session_idx = kinds
        .iter()
        .position(|k| *k == "session_start")
        .expect("session_start emitted");
    let provider_ext_indices: Vec<usize> = kinds
        .iter()
        .enumerate()
        .filter(|(_, k)| **k == "provider_extension")
        .map(|(i, _)| i)
        .collect();
    for idx in provider_ext_indices {
        assert!(
            idx > session_idx,
            "provider_extension hook event at {idx} must follow session_start at {session_idx}; got {kinds:?}"
        );
    }
}

#[test]
fn hook_events_after_session_start_emit_inline() {
    // Hooks that arrive AFTER SessionStart must NOT be buffered — they
    // pass through inline to preserve live streaming semantics.
    let (sink, mut parser) = new_parser();
    parser
        .feed_line(r#"{"type":"init","session_id":"s1","model":"claude-opus-4-6"}"#);
    parser
        .feed_line(r#"{"type":"system","subtype":"hook_started","hook_name":"PreToolUse","session_id":"s1"}"#);
    let kinds: Vec<&'static str> = sink.kinds();
    // Order: session_start, then immediately provider_extension.
    assert_eq!(kinds.first(), Some(&"session_start"));
    assert_eq!(kinds.get(1), Some(&"provider_extension"));
}

#[test]
fn pre_init_hook_buffer_flushes_when_oversized() {
    // If enough hooks arrive before init that the buffer saturates,
    // flush early so streaming wins over cosmetic ordering.
    let (sink, mut parser) = new_parser();
    for _ in 0..40 {
        parser
            .feed_line(r#"{"type":"system","subtype":"hook_started","hook_name":"X","session_id":"s1"}"#);
    }
    let kinds: Vec<&'static str> = sink.kinds();
    let provider_ext_count = kinds.iter().filter(|k| **k == "provider_extension").count();
    assert!(
        provider_ext_count > 0,
        "hooks past the buffer cap must flush inline; got {kinds:?}"
    );
}

#[test]
fn claude_fixture_full_replay_produces_no_provider_extensions() {
    let fixture = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/providers/claude.ndjson"
    ))
    .expect("claude.ndjson must exist");

    let (sink, mut parser) = new_parser();
    for line in fixture.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        parser.feed_line(line);
    }
    let events = sink.snapshot();
    // Hook events (`system/hook_started`, `system/hook_response`, etc.)
    // are intentionally surfaced as `ProviderExtension` so the sink can
    // render them after the session-ID marker. They are excluded from
    // the "no unrouted provider extensions" guarantee.
    let ext: Vec<&SemanticEvent> = events
        .iter()
        .filter(|e| match e {
            SemanticEvent::ProviderExtension { kind, .. } => !kind.starts_with("system/"),
            _ => false,
        })
        .collect();
    assert!(
        ext.is_empty(),
        "captured Claude fixture must produce zero non-hook ProviderExtension events; found {}: {:#?}",
        ext.len(),
        ext.iter().take(3).collect::<Vec<_>>()
    );
}

#[test]
fn unterminated_input_json_delta_is_bounded() {
    let (sink, mut parser) = new_parser();
    // Open a pending tool_use but never close the JSON.
    parser
        .feed_line(
            r#"{"type":"content_block_start","content_block":{"type":"tool_use","id":"t1","name":"bash"}}"#,
        );

    // Craft a chunk larger than the cap to trigger the overflow path
    // in a single delta.
    let junk = "x".repeat(MAX_PENDING_TOOL_USE_INPUT_BYTES + 10);
    let line = serde_json::json!({
        "type": "content_block_delta",
        "delta": {"type": "input_json_delta", "partial_json": junk},
    })
    .to_string();
    parser.feed_line(&line);

    let events = sink.snapshot();
    let overflow: Vec<&SemanticEvent> = events
        .iter()
        .filter(|e| {
            matches!(
                e,
                SemanticEvent::Error { extra, .. }
                    if extra.get("raw_kind").and_then(Value::as_str)
                        == Some("input_json_overflow")
            )
        })
        .collect();
    assert_eq!(
        overflow.len(),
        1,
        "expected exactly one overflow Error event; got {events:#?}"
    );
}

#[test]
fn missing_discriminator_falls_through_to_provider_extension() {
    let (sink, mut parser) = new_parser();
    parser.feed_line(r#"{"payload":{"k":1}}"#);
    let events = sink.snapshot();
    assert_eq!(events.len(), 1);
    match &events[0] {
        SemanticEvent::ProviderExtension {
            provider,
            kind,
            payload,
        } => {
            assert_eq!(*provider, Provider::Claude);
            assert_eq!(kind, "");
            assert_eq!(payload.get("payload"), Some(&json!({"k": 1})));
        }
        other => panic!("expected ProviderExtension, got {other:?}"),
    }
}

#[test]
fn round_trip_fidelity_across_mixed_events() {
    // Replay a mixed fixture and confirm every emitted event survives a
    // serde round-trip with identical JSON.
    let (sink, mut parser) = new_parser();
    for line in [
        r#"{"type":"init","session_id":"s","model":"m"}"#,
        r#"{"type":"content_block_delta","delta":{"type":"text_delta","text":"Hi"}}"#,
        r#"{"type":"tool_use","id":"t1","name":"bash","input":{"cmd":"ls"}}"#,
        r#"{"type":"rate_limit_event","is_throttled":true,"retry_after_ms":5000,"message":"Slow"}"#,
        r#"{"type":"task_progress","message":"working"}"#,
        r#"{"type":"some_future_event","x":1}"#,
        r#"{"type":"result","duration_ms":1}"#,
    ] {
        parser.feed_line(line);
    }
    let events = sink.snapshot();
    assert!(!events.is_empty());
    for event in events {
        let v = serde_json::to_value(&event).unwrap();
        let decoded: SemanticEvent = serde_json::from_value(v.clone()).unwrap();
        let v2 = serde_json::to_value(&decoded).unwrap();
        assert_eq!(v, v2, "round-trip lost fidelity for {}", event.kind_str());
    }
}
