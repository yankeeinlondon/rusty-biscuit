//! Copied from S1 (`s1-comparison/src/survey.rs`), unchanged except where noted. Defect survey: measures, from usvg's own parse of an SVG with the same font
//! database the rasterizer uses, the label defects that only show up in pixels.
//!
//! - **clipped**: a text's glyph outline box extends past an ancestor clip
//!   (a nested `<svg>` viewport with `overflow:hidden` becomes a clip in usvg).
//! - **zero-area gradient paint**: a path with zero width or height painted by an
//!   `objectBoundingBox` gradient, which per the SVG spec draws nothing. Found by
//!   re-parsing a probe copy where each such gradient reference is a sentinel color.
//! - **backing narrower**: a text extending past the smallest filled shape behind it
//!   (a node box or a label background) among its three nearest ancestors' children.
//! - **outside viewBox**: a text's glyphs extend past the canvas.
//! - **fonts**: which family each label's first requested family resolved to.
//!
//! Text that usvg drops entirely (no font matched at all) has no node to inspect;
//! `empty_texts` counts text nodes whose layout produced no glyphs.

use std::collections::BTreeMap;

use resvg::usvg::{self, Node, Paint, Rect, Transform};
use serde::Serialize;

const TOLERANCE_PX: f32 = 0.5;
/// S2 change: S1 kept 3 examples per defect kind; S2 keeps them all (up to
/// 200) so A and B can be compared label by label.
const EXAMPLES: usize = 200;
const SENTINEL: (u8, u8, u8) = (1, 2, 3);

#[derive(Debug, Default, Clone, Serialize)]
pub struct Defects {
    pub labels: usize,
    pub clipped: usize,
    pub worst_clip_px: f32,
    pub worst_clip_pct: f32,
    pub clip_examples: Vec<String>,
    pub zero_area_gradient: usize,
    pub backing_narrower: usize,
    pub backing_examples: Vec<String>,
    pub outside_viewbox: usize,
    pub outside_examples: Vec<String>,
    pub empty_texts: usize,
    /// Labels whose first requested family is a name no installed face has
    /// (case-sensitive, as fontdb matches), so a fallback face drew them.
    pub font_unmatched: usize,
    /// Of those, labels where an installed family matches ignoring case
    /// (CSS matches family names case-insensitively; fontdb does not).
    pub font_case_only: usize,
    /// "first requested family -> resolved family" -> label count
    pub fonts: BTreeMap<String, usize>,
}

pub fn survey(svg: &str, options: &usvg::Options) -> Result<Defects, String> {
    let tree = usvg::Tree::from_str(svg, options).map_err(|e| e.to_string())?;
    let mut defects = Defects::default();
    let canvas = Rect::from_xywh(0.0, 0.0, tree.size().width(), tree.size().height()).expect("canvas rect");
    let mut ctx = Walk { tree: &tree, canvas, defects: &mut defects, clips: Vec::new(), backing: Vec::new() };
    ctx.group(tree.root());
    let probe = usvg::Tree::from_str(&sentinel_probe(svg), options).map_err(|e| e.to_string())?;
    defects.zero_area_gradient = count_zero_area_sentinel(probe.root());
    Ok(defects)
}

struct Walk<'a> {
    tree: &'a usvg::Tree,
    canvas: Rect,
    defects: &'a mut Defects,
    /// Absolute clip rects of the current ancestors.
    clips: Vec<Rect>,
    /// Filled shapes among each ancestor level's direct children (absolute boxes).
    backing: Vec<Vec<Rect>>,
}

