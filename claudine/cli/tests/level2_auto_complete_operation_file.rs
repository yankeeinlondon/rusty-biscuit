//! Level 2 real-terminal tests for operation-file ENTER-path autocomplete.
//!
//! Phase 5 follow-up for the `2026-06-14-auto-complete` feature. These tests
//! start from an unresolved operation-file partial (`claudine compose plan`)
//! rather than a missing schema property, and verify the two presentation
//! paths mandated by the spec:
//!
//! - Single match → lightweight `Use this file? (Y/n)` confirmation dialog.
//! - Multiple matches → `ChooseOne` two-pane chooser with live detail pane.
//!
//! Gating: `#![cfg(unix)]`, `require_level!(Level::L2, ...)` so the tests skip
//! cleanly when the backend is unavailable and panic under
//! `BISCUIT_TEST_LEVEL_REQUIRED=2`.
//!
//! Run via the canonical recipe:
//!
//! ```text
//! just test-l2
//! ```

#![cfg(unix)]

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use biscuit_test_harness::tmux::TmuxHarness;
use biscuit_test_harness::wezterm::WezTermHarness;
use biscuit_test_harness::{CapturedFrame, TerminalHarness};
use serial_test::serial;
use test_toolkit::{Level, require_level};

mod common;
use common::{TestWorkspace, augmented_path, init_git_repo, write_executable};

const WIDE_COLUMNS: u32 = 120;
const WIDE_LINES: u32 = 24;
const TALL_COLUMNS: u32 = 30;
const TALL_LINES: u32 = 50;
const CONFIRMATION_PROMPT: &str = "Use this file? (Y/n)";
const CHOOSER_HINT: &str = "Enter=Submit";

/// Backend-specific byte/key injection for the small set of keys used by
/// these L2 tests.
trait KeySender: TerminalHarness {
    fn send_esc(&mut self) -> io::Result<()>;
}

impl KeySender for TmuxHarness {
    fn send_esc(&mut self) -> io::Result<()> {
        self.send_key("Escape")
    }
}

impl KeySender for WezTermHarness {
    fn send_esc(&mut self) -> io::Result<()> {
        self.send_text(b"\x1b")
    }
}

/// Stage a workspace with a `goose` stub, an empty claudine config, and a
/// git repo so operation-file autocomplete discovers repo-relative prompt
/// files.
fn stage_workspace(single_match: bool) -> TestWorkspace {
    let ws = TestWorkspace::named("auto-complete-operation-file");
    let bin_dir = ws.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();
    let marker = ws.path().join("launched.flag");
    let prompt_dump = ws.path().join("received.prompt");

    stage_goose_stub(&bin_dir, &marker, &prompt_dump);
    stage_default_config(ws.path());
    init_git_repo(ws.path());

    let prompts = ws.path().join("prompts");
    fs::create_dir_all(&prompts).unwrap();
    fs::write(
        prompts.join("plan.md"),
        "---\nname: 'Plan Alpha'\ndescription: 'First plan'\n---\n# Plan Alpha\n",
    )
    .unwrap();

    if !single_match {
        fs::write(
            prompts.join("planner.md"),
            "---\nname: 'Plan Beta'\ndescription: 'Second plan'\n---\n# Plan Beta\n",
        )
        .unwrap();
    }

    ws
}

/// Stage a workspace with YAML sequence files for the `sequence` operation-file
/// autocomplete path.
fn stage_yaml_sequence_workspace(single_match: bool) -> TestWorkspace {
    let ws = TestWorkspace::named("auto-complete-sequence-yaml");
    let bin_dir = ws.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();
    let marker = ws.path().join("launched.flag");
    let prompt_dump = ws.path().join("received.prompt");

    stage_goose_stub(&bin_dir, &marker, &prompt_dump);
    stage_default_config(ws.path());
    init_git_repo(ws.path());

    let prompts = ws.path().join("prompts");
    fs::create_dir_all(&prompts).unwrap();
    fs::write(
        prompts.join("steps.yaml"),
        "name: 'Build Pipeline'\ndescription: 'CI steps'\n$schema:\n  env: 'enum(dev, prod)'\nsequence:\n  - one\n",
    )
    .unwrap();

    if !single_match {
        fs::write(
            prompts.join("extra_steps.yaml"),
            "name: 'Deploy Flow'\ndescription: 'Release steps'\n$schema:\n  region: 'enum(us, eu)'\nsequence:\n  - two\n",
        )
        .unwrap();
    }

    ws
}

