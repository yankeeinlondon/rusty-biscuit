//! [`LaunchPlan`] against the real `darkmatter-fake-editor` process: the
//! argument vector, stream routing, exit and launch failures, and
//! cancellation stopping the editor's whole process tree.
//!
//! The fake editor's behavior is chosen through `FAKE_EDITOR_*` variables in
//! this process's environment, which the launched editor inherits, so every
//! test is `#[serial]`. A held editor is waited on through its ready marker,
//! never a sleep.

use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use serial_test::serial;
use test_toolkit::argv_recorder::read_argv_records;
use test_toolkit::fake_editor::{
    FAKE_EDITOR_ERR_TEXT, FAKE_EDITOR_EXIT, FAKE_EDITOR_HOLD, FAKE_EDITOR_OUT_TEXT, FAKE_EDITOR_PIDS,
    FAKE_EDITOR_RECORD, FAKE_EDITOR_SPAWN_CHILD,
};
#[cfg(unix)]
use test_toolkit::fake_editor::FAKE_EDITOR_IGNORE_TERM;

use crate::editor_support::{
    ScopedEnv, editor_dir, install_fake_editor, kill_pid, process_alive, resolved, wait_for_file, wait_for_pids,
    wait_until_dead,
};
use darkmatter::editor::{
    EDITOR_TERMINATE_GRACE, EditorError, LaunchIo, LaunchOptions, LaunchOutput, LaunchPlan, LaunchStdin,
};
use sniff::programs::EditorSurface;

fn owned<'a>(io: LaunchIo, cancelled: &'a dyn Fn() -> bool) -> LaunchOptions<'a> {
    LaunchOptions { io, own_tree: true, cancelled: Some(cancelled) }
}

const NULL_IO: LaunchIo = LaunchIo {
    stdin: LaunchStdin::Null,
    stdout: LaunchOutput::Discard,
    stderr: LaunchOutput::Discard,
};

#[test]
#[serial]
fn the_path_is_one_argument_after_the_editor_arguments() {
    let editors = editor_dir();
    let editor = install_fake_editor(editors.path(), "plain-editor");
    let work = tempfile::tempdir().unwrap();
    let record = work.path().join("argv");
    let folder = work.path().join("-dash dir");
    std::fs::create_dir(&folder).unwrap();
    let targets = [
        folder.join("with spaces.md"),
        // Only characters every OS allows in a file name (Windows forbids `|`
        // and `*`).
        folder.join("meta;$(echo nope)&^%'`.md"),
        folder.join("ünïcødé – 文書.md"),
    ];
    let mut env = ScopedEnv::new();
    env.set(FAKE_EDITOR_RECORD, &record);

    for target in &targets {
        std::fs::write(target, "").unwrap();
        LaunchPlan::new(resolved(editor.clone(), None, &["-u", "NONE"]), target)
            .run(owned(NULL_IO, &|| false))
            .unwrap();
    }

    let expected: Vec<Vec<String>> = targets
        .iter()
        .map(|target| vec!["-u".to_string(), "NONE".to_string(), target.display().to_string()])
        .collect();
    assert_eq!(read_argv_records(&record).unwrap(), expected);
}

#[test]
#[serial]
fn a_non_zero_exit_and_a_missing_program_are_typed_failures() {
    let editors = editor_dir();
    let editor = install_fake_editor(editors.path(), "failing-editor");
    let target = editors.path().join("note.md");
    std::fs::write(&target, "").unwrap();
    let mut env = ScopedEnv::new();
    env.set(FAKE_EDITOR_EXIT, "7");

    let error = LaunchPlan::new(resolved(editor, None, &[]), &target).run(owned(NULL_IO, &|| false)).unwrap_err();
    assert!(matches!(error, EditorError::NonZeroExit { code: 7, .. }), "{error}");

    let missing = editors.path().join("no-such-editor");
    let error = LaunchPlan::new(resolved(missing, None, &["--wait"]), &target)
        .run(owned(NULL_IO, &|| false))
        .unwrap_err();
    let EditorError::LaunchFailed { full_command, source, .. } = &error else {
        panic!("{error}");
    };
    assert!(full_command.ends_with("no-such-editor --wait"), "{full_command}");
    assert_eq!(source.kind(), std::io::ErrorKind::NotFound);
}

