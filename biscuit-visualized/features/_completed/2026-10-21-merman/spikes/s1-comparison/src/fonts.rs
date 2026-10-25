//! Real-font support for the "Merman (real fonts)" column.
//!
//! `FontChain` is the one font-resolution function. It is used both by the usvg
//! `FontResolver` that draws and by the Merman `TextMeasurer` that measures
//! (`measure.rs`), so every label is measured and drawn with the same face:
//!
//! 1. `MERMAID_FONT`: one family name, matched ignoring case. Set but not
//!    installed: a warning naming it, then fall through; never an error.
//! 2. The label's own `font-family` stack in order (Mermaid's default is
//!    `"trebuchet ms", verdana, arial, sans-serif`), named families matched
//!    ignoring case (CSS semantics), generic entries through step 3's answers.
//! 3. The OS's answer for the generic family (sans-serif; serif or monospace
//!    when requested) from fontique: fontconfig, CoreText, DirectWrite. The
//!    face fontique picks is mapped to usvg's fontdb by file path plus face
//!    index.
//! 4. If the OS has no answer: the installed face whose OS/2 table classifies
//!    it (sFamilyClass, else PANOSE), first by family name, then path, then
//!    face index.
//!
//! The rest of this module is the comparison path for Merman's own native
//! measurement: the font set Merman measures with and the rasterizer draws with.
//!
//! Measurement and drawing must pick the same face for every label. Merman's
//! native measurement shapes text through usvg with its own font database and
//! `browser_like_font_resolver` (merman-export, rev 482fcde). This module
//! reproduces both halves for our resvg 0.45:
//!
//! - `RealFonts::build` resolves every family the corpus requests against the
//!   installed fonts, case-insensitively (CSS does; fontdb does not, so
//!   Mermaid's `"trebuchet ms"` never matches "Trebuchet MS" by default), adds
//!   the host's generic sans-serif/serif/monospace candidates, and lists the
//!   font files of exactly those families. Merman gets those files; our
//!   database is loaded from the same files in the same order.
//! - `configure_generics` applies Merman's rule for generic families
//!   (`configure_fontdb_generic_families`): a generic family that fontdb's
//!   default name does not satisfy becomes the first non-monospace family
//!   (serif too), or the first monospace family for monospace.
//! - `case_insensitive_resolver` is Merman's resolver order: each requested
//!   family ignoring case, then the generic serif, then a browser-like fallback
//!   (sans-serif first; monospace first for monospace-like requests).

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::Arc;

use resvg::usvg::{self, FontFamily, fontdb};
use serde::Serialize;

pub fn case_insensitive_resolver() -> usvg::FontResolver<'static> {
    usvg::FontResolver {
        select_font: Box::new(|font, db: &mut Arc<fontdb::Database>| {
            let weight = fontdb::Weight(font.weight());
            let stretch: fontdb::Stretch = font.stretch().into();
            let style: fontdb::Style = font.style().into();
            let query = |families: &[fontdb::Family]| {
                db.query(&fontdb::Query { families, weight, stretch, style })
            };
            for family in font.families() {
                let found = match family {
                    FontFamily::Named(name) => installed_casing(db, name).and_then(|n| query(&[fontdb::Family::Name(&n)])),
                    FontFamily::Serif => query(&[fontdb::Family::Serif]),
                    FontFamily::SansSerif => query(&[fontdb::Family::SansSerif]),
                    FontFamily::Monospace => query(&[fontdb::Family::Monospace]),
                    FontFamily::Cursive => query(&[fontdb::Family::Cursive]),
                    FontFamily::Fantasy => query(&[fontdb::Family::Fantasy]),
                };
                if found.is_some() {
                    return found;
                }
            }
            let monospace = font.families().iter().any(|f| match f {
                FontFamily::Monospace => true,
                FontFamily::Named(n) => {
                    let n = n.to_ascii_lowercase();
                    n.contains("mono") || n.contains("courier") || n.contains("consol")
                }
                _ => false,
            });
            let browser_like: &[fontdb::Family] = if monospace {
                &[fontdb::Family::Monospace, fontdb::Family::SansSerif, fontdb::Family::Serif]
            } else {
                &[fontdb::Family::SansSerif, fontdb::Family::Serif, fontdb::Family::Monospace]
            };
            query(&[fontdb::Family::Serif]).or_else(|| query(browser_like)).or_else(|| db.faces().next().map(|f| f.id))
        }),
        select_fallback: usvg::FontResolver::default_fallback_selector(),
    }
}

