//! A [`Palette`] as one Merman `ThemeDesignV1` per diagram family (D6).
//!
//! Every family starts from the same role table ([`role_styles`]); a style
//! facet or ordinal palette reaches a family's design only when
//! `describe_theme_support` reports it `unconditional` or `conditional` for
//! that family (R-T2), so each design holds exactly what Merman says it can
//! apply. Three rules from the S0 spike's precedence surprises shape the
//! table:
//!
//! - a static fill never shares a target with an ordinal palette (a static
//!   `styles.node.fill` beats `palettes.node`), so where a family supports a
//!   node ordinal palette the node base fill is left unset;
//! - no rule a family does not support is emitted (an unsupported rule erases
//!   a supported one), which the support filter guarantees;
//! - `design.mermaid.variables` appear only in gitGraph's design, and only
//!   for commit, tag, and branch label colors (they override the ordinal
//!   palette elsewhere, R-T3).
//!
//! No design sets a font: the site config's stack, which ends in
//! `sans-serif`, is the only one (R-T9).

use merman::diagram_theme::{
    DiagramTheme, DiagramThemeCompiler, ThemeDesignV1, ThemeRecipeV1, ThemeRuleFacetV1,
    ThemeSupportOutputV1, ThemeSupportQueryV1, ThemeSupportStateV1, describe_theme_support,
};
use serde_json::{Map, Value, json};

use super::color::{Rgb, TEXT_CONTRAST, most_readable, with_contrast};
use super::error::MermaidError;
use super::palette::Palette;

/// Merman's family id for gitGraph (`git-graph` is unknown to it).
pub(crate) const GITGRAPH_FAMILY: &str = "gitGraph";

/// The color Merman draws mindmap labels in under the `base` theme
/// (`.section-N text{fill:#333}`).
const MINDMAP_LABEL: Rgb = Rgb::new(0x33, 0x33, 0x33);

/// Facets `describe_theme_support` reports `conditional` that Merman's
/// renderer then rejects as `unsupported-paint`, turning the family's evidence
/// `Residual`. They are left out; the guard tests fail if one starts working
/// or another appears.
const REJECTED_DESPITE_SUPPORT: &[(&str, &str)] = &[("kanban", "task-label")];

/// Targets whose fill is left to Merman although it is supported: C4 paints
/// a boundary after the shapes inside it, so a filled boundary hides them
/// (Mermaid's own boundary fill is `none`).
const UNFILLED: &[(&str, &str)] = &[("c4", "cluster")];

/// Merman's commit label background opacity (`.commit-label-bkg{opacity:0.5}`):
/// a label is read over the background or over a branch line it overlaps.
const COMMIT_LABEL_OPACITY: f64 = 0.5;

/// Merman's sankey link opacity (`.link{stroke-opacity:0.5}`): node labels sit
/// on their links.
const SANKEY_LINK_OPACITY: f64 = 0.5;

/// Mermaid's fill for C4 external elements, which Merman keeps.
const C4_EXTERNAL_FILL: Rgb = Rgb::new(0x99, 0x99, 0x99);

/// The C4 element types drawn on [`C4_EXTERNAL_FILL`].
const C4_EXTERNAL_ELEMENTS: [&str; 10] = [
    "external_person",
    "external_system",
    "external_system_db",
    "external_system_queue",
    "external_container",
    "external_container_db",
    "external_container_queue",
    "external_component",
    "external_component_db",
    "external_component_queue",
];

/// The color Merman draws timeline labels in under the `base` theme
/// (`.section-N text{fill:#333}`), like mindmap's.
const TIMELINE_LABEL: Rgb = Rgb::new(0x33, 0x33, 0x33);

/// One target's static paint before the support filter.
struct RoleStyle {
    target: &'static str,
    fill: Option<Rgb>,
    stroke: Option<Rgb>,
}

