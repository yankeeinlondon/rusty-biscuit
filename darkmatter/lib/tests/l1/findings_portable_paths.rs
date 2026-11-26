//! Path spelling in the shared findings: the reference fields of a
//! diagnostic's data form (`base`, `resolved`, `searched`, `suggestions`) are
//! `biscuit_file::to_forward_slash_string` text, the human report shows a
//! path under the launch directory relative to it with `/`, an `@` miss
//! lists its search roots in resolver order, and a CRLF document reports the
//! same line and column as its LF original.
//!
//! The `windows` module holds the cases that need Windows path semantics:
//! drive-letter absolutes, authored `\` separators, and a UNC base. They are
//! lexical: the UNC case renders a finding built from the spellings and never
//! probes a share.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use biscuit_file::{FileReference, FileResolutionContext, PathPosition, to_forward_slash_string};
use biscuit_terminal::terminal::Terminal;
use darkmatter::findings::reference::{DocumentCheck, DocumentConstruct, ListingMemo, check_document};
use darkmatter::findings::{Diagnostic, DiagnosticSource, FindingSink, HumanLayout, Subject};
use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::ComposeOptions;
use darkmatter::markdown::compose::report_diagnostics::reference_issue_diagnostic;
use darkmatter::markdown::reference::ReferenceGraphOptions;
use darkmatter::markdown::reference::validate::ReferenceValidationOptions;
use tempfile::TempDir;

fn plain() -> Terminal {
    let mut term = Terminal::new_optimistic(80);
    term.color_depth = biscuit_terminal::discovery::detection::ColorDepth::None;
    term
}

/// A temporary directory spelled as the resolver spells it (no 8.3 alias,
/// no `/var` symlink).
fn root() -> (TempDir, PathBuf) {
    let dir = TempDir::new().unwrap();
    let root = biscuit_file::canonicalize_simplified(dir.path()).unwrap();
    (dir, root)
}

fn admit_all(_: &Path) -> Result<(), String> {
    Ok(())
}

/// The missing-document finding for `raw`, its hint written for `launch`.
fn missing(raw: &str, context: &FileResolutionContext, launch: &Path) -> Diagnostic {
    let construct = DocumentConstruct { construct: "prompt:", noun: "prompt file", launch_dir: Some(launch) };
    match check_document(&FileReference::new(raw).unwrap(), context, construct, &mut ListingMemo::new(), admit_all) {
        DocumentCheck::Missing(finding) => *finding,
        other => panic!("expected a missing document for {raw}, got {other:?}"),
    }
}

/// The human report of `finding` alone, paths shown relative to `launch`.
fn human(finding: Diagnostic, launch: &Path) -> String {
    let mut sink = FindingSink::new();
    let subject = sink.register_subject(Subject::document("doc", None));
    sink.push(subject, finding);
    sink.finish().render_human(&plain(), &HumanLayout { width: None, launch_dir: Some(launch.to_path_buf()) })
}

/// The reference path fields of `finding`'s data form, in field order.
fn reference_paths(finding: &Diagnostic) -> Vec<String> {
    let value = serde_json::to_value(finding).unwrap();
    let mut paths = Vec::new();
    for key in ["base", "resolved"] {
        if let Some(text) = value[key].as_str() {
            paths.push(text.to_string());
        }
    }
    for key in ["searched", "suggestions"] {
        if let Some(items) = value[key].as_array() {
            paths.extend(items.iter().map(|item| item.as_str().unwrap().to_string()));
        }
    }
    paths
}

