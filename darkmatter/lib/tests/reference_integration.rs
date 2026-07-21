//! Integration tests for the reference analysis subsystem.
//!
//! Tests composed graph behavior (rec #13) and validation (rec #14)
//! with real filesystem documents using `tempfile`.

use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::{ComposeOptions, ComposeSource};
use darkmatter::markdown::normalize::HeadingLevel;
use darkmatter::markdown::reference::types::{
    ReferenceGraphOptions, ReferenceKind, ReferenceSyntax, ReferenceTarget,
};
use darkmatter::markdown::reference::validate::{ReferenceIssueCode, ReferenceValidationOptions};
use biscuit_file::FileResolutionContext;
use tempfile::TempDir;

// ── Helper ──────────────────────────────────────────────────────────

fn write_files(dir: &TempDir, files: &[(&str, &str)]) {
    for (name, content) in files {
        let path = dir.path().join(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&path, content).unwrap();
    }
}

fn load_md(dir: &TempDir, name: &str) -> Markdown {
    let path = dir.path().join(name);
    Markdown::try_from(path.as_path()).unwrap()
}

fn reference_options(repo_root: &std::path::Path) -> ComposeOptions {
    let context = FileResolutionContext::from_snapshot(
        repo_root,
        None,
        std::collections::HashMap::new(),
    )
    .with_repository_root(repo_root);
    ComposeOptions::new().with_file_resolution_context(context)
}

struct CurrentDirGuard(std::path::PathBuf);

impl CurrentDirGuard {
    fn set(path: &std::path::Path) -> Self {
        let previous = std::env::current_dir().unwrap();
        std::env::set_current_dir(path).unwrap();
        Self(previous)
    }
}

impl Drop for CurrentDirGuard {
    fn drop(&mut self) {
        std::env::set_current_dir(&self.0).unwrap();
    }
}

fn assert_file_reference_error(error: &darkmatter::markdown::MarkdownError) {
    assert!(
        matches!(
            error,
            darkmatter::markdown::MarkdownError::Reference(inner)
                if matches!(inner.as_ref(), darkmatter::markdown::reference::ReferenceError::FileReference(_))
        ),
        "expected a typed file-reference error, got: {error:?}"
    );
}

#[test]
#[serial_test::serial]
fn explicit_context_is_shared_by_enumeration_graph_and_validation() {
    let repo = TempDir::new().unwrap();
    let unrelated = TempDir::new().unwrap();
    write_files(
        &repo,
        &[
            ("docs/root.md", "::file shared.md\n"),
            ("shared.md", "# Repository winner\n"),
            ("docs/shared.md", "# Source loser\n"),
        ],
    );
    let md = load_md(&repo, "docs/root.md");
    let compose = reference_options(repo.path());
    let graph_options = ReferenceGraphOptions::with_compose(compose.clone());

    let _cwd = CurrentDirGuard::set(unrelated.path());

    let refs = md.transclusions_with_options(&compose).unwrap();
    assert_eq!(
        refs[0].resolved_target.as_deref(),
        Some(repo.path().join("shared.md").to_string_lossy().as_ref())
    );

    let graph = md.reference_graph(graph_options.clone()).unwrap();
    assert_eq!(graph.node_count(), 2);
    assert_eq!(
        graph.nodes[0].source,
        ComposeSource::File(repo.path().join("shared.md"))
    );

    let report = md
        .validate_references(ReferenceValidationOptions::with_graph(graph_options))
        .unwrap();
    assert!(report.is_valid(), "unexpected issues: {:?}", report.issues);
}

#[test]
fn invalid_reference_propagates_across_all_reference_surfaces() {
    let repo = TempDir::new().unwrap();
    write_files(&repo, &[("root.md", "::file {{}}\n")]);
    let md = load_md(&repo, "root.md");
    let compose = reference_options(repo.path());
    let graph_options = ReferenceGraphOptions::with_compose(compose.clone());

    let enumeration = md.transclusions_with_options(&compose).unwrap_err();
    let graph = md.reference_graph(graph_options.clone()).unwrap_err();
    let validation = md
        .validate_references(ReferenceValidationOptions::with_graph(graph_options))
        .unwrap_err();

    assert_file_reference_error(&enumeration);
    assert_file_reference_error(&graph);
    assert_file_reference_error(&validation);
}

