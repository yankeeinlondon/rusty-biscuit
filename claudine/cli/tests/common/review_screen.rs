//! The `claudine sequence` provider review screen, driven in a real terminal
//! pane.
//!
//! [`ReviewFixture`] stages a hermetic sequence whose steps interleave every
//! row-eligible executable with hidden `shell` and `side_effect` steps, puts
//! fake `claude` and `goose` executables on `PATH`, and writes a launcher the
//! pane's shell runs. The sequence document names no `agent`, so with both
//! fakes installed the review screen opens with Claude as every row's default.
//!
//! | Step | Name | Executable | Row |
//! | ---: | :--- | :--- | :--- |
//! | 1 | `stage-first` | `shell` | none |
//! | 2 | `review` | `prompt` | `2 review` |
//! | 3 | `note` | `side_effect` | none |
//! | 4 | `review` | `task` | `4 review` |
//! | 5 | `build` | `group` | `5 build` |
//! | 6 | [`LONG_STEP_NAME`] | none (the body) | `6 …` |
//! | 7 | `stage-last` | `shell` | none |
//!
//! Every place a step can do work leaves evidence in the fixture: a fake
//! provider records each launch (its name, argv, stdin, and `MODEL`) in its
//! own directory, and the hidden steps append to `events.log`. Each composed
//! prompt carries a token naming its step, so a recorded launch identifies the
//! row whose choice it ran under.
//!
//! The launcher runs `claudine` under `env -i` with the same hermetic
//! variables `CliProcessFixture` gives an L1 child — fixture `HOME` and `PATH`,
//! no Rendezvous report, `PLAYA_DRY_RUN=1` and a private spool — because the
//! pane's shell, not this process, owns the child's environment. It prints
//! `<marker>:<status>` when `claudine` exits.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use biscuit_test_harness::TerminalHarness;

use super::{
    CliProcessFixture, claudine_bin, minimal_system_path, sh_quote, wait_for_exit_marker,
    wait_for_pane_text, write, write_executable,
};

/// The hint the review screen draws on its last row while it is open.
pub const SCREEN_HINT: &str = "Ctrl+S=Submit";

/// Step 6's name: long enough to be clipped on a narrow pane.
pub const LONG_STEP_NAME: &str = "summarize-everything-that-happened-in-this-sequence-run";

/// The label of every row, in screen order, as the full label reads.
pub const ROW_LABELS: [&str; 4] = [
    "2 review",
    "4 review",
    "5 build",
    "6 summarize-everything-that-happened-in-this-sequence-run",
];

/// The token each eligible step's composed prompt carries, in row order.
pub const STEP_TOKENS: [&str; 4] = ["STEP2-REVIEW", "STEP4-TASK", "STEP5-GROUP", "STEP6-BODY"];

/// What the hidden steps write to `events.log`, in step order.
pub const HIDDEN_STEP_EVENTS: [&str; 3] = ["shell-first", "side-effect-ran", "shell-last"];

/// The models a [`ModelEditor::CatalogChoice`] fixture offers for Claude.
pub const CLAUDE_MODELS: [&str; 2] = ["fake-claude-a", "fake-claude-b"];

/// Bound on the review screen drawing after launch.
pub const SCREEN_TIMEOUT: Duration = Duration::from_secs(30);

/// Bound on the sequence finishing, or exiting, after the screen closes.
pub const EXIT_TIMEOUT: Duration = Duration::from_secs(60);

/// Which form the shared model column takes.
///
/// Every sequence draft shares the document-level provider plan, so the model
/// column is a catalog `ChooseOne` whenever the default provider has a
/// catalog; the free-text form appears only when that catalog is empty. The
/// user config's `replace` override pins either shape.
#[derive(Clone, Copy, Debug)]
pub enum ModelEditor {
    /// Claude's catalog is exactly [`CLAUDE_MODELS`].
    CatalogChoice,
    /// Claude's catalog is empty, so the column is a text input.
    FreeText,
}

/// Records one launch into a fresh directory under the record dir.
fn recording_provider(name: &str, record_dir: &Path, tail: &str) -> String {
    format!(
        "#!/bin/sh\n\
         d=$(/usr/bin/mktemp -d {record}/launch.XXXXXX)\n\
         printf '%s' {name} > \"$d/provider\"\n\
         if [ -n \"${{MODEL+x}}\" ]; then printf '%s' \"$MODEL\" > \"$d/model_env\"; fi\n\
         for arg in \"$@\"; do printf '%s\\n' \"$arg\" >> \"$d/argv\"; done\n\
         if [ ! -t 0 ]; then /bin/cat > \"$d/stdin\"; fi\n\
         {tail}",
        record = sh_quote(&record_dir.display().to_string()),
    )
}

