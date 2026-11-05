//! The two link-title warnings through composition's public results: located
//! in authored coordinates, one per link, kept by a failed run, and judged by
//! the blocking policy.

use std::path::{Path, PathBuf};

use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::policy::BlockingPolicy;
use darkmatter::markdown::compose::report_diagnostics::{error_diagnostics, warning_diagnostic};
use darkmatter::markdown::compose::{ComposeContext, ComposeFailure, ComposeOptions, ComposeReport, ComposeWarning};
use tempfile::TempDir;

const LEGACY: &str = "dm.link.legacy_structured_title";
const MALFORMED: &str = "dm.link.malformed_structured_title";

fn options_in(dir: &Path, path: &Path) -> ComposeOptions {
    ComposeOptions::new_with_context(ComposeContext::capture_for_content(dir, "")).with_source_file(path.to_path_buf())
}

fn compose_with(
    dir: &Path,
    text: &str,
    options: impl FnOnce(ComposeOptions) -> ComposeOptions,
) -> (PathBuf, Result<(Markdown, ComposeReport), Box<ComposeFailure>>) {
    let path = dir.join("doc.md");
    std::fs::write(&path, text).unwrap();
    let request = crate::request_support::request_at(dir, options(options_in(dir, &path)));
    let result = Markdown::try_from(path.as_path()).unwrap().compose_with_warnings(&request);
    (path, result)
}

fn link_warnings(warnings: &[ComposeWarning]) -> Vec<&ComposeWarning> {
    warnings.iter().filter(|warning| warning.code.as_deref().is_some_and(|code| code.starts_with("dm.link."))).collect()
}

fn composed(text: &str) -> (TempDir, PathBuf, ComposeReport) {
    let dir = tempfile::tempdir().unwrap();
    let (path, result) = compose_with(dir.path(), text, |options| options);
    let (_, report) = result.unwrap_or_else(|failure| panic!("composes: {}", failure.error));
    (dir, path, report)
}

#[test]
fn a_legacy_title_warns_at_its_authored_position_and_says_to_run_md_clean() {
    let (_dir, path, report) = composed("---\nkey: value\n---\n\n# Title\n\nSee [Docs](./a.md \"class='x'\") now.\n");
    let warnings = link_warnings(&report.warnings);
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    let diagnostic = warning_diagnostic(warnings[0]);
    assert_eq!(diagnostic.code, LEGACY);
    assert!(diagnostic.message.contains("[Docs](./a.md)"), "{}", diagnostic.message);
    assert!(diagnostic.message.contains("md clean"), "{}", diagnostic.message);
    let source = diagnostic.source.expect("located");
    assert_eq!((source.file.as_deref(), source.line, source.column), (Some(path.as_path()), Some(7), Some(5)));
    let text = std::fs::read_to_string(&path).unwrap();
    let span = diagnostic.span.expect("a span");
    assert_eq!(&text[span], "[Docs](./a.md \"class='x'\")");
}

#[test]
fn a_malformed_title_warns_and_names_the_link() {
    let (_dir, _path, report) = composed("[Docs](./a.md \"{ class: \")\n");
    let warnings = link_warnings(&report.warnings);
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert_eq!(warnings[0].code.as_deref(), Some(MALFORMED));
    assert!(warnings[0].message.contains("[Docs](./a.md)"), "{}", warnings[0].message);
}

#[test]
fn an_unusable_qp_warns_as_malformed() {
    let (_dir, _path, report) = composed("[Docs](./a.md '{ qp: { line: null } }')\n");
    assert_eq!(link_warnings(&report.warnings)[0].code.as_deref(), Some(MALFORMED));
}

#[test]
fn the_position_ignores_what_earlier_stages_wrote() {
    let (_dir, _path, report) = composed("---\nname: a long value that shifts the body\n---\n{{ name }}\n\nx [a](./a.md \"class='x'\")\n");
    let diagnostic = warning_diagnostic(link_warnings(&report.warnings)[0]);
    let source = diagnostic.source.unwrap();
    assert_eq!((source.line, source.column), (Some(6), Some(3)));
}

#[test]
fn equal_links_at_different_spans_each_warn() {
    let (_dir, _path, report) = composed("[a](./a.md \"class='x'\")\n\n[a](./a.md \"class='x'\")\n");
    let warnings = link_warnings(&report.warnings);
    let lines: Vec<_> = warnings.iter().map(|warning| warning.line_number).collect();
    assert_eq!(lines, [Some(1), Some(3)], "{warnings:?}");
}