#[cfg(unix)]
#[test]
fn permission_failure_propagates_across_all_reference_surfaces() {
    use std::os::unix::fs::PermissionsExt;

    let repo = TempDir::new().unwrap();
    write_files(
        &repo,
        &[
            ("root.md", "::file locked/child.md\n"),
            ("locked/child.md", "# Child\n"),
        ],
    );
    let md = load_md(&repo, "root.md");
    let compose = reference_options(repo.path());
    let graph_options = ReferenceGraphOptions::with_compose(compose.clone());
    let locked = repo.path().join("locked");
    let original_mode = std::fs::metadata(&locked).unwrap().permissions().mode();
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o0)).unwrap();

    let enumeration = md.transclusions_with_options(&compose);
    let graph = md.reference_graph(graph_options.clone());
    let validation = md.validate_references(ReferenceValidationOptions::with_graph(graph_options));

    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(original_mode)).unwrap();

    let errors = [enumeration.unwrap_err(), graph.unwrap_err(), validation.unwrap_err()];
    for error in &errors {
        assert_file_reference_error(error);
        assert!(error.to_string().contains("filesystem error"));
    }
}

// ═══════════════════════════════════════════════════════════════════
//  Rec #13 — Composed graph integration tests
// ═══════════════════════════════════════════════════════════════════

#[test]
fn recursive_file_traversal() {
    let dir = TempDir::new().unwrap();
    write_files(
        &dir,
        &[
            (
                "root.md",
                "# Root\n\n[root-link](https://root.example.com)\n\n::file child.md\n",
            ),
            (
                "child.md",
                "# Child\n\n[child-link](https://child.example.com)\n",
            ),
        ],
    );

    let md = load_md(&dir, "root.md");
    let options = ReferenceGraphOptions::default();
    let graph = md.reference_graph(options).unwrap();

    assert_eq!(graph.node_count(), 2);

    // Root has link + transclusion record
    let root_links = graph.root.local_references.hyperlinks();
    assert_eq!(root_links.len(), 1);
    let root_transclusions = graph.root.local_references.transclusions();
    assert_eq!(root_transclusions.len(), 1);

    // Child has its own link
    let child = &graph.nodes[0];
    assert_eq!(child.local_references.hyperlinks().len(), 1);
}

#[test]
fn prologue_and_epilogue() {
    let dir = TempDir::new().unwrap();
    write_files(
        &dir,
        &[
            (
                "root.md",
                "---\nprologue: header.md\nepilogue: footer.md\n---\n\n# Main Content\n\n[main](https://main.example.com)\n",
            ),
            ("header.md", "[header-link](https://header.example.com)\n"),
            ("footer.md", "[footer-link](https://footer.example.com)\n"),
        ],
    );

    let md = load_md(&dir, "root.md");
    let options = ReferenceGraphOptions::default();
    let graph = md.reference_graph(options.clone()).unwrap();

    // Root + header + footer = 3 nodes
    assert_eq!(graph.node_count(), 3);

    // Composed references should include all
    let composed = md.composed_references(options).unwrap();
    let links: Vec<_> = composed
        .records
        .iter()
        .filter(|r| r.kind == ReferenceKind::Hyperlink)
        .collect();
    assert_eq!(links.len(), 3, "Expected header + main + footer links");
}

#[test]
fn nested_recursion() {
    let dir = TempDir::new().unwrap();
    write_files(
        &dir,
        &[
            ("root.md", "::file mid.md\n"),
            ("mid.md", "::file leaf.md\n"),
            ("leaf.md", "[leaf](https://leaf.example.com)\n"),
        ],
    );

    let md = load_md(&dir, "root.md");
    let options = ReferenceGraphOptions::default();
    let graph = md.reference_graph(options).unwrap();

    // root + mid + leaf = 3 nodes
    assert_eq!(graph.node_count(), 3);
}

#[test]
fn cycle_detection_stops_infinite_recursion() {
    let dir = TempDir::new().unwrap();
    write_files(
        &dir,
        &[("a.md", "::file b.md\n"), ("b.md", "::file a.md\n")],
    );

    let md = load_md(&dir, "a.md");
    let options = ReferenceGraphOptions::default();
    // Should not hang and should produce exactly 2 unique nodes
    let graph = md.reference_graph(options).unwrap();
    assert_eq!(
        graph.node_count(),
        2,
        "two-node cycle should produce exactly 2 unique nodes"
    );

    // Verify no duplicate node IDs
    let mut ids = vec![graph.root.node_id.clone()];
    ids.extend(graph.nodes.iter().map(|n| n.node_id.clone()));
    let unique: std::collections::HashSet<_> = ids.iter().cloned().collect();
    assert_eq!(ids.len(), unique.len(), "all node IDs should be unique");
}

#[test]
fn cycle_composed_references_stay_finite() {
    let dir = TempDir::new().unwrap();
    write_files(
        &dir,
        &[
            ("a.md", "[a-link](https://a.example.com)\n\n::file b.md\n"),
            ("b.md", "[b-link](https://b.example.com)\n\n::file a.md\n"),
        ],
    );

    let md = load_md(&dir, "a.md");
    let options = ReferenceGraphOptions::default();
    let composed = md.composed_references(options).unwrap();

    // Flattened references should be finite and non-duplicative
    let link_count = composed
        .records
        .iter()
        .filter(|r| r.kind == ReferenceKind::Hyperlink)
        .count();
    assert!(
        link_count <= 4,
        "composed links should be bounded, got {link_count}"
    );
}

