//! Level 2 real-terminal tests for the ENTER-path autocomplete chooser.
//!
//! Phase 5 of the `2026-06-14-auto-complete` feature. These tests drive
//! `claudine compose` with a missing `file` or `file[]` schema property
//! inside a real terminal (tmux / WezTerm) and assert:
//!
//! - A `file` property renders a single-select `ChooseOne` chooser.
//! - A `file[]` property renders a multi-select `ChooseMany` chooser.
//! - The detail pane renders beside the list in wide terminals and above
//!   the list in tall terminals (`SplitPane` `SplitDirection::Auto`).
//!
//! Gating: `#![cfg(unix)]`, `require_level!(Level::L2, ...)` so the tests
//! skip cleanly when the backend is unavailable and panic under
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
use biscuit_test_harness::{CapturedFrame, TerminalHarness, capture_settled};
use serial_test::serial;
use test_toolkit::{Backend, Level, require_level};

mod common;
use common::{TestWorkspace, augmented_path, init_git_repo, write_executable};

const WIDE_COLUMNS: u32 = 120;
const WIDE_LINES: u32 = 24;
const TALL_COLUMNS: u32 = 30;
const TALL_LINES: u32 = 50;

/// Backend-specific key injection. tmux routes through `send-keys` key
/// names (more reliable for Enter/Space/Down); WezTerm uses raw bytes.
trait KeySender: TerminalHarness {
    fn send_enter(&mut self) -> io::Result<()>;
    fn send_space(&mut self) -> io::Result<()>;
    fn send_down(&mut self) -> io::Result<()>;
}

impl KeySender for TmuxHarness {
    fn send_enter(&mut self) -> io::Result<()> {
        self.send_key("Enter")
    }
    fn send_space(&mut self) -> io::Result<()> {
        self.send_key("Space")
    }
    fn send_down(&mut self) -> io::Result<()> {
        self.send_key("Down")
    }
}

impl KeySender for WezTermHarness {
    fn send_enter(&mut self) -> io::Result<()> {
        self.send_text(b"\r")
    }
    fn send_space(&mut self) -> io::Result<()> {
        self.send_text(b" ")
    }
    fn send_down(&mut self) -> io::Result<()> {
        self.send_text(b"\x1b[B")
    }
}

/// Stage a workspace with a `goose` stub, an empty claudine config (so the
/// init wizard does not intercept stdin), a git repo (so file labels are
/// repo-relative), and a prompt file whose schema declares the requested
/// file property.
fn stage_workspace(property: &str, body: &str) -> TestWorkspace {
    let ws = TestWorkspace::named("auto-complete-chooser");
    let bin_dir = ws.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();
    let marker = ws.path().join("launched.flag");
    let prompt_dump = prompt_dump_path(&ws);

    stage_goose_stub(&bin_dir, &marker, &prompt_dump);
    stage_default_config(ws.path());
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

fn prompt_dump_path(ws: &TestWorkspace) -> PathBuf {
    ws.path().join("received.prompt")
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
            dir.join("claudine").exists().then(|| dir.join("claudine").display().to_string())
        })
        .unwrap_or_else(|| "claudine".to_string())
}


