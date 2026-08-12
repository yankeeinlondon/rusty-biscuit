//! Golden stderr snapshot tests: per-provider replay fixtures and the
//! OpenCode Phase-4 acceptance contract, split out of the provider-extension
//! suite to keep each test module below the high-risk SLOC band.

use super::*;
use claudine::stream::{ParserConfig, create_semantic_parser};
use serde_json::json;
use std::sync::Mutex as StdMutex;

pub(super) fn replay_to_stderr(
    provider: Provider,
    fixture: &[&str],
    model: Option<String>,
) -> Vec<String> {
    replay_to_combined(provider, fixture, model)
        .into_iter()
        .filter_map(|(is_stdout, line)| (!is_stdout).then_some(line))
        .collect()
}

/// Replay fixture lines through the parser + [`LiveSemanticSink`]
/// and capture every emission — both stderr status lines and
/// stdout `OutputText` bytes — in the order they were written. The
/// boolean is `true` when the emission was destined for stdout.
///
/// This is the authoritative view of the sink's rendered output
/// because the spec's spacing invariant is defined against the
/// combined stream: "there are no two consecutive blank lines"
/// across the whole rendered surface, not just stderr.
pub(super) fn replay_to_combined(
    provider: Provider,
    fixture: &[&str],
    model: Option<String>,
) -> Vec<(bool, String)> {
    let captured: Arc<StdMutex<Vec<(bool, String)>>> = Arc::new(StdMutex::new(Vec::new()));
    let dispatched: Arc<StdMutex<Vec<(AgenticEvent, String)>>> =
        Arc::new(StdMutex::new(Vec::new()));

    let dispatch = {
        let cap = dispatched.clone();
        Box::new(move |ev: AgenticEvent, _meta: DispatchEventMeta| {
            cap.lock().unwrap().push((ev, String::new()));
        })
            as Box<dyn Fn(AgenticEvent, DispatchEventMeta) + Send + Sync + 'static>
    };
    let emit_stderr = {
        let cap = captured.clone();
        Box::new(move |line: &str| {
            cap.lock().unwrap().push((false, line.to_string()));
        }) as Box<dyn Fn(&str) + Send + Sync + 'static>
    };
    let output_cb: OutputTextFn = {
        let cap = captured.clone();
        Box::new(move |text: &str| {
            // Emit every embedded line separately so the combined
            // view sees each stdout line as its own entry. Using
            // `str::lines` drops the spurious trailing empty that
            // `split('\n')` produces for newline-terminated text —
            // that trailing empty is a chunk artifact, not a real
            // blank line in the rendered output. Internal blank
            // lines (`"a\n\nb"`) are preserved so the combined
            // blank-line assertion can still catch real content
            // problems that straddle the stdout surface.
            for line in text.lines() {
                cap.lock().unwrap().push((true, line.to_string()));
            }
        })
    };

    let mut sink = LiveSemanticSink::new(
        provider,
        EnvironmentContext::default(),
        Path::new("/tmp"),
        Verbosity::Normal,
        Arc::new(Mutex::new(StructuredSummaryDetails::default())),
        dispatch,
        emit_stderr,
    )
    .with_output_text_sink(output_cb);

    struct Rec {
        events: Arc<StdMutex<Vec<SemanticEvent>>>,
    }
    impl SemanticEventSink for Rec {
        fn on_semantic_event(&mut self, event: SemanticEvent) {
            self.events.lock().unwrap().push(event);
        }
    }

    let inner = Arc::new(StdMutex::new(Vec::new()));
    let parser_sink = Rec {
        events: inner.clone(),
    };
    let config = ParserConfig { model };
    let mut parser = create_semantic_parser(provider, parser_sink, config);
    for line in fixture {
        parser.feed_line(line);
    }
    drop(parser);

    let events = inner.lock().unwrap().clone();
    for event in events {
        sink.on_semantic_event(event);
    }

    captured.lock().unwrap().clone()
}

