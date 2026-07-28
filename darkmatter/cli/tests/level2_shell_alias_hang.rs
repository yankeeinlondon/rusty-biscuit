//! Real-PTY regression for the `::shell` alias-resolution hang.
//!
//! Covers the defect recorded in
//! `darkmatter/fixes/2026-07-27-alias-resolution-hang/spec.md`: composing a
//! `::shell` document from a **background process group of a controlling
//! terminal** wedges forever, because alias resolution spawns
//! `$SHELL -ic 'alias …'` with no timeout and a bash-family interactive shell
//! stops itself rather than run outside the terminal's foreground group. The
//! stopped child never closes its stdout pipe, so `Command::output()` blocks
//! with no bound.
//!
//! Three conditions must coincide or the defect does not appear, so all three
//! are load-bearing below: a controlling terminal (tmux pane), job control
//! (`set -m`, so `&` moves the job into its own process group), and a
//! bash-family `$SHELL`. Drop any one and this test passes against the broken
//! code — which is why it would then be worthless.

mod common;

use biscuit_test_harness::TerminalHarness;
use biscuit_test_harness::tmux::TmuxHarness;
use common::level2::MD_BIN;
use serial_test::serial;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use test_toolkit::{Level, require_level};

/// Wall time allowed for the backgrounded `md compose` to finish. A healthy
/// run of this one-directive document lands near half a second; the hang is
/// unbounded, so any generous-but-finite bound discriminates the two.
const COMPLETION_TIMEOUT: Duration = Duration::from_secs(10);

/// Executable that must not exist on any host, so composition takes the
/// `which::which` miss branch — the branch that consults `$SHELL`.
const MISSING_EXECUTABLE: &str = "nonexistent_command_xyz";

/// Written by the pane after `md` exits. The `:END` suffix makes the poll
/// immune to observing a partially written file.
const DONE_MARKER: &str = ":END";

#[test]
#[serial(level2_terminal)]
fn level2_compose_shell_directive_completes_in_background_process_group() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");
    let shell = bash_family_shell();
    require_level!(Level::L2, shell.is_some(), "bash-family shell");
    let shell = shell.expect("gated on is_some above");

    let fixture = tempfile::tempdir().expect("create fixture directory");
    let doc = fixture.path().join("doc.md");
    let log = fixture.path().join("compose.log");
    let status = fixture.path().join("status");
    std::fs::write(&doc, format!("# Test\n::shell {MISSING_EXECUTABLE}\n"))
        .expect("write markdown fixture");
    // Approve the directive up front so the run reaches command resolution
    // instead of stopping at the approval prompt.
    std::fs::write(
        fixture.path().join(".darkmatter-shell-whitelist"),
        format!("prefix {MISSING_EXECUTABLE}\n"),
    )
    .expect("write shell whitelist");

    // Own pane, not `shared_or_spawn`: this test backgrounds a job that can
    // wedge, and teardown has to reap that pane's whole process tree.
    let mut harness = TmuxHarness::new();
    harness.spawn_shell().expect("spawn tmux pane");
    let pane_pid = tmux_pane_pid(harness.session_name());
    // Declared after `harness` so it drops *first* — the tree must be reaped
    // while the session still exists.
    let _reaper = PaneTreeReaper { pane_pid };

    let command = format!(
        "set -m; {{ SHELL={shell} {md} compose {doc} > {log} 2>&1; \
         printf 'EXIT:%s{DONE_MARKER}\\n' \"$?\" > {status}; }} &",
        shell = shell.display(),
        md = MD_BIN,
        doc = doc.display(),
        log = log.display(),
        status = status.display(),
    );
    harness
        .send_command_with_env(&command, &[])
        .expect("send backgrounded compose command");

    let started = Instant::now();
    let outcome = poll_for_marker(&status, COMPLETION_TIMEOUT);
    let elapsed = started.elapsed();

    let Some(marker) = outcome else {
        panic!(
            "md compose never completed within {COMPLETION_TIMEOUT:?} in a background \
             process group of a PTY (SHELL={shell}).\n\
             Stopped-process evidence:\n{tree}\n\
             partial output:\n{log}",
            shell = shell.display(),
            tree = describe_process_tree(pane_pid),
            log = read_stripped(&log),
        );
    };

    assert!(
        elapsed < COMPLETION_TIMEOUT,
        "poll returned {marker:?} but only after {elapsed:?}"
    );

    // Asserting completion alone would stop covering anything once the alias
    // probe is gone, so pin the diagnostic too: the authored executable must
    // still surface through the typed `CommandNotFound` path.
    let exit_code = marker
        .trim()
        .strip_prefix("EXIT:")
        .and_then(|rest| rest.strip_suffix(DONE_MARKER))
        .and_then(|code| code.parse::<i32>().ok())
        .unwrap_or_else(|| panic!("unparsable completion marker {marker:?}"));
    assert_ne!(
        exit_code, 0,
        "an unresolvable executable must fail the compose. log:\n{}",
        read_stripped(&log)
    );

    let output = read_stripped(&log);
    let flattened = output.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(
        flattened.to_lowercase().contains("command not found"),
        "expected the CommandNotFound diagnostic. log:\n{output}"
    );
    assert!(
        flattened.contains(MISSING_EXECUTABLE),
        "diagnostic must name the authored executable {MISSING_EXECUTABLE}. log:\n{output}"
    );
}

