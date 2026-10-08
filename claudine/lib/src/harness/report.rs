//! Harness presentation logic.
//!
//! All output uses `Status::from_prose` with `StatusTheme::Circular` and
//! writes to stderr. Every public function takes `Terminal` for rendering.

use std::path::Path;

use biscuit_terminal::components::prose::{LineBreaks, Prose};
use biscuit_terminal::components::status::{Status, StatusState, StatusTheme};
use biscuit_terminal::prelude::TerminalRenderable;
use biscuit_terminal::terminal::Terminal;

/// Render a single status line to stderr.
fn emit_status(markup: &str, state: StatusState, term: &Terminal) {
    let rendered = Status::from_prose(markup)
        .state(state)
        .theme(StatusTheme::Circular)
        .render(term);
    crate::render::console::write_stderr_line(&rendered);
}

/// Escape user-controlled text (file references, file names, shell commands,
/// recovery messages) so Prose renders it exactly as written.
///
/// Delegates to [`Prose::escape_text_outside_code_spans`], which neutralizes
/// Markdown emphasis and tags (a hand-rolled escaper that skipped `_` and `*`
/// rendered `_draft_.md` as an italic `draft.md`) while leaving a code span the
/// text marks with backticks literal. For an attribute value such as an
/// `href`, use [`Prose::quoted_attr`] instead.
pub fn prose_escape(s: &str) -> String {
    Prose::escape_text_outside_code_spans(s)
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
    let linked_path = linked_path_markup(resolved_path);

    format!(
        "the file reference was resolved to \
         <blue-500>{linked_path}</blue-500> \
         file on this host"
    )
}