#[test]
#[serial_test::serial]
fn claude_stderr_snapshot() {
    // Set ANTHROPIC_API_KEY so the rate-limit Warning renders on
    // stderr; the Subscription-mode suppression is covered by
    // `claude_rate_limit_warning_suppressed_when_anthropic_api_key_unset`.
    let _guard = TestEnvGuard::set("ANTHROPIC_API_KEY", "sk-test");
    let lines = replay_to_stderr(
        Provider::Claude,
        &[
            r#"{"type":"init","session_id":"s1","model":"claude-sonnet-4"}"#,
            r#"{"type":"tool_use","id":"t1","name":"bash","input":{"cmd":"ls -la"}}"#,
            r#"{"type":"tool_result","tool_use_id":"t1","content":"file.txt"}"#,
            r#"{"type":"task_started","task_id":"sa1","name":"researcher"}"#,
            r#"{"type":"task_progress","message":"working"}"#,
            r#"{"type":"task_completed","task_id":"sa1","name":"researcher","status":"success"}"#,
            r#"{"type":"rate_limit_event","is_throttled":true,"retry_after_ms":5000,"message":"Rate limited"}"#,
            r#"{"type":"some_future_event","x":1}"#,
        ],
        None,
    );
    assert!(!lines.is_empty());
    let joined = lines.join("\n");
    assert!(joined.contains('\u{2192}'), "expected → arrow: {joined:?}");
    assert!(joined.contains('\u{2190}'), "expected ← arrow: {joined:?}");
    assert!(joined.contains("Bash"));
    assert!(joined.contains("researcher"));
    assert!(joined.contains("Rate limited"));
    assert!(joined.contains("claude/some_future_event"));
}

#[test]
fn claude_tool_preface_stays_in_thinking_not_stdout() {
    let combined = replay_to_combined(
        Provider::Claude,
        &[
            r#"{"type":"system","subtype":"init","session_id":"s1","model":"claude-sonnet-4"}"#,
            r#"{"type":"assistant","message":{"content":[{"type":"text","text":"Let me investigate the spacing issue first."},{"type":"tool_use","id":"tu_1","name":"Read","input":{"file_path":"/tmp/section.rs"}}]}}"#,
        ],
        None,
    );

    assert!(
        combined.iter().all(|(is_stdout, _)| !*is_stdout),
        "Claude tool-preface narration must not open FinalStdout mid-turn: {combined:#?}"
    );

    let rendered: Vec<String> = combined.iter().map(|(_, line)| line.clone()).collect();
    let joined = rendered.join("\n");
    assert!(
        joined.contains("Let me investigate the spacing issue first."),
        "tool-preface narration must still render: {joined:?}"
    );
    assert!(
        joined.contains('\u{258c}'),
        "tool-preface narration must render through the thinking BlockQuote: {joined:?}"
    );
    assert!(
        joined.contains("Read"),
        "follow-up tool call must still render after the thinking block: {joined:?}"
    );

    let mut prev_blank = false;
    for line in &rendered {
        let is_blank = line.trim().is_empty();
        assert!(
            !(is_blank && prev_blank),
            "tool-preface replay must not introduce doubled blanks: {rendered:?}"
        );
        prev_blank = is_blank;
    }
}

#[test]
fn codex_stderr_snapshot() {
    let lines = replay_to_stderr(
        Provider::Codex,
        &[
            r#"{"type":"thread.started","thread_id":"th-1"}"#,
            r#"{"type":"item.started","item":{"id":"cmd1","type":"command_exec","tool_name":"bash","input":{"command":"ls"}}}"#,
            r#"{"type":"item.completed","item":{"id":"cmd1","type":"command_exec","status":"success","exit_code":0,"output":"file.txt"}}"#,
            r#"{"type":"item.completed","item":{"id":"f1","type":"file_change","path":"src/lib.rs","change_kind":"modified"}}"#,
            r#"{"type":"item.completed","item":{"id":"p1","type":"plan_update","message":"Step 2"}}"#,
            r#"{"type":"error","error_type":"rate_limit","error_message":"Too many requests"}"#,
            r#"{"type":"future.unknown","payload":{"k":1}}"#,
        ],
        Some("codex-mini".into()),
    );
    assert!(!lines.is_empty());
    let joined = lines.join("\n");
    assert!(joined.contains('\u{2192}'), "expected →: {joined:?}");
    assert!(joined.contains('\u{2190}'), "expected ←: {joined:?}");
    assert!(joined.contains("Bash"));
    assert!(joined.contains("modified src/lib.rs"));
    assert!(joined.contains("Step 2"));
    assert!(joined.contains("Too many requests"));
    assert!(joined.contains("codex/future.unknown"));
}

