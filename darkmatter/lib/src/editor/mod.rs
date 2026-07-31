//! Reusable editor discovery and launch utilities.
//!
//! Shared by Darkmatter's `md edit` flow and Claudine wrapper prompt editing.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

use sniff::programs::{Editor, ProgramMetadata, find_program};
use thiserror::Error;

/// Default editor priority when neither `$EDITOR` nor `$VISUAL` resolve to an
/// installed binary. Ordered from most capable/modern to most basic.
pub const DEFAULT_EDITOR_PRIORITY: &[Editor] = &[
    Editor::Neovim,
    Editor::Helix,
    Editor::Vim,
    Editor::Zed,
    Editor::VSCode,
    Editor::VSCodium,
    Editor::Sublime,
    Editor::Micro,
    Editor::Kakoune,
    Editor::Emacs,
    Editor::Lapce,
    Editor::TextMate,
    Editor::BBEdit,
    Editor::Kate,
    Editor::Geany,
    Editor::Nano,
    Editor::Vi,
    Editor::Amp,
    Editor::XEmacs,
    Editor::PhpStorm,
    Editor::IntellijIdea,
    Editor::PyCharm,
    Editor::WebStorm,
    Editor::CLion,
    Editor::GoLand,
    Editor::Rider,
];

/// Errors returned by reusable editor workflows.
#[derive(Debug, Error)]
pub enum EditorError {
    /// No supported editor could be resolved from environment or fallback probes.
    #[error("no editor found; set $EDITOR or $VISUAL, or install one of: nvim, vim, code, nano")]
    NoEditorFound,
    /// The launched editor returned a non-zero exit status.
    #[error(
        "editor '{editor}' exited with status {code} while editing {path}",
        path = path.display()
    )]
    NonZeroExit {
        code: i32,
        editor: String,
        path: PathBuf,
    },
    /// The edited file disappeared before the content could be re-read.
    #[error("edited file was deleted during editing: {path}", path = path.display())]
    Missing { path: PathBuf },
    /// Spawning the editor command failed before it could run.
    #[error("failed to launch editor '{editor}' via '{full_command}': {source}")]
    LaunchFailed {
        editor: String,
        full_command: String,
        #[source]
        source: std::io::Error,
    },
    /// Generic filesystem or temp-file IO failure.
    #[error("editor workflow failed during {operation}: {source}")]
    Io {
        operation: &'static str,
        #[source]
        source: std::io::Error,
    },
}

impl biscuit_terminal::errors::BlockError for EditorError {
    fn status_block(
        &self,
        _term: &biscuit_terminal::terminal::Terminal,
    ) -> biscuit_terminal::components::status_block::StatusBlock {
        use biscuit_terminal::components::status::StatusState;
        use biscuit_terminal::components::status_block::StatusBlock;
        use biscuit_terminal::errors::{ErrorHeader, StatusBlockExt};

        match self {
            EditorError::NoEditorFound => {
                let probed = DEFAULT_EDITOR_PRIORITY
                    .iter()
                    .map(|e| format!("<cyan>{}</cyan>", e.binary_name()))
                    .collect::<Vec<_>>()
                    .join(", ");
                StatusBlock::new(StatusState::Error)
                    .error_header(ErrorHeader::new("EditorError", "no editor found"))
                    .body(format!(
                        "<dim>Checked env:</dim> <cyan>$EDITOR</cyan>, <cyan>$VISUAL</cyan>\n<dim>Probed binaries:</dim> {probed}"
                    ))
                    .hint("Set <cyan>$EDITOR</cyan> to your preferred editor, or install one of the probed binaries.")
            }

            EditorError::NonZeroExit { code, editor, path } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("EditorError", "editor exited with error"))
                .body(format!(
                    "<dim>Editor:</dim> <cyan>{editor}</cyan>\n<dim>Path:</dim> <cyan>{}</cyan>\n<dim>Exit code:</dim> {code}",
                    path.display()
                ))
                .hint("Re-run the editor manually to see its full diagnostic output."),

            EditorError::Missing { path } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("EditorError", "edited file missing"))
                .body(format!(
                    "The temporary edit buffer was deleted before it could be re-read.\n<dim>Path:</dim> <cyan>{}</cyan>",
                    path.display()
                ))
                .hint("Avoid deleting the buffer file while editing; save and close normally."),