fn stage_default_config(home_dir: &Path) {
    let claudine_dir = home_dir.join(".claudine");
    fs::create_dir_all(&claudine_dir).unwrap();
    fs::write(claudine_dir.join("config.json"), "{}").unwrap();
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

fn claudine_bin() -> String {
    std::env::current_exe()
        .ok()
        .and_then(|exe| {
            let mut dir = exe.parent()?.to_path_buf();
            if dir.file_name()?.to_str()? == "deps" {
                dir = dir.parent()?.to_path_buf();
            }
            dir.join("claudine")
                .exists()
                .then(|| dir.join("claudine").display().to_string())
        })
        .unwrap_or_else(|| "claudine".to_string())
}

/// Send the compose command with the unresolved operation-file partial.
fn send_compose_command(harness: &mut impl TerminalHarness, ws: &TestWorkspace) {
    send_operation_command(harness, ws, "compose", "plan");
}

fn send_sequence_command(harness: &mut impl TerminalHarness, ws: &TestWorkspace) {
    send_operation_command(harness, ws, "sequence", "steps");
}

fn send_operation_command(
    harness: &mut impl TerminalHarness,
    ws: &TestWorkspace,
    subcommand: &str,
    partial: &str,
) {
    let claudine = claudine_bin();
    let home = ws.path().to_string_lossy();
    let path = augmented_path(&ws.path().join("bin"));
    let done_flag = ws.path().join("shell_done.flag").display().to_string();

    harness
        .send_command_with_env(
            &format!("cd '{}'", ws.path().display()),
            &[("HOME", home.as_ref())],
        )
        .expect("cd into workspace");

    let cmd = format!("{claudine} {subcommand} --goose {partial}; touch '{done_flag}'");
    harness
        .send_command_with_env(
            &cmd,
            &[
                ("HOME", home.as_ref()),
                ("PATH", path.to_str().unwrap_or("/usr/bin")),
                ("TERM", "xterm-256color"),
                ("COLORTERM", "truecolor"),
            ],
        )
        .expect("send operation command");
}

fn shell_done_flag(ws: &TestWorkspace) -> PathBuf {
    ws.path().join("shell_done.flag")
}

/// Drive a single-match operation-file partial to the confirmation dialog,
/// capture it, then cancel with `Esc` so the process exits.
fn drive_confirmation<H: KeySender>(harness: &mut H, ws: &TestWorkspace) -> CapturedFrame {
    send_compose_command(harness, ws);

    let deadline = Instant::now() + Duration::from_secs(10);
    let mut frame = harness.capture().expect("initial capture");
    while Instant::now() < deadline {
        if frame.plain.contains(CONFIRMATION_PROMPT) {
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
        frame = harness.capture().expect("wait for confirmation dialog");
    }
    assert!(
        frame.plain.contains(CONFIRMATION_PROMPT),
        "confirmation dialog never rendered; plain:\n{}",
        frame.plain
    );

    let captured = harness.capture().expect("capture confirmation dialog");

    // Cancel so the test process returns and nextest does not flag a leak.
    harness.send_esc().expect("send Esc to cancel");
    let _ = wait_for_shell_return(harness, ws, Duration::from_secs(10));

    captured
}

/// Drive a multi-match operation-file partial to the chooser, capture it,
/// then cancel with `Esc` so the process exits.
fn drive_chooser<H: KeySender>(harness: &mut H, ws: &TestWorkspace) -> CapturedFrame {
    send_compose_command(harness, ws);

    let deadline = Instant::now() + Duration::from_secs(10);
    let mut frame = harness.capture().expect("initial capture");
    while Instant::now() < deadline {
        if frame.plain.contains(CHOOSER_HINT) {
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
        frame = harness.capture().expect("wait for chooser");
    }
    assert!(
        frame.plain.contains(CHOOSER_HINT),
        "chooser never rendered; plain:\n{}",
        frame.plain
    );

    let captured = harness.capture().expect("capture chooser");

    // Cancel so the test process returns and nextest does not flag a leak.
    harness.send_esc().expect("send Esc to cancel");
    let _ = wait_for_shell_return(harness, ws, Duration::from_secs(10));

    captured
}

fn drive_yaml_confirmation<H: KeySender>(harness: &mut H, ws: &TestWorkspace) -> CapturedFrame {
    send_sequence_command(harness, ws);

    let deadline = Instant::now() + Duration::from_secs(10);
    let mut frame = harness.capture().expect("initial capture");
    while Instant::now() < deadline {
        if frame.plain.contains(CONFIRMATION_PROMPT) {
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
        frame = harness.capture().expect("wait for yaml confirmation dialog");
    }
    assert!(
        frame.plain.contains(CONFIRMATION_PROMPT),
        "yaml confirmation dialog never rendered; plain:\n{}",
        frame.plain
    );

    let captured = harness.capture().expect("capture yaml confirmation dialog");

    harness.send_esc().expect("send Esc to cancel");
    let _ = wait_for_shell_return(harness, ws, Duration::from_secs(10));

    captured
}

fn drive_yaml_chooser<H: KeySender>(harness: &mut H, ws: &TestWorkspace) -> CapturedFrame {
    send_sequence_command(harness, ws);

    let deadline = Instant::now() + Duration::from_secs(10);
    let mut frame = harness.capture().expect("initial capture");
    while Instant::now() < deadline {
        if frame.plain.contains(CHOOSER_HINT) {
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
        frame = harness.capture().expect("wait for yaml chooser");
    }
    assert!(
        frame.plain.contains(CHOOSER_HINT),
        "yaml chooser never rendered; plain:\n{}",
        frame.plain
    );

    let captured = harness.capture().expect("capture yaml chooser");

    harness.send_esc().expect("send Esc to cancel");
    let _ = wait_for_shell_return(harness, ws, Duration::from_secs(10));

    captured
}

fn wait_for_shell_return(_harness: &mut impl TerminalHarness, ws: &TestWorkspace, deadline: Duration) -> bool {
    let flag = shell_done_flag(ws);
    let end = Instant::now() + deadline;
    while Instant::now() < end {
        if flag.exists() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    false
}

fn has_chooser_markers(plain: &str) -> bool {
    plain.contains('○')
        || plain.contains('☐')
        || plain.contains('▶')
        || plain.contains('☑')
        || plain.contains('\u{f043e}')
        || plain.contains('\u{f4aa}')
        || plain.contains('\u{f0131}')
        || plain.contains('\u{f14a}')
}

fn is_list_line(line: &str) -> bool {
    line.contains('○')
        || line.contains('☐')
        || line.contains('▶')
        || line.contains('☑')
        || line.contains('\u{f043e}')
        || line.contains('\u{f4aa}')
        || line.contains('\u{f0131}')
        || line.contains('\u{f14a}')
}

fn active_line(plain: &str) -> Option<&str> {
    plain.lines().find(|l| l.contains('▶'))
}

fn detail_marker(plain: &str) -> bool {
    plain.contains("Schema:")
        || plain.contains("no description")
        || plain.contains("no schema defined")
        || plain.contains("Plan Alpha")
        || plain.contains("Plan Beta")
        || plain.contains("SEQUENCE")
        || plain.contains("Build Pipeline")
        || plain.contains("Deploy Flow")
        || plain.contains("CI steps")
        || plain.contains("Release steps")
}

fn assert_confirmation_dialog(frame: &CapturedFrame) {
    let plain = &frame.plain;
    assert!(
        plain.contains(CONFIRMATION_PROMPT),
        "confirmation dialog must show '{CONFIRMATION_PROMPT}'; plain:\n{plain}"
    );
    assert!(
        !has_chooser_markers(plain),
        "confirmation dialog must not render chooser markers; plain:\n{plain}"
    );
    assert!(
        !plain.contains(CHOOSER_HINT),
        "confirmation dialog must not render chooser hint; plain:\n{plain}"
    );
}

fn assert_choose_one_markers(frame: &CapturedFrame) {
    let plain = &frame.plain;
    let has_radio = plain.contains('○')
        || plain.contains('\u{f043e}')
        || plain.contains('\u{f4aa}');
    assert!(
        has_radio,
        "ChooseOne must render a radio marker; plain:\n{plain}"
    );
    let has_checkbox = plain.contains('☐')
        || plain.contains('☑')
        || plain.contains('\u{f0131}')
        || plain.contains('\u{f14a}');
    assert!(
        !has_checkbox,
        "ChooseOne must not render checkbox markers; plain:\n{plain}"
    );
}

fn assert_active_detail_matches(frame: &CapturedFrame) {
    let active = active_line(&frame.plain).unwrap_or("");
    // The active list item's file name should surface in the detail pane.
    let expected = if active.contains("planner") {
        "Plan Beta"
    } else {
        "Plan Alpha"
    };
    assert!(
        frame.plain.contains(expected),
        "detail pane must derive from the active item; expected {expected:?}; plain:\n{}",
        frame.plain
    );
}

fn assert_wide_layout(frame: &CapturedFrame) {
    let has_side_by_side = frame
        .plain
        .lines()
        .any(|line| is_list_line(line) && detail_marker(line));
    assert!(
        has_side_by_side,
        "wide terminal must render list and detail side-by-side; plain:\n{}",
        frame.plain
    );
}

fn assert_tall_layout(frame: &CapturedFrame) {
    let lines: Vec<&str> = frame.plain.lines().collect();
    let first_detail = lines.iter().position(|l| detail_marker(l));
    let first_list = lines.iter().position(|l| is_list_line(l));
    assert!(
        matches!((first_detail, first_list), (Some(d), Some(c)) if d < c),
        "tall terminal must render detail above the candidate list; plain:\n{}",
        frame.plain
    );
}

fn assert_yaml_sequence_detail(frame: &CapturedFrame, expected_name: &str, expected_description: &str) {
    let plain = &frame.plain;
    assert!(
        plain.contains("Sequence"),
        "yaml sequence detail must show Sequence badge; plain:\n{plain}"
    );
    assert!(
        plain.contains(expected_name),
        "yaml sequence detail must show name {expected_name:?}; plain:\n{plain}"
    );
    assert!(
        plain.contains(expected_description),
        "yaml sequence detail must show description {expected_description:?}; plain:\n{plain}"
    );
}

fn assert_yaml_active_detail_matches(frame: &CapturedFrame) {
    let active = active_line(&frame.plain).unwrap_or("");
    let expected = if active.contains("extra") {
        "Deploy Flow"
    } else {
        "Build Pipeline"
    };
    assert!(
        frame.plain.contains(expected),
        "detail pane must derive from the active yaml item; expected {expected:?}; plain:\n{}",
        frame.plain
    );
}

fn run_yaml_single_match_test<H: KeySender>(harness: &mut H) {
    let ws = stage_yaml_sequence_workspace(true);
    let frame = drive_yaml_confirmation(harness, &ws);
    assert_confirmation_dialog(&frame);
    assert_yaml_sequence_detail(&frame, "Build Pipeline", "CI steps");
}

fn run_yaml_multi_match_test<H: KeySender>(harness: &mut H) {
    let ws = stage_yaml_sequence_workspace(false);
    let frame = drive_yaml_chooser(harness, &ws);
    assert_choose_one_markers(&frame);
    assert_yaml_active_detail_matches(&frame);
}

fn run_yaml_wide_layout_test<H: KeySender>(harness: &mut H) {
    let ws = stage_yaml_sequence_workspace(false);
    let frame = drive_yaml_chooser(harness, &ws);
    assert_choose_one_markers(&frame);
    assert_wide_layout(&frame);
    assert_yaml_active_detail_matches(&frame);
}

fn run_yaml_tall_layout_test<H: KeySender>(harness: &mut H) {
    let ws = stage_yaml_sequence_workspace(false);
    let frame = drive_yaml_chooser(harness, &ws);
    assert_choose_one_markers(&frame);
    assert_tall_layout(&frame);
    assert_yaml_active_detail_matches(&frame);
}

// ----------------------------------------------------------------------
// Single-match confirmation dialog
// ----------------------------------------------------------------------

fn run_single_match_test<H: KeySender>(harness: &mut H) {
    let ws = stage_workspace(true);
    let frame = drive_confirmation(harness, &ws);
    assert_confirmation_dialog(&frame);
}

// ----------------------------------------------------------------------
// Multi-match two-pane chooser
// ----------------------------------------------------------------------

fn run_multi_match_test<H: KeySender>(harness: &mut H) {
    let ws = stage_workspace(false);
    let frame = drive_chooser(harness, &ws);
    assert_choose_one_markers(&frame);
    assert_active_detail_matches(&frame);
}

fn run_wide_layout_test<H: KeySender>(harness: &mut H) {
    let ws = stage_workspace(false);
    let frame = drive_chooser(harness, &ws);
    assert_choose_one_markers(&frame);
    assert_wide_layout(&frame);
    assert_active_detail_matches(&frame);
}

fn run_tall_layout_test<H: KeySender>(harness: &mut H) {
    let ws = stage_workspace(false);
    let frame = drive_chooser(harness, &ws);
    assert_choose_one_markers(&frame);
    assert_tall_layout(&frame);
    assert_active_detail_matches(&frame);
}

// ----------------------------------------------------------------------
// tmux backend
// ----------------------------------------------------------------------

#[test]
#[serial(level2_terminal)]
fn level2_tmux_operation_file_single_match_shows_confirmation() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");
    let mut harness = TmuxHarness::new();
    harness.spawn_shell().expect("tmux harness");
    harness.resize(80, 24).ok();
    run_single_match_test(&mut harness);
}

#[test]
#[serial(level2_terminal)]
fn level2_tmux_operation_file_multi_match_uses_choose_one() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");
    let mut harness = TmuxHarness::new();
    harness.spawn_shell().expect("tmux harness");
    harness.resize(80, 24).ok();
    run_multi_match_test(&mut harness);
}

#[test]
#[serial(level2_terminal)]
fn level2_tmux_operation_file_chooser_detail_right_in_wide_terminal() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");
    let mut harness = TmuxHarness::new();
    harness.spawn_shell().expect("tmux harness");
    harness
        .resize(WIDE_COLUMNS, WIDE_LINES)
        .expect("resize tmux pane wide");
    run_wide_layout_test(&mut harness);
}

#[test]
#[serial(level2_terminal)]
fn level2_tmux_operation_file_chooser_detail_above_in_tall_terminal() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");
    let mut harness = TmuxHarness::new();
    harness.spawn_shell().expect("tmux harness");
    harness
        .resize(TALL_COLUMNS, TALL_LINES)
        .expect("resize tmux pane tall");
    run_tall_layout_test(&mut harness);
}

#[test]
#[serial(level2_terminal)]
fn level2_tmux_sequence_yaml_single_match_shows_confirmation() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");
    let mut harness = TmuxHarness::new();
    harness.spawn_shell().expect("tmux harness");
    harness.resize(80, 24).ok();
    run_yaml_single_match_test(&mut harness);
}

#[test]
#[serial(level2_terminal)]
fn level2_tmux_sequence_yaml_multi_match_uses_choose_one() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");
    let mut harness = TmuxHarness::new();
    harness.spawn_shell().expect("tmux harness");
    harness.resize(80, 24).ok();
    run_yaml_multi_match_test(&mut harness);
}