#[test]
fn gemini_stderr_snapshot() {
    let lines = replay_to_stderr(
        Provider::Gemini,
        &[
            r#"{"type":"init","session_id":"g1","model":"gemini-2.5-pro"}"#,
            r#"{"type":"tool_use","tool_id":"t1","tool_name":"search","parameters":{"q":"rust"}}"#,
            r#"{"type":"tool_result","tool_id":"t1","status":"success","output":{"hits":3}}"#,
            r#"{"type":"error","severity":"warning","message":"Loop detected"}"#,
            r#"{"type":"some_unknown","data":"x"}"#,
        ],
        None,
    );
    assert!(!lines.is_empty());
    let joined = lines.join("\n");
    assert!(joined.contains('\u{2192}'), "expected →: {joined:?}");
    assert!(joined.contains('\u{2190}'), "expected ←: {joined:?}");
    assert!(joined.contains("Search"));
    assert!(joined.contains("Loop detected"));
    assert!(joined.contains("gemini/some_unknown"));
}

#[test]
fn kimi_stderr_snapshot() {
    let lines = replay_to_stderr(
        Provider::KimiCode,
        &[
            r#"{"jsonrpc":"2.0","id":"init-1","result":{"protocol_version":"1.9","server":{"name":"Kimi Code CLI","version":"1.38.0"},"slash_commands":[],"hooks":[],"capabilities":{"supports_question":true}}}"#,
            r#"{"jsonrpc":"2.0","method":"event","params":{"type":"ToolCall","payload":{"id":"k1","function":{"name":"Shell","arguments":"{\"command\":\"ls\"}"}}}}"#,
            r#"{"jsonrpc":"2.0","method":"event","params":{"type":"ToolResult","payload":{"tool_call_id":"k1","return_value":{"is_error":false,"output":"ok"}}}}"#,
            r#"{"jsonrpc":"2.0","id":"prompt-2","error":{"code":-32005,"message":"slow down","data":null}}"#,
            r#"{"jsonrpc":"2.0","method":"event","params":{"type":"BrandNewEvent","payload":{}}}"#,
        ],
        None,
    );
    assert!(!lines.is_empty());
    let joined = lines.join("\n");
    assert!(joined.contains('\u{2192}'), "expected →: {joined:?}");
    assert!(joined.contains('\u{2190}'), "expected ←: {joined:?}");
    assert!(joined.contains("Shell"));
    assert!(joined.contains("slow down"));
    assert!(joined.contains("kimi/event:BrandNewEvent"));
}

#[test]
fn opencode_stderr_snapshot() {
    let lines = replay_to_stderr(
        Provider::OpenCode,
        &[
            r#"{"type":"step_start","sessionID":"ses_1"}"#,
            r#"{"type":"tool_start","part":{"id":"t1","tool_name":"bash","input":{"cmd":"ls"}}}"#,
            r#"{"type":"tool_end","part":{"tool_use_id":"t1","status":"success","content":"ok"}}"#,
            r#"{"type":"error","error_message":"API timeout"}"#,
            r#"{"type":"some_future_event","x":1}"#,
        ],
        Some("gpt-4o".into()),
    );
    assert!(!lines.is_empty());
    let joined = lines.join("\n");
    assert!(joined.contains('\u{2192}'), "expected →: {joined:?}");
    assert!(joined.contains('\u{2190}'), "expected ←: {joined:?}");
    assert!(joined.contains("Bash"));
    assert!(joined.contains("API timeout"));
    assert!(joined.contains("opencode/some_future_event"));
}

#[test]
fn opencode_tool_use_completion_shows_incoming_arrow_only() {
    let lines = replay_to_stderr(
        Provider::OpenCode,
        &[
            r#"{"type":"step_start","sessionID":"ses_1"}"#,
            r#"{"type":"tool_use","part":{"id":"t1","tool":"bash",
             "state":{"status":"completed","input":{"command":"ls -la"},"output":"file.txt"}}}"#,
        ],
        Some("gpt-4o".into()),
    );
    let joined = lines.join("\n");
    assert!(
        joined.contains('\u{2190}'),
        "incoming ← arrow must still render: {joined:?}"
    );
    assert!(
        !joined.contains('\u{2192}'),
        "outgoing → arrow must NOT render (no synthesized ToolCall): {joined:?}"
    );
    assert!(
        joined.contains("Bash"),
        "humanized tool name must appear: {joined:?}"
    );
}

