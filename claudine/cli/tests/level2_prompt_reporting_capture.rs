//! Level 2 real-terminal capture tests for prompt reporting.
//!
//! The L1 unit tests in `render/prompt/*` and the `prompt_reporting.rs`
//! CLI integration tests assert the report's *semantics* after stripping ANSI
//! (and they force `NO_COLOR=1`), so they cannot catch broken SGR styling,
//! OSC8 hyperlink emission, block-quote chrome, or width/truncation geometry in
//! a real terminal. These tests drive the real `claudine compose --goose`
//! binary inside a terminal emulator and assert against the bytes the terminal
//! actually displayed (`frame.raw` / `frame.plain`):
//!
//! - the orange (Tailwind Orange500) `System Prompt` block quote and the green
//!   (Tailwind Green500) `Agent Prompt` block quote, read from the captured SGR;
//! - the system-prompt file OSC8 hyperlink (WezTerm, which renders OSC8
//!   faithfully — the review's requested backend for link fidelity);
//! - narrow-width body wrapping and the 2-cell `┃ ` block-quote chrome;
//! - front/back truncation of an over-length agent prompt.
//!
//! ## Not covered here: the Nerd Font repo-glyph label
//!
//! The system-prompt summary renders its hyperlink label as the repo glyph
//! joined directly to the relative path (`<glyph>/system-prompt.md`, no space)
//! when the terminal reports Nerd Font support. That branch cannot be exercised
//! through this harness: capturing styled SGR requires `FORCE_COLOR=1`, which
//! routes claudine through `Terminal::new_optimistic`, and that constructor
//! hardcodes `is_nerd_font: None` — it never consults `detect_nerd_font()`, so
//! the `NERD_FONT` env declaration cannot reach the glyph branch. The no-space
//! label is therefore guarded at L1 by
//! `render::prompt::system::display_label_nerd_font_in_base_uses_glyph_with_path`,
//! which asserts the exact `<glyph>/.claude/system-prompt.md` string.
//!
//! ## Backend split
//!
//! tmux is the portable, headless backend and faithfully re-emits truecolor SGR
//! through `capture-pane -e`, so it carries the color, wrapping, and truncation
//! contracts on every host that has `tmux`. OSC8 hyperlink fidelity is verified
//! on WezTerm per the review; the link-emission *logic* is already covered at
//! L1 (`render::prompt::system::summary_emits_osc8_for_file_link`).
//!
//! `FORCE_COLOR=1` routes claudine through an optimistic terminal so the styling
//! is the emulator's capture, not claudine's raw stream; `COLUMNS` fixes the
//! logical render width independent of the pane size; `HOME` is re-rooted at the
//! per-test workspace so the system-prompt change-detection state starts empty
//! (a first run always shows the report in the default precedence branch).
//!
//! Skip-clean: each test checks `Harness::available()` first and returns early
//! when the backend is absent (no `#[ignore]`). `BISCUIT_TEST_LEVEL_REQUIRED=2`
//! flips a missing harness into a hard failure. Run via the canonical recipe:
//!
//! ```text
//! just test-l2
//! ```

#![cfg(unix)]

#[allow(deprecated)]
use assert_cmd::cargo::cargo_bin;
use biscuit_terminal::utils::block_constraint::visible_width;
use biscuit_test_harness::tmux::{
    TmuxHarness, kill_session_by_name, spawn_shell_session_with_env,
};
use biscuit_test_harness::wezterm::WezTermHarness;
use biscuit_test_harness::{CapturedFrame, TerminalHarness};
use serial_test::serial;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;
use test_toolkit::{Backend, Level, require_level};

mod common;
use common::{TestWorkspace, augmented_path, clear_no_color, write_executable};

/// Tailwind Orange500 foreground (truecolor): the `System Prompt` header and
/// block-quote border. Mirrors the generated palette in `renderable`.
const ORANGE_500_FG: &str = "38;2;255;105;0";

/// Tailwind Green500 foreground (truecolor): the `Agent Prompt` header and
/// block-quote border.
const GREEN_500_FG: &str = "38;2;0;201;80";

/// Visible cells consumed by the block-quote chrome (`┃ ` border + space).
/// Mirrors `render::prompt::formatting::PROMPT_CHROME_WIDTH`.
const PROMPT_CHROME_WIDTH: usize = 2;

const SYSTEM_PROMPT_SHORT: &str =
    "# Test System Prompt\n\nThis is a test system prompt for real-terminal capture.\n";

