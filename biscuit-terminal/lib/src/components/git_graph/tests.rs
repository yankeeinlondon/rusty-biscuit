use super::*;
use crate::discovery::detection::ImageSupport;

/// A full 40-character SHA whose display ID is `short`.
fn sha(short: &str) -> String {
    format!("{short:0<40}")
}

fn commits(shorts: &[&str]) -> Vec<LaneEntry> {
    shorts.iter().map(|short| LaneEntry::Commit(sha(short))).collect()
}

/// The spec's example graph, standing in `feat-dark-fixes` (forked from
/// `feat/theme`); local `main` is behind `origin/main`.
fn spec_example() -> GitGraph {
    let mut default_entries = commits(&["4c2e4d5", "2596f1d"]);
    default_entries.push(LaneEntry::Elided(4));
    default_entries.extend(commits(&["d44f301", "e55a412", "f66b523"]));
    GitGraph::new("main", default_entries)
        .with_ref("main", sha("2596f1d"))
        .with_ref("origin/main", sha("f66b523"))
        .with_line(
            GraphLine::new("feat/theme")
                .forked_at(sha("2596f1d"))
                .with_entries(commits(&["7a1b2c3", "8b2c3d4", "1e5f6a7"]))
                .with_created_at(100),
        )
        .with_line(
            GraphLine::new("feat/dark-fixes")
                .with_parent("feat/theme")
                .forked_at(sha("8b2c3d4"))
                .with_entries(commits(&["9c3d4e5", "0d4e5f6"]))
                .with_created_at(200),
        )
        .with_pull_request(GraphPullRequest {
            number: 104,
            source_branch: "feat/dark-fixes".into(),
            target_branch: "feat/theme".into(),
        })
        .with_current_branch("feat/dark-fixes")
}

const SPEC_EXAMPLE: &str = r#"gitGraph
    commit id: "4c2e4d5"
    commit id: "2596f1d" tag: "main"
    branch feat/theme
    checkout feat/theme
    commit id: "7a1b2c3"
    commit id: "8b2c3d4"
    branch feat/dark-fixes
    checkout feat/dark-fixes
    commit id: "9c3d4e5"
    commit id: "0d4e5f6" tag: "PR #104 → feat/theme"
    checkout feat/theme
    commit id: "1e5f6a7"
    checkout main
    commit id: "+4" type: HIGHLIGHT
    commit id: "d44f301"
    commit id: "e55a412"
    commit id: "f66b523" tag: "origin/main""#;

fn mermaid(graph: &GitGraph) -> String {
    graph.mermaid().expect("the graph has a default commit")
}

// ---------------------------------------------------------------------------
// Lane/tag rule and emitted text
// ---------------------------------------------------------------------------

#[test]
fn the_spec_example_emits_the_spec_text() {
    assert_eq!(mermaid(&spec_example()), SPEC_EXAMPLE);
}

#[test]
fn branch_and_checkout_statements_carry_no_attributes() {
    let text = mermaid(&spec_example());
    for line in text.lines().map(str::trim) {
        if let Some(name) = line.strip_prefix("branch ").or_else(|| line.strip_prefix("checkout ")) {
            assert!(!name.contains(':') && !name.contains(' '), "attribute on `{line}`");
        }
        assert!(!line.starts_with("merge"), "no merge statements: `{line}`");
    }
}

#[test]
fn a_focused_view_draws_no_lane_for_an_unrelated_branch() {
    let graph = spec_example().with_line(
        GraphLine::new("feat/other")
            .forked_at(sha("4c2e4d5"))
            .with_entries(commits(&["aaaaaaa"])),
    );
    let text = mermaid(&graph);
    assert!(!text.contains("feat/other"), "{text}");
    assert!(!text.contains("aaaaaaa"), "{text}");
}

#[test]
fn a_branch_already_in_the_default_branch_is_a_tag() {
    let graph = spec_example().with_line(GraphLine::new("chore/old-cleanup").forked_at(sha("4c2e4d5")));
    let text = mermaid(&graph);
    assert!(text.contains("commit id: \"4c2e4d5\" tag: \"chore/old-cleanup\""), "{text}");
    assert!(!text.contains("branch chore/old-cleanup"), "{text}");
}

