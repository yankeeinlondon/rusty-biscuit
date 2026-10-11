//! `SemanticStreamParser::snapshot` is the summary `finish` would return, for
//! every provider, without consuming the parser or emitting an event.
//!
//! A wrapper keeps the snapshot so the result survives when the reader that
//! owns the parser is abandoned. Each stream below ends in state that only
//! finalization turns into the summary: Claude's task ledger, Gemini's
//! held-back list text, Kimi's in-flight tool call, and Antigravity's
//! diagnosis of a run that wrote no envelope.

use std::sync::{Arc, Mutex};

use claudine::provider::Provider;
use claudine::stream::semantic::{SemanticEvent, SemanticEventSink};
use claudine::stream::{ParserConfig, create_semantic_parser};

struct Counting(Arc<Mutex<usize>>);

impl SemanticEventSink for Counting {
    fn on_semantic_event(&mut self, _event: SemanticEvent) {
        *self.0.lock().unwrap() += 1;
    }
}

/// Feed `lines`, then compare the snapshot with the real finalization at
/// `exit_code`; returns the summary so a case can assert what it carries.
fn snapshot_matches_finish(
    provider: Provider,
    lines: &[&str],
    exit_code: i32,
) -> claudine::stream::summary::StreamExecutionSummary {
    let events = Arc::new(Mutex::new(0));
    let mut parser =
        create_semantic_parser(provider, Counting(events.clone()), ParserConfig::default());
    for line in lines {
        parser.feed_line(line);
    }
    let emitted = *events.lock().unwrap();

    let snapshot = parser.snapshot(exit_code);
    assert_eq!(*events.lock().unwrap(), emitted, "{provider:?}: snapshot emitted an event");
    let again = parser.snapshot(exit_code);
    assert_eq!(snapshot, again, "{provider:?}: taking a snapshot changed the parser");

    let finished = parser.finish(exit_code);
    assert_eq!(snapshot, finished, "{provider:?} at exit {exit_code}");
    finished
}

const CLAUDE_STOPPED_TASK: &[&str] = &[
    r#"{"type":"system","subtype":"init","session_id":"incident","model":"claude-opus"}"#,
    r#"{"type":"task_started","task_id":"sa_1","name":"commit-alpha"}"#,
    r#"{"type":"assistant","message":{"content":[{"type":"text","text":"All done."}]}}"#,
    r#"{"type":"task_notification","task_id":"sa_1","name":"commit-alpha","status":"stopped"}"#,
    r#"{"type":"result","subtype":"success","stop_reason":"end_turn","num_turns":1,"duration_ms":600000}"#,
];

#[test]
fn claude_snapshot_carries_the_task_ledger_verdict() {
    for exit_code in [0, 1] {
        let summary = snapshot_matches_finish(Provider::Claude, CLAUDE_STOPPED_TASK, exit_code);
        assert!(summary.is_error);
        assert_eq!(summary.error_kind.as_deref(), Some("incomplete_subagents"));
        assert_eq!(summary.session_id.as_deref(), Some("incident"));
    }
}

#[test]
fn claude_snapshot_carries_the_answer_and_usage() {
    let summary = snapshot_matches_finish(
        Provider::Claude,
        &[
            r#"{"type":"init","session_id":"s1","model":"claude-sonnet-4"}"#,
            r#"{"type":"assistant","content":[{"type":"text","text":"review-answer"}]}"#,
            r#"{"type":"result","subtype":"success","duration_ms":1000,"num_turns":1,"stop_reason":"end_turn","usage":{"input_tokens":100,"output_tokens":50}}"#,
        ],
        0,
    );
    assert!(!summary.is_error);
    assert!(summary.assistant_text.contains("review-answer"), "{summary:?}");
    assert_eq!(summary.token_usage.and_then(|usage| usage.input), Some(100));
}

