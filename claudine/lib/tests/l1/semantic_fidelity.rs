//! Phase 4 integration tests: round-trip fidelity, summary regression,
//! and reporting fidelity for the semantic event pipeline.
//!
//! These tests exercise every provider's parser end-to-end through the
//! `create_semantic_parser` factory, confirming:
//!
//! - Every emitted `SemanticEvent` survives a serde round-trip with identical
//!   JSON.
//! - `ProviderExtension` payloads preserve arbitrary nested structures.
//! - `StreamExecutionSummary` fields are populated correctly from fixture
//!   replay.
//! - `semantic_event_to_event_meta` preserves `ProviderExtension.payload` in
//!   `extra["semantic_event"]`.

use std::sync::{Arc, Mutex};

use claudine::events::EnvironmentContext;

use claudine::provider::Provider;
use claudine::stream::parser::SemanticStreamParser;
use claudine::stream::reporting::{semantic_event_to_event_meta, summary_to_event_meta};
use claudine::stream::semantic::{SemanticErrorKind, SemanticEvent, SemanticEventSink};
use claudine::stream::{ParserConfig, StreamProtocol, create_semantic_parser};
use serde_json::{Value, json};

struct Recording {
    events: Arc<Mutex<Vec<SemanticEvent>>>,
}

impl SemanticEventSink for Recording {
    fn on_semantic_event(&mut self, event: SemanticEvent) {
        self.events.lock().unwrap().push(event);
    }
}

fn replay(
    provider: Provider,
    lines: &[&str],
    model: Option<String>,
) -> (Vec<SemanticEvent>, Box<dyn SemanticStreamParser>) {
    let events = Arc::new(Mutex::new(Vec::new()));
    let sink = Recording {
        events: events.clone(),
    };
    let config = ParserConfig { model };
    let mut parser = create_semantic_parser(provider, sink, config);
    for line in lines {
        parser.feed_line(line);
    }
    let collected = events.lock().unwrap().clone();
    (collected, parser)
}

fn assert_round_trip(events: &[SemanticEvent]) {
    for event in events {
        let v = serde_json::to_value(event).unwrap();
        let decoded: SemanticEvent = serde_json::from_value(v.clone()).unwrap();
        let v2 = serde_json::to_value(&decoded).unwrap();
        assert_eq!(v, v2, "round-trip lost fidelity for {}", event.kind_str());
    }
}

fn env() -> EnvironmentContext {
    EnvironmentContext::default()
}

fn kinds_of(events: &[SemanticEvent]) -> Vec<&'static str> {
    events.iter().map(|e| e.kind_str()).collect()
}

fn contains_kind(events: &[SemanticEvent], kind: &str) -> bool {
    events.iter().any(|e| e.kind_str() == kind)
}

mod claude_round_trip {
    use super::*;

