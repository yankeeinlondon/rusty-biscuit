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
    Box<CodexSemanticStreamParser<Recording>>,
) {
    let events = Arc::new(Mutex::new(Vec::new()));
    let sink = Recording {
        events: events.clone(),
    };
    (
        events,
        Box::new(CodexSemanticStreamParser::new(
            sink,
            Some("codex-mini".into()),
        )),
    )
}

fn kinds(events: &[SemanticEvent]) -> Vec<&'static str> {
    events.iter().map(|e| e.kind_str()).collect()
}

#[test]
fn thread_started_emits_session_start() {
    let (events, mut parser) = new_parser();
    parser
        .feed_line(r#"{"type":"thread.started","thread_id":"thrd-1"}"#);
    let collected = events.lock().unwrap().clone();
    assert!(matches!(
        collected[0],
        SemanticEvent::SessionStart { ref session_id, .. }
            if session_id.as_deref() == Some("thrd-1")
    ));
}

#[test]
fn turn_lifecycle_emits_turn_start_and_complete() {
    let (events, mut parser) = new_parser();
    parser.feed_line(r#"{"type":"turn.started"}"#);
    parser
        .feed_line(
            r#"{"type":"turn.completed","usage":{"input_tokens":100,"output_tokens":50},"duration_ms":1200,"status":"completed"}"#,
        );
    let ks = kinds(&events.lock().unwrap());
    assert_eq!(ks, vec!["turn_start", "turn_complete"]);

    let summary = parser.finish(0);
    assert_eq!(summary.num_turns, Some(1));
    assert_eq!(summary.duration_ms, Some(1200));
    assert_eq!(summary.provider_status.as_deref(), Some("completed"));
}

#[test]
fn reasoning_item_emits_reasoning_event() {
    let (events, mut parser) = new_parser();
    parser
        .feed_line(
            r#"{"type":"item.completed","item":{"id":"r1","type":"reasoning","text":"long thought"}}"#,
        );
    let collected = events.lock().unwrap().clone();
    assert!(matches!(
        collected[0],
        SemanticEvent::Reasoning { ref text, .. } if text == "long thought"
    ));
}

#[test]
fn item_updated_reasoning_emits_reasoning_event() {
    let (events, mut parser) = new_parser();
    parser
        .feed_line(
            r#"{"type":"item.updated","item":{"id":"r1","type":"reasoning","text":"still thinking"}}"#,
        );
    let collected = events.lock().unwrap().clone();
    match &collected[0] {
        SemanticEvent::Reasoning { text, .. } => assert_eq!(text, "still thinking"),
        other => panic!("expected Reasoning, got {other:?}"),
    }
}

#[test]
fn item_updated_todo_list_emits_plan_update() {
    let (events, mut parser) = new_parser();
    parser
        .feed_line(
            r#"{"type":"item.updated","item":{"id":"p1","type":"todo_list","message":"step 2 done"}}"#,
        );
    let collected = events.lock().unwrap().clone();
    match &collected[0] {
        SemanticEvent::PlanUpdate { message, .. } => {
            assert_eq!(message.as_deref(), Some("step 2 done"));
        }
        other => panic!("expected PlanUpdate, got {other:?}"),
    }
}

#[test]
fn item_updated_command_execution_is_suppressed() {
    let (events, mut parser) = new_parser();
    parser
        .feed_line(
            r#"{"type":"item.updated","item":{"id":"cmd1","type":"command_execution","command":"make","aggregated_output":"partial"}}"#,
        );
    let collected = events.lock().unwrap().clone();
    assert!(
        collected.is_empty(),
        "command_execution progress must not leak 'item.updated' Info events; got {collected:?}"
    );
}

#[test]
fn item_updated_unknown_item_is_suppressed() {
    let (events, mut parser) = new_parser();
    parser
        .feed_line(r#"{"type":"item.updated","item":{"id":"x1","type":"brand_new_kind"}}"#);
    let collected = events.lock().unwrap().clone();
    assert!(
        collected.is_empty(),
        "unknown item.updated types must not leak on stderr; got {collected:?}"
    );
}

#[test]
fn file_change_item_emits_file_change_event() {
    let (events, mut parser) = new_parser();
    parser
        .feed_line(
            r#"{"type":"item.completed","item":{"id":"f1","type":"file_change","path":"src/lib.rs","change_kind":"modified"}}"#,
        );
    let collected = events.lock().unwrap().clone();
    match &collected[0] {
        SemanticEvent::FileChange {
            path, change_kind, ..
        } => {
            assert_eq!(path.as_deref(), Some("src/lib.rs"));
            assert_eq!(change_kind.as_deref(), Some("modified"));
        }
        other => panic!("expected FileChange, got {other:?}"),
    }
}

#[test]
fn file_change_item_fans_out_per_changes_entry() {
    let (events, mut parser) = new_parser();
    parser
        .feed_line(
            r#"{"type":"item.completed","item":{"id":"f1","type":"file_change","changes":[{"path":"src/lib.rs","kind":"update"},{"path":"tests/smoke.rs","kind":"add"}],"status":"completed"}}"#,
        );
    let collected = events.lock().unwrap().clone();
    let file_changes: Vec<_> = collected
        .iter()
        .filter_map(|e| match e {
            SemanticEvent::FileChange {
                path, change_kind, ..
            } => Some((path.clone(), change_kind.clone())),
            _ => None,
        })
        .collect();
    assert_eq!(
        file_changes,
        vec![
            (Some("src/lib.rs".into()), Some("update".into())),
            (Some("tests/smoke.rs".into()), Some("add".into())),
        ]
    );
}

#[test]
fn file_change_item_without_path_or_kind_is_suppressed() {
    let (events, mut parser) = new_parser();
    parser
        .feed_line(r#"{"type":"item.started","item":{"id":"f1","type":"file_change"}}"#);
    let collected = events.lock().unwrap().clone();
    assert!(
        !collected
            .iter()
            .any(|e| matches!(e, SemanticEvent::FileChange { .. })),
        "empty file_change must not emit a FileChange event"
    );
}

#[test]
fn plan_update_item_emits_plan_update_event() {
    let (events, mut parser) = new_parser();
    parser
        .feed_line(
            r#"{"type":"item.completed","item":{"id":"p1","type":"plan_update","message":"Step 2 of 5"}}"#,
        );
    let collected = events.lock().unwrap().clone();
    match &collected[0] {
        SemanticEvent::PlanUpdate { message, .. } => {
            assert_eq!(message.as_deref(), Some("Step 2 of 5"));
        }
        other => panic!("expected PlanUpdate, got {other:?}"),
    }
}

#[test]
fn todo_list_routed_as_plan_update() {
    let (events, mut parser) = new_parser();
    parser
        .feed_line(
            r#"{"type":"item.completed","item":{"id":"t1","type":"todo_list","title":"next steps"}}"#,
        );
    let collected = events.lock().unwrap().clone();
    assert!(matches!(collected[0], SemanticEvent::PlanUpdate { .. }));
}

#[test]
fn command_execution_status_and_exit_code_preserved() {
    let (events, mut parser) = new_parser();
    // Started: records input
    parser
        .feed_line(
            r#"{"type":"item.started","item":{"id":"cmd1","type":"command_exec","tool_name":"bash","input":{"command":"false"}}}"#,
        );
    // Completed: brings status + exit_code
    parser
        .feed_line(
            r#"{"type":"item.completed","item":{"id":"cmd1","type":"command_exec","status":"failure","exit_code":1,"output":"boom"}}"#,
        );

    let collected = events.lock().unwrap().clone();
    let ks = kinds(&collected);
    assert_eq!(ks, vec!["tool_call", "tool_result"]);
    match &collected[1] {
        SemanticEvent::ToolResult {
            status,
            exit_code,
            output,
            name,
            ..
        } => {
            assert_eq!(status.as_deref(), Some("failure"));
            assert_eq!(*exit_code, Some(1));
            assert_eq!(output.as_ref().and_then(Value::as_str), Some("boom"));
            assert_eq!(name.as_deref(), Some("bash"));
        }
        other => panic!("expected ToolResult, got {other:?}"),
    }
}

#[test]
fn codex_command_execution_populates_tool_call_and_result_fields() {
    // Phase 2d.1 canonical assertion: name + input populated on ToolCall,
    // name + status + exit_code + output populated on ToolResult (not
    // just inside `extra`). Locks the contract for every tool-item type.
    let (events, mut parser) = new_parser();
    parser
        .feed_line(r#"{"type":"thread.started","thread_id":"th-1"}"#);
    parser
        .feed_line(
            r#"{"type":"item.started","item":{"id":"cmd1","type":"command_execution","tool_name":"bash","input":{"command":"ls"}}}"#,
        );
    parser
        .feed_line(
            r#"{"type":"item.completed","item":{"id":"cmd1","type":"command_execution","tool_name":"bash","status":"success","exit_code":0,"output":"file.txt"}}"#,
        );

    let evs = events.lock().unwrap().clone();

    let (call_name, call_input) = evs
        .iter()
        .find_map(|e| match e {
            SemanticEvent::ToolCall { name, input, .. } => Some((name.clone(), input.clone())),
            _ => None,
        })
        .expect("ToolCall emitted");
    assert_eq!(call_name.as_deref(), Some("bash"));
    assert_eq!(
        call_input
            .as_ref()
            .and_then(|v| v.get("command"))
            .and_then(Value::as_str),
        Some("ls"),
    );

    let (r_name, r_status, r_exit, r_output) = evs
        .iter()
        .find_map(|e| match e {
            SemanticEvent::ToolResult {
                name,
                status,
                exit_code,
                output,
                ..
            } => Some((name.clone(), status.clone(), *exit_code, output.clone())),
            _ => None,
        })
        .expect("ToolResult emitted");
    assert_eq!(r_name.as_deref(), Some("bash"));
    assert_eq!(r_status.as_deref(), Some("success"));
    assert_eq!(r_exit, Some(0));
    assert!(r_output.is_some(), "output must be populated on ToolResult");
}

#[test]
fn permission_request_emits_permission_request_event() {
    let (events, mut parser) = new_parser();
    parser
        .feed_line(
            r#"{"type":"item.started","item":{"id":"perm-1","type":"permission_request","name":"bash"}}"#,
        );
    let collected = events.lock().unwrap().clone();
    assert!(matches!(
        collected[0],
        SemanticEvent::PermissionRequest { ref tool_name, .. }
            if tool_name.as_deref() == Some("bash")
    ));
    let summary = parser.finish(0);
    assert_eq!(summary.permission_prompts, Some(1));
}

#[test]
fn user_input_request_increments_separate_counter() {
    let (events, mut parser) = new_parser();
    parser
        .feed_line(
            r#"{"type":"item.started","item":{"id":"inp-1","type":"user_input_request","name":"clarify"}}"#,
        );
    assert_eq!(kinds(&events.lock().unwrap()), vec!["permission_request"]);
    let summary = parser.finish(0);
    assert_eq!(summary.user_input_prompts, Some(1));
    assert_eq!(summary.permission_prompts, None);
}

#[test]
fn error_event_marks_summary_and_emits_error() {
    let (events, mut parser) = new_parser();
    parser
        .feed_line(
            r#"{"type":"error","error_type":"rate_limit","error_message":"Too many requests"}"#,
        );
    let collected = events.lock().unwrap().clone();
    assert!(matches!(
        collected[0],
        SemanticEvent::Error { terminal: true, .. }
    ));
    let summary = parser.finish(1);
    assert!(summary.is_error);
    assert_eq!(summary.error_kind.as_deref(), Some("rate_limit"));
}

#[test]
fn classify_error_rate_limit_kind_maps_to_api_remote() {
    assert_eq!(
        classify_error(Some("rate_limit"), Some("Too many requests")),
        SemanticErrorKind::ApiRemote,
    );
}

#[test]
fn classify_error_auth_kind_maps_to_configuration() {
    assert_eq!(
        classify_error(Some("auth_error"), Some("missing api key")),
        SemanticErrorKind::Configuration,
    );
}

#[test]
fn classify_error_unknown_kind_with_billing_message_maps_to_api_remote() {
    assert_eq!(
        classify_error(None, Some("Billing quota exceeded")),
        SemanticErrorKind::ApiRemote,
    );
}

#[test]
fn classify_error_overloaded_message_maps_to_api_remote() {
    assert_eq!(
        classify_error(None, Some("the selected model is overloaded, retry")),
        SemanticErrorKind::ApiRemote,
    );
}

#[test]
fn classify_error_overloaded_does_not_disturb_seed_precedence() {
    assert_eq!(
        classify_error(Some("auth_error"), Some("the service is overloaded")),
        SemanticErrorKind::Configuration,
    );
    assert_eq!(
        classify_error(None, Some("the selected model is available")),
        SemanticErrorKind::AgentNative,
    );
}

#[test]
fn turn_failed_capacity_message_maps_to_api_remote() {
    let (events, mut parser) = new_parser();
    parser
        .feed_line(
            r#"{"type":"turn.failed","error":{"message":"Selected model is at capacity. Please try a different model."}}"#,
        );

    let collected = events.lock().unwrap().clone();
    match &collected[0] {
        SemanticEvent::Error { kind, .. } => {
            assert_eq!(*kind, SemanticErrorKind::ApiRemote);
        }
        other => panic!("expected Error, got {other:?}"),
    }
}

#[test]
fn capacity_related_prose_does_not_match_narrow_codex_needle() {
    let (events, mut parser) = new_parser();
    parser
        .feed_line(
            r#"{"type":"turn.failed","error":{"message":"Capacity planning completed for the selected model."}}"#,
        );

    let collected = events.lock().unwrap().clone();
    match &collected[0] {
        SemanticEvent::Error { kind, .. } => {
            assert_eq!(*kind, SemanticErrorKind::AgentNative);
        }
        other => panic!("expected Error, got {other:?}"),
    }
}

#[test]
fn classify_error_defaults_to_agent_native() {
    assert_eq!(
        classify_error(Some("weird_kind"), Some("something broke")),
        SemanticErrorKind::AgentNative,
    );
}

#[test]
fn error_event_carries_typed_kind_in_semantic_event() {
    let (events, mut _parser) = new_parser();
    _parser
        .feed_line(
            r#"{"type":"error","error_type":"rate_limit","error_message":"Too many requests"}"#,
        );
    let collected = events.lock().unwrap().clone();
    match &collected[0] {
        SemanticEvent::Error { kind, .. } => {
            assert_eq!(*kind, SemanticErrorKind::ApiRemote);
        }
        other => panic!("expected Error, got {other:?}"),
    }
}

#[test]
fn duplicate_terminal_errors_are_deduplicated() {
    let (events, mut parser) = new_parser();
    // Simulate the real Codex rate-limit transcript: `turn.failed`
    // followed by a top-level `error` carrying the same resolved
    // kind + message. The live stderr surface previously rendered
    // both as identical "Agent Error" blocks.
    parser
        .feed_line(
            r#"{"type":"turn.failed","error":{"type":"rate_limit","message":"You've hit your usage limit."}}"#,
        );
    parser
        .feed_line(
            r#"{"type":"error","error_type":"rate_limit","error_message":"You've hit your usage limit."}"#,
        );
    let error_count = events
        .lock()
        .unwrap()
        .iter()
        .filter(|e| matches!(e, SemanticEvent::Error { .. }))
        .count();
    assert_eq!(error_count, 1, "duplicate errors should collapse to one");
    let summary = parser.finish(1);
    assert!(summary.is_error);
    assert_eq!(summary.error_kind.as_deref(), Some("rate_limit"));
}

#[test]
fn distinct_terminal_errors_are_both_emitted() {
    let (events, mut parser) = new_parser();
    parser
        .feed_line(
            r#"{"type":"stream.error","error":{"type":"network","message":"socket closed"}}"#,
        );
    parser
        .feed_line(
            r#"{"type":"error","error_type":"rate_limit","error_message":"Too many requests"}"#,
        );
    let error_count = events
        .lock()
        .unwrap()
        .iter()
        .filter(|e| matches!(e, SemanticEvent::Error { .. }))
        .count();
    assert_eq!(error_count, 2);
}

#[test]
fn agent_message_accumulates_text_without_emitting_output() {
    let (events, mut parser) = new_parser();
    parser
        .feed_line(
            r#"{"type":"item.completed","item":{"id":"a1","type":"agent_message","text":"hello from stream"}}"#,
        );
    let ks = kinds(&events.lock().unwrap());
    // No OutputText (file-based text source owns that), and no
    // ProviderExtension leak — the raw JSONL log preserves the event.
    assert!(!ks.contains(&"output_text"));
    assert!(!ks.contains(&"provider_extension"));
    let summary = parser.finish(0);
    assert_eq!(summary.assistant_text, "hello from stream");
}

#[test]
fn unknown_top_level_event_becomes_provider_extension() {
    let (events, mut parser) = new_parser();
    parser
        .feed_line(r#"{"type":"future.event.kind","payload":{"k":1}}"#);
    let collected = events.lock().unwrap().clone();
    match &collected[0] {
        SemanticEvent::ProviderExtension {
            provider,
            kind,
            payload,
        } => {
            assert_eq!(*provider, Provider::Codex);
            assert_eq!(kind, "future.event.kind");
            assert_eq!(payload.get("payload"), Some(&json!({"k": 1})));
        }
        other => panic!("expected ProviderExtension, got {other:?}"),
    }
}

#[test]
fn malformed_json_emits_warning_and_continues() {
    let (events, mut parser) = new_parser();
    parser.feed_line("not json");
    parser.feed_line(r#"{"type":"turn.started"}"#);
    let ks = kinds(&events.lock().unwrap());
    assert_eq!(ks, vec!["warning", "turn_start"]);
}

#[test]
fn top_level_tool_use_and_result() {
    let (events, mut parser) = new_parser();
    parser
        .feed_line(r#"{"type":"item.tool_use","name":"bash","input":{"cmd":"ls"}}"#);
    parser
        .feed_line(
            r#"{"type":"item.tool_result","name":"bash","status":"success","exit_code":0}"#,
        );
    let collected = events.lock().unwrap().clone();
    assert_eq!(kinds(&collected), vec!["tool_call", "tool_result"]);
    match &collected[1] {
        SemanticEvent::ToolResult {
            status, exit_code, ..
        } => {
            assert_eq!(status.as_deref(), Some("success"));
            assert_eq!(*exit_code, Some(0));
        }
        other => panic!("expected ToolResult, got {other:?}"),
    }
}

#[test]
fn empty_lines_emit_nothing() {
    let (events, mut parser) = new_parser();
    parser.feed_line("");
    parser.feed_line("   ");
    assert!(events.lock().unwrap().is_empty());
}

#[test]
fn round_trip_fidelity_mixed_fixture() {
    let (events, mut parser) = new_parser();
    for line in [
        r#"{"type":"thread.started","thread_id":"t1"}"#,
        r#"{"type":"turn.started"}"#,
        r#"{"type":"item.started","item":{"id":"cmd1","type":"command_exec","tool_name":"bash","input":{"command":"ls"}}}"#,
        r#"{"type":"item.updated","item":{"id":"r1","type":"reasoning","text":"thinking"}}"#,
        r#"{"type":"item.completed","item":{"id":"cmd1","type":"command_exec","status":"success","exit_code":0,"output":"file.txt"}}"#,
        r#"{"type":"item.completed","item":{"id":"f1","type":"file_change","path":"a.rs","change_kind":"modified"}}"#,
        r#"{"type":"item.completed","item":{"id":"p1","type":"plan_update","message":"next step"}}"#,
        r#"{"type":"turn.completed","usage":{"input_tokens":10,"output_tokens":5},"duration_ms":42,"status":"completed"}"#,
        r#"{"type":"future.unknown","k":1}"#,
    ] {
        parser.feed_line(line);
    }
    for event in events.lock().unwrap().iter() {
        let v = serde_json::to_value(event).unwrap();
        let decoded: SemanticEvent = serde_json::from_value(v.clone()).unwrap();
        let v2 = serde_json::to_value(&decoded).unwrap();
        assert_eq!(v, v2, "round-trip lost fidelity for {}", event.kind_str());
    }
}

#[test]
fn codex_fixture_command_execution_routes_to_tool_pair() {
    let (events, mut parser) = new_parser();
    parser
        .feed_line(r#"{"type":"item.started","item":{"id":"cmd1","type":"command_execution","command":"ls","aggregated_output":""}}"#);
    parser
        .feed_line(r#"{"type":"item.completed","item":{"id":"cmd1","type":"command_execution","command":"ls","aggregated_output":"file.txt\n","exit_code":0,"status":"success"}}"#);

    let captured = events.lock().unwrap().clone();
    let ks = kinds(&captured);
    assert_eq!(
        ks,
        vec!["tool_call", "tool_result"],
        "command_execution must route to paired ToolCall + ToolResult, got {ks:?}"
    );

    let SemanticEvent::ToolResult {
        status,
        exit_code,
        output,
        extra,
        ..
    } = &captured[1]
    else {
        panic!("expected ToolResult as second event, got {:?}", captured[1]);
    };
    assert_eq!(status.as_deref(), Some("success"));
    assert_eq!(*exit_code, Some(0));
    let output = output.as_ref().expect("output");
    assert_eq!(
        output.as_str(),
        Some("file.txt\n"),
        "aggregated_output must be preserved as the ToolResult output"
    );
    assert_eq!(
        extra.get("input"),
        Some(&json!({"command": "ls"})),
        "ToolResult extra must preserve the original command input so the live sink \
         does not fall back to rendering aggregated stdout in the status line"
    );
}

#[test]
fn codex_fixture_agent_message_does_not_leak_as_provider_extension() {
    let (events, mut parser) = new_parser();
    parser
        .feed_line(r#"{"type":"item.completed","item":{"id":"item_0","type":"agent_message","text":"Looking at the repo..."}}"#);

    let captured = events.lock().unwrap().clone();
    let ks = kinds(&captured);
    assert!(
        !ks.contains(&"provider_extension"),
        "agent_message must not leak to ProviderExtension; got {ks:?}"
    );
}

#[test]
fn codex_fixture_full_replay_produces_no_provider_extensions() {
    let fixture = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/providers/codex.ndjson"
    ))
    .expect("codex.ndjson must exist — Task 1 should have created it");

    let (events, mut parser) = new_parser();
    for line in fixture.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        parser.feed_line(line);
    }

    let captured = events.lock().unwrap().clone();
    let ext: Vec<&SemanticEvent> = captured
        .iter()
        .filter(|e| e.kind_str() == "provider_extension")
        .collect();
    assert!(
        ext.is_empty(),
        "captured fixture must not produce ProviderExtension events; found {} out of {}: {:#?}",
        ext.len(),
        captured.len(),
        ext.iter().take(3).collect::<Vec<_>>()
    );
}

#[test]
fn badges_derived_on_rate_limit_error() {
    let (_, mut parser) = new_parser();
    parser
        .feed_line(
            r#"{"type":"error","error_type":"rate_limit","error_message":"Too many requests"}"#,
        );
    let summary = parser.finish(1);
    assert_eq!(summary.badges.len(), 1);
    assert_eq!(
        summary.badges[0].category,
        crate::stream::badges::BadgeCategory::RateLimit
    );
}

#[test]
fn tool_call_extra_includes_status_when_present_on_started() {
    // Task 2d.1 regression: Codex emits `status="in_progress"` on
    // `item.started` for tool items. Previously the started-path
    // helper dropped it; the completed-path did not. This test locks
    // the symmetric behavior.
    let (events, mut parser) = new_parser();
    parser
        .feed_line(r#"{"type":"thread.started","thread_id":"th-1"}"#);
    parser
        .feed_line(r#"{"type":"item.started","item":{"id":"cmd1","type":"command_execution","tool_name":"bash","status":"in_progress","input":{"command":"ls"}}}"#);
    let call = events
        .lock()
        .unwrap()
        .iter()
        .find_map(|e| match e {
            SemanticEvent::ToolCall { extra, .. } => Some(extra.clone()),
            _ => None,
        })
        .expect("ToolCall emitted");
    assert_eq!(
        call.get("status").and_then(|v| v.as_str()),
        Some("in_progress"),
        "status from item.started must be copied to ToolCall.extra: got {call:?}"
    );
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
            assert_eq!(*provider, Provider::Codex);
            assert_eq!(kind, "");
            assert_eq!(payload.get("payload"), Some(&json!({"k": 1})));
        }
        other => panic!("expected ProviderExtension, got {other:?}"),
    }
}

#[test]
fn tool_input_string_fallback_parses_without_panic() {
    let (events, mut parser) = new_parser();
    parser
        .feed_line(r#"{"type":"item.tool_use","name":"bash","input":"ls -la"}"#);
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
fn truncated_json_line_emits_warning_and_continues() {
    let (events, mut parser) = new_parser();
    parser.feed_line(r#"{"type":"turn.started"#);
    parser.feed_line(r#"{"type":"turn.started"}"#);
    let ks = kinds(&events.lock().unwrap());
    assert_eq!(ks, vec!["warning", "turn_start"]);
}
