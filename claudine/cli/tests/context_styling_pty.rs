//! PTY smoke tests for `claudine context` styling (Level 1).
//!
//! These spawn `claudine context` under a manufactured PTY (`expectrl`) with a
//! 256-color `TERM` so the renderer emits its SGR pipeline, then assert escapes
//! are present in the byte stream. A PTY is *not* a real terminal emulator: it
//! does not run inside WezTerm/Kitty/tmux and does not capture a rendered pane,
//! so under the repo's taxonomy this is **Level 1** (process I/O), not Level 2.
//! Genuine real-terminal pane capture for these reports lives in
//! `level2_context_capture.rs`.
//!
//! Gated by `require_level!(Level::L1, pty_available(), ...)` so it skips
//! cleanly when no PTY is available.

#![cfg(unix)]

#[allow(deprecated)]
use assert_cmd::cargo::cargo_bin;
use expectrl::Session;
use expectrl::session::OsSession;
use std::io::Write;
use std::process::Command;
use std::time::{Duration, Instant};
use tempfile::tempdir;
use test_toolkit::{Level, require_level};

mod common;
use common::pty_available;

/// Drain whatever bytes are currently buffered on the PTY master side.
///
/// Mirrors the helper in `validation_reporter_pty.rs`: reads with a short
/// polling deadline so we collect the full transcript before the test
/// process exits.
fn read_all_available(session: &mut OsSession, total_deadline: Duration) -> String {
    let mut buf = Vec::new();
    let mut scratch = [0u8; 4096];
    let deadline = Instant::now() + total_deadline;
    session.set_expect_timeout(Some(Duration::from_millis(300)));
    while Instant::now() < deadline {
        match session.try_read(&mut scratch) {
            Ok(0) => break,
            Ok(n) => buf.extend_from_slice(&scratch[..n]),
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
    String::from_utf8_lossy(&buf).into_owned()
}

/// Spawn `claudine context [args...]` under a 256-color PTY and return the
/// merged stdout/stderr transcript.
fn run_context_under_pty(args: &[&str]) -> String {
    let workspace = tempdir().expect("create workspace tempdir");

    let mut cmd = Command::new(cargo_bin!("claudine"));
    cmd.arg("context");
    cmd.args(args);
    // Force a known-good color terminal so the styling pipeline runs at
    // full fidelity. Strip any env vars that would suppress color.
    cmd.env("TERM", "xterm-256color");
    cmd.env("COLORTERM", "truecolor");
    cmd.env_remove("NO_COLOR");
    cmd.env_remove("CLAUDINE_PLAIN");
    cmd.env_remove("CLICOLOR_FORCE");
    cmd.current_dir(workspace.path());

    let mut session: OsSession = Session::spawn(cmd).expect("spawn PTY session");
    session.set_expect_timeout(Some(Duration::from_secs(5)));

    let transcript = read_all_available(&mut session, Duration::from_secs(5));
    let _ = session.flush();
    transcript
}

#[test]
#[serial_test::serial(pty)]
fn l1_pty_context_default_emits_sgr_styling_in_output() {
    require_level!(Level::L1, pty_available(), "PTY (/dev/ptmx)");
    let transcript = run_context_under_pty(&[]);

    assert!(
        !transcript.is_empty(),
        "default `claudine context` should produce a non-empty transcript",
    );

    // The renderer emits SGR escapes for headings, table chrome, and the
    // styled footer hints. A bare `\x1b[` introducer is enough to prove the
    // styling pipeline fired — without it we'd be back to plain text and
    // the spec's visible styling requirement would be silently broken.
    assert!(
        transcript.contains("\x1b["),
        "transcript must contain at least one SGR/CSI escape; transcript: {transcript:?}",
    );

    // A user-visible token from the report. `ctx.today` is always present.
    assert!(
        transcript.contains("ctx.today"),
        "transcript must contain `ctx.today`; transcript: {transcript:?}",
    );
}

#[test]
#[serial_test::serial(pty)]
fn l1_pty_context_footer_uses_blue_for_flag_names() {
    require_level!(Level::L1, pty_available(), "PTY (/dev/ptmx)");
    let transcript = run_context_under_pty(&[]);

    // The footer hint reads roughly:
    //   "use --values to convert the descriptions to actual host values"
    //   "use --expressions to see the expression engine's …"
    //   "use --side-effects to see the available safe side effects …"
    //
    // The flag names are wrapped in `<blue>…</blue>` by the renderer, which
    // emits a blue SGR foreground (basic-blue `\x1b[34m` or 256/truecolor
    // variants beginning `\x1b[38;`). Search the bytes immediately before
    // each flag name for one of these blue introducers.
    for flag in ["--values", "--expressions", "--side-effects"] {
        let offset = transcript.find(flag).unwrap_or_else(|| {
            panic!("expected footer to mention `{flag}`; transcript: {transcript:?}")
        });
        // Walk the start back to a char boundary: the footer may contain
        // multi-byte nerd-font glyphs, so a raw byte offset can land mid-glyph.
        let mut window_start = offset.saturating_sub(32);
        while window_start > 0 && !transcript.is_char_boundary(window_start) {
            window_start -= 1;
        }
        let window = &transcript[window_start..offset];
        let has_basic_blue = window.contains("\x1b[34");
        let has_truecolor_or_256 = window.contains("\x1b[38;");
        assert!(
            has_basic_blue || has_truecolor_or_256,
            "footer flag `{flag}` must be preceded by a blue SGR introducer \
             (`\\x1b[34…` or `\\x1b[38;…`); window before flag: {window:?}",
        );
    }
}

#[test]
#[serial_test::serial(pty)]
fn l1_pty_context_renders_table_box_drawing() {
    require_level!(Level::L1, pty_available(), "PTY (/dev/ptmx)");
    let transcript = run_context_under_pty(&[]);

    // `biscuit-terminal::Table` draws its borders with Unicode box-drawing
    // characters. Their presence (combined with the SGR escapes verified by
    // the previous test) proves the production table renderer ran rather
    // than falling back to a degraded path. We accept any of the corner /
    // edge glyphs that `Table` uses across its border presets.
    let table_glyphs = ['┌', '┐', '└', '┘', '├', '┤', '┬', '┴', '┼', '─', '│'];
    assert!(
        table_glyphs.iter().any(|ch| transcript.contains(*ch)),
        "transcript must contain box-drawing characters from the table renderer; \
         transcript: {transcript:?}",
    );
}

#[test]
#[serial_test::serial(pty)]
fn l1_pty_context_values_renders_canonical_keys_with_styling() {
    require_level!(Level::L1, pty_available(), "PTY (/dev/ptmx)");
    let transcript = run_context_under_pty(&["--values"]);

    // The canonical keys behind the date aliases always carry real values.
    // (The aliases themselves are separate `Date and Time → Aliases` rows; this
    // check targets their canonical counterparts.)
    for key in ["ctx.now_utc", "ctx.day", "ctx.day_abbr"] {
        let line = transcript
            .lines()
            .find(|line| line.contains(key))
            .unwrap_or_else(|| panic!("expected `{key}` row; transcript: {transcript:?}"));
        assert!(
            !line.contains("null"),
            "{key} row must show a real value, not `null`; row: {line:?}",
        );
    }
}
