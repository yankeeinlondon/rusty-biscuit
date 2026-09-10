//! Cross-adapter replay contract for the provider-neutral final-response sink.

use super::*;
use claudine::stream::summary::StreamExecutionSummary;
use claudine::stream::{ParserConfig, create_semantic_parser};

fn replay_summary(
    provider: Provider,
    lines: &[&str],
) -> (StructuredSummaryDetails, StreamExecutionSummary) {
    let details = Arc::new(Mutex::new(StructuredSummaryDetails::default()));
    let sink = LiveSemanticSink::new(
        provider,
        EnvironmentContext::default(),
        Path::new("/tmp"),
        Verbosity::Quiet,
        details.clone(),
        Box::new(|_event, _meta| {}),
        Box::new(|_line| {}),
    );
    let mut parser = create_semantic_parser(provider, sink, ParserConfig::default());
    for line in lines {
        parser.feed_line(line);
    }
    let summary = parser.finish(0);
    let details = details.lock().unwrap().clone();
    (details, summary)
}

#[test]
fn line_stream_adapters_keep_only_text_after_the_last_tool_activity() {
    let cases: &[(Provider, &[&str], &str, &str)] = &[
        (
            Provider::Claude,
            &[
                r#"{"type":"assistant","content":[{"type":"text","text":"I will inspect first."}]}"#,
                r#"{"type":"tool_use","id":"t1","name":"read","input":{"path":"notes.md"}}"#,
                r#"{"type":"tool_result","tool_use_id":"t1","content":"contents"}"#,
                r#"{"type":"assistant","content":[{"type":"text","text":"Claude trailing summary."}]}"#,
            ],
            "Claude trailing summary.\n",
            "read",
        ),
        (
            Provider::Gemini,
            &[
                r#"{"type":"message","role":"assistant","content":"I will inspect first."}"#,
                r#"{"type":"tool_use","tool_id":"t1","tool_name":"search","parameters":{"q":"notes"}}"#,
                r#"{"type":"tool_result","tool_id":"t1","status":"success","output":{"hits":1}}"#,
                r#"{"type":"message","role":"assistant","content":"Gemini trailing summary."}"#,
            ],
            "Gemini trailing summary.\n",
            "search",
        ),
        (
            Provider::KimiCode,
            &[
                r#"{"jsonrpc":"2.0","method":"event","params":{"type":"ContentPart","payload":{"type":"text","text":"I will inspect first."}}}"#,
                r#"{"jsonrpc":"2.0","method":"event","params":{"type":"ToolCall","payload":{"id":"t1","function":{"name":"Shell","arguments":"{\"command\":\"cat notes.md\"}"}}}}"#,
                r#"{"jsonrpc":"2.0","method":"event","params":{"type":"ToolResult","payload":{"tool_call_id":"t1","return_value":{"is_error":false,"output":"contents"}}}}"#,
                r#"{"jsonrpc":"2.0","method":"event","params":{"type":"ContentPart","payload":{"type":"text","text":"Kimi trailing summary."}}}"#,
            ],
            "Kimi trailing summary.",
            "Shell",
        ),
        (
            Provider::QwenCode,
            &[
                r#"{"type":"message","role":"assistant","content":[{"text":"I will inspect first."}]}"#,
                r#"{"type":"tool_call","id":"t1","name":"bash","input":{"command":"cat notes.md"}}"#,
                r#"{"type":"tool_response","tool_use_id":"t1","status":"success","content":"contents"}"#,
                r#"{"type":"message","role":"assistant","content":[{"text":"Qwen trailing summary."}]}"#,
            ],
            "Qwen trailing summary.\n",
            "bash",
        ),
        (
            Provider::Pi,
            &[
                r#"{"type":"message_update","assistantMessageEvent":{"type":"text_delta","delta":"I will inspect first."}}"#,
                r#"{"type":"tool_execution_start","toolCallId":"t1","toolName":"bash","args":{"command":"cat notes.md"}}"#,
                r#"{"type":"tool_execution_end","toolCallId":"t1","toolName":"bash","result":"contents","isError":false}"#,
                r#"{"type":"message_update","assistantMessageEvent":{"type":"text_delta","delta":"Pi trailing summary."}}"#,
            ],
            "Pi trailing summary.",
            "bash",
        ),
    ];

    for (provider, lines, trailing, tool_name) in cases {
        let (details, summary) = replay_summary(*provider, lines);
        assert_eq!(
            details.final_response, *trailing,
            "{provider:?} retained pre-tool narration"
        );
        assert_eq!(details.tool_names, [*tool_name], "{provider:?} tool names");
        assert_eq!(summary.tool_calls, Some(1), "{provider:?} tool count");
    }
}

