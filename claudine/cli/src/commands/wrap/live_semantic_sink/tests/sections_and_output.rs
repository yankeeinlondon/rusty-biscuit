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
fn full_run_has_no_two_consecutive_blank_lines() {
    let lines = Arc::new(StdMutex::new(Vec::new()));
    let dispatched = Arc::new(StdMutex::new(Vec::new()));
    let mut sink = make_sink(lines.clone(), dispatched.clone());
    // Synthetic sequence representing every section transition.
    sink.on_semantic_event(SemanticEvent::SessionStart {
        session_id: Some("s1".into()),
        model: Some("claude-opus-4-6".into()),
        extra: json!({}),
    });
    sink.on_semantic_event(SemanticEvent::Reasoning {
        text: "thinking…".into(),
        extra: json!({}),
    });
    sink.on_semantic_event(SemanticEvent::ToolCall {
        name: Some("Bash".into()),
        id: None,
        input: Some(json!({"command": "ls"})),
        extra: json!({}),
    });
    sink.on_semantic_event(SemanticEvent::ToolResult {
        name: Some("Bash".into()),
        id: None,
        status: Some("success".into()),
        exit_code: Some(0),
        output: None,
        extra: json!({}),
    });
    sink.on_semantic_event(SemanticEvent::OutputText {
        text: "answer".into(),
        extra: json!({}),
    });
    sink.on_semantic_event(SemanticEvent::TurnComplete {
        provider_status: Some("ok".into()),
        token_usage: None,
        cost_usd: None,
        duration_ms: Some(100),
        extra: json!({}),
    });
    let collected = lines.lock().unwrap().clone();
    let mut prev_blank = false;
    for line in &collected {
        let is_blank = line.trim().is_empty();
        if is_blank && prev_blank {
            panic!("two consecutive blank lines in {collected:?}");
        }
        prev_blank = is_blank;
    }
}

/// Combined section golden test: feed every section transition through
/// the sink and verify no two consecutive blank lines exist in the
/// combined stderr output.
#[test]
fn combined_sections_have_no_consecutive_blanks() {
    let lines = Arc::new(StdMutex::new(Vec::new()));
    let dispatched = Arc::new(StdMutex::new(Vec::new()));
    let mut sink = make_sink(lines.clone(), dispatched.clone());

    // SessionStart -> SessionAndModel section
    sink.on_semantic_event(SemanticEvent::SessionStart {
        session_id: Some("s-combined".into()),
        model: Some("test-model".into()),
        extra: json!({}),
    });
    // Reasoning -> Thinking section
    sink.on_semantic_event(SemanticEvent::Reasoning {
        text: "pondering deeply".into(),
        extra: json!({}),
    });
    // ToolCall -> ToolUseAndEvents section
    sink.on_semantic_event(SemanticEvent::ToolCall {
        name: Some("Bash".into()),
        id: Some("t1".into()),
        input: Some(json!({"command": "echo hello"})),
        extra: json!({}),
    });
    // ToolResult -> stays in ToolUseAndEvents
    sink.on_semantic_event(SemanticEvent::ToolResult {
        name: Some("Bash".into()),
        id: Some("t1".into()),
        status: Some("success".into()),
        exit_code: Some(0),
        output: None,
        extra: json!({}),
    });
    // OutputText -> does not emit to stderr (goes to stdout callback)
    sink.on_semantic_event(SemanticEvent::OutputText {
        text: "the answer".into(),
        extra: json!({}),
    });
    // TurnComplete -> envelope-only, no section emission
    sink.on_semantic_event(SemanticEvent::TurnComplete {
        provider_status: Some("ok".into()),
        token_usage: None,
        cost_usd: None,
        duration_ms: Some(500),
        extra: json!({}),
    });

    let collected = lines.lock().unwrap().clone();
    // Basic sanity: the sink must have emitted something.
    assert!(!collected.is_empty(), "sink must emit lines: {collected:?}");

    // Core invariant: no two consecutive blank lines.
    let mut prev_blank = false;
    for line in &collected {
        let is_blank = line.trim().is_empty();
        if is_blank && prev_blank {
            panic!("two consecutive blank lines in combined section output:\n{collected:#?}");
        }
        prev_blank = is_blank;
    }
}

