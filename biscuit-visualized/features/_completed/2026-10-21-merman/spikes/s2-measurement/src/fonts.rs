//! One font-resolution chain for measuring and drawing (S1's `FontChain`,
//! adapted for S2).
//!
//! Changes from S1:
//! - The chain resolves a [`FontQuery`], our own struct, instead of a
//!   `usvg::Font`, so option A can resolve a Merman `TextStyle` without parsing
//!   an SVG. The usvg resolver converts each `usvg::Font` into a `FontQuery`.
//! - Discovery (fontique, the system database) is separated from the database
//!   that measures and draws, which is loaded from an explicit file list: the
//!   faces the chain resolves for Mermaid's font stacks, plus one CJK, one
//!   emoji, and one RTL-capable fallback face. Option B receives the same files.
//! - `MERMAID_FONT` is passed in explicitly (the font variants are run in one
//!   process).
//!
//! The chain, per query: (1) `MERMAID_FONT` if installed (case-insensitive);
//! (2) the query's families in order, named ones matched ignoring case, generic
//! ones through step 3's answers; (3) the OS's generic answer (fontique),
//! sans-serif unless the query looks monospace; (4) the first face.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use resvg::usvg::{self, fontdb};
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Generic {
    Serif,
    SansSerif,
    Monospace,
    Cursive,
    Fantasy,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Family {
    Named(String),
    Generic(Generic),
}

/// A font request: what both the usvg resolver and option A's measurer resolve.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FontQuery {
    pub families: Vec<Family>,
    pub weight: u16,
    pub style: fontdb::Style,
    pub stretch: fontdb::Stretch,
}

impl FontQuery {
    pub fn from_usvg(font: &usvg::Font) -> Self {
        let families = font
            .families()
            .iter()
            .map(|f| match f {
                usvg::FontFamily::Named(n) => Family::Named(n.clone()),
                usvg::FontFamily::Serif => Family::Generic(Generic::Serif),
                usvg::FontFamily::SansSerif => Family::Generic(Generic::SansSerif),
                usvg::FontFamily::Monospace => Family::Generic(Generic::Monospace),
                usvg::FontFamily::Cursive => Family::Generic(Generic::Cursive),
                usvg::FontFamily::Fantasy => Family::Generic(Generic::Fantasy),
            })
            .collect();
        FontQuery { families, weight: font.weight(), style: font.style().into(), stretch: font.stretch().into() }
    }

    /// A query from CSS values as Merman's `TextStyle` carries them. `default`
    /// is the family usvg uses for text with no `font-family`.
    pub fn from_css(family: Option<&str>, weight: Option<&str>, style: Option<&str>, default: &str) -> Self {
        let families = match family.map(str::trim).filter(|f| !f.is_empty()) {
            Some(list) => parse_family_list(list),
            None => vec![Family::Named(default.to_string())],
        };
        FontQuery {
            families,
            weight: parse_weight(weight),
            style: match style.map(|s| s.trim().to_ascii_lowercase()) {
                Some(s) if s == "italic" => fontdb::Style::Italic,
                Some(s) if s.starts_with("oblique") => fontdb::Style::Oblique,
                _ => fontdb::Style::Normal,
            },
            stretch: fontdb::Stretch::Normal,
        }
    }

    /// The CSS `font-family` value for this query (used for option B's probes).
    pub fn css_family(&self) -> String {
        self.families
            .iter()
            .map(|f| match f {
                Family::Named(n) => format!("\"{n}\""),
                Family::Generic(Generic::Serif) => "serif".into(),
                Family::Generic(Generic::SansSerif) => "sans-serif".into(),
                Family::Generic(Generic::Monospace) => "monospace".into(),
                Family::Generic(Generic::Cursive) => "cursive".into(),
                Family::Generic(Generic::Fantasy) => "fantasy".into(),
            })
            .collect::<Vec<_>>()
            .join(", ")
    }

