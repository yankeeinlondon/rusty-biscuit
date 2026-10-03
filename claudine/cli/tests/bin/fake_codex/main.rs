//! Fake `codex` for `tests/l1/codex_app_server.rs`: just enough of Codex's
//! app-server protocol, and of `codex exec --json`, to drive the wrapper's
//! managed launch deterministically on every OS.
//!
//! A built target rather than a script for the reason `fake_goose` gives: a
//! `.cmd` shim cannot carry Claudine's arguments on Windows, and an archived
//! CI run has no compiler.
//!
//! Each launch (1-based `<n>`) writes into `FAKE_CODEX_DIR`: `argv-<n>.json`
//! (its arguments), and either `messages-<n>.jsonl` (every JSON-RPC message it
//! read) or `stdin-<n>.txt` (the exec prompt).
//!
//! `exec` answers its stdin prompt with `ACK:<task>` in exec JSONL, writes
//! `--output-last-message`, and exits 0. `app-server` follows
//! `FAKE_CODEX_PLAN`:
//!
//! - `ok`: initialize (user agent `fake/<FAKE_CODEX_VERSION or 0.157.1>`),
//!   start the thread, run the task as one turn answering `ACK:<task>`;
//! - `refuse-initialize`: refuse `initialize`;
//! - `exit-early`: exit 3 before reading anything;
//! - `approval`: the task's turn asks a command approval and answers
//!   `ACK:<how it was answered>`;
//! - `unreadable-request`: the task's turn sends a request with a null id and
//!   waits;
//! - `fail`: the task's turn fails;
//! - `refuse-task`: refuse the task's `turn/start`;
//! - `repeat`: the task's turn writes the same agent message
//!   [`REPEATED_MESSAGES`] times, answering any `turn/steer` it reads
//!   between them, then waits, so only the wrapper's repetition stop ends it.
//!
//! `thread/read` always reports the thread idle unless a turn is running.
//! Every app-server plan exits 0 once its stdin closes.

use std::fs::{self, OpenOptions};
use std::io::{BufRead, Read, Write};
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;

use serde_json::{Value, json};

/// Messages the `repeat` plan writes: past the default repetition stop of 30.
const REPEATED_MESSAGES: usize = 40;
const THREAD: &str = "fake-thread";
const TURN: &str = "fake-turn";

fn emit(value: Value) {
    let mut out = std::io::stdout().lock();
    writeln!(out, "{value}").unwrap();
    out.flush().unwrap();
}

fn notify(method: &str, params: Value) {
    emit(json!({"method": method, "params": params}));
}

fn turn_value(status: &str) -> Value {
    json!({"id": TURN, "items": [], "status": status, "error": if status == "failed" { json!({"message": "fake model failure"}) } else { Value::Null }})
}

fn agent_message(text: &str) {
    notify("item/completed", json!({"threadId": THREAD, "turnId": TURN, "item": {"type": "agentMessage", "id": "m", "text": text}}));
}

fn complete(status: &str) {
    notify("thread/tokenUsage/updated", json!({"threadId": THREAD, "turnId": TURN, "tokenUsage": {"total": {"totalTokens": 15, "inputTokens": 10, "outputTokens": 5}}}));
    notify("turn/completed", json!({"threadId": THREAD, "turn": turn_value(status)}));
}

fn exec(dir: &std::path::Path, n: u32, args: &[String]) {
    let mut prompt = String::new();
    std::io::stdin().read_to_string(&mut prompt).unwrap();
    fs::write(dir.join(format!("stdin-{n}.txt")), &prompt).unwrap();
    let answer = format!("ACK:{}", prompt.trim());
    emit(json!({"type": "thread.started", "thread_id": "fake-exec-thread"}));
    emit(json!({"type": "turn.started"}));
    emit(json!({"type": "item.completed", "item": {"id": "m", "type": "agent_message", "text": answer}}));
    emit(json!({"type": "turn.completed", "usage": {"input_tokens": 10, "output_tokens": 5}}));
    if let Some(index) = args.iter().position(|arg| arg == "--output-last-message") {
        fs::write(&args[index + 1], &answer).unwrap();
    }
}

