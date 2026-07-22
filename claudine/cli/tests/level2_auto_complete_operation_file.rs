//! Level 2 real-terminal tests for operation-file ENTER-path autocomplete.
//!
//! Phase 5 follow-up for the `2026-06-14-auto-complete` feature. These tests
//! start from an unresolved operation-file partial (`claudine compose plan`)
//! rather than a missing schema property, and verify the presentation and
//! interaction paths mandated by the spec:
//!
//! - Single match → lightweight `Use this file? (Y/n)` confirmation dialog.
//! - Multiple matches → `ChooseOne` two-pane chooser with live detail pane.
//! - Enter/Y acceptance and arrow-key selection → the chosen operation runs.
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
    fn send_enter(&mut self) -> io::Result<()>;
    fn send_esc(&mut self) -> io::Result<()>;
    fn send_yes(&mut self) -> io::Result<()>;
    fn send_down(&mut self) -> io::Result<()>;
    fn send_up(&mut self) -> io::Result<()>;
}

impl KeySender for TmuxHarness {
    fn send_enter(&mut self) -> io::Result<()> {
        self.send_key("Enter")
    }

    fn send_esc(&mut self) -> io::Result<()> {
        self.send_key("Escape")
    }

    fn send_yes(&mut self) -> io::Result<()> {
        self.send_key("y")
    }

    fn send_down(&mut self) -> io::Result<()> {
        self.send_key("Down")
    }

    fn send_up(&mut self) -> io::Result<()> {
        self.send_key("Up")
    }
}

impl KeySender for WezTermHarness {
    fn send_enter(&mut self) -> io::Result<()> {
        self.send_text(b"\r")
    }

    fn send_esc(&mut self) -> io::Result<()> {
        self.send_text(b"\x1b")
    }

    fn send_yes(&mut self) -> io::Result<()> {
        self.send_text(b"y")
    }

    fn send_down(&mut self) -> io::Result<()> {
        self.send_text(b"\x1b[B")
    }

    fn send_up(&mut self) -> io::Result<()> {
        self.send_text(b"\x1b[A")
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
        "kind: sequence\nname: 'Build Pipeline'\ndescription: 'CI steps'\nprompt: 'Process {{ state.name }}'\n$schema:\n  env: 'enum(dev, prod)'\nsequence:\n  - one\n",
    )
    .unwrap();