    fn fixture() -> Vec<&'static str> {
        vec![
            r#"{"type":"init","session_id":"s1","model":"claude-sonnet-4"}"#,
            r#"{"type":"assistant","content":[{"type":"text","text":"Hello"}]}"#,
            r#"{"type":"content_block_delta","delta":{"type":"thinking_delta","thinking":"Let me think"}}"#,
            r#"{"type":"content_block_delta","delta":{"type":"text_delta","text":" world"}}"#,
            r#"{"type":"tool_use","id":"t1","name":"bash","input":{"cmd":"ls"}}"#,
            r#"{"type":"tool_result","tool_use_id":"t1","content":"file.txt"}"#,
            r#"{"type":"task_started","task_id":"sa1","name":"researcher"}"#,
            r#"{"type":"task_progress","message":"working"}"#,
            r#"{"type":"task_completed","task_id":"sa1","name":"researcher","status":"success"}"#,
            r#"{"type":"rate_limit_event","is_throttled":true,"retry_after_ms":5000,"message":"Rate limited"}"#,
            r#"{"type":"some_future_event","deep":{"nested":[1,2,3]}}"#,
            r#"{"type":"result","duration_ms":5000,"num_turns":1,"stop_reason":"end_turn","cost_usd":0.003,"usage":{"input_tokens":500,"output_tokens":200}}"#,
        ]
    }

    #[test]
    fn round_trip_fidelity() {
        let (events, _) = replay(Provider::Claude, &fixture(), None);
        assert!(!events.is_empty());
        assert_round_trip(&events);
    }

    #[test]
    fn all_expected_kinds_present() {
        let (events, _) = replay(Provider::Claude, &fixture(), None);
        let kinds = kinds_of(&events);
        assert!(contains_kind(&events, "session_start"), "kinds = {kinds:?}");
        assert!(contains_kind(&events, "output_text"), "kinds = {kinds:?}");
        assert!(contains_kind(&events, "reasoning"), "kinds = {kinds:?}");
        assert!(contains_kind(&events, "tool_call"), "kinds = {kinds:?}");
        assert!(contains_kind(&events, "tool_result"), "kinds = {kinds:?}");
        assert!(
            contains_kind(&events, "subagent_start"),
            "kinds = {kinds:?}"
        );
        assert!(contains_kind(&events, "subagent_stop"), "kinds = {kinds:?}");
        assert!(contains_kind(&events, "info"), "kinds = {kinds:?}");
        assert!(contains_kind(&events, "warning"), "kinds = {kinds:?}");
        assert!(
            contains_kind(&events, "provider_extension"),
            "kinds = {kinds:?}"
        );
        assert!(contains_kind(&events, "turn_complete"), "kinds = {kinds:?}");
    }

    #[test]
    fn provider_extension_preserves_deeply_nested_payload() {
        let (events, _) = replay(Provider::Claude, &fixture(), None);
        let ext = events
            .iter()
            .find_map(|e| match e {
                SemanticEvent::ProviderExtension { payload, .. } => Some(payload.clone()),
                _ => None,
            })
            .expect("expected a ProviderExtension");
        assert_eq!(ext["deep"]["nested"], json!([1, 2, 3]));
    }

    #[test]
    fn summary_fields_populated() {
        let (_, parser) = replay(Provider::Claude, &fixture(), None);
        let summary = parser.finish(0);
        assert_eq!(summary.provider, Provider::Claude);
        assert_eq!(summary.session_id.as_deref(), Some("s1"));
        assert_eq!(summary.model.as_deref(), Some("claude-sonnet-4"));
        assert!(summary.assistant_text.contains("Hello"));
        assert!(summary.assistant_text.contains("world"));
        assert_eq!(summary.tool_calls, Some(1));
        assert_eq!(summary.duration_ms, Some(5000));
        let tu = summary.token_usage.as_ref().unwrap();
        assert_eq!(tu.input, Some(500));
        assert_eq!(tu.output, Some(200));
        assert_eq!(tu.total, Some(700));
    }

    #[test]
    fn malformed_json_produces_warning_not_error() {
        let (events, _) = replay(Provider::Claude, &["not json {{{"], None);
        assert!(matches!(
            events.first(),
            Some(SemanticEvent::Warning { .. })
        ));
    }
}

mod codex_round_trip {
    use super::*;

