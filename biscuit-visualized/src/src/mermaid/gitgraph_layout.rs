//! gitGraph geometry and tag spacing from Merman's layout data.
//!
//! - **Geometry (pipeline step 6b).** [`geometry_from_layout_json`] reads the
//!   document `RenderRequest::layout_json` returns for a gitGraph and builds
//!   [`GitGraphGeometry`]: commits, lanes, and parents as Merman laid them out
//!   (both parents of a labeled merge included), the commit step derived from
//!   the commits' `pos`, and each tag's box computed with our measurer from
//!   Merman's own tag-box formula ([`TagLabels`]).
//! - **Tag spacing (pipeline step 3b).** [`commit_spacing_for`] lays a tagged
//!   gitGraph out once at the default spacing and returns the smallest
//!   `commitSpacing` that leaves one em between every colliding pair of tags,
//!   or `None` when no pair collides; [`with_commit_spacing`] writes that
//!   value into the source as a `%%{init}%%` directive.
//!
//! The layout document is read strictly: every field this module reads must
//! be present with the type Merman writes, duplicate keys anywhere in the
//! document and trailing content after it are rejected, and each error names
//! the JSON path of the offending value ([`LayoutDataError`]). Fields this
//! module does not read are ignored.
//!
//! Design: `biscuit-visualized/docs/mermaid-gitgraph.md`.

use std::collections::HashSet;
use std::fmt;

use merman::svg::{TextMeasurer, TextStyle};
use serde::de::{Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};

use super::error::MermaidError;
use super::gitgraph::{Bounds, CommitGeometry, GitGraphGeometry, TagGeometry};

/// Merman's default gitGraph commit spacing, in px: its `COMMIT_STEP` (40)
/// plus `LAYOUT_OFFSET` (10), the step a gitGraph without `commitSpacing`
/// lays out with.
pub(crate) const DEFAULT_COMMIT_SPACING: f32 = 50.0;

/// Clearance shortfall below which a pair still counts as clear, in px.
const SPACING_EPSILON: f64 = 0.01;

/// Written spacings are rounded up to this many px, so the written value is
/// short, never under the clearance, and at most this much over the minimum.
const SPACING_RESOLUTION: f64 = 0.01;

// Merman's tag-box constants (`merman-render` `svg/parity/gitgraph/render.rs`,
// mirroring Mermaid.js `drawCommitTags`).
/// Horizontal text padding inside a tag box.
const TAG_PX: f64 = 4.0;
/// Vertical text padding inside a tag box.
const TAG_PY: f64 = 2.0;
/// How far above its commit a horizontal graph's first tag is centered.
const TAG_LIFT: f64 = 19.2;
/// The distance between stacked tags on one commit.
const TAG_STACK_STEP: f64 = 20.0;

/// A gitGraph layout document that cannot be read.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub(crate) enum LayoutDataError {
    /// Not exactly one well-formed JSON value (includes trailing content).
    #[error("gitGraph layout data is not one JSON value: {0}")]
    Syntax(String),
    /// An object names `path`'s last key more than once.
    #[error("gitGraph layout data: duplicate key `{path}`")]
    DuplicateKey { path: String },
    #[error("gitGraph layout data: `{path}` is missing")]
    Missing { path: String },
    #[error("gitGraph layout data: `{path}` must be {expected}, found {found}")]
    WrongType {
        path: String,
        expected: &'static str,
        found: &'static str,
    },
    /// A value of the right type that the layout cannot carry.
    #[error("gitGraph layout data: `{path}` {reason}")]
    Invalid { path: String, reason: String },
}

impl From<LayoutDataError> for MermaidError {
    fn from(error: LayoutDataError) -> Self {
        MermaidError::RenderFailed(error.to_string())
    }
}

/// What a gitGraph's tag labels are measured with: the measurer Merman lays
/// out with and Merman's tag label style (the diagram's font stack at
/// `themeVariables.tagLabelFontSize`, 10px unless configured).
#[derive(Clone, Copy)]
pub(crate) struct TagLabels<'a> {
    pub(crate) measurer: &'a dyn TextMeasurer,
    pub(crate) style: &'a TextStyle,
}

