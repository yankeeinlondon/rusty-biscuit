//! Compiled themes (D6, R-T1–R-T3, R-T5–R-T7, R-T9, R-V4, R-V5): the
//! per-family designs, Merman's evidence for them, the precedence surprises
//! they rely on, the accent post-process, an author's `%%{init}%%`, the
//! palette in the cache key, and id-scoped styles.

use merman::diagram_theme::{
    ThemeRuleFacetV1, ThemeSupportOutputV1, ThemeSupportQueryV1, ThemeSupportStateV1, describe_theme_support,
};
use serde_json::{Value, json};
use serial_test::serial;

use crate::artifact::{Background, OutputFormat, RenderRequest};
use crate::mermaid::accents::apply_accents;
use crate::mermaid::design::{self, design_json};
use crate::mermaid::{MermaidDiagram, MermaidTheme, Palette, Rgb};
use crate::tests::isolated_cache_dir;

const FIXED: [MermaidTheme; 4] = [MermaidTheme::Dark, MermaidTheme::Default, MermaidTheme::Forest, MermaidTheme::Neutral];

/// One small input per family Merman renders, with Merman's family id.
const FAMILIES: &[(&str, &str)] = &[
    ("flowchart", "flowchart LR\n  A[a] --> B{b} --> C[c]\n  B --> D[d]"),
    ("sequence", "sequenceDiagram\n  A->>B: hi\n  Note over A: n\n  loop every\n    B->>A: yo\n  end\n  activate A\n  A->>B: go\n  deactivate A"),
    ("class", "classDiagram\n  class Animal {\n    +name: string\n    +eat()\n  }\n  Animal <|-- Dog"),
    ("state", "stateDiagram-v2\n  [*] --> A\n  A --> B: go\n  state C {\n    [*] --> D\n  }\n  B --> C\n  C --> [*]"),
    ("er", "erDiagram\n  CUSTOMER ||--o{ ORDER : places\n  ORDER {\n    string id\n  }"),
    ("gantt", "gantt\n  title T\n  dateFormat YYYY-MM-DD\n  section S\n  a :a1, 2024-01-01, 3d\n  b :after a1, 2d"),
    ("pie", "pie title Pets\n  \"Dogs\" : 386\n  \"Cats\" : 85\n  \"Rats\" : 15"),
    ("journey", "journey\n  title My day\n  section Work\n    Code: 5: Me\n    Review: 3: Me, You"),
    ("mindmap", "mindmap\n  root((root))\n    A\n    B\n      C"),
    (design::GITGRAPH_FAMILY, "gitGraph\n  commit\n  branch feature\n  commit\n  checkout main\n  merge feature tag: \"v1\""),
    ("timeline", "timeline\n  title History\n  2020 : a\n  2021 : b : c"),
    ("quadrantChart", "quadrantChart\n  title Q\n  quadrant-1 One\n  A: [0.3, 0.6]\n  B: [0.7, 0.2]"),
    ("requirement", "requirementDiagram\n  requirement r {\n    id: 1\n    text: t\n  }\n  element e {\n    type: x\n  }\n  e - satisfies -> r"),
    ("c4", "C4Context\n  title C\n  Person(p, \"P\")\n  System(s, \"S\")\n  Rel(p, s, \"uses\")"),
    ("block", "block-beta\n  columns 2\n  a b\n  a --> b"),
    ("kanban", "kanban\n  todo[Todo]\n    t1[Write]\n  done[Done]\n    t2[Ship]"),
    ("architecture", "architecture-beta\n  service a(server)[A]\n  service b(database)[B]\n  a:R -- L:b"),
    ("packet", "packet-beta\n  0-15: \"Source\"\n  16-31: \"Destination\""),
    ("radar", "radar-beta\n  axis a, b, c\n  curve x{1, 2, 3}\n  curve y{3, 2, 1}"),
    ("sankey", "sankey-beta\n  a,b,10\n  b,c,5\n  a,c,3"),
    ("treemap", "treemap-beta\n  \"A\"\n    \"B\": 10\n    \"C\": 5"),
    ("xychart", "xychart-beta\n  title \"Sales\"\n  x-axis [a, b, c]\n  bar [1, 2, 3]"),
];

fn svg_request() -> RenderRequest {
    RenderRequest { format: OutputFormat::Svg, background: Background::Transparent, ..RenderRequest::default() }
}

