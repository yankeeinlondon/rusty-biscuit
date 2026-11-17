//! [`ProcessEditorService`]: host eligibility, file preparation, launch, and
//! the checks after the editor exits, with the fake editor installed under
//! real editors' names and the host's streams described by an injected
//! [`EnvironmentProbe`] (no real terminal is needed or touched).
//!
//! `EDITOR` and the `FAKE_EDITOR_*` variables live in this process's
//! environment, so every test is `#[serial]`.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use serial_test::serial;
use sniff::os::DisplaySession;
use test_toolkit::fake_editor::{
    FAKE_EDITOR_DELETE, FAKE_EDITOR_EXIT, FAKE_EDITOR_HOLD, FAKE_EDITOR_PAUSE, FAKE_EDITOR_RECORD,
    FAKE_EDITOR_REPLACE, FAKE_EDITOR_STDIO, read_stdio_report,
};

use crate::editor_support::{ScopedEnv, editor_dir, install_fake_editor, wait_for_file};
use darkmatter::editor::{
    EditError, EditOutcome, EditRequest, EditorError, EditorHostFacts, EditorOutput, EditorService,
    EnvironmentProbe, ProcessEditorService, deny_terminal_in_this_thread,
};

const INTERACTIVE: EnvironmentProbe = EnvironmentProbe {
    stdin_tty: true,
    stdout_tty: true,
    stderr_tty: true,
    foreground: true,
    display: DisplaySession::Available,
};

/// Recognizes the expected refusal.
type Expected = fn(&EditError) -> bool;

fn service(output: EditorOutput, probe: EnvironmentProbe) -> ProcessEditorService {
    ProcessEditorService { facts: EditorHostFacts { output, stdin_reserved: false }, probe }
}

/// A workspace: the mutation root, and an editor directory outside it.
struct Workspace {
    _root_dir: tempfile::TempDir,
    root: PathBuf,
    editors: tempfile::TempDir,
}

impl Workspace {
    fn new() -> Self {
        let root_dir = tempfile::tempdir().unwrap();
        let root = biscuit_file::canonicalize_simplified(root_dir.path()).unwrap();
        Self { _root_dir: root_dir, root, editors: editor_dir() }
    }

    fn editor(&self, name: &str) -> PathBuf {
        install_fake_editor(self.editors.path(), name)
    }

    fn edit(&self, service: &ProcessEditorService, target: &Path, create_new: bool, gui: Option<bool>) -> EditOutcome {
        service.edit(&EditRequest {
            target,
            mutation_root: &self.root,
            create_new,
            gui,
            cancelled: &|| false,
        })
    }
}

/// Every eligibility refusal happens after resolution and before anything
/// is created: no file, no parent directory, `attempted == false`.
fn assert_refused(workspace: &Workspace, outcome: EditOutcome, expected: Expected, label: &str) {
    let error = outcome.result.expect_err(label);
    assert!(expected(&error), "{label}: {error}");
    assert!(!outcome.attempted, "{label}: a refusal attempts nothing");
    assert!(!workspace.root.join("new").exists(), "{label}: no parent directory was created");
}

