//! `real_` tier: every installed native provider submits Claudine's
//! interactive startup prompt as its first turn and keeps its terminal UI open
//! for a second one.
//!
//! The profile fleet test pins the argv each `prompt_delivery` builds; only the
//! real provider can show that this argv starts an interactive session whose
//! first turn is the prompt. Each test runs `claudine <provider> … -i` in a
//! detached tmux session, waits for the model's answer to the first turn, types
//! a second turn into the TUI, waits for that answer, then quits with Ctrl+C
//! and waits for the wrapper to exit. Every provider runs two shapes: a direct
//! command-line prompt and an `--edit` prompt written by a fake editor, which
//! is a Markdown bullet list so the `-`-prefixed delivery path is exercised
//! too.
//!
//! Pi has its own offline regression in `real_pi_interactive_startup`. Kimi
//! Code is absent: its interactive startup prompt is known to run one turn and
//! exit, and its contract waits on refreshed Kimi research.
//!
//! These tests use the host's real provider credentials and cost one or two
//! short model turns each. The provider runs with the host `HOME`, so its
//! stored login is used; the working directory is one stable directory per
//! test under the system temp directory, so a folder-trust answer a provider
//! persists is recorded once rather than per run.
//!
//! ## How to run
//!
//! Opt-in: skips unless `CLAUDINE_CONTRACT_REAL=1`, the provider binary is on
//! `PATH`, and tmux is available. Unix only.
//!
//! ```sh
//! just test-real real_native_interactive_startup::          # in claudine/
//! just test-real real_native_interactive_startup::claude    # one provider
//! ```

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use biscuit_test_harness::tmux::TmuxHarness;
use biscuit_test_harness::{CapturedFrame, TerminalHarness};

use crate::common;
use common::{CliProcessFixture, claudine_bin, init_git_repo, sh_quote, write, write_executable};

/// Covers provider start-up plus one model turn.
const FIRST_TURN_DEADLINE: Duration = Duration::from_secs(180);
const SECOND_TURN_DEADLINE: Duration = Duration::from_secs(120);
const EXIT_DEADLINE: Duration = Duration::from_secs(30);

/// Neither answer token appears in its own prompt, so a match is the model's
/// reply rather than the TUI echoing the user turn.
const FIRST_PROMPT: &str = "Concatenate FOX and TROT with no space and reply with only the result";
const FIRST_ANSWER: &str = "FOXTROT";
const EDITED_PROMPT: &str = "- Concatenate FOX and TROT with no space\n- reply with only the result\n";
const SECOND_PROMPT: &str = "Concatenate BRA and VO with no space and reply with only the result";
const SECOND_ANSWER: &str = "BRAVO";

/// Replaces the editor buffer with `$CLAUDINE_EDITOR_PROMPT`'s file.
const FAKE_EDITOR: &str = "#!/bin/sh\n/bin/cat \"$CLAUDINE_EDITOR_PROMPT\" > \"$1\"\n";

/// A start-up screen a provider may draw before or after the first turn, and
/// the keys that dismiss it. Each is answered at most once per launch.
struct Dialog {
    needle: &'static str,
    keys: &'static [&'static str],
}

struct NativeProvider {
    /// The `claudine` subcommand.
    command: &'static str,
    binary: &'static str,
    dialogs: &'static [Dialog],
}

const CLAUDE: NativeProvider = NativeProvider {
    command: "claude",
    binary: "claude",
    // The first option is "No, exit".
    dialogs: &[Dialog { needle: "Yes, I trust this folder", keys: &["Down", "Enter"] }],
};

const CODEX: NativeProvider = NativeProvider {
    command: "codex",
    binary: "codex",
    // Drawn by Codex 0.157 when its experimental background server cannot
    // start; unrelated to how the prompt is delivered.
    dialogs: &[
        Dialog { needle: "Run without daemon this time", keys: &["1"] },
        // The highlighted default is "1. Trust and continue".
        Dialog { needle: "Trust this folder?", keys: &["Enter"] },
    ],
};

const GEMINI: NativeProvider = NativeProvider {
    command: "gemini",
    binary: "gemini",
    // "3. Don't trust" answers for this launch without persisting a trust entry.
    dialogs: &[Dialog { needle: "Do you trust the files in this folder?", keys: &["3"] }],
};

const GOOSE: NativeProvider = NativeProvider { command: "goose", binary: "goose", dialogs: &[] };

const OPENCODE: NativeProvider = NativeProvider { command: "opencode", binary: "opencode", dialogs: &[] };

const QWEN: NativeProvider = NativeProvider { command: "qwen", binary: "qwen", dialogs: &[] };

const KILO: NativeProvider = NativeProvider { command: "kilo", binary: "kilo", dialogs: &[] };