#[test]
fn the_base_view_gives_every_branch_with_commits_a_lane() {
    let graph = GitGraph::new("main", commits(&["1111111", "2222222"]))
        .with_ref("main", sha("2222222"))
        .with_line(GraphLine::new("feat/a").forked_at(sha("1111111")).with_entries(commits(&["aaaaaaa"])))
        .with_line(GraphLine::new("feat/b").forked_at(sha("2222222")).with_entries(commits(&["bbbbbbb"])))
        .with_line(GraphLine::new("merged").forked_at(sha("2222222")))
        .with_current_branch("main");
    let text = mermaid(&graph);
    assert!(text.contains("branch feat/a") && text.contains("branch feat/b"), "{text}");
    assert!(text.contains("commit id: \"2222222\" tag: \"main\" tag: \"merged\""), "{text}");
    // No current branch is the base view too.
    let unset = GitGraph::new("main", commits(&["1111111"]))
        .with_line(GraphLine::new("feat/a").forked_at(sha("1111111")).with_entries(commits(&["aaaaaaa"])));
    assert!(mermaid(&unset).contains("branch feat/a"));
}

#[test]
fn origin_default_is_a_tag_until_it_diverges() {
    let in_sync = GitGraph::new("main", commits(&["1111111"]))
        .with_ref("main", sha("1111111"))
        .with_ref("origin/main", sha("1111111"));
    assert!(mermaid(&in_sync).contains("commit id: \"1111111\" tag: \"main\" tag: \"origin/main\""));

    let diverged = GitGraph::new("main", commits(&["1111111", "2222222"]))
        .with_ref("main", sha("2222222"))
        .with_ref("origin/main", sha("3333333"))
        .with_line(
            GraphLine::new("origin/main")
                .forked_at(sha("1111111"))
                .with_entries(commits(&["3333333"])),
        )
        .with_line(GraphLine::new("feat/x").forked_at(sha("1111111")).with_entries(commits(&["4444444"])))
        .with_current_branch("feat/x");
    let text = mermaid(&diverged);
    assert!(text.contains("branch origin/main"), "{text}");
    // Its lane label names its tip, so it is not tagged again.
    assert!(!text.contains("tag: \"origin/main\""), "{text}");
    assert!(text.contains("commit id: \"2222222\" tag: \"main\""), "{text}");
}

#[test]
fn a_default_branch_not_named_main_is_the_root_lane_and_a_tag() {
    let graph = GitGraph::new("master", commits(&["1111111", "2222222"]))
        .with_ref("master", sha("2222222"))
        .with_line(GraphLine::new("main").forked_at(sha("1111111")).with_entries(commits(&["aaaaaaa"])))
        .with_current_branch("main");
    let text = mermaid(&graph);
    assert!(text.contains("commit id: \"2222222\" tag: \"master\""), "{text}");
    // A non-default branch named `main` must not merge into the root lane.
    assert!(text.contains("branch main~\n    checkout main~"), "{text}");
    assert!(text.contains("checkout main\n    commit id: \"2222222\""), "{text}");
    assert!(!text.contains("master\n") && !text.contains("branch master"), "{text}");
}

#[test]
fn pull_requests_tag_their_source_tip_and_skip_undrawn_branches() {
    let graph = spec_example()
        .with_pull_request(GraphPullRequest {
            number: 99,
            source_branch: "feat/theme".into(),
            target_branch: "main".into(),
        })
        .with_pull_request(GraphPullRequest {
            number: 7,
            source_branch: "not/drawn".into(),
            target_branch: "main".into(),
        });
    let text = mermaid(&graph);
    assert!(text.contains("commit id: \"1e5f6a7\" tag: \"PR #99 → main\""), "{text}");
    assert!(!text.contains("PR #7"), "{text}");
}

#[test]
fn lanes_forking_at_one_commit_follow_creation_order() {
    let graph = GitGraph::new("main", commits(&["1111111"]))
        .with_line(
            GraphLine::new("late")
                .forked_at(sha("1111111"))
                .with_entries(commits(&["bbbbbbb"]))
                .with_created_at(200),
        )
        .with_line(
            GraphLine::new("undated")
                .forked_at(sha("1111111"))
                .with_entries(commits(&["ccccccc"])),
        )
        .with_line(
            GraphLine::new("early")
                .forked_at(sha("1111111"))
                .with_entries(commits(&["aaaaaaa"]))
                .with_created_at(100),
        );
    let text = mermaid(&graph);
    let early = text.find("branch early").unwrap();
    let late = text.find("branch late").unwrap();
    let undated = text.find("branch undated").unwrap();
    assert!(early < late && late < undated, "{text}");
}

