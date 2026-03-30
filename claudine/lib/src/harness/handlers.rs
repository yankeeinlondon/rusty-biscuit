//! Handler resolution for the harness execution loop.
//!
//! Resolves which recovery action to take when a failure occurs, following
//! the precedence order: subject-specific YAML handler > generic YAML handler
//! > programmatic `handle` > unhandled.

use std::path::{Path, PathBuf};

use crate::harness::error::HarnessError;
use crate::harness::model::{
    ApprovedRuntimeCommand, AttemptOutcome, FailureCheck, FailureEvent, FailurePhase,
    HandlerAction, HandlerTable, ProcessTermination, ValidationFailure,
};

/// Context describing a failure event for handler resolution.
#[derive(Debug, Clone)]
pub struct FailureContext {
    /// Provider name for the failed attempt.
    pub provider: String,
    /// Absolute source document path, if available.
    pub source_file: Option<PathBuf>,
    /// The normalized failure event.
    pub event: FailureEvent,
    /// Which phase reported the failure.
    pub failure_phase: FailurePhase,
    /// Validation-specific details, when applicable.
    pub check: Option<FailureCheck>,
    /// Subject key for subject-specific matching (e.g. a file path).
    pub subject_key: Option<String>,
    /// Human-readable failure message.
    pub message: String,
    /// Current attempt number (1-indexed).
    pub attempt: u32,
    /// Session ID from the most recent provider run, if available.
    pub session_id: Option<String>,
    /// The attempt outcome, if available.
    pub outcome: Option<AttemptOutcome>,
}

/// Resolve a handler for the given failure context.
///
/// Resolution order (per tech design):
/// 1. Subject-specific YAML handler for the failing event
/// 2. Generic YAML handler for the failing event
/// 3. Programmatic `handle` (deferred to Phase 7, returns `None` here)
/// 4. `None` (unhandled)
pub fn resolve_handler(
    failure: &FailureContext,
    table: &HandlerTable,
    programmatic: Option<&ApprovedRuntimeCommand>,
) -> Result<Option<HandlerAction>, HarnessError> {
    // 1. Subject-specific handler: match by event + subject_key
    if let Some(ref subject_key) = failure.subject_key {
        for rule in &table.exact {
            if rule.event == failure.event
                && rule.subject_key.as_deref() == Some(subject_key.as_str())
            {
                return Ok(Some(rule.action.clone()));
            }
        }
    }

    // 2. Generic handler: match by event only
    for rule in &table.generic {
        if rule.event == failure.event {
            return Ok(Some(rule.action.clone()));
        }
    }

    // 3. Programmatic handler
    if let Some(cmd) = programmatic {
        match execute_programmatic_handler(cmd, failure) {
            Ok(Some(action)) => return Ok(Some(action)),
            Ok(None) => {} // Handler returned "unhandled"
            Err(e) => return Err(e),
        }
    }

    // 4. Unhandled
    Ok(None)
}

/// Map `ProcessTermination` and `AttemptOutcome` to a `FailureEvent`.
pub fn classify_failure(outcome: &AttemptOutcome) -> Option<FailureEvent> {
    match outcome.termination {
        ProcessTermination::TimedOut => Some(FailureEvent::Timeout),
        ProcessTermination::Interrupted => None, // User canceled, no handler
        ProcessTermination::LaunchFailed => Some(FailureEvent::AgentFailure),
        ProcessTermination::Completed => {
            if outcome.exit_code != 0 {
                Some(FailureEvent::AgentFailure)
            } else {
                None // Success, no failure event
            }
        }
    }
}

/// Build a `FailureContext` from validation failures.
pub fn build_validation_failure_context(
    failures: &[ValidationFailure],
    provider: &str,
    source_file: &Path,
    attempt: u32,
    session_id: Option<String>,
    outcome: Option<AttemptOutcome>,
) -> Vec<FailureContext> {
    failures
        .iter()
        .map(|f| FailureContext {
            provider: provider.to_string(),
            source_file: Some(source_file.to_path_buf()),
            event: FailureEvent::Validation(f.event.clone()),
            failure_phase: f.phase,
            check: Some(FailureCheck {
                name: f.event.clone(),
                subject_key: f.subject_key.clone(),
            }),
            subject_key: f.subject_key.clone(),
            message: f.message.clone(),
            attempt,
            session_id: session_id.clone(),
            outcome: outcome.clone(),
        })
        .collect()
}

