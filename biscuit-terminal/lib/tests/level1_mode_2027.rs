//! Level-1 PTY tests for Mode 2027 (grapheme cluster width).
//!
//! Verifies that `enable_mode_2027` and `disable_mode_2027` emit the
//! correct escape sequences when the terminal is known to support them.

mod common;

use std::time::Duration;

use common::pty::spawn_with_env;

#[test]
fn enable_mode_2027_emits_escape_sequence() {
    let mut session = spawn_with_env(&[("PROBE", "mode2027"), ("PROBE_TERM_PROGRAM", "WezTerm")]);

    std::thread::sleep(Duration::from_millis(150));

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
        output.contains("\x1b[?2027h"),
        "expected enable sequence in output, got: {output:?}"
    );
}

#[test]
fn disable_mode_2027_emits_escape_sequence() {
    let mut session = spawn_with_env(&[("PROBE", "mode2027"), ("PROBE_TERM_PROGRAM", "WezTerm")]);

    std::thread::sleep(Duration::from_millis(150));

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
        output.contains("\x1b[?2027l"),
        "expected disable sequence in output, got: {output:?}"
    );
}
