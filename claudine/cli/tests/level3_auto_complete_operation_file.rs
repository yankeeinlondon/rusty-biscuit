//! Level 3 OS-keyboard-injection tests for operation-file ENTER-path
//! autocomplete.
//!
//! Phase 5 follow-up for the `2026-06-14-auto-complete` feature. These tests
//! start from an unresolved operation-file partial (`claudine compose plan`)
//! in a real, focused WezTerm window and inject genuine macOS keyboard events
//! with `cliclick`. They verify the single-match confirmation dialog and the
//! multi-match `ChooseOne` chooser at the OS-input layer.
//!
//! ## Platform gating
//!
//! `cliclick` is the only OS injector wired into the harness today, so the
//! runtime test body is `#[cfg(target_os = "macos")]`. The file still
//! compiles on Linux/Windows (it is simply empty there).
//!
//! ## Run
//!
//! ```text
//! just test-l3
//! ```
//!
//! Each test skips cleanly unless `RUN_LEVEL3=1` and both WezTerm
//! (`WEZTERM_UNIX_SOCKET`) and `cliclick` are available.

#[cfg(target_os = "macos")]
mod common;

#[cfg(target_os = "macos")]
use common::{augmented_path, init_git_repo, write_executable, TestWorkspace};
#[cfg(target_os = "macos")]
use common::wrap::seed_minimal_config;

#[cfg(target_os = "macos")]
use biscuit_test_harness::wezterm::WezTermHarness;
#[cfg(target_os = "macos")]
use biscuit_test_harness::{
    capture_settled, cliclick, CapturedFrame, SpawnVisibility, TerminalHarness,
};
#[cfg(target_os = "macos")]
use serial_test::serial;
#[cfg(target_os = "macos")]
use std::fs;
#[cfg(target_os = "macos")]
use std::path::{Path, PathBuf};
#[cfg(target_os = "macos")]
use std::sync::atomic::{AtomicU32, Ordering};
#[cfg(target_os = "macos")]
use std::time::{Duration, Instant};
#[cfg(target_os = "macos")]
use test_toolkit::{require_level, Level};

#[cfg(target_os = "macos")]
const CONFIRMATION_PROMPT: &str = "Use this file? (Y/n)";
#[cfg(target_os = "macos")]
const CHOOSER_HINT: &str = "Enter=Submit";

/// Stage a workspace with a `goose` stub, an empty claudine config, and a
/// git repo so operation-file autocomplete discovers repo-relative prompt
/// files.
#[cfg(target_os = "macos")]
fn stage_workspace(multi_match: bool) -> TestWorkspace {
    let ws = TestWorkspace::named("auto-complete-operation-file-l3");
    let bin_dir = ws.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();
    let marker = ws.path().join("launched.flag");
    let prompt_dump = ws.path().join("received.prompt");

    stage_goose_stub(&bin_dir, &marker, &prompt_dump);
    seed_minimal_config(ws.path());
    init_git_repo(ws.path());

    let prompts = ws.path().join("prompts");
    fs::create_dir_all(&prompts).unwrap();
    fs::write(
        prompts.join("plan.md"),
        "---\nname: 'Plan Alpha'\ndescription: 'First plan'\n---\n# Plan Alpha\n",
    )
    .unwrap();

    if multi_match {
        fs::write(
            prompts.join("planner.md"),
            "---\nname: 'Plan Beta'\ndescription: 'Second plan'\n---\n# Plan Beta\n",
        )
        .unwrap();
    }

    ws
}

/// Stage a workspace with YAML sequence files for the `sequence` operation-file
/// L3 path.
#[cfg(target_os = "macos")]
fn stage_yaml_sequence_workspace(multi_match: bool) -> TestWorkspace {
    let ws = TestWorkspace::named("auto-complete-sequence-yaml-l3");
    let bin_dir = ws.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();
    let marker = ws.path().join("launched.flag");
    let prompt_dump = ws.path().join("received.prompt");

    stage_goose_stub(&bin_dir, &marker, &prompt_dump);
    seed_minimal_config(ws.path());
    init_git_repo(ws.path());

    let prompts = ws.path().join("prompts");
    fs::create_dir_all(&prompts).unwrap();
    fs::write(
        prompts.join("steps.yaml"),
        "name: 'Build Pipeline'\ndescription: 'CI steps'\n$schema:\n  env: 'enum(dev, prod)'\nprompt: 'Process {{state}}'\nsequence:\n  - Plan Alpha\n",
    )
    .unwrap();

    if multi_match {
        fs::write(
            prompts.join("extra_steps.yaml"),
            "name: 'Deploy Flow'\ndescription: 'Release steps'\n$schema:\n  region: 'enum(us, eu)'\nprompt: 'Deploy {{state}}'\nsequence:\n  - Plan Beta\n",
        )
        .unwrap();
    }

    ws
}