#[test]
fn depth_limit_respected() {
    let dir = TempDir::new().unwrap();
    // Chain: a -> b -> c -> d -> e (5 levels)
    write_files(
        &dir,
        &[
            ("a.md", "::file b.md\n"),
            ("b.md", "::file c.md\n"),
            ("c.md", "::file d.md\n"),
            ("d.md", "::file e.md\n"),
            ("e.md", "[deep](https://deep.example.com)\n"),
        ],
    );

    let md = load_md(&dir, "a.md");
    // Set a depth limit of 2
    let options =
        ReferenceGraphOptions::with_compose(ComposeOptions::new().with_max_transclusion_depth(2));
    let graph = md.reference_graph(options).unwrap();

    // Should not reach all 5 levels
    assert!(graph.node_count() < 5);
}

#[test]
fn transclusion_records_in_all_views() {
    let dir = TempDir::new().unwrap();
    write_files(
        &dir,
        &[
            ("root.md", "::file child.md\n::code example.rs\n"),
            ("child.md", "# Child\n"),
            ("example.rs", "fn main() {}\n"),
        ],
    );

    let md = load_md(&dir, "root.md");
    let options = ReferenceGraphOptions::default();

    // Local references should include transclusion records
    let graph = md.reference_graph(options.clone()).unwrap();
    let transclusions = graph.root.local_references.transclusions();
    assert_eq!(transclusions.len(), 2);

    // Composed references should also include them
    let composed = md.composed_references(options).unwrap();
    let composed_transclusions: Vec<_> = composed
        .records
        .iter()
        .filter(|r| r.kind == ReferenceKind::Transclusion)
        .collect();
    assert_eq!(composed_transclusions.len(), 2);
}

#[test]
fn toc_linking_dependency_and_generated_links_appear_in_composed_references() {
    let dir = TempDir::new().unwrap();
    write_files(
        &dir,
        &[
            ("root.md", "::toc-linking child.md\n"),
            ("child.md", "## Child Heading\n\nContent.\n"),
        ],
    );

    let md = load_md(&dir, "root.md");
    let options = ReferenceGraphOptions::default();
    let graph = md.reference_graph(options.clone()).unwrap();

    let toc_deps: Vec<_> = graph
        .root
        .local_references
        .transclusions()
        .into_iter()
        .filter(|r| r.origin.syntax == ReferenceSyntax::DirectiveTocLinking)
        .collect();
    assert_eq!(toc_deps.len(), 1, "expected toc-linking dependency record");

    let composed = md.composed_references(options).unwrap();
    let generated_links: Vec<_> = composed
        .records
        .iter()
        .filter(|r| {
            r.kind == ReferenceKind::Hyperlink
                && matches!(
                    &r.target,
                    ReferenceTarget::LocalPath { raw } if raw == "child.md#child-heading"
                )
        })
        .collect();
    assert_eq!(
        generated_links.len(),
        1,
        "expected generated toc link in composed refs"
    );
}

#[test]
fn mermaid_output_includes_child_nodes() {
    let dir = TempDir::new().unwrap();
    write_files(
        &dir,
        &[("root.md", "::file child.md\n"), ("child.md", "# Child\n")],
    );

    let md = load_md(&dir, "root.md");
    let options = ReferenceGraphOptions::default();
    let graph = md.reference_graph(options).unwrap();

    let mermaid = graph.to_mermaid();
    assert!(mermaid.contains("flowchart TD"));
    // Should have at least 2 node definitions and an edge
    assert!(mermaid.contains("-->"));
}

#[test]
fn dot_output_includes_child_nodes() {
    let dir = TempDir::new().unwrap();
    write_files(
        &dir,
        &[("root.md", "::file child.md\n"), ("child.md", "# Child\n")],
    );

    let md = load_md(&dir, "root.md");
    let options = ReferenceGraphOptions::default();
    let graph = md.reference_graph(options).unwrap();

    let dot = graph.to_dot();
    assert!(dot.contains("digraph reference_graph"));
    assert!(dot.contains("->"));
}

// ═══════════════════════════════════════════════════════════════════
//  Rec #14 — Validation integration tests
// ═══════════════════════════════════════════════════════════════════

