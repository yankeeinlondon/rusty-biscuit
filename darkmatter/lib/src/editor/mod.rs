//! Reusable editor discovery and launch utilities.
//!
//! Shared by Darkmatter's `md edit` flow, Claudine wrapper prompt editing,
//! and the lifecycle `edit` action. One resolver ([`resolve_editor`]) picks
//! the editor, one launcher ([`LaunchPlan`]) starts and waits for it, and
//! [`ProcessEditorService`] adds the `edit` action's file preparation and
//! host eligibility. Editor facts (surface, wait flags) come only from
//! Sniff's `Editor` catalog.

mod launch;
mod resolve;
mod service;
pub mod surface;

use std::io::Write;
use std::path::{Path, PathBuf};

use sniff::programs::{Editor, ProgramMetadata};
use thiserror::Error;

pub use launch::{EDITOR_TERMINATE_GRACE, LaunchIo, LaunchOptions, LaunchOutput, LaunchPlan, LaunchStdin};
pub use resolve::{RejectReason, ResolvedEditor, SurfaceRequest, resolve_editor, resolve_editor_command};
pub use service::{EditError, EditOutcome, EditRequest, EditorHostFacts, EditorOutput, EditorService, ProcessEditorService};
pub use surface::{EnvironmentProbe, TerminalDenyScope, deny_terminal_in_this_thread, terminal_denied};

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
    /// `$EDITOR` or `$VISUAL` is set, none is accepted, and the first set one
    /// names a program that was not found.
    #[error("{variable} names '{value}', which was not found; fix it or unset it")]
    ConfiguredEditorMissing {
        /// The first variable that is set.
        variable: &'static str,
        /// What it holds.
        value: String,
    },
    /// `$EDITOR` or `$VISUAL` is set, none is accepted, and the first set one
    /// names an editor that returns before the file is closed.
    #[error("{variable} names '{value}', which cannot wait for the file to close; set it to an editor that blocks or unset it")]
    ConfiguredEditorCannotWait {
        /// The first variable that is set.
        variable: &'static str,
        /// What it holds.
        value: String,
    },
    /// No candidate has the surface the caller required.
    #[error("{}", no_matching_message(*requested, candidates))]
    NoMatchingEditor {
        requested: SurfaceRequest,
        /// Every variable and probe checked, in order, with why it was
        /// passed over.
        candidates: Vec<(String, RejectReason)>,
    },
    /// The run was cancelled while the editor was open; its process tree was
    /// terminated. Whatever the editor saved before that is kept.
    #[error("the editor on {} was closed because the run was cancelled", path.display())]
    Cancelled { path: PathBuf },
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
    /// A filesystem or temp-file I/O failure, or a failure to contain or wait
    /// for the editor's process.
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
        use biscuit_terminal::components::prose::{LineBreaks, Prose};
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
                    .body(Prose::new(format!(
                        "<dim>Checked env:</dim> <cyan>$EDITOR</cyan>, <cyan>$VISUAL</cyan>\n<dim>Probed binaries:</dim> {probed}"
                    )).with_line_breaks(LineBreaks::Hard))
                    .hint("Set <cyan>$EDITOR</cyan> to your preferred editor, or install one of the probed binaries.")
            }

            EditorError::ConfiguredEditorMissing { variable, value } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("EditorError", "configured editor not found"))
                .body(Prose::new(format!(
                    "<cyan>${variable}</cyan> names <cyan>{}</cyan>, which was not found.",
                    Prose::escape_text(value)
                )))
                .hint("Fix the variable or unset it to use the first installed editor."),

            EditorError::ConfiguredEditorCannotWait { variable, value } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("EditorError", "configured editor cannot wait"))
                .body(Prose::new(format!(
                    "<cyan>${variable}</cyan> names <cyan>{}</cyan>, which returns before the file is closed.",
                    Prose::escape_text(value)
                )))
                .hint("Set the variable to an editor that blocks (a terminal editor, or a GUI editor with a wait flag), or unset it."),

            EditorError::NoMatchingEditor { requested, candidates } => {
                let checked = candidates
                    .iter()
                    .map(|(candidate, reason)| {
                        format!("<cyan>{}</cyan> <dim>({reason})</dim>", Prose::escape_text(candidate))
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                StatusBlock::new(StatusState::Error)
                    .error_header(ErrorHeader::new(
                        "EditorError",
                        match requested {
                            SurfaceRequest::Gui => "no gui editor found",
                            SurfaceRequest::Terminal => "no terminal editor found",
                            SurfaceRequest::Any => "no editor found",
                        },
                    ))
                    .body(Prose::new(format!("<dim>Checked:</dim>\n{checked}")).with_line_breaks(LineBreaks::Hard))
                    .hint("Set <cyan>$EDITOR</cyan> or <cyan>$VISUAL</cyan> to an editor of that kind, or install one.")
            }

            EditorError::Cancelled { path } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("EditorError", "editing cancelled"))
                .body(Prose::new(format!(
                    "The run was cancelled while <cyan>{}</cyan> was open; the editor was closed.",
                    path.display()
                )))
                .hint("Anything saved before the cancellation is kept."),

            EditorError::NonZeroExit { code, editor, path } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("EditorError", "editor exited with error"))
                .body(Prose::new(format!(
                    "<dim>Editor:</dim> <cyan>{editor}</cyan>\n<dim>Path:</dim> <cyan>{}</cyan>\n<dim>Exit code:</dim> {code}",
                    path.display()
                )).with_line_breaks(LineBreaks::Hard))
                .hint("Re-run the editor manually to see its full diagnostic output."),

            EditorError::Missing { path } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("EditorError", "edited file missing"))
                .body(Prose::new(format!(
                    "The temporary edit buffer was deleted before it could be re-read.\n<dim>Path:</dim> <cyan>{}</cyan>",
                    path.display()
                )).with_line_breaks(LineBreaks::Hard))
                .hint("Avoid deleting the buffer file while editing; save and close normally."),

            EditorError::LaunchFailed {
                editor,
                full_command,
                source,
            } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("EditorError", "launch failed"))
                .body(Prose::new(format!(
                    "<dim>Editor:</dim> <cyan>{editor}</cyan>\n<dim>Command:</dim> <cyan>{full_command}</cyan>\n<dim>Kind:</dim> {:?}\n{source}",
                    source.kind()
                )).with_line_breaks(LineBreaks::Hard))
                .hint("Check the editor binary is executable and on <cyan>$PATH</cyan>."),

            EditorError::Io { operation, source } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("EditorError", "I/O error"))
                .body(Prose::new(format!(
                    "<dim>Operation:</dim> {operation}\n<dim>Kind:</dim> {:?}\n{source}",
                    source.kind()
                )).with_line_breaks(LineBreaks::Hard))
                .hint("The operation names the failed step: for a file, confirm its location is writable and has space; for the editor's process, re-run the editor manually."),
        }
    }
}

