//! gitGraph geometry and tag spacing from Merman's layout data
//! (`mermaid::gitgraph_layout`; R-C4, R-C5, AC-C11, AC-C12).
//!
//! - **Loader matrix:** one test per invalid shape of a field the loader
//!   reads, each an edit of the unedited control document
//!   (`fixtures/merman/gitgraph/layout.json`, Merman's `layout_json` for
//!   `layout.mmd` at the pinned rev), asserting the located error.
//! - **Tag boxes:** our measured tag bounds equal the polygons Merman draws.
//! - **Spacing:** the written `commitSpacing` is the smallest that gives
//!   every colliding pair one em, verified on the layout at that spacing.
//!
//! Every Merman call measures with a chain of the bundled Inter alone
//! (`MERMAID_FONT=Inter`), so widths are the same on every host.

use std::sync::Arc;

use merman::svg::{
    MeasurementProfileId, SvgEnvironment, SvgPipeline, TextMeasurementPolicy,
    TextMeasurementProfile, TextMeasurementProfileIdentity, TextMeasurer, TextStyle,
};
use merman::{
    Engine, MermaidConfig, OperationControl, RenderOutput, RenderRequest, Renderer, SvgRequest,
};
use serde_json::{Value, json};

use crate::fonts::{FontChain, LabelMeasurer};
use crate::mermaid::error::MermaidError;
use crate::mermaid::gitgraph::{Bounds, GitGraphGeometry, TAG_GAP_EM};
use crate::mermaid::gitgraph_layout::{
    DEFAULT_COMMIT_SPACING, LayoutDataError, TagLabels, commit_spacing_for,
    geometry_from_layout_json, with_commit_spacing,
};

/// The theme font size the em is taken from (Merman's `themeVariables.fontSize`).
const THEME_FONT_SIZE: f32 = 16.0;
const GAP: f32 = TAG_GAP_EM * THEME_FONT_SIZE;
/// Float noise allowed when checking a clearance or a derived step.
const TOLERANCE: f32 = 0.01;

// ------------------------------------------------------------------ helpers

type Measurer = Arc<dyn TextMeasurer + Send + Sync>;

fn measurer() -> Measurer {
    LabelMeasurer::text_measurer(&Arc::new(FontChain::injected(&[], &[], Some("Inter"))))
}

