//! Codex app-server JSON-RPC: strict readers for the messages a managed owner
//! acts on, and the projection of its notifications onto the `exec --json`
//! events the Codex parser already renders.
//!
//! `codex app-server --listen stdio://` speaks JSON-RPC 2.0 without the
//! `jsonrpc` member, one message per line. Three shapes arrive on stdout:
//! responses to the owner's requests (`id` + `result`/`error`), requests the
//! server makes of its client (`id` + `method`), and notifications (`method`
//! only). `codex exec` is itself an in-process client of the same server, and
//! [`Projector`] mirrors its JSONL projection, so an app-server run reads like
//! an exec run.
//!
//! The owner decides from the [`read_message`] shape and the typed readers
//! below whether the server is initialized, which thread and turn it runs,
//! whether that turn is still active, and whether a request was accepted.
//! Those fields must be present, non-null, and of the documented type; a
//! malformed one is an error, never a default. The projection is display
//! only and stays lenient.
//!
//! Protocol reference: `claudine/docs/topics/codex-app-server.md`.

use serde_json::{Map, Value};

use super::{
    CodexAgentMessage, CodexErrorDetail, CodexErrorEnvelope, CodexEvent, CodexFileChange, CodexFileChangeEntry,
    CodexItem, CodexItemEnvelope, CodexPlanUpdate, CodexReasoning, CodexThreadMeta, CodexToolItemFields,
    CodexTurnCompleted, CodexTurnStarted, CodexUsage,
};

#[cfg(test)]
mod tests;

/// The request id a managed owner gives the `turn/start` that submits its
/// task. The parser reads a refusal of that request as the run's failure,
/// since no turn and no turn notification will follow it.
pub const TASK_REQUEST_ID: &str = "claudine-task";

/// One stdout line, classified by JSON-RPC shape.
#[derive(Debug, Clone, PartialEq)]
pub enum Message {
    /// The answer to a request this client sent.
    Response(Response),
    /// A request the server makes of its client, which must be answered.
    Request(ServerRequest),
    /// A notification; nothing answers it.
    Notification { method: String, params: Value },
    /// Not a JSON-RPC message (a log line, blank output).
    Other,
}

/// A response. `id` is the request's string id; the owner only ever sends
/// string ids, so a numeric or null id correlates with nothing.
#[derive(Debug, Clone, PartialEq)]
pub struct Response {
    pub id: Option<String>,
    /// `Ok(result)` or the server's `error` object.
    pub outcome: Result<Value, RpcFailure>,
}

/// A JSON-RPC `error` object.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RpcFailure {
    pub code: i64,
    pub message: String,
}

impl std::fmt::Display for RpcFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} (code {})", self.message, self.code)
    }
}

/// A server-to-client request. `id` is echoed verbatim in the answer.
#[derive(Debug, Clone, PartialEq)]
pub struct ServerRequest {
    pub id: Value,
    pub method: String,
    pub params: Value,
}

