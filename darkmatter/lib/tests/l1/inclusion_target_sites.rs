//! An inclusion whose directive parses but whose target fails (an
//! incomplete `@`, a missing file, an unparsable URL) names where it was
//! written: the directive's opener for a body `::file`, `::code`, or `::url`,
//! and the key or list entry for a `prologue` or `epilogue` value, in the
//! file that wrote it, a nested child included. Every reader that refuses the
//! target (composition, the pre-flight walk, both graph modes, the local
//! query, flattened references, and validation) keeps the typed code and
//! publishes that file and line in its diagnostic's `source`; a reader that
//! only lists the target (a graph built without validation listing a
//! `::code` or `::url` target, the raw query listing a URL) still succeeds.

use biscuit_file::FileResolutionContext;
use darkmatter::markdown::compose::report_diagnostics::{error_diagnostics, reference_issue_diagnostic, warning_diagnostic};
use darkmatter::markdown::compose::{ComposeContext, ComposeOptions, ComposeRequest};
use darkmatter::markdown::reference::types::ReferenceGraphOptions;
use darkmatter::markdown::reference::validate::ReferenceValidationOptions;
use darkmatter::markdown::{Markdown, MarkdownError};
use std::path::Path;
use tempfile::TempDir;

const REFERENCE_FAILURE: &str = "dm.reference.failure";
const MISSING_TRANSCLUSION: &str = "dm.reference.missing_transclusion";

/// A temporary repository holding the existing `side.md`.
fn repository() -> TempDir {
    let repo = TempDir::new().unwrap();
    std::fs::create_dir_all(repo.path().join(".git")).unwrap();
    std::fs::write(repo.path().join("side.md"), "# Side\n").unwrap();
    repo
}

/// A request in the repository's frozen context, with an empty shell policy
/// under its own home, so nothing reads the real working tree or host.
fn request(root: &Path) -> ComposeRequest {
    let home = root.join(".home");
    std::fs::create_dir_all(&home).unwrap();
    let context = FileResolutionContext::from_snapshot(root, None, std::collections::HashMap::new())
        .with_repository_root(root);
    let options = ComposeOptions::new_with_context(ComposeContext::capture_for_content(root, ""))
        .with_config_snapshot(crate::request_support::shell_config(&home, &[]));
    crate::request_support::request_in(options, context)
}

/// `name` written with `content` and loaded from its file.
fn document(repo: &TempDir, name: &str, content: &str) -> Markdown {
    let path = repo.path().join(name);
    std::fs::write(&path, content).unwrap();
    Markdown::try_from(path.as_path()).unwrap()
}

/// The readers of an inclusion target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Reader {
    Compose,
    Preflight,
    ReferenceGraph,
    TransclusionGraph,
    LocalQuery,
    Flattened,
    Validation,
}

const READERS: [Reader; 7] = [
    Reader::Compose,
    Reader::Preflight,
    Reader::ReferenceGraph,
    Reader::TransclusionGraph,
    Reader::LocalQuery,
    Reader::Flattened,
    Reader::Validation,
];

fn read(repo: &TempDir, md: &Markdown, reader: Reader) -> Result<(), MarkdownError> {
    let request = request(repo.path());
    let graph = ReferenceGraphOptions::with_compose(&request);
    match reader {
        Reader::Compose => md.compose_with(&request).map(drop),
        Reader::Preflight => md.compose_preflight(&request).map(drop),
        Reader::ReferenceGraph => md.reference_graph(graph).map(drop),
        Reader::TransclusionGraph => md.transclusion_graph(graph).map(drop),
        Reader::LocalQuery => md.transclusions_with_options(&request).map(drop),
        Reader::Flattened => md.composed_references(graph).map(drop),
        Reader::Validation => md.validate_references(ReferenceValidationOptions::with_graph(graph)).map(drop),
    }
}

/// Asserts `error` is one `code` record naming `file` at `line`.
fn assert_located(error: &MarkdownError, code: &str, file: &str, line: usize, shape: &str) {
    let records = error_diagnostics(error);
    let [record] = records.as_slice() else { panic!("{shape}: {error}: {records:#?}") };
    assert_eq!(record.code, code, "{shape}: {error}: {record:#?}");
    let source = record.source.as_ref().unwrap_or_else(|| panic!("{shape}: {error}: {record:#?}"));
    assert!(source.file.as_ref().is_some_and(|path| path.ends_with(file)), "{shape}: {error}: {record:#?}");
    assert_eq!(source.line, Some(line), "{shape}: {error}: {record:#?}");
    assert!(source.column.is_some(), "{shape}: {error}: {record:#?}");
}

