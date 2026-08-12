//! Level-2 tests for `md schema about` terminal rendering.
//!
//! These tests drive the actual `md` binary inside a detached tmux session and
//! capture the terminal's rendered cells. The Level-1 tests prove the CLI emits
//! table-stripe escapes; this file proves the table row reaches a real terminal
//! as a painted row in the user-visible report without affecting the user's
//! visible terminal UI.

use biscuit_test_harness::shared::SharedHarness;
use biscuit_test_harness::tmux::TmuxHarness;
use biscuit_test_harness::{CapturedFrame, TerminalHarness};
use darkmatter::markdown::highlighting::{CodeHighlighter, ColorMode, ThemePair};
use serial_test::serial;
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};
use syntect::easy::HighlightLines;
use syntect::highlighting::Color;
use test_toolkit::{Backend, Level, require_level};

static SHARED_TMUX: SharedHarness<TmuxHarness> = SharedHarness::new();
static SENTINEL_COUNTER: AtomicU32 = AtomicU32::new(0);
const SENTINEL_TIMEOUT: Duration = Duration::from_secs(30);

fn wait_for_sentinel(
    harness: &mut TmuxHarness,
    sentinel: &str,
) -> Result<CapturedFrame, CapturedFrame> {
    let deadline = Instant::now() + SENTINEL_TIMEOUT;
    let mut last = CapturedFrame::from_raw(String::new());
    while Instant::now() < deadline {
        if let Ok(frame) = harness.capture() {
            if frame.plain.lines().any(|l| l.trim() == sentinel) {
                return Ok(frame);
            }
            last = frame;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    Err(last)
}

fn run_with_sentinel(harness: &mut TmuxHarness, cmd: &str, colorfgbg: &str) -> CapturedFrame {
    let id = SENTINEL_COUNTER.fetch_add(1, Ordering::Relaxed);
    let sentinel = format!("__DM_SCHEMA_ABOUT_L2_DONE_{id}__");
    let sequence = format!("{cmd}; printf '\\n{sentinel}\\n'");
    // Keep the harness-prefixed color environment active across the full
    // sequence rather than scoping it to only the first simple command.
    let wrapped = format!("sh -c {}", shell_quote(&sequence));

    harness
        .send_command_with_env(
            &wrapped,
            &[
                ("COLORFGBG", colorfgbg),
                ("THEME", "one-half"),
                ("CODE_THEME", "one-half"),
            ],
        )
        .expect("send_command_with_env failed");
    match wait_for_sentinel(harness, &sentinel) {
        Ok(frame) => {
            std::thread::sleep(Duration::from_millis(250));
            harness.capture().unwrap_or(frame)
        }
        Err(last) => panic!(
            "timed out waiting for sentinel {sentinel} after {SENTINEL_TIMEOUT:?}. \
             last plain capture:\n{}",
            last.plain
        ),
    }
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn md_bin() -> String {
    std::env::var("CARGO_BIN_EXE_md").unwrap_or_else(|_| "md".to_string())
}

fn capture_schema_about(harness: &mut TmuxHarness, colorfgbg: &str) -> CapturedFrame {
    run_with_sentinel(harness, "clear", colorfgbg);
    // The broker pane can outlive a prior fixture CWD, so every invocation
    // reestablishes a stable package directory before terminal discovery.
    let working_dir = shell_quote(env!("CARGO_MANIFEST_DIR"));
    let command = format!("cd {working_dir} && {} schema about", shell_quote(&md_bin()));
    let _visible = run_with_sentinel(harness, &command, colorfgbg);
    capture_scrollback(harness, 300).unwrap_or(_visible)
}

fn capture_scrollback(harness: &TmuxHarness, lines: i32) -> Option<CapturedFrame> {
    let output = Command::new("tmux")
        .args([
            "capture-pane",
            "-t",
            harness.session_name(),
            "-S",
            &format!("-{lines}"),
            "-p",
            "-e",
        ])
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| CapturedFrame::from_raw(String::from_utf8_lossy(&output.stdout).into_owned()))
}

fn has_background_sgr(raw: &str) -> bool {
    raw.contains("\x1b[48;2;")
        || raw.contains("\x1b[48:2:")
        || raw.contains("\x1b[48;5;")
        || raw.contains("\x1b[40m")
        || raw.contains("\x1b[100m")
}

fn fg_sgr(color: Color) -> String {
    format!("\x1b[38;2;{};{};{}m", color.r, color.g, color.b)
}

fn bg_sgr(color: Color) -> String {
    format!("\x1b[48;2;{};{};{}m", color.r, color.g, color.b)
}

fn one_half_yaml_color(mode: ColorMode, line: &str, token: &str) -> Color {
    let highlighter = CodeHighlighter::new(ThemePair::OneHalf, mode);
    let syntax = highlighter
        .syntax_set()
        .find_syntax_by_extension("yaml")
        .expect("yaml syntax");
    let mut hl = HighlightLines::new(syntax, highlighter.theme());
    let token_start = line
        .find(token)
        .unwrap_or_else(|| panic!("missing token {token:?} in source line {line:?}"));
    let mut offset = 0;
    hl.highlight_line(line, highlighter.syntax_set())
        .expect("highlight line")
        .into_iter()
        .find_map(|(style, text)| {
            let end = offset + text.len();
            let matched = (offset..end).contains(&token_start);
            offset = end;
            matched.then_some(style.foreground)
        })
        .unwrap_or_else(|| panic!("missing token {token:?} in highlighted line {line:?}"))
}

fn one_half_background(mode: ColorMode) -> Color {
    CodeHighlighter::new(ThemePair::OneHalf, mode)
        .theme()
        .settings
        .background
        .expect("theme background")
}

fn raw_line_for_plain_needle(frame: &CapturedFrame, needle: &str) -> Option<String> {
    let raw_lines: Vec<&str> = frame.raw.lines().collect();
    frame
        .plain
        .lines()
        .enumerate()
        .find(|(_, plain)| plain.contains('│') && plain.contains(needle))
        .and_then(|(idx, _)| raw_lines.get(idx).map(|raw| (*raw).to_string()))
}

fn raw_line_anywhere(frame: &CapturedFrame, needle: &str) -> Option<String> {
    let raw_lines: Vec<&str> = frame.raw.lines().collect();
    let plain_lines: Vec<&str> = frame.plain.lines().collect();
    plain_lines
        .iter()
        .enumerate()
        .rev()
        .find(|(_, plain)| plain.contains(needle))
        .and_then(|(idx, _)| raw_lines.get(idx).map(|raw| (*raw).to_string()))
}

#[test]
#[serial(level2_terminal)]
fn level2_schema_about_constraint_table_renders_striped_row() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

    let mut guard = SHARED_TMUX
        .get_or_init(|| TmuxHarness::shared_or_spawn().expect("attach/spawn tmux"));
    let harness = guard.as_mut().expect("shared tmux harness present");
    let frame = capture_schema_about(harness, "15;0");

    assert!(
        frame.plain.contains("max(number)"),
        "expected a constraint table row in the real-terminal capture. plain:\n{}",
        frame.plain,
    );

    let striped_row = [
        "any",
        "string",
        "number",
        "boolean",
        "object",
        "array",
        "required",
        "min(number)",
        "max(number)",
        "matches(regex)",
        "oneOf",
        "generated",
    ]
    .into_iter()
    .find_map(|needle| {
        raw_line_for_plain_needle(&frame, needle)
            .filter(|row| has_background_sgr(row))
            .map(|row| (needle, row))
    })
    .unwrap_or_else(|| {
        panic!(
            "could not locate any striped constraint table row in the real-terminal capture.\nplain:\n{}\nraw:\n{}",
            frame.plain, frame.raw,
        )
    });
    assert!(
        has_background_sgr(&striped_row.1),
        "expected a striped constraint table row to carry a background SGR in the real terminal capture.\nrow name: {}\nrow:\n{:?}\nplain:\n{}\nraw:\n{}",
        striped_row.0,
        striped_row.1,
        frame.plain,
        frame.raw,
    );
}

fn assert_schema_about_yaml_uses_theme(frame: &CapturedFrame, expected_mode: ColorMode) {
    let code_line = raw_line_anywhere(frame, "bar: string[]").unwrap_or_else(|| {
        panic!(
            "could not locate schema-about YAML example line.\nplain:\n{}\nraw:\n{}",
            frame.plain, frame.raw,
        )
    });
    let background = one_half_background(expected_mode);
    let object_key = one_half_yaml_color(expected_mode, "    bar: string[]", "bar");
    let string_value = one_half_yaml_color(expected_mode, "    bar: string[]", "string[]");

    assert!(
        code_line.contains(&bg_sgr(background)),
        "schema-about YAML examples should use the exact OneHalf {:?} background RGB({},{},{}).\nline:\n{code_line:?}\nplain:\n{}",
        expected_mode,
        background.r,
        background.g,
        background.b,
        frame.plain,
    );
    assert!(
        code_line.contains(&format!("{}bar", fg_sgr(object_key))),
        "schema-about YAML key should use the exact OneHalf {:?} key RGB({},{},{}).\nline:\n{code_line:?}\nplain:\n{}",
        expected_mode,
        object_key.r,
        object_key.g,
        object_key.b,
        frame.plain,
    );
    assert!(
        code_line.contains(&format!("{}string[]", fg_sgr(string_value))),
        "schema-about YAML scalar should use the exact OneHalf {:?} scalar RGB({},{},{}).\nline:\n{code_line:?}\nplain:\n{}",
        expected_mode,
        string_value.r,
        string_value.g,
        string_value.b,
        frame.plain,
    );
}

#[test]
#[serial(level2_terminal)]
fn level2_schema_about_dark_terminal_uses_light_code_theme() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

    let mut guard = SHARED_TMUX
        .get_or_init(|| TmuxHarness::shared_or_spawn().expect("attach/spawn tmux"));
    let harness = guard.as_mut().expect("shared tmux harness present");
    let frame = capture_schema_about(harness, "15;0");

    assert_schema_about_yaml_uses_theme(&frame, ColorMode::Light);
}

#[test]
#[serial(level2_terminal)]
fn level2_schema_about_light_terminal_uses_dark_code_theme() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

    let mut guard = SHARED_TMUX
        .get_or_init(|| TmuxHarness::shared_or_spawn().expect("attach/spawn tmux"));
    let harness = guard.as_mut().expect("shared tmux harness present");
    let frame = capture_schema_about(harness, "0;15");

    assert_schema_about_yaml_uses_theme(&frame, ColorMode::Dark);
}