/// The installed spelling of a family name, matched ignoring ASCII case.
fn installed_casing(db: &fontdb::Database, name: &str) -> Option<String> {
    db.faces()
        .flat_map(|face| face.families.iter())
        .find(|(family, _)| family.eq_ignore_ascii_case(name))
        .map(|(family, _)| family.clone())
}

/// The resolved font set for one host.
pub struct RealFonts {
    /// Font files in load order; Merman receives exactly these.
    pub files: Vec<PathBuf>,
    /// Our database, loaded from `files` with Merman's generic-family rule.
    pub db: Arc<fontdb::Database>,
    pub report: RealFontsReport,
}

#[derive(Debug, Clone, Serialize)]
pub struct RealFontsReport {
    /// requested family -> installed family it matched ignoring case, if any
    pub requested: Vec<(String, Option<String>)>,
    pub generic_sans_serif: Option<String>,
    pub generic_serif: Option<String>,
    pub generic_monospace: Option<String>,
    /// The family used for text with no font set (Merman's raster default).
    pub default_family: Option<String>,
    pub files: Vec<String>,
    pub file_bytes: u64,
}

impl RealFonts {
    /// `requested` are the named families found in the corpus's SVGs;
    /// `generic_candidates` are the host's sans-serif, serif, and monospace
    /// candidates (the generic mapping the rasterizer already uses).
    pub fn build(system: &fontdb::Database, requested: &BTreeSet<String>, generic_candidates: [Option<String>; 3]) -> Self {
        let mut families: Vec<String> = Vec::new();
        let push = |name: String, families: &mut Vec<String>| {
            if !families.contains(&name) {
                families.push(name);
            }
        };
        // Generic candidates first: Merman's fallback takes the first
        // non-monospace family in load order.
        for candidate in generic_candidates.iter().flatten() {
            push(candidate.clone(), &mut families);
        }
        let mut matched = Vec::new();
        for name in requested {
            let installed = installed_casing(system, name);
            if let Some(family) = &installed {
                push(family.clone(), &mut families);
            }
            matched.push((name.clone(), installed));
        }
        let mut files: Vec<PathBuf> = Vec::new();
        for family in &families {
            for face in system.faces() {
                if face.families.iter().any(|(n, _)| n == family) {
                    let path = match &face.source {
                        fontdb::Source::File(path) => Some(path.clone()),
                        fontdb::Source::SharedFile(path, _) => Some(path.clone()),
                        fontdb::Source::Binary(_) => None,
                    };
                    if let Some(path) = path
                        && !files.contains(&path)
                    {
                        files.push(path);
                    }
                }
            }
        }
        let mut db = fontdb::Database::new();
        for file in &files {
            let _ = db.load_font_file(file);
        }
        configure_generics(&mut db);
        let family_of = |generic: fontdb::Family| {
            db.query(&fontdb::Query { families: &[generic], ..Default::default() })
                .and_then(|id| db.face(id))
                .and_then(|face| face.families.first().map(|(n, _)| n.clone()))
        };
        let report = RealFontsReport {
            requested: matched,
            generic_sans_serif: family_of(fontdb::Family::SansSerif),
            generic_serif: family_of(fontdb::Family::Serif),
            generic_monospace: family_of(fontdb::Family::Monospace),
            default_family: family_of(fontdb::Family::SansSerif)
                .or_else(|| family_of(fontdb::Family::Serif))
                .or_else(|| db.faces().next().and_then(|f| f.families.first().map(|(n, _)| n.clone()))),
            files: files.iter().map(|f| f.display().to_string()).collect(),
            file_bytes: files.iter().filter_map(|f| std::fs::metadata(f).ok()).map(|m| m.len()).sum(),
        };
        RealFonts { files, db: Arc::new(db), report }
    }

