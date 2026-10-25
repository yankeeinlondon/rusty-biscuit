//! gitGraph geometry and tag spacing through the public
//! `MermaidDiagram::gitgraph_geometry` (AC-C11, AC-C12).
//!
//! The loader, the tag-box formula, and the spacing arithmetic are tested on
//! `mermaid::gitgraph_layout` directly in `gitgraph_layout_tests`; these tests
//! prove the wiring: pipeline step 3b writes `commitSpacing` into the source
//! Merman lays out, and the geometry the caller gets is that layout.
//!
//! Overlap evidence comes from the same layout that renders, never from the
//! Mermaid text.

use serial_test::serial;

use crate::mermaid::gitgraph::GitGraphGeometry;
use crate::mermaid::{MermaidDiagram, MermaidTheme, default_gitgraph_commit_step};
use crate::tests::isolated_cache_dir;

const ALPHA: &str = "feature/very-long-exact-branch-reference-alpha";
const BETA: &str = "origin/very-long-exact-branch-reference-beta";

/// Spacing is written rounded up to 0.01 px, so the tightest governed pair
/// may be up to that much (per column) wider than the gap.
const TOLERANCE: f32 = 0.02;

const THEMES: [MermaidTheme; 2] = [MermaidTheme::Default, MermaidTheme::Dark];

fn gitgraph(lines: &[&str]) -> String {
    std::iter::once("gitGraph".to_string())
        .chain(lines.iter().map(|line| format!("    {line}")))
        .collect::<Vec<_>>()
        .join("\n")
}

fn geometry(text: &str, theme: MermaidTheme) -> GitGraphGeometry {
    MermaidDiagram::new(text)
        .with_theme(theme)
        .gitgraph_geometry()
        .unwrap_or_else(|error| panic!("{error}: {text}"))
        .expect("a gitGraph")
}

/// The layout at Merman's default spacing, for the control rows: an author's
/// own `commitSpacing` wins over step 3b, so setting the default keeps it.
fn unspaced_geometry(text: &str) -> GitGraphGeometry {
    let pinned = format!("%%{{init: {{\"gitGraph\": {{\"commitSpacing\": 50}}}}}}%%\n{text}");
    geometry(&pinned, MermaidTheme::Default)
}

/// The `commitSpacing` step 3b wrote into the source, if any.
fn written_spacing(text: &str) -> Option<f32> {
    let source = MermaidDiagram::new(text).rewritten_source().expect("step 3b");
    let directive = source.strip_prefix(text)?;
    let value = directive
        .split("\"commitSpacing\": ")
        .nth(1)?
        .split('}')
        .next()?;
    Some(value.parse().expect("a number"))
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
            let (earlier, later) = if a.x <= b.x {
                (a_bounds, b_bounds)
            } else {
                (b_bounds, a_bounds)
            };
            clearances.push(later.x1 - earlier.x2);
        }
    }
    clearances
}

/// Asserts no overlaps, that every tag sits on the commit the text puts it
/// on, that the step is the written `commitSpacing`, and that it is the
/// smallest that gives every governed pair the gap.
fn assert_spaced(text: &str, expected: &[(&str, &str)]) {
    let written = written_spacing(text).expect("colliding tags get a commitSpacing");
    for theme in THEMES {
        let laid_out = geometry(text, theme);
        assert_eq!(
            laid_out.tag_overlaps(),
            Vec::<(String, String)>::new(),
            "{theme:?}: {text}"
        );
        let mut placed: Vec<(&str, &str)> = laid_out
            .tag_boxes()
            .into_iter()
            .map(|(id, tag, _)| (id, tag))
            .collect();
        placed.sort();
        let mut wanted = expected.to_vec();
        wanted.sort();
        assert_eq!(placed, wanted, "{theme:?}");
        assert!(
            (laid_out.commit_step - written).abs() < 1e-3,
            "{theme:?}: step {} is the written {written}",
            laid_out.commit_step
        );
        assert!(laid_out.commit_step > default_gitgraph_commit_step(), "{theme:?} widened");
        assert_minimal(&laid_out, &format!("{theme:?}"));
    }
}