#[test]
fn tool_call_records_tool_name_in_summary_details() {
    let lines = Arc::new(StdMutex::new(Vec::new()));
    let dispatched = Arc::new(StdMutex::new(Vec::new()));
    let details = Arc::new(Mutex::new(StructuredSummaryDetails::default()));
    let sink_details = details.clone();
    let dispatch = {
        let captured = dispatched.clone();
        Box::new(move |event: AgenticEvent, meta: DispatchEventMeta| {
            captured
                .lock()
                .unwrap()
                .push((event, meta.tool_name.unwrap_or_default()));
        })
    };
    let emit = {
        let lines = lines.clone();
        Box::new(move |line: &str| lines.lock().unwrap().push(line.to_string()))
    };
    let mut sink = LiveSemanticSink::new(
        Provider::Claude,
        EnvironmentContext::default(),
        Path::new("/tmp"),
        Verbosity::Normal,
        sink_details,
        dispatch,
        emit,
    );
    sink.on_semantic_event(SemanticEvent::ToolCall {
        name: Some("bash".into()),
        id: None,
        input: None,
        extra: json!({}),
    });
    let names = details.lock().unwrap().tool_names.clone();
    assert_eq!(names, vec!["bash".to_string()]);
}

#[test]
fn final_response_keeps_only_text_after_last_tool_call() {
    // The final-response accumulator must drop interstitial narration
    // emitted between tool calls and retain only the output text that
    // follows the LAST tool call — the agent's closing answer that
    // `inline-compose` writes into the document body.
    let lines = Arc::new(StdMutex::new(Vec::new()));
    let details = Arc::new(Mutex::new(StructuredSummaryDetails::default()));
    let sink_details = details.clone();
    let dispatch = Box::new(|_event: AgenticEvent, _meta: DispatchEventMeta| {});
    let emit = {
        let lines = lines.clone();
        Box::new(move |line: &str| lines.lock().unwrap().push(line.to_string()))
    };
    let mut sink = LiveSemanticSink::new(
        Provider::Claude,
        EnvironmentContext::default(),
        Path::new("/tmp"),
        Verbosity::Normal,
        sink_details,
        dispatch,
        emit,
    );

    let tool_call = |name: &str| SemanticEvent::ToolCall {
        name: Some(name.to_string()),
        id: None,
        input: None,
        extra: json!({}),
    };
    let output = |text: &str| SemanticEvent::OutputText {
        text: text.to_string(),
        extra: json!({}),
    };

    sink.on_semantic_event(output("Let me read the research documents. "));
    sink.on_semantic_event(tool_call("read_file"));
    sink.on_semantic_event(output("Now let me write the draft. "));
    sink.on_semantic_event(tool_call("write_file"));
    sink.on_semantic_event(output("# Final Body\n"));
    sink.on_semantic_event(output("This is the closing answer."));

    let final_response = details.lock().unwrap().final_response.clone();
    assert_eq!(
        final_response, "# Final Body\nThis is the closing answer.",
        "only the output text after the last tool call should remain"
    );
}

#[test]
fn output_text_flows_through_external_renderer() {
    let lines = Arc::new(StdMutex::new(Vec::new()));
    let dispatched = Arc::new(StdMutex::new(Vec::new()));
    let rendered_text = Arc::new(StdMutex::new(String::new()));

    let output_cb = {
        let buf = rendered_text.clone();
        Box::new(move |text: &str| {
            buf.lock().unwrap().push_str(text);
        })
    };

    let mut sink = make_sink(lines.clone(), dispatched).with_output_text_sink(output_cb);

    sink.on_semantic_event(SemanticEvent::OutputText {
        text: "hello ".into(),
        extra: json!({}),
    });
    sink.on_semantic_event(SemanticEvent::Reasoning {
        text: "pondering".into(),
        extra: json!({}),
    });
    sink.on_semantic_event(SemanticEvent::OutputText {
        text: "world".into(),
        extra: json!({}),
    });

    assert_eq!(*rendered_text.lock().unwrap(), "hello world");
    // Reasoning is now rendered directly by LiveSemanticSink via
    // render_thinking_block into the stderr lines (not through an
    // external callback).
    let captured = lines.lock().unwrap().join("\n");
    assert!(
        captured.contains("pondering"),
        "reasoning text must appear in stderr: {captured:?}"
    );
}

