//! The shared expression diagnostics through composition's public results:
//! a category 3 call composes with a warning carrying the shared record, a
//! rejected call's error yields the record of its first category 5 finding,
//! and each record is located at its authored argument on every surface
//! (body, frontmatter, `::block when`, lifecycle). The records hold no
//! evaluated value, and producing them runs nothing.
//!
//! The editor leg of the same fixtures is DMLS's
//! `expression_call_diagnostics::shared_fields`.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use darkmatter::lifecycle::md_host;
use darkmatter::markdown::compose::expression::call_binding::FunctionRegistration;
use darkmatter::markdown::compose::expression::function_test_hooks as hooks;
use darkmatter::markdown::compose::expression::{
    Diagnostic, DiagnosticSource, INCOMPATIBLE_CALL_CODE, Severity, UNCERTAIN_CALL_CODE,
};
use darkmatter::markdown::compose::{ComposeContext, ComposeOptions, ComposeReport};
use darkmatter::markdown::schemas::DarkmatterSchemas;
use darkmatter::markdown::{Markdown, MarkdownError};
use serde_json::json;
use tempfile::TempDir;

fn compose_in(dir: &Path, content: &str) -> (PathBuf, Result<(Markdown, ComposeReport), MarkdownError>) {
    let path = dir.join("doc.md");
    std::fs::write(&path, content).unwrap();
    let options =
        ComposeOptions::new_with_context(ComposeContext::capture_for_content(dir, "")).with_source_file(path.clone());
    let result = Markdown::try_from(path.as_path())
        .unwrap()
        .compose_with(&crate::request_support::request_at(dir, options));
    (path, result)
}

/// The one-based `(line, column)` of the `nth` occurrence of `needle` in
/// `text` (ASCII fixtures).
fn position_of(text: &str, needle: &str, nth: usize) -> (usize, usize) {
    let offset = text.match_indices(needle).nth(nth).unwrap_or_else(|| panic!("no `{needle}` #{nth}")).0;
    let before = &text[..offset];
    (before.matches('\n').count() + 1, offset - before.rfind('\n').map_or(0, |index| index + 1) + 1)
}

fn at(path: &Path, (line, column): (usize, usize)) -> Option<DiagnosticSource> {
    Some(DiagnosticSource { file: Some(path.to_path_buf()), line, column })
}

/// Criterion 20, severity part: a category 3 call composes, and each such
/// argument is one warning whose shared record is the uncertain-call
/// warning located at the argument, in the frontmatter and the body alike.
#[test]
fn a_category_3_call_composes_with_a_located_warning() {
    let dir = TempDir::new().unwrap();
    let content = "---\n$schema:\n  label: string\nlabel: \"7\"\ntitle: \"{{ min(label, 9) }}\"\n---\nBody {{ min(label, 5) }}\n";
    let (path, result) = compose_in(dir.path(), content);
    let (composed, report) = result.unwrap_or_else(|error| panic!("composes: {error}"));
    assert_eq!(composed.content().trim_end(), "Body 5");
    let found: Vec<&Diagnostic> = report.warnings.iter().filter_map(|warning| warning.diagnostic.as_deref()).collect();
    // An optional `label` may be absent, so its type includes `null`.
    let message = "min: argument 1 (`a`) is string | null, expected number (category 3: some values of that type convert and others do not)";
    let expected = |position| Diagnostic {
        code: UNCERTAIN_CALL_CODE.to_string(),
        severity: Severity::Warning,
        message: message.to_string(),
        source: at(&path, position),
        expected_type: Some("number".to_string()),
        actual_type: Some("string | null".to_string()),
        span: None,
        iteration_path: Default::default(),
    };
    assert_eq!(found, [&expected(position_of(content, "label, 9", 0)), &expected(position_of(content, "label, 5", 0))]);
    let warning = report.warnings.iter().find(|warning| warning.diagnostic.is_some()).unwrap();
    assert_eq!(warning.code.as_deref(), Some(UNCERTAIN_CALL_CODE));
    assert_eq!(warning.line_number, Some(5));
    assert!(warning.message.starts_with("`min(…)` at "), "{}", warning.message);
}

