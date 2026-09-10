//! Level 2 PTY tests for provided-partial `file`/`file[]` resolution.
//!
//! Phase 3 of `fixes/2026-06-30-completion-failures`. When a `compose`
//! invocation supplies a value for a `file`/`file[]` schema property that
//! does not resolve to a literal path, Claudine treats the value as a
//! **partial**: it walks the property's `match(...)` glob from the launch
//! area, filters candidates by the provided substring (case-insensitive),
//! and — finding exactly one — shows a confirmation dialog. On `y`,
//! composition proceeds with the resolved path.
//!
//! These tests drive that flow through a real pseudo-terminal:
//!
//! - A single glob+substring match reaches the `Use this file? (Y/n)`
//!   confirmation dialog and, on `y`, launches the provider stub.
//! - Zero glob+substring matches preserve the original
//!   `no existing file matched reference` error and never launch the
//!   provider.
//! - Scalar string values for `file[]` properties are normalized to a
//!   single-element array before resolution.
//!
//! Gating mirrors `level2_schema_prompt_pty.rs`: `#![cfg(unix)]` plus
//! `require_level!(Level::L2, pty_available(), ...)` so the test skips
//! cleanly without a PTY.
//!
//! Run via the canonical recipe:
//!
//! ```text
//! just test-l2
//! ```

#![cfg(unix)]

#[allow(deprecated)]
use assert_cmd::cargo::cargo_bin;
use expectrl::Session;
use expectrl::session::OsSession;
use std::fs;
use std::io::Write;
use std::process::Command;
use std::time::{Duration, Instant};
use tempfile::tempdir;
use test_toolkit::{Level, require_level};

mod common;
use common::pty::*;
use common::{augmented_path, pty_available};

/// Seed a workspace whose only `**/*spec*.md` files are two specs, exactly
/// one of which carries `everywhere` in its path.
fn seed_specs(root: &std::path::Path) {
    let target = root.join("features/2026-06-30-style-everywhere/spec.md");
    fs::create_dir_all(target.parent().unwrap()).unwrap();
    fs::write(&target, "---\ntitle: Everywhere\n---\nSpec body.\n").unwrap();
    let other = root.join("features/2026-06-01-other/spec.md");
    fs::create_dir_all(other.parent().unwrap()).unwrap();
    fs::write(&other, "---\ntitle: Other\n---\nSpec body.\n").unwrap();
}

/// Build a `claudine compose --goose <plan> <property>=<partial>` command
/// anchored at `workspace_dir` (the launch area the glob walks) with HOME set
/// to the workspace so `prompt_for_missing` reads its default (`true`).
fn compose_command(
    workspace_dir: &std::path::Path,
    bin_dir: &std::path::Path,
    md_file: &std::path::Path,
    property: &str,
    partial: &str,
) -> Command {
    stage_default_config(workspace_dir);
    let mut cmd = Command::new(cargo_bin("claudine"));
    cmd.args([
        "compose",
        "--goose",
        md_file.to_str().unwrap(),
        &format!("{property}={partial}"),
    ]);
    cmd.env("HOME", workspace_dir);
    cmd.env("PATH", augmented_path(bin_dir));
    cmd.env("TERM", "xterm-256color");
    cmd.env("COLORTERM", "truecolor");
    cmd.env_remove("NO_COLOR");
    cmd.env_remove("CLAUDINE_PLAIN");
    cmd.env_remove("CI");
    cmd.current_dir(workspace_dir);
    cmd
}

