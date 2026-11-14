//! The lifecycle `edit` action through `md compose`, with the
//! `darkmatter-fake-editor` fixture installed under real editors' names.
//!
//! An L1 spawn has piped standard streams, so a terminal editor (`vim`) is
//! always refused here; these tests prove its refusals and use a GUI editor
//! (`code`, launched with `--wait`) for every run that opens the file. The
//! blocking terminal handoff is the L2 test `level2_edit_action`.
//!
//! Each run records the editor's arguments, so "the editor never ran" is the
//! record file's absence, and "nothing was created" is checked on disk.

use std::path::{Path, PathBuf};
use std::process::Output;

use serde_json::Value;
use test_toolkit::argv_recorder::read_argv_records;
use test_toolkit::fake_editor::{
    FAKE_EDITOR_DELETE, FAKE_EDITOR_ERR_TEXT, FAKE_EDITOR_EXIT, FAKE_EDITOR_OUT_TEXT, FAKE_EDITOR_RECORD,
    FAKE_EDITOR_STDIO, FAKE_EDITOR_WRITE, read_stdio_report,
};

use crate::common::CliProcessFixture;
use crate::editor_support::{editor_dir, install_fake_editor};

/// The motivating workflow: refuse an existing spec, otherwise create and
/// open it, then skip the rest of the run.
const MOTIVATING: &str = r#"---
fqn: "{{ ctx.cwd }}/features/2026-11-14-example/spec.md"
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

/// A fixture plus an installed editor and the files its runs report into.
struct EditRun {
    fixture: CliProcessFixture,
    _editors: tempfile::TempDir,
    editor: PathBuf,
    record: PathBuf,
    /// Extra environment for the editor.
    env: Vec<(&'static str, String)>,
}

impl EditRun {
    /// `editor` is the name the fake editor is installed under: `code` is a
    /// GUI editor, `vim` a terminal one.
    fn new(prefix: &str, editor: &str) -> Self {
        let fixture = CliProcessFixture::named(prefix);
        let editors = editor_dir();
        let editor = install_fake_editor(editors.path(), editor);
        let record = fixture.tmp_dir().join("editor-record");
        Self { fixture, _editors: editors, editor, record, env: Vec::new() }
    }

    fn with(mut self, key: &'static str, value: impl Into<String>) -> Self {
        self.env.push((key, value.into()));
        self
    }

    /// The editor writes `bytes` into its target.
    fn writing(self, bytes: &str) -> Self {
        let content = self.fixture.write_file("tmp/editor-content", bytes);
        self.with(FAKE_EDITOR_WRITE, content.to_string_lossy())
    }

    fn cwd(&self) -> &Path {
        self.fixture.cwd()
    }

    fn compose(&self, document: &Path, extra: &[&str]) -> Output {
        let mut command = self.command();
        command.arg("compose").args(extra).arg(document);
        self.editor_env(&mut command);
        command.output().expect("md runs")
    }

    /// An `md` command whose stderr is wide enough that a failure line,
    /// which names two temporary paths, is never shortened.
    fn command(&self) -> std::process::Command {
        self.fixture.command_builder().rendering_input("COLUMNS", "4000").build_std()
    }

    fn editor_env(&self, command: &mut std::process::Command) {
        command
            .env("EDITOR", &self.editor)
            .env_remove("VISUAL")
            // A display session, so Linux admits a GUI editor; macOS and
            // Windows always have one.
            .env("DISPLAY", ":0")
            .env_remove("WAYLAND_DISPLAY")
            .env(FAKE_EDITOR_RECORD, &self.record);
        for (key, value) in &self.env {
            command.env(key, value);
        }
    }

    /// Each launch's arguments, in order; empty when the editor never ran.
    fn launches(&self) -> Vec<Vec<String>> {
        match read_argv_records(&self.record) {
            Ok(records) => records,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Vec::new(),
            Err(error) => panic!("read the editor record: {error}"),
        }
    }

