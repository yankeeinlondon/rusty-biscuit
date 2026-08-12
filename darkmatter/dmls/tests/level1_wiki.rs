//! Level-1 wiki-link resolution over the R-8 fixture workspace.
//!
//! The fixture is built in memory with fixed absolute paths and two wiki roots,
//! so every resolution is computed the same way on macOS, Windows, and Linux
//! (no real filesystem, no host case/NFC behavior). Each assertion pins one
//! rule from the R-8 decision list.

use std::collections::BTreeMap;
use std::path::PathBuf;

use dmls::graph::{NodeKind, WikiResolution, WorkspaceGraph};
use dmls::graph::substrate::index_document;

const SOURCE: &str = "\
# Source

## Local Heading

[[Target]]
[[folder/Target]]
[[/notes/folder/Target]]
[[My Note]]
[[My%20Note]]
[[my-note]]
[[Case]]
[[case]]
[[Same]]
[[Headed#Exact Heading]]
[[Headed#exact-heading]]
[[Headed#Missing Heading]]
[[#Local Heading]]
[[literal/note.md]]
[[No Such Note]]
[[Target|Alias]]
![[embed]]
[[bad%2]]
[[]]
";

const HEADED: &str = "# Headed\n\n## Exact Heading\n\n## exact-heading\n";

/// Builds the two-root fixture graph.
fn fixture() -> WorkspaceGraph {
    let files: &[(&str, &str)] = &[
        ("/ws/root-a/notes/Source.md", SOURCE),
        ("/ws/root-a/notes/Target.md", "# Target\n"),
        ("/ws/root-a/notes/other/Target.md", "# Other Target\n"),
        ("/ws/root-a/notes/folder/Target.md", "# Folder Target\n"),
        ("/ws/root-a/notes/My Note.md", "# My Note\n"),
        ("/ws/root-a/notes/my-note.md", "# my-note\n"),
        ("/ws/root-a/notes/Case.md", "# Case\n"),
        ("/ws/root-a/notes/case.md", "# case\n"),
        ("/ws/root-a/notes/ambiguous/Same.md", "# Same A\n"),
        ("/ws/root-a/notes/also-ambiguous/Same.md", "# Same B\n"),
        ("/ws/root-a/notes/headings/Headed.md", HEADED),
        ("/ws/root-a/notes/literal/note.md.md", "# Literal\n"),
        ("/ws/root-b/notes/Target.md", "# Root B Target\n"),
        ("/ws/root-b/notes/ambiguous/Same.md", "# Same C\n"),
    ];
    let mut indices = BTreeMap::new();
    for (path, source) in files {
        let path = PathBuf::from(path);
        indices.insert(path.clone(), index_document(&path, source));
    }
    let roots = vec![PathBuf::from("/ws/root-a"), PathBuf::from("/ws/root-b")];
    WorkspaceGraph::build_with_roots(&indices, 1, &roots)
}

/// The Source document's wiki-link payloads in document order.
fn source_wiki_links(graph: &WorkspaceGraph) -> Vec<dmls::graph::WikiLinkPayload> {
    let source = graph
        .document_id(&PathBuf::from("/ws/root-a/notes/Source.md"))
        .expect("source indexed");
    graph
        .wiki_links(source)
        .filter_map(|(_, node)| node.as_wiki_link().cloned())
        .collect()
}

/// The display path of a resolution's target document.
fn resolved_path(graph: &WorkspaceGraph, resolution: &WikiResolution) -> Option<String> {
    let node = match resolution {
        WikiResolution::Resolved(node) | WikiResolution::HeadingMissing(node) => *node,
        _ => return None,
    };
    let document = graph.node(node)?.document;
    Some(graph.document(document)?.path.display().to_string())
}

/// Finds the first Source wiki link matching a `(target, heading)` pair.
fn link<'a>(
    links: &'a [dmls::graph::WikiLinkPayload],
    target: &str,
    heading: Option<&str>,
) -> &'a dmls::graph::WikiLinkPayload {
    links
        .iter()
        .find(|link| link.target == target && link.heading.as_deref() == heading)
        .unwrap_or_else(|| panic!("no wiki link [[{target}#{heading:?}]]"))
}

#[test]
fn wiki_same_directory_wins_over_unique() {
    let graph = fixture();
    let links = source_wiki_links(&graph);
    // `[[Target]]` from notes/ resolves to notes/Target.md, not folder/other.
    let resolution = &link(&links, "Target", None).resolution;
    assert!(
        resolved_path(&graph, resolution)
            .unwrap()
            .ends_with("notes/Target.md"),
        "same-directory rank should win: {resolution:?}"
    );
}

#[test]
fn wiki_suffix_and_root_relative() {
    let graph = fixture();
    let links = source_wiki_links(&graph);
    for target in ["folder/Target", "/notes/folder/Target"] {
        let resolution = &link(&links, target, None).resolution;
        assert!(
            resolved_path(&graph, resolution)
                .unwrap()
                .ends_with("notes/folder/Target.md"),
            "[[{target}]] should resolve to folder/Target.md: {resolution:?}"
        );
    }
}

#[test]
fn wiki_percent_decode_and_literal_spaces() {
    let graph = fixture();
    let links = source_wiki_links(&graph);
    let plain = resolved_path(&graph, &link(&links, "My Note", None).resolution);
    let encoded = resolved_path(&graph, &link(&links, "My%20Note", None).resolution);
    assert_eq!(plain, encoded);
    assert!(plain.unwrap().ends_with("notes/My Note.md"));
}

#[test]
fn wiki_case_sensitive_and_no_dash_equivalence() {
    let graph = fixture();
    let links = source_wiki_links(&graph);
    assert!(
        resolved_path(&graph, &link(&links, "Case", None).resolution)
            .unwrap()
            .ends_with("notes/Case.md")
    );
    assert!(
        resolved_path(&graph, &link(&links, "case", None).resolution)
            .unwrap()
            .ends_with("notes/case.md")
    );
    // `my-note` matches my-note.md, never "My Note.md" (no dash/space equivalence).
    assert!(
        resolved_path(&graph, &link(&links, "my-note", None).resolution)
            .unwrap()
            .ends_with("notes/my-note.md")
    );
}

#[test]
fn wiki_ambiguous_across_roots() {
    let graph = fixture();
    let links = source_wiki_links(&graph);
    match &link(&links, "Same", None).resolution {
        WikiResolution::Ambiguous(candidates) => assert_eq!(candidates.len(), 3),
        other => panic!("[[Same]] should be ambiguous: {other:?}"),
    }
}

#[test]
fn wiki_heading_exact_and_slug_fallback() {
    let graph = fixture();
    let links = source_wiki_links(&graph);

    // Exact visible text resolves to a heading node in Headed.md.
    let exact = &link(&links, "Headed", Some("Exact Heading")).resolution;
    match exact {
        WikiResolution::Resolved(node) => {
            assert_eq!(graph.node(*node).unwrap().kind, NodeKind::Heading);
            assert!(resolved_path(&graph, exact).unwrap().ends_with("Headed.md"));
        }
        other => panic!("exact heading should resolve: {other:?}"),
    }

    // Same-document `[[#Local Heading]]` resolves inside Source.
    let local = &link(&links, "", Some("Local Heading")).resolution;
    assert!(matches!(local, WikiResolution::Resolved(_)));

    // A missing heading resolves the file but flags the fragment.
    let missing = &link(&links, "Headed", Some("Missing Heading")).resolution;
    assert!(matches!(missing, WikiResolution::HeadingMissing(_)));
}

#[test]
fn wiki_doubled_extension_and_confusing_info() {
    let graph = fixture();
    let links = source_wiki_links(&graph);
    let link = link(&links, "literal/note.md", None);
    assert!(
        resolved_path(&graph, &link.resolution)
            .unwrap()
            .ends_with("note.md.md")
    );
    assert_eq!(link.info, Some(dmls::graph::WikiInfo::ConfusingExtension));
}

#[test]
fn wiki_unresolved_unsupported_empty_and_percent() {
    let graph = fixture();
    let links = source_wiki_links(&graph);
    assert!(matches!(
        link(&links, "No Such Note", None).resolution,
        WikiResolution::Unresolved
    ));
    // Embed `![[…]]` is a v1-unsupported form.
    assert!(matches!(
        link(&links, "embed", None).resolution,
        WikiResolution::Unsupported
    ));
    assert!(matches!(
        link(&links, "", None).resolution,
        WikiResolution::EmptyTarget
    ));
    // Malformed percent escape is left literal and flagged.
    let bad = link(&links, "bad%2", None);
    assert!(matches!(bad.resolution, WikiResolution::Unresolved));
    assert_eq!(bad.info, Some(dmls::graph::WikiInfo::InvalidPercentEscape));
}

#[test]
fn wiki_alias_preserved() {
    let graph = fixture();
    let links = source_wiki_links(&graph);
    let aliased = links
        .iter()
        .find(|link| link.target == "Target" && link.alias.is_some())
        .expect("aliased Target link");
    assert_eq!(aliased.alias.as_deref(), Some("Alias"));
    assert!(matches!(aliased.resolution, WikiResolution::Resolved(_)));
}

#[test]
fn wiki_portability_collision_detected() {
    let graph = fixture();
    // Case.md and case.md collide under case-fold; each flags the other.
    let case = graph
        .document_id(&PathBuf::from("/ws/root-a/notes/Case.md"))
        .unwrap();
    assert!(
        !graph.portability_twins(case).is_empty(),
        "Case.md / case.md should collide"
    );
}

#[test]
fn wiki_backlinks_reach_the_target() {
    let graph = fixture();
    // Target.md's root has incoming references from the two `[[Target]]` links.
    let target = graph
        .document_id(&PathBuf::from("/ws/root-a/notes/Target.md"))
        .unwrap();
    let root = graph.document(target).unwrap().root;
    let backlinks = graph
        .incoming(root, dmls::graph::EdgeKind::References)
        .count();
    assert_eq!(backlinks, 2, "[[Target]] and [[Target|Alias]] both backlink");
}
