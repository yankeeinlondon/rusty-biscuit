//! Strict readers for the Pi RPC records a managed owner acts on.
//!
//! The owner decides from these fields whether the child is ready, which
//! conversation it holds, whether a turn is running, whether anything is
//! queued, and whether a command was accepted. Each one must therefore be
//! present, non-null, and of the documented type: an absent `isStreaming` is
//! not "idle", an absent `success` is not "refused", and a malformed record
//! is an error rather than a default. JSON permits a repeated key, and the
//! reader keeps the last value as `serde_json` does; Pi's `JSON.stringify`
//! never repeats one.
//!
//! Protocol reference: `claudine/docs/research/non-interactive-sessions/pi.md`
//! and `claudine/docs/topics/pi-rpc.md`.

use serde_json::{Map, Value};

#[cfg(test)]
mod tests;

/// A stdout record the managed owner reacts to. Every other line is
/// [`Record::Other`]: the semantic parser renders it, the owner ignores it.
#[derive(Debug, Clone, PartialEq)]
pub enum Record {
    Response(Response),
    UiRequest(UiRequest),
    AgentStart,
    AgentSettled,
    Other,
}

/// A correlated command `response`.
#[derive(Debug, Clone, PartialEq)]
pub struct Response {
    pub id: String,
    pub command: String,
    /// `Ok(data)` when Pi accepted the command (`data` is `Null` when the
    /// response carried none); `Err(error)` with Pi's error text otherwise.
    pub outcome: Result<Value, String>,
}

/// An `extension_ui_request` the owner must classify and possibly answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiRequest {
    pub id: String,
    pub method: String,
}

/// How an `extension_ui_request` method behaves in RPC mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiMethodKind {
    /// The extension waits for an `extension_ui_response`. Pi documents a
    /// `cancelled` answer for every dialog, which the extension observes as
    /// "no selection" (a `confirm` resolves `false`).
    Dialog,
    /// Fire-and-forget display; Pi expects no answer.
    Notice,
    /// Not a documented method: whether the extension waits, and what a safe
    /// answer would be, is unknown.
    Unsupported,
}

/// Classifies an `extension_ui_request` method.
pub fn ui_method_kind(method: &str) -> UiMethodKind {
    match method {
        "select" | "confirm" | "input" | "editor" => UiMethodKind::Dialog,
        "notify" | "setStatus" | "setWidget" | "setTitle" | "set_editor_text" => UiMethodKind::Notice,
        _ => UiMethodKind::Unsupported,
    }
}

/// The `get_state` fields a managed owner relies on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionState {
    pub session_id: String,
    pub is_streaming: bool,
    pub is_compacting: bool,
    /// Queued steering and follow-up messages not yet delivered.
    pub pending_messages: u64,
}

impl SessionState {
    /// No turn, compaction, or queued message remains.
    pub fn is_quiescent(&self) -> bool {
        !self.is_streaming && !self.is_compacting && self.pending_messages == 0
    }
}

/// Why a record the owner depends on could not be read.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RecordError {
    #[error("`{0}` is missing")]
    Missing(&'static str),
    #[error("`{0}` is null")]
    Null(&'static str),
    #[error("`{field}` is not {expected}")]
    WrongType { field: &'static str, expected: &'static str },
}

/// Reads one stdout line. Lines that are not JSON objects, or whose `type`
/// the owner does not act on, are [`Record::Other`]; a `response` or
/// `extension_ui_request` with an unreadable load-bearing field is an error.
pub fn read_record(line: &str) -> Result<Record, RecordError> {
    let Ok(Value::Object(record)) = serde_json::from_str::<Value>(line.trim()) else {
        return Ok(Record::Other);
    };
    match record.get("type").and_then(Value::as_str) {
        Some("response") => read_response(&record).map(Record::Response),
        Some("extension_ui_request") => Ok(Record::UiRequest(UiRequest {
            id: string(&record, "id")?,
            method: string(&record, "method")?,
        })),
        Some("agent_start") => Ok(Record::AgentStart),
        Some("agent_settled") => Ok(Record::AgentSettled),
        _ => Ok(Record::Other),
    }
}

fn read_response(record: &Map<String, Value>) -> Result<Response, RecordError> {
    let id = string(record, "id")?;
    let command = string(record, "command")?;
    let outcome = if boolean(record, "success")? {
        Ok(record.get("data").cloned().unwrap_or(Value::Null))
    } else {
        // The error text only explains a refusal that `success` already
        // decided, so a missing or odd one is described rather than fatal.
        Err(record.get("error").and_then(Value::as_str).unwrap_or("Pi gave no reason").to_string())
    };
    Ok(Response { id, command, outcome })
}

/// Reads the `data` of a successful `get_state` response.
pub fn session_state(data: &Value) -> Result<SessionState, RecordError> {
    let Value::Object(data) = data else {
        return Err(RecordError::WrongType { field: "data", expected: "an object" });
    };
    let session_id = string(data, "sessionId")?;
    if session_id.is_empty() {
        return Err(RecordError::WrongType { field: "sessionId", expected: "a non-empty string" });
    }
    Ok(SessionState {
        session_id,
        is_streaming: boolean(data, "isStreaming")?,
        is_compacting: boolean(data, "isCompacting")?,
        pending_messages: count(data, "pendingMessageCount")?,
    })
}

fn field<'a>(record: &'a Map<String, Value>, name: &'static str) -> Result<&'a Value, RecordError> {
    match record.get(name) {
        None => Err(RecordError::Missing(name)),
        Some(Value::Null) => Err(RecordError::Null(name)),
        Some(value) => Ok(value),
    }
}

fn string(record: &Map<String, Value>, name: &'static str) -> Result<String, RecordError> {
    field(record, name)?
        .as_str()
        .map(str::to_string)
        .ok_or(RecordError::WrongType { field: name, expected: "a string" })
}

fn boolean(record: &Map<String, Value>, name: &'static str) -> Result<bool, RecordError> {
    field(record, name)?.as_bool().ok_or(RecordError::WrongType { field: name, expected: "a boolean" })
}

fn count(record: &Map<String, Value>, name: &'static str) -> Result<u64, RecordError> {
    field(record, name)?
        .as_u64()
        .ok_or(RecordError::WrongType { field: name, expected: "a non-negative integer" })
}
