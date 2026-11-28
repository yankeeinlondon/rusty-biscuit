//! The lifecycle `edit` action through `claudine compose`, `claudine
//! sequence`, and the wrapper's `--edit`, with the `claudine-fake-editor`
//! fixture installed under real editors' names, and the refusal of
//! `--output` before any of it can run.
//!
//! An L1 spawn has piped standard streams, so a terminal editor (`vim`) is
//! always refused here; a GUI editor (`code`, launched with `--wait`) opens
//! every file these tests expect to be opened, with `DISPLAY` set so Linux
//! admits it. The provider is the compiled `claudine-fake-goose` in marker
//! mode, so "the provider never launched" is its marker file's absence, and
//! "the editor never ran" is the editor's argv record's absence.
//!
//! ## Not covered
//!
//! No Claudine timeout bounds a lifecycle stack: a document's `timeout:` and
//! `--timeout` bound the provider session, and a task's `timeout:` bounds its
//! shell commands, so a held editor outlives all of them. Cancellation is
//! proved here with an interrupt and with a fail-fast sibling instead.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{Duration, Instant};

use test_toolkit::argv_recorder::read_argv_records;
use test_toolkit::fake_editor::{FAKE_EDITOR_RECORD, FAKE_EDITOR_WRITE};

use crate::common::{CliProcessFixture, strip_ansi};

/// The motivating workflow: warn about an existing spec, otherwise create and
/// open it, then end the run before any provider launches.
const MOTIVATING: &str = r#"---
fqn: "{{ ctx.cwd }}/specs/2026-11-14-example/spec.md"
initialize:
  - when: file_exists(fqn)
    then:
      - warn: "The spec already exists: {{ fqn }}"
      - skip
    else:
      - edit:
          file: "{{ fqn }}"
          create_new: true
      - info: "The spec is ready at {{ fqn }}"
      - skip
---
# Never composed
"#;

/// A fixture with the fake provider and the fake editor installed, plus the
/// files their runs report into.
struct EditRun {
    fixture: CliProcessFixture,
    editor: PathBuf,
    record: PathBuf,
    provider_marker: PathBuf,
    env: Vec<(&'static str, String)>,
}

impl EditRun {
    /// `editor` is the name the fake editor is installed under: `code` is a
    /// GUI editor, `vim` a terminal one, `lapce` a GUI editor that cannot
    /// wait.
    fn new(prefix: &str, editor: &str) -> Self {
        let fixture = CliProcessFixture::named(prefix);
        fs::copy(
            biscuit_test_harness::bin_exe!("claudine-fake-goose"),
            fixture.bin_dir().join(format!("goose{}", std::env::consts::EXE_SUFFIX)),
        )
        .expect("the fake provider is built with `test-fixtures`");
        // Not the fixture `bin`, which is on `PATH`: an installed `code`
        // must not be found by anything but `EDITOR`. No space in the path,
        // because `$EDITOR` is split on whitespace.
        let editors = fixture.workspace_path().join("editors");
        fs::create_dir_all(&editors).unwrap();
        let editor_path = editors.join(format!("{editor}{}", std::env::consts::EXE_SUFFIX));
        fs::copy(biscuit_test_harness::bin_exe!("claudine-fake-editor"), &editor_path)
            .expect("the fake editor is built with `test-fixtures`");
        let record = fixture.workspace_path().join("editor-record");
        let provider_marker = fixture.workspace_path().join("provider-launches");
        Self { fixture, editor: editor_path, record, provider_marker, env: Vec::new() }
    }

    fn with(mut self, key: &'static str, value: impl Into<String>) -> Self {
        self.env.push((key, value.into()));
        self
    }

    /// The editor writes `bytes` into its target before anything else it
    /// was told to do (hold, exit).
    fn writing(self, bytes: &str) -> Self {
        let content = self.fixture.workspace_path().join("editor-content");
        fs::write(&content, bytes).unwrap();
        self.with(FAKE_EDITOR_WRITE, content.to_string_lossy())
    }

    fn cwd(&self) -> &Path {
        self.fixture.cwd()
    }

    fn write_doc(&self, name: &str, content: &str) -> PathBuf {
        let path = self.cwd().join(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&path, content).unwrap();
        path
    }