    if !single_match {
        fs::write(
            prompts.join("extra_steps.yaml"),
            "kind: sequence\nname: 'Deploy Flow'\ndescription: 'Release steps'\nprompt: 'Process {{ state.name }}'\n$schema:\n  region: 'enum(us, eu)'\nsequence:\n  - two\n",
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

fn wait_for_text(harness: &mut impl TerminalHarness, expected: &str) -> CapturedFrame {
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut frame = harness.capture().expect("initial capture");
    while Instant::now() < deadline {
        if frame.plain.contains(expected) {
            return frame;
        }
        std::thread::sleep(Duration::from_millis(50));
        frame = harness.capture().expect("poll terminal content");
    }
    panic!(
        "expected terminal content {expected:?} never rendered; plain:\n{}",
        frame.plain
    );
}

fn wait_for_active_change(
    harness: &mut impl TerminalHarness,
    previous: &str,
) -> CapturedFrame {
    let deadline = Instant::now() + Duration::from_secs(3);
    let mut frame = harness.capture().expect("initial active-item capture");
    while Instant::now() < deadline {
        if let Some(active) = active_line(&frame.plain)
            && active != previous
        {
            return frame;
        }
        std::thread::sleep(Duration::from_millis(50));
        frame = harness.capture().expect("poll active chooser item");
    }
    panic!(
        "active chooser item did not change from {previous:?}; plain:\n{}",
        frame.plain
    );
}

fn wait_for_provider(harness: &mut impl TerminalHarness, ws: &TestWorkspace) -> CapturedFrame {
    let marker = ws.path().join("launched.flag");
    let deadline = Instant::now() + Duration::from_secs(15);
    let mut frame = harness.capture().expect("initial provider capture");
    while Instant::now() < deadline {
        if marker.exists() && shell_done_flag(ws).exists() {
            return frame;
        }
        std::thread::sleep(Duration::from_millis(100));
        frame = harness.capture().expect("poll provider completion");
    }
    panic!(
        "provider did not launch and return to the shell; marker={}, shell_done={}; plain:\n{}",
        marker.exists(),
        shell_done_flag(ws).exists(),
        frame.plain
    );
}

fn read_recorded_prompt(ws: &TestWorkspace) -> String {
    let path = ws.path().join("received.prompt");
    fs::read_to_string(&path)
        .unwrap_or_else(|_| panic!("provider did not record a prompt at {}", path.display()))
}

#[derive(Clone, Copy)]
enum AcceptKey {
    Enter,
    Yes,
}

fn run_confirmation_accept_test<H: KeySender>(
    harness: &mut H,
    ws: &TestWorkspace,
    send_command: impl FnOnce(&mut H, &TestWorkspace),
    key: AcceptKey,
    expected_prompt: &str,
) {
    send_command(harness, ws);
    wait_for_text(harness, CONFIRMATION_PROMPT);
    match key {
        AcceptKey::Enter => harness.send_enter().expect("send Enter"),
        AcceptKey::Yes => harness.send_yes().expect("send y"),
    }
    wait_for_provider(harness, ws);
    let prompt = read_recorded_prompt(ws);
    assert!(
        prompt.contains(expected_prompt),
        "provider prompt must contain {expected_prompt:?}; prompt:\n{prompt}"
    );
}

fn run_chooser_navigation_test<H: KeySender>(
    harness: &mut H,
    ws: &TestWorkspace,
    send_command: impl FnOnce(&mut H, &TestWorkspace),
    expected_prompt: &str,
) {
    send_command(harness, ws);
    let baseline = wait_for_text(harness, CHOOSER_HINT);
    let baseline_active = active_line(&baseline.plain)
        .expect("chooser must show an active item")
        .to_string();

    harness.send_down().expect("send Down");
    let after_down = wait_for_active_change(harness, &baseline_active);
    let down_active = active_line(&after_down.plain)
        .expect("chooser must show an active item after Down")
        .to_string();

    harness.send_up().expect("send Up");
    let after_up = wait_for_active_change(harness, &down_active);
    assert_eq!(
        active_line(&after_up.plain),
        Some(baseline_active.as_str()),
        "Up must restore the original active item"
    );

    harness.send_down().expect("send Down before submit");
    wait_for_active_change(harness, &baseline_active);
    harness.send_enter().expect("send Enter to submit");
    wait_for_provider(harness, ws);

    let prompt = read_recorded_prompt(ws);
    assert!(
        prompt.contains(expected_prompt),
        "provider prompt must contain the navigated selection {expected_prompt:?}; prompt:\n{prompt}"
    );
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
// Plan schema detail capture
// ----------------------------------------------------------------------

/// Stage a workspace whose prompts carry the exact repo `plan.md`
/// `$schema` shape and an emphasized `_feature_` / `_plan_` description.
/// Both candidate files share the schema so the detail pane shows it
/// regardless of which item is initially active.
fn stage_plan_schema_workspace() -> TestWorkspace {
    let ws = TestWorkspace::named("auto-complete-plan-schema-detail");
    let bin_dir = ws.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();
    let marker = ws.path().join("launched.flag");
    let prompt_dump = ws.path().join("received.prompt");

    stage_goose_stub(&bin_dir, &marker, &prompt_dump);
    stage_default_config(ws.path());
    init_git_repo(ws.path());

    let prompts = ws.path().join("prompts");
    fs::create_dir_all(&prompts).unwrap();

    let plan_content = r#"---
name: 'Plan'
description: 'Creates a multi-phase, high confidence plan from a _feature_ or _plan_'
$schema:
  spec: 'file(required;match(**/*spec*.md);eager)'
  design: 'file(match(**/*design*.md))'
  plan: 'file'
---
# Plan
"#;

    fs::write(prompts.join("plan.md"), plan_content).unwrap();
    fs::write(
        prompts.join("planner.md"),
        r#"---
name: 'Planner'
description: 'Creates a multi-phase, high confidence plan from a _feature_ or _plan_'
$schema:
  spec: 'file(required;match(**/*spec*.md);eager)'
  design: 'file(match(**/*design*.md))'
  plan: 'file'
---
# Planner
"#,
    )
    .unwrap();

    ws
}

/// Drive the multi-match chooser and wait for its detail pane to settle.
fn drive_plan_schema_chooser<H: KeySender>(harness: &mut H, ws: &TestWorkspace) -> CapturedFrame {
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

    // Let the detail pane finish rendering before capturing.
    std::thread::sleep(Duration::from_millis(200));
    let captured = harness.capture().expect("capture plan schema chooser");

    harness.send_esc().expect("send Esc to cancel");
    let _ = wait_for_shell_return(harness, ws, Duration::from_secs(10));

    captured
}

/// Assert the captured chooser detail pane preserves schema property order,
/// keeps `match(...)` globs unmangled, and renders description emphasis
/// through the real terminal capture path.
fn assert_schema_detail_rendered(frame: &CapturedFrame) {
    let plain = &frame.plain;
    let raw = &frame.raw;

    assert!(
        plain.contains("spec:"),
        "detail pane must contain 'spec:' property; plain:\n{plain}"
    );
    assert!(
        plain.contains("design:"),
        "detail pane must contain 'design:' property; plain:\n{plain}"
    );
    assert!(
        plain.contains("plan:"),
        "detail pane must contain 'plan:' property; plain:\n{plain}"
    );

    let spec_idx = plain.find("spec:").unwrap();
    let design_idx = plain.find("design:").unwrap();
    let plan_idx = plain.find("plan:").unwrap();
    assert!(
        spec_idx < design_idx && design_idx < plan_idx,
        "schema properties must appear in authored order; plain:\n{plain}"
    );

    assert!(
        plain.contains("match(**/*spec*.md)"),
        "spec match glob must render unmangled; plain:\n{plain}"
    );
    assert!(
        plain.contains("match(**/*design*.md)"),
        "design match glob must render unmangled; plain:\n{plain}"
    );

    assert!(
        plain.contains("multi-phase"),
        "description must contain 'multi-phase'; plain:\n{plain}"
    );
    assert!(
        plain.contains("feature"),
        "description must contain the word 'feature'; plain:\n{plain}"
    );
    assert!(
        plain.contains("plan"),
        "description must contain the word 'plan'; plain:\n{plain}"
    );
    assert!(
        !plain.contains("_feature_"),
        "literal '_feature_' markup must not leak; plain:\n{plain}"
    );
    assert!(
        !plain.contains("_plan_"),
        "literal '_plan_' markup must not leak; plain:\n{plain}"
    );
    assert!(
        raw.contains("\u{1b}[3m"),
        "raw capture must contain italic SGR for emphasis; raw:\n{raw}"
    );
}

fn run_schema_detail_test<H: KeySender>(harness: &mut H) {
    let ws = stage_plan_schema_workspace();
    let frame = drive_plan_schema_chooser(harness, &ws);
    assert_schema_detail_rendered(&frame);
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
fn level2_tmux_operation_file_enter_accepts_single_match() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");
    let ws = stage_workspace(true);
    let mut harness = TmuxHarness::new();
    harness.spawn_shell().expect("tmux harness");
    run_confirmation_accept_test(
        &mut harness,
        &ws,
        send_compose_command,
        AcceptKey::Enter,
        "Plan Alpha",
    );
}

#[test]
#[serial(level2_terminal)]
fn level2_tmux_operation_file_y_accepts_single_match() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");
    let ws = stage_workspace(true);
    let mut harness = TmuxHarness::new();
    harness.spawn_shell().expect("tmux harness");
    run_confirmation_accept_test(
        &mut harness,
        &ws,
        send_compose_command,
        AcceptKey::Yes,
        "Plan Alpha",
    );
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
fn level2_tmux_operation_file_arrow_navigation_selects_in_chooser() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");
    let ws = stage_workspace(false);
    let mut harness = TmuxHarness::new();
    harness.spawn_shell().expect("tmux harness");
    run_chooser_navigation_test(&mut harness, &ws, send_compose_command, "Plan Beta");
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
fn level2_tmux_sequence_yaml_enter_accepts_single_match() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");
    let ws = stage_yaml_sequence_workspace(true);
    let mut harness = TmuxHarness::new();
    harness.spawn_shell().expect("tmux harness");
    run_confirmation_accept_test(
        &mut harness,
        &ws,
        send_sequence_command,
        AcceptKey::Enter,
        "one",
    );
}

#[test]
#[serial(level2_terminal)]
fn level2_tmux_sequence_yaml_y_accepts_single_match() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");
    let ws = stage_yaml_sequence_workspace(true);
    let mut harness = TmuxHarness::new();
    harness.spawn_shell().expect("tmux harness");
    run_confirmation_accept_test(
        &mut harness,
        &ws,
        send_sequence_command,
        AcceptKey::Yes,
        "one",
    );
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
fn level2_tmux_sequence_yaml_arrow_navigation_selects_in_chooser() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");
    let ws = stage_yaml_sequence_workspace(false);
    let mut harness = TmuxHarness::new();
    harness.spawn_shell().expect("tmux harness");
    run_chooser_navigation_test(&mut harness, &ws, send_sequence_command, "one");
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

#[test]
#[serial(level2_terminal)]
fn level2_tmux_operation_file_schema_detail_renders_faithfully() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");
    let mut harness = TmuxHarness::new();
    harness.spawn_shell().expect("tmux harness");
    harness.resize(120, 50).expect("resize tmux pane large");
    run_schema_detail_test(&mut harness);
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