impl TagLabels<'_> {
    /// The width and height Merman sizes every tag box of one commit with:
    /// the widest and tallest of its tags.
    fn extent(&self, tags: &[String]) -> (f64, f64) {
        tags.iter().fold((0f64, 0f64), |(width, height), tag| {
            let tag_width = self
                .measurer
                .measure_svg_raw_text_bbox_width_px(tag, self.style)
                .max(0.0);
            let tag_height = if tag.trim_end().is_empty() {
                0.0
            } else if self.style.font_size <= 10.0 {
                self.measurer
                    .measure_svg_simple_text_bbox_height_px(tag, self.style)
            } else {
                self.measurer
                    .measure_svg_tspan_text_bbox_height_px(tag, self.style)
            };
            (width.max(tag_width), height.max(tag_height.max(0.0)))
        })
    }
}

/// Builds the geometry of the gitGraph laid out in `json`, a document from
/// `RenderRequest::layout_json`, measuring tags with `tags` and recording
/// `tag_gap` (px) as the clearance colliding tags get.
///
/// ## Errors
///
/// [`LayoutDataError`] when `json` is not a gitGraph layout document of the
/// shape Merman writes; no geometry is built from a partial document.
pub(crate) fn geometry_from_layout_json(
    json: &str,
    tags: &TagLabels<'_>,
    tag_gap: f32,
) -> Result<GitGraphGeometry, LayoutDataError> {
    let document = parse_document(json)?;
    let graph = LaidOutGraph::read(&document)?;
    let commits = graph
        .commits
        .iter()
        .enumerate()
        .map(|(index, commit)| CommitGeometry {
            id: commit.id.clone(),
            index,
            x: commit.x as f32,
            lane: commit.branch.clone(),
            parents: commit.parents.clone(),
            tags: commit
                .tags
                .iter()
                .zip(tag_boxes(graph.direction, commit, tags))
                .map(|(text, bounds)| TagGeometry {
                    text: text.clone(),
                    bounds,
                })
                .collect(),
        })
        .collect();
    Ok(GitGraphGeometry {
        commits,
        commit_step: graph.commit_step() as f32,
        tag_gap,
        width: (graph.bounds.max_x - graph.bounds.min_x) as f32,
        height: (graph.bounds.max_y - graph.bounds.min_y) as f32,
    })
}

/// The `commitSpacing` that keeps a gitGraph's tags apart, or `None` when it
/// needs none.
///
/// A source without `tag:` returns `None` without calling `layout_json`.
/// Otherwise the source is laid out once (`layout_json` returns Merman's
/// layout document for it), and the result is `None` when:
///
/// - the source already sets `gitGraph.commitSpacing` (in a `%%{init}%%`
///   directive or frontmatter): the author's value wins, even one Merman
///   ignores;
/// - the graph is not left-to-right (Merman rotates top-to-bottom and
///   bottom-to-top tags, which are not spaced);
/// - no two tags on different commits share rows and are closer than
///   `tag_gap` px.
///
/// Otherwise it is the smallest spacing (rounded up to 0.01 px) that leaves
/// `tag_gap` between every such pair. A pair whose commits share a column
/// (`parallelCommits`) cannot be separated by spacing and is not counted.
///
/// ## Errors
///
/// Whatever `layout_json` returns, and [`MermaidError::RenderFailed`] when
/// its document is not a gitGraph layout ([`LayoutDataError`]).
pub(crate) fn commit_spacing_for(
    source: &str,
    tags: &TagLabels<'_>,
    tag_gap: f32,
    layout_json: impl FnOnce(&str) -> Result<String, MermaidError>,
) -> Result<Option<f32>, MermaidError> {
    if !source.contains("tag:") {
        return Ok(None);
    }
    let json = layout_json(source)?;
    let document = parse_document(&json)?;
    if author_commit_spacing(&document)? {
        return Ok(None);
    }
    let graph = LaidOutGraph::read(&document)?;
    if graph.direction != Direction::LeftRight {
        return Ok(None);
    }
    Ok(required_spacing(&graph, tags, f64::from(tag_gap)).map(|spacing| spacing as f32))
}

