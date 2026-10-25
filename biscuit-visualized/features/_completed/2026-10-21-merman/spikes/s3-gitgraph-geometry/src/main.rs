//! Spike S3 (R-S8, AC-S6): build `GitGraphGeometry` from Merman's layout data
//! and replay the geometry `worktree`'s git graph tests read.
//!
//! `inputs/` holds every `geometry_of` call those tests made (31 calls from 14
//! tests), captured from a passing run of `worktree-cli`'s suite on today's
//! `mermaid-rs-renderer` path: the exact Mermaid text and the geometry it gave.
//! For each input this spike lays the text out with Merman, rebuilds the
//! geometry from `layout.GitGraphDiagram` (tag bounds from the SVG's
//! `tag-label-bkg` polygons, as the spike plan allows), and checks every
//! property a `worktree` assertion reads: commit ids and order, lanes, parents,
//! tag texts per commit, tag overlaps, and the commit step.
//!
//! Usage: `s3-gitgraph-geometry [--out <dir>]` (default `out/`).

use std::collections::BTreeMap;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use merman::{OperationControl, RenderOutput, RenderRequest, Renderer, SvgRequest};
use serde::Deserialize;
use serde_json::{Value, json};

/// The clearance the captured geometry reports (`tag_gap`): one em at 16 px.
const TAG_GAP: f64 = 16.0;
/// Merman's fixed gitGraph step: `COMMIT_STEP` (40) plus `LAYOUT_OFFSET` (10)
/// in `merman-render/src/gitgraph.rs`. No config key changes it.
const MERMAN_STEP: f64 = 50.0;
/// Clearance below which a pair still counts as clear (float noise).
const EPSILON: f64 = 0.01;

#[derive(Deserialize)]
struct Captured {
    test: String,
    call: usize,
    mermaid: String,
    commit_step: f64,
    default_commit_step: f64,
    width: f64,
    commits: Vec<CapturedCommit>,
}

#[derive(Deserialize)]
struct CapturedCommit {
    id: String,
    lane: String,
    parents: Vec<String>,
    tags: Vec<CapturedTag>,
}

#[derive(Deserialize)]
struct CapturedTag {
    text: String,
}

/// The subset of Merman's `layout.GitGraphDiagram` the geometry needs. Merman's
/// typed structs are not re-exported by the facade, so these are our own.
#[derive(Deserialize)]
struct LayoutGitGraph {
    direction: String,
    bounds: LayoutBounds,
    commits: Vec<LayoutCommit>,
}

#[derive(Deserialize)]
struct LayoutBounds {
    min_x: f64,
    min_y: f64,
    max_x: f64,
    max_y: f64,
}

#[derive(Deserialize)]
struct LayoutCommit {
    id: String,
    parents: Vec<String>,
    branch: String,
    pos: f64,
    x: f64,
    tags: Vec<String>,
}

#[derive(Debug, Clone, Copy)]
struct Bounds {
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
}

impl Bounds {
    fn overlaps(&self, other: &Bounds) -> bool {
        self.x1 < other.x2 && other.x1 < self.x2 && self.y1 < other.y2 && other.y1 < self.y2
    }

    fn shares_rows(&self, other: &Bounds) -> bool {
        self.y1 < other.y2 && other.y1 < self.y2
    }
}

/// The equivalent of `biscuit_visualized::mermaid::GitGraphGeometry`.
struct Geometry {
    commits: Vec<CommitGeometry>,
    commit_step: f64,
    width: f64,
    height: f64,
}

struct CommitGeometry {
    id: String,
    index: usize,
    x: f64,
    lane: String,
    parents: Vec<String>,
    tags: Vec<(String, Bounds)>,
}

impl Geometry {
    fn tag_boxes(&self) -> Vec<(usize, &str, Bounds)> {
        self.commits
            .iter()
            .flat_map(|commit| commit.tags.iter().map(move |(text, bounds)| (commit.index, text.as_str(), *bounds)))
            .collect()
    }

