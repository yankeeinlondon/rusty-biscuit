//! Options and result types for program installation.
//!
//! Leaf module: depends only on the error crate. Used by `command`,
//! `execute`, `plan`, and `interview`.

use crate::error::SniffInstallationError;

/// Default timeout for installation commands (30 seconds).
pub(super) const DEFAULT_TIMEOUT_SECS: u64 = 30;

/// Options for program installation.
#[derive(Debug, Clone)]
pub struct InstallOptions {
    /// Show command without executing.
    pub dry_run: bool,
    /// Skip user confirmation prompt.
    pub skip_confirm: bool,
    /// Timeout in seconds for the installation command.
    pub timeout_secs: u64,
    /// Whether the caller has explicitly approved executing a RemoteBash method.
    ///
    /// Defaults to `false`. The plan executor returns
    /// `SniffInstallationError::RemoteBashConsentRequired` if the selected
    /// option is `RemoteBash` and this flag is `false`.
    pub approve_remote_bash: bool,
}

impl Default for InstallOptions {
    fn default() -> Self {
        Self {
            dry_run: false,
            skip_confirm: false,
            timeout_secs: DEFAULT_TIMEOUT_SECS,
            approve_remote_bash: false,
        }
    }
}

impl InstallOptions {
    /// Creates options for a dry-run (no execution).
    pub fn dry_run() -> Self {
        Self {
            dry_run: true,
            ..Default::default()
        }
    }

    /// Creates options that skip confirmation (for automated use).
    ///
    /// ## Warning
    ///
    /// Use with caution - this will execute commands without user confirmation.
    pub fn auto_confirm() -> Self {
        Self {
            skip_confirm: true,
            ..Default::default()
        }
    }

    /// Sets the timeout for the installation command.
    pub fn with_timeout(mut self, secs: u64) -> Self {
        self.timeout_secs = secs;
        self
    }

    /// Sets whether RemoteBash execution is pre-approved.
    pub fn with_approve_remote_bash(mut self, approve: bool) -> Self {
        self.approve_remote_bash = approve;
        self
    }
}

/// Result of an installation attempt.
#[derive(Debug)]
pub struct InstallResult {
    /// The command that was (or would be) executed.
    pub command: String,
    /// Whether the command was actually executed (false for dry-run).
    pub executed: bool,
    /// Exit code if executed (None for dry-run).
    pub exit_code: Option<i32>,
    /// stdout output if executed.
    pub stdout: String,
    /// stderr output if executed.
    pub stderr: String,
}

/// Captured outcome of an install attempt, preserving stdout/stderr on both
/// success and non-zero-exit failures so the interview layer can render
/// structured output. See
/// `sniff/features/2026-04-12-better-interview-for-install/tech-design.md`.
#[derive(Debug, Clone)]
pub struct InstallCapturedResult {
    pub command: String,
    pub executed: bool,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub success: bool,
    /// The command was killed at its deadline rather than exiting on its own.
    ///
    /// Termination is best-effort on Unix: the installer's process tree was
    /// signaled, but a descendant that forked and detached with `setsid()`
    /// between sniff's samples survives and may still be modifying the host.
    /// See the `process` module documentation for the exact guarantee per
    /// platform. Windows containment is kernel-enforced and total.
    pub timed_out: bool,
}

/// Two-arm outcome from a captured install run.
///
/// `Completed` covers dry-run, success, and non-zero exit (including spawn
/// failures folded into `stderr`). `SetupError` is reserved for invalid
/// inputs where no command could meaningfully run.
#[derive(Debug)]
pub enum InstallCapturedOutcome {
    Completed(InstallCapturedResult),
    SetupError(SniffInstallationError),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_install_options_defaults() {
        let opts = InstallOptions::default();
        assert!(!opts.dry_run);
        assert!(!opts.skip_confirm);
        assert_eq!(opts.timeout_secs, DEFAULT_TIMEOUT_SECS);
    }

    #[test]
    fn test_install_options_default_does_not_approve_remote_bash() {
        let opts = InstallOptions::default();
        assert!(!opts.approve_remote_bash);
    }

    #[test]
    fn test_install_options_with_approve_remote_bash_sets_flag() {
        let opts = InstallOptions::default().with_approve_remote_bash(true);
        assert!(opts.approve_remote_bash);
    }

    #[test]
    fn install_captured_outcome_completed_has_command_and_streams() {
        let ok = InstallCapturedResult {
            command: "brew install rg".into(),
            executed: true,
            exit_code: Some(0),
            stdout: "ok\n".into(),
            stderr: String::new(),
            success: true,
            timed_out: false,
        };
        let outcome = InstallCapturedOutcome::Completed(ok);
        match outcome {
            InstallCapturedOutcome::Completed(r) => {
                assert_eq!(r.command, "brew install rg");
                assert!(r.success);
            }
            _ => panic!("expected Completed"),
        }
    }
}