fn render(diagram: &MermaidDiagram) -> String {
    let artifact = diagram.render(&svg_request()).expect("the diagram renders");
    std::fs::read_to_string(artifact.path).expect("artifact")
}

fn admitted(state: ThemeSupportStateV1) -> bool {
    matches!(state, ThemeSupportStateV1::Unconditional | ThemeSupportStateV1::Conditional)
}

// ---------------------------------------------------------------------------
// The designs (AC-T1)
// ---------------------------------------------------------------------------

/// Every fixed palette's design for every family holds only facets Merman
/// reports supported or conditional, never a node base fill beside a node
/// ordinal palette, Mermaid variables only in gitGraph (and only label
/// colors), no stroke widths, and no font (R-T2, R-T3, R-T9).
#[test]
fn each_fixed_palette_design_holds_only_supported_facets() {
    let output = ThemeSupportOutputV1::StandaloneSvg;
    for theme in FIXED {
        let palette = Palette::fixed(theme);
        for (family, _) in FAMILIES {
            let design = design_json(&palette, family);
            let context = format!("{theme:?}/{family}");
            for (target, style) in design["styles"].as_object().expect("styles") {
                for (key, facet) in [("fill", ThemeRuleFacetV1::Fill), ("stroke", ThemeRuleFacetV1::StrokePaint)] {
                    if style.get(key).is_some() {
                        let query = ThemeSupportQueryV1::rule(*family, output, target, facet);
                        assert!(admitted(describe_theme_support(&query).state()), "{context}: {target}.{key}");
                    }
                }
                assert!(style.get("stroke").and_then(|stroke| stroke.get("width")).is_none(), "{context}: {target} width");
                assert!(style.get("typography").is_none(), "{context}: {target} font");
            }
            for target in design["palettes"].as_object().expect("palettes").keys() {
                let query = ThemeSupportQueryV1::ordinal_palette(*family, output, target);
                assert!(admitted(describe_theme_support(&query).state()), "{context}: palette {target}");
                assert!(design["styles"][target].get("fill").is_none(), "{context}: {target} fill beside its ordinal palette");
            }
            match *family {
                design::GITGRAPH_FAMILY => {
                    let variables = design["mermaid"]["variables"].as_object().expect("gitGraph variables");
                    for key in variables.keys() {
                        let label = key.starts_with("commitLabel")
                            || key.starts_with("tagLabel")
                            || key.starts_with("gitBranchLabel")
                            || (key.starts_with("git") && key[3..].parse::<u8>().is_ok());
                        assert!(label, "{context}: variable {key}");
                    }
                }
                _ => assert!(design.get("mermaid").is_none(), "{context}: Mermaid variables"),
            }
            design::compile(&design).unwrap_or_else(|error| panic!("{context}: {error}"));
        }
    }
}

/// The site config's font stack, the only font the styling sets, ends in
/// `sans-serif` (R-T9).
#[test]
fn the_only_font_stack_set_ends_in_sans_serif() {
    for (family, source) in FAMILIES {
        let config = MermaidDiagram::new(*source).prepared_site_config();
        let stack = config["themeVariables"]["fontFamily"].as_str().expect("font stack");
        assert!(stack.trim().ends_with("sans-serif"), "{family}: {stack}");
    }
}

/// Families whose design carries an ordinal palette (and sequence and class)
/// get no palette in the site config, which would erase it; the others get
/// the palette as `base` variables.
#[test]
fn site_palette_variables_go_only_where_they_cannot_erase_the_design() {
    let palette = Palette::fixed(MermaidTheme::Dark);
    for (family, source) in FAMILIES {
        let config = MermaidDiagram::new(*source).with_palette(palette).prepared_site_config();
        let variables = &config["themeVariables"];
        let design = design_json(&palette, family);
        let expected = design::wants_site_palette(family, &design);
        assert_eq!(variables.get("primaryColor").is_some(), expected, "{family}");
        if expected {
            assert_eq!(variables["primaryColor"], palette.surface().to_string(), "{family}");
            assert_eq!(variables["darkMode"], true, "{family}");
        }
    }
    for family in ["flowchart", "sequence", "class", "mindmap", design::GITGRAPH_FAMILY, "pie", "state"] {
        assert!(!design::wants_site_palette(family, &design_json(&palette, family)), "{family}");
    }
}

// ---------------------------------------------------------------------------
// Merman's evidence (AC-T6, R-V4, D62)
// ---------------------------------------------------------------------------