            EditorError::LaunchFailed {
                editor,
                full_command,
                source,
            } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("EditorError", "launch failed"))
                .body(format!(
                    "<dim>Editor:</dim> <cyan>{editor}</cyan>\n<dim>Command:</dim> <cyan>{full_command}</cyan>\n<dim>Kind:</dim> {:?}\n{source}",
                    source.kind()
                ))
                .hint("Check the editor binary is executable and on <cyan>$PATH</cyan>."),

            EditorError::Io { operation, source } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("EditorError", "I/O error"))
                .body(format!(
                    "<dim>Operation:</dim> {operation}\n<dim>Kind:</dim> {:?}\n{source}",
                    source.kind()
                ))
                .hint("Confirm the temp-file location is writable and has space available."),
        }
    }
}

/// Parse the configured editor command into launch pieces plus a stable
/// display string.
///
/// The returned `full_command` includes the editor binary, any configured
/// editor arguments, and any auto-added wait flags, but intentionally omits
/// the final file path so the preview is stable across temp-file runs.
fn build_editor_command_details(editor_cmd: &str) -> (String, Vec<String>, String) {
    let mut parts = editor_cmd.split_whitespace();
    let editor_bin = parts.next().unwrap_or(editor_cmd).to_string();
    let editor_args = parts.map(ToOwned::to_owned).collect::<Vec<_>>();

    let mut display_parts = vec![editor_bin.clone()];
    display_parts.extend(editor_args.iter().cloned());
    display_parts.extend(
        wait_args_for_editor(&editor_bin)
            .iter()
            .map(|flag| (*flag).to_string()),
    );
    let full_command = display_parts.join(" ");

    (editor_bin, editor_args, full_command)
}

/// Resolve the editor command using `$EDITOR`, `$VISUAL`, then fallback probes.
pub fn resolve_editor_command() -> Result<String, EditorError> {
    if let Ok(editor) = std::env::var("EDITOR") {
        let cmd = editor.split_whitespace().next().unwrap_or(&editor);
        if find_program(cmd).is_some() {
            return Ok(editor);
        }
    }

    if let Ok(visual) = std::env::var("VISUAL") {
        let cmd = visual.split_whitespace().next().unwrap_or(&visual);
        if find_program(cmd).is_some() {
            return Ok(visual);
        }
    }

    let editors = sniff::programs::InstalledEditors::new();
    for &editor in DEFAULT_EDITOR_PRIORITY {
        if editors.is_installed(editor) {
            return Ok(editor.binary_name().to_string());
        }
    }

    Err(EditorError::NoEditorFound)
}

/// Returns the CLI flags needed to make a GUI editor block until the file is closed.
pub fn wait_args_for_editor(binary: &str) -> &'static [&'static str] {
    match binary {
        "code" | "codium" | "code-insiders" => &["--wait"],
        "subl" => &["--wait"],
        "zed" => &["--wait"],
        "mate" => &["--wait"],
        "bbedit" => &["--wait"],
        "kate" => &["--block"],
        "phpstorm" | "idea" | "pycharm" | "webstorm" | "clion" | "goland" | "rider" => &["--wait"],
        _ => &[],
    }
}

/// Open a file path in the resolved editor and block until it closes.
pub fn launch_editor_on_path(path: &Path) -> Result<(), EditorError> {
    let editor_cmd = resolve_editor_command()?;
    let (editor_bin, editor_args, full_command) = build_editor_command_details(&editor_cmd);

    let mut cmd = Command::new(&editor_bin);
    for arg in &editor_args {
        cmd.arg(arg);
    }
    for flag in wait_args_for_editor(&editor_bin) {
        cmd.arg(flag);
    }
    cmd.arg(path);

    let status = cmd.status().map_err(|source| EditorError::LaunchFailed {
        editor: editor_cmd.clone(),
        full_command,
        source,
    })?;

    if !status.success() {
        return Err(EditorError::NonZeroExit {
            code: status.code().unwrap_or(-1),
            editor: editor_cmd,
            path: path.to_path_buf(),
        });
    }

    Ok(())
}

