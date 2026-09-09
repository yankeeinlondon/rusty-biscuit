//! Opt-in Pi RPC experiments using a real child and a deterministic local model.
//! No result here activates a production steering adapter.

use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use serde_json::{Value, json};
use tempfile::TempDir;

const DEADLINE: Duration = Duration::from_secs(45);

#[test]
#[ignore = "requires explicit Pi binary; controlled switch and abrupt termination experiments"]
fn real_pi_rpc_switch_and_crash() {
    let binary = PathBuf::from(std::env::var_os("CLAUDINE_PI_BINARY").expect("set Pi binary"));
    let version = Command::new(&binary).arg("--version").output().unwrap();
    assert!(version.status.success());
    let version = String::from_utf8(version.stdout).unwrap();
    let mut results = Vec::new();
    for during_switch in [true, false] {
        let mut rpc = RpcChild::start(&binary);
        let old_session = rpc.request("initial", "get_state")["data"]["sessionId"].clone();
        let seed = rpc.send(json!({"id":"seed","type":"prompt","message":"TEMPLATE_NONCE"}));
        rpc.response(seed, "seed");
        rpc.wait(seed, |e| e["type"] == "agent_settled");
        rpc.release("hold-switch");
        let switch = rpc.send(json!({"id":"switch","type":"new_session"}));
        rpc.wait_marker("switch-entered");
        assert_eq!(rpc.request("while-switching", "get_state")["data"]["sessionId"], old_session);
        if during_switch {
            let steer = rpc.send(json!({"id":"queued","type":"steer","message":"STEERING_NONCE"}));
            rpc.response(steer, "queued");
        }
        rpc.release("release-switch");
        let switched = rpc.response(switch, "switch");
        assert_eq!(switched["data"]["cancelled"], false);
        let new_session = rpc.request("after-switch", "get_state")["data"]["sessionId"].clone();
        assert_ne!(new_session, old_session);
        if !during_switch {
            let steer = rpc.send(json!({"id":"queued","type":"steer","message":"STEERING_NONCE"}));
            rpc.response(steer, "queued");
        }
        let next = rpc.send(json!({"id":"next","type":"prompt","message":"TEMPLATE_NONCE"}));
        rpc.response(next, "next");
        rpc.wait(next, |e| e["type"] == "agent_settled");
        let messages = rpc.request("messages", "get_messages");
        let count = messages["data"]["messages"].as_array().unwrap().iter()
            .filter(|m| m["role"] == "user" && m["content"].to_string().contains("STEERING_NONCE")).count();
        assert_eq!(count, usize::from(!during_switch));
        assert_eq!(stored_steering_count(&rpc), usize::from(!during_switch));
        assert_eq!(rpc.marker("steering-observed").exists(), !during_switch);
        results.push(json!({"scenario":if during_switch {"steer_during_held_switch"} else {"steer_after_switch"},
            "switch_acknowledged":true,"steering_acknowledged":true,
            "old_identity_observable_during_switch":true,"session_changed":true,
            "new_session_steering_messages":count,
            "persisted_steering_user_messages":stored_steering_count(&rpc),
            "steering_reached_model":rpc.marker("steering-observed").exists()}));
    }
    let (mut rpc, _) = failure_probe_child(&binary);
    let steer = rpc.send(json!({"id":"queued","type":"steer","message":"STEERING_NONCE"}));
    rpc.response(steer, "queued");
    rpc.child.kill().unwrap();
    let mut killed = finish_disconnected_child(&mut rpc);
    killed["scenario"] = json!("kill_provider_after_steering_ack");
    killed["acknowledged_before_kill"] = json!(true);
    assert_eq!(killed["steering_reached_model"], false);
    assert_eq!(killed["persisted_steering_user_messages"], 0);
    results.push(killed);
    let report = json!({"provider":"pi","provider_version":version.trim(),"os":std::env::consts::OS,
        "fixture":"real Pi RPC with deterministic local model, gated session switch, and cooperative tools",
        "production_activation":false,"results":results,
        "limitations":["controlled interleaving, not exhaustive scheduling stress",
            "provider process killed after observed acceptance; no host crash, disk durability, or restart tested",
            "no real cloud inference; isolated profile with fixture extensions enabled"]});
    if let Some(path) = std::env::var_os("CLAUDINE_PI_RACE_REPORT") {
        fs::write(path, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    }
}

struct RpcChild {
    child: Child,
    input: Option<ChildStdin>,
    receiver: Receiver<Value>,
    readers: Vec<JoinHandle<()>>,
    events: Vec<Value>,
    started: Instant,
    workspace: TempDir,
}

fn failure_probe_child(binary: &Path) -> (RpcChild, Value) {
    let mut rpc = RpcChild::start(binary);
    let session = rpc.request("initial", "get_state")["data"]["sessionId"].clone();
    let from = rpc.send(json!({"id":"batch","type":"prompt","message":"PROBE_TOOL_BATCH"}));
    rpc.response(from, "batch");
    rpc.wait_marker("tool-a-started");
    rpc.wait_marker("tool-b-started");
    (rpc, session)
}

fn finish_disconnected_child(rpc: &mut RpcChild) -> Value {
    let deadline = Instant::now() + DEADLINE;
    let status = loop {
        if let Some(status) = rpc.child.try_wait().unwrap() { break status; }
        assert!(Instant::now() < deadline, "Pi did not exit after stdin EOF");
        thread::sleep(Duration::from_millis(10));
    };
    for reader in rpc.readers.drain(..) { reader.join().unwrap(); }
    rpc.events.extend(rpc.receiver.try_iter());
    json!({"exit_code":status.code(),
        "persisted_steering_user_messages":stored_steering_count(rpc),
        "acknowledgments_observed_after_drain":rpc.events.iter().filter(|e| e["type"]=="response" && e["id"]=="queued" && e["success"]==true).count(),
        "steering_reached_model":rpc.marker("steering-observed").exists(),
        "tool_a_finished_normally":rpc.marker("tool-a-finished").exists(),
        "tool_b_finished_normally":rpc.marker("tool-b-finished").exists()})
}

fn stored_steering_count(rpc: &RpcChild) -> usize {
    fs::read_dir(rpc.workspace.path().join("sessions")).unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "jsonl"))
        .flat_map(|path| fs::read_to_string(path).unwrap().lines()
            .map(|line| serde_json::from_str::<Value>(line).unwrap()).collect::<Vec<_>>())
        .filter(|entry| entry["type"] == "message" && entry["message"]["role"] == "user"
            && entry["message"]["content"].to_string().contains("STEERING_NONCE"))
        .count()
}

