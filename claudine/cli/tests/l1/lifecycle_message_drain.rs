//! Outbound lifecycle messages must finish, or be reported unfinished, before
//! an ordinary process exit (`2026-09-25-lifecycle-message-exit-race`).
//!
//! Every test here posts to a [`WebhookListener`] on loopback through an
//! environment-backed `discord_webhook` route, so the production route
//! validation and the real messenger provider stay in the path.

use crate::common;

use common::CliProcessFixture;
use common::webhook_listener::{
    DUMMY_TOKEN, ListenerMode, ROUTE_NAME, RecordedRequest, WebhookListener, write_webhook_route,
};
#[cfg(unix)]
use common::write_executable;
use common::{strip_ansi, write};
use std::io::{Read, Write};
use std::path::Path;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// Ceiling on any one child run; a hung child is killed and the test fails.
pub(crate) const CHILD_WATCHDOG: Duration = Duration::from_secs(60);

/// How long a withheld reply is held while the test checks the child is
/// still waiting for it.
pub(crate) const WITHHOLD_WINDOW: Duration = Duration::from_millis(1500);

/// A `claude` stub that prints one successful result line and exits `0`.
pub(crate) fn write_one_line_claude(bin_dir: &Path) {
    #[cfg(unix)]
    write_executable(
        &bin_dir.join("claude"),
        r#"#!/bin/sh
cat > /dev/null 2>/dev/null
printf '%s\n' '{"type":"result","subtype":"success","result":"done","session_id":"session-1","is_error":false}'
exit 0
"#,
    );

    #[cfg(windows)]
    write(
        &bin_dir.join("claude.cmd"),
        "@echo off\r\n\
echo {\"type\":\"result\",\"subtype\":\"success\",\"result\":\"done\",\"session_id\":\"session-1\",\"is_error\":false}\r\n\
exit /b 0\r\n",
    );
}

/// A `claude` stub like [`write_one_line_claude`] that first stays alive for
/// about three seconds, long enough for a `start` message to finish.
fn write_slow_one_line_claude(bin_dir: &Path) {
    #[cfg(unix)]
    write_executable(
        &bin_dir.join("claude"),
        r#"#!/bin/sh
cat > /dev/null 2>/dev/null
/bin/sleep 3
printf '%s\n' '{"type":"result","subtype":"success","result":"done","session_id":"session-1","is_error":false}'
exit 0
"#,
    );

    // `ping -n 4` is the console-safe ~3 s sleep; `timeout /t` needs an
    // interactive console and fails under a piped stdin.
    #[cfg(windows)]
    write(
        &bin_dir.join("claude.cmd"),
        "@echo off\r\n\
ping -n 4 127.0.0.1 >nul\r\n\
echo {\"type\":\"result\",\"subtype\":\"success\",\"result\":\"done\",\"session_id\":\"session-1\",\"is_error\":false}\r\n\
exit /b 0\r\n",
    );
}

/// Collect a child pipe on a thread so a full pipe cannot stall the child.
fn collect<R: Read + Send + 'static>(pipe: Option<R>) -> JoinHandle<String> {
    std::thread::spawn(move || {
        let mut text = String::new();
        if let Some(mut pipe) = pipe {
            let _ = pipe.read_to_string(&mut text);
        }
        text
    })
}