/// The host's own streams are only observable from outside it, so each case
/// re-runs this test binary on [`route_helper`] as the host and captures that
/// process's stdout and stderr.
#[test]
#[serial]
fn output_goes_where_the_launch_io_says() {
    for (mode, expect_stdout, expect_stderr) in [
        ("inherit", true, true),
        ("to-host-stderr", false, true),
        ("discard", false, false),
    ] {
        let output = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "editor_launcher::route_helper", "--nocapture"])
            .env("EDITOR_ROUTE_MODE", mode)
            .env(FAKE_EDITOR_OUT_TEXT, "editor-said-out")
            .env(FAKE_EDITOR_ERR_TEXT, "editor-said-err")
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(output.status.success(), "{mode}: {stderr}");
        assert!(stdout.contains("route-helper-ran") || stderr.contains("route-helper-ran"), "{mode}: helper did not run\n{stdout}\n{stderr}");
        let out_on_stdout = stdout.contains("editor-said-out");
        let out_on_stderr = stderr.contains("editor-said-out");
        let err_on_stderr = stderr.contains("editor-said-err");
        assert_eq!(out_on_stdout, expect_stdout, "{mode}: editor stdout on host stdout\n{stdout}");
        assert_eq!(out_on_stderr, !expect_stdout && expect_stderr, "{mode}: editor stdout on host stderr\n{stderr}");
        assert_eq!(err_on_stderr, expect_stderr, "{mode}: editor stderr on host stderr\n{stderr}");
        assert!(!stdout.contains("editor-said-err"), "{mode}: editor stderr leaked to stdout");
    }
}

/// The host process for `output_goes_where_the_launch_io_says`; does nothing
/// unless that test started it.
#[test]
fn route_helper() {
    let Ok(mode) = std::env::var("EDITOR_ROUTE_MODE") else {
        return;
    };
    eprintln!("route-helper-ran");
    let (stdout, stderr) = match mode.as_str() {
        "inherit" => (LaunchOutput::Inherit, LaunchOutput::Inherit),
        "to-host-stderr" => (LaunchOutput::ToHostStderr, LaunchOutput::Inherit),
        _ => (LaunchOutput::Discard, LaunchOutput::Discard),
    };
    let editors = editor_dir();
    let editor = install_fake_editor(editors.path(), "talking-editor");
    let target = editors.path().join("note.md");
    std::fs::write(&target, "").unwrap();
    LaunchPlan::new(resolved(editor, None, &[]), &target)
        .run(owned(LaunchIo { stdin: LaunchStdin::Null, stdout, stderr }, &|| false))
        .unwrap();
}

#[test]
#[serial]
fn cancellation_terminates_the_whole_tree_and_reaps_it() {
    let editors = editor_dir();
    let editor = install_fake_editor(editors.path(), "held-editor");
    let target = editors.path().join("note.md");
    std::fs::write(&target, "saved before cancel").unwrap();
    let marker = editors.path().join("ready");
    let pids = editors.path().join("pids");
    let mut env = ScopedEnv::new();
    env.set(FAKE_EDITOR_SPAWN_CHILD, "1").set(FAKE_EDITOR_PIDS, &pids).set(FAKE_EDITOR_HOLD, &marker);

    let cancel = AtomicBool::new(false);
    let (result, child) = std::thread::scope(|scope| {
        let running = scope.spawn(|| {
            LaunchPlan::new(resolved(editor.clone(), Some(EditorSurface::Terminal), &[]), &target)
                .run(owned(NULL_IO, &|| cancel.load(Ordering::SeqCst)))
        });
        wait_for_file(&marker);
        let child = wait_for_pids(&pids, 1)[0];
        assert!(process_alive(child));
        cancel.store(true, Ordering::SeqCst);
        (running.join().unwrap(), child)
    });

    assert!(matches!(result, Err(EditorError::Cancelled { .. })), "{result:?}");
    wait_until_dead(child);
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "saved before cancel");
}