#[test]
#[serial]
fn a_terminal_editor_needs_an_interactive_terminal_the_host_can_lend() {
    let workspace = Workspace::new();
    let target = workspace.root.join("new/notes.md");
    let mut env = ScopedEnv::new();
    for name in ["nvim", "my-editor-wrapper", "emacs"] {
        env.set("EDITOR", workspace.editor(name)).remove("VISUAL");
        let no = |field: fn(&mut EnvironmentProbe)| {
            let mut probe = INTERACTIVE;
            field(&mut probe);
            probe
        };
        let cells: [(&str, ProcessEditorService, Expected); 6] = [
            ("structured output", service(EditorOutput::Structured, INTERACTIVE), |error| {
                matches!(error, EditError::StructuredOutputRefusesTerminalEditor { .. })
            }),
            (
                "stdin reserved",
                ProcessEditorService {
                    facts: EditorHostFacts { output: EditorOutput::Terminal, stdin_reserved: true },
                    probe: INTERACTIVE,
                },
                |error| matches!(error, EditError::NoInteractiveTerminal { reason, .. } if reason.contains("stdin carries")),
            ),
            ("stdin not a tty", service(EditorOutput::Terminal, no(|probe| probe.stdin_tty = false)), |error| {
                matches!(error, EditError::NoInteractiveTerminal { reason, .. } if reason.contains("stdin"))
            }),
            ("stdout not a tty", service(EditorOutput::MarkdownDocument, no(|probe| probe.stdout_tty = false)), |error| {
                matches!(error, EditError::NoInteractiveTerminal { reason, .. } if reason.contains("stdout"))
            }),
            (
                "markdown stderr not a tty",
                service(EditorOutput::MarkdownDocument, no(|probe| probe.stderr_tty = false)),
                |error| matches!(error, EditError::NoInteractiveTerminal { reason, .. } if reason.contains("stderr")),
            ),
            ("background process", service(EditorOutput::Terminal, no(|probe| probe.foreground = false)), |error| {
                matches!(error, EditError::NoInteractiveTerminal { reason, .. } if reason.contains("foreground"))
            }),
        ];
        for (label, service, expected) in cells {
            let outcome = workspace.edit(&service, &target, false, None);
            assert_refused(&workspace, outcome, expected, &format!("{name}: {label}"));
        }

        let _denied = deny_terminal_in_this_thread();
        let outcome = workspace.edit(&service(EditorOutput::Terminal, INTERACTIVE), &target, false, None);
        assert_refused(
            &workspace,
            outcome,
            |error| matches!(error, EditError::NoInteractiveSurface { .. }),
            &format!("{name}: parallel member"),
        );
    }

    // Claudine's terminal output prints to the terminal itself, so stderr
    // need not be one; a display is never required.
    env.set("EDITOR", workspace.editor("nvim"));
    let probe = EnvironmentProbe { stderr_tty: false, display: DisplaySession::Unavailable, ..INTERACTIVE };
    workspace.edit(&service(EditorOutput::Terminal, probe), &target, false, None).result.unwrap();
}

#[test]
#[serial]
fn a_gui_editor_needs_a_display_but_no_terminal() {
    let workspace = Workspace::new();
    let target = workspace.root.join("new/notes.md");
    let mut env = ScopedEnv::new();
    env.set("EDITOR", workspace.editor("code")).remove("VISUAL");

    let headless = EnvironmentProbe { display: DisplaySession::Unavailable, ..INTERACTIVE };
    let outcome = workspace.edit(&service(EditorOutput::Terminal, headless), &target, false, None);
    assert_refused(&workspace, outcome, |error| matches!(error, EditError::NoDisplaySession { .. }), "no display");

    let no_terminal = EnvironmentProbe {
        stdin_tty: false,
        stdout_tty: false,
        stderr_tty: false,
        foreground: false,
        display: DisplaySession::Available,
    };
    let stdio = workspace.editors.path().join("stdio.json");
    env.set(FAKE_EDITOR_STDIO, &stdio);
    for (label, output) in [("structured output", EditorOutput::Structured), ("markdown output", EditorOutput::MarkdownDocument)] {
        let _denied = deny_terminal_in_this_thread();
        let outcome = workspace.edit(&service(output, no_terminal), &target, false, None);
        outcome.result.unwrap_or_else(|error| panic!("{label}: {error}"));
        assert!(outcome.attempted);
        let report = read_stdio_report(&stdio).unwrap();
        assert!(!report.stdin_tty, "{label}: a GUI editor's stdin is null");
        assert_eq!(report.stdin.as_deref(), Some(""), "{label}: a GUI editor reads nothing");
    }
    assert!(target.is_file());
}

