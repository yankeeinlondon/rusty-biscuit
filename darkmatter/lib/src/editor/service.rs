//! Opening one named file in the user's editor for a lifecycle `edit`.
//!
//! [`EditorService`] is what an `edit` action calls; a host supplies one.
//! [`ProcessEditorService`] is the production implementation. Its order is
//! fixed, and everything before step 4 only reads:
//!
//! 1. refuse when the run is already cancelled;
//! 2. resolve the editor once (`gui` narrows the surface);
//! 3. check that the host can lend the selected editor what it needs: a
//!    terminal editor (or an unrecognized command) needs the host's
//!    interactive terminal, a GUI editor needs a display session;
//! 4. reserve a missing file with one exclusive create, or accept an existing
//!    regular file (`create_new` refuses an existing one);
//! 5. launch the editor on the absolute path and wait, cancellably;
//! 6. after it exits, require the target to still be a regular file inside
//!    the mutation root.
//!
//! No content is read and nothing is rolled back: an unchanged or empty file
//! is success, and saved bytes stay when the action fails or is cancelled.

use std::path::{Path, PathBuf};

use biscuit_file::FileReferenceError;
use sniff::os::DisplaySession;
use thiserror::Error;

use super::launch::{LaunchIo, LaunchOptions, LaunchOutput, LaunchPlan, LaunchStdin};
use super::resolve::{SurfaceRequest, resolve_editor};
use super::surface::{EnvironmentProbe, InteractiveSurface, terminal_denied};
use super::{EditorError, ResolvedEditor};
use crate::effects::EffectError;
use crate::effects::fs_write::{CreateOutcome, create_new_guarded, ensure_within};

/// Opens a file in an editor and waits for it; supplied by a lifecycle host.
pub trait EditorService: Sync {
    fn edit(&self, request: &EditRequest<'_>) -> EditOutcome;
}

/// One `edit` action, with its target already resolved.
pub struct EditRequest<'a> {
    /// The resolved target; it may not exist yet.
    pub target: &'a Path,
    /// Writes, creation included, are allowed only inside this directory.
    pub mutation_root: &'a Path,
    /// Fail instead of opening a file that already exists.
    pub create_new: bool,
    /// `Some(true)` requires a GUI editor, `Some(false)` a terminal editor.
    pub gui: Option<bool>,
    /// Whether the run has been cancelled; polled while the editor is open.
    pub cancelled: &'a dyn Fn() -> bool,
}

/// What an [`EditorService::edit`] call did.
#[derive(Debug)]
pub struct EditOutcome {
    /// `true` once a directory or file creation or the launch was tried, even
    /// if it failed, so the host discards cached reads; `false` for every
    /// refusal before that (cancellation, resolution, eligibility,
    /// containment).
    pub attempted: bool,
    pub result: Result<(), EditError>,
    /// Set when the edit's outcome stands but the host should tell the user
    /// something, such as a GUI editor window that had to be closed.
    pub warning: Option<String>,
}

