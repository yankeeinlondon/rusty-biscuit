//! Level 1 PTY tests: a lifecycle `shell` action runs as its own process tree
//! and holds the terminal's foreground while it runs.
//!
//! A lifecycle command inherits Claudine's terminal. In a process group of its
//! own that is not the terminal's foreground group, it is a background job: the
//! kernel stops it (`SIGTTOU`/`SIGTTIN`) the first time it changes the
//! terminal's modes or reads from it, and nothing ever resumes it. A nested
//! `claudine compose` run from a lifecycle `shell` stopped exactly this way.
//! `stty` changing the echo flag is the smallest command that touches the
//! terminal the same way. Holding the foreground also means a Ctrl-C typed
//! while the command runs reaches the command, not Claudine, so the command's
//! `SIGINT` end is what cancels the run.
//!
//! These need a controlling terminal, so only a PTY run shows them.
//! `#![cfg(unix)]` is the only exclusion, because `expectrl`'s `OsSession` is
//! Unix-only.

use expectrl::Session;
use expectrl::process::unix::WaitStatus;
use expectrl::session::OsSession;
use std::fs;
use std::time::{Duration, Instant};
use test_toolkit::{Level, expect_level};

use crate::common::{CliProcessFixture, pty_available, strip_ansi, write_executable};

#[test]
fn level1_pty_lifecycle_shell_using_the_terminal_completes() {
    expect_level!(Level::L1, pty_available(), "PTY (/dev/ptmx)");

    let fixture = CliProcessFixture::named("lifecycle-shell-terminal-pty");
    let claudine_dir = fixture.home().join(".claudine");
    fs::create_dir_all(&claudine_dir).unwrap();
    fs::write(claudine_dir.join("config.yaml"), "{}").unwrap();
    write_executable(&fixture.bin_dir().join("goose"), "#!/bin/sh\nexit 0\n");

    let marker = fixture.cwd().join("terminal-ok.txt");
    let md = fixture.cwd().join("doc.md");
    fs::write(
        &md,
        "---\ntitle: t\nsuccess:\n    - {shell: \"stty -echo && stty echo && echo ok > terminal-ok.txt\"}\n---\nBody.\n",
    )
    .unwrap();

    let mut cmd = fixture.command_std();
    cmd.args(["compose", "--goose", "--yolo", md.to_str().unwrap()]);
    let mut session: OsSession = Session::spawn(cmd).expect("spawn PTY session");

    // Drain the PTY while waiting so the child never blocks on a full buffer.
    let mut transcript = Vec::new();
    let mut scratch = [0u8; 4096];
    session.set_expect_timeout(Some(Duration::from_millis(100)));
    let stop = Instant::now() + Duration::from_secs(20);
    while Instant::now() < stop && !marker.exists() {
        match session.try_read(&mut scratch) {
            Ok(n) if n > 0 => transcript.extend_from_slice(&scratch[..n]),
            _ => std::thread::sleep(Duration::from_millis(20)),
        }
    }

    assert!(
        marker.exists(),
        "the lifecycle `shell` that changes terminal modes never finished: a \
         command in a background process group is stopped by the kernel; \
         transcript:\n{}",
        strip_ansi(&String::from_utf8_lossy(&transcript))
    );
}

/// A `start` command holding the foreground that ends by `SIGINT` (as a typed
/// Ctrl-C ends it) cancels the run: the background process it left is
/// stopped, the next `start` action does not run although `no_error`
/// tolerated the failure, `cancelled` and `finalize` run (and can still use the
/// terminal), and Claudine exits 130.
#[test]
fn level1_pty_lifecycle_shell_interrupted_in_the_foreground_cancels_the_run() {
    expect_level!(Level::L1, pty_available(), "PTY (/dev/ptmx)");

    let fixture = CliProcessFixture::named("lifecycle-shell-foreground-interrupt-pty");
    let claudine_dir = fixture.home().join(".claudine");
    fs::create_dir_all(&claudine_dir).unwrap();
    fs::write(claudine_dir.join("config.yaml"), "{}").unwrap();
    write_executable(&fixture.bin_dir().join("goose"), "#!/bin/sh\nexit 0\n");

    let md = fixture.cwd().join("doc.md");
    fs::write(
        &md,
        "---\ntitle: t\nstart:\n    - shell:\n        command: \"sleep 30 & echo $! > bg.pid; kill -INT $$\"\n        no_error: true\n    - {shell: \"echo later > later.txt\"}\ncancelled:\n    - {shell: \"stty -echo && stty echo && echo c > cancelled.txt\"}\nfinalize:\n    - {shell: \"echo f > finalize.txt\"}\n---\nBody.\n",
    )
    .unwrap();

    let mut cmd = fixture.command_std();
    cmd.args(["compose", "--goose", "--yolo", md.to_str().unwrap()]);
    let mut session: OsSession = Session::spawn(cmd).expect("spawn PTY session");

    let mut transcript = Vec::new();
    let mut scratch = [0u8; 4096];
    session.set_expect_timeout(Some(Duration::from_millis(100)));
    let finalized = fixture.cwd().join("finalize.txt");
    let stop = Instant::now() + Duration::from_secs(20);
    while Instant::now() < stop && !finalized.exists() {
        match session.try_read(&mut scratch) {
            Ok(n) if n > 0 => transcript.extend_from_slice(&scratch[..n]),
            _ => std::thread::sleep(Duration::from_millis(20)),
        }
    }
    let transcript = strip_ansi(&String::from_utf8_lossy(&transcript));
    assert!(finalized.exists(), "finalize did not run; transcript:\n{transcript}");
    let status = session.get_process().wait().expect("wait for claudine compose");

    let background: i32 = fs::read_to_string(fixture.cwd().join("bg.pid")).unwrap().trim().parse().unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    // SAFETY: signal 0 only checks that the PID exists.
    while unsafe { libc::kill(background, 0) } == 0 {
        assert!(Instant::now() < deadline, "the lifecycle command's background process survived");
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(!fixture.cwd().join("later.txt").exists(), "a start action ran after the interrupt; transcript:\n{transcript}");
    assert!(fixture.cwd().join("cancelled.txt").exists(), "cancelled did not run; transcript:\n{transcript}");
    assert!(matches!(status, WaitStatus::Exited(_, 130)), "{status:?}; transcript:\n{transcript}");
}