/// `source` with `gitGraph.commitSpacing` set to `spacing` px.
///
/// The value is appended as its own `%%{init}%%` directive on a new last
/// line. Merman (like Mermaid.js) reads directives anywhere in the source and
/// deep-merges them in source order over the frontmatter config, so this
/// composes with an author's directive or frontmatter and changes no other
/// key. Call it only when [`commit_spacing_for`] returned a value: that is
/// what keeps an author's own `commitSpacing`, which this directive would
/// override.
pub(crate) fn with_commit_spacing(source: &str, spacing: f32) -> String {
    let separator = if source.is_empty() || source.ends_with('\n') {
        ""
    } else {
        "\n"
    };
    format!("{source}{separator}%%{{init: {{\"gitGraph\": {{\"commitSpacing\": {spacing}}}}}}}%%\n")
}

// ------------------------------------------------------------------ spacing

/// One tag box with the column its commit sits in (`pos / step`).
struct PlacedTag {
    commit: usize,
    column: f64,
    bounds: Bounds,
}

/// The smallest spacing giving every colliding pair `gap`, or `None` when
/// every pair already has it.
///
/// Tag boxes move with their commit's `pos`, which is `column × spacing` in a
/// left-to-right graph, so a pair `columns` apart and `shortfall` short of the
/// gap needs `step + shortfall / columns`; the result is the largest of those.
fn required_spacing(graph: &LaidOutGraph, tags: &TagLabels<'_>, gap: f64) -> Option<f64> {
    let step = graph.commit_step();
    let placed: Vec<PlacedTag> = graph
        .commits
        .iter()
        .enumerate()
        .flat_map(|(commit, laid)| {
            tag_boxes(Direction::LeftRight, laid, tags)
                .into_iter()
                .map(move |bounds| PlacedTag {
                    commit,
                    column: (laid.pos / step).round(),
                    bounds,
                })
        })
        .collect();
    let mut required: Option<f64> = None;
    for (position, a) in placed.iter().enumerate() {
        for b in &placed[position + 1..] {
            let shares_rows = a.bounds.y1 < b.bounds.y2 && b.bounds.y1 < a.bounds.y2;
            if a.commit == b.commit || a.column == b.column || !shares_rows {
                continue;
            }
            let (earlier, later) = if a.column < b.column { (a, b) } else { (b, a) };
            let shortfall = gap - f64::from(later.bounds.x1 - earlier.bounds.x2);
            if shortfall <= SPACING_EPSILON {
                continue;
            }
            let pair = step + shortfall / (later.column - earlier.column);
            required = Some(required.map_or(pair, |required| required.max(pair)));
        }
    }
    required.map(|spacing| (spacing / SPACING_RESOLUTION).ceil() * SPACING_RESOLUTION)
}

/// Whether the document's effective config sets `gitGraph.commitSpacing`.
fn author_commit_spacing(document: &Json) -> Result<bool, LayoutDataError> {
    let root = Reader::root(document).object()?;
    let config = root
        .field("meta")?
        .object()?
        .field("effective_config")?
        .object()?;
    let Some(gitgraph) = config.optional_field("gitGraph") else {
        return Ok(false);
    };
    Ok(gitgraph.object()?.optional_field("commitSpacing").is_some())
}

// ------------------------------------------------------------------ tag boxes

/// The drawn box of each of `commit`'s tags, in `commit.tags` order
/// (Merman's `drawCommitTags`).
///
/// Every tag of a commit is sized to the widest and tallest of them, and
/// Merman draws them in reverse order, the last tag nearest the commit. In a
/// left-to-right graph the box is an unrotated pentagon (a point toward the
/// commit) centered above it; top-to-bottom and bottom-to-top graphs rotate
/// it 45° about the commit, and these bounds are the rotated outline's.
fn tag_boxes(direction: Direction, commit: &LaidOutCommit, tags: &TagLabels<'_>) -> Vec<Bounds> {
    let (width, height) = tags.extent(&commit.tags);
    let half_height = height / 2.0;
    let count = commit.tags.len();
    (0..count)
        .map(|position| {
            let stacked = (count - 1 - position) as f64 * TAG_STACK_STEP;
            let points = match direction {
                Direction::LeftRight => {
                    let center_y = commit.y - TAG_LIFT - stacked;
                    // In a left-to-right graph `x` is `pos` plus Merman's layout offset.
                    let left = (commit.pos - width / 2.0 - TAG_PX / 2.0)
                        .min(commit.x - width / 2.0 - TAG_PX);
                    let right = commit.x + width / 2.0 + TAG_PX;
                    vec![
                        (left, center_y - half_height - TAG_PY),
                        (right, center_y + half_height + TAG_PY),
                    ]
                }
                Direction::TopBottom | Direction::BottomTop => {
                    let origin = commit.pos + stacked;
                    let near = commit.x + 10.0;
                    let far = near + width + 4.0;
                    [
                        (commit.x, origin + 2.0),
                        (commit.x, origin - 2.0),
                        (near, origin - half_height - 2.0),
                        (far, origin - half_height - 2.0),
                        (far, origin + half_height + 2.0),
                        (near, origin + half_height + 2.0),
                    ]
                    .into_iter()
                    .map(|point| rotated_45(point, (commit.x, commit.pos)))
                    .map(|(x, y)| (x + 12.0, y + 12.0))
                    .collect()
                }
            };
            bounds_of(&points)
        })
        .collect()
}