fn assert_minimal(laid_out: &GitGraphGeometry, context: &str) {
    let clearances = governed_clearances(laid_out);
    let tightest = clearances.iter().copied().fold(f32::INFINITY, f32::min);
    assert!(
        tightest >= laid_out.tag_gap - 1e-3,
        "{context}: a governed pair is closer than the gap: {clearances:?}"
    );
    assert!(
        tightest <= laid_out.tag_gap + TOLERANCE,
        "{context}: step {} is wider than needed, tightest clearance {tightest} vs gap {}",
        laid_out.commit_step,
        laid_out.tag_gap
    );
}

// ---------------------------------------------------------------------------
// Merges (AC-C11)
// ---------------------------------------------------------------------------

/// S1's labeled-merge case: Merman keeps both parents, so nothing repairs it.
#[test]
fn a_labeled_merge_keeps_both_parents() {
    let laid_out = geometry(
        &gitgraph(&[
            "commit id: \"A\"",
            "branch side",
            "checkout side",
            "commit id: \"S1\"",
            "commit id: \"S2\"",
            "checkout main",
            "commit id: \"B\"",
            "merge side id: \"M\" tag: \"origin/main\"",
            "commit id: \"C\"",
        ]),
        MermaidTheme::Default,
    );
    let merge = laid_out.commit("M").expect("the merge commit");
    assert_eq!(merge.parents, ["B", "S2"]);
    assert_eq!(merge.lane, "main");
    assert_eq!(
        merge.tags.iter().map(|tag| tag.text.as_str()).collect::<Vec<_>>(),
        ["origin/main"]
    );
    assert_eq!(laid_out.commit("C").unwrap().parents, ["M"]);
    assert_eq!(laid_out.commit("S2").unwrap().lane, "side");
}

/// The input S1 found Mermaid.js and Merman both accept, which the removed
/// merge repair rejected as an "empty branch": `side` has no commit of its
/// own, so the merge's second parent is `side`'s head, `A`.
#[test]
#[serial]
fn a_merge_of_a_branch_without_its_own_commit_renders() {
    let _cache = isolated_cache_dir();
    let text = gitgraph(&[
        "commit id: \"A\"",
        "branch side",
        "checkout main",
        "commit id: \"B\"",
        "merge side id: \"M\"",
    ]);
    let diagram = MermaidDiagram::new(text.as_str());

    let laid_out = diagram.gitgraph_geometry().unwrap().expect("a gitGraph");
    let artifact = diagram
        .render(&crate::artifact::RenderRequest::default())
        .expect("the merge renders");

    assert_eq!(laid_out.commit("M").unwrap().parents, ["B", "A"]);
    assert!(artifact.path.exists());
}

#[test]
fn geometry_is_none_for_other_diagrams() {
    assert_eq!(
        MermaidDiagram::new("flowchart LR\n    A --> B")
            .gitgraph_geometry()
            .unwrap(),
        None
    );
}

// ---------------------------------------------------------------------------
// Tag spacing (AC-C12)
// ---------------------------------------------------------------------------

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
    assert_spaced(
        &text,
        &[("a2", ALPHA), ("a3", BETA), ("a4", "PR #104 → main")],
    );
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
    let text = gitgraph(&[
        "commit id: \"d1\"",
        "commit id: \"d2\"",
        &format!("commit id: \"d3\" tag: \"main\" tag: \"origin/main\" tag: \"{ALPHA}\""),
        "commit id: \"d4\"",
    ]);
    assert_eq!(written_spacing(&text), None);
    for theme in THEMES {
        let laid_out = geometry(&text, theme);
        assert_eq!(laid_out.commit_step, default_gitgraph_commit_step(), "{theme:?}");
        assert_eq!(
            laid_out.tag_overlaps(),
            Vec::<(String, String)>::new(),
            "{theme:?}"
        );
    }
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
    let (main_tag, lane_tag) = (
        control.commit("d2").unwrap().tags[0].bounds,
        control.commit("t1").unwrap().tags[0].bounds,
    );
    assert!(
        main_tag.x1 < lane_tag.x2 && lane_tag.x1 < main_tag.x2,
        "control: horizontal overlap {main_tag:?} {lane_tag:?}"
    );
    assert_eq!(
        control.commit("t1").unwrap().index,
        control.commit("d2").unwrap().index + 1
    );
    assert_eq!(written_spacing(&text), None);
    for theme in THEMES {
        let laid_out = geometry(&text, theme);
        assert_eq!(laid_out.commit_step, default_gitgraph_commit_step(), "{theme:?}");
        assert!(governed_clearances(&laid_out).is_empty(), "{theme:?}");
        assert_eq!(
            laid_out.tag_overlaps(),
            Vec::<(String, String)>::new(),
            "{theme:?}"
        );
    }
}

