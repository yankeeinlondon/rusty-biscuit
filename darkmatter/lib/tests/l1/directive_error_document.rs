//! `::toc-linking` and `::file-links` failures name the document whose body
//! holds the directive: a file-backed document its file, a transcluded local
//! child the child's file, and a file-less document its context identity (or,
//! without one, the request's source file it is admitted as). Their `line` counts that body, so a host composing
//! generated text tells its own failures from a child's by this name alone.

use std::path::{Path, PathBuf};

use darkmatter::markdown::compose::{ComposeContext, ComposeOptions};
use darkmatter::markdown::{Markdown, MarkdownError};
use tempfile::TempDir;

/// Each directive fails a different way, with part of its position-free
/// reason: a bad level, cleanup service, and glob, an unresolved target, a
/// bad `--depth`, and a missing `--dir` target.
const FAILING: [(&str, &str); 6] = [
    ("::toc-linking ./x.md level=h9\n", "heading level 'h9'"),
    ("::toc-linking ./x.md cleanup=bogus\n", "cleanup service 'bogus'"),
    ("::toc-linking ./x.md filter=\"[a\"\n", "glob pattern '[a'"),
    ("::toc-linking ./missing.md\n", "could not resolve './missing.md'"),
    ("::file-links --dir . --depth x\n", "--depth"),
    ("::file-links --dir ./nope\n", "'./nope' does not exist"),
];

/// The document a toc-linking or file-links failure names, with its line.
fn named_document(error: &MarkdownError) -> (Option<PathBuf>, String) {
    match error {
        MarkdownError::TocLinking(toc) => (toc.document().map(Path::to_path_buf), toc.reason()),
        MarkdownError::FileLinks(links) => (links.document().map(Path::to_path_buf), links.reason()),
        other => panic!("expected a toc-linking or file-links failure, got {other:?}"),
    }
}

/// Composes `markdown` in `root` with `file` as the request's source, failing
/// fast so a child's failure is not tolerated as a warning.
fn failure(root: &Path, markdown: Markdown, file: &Path) -> MarkdownError {
    let options = ComposeOptions::new_with_context(ComposeContext::capture_for_content(root, ""))
        .with_source_file(file)
        .with_fail_fast(true);
    markdown
        .compose_with(&crate::request_support::request_at(root, options))
        .expect_err("the directive fails")
}

fn workspace() -> (TempDir, PathBuf) {
    let dir = TempDir::new().unwrap();
    let root = biscuit_file::canonicalize_simplified(dir.path()).unwrap();
    std::fs::write(root.join("x.md"), "# X\n\n## Section\n").unwrap();
    (dir, root)
}

#[test]
fn toc_linking_and_file_links_failures_name_a_file_backed_document_and_a_local_child() {
    for (directive, expected_reason) in FAILING {
        let (_dir, root) = workspace();
        let file = root.join("doc.md");
        std::fs::write(&file, format!("# Doc\n\n{directive}")).unwrap();
        let error = failure(&root, Markdown::try_from(file.as_path()).unwrap(), &file);
        let (document, reason) = named_document(&error);
        assert_eq!(document.as_deref(), Some(file.as_path()), "{directive:?}: {error:?}");
        assert!(reason.contains(expected_reason), "{directive:?}: {reason}");
        assert!(!reason.contains("line"), "{directive:?}: the reason is position-free: {reason}");

        let child = root.join("child.md");
        std::fs::write(&child, format!("# Child\n\n{directive}")).unwrap();
        std::fs::write(&file, "# Doc\n\n::file ./child.md\n").unwrap();
        let error = failure(&root, Markdown::try_from(file.as_path()).unwrap(), &file);
        let (document, reason) = named_document(&error);
        assert!(reason.contains(expected_reason), "{directive:?}: {reason}");
        assert_eq!(document.as_deref(), Some(child.as_path()), "{directive:?}: the child's own: {error:?}");
    }
}

/// The file-less document is composed as a host composes text it generated
/// from a file: that file is the request's source.
#[test]
fn toc_linking_and_file_links_failures_name_a_context_identity() {
    let identity = PathBuf::from("<generated>");
    for (directive, expected_reason) in FAILING {
        for identified in [true, false] {
            let (_dir, root) = workspace();
            let markdown = Markdown::from(format!("Rewritten {{{{ 1 + 1 }}}}.\n\n{directive}"));
            let markdown = if identified { markdown.with_context_identity(&identity) } else { markdown };
            let source = root.join("source.md");
            std::fs::write(&source, "# Source\n").unwrap();
            let error = failure(&root, markdown, &source);
            let (document, reason) = named_document(&error);
            assert!(reason.contains(expected_reason), "{directive:?}: {reason}");
            // Without an identity the buffer is the request's source file's
            // text (a library caller's request-only source), so it names that
            // file; generated text names its identity instead.
            let expected = if identified { identity.as_path() } else { source.as_path() };
            let expected = Some(expected);
            assert_eq!(document.as_deref(), expected, "{directive:?} (identified: {identified}): {error:?}");
        }
    }
}

/// A tolerated `::toc-linking` or `::file-links` failure is a warning. In a
/// document with a context identity it names no line, since its lines count
/// generated text; in any other document it keeps the directive's line.
#[test]
fn tolerated_toc_linking_and_file_links_warnings_name_a_line_only_without_an_identity() {
    let identity = PathBuf::from("<generated>");
    let tolerated = ["::toc-linking ./x.md filter=\"[a\"\n", "::file-links --dir ./nope\n", "::file-links ./none/*.md\n"];
    for directive in tolerated {
        for identified in [true, false] {
            let (_dir, root) = workspace();
            let source = root.join("source.md");
            std::fs::write(&source, "# Source\n").unwrap();
            let markdown = Markdown::from(format!("Rewritten {{{{ 1 + 1 }}}}.\n\n{directive}"));
            let markdown = if identified { markdown.with_context_identity(&identity) } else { markdown };
            let options =
                ComposeOptions::new_with_context(ComposeContext::capture_for_content(&root, "")).with_source_file(&source);
            let (_, report) = markdown
                .compose_with(&crate::request_support::request_at(&root, options))
                .expect("the failure is tolerated");
            let [warning] = report.warnings.as_slice() else {
                panic!("{directive:?}: one warning: {:?}", report.warnings);
            };
            assert_eq!(warning.message.contains("line 3"), !identified, "{directive:?} (identified: {identified}): {warning:?}");
        }
    }
}