/// Drive the chooser to completion.
///
/// - Sends the compose command and waits for the chooser hint.
/// - Sends any `pre_submit_keys` (e.g. `Space` to toggle a `ChooseMany`
///   item) and captures the still-visible chooser frame.
/// - Sends `Enter` to submit and waits for the provider launch marker.
///
/// Returns `(chooser_frame, final_frame)`.
fn drive_chooser(
    harness: &mut impl KeySender,
    ws: &TestWorkspace,
    pre_submit_keys: &[Key],
) -> (CapturedFrame, CapturedFrame) {
    let marker = ws.path().join("launched.flag");
    let path = augmented_path(&ws.path().join("bin"));
    let home = ws.path().to_string_lossy();

    harness
        .send_command_with_env(
            &format!("cd '{}'", ws.path().display()),
            &[("HOME", home.as_ref())],
        )
        .expect("cd into workspace");

    let cmd = format!(
        "{} compose --goose {}",
        claudine_bin(),
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
            ],
        )
        .expect("send compose command");

    // Wait for the inline chooser to render before injecting keys.
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut frame = harness.capture().expect("initial capture");
    while Instant::now() < deadline {
        if frame.plain.contains("Enter=Submit") {
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
        frame = harness.capture().expect("wait for chooser");
    }
    assert!(
        frame.plain.contains("Enter=Submit"),
        "chooser never rendered; plain:\n{}",
        frame.plain
    );

    let mut chooser_frame = harness.capture().expect("capture chooser before input");
    for key in pre_submit_keys {
        let before = chooser_frame.raw.clone();
        match key {
            Key::Space => harness.send_space().expect("send pre-submit Space"),
            Key::Down => harness.send_down().expect("send pre-submit Down"),
        }
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut changed = false;
        while Instant::now() < deadline {
            chooser_frame = harness.capture().expect("capture chooser after input");
            if chooser_frame.raw != before {
                changed = true;
                break;
            }
            std::thread::sleep(Duration::from_millis(25));
        }
        assert!(
            changed,
            "chooser did not visibly respond to injected input; plain:\n{}",
            chooser_frame.plain
        );
    }

    harness.send_enter().expect("send Enter");

    // Wait for the goose stub to record its launch.
    let deadline = Instant::now() + Duration::from_secs(15);
    while Instant::now() < deadline {
        if marker.exists() {
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }

    let final_frame = capture_settled(harness).expect("final capture");
    assert!(
        marker.exists(),
        "provider stub did not launch; final frame:\n{}",
        final_frame.plain
    );
    (chooser_frame, final_frame)
}

#[derive(Clone, Copy)]
#[allow(dead_code)]
enum Key {
    Space,
    Down,
}

/// Candidate labels and detail text markers used by layout assertions.
/// Whether a line belongs to the candidate list (ChooseOne or ChooseMany).
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

fn candidate_marker(plain: &str) -> bool {
    plain.contains("plan") || plain.contains("Readme")
}

fn detail_marker(plain: &str) -> bool {
    plain.contains("Schema:")
        || plain.contains("no description")
        || plain.contains("no schema defined")
        || plain.contains("FILE")
}

/// Assert that the captured frame shows the chooser with a live detail pane.
fn assert_chooser_and_detail(frame: &CapturedFrame) {
    let plain = &frame.plain;
    assert!(
        detail_marker(plain),
        "detail pane must render schema/description; plain:\n{plain}"
    );
    assert!(
        candidate_marker(plain),
        "candidate list must render file labels; plain:\n{plain}"
    );
}

/// In a wide terminal the detail pane sits to the right of the list, so on
/// some list row the detail text begins at a column to the right of the list
/// glyph. This keys off the *geometry* (a list glyph with detail text to its
/// right on the same line), not a specific candidate label: the visible label
/// is the repo-relative path (never the frontmatter `name`), and which item
/// is selected is not fixed, so a label-coincidence check was brittle.
fn assert_wide_layout(frame: &CapturedFrame) {
    let has_side_by_side = frame
        .plain
        .lines()
        .any(|line| is_list_line(line) && detail_right_of_glyph(line));
    assert!(
        has_side_by_side,
        "wide terminal must render the detail pane to the right of the list; plain:\n{}",
        frame.plain
    );
}

/// True when a detail-pane marker appears to the right of the leftmost list
/// glyph on the same line — the signature of a side-by-side (horizontal)
/// split. Byte offsets are compared, which is sufficient for left-to-right
/// ordering within one line.
fn detail_right_of_glyph(line: &str) -> bool {
    let glyph = ['○', '☐', '▶', '☑', '\u{f043e}', '\u{f4aa}', '\u{f0131}', '\u{f14a}']
        .into_iter()
        .filter_map(|g| line.find(g))
        .min();
    let detail = ["Schema:", "no description", "no schema defined", "FILE"]
        .into_iter()
        .filter_map(|m| line.find(m))
        .min();
    matches!((glyph, detail), (Some(g), Some(d)) if d > g)
}

/// In a tall terminal the detail pane sits above the list, so detail text
/// appears on lines before the first candidate label.
fn assert_tall_layout(frame: &CapturedFrame) {
    let lines: Vec<&str> = frame.plain.lines().collect();
    let first_detail = lines.iter().position(|l| detail_marker(l));
    let first_list = lines
        .iter()
        .position(|l| is_list_line(l) && candidate_marker(l));
    assert!(
        matches!((first_detail, first_list), (Some(d), Some(c)) if d < c),
        "tall terminal must render detail above the candidate list; plain:\n{}",
        frame.plain
    );
}

/// Read the prompt the fake `goose` provider received via its `-t` flag.
fn read_recorded_prompt(ws: &TestWorkspace) -> String {
    let path = prompt_dump_path(ws);
    fs::read_to_string(&path).unwrap_or_else(|_| {
        panic!(
            "provider did not record its -t prompt to {}",
            path.display()
        )
    })
}

/// Assert the captured chooser frame uses `ChooseOne` (single-select) glyphs
/// and not the checkbox glyphs used by `ChooseMany`.
fn assert_choose_one_markers(frame: &CapturedFrame) {
    let plain = &frame.plain;
    let has_radio = plain.contains('○')
        || plain.contains('\u{f043e}')
        || plain.contains('\u{f4aa}');
    assert!(
        has_radio,
        "ChooseOne must render a radio marker (○/\u{f043e}/\u{f4aa}); plain:\n{plain}"
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

/// Assert the captured chooser frame uses `ChooseMany` (multi-select) glyphs
/// and that exactly `expected_checked` items are checked.
fn assert_choose_many_markers(frame: &CapturedFrame, expected_checked: usize) {
    let plain = &frame.plain;
    let has_checkbox = plain.contains('☐')
        || plain.contains('☑')
        || plain.contains('\u{f0131}')
        || plain.contains('\u{f14a}');
    assert!(
        has_checkbox,
        "ChooseMany must render checkbox markers (☐/☑/\u{f0131}/\u{f14a}); plain:\n{plain}"
    );
    let has_radio = plain.contains('○')
        || plain.contains('\u{f043e}')
        || plain.contains('\u{f4aa}');
    assert!(
        !has_radio,
        "ChooseMany must not render a radio marker (○/\u{f043e}/\u{f4aa}); plain:\n{plain}"
    );
    let checked_count = plain.matches('☑').count() + plain.matches('\u{f14a}').count();
    assert_eq!(
        checked_count, expected_checked,
        "ChooseMany must have {expected_checked} checked item(s), found {checked_count}; plain:\n{plain}"
    );
}

// ----------------------------------------------------------------------
// Type-driven chooser behavior
// ----------------------------------------------------------------------

fn run_file_chooser_test<H: KeySender>(harness: &mut H) {
    let ws = stage_workspace("cover: 'file(required)'", "cover: {{ doc.cover }}");
    let (chooser_frame, _) = drive_chooser(harness, &ws, &[]);
    assert_chooser_and_detail(&chooser_frame);
    assert_choose_one_markers(&chooser_frame);

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
        value.contains("notes.md"),
        "selected file path must appear in the composed prompt; value: {value}"
    );
}

fn run_file_array_chooser_test<H: KeySender>(harness: &mut H) {
    let ws = stage_workspace(
        "attachments: 'file[](required)'",
        "attachments: {{ doc.attachments }}",
    );
    let (chooser_frame, _) =
        drive_chooser(harness, &ws, &[Key::Space, Key::Down, Key::Down, Key::Space]);
    assert_chooser_and_detail(&chooser_frame);
    assert_choose_many_markers(&chooser_frame, 2);

    let prompt = read_recorded_prompt(&ws);
    assert!(
        prompt.starts_with("attachments: "),
        "composed prompt must begin with 'attachments: '; prompt:\n{prompt}"
    );
    // Bare array interpolation renders line-separated (spec D4): one selected
    // file path per line, no JSON brackets.
    let value = prompt.trim_start_matches("attachments: ").trim();
    let lines: Vec<&str> = value.lines().map(str::trim).collect();
    assert_eq!(
        lines.len(),
        2,
        "must have selected exactly two files; value: {value}"
    );
    assert!(
        lines.iter().any(|line| line.contains("notes.md")),
        "notes.md must be selected; value: {value}"
    );
    assert!(
        lines.iter().any(|line| line.contains("readme.md")),
        "readme.md must be selected; value: {value}"
    );
}

// ----------------------------------------------------------------------
// SplitPane Auto layout
// ----------------------------------------------------------------------

fn run_wide_layout_test<H: KeySender>(harness: &mut H) {
    let ws = stage_workspace("cover: 'file(required)'", "Plan.");
    let (chooser_frame, _) = drive_chooser(harness, &ws, &[]);
    assert_wide_layout(&chooser_frame);
}

fn run_tall_layout_test<H: KeySender>(harness: &mut H) {
    let ws = stage_workspace("cover: 'file(required)'", "Plan.");
    let (chooser_frame, _) = drive_chooser(harness, &ws, &[]);
    assert_tall_layout(&chooser_frame);
}

// ----------------------------------------------------------------------
// tmux backend
// ----------------------------------------------------------------------

#[test]
#[serial(level2_terminal)]
fn level2_tmux_file_property_uses_choose_one() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);
    let mut harness = TmuxHarness::new();
    harness.spawn_shell().expect("tmux harness");
    harness.resize(80, 24).ok();
    run_file_chooser_test(&mut harness);
}

