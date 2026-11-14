//! Editor resolution through the real process environment and `PATH` lookup,
//! with the fake editor installed under real editors' names, so the same
//! evidence holds on every OS. The full order and filtering matrix (probes,
//! dual-surface and unknown commands, candidate lists) is exercised
//! in-process by `darkmatter::editor::resolve`'s unit tests; these prove the
//! environment, Sniff identification, and the launched argument vector.
//!
//! Every case sets `EDITOR` or `VISUAL` to a program that exists and is
//! accepted (or a refusal that stops before the probes), so the host's
//! installed editors never decide an outcome.

use serial_test::serial;
use test_toolkit::argv_recorder::read_argv_records;
use test_toolkit::fake_editor::{FAKE_EDITOR_EXIT, FAKE_EDITOR_RECORD, FAKE_EDITOR_WRITE};

use crate::editor_support::{ScopedEnv, editor_dir, install_fake_editor};
use darkmatter::editor::{
    EditorError, SurfaceRequest, edit_text, launch_editor_on_path, resolve_editor, resolve_editor_command,
};
use sniff::programs::EditorSurface;

#[test]
#[serial]
fn editor_wins_over_visual_and_its_arguments_reach_the_editor() {
    let editors = editor_dir();
    let chosen = install_fake_editor(editors.path(), "editor-choice");
    let other = install_fake_editor(editors.path(), "visual-choice");
    let work = tempfile::tempdir().unwrap();
    let record = work.path().join("argv");
    let target = work.path().join("a note; $HOME & ünïcode.md");
    std::fs::write(&target, "seed").unwrap();

    let mut env = ScopedEnv::new();
    env.set("EDITOR", format!("{} --flag", chosen.display()))
        .set("VISUAL", &other)
        .set(FAKE_EDITOR_RECORD, &record);

    assert_eq!(resolve_editor_command().unwrap(), format!("{} --flag", chosen.display()));
    launch_editor_on_path(&target).unwrap();

    let records = read_argv_records(&record).unwrap();
    assert_eq!(records, vec![vec!["--flag".to_string(), target.display().to_string()]]);
}

#[test]
#[serial]
fn a_blank_or_missing_editor_falls_to_visual() {
    let editors = editor_dir();
    let visual = install_fake_editor(editors.path(), "visual-choice");

    for editor in ["   ", "/definitely/missing/editor"] {
        let mut env = ScopedEnv::new();
        env.set("EDITOR", editor).set("VISUAL", &visual);
        assert_eq!(resolve_editor_command().unwrap(), visual.display().to_string(), "EDITOR={editor:?}");
    }

    let mut env = ScopedEnv::new();
    env.remove("EDITOR").set("VISUAL", &visual);
    assert_eq!(resolve_editor_command().unwrap(), visual.display().to_string(), "EDITOR unset");
}

#[test]
#[serial]
fn a_missing_configured_editor_refuses_rather_than_probing() {
    let mut env = ScopedEnv::new();
    env.set("EDITOR", "/definitely/missing/editor").remove("VISUAL");

    let error = resolve_editor_command().unwrap_err();
    assert!(
        matches!(&error, EditorError::ConfiguredEditorMissing { variable: "EDITOR", value } if value == "/definitely/missing/editor"),
        "{error}"
    );
}

#[test]
#[serial]
fn a_configured_non_blocking_editor_is_refused_as_unable_to_wait() {
    let editors = editor_dir();
    let geany = install_fake_editor(editors.path(), "geany");
    let mut env = ScopedEnv::new();
    env.set("EDITOR", &geany).remove("VISUAL");

    let error = resolve_editor(SurfaceRequest::Any).unwrap_err();
    assert!(matches!(error, EditorError::ConfiguredEditorCannotWait { variable: "EDITOR", .. }), "{error}");
    assert!(error.to_string().contains("cannot wait"), "{error}");
}

#[test]
#[serial]
fn a_gui_editor_named_by_absolute_path_gets_its_wait_flag_before_the_path() {
    let editors = editor_dir();
    let code = install_fake_editor(editors.path(), "code");
    let work = tempfile::tempdir().unwrap();
    let record = work.path().join("argv");
    let target = work.path().join("-leading-dash.md");
    std::fs::write(&target, "").unwrap();

    let mut env = ScopedEnv::new();
    env.set("EDITOR", &code).remove("VISUAL").set(FAKE_EDITOR_RECORD, &record);

    let resolved = resolve_editor(SurfaceRequest::Any).unwrap();
    assert_eq!(resolved.surface, Some(EditorSurface::Gui));
    launch_editor_on_path(&target).unwrap();

    assert_eq!(
        read_argv_records(&record).unwrap(),
        vec![vec!["--wait".to_string(), target.display().to_string()]]
    );
}

#[test]
#[serial]
fn a_configured_wait_flag_is_not_doubled() {
    let editors = editor_dir();
    let code = install_fake_editor(editors.path(), "code");
    let mut env = ScopedEnv::new();
    env.set("EDITOR", format!("{} --wait", code.display())).remove("VISUAL");

    assert_eq!(resolve_editor(SurfaceRequest::Any).unwrap().args, vec!["--wait"]);
}

#[test]
#[serial]
fn gui_true_skips_a_terminal_editor_for_a_gui_visual_and_gui_false_keeps_it() {
    let editors = editor_dir();
    let nvim = install_fake_editor(editors.path(), "nvim");
    let code = install_fake_editor(editors.path(), "code");
    let mut env = ScopedEnv::new();
    env.set("EDITOR", &nvim).set("VISUAL", &code);

    let any = resolve_editor(SurfaceRequest::Any).unwrap();
    assert_eq!(any.program, nvim);
    let gui = resolve_editor(SurfaceRequest::Gui).unwrap();
    assert_eq!((gui.program, gui.args), (code, vec!["--wait".to_string()]));
    let terminal = resolve_editor(SurfaceRequest::Terminal).unwrap();
    assert_eq!(terminal.program, nvim);
}

#[test]
#[serial]
fn md_edit_surfaces_a_non_zero_exit() {
    let editors = editor_dir();
    let editor = install_fake_editor(editors.path(), "failing-editor");
    let work = tempfile::tempdir().unwrap();
    let target = work.path().join("note.md");
    std::fs::write(&target, "seed").unwrap();

    let mut env = ScopedEnv::new();
    env.set("EDITOR", &editor).remove("VISUAL").set(FAKE_EDITOR_EXIT, "23");

    let error = launch_editor_on_path(&target).unwrap_err();
    assert!(
        matches!(&error, EditorError::NonZeroExit { code: 23, editor: shown, path } if *shown == editor.display().to_string() && *path == target),
        "{error}"
    );
}

#[test]
#[serial]
fn wrapper_edit_text_returns_the_trimmed_buffer() {
    let editors = editor_dir();
    let editor = install_fake_editor(editors.path(), "writing-editor");
    let work = tempfile::tempdir().unwrap();
    let content = work.path().join("content");
    std::fs::write(&content, "edited body\n\n").unwrap();

    let mut env = ScopedEnv::new();
    env.set("EDITOR", &editor).remove("VISUAL").set(FAKE_EDITOR_WRITE, &content);

    assert_eq!(edit_text("seed", ".md").unwrap(), Some("edited body".to_string()));
}
