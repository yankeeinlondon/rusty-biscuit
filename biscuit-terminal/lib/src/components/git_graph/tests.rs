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
    // A line without commits is labeled at its own tip (was: its fork).
    let graph = spec_example().with_line(
        GraphLine::new("chore/old-cleanup")
            .forked_at(sha("4c2e4d5"))
            .with_tip(sha("4c2e4d5")),
    );
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
        .with_line(GraphLine::new("merged").forked_at(sha("1111111")).with_tip(sha("2222222")))
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
fn a_fork_point_outside_the_drawn_commits_is_drawn_unconnected() {
    // No substitute attachment (was: hung from the default lane's start).
    let graph = GitGraph::new("main", commits(&["1111111", "2222222"]))
        .with_line(GraphLine::new("feat/old").forked_at(sha("0ld0000")).with_entries(commits(&["aaaaaaa"])));
    let text = mermaid(&graph);
    assert_eq!(
        text,
        r#"gitGraph
    branch feat/old
    checkout main
    commit id: "1111111"
    checkout feat/old
    commit id: "aaaaaaa"
    checkout main
    commit id: "2222222""#
    );
    assert_eq!(parents_of(&text, "aaaaaaa"), Vec::<String>::new());
    assert!(plan(&graph, viewport(200, 40)).incomplete);
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

// ---------------------------------------------------------------------------
// Merges, explicit tips, and accounting for undrawn history
// ---------------------------------------------------------------------------

fn geometry_of(text: &str) -> biscuit_visualized::mermaid::GitGraphGeometry {
    biscuit_visualized::mermaid::MermaidDiagram::new(text)
        .gitgraph_geometry()
        .unwrap_or_else(|error| panic!("{error}: {text}"))
        .expect("a gitGraph")
}

/// Parent IDs of `id` as the renderer lays the text out (merge repair included).
fn parents_of(text: &str, id: &str) -> Vec<String> {
    geometry_of(text)
        .commit(id)
        .unwrap_or_else(|| panic!("{id} drawn: {text}"))
        .parents
        .clone()
}

fn lane_of_commit(text: &str, id: &str) -> String {
    geometry_of(text)
        .commit(id)
        .unwrap_or_else(|| panic!("{id} drawn: {text}"))
        .lane
        .clone()
}

/// Observation 1: `feat/x` merged into `main` at `mmmmmmm` (tagged `origin/main`).
fn merged_into_default() -> GitGraph {
    GitGraph::new("main", commits(&["1111111", "2222222", "mmmmmmm", "3333333"]))
        .with_ref("main", sha("3333333"))
        .with_ref("origin/main", sha("mmmmmmm"))
        .with_line(
            GraphLine::new("feat/x")
                .forked_at(sha("1111111"))
                .with_tip(sha("bbbbbbb"))
                .with_entries(commits(&["aaaaaaa", "bbbbbbb"]))
                .with_merge(sha("bbbbbbb"), sha("mmmmmmm")),
        )
        .with_current_branch("feat/x")
}

#[test]
fn a_lane_merged_into_the_default_lane_is_drawn_merging_at_its_commit() {
    let text = mermaid(&merged_into_default());
    assert_eq!(
        text,
        r#"gitGraph
    commit id: "1111111"
    branch feat/x
    checkout feat/x
    commit id: "aaaaaaa"
    commit id: "bbbbbbb"
    checkout main
    commit id: "2222222"
    merge feat/x id: "mmmmmmm" tag: "origin/main"
    commit id: "3333333" tag: "main""#
    );
    assert_eq!(parents_of(&text, "mmmmmmm"), ["2222222", "bbbbbbb"]);
    assert_eq!(lane_of_commit(&text, "mmmmmmm"), "main");
    assert_eq!(parents_of(&text, "3333333"), ["mmmmmmm"]);
    assert!(!plan(&merged_into_default(), viewport(200, 40)).incomplete);
}

#[test]
fn with_merge_appends_edges_oldest_first() {
    let line = GraphLine::new("feat/x")
        .with_merge(sha("b1b1b1b"), sha("c1c1c1c"))
        .with_merge(sha("b2b2b2b"), sha("c2c2c2c"));
    assert_eq!(
        line.merges,
        [
            LaneMerge { source: sha("b1b1b1b"), destination: sha("c1c1c1c") },
            LaneMerge { source: sha("b2b2b2b"), destination: sha("c2c2c2c") },
        ]
    );
}

#[test]
fn a_lane_merged_into_its_recorded_parent_merges_on_the_parent_lane() {
    let graph = GitGraph::new("main", commits(&["1111111"]))
        .with_ref("main", sha("1111111"))
        .with_line(
            GraphLine::new("feat/parent")
                .forked_at(sha("1111111"))
                .with_entries(commits(&["ppppppp", "qqqqqqq", "rrrrrrr"])),
        )
        .with_line(
            GraphLine::new("feat/child")
                .with_parent("feat/parent")
                .forked_at(sha("ppppppp"))
                .with_tip(sha("ccccccc"))
                .with_entries(commits(&["ccccccc"]))
                .with_merge(sha("ccccccc"), sha("qqqqqqq")),
        )
        .with_current_branch("feat/child");
    let text = mermaid(&graph);
    assert!(text.contains(r#"    checkout feat/parent
    merge feat/child id: "qqqqqqq""#), "{text}");
    assert_eq!(parents_of(&text, "qqqqqqq"), ["ppppppp", "ccccccc"]);
    assert_eq!(lane_of_commit(&text, "qqqqqqq"), "feat/parent");
}

/// Observation 2 (base view): `fix/sniff` forks from `fix/wt-ux`'s tip, both
/// merge into `main`, and local `main` is one merge behind `origin/main`.
fn nested_merges() -> GitGraph {
    GitGraph::new("main", commits(&["d1d1d1d", "m103000", "m104000"]))
        .with_ref("main", sha("m103000"))
        .with_ref("origin/main", sha("m104000"))
        .with_line(
            GraphLine::new("fix/wt-ux")
                .forked_at(sha("d1d1d1d"))
                .with_tip(sha("w2w2w2w"))
                .with_entries(commits(&["w1w1w1w", "w2w2w2w"]))
                .with_merge(sha("w2w2w2w"), sha("m103000"))
                .with_last_active(10),
        )
        .with_line(
            GraphLine::new("fix/sniff")
                .with_parent("fix/wt-ux")
                .forked_at(sha("w2w2w2w"))
                .with_tip(sha("s1s1s1s"))
                .with_entries(vec![LaneEntry::Elided(96), LaneEntry::Commit(sha("s1s1s1s"))])
                .with_merge(sha("s1s1s1s"), sha("m104000"))
                .with_last_active(20),
        )
        .with_current_branch("main")
}

#[test]
fn a_nested_lane_merged_into_the_default_lane_forks_from_its_parent() {
    let text = mermaid(&nested_merges());
    assert!(
        text.contains(r#"    merge fix/wt-ux id: "m103000" tag: "main"
    merge fix/sniff id: "m104000" tag: "origin/main""#),
        "{text}"
    );
    assert_eq!(parents_of(&text, "m103000"), ["d1d1d1d", "w2w2w2w"]);
    assert_eq!(parents_of(&text, "m104000"), ["m103000", "s1s1s1s"]);
    assert_eq!(parents_of(&text, "+96"), ["w2w2w2w"]);
    assert_eq!(lane_of_commit(&text, "s1s1s1s"), "fix/sniff");
    assert!(!plan(&nested_merges(), viewport(400, 60)).incomplete);
}

#[test]
fn a_merge_commit_can_also_be_a_fork_point() {
    let graph = GitGraph::new("main", commits(&["aaaaaaa", "mmmmmmm", "nnnnnnn"]))
        .with_line(
            GraphLine::new("side")
                .forked_at(sha("aaaaaaa"))
                .with_entries(commits(&["s1s1s1s"]))
                .with_merge(sha("s1s1s1s"), sha("mmmmmmm")),
        )
        .with_line(GraphLine::new("later").forked_at(sha("mmmmmmm")).with_entries(commits(&["l1l1l1l"])));
    let text = mermaid(&graph);
    assert!(
        text.contains(r#"    merge side id: "mmmmmmm"
    branch later
    checkout later
    commit id: "l1l1l1l""#),
        "{text}"
    );
    assert_eq!(parents_of(&text, "mmmmmmm"), ["aaaaaaa", "s1s1s1s"]);
    assert_eq!(parents_of(&text, "l1l1l1l"), ["mmmmmmm"]);
}

#[test]
fn a_lane_merged_into_a_sibling_is_emitted_before_it() {
    // Both fork at `1111111`; creation order alone would emit `feat/p` (and
    // its merge commit) before `feat/c` has any commits.
    let graph = GitGraph::new("main", commits(&["1111111", "2222222"]))
        .with_line(
            GraphLine::new("feat/p")
                .forked_at(sha("1111111"))
                .with_entries(commits(&["ppppppp", "qqqqqqq"]))
                .with_created_at(100),
        )
        .with_line(
            GraphLine::new("feat/c")
                .with_parent("feat/p")
                .forked_at(sha("1111111"))
                .with_entries(commits(&["ccccccc"]))
                .with_merge(sha("ccccccc"), sha("qqqqqqq"))
                .with_created_at(200),
        )
        .with_current_branch("feat/c");
    let text = mermaid(&graph);
    assert!(text.find("branch feat/c").unwrap() < text.find("branch feat/p").unwrap(), "{text}");
    assert_eq!(parents_of(&text, "qqqqqqq"), ["ppppppp", "ccccccc"]);
    assert!(!plan(&graph, viewport(200, 40)).incomplete);
}

#[test]
fn a_merge_whose_destination_is_not_drawn_is_left_out_and_reported() {
    let graph = GitGraph::new("main", commits(&["1111111", "2222222"])).with_line(
        GraphLine::new("feat/x")
            .forked_at(sha("1111111"))
            .with_entries(commits(&["aaaaaaa"]))
            .with_merge(sha("aaaaaaa"), sha("0ld0000")),
    );
    let text = mermaid(&graph);
    assert!(!text.contains("merge"), "{text}");
    assert!(plan(&graph, viewport(200, 40)).incomplete);
}

#[test]
fn a_merge_that_would_precede_its_lane_is_left_out_and_reported() {
    // The lane forks after its own merge destination (inconsistent input).
    let graph = GitGraph::new("main", commits(&["1111111", "mmmmmmm", "2222222"])).with_line(
        GraphLine::new("feat/x")
            .forked_at(sha("2222222"))
            .with_entries(commits(&["aaaaaaa"]))
            .with_merge(sha("aaaaaaa"), sha("mmmmmmm")),
    );
    let text = mermaid(&graph);
    assert!(text.contains(r#"commit id: "mmmmmmm""#), "{text}");
    assert!(!text.contains("merge"), "{text}");
    assert!(plan(&graph, viewport(200, 40)).incomplete);
}

#[test]
fn a_merge_at_the_start_of_the_default_lane_is_left_out_and_reported() {
    // The parser drops a labeled merge into a lane with no commit yet.
    let graph = GitGraph::new("main", commits(&["mmmmmmm", "2222222"])).with_line(
        GraphLine::new("feat/x")
            .forked_at(sha("0ld0000"))
            .with_entries(commits(&["aaaaaaa"]))
            .with_merge(sha("aaaaaaa"), sha("mmmmmmm")),
    );
    let text = mermaid(&graph);
    assert!(text.contains(r#"commit id: "mmmmmmm""#), "{text}");
    assert!(!text.contains("merge"), "{text}");
    assert!(plan(&graph, viewport(200, 40)).incomplete);
}

#[test]
fn only_one_lane_merges_at_a_commit() {
    let merged = |name: &str, commit: &str| {
        GraphLine::new(name)
            .forked_at(sha("1111111"))
            .with_entries(commits(&[commit]))
            .with_merge(sha(commit), sha("mmmmmmm"))
    };
    let graph = GitGraph::new("main", commits(&["1111111", "mmmmmmm"]))
        .with_line(merged("a", "aaaaaaa"))
        .with_line(merged("b", "bbbbbbb"));
    let text = mermaid(&graph);
    assert_eq!(text.matches("merge ").count(), 1, "{text}");
    assert!(text.contains(r#"merge a id: "mmmmmmm""#), "{text}");
    assert!(plan(&graph, viewport(200, 40)).incomplete);
}

#[test]
fn a_fork_commit_missing_from_the_parent_lane_is_not_replaced_by_its_start() {
    let graph = GitGraph::new("main", commits(&["1111111"]))
        .with_line(
            GraphLine::new("feat/parent")
                .forked_at(sha("1111111"))
                .with_entries(commits(&["ppppppp", "qqqqqqq"])),
        )
        .with_line(
            GraphLine::new("feat/child")
                .with_parent("feat/parent")
                .forked_at(sha("0ld0000"))
                .with_entries(commits(&["ccccccc"])),
        )
        .with_current_branch("feat/child");
    let text = mermaid(&graph);
    assert!(text.starts_with("gitGraph\n    branch feat/child\n    checkout main\n"), "{text}");
    assert_eq!(text.matches("branch feat/child").count(), 1, "declared once, unconnected: {text}");
    assert_eq!(parents_of(&text, "ccccccc"), Vec::<String>::new());
    assert_eq!(parents_of(&text, "ppppppp"), ["1111111"]);
    assert!(plan(&graph, viewport(200, 40)).incomplete);
}

#[test]
fn an_unknown_fork_is_drawn_unconnected_and_reported() {
    let graph = GitGraph::new("main", commits(&["1111111", "2222222"]))
        .with_line(GraphLine::new("feat/x").with_entries(commits(&["aaaaaaa"])));
    let text = mermaid(&graph);
    assert!(text.starts_with("gitGraph\n    branch feat/x\n    checkout main\n"), "{text}");
    assert_eq!(parents_of(&text, "aaaaaaa"), Vec::<String>::new());
    assert!(plan(&graph, viewport(200, 40)).incomplete);
}

#[test]
fn a_tag_on_an_undrawn_commit_is_reported() {
    // Control: everything the spec example tags is drawn.
    assert!(!plan(&spec_example(), viewport(200, 40)).incomplete);
    let graph = spec_example().with_ref("origin/feat/theme", sha("0ld0000"));
    assert!(!mermaid(&graph).contains("origin/feat/theme"));
    assert!(plan(&graph, viewport(200, 40)).incomplete);
}

#[test]
fn a_label_only_line_sits_at_its_tip_never_its_fork() {
    let graph = spec_example().with_line(
        GraphLine::new("chore/done")
            .forked_at(sha("4c2e4d5"))
            .with_tip(sha("d44f301")),
    );
    let text = mermaid(&graph);
    assert!(text.contains(r#"commit id: "d44f301" tag: "chore/done""#), "{text}");
    assert!(text.contains("commit id: \"4c2e4d5\"\n"), "not at the fork: {text}");
    assert!(!plan(&graph, viewport(200, 40)).incomplete);

    // Without a tip, or with an undrawn one, the fork is never used: the label
    // is reported as not shown.
    for line in [
        GraphLine::new("chore/done").forked_at(sha("4c2e4d5")),
        GraphLine::new("chore/done").forked_at(sha("4c2e4d5")).with_tip(sha("0ld0000")),
    ] {
        let graph = spec_example().with_current_branch("main").with_line(line);
        assert!(!mermaid(&graph).contains("chore/done"));
        assert!(plan(&graph, viewport(200, 40)).incomplete);
    }
}

#[test]
fn a_pull_request_for_a_branch_the_graph_does_not_know_is_not_reported() {
    let graph = spec_example().with_pull_request(GraphPullRequest {
        number: 7,
        source_branch: "not/drawn".into(),
        target_branch: "main".into(),
    });
    assert!(!plan(&graph, viewport(200, 40)).incomplete);
}

#[test]
fn the_caller_can_report_incomplete_history() {
    let planned = plan(&spec_example().with_incomplete_history(), viewport(200, 40));
    assert!(planned.incomplete);
    assert_eq!(planned.mermaid, SPEC_EXAMPLE);
}

#[test]
fn trimming_never_elides_a_merge_destination() {
    let graph = GitGraph::new("main", commits(&["1111111", "2222222", "mmmmmmm", "3333333", "4444444"]))
        .with_line(
            GraphLine::new("feat/x")
                .forked_at(sha("1111111"))
                .with_entries(commits(&["aaaaaaa", "bbbbbbb"]))
                .with_merge(sha("bbbbbbb"), sha("mmmmmmm")),
        );
    let planned = plan(&graph, viewport(5, 40));
    assert!(planned.trimmed_commits > 0, "{planned:?}");
    assert!(planned.mermaid.contains(r#"merge feat/x id: "mmmmmmm""#), "{}", planned.mermaid);
    for trimmed in ["2222222", "3333333", "aaaaaaa"] {
        assert!(!planned.mermaid.contains(trimmed), "{trimmed} trimmed: {}", planned.mermaid);
    }
    assert!(!planned.incomplete);
}

#[test]
fn the_height_cap_keeps_a_merged_lanes_destination_lane() {
    // `feat/x` forks from `main` but merged into `feat/p`, the least active
    // lane; keeping `feat/x` keeps `feat/p`, so `feat/b` is left out.
    let graph = GitGraph::new("main", commits(&["1111111"]))
        .with_ref("main", sha("1111111"))
        .with_line(line("feat/p", None, "ppppppp", 1).with_entries(commits(&["ppppppp", "qqqqqqq"])))
        .with_line(line("feat/b", None, "bbbbbbb", 50))
        .with_line(line("feat/x", None, "xxxxxxx", 90).with_merge(sha("xxxxxxx"), sha("qqqqqqq")))
        .with_current_branch("main");
    let planned = plan(&graph, viewport(200, 40));
    let text = &planned.mermaid;
    assert!(text.contains("branch feat/p") && text.contains(r#"merge feat/x id: "qqqqqqq""#), "{text}");
    assert!(!text.contains("feat/b"), "{text}");
    assert_eq!(planned.hidden_lanes, 1);
    assert!(!planned.incomplete, "the cap's own note accounts for feat/b");
}

#[test]
fn tags_of_lanes_the_height_cap_leaves_out_are_not_reported_twice() {
    let graph = base_view_with_lanes()
        .with_ref("origin/feat/a", sha("aaaaaaa"))
        .with_pull_request(GraphPullRequest {
            number: 5,
            source_branch: "feat/a".into(),
            target_branch: "main".into(),
        });
    let planned = plan(&graph, viewport(200, 40));
    assert!(!planned.mermaid.contains("feat/a"), "{}", planned.mermaid);
    assert!(planned.hidden_lanes > 0);
    assert!(!planned.incomplete);
}

// ---------------------------------------------------------------------------
// Merges from inside a lane: segments, pauses, and dropped edges
// ---------------------------------------------------------------------------

/// Every `id: "…"` in emission order.
fn emitted_ids(text: &str) -> Vec<&str> {
    text.split("id: \"").skip(1).filter_map(|rest| rest.split('"').next()).collect()
}

fn assert_each_id_once(text: &str) {
    let ids = emitted_ids(text);
    let distinct: HashSet<&str> = ids.iter().copied().collect();
    assert_eq!(ids.len(), distinct.len(), "an ID repeats: {text}");
}

/// `feat/x` merged into `main` at `c1c1c1c` from `b1b1b1b`, then continued
/// with `after` (oldest first).
fn continued_after_merge(after: Vec<LaneEntry>) -> GitGraph {
    let mut entries = commits(&["b1b1b1b"]);
    entries.extend(after);
    GitGraph::new("main", commits(&["d1d1d1d", "p1p1p1p", "c1c1c1c", "m2m2m2m"])).with_line(
        GraphLine::new("feat/x")
            .forked_at(sha("d1d1d1d"))
            .with_entries(entries)
            .with_merge(sha("b1b1b1b"), sha("c1c1c1c")),
    )
}

#[test]
fn a_lane_merged_from_its_middle_pauses_at_the_source_and_resumes_after_the_merge() {
    let graph = continued_after_merge(commits(&["n1n1n1n", "n2n2n2n"]));
    let text = mermaid(&graph);
    assert_eq!(
        text,
        r#"gitGraph
    commit id: "d1d1d1d"
    branch feat/x
    checkout feat/x
    commit id: "b1b1b1b"
    checkout main
    commit id: "p1p1p1p"
    merge feat/x id: "c1c1c1c"
    checkout feat/x
    commit id: "n1n1n1n"
    commit id: "n2n2n2n"
    checkout main
    commit id: "m2m2m2m""#
    );
    assert_eq!(parents_of(&text, "c1c1c1c"), ["p1p1p1p", "b1b1b1b"]);
    assert_eq!(parents_of(&text, "n1n1n1n"), ["b1b1b1b"]);
    assert_eq!(parents_of(&text, "m2m2m2m"), ["c1c1c1c"]);
    assert_eq!(lane_of_commit(&text, "n1n1n1n"), "feat/x");
    assert!(!plan(&graph, viewport(400, 40)).incomplete);
}

#[test]
fn a_square_folded_after_the_merge_source_hangs_from_the_source() {
    let graph = continued_after_merge(vec![LaneEntry::Elided(2), LaneEntry::Commit(sha("n2n2n2n"))]);
    let text = mermaid(&graph);
    assert!(
        text.contains(r#"    merge feat/x id: "c1c1c1c"
    checkout feat/x
    commit id: "+2" type: HIGHLIGHT
    commit id: "n2n2n2n""#),
        "{text}"
    );
    assert_eq!(parents_of(&text, "c1c1c1c"), ["p1p1p1p", "b1b1b1b"]);
    assert_eq!(parents_of(&text, "+2"), ["b1b1b1b"]);
    assert_eq!(parents_of(&text, "n2n2n2n"), ["+2"]);
}

#[test]
fn children_forked_before_and_after_the_merge_source_hang_from_their_own_commits() {
    let graph = GitGraph::new("main", commits(&["d1d1d1d", "p1p1p1p", "c1c1c1c"]))
        .with_line(
            GraphLine::new("src")
                .forked_at(sha("d1d1d1d"))
                .with_entries(commits(&["a1a1a1a", "b1b1b1b", "n1n1n1n"]))
                .with_merge(sha("b1b1b1b"), sha("c1c1c1c")),
        )
        .with_line(GraphLine::new("early").forked_at(sha("a1a1a1a")).with_entries(commits(&["e1e1e1e"])))
        .with_line(GraphLine::new("late").forked_at(sha("n1n1n1n")).with_entries(commits(&["l1l1l1l"])));
    let text = mermaid(&graph);
    assert_eq!(
        text,
        r#"gitGraph
    commit id: "d1d1d1d"
    branch src
    checkout src
    commit id: "a1a1a1a"
    branch early
    checkout early
    commit id: "e1e1e1e"
    checkout src
    commit id: "b1b1b1b"
    checkout main
    commit id: "p1p1p1p"
    merge src id: "c1c1c1c"
    checkout src
    commit id: "n1n1n1n"
    branch late
    checkout late
    commit id: "l1l1l1l"
    checkout src
    checkout main"#
    );
    assert_eq!(parents_of(&text, "e1e1e1e"), ["a1a1a1a"]);
    assert_eq!(parents_of(&text, "c1c1c1c"), ["p1p1p1p", "b1b1b1b"]);
    assert_eq!(parents_of(&text, "n1n1n1n"), ["b1b1b1b"]);
    assert_eq!(parents_of(&text, "l1l1l1l"), ["n1n1n1n"]);
    assert!(!plan(&graph, viewport(400, 60)).incomplete);
}

#[test]
fn a_child_forked_at_the_merge_source_is_declared_before_the_merge() {
    let graph = GitGraph::new("main", commits(&["d1d1d1d", "p1p1p1p", "c1c1c1c"]))
        .with_line(
            GraphLine::new("fix/wt-ux")
                .forked_at(sha("d1d1d1d"))
                .with_entries(commits(&["b1b1b1b", "n1n1n1n"]))
                .with_merge(sha("b1b1b1b"), sha("c1c1c1c")),
        )
        .with_line(GraphLine::new("fix/sniff").forked_at(sha("b1b1b1b")).with_entries(commits(&["k1k1k1k"])))
        .with_line(GraphLine::new("fix/sniff-pr").forked_at(sha("b1b1b1b")).with_tip(sha("b1b1b1b")));
    let text = mermaid(&graph);
    assert_eq!(
        text,
        r#"gitGraph
    commit id: "d1d1d1d"
    branch fix/wt-ux
    checkout fix/wt-ux
    commit id: "b1b1b1b" tag: "fix/sniff-pr"
    branch fix/sniff
    checkout fix/sniff
    commit id: "k1k1k1k"
    checkout fix/wt-ux
    checkout main
    commit id: "p1p1p1p"
    merge fix/wt-ux id: "c1c1c1c"
    checkout fix/wt-ux
    commit id: "n1n1n1n"
    checkout main"#
    );
    let geometry = geometry_of(&text);
    let source = geometry.commit("b1b1b1b").expect("the source is drawn");
    assert!(source.tags.iter().any(|tag| tag.text == "fix/sniff-pr"), "{source:?}");
    assert_eq!(parents_of(&text, "k1k1k1k"), ["b1b1b1b"]);
    assert_eq!(parents_of(&text, "c1c1c1c"), ["p1p1p1p", "b1b1b1b"]);
    assert_eq!(parents_of(&text, "n1n1n1n"), ["b1b1b1b"]);
    assert!(!plan(&graph, viewport(400, 60)).incomplete);
}

#[test]
fn a_lane_merged_twice_and_continued_draws_both_merges_in_source_order() {
    let graph = GitGraph::new("main", commits(&["d1d1d1d", "p1p1p1p", "c1c1c1c", "p2p2p2p", "c2c2c2c"])).with_line(
        GraphLine::new("feat/x")
            .forked_at(sha("d1d1d1d"))
            .with_entries(commits(&["b1b1b1b", "x1x1x1x", "b2b2b2b", "n1n1n1n"]))
            .with_merge(sha("b1b1b1b"), sha("c1c1c1c"))
            .with_merge(sha("b2b2b2b"), sha("c2c2c2c")),
    );
    let text = mermaid(&graph);
    assert_eq!(
        text,
        r#"gitGraph
    commit id: "d1d1d1d"
    branch feat/x
    checkout feat/x
    commit id: "b1b1b1b"
    checkout main
    commit id: "p1p1p1p"
    merge feat/x id: "c1c1c1c"
    checkout feat/x
    commit id: "x1x1x1x"
    commit id: "b2b2b2b"
    checkout main
    commit id: "p2p2p2p"
    merge feat/x id: "c2c2c2c"
    checkout feat/x
    commit id: "n1n1n1n"
    checkout main"#
    );
    assert_each_id_once(&text);
    assert_eq!(parents_of(&text, "c1c1c1c"), ["p1p1p1p", "b1b1b1b"]);
    assert_eq!(parents_of(&text, "x1x1x1x"), ["b1b1b1b"]);
    assert_eq!(parents_of(&text, "c2c2c2c"), ["p2p2p2p", "b2b2b2b"]);
    assert_eq!(parents_of(&text, "n1n1n1n"), ["b2b2b2b"]);
    assert!(!plan(&graph, viewport(400, 40)).incomplete);
}

#[test]
fn a_merge_into_a_sibling_that_merges_into_the_default_lane_emits_each_source_first() {
    // Creation order alone would emit `lane/s`, and its merge commit `s2`,
    // before `lane/a` has emitted the source `a1`.
    let graph = GitGraph::new("main", commits(&["d1d1d1d", "p1p1p1p", "c1c1c1c"]))
        .with_line(
            GraphLine::new("lane/s")
                .forked_at(sha("d1d1d1d"))
                .with_entries(commits(&["s1s1s1s", "s2s2s2s", "s3s3s3s"]))
                .with_merge(sha("s3s3s3s"), sha("c1c1c1c"))
                .with_created_at(100),
        )
        .with_line(
            GraphLine::new("lane/a")
                .forked_at(sha("d1d1d1d"))
                .with_entries(commits(&["a1a1a1a", "a2a2a2a"]))
                .with_merge(sha("a1a1a1a"), sha("s2s2s2s"))
                .with_created_at(200),
        );
    let text = mermaid(&graph);
    assert_eq!(
        text,
        r#"gitGraph
    commit id: "d1d1d1d"
    branch lane/a
    checkout lane/a
    commit id: "a1a1a1a"
    checkout main
    branch lane/s
    checkout lane/s
    commit id: "s1s1s1s"
    merge lane/a id: "s2s2s2s"
    checkout lane/a
    commit id: "a2a2a2a"
    checkout lane/s
    commit id: "s3s3s3s"
    checkout main
    commit id: "p1p1p1p"
    merge lane/s id: "c1c1c1c""#
    );
    assert_each_id_once(&text);
    let ids = emitted_ids(&text);
    let at = |id: &str| ids.iter().position(|emitted| *emitted == id).expect(id);
    assert!(at("a1a1a1a") < at("s2s2s2s") && at("s3s3s3s") < at("c1c1c1c"), "{text}");
    assert_eq!(parents_of(&text, "s2s2s2s"), ["s1s1s1s", "a1a1a1a"]);
    assert_eq!(parents_of(&text, "a2a2a2a"), ["a1a1a1a"]);
    assert_eq!(parents_of(&text, "c1c1c1c"), ["p1p1p1p", "s3s3s3s"]);
    assert!(!plan(&graph, viewport(400, 60)).incomplete);
}

#[test]
fn a_cycle_of_merges_draws_one_and_reports_the_other() {
    // `lane/a` merges into `lane/s` and `lane/s` into `lane/a`: neither can be
    // emitted through its source before the other's destination.
    let graph = GitGraph::new("main", commits(&["d1d1d1d"]))
        .with_line(
            GraphLine::new("lane/a")
                .forked_at(sha("d1d1d1d"))
                .with_entries(commits(&["a1a1a1a", "a2a2a2a"]))
                .with_merge(sha("a1a1a1a"), sha("s2s2s2s"))
                .with_created_at(100),
        )
        .with_line(
            GraphLine::new("lane/s")
                .forked_at(sha("d1d1d1d"))
                .with_entries(commits(&["s1s1s1s", "s2s2s2s"]))
                .with_merge(sha("s1s1s1s"), sha("a2a2a2a"))
                .with_created_at(200),
        );
    let text = mermaid(&graph);
    assert_eq!(
        text,
        r#"gitGraph
    commit id: "d1d1d1d"
    branch lane/s
    checkout lane/s
    commit id: "s1s1s1s"
    checkout main
    branch lane/a
    checkout lane/a
    commit id: "a1a1a1a"
    checkout main
    checkout lane/s
    merge lane/a id: "s2s2s2s"
    checkout lane/a
    commit id: "a2a2a2a"
    checkout lane/s"#
    );
    assert_eq!(text.matches("merge ").count(), 1, "{text}");
    assert_each_id_once(&text);
    let mut ids = emitted_ids(&text);
    ids.sort_unstable();
    assert_eq!(ids, ["a1a1a1a", "a2a2a2a", "d1d1d1d", "s1s1s1s", "s2s2s2s"]);
    assert_eq!(parents_of(&text, "s2s2s2s"), ["s1s1s1s", "a1a1a1a"]);
    assert_eq!(parents_of(&text, "a2a2a2a"), ["a1a1a1a"]);
    assert!(plan(&graph, viewport(400, 60)).incomplete);
}

#[test]
fn a_destination_emitted_before_its_source_is_a_plain_commit_and_the_source_does_not_pause() {
    // The lane forks after its own merge destination (inconsistent input).
    let graph = GitGraph::new("main", commits(&["d1d1d1d", "c1c1c1c", "p2p2p2p"])).with_line(
        GraphLine::new("feat/x")
            .forked_at(sha("p2p2p2p"))
            .with_entries(commits(&["b1b1b1b", "n1n1n1n"]))
            .with_merge(sha("b1b1b1b"), sha("c1c1c1c")),
    );
    let text = mermaid(&graph);
    assert_eq!(
        text,
        r#"gitGraph
    commit id: "d1d1d1d"
    commit id: "c1c1c1c"
    commit id: "p2p2p2p"
    branch feat/x
    checkout feat/x
    commit id: "b1b1b1b"
    commit id: "n1n1n1n"
    checkout main"#
    );
    assert_eq!(parents_of(&text, "n1n1n1n"), ["b1b1b1b"]);
    assert!(plan(&graph, viewport(400, 40)).incomplete);
}

#[test]
fn a_merge_whose_source_is_not_on_its_lane_is_left_out_and_reported() {
    let graph = GitGraph::new("main", commits(&["d1d1d1d", "p1p1p1p", "c1c1c1c"])).with_line(
        GraphLine::new("feat/x")
            .forked_at(sha("d1d1d1d"))
            .with_entries(commits(&["b1b1b1b"]))
            .with_merge(sha("p1p1p1p"), sha("c1c1c1c")),
    );
    let text = mermaid(&graph);
    assert!(!text.contains("merge"), "{text}");
    assert!(plan(&graph, viewport(400, 40)).incomplete);
}

/// `feat/x` (most active) merges from the middle of its lane into `feat/p`
/// (least active); `feat/b` sits between them in activity.
fn mid_lane_merge_into_a_branch_lane() -> GitGraph {
    GitGraph::new("main", commits(&["1111111"]))
        .with_ref("main", sha("1111111"))
        .with_line(line("feat/p", None, "ppppppp", 1).with_entries(commits(&["ppppppp", "qqqqqqq"])))
        .with_line(line("feat/b", None, "bbbbbbb", 50))
        .with_line(
            line("feat/x", None, "x1x1x1x", 90)
                .with_entries(commits(&["x1x1x1x", "x2x2x2x"]))
                .with_merge(sha("x1x1x1x"), sha("qqqqqqq")),
        )
        .with_current_branch("main")
}

#[test]
fn the_height_cap_keeps_a_mid_lane_merges_destination_lane_with_its_source() {
    // Half of 40 rows fits the default lane and two more: `feat/x` brings
    // `feat/p`, so `feat/b` is left out.
    let planned = plan(&mid_lane_merge_into_a_branch_lane(), viewport(200, 40));
    let text = &planned.mermaid;
    assert!(text.contains("branch feat/p") && text.contains(r#"merge feat/x id: "qqqqqqq""#), "{text}");
    assert!(!text.contains("feat/b"), "{text}");
    assert_eq!(planned.hidden_lanes, 1);
    assert!(!planned.incomplete, "the cap's own note accounts for feat/b");
    assert_eq!(parents_of(text, "qqqqqqq"), ["ppppppp", "x1x1x1x"]);
    assert_eq!(parents_of(text, "x2x2x2x"), ["x1x1x1x"]);
}

#[test]
fn the_height_cap_hides_a_source_lane_and_its_destination_lane_together() {
    // Half of 26 rows fits the default lane and one more (12.5 rows);
    // `feat/x` needs `feat/p` too, so neither is drawn, and the lane note, not
    // the incomplete-history notice, accounts for the merge between them.
    let planned = plan(&mid_lane_merge_into_a_branch_lane(), viewport(200, 26));
    let text = &planned.mermaid;
    for hidden in ["feat/x", "feat/p", "qqqqqqq", "x1x1x1x"] {
        assert!(!text.contains(hidden), "{hidden} hidden: {text}");
    }
    assert_eq!(planned.hidden_lanes, 3);
    assert!(!planned.incomplete, "{planned:?}");
}

#[test]
fn trimming_never_folds_a_merge_source_or_its_destination() {
    let graph = GitGraph::new("main", commits(&["d1d1d1d", "m1m1m1m", "m2m2m2m", "c1c1c1c", "m3m3m3m", "m4m4m4m"])).with_line(
        GraphLine::new("feat/x")
            .forked_at(sha("d1d1d1d"))
            .with_entries(commits(&["x1x1x1x", "b1b1b1b", "x2x2x2x", "x3x3x3x", "n1n1n1n"]))
            .with_merge(sha("b1b1b1b"), sha("c1c1c1c")),
    );
    let planned = plan(&graph, viewport(5, 40));
    let text = &planned.mermaid;
    assert!(planned.trimmed_commits > 0, "{planned:?}");
    for trimmed in ["m1m1m1m", "m2m2m2m", "m3m3m3m", "x1x1x1x", "x2x2x2x", "x3x3x3x"] {
        assert!(!text.contains(trimmed), "{trimmed} trimmed: {text}");
    }
    assert!(text.contains(r#"merge feat/x id: "c1c1c1c""#), "{text}");
    assert_eq!(parents_of(text, "c1c1c1c")[1], "b1b1b1b", "{text}");
    // The square after the source hangs from it.
    let geometry = geometry_of(text);
    let after_source = geometry
        .commits
        .iter()
        .find(|commit| commit.lane == "feat/x" && commit.parents == ["b1b1b1b"])
        .unwrap_or_else(|| panic!("a commit after b1b1b1b: {text}"));
    assert!(after_source.id.starts_with('+'), "{after_source:?}");
    assert!(!planned.incomplete);
}

#[test]
fn every_emitted_graph_parses() {
    for graph in [
        spec_example(),
        spec_example().with_current_branch("main"),
        merged_into_default(),
        nested_merges(),
    ] {
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
// Real backend: `plan()` measured by biscuit-visualized, checked against the
// layout that renders (parents, tag placement, tag bounds), never the text.
// ---------------------------------------------------------------------------

const ALPHA: &str = "feature/very-long-exact-branch-reference-alpha";
const BETA: &str = "origin/very-long-exact-branch-reference-beta";

/// Long labels on neighboring commits of two lanes, `main` and
/// `origin/main` one commit apart, and a PR tag.
fn long_label_neighbors() -> GitGraph {
    GitGraph::new("main", commits(&["d1d1d1d", "d2d2d2d", "d3d3d3d", "d4d4d4d"]))
        .with_ref("main", sha("d4d4d4d"))
        .with_ref("origin/main", sha("d3d3d3d"))
        .with_ref(BETA, sha("f1f1f1f"))
        .with_line(
            GraphLine::new(ALPHA)
                .forked_at(sha("d2d2d2d"))
                .with_tip(sha("f2f2f2f"))
                .with_entries(commits(&["f1f1f1f", "f2f2f2f"])),
        )
        .with_pull_request(GraphPullRequest {
            number: 104,
            source_branch: ALPHA.into(),
            target_branch: "main".into(),
        })
        .with_current_branch(ALPHA)
}

/// A fork 5,000 and a merge 3,000 first-parent commits back, compressed into
/// `+N` squares around the anchors.
fn compressed_old_connections() -> GitGraph {
    GitGraph::new(
        "main",
        vec![
            LaneEntry::Commit(sha("f0f0f0f")),
            LaneEntry::Elided(1999),
            LaneEntry::Commit(sha("m0m0m0m")),
            LaneEntry::Elided(2995),
            LaneEntry::Commit(sha("t1t1t1t")),
            LaneEntry::Commit(sha("t2t2t2t")),
        ],
    )
    .with_ref("main", sha("t2t2t2t"))
    .with_ref("origin/main", sha("t2t2t2t"))
    .with_line(
        GraphLine::new("feat/old")
            .forked_at(sha("f0f0f0f"))
            .with_tip(sha("o2o2o2o"))
            .with_entries(vec![LaneEntry::Elided(40), LaneEntry::Commit(sha("o2o2o2o"))])
            .with_merge(sha("o2o2o2o"), sha("m0m0m0m")),
    )
    .with_current_branch("feat/old")
}

struct Expected<'a> {
    /// `(commit display ID, tag)` that must be placed exactly so.
    tags: &'a [(&'a str, &'a str)],
    /// `(merge display ID, [first parent, second parent])`.
    merges: &'a [(&'a str, [&'a str; 2])],
}

fn assert_real_backend(name: &str, graph: &GitGraph, expected: &Expected) -> Vec<String> {
    let graph = graph.clone().with_theme(MermaidTheme::Default);
    let mut report = Vec::new();
    for vp in [viewport(120, 40), viewport(56, 60)] {
        let planned = graph.plan(vp).expect("a plan");
        let geometry = geometry_of(&planned.mermaid);
        assert_eq!(geometry.tag_overlaps(), Vec::<(String, String)>::new(), "{name} {vp:?}: {}", planned.mermaid);
        for (id, tag) in expected.tags {
            let placed = geometry
                .commit(id)
                .unwrap_or_else(|| panic!("{name} {vp:?}: {id} drawn: {}", planned.mermaid));
            assert!(
                placed.tags.iter().any(|placed| placed.text == *tag),
                "{name} {vp:?}: {tag} on {id}: {placed:?}"
            );
        }
        for (id, [first, second]) in expected.merges {
            let parents = &geometry.commit(id).unwrap_or_else(|| panic!("{name} {vp:?}: merge {id}")).parents;
            assert_eq!(parents.len(), 2, "{name} {vp:?}: {parents:?}");
            // The first parent is the destination lane's previous entry, which
            // trimming may have folded into a `+N` square.
            assert!(
                parents[0] == *first || (planned.trimmed_commits > 0 && parents[0].starts_with('+')),
                "{name} {vp:?}: {parents:?}"
            );
            assert_eq!(parents[1], *second, "{name} {vp:?}");
        }
        if planned.columns > vp.columns {
            // Too wide only once nothing more can be trimmed.
            let floor = graph.plan(viewport(1, vp.rows)).unwrap();
            assert_eq!(planned.mermaid, floor.mermaid, "{name} {vp:?}: wider than the viewport before fully trimmed");
        }
        assert!(!planned.incomplete, "{name} {vp:?}");
        report.push(format!(
            "{name} {}x{}: columns={} rows={} trimmed={} step={:.1} natural_width={:.0}",
            vp.columns, vp.rows, planned.columns, planned.rows, planned.trimmed_commits, geometry.commit_step, geometry.width
        ));
    }
    report
}

#[test]
fn measured_plans_place_merges_and_tags_without_overlap() {
    let mut report = Vec::new();
    report.extend(assert_real_backend(
        "observation-1",
        &merged_into_default(),
        &Expected {
            tags: &[("mmmmmmm", "origin/main"), ("3333333", "main")],
            merges: &[("mmmmmmm", ["2222222", "bbbbbbb"])],
        },
    ));
    report.extend(assert_real_backend(
        "observation-2",
        &nested_merges(),
        &Expected {
            tags: &[("m103000", "main"), ("m104000", "origin/main")],
            merges: &[("m103000", ["d1d1d1d", "w2w2w2w"]), ("m104000", ["m103000", "s1s1s1s"])],
        },
    ));
    report.extend(assert_real_backend(
        "long-labels",
        &long_label_neighbors(),
        &Expected {
            tags: &[
                ("d3d3d3d", "origin/main"),
                ("d4d4d4d", "main"),
                ("f1f1f1f", BETA),
                ("f2f2f2f", "PR #104 → main"),
            ],
            merges: &[],
        },
    ));
    report.extend(assert_real_backend(
        "compressed",
        &compressed_old_connections(),
        &Expected {
            tags: &[("t2t2t2t", "main"), ("t2t2t2t", "origin/main")],
            merges: &[("m0m0m0m", ["+1999", "o2o2o2o"])],
        },
    ));
    // Per-viewport size, trimming, and the tag-driven commit step, for review.
    eprintln!("{}", report.join("\n"));
}

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
fn the_incomplete_history_notice_follows_the_lane_note() {
    let term = plain_terminal(200, 24);
    let graph = base_view_with_lanes()
        .with_theme(MermaidTheme::Default)
        .with_ref("v0.1.0", sha("0ld0000"));
    let output = graph.render(&term);
    let lanes = output.find("more worktrees not shown").expect("the lane note");
    let history = output.find(INCOMPLETE_HISTORY_NOTE).expect("the history notice");
    assert!(lanes < history, "{output}");
    assert_eq!(INCOMPLETE_HISTORY_NOTE, "Some history is not shown");
    assert!(graph.try_render(&term).is_err(), "no image support is still an error");

    // A complete graph has no notice.
    let complete = spec_example().with_theme(MermaidTheme::Default).render(&term);
    assert!(!complete.contains(INCOMPLETE_HISTORY_NOTE), "{complete}");
}

#[test]
fn render_without_notes_leaves_the_notes_to_the_caller() {
    let term = plain_terminal(200, 24);
    let graph = base_view_with_lanes()
        .with_theme(MermaidTheme::Default)
        .with_ref("v0.1.0", sha("0ld0000"));
    let (output, plan) = graph.render_without_notes(&term).expect("a graph to draw");
    assert!(plan.hidden_lanes > 0 && plan.incomplete, "{plan:?}");
    assert!(!output.contains("not shown"), "{output}");
    assert_eq!(graph.render(&term).trim_end(), with_notes(output, &plan, &term).trim_end());
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
