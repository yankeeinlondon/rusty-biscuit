//! gitGraph merge repair and tag spacing (`mermaid::gitgraph`).
//!
//! Overlap evidence comes from the same layout that renders, never from the
//! Mermaid text.

use mermaid_rs_renderer::{LayoutConfig, Theme};

use crate::mermaid::gitgraph::{repair_gitgraph_merges, GitGraphGeometry};
use crate::mermaid::{MermaidDiagram, MermaidError, MermaidTheme};

const ALPHA: &str = "feature/very-long-exact-branch-reference-alpha";
const BETA: &str = "origin/very-long-exact-branch-reference-beta";

fn parse(text: &str) -> mermaid_rs_renderer::Graph {
    mermaid_rs_renderer::parse_mermaid(text).expect("parses").graph
}

fn repaired(text: &str) -> Result<mermaid_rs_renderer::Graph, MermaidError> {
    let mut graph = parse(text);
    repair_gitgraph_merges(&mut graph)?;
    Ok(graph)
}

fn parents(graph: &mermaid_rs_renderer::Graph, id: &str) -> Vec<String> {
    graph
        .gitgraph
        .commits
        .iter()
        .find(|commit| commit.id == id)
        .unwrap_or_else(|| panic!("commit {id}"))
        .parents
        .clone()
}

fn gitgraph(lines: &[&str]) -> String {
    std::iter::once("gitGraph".to_string())
        .chain(lines.iter().map(|line| format!("    {line}")))
        .collect::<Vec<_>>()
        .join("\n")
}