/// Why a message the owner depends on could not be read.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum MessageError {
    #[error("`{0}` is missing")]
    Missing(&'static str),
    #[error("`{0}` is null")]
    Null(&'static str),
    #[error("`{field}` is not {expected}")]
    WrongType { field: &'static str, expected: &'static str },
    #[error("a response carries both `result` and `error`")]
    Ambiguous,
    #[error("`{field}` is `{value}`, which this client does not know")]
    UnknownValue { field: &'static str, value: String },
}

/// Classifies one stdout line. A line that is not a JSON object is
/// [`Message::Other`]; a JSON-RPC shape with an unreadable member is an error.
pub fn read_message(line: &str) -> Result<Message, MessageError> {
    let Ok(Value::Object(message)) = serde_json::from_str::<Value>(line.trim()) else {
        return Ok(Message::Other);
    };
    if message.contains_key("method") {
        let method = string(&message, "method")?;
        let params = message.get("params").cloned().unwrap_or(Value::Null);
        return Ok(match message.get("id") {
            None => Message::Notification { method, params },
            Some(Value::Null) => return Err(MessageError::Null("id")),
            Some(id) => Message::Request(ServerRequest { id: id.clone(), method, params }),
        });
    }
    if !message.contains_key("id") {
        return Ok(Message::Other);
    }
    let id = match message.get("id") {
        Some(Value::String(id)) => Some(id.clone()),
        _ => None,
    };
    let outcome = match (message.get("result"), message.get("error")) {
        (Some(_), Some(_)) => return Err(MessageError::Ambiguous),
        (Some(result), None) => Ok(result.clone()),
        (None, Some(Value::Object(error))) => Err(RpcFailure {
            code: field(error, "code")?
                .as_i64()
                .ok_or(MessageError::WrongType { field: "code", expected: "an integer" })?,
            // The text only explains a failure the error object already
            // decided, so a missing one is described rather than fatal.
            message: error.get("message").and_then(Value::as_str).unwrap_or("no reason given").to_string(),
        }),
        (None, Some(Value::Null)) => return Err(MessageError::Null("error")),
        (None, Some(_)) => return Err(MessageError::WrongType { field: "error", expected: "an object" }),
        (None, None) => return Err(MessageError::Missing("result")),
    };
    Ok(Message::Response(Response { id, outcome }))
}

/// What `initialize` established.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InitializeInfo {
    pub user_agent: String,
    /// The exact Codex version, when the user agent names one.
    pub provider_version: Option<String>,
}

/// Reads an `initialize` result.
pub fn initialize_info(result: &Value) -> Result<InitializeInfo, MessageError> {
    let result = object(result, "result")?;
    let user_agent = string(result, "userAgent")?;
    let provider_version = provider_version(&user_agent);
    Ok(InitializeInfo { user_agent, provider_version })
}

/// The Codex version in a user agent such as
/// `claudine/0.157.1 (Mac OS 27.2.0; arm64) …`: the first token's text after
/// its `/`, when it is dotted digits. Anything else is `None`, never a guess.
pub fn provider_version(user_agent: &str) -> Option<String> {
    let (_, version) = user_agent.split_whitespace().next()?.split_once('/')?;
    let dotted = !version.is_empty()
        && version.split('.').all(|part| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit()));
    dotted.then(|| version.to_string())
}

/// Reads the thread id of a `thread/start` or `thread/resume` result.
pub fn started_thread(result: &Value) -> Result<String, MessageError> {
    let thread = object(field(object(result, "result")?, "thread")?, "thread")?;
    non_empty(thread, "id")
}

/// A turn's lifecycle status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TurnStatus {
    InProgress,
    Completed,
    Interrupted,
    Failed,
}

impl TurnStatus {
    fn read(value: &Value) -> Result<Self, MessageError> {
        match value.as_str() {
            Some("inProgress") => Ok(Self::InProgress),
            Some("completed") => Ok(Self::Completed),
            Some("interrupted") => Ok(Self::Interrupted),
            Some("failed") => Ok(Self::Failed),
            Some(other) => Err(MessageError::UnknownValue { field: "status", value: other.to_string() }),
            None if value.is_null() => Err(MessageError::Null("status")),
            None => Err(MessageError::WrongType { field: "status", expected: "a string" }),
        }
    }
}

/// A turn as `turn/start` answers it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TurnRef {
    pub id: String,
    pub status: TurnStatus,
}

/// Reads the turn of a `turn/start` result.
pub fn started_turn(result: &Value) -> Result<TurnRef, MessageError> {
    turn(field(object(result, "result")?, "turn")?)
}

/// Reads the turn a `turn/steer` result says accepted the input.
pub fn steered_turn(result: &Value) -> Result<String, MessageError> {
    non_empty(object(result, "result")?, "turnId")
}

/// A `turn/started` or `turn/completed` notification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TurnEvent {
    pub thread_id: String,
    pub turn: TurnRef,
}

/// Reads the params of `turn/started` or `turn/completed`.
pub fn turn_event(params: &Value) -> Result<TurnEvent, MessageError> {
    let params = object(params, "params")?;
    Ok(TurnEvent { thread_id: non_empty(params, "threadId")?, turn: turn(field(params, "turn")?)? })
}