#[test]
#[ignore = "requires explicit Pi binary; abort and EOF tests only use disposable children"]
fn real_pi_rpc_abort_and_disconnect() {
    let binary = PathBuf::from(std::env::var_os("CLAUDINE_PI_BINARY").expect("set Pi binary"));
    let version = Command::new(&binary).arg("--version").output().unwrap();
    assert!(version.status.success());
    let version = String::from_utf8(version.stdout).unwrap();
    let mut results = Vec::new();
    for clear_first in [false, true] {
        let (mut rpc, session) = failure_probe_child(&binary);
        let from = rpc.send(json!({"id":"queued","type":"steer","message":"STEERING_NONCE"}));
        rpc.response(from, "queued");
        if clear_first {
            let cleared = rpc.request("clear", "clear_queue");
            assert_eq!(cleared["data"]["steering"], json!(["STEERING_NONCE"]));
        }
        rpc.request("abort", "abort");
        let state = rpc.request("after-abort", "get_state");
        assert_eq!(state["data"]["sessionId"], session);
        assert_eq!(state["data"]["isStreaming"], false);
        assert!(!rpc.marker("tool-a-finished").exists());
        assert!(!rpc.marker("tool-b-finished").exists());
        let delivered = rpc.marker("steering-observed").exists();
        let messages = rpc.request("after-abort-messages", "get_messages");
        let count = messages["data"]["messages"].as_array().unwrap().iter()
            .filter(|m| m["role"]=="user" && m["content"].to_string().contains("STEERING_NONCE")).count();
        let bad = rpc.send(json!({"id":"bad-replacement","type":"prompt","message":7}));
        let rejected = rpc.wait(bad, |e| e["type"]=="response" && e["id"]=="bad-replacement");
        assert_eq!(rejected["success"], false);
        assert_eq!(rpc.request("after-rejection", "get_state")["data"]["isStreaming"], false);
        let persisted = stored_steering_count(&rpc);
        let resume = rpc.send(json!({"id":"replacement","type":"prompt","message":"TEMPLATE_NONCE"}));
        rpc.response(resume, "replacement");
        rpc.wait(resume, |e| e["type"]=="agent_settled");
        let delivered_on_next_turn = rpc.marker("steering-observed").exists();
        assert!(!delivered, "abort must not be mistaken for model delivery");
        assert_eq!(count, usize::from(!clear_first));
        assert_eq!(persisted, usize::from(!clear_first));
        assert_eq!(delivered_on_next_turn, !clear_first,
            "uncleared steering history influences the explicit next turn");
        results.push(json!({"scenario":if clear_first {"clear_then_abort"} else {"abort_with_queue"},
            "abort_acknowledged":true,"same_session":true,"tools_finished_normally":false,
            "steering_reached_model":delivered,"steering_user_messages":count,
            "persisted_steering_user_messages":persisted,
            "steering_reached_model_on_explicit_next_turn":delivered_on_next_turn,
            "invalid_replacement_rejected":true}));
    }
    for observed_ack in [false, true] {
        let (mut rpc, _) = failure_probe_child(&binary);
        let from = rpc.send(json!({"id":"queued","type":"steer","message":"STEERING_NONCE"}));
        if observed_ack { rpc.response(from, "queued"); }
        rpc.input.take();
        let mut result = finish_disconnected_child(&mut rpc);
        assert_eq!(result["exit_code"], 0);
        assert_eq!(result["steering_reached_model"], false);
        assert_eq!(result["persisted_steering_user_messages"], 0,
            "EOF while tools are held does not durably save acknowledged steering");
        assert_eq!(result["tool_a_finished_normally"], false);
        assert_eq!(result["tool_b_finished_normally"], false);
        result["scenario"] = json!(if observed_ack {"stdin_eof_after_ack"} else {"stdin_eof_without_waiting_for_ack"});
        result["acknowledged_before_disconnect"] = json!(observed_ack);
        results.push(result);
    }
    let report = json!({"provider":"pi","provider_version":version.trim(),"os":std::env::consts::OS,
        "fixture":"real Pi RPC with deterministic local model and cooperative in-process tools",
        "production_activation":false,"results":results,
        "limitations":["stdin EOF is a half-close; stdout remains observable for the experiment",
            "not waiting for acknowledgment does not prove the provider had not accepted the request",
            "no cloud model, OS subprocess cleanup, abrupt host failure, or full-duplex transport loss tested",
            "invalid replacement is a pre-acceptance protocol error, not a post-acceptance model failure"]});
    if let Some(path) = std::env::var_os("CLAUDINE_PI_FAILURE_REPORT") {
        fs::write(path, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    }
}

impl RpcChild {
    fn start(binary: &Path) -> Self {
        let workspace = tempfile::tempdir().expect("temporary project");
        let root = workspace.path();
        for dir in ["agent", "sessions", ".pi/prompts", ".pi/skills/probe-skill", "probe"] {
            fs::create_dir_all(root.join(dir)).unwrap();
        }
        fs::write(root.join("AGENTS.md"), "Fixture context marker: CONTEXT_NONCE\n").unwrap();
        fs::write(root.join(".pi/prompts/probe-template.md"), "---\ndescription: Fixture template\n---\nTEMPLATE_NONCE\n").unwrap();
        fs::write(root.join(".pi/skills/probe-skill/SKILL.md"), "---\nname: probe-skill\ndescription: Fixture skill\n---\nSKILL_NONCE\n").unwrap();
        let extension = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/steering/pi-probe.ts");
        let mut child = Command::new(binary)
            .current_dir(root)
            .env("PI_CODING_AGENT_DIR", root.join("agent"))
            .env("CLAUDINE_PI_PROBE_DIR", root.join("probe"))
            .env("PI_TELEMETRY", "0")
            .args(["--mode", "rpc", "--offline", "--approve", "--provider", "claudine-probe", "--model", "fixture", "--thinking", "off", "--extension"])
            .arg(extension)
            .arg("--session-dir").arg(root.join("sessions"))
            .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped())
            .spawn().expect("launch explicitly selected Pi binary");
        let input = child.stdin.take();
        let stdout = child.stdout.take().unwrap();
        let mut stderr = child.stderr.take().unwrap();
        let (sender, receiver) = mpsc::channel();
        let stdout_reader = thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            let mut line = Vec::new();
            loop {
                line.clear();
                match reader.read_until(b'\n', &mut line) {
                    Ok(0) | Err(_) => break,
                    Ok(_) => {
                        let record = serde_json::from_slice(&line)
                            .unwrap_or_else(|_| json!({"type":"invalid_json_record"}));
                        if sender.send(record).is_err() { break; }
                    }
                }
            }
        });
        let stderr_reader = thread::spawn(move || {
            // Continuously drain diagnostics without persisting possibly sensitive text.
            let mut bytes = [0; 4096];
            while matches!(stderr.read(&mut bytes), Ok(n) if n > 0) {}
        });
        Self { child, input, receiver, readers: vec![stdout_reader, stderr_reader],
            events: Vec::new(), started: Instant::now(), workspace }
    }

    fn send(&mut self, value: Value) -> usize {
        let start = self.events.len();
        let input = self.input.as_mut().unwrap();
        serde_json::to_writer(&mut *input, &value).unwrap();
        input.write_all(b"\n").unwrap();
        input.flush().unwrap();
        start
    }

    fn wait(&mut self, from: usize, predicate: impl Fn(&Value) -> bool) -> Value {
        let deadline = Instant::now() + DEADLINE;
        loop {
            if let Some(event) = self.events[from..].iter().find(|event| predicate(event)) {
                return event.clone();
            }
            let remaining = deadline.saturating_duration_since(Instant::now());
            let event = self.receiver.recv_timeout(remaining)
                .expect("Pi response/event deadline or child pipe closure");
            assert_ne!(event["type"], "invalid_json_record", "Pi stdout must remain JSONL");
            self.events.push(event);
        }
    }

    fn request(&mut self, id: &str, command: &str) -> Value {
        let start = self.send(json!({"id":id,"type":command}));
        self.response(start, id)
    }

    fn response(&mut self, from: usize, id: &str) -> Value {
        let event = self.wait(from, |e| e["type"] == "response" && e["id"] == id);
        assert_eq!(event["success"], true, "RPC command {id} must succeed");
        event
    }

    fn marker(&self, name: &str) -> PathBuf {
        self.workspace.path().join("probe").join(name)
    }

    fn wait_marker(&mut self, name: &str) {
        let deadline = Instant::now() + DEADLINE;
        while !self.marker(name).is_file() {
            assert!(Instant::now() < deadline, "missing fixture marker {name}");
            assert!(self.child.try_wait().unwrap().is_none(), "Pi exited before {name}");
            thread::sleep(Duration::from_millis(10));
        }
    }

    fn release(&self, name: &str) {
        fs::write(self.marker(name), b"release").unwrap();
    }
}

