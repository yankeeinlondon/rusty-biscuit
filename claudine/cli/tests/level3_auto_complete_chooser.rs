//! Level 3 OS-keyboard-injection tests for the runtime autocomplete chooser.
//!
//! Phase 5 of the `2026-06-14-auto-complete` feature. These tests drive
//! `claudine compose` with a missing `file` or `file[]` schema property inside
//! a real, focused WezTerm window and inject genuine macOS keyboard events
//! with `cliclick`. They verify the user-facing input path (Enter submit, Esc
//! cancel, arrow navigation, Space multi-select toggling) rather than the
//! lower-tier bytes-injected-through-the-multiplexer path covered by the L2
//! suite.
//!
//! ## Why these are Level 3
//!
//! `cliclick` synthesises real `CGEventCreateKeyboardEvent` events at the
//! macOS Quartz event layer. WezTerm's own input encoder receives them and
//! emits the escape sequences / bytes that the running `claudine` process
//! finally reads. This is the only test surface that verifies what the
//! terminal actually emits for the user's keypresses.
//!
//! ## Platform gating
//!
//! `cliclick` is the only OS injector wired into the harness today, so the
//! runtime test body is `#[cfg(target_os = "macos")]`. The file still
//! compiles on Linux/Windows (it is simply empty there), matching the pattern
//! used by `level3_wrap_ctrl_c.rs`.
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
use biscuit_test_harness::{capture_settled, cliclick, CapturedFrame, SpawnVisibility, TerminalHarness};
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
const CHOOSER_HINT: &str = "Enter=Submit";

/// Stage a workspace with a `goose` stub, an empty claudine config (so the
/// init wizard does not intercept stdin), a git repo (so file labels are
/// repo-relative), and a prompt file whose schema declares the requested
/// file property.
#[cfg(target_os = "macos")]
fn stage_workspace(property: &str, body: &str) -> TestWorkspace {
    let ws = TestWorkspace::named("auto-complete-l3");
    let bin_dir = ws.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();
    let marker = ws.path().join("launched.flag");
    let prompt_dump = prompt_dump_path(&ws);

    stage_goose_stub(&bin_dir, &marker, &prompt_dump);
    seed_minimal_config(ws.path());
    init_git_repo(ws.path());

    fs::write(
        ws.path().join("readme.md"),
        "---\nname: 'Readme'\ndescription: 'Project readme'\n---\n# Readme\n",
    )
    .unwrap();
    fs::write(ws.path().join("notes.md"), "# Notes\n").unwrap();

    let md_file = ws.path().join("plan.md");
    fs::write(
        &md_file,
        format!("---\n$schema:\n  {property}\n---\n{body}\n"),
    )
    .unwrap();

    ws
}

#[cfg(target_os = "macos")]
fn prompt_dump_path(ws: &TestWorkspace) -> PathBuf {
    ws.path().join("received.prompt")
}

/// A fake `goose` provider that records the `-t` prompt it receives and drops
/// a launch marker. The shell script is on `PATH`, following the established
/// `wrap_*` fixture pattern.
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

/// Read the prompt the fake `goose` provider received via its `-t` flag.
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

/// Launch `claudine compose --goose plan.md` in the harness and wait for the
/// chooser hint to render. Returns the frame that contains the chooser.
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

/// Bring the spawned WezTerm pane to the foreground and click its centre so
/// subsequent OS keyboard events land in the right window.
#[cfg(target_os = "macos")]
fn focus_and_click(harness: &WezTermHarness) -> (i32, i32) {
    let coords = harness
        .focus_spawned_pane()
        .expect("focus spawned WezTerm pane")
        .expect("AXRaise yielded no window coords (non-macOS or AX failure)");
    cliclick::click_at(coords.0, coords.1).expect("click to focus pane");
    std::thread::sleep(Duration::from_millis(200));
    coords
}

/// Poll the harness until the provider launch marker exists and the chained
/// shell sentinel has been printed, proving the command finished.
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

/// Poll the harness until the chained shell sentinel appears, proving the
/// command exited and the shell regained control.
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

/// The active (hovered) chooser row is prefixed with `▶`.
#[cfg(target_os = "macos")]
fn active_line(plain: &str) -> Option<&str> {
    plain.lines().find(|l| l.contains('▶'))
}