/// Each family's compiled theme gets the evidence status we expect and no
/// `unsupported-*` diagnostic. gitGraph is `Residual` only because Merman
/// cannot verify its Mermaid label variables (`compatibility-unverified`).
/// The budget is wide: it is not under test, and a loaded runner must not
/// turn this into a timeout.
fn assert_family_evidence(family: &str) {
    let source = FAMILIES
        .iter()
        .find(|(id, _)| *id == family)
        .map(|(_, source)| *source)
        .unwrap_or_else(|| panic!("{family} is not in FAMILIES"));
    let (status, diagnostics) = MermaidDiagram::new(source)
        .with_time_budget(std::time::Duration::from_secs(60))
        .theme_evidence()
        .unwrap_or_else(|error| panic!("{family}: {error}"));
    let expected = if family == design::GITGRAPH_FAMILY { "Residual" } else { "Verified" };
    assert_eq!(status, expected, "{family}: {diagnostics:?}");
    assert!(diagnostics.iter().all(|diagnostic| !diagnostic.starts_with("unsupported-")), "{family}: {diagnostics:?}");
    if family == design::GITGRAPH_FAMILY {
        assert_eq!(diagnostics, ["compatibility-unverified:compatibility:-"], "{family}");
    }
}

macro_rules! family_evidence_tests {
    ($($test:ident => $family:expr),+ $(,)?) => {
        $(
            #[test]
            fn $test() {
                assert_family_evidence($family);
            }
        )+

        /// Every family in `FAMILIES` has its own evidence test.
        #[test]
        fn every_family_has_an_evidence_test() {
            let tested: Vec<&str> = vec![$($family),+];
            let listed: Vec<&str> = FAMILIES.iter().map(|(family, _)| *family).collect();
            assert_eq!(tested, listed);
        }
    };
}

family_evidence_tests! {
    evidence_flowchart => "flowchart",
    evidence_sequence => "sequence",
    evidence_class => "class",
    evidence_state => "state",
    evidence_er => "er",
    evidence_gantt => "gantt",
    evidence_pie => "pie",
    evidence_journey => "journey",
    evidence_mindmap => "mindmap",
    evidence_gitgraph => design::GITGRAPH_FAMILY,
    evidence_timeline => "timeline",
    evidence_quadrant_chart => "quadrantChart",
    evidence_requirement => "requirement",
    evidence_c4 => "c4",
    evidence_block => "block",
    evidence_kanban => "kanban",
    evidence_architecture => "architecture",
    evidence_packet => "packet",
    evidence_radar => "radar",
    evidence_sankey => "sankey",
    evidence_treemap => "treemap",
    evidence_xychart => "xychart",
}

/// D62: a flowchart circle draws painted but Merman reports
/// `unsupported-paint` for its ordinal palette and node rule. That one
/// known-false diagnostic is allow-listed, for flowchart only; nothing else
/// may appear.
#[test]
#[serial]
fn the_circles_known_false_unsupported_paint_is_the_only_allowed_diagnostic() {
    let _cache_dir = isolated_cache_dir();
    // The circles' known-false `unsupported-paint` (D62), per flowchart target.
    const ALLOWED: [&str; 2] = ["unsupported-paint:ordinal-palette:node", "unsupported-paint:rule:node"];
    let diagram = MermaidDiagram::new("flowchart LR\n  A((circle)) --> B[box]");
    let (_, diagnostics) = diagram.theme_evidence().expect("renders");
    assert!(!diagnostics.is_empty(), "the circle diagnostic disappeared; drop the allow-list");
    assert!(diagnostics.iter().all(|diagnostic| ALLOWED.contains(&diagnostic.as_str())), "{diagnostics:?}");

    // The circle is painted all the same: its fill is the second tint.
    let svg = render(&diagram);
    let palette = diagram.palette();
    assert!(svg.contains(&format!("fill:{} !important", palette.tints()[0])), "{svg}");
}

// ---------------------------------------------------------------------------
// Precedence guards (AC-T7, R-V5): Merman behaviors the design rules rely on
// ---------------------------------------------------------------------------

