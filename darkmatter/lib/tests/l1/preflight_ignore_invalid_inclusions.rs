//! Pre-flight takes composition's `ignore_invalid` skip decision. Under the
//! document's `ignore_invalid: true` (or the request's
//! `with_ignore_invalid_references(Some(true))`) composition removes a body
//! `::file`/`::code` or a `prologue`/`epilogue` entry whose target does not
//! resolve, with a warning; pre-flight skips the same inclusion and walks the
//! valid sibling beside it instead of refusing the document. Without it both
//! refuse. A frontmatter value that does not parse as a file reference fails
//! before any resolution, so it refuses in both walks even under
//! `ignore_invalid`. A copy a collection body repeated takes the same
//! decision, and with remote transclusion disabled a repeated URL copy fails
//! or is skipped exactly as a written one is.

use biscuit_file::FileResolutionContext;
use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::shell_expansion::template::ShellPreApprovals;
use darkmatter::markdown::compose::transclusion::TransclusionError;
use darkmatter::markdown::compose::{ComposeContext, ComposeOptions, ComposeRequest};
use darkmatter::markdown::{MarkdownError, MarkdownResult};
use std::path::Path;
use tempfile::TempDir;

/// A temporary repository holding `side.md`, whose one shell command marks
/// it as discovered, and the plain-text `notes.txt`, which `::file`,
/// `prologue`, and `epilogue` refuse as no Markdown file.
fn repository() -> TempDir {
    let repo = TempDir::new().unwrap();
    std::fs::create_dir_all(repo.path().join(".git")).unwrap();
    std::fs::write(repo.path().join("side.md"), "Side\n\n::shell echo side\n").unwrap();
    std::fs::write(repo.path().join("notes.txt"), "plain text\n").unwrap();
    repo
}

/// A request in the repository's frozen context, so nothing scans the real
/// working tree; `ignore` is the request's own `ignore_invalid` setting.
fn request(root: &Path, ignore: Option<bool>) -> ComposeRequest {
    let context = FileResolutionContext::from_snapshot(root, None, std::collections::HashMap::new())
        .with_repository_root(root);
    crate::request_support::request_in(
        ComposeOptions::new_with_context(ComposeContext::capture_for_content(root, ""))
            .with_ignore_invalid_references(ignore),
        context,
    )
}

/// `doc.md` with `frontmatter` lines (plus `ignore_invalid: true` when
/// `ignore`) over `body`.
fn document(repo: &TempDir, frontmatter: &str, ignore: bool, body: &str) -> Markdown {
    let path = repo.path().join("doc.md");
    let flag = if ignore { "ignore_invalid: true\n" } else { "" };
    std::fs::write(&path, format!("---\ntitle: doc\n{flag}{frontmatter}---\n{body}")).unwrap();
    Markdown::try_from(path.as_path()).unwrap()
}

fn approvals(md: &Markdown, request: &ComposeRequest) -> MarkdownResult<Vec<String>> {
    md.compose_preflight(request).map(|report| report.approval_set())
}

/// Composes `md` the way the CLI does after a successful walk: under the
/// walk's approvals and with its graph. Returns the body and the warnings.
fn compose_after_preflight(md: &Markdown, request: &ComposeRequest) -> MarkdownResult<(String, Vec<String>)> {
    let report = md.compose_preflight(request)?;
    let approvals: ShellPreApprovals = report.approval_set().into_iter().collect();
    let request = request.clone().map_options(|options| {
        options.with_pre_approved_commands(approvals).with_preflight_graph(report.preflight_graph)
    });
    md.compose_with(&request).map(|(composed, report)| {
        (composed.content().to_string(), report.warnings.iter().map(|warning| warning.message.clone()).collect())
    })
}

/// The transclusion failure `error` carries, past a repeated copy's
/// iteration frame.
fn transclusion(error: &MarkdownError) -> &TransclusionError {
    match error {
        MarkdownError::Transclusion(inner) => inner,
        MarkdownError::InIteration(failure) => transclusion(&failure.error),
        other => panic!("a transclusion error: {other:?}"),
    }
}

/// Under `ignore_invalid` the unsupported target is skipped in both walks
/// and the sibling's command is discovered and run; without it both refuse.
fn assert_skipped_only_under_ignore_invalid(frontmatter: &str, body: &str) {
    let repo = repository();
    let md = document(&repo, frontmatter, true, body);
    let request = request(repo.path(), None);
    let found = approvals(&md, &request).unwrap_or_else(|error| panic!("pre-flight: {error}"));
    assert_eq!(found, ["echo side"]);
    let (composed, warnings) = compose_after_preflight(&md, &request).unwrap_or_else(|error| panic!("compose: {error}"));
    assert!(composed.contains("side"), "{composed}");
    assert!(!composed.contains("plain text"), "{composed}");
    assert!(warnings.iter().any(|warning| warning.contains("notes.txt")), "{warnings:?}");

    let md = document(&repo, frontmatter, false, body);
    let error = approvals(&md, &request).expect_err("pre-flight refuses without ignore_invalid");
    assert!(matches!(transclusion(&error).without_site(), TransclusionError::UnsupportedFileType { .. }), "{error:?}");
    md.compose_with(&request).expect_err("composition refuses it too");
}

