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
        ("PROBE_TERM", "xterm-256color"),
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

/// `PROBE_FORCE_OSC8=true` must override the canonical
/// `detection::osc8_link_support()` policy and re-enable OSC8 emission
/// even when the probe's other inputs (e.g. `TERM_PROGRAM=Apple_Terminal`)
/// would normally resolve to "no OSC8 support".
///
/// This is the positive-case Level-1 proof for the override path that
/// the production probe inherits from
/// [`biscuit_terminal::discovery::detection::osc8_link_support`].
#[test]
fn probe_force_osc8_emits_osc_when_forced_on() {
    let mut session = spawn_with_env(&[
        ("PROBE", "prose"),
        ("PROBE_TERM_PROGRAM", "Apple_Terminal"),
        ("PROBE_FORCE_OSC8", "true"),
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
        output.contains("\x1b]8;;https://example.com"),
        "expected OSC8 escape with href when override is forced on, got: {output:?}"
    );
    assert!(
        !output.contains("[click here]("),
        "expected no markdown fallback when override forces OSC8 on, got: {output:?}"
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

/// `<double-underline>` with no underline support must render as exactly
/// the literal text — no SGR sequences of any kind between the probe
/// markers. This is the strict exact-match companion to
/// `no_underline_support_emits_plain_text`.
#[test]
fn double_underline_no_underline_support_emits_exact_plain_text() {
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

    let rendered = output
        .split("---PROSE---")
        .nth(1)
        .and_then(|s| s.split("---END---").next())
        .unwrap_or("")
        .trim_matches(['\r', '\n']);

    #[cfg(windows)]
    let rendered = {
        let rendered = if let Some(title) = rendered.strip_prefix("\x1b]0;") {
            title.split_once('\x07').map_or(rendered, |(_, rest)| rest)
        } else {
            rendered
        };
        rendered
            .strip_prefix("\x1b[?25h")
            .unwrap_or(rendered)
            .trim_matches(['\r', '\n'])
    };

    // The span produces no visible SGR (the terminal lacks underline support),
    // so it opens nothing and emits no stray reset — the rendered payload is the
    // bare literal with no escapes. See prose/mod.rs
    // `test_double_underline_suppressed_when_no_underline_support`.
    assert_eq!(
        rendered, "important text",
        "expected bare literal with no escapes between probe markers, got: {rendered:?} (full output: {output:?})"
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
