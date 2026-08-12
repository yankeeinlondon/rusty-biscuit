//! Level 3 smoke test for the runtime autocomplete chooser's OS input path.
//!
//! The complete Enter, Y, arrow, Space, confirmation, and YAML sequence
//! behavior matrix lives in the Level 2 PTY suite. This file retains one
//! deliberately narrow macOS proof that a Quartz Enter event passes through
//! WezTerm's input encoder and reaches the chooser.

#![cfg(target_os = "macos")]

mod common;

use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

use biscuit_test_harness::wezterm::WezTermHarness;
use biscuit_test_harness::{
    CapturedFrame, SpawnVisibility, TerminalHarness, capture_settled, cliclick,
};
use common::wrap::seed_minimal_config;
use common::{TestWorkspace, augmented_path, init_git_repo, write_executable};
use serial_test::serial;
use test_toolkit::{Level, require_level};

const CHOOSER_HINT: &str = "Enter=Submit";

fn stage_workspace() -> TestWorkspace {
    let ws = TestWorkspace::named("auto-complete-l3-smoke");
    let bin_dir = ws.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();
    let marker = ws.path().join("launched.flag");
    let prompt_dump = ws.path().join("received.prompt");

    stage_goose_stub(&bin_dir, &marker, &prompt_dump);
    seed_minimal_config(ws.path());
    init_git_repo(ws.path());

    fs::write(ws.path().join("readme.md"), "# Readme\n").unwrap();
    fs::write(ws.path().join("notes.md"), "# Notes\n").unwrap();
    fs::write(
        ws.path().join("plan.md"),
        "---\n$schema:\n  cover: 'file(required)'\n---\ncover: {{ doc.cover }}\n",
    )
    .unwrap();

    ws
}

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

fn send_compose_command(harness: &mut WezTermHarness, ws: &TestWorkspace, sentinel: &str) {
    let claudine = common::claudine_bin();
    let home = ws.path().to_string_lossy();
    let path = augmented_path(&ws.path().join("bin"));

    harness
        .send_command_with_env(
            &format!("cd '{}'", ws.path().display()),
            &[("HOME", home.as_ref())],
        )
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
                ("BISCUIT_TUI_TRACE_KEYS", "1"),
            ],
        )
        .expect("send compose command");
}

fn wait_for_chooser(harness: &mut WezTermHarness) -> CapturedFrame {
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut frame = harness.capture().expect("initial capture");
    while Instant::now() < deadline {
        if frame.plain.contains(CHOOSER_HINT) {
            return frame;
        }
        std::thread::sleep(Duration::from_millis(50));
        frame = harness.capture().expect("poll chooser");
    }
    panic!("chooser never rendered; plain:\n{}", frame.plain);
}

fn wait_for_submission(
    harness: &mut WezTermHarness,
    marker: &Path,
    sentinel: &str,
) -> CapturedFrame {
    let deadline = Instant::now() + Duration::from_secs(15);
    while Instant::now() < deadline {
        let frame = harness.capture().expect("capture submission");
        if marker.exists() && frame.plain.contains(sentinel) {
            return frame;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    let frame = capture_settled(harness).expect("final capture");
    panic!(
        "OS Enter did not complete chooser submission; key trace: {}/biscuit-tui-keys.log; plain:\n{}",
        std::env::temp_dir().display(),
        frame.plain
    );
}

/// A real OS Enter event reaches a focused chooser and submits its active item.
#[test]
#[serial(level3_keyboard)]
fn level3_chooser_enter_submits_single_select() {
    require_level!(
        Level::L3,
        WezTermHarness::available()
            && cliclick::available()
            && cliclick::accessibility_trusted(),
        "WezTerm + cliclick + macOS Accessibility trust",
    );

    let sentinel = "L3_AUTOCOMPLETE_ENTER_SMOKE";
    let ws = stage_workspace();
    let mut harness = WezTermHarness::new()
        .with_spawn_visibility(SpawnVisibility::Foreground)
        .with_expected_window_title("claudine");
    harness.spawn_shell().expect("spawn WezTerm shell pane");

    send_compose_command(&mut harness, &ws, sentinel);
    wait_for_chooser(&mut harness);
    let coordinates = harness
        .focus_spawned_pane()
        .expect("focus spawned WezTerm pane")
        .expect("AXRaise yielded no window coordinates");
    cliclick::click_then_keys(coordinates.0, coordinates.1, &["return"])
        .expect("focus pane and inject OS Enter");

    wait_for_submission(&mut harness, &ws.path().join("launched.flag"), sentinel);
    let prompt = fs::read_to_string(ws.path().join("received.prompt"))
        .expect("provider must record the selected prompt");
    assert!(
        prompt.contains(".md"),
        "selected file path must reach the provider; prompt:\n{prompt}"
    );
}