    fn requests_monospace(&self) -> bool {
        self.families.iter().any(|f| match f {
            Family::Generic(Generic::Monospace) => true,
            Family::Named(n) => {
                let n = n.to_ascii_lowercase();
                n.contains("mono") || n.contains("courier") || n.contains("consol") || n.contains("menlo")
            }
            _ => false,
        })
    }
}

/// svgtypes' rule: the five generic keywords, everything else a name.
pub fn parse_family_list(list: &str) -> Vec<Family> {
    list.split(',')
        .map(|part| part.trim().trim_matches(['"', '\'']).trim())
        .filter(|name| !name.is_empty())
        .map(|name| match name.to_ascii_lowercase().as_str() {
            "serif" => Family::Generic(Generic::Serif),
            "sans-serif" => Family::Generic(Generic::SansSerif),
            "monospace" => Family::Generic(Generic::Monospace),
            "cursive" => Family::Generic(Generic::Cursive),
            "fantasy" => Family::Generic(Generic::Fantasy),
            _ => Family::Named(name.to_string()),
        })
        .collect()
}

fn parse_weight(weight: Option<&str>) -> u16 {
    match weight.map(|w| w.trim().to_ascii_lowercase()) {
        None => 400,
        Some(w) => match w.as_str() {
            "" | "normal" => 400,
            "bold" | "bolder" => 700,
            "lighter" => 300,
            n => n.parse::<f32>().map(|v| v.clamp(1.0, 1000.0) as u16).unwrap_or(400),
        },
    }
}

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