/// Criterion 19, composition leg: the rejection's record is the first
/// category 5 finding, located at the argument, for a body interpolation, a
/// frontmatter value, and a `::block when` condition; a quoted scalar whose
/// authored bytes differ from its value is located at its construct.
#[test]
fn a_rejected_call_yields_its_located_diagnostic_on_every_surface() {
    let schema = "---\n$schema:\n  flag: boolean(required)\nflag: true\n";
    for (label, content, needle) in [
        ("body", format!("{schema}---\n\nBody {{{{ min(flag, 5) }}}}\n"), "flag, 5"),
        ("frontmatter", format!("{schema}title: \"{{{{ min(flag, 2) }}}}\"\n---\nBody\n"), "flag, 2"),
        ("condition", format!("{schema}---\n\n::block when=\"min(flag, 3) > 1\"\nx\n::end-block\n"), "flag, 3"),
        // An escape outside the construct leaves its text verbatim; one
        // inside it (`\x34` is `4`) does not, so the construct's start is used.
        ("escaped outside", format!("{schema}title: \"\\x41{{{{ min(flag, 4) }}}}\"\n---\nBody\n"), "flag, 4"),
        ("escaped inside", format!("{schema}title: \"{{{{ min(flag, \\x34) }}}}\"\n---\nBody\n"), "{{ min(flag"),
    ] {
        let dir = TempDir::new().unwrap();
        let (path, result) = compose_in(dir.path(), &content);
        let error = result.err().unwrap_or_else(|| panic!("{label}: composed"));
        let diagnostic = error.call_diagnostic().unwrap_or_else(|| panic!("{label}: no call diagnostic: {error}"));
        assert_eq!(diagnostic.code, INCOMPATIBLE_CALL_CODE, "{label}");
        assert_eq!(diagnostic.severity, Severity::Error, "{label}");
        assert_eq!(diagnostic.expected_type.as_deref(), Some("number"), "{label}");
        assert_eq!(diagnostic.actual_type.as_deref(), Some("boolean"), "{label}");
        assert!(diagnostic.message.starts_with("min: argument 1 (`a`) is boolean"), "{label}: {}", diagnostic.message);
        assert_eq!(diagnostic.source, at(&path, position_of(&content, needle, 0)), "{label}");
    }
}

/// A lifecycle surface reports its category 3 warning through the same
/// record, located at the argument inside the event's scalar; a rejected
/// lifecycle call still rejects.
#[test]
fn a_lifecycle_surface_reports_the_same_located_warning() {
    let dir = TempDir::new().unwrap();
    let content = "---\n$schema:\n  label: string\nlabel: \"7\"\ncomposed:\n  - stdout: \"{{ min(label, 5) }}\"\n---\nBody\n";
    let path = dir.path().join("doc.md");
    std::fs::write(&path, content).unwrap();
    let markdown = Markdown::try_from(path.as_path()).unwrap();
    let frontmatter: serde_json::Value =
        markdown.frontmatter().as_map().iter().map(|(key, value)| (key.clone(), value.clone())).collect();
    let config = md_host::parse_lifecycle(&frontmatter, &path, None).expect("the lifecycle parses");
    let schema = DarkmatterSchemas::new(crate::request_support::context_at(dir.path()))
        .effective_for(&markdown)
        .unwrap()
        .expect("a schema");
    let warnings = darkmatter::lifecycle::reject_incompatible_calls(
        &frontmatter,
        &config,
        &path,
        markdown.loaded_text(),
        md_host::registry(),
        Some(&schema),
    )
    .expect("a category 3 call is not rejected");
    let found: Vec<Diagnostic> = warnings.iter().filter_map(|warning| warning.diagnostic.as_deref().cloned()).collect();
    assert_eq!(found.len(), 1, "{warnings:#?}");
    assert_eq!(found[0].code, UNCERTAIN_CALL_CODE);
    assert_eq!(found[0].source, at(&path, position_of(content, "label, 5", 0)));
    assert!(warnings[0].message.contains("lifecycle `composed[0].stdout.text`"), "{}", warnings[0].message);

    let rejected = content.replace("min(label, 5)", "min(true, 5)");
    std::fs::write(&path, &rejected).unwrap();
    let markdown = Markdown::try_from(path.as_path()).unwrap();
    let frontmatter: serde_json::Value =
        markdown.frontmatter().as_map().iter().map(|(key, value)| (key.clone(), value.clone())).collect();
    let config = md_host::parse_lifecycle(&frontmatter, &path, None).expect("the lifecycle parses");
    assert!(
        darkmatter::lifecycle::reject_incompatible_calls(&frontmatter, &config, &path, None, md_host::registry(), None)
            .is_err()
    );
}

