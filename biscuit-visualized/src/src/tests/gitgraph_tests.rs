//! gitGraph merge repair and tag spacing (`mermaid::gitgraph`).
//!
//! Overlap evidence comes from the same layout that renders, never from the
//! Mermaid text.

use mermaid_rs_renderer::{LayoutConfig, Theme};

use crate::mermaid::gitgraph::{
    repair_gitgraph_merges, required_step, spaced, Bounds, GitGraphGeometry, LabeledCommit, MAX_SPACING_PASSES,
    SPACING_EPSILON,
};
use crate::mermaid::{default_gitgraph_commit_step, MermaidDiagram, MermaidError, MermaidTheme};

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
// Tag spacing: the step calculation on synthetic labels
// ---------------------------------------------------------------------------

const STEP: f32 = 34.0;
const GAP: f32 = 16.0;

fn label(index: usize, x: f32, x1: f32, x2: f32) -> LabeledCommit {
    LabeledCommit {
        index,
        x,
        bounds: Bounds { x1, y1: 0.0, x2, y2: 10.0 },
    }
}

fn at_height(mut label: LabeledCommit, y1: f32, y2: f32) -> LabeledCommit {
    label.bounds.y1 = y1;
    label.bounds.y2 = y2;
    label
}

fn assert_close(actual: Option<f32>, expected: f32) {
    let actual = actual.expect("a widened step");
    assert!((actual - expected).abs() < 1e-4, "{actual} != {expected}");
}

#[test]
fn no_labels_or_one_label_needs_no_step() {
    assert_eq!(required_step(STEP, GAP, &[]), None);
    assert_eq!(required_step(STEP, GAP, &[label(0, 0.0, -40.0, 40.0)]), None);
}

#[test]
fn tags_on_one_commit_add_no_requirement() {
    // A stack: one commit, identical x extents, overlapping heights.
    let stack = [label(3, 102.0, 60.0, 140.0), at_height(label(3, 102.0, 60.0, 140.0), 5.0, 15.0)];
    assert_eq!(required_step(STEP, GAP, &stack), None);
}

#[test]
fn labels_without_vertical_overlap_add_no_requirement() {
    // Same x extent, different rows; a shared edge is not an overlap.
    let apart = [label(0, 0.0, -40.0, 40.0), at_height(label(1, 34.0, -6.0, 74.0), 20.0, 30.0)];
    assert_eq!(required_step(STEP, GAP, &apart), None);
    let touching = [label(0, 0.0, -40.0, 40.0), at_height(label(1, 34.0, -6.0, 74.0), 10.0, 20.0)];
    assert_eq!(required_step(STEP, GAP, &touching), None);
    // Control: one unit of vertical overlap makes the same pair count.
    let overlapping = [label(0, 0.0, -40.0, 40.0), at_height(label(1, 34.0, -6.0, 74.0), 9.0, 20.0)];
    assert!(required_step(STEP, GAP, &overlapping).is_some());
}

#[test]
fn an_adjacent_overlapping_pair_gets_exactly_the_missing_gap() {
    // Clearance 40 - 50 = -10, so the pair is 26 short of the gap.
    assert_close(required_step(STEP, GAP, &[label(0, 0.0, -10.0, 50.0), label(1, 34.0, 40.0, 60.0)]), STEP + 26.0);
}

#[test]
fn a_pair_three_indices_apart_divides_its_shortfall_by_three() {
    assert_close(
        required_step(STEP, GAP, &[label(0, 0.0, -10.0, 110.0), label(3, 102.0, 100.0, 120.0)]),
        STEP + 26.0 / 3.0,
    );
}

#[test]
fn unequal_and_offset_labels_measure_edge_to_edge() {
    // A wide earlier label reaching past the later commit; the tags are not
    // centered on their commits (the renderer's offset).
    let labels = [label(4, 136.0, 100.22, 301.49), label(5, 170.0, 158.0, 178.0)];
    assert_close(required_step(STEP, GAP, &labels), STEP + (GAP - (158.0 - 301.49)));
    // Order in the slice does not matter.
    let reversed = [labels[1], labels[0]];
    assert_eq!(required_step(STEP, GAP, &labels), required_step(STEP, GAP, &reversed));
}

#[test]
fn right_to_left_labels_take_the_smaller_x_as_earlier() {
    // Mirrored: index 0 sits right of index 1, so index 1's label is earlier.
    let labels = [label(0, 100.0, 90.0, 130.0), label(1, 66.0, 50.0, 95.0)];
    assert_close(required_step(STEP, GAP, &labels), STEP + (GAP - (90.0 - 95.0)));
}

