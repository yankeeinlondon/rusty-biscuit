//! Level 2 real-Claudine coverage for **AC2** of
//! `darkmatter/features/2026-09-09-more-context`: under a real `claudine
//! compose`, `ctx.worktree` equals the launch directory's basename when the
//! document is composed from a *linked* git worktree, and is `null` when it is
//! composed from the main checkout.
//!
//! ## Why this is Level 2
//!
//! AC2 is written against a real Claudine run. The evidence path — Claudine's
//! `InvocationContext` launch capture, its `GitInfo::current_worktree`, and
//! Darkmatter's `Git` context group — is only assembled end to end by the
//! binary. `claudine/lib/src/invocation_context/tests.rs::linked_worktrees_keep_distinct_repository_keys`
//! proves the repository *keys* stay distinct in process; it does not prove
//! that a composed document sees the worktree name.
//!
//! ## Why the assertion reads YAML rather than the composed body
//!
//! The expression language has no `null` literal, so `{{ ctx.worktree }}`
//! renders the empty string for both "not a linked worktree" and "a linked
//! worktree whose name is empty" — an assertion on the rendered body alone
//! cannot tell AC2's two cases apart. A whole-value `set_frontmatter` argument
//! keeps Darkmatter's *type* (the same escape hatch
//! `level2_lifecycle_action_forms.rs` pins), so the fixture document writes
//! `worktree: null` from the main checkout and `worktree: <directory name>`
//! from the linked one. The composed body is asserted as well, because AC2
//! names the composed value and not only the lifecycle namespace.
//!
//! The linked worktree's directory name and its branch name differ on purpose:
//! `ctx.worktree` must be the directory basename, and a test whose fixture
//! spells both the same way could not tell the two apart.
//!
//! ## Focus
//!
//! The backend is tmux, which is headless: it creates a detached session and
//! never opens, raises, or activates a window. This file names no
//! `SpawnVisibility::Foreground`, no `focus_spawned_pane`, and no GUI
//! automation, so running it cannot take focus from whoever is at the machine.
//!
//! ## Skip-clean
//!
//! `require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux)` skips
//! when tmux is absent and hard-fails under
//! `BISCUIT_TEST_REQUIRED_BACKENDS=tmux`. Run via `just test-l2`.

#![cfg(unix)]

use crate::common;
use common::wrap::seed_minimal_config;
use common::{augmented_path, helper_command, write_executable};

use biscuit_test_harness::TerminalHarness;
use biscuit_test_harness::tmux::{TmuxHarness, kill_session_by_name, spawn_shell_session};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};
use tempfile::tempdir;
use test_toolkit::{Backend, Level, require_level};

/// The fixture document, committed once into the main checkout so the linked
/// worktree receives the identical file.
///
/// `set_frontmatter` carries the whole-value `{{ ctx.worktree }}` so the
/// written YAML keeps the value's type; the body carries the rendered spelling.
const PROBE_DOCUMENT: &str = r#"---
title: AC2 worktree probe
success:
  stack:
    - action: {set_frontmatter: ["state.md", "worktree", "{{ ctx.worktree }}"]}
---
WT=[{{ ctx.worktree }}]
"#;

/// The linked worktree's directory name — the value AC2 says `ctx.worktree`
/// must carry.
const LINKED_DIRECTORY: &str = "ac2-linked-dir";

/// The linked worktree's branch, deliberately spelled differently from
/// [`LINKED_DIRECTORY`].
const LINKED_BRANCH: &str = "ac2-linked-branch";

/// A fixture-owned repository pair: a main checkout and a linked worktree of
/// it, plus the fake provider and home directory the composes run against.
struct Fixture {
    _root: tempfile::TempDir,
    home: PathBuf,
    bin: PathBuf,
    spool: PathBuf,
    /// Where each run's completion file lands — outside both checkouts, so the
    /// marker cannot show up as repository state the run observes.
    done_dir: PathBuf,
    main: PathBuf,
    linked: PathBuf,
}