    fn fixture() -> Vec<&'static str> {
        vec![
            r#"{"type":"thread.started","thread_id":"th-1"}"#,
            r#"{"type":"turn.started"}"#,
            r#"{"type":"item.started","item":{"id":"cmd1","type":"command_exec","tool_name":"bash","input":{"command":"ls"}}}"#,
            r#"{"type":"item.updated","item":{"id":"r1","type":"reasoning","text":"thinking"}}"#,
            r#"{"type":"item.completed","item":{"id":"cmd1","type":"command_exec","status":"success","exit_code":0,"output":"file.txt"}}"#,
            r#"{"type":"item.completed","item":{"id":"f1","type":"file_change","path":"src/lib.rs","change_kind":"modified"}}"#,
            r#"{"type":"item.completed","item":{"id":"p1","type":"plan_update","message":"Step 2"}}"#,
            r#"{"type":"item.completed","item":{"id":"a1","type":"agent_message","text":"Done"}}"#,
            r#"{"type":"future.unknown","payload":{"x":42}}"#,
            r#"{"type":"turn.completed","usage":{"input_tokens":100,"output_tokens":50},"duration_ms":800,"status":"completed"}"#,
        ]
    }

    #[test]
    fn round_trip_fidelity() {
        let (events, _) = replay(Provider::Codex, &fixture(), Some("codex-mini".into()));
        assert!(!events.is_empty());
        assert_round_trip(&events);
    }

    #[test]
    fn all_expected_kinds_present() {
        let (events, _) = replay(Provider::Codex, &fixture(), Some("codex-mini".into()));
        let kinds = kinds_of(&events);
        assert!(contains_kind(&events, "session_start"), "kinds = {kinds:?}");
        assert!(contains_kind(&events, "turn_start"), "kinds = {kinds:?}");
        assert!(contains_kind(&events, "tool_call"), "kinds = {kinds:?}");
        assert!(contains_kind(&events, "tool_result"), "kinds = {kinds:?}");
        assert!(contains_kind(&events, "file_change"), "kinds = {kinds:?}");
        assert!(contains_kind(&events, "plan_update"), "kinds = {kinds:?}");
        assert!(
            contains_kind(&events, "provider_extension"),
            "kinds = {kinds:?}"
        );
        assert!(contains_kind(&events, "turn_complete"), "kinds = {kinds:?}");
    }

    #[test]
    fn summary_fields_populated() {
        let (_, parser) = replay(Provider::Codex, &fixture(), Some("codex-mini".into()));
        let summary = parser.finish(0);
        assert_eq!(summary.provider, Provider::Codex);
        assert_eq!(summary.session_id.as_deref(), Some("th-1"));
        assert_eq!(summary.tool_calls, Some(1));
        assert_eq!(summary.assistant_text, "Done");
        assert_eq!(summary.duration_ms, Some(800));
    }
}

mod gemini_round_trip {
    use super::*;

    fn fixture() -> Vec<&'static str> {
        vec![
            r#"{"type":"init","session_id":"g1","model":"gemini-2.5-pro"}"#,
            r#"{"type":"message","role":"assistant","content":"Hello"}"#,
            r#"{"type":"tool_use","tool_id":"t1","tool_name":"search","parameters":{"q":"rust"}}"#,
            r#"{"type":"tool_result","tool_id":"t1","status":"success","output":{"hits":3}}"#,
            r#"{"type":"error","severity":"warning","message":"Loop detected"}"#,
            r#"{"type":"some_unknown","data":"x"}"#,
            r#"{"type":"result","status":"success","stats":{"input_tokens":500,"output_tokens":250,"cached":100,"duration_ms":8000,"tool_calls":2}}"#,
        ]
    }

    #[test]
    fn round_trip_fidelity() {
        let (events, _) = replay(Provider::Gemini, &fixture(), None);
        assert!(!events.is_empty());
        assert_round_trip(&events);
    }

    #[test]
    fn all_expected_kinds_present() {
        let (events, _) = replay(Provider::Gemini, &fixture(), None);
        let kinds = kinds_of(&events);
        assert!(contains_kind(&events, "session_start"), "kinds = {kinds:?}");
        assert!(contains_kind(&events, "output_text"), "kinds = {kinds:?}");
        assert!(contains_kind(&events, "tool_call"), "kinds = {kinds:?}");
        assert!(contains_kind(&events, "tool_result"), "kinds = {kinds:?}");
        assert!(contains_kind(&events, "warning"), "kinds = {kinds:?}");
        assert!(
            contains_kind(&events, "provider_extension"),
            "kinds = {kinds:?}"
        );
        assert!(contains_kind(&events, "turn_complete"), "kinds = {kinds:?}");
    }

    #[test]
    fn summary_fields_populated() {
        let (_, parser) = replay(Provider::Gemini, &fixture(), None);
        let summary = parser.finish(0);
        assert_eq!(summary.provider, Provider::Gemini);
        assert_eq!(summary.assistant_text, "Hello");
        assert_eq!(summary.tool_calls, Some(2));
        let tu = summary.token_usage.as_ref().unwrap();
        assert_eq!(tu.input, Some(500));
        assert_eq!(tu.output, Some(250));
    }
}

mod kimi_round_trip {
    use super::*;

