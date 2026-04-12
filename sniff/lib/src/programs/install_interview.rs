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
    fn on_event(&mut self, event: &InstallInterviewEvent) -> Result<(), SniffInstallationError>;

    fn confirm_remote_script(&mut self, prose: &str) -> Result<bool, SniffInstallationError>;

    fn choose_retry(&mut self, prompt: &RetryPrompt)
    -> Result<RetryChoice, SniffInstallationError>;
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
    InstallCapturedOutcome, build_install_announcement, build_install_failure_status,
    build_install_success_status, build_retry_choice_prose, build_retry_quit_prose,
    execute_install_captured, get_install_command,
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
    let command = match get_install_command(&method) {
        Ok(cmd) => cmd,
        Err(e) => {
            attempted.push(method);
            return handle_failure(
                input,
                options,
                delegate,
                Some((InstallOutputStream::Stderr, e.to_string())),
                attempted,
            );
        }
    };

    delegate.on_event(&InstallInterviewEvent::Announcement {
        prose: build_install_announcement(&input.program, input.website, &method, &command),
    })?;

    let needs_consent = matches!(
        method,
        InstallationMethod::RemoteBash(_) | InstallationMethod::UvWithInstall(_)
    );
    if needs_consent && !options.install.dry_run && !options.install.approve_remote_bash {
        let warning = build_remote_script_warning(&input.program, &method);
        delegate.on_event(&InstallInterviewEvent::ConsentWarning {
            prose: warning.clone(),
        })?;
        if !delegate.confirm_remote_script(&warning)? {
            return Ok(InstallInterviewOutcome::AbortedByUser);
        }
    }

    let mut exec_opts = options.install.clone();
    if needs_consent {
        exec_opts.approve_remote_bash = true;
    }
    let outcome = execute_install_captured(&method, &exec_opts);
    attempted.push(method.clone());

    match outcome {
        InstallCapturedOutcome::SetupError(e) => handle_failure(
            input,
            options,
            delegate,
            Some((InstallOutputStream::Stderr, e.to_string())),
            attempted,
        ),
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
            handle_failure(
                input,
                options,
                delegate,
                Some((InstallOutputStream::Stderr, body)),
                attempted,
            )
        }
    }
}

fn handle_failure<D: InstallInterviewDelegate>(
    input: &InstallInterviewInput,
    options: &InstallInterviewOptions,
    delegate: &mut D,
    captured_body: Option<(InstallOutputStream, String)>,
    attempted: Vec<InstallationMethod>,
) -> Result<InstallInterviewOutcome, SniffInstallationError> {
    if let Some((stream, body)) = captured_body
        && !body.trim().is_empty()
    {
        delegate.on_event(&InstallInterviewEvent::CapturedOutput { stream, body })?;
    }
    delegate.on_event(&InstallInterviewEvent::Status {
        kind: InstallStatusKind::Error,
        text: build_install_failure_status(&input.program, input.website),
    })?;

    let alts = input.plan.retryable_alternatives(&attempted);
    if alts.is_empty() || !options.prompt_on_failure {
        return Ok(InstallInterviewOutcome::Failed { attempted });
    }

    let prompt = RetryPrompt {
        heading_prose: build_retry_quit_prose(),
        choices: alts
            .iter()
            .map(|o| RetryPromptChoice {
                label: format!("Retry with {}", o.kind.manager_name()),
                prose: build_retry_choice_prose(&o.kind),
                method: o.kind.clone(),
            })
            .collect(),
    };

    match delegate.choose_retry(&prompt)? {
        RetryChoice::Quit => Ok(InstallInterviewOutcome::AbortedByUser),
        RetryChoice::RetryWith(next) => run_attempt(input, options, delegate, next, attempted),
    }
}

