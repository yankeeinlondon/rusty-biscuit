//! Reference validation judges an inclusion's target as composition opens
//! it, never as a hyperlink. `::file`, `::code`, `prologue`, and `epilogue`
//! open one whole filename (`./side.md#side` names the file `side.md#side`),
//! and a `::toc-linking` chain keeps its selector's outcome: the selected
//! file, a required failure, or a suppressed chain that nothing validates and
//! no remote check reaches. Ordinary links keep their hyperlink rules.

use biscuit_file::FileResolutionContext;
use darkmatter::markdown::compose::preflight::PreflightResolvedTarget;
use darkmatter::markdown::compose::{ComposeContext, ComposeOptions, ComposeRequest};
use darkmatter::markdown::reference::file_tree::FileTree;
use darkmatter::markdown::reference::types::{
    ReferenceGraphOptions, ReferenceKind, ReferenceRecord, ReferenceSyntax, ReferenceTarget, TransclusionRefKind,
};
use darkmatter::markdown::reference::validate::{
    ReferenceIssue, ReferenceIssueCode, ReferenceValidationOptions, ReferenceValidationReport,
};
use darkmatter::markdown::Markdown;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use tempfile::TempDir;

/// A temporary repository holding `side.md` with the heading `# Side`.
fn repository() -> TempDir {
    let repo = TempDir::new().unwrap();
    std::fs::create_dir_all(repo.path().join(".git")).unwrap();
    std::fs::write(repo.path().join("side.md"), "# Side\n\n## Part\n").unwrap();
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

/// A loopback HTTP server answering every request with `200 OK`, and the
/// number of requests it has answered.
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
    (format!("http://127.0.0.1:{port}/guide.md"), requests)
}

/// The validation modes a check must agree in.
#[derive(Clone, Copy, Debug)]
enum Mode {
    LocalPaths,
    Fragments,
    Remote,
}

const MODES: [Mode; 3] = [Mode::LocalPaths, Mode::Fragments, Mode::Remote];

fn options(graph: ReferenceGraphOptions, mode: Mode) -> ReferenceValidationOptions {
    let mut options = ReferenceValidationOptions::with_graph(graph);
    options.validate_fragments = matches!(mode, Mode::Fragments);
    options.validate_remote = matches!(mode, Mode::Remote);
    options.remote_timeout = Duration::from_secs(2);
    options
}

type Verdict = Vec<(ReferenceIssueCode, ReferenceKind)>;

