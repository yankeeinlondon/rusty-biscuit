//! Level 2: `--edit` composed with `--interactive` from a real terminal.
//!
//! L1 can only reach `--edit` through piped stdio, where it stops at the
//! editor's terminal precondition. A user runs `claudine codex --edit -i` at a
//! terminal: the editor inherits the TTY, and after it exits the provider
//! inherits the same TTY as an interactive session whose first turn is the
//! edited text. This binary runs that launch in a detached tmux session
//! against a fake editor and fake providers, then reads back what each
//! observed:
//!
//! - the editor's starting buffer (empty, or the command-line seed);
//! - the provider's exact argv, whether its stdin and stdout were TTYs, and
//!   anything it read from a non-TTY stdin;
//! - how many times the provider was launched.
//!
//! The spec is `2026-09-18-edit-integration` (AC1–AC4, AC6, AC8, AC9).
//!
//! ## Hermetic launch
//!
//! The pane's shell belongs to the harness, so the launcher starts `claudine`
//! under `env -i` with only the fixture's variables, as
//! `level2_provider_overlay_capture.rs` does. `EDITOR` names the fake editor
//! by bare name (it sits in the fixture `bin`), because Claudine splits the
//! editor command on whitespace. `TMPDIR` keeps the editor buffer inside the
//! fixture. No window is opened or focused.
//!
//! ## Windows
//!
//! Unix only. The change under test deletes two validation checks and moves
//! one earlier; native Windows is covered by the L1 non-TTY tests in
//! `wrap_basics.rs` and the profile unit tests (plan ruling N9).
//!
//! Run via the canonical recipe: `just test-l2 edit_interactive_capture`.

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
    write, write_executable,
};

/// Drawn by the fake provider on the terminal it inherited.
const PROVIDER_BANNER: &str = "fake provider session attached";

/// Records every launch into `$CLAUDINE_RECORD_DIR`: one file per argument
/// (so a multiline argument survives intact), the argument count, both TTY
/// checks, stdin when it is not a TTY, and one line per invocation.
const RECORD_PROVIDER: &str = r#"#!/bin/sh
dir="$CLAUDINE_RECORD_DIR"
echo launch >> "$dir/invocations"
if [ -t 1 ]; then echo yes > "$dir/stdout_tty"; else echo no > "$dir/stdout_tty"; fi
if [ -t 0 ]; then
  echo yes > "$dir/stdin_tty"
else
  echo no > "$dir/stdin_tty"
  /bin/cat > "$dir/stdin"
fi
i=0
for arg in "$@"; do
  printf '%s' "$arg" > "$dir/arg.$i"
  i=$((i + 1))
done
echo "$i" > "$dir/argc"
echo 'fake provider session attached'
exit 0
"#;

/// Copies the buffer it was given to `seed.md`, then acts on `mode`: `write`
/// replaces the buffer with `prompt.md`, `empty` truncates it, `fail` exits 3.
const FAKE_EDITOR: &str = r#"#!/bin/sh
dir="$CLAUDINE_EDITOR_DIR"
/bin/cat "$1" > "$dir/seed.md"
case "$(/bin/cat "$dir/mode")" in
  write) /bin/cat "$dir/prompt.md" > "$1" ;;
  empty) : > "$1" ;;
  fail) exit 3 ;;
esac
"#;

const PLAIN_PROMPT: &str = "review the edited plan";
const BULLET_PROMPT: &str = "- first edited item\n- second edited item\n\nclosing line";

#[derive(Clone, Copy)]
enum EditorMode {
    Write(&'static str),
    Empty,
    Fail,
}

/// One terminal launch and everything it left behind.
struct Launch {
    fixture: CliProcessFixture,
    frame: String,
    status: String,
}

impl Launch {
    fn record_dir(&self) -> PathBuf {
        self.fixture.cwd().join("record")
    }

    fn editor_dir(&self) -> PathBuf {
        self.fixture.cwd().join("editor")
    }

    fn provider_launched(&self) -> bool {
        self.record_dir().join("invocations").exists()
    }

    fn record(&self, name: &str) -> String {
        let path = self.record_dir().join(name);
        fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("{} was not recorded: {error}\n{}", path.display(), self.frame))
    }