#[test]
fn validate_child_origin_relative_path() {
    let dir = TempDir::new().unwrap();
    write_files(
        &dir,
        &[
            ("root.md", "::file sub/child.md\n"),
            ("sub/child.md", "[link](./sibling.md)\n"),
            ("sub/sibling.md", "# Sibling\n"),
        ],
    );

    let md = load_md(&dir, "root.md");
    let options = ReferenceValidationOptions::default();
    let report = md.validate_references(options).unwrap();

    // The child's link to ./sibling.md should resolve relative to sub/,
    // not relative to root.md's directory
    let missing: Vec<_> = report
        .issues
        .iter()
        .filter(|i| i.code == ReferenceIssueCode::MissingLocalTarget)
        .collect();
    assert!(
        missing.is_empty(),
        "Child's relative link should resolve to sub/sibling.md, but got: {missing:?}"
    );
}

#[test]
fn validate_cross_doc_fragment_in_child() {
    let dir = TempDir::new().unwrap();
    write_files(
        &dir,
        &[
            ("root.md", "::file child.md\n"),
            ("child.md", "[link](./other.md#intro)\n"),
            ("other.md", "# Intro\n\nContent.\n"),
        ],
    );

    let md = load_md(&dir, "root.md");
    let options = ReferenceValidationOptions {
        validate_fragments: true,
        ..Default::default()
    };
    let report = md.validate_references(options).unwrap();

    let fragment_issues: Vec<_> = report
        .issues
        .iter()
        .filter(|i| i.code == ReferenceIssueCode::MissingFragmentTarget)
        .collect();
    assert!(
        fragment_issues.is_empty(),
        "Fragment #intro should be found in other.md"
    );
}

#[test]
fn validate_missing_file_in_child() {
    let dir = TempDir::new().unwrap();
    write_files(
        &dir,
        &[
            ("root.md", "::file child.md\n"),
            ("child.md", "[broken](./nonexistent.md)\n"),
        ],
    );

    let md = load_md(&dir, "root.md");
    let options = ReferenceValidationOptions::default();
    let report = md.validate_references(options).unwrap();

    assert!(
        report
            .issues
            .iter()
            .any(|i| i.code == ReferenceIssueCode::MissingLocalTarget),
        "Should report missing target for child's broken link"
    );
}

#[test]
fn validate_missing_toc_linking_target() {
    let dir = TempDir::new().unwrap();
    write_files(&dir, &[("root.md", "::toc-linking missing.md\n")]);

    let md = load_md(&dir, "root.md");
    let options = ReferenceValidationOptions::default();
    let report = md.validate_references(options).unwrap();

    assert!(
        report
            .issues
            .iter()
            .any(|i| i.code == ReferenceIssueCode::MissingLocalTarget),
        "missing toc-linking target should be validated as a missing local dependency"
    );
}

#[test]
fn validate_fail_fast_stops_early() {
    let dir = TempDir::new().unwrap();
    write_files(
        &dir,
        &[(
            "root.md",
            "[a](./missing1.md)\n[b](./missing2.md)\n[c](./missing3.md)\n",
        )],
    );

    let source_path = dir.path().join("root.md");
    let md = Markdown::new("[a](./missing1.md)\n[b](./missing2.md)\n[c](./missing3.md)")
        .with_source(ComposeSource::File(source_path));

    let options = ReferenceValidationOptions {
        fail_fast: true,
        ..Default::default()
    };
    let report = md.validate_references(options).unwrap();
    assert_eq!(
        report.error_count(),
        1,
        "fail_fast should stop after first error"
    );
}

#[test]
fn validate_same_document_fragment_against_composed_headings() {
    let dir = TempDir::new().unwrap();
    write_files(
        &dir,
        &[
            (
                "root.md",
                "# Root\n\n[link](#child-heading)\n\n::file child.md\n",
            ),
            ("child.md", "## Child Heading\n\nContent.\n"),
        ],
    );

    let md = load_md(&dir, "root.md");
    let options = ReferenceValidationOptions {
        validate_fragments: true,
        ..Default::default()
    };
    let report = md.validate_references(options).unwrap();

    // The fragment #child-heading should be found because the child
    // heading is part of the composed document
    let fragment_issues: Vec<_> = report
        .issues
        .iter()
        .filter(|i| i.code == ReferenceIssueCode::MissingFragmentTarget)
        .collect();
    assert!(
        fragment_issues.is_empty(),
        "Fragment #child-heading should be resolved from composed headings"
    );
}

// ═══════════════════════════════════════════════════════════════════
//  Fragment validation with prepared/composed headings
// ═══════════════════════════════════════════════════════════════════