    /// usvg options that draw with the same faces Merman measured with.
    pub fn usvg_options(&self) -> usvg::Options<'static> {
        let mut options = usvg::Options::default();
        options.fontdb = Arc::clone(&self.db);
        options.font_resolver = case_insensitive_resolver();
        if let Some(family) = &self.report.default_family {
            options.font_family = family.clone();
        }
        options
    }
}

/// Merman's `configure_fontdb_generic_families`, for our fontdb version.
pub fn configure_generics(db: &mut fontdb::Database) {
    let first = |db: &fontdb::Database, mono: Option<bool>| {
        db.faces()
            .find(|f| mono.is_none_or(|m| f.monospaced == m))
            .and_then(|f| f.families.first().map(|(n, _)| n.clone()))
    };
    let sans = first(db, Some(false)).or_else(|| first(db, None));
    let mono = first(db, Some(true)).or_else(|| sans.clone());
    let resolves = |db: &fontdb::Database, family: fontdb::Family| {
        db.query(&fontdb::Query { families: &[family], ..Default::default() }).is_some()
    };
    if !resolves(db, fontdb::Family::SansSerif)
        && let Some(f) = &sans
    {
        db.set_sans_serif_family(f.clone());
    }
    if !resolves(db, fontdb::Family::Serif)
        && let Some(f) = &sans
    {
        db.set_serif_family(f.clone());
    }
    if !resolves(db, fontdb::Family::Monospace)
        && let Some(f) = &mono
    {
        db.set_monospace_family(f.clone());
    }
}

/// Named families declared in an SVG's `font-family` properties and attributes.
pub fn requested_families(svg: &str, into: &mut BTreeSet<String>) {
    let svg = svg.replace("&quot;", "\"");
    for key in ["font-family:", "font-family=\""] {
        let mut rest = svg.as_str();
        while let Some(at) = rest.find(key) {
            rest = &rest[at + key.len()..];
            let end = rest.find([';', '}', '>', '\n']).unwrap_or(rest.len());
            let value = &rest[..end];
            let value = if key.ends_with('"') { value.split('"').next().unwrap_or("") } else { value };
            for part in value.split(',') {
                let name = part.trim().trim_matches(['"', '\'']).trim();
                let generic = matches!(
                    name.to_ascii_lowercase().as_str(),
                    "serif" | "sans-serif" | "monospace" | "cursive" | "fantasy" | "system-ui" | "inherit" | "initial" | ""
                );
                if !generic && !name.contains(['(', ')', '!', '&', ';']) {
                    into.insert(name.to_string());
                }
            }
        }
    }
}

// ------------------------------------------------------------ the chain

/// Which step of the chain produced a face.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Step {
    MermaidFont,
    Stack,
    StackGeneric,
    GenericFallback,
    FirstFace,
}

