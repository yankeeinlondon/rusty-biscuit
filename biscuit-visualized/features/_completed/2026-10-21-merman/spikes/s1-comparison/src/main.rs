//! Spike S1: render the comparison corpus with today's `biscuit-visualized`
//! (mermaid-rs-renderer plus our corrections and themes, D26) and with Merman,
//! rasterize both through the same resvg 0.45 path, and time each renderer.
//!
//! Output (outside the repository by default): one PNG per input, renderer, and
//! theme, plus `results-<os>.json`, which `scripts/build_gallery.py` turns into the
//! gallery and `gallery/scorecard.json`.
//!
//! Usage: `s1-comparison --os <label> --out <dir> [--iterations N] [--only <substr>]`

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock, mpsc};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

mod fix;
mod fonts;
#[cfg(feature = "merman")]
mod measure;
mod survey;

/// PNGs are 2x the SVG's natural size, at most 1000 px wide. tiny-skia writes
/// them barely compressed; the gallery script recompresses them losslessly.
const FULL: (f32, u32) = (2.0, 1000);
const PROBE_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
enum Theme {
    Dark,
    Default,
}

impl Theme {
    fn id(self) -> &'static str {
        match self {
            Theme::Dark => "dark",
            Theme::Default => "default",
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
struct ManifestEntry {
    name: String,
    family: String,
    #[serde(default)]
    group: Option<String>,
    #[serde(default)]
    source: Option<String>,
    #[serde(default)]
    occurrences: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct Input {
    name: String,
    family: String,
    /// `merman-fixture`, `repo-test`, or `gitgraph-case`.
    origin: String,
    group: String,
    source_ref: String,
    mmd_path: String,
    reference_svg: Option<String>,
    #[serde(skip)]
    text: String,
}

#[derive(Debug, Default, Serialize)]
struct Timing {
    iterations: usize,
    median_ms: f64,
    p95_ms: f64,
    min_ms: f64,
}

#[derive(Debug, Default, Serialize)]
struct RenderResult {
    ok: bool,
    error: Option<String>,
    svg_bytes: usize,
    /// SVG natural size in user units (usvg's view of width/height).
    svg_width: f32,
    svg_height: f32,
    png: Option<String>,
    png_width: u32,
    png_height: u32,
    /// Rasterization time for the PNG (one run, not in `timing`).
    raster_ms: f64,
    timing: Option<Timing>,
    /// For `current` only: the file-system work `MermaidDiagram::render` (and the
    /// spike's read-back) adds to each timed render, measured alone.
    cache_io: Option<Timing>,
    has_foreign_object: bool,
    /// Real-font columns only: renders sharing one warm measurer (`timing` uses
    /// a fresh, cold one per render).
    timing_warm: Option<Timing>,
    /// For `merman-real` only: `fix_merman_svg` alone on the raw Merman SVG.
    fix_timing: Option<Timing>,
    /// Merman columns only: label and paint defects measured from usvg's parse
    /// with the column's own rasterizer options.
    defects: Option<survey::Defects>,
}

#[derive(Debug, Serialize)]
struct Row {
    input: Input,
    /// theme -> renderer -> result
    results: BTreeMap<String, BTreeMap<String, RenderResult>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    gitgraph: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
struct Report {
    os: String,
    host: String,
    iterations: usize,
    merman_rev: &'static str,
    /// Which installed family each generic family resolved to on this host.
    generic_families: GenericFamilies,
    /// The resolution chain on this host (generic answers, MERMAID_FONT, and
    /// the step that resolved each requested family).
    font_chain: Option<fonts::ChainReport>,
    /// The font files handed to Merman's native measurement (comparison).
    real_fonts: Option<fonts::RealFontsReport>,
    /// Loading one native measurement context from those files.
    native_context_load_ms: Option<f64>,
    rows: Vec<Row>,
}

struct Args {
    os: String,
    out: PathBuf,
    iterations: usize,
    only: Option<String>,
}

fn parse_args() -> Args {
    let mut args = Args {
        os: std::env::consts::OS.to_string(),
        out: PathBuf::from("s1-out"),
        iterations: 15,
        only: None,
    };
    let mut it = std::env::args().skip(1);
    while let Some(flag) = it.next() {
        let value = it.next().unwrap_or_default();
        match flag.as_str() {
            "--os" => args.os = value,
            "--out" => args.out = PathBuf::from(value),
            "--iterations" => args.iterations = value.parse().expect("--iterations is a number"),
            "--only" => args.only = Some(value),
            other => panic!("unknown flag {other}"),
        }
    }
    args
}

fn spike_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn load_corpus() -> Vec<Input> {
    let corpus = spike_dir().join("corpus");
    let rel = |p: &Path| p.strip_prefix(spike_dir()).unwrap_or(p).display().to_string();
    let mut inputs = Vec::new();

    let read_manifest = |path: PathBuf| -> Vec<ManifestEntry> {
        serde_json::from_slice(&fs::read(&path).expect("manifest readable")).expect("manifest parses")
    };
    for entry in read_manifest(corpus.join("merman/manifest.json")) {
        let dir = corpus.join("merman").join(&entry.family);
        let mmd = dir.join(format!("{}.mmd", entry.name));
        let reference = dir.join(format!("{}.mermaid-js.svg", entry.name));
        inputs.push(Input {
            text: fs::read_to_string(&mmd).expect("fixture readable"),
            mmd_path: rel(&mmd),
            reference_svg: reference.exists().then(|| rel(&reference)),
            origin: "merman-fixture".into(),
            group: entry.group.clone().unwrap_or_else(|| "shared".into()),
            source_ref: entry.source.clone().unwrap_or_default(),
            name: entry.name,
            family: entry.family,
        });
    }
    for (folder, origin) in [("cases", "gitgraph-case"), ("tests", "repo-test")] {
        let dir = corpus.join(folder);
        let manifest: BTreeMap<String, ManifestEntry> = if folder == "tests" {
            read_manifest(dir.join("manifest.json")).into_iter().map(|e| (e.name.clone(), e)).collect()
        } else {
            BTreeMap::new()
        };
        let mut files: Vec<_> = fs::read_dir(&dir)
            .expect("corpus folder")
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|x| x == "mmd"))
            .collect();
        files.sort();
        for mmd in files {
            let name = mmd.file_stem().unwrap().to_string_lossy().to_string();
            let reference = mmd.with_file_name(format!("{name}.mermaid-js.svg"));
            let entry = manifest.get(&name);
            inputs.push(Input {
                text: fs::read_to_string(&mmd).expect("input readable"),
                mmd_path: rel(&mmd),
                reference_svg: reference.exists().then(|| rel(&reference)),
                origin: origin.into(),
                group: "shared".into(),
                source_ref: match entry {
                    Some(e) => e.occurrences.join(", "),
                    None => "spike case (docs/mermaid-gitgraph.md, R-S7)".into(),
                },
                family: entry.map(|e| e.family.clone()).unwrap_or_else(|| "gitgraph".into()),
                name,
            });
        }
    }
    inputs
}

// ---------------------------------------------------------------- renderers

type RenderFn = Arc<dyn Fn(&str, Theme) -> Result<String, String> + Send + Sync>;
/// Run before each timed iteration, outside the timed region.
type Prepare = Arc<dyn Fn() + Send + Sync>;

struct Column {
    name: &'static str,
    render: RenderFn,
    isolate_cache: bool,
    /// Installs a fresh, cold-cache measurement context before each timed render.
    prepare: Option<Prepare>,
    /// Fonts this column is rasterized and surveyed with.
    fonts: FontsKind,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum FontsKind {
    /// System fonts, usvg's default resolver (today's rasterizer).
    System,
    /// The `FontChain` resolution function (the real-fonts column).
    Chain,
    /// The resolved font files handed to Merman's native measurement.
    Native,
}

#[cfg(feature = "current")]
fn current_renderer() -> RenderFn {
    use biscuit_visualized::artifact::{OutputFormat, RenderRequest};
    use biscuit_visualized::mermaid::{MermaidDiagram, MermaidTheme};
    Arc::new(|source: &str, theme: Theme| {
        let theme = match theme {
            Theme::Dark => MermaidTheme::Dark,
            Theme::Default => MermaidTheme::Default,
        };
        let request = RenderRequest { format: OutputFormat::Svg, scale: 1, ..RenderRequest::default() };
        let artifact = MermaidDiagram::new(source)
            .with_theme(theme)
            .render(&request)
            .map_err(|e| e.to_string())?;
        fs::read_to_string(&artifact.path).map_err(|e| e.to_string())
    })
}

/// A fresh, empty cache root so `MermaidDiagram::render` never serves a cached
/// artifact. Called outside the timed region.
#[cfg(feature = "current")]
fn fresh_cache_dir() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("temp dir");
    // SAFETY: single-threaded at this point; no other thread reads the environment.
    unsafe { std::env::set_var("BISCUIT_VISUALIZED_CACHE_DIR", dir.path()) };
    dir
}

/// Builds the measurement environment for one render: Merman's deterministic
/// default, its native measurement on the real fonts, or our own measurer.
#[cfg(feature = "merman")]
type EnvFn = Arc<dyn Fn() -> merman::svg::SvgEnvironment + Send + Sync>;

#[cfg(feature = "merman")]
fn merman_renderer(environment: EnvFn, post: fn(&str) -> String) -> RenderFn {
    use merman::svg::SvgPipeline;
    use merman::{Engine, MermaidConfig, OperationControl, RenderOutput, RenderRequest, Renderer, SvgRequest};
    let make = |theme: &str| {
        Renderer::new().with_engine(
            Engine::new().with_site_config(MermaidConfig::from_value(serde_json::json!({ "theme": theme }))),
        )
    };
    let dark = make("dark");
    let default = make("default");
    Arc::new(move |source: &str, theme: Theme| {
        let renderer = match theme {
            Theme::Dark => &dark,
            Theme::Default => &default,
        };
        let mut request = SvgRequest { environment: environment(), ..SvgRequest::default() };
        request.options.diagram_id = Some("s1".into());
        request.pipeline = Some(SvgPipeline::resvg_safe());
        match renderer.render(RenderRequest::svg(source, OperationControl::new(), request)) {
            Ok(RenderOutput::Svg(Some(svg))) => Ok(post(svg.svg())),
            Ok(_) => Err("no diagram detected".into()),
            Err(e) => Err(e.to_string()),
        }
    })
}

/// Merman's native measurement on exactly the resolved font files. The context
/// is held in a slot so the timing loop can install a fresh (cold-cache)
/// context before each timed render, outside the timed region.
#[cfg(feature = "merman")]
struct NativeFonts {
    options: merman::svg::export::NativeFontOptions,
    slot: std::sync::Mutex<merman::svg::export::NativeFontContext>,
}

#[cfg(feature = "merman")]
impl NativeFonts {
    fn new(files: &[PathBuf]) -> Result<Self, String> {
        use merman::svg::export::{NativeFontLimits, NativeFontOptions};
        let mut options = NativeFontOptions::default().with_system_fonts(false).with_limits(NativeFontLimits {
            // The resolved families (all styles) can exceed the 64 MB default.
            max_bytes: 512 * 1024 * 1024,
            ..NativeFontLimits::default()
        });
        for file in files {
            options = options.with_font_file(file);
        }
        let context = options.clone().load(&merman::OperationControl::new()).map_err(|e| e.to_string())?;
        Ok(Self { options, slot: std::sync::Mutex::new(context) })
    }

    fn reload(&self) {
        let fresh = self.options.clone().load(&merman::OperationControl::new()).expect("font context reloads");
        *self.slot.lock().expect("slot") = fresh;
    }

    fn environment(&self) -> merman::svg::SvgEnvironment {
        let context = self.slot.lock().expect("slot").clone();
        merman::svg::SvgEnvironment::deterministic().with_native_font_context(context).expect("native font environment")
    }
}

#[cfg(feature = "merman")]
fn merman_gitgraph_layout(source: &str) -> Option<serde_json::Value> {
    use merman::{OperationControl, RenderOutput, RenderRequest, Renderer, SvgRequest};
    let output = Renderer::new()
        .render(RenderRequest::layout_json(source, OperationControl::new(), SvgRequest::default()))
        .ok()?;
    let RenderOutput::LayoutJson(Some(layout)) = output else { return None };
    let mut bytes = Vec::new();
    layout.layout().write_json_pretty(&mut bytes).ok()?;
    let value: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    let commits = value.pointer("/layout/GitGraphDiagram/commits")?.as_array()?.clone();
    Some(serde_json::Value::Array(
        commits
            .iter()
            .map(|c| {
                serde_json::json!({
                    "id": c.get("id"), "parents": c.get("parents"),
                    "tags": c.get("tags"), "branch": c.get("branch"),
                })
            })
            .collect(),
    ))
}

#[cfg(feature = "current")]
fn current_gitgraph_parents(source: &str) -> Option<serde_json::Value> {
    use biscuit_visualized::mermaid::MermaidDiagram;
    let geometry = MermaidDiagram::new(source).gitgraph_geometry().ok()??;
    Some(serde_json::to_value(format!("{geometry:?}")).ok()?)
}

/// First render on a worker thread, so a hang or panic marks the input failed
/// instead of stopping the run.
fn probe(render: &RenderFn, source: &str, theme: Theme) -> Result<String, String> {
    let (tx, rx) = mpsc::channel();
    let render = Arc::clone(render);
    let source = source.to_string();
    std::thread::spawn(move || {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| render(&source, theme)));
        let _ = tx.send(result.unwrap_or_else(|panic| {
            let message = panic
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| panic.downcast_ref::<&str>().map(|s| s.to_string()))
                .unwrap_or_else(|| "unknown panic".into());
            Err(format!("panic: {message}"))
        }));
    });
    rx.recv_timeout(PROBE_TIMEOUT).unwrap_or_else(|_| Err(format!("timeout after {PROBE_TIMEOUT:?}")))
}

