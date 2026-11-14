//! Starting a resolved editor on one file and waiting for it.
//!
//! [`LaunchPlan`] holds an editor resolved once and the path it opens. A plan
//! run with [`LaunchOptions::own_tree`] off behaves as `md edit` and wrapper
//! `--edit` always have: the editor shares the host's process group and the
//! host waits until it exits. With it on, the editor leads a process tree the
//! host can stop as a unit (its own process group on Unix, a Job Object on
//! Windows), the wait observes a cancellation signal, and a terminal editor is
//! made the terminal's foreground process group for as long as it runs.
//!
//! ## Terminal handoff (Unix)
//!
//! A terminal editor in its own process group can read the terminal only
//! while that group is the terminal's foreground group; otherwise the kernel
//! stops it with `SIGTTIN`. The child makes itself the foreground group
//! between `fork` and `exec`, so it owns the terminal before it reads
//! anything. Once the editor has exited (or been terminated) the host takes
//! the foreground back and restores the terminal settings it saved before the
//! launch, ignoring `SIGTTOU` while it does so because it is, at that moment,
//! a background process writing terminal state. While the editor runs, a
//! Ctrl-C from the keyboard reaches the editor, not the host.

use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::time::Duration;

use super::{EditorError, ResolvedEditor};
use crate::process_tree::ProcessTree;

