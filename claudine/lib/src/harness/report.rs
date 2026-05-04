//! Validation-related presentation logic.
//!
//! All output uses `Status::from_prose` with `StatusTheme::Circular` and
//! writes to stderr. Every public function takes `&Terminal` for rendering.

#![allow(deprecated)]

use std::path::{Path, PathBuf};

use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::status::{Status, StatusState, StatusTheme};
use biscuit_terminal::prelude::Renderable;
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::layout::WordWrap;
use darkmatter::markdown::highlighting::highlight_yaml_lines;

use crate::harness::model::{
    FailurePhase, ShellAuditReport, ValidationCheckOutcome, ValidationPhaseReport,
};

/// Sink that receives one rendered line at a time.
///
/// Implementations decide whether the line is written to stderr, captured
/// in memory for tests, or routed elsewhere. The sink is intentionally
/// minimal so the I/O seam stays trivial to swap.
trait LineSink {
    fn write_line(&mut self, line: &str);
}

/// `LineSink` that writes each line to stderr via `eprintln!`.
struct StderrSink;

impl LineSink for StderrSink {
    fn write_line(&mut self, line: &str) {
        eprintln!("{line}");
    }
}

/// `LineSink` that accumulates lines into an in-memory string buffer.
///
/// Each `write_line` call appends the line followed by `\n`, matching the
/// behavior of `eprintln!` so captured output is byte-equivalent to what a
/// `StderrSink` run would have emitted.
///
/// Currently only used by the test suite and the test-only
/// [`report_check_outcomes_to_string`] entry point; later phases of the
/// validation-reporter rework may expose this to additional consumers.
#[cfg(test)]
#[derive(Default)]
struct StringSink {
    buf: String,
}

#[cfg(test)]
impl StringSink {
    fn into_string(self) -> String {
        self.buf
    }
}

#[cfg(test)]
impl LineSink for StringSink {
    fn write_line(&mut self, line: &str) {
        self.buf.push_str(line);
        self.buf.push('\n');
    }
}

/// Render a single status line into the provided sink.
fn emit_status_to(
    sink: &mut dyn LineSink,
    markup: &str,
    state: StatusState,
    term: &Terminal,
) {
    let rendered = Status::from_prose(markup)
        .state(state)
        .theme(StatusTheme::Circular)
        .render(term);
    sink.write_line(&rendered);
}

/// Render a single status line to stderr.
fn emit_status(markup: &str, state: StatusState, term: &Terminal) {
    emit_status_to(&mut StderrSink, markup, state, term);
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
///
/// Passing checks render as a single compact `Status` line. Failing checks
/// with `RuleSource` populated render the four-section failure block
/// (status header, source location, YAML snippet, reason). Failing checks
/// without `source` fall back to a legacy single-line failure plus reason.
pub fn report_check_outcomes(report: &ValidationPhaseReport, term: &Terminal) {
    report_check_outcomes_to(&mut StderrSink, report, term);
}

/// Like [`report_check_outcomes`] but captures rendered output to a string.
///
/// This is intended for tests and any other consumer that needs to inspect
/// the exact bytes the reporter would have written to stderr without having
/// to capture the file descriptor.
///
/// ## Returns
///
/// The captured output, with each rendered line followed by `\n`.
#[cfg(test)]
pub(crate) fn report_check_outcomes_to_string(
    report: &ValidationPhaseReport,
    term: &Terminal,
) -> String {
    let mut sink = StringSink::default();
    report_check_outcomes_to(&mut sink, report, term);
    sink.into_string()
}

/// Sink-routed core of [`report_check_outcomes`].
fn report_check_outcomes_to(
    sink: &mut dyn LineSink,
    report: &ValidationPhaseReport,
    term: &Terminal,
) {
    for outcome in &report.outcomes {
        if outcome.passed {
            emit_status_to(sink, &outcome.markup, StatusState::Success, term);
        } else if outcome.source.is_some() {
            render_failure_block_to(sink, outcome, report.phase, term);
        } else {
            emit_status_to(sink, &outcome.markup, StatusState::Failure, term);
            if let Some(reason) = &outcome.failure_message {
                let escaped = prose_escape(reason);
                let prose = Prose::new(format!("  Reason: <dim>{escaped}</dim>"));
                sink.write_line(&prose.render(term));
            }
        }
    }
}

/// Header label for a failed validation phase.
fn failure_header_text(phase: FailurePhase) -> &'static str {
    match phase {
        FailurePhase::PreCheck => "Pre-validation failed",
        FailurePhase::PostCheck => "Post-validation failed",
        FailurePhase::Agent => "Agent execution failed",
        FailurePhase::ShellAudit => "Shell audit failed",
    }
}