fn time_renders(
    render: &RenderFn,
    source: &str,
    theme: Theme,
    iterations: usize,
    isolate_cache: bool,
    prepare: Option<&Prepare>,
) -> Timing {
    let mut samples = Vec::with_capacity(iterations);
    for _ in 0..iterations {
        #[cfg(feature = "current")]
        let _cache = isolate_cache.then(fresh_cache_dir);
        #[cfg(not(feature = "current"))]
        let _ = isolate_cache;
        if let Some(prepare) = prepare {
            prepare();
        }
        let started = Instant::now();
        let _ = render(source, theme);
        samples.push(started.elapsed().as_secs_f64() * 1000.0);
    }
    samples.sort_by(f64::total_cmp);
    let pick = |q: f64| samples[((samples.len() as f64 - 1.0) * q).round() as usize];
    Timing { iterations, median_ms: pick(0.5), p95_ms: pick(0.95), min_ms: samples[0] }
}

/// Replays what a cache-missing `MermaidDiagram::render` does on disk (the
/// `FileCache::get` existence check, `create_dir_all`, the write) plus the
/// spike's read-back, each iteration in a fresh temp dir like the timed renders.
#[cfg(feature = "current")]
fn time_cache_io(svg: &str, iterations: usize) -> Timing {
    let mut samples = Vec::with_capacity(iterations);
    for _ in 0..iterations {
        let root = tempfile::tempdir().expect("temp dir");
        let dir = root.path().join("biscuit-visualized/v1/mermaid/svg");
        let path = dir.join("0123456789abcdef.svg");
        let started = Instant::now();
        let _ = path.exists();
        fs::create_dir_all(&dir).expect("cache dir");
        fs::write(&path, svg).expect("cache write");
        let _ = fs::read_to_string(&path).expect("cache read");
        samples.push(started.elapsed().as_secs_f64() * 1000.0);
    }
    samples.sort_by(f64::total_cmp);
    let pick = |q: f64| samples[((samples.len() as f64 - 1.0) * q).round() as usize];
    Timing { iterations, median_ms: pick(0.5), p95_ms: pick(0.95), min_ms: samples[0] }
}