#[test]
fn a_nested_relative_miss_is_portable_data_and_a_relative_slash_path_for_a_person() {
    let (_dir, root) = root();
    let docs = root.join("docs");
    fs::create_dir_all(docs.join("parts")).unwrap();
    fs::write(docs.join("parts/notez.md"), "near miss").unwrap();
    let context = FileResolutionContext::from_snapshot(&docs, None, HashMap::new());

    let finding = missing("./parts/notes.md", &context, &root)
        .with_source(DiagnosticSource::at(Some(docs.join("guide.md")), 3, 7));
    let value = serde_json::to_value(&finding).unwrap();
    assert_eq!(value["resolved"], to_forward_slash_string(&docs.join("parts").join("notes.md")), "{value:#}");
    assert_eq!(value["base"], to_forward_slash_string(&docs), "{value:#}");
    assert_eq!(value["suggestions"][0], to_forward_slash_string(&docs.join("parts").join("notez.md")), "{value:#}");
    assert!(reference_paths(&finding).iter().all(|path| !path.contains('\\')), "{value:#}");

    let shown = human(finding, &root);
    for expected in [
        "resolved to docs/parts/notes.md (relative to docs)",
        "at docs/guide.md:3:7",
        "hint: create docs/parts/notes.md before a real run",
    ] {
        assert!(shown.contains(expected), "missing `{expected}` in:\n{shown}");
    }
}

#[test]
fn an_at_miss_lists_its_search_roots_in_resolver_order() {
    let (_dir, root) = root();
    let launch = root.join("launch");
    let home = root.join("home");
    let prepended = launch.join("first");
    for directory in [&launch, &home, &prepended] {
        fs::create_dir_all(directory).unwrap();
    }
    let context = FileResolutionContext::from_snapshot(&launch, Some(home.clone()), HashMap::new())
        .add_magic_path(&prepended, PathPosition::Start);
    let roots: Vec<PathBuf> = context.magic_search_roots().iter().map(|root| root.path().to_path_buf()).collect();
    assert_eq!(roots, [prepended, launch.clone(), home], "local prepends, the local root, then home");

    let finding = missing("@notes.md", &context, &launch);
    let expected: Vec<String> = roots.iter().map(|root| to_forward_slash_string(&root.join("notes.md"))).collect();
    assert_eq!(finding.reference_details.as_ref().unwrap().searched.as_ref(), Some(&expected));
    assert_eq!(finding.notes, ["searched 3 locations"]);
}

/// `(line, column)` of each reference issue `text` reports, as its data form
/// and as the human report's `file:line:column`.
fn transclusion_positions(root: &Path, name: &str, text: &str) -> (Vec<(Option<usize>, Option<usize>)>, String) {
    let path = root.join(name);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, text).unwrap();
    let options = ReferenceValidationOptions::with_graph(ReferenceGraphOptions::with_compose(
        &crate::request_support::request_at(root, ComposeOptions::new()),
    ));
    let report = Markdown::try_from(path.as_path()).unwrap().validate_references(options).unwrap();
    let positions = report
        .issues
        .iter()
        .map(|issue| {
            let diagnostic = reference_issue_diagnostic(issue);
            let source = diagnostic.source.expect("a reference issue is located");
            (source.line, source.column)
        })
        .collect();
    let shown = darkmatter::findings::ReferencePresentation::new(report)
        .with_launch_dir(root)
        .render_terminal(&renderable::output::OutputOptions::new(
            renderable::output::Width::Unlimited,
            renderable::output::TerminalProfile::plain(),
        ))
        .unwrap();
    (positions, shown)
}

#[test]
fn a_crlf_document_reports_the_same_line_and_column_as_lf() {
    let (_dir, root) = root();
    let lf = "---\ntitle: Guide\n---\n\n# Guide\n\nCafé intro.\n\n::file ./parts/missing.md\n";
    let (lf_positions, lf_shown) = transclusion_positions(&root, "docs/lf/guide.md", lf);
    let (crlf_positions, crlf_shown) = transclusion_positions(&root, "docs/crlf/guide.md", &lf.replace('\n', "\r\n"));

    assert_eq!(lf_positions, [(Some(9), Some(1))], "{lf_shown}");
    assert_eq!(crlf_positions, lf_positions, "{crlf_shown}");
    // The document heading is the launch-relative path; its rows name the
    // file under it.
    assert!(lf_shown.contains("\ndocs/lf/guide.md\n") && lf_shown.contains("  guide.md:9:1"), "{lf_shown}");
    assert!(crlf_shown.contains("\ndocs/crlf/guide.md\n") && crlf_shown.contains("  guide.md:9:1"), "{crlf_shown}");
    assert!(!crlf_shown.contains('\\'), "{crlf_shown}");
}

