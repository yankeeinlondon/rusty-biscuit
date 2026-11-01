//! The compiled `dmls` binary types `content_policy` from the embedded
//! baseline alone: launched over stdio from a directory outside the checkout
//! that holds only the test documents, with a fixture-owned home, config,
//! cache, and data directory and no schema variable in its environment.
//!
//! `content_policy_baseline` proves the editor features in memory; this file
//! proves the shipped binary reaches the same schema with nothing on disk.

use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::process::{ChildStdin, ChildStdout, Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use biscuit_test_harness::bin_exe;
use serde_json::{Value, json};

use crate::content_policy_baseline::content_policy_fixtures::{
    ACCEPTANCE_REJECTIONS, MIXED_POLICY_FRONTMATTER, RULE_SUGGESTIONS, declaration, render,
};
use crate::stdio_subprocess::{ChildGuard, read_message, write_message};

/// Schema inputs a host shell may export; none may reach the child.
const SCHEMA_VARIABLES: [&str; 3] = ["BASELINE_SCHEMA", "DARKMATTER_NO_BASELINE_SCHEMA", "SCHEMAS_DIR"];

const COMPLETION_DOCUMENT: &str =
    "---\ncontent_policy:\n  - ValidFor(3mo)\n  - \n  - rule: \n    action: archive\n  - rule: TimeSensitive\n    action: \n---\n\nBody\n";

/// One client side of the stdio conversation. Server notifications that
/// arrive while a response is awaited are kept for later lookups; server
/// requests are left unanswered, as the in-memory fixture does.
struct Client {
    stdin: ChildStdin,
    reader: BufReader<ChildStdout>,
    notifications: Vec<Value>,
    next_id: i64,
}

impl Client {
    fn read(&mut self) -> Result<Value, String> {
        read_message(&mut self.reader).ok_or_else(|| "the server closed its output".to_string())
    }

    fn notify(&mut self, method: &str, params: Value) {
        write_message(&mut self.stdin, &json!({ "jsonrpc": "2.0", "method": method, "params": params }));
    }

    fn request(&mut self, method: &str, params: Value) -> Result<Value, String> {
        self.next_id += 1;
        let id = self.next_id;
        write_message(&mut self.stdin, &json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }));
        loop {
            let message = self.read()?;
            if message.get("method").is_none() && message["id"] == json!(id) {
                return Ok(message);
            }
            if message.get("id").is_none() {
                self.notifications.push(message);
            }
        }
    }

    fn diagnostics(&mut self, uri: &str) -> Result<Vec<Value>, String> {
        loop {
            if let Some(index) = self.notifications.iter().position(|notification| {
                notification["method"] == "textDocument/publishDiagnostics" && notification["params"]["uri"] == uri
            }) {
                let notification = self.notifications.remove(index);
                return Ok(notification["params"]["diagnostics"].as_array().cloned().unwrap_or_default());
            }
            let message = self.read()?;
            if message.get("id").is_none() {
                self.notifications.push(message);
            }
        }
    }

    fn open(&mut self, path: &Path, text: &str) -> Result<(String, Vec<Value>), String> {
        std::fs::write(path, text).map_err(|error| error.to_string())?;
        let uri = url::Url::from_file_path(path).expect("absolute document path").to_string();
        self.notify(
            "textDocument/didOpen",
            json!({ "textDocument": { "uri": uri, "languageId": "markdown", "version": 1, "text": text } }),
        );
        let diagnostics = self.diagnostics(&uri)?;
        Ok((uri, diagnostics))
    }

    fn completion_labels(&mut self, uri: &str, text: &str, after: &str) -> Result<Vec<String>, String> {
        let offset = text.find(after).expect("completion anchor") + after.len();
        let before = &text[..offset];
        let line = before.matches('\n').count();
        let character = offset - before.rfind('\n').map_or(0, |index| index + 1);
        let response = self.request(
            "textDocument/completion",
            json!({ "textDocument": { "uri": uri }, "position": { "line": line, "character": character } }),
        )?;
        let result = &response["result"];
        let items = result.as_array().or_else(|| result["items"].as_array()).cloned().unwrap_or_default();
        Ok(items.iter().filter_map(|item| item["label"].as_str().map(str::to_string)).collect())
    }
}

