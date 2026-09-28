//! gitGraph corrections applied between parsing and layout.
//!
//! Both run inside `MermaidDiagram::compute_layout`, which measurement
//! (`natural_size`) and rendering (`render_svg`) share, so a measured size is
//! always the size of the image drawn. Neither adds a rendering input: each is
//! derived from the instructions and the resolved theme, which the artifact
//! cache key already covers (with the backend id bumped when they changed).
//!
//! - **Merge repair.** `mermaid-rs-renderer` 0.3.1 looks up everything after
//!   `merge` as the source lane's name, attributes included, so a labeled merge
//!   (`merge feat id: "M" tag: "v1"`) finds no lane and keeps only its first
//!   parent. [`repair_gitgraph_merges`] restores the second parent.
//! - **Tag spacing.** The renderer places a tag above its commit with no
//!   collision avoidance and spaces commits a fixed `commit_step` apart, so a
//!   tag wider than that step can cover a neighbor's. [`layout`] lays out
//!   again with the smallest step that separates every colliding pair by one
//!   em, and only when such a pair exists.

use mermaid_rs_renderer::ir::GitGraphCommitType;
use mermaid_rs_renderer::layout::{DiagramData, GitGraphLayout};
use mermaid_rs_renderer::{Direction, Graph, Layout, LayoutConfig, Theme};

use super::error::MermaidError;

/// The gap between neighboring tags, in ems of the theme's font size.
///
/// A readability policy, not a measured margin: one em keeps two adjacent
/// labels visibly separate at every theme's font size.
pub(crate) const TAG_GAP_EM: f32 = 1.0;

/// The IR message the renderer's parser gives every merge commit:
/// `merged branch {suffix} into {current lane}`. Pinned by a test, because the
/// repair recovers the `merge` statement's suffix from it.
const MERGE_MESSAGE_PREFIX: &str = "merged branch ";

/// Keys that may follow the source lane's name in a `merge` statement.
const MERGE_ATTRIBUTE_KEYS: [&str; 3] = ["id:", "tag:", "type:"];

/// Gives every labeled merge commit its source lane's tip as second parent.
///
/// A merge with two or more parents is left alone, so this is a no-op once the
/// dependency parses attributes itself. So is a merge whose single parent
/// already is the source tip, which is the parser's shape for a merge into a
/// lane with no head of its own.
///
/// ## Errors
///
/// `MermaidError::RenderFailed` when a merge's source lane cannot be named
/// exactly (no declared lane matches, or more than one does), or the source
/// lane has no commit before the merge (mermaid.js refuses to merge an empty
/// branch too). A merge is never drawn without its source.
pub(crate) fn repair_gitgraph_merges(graph: &mut Graph) -> Result<(), MermaidError> {
    let failed = |id: &str, reason: String| MermaidError::RenderFailed(format!("gitGraph merge \"{id}\": {reason}"));
    for index in 0..graph.gitgraph.commits.len() {
        let commit = &graph.gitgraph.commits[index];
        if commit.commit_type != GitGraphCommitType::Merge || commit.parents.len() >= 2 {
            continue;
        }
        let message = commit.message.as_deref().unwrap_or_default();
        // `rsplit_once`: the suffix itself may contain ` into `.
        let suffix = message
            .strip_prefix(MERGE_MESSAGE_PREFIX)
            .and_then(|rest| rest.rsplit_once(&format!(" into {}", commit.branch)))
            .map(|(suffix, _)| suffix)
            .ok_or_else(|| failed(&commit.id, format!("unrecognized parser message {message:?}")))?;
        let matching: Vec<&str> = graph
            .gitgraph
            .branches
            .iter()
            .map(|branch| branch.name.as_str())
            .filter(|name| names_merge_source(suffix, name))
            .collect();
        let source = match matching.as_slice() {
            [one] => *one,
            [] => return Err(failed(&commit.id, format!("no lane matches {suffix:?}"))),
            many => return Err(failed(&commit.id, format!("{suffix:?} matches several lanes: {many:?}"))),
        };
        let source_tip = graph
            .gitgraph
            .commits
            .iter()
            .filter(|other| other.branch == source && other.seq < commit.seq)
            .max_by_key(|other| other.seq)
            .map(|other| other.id.clone())
            .ok_or_else(|| failed(&commit.id, format!("lane {source:?} has no commit to merge")))?;
        let commit = &mut graph.gitgraph.commits[index];
        if commit.parents.first() != Some(&source_tip) {
            commit.parents.push(source_tip);
        }
    }
    Ok(())
}

/// Whether a `merge` statement's `suffix` names lane `name`: it is the name,
/// or the name followed by whitespace and an attribute key.
fn names_merge_source(suffix: &str, name: &str) -> bool {
    if suffix == name {
        return true;
    }
    suffix.strip_prefix(name).is_some_and(|rest| {
        let attributes = rest.trim_start();
        attributes.len() < rest.len() && MERGE_ATTRIBUTE_KEYS.iter().any(|key| attributes.starts_with(key))
    })
}