#[test]
fn first_output_text_inserts_section_separator_between_tool_and_final_stdout() {
    let lines = Arc::new(StdMutex::new(Vec::new()));
    let dispatched = Arc::new(StdMutex::new(Vec::new()));
    let rendered_text = Arc::new(StdMutex::new(String::new()));

    let output_cb = {
        let buf = rendered_text.clone();
        Box::new(move |text: &str| {
            buf.lock().unwrap().push_str(text);
        })
    };

    let mut sink = make_sink(lines.clone(), dispatched).with_output_text_sink(output_cb);

    sink.on_semantic_event(SemanticEvent::ToolResult {
        name: Some("bash".into()),
        id: None,
        status: Some("success".into()),
        exit_code: None,
        output: None,
        extra: json!({}),
    });
    sink.on_semantic_event(SemanticEvent::OutputText {
        text: "chunk-a ".into(),
        extra: json!({}),
    });
    sink.on_semantic_event(SemanticEvent::OutputText {
        text: "chunk-b".into(),
        extra: json!({}),
    });

    let captured = lines.lock().unwrap();
    // Exactly one blank separator between the tool render (stderr)
    // and the stdout-bound assistant text.
    let blanks = captured.iter().filter(|l| l.is_empty()).count();
    assert_eq!(
        blanks, 1,
        "expected exactly one section-separator blank: {captured:?}"
    );
    // The last stderr line is that separator (the OutputText payload
    // does not go through `emit_stderr`).
    assert!(captured.last().is_some_and(|l| l.is_empty()));
    // And both OutputText chunks still reach the external renderer.
    assert_eq!(*rendered_text.lock().unwrap(), "chunk-a chunk-b");
}

#[test]
fn blank_line_terminated_output_text_suppresses_next_section_separator() {
    // Regression: OpenCode (and any provider that emits assistant text
    // terminated with `\n\n`) previously produced two visually blank
    // rows between a stdout paragraph and the next stderr tool line —
    // one from the text's own trailing blank, another from the section
    // transition separator. When the stdout already ends on a blank
    // line the separator is redundant and must be suppressed.
    let lines = Arc::new(StdMutex::new(Vec::new()));
    let dispatched = Arc::new(StdMutex::new(Vec::new()));
    let rendered_text = Arc::new(StdMutex::new(String::new()));

    let output_cb = {
        let buf = rendered_text.clone();
        Box::new(move |text: &str| {
            buf.lock().unwrap().push_str(text);
        })
    };

    let mut sink = make_sink(lines.clone(), dispatched).with_output_text_sink(output_cb);

    sink.on_semantic_event(SemanticEvent::ToolResult {
        name: Some("bash".into()),
        id: None,
        status: Some("success".into()),
        exit_code: Some(0),
        output: None,
        extra: json!({}),
    });
    // Assistant prose ending with `\n\n` — the stdout stream now ends
    // on a blank line.
    sink.on_semantic_event(SemanticEvent::OutputText {
        text: "I'll start by reading the file.\n\n".into(),
        extra: json!({}),
    });
    // Next tool result triggers a FinalStdout → ToolUseAndEvents
    // transition. The classifier wants a separator but the stdout
    // already shows a blank line, so the sink must skip it.
    sink.on_semantic_event(SemanticEvent::ToolResult {
        name: Some("bash".into()),
        id: None,
        status: Some("success".into()),
        exit_code: Some(0),
        output: None,
        extra: json!({}),
    });

    let captured = lines.lock().unwrap().clone();
    let blanks = captured.iter().filter(|l| l.is_empty()).count();
    assert_eq!(
        blanks, 1,
        "exactly one separator blank (the one before the stdout text); \
         the transition back to stderr must reuse the stdout trailing blank: {captured:?}"
    );
}