#[test]
#[serial]
fn an_eligible_terminal_editor_creates_missing_parents_and_succeeds() {
    let workspace = Workspace::new();
    let target = workspace.root.join("new/deeper/ünïcode notes.md");
    let record = workspace.editors.path().join("argv");
    let mut env = ScopedEnv::new();
    env.set("EDITOR", workspace.editor("nvim")).remove("VISUAL").set(FAKE_EDITOR_RECORD, &record);

    let outcome = workspace.edit(&service(EditorOutput::Terminal, INTERACTIVE), &target, true, None);

    outcome.result.unwrap();
    assert!(outcome.attempted);
    assert_eq!(std::fs::read(&target).unwrap(), b"", "a new file starts empty");
    let records = test_toolkit::argv_recorder::read_argv_records(&record).unwrap();
    let launched = biscuit_file::canonicalize_simplified(&target).unwrap();
    assert_eq!(records, vec![vec![launched.display().to_string()]], "the editor got the absolute native path");
}

#[test]
#[serial]
fn create_new_refuses_an_occupied_name_without_launching() {
    let workspace = Workspace::new();
    let record = workspace.editors.path().join("argv");
    let mut env = ScopedEnv::new();
    env.set("EDITOR", workspace.editor("nvim")).remove("VISUAL").set(FAKE_EDITOR_RECORD, &record);
    let host = service(EditorOutput::Terminal, INTERACTIVE);

    let file = workspace.root.join("existing.md");
    std::fs::write(&file, b"keep\r\n").unwrap();
    let folder = workspace.root.join("folder");
    std::fs::create_dir(&folder).unwrap();
    #[cfg_attr(not(unix), allow(unused_mut))]
    let mut occupied = vec![file.clone(), folder];
    #[cfg(unix)]
    {
        let link = workspace.root.join("link.md");
        std::os::unix::fs::symlink(&file, &link).unwrap();
        let dangling = workspace.root.join("dangling.md");
        std::os::unix::fs::symlink(workspace.root.join("absent.md"), &dangling).unwrap();
        occupied.extend([link, dangling]);
    }

    for target in &occupied {
        let outcome = workspace.edit(&host, target, true, None);
        let error = outcome.result.unwrap_err();
        assert!(matches!(&error, EditError::Occupied { path } if path == target), "{}: {error}", target.display());
    }
    assert!(!record.exists(), "the editor never launched");
    assert_eq!(std::fs::read(&file).unwrap(), b"keep\r\n");
    assert!(!workspace.root.join("absent.md").exists());
}

#[test]
#[serial]
fn an_existing_regular_file_is_opened_with_its_bytes_preserved() {
    let workspace = Workspace::new();
    let mut env = ScopedEnv::new();
    env.set("EDITOR", workspace.editor("nvim")).remove("VISUAL");
    let target = workspace.root.join("kept.md");
    let bytes: &[u8] = b"line one  \r\nline two\t\n\xff\xfe not utf-8 \n\n";
    std::fs::write(&target, bytes).unwrap();

    let outcome = workspace.edit(&service(EditorOutput::Terminal, INTERACTIVE), &target, false, None);

    outcome.result.unwrap();
    assert_eq!(std::fs::read(&target).unwrap(), bytes, "an unchanged file is success and untouched");
}

#[test]
#[serial]
fn a_directory_target_is_not_a_regular_file() {
    let workspace = Workspace::new();
    let record = workspace.editors.path().join("argv");
    let mut env = ScopedEnv::new();
    env.set("EDITOR", workspace.editor("nvim")).remove("VISUAL").set(FAKE_EDITOR_RECORD, &record);
    let folder = workspace.root.join("folder");
    std::fs::create_dir(&folder).unwrap();

    let outcome = workspace.edit(&service(EditorOutput::Terminal, INTERACTIVE), &folder, false, None);

    assert!(matches!(outcome.result, Err(EditError::NotRegularFile { .. })), "{:?}", outcome.result);
    assert!(!record.exists(), "the editor never launched");
}