/// Execute a programmatic `handle` command with failure context on stdin.
///
/// The command receives a JSON payload on stdin with all failure context.
/// Its stdout response is parsed to determine the handler action:
/// - Empty / `null` / `false` -> unhandled (returns `Ok(None)`)
/// - `true` -> `HandlerAction::Retry` with defaults
/// - JSON object -> parsed as `HandlerAction`
/// - `deviate` in response -> rejected (cannot pre-screen runtime commands)
fn execute_programmatic_handler(
    command: &ApprovedRuntimeCommand,
    failure: &FailureContext,
) -> Result<Option<HandlerAction>, HarnessError> {
    use std::io::Write;
    use std::process::{Command, Stdio};

    let termination = failure
        .outcome
        .as_ref()
        .map(|outcome| outcome.termination.to_string());
    let response = failure
        .outcome
        .as_ref()
        .filter(|outcome| !outcome.final_response.is_empty())
        .map(|outcome| serde_json::json!({ "text": outcome.final_response }));
    let check = failure.check.as_ref().map(|check| {
        serde_json::json!({
            "name": check.name.to_string(),
            "subject_key": check.subject_key,
        })
    });

    // Build JSON payload
    let payload = serde_json::json!({
        "provider": failure.provider,
        "source_file": failure.source_file.as_ref().map(|path| path.display().to_string()),
        "attempt": failure.attempt,
        "session_id": failure.session_id,
        "termination": termination,
        "failure_event": failure.event.to_string(),
        "failure_phase": failure.failure_phase.to_string(),
        "message": failure.message,
        "check": check,
        "response": response,
    });

    let exe = which::which(&command.executable).map_err(|_| HarnessError::HandlerFailed {
        action: "handle".to_string(),
        detail: format!("executable '{}' not found in PATH", command.executable),
    })?;

    let mut child = Command::new(&exe)
        .args(&command.args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env("CLAUDINE_PROVIDER", &failure.provider)
        .env("CLAUDINE_ATTEMPT", failure.attempt.to_string())
        .env("CLAUDINE_FAILURE_EVENT", failure.event.to_string())
        .env("CLAUDINE_FAILURE_PHASE", failure.failure_phase.to_string())
        .env(
            "CLAUDINE_SESSION_ID",
            failure.session_id.as_deref().unwrap_or(""),
        )
        .env("CLAUDINE_TERMINATION", termination.as_deref().unwrap_or(""))
        .env(
            "CLAUDINE_SOURCE_FILE",
            failure
                .source_file
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_default(),
        )
        .spawn()
        .map_err(|e| HarnessError::HandlerFailed {
            action: "handle".to_string(),
            detail: format!("failed to spawn '{}': {e}", command.executable),
        })?;

    // Write payload to stdin
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(payload.to_string().as_bytes());
    }

    let output = child
        .wait_with_output()
        .map_err(|e| HarnessError::HandlerFailed {
            action: "handle".to_string(),
            detail: format!("failed to wait for '{}': {e}", command.executable),
        })?;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();

    // Parse response
    if stdout.is_empty() || stdout == "null" || stdout == "false" {
        return Ok(None);
    }

    if stdout == "true" {
        return Ok(Some(HandlerAction::Retry {
            prompt_suffix: None,
            set: None,
            msg: None,
            say: None,
            retries: None,
        }));
    }

    // Try to parse as JSON
    let value: serde_json::Value =
        serde_json::from_str(&stdout).map_err(|e| HarnessError::HandlerFailed {
            action: "handle".to_string(),
            detail: format!("invalid JSON response from handler: {e}"),
        })?;

    // Reject deviate from programmatic handlers
    if let Some(action_type) = value.get("action").and_then(|v| v.as_str())
        && action_type == "deviate"
    {
        return Err(HarnessError::HandlerFailed {
            action: "handle".to_string(),
            detail: "programmatic handlers cannot return 'deviate' — \
                     deviate commands must be declared in frontmatter so they \
                     can be pre-screened at parse time"
                .to_string(),
        });
    }

    parse_programmatic_response(&value)
}

