//! A failure keeps its specific diagnostic code wherever it is projected: a
//! `::toc-linking` or `::file-links` failure composition tolerates is a
//! warning under the code its error reports, a `when=` condition that does
//! not parse is `dm.expression.parse_failure` from composition and every
//! graph reader, and a typed reference failure a `FileTree` returns is
//! `dm.reference.failure`, as reference validation reports it. A heading
//! normalization failure is `dm.normalization.*`, and a transcluded heading
//! re-leveled past H6 warns as `dm.transclusion.heading_overflow`.

use biscuit_file::FileResolutionContext;
use darkmatter::markdown::compose::expression::Severity;
use darkmatter::markdown::compose::report_diagnostics::{error_diagnostics, warning_diagnostic};
use darkmatter::markdown::compose::{ComposeContext, ComposeOptions, ComposeRequest};
use darkmatter::markdown::reference::file_tree::FileTree;
use darkmatter::markdown::reference::types::ReferenceGraphOptions;
use darkmatter::markdown::reference::validate::ReferenceValidationOptions;
use darkmatter::markdown::compose::policy::{BlockingPolicy, COMPOSE_DIAGNOSTIC_CODES};
use darkmatter::markdown::normalize::NormalizationError;
use darkmatter::markdown::{HeadingLevel, Markdown, MarkdownError};
use std::path::Path;
use tempfile::TempDir;

/// A temporary repository holding `files/a.md` and the Markdown file
/// `notdir.md`.
fn repository() -> TempDir {
    let repo = TempDir::new().unwrap();
    std::fs::create_dir_all(repo.path().join(".git")).unwrap();
    std::fs::create_dir_all(repo.path().join("files")).unwrap();
    std::fs::write(repo.path().join("files/a.md"), "# A\n\n## Part\n").unwrap();
    std::fs::write(repo.path().join("notdir.md"), "# Not a directory\n").unwrap();
    repo
}

/// A request in the repository's frozen context, so nothing scans the real
/// working tree.
fn request(root: &Path) -> ComposeRequest {
    let context = FileResolutionContext::from_snapshot(root, None, std::collections::HashMap::new())
        .with_repository_root(root);
    crate::request_support::request_in(
        ComposeOptions::new_with_context(ComposeContext::capture_for_content(root, "")),
        context,
    )
}

/// `doc.md` written with `content` and loaded from its file.
fn document(repo: &TempDir, content: &str) -> Markdown {
    let path = repo.path().join("doc.md");
    std::fs::write(&path, content).unwrap();
    Markdown::try_from(path.as_path()).unwrap()
}

/// The code of the one warning `md`'s composition reports, after checking
/// the failure was tolerated as a warning.
fn tolerated_code(md: &Markdown, request: &ComposeRequest, case: &str) -> String {
    let (_, report) = md.compose_with(request).unwrap_or_else(|error| panic!("{case}: tolerated, got {error}"));
    let [warning] = report.warnings.as_slice() else {
        panic!("{case}: one warning: {:?}", report.warnings);
    };
    let record = warning_diagnostic(warning);
    assert_eq!(record.severity, Severity::Warning, "{case}: {record:?}");
    record.code
}

#[test]
fn tolerated_toc_linking_and_file_links_failures_keep_their_codes() {
    let repo = repository();
    let request = request(repo.path());
    for (directive, code) in [
        ("::toc-linking ./files/a.md filter=\"[\"", "dm.toc_linking.invalid_filter"),
        ("::file-links ./files/[.md", "dm.file_links.invalid_glob"),
        ("::file-links --dir ./absent", "dm.reference.failure"),
        ("::file-links --dir ./notdir.md", "dm.file_links.not_a_directory"),
    ] {
        let md = document(&repo, &format!("# Doc\n\n{directive}\n"));
        assert_eq!(tolerated_code(&md, &request, directive), code, "{directive}");
    }

    // A valid directory and glob are controls: nothing to tolerate.
    for directive in ["::file-links --dir ./files", "::file-links ./files/*.md", "::toc-linking ./files/a.md"] {
        let md = document(&repo, &format!("# Doc\n\n{directive}\n"));
        let (_, report) = md.compose_with(&request).unwrap_or_else(|error| panic!("{directive}: {error}"));
        assert!(report.warnings.is_empty(), "{directive}: {:?}", report.warnings);
    }
}

/// Text with no source file (`md compose -` reads stdin) cannot root a glob.
#[test]
fn a_tolerated_file_links_glob_without_a_source_keeps_its_code() {
    let repo = repository();
    let md = Markdown::from("# Doc\n\n::file-links ./files/*.md\n".to_string());
    let request = crate::request_support::request_at(
        repo.path(),
        ComposeOptions::new_with_context(ComposeContext::capture_for_content(repo.path(), "")),
    );
    assert_eq!(tolerated_code(&md, &request, "stdin"), "dm.file_links.missing_source");
}

