//! Starting a resolved editor on one file and waiting for it.
//!
//! [`LaunchPlan`] holds an editor resolved once and the path it opens. A plan
//! run with [`LaunchOptions::own_tree`] off behaves as `md edit` and wrapper
//! `--edit` always have: the editor shares the host's process group and the
//! host waits until it exits. With it on, the editor leads a process tree the
//! host can stop as a unit, owned by `sniff::process::ProcessTree` (its own
//! process group on Unix; on Windows a Job Object the editor is assigned to
//! before it runs), the wait observes a cancellation signal, and a terminal
//! editor is made the terminal's foreground process group for as long as it
//! runs. When the editor exits on its own the tree is released, so a GUI
//! editor window started by a waiting client stays open. The owner's rules on
//! each OS are described in `sniff/docs/topics/process-trees.md`.
//!
//! ## Terminal handoff (Unix)
//!
//! A terminal editor in its own process group can read the terminal only
//! while that group is the terminal's foreground group; otherwise the kernel
//! stops it with `SIGTTIN`. An owned terminal editor that inherits the host's
//! stdin therefore runs under `sniff::process::ForegroundHandoff`, which makes
//! its group the foreground before it executes and gives the foreground and
//! the saved terminal settings back to the host once the editor has been
//! reaped. While the editor runs, a Ctrl-C from the keyboard reaches the
//! editor, not the host.

use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::time::Duration;

use sniff::process::{ForegroundHandoff, ProcessTree, TreeConfig};

use super::{EditorError, ResolvedEditor};

/// How long a cancelled editor's process tree gets between `SIGTERM` and
/// `SIGKILL`. Unused on Windows, where the Job is terminated at once.
pub const EDITOR_TERMINATE_GRACE: Duration = Duration::from_secs(2);

/// How often the wait checks for exit and for cancellation.
const POLL_INTERVAL: Duration = Duration::from_millis(20);

/// Where the editor's stdin comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaunchStdin {
    /// The host's stdin.
    Inherit,
    /// Nothing; reading gets end of file at once.
    Null,
}

/// Where one of the editor's output streams goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaunchOutput {
    /// The host's same stream.
    Inherit,
    /// The host's stderr, so nothing reaches a stdout that carries a
    /// document or a structured result.
    ToHostStderr,
    /// Nowhere.
    Discard,
}

/// The editor's standard streams.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LaunchIo {
    pub stdin: LaunchStdin,
    pub stdout: LaunchOutput,
    pub stderr: LaunchOutput,
}

impl LaunchIo {
    /// Every stream inherited from the host.
    pub const INHERIT: Self = Self {
        stdin: LaunchStdin::Inherit,
        stdout: LaunchOutput::Inherit,
        stderr: LaunchOutput::Inherit,
    };
}

/// How a [`LaunchPlan`] runs.
#[derive(Clone, Copy)]
pub struct LaunchOptions<'a> {
    pub io: LaunchIo,
    /// Start the editor as the root of a process tree that cancellation
    /// stops as a unit, and hand a terminal editor the terminal. The tree is
    /// released, not stopped, when the editor exits on its own.
    pub own_tree: bool,
    /// Polled while the editor runs; `true` terminates it and the run returns
    /// [`EditorError::Cancelled`]. `None` waits until the editor exits.
    pub cancelled: Option<&'a dyn Fn() -> bool>,
}

impl LaunchOptions<'_> {
    /// `md edit` and wrapper `--edit`: inherited streams, the host's process
    /// group, and an uninterruptible wait.
    pub const PLAIN: LaunchOptions<'static> = LaunchOptions {
        io: LaunchIo::INHERIT,
        own_tree: false,
        cancelled: None,
    };
}

/// A resolved editor and the file it opens.
#[derive(Debug, Clone)]
pub struct LaunchPlan {
    editor: ResolvedEditor,
    path: PathBuf,
}

impl LaunchPlan {
    /// The editor receives `path` as one argument after its own arguments.
    /// The caller passes an absolute path, so a name beginning with a dash
    /// cannot be read as an option.
    pub fn new(editor: ResolvedEditor, path: impl Into<PathBuf>) -> Self {
        Self { editor, path: path.into() }
    }

    #[allow(missing_docs)]
    pub fn editor(&self) -> &ResolvedEditor {
        &self.editor
    }