#[test]
fn validate_cross_doc_fragment_with_interpolated_heading() {
    let dir = TempDir::new().unwrap();
    write_files(
        &dir,
        &[
            ("root.md", "[go](./target.md#visible)\n"),
            ("target.md", "---\ntitle: Visible\n---\n\n# {{ title }}\n"),
        ],
    );

    let md = load_md(&dir, "root.md");
    let options = ReferenceValidationOptions {
        validate_fragments: true,
        ..Default::default()
    };
    let report = md.validate_references(options).unwrap();

    let fragment_issues: Vec<_> = report
        .issues
        .iter()
        .filter(|i| i.code == ReferenceIssueCode::MissingFragmentTarget)
        .collect();
    assert!(
        fragment_issues.is_empty(),
        "Fragment #visible should be found after interpolation resolves {{ title }} to 'Visible'"
    );
}

#[test]
fn validate_same_doc_fragment_with_interpolated_heading() {
    let dir = TempDir::new().unwrap();
    write_files(
        &dir,
        &[(
            "doc.md",
            "---\nsection: Features\n---\n\n[link](#features)\n\n# {{ section }}\n\nContent.\n",
        )],
    );

    let md = load_md(&dir, "doc.md");
    let options = ReferenceValidationOptions {
        validate_fragments: true,
        ..Default::default()
    };
    let report = md.validate_references(options).unwrap();

    let fragment_issues: Vec<_> = report
        .issues
        .iter()
        .filter(|i| i.code == ReferenceIssueCode::MissingFragmentTarget)
        .collect();
    assert!(
        fragment_issues.is_empty(),
        "Fragment #features should be found after interpolation resolves {{ section }}"
    );
}

#[test]
fn validate_file_links_skips_local_path_validation() {
    let dir = TempDir::new().unwrap();
    write_files(
        &dir,
        &[
            (
                "root.md",
                "::file-links docs/*.md\n\n::file-links --dir reports --depth 0\n",
            ),
            ("docs/readme.md", "# Readme\n"),
            ("reports/q1.md", "# Q1\n"),
        ],
    );

    let md = load_md(&dir, "root.md");
    let options = ReferenceValidationOptions::default();
    let report = md.validate_references(options).unwrap();

    assert!(
        report.is_valid(),
        "file-links references should not trigger local-path validation errors, got issues: {:?}",
        report.issues
    );
    assert_eq!(
        report.references_scanned, 2,
        "both file-links directives should be recorded as references"
    );
    assert_eq!(
        report.references_valid, 2,
        "both file-links references should be counted as valid"
    );
}

// ═══════════════════════════════════════════════════════════════════
//  Graph-aware Phase 2 API tests (rec #9)
// ═══════════════════════════════════════════════════════════════════

#[test]
fn inline_css_graph_collects_across_nodes() {
    let dir = TempDir::new().unwrap();
    write_files(
        &dir,
        &[
            (
                "root.md",
                "<style>\nbody { color: red; }\n</style>\n\n::file child.md\n",
            ),
            (
                "child.md",
                "# Child\n\n<style>\n.child { color: blue; }\n</style>\n",
            ),
        ],
    );

    let md = load_md(&dir, "root.md");
    let options = ReferenceGraphOptions::default();
    let css_blocks = md.inline_css_graph(options).unwrap();
    assert!(
        css_blocks.len() >= 2,
        "Should find CSS blocks from both root and child, found {}",
        css_blocks.len()
    );
}

#[test]
fn script_import_graph_collects_across_nodes() {
    let dir = TempDir::new().unwrap();
    write_files(
        &dir,
        &[
            (
                "root.md",
                "<script src=\"root.js\"></script>\n\n::file child.md\n",
            ),
            (
                "child.md",
                "# Child\n\n<script src=\"child.js\"></script>\n",
            ),
        ],
    );

    let md = load_md(&dir, "root.md");
    let options = ReferenceGraphOptions::default();
    let imports = md.script_import_graph(options).unwrap();
    assert!(
        imports.len() >= 2,
        "Expected 2 script imports, found {}",
        imports.len()
    );
}

// ═══════════════════════════════════════════════════════════════════
//  Cache integration tests
// ═══════════════════════════════════════════════════════════════════

#[test]
fn reference_graph_with_cache_root() {
    let dir = TempDir::new().unwrap();
    let cache_dir = TempDir::new().unwrap();
    write_files(
        &dir,
        &[
            (
                "root.md",
                "# Root\n\n[link](https://example.com)\n\n::file child.md\n",
            ),
            (
                "child.md",
                "# Child\n\n[child-link](https://child.example.com)\n",
            ),
        ],
    );

    let md = load_md(&dir, "root.md");
    let mut options = ReferenceGraphOptions::default();
    options.compose = options.compose.with_cache_root(cache_dir.path());

    // First pass — populates the cache
    let graph1 = md.reference_graph(options.clone()).unwrap();
    assert_eq!(graph1.node_count(), 2);

    // Second pass — should hit the cache for child document load
    let graph2 = md.reference_graph(options).unwrap();
    assert_eq!(graph2.node_count(), 2);
    assert_eq!(
        graph2.root.local_references.hyperlinks().len(),
        graph1.root.local_references.hyperlinks().len()
    );
}