fn main() {
    let dir = PathBuf::from(std::env::var_os("FAKE_CODEX_DIR").expect("FAKE_CODEX_DIR"));
    fs::create_dir_all(&dir).unwrap();
    let mut n = 1;
    while OpenOptions::new().write(true).create_new(true).open(dir.join(format!("argv-{n}.json"))).is_err() {
        n += 1;
    }
    let args: Vec<String> = std::env::args().skip(1).collect();
    fs::write(dir.join(format!("argv-{n}.json")), serde_json::to_string(&args).unwrap()).unwrap();
    if args.first().map(String::as_str) != Some("app-server") {
        exec(&dir, n, &args);
        return;
    }

    let plan = std::env::var("FAKE_CODEX_PLAN").unwrap_or_else(|_| "ok".into());
    let version = std::env::var("FAKE_CODEX_VERSION").unwrap_or_else(|_| "0.157.1".into());
    if plan == "exit-early" {
        eprintln!("fake codex: this build has no app-server");
        std::process::exit(3);
    }
    let mut log = OpenOptions::new().create(true).append(true).open(dir.join(format!("messages-{n}.jsonl"))).unwrap();

    // One reader thread, so a plan that waits inside a turn still records and
    // answers what arrives meanwhile.
    let (lines, incoming) = mpsc::channel::<Value>();
    std::thread::spawn(move || {
        for line in std::io::stdin().lock().lines() {
            let Ok(line) = line else { break };
            writeln!(log, "{line}").unwrap();
            if lines.send(serde_json::from_str(&line).unwrap()).is_err() {
                break;
            }
        }
    });

    let mut turn_running = false;
    let answer = |message: &Value, result: Value| emit(json!({"id": message["id"], "result": result}));
    let refuse = |message: &Value, text: &str| emit(json!({"id": message["id"], "error": {"code": -32600, "message": text}}));
    while let Ok(message) = incoming.recv() {
        match message["method"].as_str().unwrap_or_default() {
            "initialize" if plan == "refuse-initialize" => refuse(&message, "fake refusal"),
            "initialize" => answer(&message, json!({"userAgent": format!("fake/{version} (fake os)"), "codexHome": "/fake", "platformFamily": "unix", "platformOs": "fake"})),
            "initialized" => {}
            "thread/start" | "thread/resume" => {
                answer(&message, json!({"thread": {"id": THREAD, "status": {"type": "idle"}}, "model": "fake-model"}));
            }
            "thread/read" => {
                let status = if turn_running { "active" } else { "idle" };
                answer(&message, json!({"thread": {"id": THREAD, "status": {"type": status}}}));
            }
            "turn/steer" => answer(&message, json!({"turnId": TURN})),
            "turn/start" if plan == "refuse-task" => refuse(&message, "fake refusal of the task"),
            "turn/start" => {
                let task = message["params"]["input"][0]["text"].as_str().unwrap_or_default().to_string();
                answer(&message, json!({"turn": turn_value("inProgress")}));
                notify("thread/started", json!({"thread": {"id": THREAD}}));
                notify("turn/started", json!({"threadId": THREAD, "turn": turn_value("inProgress")}));
                match plan.as_str() {
                    "approval" => {
                        emit(json!({"id": 0, "method": "item/commandExecution/requestApproval", "params": {"threadId": THREAD, "turnId": TURN, "itemId": "c"}}));
                        let reply = incoming.recv().unwrap();
                        let how = if reply.get("error").is_some() {
                            "refused"
                        } else if reply["result"]["decision"] == "accept" {
                            "approved"
                        } else {
                            "declined"
                        };
                        agent_message(&format!("ACK:{how}"));
                        complete("completed");
                    }
                    "unreadable-request" => {
                        emit(json!({"id": null, "method": "item/tool/requestUserInput", "params": {}}));
                        turn_running = true;
                    }
                    "fail" => complete("failed"),
                    "repeat" => {
                        turn_running = true;
                        for _ in 0..REPEATED_MESSAGES {
                            agent_message("I will try the same fix again.");
                            std::thread::sleep(Duration::from_millis(20));
                            while let Ok(pending) = incoming.try_recv() {
                                if pending["method"] == "turn/steer" {
                                    answer(&pending, json!({"turnId": TURN}));
                                }
                            }
                        }
                        // Stay in the turn; the wrapper's repetition stop ends the run.
                        std::thread::sleep(Duration::from_secs(30));
                    }
                    _ => {
                        agent_message(&format!("ACK:{task}"));
                        complete("completed");
                    }
                }
            }
            _ => {
                if message.get("id").is_some() && message.get("method").is_some() {
                    refuse(&message, "unknown method");
                }
            }
        }
    }
}