/// One provider launch a fake recorded.
#[derive(Debug)]
pub struct Launch {
    /// The fake's binary name (`claude` or `goose`).
    pub provider: String,
    /// The value of `--model` on argv, if any.
    pub model_arg: Option<String>,
    /// The child's `MODEL`, if set.
    pub model_env: Option<String>,
    /// Argv and stdin joined, for token lookup.
    text: String,
}

impl Launch {
    /// The step token the composed prompt carried.
    pub fn step_token(&self) -> Option<&'static str> {
        STEP_TOKENS
            .into_iter()
            .find(|token| self.text.contains(token))
    }
}

/// A staged review-screen sequence and the launcher that runs it.
pub struct ReviewFixture {
    pub fixture: CliProcessFixture,
    record_dir: PathBuf,
    launcher: PathBuf,
    marker: String,
}

impl ReviewFixture {
    /// Stage the sequence, the fakes, the user config, and the launcher.
    pub fn stage(name: &str, editor: ModelEditor) -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let fixture = CliProcessFixture::named(name);
        let cwd = fixture.cwd().to_path_buf();
        let record_dir = fixture.workspace_path().join("record");
        let tmp_dir = fixture.workspace_path().join("tmp");
        for dir in [&record_dir, &tmp_dir] {
            fs::create_dir_all(dir).unwrap();
        }

        // A user config must exist, or an interactive session runs the
        // first-run wizard, which would take the keys meant for the screen.
        let claude_models = match editor {
            ModelEditor::CatalogChoice => format!("\"{}\", \"{}\"", CLAUDE_MODELS[0], CLAUDE_MODELS[1]),
            ModelEditor::FreeText => String::new(),
        };
        write(
            &fixture.home().join(".claudine/config.json"),
            &format!(
                "{{\"models\": {{\"claude\": {{\"mode\": \"replace\", \"values\": [{claude_models}]}}}}}}\n"
            ),
        );

        write_executable(
            &fixture.bin_dir().join("claude"),
            &recording_provider(
                "claude",
                &record_dir,
                "printf '%s\\n' '{\"type\":\"system\",\"subtype\":\"init\",\"session_id\":\"stub\",\"model\":\"stub-model\"}'\n\
                 printf '%s\\n' '{\"type\":\"result\",\"subtype\":\"success\",\"is_error\":false,\"result\":\"done\"}'\n\
                 exit 0\n",
            ),
        );
        write_executable(
            &fixture.bin_dir().join("goose"),
            &recording_provider("goose", &record_dir, "printf 'goose-said\\n'\nexit 0\n"),
        );

        write(&cwd.join("review.md"), "Review token STEP2-REVIEW.\n");
        write(&cwd.join("task-review.md"), "Task token STEP4-TASK.\n");
        write(&cwd.join("task.yaml"), "kind: task\nprompt: \"./task-review.md\"\n");
        write(&cwd.join("member.md"), "Group member token STEP5-GROUP.\n");
        write(
            &cwd.join("seq.md"),
            &format!(
                "---\n\
                 sequence:\n\
                 \x20   - name: stage-first\n\
                 \x20     shell: echo shell-first >> events.log\n\
                 \x20   - name: review\n\
                 \x20     prompt: \"./review.md\"\n\
                 \x20   - name: note\n\
                 \x20     side_effect: {{ append_line: [\"events.log\", \"side-effect-ran\"] }}\n\
                 \x20   - name: review\n\
                 \x20     task: ./task.yaml\n\
                 \x20   - name: build\n\
                 \x20     group:\n\
                 \x20         name: build-group\n\
                 \x20         execution: serial\n\
                 \x20         tasks:\n\
                 \x20             - prompt: \"./member.md\"\n\
                 \x20   - name: {LONG_STEP_NAME}\n\
                 \x20   - name: stage-last\n\
                 \x20     shell: echo shell-last >> events.log\n\
                 ---\n\
                 Body token STEP6-BODY.\n"
            ),
        );

        let marker = format!(
            "REVIEW_SCREEN_{}_{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        );
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
            ("LC_ALL", "C.UTF-8".to_string()),
            ("NO_COLOR", "1".to_string()),
            ("CLAUDINE_RENDEZVOUS_REPORT", "false".to_string()),
            (
                "RENDEZVOUS_ENDPOINT",
                sh_quote(&fixture.rendezvous_endpoint().to_string_lossy()),
            ),
            ("PLAYA_DRY_RUN", "1".to_string()),
            ("PLAYA_SPOOL_DIR", quoted(&fixture.audio_spool())),
        ]
        .map(|(key, value)| format!("{key}={value}"))
        .join(" ");
        // `--yolo` approves the two shell steps, which would otherwise prompt
        // before the review screen opens. It selects no provider, so the
        // screen still opens.
        let script = format!(
            "#!/bin/sh\nprintf '\\033[2J\\033[H'\ncd {cwd} || exit 1\n\
             /usr/bin/env -i {variables} {claudine} sequence --yolo seq.md\n\
             printf '\\n{marker}:%s\\n' \"$?\"\n",
            cwd = quoted(&cwd),
            claudine = sh_quote(claudine_bin()),
        );
        let launcher = fixture.workspace_path().join("launch.sh");
        write_executable(&launcher, &script);

