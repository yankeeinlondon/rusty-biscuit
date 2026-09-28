//! Phase 1 spikes S1 (renderer topologies) and S2 (tag geometry on real
//! `GitGraph` output). Throwaway: `run.sh` builds it as a standalone crate
//! outside the repository, so it is never compiled into the monorepo. It takes
//! the output directory as its only argument, prints the report, and writes
//! `.mmd`, `.svg`, and `.png` files.
//!
//! The merge repair (`repair`) and the two-pass spacing (`layout_spaced`) are
//! prototypes of the plan's R2 and R3, written against the renderer's public IR
//! so the rules can be checked before production code relies on them.

use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use biscuit_terminal::components::git_graph::{GitGraph, GraphLine, GraphPullRequest, LaneEntry};
use mermaid_rs_renderer::ir::GitGraphCommitType;
use mermaid_rs_renderer::layout::{DiagramData, Layout};
use mermaid_rs_renderer::{LayoutConfig, Theme, compute_layout, parse_mermaid, render_svg};

type Graph = mermaid_rs_renderer::ir::Graph;

#[derive(Debug, Clone, Copy, PartialEq)]
struct Rect {
    x1: f32,
    y1: f32,
    x2: f32,
    y2: f32,
}

impl Rect {
    fn of(points: &[(f32, f32)]) -> Self {
        let fold = |f: fn(f32, f32) -> f32, init: f32, pick: fn(&(f32, f32)) -> f32| {
            points.iter().map(pick).fold(init, f)
        };
        Rect {
            x1: fold(f32::min, f32::INFINITY, |p| p.0),
            y1: fold(f32::min, f32::INFINITY, |p| p.1),
            x2: fold(f32::max, f32::NEG_INFINITY, |p| p.0),
            y2: fold(f32::max, f32::NEG_INFINITY, |p| p.1),
        }
    }

    fn overlaps(&self, other: &Rect) -> bool {
        self.x1 < other.x2 && other.x1 < self.x2 && self.y1 < other.y2 && other.y1 < self.y2
    }
}

/// R2 prototype. Errors are strings; production uses `MermaidError::RenderFailed`.
fn repair(graph: &mut Graph) -> Result<Vec<String>, String> {
    let names: Vec<String> = graph.gitgraph.branches.iter().map(|b| b.name.clone()).collect();
    let mut notes = Vec::new();
    for index in 0..graph.gitgraph.commits.len() {
        let commit = &graph.gitgraph.commits[index];
        if commit.commit_type != GitGraphCommitType::Merge || commit.parents.len() >= 2 {
            continue;
        }
        let message = commit.message.clone().unwrap_or_default();
        let suffix = message
            .strip_prefix("merged branch ")
            .and_then(|rest| rest.rsplit_once(&format!(" into {}", commit.branch)))
            .map(|(from, _)| from.to_string())
            .ok_or_else(|| format!("{}: message format changed: {message:?}", commit.id))?;
        let matching: Vec<&String> = names
            .iter()
            .filter(|name| {
                suffix == **name
                    || suffix.strip_prefix(name.as_str()).is_some_and(|rest| {
                        let rest = rest.trim_start();
                        rest.len() < suffix.len() - name.len()
                            && ["id:", "tag:", "type:"].iter().any(|key| rest.starts_with(key))
                    })
            })
            .collect();
        let from = match matching.as_slice() {
            [one] => (*one).clone(),
            [] => return Err(format!("{}: no lane matches merge source {suffix:?}", commit.id)),
            many => return Err(format!("{}: ambiguous merge source {suffix:?}: {many:?}", commit.id)),
        };
        let seq = commit.seq;
        let source_tip = graph
            .gitgraph
            .commits
            .iter()
            .filter(|c| c.branch == from && c.seq < seq)
            .max_by_key(|c| c.seq)
            .map(|c| c.id.clone())
            .ok_or_else(|| format!("{}: merge source lane {from:?} has no earlier commit", commit.id))?;
        let commit = &mut graph.gitgraph.commits[index];
        if commit.parents.first() == Some(&source_tip) {
            notes.push(format!("{}: single parent already is the source tip; left alone", commit.id));
            continue;
        }
        commit.parents.push(source_tip.clone());
        notes.push(format!("{}: second parent {source_tip} from lane {from:?}", commit.id));
    }
    Ok(notes)
}

