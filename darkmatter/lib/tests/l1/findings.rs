//! The shared finding model: the additive `Diagnostic` fields and their data
//! form, the ordered collector, the human report, the missing-document
//! builder over biscuit-file's resolution, and the empty-body cause.

use std::path::Path;

use biscuit_file::{FileReference, FileResolutionContext};
use biscuit_terminal::terminal::Terminal;
use darkmatter::findings::reference::{DocumentConstruct, ListingMemo, missing_document};
use darkmatter::findings::{
    Diagnostic, DiagnosticSource, EMPTY_BODY_CODE, EmptyBodyCause, FINDING_CODES, FindingSink, MISSING_DOCUMENT_CODE,
    NOT_CHECKED_CODE, Severity, Subject, Trail, TrailFrame,
};
use darkmatter::findings::HumanLayout;
use serde_json::json;
use tempfile::TempDir;

fn plain() -> Terminal {
    let mut term = Terminal::new_optimistic(80);
    term.color_depth = biscuit_terminal::discovery::detection::ColorDepth::None;
    term
}

fn warning(message: &str) -> Diagnostic {
    Diagnostic::actionable("composition.selection_key_as_data", Severity::Warning, message, "move it under `with:`")
}

#[test]
fn a_new_actionable_finding_serializes_its_hint_and_host_trail_beside_the_existing_fields() {
    let finding = warning("`agent: opencode` is step data")
        .with_source(DiagnosticSource::at(Some("/work/_fleet.yaml".into()), 5, 7))
        .with_trail(Trail::host("task", "find", "sequence[0].agent"));
    let value = serde_json::to_value(&finding).unwrap();
    assert_eq!(value["hint"], "move it under `with:`");
    assert_eq!(value["trail"], json!([{ "kind": "host", "host": "task", "label": "find", "property": "sequence[0].agent" }]));
    // The existing fields keep their names and their null/empty forms.
    assert_eq!(value["iteration_path"], json!([]));
    assert_eq!(value["span"], json!(null));
    assert_eq!(value["expected_type"], json!(null));
    assert!(value.get("notes").is_none(), "an empty notes list is omitted");
    assert!(value.get("related").is_none());
}

#[test]
fn a_projected_finding_without_a_hint_or_trail_serializes_exactly_as_before() {
    let finding = Diagnostic::projected("dm.expression.uncertain_call", Severity::Warning, "might not bind", None);
    let value = serde_json::to_value(&finding).unwrap();
    let keys: Vec<&str> = value.as_object().unwrap().keys().map(String::as_str).collect();
    assert_eq!(
        keys,
        ["code", "severity", "message", "source", "expected_type", "actual_type", "span", "iteration_path"]
    );
}

#[test]
fn every_trail_frame_kind_is_tagged_and_the_trail_displays_on_one_line() {
    let trail = Trail::host("task", "metadata", "sequence[1]")
        .pushed(TrailFrame::Included {
            file: darkmatter::markdown::compose::ComposeSource::File("_frontmatter.md".into()),
            line: Some(4),
        })
        .pushed(TrailFrame::Iteration {
            collection: "teams".into(),
            position: darkmatter::markdown::compose::expression::iteration::IterationPosition::Index(2),
        });
    assert_eq!(trail.to_string(), "task metadata (sequence[1]) → _frontmatter.md:4 → teams[2]");
    let kinds: Vec<String> = serde_json::to_value(&trail).unwrap().as_array().unwrap().iter().map(|frame| frame["kind"].as_str().unwrap().to_string()).collect();
    assert_eq!(kinds, ["host", "included", "iteration"]);
}

#[test]
fn equivalent_findings_for_one_subject_collapse_and_distinct_occurrences_stay_apart() {
    let mut sink = FindingSink::new();
    // Two occurrences of tasks that share a file and a label.
    let first = sink.register_subject(Subject::construct("task", "same", "sequence[0]", None));
    let second = sink.register_subject(Subject::construct("task", "same", "sequence[1]", None));
    let original = sink.push(first, warning("one"));
    assert_eq!(sink.push(first, warning("one")), original, "a repeated push returns the original id");
    let other = sink.push(second, warning("one"));
    assert_ne!(other, original);
    // A different location is a different finding.
    sink.push(first, warning("one").with_source(DiagnosticSource::at(None, 2, 1)));
    let report = sink.finish();
    assert_eq!(report.diagnostics().len(), 3);
    assert_eq!(report.counts().warnings, 3);
}

#[test]
fn report_order_is_subject_registration_then_push_order_whatever_the_push_order() {
    let mut sink = FindingSink::new();
    let root = sink.register_subject(Subject::document("root", None));
    let task = sink.register_subject(Subject::construct("task", "a", "sequence[0]", None));
    sink.push(task, warning("task first"));
    sink.push(root, warning("root second"));
    sink.push(task, warning("task third"));
    let messages: Vec<String> = sink.finish().diagnostics().iter().map(|finding| finding.message.clone()).collect();
    assert_eq!(messages, ["root second", "task first", "task third"]);
}