#[test]
#[serial(level2_terminal)]
fn level2_tmux_sequence_yaml_chooser_detail_right_in_wide_terminal() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");
    let mut harness = TmuxHarness::new();
    harness.spawn_shell().expect("tmux harness");
    harness
        .resize(WIDE_COLUMNS, WIDE_LINES)
        .expect("resize tmux pane wide");
    run_yaml_wide_layout_test(&mut harness);
}

#[test]
#[serial(level2_terminal)]
fn level2_tmux_sequence_yaml_chooser_detail_above_in_tall_terminal() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");
    let mut harness = TmuxHarness::new();
    harness.spawn_shell().expect("tmux harness");
    harness
        .resize(TALL_COLUMNS, TALL_LINES)
        .expect("resize tmux pane tall");
    run_yaml_tall_layout_test(&mut harness);
}

// ----------------------------------------------------------------------
// WezTerm backend
// ----------------------------------------------------------------------

#[test]
#[serial(level2_terminal)]
fn level2_wezterm_operation_file_single_match_shows_confirmation() {
    require_level!(
        Level::L2,
        WezTermHarness::available(),
        "WezTerm CLI (set WEZTERM_UNIX_SOCKET)",
    );
    let mut harness = WezTermHarness::shared_or_spawn().expect("attach/spawn WezTerm");
    run_single_match_test(&mut harness);
}

