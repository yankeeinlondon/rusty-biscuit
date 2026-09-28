//! Spike S1 (throwaway; see `run.sh`): is gitGraph label geometry linear in
//! `commit_step`, how does a right-to-left gitGraph lay out, and what step
//! does the collision rule (plan R1-R3) choose compared with the widest-tag
//! rule?
//!
//! For each case and theme it lays out at steps 34, 50, and 79 with
//! `mermaid-rs-renderer` directly (the same `compute_layout` biscuit-visualized
//! calls), prints each commit's placement index, x, and tag bounds, checks
//! every x and tag edge against `a + index * step`, computes the R3 step,
//! lays out again at it, and reports the remaining minimum clearance.

use std::fs;

use mermaid_rs_renderer::layout::DiagramData;
use mermaid_rs_renderer::{Direction, LayoutConfig, Theme};

const ALPHA: &str = "feature/very-long-exact-branch-reference-alpha";
const BETA: &str = "origin/very-long-exact-branch-reference-beta";
const TAG_GAP_EM: f32 = 1.0;
const STEPS: [f32; 3] = [34.0, 50.0, 79.0];
const EPSILON: f32 = 0.01;

#[derive(Debug, Clone, Copy)]
struct Bounds {
    x1: f32,
    y1: f32,
    x2: f32,
    y2: f32,
}

fn bounds(points: &[(f32, f32)]) -> Bounds {
    points.iter().fold(
        Bounds { x1: f32::INFINITY, y1: f32::INFINITY, x2: f32::NEG_INFINITY, y2: f32::NEG_INFINITY },
        |b, &(x, y)| Bounds { x1: b.x1.min(x), y1: b.y1.min(y), x2: b.x2.max(x), y2: b.y2.max(y) },
    )
}

/// One tag as laid out: its commit's placement index and x, and its bounds.
#[derive(Debug, Clone)]
struct Labeled {
    index: usize,
    commit: String,
    commit_x: f32,
    text: String,
    bounds: Bounds,
}

struct Laid {
    direction: Direction,
    commits: Vec<(usize, String, f32, f32)>,
    tags: Vec<Labeled>,
    width: f32,
}

fn lay_out(text: &str, theme: &Theme, step: f32, direction: Option<Direction>) -> Laid {
    let mut parsed = mermaid_rs_renderer::parse_mermaid(text).expect("parse");
    if let Some(direction) = direction {
        parsed.graph.direction = direction;
    }
    let mut config = LayoutConfig::default();
    config.gitgraph.commit_step = step;
    let layout = mermaid_rs_renderer::compute_layout(&parsed.graph, theme, &config);
    let DiagramData::GitGraph(gitgraph) = &layout.diagram else { panic!("not a gitGraph") };
    let mut tags = Vec::new();
    let mut commits = Vec::new();
    for (index, commit) in gitgraph.commits.iter().enumerate() {
        commits.push((index, commit.id.clone(), commit.x, commit.y));
        for tag in &commit.tags {
            assert!(tag.transform.is_none(), "rotated tag");
            tags.push(Labeled { index, commit: commit.id.clone(), commit_x: commit.x, text: tag.text.clone(), bounds: bounds(&tag.points) });
        }
    }
    Laid { direction: gitgraph.direction, commits, tags, width: gitgraph.width }
}

/// Plan R1-R3: pairs of tags on different commits whose vertical interiors
/// overlap; the earlier is the one whose commit has the smaller x.
fn required_step(default: f32, gap: f32, tags: &[Labeled]) -> Option<f32> {
    let mut chosen: Option<f32> = None;
    for (i, a) in tags.iter().enumerate() {
        for b in &tags[i + 1..] {
            if a.index == b.index || !(a.bounds.y1 < b.bounds.y2 && b.bounds.y1 < a.bounds.y2) {
                continue;
            }
            let (earlier, later) = if a.commit_x <= b.commit_x { (a, b) } else { (b, a) };
            let clearance = later.bounds.x1 - earlier.bounds.x2;
            let shortfall = gap - clearance;
            if shortfall <= EPSILON {
                continue;
            }
            let distance = a.index.abs_diff(b.index) as f32;
            let step = default + shortfall / distance;
            chosen = Some(chosen.map_or(step, |c| c.max(step)));
        }
    }
    chosen
}

