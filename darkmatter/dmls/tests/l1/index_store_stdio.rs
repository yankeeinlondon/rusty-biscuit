//! Level-1 wiring of `DMLS_CACHE_DIR` through the **compiled** `dmls`
//! binary over real stdio pipes: the variable reaches the server from the
//! process environment, and all four store-backed requests answer whether
//! the stores are off (memory, one `warn`, no store file) or corrupt
//! (recreated once, then filled).
//!
//! The request behavior itself is proven in process (`index_store_requests`,
//! `index_store_live`); this file proves only the launch path. No window or
//! terminal is involved.

use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::process::{ChildStdin, ChildStdout, Command, Stdio};
use std::sync::{Arc, mpsc};
use std::thread;
use std::time::Duration;

use biscuit_test_harness::bin_exe;
use dmls::clock::SystemClock;
use dmls::index_store::{IndexStore, store_path};
use serde_json::{Value, json};

use crate::stdio_subprocess::{ChildGuard, read_message, write_message};

const TARGET: &str = "# Target\n\nBody.\n";
const REFERRER: &str = "# Referrer\n\nSee [the target](target.md#target).\n\nAlso [a path](target.md).\n";

/// An isolated process root: a workspace folder with `target.md` and a
/// never-opened `referrer.md`, and fixture home and cache directories.
struct Session {
    dir: tempfile::TempDir,
}

impl Session {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        for name in ["home", "cache", "workspace"] {
            std::fs::create_dir(dir.path().join(name)).unwrap();
        }
        let session = Self { dir };
        std::fs::write(session.target(), TARGET).unwrap();
        std::fs::write(session.workspace().join("referrer.md"), REFERRER).unwrap();
        session
    }

    fn workspace(&self) -> PathBuf {
        self.dir.path().join("workspace")
    }

    fn target(&self) -> PathBuf {
        self.workspace().join("target.md")
    }

    fn log(&self) -> PathBuf {
        self.dir.path().join("dmls.log")
    }

    /// Every `.sqlite` file under the process root.
    fn store_files(&self) -> Vec<PathBuf> {
        fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
            for entry in std::fs::read_dir(dir).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    walk(&path, out);
                } else if path.extension().is_some_and(|extension| extension == "sqlite") {
                    out.push(path);
                }
            }
        }
        let mut out = Vec::new();
        walk(self.dir.path(), &mut out);
        out
    }

    /// Runs one editor session with `DMLS_CACHE_DIR` set to `cache_dir`:
    /// opens `target.md`, then asks references, heading rename, file
    /// rename, and workspace symbols. Returns the four results in that order.
    fn run(&self, cache_dir: &str) -> Vec<Value> {
        let home = self.dir.path().join("home");
        let child = Command::new(bin_exe!("dmls"))
            .args(["--log-level", "info", "--log-file"])
            .arg(self.log())
            .current_dir(self.dir.path())
            .env_remove("RUST_LOG")
            .env("DMLS_CACHE_DIR", cache_dir)
            .env("HOME", &home)
            .env("USERPROFILE", &home)
            .env("XDG_CACHE_HOME", self.dir.path().join("cache"))
            .env("LOCALAPPDATA", self.dir.path().join("cache"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn the compiled dmls binary");
        let mut child = ChildGuard::new(child);
        let mut stdin = child.child_mut().stdin.take().unwrap();
        let stdout = child.child_mut().stdout.take().unwrap();
        let root = url::Url::from_directory_path(self.workspace()).unwrap().to_string();
        let target = url::Url::from_file_path(self.target()).unwrap().to_string();
        let goal = url::Url::from_file_path(self.workspace().join("goal.md")).unwrap().to_string();

        let requests = [
            (
                "textDocument/references",
                json!({
                    "textDocument": { "uri": target },
                    "position": { "line": 0, "character": 3 },
                    "context": { "includeDeclaration": false }
                }),
            ),
            (
                "textDocument/rename",
                json!({ "textDocument": { "uri": target }, "position": { "line": 0, "character": 3 }, "newName": "Goal" }),
            ),
            ("workspace/willRenameFiles", json!({ "files": [ { "oldUri": target, "newUri": goal } ] })),
            ("workspace/symbol", json!({ "query": "Referrer" })),
        ];
        let initialize = json!({
            "processId": null,
            "capabilities": { "general": { "positionEncodings": ["utf-16"] } },
            "workspaceFolders": [ { "uri": root, "name": "workspace" } ]
        });
        let opened = json!({ "textDocument": { "uri": target, "languageId": "markdown", "version": 1, "text": TARGET } });

        let (tx, rx) = mpsc::channel();
        let worker = thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            let outcome = || -> Result<Vec<Value>, String> {
                exchange(&mut stdin, &mut reader, 1, "initialize", initialize)?;
                notify(&mut stdin, "initialized", json!({}));
                notify(&mut stdin, "textDocument/didOpen", opened);
                let mut answers = Vec::new();
                for (id, (method, params)) in (2..).zip(requests) {
                    answers.push(exchange(&mut stdin, &mut reader, id, method, params)?);
                }
                exchange(&mut stdin, &mut reader, 99, "shutdown", Value::Null)?;
                notify(&mut stdin, "exit", Value::Null);
                Ok(answers)
            };
            let _ = tx.send(outcome());
        });

        let outcome = rx.recv_timeout(Duration::from_secs(30));
        if outcome.is_err() {
            child.terminate();
        }
        let _ = worker.join();
        let status = child.wait_timeout(Duration::from_secs(10)).expect("wait for dmls");
        child.terminate();
        let answers = outcome.expect("the session finished before the timeout").expect("every request answered");
        assert!(status.is_some_and(|status| status.success()), "dmls exits cleanly: {status:?}");
        answers
            .into_iter()
            .map(|answer| {
                assert!(answer["error"].is_null(), "{answer}");
                answer["result"].clone()
            })
            .collect()
    }

    fn log_lines(&self, needle: &str) -> Vec<String> {
        std::fs::read_to_string(self.log())
            .unwrap_or_default()
            .lines()
            .filter(|line| line.contains(needle))
            .map(str::to_string)
            .collect()
    }
}