#[test]
fn a_pair_within_epsilon_of_the_gap_is_clear() {
    let within = [label(0, 0.0, -10.0, 10.0), label(1, 34.0, 10.0 + GAP - SPACING_EPSILON / 2.0, 40.0)];
    assert_eq!(required_step(STEP, GAP, &within), None);
    let beyond = [label(0, 0.0, -10.0, 10.0), label(1, 34.0, 10.0 + GAP - SPACING_EPSILON * 2.0, 40.0)];
    assert_close(required_step(STEP, GAP, &beyond), STEP + SPACING_EPSILON * 2.0);
}

#[test]
fn the_widest_requirement_over_several_pairs_wins() {
    let labels = [
        label(0, 0.0, -10.0, 30.0),  // with 1: clearance 0, short 16
        label(1, 34.0, 30.0, 60.0),  // with 2: clearance -20, short 36
        label(2, 68.0, 40.0, 80.0),
        label(6, 204.0, 200.0, 210.0),
    ];
    assert_close(required_step(STEP, GAP, &labels), STEP + 36.0);
}

// ---------------------------------------------------------------------------
// Tag spacing: the layout loop over an injected placement
// ---------------------------------------------------------------------------

/// A linear placement like the renderer's: a label's edges move by its
/// commit's index times the step.
fn linear(step: f32) -> Vec<LabeledCommit> {
    [(0, -30.0, 30.0), (1, -20.0, 20.0)]
        .into_iter()
        .map(|(index, left, right)| {
            let x = index as f32 * step;
            label(index, x, x + left, x + right)
        })
        .collect()
}

#[test]
fn an_unspaced_placement_is_laid_out_once() {
    let mut steps = Vec::new();
    let (_, step) = spaced(STEP, GAP, |step| {
        steps.push(step);
        ((), None)
    });
    assert_eq!((steps, step), (vec![STEP], STEP));

    let mut steps = Vec::new();
    let (_, step) = spaced(STEP, GAP, |step| {
        steps.push(step);
        ((), Some(vec![label(0, 0.0, -30.0, 30.0)]))
    });
    assert_eq!((steps, step), (vec![STEP], STEP));
}

#[test]
fn a_linear_placement_verifies_after_one_widening() {
    let mut steps = Vec::new();
    let (laid, step) = spaced(STEP, GAP, |step| {
        steps.push(step);
        (step, Some(linear(step)))
    });
    // 30 + 20 + 16 = 66 between commit centers.
    assert_eq!(steps, [STEP, 66.0]);
    assert_eq!((laid, step), (66.0, 66.0));
    assert_eq!(required_step(step, GAP, &linear(step)), None);
}

#[test]
fn a_placement_that_never_clears_falls_back_to_the_widest_tag() {
    // Labels that ignore the step can never be widened apart.
    let stuck = vec![label(0, 0.0, -10.0, 50.0), label(1, 34.0, 40.0, 60.0)];
    let mut steps = Vec::new();
    let (laid, step) = spaced(STEP, GAP, |step| {
        steps.push(step);
        (step, Some(stuck.clone()))
    });
    assert_eq!(steps.len(), 1 + MAX_SPACING_PASSES, "{steps:?}");
    let chosen = steps[MAX_SPACING_PASSES - 1] + 26.0;
    assert_eq!(step, chosen.max(60.0 + GAP));
    assert_eq!(laid, step, "the fallback layout is the one returned");
    assert!(steps.windows(2).all(|pair| pair[0] < pair[1]), "{steps:?}");
}

// ---------------------------------------------------------------------------
// Tag spacing: real layouts
// ---------------------------------------------------------------------------

const THEMES: [MermaidTheme; 2] = [MermaidTheme::Default, MermaidTheme::Dark];

/// `wt list`'s gathered Mermaid text for the observed sparse-lanes history:
/// its only tags, `main` and `origin/main`, stack on one commit.
const OBSERVED: &str = r#"gitGraph
    branch fix/sniff
    checkout main
    commit id: "a0f1d1e"
    checkout fix/sniff
    commit id: "+89" type: HIGHLIGHT
    commit id: "85b1c02"
    commit id: "34022b8"
    commit id: "7966e7a"
    commit id: "1bfaea2"
    commit id: "51bfb8a"
    checkout main
    commit id: "12ee425"
    branch feat/schema-enhancement
    checkout feat/schema-enhancement
    commit id: "+50" type: HIGHLIGHT
    commit id: "a811c10"
    commit id: "5ddf29b"
    commit id: "4385291"
    commit id: "4002e8b"
    commit id: "389e3dc"
    checkout main
    commit id: "+3" type: HIGHLIGHT
    commit id: "dfbdce6"
    commit id: "a91fbf0"
    commit id: "92d4328"
    commit id: "3102d9a"
    commit id: "f234565"
    commit id: "2658996"
    commit id: "5fb5502"
    commit id: "f2a3857"
    commit id: "82ecff7"
    commit id: "999032d" tag: "main" tag: "origin/main"
    branch fix/wt-ux
    checkout fix/wt-ux
    commit id: "+63" type: HIGHLIGHT
    commit id: "49f02c6"
    commit id: "b6d2e53"
    commit id: "1e514f6"
    commit id: "d0063a1"
    commit id: "ff61ca9"
    checkout main"#;

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
    GitGraphGeometry::from_layout(&graph, &layout, &config, &theme).unwrap()
}