/// Run `git` from the test process with the inherited `GIT_*` plumbing removed,
/// pinned to a fixture identity so a host `commit.gpgsign` cannot make the
/// fixture commit prompt for a passphrase or fail.
///
/// Both output streams are discarded rather than inherited. Nothing here is
/// asserted on, and an inherited stream is nextest's `LEAK` hazard: any
/// background helper `git` detaches — a `gc --auto`, an fsmonitor daemon a host
/// `~/.gitconfig` asks for — would hold the *test's* stdout and stderr open
/// after the test itself has finished.
fn git(dir: &Path, args: &[&str]) {
    let status = helper_command("git")
        .args([
            "-c",
            "user.name=Claudine Test",
            "-c",
            "user.email=claudine@example.invalid",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .current_dir(dir)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .unwrap_or_else(|error| panic!("git {args:?} failed to start: {error}"));
    assert!(status.success(), "git {args:?} failed in {}", dir.display());
}

fn stage() -> Fixture {
    let root = tempdir().unwrap();
    let home = root.path().join("home");
    let bin = root.path().join("bin");
    let spool = root.path().join("audio-spool");
    let done_dir = root.path().join("done");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(&bin).unwrap();
    fs::create_dir_all(&done_dir).unwrap();
    seed_minimal_config(&home);
    // Drains stdin the way the real wrappers expect, then exits 0 so the
    // `success` event — which carries the assertion — fires.
    write_executable(&bin.join("goose"), "#!/bin/sh\ncat > /dev/null\nexit 0\n");

    let main = root.path().join("main");
    fs::create_dir_all(&main).unwrap();
    git(&main, &["init", "-q", "-b", "main"]);
    fs::write(main.join("doc.md"), PROBE_DOCUMENT).unwrap();
    fs::write(
        main.join("state.md"),
        "---\nseeded: true\n---\nstate body\n",
    )
    .unwrap();
    git(&main, &["add", "doc.md", "state.md"]);
    git(&main, &["commit", "-q", "-m", "ac2 fixture"]);

    let linked = root.path().join(LINKED_DIRECTORY);
    git(
        &main,
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            LINKED_BRANCH,
            linked.to_str().expect("fixture path is UTF-8"),
        ],
    );

    Fixture {
        _root: root,
        home,
        bin,
        spool,
        done_dir,
        main,
        linked,
    }
}

/// Compose `doc.md` from `launch_dir` inside a real tmux pane.
///
/// Readiness is the *process having exited*, signalled by a fixture-owned
/// completion file the pane's shell writes after `claudine` returns — not by
/// the `success` stack's own output. Killing the session while the run is still
/// finalizing would orphan whatever it had spawned, which nextest reports as a
/// leak rather than as the race it is. The file lives outside the repository so
/// it cannot alter the repository state the run observes, and its name is typed
/// into the pane before it exists, so its *presence* is unambiguous.
///
/// Returns the captured pane text so the composed body can be asserted too.
fn compose_in_tmux(fixture: &Fixture, launch_dir: &Path) -> String {
    static SEQ: AtomicU32 = AtomicU32::new(0);
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);

    let session = format!("biscuit_l2_ac2_{}_{seq}", std::process::id());
    spawn_shell_session(&session, 180, 60).expect("failed to spawn tmux session");
    let mut harness = TmuxHarness::attach(&session);
    let _ = biscuit_test_harness::wait_for_prompt(&mut harness);

    let done = fixture.done_dir.join(format!("run-{seq}.done"));
    let command = format!(
        "cd {dir} && NO_COLOR='1' HOME='{home}' PATH='{path}' \
         CLAUDINE_RENDEZVOUS_REPORT='false' PLAYA_DRY_RUN='1' PLAYA_SPOOL_DIR='{spool}' \
         {claudine} compose --goose doc.md ; echo done > {done}",
        dir = launch_dir.display(),
        home = fixture.home.display(),
        path = augmented_path(&fixture.bin).to_string_lossy(),
        spool = fixture.spool.display(),
        claudine = common::claudine_bin(),
        done = done.display(),
    );
    harness
        .send_command_with_env(&command, &[])
        .expect("send compose command");

    let deadline = Instant::now() + Duration::from_secs(20);
    while Instant::now() < deadline && !done.exists() {
        std::thread::sleep(Duration::from_millis(100));
    }
    assert!(
        done.exists(),
        "the compose launched in {} did not finish within the deadline; pane:\n{}",
        launch_dir.display(),
        harness
            .capture()
            .map(|frame| frame.plain)
            .unwrap_or_default(),
    );

    let pane = harness
        .capture()
        .map(|frame| frame.plain)
        .unwrap_or_default();
    kill_session_by_name(&session);
    pane
}

fn state_frontmatter(launch_dir: &Path) -> String {
    fs::read_to_string(launch_dir.join("state.md")).unwrap_or_default()
}

/// AC2, both halves: a real `claudine compose` launched from a linked worktree
/// reports that worktree's *directory* name, and the same document composed
/// from the main checkout reports `null`.
///
/// Both halves live in one test because the contract is the difference between
/// them: either value alone is consistent with a broken capture that always
/// answers the same way.
#[test]
fn level2_ac2_worktree_is_the_linked_directory_and_null_in_the_main_checkout() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

    let fixture = stage();

    let linked_pane = compose_in_tmux(&fixture, &fixture.linked);
    let linked_state = state_frontmatter(&fixture.linked);
    assert!(
        linked_state.contains(&format!("worktree: {LINKED_DIRECTORY}")),
        "a compose launched from the linked worktree must write the worktree \
         directory name; state.md was:\n{linked_state}\npane:\n{linked_pane}"
    );
    assert!(
        !linked_state.contains(LINKED_BRANCH),
        "`ctx.worktree` is the worktree directory basename, never the branch \
         name; state.md was:\n{linked_state}\npane:\n{linked_pane}"
    );
    assert!(
        linked_pane.contains(&format!("WT=[{LINKED_DIRECTORY}]")),
        "the composed body must carry the same worktree name the lifecycle \
         namespace saw; pane:\n{linked_pane}"
    );

    let main_pane = compose_in_tmux(&fixture, &fixture.main);
    let main_state = state_frontmatter(&fixture.main);
    assert!(
        main_state.contains("worktree: null"),
        "the same document composed from the main checkout must write a typed \
         YAML null, not a worktree name; state.md was:\n{main_state}\npane:\n{main_pane}"
    );
    assert!(
        main_pane.contains("WT=[]"),
        "the composed body in the main checkout must render the empty spelling \
         of `null`; pane:\n{main_pane}"
    );
}
