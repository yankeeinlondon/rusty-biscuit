//! Deserialization-fidelity fixture replay for the typed stream-protocol
//! models. Each test loads a captured wire-mode session
//! (`claudine/lib/src/stream/protocol/fixtures/<provider>/*.jsonl`) and replays
//! it through the provider's typed envelope/event model, asserting the resulting
//! variants and payload fields.
//!
//! This is the typed-model layer counterpart to `kimi_wire.rs`, which replays
//! the same Kimi corpus one level up through the semantic parser. Relocating
//! these cases out of `stream::protocol::kimi`'s `#[cfg(test)]` block shrinks
//! that model's lib test compile unit (see the module-structure refactor, N6).
//!
//! ## Notes
//!
//! Codex has no fixture corpus: every `stream::protocol::codex` test is an
//! inline JSON-literal deserialization check, which stays co-located as a
//! genuinely unit-scoped helper test.

use std::path::PathBuf;

use serde_json::Value;

use claudine::stream::protocol::kimi::*;

fn fixture_path(name: &str) -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("src/stream/protocol/fixtures/kimi");
    p.push(name);
    p
}

fn load_fixture_lines(name: &str) -> Vec<Value> {
    let raw = std::fs::read_to_string(fixture_path(name))
        .unwrap_or_else(|e| panic!("read fixture {name}: {e}"));
    let mut out = Vec::new();
    for (idx, line) in raw.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let value: Value = serde_json::from_str(trimmed)
            .unwrap_or_else(|e| panic!("parse {name} line {}: {e}", idx + 1));
        out.push(value);
    }
    out
}

fn classify_lines(name: &str) -> Vec<KimiEnvelope> {
    load_fixture_lines(name)
        .into_iter()
        .filter_map(KimiEnvelope::classify)
        .collect()
}

#[test]
fn wire_greet_fixture_classifies_every_line() {
    let envelopes = classify_lines("wire-greet.jsonl");
    assert!(!envelopes.is_empty());
    let mut notif = 0;
    let mut requests = 0;
    let mut success = 0;
    let mut errors = 0;
    for env in &envelopes {
        match env {
            KimiEnvelope::Notification(_) => notif += 1,
            KimiEnvelope::Request { .. } => requests += 1,
            KimiEnvelope::SuccessResponse { .. } => success += 1,
            KimiEnvelope::ErrorResponse { .. } => errors += 1,
        }
    }
    assert!(notif > 0, "expected at least one notification");
    assert!(
        success >= 2,
        "expected initialize and prompt success responses, got {success}"
    );
    assert_eq!(requests, 0, "greet fixture should have no server requests");
    assert_eq!(errors, 0, "greet fixture should have no error responses");
}

#[test]
fn wire_greet_fixture_produces_typed_events() {
    let envelopes = classify_lines("wire-greet.jsonl");
    let mut typed = 0;
    let mut think_parts = 0;
    let mut text_parts = 0;
    let mut turn_begin = 0;
    let mut turn_end = 0;
    let mut status_updates = 0;
    for env in envelopes {
        if let KimiEnvelope::Notification(params) = env
            && let Some(typed_event) = params.into_event()
        {
            typed += 1;
            match typed_event {
                KimiWireEvent::TurnBegin(_) => turn_begin += 1,
                KimiWireEvent::TurnEnd(_) => turn_end += 1,
                KimiWireEvent::StatusUpdate(_) => status_updates += 1,
                KimiWireEvent::ContentPart(part) => {
                    if part.is_thinking() {
                        think_parts += 1;
                    } else if part.is_text() {
                        text_parts += 1;
                    }
                }
                _ => {}
            }
        }
    }
    assert!(typed > 0);
    assert_eq!(turn_begin, 1, "expected exactly one TurnBegin");
    assert_eq!(turn_end, 1, "expected exactly one TurnEnd");
    assert!(status_updates >= 1, "expected at least one StatusUpdate");
    assert!(think_parts > 0, "expected reasoning ContentPart deltas");
    assert!(text_parts > 0, "expected assistant text ContentPart deltas");
}

#[test]
fn wire_greet_fixture_prompt_status_finished() {
    let envelopes = classify_lines("wire-greet.jsonl");
    let mut last_prompt_status: Option<String> = None;
    for env in envelopes {
        if let KimiEnvelope::SuccessResponse { id, result } = env
            && id.as_str() == Some("prompt-2")
            && let Ok(parsed) = serde_json::from_value::<KimiPromptResult>(result)
        {
            last_prompt_status = parsed.status;
        }
    }
    assert_eq!(
        last_prompt_status.as_deref(),
        Some(KimiPromptResult::STATUS_FINISHED)
    );
}

