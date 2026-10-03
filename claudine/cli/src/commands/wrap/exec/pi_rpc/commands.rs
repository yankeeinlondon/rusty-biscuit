//! Pi RPC commands Claudine writes, as `serde_json::Value` lines.
//!
//! Each command carries an `id` Pi echoes on its `response`. Pi documents no
//! duplicate suppression for ids, so an id correlates exactly one answer and
//! never makes a resend safe.

use serde_json::{Value, json};

/// Correlation id of the readiness check.
pub(crate) const READY_ID: &str = "claudine-ready";
/// Correlation id of the task prompt.
pub(crate) const TASK_ID: &str = "claudine-task";

pub(crate) fn get_state(id: &str) -> Value {
    json!({"id": id, "type": "get_state"})
}

pub(crate) fn prompt(id: &str, message: &str) -> Value {
    json!({"id": id, "type": "prompt", "message": message})
}

pub(crate) fn steer(id: &str, message: &str) -> Value {
    json!({"id": id, "type": "steer", "message": message})
}

pub(crate) fn abort(id: &str) -> Value {
    json!({"id": id, "type": "abort"})
}

/// Pi's documented dialog answer for "no selection": `confirm` resolves
/// `false`, and `select`, `input`, and `editor` resolve without a value.
pub(crate) fn cancel_dialog(request_id: &str) -> Value {
    json!({"type": "extension_ui_response", "id": request_id, "cancelled": true})
}