    /// The one launch's target: its last argument.
    fn launched_target(&self) -> PathBuf {
        let launches = self.launches();
        assert_eq!(launches.len(), 1, "exactly one launch: {launches:?}");
        PathBuf::from(launches[0].last().expect("the target argument"))
    }
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn canonical(path: &Path) -> PathBuf {
    biscuit_file::canonicalize_simplified(path).expect("the path exists")
}

/// The only JSON envelope a data-output run printed.
fn envelope(output: &Output) -> Value {
    let text = stdout(output);
    let mut values = serde_json::Deserializer::from_str(&text).into_iter::<Value>();
    let first = values.next().expect("one envelope").expect("valid JSON");
    assert!(values.next().is_none(), "exactly one envelope: {text}");
    first
}

#[test]
fn the_motivating_workflow_creates_and_opens_the_spec_then_skips() {
    let run = EditRun::new("edit-motivating", "code").writing("# Spec\r\n\ttrailing  \n");
    let document = run.fixture.write_file("cwd/new-spec.md", MOTIVATING);
    let target = run.cwd().join("features/2026-11-14-example/spec.md");

    let output = run.compose(&document, &[]);

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let launch = &run.launches()[0];
    assert_eq!(launch.first().map(String::as_str), Some("--wait"), "the GUI editor's wait flag: {launch:?}");
    assert_eq!(run.launched_target(), canonical(&target), "an absolute native path");
    assert_eq!(std::fs::read(&target).unwrap(), b"# Spec\r\n\ttrailing  \n", "saved bytes kept exactly");
    assert!(stderr(&output).contains("The spec is ready at"), "{}", stderr(&output));
    assert!(stdout(&output).trim().is_empty(), "`skip` writes no document: {}", stdout(&output));

    // Now the spec exists: the warning branch, no editor.
    let again = run.compose(&document, &[]);
    assert_eq!(again.status.code(), Some(0), "{}", stderr(&again));
    assert!(stderr(&again).contains("The spec already exists"), "{}", stderr(&again));
    assert_eq!(run.launches().len(), 1, "the warning branch never launches");
    assert_eq!(std::fs::read(&target).unwrap(), b"# Spec\r\n\ttrailing  \n");
}

/// The prompt lives in the workspace above the launch directory; the absolute
/// `{{ ctx.cwd }}` target names the launch directory's file for both the
/// probe and the edit. `md`'s mutation root is the prompt's own directory,
/// so a prompt in a sibling library cannot edit the launch directory: the
/// edit is refused before anything is created.
#[test]
fn a_prompt_outside_the_launch_directory_edits_the_absolute_target_inside_its_root() {
    let run = EditRun::new("edit-outside-prompt", "code").writing("written");
    let above = run.fixture.write_file("prompt.md", MOTIVATING);
    let target = run.cwd().join("features/2026-11-14-example/spec.md");

    let output = run.compose(&above, &[]);

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(run.launched_target(), canonical(&target));
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "written");

    std::fs::remove_dir_all(run.cwd().join("features")).unwrap();
    let sibling = run.fixture.write_file("library/prompt.md", MOTIVATING);
    let refused = run.compose(&sibling, &[]);
    assert_eq!(refused.status.code(), Some(1), "{}", stderr(&refused));
    assert!(stderr(&refused).contains("initialize[0].else[0].edit"), "{}", stderr(&refused));
    assert_eq!(run.launches().len(), 1, "the refused edit never launched");
    assert!(!run.cwd().join("features").exists(), "nothing was created");
}

/// A side effect between the existence check and the edit plays the
/// concurrent creator.
#[test]
fn a_name_taken_after_the_check_fails_create_new_and_opens_without_it() {
    let race = |create_new: &str, no_error: &str| {
        format!(
            "---\nfqn: \"{{{{ ctx.cwd }}}}/spec.md\"\ninitialize:\n  - when: file_exists(fqn)\n    then:\n      - warn: exists\n    \
             else:\n      - ensure_file: [\"{{{{ fqn }}}}\", \"theirs\"]\n      - edit:\n          file: \"{{{{ fqn }}}}\"\n{create_new}{no_error}      \
             - info: \"after the edit\"\n---\nbody\n"
        )
    };
    let run = EditRun::new("edit-race", "code").writing("mine");
    let target = run.cwd().join("spec.md");

    let exclusive = run.fixture.write_file("cwd/exclusive.md", &race("          create_new: true\n", ""));
    let output = run.compose(&exclusive, &[]);
    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert!(stderr(&output).contains("initialize[0].else[1].edit"), "{}", stderr(&output));
    assert!(stderr(&output).contains("`create_new` requires a new file"), "{}", stderr(&output));
    assert!(!stderr(&output).contains("after the edit"));
    assert!(run.launches().is_empty(), "never launched");
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "theirs", "the other creator's bytes are untouched");

