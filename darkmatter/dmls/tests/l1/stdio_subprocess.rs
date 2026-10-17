//! Level-1 subprocess smoke test: launch the **compiled** `dmls` binary and
//! drive a full `initialize` → `initialized` → `shutdown` → `exit` LSP handshake
//! over real OS stdin/stdout pipes — the exact launch path a Zed/VS Code/Neovim
//! integration uses.
//!
//! The in-memory `Connection::memory()` sessions in `lsp_session.rs`
//! prove the request handlers; they never exercise the native binary's argv
//! parsing, `Connection::stdio()` framing, or process shutdown. This test proves
//! the binary starts, speaks LSP over real pipes, and exits cleanly — release
//! readiness the in-memory suite cannot observe.
//!
//! No terminal emulator is involved (only OS pipes), so it runs in the standard
//! `just test` gate. It needs the built binary (located via `CARGO_BIN_EXE_dmls`)
//! but no terminal harness. Every read is bounded by a hard timeout that kills
//! the child, so the test can never hang the suite.

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use biscuit_test_harness::bin_exe;
use serde_json::{Value, json};
use wait_timeout::ChildExt;

pub(crate) struct ChildGuard {
    child: Option<Child>,
}

impl ChildGuard {
    pub(crate) fn new(child: Child) -> Self {
        Self { child: Some(child) }
    }

    pub(crate) fn child_mut(&mut self) -> &mut Child {
        self.child.as_mut().expect("child already reaped")
    }

    pub(crate) fn wait_timeout(&mut self, timeout: Duration) -> std::io::Result<Option<ExitStatus>> {
        let status = self.child_mut().wait_timeout(timeout)?;
        if status.is_some() {
            self.child.take();
        }
        Ok(status)
    }

    pub(crate) fn terminate(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        self.terminate();
    }
}

const CANCELLATION_READY: &str = "DMLS_CANCELLATION_READY";
const CANCELLATION_FINISHED: &str = "DMLS_CANCELLATION_FINISHED";

#[test]
fn child_guard_cancellation_probe() {
    let (Ok(ready), Ok(finished)) = (
        std::env::var(CANCELLATION_READY),
        std::env::var(CANCELLATION_FINISHED),
    ) else {
        return;
    };
    std::fs::write(ready, b"ready").unwrap();
    thread::sleep(Duration::from_millis(250));
    std::fs::write(finished, b"finished").unwrap();
}

#[test]
fn child_guard_reaps_process_during_unwind() {
    let dir = tempfile::tempdir().unwrap();
    let ready = dir.path().join("ready");
    let finished = dir.path().join("finished");
    let result = std::panic::catch_unwind(|| {
        let child = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "stdio_subprocess::child_guard_cancellation_probe", "--nocapture"])
            .env(CANCELLATION_READY, &ready)
            .env(CANCELLATION_FINISHED, &finished)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn cancellation probe");
        let _guard = ChildGuard::new(child);
        let deadline = Instant::now() + Duration::from_secs(2);
        while !ready.exists() && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(5));
        }
        assert!(ready.exists(), "cancellation probe did not start");
        panic!("simulate a panicking assertion");
    });
    assert!(result.is_err(), "the cancellation path must unwind");

    // The child would write this marker after 250 ms if the guard detached it.
    // Waiting beyond that floor is the observable cancellation contract.
    thread::sleep(Duration::from_millis(350));
    assert!(
        !finished.exists(),
        "the child survived the parent assertion panic"
    );
}

/// Frames a JSON-RPC message with the LSP `Content-Length` header.
pub(crate) fn write_message(stdin: &mut impl Write, message: &Value) {
    let body = serde_json::to_vec(message).expect("serialize message");
    write!(stdin, "Content-Length: {}\r\n\r\n", body.len()).expect("write header");
    stdin.write_all(&body).expect("write body");
    stdin.flush().expect("flush stdin");
}

/// Reads one framed LSP message, or `None` on EOF (the child closed stdout).
pub(crate) fn read_message(reader: &mut impl BufRead) -> Option<Value> {
    let mut content_length: Option<usize> = None;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).ok()? == 0 {
            return None; // EOF before a complete header block.
        }
        let trimmed = line.trim_end_matches(['\r', '\n']);
        if trimmed.is_empty() {
            break; // Blank line terminates the header block.
        }
        if let Some(value) = trimmed.strip_prefix("Content-Length:") {
            content_length = value.trim().parse().ok();
        }
    }
    let length = content_length?;
    let mut body = vec![0u8; length];
    reader.read_exact(&mut body).ok()?;
    serde_json::from_slice(&body).ok()
}