fn time_fix(raw: &str, iterations: usize) -> Timing {
    let mut samples = Vec::with_capacity(iterations);
    for _ in 0..iterations {
        let started = Instant::now();
        std::hint::black_box(fix::fix_merman_svg(std::hint::black_box(raw)));
        samples.push(started.elapsed().as_secs_f64() * 1000.0);
    }
    samples.sort_by(f64::total_cmp);
    let pick = |q: f64| samples[((samples.len() as f64 - 1.0) * q).round() as usize];
    Timing { iterations, median_ms: pick(0.5), p95_ms: pick(0.95), min_ms: samples[0] }
}

// --------------------------------------------------------------- rasterizer

/// The same path as `biscuit_visualized::raster::rasterize_svg_to_png`: default
/// usvg options with the system font database, uniform scale to a target width.
/// Reimplemented here so the Merman-only build variant rasterizes identically.
///
/// One deliberate difference (the rasterizer-side fallback the spec plans as
/// D24): fontdb maps the generic families to fixed names ("Arial", "Times New
/// Roman", "Courier New"), and when the host lacks them a `sans-serif` fallback
/// resolves to nothing and resvg silently drops the text. Each generic family is
/// pointed at the first candidate the host actually has. Both columns use it.
fn fontdb() -> Arc<resvg::usvg::fontdb::Database> {
    static DB: OnceLock<Arc<resvg::usvg::fontdb::Database>> = OnceLock::new();
    DB.get_or_init(|| {
        let mut db = resvg::usvg::fontdb::Database::new();
        db.load_system_fonts();
        let generics = resolve_generic_families(&db);
        if let Some(name) = &generics.sans_serif {
            db.set_sans_serif_family(name.clone());
        }
        if let Some(name) = &generics.serif {
            db.set_serif_family(name.clone());
        }
        if let Some(name) = &generics.monospace {
            db.set_monospace_family(name.clone());
        }
        Arc::new(db)
    })
    .clone()
}