/// Render a four-section failure block for an outcome that has
/// `RuleSource` metadata. Routes output through the provided sink.
fn render_failure_block_to(
    sink: &mut dyn LineSink,
    outcome: &ValidationCheckOutcome,
    phase: FailurePhase,
    term: &Terminal,
) {
    // Section 1: status header.
    emit_status_to(sink, failure_header_text(phase), StatusState::Failure, term);

    // Sections 2 + 3 require the source metadata; the caller has already
    // verified `source.is_some()` so the unwrap below is sound.
    let Some(src) = &outcome.source else {
        return;
    };

    // Section 2: source location, OSC8-linked when terminal supports it.
    let abs = src.file.display().to_string();
    let display = relative_or_abs(&src.file);
    let display_escaped = prose_escape(&display);
    let abs_escaped = prose_escape(&abs);
    let suffix = match &src.line_range {
        Some(r) => format!(":{}-{}", r.start(), r.end()),
        None => String::new(),
    };
    let suffix_escaped = prose_escape(&suffix);
    emit_status_to(
        sink,
        &format!("in <a href=\"{abs_escaped}\">{display_escaped}{suffix_escaped}</a>"),
        StatusState::Info,
        term,
    );

    // Section 3: chrome-free, syntax-highlighted YAML snippet.
    //
    // We deliberately skip darkmatter's `YamlBlock::render` here because it
    // wraps the body in a `yaml`-labelled code-block frame with full-width
    // background fill, which is too visually heavy for an inline failure
    // block. `highlight_yaml_lines` returns one ANSI-styled line per input
    // line; if the result count does not match the plain-line count we
    // fall back to unstyled snippet lines so the user is never shown a
    // missing snippet.
    let trimmed_snippet = src.yaml_snippet.trim_end();
    if !trimmed_snippet.is_empty() {
        let plain_lines: Vec<&str> = trimmed_snippet.lines().collect();
        let highlighted = highlight_yaml_lines(trimmed_snippet);
        if highlighted.len() == plain_lines.len() {
            for line in &highlighted {
                sink.write_line(&format!("    {line}"));
            }
        } else {
            for line in &plain_lines {
                sink.write_line(&format!("    {line}"));
            }
        }
    }

    // Section 4: muted reason line. Severity is already conveyed by the
    // status glyph above so the reason is rendered dim.
    if let Some(reason) = &outcome.failure_message {
        let escaped = prose_escape(reason);
        let prose = Prose::new(format!("  Reason: <dim>{escaped}</dim>"))
            .with_word_wrap(WordWrap::WrapProse(None, None));
        sink.write_line(&prose.render(term));
    }
}

