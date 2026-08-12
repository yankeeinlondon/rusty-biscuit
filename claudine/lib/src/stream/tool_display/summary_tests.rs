use super::*;
use serde_json::json;

#[test]
fn web_search_extracts_query() {
    let input = json!({"query": "NFL draft 2026 date", "limit": 5});
    assert_eq!(
        extract_tool_summary("firecrawl_firecrawl_search", &input).as_deref(),
        Some("NFL draft 2026 date")
    );
}

#[test]
fn bash_extracts_command() {
    let input = json!({"command": "ls -la"});
    assert_eq!(
        extract_tool_summary("Bash", &input).as_deref(),
        Some("bash ls -la")
    );
}

#[test]
fn read_extracts_file_path() {
    let input = json!({"file_path": "/etc/hosts"});
    assert_eq!(
        extract_tool_summary("Read", &input).as_deref(),
        Some("/etc/hosts")
    );
}

#[test]
fn unknown_tool_falls_back_to_first_string() {
    let input = json!({"weirdo": "interesting", "n": 5});
    assert_eq!(
        extract_tool_summary("custom_unknown", &input).as_deref(),
        Some("interesting")
    );
}

#[test]
fn falls_back_to_raw_json_for_object_with_no_strings() {
    let input = json!({"a": 1, "b": [1, 2]});
    let rendered = extract_tool_summary("custom_unknown", &input).expect("raw JSON fallback");
    // Parse both ends and compare semantically so we don't depend on
    // serde_json's key-ordering behavior.
    let roundtrip: serde_json::Value = serde_json::from_str(&rendered).unwrap();
    assert_eq!(roundtrip, input);
}

#[test]
fn returns_none_for_null_or_empty_object() {
    assert!(extract_tool_summary("custom_unknown", &json!(null)).is_none());
    assert!(extract_tool_summary("custom_unknown", &json!({})).is_none());
}

#[test]
fn falls_back_to_raw_json_for_array_input() {
    let input = json!([1, 2, 3]);
    let rendered = extract_tool_summary("custom_unknown", &input).unwrap();
    assert_eq!(rendered, "[1,2,3]");
}

#[test]
fn bash_summary_prepends_shell_name() {
    let input = json!({"command": "ls -la"});
    assert_eq!(
        extract_tool_summary("Bash", &input).as_deref(),
        Some("bash ls -la"),
        "Bash summary must prepend the shell name so users see the invoked shell"
    );
}

#[test]
fn lowercase_bash_summary_prepends_shell_name() {
    let input = json!({"command": "pwd"});
    assert_eq!(
        extract_tool_summary("bash", &input).as_deref(),
        Some("bash pwd")
    );
}

#[test]
fn codex_shell_summary_prepends_shell_prefix() {
    let input = json!({"command": "echo hi"});
    assert_eq!(
        extract_tool_summary("shell", &input).as_deref(),
        Some("shell echo hi")
    );
}

#[test]
fn codex_concrete_shell_summary_does_not_duplicate_shell_name() {
    // The Codex protocol layer already resolves concrete shells to a
    // humanized tool name (`Zsh(...)`, `Bash(...)`, `Fish(...)`), and
    // strips the leading `/bin/<shell>` path from the command. The
    // summary inside the parentheses should NOT add another copy of the
    // shell name — `Zsh(zsh -lc ...)` reads worse than
    // `Zsh(-lc ...)`.
    for shell in ["zsh", "sh", "fish", "dash", "ksh"] {
        let input = json!({"command": "-lc 'sed -n 1,5p file'"});
        assert_eq!(
            extract_tool_summary(shell, &input).as_deref(),
            Some("-lc 'sed -n 1,5p file'"),
            "{shell} summary must not duplicate the shell name inside the parens"
        );
    }
}

#[test]
fn task_extracts_description_first() {
    let input = json!({
        "description": "Review the plan",
        "subject": "Planning review",
        "prompt": "Please review the plan...",
        "task": "review",
        "subagent_type": "general-purpose",
    });
    assert_eq!(
        extract_tool_summary("Task", &input).as_deref(),
        Some("Review the plan"),
        "Task extractor must prefer description over other fields"
    );
}

#[test]
fn task_falls_back_to_subject_when_description_absent() {
    let input = json!({
        "subject": "Planning review",
        "prompt": "Please review the plan...",
        "task": "review",
    });
    assert_eq!(
        extract_tool_summary("Task", &input).as_deref(),
        Some("Planning review")
    );
}

#[test]
fn task_falls_back_to_prompt_when_description_absent() {
    let input = json!({
        "prompt": "Please review the plan in detail.",
        "task": "review",
        "subagent_type": "general-purpose",
    });
    assert_eq!(
        extract_tool_summary("Task", &input).as_deref(),
        Some("Please review the plan in detail."),
        "Task extractor must fall back to prompt when description and subject are absent"
    );
}

#[test]
fn task_falls_back_to_task_field_as_last_resort() {
    let input = json!({
        "task": "review the plan",
        "subagent_type": "general-purpose",
    });
    assert_eq!(
        extract_tool_summary("Task", &input).as_deref(),
        Some("review the plan")
    );
}