#[derive(Debug, Clone, Serialize)]
struct GenericFamilies {
    sans_serif: Option<String>,
    serif: Option<String>,
    monospace: Option<String>,
}

const SANS_CANDIDATES: [&str; 5] = ["Arial", "Helvetica", "Liberation Sans", "DejaVu Sans", "Noto Sans"];
const SERIF_CANDIDATES: [&str; 5] = ["Times New Roman", "Times", "Liberation Serif", "DejaVu Serif", "Noto Serif"];
const MONO_CANDIDATES: [&str; 6] =
    ["Courier New", "Courier", "Menlo", "Liberation Mono", "DejaVu Sans Mono", "Noto Sans Mono"];

fn resolve_generic_families(db: &resvg::usvg::fontdb::Database) -> GenericFamilies {
    let first_present = |candidates: &[&str]| {
        candidates
            .iter()
            .find(|name| db.faces().any(|face| face.families.iter().any(|(family, _)| family == *name)))
            .map(|name| name.to_string())
    };
    GenericFamilies {
        sans_serif: first_present(&SANS_CANDIDATES),
        serif: first_present(&SERIF_CANDIDATES),
        monospace: first_present(&MONO_CANDIDATES),
    }
}

struct Raster {
    png: Vec<u8>,
    svg_width: f32,
    svg_height: f32,
    width: u32,
    height: u32,
}