fn gitgraph(layout: &Layout) -> &mermaid_rs_renderer::layout::GitGraphLayout {
    let DiagramData::GitGraph(g) = &layout.diagram else {
        panic!("not a gitGraph layout")
    };
    g
}

/// R3 prototype: returns the layout, the config it used, and whether it took two passes.
fn layout_spaced(graph: &Graph, theme: &Theme, gap: Option<f32>) -> (Layout, LayoutConfig, bool) {
    let mut config = LayoutConfig::default();
    let first = compute_layout(graph, theme, &config);
    let Some(gap) = gap else {
        return (first, config, false);
    };
    let tags: Vec<_> = gitgraph(&first).commits.iter().flat_map(|c| &c.tags).collect();
    if tags.is_empty() || tags.iter().any(|t| t.transform.is_some()) {
        return (first, config, false);
    }
    let widest = tags
        .iter()
        .map(|t| {
            let r = Rect::of(&t.points);
            r.x2 - r.x1
        })
        .fold(0f32, f32::max);
    config.gitgraph.commit_step = config.gitgraph.commit_step.max(widest + gap);
    (compute_layout(graph, theme, &config), config, true)
}

struct Measured {
    report: String,
    tag_overlaps: usize,
    cross_lane_overlaps: usize,
}

fn analyze(out: &Path, name: &str, source: &str, fix: bool, theme: &Theme) -> Result<Measured, String> {
    fs::write(out.join(format!("{name}.mmd")), source).unwrap();
    let mut graph = parse_mermaid(source).map_err(|e| e.to_string())?.graph;
    let mut report = String::new();
    if fix {
        for note in repair(&mut graph)? {
            writeln!(report, "  repair: {note}").unwrap();
        }
    }
    let gap = fix.then_some(theme.font_size);
    let (layout, config, two_pass) = layout_spaced(&graph, theme, gap);
    let g = gitgraph(&layout);

    let mut boxes = Vec::new();
    for commit in &g.commits {
        let original = graph.gitgraph.commits.iter().find(|c| c.id == commit.id).unwrap();
        for tag in &commit.tags {
            assert!(original.tags.contains(&tag.text), "{name}: tag {:?} moved off {}", tag.text, commit.id);
            boxes.push((commit.id.clone(), commit.branch_index, tag.text.clone(), Rect::of(&tag.points)));
        }
    }
    let mut tag_overlaps = 0;
    let mut cross_lane_overlaps = 0;
    for (i, a) in boxes.iter().enumerate() {
        for b in boxes.iter().skip(i + 1) {
            if a.3.overlaps(&b.3) {
                tag_overlaps += 1;
                if a.1 != b.1 {
                    cross_lane_overlaps += 1;
                }
                writeln!(report, "  OVERLAP {:?}@{} x {:?}@{}", a.2, a.0, b.2, b.0).unwrap();
            }
        }
    }
    // Informational: tags covering another commit's dot (radius 10 around its center).
    let mut tag_over_dot = 0;
    for (owner, _, text, rect) in &boxes {
        for commit in &g.commits {
            let dot = Rect { x1: commit.x - 10.0, y1: commit.y - 10.0, x2: commit.x + 10.0, y2: commit.y + 10.0 };
            if &commit.id != owner && rect.overlaps(&dot) {
                tag_over_dot += 1;
                writeln!(report, "  note: tag {text:?}@{owner} covers commit {}", commit.id).unwrap();
            }
        }
    }
    let parent_edges: usize = graph
        .gitgraph
        .commits
        .iter()
        .map(|c| c.parents.iter().filter(|p| graph.gitgraph.commits.iter().any(|o| &o.id == *p)).count())
        .sum();

    let svg = render_svg(&layout, theme, &config);
    fs::write(out.join(format!("{name}.svg")), &svg).unwrap();
    mermaid_rs_renderer::render::write_output_png(&svg, &out.join(format!("{name}.png")), &Default::default(), theme)
        .map_err(|e| e.to_string())?;

    let mut head = String::new();
    writeln!(
        head,
        "{name}: commits={} arrows={} parent_edges={} step={} two_pass={two_pass} size={:.1}x{:.1} tag_overlaps={tag_overlaps} cross_lane={cross_lane_overlaps} tag_over_other_dot={tag_over_dot}",
        g.commits.len(),
        g.arrows.len(),
        parent_edges,
        config.gitgraph.commit_step,
        g.width,
        g.height
    )
    .unwrap();
    for c in &graph.gitgraph.commits {
        writeln!(head, "  {} lane={:?} type={:?} parents={:?} tags={:?}", c.id, c.branch, c.commit_type, c.parents, c.tags).unwrap();
    }
    Ok(Measured { report: head + &report, tag_overlaps, cross_lane_overlaps })
}