/// Criterion 22: a diagnostic carries types and authored text only. The
/// frontmatter value, the environment value, and the referenced file's
/// contents behind a flagged call appear in no record, and producing the
/// records (a call in a branch that never runs included) enters no handler
/// and attempts no network access.
#[test]
fn diagnostics_hold_no_evaluated_value_and_run_nothing() {
    const SECRET: &str = "s3cr3t-value";
    let registration = FunctionRegistration::builder("probe", "function(parameters(v: number); returns(number))")
        .build()
        .expect("the test signature compiles");
    let _probe = hooks::register(registration, Arc::new(|_| Ok(json!(1))));

    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("other.md"), format!("---\ntitle: {SECRET}\n---\n{SECRET}\n")).unwrap();
    let content = format!(
        "---\n$schema:\n  label: string\n  flag: boolean(required)\nlabel: \"{SECRET}\"\nflag: true\n---\n\
         {{{{ false ? probe(label) : 0 }}}} {{{{ false ? probe(env.SECRET_VALUE) : 0 }}}} {{{{ true ? 0 : min(flag, 1) }}}}\n"
    );
    hooks::reset_handler_entries();
    let network = darkmatter::effects::network_attempt_count();
    let path = dir.path().join("doc.md");
    std::fs::write(&path, &content).unwrap();
    let mut context = ComposeContext::capture_for_content(dir.path(), "");
    context.env_mut().insert("SECRET_VALUE".to_string(), SECRET.to_string());
    let options = ComposeOptions::new_with_context(context).with_source_file(path.clone());
    let error = Markdown::try_from(path.as_path())
        .unwrap()
        .compose_with(&crate::request_support::request_at(dir.path(), options))
        .expect_err("the dead-branch category 5 call rejects the document");
    let rejected = error.call_diagnostic().expect("a call diagnostic");

    // The non-rejecting calls of the same text, as composition reports them.
    let (_, result) = compose_in(dir.path(), &content.replace("min(flag, 1)", "0"));
    let (_, report) = result.unwrap_or_else(|error| panic!("composes: {error}"));
    let warnings: Vec<Diagnostic> = report.warnings.iter().filter_map(|warning| warning.diagnostic.as_deref().cloned()).collect();
    assert_eq!(warnings.len(), 1, "probe(label) is category 3: {:#?}", report.warnings);

    for record in std::iter::once(&rejected).chain(&warnings) {
        let serialized = serde_json::to_string(record).unwrap();
        assert!(!serialized.contains(SECRET), "{serialized}");
    }
    assert_eq!(hooks::handler_entries("probe"), 0, "no handler ran");
    assert_eq!(hooks::handler_entries("min"), 0, "no handler ran");
    assert_eq!(darkmatter::effects::network_attempt_count(), network);
}

/// A transcluded child is checked against its own source, so its warning's
/// record names the child file and the child's line, not the parent's.
#[test]
fn a_transcluded_child_warning_is_located_in_the_child() {
    let dir = TempDir::new().unwrap();
    let child = "---\n$schema:\n  label: string\nlabel: \"7\"\n---\nChild {{ min(label, 5) }}\n";
    let child_path = dir.path().join("child.md");
    std::fs::write(&child_path, child).unwrap();
    let (_, result) = compose_in(dir.path(), "---\ntitle: parent\n---\nBefore\n\n::file ./child.md\n");
    let (composed, report) = result.unwrap_or_else(|error| panic!("composes: {error}"));
    assert!(composed.content().contains("Child 5"), "{}", composed.content());
    let found: Vec<&Diagnostic> = report.warnings.iter().filter_map(|warning| warning.diagnostic.as_deref()).collect();
    assert_eq!(found.len(), 1, "{:#?}", report.warnings);
    let source = found[0].source.as_ref().expect("located");
    let file = source.file.as_deref().expect("a file");
    assert_eq!(file.file_name(), child_path.file_name(), "{file:?}");
    assert_eq!((source.line, source.column), position_of(child, "label, 5", 0));
}

