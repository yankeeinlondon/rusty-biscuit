//! Level-1 PTY tests for Unix OSC color queries.
//!
//! These tests exercise the live OSC color queries by spawning
//! `discovery_probe` inside a pseudoterminal, manufacturing the OSC reply
//! bytes (followed by a DA1 reply, as a terminal sends one for the library's
//! sentinel), and asserting on the parsed output.
//!
//! Run `cargo build -p biscuit-terminal --example discovery_probe` first.

use crate::common;

use std::time::Duration;

use common::pty::{DA1_QUERY, DA1_REPLY, ProbeAnswer, drive_probe, spawn_with_env};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Answer the probe's OSC query with `reply` (and its DA1 sentinel, as a
/// terminal would) once the query appears, and return the output collected
/// through the probe's `marker` line.
fn query_with_osc_reply(
    probe_mode: &str,
    query: &'static [u8],
    reply: &'static [u8],
    marker: &str,
) -> String {
    let mut session = spawn_with_env(&[("PROBE", probe_mode), ("PROBE_TERM_PROGRAM", "WezTerm")]);
    let mut answers = [
        ProbeAnswer::new(query, reply),
        ProbeAnswer::every(DA1_QUERY, DA1_REPLY),
    ];
    let collected = drive_probe(&mut session, &mut answers, marker, Duration::from_secs(5));
    String::from_utf8_lossy(&collected).into_owned()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[test]
fn bg_color_query_returns_some_with_manufactured_reply() {
    let output = query_with_osc_reply(
        "osc11",
        b"\x1b];r;?\x07",
        b"\x1b]11;rgb:8080/8080/8080\x07",
        "bg_color=",
    );
    assert!(
        output.contains("bg_color=Some("),
        "expected parsed bg_color in output, got: {output}"
    );
}

#[test]
fn text_color_query_returns_some_with_manufactured_reply() {
    let output = query_with_osc_reply(
        "osc10",
        b"\x1b];r;?\x07",
        b"\x1b]10;rgb:e5e5/e5e5/e5e5\x07",
        "text_color=",
    );
    assert!(
        output.contains("text_color=Some("),
        "expected parsed text_color in output, got: {output}"
    );
}

#[test]
fn cursor_color_query_returns_some_with_manufactured_reply() {
    let output = query_with_osc_reply(
        "osc12",
        b"\x1b];r;?\x07",
        b"\x1b]12;rgb:0000/ff00/0000\x07",
        "cursor_color=",
    );
    assert!(
        output.contains("cursor_color=Some("),
        "expected parsed cursor_color in output, got: {output}"
    );
}

#[test]
fn bg_color_with_timeout_returns_some_with_manufactured_reply() {
    let output = query_with_osc_reply(
        "osc11_timeout",
        b"\x1b];r;?\x07",
        b"\x1b]11;rgb:8080/8080/8080\x07",
        "osc11_timeout=",
    );
    assert!(
        output.contains("osc11_timeout=Some("),
        "expected parsed osc11_timeout in output, got: {output}"
    );
}

#[test]
fn text_color_with_timeout_returns_some_with_manufactured_reply() {
    let output = query_with_osc_reply(
        "osc10_timeout",
        b"\x1b];r;?\x07",
        b"\x1b]10;rgb:e5e5/e5e5/e5e5\x07",
        "osc10_timeout=",
    );
    assert!(
        output.contains("osc10_timeout=Some("),
        "expected parsed osc10_timeout in output, got: {output}"
    );
}

#[test]
fn cursor_color_with_timeout_returns_some_with_manufactured_reply() {
    let output = query_with_osc_reply(
        "osc12_timeout",
        b"\x1b];r;?\x07",
        b"\x1b]12;rgb:0000/ff00/0000\x07",
        "osc12_timeout=",
    );
    assert!(
        output.contains("osc12_timeout=Some("),
        "expected parsed osc12_timeout in output, got: {output}"
    );
}