/// The edge-to-edge clearance of every pair that spacing governs: tags on
/// different commits whose labels share vertical extent.
fn governed_clearances(laid_out: &GitGraphGeometry) -> Vec<f32> {
    let tags: Vec<_> = laid_out
        .commits
        .iter()
        .flat_map(|commit| commit.tags.iter().map(move |tag| (commit, tag.bounds)))
        .collect();
    let mut clearances = Vec::new();
    for (position, (a, a_bounds)) in tags.iter().enumerate() {
        for (b, b_bounds) in &tags[position + 1..] {
            if a.index == b.index || !(a_bounds.y1 < b_bounds.y2 && b_bounds.y1 < a_bounds.y2) {
                continue;
            }
            let (earlier, later) = if a.x <= b.x { (a_bounds, b_bounds) } else { (b_bounds, a_bounds) };
            clearances.push(later.x1 - earlier.x2);
        }
    }
    clearances
}

/// Asserts no overlaps, that every tag sits on the commit the text puts it on,
/// and that the step is the smallest that gives every governed pair the gap:
/// none is closer, and the tightest is exactly the gap.
fn assert_spaced(text: &str, expected: &[(&str, &str)]) {
    for theme in THEMES {
        let laid_out = geometry(text, theme);
        assert_eq!(laid_out.tag_overlaps(), Vec::<(String, String)>::new(), "{theme:?}: {text}");
        let mut placed: Vec<(&str, &str)> = laid_out.tag_boxes().into_iter().map(|(id, tag, _)| (id, tag)).collect();
        placed.sort();
        let mut wanted = expected.to_vec();
        wanted.sort();
        assert_eq!(placed, wanted, "{theme:?}");
        assert!(laid_out.commit_step > default_gitgraph_commit_step(), "{theme:?} widened");
        assert_minimal(&laid_out, &format!("{theme:?}"));
    }
}

