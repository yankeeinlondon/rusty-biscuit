//! Level 2 real-terminal captures for the interactive prompts that must run
//! inline: the one-shot provider picker (`compose` and `inline-compose`), the
//! required-missing `$schema` prompts, and the supplied partial-file chooser
//! and confirmation.
//!
//! The PTY suites prove from raw bytes that these prompts never enter the
//! alternate screen and that the answer reaches the provider. A PTY renders
//! nothing and keeps no scrollback, so it cannot show what the contract is
//! about: the prompt draws in a bounded region below the cursor, the lines
//! above it survive, and closing it leaves no blank screen behind. Each test
//! here runs the command inside tmux or WezTerm and reads that back from the
//! emulator:
//!
//! - two recognizable lines are printed before the command, one pushed toward
//!   the history by filler and one directly above the prompt;
//! - the capture taken while the prompt is open still shows the line above it,
//!   and the prompt's own widget rows are visible;
//! - after the prompt closes, the terminal history holds both lines and every
//!   filler line, the prompt's help hint is gone, and no run of blank rows
//!   longer than [`MAX_BLANK_RUN`] sits between the prompt's position and the
//!   command's exit marker;
//! - a submitted picker launches the chosen stub; a cancelled one launches
//!   nothing.
//!
//! Both picker entry paths are exercised: `compose` and `inline-compose` reach
//! the picker through different preparation code before sharing the target
//! resolver.
//!
//! Every command runs under `env -i` with a fixture `HOME`, cwd, and a `PATH`
//! of stub providers plus the minimal system directories, and with
//! `PLAYA_DRY_RUN=1` and a private spool, which each test proves stayed empty.
//!
//! Gating: `require_level!(Level::L2, ...)` per backend, so a test skips when
//! its terminal is unavailable and fails under
//! `BISCUIT_TEST_REQUIRED_BACKENDS`. WezTerm panes spawn in the background
//! workspace and never take focus.
//!
//! ```text
//! just test-l2 inline_prompt_scrollback
//! ```

use std::fs;
use std::io;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use biscuit_test_harness::tmux::TmuxHarness;
use biscuit_test_harness::wezterm::WezTermHarness;
use biscuit_test_harness::{CapturedFrame, TerminalHarness, strip_ansi};
use serial_test::serial;
use test_toolkit::{Backend, Level, require_level};

use crate::common;
use common::{
    CliProcessFixture, InlineAgentStub, claudine_bin, minimal_system_path, sh_quote,
    wait_for_exit_marker, wait_for_pane_text, write_executable,
};

/// The help hint every `biscuit-tui` standalone prompt overlays on its last
/// row. It must not survive the prompt.
const PROMPT_HINT: &str = "Enter=Submit";

/// Filler printed between the two sentinels, enough to push the first one
/// toward the history on a typical pane.
const FILLER_LINES: usize = 60;

/// Longest run of blank rows allowed between the prompt's position and the
/// exit marker. The commands print at most two consecutive blank lines of
/// their own; a viewport left uncleared, or a cleared screen, leaves a run as
/// tall as the prompt or the pane.
const MAX_BLANK_RUN: usize = 2;

/// Rows of WezTerm scrollback read back. The filler, the command, and its
/// provider report fit with headroom.
const WEZTERM_HISTORY_LINES: u32 = 600;

/// Backend-specific key injection and full-history capture.
trait PromptPane: TerminalHarness {
    fn press(&mut self, key: Key) -> io::Result<()>;
    fn history(&mut self) -> io::Result<CapturedFrame>;
}

#[derive(Clone, Copy, Debug)]
enum Key {
    Down,
    Enter,
    Backspace,
    CtrlC,
    Char(char),
}

/// One step of answering a prompt.
#[derive(Clone, Copy, Debug)]
enum Step {
    Press(Key),
    /// Wait until the pane shows this text, proving the widget drew it.
    Expect(&'static str),
}

impl PromptPane for TmuxHarness {
    fn press(&mut self, key: Key) -> io::Result<()> {
        match key {
            Key::Down => self.send_key("Down"),
            Key::Enter => self.send_key("Enter"),
            Key::Backspace => self.send_key("BSpace"),
            Key::CtrlC => self.send_key("C-c"),
            Key::Char(' ') => self.send_key("Space"),
            Key::Char(c) => self.send_key(&c.to_string()),
        }
    }

