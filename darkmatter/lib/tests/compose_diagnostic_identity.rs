//! Requirement 5 of the dasherized-identifiers spec
//! (`darkmatter/features/2026-09-15-dasherized-identifiers`): a compose run
//! reports each issue exactly once, keyed by source document identity rather
//! than by occurrence or rescan pass.
//!
//! The unresolved-root family is exercised through the `ctx.*` typo warning,
//! the root warning that exists today; Phase 5's
//! `dm.expression.unknown_identifier` warning shares its identity shape.

use std::path::{Path, PathBuf};

use darkmatter::markdown::compose::expression::ExpressionError;
use darkmatter::markdown::{Markdown, MarkdownError};
use darkmatter::markdown::compose::{ComposeContext, ComposeOptions, ComposeReport, ComposeWarning};
use tempfile::TempDir;

fn write(dir: &Path, name: &str, content: &str) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, content).unwrap();
    path
}

/// A context capturing only the date/time group, so no test observes the host
/// repository or environment.
fn options(dir: &Path) -> ComposeOptions {
    ComposeOptions::new_with_context(ComposeContext::capture_for_content(dir, ""))
}

fn compose_file(path: &Path) -> (String, ComposeReport) {
    let md = Markdown::try_from(path).expect("document loads");
    let dir = path.parent().expect("fixture documents live in a directory");
    let (composed, report) = md
        .compose_with(options(dir).with_source_file(path.to_path_buf()))
        .expect("lenient warnings do not fail composition");
    (composed.content().to_string(), report)
}

fn with_code<'a>(report: &'a ComposeReport, code: &str) -> Vec<&'a ComposeWarning> {
    report.warnings.iter().filter(|w| w.code.as_deref() == Some(code)).collect()
}

fn unknown_context(report: &ComposeReport) -> Vec<&ComposeWarning> {
    with_code(report, ComposeWarning::UNKNOWN_CONTEXT_VARIABLE_CODE)
}

/// The original rescan fixture, through full composition: the successful
/// `{{ title }}` replacement forces a second pass over the unchanged failure,
/// which used to report it twice. Since Phase 4 the failure is fatal, so the
/// one issue is the one error, naming the failing expression.
#[test]
fn one_replacement_and_one_bad_body_expression_fail_with_one_error() {
    let dir = TempDir::new().unwrap();
    let path = write(dir.path(), "doc.md", "---\ntitle: Hello\n---\n{{ title }} {{ > invalid }}\n");

    let md = Markdown::try_from(path.as_path()).expect("document loads");
    let error = md
        .compose_with(options(dir.path()).with_source_file(path.clone()))
        .expect_err("a bad body expression fails composition");

    let MarkdownError::Interpolation { expression, cause, .. } = &error else {
        panic!("expected a typed interpolation error, got {error:?}");
    };
    assert_eq!(expression, "> invalid");
    assert!(matches!(cause.as_ref(), ExpressionError::Parse(_)), "{cause:?}");
}

#[test]
fn one_unknown_root_referenced_ten_times_warns_once() {
    let dir = TempDir::new().unwrap();
    let body = "- {{ ctx.toady }}\n".repeat(10);
    let path = write(dir.path(), "doc.md", &body);

    let (content, report) = compose_file(&path);

    assert_eq!(content.matches("- ").count(), 10, "every reference still renders: {content}");
    let warnings = unknown_context(&report);
    assert_eq!(warnings.len(), 1, "{:?}", report.warnings);
    assert!(warnings[0].message.contains("'ctx.toady'"));
}

/// Frontmatter and body are one source document: the root is one issue, and
/// the first authored occurrence (frontmatter, which composes first) is kept.
#[test]
fn one_unknown_root_in_frontmatter_and_body_of_one_document_warns_once() {
    let dir = TempDir::new().unwrap();
    let path = write(
        dir.path(),
        "doc.md",
        "---\nlabel: \"x {{ ctx.toady }}\"\nplain: \"{{ ctx.toady.deeper }}\"\n---\n{{ ctx.toady }}\n",
    );

    let (_, report) = compose_file(&path);

    let warnings = unknown_context(&report);
    assert_eq!(warnings.len(), 1, "{:?}", report.warnings);
    assert!(warnings[0].message.contains("key 'label'"), "{warnings:?}");
}

#[test]
fn two_different_unknown_roots_warn_twice() {
    let dir = TempDir::new().unwrap();
    let path = write(
        dir.path(),
        "doc.md",
        "{{ ctx.toady }} {{ ctx.fooo }} {{ ctx.toady }} {{ ctx.fooo.bar }}\n",
    );

    let (_, report) = compose_file(&path);

    let messages: Vec<&str> = unknown_context(&report).iter().map(|w| w.message.as_str()).collect();
    assert_eq!(messages.len(), 2, "{messages:?}");
    assert!(messages[0].contains("'ctx.toady'"), "document order: {messages:?}");
    assert!(messages[1].contains("'ctx.fooo'"), "document order: {messages:?}");
}

/// The same root in two transcluded source documents is two independently
/// actionable issues; transcluding one of them twice is still one issue for
/// that document. The root references nothing unknown itself.
#[test]
fn one_root_in_two_transcluded_documents_warns_once_per_document() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "a.md", "A {{ ctx.toady }} {{ ctx.toady }}\n");
    write(dir.path(), "b.md", "B {{ ctx.toady }}\n");
    let root = write(
        dir.path(),
        "root.md",
        "# Root\n\n::file ./a.md\n\n::file ./b.md\n\n::file ./a.md\n",
    );

    let (content, report) = compose_file(&root);

    let lines: Vec<&str> = content.lines().collect();
    assert_eq!(lines.iter().filter(|line| **line == "A").count(), 2, "{content}");
    assert_eq!(lines.iter().filter(|line| **line == "B").count(), 1, "{content}");
    assert_eq!(unknown_context(&report).len(), 2, "{:?}", report.warnings);
}

/// Each frontmatter value is its own scanned text, so failures at the same
/// offsets in two keys would be two issues (`report::tests` pins the scoped
/// identity). Composition stops at the first, naming its key.
#[test]
fn identical_bad_expressions_in_two_frontmatter_keys_fail_on_one_key() {
    let dir = TempDir::new().unwrap();
    let path = write(
        dir.path(),
        "doc.md",
        "---\nfirst: \"x {{ > bad }}\"\nsecond: \"x {{ > bad }}\"\n---\nbody\n",
    );

    let md = Markdown::try_from(path.as_path()).expect("document loads");
    let error = md
        .compose_with(options(dir.path()).with_source_file(path.clone()))
        .expect_err("a bad mixed-text frontmatter expression fails composition");

    let MarkdownError::Interpolation { key, .. } = &error else {
        panic!("expected a typed interpolation error, got {error:?}");
    };
    assert!(matches!(key.as_deref(), Some("first" | "second")), "{error:?}");
}