    #[allow(missing_docs)]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Starts the editor and waits for it.
    ///
    /// Any warning the run produces is dropped; a host with a warning channel
    /// uses [`LaunchPlan::run_reporting`]. A warning arises only when
    /// [`LaunchOptions::own_tree`] is on, which `md edit` and wrapper `--edit`
    /// never set.
    ///
    /// ## Errors
    ///
    /// - [`EditorError::LaunchFailed`]: the program could not be started.
    /// - [`EditorError::NonZeroExit`]: it exited unsuccessfully (a signal is
    ///   code `-1`).
    /// - [`EditorError::Cancelled`]: `cancelled` returned `true`; the tree was
    ///   terminated and the root reaped before returning.
    /// - [`EditorError::Io`]: the process tree could not be set up or the
    ///   wait failed (the editor was stopped), or the terminal could not be
    ///   handed to an owned terminal editor (`prepare the terminal`; another
    ///   handoff in this process is [`std::io::ErrorKind::WouldBlock`]) or
    ///   given back after an edit that otherwise succeeded (`restore the
    ///   terminal`). A restoration failure after another failure is logged,
    ///   and that other failure is returned.
    pub fn run(self, options: LaunchOptions<'_>) -> Result<(), EditorError> {
        self.run_reporting(options).result
    }

    /// Starts the editor and waits for it, like [`LaunchPlan::run`], and also
    /// returns any warning for the user.
    ///
    /// A warning does not change the result: on Windows, when an owned
    /// editor's tree cannot be released after the editor exited, the tree is
    /// terminated (closing a GUI editor window it started), the edit still
    /// counts, and the warning says what happened.
    pub fn run_reporting(self, options: LaunchOptions<'_>) -> LaunchReport {
        let mut warning = None;
        let result = self.launch(options, &mut warning);
        LaunchReport { result, warning }
    }

    fn launch(self, options: LaunchOptions<'_>, warning: &mut Option<String>) -> Result<(), EditorError> {
        let mut command = Command::new(&self.editor.program);
        command.args(&self.editor.args).arg(&self.path);
        command
            .stdin(match options.io.stdin {
                LaunchStdin::Inherit => Stdio::inherit(),
                LaunchStdin::Null => Stdio::null(),
            })
            .stdout(output_stdio(options.io.stdout))
            .stderr(output_stdio(options.io.stderr));

        // Declared before the tree, so an unwind stops the tree before the
        // terminal goes back to the host.
        let mut handoff = if options.own_tree {
            // Before the handoff: its `pre_exec` hook must run once the child
            // already leads its own group.
            TreeConfig::process_group().configure(&mut command);
            if self.editor.uses_terminal() && options.io.stdin == LaunchStdin::Inherit {
                let prepared = ForegroundHandoff::prepare(&mut command).map_err(|source| EditorError::Io {
                    operation: "prepare the terminal",
                    source,
                })?;
                Some(prepared)
            } else {
                None
            }
        } else {
            None
        };

        let mut child = match command.spawn() {
            Ok(child) => child,
            Err(source) => {
                let failed = EditorError::LaunchFailed {
                    editor: self.editor.display.clone(),
                    full_command: self.editor.full_command(),
                    source,
                };
                return with_restoration(Err(failed), restore(&mut handoff));
            }
        };

        let mut tree = if options.own_tree {
            match ProcessTree::attach(&child) {
                Ok(tree) => Some(tree),
                Err(source) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    let failed = EditorError::Io {
                        operation: "contain the editor's process tree",
                        source,
                    };
                    return with_restoration(Err(failed), restore(&mut handoff));
                }
            }
        } else {
            None
        };

