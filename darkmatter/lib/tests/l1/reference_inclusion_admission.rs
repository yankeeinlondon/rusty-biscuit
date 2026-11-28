//! A reference check admits an inclusion's target by the rule of its
//! directive, the one composition applies: `::file`, `prologue`, and
//! `epilogue` open only a Markdown file (`.md`, `.markdown`), `::code` any
//! file, and `::url` only an absolute URL. A target the rule refuses is
//! composition's own typed error, at the directive or key that wrote it, in
//! every reader that judges the target; the graph never opens it as a child,
//! and no URL it refuses reaches the remote request batch. The local query
//! is a lookup, not an admission, and still reports where a file target
//! resolves.

use biscuit_file::FileResolutionContext;
use darkmatter::markdown::compose::report_diagnostics::error_diagnostics;
use darkmatter::markdown::compose::{ComposeContext, ComposeOptions, ComposeRequest};
use darkmatter::markdown::reference::file_tree::FileTree;
use darkmatter::markdown::reference::types::{ReferenceGraphOptions, ReferenceSyntax, ReferenceTarget};
use darkmatter::markdown::reference::validate::{ReferenceIssueCode, ReferenceValidationOptions};
use darkmatter::markdown::Markdown;
use std::error::Error;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use tempfile::TempDir;

/// A temporary repository holding `side.md`, the plain-text `side.txt`, and
/// the directory `dir`.
fn repository() -> TempDir {
    let repo = TempDir::new().unwrap();
    std::fs::create_dir_all(repo.path().join(".git")).unwrap();
    std::fs::create_dir_all(repo.path().join("dir")).unwrap();
    std::fs::write(repo.path().join("side.md"), "# Side\n").unwrap();
    std::fs::write(repo.path().join("side.txt"), "plain text\n").unwrap();
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

/// A loopback HTTP server answering every request with `200 OK`, its
/// `host:port`, and the number of requests it has answered.
fn loopback() -> (String, Arc<AtomicUsize>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("a loopback port");
    let port = listener.local_addr().expect("address").port();
    let requests = Arc::new(AtomicUsize::new(0));
    let counted = Arc::clone(&requests);
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let mut reader = BufReader::new(&stream);
            let mut line = String::new();
            while reader.read_line(&mut line).is_ok_and(|read| read > 2) {
                line.clear();
            }
            counted.fetch_add(1, Ordering::SeqCst);
            let _ = (&stream).write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
        }
    });
    (format!("127.0.0.1:{port}"), requests)
}

fn validation(graph: ReferenceGraphOptions, remote: bool) -> ReferenceValidationOptions {
    let mut options = ReferenceValidationOptions::with_graph(graph);
    options.validate_remote = remote;
    options.remote_timeout = Duration::from_secs(2);
    options
}

