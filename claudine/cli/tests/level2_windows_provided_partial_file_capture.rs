//! Level 2 real-terminal capture for provided-partial resolution ahead of
//! `initialize` — native Windows.
//!
//! The Windows twin of `level2_provided_partial_file_capture.rs`. Same claim,
//! same shipped `prompts/review.md` router, same partial: inside a real
//! WezTerm pane the `Use this file? (Y/n)` confirmation and its styled
//! candidate card are drawn before the router's `initialize` guard can
//! dereference the partial, and accepting through the emulator's key path
//! carries the selected spec across the proxy hop to the provider. Read the
//! Unix file first; this one documents only what differs.
//!
//! ## Why the pane runs `cmd.exe` directly rather than `spawn_shell`
//!
//! `TerminalHarness::spawn_shell` resolves a POSIX login shell and joins
//! `PATH` with `:`, both Unix-shaped. The pane is spawned with
//! `spawn_program("cmd.exe")` and driven with raw `send_text`, as
//! `level3_windows_sequence_ctrl_c.rs` does, which also keeps the fixture in
//! the shell Claudine itself uses on Windows.
//!
//! ## Why the exit marker is typed as `%M%` rather than its value
//!
//! The console echoes each typed line, so a trailer spelled
//! `echo PARTIAL_1:0 || echo PARTIAL_1:1` would put the marker on screen —
//! with a status digit after it — before the command has run. Setting `M`
//! first and typing `echo %M%:0` keeps the literal off the echoed line; only
//! the expanded output starts with the marker.
//!
//! ## Provider fixture
//!
//! `common::review_router` compiles a native `goose.exe` here: a `.cmd` cannot
//! receive the multi-line prompt the proxy target renders.
//!
//! ## Where this runs
//!
//! Locally on the Windows build host, against a headless
//! `wezterm-mux-server` (started once with `Start-Process`, addressed through
//! `WEZTERM_UNIX_SOCKET`). CI has no Windows Level 2 leg — a temporary
//! provisioning gap recorded in `.github/ci/environments.json`, not a reason
//! to exclude the platform.

#![cfg(windows)]

use std::fs;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use biscuit_test_harness::wezterm::WezTermHarness;
use biscuit_test_harness::TerminalHarness;
use serial_test::serial;
use test_toolkit::{Backend, Level, require_level};

mod common;
use common::review_router::{
    PARTIAL, assert_provider_received, launch_area, names_selected_spec, provider_prompt,
    review_router_fixture,
};
use common::{
    assert_row_is_styled, claudine_bin, minimal_system_path, wait_for_exit_marker,
    wait_for_pane_text,
};

const CONFIRMATION_PROMPT: &str = "Use this file? (Y/n)";
const PARTIAL_NOTICE: &str = "did not match a file directly";
const CANDIDATE_BADGE: &str = " FILE ";
const LIFECYCLE_ERROR: &str = "lifecycle evaluation error";
const SELECTED_SPEC: &str = "2026-09-10-local-a/spec.md";

/// Types one line into the pane. `cmd.exe` needs CRLF to accept it.
fn send_line(harness: &mut WezTermHarness, line: &str) {
    harness
        .send_text(format!("{line}\r\n").as_bytes())
        .unwrap_or_else(|error| panic!("send {line:?} to cmd pane: {error}"));
}