/// Asserts each reader of `content` (written as `doc.md`) either refuses it
/// with a located `code` record at `line`, or, for a reader in `listing`,
/// succeeds.
fn assert_readers(repo: &TempDir, content: &str, line: usize, code: &str, listing: &[Reader], shape: &str) {
    let md = document(repo, "doc.md", content);
    for reader in READERS {
        let shape = format!("{shape}: {reader:?}");
        match read(repo, &md, reader) {
            Ok(()) => assert!(listing.contains(&reader), "{shape} must refuse"),
            Err(error) => {
                assert!(!listing.contains(&reader), "{shape} lists without judging: {error}");
                assert_located(&error, code, "doc.md", line, &shape);
            }
        }
    }
}

/// A YAML header, then `directive` on file line 8.
fn plain(directive: &str) -> (String, usize) {
    (format!("---\ntitle: x\nother: y\n---\n\n# Doc\n\n{directive}\n"), 8)
}

/// The same header and an earlier false page block, then `directive` on
/// file line 11: the prepared body loses the block, the raw body keeps it.
fn after_removed_block(directive: &str) -> (String, usize) {
    (format!("---\ntitle: x\nother: y\n---\n\n::block when=\"false\"\nhidden\n::end-block\n# Doc\n\n{directive}\n"), 11)
}

/// R12-F2.1: an incomplete `@` body target, bare and quoted, after a header
/// and after a removed block, is refused at its opener's file line. A graph
/// built without validation lists a `::code` or `::url` target unjudged,
/// and the raw query lists a URL.
#[test]
fn an_invalid_body_target_names_its_directive_line() {
    let repo = repository();
    let listing_code = [Reader::ReferenceGraph, Reader::TransclusionGraph, Reader::Flattened];
    let listing_url = [Reader::ReferenceGraph, Reader::TransclusionGraph, Reader::Flattened, Reader::LocalQuery];
    for target in ["@", "'@'"] {
        for (kind, listing) in [("file", &[][..]), ("code", &listing_code[..]), ("url", &listing_url[..])] {
            let directive = format!("::{kind} {target}");
            for (content, line) in [plain(&directive), after_removed_block(&directive)] {
                assert_readers(&repo, &content, line, REFERENCE_FAILURE, listing, &directive);
            }
        }
    }
}

/// R12-F2.1 control: an existing file and a well-formed URL raise nothing
/// a reader judges from the target.
#[test]
fn a_valid_body_target_is_accepted_by_every_reader() {
    let repo = repository();
    for directive in ["::file ./side.md", "::code ./side.md"] {
        let (content, _) = after_removed_block(directive);
        let md = document(&repo, "doc.md", &content);
        for reader in READERS {
            read(&repo, &md, reader).unwrap_or_else(|error| panic!("{directive}: {reader:?}: {error}"));
        }
    }
}

/// R12-F2.2: an incomplete `@` in `prologue` or `epilogue` on line 3, as a
/// scalar and as a list entry, is refused at that key or entry; the shape
/// error of a non-string value keeps the key line, and an existing file is
/// accepted.
#[test]
fn an_invalid_frontmatter_target_names_its_key_line() {
    let repo = repository();
    for key in ["prologue", "epilogue"] {
        for (value, line) in [("\"@\"", 3), ("\n  - ./side.md\n  - \"@\"", 5)] {
            let content = format!("---\ntitle: x\n{key}: {value}\n---\n\n# Doc\n");
            assert_readers(&repo, &content, line, REFERENCE_FAILURE, &[], &format!("{key}: {value}"));
        }
        let content = format!("---\ntitle: x\n{key}: 3\n---\n\n# Doc\n");
        let md = document(&repo, "doc.md", &content);
        let error = read(&repo, &md, Reader::Compose).expect_err("a number is refused");
        let records = error_diagnostics(&error);
        assert_eq!(records[0].code, "dm.directive.parse_failure", "{records:#?}");
        assert_eq!(records[0].source.as_ref().and_then(|source| source.line), Some(3), "{records:#?}");
        let content = format!("---\ntitle: x\n{key}: \"./side.md\"\n---\n\n# Doc\n");
        let md = document(&repo, "doc.md", &content);
        for reader in READERS {
            read(&repo, &md, reader).unwrap_or_else(|error| panic!("{key}: {reader:?}: {error}"));
        }
    }
}

