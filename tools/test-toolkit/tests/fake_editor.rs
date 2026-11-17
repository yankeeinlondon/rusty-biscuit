//! The fake editor's environment-variable contract, proved by launching the
//! real binary the way a consuming package's tests do.

use std::io::Write as _;
use std::path::Path;
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};

use test_toolkit::argv_recorder::read_argv_records;
use test_toolkit::fake_editor::{
    FAKE_EDITOR_DELETE, FAKE_EDITOR_ERR_TEXT, FAKE_EDITOR_EXIT, FAKE_EDITOR_HOLD, FAKE_EDITOR_OUT_TEXT,
    FAKE_EDITOR_PAUSE, FAKE_EDITOR_PIDS, FAKE_EDITOR_READ_LINE, FAKE_EDITOR_RECORD, FAKE_EDITOR_REPLACE, FAKE_EDITOR_SPAWN_CHILD,
    FAKE_EDITOR_STDIO, FAKE_EDITOR_WRITE, read_pids, read_stdio_report,
};

fn editor() -> Command {
    let mut command = Command::new(biscuit_test_harness::bin_exe!("test-toolkit-fake-editor"));
    // A caller's environment must not leak a variable into a case.
    for (name, _) in std::env::vars_os() {
        if name.to_string_lossy().starts_with("FAKE_EDITOR_") {
            command.env_remove(name);
        }
    }
    command.stdin(Stdio::null());
    command
}

fn run(command: &mut Command) -> Output {
    command.output().expect("launch the fake editor")
}

fn wait_for(path: &Path) {
    let started = Instant::now();
    while !path.exists() {
        assert!(started.elapsed() < Duration::from_secs(10), "{} never appeared", path.display());
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn stop(mut child: Child) {
    let _ = child.kill();
    let _ = child.wait();
}

#[test]
fn no_variables_exits_zero_and_leaves_the_target_untouched() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("note.md");
    std::fs::write(&target, "original").unwrap();
    let output = run(editor().arg(&target));
    assert!(output.status.success(), "{output:?}");
    assert!(output.stdout.is_empty() && output.stderr.is_empty());
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "original");
}

#[test]
fn record_appends_one_argv_record_per_run() {
    let dir = tempfile::tempdir().unwrap();
    let record = dir.path().join("argv.bin");
    let target = dir.path().join("my note.md");
    for _ in 0..2 {
        let output = run(editor().env(FAKE_EDITOR_RECORD, &record).args(["--wait"]).arg(&target));
        assert!(output.status.success(), "{output:?}");
    }
    let expected = vec!["--wait".to_string(), target.to_string_lossy().into_owned()];
    assert_eq!(read_argv_records(&record).unwrap(), vec![expected.clone(), expected]);
}

#[test]
fn write_copies_content_into_the_target_in_place() {
    let dir = tempfile::tempdir().unwrap();
    let content = dir.path().join("content.txt");
    let target = dir.path().join("note.md");
    std::fs::write(&content, "edited\n").unwrap();
    std::fs::write(&target, "a much longer original body").unwrap();
    let output = run(editor().env(FAKE_EDITOR_WRITE, &content).arg(&target));
    assert!(output.status.success(), "{output:?}");
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "edited\n");
}

#[test]
fn replace_saves_through_a_rename_and_leaves_no_temporary() {
    let dir = tempfile::tempdir().unwrap();
    let content = dir.path().join("content.txt");
    let target = dir.path().join("note.md");
    std::fs::write(&content, "replaced").unwrap();
    std::fs::write(&target, "original").unwrap();
    let output = run(editor().env(FAKE_EDITOR_REPLACE, &content).arg(&target));
    assert!(output.status.success(), "{output:?}");
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "replaced");
    let names: Vec<_> = std::fs::read_dir(dir.path()).unwrap().map(|entry| entry.unwrap().file_name()).collect();
    assert_eq!(names.len(), 2, "only the content file and the target remain: {names:?}");
}

#[test]
fn delete_removes_the_target() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("note.md");
    std::fs::write(&target, "original").unwrap();
    let output = run(editor().env(FAKE_EDITOR_DELETE, "1").arg(&target));
    assert!(output.status.success(), "{output:?}");
    assert!(!target.exists());
}

#[test]
fn exit_sets_the_exit_code() {
    let dir = tempfile::tempdir().unwrap();
    let output = run(editor().env(FAKE_EDITOR_EXIT, "3").arg(dir.path().join("note.md")));
    assert_eq!(output.status.code(), Some(3));
}

#[test]
fn an_unreadable_content_file_fails_loudly_with_exit_two() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("note.md");
    let output = run(editor().env(FAKE_EDITOR_WRITE, dir.path().join("missing.txt")).arg(&target));
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("fake editor: read"), "{output:?}");
    assert!(!target.exists());
}

#[test]
fn out_and_err_text_reach_their_streams() {
    let dir = tempfile::tempdir().unwrap();
    let output = run(editor()
        .env(FAKE_EDITOR_OUT_TEXT, "to stdout")
        .env(FAKE_EDITOR_ERR_TEXT, "to stderr")
        .arg(dir.path().join("note.md")));
    assert!(output.status.success(), "{output:?}");
    assert_eq!(output.stdout, b"to stdout");
    assert_eq!(output.stderr, b"to stderr");
}