/// A Merman render of `source` under `design` alone, with the font-only site
/// config the compiled families get.
fn raw_merman(source: &str, design: Value) -> (String, String) {
    use merman::svg::SvgPipeline;
    use merman::{Engine, OperationControl, RenderOutput, Renderer, SvgRequest};

    let mut request = SvgRequest::default();
    request.options.diagram_id = Some("guard".to_string());
    request.pipeline = Some(SvgPipeline::resvg_safe());
    let theme = design::compile(&design).expect("guard design compiles");
    let site = json!({"theme": "base", "themeVariables": {"fontFamily": "sans-serif"}});
    let renderer = Renderer::new().with_engine(Engine::new().with_site_config(merman::MermaidConfig::from_value(site)));
    match renderer.render(merman::RenderRequest::svg(source, OperationControl::new(), request).with_theme(theme)) {
        Ok(RenderOutput::Svg(Some(svg))) => {
            let status = format!("{:?}", svg.evidence().theme_evidence().status());
            (svg.svg().to_string(), status)
        }
        other => panic!("guard render: {other:?}"),
    }
}

const GUARD_FLOWCHART: &str = "flowchart LR\n  A[one] --> B[two]";

/// Surprise 1: a static `styles.node.fill` beats `palettes.node`, so the
/// designs never set both.
#[test]
fn guard_a_static_node_fill_beats_the_ordinal_palette() {
    let (svg, _) = raw_merman(
        GUARD_FLOWCHART,
        json!({"styles": {"node": {"fill": "#123456"}}, "palettes": {"node": ["#aa0000", "#00aa00"]}}),
    );
    assert!(svg.contains("#123456"));
    assert!(!svg.contains("#aa0000") && !svg.contains("#00aa00"), "the palette won; the rule may be relaxed");
}

/// Surprise 2: an unsupported rule erases a supported one (an ordinal stroke
/// rule removes the static node stroke), so the designs emit only supported
/// facets.
#[test]
fn guard_an_unsupported_rule_erases_a_supported_one() {
    let (svg, _) = raw_merman(
        GUARD_FLOWCHART,
        json!({
            "styles": {"node": {"stroke": {"paint": "#123456"}}},
            "rules": [
                {"family": "flowchart", "target": "node", "ordinal": {"cycle": {"period": 2, "offset": 0}}, "style": {"stroke": {"paint": "#aa0000"}}},
                {"family": "flowchart", "target": "node", "ordinal": {"cycle": {"period": 2, "offset": 1}}, "style": {"stroke": {"paint": "#00aa00"}}}
            ]
        }),
    );
    assert!(!svg.contains("#123456"), "the static stroke survived an unsupported rule");
}

/// Surprise 3: Mermaid variables override the ordinal palette in a flowchart
/// and downgrade sequence evidence from `Verified`, so only gitGraph's design
/// carries them.
#[test]
fn guard_mermaid_variables_override_the_palette_and_downgrade_sequence() {
    let variables = json!({"theme": "base", "variables": {"primaryColor": "#123456"}});
    let (svg, _) = raw_merman(GUARD_FLOWCHART, json!({"palettes": {"node": ["#aa0000", "#00aa00"]}, "mermaid": variables}));
    assert!(svg.contains("#123456") && !svg.contains("#aa0000"), "the palette beat the variables");

    let sequence = "sequenceDiagram\n  A->>B: hi";
    let actor = json!({"actor": {"fill": "#272937"}});
    let (_, plain) = raw_merman(sequence, json!({"styles": actor}));
    let (_, with_variables) = raw_merman(sequence, json!({"styles": actor, "mermaid": variables}));
    assert_eq!(plain, "Verified");
    assert_ne!(with_variables, "Verified");
}

// ---------------------------------------------------------------------------
// Accents (AC-T3, R-T5, R-T6)
// ---------------------------------------------------------------------------

/// Un-styled flowchart nodes get rotating accent strokes over their tints; a
/// `classDef` node and a `style` node keep the author's paint.
#[test]
#[serial]
fn flowchart_nodes_get_rotating_accents_and_author_styles_are_kept() {
    let _cache_dir = isolated_cache_dir();
    let source = concat!(
        "flowchart LR\n  A[one] --> B[two] --> C[three]\n",
        "  D[warn]:::w\n  E[styled]\n",
        "  classDef w fill:#550000,stroke:#ff0000\n  style E fill:#003300,stroke:#00ff00\n"
    );
    for theme in FIXED {
        let palette = Palette::fixed(theme);
        let svg = render(&MermaidDiagram::new(source).with_palette(palette));
        for index in 0..3 {
            let pair = format!("fill:{} !important;stroke:{} !important", palette.tints()[index], palette.accents()[index]);
            assert!(svg.contains(&pair), "{theme:?}: node {index}");
        }
        assert!(svg.contains("#550000") && svg.contains("#ff0000"), "{theme:?}: classDef");
        assert!(svg.contains("#003300") && svg.contains("#00ff00"), "{theme:?}: style");
        assert!(!svg.contains(&format!("stroke:{} !important", palette.border())), "{theme:?}: an un-accented node");
    }
}