#[test]
fn trailing_newline_accumulates_across_chunks() {
    // Provider streams that split `\n\n` into separate events must
    // still accumulate to the "ends on a blank line" state, so the
    // next section transition suppresses the redundant separator.
    let lines = Arc::new(StdMutex::new(Vec::new()));
    let dispatched = Arc::new(StdMutex::new(Vec::new()));
    let rendered_text = Arc::new(StdMutex::new(String::new()));

    let output_cb = {
        let buf = rendered_text.clone();
        Box::new(move |text: &str| {
            buf.lock().unwrap().push_str(text);
        })
    };

    let mut sink = make_sink(lines.clone(), dispatched).with_output_text_sink(output_cb);

    sink.on_semantic_event(SemanticEvent::ToolResult {
        name: Some("bash".into()),
        id: None,
        status: Some("success".into()),
        exit_code: Some(0),
        output: None,
        extra: json!({}),
    });
    sink.on_semantic_event(SemanticEvent::OutputText {
        text: "paragraph.\n".into(),
        extra: json!({}),
    });
    sink.on_semantic_event(SemanticEvent::OutputText {
        text: "\n".into(),
        extra: json!({}),
    });
    sink.on_semantic_event(SemanticEvent::ToolResult {
        name: Some("bash".into()),
        id: None,
        status: Some("success".into()),
        exit_code: Some(0),
        output: None,
        extra: json!({}),
    });

    let captured = lines.lock().unwrap().clone();
    let blanks = captured.iter().filter(|l| l.is_empty()).count();
    assert_eq!(
        blanks, 1,
        "chunked `\\n` + `\\n` must accumulate to a blank-line trailing \
         state and suppress the next section separator: {captured:?}"
    );
}

#[test]
fn single_trailing_newline_does_not_suppress_separator() {
    // A single `\n` ends the line but does not produce a blank line,
    // so the section separator is still required to visually separate
    // stdout prose from the next stderr event.
    let lines = Arc::new(StdMutex::new(Vec::new()));
    let dispatched = Arc::new(StdMutex::new(Vec::new()));
    let rendered_text = Arc::new(StdMutex::new(String::new()));

    let output_cb = {
        let buf = rendered_text.clone();
        Box::new(move |text: &str| {
            buf.lock().unwrap().push_str(text);
        })
    };

    let mut sink = make_sink(lines.clone(), dispatched).with_output_text_sink(output_cb);

    sink.on_semantic_event(SemanticEvent::ToolResult {
        name: Some("bash".into()),
        id: None,
        status: Some("success".into()),
        exit_code: Some(0),
        output: None,
        extra: json!({}),
    });
    sink.on_semantic_event(SemanticEvent::OutputText {
        text: "single trailing newline.\n".into(),
        extra: json!({}),
    });
    sink.on_semantic_event(SemanticEvent::ToolResult {
        name: Some("bash".into()),
        id: None,
        status: Some("success".into()),
        exit_code: Some(0),
        output: None,
        extra: json!({}),
    });

    let captured = lines.lock().unwrap().clone();
    let blanks = captured.iter().filter(|l| l.is_empty()).count();
    assert_eq!(
        blanks, 2,
        "one separator into FinalStdout and one back into ToolUseAndEvents: {captured:?}"
    );
}

#[test]
fn output_text_starting_with_newline_suppresses_separator() {
    // When the OutputText itself starts with a \n, the text provides
    // its own visual blank line. The section-transition separator must
    // be suppressed to avoid double-blanking.
    let lines = Arc::new(StdMutex::new(Vec::new()));
    let dispatched = Arc::new(StdMutex::new(Vec::new()));
    let rendered_text = Arc::new(StdMutex::new(String::new()));

    let output_cb = {
        let buf = rendered_text.clone();
        Box::new(move |text: &str| {
            buf.lock().unwrap().push_str(text);
        })
    };

    let mut sink = make_sink(lines.clone(), dispatched).with_output_text_sink(output_cb);

    sink.on_semantic_event(SemanticEvent::ToolResult {
        name: Some("bash".into()),
        id: None,
        status: Some("success".into()),
        exit_code: Some(0),
        output: None,
        extra: json!({}),
    });
    // Text starting with \n provides its own visual break.
    sink.on_semantic_event(SemanticEvent::OutputText {
        text: "\nanswer".into(),
        extra: json!({}),
    });
    sink.on_semantic_event(SemanticEvent::ToolResult {
        name: Some("bash".into()),
        id: None,
        status: Some("success".into()),
        exit_code: Some(0),
        output: None,
        extra: json!({}),
    });

    let captured = lines.lock().unwrap().clone();
    // The separator INTO FinalStdout is suppressed (text starts with \n).
    // The separator BACK into ToolUseAndEvents should be emitted (stdout
    // trailing newlines = 0, "answer" has no trailing \n).
    let blanks = captured.iter().filter(|l| l.is_empty()).count();
    assert_eq!(
        blanks, 1,
        "separator into stdout suppressed (text starts with \\n),              separator back to stderr emitted: {captured:?}"
    );
}