/// usvg options for every column. `default_sans` is fix (c) of the fixed
/// column: text with no font set uses the host's sans-serif family instead of
/// usvg's default "Times New Roman".
fn usvg_options(default_sans: bool, case_insensitive: bool) -> resvg::usvg::Options<'static> {
    let mut options = resvg::usvg::Options::default();
    options.fontdb = fontdb();
    if case_insensitive {
        options.font_resolver = fonts::case_insensitive_resolver();
    }
    if default_sans && let Some(sans) = resolve_generic_families(&options.fontdb).sans_serif {
        options.font_family = sans;
    }
    options
}

fn rasterize(svg: &str, (factor, cap): (f32, u32), options: &resvg::usvg::Options) -> Result<Raster, String> {
    use resvg::{tiny_skia, usvg};
    let tree = usvg::Tree::from_str(svg, options).map_err(|e| format!("usvg: {e}"))?;
    let size = tree.size();
    let target = ((size.width() * factor).round() as u32).clamp(1, cap);
    let scale = target as f32 / size.width();
    let width = ((size.width() * scale).round() as u32).max(1);
    let height = ((size.height() * scale).round() as u32).max(1);
    let mut pixmap = tiny_skia::Pixmap::new(width, height).ok_or("pixmap allocation failed")?;
    resvg::render(&tree, usvg::Transform::from_scale(scale, scale), &mut pixmap.as_mut());
    let png = pixmap.encode_png().map_err(|e| e.to_string())?;
    Ok(Raster { png, svg_width: size.width(), svg_height: size.height(), width, height })
}

