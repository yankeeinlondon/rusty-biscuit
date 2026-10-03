//! `real_` tier: Pi submits Claudine's interactive startup prompt as its first
//! turn and keeps its terminal UI for the next one (spec
//! `2026-09-18-edit-integration`, AC12).
//!
//! The profile unit tests pin the argv Claudine builds (`pi … -- <prompt>`);
//! only the real Pi can show that this argv starts an interactive session whose
//! first turn is the prompt. Each test runs `claudine pi … -i` against the
//! installed Pi in a detached tmux session, waits for the model's reply to the
//! first turn, types a second turn into the TUI, waits for that reply, then
//! quits. Replies come from the deterministic `claudine-probe` model in
//! `tests/fixtures/steering/pi-probe.ts`, which answers `ACK:<marker>` for the
//! fixture marker in the latest user turn and records every turn it saw in
//! `probe.json`. No network or credentials are used.
//!
//! The four launches cover the direct prompt, the `--edit` prompt, a prompt of
//! about 2 KB (upstream Pi issue #9200 reports a SIGKILL for positional
//! messages from about 1 KB), and a prompt starting with `@`, which Pi would
//! otherwise read as a file reference.
//!
//! ## How to run
//!
//! Opt-in: skips unless `CLAUDINE_CONTRACT_REAL=1`, `pi` (0.84.3 or later) is
//! on `PATH`, and tmux is available. Unix only.
//!
//! ```sh
//! just test-real real_pi_interactive_startup::   # in claudine/
//! ```

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use biscuit_test_harness::TerminalHarness;
use biscuit_test_harness::tmux::TmuxHarness;
use serde_json::Value;
use serial_test::serial;

use crate::common;
use common::{CliProcessFixture, claudine_bin, sh_quote, wait_for_pane_text, write, write_executable};

const FIRST_MARKER: &str = "TEMPLATE_NONCE";
const SECOND_MARKER: &str = "STEERING_NONCE";
const TURN_DEADLINE: Duration = Duration::from_secs(20);

/// Replaces the editor buffer with `$CLAUDINE_EDITOR_PROMPT`'s file.
const FAKE_EDITOR: &str = "#!/bin/sh\n/bin/cat \"$CLAUDINE_EDITOR_PROMPT\" > \"$1\"\n";

/// Sits in front of the real Pi as `pi`: records the argv Claudine built, one
/// file per argument, then hands over to the real binary.
const PI_SHIM: &str = r#"#!/bin/sh
i=0
for arg in "$@"; do
  printf '%s' "$arg" > "$CLAUDINE_PI_ARGV_DIR/arg.$i"
  i=$((i + 1))
done
echo "$i" > "$CLAUDINE_PI_ARGV_DIR/argc"
exec "$CLAUDINE_REAL_PI" "$@"
"#;

/// The installed Pi, or `None` (with the reason printed) when this opt-in
/// tier cannot run here.
fn real_pi() -> Option<PathBuf> {
    if std::env::var("CLAUDINE_CONTRACT_REAL").as_deref() != Ok("1") {
        eprintln!("skipping real_pi_interactive_startup (set CLAUDINE_CONTRACT_REAL=1 to run)");
        return None;
    }
    let Some(pi) = std::env::var_os("PATH").and_then(|path| {
        std::env::split_paths(&path).map(|dir| dir.join("pi")).find(|candidate| candidate.is_file())
    }) else {
        eprintln!("skipping real_pi_interactive_startup (binary `pi` not on PATH)");
        return None;
    };
    if !TmuxHarness::available() {
        eprintln!("skipping real_pi_interactive_startup (tmux not available)");
        return None;
    }
    let version = std::process::Command::new(&pi).arg("--version").output().expect("pi --version");
    eprintln!("real_pi_interactive_startup: {} is Pi {}", pi.display(), String::from_utf8_lossy(&version.stdout).trim());
    Some(pi)
}

fn unique_marker() -> String {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    format!("PI_START_{}_{}", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed))
}