#[test]
fn a_fork_point_outside_the_drawn_commits_hangs_from_the_lane_start() {
    let graph = GitGraph::new("main", commits(&["1111111", "2222222"]))
        .with_line(GraphLine::new("feat/old").forked_at(sha("0ld0000")).with_entries(commits(&["aaaaaaa"])));
    let text = mermaid(&graph);
    assert!(
        text.contains("commit id: \"1111111\"\n    branch feat/old"),
        "anchors after the first drawn commit: {text}"
    );
}

#[test]
fn ids_are_unique_even_when_short_shas_or_elisions_repeat() {
    let graph = GitGraph::new(
        "main",
        vec![
            LaneEntry::Commit(format!("abcdef1{}", "1".repeat(33))),
            LaneEntry::Elided(2),
            LaneEntry::Commit(format!("abcdef1{}", "2".repeat(33))),
        ],
    )
    .with_line(
        GraphLine::new("feat/x")
            .forked_at(format!("abcdef1{}", "1".repeat(33)))
            .with_entries(vec![LaneEntry::Elided(2), LaneEntry::Commit(sha("aaaaaaa"))]),
    );
    let text = mermaid(&graph);
    assert!(text.contains("commit id: \"abcdef11\""), "{text}");
    assert!(text.contains("commit id: \"abcdef12\""), "{text}");
    assert!(text.contains("commit id: \"+2\" type: HIGHLIGHT"), "{text}");
    assert!(text.contains("commit id: \"+2 \" type: HIGHLIGHT"), "{text}");
}

#[test]
fn quotes_in_ref_names_cannot_break_a_tag() {
    let graph = GitGraph::new("main", commits(&["1111111"])).with_ref("say\"hi", sha("1111111"));
    assert!(mermaid(&graph).contains("tag: \"say'hi\""));
}

#[test]
fn a_default_lane_without_commits_draws_nothing() {
    let graph = GitGraph::new("main", vec![LaneEntry::Elided(3)]);
    assert_eq!(graph.mermaid(), None);
    assert_eq!(graph.plan(viewport(200, 50)), None);
    let term = plain_terminal(120, 40);
    assert_eq!(graph.render(&term), "");
    assert!(graph.try_render(&term).is_err());
}

#[test]
fn every_emitted_graph_parses() {
    for graph in [spec_example(), spec_example().with_current_branch("main")] {
        let text = mermaid(&graph);
        biscuit_visualized::mermaid::MermaidDiagram::new(text.as_str())
            .natural_size()
            .unwrap_or_else(|error| panic!("{error}: {text}"));
    }
}

// ---------------------------------------------------------------------------
// Fitting (injected measurement: 20 units per `commit` line, 80 per lane)
// ---------------------------------------------------------------------------

fn viewport(columns: u32, rows: u32) -> GraphViewport {
    GraphViewport {
        columns,
        rows,
        cell: CellSize::FALLBACK,
    }
}

fn fake_measure(text: &str) -> Option<NaturalSize> {
    let commits = text.lines().filter(|line| line.trim_start().starts_with("commit")).count();
    let lanes = 1 + text.lines().filter(|line| line.trim_start().starts_with("branch ")).count();
    Some(NaturalSize {
        width: 20.0 * commits as f32,
        height: 80.0 * lanes as f32,
    })
}

fn plan(graph: &GitGraph, viewport: GraphViewport) -> GitGraphPlan {
    graph.plan_with(viewport, &fake_measure).expect("a plan")
}

#[test]
fn a_graph_that_fits_is_not_trimmed_and_sizes_from_the_scale() {
    // 11 commit lines × 20 units × 1.25 ÷ 8 px = 34.4 → 35 columns;
    // 3 lanes × 80 units × 1.25 ÷ 16 = 18.75 → 19 rows.
    let planned = plan(&spec_example(), viewport(120, 40));
    assert_eq!(planned.mermaid, SPEC_EXAMPLE);
    assert_eq!((planned.columns, planned.rows), (35, 19));
    assert_eq!((planned.hidden_lanes, planned.trimmed_commits), (0, 0));
}

#[test]
fn the_width_cap_trims_commits_before_anything_shrinks() {
    // 30 columns fit 9 commit lines (9 × 20 × 1.25 ÷ 8 = 28.1).
    let planned = plan(&spec_example(), viewport(30, 40));
    assert_eq!(planned.trimmed_commits, 2);
    assert!(planned.columns <= 30, "{planned:?}");
    // The lane showing the most commits loses its oldest unpinned commits:
    // `d44f301` and `e55a412` fold into the existing `+4`.
    let text = &planned.mermaid;
    assert!(text.contains("commit id: \"+6\" type: HIGHLIGHT\n    commit id: \"f66b523\""), "{text}");
    // Tips, fork points, and tagged commits stay.
    for kept in ["2596f1d", "8b2c3d4", "0d4e5f6", "1e5f6a7", "f66b523"] {
        assert!(text.contains(kept), "{kept} kept: {text}");
    }
}

