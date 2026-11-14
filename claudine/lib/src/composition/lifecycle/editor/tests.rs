use std::path::{Path, PathBuf};

use darkmatter::editor::{EditError, EditRequest, EditorOutput, EnvironmentProbe};
use sniff::os::DisplaySession;

use super::*;

/// A host with every terminal and no display: what a Linux SSH session
/// without X11 or Wayland forwarding looks like.
fn no_display_probe() -> EnvironmentProbe {
    EnvironmentProbe {
        stdin_tty: true,
        stdout_tty: true,
        stderr_tty: true,
        foreground: true,
        display: DisplaySession::Unavailable,
    }
}

/// An executable named like VS Code, so the resolver identifies a GUI
/// editor. It is never launched.
fn install_gui_editor(dir: &Path) -> PathBuf {
    let path = dir.join(if cfg!(windows) { "code.exe" } else { "code" });
    std::fs::write(&path, b"").expect("write the stand-in editor");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).expect("make it executable");
    }
    path
}

#[test]
fn the_output_mode_follows_the_invocation_and_stdin_is_never_reserved() {
    set_structured_output(true);
    let structured = service_with(no_display_probe());
    assert_eq!(structured.facts.output, EditorOutput::Structured);
    assert!(!structured.facts.stdin_reserved);

    set_structured_output(false);
    let text = service_with(no_display_probe());
    assert_eq!(text.facts.output, EditorOutput::Terminal);
    assert!(!text.facts.stdin_reserved);
    assert_eq!(text.probe, no_display_probe(), "the injected probe is the one used");
}

#[test]
fn a_host_without_a_display_refuses_a_gui_editor_before_creating_the_file() {
    let workspace = tempfile::tempdir().expect("workspace");
    let tools = tempfile::tempdir().expect("tools");
    let editor = install_gui_editor(tools.path());
    // SAFETY: nextest runs each test in its own process, and nothing else in
    // this one reads the environment concurrently.
    unsafe {
        std::env::set_var("EDITOR", &editor);
        std::env::remove_var("VISUAL");
    }
    set_structured_output(false);

    let target = workspace.path().join("specs").join("new-spec.md");
    let cancelled = || false;
    let outcome = service_with(no_display_probe()).edit(&EditRequest {
        target: &target,
        mutation_root: workspace.path(),
        create_new: true,
        gui: Some(true),
        cancelled: &cancelled,
    });

    assert!(!outcome.attempted, "refused before any file preparation");
    let error = outcome.result.expect_err("no display session");
    assert!(matches!(error, EditError::NoDisplaySession { .. }), "{error:?}");
    assert!(error.to_string().contains("no display session"), "{error}");
    assert!(!target.exists(), "the target was not created");
    assert!(!target.parent().expect("parent").exists(), "no parent directory was created");
}