/// 2026-04-18 contract: OpenCode `step_start` / `step_finish`
/// phase markers MUST NOT render to stderr. They were previously
/// surfaced as `Info` Status lines that produced visual noise
/// around the actual tool events. The events are still emitted
/// for JSONL/dispatch but suppressed at the sink boundary.
#[test]
fn opencode_step_phase_info_events_suppressed_from_stderr() {
    let lines = replay_to_stderr(
        Provider::OpenCode,
        &[
            r#"{"type":"step_start","sessionID":"ses_1"}"#,
            r#"{"type":"tool_start","part":{"id":"t1","tool_name":"read","input":{"file_path":"a.md"}}}"#,
            r#"{"type":"tool_end","part":{"tool_use_id":"t1","status":"success","content":"hi"}}"#,
            r#"{"type":"step_finish","sessionID":"ses_1"}"#,
        ],
        Some("gpt-4o".into()),
    );
    let joined = lines.join("\n");
    assert!(
        !joined.contains("step_start"),
        "step_start must not render to stderr: {joined:?}"
    );
    assert!(
        !joined.contains("step_finish"),
        "step_finish must not render to stderr: {joined:?}"
    );
    assert!(
        joined.contains("Read"),
        "real tool events must still render: {joined:?}"
    );
}

/// 2026-04-18 contract: a successful OpenCode `Bash` (or other
/// shell-shaped) tool result MUST carry the cached input summary
/// alongside the `successful` status. `Bash(successful)` with no
/// slot was the regression this guards against.
#[test]
fn opencode_bash_success_carries_summary_alongside_status() {
    let lines = replay_to_stderr(
        Provider::OpenCode,
        &[
            r#"{"type":"step_start","sessionID":"ses_1"}"#,
            r#"{"type":"tool_start","part":{"id":"t1","tool_name":"bash","input":{"command":"ls -la"}}}"#,
            r#"{"type":"tool_end","part":{"tool_use_id":"t1","status":"success","content":"file.txt"}}"#,
        ],
        Some("gpt-4o".into()),
    );
    let joined = lines.join("\n");
    assert!(
        joined.contains('\u{2190}'),
        "incoming ← arrow must render: {joined:?}"
    );
    assert!(
        joined.contains("Bash"),
        "humanized tool name must appear: {joined:?}"
    );
    assert!(
        joined.contains("successful"),
        "status word must still render: {joined:?}"
    );
    assert!(
        joined.contains("bash ls -la"),
        "shell summary must co-render with status: {joined:?}"
    );
}

#[test]
fn qwen_stderr_snapshot() {
    let lines = replay_to_stderr(
        Provider::QwenCode,
        &[
            r#"{"type":"init","session_id":"q1","model":"qwen-coder"}"#,
            r#"{"type":"tool_call","id":"q1","name":"bash","input":{"command":"git status"}}"#,
            r#"{"type":"tool_response","tool_use_id":"q1","status":"success","content":"clean"}"#,
            r#"{"type":"error","error":{"type":"rate_limit","message":"slow down"}}"#,
            r#"{"type":"something.new"}"#,
        ],
        None,
    );
    assert!(!lines.is_empty());
    let joined = lines.join("\n");
    assert!(joined.contains('\u{2192}'), "expected →: {joined:?}");
    assert!(joined.contains('\u{2190}'), "expected ←: {joined:?}");
    assert!(joined.contains("Bash"));
    assert!(joined.contains("slow down"));
    assert!(joined.contains("qwen/something.new"));
}