        let waited = wait(&mut child, tree.as_mut(), options.cancelled);
        let outcome = match waited {
            Waited::Exited(status) => {
                if let Some(tree) = tree
                    && let Err(error) = tree.release()
                {
                    // A GUI client may have started the editor's main window;
                    // that window is the user's and should outlive the wait.
                    // When Windows cannot clear the Job's kill-on-close limit,
                    // the dropped owner closes it instead; the edit itself
                    // already happened.
                    *warning = Some(format!(
                        "Windows could not keep the editor running after the edit; it was closed: {error}"
                    ));
                }
                Ok(status)
            }
            Waited::Cancelled | Waited::Failed(_) => {
                match tree.as_mut() {
                    Some(tree) => {
                        let termination = tree.terminate_gracefully(EDITOR_TERMINATE_GRACE, &mut child);
                        if let Err(error) = termination.cleanup {
                            tracing::warn!(error = %error, "could not stop the editor's whole process tree");
                        }
                    }
                    None => {
                        let _ = child.kill();
                        let _ = child.wait();
                    }
                }
                match waited {
                    Waited::Failed(source) => Err(EditorError::Io {
                        operation: "wait for the editor",
                        source,
                    }),
                    _ => Err(EditorError::Cancelled { path: self.path.clone() }),
                }
            }
        };
        let result = outcome.and_then(|status| {
            if status.success() {
                Ok(())
            } else {
                Err(EditorError::NonZeroExit {
                    code: status.code().unwrap_or(-1),
                    editor: self.editor.display,
                    path: self.path,
                })
            }
        });
        // The terminal goes back to the host only after the root is reaped.
        with_restoration(result, restore(&mut handoff))
    }
}

/// Gives the terminal back to the host; nothing to do without a handoff.
fn restore(handoff: &mut Option<ForegroundHandoff>) -> std::io::Result<()> {
    handoff.as_mut().map_or(Ok(()), ForegroundHandoff::restore)
}

/// The launch's result once the terminal has been given back. A restoration
/// failure fails an edit that otherwise succeeded; after an earlier failure
/// it is only logged, so the first failure stays the one reported. The
/// handoff's `Drop` still retries the restoration once.
fn with_restoration(result: Result<(), EditorError>, restored: std::io::Result<()>) -> Result<(), EditorError> {
    match (result, restored) {
        (result, Ok(())) => result,
        (Ok(()), Err(source)) => Err(EditorError::Io {
            operation: "restore the terminal",
            source,
        }),
        (Err(error), Err(restoration)) => {
            tracing::warn!(error = %restoration, "could not restore the terminal after the editor");
            Err(error)
        }
    }
}

/// What [`LaunchPlan::run_reporting`] observed.
#[derive(Debug)]
pub struct LaunchReport {
    /// The edit's outcome, exactly as [`LaunchPlan::run`] returns it.
    pub result: Result<(), EditorError>,
    /// Something the host should tell the user even though `result` stands.
    pub warning: Option<String>,
}

enum Waited {
    Exited(ExitStatus),
    Cancelled,
    Failed(std::io::Error),
}

/// Waits for the editor. An owned tree needs a polling wait so its
/// descendants are sampled even when there is no cancellation callback; only
/// an unowned launch with no callback blocks in `Child::wait`.
fn wait(
    child: &mut std::process::Child,
    mut tree: Option<&mut ProcessTree>,
    cancelled: Option<&dyn Fn() -> bool>,
) -> Waited {
    if tree.is_none() && cancelled.is_none() {
        return match child.wait() {
            Ok(status) => Waited::Exited(status),
            Err(error) => Waited::Failed(error),
        };
    }
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Waited::Exited(status),
            Ok(None) => {
                if let Some(tree) = tree.as_deref_mut() {
                    tree.sample();
                }
                if cancelled.is_some_and(|cancelled| cancelled()) {
                    return Waited::Cancelled;
                }
                std::thread::sleep(POLL_INTERVAL);
            }
            Err(error) => return Waited::Failed(error),
        }
    }
}

fn output_stdio(output: LaunchOutput) -> Stdio {
    match output {
        LaunchOutput::Inherit => Stdio::inherit(),
        LaunchOutput::Discard => Stdio::null(),
        // A host with no stderr to duplicate has nowhere to show the output;
        // discarding still keeps it off stdout.
        LaunchOutput::ToHostStderr => host_stderr().unwrap_or_else(|_| Stdio::null()),
    }
}

#[cfg(unix)]
fn host_stderr() -> std::io::Result<Stdio> {
    use std::os::fd::AsFd;
    Ok(Stdio::from(std::io::stderr().as_fd().try_clone_to_owned()?))
}

#[cfg(windows)]
fn host_stderr() -> std::io::Result<Stdio> {
    use std::os::windows::io::AsHandle;
    Ok(Stdio::from(std::io::stderr().as_handle().try_clone_to_owned()?))
}

#[cfg(not(any(unix, windows)))]
fn host_stderr() -> std::io::Result<Stdio> {
    Ok(Stdio::inherit())
}

#[cfg(test)]
mod tests;