fn notify(stdin: &mut ChildStdin, method: &str, params: Value) {
    write_message(stdin, &json!({ "jsonrpc": "2.0", "method": method, "params": params }));
}

/// Sends a request and reads until its response, skipping server
/// notifications and server-to-client requests (both carry a `method`).
fn exchange(
    stdin: &mut ChildStdin,
    reader: &mut BufReader<ChildStdout>,
    id: i64,
    method: &str,
    params: Value,
) -> Result<Value, String> {
    write_message(stdin, &json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }));
    loop {
        let message = read_message(reader).ok_or_else(|| format!("stream ended before the {method} response"))?;
        if message["id"] == json!(id) && message.get("method").is_none() {
            return Ok(message);
        }
    }
}

/// The four answers name the never-opened referrer.
fn assert_answers(session: &Session, answers: &[Value]) {
    let referrer = url::Url::from_file_path(session.workspace().join("referrer.md")).unwrap().to_string();
    let lines = |locations: &Value| -> Vec<(String, u64)> {
        locations
            .as_array()
            .unwrap()
            .iter()
            .map(|location| (location["uri"].as_str().unwrap().to_string(), location["range"]["start"]["line"].as_u64().unwrap()))
            .collect()
    };
    assert_eq!(lines(&answers[0]), vec![(referrer.clone(), 2)], "references");
    let edited = |edit: &Value| -> Vec<String> {
        edit["changes"][referrer.as_str()].as_array().unwrap().iter().map(|edit| edit["newText"].as_str().unwrap().to_string()).collect()
    };
    assert_eq!(edited(&answers[1]), vec!["goal".to_string()], "heading rename");
    assert_eq!(edited(&answers[2]), vec!["goal.md".to_string()], "file rename");
    let names: Vec<&str> = answers[3].as_array().unwrap().iter().map(|symbol| symbol["name"].as_str().unwrap()).collect();
    assert_eq!(names, vec!["Referrer", "Referrer"], "workspace symbols");
}

/// `DMLS_CACHE_DIR=off` from the process environment: memory, one `warn`,
/// and no store file anywhere under the process root.
#[test]
fn the_binary_answers_from_memory_when_the_stores_are_off() {
    let session = Session::new();
    let answers = session.run("off");
    assert_answers(&session, &answers);
    let warns = session.log_lines("index store unavailable, using memory");
    assert_eq!(warns.len(), 1, "{warns:#?}");
    assert!(warns[0].contains("WARN"), "{warns:#?}");
    assert_eq!(session.store_files(), Vec::<PathBuf>::new());
}

/// A corrupt store at the `DMLS_CACHE_DIR` location is recreated once and
/// filled; the requests answer.
#[test]
fn the_binary_recreates_a_corrupt_store_and_answers() {
    let session = Session::new();
    let stores = session.dir.path().join("stores");
    std::fs::create_dir(&stores).unwrap();
    let store = store_path(&stores, &session.workspace());
    std::fs::write(&store, b"this is not a database").unwrap();

    let answers = session.run(stores.to_str().unwrap());
    assert_answers(&session, &answers);
    assert_eq!(session.log_lines("index store recreated").len(), 1, "{:#?}", session.log_lines("index store"));
    assert!(session.log_lines("index store unavailable").is_empty());
    let (reopened, _) = IndexStore::open(&store, &session.workspace(), Arc::new(SystemClock)).expect("a valid store now");
    assert_eq!(reopened.hashes().unwrap().len(), 2);
}
