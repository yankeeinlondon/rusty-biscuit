use super::*;
use biscuit_terminal::discovery::detection::ColorDepth;
use serde_json::json;
use std::sync::Mutex as StdMutex;

fn make_sink(
    captured_lines: Arc<StdMutex<Vec<String>>>,
    captured_dispatch: Arc<StdMutex<Vec<(AgenticEvent, String)>>>,
) -> LiveSemanticSink {
    make_sink_for_provider(
        Provider::Claude,
        Verbosity::Normal,
        captured_lines,
        captured_dispatch,
    )
}

fn make_sink_for_provider(
    provider: Provider,
    verbosity: Verbosity,
    captured_lines: Arc<StdMutex<Vec<String>>>,
    captured_dispatch: Arc<StdMutex<Vec<(AgenticEvent, String)>>>,
) -> LiveSemanticSink {
    let dispatch = {
        let captured = captured_dispatch.clone();
        Box::new(move |event: AgenticEvent, meta: DispatchEventMeta| {
            captured
                .lock()
                .unwrap()
                .push((event, meta.tool_name.unwrap_or_default()));
        })
    };
    let emit = {
        let lines = captured_lines.clone();
        Box::new(move |line: &str| {
            lines.lock().unwrap().push(line.to_string());
        })
    };
    let mut sink = LiveSemanticSink::new(
        provider,
        EnvironmentContext::default(),
        Path::new("/tmp"),
        verbosity,
        Arc::new(Mutex::new(StructuredSummaryDetails::default())),
        dispatch,
        emit,
    );
    sink.terminal = Terminal::builder()
        .is_tty(true)
        .color_depth(ColorDepth::TrueColor)
        .osc_link_support(true)
        .build();
    sink
}

#[test]
fn tool_call_renders_arrow_right_prefix() {
    let lines = Arc::new(StdMutex::new(Vec::new()));
    let dispatched = Arc::new(StdMutex::new(Vec::new()));
    let mut sink = make_sink(lines.clone(), dispatched.clone());
    sink.on_semantic_event(SemanticEvent::ToolCall {
        name: Some("bash".into()),
        id: Some("t1".into()),
        input: Some(json!({"command": "ls"})),
        extra: json!({}),
    });
    let rendered = lines.lock().unwrap().join("\n");
    assert!(rendered.contains('\u{2192}'), "expected → in {rendered:?}");
    assert!(rendered.contains("Bash"));
    assert!(rendered.contains("ls"));
}

#[test]
fn tool_call_renders_with_parentheses_format() {
    let lines = Arc::new(StdMutex::new(Vec::new()));
    let dispatched = Arc::new(StdMutex::new(Vec::new()));
    let mut sink = make_sink(lines.clone(), dispatched.clone());
    sink.on_semantic_event(SemanticEvent::ToolCall {
        name: Some("Bash".into()),
        id: Some("t1".into()),
        input: Some(json!({"command": "ls -la"})),
        extra: json!({}),
    });
    let rendered = lines.lock().unwrap().join("\n");
    assert!(
        rendered.contains("Bash(") && rendered.contains(")"),
        "tool call must render as Name(summary) with parentheses: {rendered:?}"
    );
    assert!(
        !rendered.contains(" \u{00b7} "),
        "tool call must no longer use the `·` separator: {rendered:?}"
    );
    // Shell name gets prepended to the command inside the parens.
    assert!(
        rendered.contains("bash ls -la"),
        "summary inside parens must include prepended shell name: {rendered:?}"
    );
}

#[test]
fn tool_result_renders_arrow_left_prefix_with_error_status() {
    let lines = Arc::new(StdMutex::new(Vec::new()));
    let dispatched = Arc::new(StdMutex::new(Vec::new()));
    let mut sink = make_sink(lines.clone(), dispatched.clone());
    sink.on_semantic_event(SemanticEvent::ToolResult {
        name: Some("bash".into()),
        id: Some("t1".into()),
        status: Some("failure".into()),
        exit_code: None,
        output: None,
        extra: json!({}),
    });
    let rendered = lines.lock().unwrap().join("\n");
    assert!(rendered.contains('\u{2190}'));
    assert!(rendered.contains("Bash"));
    // Status wins over summary/exit code; "failure" maps to ToolStatus::Error
    // which renders as "error" in the dim slot.
    assert!(
        rendered.contains("error"),
        "expected error status word: {rendered:?}"
    );
    assert!(
        !rendered.contains("<red>"),
        "prose markup must be interpreted, not leaked as literal text: {rendered:?}"
    );
    assert!(
        !rendered.contains("<b>"),
        "prose markup must be interpreted, not leaked as literal text: {rendered:?}"
    );
    assert!(
        rendered.contains("\u{1b}[31m"),
        "error-path rendering must emit red ANSI escape: {rendered:?}"
    );
    assert!(
        !rendered.contains("exit 1"),
        "exit_code must not render when status is present; status wins: {rendered:?}"
    );
}

