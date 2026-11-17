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
#[cfg(unix)]
use std::time::{Duration, Instant};

use serial_test::serial;
use test_toolkit::argv_recorder::read_argv_records;
use test_toolkit::fake_editor::{
    FAKE_EDITOR_ERR_TEXT, FAKE_EDITOR_EXIT, FAKE_EDITOR_HOLD, FAKE_EDITOR_OUT_TEXT, FAKE_EDITOR_PIDS,
    FAKE_EDITOR_RECORD, FAKE_EDITOR_SPAWN_CHILD,
};
#[cfg(unix)]
use test_toolkit::fake_editor::FAKE_EDITOR_IGNORE_TERM;

#[cfg(unix)]
use crate::editor_support::kill_pid;
use crate::editor_support::{
    ScopedEnv, editor_dir, install_fake_editor, process_alive, resolved, wait_for_file, wait_for_pids,
    wait_until_dead,
};
#[cfg(unix)]
use darkmatter::editor::EDITOR_TERMINATE_GRACE;
use darkmatter::editor::{EditorError, LaunchIo, LaunchOptions, LaunchOutput, LaunchPlan, LaunchStdin};
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

/// An owned terminal editor that inherits the host's stdin runs in the
/// terminal's foreground (it reads a line typed before it started, which a
/// background group could not do without being stopped), and the host has
/// the foreground back after a successful, an unsuccessful, and a cancelled
/// edit. A second handoff while one is active is refused as `prepare the
/// terminal`. The host must lead a session on a terminal, so each step runs
/// in [`terminal_helper`] on a PTY and writes what it saw to a report.
#[cfg(unix)]
#[test]
#[serial]
fn an_owned_terminal_editor_takes_the_foreground_and_gives_it_back() {
    use std::io::{Read, Write};
    use std::sync::{Arc, Mutex};

    let work = tempfile::tempdir().unwrap();
    let report = work.path().join("report");
    let mut helper = Command::new(std::env::current_exe().unwrap());
    helper
        .args(["--exact", "editor_launcher::terminal_helper", "--nocapture"])
        .env("EDITOR_TERMINAL_REPORT", &report)
        .current_dir(work.path());
    let (mut child, mut terminal) = spawn_on_terminal(helper);
    let transcript = Arc::new(Mutex::new(Vec::new()));
    let mut reader = terminal.try_clone().unwrap();
    let drained = Arc::clone(&transcript);
    // Never joined: whether a PTY read ends once the helper has gone differs
    // between kernels.
    std::thread::spawn(move || {
        let mut chunk = [0u8; 4096];
        while let Ok(read) = reader.read(&mut chunk) {
            if read == 0 {
                break;
            }
            drained.lock().unwrap().extend_from_slice(&chunk[..read]);
        }
    });
    // Queued in the terminal's input until the editor reads it.
    terminal.write_all(b"typed-into-editor\n").unwrap();
    let transcript = || String::from_utf8_lossy(&transcript.lock().unwrap()).into_owned();

    let deadline = Instant::now() + Duration::from_secs(20);
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if Instant::now() > deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!(
                "the helper never finished; report so far:\n{}\ntranscript:\n{}",
                std::fs::read_to_string(&report).unwrap_or_default(),
                transcript()
            );
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    let transcript = transcript();
    let report = std::fs::read_to_string(&report).unwrap_or_default();
    assert!(status.success(), "helper failed\nreport:\n{report}\ntranscript:\n{transcript}");
    assert_eq!(
        report.lines().collect::<Vec<_>>(),
        [
            "contended: prepare the terminal WouldBlock",
            "read: typed-into-editor host-foreground",
            "nonzero: 5 host-foreground",
            "cancelled: host-foreground",
        ],
        "transcript:\n{transcript}"
    );
}