#[test]
fn one_span_reached_twice_warns_once() {
    // The first scan and the scan after the inline stages both see this link;
    // the title's interpolation changes neither its span nor its code.
    let (_dir, _path, report) = composed("---\nkind: x\n---\n[a](./a.md \"class='y' data-k='{{ kind }}'\")\n");
    assert_eq!(link_warnings(&report.warnings).len(), 1, "{:?}", report.warnings);
}

#[test]
fn a_link_repeated_by_a_map_warns_once_at_its_authored_span() {
    let (_dir, _path, report) = composed(
        "---\n$schema:\n  names: string[](required)\nnames: [ada, bob, cy]\n---\n::map names \"n => n\"\n[{{n}}](https://example.com \"class='x'\")\n::end-map\n",
    );
    let warnings = link_warnings(&report.warnings);
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert_eq!(warnings[0].line_number, Some(7));
}

#[test]
fn a_child_warns_in_its_own_file() {
    let dir = tempfile::tempdir().unwrap();
    let child = dir.path().join("child.md");
    std::fs::write(&child, "\n[c](./c.md \"class='x'\")\n").unwrap();
    let (_path, result) = compose_with(dir.path(), "::file ./child.md\n", |options| options);
    let (_, report) = result.unwrap_or_else(|failure| panic!("{}", failure.error));
    let warnings = link_warnings(&report.warnings);
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    let source = warning_diagnostic(warnings[0]).source.expect("located");
    assert_eq!(source.line, Some(2));
    assert_eq!(source.file.unwrap().file_name().unwrap(), "child.md");
}

#[test]
fn a_title_an_expression_wrote_warns_at_that_expression() {
    let (_dir, _path, report) = composed("---\ntip: \"class=x\"\n---\n\n[a](./a.md \"{{ tip }}\")\n");
    let warnings = link_warnings(&report.warnings);
    assert_eq!(warnings.len(), 1, "{:?}", report.warnings);
    let source = warning_diagnostic(warnings[0]).source.expect("located");
    assert_eq!((source.line, source.column), (Some(5), Some(13)));
    assert!(source.inserted, "{source:?}");
}

#[test]
fn an_authored_expression_title_is_not_malformed() {
    let (_dir, _path, report) = composed("---\ntip: Plain\n---\n[a](./a.md \"{{ tip }}\")\n");
    assert!(link_warnings(&report.warnings).is_empty(), "{:?}", report.warnings);
}

#[test]
fn code_and_directive_lines_do_not_warn() {
    let (_dir, _path, report) = composed("`[a](./a.md \"class='x'\")`\n\n```text\n[b](./b.md \"class='y'\")\n```\n\n![c](./c.png \"class='z'\")\n");
    assert!(link_warnings(&report.warnings).is_empty(), "{:?}", report.warnings);
}

#[test]
fn a_failed_run_keeps_the_warning() {
    let dir = tempfile::tempdir().unwrap();
    let (_path, result) = compose_with(dir.path(), "[a](./a.md \"class='x'\")\n\n{{ 1 / 0 }}\n", |options| options);
    let failure = result.expect_err("the division fails");
    assert_eq!(link_warnings(&failure.warnings).len(), 1, "{:?}", failure.warnings);
}

#[test]
fn a_denied_code_stops_the_document_with_the_warning() {
    let dir = tempfile::tempdir().unwrap();
    let (_path, result) = compose_with(dir.path(), "[a](./a.md \"class='x'\")\n", |options| {
        options.with_blocking_policy(BlockingPolicy::default().deny_code(LEGACY))
    });
    let failure = result.expect_err("blocked");
    let diagnostics = error_diagnostics(&failure.error);
    assert_eq!(diagnostics[0].code, LEGACY, "{diagnostics:?}");
    assert!(diagnostics[0].source.is_some(), "{diagnostics:?}");
}

#[test]
fn another_denied_code_leaves_the_document_alone() {
    let dir = tempfile::tempdir().unwrap();
    let (_path, result) = compose_with(dir.path(), "[a](./a.md \"class='x'\")\n", |options| {
        options.with_blocking_policy(BlockingPolicy::default().deny_code(MALFORMED))
    });
    assert!(result.is_ok());
}

#[test]
fn an_in_memory_document_warns_without_a_position() {
    let md: Markdown = "[a](./a.md \"class='x'\")\n".into();
    let warnings = md.link_title_warnings();
    assert_eq!(warnings.len(), 1);
    assert!(warning_diagnostic(&warnings[0]).source.is_none());
}