#[test]
fn stdio_reports_non_terminal_streams_and_piped_input() {
    let dir = tempfile::tempdir().unwrap();
    let report = dir.path().join("stdio.json");
    let mut child = editor()
        .env(FAKE_EDITOR_STDIO, &report)
        .arg(dir.path().join("note.md"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(b"piped input").unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success(), "{output:?}");
    let report = read_stdio_report(&report).unwrap();
    assert!(!report.stdin_tty && !report.stdout_tty && !report.stderr_tty, "{report:?}");
    assert_eq!(report.stdin.as_deref(), Some("piped input"));
}

#[test]
fn read_line_writes_one_stdin_line_into_the_target() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("note.md");
    let report = dir.path().join("stdio.json");
    let mut child = editor()
        .env(FAKE_EDITOR_READ_LINE, "1")
        .env(FAKE_EDITOR_STDIO, &report)
        .arg(&target)
        .stdin(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(b"first line\r\nsecond line\n").unwrap();
    assert!(child.wait().unwrap().success());
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "first line");
    assert_eq!(read_stdio_report(&report).unwrap().stdin.as_deref(), Some("first line"));
}

#[test]
fn hold_creates_its_marker_and_stays_running_until_killed() {
    let dir = tempfile::tempdir().unwrap();
    let marker = dir.path().join("ready");
    let mut child = editor().env(FAKE_EDITOR_HOLD, &marker).arg(dir.path().join("note.md")).spawn().unwrap();
    wait_for(&marker);
    assert!(child.try_wait().unwrap().is_none(), "a held editor is still running");
    stop(child);
}

#[test]
fn pause_waits_for_its_marker_to_be_removed_then_exits_normally() {
    let dir = tempfile::tempdir().unwrap();
    let marker = dir.path().join("paused");
    let mut child = editor()
        .env(FAKE_EDITOR_PAUSE, &marker)
        .env(FAKE_EDITOR_EXIT, "3")
        .arg(dir.path().join("note.md"))
        .spawn()
        .unwrap();
    wait_for(&marker);
    assert!(child.try_wait().unwrap().is_none(), "a paused editor waits");
    std::fs::remove_file(&marker).unwrap();
    assert_eq!(child.wait().unwrap().code(), Some(3), "it continues to its normal exit");
}

#[test]
fn spawn_child_records_a_live_child_pid() {
    let dir = tempfile::tempdir().unwrap();
    let pids = dir.path().join("pids");
    // Not captured: on Windows the child inherits the editor's standard
    // handles, so reading the editor's output would wait for the child.
    let status = editor()
        .env(FAKE_EDITOR_SPAWN_CHILD, "1")
        .env(FAKE_EDITOR_PIDS, &pids)
        .arg(dir.path().join("note.md"))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .unwrap();
    assert!(status.success(), "{status:?}");
    let recorded = read_pids(&pids).unwrap();
    assert_eq!(recorded.len(), 1, "{recorded:?}");
    let child = recorded[0];
    assert!(process_alive(child), "the child outlives the editor");
    kill_pid(child);
}

#[cfg(unix)]
#[test]
fn ignore_term_keeps_a_held_editor_running_after_sigterm() {
    let dir = tempfile::tempdir().unwrap();
    let marker = dir.path().join("ready");
    let mut child = editor()
        .env(test_toolkit::fake_editor::FAKE_EDITOR_IGNORE_TERM, "1")
        .env(FAKE_EDITOR_HOLD, &marker)
        .arg(dir.path().join("note.md"))
        .spawn()
        .unwrap();
    wait_for(&marker);
    // SAFETY: plain `kill(2)` on a PID this test started.
    assert_eq!(unsafe { libc::kill(child.id() as libc::pid_t, libc::SIGTERM) }, 0);
    std::thread::sleep(Duration::from_millis(200));
    assert!(child.try_wait().unwrap().is_none(), "the editor ignored SIGTERM");
    stop(child);
}

#[cfg(unix)]
#[test]
fn ignore_term_reaches_a_spawned_child() {
    let dir = tempfile::tempdir().unwrap();
    let pids = dir.path().join("pids");
    // The editor exits at once, so the child is adopted (and, if it died,
    // reaped) by init rather than lingering as this test's zombie.
    let output = run(editor()
        .env(test_toolkit::fake_editor::FAKE_EDITOR_IGNORE_TERM, "1")
        .env(FAKE_EDITOR_SPAWN_CHILD, "1")
        .env(FAKE_EDITOR_PIDS, &pids)
        .arg(dir.path().join("note.md")));
    assert!(output.status.success(), "{output:?}");
    let grandchild = read_pids(&pids).unwrap()[0];
    // SAFETY: plain `kill(2)` on a PID this test started.
    assert_eq!(unsafe { libc::kill(grandchild as libc::pid_t, libc::SIGTERM) }, 0);
    std::thread::sleep(Duration::from_millis(200));
    assert!(process_alive(grandchild), "the child ignored SIGTERM");
    kill_pid(grandchild);
}

#[cfg(unix)]
fn process_alive(pid: u32) -> bool {
    // SAFETY: signal 0 only checks that the PID exists.
    unsafe { libc::kill(pid as libc::pid_t, 0) == 0 }
}

#[cfg(unix)]
fn kill_pid(pid: u32) {
    // SAFETY: plain `kill(2)` on a PID this test started.
    unsafe {
        libc::kill(pid as libc::pid_t, libc::SIGKILL);
    }
}

#[cfg(windows)]
fn process_alive(pid: u32) -> bool {
    let output = Command::new("tasklist")
        .args(["/FI", &format!("PID eq {pid}"), "/NH"])
        .output()
        .expect("run tasklist");
    String::from_utf8_lossy(&output.stdout).contains(&pid.to_string())
}

#[cfg(windows)]
fn kill_pid(pid: u32) {
    let _ = Command::new("taskkill").args(["/PID", &pid.to_string(), "/F"]).output();
}