fn plan_with_file_schema(root: &std::path::Path) -> std::path::PathBuf {
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

fn plan_with_file_array_schema(root: &std::path::Path) -> std::path::PathBuf {
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

#[test]
#[serial_test::serial(pty)]
fn level2_pty_provided_partial_single_match_confirms_and_launches() {
    require_level!(Level::L2, pty_available(), "PTY (/dev/ptmx)");

    let workspace = tempdir().unwrap();
    let bin_dir = workspace.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();
    let marker = workspace.path().join("launched.flag");

    stage_goose_stub(&bin_dir, &marker);
    seed_specs(workspace.path());
    let md_file = plan_with_file_schema(workspace.path());

    let cmd = compose_command(workspace.path(), &bin_dir, &md_file, "spec", "everywhere");
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

    // `confirm_one_file` enables raw mode via crossterm directly (not the
    // `run_standalone` path), so there is no kitty-protocol raw-mode marker to
    // wait on. Raw mode is enabled synchronously right after the dialog flushes;
    // a brief settle guards against sending the key before the read loop starts.
    std::thread::sleep(Duration::from_millis(300));

    // `y` confirms; the resolved path override satisfies the schema and the
    // stub launches.
    session.write_all(b"y").expect("confirm file selection");
    session.flush().ok();

    let stop = Instant::now() + Duration::from_secs(15);
    let mut transcript = pre;
    while Instant::now() < stop {
        if marker.exists() {
            break;
        }
        transcript.push_str(&read_for(&mut session, Duration::from_millis(200)));
    }

    assert!(
        marker.exists(),
        "provider stub should have launched after the confirmation dialog \
         resolved `everywhere` to the one matching spec.\ntranscript:\n{}",
        common::strip_ansi(&transcript)
    );
}

#[test]
#[serial_test::serial(pty)]
fn level2_pty_provided_partial_zero_match_preserves_error() {
    require_level!(Level::L2, pty_available(), "PTY (/dev/ptmx)");

    let workspace = tempdir().unwrap();
    let bin_dir = workspace.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();
    let marker = workspace.path().join("launched.flag");

    stage_goose_stub(&bin_dir, &marker);
    seed_specs(workspace.path());
    let md_file = plan_with_file_schema(workspace.path());

    // No spec path contains `no-such-partial`, so the glob+substring filter
    // yields zero candidates and the original error is preserved unchanged.
    let cmd = compose_command(workspace.path(), &bin_dir, &md_file, "spec", "no-such-partial");
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
#[serial_test::serial(pty)]
fn level2_pty_provided_partial_file_array_scalar_confirms_and_launches() {
    require_level!(Level::L2, pty_available(), "PTY (/dev/ptmx)");

    let workspace = tempdir().unwrap();
    let bin_dir = workspace.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();
    let marker = workspace.path().join("launched.flag");

    stage_goose_stub(&bin_dir, &marker);
    seed_specs(workspace.path());
    let md_file = plan_with_file_array_schema(workspace.path());

    // A scalar string provided for a `file[]` property is normalized to a
    // single-element array and treated as a partial.
    let cmd = compose_command(workspace.path(), &bin_dir, &md_file, "attachments", "everywhere");
    let mut session: OsSession = Session::spawn(cmd).expect("spawn PTY session");

    let pre = wait_for_marker(&mut session, "Use this file", Duration::from_secs(10));

    assert!(
        !marker.exists(),
        "provider launched before the confirmation dialog was answered; \
         transcript so far:\n{}",
        common::strip_ansi(&pre)
    );

    std::thread::sleep(Duration::from_millis(300));
    session.write_all(b"y").expect("confirm file selection");
    session.flush().ok();

    let stop = Instant::now() + Duration::from_secs(15);
    let mut transcript = pre;
    while Instant::now() < stop {
        if marker.exists() {
            break;
        }
        transcript.push_str(&read_for(&mut session, Duration::from_millis(200)));
    }

    assert!(
        marker.exists(),
        "provider stub should have launched after the file[] confirmation dialog \
         resolved `everywhere`.\ntranscript:\n{}",
        common::strip_ansi(&transcript)
    );
}

#[test]
#[serial_test::serial(pty)]
fn level2_pty_provided_partial_file_array_array_confirms_and_launches() {
    require_level!(Level::L2, pty_available(), "PTY (/dev/ptmx)");

    let workspace = tempdir().unwrap();
    let bin_dir = workspace.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();
    let marker = workspace.path().join("launched.flag");

    stage_goose_stub(&bin_dir, &marker);
    seed_specs(workspace.path());
    let md_file = plan_with_file_array_schema(workspace.path());

    // An explicit JSON array value is also accepted as a `file[]` partial.
    let cmd = compose_command(
        workspace.path(),
        &bin_dir,
        &md_file,
        "attachments",
        "[\"everywhere\"]",
    );
    let mut session: OsSession = Session::spawn(cmd).expect("spawn PTY session");

    let pre = wait_for_marker(&mut session, "Use this file", Duration::from_secs(10));

    assert!(
        !marker.exists(),
        "provider launched before the confirmation dialog was answered; \
         transcript so far:\n{}",
        common::strip_ansi(&pre)
    );

    std::thread::sleep(Duration::from_millis(300));
    session.write_all(b"y").expect("confirm file selection");
    session.flush().ok();

    let stop = Instant::now() + Duration::from_secs(15);
    let mut transcript = pre;
    while Instant::now() < stop {
        if marker.exists() {
            break;
        }
        transcript.push_str(&read_for(&mut session, Duration::from_millis(200)));
    }

    assert!(
        marker.exists(),
        "provider stub should have launched after the file[] confirmation dialog \
         resolved `[\"everywhere\"]`.\ntranscript:\n{}",
        common::strip_ansi(&transcript)
    );
}

fn review_router_fixture(multiple: bool) -> (common::CliProcessFixture, std::path::PathBuf) {
    let fixture = common::CliProcessFixture::named("review-router-partial");
    fixture.initialize_repository();
    fixture.seed_user_config();
    let router = fixture.cwd().join("prompts/review.md");
    common::write(&router, include_str!("../../../prompts/review.md"));
    common::write(
        &fixture.cwd().join("prompts/_reviews/feature-review.md"),
        "---\n$schema:\n  spec: file(required;eager;match(**/*spec*.md))\nselected: \"{{ frontmatter(spec, 'marker') }}\"\n---\nSELECTED={{ selected }}\nSPEC={{ spec }}\nTOKEN={{ token }}\n",
    );
    common::write(
        &fixture.cwd().join("packages/example/fixes/2026-09-10-local-a/spec.md"),
        "---\nreviewed: true\nmarker: alpha\n---\nAlpha specification.\n",
    );
    if multiple {
        common::write(
            &fixture.cwd().join("packages/example/fixes/2026-09-10-local-b/spec.md"),
            "---\nreviewed: true\nmarker: beta\n---\nBeta specification.\n",
        );
    }
    common::write_executable(
        &fixture.bin_dir().join("goose"),
        "#!/bin/sh\nprintf '%s\\n' \"$*\" > \"$HOME/provider-prompt\"\nprintf 'provider reached\\n'\n",
    );
    (fixture, router)
}

fn review_router_session(
    fixture: &common::CliProcessFixture,
    router: &std::path::Path,
) -> OsSession {
    let package = fixture.cwd().join("packages/example");
    let mut cmd = compose_command(
        &package,
        fixture.bin_dir(),
        router,
        "spec",
        "fixes/2026-09-10-local",
    );
    let paths = std::iter::once(fixture.bin_dir().to_path_buf())
        .chain(common::minimal_system_path());
    cmd.env("PATH", std::env::join_paths(paths).unwrap());
    cmd.env("HOME", fixture.home());
    cmd.env("CLAUDINE_RENDEZVOUS_REPORT", "false");
    cmd.args(["-y", "token=retained"]);
    Session::spawn(cmd).expect("spawn headless review router PTY")
}

fn assert_review_router_completed(
    session: &mut OsSession,
    fixture: &common::CliProcessFixture,
    mut transcript: String,
    selected: &str,
    directory: &str,
) {
    transcript.push_str(&wait_for_marker(session, "provider reached", Duration::from_secs(20)));
    let prompt = fs::read_to_string(fixture.home().join("provider-prompt")).unwrap();
    assert!(prompt.contains(&format!("SELECTED={selected}")), "prompt: {prompt}");
    assert!(prompt.contains(directory), "prompt: {prompt}");
    assert!(prompt.contains("TOKEN=retained"), "prompt: {prompt}");
    let plain = common::strip_ansi(&transcript);
    assert_eq!(plain.matches("did not match a file directly").count(), 1, "{plain}");
    assert!(!plain.contains("lifecycle evaluation error"), "{plain}");
    assert!(!plain.contains("MissingProperties"), "{plain}");
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
fn level2_review_router_partial_yolo_confirms_before_initialize_and_survives_proxy() {
    require_level!(Level::L2, pty_available(), "PTY (/dev/ptmx)");
    let (fixture, router) = review_router_fixture(false);
    let mut session = review_router_session(&fixture, &router);
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
    );
}

#[test]
#[serial_test::serial(pty)]
fn level2_review_router_partial_chooser_keeps_selected_identity_in_proxy() {
    require_level!(Level::L2, pty_available(), "PTY (/dev/ptmx)");
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
    );
}

#[test]
#[serial_test::serial(pty)]
fn level2_review_router_partial_decline_and_cancel_stop_before_initialize() {
    require_level!(Level::L2, pty_available(), "PTY (/dev/ptmx)");
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
fn level2_proxy_target_schema_resolves_partial_once_before_its_initialize() {
    require_level!(Level::L2, pty_available(), "PTY (/dev/ptmx)");
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
    );
}