#[test]
fn output_text_pure_newlines_suppress_both_separators() {
    // When OutputText is purely "\n\n" (no visible content), both
    // separators (into and out of FinalStdout) must be suppressed to
    // prevent triple-blanking.
    let lines = Arc::new(StdMutex::new(Vec::new()));
    let dispatched = Arc::new(StdMutex::new(Vec::new()));
    let rendered_text = Arc::new(StdMutex::new(String::new()));

    let output_cb = {
        let buf = rendered_text.clone();
        Box::new(move |text: &str| {
            buf.lock().unwrap().push_str(text);
        })
    };

    let mut sink = make_sink(lines.clone(), dispatched).with_output_text_sink(output_cb);

    sink.on_semantic_event(SemanticEvent::ToolResult {
        name: Some("bash".into()),
        id: None,
        status: Some("success".into()),
        exit_code: Some(0),
        output: None,
        extra: json!({}),
    });
    sink.on_semantic_event(SemanticEvent::OutputText {
        text: "\n\n".into(),
        extra: json!({}),
    });
    sink.on_semantic_event(SemanticEvent::ToolResult {
        name: Some("bash".into()),
        id: None,
        status: Some("success".into()),
        exit_code: Some(0),
        output: None,
        extra: json!({}),
    });

    let captured = lines.lock().unwrap().clone();
    // The separator INTO FinalStdout is suppressed (text starts with \n).
    // The separator BACK into ToolUseAndEvents is suppressed (stdout
    // trailing newlines = 2, so at_visual_blank() = true).
    let blanks = captured.iter().filter(|l| l.is_empty()).count();
    assert_eq!(
        blanks, 0,
        "both separators suppressed: into stdout (text starts with \\n),              back to stderr (stdout trailing newlines >= 2): {captured:?}"
    );

    // Verify no consecutive blanks in the combined output.
    let mut prev_blank = false;
    for line in &captured {
        let is_blank = line.trim().is_empty();
        assert!(
            !(is_blank && prev_blank),
            "no consecutive blank lines: {captured:?}"
        );
        prev_blank = is_blank;
    }
}

#[test]
fn stdout_trailing_resets_after_intervening_stderr_line() {
    // Between two OutputText events that are separated by stderr
    // content, the second transition back to stdout still needs a
    // full separator — the stderr scroll between the two stdout
    // writes visually breaks the "stdout ended on blank" signal.
    let lines = Arc::new(StdMutex::new(Vec::new()));
    let dispatched = Arc::new(StdMutex::new(Vec::new()));
    let rendered_text = Arc::new(StdMutex::new(String::new()));

    let output_cb = {
        let buf = rendered_text.clone();
        Box::new(move |text: &str| {
            buf.lock().unwrap().push_str(text);
        })
    };

    let mut sink = make_sink(lines.clone(), dispatched).with_output_text_sink(output_cb);

    sink.on_semantic_event(SemanticEvent::OutputText {
        text: "first block.\n\n".into(),
        extra: json!({}),
    });
    sink.on_semantic_event(SemanticEvent::ToolResult {
        name: Some("bash".into()),
        id: None,
        status: Some("success".into()),
        exit_code: Some(0),
        output: None,
        extra: json!({}),
    });
    sink.on_semantic_event(SemanticEvent::OutputText {
        text: "second block.".into(),
        extra: json!({}),
    });

    let captured = lines.lock().unwrap().clone();
    let blanks = captured.iter().filter(|l| l.is_empty()).count();
    assert_eq!(
        blanks, 1,
        "first transition uses stdout trailing blank (skipped), second \
         transition back into FinalStdout must still emit the separator: \
         {captured:?}"
    );
}

