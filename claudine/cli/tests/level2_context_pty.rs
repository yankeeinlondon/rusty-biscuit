//! PTY/real-terminal capture tests for `claudine context`.
//!
//! These tests address the third High finding in
//! `claudine/features/2026-05-23-context/review-2.md`: the styled footer
//! messages and styled headings/tables are user-visible behavior that
//! cannot be verified by in-process unit tests. The integration tests in
//! `context_command.rs` run with `NO_COLOR=1` and only verify routing and
//! content. These tests spawn `claudine context` under a PTY with a
//! 256-color terminal so the renderer emits its full SGR pipeline, and
//! assert the styling escapes are present where the spec requires them.
//!
//! The tests follow the gating and harness pattern established by
//! `level2_validation_reporter_pty.rs` — `#[cfg(unix)]` + `#[ignore]` so
//! they mirror the rest of the PTY suite.

#![cfg(unix)]

#[allow(deprecated)]
use assert_cmd::cargo::cargo_bin;
use expectrl::Session;
use expectrl::session::OsSession;
use std::io::Write;
use std::process::Command;
use std::time::{Duration, Instant};
use tempfile::tempdir;

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
#[ignore = "PTY tests are timing-sensitive; gated identically to claudine pty_tests.rs"]
fn pty_context_default_emits_sgr_styling_in_output() {
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
#[ignore = "PTY tests are timing-sensitive; gated identically to claudine pty_tests.rs"]
fn pty_context_footer_uses_blue_for_flag_names() {
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
        let window_start = offset.saturating_sub(32);
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
#[ignore = "PTY tests are timing-sensitive; gated identically to claudine pty_tests.rs"]
fn pty_context_renders_table_box_drawing() {
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
#[ignore = "PTY tests are timing-sensitive; gated identically to claudine pty_tests.rs"]
fn pty_context_values_resolves_aliases_with_styling() {
    let transcript = run_context_under_pty(&["--values"]);

    // Locate the row for `ctx.utc` (alias for `now_utc`) and confirm the
    // value cell is not the dim `null` rendering. The renderer emits
    // `null` only when the underlying value is missing; the alias fix
    // populates it so the row must show a real value.
    let utc_line = transcript
        .lines()
        .find(|line| line.contains("ctx.utc"))
        .unwrap_or_else(|| panic!("expected `ctx.utc` row; transcript: {transcript:?}"));
    assert!(
        !utc_line.contains("null"),
        "ctx.utc row must show a real value, not `null`; row: {utc_line:?}",
    );
}
