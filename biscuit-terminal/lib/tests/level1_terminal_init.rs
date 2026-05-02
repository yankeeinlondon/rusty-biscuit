//! Level-1 PTY test for Terminal initialisation.
//!
//! Verifies that `Terminal::new()` produces consistent capability fields
//! when run inside a PTY with a manufactured terminal identity.

mod common;

use std::time::Duration;

use common::pty::spawn_with_env;

#[test]
fn terminal_new_cascade_produces_consistent_fields_in_pty() {
    let mut session = spawn_with_env(&[("PROBE", "terminal"), ("PROBE_TERM_PROGRAM", "ghostty")]);

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

    // Inside a PTY stdout is a TTY.
    assert!(
        output.contains("is_tty=true"),
        "expected TTY detection inside PTY, got: {output}"
    );

    // The terminal app should match the env override.
    assert!(
        output.contains("app=Ghostty"),
        "expected Ghostty app detection, got: {output}"
    );

    // Ghostty is known to support several features.
    assert!(
        output.contains("supports_italic=true"),
        "expected italic support for Ghostty, got: {output}"
    );
    assert!(
        output.contains("osc_link_support=true"),
        "expected OSC8 support for Ghostty, got: {output}"
    );
}
