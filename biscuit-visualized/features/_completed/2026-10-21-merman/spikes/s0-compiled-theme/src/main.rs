//! Spike S0: express a terminal palette as a Merman compiled theme.
//!
//! Builds a `ThemeDesignV1` from a background, a foreground, and the 16 ANSI
//! colors, compiles it with `DiagramThemeCompiler`, renders a flowchart, a
//! sequence diagram, and a gitGraph through `SvgPipeline::resvg_safe()`, and
//! rasterizes each SVG with resvg 0.45. Findings are printed to stdout; the
//! artifacts land in `out/`.

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use merman::diagram_theme::{
    DiagramThemeCompiler, ThemeDesignV1, ThemeRecipeV1, ThemeSupportQueryV1,
    describe_theme_support,
};
use merman::svg::SvgPipeline;
use merman::{OperationControl, RenderOutput, RenderRequest, Renderer, SvgRequest};
use serde_json::{Value, json};

/// A terminal-shaped palette: background, foreground, ANSI 0-15.
struct Palette {
    background: &'static str,
    foreground: &'static str,
    ansi: [&'static str; 16],
}

/// A Tokyo-Night-like dark palette, typed in by hand for the spike.
const PALETTE: Palette = Palette {
    background: "#1a1b26",
    foreground: "#c0caf5",
    ansi: [
        "#15161e", "#f7768e", "#9ece6a", "#e0af68", "#7aa2f7", "#bb9af7", "#7dcfff", "#a9b1d6",
        "#414868", "#ff899d", "#9fe044", "#faba4a", "#8db0ff", "#c7a9ff", "#a4daff", "#c0caf5",
    ],
};

/// ANSI indices used as rotating accents (red, green, yellow, blue, magenta, cyan).
const ACCENT_INDICES: [usize; 6] = [1, 2, 3, 4, 5, 6];

const FLOWCHART: &str = r#"flowchart LR
    A[Detect palette] --> B{Terminal answers?}
    B -->|yes| C[ANSI accents]
    B -->|no| D[Derived accents]
    C --> E[Compile theme]
    D --> E
    E --> F[Render SVG]
    F --> G[Rasterize]
    G --> H((Done))
    X[classDef styled]:::warn --> F
    Y[inline styled] --> F
    classDef warn fill:#550000,stroke:#ff0000,color:#ffffff
    style Y fill:#003300,stroke:#00ff00,color:#ffffff
"#;

const SEQUENCE: &str = r#"sequenceDiagram
    participant Host
    participant Terminal
    participant Merman
    Host->>Terminal: OSC 4 query
    Terminal-->>Host: ANSI palette
    Host->>Merman: render(source, theme)
    activate Merman
    Note over Host,Merman: Compiled theme
    Merman-->>Host: SVG
    deactivate Merman
    loop each diagram
        Host->>Host: rasterize
    end
"#;

const GITGRAPH: &str = r#"gitGraph
    commit id: "init"
    branch feature
    checkout feature
    commit id: "f1"
    commit id: "f2" tag: "v0.1"
    checkout main
    commit id: "m1"
    branch hotfix
    checkout hotfix
    commit id: "h1"
    checkout main
    merge feature
    merge hotfix tag: "v1.0"
    commit id: "m2"
"#;

fn main() -> Result<(), Box<dyn Error>> {
    let out_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("out");
    fs::create_dir_all(&out_dir)?;

    if std::env::args().any(|a| a == "--probe") {
        return probe();
    }
    report_support_queries();

    // Recommended shape: compiled theme only, palette-driven node fills.
    let compiled = Variant::default();
    // Plus Mermaid compatibility variables for what the compiled format misses.
    let with_vars = Variant { mermaid_variables: true, ..Variant::default() };
    // Negative probe: ordinal stroke rules (unsupported) and an unsupported marker style.
    let with_rules = Variant { ordinal_stroke_rules: true, marker_style: true, ..Variant::default() };

    for (file, variant) in [
        ("theme-design.json", compiled),
        ("theme-design-with-vars.json", with_vars),
        ("theme-design-ordinal-stroke-rules.json", with_rules),
    ] {
        fs::write(
            out_dir.join(file),
            serde_json::to_string_pretty(&json!({
                "schema_version": 1, "kind": "design", "design": build_design(&PALETTE, variant)
            }))?,
        )?;
    }
    let compile = |variant: Variant| -> Result<_, Box<dyn Error>> {
        let design: ThemeDesignV1 = serde_json::from_value(build_design(&PALETTE, variant))?;
        let started = Instant::now();
        let theme = DiagramThemeCompiler::new().compile_recipe(ThemeRecipeV1::Design { design })?;
        println!("\ncompile_recipe: {:?}", started.elapsed());
        Ok(theme)
    };
    let theme = compile(compiled)?;
    let theme_vars = compile(with_vars)?;
    let theme_rules = compile(with_rules)?;
    let gitgraph_family = Variant { full_accent_palette: true, mermaid_variables: true, ..Variant::default() };
    fs::write(
        out_dir.join("theme-design-gitgraph.json"),
        serde_json::to_string_pretty(&json!({
            "schema_version": 1, "kind": "design", "design": build_design(&PALETTE, gitgraph_family)
        }))?,
    )?;
    let theme_gitgraph = compile(gitgraph_family)?;
    render_one(&out_dir, "gitgraph-ordinal-rules", GITGRAPH, Some(theme_rules.clone()), false)?;
    render_one(&out_dir, "gitgraph-themed-family", GITGRAPH, Some(theme_gitgraph), false)?;

    let classic = format!("---\nconfig:\n  look: classic\n---\n{FLOWCHART}");
    render_one(&out_dir, "flowchart-ordinal-stroke-rules", FLOWCHART, Some(theme_rules), false)?;
    render_one(&out_dir, "flowchart-classic-themed", &classic, Some(theme.clone()), false)?;
    render_one(&out_dir, "flowchart-themed-accent-postprocess", FLOWCHART, Some(theme.clone()), true)?;

    for (name, source) in [
        ("flowchart", FLOWCHART),
        ("sequence", SEQUENCE),
        ("gitgraph", GITGRAPH),
    ] {
        render_one(&out_dir, &format!("{name}-baseline"), source, None, false)?;
        render_one(&out_dir, &format!("{name}-themed"), source, Some(theme.clone()), false)?;
        render_one(&out_dir, &format!("{name}-themed-vars"), source, Some(theme_vars.clone()), false)?;
    }

    dump_gitgraph_layout(&out_dir)?;
    Ok(())
}

/// Builds the design document. Plain JSON on purpose: the spike measures the
/// wire format a palette type would emit, not a Rust builder.
#[derive(Clone, Copy, Default)]
struct Variant {
    /// Full-strength accents in `palettes.node` (gitGraph branches) instead of
    /// tinted fills (flowchart nodes); palettes cannot be family-scoped.
    full_accent_palette: bool,
    ordinal_stroke_rules: bool,
    marker_style: bool,
    mermaid_variables: bool,
}

fn build_design(palette: &Palette, variant: Variant) -> Value {
    let bg = palette.background;
    let fg = palette.foreground;
    let muted = palette.ansi[8];
    let surface = mix(fg, bg, 0.08);
    let surface_alt = mix(fg, bg, 0.14);
    let accents: Vec<&str> = ACCENT_INDICES.iter().map(|&i| palette.ansi[i]).collect();
    // Tinted node fills: 22% accent over the background keeps labels readable.
    let accent_fills: Vec<String> = accents.iter().map(|a| mix(a, bg, 0.22)).collect();
    let line = mix(fg, bg, 0.55);

    let mut rules: Vec<Value> = Vec::new();
    // Rotating accent stroke per un-styled flowchart node. Ordinal fill comes
    // from `palettes.node`; this probes whether ordinal *stroke* is admitted.
    for (offset, accent) in accents.iter().enumerate().filter(|_| variant.ordinal_stroke_rules) {
        rules.push(json!({
            "family": "flowchart",
            "target": "node",
            "ordinal": {"cycle": {"period": accents.len(), "offset": offset}},
            "style": {"stroke": {"paint": accent}}
        }));
    }

    // Negative probe: family-scoped ordinal fill rules for gitGraph are
    // unsupported and knock commits back to Mermaid's default colors.
    for (offset, accent) in accents.iter().enumerate().filter(|_| variant.ordinal_stroke_rules) {
        rules.push(json!({
            "family": "gitGraph",
            "target": "node",
            "ordinal": {"cycle": {"period": accents.len(), "offset": offset}},
            "style": {"fill": accent}
        }));
    }
    let mut design = json!({
        "tokens": {
            "bg": {"type": "color", "value": bg, "label": "Background"},
            "fg": {"type": "color", "value": fg, "label": "Foreground"}
        },
        "canvas": {"base": {"kind": "solid", "color": {"ref": "bg"}}},
        "styles": {
            // No static fill: a static `styles.node.fill` beats `palettes.node`.
            "node": {"stroke": {"paint": muted, "width": 1.5}},
            "node-label": {"fill": {"kind": "solid", "color": {"ref": "fg"}}},
            "edge": {"stroke": {"paint": line, "width": 1.5}},
            "edge-label": {"fill": {"kind": "solid", "color": {"ref": "fg"}}},
            "edge-label-background": {"fill": {"kind": "solid", "color": {"ref": "bg"}}},
            "cluster": {"fill": surface_alt, "stroke": {"paint": muted}},
            "cluster-label": {"fill": {"kind": "solid", "color": {"ref": "fg"}}},
            "text": {"fill": {"kind": "solid", "color": {"ref": "fg"}}},
            "title": {"fill": {"kind": "solid", "color": {"ref": "fg"}}},
            "actor": {"fill": surface, "stroke": {"paint": accents[3]}},
            "actor-label": {"fill": {"kind": "solid", "color": {"ref": "fg"}}},
            "lifeline": {"stroke": {"paint": muted}},
            "message": {"stroke": {"paint": line}},
            "message-label": {"fill": {"kind": "solid", "color": {"ref": "fg"}}},
            "note": {"fill": mix(accents[2], bg, 0.2), "stroke": {"paint": accents[2]}},
            "note-label": {"fill": {"kind": "solid", "color": {"ref": "fg"}}},
            "activation": {"fill": surface_alt, "stroke": {"paint": accents[4]}},
            "loop": {"stroke": {"paint": accents[5]}},
            "loop-label": {"fill": {"kind": "solid", "color": {"ref": "fg"}}},
            "loop-label-background": {"fill": surface_alt}
        },
        "rules": rules,
        "palettes": {
            // Ordinal node fills: flowchart nodes, gitGraph branches.
            "node": if variant.full_accent_palette {
                accents.iter().map(|a| (*a).to_owned()).collect::<Vec<_>>()
            } else {
                accent_fills
            }
        }
    });
    if variant.marker_style {
        design["styles"]["marker"] = json!({"fill": line, "stroke": {"paint": line}});
    }
    if variant.mermaid_variables {
        // Families and parts the compiled format does not reach (markers,
        // gitGraph branch lines and labels) still read Mermaid themeVariables.
        let mut variables = serde_json::Map::new();
        for (key, value) in [
            ("background", bg.to_owned()),
            ("primaryColor", surface.clone()),
            ("primaryTextColor", fg.to_owned()),
            ("primaryBorderColor", muted.to_owned()),
            ("lineColor", line.clone()),
            ("textColor", fg.to_owned()),
            ("mainBkg", surface.clone()),
            ("clusterBkg", surface_alt.clone()),
            ("titleColor", fg.to_owned()),
            ("commitLabelColor", fg.to_owned()),
            ("commitLabelBackground", surface_alt.clone()),
            ("tagLabelColor", bg.to_owned()),
            ("tagLabelBackground", accents[2].to_owned()),
            ("tagLabelBorder", accents[2].to_owned()),
        ] {
            variables.insert(key.to_owned(), Value::String(value));
        }
        for (i, accent) in accents.iter().chain(accents.iter()).take(8).enumerate() {
            variables.insert(format!("git{i}"), Value::String((*accent).to_owned()));
            variables.insert(format!("gitBranchLabel{i}"), Value::String(bg.to_owned()));
        }
        design["mermaid"] = json!({"theme": "base", "dark_mode": true, "variables": variables});
    }
    design
}

/// Prints the runtime's own support claims for the mechanisms the plan needs.
fn report_support_queries() {
    println!("== describe_theme_support ==");
    let queries = [
        ("flowchart", "node", json!({"kind": "ordinal-palette", "target": "node"})),
        ("flowchart", "node fill", json!({"kind": "rule", "target": "node", "facet": "fill"})),
        ("flowchart", "node stroke-paint", json!({"kind": "rule", "target": "node", "facet": "stroke-paint"})),
        ("flowchart", "edge stroke-paint", json!({"kind": "rule", "target": "edge", "facet": "stroke-paint"})),
        ("flowchart", "node-label fill", json!({"kind": "rule", "target": "node-label", "facet": "fill"})),
        ("flowchart", "canvas fill", json!({"kind": "rule", "target": "canvas", "facet": "fill"})),
        ("flowchart", "marker fill", json!({"kind": "rule", "target": "marker", "facet": "fill"})),
        ("sequence", "actor ordinal", json!({"kind": "ordinal-palette", "target": "actor"})),
        ("sequence", "actor fill", json!({"kind": "rule", "target": "actor", "facet": "fill"})),
        ("sequence", "message stroke-paint", json!({"kind": "rule", "target": "message", "facet": "stroke-paint"})),
        ("gitGraph", "node ordinal", json!({"kind": "ordinal-palette", "target": "node"})),
        ("gitGraph", "edge stroke-paint", json!({"kind": "rule", "target": "edge", "facet": "stroke-paint"})),
        ("gitGraph", "text fill", json!({"kind": "rule", "target": "text", "facet": "fill"})),
        ("gitGraph", "node-label fill", json!({"kind": "rule", "target": "node-label", "facet": "fill"})),
        ("gitGraph", "edge-label fill", json!({"kind": "rule", "target": "edge-label", "facet": "fill"})),
        ("gitGraph", "node fill", json!({"kind": "rule", "target": "node", "facet": "fill"})),
    ];
    for output in ["standalone-svg", "png"] {
        for (family, label, subject) in &queries {
            let query = json!({
                "schema_version": 1, "family": family, "output": output, "subject": subject
            });
            match serde_json::from_value::<ThemeSupportQueryV1>(query) {
                Ok(query) => {
                    let descriptor = describe_theme_support(&query);
                    let value = serde_json::to_value(&descriptor).unwrap_or(Value::Null);
                    println!(
                        "  [{output}] {family:<10} {label:<18} -> {} {}",
                        value["state"], value["reason_ids"]
                    );
                }
                Err(error) => println!("  [{output}] {family} {label}: bad query: {error}"),
            }
        }
    }
}

fn render_one(
    out_dir: &Path,
    name: &str,
    source: &str,
    theme: Option<merman::diagram_theme::DiagramTheme>,
    accent_postprocess: bool,
) -> Result<(), Box<dyn Error>> {
    let mut request = SvgRequest::default();
    request.options.diagram_id = Some(format!("s0-{name}"));
    request.pipeline = Some(SvgPipeline::resvg_safe());
    let mut render_request = RenderRequest::svg(source, OperationControl::new(), request);
    if let Some(theme) = theme {
        render_request = render_request.with_theme(theme);
    }
    let started = Instant::now();
    let output = Renderer::new().render(render_request)?;
    let elapsed = started.elapsed();
    let RenderOutput::Svg(Some(svg)) = output else {
        return Err(format!("{name}: no diagram").into());
    };
    let evidence = svg.evidence();
    let processed;
    let text = if accent_postprocess {
        processed = accent_strokes(svg.svg());
        processed.as_str()
    } else {
        svg.svg()
    };
    fs::write(out_dir.join(format!("{name}.svg")), text)?;

    println!("\n== {name} ({elapsed:?}, {} bytes) ==", text.len());
    println!(
        "  family={:?} theme_evidence={:?} foreignObject={} <style>={}",
        evidence.family_id(),
        evidence.theme_evidence(),
        text.contains("<foreignObject"),
        text.matches("<style").count(),
    );
    for diagnostic in evidence.theme_diagnostics() {
        println!("  diagnostic: {diagnostic:?}");
    }
    let colors = colors_present(text);
    println!("  palette colors present: {colors:?}");

    let started = Instant::now();
    match rasterize(text) {
        Ok(png) => {
            fs::write(out_dir.join(format!("{name}.png")), &png.0)?;
            println!(
                "  resvg 0.45: ok {}x{} in {:?}, corner pixel {}",
                png.1,
                png.2,
                started.elapsed(),
                png.3
            );
        }
        Err(error) => println!("  resvg 0.45: FAILED {error}"),
    }
    Ok(())
}

/// Reports which palette-derived colors made it into the SVG text.
fn colors_present(svg: &str) -> Vec<String> {
    let lower = svg.to_ascii_lowercase();
    let bg = PALETTE.background;
    let mut named = vec![("bg", bg.to_owned()), ("fg", PALETTE.foreground.to_owned())];
    for (slot, &index) in ACCENT_INDICES.iter().enumerate() {
        named.push((["a1", "a2", "a3", "a4", "a5", "a6"][slot], PALETTE.ansi[index].to_owned()));
        named.push((
            ["t1", "t2", "t3", "t4", "t5", "t6"][slot],
            mix(PALETTE.ansi[index], bg, 0.22),
        ));
    }
    named
        .into_iter()
        .filter(|(_, hex)| lower.contains(hex.as_str()))
        .map(|(label, hex)| format!("{label}={hex}"))
        .collect()
}

/// Rasterizes with resvg 0.45 the way biscuit-visualized does (system fonts).
fn rasterize(svg: &str) -> Result<(Vec<u8>, u32, u32, String), Box<dyn Error>> {
    use resvg::{tiny_skia, usvg};
    let mut options = usvg::Options::default();
    options.fontdb_mut().load_system_fonts();
    let tree = usvg::Tree::from_str(svg, &options)?;
    let size = tree.size().to_int_size();
    let (width, height) = (size.width(), size.height());
    let mut pixmap = tiny_skia::Pixmap::new(width, height).ok_or("zero-size pixmap")?;
    resvg::render(&tree, tiny_skia::Transform::identity(), &mut pixmap.as_mut());
    let corner = pixmap.pixel(1, 1).map_or("n/a".to_owned(), |p| {
        format!("#{:02x}{:02x}{:02x}/a{}", p.red(), p.green(), p.blue(), p.alpha())
    });
    Ok((pixmap.encode_png()?, width, height, corner))
}

/// Checks the gitGraph layout JSON shape: commits, parents, branch lanes, tags.
fn dump_gitgraph_layout(out_dir: &Path) -> Result<(), Box<dyn Error>> {
    let output = Renderer::new().render(RenderRequest::layout_json(
        GITGRAPH,
        OperationControl::new(),
        SvgRequest::default(),
    ))?;
    let RenderOutput::LayoutJson(Some(layout)) = output else {
        return Err("gitGraph layout: no diagram".into());
    };
    let mut bytes = Vec::new();
    layout.layout().write_json_pretty(&mut bytes)?;
    fs::write(out_dir.join("gitgraph-layout.json"), &bytes)?;
    let value: Value = serde_json::from_slice(&bytes)?;
    println!("\n== gitGraph layout_json ==");
    let keys = |v: &Value| {
        v.as_object()
            .map(|o| o.keys().cloned().collect::<Vec<_>>().join(","))
            .unwrap_or_default()
    };
    println!("  top-level keys: {}", keys(&value));
    // The layout may be wrapped; find the object that carries `commits`.
    let root = find_with_key(&value["layout"], "commits").unwrap_or(&value);
    println!("  layout keys: {}", keys(root));
    if let Some(commit) = root["commits"].get(0) {
        println!("  commit keys: {}", keys(commit));
    }
    if let Some(branch) = root["branches"].get(0) {
        println!("  branch keys: {}", keys(branch));
    }
    if let Some(arrow) = root["arrows"].get(0) {
        println!("  arrow keys: {}", keys(arrow));
    }
    for commit in root["commits"].as_array().into_iter().flatten() {
        println!(
            "    {} branch={} x={} y={} parents={} tags={}",
            commit["id"], commit["branch"], commit["x"], commit["y"], commit["parents"],
            commit["tags"]
        );
    }
    Ok(())
}

fn find_with_key<'a>(value: &'a Value, key: &str) -> Option<&'a Value> {
    match value {
        Value::Object(map) if map.contains_key(key) => Some(value),
        Value::Object(map) => map.values().find_map(|v| find_with_key(v, key)),
        Value::Array(items) => items.iter().find_map(|v| find_with_key(v, key)),
        _ => None,
    }
}