#[test]
fn tool_call_with_markup_looking_summary_is_not_interpreted() {
    let lines = Arc::new(StdMutex::new(Vec::new()));
    let dispatched = Arc::new(StdMutex::new(Vec::new()));
    let mut sink = make_sink(lines.clone(), dispatched.clone());
    sink.on_semantic_event(SemanticEvent::ToolCall {
        name: Some("Bash".into()),
        id: None,
        input: Some(json!({"command": "echo '<b>hi</b>'"})),
        extra: json!({}),
    });
    let rendered = lines.lock().unwrap().join("\n");
    // User input containing markup must appear verbatim — Status::new
    // does NOT interpret prose markup on the summary path.
    assert!(
        rendered.contains("<b>hi</b>"),
        "user input with prose tokens must render literally: {rendered:?}"
    );
}

#[test]
fn tool_result_success_status_co_renders_with_input_summary() {
    let lines = Arc::new(StdMutex::new(Vec::new()));
    let dispatched = Arc::new(StdMutex::new(Vec::new()));
    let mut sink = make_sink(lines.clone(), dispatched.clone());
    sink.on_semantic_event(SemanticEvent::ToolResult {
        name: Some("bash".into()),
        id: Some("t1".into()),
        status: Some("completed".into()),
        exit_code: None,
        output: None,
        extra: json!({ "input": { "command": "ls -la" } }),
    });
    let rendered = lines.lock().unwrap().join("\n");
    assert!(rendered.contains('\u{2190}'), "expected ← arrow");
    assert!(rendered.contains("Bash"), "expected humanized tool name");
    // Per the 2026-04-18 contract, a successful incoming tool result
    // renders `status + summary` together when a summary derived from
    // the cached input is available — so the user can see what command
    // succeeded, not just that something succeeded.
    assert!(
        rendered.contains("successful"),
        "expected mapped status word 'successful': {rendered:?}"
    );
    assert!(
        rendered.contains("bash ls -la"),
        "expected shell summary `bash ls -la` to co-render with status: {rendered:?}"
    );
}

#[test]
fn subagent_start_and_stop_use_arrows() {
    let lines = Arc::new(StdMutex::new(Vec::new()));
    let dispatched = Arc::new(StdMutex::new(Vec::new()));
    let mut sink = make_sink(lines.clone(), dispatched.clone());
    sink.on_semantic_event(SemanticEvent::SubagentStart {
        name: Some("researcher".into()),
        id: Some("sa1".into()),
        extra: json!({}),
    });
    sink.on_semantic_event(SemanticEvent::SubagentStop {
        name: Some("researcher".into()),
        id: Some("sa1".into()),
        status: Some("success".into()),
        extra: json!({}),
    });
    let collected = lines.lock().unwrap().clone();
    assert!(collected[0].contains('\u{2192}') && collected[0].contains("researcher"));
    assert!(collected[1].contains('\u{2190}') && collected[1].contains("researcher"));
}

#[test]
fn provider_extension_formatter_uses_summary_extraction_order() {
    let lines = Arc::new(StdMutex::new(Vec::new()));
    let dispatched = Arc::new(StdMutex::new(Vec::new()));
    let mut sink = make_sink(lines.clone(), dispatched.clone());
    sink.on_semantic_event(SemanticEvent::ProviderExtension {
        provider: Provider::Codex,
        kind: "item.updated".into(),
        payload: json!({"message": "still working"}),
    });
    let rendered = lines.lock().unwrap().join("\n");
    assert!(rendered.contains("codex/item.updated"));
    assert!(rendered.contains("still working"));
}

#[test]
fn warning_renders_and_dispatches_as_notification() {
    let lines = Arc::new(StdMutex::new(Vec::new()));
    let dispatched = Arc::new(StdMutex::new(Vec::new()));
    let mut sink = make_sink(lines.clone(), dispatched.clone());
    sink.on_semantic_event(SemanticEvent::Warning {
        message: "rate limited".into(),
        extra: json!({}),
    });
    let rendered = lines.lock().unwrap().join("\n");
    assert!(rendered.contains("rate limited"));
    let dispatches = dispatched.lock().unwrap().clone();
    assert_eq!(dispatches[0].0, AgenticEvent::Notification);
}

#[test]
fn terminal_error_dispatches_turn_error() {
    let lines = Arc::new(StdMutex::new(Vec::new()));
    let dispatched = Arc::new(StdMutex::new(Vec::new()));
    let mut sink = make_sink(lines.clone(), dispatched.clone());
    sink.on_semantic_event(SemanticEvent::Error {
        message: "billing".into(),
        terminal: true,
        kind: SemanticErrorKind::ApiRemote,
        extra: json!({}),
    });
    let dispatches = dispatched.lock().unwrap().clone();
    assert_eq!(dispatches[0].0, AgenticEvent::TurnError);
}