/// Polls `path` until it holds a complete completion marker or `timeout`
/// elapses, returning the marker text.
fn poll_for_marker(path: &Path, timeout: Duration) -> Option<String> {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if let Ok(text) = std::fs::read_to_string(path)
            && text.contains(DONE_MARKER)
        {
            return Some(text);
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    None
}

fn read_stripped(path: &Path) -> String {
    match std::fs::read_to_string(path) {
        Ok(text) => biscuit_test_harness::strip_ansi(&text),
        Err(err) => format!("<unreadable: {err}>"),
    }
}

/// Locates a shell whose interactive startup performs job control and stops
/// itself outside the foreground process group. zsh does not take that path,
/// so it cannot reproduce the defect and is deliberately not a candidate.
///
/// ## Returns
///
/// The resolved absolute path, so the pane runs the same binary this probe
/// found rather than re-resolving a bare name against the pane's own `PATH`.
fn bash_family_shell() -> Option<PathBuf> {
    ["bash", "dash"].into_iter().find_map(on_path)
}

fn on_path(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path).find_map(|dir| {
        let candidate = dir.join(name);
        is_executable_file(&candidate).then_some(candidate)
    })
}

#[cfg(unix)]
fn is_executable_file(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path)
        .map(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn is_executable_file(path: &Path) -> bool {
    path.is_file()
}

fn tmux_pane_pid(session: &str) -> Option<u32> {
    let out = Command::new("tmux")
        .args(["display-message", "-p", "-t", session, "#{pane_pid}"])
        .stderr(Stdio::null())
        .output()
        .ok()?;
    String::from_utf8_lossy(&out.stdout).trim().parse().ok()
}

/// Kills everything the pane spawned when the test leaves, by any path.
///
/// A wedged run leaves `md` and its `$SHELL -ic` child in state `T`
/// (stopped). Those survive the pane's own teardown on some hosts, and
/// nextest's `LEAK` check fails the whole run when they do — so reaping is
/// part of the test, not housekeeping.
struct PaneTreeReaper {
    pane_pid: Option<u32>,
}

impl Drop for PaneTreeReaper {
    fn drop(&mut self) {
        let Some(root) = self.pane_pid else {
            return;
        };
        let victims = descendants_of(root);
        if victims.is_empty() {
            return;
        }
        let mut kill = Command::new("kill");
        kill.arg("-9");
        for pid in &victims {
            kill.arg(pid.to_string());
        }
        let _ = kill.stdout(Stdio::null()).stderr(Stdio::null()).status();
    }
}

/// Returns every descendant of `root`, deepest last, excluding `root` itself
/// (the pane process is the tmux session's to kill).
fn descendants_of(root: u32) -> Vec<u32> {
    let Ok(out) = Command::new("ps")
        .args(["-eo", "pid=,ppid="])
        .stderr(Stdio::null())
        .output()
    else {
        return Vec::new();
    };
    let table: Vec<(u32, u32)> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            let pid = fields.next()?.parse().ok()?;
            let ppid = fields.next()?.parse().ok()?;
            Some((pid, ppid))
        })
        .collect();

    let mut found = vec![root];
    let mut cursor = 0;
    while cursor < found.len() {
        let parent = found[cursor];
        for (pid, ppid) in &table {
            if *ppid == parent && !found.contains(pid) {
                found.push(*pid);
            }
        }
        cursor += 1;
    }
    found.remove(0);
    found
}

/// Renders `ps` detail for the pane subtree, so a hang failure carries the
/// process-group evidence that identifies it (`T` state, `PGID != TPGID`).
fn describe_process_tree(root: Option<u32>) -> String {
    let Some(root) = root else {
        return "<pane pid unavailable>".to_string();
    };
    let mut pids = vec![root];
    pids.extend(descendants_of(root));
    let mut cmd = Command::new("ps");
    cmd.args(["-o", "pid=,ppid=,pgid=,tpgid=,stat=,tty=,args="]);
    for pid in &pids {
        cmd.args(["-p", &pid.to_string()]);
    }
    match cmd.stderr(Stdio::null()).output() {
        Ok(out) => String::from_utf8_lossy(&out.stdout).into_owned(),
        Err(err) => format!("<ps failed: {err}>"),
    }
}