/// Class boxes take one accent each, in document order: the box fill becomes
/// the accent's tint and its outline and dividers the accent.
#[test]
#[serial]
fn classes_take_accents_in_document_order() {
    let _cache_dir = isolated_cache_dir();
    let palette = Palette::fixed(MermaidTheme::Default);
    let svg = render(
        &MermaidDiagram::new("classDiagram\n  class First {\n    +a\n    +b()\n  }\n  class Second\n  class Third\n  First <|-- Second")
            .with_palette(palette),
    );
    let first_of = |color: Rgb| svg.find(&color.to_string());
    let positions: Vec<usize> = (0..3)
        .map(|index| first_of(palette.tints()[index]).unwrap_or_else(|| panic!("class {index}: {svg}")))
        .collect();
    assert!(positions.windows(2).all(|pair| pair[0] < pair[1]), "{positions:?}");
    for index in 0..3 {
        assert!(svg.contains(&format!("stroke:{} !important", palette.accents()[index])), "class {index}");
    }
    assert!(!svg.contains(&format!("fill:{} !important", palette.surface())), "an un-accented class box");
}

/// Each participant keeps one accent, top box and bottom box alike.
#[test]
#[serial]
fn sequence_participants_take_one_accent_each() {
    let _cache_dir = isolated_cache_dir();
    let palette = Palette::fixed(MermaidTheme::Dark);
    let svg = render(&MermaidDiagram::new("sequenceDiagram\n  participant A\n  participant B\n  participant C\n  A->>B: x\n  B->>C: y").with_palette(palette));
    for (index, name) in ["A", "B", "C"].iter().enumerate() {
        let accent = palette.accents()[index];
        let boxes: Vec<&str> = svg
            .match_indices(&format!(" name=\"{name}\""))
            .map(|(at, _)| &svg[svg[..at].rfind('<').unwrap()..at + svg[at..].find('>').unwrap()])
            .filter(|tag| tag.starts_with("<rect"))
            .collect();
        assert_eq!(boxes.len(), 2, "{name}: top and bottom box");
        for tag in boxes {
            assert!(tag.contains(&format!("stroke:{accent} !important")), "{name}: {tag}");
        }
    }
}

/// Mindmap accents come from the compiled theme's ordinal palette, with no
/// post-process edit.
#[test]
#[serial]
fn mindmap_accents_come_from_the_compiled_theme() {
    let _cache_dir = isolated_cache_dir();
    let palette = Palette::fixed(MermaidTheme::Dark);
    let diagram = MermaidDiagram::new("mindmap\n  root((root))\n    A\n    B\n    C").with_palette(palette);
    let fills: Vec<String> = diagram.prepared_design().expect("a design")["palettes"]["node"]
        .as_array()
        .expect("node palette")
        .iter()
        .map(|color| color.as_str().unwrap().to_string())
        .collect();
    let svg = render(&diagram);
    assert!(fills.iter().take(3).all(|fill| svg.contains(fill.as_str())), "{fills:?}");
    // Merman's mindmap labels (`#333`) read on every fill.
    let label = Rgb::new(0x33, 0x33, 0x33);
    assert!(fills.iter().all(|fill| Rgb::from_hex(fill).unwrap().contrast(label) >= 4.5));
}

/// The post-process rewrites only paint the palette emitted (R-T6): an SVG
/// whose paint is all the author's, even in the same declaration shape, is
/// returned untouched.
#[test]
fn the_accent_post_process_touches_no_paint_the_palette_did_not_emit() {
    let palette = Palette::fixed(MermaidTheme::Dark);
    let author = concat!(
        r##"<svg><rect class="label-container" style="fill:#550000 !important;stroke:#ff0000 !important"/>"##,
        r##"<path class="node" fill="#222222" style="fill:#222222 !important"/>"##,
        r##"<rect name="Alice" x="0"/></svg>"##
    );
    for family in ["flowchart", "class", "sequence", "mindmap"] {
        assert!(matches!(apply_accents(author, &palette, family), std::borrow::Cow::Borrowed(_)), "{family}");
    }
}