/// Label clearance below which a pair still counts as clear, in SVG user
/// units. Absorbs float noise in a pair that was widened to exactly the gap.
pub(crate) const SPACING_EPSILON: f32 = 0.01;

/// Layouts allowed after the first, the fallback's included.
pub(crate) const MAX_SPACING_PASSES: usize = 3;

/// Lays out `graph`, spacing a gitGraph's commits so no two tags collide.
///
/// Only a left-to-right or right-to-left gitGraph with no rotated tag is
/// spaced. Its first pass uses the default `commit_step`; when two tags on
/// different commits share vertical extent and are closer than [`TAG_GAP_EM`],
/// the step becomes the smallest one that gives every such pair that gap
/// ([`required_step`]). Every other diagram, and a spaced one with no
/// colliding pair, keeps the single default pass, so an isolated long label
/// never widens the graph.
pub(crate) fn layout(graph: &Graph, theme: &Theme) -> (Layout, LayoutConfig) {
    let mut config = LayoutConfig::default();
    let (layout, step) = spaced(config.gitgraph.commit_step, TAG_GAP_EM * theme.font_size, |step| {
        let mut config = config.clone();
        config.gitgraph.commit_step = step;
        let layout = mermaid_rs_renderer::compute_layout(graph, theme, &config);
        let labels = gitgraph_of(&layout).and_then(horizontal_labels);
        (layout, labels)
    });
    config.gitgraph.commit_step = step;
    (layout, config)
}

/// The spacing loop over an injected placement, which lays out at a step and
/// returns the labels to space (`None` when the layout is not spaced at all).
///
/// Tag edges move linearly with the step, so the first widening normally
/// verifies. If a collision survives `MAX_SPACING_PASSES - 1` widenings, the
/// last layout uses the widest tag plus the gap, the bound that cannot
/// collide, and is not verified again.
pub(crate) fn spaced<L>(default: f32, gap: f32, mut place: impl FnMut(f32) -> (L, Option<Vec<LabeledCommit>>)) -> (L, f32) {
    let (first, labels) = place(default);
    let Some(labels) = labels else {
        return (first, default);
    };
    let Some(mut step) = required_step(default, gap, &labels) else {
        return (first, default);
    };
    for _ in 1..MAX_SPACING_PASSES {
        let (laid, relaid) = place(step);
        match relaid.and_then(|relaid| required_step(step, gap, &relaid)) {
            None => return (laid, step),
            Some(wider) => step = wider,
        }
    }
    let widest = labels.iter().map(|label| label.bounds.width()).fold(0f32, f32::max);
    let step = step.max(widest + gap);
    (place(step).0, step)
}

/// One tag's label, with its commit's placement index and x.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct LabeledCommit {
    /// The commit's position in the laid-out commit order.
    pub index: usize,
    pub x: f32,
    pub bounds: Bounds,
}

/// The smallest commit step that leaves `gap` between every colliding pair of
/// labels, given labels laid out at `step`; `None` when every pair is clear.
///
/// A pair counts only when its labels are on different commits and their
/// vertical interiors overlap. The earlier label is the one whose commit has
/// the smaller x, and each step unit moves the later label by the two
/// commits' index distance, so a pair short of the gap by `shortfall` needs
/// `step + shortfall / distance`. The result is the maximum over all pairs.
pub(crate) fn required_step(step: f32, gap: f32, labels: &[LabeledCommit]) -> Option<f32> {
    let mut required: Option<f32> = None;
    for (position, a) in labels.iter().enumerate() {
        for b in &labels[position + 1..] {
            if a.index == b.index || !(a.bounds.y1 < b.bounds.y2 && b.bounds.y1 < a.bounds.y2) {
                continue;
            }
            let (earlier, later) = if a.x <= b.x { (a, b) } else { (b, a) };
            let shortfall = gap - (later.bounds.x1 - earlier.bounds.x2);
            if shortfall <= SPACING_EPSILON {
                continue;
            }
            let pair = step + shortfall / a.index.abs_diff(b.index) as f32;
            required = Some(required.map_or(pair, |required| required.max(pair)));
        }
    }
    required
}

fn gitgraph_of(layout: &Layout) -> Option<&GitGraphLayout> {
    match &layout.diagram {
        DiagramData::GitGraph(gitgraph) => Some(gitgraph),
        _ => None,
    }
}

