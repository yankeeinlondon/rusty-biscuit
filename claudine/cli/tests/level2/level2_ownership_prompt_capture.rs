//! Level 2: the provider-argument ambiguity chooser in a real terminal.
//!
//! The document lists Claude and Codex as candidates, so `-c foo` after the
//! file is read two ways: Claude's `-c` takes no value, Codex's takes one.
//! At a terminal Claudine asks which agent the arguments are for. The answer
//! decides who owns `foo` and nothing else: the run still resolves its own
//! provider (Codex, the only one installed), and the resolved-provider check
//! judges the result.
//!
//! `level1_ownership_prompt_pty.rs` proves both outcomes from manufactured
//! PTY bytes. This binary runs the shipped `claudine` in a detached tmux pane
//! for every composition entrypoint that owns caller arguments (`compose`,
//! `inline-compose`, `sequence`) and each answer, so the question and both
//! choices are read back from what the emulator drew and the answer arrives
//! through the pane's input path:
//!
//! - **Codex:** the fake Codex receives `-c foo` as one switch and its value;
//! - **Claude:** `foo` is Claudine's, so the actual Codex launch has `-c`
//!   without a value and the resolved-provider check refuses it before any
//!   spawn.
//!
//! Every command runs under `env -i` with a fixture `HOME`, cwd, and a
//! `PATH` of the fake Codex plus the minimal system directories, with
//! `PLAYA_DRY_RUN=1` and a private spool. tmux sessions are detached, so no
//! window opens or takes focus.
//!
//! Run via the canonical recipe: `just test-l2 ownership_prompt_capture`.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use biscuit_test_harness::TerminalHarness;
use biscuit_test_harness::tmux::TmuxHarness;
use serial_test::serial;
use test_toolkit::{Backend, Level, require_level};

use crate::common;
use common::{
    CliProcessFixture, claudine_bin, minimal_system_path, sh_quote, wait_for_exit_marker,
    wait_for_pane_text, write, write_executable,
};

const QUESTION: &str = "Which agent are these arguments for?";

/// Records each launch's argv, `\037`-separated, in its own file, and gives an
/// inline document a new body line so the run passes its body-change check.
const RECORDING_CODEX: &str = r#"#!/bin/sh
for arg in "$@"; do printf '%s\037' "$arg"; done > "$CLAUDINE_RECORD_DIR/launch-$$"
if [ -f "$CLAUDINE_INLINE_DOC" ]; then printf '\nAgent wrote this.\n' >> "$CLAUDINE_INLINE_DOC"; fi
exit 0
"#;

#[derive(Clone, Copy, Debug)]
enum Entrypoint {
    Compose,
    InlineCompose,
    Sequence,
}

#[derive(Clone, Copy, Debug)]
enum Answer {
    Claude,
    Codex,
}

/// One answered chooser and everything it left behind.
struct Run {
    fixture: CliProcessFixture,
    chooser: String,
    frame: String,
    status: String,
}

impl Run {
    fn record_dir(&self) -> PathBuf {
        self.fixture.cwd().join("record")
    }

    fn launches(&self) -> Vec<Vec<String>> {
        let mut launches: Vec<Vec<String>> = fs::read_dir(self.record_dir())
            .expect("record dir")
            .map(|entry| {
                fs::read_to_string(entry.expect("record entry").path())
                    .expect("launch record")
                    .split('\u{1f}')
                    .filter(|arg| !arg.is_empty())
                    .map(str::to_owned)
                    .collect()
            })
            .collect();
        launches.sort();
        launches
    }
}

fn unique_marker() -> String {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    format!("OWNERSHIP_{}_{}", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed))
}

/// Writes the entrypoint's documents and returns the file the command names.
fn stage_documents(fixture: &CliProcessFixture, entrypoint: Entrypoint) -> PathBuf {
    let plan = fixture.cwd().join("plan.md");
    write(&plan, "---\nagent: [claude, codex]\n---\nPlan body.\n");
    match entrypoint {
        Entrypoint::Compose => plan,
        Entrypoint::InlineCompose => {
            let doc = fixture.cwd().join("inline.md");
            write(&doc, "---\nagent: [claude, codex]\nprompt: Write the plan.\n---\nOld body\n");
            doc
        }
        Entrypoint::Sequence => {
            let seq = fixture.cwd().join("seq.md");
            write(
                &seq,
                "---\nagent: [claude, codex]\nsequence:\n  - name: plan\n    prompt: plan.md\n---\nBody\n",
            );
            seq
        }
    }
}

/// Writes the launcher the pane runs. The exit marker is printed by the
/// script rather than typed with `$?`, which opens Atuin AI on hosts that
/// load it.
fn write_launcher(fixture: &CliProcessFixture, command: &str, document: &Path, marker: &str) -> PathBuf {
    let record_dir = fixture.cwd().join("record");
    fs::create_dir_all(&record_dir).unwrap();
    write_executable(&fixture.bin_dir().join("codex"), RECORDING_CODEX);

    let path = std::env::join_paths(
        std::iter::once(fixture.bin_dir().to_path_buf()).chain(minimal_system_path()),
    )
    .expect("join fixture PATH");
    let quoted = |value: &Path| sh_quote(&value.display().to_string());
    let variables = [
        ("HOME", quoted(fixture.home())),
        ("PATH", quoted(Path::new(&path))),
        ("TERM", "xterm-256color".to_string()),
        ("CLAUDINE_RENDEZVOUS_REPORT", "false".to_string()),
        ("PLAYA_DRY_RUN", "1".to_string()),
        ("PLAYA_SPOOL_DIR", quoted(&fixture.audio_spool())),
        ("CLAUDINE_RECORD_DIR", quoted(&record_dir)),
        ("CLAUDINE_INLINE_DOC", quoted(&fixture.cwd().join("inline.md"))),
    ]
    .map(|(name, value)| format!("{name}={value}"))
    .join(" ");
    let script = format!(
        "#!/bin/sh\nprintf '\\033[2J\\033[H'\ncd {cwd} || exit 1\n\
         /usr/bin/env -i {variables} {claudine} {command} {document} -c foo\n\
         printf '\\n{marker}:%s\\n' \"$?\"\n",
        cwd = quoted(fixture.cwd()),
        claudine = sh_quote(claudine_bin()),
        document = quoted(document),
    );
    let launcher = fixture.workspace_path().join("launch.sh");
    write_executable(&launcher, &script);
    launcher
}