// ---------------------------------------------------------------------------
// The author's own theme keys (AC-T8, D54)
// ---------------------------------------------------------------------------

/// A color an author sets in `%%{init}%%` or frontmatter wins: no compiled
/// theme applies and the author's value is in the SVG, with the palette
/// beneath it for the keys the author left alone.
#[test]
#[serial]
fn an_author_init_color_wins_over_the_compiled_theme() {
    let _cache_dir = isolated_cache_dir();
    let palette = Palette::fixed(MermaidTheme::Dark);
    let flowchart = "flowchart LR\n  A[one] --> B[two]";
    for source in [
        format!("%%{{init: {{\"themeVariables\": {{\"primaryColor\": \"#ff00ff\"}}}}}}%%\n{flowchart}"),
        format!("---\nconfig:\n  themeVariables:\n    primaryColor: '#ff00ff'\n---\n{flowchart}"),
    ] {
        let diagram = MermaidDiagram::new(source.as_str()).with_palette(palette);
        assert_eq!(diagram.prepared_design(), None, "{source}");
        let config = diagram.prepared_site_config();
        assert_eq!(config["themeVariables"]["lineColor"], palette.line().to_string());
        let svg = render(&diagram);
        assert!(svg.to_ascii_lowercase().contains("#ff00ff"), "{source}");
        assert!(!svg.contains(&palette.tints()[0].to_string()), "{source}: the compiled palette applied");
    }
}

/// A theme variable that is not a color (a width) cannot conflict with the
/// designs, which set colors only, so the compiled theme still applies; an
/// author's named theme owns every color.
#[test]
fn only_author_colors_or_a_named_theme_displace_the_compiled_theme() {
    let width = "---\nconfig:\n  themeVariables:\n    pieOuterStrokeWidth: \"5px\"\n---\npie\n  \"a\" : 1";
    assert!(MermaidDiagram::new(width).prepared_design().is_some());

    let named = "%%{init: {\"theme\": \"forest\"}}%%\nflowchart LR\n  A --> B";
    let diagram = MermaidDiagram::new(named);
    assert_eq!(diagram.prepared_design(), None);
    assert!(diagram.prepared_site_config()["themeVariables"].get("primaryColor").is_none());

    let plain = MermaidDiagram::new("flowchart LR\n  A --> B");
    assert!(plain.prepared_design().is_some());
}

// ---------------------------------------------------------------------------
// The palette in the cache key (AC-P4, R-P6)
// ---------------------------------------------------------------------------

/// Changing a single palette color changes the key; the same palette gives
/// the same key.
#[test]
fn one_palette_color_changes_the_cache_key() {
    let request = RenderRequest::default();
    let chain = crate::fonts::default_chain();
    let accents = ["#f7768e", "#9ece6a", "#e0af68", "#7aa2f7", "#bb9af7", "#7dcfff"].map(|hex| Rgb::from_hex(hex).unwrap());
    let base = Palette::new(Rgb::from_hex("#1a1b26").unwrap(), Rgb::from_hex("#c0caf5").unwrap(), accents);
    let mut shifted_accents = accents;
    shifted_accents[3] = Rgb::from_hex("#7aa2f8").unwrap();
    let shifted = Palette::new(base.background(), base.text(), shifted_accents);
    let key = |palette: Palette| {
        MermaidDiagram::new("flowchart LR\n  A --> B --> C --> D --> E")
            .with_palette(palette)
            .cache_key_with(chain, &request)
            .unwrap()
    };

    assert_ne!(key(base), key(shifted));
    assert_eq!(key(base), key(base));
    assert_ne!(key(base), key(Palette::new(Rgb::from_hex("#1a1b27").unwrap(), base.text(), accents)));
}

// ---------------------------------------------------------------------------
// Scoped styles (AC-T2, R-T7)
// ---------------------------------------------------------------------------

