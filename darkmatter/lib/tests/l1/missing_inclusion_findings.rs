//! An inclusion whose target composition could not open is reported for the
//! cause that applies: a target the resolver proved missing is the reference
//! family's `dm.reference.missing_transclusion` record, worded and detailed by
//! the shared missing-target builder (the construct, the written and resolved
//! reference, the directory it was resolved against, and the file to create);
//! a target that exists but cannot be read keeps its read failure and its
//! own hint; a target that cannot be parsed keeps its parse failure in its
//! own file. The reference check's record of the same missing target carries
//! the same evidence.

use biscuit_file::FileResolutionContext;
use biscuit_terminal::utils::escape_codes::strip_escape_codes;
use darkmatter::findings::Diagnostic;
use darkmatter::markdown::compose::report_diagnostics::{error_diagnostics, reference_issue_diagnostic};
use darkmatter::markdown::compose::{ComposeContext, ComposeOptions, ComposeRequest};
use darkmatter::markdown::reference::types::ReferenceGraphOptions;
use darkmatter::markdown::reference::validate::{ReferenceIssueCode, ReferenceValidationOptions};
use darkmatter::markdown::{Markdown, MarkdownError};
use std::path::Path;
use tempfile::TempDir;

const MISSING: &str = "dm.reference.missing_transclusion";

/// A temporary repository.
fn repository() -> TempDir {
    let repo = TempDir::new().unwrap();
    std::fs::create_dir_all(repo.path().join(".git")).unwrap();
    repo
}

/// A request in the repository's frozen context, so nothing scans the real
/// working tree.
fn request(root: &Path) -> ComposeRequest {
    request_with(root, ComposeOptions::new_with_context(ComposeContext::capture_for_content(root, "")))
}

/// [`request`] that fails on a target composition would otherwise replace
/// with a notice (a read or parse failure), as `md compose` does.
fn strict_request(root: &Path) -> ComposeRequest {
    request_with(root, ComposeOptions::new_with_context(ComposeContext::capture_for_content(root, "")).with_fail_fast(true))
}

fn request_with(root: &Path, options: ComposeOptions) -> ComposeRequest {
    let context = FileResolutionContext::from_snapshot(root, None, std::collections::HashMap::new())
        .with_repository_root(root);
    crate::request_support::request_in(options, context)
}

/// `name` written under the repository with `content`, loaded from its file.
fn document(repo: &TempDir, name: &str, content: &str) -> Markdown {
    let path = repo.path().join(name);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, content).unwrap();
    Markdown::try_from(path.as_path()).unwrap()
}

/// The one record of `md`'s failed composition.
fn failure(repo: &TempDir, md: &Markdown) -> (MarkdownError, Diagnostic) {
    failure_under(md, &request(repo.path()))
}

fn failure_under(md: &Markdown, request: &ComposeRequest) -> (MarkdownError, Diagnostic) {
    let error = md.compose_with(request).expect_err("the inclusion fails");
    let records = error_diagnostics(&error);
    assert_eq!(records.len(), 1, "{records:#?}");
    (error, records.into_iter().next().unwrap())
}

/// `md`'s failure as its plain status block.
fn status_block(error: &MarkdownError) -> String {
    let block = darkmatter::markdown::errors::as_block_error(error).expect("a block error");
    strip_escape_codes(block.report_block_error_optimistic(Some(100)))
}

fn forward(path: &Path) -> String {
    biscuit_file::to_forward_slash_string(&biscuit_file::canonicalize_simplified(path).unwrap())
}

/// `path`, which may not exist, under its canonical parent.
fn canonical_missing(path: &Path) -> String {
    format!("{}/{}", forward(path.parent().unwrap()), path.file_name().unwrap().to_string_lossy())
}

/// `record`'s base and resolved paths, canonicalized for comparison (macOS
/// `/var` versus `/private/var`).
fn evidence(record: &Diagnostic) -> (String, String) {
    let details = record.reference_details.as_ref().expect("reference details");
    let canonical = |path: &str| {
        let path = Path::new(path);
        match path.exists() {
            true => forward(path),
            false => canonical_missing(path),
        }
    };
    (canonical(details.base.as_deref().expect("base")), canonical(details.resolved.as_deref().expect("resolved")))
}