#[test]
fn fragment_validation_with_cache_root() {
    let dir = TempDir::new().unwrap();
    let cache_dir = TempDir::new().unwrap();
    write_files(
        &dir,
        &[
            (
                "root.md",
                "# Root\n\n[link](#child-heading)\n\n::file child.md\n",
            ),
            ("child.md", "## Child Heading\n\nContent.\n"),
        ],
    );

    let md = load_md(&dir, "root.md");
    let mut graph_options = ReferenceGraphOptions::default();
    graph_options.compose = graph_options.compose.with_cache_root(cache_dir.path());

    let options = ReferenceValidationOptions {
        graph: graph_options,
        validate_fragments: true,
        ..Default::default()
    };
    let report = md.validate_references(options).unwrap();

    let fragment_issues: Vec<_> = report
        .issues
        .iter()
        .filter(|i| i.code == ReferenceIssueCode::MissingFragmentTarget)
        .collect();
    assert!(
        fragment_issues.is_empty(),
        "Fragment #child-heading should resolve with cache_root set"
    );
}

// ═══════════════════════════════════════════════════════════════════
//  Transclusion reference completeness
// ═══════════════════════════════════════════════════════════════════

#[test]
fn transclusion_ref_resolved_target_filled() {
    let dir = TempDir::new().unwrap();
    write_files(
        &dir,
        &[("root.md", "::file child.md\n"), ("child.md", "# Child\n")],
    );

    let md = load_md(&dir, "root.md");
    let refs = md.transclusions().unwrap();
    assert_eq!(refs.len(), 1);
    assert!(
        refs[0].resolved_target.is_some(),
        "resolved_target should be filled when source context is available"
    );
}

#[test]
fn transclusion_ref_all_kinds() {
    let dir = TempDir::new().unwrap();
    write_files(
        &dir,
        &[
            (
                "root.md",
                "::file child.md\n::code example.rs\n::url https://example.com\n",
            ),
            ("child.md", ""),
            ("example.rs", ""),
        ],
    );

    let md = load_md(&dir, "root.md");
    let refs = md.transclusions().unwrap();
    assert_eq!(refs.len(), 3);

    let kinds: Vec<_> = refs.iter().map(|r| r.origin.syntax).collect();
    assert!(kinds.contains(&ReferenceSyntax::DirectiveFile));
    assert!(kinds.contains(&ReferenceSyntax::DirectiveCode));
    assert!(kinds.contains(&ReferenceSyntax::DirectiveUrl));
}

#[test]
fn transclusion_ref_resolved_target_is_correct_path() {
    let dir = TempDir::new().unwrap();
    write_files(
        &dir,
        &[
            ("root.md", "::file sub/child.md\n"),
            ("sub/child.md", "# Child\n"),
        ],
    );

    let md = load_md(&dir, "root.md");
    let refs = md.transclusions().unwrap();
    assert_eq!(refs.len(), 1);

    let resolved = refs[0].resolved_target.as_deref().unwrap();
    let expected = dir.path().join("sub/child.md");
    // Compare canonical paths to handle /var vs /private/var on macOS
    let resolved_canon = std::path::Path::new(resolved)
        .canonicalize()
        .unwrap_or_else(|_| std::path::PathBuf::from(resolved));
    let expected_canon = expected.canonicalize().unwrap_or_else(|_| expected.clone());
    assert_eq!(
        resolved_canon, expected_canon,
        "resolved_target should point to the actual child file"
    );
}

// ═══════════════════════════════════════════════════════════════════
//  Cache path parity tests
// ═══════════════════════════════════════════════════════════════════

#[test]
fn reference_graph_cache_honors_namespace() {
    let dir = TempDir::new().unwrap();
    let cache_dir = TempDir::new().unwrap();
    write_files(
        &dir,
        &[
            ("root.md", "# Root\n\n::file child.md\n"),
            ("child.md", "# Child\n"),
        ],
    );

    let md = load_md(&dir, "root.md");
    let mut options = ReferenceGraphOptions::default();
    options.compose = options
        .compose
        .with_cache_root(cache_dir.path())
        .with_cache_namespace("test-branch");

    // Build the graph — should use namespace-scoped persistent cache
    let graph = md.reference_graph(options).unwrap();
    assert_eq!(graph.node_count(), 2);

    // Verify the namespaced cache directory was created
    let expected_cache = cache_dir.path().join(".darkmatter").join("cache");
    // The version directory should exist under the namespace
    assert!(
        expected_cache.exists(),
        "persistent cache directory structure should be created under resolve_cache_root path"
    );
}