// ── collection sites ─────────────────────────────────────────────────────

/// `(surface, opener tail, body)` for a `min(flag, 5)` call at every place a
/// `::map` / `::filter` holds an expression. The callbacks carry an escaped
/// quote and a multibyte character ahead of the call, so a position projected
/// from the decoded text would be off.
const COLLECTION_SURFACES: &[(&str, &str, &str)] = &[
    ("positional callback", "data \"i => i == \\\"é\\\" || min(flag, 5) > 1\"", "x\n"),
    ("filter option", "data filter=\"i => i == \\\"é\\\" || min(flag, 5) > 1\"", "x\n"),
    ("sort option", "data sort=\"i => min(flag, 5)\"", "x\n"),
    ("while option", "data while=\"i => min(flag, 5) > 1\"", "x\n"),
    ("until option", "data until=\"i => min(flag, 5) > 1\"", "x\n"),
    ("gate", "data when=\"min(flag, 5) > 1\"", "x\n"),
    ("body", "data", "{{ min(flag, 5) }}\n"),
];

const COLLECTION_SCHEMA: &str = "---\n$schema:\n  flag: boolean(required)\n  label: string\n  data: any[](required)\nflag: true\nlabel: \"7\"\ndata: [1, 2, 3]\n---\n";

/// [`position_of`] counting columns in characters, for fixtures with multibyte text.
fn char_position_of(text: &str, needle: &str) -> (usize, usize) {
    let offset = text.find(needle).unwrap_or_else(|| panic!("no `{needle}`"));
    let before = &text[..offset];
    let line_start = before.rfind('\n').map_or(0, |index| index + 1);
    (before.matches('\n').count() + 1, before[line_start..].chars().count() + 1)
}

/// The rejection's record is located at the authored argument on every
/// collection surface, for both kinds, and the call never ran.
#[test]
fn a_rejected_call_in_a_collection_is_located_at_its_authored_argument() {
    for (opener, closer) in [("map", "end-map"), ("filter", "end-filter")] {
        for (surface, tail, body) in COLLECTION_SURFACES {
            let content = format!("{COLLECTION_SCHEMA}::{opener} {tail}\n{body}::{closer}\n");
            let dir = TempDir::new().unwrap();
            let (path, result) = compose_in(dir.path(), &content);
            let error = result.err().unwrap_or_else(|| panic!("{opener} {surface}: composed"));
            let diagnostic = error.call_diagnostic().unwrap_or_else(|| panic!("{opener} {surface}: no call diagnostic: {error}"));
            assert_eq!(diagnostic.code, INCOMPATIBLE_CALL_CODE, "{opener} {surface}");
            assert_eq!(diagnostic.severity, Severity::Error, "{opener} {surface}");
            assert_eq!(diagnostic.source, at(&path, char_position_of(&content, "flag, 5")), "{opener} {surface}");
        }
    }
}

/// A category 3 call in a callback or a body is one warning per authored
/// site, located at the argument, however many items the region visits.
#[test]
fn an_uncertain_call_in_a_collection_warns_once_per_site_at_its_argument() {
    for (opener, closer) in [("map", "end-map"), ("filter", "end-filter")] {
        let content = format!(
            "{COLLECTION_SCHEMA}::{opener} data \"i => min(label, 5) > 1 ? i : i\"\n{{{{ min(label, 6) }}}}\n::{closer}\n"
        );
        let dir = TempDir::new().unwrap();
        let (path, result) = compose_in(dir.path(), &content);
        let (_, report) = result.unwrap_or_else(|error| panic!("{opener}: composes: {error}"));
        let found: Vec<&Diagnostic> = report.warnings.iter().filter_map(|warning| warning.diagnostic.as_deref()).collect();
        let sources: Vec<_> = found.iter().map(|diagnostic| diagnostic.source.clone()).collect();
        assert_eq!(
            sources,
            [at(&path, position_of(&content, "label, 5", 0)), at(&path, position_of(&content, "label, 6", 0))],
            "{opener}: {found:#?}"
        );
        assert!(found.iter().all(|diagnostic| diagnostic.code == UNCERTAIN_CALL_CODE), "{opener}");
    }
}
