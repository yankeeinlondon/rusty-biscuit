//! A deterministic model for real Codex runs: a loopback server that speaks
//! the OpenAI Responses API streaming format Codex requests from a custom
//! model provider.
//!
//! Each `POST …/responses` is answered by a responder that sees every request
//! body so far, so a script can react to what reached the model (a steering
//! nonce, a tool's output). [`ScriptedModel::codex_args`] points Codex at the
//! server; run Codex with a disposable `CODEX_HOME` so the user's
//! configuration, credentials, and history are never read or written.
//!
//! Used by the `real_` tier only: `tests/real/real_codex_app_server.rs` and,
//! through `#[path]`, `src/commands/wrap/exec/codex_app_server/tests.rs`.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use serde_json::{Value, json};

/// One model response.
#[derive(Debug, Clone)]
pub enum Reply {
    /// A final assistant message.
    Message(String),
    /// Assistant text (possibly empty) followed by tool calls, which the model
    /// then waits on. Each call is `(tool name, arguments)`.
    Calls { text: String, calls: Vec<(String, Value)> },
    /// Another reply, completed only after `delay`, so a request arrives while
    /// the model is still generating.
    Slow { delay: Duration, reply: Box<Reply> },
    /// An HTTP 500 with no body: the model request fails.
    Fail,
}

impl Reply {
    pub fn message(text: impl Into<String>) -> Self {
        Self::Message(text.into())
    }

    /// One `exec_command` call running `cmd` in the shell.
    pub fn shell(cmd: impl Into<String>) -> Self {
        Self::Calls { text: String::new(), calls: vec![("exec_command".into(), json!({"cmd": cmd.into()}))] }
    }
}

/// Decides each reply from every request body received so far (the current
/// one last).
pub type Responder = Box<dyn Fn(&[String]) -> Reply + Send + Sync>;

/// A running scripted model. Dropping it stops accepting connections.
pub struct ScriptedModel {
    port: u16,
    requests: Arc<Mutex<Vec<String>>>,
    originators: Arc<Mutex<Vec<String>>>,
}

impl ScriptedModel {
    /// Replies from `script` in order; once it runs out, every reply is
    /// `fallback`.
    pub fn scripted(script: Vec<Reply>, fallback: Reply) -> Self {
        Self::start(Box::new(move |requests: &[String]| {
            script.get(requests.len() - 1).cloned().unwrap_or_else(|| fallback.clone())
        }))
    }

    pub fn start(responder: Responder) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind the scripted model");
        let port = listener.local_addr().unwrap().port();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let originators = Arc::new(Mutex::new(Vec::new()));
        let responder = Arc::new(responder);
        let (seen, clients) = (Arc::clone(&requests), Arc::clone(&originators));
        thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(stream) = stream else { continue };
                let (seen, clients, responder) = (Arc::clone(&seen), Arc::clone(&clients), Arc::clone(&responder));
                thread::spawn(move || serve(stream, &seen, &clients, &responder));
            }
        });
        Self { port, requests, originators }
    }

    /// The `originator` header of every request: `codex_exec` for a
    /// `codex exec` run, the client name (`claudine`) for a managed
    /// app-server run.
    pub fn originators(&self) -> Vec<String> {
        self.originators.lock().unwrap().clone()
    }

    /// Every request body received, oldest first.
    pub fn requests(&self) -> Vec<String> {
        self.requests.lock().unwrap().clone()
    }

    /// Whether any request carried `needle` (for example a steering nonce
    /// that reached the model).
    pub fn saw(&self, needle: &str) -> bool {
        self.requests().iter().any(|body| body.contains(needle))
    }

    /// `-c` overrides that make Codex use this model and never retry.
    pub fn codex_args(&self) -> Vec<String> {
        [
            "model_provider=\"claudine-scripted\"".to_string(),
            "model=\"scripted-model\"".to_string(),
            "model_providers.claudine-scripted.name=\"claudine-scripted\"".to_string(),
            format!("model_providers.claudine-scripted.base_url=\"http://127.0.0.1:{}/v1\"", self.port),
            "model_providers.claudine-scripted.wire_api=\"responses\"".to_string(),
            "model_providers.claudine-scripted.request_max_retries=0".to_string(),
            "model_providers.claudine-scripted.stream_max_retries=0".to_string(),
        ]
        .into_iter()
        .flat_map(|value| ["-c".to_string(), value])
        .collect()
    }
}

fn serve(stream: TcpStream, seen: &Mutex<Vec<String>>, originators: &Mutex<Vec<String>>, responder: &Responder) {
    let mut writer = match stream.try_clone() {
        Ok(writer) => writer,
        Err(_) => return,
    };
    let mut reader = BufReader::new(stream);
    let mut length = 0usize;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).unwrap_or(0) == 0 {
            return;
        }
        let lower = line.to_ascii_lowercase();
        if let Some(value) = lower.strip_prefix("content-length:") {
            length = value.trim().parse().unwrap_or(0);
        }
        if let Some(value) = lower.strip_prefix("originator:") {
            originators.lock().unwrap().push(value.trim().to_string());
        }
        if line == "\r\n" {
            break;
        }
    }
    let mut body = vec![0; length];
    if reader.read_exact(&mut body).is_err() {
        return;
    }
    let reply = {
        let mut seen = seen.lock().unwrap();
        seen.push(String::from_utf8_lossy(&body).into_owned());
        responder(&seen)
    };
    let id = format!("resp-{}", seen.lock().unwrap().len());
    if matches!(reply, Reply::Fail) {
        let _ = writer.write_all(b"HTTP/1.1 500 Internal Server Error\r\ncontent-length: 0\r\nconnection: close\r\n\r\n");
        return;
    }
    let _ = writer.write_all(b"HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\ncache-control: no-cache\r\nconnection: close\r\n\r\n");
    let (delay, reply) = match reply {
        Reply::Slow { delay, reply } => (delay, *reply),
        reply => (Duration::ZERO, reply),
    };
    let mut events = vec![event("response.created", json!({"response": {"id": id}}))];
    let (text, calls) = match reply {
        Reply::Message(text) => (text, Vec::new()),
        Reply::Calls { text, calls } => (text, calls),
        Reply::Slow { .. } | Reply::Fail => (String::new(), Vec::new()),
    };
    if !text.is_empty() {
        events.push(event(
            "response.output_item.done",
            json!({"item": {"type": "message", "role": "assistant", "id": format!("msg-{id}"), "content": [{"type": "output_text", "text": text}]}}),
        ));
    }
    for (index, (name, arguments)) in calls.into_iter().enumerate() {
        events.push(event(
            "response.output_item.done",
            json!({"item": {"type": "function_call", "call_id": format!("call-{id}-{index}"), "name": name, "arguments": arguments.to_string()}}),
        ));
    }
    let completed = event(
        "response.completed",
        json!({"response": {"id": id, "usage": {"input_tokens": 10, "input_tokens_details": null, "output_tokens": 5, "output_tokens_details": null, "total_tokens": 15}}}),
    );
    for event in events {
        let _ = writer.write_all(event.as_bytes());
        let _ = writer.flush();
    }
    thread::sleep(delay);
    let _ = writer.write_all(completed.as_bytes());
    let _ = writer.flush();
}

fn event(kind: &str, mut data: Value) -> String {
    data["type"] = Value::from(kind);
    format!("event: {kind}\ndata: {data}\n\n")
}
