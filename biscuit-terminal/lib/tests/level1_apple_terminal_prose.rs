//! Level-1 PTY tests for Apple Terminal prose degradation.
//!
//! Spawns the `discovery_probe` example in a PTY with
//! `TERM_PROGRAM=Apple_Terminal` (or with explicit capability overrides)
//! and asserts that [`Prose`] degrades unsupported escape sequences
//! exactly as required by the Apple Terminal spec.
//!
//! Run `cargo build -p biscuit-terminal --example discovery_probe` first.

mod common;

use std::time::Duration;

use common::pty::spawn_with_env;

/// Drain a probe session, polling until both the `---END---` marker is
/// seen and the pipe returns zero bytes. The probe runs several
/// capability detection queries before printing the rendered prose so a
/// single short read is not enough to capture the payload.
fn drain(session: &mut expectrl::session::OsSession) -> String {
    let deadline = std::time::Instant::now() + Duration::from_secs(3);
    let mut output = String::new();
    let mut scratch = [0u8; 4096];
    loop {
        match session.try_read(&mut scratch) {
            Ok(0) => {
                if output.contains("---END---") || std::time::Instant::now() > deadline {
                    break;
                }
                std::thread::sleep(Duration::from_millis(25));
            }
            Ok(n) => {
                output.push_str(&String::from_utf8_lossy(&scratch[..n]));
                if output.contains("---END---") {
                    break;
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e)
                if e.kind() == std::io::ErrorKind::WouldBlock
                    || e.kind() == std::io::ErrorKind::TimedOut =>
            {
                if output.contains("---END---") || std::time::Instant::now() > deadline {
                    break;
                }
                std::thread::sleep(Duration::from_millis(25));
            }
            Err(_) => break,
        }
    }
    output
}

#[test]
fn apple_terminal_link_falls_back_to_markdown() {
    let mut session = spawn_with_env(&[
        ("PROBE", "prose"),
        ("PROBE_TERM_PROGRAM", "Apple_Terminal"),
        (
            "PROBE_PROSE_INPUT",
            "<a href=\"https://example.com\">click here</a>",
        ),
    ]);

    let output = drain(&mut session);

    assert!(
        output.contains("---PROSE---"),
        "expected prose probe markers, got: {output:?}"
    );
    assert!(
        output.contains("[click here](https://example.com)"),
        "expected markdown link fallback, got: {output:?}"
    );
    assert!(
        !output.contains("\x1b]8;;"),
        "expected no OSC8 escape on Apple Terminal, got: {output:?}"
    );
}

#[test]
fn apple_terminal_double_underline_degrades_to_straight() {
    let mut session = spawn_with_env(&[
        ("PROBE", "prose"),
        ("PROBE_TERM_PROGRAM", "Apple_Terminal"),
        (
            "PROBE_PROSE_INPUT",
            "<double-underline>important text</double-underline>",
        ),
    ]);

    let output = drain(&mut session);

    assert!(
        output.contains("---PROSE---"),
        "expected prose probe markers, got: {output:?}"
    );
    assert!(
        !output.contains("\x1b[4:2m"),
        "expected no double-underline SGR on Apple Terminal, got: {output:?}"
    );
    assert!(
        output.contains("\x1b[4mimportant text"),
        "expected straight underline opening before content, got: {output:?}"
    );
}

#[test]
fn no_underline_support_emits_plain_text() {
    // Use explicit overrides to manufacture the "no underline at all"
    // capability profile — no real terminal in the detection cascade
    // currently produces this state, but the policy must still hold.
    let mut session = spawn_with_env(&[
        ("PROBE", "prose"),
        ("PROBE_TERM_PROGRAM", "Apple_Terminal"),
        ("PROBE_FORCE_UNDERLINE_STRAIGHT", "false"),
        ("PROBE_FORCE_UNDERLINE_DOUBLE", "false"),
        (
            "PROBE_PROSE_INPUT",
            "<double-underline>important text</double-underline>",
        ),
    ]);

    let output = drain(&mut session);

    assert!(
        output.contains("---PROSE---"),
        "expected prose probe markers, got: {output:?}"
    );
    assert!(
        output.contains("important text"),
        "expected literal content, got: {output:?}"
    );
    assert!(
        !output.contains("\x1b[4:2m"),
        "expected no double-underline SGR, got: {output:?}"
    );
    assert!(
        !output.contains("\x1b[4m"),
        "expected no straight-underline SGR, got: {output:?}"
    );
}