/// The smallest clearance over counting pairs, and whether any pair of tags
/// on different commits intersects at all.
fn min_clearance(tags: &[Labeled]) -> (Option<f32>, bool) {
    let mut min: Option<f32> = None;
    let mut overlap = false;
    for (i, a) in tags.iter().enumerate() {
        for b in &tags[i + 1..] {
            if a.index == b.index {
                continue;
            }
            let vertical = a.bounds.y1 < b.bounds.y2 && b.bounds.y1 < a.bounds.y2;
            if vertical && a.bounds.x1 < b.bounds.x2 && b.bounds.x1 < a.bounds.x2 {
                overlap = true;
            }
            if !vertical {
                continue;
            }
            let (earlier, later) = if a.commit_x <= b.commit_x { (a, b) } else { (b, a) };
            let clearance = later.bounds.x1 - earlier.bounds.x2;
            min = Some(min.map_or(clearance, |m: f32| m.min(clearance)));
        }
    }
    (min, overlap)
}

fn gitgraph(lines: &[&str]) -> String {
    let mut text = String::from("gitGraph\n");
    for line in lines {
        text.push_str("    ");
        text.push_str(line);
        text.push('\n');
    }
    text
}

fn cases(observed: &str) -> Vec<(&'static str, String, Option<Direction>)> {
    let isolated = gitgraph(&[
        "commit id: \"d1\"",
        "commit id: \"d2\"",
        "commit id: \"d3\"",
        "commit id: \"d4\" tag: \"main\" tag: \"origin/main\"",
    ]);
    let one_apart = gitgraph(&["commit id: \"d1\"", "commit id: \"d2\" tag: \"main\"", "commit id: \"d3\" tag: \"origin/main\""]);
    let cross_lane_unequal = gitgraph(&[
        "commit id: \"d1\"",
        "commit id: \"d2\"",
        "branch feature",
        "checkout feature",
        &format!("commit id: \"f1\" tag: \"{BETA}\""),
        "checkout main",
        "commit id: \"d3\" tag: \"v1\"",
        "commit id: \"d4\" tag: \"main\"",
    ]);
    let neighbors_unequal = gitgraph(&[
        "commit id: \"a1\"",
        &format!("commit id: \"a2\" tag: \"{ALPHA}\""),
        "commit id: \"a3\" tag: \"v1\"",
        "commit id: \"a4\"",
        "commit id: \"a5\" tag: \"PR #104 → main\"",
    ]);
    let stack = gitgraph(&[
        "commit id: \"d1\"",
        &format!("commit id: \"d2\" tag: \"{ALPHA}\""),
        "commit id: \"d3\" tag: \"main\" tag: \"origin/main\" tag: \"release/2026-09\"",
        "commit id: \"d4\" tag: \"v1.0.0\"",
    ]);
    let rl_line = format!("gitGraph\n    direction RL\n{}", &one_apart["gitGraph\n".len()..]);
    vec![
        ("isolated-long-tag-stack", isolated, None),
        ("main-origin-one-apart", one_apart.clone(), None),
        ("cross-lane-unequal", cross_lane_unequal.clone(), None),
        ("neighbors-unequal", neighbors_unequal, None),
        ("stack-of-three", stack, None),
        ("rl-text-direction-line", rl_line, None),
        ("rl-programmatic-one-apart", one_apart, Some(Direction::RightLeft)),
        ("rl-programmatic-cross-lane", cross_lane_unequal, Some(Direction::RightLeft)),
        ("observed", observed.to_string(), None),
    ]
}