/// Runs `claudine <entrypoint> <file> -c foo` in the pane, captures the open
/// chooser, answers it through tmux, and waits for the command to exit.
fn answer(harness: &mut TmuxHarness, entrypoint: Entrypoint, answer: Answer) -> Run {
    let fixture = CliProcessFixture::named("l2-ownership-prompt");
    fixture.seed_user_config();
    let command = match entrypoint {
        Entrypoint::Compose => "compose",
        Entrypoint::InlineCompose => "inline-compose",
        Entrypoint::Sequence => "sequence",
    };
    let document = stage_documents(&fixture, entrypoint);
    let marker = unique_marker();
    let launcher = write_launcher(&fixture, command, &document, &marker);

    harness
        .send_text(format!("/bin/sh {}\n", sh_quote(&launcher.display().to_string())).as_bytes())
        .expect("send launcher");
    let chooser = wait_for_pane_text(harness, QUESTION, Duration::from_secs(30));
    // The question renders before the widget; wait for its last choice.
    let chooser = if chooser.plain.lines().any(|line| line.contains("Codex")) {
        chooser
    } else {
        wait_for_pane_text(harness, "Codex", Duration::from_secs(10))
    };
    assert!(
        fs::read_dir(fixture.cwd().join("record")).unwrap().next().is_none(),
        "{entrypoint:?}: Codex launched before the chooser was answered:\n{}",
        chooser.plain
    );

    if let Answer::Codex = answer {
        harness.send_key("Down").expect("move to Codex");
        harness.settle();
    }
    harness.send_key("Enter").expect("submit the chooser");
    let (frame, status) = wait_for_exit_marker(harness, &marker, Duration::from_secs(30));
    Run {
        fixture,
        chooser: chooser.plain,
        frame: frame.plain,
        status,
    }
}

fn flattened(text: &str) -> String {
    text.split_whitespace()
        .filter(|word| *word != "┃")
        .collect::<Vec<_>>()
        .join(" ")
}

/// Every entrypoint × answer: the chooser is legible in the emulator, and the
/// answer decides ownership only.
#[test]
#[serial(level2_terminal)]
fn level2_tmux_ambiguity_chooser_decides_ownership_for_every_entrypoint() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

    let mut harness = TmuxHarness::shared_or_spawn().expect("tmux harness");
    harness.resize(100, 30).expect("resize pane");
    for entrypoint in [Entrypoint::Compose, Entrypoint::InlineCompose, Entrypoint::Sequence] {
        for choice in [Answer::Codex, Answer::Claude] {
            let case = format!("{entrypoint:?} answered {choice:?}");
            let run = answer(&mut harness, entrypoint, choice);

            let question = flattened(&run.chooser);
            for phrase in [
                "Resolving how the arguments after the composition file are read",
                "read the word after -c differently",
                QUESTION,
                "(This decides how the arguments are read, not which agent runs.)",
            ] {
                assert!(question.contains(phrase), "{case}: {phrase:?} not drawn:\n{}", run.chooser);
            }
            let choices: Vec<&str> = run
                .chooser
                .lines()
                .map(str::trim)
                .filter(|line| line.ends_with("Claude") || line.ends_with("Codex"))
                .collect();
            assert_eq!(choices.len(), 2, "{case}: one row per choice expected:\n{}", run.chooser);
            assert!(choices[0].ends_with("Claude") && choices[1].ends_with("Codex"), "{case}: {choices:?}");
            assert_ne!(choices[0], "Claude", "{case}: the default choice is not marked:\n{}", run.chooser);

            let launches = run.launches();
            match choice {
                Answer::Codex => {
                    assert_eq!(run.status, "0", "{case}: claudine failed in the pane:\n{}", run.frame);
                    assert_eq!(launches.len(), 1, "{case}: {launches:?}\n{}", run.frame);
                    let argv = &launches[0];
                    let at = argv
                        .iter()
                        .position(|arg| arg == "-c")
                        .unwrap_or_else(|| panic!("{case}: `-c` not forwarded: {argv:?}"));
                    assert_eq!(argv.get(at + 1).map(String::as_str), Some("foo"), "{case}: {argv:?}");
                }
                Answer::Claude => {
                    assert_ne!(run.status, "0", "{case}: the refused launch exited 0:\n{}", run.frame);
                    assert!(launches.is_empty(), "{case}: Codex must not be spawned: {launches:?}");
                    let frame = flattened(&run.frame);
                    assert!(frame.contains("takes a value for Codex"), "{case}:\n{}", run.frame);
                }
            }
            assert!(
                !run.fixture.audio_spool().exists(),
                "{case}: lifecycle audio was published to {}",
                run.fixture.audio_spool().display()
            );
        }
    }
}
