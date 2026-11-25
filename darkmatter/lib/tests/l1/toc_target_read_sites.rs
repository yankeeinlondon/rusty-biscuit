//! A `::toc-linking` target that resolves but cannot be read (a Markdown file
//! saved as UTF-16) is reported at the directive that selected it, as any
//! other inclusion target that cannot be read is: composition tolerates it
//! with a `dm.input.read_failure` warning, and every graph reader that reads
//! the target's headings fails with that code, each naming the directive's
//! file and authored opener line and the selected file. The pre-flight walk
//! selects the target without reading it, and a readable or suppressed
//! target adds no read diagnostic.

use biscuit_file::FileResolutionContext;
use darkmatter::markdown::compose::expression::Diagnostic;
use darkmatter::markdown::compose::report_diagnostics::{error_diagnostics, warning_diagnostic};
use darkmatter::markdown::compose::{ComposeContext, ComposeOptions, ComposeRequest};
use darkmatter::markdown::reference::file_tree::FileTree;
use darkmatter::markdown::reference::types::ReferenceGraphOptions;
use darkmatter::markdown::reference::validate::ReferenceValidationOptions;
use darkmatter::markdown::Markdown;
use std::error::Error;
use std::path::Path;
use tempfile::TempDir;

const READ_FAILURE: &str = "dm.input.read_failure";

/// `# Utf16` as UTF-16LE with its byte-order mark: not valid UTF-8.
fn utf16_markdown() -> Vec<u8> {
    let mut bytes = vec![0xFF, 0xFE];
    bytes.extend("# Utf16\n".encode_utf16().flat_map(u16::to_le_bytes));
    bytes
}

/// A temporary repository holding the readable `side.md` and the UTF-16
/// `utf16.md`.
fn repository() -> TempDir {
    let repo = TempDir::new().unwrap();
    std::fs::create_dir_all(repo.path().join(".git")).unwrap();
    std::fs::write(repo.path().join("side.md"), "# Side\n").unwrap();
    std::fs::write(repo.path().join("utf16.md"), utf16_markdown()).unwrap();
    repo
}

/// A request in the repository's frozen context, so nothing reads the real
/// working tree.
fn request(root: &Path) -> ComposeRequest {
    let context = FileResolutionContext::from_snapshot(root, None, std::collections::HashMap::new())
        .with_repository_root(root);
    crate::request_support::request_in(
        ComposeOptions::new_with_context(ComposeContext::capture_for_content(root, "")),
        context,
    )
}

/// `name` written with `content` and loaded from its file.
fn document(repo: &TempDir, name: &str, content: &str) -> Markdown {
    let path = repo.path().join(name);
    std::fs::write(&path, content).unwrap();
    Markdown::try_from(path.as_path()).unwrap()
}

/// A YAML header, then `directive` on file line 8.
fn plain(directive: &str) -> (String, usize) {
    (format!("---\ntitle: x\nother: y\n---\n\n# Doc\n\n{directive}\n"), 8)
}

/// The same header and an earlier false page block, then `directive` on
/// file line 11.
fn after_removed_block(directive: &str) -> (String, usize) {
    (format!("---\ntitle: x\nother: y\n---\n\n::block when=\"false\"\nhidden\n::end-block\n# Doc\n\n{directive}\n"), 11)
}

/// Every reader that reads a selected TOC target's headings, with its error.
fn graph_readers(repo: &TempDir, md: &Markdown) -> Vec<(&'static str, Option<Box<dyn Error>>)> {
    let request = request(repo.path());
    let graph = ReferenceGraphOptions::with_compose(&request);
    let boxed = |error: darkmatter::markdown::MarkdownError| -> Box<dyn Error> { Box::new(error) };
    let mut readers: Vec<(&'static str, Option<Box<dyn Error>>)> = vec![
        ("reference graph", md.reference_graph(graph.clone()).err().map(boxed)),
        ("transclusion graph", md.transclusion_graph(graph.clone()).err().map(boxed)),
        ("flattened", md.composed_references(graph.clone()).err().map(boxed)),
        ("fresh", md.validate_references(ReferenceValidationOptions::with_graph(graph)).err().map(boxed)),
    ];
    for (name, follow) in [("file tree", false), ("followed file tree", true)] {
        let tree = FileTree::from_markdown(md.clone(), &request).validate();
        let mut tree = if follow { tree.follow_transclusions() } else { tree };
        readers.push((name, tree.ensure_built().err().map(|error| -> Box<dyn Error> { Box::new(error) })));
    }
    readers
}