/// The static paint every family's design starts from. Targets a family does
/// not have are dropped by the support filter.
fn role_styles(palette: &Palette) -> Vec<RoleStyle> {
    let [_, _, yellow, blue, magenta, cyan] = palette.accents();
    let tints = palette.tints();
    let text = palette.text();
    let style = |target, fill, stroke| RoleStyle { target, fill, stroke };
    let label = |target| RoleStyle { target, fill: Some(text), stroke: None };
    let mut styles = vec![
        style("node", Some(palette.surface()), Some(palette.border())),
        style("edge", None, Some(palette.line())),
        style("edge-label-background", Some(palette.background()), None),
        style("cluster", Some(palette.surface_alt()), Some(palette.border())),
        style("marker", Some(palette.line()), Some(palette.line())),
        // Sequence. The actor stroke is the first accent; the accent
        // post-process rotates it per actor.
        style("actor", Some(palette.surface()), Some(palette.accents()[0])),
        style("lifeline", None, Some(palette.border())),
        style("message", None, Some(palette.line())),
        style("loop", None, Some(cyan)),
        style("loop-label-background", Some(palette.surface_alt()), Some(cyan)),
        style("note", Some(tints[2]), Some(yellow)),
        style("activation", Some(palette.surface_alt()), Some(magenta)),
        // State.
        style("state", Some(palette.surface()), Some(palette.border())),
        style("transition", None, Some(palette.line())),
        style("transition-marker", Some(palette.line()), Some(palette.line())),
        style("transition-label-background", Some(palette.background()), None),
        style("composite", Some(palette.surface_alt()), Some(palette.border())),
        style("composite-header", Some(palette.surface_alt()), Some(palette.border())),
        style("special-state", Some(text), Some(text)),
        // ER, requirement, gantt, charts.
        style("entity", Some(palette.surface()), Some(palette.border())),
        style("table", Some(palette.surface_alt()), None),
        style("relation", None, Some(palette.line())),
        style("requirement", Some(palette.surface()), Some(palette.border())),
        style("task", Some(tints[3]), Some(blue)),
        style("pie-slice", None, Some(palette.background())),
        style("journey-task", None, Some(palette.border())),
        style("timeline-event", None, Some(palette.border())),
        style("axis", None, Some(palette.line())),
        style("axis-tick", None, Some(palette.line())),
        style("chart-series", Some(blue), None),
    ];
    styles.extend(
        [
            "node-label",
            "edge-label",
            "cluster-label",
            "title",
            "text",
            "actor-label",
            "message-label",
            "loop-label",
            "note-label",
            "state-label",
            "transition-label",
            "composite-label",
            "task-label",
            "packet-byte-label",
            "packet-field-label",
            "axis-title",
            "axis-label",
            "legend",
        ]
        .map(label),
    );
    styles
}

/// The ordinal palettes, before the support filter: tints where text sits on
/// the fill, full accents where it does not (branches, chart series, sankey
/// nodes).
fn role_palettes(palette: &Palette, family: &str) -> Vec<(&'static str, [Rgb; 6])> {
    let node = match family {
        GITGRAPH_FAMILY => palette.accents(),
        // Merman draws mindmap labels in the `base` theme's fixed label color,
        // which no compiled style reaches: the fills are the accents lightened
        // or darkened until that color reads on them.
        "mindmap" => palette
            .accents()
            .map(|accent| with_contrast(accent, MINDMAP_LABEL, TEXT_CONTRAST)),
        "sankey" => palette.accents().map(|accent| sankey_node(palette, accent)),
        _ => palette.tints(),
    };
    // Timeline labels, like mindmap's, keep `base`'s fixed color.
    let timeline = palette
        .accents()
        .map(|accent| with_contrast(accent, TIMELINE_LABEL, TEXT_CONTRAST));
    vec![
        ("node", node),
        ("state", palette.tints()),
        ("task", palette.tints()),
        ("pie-slice", palette.chart_tints()),
        ("journey-task", palette.tints()),
        ("timeline-event", timeline),
        ("chart-series", palette.accents()),
    ]
}

