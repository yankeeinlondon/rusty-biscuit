use super::*;

fn parse(line: &str) -> CodexEvent {
    serde_json::from_str(line).expect("valid CodexEvent")
}

#[test]
fn codex_thread_started_deserializes() {
    let event = parse(r#"{"type":"thread.started","thread_id":"thrd-1"}"#);
    let CodexEvent::ThreadStarted(meta) = event else {
        panic!("expected ThreadStarted");
    };
    assert_eq!(meta.resolved_id(), Some("thrd-1".into()));
}

#[test]
fn codex_thread_created_alias() {
    let event = parse(r#"{"type":"thread.created","id":"thrd-2"}"#);
    let CodexEvent::ThreadCreated(meta) = event else {
        panic!("expected ThreadCreated");
    };
    assert_eq!(meta.resolved_id(), Some("thrd-2".into()));
}

#[test]
fn codex_turn_started_accepts_empty() {
    let event = parse(r#"{"type":"turn.started"}"#);
    assert!(matches!(event, CodexEvent::TurnStarted(_)));
}

#[test]
fn codex_turn_completed_with_usage() {
    let event = parse(
        r#"{"type":"turn.completed","usage":{"input_tokens":200,"output_tokens":100,"cached_input_tokens":50},"duration_ms":5000,"status":"completed"}"#,
    );
    let CodexEvent::TurnCompleted(tc) = event else {
        panic!("expected TurnCompleted");
    };
    assert_eq!(tc.duration_ms, Some(5000));
    assert_eq!(tc.provider_status(), Some("completed"));
    let usage = tc.usage.expect("usage");
    assert_eq!(usage.input_tokens, Some(200));
    assert_eq!(usage.output_tokens, Some(100));
    assert_eq!(usage.cache_read(), Some(50));
}

#[test]
fn codex_error_flat_fields() {
    let event = parse(
        r#"{"type":"error","error_type":"rate_limit","error_message":"Too many requests"}"#,
    );
    let CodexEvent::Error(err) = event else {
        panic!("expected Error");
    };
    assert_eq!(err.resolved_kind(), Some("rate_limit".into()));
    assert_eq!(err.resolved_message(), Some("Too many requests".into()));
}

#[test]
fn codex_error_nested_object() {
    let event = parse(
        r#"{"type":"stream.error","error":{"type":"network","message":"socket closed"}}"#,
    );
    let CodexEvent::StreamError(err) = event else {
        panic!("expected StreamError");
    };
    assert_eq!(err.resolved_kind(), Some("network".into()));
    assert_eq!(err.resolved_message(), Some("socket closed".into()));
}

#[test]
fn codex_item_started_agent_message() {
    let event = parse(
        r#"{"type":"item.started","item":{"id":"item_0","type":"agent_message","text":"hi"}}"#,
    );
    let CodexEvent::ItemStarted(env) = event else {
        panic!("expected ItemStarted");
    };
    let item = env.item.expect("item");
    let msg = item.as_agent_message().expect("agent_message");
    assert_eq!(msg.id.as_deref(), Some("item_0"));
    assert_eq!(msg.text.as_deref(), Some("hi"));
    assert_eq!(msg.collected_text(), Some("hi".into()));
}

#[test]
fn codex_item_completed_tool_use() {
    let event = parse(
        r#"{"type":"item.completed","item":{"id":"tu-1","type":"tool_use","tool_name":"bash","input":{"command":"ls"},"output":"ok"}}"#,
    );
    let CodexEvent::ItemCompleted(env) = event else {
        panic!("expected ItemCompleted");
    };
    let item = env.item.expect("item");
    assert!(item.is_tool_item());
    let fields = item.as_tool_fields().expect("tool fields");
    assert_eq!(fields.resolved_tool_name(), Some("bash"));
    let input = fields.resolved_input();
    assert_eq!(
        input
            .as_ref()
            .and_then(|v| v.get("command"))
            .and_then(Value::as_str),
        Some("ls")
    );
    let output = fields.resolved_output();
    assert_eq!(output.as_ref().and_then(Value::as_str), Some("ok"));
}

#[test]
fn codex_item_completed_typed_agent_message_content() {
    let event = parse(
        r#"{"type":"item.completed","item":{"id":"item_0","type":"agent_message","content":[{"type":"text","text":"Hi "},{"type":"text","text":"there"}]}}"#,
    );
    let CodexEvent::ItemCompleted(env) = event else {
        panic!("expected ItemCompleted");
    };
    let item = env.item.expect("item");
    let msg = item.as_agent_message().expect("agent_message");
    let parts = msg.content.as_ref().expect("content");
    assert_eq!(parts.len(), 2);
    assert_eq!(parts[0].kind.as_deref(), Some("text"));
    assert_eq!(parts[0].text.as_deref(), Some("Hi "));
    assert_eq!(msg.collected_text(), Some("Hi there".into()));
}

#[test]
fn codex_item_permission_request_typed() {
    let event = parse(
        r#"{"type":"item.started","item":{"id":"perm-1","type":"permission_request","name":"bash"}}"#,
    );
    let CodexEvent::ItemStarted(env) = event else {
        panic!("expected ItemStarted");
    };
    let item = env.item.expect("item");
    assert!(item.is_permission_item());
    let perm = item.as_permission().expect("permission");
    assert_eq!(perm.id.as_deref(), Some("perm-1"));
    assert_eq!(perm.name.as_deref(), Some("bash"));
}

#[test]
fn codex_item_unknown_kind_falls_back() {
    let event =
        parse(r#"{"type":"item.started","item":{"id":"x","type":"some_brand_new_kind"}}"#);
    let CodexEvent::ItemStarted(env) = event else {
        panic!("expected ItemStarted");
    };
    let item = env.item.expect("item");
    assert!(matches!(item, CodexItem::Unknown));
}

#[test]
fn codex_top_level_tool_use_deserializes() {
    let event = parse(r#"{"type":"item.tool_use","name":"bash"}"#);
    let CodexEvent::ItemToolUse(fields) = event else {
        panic!("expected ItemToolUse");
    };
    assert_eq!(fields.resolved_tool_name(), Some("bash"));
}

#[test]
fn codex_top_level_tool_result_deserializes() {
    let event = parse(r#"{"type":"item.tool_result","status":"ok"}"#);
    assert!(matches!(event, CodexEvent::ItemToolResult(_)));
}

#[test]
fn codex_merge_started_populates_missing_fields() {
    let started = CodexItem::ToolUse(CodexToolItemFields {
        id: Some("tu-1".into()),
        name: Some("bash".into()),
        input: Some(serde_json::json!({"command": "ls"})),
        ..Default::default()
    });
    let completed = CodexItem::ToolUse(CodexToolItemFields {
        id: Some("tu-1".into()),
        output: Some(Value::String("clean".into())),
        ..Default::default()
    });
    let merged = completed.merge_started(started);
    let fields = merged.as_tool_fields().expect("tool fields");
    assert_eq!(fields.name.as_deref(), Some("bash"));
    let input = fields.resolved_input();
    assert_eq!(
        input
            .as_ref()
            .and_then(|v| v.get("command"))
            .and_then(Value::as_str),
        Some("ls")
    );
    let output = fields.resolved_output();
    assert_eq!(output.as_ref().and_then(Value::as_str), Some("clean"));
}

#[test]
fn codex_unknown_event_type_fails_typed() {
    let err = serde_json::from_str::<CodexEvent>(r#"{"type":"session.not_a_real_event"}"#);
    assert!(err.is_err());
}

#[test]
fn codex_command_execution_started_deserializes() {
    let line = r#"{"type":"item.started","item":{"id":"item_1","type":"command_execution","command":"/bin/zsh -lc 'ls'","aggregated_output":""}}"#;
    let event: CodexEvent = serde_json::from_str(line).expect("valid event");
    let CodexEvent::ItemStarted(env) = event else {
        panic!("expected ItemStarted");
    };
    let item = env.item.expect("item");
    assert!(
        matches!(item, CodexItem::CommandExec(_)),
        "expected CommandExec variant (with command_execution alias), got {item:?}"
    );
    let fields = item.as_tool_fields().expect("tool fields");
    let input = fields.resolved_input();
    let extracted = input
        .as_ref()
        .and_then(|v| v.as_str().map(String::from))
        .or_else(|| {
            input
                .as_ref()
                .and_then(|v| v.get("command"))
                .and_then(|v| v.as_str())
                .map(String::from)
        });
    assert_eq!(
        extracted.as_deref(),
        Some("-lc 'ls'"),
        "command must be exposed via resolved_input with the `/bin/<shell>` path stripped so the rendered summary reads as `zsh -lc '…'`: fields = {fields:?}"
    );
}

#[test]
fn codex_file_change_completion_with_changes_array() {
    let line = r#"{"type":"item.completed","item":{"id":"item_4","type":"file_change","changes":[{"path":"src/lib.rs","kind":"update"},{"path":"tests/smoke.rs","kind":"add"}],"status":"completed"}}"#;
    let event: CodexEvent = serde_json::from_str(line).expect("valid event");
    let CodexEvent::ItemCompleted(env) = event else {
        panic!("expected ItemCompleted");
    };
    let item = env.item.expect("item");
    let CodexItem::FileChange(fc) = item else {
        panic!("expected FileChange item");
    };
    assert_eq!(fc.status.as_deref(), Some("completed"));
    let entries = fc.resolved_entries();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].0.as_deref(), Some("src/lib.rs"));
    assert_eq!(entries[0].1.as_deref(), Some("update"));
    assert_eq!(entries[1].0.as_deref(), Some("tests/smoke.rs"));
    assert_eq!(entries[1].1.as_deref(), Some("add"));
    assert_eq!(fc.resolved_path(), Some("src/lib.rs"));
    assert_eq!(fc.resolved_kind(), Some("update"));
}

#[test]
fn codex_file_change_flat_fields_fallback() {
    let line = r#"{"type":"item.completed","item":{"id":"f1","type":"file_change","path":"src/lib.rs","change_kind":"modified"}}"#;
    let event: CodexEvent = serde_json::from_str(line).expect("valid event");
    let CodexEvent::ItemCompleted(env) = event else {
        panic!("expected ItemCompleted");
    };
    let CodexItem::FileChange(fc) = env.item.expect("item") else {
        panic!("expected FileChange item");
    };
    let entries = fc.resolved_entries();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].0.as_deref(), Some("src/lib.rs"));
    assert_eq!(entries[0].1.as_deref(), Some("modified"));
}

#[test]
fn codex_file_change_empty_payload_produces_no_entries() {
    let line = r#"{"type":"item.started","item":{"id":"f1","type":"file_change"}}"#;
    let event: CodexEvent = serde_json::from_str(line).expect("valid event");
    let CodexEvent::ItemStarted(env) = event else {
        panic!("expected ItemStarted");
    };
    let CodexItem::FileChange(fc) = env.item.expect("item") else {
        panic!("expected FileChange item");
    };
    assert!(
        fc.resolved_entries().is_empty(),
        "empty file_change must yield zero entries"
    );
}

#[test]
fn codex_command_execution_completed_exposes_output_and_status() {
    let line = r#"{"type":"item.completed","item":{"id":"item_1","type":"command_execution","command":"ls","aggregated_output":"file.txt\n","exit_code":0,"status":"success"}}"#;
    let event: CodexEvent = serde_json::from_str(line).expect("valid event");
    let CodexEvent::ItemCompleted(env) = event else {
        panic!("expected ItemCompleted");
    };
    let item = env.item.expect("item");
    let fields = item.as_tool_fields().expect("tool fields");
    assert_eq!(fields.exit_code, Some(0));
    assert_eq!(fields.status.as_deref(), Some("success"));
    let output = fields.resolved_output().expect("output");
    let s = output.as_str().map(String::from).or_else(|| {
        output
            .get("aggregated_output")
            .and_then(|v| v.as_str())
            .map(String::from)
    });
    assert_eq!(
        s.as_deref(),
        Some("file.txt\n"),
        "aggregated_output must be exposed via resolved_output"
    );
}

#[test]
fn detect_shell_from_command_recognizes_absolute_paths() {
    assert_eq!(detect_shell_from_command("/bin/zsh -lc 'ls'"), Some("zsh"));
    assert_eq!(
        detect_shell_from_command("/usr/local/bin/bash -c 'ls'"),
        Some("bash")
    );
    assert_eq!(detect_shell_from_command("/bin/fish -c 'ls'"), Some("fish"));
}

#[test]
fn detect_shell_from_command_returns_none_for_non_shell_commands() {
    assert_eq!(detect_shell_from_command("ls -la"), None);
    assert_eq!(detect_shell_from_command("/usr/bin/git status"), None);
    assert_eq!(detect_shell_from_command(""), None);
}

#[test]
fn strip_shell_path_prefix_removes_absolute_shell_token() {
    assert_eq!(
        strip_shell_path_prefix("/bin/zsh -lc 'sed -n 1,5p file'"),
        "-lc 'sed -n 1,5p file'"
    );
    assert_eq!(
        strip_shell_path_prefix("/usr/local/bin/bash -c 'ls -la'"),
        "-c 'ls -la'"
    );
}

#[test]
fn strip_shell_path_prefix_preserves_commands_without_shell_path() {
    assert_eq!(strip_shell_path_prefix("ls -la"), "ls -la");
    assert_eq!(strip_shell_path_prefix("git status"), "git status");
    // Bare `zsh` without a path prefix is preserved — the prefix handler
    // only activates on absolute paths so the "zsh -lc ..." summary
    // emitted by tool_display still reads sensibly.
    assert_eq!(strip_shell_path_prefix("zsh -lc 'x'"), "zsh -lc 'x'");
}

#[test]
fn codex_command_execution_resolves_shell_name_from_command() {
    let line = r#"{"type":"item.started","item":{"id":"cmd1","type":"command_execution","command":"/bin/zsh -lc 'ls'"}}"#;
    let event: CodexEvent = serde_json::from_str(line).expect("valid event");
    let CodexEvent::ItemStarted(env) = event else {
        panic!("expected ItemStarted");
    };
    let fields = env.item.expect("item").as_tool_fields().cloned_via_merge();
    assert_eq!(fields.resolved_tool_name(), Some("zsh"));
    let input = fields.resolved_input().expect("synthesized input");
    assert_eq!(
        input.get("command").and_then(Value::as_str),
        Some("-lc 'ls'"),
        "shell path prefix must be stripped from the synthesized command",
    );
}

#[test]
fn item_completed_round_trips_through_json() {
    let line = r#"{"type":"item.completed","item":{"id":"tu-1","type":"tool_use","tool_name":"bash","input":{"command":"ls"},"output":"ok"}}"#;
    let event = parse(line);
    let serialized = serde_json::to_string(&event).unwrap();
    let reparsed: CodexEvent = serde_json::from_str(&serialized).unwrap();
    assert_eq!(
        serde_json::to_string(&reparsed).unwrap(),
        serialized,
        "parse -> serialize -> parse should be stable for a known event"
    );
}

#[test]
fn turn_completed_known_payload_keeps_extra_empty() {
    let line = r#"{"type":"turn.completed","usage":{"input_tokens":10,"output_tokens":5},"duration_ms":42,"status":"completed"}"#;
    let event = parse(line);
    let CodexEvent::TurnCompleted(tc) = event else {
        panic!("expected TurnCompleted");
    };
    assert!(
        tc.extra.is_empty(),
        "known turn.completed fields must not land in extra; extra={:?}",
        tc.extra
    );
}

// Tiny helper so the test above can borrow-then-clone the fields without
// adding a public clone method to the production API.
trait CloneViaMerge {
    fn cloned_via_merge(self) -> CodexToolItemFields;
}
impl CloneViaMerge for Option<&CodexToolItemFields> {
    fn cloned_via_merge(self) -> CodexToolItemFields {
        let src = self.expect("tool fields");
        let mut dst = CodexToolItemFields::default();
        dst.merge_started(CodexToolItemFields {
            id: src.id.clone(),
            name: src.name.clone(),
            tool_name: src.tool_name.clone(),
            input: src.input.clone(),
            arguments: src.arguments.clone(),
            parameters: src.parameters.clone(),
            output: src.output.clone(),
            result: src.result.clone(),
            content: src.content.clone(),
            status: src.status.clone(),
            exit_code: src.exit_code,
            command: src.command.clone(),
            aggregated_output: src.aggregated_output.clone(),
        });
        dst
    }
}