/// Reads messages until the response with `id` arrives, skipping out-of-band
/// server notifications. `None` if the stream ends first.
fn await_response(reader: &mut impl BufRead, id: i64) -> Option<Value> {
    loop {
        let message = read_message(reader)?;
        if message["id"] == json!(id) {
            return Some(message);
        }
    }
}

#[test]
fn native_binary_speaks_lsp_over_stdio() {
    let fixture = tempfile::tempdir().expect("create isolated dmls process root");
    for name in ["home", "cache", "config", "data"] {
        std::fs::create_dir(fixture.path().join(name)).unwrap();
    }
    let child = Command::new(bin_exe!("dmls"))
        // `--stdio` is a no-op the binary accepts for editor compatibility.
        .arg("--stdio")
        .current_dir(fixture.path())
        .env("HOME", fixture.path().join("home"))
        .env("USERPROFILE", fixture.path().join("home"))
        .env("XDG_CACHE_HOME", fixture.path().join("cache"))
        .env("XDG_CONFIG_HOME", fixture.path().join("config"))
        .env("XDG_DATA_HOME", fixture.path().join("data"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        // Discard the log stream so it can never fill a pipe or trip the
        // nextest leak detector after the child exits.
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn the compiled dmls binary");
    let mut child = ChildGuard::new(child);

    let mut stdin = child.child_mut().stdin.take().expect("child stdin");
    let stdout = child.child_mut().stdout.take().expect("child stdout");

    // Run the whole conversation on a worker so the main thread can enforce a
    // hard timeout and kill the child if the server ever stalls.
    let (tx, rx) = mpsc::channel();
    let worker = thread::spawn(move || {
        let mut reader = BufReader::new(stdout);

        write_message(
            &mut stdin,
            &json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                // No workspace folder → no disk walk; a bare lifecycle probe.
                "params": { "processId": null, "rootUri": null, "capabilities": {} }
            }),
        );
        let Some(initialize) = await_response(&mut reader, 1) else {
            let _ = tx.send(Err(
                "stream ended before the initialize response".to_string()
            ));
            return;
        };

        write_message(
            &mut stdin,
            &json!({ "jsonrpc": "2.0", "method": "initialized", "params": {} }),
        );
        write_message(
            &mut stdin,
            &json!({ "jsonrpc": "2.0", "id": 2, "method": "shutdown", "params": null }),
        );
        if await_response(&mut reader, 2).is_none() {
            let _ = tx.send(Err("stream ended before the shutdown response".to_string()));
            return;
        }
        write_message(
            &mut stdin,
            &json!({ "jsonrpc": "2.0", "method": "exit", "params": null }),
        );

        let _ = tx.send(Ok(initialize));
        // `stdin` drops here, closing the child's input as an editor would.
    });

    let outcome = rx.recv_timeout(Duration::from_secs(20));
    if outcome.is_err() {
        child.terminate();
    }

    let _ = worker.join();

    let status = child
        .wait_timeout(Duration::from_secs(10))
        .expect("wait for dmls process")
        .unwrap_or_else(|| {
            child.terminate();
            panic!("dmls did not exit within 10 seconds after the exit notification")
        });
    assert!(status.success(), "dmls exited unsuccessfully: {status}");

    let initialize = outcome
        .expect("dmls answered the handshake before the timeout")
        .expect("handshake completed without a stream error");

    assert!(
        initialize["error"].is_null(),
        "initialize must not error: {initialize:?}"
    );
    let result = &initialize["result"];
    assert_eq!(
        result["serverInfo"]["name"],
        json!("dmls"),
        "unexpected serverInfo: {result:?}"
    );
    assert!(
        result["capabilities"]["textDocumentSync"].is_object(),
        "the native binary must advertise document sync: {result:?}"
    );
    assert!(
        result["capabilities"]["hoverProvider"].is_boolean()
            || result["capabilities"]["hoverProvider"].is_object(),
        "the native binary must advertise the Layer-0 surface: {result:?}"
    );
}

