//! Outbound lifecycle messages must finish, or be reported unfinished, before
//! an ordinary process exit (`2026-09-25-lifecycle-message-exit-race`).
//!
//! Every test here posts to a [`WebhookListener`] on loopback through an
//! environment-backed `discord_webhook` route, so the production route
//! validation and the real messenger provider stay in the path.

use crate::common;

use common::CliProcessFixture;
use common::webhook_listener::{ListenerMode, WebhookListener, write_webhook_route};
use common::write;
#[cfg(unix)]
use common::write_executable;
use std::io::Read;
use std::path::Path;
use std::process::{Child, ExitStatus, Stdio};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// Ceiling on any one child run; a hung child is killed and the test fails.
const CHILD_WATCHDOG: Duration = Duration::from_secs(60);

/// How long a withheld reply is held while the test checks the child is
/// still waiting for it.
const WITHHOLD_WINDOW: Duration = Duration::from_millis(1500);

/// A `claude` stub that prints one successful result line and exits `0`.
fn write_one_line_claude(bin_dir: &Path) {
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
#[ignore = "red until the CLI drains deliveries before exit; the change that lands the drain removes this attribute"]
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