/// Parse a JSON response from a programmatic handler into a `HandlerAction`.
fn parse_programmatic_response(
    value: &serde_json::Value,
) -> Result<Option<HandlerAction>, HarnessError> {
    let action_type = value
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("retry");

    match action_type {
        "retry" => Ok(Some(HandlerAction::Retry {
            prompt_suffix: value
                .get("prompt_suffix")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            set: parse_set_from_value(value),
            msg: value
                .get("msg")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            say: value
                .get("say")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            retries: value
                .get("retries")
                .and_then(|v| v.as_u64())
                .map(|n| n as u32),
        })),
        "resume" => {
            let prompt = value
                .get("prompt")
                .and_then(|v| v.as_str())
                .ok_or_else(|| HarnessError::HandlerFailed {
                    action: "handle".to_string(),
                    detail: "resume action requires a 'prompt' field".to_string(),
                })?;
            Ok(Some(HandlerAction::Resume {
                prompt: prompt.to_string(),
                set: parse_set_from_value(value),
                msg: value
                    .get("msg")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                say: value
                    .get("say")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                retries: value
                    .get("retries")
                    .and_then(|v| v.as_u64())
                    .map(|n| n as u32),
            }))
        }
        "redirect" => {
            let file = value.get("file").and_then(|v| v.as_str()).ok_or_else(|| {
                HarnessError::HandlerFailed {
                    action: "handle".to_string(),
                    detail: "redirect action requires a 'file' field".to_string(),
                }
            })?;
            Ok(Some(HandlerAction::Redirect {
                file: file.to_string(),
                set: parse_set_from_value(value),
                msg: value
                    .get("msg")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                say: value
                    .get("say")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                resume: value
                    .get("resume")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false),
            }))
        }
        other => Err(HarnessError::HandlerFailed {
            action: "handle".to_string(),
            detail: format!("unknown handler action type: '{other}'"),
        }),
    }
}