    fn history(&mut self) -> io::Result<CapturedFrame> {
        // `capture()` reads the visible pane only; `-S -` starts at the
        // oldest history line.
        let out = Command::new("tmux")
            .args([
                "capture-pane",
                "-p",
                "-e",
                "-S",
                "-",
                "-t",
                self.session_name(),
            ])
            .output()?;
        if !out.status.success() {
            return Err(io::Error::other(format!(
                "tmux capture-pane (history) failed: {}",
                String::from_utf8_lossy(&out.stderr)
            )));
        }
        Ok(CapturedFrame::from_raw(
            String::from_utf8_lossy(&out.stdout).into_owned(),
        ))
    }
}

impl PromptPane for WezTermHarness {
    fn press(&mut self, key: Key) -> io::Result<()> {
        match key {
            Key::Down => self.send_text(b"\x1b[B"),
            Key::Enter => self.send_text(b"\r"),
            Key::Backspace => self.send_text(b"\x7f"),
            Key::CtrlC => self.send_text(b"\x03"),
            Key::Char(c) => self.send_text(c.to_string().as_bytes()),
        }
    }

    fn history(&mut self) -> io::Result<CapturedFrame> {
        self.capture_scrollback(WEZTERM_HISTORY_LINES)
    }
}

/// One prompt interaction to drive and what it must leave behind.
struct Scenario {
    fixture: CliProcessFixture,
    args: Vec<String>,
    /// Text that proves the prompt is drawn and waiting.
    prompt_text: &'static str,
    /// Further texts that must be visible while the prompt is open.
    also_visible: &'static [&'static str],
    /// Keys answering the prompt, pressed in order with a redraw between.
    steps: &'static [Step],
    expected_status: &'static str,
    /// Provider stubs that must, and must not, have launched.
    launched: Vec<PathBuf>,
    not_launched: Vec<PathBuf>,
}

fn staged_fixture(name: &str) -> CliProcessFixture {
    let fixture = CliProcessFixture::named(name);
    // Without a user config, an interactive session runs the first-run
    // wizard, which would take the keys meant for the prompt.
    fixture.seed_user_config();
    fixture
}

fn marker_stub(fixture: &CliProcessFixture, binary: &str) -> PathBuf {
    let marker = fixture.cwd().join(format!("{binary}.launched"));
    write_executable(
        &fixture.bin_dir().join(binary),
        &format!(
            "#!/bin/sh\necho launched > {}\nexit 0\n",
            sh_quote(&marker.display().to_string())
        ),
    );
    marker
}

/// `compose` with Claude and Goose on `PATH` and no provider flag, so the
/// picker opens with Claude as the default.
fn compose_picker(name: &str, steps: &'static [Step], cancel: bool) -> Scenario {
    let fixture = staged_fixture(name);
    let claude = marker_stub(&fixture, "claude");
    let goose = marker_stub(&fixture, "goose");
    let doc = fixture.cwd().join("plan.md");
    fs::write(&doc, "Plan body.\n").unwrap();
    picker_scenario(fixture, "compose", doc, steps, cancel, claude, goose)
}

/// `inline-compose` on a document carrying frontmatter `prompt`, with inline
/// agent stubs that edit the document so a submitted run completes.
fn inline_compose_picker(name: &str, steps: &'static [Step], cancel: bool) -> Scenario {
    let fixture = staged_fixture(name);
    let doc = fixture.cwd().join("notes.md");
    fs::write(&doc, "---\nprompt: Write the notes\n---\nOriginal body\n").unwrap();
    let mut markers = Vec::new();
    for binary in ["claude", "goose"] {
        let marker = fixture.cwd().join(format!("{binary}.launched"));
        let prelude = format!(
            "echo launched > {}\n",
            sh_quote(&marker.display().to_string())
        );
        InlineAgentStub::new(&doc)
            .prelude(&prelude)
            .body("Agent notes\n")
            .install(fixture.bin_dir(), binary);
        markers.push(marker);
    }
    let goose = markers.pop().unwrap();
    let claude = markers.pop().unwrap();
    picker_scenario(fixture, "inline-compose", doc, steps, cancel, claude, goose)
}

