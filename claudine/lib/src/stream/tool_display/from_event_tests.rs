use super::*;
use serde_json::json;

#[test]
fn from_call_humanizes_and_extracts_query() {
    let event = SemanticEvent::ToolCall {
        name: Some("firecrawl_firecrawl_search".into()),
        id: None,
        input: Some(json!({"query": "NFL"})),
        extra: json!({}),
    };
    let display = ToolCallDisplay::from_call(&event).unwrap();
    assert_eq!(display.direction, ToolDirection::Outgoing);
    assert_eq!(display.display_name, "Firecrawl Search");
    assert_eq!(display.summary.as_deref(), Some("NFL"));
    assert!(display.status.is_none());
}

#[test]
fn from_result_keeps_summary_alongside_status_for_shell_success() {
    // Per the 2026-04-18 OpenCode reporting contract, a successful
    // shell result must surface the same slot content the outgoing
    // arrow carried. Status and summary are co-equal: both populate
    // so the rendered line reads `← Bash(successful, bash ls -la)`.
    let event = SemanticEvent::ToolResult {
        name: Some("Bash".into()),
        id: None,
        status: Some("success".into()),
        exit_code: None,
        output: None,
        extra: json!({"input": {"command": "ls -la"}}),
    };
    let display = ToolCallDisplay::from_result(&event).unwrap();
    assert_eq!(display.status, Some(ToolStatus::Success));
    assert_eq!(
        display.summary.as_deref(),
        Some("bash ls -la"),
        "successful shell results must keep the cached command summary"
    );
}

#[test]
fn from_result_falls_back_to_output_when_input_absent() {
    // Unknown / synthetic tools without a cached input still surface a
    // best-effort output-derived summary so the slot is not silently
    // dropped.
    let event = SemanticEvent::ToolResult {
        name: Some("Bash".into()),
        id: None,
        status: Some("success".into()),
        exit_code: None,
        output: Some(json!({"command": "pwd"})),
        extra: json!({}),
    };
    let display = ToolCallDisplay::from_result(&event).unwrap();
    assert_eq!(display.status, Some(ToolStatus::Success));
    assert_eq!(display.summary.as_deref(), Some("bash pwd"));
}

#[test]
fn from_result_uses_extra_tool_name_when_name_missing() {
    let event = SemanticEvent::ToolResult {
        name: None,
        id: None,
        status: Some("success".into()),
        exit_code: None,
        output: None,
        extra: json!({"tool_name": "Bash"}),
    };
    let display = ToolCallDisplay::from_result(&event).unwrap();
    assert_eq!(display.display_name, "Bash");
    assert_eq!(display.status, Some(ToolStatus::Success));
}

#[test]
fn from_result_falls_back_to_summary_when_status_absent() {
    let event = SemanticEvent::ToolResult {
        name: Some("Bash".into()),
        id: None,
        status: None,
        exit_code: None,
        output: Some(json!({"command": "ls"})),
        extra: json!({}),
    };
    let display = ToolCallDisplay::from_result(&event).unwrap();
    assert!(display.status.is_none());
    assert_eq!(display.summary.as_deref(), Some("bash ls"));
}

#[test]
fn from_result_maps_timeout_and_cancelled_to_error() {
    for raw in ["timeout", "cancelled", "aborted"] {
        let event = SemanticEvent::ToolResult {
            name: Some("Bash".into()),
            id: None,
            status: Some(raw.into()),
            exit_code: None,
            output: None,
            extra: json!({}),
        };
        let display = ToolCallDisplay::from_result(&event).unwrap();
        assert_eq!(
            display.status,
            Some(ToolStatus::Error),
            "raw status {raw:?} should map to Error"
        );
    }
}