#[test]
#[serial]
fn a_target_outside_the_mutation_root_is_refused_before_creation() {
    let workspace = Workspace::new();
    let mut env = ScopedEnv::new();
    env.set("EDITOR", workspace.editor("nvim")).remove("VISUAL");
    let outside = workspace.editors.path().join("elsewhere/new.md");

    let outcome = workspace.edit(&service(EditorOutput::Terminal, INTERACTIVE), &outside, false, None);

    assert!(matches!(outcome.result, Err(EditError::Containment(_))), "{:?}", outcome.result);
    assert!(!outcome.attempted);
    assert!(!workspace.editors.path().join("elsewhere").exists());
}

#[test]
#[serial]
fn an_atomic_save_that_replaces_the_file_is_success() {
    let workspace = Workspace::new();
    let content = workspace.editors.path().join("content");
    std::fs::write(&content, b"saved through a rename\n").unwrap();
    let mut env = ScopedEnv::new();
    env.set("EDITOR", workspace.editor("nvim")).remove("VISUAL").set(FAKE_EDITOR_REPLACE, &content);
    let target = workspace.root.join("notes.md");
    std::fs::write(&target, b"before").unwrap();

    workspace.edit(&service(EditorOutput::Terminal, INTERACTIVE), &target, false, None).result.unwrap();

    assert_eq!(std::fs::read(&target).unwrap(), b"saved through a rename\n");
}

#[test]
#[serial]
fn a_file_deleted_by_the_editor_fails_after_exit() {
    let workspace = Workspace::new();
    let mut env = ScopedEnv::new();
    env.set("EDITOR", workspace.editor("nvim")).remove("VISUAL").set(FAKE_EDITOR_DELETE, "1");
    let target = workspace.root.join("notes.md");

    let outcome = workspace.edit(&service(EditorOutput::Terminal, INTERACTIVE), &target, false, None);

    assert!(outcome.attempted);
    assert!(
        matches!(&outcome.result, Err(EditError::Editor(EditorError::Missing { path })) if *path == target),
        "{:?}",
        outcome.result
    );
}

/// The editor is paused while the test turns the target into a directory or
/// (Unix) a symlink escaping the root, then exits normally.
#[test]
#[serial]
fn the_target_is_rechecked_after_the_editor_exits() {
    let workspace = Workspace::new();
    let pause = workspace.editors.path().join("paused");
    let mut env = ScopedEnv::new();
    env.set("EDITOR", workspace.editor("nvim")).remove("VISUAL").set(FAKE_EDITOR_PAUSE, &pause);
    let host = service(EditorOutput::Terminal, INTERACTIVE);
    let outside = workspace.editors.path().join("outside.md");
    std::fs::write(&outside, b"outside").unwrap();

    type Swap = fn(&Path, &Path);
    #[cfg_attr(not(unix), allow(unused_mut))]
    let mut swaps: Vec<(&str, Swap, Expected)> = vec![(
        "directory",
        |target, _| {
            std::fs::remove_file(target).unwrap();
            std::fs::create_dir(target).unwrap();
        },
        |error| matches!(error, EditError::NotRegularFile { .. }),
    )];
    #[cfg(unix)]
    swaps.push((
        "escaping symlink",
        |target, outside| {
            std::fs::remove_file(target).unwrap();
            std::os::unix::fs::symlink(outside, target).unwrap();
        },
        |error| matches!(error, EditError::Containment(_)),
    ));

    for (label, swap, expected) in swaps {
        let target = workspace.root.join(format!("{}.md", label.replace(' ', "-")));
        let outcome = std::thread::scope(|scope| {
            let editing = scope.spawn(|| workspace.edit(&host, &target, false, None));
            wait_for_file(&pause);
            swap(&target, &outside);
            std::fs::remove_file(&pause).unwrap();
            editing.join().unwrap()
        });
        let error = outcome.result.expect_err(label);
        assert!(expected(&error), "{label}: {error}");
    }
    assert_eq!(std::fs::read(&outside).unwrap(), b"outside");
}