/// Why an `edit` failed.
#[derive(Debug, Error)]
pub enum EditError {
    /// The host supplies no editor service.
    #[error("this host cannot run `edit`")]
    Unsupported,
    /// A data-output run keeps stdout and stderr for its one result, so it
    /// cannot lend them to a terminal editor.
    #[error("`{editor}` is a terminal editor, and structured output cannot lend it the terminal; set `gui: true` or use a Markdown output")]
    StructuredOutputRefusesTerminalEditor { editor: String },
    /// The calling stack has no interactive terminal of its own, whatever the
    /// process has.
    #[error("`{editor}` is a terminal editor, and a {context} has no interactive terminal; set `gui: true` to use a GUI editor")]
    NoInteractiveSurface { editor: String, context: &'static str },
    /// A terminal editor needs a terminal the host does not have.
    #[error("`{editor}` is a terminal editor and needs an interactive terminal, but {reason}")]
    NoInteractiveTerminal { editor: String, reason: &'static str },
    /// A GUI editor needs a display session.
    #[error("`{editor}` opens a window, and no display session is available")]
    NoDisplaySession { editor: String },
    /// Another terminal editor already holds the host's terminal.
    #[error("`{editor}` cannot start: another editor is using the terminal")]
    SurfaceBusy { editor: String },
    /// `create_new: true` and something already holds the name.
    #[error("{} already exists, and `create_new` requires a new file", path.display())]
    Occupied { path: PathBuf },
    /// The target is a directory or another non-regular entry.
    #[error("{} is not a regular file", path.display())]
    NotRegularFile { path: PathBuf },
    /// `edit` opens only local files and never fetches.
    #[error("`{reference}` is a URL; `edit` opens only local files")]
    UrlRefused { reference: String },
    /// The reference has no local path a file could be created at.
    #[error("file reference `{reference}` has no local target")]
    NoLocalTarget { reference: String },
    /// The file reference did not resolve.
    #[error(transparent)]
    Reference(#[from] FileReferenceError),
    /// The target is outside the mutation root (before launch or after exit).
    #[error(transparent)]
    Containment(EffectError),
    /// Creating or inspecting the target failed.
    #[error("could not {operation} {}: {source}", path.display())]
    Io {
        operation: &'static str,
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    /// Resolving, launching, or waiting for the editor failed, or the run was
    /// cancelled while it was open ([`EditorError::Cancelled`]).
    #[error(transparent)]
    Editor(#[from] EditorError),
}

impl EditError {
    /// Whether the edit ended because the run was cancelled.
    pub fn is_cancelled(&self) -> bool {
        matches!(self, Self::Editor(EditorError::Cancelled { .. }))
    }
}

/// How the host uses its standard streams, which decides what it can lend.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorOutput {
    /// The terminal is the host's own output (Claudine's text output).
    Terminal,
    /// stdout carries a Markdown document (`md compose`), so editor output
    /// goes to stderr, which must also be a terminal for a terminal editor.
    MarkdownDocument,
    /// stdout and stderr are reserved for one structured result.
    Structured,
}

/// What the host's own configuration says about its streams.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EditorHostFacts {
    pub output: EditorOutput,
    /// stdin carries the prompt or document, so it is not the user's.
    pub stdin_reserved: bool,
}

/// The production [`EditorService`]: launches the user's editor as a process.
#[derive(Debug, Clone, Copy)]
pub struct ProcessEditorService {
    pub facts: EditorHostFacts,
    pub probe: EnvironmentProbe,
}

impl ProcessEditorService {
    /// A service for `facts`, observing the current process.
    pub fn new(facts: EditorHostFacts) -> Self {
        Self { facts, probe: EnvironmentProbe::process() }
    }
}

impl EditorService for ProcessEditorService {
    fn edit(&self, request: &EditRequest<'_>) -> EditOutcome {
        let mut attempted = false;
        let mut warning = None;
        let result = self.run(request, &mut attempted, &mut warning);
        EditOutcome { attempted, result, warning }
    }
}

impl ProcessEditorService {
    fn run(
        &self,
        request: &EditRequest<'_>,
        attempted: &mut bool,
        warning: &mut Option<String>,
    ) -> Result<(), EditError> {
        if (request.cancelled)() {
            return Err(EditorError::Cancelled { path: request.target.to_path_buf() }.into());
        }
        let editor = resolve_editor(SurfaceRequest::from(request.gui))?;
        let (io, _surface) = self.eligibility(&editor)?;
        ensure_within(request.mutation_root, request.target).map_err(EditError::Containment)?;

        *attempted = true;
        let (cleaned, outcome) = create_new_guarded(request.mutation_root, request.target).map_err(creation_error)?;
        if outcome == CreateOutcome::Existing {
            if request.create_new {
                return Err(EditError::Occupied { path: cleaned });
            }
            // `metadata` follows an in-root symlink to the contained file
            // `ensure_within` already approved.
            match std::fs::metadata(&cleaned) {
                Ok(metadata) if metadata.is_file() => {}
                _ => return Err(EditError::NotRegularFile { path: cleaned }),
            }
        }

        let launch_path = biscuit_file::canonicalize_simplified(&cleaned).map_err(|source| EditError::Io {
            operation: "resolve",
            path: cleaned.clone(),
            source,
        })?;
        let report = LaunchPlan::new(editor, launch_path).run_reporting(LaunchOptions {
            io,
            own_tree: true,
            cancelled: Some(request.cancelled),
        });
        *warning = report.warning;
        report.result?;

        ensure_within(request.mutation_root, &cleaned).map_err(EditError::Containment)?;
        match std::fs::metadata(&cleaned) {
            Ok(metadata) if metadata.is_file() => Ok(()),
            Ok(_) => Err(EditError::NotRegularFile { path: cleaned }),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                Err(EditorError::Missing { path: cleaned }.into())
            }
            Err(source) => Err(EditError::Io {
                operation: "inspect",
                path: cleaned,
                source,
            }),
        }
    }

    /// The streams the selected editor gets, and, for a terminal editor, the
    /// claim on the host's terminal it holds until it exits.
    fn eligibility(&self, editor: &ResolvedEditor) -> Result<(LaunchIo, Option<InteractiveSurface>), EditError> {
        let name = || editor.display.clone();
        let structured = self.facts.output == EditorOutput::Structured;

        if !editor.uses_terminal() {
            if self.probe.display == DisplaySession::Unavailable {
                return Err(EditError::NoDisplaySession { editor: name() });
            }
            let output = if structured { LaunchOutput::Discard } else { LaunchOutput::ToHostStderr };
            let error_output = if structured { LaunchOutput::Discard } else { LaunchOutput::Inherit };
            return Ok((
                LaunchIo { stdin: LaunchStdin::Null, stdout: output, stderr: error_output },
                None,
            ));
        }

        if structured {
            return Err(EditError::StructuredOutputRefusesTerminalEditor { editor: name() });
        }
        if terminal_denied() {
            return Err(EditError::NoInteractiveSurface { editor: name(), context: "parallel group member" });
        }
        let markdown = self.facts.output == EditorOutput::MarkdownDocument;
        let missing = if self.facts.stdin_reserved {
            Some("stdin carries the prompt or document")
        } else if !self.probe.stdin_tty {
            Some("stdin is not a terminal")
        } else if !self.probe.stdout_tty {
            Some("stdout is not a terminal")
        } else if markdown && !self.probe.stderr_tty {
            Some("stderr, where the editor's output goes, is not a terminal")
        } else if !self.probe.foreground {
            Some("the process is not in the terminal's foreground")
        } else {
            None
        };
        if let Some(reason) = missing {
            return Err(EditError::NoInteractiveTerminal { editor: name(), reason });
        }
        let surface = InteractiveSurface::claim().ok_or_else(|| EditError::SurfaceBusy { editor: name() })?;
        let stdout = if markdown { LaunchOutput::ToHostStderr } else { LaunchOutput::Inherit };
        Ok((
            LaunchIo { stdin: LaunchStdin::Inherit, stdout, stderr: LaunchOutput::Inherit },
            Some(surface),
        ))
    }
}

fn creation_error(error: EffectError) -> EditError {
    match error {
        EffectError::Io { path, source } => EditError::Io {
            operation: "create",
            path,
            source,
        },
        other => EditError::Containment(other),
    }
}