#[test]
#[serial(level2_terminal)]
fn level2_wezterm_operation_file_multi_match_uses_choose_one() {
    require_level!(
        Level::L2,
        WezTermHarness::available(),
        "WezTerm CLI (set WEZTERM_UNIX_SOCKET)",
    );
    let mut harness = WezTermHarness::shared_or_spawn().expect("attach/spawn WezTerm");
    run_multi_match_test(&mut harness);
}

#[test]
#[serial(level2_terminal)]
fn level2_wezterm_sequence_yaml_single_match_shows_confirmation() {
    require_level!(
        Level::L2,
        WezTermHarness::available(),
        "WezTerm CLI (set WEZTERM_UNIX_SOCKET)",
    );
    let mut harness = WezTermHarness::shared_or_spawn().expect("attach/spawn WezTerm");
    run_yaml_single_match_test(&mut harness);
}

#[test]
#[serial(level2_terminal)]
fn level2_wezterm_sequence_yaml_multi_match_uses_choose_one() {
    require_level!(
        Level::L2,
        WezTermHarness::available(),
        "WezTerm CLI (set WEZTERM_UNIX_SOCKET)",
    );
    let mut harness = WezTermHarness::shared_or_spawn().expect("attach/spawn WezTerm");
    run_yaml_multi_match_test(&mut harness);
}