#[test]
fn trimming_stops_when_only_pinned_commits_remain() {
    let planned = plan(&spec_example(), viewport(5, 40));
    let text = &planned.mermaid;
    // Every unpinned commit is gone: the two beside `+4` first, then one lone
    // commit per lane, each becoming its own `+1`.
    assert_eq!(planned.trimmed_commits, 5);
    for trimmed in ["4c2e4d5", "d44f301", "e55a412", "7a1b2c3", "9c3d4e5"] {
        assert!(!text.contains(trimmed), "{trimmed} trimmed: {text}");
    }
    // Lane tips, fork points, and tagged commits stay.
    for kept in ["2596f1d", "8b2c3d4", "0d4e5f6", "1e5f6a7", "f66b523"] {
        assert!(text.contains(kept), "{kept} kept: {text}");
    }
    assert!(text.contains("commit id: \"+6\" type: HIGHLIGHT"), "{text}");
    assert!(text.contains("commit id: \"+1\" type: HIGHLIGHT"), "{text}");
    assert!(text.contains("commit id: \"+1 \" type: HIGHLIGHT"), "{text}");
    // Still wider than the viewport, so the image will shrink.
    assert!(planned.columns > 5, "{planned:?}");
    assert!(
        biscuit_visualized::mermaid::MermaidDiagram::new(text.as_str())
            .natural_size()
            .is_ok(),
        "{text}"
    );
}

#[test]
fn an_explicit_width_is_never_trimmed_to() {
    let planned = plan(&spec_example().with_width(ImageWidth::Characters(20)), viewport(30, 40));
    assert_eq!(planned.trimmed_commits, 0);
    assert_eq!(planned.columns, 20);
    assert_eq!(planned.mermaid, SPEC_EXAMPLE);
}

#[test]
fn a_scale_width_replaces_the_scale() {
    // At 100%: 11 × 20 ÷ 8 = 27.5 → 28 columns, 3 × 80 ÷ 16 = 15 rows.
    let planned = plan(&spec_example().with_width(ImageWidth::Scale(1.0)), viewport(120, 40));
    assert_eq!((planned.columns, planned.rows), (28, 15));
}

fn base_view_with_lanes() -> GitGraph {
    // Five worktree lanes off `main`; `feat/child` forks from `feat/parent`.
    GitGraph::new("main", commits(&["1111111"]))
        .with_ref("main", sha("1111111"))
        .with_line(line("feat/a", None, "aaaaaaa", 10))
        .with_line(line("feat/b", None, "bbbbbbb", 50))
        .with_line(line("feat/parent", None, "ppppppp", 5))
        .with_line(line("feat/child", Some(("feat/parent", "ppppppp")), "ccccccc", 90))
        .with_line(line("feat/e", None, "eeeeeee", 30))
        .with_current_branch("main")
}

fn line(branch: &str, parent: Option<(&str, &str)>, commit: &str, last_active: i64) -> GraphLine {
    let (parent, fork) = parent.unwrap_or(("main", "1111111"));
    GraphLine::new(branch)
        .with_parent(parent)
        .forked_at(sha(fork))
        .with_entries(commits(&[commit]))
        .with_last_active(last_active)
}

#[test]
fn the_base_view_height_cap_keeps_the_most_recently_active_lanes() {
    // Half of 40 rows is 20; each lane is 80 × 1.25 ÷ 16 = 6.25 rows, so the
    // default lane plus two more fit (18.75) and a fourth does not (25).
    let planned = plan(&base_view_with_lanes(), viewport(200, 40));
    let text = &planned.mermaid;
    // `feat/child` is the most recent and brings its parent.
    assert!(text.contains("branch feat/parent") && text.contains("branch feat/child"), "{text}");
    for hidden in ["feat/a", "feat/b", "feat/e"] {
        assert!(!text.contains(hidden), "{hidden} hidden: {text}");
    }
    assert_eq!(planned.hidden_lanes, 3);
    assert_eq!(planned.rows, 19);
}