    /// The provider's argv, asserting it was launched exactly once.
    fn provider_args(&self) -> Vec<String> {
        assert_eq!(
            self.record("invocations").lines().count(),
            1,
            "the provider must be launched exactly once:\n{}",
            self.frame
        );
        let argc: usize = self.record("argc").trim().parse().expect("argc");
        (0..argc).map(|index| self.record(&format!("arg.{index}"))).collect()
    }

    fn editor_seed(&self) -> String {
        let path = self.editor_dir().join("seed.md");
        fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("the editor never ran ({error}):\n{}", self.frame))
    }

    fn assert_interactive_tty(&self) {
        assert_eq!(self.record("stdin_tty").trim(), "yes", "stdin was not the terminal:\n{}", self.frame);
        assert_eq!(self.record("stdout_tty").trim(), "yes", "stdout was not the terminal:\n{}", self.frame);
        assert!(
            !self.record_dir().join("stdin").exists(),
            "an interactive provider must not be fed a prompt on stdin"
        );
        assert!(
            self.frame.contains(PROVIDER_BANNER),
            "the provider's output never reached the terminal:\n{}",
            self.frame
        );
    }
}

fn unique_marker() -> String {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    format!("EDIT_I_{}_{}", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed))
}

/// Writes the fixtures and the launcher the pane runs, returning its path.
///
/// `provider` is the stub's binary name, or `None` to leave every provider off
/// `PATH`. The exit marker is printed by the script rather than typed with
/// `$?`: on a host that loads Atuin AI, a typed `?` opens its overlay and
/// wedges the pane.
fn write_launcher(
    fixture: &CliProcessFixture,
    provider: Option<&str>,
    mode: EditorMode,
    args: &[&str],
    marker: &str,
) -> PathBuf {
    let record_dir = fixture.cwd().join("record");
    let editor_dir = fixture.cwd().join("editor");
    let tmp_dir = fixture.workspace_path().join("tmp");
    for dir in [&record_dir, &editor_dir, &tmp_dir] {
        fs::create_dir_all(dir).unwrap();
    }
    let mode_name = match mode {
        EditorMode::Write(prompt) => {
            write(&editor_dir.join("prompt.md"), prompt);
            "write"
        }
        EditorMode::Empty => "empty",
        EditorMode::Fail => "fail",
    };
    write(&editor_dir.join("mode"), mode_name);
    write_executable(&fixture.bin_dir().join("fake-editor"), FAKE_EDITOR);
    if let Some(provider) = provider {
        write_executable(&fixture.bin_dir().join(provider), RECORD_PROVIDER);
    }

    let path = std::env::join_paths(
        std::iter::once(fixture.bin_dir().to_path_buf()).chain(minimal_system_path()),
    )
    .expect("join fixture PATH");
    let quoted = |value: &Path| sh_quote(&value.display().to_string());
    let variables = [
        ("HOME", quoted(fixture.home())),
        ("PATH", quoted(Path::new(&path))),
        ("TMPDIR", quoted(&tmp_dir)),
        ("TERM", "xterm-256color".to_string()),
        ("NO_COLOR", "1".to_string()),
        ("EDITOR", "fake-editor".to_string()),
        ("CLAUDINE_RENDEZVOUS_REPORT", "false".to_string()),
        ("PLAYA_DRY_RUN", "1".to_string()),
        ("PLAYA_SPOOL_DIR", quoted(&fixture.audio_spool())),
        ("CLAUDINE_RECORD_DIR", quoted(&record_dir)),
        ("CLAUDINE_EDITOR_DIR", quoted(&editor_dir)),
    ]
    .map(|(name, value)| format!("{name}={value}"))
    .join(" ");
    let argv = args.iter().map(|arg| sh_quote(arg)).collect::<Vec<_>>().join(" ");
    let script = format!(
        "#!/bin/sh\nprintf '\\033[2J\\033[H'\ncd {cwd} || exit 1\n\
         if /usr/bin/env -i {variables} {claudine} {argv}; then\n  \
         printf '\\n{marker}:0\\n'\nelse\n  printf '\\n{marker}:1\\n'\nfi\n",
        cwd = quoted(fixture.cwd()),
        claudine = sh_quote(claudine_bin()),
    );
    let launcher = fixture.workspace_path().join("launch.sh");
    write_executable(&launcher, &script);
    launcher
}

