//! Level-1 PTY tests for cursor position queries.
//!
//! Verifies that `cursor_position` emits `CSI 6n` and correctly parses
//! the CPR (`CSI row;col R`) response.
//!
//! The production query uses `/dev/tty` and is Unix-only.
//!
//! Run `cargo build -p biscuit-terminal --example discovery_probe` first.

#![cfg(unix)]

mod common;

use std::time::Duration;

use common::pty::{ProbeAnswer, drive_probe, spawn_with_env};
use serial_test::serial;

/// The DSR cursor-position query, `CSI 6 n`, as it appears in the master
/// stream once the probe has entered raw mode and written it to `/dev/tty`.
const DSR_QUERY: &[u8] = b"\x1b[6n";

/// Upper bound on the whole spawn/query/reply/print cycle. Generous on
/// purpose: it exists to fail a wedged probe, not to police timing.
const PROBE_DEADLINE: Duration = Duration::from_secs(5);

/// Spawn the cursor probe, answer its DSR query with `reply` once the query
/// appears in the master stream, and return the collected output after the
/// probe has printed `marker`.
///
/// Replying on observation rather than after a fixed sleep is load-bearing.
/// `cursor_position` allows the terminal one second to respond, and
/// `Session::spawn` returns long before the probe emits its query, so a timed
/// reply spends most of that budget on process startup and lands with only a
/// few tens of milliseconds to spare — enough on an idle host, not enough on a
/// contended CI runner, where the probe reports `None`.
///
/// This removes spawn latency from the response path entirely. It does not
/// make the exchange instantaneous: the driver polls on an interval and can
/// still be descheduled between seeing the query and writing the reply. The
/// gain is a far larger share of the probe's budget, not a guaranteed margin.
///
/// Draining through `marker` (rather than stopping at the query) also lets the
/// probe run to completion before the session is dropped.
fn cursor_probe_output(mode: &str, reply: &'static [u8], marker: &str) -> String {
    let mut session = spawn_with_env(&[("PROBE", mode), ("PROBE_TERM_PROGRAM", "WezTerm")]);
    let mut answers = [ProbeAnswer::new(DSR_QUERY, reply)];
    let collected = drive_probe(&mut session, &mut answers, marker, PROBE_DEADLINE);
    String::from_utf8_lossy(&collected).into_owned()
}

#[test]
#[serial]
fn cursor_position_query_emits_csi_6n() {
    // A valid CPR is supplied even though this test only asserts on the query,
    // so the probe completes its exchange instead of being torn down mid-read.
    let output = cursor_probe_output("cursor", b"\x1b[12;34R", "cursor_position=");

    // The query goes to /dev/tty, which is the same PTY the probe's stdout
    // lands on, so it is visible in the collected master stream.
    assert!(
        output.contains("\x1b[6n"),
        "expected DSR query in output, got: {output:?}"
    );
}

#[test]
#[serial]
fn cursor_position_parses_csi_r_reply() {
    let output = cursor_probe_output("cursor", b"\x1b[12;34R", "cursor_position=");

    assert!(
        output.contains("cursor_position=Some(CursorPosition { row: 12, col: 34 })"),
        "expected parsed cursor position in output, got: {output}"
    );
}

#[test]
#[serial]
fn cursor_position_with_timeout_parses_cpr_reply() {
    let output = cursor_probe_output("cursor_timeout", b"\x1b[7;42R", "cursor_timeout=");

    assert!(
        output.contains("cursor_timeout=Some(CursorPosition { row: 7, col: 42 })"),
        "expected parsed cursor_timeout position in output, got: {output}"
    );
}