    fn fixture() -> Vec<&'static str> {
        vec![
            r#"{"jsonrpc":"2.0","id":"init-1","result":{"protocol_version":"1.9","server":{"name":"Kimi Code CLI","version":"1.38.0"},"slash_commands":[],"hooks":[],"capabilities":{"supports_question":true}}}"#,
            r#"{"jsonrpc":"2.0","method":"event","params":{"type":"TurnBegin","payload":{"user_input":"hi"}}}"#,
            r#"{"jsonrpc":"2.0","method":"event","params":{"type":"ContentPart","payload":{"type":"text","text":"Hello from Kimi"}}}"#,
            r#"{"jsonrpc":"2.0","method":"event","params":{"type":"ToolCall","payload":{"id":"k1","function":{"name":"Shell","arguments":"{\"cmd\":\"ls\"}"}}}}"#,
            r#"{"jsonrpc":"2.0","method":"event","params":{"type":"ToolResult","payload":{"tool_call_id":"k1","return_value":{"is_error":false,"output":"clean"}}}}"#,
            r#"{"jsonrpc":"2.0","method":"event","params":{"type":"TurnEnd","payload":{}}}"#,
            r#"{"jsonrpc":"2.0","id":"prompt-2","error":{"code":-32004,"message":"Authentication expired","data":null}}"#,
            r#"{"jsonrpc":"2.0","method":"event","params":{"type":"FutureUnknownEvent","payload":{}}}"#,
        ]
    }

    #[test]
    fn round_trip_fidelity() {
        let (events, _) = replay(Provider::KimiCode, &fixture(), None);
        assert!(!events.is_empty());
        assert_round_trip(&events);
    }

    #[test]
    fn all_expected_kinds_present() {
        let (events, _) = replay(Provider::KimiCode, &fixture(), None);
        assert!(contains_kind(&events, "session_start"));
        assert!(contains_kind(&events, "output_text"));
        assert!(contains_kind(&events, "tool_call"));
        assert!(contains_kind(&events, "tool_result"));
        assert!(contains_kind(&events, "error"));
        assert!(contains_kind(&events, "provider_extension"));
    }
}

mod opencode_round_trip {
    use super::*;

    fn fixture() -> Vec<&'static str> {
        vec![
            r#"{"type":"step_start","sessionID":"ses_1"}"#,
            r#"{"type":"text","text":"hello"}"#,
            r#"{"type":"tool_start","part":{"id":"t1","tool_name":"bash","input":{"cmd":"ls"}}}"#,
            r#"{"type":"tool_end","part":{"tool_use_id":"t1","status":"success","content":"ok"}}"#,
            r#"{"type":"error","error_message":"API timeout"}"#,
            r#"{"type":"some_future_event","x":1}"#,
            r#"{"type":"step_complete","usage":{"input_tokens":10,"output_tokens":5,"total_tokens":15},"cost_usd":0.001,"duration_ms":200}"#,
        ]
    }

    #[test]
    fn round_trip_fidelity() {
        let (events, _) = replay(Provider::OpenCode, &fixture(), Some("gpt-4o".into()));
        assert!(!events.is_empty());
        assert_round_trip(&events);
    }

    #[test]
    fn all_expected_kinds_present() {
        let (events, _) = replay(Provider::OpenCode, &fixture(), Some("gpt-4o".into()));
        assert!(contains_kind(&events, "session_start"));
        assert!(contains_kind(&events, "output_text"));
        assert!(contains_kind(&events, "tool_call"));
        assert!(contains_kind(&events, "tool_result"));
        assert!(contains_kind(&events, "error"));
        assert!(contains_kind(&events, "provider_extension"));
        assert!(contains_kind(&events, "turn_complete"));
    }

    #[test]
    fn summary_fields_populated() {
        let (_, parser) = replay(Provider::OpenCode, &fixture(), Some("gpt-4o".into()));
        let summary = parser.finish(0);
        assert_eq!(summary.provider, Provider::OpenCode);
        assert_eq!(summary.assistant_text, "hello");
        assert_eq!(summary.tool_calls, Some(1));
    }

    #[test]
    fn tool_start_input_survives_to_tool_result_display() {
        // Regression for the 2026-04-18 OpenCode reporting contract:
        // `tool_start` → `tool_end` pairs must carry the request-side input
        // through to the emitted `ToolResult` so renderers can annotate
        // successful incoming events with a meaningful slot.
        use claudine::stream::tool_display::{ToolCallDisplay, ToolStatus};

        let lines = [
            r#"{"type":"tool_start","part":{"id":"t1","tool_name":"bash","input":{"command":"ls -la"}}}"#,
            r#"{"type":"tool_end","part":{"tool_use_id":"t1","status":"success","content":"ok"}}"#,
        ];
        let (events, _) = replay(Provider::OpenCode, &lines, Some("gpt-4o".into()));
        let result = events
            .iter()
            .find(|e| matches!(e, SemanticEvent::ToolResult { .. }))
            .expect("expected a ToolResult");
        let SemanticEvent::ToolResult { extra, .. } = result else {
            unreachable!()
        };
        assert_eq!(
            extra.get("input").and_then(|v| v.get("command")),
            Some(&json!("ls -la")),
            "OpenCode tool_start input must survive into ToolResult.extra"
        );

        // And the display layer must surface it alongside a resolved status.
        let display = ToolCallDisplay::from_result(result).expect("Bash ToolResult");
        assert_eq!(display.status, Some(ToolStatus::Success));
        assert_eq!(
            display.summary.as_deref(),
            Some("bash ls -la"),
            "successful Bash results must keep the cached command summary"
        );
    }
}