impl Walk<'_> {
    fn group(&mut self, group: &usvg::Group) {
        let pushed_clip = group.clip_path().and_then(|clip| clip_rect(group, clip)).map(|r| self.clips.push(r)).is_some();
        let shapes = group
            .children()
            .iter()
            .filter_map(|node| match node {
                Node::Path(path) if is_filled(path) => {
                    let b = path.abs_bounding_box();
                    (b.width() > 1.0 && b.height() > 1.0).then_some(b)
                }
                _ => None,
            })
            .collect();
        self.backing.push(shapes);
        for node in group.children() {
            match node {
                Node::Group(child) => self.group(child),
                Node::Text(text) => self.text(text),
                _ => {}
            }
        }
        self.backing.pop();
        if pushed_clip {
            self.clips.pop();
        }
    }

    fn text(&mut self, text: &usvg::Text) {
        let content: String = text.chunks().iter().map(|c| c.text()).collect();
        let label = content.trim().chars().take(40).collect::<String>();
        if label.is_empty() {
            return;
        }
        self.defects.labels += 1;
        let glyphs: Vec<_> = text.layouted().iter().flat_map(|s| s.positioned_glyphs.iter()).collect();
        let first_family = text.chunks().first().and_then(|c| c.spans().first()).and_then(|s| s.font().families().first());
        let requested = first_family.map(|f| f.to_string()).unwrap_or_else(|| "(none)".into());
        if let Some(usvg::FontFamily::Named(name)) = first_family {
            let db = self.tree.fontdb();
            let has = |cmp: &dyn Fn(&str) -> bool| db.faces().any(|f| f.families.iter().any(|(n, _)| cmp(n)));
            if !has(&|n| n == name) {
                self.defects.font_unmatched += 1;
                if has(&|n| n.eq_ignore_ascii_case(name)) {
                    self.defects.font_case_only += 1;
                }
            }
        }
        let resolved = glyphs
            .first()
            .and_then(|g| self.tree.fontdb().face(g.font))
            .and_then(|face| face.families.first().map(|(name, _)| name.clone()))
            .unwrap_or_else(|| "(no glyphs)".into());
        *self.defects.fonts.entry(format!("{requested} -> {resolved}")).or_default() += 1;
        if glyphs.is_empty() {
            self.defects.empty_texts += 1;
            return;
        }
        // The flattened group's own boxes are in text space; apply the text's
        // absolute transform for the tight glyph box in canvas coordinates.
        let Some(bbox) = text.flattened().bounding_box().transform(text.abs_transform()) else { return };
        if bbox.width() <= 0.0 {
            return;
        }

        // Clipping by any ancestor clip rect.
        let mut worst = 0.0_f32;
        for clip in &self.clips {
            worst = worst.max(overflow(&bbox, clip));
        }
        if worst > TOLERANCE_PX {
            self.defects.clipped += 1;
            let pct = 100.0 * worst / bbox.width();
            if worst > self.defects.worst_clip_px {
                self.defects.worst_clip_px = worst;
            }
            self.defects.worst_clip_pct = self.defects.worst_clip_pct.max(pct);
            if self.defects.clip_examples.len() < EXAMPLES {
                self.defects.clip_examples.push(format!("\"{label}\" cut by {worst:.1}px ({pct:.0}%)"));
            }
        }

        // Narrower backing shape.
        let center = (bbox.x() + bbox.width() / 2.0, bbox.y() + bbox.height() / 2.0);
        let backing = self
            .backing
            .iter()
            .rev()
            .take(3)
            .flatten()
            .filter(|r| r.x() <= center.0 && center.0 <= r.right() && r.y() <= center.1 && center.1 <= r.bottom())
            .min_by(|a, b| (a.width() * a.height()).total_cmp(&(b.width() * b.height())));
        if let Some(shape) = backing {
            let over = (shape.x() - bbox.x()).max(0.0) + (bbox.right() - shape.right()).max(0.0);
            if over > 1.0 {
                self.defects.backing_narrower += 1;
                if self.defects.backing_examples.len() < EXAMPLES {
                    self.defects.backing_examples.push(format!(
                        "\"{label}\" {:.0}px wide on a {:.0}px shape",
                        bbox.width(),
                        shape.width()
                    ));
                }
            }
        }

        // Outside the canvas.
        let out = overflow(&bbox, &self.canvas);
        if out > TOLERANCE_PX {
            self.defects.outside_viewbox += 1;
            if self.defects.outside_examples.len() < EXAMPLES {
                self.defects.outside_examples.push(format!("\"{label}\" {out:.1}px outside"));
            }
        }
    }
}