/// Every tag's label in a horizontal gitGraph, or `None` when the graph is
/// vertical or any tag is rotated.
fn horizontal_labels(gitgraph: &GitGraphLayout) -> Option<Vec<LabeledCommit>> {
    if !matches!(gitgraph.direction, Direction::LeftRight | Direction::RightLeft) {
        return None;
    }
    let mut labels = Vec::new();
    for (index, commit) in gitgraph.commits.iter().enumerate() {
        for tag in &commit.tags {
            if tag.transform.is_some() {
                return None;
            }
            labels.push(LabeledCommit {
                index,
                x: commit.x,
                bounds: Bounds::of(&tag.points),
            });
        }
    }
    Some(labels)
}

/// The renderer's default commit spacing, which an unspaced gitGraph keeps.
/// Test support for callers without a `mermaid-rs-renderer` dependency.
#[doc(hidden)]
pub fn default_gitgraph_commit_step() -> f32 {
    LayoutConfig::default().gitgraph.commit_step
}

/// An axis-aligned box in SVG user units.
#[doc(hidden)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bounds {
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
}

#[allow(missing_docs)]
impl Bounds {
    fn of(points: &[(f32, f32)]) -> Self {
        points.iter().fold(
            Bounds {
                x1: f32::INFINITY,
                y1: f32::INFINITY,
                x2: f32::NEG_INFINITY,
                y2: f32::NEG_INFINITY,
            },
            |bounds, &(x, y)| Bounds {
                x1: bounds.x1.min(x),
                y1: bounds.y1.min(y),
                x2: bounds.x2.max(x),
                y2: bounds.y2.max(y),
            },
        )
    }

    pub fn width(&self) -> f32 {
        self.x2 - self.x1
    }

    /// Whether the interiors intersect; shared edges do not count.
    pub fn overlaps(&self, other: &Bounds) -> bool {
        self.x1 < other.x2 && other.x1 < self.x2 && self.y1 < other.y2 && other.y1 < self.y2
    }
}

/// A gitGraph as laid out for rendering: the evidence for merge parents and
/// tag placement that Mermaid text alone cannot give. Test support; not a
/// stable API.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq)]
pub struct GitGraphGeometry {
    /// Commits in layout order.
    pub commits: Vec<CommitGeometry>,
    /// The commit spacing the layout used.
    pub commit_step: f32,
    /// The clearance spacing gives colliding tags: [`TAG_GAP_EM`] of the
    /// theme's font size.
    pub tag_gap: f32,
    pub width: f32,
    pub height: f32,
}

/// One laid-out commit.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq)]
pub struct CommitGeometry {
    pub id: String,
    /// The position in layout order, which the step multiplies.
    pub index: usize,
    pub x: f32,
    /// The lane (branch) name the commit sits on.
    pub lane: String,
    /// Parent IDs after the merge repair.
    pub parents: Vec<String>,
    pub tags: Vec<TagGeometry>,
}

/// One tag's text and the bounds of its drawn label.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq)]
pub struct TagGeometry {
    pub text: String,
    pub bounds: Bounds,
}

#[allow(missing_docs)]
impl GitGraphGeometry {
    pub(crate) fn from_layout(graph: &Graph, layout: &Layout, config: &LayoutConfig, theme: &Theme) -> Option<Self> {
        let gitgraph = gitgraph_of(layout)?;
        let commits = gitgraph
            .commits
            .iter()
            .enumerate()
            .map(|(index, laid)| {
                let parsed = graph.gitgraph.commits.iter().find(|commit| commit.id == laid.id);
                CommitGeometry {
                    id: laid.id.clone(),
                    index,
                    x: laid.x,
                    lane: parsed.map(|commit| commit.branch.clone()).unwrap_or_default(),
                    parents: parsed.map(|commit| commit.parents.clone()).unwrap_or_default(),
                    tags: laid
                        .tags
                        .iter()
                        .map(|tag| TagGeometry {
                            text: tag.text.clone(),
                            bounds: Bounds::of(&tag.points),
                        })
                        .collect(),
                }
            })
            .collect();
        Some(Self {
            commits,
            commit_step: config.gitgraph.commit_step,
            tag_gap: TAG_GAP_EM * theme.font_size,
            width: gitgraph.width,
            height: gitgraph.height,
        })
    }

    pub fn commit(&self, id: &str) -> Option<&CommitGeometry> {
        self.commits.iter().find(|commit| commit.id == id)
    }

    /// Every `(commit id, tag text, bounds)` in layout order.
    pub fn tag_boxes(&self) -> Vec<(&str, &str, Bounds)> {
        self.commits
            .iter()
            .flat_map(|commit| {
                commit
                    .tags
                    .iter()
                    .map(move |tag| (commit.id.as_str(), tag.text.as_str(), tag.bounds))
            })
            .collect()
    }

    /// Every pair of tags whose drawn labels intersect, as `(tag text, tag text)`.
    pub fn tag_overlaps(&self) -> Vec<(String, String)> {
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
}
