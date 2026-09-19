#![cfg(unix)]

//! Level-2: the shell tmux gives the harness must not have run the host's
//! *interactive* rc file.
//!
//! tmux is the portable Level-2 backend CI uses, so a pane whose shell picks
//! up Atuin, starship, fzf, or zoxide is not a controlled terminal — an Atuin
//! first-run picker rendered into a WezTerm pane once swallowed the command
//! line the harness had sent and stalled the test until its exit marker timed
//! out.
//!
//! The rc file here is reached the way a real host reaches it: through the
//! login profile, which is the chain the harness cannot refuse (it is where
//! `PATH` is assembled). What the harness controls is whether the shell that
//! sources it is the one it drives.

use std::io;
use std::process::Command;

use biscuit_test_harness::tmux::{TmuxHarness, kill_session_by_name, spawn_shell_session_with_env};
use biscuit_test_harness::{
    TerminalHarness, frame_shows_prompt, skip_with_reason, wait_for_prompt,
};

/// Printed by the fixture rc file, but only from an interactive shell — which
/// is exactly the shell the harness drives and exactly what a prompt hook
/// would install itself into.
const RC_SENTINEL: &str = "BISCUIT_RC_SENTINEL_LOADED";

/// The prompt the fixture rc file installs. Chosen to end in `$` so a pane
/// that *did* read it still satisfies `wait_for_prompt`; the assertion, not a
/// timeout, is what has to distinguish the two shells.
const RC_PROMPT: &str = "BISCUIT_RC_PROMPT$";

#[test]
fn level2_tmux_pane_shell_skips_the_interactive_rc() -> io::Result<()> {
    // No `require_level!`: `test-toolkit` depends on nothing in this workspace,
    // but this crate is what `test-toolkit`'s L2 consumers build on, so the
    // harness keeps its own skip helper rather than acquiring a test-only
    // dependency edge back through the tier it provides.
    if !TmuxHarness::available() {
        skip_with_reason("tmux");
        return Ok(());
    }
    // The fixture rc file is bash's. A host without bash drives some other
    // shell and would never read it, which would make the assertion vacuous.
    if biscuit_test_harness::detect_shell() != "bash" {
        skip_with_reason("bash (the fixture rc file is `~/.bashrc`)");
        return Ok(());
    }

    let home = tempfile::tempdir()?;
    std::fs::write(
        home.path().join(".bashrc"),
        format!("case $- in *i*) echo {RC_SENTINEL} ;; esac\nPS1='{RC_PROMPT} '\n"),
    )?;
    // The near-universal host convention, and the one the finding is about:
    // the login profile is what pulls the interactive rc in.
    std::fs::write(home.path().join(".bash_profile"), ". \"$HOME/.bashrc\"\n")?;

    let session = OwnedSession(format!("biscuit_rc_probe_{}", std::process::id()));
    spawn_shell_session_with_env(
        &session.0,
        80,
        24,
        &[("HOME", &home.path().to_string_lossy())],
    )?;
    let mut harness = TmuxHarness::attach(&session.0);
    assert_pane_is_rc_suppressed(&mut harness, &session.0)
}

/// Kills the session on the way out, including when an assertion unwinds
/// through it. A tmux session outlives the process that created it, so a
/// failing run would otherwise leave one behind for every attempt.
struct OwnedSession(String);

impl Drop for OwnedSession {
    fn drop(&mut self) {
        kill_session_by_name(&self.0);
    }
}

fn assert_pane_is_rc_suppressed(harness: &mut TmuxHarness, session: &str) -> io::Result<()> {
    wait_for_prompt(harness)?;
    let frame = harness.capture()?;

    assert!(
        frame_shows_prompt(&frame.plain),
        "the pane never reached a prompt:\n{}",
        frame.plain
    );
    assert!(
        !frame.plain.contains(RC_SENTINEL),
        "the driven shell ran ~/.bashrc:\n{}",
        frame.plain
    );
    assert!(
        !frame.plain.contains(RC_PROMPT),
        "the driven shell took its prompt from ~/.bashrc:\n{}",
        frame.plain
    );

    let argv = pane_process_argv(session)?;
    assert!(
        argv.contains("--norc"),
        "the pane's own process must be the rc-suppressed interactive shell; got {argv:?}"
    );
    Ok(())
}

/// The command line of the process tmux started for the pane.
///
/// The outer login shell `exec`s the inner one, so the pid is unchanged and
/// this reports the shell the harness actually drives.
fn pane_process_argv(session: &str) -> io::Result<String> {
    let out = Command::new("tmux")
        .args(["display-message", "-p", "-t", session, "#{pane_pid}"])
        .output()?;
    let pid = String::from_utf8_lossy(&out.stdout).trim().to_string();
    let out = Command::new("ps")
        .args(["-o", "args=", "-p", &pid])
        .output()?;
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}