fn picker_scenario(
    fixture: CliProcessFixture,
    command: &str,
    doc: PathBuf,
    steps: &'static [Step],
    cancel: bool,
    claude: PathBuf,
    goose: PathBuf,
) -> Scenario {
    let (launched, not_launched) = if cancel {
        (vec![], vec![claude, goose])
    } else {
        // `Down` moves the highlight off the default, so the launch proves
        // the keystroke chose the provider.
        (vec![goose], vec![claude])
    };
    Scenario {
        fixture,
        args: vec![command.to_string(), doc.display().to_string()],
        prompt_text: PROMPT_HINT,
        also_visible: &["Claude", "Goose"],
        steps,
        expected_status: if cancel { "1" } else { "0" },
        launched,
        not_launched,
    }
}

/// Every required-missing `$schema` prompt shape in one run: number, boolean,
/// single and multiple enum, and text. Missing properties are collected in
/// name order.
fn schema_prompts(name: &str) -> Scenario {
    let fixture = staged_fixture(name);
    let goose = marker_stub(&fixture, "goose");
    let doc = fixture.cwd().join("plan.md");
    fs::write(
        &doc,
        concat!(
            "---\n",
            "$schema:\n",
            "  count: 'number(required; integer)'\n",
            "  ready: 'boolean(required)'\n",
            "  tags: 'enum(red, green, blue; required)[]'\n",
            "  tier: 'enum(small, medium, large; required)'\n",
            "  topic: 'string(required)'\n",
            "---\n",
            "Plan {{count}} {{ready}} {{tags}} {{tier}} {{topic}}.\n",
        ),
    )
    .unwrap();
    Scenario {
        fixture,
        args: vec![
            "compose".into(),
            "--goose".into(),
            doc.display().to_string(),
        ],
        prompt_text: PROMPT_HINT,
        also_visible: &["count"],
        steps: SCHEMA_STEPS,
        expected_status: "0",
        launched: vec![goose],
        not_launched: vec![],
    }
}

/// Each `Expect` names a row the widget draws only when its viewport leaves
/// room for it beside the chrome: the number prompt's validation error, the
/// boolean switch, the last option of each enum, and the typed text.
const SCHEMA_STEPS: &[Step] = &[
    // count: an invalid value first, so the retry's error row must show.
    Step::Press(Key::Char('x')),
    Step::Press(Key::Enter),
    Step::Expect("is not a valid integer"),
    Step::Press(Key::Backspace),
    Step::Press(Key::Char('7')),
    Step::Press(Key::Enter),
    // ready
    Step::Expect("OFF | ON"),
    Step::Press(Key::Enter),
    // tags: toggle the first option.
    Step::Expect("☐ blue"),
    Step::Press(Key::Char(' ')),
    Step::Press(Key::Enter),
    // tier
    Step::Expect("○ large"),
    Step::Press(Key::Enter),
    // topic
    Step::Press(Key::Char('z')),
    Step::Press(Key::Char('e')),
    Step::Press(Key::Char('b')),
    Step::Press(Key::Char('r')),
    Step::Press(Key::Char('a')),
    Step::Expect("zebra"),
    Step::Press(Key::Enter),
];

fn seed_specs(fixture: &CliProcessFixture) {
    for dir in ["2026-06-30-style-everywhere", "2026-06-01-style-other"] {
        let path = fixture.cwd().join("features").join(dir).join("spec.md");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "---\ntitle: Spec\n---\nSpec body.\n").unwrap();
    }
}

/// A supplied partial for an eager `file(match)` property: `style` matches two
/// specs and opens the chooser, `everywhere` matches one and opens the
/// `Use this file? (Y/n)` confirmation.
fn partial_file(
    name: &str,
    partial: &str,
    prompt_text: &'static str,
    steps: &'static [Step],
) -> Scenario {
    let fixture = staged_fixture(name);
    seed_specs(&fixture);
    let goose = marker_stub(&fixture, "goose");
    let doc = fixture.cwd().join("plan.md");
    fs::write(
        &doc,
        "---\n$schema:\n  spec: 'file(required;match(**/*spec*.md);eager)'\n---\nPlan for {{spec}}.\n",
    )
    .unwrap();
    Scenario {
        fixture,
        args: vec![
            "compose".into(),
            "--goose".into(),
            doc.display().to_string(),
            format!("spec={partial}"),
        ],
        prompt_text,
        also_visible: &[],
        steps,
        expected_status: "0",
        launched: vec![goose],
        not_launched: vec![],
    }
}