const USER_PROMPT_SHORT: &str = "# Compose body\nHello from compose.\n";

/// Distinct, space-separated token repeated to build a long single-paragraph
/// system body that wraps purely on word boundaries (no unbreakable token), so
/// the wrap test exercises geometry rather than overflow.
const WRAP_TOKEN: &str = "wrapword";

/// A staged compose workspace: a stubbed `goose` on `PATH` (echoes and exits),
/// an isolated `HOME` for change-detection state, a user prompt, and an optional
/// system prompt. Kept alive for the duration of a test so its files outlive the
/// capture.
struct Fixture {
    workspace: TestWorkspace,
    bin_dir: PathBuf,
    sp_path: Option<PathBuf>,
    md_path: PathBuf,
}

impl Fixture {
    fn new(system_prompt: Option<&str>, user_prompt: &str) -> Self {
        let workspace = TestWorkspace::named("claudine-promptrep-l2");
        let bin_dir = workspace.path().join("bin");
        fs::create_dir_all(&bin_dir).unwrap();
        write_executable(
            &bin_dir.join("goose"),
            "#!/bin/sh\necho 'Agent response'\nexit 0\n",
        );

        // HOME points here, so change-detection state lives under this dir and
        // every run is a first run (the report always renders by default).
        let claudine_dir = workspace.path().join(".claudine");
        fs::create_dir_all(&claudine_dir).unwrap();
        fs::write(claudine_dir.join("config.json"), "{}").unwrap();

        let md_path = workspace.path().join("prompt.md");
        fs::write(&md_path, user_prompt).unwrap();

        let sp_path = system_prompt.map(|sp| {
            let p = workspace.path().join("system-prompt.md");
            fs::write(&p, sp).unwrap();
            p
        });

        Self {
            workspace,
            bin_dir,
            sp_path,
            md_path,
        }
    }
}

/// `# Wrap Test` document whose single body paragraph is `WRAP_TOKEN` repeated
/// 40 times — long enough to wrap onto several lines at a narrow `COLUMNS`.
fn system_prompt_long_paragraph() -> String {
    let words = std::iter::repeat_n(WRAP_TOKEN, 40)
        .collect::<Vec<_>>()
        .join(" ");
    format!("# Wrap Test\n\n{words}\n")
}