fn main() {
    let observed_path = std::env::args().nth(1).expect("observed.mmd path");
    let observed = fs::read_to_string(&observed_path).expect("read observed.mmd");
    let mut large = Theme::mermaid_default();
    large.font_size = 24.0;
    let themes = [("default", Theme::mermaid_default()), ("font-24", large)];
    let default_step = LayoutConfig::default().gitgraph.commit_step;
    let offset = LayoutConfig::default().gitgraph.layout_offset;
    println!("renderer default commit_step={default_step} layout_offset={offset} parallel_commits={}", LayoutConfig::default().gitgraph.parallel_commits);

    for (name, text, direction) in cases(&observed) {
        for (theme_name, theme) in &themes {
            let gap = TAG_GAP_EM * theme.font_size;
            let runs: Vec<Laid> = STEPS.iter().map(|&step| lay_out(&text, theme, step, direction)).collect();
            println!("\n== {name} [{theme_name}, font {}] direction={:?}", theme.font_size, runs[0].direction);

            // Linearity: every commit x and tag edge moves by index * (step delta).
            let mut worst: f32 = 0.0;
            for pair in [(0, 1), (0, 2)] {
                let delta = STEPS[pair.1] - STEPS[pair.0];
                for (a, b) in runs[pair.0].commits.iter().zip(&runs[pair.1].commits) {
                    worst = worst.max(((b.2 - a.2) - a.0 as f32 * delta).abs());
                }
                for (a, b) in runs[pair.0].tags.iter().zip(&runs[pair.1].tags) {
                    worst = worst.max(((b.bounds.x1 - a.bounds.x1) - a.index as f32 * delta).abs());
                    worst = worst.max(((b.bounds.x2 - a.bounds.x2) - a.index as f32 * delta).abs());
                    worst = worst.max((b.bounds.y1 - a.bounds.y1).abs()).max((b.bounds.y2 - a.bounds.y2).abs());
                }
            }
            println!("linearity: worst deviation from a + index*step = {worst:.4}");
            let first = &runs[0];
            let monotone = first.commits.windows(2).all(|w| w[0].2 < w[1].2);
            println!("commit x increases with placement index: {monotone}");
            for tag in &first.tags {
                println!(
                    "  step34 tag {:>45} on {:>8} index {:>2} commit_x {:>8.2} bounds x {:>8.2}..{:>8.2} y {:>7.2}..{:>7.2} (x1-commit_x {:>7.2})",
                    tag.text, tag.commit, tag.index, tag.commit_x, tag.bounds.x1, tag.bounds.x2, tag.bounds.y1, tag.bounds.y2, tag.bounds.x1 - tag.commit_x
                );
            }

            let widest = first.tags.iter().map(|t| t.bounds.x2 - t.bounds.x1).fold(0f32, f32::max);
            let old = if first.tags.is_empty() { default_step } else { default_step.max(widest + gap) };
            let (clear_before, overlap_before) = min_clearance(&first.tags);
            let chosen = required_step(default_step, gap, &first.tags);
            let step = chosen.unwrap_or(default_step);
            let second = lay_out(&text, theme, step, direction);
            let (clear_after, overlap_after) = min_clearance(&second.tags);
            let remaining = required_step(step, gap, &second.tags);
            println!(
                "old widest-tag step {old:.3} | R3 step {} | default: min clearance {:?} overlap {overlap_before} width {:.1} | at R3 step: min clearance {:?} (gap {gap}) overlap {overlap_after} width {:.1} | second-pass shortfall {:?}",
                chosen.map_or("default".to_string(), |s| format!("{s:.3}")),
                clear_before.map(|c| (c * 100.0).round() / 100.0),
                first.width,
                clear_after.map(|c| (c * 100.0).round() / 100.0),
                second.width,
                remaining,
            );
        }
    }
}