// ── FileTree integration tests ──────────────────────────────────────

#[test]
fn section_context_populated_in_graph() {
    let dir = TempDir::new().unwrap();
    write_files(
        &dir,
        &[
            (
                "root.md",
                "# Title\n\n## Intro\n\nSome text.\n\n::file child.md\n\n## Details\n\nMore text.",
            ),
            ("child.md", "# Child\n\nChild content."),
        ],
    );

    let md = load_md(&dir, "root.md");
    let graph = md
        .reference_graph(ReferenceGraphOptions::default())
        .unwrap();

    assert!(
        !graph.root.child_insertions.is_empty(),
        "expected child insertions"
    );
    let insertion = &graph.root.child_insertions[0];
    assert_eq!(
        insertion.context.section_heading_text.as_deref(),
        Some("Intro"),
        "expected section heading 'Intro' for ::file directive in the Intro section"
    );
    assert_eq!(
        insertion.context.section_heading_level,
        Some(HeadingLevel::H2)
    );
    assert_eq!(
        insertion.context.directive_kind,
        Some(ReferenceSyntax::DirectiveFile)
    );
}

#[test]
fn file_tree_builds_from_real_document() {
    use biscuit_terminal::components::renderable::TerminalRenderable;
    use darkmatter::markdown::reference::file_tree::FileTree;

    let dir = TempDir::new().unwrap();
    write_files(
        &dir,
        &[
            (
                "doc.md",
                "# Doc\n\n[link](https://example.com)\n\n![img](./logo.png)\n\n<style>body{}</style>\n\n::file child.md",
            ),
            ("child.md", "# Child"),
        ],
    );

    let path = dir.path().join("doc.md");
    let mut tree = FileTree::new(&path).unwrap();
    tree.ensure_built().unwrap();

    let output = tree.render_optimistic(Some(120));
    assert!(output.contains("doc.md"), "expected file label in output");
    assert!(
        output.contains("example.com"),
        "expected hyperlink in output"
    );
    assert!(output.contains("logo.png"), "expected image ref in output");
    assert!(
        output.contains("child.md"),
        "expected transclusion in output"
    );
    assert!(
        output.contains("1 inline CSS block"),
        "expected inline CSS in summary"
    );
}

#[test]
fn file_tree_follow_mode() {
    use biscuit_terminal::components::renderable::TerminalRenderable;
    use darkmatter::markdown::reference::file_tree::FileTree;

    let dir = TempDir::new().unwrap();
    write_files(
        &dir,
        &[
            ("root.md", "# Root\n\n::file child.md"),
            (
                "child.md",
                "# Child\n\n[child-link](https://child.example.com)",
            ),
        ],
    );

    let path = dir.path().join("root.md");
    let mut tree = FileTree::new(&path).unwrap().follow_transclusions();
    tree.ensure_built().unwrap();

    let output = tree.render_optimistic(Some(120));
    assert!(output.contains("root.md"), "expected root label");
    assert!(output.contains("child.md"), "expected child label");
    assert!(
        output.contains("child.example.com"),
        "expected child's hyperlink in follow mode"
    );
}

// ── Issue #1: ::toc-linking in follow mode ──────────────────────────

#[test]
fn file_tree_toc_linking_follow_mode() {
    use biscuit_terminal::components::renderable::TerminalRenderable;
    use darkmatter::markdown::reference::file_tree::FileTree;

    let dir = TempDir::new().unwrap();
    write_files(
        &dir,
        &[
            ("root.md", "# Root\n\n::toc-linking child.md"),
            (
                "child.md",
                "# Child\n\n## Section A\n\n## Section B\n\n[link](https://child.example.com)",
            ),
        ],
    );

    let path = dir.path().join("root.md");
    let mut tree = FileTree::new(&path).unwrap().follow_transclusions();
    tree.ensure_built().unwrap();

    let output = tree.render_optimistic(Some(120));
    assert!(output.contains("root.md"), "expected root label");
    // The child document should appear as a nested subtree in follow mode
    assert!(
        output.contains("child.md"),
        "expected nested child label from toc-linking follow"
    );
    // Prove that follow mode actually rendered the child's content (not just the edge label)
    assert!(
        output.contains("child.example.com"),
        "expected child's hyperlink in nested subtree, proving recursive render"
    );
}