#[test]
fn the_height_cap_adds_lanes_in_activity_order_until_one_does_not_fit() {
    // 60 rows: cap 30 fits four lanes in total (25 rows).
    let planned = plan(&base_view_with_lanes(), viewport(200, 60));
    let text = &planned.mermaid;
    assert!(text.contains("branch feat/child") && text.contains("branch feat/b"), "{text}");
    assert!(!text.contains("feat/e") && !text.contains("feat/a"), "{text}");
    assert_eq!(planned.hidden_lanes, 2);
}

#[test]
fn a_focused_view_is_never_cut_by_the_height_cap() {
    let planned = plan(&spec_example(), viewport(200, 10));
    assert_eq!(planned.hidden_lanes, 0);
    assert_eq!(planned.mermaid, SPEC_EXAMPLE);
}

#[test]
fn a_failed_measurement_trims_nothing() {
    let planned = spec_example()
        .plan_with(viewport(5, 5), &|_| None)
        .expect("a plan");
    assert_eq!(planned.mermaid, SPEC_EXAMPLE);
    assert_eq!((planned.hidden_lanes, planned.trimmed_commits), (0, 0));
}

#[test]
fn the_viewport_comes_from_the_terminal_after_margins() {
    let term = Terminal::builder()
        .width(100)
        .height(30)
        .cell_size(CellSize {
            width: 10,
            height: 20,
        })
        .build();
    let mut layout = Layout::default();
    layout.margin.left = crate::utils::layout::TargetValue::universal(crate::utils::layout::Length::ch(4));
    let viewport = GraphViewport::for_terminal(&term, &layout);
    assert_eq!(viewport.columns, 96);
    assert_eq!(viewport.rows, 30);
    assert_eq!(viewport.cell.height, 20);
}

// ---------------------------------------------------------------------------
// Renderer measurement (sizes depend on the host's fonts, so only relations)
// ---------------------------------------------------------------------------

#[test]
fn measured_sizes_trim_to_the_width_cap() {
    let graph = spec_example().with_theme(MermaidTheme::Default);
    let wide = graph.plan(viewport(400, 60)).unwrap();
    assert_eq!(wide.trimmed_commits, 0);
    let size = biscuit_visualized::mermaid::MermaidDiagram::new(wide.mermaid.as_str())
        .with_theme(MermaidTheme::Default)
        .natural_size()
        .unwrap();
    assert_eq!(wide.columns, ImageWidth::scaled_columns(1.25, size.width, CellSize::FALLBACK));
    assert_eq!(wide.rows, rows_for(1.25, size.height));

    let narrow_columns = wide.columns - 10;
    let narrow = graph.plan(viewport(narrow_columns, 60)).unwrap();
    assert!(narrow.trimmed_commits > 0, "{narrow:?}");
    assert!(narrow.columns <= narrow_columns, "{narrow:?}");
    assert!(narrow.mermaid.contains("type: HIGHLIGHT"));
}

// ---------------------------------------------------------------------------
// Rendering
// ---------------------------------------------------------------------------

fn plain_terminal(width: u32, height: u32) -> Terminal {
    Terminal::builder()
        .width(width)
        .height(height)
        .is_tty(false)
        .image_support(ImageSupport::None)
        .build()
}

#[test]
fn without_image_support_the_terminal_gets_the_code_block_and_the_lane_note() {
    let term = plain_terminal(200, 24);
    let graph = base_view_with_lanes().with_theme(MermaidTheme::Default);
    let output = graph.render(&term);
    assert!(output.contains("```mermaid"), "{output}");
    assert!(output.contains("gitGraph"), "{output}");
    let planned = graph.plan(GraphViewport::for_terminal(&term, &Layout::default())).unwrap();
    assert!(planned.hidden_lanes > 0, "24 rows cap the five-lane base view: {planned:?}");
    let noun = if planned.hidden_lanes == 1 { "worktree" } else { "worktrees" };
    assert!(output.contains(&format!("{} more {noun} not shown", planned.hidden_lanes)), "{output}");
    assert!(graph.try_render(&term).is_err(), "no image support is an error on the fallible path");
}

#[test]
fn the_tree_projection_carries_the_untrimmed_source() {
    let node = spec_example().render_tree();
    let debug = format!("{node:?}");
    assert!(debug.contains("gitGraph") && debug.contains("feat/dark-fixes"), "{debug}");
}

#[test]
fn the_browser_output_is_an_svg_island() {
    let fragment = spec_example().render_html_fragment();
    let html = fragment.render();
    assert!(html.contains("<svg"), "{html}");
}
