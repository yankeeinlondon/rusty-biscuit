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

struct ChildGuard {
    child: Option<Child>,
}

impl ChildGuard {
    fn new(child: Child) -> Self {
        Self { child: Some(child) }
    }

    fn child_mut(&mut self) -> &mut Child {
        self.child.as_mut().expect("child already reaped")
    }

    fn wait_timeout(&mut self, timeout: Duration) -> std::io::Result<Option<ExitStatus>> {
        let status = self.child_mut().wait_timeout(timeout)?;
        if status.is_some() {
            self.child.take();
        }
        Ok(status)
    }

    fn terminate(&mut self) {
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
            .args(["--exact", "child_guard_cancellation_probe", "--nocapture"])
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
fn write_message(stdin: &mut impl Write, message: &Value) {
    let body = serde_json::to_vec(message).expect("serialize message");
    write!(stdin, "Content-Length: {}\r\n\r\n", body.len()).expect("write header");
    stdin.write_all(&body).expect("write body");
    stdin.flush().expect("flush stdin");
}

/// Reads one framed LSP message, or `None` on EOF (the child closed stdout).
fn read_message(reader: &mut impl BufRead) -> Option<Value> {
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