mod qwen_round_trip {
    use super::*;

    fn fixture() -> Vec<&'static str> {
        vec![
            r#"{"type":"init","session_id":"q1","model":"qwen-coder"}"#,
            r#"{"type":"message","role":"assistant","content":[{"text":"Hello from Qwen"}]}"#,
            r#"{"type":"tool_call","id":"q1","name":"bash","input":{"command":"git status"}}"#,
            r#"{"type":"tool_response","tool_use_id":"q1","status":"success","content":"clean"}"#,
            r#"{"type":"error","error":{"type":"rate_limit","message":"slow down"}}"#,
            r#"{"type":"something.new"}"#,
            r#"{"type":"summary","duration_ms":3000,"token_usage":{"input_tokens":100,"output_tokens":50}}"#,
        ]
    }

    #[test]
    fn round_trip_fidelity() {
        let (events, _) = replay(Provider::QwenCode, &fixture(), None);
        assert!(!events.is_empty());
        assert_round_trip(&events);
    }

    #[test]
    fn all_expected_kinds_present() {
        let (events, _) = replay(Provider::QwenCode, &fixture(), None);
        assert!(contains_kind(&events, "session_start"));
        assert!(contains_kind(&events, "output_text"));
        assert!(contains_kind(&events, "tool_call"));
        assert!(contains_kind(&events, "tool_result"));
        assert!(contains_kind(&events, "error"));
        assert!(contains_kind(&events, "provider_extension"));
        assert!(contains_kind(&events, "turn_complete"));
    }

    #[test]
    fn summary_fields_populated() {
        let (_, parser) = replay(Provider::QwenCode, &fixture(), None);
        let summary = parser.finish(0);
        assert_eq!(summary.provider, Provider::QwenCode);
        assert_eq!(summary.assistant_text, "Hello from Qwen");
        assert_eq!(summary.tool_calls, Some(1));
        assert_eq!(summary.duration_ms, Some(3000));
    }
}

mod summary_regression {
    use super::*;
    use claudine::stream::summary::StreamExecutionSummary;

    #[test]
    fn summary_fields_unchanged_by_claude_replay() {
        let (_, parser) = replay(
            Provider::Claude,
            &[
                r#"{"type":"init","session_id":"s1","model":"m1"}"#,
                r#"{"type":"assistant","content":[{"type":"text","text":"Hello world"}]}"#,
                r#"{"type":"tool_use","id":"t1","name":"bash","input":{"cmd":"ls"}}"#,
                r#"{"type":"tool_result","tool_use_id":"t1","content":"ok"}"#,
                r#"{"type":"result","duration_ms":1000,"num_turns":1,"stop_reason":"end_turn","cost_usd":0.01,"usage":{"input_tokens":100,"output_tokens":50}}"#,
            ],
            None,
        );
        let s = parser.finish(0);
        assert_eq!(s.provider, Provider::Claude);
        assert_eq!(s.session_id.as_deref(), Some("s1"));
        assert_eq!(s.model.as_deref(), Some("m1"));
        assert_eq!(s.assistant_text, "Hello world");
        assert_eq!(s.tool_calls, Some(1));
        assert_eq!(s.duration_ms, Some(1000));
        assert_eq!(s.cost_usd, Some(0.01));
        assert_eq!(s.provider_status.as_deref(), Some("end_turn"));
        assert_eq!(s.exit_code, 0);
        assert!(!s.is_error);
        let tu = s.token_usage.unwrap();
        assert_eq!(tu.input, Some(100));
        assert_eq!(tu.output, Some(50));
        assert_eq!(tu.total, Some(150));
    }