fn run_scenario<H: PromptPane>(harness: &mut H, scenario: Scenario) {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let id = format!(
        "{}_{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    );
    // Built by `printf` from two words so the typed command line, which the
    // pane echoes, never contains the sentinel itself.
    let history_sentinel = format!("SCROLLBACK-SENTINEL-HISTORY-{id}");
    let above_sentinel = format!("SCROLLBACK-SENTINEL-ABOVE-{id}");
    let filler = format!("filler-{id}-");
    let marker = format!("PROMPT_{id}");

    let fixture = &scenario.fixture;
    let path = std::env::join_paths(
        std::iter::once(fixture.bin_dir().to_path_buf()).chain(minimal_system_path()),
    )
    .expect("join fixture PATH")
    .to_string_lossy()
    .into_owned();
    let env = [
        ("HOME", fixture.home().display().to_string()),
        ("PATH", path),
        ("TERM", "xterm-256color".to_string()),
        ("CLAUDINE_RENDEZVOUS_REPORT", "false".to_string()),
        ("PLAYA_DRY_RUN", "1".to_string()),
        (
            "PLAYA_SPOOL_DIR",
            fixture.audio_spool().display().to_string(),
        ),
    ]
    .iter()
    .map(|(key, value)| format!("{key}={}", sh_quote(value)))
    .collect::<Vec<_>>()
    .join(" ");
    let args = scenario
        .args
        .iter()
        .map(|arg| sh_quote(arg))
        .collect::<Vec<_>>()
        .join(" ");

    // The marker reports status through `&&`/`||`, not `"$?"`: a `?` typed
    // into the pane's shell triggers Atuin AI on a host that loads it.
    let script = format!(
        "printf '%s-%s\\n' SCROLLBACK SENTINEL-HISTORY-{id}; \
         i=1; while [ $i -le {FILLER_LINES} ]; do printf 'filler-%s-%s\\n' {id} $i; i=$((i+1)); done; \
         printf '%s-%s\\n' SCROLLBACK SENTINEL-ABOVE-{id}; \
         cd {cwd} && /usr/bin/env -i {env} {bin} {args} \
         && printf '\\n{marker}:0\\n' || printf '\\n{marker}:1\\n'",
        cwd = sh_quote(&fixture.cwd().display().to_string()),
        bin = claudine_bin(),
    );
    harness
        .send_command_with_env(&format!("/bin/sh -c {}", sh_quote(&script)), &[])
        .expect("send command");

    let open = wait_for_pane_text(harness, scenario.prompt_text, Duration::from_secs(20));
    let open_lines: Vec<&str> = open.plain.lines().collect();
    let above_row = open_lines
        .iter()
        .position(|line| line.trim_end() == above_sentinel)
        .unwrap_or_else(|| {
            panic!(
                "the line printed above the prompt is not on screen while it is open; plain:\n{}",
                open.plain
            )
        });
    let prompt_row = open_lines
        .iter()
        .position(|line| line.contains(scenario.prompt_text))
        .expect("prompt row");
    assert!(
        above_row < prompt_row,
        "the prompt must draw below the prior output; plain:\n{}",
        open.plain
    );
    for text in scenario.also_visible {
        assert!(
            open.plain.contains(text),
            "{text:?} is not visible while the prompt is open; plain:\n{}",
            open.plain
        );
    }
    for marker_file in scenario.launched.iter().chain(&scenario.not_launched) {
        assert!(
            !marker_file.exists(),
            "{} launched before the prompt was answered",
            marker_file.display()
        );
    }

    for step in scenario.steps {
        match step {
            Step::Press(key) => {
                harness.press(*key).expect("press key");
                harness.settle();
            }
            Step::Expect(text) => {
                wait_for_pane_text(harness, text, Duration::from_secs(10));
            }
        }
    }

    let (done, status) = wait_for_exit_marker(harness, &marker, Duration::from_secs(30));
    assert_eq!(
        status, scenario.expected_status,
        "unexpected exit status; plain:\n{}",
        done.plain
    );

    let history = harness.history().expect("capture history");
    let history_plain = strip_ansi(&history.raw);
    let lines: Vec<&str> = history_plain.lines().map(str::trim_end).collect();
    let history_row = lines
        .iter()
        .rposition(|line| *line == history_sentinel)
        .unwrap_or_else(|| {
            panic!("the history lost {history_sentinel:?}; history:\n{history_plain}")
        });
    let above_row = lines
        .iter()
        .rposition(|line| *line == above_sentinel)
        .unwrap_or_else(|| {
            panic!("the history lost {above_sentinel:?}; history:\n{history_plain}")
        });
    let expected_filler: Vec<String> = (1..=FILLER_LINES).map(|n| format!("{filler}{n}")).collect();
    assert_eq!(
        lines[history_row + 1..above_row].to_vec(),
        expected_filler,
        "the output printed before the prompt did not survive intact; history:\n{history_plain}"
    );
    let marker_line = format!("{marker}:{status}");
    let marker_row = lines[above_row..]
        .iter()
        .position(|line| line.trim() == marker_line)
        .map(|offset| above_row + offset)
        .unwrap_or_else(|| {
            panic!("exit marker missing after the prompt; history:\n{history_plain}")
        });
    let after_prompt = &lines[above_row + 1..marker_row];
    assert!(
        !after_prompt.iter().any(|line| line.contains(PROMPT_HINT)),
        "the prompt left its rows behind after closing; history:\n{history_plain}"
    );
    let longest_blank_run = after_prompt
        .split(|line| !line.trim().is_empty())
        .map(<[&str]>::len)
        .max()
        .unwrap_or(0);
    assert!(
        longest_blank_run <= MAX_BLANK_RUN,
        "{longest_blank_run} consecutive blank rows follow the prompt; history:\n{history_plain}"
    );

    for marker_file in &scenario.launched {
        assert!(
            marker_file.exists(),
            "{} did not launch; history:\n{history_plain}",
            marker_file.display()
        );
    }
    for marker_file in &scenario.not_launched {
        assert!(
            !marker_file.exists(),
            "{} launched but should not have; history:\n{history_plain}",
            marker_file.display()
        );
    }
    assert!(
        !fixture.audio_spool().exists(),
        "lifecycle audio was published to {}",
        fixture.audio_spool().display()
    );
}

const PICK_SECOND: &[Step] = &[Step::Press(Key::Down), Step::Press(Key::Enter)];
const CANCEL: &[Step] = &[Step::Press(Key::CtrlC)];
const CHOOSE_FIRST: &[Step] = &[Step::Press(Key::Enter)];
const ACCEPT: &[Step] = &[Step::Press(Key::Char('y'))];

fn tmux() -> TmuxHarness {
    TmuxHarness::shared_or_spawn().expect("tmux harness")
}

fn wezterm() -> WezTermHarness {
    WezTermHarness::shared_or_spawn().expect("attach/spawn WezTerm")
}

macro_rules! scenario_tests {
    ($($name:ident => $scenario:expr;)*) => {
        mod tmux_backend {
            use super::*;
            $(
                #[test]
                #[serial(level2_terminal)]
                fn $name() {
                    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);
                    run_scenario(&mut tmux(), $scenario);
                }
            )*
        }
        mod wezterm_backend {
            use super::*;
            $(
                #[test]
                #[serial(level2_terminal)]
                fn $name() {
                    require_level!(Level::L2, WezTermHarness::available(), Backend::WezTerm);
                    run_scenario(&mut wezterm(), $scenario);
                }
            )*
        }
    };
}

scenario_tests! {
    compose_provider_picker_submit_keeps_scrollback =>
        compose_picker("l2-picker-compose-submit", PICK_SECOND, false);
    compose_provider_picker_cancel_keeps_scrollback =>
        compose_picker("l2-picker-compose-cancel", CANCEL, true);
    inline_compose_provider_picker_submit_keeps_scrollback =>
        inline_compose_picker("l2-picker-inline-submit", PICK_SECOND, false);
    inline_compose_provider_picker_cancel_keeps_scrollback =>
        inline_compose_picker("l2-picker-inline-cancel", CANCEL, true);
    schema_required_prompts_keep_scrollback => schema_prompts("l2-schema-prompts");
    partial_file_chooser_keeps_scrollback =>
        partial_file("l2-partial-chooser", "style", PROMPT_HINT, CHOOSE_FIRST);
    partial_file_confirmation_keeps_scrollback =>
        partial_file("l2-partial-confirm", "everywhere", "Use this file? (Y/n)", ACCEPT);
}