#[derive(Debug, Clone, Serialize)]
pub struct GenericChoice {
    pub family: Option<String>,
    /// "fontique", "os2", or "none".
    pub source: &'static str,
    pub fontique_family: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FallbackFace {
    pub role: &'static str,
    pub family: Option<String>,
    pub probe: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChainReport {
    pub mermaid_font: Option<String>,
    pub mermaid_font_installed: Option<String>,
    pub sans_serif: GenericChoice,
    pub serif: GenericChoice,
    pub monospace: GenericChoice,
    pub fallbacks: Vec<FallbackFace>,
    /// requested family -> (step, face family), resolved on the system database.
    pub requested: Vec<(String, Step, Option<String>)>,
    pub files: Vec<String>,
    pub file_bytes: u64,
    pub discovery_ms: f64,
}

/// Host discovery: the system database plus the chain's per-host answers.
pub struct Discovery {
    pub system: fontdb::Database,
    pub sans_serif: GenericChoice,
    pub serif: GenericChoice,
    pub monospace: GenericChoice,
    pub mermaid_font: Option<String>,
    pub mermaid_font_installed: Option<String>,
    pub fallbacks: Vec<FallbackFace>,
    pub ms: f64,
}

const CJK: &[&str] = &["PingFang SC", "Hiragino Sans GB", "Hiragino Sans", "Noto Sans CJK SC", "Droid Sans Fallback", "Microsoft YaHei"];
const EMOJI: &[&str] = &["Apple Color Emoji", "Noto Color Emoji", "Segoe UI Emoji", "DejaVu Sans"];
const RTL: &[&str] = &["Arial Unicode MS", "DejaVu Sans", "Arial", "Noto Sans Arabic", "Geeza Pro", "Segoe UI"];

impl Discovery {
    pub fn run(mermaid_font: Option<String>) -> Self {
        let started = std::time::Instant::now();
        let mut system = fontdb::Database::new();
        system.load_system_fonts();
        let mut collection = fontique::Collection::new(fontique::CollectionOptions { system_fonts: true, ..Default::default() });
        let mut pick = |generic: fontique::GenericFamily, class: Class| {
            fontique_choice(&mut collection, &system, generic).unwrap_or_else(|first| {
                let mut choice = os2_choice(&system, class);
                choice.fontique_family = first;
                choice
            })
        };
        let sans_serif = pick(fontique::GenericFamily::SansSerif, Class::Sans);
        let serif = pick(fontique::GenericFamily::Serif, Class::Serif);
        let monospace = pick(fontique::GenericFamily::Monospace, Class::Mono);
        let mermaid_font = mermaid_font.filter(|v| !v.trim().is_empty());
        let mermaid_font_installed = mermaid_font.as_deref().and_then(|n| installed_casing(&system, n.trim()));
        if let (Some(name), None) = (&mermaid_font, &mermaid_font_installed) {
            eprintln!("warning: MERMAID_FONT={name:?} is not an installed font family; using the Mermaid font stack instead");
        }
        let fallback = |role: &'static str, candidates: &[&str], probes: &[char]| FallbackFace {
            role,
            probe: probes.iter().collect(),
            family: candidates
                .iter()
                .find(|name| {
                    system.faces().any(|f| {
                        f.families.iter().any(|(n, _)| n == *name)
                            && probes.iter().all(|c| system.with_face_data(f.id, |d, i| has_glyph(d, i, *c)).unwrap_or(false))
                    })
                })
                .map(|s| s.to_string()),
        };
        let fallbacks = vec![
            fallback("cjk", CJK, &['中', 'か']),
            fallback("emoji", EMOJI, &['😀']),
            fallback("rtl", RTL, &['ش', 'ש']),
        ];
        Discovery {
            system,
            sans_serif,
            serif,
            monospace,
            mermaid_font,
            mermaid_font_installed,
            fallbacks,
            ms: started.elapsed().as_secs_f64() * 1000.0,
        }
    }
}

fn has_glyph(data: &[u8], index: u32, c: char) -> bool {
    ttf_parser::Face::parse(data, index).ok().and_then(|f| f.glyph_index(c)).is_some()
}

pub struct FontChain {
    /// Exactly `files`, in order. Measures (A) and draws (usvg resolver).
    pub db: Arc<fontdb::Database>,
    pub files: Vec<PathBuf>,
    pub report: ChainReport,
}

impl FontChain {
    /// Builds the measuring/drawing database from the faces the chain resolves
    /// for `requested` (Mermaid's stacks, found in the corpus SVGs) plus the
    /// fallback faces.
    pub fn build(discovery: &Discovery, requested: &BTreeSet<String>) -> Self {
        let rules = Rules {
            mermaid_font: discovery.mermaid_font_installed.clone(),
            sans: discovery.sans_serif.family.clone(),
            serif: discovery.serif.family.clone(),
            mono: discovery.monospace.family.clone(),
        };
        let system = &discovery.system;
        let mut resolved = Vec::new();
        let mut families: Vec<String> = Vec::new();
        let push = |name: &str, families: &mut Vec<String>| {
            if !families.iter().any(|f| f == name) {
                families.push(name.to_string());
            }
        };
        if let Some(mf) = &rules.mermaid_font {
            push(mf, &mut families);
        } else {
            // Generic answers first: Merman's generic-family rule picks the
            // first non-monospace family in load order.
            for f in [&rules.sans, &rules.serif, &rules.mono].into_iter().flatten() {
                push(f, &mut families);
            }
        }
        for name in requested {
            let query = FontQuery::from_css(Some(name), None, None, name);
            let hit = rules.resolve(system, &query);
            let face = hit.and_then(|(id, _)| system.face(id)).and_then(|f| f.families.first().map(|(n, _)| n.clone()));
            if let Some(face) = &face {
                push(face, &mut families);
            }
            resolved.push((name.clone(), hit.map_or(Step::FirstFace, |(_, s)| s), face));
        }
        for fb in &discovery.fallbacks {
            if let Some(f) = &fb.family {
                push(f, &mut families);
            }
        }
        let mut files: Vec<PathBuf> = Vec::new();
        for family in &families {
            for face in system.faces() {
                if face.families.iter().any(|(n, _)| n == family) {
                    let path = match &face.source {
                        fontdb::Source::File(p) | fontdb::Source::SharedFile(p, _) => Some(p.clone()),
                        fontdb::Source::Binary(_) => None,
                    };
                    if let Some(p) = path
                        && !files.contains(&p)
                    {
                        files.push(p);
                    }
                }
            }
        }
        let mut db = fontdb::Database::new();
        for f in &files {
            let _ = db.load_font_file(f);
        }
        for (generic, name) in [(Generic::SansSerif, &rules.sans), (Generic::Serif, &rules.serif), (Generic::Monospace, &rules.mono)] {
            if let Some(n) = name.as_ref().filter(|n| installed_casing(&db, n).is_some()) {
                match generic {
                    Generic::SansSerif => db.set_sans_serif_family(n.clone()),
                    Generic::Serif => db.set_serif_family(n.clone()),
                    _ => db.set_monospace_family(n.clone()),
                }
            }
        }
        let report = ChainReport {
            mermaid_font: discovery.mermaid_font.clone(),
            mermaid_font_installed: discovery.mermaid_font_installed.clone(),
            sans_serif: discovery.sans_serif.clone(),
            serif: discovery.serif.clone(),
            monospace: discovery.monospace.clone(),
            fallbacks: discovery.fallbacks.clone(),
            requested: resolved,
            files: files.iter().map(|f| f.display().to_string()).collect(),
            file_bytes: files.iter().filter_map(|f| std::fs::metadata(f).ok()).map(|m| m.len()).sum(),
            discovery_ms: discovery.ms,
        };
        FontChain { db: Arc::new(db), files, report }
    }