    #[test]
    fn error_summary_populated_from_error_event() {
        let (_, parser) = replay(
            Provider::Claude,
            &[
                r#"{"type":"init","session_id":"s1","model":"m1"}"#,
                r#"{"type":"error","error":{"type":"billing_error","message":"Insufficient credits"}}"#,
            ],
            None,
        );
        let s = parser.finish(1);
        assert!(s.is_error);
        assert_eq!(s.error_kind.as_deref(), Some("billing_error"));
        assert_eq!(s.error_message.as_deref(), Some("Insufficient credits"));
        assert_eq!(s.exit_code, 1);
    }

    #[test]
    fn assistant_text_concatenated_across_multiple_messages() {
        let (_, parser) = replay(
            Provider::Claude,
            &[
                r#"{"type":"assistant","content":[{"type":"text","text":"First. "}]}"#,
                r#"{"type":"assistant","content":[{"type":"text","text":"Second."}]}"#,
            ],
            None,
        );
        let s = parser.finish(0);
        assert_eq!(s.assistant_text, "First. Second.");
    }

    #[test]
    fn empty_replay_produces_default_summary() {
        let (_, parser) = replay(Provider::Claude, &[], None);
        let s = parser.finish(0);
        assert_eq!(s.provider, Provider::Claude);
        assert_eq!(s.assistant_text, "");
        assert_eq!(s.exit_code, 0);
        assert!(!s.is_error);
    }

    #[test]
    fn codex_permission_counters_tracked() {
        let (_, parser) = replay(
            Provider::Codex,
            &[
                r#"{"type":"thread.started","thread_id":"t1"}"#,
                r#"{"type":"item.started","item":{"id":"p1","type":"permission_request","name":"bash"}}"#,
                r#"{"type":"item.started","item":{"id":"u1","type":"user_input_request","name":"clarify"}}"#,
            ],
            Some("codex-mini".into()),
        );
        let s = parser.finish(0);
        assert_eq!(s.permission_prompts, Some(1));
        assert_eq!(s.user_input_prompts, Some(1));
    }

    #[test]
    fn summary_serde_round_trip_after_replay() {
        let (_, parser) = replay(
            Provider::Claude,
            &[
                r#"{"type":"init","session_id":"s1","model":"m1"}"#,
                r#"{"type":"assistant","content":[{"type":"text","text":"Hello"}]}"#,
                r#"{"type":"result","duration_ms":100,"usage":{"input_tokens":10,"output_tokens":5}}"#,
            ],
            None,
        );
        let s = parser.finish(0);
        let json = serde_json::to_string(&s).unwrap();
        let restored: StreamExecutionSummary = serde_json::from_str(&json).unwrap();
        assert_eq!(s, restored);
    }
}

mod reporting_fidelity {
    use super::*;

    #[test]
    fn semantic_event_to_event_meta_preserves_provider_extension_payload() {
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
        let meta = semantic_event_to_event_meta(&event, Provider::Claude, &env(), None);
        assert_eq!(meta.provider, Provider::Codex);
        assert_eq!(meta.extra["synthetic"], Value::Bool(true));
        assert_eq!(
            meta.extra["synthetic_kind"],
            Value::String("stream_semantic_event".into())
        );
        assert_eq!(
            meta.extra["semantic_kind"],
            Value::String("provider_extension".into())
        );
        let sem = meta
            .extra
            .get("semantic_event")
            .expect("semantic_event missing");
        let decoded: SemanticEvent = serde_json::from_value(sem.clone()).unwrap();
        match decoded {
            SemanticEvent::ProviderExtension { payload: p, .. } => assert_eq!(p, payload),
            other => panic!("expected ProviderExtension, got {other:?}"),
        }
    }