#[cfg(windows)]
mod windows {
    //! Native Windows spellings at the findings' public API.

    use super::*;

    /// A UNC base nothing probes: the finding is built from its spellings.
    const UNC_BASE: &str = r"\\server\share\repo";

    #[test]
    fn a_drive_letter_absolute_reference_is_slash_data_and_relative_for_a_person() {
        let (_dir, root) = root();
        let absolute = root.join("sub").join("x.md");
        let raw = absolute.display().to_string();
        assert_eq!(raw.as_bytes()[1], b':', "a drive-letter absolute: {raw}");
        let context = FileResolutionContext::from_snapshot(&root, None, HashMap::new());

        let finding = missing(&raw, &context, &root);
        let resolved = finding.reference_details.as_ref().unwrap().resolved.clone().unwrap();
        assert_eq!(resolved, raw.replace('\\', "/"));
        assert!(reference_paths(&finding).iter().all(|path| !path.contains('\\')), "{finding:#?}");
        let shown = human(finding, &root);
        assert!(shown.contains("resolved to sub/x.md"), "{shown}");
        assert!(shown.contains("hint: create sub/x.md before a real run"), "{shown}");
    }

    #[test]
    fn authored_backslashes_resolve_to_slash_data_and_a_slash_path_for_a_person() {
        let (_dir, root) = root();
        fs::create_dir_all(root.join("sub")).unwrap();
        fs::write(root.join("sub").join("xx.md"), "near miss").unwrap();
        let context = FileResolutionContext::from_snapshot(&root, None, HashMap::new());

        let finding = missing(r".\sub\x.md", &context, &root);
        let details = finding.reference_details.clone().unwrap();
        assert_eq!(details.reference.as_deref(), Some(r".\sub\x.md"), "the reference is kept as written");
        assert_eq!(details.resolved, Some(to_forward_slash_string(&root.join("sub").join("x.md"))));
        assert!(reference_paths(&finding).iter().all(|path| !path.contains('\\')), "{finding:#?}");
        let shown = human(finding, &root);
        assert!(shown.contains("resolved to sub/x.md"), "{shown}");
    }

    #[test]
    fn a_unc_base_keeps_its_native_data_spelling_and_shows_relative_slash_paths() {
        let base = Path::new(UNC_BASE);
        let target = base.join("sub").join("x.md");
        // `to_forward_slash_string` keeps a UNC path native: a slash spelling
        // of it is no longer a UNC path.
        assert_eq!(to_forward_slash_string(&target), r"\\server\share\repo\sub\x.md");
        let finding = Diagnostic::actionable("dm.reference.missing_document", darkmatter::findings::Severity::Error, "the prompt file does not exist", "correct the reference")
            .with_source(DiagnosticSource::at(Some(base.join("docs").join("guide.md")), 2, 3))
            .with_reference_details(darkmatter::findings::ReferenceDetails {
                construct: Some("prompt:".into()),
                reference: Some("./sub/x.md".into()),
                base: Some(to_forward_slash_string(base)),
                resolved: Some(to_forward_slash_string(&target)),
                ..Default::default()
            });
        let value = serde_json::to_value(&finding).unwrap();
        assert_eq!(value["resolved"], to_forward_slash_string(&target), "{value:#}");

        let shown = human(finding, base);
        assert!(shown.contains("resolved to sub/x.md"), "{shown}");
        assert!(shown.contains("at docs/guide.md:2:3"), "{shown}");
    }
}