const ANTIGRAVITY: NativeProvider = NativeProvider {
    command: "antigravity",
    binary: "agy",
    // The highlighted default is "Yes, I trust this folder".
    dialogs: &[Dialog { needle: "Do you trust the contents of this project?", keys: &["Enter"] }],
};

enum Shape {
    Direct,
    Edited,
}

/// The provider binary, or `None` (with the reason printed) when this opt-in
/// tier cannot run here.
fn available(provider: &NativeProvider) -> Option<PathBuf> {
    let name = format!("real_native_interactive_startup ({})", provider.command);
    if std::env::var("CLAUDINE_CONTRACT_REAL").as_deref() != Ok("1") {
        eprintln!("skipping {name} (set CLAUDINE_CONTRACT_REAL=1 to run)");
        return None;
    }
    let Ok(binary) = which::which(provider.binary) else {
        eprintln!("skipping {name} (binary `{}` not on PATH)", provider.binary);
        return None;
    };
    if !TmuxHarness::available() {
        eprintln!("skipping {name} (tmux not available)");
        return None;
    }
    let version = std::process::Command::new(&binary).arg("--version").output();
    let version = version.map(|out| String::from_utf8_lossy(&out.stdout).trim().to_string()).unwrap_or_default();
    eprintln!("{name}: {} reports {version:?}", binary.display());
    Some(binary)
}

fn unique_marker() -> String {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    format!("NATIVE_START_{}_{}", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed))
}

/// A stable, git-initialized working directory for one provider and shape.
fn workspace(provider: &NativeProvider, shape: &Shape) -> PathBuf {
    let shape = match shape {
        Shape::Direct => "direct",
        Shape::Edited => "edited",
    };
    let dir = std::env::temp_dir()
        .join("claudine-real-interactive-startup")
        .join(format!("{}-{shape}", provider.command));
    fs::create_dir_all(&dir).unwrap();
    if !dir.join(".git").exists() {
        assert!(init_git_repo(&dir), "git init {}", dir.display());
    }
    dir
}

/// Writes the launcher for `claudine <claudine_args>`, returning its path.
///
/// The pane's login shell does not inherit this process's environment, so the
/// launcher carries `HOME` and `PATH` (the fixture `bin` ahead of the host
/// `PATH`) explicitly. Lifecycle audio and rendezvous reporting are silenced.
fn write_launcher(fixture: &CliProcessFixture, cwd: &Path, claudine_args: &[&str], marker: &str) -> PathBuf {
    let root = fixture.workspace_path();
    let editor = fixture.bin_dir().join("fake-editor");
    let editor_prompt = root.join("editor-prompt.md");
    write(&editor_prompt, EDITED_PROMPT);
    write_executable(&editor, FAKE_EDITOR);

    let home = std::env::var_os("HOME").expect("HOME");
    let host_path = std::env::var_os("PATH").unwrap_or_default();
    let path = std::env::join_paths(
        std::iter::once(fixture.bin_dir().to_path_buf()).chain(std::env::split_paths(&host_path)),
    )
    .expect("join PATH");
    let quoted = |value: &Path| sh_quote(&value.display().to_string());
    let variables = [
        ("HOME", quoted(Path::new(&home))),
        ("PATH", quoted(Path::new(&path))),
        ("TERM", "xterm-256color".to_string()),
        ("EDITOR", quoted(&editor)),
        ("CLAUDINE_EDITOR_PROMPT", quoted(&editor_prompt)),
        ("CLAUDINE_RENDEZVOUS_REPORT", "false".to_string()),
        ("PLAYA_DRY_RUN", "1".to_string()),
        ("PLAYA_SPOOL_DIR", quoted(&fixture.audio_spool())),
    ]
    .map(|(name, value)| format!("{name}={value}"))
    .join(" ");
    let argv = claudine_args.iter().map(|arg| sh_quote(arg)).collect::<Vec<_>>().join(" ");
    let script = format!(
        "#!/bin/sh\nprintf '\\033[2J\\033[H'\ncd {cwd} || exit 1\n\
         if /usr/bin/env {variables} {claudine} {argv}; then\n  \
         printf '\\n{marker}:0\\n'\nelse\n  printf '\\n{marker}:1\\n'\nfi\n",
        cwd = quoted(cwd),
        claudine = sh_quote(claudine_bin()),
    );
    let launcher = root.join("launch.sh");
    write_executable(&launcher, &script);
    launcher
}

/// Whether a line of the pane shows `token` as a word outside an echo of a
/// user turn (every prompt here contains "Concatenate"; answers do not).
fn shows_answer(frame: &CapturedFrame, token: &str) -> bool {
    frame.plain.lines().any(|line| {
        !line.contains("Concatenate")
            && line.split(|c: char| !c.is_ascii_alphanumeric()).any(|word| word == token)
    })
}

