//! Windows console Ctrl+C behavior for the wrapper.
//!
//! **Level 1.** This is an ordinary test, gated by `#[cfg(windows)]` exactly like
//! `wrap_sigint.rs` is gated by `#[cfg(unix)]`: it runs on the Windows leg of the
//! normal test matrix and nowhere else. The interrupt is synthesized with the
//! `GenerateConsoleCtrlEvent` Win32 call, which needs no terminal harness and no
//! keyboard injection, so it is neither Level 2 nor Level 3 — the same reasoning
//! `sequence_ctrl_c_windows.rs` records.
//!
//! It previously lived in `level3_wrap_ctrl_c.rs` as an `#[ignore]`d test, which
//! made it unreachable by every canonical recipe (`just test` filters out
//! `level3_`, `just test-l2` selects only `level2_`, and `just test-l3` neither
//! runs unattended nor runs ignored tests) and required a bespoke CI workflow to
//! invoke it by name. That workflow never passed and has been removed.

/// Windows console Ctrl+C integration test: a console control event delivered
/// to the wrapped child's process group must terminate it, mirroring the Unix
/// `wrap_sigint.rs` proof.
///
/// Builds a Windows-executable fake `opencode` provider (a `.cmd` batch on
/// `PATH`) that emits one init line, drops a readiness marker, then loops
/// forever without trapping `CTRL_BREAK`. It spawns the real `claudine compose
/// --opencode` wrapper in its **own** process group, polls for the marker (so
/// the wrapped grandchild is in its run loop and the console handler is
/// installed), injects `GenerateConsoleCtrlEvent(CTRL_BREAK_EVENT, child_pid)`,
/// and asserts the wrapper child exits within a deadline.
///
/// ## Why `CTRL_BREAK_EVENT` (not `CTRL_C_EVENT`)
///
/// `GenerateConsoleCtrlEvent` can target a specific process group only with
/// `CTRL_BREAK_EVENT`; `CTRL_C_EVENT` ignores the group argument and is sent to
/// every process sharing the console. Sending Ctrl+Break to the child's group
/// is the genuine, scoped, user-equivalent interrupt: it reaches the wrapped
/// child (which is its own group leader via `CREATE_NEW_PROCESS_GROUP`) and
/// drives the wrapper's `claudine_console_ctrl_handler` → escalation ladder.
///
/// ## Why the wrapper child runs in its own process group
///
/// The event is addressed by process-group id. If the wrapper child shared the
/// test runner's console group, the event would also be delivered to the test
/// process itself and could tear down the harness. Spawning `claudine` with
/// `CREATE_NEW_PROCESS_GROUP` isolates it so the event hits only the wrapper
/// subtree. (This is also what production does — `spawn.rs` sets the same flag.)
///
#[cfg(windows)]
#[test]
fn ctrl_c_terminates_wrapped_child_on_windows() {
    use std::fs;
    use std::os::windows::process::CommandExt;
    use std::process::Command;
    use std::time::{Duration, Instant};

    use tempfile::tempdir;
    use windows::Win32::System::Console::{
        CTRL_BREAK_EVENT, GenerateConsoleCtrlEvent,
    };
    use windows::Win32::System::Threading::CREATE_NEW_PROCESS_GROUP;

    let workspace = tempdir().expect("tempdir");
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).expect("create bin dir");

    // Minimal config so the wrapper's startup config load succeeds — the
    // Windows arm cannot use the `#[cfg(target_os = "macos")]` `common`
    // helpers, so seed the same `{}` config inline.
    let claudine_dir = workspace.path().join(".claudine");
    fs::create_dir_all(&claudine_dir).expect("create .claudine");
    fs::write(claudine_dir.join("config.json"), "{}").expect("seed config");

    let md_file = workspace.path().join("run.md");
    fs::write(&md_file, "---\ntitle: win\nmodel: test-model\n---\nBody\n").expect("write md");

    let ready_marker = workspace.path().join("opencode-started");

    // Windows-executable fake `opencode`, mirroring the Unix shell-script fake:
    // `models` returns the catalog (so model-validation resolves without a
    // network call), the run path prints one init JSON line, touches the
    // readiness marker, then loops forever. PATH resolution finds `opencode.cmd`
    // via `PATHEXT`. The `:loop` body is interruptible — the batch interpreter
    // receives the console event and the wrapper's escalation (CTRL_BREAK then
    // TerminateJobObject) tears the whole Job tree down. It does NOT trap
    // CTRL_BREAK, so the wrapper-driven termination path runs.
    // Each batch line starts at column 0 (no Rust line-continuation leading
    // whitespace): `cmd.exe` label resolution (`:loop` / `goto loop`) is
    // sensitive to indentation on some interpreters, so the lines are joined
    // explicitly rather than via `\`-continuation.
    let opencode_cmd = path_dir.join("opencode.cmd");
    let cmd_body = [
        "@echo off",
        "if \"%~1\"==\"models\" (",
        "echo [\"test-model\"]",
        "exit /b 0",
        ")",
        "echo {\"type\":\"init\",\"session_id\":\"win-ctrl-c\",\"model\":\"test-model\"}",
        "type nul > \"%CLAUDINE_READY_MARKER%\"",
        ":loop",
        "timeout /t 1 >nul",
        "goto loop",
        "",
    ]
    .join("\r\n");
    fs::write(&opencode_cmd, cmd_body).expect("write opencode.cmd");

    // PATH with the fake-provider dir first so `opencode` resolves to our `.cmd`.
    let system_path = std::env::var_os("PATH").unwrap_or_default();
    let mut path_entries = vec![path_dir.clone()];
    path_entries.extend(std::env::split_paths(&system_path));
    let augmented = std::env::join_paths(path_entries).expect("join_paths");

    let claudine = env!("CARGO_BIN_EXE_claudine");

    // Spawn the wrapper in its OWN process group so the console event we send
    // targets only the wrapper subtree, never this test runner. Anchor CWD to
    // the small temp workspace so the wrapper's repo detection stays bounded.
    let mut child = Command::new(claudine)
        .arg("compose")
        .arg("--opencode")
        .arg(&md_file)
        .current_dir(workspace.path())
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("USERPROFILE", workspace.path())
        .env("PATH", &augmented)
        .env("OPENCODE_MODEL", "test-model")
        .env("CLAUDINE_READY_MARKER", &ready_marker)
        .creation_flags(CREATE_NEW_PROCESS_GROUP.0)
        .spawn()
        .expect("spawn claudine wrapper");

    // Poll for the readiness marker: proves the wrapped grandchild reached its
    // run loop, which is strictly after the wrapper installed the console
    // handler. Injecting before that would race the handler.
    let marker_deadline = Instant::now() + Duration::from_secs(30);
    while !ready_marker.exists() {
        if Instant::now() >= marker_deadline {
            let _ = child.kill();
            panic!("wrapped child never reached its run loop within 30s");
        }
        std::thread::sleep(Duration::from_millis(50));
    }

    // Genuine console interrupt: Ctrl+Break to the wrapper child's process
    // group (it leads its own group). The wrapper's console handler counts the
    // press and escalates to terminate the Job Object tree.
    let child_pid = child.id();
    let sent = unsafe { GenerateConsoleCtrlEvent(CTRL_BREAK_EVENT, child_pid) };
    assert!(
        sent.is_ok(),
        "GenerateConsoleCtrlEvent(CTRL_BREAK_EVENT, {child_pid}) failed: {sent:?}",
    );

    // Assert the wrapper child terminates within the interrupt window. Waiting
    // on the spawned `Child` and observing its exit is the Windows analogue of
    // the Unix sentinel/return-to-prompt proof.
    let term_deadline = Instant::now() + Duration::from_secs(15);
    let mut exited = false;
    while Instant::now() < term_deadline {
        match child.try_wait() {
            Ok(Some(_status)) => {
                exited = true;
                break;
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(100)),
            Err(e) => panic!("try_wait failed: {e}"),
        }
    }

    if !exited {
        let _ = child.kill();
    }
    assert!(
        exited,
        "console Ctrl+Break to the wrapped child's process group must terminate \
         the wrapper child within 15s (Job-Object / CTRL_BREAK_EVENT path)",
    );
}