#[test]
fn the_observed_graph_keeps_the_default_step() {
    assert_eq!(written_spacing(OBSERVED), None);
    for theme in THEMES {
        let laid_out = geometry(OBSERVED, theme);
        assert_eq!(laid_out.commit_step, default_gitgraph_commit_step(), "{theme:?}");
        assert_eq!(
            laid_out.tag_overlaps(),
            Vec::<(String, String)>::new(),
            "{theme:?}"
        );
    }
    let mut tagged: Vec<_> = geometry(OBSERVED, MermaidTheme::Default)
        .tag_boxes()
        .into_iter()
        .map(|(id, tag, _)| (id.to_string(), tag.to_string()))
        .collect();
    tagged.sort();
    assert_eq!(
        tagged,
        [
            ("999032d".to_string(), "main".to_string()),
            ("999032d".to_string(), "origin/main".to_string())
        ]
    );
}

#[test]
fn the_gap_scales_with_an_overridden_font_size() {
    let body = gitgraph(&[
        "commit id: \"d1\"",
        "commit id: \"d2\" tag: \"main\"",
        "commit id: \"d3\" tag: \"origin/main\"",
    ]);
    let text = format!("%%{{init: {{'themeVariables': {{'fontSize': 24}}}}}}%%\n{body}");

    let default_font = geometry(&body, MermaidTheme::Default);
    let large_font = geometry(&text, MermaidTheme::Default);

    assert_eq!(default_font.tag_gap, 16.0);
    assert_eq!(large_font.tag_gap, 24.0);
    assert_minimal(&default_font, "font 16");
    assert_minimal(&large_font, "font 24");
    // Tag widths follow the tag font, not the theme's, so the larger gap
    // alone widens the step by its 8 px (to the 0.01 px rounding).
    assert!(
        (large_font.commit_step - default_font.commit_step - 8.0).abs() <= TOLERANCE,
        "{} vs {}",
        large_font.commit_step,
        default_font.commit_step
    );
}

#[test]
fn a_diagram_without_tags_keeps_the_default_step() {
    let text = gitgraph(&[
        "commit id: \"a1\"",
        "branch side",
        "checkout side",
        "commit id: \"s1\"",
        "checkout main",
        "merge side id: \"M\"",
    ]);
    let laid_out = geometry(&text, MermaidTheme::Default);
    assert_eq!(
        MermaidDiagram::new(text.as_str()).rewritten_source().unwrap(),
        text
    );
    assert_eq!(laid_out.commit_step, default_gitgraph_commit_step());
}

#[test]
fn a_vertical_graph_with_tags_keeps_the_default_step() {
    // Merman rotates top-to-bottom and bottom-to-top tags; those graphs are
    // not spaced (documented limitation).
    for direction in ["TB", "BT"] {
        let text = format!(
            "gitGraph {direction}:\n    commit id: \"a1\" tag: \"main\"\n    commit id: \"a2\" tag: \"origin/main\""
        );
        assert_eq!(written_spacing(&text), None, "{direction}");
        assert_eq!(
            geometry(&text, MermaidTheme::Default).commit_step,
            default_gitgraph_commit_step(),
            "{direction}"
        );
    }
    let horizontal =
        "gitGraph LR:\n    commit id: \"a1\" tag: \"main\"\n    commit id: \"a2\" tag: \"origin/main\"";
    assert!(geometry(horizontal, MermaidTheme::Default).commit_step > default_gitgraph_commit_step());
}

#[test]
fn the_default_step_is_mermans_fifty() {
    assert_eq!(default_gitgraph_commit_step(), 50.0);
}