/// One opened rejection document: its row, the `(line, character, instance
/// path)` the row expects of each diagnostic, and what the server published.
struct Rejection {
    name: &'static str,
    expected: Vec<(u64, u64, String)>,
    diagnostics: Vec<Value>,
}

/// What the conversation observed, asserted on the test thread.
struct Observed {
    rejections: Vec<Rejection>,
    example: Vec<Value>,
    item_labels: Vec<String>,
    rule_labels: Vec<String>,
    action_labels: Vec<String>,
}

fn converse(client: &mut Client, root: &Path) -> Result<Observed, String> {
    let root_uri = url::Url::from_directory_path(root).expect("absolute root").to_string();
    let initialize = client.request(
        "initialize",
        json!({
            "processId": null,
            "capabilities": { "general": { "positionEncodings": ["utf-16"] } },
            "workspaceFolders": [{ "uri": root_uri, "name": "standalone" }]
        }),
    )?;
    if !initialize["error"].is_null() {
        return Err(format!("initialize failed: {initialize}"));
    }
    client.notify("initialized", json!({}));

    let mut rejections = Vec::new();
    for name in ACCEPTANCE_REJECTIONS {
        let rendered = render(declaration(name).declaration).remove(0);
        let mut expected: Vec<(u64, u64, String)> = declaration(name)
            .problems
            .iter()
            .map(|problem| {
                let (line, column) = rendered.start(problem.at);
                (u64::from(line) - 1, u64::from(column) - 1, problem.path.to_string())
            })
            .collect();
        expected.sort();
        let (_, diagnostics) = client.open(&root.join(format!("{name}.md")), &rendered.document())?;
        rejections.push(Rejection {
            name,
            expected,
            diagnostics,
        });
    }
    let (_, example) =
        client.open(&root.join("example.md"), &format!("---\n{MIXED_POLICY_FRONTMATTER}---\n\nBody\n"))?;
    let (uri, _) = client.open(&root.join("complete.md"), COMPLETION_DOCUMENT)?;
    let item_labels = client.completion_labels(&uri, COMPLETION_DOCUMENT, "ValidFor(3mo)\n  - ")?;
    let rule_labels = client.completion_labels(&uri, COMPLETION_DOCUMENT, "  - rule: ")?;
    let action_labels = client.completion_labels(&uri, COMPLETION_DOCUMENT, "TimeSensitive\n    action: ")?;

    client.request("shutdown", Value::Null)?;
    client.notify("exit", Value::Null);
    Ok(Observed {
        rejections,
        example,
        item_labels,
        rule_labels,
        action_labels,
    })
}

/// Spawns the built server in `cwd` with every home-like location pointed
/// into `fixture` and every schema or Darkmatter variable removed.
fn spawn_server(fixture: &Path, cwd: &Path) -> ChildGuard {
    let home = fixture.join("home");
    let mut command = Command::new(bin_exe!("dmls"));
    command
        .arg("--stdio")
        .current_dir(cwd)
        .env("HOME", &home)
        .env("USERPROFILE", &home)
        .env("APPDATA", &home)
        .env("LOCALAPPDATA", &home)
        .env("XDG_CONFIG_HOME", fixture.join("config"))
        .env("XDG_CACHE_HOME", fixture.join("cache"))
        .env("XDG_DATA_HOME", fixture.join("data"))
        .env_remove("HOMEDRIVE")
        .env_remove("HOMEPATH");
    for (key, _) in std::env::vars_os() {
        let name = key.to_string_lossy();
        if name.starts_with("DARKMATTER_") || name.starts_with("DM_") || name.starts_with("DMLS_") {
            command.env_remove(&key);
        }
    }
    for variable in SCHEMA_VARIABLES {
        command.env_remove(variable);
    }
    let child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn the compiled dmls binary");
    ChildGuard::new(child)
}

