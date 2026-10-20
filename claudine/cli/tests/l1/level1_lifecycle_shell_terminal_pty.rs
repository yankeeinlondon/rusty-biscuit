//! Level 1 PTY test: a lifecycle `shell` action that uses the terminal is not
//! stopped by job control.
//!
//! A lifecycle command inherits Claudine's terminal. Moved into a process
//! group of its own while Claudine holds the terminal's foreground, it is a
//! background job: the kernel stops it (`SIGTTOU`/`SIGTTIN`) the first time it
//! changes the terminal's modes or reads from it, and nothing ever resumes it.
//! A nested `claudine compose` run from a lifecycle `shell` stopped exactly this
//! way. `stty` changing the echo flag is the smallest command that touches the
//! terminal the same way.
//!
//! The hazard needs a controlling terminal, so only a PTY run shows it.
//! `#![cfg(unix)]` is the only exclusion, because `expectrl`'s `OsSession` is
//! Unix-only.

use expectrl::Session;
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
    fs::write(claudine_dir.join("config.json"), "{}").unwrap();
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
