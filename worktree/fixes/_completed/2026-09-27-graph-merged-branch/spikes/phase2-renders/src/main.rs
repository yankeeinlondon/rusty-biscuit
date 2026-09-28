//! Phase 2 visual evidence (throwaway; see `run.sh`). For each fixture and
//! viewport it prints the plan, the layout's merge parents and tag overlaps,
//! and writes `<fixture>-<columns>x<rows>.png` and `.mmd`.

use std::fs;
use std::path::Path;

use biscuit_terminal::components::git_graph::{GitGraph, GraphLine, GraphPullRequest, GraphViewport, LaneEntry};
use biscuit_terminal::components::mermaid::MermaidTheme;
use biscuit_terminal::discovery::fonts::CellSize;
use biscuit_visualized::artifact::{OutputFormat, RenderRequest};
use biscuit_visualized::mermaid::MermaidDiagram;

const ALPHA: &str = "feature/very-long-exact-branch-reference-alpha";
const BETA: &str = "origin/very-long-exact-branch-reference-beta";

fn sha(short: &str) -> String {
    format!("{short:0<40}")
}

fn commits(shorts: &[&str]) -> Vec<LaneEntry> {
    shorts.iter().map(|short| LaneEntry::Commit(sha(short))).collect()
}

fn fixtures() -> Vec<(&'static str, GitGraph)> {
    let observation_1 = GitGraph::new("main", commits(&["1111111", "2222222", "08cf96a", "3333333"]))
        .with_ref("main", sha("3333333"))
        .with_ref("origin/main", sha("08cf96a"))
        .with_line(
            GraphLine::new("fix/wt-ux")
                .forked_at(sha("1111111"))
                .with_tip(sha("2a7298a"))
                .with_entries(commits(&["7ba6ea8", "2a7298a"]))
                .merged_into(sha("08cf96a")),
        )
        .with_current_branch("fix/wt-ux");
    let observation_2 = GitGraph::new("main", commits(&["1314bd6", "08cf96a", "b47a046"]))
        .with_ref("main", sha("08cf96a"))
        .with_ref("origin/main", sha("b47a046"))
        .with_line(
            GraphLine::new("fix/wt-ux")
                .forked_at(sha("1314bd6"))
                .with_tip(sha("2a7298a"))
                .with_entries(commits(&["7ba6ea8", "2a7298a"]))
                .merged_into(sha("08cf96a"))
                .with_last_active(10),
        )
        .with_line(
            GraphLine::new("fix/sniff")
                .with_parent("fix/wt-ux")
                .forked_at(sha("2a7298a"))
                .with_tip(sha("b3b17f3"))
                .with_entries(vec![LaneEntry::Elided(95), LaneEntry::Commit(sha("b3b17f3"))])
                .merged_into(sha("b47a046"))
                .with_last_active(20),
        )
        .with_current_branch("main");
    let long_labels = GitGraph::new("main", commits(&["d1d1d1d", "d2d2d2d", "d3d3d3d", "d4d4d4d"]))
        .with_ref("main", sha("d4d4d4d"))
        .with_ref("origin/main", sha("d3d3d3d"))
        .with_ref(BETA, sha("f1f1f1f"))
        .with_line(
            GraphLine::new(ALPHA)
                .forked_at(sha("d2d2d2d"))
                .with_tip(sha("f2f2f2f"))
                .with_entries(commits(&["f1f1f1f", "f2f2f2f"])),
        )
        .with_pull_request(GraphPullRequest { number: 104, source_branch: ALPHA.into(), target_branch: "main".into() })
        .with_current_branch(ALPHA);
    vec![("observation-1", observation_1), ("observation-2", observation_2), ("long-labels", long_labels)]
}

fn main() {
    let out = std::env::args().nth(1).expect("output directory");
    let out = Path::new(&out);
    for (name, graph) in fixtures() {
        let graph = graph.with_theme(MermaidTheme::Default);
        for (columns, rows) in [(120, 40), (56, 60)] {
            let viewport = GraphViewport { columns, rows, cell: CellSize::FALLBACK };
            let plan = graph.plan(viewport).expect("a plan");
            let diagram = MermaidDiagram::new(plan.mermaid.as_str()).with_theme(MermaidTheme::Default);
            let geometry = diagram.gitgraph_geometry().expect("lays out").expect("a gitGraph");
            let merges: Vec<String> = geometry
                .commits
                .iter()
                .filter(|commit| commit.parents.len() > 1)
                .map(|commit| format!("{}<-{:?}", commit.id, commit.parents))
                .collect();
            println!(
                "{name} {columns}x{rows}: columns={} rows={} trimmed={} incomplete={} step={:.1} overlaps={:?} merges={merges:?}",
                plan.columns,
                plan.rows,
                plan.trimmed_commits,
                plan.incomplete,
                geometry.commit_step,
                geometry.tag_overlaps()
            );
            let artifact = diagram
                .render(&RenderRequest { format: OutputFormat::Png, scale: 2, target_width: None, transparent_background: false })
                .expect("renders");
            let stem = format!("{name}-{columns}x{rows}");
            fs::copy(&artifact.path, out.join(format!("{stem}.png"))).unwrap();
            fs::write(out.join(format!("{stem}.mmd")), &plan.mermaid).unwrap();
        }
    }
}