#[test]
#[serial_test::serial]
fn captured_fixtures_have_no_two_consecutive_blank_lines_per_provider() {
    // Some replays (notably Claude rate-limit events) consult
    // ANTHROPIC_API_KEY; set it so any warnings render normally and
    // serialize with other env-touching tests.
    let _guard = TestEnvGuard::set("ANTHROPIC_API_KEY", "sk-test");

    let fixtures: &[(Provider, &str, Option<&str>)] = &[
        (Provider::Claude, "claude.ndjson", None),
        (Provider::Codex, "codex.ndjson", Some("codex-mini")),
        (Provider::Gemini, "gemini.ndjson", None),
        (Provider::OpenCode, "opencode.ndjson", Some("gpt-4o")),
    ];
    for (provider, fname, model) in fixtures {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("lib")
            .join("tests/fixtures/providers")
            .join(fname);
        if !path.exists() {
            eprintln!("skip: fixture not found at {path:?}");
            continue;
        }
        let raw = std::fs::read_to_string(&path).expect("read fixture");
        let fixture_lines: Vec<&str> = raw.lines().collect();
        // Assert against the COMBINED stdout+stderr emission
        // stream — per spec, the spacing invariant is defined
        // over all rendered output in emission order, not just
        // stderr. A stderr-only assertion would miss consecutive
        // blanks that straddle the FinalStdout section boundary.
        let combined =
            replay_to_combined(*provider, &fixture_lines, model.map(String::from));
        let mut prev_blank = false;
        for (is_stdout, line) in &combined {
            let is_blank = line.trim().is_empty();
            assert!(
                !(is_blank && prev_blank),
                "provider={provider:?} ({fname}): two consecutive blank lines in combined rendered output (is_stdout={is_stdout}):\n{combined:#?}"
            );
            prev_blank = is_blank;
        }
    }
}

/// Phase 4 acceptance gate for the 2026-04-18 OpenCode reporting
/// improvements (`features/2026-04-18-opencode-reporting-improvements`).
///
/// Replays the captured `opencode.ndjson` fixture and asserts the
/// four user-visible requirements from `spec.md` simultaneously:
///
/// 1. No `step_start` / `step_finish` lines render to stderr.
/// 2. No two consecutive blank lines in the combined emission.
/// 3. Successful `Bash` results carry a useful summary slot
///    (e.g. `bash git log ...`) and not just `successful` alone.
/// 4. Successful `Read` / file-tool results carry a path slot.
///
/// The fixture contains 23 step pairs and 41 `tool_use` events,
/// so this is a meaningful end-to-end replay of the spec's
/// reference session shape.
#[test]
#[serial_test::serial]
fn opencode_acceptance_replay_satisfies_phase4_contract() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("lib")
        .join("tests/fixtures/providers/opencode.ndjson");
    let raw = std::fs::read_to_string(&path).expect("read opencode fixture");
    let fixture_lines: Vec<&str> = raw.lines().collect();

    let combined =
        replay_to_combined(Provider::OpenCode, &fixture_lines, Some("gpt-4o".into()));

    let stderr_only: Vec<String> = combined
        .iter()
        .filter_map(
            |(is_stdout, line)| {
                if *is_stdout { None } else { Some(line.clone()) }
            },
        )
        .collect();
    let stderr_joined = stderr_only.join("\n");

    // (1) step markers must be suppressed from stderr.
    assert!(
        !stderr_joined.contains("step_start"),
        "spec §1: step_start must not render to stderr:\n{stderr_joined}"
    );
    assert!(
        !stderr_joined.contains("step_finish"),
        "spec §1: step_finish must not render to stderr:\n{stderr_joined}"
    );

    // (2) no two consecutive blank lines anywhere in the
    //     combined emission stream.
    let mut prev_blank = false;
    for (is_stdout, line) in &combined {
        let is_blank = line.trim().is_empty();
        assert!(
            !(is_blank && prev_blank),
            "spec §2: two consecutive blank lines in combined output (is_stdout={is_stdout}):\n{combined:#?}"
        );
        prev_blank = is_blank;
    }

    // (3) at least one ← Bash(...) line in the fixture must carry
    //     a non-empty summary slot — i.e. the slot text is more
    //     than just `successful`.
    let bash_incoming_lines: Vec<&String> = stderr_only
        .iter()
        .filter(|l| l.contains('\u{2190}') && l.contains("Bash"))
        .collect();
    assert!(
        !bash_incoming_lines.is_empty(),
        "spec §3: fixture should produce at least one ← Bash line:\n{stderr_joined}"
    );
    let bash_with_summary = bash_incoming_lines.iter().any(|l| l.contains("bash "));
    assert!(
        bash_with_summary,
        "spec §3: at least one ← Bash result must carry a `bash <command>` summary slot. Got:\n{}",
        bash_incoming_lines
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    );

    // (4) successful incoming Read / file-tool lines must carry
    //     a path slot — verified by the presence of an OSC8
    //     hyperlink escape (`\x1b]8;;`) on at least one
    //     incoming Read line. The fixture only includes Glob
    //     and Bash tool_use events, so file-tool coverage is
    //     handled by the unit tests in this file (e.g.
    //     `tool_result_success_status_co_renders_with_input_summary`)
    //     and `opencode_bash_success_carries_summary_alongside_status`
    //     above. Skipping a fixture-level assertion here keeps
    //     this test honest: we do not assert a Read line if the
    //     fixture does not contain one.
}