    fn tag_overlaps(&self) -> Vec<(String, String)> {
        let boxes = self.tag_boxes();
        let mut overlaps = Vec::new();
        for (index, (_, text, bounds)) in boxes.iter().enumerate() {
            for (_, other_text, other_bounds) in &boxes[index + 1..] {
                if bounds.overlaps(other_bounds) {
                    overlaps.push((text.to_string(), other_text.to_string()));
                }
            }
        }
        overlaps
    }

    /// The D36 model: the smallest diagram-wide step that gives every pair of
    /// tags on different commits that share rows at least `TAG_GAP`. A tag's
    /// x moves with its commit's `pos`, which is `index * step` in LR.
    fn required_step(&self) -> f64 {
        let boxes = self.tag_boxes();
        let mut step = self.commit_step;
        for (i, (left_index, _, left)) in boxes.iter().enumerate() {
            for (right_index, _, right) in &boxes[i + 1..] {
                if left_index == right_index || !left.shares_rows(right) {
                    continue;
                }
                let (a, b, k) = if left_index < right_index {
                    (left, right, (right_index - left_index) as f64)
                } else {
                    (right, left, (left_index - right_index) as f64)
                };
                let gap = b.x1 - a.x2;
                if gap < TAG_GAP - EPSILON {
                    step = step.max(self.commit_step + (TAG_GAP - gap) / k);
                }
            }
        }
        step
    }
}

fn layout_of(source: &str) -> Result<LayoutGitGraph, Box<dyn Error>> {
    let output = Renderer::new().render(RenderRequest::layout_json(source, OperationControl::new(), SvgRequest::default()))?;
    let RenderOutput::LayoutJson(Some(layout)) = output else {
        return Err("no layout".into());
    };
    let mut bytes = Vec::new();
    layout.layout().write_json_pretty(&mut bytes)?;
    let value: Value = serde_json::from_slice(&bytes)?;
    let gitgraph = value
        .pointer("/layout/GitGraphDiagram")
        .ok_or("layout has no GitGraphDiagram")?
        .clone();
    Ok(serde_json::from_value(gitgraph)?)
}

fn svg_of(source: &str) -> Result<String, Box<dyn Error>> {
    let output = Renderer::new().render(RenderRequest::svg(source, OperationControl::new(), SvgRequest::default()))?;
    let RenderOutput::Svg(Some(svg)) = output else {
        return Err("no svg".into());
    };
    Ok(svg.svg().to_string())
}

fn unescape(text: &str) -> String {
    text.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&amp;", "&")
}