/// A thread's runtime status, as `thread/read` reports it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThreadActivity {
    /// Loaded with no running turn.
    Idle,
    /// A turn is running.
    Active,
    /// Not loaded in this server.
    NotLoaded,
    /// The thread failed.
    SystemError,
}

/// Reads a `thread/read` result for the thread `expected`. A different thread
/// is an error: the owner never reads one thread's state as another's.
pub fn thread_activity(result: &Value, expected: &str) -> Result<ThreadActivity, MessageError> {
    let thread = object(field(object(result, "result")?, "thread")?, "thread")?;
    let id = non_empty(thread, "id")?;
    if id != expected {
        return Err(MessageError::UnknownValue { field: "thread.id", value: id });
    }
    let status = object(field(thread, "status")?, "status")?;
    match string(status, "type")?.as_str() {
        "idle" => Ok(ThreadActivity::Idle),
        "active" => Ok(ThreadActivity::Active),
        "notLoaded" => Ok(ThreadActivity::NotLoaded),
        "systemError" => Ok(ThreadActivity::SystemError),
        other => Err(MessageError::UnknownValue { field: "status.type", value: other.to_string() }),
    }
}

fn turn(value: &Value) -> Result<TurnRef, MessageError> {
    let turn = object(value, "turn")?;
    Ok(TurnRef { id: non_empty(turn, "id")?, status: TurnStatus::read(field(turn, "status")?)? })
}

fn field<'a>(record: &'a Map<String, Value>, name: &'static str) -> Result<&'a Value, MessageError> {
    match record.get(name) {
        None => Err(MessageError::Missing(name)),
        Some(Value::Null) => Err(MessageError::Null(name)),
        Some(value) => Ok(value),
    }
}

fn object<'a>(value: &'a Value, name: &'static str) -> Result<&'a Map<String, Value>, MessageError> {
    match value {
        Value::Object(map) => Ok(map),
        Value::Null => Err(MessageError::Null(name)),
        _ => Err(MessageError::WrongType { field: name, expected: "an object" }),
    }
}

fn string(record: &Map<String, Value>, name: &'static str) -> Result<String, MessageError> {
    field(record, name)?
        .as_str()
        .map(str::to_string)
        .ok_or(MessageError::WrongType { field: name, expected: "a string" })
}

fn non_empty(record: &Map<String, Value>, name: &'static str) -> Result<String, MessageError> {
    let value = string(record, name)?;
    if value.is_empty() {
        return Err(MessageError::WrongType { field: name, expected: "a non-empty string" });
    }
    Ok(value)
}

/// What one notification means for the rendered stream.
#[derive(Debug)]
pub enum Projected {
    /// The `exec --json` event exec would have printed.
    Event(Box<CodexEvent>),
    /// A non-fatal provider notice.
    Warning(String),
    /// A turn ended as interrupted. exec counts this as a failed run unless
    /// another turn follows.
    Interrupted,
}

/// Token totals, for per-turn usage.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct Totals {
    input: u64,
    cached_input: u64,
    output: u64,
    total: u64,
}

impl Totals {
    fn read(breakdown: &Value) -> Option<Self> {
        let count = |name: &str| breakdown.get(name).and_then(Value::as_u64);
        Some(Self {
            input: count("inputTokens")?,
            cached_input: count("cachedInputTokens").unwrap_or(0),
            output: count("outputTokens")?,
            total: count("totalTokens").unwrap_or(0),
        })
    }

    fn since(self, start: Self) -> Self {
        Self {
            input: self.input.saturating_sub(start.input),
            cached_input: self.cached_input.saturating_sub(start.cached_input),
            output: self.output.saturating_sub(start.output),
            total: self.total.saturating_sub(start.total),
        }
    }
}

/// Projects app-server notifications onto `exec --json` events.
///
/// Stateful only where exec is: the thread id is announced once (from
/// `thread/started`, or the first turn that names it), and a turn's usage is
/// the change in thread totals since the turn started.
#[derive(Debug, Default)]
pub struct Projector {
    thread_announced: bool,
    totals: Totals,
    turn_start_totals: Totals,
}