    std::fs::remove_file(&target).unwrap();
    let tolerated = run.fixture.write_file(
        "cwd/tolerated.md",
        &race("          create_new: true\n", "          no_error: true\n"),
    );
    let output = run.compose(&tolerated, &[]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert!(stderr(&output).contains("after the edit"), "`no_error` continues: {}", stderr(&output));
    assert!(run.launches().is_empty());
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "theirs");

    std::fs::remove_file(&target).unwrap();
    let ordinary = run.fixture.write_file("cwd/ordinary.md", &race("", ""));
    let output = run.compose(&ordinary, &[]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(run.launched_target(), canonical(&target), "the existing file is opened");
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "mine");
}

#[test]
fn a_create_new_name_held_by_a_directory_or_symlink_fails_without_launch() {
    let run = EditRun::new("edit-occupied", "code");
    std::fs::create_dir_all(run.cwd().join("dir.md")).unwrap();
    let mut names = vec!["dir.md"];
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(run.cwd().join("absent.md"), run.cwd().join("link.md")).unwrap();
        names.push("link.md");
    }
    for name in names {
        let document = run.fixture.write_file(
            "cwd/doc.md",
            &format!("---\ncomposed:\n  - edit:\n      file: \"{name}\"\n      create_new: true\n---\nbody\n"),
        );
        let output = run.compose(&document, &[]);
        assert_eq!(output.status.code(), Some(1), "{name}: {}", stderr(&output));
        assert!(stderr(&output).contains("`create_new` requires a new file"), "{name}: {}", stderr(&output));
        assert!(run.launches().is_empty(), "{name}: never launched");
    }
    assert!(!run.cwd().join("absent.md").exists(), "a dangling symlink is never followed");
}