/// Every LR tag in drawing order: its text and the bounds of its background
/// polygon. A polygon with a `transform` (TB/BT) is reported as an error.
fn svg_tags(svg: &str) -> Result<Vec<(String, Bounds)>, Box<dyn Error>> {
    const POLYGON: &str = r#"<polygon class="tag-label-bkg" points=""#;
    let mut tags = Vec::new();
    let mut rest = svg;
    while let Some(start) = rest.find(POLYGON) {
        rest = &rest[start + POLYGON.len()..];
        let end = rest.find('"').ok_or("unterminated points")?;
        let numbers: Vec<f64> = rest[..end].split([' ', ',']).filter(|n| !n.is_empty()).map(str::parse).collect::<Result<_, _>>()?;
        if rest[end..].starts_with("\" transform") {
            return Err("rotated tag polygon (TB/BT) is not handled by this spike".into());
        }
        let points: Vec<(f64, f64)> = numbers.chunks(2).map(|pair| (pair[0], pair[1])).collect();
        let bounds = points.iter().fold(
            Bounds { x1: f64::INFINITY, y1: f64::INFINITY, x2: f64::NEG_INFINITY, y2: f64::NEG_INFINITY },
            |b, &(x, y)| Bounds { x1: b.x1.min(x), y1: b.y1.min(y), x2: b.x2.max(x), y2: b.y2.max(y) },
        );
        let label = rest.find(r#"class="tag-label""#).ok_or("tag polygon without label")?;
        rest = &rest[label..];
        let open = rest.find('>').ok_or("unterminated label")? + 1;
        let close = rest.find("</text>").ok_or("unterminated label text")?;
        tags.push((unescape(&rest[open..close]), bounds));
        rest = &rest[close..];
    }
    Ok(tags)
}

fn geometry_of(source: &str) -> Result<(Geometry, String), Box<dyn Error>> {
    let layout = layout_of(source)?;
    if layout.direction != "LR" {
        return Err(format!("direction {} not handled", layout.direction).into());
    }
    let mut drawn = svg_tags(&svg_of(source)?)?.into_iter();
    let mut commits = Vec::new();
    for (index, laid) in layout.commits.iter().enumerate() {
        let tags: Vec<(String, Bounds)> = drawn.by_ref().take(laid.tags.len()).collect();
        let mut drawn_texts: Vec<&str> = tags.iter().map(|(text, _)| text.as_str()).collect();
        let mut laid_texts: Vec<&str> = laid.tags.iter().map(String::as_str).collect();
        drawn_texts.sort_unstable();
        laid_texts.sort_unstable();
        if drawn_texts != laid_texts {
            return Err(format!("tags of {}: layout {laid_texts:?}, svg {drawn_texts:?}", laid.id).into());
        }
        commits.push(CommitGeometry {
            id: laid.id.clone(),
            index,
            x: laid.x,
            lane: laid.branch.clone(),
            parents: laid.parents.clone(),
            tags,
        });
    }
    if drawn.next().is_some() {
        return Err("more tag polygons than layout tags".into());
    }
    // No `commit_step` field: the step is the spacing of consecutive `pos`.
    let commit_step = layout
        .commits
        .windows(2)
        .map(|pair| pair[1].pos - pair[0].pos)
        .filter(|delta| *delta > 0.0)
        .fold(f64::INFINITY, f64::min);
    let commit_step = if commit_step.is_finite() { commit_step } else { MERMAN_STEP };
    let ordered = commits.windows(2).all(|pair| pair[0].x < pair[1].x);
    let geometry = Geometry {
        commits,
        commit_step,
        width: layout.bounds.max_x - layout.bounds.min_x,
        height: layout.bounds.max_y - layout.bounds.min_y,
    };
    Ok((geometry, if ordered { "x increases with index".into() } else { "x NOT monotonic".into() }))
}

/// Which geometry-reading assertion groups each captured test contains.
/// S: ids, order, lanes, parents, tag texts. O: `tag_overlaps()` is empty.
/// P: `commit_step == default_gitgraph_commit_step()` (tests.rs:2193).
fn groups(test: &str) -> &'static str {
    match test.trim_start_matches("commands__git_graph__tests__") {
        t if t.starts_with("gathered_graph_lays_out_") => "SO",
        "the_observed_graph_keeps_recent_commits_on_every_lane_at_200x60" => "SOP",
        _ => "S",
    }
}