/// The `ThemeDesignV1` document (its wire JSON) for `family`.
pub(crate) fn design_json(palette: &Palette, family: &str) -> Value {
    let ordinals: Vec<(&str, [Rgb; 6])> = role_palettes(palette, family)
        .into_iter()
        .filter(|(target, _)| supported(family, target, Facet::Ordinal))
        .collect();
    let has_ordinal = |target: &str| ordinals.iter().any(|(ordinal, _)| *ordinal == target);

    let mut styles = Map::new();
    for role in role_styles(palette) {
        if REJECTED_DESPITE_SUPPORT.contains(&(family, role.target)) {
            continue;
        }
        let mut style = Map::new();
        // A static fill beats the target's ordinal palette, so it is left out
        // wherever the palette applies.
        if let Some(fill) = role.fill
            && !has_ordinal(role.target)
            && !UNFILLED.contains(&(family, role.target))
            && supported(family, role.target, Facet::Fill)
        {
            style.insert("fill".to_string(), json!(fill.to_string()));
        }
        if let Some(stroke) = role.stroke
            && supported(family, role.target, Facet::StrokePaint)
        {
            style.insert("stroke".to_string(), json!({ "paint": stroke.to_string() }));
        }
        if !style.is_empty() {
            styles.insert(role.target.to_string(), Value::Object(style));
        }
    }
    let palettes: Map<String, Value> = ordinals
        .iter()
        .map(|(target, colors)| (target.to_string(), json!(colors.map(|color| color.to_string()))))
        .collect();

    let mut design = json!({ "styles": styles, "palettes": palettes });
    if family == GITGRAPH_FAMILY {
        design["mermaid"] = json!({
            "dark_mode": palette.is_dark(),
            "variables": gitgraph_label_variables(palette),
        });
    }
    design
}

/// gitGraph's commit, tag, and branch label colors, which its compiled
/// styles do not reach (S0). Branch `i` uses accent `i mod 6`, as the ordinal
/// node palette does, and its label is the readable color on it.
fn gitgraph_label_variables(palette: &Palette) -> Map<String, Value> {
    let accents = palette.accents();
    let tag = accents[2];
    let mut variables = Map::new();
    let mut set = |key: String, color: Rgb| {
        variables.insert(key, json!(color.to_string()));
    };
    let (label, label_background) = commit_label_colors(palette);
    set("commitLabelColor".to_string(), label);
    set("commitLabelBackground".to_string(), label_background);
    set("tagLabelColor".to_string(), palette.text_on(tag));
    set("tagLabelBackground".to_string(), tag);
    set("tagLabelBorder".to_string(), tag);
    for branch in 0..8 {
        let accent = accents[branch % accents.len()];
        set(format!("git{branch}"), accent);
        set(format!("gitBranchLabel{branch}"), palette.text_on(accent));
    }
    variables
}

/// The commit label's text and background colors: the label background is
/// half transparent, so it is darkened (or lightened) from the second surface
/// until the text reads on it over the background and over every branch
/// color; when the palette text cannot, black or white is used.
fn commit_label_colors(palette: &Palette) -> (Rgb, Rgb) {
    let mut unders = palette.accents().to_vec();
    unders.push(palette.background());
    let away = if palette.text().is_dark() { Rgb::WHITE } else { Rgb::BLACK };
    let extreme = if away == Rgb::BLACK { Rgb::WHITE } else { Rgb::BLACK };
    for text in [palette.text(), extreme] {
        for step in 0..=20 {
            let label_background = away.over(palette.surface_alt(), f64::from(step) / 20.0);
            let reads = unders.iter().all(|under| {
                text.contrast(label_background.over(*under, COMMIT_LABEL_OPACITY)) >= TEXT_CONTRAST
            });
            if reads {
                return (text, label_background);
            }
        }
    }
    (extreme, away)
}

/// A sankey node color: `accent` moved toward the background until the
/// palette text reads on its half-transparent link.
fn sankey_node(palette: &Palette, accent: Rgb) -> Rgb {
    let background = palette.background();
    (0..=20)
        .map(|step| background.over(accent, f64::from(step) / 20.0))
        .find(|node| palette.text().contrast(node.over(background, SANKEY_LINK_OPACITY)) >= TEXT_CONTRAST)
        .unwrap_or(background)
}