/// Merman's gitGraph tag label style under the `dark` theme.
fn tag_style() -> TextStyle {
    TextStyle {
        font_family: Some(r#""trebuchet ms", verdana, arial, sans-serif"#.to_string()),
        font_size: 10.0,
        font_weight: None,
        font_style: None,
    }
}

fn request(measurer: &Measurer) -> SvgRequest {
    let identity = TextMeasurementProfileIdentity::new(
        MeasurementProfileId::new("gitgraph-layout-test").expect("profile id"),
        "1",
    )
    .expect("profile identity");
    let mut request = SvgRequest {
        environment: SvgEnvironment::deterministic().with_text_measurement_policy(
            TextMeasurementPolicy::uniform(TextMeasurementProfile::new(
                identity,
                Arc::clone(measurer),
            )),
        ),
        ..SvgRequest::default()
    };
    request.options.diagram_id = Some("gitgraph".into());
    request.pipeline = Some(SvgPipeline::resvg_safe());
    request
}

/// An explicit theme, as the render path sets one (see `measure_tests`).
fn renderer() -> Renderer {
    Renderer::new().with_engine(
        Engine::new().with_site_config(MermaidConfig::from_value(json!({ "theme": "dark" }))),
    )
}

fn layout_json_with(source: &str, measurer: &Measurer) -> Result<String, MermaidError> {
    match renderer().render(RenderRequest::layout_json(
        source,
        OperationControl::new(),
        request(measurer),
    )) {
        Ok(RenderOutput::LayoutJson(Some(layout))) => {
            let mut bytes = Vec::new();
            layout
                .layout()
                .write_json(&mut bytes)
                .expect("layout JSON writes");
            Ok(String::from_utf8(bytes).expect("layout JSON is UTF-8"))
        }
        other => Err(MermaidError::RenderFailed(format!("{other:?}"))),
    }
}

fn svg_with(source: &str, measurer: &Measurer) -> String {
    match renderer().render(RenderRequest::svg(
        source,
        OperationControl::new(),
        request(measurer),
    )) {
        Ok(RenderOutput::Svg(Some(svg))) => svg.svg().to_string(),
        other => panic!("Merman renders the gitGraph: {other:?}"),
    }
}

/// Geometry from a layout document, measured as Merman measured it.
fn geometry_of(json: &str, measurer: &Measurer) -> Result<GitGraphGeometry, LayoutDataError> {
    let style = tag_style();
    let labels = TagLabels {
        measurer: measurer.as_ref(),
        style: &style,
    };
    geometry_from_layout_json(json, &labels, GAP)
}

fn laid_out(source: &str, measurer: &Measurer) -> GitGraphGeometry {
    let json = layout_json_with(source, measurer).expect("Merman lays the gitGraph out");
    geometry_of(&json, measurer).expect("the layout document loads")
}

fn spacing_for(source: &str, measurer: &Measurer) -> Option<f32> {
    let style = tag_style();
    let labels = TagLabels {
        measurer: measurer.as_ref(),
        style: &style,
    };
    commit_spacing_for(source, &labels, GAP, |source| {
        layout_json_with(source, measurer)
    })
    .expect("spacing computes")
}

/// Every pair of tags on different commits whose rows overlap, as
/// `(texts, horizontal clearance)`, the earlier label first.
fn row_sharing_clearances(geometry: &GitGraphGeometry) -> Vec<(String, f32)> {
    let boxes = geometry.tag_boxes();
    let mut pairs = Vec::new();
    for (position, (commit, text, bounds)) in boxes.iter().enumerate() {
        for (other_commit, other_text, other) in &boxes[position + 1..] {
            if commit == other_commit || !(bounds.y1 < other.y2 && other.y1 < bounds.y2) {
                continue;
            }
            let (earlier, later) = if bounds.x1 <= other.x1 {
                (bounds, other)
            } else {
                (other, bounds)
            };
            pairs.push((format!("{text} / {other_text}"), later.x1 - earlier.x2));
        }
    }
    pairs
}

fn min_clearance(geometry: &GitGraphGeometry) -> f32 {
    row_sharing_clearances(geometry)
        .into_iter()
        .map(|(_, clearance)| clearance)
        .fold(f32::INFINITY, f32::min)
}

/// AC-C12 on one source: tags collide at the default, the written spacing
/// clears every pair by one em with no intersecting bounds, the step derived
/// from the layout is the written value, and half a pixel less leaves some
/// pair short of the em. Returns the spacing written.
fn assert_smallest_clearing_spacing(source: &str) -> f32 {
    let measurer = measurer();
    let unspaced = laid_out(source, &measurer);
    assert_eq!(unspaced.commit_step, DEFAULT_COMMIT_SPACING);
    assert!(
        min_clearance(&unspaced) < GAP,
        "control: tags collide at the default step"
    );

    let spacing = spacing_for(source, &measurer).expect("colliding tags get a commitSpacing");
    let spaced = laid_out(&with_commit_spacing(source, spacing), &measurer);
    assert!(
        spaced.tag_overlaps().is_empty(),
        "{:?}",
        spaced.tag_overlaps()
    );
    assert!(
        (spaced.commit_step - spacing).abs() < TOLERANCE,
        "derived step {} vs written {spacing}",
        spaced.commit_step
    );
    let clearances = row_sharing_clearances(&spaced);
    assert!(
        clearances
            .iter()
            .all(|(_, clearance)| *clearance >= GAP - TOLERANCE),
        "{clearances:?}"
    );
    // Rounded up to 0.01 px: the tightest pair is within that of one em.
    assert!(min_clearance(&spaced) < GAP + 0.02, "{clearances:?}");

    let tighter = laid_out(&with_commit_spacing(source, spacing - 0.5), &measurer);
    assert!(
        min_clearance(&tighter) < GAP,
        "spacing - 0.5 px still clears every pair"
    );
    spacing
}

/// The polygons Merman drew with class `tag-label-bkg`, as `(text, bounds)`
/// in drawing order, each transform (`translate(a,b) rotate(45, cx,cy)`)
/// applied.
pub(super) fn drawn_tags(svg: &str) -> Vec<(String, Bounds)> {
    const POLYGON: &str = r#"<polygon class="tag-label-bkg" points=""#;
    let numbers = |text: &str| -> Vec<f64> {
        text.split(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-'))
            .filter(|token| !token.is_empty())
            .map(|token| token.parse().expect("a number"))
            .collect()
    };
    let mut tags = Vec::new();
    let mut rest = svg;
    while let Some(start) = rest.find(POLYGON) {
        rest = &rest[start + POLYGON.len()..];
        let end = rest.find('"').expect("points end");
        let coordinates = numbers(&rest[..end]);
        let mut points: Vec<(f64, f64)> = coordinates
            .chunks(2)
            .map(|pair| (pair[0], pair[1]))
            .collect();
        rest = &rest[end + 1..];
        if let Some(transform) = rest.strip_prefix(r#" transform=""#) {
            let close = transform.find('"').expect("transform end");
            let [dx, dy, angle, cx, cy] = numbers(&transform[..close])[..] else {
                panic!("unexpected transform {}", &transform[..close]);
            };
            let (sin, cos) = angle.to_radians().sin_cos();
            points = points
                .into_iter()
                .map(|(x, y)| {
                    (
                        cx + (x - cx) * cos - (y - cy) * sin + dx,
                        cy + (x - cx) * sin + (y - cy) * cos + dy,
                    )
                })
                .collect();
        }
        let label = rest.find(r#"class="tag-label""#).expect("tag label");
        rest = &rest[label..];
        let open = rest.find('>').expect("label open") + 1;
        let close = rest.find("</text>").expect("label close");
        let text = rest[open..close]
            .replace("&gt;", ">")
            .replace("&lt;", "<")
            .replace("&amp;", "&");
        let bounds = points.iter().fold(
            Bounds {
                x1: f32::INFINITY,
                y1: f32::INFINITY,
                x2: f32::NEG_INFINITY,
                y2: f32::NEG_INFINITY,
            },
            |b, &(x, y)| Bounds {
                x1: b.x1.min(x as f32),
                y1: b.y1.min(y as f32),
                x2: b.x2.max(x as f32),
                y2: b.y2.max(y as f32),
            },
        );
        tags.push((text, bounds));
        rest = &rest[close..];
    }
    tags
}

/// Our tag bounds match Merman's drawn polygons, tag for tag (by text).
fn assert_tag_bounds_match_drawing(source: &str) {
    let measurer = measurer();
    let geometry = laid_out(source, &measurer);
    let drawn = drawn_tags(&svg_with(source, &measurer));
    let ours: Vec<(&str, Bounds)> = geometry
        .tag_boxes()
        .into_iter()
        .map(|(_, text, bounds)| (text, bounds))
        .collect();
    assert_eq!(ours.len(), drawn.len(), "ours {ours:?}, drawn {drawn:?}");
    assert!(!ours.is_empty());
    for (text, bounds) in &ours {
        let (_, drawn) = drawn
            .iter()
            .find(|(drawn_text, _)| drawn_text == text)
            .expect("a drawn tag");
        for (mine, theirs) in [
            (bounds.x1, drawn.x1),
            (bounds.y1, drawn.y1),
            (bounds.x2, drawn.x2),
            (bounds.y2, drawn.y2),
        ] {
            assert!(
                (mine - theirs).abs() < TOLERANCE,
                "{text}: ours {bounds:?}, drawn {drawn:?}"
            );
        }
    }
}

// ------------------------------------------------------------------ control

fn control() -> &'static str {
    include_str!("fixtures/merman/gitgraph/layout.json")
}

/// The control document, edited.
fn edited(edit: impl FnOnce(&mut Value)) -> String {
    let mut document: Value = serde_json::from_str(control()).expect("the control is JSON");
    edit(&mut document);
    serde_json::to_string(&document).expect("serializes")
}

fn gitgraph(document: &mut Value) -> &mut serde_json::Map<String, Value> {
    document["layout"]["GitGraphDiagram"]
        .as_object_mut()
        .expect("GitGraphDiagram")
}

/// Commit 4 of the control, the labeled merge `M` (parents `m1`, `f2`; tags
/// `v1.0`, `stable`).
fn merge_commit(document: &mut Value) -> &mut serde_json::Map<String, Value> {
    gitgraph(document)["commits"][4]
        .as_object_mut()
        .expect("commit 4")
}

const GRAPH: &str = "layout.GitGraphDiagram";
const MERGE: &str = "layout.GitGraphDiagram.commits[4]";

fn missing(path: &str) -> LayoutDataError {
    LayoutDataError::Missing {
        path: path.to_string(),
    }
}

fn wrong(path: &str, expected: &'static str, found: &'static str) -> LayoutDataError {
    LayoutDataError::WrongType {
        path: path.to_string(),
        expected,
        found,
    }
}

fn invalid(path: &str, reason: &str) -> LayoutDataError {
    LayoutDataError::Invalid {
        path: path.to_string(),
        reason: reason.to_string(),
    }
}

fn load(json: &str) -> Result<GitGraphGeometry, LayoutDataError> {
    geometry_of(json, &measurer())
}

#[test]
fn gitgraph_layout_control_document_builds_geometry() {
    let geometry = load(control()).expect("the unedited document loads");

    let ids: Vec<&str> = geometry
        .commits
        .iter()
        .map(|commit| commit.id.as_str())
        .collect();
    assert_eq!(ids, ["init", "f1", "f2", "m1", "M", "m2"]);
    let merge = geometry.commit("M").expect("M");
    assert_eq!(merge.parents, ["m1", "f2"]);
    assert_eq!(merge.lane, "main");
    assert_eq!(merge.index, 4);
    assert_eq!(merge.x, 210.0);
    let tags: Vec<&str> = merge.tags.iter().map(|tag| tag.text.as_str()).collect();
    assert_eq!(tags, ["v1.0", "stable"]);
    assert!(geometry.commit("init").expect("init").parents.is_empty());
    assert_eq!(geometry.commit_step, DEFAULT_COMMIT_SPACING);
    assert_eq!(geometry.tag_gap, GAP);
    assert_eq!(
        (geometry.width, geometry.height),
        (308.0 + 114.703125, 106.0 + 20.0)
    );
}

#[test]
fn gitgraph_layout_control_document_is_merman_output_for_its_source() {
    let regenerated = layout_json_with(
        include_str!("fixtures/merman/gitgraph/layout.mmd"),
        &measurer(),
    )
    .expect("lays out");
    let fixture: Value = serde_json::from_str(control()).expect("JSON");
    let fresh: Value = serde_json::from_str(&regenerated).expect("JSON");
    assert_eq!(
        fixture["layout"], fresh["layout"],
        "regenerate layout.json from layout.mmd"
    );
}

#[test]
fn gitgraph_layout_default_step_is_mermans_fifty() {
    assert_eq!(DEFAULT_COMMIT_SPACING, 50.0);
    let geometry = laid_out(
        "gitGraph\n    commit id: \"A\"\n    commit id: \"B\"\n",
        &measurer(),
    );
    assert_eq!(geometry.commit_step, DEFAULT_COMMIT_SPACING);
}

// ------------------------------------------------------------------ loader matrix

macro_rules! loader_matrix {
    ($($name:ident: $edit:expr => $expected:expr;)*) => {
        $(
            #[test]
            fn $name() {
                assert_eq!(load(&edited($edit)).map(|_| ()), $expected);
            }
        )*
    };
}

loader_matrix! {
    // parents: `[]` is the one valid empty shape (an explicit root commit).
    gitgraph_layout_parents_absent: |d| { merge_commit(d).remove("parents"); }
        => Err(missing(&format!("{MERGE}.parents")));
    gitgraph_layout_parents_null: |d| { merge_commit(d)["parents"] = Value::Null; }
        => Err(wrong(&format!("{MERGE}.parents"), "an array", "null"));
    gitgraph_layout_parents_wrong_type: |d| { merge_commit(d)["parents"] = json!("p"); }
        => Err(wrong(&format!("{MERGE}.parents"), "an array", "a string"));
    gitgraph_layout_parents_wrong_element: |d| { merge_commit(d)["parents"] = json!(["m1", 1]); }
        => Err(wrong(&format!("{MERGE}.parents[1]"), "a string", "a number"));
    gitgraph_layout_parents_empty_is_a_root: |d| { merge_commit(d)["parents"] = json!([]); }
        => Ok(());
    gitgraph_layout_parents_unknown_commit: |d| { merge_commit(d)["parents"] = json!(["m1", "nope"]); }
        => Err(invalid(&format!("{MERGE}.parents[1]"), "names \"nope\", which is not a laid-out commit"));

    gitgraph_layout_x_absent: |d| { merge_commit(d).remove("x"); } => Err(missing(&format!("{MERGE}.x")));
    gitgraph_layout_x_null: |d| { merge_commit(d)["x"] = Value::Null; }
        => Err(wrong(&format!("{MERGE}.x"), "a number", "null"));
    gitgraph_layout_x_wrong_type: |d| { merge_commit(d)["x"] = json!(true); }
        => Err(wrong(&format!("{MERGE}.x"), "a number", "a boolean"));
    gitgraph_layout_x_empty_string: |d| { merge_commit(d)["x"] = json!(""); }
        => Err(wrong(&format!("{MERGE}.x"), "a number", "a string"));
    gitgraph_layout_y_absent: |d| { merge_commit(d).remove("y"); } => Err(missing(&format!("{MERGE}.y")));
    gitgraph_layout_y_null: |d| { merge_commit(d)["y"] = Value::Null; }
        => Err(wrong(&format!("{MERGE}.y"), "a number", "null"));
    gitgraph_layout_y_wrong_type: |d| { merge_commit(d)["y"] = json!([1]); }
        => Err(wrong(&format!("{MERGE}.y"), "a number", "an array"));
    gitgraph_layout_y_empty_string: |d| { merge_commit(d)["y"] = json!(""); }
        => Err(wrong(&format!("{MERGE}.y"), "a number", "a string"));
    gitgraph_layout_pos_absent: |d| { merge_commit(d).remove("pos"); } => Err(missing(&format!("{MERGE}.pos")));
    gitgraph_layout_pos_null: |d| { merge_commit(d)["pos"] = Value::Null; }
        => Err(wrong(&format!("{MERGE}.pos"), "a number", "null"));
    gitgraph_layout_pos_wrong_type: |d| { merge_commit(d)["pos"] = json!("200"); }
        => Err(wrong(&format!("{MERGE}.pos"), "a number", "a string"));
    gitgraph_layout_pos_empty_string: |d| { merge_commit(d)["pos"] = json!(""); }
        => Err(wrong(&format!("{MERGE}.pos"), "a number", "a string"));

    gitgraph_layout_branch_absent: |d| { merge_commit(d).remove("branch"); }
        => Err(missing(&format!("{MERGE}.branch")));
    gitgraph_layout_branch_null: |d| { merge_commit(d)["branch"] = Value::Null; }
        => Err(wrong(&format!("{MERGE}.branch"), "a string", "null"));
    gitgraph_layout_branch_wrong_type: |d| { merge_commit(d)["branch"] = json!(0); }
        => Err(wrong(&format!("{MERGE}.branch"), "a string", "a number"));
    gitgraph_layout_branch_empty_string: |d| { merge_commit(d)["branch"] = json!(""); }
        => Err(invalid(&format!("{MERGE}.branch"), "is empty"));

    // tags: Merman writes `[]` for a commit without tags, so `[]` is valid
    // and an absent field is an error.
    gitgraph_layout_tags_absent: |d| { merge_commit(d).remove("tags"); }
        => Err(missing(&format!("{MERGE}.tags")));
    gitgraph_layout_tags_null: |d| { merge_commit(d)["tags"] = Value::Null; }
        => Err(wrong(&format!("{MERGE}.tags"), "an array", "null"));
    gitgraph_layout_tags_wrong_type: |d| { merge_commit(d)["tags"] = json!("v1.0"); }
        => Err(wrong(&format!("{MERGE}.tags"), "an array", "a string"));
    gitgraph_layout_tags_wrong_element: |d| { merge_commit(d)["tags"] = json!(["v1.0", 2]); }
        => Err(wrong(&format!("{MERGE}.tags[1]"), "a string", "a number"));
    gitgraph_layout_tags_empty_is_no_tags: |d| { merge_commit(d)["tags"] = json!([]); }
        => Ok(());

    gitgraph_layout_id_absent: |d| { merge_commit(d).remove("id"); } => Err(missing(&format!("{MERGE}.id")));
    gitgraph_layout_id_wrong_type: |d| { merge_commit(d)["id"] = json!(4); }
        => Err(wrong(&format!("{MERGE}.id"), "a string", "a number"));
    gitgraph_layout_id_empty_string: |d| { merge_commit(d)["id"] = json!(""); }
        => Err(invalid(&format!("{MERGE}.id"), "is empty"));
    gitgraph_layout_commit_not_an_object: |d| { gitgraph(d)["commits"][4] = json!("M"); }
        => Err(wrong(MERGE, "an object", "a string"));

    gitgraph_layout_commits_absent: |d| { gitgraph(d).remove("commits"); }
        => Err(missing(&format!("{GRAPH}.commits")));
    gitgraph_layout_commits_null: |d| { gitgraph(d)["commits"] = Value::Null; }
        => Err(wrong(&format!("{GRAPH}.commits"), "an array", "null"));
    gitgraph_layout_commits_wrong_type: |d| { gitgraph(d)["commits"] = json!({}); }
        => Err(wrong(&format!("{GRAPH}.commits"), "an array", "an object"));

    gitgraph_layout_direction_absent: |d| { gitgraph(d).remove("direction"); }
        => Err(missing(&format!("{GRAPH}.direction")));
    gitgraph_layout_direction_null: |d| { gitgraph(d)["direction"] = Value::Null; }
        => Err(wrong(&format!("{GRAPH}.direction"), "a string", "null"));
    gitgraph_layout_direction_wrong_type: |d| { gitgraph(d)["direction"] = json!(1); }
        => Err(wrong(&format!("{GRAPH}.direction"), "a string", "a number"));
    gitgraph_layout_direction_unknown: |d| { gitgraph(d)["direction"] = json!("RL"); }
        => Err(invalid(&format!("{GRAPH}.direction"), "is \"RL\", not LR, TB, or BT"));

    gitgraph_layout_bounds_absent: |d| { gitgraph(d).remove("bounds"); }
        => Err(missing(&format!("{GRAPH}.bounds")));
    gitgraph_layout_bounds_null: |d| { gitgraph(d)["bounds"] = Value::Null; }
        => Err(wrong(&format!("{GRAPH}.bounds"), "an object", "null"));
    gitgraph_layout_bounds_wrong_type: |d| { gitgraph(d)["bounds"] = json!([0, 0, 1, 1]); }
        => Err(wrong(&format!("{GRAPH}.bounds"), "an object", "an array"));
    gitgraph_layout_bounds_edge_absent: |d| { gitgraph(d)["bounds"].as_object_mut().unwrap().remove("max_y"); }
        => Err(missing(&format!("{GRAPH}.bounds.max_y")));
    gitgraph_layout_bounds_edge_null: |d| { gitgraph(d)["bounds"]["min_x"] = Value::Null; }
        => Err(wrong(&format!("{GRAPH}.bounds.min_x"), "a number", "null"));
    gitgraph_layout_bounds_edge_wrong_type: |d| { gitgraph(d)["bounds"]["max_x"] = json!("308"); }
        => Err(wrong(&format!("{GRAPH}.bounds.max_x"), "a number", "a string"));

    gitgraph_layout_layout_absent: |d| { d.as_object_mut().unwrap().remove("layout"); }
        => Err(missing("layout"));
    gitgraph_layout_not_a_gitgraph_layout: |d| { d["layout"] = json!({ "FlowchartV2": {} }); }
        => Err(missing(GRAPH));
    gitgraph_layout_document_not_an_object: |d| { *d = json!([]); }
        => Err(wrong("(document)", "an object", "an array"));
}

#[test]
fn gitgraph_layout_rejects_trailing_content() {
    let json = format!("{} {{}}", control());
    assert!(
        matches!(load(&json), Err(LayoutDataError::Syntax(message)) if message.contains("trailing"))
    );
}

#[test]
fn gitgraph_layout_allows_trailing_whitespace() {
    assert!(load(&format!("{}\n", control())).is_ok());
}

#[test]
fn gitgraph_layout_rejects_truncated_json() {
    let json = &control()[..control().len() - 1];
    assert!(matches!(load(json), Err(LayoutDataError::Syntax(_))));
}

#[test]
fn gitgraph_layout_rejects_a_duplicate_commit_field() {
    // `"parents"` twice in commit 4: the second would win under last-wins.
    let document = edited(|_| {});
    let layout = document
        .find(r#""layout":{"GitGraphDiagram""#)
        .expect("the layout section");
    let (before, after) = document.split_at(layout);
    let json = format!(
        "{before}{}",
        after.replacen(
            r#""parents":["m1","f2"]"#,
            r#""parents":["m1","f2"],"parents":["m1"]"#,
            1
        )
    );
    assert_ne!(json, document);
    assert_eq!(
        load(&json).map(|_| ()),
        Err(LayoutDataError::DuplicateKey {
            path: format!("{MERGE}.parents")
        })
    );
}

#[test]
fn gitgraph_layout_rejects_a_duplicate_top_level_key() {
    let json = format!(r#"{{"layout":{{}},{}"#, &control()[1..]);
    assert_eq!(
        load(&json).map(|_| ()),
        Err(LayoutDataError::DuplicateKey {
            path: "layout".to_string()
        })
    );
}

#[test]
fn gitgraph_layout_rejects_a_duplicate_key_in_a_field_it_does_not_read() {
    let json = control().replacen(r#""message":"""#, r#""message":"","message":"""#, 1);
    assert!(
        matches!(load(&json), Err(LayoutDataError::DuplicateKey { path }) if path.ends_with(".message"))
    );
}

// ------------------------------------------------------------------ AC-C11

#[test]
fn gitgraph_layout_labeled_merge_keeps_both_parents() {
    let geometry = laid_out(
        include_str!("fixtures/merman/gitgraph/labeled_merge.mmd"),
        &measurer(),
    );
    let merge = geometry.commit("M").expect("the labeled merge");
    assert_eq!(merge.parents, ["A", "B"]);
    assert_eq!(merge.lane, "main");
    assert_eq!(
        merge
            .tags
            .iter()
            .map(|tag| tag.text.as_str())
            .collect::<Vec<_>>(),
        ["v1"]
    );
}

#[test]
fn gitgraph_layout_merge_of_a_branch_without_its_own_commit_lays_out() {
    let geometry = laid_out(
        include_str!("fixtures/merman/gitgraph/empty_branch_merge.mmd"),
        &measurer(),
    );
    let merge = geometry.commit("M").expect("the merge");
    assert_eq!(merge.parents, ["B", "A"]);
}

// ------------------------------------------------------------------ tag boxes

#[test]
fn gitgraph_layout_tag_bounds_match_drawn_tags_left_to_right() {
    assert_tag_bounds_match_drawing(include_str!("fixtures/merman/gitgraph/long_labels.mmd"));
}

#[test]
fn gitgraph_layout_stacked_tag_bounds_match_drawn_tags() {
    assert_tag_bounds_match_drawing(include_str!("fixtures/merman/gitgraph/layout.mmd"));
}

#[test]
fn gitgraph_layout_rotated_tag_bounds_match_drawn_tags_top_to_bottom() {
    let source =
        include_str!("fixtures/merman/gitgraph/layout.mmd").replacen("gitGraph", "gitGraph TB:", 1);
    assert_tag_bounds_match_drawing(&source);
}

#[test]
fn gitgraph_layout_rotated_tag_bounds_match_drawn_tags_bottom_to_top() {
    let source =
        include_str!("fixtures/merman/gitgraph/layout.mmd").replacen("gitGraph", "gitGraph BT:", 1);
    assert_tag_bounds_match_drawing(&source);
}

// ------------------------------------------------------------------ AC-C12 spacing

#[test]
fn gitgraph_layout_spacing_separates_s1_adjacent_tags() {
    let spacing = assert_smallest_clearing_spacing(include_str!(
        "fixtures/merman/gitgraph/tag_collision_adjacent.mmd"
    ));
    assert!(spacing > DEFAULT_COMMIT_SPACING, "{spacing}");
}

#[test]
fn gitgraph_layout_spacing_separates_worktree_long_labels() {
    let spacing =
        assert_smallest_clearing_spacing(include_str!("fixtures/merman/gitgraph/long_labels.mmd"));
    assert!(spacing > DEFAULT_COMMIT_SPACING, "{spacing}");
}

#[test]
fn gitgraph_layout_spacing_separates_worktree_observation() {
    let spacing = assert_smallest_clearing_spacing(include_str!(
        "fixtures/merman/gitgraph/observation_1.mmd"
    ));
    assert!(spacing > DEFAULT_COMMIT_SPACING, "{spacing}");
}

#[test]
fn gitgraph_layout_spacing_reports_the_values_found() {
    // Pins the spacings our measurer gives with the bundled Inter, so a
    // change to the formula or the measurer shows here.
    let measurer = measurer();
    let spacings: Vec<Option<f32>> = [
        include_str!("fixtures/merman/gitgraph/tag_collision_adjacent.mmd"),
        include_str!("fixtures/merman/gitgraph/long_labels.mmd"),
        include_str!("fixtures/merman/gitgraph/observation_1.mmd"),
    ]
    .iter()
    .map(|source| spacing_for(source, &measurer))
    .collect();
    assert_eq!(spacings, [Some(97.42), Some(145.35), Some(69.88)]);
}

#[test]
fn gitgraph_layout_no_tags_sets_no_spacing_without_laying_out() {
    let style = tag_style();
    let measurer = measurer();
    let labels = TagLabels {
        measurer: measurer.as_ref(),
        style: &style,
    };
    let source = "gitGraph\n    commit id: \"A\"\n    commit id: \"B\"\n";
    let spacing = commit_spacing_for(source, &labels, GAP, |_| {
        panic!("a source without tags is not laid out")
    });
    assert_eq!(spacing.expect("no error"), None);
}

#[test]
fn gitgraph_layout_stacked_tags_on_one_commit_set_no_spacing() {
    let source = "gitGraph\n    commit id: \"a1\"\n    commit id: \"a2\" tag: \"main\" tag: \"origin/main\"\n    commit id: \"a3\"\n";
    let measurer = measurer();
    assert!(laid_out(source, &measurer).tag_overlaps().is_empty());
    assert_eq!(spacing_for(source, &measurer), None);
}

#[test]
fn gitgraph_layout_tags_far_apart_set_no_spacing() {
    let source = concat!(
        "gitGraph\n    commit id: \"a1\" tag: \"release/2026-10\"\n",
        "    commit id: \"a2\"\n    commit id: \"a3\"\n    commit id: \"a4\"\n",
        "    commit id: \"a5\" tag: \"release/2026-11\"\n",
    );
    let measurer = measurer();
    assert!(
        min_clearance(&laid_out(source, &measurer)) >= GAP,
        "control: the tags are a full em apart"
    );
    assert_eq!(spacing_for(source, &measurer), None);
}

#[test]
fn gitgraph_layout_vertical_graphs_set_no_spacing() {
    let source = include_str!("fixtures/merman/gitgraph/tag_collision_adjacent.mmd").replacen(
        "gitGraph",
        "gitGraph TB:",
        1,
    );
    assert_eq!(spacing_for(&source, &measurer()), None);
}

#[test]
fn gitgraph_layout_author_directive_spacing_wins() {
    let source = format!(
        "%%{{init: {{\"gitGraph\": {{\"commitSpacing\": 60}}}}}}%%\n{}",
        include_str!("fixtures/merman/gitgraph/tag_collision_adjacent.mmd")
    );
    let measurer = measurer();
    assert_eq!(laid_out(&source, &measurer).commit_step, 60.0);
    assert_eq!(spacing_for(&source, &measurer), None);
}

#[test]
fn gitgraph_layout_author_frontmatter_spacing_wins() {
    let source = format!(
        "---\nconfig:\n  gitGraph:\n    commitSpacing: 60\n---\n{}",
        include_str!("fixtures/merman/gitgraph/tag_collision_adjacent.mmd")
    );
    let measurer = measurer();
    assert_eq!(laid_out(&source, &measurer).commit_step, 60.0);
    assert_eq!(spacing_for(&source, &measurer), None);
}

#[test]
fn gitgraph_layout_written_spacing_composes_with_an_author_directive() {
    let source = format!(
        "%%{{init: {{\"gitGraph\": {{\"showBranches\": false}}}}}}%%\n{}",
        include_str!("fixtures/merman/gitgraph/tag_collision_adjacent.mmd")
    );
    let measurer = measurer();
    let spacing = spacing_for(&source, &measurer).expect("tags collide");
    let json =
        layout_json_with(&with_commit_spacing(&source, spacing), &measurer).expect("lays out");
    let document: Value = serde_json::from_str(&json).expect("JSON");
    assert_eq!(
        document["layout"]["GitGraphDiagram"]["show_branches"],
        json!(false)
    );
    assert!(
        (geometry_of(&json, &measurer).expect("loads").commit_step - spacing).abs() < TOLERANCE
    );
}

#[test]
fn gitgraph_layout_written_spacing_composes_with_author_frontmatter() {
    let source = format!(
        "---\nconfig:\n  gitGraph:\n    showBranches: false\n---\n{}",
        include_str!("fixtures/merman/gitgraph/tag_collision_adjacent.mmd")
    );
    let measurer = measurer();
    let spacing = spacing_for(&source, &measurer).expect("tags collide");
    let json =
        layout_json_with(&with_commit_spacing(&source, spacing), &measurer).expect("lays out");
    let document: Value = serde_json::from_str(&json).expect("JSON");
    assert_eq!(
        document["layout"]["GitGraphDiagram"]["show_branches"],
        json!(false)
    );
    assert!(
        (geometry_of(&json, &measurer).expect("loads").commit_step - spacing).abs() < TOLERANCE
    );
}

#[test]
fn gitgraph_layout_with_commit_spacing_appends_one_directive() {
    assert_eq!(
        with_commit_spacing("gitGraph\n    commit\n", 64.25),
        "gitGraph\n    commit\n%%{init: {\"gitGraph\": {\"commitSpacing\": 64.25}}}%%\n"
    );
    assert_eq!(
        with_commit_spacing("gitGraph\n    commit", 64.0),
        "gitGraph\n    commit\n%%{init: {\"gitGraph\": {\"commitSpacing\": 64}}}%%\n"
    );
}

#[test]
fn gitgraph_layout_spacing_reports_a_bad_layout_document() {
    let style = tag_style();
    let measurer = measurer();
    let labels = TagLabels {
        measurer: measurer.as_ref(),
        style: &style,
    };
    let result = commit_spacing_for("gitGraph\n    commit tag: \"v\"\n", &labels, GAP, |_| {
        Ok("{} []".to_string())
    });
    assert!(
        matches!(result, Err(MermaidError::RenderFailed(message)) if message.contains("trailing"))
    );
}