impl Projector {
    pub fn new() -> Self {
        Self::default()
    }

    /// The events one notification projects to; empty for notifications exec
    /// does not print (deltas, status changes, rate limits, user input).
    pub fn project(&mut self, method: &str, params: &Value) -> Vec<Projected> {
        match method {
            "thread/started" => {
                let id = params.pointer("/thread/id").and_then(Value::as_str);
                self.announce_thread(id).into_iter().collect()
            }
            "turn/started" => {
                let mut out: Vec<Projected> =
                    self.announce_thread(params.get("threadId").and_then(Value::as_str)).into_iter().collect();
                self.turn_start_totals = self.totals;
                out.push(event(CodexEvent::TurnStarted(CodexTurnStarted {})));
                out
            }
            "thread/tokenUsage/updated" => {
                if let Some(totals) = params.pointer("/tokenUsage/total").and_then(Totals::read) {
                    self.totals = totals;
                }
                Vec::new()
            }
            "turn/completed" => self.turn_completed(params),
            "item/started" => params
                .get("item")
                .and_then(started_item)
                .map(|item| vec![event(CodexEvent::ItemStarted(CodexItemEnvelope { item: Some(item) }))])
                .unwrap_or_default(),
            "item/completed" => params
                .get("item")
                .and_then(completed_item)
                .map(|item| vec![event(CodexEvent::ItemCompleted(CodexItemEnvelope { item: Some(item) }))])
                .unwrap_or_default(),
            "error" => {
                let message = params.pointer("/error/message").and_then(Value::as_str).unwrap_or("Codex reported an error");
                if params.get("willRetry").and_then(Value::as_bool) == Some(true) {
                    vec![Projected::Warning(format!("{message} (retrying)"))]
                } else {
                    vec![event(CodexEvent::Error(error_envelope(params.get("error"))))]
                }
            }
            "warning" | "configWarning" => params
                .get("message")
                .or_else(|| params.get("summary"))
                .and_then(Value::as_str)
                .map(|message| vec![Projected::Warning(message.to_string())])
                .unwrap_or_default(),
            _ => Vec::new(),
        }
    }

    fn announce_thread(&mut self, id: Option<&str>) -> Option<Projected> {
        let id = id.filter(|id| !id.is_empty())?;
        if self.thread_announced {
            return None;
        }
        self.thread_announced = true;
        Some(event(CodexEvent::ThreadStarted(CodexThreadMeta { thread_id: Some(id.to_string()), id: None })))
    }

    fn turn_completed(&mut self, params: &Value) -> Vec<Projected> {
        let turn = params.get("turn");
        match turn.and_then(|turn| turn.get("status")).and_then(Value::as_str) {
            Some("completed") => {
                let used = self.totals.since(self.turn_start_totals);
                let usage = (used != Totals::default()).then_some(CodexUsage {
                    input_tokens: Some(used.input),
                    output_tokens: Some(used.output),
                    cached_input_tokens: Some(used.cached_input),
                    cache_read_input_tokens: None,
                    total_tokens: Some(used.total),
                });
                let duration_ms = turn.and_then(|turn| turn.get("durationMs")).and_then(Value::as_u64);
                vec![event(CodexEvent::TurnCompleted(CodexTurnCompleted {
                    usage,
                    duration_ms,
                    ..Default::default()
                }))]
            }
            Some("failed") => {
                vec![event(CodexEvent::TurnFailed(error_envelope(turn.and_then(|turn| turn.get("error")))))]
            }
            Some("interrupted") => vec![Projected::Interrupted],
            _ => Vec::new(),
        }
    }
}

fn event(event: CodexEvent) -> Projected {
    Projected::Event(Box::new(event))
}

fn error_envelope(error: Option<&Value>) -> CodexErrorEnvelope {
    let message = error.and_then(|error| error.get("message")).and_then(Value::as_str).map(str::to_string);
    // `codexErrorInfo` is a string for unit variants (`contextWindowExceeded`)
    // and an object keyed by the variant otherwise.
    let kind = error.and_then(|error| error.get("codexErrorInfo")).and_then(|info| match info {
        Value::String(kind) => Some(kind.clone()),
        Value::Object(map) => map.keys().next().cloned(),
        _ => None,
    });
    CodexErrorEnvelope {
        message: message.clone(),
        error: Some(CodexErrorDetail { kind, message }),
        ..Default::default()
    }
}