/// R12-F2.3: a missing target in direct composition and the pre-flight walk
/// keeps `dm.reference.missing_transclusion` and names the inclusion's site.
#[test]
fn a_missing_target_names_its_inclusion_site() {
    let repo = repository();
    let shapes = [
        plain("::file ./gone.md"),
        after_removed_block("::code ./gone.md"),
        ("---\ntitle: x\nprologue: \"./gone.md\"\n---\n\n# Doc\n".to_string(), 3),
        ("---\ntitle: x\nepilogue: \"./gone.md\"\n---\n\n# Doc\n".to_string(), 3),
    ];
    for (content, line) in shapes {
        let md = document(&repo, "doc.md", &content);
        for reader in [Reader::Compose, Reader::Preflight] {
            let error = read(&repo, &md, reader).expect_err(&content);
            assert_located(&error, MISSING_TRANSCLUSION, "doc.md", line, &format!("{content}: {reader:?}"));
        }
    }
}

/// R12-F2.4: a nested child's incomplete target names the child and its
/// line, never the input document: each reader that refuses it, and the
/// warning composition reports when it tolerates the child's failure.
#[test]
fn a_nested_invalid_target_names_the_child() {
    let repo = repository();
    document(&repo, "nested.md", "---\na: b\n---\n\n# Nested\n\n::file @\n");
    let md = document(&repo, "doc.md", "# Root\n\n::file ./nested.md\n");
    // The local query lists the root's own inclusions and never opens one;
    // composition tolerates a child's failure.
    let refusing = READERS.into_iter().filter(|reader| !matches!(reader, Reader::LocalQuery | Reader::Compose));
    for reader in refusing {
        let error = read(&repo, &md, reader).expect_err(&format!("{reader:?} must refuse"));
        assert_located(&error, REFERENCE_FAILURE, "nested.md", 7, &format!("{reader:?}"));
    }
    let (_, report) = md.compose_with(&request(repo.path())).expect("a child's failure is tolerated");
    let records: Vec<_> = report.warnings.iter().map(warning_diagnostic).collect();
    let [record] = records.as_slice() else { panic!("{records:#?}") };
    assert_eq!(record.code, REFERENCE_FAILURE, "{record:#?}");
    let source = record.source.as_ref().unwrap_or_else(|| panic!("{record:#?}"));
    assert!(source.file.as_ref().is_some_and(|path| path.ends_with("nested.md")), "{record:#?}");
    assert_eq!((source.line, source.column), (Some(7), Some(1)), "{record:#?}");
}


/// R12-F2.5: a required `::toc-linking` whose only alternative is a bare `@`
/// is located at its opener by every reader that reports it: the pre-flight
/// walk refuses it, validation reports a missing target, and composition
/// tolerates it with a warning. An existing file is accepted.
#[test]
fn a_required_toc_target_the_selector_rejects_names_its_directive_line() {
    let repo = repository();
    for (content, line) in [plain("::toc-linking @"), after_removed_block("::toc-linking @")] {
        let md = document(&repo, "doc.md", &content);
        let error = read(&repo, &md, Reader::Preflight).expect_err("pre-flight refuses");
        assert_located(&error, REFERENCE_FAILURE, "doc.md", line, "preflight");

        let request = request(repo.path());
        let graph = ReferenceGraphOptions::with_compose(&request);
        let report = md.validate_references(ReferenceValidationOptions::with_graph(graph)).expect("validation runs");
        let records: Vec<_> = report.issues.iter().map(reference_issue_diagnostic).collect();
        let [record] = records.as_slice() else { panic!("{records:#?}") };
        let source = record.source.as_ref().unwrap_or_else(|| panic!("{record:#?}"));
        assert!(source.file.as_ref().is_some_and(|path| path.ends_with("doc.md")), "{record:#?}");
        assert_eq!((source.line, source.column), (Some(line), Some(1)), "{record:#?}");

        let (_, composed) = md.compose_with(&request).expect("composition tolerates it");
        let records: Vec<_> = composed.warnings.iter().map(warning_diagnostic).collect();
        let [record] = records.as_slice() else { panic!("{records:#?}") };
        assert_eq!(record.code, REFERENCE_FAILURE, "{record:#?}");
        assert_eq!(record.source.as_ref().and_then(|source| source.line), Some(line), "{record:#?}");
    }
    let (content, _) = plain("::toc-linking ./side.md");
    let md = document(&repo, "doc.md", &content);
    for reader in READERS {
        read(&repo, &md, reader).unwrap_or_else(|error| panic!("{reader:?}: {error}"));
    }
}

/// A record as `(code, file name, line)`.
type Outcome = (String, Option<String>, Option<usize>);