    fn rules(&self) -> Rules {
        Rules {
            mermaid_font: self.report.mermaid_font_installed.clone(),
            sans: self.report.sans_serif.family.clone(),
            serif: self.report.serif.family.clone(),
            mono: self.report.monospace.family.clone(),
        }
    }

    /// The one resolution function, on the measuring/drawing database.
    pub fn resolve(&self, query: &FontQuery) -> Option<(fontdb::ID, Step)> {
        self.rules().resolve(&self.db, query)
    }

    /// The family for text with no font set.
    pub fn default_family(&self) -> String {
        self.report
            .mermaid_font_installed
            .clone()
            .or_else(|| self.report.sans_serif.family.clone())
            .unwrap_or_else(|| "sans-serif".into())
    }

    pub fn usvg_options(self: &Arc<Self>) -> usvg::Options<'static> {
        let mut options = usvg::Options::default();
        options.fontdb = Arc::clone(&self.db);
        let chain = Arc::clone(self);
        options.font_resolver = usvg::FontResolver {
            select_font: Box::new(move |font, _db| chain.resolve(&FontQuery::from_usvg(font)).map(|(id, _)| id)),
            select_fallback: usvg::FontResolver::default_fallback_selector(),
        };
        options.font_family = self.default_family();
        options
    }

    pub fn family_name(&self, id: fontdb::ID) -> String {
        self.db
            .face(id)
            .map(|f| format!("{}{}", f.post_script_name, if f.index > 0 { format!("#{}", f.index) } else { String::new() }))
            .unwrap_or_else(|| "?".into())
    }
}

struct Rules {
    mermaid_font: Option<String>,
    sans: Option<String>,
    serif: Option<String>,
    mono: Option<String>,
}

