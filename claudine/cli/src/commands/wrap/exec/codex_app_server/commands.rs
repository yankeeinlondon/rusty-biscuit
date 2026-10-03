//! Codex app-server messages Claudine writes, as `serde_json::Value` lines.
//!
//! Requests carry a string `id` the server echoes on its response. A
//! `clientUserMessageId` is echoed on the resulting user-message item but is
//! not deduplicated (Codex 0.157.1 delivered a repeated one twice), so an id
//! correlates an answer and never makes a resend safe.

use serde_json::{Value, json};

pub(crate) use claudine::stream::protocol::codex::app_server::TASK_REQUEST_ID;

/// Correlation id of `initialize`.
pub(crate) const INITIALIZE_ID: &str = "claudine-initialize";
/// Correlation id of `thread/start` or `thread/resume`.
pub(crate) const THREAD_ID: &str = "claudine-thread";

/// JSON-RPC error code for a server request this client does not serve, as
/// `codex exec` answers the same requests.
const NOT_SUPPORTED: i64 = -32000;

pub(crate) fn initialize() -> Value {
    json!({
        "id": INITIALIZE_ID,
        "method": "initialize",
        "params": {
            "clientInfo": {"name": "claudine", "title": "Claudine", "version": env!("CARGO_PKG_VERSION")},
            // `codex exec` enables the experimental API too; `turn/steer`
            // is part of it.
            "capabilities": {"experimentalApi": true, "requestAttestation": false},
        },
    })
}

pub(crate) fn initialized() -> Value {
    json!({"method": "initialized"})
}

pub(crate) fn thread_start(params: &Value) -> Value {
    json!({"id": THREAD_ID, "method": "thread/start", "params": params})
}

pub(crate) fn thread_resume(thread_id: &str, params: &Value) -> Value {
    let mut params = params.clone();
    params["threadId"] = Value::from(thread_id);
    json!({"id": THREAD_ID, "method": "thread/resume", "params": params})
}

fn text_input(text: &str) -> Value {
    json!([{"type": "text", "text": text, "text_elements": []}])
}

pub(crate) fn turn_start(id: &str, thread_id: &str, text: &str, client_message_id: &str) -> Value {
    json!({
        "id": id,
        "method": "turn/start",
        "params": {"threadId": thread_id, "input": text_input(text), "clientUserMessageId": client_message_id},
    })
}

/// `expectedTurnId` is Codex's atomic target guard: the server refuses the
/// input unless that exact turn is still the thread's active turn.
pub(crate) fn turn_steer(id: &str, thread_id: &str, expected_turn_id: &str, text: &str, client_message_id: &str) -> Value {
    json!({
        "id": id,
        "method": "turn/steer",
        "params": {
            "threadId": thread_id,
            "expectedTurnId": expected_turn_id,
            "input": text_input(text),
            "clientUserMessageId": client_message_id,
        },
    })
}

pub(crate) fn turn_interrupt(id: &str, thread_id: &str, turn_id: &str) -> Value {
    json!({"id": id, "method": "turn/interrupt", "params": {"threadId": thread_id, "turnId": turn_id}})
}

pub(crate) fn thread_read(id: &str, thread_id: &str) -> Value {
    json!({"id": id, "method": "thread/read", "params": {"threadId": thread_id, "includeTurns": false}})
}

/// `codex exec`'s answer to an MCP elicitation: cancel it.
pub(crate) fn cancel_elicitation(request_id: &Value) -> Value {
    json!({"id": request_id, "result": {"action": "cancel", "content": null, "_meta": null}})
}

/// `codex exec`'s answer to every other server request: a JSON-RPC error.
/// It is a refusal, never an approval.
pub(crate) fn not_supported(request_id: &Value, method: &str) -> Value {
    json!({
        "id": request_id,
        "error": {"code": NOT_SUPPORTED, "message": format!("`{method}` is not supported in a managed Claudine run")},
    })
}