/// What every reader reported for one document in one mode.
struct Readings {
    /// Flattened records of the full graph.
    records: Vec<ReferenceRecord>,
    /// Node counts of the full and transclusion-only graphs.
    nodes: (usize, usize),
    /// `(reader, issue codes and kinds)` for fresh, prebuilt, and `md
    /// compose`'s reference check (and FileTree validation in local-path
    /// mode, its only mode).
    verdicts: Vec<(&'static str, Verdict)>,
    /// The warnings of the fresh report.
    warnings: Vec<String>,
}

fn codes(issues: &[ReferenceIssue]) -> Verdict {
    issues.iter().map(|issue| (issue.code, issue.kind)).collect()
}

/// Runs every graph and validation reader over `md` in `mode`; each must
/// build.
fn read(repo: &TempDir, md: &Markdown, shape: &str, mode: Mode) -> Readings {
    let request = request(repo.path());
    let graph_options = ReferenceGraphOptions::with_compose(&request);
    let graph = md.reference_graph(graph_options.clone()).unwrap_or_else(|e| panic!("{shape}: graph: {e}"));
    let transclusion =
        md.transclusion_graph(graph_options.clone()).unwrap_or_else(|e| panic!("{shape}: transclusion graph: {e}"));
    let records = md
        .composed_references(graph_options.clone())
        .unwrap_or_else(|e| panic!("{shape}: flattened: {e}"))
        .records;
    let fresh: ReferenceValidationReport = md
        .validate_references(options(graph_options.clone(), mode))
        .unwrap_or_else(|e| panic!("{shape}: {mode:?}: fresh: {e}"));
    let prebuilt = md
        .validate_references_with_graph(&graph, options(graph_options, mode))
        .unwrap_or_else(|e| panic!("{shape}: {mode:?}: prebuilt: {e}"));
    let check_request = request.clone().map_options(ComposeOptions::for_reference_check);
    let check = md
        .validate_references(options(ReferenceGraphOptions::with_compose(&check_request), mode))
        .unwrap_or_else(|e| panic!("{shape}: {mode:?}: reference check: {e}"));
    let mut verdicts =
        vec![("fresh", codes(&fresh.issues)), ("prebuilt", codes(&prebuilt.issues)), ("reference check", codes(&check.issues))];
    if matches!(mode, Mode::LocalPaths) {
        let mut tree = FileTree::from_markdown(md.clone(), &request).follow_transclusions().validate();
        tree.ensure_built().unwrap_or_else(|e| panic!("{shape}: file tree: {e}"));
        verdicts.push(("file tree", codes(&tree.validation_report().expect("a validating tree has a report").issues)));
    }
    Readings {
        records,
        nodes: (graph.node_count(), transclusion.node_count()),
        verdicts,
        warnings: fresh.warnings,
    }
}

/// The single inclusion record of `syntax` in `records`.
fn inclusion(records: &[ReferenceRecord], syntax: ReferenceSyntax) -> &ReferenceRecord {
    let found: Vec<_> = records
        .iter()
        .filter(|record| record.origin.syntax == syntax && record.kind == ReferenceKind::Transclusion)
        .collect();
    assert_eq!(found.len(), 1, "{found:?}");
    found[0]
}

fn missing_inclusion() -> Verdict {
    vec![(ReferenceIssueCode::MissingLocalTarget, ReferenceKind::Transclusion)]
}

/// R12-F1.1, .2: a `#` suffix on a file inclusion is part of its filename.
/// Composition opens `side.md#side`, which does not exist, so every reader
/// reports a missing inclusion even though `side.md` and its `# Side`
/// heading exist; the authored spelling is what the record and issue show.
/// The plain path is the passing control.
#[test]
fn a_hash_suffix_on_a_file_inclusion_is_part_of_its_filename() {
    let repo = repository();
    let request = request(repo.path());
    let cases = [
        ("::file ./side.md#side", ReferenceSyntax::DirectiveFile),
        ("::file \"./side.md#side\"", ReferenceSyntax::DirectiveFile),
        ("::code ./side.md#side", ReferenceSyntax::DirectiveCode),
        ("::code \"./side.md#side\"", ReferenceSyntax::DirectiveCode),
        ("---\nprologue: \"./side.md#side\"\n---\n", ReferenceSyntax::FrontmatterPrologue),
        ("---\nepilogue: \"./side.md#side\"\n---\n", ReferenceSyntax::FrontmatterEpilogue),
    ];
    for (inclusion_text, syntax) in cases {
        let content = if inclusion_text.starts_with("---") {
            format!("{inclusion_text}# Doc\n")
        } else {
            format!("# Doc\n\n{inclusion_text}\n")
        };
        let md = document(&repo, &content);
        assert!(md.compose_with(&request).is_err(), "{inclusion_text}: composition opens `side.md#side`");

        let local = md.transclusions_with_options(&request).unwrap();
        assert_eq!(local.len(), 1, "{inclusion_text}: {local:?}");
        assert_eq!(local[0].raw_target, "./side.md#side", "{inclusion_text}");
        assert_eq!(local[0].resolved_target, None, "{inclusion_text}: no such file");

        for mode in MODES {
            let readings = read(&repo, &md, inclusion_text, mode);
            assert_eq!(readings.nodes, (1, 1), "{inclusion_text}: no child document");
            let record = inclusion(&readings.records, syntax);
            assert_eq!(
                record.target,
                ReferenceTarget::LocalPath { raw: "./side.md#side".into() },
                "{inclusion_text}"
            );
            for (reader, issues) in &readings.verdicts {
                assert_eq!(issues, &missing_inclusion(), "{inclusion_text}: {mode:?}: {reader}");
            }
        }

        let control = document(&repo, &content.replace("./side.md#side", "./side.md"));
        control.compose_with(&request).unwrap_or_else(|e| panic!("{inclusion_text}: control compose: {e}"));
        for mode in MODES {
            for (reader, issues) in read(&repo, &control, inclusion_text, mode).verdicts {
                assert!(issues.is_empty(), "{inclusion_text}: control: {mode:?}: {reader}: {issues:?}");
            }
        }
    }
}

/// R12-F1.1 sibling: a body inclusion whose target only looks like a
/// fragment is still one filename to composition, so validation reports it
/// missing instead of checking it against the root's `# Side` heading.
#[test]
fn a_fragment_shaped_file_inclusion_names_a_local_file() {
    let repo = repository();
    for directive in ["::file \"#side\"", "::code \"#side\""] {
        let md = document(&repo, &format!("# Side\n\n{directive}\n"));
        for mode in MODES {
            for (reader, issues) in read(&repo, &md, directive, mode).verdicts {
                assert_eq!(issues, missing_inclusion(), "{directive}: {mode:?}: {reader}");
            }
        }
    }
}

/// R12-F1.3: a required `::toc-linking` target the selector rejects is a
/// required failure in every mode. The heading `# Here` in the root, the
/// heading `# Side` in `side.md`, and the loopback server's `200 OK` have no
/// bearing on it, and no request is sent. Plain `./side.md` succeeds and a
/// missing `./gone.md` is reported.
#[test]
fn a_required_toc_target_the_selector_rejects_is_a_required_failure() {
    let repo = repository();
    let request = request(repo.path());
    let (url, requests) = loopback();
    let rejected = [
        "./side.md#side".to_string(),
        "\"./side.md#side\"".to_string(),
        "\"#here\"".to_string(),
        format!("\"{url}\""),
        "\"mailto:docs@example.invalid\"".to_string(),
        "\"data:text/plain,guide\"".to_string(),
        "./gone.md".to_string(),
    ];
    for target in &rejected {
        let md = document(&repo, &format!("# Here\n\n::toc-linking {target}\n"));
        let refused = match md.compose_with(&request) {
            Err(_) => true,
            Ok((_, report)) => !report.warnings.is_empty(),
        };
        assert!(refused, "{target}: composition never passes it silently");
        for mode in MODES {
            let readings = read(&repo, &md, target, mode);
            assert_eq!(readings.nodes, (1, 1), "{target}");
            let record = inclusion(&readings.records, ReferenceSyntax::DirectiveTocLinking);
            assert!(matches!(record.target, ReferenceTarget::LocalPath { .. }), "{target}: {:?}", record.target);
            for (reader, issues) in &readings.verdicts {
                assert_eq!(issues, &missing_inclusion(), "{target}: {mode:?}: {reader}");
            }
            assert!(readings.warnings.is_empty(), "{target}: {mode:?}: {:?}", readings.warnings);
        }
    }
    assert_eq!(requests.load(Ordering::SeqCst), 0, "no request decides a TOC target");

    let control = document(&repo, "# Here\n\n::toc-linking ./side.md\n");
    for mode in MODES {
        for (reader, issues) in read(&repo, &control, "control", mode).verdicts {
            assert!(issues.is_empty(), "control: {mode:?}: {reader}: {issues:?}");
        }
    }
}

/// R12-F1.4: a chain ending in `false` (in any letter case) that finds
/// nothing is suppressed before validation reads its target kind: no issue
/// and no remote warning in any mode, and the suppressed URL never reaches
/// the remote batch. `false` alone lists no target at all.
#[test]
fn a_suppressed_toc_chain_is_never_validated_or_requested() {
    let repo = repository();
    let request = request(repo.path());
    let (url, requests) = loopback();
    for chain in [
        "\"./gone.md | false\"".to_string(),
        "\"#absent | false\"".to_string(),
        format!("\"{url} | false\""),
        format!("\"{url} | FALSE\""),
        "\"./gone.md | FALSE\"".to_string(),
        "false".to_string(),
    ] {
        let md = document(&repo, &format!("# Here\n\n::toc-linking {chain}\n"));
        let (_, report) = md.compose_with(&request).unwrap_or_else(|e| panic!("{chain}: compose: {e}"));
        assert!(report.warnings.is_empty(), "{chain}: {:?}", report.warnings);
        for mode in MODES {
            let readings = read(&repo, &md, &chain, mode);
            assert_eq!(readings.nodes, (1, 1), "{chain}");
            for (reader, issues) in &readings.verdicts {
                assert!(issues.is_empty(), "{chain}: {mode:?}: {reader}: {issues:?}");
            }
            assert!(readings.warnings.is_empty(), "{chain}: {mode:?}: {:?}", readings.warnings);
        }
    }
    assert_eq!(requests.load(Ordering::SeqCst), 0, "a suppressed chain sends no request");

    // Without `false` the same chains are required failures again.
    for chain in [format!("\"{url}\""), "\"#absent\"".to_string(), "\"./gone.md\"".to_string()] {
        let md = document(&repo, &format!("# Here\n\n::toc-linking {chain}\n"));
        for mode in MODES {
            for (reader, issues) in read(&repo, &md, &chain, mode).verdicts {
                assert_eq!(issues, missing_inclusion(), "{chain}: {mode:?}: {reader}");
            }
        }
    }
    assert_eq!(requests.load(Ordering::SeqCst), 0, "a required failure sends no request either");
}

/// R12-F1.5: a chain whose URL alternative fails over to an existing local
/// file selects the file in both graphs and every validator, sending no
/// request; the local query lists the authored alternatives unfollowed.
#[test]
fn a_toc_chain_falls_back_from_a_url_to_a_local_file() {
    let repo = repository();
    let request = request(repo.path());
    let (url, requests) = loopback();
    let md = document(&repo, &format!("# Here\n\n::toc-linking \"{url} | ./side.md | false\"\n"));
    md.compose_with(&request).unwrap_or_else(|e| panic!("compose: {e}"));
    for mode in MODES {
        let readings = read(&repo, &md, "fallback", mode);
        assert_eq!(readings.nodes, (2, 2), "{mode:?}: the local file is followed");
        let record = inclusion(&readings.records, ReferenceSyntax::DirectiveTocLinking);
        assert_eq!(record.target, ReferenceTarget::LocalPath { raw: "./side.md".into() });
        for (reader, issues) in &readings.verdicts {
            assert!(issues.is_empty(), "{mode:?}: {reader}: {issues:?}");
        }
    }
    assert_eq!(requests.load(Ordering::SeqCst), 0);

    let local = md.transclusions_with_options(&request).unwrap();
    assert_eq!(local.len(), 1);
    assert_eq!(local[0].kind, TransclusionRefKind::TocLinking);
    assert_eq!(local[0].raw_target, format!("{url} | ./side.md"));
    assert_eq!(local[0].resolved_target, None);

    let preflight = md.compose_preflight(&request).unwrap_or_else(|e| panic!("preflight: {e}"));
    let selected: Vec<_> = preflight
        .preflight_graph
        .targets
        .iter()
        .map(|target| match &target.resolved_target {
            PreflightResolvedTarget::File { path, .. } => path.file_name().map(|name| name.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect();
    assert_eq!(selected, vec![Some("side.md".to_string())], "pre-flight selects the local file");
    assert_eq!(requests.load(Ordering::SeqCst), 0);
}

/// Sibling: a `::file-links` listing is judged by composition's discovery,
/// so even a URL-shaped glob is never sent to the remote batch or reported
/// as an unverified URL.
#[test]
fn a_file_list_glob_is_never_a_remote_reference() {
    let repo = repository();
    let (url, requests) = loopback();
    let md = document(&repo, &format!("# Here\n\n::file-links \"{}*.md\"\n", url.trim_end_matches("guide.md")));
    for mode in MODES {
        let readings = read(&repo, &md, "file-links", mode);
        for (reader, issues) in &readings.verdicts {
            assert!(issues.is_empty(), "{mode:?}: {reader}: {issues:?}");
        }
        assert!(readings.warnings.is_empty(), "{mode:?}: {:?}", readings.warnings);
    }
    assert_eq!(requests.load(Ordering::SeqCst), 0);
}

/// R12-F1.6: ordinary links keep hyperlink rules (a fragment resolves
/// against headings, a path's fragment against its file's headings, a URL
/// is requested under remote validation), `::url` stays a remote reference,
/// and the links `::toc-linking` generates validate against the target's
/// raw headings.
#[test]
fn ordinary_links_and_remote_inclusions_keep_their_rules() {
    let repo = repository();
    let (url, requests) = loopback();
    let md = document(
        &repo,
        &format!("# Here\n\n[a](#here) [b](./side.md#side) [c]({url})\n\n::url {url}\n\n::toc-linking ./side.md\n"),
    );
    let url_record = |readings: &Readings| {
        inclusion(&readings.records, ReferenceSyntax::DirectiveUrl).target.clone()
    };
    for mode in MODES {
        let readings = read(&repo, &md, "links", mode);
        assert_eq!(url_record(&readings), ReferenceTarget::RemoteUrl { raw: url.clone() });
        for (reader, issues) in &readings.verdicts {
            assert!(issues.is_empty(), "{mode:?}: {reader}: {issues:?}");
        }
    }
    // The link and `::url`, once per remote-mode validator (fresh, prebuilt,
    // reference check).
    assert_eq!(requests.load(Ordering::SeqCst), 6);

    let broken = document(&repo, "# Here\n\n[a](#nowhere) [b](./side.md#nowhere)\n");
    let readings = read(&repo, &broken, "broken links", Mode::Fragments);
    for (reader, issues) in readings.verdicts {
        assert_eq!(
            issues,
            vec![
                (ReferenceIssueCode::MissingFragmentTarget, ReferenceKind::Hyperlink),
                (ReferenceIssueCode::MissingFragmentTarget, ReferenceKind::Hyperlink),
            ],
            "{reader}"
        );
    }

    // A quoted remote `::file` keeps its existing remote reference.
    let remote_file = document(&repo, &format!("# Here\n\n::file \"{url}\"\n"));
    let readings = read(&repo, &remote_file, "remote ::file", Mode::LocalPaths);
    assert_eq!(
        inclusion(&readings.records, ReferenceSyntax::DirectiveFile).target,
        ReferenceTarget::RemoteUrl { raw: url.clone() }
    );
}

/// A link or image target that is no file reference (`@`, also with a
/// fragment, and in a `prologue`'s inline text) is a located
/// `InvalidReference` error issue in every validator, never a hard error, and
/// projects to `dm.reference.failure` at its authored line and column with
/// its `invalid-reference` class. An existing file is valid and a missing one
/// stays `MissingLocalTarget`.
#[test]
fn an_invalid_link_target_is_a_located_issue_in_every_validator() {
    use darkmatter::markdown::compose::report_diagnostics::reference_issue_diagnostic;
    let repo = repository();
    let md = document(
        &repo,
        "---\nprologue: \"See [p](@) inline\"\n---\n# Doc\n\n[x](@) [f](@#part)\n\n![y](@)\n\n[ok](./side.md) [gone](./gone.md)\n",
    );
    let invalid = |kind| (ReferenceIssueCode::InvalidReference, kind);
    let expected = vec![
        invalid(ReferenceKind::Hyperlink),
        invalid(ReferenceKind::Hyperlink),
        invalid(ReferenceKind::Hyperlink),
        invalid(ReferenceKind::Image),
        (ReferenceIssueCode::MissingLocalTarget, ReferenceKind::Hyperlink),
    ];
    for mode in MODES {
        for (reader, issues) in read(&repo, &md, "invalid links", mode).verdicts {
            assert_eq!(issues, expected, "{mode:?}: {reader}");
        }
    }

    let request = request(repo.path());
    let report = md
        .validate_references(options(ReferenceGraphOptions::with_compose(&request), Mode::LocalPaths))
        .expect("an invalid link target is an issue, not an error");
    assert!(!report.is_valid());
    let located: Vec<_> = report
        .issues
        .iter()
        .map(|issue| {
            let record = reference_issue_diagnostic(issue);
            let source = record.source.expect("an issue names its document");
            let details = record.reference_details.expect("a reference record");
            (record.code, source.line, source.column, details.failure)
        })
        .collect();
    let failure = |line, column| {
        ("dm.reference.failure".to_string(), Some(line), Some(column), Some("invalid-reference".to_string()))
    };
    assert_eq!(
        located,
        vec![
            failure(2, 11),
            failure(6, 1),
            failure(6, 8),
            failure(8, 1),
            ("dm.reference.missing_hyperlink".to_string(), Some(10), Some(17), Some("no-match".to_string())),
        ]
    );
}