#[test]
fn an_unsupported_body_file_is_skipped_under_ignore_invalid() {
    assert_skipped_only_under_ignore_invalid("", "Body\n\n::file ./notes.txt\n\n::file ./side.md\n");
}

#[test]
fn an_unsupported_prologue_entry_is_skipped_under_ignore_invalid() {
    assert_skipped_only_under_ignore_invalid("prologue: [./notes.txt, ./side.md]\n", "Body\n");
}

#[test]
fn an_unsupported_epilogue_entry_is_skipped_under_ignore_invalid() {
    assert_skipped_only_under_ignore_invalid("epilogue: [./notes.txt, ./side.md]\n", "Body\n");
}

/// A copy a collection body repeated takes the same skip: composition
/// removes the unsupported copy pre-flight skipped instead of refusing it as
/// unapproved.
#[test]
fn an_unsupported_repeated_inclusion_is_skipped_under_ignore_invalid() {
    assert_skipped_only_under_ignore_invalid(
        "$schema:\n  names: string[](required)\nnames: [./notes.txt, ./side.md]\n",
        "::map names\n::file {{ i }}\n::end-map\n",
    );
}

/// The request's own setting decides like the document's frontmatter.
#[test]
fn the_request_setting_skips_like_the_frontmatter() {
    let repo = repository();
    let md = document(&repo, "", false, "Body\n\n::file ./notes.txt\n\n::file ./side.md\n");
    let request = request(repo.path(), Some(true));
    assert_eq!(approvals(&md, &request).unwrap_or_else(|error| panic!("{error}")), ["echo side"]);
    let (composed, _) = compose_after_preflight(&md, &request).unwrap_or_else(|error| panic!("compose: {error}"));
    assert!(composed.contains("side"), "{composed}");
}

/// A `::code` target that does not resolve is skipped in both walks.
#[test]
fn an_unresolved_code_target_is_skipped_under_ignore_invalid() {
    let repo = repository();
    let body = "Body\n\n::code ./missing/\n\n::file ./side.md\n";
    let md = document(&repo, "", true, body);
    let request = request(repo.path(), None);
    assert_eq!(approvals(&md, &request).unwrap_or_else(|error| panic!("{error}")), ["echo side"]);
    let (composed, _) = compose_after_preflight(&md, &request).unwrap_or_else(|error| panic!("compose: {error}"));
    assert!(composed.contains("side"), "{composed}");
    assert!(!composed.contains("::code"), "{composed}");

    let md = document(&repo, "", false, body);
    approvals(&md, &request).expect_err("pre-flight refuses without ignore_invalid");
}

/// A frontmatter value that is no file reference (`@`) fails before it is
/// resolved, so `ignore_invalid` does not cover it in either walk.
#[test]
fn a_malformed_frontmatter_entry_refuses_under_ignore_invalid() {
    let repo = repository();
    for key in ["prologue", "epilogue"] {
        let md = document(&repo, &format!("{key}: \"@\"\n"), true, "Body\n");
        let request = request(repo.path(), None);
        approvals(&md, &request).expect_err("pre-flight refuses a malformed entry");
        md.compose_with(&request).expect_err("composition refuses it too");
    }
}

/// With remote transclusion disabled a repeated URL copy, which pre-flight
/// never walks, is skipped under `ignore_invalid` and otherwise fails with
/// the same `UrlExecutionDisabled` a written `::file <url>` fails with.
#[test]
fn a_repeated_url_copy_agrees_with_a_written_one_when_remote_is_disabled() {
    let repo = repository();
    let url = "https://example.invalid/remote.md";
    let repeated = format!(
        "---\ntitle: doc\n$schema:\n  names: string[](required)\nnames: [\"{url}\", ./side.md]\n---\n::map names\n::file {{{{ i }}}}\n::end-map\n"
    );
    let written = format!("---\ntitle: doc\n---\n::file {url}\n\n::file ./side.md\n");
    let path = repo.path().join("doc.md");
    for text in [&repeated, &written] {
        std::fs::write(&path, text).unwrap();
        let md = Markdown::try_from(path.as_path()).unwrap();
        let error = compose_after_preflight(&md, &request(repo.path(), None)).expect_err("a disabled URL fails");
        assert!(matches!(transclusion(&error).without_site(), TransclusionError::UrlExecutionDisabled { .. }), "{text}: {error:?}");

        let (composed, _) = compose_after_preflight(&md, &request(repo.path(), Some(true)))
            .unwrap_or_else(|error| panic!("{text}: {error}"));
        assert!(composed.contains("side"), "{composed}");
        assert!(!composed.contains("example.invalid"), "{composed}");
    }
}