/// `point` rotated 45° clockwise (SVG `rotate(45, cx, cy)`) about `center`.
fn rotated_45((x, y): (f64, f64), (cx, cy): (f64, f64)) -> (f64, f64) {
    let (sin, cos) = std::f64::consts::FRAC_PI_4.sin_cos();
    let (dx, dy) = (x - cx, y - cy);
    (cx + dx * cos - dy * sin, cy + dx * sin + dy * cos)
}

fn bounds_of(points: &[(f64, f64)]) -> Bounds {
    let (x1, y1, x2, y2) = points.iter().fold(
        (
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        ),
        |(x1, y1, x2, y2), &(x, y)| (x1.min(x), y1.min(y), x2.max(x), y2.max(y)),
    );
    Bounds {
        x1: x1 as f32,
        y1: y1 as f32,
        x2: x2 as f32,
        y2: y2 as f32,
    }
}

// ------------------------------------------------------------------ the layout

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Direction {
    LeftRight,
    TopBottom,
    BottomTop,
}

struct LayoutBounds {
    min_x: f64,
    max_x: f64,
    min_y: f64,
    max_y: f64,
}

/// The fields of Merman's `layout.GitGraphDiagram` this module reads.
struct LaidOutGraph {
    direction: Direction,
    bounds: LayoutBounds,
    commits: Vec<LaidOutCommit>,
}

struct LaidOutCommit {
    id: String,
    parents: Vec<String>,
    branch: String,
    pos: f64,
    x: f64,
    y: f64,
    tags: Vec<String>,
}

impl LaidOutGraph {
    fn read(document: &Json) -> Result<Self, LayoutDataError> {
        let root = Reader::root(document).object()?;
        let gitgraph = root
            .field("layout")?
            .object()?
            .field("GitGraphDiagram")?
            .object()?;
        let direction_field = gitgraph.field("direction")?;
        let direction = match direction_field.string()? {
            "LR" => Direction::LeftRight,
            "TB" => Direction::TopBottom,
            "BT" => Direction::BottomTop,
            other => {
                return Err(direction_field.invalid(format!("is {other:?}, not LR, TB, or BT")));
            }
        };
        let bounds = gitgraph.field("bounds")?.object()?;
        let bounds = LayoutBounds {
            min_x: bounds.field("min_x")?.number()?,
            max_x: bounds.field("max_x")?.number()?,
            min_y: bounds.field("min_y")?.number()?,
            max_y: bounds.field("max_y")?.number()?,
        };
        let commits = gitgraph
            .field("commits")?
            .array()?
            .into_iter()
            .map(|commit| LaidOutCommit::read(commit.object()?))
            .collect::<Result<Vec<_>, _>>()?;
        let graph = LaidOutGraph {
            direction,
            bounds,
            commits,
        };
        graph.check_parents(&gitgraph)?;
        Ok(graph)
    }