#[test]
fn emit_trailer_line_inserts_single_separator_after_final_stdout() {
    let lines = Arc::new(StdMutex::new(Vec::new()));
    let dispatched = Arc::new(StdMutex::new(Vec::new()));

    let output_cb = Box::new(move |_text: &str| {
        // The renderer would normally write to stdout; we do not need
        // to capture it for this assertion. The section tracker is
        // what matters.
    });

    let mut sink = make_sink(lines.clone(), dispatched).with_output_text_sink(output_cb);

    sink.on_semantic_event(SemanticEvent::ToolResult {
        name: Some("bash".into()),
        id: None,
        status: Some("success".into()),
        exit_code: None,
        output: None,
        extra: json!({}),
    });
    sink.on_semantic_event(SemanticEvent::OutputText {
        text: "result".into(),
        extra: json!({}),
    });

    sink.emit_trailer_line("✓ 5s");
    sink.emit_trailer_line("  secondary");

    let captured = lines.lock().unwrap().clone();
    // Captured stderr lines should be, in order:
    //   <tool line>, "" (tool→final separator), "" (final→trailer
    //   separator), "✓ 5s", "  secondary"
    // The two separators are tagged to different sections so the
    // dedup does NOT collapse them; every section transition emits a
    // separator.
    assert!(!captured.is_empty());
    let blanks_then: Vec<_> = captured
        .iter()
        .enumerate()
        .filter(|(_, l)| l.is_empty())
        .map(|(idx, _)| idx)
        .collect();
    assert_eq!(
        blanks_then.len(),
        2,
        "one separator into FinalStdout and one into TrailerMetadata: {captured:?}"
    );
    assert!(
        captured.iter().any(|l| l.contains("✓ 5s")),
        "trailer line must be present: {captured:?}"
    );
}

#[test]
fn output_text_without_callback_is_dropped_not_rendered_as_status() {
    let lines = Arc::new(StdMutex::new(Vec::new()));
    let dispatched = Arc::new(StdMutex::new(Vec::new()));
    let mut sink = make_sink(lines.clone(), dispatched.clone());
    sink.on_semantic_event(SemanticEvent::OutputText {
        text: "hello".into(),
        extra: json!({}),
    });
    // No status line should be emitted for OutputText, and no hook dispatch.
    assert!(lines.lock().unwrap().is_empty());
    assert!(dispatched.lock().unwrap().is_empty());
}

#[test]
fn event_logger_records_every_event_with_full_payload() {
    let lines = Arc::new(StdMutex::new(Vec::new()));
    let dispatched = Arc::new(StdMutex::new(Vec::new()));
    let logged: Arc<StdMutex<Vec<(String, DispatchEventMeta)>>> =
        Arc::new(StdMutex::new(Vec::new()));

    let logger = {
        let captured = logged.clone();
        Box::new(move |event: &SemanticEvent, meta: &DispatchEventMeta| {
            captured
                .lock()
                .unwrap()
                .push((event.kind_str().into(), meta.clone()));
        })
    };

    let mut sink = make_sink(lines, dispatched).with_event_logger(logger);

    // OutputText has no agentic mapping but should still be logged.
    sink.on_semantic_event(SemanticEvent::OutputText {
        text: "hello".into(),
        extra: json!({}),
    });
    sink.on_semantic_event(SemanticEvent::ToolCall {
        name: Some("bash".into()),
        id: Some("t1".into()),
        input: Some(json!({"command": "ls"})),
        extra: json!({}),
    });
    sink.on_semantic_event(SemanticEvent::ProviderExtension {
        provider: Provider::Codex,
        kind: "item.updated".into(),
        payload: json!({"message": "still working"}),
    });

    let collected = logged.lock().unwrap().clone();
    let kinds: Vec<&str> = collected.iter().map(|(k, _)| k.as_str()).collect();
    assert_eq!(
        kinds,
        vec!["output_text", "tool_call", "provider_extension"]
    );

    // Every row must carry the full serialized semantic event under
    // extra["semantic_event"] so JSONL readers can replay fidelity.
    for (kind, meta) in &collected {
        let sem = meta
            .extra
            .get("semantic_event")
            .expect("semantic_event payload missing");
        assert_eq!(sem["type"], *kind);
        assert_eq!(
            meta.extra.get("synthetic_kind"),
            Some(&Value::String("stream_semantic_event".into()))
        );
    }
}
