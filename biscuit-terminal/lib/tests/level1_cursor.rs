//! Level-1 PTY tests for cursor position queries.
//!
//! Verifies that `cursor_position` emits `CSI 6n` and correctly parses
//! the CPR (`CSI row;col R`) response.

mod common;

use std::io::Write;
use std::time::Duration;

use common::pty::spawn_with_env;

#[test]
fn cursor_position_query_emits_csi_6n() {
    let mut session = spawn_with_env(&[("PROBE", "cursor"), ("PROBE_TERM_PROGRAM", "WezTerm")]);

    std::thread::sleep(Duration::from_millis(80));

    // Manufacture a CPR response.
    session
        .write_all(b"\x1b[12;34R")
        .expect("failed to send CPR reply");
    session.flush().unwrap();

    std::thread::sleep(Duration::from_millis(80));

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

    // The query itself is sent to stdout (the PTY), so we should be able
    // to see it in the captured output as well.
    assert!(
        output.contains("\x1b[6n"),
        "expected DSR query in output, got: {output:?}"
    );
}

#[test]
fn cursor_position_parses_csi_r_reply() {
    let mut session = spawn_with_env(&[("PROBE", "cursor"), ("PROBE_TERM_PROGRAM", "WezTerm")]);

    std::thread::sleep(Duration::from_millis(80));

    // Manufacture a CPR response: row 12, col 34.
    session
        .write_all(b"\x1b[12;34R")
        .expect("failed to send CPR reply");
    session.flush().unwrap();

    std::thread::sleep(Duration::from_millis(80));

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

    assert!(
        output.contains("cursor_position=Some(CursorPosition { row: 12, col: 34 })"),
        "expected parsed cursor position in output, got: {output}"
    );
}