#[test]
fn gemini_snapshot_counts_text_still_held_for_a_list_boundary() {
    let summary = snapshot_matches_finish(
        Provider::Gemini,
        &[
            r#"{"type":"init","session_id":"g1","model":"gemini-2.5"}"#,
            r#"{"type":"message","role":"assistant","delta":true,"content":"partial "}"#,
            r#"{"type":"message","role":"assistant","delta":true,"content":"more"}"#,
        ],
        0,
    );
    assert_eq!(summary.assistant_text, "partial more");
}

#[test]
fn kimi_snapshot_counts_an_in_flight_tool_call() {
    let summary = snapshot_matches_finish(
        Provider::KimiCode,
        &[
            r#"{"jsonrpc":"2.0","method":"event","params":{"type":"TurnBegin","payload":{"user_input":"hi"}}}"#,
            r#"{"jsonrpc":"2.0","method":"event","params":{"type":"ContentPart","payload":{"type":"text","text":"Hello from Kimi"}}}"#,
            r#"{"jsonrpc":"2.0","method":"event","params":{"type":"ToolCall","payload":{"id":"k1","function":{"name":"Shell","arguments":"{\"cmd\":\"ls\"}"}}}}"#,
        ],
        0,
    );
    assert_eq!(summary.tool_calls, Some(1));
}

#[test]
fn antigravity_snapshot_diagnoses_a_run_without_an_envelope() {
    for exit_code in [0, 1] {
        let summary = snapshot_matches_finish(
            Provider::Antigravity,
            &["Error: authentication failed or timed out"],
            exit_code,
        );
        assert!(summary.is_error);
        assert_eq!(
            summary.error_message.as_deref(),
            Some("Error: authentication failed or timed out")
        );
    }
    let summary = snapshot_matches_finish(Provider::Antigravity, &[], 3);
    assert_eq!(summary.error_message.as_deref(), Some("agy exited with code 3"));
}

#[test]
fn every_other_provider_snapshot_matches_its_finalization() {
    let cases: &[(Provider, &[&str])] = &[
        (
            Provider::Codex,
            &[
                r#"{"type":"thread.started","thread_id":"th-1"}"#,
                r#"{"type":"item.completed","item":{"id":"a1","type":"agent_message","text":"Done"}}"#,
                r#"{"type":"turn.completed","usage":{"input_tokens":100,"output_tokens":50},"duration_ms":800,"status":"completed"}"#,
            ],
        ),
        (
            Provider::OpenCode,
            &[
                r#"{"type":"step_start","sessionID":"ses_1"}"#,
                r#"{"type":"text","text":"hello"}"#,
                r#"{"type":"step_complete","usage":{"input_tokens":10,"output_tokens":5,"total_tokens":15},"cost_usd":0.001,"duration_ms":200}"#,
            ],
        ),
        (
            Provider::QwenCode,
            &[
                r#"{"type":"init","session_id":"q1","model":"qwen-coder"}"#,
                r#"{"type":"message","role":"assistant","content":[{"text":"Hello from Qwen"}]}"#,
                r#"{"type":"summary","duration_ms":3000,"token_usage":{"input_tokens":100,"output_tokens":50}}"#,
            ],
        ),
        (
            Provider::Pi,
            &[
                r#"{"type":"session","version":3,"id":"s-1","cwd":"/work"}"#,
                r#"{"type":"message_update","assistantMessageEvent":{"type":"text_delta","delta":"Pi answer"}}"#,
                r#"{"type":"message_end","message":{"stopReason":"stop","usage":{"input":10,"output":5,"totalTokens":15}}}"#,
                r#"{"type":"agent_end","willRetry":false}"#,
            ],
        ),
    ];
    for (provider, lines) in cases {
        for exit_code in [0, 2] {
            let summary = snapshot_matches_finish(*provider, lines, exit_code);
            assert!(!summary.assistant_text.is_empty(), "{provider:?}: {summary:?}");
        }
    }
}
