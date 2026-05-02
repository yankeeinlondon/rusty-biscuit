//! Level-1 PTY tests for OSC52 clipboard.
//!
//! Verifies that `set_clipboard` emits the correct OSC52 escape sequence
//! when running inside a PTY with a supported terminal.

mod common;

use std::time::Duration;

use common::pty::spawn_with_env;

#[test]
fn osc52_sequence_emitted_to_tty() {
    let mut session = spawn_with_env(&[("PROBE", "clipboard"), ("PROBE_TERM_PROGRAM", "WezTerm")]);

    // Give the probe time to run and emit the sequence.
    std::thread::sleep(Duration::from_millis(150));

    // Drain all output.
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

    // OSC52 write sequence:
    // ESC ] 52 ; c ; <base64> BEL
    // base64("hello-pty") = "aGVsbG8tcHR5"
    assert!(
        output.contains("\x1b]52;c;"),
        "expected OSC52 header in output, got: {output:?}"
    );
    assert!(
        output.contains("aGVsbG8tcHR5"),
        "expected base64 payload in output, got: {output:?}"
    );
    assert!(
        output.contains("\x07"),
        "expected BEL terminator in output, got: {output:?}"
    );
}