#[cfg(unix)]
#[test]
#[serial]
fn an_editor_ignoring_sigterm_is_killed_after_the_grace_period() {
    let editors = editor_dir();
    let editor = install_fake_editor(editors.path(), "stubborn-editor");
    let target = editors.path().join("note.md");
    std::fs::write(&target, "").unwrap();
    let marker = editors.path().join("ready");
    let pids = editors.path().join("pids");
    let mut env = ScopedEnv::new();
    env.set(FAKE_EDITOR_IGNORE_TERM, "1")
        .set(FAKE_EDITOR_SPAWN_CHILD, "1")
        .set(FAKE_EDITOR_PIDS, &pids)
        .set(FAKE_EDITOR_HOLD, &marker);

    let cancel = AtomicBool::new(false);
    let (result, child, elapsed) = std::thread::scope(|scope| {
        let running = scope.spawn(|| {
            LaunchPlan::new(resolved(editor.clone(), None, &[]), &target)
                .run(owned(NULL_IO, &|| cancel.load(Ordering::SeqCst)))
        });
        wait_for_file(&marker);
        let child = wait_for_pids(&pids, 1)[0];
        let started = Instant::now();
        cancel.store(true, Ordering::SeqCst);
        let result = running.join().unwrap();
        (result, child, started.elapsed())
    });

    assert!(matches!(result, Err(EditorError::Cancelled { .. })), "{result:?}");
    assert!(elapsed >= EDITOR_TERMINATE_GRACE, "SIGKILL came before the grace period: {elapsed:?}");
    assert!(elapsed < EDITOR_TERMINATE_GRACE + Duration::from_secs(5), "{elapsed:?}");
    wait_until_dead(child);
}

/// A GUI editor's waiting client is the only process the launch owns: an
/// editor window that was already running is never touched.
#[test]
#[serial]
fn cancelling_a_gui_client_leaves_an_already_running_editor_alone() {
    let editors = editor_dir();
    let code = install_fake_editor(editors.path(), "code");
    let target = editors.path().join("note.md");
    std::fs::write(&target, "").unwrap();

    let running_marker = editors.path().join("already-running");
    let mut already_running = Command::new(&code)
        .arg(&target)
        .env(FAKE_EDITOR_HOLD, &running_marker)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    wait_for_file(&running_marker);

    let client_marker = editors.path().join("client");
    let mut env = ScopedEnv::new();
    env.set(FAKE_EDITOR_HOLD, &client_marker);
    let cancel = AtomicBool::new(false);
    let result = std::thread::scope(|scope| {
        let running = scope.spawn(|| {
            LaunchPlan::new(resolved(code.clone(), Some(EditorSurface::Gui), &["--wait"]), &target)
                .run(owned(NULL_IO, &|| cancel.load(Ordering::SeqCst)))
        });
        wait_for_file(&client_marker);
        cancel.store(true, Ordering::SeqCst);
        running.join().unwrap()
    });

    assert!(matches!(result, Err(EditorError::Cancelled { .. })), "{result:?}");
    assert!(already_running.try_wait().unwrap().is_none(), "the already-running editor was stopped");
    let _ = already_running.kill();
    let _ = already_running.wait();
}

/// `md edit` and wrapper `--edit` (`own_tree: false`) keep the editor in the
/// host's process group, exactly as before the launcher existed.
#[cfg(unix)]
#[test]
#[serial]
fn a_plain_launch_keeps_the_host_process_group() {
    let editors = editor_dir();
    let editor = install_fake_editor(editors.path(), "plain-editor");
    let target = editors.path().join("note.md");
    std::fs::write(&target, "").unwrap();
    let pids = editors.path().join("pids");
    let mut env = ScopedEnv::new();
    env.set(FAKE_EDITOR_SPAWN_CHILD, "1").set(FAKE_EDITOR_PIDS, &pids);

    LaunchPlan::new(resolved(editor.clone(), None, &[]), &target).run(LaunchOptions::PLAIN).unwrap();
    let plain_child = wait_for_pids(&pids, 1)[0];
    // SAFETY: `getpgid` only queries a process group.
    let (host_group, plain_group) = unsafe { (libc::getpgid(0), libc::getpgid(plain_child as libc::pid_t)) };
    kill_pid(plain_child);
    assert_eq!(plain_group, host_group, "a plain launch must not start a new process group");

    LaunchPlan::new(resolved(editor, None, &[]), &target).run(owned(NULL_IO, &|| false)).unwrap();
    let owned_child = wait_for_pids(&pids, 2)[1];
    // SAFETY: as above.
    let owned_group = unsafe { libc::getpgid(owned_child as libc::pid_t) };
    kill_pid(owned_child);
    assert_ne!(owned_group, host_group, "an owned launch leads its own process group");
}
