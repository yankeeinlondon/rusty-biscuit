//! Shared install-interview event and delegate types.
//!
//! The library owns sequencing, command execution, and copy strings. The
//! caller (e.g. the sniff CLI) supplies a delegate that decides how to
//! render each event and how to handle interactive prompts. This avoids a
//! circular dependency on `biscuit-terminal`.
//!
//! See `sniff/features/2026-04-12-better-interview-for-install/tech-design.md`.
//!
//! The runner function `run_install_interview` is added in a follow-up task.

use crate::error::SniffInstallationError;
use crate::programs::install_plan::InstallPlan;
use crate::programs::installer::InstallOptions;
use crate::programs::types::InstallationMethod;

/// Semantic interview events emitted by the runner.
///
/// Each variant carries a caller-renderable string; the caller decides
/// which concrete component to wrap around it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstallInterviewEvent {
    /// Pre-execution announcement (renders as `Prose`).
    Announcement { prose: String },
    /// Warning before a remote-script install (renders as `Prose`).
    ConsentWarning { prose: String },
    /// Captured program output (renders as `BlockQuote`). The body is raw
    /// text without prose markup.
    CapturedOutput {
        stream: InstallOutputStream,
        body: String,
    },
    /// Terminal success/error status (renders as `Status`).
    Status {
        kind: InstallStatusKind,
        text: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallOutputStream {
    Stdout,
    Stderr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallStatusKind {
    Success,
    Error,
}

/// Input to a single interview session.
#[derive(Debug, Clone)]
pub struct InstallInterviewInput {
    pub program: String,
    pub website: &'static str,
    pub plan: InstallPlan,
}

/// Options controlling the interview runner.
#[derive(Debug, Clone)]
pub struct InstallInterviewOptions {
    pub install: InstallOptions,
    /// When `true` (default for CLI interactive flows), the runner asks the
    /// delegate whether to retry after a failed attempt. Silent callers set
    /// this to `false`.
    pub prompt_on_failure: bool,
}

impl Default for InstallInterviewOptions {
    fn default() -> Self {
        Self {
            install: InstallOptions::default(),
            prompt_on_failure: true,
        }
    }
}

/// Retry prompt payload handed to the delegate.
#[derive(Debug, Clone)]
pub struct RetryPrompt {
    /// Optional heading prose the delegate may render before prompting.
    pub heading_prose: String,
    pub choices: Vec<RetryPromptChoice>,
}

#[derive(Debug, Clone)]
pub struct RetryPromptChoice {
    /// Plain-string label suitable for `inquire::Select`.
    pub label: String,
    /// Rich prose line the delegate may render before prompting.
    pub prose: String,
    /// The method this choice would retry with.
    pub method: InstallationMethod,
}

/// Delegate's decision on a retry prompt.
#[derive(Debug, Clone)]
pub enum RetryChoice {
    RetryWith(InstallationMethod),
    Quit,
}

/// Caller-provided adapter for rendering events and handling prompts.
pub trait InstallInterviewDelegate {
    fn on_event(
        &mut self,
        event: &InstallInterviewEvent,
    ) -> Result<(), SniffInstallationError>;

    fn confirm_remote_script(
        &mut self,
        prose: &str,
    ) -> Result<bool, SniffInstallationError>;

    fn choose_retry(
        &mut self,
        prompt: &RetryPrompt,
    ) -> Result<RetryChoice, SniffInstallationError>;
}

/// Final outcome of an interview session.
#[derive(Debug, Clone)]
pub enum InstallInterviewOutcome {
    Installed { method: InstallationMethod },
    DryRun { method: InstallationMethod },
    AbortedByUser,
    Failed { attempted: Vec<InstallationMethod> },
    NotInstallable,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_variants_carry_expected_fields() {
        let events = [
            InstallInterviewEvent::Announcement { prose: "hi".into() },
            InstallInterviewEvent::ConsentWarning { prose: "warn".into() },
            InstallInterviewEvent::CapturedOutput {
                stream: InstallOutputStream::Stdout,
                body: "out".into(),
            },
            InstallInterviewEvent::Status {
                kind: InstallStatusKind::Success,
                text: "ok".into(),
            },
        ];
        for e in events {
            match e {
                InstallInterviewEvent::Announcement { .. }
                | InstallInterviewEvent::ConsentWarning { .. }
                | InstallInterviewEvent::CapturedOutput { .. }
                | InstallInterviewEvent::Status { .. } => {}
            }
        }
    }

    #[test]
    fn interview_options_default_prompts_on_failure() {
        let opts = InstallInterviewOptions::default();
        assert!(opts.prompt_on_failure);
        assert!(!opts.install.dry_run);
    }
}