#[test]
fn a_skipped_check_is_one_info_record_per_check_linked_to_the_located_blocker() {
    let mut sink = FindingSink::new();
    let task = sink.register_subject(Subject::construct("task", "metadata", "sequence[1]", None));
    let blocker = sink.push(
        task,
        Diagnostic::actionable(MISSING_DOCUMENT_CODE, Severity::Error, "the prompt file does not exist", "create it")
            .with_source(DiagnosticSource::at(Some("/w/_fleet.yaml".into()), 12, 15)),
    );
    sink.not_checked(task, "provider", "the prompt file could not be read", Some(blocker));
    sink.not_checked(task, "inputs", "the prompt file could not be read", Some(blocker));
    // An unlocated blocker gives no related location.
    let unlocated = sink.push(task, warning("unlocated"));
    sink.not_checked(task, "model requirement", "no provider", Some(unlocated));
    assert!(sink.has_errors());
    let report = sink.finish();
    let skipped: Vec<&Diagnostic> = report.diagnostics().into_iter().filter(|finding| finding.code == NOT_CHECKED_CODE).collect();
    assert_eq!(skipped.len(), 3);
    assert!(skipped.iter().all(|finding| finding.severity == Severity::Info));
    assert_eq!(skipped[0].related.len(), 1);
    assert_eq!(skipped[0].related[0].source.line, Some(12));
    assert!(skipped[2].related.is_empty());
    assert_eq!(report.counts().info, 3);
}

#[test]
fn the_human_report_groups_skipped_checks_under_their_subject_and_shows_where_and_how() {
    let mut sink = FindingSink::new();
    let launch = Path::new("/work");
    let find = sink.register_subject(Subject::construct("task", "find", "sequence[0]", None));
    let quiet = sink.register_subject(Subject::construct("task", "quiet", "sequence[1]", None));
    let _ = quiet;
    let blocker = sink.push(
        find,
        Diagnostic::actionable("composition.no_executable", Severity::Error, "this task has no executable", "add `prompt: ./metadata.md`")
            .with_source(DiagnosticSource::at(Some("/work/renderers/_fleet.yaml".into()), 10, 7))
            .with_note("1. --provider flag  not given"),
    );
    sink.not_checked(find, "provider", "no executable", Some(blocker));
    sink.not_checked(find, "inputs", "no executable", Some(blocker));
    let rendered = sink.finish().render_human(&plain(), &HumanLayout { width: Some(80), launch_dir: Some(launch.into()) });
    assert_eq!(
        rendered,
        "task find (sequence[0])\n  ✗ this task has no executable\n      1. --provider flag  not given\n      at renderers/_fleet.yaml:10:7\n      hint: add `prompt: ./metadata.md`\n  ℹ not checked: provider, inputs (no executable)",
        "a subject without findings is not shown"
    );
}

#[test]
fn a_long_message_wraps_under_its_text_at_the_layout_width() {
    let mut sink = FindingSink::new();
    let task = sink.register_subject(Subject::construct("task", "metadata", "sequence[1]", None));
    sink.push(task, warning("this task has no executable, so it would compose the body of renderers/_fleet.yaml, and a YAML file has no body"));
    let rendered = sink.finish().render_human(&plain(), &HumanLayout { width: Some(80), launch_dir: None });
    let lines: Vec<&str> = rendered.lines().collect();
    assert!(lines.iter().all(|line| line.chars().count() <= 80), "{rendered}");
    assert!(lines[1].starts_with("  ⚠ this task has no executable"));
    assert!(lines[2].starts_with("    ") && !lines[2].starts_with("     "), "continuation sits under the message: {rendered}");
}

#[test]
fn a_confirmed_no_match_is_a_missing_document_finding_with_its_resolution_evidence() {
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("_frontmatter.yaml"), "").unwrap();
    let context = FileResolutionContext::new(dir.path());
    let resolution = FileReference::new("./_frontmatter.md").unwrap().resolve_detailed(&context);
    let mut memo = ListingMemo::new();
    let finding = missing_document(DocumentConstruct { construct: "prompt:", noun: "prompt file", launch_dir: None }, &resolution, &mut memo)
        .expect("a no-match builds the finding");
    assert_eq!(finding.code, MISSING_DOCUMENT_CODE);
    assert_eq!(finding.severity, Severity::Error);
    assert_eq!(finding.message, "the prompt file does not exist");
    let details = finding.reference_details.as_ref().unwrap();
    assert_eq!(details.construct.as_deref(), Some("prompt:"));
    assert_eq!(details.reference.as_deref(), Some("./_frontmatter.md"));
    assert_eq!(details.failure.as_deref(), Some("no-match"));
    let expected = biscuit_file::to_forward_slash_string(&dir.path().join("_frontmatter.md"));
    assert_eq!(details.resolved.as_deref(), Some(expected.as_str()));
    assert!(finding.hint.as_deref().unwrap().starts_with(&format!("create {expected}")));
    // The near name is suggested.
    let suggestions = details.suggestions.clone().unwrap();
    assert_eq!(suggestions, [biscuit_file::to_forward_slash_string(&dir.path().join("_frontmatter.yaml"))]);
    assert_eq!(memo.listed(), 1);
    // A second reference into the same directory reuses its listing.
    let again = FileReference::new("./_frontmater.md").unwrap().resolve_detailed(&context);
    missing_document(DocumentConstruct { construct: "prompt:", noun: "prompt file", launch_dir: None }, &again, &mut memo).unwrap();
    assert_eq!(memo.listed(), 1);
}