    /// A `claudine` command carrying the editor environment, with stderr wide
    /// enough that a failure line naming temporary paths is never shortened.
    fn command(&self, args: &[&str]) -> Command {
        let mut command = self.fixture.command_std();
        command
            .args(args)
            .env("COLUMNS", "4000")
            // The sequence task header glyph needs a UTF-8 locale.
            .env("LC_ALL", "C.UTF-8")
            .env("EDITOR", &self.editor)
            .env_remove("VISUAL")
            // A display session, so Linux admits a GUI editor; macOS and
            // Windows always have one.
            .env("DISPLAY", ":0")
            .env_remove("WAYLAND_DISPLAY")
            .env(FAKE_EDITOR_RECORD, &self.record)
            .env("PROVIDER_MARKER", &self.provider_marker);
        for (key, value) in &self.env {
            command.env(key, value);
        }
        command
    }

    fn run(&self, args: &[&str]) -> Output {
        self.command(args).output().expect("claudine runs")
    }

    fn compose(&self, document: &Path, extra: &[&str]) -> Output {
        let mut args = vec!["compose", "--goose"];
        args.extend_from_slice(extra);
        let document = document.to_str().unwrap();
        args.push(document);
        self.run(&args)
    }

    fn sequence(&self, document: &Path) -> Output {
        self.run(&["sequence", "--goose", "--yolo", document.to_str().unwrap()])
    }

    /// Each editor launch's arguments, in order; empty when it never ran.
    fn launches(&self) -> Vec<Vec<String>> {
        match read_argv_records(&self.record) {
            Ok(records) => records,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Vec::new(),
            Err(error) => panic!("read the editor record: {error}"),
        }
    }