fn linked_path_markup(path: &Path) -> String {
    let label_path = path.file_name().map(Path::new).unwrap_or(path);
    let label = prose_escape(&biscuit_file::to_portable_string(label_path));
    match url::Url::from_file_path(path) {
        Ok(href) => format!("<a href={}>{label}</a>", Prose::quoted_attr(href.as_str())),
        Err(()) => label,
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

/// Emit a lifecycle recovery status line once per recovery episode.
///
/// Callers pass a fully-formed status message (e.g. `"lifecycle retry:
/// re-running the agent (attempt 2)"`); this function only styles and emits
/// it. Markup characters in the message are escaped via [`prose_escape`].
pub fn report_lifecycle_recovery(message: &str, term: &Terminal) {
    let escaped = prose_escape(message);
    emit_status(&escaped, StatusState::Warning, term);
}

/// Emit an INFO status announcing a flow-control `proxy` hand-off to `target`.
///
/// Fired once per hand-off — from an `initialize` stack or a recovery
/// (`blocked`/`failure`/`finalize`) stack — at the point the harness loop
/// adopts the target document, so the operator sees *why* the running prompt
/// changed before the target's own lifecycle and prompt render.
pub fn report_proxy_handoff(target: &Path, term: &Terminal) {
    let linked_path = linked_path_markup(target);
    emit_status(
        &format!(
            "flow control redirected to \
             <blue-500>{linked_path}</blue-500>"
        ),
        StatusState::Info,
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
///
/// `message` is plain text — often a provider's own diagnostic — and is shown
/// exactly as written, so a masked secret (`****`) is not read as emphasis.
/// Its first line is the status line; any further line (such as a `hint:`
/// row) follows on a row of its own.
pub fn report_unhandled_failure(message: &str, term: &Terminal) {
    crate::render::console::write_stderr_line(&unhandled_failure_text(message, term));
}

fn unhandled_failure_text(message: &str, term: &Terminal) -> String {
    Status::from_prose(prose_escape(message))
        .with_line_breaks(LineBreaks::Hard)
        .state(StatusState::Error)
        .theme(StatusTheme::Circular)
        .render(term)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::harness::model::{
        AuditedCommand, AuditedCommandSource, ShellAuditOutcome, ShellAuditReport,
    };

    /// Create a Terminal without real terminal probing (avoids TTY hangs in tests).
    fn test_terminal() -> Terminal {
        Terminal::new_optimistic(80)
    }

    // -- prose_escape --

    /// What Prose shows for `markup`, with terminal styling stripped.
    fn rendered(markup: &str) -> String {
        biscuit_terminal::utils::escape_codes::strip_escape_codes(
            biscuit_terminal::components::prose::Prose::new(markup).render_optimistic(None),
        )
    }

    #[test]
    fn escaped_text_renders_exactly_as_written() {
        for text in [
            "<b>{x}</b> a\\b plain",
            "_draft_.md",
            "**bold** [link](target) *star*",
            r#"lifecycle retry: "quoted" _loop_count"#,
            "git log --format=%s *_test*",
        ] {
            assert_eq!(rendered(&prose_escape(text)), text, "{text:?}");
        }
    }

    #[test]
    fn a_backtick_code_span_renders_as_code_with_its_contents_literal() {
        assert_eq!(rendered(&prose_escape("**bold** `code_span *x*`")), "**bold** code_span *x*");
    }

    #[test]
    fn a_linked_file_name_keeps_its_underscores() {
        let path = std::env::temp_dir().join("_draft_ \"x\".md");
        let markup = linked_path_markup(&path);
        assert_eq!(rendered(&markup).trim(), "_draft_ \"x\".md", "{markup}");
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
        let path = std::env::temp_dir().join("_details.md");
        let href = url::Url::from_file_path(&path).unwrap();

        let markup = source_file_success_markup(&path);
        assert!(markup.contains(&format!("href=\"{href}\"")), "{markup}");
        assert_eq!(
            rendered(&markup).trim(),
            "the file reference was resolved to _details.md file on this host"
        );
    }

    #[test]
    fn linked_path_markup_uses_encoded_file_url() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("a b#%.md");
        let markup = linked_path_markup(&path);

        assert!(markup.contains("file://"), "expected file URL: {markup}");
        assert!(
            markup.contains("a%20b%23%25.md"),
            "expected encoded URL: {markup}"
        );
        assert!(
            markup.ends_with(">a b#%.md</a>"),
            "expected readable label: {markup}"
        );
    }

    #[test]
    fn linked_path_markup_leaves_unrepresentable_target_unlinked() {
        assert_eq!(linked_path_markup(Path::new("relative.md")), "relative.md");
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
                        source: AuditedCommandSource::ComposeSourceLine { line: 1 },
                        raw: "echo ok".to_string(),
                        executable: "echo".to_string(),
                        args: vec!["ok".to_string()],
                    },
                    passed: true,
                    message: "<green-500>echo ok</green-500> approved".to_string(),
                },
                ShellAuditOutcome {
                    command: AuditedCommand {
                        source: AuditedCommandSource::ComposeSourceLine { line: 2 },
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

    // -- report_lifecycle_recovery --

    #[test]
    fn report_lifecycle_recovery_escapes_message() {
        let term = test_terminal();
        // Message with markup-like characters should not panic
        report_lifecycle_recovery("/path/to/<source>.md", &term);
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
    fn unhandled_failure_keeps_each_further_line_on_its_own_row() {
        let term = Terminal::builder()
            .width(200)
            .color_depth(biscuit_terminal::discovery::detection::ColorDepth::None)
            .build();
        let rendered = biscuit_terminal::utils::escape_codes::strip_escape_codes(
            unhandled_failure_text("file not found: a*.md\nhint: quote it", &term),
        );
        let rows: Vec<&str> = rendered.lines().collect();
        assert_eq!(rows.len(), 2, "{rendered:?}");
        assert!(rows[0].ends_with("file not found: a*.md"), "{rendered:?}");
        assert_eq!(rows[1], "hint: quote it", "{rendered:?}");
    }

    #[test]
    fn report_unhandled_failure_renders() {
        let term = test_terminal();
        report_unhandled_failure("shell audit failed", &term);
    }
}