/// Runs `claudine <args>` in the terminal with its own fixture and waits for
/// it to exit.
fn launch(name: &str, provider: Option<&str>, mode: EditorMode, args: &[&str]) -> Launch {
    let fixture = CliProcessFixture::named(name);
    fixture.seed_user_config();
    let marker = unique_marker();
    let launcher = write_launcher(&fixture, provider, mode, args, &marker);

    let mut harness = TmuxHarness::shared_or_spawn().expect("tmux harness");
    harness
        .send_text(format!("/bin/sh {}\n", sh_quote(&launcher.display().to_string())).as_bytes())
        .expect("send launcher");
    let (frame, status) = wait_for_exit_marker(&mut harness, &marker, Duration::from_secs(30));
    Launch {
        fixture,
        frame: frame.plain,
        status,
    }
}

/// AC1, AC2: every spelling of `--edit` with interactive mode, with and
/// without a seed on either side of the positional, launches Codex
/// interactively with the edited text as its first turn.
#[test]
#[serial(level2_terminal)]
fn level2_tmux_edit_interactive_delivers_the_edited_prompt_as_the_first_turn() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

    let cases: [(&str, &[&str], &str); 3] = [
        ("no seed, --interactive", &["codex", "--edit", "--interactive"], ""),
        ("seed after the flags", &["codex", "--edit", "-i", "seed from argv"], "seed from argv"),
        ("seed before the flags", &["codex", "seed from argv", "--edit", "-i"], "seed from argv"),
    ];
    for (label, args, seed) in cases {
        let run = launch("l2-edit-i", Some("codex"), EditorMode::Write(PLAIN_PROMPT), args);
        assert_eq!(run.status, "0", "{label}: claudine failed in the pane:\n{}", run.frame);
        assert_eq!(run.editor_seed(), seed, "{label}: the editor did not start from the seed");
        run.assert_interactive_tty();

        let argv = run.provider_args();
        assert_eq!(
            argv.first().map(String::as_str),
            Some(PLAIN_PROMPT),
            "{label}: interactive Codex takes the edited prompt as its leading positional: {argv:?}"
        );
        for leaked in ["exec", "--edit", "-i", "--interactive", "seed from argv"] {
            assert!(!argv.iter().any(|arg| arg == leaked), "{label}: {leaked:?} reached Codex: {argv:?}");
        }
        assert!(
            !run.frame.contains("cannot be used with"),
            "{label}: a flag conflict was reported:\n{}",
            run.frame
        );
    }
}

/// AC8: a multiline Markdown prompt that opens with `- ` reaches interactive
/// Codex whole, after the `--` end-of-options separator.
#[test]
#[serial(level2_terminal)]
fn level2_tmux_edit_interactive_delivers_a_bullet_prompt_after_end_of_options() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

    let run = launch("l2-edit-i-bullet", Some("codex"), EditorMode::Write(BULLET_PROMPT), &["codex", "--edit", "-i"]);
    assert_eq!(run.status, "0", "claudine failed in the pane:\n{}", run.frame);
    run.assert_interactive_tty();

    let argv = run.provider_args();
    assert_eq!(
        argv[argv.len().saturating_sub(2)..],
        ["--".to_string(), BULLET_PROMPT.to_string()],
        "the bullet prompt must be one argument after `--`: {argv:?}"
    );
    assert!(!argv.iter().any(|arg| arg == "exec"), "{argv:?}");
}

/// AC3: an empty buffer in interactive mode exits cleanly without a launch.
///
/// `prompt empty; aborted` is an info-level message: without `RUST_LOG` or
/// `--debug info` the abort is silent.
#[test]
#[serial(level2_terminal)]
fn level2_tmux_edit_interactive_empty_buffer_aborts_without_launching() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

    let run = launch(
        "l2-edit-i-empty",
        Some("codex"),
        EditorMode::Empty,
        &["--debug", "info", "codex", "--edit", "-i", "seed"],
    );
    assert_eq!(run.status, "0", "an empty buffer must exit successfully:\n{}", run.frame);
    assert_eq!(run.editor_seed(), "seed");
    assert!(run.frame.contains("opening fake-editor for prompt..."), "{}", run.frame);
    assert!(run.frame.contains("prompt empty; aborted"), "{}", run.frame);
    assert!(!run.provider_launched(), "the provider was launched:\n{}", run.frame);
    assert!(!run.frame.contains(PROVIDER_BANNER), "{}", run.frame);
}

