//! Level-1 PTY tests for Unix OSC color queries.
//!
//! These tests exercise `query_osc_actual` by spawning `discovery_probe`
//! inside a pseudoterminal, manufacturing the OSC reply bytes, and
//! asserting on the parsed output.
//!
//! Run `cargo build -p biscuit-terminal --example discovery_probe` first.

#![cfg(unix)]

mod common;

use std::io::Write;
use std::time::Duration;

use common::pty::spawn_with_env;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Send a manufactured OSC colour reply into the PTY and collect the
/// probe's stdout.
fn query_with_osc_reply(probe_mode: &str, reply: &str) -> String {
    let mut session = spawn_with_env(&[("PROBE", probe_mode), ("PROBE_TERM_PROGRAM", "WezTerm")]);

    // Wait for the probe to enter raw mode and emit the query.
    std::thread::sleep(Duration::from_millis(80));

    // Manufacture the terminal response.
    session
        .write_all(reply.as_bytes())
        .expect("failed to send OSC reply");
    session.flush().unwrap();

    // Give the probe time to parse and print.
    std::thread::sleep(Duration::from_millis(80));

    // Drain remaining output.
    let mut output = String::new();
    let mut scratch = [0u8; 4096];
    loop {
        match session.try_read(&mut scratch) {
            Ok(0) => break,
            Ok(n) => output.push_str(&String::from_utf8_lossy(&scratch[..n])),
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
            Err(e) if e.kind() == std::io::ErrorKind::TimedOut => break,
            Err(_) => break,
        }
    }
    output
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[test]
fn bg_color_query_returns_some_with_manufactured_reply() {
    let output = query_with_osc_reply("osc11", "\x1b]11;rgb:8080/8080/8080\x07");
    assert!(
        output.contains("bg_color=Some("),
        "expected parsed bg_color in output, got: {output}"
    );
}

#[test]
fn text_color_query_returns_some_with_manufactured_reply() {
    let output = query_with_osc_reply("osc10", "\x1b]10;rgb:e5e5/e5e5/e5e5\x07");
    assert!(
        output.contains("text_color=Some("),
        "expected parsed text_color in output, got: {output}"
    );
}

#[test]
fn cursor_color_query_returns_some_with_manufactured_reply() {
    let output = query_with_osc_reply("osc12", "\x1b]12;rgb:0000/ff00/0000\x07");
    assert!(
        output.contains("cursor_color=Some("),
        "expected parsed cursor_color in output, got: {output}"
    );
}

#[test]
fn bg_color_with_timeout_returns_some_with_manufactured_reply() {
    let output = query_with_osc_reply("osc11_timeout", "\x1b]11;rgb:8080/8080/8080\x07");
    assert!(
        output.contains("osc11_timeout=Some("),
        "expected parsed osc11_timeout in output, got: {output}"
    );
}

#[test]
fn text_color_with_timeout_returns_some_with_manufactured_reply() {
    let output = query_with_osc_reply("osc10_timeout", "\x1b]10;rgb:e5e5/e5e5/e5e5\x07");
    assert!(
        output.contains("osc10_timeout=Some("),
        "expected parsed osc10_timeout in output, got: {output}"
    );
}

#[test]
fn cursor_color_with_timeout_returns_some_with_manufactured_reply() {
    let output = query_with_osc_reply("osc12_timeout", "\x1b]12;rgb:0000/ff00/0000\x07");
    assert!(
        output.contains("osc12_timeout=Some("),
        "expected parsed osc12_timeout in output, got: {output}"
    );
}