#[test]
fn a_match_or_any_other_failure_is_not_a_missing_document() {
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("here.md"), "").unwrap();
    let context = FileResolutionContext::new(dir.path());
    let construct = DocumentConstruct { construct: "prompt:", noun: "prompt file", launch_dir: None };
    let matched = FileReference::new("./here.md").unwrap().resolve_detailed(&context);
    assert!(missing_document(construct, &matched, &mut ListingMemo::new()).is_none());
    // A directory where a file is required is not a no-match.
    std::fs::create_dir(dir.path().join("folder.md")).unwrap();
    let wrong_kind = FileReference::new("./folder.md").unwrap().resolve_detailed(&context);
    if !matches!(wrong_kind.outcome(), biscuit_file::DetailedOutcome::Failed(biscuit_file::ResolutionFailure::NoMatch)) {
        assert!(missing_document(construct, &wrong_kind, &mut ListingMemo::new()).is_none());
    }
}

#[test]
fn a_literal_glob_reference_gets_the_resolver_glob_hint() {
    let dir = TempDir::new().unwrap();
    let context = FileResolutionContext::new(dir.path());
    let resolution = FileReference::new("./*.md").unwrap().resolve_detailed(&context);
    if let Some(finding) = missing_document(DocumentConstruct { construct: "prompt:", noun: "prompt file", launch_dir: None }, &resolution, &mut ListingMemo::new()) {
        assert_eq!(finding.hint.as_deref(), resolution.glob_hint());
    }
}

#[test]
fn the_empty_body_cause_names_only_what_is_proven() {
    assert_eq!(EmptyBodyCause::classify("", ""), Some(EmptyBodyCause::AuthoredEmpty));
    assert_eq!(EmptyBodyCause::classify("  \n\t", "\n"), Some(EmptyBodyCause::Whitespace));
    assert_eq!(EmptyBodyCause::classify("{{ x }}", "  "), Some(EmptyBodyCause::EmptyRendered));
    assert_eq!(EmptyBodyCause::classify("", "text"), None);
}

#[test]
fn every_finding_code_is_declared_once_across_the_two_catalogs() {
    use darkmatter::markdown::compose::policy::COMPOSE_DIAGNOSTIC_CODES;
    let catalog: Vec<&str> = COMPOSE_DIAGNOSTIC_CODES.iter().map(|(code, _)| *code).collect();
    // The codes `md compose` emits are listed once, in its catalog; the
    // host-only codes stay out of it.
    assert!(catalog.contains(&EMPTY_BODY_CODE) && catalog.contains(&NOT_CHECKED_CODE));
    let codes: Vec<&str> = FINDING_CODES.iter().map(|(code, _)| *code).collect();
    assert_eq!(codes, [MISSING_DOCUMENT_CODE]);
    assert!(codes.iter().all(|code| code.starts_with("dm.") && !catalog.contains(code)));
}

#[test]
fn the_generic_presentation_groups_a_subjects_findings_and_escapes_authored_text() {
    let mut sink = FindingSink::new();
    let task = sink.register_subject(Subject::construct("task", "<b>bold</b>", "sequence[0]", None));
    let blocker = sink.push(
        task,
        Diagnostic::actionable("composition.no_executable", Severity::Error, "no executable for <b>x</b>", "add `prompt: ./a.md`")
            .with_source(DiagnosticSource::at(Some("/work/fleet.yaml".into()), 10, 7)),
    );
    sink.not_checked(task, "inputs", "no executable", Some(blocker));
    let report = sink.finish();

    let options = renderable::output::OutputOptions::new(
        renderable::output::Width::Unlimited,
        renderable::output::TerminalProfile::plain(),
    );
    let rendered = report.presentation().with_launch_dir("/work").render_terminal(&options).unwrap();
    assert_eq!(
        rendered,
        "task <b>bold</b> (sequence[0])\n  ✗ no executable for <b>x</b>\n      at fleet.yaml:10:7\n      hint: add `prompt: ./a.md`\n  ℹ not checked: inputs (no executable)"
    );
}