fn assert_minimal(laid_out: &GitGraphGeometry, context: &str) {
    let clearances = governed_clearances(laid_out);
    let tightest = clearances.iter().copied().fold(f32::INFINITY, f32::min);
    assert!(
        tightest >= laid_out.tag_gap - SPACING_EPSILON,
        "{context}: a governed pair is closer than the gap: {clearances:?}"
    );
    assert!(
        tightest <= laid_out.tag_gap + SPACING_EPSILON,
        "{context}: step {} is wider than needed, tightest clearance {tightest} vs gap {}",
        laid_out.commit_step,
        laid_out.tag_gap
    );
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
fn tags_stacked_on_one_commit_keep_the_default_step() {
    // Formerly widened to the widest tag plus the gap.
    let text = gitgraph(&[
        "commit id: \"d1\"",
        "commit id: \"d2\"",
        &format!("commit id: \"d3\" tag: \"main\" tag: \"origin/main\" tag: \"{ALPHA}\""),
        "commit id: \"d4\"",
    ]);
    for theme in THEMES {
        let laid_out = geometry(&text, theme);
        assert_eq!(laid_out.commit_step, default_gitgraph_commit_step(), "{theme:?}");
        assert_eq!(laid_out.tag_overlaps(), Vec::<(String, String)>::new(), "{theme:?}");
    }
    assert_eq!(geometry(&text, MermaidTheme::Default), unspaced_geometry(&text));
}

#[test]
fn tags_on_lanes_apart_without_vertical_overlap_keep_the_default_step() {
    // `main`'s tag and the third lane's tag are one column apart and overlap
    // horizontally, but their rows do not meet.
    let text = gitgraph(&[
        "commit id: \"d1\"",
        "branch one",
        "branch two",
        "branch three",
        "checkout main",
        &format!("commit id: \"d2\" tag: \"{ALPHA}\""),
        "checkout three",
        &format!("commit id: \"t1\" tag: \"{BETA}\""),
    ]);
    let control = unspaced_geometry(&text);
    let (main_tag, lane_tag) = (control.commit("d2").unwrap().tags[0].bounds, control.commit("t1").unwrap().tags[0].bounds);
    assert!(main_tag.x1 < lane_tag.x2 && lane_tag.x1 < main_tag.x2, "control: horizontal overlap {main_tag:?} {lane_tag:?}");
    assert_eq!(control.commit("t1").unwrap().index, control.commit("d2").unwrap().index + 1);
    for theme in THEMES {
        let laid_out = geometry(&text, theme);
        assert_eq!(laid_out.commit_step, default_gitgraph_commit_step(), "{theme:?}");
        assert!(governed_clearances(&laid_out).is_empty(), "{theme:?}");
        assert_eq!(laid_out.tag_overlaps(), Vec::<(String, String)>::new(), "{theme:?}");
    }
}

#[test]
fn the_observed_graph_keeps_the_default_step() {
    for theme in THEMES {
        let laid_out = geometry(OBSERVED, theme);
        assert_eq!(laid_out.commit_step, default_gitgraph_commit_step(), "{theme:?}");
        assert_eq!(laid_out.tag_overlaps(), Vec::<(String, String)>::new(), "{theme:?}");
    }
    let mut tagged: Vec<_> = geometry(OBSERVED, MermaidTheme::Default)
        .tag_boxes()
        .into_iter()
        .map(|(id, tag, _)| (id.to_string(), tag.to_string()))
        .collect();
    tagged.sort();
    assert_eq!(
        tagged,
        [("999032d".to_string(), "main".to_string()), ("999032d".to_string(), "origin/main".to_string())]
    );
}

#[test]
fn right_to_left_text_lays_out_like_its_left_to_right_twin() {
    // 0.3.1 parses only LR, TB, and BT; `direction RL` reads as LR, and a
    // programmatic RL graph is not mirrored either.
    let body = "    commit id: \"d1\"\n    commit id: \"d2\" tag: \"main\"\n    commit id: \"d3\" tag: \"origin/main\"";
    for theme in THEMES {
        let right_to_left = geometry(&format!("gitGraph\n    direction RL\n{body}"), theme);
        let left_to_right = geometry(&format!("gitGraph\n    direction LR\n{body}"), theme);
        assert_eq!(right_to_left, left_to_right, "{theme:?}");
        assert_minimal(&right_to_left, &format!("{theme:?} RL"));
    }
}

#[test]
fn the_gap_scales_with_an_overridden_font_size() {
    let body = gitgraph(&[
        "commit id: \"d1\"",
        "commit id: \"d2\" tag: \"main\"",
        "commit id: \"d3\" tag: \"origin/main\"",
    ]);
    let default_font = geometry(&body, MermaidTheme::Default);
    let text = format!("%%{{init: {{'themeVariables': {{'fontSize': 24}}}}}}%%\n{body}");
    let large_font = geometry(&text, MermaidTheme::Default);
    assert_eq!(default_font.tag_gap, 16.0);
    assert_eq!(large_font.tag_gap, 24.0);
    assert_minimal(&default_font, "font 16");
    assert_minimal(&large_font, "font 24");
    // Label widths follow the renderer's tag font, not the theme's, so the
    // larger gap alone widens the step by its 8 units.
    assert!((large_font.commit_step - default_font.commit_step - 8.0).abs() < 1e-3);

    // Measurement shares the spaced layout, so the wider step shows in the
    // measured size. The viewBox is not compared with the layout's own width:
    // with this font override on Windows it comes out narrower.
    let default_size = MermaidDiagram::new(body.as_str()).natural_size().unwrap();
    let large_size = MermaidDiagram::new(text.as_str()).natural_size().unwrap();
    assert!(large_size.width > default_size.width, "{large_size:?} vs {default_size:?}");
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
    assert_eq!(laid_out.commit_step, default_gitgraph_commit_step());
}

#[test]
fn a_vertical_graph_with_tags_keeps_the_single_pass_layout() {
    // Vertical graphs are not spaced (documented limitation).
    for direction in ["TB", "BT"] {
        // 0.3.1 reads the direction from its own line, not the header.
        let text = format!("gitGraph\n    direction {direction}\n    commit id: \"a1\" tag: \"main\"\n    commit id: \"a2\" tag: \"origin/main\"");
        assert_eq!(
            geometry(&text, MermaidTheme::Default).commit_step,
            default_gitgraph_commit_step(),
            "{direction}"
        );
    }
    let horizontal = "gitGraph\n    direction LR\n    commit id: \"a1\" tag: \"main\"\n    commit id: \"a2\" tag: \"origin/main\"";
    assert!(geometry(horizontal, MermaidTheme::Default).commit_step > default_gitgraph_commit_step());
}

#[test]
fn the_default_step_is_the_renderers() {
    assert_eq!(default_gitgraph_commit_step(), LayoutConfig::default().gitgraph.commit_step);
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