/// `n` `LineNNN` list items — a deterministic body for exercising front/back
/// truncation. List items (not a bare paragraph) each render on their own row;
/// consecutive non-blank paragraph lines would reflow into a single row and
/// never exceed the front/back row budget.
fn numbered_lines(n: usize) -> String {
    (1..=n)
        .map(|i| format!("- Line{i:03}"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Spawn a detached tmux session of `cols` × `rows` cells and return its name.
fn spawn_tmux_session(cols: u32, rows: u32) -> String {
    static SEQ: AtomicU32 = AtomicU32::new(0);
    let session = format!(
        "biscuit_promptrep_l2_{}_{}",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::Relaxed)
    );
    // POSIX shell (bash/sh), not the developer's `$SHELL`: a custom login
    // prompt (e.g. Starship's `❯`) never ends in `$`/`#`/`%`, so
    // `wait_for_prompt` would never match and burn its full timeout.
    spawn_shell_session_with_env(&session, cols, rows, &[("FORCE_COLOR", "1")])
        .expect("failed to spawn tmux session");
    session
}

/// Drive `claudine compose --goose [extra_args] [--append-system-prompt sp] md`
/// inside an already-spawned `harness` and return the captured pane frame.
///
/// The shell is re-rooted at the workspace so launch-CWD discovery (and thus the
/// link's base path) is deterministic; `COLUMNS` is set to `cols` so the logical
/// render width is fixed regardless of the pane geometry.
fn send_compose<H: TerminalHarness>(
    harness: &mut H,
    fx: &Fixture,
    cols: u32,
    extra_args: &[&str],
) -> CapturedFrame {
    let claudine = cargo_bin("claudine").display().to_string();
    let path = augmented_path(&fx.bin_dir);
    let path = path.to_string_lossy().into_owned();
    let home = fx.workspace.path().to_string_lossy().into_owned();
    let cols_s = cols.to_string();

    harness
        .send_text(format!("cd {}\n", fx.workspace.path().display()).as_bytes())
        .expect("cd into workspace");
    let _ = biscuit_test_harness::wait_for_prompt(harness);

    let mut cmd = format!("{claudine} compose --goose");
    for a in extra_args {
        cmd.push(' ');
        cmd.push_str(a);
    }
    if let Some(sp) = &fx.sp_path {
        cmd.push_str(&format!(" --append-system-prompt {}", sp.display()));
    }
    cmd.push_str(&format!(" {}", fx.md_path.display()));

    harness
        .send_command_with_env(
            &cmd,
            &[
                ("HOME", home.as_str()),
                ("PATH", path.as_str()),
                ("FORCE_COLOR", "1"),
                ("COLUMNS", cols_s.as_str()),
            ],
        )
        .expect("send compose command");
    let _ = biscuit_test_harness::wait_for_prompt(harness);
    std::thread::sleep(Duration::from_millis(400));
    harness.capture().expect("capture failed")
}

/// Spawn a dedicated tmux session sized `cols` × `rows`, run the compose, and
/// tear the session down.
fn capture_compose_tmux(fx: &Fixture, cols: u32, rows: u32, extra_args: &[&str]) -> CapturedFrame {
    let session = spawn_tmux_session(cols, rows);
    let mut harness = TmuxHarness::attach(&session);
    let _ = biscuit_test_harness::wait_for_prompt(&mut harness);
    // This fixture asserts a *colored* surface under `FORCE_COLOR=1`, which an
    // ambient `NO_COLOR` out-votes — see `common::clear_no_color`.
    clear_no_color(&mut harness);
    let frame = send_compose(&mut harness, fx, cols, extra_args);
    kill_session_by_name(&session);
    frame
}

/// Return the raw (escape-bearing) capture line whose plain (escape-stripped)
/// form contains `needle`. `frame.plain` is `frame.raw` stripped per line, so
/// the indices align.
fn raw_line<'a>(frame: &'a CapturedFrame, needle: &str) -> Option<&'a str> {
    let raw_lines: Vec<&str> = frame.raw.lines().collect();
    frame
        .plain
        .lines()
        .position(|l| l.contains(needle))
        .and_then(|i| raw_lines.get(i).copied())
}

/// tmux faithfully re-emits truecolor SGR, so it proves the two block quotes
/// render in their distinct brand colors: orange for the system prompt header,
/// green for the agent prompt header.
#[test]
#[serial(level2_terminal)]
fn level2_prompt_reporting_block_quote_colors_in_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

    let fx = Fixture::new(Some(SYSTEM_PROMPT_SHORT), USER_PROMPT_SHORT);
    let frame = capture_compose_tmux(&fx, 100, 40, &[]);

    assert!(
        frame.plain.contains("System Prompt"),
        "default compose must show the system prompt header.\nplain:\n{}",
        frame.plain,
    );
    assert!(
        frame.plain.contains("Agent Prompt"),
        "default compose must show the agent prompt header.\nplain:\n{}",
        frame.plain,
    );
    assert!(
        frame.plain.contains('┃'),
        "both reports must render the `┃` block-quote border.\nplain:\n{}",
        frame.plain,
    );

    let sys = raw_line(&frame, "System Prompt").unwrap_or_else(|| {
        panic!(
            "`System Prompt` line not found in capture.\nraw:\n{}",
            frame.raw
        )
    });
    assert!(
        sys.contains(ORANGE_500_FG),
        "system prompt header must render Tailwind Orange500 ({ORANGE_500_FG}).\nrow: {sys:?}",
    );

    let agent = raw_line(&frame, "Agent Prompt").unwrap_or_else(|| {
        panic!(
            "`Agent Prompt` line not found in capture.\nraw:\n{}",
            frame.raw
        )
    });
    assert!(
        agent.contains(GREEN_500_FG),
        "agent prompt header must render Tailwind Green500 ({GREEN_500_FG}).\nrow: {agent:?}",
    );
}