#[test]
fn error_event_renders_blockquote_with_red_border() {
    let lines = Arc::new(StdMutex::new(Vec::new()));
    let dispatched = Arc::new(StdMutex::new(Vec::new()));
    let mut sink = make_sink(lines.clone(), dispatched.clone());
    sink.on_semantic_event(SemanticEvent::Error {
        message: "Quota exceeded".into(),
        terminal: true,
        kind: SemanticErrorKind::ApiRemote,
        extra: json!({}),
    });
    let rendered = lines.lock().unwrap().join("\n");
    assert!(
        rendered.contains("API Error"),
        "expected API Error label, got: {rendered:?}"
    );
    assert!(
        rendered.contains("Quota exceeded"),
        "expected message text, got: {rendered:?}"
    );
    assert!(
        rendered.contains('\u{2503}'),
        "expected centered block-quote border (┃), got: {rendered:?}"
    );
}

#[test]
fn interrupted_error_renders_blockquote_with_interrupted_label() {
    let lines = Arc::new(StdMutex::new(Vec::new()));
    let dispatched = Arc::new(StdMutex::new(Vec::new()));
    let mut sink = make_sink(lines.clone(), dispatched.clone());
    sink.on_semantic_event(SemanticEvent::Error {
        message: "User cancelled".into(),
        terminal: true,
        kind: SemanticErrorKind::Interrupted,
        extra: json!({}),
    });
    let rendered = lines.lock().unwrap().join("\n");
    assert!(
        rendered.contains("Interrupted"),
        "expected Interrupted label, got: {rendered:?}"
    );
    assert!(rendered.contains('\u{2503}'));
}

#[test]
fn configuration_error_renders_blockquote_with_configuration_label() {
    let lines = Arc::new(StdMutex::new(Vec::new()));
    let dispatched = Arc::new(StdMutex::new(Vec::new()));
    let mut sink = make_sink(lines.clone(), dispatched.clone());
    sink.on_semantic_event(SemanticEvent::Error {
        message: "Bad API key".into(),
        terminal: true,
        kind: SemanticErrorKind::Configuration,
        extra: json!({}),
    });
    let rendered = lines.lock().unwrap().join("\n");
    assert!(
        rendered.contains("Configuration Error"),
        "expected Configuration Error label, got: {rendered:?}"
    );
    assert!(rendered.contains('\u{2503}'));
}

#[test]
fn error_kind_presentation_returns_expected_labels() {
    assert_eq!(
        error_kind_presentation(SemanticErrorKind::Configuration).0,
        "Configuration Error"
    );
    assert_eq!(
        error_kind_presentation(SemanticErrorKind::AgentNative).0,
        "Agent Error"
    );
    assert_eq!(
        error_kind_presentation(SemanticErrorKind::ApiRemote).0,
        "API Error"
    );
    assert_eq!(
        error_kind_presentation(SemanticErrorKind::Interrupted).0,
        "Interrupted"
    );
    assert_eq!(
        error_kind_presentation(SemanticErrorKind::Unknown).0,
        "Error"
    );
}

#[test]
fn silent_verbosity_suppresses_stderr_but_not_dispatch() {
    let lines = Arc::new(StdMutex::new(Vec::new()));
    let dispatched = Arc::new(StdMutex::new(Vec::new()));
    let mut sink = make_sink(lines.clone(), dispatched.clone());
    sink.verbosity = Verbosity::Silent;
    sink.on_semantic_event(SemanticEvent::Warning {
        message: "rate limited".into(),
        extra: json!({}),
    });
    assert!(lines.lock().unwrap().is_empty());
    assert!(!dispatched.lock().unwrap().is_empty());
}

#[test]
fn session_start_updates_cached_state_and_emits_session_header() {
    let lines = Arc::new(StdMutex::new(Vec::new()));
    let dispatched = Arc::new(StdMutex::new(Vec::new()));
    let mut sink = make_sink(lines.clone(), dispatched.clone());
    sink.on_semantic_event(SemanticEvent::SessionStart {
        session_id: Some("s1".into()),
        model: Some("claude".into()),
        extra: json!({"api_key_source": "none"}),
    });
    assert_eq!(sink.session_id.as_deref(), Some("s1"));
    assert_eq!(sink.model.as_deref(), Some("claude"));
    assert_eq!(sink.renderer.api_key_source(), Some("none"));
    // Task 3.2 routes the session header through the section-aware
    // emit path so the `emit_stderr` closure captures it. The header
    // line must appear; a trailing blank is allowed but not required.
    let collected = lines.lock().unwrap().clone();
    assert!(
        collected.iter().any(|l| l.contains("s1")),
        "session id must appear in stderr capture: {collected:?}"
    );
    let dispatches = dispatched.lock().unwrap().clone();
    assert_eq!(dispatches[0].0, AgenticEvent::SessionStart);
}