fn parents_of(source: &str, id: &str) -> Vec<String> {
    let mut graph = parse_mermaid(source).unwrap().graph;
    repair(&mut graph).unwrap();
    graph.gitgraph.commits.iter().find(|c| c.id == id).unwrap().parents.clone()
}

fn lane_of(source: &str, id: &str) -> String {
    let graph = parse_mermaid(source).unwrap().graph;
    graph.gitgraph.commits.iter().find(|c| c.id == id).unwrap().branch.clone()
}

fn sha(seed: &str) -> String {
    format!("{:0<40}", seed)
}

/// S2 fixtures, emitted through the real `GitGraph::mermaid()`.
fn s2_fixtures() -> Vec<(&'static str, String)> {
    let c = |s: &str| LaneEntry::Commit(sha(s));
    let alpha = "feature/very-long-exact-branch-reference-alpha";
    let beta = "origin/very-long-exact-branch-reference-beta";
    let mut out = Vec::new();

    // Cross-lane tag rows: long labels on neighboring commits of two lanes,
    // plus main/origin/main one commit apart and a PR tag.
    let cross = GitGraph::new("main", vec![c("d1"), c("d2"), c("d3"), c("d4")])
        .with_ref("main", sha("d4"))
        .with_ref("origin/main", sha("d3"))
        .with_ref(beta, sha("f1"))
        .with_line(GraphLine::new(alpha).forked_at(sha("d2")).with_entries(vec![c("f1"), c("f2")]).with_last_active(10))
        .with_pull_request(GraphPullRequest { number: 104, source_branch: alpha.into(), target_branch: "main".into() });
    out.push(("s2-cross-lane", cross.mermaid().unwrap()));

    // A stack of four tags on one commit beside a neighbor's long tag.
    let stack = GitGraph::new("main", vec![c("d1"), c("d2"), c("d3")])
        .with_ref("main", sha("d3"))
        .with_ref("origin/main", sha("d3"))
        .with_ref("v1.0.0", sha("d3"))
        .with_ref("release/2026-09", sha("d3"))
        .with_ref(alpha, sha("d2"));
    out.push(("s2-stack", stack.mermaid().unwrap()));

    // Tags beside `+N` squares on both lanes.
    let squares = GitGraph::new("main", vec![LaneEntry::Elided(40), c("d1"), LaneEntry::Elided(3), c("d2")])
        .with_ref("main", sha("d2"))
        .with_ref("origin/main", sha("d1"))
        .with_line(
            GraphLine::new(alpha)
                .forked_at(sha("d1"))
                .with_entries(vec![LaneEntry::Elided(12), c("f1")]),
        )
        .with_pull_request(GraphPullRequest { number: 104, source_branch: alpha.into(), target_branch: "main".into() });
    out.push(("s2-squares", squares.mermaid().unwrap()));

    // Base view with several lanes, each tip carrying a PR tag, and a tall
    // stack on a lower lane below an upper lane's tag.
    let mut base = GitGraph::new("main", vec![c("d1"), c("d2"), c("d3"), c("d4"), c("d5")])
        .with_ref("main", sha("d5"))
        .with_ref("origin/main", sha("d4"));
    for (i, fork) in ["d1", "d2", "d3"].into_iter().enumerate() {
        let branch = format!("fix/very-long-worktree-name-number-{i}");
        let tip = format!("b{i}t");
        base = base
            .with_line(GraphLine::new(&branch).forked_at(sha(fork)).with_entries(vec![c(&format!("b{i}a")), c(&tip)]))
            .with_pull_request(GraphPullRequest { number: 200 + i as u64, source_branch: branch.clone(), target_branch: "main".into() })
            .with_ref(format!("origin/{branch}"), sha(&tip))
            .with_ref(format!("v{i}-tag"), sha(&tip));
    }
    out.push(("s2-base-lanes", base.mermaid().unwrap()));
    out
}

