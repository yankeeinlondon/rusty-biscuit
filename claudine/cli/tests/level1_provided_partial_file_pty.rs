//! Level 1 PTY tests for provided-partial `file`/`file[]` resolution.
//!
//! Phase 3 of `fixes/2026-06-30-completion-failures`. When a `compose`
//! invocation supplies a value for a `file`/`file[]` schema property that
//! does not resolve to a literal path, Claudine treats the value as a
//! **partial**: it walks the property's `match(...)` glob from the launch
//! area, filters candidates by the provided substring (case-insensitive),
//! and — finding exactly one — shows a confirmation dialog. On `y`,
//! composition proceeds with the resolved path.
//!
//! These tests drive that flow through a pseudo-terminal:
//!
//! - A single glob+substring match reaches the `Use this file? (Y/n)`
//!   confirmation dialog and, on `y`, launches the provider stub.
//! - Zero glob+substring matches preserve the original
//!   `no existing file matched reference` error and never launch the
//!   provider.
//! - Scalar string values for `file[]` properties are normalized to a
//!   single-element array before resolution.
//!
//! ## Tier
//!
//! **Level 1**, and gating mirrors `level1_schema_prompt_pty.rs`:
//! `#![cfg(unix)]` is the only exclusion, because `expectrl`'s `OsSession` is
//! Unix-only. On a selected platform
//! `expect_level!(Level::L1, pty_available(), ...)` **fails** when the PTY is
//! missing rather than skipping: Level 1 is the mandatory suite, where a skip
//! is indistinguishable from a pass. `expectrl` opens `/dev/ptmx` and the test
//! manufactures every byte the child reads; no terminal emulator participates.
//!
//! Run via the canonical recipe:
//!
//! ```text
//! just test
//! ```

#![cfg(unix)]

use expectrl::Session;
use expectrl::session::OsSession;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};
use test_toolkit::{Level, expect_level};

mod common;
use common::pty::*;
use common::{CliProcessFixture, pty_available};

/// Seed a workspace whose only `**/*spec*.md` files are two specs, exactly
/// one of which carries `everywhere` in its path.
fn seed_specs(root: &Path) {
    let target = root.join("features/2026-06-30-style-everywhere/spec.md");
    fs::create_dir_all(target.parent().unwrap()).unwrap();
    fs::write(&target, "---\ntitle: Everywhere\n---\nSpec body.\n").unwrap();
    let other = root.join("features/2026-06-01-other/spec.md");
    fs::create_dir_all(other.parent().unwrap()).unwrap();
    fs::write(&other, "---\ntitle: Other\n---\nSpec body.\n").unwrap();
}

/// Build a `claudine compose --goose <plan> <property>=<partial>` command
/// anchored at the fixture's launch area (the directory the glob walks) with
/// `HOME` inside the fixture so `prompt_for_missing` reads its default (`true`).
fn compose_command(
    fixture: &CliProcessFixture,
    md_file: &Path,
    property: &str,
    partial: &str,
) -> Command {
    stage_default_config(fixture.home());
    // `expectrl` needs a live `std::process::Command`; the builder's raw
    // surface hands one over carrying the same policy.
    let mut cmd = fixture.command_std();
    cmd.args([
        "compose",
        "--goose",
        md_file.to_str().unwrap(),
        &format!("{property}={partial}"),
    ]);
    cmd.env("TERM", "xterm-256color");
    cmd.env("COLORTERM", "truecolor");
    cmd.env_remove("NO_COLOR");
    cmd.env_remove("CLAUDINE_PLAIN");
    cmd.env_remove("CI");
    cmd
}

fn plan_with_file_schema(root: &Path) -> PathBuf {
    let md_file = root.join("plan.md");
    fs::write(
        &md_file,
        concat!(
            "---\n",
            "$schema:\n",
            "  spec: 'file(required;match(**/*spec*.md);eager)'\n",
            "---\n",
            "Plan body.\n",
        ),
    )
    .unwrap();
    md_file
}

fn plan_with_file_array_schema(root: &Path) -> PathBuf {
    let md_file = root.join("plan.md");
    fs::write(
        &md_file,
        concat!(
            "---\n",
            "$schema:\n",
            "  attachments: 'file(required;match(**/*spec*.md);eager)[]'\n",
            "---\n",
            "Plan body.\n",
        ),
    )
    .unwrap();
    md_file
}

/// A fixture with the two specs seeded, a `goose` stub staged, and the marker
/// path the stub writes when it is launched.
fn staged_fixture() -> (CliProcessFixture, PathBuf) {
    let fixture = CliProcessFixture::named("level1-provided-partial-pty");
    let marker = fixture.cwd().join("launched.flag");
    stage_goose_stub(fixture.bin_dir(), &marker);
    seed_specs(fixture.cwd());
    (fixture, marker)
}