/// Edit text in a temp file and return the trimmed contents on success.
pub fn edit_text(initial: &str, suffix: &str) -> Result<Option<String>, EditorError> {
    let mut temp_file = tempfile::Builder::new()
        .suffix(suffix)
        .tempfile()
        .map_err(|source| EditorError::Io {
            operation: "create temp file",
            source,
        })?;
    temp_file
        .write_all(initial.as_bytes())
        .map_err(|source| EditorError::Io {
            operation: "write temp file",
            source,
        })?;
    temp_file.flush().map_err(|source| EditorError::Io {
        operation: "flush temp file",
        source,
    })?;
    temp_file
        .as_file_mut()
        .sync_all()
        .map_err(|source| EditorError::Io {
            operation: "sync temp file",
            source,
        })?;

    let path = temp_file.path().to_path_buf();
    launch_editor_on_path(&path)?;

    if !path.exists() {
        return Err(EditorError::Missing { path });
    }

    let edited = std::fs::read_to_string(&path).map_err(|source| EditorError::Io {
        operation: "read temp file",
        source,
    })?;
    let trimmed = edited.trim_end().to_string();
    if trimmed.is_empty() {
        Ok(None)
    } else {
        Ok(Some(trimmed))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(unix)]
    use serial_test::serial;
    #[cfg(unix)]
    use std::{env, ffi::OsString};
    #[cfg(unix)]
    use tempfile::tempdir;

    #[cfg(unix)]
    struct ScopedEnv {
        vars: Vec<(&'static str, Option<OsString>)>,
    }

    #[cfg(unix)]
    impl ScopedEnv {
        fn new() -> Self {
            Self { vars: Vec::new() }
        }

        fn set(&mut self, key: &'static str, value: impl AsRef<std::ffi::OsStr>) {
            self.vars.push((key, env::var_os(key)));
            // SAFETY: tests using this helper are marked serial, so environment
            // mutation is not concurrent within this process.
            unsafe { env::set_var(key, value) };
        }

        fn remove(&mut self, key: &'static str) {
            self.vars.push((key, env::var_os(key)));
            // SAFETY: tests using this helper are marked serial, so environment
            // mutation is not concurrent within this process.
            unsafe { env::remove_var(key) };
        }
    }

    #[cfg(unix)]
    impl Drop for ScopedEnv {
        fn drop(&mut self) {
            for (key, original) in self.vars.iter().rev() {
                // SAFETY: tests using this helper are marked serial, so environment
                // mutation is not concurrent within this process.
                unsafe {
                    match original {
                        Some(value) => env::set_var(key, value),
                        None => env::remove_var(key),
                    }
                }
            }
        }
    }

    #[cfg(unix)]
    fn write_editor_script(dir: &Path, name: &str, body: &str) -> std::path::PathBuf {
        use std::os::unix::fs::PermissionsExt;

        let path = dir.join(name);
        std::fs::write(&path, body).unwrap();
        let mut perms = std::fs::metadata(&path).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&path, perms).unwrap();
        path
    }

    #[test]
    fn wait_args_vscode_returns_wait() {
        assert_eq!(wait_args_for_editor("code"), &["--wait"]);
    }

    #[test]
    fn wait_args_codium_returns_wait() {
        assert_eq!(wait_args_for_editor("codium"), &["--wait"]);
    }

    #[test]
    fn wait_args_code_insiders_returns_wait() {
        assert_eq!(wait_args_for_editor("code-insiders"), &["--wait"]);
    }

    #[test]
    fn wait_args_sublime_returns_wait() {
        assert_eq!(wait_args_for_editor("subl"), &["--wait"]);
    }

    #[test]
    fn wait_args_zed_returns_wait() {
        assert_eq!(wait_args_for_editor("zed"), &["--wait"]);
    }

    #[test]
    fn wait_args_kate_returns_block() {
        assert_eq!(wait_args_for_editor("kate"), &["--block"]);
    }

    #[test]
    fn wait_args_jetbrains_ides_return_wait() {
        for ide in [
            "phpstorm", "idea", "pycharm", "webstorm", "clion", "goland", "rider",
        ] {
            assert_eq!(
                wait_args_for_editor(ide),
                &["--wait"],
                "expected --wait for {ide}"
            );
        }
    }

    #[test]
    fn wait_args_textmate_returns_wait() {
        assert_eq!(wait_args_for_editor("mate"), &["--wait"]);
    }

    #[test]
    fn wait_args_bbedit_returns_wait() {
        assert_eq!(wait_args_for_editor("bbedit"), &["--wait"]);
    }

    #[test]
    fn wait_args_terminal_editors_return_empty() {
        for editor in ["nvim", "vim", "vi", "hx", "nano", "micro", "kak", "emacs"] {
            assert!(
                wait_args_for_editor(editor).is_empty(),
                "expected no wait args for {editor}"
            );
        }
    }

    #[test]
    fn wait_args_unknown_binary_returns_empty() {
        assert!(wait_args_for_editor("my-custom-editor").is_empty());
    }

    #[cfg(unix)]
    #[test]
    #[serial]
    fn resolve_editor_command_prefers_editor_and_preserves_args() {
        let dir = tempdir().unwrap();
        let editor = write_editor_script(dir.path(), "editor-choice", "#!/bin/sh\nexit 0\n");
        let visual = write_editor_script(dir.path(), "visual-choice", "#!/bin/sh\nexit 0\n");

        let mut env_guard = ScopedEnv::new();
        env_guard.set("EDITOR", format!("{} --wait", editor.display()));
        env_guard.set("VISUAL", visual.as_os_str());

        let resolved = resolve_editor_command().unwrap();
        assert_eq!(resolved, format!("{} --wait", editor.display()));
    }

    #[cfg(unix)]
    #[test]
    #[serial]
    fn resolve_editor_command_falls_back_to_visual_when_editor_is_missing() {
        let dir = tempdir().unwrap();
        let visual = write_editor_script(dir.path(), "visual-choice", "#!/bin/sh\nexit 0\n");

        let mut env_guard = ScopedEnv::new();
        env_guard.set("EDITOR", "/definitely/missing/editor");
        env_guard.set("VISUAL", visual.as_os_str());

        let resolved = resolve_editor_command().unwrap();
        assert_eq!(resolved, visual.display().to_string());
    }

    #[cfg(unix)]
    #[test]
    #[serial]
    fn edit_text_returns_trimmed_contents() {
        let dir = tempdir().unwrap();
        let editor = write_editor_script(
            dir.path(),
            "editor-write",
            "#!/bin/sh\nprintf 'edited body\\n\\n' > \"$1\"\n",
        );

        let mut env_guard = ScopedEnv::new();
        env_guard.set("EDITOR", editor.as_os_str());
        env_guard.remove("VISUAL");

        let edited = edit_text("seed", ".md").unwrap();
        assert_eq!(edited, Some("edited body".to_string()));
    }

    #[cfg(unix)]
    #[test]
    #[serial]
    fn edit_text_returns_none_for_empty_buffer() {
        let dir = tempdir().unwrap();
        let editor = write_editor_script(
            dir.path(),
            "editor-empty",
            "#!/bin/sh\nprintf '   \\n\\n' > \"$1\"\n",
        );

        let mut env_guard = ScopedEnv::new();
        env_guard.set("EDITOR", editor.as_os_str());
        env_guard.remove("VISUAL");

        let edited = edit_text("seed", ".md").unwrap();
        assert_eq!(edited, None);
    }

    #[cfg(unix)]
    #[test]
    #[serial]
    fn edit_text_returns_missing_when_buffer_is_deleted() {
        let dir = tempdir().unwrap();
        let editor = write_editor_script(dir.path(), "editor-delete", "#!/bin/sh\nrm -f \"$1\"\n");

        let mut env_guard = ScopedEnv::new();
        env_guard.set("EDITOR", editor.as_os_str());
        env_guard.remove("VISUAL");

        let err = edit_text("seed", ".md").unwrap_err();
        assert!(
            matches!(err, EditorError::Missing { ref path } if path.extension().and_then(|ext| ext.to_str()) == Some("md"))
        );
    }

    #[cfg(unix)]
    #[test]
    #[serial]
    fn launch_editor_on_path_surfaces_non_zero_exit() {
        let dir = tempdir().unwrap();
        let editor_script = write_editor_script(dir.path(), "editor-fail", "#!/bin/sh\nexit 23\n");
        let file_path = dir.path().join("note.md");
        std::fs::write(&file_path, "seed").unwrap();

        let mut env_guard = ScopedEnv::new();
        env_guard.set("EDITOR", editor_script.as_os_str());
        env_guard.remove("VISUAL");

        let err = launch_editor_on_path(&file_path).unwrap_err();
        assert!(matches!(
            err,
            EditorError::NonZeroExit {
                code: 23,
                ref editor,
                ref path,
            } if editor == &editor_script.display().to_string() && path == &file_path
        ));
    }
}
