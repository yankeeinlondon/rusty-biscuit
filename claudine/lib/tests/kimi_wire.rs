//! Fixture-replay integration tests for Kimi Code's `--wire` JSON-RPC
//! protocol. Each test loads a captured wire-mode session
//! (`claudine/lib/src/stream/protocol/fixtures/kimi/*.jsonl`) through the
//! `Provider::KimiCode` semantic parser and asserts the resulting event
//! kinds and summary fields.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use claudine::provider::Provider;
use claudine::stream::parser::SemanticStreamParser;
use claudine::stream::semantic::{SemanticErrorKind, SemanticEvent, SemanticEventSink};
use claudine::stream::{ParserConfig, create_semantic_parser};

struct Recording {
    events: Arc<Mutex<Vec<SemanticEvent>>>,
}

impl SemanticEventSink for Recording {
    fn on_semantic_event(&mut self, event: SemanticEvent) {
        self.events.lock().unwrap().push(event);
    }
}

fn fixture_path(name: &str) -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("src/stream/protocol/fixtures/kimi");
    p.push(name);
    p
}

fn load_fixture(name: &str) -> Vec<String> {
    let raw = std::fs::read_to_string(fixture_path(name))
        .unwrap_or_else(|e| panic!("read fixture {name}: {e}"));
    raw.lines()
        .filter(|line| !line.trim().is_empty())
        .map(str::to_owned)
        .collect()
}

fn replay_fixture(name: &str) -> (Vec<SemanticEvent>, Box<dyn SemanticStreamParser>) {
    let events = Arc::new(Mutex::new(Vec::new()));
    let sink = Recording {
        events: events.clone(),
    };
    let mut parser = create_semantic_parser(Provider::KimiCode, sink, ParserConfig::default());
    for line in load_fixture(name) {
        parser.feed_line(&line);
    }
    let collected = events.lock().unwrap().clone();
    (collected, parser)
}

fn count_kind(events: &[SemanticEvent], kind: &str) -> usize {
    events.iter().filter(|e| e.kind_str() == kind).count()
}

fn assistant_text(events: &[SemanticEvent]) -> String {
    let mut buf = String::new();
    for event in events {
        if let SemanticEvent::OutputText { text, .. } = event {
            buf.push_str(text);
        }
    }
    buf
}

fn reasoning_text(events: &[SemanticEvent]) -> String {
    let mut buf = String::new();
    for event in events {
        if let SemanticEvent::Reasoning { text, .. } = event {
            buf.push_str(text);
        }
    }
    buf
}

#[test]
fn wire_greet_fixture_replays_end_to_end() {
    let (events, parser) = replay_fixture("wire-greet.jsonl");
    assert!(!events.is_empty());

    // Initialize → SessionStart
    assert_eq!(count_kind(&events, "session_start"), 1);
    // TurnBegin → TurnStart
    assert_eq!(count_kind(&events, "turn_start"), 1);
    // TurnEnd → TurnComplete
    assert_eq!(count_kind(&events, "turn_complete"), 1);
    // Reasoning ContentPart deltas mapped through SemanticEvent::Reasoning
    assert!(count_kind(&events, "reasoning") > 0);
    // Assistant text streamed as OutputText deltas
    assert!(count_kind(&events, "output_text") > 0);
    // Prompt success response → Info { kind: prompt_status }
    let prompt_status = events.iter().any(|e| match e {
        SemanticEvent::Info { extra, .. } => {
            extra.get("kind").and_then(|v| v.as_str()) == Some("prompt_status")
        }
        _ => false,
    });
    assert!(prompt_status, "expected a prompt_status info event");

    let assistant = assistant_text(&events);
    assert!(
        assistant.starts_with("Hi Bob!"),
        "assistant text did not start with greeting; got {assistant:?}"
    );

    let reasoning = reasoning_text(&events);
    assert!(reasoning.contains("user"));

    let summary = parser.finish(0);
    assert_eq!(summary.provider, Provider::KimiCode);
    assert_eq!(summary.provider_status.as_deref(), Some("finished"));
    assert!(!summary.is_error);
    // TurnEnd ensures `assistant_text` ends with `\n` so subsequent assistant
    // messages don't run together — strip the trailing newline before
    // comparing to the streamed delta concatenation.
    assert_eq!(summary.assistant_text.trim_end_matches('\n'), assistant);
    assert!(summary.token_usage.is_some());
    let context = summary.context_usage.expect("context_usage");
    let pct = context.percent.expect("percent");
    assert!(pct > 0.0 && pct < 80.0);
    assert_eq!(summary.num_turns, Some(1));
    assert_eq!(summary.tool_calls, None);
}