/// Every reader that judges an inclusion target, with what it returned: the
/// error, or the issue codes of its report.
type Judged = Vec<(&'static str, Result<Vec<ReferenceIssueCode>, Box<dyn Error>>)>;

fn issue_codes(report: &darkmatter::markdown::reference::validate::ReferenceValidationReport) -> Vec<ReferenceIssueCode> {
    report.issues.iter().map(|issue| issue.code).collect()
}

/// Runs composition, the pre-flight walk, both graphs, flattening, fresh,
/// prebuilt, and FileTree validation (with and without following), with
/// `remote` validation, over `md`.
fn judge(repo: &TempDir, md: &Markdown, remote: bool) -> Judged {
    let request = request(repo.path());
    let graph = ReferenceGraphOptions::with_compose(&request);
    let boxed = |error: darkmatter::markdown::MarkdownError| -> Box<dyn Error> { Box::new(error) };
    let mut judged: Judged = vec![
        ("compose", md.compose_with(&request).map(|_| Vec::new()).map_err(boxed)),
        ("preflight", md.compose_preflight(&request).map(|_| Vec::new()).map_err(boxed)),
        ("reference graph", md.reference_graph(graph.clone()).map(|_| Vec::new()).map_err(boxed)),
        ("transclusion graph", md.transclusion_graph(graph.clone()).map(|_| Vec::new()).map_err(boxed)),
        ("flattened", md.composed_references(graph.clone()).map(|_| Vec::new()).map_err(boxed)),
        ("fresh", md.validate_references(validation(graph.clone(), remote)).map(|r| issue_codes(&r)).map_err(boxed)),
    ];
    let prebuilt = match md.reference_graph(graph.clone()) {
        Ok(built) => md.validate_references_with_graph(&built, validation(graph, remote)).map(|r| issue_codes(&r)).map_err(boxed),
        Err(error) => Err(boxed(error)),
    };
    judged.push(("prebuilt", prebuilt));
    if !remote {
        for (name, follow) in [("file tree", false), ("followed file tree", true)] {
            let tree = FileTree::from_markdown(md.clone(), &request).validate();
            let mut tree = if follow { tree.follow_transclusions() } else { tree };
            let result = tree
                .ensure_built()
                .map(|()| issue_codes(tree.validation_report().expect("a validating tree has a report")))
                .map_err(|error| -> Box<dyn Error> { Box::new(error) });
            judged.push((name, result));
        }
    }
    judged
}

/// `(code, file name, line)` of the one record `error` publishes.
fn located(error: &(dyn Error + 'static), shape: &str) -> (String, String, Option<usize>) {
    let records = error_diagnostics(error);
    let [record] = records.as_slice() else { panic!("{shape}: {error}: {records:#?}") };
    let source = record.source.as_ref().unwrap_or_else(|| panic!("{shape}: {error}: {record:#?}"));
    let file = source.file.as_ref().and_then(|file| file.file_name()).map(|name| name.to_string_lossy().into_owned());
    (record.code.clone(), file.unwrap_or_default(), source.line)
}

/// Asserts every reader not in `listing` refuses `content` with composition's
/// own code at `doc.md` line `line`, and every reader in `listing` succeeds.
fn assert_refused(repo: &TempDir, content: &str, line: usize, code: &str, listing: &[&str], remote: bool) {
    let md = document(repo, content);
    for (reader, result) in judge(repo, &md, remote) {
        let shape = format!("{content:?}: {reader}");
        match result {
            Ok(issues) => assert!(listing.contains(&reader), "{shape} must refuse, gave {issues:?}"),
            Err(error) => {
                assert!(!listing.contains(&reader), "{shape} lists without judging: {error}");
                assert_eq!(located(error.as_ref(), &shape), (code.to_string(), "doc.md".to_string(), Some(line)), "{shape}");
            }
        }
    }
}

/// Asserts every reader accepts `content` with no issue.
fn assert_clean(repo: &TempDir, content: &str) {
    let md = document(repo, content);
    for (reader, result) in judge(repo, &md, false) {
        match result {
            Ok(issues) => assert!(issues.is_empty(), "{content:?}: {reader}: {issues:?}"),
            Err(error) => panic!("{content:?}: {reader}: {error}"),
        }
    }
}

/// A header, then `directive` on file line 7.
fn body(directive: &str) -> String {
    format!("---\ntitle: x\n---\n\n# Doc\n\n{directive}\n")
}

/// The same body after a false page block: `directive` is on file line 10.
fn after_removed_block(directive: &str) -> String {
    format!("---\ntitle: x\n---\n\n::block when=\"false\"\nhidden\n::end-block\n# Doc\n\n{directive}\n")
}

const TRANSCLUSION_FAILURE: &str = "dm.transclusion.failure";
const REFERENCE_FAILURE: &str = "dm.reference.failure";

/// R13-F1.1: a `::file` naming an existing plain-text file, bare and
/// quoted, is the unsupported-file-type error composition raises, at the
/// directive's line, in every reader; no graph opens it as a child. The
/// Markdown file is the control. The local query still reports where the
/// target resolves.
#[test]
fn a_file_inclusion_of_a_text_file_is_refused_at_its_directive() {
    let repo = repository();
    for target in ["./side.txt", "\"./side.txt\""] {
        let directive = format!("::file {target}");
        assert_refused(&repo, &body(&directive), 7, TRANSCLUSION_FAILURE, &[], false);
        assert_refused(&repo, &after_removed_block(&directive), 10, TRANSCLUSION_FAILURE, &[], false);

        let md = document(&repo, &body(&directive));
        let local = md.transclusions_with_options(&request(repo.path())).expect("the lookup resolves the target");
        assert!(
            local[0].resolved_target.as_deref().is_some_and(|path| path.ends_with("side.txt")),
            "{local:?}"
        );
    }
    assert_clean(&repo, &body("::file ./side.md"));
    assert_clean(&repo, &body("::file \"./side.md\""));
}

/// R13-F1.2: `prologue` and `epilogue` naming the text file are refused at
/// their key, as scalar and as list entry; the Markdown file and inline
/// Markdown text are accepted.
#[test]
fn a_frontmatter_inclusion_of_a_text_file_is_refused_at_its_key() {
    let repo = repository();
    for key in ["prologue", "epilogue"] {
        for (value, line) in [("\"./side.txt\"", 3), ("\n  - ./side.md\n  - \"./side.txt\"", 5)] {
            let content = format!("---\ntitle: x\n{key}: {value}\n---\n\n# Doc\n");
            assert_refused(&repo, &content, line, TRANSCLUSION_FAILURE, &[], false);
        }
        assert_clean(&repo, &format!("---\ntitle: x\n{key}: \"./side.md\"\n---\n\n# Doc\n"));
        assert_clean(&repo, &format!("---\ntitle: x\n{key}: \"Some *inline* text\"\n---\n\n# Doc\n"));
    }
}

/// R13-F1.3: a `::url` whose target composition's URL parser refuses (a
/// relative path, quoted or not, a fragment, a scheme-relative address) or
/// whose file reference it refuses (`mailto:`) is composition's error at the
/// directive in every validator, and no remote validator sends a request for
/// it. The graphs and flattening list the authored target as the URL the
/// directive names. An absolute URL is the control: passively accepted, and
/// requested once per remote validator.
#[test]
fn a_url_inclusion_of_a_non_url_is_refused_before_any_request() {
    let repo = repository();
    let (authority, requests) = loopback();
    let listing = ["reference graph", "transclusion graph", "flattened"];
    let targets = [
        ("./side.md".to_string(), TRANSCLUSION_FAILURE),
        ("\"./side.md\"".to_string(), TRANSCLUSION_FAILURE),
        ("\"#here\"".to_string(), TRANSCLUSION_FAILURE),
        (format!("//{authority}/x.md"), TRANSCLUSION_FAILURE),
        ("mailto:guide@example.invalid".to_string(), REFERENCE_FAILURE),
    ];
    for (target, code) in &targets {
        let directive = format!("::url {target}");
        for remote in [false, true] {
            assert_refused(&repo, &body(&directive), 7, code, &listing, remote);
        }
        assert_refused(&repo, &after_removed_block(&directive), 10, code, &listing, false);

        let md = document(&repo, &body(&directive));
        let graph = md.reference_graph(ReferenceGraphOptions::with_compose(&request(repo.path()))).unwrap();
        let records = md.composed_references(ReferenceGraphOptions::with_compose(&request(repo.path()))).unwrap().records;
        let record = records.iter().find(|record| record.origin.syntax == ReferenceSyntax::DirectiveUrl).expect("listed");
        assert!(matches!(record.target, ReferenceTarget::RemoteUrl { .. }), "{target}: {:?}", record.target);
        assert_eq!(graph.node_count(), 1, "{target}");
    }
    assert_eq!(requests.load(Ordering::SeqCst), 0, "a refused URL target is never requested");

    let md = document(&repo, &body(&format!("::url http://{authority}/guide.md")));
    for (reader, result) in judge(&repo, &md, true) {
        if ["compose", "preflight"].contains(&reader) {
            continue;
        }
        assert_eq!(result.unwrap_or_else(|e| panic!("{reader}: {e}")), Vec::new(), "{reader}");
    }
    assert_eq!(requests.load(Ordering::SeqCst), 2, "fresh and prebuilt each request the URL once");
}

/// R13-F1.3 control: a bare `@` keeps the located invalid-reference failure
/// it had, and the local query keeps listing an authored URL target.
#[test]
fn a_url_inclusion_of_an_incomplete_reference_keeps_its_failure() {
    let repo = repository();
    let listing = ["reference graph", "transclusion graph", "flattened"];
    assert_refused(&repo, &body("::url @"), 7, REFERENCE_FAILURE, &listing, false);
    let md = document(&repo, &body("::url ./side.md"));
    let local = md.transclusions_with_options(&request(repo.path())).unwrap();
    assert_eq!(local[0].raw_target, "./side.md");
}

/// R13-F1.4: `::code` admits any existing file, so the text file composes
/// and validates; a directory stays a missing file for the validators of
/// both directives, and composition refuses it.
#[test]
fn a_code_inclusion_admits_a_text_file_and_no_inclusion_admits_a_directory() {
    let repo = repository();
    assert_clean(&repo, &body("::code ./side.txt"));
    for directive in ["::code ./dir", "::file ./dir"] {
        let md = document(&repo, &body(directive));
        assert!(md.compose_with(&request(repo.path())).is_err(), "{directive}: composition refuses a directory");
        for (reader, result) in judge(&repo, &md, false) {
            if ["fresh", "prebuilt", "file tree", "followed file tree"].contains(&reader) {
                let issues = result.unwrap_or_else(|e| panic!("{directive}: {reader}: {e}"));
                assert_eq!(issues, vec![ReferenceIssueCode::MissingLocalTarget], "{directive}: {reader}");
            }
        }
    }
}

/// R13-F1.5: ordinary links and images keep their own rules: a link may name
/// a heading of a Markdown file, and an image may be any existing file.
#[test]
fn links_and_images_keep_their_own_target_rules() {
    let repo = repository();
    let md = document(&repo, "# Doc\n\n[Side](./side.md#side) ![Text](./side.txt) [Plain](./side.txt)\n");
    let request = request(repo.path());
    let mut options = validation(ReferenceGraphOptions::with_compose(&request), false);
    options.validate_fragments = true;
    let report = md.validate_references(options).unwrap();
    assert!(report.issues.is_empty(), "{:?}", report.issues);
}

/// `md compose`'s reference check runs ahead of composition and leaves a
/// target the directive refuses to composition, which rejects or tolerates
/// it: neither a text `::file` child nor a relative `::url` fails the check,
/// is opened, or is requested.
#[test]
fn the_compose_reference_check_leaves_a_refused_target_to_composition() {
    let repo = repository();
    let (authority, requests) = loopback();
    let check = request(repo.path()).map_options(ComposeOptions::for_reference_check);
    let scheme_relative = format!("::url //{authority}/x.md");
    for directive in ["::file ./side.txt", "::url ./side.md", scheme_relative.as_str()] {
        let md = document(&repo, &body(directive));
        let report = md
            .validate_references(validation(ReferenceGraphOptions::with_compose(&check), true))
            .unwrap_or_else(|e| panic!("{directive}: {e}"));
        assert!(report.issues.is_empty(), "{directive}: {:?}", report.issues);
        let graph = md.reference_graph(ReferenceGraphOptions::with_compose(&check)).unwrap();
        assert_eq!(graph.node_count(), 1, "{directive}: no child is opened");
    }
    assert_eq!(requests.load(Ordering::SeqCst), 0);
}

/// Sibling: a request that disallows local sources
/// (`with_allow_local_code(false)`, `with_allow_local_markdown(false)`)
/// makes composition refuse an existing `::code` or `::file` target, so the
/// validators refuse it with composition's code at the directive; the
/// default request is the control.
#[test]
fn a_request_that_disallows_local_sources_is_refused_by_its_validators() {
    let repo = repository();
    for (directive, restrict) in [
        ("::code ./side.txt", ComposeOptions::with_allow_local_code as fn(ComposeOptions, bool) -> ComposeOptions),
        ("::file ./side.md", ComposeOptions::with_allow_local_markdown),
    ] {
        let md = document(&repo, &body(directive));
        let restricted = request(repo.path()).map_options(|options| restrict(options, false));
        let composed = md.compose_with(&restricted).expect_err(directive);
        let expected = located(&composed, directive);
        assert_eq!(expected, (TRANSCLUSION_FAILURE.to_string(), "doc.md".to_string(), Some(7)), "{directive}");
        let graph = ReferenceGraphOptions::with_compose(&restricted);
        let fresh = md.validate_references(validation(graph, false)).expect_err(directive);
        assert_eq!(located(&fresh, directive), expected, "{directive}: fresh");
        let mut tree = FileTree::from_markdown(md.clone(), &restricted).validate();
        let error = tree.ensure_built().expect_err(directive);
        assert_eq!(located(&error, directive), expected, "{directive}: file tree");
    }
}