/// The answer for one generic family, and how it was obtained.
#[derive(Debug, Clone, Serialize)]
pub struct GenericChoice {
    pub family: Option<String>,
    /// "fontique", "os2", or "none".
    pub source: &'static str,
    /// fontique's first candidate family, even when it did not map to fontdb.
    pub fontique_family: Option<String>,
    pub path: Option<String>,
    pub index: Option<u32>,
    /// Whether fontique's face (path + index) is a face in usvg's fontdb.
    pub in_fontdb: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChainReport {
    pub mermaid_font: Option<String>,
    pub mermaid_font_installed: Option<String>,
    pub sans_serif: GenericChoice,
    pub serif: GenericChoice,
    pub monospace: GenericChoice,
    /// requested family -> (step, face family) for the corpus's families.
    pub requested: Vec<(String, Step, Option<String>)>,
    pub fontique_ms: f64,
}

pub struct FontChain {
    pub db: Arc<fontdb::Database>,
    pub report: ChainReport,
}

impl FontChain {
    pub fn new() -> Self {
        let mut db = fontdb::Database::new();
        db.load_system_fonts();
        let started = std::time::Instant::now();
        let mut collection = fontique::Collection::new(fontique::CollectionOptions { system_fonts: true, ..Default::default() });
        let mut pick = |generic: fontique::GenericFamily, class: Class| match fontique_choice(&mut collection, &db, generic) {
            Ok(choice) => choice,
            Err(unmapped) => {
                let mut choice = os2_choice(&db, class);
                if let Some(u) = unmapped {
                    // fontique answered with a face usvg's fontdb does not have.
                    choice.fontique_family = u.fontique_family;
                    choice.in_fontdb = Some(false);
                }
                choice
            }
        };
        let sans_serif = pick(fontique::GenericFamily::SansSerif, Class::Sans);
        let serif = pick(fontique::GenericFamily::Serif, Class::Serif);
        let monospace = pick(fontique::GenericFamily::Monospace, Class::Mono);
        let fontique_ms = started.elapsed().as_secs_f64() * 1000.0;
        // usvg's own generic queries (and Merman's) then agree with the chain.
        if let Some(f) = &sans_serif.family {
            db.set_sans_serif_family(f.clone());
        }
        if let Some(f) = &serif.family {
            db.set_serif_family(f.clone());
        }
        if let Some(f) = &monospace.family {
            db.set_monospace_family(f.clone());
        }
        let mermaid_font = std::env::var("MERMAID_FONT").ok().filter(|v| !v.trim().is_empty());
        let mermaid_font_installed = mermaid_font.as_deref().and_then(|name| installed_casing(&db, name.trim()));
        if let (Some(name), None) = (&mermaid_font, &mermaid_font_installed) {
            eprintln!("warning: MERMAID_FONT={name:?} is not an installed font family; using the Mermaid font stack instead");
        }
        let report = ChainReport {
            mermaid_font,
            mermaid_font_installed,
            sans_serif,
            serif,
            monospace,
            requested: Vec::new(),
            fontique_ms,
        };
        FontChain { db: Arc::new(db), report }
    }

    /// The one resolution function.
    pub fn resolve(&self, font: &usvg::Font) -> Option<(fontdb::ID, Step)> {
        let db = &self.db;
        let weight = fontdb::Weight(font.weight());
        let stretch: fontdb::Stretch = font.stretch().into();
        let style: fontdb::Style = font.style().into();
        let query = |name: &str| db.query(&fontdb::Query { families: &[fontdb::Family::Name(name)], weight, stretch, style });
        if let Some(name) = &self.report.mermaid_font_installed
            && let Some(id) = query(name)
        {
            return Some((id, Step::MermaidFont));
        }
        for family in font.families() {
            let found = match family {
                FontFamily::Named(name) => installed_casing(db, name).and_then(|n| query(&n)).map(|id| (id, Step::Stack)),
                generic => self.generic(generic).and_then(|n| query(n)).map(|id| (id, Step::StackGeneric)),
            };
            if found.is_some() {
                return found;
            }
        }
        let monospace = requests_monospace(font.families());
        let fallback = if monospace { &self.report.monospace } else { &self.report.sans_serif };
        fallback
            .family
            .as_deref()
            .and_then(query)
            .map(|id| (id, Step::GenericFallback))
            .or_else(|| db.faces().next().map(|f| (f.id, Step::FirstFace)))
    }

    fn generic(&self, family: &FontFamily) -> Option<&str> {
        match family {
            FontFamily::Serif => self.report.serif.family.as_deref(),
            FontFamily::Monospace => self.report.monospace.family.as_deref(),
            _ => self.report.sans_serif.family.as_deref(),
        }
    }