#[test]
fn wire_tool_shell_fixture_renders_tool_lifecycle() {
    let (events, parser) = replay_fixture("wire-tool-shell.jsonl");

    let tool_calls = count_kind(&events, "tool_call");
    let tool_results = count_kind(&events, "tool_result");
    assert_eq!(tool_calls, 1, "kinds = {:?}", kind_summary(&events));
    assert_eq!(tool_results, 1, "kinds = {:?}", kind_summary(&events));

    // The single ApprovalRequest must surface as auto-approved info.
    let auto_approved = events.iter().any(|e| match e {
        SemanticEvent::Info { extra, .. } => {
            extra.get("kind").and_then(|v| v.as_str()) == Some("auto_approved")
        }
        _ => false,
    });
    assert!(auto_approved, "expected auto-approved info event");

    // Decoded tool call input includes the streamed shell command.
    let tool_call_input = events
        .iter()
        .find_map(|e| match e {
            SemanticEvent::ToolCall { input, .. } => input.clone(),
            _ => None,
        })
        .expect("tool_call input");
    assert_eq!(
        tool_call_input.get("command").and_then(|v| v.as_str()),
        Some("echo hello-from-kimi")
    );

    let summary = parser.finish(0);
    assert_eq!(summary.tool_calls, Some(1));
    assert_eq!(summary.provider_status.as_deref(), Some("finished"));
    assert!(!summary.is_error);
}

#[test]
fn wire_subagent_fixture_renders_subagent_info() {
    let (events, parser) = replay_fixture("wire-subagent.jsonl");

    let subagent_count = events
        .iter()
        .filter(|e| match e {
            SemanticEvent::Info { extra, .. } => {
                extra.get("kind").and_then(|v| v.as_str()) == Some("subagent_event")
            }
            _ => false,
        })
        .count();
    assert!(subagent_count > 0, "expected subagent_event info events");

    let summary = parser.finish(0);
    assert_eq!(summary.provider, Provider::KimiCode);
}

#[test]
fn wire_cancelled_fixture_classifies_interruption() {
    let (events, parser) = replay_fixture("wire-cancelled.jsonl");

    let interruption = events.iter().any(|e| match e {
        SemanticEvent::Error { kind, terminal, .. } => {
            *kind == SemanticErrorKind::Interrupted && *terminal
        }
        _ => false,
    });
    assert!(interruption, "expected terminal Interrupted error event");

    let summary = parser.finish(130);
    assert!(summary.is_error);
    assert_eq!(summary.provider_status.as_deref(), Some("cancelled"));
}

#[test]
fn wire_auth_expired_fixture_classifies_configuration() {
    let (events, parser) = replay_fixture("wire-auth-expired.jsonl");

    let configuration = events.iter().any(|e| match e {
        SemanticEvent::Error { kind, terminal, .. } => {
            *kind == SemanticErrorKind::Configuration && *terminal
        }
        _ => false,
    });
    assert!(
        configuration,
        "expected terminal Configuration error event for auth_expired"
    );

    let summary = parser.finish(1);
    assert!(summary.is_error);
    assert_eq!(summary.error_kind.as_deref(), Some("auth_expired"));
}

#[test]
fn unknown_event_type_falls_back_to_provider_extension() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let sink = Recording {
        events: events.clone(),
    };
    let mut parser = create_semantic_parser(Provider::KimiCode, sink, ParserConfig::default());
    parser
        .feed_line(
            r#"{"jsonrpc":"2.0","method":"event","params":{"type":"BrandNewEvent","payload":{"x":1}}}"#,
        );
    let collected = events.lock().unwrap().clone();
    assert!(matches!(
        collected[0],
        SemanticEvent::ProviderExtension { .. }
    ));
}

fn kind_summary(events: &[SemanticEvent]) -> Vec<&'static str> {
    events.iter().map(|e| e.kind_str()).collect()
}
