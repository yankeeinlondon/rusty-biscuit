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

/// RAII wrapper that restores the prior env var value on drop. Tests
/// using this guard must be annotated `#[serial_test::serial]` because
/// env vars are process-wide.
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

/// Build a sink rooted at `cwd` with a captured stderr line buffer.
/// Used by the link-rendering tests so fixtures can feed absolute
/// paths that resolve against a non-`/tmp` root.
fn make_sink_with_cwd(
    captured_lines: Arc<StdMutex<Vec<String>>>,
    cwd: &Path,
) -> LiveSemanticSink {
    let dispatch = Box::new(|_: AgenticEvent, _: DispatchEventMeta| {});
    let emit = {
        let lines = captured_lines.clone();
        Box::new(move |line: &str| {
            lines.lock().unwrap().push(line.to_string());
        })
    };
    let mut sink = LiveSemanticSink::new(
        Provider::Claude,
        EnvironmentContext::default(),
        cwd,
        Verbosity::Normal,
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

/// Strip biscuit-terminal Layout soft-wrap continuations so assertions
/// can check the pre-wrap content regardless of the terminal-aware
/// column budget applied by `Status` + `Layout`.
///
/// Handles both the 2-space hanging indent used by generic status lines
/// (`ProviderExtension`, `Info`, `Warning`) and the 4-space hanging
/// indent used by tool-call status lines (`→ Name(...)` / `← Name(...)`)
/// which bump the indent to align continuation text under the tool
/// name rather than under the state-icon glyph.
fn strip_layout_wraps(rendered: &str) -> String {
    rendered
        .replace("-\n    ", "")
        .replace("\n    ", "")
        .replace("-\n  ", "")
        .replace("\n  ", "")
}

#[test]
fn provider_extension_with_only_nested_text_renders_summary_not_raw_json() {
    let lines = Arc::new(StdMutex::new(Vec::new()));
    let dispatched = Arc::new(StdMutex::new(Vec::new()));
    let mut sink = make_sink(lines.clone(), dispatched.clone());

    // Payload has no top-level message/status/name/path, but nested text.
    sink.on_semantic_event(SemanticEvent::ProviderExtension {
        provider: Provider::Codex,
        kind: "future.unknown".into(),
        payload: json!({
            "item": {
                "content": { "parts": [ { "text": "meaningful text here" } ] }
            }
        }),
    });

    let rendered = lines.lock().unwrap().join("\n");
    assert!(
        rendered.contains("meaningful text here"),
        "expected nested text preview in stderr: {rendered}"
    );
    assert!(
        !rendered.contains(r#"{"item":"#),
        "raw JSON must not appear in stderr: {rendered}"
    );
}

#[test]
fn provider_extension_unresolvable_drops_payload_tail_entirely() {
    let lines = Arc::new(StdMutex::new(Vec::new()));
    let dispatched = Arc::new(StdMutex::new(Vec::new()));
    let mut sink = make_sink(lines.clone(), dispatched.clone());

    sink.on_semantic_event(SemanticEvent::ProviderExtension {
        provider: Provider::Codex,
        kind: "opaque.event".into(),
        payload: json!({
            "some_numeric_field": 42,
            "another": [1, 2, 3]
        }),
    });

    let rendered = lines.lock().unwrap().join("\n");
    assert!(
        rendered.contains("codex/opaque.event"),
        "provider/kind label must still appear: {rendered}"
    );
    assert!(
        !rendered.contains(r#"{"some_numeric_field":"#) && !rendered.contains("42"),
        "raw payload must not appear when no human-readable summary is available: {rendered}"
    );
    assert!(
        !rendered.contains(" \u{00b7} {"),
        "must not render the summary separator followed by raw JSON: {rendered}"
    );
}

#[test]
fn provider_extension_respects_silent_kind_allowlist() {
    let lines = Arc::new(StdMutex::new(Vec::new()));
    let dispatched = Arc::new(StdMutex::new(Vec::new()));
    let mut sink = make_sink(lines.clone(), dispatched.clone());

    // Kinds in the silent allowlist must produce NO stderr line at all
    // (they still get dispatched and logged, just not rendered).
    sink.on_semantic_event(SemanticEvent::ProviderExtension {
        provider: Provider::Claude,
        kind: "stream_event".into(),
        payload: json!({ "delta": "chunk" }),
    });

    let rendered = lines.lock().unwrap().join("\n");
    assert!(
        !rendered.contains("claude/stream_event"),
        "silent-kind allowlist must suppress the status line entirely: {rendered}"
    );
}

#[test]
fn provider_extension_kimi_high_volume_kinds_are_silent() {
    // Phase 5 of the fix-kimi plan adds defensive Kimi entries to the
    // silent-extension allowlist so high-volume wire fallback kinds
    // (ContentPart, ToolCallPart, StatusUpdate, and legacy
    // stream-json names) don't flood stderr if a future Kimi
    // protocol revision changes payload shapes and the typed
    // deserialization falls back to ProviderExtension.
    for kind in [
        "event:ContentPart",
        "event:ToolCallPart",
        "event:StatusUpdate",
        "event:MessageStart",
        "event:MessageDelta",
        "event:MessageEnd",
        "event:Thinking",
    ] {
        let lines = Arc::new(StdMutex::new(Vec::new()));
        let dispatched = Arc::new(StdMutex::new(Vec::new()));
        let mut sink = make_sink_for_provider(
            Provider::KimiCode,
            Verbosity::Normal,
            lines.clone(),
            dispatched.clone(),
        );
        sink.on_semantic_event(SemanticEvent::ProviderExtension {
            provider: Provider::KimiCode,
            kind: kind.into(),
            payload: json!({"delta": "x"}),
        });
        let rendered = lines.lock().unwrap().join("\n");
        assert!(
            !rendered.contains(&format!("kimi/{kind}")),
            "kimi {kind} must be suppressed: {rendered}"
        );
    }
}

#[test]
fn provider_extension_kimi_unknown_kinds_still_surface() {
    // Inverse of the silent-allowlist: unknown Kimi event types must
    // still render as ProviderExtension status lines so operators
    // can see protocol drift and add explicit handling.
    let lines = Arc::new(StdMutex::new(Vec::new()));
    let dispatched = Arc::new(StdMutex::new(Vec::new()));
    let mut sink = make_sink_for_provider(
        Provider::KimiCode,
        Verbosity::Normal,
        lines.clone(),
        dispatched.clone(),
    );
    sink.on_semantic_event(SemanticEvent::ProviderExtension {
        provider: Provider::KimiCode,
        kind: "event:FutureKimiEvent".into(),
        payload: json!({"x": 1}),
    });
    let rendered = lines.lock().unwrap().join("\n");
    assert!(
        rendered.contains("kimi/event:FutureKimiEvent"),
        "unknown Kimi event types must surface as ProviderExtension: {rendered}"
    );
}

#[test]
fn provider_extension_claude_system_hook_kinds_are_silent() {
    // Task 2a.2 parser emits hook events with kinds `system/hook_started`,
    // `system/hook_response`, `system/hook_progress`; newer Claude builds add
    // `system/thinking_tokens` telemetry. The sink allowlist must suppress all
    // of them so subscription users don't see this per-turn system noise.
    for kind in [
        "system/hook_started",
        "system/hook_response",
        "system/hook_progress",
        "system/thinking_tokens",
    ] {
        let lines = Arc::new(StdMutex::new(Vec::new()));
        let dispatched = Arc::new(StdMutex::new(Vec::new()));
        let mut sink = make_sink(lines.clone(), dispatched.clone());
        sink.on_semantic_event(SemanticEvent::ProviderExtension {
            provider: Provider::Claude,
            kind: kind.into(),
            payload: json!({"hook_name": "SessionStart:startup"}),
        });
        let rendered = lines.lock().unwrap().join("\n");
        assert!(
            !rendered.contains(&format!("claude/{kind}")),
            "kind {kind:?} must be suppressed: {rendered}"
        );
    }
}

#[test]
fn opencode_firecrawl_tool_use_does_not_render_via_info_glyph() {
    // Task 2c.4 regression: the ⚙ firecrawl line was OpenCode's own
    // TUI output (suppressed via noise-prefix list in profile.rs);
    // the sink's own rendering of a firecrawl ToolResult must use
    // the ← arrow via ToolCallDisplay, NOT the ⚙ Info glyph.
    let lines = Arc::new(StdMutex::new(Vec::new()));
    let dispatched = Arc::new(StdMutex::new(Vec::new()));
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
        Box::new(move |line: &str| {
            lines.lock().unwrap().push(line.to_string());
        })
    };
    let mut sink = LiveSemanticSink::new(
        Provider::OpenCode,
        EnvironmentContext::default(),
        Path::new("/tmp"),
        Verbosity::Normal,
        Arc::new(Mutex::new(StructuredSummaryDetails::default())),
        dispatch,
        emit,
    );
    sink.on_semantic_event(SemanticEvent::ToolResult {
        name: Some("firecrawl_firecrawl_search".into()),
        id: None,
        status: Some("success".into()),
        exit_code: None,
        output: None,
        extra: json!({"input": {"query": "NFL draft 2026 date"}}),
    });
    let rendered = lines.lock().unwrap().join("\n");
    assert!(
        !rendered.contains('\u{2699}'),
        "must not use the ⚙ Info glyph for tool events: {rendered:?}"
    );
    assert!(
        rendered.contains("Firecrawl Search"),
        "humanized tool name must appear: {rendered:?}"
    );
    assert!(
        rendered.contains('\u{2190}'),
        "incoming ← arrow must render: {rendered:?}"
    );
}

#[test]
fn tool_call_renders_canonical_format_with_humanized_name_and_query_summary() {
    let lines = Arc::new(StdMutex::new(Vec::new()));
    let dispatched = Arc::new(StdMutex::new(Vec::new()));
    let mut sink = make_sink(lines.clone(), dispatched.clone());
    sink.on_semantic_event(SemanticEvent::ToolCall {
        name: Some("firecrawl_firecrawl_search".into()),
        id: None,
        input: Some(json!({"query": "NFL draft 2026 date"})),
        extra: json!({}),
    });
    let rendered = lines.lock().unwrap().join("\n");
    assert!(
        rendered.contains("Firecrawl Search"),
        "expected humanized name in {rendered:?}"
    );
    assert!(
        rendered.contains("NFL draft 2026 date"),
        "expected query summary in {rendered:?}"
    );
    assert!(rendered.contains('\u{2192}'), "expected → arrow");
}

#[test]
fn tool_result_renders_status_word_when_status_present() {
    let lines = Arc::new(StdMutex::new(Vec::new()));
    let dispatched = Arc::new(StdMutex::new(Vec::new()));
    let mut sink = make_sink(lines.clone(), dispatched.clone());
    sink.on_semantic_event(SemanticEvent::ToolResult {
        name: Some("firecrawl_firecrawl_search".into()),
        id: None,
        status: Some("success".into()),
        exit_code: None,
        output: None,
        extra: json!({}),
    });
    let rendered = lines.lock().unwrap().join("\n");
    assert!(rendered.contains("Firecrawl Search"));
    assert!(rendered.contains("successful"));
    assert!(rendered.contains('\u{2190}'));
}

#[test]
fn no_captured_fixture_ever_renders_raw_json_on_stderr() {
    use std::path::Path as StdPath;

    let fixtures_dir = StdPath::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("lib")
        .join("tests")
        .join("fixtures")
        .join("providers");

    assert!(
        fixtures_dir.exists(),
        "fixtures dir must exist: {fixtures_dir:?}"
    );

    for provider_slug in &["claude", "codex", "gemini", "opencode"] {
        let fixture = fixtures_dir.join(format!("{provider_slug}.ndjson"));
        if !fixture.exists() {
            continue; // optional fixtures
        }
        let provider = match *provider_slug {
            "claude" => Provider::Claude,
            "codex" => Provider::Codex,
            "gemini" => Provider::Gemini,
            "opencode" => Provider::OpenCode,
            _ => unreachable!(),
        };
        let fixture_lines: Vec<String> = std::fs::read_to_string(&fixture)
            .expect("read fixture")
            .lines()
            .map(String::from)
            .collect();

        let lines_ref: Vec<&str> = fixture_lines.iter().map(String::as_str).collect();
        let stderr_lines = super::golden_stderr::replay_to_stderr(provider, &lines_ref, None);

        for line in &stderr_lines {
            // Tool result lines (`← Name(...)`) and tool call lines
            // (`→ Name(...)`) legitimately carry tool slot content
            // derived from input or output text, which may include
            // arbitrary characters — for example, a successful
            // `run_shell_command` whose output is grep results that
            // happen to embed JSON strings. Per the 2026-04-18
            // contract, those summaries co-render with the status,
            // so the raw-JSON guard does not apply to lines wrapped
            // in a tool arrow.
            if line.contains('\u{2190}') || line.contains('\u{2192}') {
                continue;
            }
            // Heuristic: a line is "raw JSON" if it contains both `{`
            // and a JSON-shaped key-value opener like `"\:`.
            let has_json_obj_opener = line.contains('{') && line.contains("\":");
            assert!(
                !has_json_obj_opener,
                "provider={provider_slug}: stderr line contains raw JSON: {line:?}"
            );
        }
    }
}

#[test]
fn long_summary_is_not_truncated_to_60_or_80_chars() {
    let lines = Arc::new(StdMutex::new(Vec::new()));
    let dispatched = Arc::new(StdMutex::new(Vec::new()));
    let mut sink = make_sink(lines.clone(), dispatched.clone());
    let long = "a".repeat(200);
    sink.on_semantic_event(SemanticEvent::ToolCall {
        name: Some("Bash".into()),
        id: None,
        input: Some(json!({"command": long.clone()})),
        extra: json!({}),
    });
    let rendered = lines.lock().unwrap().join("\n");
    let unwrapped = strip_layout_wraps(&rendered);
    assert!(
        unwrapped.contains(&long),
        "long command must not be truncated; got {rendered:?}"
    );
    assert!(!rendered.contains('\u{2026}'), "no ellipsis expected");
}

#[test]
fn long_provider_extension_payload_is_not_capped_at_80() {
    let lines = Arc::new(StdMutex::new(Vec::new()));
    let dispatched = Arc::new(StdMutex::new(Vec::new()));
    let mut sink = make_sink(lines.clone(), dispatched.clone());
    let long = "x".repeat(300);
    sink.on_semantic_event(SemanticEvent::ProviderExtension {
        provider: Provider::Codex,
        kind: "custom.kind".into(),
        payload: json!({"message": long.clone()}),
    });
    let rendered = lines.lock().unwrap().join("\n");
    let unwrapped = strip_layout_wraps(&rendered);
    assert!(
        unwrapped.contains(&long),
        "long provider-extension message must not be truncated; got {rendered:?}"
    );
    assert!(!rendered.contains('\u{2026}'), "no ellipsis expected");
}

#[test]
#[serial_test::serial]
fn claude_generic_rate_limit_warning_suppressed_for_subscription_metadata() {
    let _guard = TestEnvGuard::set("ANTHROPIC_API_KEY", "sk-test");
    let lines = Arc::new(StdMutex::new(Vec::new()));
    let dispatched = Arc::new(StdMutex::new(Vec::new()));
    let mut sink = make_sink(lines.clone(), dispatched.clone());
    sink.on_semantic_event(SemanticEvent::SessionStart {
        session_id: Some("s1".into()),
        model: Some("claude".into()),
        extra: json!({"api_key_source": "none"}),
    });
    lines.lock().unwrap().clear();
    sink.on_semantic_event(SemanticEvent::Warning {
        message: "rate limit".into(),
        extra: json!({"raw_kind": "rate_limit_event"}),
    });
    assert!(
        lines.lock().unwrap().is_empty(),
        "generic rate-limit Warning must not render for subscription auth"
    );
    assert!(
        !dispatched.lock().unwrap().is_empty(),
        "underlying dispatch must still fire so JSONL log retains the event"
    );
}

#[test]
#[serial_test::serial]
fn claude_generic_rate_limit_warning_suppression_preserves_jsonl_log() {
    // Definition of Done: "underlying event still in JSONL for
    // subscription users". Explicit assertion that the event-log
    // closure fires even when the stderr render is suppressed.
    let _guard = TestEnvGuard::set("ANTHROPIC_API_KEY", "sk-test");
    let lines = Arc::new(StdMutex::new(Vec::new()));
    let dispatched = Arc::new(StdMutex::new(Vec::new()));
    let logged: Arc<StdMutex<Vec<String>>> = Arc::new(StdMutex::new(Vec::new()));
    let logger = {
        let captured = logged.clone();
        Box::new(move |event: &SemanticEvent, _meta: &DispatchEventMeta| {
            captured.lock().unwrap().push(event.kind_str().into());
        })
    };
    let mut sink = make_sink(lines.clone(), dispatched.clone()).with_event_logger(logger);
    sink.on_semantic_event(SemanticEvent::SessionStart {
        session_id: Some("s1".into()),
        model: Some("claude".into()),
        extra: json!({"api_key_source": "none"}),
    });
    lines.lock().unwrap().clear();
    logged.lock().unwrap().clear();
    sink.on_semantic_event(SemanticEvent::Warning {
        message: "rate limit".into(),
        extra: json!({"raw_kind": "rate_limit_event"}),
    });
    assert!(
        lines.lock().unwrap().is_empty(),
        "stderr must be suppressed"
    );
    let kinds = logged.lock().unwrap().clone();
    assert_eq!(
        kinds,
        vec!["warning".to_string()],
        "JSONL log must still receive the Warning event"
    );
}

#[test]
#[serial_test::serial]
fn claude_generic_rate_limit_warning_renders_for_api_key_metadata() {
    let _guard = TestEnvGuard::remove("ANTHROPIC_API_KEY");
    let lines = Arc::new(StdMutex::new(Vec::new()));
    let dispatched = Arc::new(StdMutex::new(Vec::new()));
    let mut sink = make_sink(lines.clone(), dispatched.clone());
    sink.on_semantic_event(SemanticEvent::SessionStart {
        session_id: Some("s1".into()),
        model: Some("claude".into()),
        extra: json!({"api_key_source": "ANTHROPIC_API_KEY"}),
    });
    lines.lock().unwrap().clear();
    sink.on_semantic_event(SemanticEvent::Warning {
        message: "rate limit".into(),
        extra: json!({"raw_kind": "rate_limit_event"}),
    });
    let rendered = lines.lock().unwrap().join("\n");
    assert!(
        rendered.contains("rate limit"),
        "generic rate-limit Warning must render for API-key auth: {rendered:?}"
    );
}

#[test]
#[serial_test::serial]
fn claude_explicit_rate_limit_message_renders_for_subscription_metadata() {
    let _guard = TestEnvGuard::remove("ANTHROPIC_API_KEY");
    let lines = Arc::new(StdMutex::new(Vec::new()));
    let dispatched = Arc::new(StdMutex::new(Vec::new()));
    let mut sink = make_sink(lines.clone(), dispatched.clone());
    sink.on_semantic_event(SemanticEvent::SessionStart {
        session_id: Some("s1".into()),
        model: Some("claude".into()),
        extra: json!({"api_key_source": "none"}),
    });
    lines.lock().unwrap().clear();
    sink.on_semantic_event(SemanticEvent::Warning {
        message: "Claude rate limit warning: your 5-hour session window is approaching the cap. Window resets on 2024-04-01 at 19:33".into(),
        extra: json!({
            "raw_kind": "rate_limit_event",
            "rate_limit_status": "approaching_limit",
            "reset_at": "2024-04-01T19:33:20+00:00"
        }),
    });
    let rendered = lines.lock().unwrap().join("\n");
    assert!(
        rendered.contains("Window resets on"),
        "explicit Claude rate-limit metadata must render for subscriptions: {rendered:?}"
    );
    assert!(
        rendered.contains("approaching the cap"),
        "explicit Claude rate-limit metadata must include user-friendly wording: {rendered:?}"
    );
}

#[test]
fn reasoning_emits_block_quote_to_stderr_in_thinking_section() {
    let lines = Arc::new(StdMutex::new(Vec::new()));
    let dispatched = Arc::new(StdMutex::new(Vec::new()));
    let mut sink = make_sink(lines.clone(), dispatched.clone());
    sink.on_semantic_event(SemanticEvent::Reasoning {
        text: "considering the options".into(),
        extra: json!({}),
    });
    // A lone thought stays buffered until a boundary; with no following
    // event, the `Drop` close-flush renders it. Drop before inspecting.
    drop(sink);
    let rendered = lines.lock().unwrap().join("\n");
    assert!(
        rendered.contains("considering the options"),
        "reasoning text must appear in stderr: {rendered:?}"
    );
}

#[test]
fn tracing_warning_renders_header_and_block_quote_body() {
    let lines = Arc::new(StdMutex::new(Vec::new()));
    let dispatched = Arc::new(StdMutex::new(Vec::new()));
    let mut sink = make_sink(lines.clone(), dispatched.clone());
    sink.on_semantic_event(SemanticEvent::Warning {
        message: "forked agents inherit the parent agent type".into(),
        extra: json!({
            "provider": "codex",
            "tracing_target": "codex_core::tools::router",
            "tracing_level": "error",
        }),
    });
    let rendered = lines.lock().unwrap().join("\n");
    assert!(
        rendered.contains("WARN"),
        "tracing warning must render the WARN label regardless of the incoming level: {rendered:?}"
    );
    assert!(
        rendered.contains("codex_core::tools::router"),
        "tracing target must appear in the header: {rendered:?}"
    );
    assert!(
        rendered.contains("forked agents inherit the parent agent type"),
        "tracing body must appear in the BlockQuote: {rendered:?}"
    );
    // Warning BlockQuote must use the centered ┃ (U+2503) glyph so the
    // bar aligns under the centered ⚠ Warning icon. The left-aligned
    // ▌ (U+258C) used by thinking blocks must NOT appear here.
    assert!(
        rendered.contains("\u{2503}"),
        "tracing BlockQuote must use the centered ┃ border: {rendered:?}"
    );
    assert!(
        !rendered.contains("\u{258c}"),
        "tracing BlockQuote must not use the left-aligned ▌ border: {rendered:?}"
    );
}

#[test]
fn reasoning_then_tool_call_transitions_with_single_blank() {
    let lines = Arc::new(StdMutex::new(Vec::new()));
    let dispatched = Arc::new(StdMutex::new(Vec::new()));
    let mut sink = make_sink(lines.clone(), dispatched.clone());
    sink.on_semantic_event(SemanticEvent::Reasoning {
        text: "planning".into(),
        extra: json!({}),
    });
    sink.on_semantic_event(SemanticEvent::ToolCall {
        name: Some("Bash".into()),
        id: None,
        input: Some(json!({"command": "ls"})),
        extra: json!({}),
    });
    let collected = lines.lock().unwrap().clone();
    let mut prev_blank = false;
    for line in &collected {
        let is_blank = line.trim().is_empty();
        assert!(
            !(is_blank && prev_blank),
            "two consecutive blank lines in reasoning→tool transition: {collected:?}"
        );
        prev_blank = is_blank;
    }
}

#[test]
fn read_tool_call_renders_path_as_cwd_relative_blue_link() {
    let lines = Arc::new(StdMutex::new(Vec::new()));
    let repo = tempfile::tempdir().unwrap();
    let cwd = repo.path();
    let source = cwd.join("src/main.rs");
    let mut sink = make_sink_with_cwd(lines.clone(), cwd);
    sink.on_semantic_event(SemanticEvent::ToolCall {
        name: Some("Read".into()),
        id: Some("t1".into()),
        input: Some(json!({"file_path": source.display().to_string()})),
        extra: json!({}),
    });
    let rendered = lines.lock().unwrap().join("\n");
    assert!(
        rendered.contains("Read("),
        "expected Read(...) form: {rendered:?}"
    );
    assert!(
        rendered.contains("src/main.rs"),
        "expected cwd-relative visible path: {rendered:?}"
    );
    // Link is rendered in blue (ANSI 34). Terminals without OSC8
    // support drop the hyperlink wrapper but keep the blue colour,
    // which is what biscuit-terminal emits in the test harness.
    assert!(
        rendered.contains("\u{1b}[34m"),
        "file-tool path must render with blue styling: {rendered:?}"
    );
    // The visible label (the part inside `[...]` for markdown-link
    // fallback, or the OSC8 link text for terminals with hyperlink
    // support) must be the cwd-relative path. The link target may
    // legitimately carry the absolute `file://` URL — that is metadata,
    // not visible text — so we only forbid the absolute path from
    // appearing as the bracket label.
    let absolute_label = format!("[{}]", biscuit_file::to_portable_string(&source));
    assert!(
        !rendered.contains(&absolute_label),
        "absolute path must not appear as visible label: {rendered:?}"
    );
}

#[test]
fn read_tool_success_result_includes_path_link_in_slot() {
    let lines = Arc::new(StdMutex::new(Vec::new()));
    let cwd = Path::new("/repo");
    let mut sink = make_sink_with_cwd(lines.clone(), cwd);
    sink.on_semantic_event(SemanticEvent::ToolResult {
        name: Some("Read".into()),
        id: Some("t1".into()),
        status: Some("success".into()),
        exit_code: None,
        output: Some(json!({"file_path": "/repo/src/main.rs"})),
        extra: json!({"input": {"file_path": "/repo/src/main.rs"}}),
    });
    let rendered = lines.lock().unwrap().join("\n");
    assert!(
        rendered.contains("successful"),
        "success result must retain 'successful' label: {rendered:?}"
    );
    assert!(
        rendered.contains("src/main.rs"),
        "success result must surface the file path: {rendered:?}"
    );
}

#[test]
fn read_tool_error_renders_warning_header_and_blockquote() {
    let lines = Arc::new(StdMutex::new(Vec::new()));
    let cwd = Path::new("/repo");
    let mut sink = make_sink_with_cwd(lines.clone(), cwd);
    let error_body = "File content (30485 tokens) exceeds maximum allowed tokens (25000). \
        Use offset and limit parameters to read specific portions of the file.";
    sink.on_semantic_event(SemanticEvent::ToolResult {
        name: Some("Read".into()),
        id: Some("t1".into()),
        status: Some("error".into()),
        exit_code: None,
        output: Some(Value::String(error_body.to_string())),
        extra: json!({"input": {"file_path": "/repo/src/args.rs"}, "error": error_body}),
    });
    let rendered = lines.lock().unwrap().join("\n");
    assert!(
        rendered.contains("Read("),
        "header should read like a Read() call: {rendered:?}"
    );
    assert!(
        rendered.contains("error"),
        "header should carry 'error' label: {rendered:?}"
    );
    // The header is styled red-bold for the 'error' label.
    assert!(
        rendered.contains("\u{1b}[31m"),
        "'error' label must render red: {rendered:?}"
    );
    assert!(
        rendered.contains("src/args.rs"),
        "header should carry the relative path: {rendered:?}"
    );
    // BlockQuote body uses the centered ┃ (U+2503) glyph to align
    // under the warning status icon.
    assert!(
        rendered.contains("\u{2503}"),
        "body should render inside a centered BlockQuote: {rendered:?}"
    );
    assert!(
        rendered.contains("exceeds maximum allowed tokens"),
        "error body text must appear verbatim: {rendered:?}"
    );
}

#[test]
fn task_progress_suppressed_when_followed_by_matching_read_call() {
    let lines = Arc::new(StdMutex::new(Vec::new()));
    let cwd = Path::new("/repo");
    let mut sink = make_sink_with_cwd(lines.clone(), cwd);
    sink.on_semantic_event(SemanticEvent::Info {
        message: "Reading src/args.rs".into(),
        extra: json!({"type": "task_progress", "message": "Reading src/args.rs"}),
    });
    sink.on_semantic_event(SemanticEvent::ToolCall {
        name: Some("Read".into()),
        id: Some("t1".into()),
        input: Some(json!({"file_path": "/repo/src/args.rs"})),
        extra: json!({}),
    });
    let rendered = lines.lock().unwrap().join("\n");
    assert!(
        !rendered.contains("Reading src/args.rs"),
        "redundant task_progress narration must be suppressed: {rendered:?}"
    );
    assert!(
        rendered.contains("Read("),
        "the follow-up Read tool call must still render: {rendered:?}"
    );
}

#[test]
fn task_progress_flushed_when_followed_by_unrelated_tool_call() {
    let lines = Arc::new(StdMutex::new(Vec::new()));
    let cwd = Path::new("/repo");
    let mut sink = make_sink_with_cwd(lines.clone(), cwd);
    sink.on_semantic_event(SemanticEvent::Info {
        message: "Running git status".into(),
        extra: json!({"type": "task_progress", "message": "Running git status"}),
    });
    sink.on_semantic_event(SemanticEvent::ToolCall {
        name: Some("Read".into()),
        id: Some("t1".into()),
        input: Some(json!({"file_path": "/repo/Cargo.toml"})),
        extra: json!({}),
    });
    let rendered = lines.lock().unwrap().join("\n");
    assert!(
        rendered.contains("Running git status"),
        "unrelated task_progress must be flushed before the tool call: {rendered:?}"
    );
    assert!(
        rendered.contains("Cargo.toml"),
        "tool call must still render: {rendered:?}"
    );
}

#[test]
fn task_progress_flushed_on_drop_if_no_matching_tool_follows() {
    let lines = Arc::new(StdMutex::new(Vec::new()));
    let cwd = Path::new("/repo");
    {
        let mut sink = make_sink_with_cwd(lines.clone(), cwd);
        sink.on_semantic_event(SemanticEvent::Info {
            message: "Reading src/main.rs".into(),
            extra: json!({"type": "task_progress", "message": "Reading src/main.rs"}),
        });
    }
    let rendered = lines.lock().unwrap().join("\n");
    assert!(
        rendered.contains("Reading src/main.rs"),
        "pending task_progress must be flushed on sink drop: {rendered:?}"
    );
}

#[test]
fn task_progress_for_non_claude_provider_renders_immediately() {
    let lines = Arc::new(StdMutex::new(Vec::new()));
    let dispatch = Box::new(|_: AgenticEvent, _: DispatchEventMeta| {});
    let emit = {
        let lines = lines.clone();
        Box::new(move |line: &str| {
            lines.lock().unwrap().push(line.to_string());
        })
    };
    let mut sink = LiveSemanticSink::new(
        Provider::Codex,
        EnvironmentContext::default(),
        Path::new("/repo"),
        Verbosity::Normal,
        Arc::new(Mutex::new(StructuredSummaryDetails::default())),
        dispatch,
        emit,
    );
    sink.on_semantic_event(SemanticEvent::Info {
        message: "Reading src/args.rs".into(),
        extra: json!({"type": "task_progress"}),
    });
    sink.on_semantic_event(SemanticEvent::ToolCall {
        name: Some("read_file".into()),
        id: Some("t1".into()),
        input: Some(json!({"file_path": "/repo/src/args.rs"})),
        extra: json!({}),
    });
    let rendered = lines.lock().unwrap().join("\n");
    assert!(
        rendered.contains("Reading src/args.rs"),
        "non-Claude task_progress must render unchanged: {rendered:?}"
    );
}

#[test]
fn pending_matches_tool_call_requires_minimum_shared_overlap() {
    // Short progress + short summary must NOT auto-match.
    let event = SemanticEvent::ToolCall {
        name: Some("Read".into()),
        id: None,
        input: Some(json!({"file_path": "/a"})),
        extra: json!({}),
    };
    assert!(!super::pending_matches_tool_call("Reading b", &event));
}

#[test]
fn strip_progress_verb_removes_known_prefixes() {
    assert_eq!(
        super::strip_progress_verb("Reading src/main.rs"),
        "src/main.rs"
    );
    assert_eq!(
        super::strip_progress_verb("Running git status"),
        "git status"
    );
    assert_eq!(super::strip_progress_verb("Writing foo"), "foo");
    // Unknown verbs pass through untouched.
    assert_eq!(
        super::strip_progress_verb("Pondering life"),
        "Pondering life"
    );
}

#[test]
fn subagent_start_inserts_active_entry_via_on_semantic_event() {
    let lines = Arc::new(StdMutex::new(Vec::new()));
    let dispatched = Arc::new(StdMutex::new(Vec::new()));
    let mut sink = make_sink(lines.clone(), dispatched.clone());
    sink.on_semantic_event(SemanticEvent::SubagentStart {
        name: Some("researcher".into()),
        id: Some("sa1".into()),
        extra: json!({}),
    });
    let state = sink.watchdog_state.lock().unwrap();
    let active = state.active_subagents(std::time::Instant::now());
    assert_eq!(active.len(), 1);
    assert_eq!(active[0].id, "sa1");
    assert_eq!(active[0].name.as_deref(), Some("researcher"));
}

#[test]
fn subagent_stop_removes_active_entry_via_on_semantic_event() {
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
    let state = sink.watchdog_state.lock().unwrap();
    let active = state.active_subagents(std::time::Instant::now());
    assert!(active.is_empty());
}

#[test]
fn opencode_progress_payload_resets_last_progress_at() {
    let lines = Arc::new(StdMutex::new(Vec::new()));
    let dispatched = Arc::new(StdMutex::new(Vec::new()));
    let mut sink = make_sink(lines.clone(), dispatched.clone());
    sink.on_semantic_event(SemanticEvent::SubagentStart {
        name: Some("researcher".into()),
        id: Some("sa1".into()),
        extra: json!({}),
    });
    // Small delay to ensure progress timestamp moves forward
    std::thread::sleep(std::time::Duration::from_millis(10));
    sink.on_semantic_event(SemanticEvent::Info {
        message: "working".into(),
        extra: json!({"task_id": "sa1"}),
    });
    let state = sink.watchdog_state.lock().unwrap();
    let active = state.active_subagents(std::time::Instant::now());
    assert_eq!(active.len(), 1);
    // elapsed_since_progress should be very small (< 1s) because the
    // Info event with task_id reset last_progress_at just now.
    assert!(
        active[0].elapsed_since_progress < std::time::Duration::from_secs(1),
        "progress should have been reset by the Info event: {:?}",
        active[0].elapsed_since_progress
    );
}

#[test]
fn unknown_subagent_id_in_progress_is_silently_ignored() {
    let lines = Arc::new(StdMutex::new(Vec::new()));
    let dispatched = Arc::new(StdMutex::new(Vec::new()));
    let mut sink = make_sink(lines.clone(), dispatched.clone());
    // No subagent started yet — progress for unknown id must not panic
    sink.on_semantic_event(SemanticEvent::Info {
        message: "working".into(),
        extra: json!({"task_id": "unknown"}),
    });
    let state = sink.watchdog_state.lock().unwrap();
    let active = state.active_subagents(std::time::Instant::now());
    assert!(active.is_empty());
}

#[test]
fn provider_extension_with_task_id_updates_progress() {
    let lines = Arc::new(StdMutex::new(Vec::new()));
    let dispatched = Arc::new(StdMutex::new(Vec::new()));
    let mut sink = make_sink(lines.clone(), dispatched.clone());
    sink.on_semantic_event(SemanticEvent::SubagentStart {
        name: Some("worker".into()),
        id: Some("ext1".into()),
        extra: json!({}),
    });
    std::thread::sleep(std::time::Duration::from_millis(10));
    sink.on_semantic_event(SemanticEvent::ProviderExtension {
        provider: Provider::OpenCode,
        kind: "task_progress".into(),
        payload: json!({"task_id": "ext1", "percent": 50}),
    });
    let state = sink.watchdog_state.lock().unwrap();
    let active = state.active_subagents(std::time::Instant::now());
    assert_eq!(active.len(), 1);
    assert!(
        active[0].elapsed_since_progress < std::time::Duration::from_secs(1),
        "progress should have been reset by the ProviderExtension event: {:?}",
        active[0].elapsed_since_progress
    );
}