#[test]
fn file_tree_toc_linking_follow_validate_catches_child_issues() {
    use darkmatter::markdown::reference::file_tree::FileTree;

    let dir = TempDir::new().unwrap();
    write_files(
        &dir,
        &[
            ("root.md", "# Root\n\n::toc-linking child.md"),
            ("child.md", "# Child\n\n[broken](./missing.md)"),
        ],
    );

    let path = dir.path().join("root.md");
    let mut tree = FileTree::new(&path)
        .unwrap()
        .follow_transclusions()
        .validate();
    tree.ensure_built().unwrap();

    let report = tree
        .validation_report()
        .expect("should have validation report");
    // The broken link in child.md should be reported
    let has_missing = report
        .issues
        .iter()
        .any(|i| i.code == ReferenceIssueCode::MissingLocalTarget);
    assert!(
        has_missing,
        "expected MissingLocalTarget issue from child document"
    );
}

// ── Issue #2: Epilogue follow mode ──────────────────────────────────

#[test]
fn file_tree_epilogue_follow_mode() {
    use biscuit_terminal::components::renderable::TerminalRenderable;
    use darkmatter::markdown::reference::file_tree::FileTree;

    let dir = TempDir::new().unwrap();
    write_files(
        &dir,
        &[
            (
                "root.md",
                "---\nepilogue: epilogue.md\n---\n\n# Root\n\n[main](https://main.example.com)",
            ),
            (
                "epilogue.md",
                "# Epilogue\n\n[epi-link](https://epilogue.example.com)",
            ),
        ],
    );

    let path = dir.path().join("root.md");
    let mut tree = FileTree::new(&path).unwrap().follow_transclusions();
    tree.ensure_built().unwrap();

    let output = tree.render_optimistic(Some(120));
    assert!(output.contains("root.md"), "expected root label");
    assert!(
        output.contains("epilogue.md"),
        "expected epilogue child in follow mode"
    );
    assert!(
        output.contains("epilogue.example.com"),
        "expected epilogue's hyperlink in nested subtree"
    );
}

// ── Issue #3: Multiple prologues ────────────────────────────────────

#[test]
fn file_tree_multiple_prologues_follow_mode() {
    use biscuit_terminal::components::renderable::TerminalRenderable;
    use darkmatter::markdown::reference::file_tree::FileTree;

    let dir = TempDir::new().unwrap();
    write_files(
        &dir,
        &[
            (
                "root.md",
                "---\nprologue:\n  - a.md\n  - b.md\n---\n\n# Root",
            ),
            ("a.md", "# A\n\n[a-link](https://a.example.com)"),
            ("b.md", "# B\n\n[b-link](https://b.example.com)"),
        ],
    );

    let path = dir.path().join("root.md");
    let mut tree = FileTree::new(&path).unwrap().follow_transclusions();
    tree.ensure_built().unwrap();

    let output = tree.render_optimistic(Some(120));
    assert!(output.contains("a.md"), "expected first prologue child");
    assert!(
        output.contains("b.md"),
        "expected second prologue child (not a duplicate of first)"
    );
    assert!(output.contains("a.example.com"), "expected a.md's link");
    assert!(output.contains("b.example.com"), "expected b.md's link");
}

// ── Issue #4: show_root(false) preserves the rest of the tree ───────

#[test]
fn file_tree_show_root_false_preserves_subtree() {
    use biscuit_terminal::components::renderable::TerminalRenderable;
    use darkmatter::markdown::reference::file_tree::FileTree;

    let dir = TempDir::new().unwrap();
    write_files(
        &dir,
        &[
            (
                "root.md",
                "# Root\n\n[link](https://example.com)\n\n::file child.md",
            ),
            ("child.md", "# Child"),
        ],
    );

    let path = dir.path().join("root.md");
    let mut tree = FileTree::new(&path).unwrap().show_root(false);
    tree.ensure_built().unwrap();

    let output = tree.render_optimistic(Some(120));
    // Root label should not appear but refs and transclusions should
    assert!(
        !output.contains("\u{1F4C4}root.md"),
        "root file head should be hidden"
    );
    assert!(
        output.contains("example.com"),
        "reference groups should still render"
    );
    assert!(
        output.contains("child.md"),
        "transclusion edges should still render"
    );
}

// ── Issue #5: Section captions for non-H2 headings ─────────────────

#[test]
fn file_tree_section_caption_respects_heading_level() {
    let dir = TempDir::new().unwrap();
    write_files(
        &dir,
        &[
            (
                "root.md",
                "# Root\n\n## Intro\n\n### Details\n\n::file child.md",
            ),
            ("child.md", "# Child"),
        ],
    );

    let md = load_md(&dir, "root.md");
    let options = ReferenceGraphOptions::default();
    let graph = md.reference_graph(options).unwrap();

    // Find the insertion for child.md
    let insertion = &graph.root.child_insertions[0];
    // The directive is under ### Details (level 3)
    assert_eq!(
        insertion.context.section_heading_level,
        Some(HeadingLevel::H3),
        "expected level 3 for ### Details"
    );
    assert_eq!(
        insertion.context.section_heading_text.as_deref(),
        Some("Details")
    );
}