/// Starts `command` as a session leader whose controlling terminal is a new
/// PTY, with all three streams on it, and returns the PTY's master side.
#[cfg(unix)]
fn spawn_on_terminal(mut command: Command) -> (std::process::Child, std::fs::File) {
    use std::ffi::{CStr, OsStr};
    use std::os::fd::FromRawFd;
    use std::os::unix::ffi::OsStrExt;
    use std::os::unix::fs::OpenOptionsExt;
    use std::os::unix::process::CommandExt;

    // SAFETY: `posix_openpt` returns a new descriptor this function owns;
    // `grantpt`/`unlockpt` act on it; `ptsname`'s static buffer is copied at
    // once, and every test here is `#[serial]`.
    let (master, slave_name) = unsafe {
        let master = libc::posix_openpt(libc::O_RDWR | libc::O_NOCTTY);
        assert!(master >= 0, "open a PTY: {}", std::io::Error::last_os_error());
        let master = std::fs::File::from_raw_fd(master);
        let raw = std::os::fd::AsRawFd::as_raw_fd(&master);
        assert_eq!(libc::fcntl(raw, libc::F_SETFD, libc::FD_CLOEXEC), 0);
        assert_eq!(libc::grantpt(raw), 0, "grantpt");
        assert_eq!(libc::unlockpt(raw), 0, "unlockpt");
        let name = libc::ptsname(raw);
        assert!(!name.is_null(), "ptsname");
        (master, OsStr::from_bytes(CStr::from_ptr(name).to_bytes()).to_owned())
    };
    let slave = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .custom_flags(libc::O_NOCTTY)
        .open(&slave_name)
        .expect("open the PTY's terminal side");
    command
        .stdin(slave.try_clone().unwrap())
        .stdout(slave.try_clone().unwrap())
        .stderr(slave);
    // SAFETY: runs in the forked child before `exec` and calls only the
    // async-signal-safe `setsid` and `ioctl`; stdin is already the PTY.
    unsafe {
        command.pre_exec(|| {
            if libc::setsid() < 0 || libc::ioctl(libc::STDIN_FILENO, libc::TIOCSCTTY as _, 0) < 0 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    let child = command.spawn().expect("start the helper on the PTY");
    (child, master)
}

/// The host process for
/// `an_owned_terminal_editor_takes_the_foreground_and_gives_it_back`; does
/// nothing unless that test started it.
#[cfg(unix)]
#[test]
fn terminal_helper() {
    use std::io::Write;
    use sniff::process::{ForegroundHandoff, TreeConfig};
    use test_toolkit::fake_editor::FAKE_EDITOR_READ_LINE;

    let Ok(report) = std::env::var("EDITOR_TERMINAL_REPORT") else {
        return;
    };
    let mut report = std::fs::File::create(report).unwrap();
    let host_foreground = || {
        let terminal = std::fs::File::open("/dev/tty").expect("the helper has a controlling terminal");
        // SAFETY: both calls only query process state through an open descriptor.
        let owner = unsafe { libc::tcgetpgrp(std::os::fd::AsRawFd::as_raw_fd(&terminal)) };
        if owner == unsafe { libc::getpgrp() } { "host-foreground" } else { "host-background" }
    };
    let editors = editor_dir();
    let editor = install_fake_editor(editors.path(), "terminal-editor");
    let target = editors.path().join("note.md");
    std::fs::write(&target, "").unwrap();
    let terminal_editor = || LaunchPlan::new(resolved(editor.clone(), Some(EditorSurface::Terminal), &[]), &target);
    fn with_terminal(cancelled: &dyn Fn() -> bool) -> LaunchOptions<'_> {
        LaunchOptions {
            io: LaunchIo { stdin: LaunchStdin::Inherit, stdout: LaunchOutput::Discard, stderr: LaunchOutput::Discard },
            own_tree: true,
            cancelled: Some(cancelled),
        }
    }

    let mut holder = Command::new("true");
    TreeConfig::process_group().configure(&mut holder);
    let mut held = ForegroundHandoff::prepare(&mut holder).unwrap();
    assert!(held.is_active(), "the helper does not hold the PTY's foreground");
    let error = terminal_editor().run(with_terminal(&|| false)).unwrap_err();
    let EditorError::Io { operation, source } = &error else {
        panic!("{error:?}");
    };
    writeln!(report, "contended: {operation} {:?}", source.kind()).unwrap();
    held.restore().unwrap();
    drop(held);

    let mut env = ScopedEnv::new();
    env.set(FAKE_EDITOR_READ_LINE, "1");
    terminal_editor().run(with_terminal(&|| false)).unwrap();
    let read = std::fs::read_to_string(&target).unwrap();
    writeln!(report, "read: {} {}", read.trim_end(), host_foreground()).unwrap();
    drop(env);

    let mut env = ScopedEnv::new();
    env.set(FAKE_EDITOR_EXIT, "5");
    let error = terminal_editor().run(with_terminal(&|| false)).unwrap_err();
    let EditorError::NonZeroExit { code, .. } = error else {
        panic!("{error:?}");
    };
    writeln!(report, "nonzero: {code} {}", host_foreground()).unwrap();
    drop(env);

    let marker = editors.path().join("held");
    let mut env = ScopedEnv::new();
    env.set(FAKE_EDITOR_HOLD, &marker);
    let error = terminal_editor().run(with_terminal(&|| marker.exists())).unwrap_err();
    assert!(matches!(error, EditorError::Cancelled { .. }), "{error:?}");
    writeln!(report, "cancelled: {}", host_foreground()).unwrap();
    drop(env);
}