/// Count how many items are visibly checked in a `ChooseMany` chooser.
#[cfg(target_os = "macos")]
fn checked_count(plain: &str) -> usize {
    plain.matches('☑').count() + plain.matches('\u{f14a}').count()
}

/// Send the compose command with the required environment, appending an echo
/// sentinel so we can detect when the shell regains control.
#[cfg(target_os = "macos")]
fn send_compose_command(harness: &mut WezTermHarness, ws: &TestWorkspace, sentinel: &str) {
    let claudine = env!("CARGO_BIN_EXE_claudine");
    let home = ws.path().to_string_lossy();
    let path = augmented_path(&ws.path().join("bin"));

    harness
        .send_command_with_env(&format!("cd '{}'", ws.path().display()), &[("HOME", home.as_ref())])
        .expect("cd into workspace");

    let cmd = format!(
        "{claudine} compose --goose {} ; echo {sentinel}",
        ws.path().join("plan.md").display(),
    );
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
        .expect("send compose command");
}

// ---------------------------------------------------------------------------
// Enter submit
// ---------------------------------------------------------------------------

/// A real OS Enter keystroke in a single-select `file` chooser submits the
/// active item and launches the provider.
#[cfg(target_os = "macos")]
#[test]
#[serial(level3_keyboard)]
fn level3_chooser_enter_submits_single_select() {
    require_level!(
        Level::L3,
        WezTermHarness::available() && cliclick::available(),
        "WezTerm + cliclick",
    );

    static SEQ: AtomicU32 = AtomicU32::new(0);
    let sentinel = format!("L3_ENTER_{}", SEQ.fetch_add(1, Ordering::Relaxed));
    let ws = stage_workspace("cover: 'file(required)'", "cover: {{ doc.cover }}");

    let mut harness = WezTermHarness::new()
        .with_spawn_visibility(SpawnVisibility::Foreground)
        .with_expected_window_title("claudine");
    harness.spawn_shell().expect("spawn WezTerm shell pane");

    send_compose_command(&mut harness, &ws, &sentinel);
    let _chooser_frame = wait_for_chooser(&mut harness);
    focus_and_click(&harness);

    cliclick::press("return").expect("inject OS Enter");
    wait_for_submission(&mut harness, &ws.path().join("launched.flag"), &sentinel);

    let prompt = read_recorded_prompt(&ws);
    assert!(
        prompt.starts_with("cover: "),
        "composed prompt must begin with 'cover: '; prompt:\n{prompt}"
    );
    let value = prompt.trim_start_matches("cover: ").trim();
    assert!(
        !value.starts_with('['),
        "single-select file value must be a scalar string, not an array; value: {value}"
    );
    assert!(
        value.contains(".md"),
        "selected file path must appear in the composed prompt; value: {value}"
    );
}

// ---------------------------------------------------------------------------
// Esc cancel
// ---------------------------------------------------------------------------

