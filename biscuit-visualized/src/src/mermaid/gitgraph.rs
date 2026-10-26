//! The gitGraph geometry types that `MermaidDiagram::gitgraph_geometry`
//! returns: commits, lanes, parents, and tag bounds as Merman laid them out.
//!
//! The geometry is built from Merman's layout data, not from the SVG, by
//! [`gitgraph_layout`](super::gitgraph_layout), which also computes the
//! `commitSpacing` that keeps colliding tags one em apart. Merman keeps both
//! parents of a labeled merge and accepts a merge of a branch with no commit
//! of its own, as Mermaid.js does, so no merge is repaired or rejected here.
//!
//! Test support for callers that must prove label placement without
//! inspecting pixels; not a stable API.

use super::gitgraph_layout::DEFAULT_COMMIT_SPACING;

/// The gap between neighboring tags, in ems of the theme's font size.
///
/// A readability policy, not a measured margin: one em keeps two adjacent
/// labels visibly separate at every theme's font size.
pub(crate) const TAG_GAP_EM: f32 = 1.0;

/// The commit spacing a gitGraph without colliding tags keeps: Merman's
/// default of 50 px.
#[doc(hidden)]
pub fn default_gitgraph_commit_step() -> f32 {
    DEFAULT_COMMIT_SPACING
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
    /// The commit spacing the layout used, derived from the commits'
    /// positions: [`default_gitgraph_commit_step`] unless tags collided and a
    /// wider `commitSpacing` was written.
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
    /// The position in layout order.
    pub index: usize,
    pub x: f32,
    /// The lane (branch) name the commit sits on.
    pub lane: String,
    /// Parent IDs as Merman laid them out (both parents of a merge).
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