/// `edit` runs where it is authored: in a later event, inside `then`/`else`,
/// and after a compact dictionary's communication. A missing include routes
/// the second run to `failed`.
#[test]
fn edit_runs_in_order_in_composed_failed_and_finalize() {
    let run = EditRun::new("edit-order", "code");
    let launched = |run: &EditRun| -> Vec<String> {
        run.launches()
            .iter()
            .map(|launch| Path::new(launch.last().unwrap()).file_name().unwrap().to_string_lossy().into_owned())
            .collect()
    };
    let markers = |text: &str| -> Vec<String> {
        text.lines()
            .filter_map(|line| line.split("event:").nth(1))
            .map(|rest| rest.split_whitespace().next().unwrap_or_default().to_string())
            .collect()
    };
    let composed = run.fixture.write_file(
        "cwd/composed.md",
        "---\ncomposed:\n  - info: \"event:composed-before\"\n  - when: \"true\"\n    then:\n      - edit: composed.md\n        \
         info: \"event:composed-same-item\"\n  - info: \"event:composed-after\"\n\
         finalize:\n  - edit: finalize.md\n  - info: \"event:finalize-after\"\n---\nbody\n",
    );
    let output = run.compose(&composed, &[]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(launched(&run), ["composed.md", "finalize.md"], "{}", stderr(&output));
    assert_eq!(
        markers(&stderr(&output)),
        ["composed-before", "composed-same-item", "composed-after", "finalize-after"],
        "{}",
        stderr(&output)
    );

    std::fs::remove_file(&run.record).unwrap();
    let failing = run.fixture.write_file(
        "cwd/failing.md",
        "---\nfailed:\n  - when: \"false\"\n    then: [info: never]\n    else:\n      - edit: failed.md\n  \
         - info: \"event:failed-after\"\nfinalize:\n  - info: \"event:finalize-after\"\n---\n::file ./missing.md\n",
    );
    let output = run.compose(&failing, &[]);
    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(launched(&run), ["failed.md"], "{}", stderr(&output));
    assert_eq!(markers(&stderr(&output)), ["failed-after", "finalize-after"], "{}", stderr(&output));
    for name in ["composed.md", "failed.md", "finalize.md"] {
        assert!(run.cwd().join(name).is_file(), "{name} was created");
    }
}

/// A read later in the same stack sees what the editor did, including after
/// a failed edit that `no_error` absorbed.
#[test]
fn a_read_after_the_edit_sees_changed_and_deleted_data() {
    let changed = EditRun::new("edit-read-after", "code").writing("- v2\n").with(FAKE_EDITOR_EXIT, "3");
    changed.fixture.write_file("cwd/data.yaml", "- v1\n");
    let document = changed.fixture.write_file(
        "cwd/doc.md",
        "---\ncomposed:\n  - info: \"before={{ load_data('data.yaml', 'string[]')[0] }}\"\n  - edit:\n      file: data.yaml\n      \
         no_error: true\n  - info: \"after={{ load_data('data.yaml', 'string[]')[0] }}\"\n---\nbody\n",
    );
    let output = changed.compose(&document, &[]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let text = stderr(&output);
    assert!(text.contains("before=v1") && text.contains("after=v2"), "{text}");
    assert_eq!(changed.launches().len(), 1);

    let deleted = EditRun::new("edit-read-deleted", "code").with(FAKE_EDITOR_DELETE, "1");
    deleted.fixture.write_file("cwd/gone.md", "x");
    let document = deleted.fixture.write_file(
        "cwd/doc.md",
        "---\ncomposed:\n  - info: \"before={{ file_exists('gone.md') }}\"\n  - edit:\n      file: gone.md\n      \
         no_error: true\n  - info: \"after={{ file_exists('gone.md') }}\"\n---\nbody\n",
    );
    let output = deleted.compose(&document, &[]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let text = stderr(&output);
    assert!(text.contains("before=true") && text.contains("after=false"), "{text}");
}

#[test]
fn editor_failures_name_the_action_property_and_target_and_stop_the_stack() {
    let document_for = |run: &EditRun, file: &str| {
        run.fixture.write_file(
            "cwd/doc.md",
            &format!("---\ncomposed:\n  - edit: \"{file}\"\n  - info: \"after the edit\"\n---\nbody\n"),
        )
    };
    let cases: Vec<(&str, EditRun, &str, &str)> = vec![
        ("non-zero exit", EditRun::new("edit-fail-exit", "code").with(FAKE_EDITOR_EXIT, "3"), "a.md", "exited with status 3"),
        ("deleted target", EditRun::new("edit-fail-deleted", "code").with(FAKE_EDITOR_DELETE, "1"), "b.md", "deleted"),
        ("directory target", EditRun::new("edit-fail-dir", "code"), "folder", "is not a regular file"),
        ("outside the root", EditRun::new("edit-fail-outside", "code"), "../outside.md", "outside"),
        ("no editor", EditRun::new("edit-fail-missing", "code"), "c.md", "EDITOR"),
        ("url", EditRun::new("edit-fail-url", "code"), "https://example.invalid/spec.md", "is a URL"),
    ];
    for (label, mut run, file, expected) in cases {
        if label == "directory target" {
            std::fs::create_dir_all(run.cwd().join("folder")).unwrap();
        }
        if label == "no editor" {
            run.editor = run.cwd().join("no-such-editor");
        }
        let document = document_for(&run, file);
        let output = run.compose(&document, &[]);
        let text = stderr(&output);
        assert_eq!(output.status.code(), Some(1), "{label}: {text}");
        assert!(text.contains("`composed[0].edit` could not edit the file"), "{label}: {text}");
        assert!(text.contains(expected), "{label}: {text}");
        assert!(!text.contains("after the edit"), "{label}: no continuation");
        let launched = !run.launches().is_empty();
        assert_eq!(launched, matches!(label, "non-zero exit" | "deleted target"), "{label}: launched");
    }
    assert!(!std::env::temp_dir().join("outside.md").exists());
}

#[cfg(unix)]
#[test]
fn a_symlink_escaping_the_root_is_refused_before_anything_is_created() {
    let run = EditRun::new("edit-symlink-escape", "code");
    let outside = run.fixture.workspace_path().join("elsewhere");
    std::fs::create_dir_all(&outside).unwrap();
    std::os::unix::fs::symlink(&outside, run.cwd().join("escape")).unwrap();
    let document = run.fixture.write_file("cwd/doc.md", "---\ncomposed:\n  - edit: escape/new/spec.md\n---\nbody\n");

    let output = run.compose(&document, &[]);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert!(stderr(&output).contains("`composed[0].edit` could not edit the file"), "{}", stderr(&output));
    assert!(run.launches().is_empty());
    assert!(!outside.join("new").exists(), "no parent directory was created through the link");
}

/// Every terminal-editor refusal happens before creation; the document read
/// from stdin keeps stdin from the editor.
#[test]
fn a_terminal_editor_without_an_interactive_terminal_is_refused_before_creation() {
    let run = EditRun::new("edit-terminal-refused", "vim");
    let body = "---\ncomposed:\n  - edit: new/spec.md\n  - info: \"after the edit\"\n---\nbody\n";
    let document = run.fixture.write_file("cwd/doc.md", body);

    let output = run.compose(&document, &[]);
    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert!(stderr(&output).contains("is a terminal editor and needs an interactive terminal"), "{}", stderr(&output));
    assert!(!stderr(&output).contains("after the edit"));
    assert!(run.launches().is_empty());
    assert!(!run.cwd().join("new").exists(), "no target or parent directory");

    let mut command = run.command();
    command.args(["compose", "-"]).stdin(std::process::Stdio::piped()).stdout(std::process::Stdio::piped());
    command.stderr(std::process::Stdio::piped());
    run.editor_env(&mut command);
    let mut child = command.spawn().expect("md runs");
    std::io::Write::write_all(&mut child.stdin.take().unwrap(), body.as_bytes()).unwrap();
    let piped = child.wait_with_output().unwrap();
    assert_eq!(piped.status.code(), Some(1), "{}", stderr(&piped));
    assert!(stderr(&piped).contains("stdin carries the prompt or document"), "{}", stderr(&piped));
    assert!(run.launches().is_empty());
    assert!(!run.cwd().join("new").exists());
}

/// Markdown output: the editor's own output goes to stderr, never into the
/// document on stdout; a GUI editor's stdin is null.
#[test]
fn a_gui_editors_output_never_reaches_the_composed_document() {
    let run = EditRun::new("edit-markdown-output", "code")
        .with(FAKE_EDITOR_OUT_TEXT, "EDITOR-SCREEN-OUT")
        .with(FAKE_EDITOR_ERR_TEXT, "EDITOR-SCREEN-ERR");
    let report = run.fixture.tmp_dir().join("stdio.json");
    let run = run.with(FAKE_EDITOR_STDIO, report.to_string_lossy());
    let document = run.fixture.write_file("cwd/doc.md", "---\ncomposed:\n  - edit: notes.md\n---\nthe-document\n");

    let output = run.compose(&document, &[]);

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(stdout(&output), "the-document\n");
    assert!(stderr(&output).contains("EDITOR-SCREEN-OUT"), "{}", stderr(&output));
    assert!(stderr(&output).contains("EDITOR-SCREEN-ERR"), "{}", stderr(&output));
    let stdio = read_stdio_report(&report).unwrap();
    assert!(!stdio.stdin_tty);
    assert_eq!(stdio.stdin.as_deref(), Some(""), "stdin is null");
}

/// Data output keeps both streams for one envelope: a terminal editor is
/// refused into it, and a GUI editor runs with its output discarded.
#[test]
fn structured_output_refuses_a_terminal_editor_and_silences_a_gui_editor() {
    let terminal = EditRun::new("edit-json-terminal", "vim").with(FAKE_EDITOR_OUT_TEXT, "EDITOR-SCREEN");
    let failing = terminal.fixture.write_file("cwd/fail.md", "---\ncomposed:\n  - edit: a.md\n---\nbody\n");
    let output = terminal.compose(&failing, &["--output", "json"]);
    assert_eq!(output.status.code(), Some(1));
    let report = envelope(&output);
    assert_eq!(report["outcome"], "failure", "{report:#}");
    let message = report["diagnostics"][0]["message"].as_str().unwrap_or_default();
    assert!(message.contains("structured output cannot lend it the terminal"), "{report:#}");
    assert!(output.stderr.is_empty(), "{}", stderr(&output));
    assert!(!stdout(&output).contains("EDITOR-SCREEN"));
    assert!(terminal.launches().is_empty());

    let tolerated = terminal
        .fixture
        .write_file("cwd/tolerated.md", "---\ncomposed:\n  - edit:\n      file: a.md\n      no_error: true\n---\nbody\n");
    let output = terminal.compose(&tolerated, &["--output", "json"]);
    assert_eq!(output.status.code(), Some(0), "{}", stdout(&output));
    assert_eq!(envelope(&output)["outcome"], "success");
    assert_eq!(envelope(&output)["document"], "body");
    assert!(output.stderr.is_empty(), "{}", stderr(&output));
    assert!(!terminal.cwd().join("a.md").exists());

    let gui = EditRun::new("edit-json-gui", "code")
        .with(FAKE_EDITOR_OUT_TEXT, "EDITOR-SCREEN-OUT")
        .with(FAKE_EDITOR_ERR_TEXT, "EDITOR-SCREEN-ERR");
    let document = gui.fixture.write_file("cwd/doc.md", "---\ncomposed:\n  - edit: a.md\n---\nbody\n");
    let output = gui.compose(&document, &["--output", "json"]);
    assert_eq!(output.status.code(), Some(0), "{}", stdout(&output));
    assert_eq!(envelope(&output)["outcome"], "success");
    assert!(output.stderr.is_empty(), "{}", stderr(&output));
    assert!(!stdout(&output).contains("EDITOR-SCREEN"));
    assert_eq!(gui.launches().len(), 1);
}

/// Passive and report-only surfaces never launch an editor or create the
/// target; an untaken branch runs nothing, yet a bad call in it still stops
/// the run before anything runs.
#[test]
fn passive_surfaces_and_untaken_branches_never_launch_or_create() {
    let run = EditRun::new("edit-passive", "code");
    let document = run.fixture.write_file(
        "cwd/doc.md",
        "---\ncomposed:\n  - edit:\n      file: new/spec.md\n      create_new: true\n---\nbody\n",
    );
    let shell = run.compose(&document, &["--shell"]);
    assert_eq!(shell.status.code(), Some(0), "{}", stderr(&shell));
    let mut validate = run.command();
    validate.args(["schema", "validate"]).arg(&document);
    run.editor_env(&mut validate);
    let validate = validate.output().unwrap();
    assert!(validate.status.code().is_some_and(|code| code < 2), "{}", stderr(&validate));
    assert!(run.launches().is_empty());
    assert!(!run.cwd().join("new").exists());

    let untaken = run.fixture.write_file(
        "cwd/untaken.md",
        "---\ncomposed:\n  - when: \"false\"\n    then:\n      - edit: new/spec.md\n---\nbody\n",
    );
    let output = run.compose(&untaken, &[]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert!(run.launches().is_empty());
    assert!(!run.cwd().join("new").exists());

    let invalid = run.fixture.write_file(
        "cwd/invalid.md",
        "---\ncomposed:\n  - when: \"false\"\n    then:\n      - edit: \"{{ min(true, 5) }}\"\n---\nbody\n",
    );
    let output = run.compose(&invalid, &[]);
    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert!(stderr(&output).contains("composed[0].then[0].edit.file"), "{}", stderr(&output));
    assert!(run.launches().is_empty());
}

/// Library composition runs no lifecycle, so an authored `edit` neither
/// launches nor creates anything.
#[test]
fn library_composition_never_runs_an_edit() {
    use darkmatter::markdown::Markdown;
    use darkmatter::markdown::compose::{ComposeContext, ComposeOptions, ComposeRequest, RequestSnapshot};

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("doc.md");
    std::fs::write(&path, "---\ncomposed:\n  - edit:\n      file: new/spec.md\n      create_new: true\n---\nbody\n").unwrap();
    let options =
        ComposeOptions::new_with_context(ComposeContext::capture_for_content(dir.path(), "")).with_source_file(path.clone());
    let snapshot = RequestSnapshot::new(dir.path()).with_home(Some(dir.path().to_path_buf()));
    let request = ComposeRequest::prepare(options, &snapshot).expect("request");

    let (composed, _) = Markdown::try_from(path.as_path()).unwrap().compose_with(&request).expect("composes");

    assert!(composed.content().contains("body"), "{}", composed.content());
    assert!(!dir.path().join("new").exists(), "nothing was created");
}

/// `md schema validate` and `md compose` refuse the same malformed `edit`
/// operands at the same authored place, and neither launches an editor or
/// creates the target; the unedited motivating workflow validates clean.
#[test]
fn schema_validate_and_compose_refuse_malformed_operands_alike_without_launch() {
    let run = EditRun::new("edit-schema-wiring", "code");
    let validate = |document: &Path| {
        let mut command = run.command();
        command.args(["schema", "validate"]).arg(document);
        run.editor_env(&mut command);
        command.output().expect("md runs")
    };

    let control = run.fixture.write_file("cwd/control.md", MOTIVATING);
    let output = validate(&control);
    assert_eq!(output.status.code(), Some(0), "{}{}", stdout(&output), stderr(&output));

    for (name, text, pointer, position, property) in [
        (
            "gui.md",
            "---\ncomposed:\n  - edit:\n      file: notes.md\n      gui: \"yes\"\n---\nbody\n",
            "composed/0/edit/gui",
            "line 5, column 12",
            "composed[0].edit.gui",
        ),
        ("empty.md", "---\ncomposed:\n  - edit: []\n---\nbody\n", "composed/0/edit", "line 3, column 11", "composed[0].edit"),
    ] {
        let document = run.fixture.write_file(format!("cwd/{name}"), text);
        let checked = validate(&document);
        let report = format!("{}{}", stdout(&checked), stderr(&checked));
        assert_eq!(checked.status.code(), Some(1), "{name}: {report}");
        assert!(report.contains(pointer) && report.contains(position), "{name}: {report}");

        let composed = run.compose(&document, &[]);
        assert_eq!(composed.status.code(), Some(1), "{name}: {}", stderr(&composed));
        assert!(stderr(&composed).contains(property), "{name}: {}", stderr(&composed));
    }
    assert!(run.launches().is_empty(), "no editor ran: {:?}", run.launches());
    assert!(!run.cwd().join("notes.md").exists(), "nothing was created");
    assert!(!run.cwd().join("features").exists(), "validation created nothing");
}