        Self {
            fixture,
            record_dir,
            launcher,
            marker,
        }
    }

    /// Type the launcher into `harness`'s shell and wait for the screen.
    pub fn open<H: TerminalHarness>(&self, harness: &mut H) -> biscuit_test_harness::CapturedFrame {
        harness
            .send_text(
                format!("/bin/sh {}\n", sh_quote(&self.launcher.display().to_string())).as_bytes(),
            )
            .expect("send launcher");
        wait_for_pane_text(harness, SCREEN_HINT, SCREEN_TIMEOUT)
    }

    /// Wait for `claudine` to exit and return the final frame's text and the
    /// exit status.
    pub fn wait_for_exit<H: TerminalHarness>(&self, harness: &mut H) -> (String, String) {
        let (frame, status) = wait_for_exit_marker(harness, &self.marker, EXIT_TIMEOUT);
        (frame.plain, status)
    }

    /// Every provider launch recorded so far, in no particular order.
    pub fn launches(&self) -> Vec<Launch> {
        let mut launches = Vec::new();
        for entry in fs::read_dir(&self.record_dir).expect("read record dir") {
            let dir = entry.expect("record entry").path();
            let read = |name: &str| fs::read_to_string(dir.join(name)).ok();
            let argv = read("argv").unwrap_or_default();
            let model_arg = argv
                .lines()
                .skip_while(|line| *line != "--model")
                .nth(1)
                .map(str::to_string);
            launches.push(Launch {
                provider: read("provider").expect("recorded provider"),
                model_arg,
                model_env: read("model_env"),
                text: format!("{argv}\n{}", read("stdin").unwrap_or_default()),
            });
        }
        launches
    }

    /// `(step token, provider, MODEL)` for every launch, sorted by token.
    ///
    /// ## Panics
    ///
    /// When a launch carries no step token, or its `--model` argument and
    /// `MODEL` disagree.
    pub fn targets_by_step(&self) -> Vec<(&'static str, String, Option<String>)> {
        let mut targets: Vec<(&'static str, String, Option<String>)> = self
            .launches()
            .into_iter()
            .map(|launch| {
                let token = launch
                    .step_token()
                    .unwrap_or_else(|| panic!("a launch carried no step token: {launch:?}"));
                if let Some(argument) = &launch.model_arg {
                    assert_eq!(
                        Some(argument),
                        launch.model_env.as_ref(),
                        "`--model` and `MODEL` must name one model: {launch:?}"
                    );
                }
                (token, launch.provider, launch.model_env)
            })
            .collect();
        targets.sort();
        targets
    }

    /// The lines the hidden steps wrote, in order; empty when none ran.
    pub fn events(&self) -> Vec<String> {
        fs::read_to_string(self.fixture.cwd().join("events.log"))
            .map(|text| text.lines().map(str::to_string).collect())
            .unwrap_or_default()
    }

    /// Assert that nothing ran: no provider launched, no hidden step wrote,
    /// and no lifecycle audio was published.
    pub fn assert_no_work(&self, frame: &str) {
        let launches = self.launches();
        assert!(
            launches.is_empty(),
            "no provider may launch after the review screen is cancelled; launched: {launches:?}\n{frame}"
        );
        assert_eq!(
            self.events(),
            Vec::<String>::new(),
            "no shell or side-effect step may run after the review screen is cancelled\n{frame}"
        );
        self.assert_no_audio();
    }

    /// The `PLAYA_DRY_RUN` guard held: nothing was published to the spool.
    pub fn assert_no_audio(&self) {
        assert!(
            !self.fixture.audio_spool().exists(),
            "lifecycle audio was published to {}",
            self.fixture.audio_spool().display()
        );
    }
}

/// The three screen rows a table row occupies, starting at the line whose
/// label is `label`.
///
/// ## Panics
///
/// When no line starts with `label`.
pub fn row_block<'a>(plain: &'a str, label: &str) -> Vec<&'a str> {
    let lines: Vec<&str> = plain.lines().collect();
    let start = lines
        .iter()
        .position(|line| line.trim_start().starts_with(label))
        .unwrap_or_else(|| panic!("no row labeled {label:?} is on screen:\n{plain}"));
    lines[start..(start + 3).min(lines.len())].to_vec()
}

/// Whether `block` shows `option` as the selected choice (`●`) in its cell.
pub fn shows_selected(block: &[&str], option: &str) -> bool {
    block
        .iter()
        .any(|line| line.contains(&format!("● {option}")))
}