fn compare(captured: &Captured) -> Result<Value, Box<dyn Error>> {
    let (geometry, order) = geometry_of(&captured.mermaid)?;
    let mut structure = Vec::new();
    let merman_ids: Vec<&str> = geometry.commits.iter().map(|c| c.id.as_str()).collect();
    let captured_ids: Vec<&str> = captured.commits.iter().map(|c| c.id.as_str()).collect();
    if merman_ids != captured_ids {
        structure.push(format!("commit ids/order differ: merman {merman_ids:?} vs captured {captured_ids:?}"));
    }
    let by_id: BTreeMap<&str, &CommitGeometry> = geometry.commits.iter().map(|c| (c.id.as_str(), c)).collect();
    for expected in &captured.commits {
        let Some(actual) = by_id.get(expected.id.as_str()) else { continue };
        if actual.lane != expected.lane {
            structure.push(format!("{}: lane {} vs {}", expected.id, actual.lane, expected.lane));
        }
        if actual.parents != expected.parents {
            structure.push(format!("{}: parents {:?} vs {:?}", expected.id, actual.parents, expected.parents));
        }
        let mut a: Vec<&str> = actual.tags.iter().map(|(t, _)| t.as_str()).collect();
        let mut e: Vec<&str> = expected.tags.iter().map(|t| t.text.as_str()).collect();
        a.sort_unstable();
        e.sort_unstable();
        if a != e {
            structure.push(format!("{}: tags {a:?} vs {e:?}", expected.id));
        }
    }
    let overlaps = geometry.tag_overlaps();
    let required_step = geometry.required_step();
    let groups = groups(&captured.test);
    let step_holds_unspaced = geometry.commit_step == MERMAN_STEP;
    let step_holds_d36 = required_step == MERMAN_STEP;
    let mut failing = Vec::new();
    if !structure.is_empty() {
        failing.push("S");
    }
    if groups.contains('O') && !overlaps.is_empty() {
        failing.push("O (fixed step)");
    }
    if groups.contains('P') && !step_holds_d36 {
        failing.push("P (after D36 widening)");
    }
    Ok(json!({
        "test": captured.test,
        "call": captured.call,
        "groups": groups,
        "structure_differences": structure,
        "order": order,
        "merman_step": geometry.commit_step,
        "merman_required_step_d36": required_step,
        "mmdr_step": captured.commit_step,
        "mmdr_default_step": captured.default_commit_step,
        "mmdr_widened": captured.commit_step > captured.default_commit_step,
        "merman_tag_overlaps_fixed_step": overlaps,
        "line_2193_holds_fixed_step": step_holds_unspaced,
        "line_2193_holds_after_d36": step_holds_d36,
        "merman_width": geometry.width,
        "merman_height": geometry.height,
        "mmdr_width": captured.width,
        "first_x": geometry.commits.first().map(|c| c.x),
        "failing_groups": failing,
    }))
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut out = PathBuf::from("out");
    let mut args = std::env::args().skip(1);
    while let Some(flag) = args.next() {
        match flag.as_str() {
            "--out" => out = PathBuf::from(args.next().ok_or("--out needs a value")?),
            other => return Err(format!("unknown argument {other}").into()),
        }
    }
    let inputs = Path::new(env!("CARGO_MANIFEST_DIR")).join("inputs");
    let mut paths: Vec<PathBuf> = fs::read_dir(&inputs)?.map(|e| e.map(|e| e.path())).collect::<Result<_, _>>()?;
    paths.sort();
    let mut rows = Vec::new();
    for path in &paths {
        let captured: Captured = serde_json::from_str(&fs::read_to_string(path)?)?;
        let row = compare(&captured).unwrap_or_else(|error| {
            json!({"test": captured.test, "call": captured.call, "error": error.to_string(), "failing_groups": ["error"]})
        });
        println!(
            "{:<88} {:<4} fail={:<28} step mmdr={:>6.1} merman={:>4.0} d36={:>6.1} overlaps={}",
            format!("{}#{}", captured.test, captured.call),
            row["groups"].as_str().unwrap_or("-"),
            row["failing_groups"].to_string(),
            captured.commit_step,
            row["merman_step"].as_f64().unwrap_or(f64::NAN),
            row["merman_required_step_d36"].as_f64().unwrap_or(f64::NAN),
            row["merman_tag_overlaps_fixed_step"].as_array().map_or(0, Vec::len),
        );
        if let Some(error) = row.get("error") {
            println!("    error: {error}");
        }
        for difference in row["structure_differences"].as_array().into_iter().flatten() {
            println!("    {difference}");
        }
        rows.push(row);
    }
    fs::create_dir_all(&out)?;
    fs::write(out.join("results.json"), serde_json::to_vec_pretty(&rows)?)?;
    Ok(())
}