/// Produce a `cwd`-relative display string when possible, falling back to
/// the absolute path.
fn relative_or_abs(path: &Path) -> String {
    if let Ok(cwd) = std::env::current_dir()
        && let Ok(rel) = path.strip_prefix(&cwd)
    {
        return PathBuf::from(rel).display().to_string();
    }
    path.display().to_string()
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
        AuditedCommand, AuditedCommandSource, RuleSource, ShellAuditOutcome,
        ValidationCheckOutcome, ValidationEvent, ValidationRuleId,
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
                    source: None,
                },
                ValidationCheckOutcome {
                    rule_id: ValidationRuleId(1),
                    event: ValidationEvent::DirExists,
                    subject_key: None,
                    passed: false,
                    markup: "the directory /b exists".to_string(),
                    failure_message: Some("not found".to_string()),
                    source: None,
                },
            ],
        };
        // Should render both without panicking
        report_check_outcomes(&report, &term);
    }

    #[test]
    fn report_check_outcomes_failure_with_source_emits_block() {
        // Failing outcome with `source: Some(...)` should drive the
        // four-section failure block path. We can't easily capture stderr
        // in unit tests, so this is primarily a smoke test that ensures
        // the renderer does not panic and exercises the
        // `render_failure_block_to` branch.
        let term = test_terminal();
        let report = ValidationPhaseReport {
            phase: FailurePhase::PreCheck,
            outcomes: vec![ValidationCheckOutcome {
                rule_id: ValidationRuleId(0),
                event: ValidationEvent::FileExists,
                subject_key: Some("Cargo.toml".to_string()),
                passed: false,
                markup: "the file Cargo.toml exists".to_string(),
                failure_message: Some("file does not exist: Cargo.toml".to_string()),
                source: Some(RuleSource {
                    file: std::path::PathBuf::from("/repo/prompts/run.md"),
                    line_range: None,
                    yaml_snippet: "file_exists: \"Cargo.toml\"\n".to_string(),
                }),
            }],
        };
        report_check_outcomes(&report, &term);
    }

    #[test]
    fn report_check_outcomes_failure_without_source_uses_legacy_path() {
        // Failing outcome with `source: None` should fall back to the
        // legacy single-line + reason rendering. Verifies the dispatch
        // logic without requiring stderr capture.
        let term = test_terminal();
        let report = ValidationPhaseReport {
            phase: FailurePhase::PreCheck,
            outcomes: vec![ValidationCheckOutcome {
                rule_id: ValidationRuleId(0),
                event: ValidationEvent::DirExists,
                subject_key: None,
                passed: false,
                markup: "the directory /b exists".to_string(),
                failure_message: Some("not found".to_string()),
                source: None,
            }],
        };
        report_check_outcomes(&report, &term);
    }

    #[test]
    fn report_check_outcomes_pass_path_unchanged() {
        // Regression guard: passing outcomes should never enter the
        // failure-block branch regardless of whether `source` is set.
        // The renderer should produce a single compact `Status` line.
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
                    source: None,
                },
                ValidationCheckOutcome {
                    rule_id: ValidationRuleId(1),
                    event: ValidationEvent::FileExists,
                    subject_key: None,
                    passed: true,
                    markup: "the file /b exists".to_string(),
                    failure_message: None,
                    source: Some(RuleSource {
                        file: std::path::PathBuf::from("/repo/prompts/run.md"),
                        line_range: None,
                        yaml_snippet: "file_exists: \"/b\"\n".to_string(),
                    }),
                },
            ],
        };
        report_check_outcomes(&report, &term);
    }

    #[test]
    fn render_failure_block_handles_all_phases() {
        // Cover each `FailurePhase` arm of `failure_header_text` via the
        // failure-block path so that future header changes are exercised.
        let term = test_terminal();
        let phases = [
            FailurePhase::PreCheck,
            FailurePhase::PostCheck,
            FailurePhase::Agent,
            FailurePhase::ShellAudit,
        ];
        for phase in phases {
            let report = ValidationPhaseReport {
                phase,
                outcomes: vec![ValidationCheckOutcome {
                    rule_id: ValidationRuleId(0),
                    event: ValidationEvent::FileExists,
                    subject_key: None,
                    passed: false,
                    markup: "the file x exists".to_string(),
                    failure_message: Some("missing".to_string()),
                    source: Some(RuleSource {
                        file: std::path::PathBuf::from("/repo/prompts/run.md"),
                        line_range: Some(7..=9),
                        yaml_snippet: "file_exists: \"x\"\n".to_string(),
                    }),
                }],
            };
            report_check_outcomes(&report, &term);
        }
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
                source: None,
            }],
        };
        report_check_outcomes(&report, &term);
    }

    // -- sink-routed capture tests (Phase 1) --

    #[test]
    fn report_check_outcomes_to_string_pass_renders_single_compact_row() {
        let term = test_terminal();
        let report = ValidationPhaseReport {
            phase: FailurePhase::PreCheck,
            outcomes: vec![ValidationCheckOutcome {
                rule_id: ValidationRuleId(0),
                event: ValidationEvent::FileExists,
                subject_key: None,
                passed: true,
                markup: "the file /a exists".to_string(),
                failure_message: None,
                source: None,
            }],
        };

        let captured = report_check_outcomes_to_string(&report, &term);

        let non_empty: Vec<&str> = captured.lines().filter(|l| !l.trim().is_empty()).collect();
        assert_eq!(
            non_empty.len(),
            1,
            "pass-state should render exactly one non-empty line, got {non_empty:?}"
        );
        assert!(
            captured.contains("the file /a exists"),
            "expected captured output to contain rendered markup, got {captured:?}"
        );
    }

    #[test]
    fn render_failure_block_to_string_emits_four_sections_for_full_source() {
        let term = test_terminal();
        let outcome = ValidationCheckOutcome {
            rule_id: ValidationRuleId(0),
            event: ValidationEvent::FileExists,
            subject_key: Some("Cargo.toml".to_string()),
            passed: false,
            markup: "the file Cargo.toml exists".to_string(),
            failure_message: Some("file does not exist: Cargo.toml".to_string()),
            source: Some(RuleSource {
                file: std::path::PathBuf::from("/repo/prompts/run.md"),
                line_range: Some(7..=9),
                yaml_snippet: "file_exists: \"Cargo.toml\"\nmessage: \"need toml\"\n".to_string(),
            }),
        };

        let mut sink = StringSink::default();
        render_failure_block_to(&mut sink, &outcome, FailurePhase::PreCheck, &term);
        let captured = sink.into_string();

        let lines: Vec<&str> = captured.lines().collect();
        assert!(
            !lines.is_empty(),
            "expected captured output to have at least one line"
        );

        // Section 1: status header on the first non-empty line.
        let first_non_empty = lines
            .iter()
            .find(|l| !l.trim().is_empty())
            .expect("at least one non-empty line");
        assert!(
            first_non_empty.contains("Pre-validation failed"),
            "first non-empty line should contain failure header, got {first_non_empty:?}"
        );

        // Section 2: source location with `:7-9` suffix.
        assert!(
            lines
                .iter()
                .any(|l| l.contains("in ") && l.contains(":7-9")),
            "expected a line containing 'in ' and ':7-9', got {captured:?}"
        );

        // Section 3: indented YAML snippet referencing the rule name.
        assert!(
            lines
                .iter()
                .any(|l| l.starts_with("    ") && l.contains("file_exists")),
            "expected an indented YAML line containing 'file_exists', got {captured:?}"
        );

        // Section 4: Reason line carrying the failure_message verbatim.
        assert!(
            lines
                .iter()
                .any(|l| l.contains("Reason:") && l.contains("file does not exist: Cargo.toml")),
            "expected a Reason line containing the failure message, got {captured:?}"
        );
    }

    // -- Phase 4: chrome-free YAML rendering --

    /// Strip every ANSI/OSC escape sequence so we can reason about visible
    /// text only. We need both the SGR strip and the OSC-8 strip because
    /// `biscuit_terminal::utils::escape_codes::strip_escape_codes` already
    /// handles CSI + OSC. We re-export it here to keep test imports local.
    fn strip_ansi(text: &str) -> String {
        biscuit_terminal::utils::escape_codes::strip_escape_codes(text)
    }

    #[test]
    fn render_failure_block_yaml_section_has_no_yaml_label_or_box() {
        let term = test_terminal();
        let outcome = ValidationCheckOutcome {
            rule_id: ValidationRuleId(0),
            event: ValidationEvent::FileExists,
            subject_key: Some("Cargo.toml".to_string()),
            passed: false,
            markup: "the file Cargo.toml exists".to_string(),
            failure_message: Some("file does not exist: Cargo.toml".to_string()),
            source: Some(RuleSource {
                file: std::path::PathBuf::from("/repo/prompts/run.md"),
                line_range: Some(7..=8),
                yaml_snippet: "file_exists: \"Cargo.toml\"\nmessage: \"need toml\"\n".to_string(),
            }),
        };

        let mut sink = StringSink::default();
        render_failure_block_to(&mut sink, &outcome, FailurePhase::PreCheck, &term);
        let captured = sink.into_string();
        let plain = strip_ansi(&captured);

        // Should NEVER contain the standalone `yaml` language label that
        // fenced code-block headers use.
        assert!(
            !plain.contains("\nyaml\n"),
            "YAML section must not contain the 'yaml' header label, got plain output: {plain:?}"
        );

        // Locate the YAML region (lines between the source line containing
        // `in ` and the `Reason:` line) and assert no all-spaces/empty
        // background-fill row appears in it. With the chrome-free renderer
        // every YAML line carries actual snippet content.
        let plain_lines: Vec<&str> = plain.lines().collect();
        let in_idx = plain_lines
            .iter()
            .position(|l| l.contains("in ") && l.contains(":7-8"))
            .expect("expected an 'in <path>:7-8' source-location line");
        let reason_idx = plain_lines
            .iter()
            .position(|l| l.contains("Reason:"))
            .expect("expected a 'Reason:' line after the YAML region");
        assert!(
            reason_idx > in_idx,
            "Reason line must appear after the source-location line"
        );
        let yaml_region = &plain_lines[(in_idx + 1)..reason_idx];
        assert!(
            !yaml_region.is_empty(),
            "expected at least one YAML region line, got {plain_lines:?}"
        );
        for line in yaml_region {
            // No "all-spaces" background-fill rows. After ANSI strip, code
            // block padding rows show up as either an empty line or a line
            // consisting only of whitespace; the chrome-free renderer must
            // not produce any such filler.
            assert!(
                !line.chars().all(|c| c.is_whitespace()),
                "YAML region must not contain a whitespace-only background-fill row, got: {line:?}"
            );
        }
    }

    #[test]
    fn render_failure_block_yaml_section_indents_each_line_by_four_spaces() {
        let term = test_terminal();
        let outcome = ValidationCheckOutcome {
            rule_id: ValidationRuleId(0),
            event: ValidationEvent::FileExists,
            subject_key: Some("x".to_string()),
            passed: false,
            markup: "the file x exists".to_string(),
            failure_message: Some("missing".to_string()),
            source: Some(RuleSource {
                file: std::path::PathBuf::from("/repo/prompts/run.md"),
                line_range: Some(3..=4),
                yaml_snippet: "file_exists: x\nmessage: \"...\"\n".to_string(),
            }),
        };

        let mut sink = StringSink::default();
        render_failure_block_to(&mut sink, &outcome, FailurePhase::PreCheck, &term);
        let captured = sink.into_string();
        let plain = strip_ansi(&captured);

        let plain_lines: Vec<&str> = plain.lines().collect();
        let in_idx = plain_lines
            .iter()
            .position(|l| l.contains("in ") && l.contains(":3-4"))
            .expect("expected an 'in <path>:3-4' source-location line");
        let reason_idx = plain_lines
            .iter()
            .position(|l| l.contains("Reason:"))
            .expect("expected a 'Reason:' line after the YAML region");
        let yaml_region = &plain_lines[(in_idx + 1)..reason_idx];

        assert_eq!(
            yaml_region.len(),
            2,
            "expected exactly two YAML lines for a two-line snippet, got {yaml_region:?}"
        );

        for line in yaml_region {
            assert!(
                line.starts_with("    "),
                "every YAML line must start with exactly four spaces, got: {line:?}"
            );
            // And exactly four (not five+): char at index 4 must not be a space.
            let after_indent = &line[4..];
            assert!(
                !after_indent.starts_with(' '),
                "indent must be exactly four spaces, got: {line:?}"
            );
        }

        // First snippet line should still contain the rule key after the indent.
        assert!(
            yaml_region[0].contains("file_exists"),
            "first YAML region line should contain 'file_exists', got: {:?}",
            yaml_region[0]
        );
    }

    #[test]
    fn legacy_failure_path_without_source_emits_status_then_reason() {
        let term = test_terminal();
        let report = ValidationPhaseReport {
            phase: FailurePhase::PreCheck,
            outcomes: vec![ValidationCheckOutcome {
                rule_id: ValidationRuleId(0),
                event: ValidationEvent::DirExists,
                subject_key: None,
                passed: false,
                markup: "the directory /b exists".to_string(),
                failure_message: Some("not found".to_string()),
                source: None,
            }],
        };

        let captured = report_check_outcomes_to_string(&report, &term);
        let non_empty: Vec<&str> = captured.lines().filter(|l| !l.trim().is_empty()).collect();
        assert_eq!(
            non_empty.len(),
            2,
            "legacy fallback should produce exactly two non-empty lines, got {non_empty:?}"
        );
        assert!(
            non_empty[1].starts_with("  "),
            "second line should begin with two leading spaces, got {:?}",
            non_empty[1]
        );
        assert!(
            non_empty[1].contains("not found"),
            "second line should contain the failure_message, got {:?}",
            non_empty[1]
        );
    }

    // -- Phase 5: Level 1 asserted text tests for the full failure block --

    /// Capture the rendered output of `report_check_outcomes` and strip ANSI
    /// escape codes (CSI, OSC, Fe) so tests can assert on visible text only.
    fn capture_check_outcomes(report: &ValidationPhaseReport) -> String {
        let term = test_terminal();
        let raw = report_check_outcomes_to_string(report, &term);
        biscuit_terminal::utils::escape_codes::strip_escape_codes(raw)
    }

    /// Build a single-outcome `ValidationPhaseReport` for failure-block tests.
    fn single_failure_report(
        phase: FailurePhase,
        source: Option<RuleSource>,
        failure_message: Option<String>,
    ) -> ValidationPhaseReport {
        ValidationPhaseReport {
            phase,
            outcomes: vec![ValidationCheckOutcome {
                rule_id: ValidationRuleId(0),
                event: ValidationEvent::FileExists,
                subject_key: Some("Cargo.toml".to_string()),
                passed: false,
                markup: "the file Cargo.toml exists".to_string(),
                failure_message,
                source,
            }],
        }
    }

    #[test]
    fn pass_state_renders_exactly_one_compact_row() {
        let report = ValidationPhaseReport {
            phase: FailurePhase::PreCheck,
            outcomes: vec![ValidationCheckOutcome {
                rule_id: ValidationRuleId(0),
                event: ValidationEvent::FileExists,
                subject_key: None,
                passed: true,
                markup: "the file /a exists".to_string(),
                failure_message: None,
                source: None,
            }],
        };

        let plain = capture_check_outcomes(&report);
        let non_empty: Vec<&str> = plain.lines().filter(|l| !l.trim().is_empty()).collect();
        assert_eq!(
            non_empty.len(),
            1,
            "pass-state should render exactly one trimmed non-empty line, got {non_empty:?}"
        );
        assert!(
            plain.contains("the file /a exists"),
            "captured output should contain rendered markup, got {plain:?}"
        );
    }

    #[test]
    fn failure_state_full_block_contains_all_four_sections() {
        let test_path = "/Users/ken/repo/Cargo.toml";
        let source_file = std::path::PathBuf::from(
            "/Users/ken/repo/features/2026-04-29-leverage-dm-parser/spec.md",
        );
        let report = single_failure_report(
            FailurePhase::PreCheck,
            Some(RuleSource {
                file: source_file.clone(),
                line_range: Some(42..=44),
                yaml_snippet:
                    "- file_exists: \"Cargo.toml\"\n  message: \"{{source_file}} requires a Cargo.toml\"\n"
                        .to_string(),
            }),
            Some(format!("file does not exist: {test_path}")),
        );

        let plain = capture_check_outcomes(&report);
        let lines: Vec<&str> = plain.lines().collect();

        // 1. Header `Pre-validation failed` on the first non-empty line.
        let header_idx = lines
            .iter()
            .position(|l| !l.trim().is_empty())
            .expect("expected at least one non-empty line");
        assert!(
            lines[header_idx].contains("Pre-validation failed"),
            "first non-empty line should contain 'Pre-validation failed', got {:?}",
            lines[header_idx]
        );

        // 2. Source-location line `in <path>:42-44` after the header.
        let source_idx = lines[header_idx + 1..]
            .iter()
            .position(|l| l.contains("in ") && l.contains(":42-44"))
            .map(|i| i + header_idx + 1)
            .expect("expected source-location line containing 'in ' and ':42-44'");
        assert!(
            source_idx > header_idx,
            "source-location line must follow the header"
        );

        // 3. YAML snippet content with four-space indent appears between source and reason.
        let yaml_idx = lines[source_idx + 1..]
            .iter()
            .position(|l| l.starts_with("    ") && l.contains("file_exists"))
            .map(|i| i + source_idx + 1)
            .expect("expected indented YAML snippet line containing 'file_exists'");

        // 4. `Reason: file does not exist:` followed by test path, after YAML.
        let reason_idx = lines[yaml_idx + 1..]
            .iter()
            .position(|l| l.contains("Reason: file does not exist:") && l.contains(test_path))
            .map(|i| i + yaml_idx + 1)
            .expect("expected Reason line containing 'file does not exist:' and the test path");
        assert!(
            reason_idx > yaml_idx,
            "Reason line must follow the YAML snippet"
        );
    }

    #[test]
    fn failure_state_pre_phase_uses_pre_validation_failed() {
        let report = single_failure_report(
            FailurePhase::PreCheck,
            Some(RuleSource {
                file: std::path::PathBuf::from("/repo/prompts/run.md"),
                line_range: Some(7..=9),
                yaml_snippet: "file_exists: \"x\"\n".to_string(),
            }),
            Some("missing".to_string()),
        );

        let plain = capture_check_outcomes(&report);
        assert!(
            plain.contains("Pre-validation failed"),
            "PreCheck phase should render 'Pre-validation failed' header, got {plain:?}"
        );
        assert!(
            !plain.contains("Post-validation failed"),
            "PreCheck phase must not render 'Post-validation failed', got {plain:?}"
        );
    }

    #[test]
    fn failure_state_post_phase_uses_post_validation_failed() {
        let report = single_failure_report(
            FailurePhase::PostCheck,
            Some(RuleSource {
                file: std::path::PathBuf::from("/repo/prompts/run.md"),
                line_range: Some(7..=9),
                yaml_snippet: "file_exists: \"x\"\n".to_string(),
            }),
            Some("missing".to_string()),
        );

        let plain = capture_check_outcomes(&report);
        assert!(
            plain.contains("Post-validation failed"),
            "PostCheck phase should render 'Post-validation failed' header, got {plain:?}"
        );
        assert!(
            !plain.contains("Pre-validation failed"),
            "PostCheck phase must not render 'Pre-validation failed', got {plain:?}"
        );
    }

    #[test]
    fn failure_block_omits_line_range_suffix_when_unknown() {
        let source_file = std::path::PathBuf::from("/repo/prompts/run.md");
        let report = single_failure_report(
            FailurePhase::PreCheck,
            Some(RuleSource {
                file: source_file.clone(),
                line_range: None,
                yaml_snippet: "file_exists: \"x\"\n".to_string(),
            }),
            Some("missing".to_string()),
        );

        let plain = capture_check_outcomes(&report);
        let lines: Vec<&str> = plain.lines().collect();
        let source_line = lines
            .iter()
            .find(|l| l.contains("in "))
            .expect("expected an 'in <path>' source-location line");

        // The path display is cwd-relative when possible; ensure either the
        // absolute path or the file_name portion is present without any
        // `:N-M` numeric range suffix attached.
        let abs_display = source_file.display().to_string();
        let trimmed = source_line.trim_end();
        assert!(
            trimmed.ends_with(&abs_display)
                || trimmed.ends_with(
                    source_file.file_name().unwrap().to_string_lossy().as_ref()
                ),
            "source line should end at the file path with no ':N-M' suffix, got {trimmed:?}"
        );
        // Belt-and-suspenders: the source line must not contain a `:digit-digit`
        // pattern that would indicate a leaked line range.
        let range_re = regex::Regex::new(r":\d+-\d+").expect("static regex compiles");
        assert!(
            !range_re.is_match(source_line),
            "source line must not contain a ':N-M' line-range suffix when range is None, got {source_line:?}"
        );
    }

    #[test]
    fn legacy_failure_path_renders_two_lines_only() {
        let report = ValidationPhaseReport {
            phase: FailurePhase::PreCheck,
            outcomes: vec![ValidationCheckOutcome {
                rule_id: ValidationRuleId(0),
                event: ValidationEvent::DirExists,
                subject_key: None,
                passed: false,
                markup: "the directory /b exists".to_string(),
                failure_message: Some("not found".to_string()),
                source: None,
            }],
        };

        let plain = capture_check_outcomes(&report);
        let non_empty: Vec<&str> = plain.lines().filter(|l| !l.trim().is_empty()).collect();
        assert_eq!(
            non_empty.len(),
            2,
            "legacy no-source fallback should produce exactly two non-empty lines, got {non_empty:?}"
        );
    }

    #[test]
    fn render_failure_block_yaml_region_contains_sgr_styling() {
        // Positive assertion that the syntax-highlighted YAML path actually
        // fires for the canonical happy-path snippet. The renderer falls back
        // to plain unstyled lines when `highlight_yaml_lines` returns a line
        // count that doesn't match the plain count -- without this assertion
        // a degraded highlighter (broken theme load, missing yaml grammar,
        // empty palette) would silently strip styling and every other test
        // would still pass.
        let term = test_terminal();
        let outcome = ValidationCheckOutcome {
            rule_id: ValidationRuleId(0),
            event: ValidationEvent::FileExists,
            subject_key: Some("x".to_string()),
            passed: false,
            markup: "the file x exists".to_string(),
            failure_message: Some("missing".to_string()),
            source: Some(RuleSource {
                file: std::path::PathBuf::from("/repo/prompts/run.md"),
                line_range: Some(1..=2),
                yaml_snippet: "file_exists: \"x\"\nmessage: \"y\"\n".to_string(),
            }),
        };

        let mut sink = StringSink::default();
        render_failure_block_to(&mut sink, &outcome, FailurePhase::PreCheck, &term);
        let captured = sink.into_string();

        let lines: Vec<&str> = captured.lines().collect();
        let in_idx = lines
            .iter()
            .position(|l| l.contains("in ") && l.contains(":1-2"))
            .expect("expected source-location line containing ':1-2'");
        let reason_idx = lines
            .iter()
            .position(|l| l.contains("Reason:"))
            .expect("expected Reason line after the YAML region");
        let yaml_region = &lines[(in_idx + 1)..reason_idx];
        assert!(
            !yaml_region.is_empty(),
            "expected at least one YAML region line, got {lines:?}"
        );

        // `highlight_yaml_lines` emits truecolor foreground SGR escapes
        // (`\x1b[38;2;R;G;Bm`) for each highlighted token. Asserting on the
        // `\x1b[38;` introducer guarantees real styling fired -- not just a
        // bare reset like `\x1b[0m`.
        let styled_lines = yaml_region
            .iter()
            .filter(|l| l.contains("\x1b[38;"))
            .count();
        assert!(
            styled_lines > 0,
            "YAML region must contain at least one foreground-color SGR \
             ('\\x1b[38;...m'), got region: {yaml_region:?}"
        );
    }

    #[test]
    fn osc8_hyperlink_target_present_in_raw_output() {
        // Capture WITHOUT stripping ANSI: OSC-8 hyperlinks must survive into
        // the raw transcript so terminal emulators that support them can
        // present a clickable link to the failing rule's source file.
        let abs_path = "/repo/prompts/run.md";
        let report = single_failure_report(
            FailurePhase::PreCheck,
            Some(RuleSource {
                file: std::path::PathBuf::from(abs_path),
                line_range: Some(3..=4),
                yaml_snippet: "file_exists: \"x\"\n".to_string(),
            }),
            Some("missing".to_string()),
        );

        let term = test_terminal();
        let raw = report_check_outcomes_to_string(&report, &term);

        // OSC-8 introducer must appear, followed by a target whose URI
        // contains the rule's absolute source path. The link target may be a
        // bare path (`/repo/...`) or a `file://`-scheme URL (`file:///repo/...`)
        // depending on how the renderer formats hyperlink targets.
        let osc8_introducer = "\x1b]8;;";
        let intro_idx = raw
            .find(osc8_introducer)
            .expect("expected OSC-8 introducer '\\x1b]8;;' in raw output");
        let after_intro = &raw[intro_idx + osc8_introducer.len()..];
        // Extract the target portion: bytes after the introducer up to the
        // nearest ST (`\x1b\\`) or BEL (`\x07`) terminator.
        let st_idx = after_intro.find("\x1b\\").unwrap_or(after_intro.len());
        let bel_idx = after_intro.find('\x07').unwrap_or(after_intro.len());
        let target_end = st_idx.min(bel_idx);
        let target = &after_intro[..target_end];
        assert!(
            target.contains(abs_path),
            "OSC-8 link target should contain the absolute path {abs_path:?}, \
             got target: {target:?}"
        );
    }
}