/// Each reader's records for `md`: an error's, or composition's warnings.
fn outcomes(repo: &TempDir, md: &Markdown, readers: &[Reader]) -> Vec<(Reader, Vec<Outcome>)> {
    let project = |record: darkmatter::markdown::compose::expression::Diagnostic| {
        let source = record.source.as_ref();
        let file = source.and_then(|source| source.file.as_ref()).and_then(|file| file.file_name()).map(|name| name.to_string_lossy().into_owned());
        (record.code.clone(), file, source.and_then(|source| source.line))
    };
    readers
        .iter()
        .map(|reader| {
            let records = match (reader, read(repo, md, *reader)) {
                (_, Err(error)) => error_diagnostics(&error).into_iter().map(project).collect(),
                (Reader::Compose, Ok(())) => {
                    let (_, report) = md.compose_with(&request(repo.path())).unwrap();
                    report.warnings.iter().map(warning_diagnostic).map(project).collect()
                }
                (_, Ok(())) => Vec::new(),
            };
            (*reader, records)
        })
        .collect()
}

/// Siblings: a target that resolves but cannot be read (a `::code` source
/// that is not UTF-8, a `::file` child that is not UTF-8) is reported at the
/// directive that named it, by composition (which tolerates it with a
/// warning), the pre-flight walk, and the graph readers, never at no line.
#[test]
fn an_unreadable_target_names_its_directive_line() {
    let repo = repository();
    std::fs::write(repo.path().join("bin.txt"), b"\xff\xfe bin").unwrap();
    std::fs::write(repo.path().join("bad.md"), b"\xff\xfe bad").unwrap();
    let graph_readers = [Reader::Compose, Reader::Preflight, Reader::ReferenceGraph, Reader::TransclusionGraph, Reader::Validation];
    // A tolerated warning keeps its failure's code.
    for (directive, code, readers) in [
        ("::code ./bin.txt", "dm.transclusion.failure", &[Reader::Compose][..]),
        ("::file ./bad.md", "dm.input.read_failure", &graph_readers[..]),
    ] {
        let (content, line) = after_removed_block(directive);
        let md = document(&repo, "doc.md", &content);
        for (reader, records) in outcomes(&repo, &md, readers) {
            let [record] = records.as_slice() else { panic!("{directive}: {reader:?}: {records:?}") };
            assert_eq!(record, &(code.to_string(), Some("doc.md".to_string()), Some(line)), "{directive}: {reader:?}");
        }
    }
}

/// Sibling: a remote target whose fetch the deny-all network policy refuses
/// is composition's fatal fetch failure, at the directive that named the URL.
#[test]
fn a_refused_remote_target_names_its_directive_line() {
    let repo = repository();
    for directive in ["::url https://example.invalid/x.md", "::code https://example.invalid/x.rs"] {
        let (content, line) = after_removed_block(directive);
        let md = document(&repo, "doc.md", &content);
        let error = read(&repo, &md, Reader::Compose).expect_err(directive);
        assert_located(&error, "dm.transclusion.failure", "doc.md", line, directive);
    }
}

/// A child whose frontmatter does not parse is a failure of its own with no
/// inclusion site; when composition tolerates it, the warning keeps the
/// tolerated-transclusion code and is placed at the directive that included
/// the child. Lines added above the directive move it by as many lines; a
/// child that parses is transcluded with no warning.
#[test]
fn a_tolerated_child_parse_failure_is_reported_at_its_inclusion() {
    let repo = repository();
    for n in [0, 2] {
        std::fs::write(repo.path().join("part.md"), "---\ntitle: [unclosed\n---\npart text\n").unwrap();
        let md = document(&repo, "doc.md", &format!("{}# Doc\n\n::file ./part.md\n", "\n".repeat(n)));
        let (_, report) = md.compose_with(&request(repo.path())).expect("a child's failure is tolerated");
        let records: Vec<_> = report.warnings.iter().map(warning_diagnostic).collect();
        let [record] = records.as_slice() else { panic!("+{n}: {records:#?}") };
        assert_eq!(record.code, "dm.transclusion.failure", "+{n}: {record:#?}");
        let source = record.source.as_ref().unwrap_or_else(|| panic!("{record:#?}"));
        assert!(source.file.as_ref().is_some_and(|path| path.ends_with("doc.md")), "+{n}: {record:#?}");
        assert_eq!((source.line, source.column), (Some(3 + n), Some(1)), "+{n}: {record:#?}");

        std::fs::write(repo.path().join("part.md"), "---\ntitle: closed\n---\npart text\n").unwrap();
        let (composed, report) = md.compose_with(&request(repo.path())).expect("the fixed child composes");
        assert!(report.warnings.is_empty(), "+{n}: {:#?}", report.warnings);
        assert!(composed.content().contains("part text"), "+{n}");
    }
}
