//! Harness presentation logic.
//!
//! All output uses `Status::from_prose` with `StatusTheme::Circular` and
//! writes to stderr. Every public function takes `Terminal` for rendering.

use std::path::Path;

use biscuit_terminal::components::status::{Status, StatusState, StatusTheme};
use biscuit_terminal::prelude::TerminalRenderable;
use biscuit_terminal::terminal::Terminal;

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
/// Escapes `<`, `>`, `{`, `}`, and `\` to backslash form to prevent
/// unintended markup, and `"` to the HTML entity `&quot;` because Prose's
/// attribute parser (e.g. inside `href="..."`) treats a backslashed quote
/// as a literal quote rather than a string delimiter -- the entity form is
/// the documented escape for embedding a quote inside an attribute value.
///
/// Performs a single linear scan over the input and reserves capacity up
/// front so the typical input (no special characters) costs one allocation.
pub fn prose_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 8);
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '<' => out.push_str("\\<"),
            '>' => out.push_str("\\>"),
            '{' => out.push_str("\\{"),
            '}' => out.push_str("\\}"),
            '"' => out.push_str("&quot;"),
            other => out.push(other),
        }
    }
    out
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
            StatusState::Error,
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
pub fn report_shell_audit_outcomes(report: &crate::harness::model::ShellAuditReport, term: &Terminal) {
    for outcome in &report.outcomes {
        let state = if outcome.passed {
            StatusState::Success
        } else {
            StatusState::Error
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
            StatusState::Error,
            term,
        );
    } else {
        emit_status(
            "the <blue-500>prompt</blue-500> frontmatter property is missing",
            StatusState::Error,
            term,
        );
    }
}

/// Emit a terminal unhandled failure banner.
pub fn report_unhandled_failure(message: &str, term: &Terminal) {
    emit_status(message, StatusState::Error, term);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::harness::model::{
        AuditedCommand, AuditedCommandSource, ApprovedRuntimeCommand, ShellAuditOutcome,
        ShellAuditReport,
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
                        source: AuditedCommandSource::ProgrammaticHandle,
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
        report_unhandled_failure("shell audit failed", &term);
    }
}