/// Two diagrams embedded in one document cannot restyle each other: their
/// root ids differ, and every selector in every stylesheet of each SVG
/// starts with that SVG's own id.
#[test]
#[serial]
fn two_diagrams_in_one_document_keep_their_own_styles() {
    let _cache_dir = isolated_cache_dir();
    let first = render(&MermaidDiagram::new("flowchart LR\n  A --> B").with_theme(MermaidTheme::Dark));
    let second = render(&MermaidDiagram::new("sequenceDiagram\n  A->>B: hi").with_theme(MermaidTheme::Forest));
    let id = |svg: &str| {
        let tag = &svg[svg.find("<svg").unwrap()..];
        let start = tag.find(" id=\"").unwrap() + 5;
        tag[start..start + tag[start..].find('"').unwrap()].to_string()
    };
    let (first_id, second_id) = (id(&first), id(&second));
    assert_ne!(first_id, second_id);

    let document = format!("<html><body>{first}{second}</body></html>");
    for (svg, own) in [(&first, &first_id), (&second, &second_id)] {
        let mut sheets = 0;
        for block in svg.split("<style>").skip(1) {
            sheets += 1;
            let css = &block[..block.find("</style>").unwrap()];
            for rule in css.split('}') {
                let Some((selectors, _)) = rule.split_once('{') else { continue };
                let selectors = selectors.trim();
                if selectors.is_empty() || selectors.starts_with('@') {
                    continue;
                }
                for selector in selectors.split(',') {
                    assert!(selector.trim().starts_with(&format!("#{own}")), "{own}: {selector}");
                }
            }
        }
        assert!(sheets >= 1, "{own}: no stylesheet");
    }
    assert!(document.contains(&first_id) && document.contains(&second_id));
}

/// R-T8 on the input that first failed: an author's pink `style` fill on a
/// requirement element under a dark palette. The author's fill stays, and its
/// labels are recolored to read on it (Merman draws them in the palette's
/// light text otherwise).
#[test]
#[serial]
fn labels_on_an_author_fill_are_made_readable() {
    let _cache_dir = isolated_cache_dir();
    let source = concat!(
        "requirementDiagram\n\n",
        "requirement test_req:::important {\n    id: 1\n    text: \"class styling example\"\n    risk: low\n    verifymethod: test\n}\n\n",
        "element test_entity {\n    type: simulation\n}\n\n",
        "classDef important font-weight:bold\n\nclass test_entity important\nstyle test_entity fill:#f9f,stroke:#333\n"
    );
    let palette = Palette::fixed(MermaidTheme::Dark);
    let svg = render(&MermaidDiagram::new(source).with_palette(palette));
    assert!(svg.contains("fill:#f9f !important"), "the author's fill");

    let pink = Rgb::from_hex("#ff99ff").unwrap();
    let label = |text: &str| {
        let at = svg.find(&format!(">{text}</text>")).unwrap_or_else(|| panic!("no {text} label"));
        let tag = &svg[svg[..at].rfind("<text").unwrap()..at];
        let fill = tag.split("fill:").nth(1).and_then(|rest| rest.split([';', '"']).next()).unwrap();
        Rgb::from_hex(fill.trim()).unwrap_or_else(|| panic!("{text}: {fill}"))
    };
    for text in ["test_entity", "Type: simulation"] {
        assert!(label(text).contrast(pink) >= 4.5, "{text} on the author's pink");
    }
    // The requirement on the palette's own surface keeps the palette text.
    assert_eq!(label("test_req"), palette.text());
}

/// Packet reads its colors from its own `packet` config, which the palette
/// fills (so its labels read on a dark palette); a key the author set in the
/// source keeps the author's value.
#[test]
#[serial]
fn packet_colors_follow_the_palette_and_the_author_keeps_theirs() {
    let _cache_dir = isolated_cache_dir();
    let palette = Palette::fixed(MermaidTheme::Dark);
    let source = "packet-beta\n  0-15: \"Source\"\n  16-31: \"Destination\"";
    let diagram = MermaidDiagram::new(source).with_palette(palette);
    let config = diagram.prepared_site_config();
    assert_eq!(config["packet"]["blockFillColor"], palette.surface().to_string());
    assert_eq!(config["packet"]["labelColor"], palette.text().to_string());
    let svg = render(&diagram);
    assert!(svg.contains(&palette.surface().to_string()), "the block fill");

    let authored = format!("---\nconfig:\n  packet:\n    blockFillColor: '#123456'\n---\n{source}");
    let config = MermaidDiagram::new(authored.as_str()).with_palette(palette).prepared_site_config();
    assert!(config["packet"].get("blockFillColor").is_none(), "{config}");
    assert_eq!(config["packet"]["labelColor"], palette.text().to_string());
    assert!(render(&MermaidDiagram::new(authored.as_str()).with_palette(palette)).contains("#123456"));
}