#[test]
#[serial(level2_terminal)]
fn level2_wezterm_windows_review_router_partial_confirms_before_initialize() {
    require_level!(Level::L2, WezTermHarness::available(), Backend::WezTerm);

    let (fixture, router) = review_router_fixture(false);
    let home = fixture.home().display().to_string();
    let path = std::env::join_paths(
        std::iter::once(fixture.bin_dir().to_path_buf()).chain(minimal_system_path()),
    )
    .expect("join fixture PATH")
    .to_string_lossy()
    .into_owned();

    let mut harness = WezTermHarness::new();
    harness
        .spawn_program("cmd.exe", &[])
        .expect("spawn WezTerm cmd.exe pane");
    // `spawn_program` performs no prompt-readiness wait, so settle before typing.
    std::thread::sleep(Duration::from_secs(1));

    static NEXT_CAPTURE: AtomicUsize = AtomicUsize::new(0);
    let marker = format!(
        "PARTIAL_{}_{}",
        std::process::id(),
        NEXT_CAPTURE.fetch_add(1, Ordering::Relaxed)
    );

    // `/d` changes drive as well as directory: the fixture lives on the
    // system drive, the pane may open on another.
    send_line(&mut harness, &format!("cd /d \"{}\"", launch_area(&fixture).display()));
    send_line(&mut harness, &format!("set PATH={path}"));
    // The same home quartet the L1 command builder sets, so Claudine's
    // environment-first home resolution lands on the fixture.
    for variable in ["HOME", "USERPROFILE", "APPDATA", "LOCALAPPDATA"] {
        send_line(&mut harness, &format!("set {variable}={home}"));
    }
    send_line(&mut harness, "set NO_COLOR=");
    send_line(&mut harness, "set TERM=xterm-256color");
    send_line(&mut harness, "set COLORTERM=truecolor");
    send_line(&mut harness, "set FORCE_COLOR=1");
    send_line(&mut harness, "set CLAUDINE_RENDEZVOUS_REPORT=false");
    send_line(&mut harness, &format!("set M={marker}"));
    send_line(&mut harness, "cls");
    send_line(
        &mut harness,
        &format!(
            "\"{}\" compose --goose \"{}\" spec={PARTIAL} -y token=retained && echo %M%:0 || echo %M%:1",
            claudine_bin(),
            router.display(),
        ),
    );

    let dialog = wait_for_pane_text(&mut harness, CONFIRMATION_PROMPT, Duration::from_secs(30));
    assert!(
        !provider_prompt(&fixture).exists(),
        "provider was reached before the confirmation was drawn; plain:\n{}",
        dialog.plain
    );
    assert!(
        !dialog.plain.contains(LIFECYCLE_ERROR),
        "initialize dereferenced the partial before the confirmation; plain:\n{}",
        dialog.plain
    );
    assert!(
        !dialog.plain.contains(&format!("{marker}:")),
        "command exited before the confirmation was answered; plain:\n{}",
        dialog.plain
    );
    assert!(
        dialog.plain.contains(PARTIAL_NOTICE),
        "the partial notice must precede the confirmation; plain:\n{}",
        dialog.plain
    );
    // The candidate path is longer than the pane is wide and wraps wherever
    // the emulator's column count puts the break, so match it unwrapped; on
    // this platform it is drawn in native spelling.
    let unwrapped: String = dialog.plain.lines().collect();
    assert!(
        names_selected_spec(&unwrapped, SELECTED_SPEC) && !unwrapped.contains("local-decoy"),
        "the confirmation must name the launch-area candidate only; plain:\n{}",
        dialog.plain
    );
    eprintln!("RAWDUMP {:?}", dialog.raw);
    assert_row_is_styled(&dialog.raw, CANDIDATE_BADGE, "candidate file badge");

    harness.send_text(b"y").expect("accept the candidate");

    let (done, status) = wait_for_exit_marker(&mut harness, &marker, Duration::from_secs(30));
    assert_eq!(status, "0", "review router failed after acceptance; plain:\n{}", done.plain);
    assert!(
        done.plain.contains("provider reached"),
        "proxy target never reached the provider stub; plain:\n{}",
        done.plain
    );
    assert!(!done.plain.contains(LIFECYCLE_ERROR), "plain:\n{}", done.plain);
    assert!(
        !done.plain.contains("requires a valid file reference"),
        "an unrelated property was collected before initialize; plain:\n{}",
        done.plain
    );
    let prompt = fs::read_to_string(provider_prompt(&fixture))
        .expect("provider stub recorded its prompt");
    assert_provider_received(&prompt, "alpha", SELECTED_SPEC);

    // `harness` Drop kills the pane; no explicit teardown needed.
}