/// Writes the launcher for `claudine <claudine_args>`, returning its path.
///
/// Pi's fixture model, extension, and session directory come from
/// `settings.json` in its agent directory rather than from Pi flags, so
/// Claudine's argv carries only the wrapper arguments and the prompt. Pi is a
/// Node program, so the pane keeps the host `PATH` (behind the fixture `bin`,
/// which holds the `pi` shim and the fake editor); everything else is the
/// fixture's.
fn write_launcher(
    fixture: &CliProcessFixture,
    real_pi: &Path,
    claudine_args: &[&str],
    editor_prompt: Option<&str>,
    marker: &str,
) -> PathBuf {
    let root = fixture.workspace_path();
    let probe = root.join("probe");
    let sessions = root.join("sessions");
    let agent = root.join("agent");
    let argv_dir = root.join("pi-argv");
    for dir in [&probe, &sessions, &agent, &argv_dir] {
        fs::create_dir_all(dir).unwrap();
    }
    let extension = biscuit_test_harness::manifest_dir!().join("tests/fixtures/steering/pi-probe.ts");
    let settings = serde_json::json!({
        "defaultProvider": "claudine-probe",
        "defaultModel": "fixture",
        "defaultThinkingLevel": "off",
        "extensions": [extension],
        "sessionDir": sessions,
    });
    write(&agent.join("settings.json"), &settings.to_string());
    write_executable(&fixture.bin_dir().join("pi"), PI_SHIM);
    let editor_prompt_file = root.join("editor-prompt.md");
    if let Some(prompt) = editor_prompt {
        write(&editor_prompt_file, prompt);
        write_executable(&fixture.bin_dir().join("fake-editor"), FAKE_EDITOR);
    }

    let host_path = std::env::var_os("PATH").unwrap_or_default();
    let path = std::env::join_paths(
        std::iter::once(fixture.bin_dir().to_path_buf()).chain(std::env::split_paths(&host_path)),
    )
    .expect("join PATH");
    let quoted = |value: &Path| sh_quote(&value.display().to_string());
    let variables = [
        ("HOME", quoted(fixture.home())),
        ("PATH", quoted(Path::new(&path))),
        ("TERM", "xterm-256color".to_string()),
        ("EDITOR", "fake-editor".to_string()),
        ("CLAUDINE_EDITOR_PROMPT", quoted(&editor_prompt_file)),
        ("CLAUDINE_RENDEZVOUS_REPORT", "false".to_string()),
        ("PLAYA_DRY_RUN", "1".to_string()),
        ("PLAYA_SPOOL_DIR", quoted(&fixture.audio_spool())),
        ("PI_CODING_AGENT_DIR", quoted(&agent)),
        ("PI_OFFLINE", "1".to_string()),
        ("PI_TELEMETRY", "0".to_string()),
        ("CLAUDINE_PI_PROBE_DIR", quoted(&probe)),
        ("CLAUDINE_PI_ARGV_DIR", quoted(&argv_dir)),
        ("CLAUDINE_REAL_PI", quoted(real_pi)),
    ]
    .map(|(name, value)| format!("{name}={value}"))
    .join(" ");
    let argv = claudine_args.iter().map(|arg| sh_quote(arg)).collect::<Vec<_>>().join(" ");
    let script = format!(
        "#!/bin/sh\nprintf '\\033[2J\\033[H'\ncd {cwd} || exit 1\n\
         if /usr/bin/env -i {variables} {claudine} {argv}; then\n  \
         printf '\\n{marker}:0\\n'\nelse\n  printf '\\n{marker}:1\\n'\nfi\n",
        cwd = quoted(fixture.cwd()),
        claudine = sh_quote(claudine_bin()),
    );
    let launcher = root.join("launch.sh");
    write_executable(&launcher, &script);
    launcher
}

/// The fixture markers of every user turn the probe model has seen.
fn probed_user_turns(fixture: &CliProcessFixture) -> Vec<String> {
    let path = fixture.workspace_path().join("probe").join("probe.json");
    let record: Value = serde_json::from_str(&fs::read_to_string(&path).expect("probe.json"))
        .expect("probe.json is JSON");
    record["userTexts"]
        .as_array()
        .expect("userTexts")
        .iter()
        .map(|text| text.as_str().expect("user text").to_string())
        .collect()
}

/// The argv Claudine passed to Pi, as the shim recorded it.
fn recorded_pi_argv(fixture: &CliProcessFixture) -> Vec<String> {
    let dir = fixture.workspace_path().join("pi-argv");
    let argc: usize = fs::read_to_string(dir.join("argc"))
        .expect("Pi was never launched")
        .trim()
        .parse()
        .expect("argc");
    (0..argc)
        .map(|index| fs::read_to_string(dir.join(format!("arg.{index}"))).expect("recorded argument"))
        .collect()
}