#[test]
fn wire_tool_shell_fixture_covers_tool_lifecycle() {
    let envelopes = classify_lines("wire-tool-shell.jsonl");
    let mut tool_calls = 0;
    let mut tool_call_parts = 0;
    let mut tool_results = 0;
    let mut approval_requests = 0;
    let mut approval_responses = 0;
    for env in envelopes {
        match env {
            KimiEnvelope::Notification(params) => {
                if let Some(typed) = params.into_event() {
                    match typed {
                        KimiWireEvent::ToolCall(_) => tool_calls += 1,
                        KimiWireEvent::ToolCallPart(_) => tool_call_parts += 1,
                        KimiWireEvent::ToolResult(_) => tool_results += 1,
                        KimiWireEvent::ApprovalResponse(_) => approval_responses += 1,
                        _ => {}
                    }
                }
            }
            KimiEnvelope::Request { params, .. } => {
                if let Some(KimiWireRequest::Approval(_)) = params.into_request() {
                    approval_requests += 1;
                }
            }
            _ => {}
        }
    }
    assert!(tool_calls >= 1, "expected at least one ToolCall");
    assert!(tool_call_parts > 0, "expected ToolCallPart deltas");
    assert!(tool_results >= 1, "expected at least one ToolResult");
    assert!(approval_requests >= 1, "expected ApprovalRequest");
    assert!(approval_responses >= 1, "expected ApprovalResponse echo");
}

#[test]
fn wire_tool_shell_fixture_arguments_decode() {
    let envelopes = classify_lines("wire-tool-shell.jsonl");
    let mut arg_buffer = String::new();
    let mut tool_call_seen = false;
    for env in envelopes {
        if let KimiEnvelope::Notification(params) = env
            && let Some(typed) = params.into_event()
        {
            match typed {
                KimiWireEvent::ToolCall(mut call) => {
                    tool_call_seen = true;
                    if let Some(initial) = call.take_arguments_string() {
                        arg_buffer.push_str(&initial);
                    }
                }
                KimiWireEvent::ToolCallPart(part) => {
                    if let Some(delta) = part.arguments_part {
                        arg_buffer.push_str(&delta);
                    }
                }
                _ => {}
            }
        }
    }
    assert!(tool_call_seen);
    let parsed = KimiToolCall::parse_arguments_string(&arg_buffer)
        .unwrap()
        .expect("parsed args");
    assert_eq!(
        parsed.get("command").and_then(Value::as_str),
        Some("echo hello-from-kimi")
    );
}

#[test]
fn wire_subagent_fixture_nested_events_decode() {
    let envelopes = classify_lines("wire-subagent.jsonl");
    let mut subagent_count = 0;
    let mut nested_typed = 0;
    for env in envelopes {
        if let KimiEnvelope::Notification(params) = env
            && let Some(KimiWireEvent::SubagentEvent(sub)) = params.into_event()
        {
            subagent_count += 1;
            if sub.nested_event().is_some() {
                nested_typed += 1;
            }
        }
    }
    assert!(subagent_count > 0, "expected at least one SubagentEvent");
    assert!(
        nested_typed > 0,
        "expected at least one nested event to decode"
    );
}

#[test]
fn wire_protocol_110_fixture_decodes_new_event_surface() {
    let envelopes = classify_lines("wire-protocol-110.jsonl");
    let mut init_version: Option<String> = None;
    let mut step_retry = 0;
    let mut mcp_snapshots = 0;
    let mut rich_notifications = 0;
    for env in envelopes {
        match env {
            KimiEnvelope::SuccessResponse { id, result } if id.as_str() == Some("init-1") => {
                let parsed: KimiInitializeResult = serde_json::from_value(result).unwrap();
                init_version = parsed.protocol_version;
            }
            KimiEnvelope::Notification(params) => match params.into_event() {
                Some(KimiWireEvent::StepRetry(retry)) => {
                    step_retry += 1;
                    assert_eq!(retry.error_type.as_deref(), Some("APIEmptyResponseError"));
                    assert_eq!(retry.status_code, Some(500));
                }
                Some(KimiWireEvent::StatusUpdate(status)) => {
                    let snapshot = status.mcp_status.expect("typed mcp_status");
                    assert_eq!(snapshot.connected, Some(1));
                    mcp_snapshots += 1;
                }
                Some(KimiWireEvent::Notification(notification)) => {
                    assert_eq!(notification.category.as_deref(), Some("task"));
                    assert_eq!(notification.body.as_deref(), Some("Task `lint` completed"));
                    rich_notifications += 1;
                }
                Some(_) => {}
                None => panic!("1.10 fixture line failed typed decode"),
            },
            _ => {}
        }
    }
    assert_eq!(init_version.as_deref(), Some("1.10"));
    assert_eq!(step_retry, 1);
    assert_eq!(mcp_snapshots, 1);
    assert_eq!(rich_notifications, 1);
}

#[test]
fn wire_cancelled_fixture_prompt_status_cancelled() {
    let envelopes = classify_lines("wire-cancelled.jsonl");
    let mut prompt_status: Option<String> = None;
    for env in envelopes {
        if let KimiEnvelope::SuccessResponse { id, result } = env
            && id.as_str() == Some("prompt-2")
            && let Ok(parsed) = serde_json::from_value::<KimiPromptResult>(result)
        {
            prompt_status = parsed.status;
        }
    }
    assert_eq!(
        prompt_status.as_deref(),
        Some(KimiPromptResult::STATUS_CANCELLED)
    );
}

#[test]
fn wire_auth_expired_fixture_classifies_error_response() {
    let envelopes = classify_lines("wire-auth-expired.jsonl");
    let mut found_auth_expired = false;
    for env in envelopes {
        if let KimiEnvelope::ErrorResponse { id, error } = env
            && id.as_str() == Some("prompt-2")
            && error.code == KimiJsonRpcError::AUTH_EXPIRED
        {
            found_auth_expired = true;
        }
    }
    assert!(
        found_auth_expired,
        "expected an AUTH_EXPIRED error response on prompt-2"
    );
}