fn build_remote_script_warning(program: &str, method: &InstallationMethod) -> String {
    match method {
        InstallationMethod::RemoteBash(url) => format!(
            "<yellow>Warning:</yellow> installing <b>{program}</b> will download and execute a remote shell script from <a href=\"{url}\">{url}</a>."
        ),
        InstallationMethod::UvWithInstall(_) => {
            let url = crate::programs::installer::astral_installer_url();
            format!(
                "<yellow>Warning:</yellow> installing <b>{program}</b> will bootstrap <b>uv</b> by downloading and executing a remote script from <a href=\"{url}\">{url}</a>."
            )
        }
        _ => String::new(),
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
        assert!(
            !d.events
                .iter()
                .any(|e| matches!(e, InstallInterviewEvent::CapturedOutput { .. }))
        );
    }

    fn remote_bash_plan() -> InstallInterviewInput {
        InstallInterviewInput {
            program: "Rustup".into(),
            website: "https://rustup.rs",
            plan: InstallPlan {
                program: "Rustup".into(),
                website: "https://rustup.rs",
                successful: true,
                options: vec![InstallPlanOption {
                    kind: InstallationMethod::RemoteBash("https://sh.rustup.rs"),
                    requires_sudo: false,
                    choose: true,
                    reason_type: InstallPlanReason::Selected,
                    reason: "chosen".into(),
                }],
            },
        }
    }

    #[test]
    fn denied_remote_consent_returns_aborted_by_user() {
        let input = remote_bash_plan();
        let mut opts = InstallInterviewOptions::default();
        opts.install.dry_run = false;
        opts.install.approve_remote_bash = false;
        let mut d = RecordingDelegate::new();
        d.consent_answer = false;
        let outcome = run_install_interview(&input, &opts, &mut d).unwrap();
        assert!(matches!(outcome, InstallInterviewOutcome::AbortedByUser));
        assert!(
            d.events
                .iter()
                .any(|e| matches!(e, InstallInterviewEvent::ConsentWarning { .. }))
        );
        // No status event was emitted because user aborted before execution.
        assert!(
            !d.events
                .iter()
                .any(|e| matches!(e, InstallInterviewEvent::Status { .. }))
        );
    }

    #[test]
    fn remote_bash_dry_run_skips_consent() {
        let input = remote_bash_plan();
        let mut opts = InstallInterviewOptions::default();
        opts.install.dry_run = true;
        let mut d = RecordingDelegate::new();
        d.consent_answer = false; // would deny but must not be asked
        let outcome = run_install_interview(&input, &opts, &mut d).unwrap();
        assert!(matches!(outcome, InstallInterviewOutcome::DryRun { .. }));
        assert!(
            !d.events
                .iter()
                .any(|e| matches!(e, InstallInterviewEvent::ConsentWarning { .. }))
        );
    }

    #[test]
    fn remote_bash_preapproved_skips_consent() {
        let input = remote_bash_plan();
        let mut opts = InstallInterviewOptions::default();
        opts.install.dry_run = false;
        opts.install.approve_remote_bash = true;
        let mut d = RecordingDelegate::new();
        d.consent_answer = false;
        // The method will actually execute; we don't assert on final outcome
        // because whether the command succeeds depends on the host. We only
        // assert that no ConsentWarning event was emitted.
        let _ = run_install_interview(&input, &opts, &mut d);
        assert!(
            !d.events
                .iter()
                .any(|e| matches!(e, InstallInterviewEvent::ConsentWarning { .. }))
        );
    }

    fn fake_setup_error_plan() -> InstallInterviewInput {
        InstallInterviewInput {
            program: "bad".into(),
            website: "https://example.com",
            plan: InstallPlan {
                program: "bad".into(),
                website: "https://example.com",
                successful: true,
                options: vec![
                    InstallPlanOption {
                        kind: InstallationMethod::Brew("bad;pkg"),
                        requires_sudo: false,
                        choose: true,
                        reason_type: InstallPlanReason::Selected,
                        reason: "chosen".into(),
                    },
                    InstallPlanOption {
                        kind: InstallationMethod::Cargo("goodpkg"),
                        requires_sudo: false,
                        choose: false,
                        reason_type: InstallPlanReason::LowerPriorityAlternative,
                        reason: "alternative".into(),
                    },
                ],
            },
        }
    }

    #[test]
    fn failure_with_alternatives_prompts_retry_and_loops() {
        let input = fake_setup_error_plan();
        let mut d = RecordingDelegate::new();
        d.retry_answer = Some(RetryChoice::RetryWith(InstallationMethod::Cargo("goodpkg")));

        let mut opts = InstallInterviewOptions::default();
        opts.install.dry_run = true; // second attempt succeeds as dry-run
        opts.prompt_on_failure = true;

        let outcome = run_install_interview(&input, &opts, &mut d).unwrap();
        // NOTE: first attempt hits SetupError (shell metachars) even with dry_run,
        // because build_install_command validates before honoring dry_run. That
        // drives us into handle_failure → retry → second attempt (Cargo dry-run) → success.
        assert!(matches!(outcome, InstallInterviewOutcome::DryRun { .. }));
        let error_count = d
            .events
            .iter()
            .filter(|e| {
                matches!(
                    e,
                    InstallInterviewEvent::Status {
                        kind: InstallStatusKind::Error,
                        ..
                    }
                )
            })
            .count();
        let success_count = d
            .events
            .iter()
            .filter(|e| {
                matches!(
                    e,
                    InstallInterviewEvent::Status {
                        kind: InstallStatusKind::Success,
                        ..
                    }
                )
            })
            .count();
        assert_eq!(error_count, 1);
        assert_eq!(success_count, 1);
    }

    #[test]
    fn failure_without_alternatives_returns_failed_and_does_not_prompt() {
        let input = InstallInterviewInput {
            program: "bad".into(),
            website: "https://example.com",
            plan: InstallPlan {
                program: "bad".into(),
                website: "https://example.com",
                successful: true,
                options: vec![InstallPlanOption {
                    kind: InstallationMethod::Brew("bad;pkg"),
                    requires_sudo: false,
                    choose: true,
                    reason_type: InstallPlanReason::Selected,
                    reason: "chosen".into(),
                }],
            },
        };
        let mut d = RecordingDelegate::new();
        let outcome =
            run_install_interview(&input, &InstallInterviewOptions::default(), &mut d).unwrap();
        assert!(matches!(outcome, InstallInterviewOutcome::Failed { .. }));
    }

    #[test]
    fn failure_with_prompt_disabled_does_not_call_delegate_choose_retry() {
        // If prompt_on_failure is false, we must return Failed without calling choose_retry.
        let input = fake_setup_error_plan();
        let mut d = RecordingDelegate::new();
        // If choose_retry gets called, the default returns Quit (AbortedByUser) — but
        // we want to verify we short-circuit to Failed without prompting at all.
        d.retry_answer = Some(RetryChoice::RetryWith(InstallationMethod::Cargo("goodpkg")));

        let mut opts = InstallInterviewOptions::default();
        opts.install.dry_run = true;
        opts.prompt_on_failure = false;

        let outcome = run_install_interview(&input, &opts, &mut d).unwrap();
        assert!(matches!(outcome, InstallInterviewOutcome::Failed { .. }));
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
        let outcome =
            run_install_interview(&input, &InstallInterviewOptions::default(), &mut d).unwrap();
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
            InstallInterviewEvent::ConsentWarning {
                prose: "warn".into(),
            },
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