#[cfg(target_os = "macos")]
fn prompt_dump_path(ws: &TestWorkspace) -> PathBuf {
    ws.path().join("received.prompt")
}

#[cfg(target_os = "macos")]
fn stage_goose_stub(bin_dir: &Path, marker_file: &Path, prompt_file: &Path) {
    write_executable(
        &bin_dir.join("goose"),
        &format!(
            "#!/bin/sh\n\
             while [ $# -gt 0 ]; do\n\
               case \"$1\" in\n\
                 -t)\n\
                   shift\n\
                   printf '%s\\n' \"$1\" > {}\n\
                   break\n\
                   ;;\n\
               esac\n\
               shift\n\
             done\n\
             echo 'launched' > {}\n\
             exit 0\n",
            prompt_file.display(),
            marker_file.display()
        ),
    );
}

#[cfg(target_os = "macos")]
fn read_recorded_prompt(ws: &TestWorkspace) -> String {
    let path = prompt_dump_path(ws);
    fs::read_to_string(&path).unwrap_or_else(|_| {
        panic!(
            "provider did not record its -t prompt to {}",
            path.display()
        )
    })
}

/// Send the compose command with the unresolved operation-file partial.
#[cfg(target_os = "macos")]
fn send_compose_command(harness: &mut WezTermHarness, ws: &TestWorkspace, sentinel: &str) {
    send_operation_command(harness, ws, "compose", "plan", sentinel);
}

/// Send the sequence command with the unresolved YAML sequence partial.
#[cfg(target_os = "macos")]
fn send_sequence_command(harness: &mut WezTermHarness, ws: &TestWorkspace, sentinel: &str) {
    send_operation_command(harness, ws, "sequence", "steps", sentinel);
}

#[cfg(target_os = "macos")]
fn send_operation_command(
    harness: &mut WezTermHarness,
    ws: &TestWorkspace,
    subcommand: &str,
    partial: &str,
    sentinel: &str,
) {
    let claudine = env!("CARGO_BIN_EXE_claudine");
    let home = ws.path().to_string_lossy();
    let path = augmented_path(&ws.path().join("bin"));

    harness
        .send_command_with_env(
            &format!("cd '{}'", ws.path().display()),
            &[("HOME", home.as_ref())],
        )
        .expect("cd into workspace");

    let cmd = format!("{claudine} {subcommand} --goose {partial} ; echo {sentinel}");
    harness
        .send_command_with_env(
            &cmd,
            &[
                ("HOME", home.as_ref()),
                ("PATH", path.to_str().unwrap_or("/usr/bin")),
                ("TERM", "xterm-256color"),
                ("COLORTERM", "truecolor"),
                ("NO_COLOR", "1"),
            ],
        )
        .expect("send operation command");
}

/// Bring the spawned WezTerm pane to the foreground and click its centre so
/// subsequent OS keyboard events land in the right window.
#[cfg(target_os = "macos")]
fn focus_and_click(harness: &WezTermHarness) -> (i32, i32) {
    let coords = harness
        .focus_spawned_pane()
        .expect("focus spawned WezTerm pane")
        .expect("AXRaise yielded no window coords");
    cliclick::click_at(coords.0, coords.1).expect("click to focus pane");
    std::thread::sleep(Duration::from_millis(200));
    coords
}

#[cfg(target_os = "macos")]
fn wait_for_confirmation(harness: &mut WezTermHarness) -> CapturedFrame {
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut frame = harness.capture().expect("initial capture");
    while Instant::now() < deadline {
        if frame.plain.contains(CONFIRMATION_PROMPT) {
            return frame;
        }
        std::thread::sleep(Duration::from_millis(50));
        frame = harness.capture().expect("poll for confirmation dialog");
    }
    panic!(
        "confirmation dialog never rendered; expected {:?}; plain:\n{}",
        CONFIRMATION_PROMPT, frame.plain
    );
}