/// Launches `claudine <claudine_args>` and proves that Pi received
/// `-- <message>`, submitted it as the first turn, answered a second turn
/// typed into the TUI, and quit.
fn assert_interactive_startup(name: &str, real_pi: &Path, claudine_args: &[&str], editor_prompt: Option<&str>, message: &str) {
    let fixture = CliProcessFixture::named(name);
    fixture.seed_user_config();
    let marker = unique_marker();
    let launcher = write_launcher(&fixture, real_pi, claudine_args, editor_prompt, &marker);

    // Owned, so the session and any Pi left in it are killed on drop.
    let mut harness = TmuxHarness::new();
    harness.spawn_shell().expect("spawn tmux session");
    harness
        .send_text(format!("/bin/sh {}\n", sh_quote(&launcher.display().to_string())).as_bytes())
        .expect("send launcher");

    let first_reply = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        wait_for_pane_text(&mut harness, &format!("ACK:{FIRST_MARKER}"), TURN_DEADLINE)
    }));
    let argv = recorded_pi_argv(&fixture);
    assert!(first_reply.is_ok(), "Pi never answered the first turn; Claudine passed {argv:?}");
    assert_eq!(
        argv[argv.len().saturating_sub(2)..],
        ["--".to_string(), message.to_string()],
        "the startup prompt must follow `--` as one message: {argv:?}"
    );
    assert_eq!(probed_user_turns(&fixture), [FIRST_MARKER], "the startup prompt must be the only first turn");

    harness.send_text(SECOND_MARKER.as_bytes()).expect("type second turn");
    harness.send_key("Enter").expect("submit second turn");
    let frame = wait_for_pane_text(&mut harness, &format!("ACK:{SECOND_MARKER}"), TURN_DEADLINE);
    assert_eq!(
        probed_user_turns(&fixture),
        [FIRST_MARKER, SECOND_MARKER],
        "the TUI must take a second turn in the same session:\n{}",
        frame.plain
    );

    let deadline = Instant::now() + Duration::from_secs(20);
    let exit = format!("{marker}:");
    loop {
        harness.send_key("C-c").expect("send Ctrl+C");
        std::thread::sleep(Duration::from_millis(200));
        harness.send_key("C-c").expect("send Ctrl+C");
        std::thread::sleep(Duration::from_millis(500));
        let frame = harness.capture().expect("capture pane");
        if frame.plain.lines().any(|line| line.trim().starts_with(&exit)) {
            break;
        }
        assert!(Instant::now() < deadline, "Pi did not quit on Ctrl+C:\n{}", frame.plain);
    }
}

#[test]
#[serial(real_pi_tui)]
fn real_pi_direct_interactive_prompt_is_the_first_turn() {
    let Some(pi) = real_pi() else { return };
    let prompt = format!("{FIRST_MARKER}: reply with the acknowledgement.");
    assert_interactive_startup("real-pi-direct", &pi, &["pi", &prompt, "-i"], None, &prompt);
}

#[test]
#[serial(real_pi_tui)]
fn real_pi_edited_interactive_prompt_is_the_first_turn() {
    let Some(pi) = real_pi() else { return };
    let edited = format!("- {FIRST_MARKER} from the editor\n- a second bullet\n");
    assert_interactive_startup("real-pi-edit", &pi, &["pi", "--edit", "-i"], Some(&edited), edited.trim_end());
}

/// Upstream Pi issue #9200 reports a SIGKILL for a positional message of
/// about 1 KB or more.
#[test]
#[serial(real_pi_tui)]
fn real_pi_two_kilobyte_interactive_prompt_is_the_first_turn() {
    let Some(pi) = real_pi() else { return };
    let filler = "The quick brown fox jumps over the lazy dog. ".repeat(46);
    let prompt = format!("{FIRST_MARKER}\n\n{}", filler.trim_end());
    assert!(prompt.len() >= 2048, "prompt is {} bytes", prompt.len());
    assert_interactive_startup("real-pi-2kb", &pi, &["pi", &prompt, "-i"], None, &prompt);
}

/// Without Claudine's leading space, Pi would read this prompt as `@file`.
#[test]
#[serial(real_pi_tui)]
fn real_pi_interactive_prompt_starting_with_at_is_a_message() {
    let Some(pi) = real_pi() else { return };
    let prompt = format!("@{FIRST_MARKER} is not a file");
    assert_interactive_startup("real-pi-at", &pi, &["pi", &prompt, "-i"], None, &format!(" {prompt}"));
}
