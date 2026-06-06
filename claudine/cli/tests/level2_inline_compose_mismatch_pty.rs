//! L2 PTY / real-terminal capture tests for the inline-compose / sequence
//! mismatch diagnostic.
//!
//! These address the second High finding in
//! `claudine/fixes/2026-06-06-inline-sequence/review-2.md`: the piped
//! `assert_cmd` tests in `inline_compose_sequence_mismatch.rs` always exercise
//! the non-TTY (YAML-withheld) branch, so they cannot prove that the real
//! command, with a TTY error stream, (a) takes the TTY branch and emits the
//! verbatim authored YAML, and (b) renders the styled diagnostic and the OSC 8
//! document link.
//!
//! Each test spawns the real `claudine` binary with stderr attached to a PTY so
//! `std::io::stderr().is_terminal()` is true (TTY branch) and forces an
//! optimistic color terminal (`FORCE_COLOR=1`) so the SGR + OSC 8 pipeline runs
//! at full fidelity. Gated by `require_level!(Level::L2, pty_available(), …)` so
//! they skip cleanly without a PTY and panic under `BISCUIT_TEST_LEVEL_REQUIRED=2`.
//!
//! Exact YAML line-ending fidelity (LF vs CRLF, delimiter exclusion) is proved
//! precisely by the L1 capture/render tests in
//! `claudine/lib/src/composition/error.rs` and `…/composition/mismatch.rs`; a
//! PTY rewrites `\n` to `\r\n` on output, so these tests assert per-line
//! verbatim fragments rather than the exact multi-line payload.

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

/// A mismatch fixture that exercises the YAML fidelity surface: a leading
/// comment, non-canonical property order (`sequence` before `prompt`), a YAML
/// anchor + alias, and a block scalar. Both `prompt` and `sequence` are
/// non-null, so the document is an inline-compose / sequence mismatch.
// NB: a plain literal with explicit `\n` — a `\`-newline line continuation
// would strip each line's leading indentation and corrupt the YAML structure.
const MISMATCH_FIXTURE: &str = "---\n# leading comment\nsequence: &seq\n  - name: Hello\n  - name: Goodbye\nprompt: |-\n  multi\n  line\nalias: *seq\n---\nbody\n";

/// Drain whatever bytes are currently buffered on the PTY master side until a
/// quiet period or the overall deadline elapses.
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

/// Write `MISMATCH_FIXTURE` into a fresh workspace and run
/// `claudine inline-compose <doc>` under an optimistic-color PTY, returning the
/// merged stdout/stderr transcript and the resolved document path.
fn run_mismatch_under_pty() -> (String, std::path::PathBuf) {
    let workspace = tempdir().expect("create workspace tempdir");
    let md_file = workspace.path().join("doc.md");
    std::fs::write(&md_file, MISMATCH_FIXTURE).expect("write fixture");

    let mut cmd = Command::new(cargo_bin!("claudine"));
    cmd.arg("inline-compose").arg(&md_file);
    // Force a known-good color terminal so SGR + OSC 8 are emitted regardless of
    // the inherited environment; strip anything that would suppress color.
    cmd.env("TERM", "xterm-256color");
    cmd.env("COLORTERM", "truecolor");
    cmd.env("FORCE_COLOR", "1");
    cmd.env_remove("NO_COLOR");
    cmd.env_remove("CLAUDINE_PLAIN");
    cmd.current_dir(workspace.path());

    let mut session: OsSession = Session::spawn(cmd).expect("spawn PTY session");
    session.set_expect_timeout(Some(Duration::from_secs(5)));
    let transcript = read_all_available(&mut session, Duration::from_secs(5));
    let _ = session.flush();

    // `canonicalize` mirrors `render_file_link`, which canonicalizes the path
    // before building the OSC 8 target.
    let canonical = md_file.canonicalize().unwrap_or(md_file);
    (transcript, canonical)
}

#[test]
#[serial_test::serial(pty)]
fn level2_pty_mismatch_takes_tty_branch_with_verbatim_yaml() {
    require_level!(Level::L2, pty_available(), "PTY (/dev/ptmx)");
    let (transcript, _doc) = run_mismatch_under_pty();

    assert!(
        !transcript.is_empty(),
        "inline-compose on a mismatch must produce a non-empty transcript",
    );

    // The TTY branch was taken: the YAML intro and the directive are present,
    // and the withheld note is not.
    assert!(
        transcript.contains("Below is the full YAML definition"),
        "TTY output must include the YAML intro; transcript: {transcript:?}",
    );
    assert!(
        transcript.contains("claudine sequence"),
        "diagnostic must direct the user to `claudine sequence`; transcript: {transcript:?}",
    );
    assert!(
        !transcript.contains("withheld"),
        "TTY output must NOT claim the YAML was withheld; transcript: {transcript:?}",
    );

    // The authored YAML is reproduced verbatim. A PTY rewrites `\n` to `\r\n`,
    // so assert per-line fragments that prove fidelity: the leading comment,
    // the anchor and alias, the non-canonical ordering, and the block scalar.
    for fragment in [
        "# leading comment",
        "sequence: &seq",
        "- name: Hello",
        "- name: Goodbye",
        "prompt: |-",
        "alias: *seq",
    ] {
        assert!(
            transcript.contains(fragment),
            "verbatim YAML fragment `{fragment}` missing; transcript: {transcript:?}",
        );
    }
}

#[test]
#[serial_test::serial(pty)]
fn level2_pty_mismatch_emits_sgr_and_osc8_link() {
    require_level!(Level::L2, pty_available(), "PTY (/dev/ptmx)");
    let (transcript, doc) = run_mismatch_under_pty();

    // Styling fired: the status block border and inline `<cyan>` tags emit SGR
    // escapes. A bare CSI introducer proves the styled pipeline ran rather than
    // degrading to plain text.
    assert!(
        transcript.contains("\x1b["),
        "transcript must contain at least one SGR/CSI escape; transcript: {transcript:?}",
    );

    // The resolved document is rendered as an OSC 8 hyperlink: `\x1b]8;;<uri>`
    // with the canonical path as the target.
    assert!(
        transcript.contains("\x1b]8;;"),
        "transcript must contain an OSC 8 hyperlink introducer; transcript: {transcript:?}",
    );
    let doc_name = doc.file_name().unwrap().to_string_lossy();
    assert!(
        transcript.contains(doc_name.as_ref()),
        "OSC 8 link must reference the resolved document `{doc_name}`; transcript: {transcript:?}",
    );
}