#[test]
fn the_built_server_validates_and_completes_policies_without_schema_files() {
    let _ = include_str!("../../../lib/tests/content_policy_fixtures/mod.rs");
    let fixture = tempfile::tempdir().expect("fixture root");
    let fixture_root = biscuit_file::canonicalize_simplified(fixture.path()).expect("canonical fixture root");
    for name in ["cwd", "home", "config", "cache", "data"] {
        std::fs::create_dir(fixture_root.join(name)).unwrap();
    }
    let cwd: PathBuf = fixture_root.join("cwd");
    if let Some(checkout) = biscuit_test_harness::manifest_dir!()
        .ancestors()
        .find(|ancestor| ancestor.join(".git").exists())
    {
        let checkout = biscuit_file::canonicalize_simplified(checkout).expect("canonical checkout");
        assert!(!cwd.starts_with(&checkout), "{} is inside {}", cwd.display(), checkout.display());
    }

    let mut child = spawn_server(&fixture_root, &cwd);
    let mut client = Client {
        stdin: child.child_mut().stdin.take().expect("child stdin"),
        reader: BufReader::new(child.child_mut().stdout.take().expect("child stdout")),
        notifications: Vec::new(),
        next_id: 0,
    };
    let (tx, rx) = mpsc::channel();
    let root = cwd.clone();
    let worker = thread::spawn(move || {
        let _ = tx.send(converse(&mut client, &root));
    });
    let outcome = rx.recv_timeout(Duration::from_secs(20));
    if outcome.is_err() {
        child.terminate();
    }
    let _ = worker.join();
    let status = child
        .wait_timeout(Duration::from_secs(10))
        .expect("wait for dmls")
        .unwrap_or_else(|| {
            child.terminate();
            panic!("dmls did not exit within 10 seconds after the exit notification")
        });
    let observed = outcome
        .expect("the conversation finished before the timeout")
        .unwrap_or_else(|error| panic!("the conversation failed: {error}"));
    assert!(status.success(), "dmls exited unsuccessfully: {status}");

    for Rejection {
        name,
        expected,
        diagnostics,
    } in &observed.rejections
    {
        let mut published: Vec<(u64, u64, String)> = diagnostics
            .iter()
            .map(|diagnostic| {
                let start = &diagnostic["range"]["start"];
                let path = diagnostic["data"]["instance_path"].as_str().unwrap_or_default();
                (start["line"].as_u64().unwrap(), start["character"].as_u64().unwrap(), path.to_string())
            })
            .collect();
        published.sort();
        assert_eq!(&published, expected, "{name}: {diagnostics:#?}");
        assert!(
            diagnostics.iter().all(|diagnostic| diagnostic["source"] == "darkmatter.schema"),
            "{name}: {diagnostics:#?}"
        );
    }
    assert!(observed.example.is_empty(), "{:#?}", observed.example);
    // A blank compact item offers every shipped rule suggestion plus the
    // long-form keys; a blank long-form `rule:` value, the suggestions alone.
    let mut item_labels = observed.item_labels.clone();
    item_labels.sort();
    let mut expected_item: Vec<String> =
        RULE_SUGGESTIONS.iter().chain(&["rule", "action"]).map(|label| label.to_string()).collect();
    expected_item.sort();
    assert_eq!(item_labels, expected_item, "{:?}", observed.item_labels);
    assert_eq!(observed.rule_labels, RULE_SUGGESTIONS, "{:?}", observed.rule_labels);
    assert_eq!(observed.action_labels, ["refresh", "archive", "remove"]);

    let mut entries: Vec<_> = std::fs::read_dir(&cwd)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    entries.sort();
    let mut documents: Vec<String> = ACCEPTANCE_REJECTIONS.iter().map(|name| format!("{name}.md")).collect();
    documents.extend(["complete.md".to_string(), "example.md".to_string()]);
    documents.sort();
    assert_eq!(entries, documents, "the server wrote nothing beside the test documents");
}
