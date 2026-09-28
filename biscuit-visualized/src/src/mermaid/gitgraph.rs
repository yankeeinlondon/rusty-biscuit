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
//!   tag wider than that step covers its neighbor's. [`layout`] lays out a
//!   second time with the step widened to fit the widest tag.

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

/// Lays out `graph`, spacing a gitGraph's commits so no two tags overlap.
///
/// A left-to-right or right-to-left gitGraph with at least one tag, none of
/// them rotated, is laid out twice: the second pass sets `commit_step` to the
/// widest tag plus [`TAG_GAP_EM`] (never less than the default step). Every
/// other diagram, a gitGraph without tags, a vertical one (`TB`, `BT`), and one
/// with a rotated tag get the single default pass, so their tags are not
/// spaced.
pub(crate) fn layout(graph: &Graph, theme: &Theme) -> (Layout, LayoutConfig) {
    let mut config = LayoutConfig::default();
    let first = mermaid_rs_renderer::compute_layout(graph, theme, &config);
    let Some(widest) = gitgraph_of(&first)
        .filter(|gitgraph| matches!(gitgraph.direction, Direction::LeftRight | Direction::RightLeft))
        .and_then(widest_horizontal_tag)
    else {
        return (first, config);
    };
    let step = config.gitgraph.commit_step.max(widest + TAG_GAP_EM * theme.font_size);
    if step == config.gitgraph.commit_step {
        return (first, config);
    }
    config.gitgraph.commit_step = step;
    (mermaid_rs_renderer::compute_layout(graph, theme, &config), config)
}

fn gitgraph_of(layout: &Layout) -> Option<&GitGraphLayout> {
    match &layout.diagram {
        DiagramData::GitGraph(gitgraph) => Some(gitgraph),
        _ => None,
    }
}

/// The widest tag's width, or `None` when there is no tag or any is rotated.
fn widest_horizontal_tag(gitgraph: &GitGraphLayout) -> Option<f32> {
    let mut tags = gitgraph.commits.iter().flat_map(|commit| &commit.tags).peekable();
    tags.peek()?;
    let mut widest = 0f32;
    for tag in tags {
        if tag.transform.is_some() {
            return None;
        }
        widest = widest.max(Bounds::of(&tag.points).width());
    }
    Some(widest)
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
    pub width: f32,
    pub height: f32,
}

/// One laid-out commit.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq)]
pub struct CommitGeometry {
    pub id: String,
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
    pub(crate) fn from_layout(graph: &Graph, layout: &Layout, config: &LayoutConfig) -> Option<Self> {
        let gitgraph = gitgraph_of(layout)?;
        let commits = gitgraph
            .commits
            .iter()
            .map(|laid| {
                let parsed = graph.gitgraph.commits.iter().find(|commit| commit.id == laid.id);
                CommitGeometry {
                    id: laid.id.clone(),
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