/// Malformed JSON in an editor settings notification never reaches the
/// configuration loader: the `lsp-server` transport rejects the payload and
/// stops reading the connection, so no later message can be applied under
/// rules a malformed message half-set. This pins that transport boundary
/// (plan ruling 7); the loader itself only ever sees a parsed
/// `serde_json::Value`.
#[test]
fn malformed_settings_json_is_rejected_by_the_transport() {
    let fixture = tempfile::tempdir().expect("create isolated dmls process root");
    for name in ["home", "cache", "config", "data"] {
        std::fs::create_dir(fixture.path().join(name)).unwrap();
    }
    let child = Command::new(bin_exe!("dmls"))
        .current_dir(fixture.path())
        .env("HOME", fixture.path().join("home"))
        .env("USERPROFILE", fixture.path().join("home"))
        .env("XDG_CACHE_HOME", fixture.path().join("cache"))
        .env("XDG_CONFIG_HOME", fixture.path().join("config"))
        .env("XDG_DATA_HOME", fixture.path().join("data"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn the compiled dmls binary");
    let mut child = ChildGuard::new(child);
    let mut stdin = child.child_mut().stdin.take().expect("child stdin");
    let stdout = child.child_mut().stdout.take().expect("child stdout");

    let (tx, rx) = mpsc::channel();
    let worker = thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        write_message(
            &mut stdin,
            &json!({
                "jsonrpc": "2.0", "id": 1, "method": "initialize",
                "params": { "processId": null, "rootUri": null, "capabilities": {} }
            }),
        );
        if await_response(&mut reader, 1).is_none() {
            let _ = tx.send(Err("no initialize response".to_string()));
            return;
        }
        write_message(&mut stdin, &json!({ "jsonrpc": "2.0", "method": "initialized", "params": {} }));
        // A trailing comma: not JSON, so not an LSP message.
        let body = r#"{"jsonrpc":"2.0","method":"workspace/didChangeConfiguration","params":{"settings":{"dmls":{"formatting":{"markdown":{"unwrap":true,}}}}}}"#;
        let _ = write!(stdin, "Content-Length: {}\r\n\r\n{body}", body.len());
        let _ = stdin.flush();
        // A valid request after it is never answered: the transport stopped
        // reading at the malformed payload.
        write_message(
            &mut stdin,
            &json!({ "jsonrpc": "2.0", "id": 2, "method": "shutdown", "params": null }),
        );
        let _ = tx.send(Ok(await_response(&mut reader, 2)));
    });

    let outcome = rx.recv_timeout(Duration::from_secs(20));
    let status = child.wait_timeout(Duration::from_secs(10)).expect("wait for dmls");
    child.terminate();
    let _ = worker.join();

    let after = outcome.expect("the conversation finished before the timeout").expect("initialized");
    assert!(after.is_none(), "a request after malformed JSON must not be answered: {after:?}");
    let status = status.expect("dmls exits once its transport rejects the payload");
    assert!(!status.success(), "a rejected transport is not a clean exit: {status}");
}

/// Splits `bytes` into LSP frames, failing on any byte outside a
/// `Content-Length` header and its JSON-RPC body.
fn strict_frames(bytes: &[u8]) -> Result<Vec<Value>, String> {
    const PREFIX: &[u8] = b"Content-Length: ";
    let mut frames = Vec::new();
    let mut rest = bytes;
    while !rest.is_empty() {
        let after_prefix = rest
            .strip_prefix(PREFIX)
            .ok_or_else(|| format!("non-frame bytes on stdout: {:?}", String::from_utf8_lossy(rest)))?;
        let header_end = after_prefix
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
            .ok_or("unterminated header")?;
        let length: usize = std::str::from_utf8(&after_prefix[..header_end])
            .ok()
            .and_then(|digits| digits.parse().ok())
            .ok_or("non-numeric Content-Length")?;
        let body = after_prefix
            .get(header_end + 4..header_end + 4 + length)
            .ok_or("truncated body")?;
        let value: Value = serde_json::from_slice(body).map_err(|error| error.to_string())?;
        if value["jsonrpc"] != json!("2.0") {
            return Err(format!("not a JSON-RPC message: {value}"));
        }
        frames.push(value);
        rest = &after_prefix[header_end + 4 + length..];
    }
    Ok(frames)
}

/// What a debug-logging session wrote to each stream.
struct LoggedSession {
    stdout: Vec<u8>,
    stderr: String,
}

const LOGGED_SECRET: &str = "logged-session-secret-51c7";