#[test]
#[serial(level2_terminal)]
fn level2_tmux_file_array_property_uses_choose_many() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);
    let mut harness = TmuxHarness::new();
    harness.spawn_shell().expect("tmux harness");
    harness.resize(80, 24).ok();
    run_file_array_chooser_test(&mut harness);
}

#[test]
#[serial(level2_terminal)]
fn level2_tmux_chooser_detail_right_in_wide_terminal() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);
    let mut harness = TmuxHarness::new();
    harness.spawn_shell().expect("tmux harness");
    harness.resize(WIDE_COLUMNS, WIDE_LINES).expect("resize tmux pane wide");
    run_wide_layout_test(&mut harness);
}

#[test]
#[serial(level2_terminal)]
fn level2_tmux_chooser_detail_above_in_tall_terminal() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);
    let mut harness = TmuxHarness::new();
    harness.spawn_shell().expect("tmux harness");
    harness.resize(TALL_COLUMNS, TALL_LINES).expect("resize tmux pane tall");
    run_tall_layout_test(&mut harness);
}

// ----------------------------------------------------------------------
// WezTerm backend
// ----------------------------------------------------------------------

#[test]
#[serial(level2_terminal)]
fn level2_wezterm_file_property_uses_choose_one() {
    require_level!(Level::L2, WezTermHarness::available(), Backend::WezTerm);
    let mut harness = WezTermHarness::shared_or_spawn().expect("attach/spawn WezTerm");
    run_file_chooser_test(&mut harness);
}

#[test]
#[serial(level2_terminal)]
fn level2_wezterm_file_array_property_uses_choose_many() {
    require_level!(Level::L2, WezTermHarness::available(), Backend::WezTerm);
    let mut harness = WezTermHarness::shared_or_spawn().expect("attach/spawn WezTerm");
    run_file_array_chooser_test(&mut harness);
}