    /// The family drawn for text with no font set (fix c): the sans-serif answer.
    pub fn default_family(&self) -> Option<String> {
        self.report.mermaid_font_installed.clone().or_else(|| self.report.sans_serif.family.clone())
    }

    pub fn usvg_options(self: &Arc<Self>) -> usvg::Options<'static> {
        let mut options = usvg::Options::default();
        options.fontdb = Arc::clone(&self.db);
        let chain = Arc::clone(self);
        options.font_resolver = usvg::FontResolver {
            select_font: Box::new(move |font, _db| chain.resolve(font).map(|(id, _)| id)),
            select_fallback: usvg::FontResolver::default_fallback_selector(),
        };
        if let Some(family) = self.default_family() {
            options.font_family = family;
        }
        options
    }

    /// Records, for each requested family, which step resolved it and to what.
    pub fn record_requested(&mut self, requested: &BTreeSet<String>) {
        let faces: Vec<_> = requested
            .iter()
            .map(|name| {
                let font = probe_font(&self.db, name);
                let resolved = font.as_ref().and_then(|f| self.resolve(f));
                let face = resolved.and_then(|(id, _)| self.db.face(id)).and_then(|f| f.families.first().map(|(n, _)| n.clone()));
                (name.clone(), resolved.map_or(Step::FirstFace, |(_, step)| step), face)
            })
            .collect();
        self.report.requested = faces;
    }

    /// The families a native-measurement comparison should be handed: the faces
    /// the chain resolves for the corpus, plus the generic answers.
    pub fn resolved_families(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        for f in [&self.report.sans_serif.family, &self.report.serif.family, &self.report.monospace.family].into_iter().flatten() {
            if !out.contains(f) {
                out.push(f.clone());
            }
        }
        for (_, _, face) in &self.report.requested {
            if let Some(f) = face
                && !out.contains(f)
            {
                out.push(f.clone());
            }
        }
        out
    }
}

/// A `usvg::Font` for one family name, obtained by parsing a one-text SVG
/// (usvg's `Font` has no public constructor).
fn probe_font(db: &Arc<fontdb::Database>, name: &str) -> Option<usvg::Font> {
    let svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="1" height="1"><text id="t" font-family="{}">x</text></svg>"#,
        name.replace('"', "&quot;")
    );
    // Resolution is irrelevant here, but usvg drops text it cannot lay out, so
    // the parse needs a database with at least one face.
    let mut options = usvg::Options::default();
    options.fontdb = Arc::clone(db);
    options.font_resolver.select_font = Box::new(|_, db| db.faces().next().map(|f| f.id));
    let tree = usvg::Tree::from_str(&svg, &options).ok()?;
    match tree.node_by_id("t") {
        Some(usvg::Node::Text(t)) => t.chunks().first().and_then(|c| c.spans().first()).map(|s| s.font().clone()),
        _ => None,
    }
}

fn requests_monospace(families: &[FontFamily]) -> bool {
    families.iter().any(|f| match f {
        FontFamily::Monospace => true,
        FontFamily::Named(n) => {
            let n = n.to_ascii_lowercase();
            n.contains("mono") || n.contains("courier") || n.contains("consol") || n.contains("menlo")
        }
        _ => false,
    })
}