/// Whether `family` also gets the palette as Mermaid `base` theme variables
/// in the site config, so the surfaces its compiled styles do not reach
/// (treemap cells, quadrant backgrounds, ER label boxes) follow the palette
/// instead of `base`'s light defaults.
///
/// Not for a family whose design carries an ordinal palette, nor for
/// sequence and class: explicit site variables win over compiled styles and
/// erase ordinal palettes, accents, and static actor and class paint. Those
/// families rely on their compiled design alone, with the contrast pass for
/// any label it does not reach.
pub(crate) fn wants_site_palette(family: &str, design: &Value) -> bool {
    let has_ordinal = design["palettes"].as_object().is_some_and(|palettes| !palettes.is_empty());
    !has_ordinal && !matches!(family, "sequence" | "class")
}

/// Site-config theme variables for colors a family's compiled styles do not
/// reach, each measured not to erase the family's compiled design: pie
/// legend and slice labels; XY chart's plot background and axis text; the
/// surfaces behind labels in gantt (alternate sections), cynefin (domains),
/// event modeling (lanes and boxes), treemap and kanban (sections); and
/// treeView's description text.
pub(crate) fn family_site_variables(family: &str, palette: &Palette) -> Map<String, Value> {
    let text = json!(palette.text().to_string());
    let line = json!(palette.line().to_string());
    let color = |color: Rgb| json!(color.to_string());
    let [red, green, yellow, blue, magenta, _] = palette.tints();
    let mut variables = Map::new();
    match family {
        "gantt" => {
            // Drawn at Merman's 0.2 opacity; `base`'s white turns a dark
            // palette's alternate sections light gray.
            variables.insert("altSectionBkgColor".to_string(), color(palette.background()));
        }
        "cynefin" => {
            let domains = [
                ("complexBg", green),
                ("complicatedBg", blue),
                ("chaoticBg", red),
                ("clearBg", yellow),
                ("confusionBg", magenta),
            ];
            let domains: Map<String, Value> =
                domains.into_iter().map(|(key, tint)| (key.to_string(), color(tint))).collect();
            variables.insert("cynefin".to_string(), Value::Object(domains));
        }
        "treeView" => {
            let description = with_contrast(palette.accents()[1], palette.background(), TEXT_CONTRAST);
            variables.insert("treeView".to_string(), json!({ "descriptionColor": description.to_string() }));
        }
        "eventmodeling" => {
            let [_, green_accent, yellow_accent, blue_accent, magenta_accent, _] = palette.accents();
            let paints = [
                ("emSwimlaneBackgroundOdd", palette.surface_alt()),
                ("emSwimlaneBackgroundStroke", palette.border()),
                ("emUiFill", palette.surface()),
                ("emUiStroke", palette.border()),
                ("emProcessorFill", magenta),
                ("emProcessorStroke", magenta_accent),
                ("emReadModelFill", green),
                ("emReadModelStroke", green_accent),
                ("emCommandFill", blue),
                ("emCommandStroke", blue_accent),
                ("emEventFill", yellow),
                ("emEventStroke", yellow_accent),
                ("emRelationStroke", palette.line()),
                ("emArrowhead", palette.line()),
            ];
            for (key, paint) in paints {
                variables.insert(key.to_string(), color(paint));
            }
        }
        "venn" => {
            // Sets are drawn at Merman's 0.1 fill opacity, their labels in the
            // set color lightened (dark) or darkened (light) by 30 points.
            for (index, accent) in palette.accents().iter().cycle().take(8).enumerate() {
                variables.insert(format!("venn{}", index + 1), color(*accent));
            }
        }
        "treemap" => {
            for (index, tint) in palette.tints().iter().cycle().take(12).enumerate() {
                variables.insert(format!("cScale{index}"), color(*tint));
            }
        }
        "kanban" => {
            // Merman lightens each `cScale` by 10 points of HSL lightness
            // for its column fill; the columns are the second surface.
            let (hue, saturation, lightness) = palette.surface_alt().to_hsl();
            let scale = format!(
                "hsl({hue}, {}%, {}%)",
                saturation * 100.0,
                ((lightness - 0.1) * 100.0).max(0.0)
            );
            for index in 0..12 {
                variables.insert(format!("cScale{index}"), json!(scale));
            }
        }
        "pie" => {
            for key in ["pieTitleTextColor", "pieSectionTextColor", "pieLegendTextColor"] {
                variables.insert(key.to_string(), text.clone());
            }
        }
        "xychart" => {
            let mut chart = Map::new();
            chart.insert("backgroundColor".to_string(), json!(palette.background().to_string()));
            for key in ["titleColor", "xAxisLabelColor", "yAxisLabelColor", "xAxisTitleColor", "yAxisTitleColor"] {
                chart.insert(key.to_string(), text.clone());
            }
            for key in ["xAxisLineColor", "yAxisLineColor", "xAxisTickColor", "yAxisTickColor"] {
                chart.insert(key.to_string(), line.clone());
            }
            variables.insert("xyChart".to_string(), Value::Object(chart));
        }
        _ => {}
    }
    variables
}