/// Linear sRGB-space mix of `top` over `bottom` by `amount` (0..=1), as `#rrggbb`.
fn mix(top: &str, bottom: &str, amount: f32) -> String {
    let parse = |hex: &str| {
        let hex = hex.trim_start_matches('#');
        [0, 2, 4].map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap_or(0) as f32)
    };
    let (t, b) = (parse(top), parse(bottom));
    let c: Vec<u8> = (0..3)
        .map(|i| (t[i] * amount + b[i] * (1.0 - amount)).round() as u8)
        .collect();
    format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2])
}

/// Minimal one-facet designs, to isolate which flowchart node paints apply.
fn probe() -> Result<(), Box<dyn Error>> {
    let design_value = match std::env::var("S0_PROBE_DESIGN").as_deref() {
        Ok("main") => build_design(&PALETTE, Variant::default()),
        Ok("fsw") => json!({"styles": {"node": {"fill": "#272937", "stroke": {"paint": "#00aa00", "width": 1.5}}}}),
        _ => json!({
            "styles": {"node": {"stroke": {"paint": "#00aa00"}}},
            "palettes": {"node": ["#aa0000", "#0000aa"]}
        }),
    };
    let design: ThemeDesignV1 = serde_json::from_value(design_value)?;
    let theme = DiagramThemeCompiler::new().compile_recipe(ThemeRecipeV1::Design { design })?;
    for (name, src) in [
        ("rect", "flowchart LR\n  A[a] --> B[b]\n"),
        ("diamond", "flowchart LR\n  A[a] --> B{b}\n"),
        ("circle", "flowchart LR\n  A[a] --> B((b))\n"),
        ("classDef", "flowchart LR\n  A[a] --> B[b]:::w\n  classDef w fill:#550000,stroke:#ff0000\n"),
        ("style", "flowchart LR\n  A[a] --> B[b]\n  style B fill:#003300,stroke:#00ff00\n"),
        ("edge-label", "flowchart LR\n  A[a] -->|x| B[b]\n"),
    ] {
        let mut request = SvgRequest::default();
        request.options.diagram_id = Some(format!("q-{name}"));
        request.pipeline = Some(SvgPipeline::resvg_safe());
        let output = Renderer::new().render(
            RenderRequest::svg(src, OperationControl::new(), request).with_theme(theme.clone()),
        )?;
        let RenderOutput::Svg(Some(svg)) = output else { continue };
        let text = svg.svg();
        let styles: Vec<&str> = text
            .match_indices("label-container")
            .filter_map(|(i, _)| {
                let rest = &text[i..];
                let start = rest.find("style=\"")? + 7;
                let end = rest[start..].find('"')?;
                Some(&rest[start..start + end])
            })
            .collect();
        println!("{name:<12} status={:?} diag={} styles={styles:?}",
            svg.evidence().theme_evidence().status(), svg.evidence().theme_diagnostics().len());
        for d in svg.evidence().theme_diagnostics() {
            println!("      {d:?}");
        }
    }
    let source = "flowchart LR\n  A[one] --> B[two] --> C[three] --> D[four]\n";
    let classic = format!("---\nconfig:\n  look: classic\n---\n{source}");
    let cases = [
        ("palette-only", json!({"palettes": {"node": ["#aa0000", "#00aa00", "#0000aa"]}})),
        ("fill-only", json!({"styles": {"node": {"fill": "#aa0000"}}})),
        ("stroke-only", json!({"styles": {"node": {"stroke": {"paint": "#00aa00"}}}})),
        ("stroke-solid-obj", json!({"styles": {"node": {"stroke": {"paint": {"kind": "solid", "color": "#00aa00"}}}}})),
        ("stroke-width-only", json!({"styles": {"node": {"stroke": {"width": 3}}}})),
        ("ordinal-fill-rule", json!({"rules": [
            {"target": "node", "ordinal": {"cycle": {"period": 2, "offset": 0}}, "style": {"fill": "#aa0000"}},
            {"target": "node", "ordinal": {"cycle": {"period": 2, "offset": 1}}, "style": {"fill": "#0000aa"}}
        ]})),
        ("ordinal-stroke-rule", json!({"rules": [
            {"target": "node", "ordinal": {"exact": 1}, "style": {"stroke": {"paint": "#aa0000"}}}
        ]})),
        ("marker", json!({"styles": {"marker": {"fill": "#aa0000"}}})),
        ("marker-stroke", json!({"styles": {"marker": {"stroke": {"paint": "#aa0000"}}}})),
        ("edge-stroke", json!({"styles": {"edge": {"stroke": {"paint": "#aa0000"}}}})),
        ("fill+palette", json!({"styles": {"node": {"fill": "#272937"}}, "palettes": {"node": ["#aa0000", "#00aa00"]}})),
        ("stroke+palette", json!({"styles": {"node": {"stroke": {"paint": "#414868"}}}, "palettes": {"node": ["#aa0000", "#00aa00"]}})),
        ("fill+stroke", json!({"styles": {"node": {"fill": "#272937", "stroke": {"paint": "#00aa00"}}}})),
        ("stroke paint+width", json!({"styles": {"node": {"stroke": {"paint": "#00aa00", "width": 1.5}}}})),
        ("palette+ordinal-fill", json!({"palettes": {"node": ["#aa0000", "#00aa00"]}, "rules": [
            {"target": "node", "ordinal": {"exact": 1}, "style": {"fill": "#0000aa"}}]})),
        ("mermaid-vars", json!({"mermaid": {"theme": "base", "dark_mode": true, "variables": {
            "primaryColor": "#272937", "primaryBorderColor": "#414868", "lineColor": "#7aa2f7"}}})),
    ];
    for (name, design) in cases {
        let design: ThemeDesignV1 = serde_json::from_value(design)?;
        let theme = DiagramThemeCompiler::new().compile_recipe(ThemeRecipeV1::Design { design })?;
        for (look, src) in [("neo", source), ("classic", classic.as_str())] {
            let mut request = SvgRequest::default();
            request.options.diagram_id = Some(format!("p-{name}-{look}"));
            request.pipeline = Some(SvgPipeline::resvg_safe());
            let output = Renderer::new().render(
                RenderRequest::svg(src, OperationControl::new(), request).with_theme(theme.clone()),
            )?;
            let RenderOutput::Svg(Some(svg)) = output else { continue };
            let text = svg.svg();
            let styles: Vec<&str> = text
                .match_indices("label-container\" style=\"")
                .map(|(i, m)| {
                    let rest = &text[i + m.len()..];
                    &rest[..rest.find('"').unwrap_or(0)]
                })
                .collect();
            let hits: Vec<&str> = ["#aa0000", "#00aa00", "#0000aa", "#7aa2f7", "#414868", "#272937"]
                .into_iter()
                .filter(|c| text.to_ascii_lowercase().contains(c))
                .collect();
            let codes: Vec<String> = svg
                .evidence()
                .theme_diagnostics()
                .iter()
                .map(|d| format!("{d:?}").split('"').nth(1).unwrap_or("").to_owned())
                .collect();
            println!("{name:<20} {look:<8} status={:?} diag={codes:?} hits={hits:?}", svg.evidence().theme_evidence().status());
            println!("    node styles: {:?}", &styles[..styles.len().min(4)]);
        }
    }
    Ok(())
}

/// Our post-processing for what the compiled format cannot express: an accent
/// stroke matching each node's ordinal palette fill. Merman writes theme paint
/// as inline `!important` declarations, so a `<style>` block cannot override
/// it; the rewrite edits the inline declaration instead.
fn accent_strokes(svg: &str) -> String {
    let bg = PALETTE.background;
    let muted = PALETTE.ansi[8];
    let mut out = svg.to_owned();
    for &index in &ACCENT_INDICES {
        let accent = PALETTE.ansi[index];
        let tint = mix(accent, bg, 0.22);
        out = out.replace(
            &format!("fill:{tint} !important;stroke:{muted} !important"),
            &format!("fill:{tint} !important;stroke:{accent} !important"),
        );
    }
    out
}