/// A real OS Esc keystroke in a single-select `file` chooser cancels the
/// prompt and returns to the shell without launching the provider.
#[cfg(target_os = "macos")]
#[test]
#[serial(level3_keyboard)]
fn level3_chooser_esc_cancels_single_select() {
    require_level!(
        Level::L3,
        WezTermHarness::available() && cliclick::available(),
        "WezTerm + cliclick",
    );

    static SEQ: AtomicU32 = AtomicU32::new(0);
    let sentinel = format!("L3_ESC_{}", SEQ.fetch_add(1, Ordering::Relaxed));
    let ws = stage_workspace("cover: 'file(required)'", "cover: {{ doc.cover }}");

    let mut harness = WezTermHarness::new()
        .with_spawn_visibility(SpawnVisibility::Foreground)
        .with_expected_window_title("claudine");
    harness.spawn_shell().expect("spawn WezTerm shell pane");

    send_compose_command(&mut harness, &ws, &sentinel);
    let _chooser_frame = wait_for_chooser(&mut harness);
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

// ---------------------------------------------------------------------------
// Arrow navigation
// ---------------------------------------------------------------------------

/// Real OS Down/Up arrow keys move the active (hovered) item in a `file`
/// chooser.
#[cfg(target_os = "macos")]
#[test]
#[serial(level3_keyboard)]
fn level3_chooser_arrow_navigation_moves_active_item() {
    require_level!(
        Level::L3,
        WezTermHarness::available() && cliclick::available(),
        "WezTerm + cliclick",
    );

    let ws = stage_workspace("cover: 'file(required)'", "cover: {{ doc.cover }}");

    let mut harness = WezTermHarness::new()
        .with_spawn_visibility(SpawnVisibility::Foreground)
        .with_expected_window_title("claudine");
    harness.spawn_shell().expect("spawn WezTerm shell pane");

    send_compose_command(&mut harness, &ws, "unused");
    let _chooser_frame = wait_for_chooser(&mut harness);
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
}

/// Poll until the active chooser line differs from `previous`.
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
// Space toggling for file[]
// ---------------------------------------------------------------------------

/// Real OS Space keys toggle selection in a multi-select `file[]` chooser.
/// Two items are toggled, both show the checked marker, and Enter submits the
/// array to the provider.
#[cfg(target_os = "macos")]
#[test]
#[serial(level3_keyboard)]
fn level3_chooser_space_toggles_multi_select() {
    require_level!(
        Level::L3,
        WezTermHarness::available() && cliclick::available(),
        "WezTerm + cliclick",
    );

    static SEQ: AtomicU32 = AtomicU32::new(0);
    let sentinel = format!("L3_SPACE_{}", SEQ.fetch_add(1, Ordering::Relaxed));
    let ws = stage_workspace(
        "attachments: 'file[](required)'",
        "attachments: {{ doc.attachments }}",
    );

    let mut harness = WezTermHarness::new()
        .with_spawn_visibility(SpawnVisibility::Foreground)
        .with_expected_window_title("claudine");
    harness.spawn_shell().expect("spawn WezTerm shell pane");

    send_compose_command(&mut harness, &ws, &sentinel);
    let _chooser_frame = wait_for_chooser(&mut harness);
    focus_and_click(&harness);

    // Toggle the active item, move down, toggle a second item.
    cliclick::press("space").expect("inject first OS Space");
    std::thread::sleep(Duration::from_millis(150));
    cliclick::press("arrow-down").expect("inject OS Down arrow");
    std::thread::sleep(Duration::from_millis(150));
    cliclick::press("space").expect("inject second OS Space");

    let toggled_frame = wait_for_checked_count(&mut harness, 2, Duration::from_secs(3));
    assert_eq!(
        checked_count(&toggled_frame.plain),
        2,
        "two items must show the checked marker before submit; plain:\n{}",
        toggled_frame.plain
    );

    cliclick::press("return").expect("inject OS Enter to submit");
    wait_for_submission(&mut harness, &ws.path().join("launched.flag"), &sentinel);

    let prompt = read_recorded_prompt(&ws);
    assert!(
        prompt.starts_with("attachments: "),
        "composed prompt must begin with 'attachments: '; prompt:\n{prompt}"
    );
    let value = prompt.trim_start_matches("attachments: ").trim();
    assert!(
        value.starts_with('[') && value.ends_with(']'),
        "multi-select file value must be a JSON array; value: {value}"
    );
    let parsed: serde_json::Value = serde_json::from_str(value)
        .unwrap_or_else(|_| panic!("multi-select value must be valid JSON; value: {value}"));
    let arr = parsed
        .as_array()
        .unwrap_or_else(|| panic!("multi-select file value must be a JSON array; value: {value}"));
    assert_eq!(arr.len(), 2, "must have selected exactly two files; array: {arr:?}");
    for item in arr {
        assert!(item.is_string(), "each selected file must be a string; item: {item}");
    }
}

/// Poll until the captured frame shows exactly `expected` checked items.
#[cfg(target_os = "macos")]
fn wait_for_checked_count(
    harness: &mut WezTermHarness,
    expected: usize,
    deadline: Duration,
) -> CapturedFrame {
    let end = Instant::now() + deadline;
    while Instant::now() < end {
        let frame = harness.capture().expect("capture during toggle wait");
        if checked_count(&frame.plain) == expected {
            return frame;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    let frame = harness.capture().expect("final toggle capture");
    let count = checked_count(&frame.plain);
    panic!(
        "expected {expected} checked items, found {count}; plain:\n{}",
        frame.plain
    );
}