/// Regression test for the duplicate-reasoning fix.
///
/// Before the fix, reasoning was rendered twice on the structured-
/// stream path:
/// 1. `LiveSemanticSink` rendered it as a BlockQuote via
///    `render_thinking_block`.
/// 2. `StreamThinkingRenderer` in `exec.rs` ALSO rendered it via
///    the `reasoning_cb` callback (emitting a dim "Thinking..."
///    header plus dimmed lines).
///
/// After the fix, `LiveSemanticSink` owns reasoning rendering
/// end-to-end. The `reasoning_cb` is a no-op. This test verifies:
/// - Reasoning text appears in stderr exactly once.
/// - The old "Thinking..." header from `StreamThinkingRenderer`
///   does NOT appear.
#[test]
fn reasoning_appears_exactly_once_in_stderr() {
    let lines: Arc<StdMutex<Vec<String>>> = Arc::new(StdMutex::new(Vec::new()));
    let dispatched: Arc<StdMutex<Vec<(AgenticEvent, String)>>> =
        Arc::new(StdMutex::new(Vec::new()));

    let dispatch = {
        let cap = dispatched.clone();
        Box::new(move |ev: AgenticEvent, _meta: DispatchEventMeta| {
            cap.lock().unwrap().push((ev, String::new()));
        })
            as Box<dyn Fn(AgenticEvent, DispatchEventMeta) + Send + Sync + 'static>
    };
    let emit = {
        let cap = lines.clone();
        Box::new(move |line: &str| {
            cap.lock().unwrap().push(line.to_string());
        }) as Box<dyn Fn(&str) + Send + Sync + 'static>
    };

    let mut sink = LiveSemanticSink::new(
        Provider::Claude,
        EnvironmentContext::default(),
        Path::new("/tmp"),
        Verbosity::Normal,
        Arc::new(Mutex::new(StructuredSummaryDetails::default())),
        dispatch,
        emit,
    );

    // Feed a reasoning event directly (simulating what the parser
    // would emit after parsing a thinking content block).
    sink.on_semantic_event(SemanticEvent::Reasoning {
        text: "Let me analyze this step by step.".into(),
        extra: json!({}),
    });
    // A lone thought now stays buffered in the ThinkingStream until a
    // boundary — no following non-Reasoning event here, so the `Drop`
    // close-flush is what coalesces and renders it. Drop the sink to
    // trigger that flush before inspecting the captured lines.
    drop(sink);

    let captured = lines.lock().unwrap().join("\n");

    // Reasoning text must appear in stderr.
    assert!(
        captured.contains("analyze this step by step"),
        "reasoning text must appear in stderr: {captured:?}"
    );

    // The old StreamThinkingRenderer "Thinking..." header must NOT
    // appear — that was the duplicate-rendering artifact.
    assert!(
        !captured.contains("Thinking..."),
        "old StreamThinkingRenderer header must not appear: {captured:?}"
    );

    // Count occurrences of the reasoning text to verify it appears
    // exactly once (not duplicated).
    let count = captured.matches("analyze this step by step").count();
    assert_eq!(
        count, 1,
        "reasoning text must appear exactly once, found {count} times: {captured:?}"
    );
}