fn main() {
    let out = std::env::args().nth(1).expect("output directory");
    let out = Path::new(&out);
    fs::create_dir_all(out).unwrap();
    let mut report = String::new();
    let light = Theme::mermaid_default();
    let dark = Theme::modern();

    // ---- S1: topologies GitGraph will emit ----
    let disconnected = "gitGraph\n    branch lost\n    checkout main\n    commit id: \"A\"\n    commit id: \"B\" tag: \"main\"\n    checkout lost\n    commit id: \"L1\"\n    commit id: \"L2\" tag: \"PR #9 → main\"\n    checkout main\n    commit id: \"C\" tag: \"origin/main\"\n";
    let nested = "gitGraph\n    commit id: \"D1\"\n    branch fix/wt-ux\n    checkout fix/wt-ux\n    commit id: \"W1\"\n    commit id: \"W2\" tag: \"fix/wt-ux-tip\"\n    branch fix/sniff\n    checkout fix/sniff\n    commit id: \"+96\" type: HIGHLIGHT\n    commit id: \"S1\"\n    checkout fix/wt-ux\n    checkout main\n    merge fix/wt-ux id: \"M103\" tag: \"main\"\n    merge fix/sniff id: \"M104\" tag: \"origin/main\"\n";
    let into_parent = "gitGraph\n    commit id: \"A\"\n    branch parent\n    checkout parent\n    commit id: \"P1\"\n    branch child\n    checkout child\n    commit id: \"C1\"\n    commit id: \"C2\"\n    checkout parent\n    merge child id: \"PM\" tag: \"parent-merge\"\n    commit id: \"P2\"\n    checkout main\n    commit id: \"B\" tag: \"main\"\n";
    let merge_forks = "gitGraph\n    commit id: \"A\"\n    branch side\n    checkout side\n    commit id: \"S1\"\n    checkout main\n    merge side id: \"M\" tag: \"origin/main\"\n    branch later\n    checkout later\n    commit id: \"L1\" tag: \"later-tip\"\n    checkout main\n    commit id: \"N\" tag: \"main\"\n";
    let squares = "gitGraph\n    commit id: \"F\"\n    branch side\n    checkout side\n    commit id: \"+5\" type: HIGHLIGHT\n    commit id: \"T\"\n    checkout main\n    commit id: \"+7\" type: HIGHLIGHT\n    merge side id: \"M\" tag: \"origin/main\"\n    commit id: \"+3 \" type: HIGHLIGHT\n    commit id: \"N\" tag: \"main\"\n";
    let unlabeled = "gitGraph\n    commit id: \"A\"\n    branch side\n    checkout side\n    commit id: \"S1\"\n    checkout main\n    merge side\n";
    let headless_dest = "gitGraph\n    branch side\n    checkout side\n    commit id: \"S1\"\n    checkout main\n    merge side\n    commit id: \"A\"\n";
    let ambiguous = "gitGraph\n    commit id: \"A\"\n    branch x\n    checkout x\n    commit id: \"X1\"\n    checkout main\n    branch x tag: \"t\"\n    commit id: \"Y1\"\n    checkout main\n    merge x tag: \"t\" id: \"M\"\n";
    let prefix_only = "gitGraph\n    commit id: \"A\"\n    branch feat\n    checkout feat\n    commit id: \"F1\"\n    checkout main\n    branch feat x\n    commit id: \"G1\"\n    checkout main\n    merge feat x id: \"M\"\n";
    let unknown_lane = "gitGraph\n    commit id: \"A\"\n    checkout main\n    merge nowhere id: \"M\"\n";
    let empty_source = "gitGraph\n    commit id: \"A\"\n    branch side\n    checkout main\n    commit id: \"B\"\n    merge side id: \"M\"\n";

    writeln!(report, "== S1: renderer topologies ==").unwrap();
    for (name, source) in [
        ("s1-disconnected", disconnected),
        ("s1-nested-into-root", nested),
        ("s1-into-parent-lane", into_parent),
        ("s1-merge-is-fork", merge_forks),
        ("s1-squares-both-sides", squares),
        ("s1-unlabeled", unlabeled),
        ("s1-headless-destination", headless_dest),
    ] {
        for (fixed, suffix) in [(false, "raw"), (true, "fixed")] {
            match analyze(out, &format!("{name}-{suffix}"), source, fixed, &light) {
                Ok(m) => report.push_str(&m.report),
                Err(e) => writeln!(report, "{name}-{suffix}: ERROR {e}").unwrap(),
            }
        }
    }
    for (name, source) in [("s1-ambiguous", ambiguous), ("s1-prefix-feat-vs-feat-x", prefix_only), ("s1-unknown-lane", unknown_lane), ("s1-empty-source", empty_source)] {
        let mut graph = parse_mermaid(source).unwrap().graph;
        let parsed = format!("{:?}", graph.gitgraph.commits.iter().map(|c| (&c.id, &c.parents)).collect::<Vec<_>>());
        let repaired = repair(&mut graph);
        writeln!(report, "{name}: parsed parents={parsed} repair={repaired:?}").unwrap();
    }

    // Structural assertions (fail loudly).
    assert_eq!(parents_of(disconnected, "L1"), Vec::<String>::new());
    assert_eq!(lane_of(disconnected, "L1"), "lost");
    assert_eq!(parents_of(nested, "M103"), ["D1", "W2"]);
    assert_eq!(parents_of(nested, "M104"), ["M103", "S1"]);
    assert_eq!(parents_of(nested, "+96"), ["W2"]);
    assert_eq!(lane_of(nested, "S1"), "fix/sniff");
    assert_eq!(parents_of(into_parent, "PM"), ["P1", "C2"]);
    assert_eq!(lane_of(into_parent, "PM"), "parent");
    assert_eq!(parents_of(merge_forks, "M"), ["A", "S1"]);
    assert_eq!(parents_of(merge_forks, "L1"), ["M"]);
    assert_eq!(parents_of(squares, "M"), ["+7", "T"]);
    assert_eq!(parents_of(squares, "+3 "), ["M"]);
    assert_eq!(parents_of(squares, "T"), ["+5"]);

    // ---- S2: tag geometry on real GitGraph output ----
    writeln!(report, "\n== S2: tag geometry (R3 prototype, TAG_GAP = theme font_size) ==").unwrap();
    let mut s2_total = 0;
    for (name, source) in s2_fixtures() {
        for (theme_name, theme) in [("light", &light), ("dark", &dark)] {
            for (fixed, suffix) in [(false, "default"), (true, "spaced")] {
                let m = analyze(out, &format!("{name}-{theme_name}-{suffix}"), &source, fixed, theme).unwrap();
                report.push_str(&m.report);
                if fixed {
                    s2_total += m.tag_overlaps;
                    if m.cross_lane_overlaps > 0 {
                        writeln!(report, "  CROSS-LANE OVERLAP REMAINS").unwrap();
                    }
                }
            }
        }
    }
    writeln!(report, "\nS2 spaced tag overlaps across all fixtures and themes: {s2_total}").unwrap();

    fs::write(out.join("results.txt"), &report).unwrap();
    print!("{report}");
}
