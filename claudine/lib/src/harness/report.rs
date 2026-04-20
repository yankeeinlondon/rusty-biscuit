//! Validation-related presentation logic.
//!
//! All output uses `Status::from_prose` with `StatusTheme::Circular` and
//! writes to stderr. Every public function takes `&Terminal` for rendering.

#![allow(deprecated)]

use std::path::Path;

use biscuit_terminal::components::status::{Status, StatusState, StatusTheme};
use biscuit_terminal::prelude::Renderable;
use biscuit_terminal::terminal::Terminal;

use crate::harness::model::{FailurePhase, ShellAuditReport, ValidationPhaseReport};

/// Render a single status line to stderr.
fn emit_status(markup: &str, state: StatusState, term: &Terminal) {
    let rendered = Status::from_prose(markup)
        .state(state)
        .theme(StatusTheme::Circular)
        .render(term);
    eprintln!("{rendered}");
}

/// Escape user-controlled strings for safe Prose interpolation.
///
/// Escapes `<`, `>`, `{`, `}`, `"`, and `\` to prevent unintended markup
/// or attribute injection (e.g. inside `href="..."`).
pub fn prose_escape(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('<', "\\<")
        .replace('>', "\\>")
        .replace('{', "\\{")
        .replace('}', "\\}")
        .replace('"', "&quot;")
}

/// Emit the source-file existence status.
pub fn report_source_file(original_ref: &str, resolved_path: &Path, term: &Terminal) {
    if resolved_path.exists() {
        emit_status(
            &source_file_success_markup(resolved_path),
            StatusState::Success,
            term,
        );
    } else {
        let ref_escaped = prose_escape(original_ref);
        emit_status(
            &format!(
                "the file reference <blue-500>{ref_escaped}</blue-500> \
                 found no match on host computer!"
            ),
            StatusState::Failure,
            term,
        );
    }
}

fn source_file_success_markup(resolved_path: &Path) -> String {
    let path_display = resolved_path.display().to_string();
    let filepath = resolved_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or(path_display);
    let filepath_escaped = prose_escape(&filepath);
    let abs_escaped = prose_escape(&resolved_path.display().to_string());

    format!(
        "the file reference was resolved to \
         <blue-500><a href=\"{abs_escaped}\">{filepath_escaped}</a></blue-500> \
         file on this host"
    )
}

/// Emit the discovery header for a validation phase.
///
/// Only emits when count > 0. Uses correct singular/plural grammar.
pub fn report_phase_discovery(phase: FailurePhase, count: usize, term: &Terminal) {
    if count == 0 {
        return;
    }
    let phase_label = match phase {
        FailurePhase::PreCheck => "pre",
        FailurePhase::PostCheck => "post",
        FailurePhase::Agent | FailurePhase::ShellAudit => return,
    };
    let (check_word, verb) = if count == 1 {
        ("check", "was")
    } else {
        ("checks", "were")
    };
    emit_status(
        &format!("<b>{count}</b> validation <i>{phase_label} {check_word}</i> {verb} found:"),
        StatusState::Info,
        term,
    );
}

/// Emit individual check outcomes from a phase report.
pub fn report_check_outcomes(report: &ValidationPhaseReport, term: &Terminal) {
    for outcome in &report.outcomes {
        let state = if outcome.passed {
            StatusState::Success
        } else {
            StatusState::Failure
        };
        emit_status(&outcome.markup, state, term);
    }
}

/// Emit the shell audit header.
pub fn report_shell_audit_header(count: usize, term: &Terminal) {
    if count == 0 {
        return;
    }
    let (cmd_word, verb) = if count == 1 {
        ("command", "was")
    } else {
        ("commands", "were")
    };
    emit_status(
        &format!("<b>{count}</b> shell {cmd_word} {verb} audited:"),
        StatusState::Info,
        term,
    );
}

/// Emit individual shell audit outcomes.
pub fn report_shell_audit_outcomes(report: &ShellAuditReport, term: &Terminal) {
    for outcome in &report.outcomes {
        let state = if outcome.passed {
            StatusState::Success
        } else {
            StatusState::Failure
        };
        emit_status(&outcome.message, state, term);
    }
}