#[test]
#[serial]
fn the_spaced_layout_is_the_measured_one() {
    let _cache = isolated_cache_dir();
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

// ---------------------------------------------------------------------------
// Tag font size as Merman reads it
// ---------------------------------------------------------------------------

/// Two long tags on neighboring commits, so spacing is written, with the
/// tag font size an author sets in `%%{init}%%` (`themeVariables` JSON).
fn sized_tags(variables: &str) -> String {
    let body = gitgraph(&[
        "commit id: \"a1\" tag: \"release-candidate-one\"",
        "commit id: \"a2\" tag: \"release-candidate-two\"",
    ]);
    if variables.is_empty() {
        return body;
    }
    format!("%%{{init: {{\"themeVariables\": {{{variables}}}}}}}%%\n{body}")
}

/// The tag bounds the geometry reports, and that spacing was computed from,
/// are the polygons Merman draws in the rendered SVG, and those polygons do
/// not overlap: the tags were measured at the size Merman draws them.
fn assert_tags_measured_as_drawn(text: &str) {
    let _cache = isolated_cache_dir();
    let diagram = MermaidDiagram::new(text).with_theme(MermaidTheme::Default);
    let laid_out = diagram
        .gitgraph_geometry()
        .unwrap_or_else(|error| panic!("{error}: {text}"))
        .expect("a gitGraph");
    let request = crate::artifact::RenderRequest {
        format: crate::artifact::OutputFormat::Svg,
        ..crate::artifact::RenderRequest::default()
    };
    let artifact = diagram.render(&request).expect("the gitGraph renders");
    let svg = std::fs::read_to_string(&artifact.path).expect("the SVG artifact");
    let drawn = super::gitgraph_layout_tests::drawn_tags(&svg);

    let ours = laid_out.tag_boxes();
    assert_eq!(ours.len(), drawn.len(), "ours {ours:?}, drawn {drawn:?}: {text}");
    for (_, tag, bounds) in &ours {
        let (_, drawn) = drawn
            .iter()
            .find(|(drawn_text, _)| drawn_text == tag)
            .expect("a drawn tag");
        assert!(
            (bounds.x2 - bounds.x1 - (drawn.x2 - drawn.x1)).abs() < TOLERANCE
                && (bounds.y2 - bounds.y1 - (drawn.y2 - drawn.y1)).abs() < TOLERANCE,
            "{tag}: measured {bounds:?}, drawn {drawn:?}: {text}"
        );
    }
    let (first, second) = (drawn[0].1, drawn[1].1);
    let (earlier, later) = if first.x1 <= second.x1 { (first, second) } else { (second, first) };
    assert!(
        later.x1 - earlier.x2 >= laid_out.tag_gap - TOLERANCE,
        "drawn tags {earlier:?} and {later:?} are closer than the {} px gap: {text}",
        laid_out.tag_gap
    );
}

macro_rules! tag_font_size_cases {
    ($($name:ident => $variables:expr,)+) => {
        $(
            #[test]
            #[serial]
            fn $name() {
                assert_tags_measured_as_drawn(&sized_tags($variables));
            }
        )+
    };
}

tag_font_size_cases! {
    tag_font_size_absent_is_measured_as_drawn => "",
    tag_font_size_in_px_is_measured_as_drawn => r#""tagLabelFontSize": "14px""#,
    tag_font_size_in_percent_is_measured_as_drawn => r#""tagLabelFontSize": "150%""#,
    tag_font_size_in_em_is_measured_as_drawn => r#""tagLabelFontSize": "1.5em""#,
    tag_font_size_relative_to_an_overridden_base_is_measured_as_drawn =>
        r#""fontSize": 20, "tagLabelFontSize": "120%""#,
    tag_font_size_inherit_is_measured_as_drawn => r#""tagLabelFontSize": "inherit""#,
    tag_font_size_important_is_measured_as_drawn => r#""tagLabelFontSize": "14px !important""#,
    tag_font_size_as_a_json_number_is_measured_as_drawn => r#""tagLabelFontSize": 14"#,
    tag_font_size_unitless_string_is_measured_as_drawn => r#""tagLabelFontSize": "14""#,
    tag_font_size_not_a_size_is_measured_as_drawn => r#""tagLabelFontSize": "large""#,
    tag_font_size_unset_is_measured_as_drawn => r#""tagLabelFontSize": "unset""#,
    tag_font_size_zero_is_measured_as_drawn => r#""tagLabelFontSize": "0""#,
}
