//! Integration tests for the reference analysis subsystem.
//!
//! Tests composed graph behavior (rec #13) and validation (rec #14)
//! with real filesystem documents using `tempfile`.

use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::ComposeSource;
use darkmatter::markdown::reference::types::{
    ReferenceGraphOptions, ReferenceKind, ReferenceSyntax,
};
use darkmatter::markdown::reference::validate::{
    ReferenceIssueCode, ReferenceValidationOptions,
};
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

// ═══════════════════════════════════════════════════════════════════
//  Rec #13 — Composed graph integration tests
// ═══════════════════════════════════════════════════════════════════

#[test]
fn recursive_file_traversal() {
    let dir = TempDir::new().unwrap();
    write_files(
        &dir,
        &[
            ("root.md", "# Root\n\n[root-link](https://root.example.com)\n\n::file child.md\n"),
            ("child.md", "# Child\n\n[child-link](https://child.example.com)\n"),
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
        &[
            ("a.md", "::file b.md\n"),
            ("b.md", "::file a.md\n"),
        ],
    );

    let md = load_md(&dir, "a.md");
    let options = ReferenceGraphOptions::default();
    // Should not hang
    let graph = md.reference_graph(options).unwrap();
    assert!(graph.node_count() <= 3);
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
    let mut options = ReferenceGraphOptions::default();
    // Set a depth limit of 2
    options.compose.max_transclusion_depth = 2;
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
fn mermaid_output_includes_child_nodes() {
    let dir = TempDir::new().unwrap();
    write_files(
        &dir,
        &[
            ("root.md", "::file child.md\n"),
            ("child.md", "# Child\n"),
        ],
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
        &[
            ("root.md", "::file child.md\n"),
            ("child.md", "# Child\n"),
        ],
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
fn validate_fail_fast_stops_early() {
    let dir = TempDir::new().unwrap();
    write_files(
        &dir,
        &[("root.md", "[a](./missing1.md)\n[b](./missing2.md)\n[c](./missing3.md)\n")],
    );

    let source_path = dir.path().join("root.md");
    let md = Markdown::new("[a](./missing1.md)\n[b](./missing2.md)\n[c](./missing3.md)")
        .with_source(ComposeSource::File(source_path));

    let options = ReferenceValidationOptions {
        fail_fast: true,
        ..Default::default()
    };
    let report = md.validate_references(options).unwrap();
    assert_eq!(report.error_count(), 1, "fail_fast should stop after first error");
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
//  Graph-aware Phase 2 API tests (rec #9)
// ═══════════════════════════════════════════════════════════════════

#[test]
fn inline_css_graph_collects_across_nodes() {
    let dir = TempDir::new().unwrap();
    write_files(
        &dir,
        &[
            ("root.md", "<style>\nbody { color: red; }\n</style>\n\n::file child.md\n"),
            ("child.md", "# Child\n\n<style>\n.child { color: blue; }\n</style>\n"),
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
            ("root.md", "<script src=\"root.js\"></script>\n\n::file child.md\n"),
            ("child.md", "# Child\n\n<script src=\"child.js\"></script>\n"),
        ],
    );

    let md = load_md(&dir, "root.md");
    let options = ReferenceGraphOptions::default();
    let imports = md.script_import_graph(options).unwrap();
    assert!(imports.len() >= 2, "Expected 2 script imports, found {}", imports.len());
}

// ═══════════════════════════════════════════════════════════════════
//  Transclusion reference completeness
// ═══════════════════════════════════════════════════════════════════

#[test]
fn transclusion_ref_resolved_target_filled() {
    let dir = TempDir::new().unwrap();
    write_files(
        &dir,
        &[
            ("root.md", "::file child.md\n"),
            ("child.md", "# Child\n"),
        ],
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
            ("root.md", "::file child.md\n::code example.rs\n::url https://example.com\n"),
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