/// Asserts `record` is a read failure at `file` line `line`, column 1, whose
/// message names the selected `utf16.md`.
fn assert_read_failure_at(record: &Diagnostic, file: &str, line: usize, shape: &str) {
    assert_eq!(record.code, READ_FAILURE, "{shape}: {record:#?}");
    assert!(record.message.contains("utf16.md"), "{shape}: the selected file is named: {record:#?}");
    let source = record.source.as_ref().unwrap_or_else(|| panic!("{shape}: {record:#?}"));
    assert!(source.file.as_ref().is_some_and(|path| path.ends_with(file)), "{shape}: {record:#?}");
    assert_eq!((source.line, source.column), (Some(line), Some(1)), "{shape}: {record:#?}");
}

/// The shapes of an unreadable selected target, with the line the
/// directive is on: bare and quoted, selected directly or after an absent
/// first alternative, after a header and after a removed block.
fn unreadable_shapes() -> Vec<(String, usize)> {
    let mut shapes = Vec::new();
    for target in ["./utf16.md", "\"./utf16.md\"", "\"./absent.md | ./utf16.md\""] {
        let directive = format!("::toc-linking {target}");
        shapes.push(plain(&directive));
        shapes.push(after_removed_block(&directive));
    }
    shapes
}

/// R13-F2.1: composition tolerates the unreadable target with one
/// `dm.input.read_failure` warning at the directive's authored opener,
/// naming the selected file, in the root and in a nested child.
#[test]
fn composition_warns_at_the_toc_directive_that_selected_an_unreadable_file() {
    let repo = repository();
    for (content, line) in unreadable_shapes() {
        for nested in [false, true] {
            let md = if nested {
                document(&repo, "nested.md", &content);
                document(&repo, "guide.md", "# Guide\n\n::file ./nested.md\n")
            } else {
                document(&repo, "doc.md", &content)
            };
            let file = if nested { "nested.md" } else { "doc.md" };
            let shape = format!("{content:?} nested={nested}");
            let (composed, report) = md.compose_with(&request(repo.path())).unwrap_or_else(|error| panic!("{shape}: {error}"));
            assert!(composed.content().contains("Could not link headings"), "{shape}: the notice stays visible");
            let records: Vec<_> = report.warnings.iter().map(warning_diagnostic).collect();
            let [record] = records.as_slice() else { panic!("{shape}: {records:#?}") };
            assert_read_failure_at(record, file, line, &shape);
        }
    }
}

/// R13-F2.2 and R13-F2.3: every graph reader fails with the read error at
/// the directive, never at the input document with no line, directly and
/// for a directive in a nested child. Pre-flight selects the target without
/// reading it, and the local query lists inclusions without opening them.
#[test]
fn graph_readers_fail_at_the_toc_directive_that_selected_an_unreadable_file() {
    let repo = repository();
    for (content, line) in unreadable_shapes() {
        for nested in [false, true] {
            let md = if nested {
                document(&repo, "nested.md", &content);
                document(&repo, "guide.md", "# Guide\n\n::file ./nested.md\n")
            } else {
                document(&repo, "doc.md", &content)
            };
            let file = if nested { "nested.md" } else { "doc.md" };
            for (reader, error) in graph_readers(&repo, &md) {
                let shape = format!("{content:?} nested={nested}: {reader}");
                let error = error.unwrap_or_else(|| panic!("{shape} must fail"));
                let records = error_diagnostics(error.as_ref());
                let [record] = records.as_slice() else { panic!("{shape}: {error}: {records:#?}") };
                assert_read_failure_at(record, file, line, &shape);
            }
            md.compose_preflight(&request(repo.path())).unwrap_or_else(|error| panic!("{content:?}: preflight: {error}"));
            md.transclusions_with_options(&request(repo.path()))
                .unwrap_or_else(|error| panic!("{content:?}: local query: {error}"));
        }
    }
}

/// R13-F2.4: a readable target and a suppressed chain add no read
/// diagnostic in any reader, and an ordinary `::file` of the same unreadable
/// file keeps its own located failure.
#[test]
fn readable_and_suppressed_toc_targets_add_no_read_diagnostic() {
    let repo = repository();
    for directive in ["::toc-linking ./side.md", "::toc-linking \"./absent.md | false\""] {
        let (content, _) = after_removed_block(directive);
        let md = document(&repo, "doc.md", &content);
        let (_, report) = md.compose_with(&request(repo.path())).unwrap_or_else(|error| panic!("{directive}: {error}"));
        assert!(report.warnings.is_empty(), "{directive}: {:#?}", report.warnings);
        for (reader, error) in graph_readers(&repo, &md) {
            assert!(error.is_none(), "{directive}: {reader}: {error:?}");
        }
    }
    let (content, line) = after_removed_block("::file ./utf16.md");
    let md = document(&repo, "doc.md", &content);
    for (reader, error) in graph_readers(&repo, &md) {
        let shape = format!("::file: {reader}");
        let error = error.unwrap_or_else(|| panic!("{shape} must fail"));
        let records = error_diagnostics(error.as_ref());
        let [record] = records.as_slice() else { panic!("{shape}: {error}: {records:#?}") };
        assert_read_failure_at(record, "doc.md", line, &shape);
    }
}
