//! Validation-related presentation logic.
//!
//! All output uses `Status::from_prose` with `StatusTheme::Circular` and
//! writes to stderr. Every public function takes `&Terminal` for rendering.

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
/// Escapes `<`, `>`, `{`, `}`, and `\` to prevent unintended markup.
pub fn prose_escape(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('<', "\\<")
        .replace('>', "\\>")
        .replace('{', "\\{")
        .replace('}', "\\}")
}

/// Emit the source-file existence status.
pub fn report_source_file(original_ref: &str, resolved_path: &Path, term: &Terminal) {
    let ref_escaped = prose_escape(original_ref);
    let path_display = resolved_path.display().to_string();

    if resolved_path.exists() {
        let filepath = resolved_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path_display.clone());
        let filepath_escaped = prose_escape(&filepath);
        let abs_escaped = prose_escape(&path_display);
        emit_status(
            &format!(
                "the file reference <blue-500>{ref_escaped}</blue-500> to the \
                 <blue-500><a href=\"{abs_escaped}\">{filepath_escaped}</a></blue-500> \
                 file on this host"
            ),
            StatusState::Success,
            term,
        );
    } else {
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
        FailurePhase::Agent => return,
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

/// Emit a terminal unhandled failure banner.
pub fn report_unhandled_failure(message: &str, term: &Terminal) {
    emit_status(message, StatusState::Failure, term);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prose_escape_handles_special_chars() {
        assert_eq!(prose_escape("<b>"), "\\<b\\>");
        assert_eq!(prose_escape("{x}"), "\\{x\\}");
        assert_eq!(prose_escape("a\\b"), "a\\\\b");
        assert_eq!(prose_escape("plain"), "plain");
    }

    #[test]
    fn report_phase_discovery_emits_nothing_for_zero() {
        // No panic, no output — just verifies the early return path
        let term = Terminal::default();
        report_phase_discovery(FailurePhase::PreCheck, 0, &term);
    }

    #[test]
    fn report_shell_audit_header_emits_nothing_for_zero() {
        let term = Terminal::default();
        report_shell_audit_header(0, &term);
    }
}