/// Answer the `Use this file? (Y/n)` dialog with `y` and drain until the stub
/// records its launch, returning the accumulated transcript.
///
/// `confirm_one_file` enables raw mode via crossterm directly (not the
/// `run_standalone` path), so it emits no raw-mode marker byte — nothing at all
/// between flushing the dialog and blocking on the key read. The line
/// discipline is the observable condition instead; see
/// [`wait_for_raw_mode_termios`].
fn confirm_and_drain(session: &mut OsSession, marker: &Path, seed: String) -> String {
    wait_for_raw_mode_termios(session, Duration::from_secs(10));
    session.write_all(b"y").expect("confirm file selection");
    session.flush().ok();

    let stop = Instant::now() + Duration::from_secs(15);
    let mut transcript = seed;
    while Instant::now() < stop {
        if marker.exists() {
            break;
        }
        transcript.push_str(&read_for(session, Duration::from_millis(200)));
    }
    transcript
}

#[test]
fn level1_pty_provided_partial_single_match_confirms_and_launches() {
    expect_level!(Level::L1, pty_available(), "PTY (/dev/ptmx)");

    let (fixture, marker) = staged_fixture();
    let md_file = plan_with_file_schema(fixture.cwd());

    let cmd = compose_command(&fixture, &md_file, "spec", "everywhere");
    let mut session: OsSession = Session::spawn(cmd).expect("spawn PTY session");

    // A single glob+substring match drives the confirmation dialog, whose
    // trailer is `Use this file? (Y/n)`.
    let pre = wait_for_marker(&mut session, "Use this file", Duration::from_secs(10));

    // The provider must NOT have launched before the dialog is answered.
    assert!(
        !marker.exists(),
        "provider launched before the confirmation dialog was answered; \
         transcript so far:\n{}",
        common::strip_ansi(&pre)
    );

    let transcript = confirm_and_drain(&mut session, &marker, pre);

    assert!(
        marker.exists(),
        "provider stub should have launched after the confirmation dialog \
         resolved `everywhere` to the one matching spec.\ntranscript:\n{}",
        common::strip_ansi(&transcript)
    );
}

#[test]
fn level1_pty_provided_partial_zero_match_preserves_error() {
    expect_level!(Level::L1, pty_available(), "PTY (/dev/ptmx)");

    let (fixture, marker) = staged_fixture();
    let md_file = plan_with_file_schema(fixture.cwd());

    // No spec path contains `no-such-partial`, so the glob+substring filter
    // yields zero candidates and the original error is preserved unchanged.
    let cmd = compose_command(&fixture, &md_file, "spec", "no-such-partial");
    let mut session: OsSession = Session::spawn(cmd).expect("spawn PTY session");

    let transcript = read_for(&mut session, Duration::from_secs(8));
    let plain = common::strip_ansi(&transcript);

    assert!(
        !marker.exists(),
        "provider must NOT launch when the partial matches no candidate; \
         transcript:\n{plain}"
    );
    assert!(
        plain.contains("no existing file matched reference"),
        "expected the original file-reference failure text to be preserved; \
         transcript:\n{plain}"
    );
}

#[test]
fn level1_pty_provided_partial_file_array_scalar_confirms_and_launches() {
    expect_level!(Level::L1, pty_available(), "PTY (/dev/ptmx)");

    let (fixture, marker) = staged_fixture();
    let md_file = plan_with_file_array_schema(fixture.cwd());

    // A scalar string provided for a `file[]` property is normalized to a
    // single-element array and treated as a partial.
    let cmd = compose_command(&fixture, &md_file, "attachments", "everywhere");
    let mut session: OsSession = Session::spawn(cmd).expect("spawn PTY session");

    let pre = wait_for_marker(&mut session, "Use this file", Duration::from_secs(10));

    assert!(
        !marker.exists(),
        "provider launched before the confirmation dialog was answered; \
         transcript so far:\n{}",
        common::strip_ansi(&pre)
    );

    let transcript = confirm_and_drain(&mut session, &marker, pre);

    assert!(
        marker.exists(),
        "provider stub should have launched after the file[] confirmation dialog \
         resolved `everywhere`.\ntranscript:\n{}",
        common::strip_ansi(&transcript)
    );
}

#[test]
fn level1_pty_provided_partial_file_array_array_confirms_and_launches() {
    expect_level!(Level::L1, pty_available(), "PTY (/dev/ptmx)");

    let (fixture, marker) = staged_fixture();
    let md_file = plan_with_file_array_schema(fixture.cwd());

    // An explicit JSON array value is also accepted as a `file[]` partial.
    let cmd = compose_command(&fixture, &md_file, "attachments", "[\"everywhere\"]");
    let mut session: OsSession = Session::spawn(cmd).expect("spawn PTY session");

    let pre = wait_for_marker(&mut session, "Use this file", Duration::from_secs(10));

    assert!(
        !marker.exists(),
        "provider launched before the confirmation dialog was answered; \
         transcript so far:\n{}",
        common::strip_ansi(&pre)
    );

    let transcript = confirm_and_drain(&mut session, &marker, pre);

    assert!(
        marker.exists(),
        "provider stub should have launched after the file[] confirmation dialog \
         resolved `[\"everywhere\"]`.\ntranscript:\n{}",
        common::strip_ansi(&transcript)
    );
}