/// `Ok` when one of fontique's candidates maps to a fontdb face; otherwise
/// `Err` with fontique's first (unmapped) candidate, if it had any.
fn fontique_choice(
    collection: &mut fontique::Collection,
    db: &fontdb::Database,
    generic: fontique::GenericFamily,
) -> Result<GenericChoice, Option<GenericChoice>> {
    let ids: Vec<_> = collection.generic_families(generic).collect();
    let mut first: Option<GenericChoice> = None;
    for id in ids {
        let Some(family) = collection.family(id) else { continue };
        let Some(font) = family.match_font(
            fontique::FontWidth::NORMAL,
            fontique::FontStyle::Normal,
            fontique::FontWeight::NORMAL,
            false,
        ) else {
            continue;
        };
        let (path, index) = match font.source().kind() {
            fontique::SourceKind::Path(path) => (Some(path.to_path_buf()), font.index()),
            _ => (None, font.index()),
        };
        let face = path.as_ref().and_then(|p| {
            db.faces().find(|f| {
                let face_path = match &f.source {
                    fontdb::Source::File(fp) | fontdb::Source::SharedFile(fp, _) => Some(fp),
                    fontdb::Source::Binary(_) => None,
                };
                face_path.is_some_and(|fp| same_file(fp, p)) && f.index == index
            })
        });
        let choice = GenericChoice {
            family: face.and_then(|f| f.families.first().map(|(n, _)| n.clone())),
            source: "fontique",
            fontique_family: Some(family.name().to_string()),
            path: path.as_ref().map(|p| p.display().to_string()),
            index: Some(index),
            in_fontdb: Some(face.is_some()),
        };
        if choice.family.is_some() {
            return Ok(choice);
        }
        first.get_or_insert(choice);
    }
    Err(first)
}

fn same_file(a: &std::path::Path, b: &std::path::Path) -> bool {
    a == b || std::fs::canonicalize(a).ok() == std::fs::canonicalize(b).ok()
}

#[derive(Clone, Copy)]
enum Class {
    Sans,
    Serif,
    Mono,
}

/// Step 4: the first face (by family name, path, index) whose OS/2 table
/// classifies it. Sans: sFamilyClass class 8, or PANOSE Latin Text (2) with a
/// sans serif style (11-13). Serif: sFamilyClass 1-5 or 7, or PANOSE Latin Text
/// with serif styles 2-10. Monospace: fontdb's `monospaced` (post table).
fn os2_choice(db: &fontdb::Database, class: Class) -> GenericChoice {
    let mut faces: Vec<&fontdb::FaceInfo> = db.faces().collect();
    let key = |f: &fontdb::FaceInfo| {
        let path = match &f.source {
            fontdb::Source::File(p) | fontdb::Source::SharedFile(p, _) => p.display().to_string(),
            fontdb::Source::Binary(_) => String::new(),
        };
        (f.families.first().map(|(n, _)| n.clone()).unwrap_or_default(), path, f.index)
    };
    faces.sort_by_key(|f| key(f));
    let classified = faces.into_iter().find(|f| match class {
        Class::Mono => f.monospaced,
        Class::Sans | Class::Serif => {
            !f.monospaced
                && db
                    .with_face_data(f.id, |data, index| os2_class(data, index))
                    .flatten()
                    .is_some_and(|(family_class, panose_family, panose_serif)| {
                        let latin = panose_family == 2;
                        match class {
                            Class::Sans => family_class == 8 || (latin && (11..=13).contains(&panose_serif)),
                            _ => matches!(family_class, 1..=5 | 7) || (latin && (2..=10).contains(&panose_serif)),
                        }
                    })
        }
    });
    GenericChoice {
        family: classified.and_then(|f| f.families.first().map(|(n, _)| n.clone())),
        source: if classified.is_some() { "os2" } else { "none" },
        fontique_family: None,
        path: classified.map(|f| key(f).1),
        index: classified.map(|f| f.index),
        in_fontdb: None,
    }
}

/// (sFamilyClass class byte, PANOSE bFamilyType, PANOSE bSerifStyle)
fn os2_class(data: &[u8], index: u32) -> Option<(u8, u8, u8)> {
    let face = ttf_parser::RawFace::parse(data, index).ok()?;
    let os2 = face.table(ttf_parser::Tag::from_bytes(b"OS/2"))?;
    // sFamilyClass at offset 30 (class in the high byte), PANOSE at 32..42.
    Some((*os2.get(30)?, *os2.get(32)?, *os2.get(33)?))
}