/// Extract a `set` overlay from a programmatic handler JSON response.
fn parse_set_from_value(
    value: &serde_json::Value,
) -> Option<indexmap::IndexMap<String, serde_json::Value>> {
    value
        .get("set")
        .and_then(|v| v.as_object())
        .map(|obj| obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
}

/// Execute a `deviate` handler command with attempt context environment variables.
///
/// Sets `CLAUDINE_ATTEMPT`, `CLAUDINE_SESSION_ID`, `CLAUDINE_FAILURE_EVENT`,
/// and `CLAUDINE_SOURCE_FILE` in the command's environment.
///
/// Returns `Ok(exit_code)` after execution.
pub fn execute_deviate_command(
    command: &ApprovedRuntimeCommand,
    failure: &FailureContext,
    source_file: Option<&std::path::Path>,
) -> Result<i32, HarnessError> {
    let exe = which::which(&command.executable).map_err(|_| HarnessError::HandlerFailed {
        action: "deviate".to_string(),
        detail: format!("executable '{}' not found in PATH", command.executable),
    })?;

    let output = std::process::Command::new(&exe)
        .args(&command.args)
        .env("CLAUDINE_ATTEMPT", failure.attempt.to_string())
        .env("CLAUDINE_FAILURE_EVENT", failure.event.to_string())
        .env(
            "CLAUDINE_SESSION_ID",
            failure.session_id.as_deref().unwrap_or(""),
        )
        .env(
            "CLAUDINE_SOURCE_FILE",
            source_file
                .map(|p| p.display().to_string())
                .unwrap_or_default(),
        )
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .output()
        .map_err(|e| HarnessError::HandlerFailed {
            action: "deviate".to_string(),
            detail: format!("failed to execute '{}': {e}", command.executable),
        })?;

    Ok(output.status.code().unwrap_or(-1))
}

/// Validate that a `Resume` handler can proceed.
///
/// Checks that:
/// 1. The provider supports session resume.
/// 2. A session ID is available from the previous attempt.
///
/// ## Errors
///
/// - `HarnessError::ResumeUnsupported` if the provider doesn't support resume.
/// - `HarnessError::ResumeNoSession` if no session ID is available.
pub fn validate_resume(
    provider_name: &str,
    supports_resume: bool,
    session_id: Option<&str>,
) -> Result<(), HarnessError> {
    if !supports_resume {
        return Err(HarnessError::ResumeUnsupported {
            provider: provider_name.to_string(),
        });
    }
    if session_id.is_none() {
        return Err(HarnessError::ResumeNoSession);
    }
    Ok(())
}

/// Build a `FailureContext` from an agent failure or timeout.
pub fn build_agent_failure_context(
    provider: &str,
    source_file: &Path,
    event: FailureEvent,
    message: String,
    attempt: u32,
    session_id: Option<String>,
    outcome: Option<AttemptOutcome>,
) -> FailureContext {
    FailureContext {
        provider: provider.to_string(),
        source_file: Some(source_file.to_path_buf()),
        event,
        failure_phase: FailurePhase::Agent,
        check: None,
        subject_key: None,
        message,
        attempt,
        session_id,
        outcome,
    }
}

/// Build `FailureContext` values from shell-audit failures.
///
/// Only non-source failures (pre-check, post-check, programmatic, declarative)
/// are converted. Source-page `::shell` failures are terminal in v1 and should
/// be handled separately before calling this function.
pub fn build_audit_failure_context(
    failures: &[&crate::harness::model::ShellAuditOutcome],
    provider: &str,
    source_file: &Path,
    attempt: u32,
) -> Vec<FailureContext> {
    use crate::harness::model::AuditedCommandSource;

    failures
        .iter()
        .map(|outcome| {
            let subject_key = match &outcome.command.source {
                AuditedCommandSource::PreCheck(id) => Some(format!("pre_check_{}", id.0)),
                AuditedCommandSource::PostCheck(id) => Some(format!("post_check_{}", id.0)),
                AuditedCommandSource::ProgrammaticHandle => Some("handle".to_string()),
                AuditedCommandSource::DeclarativeHandler {
                    subject_key, ..
                } => subject_key.clone(),
                AuditedCommandSource::ComposeSourceLine { .. } => None,
            };

            FailureContext {
                provider: provider.to_string(),
                source_file: Some(source_file.to_path_buf()),
                event: FailureEvent::ShellAuditDenied,
                failure_phase: FailurePhase::ShellAudit,
                check: None,
                subject_key,
                message: outcome.message.clone(),
                attempt,
                session_id: None,
                outcome: None,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::harness::model::{HandlerRule, ProcessTermination, ValidationEvent};

    fn make_handler_table(exact: Vec<HandlerRule>, generic: Vec<HandlerRule>) -> HandlerTable {
        HandlerTable { exact, generic }
    }

    fn test_failure_context(
        event: FailureEvent,
        subject_key: Option<String>,
        message: &str,
    ) -> FailureContext {
        FailureContext {
            provider: "codex".to_string(),
            source_file: Some(PathBuf::from("/repo/prompts/test.md")),
            event,
            failure_phase: FailurePhase::PostCheck,
            check: None,
            subject_key,
            message: message.to_string(),
            attempt: 1,
            session_id: None,
            outcome: None,
        }
    }

    #[test]
    fn subject_specific_handler_matched() {
        let table = make_handler_table(
            vec![HandlerRule {
                event: FailureEvent::Validation(ValidationEvent::FileExists),
                subject_key: Some("/repo/docs/output.md".to_string()),
                action: HandlerAction::Retry {
                    prompt_suffix: Some("Create the file.".to_string()),
                    set: None,
                    msg: None,
                    say: None,
                    retries: Some(1),
                },
            }],
            vec![],
        );

        let failure = test_failure_context(
            FailureEvent::Validation(ValidationEvent::FileExists),
            Some("/repo/docs/output.md".to_string()),
            "file not found",
        );

        let result = resolve_handler(&failure, &table, None).unwrap();
        assert!(result.is_some());
        assert!(matches!(result.unwrap(), HandlerAction::Retry { .. }));
    }

    #[test]
    fn generic_handler_matched_when_no_subject_specific() {
        let table = make_handler_table(
            vec![],
            vec![HandlerRule {
                event: FailureEvent::Timeout,
                subject_key: None,
                action: HandlerAction::Resume {
                    prompt: "Continue.".to_string(),
                    set: None,
                    msg: None,
                    say: None,
                    retries: Some(2),
                },
            }],
        );

        let failure = test_failure_context(FailureEvent::Timeout, None, "timed out");

        let result = resolve_handler(&failure, &table, None).unwrap();
        assert!(result.is_some());
        assert!(matches!(result.unwrap(), HandlerAction::Resume { .. }));
    }

    #[test]
    fn no_handler_returns_none() {
        let table = make_handler_table(vec![], vec![]);
        let failure = test_failure_context(FailureEvent::AgentFailure, None, "agent failed");

        let result = resolve_handler(&failure, &table, None).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn subject_specific_takes_precedence_over_generic() {
        let table = make_handler_table(
            vec![HandlerRule {
                event: FailureEvent::Validation(ValidationEvent::FileExists),
                subject_key: Some("/specific.md".to_string()),
                action: HandlerAction::Retry {
                    prompt_suffix: Some("specific retry".to_string()),
                    set: None,
                    msg: None,
                    say: None,
                    retries: None,
                },
            }],
            vec![HandlerRule {
                event: FailureEvent::Validation(ValidationEvent::FileExists),
                subject_key: None,
                action: HandlerAction::Retry {
                    prompt_suffix: Some("generic retry".to_string()),
                    set: None,
                    msg: None,
                    say: None,
                    retries: None,
                },
            }],
        );

        let failure = test_failure_context(
            FailureEvent::Validation(ValidationEvent::FileExists),
            Some("/specific.md".to_string()),
            "file not found",
        );

        let result = resolve_handler(&failure, &table, None).unwrap();
        let action = result.unwrap();
        if let HandlerAction::Retry { prompt_suffix, .. } = action {
            assert_eq!(prompt_suffix.as_deref(), Some("specific retry"));
        } else {
            panic!("expected Retry");
        }
    }

    // --- Resume validation tests ---

    #[test]
    fn resume_with_supported_provider_and_session_id() {
        let result = super::validate_resume("claude", true, Some("session-123"));
        assert!(result.is_ok());
    }

    #[test]
    fn resume_with_unsupported_provider() {
        let result = super::validate_resume("gemini", false, Some("session-123"));
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, crate::harness::error::HarnessError::ResumeUnsupported { ref provider } if provider == "gemini")
        );
    }

    #[test]
    fn resume_without_session_id() {
        let result = super::validate_resume("claude", true, None);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            crate::harness::error::HarnessError::ResumeNoSession
        ));
    }

    // --- Classify tests ---

    #[test]
    fn classify_timeout() {
        let outcome = AttemptOutcome {
            attempt: 1,
            session_id: None,
            final_response: String::new(),
            exit_code: 143,
            termination: ProcessTermination::TimedOut,
            stderr_text: None,
        };
        assert_eq!(classify_failure(&outcome), Some(FailureEvent::Timeout));
    }

    #[test]
    fn classify_agent_failure() {
        let outcome = AttemptOutcome {
            attempt: 1,
            session_id: None,
            final_response: String::new(),
            exit_code: 1,
            termination: ProcessTermination::Completed,
            stderr_text: None,
        };
        assert_eq!(classify_failure(&outcome), Some(FailureEvent::AgentFailure));
    }

    #[test]
    fn classify_success_returns_none() {
        let outcome = AttemptOutcome {
            attempt: 1,
            session_id: None,
            final_response: "done".to_string(),
            exit_code: 0,
            termination: ProcessTermination::Completed,
            stderr_text: None,
        };
        assert_eq!(classify_failure(&outcome), None);
    }

    #[test]
    fn classify_interrupt_returns_none() {
        let outcome = AttemptOutcome {
            attempt: 1,
            session_id: None,
            final_response: String::new(),
            exit_code: 130,
            termination: ProcessTermination::Interrupted,
            stderr_text: None,
        };
        assert_eq!(classify_failure(&outcome), None);
    }
}