/// One line naming the requested surface and every candidate checked.
fn no_matching_message(requested: SurfaceRequest, candidates: &[(String, RejectReason)]) -> String {
    let mut checked = Vec::new();
    let mut not_installed = Vec::new();
    for (candidate, reason) in candidates {
        match reason {
            RejectReason::NotInstalled => not_installed.push(candidate.as_str()),
            reason => checked.push(format!("{candidate} ({reason})")),
        }
    }
    if !not_installed.is_empty() {
        checked.push(format!("not installed: {}", not_installed.join(", ")));
    }
    format!("no {requested} editor found; checked {}", checked.join("; "))
}

/// Open a file path in the resolved editor and block until it closes.
///
/// The editor shares the host's process group and streams, and the wait
/// cannot be interrupted.
pub fn launch_editor_on_path(path: &Path) -> Result<(), EditorError> {
    LaunchPlan::new(resolve_editor(SurfaceRequest::Any)?, path).run(LaunchOptions::PLAIN)
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
    fn a_configured_editor_that_is_missing_refuses_rather_than_probing() {
        let mut env_guard = ScopedEnv::new();
        env_guard.set("EDITOR", "/definitely/missing/editor");
        env_guard.set("VISUAL", "/definitely/missing/visual");

        let error = resolve_editor_command().unwrap_err();
        assert!(
            matches!(&error, EditorError::ConfiguredEditorMissing { variable: "EDITOR", value } if value == "/definitely/missing/editor"),
            "{error}"
        );
    }

    #[cfg(unix)]
    #[test]
    #[serial]
    fn a_blank_editor_variable_counts_as_unset() {
        let dir = tempdir().unwrap();
        let visual = write_editor_script(dir.path(), "visual-choice", "#!/bin/sh\nexit 0\n");

        let mut env_guard = ScopedEnv::new();
        env_guard.set("EDITOR", "   ");
        env_guard.set("VISUAL", visual.as_os_str());

        assert_eq!(resolve_editor_command().unwrap(), visual.display().to_string());
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