    #[test]
    fn every_semantic_kind_has_correct_tag() {
        for (event, expected_kind) in [
            (
                SemanticEvent::SessionStart {
                    session_id: None,
                    model: None,
                    extra: json!({}),
                },
                "session_start",
            ),
            (SemanticEvent::TurnStart { extra: json!({}) }, "turn_start"),
            (
                SemanticEvent::TurnComplete {
                    provider_status: None,
                    token_usage: None,
                    cost_usd: None,
                    duration_ms: None,
                    extra: json!({}),
                },
                "turn_complete",
            ),
            (
                SemanticEvent::OutputText {
                    text: "x".into(),
                    extra: json!({}),
                },
                "output_text",
            ),
            (
                SemanticEvent::Reasoning {
                    text: "x".into(),
                    extra: json!({}),
                },
                "reasoning",
            ),
            (
                SemanticEvent::ToolCall {
                    name: None,
                    id: None,
                    input: None,
                    extra: json!({}),
                },
                "tool_call",
            ),
            (
                SemanticEvent::ToolResult {
                    name: None,
                    id: None,
                    status: None,
                    exit_code: None,
                    output: None,
                    extra: json!({}),
                },
                "tool_result",
            ),
            (
                SemanticEvent::PermissionRequest {
                    kind: None,
                    tool_name: None,
                    extra: json!({}),
                },
                "permission_request",
            ),
            (
                SemanticEvent::SubagentStart {
                    name: None,
                    id: None,
                    extra: json!({}),
                },
                "subagent_start",
            ),
            (
                SemanticEvent::SubagentStop {
                    name: None,
                    id: None,
                    status: None,
                    extra: json!({}),
                },
                "subagent_stop",
            ),
            (
                SemanticEvent::FileChange {
                    path: None,
                    change_kind: None,
                    extra: json!({}),
                },
                "file_change",
            ),
            (
                SemanticEvent::PlanUpdate {
                    message: None,
                    extra: json!({}),
                },
                "plan_update",
            ),
            (
                SemanticEvent::Info {
                    message: "x".into(),
                    extra: json!({}),
                },
                "info",
            ),
            (
                SemanticEvent::Warning {
                    message: "x".into(),
                    extra: json!({}),
                },
                "warning",
            ),
            (
                SemanticEvent::Error {
                    message: "x".into(),
                    terminal: false,
                    kind: SemanticErrorKind::Unknown,
                    extra: json!({}),
                },
                "error",
            ),
            (
                SemanticEvent::ProviderExtension {
                    provider: Provider::Claude,
                    kind: "test".into(),
                    payload: json!({}),
                },
                "provider_extension",
            ),
        ] {
            let meta = semantic_event_to_event_meta(&event, Provider::Claude, &env(), None);
            assert_eq!(
                meta.extra["semantic_kind"],
                Value::String(expected_kind.into()),
                "semantic_kind mismatch for {expected_kind}"
            );
            let sem = meta
                .extra
                .get("semantic_event")
                .expect("semantic_event missing");
            assert_eq!(
                sem["type"], expected_kind,
                "type tag mismatch for {expected_kind}"
            );
        }
    }

    #[test]
    fn tool_call_maps_to_before_tool_in_event_meta() {
        let event = SemanticEvent::ToolCall {
            name: Some("bash".into()),
            id: Some("t1".into()),
            input: Some(json!({"cmd": "ls"})),
            extra: json!({}),
        };
        let meta = semantic_event_to_event_meta(&event, Provider::Claude, &env(), None);
        assert_eq!(meta.event, claudine::events::AgenticEvent::BeforeTool);
        assert_eq!(meta.tool_name.as_deref(), Some("bash"));
        assert_eq!(meta.session_id.as_deref(), Some("t1"));
    }

    #[test]
    fn tool_result_maps_to_after_tool_in_event_meta() {
        let event = SemanticEvent::ToolResult {
            name: Some("bash".into()),
            id: Some("t1".into()),
            status: Some("success".into()),
            exit_code: Some(0),
            output: Some(json!("ok")),
            extra: json!({}),
        };
        let meta = semantic_event_to_event_meta(&event, Provider::Claude, &env(), None);
        assert_eq!(meta.event, claudine::events::AgenticEvent::AfterTool);
        assert_eq!(meta.tool_name.as_deref(), Some("bash"));
        assert_eq!(meta.tool_response, Some(json!("ok")));
    }

