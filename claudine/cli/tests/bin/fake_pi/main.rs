//! Fake `pi` for `tests/l1/pi_managed_rpc.rs`: just enough of Pi's RPC
//! protocol, and of its JSON stream, to drive the wrapper's managed launch
//! deterministically on every OS.
//!
//! A built target rather than a script for the reason `fake_goose` gives: a
//! `.cmd` shim cannot carry Claudine's arguments on Windows, and an archived
//! CI run has no compiler.
//!
//! Each launch (1-based `<n>`) writes into `FAKE_PI_DIR`:
//! `argv-<n>.json` (its arguments), and either `commands-<n>.jsonl` (every RPC
//! command it read) or `stdin-<n>.txt` (the JSON-stream prompt).
//!
//! `FAKE_PI_PLAN` picks the RPC behavior; JSON mode (`--mode json`) always
//! answers its stdin prompt and exits:
//!
//! - `ok`: ready, accept the task, one turn answering `ACK:<task>`, settle;
//! - `refuse-ready`: refuse the readiness `get_state`;
//! - `exit-early`: exit 3 before reading anything;
//! - `dialog`: the task asks a `confirm`, answers `ACK:<cancelled|confirmed>`,
//!   and only then is the task's `prompt` answered;
//! - `custom-ui`: the task sends an undocumented UI method and waits, its
//!   `prompt` never answered;
//! - `crash`: after the task, start a turn and exit 3;
//! - `repeat`: after the task, start a turn that writes the same line
//!   [`REPEATED_LINES`] times and then waits, so only the wrapper's repetition
//!   stop ends it.
//!
//! Every RPC plan exits 0 once its stdin closes.

use std::fs::{self, OpenOptions};
use std::io::{BufRead, Read, Write};
use std::path::PathBuf;

use serde_json::{Value, json};

/// Lines the `repeat` plan writes: past the default repetition stop of 30.
const REPEATED_LINES: usize = 40;

fn emit(value: Value) {
    let mut out = std::io::stdout().lock();
    writeln!(out, "{value}").unwrap();
    out.flush().unwrap();
}

fn turn(answer: &str) {
    emit(json!({"type": "agent_start"}));
    emit(json!({"type": "turn_start"}));
    emit(json!({"type": "message_update", "assistantMessageEvent": {"type": "text_delta", "delta": answer}}));
    emit(json!({"type": "message_end", "message": {"role": "assistant", "stopReason": "stop", "model": "fixture"}}));
    emit(json!({"type": "turn_end"}));
    emit(json!({"type": "agent_end", "willRetry": false}));
    emit(json!({"type": "agent_settled"}));
}

fn main() {
    let dir = PathBuf::from(std::env::var_os("FAKE_PI_DIR").expect("FAKE_PI_DIR"));
    fs::create_dir_all(&dir).unwrap();
    let mut n = 1;
    while OpenOptions::new().write(true).create_new(true).open(dir.join(format!("argv-{n}.json"))).is_err() {
        n += 1;
    }
    let args: Vec<String> = std::env::args().skip(1).collect();
    fs::write(dir.join(format!("argv-{n}.json")), serde_json::to_string(&args).unwrap()).unwrap();

    let mode = args.iter().position(|arg| arg == "--mode").and_then(|index| args.get(index + 1)).cloned();
    if mode.as_deref() == Some("json") {
        let mut prompt = String::new();
        std::io::stdin().read_to_string(&mut prompt).unwrap();
        fs::write(dir.join(format!("stdin-{n}.txt")), &prompt).unwrap();
        emit(json!({"type": "session", "id": "fake-json-session", "cwd": "."}));
        turn(&format!("ACK:{}", prompt.trim()));
        return;
    }

    let plan = std::env::var("FAKE_PI_PLAN").unwrap_or_else(|_| "ok".into());
    if plan == "exit-early" {
        eprintln!("fake pi: this build cannot start RPC mode");
        std::process::exit(3);
    }
    let mut log = OpenOptions::new().create(true).append(true).open(dir.join(format!("commands-{n}.jsonl"))).unwrap();
    // Every turn is emitted whole before the next command is read, so any
    // `get_state` sees an idle session.
    let mut pending_prompt: Option<Value> = None;
    for line in std::io::stdin().lock().lines() {
        let Ok(line) = line else { break };
        writeln!(log, "{line}").unwrap();
        let command: Value = serde_json::from_str(&line).unwrap();
        let respond = |success: bool, data: Option<Value>| {
            let mut response = json!({"type": "response", "id": command["id"], "command": command["type"], "success": success});
            if let Some(data) = data {
                response["data"] = data;
            }
            if !success {
                response["error"] = json!("fake refusal");
            }
            emit(response);
        };
        match command["type"].as_str().unwrap_or_default() {
            "get_state" if plan == "refuse-ready" => respond(false, None),
            "get_state" => respond(
                true,
                Some(json!({"sessionId": "fake-session", "isStreaming": false, "isCompacting": false, "pendingMessageCount": 0})),
            ),
            // Like Pi, an extension command answers its `prompt` only once
            // the command (and any dialog it opened) has finished.
            "prompt" => {
                let message = command["message"].as_str().unwrap_or_default().to_string();
                match plan.as_str() {
                    "dialog" => {
                        pending_prompt = Some(command.clone());
                        emit(json!({"type": "extension_ui_request", "id": "ui-1", "method": "confirm", "title": "Fake", "message": "Approve?"}));
                    }
                    "custom-ui" => emit(json!({"type": "extension_ui_request", "id": "ui-2", "method": "custom"})),
                    "crash" => {
                        respond(true, None);
                        emit(json!({"type": "agent_start"}));
                        std::process::exit(3);
                    }
                    "repeat" => {
                        respond(true, None);
                        emit(json!({"type": "agent_start"}));
                        emit(json!({"type": "turn_start"}));
                        for _ in 0..REPEATED_LINES {
                            emit(json!({"type": "message_update", "assistantMessageEvent": {"type": "text_delta", "delta": "I will try the same fix again.\n"}}));
                        }
                        // Stay in the turn; the wrapper's repetition stop ends the run.
                        std::thread::sleep(std::time::Duration::from_secs(30));
                    }
                    _ => {
                        respond(true, None);
                        turn(&format!("ACK:{message}"));
                    }
                }
            }
            "extension_ui_response" => {
                let answer = if command["cancelled"] == json!(true) { "cancelled" } else { "confirmed" };
                turn(&format!("ACK:{answer}"));
                if let Some(prompt) = pending_prompt.take() {
                    emit(json!({"type": "response", "id": prompt["id"], "command": "prompt", "success": true}));
                }
            }
            _ => respond(false, None),
        }
    }
}
