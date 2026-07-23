use super::*;

fn parse(line: &str) -> OpenCodeEvent {
    serde_json::from_str(line).expect("valid OpenCodeEvent")
}

#[test]
fn opencode_init_deserializes() {
    let event = parse(r#"{"type":"init","model":"gpt-4-turbo"}"#);
    let OpenCodeEvent::Init(init) = event else {
        panic!("expected Init");
    };
    assert_eq!(init.model.as_deref(), Some("gpt-4-turbo"));
}

#[test]
fn opencode_step_start_camel_case_session_id() {
    let event = parse(r#"{"type":"step_start","sessionID":"ses_abc","part":{"id":"prt_1"}}"#);
    let OpenCodeEvent::StepStart(step) = event else {
        panic!("expected StepStart");
    };
    assert_eq!(step.resolved_session_id(), Some("ses_abc".into()));
}

#[test]
fn opencode_text_from_part() {
    let event = parse(r#"{"type":"text","part":{"id":"prt_2","type":"text","text":"hello"}}"#);
    let OpenCodeEvent::Text(text) = event else {
        panic!("expected Text");
    };
    assert_eq!(text.resolved_text(), Some("hello".into()));
}

#[test]
fn opencode_text_from_top_level() {
    let event = parse(r#"{"type":"text","text":"legacy"}"#);
    let OpenCodeEvent::Text(text) = event else {
        panic!("expected Text");
    };
    assert_eq!(text.resolved_text(), Some("legacy".into()));
}

#[test]
fn opencode_text_from_content_fallback() {
    let event = parse(r#"{"type":"text","content":"from content"}"#);
    let OpenCodeEvent::Text(text) = event else {
        panic!("expected Text");
    };
    assert_eq!(text.resolved_text(), Some("from content".into()));
}

#[test]
fn opencode_step_finish_with_tokens() {
    let event = parse(
        r#"{"type":"step_finish","part":{"reason":"stop","cost":0.0205797,"tokens":{"total":54665,"input":150,"output":23,"cache":{"read":100}}}}"#,
    );
    let OpenCodeEvent::StepFinish(sf) = event else {
        panic!("expected StepFinish");
    };
    let part = sf.part.expect("part");
    assert_eq!(part.reason.as_deref(), Some("stop"));
    assert_eq!(part.cost, Some(0.0205797));
    let tokens = part.tokens.expect("tokens");
    assert_eq!(tokens.input, Some(150));
    assert_eq!(tokens.output, Some(23));
    assert_eq!(tokens.total, Some(54665));
    assert_eq!(tokens.cache.expect("cache").read, Some(100));
}

#[test]
fn opencode_step_complete_legacy_usage() {
    let event = parse(
        r#"{"type":"step_complete","usage":{"input_tokens":100,"output_tokens":50},"cost_usd":0.001}"#,
    );
    let OpenCodeEvent::StepComplete(sc) = event else {
        panic!("expected StepComplete");
    };
    let usage = sc.usage.expect("usage");
    assert_eq!(usage.input_tokens, Some(100));
    assert_eq!(usage.output_tokens, Some(50));
    assert_eq!(sc.cost_usd, Some(0.001));
}

#[test]
fn opencode_error_flat_fields() {
    let event = parse(r#"{"type":"error","error_message":"API timeout"}"#);
    let OpenCodeEvent::Error(err) = event else {
        panic!("expected Error");
    };
    assert_eq!(err.resolved_message(), Some("API timeout".into()));
}

#[test]
fn opencode_task_started_deserializes() {
    let event = parse(r#"{"type":"task_started","task_id":"sa1","name":"researcher"}"#);
    let OpenCodeEvent::TaskStarted(task) = event else {
        panic!("expected TaskStarted");
    };
    assert_eq!(task.resolved_task_id().as_deref(), Some("sa1"));
    assert_eq!(task.resolved_name().as_deref(), Some("researcher"));
}

#[test]
fn opencode_task_completed_deserializes() {
    let event = parse(
        r#"{"type":"task_completed","task_id":"sa1","name":"researcher","status":"success"}"#,
    );
    let OpenCodeEvent::TaskCompleted(task) = event else {
        panic!("expected TaskCompleted");
    };
    assert_eq!(task.resolved_task_id().as_deref(), Some("sa1"));
    assert_eq!(task.resolved_name().as_deref(), Some("researcher"));
    assert_eq!(task.status.as_deref(), Some("success"));
}

#[test]
fn opencode_task_progress_deserializes() {
    let event = parse(r#"{"type":"task_progress","message":"working"}"#);
    let OpenCodeEvent::TaskProgress(progress) = event else {
        panic!("expected TaskProgress");
    };
    assert_eq!(progress.message.as_deref(), Some("working"));
}

#[test]
fn opencode_tool_name_from_top_level_name() {
    let event = parse(r#"{"type":"tool_use","name":"search"}"#);
    let OpenCodeEvent::ToolUse(tool) = event else {
        panic!("expected ToolUse");
    };
    let resolved = tool.resolve();
    assert_eq!(resolved.name.as_deref(), Some("search"));
}

#[test]
fn opencode_tool_name_from_nested_part() {
    let event = parse(r#"{"type":"tool_use","part":{"name":"search"}}"#);
    let OpenCodeEvent::ToolUse(tool) = event else {
        panic!("expected ToolUse");
    };
    let resolved = tool.resolve();
    assert_eq!(resolved.name.as_deref(), Some("search"));
}

#[test]
fn opencode_tool_name_camel_case_nested() {
    let event = parse(r#"{"type":"tool_start","part":{"tool_name":"write_file"}}"#);
    let OpenCodeEvent::ToolStart(tool) = event else {
        panic!("expected ToolStart");
    };
    let resolved = tool.resolve();
    assert_eq!(resolved.name.as_deref(), Some("write_file"));
}

#[test]
fn opencode_tool_all_fields_from_part() {
    let event = parse(
        r#"{"type":"tool_start","part":{"id":"tool-1","tool_name":"bash","input":{"command":"git status"}}}"#,
    );
    let OpenCodeEvent::ToolStart(tool) = event else {
        panic!("expected ToolStart");
    };
    let resolved = tool.resolve();
    assert_eq!(resolved.id.as_deref(), Some("tool-1"));
    assert_eq!(resolved.name.as_deref(), Some("bash"));
    let input = resolved.input.expect("input");
    assert_eq!(
        input.get("command").and_then(Value::as_str),
        Some("git status")
    );
}

#[test]
fn opencode_tool_result_from_part_content_and_status() {
    let event = parse(
        r#"{"type":"tool_end","part":{"tool_use_id":"tool-1","status":"success","content":"working tree clean"}}"#,
    );
    let OpenCodeEvent::ToolEnd(tool) = event else {
        panic!("expected ToolEnd");
    };
    let resolved = tool.resolve();
    assert_eq!(resolved.id.as_deref(), Some("tool-1"));
    assert_eq!(resolved.status.as_deref(), Some("success"));
    let response = resolved.output.expect("output");
    assert_eq!(response.as_str(), Some("working tree clean"));
}

#[test]
fn opencode_tool_state_fills_when_part_missing_fields() {
    // When tool info lives under `part.state.*` (current OpenCode wire format
    // per message-v2.ts) and is absent at top-level and `part.*`, resolve()
    // must pick it up.
    let event = parse(
        r#"{"type":"tool_use","part":{"id":"t1","tool":"bash",
             "state":{"status":"completed","input":{"command":"ls -la"},"output":"file.txt"}}}"#,
    );
    let OpenCodeEvent::ToolUse(tool) = event else {
        panic!("expected ToolUse");
    };
    let resolved = tool.resolve();
    assert_eq!(resolved.id.as_deref(), Some("t1"));
    assert_eq!(resolved.name.as_deref(), Some("bash"));
    assert_eq!(resolved.status.as_deref(), Some("completed"));
    let input = resolved.input.expect("input from part.state");
    assert_eq!(input.get("command").and_then(Value::as_str), Some("ls -la"));
    let output = resolved.output.expect("output from part.state");
    assert_eq!(output.as_str(), Some("file.txt"));
}

#[test]
fn opencode_tool_part_fields_beat_state_when_both_present() {
    // Priority: top-level > part.fields > part.state. When part.fields and
    // part.state both carry `status`, part.fields must win.
    let event = parse(
        r#"{"type":"tool_use","part":{"id":"t1","tool":"bash","status":"from_part",
             "state":{"status":"from_state"}}}"#,
    );
    let OpenCodeEvent::ToolUse(tool) = event else {
        panic!("expected ToolUse");
    };
    let resolved = tool.resolve();
    assert_eq!(resolved.status.as_deref(), Some("from_part"));
}

#[test]
fn opencode_tool_args_params_aliases() {
    let event = parse(r#"{"type":"tool_use","args":{"x":1}}"#);
    let OpenCodeEvent::ToolUse(tool) = event else {
        panic!("expected ToolUse");
    };
    let resolved = tool.resolve();
    let input = resolved.input.expect("input");
    assert_eq!(input.get("x").and_then(Value::as_u64), Some(1));
}

#[test]
fn opencode_unknown_event_type_fails_typed() {
    let err = serde_json::from_str::<OpenCodeEvent>(r#"{"type":"not_a_real_event"}"#);
    assert!(err.is_err());
}

#[test]
fn opencode_reasoning_top_level_text_deserializes() {
    let event = parse(r#"{"type":"reasoning","text":"Thinking it over"}"#);
    let OpenCodeEvent::Reasoning(reasoning) = event else {
        panic!("expected Reasoning");
    };
    assert_eq!(reasoning.resolved_text(), Some("Thinking it over".into()));
}

#[test]
fn opencode_reasoning_nested_part_text_resolves() {
    let event = parse(
        r#"{"type":"reasoning","part":{"id":"prt_1","type":"reasoning","text":"nested prose"}}"#,
    );
    let OpenCodeEvent::Reasoning(reasoning) = event else {
        panic!("expected Reasoning");
    };
    assert_eq!(reasoning.resolved_text(), Some("nested prose".into()));
}

#[test]
fn opencode_tool_task_subagent_id_from_metadata() {
    let event = parse(
        r#"{"type":"tool_use","part":{"id":"t1","tool":"task","state":{"metadata":{"sessionId":"sa_123"}}}}"#,
    );
    let OpenCodeEvent::ToolUse(tool) = event else {
        panic!("expected ToolUse");
    };
    let resolved = tool.resolve();
    assert_eq!(resolved.task_subagent_id(), Some("sa_123"));
}

#[test]
fn opencode_tool_task_subagent_id_missing() {
    let event = parse(r#"{"type":"tool_use","part":{"id":"t1","tool":"task"}}"#);
    let OpenCodeEvent::ToolUse(tool) = event else {
        panic!("expected ToolUse");
    };
    let resolved = tool.resolve();
    assert_eq!(resolved.task_subagent_id(), None);
}

#[test]
fn opencode_tool_task_started_at_epoch_ms() {
    let event = parse(
        r#"{"type":"tool_use","part":{"id":"t1","tool":"task","state":{"time":{"start":1715432100000}}}}"#,
    );
    let OpenCodeEvent::ToolUse(tool) = event else {
        panic!("expected ToolUse");
    };
    let resolved = tool.resolve();
    assert_eq!(resolved.task_started_at_epoch_ms(), Some(1715432100000));
}

#[test]
fn opencode_tool_task_started_at_epoch_ms_missing() {
    let event = parse(r#"{"type":"tool_use","part":{"id":"t1","tool":"task"}}"#);
    let OpenCodeEvent::ToolUse(tool) = event else {
        panic!("expected ToolUse");
    };
    let resolved = tool.resolve();
    assert_eq!(resolved.task_started_at_epoch_ms(), None);
}

#[test]
fn opencode_tool_task_accessors_from_part_fields() {
    // Top-level fields should be used when present
    let event = parse(
        r#"{"type":"tool_use","metadata":{"sessionId":"top_id"},"time":{"start":1000},"part":{"tool":"task","state":{"metadata":{"sessionId":"state_id"},"time":{"start":2000}}}}"#,
    );
    let OpenCodeEvent::ToolUse(tool) = event else {
        panic!("expected ToolUse");
    };
    let resolved = tool.resolve();
    // Top-level metadata/sessionId should win
    assert_eq!(resolved.task_subagent_id(), Some("top_id"));
    // Top-level time/start should win
    assert_eq!(resolved.task_started_at_epoch_ms(), Some(1000));
}

#[test]
fn opencode_tool_task_accessors_from_part_fields_fallback() {
    // When top-level is absent, part.state should be used
    let event = parse(
        r#"{"type":"tool_use","part":{"tool":"task","state":{"metadata":{"sessionId":"state_id"},"time":{"start":2000}}}}"#,
    );
    let OpenCodeEvent::ToolUse(tool) = event else {
        panic!("expected ToolUse");
    };
    let resolved = tool.resolve();
    assert_eq!(resolved.task_subagent_id(), Some("state_id"));
    assert_eq!(resolved.task_started_at_epoch_ms(), Some(2000));
}