/// Poll `child` until it exits or `within` elapses.
fn wait_for_exit(child: &mut Child, within: Duration) -> Option<ExitStatus> {
    let deadline = Instant::now() + within;
    loop {
        if let Some(status) = child.try_wait().expect("poll child") {
            return Some(status);
        }
        if Instant::now() >= deadline {
            return None;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// Kill `child` and collect its output for a failure message.
fn kill_and_collect(child: &mut Child, stdout: JoinHandle<String>, stderr: JoinHandle<String>) -> String {
    let _ = child.kill();
    let _ = child.wait();
    format!(
        "stdout:\n{}\nstderr:\n{}",
        stdout.join().unwrap_or_default(),
        stderr.join().unwrap_or_default()
    )
}

/// R6 reproduction: a `success` message must reach its route before
/// `claudine compose` exits.
///
/// The listener withholds its reply, so a CLI that waits for the delivery is
/// still running while the reply is held. Before the fix the child exits while
/// the reply is withheld, or before any request arrives; both outcomes fail
/// here. The test does not assert that the old code sent nothing, because a
/// killed task may already have written its request.
#[test]
fn compose_success_message_is_delivered_before_exit() {
    let fixture = CliProcessFixture::named("lifecycle-message-drain");
    write_webhook_route(fixture.home());
    write_one_line_claude(fixture.bin_dir());
    let prompt = fixture.cwd().join("drain.md");
    write(
        &prompt,
        "---\nsuccess:\n  message: \"drain-success-marker\"\n---\nSay hello.\n",
    );

    let listener = WebhookListener::start(ListenerMode::WithholdUntilReleased);
    let mut command = fixture.command_std();
    listener.apply_route_env(&mut command);
    let mut child = command
        .args(["compose", "--claude", prompt.to_str().expect("UTF-8 prompt path")])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn claudine compose");
    let stdout = collect(child.stdout.take());
    let stderr = collect(child.stderr.take());

    // Watch for the request and the child's exit together, so a child that
    // exits before sending fails at once instead of at the watchdog.
    let deadline = Instant::now() + CHILD_WATCHDOG;
    loop {
        if !listener.requests().is_empty() {
            break;
        }
        if let Some(status) = child.try_wait().expect("poll child") {
            let output = kill_and_collect(&mut child, stdout, stderr);
            panic!(
                "claudine exited ({status}) before the success message reached the listener\n{output}"
            );
        }
        if Instant::now() >= deadline {
            let output = kill_and_collect(&mut child, stdout, stderr);
            panic!("no request reached the listener within {CHILD_WATCHDOG:?}\n{output}");
        }
        std::thread::sleep(Duration::from_millis(20));
    }

    if let Some(status) = wait_for_exit(&mut child, WITHHOLD_WINDOW) {
        let output = kill_and_collect(&mut child, stdout, stderr);
        panic!(
            "claudine exited ({status}) while the delivery's reply was still withheld\n{output}"
        );
    }

    listener.release();
    let Some(status) = wait_for_exit(&mut child, CHILD_WATCHDOG) else {
        let output = kill_and_collect(&mut child, stdout, stderr);
        panic!("claudine did not exit after the reply was released\n{output}");
    };
    let stdout = stdout.join().unwrap_or_default();
    let stderr = stderr.join().unwrap_or_default();
    assert_eq!(
        status.code(),
        Some(0),
        "a delivered message leaves the exit code alone\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );

    let requests = listener.requests();
    assert_eq!(
        requests.len(),
        1,
        "exactly one POST per lifecycle message: {requests:?}"
    );
    assert_eq!(requests[0].method, "POST");
    assert!(
        requests[0].path.starts_with("/webhooks/1/"),
        "the post targets the configured webhook: {}",
        requests[0].path
    );
    assert!(
        requests[0].body.contains("drain-success-marker"),
        "the post carries the rendered success text: {}",
        requests[0].body
    );
}

/// Control: the route, the environment variable, and the listener are wired
/// correctly. A `start` message has the whole agent run to finish, so it is
/// delivered with or without the exit drain; if this fails, a failure of the
/// reproduction above says nothing about the exit race.
#[test]
fn compose_start_message_reaches_the_listener_during_the_run() {
    let fixture = CliProcessFixture::named("lifecycle-message-start");
    write_webhook_route(fixture.home());
    write_slow_one_line_claude(fixture.bin_dir());
    let prompt = fixture.cwd().join("start.md");
    write(
        &prompt,
        "---\nstart:\n  message: \"drain-start-marker\"\n---\nSay hello.\n",
    );

    let listener = WebhookListener::start(ListenerMode::Reply(200));
    let mut command = fixture.command_std();
    listener.apply_route_env(&mut command);
    let mut child = command
        .args(["compose", "--claude", prompt.to_str().expect("UTF-8 prompt path")])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn claudine compose");
    let stdout = collect(child.stdout.take());
    let stderr = collect(child.stderr.take());

    let Some(status) = wait_for_exit(&mut child, CHILD_WATCHDOG) else {
        let output = kill_and_collect(&mut child, stdout, stderr);
        panic!("claudine did not exit within {CHILD_WATCHDOG:?}\n{output}");
    };
    let stderr = stderr.join().unwrap_or_default();
    let _ = stdout.join();
    assert_eq!(status.code(), Some(0), "stderr:\n{stderr}");

    let requests = listener.requests();
    assert_eq!(requests.len(), 1, "one POST for the start message: {requests:?}");
    assert!(
        requests[0].body.contains("drain-start-marker"),
        "the post carries the rendered start text: {}",
        requests[0].body
    );
}

/// A spawned `claudine` whose output is collected while it runs.
///
/// Stderr is appended to a shared buffer as it arrives, so a test can wait
/// for a line while the child is still alive. Every wait is bounded, and
/// dropping the run kills a child that is still going, so a failed assertion
/// never leaves a stalled `claudine` behind.
pub(crate) struct PipedRun {
    child: Child,
    stdout: Arc<Mutex<String>>,
    stderr: Arc<Mutex<String>>,
    readers: Vec<JoinHandle<()>>,
}

/// How a [`PipedRun`] ended.
pub(crate) struct Finished {
    pub status: ExitStatus,
    /// When the exit was observed; at most one poll interval late.
    pub exited_at: Instant,
    /// ANSI-stripped stderr.
    pub stderr: String,
}

impl PipedRun {
    /// Spawn `command` with both output streams piped. With `stdin`, the text
    /// is written and the pipe closed; without it, stdin is null.
    pub(crate) fn spawn(mut command: Command, stdin: Option<&str>) -> Self {
        command
            .stdin(if stdin.is_some() { Stdio::piped() } else { Stdio::null() })
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = command.spawn().expect("spawn claudine");
        if let Some(text) = stdin {
            let mut pipe = child.stdin.take().expect("child stdin");
            pipe.write_all(text.as_bytes()).expect("write child stdin");
        }
        let stdout = Arc::new(Mutex::new(String::new()));
        let stderr = Arc::new(Mutex::new(String::new()));
        let readers = vec![
            stream_into(child.stdout.take(), Arc::clone(&stdout)),
            stream_into(child.stderr.take(), Arc::clone(&stderr)),
        ];
        Self {
            child,
            stdout,
            stderr,
            readers,
        }
    }

    /// ANSI-stripped stderr written so far.
    pub(crate) fn stderr(&self) -> String {
        strip_ansi(&self.stderr.lock().unwrap())
    }

    /// Kill the child and panic with `message` and everything it wrote.
    pub(crate) fn fail(mut self, message: &str) -> ! {
        let _ = self.child.kill();
        let _ = self.child.wait();
        self.join_readers();
        panic!(
            "{message}\nstdout:\n{}\nstderr:\n{}",
            self.stdout.lock().unwrap(),
            self.stderr()
        );
    }

    /// Wait until `listener` holds `count` requests, failing at once if the
    /// child exits first.
    pub(crate) fn await_requests(
        self,
        listener: &WebhookListener,
        count: usize,
    ) -> (Self, Vec<RecordedRequest>) {
        let mut run = self;
        let deadline = Instant::now() + CHILD_WATCHDOG;
        loop {
            let requests = listener.requests();
            if requests.len() >= count {
                return (run, requests);
            }
            if let Some(status) = run.child.try_wait().expect("poll child") {
                run.fail(&format!(
                    "claudine exited ({status}) with {} of {count} requests delivered",
                    requests.len()
                ));
            }
            if Instant::now() >= deadline {
                run.fail(&format!("fewer than {count} requests within {CHILD_WATCHDOG:?}"));
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    /// Wait until stderr contains `needle`, failing if the child exits first.
    pub(crate) fn await_stderr(self, needle: &str) -> Self {
        let mut run = self;
        let deadline = Instant::now() + CHILD_WATCHDOG;
        loop {
            if run.stderr().contains(needle) {
                return run;
            }
            if let Some(status) = run.child.try_wait().expect("poll child") {
                run.fail(&format!("claudine exited ({status}) before writing {needle:?}"));
            }
            if Instant::now() >= deadline {
                run.fail(&format!("stderr lacked {needle:?} after {CHILD_WATCHDOG:?}"));
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    /// Fail if the child exits within `window`.
    pub(crate) fn assert_running_for(self, window: Duration, why: &str) -> Self {
        let mut run = self;
        if let Some(status) = wait_for_exit(&mut run.child, window) {
            run.fail(&format!("claudine exited ({status}) {why}"));
        }
        run
    }

    /// Wait for the child to exit within `within`, then collect its output.
    pub(crate) fn finish(self, within: Duration) -> Finished {
        let mut run = self;
        let Some(status) = wait_for_exit(&mut run.child, within) else {
            run.fail(&format!("claudine did not exit within {within:?}"));
        };
        let exited_at = Instant::now();
        run.join_readers();
        Finished {
            status,
            exited_at,
            stderr: run.stderr(),
        }
    }

    fn join_readers(&mut self) {
        for reader in self.readers.drain(..) {
            let _ = reader.join();
        }
    }
}

impl Drop for PipedRun {
    fn drop(&mut self) {
        if matches!(self.child.try_wait(), Ok(None)) {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

/// Append everything read from `pipe` to `sink` until the pipe closes.
fn stream_into<R: Read + Send + 'static>(pipe: Option<R>, sink: Arc<Mutex<String>>) -> JoinHandle<()> {
    std::thread::spawn(move || {
        let Some(mut pipe) = pipe else { return };
        let mut chunk = [0_u8; 4096];
        while let Ok(read) = pipe.read(&mut chunk) {
            if read == 0 {
                break;
            }
            sink.lock()
                .unwrap()
                .push_str(&String::from_utf8_lossy(&chunk[..read]));
        }
    })
}

/// A `compose --claude` run of `document` against the one-line stub, with
/// the route pointed at `listener`.
fn spawn_compose(
    fixture: &CliProcessFixture,
    listener: &WebhookListener,
    document: &str,
) -> PipedRun {
    write_webhook_route(fixture.home());
    write_one_line_claude(fixture.bin_dir());
    let prompt = fixture.cwd().join("drain.md");
    write(&prompt, document);
    let mut command = fixture.command_std();
    listener.apply_route_env(&mut command);
    command.args(["compose", "--claude", prompt.to_str().expect("UTF-8 prompt path")]);
    PipedRun::spawn(command, None)
}

/// R5: a rejected send is reported from inside the tracked task, before the
/// exit, and a failed delivery never changes the exit code.
#[test]
fn a_rejected_success_message_is_reported_and_the_exit_code_stays_zero() {
    let fixture = CliProcessFixture::named("lifecycle-message-rejected");
    let listener = WebhookListener::start(ListenerMode::Reply(400));
    let run = spawn_compose(
        &fixture,
        &listener,
        "---\nsuccess:\n  message: \"drain-rejected-marker\"\n---\nSay hello.\n",
    );

    let finished = run.finish(CHILD_WATCHDOG);
    let stderr = &finished.stderr;
    assert_eq!(finished.status.code(), Some(0), "stderr:\n{stderr}");
    assert!(
        stderr.contains("Failed to send lifecycle message"),
        "the rejected send is reported before exit:\n{stderr}"
    );
    assert!(
        stderr.contains(ROUTE_NAME),
        "the report names the route:\n{stderr}"
    );
    assert!(
        !stderr.contains(DUMMY_TOKEN),
        "the webhook token never reaches stderr:\n{stderr}"
    );
    assert!(
        !stderr.contains("still sending at exit"),
        "a finished (failed) send is not also reported as pending:\n{stderr}"
    );
    let requests = listener.requests();
    assert_eq!(requests.len(), 1, "one POST, not retried: {requests:?}");
    assert!(requests[0].body.contains("drain-rejected-marker"));
}

/// R3: two deliveries that never finish share one 10-second drain budget,
/// are named in one warning that says delivery is unknown, and leave the
/// exit code alone.
///
/// This waits out the real budget; there is deliberately no production
/// override to shorten it. It is not named `slow_`, because claudine's L1
/// filter excludes `slow_` tests from every recipe (only darkmatter declares
/// `l1-include-slow`), so the marker would leave it running nowhere.
#[test]
fn stalled_deliveries_share_one_drain_budget_and_are_reported_as_unknown() {
    let fixture = CliProcessFixture::named("lifecycle-message-stalled");
    let listener = WebhookListener::start(ListenerMode::NeverReply);
    let run = spawn_compose(
        &fixture,
        &listener,
        "---\nsuccess:\n  message: \"drain-stalled-success-body\"\n\
         finalize:\n  message: \"drain-stalled-finalize-body\"\n---\nSay hello.\n",
    );

    let (run, requests) = run.await_requests(&listener, 2);
    let both_sent = Instant::now();
    assert!(
        requests.iter().any(|request| request.body.contains("drain-stalled-success-body"))
            && requests.iter().any(|request| request.body.contains("drain-stalled-finalize-body")),
        "both terminal messages were sent: {requests:?}"
    );

    let finished = run.finish(CHILD_WATCHDOG);
    let waited = finished.exited_at.duration_since(both_sent);
    let stderr = &finished.stderr;
    assert_eq!(finished.status.code(), Some(0), "stderr:\n{stderr}");
    // The drain starts just before the second request lands, so it ends a
    // little under 10 s after it. One budget per delivery would take 20 s.
    assert!(
        waited >= Duration::from_secs(8),
        "the drain waited for the stalled deliveries ({waited:?})\n{stderr}"
    );
    assert!(
        waited < Duration::from_secs(15),
        "both deliveries shared one 10 s budget ({waited:?})\n{stderr}"
    );

    let warning = pending_warning(stderr)
        .unwrap_or_else(|| panic!("no pending-delivery warning:\n{stderr}"));
    assert!(
        warning.contains(&format!("Route {ROUTE_NAME}, route {ROUTE_NAME} were")),
        "one warning names each pending delivery: {warning}"
    );
    for secret in [DUMMY_TOKEN, "drain-stalled-success-body", "drain-stalled-finalize-body"] {
        assert!(!warning.contains(secret), "the warning leaks {secret:?}: {warning}");
    }
    assert!(!stderr.contains(DUMMY_TOKEN), "the token never reaches stderr:\n{stderr}");
    assert!(
        !stderr.contains("Failed to send"),
        "an unfinished delivery is not claimed to have failed:\n{stderr}"
    );
}

/// The pending-delivery warning in `stderr` with its word wrap undone, from
/// its first label through "delivery is unknown".
pub(crate) fn pending_warning(stderr: &str) -> Option<String> {
    const TAIL: &str = "still sending at exit; delivery is unknown";
    let joined = stderr.split_whitespace().collect::<Vec<_>>().join(" ");
    let end = joined.find(TAIL)? + TAIL.len();
    // The first label is the only capitalized one.
    let start = ["Route ", "Desktop notification"]
        .into_iter()
        .filter_map(|label| joined[..end].rfind(label))
        .max()?;
    Some(joined[start..end].to_string())
}

/// R6: the `success` message of a sequence's last step reaches its route
/// before the process exits.
///
/// Each step fires `success`, so the listener sees one post per step; the
/// replies are withheld until both have arrived.
#[test]
fn sequence_last_step_success_message_is_delivered_before_exit() {
    let fixture = CliProcessFixture::named("lifecycle-message-sequence");
    write_webhook_route(fixture.home());
    write_one_line_claude(fixture.bin_dir());
    let document = fixture.cwd().join("steps.md");
    write(
        &document,
        "---\nsequence:\n  - alpha\n  - beta\nsuccess:\n  message: \"seq-success-{{ state.name }}\"\n---\nRun step {{ state.name }}\n",
    );

    let listener = WebhookListener::start(ListenerMode::WithholdUntilReleased);
    let mut command = fixture.command_std();
    listener.apply_route_env(&mut command);
    command.args(["sequence", "--claude", document.to_str().expect("UTF-8 path")]);
    let run = PipedRun::spawn(command, None);

    let (run, _) = run.await_requests(&listener, 2);
    let run = run.assert_running_for(WITHHOLD_WINDOW, "while the last step's reply was still withheld");
    listener.release();
    let finished = run.finish(CHILD_WATCHDOG);
    assert_eq!(finished.status.code(), Some(0), "stderr:\n{}", finished.stderr);

    let bodies: Vec<_> = listener.requests().into_iter().map(|request| request.body).collect();
    assert_eq!(bodies.len(), 2, "one post per step: {bodies:?}");
    assert!(
        bodies.iter().any(|body| body.contains("seq-success-beta")),
        "the last step's message was delivered: {bodies:?}"
    );
}

/// R6: a terminal `notify` is tracked and drained, a failed notification
/// keeps its existing warning, and the exit code is unchanged.
///
/// The run is kept silent by making every desktop backend fail fast rather
/// than by any override: Windows has no AppUserModelID configured and fails
/// before touching the toast API; on Unix the `PATH` holds nothing but the
/// fixture stubs, so neither a notification helper nor `osascript` can be
/// found, and on Linux the D-Bus session address points nowhere. The
/// *unfinished* case (the `desktop notification` label in the pending
/// warning) needs a notification that stalls, which no silent host seam
/// provides; the library's `delivery::tests` pin it.
#[test]
fn a_terminal_notify_is_drained_and_its_failure_reported_without_host_ui() {
    let fixture = CliProcessFixture::named("lifecycle-notify-drain");
    write_one_line_claude(fixture.bin_dir());
    let prompt = fixture.cwd().join("notify.md");
    write(
        &prompt,
        "---\nsuccess:\n  notify: \"drain-notify-title\"\n---\nSay hello.\n",
    );

    // Unix: `fake_only_path` is the silencing seam described above; the stub
    // needs no tool from `PATH`.
    #[cfg(unix)]
    let mut command = fixture.command_builder().fake_only_path().build_std();
    #[cfg(windows)]
    let mut command = fixture.command_std();
    command
        .env("DBUS_SESSION_BUS_ADDRESS", "unix:path=/nonexistent/claudine-test-bus")
        .env_remove("XDG_RUNTIME_DIR")
        .args(["compose", "--claude", prompt.to_str().expect("UTF-8 prompt path")]);
    let run = PipedRun::spawn(command, None);

    let finished = run.finish(CHILD_WATCHDOG);
    let stderr = &finished.stderr;
    assert_eq!(finished.status.code(), Some(0), "stderr:\n{stderr}");
    assert!(
        stderr.contains("Failed to send desktop notification"),
        "the failed notification is reported before exit:\n{stderr}"
    );
    assert!(
        !stderr.contains("still sending at exit"),
        "a notification that failed fast is not pending:\n{stderr}"
    );
}

/// R2: an error that ends the run through `?` is rendered, and a message
/// sent earlier in the run is still drained before the exit.
///
/// The `success` stack calls an unknown function, which raises a lifecycle
/// evaluation error after `start` has already sent its message. The reply is
/// withheld, so the error block is on stderr while the child waits for it.
#[test]
fn a_top_level_error_after_a_send_still_drains_and_exits_one() {
    let fixture = CliProcessFixture::named("lifecycle-message-error");
    let listener = WebhookListener::start(ListenerMode::WithholdUntilReleased);
    let run = spawn_compose(
        &fixture,
        &listener,
        "---\nstart:\n  message: \"drain-error-start-marker\"\n\
         success:\n  stderr: \"{{ drain_test_unknown_fn() }}\"\n---\nSay hello.\n",
    );

    let (run, _) = run.await_requests(&listener, 1);
    let run = run.await_stderr("lifecycle evaluation error");
    let run = run.assert_running_for(
        WITHHOLD_WINDOW,
        "after rendering the error while the start message's reply was withheld",
    );
    listener.release();
    let finished = run.finish(CHILD_WATCHDOG);
    assert_eq!(
        finished.status.code(),
        Some(1),
        "the error's exit code is unchanged by the drain\nstderr:\n{}",
        finished.stderr
    );

    let requests = listener.requests();
    assert_eq!(requests.len(), 1, "{requests:?}");
    assert!(requests[0].body.contains("drain-error-start-marker"));
}

/// Self-tests for the loopback listener every test above relies on.
mod listener_fixture {
    use super::common::webhook_listener::{DUMMY_TOKEN, ListenerMode, WebhookListener};
    use std::io::{Read, Write};
    use std::net::TcpStream;
    use std::time::{Duration, Instant};

    fn post(listener: &WebhookListener, body: &str) -> TcpStream {
        let mut stream = TcpStream::connect(listener.addr()).expect("connect to listener");
        let request = format!(
            "POST /webhooks/1/{DUMMY_TOKEN}?wait=true HTTP/1.1\r\nHost: x\r\nContent-Length: {}\r\n\r\n{body}",
            body.len()
        );
        stream.write_all(request.as_bytes()).unwrap();
        stream
    }

    fn read_reply(stream: &mut TcpStream) -> String {
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let mut reply = String::new();
        let _ = stream.read_to_string(&mut reply);
        reply
    }

    #[test]
    fn reply_mode_records_the_request_and_answers_with_its_status() {
        let listener = WebhookListener::start(ListenerMode::Reply(400));
        let mut stream = post(&listener, r#"{"content":"hello"}"#);
        let request = listener
            .wait_for_request(Duration::from_secs(5))
            .expect("request recorded");
        assert_eq!(request.method, "POST");
        assert_eq!(request.path, format!("/webhooks/1/{DUMMY_TOKEN}?wait=true"));
        assert_eq!(request.body, r#"{"content":"hello"}"#);
        assert!(read_reply(&mut stream).starts_with("HTTP/1.1 400"));
    }

    #[test]
    fn withheld_connections_answer_only_after_release() {
        let listener = WebhookListener::start(ListenerMode::WithholdUntilReleased);
        let mut first = post(&listener, "a");
        let mut second = post(&listener, "b");
        listener
            .wait_for_requests(2, Duration::from_secs(5))
            .expect("both requests recorded while withheld");

        first
            .set_read_timeout(Some(Duration::from_millis(200)))
            .unwrap();
        let mut probe = [0_u8; 1];
        assert!(
            first.read(&mut probe).is_err(),
            "no reply may arrive before release"
        );

        listener.release();
        assert!(read_reply(&mut first).starts_with("HTTP/1.1 200"));
        assert!(read_reply(&mut second).starts_with("HTTP/1.1 200"));
    }

    #[test]
    fn never_reply_holds_the_connection_until_the_watchdog_fires() {
        let listener = WebhookListener::start_with_watchdog(
            ListenerMode::NeverReply,
            Duration::from_millis(500),
        );
        let mut stream = post(&listener, "a");
        listener
            .wait_for_request(Duration::from_secs(5))
            .expect("request recorded");
        let started = Instant::now();
        let reply = read_reply(&mut stream);
        assert!(reply.is_empty(), "a stalled listener never answers: {reply:?}");
        assert!(started.elapsed() < Duration::from_secs(5));
    }

    #[test]
    fn webhook_url_uses_the_dummy_token_on_loopback() {
        let listener = WebhookListener::start(ListenerMode::Reply(200));
        let url = listener.webhook_url();
        assert!(url.starts_with("http://127.0.0.1:"), "{url}");
        assert!(url.ends_with(&format!("/webhooks/1/{DUMMY_TOKEN}")), "{url}");
    }
}
