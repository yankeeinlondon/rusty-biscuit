//! Renders every frozen corpus input through `MermaidDiagram::render` under one
//! comparison configuration and records what each render produced.
//!
//! Run through `run-leg.sh`, which assembles a throwaway crate outside the
//! workspace, pins it with a copy of the workspace lockfile (so the leg uses
//! exactly the workspace's resolved renderer and rasterizer), and points
//! `BISCUIT_VISUALIZED_CACHE_DIR` at an empty directory.

use std::fmt::Write as _;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use biscuit_visualized::artifact::{OutputFormat, RenderRequest};
use biscuit_visualized::mermaid::{MermaidDiagram, MermaidTheme};
use resvg::usvg;
use resvg::usvg::fontdb::{Database, Family, Query};

/// Identical across legs A, B and C; changing any of these invalidates the
/// comparison with earlier legs.
const THEMES: [(MermaidTheme, &str); 2] = [(MermaidTheme::Default, "default"), (MermaidTheme::Dark, "dark")];
const TARGET_WIDTH: u32 = 1200;
const SCALE: u32 = 2;

struct Variant {
    suffix: &'static str,
    request: RenderRequest,
}

fn variants() -> Vec<Variant> {
    let svg = RenderRequest {
        format: OutputFormat::Svg,
        ..RenderRequest::default()
    };
    let width = RenderRequest {
        transparent_background: false,
        ..RenderRequest::default()
    }
    .with_target_width(TARGET_WIDTH);
    let scaled = RenderRequest {
        scale: SCALE,
        transparent_background: true,
        ..RenderRequest::default()
    };
    vec![
        Variant { suffix: "svg", request: svg },
        Variant { suffix: "w1200.png", request: width },
        Variant { suffix: "s2.transparent.png", request: scaled },
    ]
}

fn main() {
    let mut args = std::env::args().skip(1);
    let corpus = PathBuf::from(args.next().expect("corpus dir"));
    let out = PathBuf::from(args.next().expect("output dir"));
    let leg = args.next().expect("leg label");
    fs::create_dir_all(&out).expect("output dir");

    let mut fonts = Database::new();
    fonts.load_system_fonts();
    let fonts = Arc::new(fonts);

    let mut inputs: Vec<PathBuf> = fs::read_dir(&corpus)
        .expect("corpus dir")
        .map(|entry| entry.expect("entry").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "mmd"))
        .collect();
    inputs.sort();

    let mut report = String::new();
    writeln!(report, "# Leg {leg}\n").unwrap();
    writeln!(
        report,
        "Configuration: themes default+dark; SVG; PNG target_width={TARGET_WIDTH} opaque; PNG scale={SCALE} transparent. Isolated cache: `{}`.\n",
        std::env::var("BISCUIT_VISUALIZED_CACHE_DIR").unwrap_or_else(|_| "UNSET".into())
    )
    .unwrap();
    writeln!(report, "| input | theme | artifact | outcome | natural size | viewBox / PNG px | corner alpha | xxh64 |").unwrap();
    writeln!(report, "| --- | --- | --- | --- | --- | --- | --- | --- |").unwrap();

    let mut families = std::collections::BTreeMap::<String, String>::new();
    for input in &inputs {
        let name = input.file_stem().unwrap().to_string_lossy().to_string();
        let source = fs::read_to_string(input).expect("read input");
        for (theme, theme_name) in THEMES {
            let diagram = MermaidDiagram::new(source.clone()).with_theme(theme);
            let natural = match diagram.natural_size() {
                Ok(size) => format!("{:.1}×{:.1}", size.width, size.height),
                Err(_) => "—".into(),
            };
            for variant in variants() {
                let file = format!("{name}.{theme_name}.{}", variant.suffix);
                let row = match diagram.render(&variant.request) {
                    Err(err) => {
                        let message = err.to_string().replace('|', "\\|").replace('\n', " ");
                        format!("error: {message} | {natural} | — | — | —")
                    }
                    Ok(artifact) => {
                        assert!(!artifact.cache_hit, "{file}: isolated cache served a hit");
                        let bytes = fs::read(&artifact.path).expect("artifact");
                        fs::write(out.join(&file), &bytes).expect("copy artifact");
                        let hash = format!("{:016x}", biscuit_hash::xx_hash_bytes(&bytes));
                        if variant.suffix == "svg" {
                            let svg = String::from_utf8_lossy(&bytes).to_string();
                            for family in resolve_families(&svg, &fonts) {
                                families.entry(family.0).or_insert(family.1);
                            }
                            format!("ok | {natural} | {} | — | {hash}", view_box(&svg))
                        } else {
                            let pixmap = resvg::tiny_skia::Pixmap::decode_png(&bytes).expect("decode png");
                            let alpha = pixmap.pixel(0, 0).map(|p| p.alpha()).unwrap_or(0);
                            format!("ok | {natural} | {}×{} | {alpha} | {hash}", pixmap.width(), pixmap.height())
                        }
                    }
                };
                writeln!(report, "| {name} | {theme_name} | {} | {row} |", variant.suffix).unwrap();
            }
        }
    }

    writeln!(report, "\n## Font families (requested → resolved by fontdb)\n").unwrap();
    for (requested, resolved) in &families {
        writeln!(report, "- `{requested}` → {resolved}").unwrap();
    }
    fs::write(out.join("report.md"), &report).expect("report");
    print!("{report}");
}

fn view_box(svg: &str) -> String {
    attribute(svg, "viewBox").unwrap_or_else(|| "none".into())
}

fn attribute(text: &str, name: &str) -> Option<String> {
    let start = text.find(&format!("{name}=\""))? + name.len() + 2;
    let end = text[start..].find('"')? + start;
    Some(text[start..end].to_string())
}

/// Every `font-family` list the SVG names, with the face fontdb (the database
/// the rasterizer is given) selects for it. Parsing through usvg first proves
/// the artifact is standalone-parseable with these fonts.
fn resolve_families(svg: &str, fonts: &Arc<Database>) -> Vec<(String, String)> {
    let options = usvg::Options {
        fontdb: fonts.clone(),
        ..usvg::Options::default()
    };
    if let Err(err) = usvg::Tree::from_str(svg, &options) {
        return vec![("<usvg parse failure>".into(), err.to_string())];
    }
    let mut lists = Vec::new();
    for marker in ["font-family=\"", "font-family:"] {
        let mut rest = svg;
        while let Some(index) = rest.find(marker) {
            rest = &rest[index + marker.len()..];
            let end = rest.find(['"', ';', '}']).unwrap_or(rest.len());
            lists.push(rest[..end].trim().replace("&quot;", "\"").to_string());
        }
    }
    lists.sort();
    lists.dedup();
    lists
        .into_iter()
        .map(|list| {
            let names: Vec<String> = list.split(',').map(|n| n.trim().trim_matches(['"', '\'']).to_string()).collect();
            let families: Vec<Family> = names
                .iter()
                .map(|name| match name.as_str() {
                    "serif" => Family::Serif,
                    "sans-serif" => Family::SansSerif,
                    "monospace" => Family::Monospace,
                    "cursive" => Family::Cursive,
                    "fantasy" => Family::Fantasy,
                    other => Family::Name(other),
                })
                .collect();
            let resolved = fonts
                .query(&Query {
                    families: &families,
                    ..Query::default()
                })
                .and_then(|id| fonts.face(id))
                .map(|face| face.families.first().map(|f| f.0.clone()).unwrap_or_default())
                .unwrap_or_else(|| "NO MATCH (usvg falls back to its default family)".into());
            (list, resolved)
        })
        .collect()
}