fn render_failure(result: Result<mermaid_rs_renderer::Graph, MermaidError>) -> String {
    match result {
        Err(MermaidError::RenderFailed(reason)) => reason,
        other => panic!("expected RenderFailed, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Merge repair
// ---------------------------------------------------------------------------

#[test]
fn the_parser_message_still_carries_the_whole_merge_suffix() {
    // The repair depends on this exact message; a dependency upgrade that
    // changes it must fail here, not silently skip every repair.
    let graph = parse(&gitgraph(&[
        "commit id: \"A\"",
        "branch side",
        "checkout side",
        "commit id: \"S1\"",
        "checkout main",
        "merge side id: \"M\" tag: \"v1\"",
    ]));
    let merge = graph.gitgraph.commits.iter().find(|commit| commit.id == "M").unwrap();
    assert_eq!(
        merge.message.as_deref(),
        Some("merged branch side id: \"M\" tag: \"v1\" into main")
    );
    assert_eq!(merge.parents, ["A"], "0.3.1 loses a labeled merge's second parent");
    assert_eq!(merge.tags, ["v1"]);
}

#[test]
fn a_labeled_merge_gets_its_source_tip_as_second_parent() {
    let graph = repaired(&gitgraph(&[
        "commit id: \"A\"",
        "branch side",
        "checkout side",
        "commit id: \"S1\"",
        "commit id: \"S2\"",
        "checkout main",
        "commit id: \"B\"",
        "merge side id: \"M\" tag: \"origin/main\"",
        "commit id: \"C\"",
    ]))
    .unwrap();
    assert_eq!(parents(&graph, "M"), ["B", "S2"]);
    assert_eq!(parents(&graph, "C"), ["M"]);
    let merge = graph.gitgraph.commits.iter().find(|commit| commit.id == "M").unwrap();
    assert_eq!(merge.tags, ["origin/main"]);
    assert_eq!(merge.branch, "main");
}

#[test]
fn a_merge_the_parser_already_resolved_is_untouched() {
    // Unlabeled: the parser finds the lane itself.
    let unlabeled = gitgraph(&[
        "commit id: \"A\"",
        "branch side",
        "checkout side",
        "commit id: \"S1\"",
        "checkout main",
        "merge side",
    ]);
    let before = parse(&unlabeled);
    let merge_id = before.gitgraph.commits.last().unwrap().id.clone();
    assert_eq!(parents(&before, &merge_id), ["A", "S1"]);
    assert_eq!(parents(&repaired(&unlabeled).unwrap(), &merge_id), ["A", "S1"]);

    // Two parents from any source (a fixed parser) are left as they are.
    let mut graph = parse(&gitgraph(&[
        "commit id: \"A\"",
        "branch side",
        "checkout side",
        "commit id: \"S1\"",
        "checkout main",
        "merge side id: \"M\"",
    ]));
    let merge = graph.gitgraph.commits.iter_mut().find(|commit| commit.id == "M").unwrap();
    merge.parents = vec!["A".into(), "S1".into()];
    repair_gitgraph_merges(&mut graph).unwrap();
    assert_eq!(parents(&graph, "M"), ["A", "S1"]);
}

#[test]
fn a_merge_into_a_lane_without_a_head_keeps_its_single_source_parent() {
    // The parser's shape for an unlabeled `merge side` into a lane with no
    // commits: the single parent already is the source tip, so no duplicate
    // is added.
    let graph = repaired(&gitgraph(&[
        "branch side",
        "checkout side",
        "commit id: \"S1\"",
        "checkout main",
        "merge side",
        "commit id: \"A\"",
    ]))
    .unwrap();
    let merge = graph.gitgraph.commits.iter().find(|commit| commit.branch == "main").unwrap();
    assert_eq!(merge.parents, ["S1"]);

    // A labeled one there is dropped by the parser itself (it finds neither
    // the suffix lane nor a head), so the repair never sees it.
    let dropped = repaired(&gitgraph(&[
        "branch side",
        "checkout side",
        "commit id: \"S1\"",
        "checkout main",
        "merge side id: \"M\"",
    ]))
    .unwrap();
    assert!(dropped.gitgraph.commits.iter().all(|commit| commit.id != "M"));
}

#[test]
fn a_nested_lane_merges_into_the_root_lane() {
    // The second observation's shape: `fix/sniff` forks from `fix/wt-ux`, and
    // both merge into the default lane, each merge labeled.
    let graph = repaired(&gitgraph(&[
        "commit id: \"D1\"",
        "branch fix/wt-ux",
        "checkout fix/wt-ux",
        "commit id: \"W1\"",
        "commit id: \"W2\"",
        "branch fix/sniff",
        "checkout fix/sniff",
        "commit id: \"+96\" type: HIGHLIGHT",
        "commit id: \"S1\"",
        "checkout fix/wt-ux",
        "checkout main",
        "merge fix/wt-ux id: \"M103\" tag: \"main\"",
        "merge fix/sniff id: \"M104\" tag: \"origin/main\"",
    ]))
    .unwrap();
    assert_eq!(parents(&graph, "M103"), ["D1", "W2"]);
    assert_eq!(parents(&graph, "M104"), ["M103", "S1"]);
    assert_eq!(parents(&graph, "+96"), ["W2"]);
}

#[test]
fn a_merge_into_a_non_root_lane_is_repaired() {
    let graph = repaired(&gitgraph(&[
        "commit id: \"A\"",
        "branch parent",
        "checkout parent",
        "commit id: \"P1\"",
        "branch child",
        "checkout child",
        "commit id: \"C1\"",
        "checkout parent",
        "merge child id: \"PM\" tag: \"parent-merge\"",
    ]))
    .unwrap();
    assert_eq!(parents(&graph, "PM"), ["P1", "C1"]);
    let merge = graph.gitgraph.commits.iter().find(|commit| commit.id == "PM").unwrap();
    assert_eq!(merge.branch, "parent");
}

#[test]
fn a_lane_name_that_prefixes_another_resolves_to_the_exact_name() {
    // `feat` is followed by `x`, not an attribute key, so only `feat x` matches.
    let graph = repaired(&gitgraph(&[
        "commit id: \"A\"",
        "branch feat",
        "checkout feat",
        "commit id: \"F1\"",
        "checkout main",
        "branch feat x",
        "commit id: \"G1\"",
        "checkout main",
        "merge feat x id: \"M\"",
    ]))
    .unwrap();
    assert_eq!(parents(&graph, "M"), ["A", "G1"]);
}

#[test]
fn a_suffix_that_names_two_lanes_is_rejected() {
    let reason = render_failure(repaired(&gitgraph(&[
        "commit id: \"A\"",
        "branch x",
        "checkout x",
        "commit id: \"X1\"",
        "checkout main",
        "branch x tag: \"t\"",
        "commit id: \"Y1\"",
        "checkout main",
        "merge x tag: \"t\" id: \"M\"",
    ])));
    assert!(reason.contains("\"M\"") && reason.contains("several lanes"), "{reason}");
}

#[test]
fn a_merge_from_an_unknown_lane_is_rejected() {
    let reason = render_failure(repaired(&gitgraph(&[
        "commit id: \"A\"",
        "checkout main",
        "merge nowhere id: \"M\"",
    ])));
    assert!(reason.contains("no lane matches"), "{reason}");
}

#[test]
fn a_merge_from_an_empty_lane_is_rejected() {
    let reason = render_failure(repaired(&gitgraph(&[
        "commit id: \"A\"",
        "branch side",
        "checkout main",
        "commit id: \"B\"",
        "merge side id: \"M\"",
    ])));
    assert!(reason.contains("\"side\" has no commit"), "{reason}");
}

#[test]
fn measurement_and_geometry_report_a_rejected_merge() {
    let diagram = MermaidDiagram::new(gitgraph(&["commit id: \"A\"", "merge nowhere id: \"M\""]));
    assert!(matches!(diagram.natural_size(), Err(MermaidError::RenderFailed(_))));
    assert!(matches!(diagram.gitgraph_geometry(), Err(MermaidError::RenderFailed(_))));
}

#[test]
fn geometry_carries_the_repaired_parents() {
    let geometry = MermaidDiagram::new(gitgraph(&[
        "commit id: \"A\"",
        "branch side",
        "checkout side",
        "commit id: \"S1\"",
        "checkout main",
        "merge side id: \"M\" tag: \"main\"",
    ]))
    .gitgraph_geometry()
    .unwrap()
    .expect("a gitGraph");
    let merge = geometry.commit("M").unwrap();
    assert_eq!(merge.parents, ["A", "S1"]);
    assert_eq!(merge.lane, "main");
    assert_eq!(geometry.commit("S1").unwrap().lane, "side");
}

#[test]
fn geometry_is_none_for_other_diagrams() {
    assert_eq!(MermaidDiagram::new("flowchart LR\n    A --> B").gitgraph_geometry().unwrap(), None);
}

// ---------------------------------------------------------------------------
// Tag spacing
// ---------------------------------------------------------------------------

const THEMES: [MermaidTheme; 2] = [MermaidTheme::Default, MermaidTheme::Dark];

fn geometry(text: &str, theme: MermaidTheme) -> GitGraphGeometry {
    MermaidDiagram::new(text)
        .with_theme(theme)
        .gitgraph_geometry()
        .unwrap_or_else(|error| panic!("{error}: {text}"))
        .expect("a gitGraph")
}

/// The single default pass, for the control rows.
fn unspaced_geometry(text: &str) -> GitGraphGeometry {
    let mut graph = parse(text);
    repair_gitgraph_merges(&mut graph).unwrap();
    let theme = Theme::mermaid_default();
    let config = LayoutConfig::default();
    let layout = mermaid_rs_renderer::compute_layout(&graph, &theme, &config);
    GitGraphGeometry::from_layout(&graph, &layout, &config).unwrap()
}

/// Asserts no overlaps and that every tag sits on the commit the text puts it on.
fn assert_spaced(text: &str, expected: &[(&str, &str)]) {
    for theme in THEMES {
        let laid_out = geometry(text, theme);
        assert_eq!(laid_out.tag_overlaps(), Vec::<(String, String)>::new(), "{theme:?}: {text}");
        let mut placed: Vec<(&str, &str)> = laid_out.tag_boxes().into_iter().map(|(id, tag, _)| (id, tag)).collect();
        placed.sort();
        let mut wanted = expected.to_vec();
        wanted.sort();
        assert_eq!(placed, wanted, "{theme:?}");
        assert!(laid_out.commit_step > LayoutConfig::default().gitgraph.commit_step, "{theme:?} widened");
    }
}

#[test]
fn long_labels_on_neighboring_commits_do_not_overlap() {
    let text = gitgraph(&[
        "commit id: \"a1\"",
        &format!("commit id: \"a2\" tag: \"{ALPHA}\""),
        &format!("commit id: \"a3\" tag: \"{BETA}\""),
        "commit id: \"a4\" tag: \"PR #104 → main\"",
    ]);
    // Control: the default layout overlaps, so the spacing is what fixes it.
    assert!(!unspaced_geometry(&text).tag_overlaps().is_empty());
    assert_spaced(&text, &[("a2", ALPHA), ("a3", BETA), ("a4", "PR #104 → main")]);
}

#[test]
fn main_and_origin_main_one_commit_apart_do_not_overlap() {
    let text = gitgraph(&[
        "commit id: \"d1\"",
        "commit id: \"d2\" tag: \"main\"",
        "commit id: \"d3\" tag: \"origin/main\"",
    ]);
    assert!(!unspaced_geometry(&text).tag_overlaps().is_empty());
    assert_spaced(&text, &[("d2", "main"), ("d3", "origin/main")]);
}

#[test]
fn a_stack_of_three_tags_clears_its_neighbors() {
    let text = gitgraph(&[
        "commit id: \"d1\"",
        &format!("commit id: \"d2\" tag: \"{ALPHA}\""),
        "commit id: \"d3\" tag: \"main\" tag: \"origin/main\" tag: \"release/2026-09\"",
        "commit id: \"d4\" tag: \"v1.0.0\"",
    ]);
    assert!(!unspaced_geometry(&text).tag_overlaps().is_empty());
    assert_spaced(
        &text,
        &[
            ("d2", ALPHA),
            ("d3", "main"),
            ("d3", "origin/main"),
            ("d3", "release/2026-09"),
            ("d4", "v1.0.0"),
        ],
    );
}

#[test]
fn tags_on_different_lanes_do_not_overlap() {
    let text = gitgraph(&[
        "commit id: \"d1\"",
        "commit id: \"d2\"",
        "branch feature",
        "checkout feature",
        &format!("commit id: \"f1\" tag: \"{BETA}\""),
        "commit id: \"f2\" tag: \"PR #104 → main\"",
        "checkout main",
        "commit id: \"d3\" tag: \"origin/main\"",
        &format!("commit id: \"d4\" tag: \"main\" tag: \"{ALPHA}\""),
    ]);
    assert!(!unspaced_geometry(&text).tag_overlaps().is_empty());
    assert_spaced(
        &text,
        &[
            ("f1", BETA),
            ("f2", "PR #104 → main"),
            ("d3", "origin/main"),
            ("d4", "main"),
            ("d4", ALPHA),
        ],
    );
}

#[test]
fn a_diagram_without_tags_keeps_the_single_pass_layout() {
    let text = gitgraph(&[
        "commit id: \"a1\"",
        "branch side",
        "checkout side",
        "commit id: \"s1\"",
        "checkout main",
        "merge side id: \"M\"",
    ]);
    let laid_out = geometry(&text, MermaidTheme::Default);
    assert_eq!(laid_out, unspaced_geometry(&text));
    assert_eq!(laid_out.commit_step, LayoutConfig::default().gitgraph.commit_step);
}

#[test]
fn a_vertical_graph_with_tags_keeps_the_single_pass_layout() {
    // Vertical graphs are not spaced (documented limitation).
    for direction in ["TB", "BT"] {
        // 0.3.1 reads the direction from its own line, not the header.
        let text = format!("gitGraph\n    direction {direction}\n    commit id: \"a1\" tag: \"main\"\n    commit id: \"a2\" tag: \"origin/main\"");
        assert_eq!(
            geometry(&text, MermaidTheme::Default).commit_step,
            LayoutConfig::default().gitgraph.commit_step,
            "{direction}"
        );
    }
    let horizontal = "gitGraph\n    direction LR\n    commit id: \"a1\" tag: \"main\"\n    commit id: \"a2\" tag: \"origin/main\"";
    assert!(geometry(horizontal, MermaidTheme::Default).commit_step > LayoutConfig::default().gitgraph.commit_step);
}

#[test]
fn the_spaced_layout_is_the_measured_one() {
    let text = gitgraph(&[
        "commit id: \"a1\"",
        &format!("commit id: \"a2\" tag: \"{ALPHA}\""),
        &format!("commit id: \"a3\" tag: \"{BETA}\""),
    ]);
    let diagram = MermaidDiagram::new(text.as_str());
    let size = diagram.natural_size().unwrap();
    let laid_out = diagram.gitgraph_geometry().unwrap().unwrap();
    assert!(size.width > unspaced_geometry(&text).width, "{size:?}");
    assert!(size.width >= laid_out.width, "{size:?} vs {}", laid_out.width);
}
