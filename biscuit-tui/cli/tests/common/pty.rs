//! Shared PTY helpers for `biscuit-tui-cli` integration tests.
//!
//! Any test that spawns `question` with `--height` (Inline-viewport mode)
//! MUST call [`answer_cursor_position_request`] before sending input;
//! otherwise the binary blocks during initialisation while crossterm
//! waits synchronously for the DSR (`\x1b[6n`) reply.
//!
//! See `biscuit-tui/features/2026-04-28-choose-one-improvements/pty-test-bugs.md`
//! Bug 2 for the underlying analysis. Inline-viewport mode is a
//! ratatui `CrosstermBackend` requirement — the backend issues a DSR
//! cursor-position query during `Terminal::with_options` initialisation
//! and synchronously reads the reply before returning. Without a
//! responder on the master side of the PTY, the binary deadlocks at
//! startup and the harness sees `ABORTED` (exit 1) once crossterm
//! eventually times out.

#![cfg(unix)]
// See `mod.rs` for context: each integration test crate consumes a
// subset of these helpers, so the dead-code lint over-fires without
// this allow.
#![allow(dead_code)]

use std::io::Write;
use std::time::{Duration, Instant};

use expectrl::session::OsSession;

/// Drains the PTY master FD until a DSR cursor-position query
/// (`\x1b[6n`) is observed, then writes a synthetic
/// `\x1b[1;1R` reply on behalf of the host terminal.
///
/// Behaviour:
///
/// - Polls the master FD with a 2 second deadline waiting for the DSR
///   query. Interrupted/wouldblock/timedout reads back off briefly and
///   keep polling until the deadline.
/// - On observing the query, writes `\x1b[1;1R` to the master so the
///   crossterm backend inside the child can finish initialising.
/// - Sleeps 300ms after the response so the child's initial render has
///   time to flush before any subsequent `try_read` calls.
/// - Resets the session's expect timeout to 10 seconds before returning
///   so downstream callers see a sane default regardless of what the
///   spawn site configured.
///
/// ## Notes
///
/// This helper is intentionally a single source of truth for both
/// `choose_cli.rs::pty` and `keyboard_protocol.rs`. Future fixes to the
/// DSR handshake belong here so the two harnesses cannot drift.
pub fn answer_cursor_position_request(session: &mut OsSession) {
    let deadline = Instant::now() + Duration::from_secs(2);
    let mut output = String::new();
    let mut scratch = [0u8; 4096];

    while Instant::now() < deadline {
        match session.try_read(&mut scratch) {
            Ok(0) => break,
            Ok(n) => {
                output.push_str(&String::from_utf8_lossy(&scratch[..n]));
                if output.contains("\x1b[6n") {
                    session
                        .write_all(b"\x1b[1;1R")
                        .expect("send cursor position response");
                    break;
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(20));
            }
            Err(e) if e.kind() == std::io::ErrorKind::TimedOut => {
                std::thread::sleep(Duration::from_millis(20));
            }
            Err(_) => break,
        }
    }

    session.set_expect_timeout(Some(Duration::from_secs(10)));
    std::thread::sleep(Duration::from_millis(300));
}