#[test]
fn a_malformed_when_condition_is_an_expression_parse_failure_from_every_reader() {
    let repo = repository();
    let condition = |when: &str| document(&repo, &format!("---\nflag: null\n---\n# Doc\n\n::file ./files/a.md when=\"{when}\"\n"));
    let request = request(repo.path());
    let graph = ReferenceGraphOptions::with_compose(&request);
    for (when, code) in [("1 +", "dm.expression.parse_failure"), ("1 / 0", "dm.expression.evaluation_failure")] {
        let md = condition(when);
        let results: [(&str, Result<(), MarkdownError>); 5] = [
            ("compose", md.compose_with(&request).map(drop)),
            ("reference graph", md.reference_graph(graph.clone()).map(drop)),
            ("transclusion graph", md.transclusion_graph(graph.clone()).map(drop)),
            ("flattened references", md.composed_references(graph.clone()).map(drop)),
            ("validation", md.validate_references(ReferenceValidationOptions::with_graph(graph.clone())).map(drop)),
        ];
        for (reader, result) in results {
            let error = result.expect_err(&format!("{when}: {reader}"));
            let records = error_diagnostics(&error);
            let source = records[0].source.as_ref().expect("a file");
            assert_eq!((records[0].code.as_str(), source.line), (code, Some(6)), "{when}: {reader}: {error}");
        }
        // The local query reads syntax only and keeps the raw directive.
        let local = md.transclusions_with_options(&request).unwrap_or_else(|error| panic!("{when}: local query: {error}"));
        assert_eq!(local.len(), 1, "{when}: {local:?}");
    }
}

#[test]
fn a_file_tree_reports_a_typed_reference_failure_under_its_code() {
    let repo = repository();
    let request = request(repo.path());
    for content in ["# Doc\n\n::file @\n", "---\nprologue: \"@\"\n---\n# Doc\n"] {
        for validate in [false, true] {
            let tree = FileTree::from_markdown(document(&repo, content), &request);
            let mut tree = if validate { tree.validate() } else { tree };
            let error = tree.ensure_built().expect_err(content);
            let [record] = error_diagnostics(&error).try_into().unwrap_or_else(|records| panic!("{content}: {records:?}"));
            assert_eq!(record.code, "dm.reference.failure", "{content} (validate: {validate}): {record:?}");
            assert!(record.reference_details.is_some(), "{content} (validate: {validate}): {record:?}");
        }
    }
}

/// A transclusion whose target matches no file is the missing-transclusion
/// code, not the generic transclusion failure; an existing target composes.
#[test]
fn a_transclusion_target_that_matches_no_file_is_a_missing_transclusion() {
    let repo = repository();
    let request = request(repo.path());

    let md = document(&repo, "# Doc\n\n::file ./files/absent.md\n");
    let failure = md.compose_with(&request).expect_err("a missing target fails");
    let codes: Vec<String> = error_diagnostics(&failure).into_iter().map(|record| record.code).collect();
    assert_eq!(codes, ["dm.reference.missing_transclusion"], "{failure}");

    let md = document(&repo, "# Doc\n\n::file ./files/a.md\n");
    md.compose_with(&request).expect("an existing target composes");
}

/// A public re-level or normalization that would push a heading past H6 is
/// `dm.normalization.level_overflow`, bare or carried by `MarkdownError`; a
/// structure violation is `dm.normalization.validation_failed`.
#[test]
fn a_normalization_failure_reports_its_specific_code() {
    let md = Markdown::from("#### Four\n\n##### Five\n\n###### Six\n".to_string());
    let relevel = md.relevel(HeadingLevel::H5).expect_err("relevel overflows");
    let normalize = md.normalize(Some(HeadingLevel::H5)).expect_err("normalize overflows");
    for error in [relevel, normalize] {
        assert!(matches!(error, NormalizationError::LevelOverflow { .. }), "{error:?}");
        assert_eq!(error_diagnostics(&error)[0].code, "dm.normalization.level_overflow", "{error}");
        let carried = MarkdownError::from(error);
        let [record] = error_diagnostics(&carried).try_into().unwrap_or_else(|records| panic!("{records:?}"));
        assert_eq!((record.code.as_str(), record.severity), ("dm.normalization.level_overflow", Severity::Error));
    }
    let violation = MarkdownError::from(NormalizationError::ValidationFailed(Vec::new()));
    assert_eq!(error_diagnostics(&violation)[0].code, "dm.normalization.validation_failed");

    // Re-leveling within H6 is the control, and so is composing a document
    // whose headings skip levels: normalization changes nothing it can fail on.
    md.relevel(HeadingLevel::H4).expect("within H6");
    let repo = repository();
    let doc = document(&repo, "### Three\n\n# One\n\n###### Six\n");
    let (_, report) = doc.compose_with(&request(repo.path())).expect("composes");
    assert!(report.warnings.is_empty(), "{:?}", report.warnings);
}

/// A transcluded heading pushed past H6 becomes bold text with a warning
/// under its own code, which `--deny` can name; a child that fits is the
/// control.
#[test]
fn a_transcluded_heading_overflow_warns_under_its_code() {
    const CODE: &str = "dm.transclusion.heading_overflow";
    assert!(COMPOSE_DIAGNOSTIC_CODES.iter().any(|(code, _)| *code == CODE));
    let repo = repository();
    std::fs::write(repo.path().join("files/deep.md"), "## Child\n\n### Deep\n").unwrap();
    let md = document(&repo, "###### Root\n\n::file ./files/deep.md\n");
    let request = request(repo.path());
    let (composed, report) = md.compose_with(&request).expect("tolerated");
    assert!(composed.content().contains("**Deep**"), "{}", composed.content());
    let codes: Vec<String> = report.warnings.iter().map(|warning| warning_diagnostic(warning).code).collect();
    assert_eq!(codes, [CODE]);

    let denied = request.clone().map_options(|options| options.with_blocking_policy(BlockingPolicy::default().deny_code(CODE)));
    let error = md.compose_with(&denied).expect_err("denied");
    assert!(error_diagnostics(&error).iter().any(|record| record.code == CODE), "{error}");

    let fits = document(&repo, "# Root\n\n::file ./files/deep.md\n");
    let (_, report) = fits.compose_with(&request).expect("fits");
    assert!(report.warnings.is_empty(), "{:?}", report.warnings);
}
