//! Level 2 PTY/real-terminal capture test for the validation failure reporter.
//!
//! This test addresses the second half of the High finding in
//! `review-1.md`: the validation failure block emits OSC-8 hyperlinks, SGR
//! styling, and a chrome-free YAML snippet — all of which are observable only
//! when a real terminal is attached. It complements the Level 1 asserted
//! string tests in `claudine::harness::report::tests` (Phase 5) by verifying
//! the same renderer survives a real PTY end-to-end.
//!
//! # Approach
//!
//! Rather than coercing the full `claudine compose` pipeline into emitting a
//! failure block under PTY (which would require staging a stub provider on
//! `PATH`, configuring a writable workspace, and matching the harness'
//! resolver expectations), this test invokes the dedicated test-only binary
//! `validation_reporter_pty_harness` (defined in
//! `src/bin/validation_reporter_pty_harness.rs`). That binary calls the
//! real reporter through its public API, so the rendered output is
//! byte-identical to what the production CLI would emit for an equivalent
//! failure.
//!
//! The plan in `review-plan-1.md` Phase 6 explicitly permits this fallback
//! path; see the "Risks / open questions" section.
//!
//! # Gating
//!
//! Marked `#[cfg(unix)]` and `#[ignore]` to mirror the existing PTY tests in
//! `level2_pty_tests.rs`. Run locally with:
//!
//! ```text
//! cargo test -p claudine --test level2_validation_reporter_pty -- --ignored
//! ```

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
/// Reads with a short polling deadline so we collect the full transcript a
/// short-lived child writes before exiting. Returns the concatenated bytes
/// as a `String` with `from_utf8_lossy` so we never panic on stray ANSI
/// fragments.
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

/// Strip ANSI/OSC escape sequences from a transcript so tests can reason
/// about visible text only. Mirrors the helper in
/// `claudine::harness::report::tests`.
fn strip_ansi(text: &str) -> String {
    biscuit_terminal::utils::escape_codes::strip_escape_codes(text)
}

/// Spawn the PTY harness binary against a real fixture file and return the
/// captured transcript.
fn run_harness_under_pty(fixture_abs_path: &str) -> String {
    // Build a fresh workspace so any side-effect files the harness might
    // produce don't pollute the test directory.
    let workspace = tempdir().expect("create workspace tempdir");

    let mut cmd = Command::new(cargo_bin!("validation_reporter_pty_harness"));
    cmd.arg(fixture_abs_path);
    // Ensure ANSI is enabled and capabilities are detected from a 256-color
    // terminal. We deliberately do NOT set `TERM_WIDTH=80` — that would
    // suppress color in some Terminal builders. We also unset `NO_COLOR`
    // and `CLAUDINE_PLAIN` so the reporter's full styling path runs.
    cmd.env("TERM", "xterm-256color");
    cmd.env("COLORTERM", "truecolor");
    cmd.env_remove("NO_COLOR");
    cmd.env_remove("CLAUDINE_PLAIN");
    cmd.env_remove("CLICOLOR_FORCE");
    cmd.current_dir(workspace.path());

    let mut session: OsSession = Session::spawn(cmd).expect("spawn PTY session");
    session.set_expect_timeout(Some(Duration::from_secs(5)));

    // Send no input; the harness writes its output to stderr and exits.
    // Drain everything for a generous window so the child can emit its
    // entire transcript.
    let transcript = read_all_available(&mut session, Duration::from_secs(3));

    // Best-effort flush; ignore any error from a child that has already
    // exited.
    let _ = session.flush();

    transcript
}

