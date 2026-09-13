//! Level 1 PTY tests for provided-partial resolution through the shipped review router.
//!
//! From `fixes/2026-09-10-no-interactive-completion`: a supplied `spec=` partial
//! must be offered for completion *before* the router's first `initialize` guard
//! dereferences it with `frontmatter(spec, ...)`, and the accepted identity must
//! survive a proxy handoff. The fixture lives in `common::review_router`, which
//! seeds the shipped `prompts/review.md` router, its proxy target, spec
//! candidates under the `packages/example` launch area, and a same-substring
//! decoy at the repository root that only a mis-anchored candidate scope would
//! find.
//!
//! These tests prove ordering and data flow through manufactured bytes; what a
//! terminal emulator draws for the same flow is
//! `level2_provided_partial_file_capture.rs`'s job. The single/zero-match
//! confirmation flow this one builds on is
//! `level1_provided_partial_file_pty.rs`.
//!
//! ## Tier
//!
//! **Level 1**, and gating mirrors `level1_provided_partial_file_pty.rs`:
//! `#![cfg(unix)]` is the only exclusion, because `expectrl`'s `OsSession` is
//! Unix-only. On a selected platform `expect_level!(Level::L1, pty_available(),
//! ...)` **fails** when the PTY is missing rather than skipping: Level 1 is the
//! mandatory suite, where a skip is indistinguishable from a pass. `expectrl`
//! opens `/dev/ptmx` and the test manufactures every byte the child reads; no
//! terminal emulator participates.
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
use std::time::{Duration, Instant};
use test_toolkit::{Level, expect_level};

mod common;
use common::pty::*;
use common::pty_available;
use common::review_router::review_router_fixture;

fn review_router_session(
    fixture: &common::CliProcessFixture,
    router: &std::path::Path,
) -> OsSession {
    review_router_session_in(fixture, router, &common::review_router::launch_area(fixture))
}

/// Launch the router from `launch_dir`, which is the origin candidate discovery
/// must stay anchored to for the rest of the run.
///
/// `review_router_fixture` already seeded the user config under the fixture
/// home, and the builder's raw surface carries the spawn contract's `HOME`,
/// `PATH`, and rendezvous defaults — only the colour and TTY inputs the
/// confirmation dialog reads are set here.
fn review_router_session_in(
    fixture: &common::CliProcessFixture,
    router: &std::path::Path,
    launch_dir: &std::path::Path,
) -> OsSession {
    // `expectrl` needs a live `std::process::Command`; the builder's raw
    // surface hands one over carrying the same policy.
    let mut cmd = fixture
        .command_builder()
        .ambient_context(launch_dir)
        .build_std();
    cmd.args([
        "compose",
        "--goose",
        router.to_str().unwrap(),
        &format!("spec={}", common::review_router::PARTIAL),
    ]);
    cmd.env("TERM", "xterm-256color");
    cmd.env("COLORTERM", "truecolor");
    cmd.env_remove("NO_COLOR");
    cmd.env_remove("CLAUDINE_PLAIN");
    cmd.env_remove("CI");
    cmd.args(["-y", "token=retained"]);
    Session::spawn(cmd).expect("spawn headless review router PTY")
}