    /// Every parent names a laid-out commit.
    fn check_parents(&self, gitgraph: &ObjectReader<'_>) -> Result<(), LayoutDataError> {
        let ids: HashSet<&str> = self
            .commits
            .iter()
            .map(|commit| commit.id.as_str())
            .collect();
        for (index, commit) in self.commits.iter().enumerate() {
            if let Some(position) = commit
                .parents
                .iter()
                .position(|parent| !ids.contains(parent.as_str()))
            {
                return Err(LayoutDataError::Invalid {
                    path: format!("{}.commits[{index}].parents[{position}]", gitgraph.path),
                    reason: format!(
                        "names {:?}, which is not a laid-out commit",
                        commit.parents[position]
                    ),
                });
            }
        }
        Ok(())
    }

    /// The commit step: the smallest positive `pos` difference between
    /// consecutive commits or between a commit and a parent.
    ///
    /// Every `pos` is a whole number of steps from the axis start, and each
    /// commit sits exactly one step after the previous commit (or, with
    /// `parallelCommits`, after its closest parent), so the smallest positive
    /// difference is the step. A graph with no such pair (one commit) reports
    /// [`DEFAULT_COMMIT_SPACING`].
    fn commit_step(&self) -> f64 {
        let consecutive = self
            .commits
            .windows(2)
            .map(|pair| (pair[1].pos - pair[0].pos).abs());
        let parental = self.commits.iter().flat_map(|commit| {
            commit.parents.iter().filter_map(move |parent| {
                self.commits
                    .iter()
                    .find(|other| &other.id == parent)
                    .map(|other| (commit.pos - other.pos).abs())
            })
        });
        consecutive
            .chain(parental)
            .filter(|difference| *difference > SPACING_EPSILON)
            .fold(None, |step: Option<f64>, difference| {
                Some(step.map_or(difference, |step| step.min(difference)))
            })
            .unwrap_or(f64::from(DEFAULT_COMMIT_SPACING))
    }
}

impl LaidOutCommit {
    fn read(commit: ObjectReader<'_>) -> Result<Self, LayoutDataError> {
        Ok(LaidOutCommit {
            id: commit.field("id")?.non_empty_string()?.to_string(),
            parents: commit.field("parents")?.strings()?,
            branch: commit.field("branch")?.non_empty_string()?.to_string(),
            pos: commit.field("pos")?.number()?,
            x: commit.field("x")?.number()?,
            y: commit.field("y")?.number()?,
            tags: commit.field("tags")?.strings()?,
        })
    }
}

// ------------------------------------------------------------------ strict JSON

/// A JSON value that keeps every object key, duplicates included, so a
/// duplicate is an error rather than serde_json's last-wins.
#[derive(Debug)]
enum Json {
    Null,
    Bool,
    Number(f64),
    String(String),
    Array(Vec<Json>),
    Object(Vec<(String, Json)>),
}

impl Json {
    fn kind(&self) -> &'static str {
        match self {
            Json::Null => "null",
            Json::Bool => "a boolean",
            Json::Number(_) => "a number",
            Json::String(_) => "a string",
            Json::Array(_) => "an array",
            Json::Object(_) => "an object",
        }
    }
}

impl<'de> Deserialize<'de> for Json {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct JsonVisitor;

        impl<'de> Visitor<'de> for JsonVisitor {
            type Value = Json;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a JSON value")
            }

            fn visit_unit<E>(self) -> Result<Json, E> {
                Ok(Json::Null)
            }

            fn visit_bool<E>(self, _: bool) -> Result<Json, E> {
                Ok(Json::Bool)
            }

            fn visit_i64<E>(self, value: i64) -> Result<Json, E> {
                Ok(Json::Number(value as f64))
            }

            fn visit_u64<E>(self, value: u64) -> Result<Json, E> {
                Ok(Json::Number(value as f64))
            }

            fn visit_f64<E>(self, value: f64) -> Result<Json, E> {
                Ok(Json::Number(value))
            }

            fn visit_str<E>(self, value: &str) -> Result<Json, E> {
                Ok(Json::String(value.to_string()))
            }

            fn visit_string<E>(self, value: String) -> Result<Json, E> {
                Ok(Json::String(value))
            }

            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Json, A::Error> {
                let mut items = Vec::new();
                while let Some(item) = seq.next_element()? {
                    items.push(item);
                }
                Ok(Json::Array(items))
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Json, A::Error> {
                let mut entries = Vec::new();
                while let Some(entry) = map.next_entry::<String, Json>()? {
                    entries.push(entry);
                }
                Ok(Json::Object(entries))
            }
        }