fn text(item: &Value, name: &str) -> Option<String> {
    item.get(name).and_then(Value::as_str).map(str::to_string)
}

/// exec prints a start event only for work that runs for a while.
fn started_item(item: &Value) -> Option<CodexItem> {
    match item.get("type").and_then(Value::as_str)? {
        "commandExecution" | "mcpToolCall" | "webSearch" | "dynamicToolCall" => completed_item(item),
        _ => None,
    }
}

fn completed_item(item: &Value) -> Option<CodexItem> {
    let id = text(item, "id");
    let tool = |fields: CodexToolItemFields| CodexToolItemFields { id: id.clone(), ..fields };
    Some(match item.get("type").and_then(Value::as_str)? {
        "agentMessage" => CodexItem::AgentMessage(CodexAgentMessage { id, text: text(item, "text"), content: None }),
        "reasoning" => {
            let joined = |name: &str| {
                let parts: Vec<&str> = item.get(name)?.as_array()?.iter().filter_map(Value::as_str).collect();
                (!parts.is_empty()).then(|| parts.join("\n\n"))
            };
            CodexItem::Reasoning(CodexReasoning { text: joined("summary").or_else(|| joined("content")), summary: None })
        }
        "commandExecution" => CodexItem::CommandExec(tool(CodexToolItemFields {
            command: text(item, "command"),
            aggregated_output: text(item, "aggregatedOutput"),
            exit_code: item.get("exitCode").and_then(Value::as_i64).and_then(|code| i32::try_from(code).ok()),
            status: status(item),
            ..Default::default()
        })),
        "fileChange" => CodexItem::FileChange(CodexFileChange {
            id,
            changes: item.get("changes").and_then(Value::as_array).map(|changes| {
                changes
                    .iter()
                    .map(|change| CodexFileChangeEntry {
                        path: text(change, "path"),
                        kind: change.pointer("/kind/type").and_then(Value::as_str).map(str::to_string),
                        ..Default::default()
                    })
                    .collect()
            }),
            status: status(item),
            ..Default::default()
        }),
        "mcpToolCall" => CodexItem::McpToolCall(tool(CodexToolItemFields {
            tool_name: match (text(item, "server"), text(item, "tool")) {
                (Some(server), Some(tool)) => Some(format!("{server}.{tool}")),
                (_, tool) => tool,
            },
            arguments: item.get("arguments").cloned(),
            result: item.get("result").filter(|value| !value.is_null()).cloned(),
            content: item.get("error").filter(|value| !value.is_null()).cloned(),
            status: status(item),
            ..Default::default()
        })),
        "dynamicToolCall" => CodexItem::ToolCall(tool(CodexToolItemFields {
            tool_name: text(item, "tool"),
            arguments: item.get("arguments").cloned(),
            content: item.get("contentItems").filter(|value| !value.is_null()).cloned(),
            status: status(item),
            ..Default::default()
        })),
        "webSearch" => CodexItem::WebSearch(tool(CodexToolItemFields {
            tool_name: Some("web_search".to_string()),
            arguments: text(item, "query").map(|query| serde_json::json!({ "query": query })),
            ..Default::default()
        })),
        "imageView" => CodexItem::ViewImage(tool(CodexToolItemFields {
            tool_name: Some("view_image".to_string()),
            arguments: text(item, "path").map(|path| serde_json::json!({ "path": path })),
            ..Default::default()
        })),
        "plan" => CodexItem::PlanUpdate(CodexPlanUpdate { id, message: text(item, "text"), ..Default::default() }),
        _ => return None,
    })
}

/// exec's lower-case status words.
fn status(item: &Value) -> Option<String> {
    let status = item.get("status")?.as_str()?;
    Some(
        match status {
            "inProgress" => "in_progress",
            other => other,
        }
        .to_string(),
    )
}