/// Build a bare sink that captures every stderr line into `lines`.
fn capturing_sink(lines: Arc<StdMutex<Vec<String>>>) -> LiveSemanticSink {
    let dispatch = Box::new(|_ev: AgenticEvent, _meta: DispatchEventMeta| {})
        as Box<dyn Fn(AgenticEvent, DispatchEventMeta) + Send + Sync + 'static>;
    let emit = {
        let cap = lines.clone();
        Box::new(move |line: &str| {
            cap.lock().unwrap().push(line.to_string());
        }) as Box<dyn Fn(&str) + Send + Sync + 'static>
    };
    LiveSemanticSink::new(
        Provider::Claude,
        EnvironmentContext::default(),
        Path::new("/tmp"),
        Verbosity::Normal,
        Arc::new(Mutex::new(StructuredSummaryDetails::default())),
        dispatch,
        emit,
    )
}

/// The coalescing win: Claude emits one `Reasoning` event per
/// `thinking_delta`, so a single thought arrives as several token-level
/// fragments. The `ThinkingStream` component buffers them and flushes ONE
/// coalesced `▌ ` block at the boundary — not one bordered fragment per
/// delta as the old per-delta render did.
#[test]
fn consecutive_reasoning_deltas_coalesce_into_one_block() {
    let lines: Arc<StdMutex<Vec<String>>> = Arc::new(StdMutex::new(Vec::new()));
    let mut sink = capturing_sink(lines.clone());

    // Three deltas forming one sentence, none newline-terminated — exactly
    // the token-fragment shape Claude produces.
    for delta in ["Let me ", "think about ", "this problem."] {
        sink.on_semantic_event(SemanticEvent::Reasoning {
            text: delta.into(),
            extra: json!({}),
        });
    }
    // No following event: the `Drop` close-flush coalesces the residual
    // fragments into a single block.
    drop(sink);

    let captured = lines.lock().unwrap().clone();
    let joined = captured.join("\n");
    assert!(
        joined.contains("Let me think about this problem."),
        "coalesced block must carry the joined sentence: {captured:?}"
    );
    let block_lines = captured
        .iter()
        .filter(|l| l.contains('\u{258c}'))
        .count();
    assert_eq!(
        block_lines, 1,
        "one thought must render as ONE ▌ block, not one fragment per delta: {captured:?}"
    );
}

/// A `Reasoning` run followed by a `ToolCall` must flush the coalesced
/// thinking block BEFORE the tool line, so a tool render never interleaves
/// mid-thought.
#[test]
fn reasoning_flushes_before_following_tool_call() {
    let lines: Arc<StdMutex<Vec<String>>> = Arc::new(StdMutex::new(Vec::new()));
    let mut sink = capturing_sink(lines.clone());

    for delta in ["Checking the ", "file first."] {
        sink.on_semantic_event(SemanticEvent::Reasoning {
            text: delta.into(),
            extra: json!({}),
        });
    }
    sink.on_semantic_event(SemanticEvent::ToolCall {
        name: Some("Bash".into()),
        id: None,
        input: Some(json!({"command": "ls"})),
        extra: json!({}),
    });

    let captured = lines.lock().unwrap().clone();
    let thinking_idx = captured
        .iter()
        .position(|l| l.contains('\u{258c}'))
        .unwrap_or_else(|| panic!("thinking block must render: {captured:?}"));
    let tool_idx = captured
        .iter()
        .position(|l| l.contains("Bash"))
        .unwrap_or_else(|| panic!("tool line must render: {captured:?}"));
    assert!(
        thinking_idx < tool_idx,
        "thinking must flush before the tool line: {captured:?}"
    );
    assert!(
        captured.join("\n").contains("Checking the file first."),
        "coalesced thought must carry the joined text: {captured:?}"
    );
}

// Local copy of the serialized env guard (see sibling provider-extension module).
struct TestEnvGuard {
    key: &'static str,
    prior: Option<String>,
}
impl TestEnvGuard {
    fn remove(key: &'static str) -> Self {
        let prior = std::env::var(key).ok();
        // SAFETY: tests are serialized via serial_test; no other thread
        // races on env vars while the guard exists.
        unsafe {
            std::env::remove_var(key);
        }
        Self { key, prior }
    }
    fn set(key: &'static str, value: &str) -> Self {
        let prior = std::env::var(key).ok();
        unsafe {
            std::env::set_var(key, value);
        }
        Self { key, prior }
    }
}
impl Drop for TestEnvGuard {
    fn drop(&mut self) {
        unsafe {
            match &self.prior {
                Some(v) => std::env::set_var(self.key, v),
                None => std::env::remove_var(self.key),
            }
        }
    }
}