    #[test]
    fn subagent_start_stop_map_correctly() {
        let start = SemanticEvent::SubagentStart {
            name: Some("researcher".into()),
            id: Some("sa1".into()),
            extra: json!({}),
        };
        let stop = SemanticEvent::SubagentStop {
            name: Some("researcher".into()),
            id: Some("sa1".into()),
            status: Some("success".into()),
            extra: json!({}),
        };
        let m1 = semantic_event_to_event_meta(&start, Provider::Claude, &env(), None);
        let m2 = semantic_event_to_event_meta(&stop, Provider::Claude, &env(), None);
        assert_eq!(m1.event, claudine::events::AgenticEvent::SubagentStart);
        assert_eq!(m2.event, claudine::events::AgenticEvent::SubagentStop);
    }

    #[test]
    fn error_terminal_maps_to_turn_error() {
        let event = SemanticEvent::Error {
            message: "billing failed".into(),
            terminal: true,
            kind: SemanticErrorKind::Unknown,
            extra: json!({}),
        };
        let meta = semantic_event_to_event_meta(&event, Provider::Claude, &env(), None);
        assert_eq!(meta.event, claudine::events::AgenticEvent::TurnError);
        assert_eq!(meta.error.as_deref(), Some("billing failed"));
    }

    #[test]
    fn warning_maps_to_notification_with_error_field() {
        let event = SemanticEvent::Warning {
            message: "rate limited".into(),
            extra: json!({}),
        };
        let meta = semantic_event_to_event_meta(&event, Provider::Claude, &env(), None);
        assert_eq!(meta.event, claudine::events::AgenticEvent::Notification);
        assert_eq!(meta.error.as_deref(), Some("rate limited"));
        assert_eq!(meta.notification_type.as_deref(), Some("warning"));
    }

    #[test]
    fn context_extra_merges_into_semantic_event_meta() {
        let event = SemanticEvent::Info {
            message: "x".into(),
            extra: json!({}),
        };
        let mut ctx = std::collections::HashMap::new();
        ctx.insert(
            "composition_file_ref".into(),
            Value::String("notes.md".into()),
        );
        let meta = semantic_event_to_event_meta(&event, Provider::Claude, &env(), Some(&ctx));
        assert_eq!(
            meta.extra["composition_file_ref"],
            Value::String("notes.md".into())
        );
    }

    #[test]
    fn full_fixture_replay_reporting_chain() {
        let (events, parser) = replay(
            Provider::Claude,
            &[
                r#"{"type":"init","session_id":"s1","model":"m1"}"#,
                r#"{"type":"assistant","content":[{"type":"text","text":"Hello"}]}"#,
                r#"{"type":"tool_use","id":"t1","name":"bash","input":{"cmd":"ls"}}"#,
                r#"{"type":"tool_result","tool_use_id":"t1","content":"ok"}"#,
                r#"{"type":"rate_limit_event","is_throttled":true,"retry_after_ms":5000,"message":"Rate limited"}"#,
                r#"{"type":"some_future_event","x":1}"#,
                r#"{"type":"result","duration_ms":100,"usage":{"input_tokens":10,"output_tokens":5}}"#,
            ],
            None,
        );

        for event in &events {
            let meta = semantic_event_to_event_meta(event, Provider::Claude, &env(), None);
            assert!(meta.extra.contains_key("semantic_event"));
            assert_eq!(meta.extra["synthetic"], Value::Bool(true));
            assert_eq!(
                meta.extra["synthetic_kind"],
                Value::String("stream_semantic_event".into())
            );
            assert_eq!(
                meta.extra["semantic_kind"],
                Value::String(event.kind_str().into())
            );
        }

        let summary = parser.finish(0);
        let summary_meta = summary_to_event_meta(&summary, StreamProtocol::StreamJson, &env());
        assert_eq!(
            summary_meta.event,
            claudine::events::AgenticEvent::SessionEnd
        );
        assert_eq!(summary_meta.extra["synthetic"], Value::Bool(true));
        assert_eq!(
            summary_meta.extra["synthetic_kind"],
            Value::String("stream_wrapper_summary".into())
        );
    }
}
