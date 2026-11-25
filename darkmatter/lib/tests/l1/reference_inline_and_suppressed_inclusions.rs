//! A valid inclusion that composition resolves to something other than a
//! required file stays that outcome in every reference reader: inline
//! `prologue`/`epilogue` text (prose, a Markdown link, a block scalar, an
//! empty scalar) is never resolved as a path, and a `::toc-linking` fallback
//! chain ending in `false` that finds no file is suppressed, not missing.
//! The links inside inline text are still checked as links, and a chain
//! without `false` is still a required missing target.

use biscuit_file::FileResolutionContext;
use darkmatter::markdown::compose::{ComposeContext, ComposeOptions, ComposeRequest};
use darkmatter::markdown::reference::file_tree::FileTree;
use darkmatter::markdown::reference::types::{
    ReferenceGraphOptions, ReferenceKind, ReferenceRecord, ReferenceSyntax, ReferenceTarget, TransclusionRefKind,
};
use darkmatter::markdown::reference::validate::{ReferenceIssueCode, ReferenceValidationOptions};
use darkmatter::markdown::Markdown;
use std::path::Path;
use tempfile::TempDir;

/// A temporary repository holding the included `side.md`.
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

/// What every reader reported for one document.
struct Readings {
    /// Flattened records of the full graph.
    records: Vec<ReferenceRecord>,
    /// Node counts of the full and transclusion-only graphs.
    nodes: (usize, usize),
    /// `(reader, issue codes and kinds)` for fresh, prebuilt, `md compose`'s
    /// reference check, and FileTree validation.
    verdicts: Vec<(&'static str, Vec<(ReferenceIssueCode, ReferenceKind)>)>,
}

/// Runs every graph and validation reader over `md`; each must build.
fn read(repo: &TempDir, md: &Markdown, shape: &str) -> Readings {
    let request = request(repo.path());
    let graph_options = ReferenceGraphOptions::with_compose(&request);
    let graph = md.reference_graph(graph_options.clone()).unwrap_or_else(|e| panic!("{shape}: graph: {e}"));
    let transclusion =
        md.transclusion_graph(graph_options.clone()).unwrap_or_else(|e| panic!("{shape}: transclusion graph: {e}"));
    let records = md
        .composed_references(graph_options.clone())
        .unwrap_or_else(|e| panic!("{shape}: flattened: {e}"))
        .records;
    let codes = |issues: &[darkmatter::markdown::reference::validate::ReferenceIssue]| {
        issues.iter().map(|issue| (issue.code, issue.kind)).collect::<Vec<_>>()
    };
    let fresh = md
        .validate_references(ReferenceValidationOptions::with_graph(graph_options.clone()))
        .unwrap_or_else(|e| panic!("{shape}: fresh: {e}"));
    let prebuilt = md
        .validate_references_with_graph(&graph, ReferenceValidationOptions::with_graph(graph_options))
        .unwrap_or_else(|e| panic!("{shape}: prebuilt: {e}"));
    let check_request = request.clone().map_options(ComposeOptions::for_reference_check);
    let check = md
        .validate_references(ReferenceValidationOptions::with_graph(ReferenceGraphOptions::with_compose(
            &check_request,
        )))
        .unwrap_or_else(|e| panic!("{shape}: reference check: {e}"));
    let mut tree = FileTree::from_markdown(md.clone(), &request).follow_transclusions().validate();
    tree.ensure_built().unwrap_or_else(|e| panic!("{shape}: file tree: {e}"));
    let tree_issues = codes(&tree.validation_report().expect("a validating tree has a report").issues);
    Readings {
        records,
        nodes: (graph.node_count(), transclusion.node_count()),
        verdicts: vec![
            ("fresh", codes(&fresh.issues)),
            ("prebuilt", codes(&prebuilt.issues)),
            ("reference check", codes(&check.issues)),
            ("file tree", tree_issues),
        ],
    }
}

fn frontmatter_records(records: &[ReferenceRecord]) -> Vec<&ReferenceRecord> {
    records
        .iter()
        .filter(|record| {
            matches!(record.origin.syntax, ReferenceSyntax::FrontmatterPrologue | ReferenceSyntax::FrontmatterEpilogue)
        })
        .collect()
}

fn hyperlink_targets(records: &[ReferenceRecord]) -> Vec<String> {
    records
        .iter()
        .filter(|record| record.kind == ReferenceKind::Hyperlink)
        .map(|record| record.target.raw().unwrap_or_default().to_string())
        .collect()
}

/// The inline shapes of one field: `(shape, YAML value, the link target the
/// text holds)`. Each scalar has a one-element list spelling.
fn inline_shapes(target: &str) -> Vec<(&'static str, String, Option<String>)> {
    let link = format!("[Setup]({target})");
    vec![
        ("prose", " Read this first".to_string(), None),
        ("prose list", "\n  - Read this first".to_string(), None),
        ("link", format!(" \"{link}\""), Some(target.to_string())),
        ("link list", format!("\n  - \"{link}\""), Some(target.to_string())),
        ("block scalar", format!(" |\n  Intro\n  {link}"), Some(target.to_string())),
        ("block scalar list", format!("\n  - |\n    Intro\n    {link}"), Some(target.to_string())),
        ("empty scalar", " \"\"".to_string(), None),
        ("empty list item", "\n  - \"\"".to_string(), None),
    ]
}

/// R11-F1.1, .2: inline `prologue`/`epilogue` text is never resolved as a
/// file in either graph mode, the flattened projection, or the local query;
/// it composes, every validator passes it, and a link it holds is checked as
/// a hyperlink to an existing file.
#[test]
fn inline_frontmatter_text_is_not_a_file_reference() {
    let repo = repository();
    let request = request(repo.path());
    for field in ["prologue", "epilogue"] {
        for (shape, value, link) in inline_shapes("./side.md") {
            let shape = format!("{field} {shape}");
            let md = document(&repo, &format!("---\n{field}:{value}\n---\n# Doc\n"));
            md.compose_with(&request).unwrap_or_else(|e| panic!("{shape}: compose: {e}"));

            let readings = read(&repo, &md, &shape);
            assert_eq!(readings.nodes, (1, 1), "{shape}: inline text has no child document");
            for (reader, issues) in &readings.verdicts {
                assert!(issues.is_empty(), "{shape}: {reader}: {issues:?}");
            }
            let inclusion = frontmatter_records(&readings.records);
            assert_eq!(inclusion.len(), 1, "{shape}: {inclusion:?}");
            assert!(matches!(inclusion[0].target, ReferenceTarget::Inline), "{shape}: {:?}", inclusion[0].target);
            assert!(inclusion[0].attributes.contains_key("fm_literal"), "{shape}");
            assert_eq!(hyperlink_targets(&readings.records), link.into_iter().collect::<Vec<_>>(), "{shape}");

            let local = md.transclusions_with_options(&request).unwrap_or_else(|e| panic!("{shape}: local: {e}"));
            assert_eq!(local.len(), 1, "{shape}: {local:?}");
            let kind = if field == "prologue" { TransclusionRefKind::Prologue } else { TransclusionRefKind::Epilogue };
            assert_eq!(local[0].kind, kind, "{shape}");
            assert_eq!(local[0].resolved_target, None, "{shape}: inline text resolves nothing");
        }
    }
}

/// R11-F1.2 control: the same inline shapes linking a missing file give a
/// genuine missing-target issue for the hyperlink in every validator, while
/// composition (which does not check links) still succeeds.
#[test]
fn a_missing_link_inside_inline_frontmatter_text_is_reported_as_a_link() {
    let repo = repository();
    for field in ["prologue", "epilogue"] {
        for (shape, value, link) in inline_shapes("./missing.md") {
            if link.is_none() {
                continue;
            }
            let shape = format!("{field} {shape}");
            let md = document(&repo, &format!("---\n{field}:{value}\n---\n# Doc\n"));
            md.compose_with(&request(repo.path())).unwrap_or_else(|e| panic!("{shape}: compose: {e}"));
            for (reader, issues) in read(&repo, &md, &shape).verdicts {
                assert_eq!(
                    issues,
                    vec![(ReferenceIssueCode::MissingLocalTarget, ReferenceKind::Hyperlink)],
                    "{shape}: {reader}"
                );
            }
        }
    }
}

/// R11-F1.1 control: a real path still resolves as a file in the local query
/// and graph, and a missing path is still a required missing transclusion.
#[test]
fn a_frontmatter_path_is_still_a_file_reference() {
    let repo = repository();
    let request = request(repo.path());
    let md = document(&repo, "---\nprologue: ./side.md\n---\n# Doc\n");
    let local = md.transclusions_with_options(&request).unwrap();
    assert!(local[0].resolved_target.as_deref().is_some_and(|target| target.ends_with("side.md")), "{local:?}");
    let readings = read(&repo, &md, "path");
    assert_eq!(readings.nodes, (2, 2));
    for (reader, issues) in &readings.verdicts {
        assert!(issues.is_empty(), "{reader}: {issues:?}");
    }

    let missing = document(&repo, "---\nprologue: ./missing.md\n---\n# Doc\n");
    for (reader, issues) in read(&repo, &missing, "missing path").verdicts {
        assert_eq!(issues, vec![(ReferenceIssueCode::MissingLocalTarget, ReferenceKind::Transclusion)], "{reader}");
    }
}

/// The `::toc-linking` record of the flattened projection.
fn toc_record(records: &[ReferenceRecord]) -> &ReferenceRecord {
    let toc: Vec<_> = records
        .iter()
        .filter(|record| {
            record.origin.syntax == ReferenceSyntax::DirectiveTocLinking && record.kind == ReferenceKind::Transclusion
        })
        .collect();
    assert_eq!(toc.len(), 1, "{toc:?}");
    toc[0]
}

/// R11-F1.3: the graph uses composition's chain selection. An existing first
/// alternative or fallback is the selected path; an exhausted chain ending
/// in `false` is suppressed (recorded as such, no issue, composition
/// succeeds); without `false` it is a required missing target. The local
/// query lists the authored chain in every case.
#[test]
fn toc_linking_chain_outcomes_match_composition() {
    let repo = repository();
    let request = request(repo.path());
    for (chain, selected, nodes) in [
        ("./side.md | ./other.md", "./side.md", 2),
        ("./missing.md | ./side.md", "./side.md", 2),
    ] {
        let md = document(&repo, &format!("# Doc\n\n::toc-linking \"{chain}\"\n"));
        md.compose_with(&request).unwrap_or_else(|e| panic!("{chain}: compose: {e}"));
        let readings = read(&repo, &md, chain);
        let record = toc_record(&readings.records);
        assert_eq!(record.target.raw(), Some(selected), "{chain}");
        assert!(!record.attributes.contains_key("toc_suppressed"), "{chain}");
        assert_eq!(readings.nodes.0, nodes, "{chain}");
        for (reader, issues) in &readings.verdicts {
            assert!(issues.is_empty(), "{chain}: {reader}: {issues:?}");
        }
    }

    let suppressed = document(&repo, "# Doc\n\n::toc-linking \"./missing.md | ./gone.md | false\"\n");
    let (_, report) = suppressed.compose_with(&request).expect("composition omits a suppressed chain");
    assert!(report.warnings.is_empty(), "suppressed: {:?}", report.warnings);
    let readings = read(&repo, &suppressed, "suppressed");
    let record = toc_record(&readings.records);
    assert_eq!(record.attributes.get("toc_suppressed"), Some(&serde_json::Value::Bool(true)));
    assert_eq!(readings.nodes, (1, 1));
    for (reader, issues) in &readings.verdicts {
        assert!(issues.is_empty(), "suppressed: {reader}: {issues:?}");
    }

    let required = document(&repo, "# Doc\n\n::toc-linking \"./missing.md | ./gone.md\"\n");
    // Composition tolerates the failure as a warning; it is never silent.
    let (_, report) = required.compose_with(&request).expect("a tolerated toc-linking failure");
    assert_eq!(report.warnings.len(), 1, "required: {:?}", report.warnings);
    let readings = read(&repo, &required, "required");
    assert!(!toc_record(&readings.records).attributes.contains_key("toc_suppressed"));
    for (reader, issues) in readings.verdicts {
        assert_eq!(issues, vec![(ReferenceIssueCode::MissingLocalTarget, ReferenceKind::Transclusion)], "{reader}");
    }

    for (md, chain) in [(&suppressed, "./missing.md | ./gone.md"), (&required, "./missing.md | ./gone.md")] {
        let local = md.transclusions_with_options(&request).unwrap();
        assert_eq!(local.len(), 1);
        assert_eq!((local[0].kind, local[0].raw_target.as_str()), (TransclusionRefKind::TocLinking, chain));
        assert_eq!(local[0].resolved_target, None);
    }
}

/// R11-F1.4: the neighboring selectors keep their decisions: a present
/// `::file`/`::code` passes and a missing one is required; a `when="false"`
/// inclusion of a real file is omitted; a quoted URL is classified without a
/// remote read; a `::file-links` listing is no path-existence check.
#[test]
fn neighboring_inclusion_selectors_keep_their_decisions() {
    let repo = repository();
    for (directive, nodes, expected) in [
        ("::file ./side.md", 2, vec![]),
        ("::code ./side.md", 1, vec![]),
        ("::file ./missing.md", 1, vec![(ReferenceIssueCode::MissingLocalTarget, ReferenceKind::Transclusion)]),
        ("::code ./missing.md", 1, vec![(ReferenceIssueCode::MissingLocalTarget, ReferenceKind::Transclusion)]),
        ("::file ./side.md when=\"false\"", 1, vec![]),
        ("::file \"https://example.invalid/remote.md\"", 1, vec![]),
        ("::file-links \"./*.md\"", 1, vec![]),
        ("::file-links --dir . --depth 1", 1, vec![]),
    ] {
        let md = document(&repo, &format!("# Doc\n\n{directive}\n"));
        let readings = read(&repo, &md, directive);
        assert_eq!(readings.nodes.0, nodes, "{directive}");
        for (reader, issues) in readings.verdicts {
            assert_eq!(issues, expected, "{directive}: {reader}");
        }
    }
}

/// Inline text is inserted as body Markdown, so an image it holds is read as
/// an image reference, like one in the body: an existing file passes and a
/// missing one is a missing image in every validator.
#[test]
fn an_image_inside_inline_frontmatter_text_is_checked_as_an_image() {
    let repo = repository();
    std::fs::write(repo.path().join("logo.png"), "x").unwrap();
    for field in ["prologue", "epilogue"] {
        let present = document(&repo, &format!("---\n{field}: \"![Logo](./logo.png)\"\n---\n# Doc\n"));
        for (reader, issues) in read(&repo, &present, field).verdicts {
            assert!(issues.is_empty(), "{field}: {reader}: {issues:?}");
        }
        let missing = document(&repo, &format!("---\n{field}: \"![Logo](./gone.png)\"\n---\n# Doc\n"));
        for (reader, issues) in read(&repo, &missing, field).verdicts {
            assert_eq!(issues, vec![(ReferenceIssueCode::MissingLocalTarget, ReferenceKind::Image)], "{field}: {reader}");
        }
    }
}