/// Polls the pane until `token` is answered, dismissing each known dialog once.
fn wait_for_answer(
    harness: &mut TmuxHarness,
    provider: &NativeProvider,
    answered_dialogs: &mut Vec<&'static str>,
    token: &str,
    deadline: Duration,
    turn: &str,
) -> CapturedFrame {
    let until = Instant::now() + deadline;
    loop {
        let frame = harness.capture().expect("capture pane");
        if shows_answer(&frame, token) {
            return frame;
        }
        for dialog in provider.dialogs {
            if !answered_dialogs.contains(&dialog.needle) && frame.plain.contains(dialog.needle) {
                eprintln!("{}: answering start-up dialog {:?}", provider.command, dialog.needle);
                for key in dialog.keys {
                    harness.send_key(key).expect("answer dialog");
                    std::thread::sleep(Duration::from_millis(300));
                }
                answered_dialogs.push(dialog.needle);
            }
        }
        assert!(
            Instant::now() < until,
            "{} never answered the {turn} turn with {token:?} within {deadline:?}:\n{}",
            provider.command,
            frame.plain
        );
        std::thread::sleep(Duration::from_millis(500));
    }
}

/// Launches `claudine <provider> … -i` and proves that the provider answered
/// the startup prompt as its first turn, answered a second turn typed into the
/// same TUI, and quit.
fn assert_native_interactive_startup(provider: &NativeProvider, shape: Shape) {
    if available(provider).is_none() {
        return;
    }
    let fixture = CliProcessFixture::named(&format!("real-native-{}", provider.command));
    let cwd = workspace(provider, &shape);
    let marker = unique_marker();
    let claudine_args: Vec<&str> = match shape {
        Shape::Direct => vec![provider.command, FIRST_PROMPT, "-i"],
        Shape::Edited => vec![provider.command, "--edit", "-i"],
    };
    let launcher = write_launcher(&fixture, &cwd, &claudine_args, &marker);

    // Owned, so the session and any provider left in it are killed on drop.
    let mut harness = TmuxHarness::new();
    harness.spawn_shell().expect("spawn tmux session");
    harness
        .send_text(format!("/bin/sh {}\n", sh_quote(&launcher.display().to_string())).as_bytes())
        .expect("send launcher");

    let mut answered_dialogs = Vec::new();
    wait_for_answer(&mut harness, provider, &mut answered_dialogs, FIRST_ANSWER, FIRST_TURN_DEADLINE, "first");
    let before_second = harness.capture().expect("capture pane");
    assert!(
        !shows_answer(&before_second, SECOND_ANSWER),
        "{} shows {SECOND_ANSWER:?} before the second turn was typed:\n{}",
        provider.command,
        before_second.plain
    );

    harness.send_text(SECOND_PROMPT.as_bytes()).expect("type second turn");
    // Lets the TUI finish treating the typed burst as input before the submit.
    std::thread::sleep(Duration::from_millis(500));
    harness.send_key("Enter").expect("submit second turn");
    let answered =
        wait_for_answer(&mut harness, provider, &mut answered_dialogs, SECOND_ANSWER, SECOND_TURN_DEADLINE, "second");
    eprintln!("{}: both turns answered:\n{}", provider.command, answered.plain);

    let until = Instant::now() + EXIT_DEADLINE;
    let exit = format!("{marker}:");
    loop {
        harness.send_key("C-c").expect("send Ctrl+C");
        std::thread::sleep(Duration::from_millis(200));
        harness.send_key("C-c").expect("send Ctrl+C");
        std::thread::sleep(Duration::from_millis(1000));
        let frame = harness.capture().expect("capture pane");
        if frame.plain.lines().any(|line| line.trim().starts_with(&exit)) {
            break;
        }
        assert!(Instant::now() < until, "{} did not quit on Ctrl+C:\n{}", provider.command, frame.plain);
    }
}

macro_rules! native_provider_tests {
    ($($module:ident => $provider:expr),+ $(,)?) => {$(
        mod $module {
            use super::*;

            #[test]
            fn real_direct_interactive_prompt_is_answered_and_session_continues() {
                assert_native_interactive_startup(&$provider, Shape::Direct);
            }

            #[test]
            fn real_edited_interactive_prompt_is_answered_and_session_continues() {
                assert_native_interactive_startup(&$provider, Shape::Edited);
            }
        }
    )+};
}

native_provider_tests! {
    claude => CLAUDE,
    codex => CODEX,
    gemini => GEMINI,
    goose => GOOSE,
    opencode => OPENCODE,
    qwen => QWEN,
    kilo => KILO,
    antigravity => ANTIGRAVITY,
}