/// Runs `dmls --log-level debug <extra>` through open, hover, and shutdown,
/// writing the whole conversation up front and reading both streams to EOF.
fn run_logged_session(workspace: &std::path::Path, extra: &[&str]) -> LoggedSession {
    for name in ["home", "cache", "config", "data"] {
        std::fs::create_dir_all(workspace.join(name)).unwrap();
    }
    let doc = workspace.join("doc.md");
    let text = format!("---\ntitle: {LOGGED_SECRET}\n---\n\n# Heading\n\n{LOGGED_SECRET} body\n");
    std::fs::write(&doc, &text).unwrap();
    let root_uri = url::Url::from_directory_path(workspace).unwrap().to_string();
    let doc_uri = url::Url::from_file_path(&doc).unwrap().to_string();

    let child = Command::new(bin_exe!("dmls"))
        .args(["--log-level", "debug"])
        .args(extra)
        .current_dir(workspace)
        .env("HOME", workspace.join("home"))
        .env("USERPROFILE", workspace.join("home"))
        .env("XDG_CACHE_HOME", workspace.join("cache"))
        .env("XDG_CONFIG_HOME", workspace.join("config"))
        .env("XDG_DATA_HOME", workspace.join("data"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn the compiled dmls binary");
    let mut child = ChildGuard::new(child);
    let mut stdin = child.child_mut().stdin.take().expect("child stdin");
    let mut stdout = child.child_mut().stdout.take().expect("child stdout");
    let mut stderr = child.child_mut().stderr.take().expect("child stderr");
    let stdout_reader = thread::spawn(move || {
        let mut bytes = Vec::new();
        let _ = std::io::Read::read_to_end(&mut stdout, &mut bytes);
        bytes
    });
    let stderr_reader = thread::spawn(move || {
        let mut text = String::new();
        let _ = std::io::Read::read_to_string(&mut stderr, &mut text);
        text
    });

    for message in [
        json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize",
                "params": { "processId": null, "rootUri": root_uri, "capabilities": {} } }),
        json!({ "jsonrpc": "2.0", "method": "initialized", "params": {} }),
        json!({ "jsonrpc": "2.0", "method": "textDocument/didOpen", "params": { "textDocument": {
                "uri": doc_uri, "languageId": "markdown", "version": 1, "text": text } } }),
        json!({ "jsonrpc": "2.0", "id": 2, "method": "textDocument/hover", "params": {
                "textDocument": { "uri": doc_uri }, "position": { "line": 4, "character": 3 } } }),
        json!({ "jsonrpc": "2.0", "id": 3, "method": "shutdown", "params": null }),
        json!({ "jsonrpc": "2.0", "method": "exit", "params": null }),
    ] {
        write_message(&mut stdin, &message);
    }
    drop(stdin);

    let status = child
        .wait_timeout(Duration::from_secs(20))
        .expect("wait for dmls")
        .unwrap_or_else(|| panic!("dmls did not exit within 20 seconds"));
    assert!(status.success(), "dmls exited unsuccessfully: {status}");
    LoggedSession {
        stdout: stdout_reader.join().unwrap(),
        stderr: stderr_reader.join().unwrap(),
    }
}

/// The records DMLS and the Darkmatter library wrote, continuation lines
/// included. `lsp_server`'s own debug transport trace prints whole protocol
/// payloads and is outside this contract.
fn own_records(log: &str) -> String {
    let mut own = String::new();
    let mut keep = false;
    for line in log.lines() {
        let mut words = line.split_whitespace();
        let is_record_start = words.next().is_some_and(|stamp| stamp.ends_with('Z') && stamp.contains('T'));
        if is_record_start {
            let target = words.nth(1).unwrap_or_default();
            keep = target.starts_with("dmls") || target.starts_with("darkmatter");
        }
        if keep {
            own.push_str(line);
            own.push('\n');
        }
    }
    own
}

fn hover_completion_lines(log: &str) -> Vec<&str> {
    log.lines()
        .filter(|line| line.contains("request complete") || line.contains("slow request"))
        .filter(|line| line.contains("method=\"textDocument/hover\""))
        .collect()
}

#[test]
fn request_logs_go_to_stderr_and_never_into_the_stdout_protocol_stream() {
    let workspace = tempfile::tempdir().unwrap();
    let session = run_logged_session(workspace.path(), &[]);

    let frames = strict_frames(&session.stdout).unwrap();
    assert!(
        frames.iter().any(|frame| frame["id"] == json!(2) && frame.get("result").is_some()),
        "the hover was answered on stdout: {frames:#?}"
    );
    let stdout = String::from_utf8_lossy(&session.stdout);
    assert!(!stdout.contains("request complete") && !stdout.contains("dmls::"), "{stdout}");

    let lines = hover_completion_lines(&session.stderr);
    assert_eq!(lines.len(), 1, "{}", session.stderr);
    assert!(lines[0].contains("id=2"), "{}", lines[0]);
    let own = own_records(&session.stderr);
    assert!(own.contains("dmls::request_timing"), "{own}");
    assert!(!own.contains(LOGGED_SECRET), "source text reached the log: {own}");
}

#[test]
fn a_log_file_receives_request_logs_instead_of_stderr() {
    let workspace = tempfile::tempdir().unwrap();
    let log_file = workspace.path().join("dmls.log");
    let session = run_logged_session(workspace.path(), &["--log-file", log_file.to_str().unwrap()]);

    strict_frames(&session.stdout).unwrap();
    assert!(hover_completion_lines(&session.stderr).is_empty(), "{}", session.stderr);
    let log = std::fs::read_to_string(&log_file).unwrap();
    assert_eq!(hover_completion_lines(&log).len(), 1, "{log}");
    let own = own_records(&log);
    assert!(own.contains("dmls::request_timing"), "{own}");
    assert!(!own.contains(LOGGED_SECRET), "source text reached the log: {own}");
}