impl Drop for RpcChild {
    fn drop(&mut self) {
        self.input.take();
        let _ = self.child.kill();
        let _ = self.child.wait();
        for reader in self.readers.drain(..) { let _ = reader.join(); }
    }
}

#[test]
#[ignore = "requires explicit Pi binary; runs disposable real-provider protocol experiments"]
fn real_pi_rpc_steering_contract() {
    let binary = PathBuf::from(std::env::var_os("CLAUDINE_PI_BINARY")
        .expect("set CLAUDINE_PI_BINARY to a Sniff-discovered Pi executable"));
    let version = Command::new(&binary).arg("--version").output().unwrap();
    assert!(version.status.success(), "Pi version query must succeed");
    let version = String::from_utf8(version.stdout).unwrap();
    let mut rpc = RpcChild::start(&binary);
    let state = rpc.request("initial-state", "get_state");
    let session = state["data"]["sessionId"].clone();
    assert!(session.as_str().is_some_and(|s| !s.is_empty()));
    assert_eq!(state["data"]["model"]["provider"], "claudine-probe");

    let start = rpc.send(json!({"id":"batch","type":"prompt","message":"PROBE_TOOL_BATCH"}));
    rpc.response(start, "batch");
    rpc.wait_marker("tool-a-started");
    let steer = rpc.send(json!({"id":"steer","type":"steer","message":"STEERING_NONCE"}));
    rpc.response(steer, "steer");
    let duplicate = rpc.send(json!({"id":"steer","type":"steer","message":"STEERING_NONCE"}));
    rpc.response(duplicate, "steer");
    assert!(!rpc.marker("tool-a-finished").exists(), "acceptance precedes tool completion");
    rpc.release("release-a");
    rpc.wait_marker("tool-b-started");
    assert!(!rpc.marker("steering-observed").exists(), "steering must wait for the full tool batch");
    rpc.release("release-b");
    rpc.wait(start, |e| e["type"] == "agent_settled");
    rpc.wait_marker("steering-observed");
    assert!(rpc.marker("tool-a-finished").exists());
    assert!(rpc.marker("tool-b-finished").exists());
    assert_eq!(rpc.request("after-batch", "get_state")["data"]["sessionId"], session);
    rpc.wait_marker("context-observed");
    let messages = rpc.request("messages", "get_messages");
    let duplicate_count = messages["data"]["messages"].as_array().unwrap().iter()
        .filter(|m| m["role"] == "user")
        .filter(|m| m["content"].to_string().contains("STEERING_NONCE")).count();
    assert_eq!(duplicate_count, 2, "correlation IDs do not suppress duplicate submissions");

    for (id, message, marker) in [
        ("template", "/probe-template", "template-observed"),
        ("skill", "/skill:probe-skill", "skill-observed"),
    ] {
        let from = rpc.send(json!({"id":id,"type":"prompt","message":message}));
        rpc.response(from, id);
        rpc.wait(from, |e| e["type"] == "agent_settled");
        rpc.wait_marker(marker);
        assert_eq!(rpc.request(&format!("state-{id}"), "get_state")["data"]["sessionId"], session);
    }
    let calls_before = fs::read(rpc.marker("model-call-count")).unwrap();
    let handled = rpc.send(json!({"id":"handled","type":"prompt","message":"/probe-handled"}));
    rpc.response(handled, "handled");
    rpc.wait_marker("extension-handled");
    assert_eq!(fs::read(rpc.marker("model-call-count")).unwrap(), calls_before,
        "extension-handled acceptance must not imply a model invocation");

    let reset = rpc.request("new-session", "new_session");
    assert_eq!(reset["data"]["cancelled"], false);
    assert_ne!(rpc.request("new-state", "get_state")["data"]["sessionId"], session,
        "current session identity is mutable within one process");
    let ui = rpc.send(json!({"id":"ui","type":"prompt","message":"/probe-ui"}));
    rpc.wait(ui, |e| e["type"] == "extension_ui_request" && e["method"] == "confirm");
    rpc.request("ui-state", "get_state");
    assert!(!rpc.events[ui..].iter().any(|e| e["type"] == "response" && e["id"] == "ui"),
        "no fabricated answer is sent to the pending extension dialog");

    let report = json!({"provider":"pi","provider_version":version.trim(),"os":std::env::consts::OS,
        "fixture":"deterministic local model through real Pi RPC",
        "production_activation":false,"elapsed_ms":rpc.started.elapsed().as_millis(),
        "assertions":["correlated acceptance before tool completion", "full tool batch preserved",
            "steering present in subsequent model input", "same conversation after delivery", "agent_settled observed",
            "duplicate request ID does not deduplicate user text", "context loaded", "template expanded", "skill expanded",
            "extension-handled prompt does not invoke model", "session identity changes on new_session",
            "unanswered extension confirmation remains pending"],
        "limitations":["deterministic local model, not cloud inference", "isolated fixture resources, not arbitrary user extensions",
            "no production adapter tested", "no concurrent writer or switch race tested", "no disconnect or abort cleanup tested"],
        "events":rpc.events.iter().map(|e| json!({"type":e["type"],"id":e["id"],"command":e["command"],"success":e["success"],"method":e["method"]})).collect::<Vec<_>>()});
    if let Some(path) = std::env::var_os("CLAUDINE_PI_REPORT") {
        fs::write(path, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    }
}