    /// The file name of each launch's target (its last argument), in order.
    fn launched_names(&self) -> Vec<String> {
        self.launches()
            .iter()
            .map(|launch| {
                Path::new(launch.last().expect("a target argument"))
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect()
    }

    fn provider_launches(&self) -> usize {
        fs::read_to_string(&self.provider_marker).map_or(0, |log| log.lines().count())
    }
}

fn stdout(output: &Output) -> String {
    strip_ansi(&String::from_utf8_lossy(&output.stdout))
}

fn stderr(output: &Output) -> String {
    strip_ansi(&String::from_utf8_lossy(&output.stderr))
}

fn canonical(path: &Path) -> PathBuf {
    biscuit_file::canonicalize_simplified(path).expect("the path exists")
}

/// The `event:<name>` markers an `info` stream printed, in order.
fn markers(text: &str) -> Vec<String> {
    text.lines()
        .filter_map(|line| line.split("event:").nth(1))
        .map(|rest| rest.split_whitespace().next().unwrap_or_default().to_string())
        .collect()
}

#[cfg(unix)]
fn wait_for_file(path: &Path) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !path.exists() {
        assert!(Instant::now() < deadline, "{} never appeared", path.display());
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[cfg(unix)]
fn wait_for_pids(path: &Path, count: usize) -> Vec<u32> {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Ok(pids) = test_toolkit::fake_editor::read_pids(path)
            && pids.len() >= count
        {
            return pids;
        }
        assert!(Instant::now() < deadline, "{} never listed {count} PID(s)", path.display());
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[cfg(unix)]
fn process_alive(pid: u32) -> bool {
    // SAFETY: signal 0 only checks that the PID exists.
    unsafe { libc::kill(pid as libc::pid_t, 0) == 0 }
}

/// Polls until `pid` is gone; an orphan may take a moment to be reaped.
#[cfg(unix)]
fn wait_until_dead(pid: u32) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while process_alive(pid) {
        assert!(Instant::now() < deadline, "process {pid} survived the cancellation");
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// Kills the fake editor's spawned children if a test fails before the
/// cancellation reaped them, so a failure leaks nothing.
#[cfg(unix)]
struct PidCleanup(PathBuf);

#[cfg(unix)]
impl Drop for PidCleanup {
    fn drop(&mut self) {
        for pid in test_toolkit::fake_editor::read_pids(&self.0).unwrap_or_default() {
            // SAFETY: plain `kill(2)` on a PID the fake editor recorded.
            unsafe {
                libc::kill(pid as libc::pid_t, libc::SIGKILL);
            }
        }
    }
}

// Item 1: the motivating workflow through `claudine compose`.

#[test]
fn compose_motivating_workflow_creates_and_opens_the_spec_then_skips_the_provider() {
    let run = EditRun::new("edit-compose-motivating", "code").writing("# Spec\r\n\ttrailing  \n");
    let document = run.write_doc("new-spec.md", MOTIVATING);
    let target = run.cwd().join("specs/2026-11-14-example/spec.md");

    let output = run.compose(&document, &[]);

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let launches = run.launches();
    assert_eq!(launches.len(), 1, "{launches:?}");
    assert_eq!(launches[0].first().map(String::as_str), Some("--wait"), "the GUI editor waits: {launches:?}");
    assert_eq!(
        PathBuf::from(launches[0].last().unwrap()),
        canonical(&target),
        "the editor receives the absolute target"
    );
    assert_eq!(fs::read(&target).unwrap(), b"# Spec\r\n\ttrailing  \n", "the saved bytes are kept exactly");
    assert!(stderr(&output).contains("The spec is ready at"), "{}", stderr(&output));
    assert_eq!(run.provider_launches(), 0, "`skip` never launches the provider: {}", stderr(&output));

    let again = run.compose(&document, &[]);

    assert_eq!(again.status.code(), Some(0), "{}", stderr(&again));
    assert!(stderr(&again).contains("The spec already exists"), "{}", stderr(&again));
    assert_eq!(run.launches().len(), 1, "the warning branch never launches the editor");
    assert_eq!(fs::read(&target).unwrap(), b"# Spec\r\n\ttrailing  \n");
    assert_eq!(run.provider_launches(), 0);
}

/// A prompt kept in a library outside the launch directory edits a file in
/// the launch directory by naming it through `ctx.cwd`: a relative target
/// would be created beside the prompt, outside the mutation root.
#[test]
fn compose_edits_inside_the_launch_workspace_from_a_prompt_kept_elsewhere() {
    let run = EditRun::new("edit-compose-library-prompt", "code").writing("written");
    let library = run.fixture.workspace_path().join("library");
    fs::create_dir_all(&library).unwrap();
    let document = library.join("prompt.md");
    fs::write(&document, "---\ninitialize:\n  - edit: \"{{ ctx.cwd }}/notes/today.md\"\n  - skip\n---\nbody\n").unwrap();

    let output = run.compose(&document, &[]);

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let target = run.cwd().join("notes/today.md");
    assert_eq!(fs::read_to_string(&target).unwrap(), "written", "{}", stderr(&output));
    assert!(!library.join("notes").exists(), "nothing was created beside the prompt");
}

/// A numeric property named by a whole-value template opens the file its
/// digits spell, as the native `edit: 5` does.
#[test]
fn compose_opens_the_file_a_numeric_property_spells() {
    let run = EditRun::new("edit-compose-numeric", "code").writing("five");
    let document = run.write_doc("doc.md", "---\nnumber: 5\ninitialize:\n  - edit: \"{{ number }}\"\n  - skip\n---\nbody\n");

    let output = run.compose(&document, &[]);

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(run.launched_names(), ["5"], "{}", stderr(&output));
    assert_eq!(fs::read_to_string(run.cwd().join("5")).unwrap(), "five");
    assert_eq!(run.provider_launches(), 0);
}

// Item 2: the motivating workflow through `claudine sequence`.

#[test]
fn sequence_root_initialize_runs_the_motivating_workflow_before_any_task() {
    let run = EditRun::new("edit-sequence-motivating", "code").writing("spec bytes");
    let document = run.write_doc(
        "ship.md",
        r#"---
fqn: "{{ ctx.cwd }}/specs/spec.md"
initialize:
  - when: file_exists(fqn)
    then:
      - warn: "The spec already exists: {{ fqn }}"
      - skip
    else:
      - edit:
          file: "{{ fqn }}"
          create_new: true
      - info: "The spec is ready at {{ fqn }}"
      - skip
sequence:
  - name: work
    shell: "echo work > trace.txt"
---
"#,
    );
    let target = run.cwd().join("specs/spec.md");

    let output = run.sequence(&document);

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(run.launches().len(), 1, "{}", stderr(&output));
    assert_eq!(PathBuf::from(run.launches()[0].last().unwrap()), canonical(&target));
    assert_eq!(fs::read_to_string(&target).unwrap(), "spec bytes");
    assert!(stderr(&output).contains("The spec is ready at"), "{}", stderr(&output));
    assert!(!run.cwd().join("trace.txt").exists(), "`skip` runs no task");
    assert_eq!(run.provider_launches(), 0);

    let again = run.sequence(&document);

    assert_eq!(again.status.code(), Some(0), "{}", stderr(&again));
    assert!(stderr(&again).contains("The spec already exists"), "{}", stderr(&again));
    assert_eq!(run.launches().len(), 1, "the warning branch never launches the editor");
    assert!(!run.cwd().join("trace.txt").exists());
}

/// A prompt task whose document carries the motivating `initialize`.
#[test]
fn sequence_prompt_task_runs_the_motivating_workflow_and_launches_no_provider() {
    let run = EditRun::new("edit-sequence-prompt-task", "code").writing("spec bytes");
    run.write_doc("draft.md", MOTIVATING);
    let document = run.write_doc(
        "ship.md",
        "---\nsequence:\n  - name: draft\n    prompt: draft.md\n  - name: after\n    shell: \"echo after > trace.txt\"\n---\n",
    );

    let output = run.sequence(&document);

    let target = run.cwd().join("specs/2026-11-14-example/spec.md");
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(fs::read_to_string(&target).unwrap_or_default(), "spec bytes", "{}", stderr(&output));
    assert_eq!(run.launches().len(), 1, "{}", stderr(&output));
    assert_eq!(run.provider_launches(), 0, "{}", stderr(&output));
    assert!(run.cwd().join("trace.txt").exists(), "a skipped task lets the sequence continue: {}", stderr(&output));
}

// Item 3: `edit` runs in order with the actions around it.

/// Inside one `initialize` stack, in a taken branch and between other
/// actions, each `edit` finishes before the next action starts.
#[test]
fn initialize_runs_each_edit_in_order_with_the_actions_around_it() {
    let run = EditRun::new("edit-initialize-order", "code");
    let document = run.write_doc(
        "doc.md",
        "---\ninitialize:\n  - info: \"event:before\"\n  - edit: first.md\n  - info: \"event:between\"\n  \
         - when: file_exists(\"first.md\")\n    then:\n      - edit: second.md\n      - info: \"event:branch\"\n  \
         - info: \"event:after\"\n  - skip\n---\nbody\n",
    );

    let output = run.compose(&document, &[]);

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(run.launched_names(), ["first.md", "second.md"], "{}", stderr(&output));
    assert_eq!(markers(&stderr(&output)), ["before", "between", "branch", "after"], "{}", stderr(&output));
    assert_eq!(run.provider_launches(), 0);
}

#[test]
fn compose_runs_edit_in_order_in_start_success_and_finalize() {
    let run = EditRun::new("edit-compose-event-order", "code");
    let document = run.write_doc(
        "doc.md",
        "---\nstart:\n  - edit: start.md\n  - info: \"event:start-after\"\n\
         success:\n  - info: \"event:success-before\"\n  - edit: success.md\n  - info: \"event:success-after\"\n\
         finalize:\n  - edit: finalize.md\n  - info: \"event:finalize-after\"\n---\nbody\n",
    );

    let output = run.compose(&document, &[]);

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(run.launched_names(), ["start.md", "success.md", "finalize.md"], "{}", stderr(&output));
    assert_eq!(
        markers(&stderr(&output)),
        ["start-after", "success-before", "success-after", "finalize-after"],
        "{}",
        stderr(&output)
    );
    assert_eq!(run.provider_launches(), 1, "{}", stderr(&output));
}

/// Task `setup:`/`teardown:` are removed grammar and groups have no serial
/// mode, so the serial stacks around ordered work are an inline sequence
/// container's `start`/`finalize`. Its child observes the `start` edit's file
/// and not yet the `finalize` one.
#[test]
fn a_sequence_container_runs_edit_in_its_start_and_finalize_around_its_children() {
    let run = EditRun::new("edit-container-order", "code");
    // Shell commands run under `cmd` on Windows, which has no `test`.
    #[cfg(windows)]
    let ordered = "if exist start.md if not exist finalize.md echo ordered> trace.txt";
    #[cfg(not(windows))]
    let ordered = "test -f start.md && test ! -f finalize.md && echo ordered > trace.txt";
    let document = run.write_doc(
        "ship.md",
        &format!(
            "---\nsequence:\n  - name: ordered\n    sequence:\n      - name: child\n        \
             shell: \"{ordered}\"\n    \
             start:\n      - edit: start.md\n    finalize:\n      - edit: finalize.md\n---\n"
        ),
    );

    let output = run.sequence(&document);

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(run.launched_names(), ["start.md", "finalize.md"], "{}", stderr(&output));
    assert!(run.cwd().join("trace.txt").exists(), "the child ran between the edits: {}", stderr(&output));
}

// Item 4: `edit` is not a task side effect.

#[test]
fn a_side_effect_task_containing_edit_is_refused_without_launching_or_creating() {
    let run = EditRun::new("edit-side-effect", "code");
    let document = run.write_doc(
        "ship.md",
        "---\nsequence:\n  - name: draft\n    side_effect:\n      edit: notes.md\n  - name: after\n    \
         shell: \"echo after > trace.txt\"\n---\n",
    );

    let output = run.sequence(&document);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert!(
        stderr(&output).contains("`edit` opens an editor and is not a task side effect"),
        "{}",
        stderr(&output)
    );
    assert!(run.launches().is_empty(), "the editor never ran");
    assert!(!run.cwd().join("notes.md").exists(), "nothing was created");
    assert!(!run.cwd().join("trace.txt").exists(), "the refusal stops the sequence");
}

// Item 5: parallel group members never lend the terminal.

/// A group whose two legs are sequence containers that each run `edit` in
/// their `start`, before their one child records that it ran. Containers
/// rather than prompt tasks, because a prompt leg's failure line names the
/// prompt's absolute path and is clamped before the refusal's reason.
fn parallel_edit_group(run: &EditRun) -> PathBuf {
    let leg = |name: &str| {
        format!(
            "        - name: {name}\n          sequence:\n            - name: {name}-child\n              \
             shell: \"echo {name} > {name}-ran.txt\"\n          start:\n            - edit: {name}/notes.md\n"
        )
    };
    run.write_doc(
        "ship.md",
        &format!(
            "---\nsequence:\n  - name: fan-out\n    group:\n      name: fan-out\n      tasks:\n{}{}---\n",
            leg("one"),
            leg("two")
        ),
    )
}

#[test]
fn a_parallel_member_refuses_a_terminal_editor_before_creating_anything() {
    let run = EditRun::new("edit-parallel-terminal", "vim");
    let document = parallel_edit_group(&run);

    let output = run.sequence(&document);

    let text = stderr(&output);
    assert_eq!(output.status.code(), Some(1), "{text}");
    assert!(text.contains("`start[0].edit` could not edit the file"), "{text}");
    assert!(text.contains("a parallel group member has no interactive terminal"), "{text}");
    assert!(run.launches().is_empty(), "{text}");
    for leg in ["one", "two"] {
        assert!(!run.cwd().join(leg).exists(), "{leg}: nothing was created: {text}");
        assert!(!run.cwd().join(format!("{leg}-ran.txt")).exists(), "{leg}: the failed start stops the leg");
    }
}

#[test]
fn a_parallel_member_runs_a_gui_editor() {
    let run = EditRun::new("edit-parallel-gui", "code");
    let document = parallel_edit_group(&run);

    let output = run.sequence(&document);

    let text = stderr(&output);
    assert_eq!(output.status.code(), Some(0), "{text}");
    let mut names: Vec<_> = run.launches().iter().map(|launch| PathBuf::from(launch.last().unwrap())).collect();
    names.sort();
    let expected: Vec<_> = ["one", "two"].iter().map(|leg| canonical(&run.cwd().join(leg)).join("notes.md")).collect();
    assert_eq!(names, expected, "{text}");
    for leg in ["one", "two"] {
        assert!(run.cwd().join(format!("{leg}-ran.txt")).is_file(), "{leg}: the child ran after the edit: {text}");
    }
}

// Item 6: `--output` is refused before anything runs.

/// A document whose `initialize` would create a file and open the editor,
/// so a refusal that came too late leaves a trace on disk or in the editor
/// record.
const REFUSED_DOCUMENT: &str = "---\ninitialize:\n  - ensure_file: made.md\n  - edit: a.md\n---\nbody\n";

/// [`REFUSED_DOCUMENT`] with the `prompt` that `inline-compose` sends.
const REFUSED_INLINE_DOCUMENT: &str =
    "---\nprompt: Rewrite the body.\ninitialize:\n  - ensure_file: made.md\n  - edit: a.md\n---\nbody\n";

/// The same `initialize` on a sequence root, ahead of a task that writes a
/// file of its own.
const REFUSED_SEQUENCE: &str = "---\ninitialize:\n  - ensure_file: made.md\n  - edit: a.md\nsequence:\n  - name: work\n    shell: \"echo work > trace.txt\"\n---\n";

/// Every value is refused alike: the three the option once accepted and one
/// it never did.
const OUTPUT_VALUES: [&str; 4] = ["json", "text", "stream", "bogus"];

/// How the option and its value are spelled on the command line.
#[derive(Clone, Copy)]
enum OutputSpelling {
    Short,
    ShortAttached,
    Long,
    Attached,
}

impl OutputSpelling {
    fn words(self, value: &str) -> Vec<String> {
        match self {
            Self::Short => vec!["-o".into(), value.into()],
            Self::ShortAttached => vec![format!("-o{value}")],
            Self::Long => vec!["--output".into(), value.into()],
            Self::Attached => vec![format!("--output={value}")],
        }
    }
}

/// Every file under the fixture's workspace (home, cwd, and tools).
fn workspace_files(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        for entry in fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                pending.push(path.clone());
            }
            found.push(path);
        }
    }
    found.sort();
    found
}

/// Where the option sits relative to the command's target (the wrapper's
/// prompt or the composition file) and its other options.
#[derive(Clone, Copy, Debug)]
enum OutputPosition {
    /// `<command> -o v -y --sandbox <target>`
    BeforeTarget,
    /// `<command> -y -o v --sandbox <target>`
    BetweenOptions,
    /// `<command> -y --sandbox <target> -o v`
    AfterTarget,
}

/// What the command acts on: the wrapper's prompt or a document written into
/// the fixture.
#[derive(Clone, Copy)]
enum Target {
    Prompt,
    Document(&'static str),
}

/// Runs the command for every value with the option at `position` and asserts
/// the refusal contract: exit 2, the message alone on stderr, empty stdout,
/// nothing created, and neither the editor nor the provider launched.
fn assert_output_refused(
    prefix: &str,
    command: &[&str],
    target: Target,
    spelling: OutputSpelling,
    position: OutputPosition,
    extra: &[&str],
) {
    let run = EditRun::new(prefix, "code");
    let target = match target {
        Target::Prompt => "say hello".to_string(),
        Target::Document(content) => run.write_doc("doc.md", content).to_string_lossy().into_owned(),
    };
    for value in OUTPUT_VALUES {
        let option = spelling.words(value);
        let mut args: Vec<String> = command.iter().map(|word| (*word).to_string()).collect();
        args.extend(extra.iter().map(|word| (*word).to_string()));
        match position {
            OutputPosition::BeforeTarget => {
                args.extend(option);
                args.extend(["-y".to_string(), "--sandbox".to_string(), target.clone()]);
            }
            OutputPosition::BetweenOptions => {
                args.push("-y".to_string());
                args.extend(option);
                args.extend(["--sandbox".to_string(), target.clone()]);
            }
            OutputPosition::AfterTarget => {
                args.extend(["-y".to_string(), "--sandbox".to_string(), target.clone()]);
                args.extend(option);
            }
        }
        let args: Vec<&str> = args.iter().map(String::as_str).collect();
        let files_before = workspace_files(run.fixture.workspace_path());

        let output = run.run(&args);

        let text = stderr(&output);
        assert_eq!(output.status.code(), Some(2), "{args:?}: {text}");
        assert_eq!(
            text.trim(),
            format!("the --output {value} CLI option has not been implemented yet in Claudine"),
            "{args:?}"
        );
        assert!(output.stdout.is_empty(), "{args:?}: {}", stdout(&output));
        assert_eq!(workspace_files(run.fixture.workspace_path()), files_before, "{args:?}: nothing was created");
        assert!(run.launches().is_empty(), "{args:?}: the editor never ran");
        assert_eq!(run.provider_launches(), 0, "{args:?}: the provider never ran");
    }
}

macro_rules! output_refused_at {
    ($prefix:expr, $command:expr, $target:expr, $position:ident) => {
        mod $position {
            use super::*;

            #[test]
            fn short_spelling() {
                assert_output_refused(&format!("{}-{}-short", $prefix, stringify!($position)), &$command, $target, OutputSpelling::Short, OutputPosition::$position, &[]);
            }

            #[test]
            fn short_attached_spelling() {
                assert_output_refused(&format!("{}-{}-short-attached", $prefix, stringify!($position)), &$command, $target, OutputSpelling::ShortAttached, OutputPosition::$position, &[]);
            }

            #[test]
            fn long_spelling() {
                assert_output_refused(&format!("{}-{}-long", $prefix, stringify!($position)), &$command, $target, OutputSpelling::Long, OutputPosition::$position, &[]);
            }

            #[test]
            fn attached_spelling() {
                assert_output_refused(&format!("{}-{}-attached", $prefix, stringify!($position)), &$command, $target, OutputSpelling::Attached, OutputPosition::$position, &[]);
            }

            #[test]
            fn under_dry_run() {
                assert_output_refused(&format!("{}-{}-dry-run", $prefix, stringify!($position)), &$command, $target, OutputSpelling::Long, OutputPosition::$position, &["--dry-run"]);
            }
        }
    };
}

macro_rules! output_refused_tests {
    ($module:ident, $prefix:literal, $command:expr, $target:expr) => {
        #[allow(non_snake_case)]
        mod $module {
            use super::*;

            output_refused_at!($prefix, $command, $target, BeforeTarget);
            output_refused_at!($prefix, $command, $target, BetweenOptions);
            output_refused_at!($prefix, $command, $target, AfterTarget);
        }
    };
}

output_refused_tests!(wrapper_output_is_refused, "output-wrapper", ["goose"], Target::Prompt);
output_refused_tests!(compose_output_is_refused, "output-compose", ["compose", "--goose"], Target::Document(REFUSED_DOCUMENT));
output_refused_tests!(
    inline_compose_output_is_refused,
    "output-inline-compose",
    ["inline-compose", "--goose"],
    Target::Document(REFUSED_INLINE_DOCUMENT)
);
output_refused_tests!(sequence_output_is_refused, "output-sequence", ["sequence", "--goose"], Target::Document(REFUSED_SEQUENCE));

/// A loop document is refused before its first iteration.
#[test]
fn a_loop_run_output_is_refused() {
    let document = "---\nloop:\n  until: \"loop.count >= 2\"\ninitialize:\n  - ensure_file: made.md\n  - edit: a.md\n---\nbody\n";
    for position in [OutputPosition::BeforeTarget, OutputPosition::AfterTarget] {
        assert_output_refused(&format!("output-loop-{position:?}"), &["compose", "--goose"], Target::Document(document), OutputSpelling::Long, position, &[]);
    }
}

/// A wrapper without a prompt starts an interactive session; the option is
/// refused before that too.
#[test]
fn an_interactive_wrapper_output_is_refused() {
    let run = EditRun::new("output-wrapper-interactive", "code");

    let output = run.run(&["goose", "-o", "json"]);

    assert_eq!(output.status.code(), Some(2), "{}", stderr(&output));
    assert_eq!(stderr(&output).trim(), "the --output json CLI option has not been implemented yet in Claudine");
    assert!(output.stdout.is_empty());
    assert_eq!(run.provider_launches(), 0);
}

// Item 7: cancellation terminates and reaps the editor.

/// Runs `compose` on a document whose `initialize` holds a GUI editor that
/// saved bytes and spawned a child, interrupts it once the editor is ready,
/// and asserts the cancellation contract.
#[cfg(unix)]
fn interrupt_a_held_edit(prefix: &str, ignore_term: bool) {
    use crate::common::signal::SignalledRun;
    use test_toolkit::fake_editor::{
        FAKE_EDITOR_HOLD, FAKE_EDITOR_IGNORE_TERM, FAKE_EDITOR_PIDS, FAKE_EDITOR_SPAWN_CHILD,
    };

    let mut run = EditRun::new(prefix, "code").writing("saved before cancel");
    let ready = run.fixture.workspace_path().join("editor-ready");
    let pids = run.fixture.workspace_path().join("editor-pids");
    let _cleanup = PidCleanup(pids.clone());
    run = run
        .with(FAKE_EDITOR_HOLD, ready.to_string_lossy())
        .with(FAKE_EDITOR_SPAWN_CHILD, "1")
        .with(FAKE_EDITOR_PIDS, pids.to_string_lossy());
    if ignore_term {
        run = run.with(FAKE_EDITOR_IGNORE_TERM, "1");
    }
    let document = run.write_doc(
        "doc.md",
        "---\ninitialize:\n  - edit:\n      file: held.md\n      create_new: true\n      no_error: true\n  \
         - info: \"event:after-edit\"\ncancelled:\n  - info: \"event:cancelled\"\nfinalize:\n  \
         - info: \"event:finalize\"\n---\nbody\n",
    );
    let mut child = SignalledRun::spawn(
        run.command(&["compose", "--goose", document.to_str().unwrap()]),
        run.fixture.workspace_path(),
    );
    wait_for_file(&ready);
    let child_pid = wait_for_pids(&pids, 1)[0];

    child.interrupt();
    let exit = child.wait_for_exit(Duration::from_secs(4));

    assert_eq!(exit.status.code(), Some(130), "{}", exit.stderr);
    wait_until_dead(child_pid);
    assert_eq!(
        fs::read_to_string(run.cwd().join("held.md")).unwrap(),
        "saved before cancel",
        "the created file and its saved bytes are kept"
    );
    assert_eq!(
        markers(&exit.stderr),
        ["cancelled", "finalize"],
        "the next action never runs, even with `no_error`: {}",
        exit.stderr
    );
    assert_eq!(run.provider_launches(), 0);
}

#[cfg(unix)]
#[test]
fn an_interrupt_terminates_a_held_editor_tree_and_keeps_what_it_saved() {
    interrupt_a_held_edit("edit-cancel-held", false);
}

/// The editor and its child ignore `SIGTERM`; the cancellation escalates to
/// `SIGKILL` after its grace period.
#[cfg(unix)]
#[test]
fn an_interrupt_escalates_to_kill_for_an_editor_that_ignores_term() {
    interrupt_a_held_edit("edit-cancel-ignore-term", true);
}

/// A failed sibling in a fail-fast group cancels a leg whose prompt holds a
/// GUI editor open in `initialize`.
#[cfg(unix)]
#[test]
fn a_failing_sibling_cancels_a_held_editor_in_a_fail_fast_group() {
    use test_toolkit::fake_editor::{FAKE_EDITOR_HOLD, FAKE_EDITOR_PIDS, FAKE_EDITOR_SPAWN_CHILD};

    let run = EditRun::new("edit-cancel-sibling", "code").writing("saved before cancel");
    let ready = run.fixture.workspace_path().join("editor-ready");
    let pids = run.fixture.workspace_path().join("editor-pids");
    let _cleanup = PidCleanup(pids.clone());
    let run = run
        .with(FAKE_EDITOR_HOLD, ready.to_string_lossy())
        .with(FAKE_EDITOR_SPAWN_CHILD, "1")
        .with(FAKE_EDITOR_PIDS, pids.to_string_lossy());
    run.write_doc(
        "review.md",
        "---\ninitialize:\n  - edit:\n      file: held.md\n      no_error: true\n  - info: \"event:after-edit\"\n\
         cancelled: []\nfinalize:\n  - info: \"event:review-finalize\"\n---\nreview\n",
    );
    let document = run.write_doc(
        "ship.md",
        &format!(
            "---\nsequence:\n  - name: checks\n    group:\n      name: checks\n      tasks:\n        \
             - name: review\n          prompt: review.md\n        \
             - name: boom\n          shell: \"n=0; while [ ! -f '{}' ] && [ $n -lt 100 ]; do n=$((n + 1)); sleep 0.01; done; exit 3\"\n---\n",
            ready.display()
        ),
    );

    let output = run.sequence(&document);

    let text = stderr(&output);
    assert_eq!(output.status.code(), Some(1), "{text}");
    for pid in test_toolkit::fake_editor::read_pids(&pids).unwrap_or_default() {
        wait_until_dead(pid);
    }
    assert_eq!(fs::read_to_string(run.cwd().join("held.md")).unwrap_or_default(), "saved before cancel", "{text}");
    assert_eq!(markers(&text), ["review-finalize"], "{text}");
    assert!(text.contains("review — interrupted"), "the leg reports as interrupted: {text}");
}

// Item 8: an editor that cannot wait is refused.

/// `lapce` has no blocking flag, so the action refuses it before creating
/// the target.
#[test]
fn an_edit_refuses_a_configured_editor_that_cannot_wait() {
    let run = EditRun::new("edit-cannot-wait", "lapce");
    let document = run.write_doc("doc.md", "---\ninitialize:\n  - edit: a.md\n  - skip\n---\nbody\n");

    let output = run.compose(&document, &[]);

    let text = stderr(&output);
    assert_eq!(output.status.code(), Some(1), "{text}");
    assert!(text.contains("cannot wait for the file to close"), "{text}");
    assert!(run.launches().is_empty(), "{text}");
    assert!(!run.cwd().join("a.md").exists(), "{text}");
}

/// The wrapper's `--edit` resolves the same editor and refuses it before the
/// editor or the provider runs. `--edit` needs an interactive terminal, so
/// this runs in a bare PTY.
#[cfg(unix)]
#[test]
fn wrapper_edit_refuses_a_configured_editor_that_cannot_wait() {
    use expectrl::{Expect as _, Session};
    use test_toolkit::{Level, expect_level};

    expect_level!(Level::L1, crate::common::pty_available(), "PTY (/dev/ptmx)");
    let run = EditRun::new("edit-wrapper-cannot-wait", "lapce");
    run.fixture.seed_user_config();
    let mut command = run.command(&["goose", "--edit", "seed prompt"]);
    command.env("TERM_WIDTH", "4000");

    let mut session = Session::spawn(command).expect("spawn claudine in a PTY");
    session.set_expect_timeout(Some(Duration::from_secs(4)));
    let captured = session.expect(expectrl::Eof).expect("claudine exits");
    let transcript = strip_ansi(&String::from_utf8_lossy(&[captured.before(), captured.as_bytes()].concat()));

    assert!(transcript.contains("configured editor cannot wait"), "{transcript}");

    assert!(run.launches().is_empty(), "the editor never ran: {transcript}");
    assert_eq!(run.provider_launches(), 0, "the provider never ran: {transcript}");
}