/// Top-level site-config sections for families that read their colors from
/// their own config rather than from theme variables (packet's
/// `packet.blockFillColor` and label colors; C4's boundary and relationship
/// text, and the text on its external elements' fixed gray).
pub(crate) fn family_site_config(family: &str, palette: &Palette) -> Map<String, Value> {
    let mut sections = Map::new();
    if family == "c4" {
        let text = json!(palette.text().to_string());
        let mut c4 = Map::new();
        c4.insert("boundary_font_color".to_string(), text.clone());
        c4.insert("relationship_text_color".to_string(), text);
        // Merman's external elements keep Mermaid's `#999999` fill, on which
        // its white text reads at 2.8:1.
        let on_gray = most_readable(C4_EXTERNAL_FILL, &[palette.text(), Rgb::BLACK, Rgb::WHITE]);
        for element in C4_EXTERNAL_ELEMENTS {
            c4.insert(format!("{element}_font_color"), json!(on_gray.to_string()));
        }
        sections.insert("c4".to_string(), Value::Object(c4));
    }
    if family == "packet" {
        let text = json!(palette.text().to_string());
        let mut packet = Map::new();
        packet.insert("blockFillColor".to_string(), json!(palette.surface().to_string()));
        packet.insert("blockStrokeColor".to_string(), json!(palette.border().to_string()));
        for key in ["labelColor", "titleColor", "startByteColor", "endByteColor"] {
            packet.insert(key.to_string(), text.clone());
        }
        sections.insert("packet".to_string(), Value::Object(packet));
    }
    sections
}

/// Compiles `design` (the output of [`design_json`]).
pub(crate) fn compile(design: &Value) -> Result<DiagramTheme, MermaidError> {
    let failed = |error: &dyn std::fmt::Display| MermaidError::RenderFailed(format!("diagram theme: {error}"));
    let design: ThemeDesignV1 = serde_json::from_value(design.clone()).map_err(|error| failed(&error))?;
    DiagramThemeCompiler::new()
        .compile_recipe(ThemeRecipeV1::Design { design })
        .map_err(|error| failed(&error))
}

#[derive(Clone, Copy)]
pub(crate) enum Facet {
    Fill,
    StrokePaint,
    Ordinal,
}

/// Whether Merman reports `facet` of `target` as `unconditional` or
/// `conditional` for `family`'s standalone SVG, the output both of this
/// crate's targets are (the rasterizer draws it; the HTML static target
/// embeds it).
pub(crate) fn supported(family: &str, target: &str, facet: Facet) -> bool {
    let output = ThemeSupportOutputV1::StandaloneSvg;
    let query = match facet {
        Facet::Fill => ThemeSupportQueryV1::rule(family, output, target, ThemeRuleFacetV1::Fill),
        Facet::StrokePaint => ThemeSupportQueryV1::rule(family, output, target, ThemeRuleFacetV1::StrokePaint),
        Facet::Ordinal => ThemeSupportQueryV1::ordinal_palette(family, output, target),
    };
    matches!(
        describe_theme_support(&query).state(),
        ThemeSupportStateV1::Unconditional | ThemeSupportStateV1::Conditional
    )
}