/// A name another process creates first is opened, never replaced: the
/// exclusive create reports it as existing.
#[test]
#[serial]
fn a_name_created_concurrently_is_opened_not_replaced() {
    let workspace = Workspace::new();
    let mut env = ScopedEnv::new();
    env.set("EDITOR", workspace.editor("nvim")).remove("VISUAL");
    let target = workspace.root.join("raced.md");
    std::fs::write(&target, b"the other writer's bytes").unwrap();

    workspace.edit(&service(EditorOutput::Terminal, INTERACTIVE), &target, false, None).result.unwrap();
    assert_eq!(std::fs::read(&target).unwrap(), b"the other writer's bytes");

    let outcome = workspace.edit(&service(EditorOutput::Terminal, INTERACTIVE), &target, true, None);
    assert!(matches!(outcome.result, Err(EditError::Occupied { .. })), "{:?}", outcome.result);
}

#[test]
#[serial]
fn a_failed_launch_still_counts_as_attempted() {
    let workspace = Workspace::new();
    let mut env = ScopedEnv::new();
    env.set("EDITOR", workspace.editor("nvim")).remove("VISUAL").set(FAKE_EDITOR_EXIT, "9");
    let target = workspace.root.join("notes.md");

    let outcome = workspace.edit(&service(EditorOutput::Terminal, INTERACTIVE), &target, false, None);

    assert!(outcome.attempted);
    assert!(matches!(outcome.result, Err(EditError::Editor(EditorError::NonZeroExit { code: 9, .. }))), "{:?}", outcome.result);
    assert!(target.is_file(), "the created file stays after a failure");
}

#[test]
#[serial]
fn an_edit_is_refused_once_the_run_is_cancelled() {
    let workspace = Workspace::new();
    let mut env = ScopedEnv::new();
    env.set("EDITOR", workspace.editor("nvim")).remove("VISUAL");
    let target = workspace.root.join("new/notes.md");

    let outcome = service(EditorOutput::Terminal, INTERACTIVE).edit(&EditRequest {
        target: &target,
        mutation_root: &workspace.root,
        create_new: false,
        gui: None,
        cancelled: &|| true,
    });

    let error = outcome.result.unwrap_err();
    assert!(error.is_cancelled(), "{error}");
    assert!(!outcome.attempted);
    assert!(!workspace.root.join("new").exists());
}

/// Only one terminal editor may hold the host's terminal at a time.
#[test]
#[serial]
fn a_second_terminal_editor_waits_for_no_one_and_is_refused() {
    let workspace = Workspace::new();
    let marker = workspace.editors.path().join("ready");
    let mut env = ScopedEnv::new();
    env.set("EDITOR", workspace.editor("nvim")).remove("VISUAL").set(FAKE_EDITOR_HOLD, &marker);
    let host = service(EditorOutput::Terminal, INTERACTIVE);
    let first = workspace.root.join("first.md");
    let second = workspace.root.join("second.md");
    let cancel = AtomicBool::new(false);

    let (first_outcome, second_outcome) = std::thread::scope(|scope| {
        let holding = scope.spawn(|| {
            host.edit(&EditRequest {
                target: &first,
                mutation_root: &workspace.root,
                create_new: false,
                gui: None,
                cancelled: &|| cancel.load(Ordering::SeqCst),
            })
        });
        wait_for_file(&marker);
        let second_outcome = workspace.edit(&host, &second, false, None);
        cancel.store(true, Ordering::SeqCst);
        (holding.join().unwrap(), second_outcome)
    });

    assert!(matches!(second_outcome.result, Err(EditError::SurfaceBusy { .. })), "{:?}", second_outcome.result);
    assert!(!second_outcome.attempted);
    assert!(!second.exists());
    assert!(first_outcome.result.unwrap_err().is_cancelled());
    assert!(first_outcome.attempted);
}