fn main() {
    // `--raster-file <in.svg> <out.png>`: rasterize one SVG through the same path
    // (used to test SVG post-processing ideas against resvg 0.45).
    let argv: Vec<String> = std::env::args().collect();
    if argv.get(1).is_some_and(|a| a == "--raster-file") {
        let svg = fs::read_to_string(&argv[2]).expect("svg readable");
        let flag = |f: &str| argv.iter().skip(4).any(|a| a == f);
        let raster = rasterize(&svg, FULL, &usvg_options(flag("--sans"), flag("--ci"))).expect("rasterize");
        fs::write(&argv[3], raster.png).expect("png write");
        return;
    }
    // `--survey-file <in.svg> [--ci] [--sans]`: print the defect survey as JSON
    // (used on merman-cli's own output).
    if argv.get(1).is_some_and(|a| a == "--survey-file") {
        let svg = fs::read_to_string(&argv[2]).expect("svg readable");
        let flag = |f: &str| argv.iter().skip(3).any(|a| a == f);
        let defects = survey::survey(&svg, &usvg_options(flag("--sans"), flag("--ci"))).expect("survey");
        println!("{}", serde_json::to_string(&defects).expect("json"));
        return;
    }
    // `--font-demo <out.png> <out.json>`: one small flowchart through the font
    // chain (honors MERMAID_FONT), to show the override and its fall-through.
    #[cfg(feature = "merman")]
    if argv.get(1).is_some_and(|a| a == "--font-demo") {
        font_demo(&argv[2], &argv[3]);
        return;
    }
    let args = parse_args();
    let mut inputs = load_corpus();
    if let Some(only) = &args.only {
        inputs.retain(|i| i.name.contains(only.as_str()) || i.family == *only);
    }
    let _ = fontdb();
    let mut columns: Vec<Column> = Vec::new();
    #[cfg(feature = "current")]
    columns.push(Column { name: "current", render: current_renderer(), isolate_cache: true, prepare: None, fonts: FontsKind::System });

    // Real fonts: discover every family the corpus's Merman SVGs request; the
    // chain resolves them, and both our measurer and the rasterizer use it.
    #[allow(unused_mut, unused_assignments)]
    let mut chain: Option<Arc<fonts::FontChain>> = None;
    #[allow(unused_mut, unused_assignments)]
    let mut real: Option<Arc<fonts::RealFonts>> = None;
    #[allow(unused_mut, unused_assignments)]
    let mut native_context_load_ms = None;
    #[cfg(feature = "merman")]
    {
        let deterministic: EnvFn = Arc::new(merman::svg::SvgEnvironment::deterministic);
        let raw = merman_renderer(deterministic, |svg| svg.to_string());
        let mut requested = std::collections::BTreeSet::new();
        for input in &inputs {
            for theme in [Theme::Dark, Theme::Default] {
                if let Ok(svg) = probe(&raw, &input.text, theme) {
                    fonts::requested_families(&svg, &mut requested);
                }
            }
        }
        let mut font_chain = fonts::FontChain::new();
        font_chain.record_requested(&requested);
        let font_chain = Arc::new(font_chain);
        eprintln!("font chain: {:?}", font_chain.report);

        // Our measurer: one per render for cold timing, shared for warm.
        let measurer_slot = Arc::new(std::sync::Mutex::new(Arc::new(measure::UsvgMeasurer::new(&font_chain))));
        let chain_env: EnvFn = {
            let slot = Arc::clone(&measurer_slot);
            Arc::new(move || {
                use merman::svg::{MeasurementProfileId, TextMeasurementPolicy, TextMeasurementProfile, TextMeasurementProfileIdentity};
                let identity = TextMeasurementProfileIdentity::new(
                    MeasurementProfileId::new("s1-usvg-0.45").expect("profile id"),
                    "1",
                )
                .expect("profile identity");
                let measurer: Arc<measure::UsvgMeasurer> = Arc::clone(&slot.lock().expect("slot"));
                let profile = TextMeasurementProfile::new(identity, measurer);
                merman::svg::SvgEnvironment::deterministic().with_text_measurement_policy(TextMeasurementPolicy::uniform(profile))
            })
        };
        let fresh_measurer: Prepare = {
            let slot = Arc::clone(&measurer_slot);
            let font_chain = Arc::clone(&font_chain);
            Arc::new(move || *slot.lock().expect("slot") = Arc::new(measure::UsvgMeasurer::new(&font_chain)))
        };

        // Merman's own native measurement on the files of the faces the chain
        // resolved (comparison only).
        let resolved: std::collections::BTreeSet<String> = font_chain.resolved_families().into_iter().collect();
        let real_fonts = Arc::new(fonts::RealFonts::build(
            &font_chain.db,
            &resolved,
            [
                font_chain.report.sans_serif.family.clone(),
                font_chain.report.serif.family.clone(),
                font_chain.report.monospace.family.clone(),
            ],
        ));
        let started = Instant::now();
        let native = Arc::new(NativeFonts::new(&real_fonts.files).expect("native font context"));
        native_context_load_ms = Some(started.elapsed().as_secs_f64() * 1000.0);
        let native_env: EnvFn = {
            let native = Arc::clone(&native);
            Arc::new(move || native.environment())
        };
        let reload: Prepare = {
            let native = Arc::clone(&native);
            Arc::new(move || native.reload())
        };
        columns.push(Column { name: "merman", render: raw, isolate_cache: false, prepare: None, fonts: FontsKind::System });
        columns.push(Column {
            name: "merman-real",
            render: merman_renderer(chain_env, fix::fix_merman_svg),
            isolate_cache: false,
            prepare: Some(fresh_measurer),
            fonts: FontsKind::Chain,
        });
        columns.push(Column {
            name: "merman-native",
            render: merman_renderer(native_env, fix::fix_merman_svg),
            isolate_cache: false,
            prepare: Some(reload),
            fonts: FontsKind::Native,
        });
        chain = Some(font_chain);
        real = Some(real_fonts);
    }

    eprintln!("{} inputs, {} columns, {} iterations", inputs.len(), columns.len(), args.iterations);
    let mut rows = Vec::new();
    for input in inputs {
        let mut results: BTreeMap<String, BTreeMap<String, RenderResult>> = BTreeMap::new();
        for theme in [Theme::Dark, Theme::Default] {
            let mut raw_merman_svg: Option<String> = None;
            for column in &columns {
                let renderer_name = column.name;
                let render = &column.render;
                #[cfg(feature = "current")]
                let _cache = column.isolate_cache.then(fresh_cache_dir);
                let mut result = RenderResult::default();
                match probe(render, &input.text, theme) {
                    Err(error) => result.error = Some(error),
                    Ok(svg) => {
                        result.svg_bytes = svg.len();
                        result.has_foreign_object = svg.contains("<foreignObject");
                        if renderer_name == "merman" {
                            raw_merman_svg = Some(svg.clone());
                        }
                        let options = match (column.fonts, &chain, &real) {
                            (FontsKind::Chain, Some(chain), _) => chain.usvg_options(),
                            (FontsKind::Native, _, Some(real)) => real.usvg_options(),
                            _ => usvg_options(false, false),
                        };
                        if renderer_name.starts_with("merman") {
                            result.defects = survey::survey(&svg, &options).ok();
                        }
                        let started = Instant::now();
                        match rasterize(&svg, FULL, &options) {
                            Err(error) => result.error = Some(format!("rasterize: {error}")),
                            Ok(raster) => {
                                result.raster_ms = started.elapsed().as_secs_f64() * 1000.0;
                                let dir = args.out.join("png").join(theme.id()).join(renderer_name);
                                fs::create_dir_all(&dir).expect("png dir");
                                let path = dir.join(format!("{}.png", input.name));
                                fs::write(&path, &raster.png).expect("png write");
                                result.ok = true;
                                result.png = Some(path.display().to_string());
                                result.svg_width = raster.svg_width;
                                result.svg_height = raster.svg_height;
                                result.png_width = raster.width;
                                result.png_height = raster.height;
                                let svg_dir = args.out.join("svg").join(theme.id()).join(renderer_name);
                                fs::create_dir_all(&svg_dir).expect("svg dir");
                                fs::write(svg_dir.join(format!("{}.svg", input.name)), &svg).expect("svg write");
                            }
                        }
                        // Timing is taken for the dark theme only (the gallery's primary pairing).
                        if theme == Theme::Dark && args.iterations > 0 {
                            let _ = render(&input.text, theme); // warm-up
                            result.timing = Some(time_renders(
                                render,
                                &input.text,
                                theme,
                                args.iterations,
                                column.isolate_cache,
                                column.prepare.as_ref(),
                            ));
                            if column.prepare.is_some() {
                                let _ = render(&input.text, theme); // warm the shared context
                                result.timing_warm =
                                    Some(time_renders(render, &input.text, theme, args.iterations, false, None));
                            }
                            #[cfg(feature = "current")]
                            if column.isolate_cache {
                                result.cache_io = Some(time_cache_io(&svg, args.iterations));
                            }
                            if renderer_name == "merman-real"
                                && let Some(raw) = &raw_merman_svg
                            {
                                result.fix_timing = Some(time_fix(raw, args.iterations.max(100)));
                            }
                        }
                    }
                }
                eprintln!(
                    "{:<10} {:<13} {:<7} {:<60} {}",
                    input.family,
                    renderer_name,
                    theme.id(),
                    &input.name[..input.name.len().min(60)],
                    match (&result.error, &result.timing) {
                        (Some(e), _) => format!("FAIL {}", e.lines().next().unwrap_or("")),
                        (None, Some(t)) => format!("{:.2} ms", t.median_ms),
                        (None, None) => "ok".into(),
                    }
                );
                results.entry(theme.id().into()).or_default().insert(renderer_name.to_string(), result);
            }
        }
        let gitgraph = (input.family == "gitgraph" && input.origin != "repo-test").then(|| {
            let mut check = serde_json::Map::new();
            #[cfg(feature = "merman")]
            check.insert("merman_commits".into(), merman_gitgraph_layout(&input.text).unwrap_or_default());
            #[cfg(feature = "current")]
            check.insert("current_geometry".into(), current_gitgraph_parents(&input.text).unwrap_or_default());
            serde_json::Value::Object(check)
        });
        rows.push(Row { input, results, gitgraph });
    }

    let report = Report {
        host: host_description(),
        os: args.os.clone(),
        iterations: args.iterations,
        merman_rev: "482fcde7faffb05a466f198085d615e604075d97",
        generic_families: resolve_generic_families(&fontdb()),
        font_chain: chain.as_ref().map(|c| c.report.clone()),
        real_fonts: real.as_ref().map(|r| r.report.clone()),
        native_context_load_ms,
        rows,
    };
    fs::create_dir_all(&args.out).expect("out dir");
    let path = args.out.join(format!("results-{}.json", args.os));
    fs::write(&path, serde_json::to_vec_pretty(&report).expect("serialize")).expect("results write");
    eprintln!("wrote {}", path.display());
}