impl Rules {
    fn resolve(&self, db: &fontdb::Database, q: &FontQuery) -> Option<(fontdb::ID, Step)> {
        let query = |name: &str| {
            let name = installed_casing(db, name)?;
            db.query(&fontdb::Query { families: &[fontdb::Family::Name(&name)], weight: fontdb::Weight(q.weight), stretch: q.stretch, style: q.style })
        };
        if let Some(name) = &self.mermaid_font
            && let Some(id) = query(name)
        {
            return Some((id, Step::MermaidFont));
        }
        for family in &q.families {
            let found = match family {
                Family::Named(name) => query(name).map(|id| (id, Step::Stack)),
                Family::Generic(g) => self.generic(*g).and_then(query).map(|id| (id, Step::StackGeneric)),
            };
            if found.is_some() {
                return found;
            }
        }
        let fallback = if q.requests_monospace() { &self.mono } else { &self.sans };
        fallback
            .as_deref()
            .and_then(query)
            .map(|id| (id, Step::GenericFallback))
            .or_else(|| db.faces().next().map(|f| (f.id, Step::FirstFace)))
    }

    fn generic(&self, g: Generic) -> Option<&str> {
        match g {
            Generic::Serif => self.serif.as_deref(),
            Generic::Monospace => self.mono.as_deref(),
            _ => self.sans.as_deref(),
        }
    }
}

/// The installed spelling of a family name, matched ignoring ASCII case.
pub fn installed_casing(db: &fontdb::Database, name: &str) -> Option<String> {
    db.faces()
        .flat_map(|face| face.families.iter())
        .find(|(family, _)| family.eq_ignore_ascii_case(name))
        .map(|(family, _)| family.clone())
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

// ---------------------------------------------------- copied from S1

fn fontique_choice(
    collection: &mut fontique::Collection,
    db: &fontdb::Database,
    generic: fontique::GenericFamily,
) -> Result<GenericChoice, Option<String>> {
    let ids: Vec<_> = collection.generic_families(generic).collect();
    let mut first: Option<String> = None;
    for id in ids {
        let Some(family) = collection.family(id) else { continue };
        let Some(font) =
            family.match_font(fontique::FontWidth::NORMAL, fontique::FontStyle::Normal, fontique::FontWeight::NORMAL, false)
        else {
            continue;
        };
        let path = match font.source().kind() {
            fontique::SourceKind::Path(path) => Some(path.to_path_buf()),
            _ => None,
        };
        let index = font.index();
        let face = path.as_ref().and_then(|p| {
            db.faces().find(|f| {
                let fp = match &f.source {
                    fontdb::Source::File(fp) | fontdb::Source::SharedFile(fp, _) => Some(fp),
                    fontdb::Source::Binary(_) => None,
                };
                fp.is_some_and(|fp| same_file(fp, p)) && f.index == index
            })
        });
        if let Some(face) = face {
            return Ok(GenericChoice {
                family: face.families.first().map(|(n, _)| n.clone()),
                source: "fontique",
                fontique_family: Some(family.name().to_string()),
            });
        }
        first.get_or_insert(family.name().to_string());
    }
    Err(first)
}

fn same_file(a: &Path, b: &Path) -> bool {
    a == b || std::fs::canonicalize(a).ok() == std::fs::canonicalize(b).ok()
}

#[derive(Clone, Copy)]
enum Class {
    Sans,
    Serif,
    Mono,
}

fn os2_choice(db: &fontdb::Database, class: Class) -> GenericChoice {
    let mut faces: Vec<&fontdb::FaceInfo> = db.faces().collect();
    faces.sort_by_key(|f| f.families.first().map(|(n, _)| n.clone()).unwrap_or_default());
    let classified = faces.into_iter().find(|f| match class {
        Class::Mono => f.monospaced,
        Class::Sans | Class::Serif => {
            !f.monospaced
                && db.with_face_data(f.id, os2_class).flatten().is_some_and(|(family_class, panose_family, panose_serif)| {
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
    }
}

fn os2_class(data: &[u8], index: u32) -> Option<(u8, u8, u8)> {
    let face = ttf_parser::RawFace::parse(data, index).ok()?;
    let os2 = face.table(ttf_parser::Tag::from_bytes(b"OS/2"))?;
    Some((*os2.get(30)?, *os2.get(32)?, *os2.get(33)?))
}