/// Assert the router run reached the provider with the selected identity and
/// asked exactly one question along the way.
///
/// `widget_dialogs` is how many `biscuit-tui` widgets the resolution legitimately
/// needs: the multi-candidate chooser runs through `run_standalone` and pushes
/// the keyboard-enhancement flags, while the single-candidate confirmation reads
/// its key with a bare `enable_raw_mode` and pushes nothing. Any extra prompt —
/// including one for a property this run never supplied — shows up as an extra
/// push.
fn assert_review_router_completed(
    session: &mut OsSession,
    fixture: &common::CliProcessFixture,
    mut transcript: String,
    selected: &str,
    directory: &str,
    widget_dialogs: usize,
) {
    transcript.push_str(&wait_for_marker(session, "provider reached", Duration::from_secs(20)));
    let prompt = fs::read_to_string(fixture.home().join("provider-prompt")).unwrap();
    assert!(prompt.contains(&format!("SELECTED={selected}")), "prompt: {prompt}");
    assert!(prompt.contains(directory), "prompt: {prompt}");
    assert!(prompt.contains("TOKEN=retained"), "prompt: {prompt}");
    let plain = common::strip_ansi(&transcript);
    assert_eq!(plain.matches("did not match a file directly").count(), 1, "{plain}");
    assert!(!plain.contains("lifecycle evaluation error"), "{plain}");
    // The shipped router requires `plan` and `review` beside `spec`, and only
    // `spec` was supplied. Missing-value collection for a `file` property
    // announces itself with this sentence (`schema_interactive::collect_file`),
    // so its absence is what proves the router's other inputs were never
    // collected ahead of `initialize`.
    assert!(
        !plain.contains("requires a valid file reference"),
        "an unrelated property was collected before initialize:\n{plain}"
    );
    assert_eq!(
        transcript.matches(common::pty::KBD_ENHANCEMENT_PUSH).count(),
        widget_dialogs,
        "unexpected number of interactive widgets:\n{plain}"
    );
}