#[cfg(feature = "merman")]
fn font_demo(png_path: &str, json_path: &str) {
    use merman::svg::{MeasurementProfileId, TextMeasurementPolicy, TextMeasurementProfile, TextMeasurementProfileIdentity};
    const SOURCE: &str = "flowchart LR\n    A[Measured and drawn] --> B{Same face?}\n    B -->|yes| C[No clipping]\n";
    let chain = Arc::new(fonts::FontChain::new());
    let env: EnvFn = {
        let chain = Arc::clone(&chain);
        Arc::new(move || {
            let identity =
                TextMeasurementProfileIdentity::new(MeasurementProfileId::new("s1-usvg-0.45").expect("id"), "1").expect("identity");
            let profile = TextMeasurementProfile::new(identity, Arc::new(measure::UsvgMeasurer::new(&chain)));
            merman::svg::SvgEnvironment::deterministic().with_text_measurement_policy(TextMeasurementPolicy::uniform(profile))
        })
    };
    let render = merman_renderer(env, fix::fix_merman_svg);
    let svg = render(SOURCE, Theme::Default).expect("demo renders");
    let options = chain.usvg_options();
    let defects = survey::survey(&svg, &options).expect("survey");
    let raster = rasterize(&svg, FULL, &options).expect("rasterize");
    fs::write(png_path, raster.png).expect("png write");
    let report = serde_json::json!({
        "mermaid_font": chain.report.mermaid_font,
        "mermaid_font_installed": chain.report.mermaid_font_installed,
        "drawn_with": defects.fonts,
        "clipped": defects.clipped,
        "png": png_path,
    });
    fs::write(json_path, serde_json::to_vec_pretty(&report).expect("json")).expect("json write");
    println!("{report}");
}

fn host_description() -> String {
    let run = |cmd: &str, args: &[&str]| {
        std::process::Command::new(cmd)
            .args(args)
            .output()
            .ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .filter(|s| !s.is_empty())
    };
    let cpu = run("sysctl", &["-n", "machdep.cpu.brand_string"])
        .or_else(|| {
            fs::read_to_string("/proc/cpuinfo").ok().and_then(|c| {
                c.lines().find(|l| l.starts_with("model name")).map(|l| l.split(':').nth(1).unwrap_or("").trim().to_string())
            })
        })
        .unwrap_or_else(|| "unknown CPU".into());
    let os = run("sw_vers", &["-productVersion"])
        .map(|v| format!("macOS {v}"))
        .or_else(|| {
            fs::read_to_string("/etc/os-release").ok().and_then(|c| {
                c.lines().find(|l| l.starts_with("PRETTY_NAME=")).map(|l| l[12..].trim_matches('"').to_string())
            })
        })
        .unwrap_or_else(|| std::env::consts::OS.into());
    format!("{os}, {cpu}, {} threads", std::thread::available_parallelism().map(|n| n.get()).unwrap_or(0))
}