#[test]
#[ignore = "PTY tests are timing-sensitive; gated identically to claudine level2_pty_tests.rs"]
fn pty_pre_check_failure_emits_osc8_link_and_styled_header() {
    // The fixture path is propagated into the harness binary, which then
    // emits a `RuleSource { file: <abs-path>, .. }`. We assert the
    // resulting transcript carries the OSC-8 link target referencing this
    // exact absolute path.
    let fixture_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("validation_reporter")
        .join("missing_file.md");
    let fixture_abs = std::fs::canonicalize(&fixture_path)
        .expect("fixture must exist and be canonicalizable")
        .display()
        .to_string();

    let transcript = run_harness_under_pty(&fixture_abs);

    assert!(
        !transcript.is_empty(),
        "PTY transcript should not be empty; got: {transcript:?}"
    );

    // -- OSC-8 introducer present and references the fixture -----------
    let osc8_introducer = "\x1b]8;;";
    let intro_idx = transcript.find(osc8_introducer).unwrap_or_else(|| {
        panic!("expected OSC-8 introducer '\\x1b]8;;' in transcript: {transcript:?}")
    });
    let after_intro = &transcript[intro_idx + osc8_introducer.len()..];
    let st_idx = after_intro.find("\x1b\\").unwrap_or(after_intro.len());
    let bel_idx = after_intro.find('\x07').unwrap_or(after_intro.len());
    let target_end = st_idx.min(bel_idx);
    let target = &after_intro[..target_end];
    assert!(
        target.contains(&fixture_abs),
        "OSC-8 link target should contain the absolute fixture path {fixture_abs:?}, \
         got target: {target:?}"
    );
    // The Phase 5 hint notes that link targets are emitted with a `file://`
    // scheme prefix. Use a `contains` check (per the prompt instructions)
    // so this remains robust if the renderer ever switches to a bare path.
    assert!(
        target.contains("file://"),
        "OSC-8 link target should include the 'file://' scheme prefix, \
         got target: {target:?}"
    );

    // -- OSC-8 close sequence present ----------------------------------
    let close_seq = "\x1b]8;;\x1b\\";
    assert!(
        transcript.matches(close_seq).count() >= 1,
        "expected at least one empty OSC-8 close sequence (\\x1b]8;;\\x1b\\\\) \
         in transcript: {transcript:?}"
    );

    // -- SGR styling on the failure-header line -----------------------
    //
    // The reporter renders the "Pre-validation failed" header through the
    // status-failure pipeline which wraps the text in a SGR sequence
    // (typically a red foreground, e.g. `\x1b[31m` or `\x1b[38;5;…m`).
    // Locate the byte offset of `Pre-validation failed` and assert that
    // the preceding 32-byte window contains an SGR introducer and a
    // foreground-color parameter starting with `3`.
    let header_offset = transcript
        .find("Pre-validation failed")
        .expect("expected 'Pre-validation failed' header in transcript");
    let window_start = header_offset.saturating_sub(64);
    let pre_header = &transcript[window_start..header_offset];
    assert!(
        pre_header.contains("\x1b["),
        "expected an SGR/CSI introducer ('\\x1b[') before 'Pre-validation failed', \
         got window: {pre_header:?}"
    );
    // The status-failure rendering is a foreground color (parameter family
    // `3x` or `38;...`), so the window should contain `\x1b[3` somewhere.
    assert!(
        pre_header.contains("\x1b[3"),
        "expected an SGR foreground-color parameter (e.g. '\\x1b[31m', \
         '\\x1b[38;...m', or italic '\\x1b[3m') before 'Pre-validation failed', \
         got window: {pre_header:?}"
    );

    // -- YAML region carries SGR styling (positive assertion) ---------
    //
    // The renderer falls back to unstyled lines when
    // `darkmatter::markdown::highlighting::highlight_yaml_lines` returns a
    // line count that doesn't match the plain count. Without a positive
    // assertion here, a degraded highlighter (broken theme load, missing
    // yaml grammar, empty palette) would silently strip styling and the
    // existing chrome-removal guard would still pass.
    //
    // The harness emits `line_range: Some(3..=5)`, so the source line
    // contains `:3-5` and the reason line contains `Reason:`. The bytes
    // between those markers are the YAML region (plus one closing OSC-8
    // sequence and a CSI reset on the source line). `highlight_yaml_lines`
    // emits truecolor foreground SGR escapes (`\x1b[38;2;R;G;Bm`) for each
    // highlighted token; asserting on the `\x1b[38;` introducer guarantees
    // real styling fired in the YAML region rather than just a bare reset.
    let source_marker = transcript
        .find(":3-5")
        .expect("expected source-location ':3-5' marker in transcript");
    let reason_marker = transcript
        .find("Reason:")
        .expect("expected 'Reason:' marker in transcript");
    assert!(
        reason_marker > source_marker,
        "Reason marker must follow source-location marker"
    );
    let yaml_region = &transcript[source_marker..reason_marker];
    assert!(
        yaml_region.contains("\x1b[38;"),
        "YAML region between ':3-5' and 'Reason:' must contain at least one \
         foreground-color SGR ('\\x1b[38;...m'), got region: {yaml_region:?}"
    );

    // -- No `yaml` fence label (chrome-removal regression guard) -------
    //
    // The chrome-free YAML helper introduced in Phase 4 must never emit a
    // standalone `yaml` language label, which is the tell-tale sign of a
    // fenced code-block frame. This is the integration-level analog of
    // `render_failure_block_yaml_section_has_no_yaml_label_or_box`.
    assert!(
        !transcript.contains("\nyaml\n"),
        "transcript must not contain the standalone 'yaml' fence label, \
         got transcript: {transcript:?}"
    );

    // -- Reason line is visible to the user ----------------------------
    let plain = strip_ansi(&transcript);
    assert!(
        plain.contains("Reason: file does not exist:"),
        "ANSI-stripped transcript should contain 'Reason: file does not exist:', \
         got plain: {plain:?}"
    );
}