/// How long a terminated editor's process group gets between `SIGTERM` and
/// `SIGKILL`.
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
    /// stops as a unit, and hand a terminal editor the terminal.
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
    /// ## Errors
    ///
    /// - [`EditorError::LaunchFailed`]: the program could not be started.
    /// - [`EditorError::NonZeroExit`]: it exited unsuccessfully (a signal is
    ///   code `-1`).
    /// - [`EditorError::Cancelled`]: `cancelled` returned `true`; the tree was
    ///   terminated and the root reaped before returning.
    /// - [`EditorError::Io`]: the process tree could not be set up or the
    ///   wait failed; the editor was stopped.
    pub fn run(self, options: LaunchOptions<'_>) -> Result<(), EditorError> {
        let mut command = Command::new(&self.editor.program);
        command.args(&self.editor.args).arg(&self.path);
        command
            .stdin(match options.io.stdin {
                LaunchStdin::Inherit => Stdio::inherit(),
                LaunchStdin::Null => Stdio::null(),
            })
            .stdout(output_stdio(options.io.stdout))
            .stderr(output_stdio(options.io.stderr));

        let handoff = if options.own_tree {
            own_process_group(&mut command);
            if self.editor.uses_terminal() && options.io.stdin == LaunchStdin::Inherit {
                terminal::Handoff::prepare(&mut command)
            } else {
                None
            }
        } else {
            None
        };

        let mut child = command.spawn().map_err(|source| EditorError::LaunchFailed {
            editor: self.editor.display.clone(),
            full_command: self.editor.full_command(),
            source,
        })?;

        let tree = if options.own_tree {
            // Windows: a process the editor starts before this assignment
            // escapes the Job and survives cancellation; that window is
            // accepted. Unix: the group exists from `exec`, so nothing escapes
            // except a descendant that changes its own group or session.
            match ProcessTree::attach(&child) {
                Some(tree) => Some(tree),
                None => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(EditorError::Io {
                        operation: "contain the editor's process tree",
                        source: std::io::Error::other("the editor could not be placed in its own process tree"),
                    });
                }
            }
        } else {
            None
        };

        let waited = wait(&mut child, options.cancelled);
        let outcome = match waited {
            Waited::Exited(status) => {
                if let Some(tree) = tree {
                    // A GUI client may have started the editor's main window;
                    // that window is the user's and outlives the wait.
                    tree.release();
                }
                Ok(status)
            }
            Waited::Cancelled | Waited::Failed(_) => {
                match &tree {
                    Some(tree) => tree.terminate_gracefully(EDITOR_TERMINATE_GRACE, &mut child),
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
        // The terminal goes back to the host only after the root is reaped.
        drop(handoff);

        let status = outcome?;
        if !status.success() {
            return Err(EditorError::NonZeroExit {
                code: status.code().unwrap_or(-1),
                editor: self.editor.display,
                path: self.path,
            });
        }
        Ok(())
    }
}

enum Waited {
    Exited(ExitStatus),
    Cancelled,
    Failed(std::io::Error),
}

fn wait(child: &mut std::process::Child, cancelled: Option<&dyn Fn() -> bool>) -> Waited {
    let Some(cancelled) = cancelled else {
        return match child.wait() {
            Ok(status) => Waited::Exited(status),
            Err(error) => Waited::Failed(error),
        };
    };
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Waited::Exited(status),
            Ok(None) if cancelled() => return Waited::Cancelled,
            Ok(None) => std::thread::sleep(POLL_INTERVAL),
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

#[cfg(unix)]
fn own_process_group(command: &mut Command) {
    use std::os::unix::process::CommandExt;
    command.process_group(0);
}

#[cfg(not(unix))]
fn own_process_group(_command: &mut Command) {}

#[cfg(unix)]
mod terminal {
    use std::os::unix::process::CommandExt;
    use std::process::Command;

    /// The host's terminal state, restored when this is dropped.
    pub(super) struct Handoff {
        host_group: libc::pid_t,
        settings: libc::termios,
    }

    impl Handoff {
        /// Arranges for the child to take the terminal's foreground, when
        /// stdin is a terminal whose foreground group is the host's. `None`
        /// changes nothing: a background host must not take the terminal from
        /// the group that has it.
        pub(super) fn prepare(command: &mut Command) -> Option<Self> {
            // SAFETY: plain queries on fd 0 and the calling process; `termios`
            // is plain data that `tcgetattr` fills completely on success.
            let (host_group, settings) = unsafe {
                if libc::isatty(libc::STDIN_FILENO) != 1 {
                    return None;
                }
                let host_group = libc::getpgrp();
                if libc::tcgetpgrp(libc::STDIN_FILENO) != host_group {
                    return None;
                }
                let mut settings: libc::termios = std::mem::zeroed();
                if libc::tcgetattr(libc::STDIN_FILENO, &mut settings) != 0 {
                    return None;
                }
                (host_group, settings)
            };

            // SAFETY: `sigemptyset`/`sigaddset` only fill this local set.
            let ttou = unsafe {
                let mut set: libc::sigset_t = std::mem::zeroed();
                libc::sigemptyset(&mut set);
                libc::sigaddset(&mut set, libc::SIGTTOU);
                set
            };
            // SAFETY: the closure runs in the forked child, after std has
            // applied `process_group(0)` and reset the signal mask, and before
            // `exec`. It calls only async-signal-safe functions (`sigprocmask`,
            // `tcsetpgrp`, `getpgrp`) on a copied signal set; it allocates and
            // locks nothing. Blocking `SIGTTOU` lets the new background group
            // take the foreground without being stopped; the previous mask is
            // restored before `exec`.
            unsafe {
                command.pre_exec(move || {
                    let mut previous: libc::sigset_t = std::mem::zeroed();
                    libc::sigprocmask(libc::SIG_BLOCK, &ttou, &mut previous);
                    let result = libc::tcsetpgrp(libc::STDIN_FILENO, libc::getpgrp());
                    let error = std::io::Error::last_os_error();
                    libc::sigprocmask(libc::SIG_SETMASK, &previous, std::ptr::null_mut());
                    if result != 0 {
                        return Err(error);
                    }
                    Ok(())
                });
            }
            Some(Self { host_group, settings })
        }
    }

    impl Drop for Handoff {
        fn drop(&mut self) {
            // SAFETY: `sigaction` with valid pointers to local structs;
            // `tcsetpgrp`/`tcsetattr` on fd 0 with the group and settings saved
            // in `prepare`. The host is in the background here, so `SIGTTOU`
            // is ignored for the two calls and its previous action restored.
            unsafe {
                let mut ignore: libc::sigaction = std::mem::zeroed();
                ignore.sa_sigaction = libc::SIG_IGN;
                libc::sigemptyset(&mut ignore.sa_mask);
                let mut previous: libc::sigaction = std::mem::zeroed();
                libc::sigaction(libc::SIGTTOU, &ignore, &mut previous);
                libc::tcsetpgrp(libc::STDIN_FILENO, self.host_group);
                libc::tcsetattr(libc::STDIN_FILENO, libc::TCSADRAIN, &self.settings);
                libc::sigaction(libc::SIGTTOU, &previous, std::ptr::null_mut());
            }
        }
    }
}

#[cfg(not(unix))]
mod terminal {
    /// Windows shares one console among its processes, so there is no
    /// foreground group to hand over; there is never a handoff to restore.
    pub(super) enum Handoff {}

    impl Handoff {
        pub(super) fn prepare(_command: &mut std::process::Command) -> Option<Self> {
            None
        }
    }
}