fn wait_for_confirmation_input(session: &OsSession) {
    let deadline = Instant::now() + Duration::from_secs(5);
    // Confirmation flushes its text before enabling raw mode; disabled echo
    // proves the terminal is ready without depending on a fixed settle delay.
    while session.get_process().get_echo().expect("read PTY echo state") {
        assert!(Instant::now() < deadline, "confirmation never enabled raw mode");
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[test]
#[serial_test::serial(pty)]
fn level1_review_router_partial_yolo_confirms_before_initialize_and_survives_proxy() {
    expect_level!(Level::L1, pty_available(), "PTY (/dev/ptmx)");
    let (fixture, router) = review_router_fixture(false);
    let mut session = review_router_session(&fixture, &router);
    // Only the in-area spec is a candidate, so the flow must reach the
    // single-file confirmation; `wait_for_marker` panics if it does not. A scope
    // anchored anywhere but the launch origin picks the decoy up as a second
    // candidate and renders the chooser instead, which never prints this text.
    let transcript = wait_for_marker(&mut session, "Use this file", Duration::from_secs(15));
    // Belt-and-braces restatement: the chooser runs through `run_standalone`,
    // which pushes the keyboard-enhancement flags, while the confirmation
    // enables raw mode without them.
    assert!(
        !transcript.contains(common::pty::KBD_ENHANCEMENT_PUSH),
        "chooser rendered instead of the single-file confirmation, so a candidate \
         outside the launch area was discovered; transcript:\n{}",
        common::strip_ansi(&transcript)
    );
    assert!(!fixture.home().join("provider-prompt").exists());
    wait_for_confirmation_input(&session);
    session.write_all(b"y").unwrap();
    session.flush().unwrap();
    assert_review_router_completed(
        &mut session,
        &fixture,
        transcript,
        "alpha",
        "2026-09-10-local-a/spec.md",
        0,
    );
    let prompt = fs::read_to_string(fixture.home().join("provider-prompt")).unwrap();
    assert!(!prompt.contains("local-decoy"), "prompt: {prompt}");
}

/// Companion to the confirmation test above: the same fixture, launched from the
/// repository root instead of `packages/example`, must see both candidates.
///
/// Without this variant the decoy could be unreachable for a reason other than
/// launch-area anchoring (an ignore rule, a glob that never leaves the router's
/// directory), and the confirmation assertion would pass vacuously.
#[test]
#[serial_test::serial(pty)]
fn level1_review_router_partial_repo_root_launch_widens_candidates_to_the_chooser() {
    expect_level!(Level::L1, pty_available(), "PTY (/dev/ptmx)");
    let (fixture, router) = review_router_fixture(false);
    let repo_root = fixture.cwd().to_path_buf();
    let mut session = review_router_session_in(&fixture, &router, &repo_root);
    let pre = wait_for_marker(
        &mut session,
        "did not match a file directly",
        Duration::from_secs(15),
    );
    // Raw mode proves the chooser rendered, and the chooser is only reached with
    // more than one candidate. `fixes/` sorts before `packages/`, so Enter takes
    // the decoy — the candidate the package-area launch could not see.
    let transcript = wait_for_raw_mode(&mut session, pre, Duration::from_secs(10));
    assert!(!fixture.home().join("provider-prompt").exists());
    session.write_all(b"\r").unwrap();
    session.flush().unwrap();
    assert_review_router_completed(
        &mut session,
        &fixture,
        transcript,
        "decoy",
        "2026-09-10-local-decoy/spec.md",
        1,
    );
}

#[test]
#[serial_test::serial(pty)]
fn level1_review_router_partial_chooser_keeps_selected_identity_in_proxy() {
    expect_level!(Level::L1, pty_available(), "PTY (/dev/ptmx)");
    let (fixture, router) = review_router_fixture(true);
    let mut session = review_router_session(&fixture, &router);
    let transcript = wait_for_marker(&mut session, "did not match a file directly", Duration::from_secs(15));
    let transcript = wait_for_raw_mode(&mut session, transcript, Duration::from_secs(10));
    assert!(!fixture.home().join("provider-prompt").exists());
    session.write_all(b"\x1b[B\r").unwrap();
    session.flush().unwrap();
    assert_review_router_completed(
        &mut session,
        &fixture,
        transcript,
        "beta",
        "2026-09-10-local-b/spec.md",
        1,
    );
}

#[test]
#[serial_test::serial(pty)]
fn level1_review_router_partial_decline_and_cancel_stop_before_initialize() {
    expect_level!(Level::L1, pty_available(), "PTY (/dev/ptmx)");
    for multiple in [false, true] {
        let (fixture, router) = review_router_fixture(multiple);
        let mut session = review_router_session(&fixture, &router);
        let transcript = if multiple {
            let pre = wait_for_marker(&mut session, "did not match a file directly", Duration::from_secs(15));
            wait_for_raw_mode(&mut session, pre, Duration::from_secs(10))
        } else {
            let pre = wait_for_marker(&mut session, "Use this file", Duration::from_secs(15));
            wait_for_confirmation_input(&session);
            pre
        };
        session.write_all(if multiple { b"\x1b" } else { b"n" }).unwrap();
        session.flush().unwrap();
        let error = wait_for_marker(&mut session, "no existing file matched reference", Duration::from_secs(10));
        let plain = common::strip_ansi(&(transcript + &error));
        assert!(!plain.contains("lifecycle evaluation error"), "{plain}");
        assert!(!fixture.home().join("provider-prompt").exists(), "{plain}");
    }
}

#[test]
#[serial_test::serial(pty)]
fn level1_proxy_target_schema_resolves_partial_once_before_its_initialize() {
    expect_level!(Level::L1, pty_available(), "PTY (/dev/ptmx)");
    let (fixture, _) = review_router_fixture(false);
    let entry = fixture.cwd().join("prompts/entry.md");
    common::write(
        &entry,
        "---\ninitialize:\n  stack:\n    - action:\n        - proxy: ./review.md\n---\nEntry without a schema.\n",
    );
    let mut session = review_router_session(&fixture, &entry);
    let transcript = wait_for_marker(&mut session, "Use this file", Duration::from_secs(15));
    assert!(!fixture.home().join("provider-prompt").exists());
    wait_for_confirmation_input(&session);
    session.write_all(b"y").unwrap();
    session.flush().unwrap();
    assert_review_router_completed(
        &mut session,
        &fixture,
        transcript,
        "alpha",
        "2026-09-10-local-a/spec.md",
        0,
    );
}
