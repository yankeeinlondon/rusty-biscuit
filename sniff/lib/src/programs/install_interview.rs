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

use crate::programs::installer::{
    build_install_announcement, build_install_failure_status, build_install_success_status,
    execute_install_captured, get_install_command, InstallCapturedOutcome,
};

/// Runs the install interview for the given program.
///
/// Emits events through `delegate` and returns the session outcome.
///
/// ## Returns
///
/// `Ok(NotInstallable)` when the plan has no viable method, emitting an error
/// status first. Otherwise delegates to the private `run_attempt` helper.
///
/// ## Errors
///
/// Propagates any `SniffInstallationError` returned by the delegate.
pub fn run_install_interview<D: InstallInterviewDelegate>(
    input: &InstallInterviewInput,
    options: &InstallInterviewOptions,
    delegate: &mut D,
) -> Result<InstallInterviewOutcome, SniffInstallationError> {
    if !input.plan.successful {
        delegate.on_event(&InstallInterviewEvent::Status {
            kind: InstallStatusKind::Error,
            text: build_install_failure_status(&input.program, input.website),
        })?;
        return Ok(InstallInterviewOutcome::NotInstallable);
    }

    let chosen = input
        .plan
        .chosen()
        .cloned()
        .expect("successful plan has a chosen option");
    run_attempt(input, options, delegate, chosen.kind, Vec::new())
}

fn run_attempt<D: InstallInterviewDelegate>(
    input: &InstallInterviewInput,
    options: &InstallInterviewOptions,
    delegate: &mut D,
    method: InstallationMethod,
    mut attempted: Vec<InstallationMethod>,
) -> Result<InstallInterviewOutcome, SniffInstallationError> {
    let command = get_install_command(&method)?;

    delegate.on_event(&InstallInterviewEvent::Announcement {
        prose: build_install_announcement(&input.program, input.website, &method, &command),
    })?;

    // Consent gate added in Task 8.

    let outcome = execute_install_captured(&method, &options.install);
    attempted.push(method.clone());

    match outcome {
        InstallCapturedOutcome::SetupError(e) => {
            let body = e.to_string();
            if !body.trim().is_empty() {
                delegate.on_event(&InstallInterviewEvent::CapturedOutput {
                    stream: InstallOutputStream::Stderr,
                    body,
                })?;
            }
            delegate.on_event(&InstallInterviewEvent::Status {
                kind: InstallStatusKind::Error,
                text: build_install_failure_status(&input.program, input.website),
            })?;
            Ok(InstallInterviewOutcome::Failed { attempted })
        }
        InstallCapturedOutcome::Completed(r) if r.success && !r.executed => {
            delegate.on_event(&InstallInterviewEvent::Status {
                kind: InstallStatusKind::Success,
                text: build_install_success_status(&input.program, input.website),
            })?;
            Ok(InstallInterviewOutcome::DryRun { method })
        }
        InstallCapturedOutcome::Completed(r) if r.success => {
            if !r.stdout.trim().is_empty() {
                delegate.on_event(&InstallInterviewEvent::CapturedOutput {
                    stream: InstallOutputStream::Stdout,
                    body: r.stdout,
                })?;
            }
            delegate.on_event(&InstallInterviewEvent::Status {
                kind: InstallStatusKind::Success,
                text: build_install_success_status(&input.program, input.website),
            })?;
            Ok(InstallInterviewOutcome::Installed { method })
        }
        InstallCapturedOutcome::Completed(r) => {
            let body = if !r.stderr.trim().is_empty() {
                r.stderr
            } else {
                r.stdout
            };
            if !body.trim().is_empty() {
                delegate.on_event(&InstallInterviewEvent::CapturedOutput {
                    stream: InstallOutputStream::Stderr,
                    body,
                })?;
            }
            delegate.on_event(&InstallInterviewEvent::Status {
                kind: InstallStatusKind::Error,
                text: build_install_failure_status(&input.program, input.website),
            })?;
            Ok(InstallInterviewOutcome::Failed { attempted })
        }
    }
}

#[cfg(test)]
mod runner_tests {
    use super::*;
    use crate::programs::install_plan::{InstallPlan, InstallPlanOption, InstallPlanReason};

    struct RecordingDelegate {
        events: Vec<InstallInterviewEvent>,
        consent_answer: bool,
        retry_answer: Option<RetryChoice>,
    }

    impl RecordingDelegate {
        fn new() -> Self {
            Self {
                events: Vec::new(),
                consent_answer: true,
                retry_answer: None,
            }
        }
    }

    impl InstallInterviewDelegate for RecordingDelegate {
        fn on_event(&mut self, e: &InstallInterviewEvent) -> Result<(), SniffInstallationError> {
            self.events.push(e.clone());
            Ok(())
        }
        fn confirm_remote_script(&mut self, _p: &str) -> Result<bool, SniffInstallationError> {
            Ok(self.consent_answer)
        }
        fn choose_retry(
            &mut self,
            _p: &RetryPrompt,
        ) -> Result<RetryChoice, SniffInstallationError> {
            Ok(self.retry_answer.clone().unwrap_or(RetryChoice::Quit))
        }
    }

    fn brew_plan() -> InstallInterviewInput {
        InstallInterviewInput {
            program: "Ripgrep".into(),
            website: "https://github.com/BurntSushi/ripgrep",
            plan: InstallPlan {
                program: "Ripgrep".into(),
                website: "https://github.com/BurntSushi/ripgrep",
                successful: true,
                options: vec![InstallPlanOption {
                    kind: InstallationMethod::Brew("ripgrep"),
                    requires_sudo: false,
                    choose: true,
                    reason_type: InstallPlanReason::Selected,
                    reason: "chosen".into(),
                }],
            },
        }
    }

    #[test]
    fn dry_run_emits_announcement_and_success_status_without_execution() {
        let input = brew_plan();
        let mut opts = InstallInterviewOptions::default();
        opts.install.dry_run = true;
        let mut d = RecordingDelegate::new();
        let outcome = run_install_interview(&input, &opts, &mut d).unwrap();
        assert!(matches!(outcome, InstallInterviewOutcome::DryRun { .. }));
        assert!(matches!(
            d.events[0],
            InstallInterviewEvent::Announcement { .. }
        ));
        assert!(d.events.iter().any(|e| matches!(
            e,
            InstallInterviewEvent::Status {
                kind: InstallStatusKind::Success,
                ..
            }
        )));
        assert!(!d.events.iter().any(
            |e| matches!(e, InstallInterviewEvent::CapturedOutput { .. })
        ));
    }

    #[test]
    fn not_installable_plan_returns_not_installable_outcome() {
        let input = InstallInterviewInput {
            program: "nope".into(),
            website: "https://example.com",
            plan: InstallPlan {
                program: "nope".into(),
                website: "https://example.com",
                successful: false,
                options: vec![],
            },
        };
        let mut d = RecordingDelegate::new();
        let outcome = run_install_interview(&input, &InstallInterviewOptions::default(), &mut d)
            .unwrap();
        assert!(matches!(outcome, InstallInterviewOutcome::NotInstallable));
        assert!(d.events.iter().any(|e| matches!(
            e,
            InstallInterviewEvent::Status {
                kind: InstallStatusKind::Error,
                ..
            }
        )));
    }
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
