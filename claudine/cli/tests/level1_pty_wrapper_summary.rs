//! Level 1 PTY tests for the wrapper's pre-delegation summary.
//!
//! `claudine <provider> -- <prompt>` prints a summary before it execs the child
//! binary. These tests drive it through a pseudo-terminal so the summary is
//! produced on the interactive path rather than the piped one.
//!
//! ## What these assertions establish, and what they do not
//!
//! They establish **textual content and ordering** in the child's byte stream:
//! the header's provider name, the `YOLO` badge's label text, the
//! `INTERACTIVE` row of the environment-variables table, and that the whole
//! summary precedes the wrapped child's own output.
//!
//! They establish **nothing about how any of it looks**. Glyph width, SGR
//! styling, and the badge row's layout and wrapping are invisible to
//! `expectrl`, which matches substrings in a stream no emulator ever laid out.
//!
//! No real-emulator capture asserts the wrapper header's rendered form. The
//! nearest L2 evidence covers neighboring surfaces rather than this one:
//! `claudine::badges` builds each badge from `Prose` markup, and
//! `biscuit-terminal-cli`'s `level2_prose_styling.rs` proves that renderer
//! emits bold and foreground/background color SGR in real WezTerm and Kitty
//! sessions; this crate's `level2_dry_run_metadata_capture.rs` captures the
//! `--dry-run` metadata table's `YOLO` cell, a different surface from the
//! header badge row.
//!
//! ## Tier
//!
//! **Level 1.** `expectrl` opens `/dev/ptmx` and the test manufactures every
//! byte the child reads; no terminal emulator is involved, so nothing about the
//! emulator's rendering is under test. Level 2 means a real emulator session
//! reached through `biscuit-test-harness`.
//!
//! Gating: `#![cfg(unix)]`, `require_level!(Level::L1, pty_available(), ...)`
//! so the test skips cleanly without a PTY.
//!
//! Run via the canonical recipe:
//!
//! ```text
//! just test
//! ```

#![cfg(unix)]

use expectrl::{Expect, Session};
use test_toolkit::{Level, require_level};

mod common;
use common::{CliProcessFixture, pty_available, write_executable};

/// The summary's text — provider header, `YOLO` badge label, `INTERACTIVE`
/// environment row — reaches an interactive terminal before the wrapped child's
/// own output. Textual content and ordering only; see the module docs for why
/// this says nothing about the badge row's rendered appearance.
#[test]
fn level1_pty_wrapper_summary_text_precedes_child_output() {
    require_level!(Level::L1, pty_available(), "PTY (/dev/ptmx)");

    let fixture = CliProcessFixture::named("level1-pty-wrapper-summary");
    write_executable(
        &fixture.bin_dir().join("goose"),
        "#!/bin/sh\necho 'child-output'\n",
    );
    fixture.seed_user_config();

    // `expectrl` needs a live `std::process::Command`; the builder's raw
    // surface hands one over carrying the same policy. Only the staged stub may
    // count as an installed provider, so the badge row cannot be decided by a
    // real agentic CLI on the host.
    let mut cmd = fixture
        .command_builder()
        .fake_only_path()
        .build_std();
    cmd.args(["goose", "-y", "-n", "--", "hi"]);
    cmd.env("TERM_WIDTH", "80");

    let mut p = Session::spawn(cmd).expect("failed to spawn PTY");
    p.expect("Claudine").unwrap();
    p.expect("Goose").unwrap();
    p.expect("YOLO").unwrap();
    // `-n` records `INTERACTIVE=false` in the environment-variables
    // table the wrapper prints before delegating to the child binary.
    p.expect("INTERACTIVE").unwrap();
    p.expect("child-output").unwrap();
}

#[test]
fn level1_pty_non_interactive_detection() {
    require_level!(Level::L1, pty_available(), "PTY (/dev/ptmx)");

    let fixture = CliProcessFixture::named("level1-pty-wrapper-summary");
    write_executable(&fixture.bin_dir().join("goose"), "#!/bin/sh\nexit 0\n");
    fixture.seed_user_config();

    // Same fake-only rationale as above.
    let mut cmd = fixture
        .command_builder()
        .fake_only_path()
        .build_std();
    cmd.args(["goose", "--", "hi"]);
    cmd.env("TERM_WIDTH", "80");

    let mut p = Session::spawn(cmd).expect("failed to spawn PTY");
    p.expect("Claudine").unwrap();
    p.expect("Goose").unwrap();
}