/// Emit the handler-engagement banner once per failure episode.
pub fn report_handler_engagement(source_display: &str, term: &Terminal) {
    let escaped = prose_escape(source_display);
    emit_status(
        &format!(
            "an <red>error</red> was encountered while processing \
             <blue>{escaped}</blue>, engaging registered handlers."
        ),
        StatusState::Warning,
        term,
    );
}

/// Emit the prompt frontmatter property status.
///
/// Reports success when `prompt` exists and is a non-empty string,
/// failure otherwise. The `outcome` describes what was found.
pub fn report_prompt_property(has_prompt: bool, is_non_empty: bool, term: &Terminal) {
    if has_prompt && is_non_empty {
        emit_status(
            "the <blue-500>prompt</blue-500> frontmatter property is present and non-empty",
            StatusState::Success,
            term,
        );
    } else if has_prompt {
        emit_status(
            "the <blue-500>prompt</blue-500> frontmatter property is present but empty",
            StatusState::Failure,
            term,
        );
    } else {
        emit_status(
            "the <blue-500>prompt</blue-500> frontmatter property is missing",
            StatusState::Failure,
            term,
        );
    }
}

/// Emit a terminal unhandled failure banner.
pub fn report_unhandled_failure(message: &str, term: &Terminal) {
    emit_status(message, StatusState::Failure, term);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::harness::model::{
        AuditedCommand, AuditedCommandSource, ShellAuditOutcome, ValidationCheckOutcome,
        ValidationEvent, ValidationRuleId,
    };

    /// Create a Terminal without real terminal probing (avoids TTY hangs in tests).
    fn test_terminal() -> Terminal {
        Terminal::new_optimistic(80)
    }

    // -- prose_escape --

    #[test]
    fn prose_escape_handles_special_chars() {
        assert_eq!(prose_escape("<b>"), "\\<b\\>");
        assert_eq!(prose_escape("{x}"), "\\{x\\}");
        assert_eq!(prose_escape("a\\b"), "a\\\\b");
        assert_eq!(prose_escape("plain"), "plain");
    }

    #[test]
    fn prose_escape_escapes_double_quotes() {
        assert_eq!(prose_escape(r#"href="evil""#), r#"href=&quot;evil&quot;"#);
    }

    // -- report_source_file --

    #[test]
    fn report_source_file_success_path() {
        let term = test_terminal();
        let tmp = tempfile::NamedTempFile::new().unwrap();
        // Should not panic; emits a success status
        report_source_file("@my-ref", tmp.path(), &term);
    }

    #[test]
    fn source_file_success_markup_uses_resolved_filename_link_text() {
        let path = Path::new("/tmp/_details.md");

        assert_eq!(
            source_file_success_markup(path),
            "the file reference was resolved to \
             <blue-500><a href=\"/tmp/_details.md\">_details.md</a></blue-500> \
             file on this host"
        );
    }

    #[test]
    fn report_source_file_missing_path() {
        let term = test_terminal();
        // Should not panic; emits a failure status
        report_source_file("@missing", Path::new("/nonexistent/file.md"), &term);
    }

    // -- report_phase_discovery --

    #[test]
    fn report_phase_discovery_emits_nothing_for_zero() {
        let term = test_terminal();
        report_phase_discovery(FailurePhase::PreCheck, 0, &term);
    }

    #[test]
    fn report_phase_discovery_singular() {
        let term = test_terminal();
        // Singular grammar: "1 validation pre check was found"
        report_phase_discovery(FailurePhase::PreCheck, 1, &term);
    }

    #[test]
    fn report_phase_discovery_plural() {
        let term = test_terminal();
        // Plural grammar: "3 validation post checks were found"
        report_phase_discovery(FailurePhase::PostCheck, 3, &term);
    }

    #[test]
    fn report_phase_discovery_agent_is_noop() {
        let term = test_terminal();
        // Agent phase should silently return
        report_phase_discovery(FailurePhase::Agent, 5, &term);
    }

    #[test]
    fn report_phase_discovery_shell_audit_is_noop() {
        let term = test_terminal();
        report_phase_discovery(FailurePhase::ShellAudit, 2, &term);
    }

    // -- report_check_outcomes --

    #[test]
    fn report_check_outcomes_success_and_failure() {
        let term = test_terminal();
        let report = ValidationPhaseReport {
            phase: FailurePhase::PreCheck,
            outcomes: vec![
                ValidationCheckOutcome {
                    rule_id: ValidationRuleId(0),
                    event: ValidationEvent::FileExists,
                    subject_key: None,
                    passed: true,
                    markup: "the file /a exists".to_string(),
                    failure_message: None,
                },
                ValidationCheckOutcome {
                    rule_id: ValidationRuleId(1),
                    event: ValidationEvent::DirExists,
                    subject_key: None,
                    passed: false,
                    markup: "the directory /b exists".to_string(),
                    failure_message: Some("not found".to_string()),
                },
            ],
        };
        // Should render both without panicking
        report_check_outcomes(&report, &term);
    }

    // -- report_shell_audit_header --

    #[test]
    fn report_shell_audit_header_emits_nothing_for_zero() {
        let term = test_terminal();
        report_shell_audit_header(0, &term);
    }

    #[test]
    fn report_shell_audit_header_singular() {
        let term = test_terminal();
        report_shell_audit_header(1, &term);
    }

    #[test]
    fn report_shell_audit_header_plural() {
        let term = test_terminal();
        report_shell_audit_header(4, &term);
    }

    // -- report_shell_audit_outcomes --

    #[test]
    fn report_shell_audit_outcomes_mixed() {
        let term = test_terminal();
        let report = ShellAuditReport {
            outcomes: vec![
                ShellAuditOutcome {
                    command: AuditedCommand {
                        source: AuditedCommandSource::PreCheck(ValidationRuleId(0)),
                        raw: "echo ok".to_string(),
                        executable: "echo".to_string(),
                        args: vec!["ok".to_string()],
                    },
                    passed: true,
                    message: "<green-500>echo ok</green-500> approved".to_string(),
                },
                ShellAuditOutcome {
                    command: AuditedCommand {
                        source: AuditedCommandSource::ProgrammaticHandle,
                        raw: "rm -rf /".to_string(),
                        executable: "rm".to_string(),
                        args: vec!["-rf".to_string(), "/".to_string()],
                    },
                    passed: false,
                    message: "<red-500>rm -rf /</red-500> denied by policy".to_string(),
                },
            ],
        };
        report_shell_audit_outcomes(&report, &term);
    }

    // -- report_handler_engagement --

    #[test]
    fn report_handler_engagement_escapes_source_display() {
        let term = test_terminal();
        // Source with markup-like characters should not panic
        report_handler_engagement("/path/to/<source>.md", &term);
    }

    // -- report_prompt_property --

    #[test]
    fn report_prompt_property_present_and_non_empty() {
        let term = test_terminal();
        report_prompt_property(true, true, &term);
    }

    #[test]
    fn report_prompt_property_present_but_empty() {
        let term = test_terminal();
        report_prompt_property(true, false, &term);
    }

    #[test]
    fn report_prompt_property_missing() {
        let term = test_terminal();
        report_prompt_property(false, false, &term);
    }

    // -- report_unhandled_failure --

    #[test]
    fn report_unhandled_failure_renders() {
        let term = test_terminal();
        report_unhandled_failure("pre-check validation failed (2 failures)", &term);
    }

    // -- state mapping --

    #[test]
    fn report_check_outcomes_maps_pass_to_success_state() {
        // Verify the state mapping logic: passed → Success, failed → Failure.
        // We can't easily capture stderr, but we verify no panic and
        // the mapping code is exercised.
        let term = test_terminal();
        let report = ValidationPhaseReport {
            phase: FailurePhase::PostCheck,
            outcomes: vec![ValidationCheckOutcome {
                rule_id: ValidationRuleId(0),
                event: ValidationEvent::FileChanged,
                subject_key: Some("/a.md".to_string()),
                passed: true,
                markup: "file changed".to_string(),
                failure_message: None,
            }],
        };
        report_check_outcomes(&report, &term);
    }
}