#[test]
fn codex_adapter_keeps_final_text_on_its_out_of_band_path() {
    // Codex deliberately renders `agent_message` as reasoning; its final
    // response comes from `--output-last-message`, outside the JSONL adapter.
    let lines = [
        r#"{"type":"item.completed","item":{"id":"a1","type":"agent_message","text":"I will inspect first."}}"#,
        r#"{"type":"item.started","item":{"id":"t1","type":"command_exec","tool_name":"bash","input":{"command":"cat notes.md"}}}"#,
        r#"{"type":"item.completed","item":{"id":"t1","type":"command_exec","status":"success","exit_code":0,"output":"contents"}}"#,
        r#"{"type":"item.completed","item":{"id":"a2","type":"agent_message","text":"Codex trailing summary."}}"#,
    ];
    let (details, summary) = replay_summary(Provider::Codex, &lines);
    assert!(details.final_response.is_empty());
    assert_eq!(details.tool_names, ["bash"]);
    assert_eq!(summary.tool_calls, Some(1));
    assert_eq!(
        summary.assistant_text,
        "I will inspect first.Codex trailing summary."
    );
}

#[test]
fn opencode_and_kilo_completion_only_tools_keep_trailing_text_and_count_once() {
    let lines = [
        r#"{"type":"text","text":"I will inspect first."}"#,
        r#"{"type":"tool_use","part":{"id":"t1","tool":"read","state":{"status":"completed","input":{"path":"notes.md"},"output":"contents"}}}"#,
        r#"{"type":"text","text":"OpenCode trailing summary."}"#,
    ];

    for provider in [Provider::OpenCode, Provider::Kilo] {
        let (details, summary) = replay_summary(provider, &lines);
        assert_eq!(details.final_response, "OpenCode trailing summary.");
        assert_eq!(details.tool_names, ["read"]);
        assert_eq!(summary.tool_calls, Some(1), "{provider:?} tool count");
    }
}

#[test]
fn saved_opencode_voip_replay_keeps_only_its_last_text_part() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../lib/tests/fixtures/providers/opencode-voip-2026-09-05.ndjson");
    let raw = std::fs::read_to_string(path).expect("read saved OpenCode voip transcript");
    let lines: Vec<&str> = raw.lines().collect();
    assert_eq!(lines.len(), 75, "the regression input must remain intact");
    let last_text = lines
        .iter()
        .rev()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .filter(|value| value.get("type").and_then(serde_json::Value::as_str) == Some("text"))
        .filter_map(|value| {
            value
                .get("text")
                .or_else(|| value.pointer("/part/text"))
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
        })
        .next()
        .expect("saved transcript contains trailing assistant text");

    let (details, summary) = replay_summary(Provider::OpenCode, &lines);
    assert_eq!(details.final_response, last_text);
    assert_eq!(details.tool_names, ["skill", "read", "bash", "webfetch"]);
    assert_eq!(summary.tool_calls, Some(23));
}

#[test]
fn antigravity_buffered_adapter_preserves_its_terminal_response() {
    // Antigravity exposes only one terminal envelope and no tool lifecycle,
    // so its adapter cannot represent the text/tool/text sequence exercised by
    // line-stream providers. Its final response is the envelope response.
    let (details, summary) = replay_summary(
        Provider::Antigravity,
        &[r#"{"status":"SUCCESS","response":"Antigravity terminal summary."}"#],
    );
    assert_eq!(details.final_response, "Antigravity terminal summary.");
    assert!(details.tool_names.is_empty());
    assert_eq!(summary.tool_calls, None);
}
