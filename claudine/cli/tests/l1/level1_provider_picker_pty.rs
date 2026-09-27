//! Level 1 PTY test for the one-shot provider picker (R5, R7.7 of
//! `2026-09-27-union-partial-file-completion`).
//!
//! With no provider flag, no `agent` frontmatter, and two runnable providers
//! on the fixture `PATH`, `compose` under a TTY opens the provider picker. The
//! picker must run in a bounded inline viewport, so the capture never contains
//! the alternate-screen enter sequence, and the provider the user picks must
//! be the one launched.
//!
//! ## Tier
//!
//! **Level 1**, gated like `level1_schema_prompt_pty.rs`: `#![cfg(unix)]` is
//! the only exclusion, because `expectrl`'s `OsSession` is Unix-only, and
//! `expect_level!(Level::L1, pty_available(), ...)` **fails** when the PTY is
//! missing. `expectrl` opens `/dev/ptmx` and the test manufactures every byte
//! the child reads; no terminal emulator participates.

use expectrl::Session;
use expectrl::session::OsSession;
use std::fs;
use std::io::Write;
use std::time::{Duration, Instant};
use test_toolkit::{Level, expect_level};

use crate::common;
use common::pty::*;
use common::{CliProcessFixture, pty_available, write_executable};

#[test]
fn level1_pty_provider_picker_is_inline_and_launches_selection() {
    expect_level!(Level::L1, pty_available(), "PTY (/dev/ptmx)");

    let fixture = CliProcessFixture::named("level1-provider-picker-pty");
    stage_default_config(fixture.home());
    let goose_marker = fixture.cwd().join("goose.flag");
    let claude_marker = fixture.cwd().join("claude.flag");
    stage_goose_stub(fixture.bin_dir(), &goose_marker);
    // Claude sorts before Goose in the picker, so it is the default
    // selection; choosing Goose proves the keystroke, not the default,
    // picked the launched provider.
    write_executable(
        &fixture.bin_dir().join("claude"),
        &format!(
            "#!/bin/sh\necho 'launched' > {marker}\nexit 0\n",
            marker = claude_marker.display()
        ),
    );
    let md_file = fixture.cwd().join("plan.md");
    fs::write(&md_file, "Plan body.\n").unwrap();

    // `expectrl` needs a live `std::process::Command`; the builder's raw
    // surface hands one over carrying the same policy.
    let mut cmd = fixture.command_std();
    cmd.args(["compose", md_file.to_str().unwrap()]);
    cmd.env("TERM", "xterm-256color");
    cmd.env("COLORTERM", "truecolor");
    cmd.env_remove("NO_COLOR");
    cmd.env_remove("CLAUDINE_PLAIN");
    cmd.env_remove("CI");
    let mut session: OsSession = Session::spawn(cmd).expect("spawn PTY session");

    let pre = wait_for_marker(&mut session, "Goose", Duration::from_secs(10));
    let pre = wait_for_raw_mode(&mut session, pre, Duration::from_secs(10));
    assert!(
        !goose_marker.exists() && !claude_marker.exists(),
        "a provider launched before the picker was answered; transcript so \
         far:\n{}",
        common::strip_ansi(&pre)
    );

    // `j` moves the highlight from the default (Claude) to Goose; `\r` is
    // Enter once raw mode has disabled `ICRNL`.
    session.write_all(b"j\r").expect("select Goose");
    session.flush().ok();

    let stop = Instant::now() + Duration::from_secs(15);
    let mut transcript = pre;
    while Instant::now() < stop && !goose_marker.exists() {
        transcript.push_str(&read_for(&mut session, Duration::from_millis(200)));
    }
    let plain = common::strip_ansi(&transcript);

    assert!(
        !transcript.contains(ALT_SCREEN_ENTER),
        "the provider picker must render inline, not on the alternate \
         screen; transcript:\n{plain}"
    );
    assert!(
        goose_marker.exists(),
        "selecting Goose should launch the goose stub; transcript:\n{plain}"
    );
    assert!(
        !claude_marker.exists(),
        "the unselected default provider must not launch; transcript:\n{plain}"
    );
}