#[test]
fn a_missing_file_target_is_the_missing_transclusion_finding_at_the_directive() {
    let repo = repository();
    let md = document(&repo, "notes/doc.md", "---\ntitle: x\n---\n\n# Doc\n\n::file ./missing.md\n");
    let (_, record) = failure(&repo, &md);

    assert_eq!(record.code, MISSING);
    assert_eq!(record.message, "file not found in `::file`");
    assert_eq!(record.hint.as_deref(), Some("create missing.md, or correct the reference"));
    let details = record.reference_details.as_ref().unwrap();
    assert_eq!(details.construct.as_deref(), Some("::file"));
    assert_eq!(details.reference.as_deref(), Some("./missing.md"));
    assert_eq!(details.failure.as_deref(), Some("no-match"));
    assert_eq!(evidence(&record), (forward(&repo.path().join("notes")), canonical_missing(&repo.path().join("notes/missing.md"))));
    let source = record.source.as_ref().unwrap();
    assert!(source.file.as_deref().is_some_and(|file| file.ends_with("doc.md")), "{source:?}");
    assert_eq!((source.line, source.column), (Some(7), Some(1)));
    // The permissions advice belongs to a read failure, never to a target
    // proven missing.
    assert!(!record.hint.as_deref().unwrap_or_default().contains("permission"));
}

#[test]
fn a_missing_target_suggests_a_nearby_file() {
    let repo = repository();
    std::fs::write(repo.path().join("chapter.md"), "# Chapter\n").unwrap();
    let md = document(&repo, "doc.md", "# Doc\n\n::file ./chaptr.md\n");
    let (_, record) = failure(&repo, &md);

    let suggestions = record.reference_details.as_ref().unwrap().suggestions.clone().unwrap_or_default();
    assert_eq!(suggestions.len(), 1, "{suggestions:?}");
    assert!(suggestions[0].ends_with("/chapter.md"), "{suggestions:?}");
}

#[test]
fn a_missing_prologue_target_names_the_key_that_wrote_it() {
    let repo = repository();
    let md = document(&repo, "doc.md", "---\nprologue: ./intro.md\n---\nBody\n");
    let (_, record) = failure(&repo, &md);

    assert_eq!(record.code, MISSING);
    assert_eq!(record.message, "file not found in prologue:");
    assert_eq!(record.reference_details.as_ref().unwrap().construct.as_deref(), Some("prologue:"));
    assert_eq!(record.hint.as_deref(), Some("create intro.md, or correct the reference"));
    let source = record.source.as_ref().unwrap();
    assert_eq!((source.line, source.column), (Some(2), Some(11)));
}

#[test]
fn a_missing_target_status_block_gives_the_missing_file_hint() {
    let repo = repository();
    let md = document(&repo, "doc.md", "# Doc\n\n::file ./missing.md\n");
    let (error, _) = failure(&repo, &md);
    let rendered = status_block(&error);

    assert!(rendered.contains("TransclusionError: file not found in ::file"), "{rendered}");
    assert!(rendered.contains("File not found: ./missing.md"), "{rendered}");
    assert!(rendered.contains("Resolved:"), "{rendered}");
    assert!(rendered.contains("failure: no-match"), "{rendered}");
    assert!(rendered.contains("Create missing.md, or correct the reference."), "{rendered}");
    assert!(!rendered.contains("permissions"), "{rendered}");
}

#[test]
fn crlf_and_lf_documents_report_the_same_position() {
    let positions: Vec<_> = ["\n", "\r\n"]
        .into_iter()
        .map(|eol| {
            let repo = repository();
            let text = ["---", "title: x", "---", "", "::file ./missing.md", ""].join(eol);
            let md = document(&repo, "doc.md", &text);
            let (_, record) = failure(&repo, &md);
            let source = record.source.unwrap();
            (source.line, source.column)
        })
        .collect();
    assert_eq!(positions, vec![(Some(5), Some(1)); 2]);
}