/// At a narrow `COLUMNS`, the verbose system body wraps onto multiple lines and
/// every wrapped line carries the 2-cell `┃ ` chrome while fitting within the
/// pane — proving both the wrapping and the chrome-width arithmetic in a real
/// terminal (the in-process `prompt_body_width` math is not trusted here).
#[test]
#[serial(level2_terminal)]
fn level2_prompt_reporting_body_wraps_and_reserves_chrome_in_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

    let sp = system_prompt_long_paragraph();
    let fx = Fixture::new(Some(&sp), USER_PROMPT_SHORT);
    let cols = 40u32;
    let frame = capture_compose_tmux(&fx, cols, 80, &["--verbose"]);

    let body_lines: Vec<&str> = frame
        .plain
        .lines()
        .map(|l| l.trim_end())
        .filter(|l| l.starts_with("┃ ") && l.contains(WRAP_TOKEN))
        .collect();

    assert!(
        body_lines.len() >= 3,
        "the long system body must wrap onto >=3 `┃ ` lines at COLUMNS={cols}.\nplain:\n{}",
        frame.plain,
    );

    for line in &body_lines {
        let line_width = visible_width(line) as usize;
        assert!(
            line_width <= cols as usize,
            "wrapped body line must fit COLUMNS={cols}; got {line_width}.\nline: {line:?}",
        );
        let content_width = visible_width(line.trim_start_matches("┃ ")) as usize;
        assert!(
            content_width <= cols as usize - PROMPT_CHROME_WIDTH,
            "body content must wrap within COLUMNS - chrome ({}); got {content_width}.\nline: {line:?}",
            cols as usize - PROMPT_CHROME_WIDTH,
        );
    }
}

/// An over-length agent prompt (>40 lines) renders front/back truncation
/// (20 leading + 10 trailing lines): the first and last rows survive while the
/// middle is dropped, observed in a real pane.
#[test]
#[serial(level2_terminal)]
fn level2_prompt_reporting_front_back_truncation_in_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

    let body = numbered_lines(50);
    let fx = Fixture::new(None, &body);
    let frame = capture_compose_tmux(&fx, 80, 60, &[]);
    let plain = &frame.plain;

    assert!(
        plain.contains("Agent Prompt"),
        "over-length prompt must still show the agent prompt header.\nplain:\n{plain}",
    );
    assert!(
        plain.contains("Line001"),
        "front portion of the >40-line prompt must render.\nplain:\n{plain}",
    );
    assert!(
        plain.contains("Line050"),
        "back portion of the >40-line prompt must render.\nplain:\n{plain}",
    );
    assert!(
        !plain.contains("Line025"),
        "middle lines must be dropped under FrontBack (20/10) truncation.\nplain:\n{plain}",
    );
}

/// WezTerm renders OSC8 hyperlinks faithfully, so it proves the system-prompt
/// summary emits a `file://` link to the source file. The link target must stay
/// inside the OSC8 control bytes and never leak into the visible text.
#[test]
#[serial(level2_terminal)]
fn level2_prompt_reporting_system_link_osc8_in_wezterm() {
    require_level!(Level::L2, WezTermHarness::available(), Backend::WezTerm);

    let fx = Fixture::new(Some(SYSTEM_PROMPT_SHORT), USER_PROMPT_SHORT);
    let mut harness = WezTermHarness::shared_or_spawn().expect("attach/spawn WezTerm");
    // This fixture asserts a *colored* surface under `FORCE_COLOR=1`, which an
    // ambient `NO_COLOR` out-votes — see `common::clear_no_color`.
    clear_no_color(&mut harness);

    // Clear the shared pane's screen *and* scrollback (`ESC[3J`) so the
    // scrollback capture below sees only this run's output — the pane is
    // reused across serial tests.
    harness
        .send_text(b"clear; printf '\\033[3J'\n")
        .expect("clear shared pane");
    let _ = biscuit_test_harness::wait_for_prompt(&mut harness);

    // Drive the compose, then read the scrollback rather than the returned
    // viewport frame. claudine prepends its built-in system prompt, so the
    // rendered System Prompt report is taller than the short shared pane and
    // its OSC8 hyperlink (near the report's top) scrolls into history — the
    // viewport-only capture never sees it.
    let _ = send_compose(&mut harness, &fx, 100, &[]);
    let frame = harness
        .capture_scrollback(400)
        .expect("scrollback capture failed");

    assert!(
        frame.raw.contains("\x1b]8;;file://"),
        "system-prompt summary must emit an OSC8 `file://` hyperlink.\nraw:\n{}",
        frame.raw,
    );
    assert!(
        frame.raw.contains("system-prompt.md"),
        "the OSC8 hyperlink must reference the system-prompt file.\nraw:\n{}",
        frame.raw,
    );
    assert!(
        !frame.plain.contains("]8;;"),
        "OSC8 control bytes must not appear in the visible text.\nplain:\n{}",
        frame.plain,
    );
}