/// AC4: an editor that exits non-zero keeps its typed diagnostic and
/// launches nothing.
#[test]
#[serial(level2_terminal)]
fn level2_tmux_edit_interactive_editor_failure_launches_nothing() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

    let run = launch("l2-edit-i-fail", Some("codex"), EditorMode::Fail, &["codex", "--edit", "--interactive"]);
    assert_eq!(run.status, "1", "an editor failure must fail the command:\n{}", run.frame);
    for field in ["EditorError: editor exited with error", "Editor: fake-editor", "Exit code: 3"] {
        assert!(
            run.frame.contains(field),
            "the typed editor diagnostic lacks {field:?}:\n{}",
            run.frame
        );
    }
    assert!(!run.provider_launched(), "the provider was launched:\n{}", run.frame);
}

/// AC6: `--dry-run --edit -i` runs the editor and previews an interactive
/// launch of the edited prompt with no provider binary installed.
#[test]
#[serial(level2_terminal)]
fn level2_tmux_edit_interactive_dry_run_previews_without_a_provider() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

    let run = launch(
        "l2-edit-i-dry",
        None,
        EditorMode::Write(PLAIN_PROMPT),
        &["codex", "--dry-run", "--edit", "-i"],
    );
    assert_eq!(run.status, "0", "the dry run failed:\n{}", run.frame);
    assert_eq!(run.editor_seed(), "", "the editor must run in a dry run");
    assert!(run.frame.contains("[DRY RUN]"), "{}", run.frame);
    let header = run
        .frame
        .lines()
        .find(|line| line.contains("Codex") && line.contains(PLAIN_PROMPT))
        .unwrap_or_else(|| panic!("no header shows the edited prompt:\n{}", run.frame));
    assert!(header.contains("Interactive"), "the header must report interactive mode: {header}");
    assert!(!run.provider_launched());
}

/// AC9 (first half): plain `--edit` still launches a non-empty edit
/// non-interactively, with the prompt on stdin.
#[test]
#[serial(level2_terminal)]
fn level2_tmux_edit_without_interactive_stays_non_interactive() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

    let run = launch("l2-edit-plain", Some("codex"), EditorMode::Write(PLAIN_PROMPT), &["codex", "--edit"]);
    assert_eq!(run.status, "0", "claudine failed in the pane:\n{}", run.frame);
    let argv = run.provider_args();
    assert_eq!(argv.first().map(String::as_str), Some("exec"), "{argv:?}");
    assert!(!argv.iter().any(|arg| arg == PLAIN_PROMPT), "{argv:?}");
    assert_eq!(run.record("stdin_tty").trim(), "no");
    assert_eq!(run.record("stdin"), PLAIN_PROMPT);
}

/// A repaired profile through the real pipeline: Pi receives the edited
/// bullet prompt as a positional message after `--`, with the terminal still
/// on stdin (piped stdin would force Pi into print mode).
#[test]
#[serial(level2_terminal)]
fn level2_tmux_pi_edit_interactive_passes_the_prompt_on_argv_and_keeps_the_terminal() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

    let run = launch("l2-edit-i-pi", Some("pi"), EditorMode::Write(BULLET_PROMPT), &["pi", "--edit", "-i"]);
    assert_eq!(run.status, "0", "claudine failed in the pane:\n{}", run.frame);
    run.assert_interactive_tty();

    let argv = run.provider_args();
    assert_eq!(
        argv[argv.len().saturating_sub(2)..],
        ["--".to_string(), BULLET_PROMPT.to_string()],
        "Pi must take the edited prompt as a message after `--`: {argv:?}"
    );
    for print_mode in ["-p", "--print", "--mode"] {
        assert!(!argv.iter().any(|arg| arg == print_mode), "interactive Pi got {print_mode}: {argv:?}");
    }
}