        deserializer.deserialize_any(JsonVisitor)
    }
}

/// Parses exactly one JSON value and rejects any duplicate key in it.
fn parse_document(json: &str) -> Result<Json, LayoutDataError> {
    let syntax = |error: serde_json::Error| LayoutDataError::Syntax(error.to_string());
    let mut deserializer = serde_json::Deserializer::from_str(json);
    let document = Json::deserialize(&mut deserializer).map_err(syntax)?;
    deserializer.end().map_err(syntax)?;
    reject_duplicates(&document, "")?;
    Ok(document)
}

fn reject_duplicates(value: &Json, path: &str) -> Result<(), LayoutDataError> {
    match value {
        Json::Array(items) => items
            .iter()
            .enumerate()
            .try_for_each(|(index, item)| reject_duplicates(item, &format!("{path}[{index}]"))),
        Json::Object(entries) => {
            let mut seen = HashSet::new();
            for (key, item) in entries {
                let child = child_path(path, key);
                if !seen.insert(key.as_str()) {
                    return Err(LayoutDataError::DuplicateKey { path: child });
                }
                reject_duplicates(item, &child)?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

fn child_path(path: &str, key: &str) -> String {
    if path.is_empty() {
        key.to_string()
    } else {
        format!("{path}.{key}")
    }
}

/// A value and its JSON path, for located errors.
struct Reader<'a> {
    value: &'a Json,
    path: String,
}

struct ObjectReader<'a> {
    entries: &'a [(String, Json)],
    path: String,
}

impl<'a> Reader<'a> {
    fn root(value: &'a Json) -> Self {
        Reader {
            value,
            path: String::new(),
        }
    }

    fn wrong_type(&self, expected: &'static str) -> LayoutDataError {
        LayoutDataError::WrongType {
            path: if self.path.is_empty() {
                "(document)".to_string()
            } else {
                self.path.clone()
            },
            expected,
            found: self.value.kind(),
        }
    }

    fn invalid(&self, reason: String) -> LayoutDataError {
        LayoutDataError::Invalid {
            path: self.path.clone(),
            reason,
        }
    }

    fn object(&self) -> Result<ObjectReader<'a>, LayoutDataError> {
        match self.value {
            Json::Object(entries) => Ok(ObjectReader {
                entries,
                path: self.path.clone(),
            }),
            _ => Err(self.wrong_type("an object")),
        }
    }

    fn array(&self) -> Result<Vec<Reader<'a>>, LayoutDataError> {
        match self.value {
            Json::Array(items) => Ok(items
                .iter()
                .enumerate()
                .map(|(index, value)| Reader {
                    value,
                    path: format!("{}[{index}]", self.path),
                })
                .collect()),
            _ => Err(self.wrong_type("an array")),
        }
    }

    fn number(&self) -> Result<f64, LayoutDataError> {
        match self.value {
            Json::Number(number) => Ok(*number),
            _ => Err(self.wrong_type("a number")),
        }
    }

    fn string(&self) -> Result<&'a str, LayoutDataError> {
        match self.value {
            Json::String(text) => Ok(text),
            _ => Err(self.wrong_type("a string")),
        }
    }

    fn non_empty_string(&self) -> Result<&'a str, LayoutDataError> {
        let text = self.string()?;
        if text.is_empty() {
            return Err(self.invalid("is empty".to_string()));
        }
        Ok(text)
    }

    /// An array of strings; `[]` is valid.
    fn strings(&self) -> Result<Vec<String>, LayoutDataError> {
        self.array()?
            .iter()
            .map(|item| item.string().map(str::to_string))
            .collect()
    }
}

impl<'a> ObjectReader<'a> {
    fn optional_field(&self, key: &str) -> Option<Reader<'a>> {
        self.entries
            .iter()
            .find(|(name, _)| name == key)
            .map(|(_, value)| Reader {
                value,
                path: child_path(&self.path, key),
            })
    }

    fn field(&self, key: &str) -> Result<Reader<'a>, LayoutDataError> {
        self.optional_field(key)
            .ok_or_else(|| LayoutDataError::Missing {
                path: child_path(&self.path, key),
            })
    }
}