#[test]
fn a_crlf_unterminated_block_excerpt_numbers_file_lines() {
    let repo = repository();
    let md = document(&repo, "doc.md", "---\r\ntitle: x\r\n---\r\n::block when=\"title\"\r\nbody\r\n");
    let error = md.compose_with(&request(repo.path())).expect_err("an unterminated block is refused");
    let rendered = status_block(&error);
    assert!(rendered.contains("> 4 │ ::block when=\"title\""), "{rendered}");
    assert!(!rendered.contains("> 1 │"), "{rendered}");
    let record = error_diagnostics(&error).remove(0);
    let source = record.source.unwrap();
    assert_eq!((source.line, source.column), (Some(4), Some(1)));
}

#[test]
fn an_unparseable_target_keeps_its_parse_failure_in_its_own_file() {
    let repo = repository();
    std::fs::write(repo.path().join("bad.md"), "---\ntitle: [\n---\nx\n").unwrap();
    let md = document(&repo, "doc.md", "# Doc\n\n::file ./bad.md\n");
    let (_, record) = failure_under(&md, &strict_request(repo.path()));

    assert_eq!(record.code, "dm.frontmatter.yaml_parse");
    assert!(record.hint.is_none(), "{record:#?}");
    let source = record.source.as_ref().unwrap();
    assert!(source.file.as_deref().is_some_and(|file| file.ends_with("bad.md")), "{source:?}");
}

/// A target that exists but cannot be read is no missing target: it keeps
/// its read failure and the permissions advice. Unix only: a mode-`000` file
/// is readable to an elevated Windows token, and to root.
#[cfg(unix)]
#[test]
fn an_unreadable_target_keeps_its_read_failure_and_hint() {
    use std::os::unix::fs::PermissionsExt;
    let repo = repository();
    let locked = repo.path().join("locked.md");
    std::fs::write(&locked, "# Locked\n").unwrap();
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000)).unwrap();
    if std::fs::read(&locked).is_ok() {
        // Running as root: the mode does not deny the read.
        return;
    }
    let md = document(&repo, "doc.md", "# Doc\n\n::file ./locked.md\n");
    let (error, record) = failure_under(&md, &strict_request(repo.path()));
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o644)).unwrap();

    assert_eq!(record.code, "dm.input.read_failure");
    assert!(record.hint.is_none(), "{record:#?}");
    assert!(record.message.contains("Permission denied"), "{record:#?}");
    let source = record.source.as_ref().unwrap();
    assert_eq!((source.line, source.column), (Some(3), Some(1)));
    let rendered = status_block(&error);
    assert!(rendered.contains("TransclusionError: I/O failure"), "{rendered}");
    assert!(rendered.contains("Check file existence and permissions."), "{rendered}");
}

#[test]
fn the_reference_checks_missing_target_record_carries_the_same_evidence() {
    let repo = repository();
    let md = document(&repo, "notes/doc.md", "# Doc\n\n::file ./missing.md\n\n[gone](./gone.md)\n");
    let request = request(repo.path());
    let graph = ReferenceGraphOptions::with_compose(&request);
    let report = md.validate_references(ReferenceValidationOptions::with_graph(graph)).expect("validation runs");
    let records: Vec<Diagnostic> = report
        .issues
        .iter()
        .filter(|issue| issue.code == ReferenceIssueCode::MissingLocalTarget)
        .map(reference_issue_diagnostic)
        .collect();
    assert_eq!(records.len(), 2, "{records:#?}");

    let inclusion = records.iter().find(|record| record.code == MISSING).expect("the inclusion's record");
    assert_eq!(inclusion.hint.as_deref(), Some("create missing.md, or correct the reference"));
    assert_eq!(evidence(inclusion), (forward(&repo.path().join("notes")), canonical_missing(&repo.path().join("notes/missing.md"))));
    // The record keeps the reference report's code and message.
    assert_eq!(inclusion.message, "Missing local target: ./missing.md");

    let link = records.iter().find(|record| record.code == "dm.reference.missing_hyperlink").expect("the link's record");
    assert_eq!(link.hint.as_deref(), Some("create gone.md, or correct the reference"));
    assert_eq!(evidence(link).1, canonical_missing(&repo.path().join("notes/gone.md")));
}