#[cfg(target_os = "macos")]
fn wait_for_chooser(harness: &mut WezTermHarness) -> CapturedFrame {
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut frame = harness.capture().expect("initial capture");
    while Instant::now() < deadline {
        if frame.plain.contains(CHOOSER_HINT) {
            return frame;
        }
        std::thread::sleep(Duration::from_millis(50));
        frame = harness.capture().expect("poll for chooser");
    }
    panic!(
        "chooser never rendered; expected hint {:?}; plain:\n{}",
        CHOOSER_HINT, frame.plain
    );
}

#[cfg(target_os = "macos")]
fn wait_for_submission(harness: &mut WezTermHarness, marker: &Path, sentinel: &str) -> CapturedFrame {
    let deadline = Instant::now() + Duration::from_secs(15);
    while Instant::now() < deadline {
        let frame = harness.capture().expect("capture during submission wait");
        if marker.exists() && frame.plain.contains(sentinel) {
            return frame;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    let final_frame = capture_settled(harness).expect("final capture");
    assert!(
        marker.exists(),
        "provider stub did not launch; final frame:\n{}",
        final_frame.plain
    );
    assert!(
        final_frame.plain.contains(sentinel),
        "shell sentinel never appeared; final frame:\n{}",
        final_frame.plain
    );
    final_frame
}

#[cfg(target_os = "macos")]
fn wait_for_shell_return(harness: &mut WezTermHarness, sentinel: &str, deadline: Duration) -> bool {
    let end = Instant::now() + deadline;
    while Instant::now() < end {
        if let Ok(frame) = harness.capture()
            && frame.plain.contains(sentinel)
        {
            return true;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    false
}

#[cfg(target_os = "macos")]
fn active_line(plain: &str) -> Option<&str> {
    plain.lines().find(|l| l.contains('▶'))
}

#[cfg(target_os = "macos")]
fn wait_for_active_change(
    harness: &mut WezTermHarness,
    previous: &str,
    deadline: Duration,
) -> CapturedFrame {
    let end = Instant::now() + deadline;
    while Instant::now() < end {
        std::thread::sleep(Duration::from_millis(50));
        let frame = harness.capture().expect("capture during navigation");
        if let Some(line) = active_line(&frame.plain)
            && line != previous
        {
            return frame;
        }
    }
    let frame = harness.capture().expect("final navigation capture");
    let active = active_line(&frame.plain).unwrap_or("<none>");
    panic!(
        "active item did not change from {previous:?}; final active={active:?}; plain:\n{}",
        frame.plain
    );
}

// ---------------------------------------------------------------------------
// Single-match accept
// ---------------------------------------------------------------------------

/// A real OS Enter keystroke in the single-match confirmation dialog accepts
/// the file and launches the provider.
#[cfg(target_os = "macos")]
#[test]
#[serial(level3_keyboard)]
fn level3_operation_file_enter_accepts_single_match() {
    require_level!(
        Level::L3,
        WezTermHarness::available() && cliclick::available(),
        "WezTerm + cliclick",
    );

    static SEQ: AtomicU32 = AtomicU32::new(0);
    let sentinel = format!("L3_OPFILE_ENTER_{}", SEQ.fetch_add(1, Ordering::Relaxed));
    let ws = stage_workspace(false);

    let mut harness = WezTermHarness::new()
        .with_spawn_visibility(SpawnVisibility::Foreground)
        .with_expected_window_title("claudine");
    harness.spawn_shell().expect("spawn WezTerm shell pane");

    send_compose_command(&mut harness, &ws, &sentinel);
    let _frame = wait_for_confirmation(&mut harness);
    focus_and_click(&harness);

    cliclick::press("return").expect("inject OS Enter");
    wait_for_submission(&mut harness, &ws.path().join("launched.flag"), &sentinel);

    let prompt = read_recorded_prompt(&ws);
    assert!(
        prompt.contains("Plan Alpha"),
        "provider must receive the selected operation file's body; prompt:\n{prompt}"
    );
}

/// A real OS `Y` keystroke in the single-match confirmation dialog accepts the
/// file and launches the provider.
#[cfg(target_os = "macos")]
#[test]
#[serial(level3_keyboard)]
fn level3_operation_file_y_accepts_single_match() {
    require_level!(
        Level::L3,
        WezTermHarness::available() && cliclick::available(),
        "WezTerm + cliclick",
    );

    static SEQ: AtomicU32 = AtomicU32::new(0);
    let sentinel = format!("L3_OPFILE_Y_{}", SEQ.fetch_add(1, Ordering::Relaxed));
    let ws = stage_workspace(false);

    let mut harness = WezTermHarness::new()
        .with_spawn_visibility(SpawnVisibility::Foreground)
        .with_expected_window_title("claudine");
    harness.spawn_shell().expect("spawn WezTerm shell pane");

    send_compose_command(&mut harness, &ws, &sentinel);
    let _frame = wait_for_confirmation(&mut harness);
    focus_and_click(&harness);

    cliclick::type_text("Y").expect("inject OS Y");
    wait_for_submission(&mut harness, &ws.path().join("launched.flag"), &sentinel);

    let prompt = read_recorded_prompt(&ws);
    assert!(
        prompt.contains("Plan Alpha"),
        "provider must receive the selected operation file's body; prompt:\n{prompt}"
    );
}

// ---------------------------------------------------------------------------
// Single-match cancel
// ---------------------------------------------------------------------------

/// A real OS Esc keystroke in the single-match confirmation dialog cancels and
/// returns to the shell without launching the provider.
#[cfg(target_os = "macos")]
#[test]
#[serial(level3_keyboard)]
fn level3_operation_file_esc_cancels_single_match() {
    require_level!(
        Level::L3,
        WezTermHarness::available() && cliclick::available(),
        "WezTerm + cliclick",
    );

    static SEQ: AtomicU32 = AtomicU32::new(0);
    let sentinel = format!("L3_OPFILE_ESC_{}", SEQ.fetch_add(1, Ordering::Relaxed));
    let ws = stage_workspace(false);

    let mut harness = WezTermHarness::new()
        .with_spawn_visibility(SpawnVisibility::Foreground)
        .with_expected_window_title("claudine");
    harness.spawn_shell().expect("spawn WezTerm shell pane");

    send_compose_command(&mut harness, &ws, &sentinel);
    let _frame = wait_for_confirmation(&mut harness);
    focus_and_click(&harness);

    cliclick::press("esc").expect("inject OS Esc");
    let returned = wait_for_shell_return(&mut harness, &sentinel, Duration::from_secs(10));
    let marker = ws.path().join("launched.flag");
    assert!(
        !marker.exists(),
        "provider stub must not launch after Esc cancel"
    );
    assert!(
        returned,
        "shell prompt must return after Esc cancel; sentinel {sentinel} not seen"
    );
}

/// A real OS `n` keystroke in the single-match confirmation dialog cancels and
/// returns to the shell without launching the provider.
#[cfg(target_os = "macos")]
#[test]
#[serial(level3_keyboard)]
fn level3_operation_file_n_cancels_single_match() {
    require_level!(
        Level::L3,
        WezTermHarness::available() && cliclick::available(),
        "WezTerm + cliclick",
    );

    static SEQ: AtomicU32 = AtomicU32::new(0);
    let sentinel = format!("L3_OPFILE_N_{}", SEQ.fetch_add(1, Ordering::Relaxed));
    let ws = stage_workspace(false);

    let mut harness = WezTermHarness::new()
        .with_spawn_visibility(SpawnVisibility::Foreground)
        .with_expected_window_title("claudine");
    harness.spawn_shell().expect("spawn WezTerm shell pane");

    send_compose_command(&mut harness, &ws, &sentinel);
    let _frame = wait_for_confirmation(&mut harness);
    focus_and_click(&harness);

    cliclick::type_text("n").expect("inject OS n");
    let returned = wait_for_shell_return(&mut harness, &sentinel, Duration::from_secs(10));
    let marker = ws.path().join("launched.flag");
    assert!(
        !marker.exists(),
        "provider stub must not launch after n cancel"
    );
    assert!(
        returned,
        "shell prompt must return after n cancel; sentinel {sentinel} not seen"
    );
}

// ---------------------------------------------------------------------------
// Multi-match navigation and selection
// ---------------------------------------------------------------------------

/// Real OS Down/Up arrow keys move the active item in the operation-file
/// chooser, and Enter submits the highlighted file.
#[cfg(target_os = "macos")]
#[test]
#[serial(level3_keyboard)]
fn level3_operation_file_arrow_navigation_selects_in_chooser() {
    require_level!(
        Level::L3,
        WezTermHarness::available() && cliclick::available(),
        "WezTerm + cliclick",
    );

    static SEQ: AtomicU32 = AtomicU32::new(0);
    let sentinel = format!(
        "L3_OPFILE_NAV_{}",
        SEQ.fetch_add(1, Ordering::Relaxed)
    );
    let ws = stage_workspace(true);

    let mut harness = WezTermHarness::new()
        .with_spawn_visibility(SpawnVisibility::Foreground)
        .with_expected_window_title("claudine");
    harness.spawn_shell().expect("spawn WezTerm shell pane");

    send_compose_command(&mut harness, &ws, &sentinel);
    let _frame = wait_for_chooser(&mut harness);
    focus_and_click(&harness);

    let baseline = harness.capture().expect("baseline capture");
    let baseline_active = active_line(&baseline.plain)
        .expect("baseline chooser must show an active item")
        .to_string();
    assert!(
        baseline.plain.contains('▶'),
        "baseline chooser must render an active marker; plain:\n{}",
        baseline.plain
    );

    // Move down to the second candidate.
    cliclick::press("arrow-down").expect("inject OS Down arrow");
    let after_down = wait_for_active_change(&mut harness, &baseline_active, Duration::from_secs(3));
    let down_active = active_line(&after_down.plain)
        .expect("chooser must still show an active item after Down")
        .to_string();
    assert_ne!(
        down_active, baseline_active,
        "Down arrow must move the active item; baseline={baseline_active:?}, after={down_active:?}"
    );

    // Move back up to the first candidate.
    cliclick::press("arrow-up").expect("inject OS Up arrow");
    let after_up = wait_for_active_change(&mut harness, &down_active, Duration::from_secs(3));
    let up_active = active_line(&after_up.plain)
        .expect("chooser must still show an active item after Up")
        .to_string();
    assert_eq!(
        up_active, baseline_active,
        "Up arrow must return the active item to the original row; expected={baseline_active:?}, got={up_active:?}"
    );

    // Move down again and submit the second candidate.
    cliclick::press("arrow-down").expect("inject OS Down arrow");
    let _after_down2 = wait_for_active_change(&mut harness, &up_active, Duration::from_secs(3));

    cliclick::press("return").expect("inject OS Enter to submit");
    wait_for_submission(&mut harness, &ws.path().join("launched.flag"), &sentinel);

    let prompt = read_recorded_prompt(&ws);
    assert!(
        prompt.contains("Plan Beta"),
        "provider must receive the second operation file's body after navigation; prompt:\n{prompt}"
    );
}

// ---------------------------------------------------------------------------
// YAML sequence candidate acceptance and navigation
// ---------------------------------------------------------------------------

/// A real OS Enter keystroke in the single-match YAML sequence confirmation
/// dialog accepts the file and launches the provider.
#[cfg(target_os = "macos")]
#[test]
#[serial(level3_keyboard)]
fn level3_sequence_yaml_enter_accepts_single_match() {
    require_level!(
        Level::L3,
        WezTermHarness::available() && cliclick::available(),
        "WezTerm + cliclick",
    );

    static SEQ: AtomicU32 = AtomicU32::new(0);
    let sentinel = format!(
        "L3_SEQ_YAML_ENTER_{}",
        SEQ.fetch_add(1, Ordering::Relaxed)
    );
    let ws = stage_yaml_sequence_workspace(false);

    let mut harness = WezTermHarness::new()
        .with_spawn_visibility(SpawnVisibility::Foreground)
        .with_expected_window_title("claudine");
    harness.spawn_shell().expect("spawn WezTerm shell pane");

    send_sequence_command(&mut harness, &ws, &sentinel);
    let _frame = wait_for_confirmation(&mut harness);
    focus_and_click(&harness);

    cliclick::press("return").expect("inject OS Enter");
    wait_for_submission(&mut harness, &ws.path().join("launched.flag"), &sentinel);

    let prompt = read_recorded_prompt(&ws);
    assert!(
        prompt.contains("Process Plan Alpha"),
        "provider must receive the rendered YAML sequence prompt; prompt:\n{prompt}"
    );
}

/// A real OS `Y` keystroke in the single-match YAML sequence confirmation
/// dialog accepts the file and launches the provider.
#[cfg(target_os = "macos")]
#[test]
#[serial(level3_keyboard)]
fn level3_sequence_yaml_y_accepts_single_match() {
    require_level!(
        Level::L3,
        WezTermHarness::available() && cliclick::available(),
        "WezTerm + cliclick",
    );

    static SEQ: AtomicU32 = AtomicU32::new(0);
    let sentinel = format!(
        "L3_SEQ_YAML_Y_{}",
        SEQ.fetch_add(1, Ordering::Relaxed)
    );
    let ws = stage_yaml_sequence_workspace(false);

    let mut harness = WezTermHarness::new()
        .with_spawn_visibility(SpawnVisibility::Foreground)
        .with_expected_window_title("claudine");
    harness.spawn_shell().expect("spawn WezTerm shell pane");

    send_sequence_command(&mut harness, &ws, &sentinel);
    let _frame = wait_for_confirmation(&mut harness);
    focus_and_click(&harness);

    cliclick::type_text("Y").expect("inject OS Y");
    wait_for_submission(&mut harness, &ws.path().join("launched.flag"), &sentinel);

    let prompt = read_recorded_prompt(&ws);
    assert!(
        prompt.contains("Process Plan Alpha"),
        "provider must receive the rendered YAML sequence prompt; prompt:\n{prompt}"
    );
}

/// Real OS Down/Up arrow keys move the active item in the YAML sequence
/// chooser, and Enter submits the highlighted file.
#[cfg(target_os = "macos")]
#[test]
#[serial(level3_keyboard)]
fn level3_sequence_yaml_arrow_navigation_selects_in_chooser() {
    require_level!(
        Level::L3,
        WezTermHarness::available() && cliclick::available(),
        "WezTerm + cliclick",
    );

    static SEQ: AtomicU32 = AtomicU32::new(0);
    let sentinel = format!(
        "L3_SEQ_YAML_NAV_{}",
        SEQ.fetch_add(1, Ordering::Relaxed)
    );
    let ws = stage_yaml_sequence_workspace(true);

    let mut harness = WezTermHarness::new()
        .with_spawn_visibility(SpawnVisibility::Foreground)
        .with_expected_window_title("claudine");
    harness.spawn_shell().expect("spawn WezTerm shell pane");

    send_sequence_command(&mut harness, &ws, &sentinel);
    let _frame = wait_for_chooser(&mut harness);
    focus_and_click(&harness);

    let baseline = harness.capture().expect("baseline capture");
    let baseline_active = active_line(&baseline.plain)
        .expect("baseline chooser must show an active item")
        .to_string();
    assert!(
        baseline.plain.contains('▶'),
        "baseline chooser must render an active marker; plain:\n{}",
        baseline.plain
    );

    cliclick::press("arrow-down").expect("inject OS Down arrow");
    let after_down = wait_for_active_change(&mut harness, &baseline_active, Duration::from_secs(3));
    let down_active = active_line(&after_down.plain)
        .expect("chooser must still show an active item after Down")
        .to_string();
    assert_ne!(
        down_active, baseline_active,
        "Down arrow must move the active item; baseline={baseline_active:?}, after={down_active:?}"
    );

    cliclick::press("arrow-up").expect("inject OS Up arrow");
    let after_up = wait_for_active_change(&mut harness, &down_active, Duration::from_secs(3));
    let up_active = active_line(&after_up.plain)
        .expect("chooser must still show an active item after Up")
        .to_string();
    assert_eq!(
        up_active, baseline_active,
        "Up arrow must return the active item to the original row; expected={baseline_active:?}, got={up_active:?}"
    );

    cliclick::press("arrow-down").expect("inject OS Down arrow");
    let _after_down2 = wait_for_active_change(&mut harness, &up_active, Duration::from_secs(3));

    cliclick::press("return").expect("inject OS Enter to submit");
    wait_for_submission(&mut harness, &ws.path().join("launched.flag"), &sentinel);

    let prompt = read_recorded_prompt(&ws);
    assert!(
        prompt.contains("Deploy Plan Beta"),
        "provider must receive the second YAML sequence prompt after navigation; prompt:\n{prompt}"
    );
}