#[test]
fn from_result_unknown_status_falls_through_to_summary() {
    let event = SemanticEvent::ToolResult {
        name: Some("Bash".into()),
        id: None,
        status: Some("xyz".into()),
        exit_code: None,
        output: Some(json!({"command": "ls"})),
        extra: json!({}),
    };
    let display = ToolCallDisplay::from_result(&event).unwrap();
    assert!(display.status.is_none());
    assert_eq!(display.summary.as_deref(), Some("bash ls"));
}

#[test]
fn from_result_uses_extra_input_when_output_absent() {
    let event = SemanticEvent::ToolResult {
        name: Some("Bash".into()),
        id: None,
        status: None,
        exit_code: None,
        output: None,
        extra: json!({"input": {"command": "ls -la"}}),
    };
    let display = ToolCallDisplay::from_result(&event).unwrap();
    assert_eq!(display.summary.as_deref(), Some("bash ls -la"));
}

#[test]
fn from_result_error_detail_combines_exit_code_and_last_line() {
    let event = SemanticEvent::ToolResult {
        name: Some("shell".into()),
        id: None,
        status: Some("failure".into()),
        exit_code: Some(1),
        output: Some(json!(
            "sed: 1,260p: command not found\nsed: invalid range\n"
        )),
        extra: json!({}),
    };
    let display = ToolCallDisplay::from_result(&event).unwrap();
    assert_eq!(display.status, Some(ToolStatus::Error));
    assert_eq!(
        display.error_detail.as_deref(),
        Some("exit=1 · sed: invalid range")
    );
}

#[test]
fn from_result_error_detail_reads_aggregated_output_wrapper() {
    let event = SemanticEvent::ToolResult {
        name: Some("shell".into()),
        id: None,
        status: Some("failure".into()),
        exit_code: Some(2),
        output: Some(json!({"aggregated_output": "no matches\n"})),
        extra: json!({}),
    };
    let display = ToolCallDisplay::from_result(&event).unwrap();
    assert_eq!(display.error_detail.as_deref(), Some("exit=2 · no matches"));
}

#[test]
fn from_result_error_detail_falls_back_to_exit_code_when_output_empty() {
    let event = SemanticEvent::ToolResult {
        name: Some("shell".into()),
        id: None,
        status: Some("failure".into()),
        exit_code: Some(127),
        output: None,
        extra: json!({}),
    };
    let display = ToolCallDisplay::from_result(&event).unwrap();
    assert_eq!(display.error_detail.as_deref(), Some("exit=127"));
}

#[test]
fn from_result_error_detail_reads_mcp_error_message() {
    let event = SemanticEvent::ToolResult {
        name: Some("memory".into()),
        id: None,
        status: Some("failed".into()),
        exit_code: None,
        output: None,
        extra: json!({"error": {"message": "user cancelled MCP tool call"}}),
    };
    let display = ToolCallDisplay::from_result(&event).unwrap();
    assert_eq!(
        display.error_detail.as_deref(),
        Some("user cancelled MCP tool call")
    );
}

#[test]
fn from_result_error_detail_absent_for_success_path() {
    let event = SemanticEvent::ToolResult {
        name: Some("shell".into()),
        id: None,
        status: Some("success".into()),
        exit_code: Some(0),
        output: Some(json!("file.txt\n")),
        extra: json!({}),
    };
    let display = ToolCallDisplay::from_result(&event).unwrap();
    assert!(display.error_detail.is_none());
}

#[test]
fn from_result_error_detail_truncates_long_snippets() {
    let long = "x".repeat(300);
    let event = SemanticEvent::ToolResult {
        name: Some("shell".into()),
        id: None,
        status: Some("failure".into()),
        exit_code: Some(1),
        output: Some(json!(long)),
        extra: json!({}),
    };
    let display = ToolCallDisplay::from_result(&event).unwrap();
    let detail = display.error_detail.expect("error_detail");
    assert!(detail.starts_with("exit=1 · "));
    assert!(
        detail.chars().count() <= "exit=1 · ".chars().count() + 160,
        "snippet length must be capped, got {}",
        detail.chars().count()
    );
    assert!(detail.ends_with('\u{2026}'));
}