/// How far `inner` extends past `outer` on its worst axis.
fn overflow(inner: &Rect, outer: &Rect) -> f32 {
    let x = (outer.x() - inner.x()).max(0.0) + (inner.right() - outer.right()).max(0.0);
    let y = (outer.y() - inner.y()).max(0.0) + (inner.bottom() - outer.bottom()).max(0.0);
    x.max(y)
}

/// A clip path's region in canvas coordinates. Clip content lives in the user
/// space of the clipped element, its own transform included.
fn clip_rect(group: &usvg::Group, clip: &usvg::ClipPath) -> Option<Rect> {
    // usvg does not compute bounding boxes inside clip paths, so take the
    // union of the clip's path data in clip space.
    let local = clip_space_bounds(clip.root())?;
    let ts: Transform = group.abs_transform().pre_concat(clip.transform());
    local.transform(ts)
}

fn clip_space_bounds(group: &usvg::Group) -> Option<Rect> {
    let mut acc: Option<Rect> = None;
    for node in group.children() {
        let rect = match node {
            Node::Path(path) => path.data().bounds().transform(path.abs_transform()),
            Node::Group(child) => clip_space_bounds(child),
            _ => None,
        };
        if let Some(r) = rect {
            acc = Some(match acc {
                None => r,
                Some(a) => Rect::from_ltrb(a.left().min(r.left()), a.top().min(r.top()), a.right().max(r.right()), a.bottom().max(r.bottom()))?,
            });
        }
    }
    acc
}

fn is_filled(path: &usvg::Path) -> bool {
    path.is_visible()
        && path.fill().is_some_and(|fill| {
            fill.opacity().get() > 0.05 && !matches!(fill.paint(), Paint::Color(c) if (c.red, c.green, c.blue) == SENTINEL)
        })
}

/// Every reference to an `objectBoundingBox` gradient (the default units)
/// replaced by the sentinel color, so the probe parse keeps the paint that a
/// zero-area box would otherwise drop.
fn sentinel_probe(svg: &str) -> String {
    let mut ids = Vec::new();
    for kind in ["<linearGradient", "<radialGradient"] {
        let mut rest = svg;
        while let Some(at) = rest.find(kind) {
            rest = &rest[at + 1..];
            let tag = &rest[..rest.find('>').unwrap_or(rest.len())];
            if tag.contains("userSpaceOnUse") {
                continue;
            }
            if let Some(start) = tag.find(" id=\"") {
                let start = start + 5;
                if let Some(len) = tag[start..].find('"') {
                    ids.push(tag[start..start + len].to_string());
                }
            }
        }
    }
    let (r, g, b) = SENTINEL;
    let color = format!("#{r:02x}{g:02x}{b:02x}");
    let mut probe = svg.to_string();
    for id in ids {
        probe = probe.replace(&format!("url(#{id})"), &color).replace(&format!("url('#{id}')"), &color);
    }
    probe
}

fn count_zero_area_sentinel(group: &usvg::Group) -> usize {
    group
        .children()
        .iter()
        .map(|node| match node {
            Node::Group(child) => count_zero_area_sentinel(child),
            Node::Path(path) => {
                let b = path.data().bounds();
                let zero = b.width() < 0.01 || b.height() < 0.01;
                let sentinel = |paint: &Paint| matches!(paint, Paint::Color(c) if (c.red, c.green, c.blue) == SENTINEL);
                let painted = path.stroke().is_some_and(|s| sentinel(s.paint()))
                    || path.fill().is_some_and(|f| sentinel(f.paint()));
                usize::from(zero && painted)
            }
            _ => 0,
        })
        .sum()
}
